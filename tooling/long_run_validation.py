"""C28 長時間運用の開発用検証器。

実Broker IPCを通じて、対話の反復、Runtime相当のAdapter再起動、Broker再起動、
接続断・再接続、履歴のbounded読取、Brokerの資源観測を実行する。
実installed製品や外部Runtimeの8時間稼働を、この検証器だけで証明してはならない。
"""

from __future__ import annotations

import argparse
import ctypes
import hashlib
import http.server
import json
import os
import shutil
import socket
import subprocess
import sys
import tempfile
import threading
import time
import uuid
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
HELPER_ROOT = ROOT / "native" / "rust_helper"
HELPER_BINARY = HELPER_ROOT / "target" / "debug" / (
    "gui_shell_rust_helper.exe" if os.name == "nt" else "gui_shell_rust_helper"
)
MAX_REPORT_SAMPLES = 512


def json_bytes(value: Any) -> bytes:
    return json.dumps(value, ensure_ascii=False, separators=(",", ":")).encode("utf-8")


def payload_hash(value: Any) -> str:
    canonical = json.dumps(value, ensure_ascii=False, separators=(",", ":"), sort_keys=True).encode("utf-8")
    return "sha256:" + hashlib.sha256(canonical).hexdigest()


def now_text() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def wait_for_file(path: Path, process: subprocess.Popen, timeout: float = 30.0) -> dict[str, Any]:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if path.exists():
            return json.loads(path.read_text(encoding="utf-8"))
        if process.poll() is not None:
            raise RuntimeError(f"Brokerが起動直後に終了した: {process.returncode}")
        time.sleep(0.05)
    raise TimeoutError(f"Broker資格fileが制限時間内に作成されない: {path}")


def require_endpoint(value: Any) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise RuntimeError("Broker資格fileがobjectではない")
    required = {
        "host",
        "port",
        "session_id",
        "session_secret",
        "credential_role",
        "transport",
        "max_request_bytes",
    }
    if set(value) != required:
        raise RuntimeError("Broker資格fileのfield集合が不正")
    if value["host"] != "127.0.0.1" or value["transport"] != "authenticated_loopback_tcp":
        raise RuntimeError("Broker資格fileの接続境界が不正")
    if not isinstance(value["port"], int) or not 1 <= value["port"] <= 65535:
        raise RuntimeError("Broker資格fileのportが不正")
    if not isinstance(value["session_secret"], str) or len(value["session_secret"]) != 64:
        raise RuntimeError("Broker資格fileのsecret形式が不正")
    return value


def directory_bytes(path: Path) -> int:
    total = 0
    if not path.exists():
        return total
    for item in path.rglob("*"):
        try:
            if item.is_file():
                total += item.stat().st_size
        except OSError:
            continue
    return total


def process_rss_bytes(pid: int) -> int | None:
    if os.name == "nt":
        class Counters(ctypes.Structure):
            _fields_ = [
                ("cb", ctypes.c_ulong),
                ("page_fault_count", ctypes.c_ulong),
                ("peak_working_set_size", ctypes.c_size_t),
                ("working_set_size", ctypes.c_size_t),
                ("quota_peak_paged_pool_usage", ctypes.c_size_t),
                ("quota_paged_pool_usage", ctypes.c_size_t),
                ("quota_peak_non_paged_pool_usage", ctypes.c_size_t),
                ("quota_non_paged_pool_usage", ctypes.c_size_t),
                ("pagefile_usage", ctypes.c_size_t),
                ("peak_pagefile_usage", ctypes.c_size_t),
            ]

        try:
            handle = ctypes.windll.kernel32.OpenProcess(0x1000 | 0x0400, False, pid)
            if not handle:
                return None
            counters = Counters()
            counters.cb = ctypes.sizeof(counters)
            ok = ctypes.windll.psapi.GetProcessMemoryInfo(
                handle, ctypes.byref(counters), ctypes.sizeof(counters)
            )
            ctypes.windll.kernel32.CloseHandle(handle)
            return int(counters.working_set_size) if ok else None
        except (AttributeError, OSError):
            return None
    statm = Path(f"/proc/{pid}/statm")
    try:
        pages = int(statm.read_text(encoding="ascii").split()[1])
        return pages * os.sysconf("SC_PAGE_SIZE")
    except (FileNotFoundError, IndexError, OSError, ValueError):
        return None


class RuntimeState:
    def __init__(self) -> None:
        self.lock = threading.Lock()
        self.request_count = 0
        self.http_request_count = 0
        self.http_paths: dict[str, int] = {}
        self.http_results: list[dict[str, Any]] = []
        self.traces: dict[str, dict[str, Any]] = {}

    def next_trace(self, session_id: str, message: str) -> tuple[str, str]:
        with self.lock:
            self.request_count += 1
            trace_id = f"{self.request_count:032x}"
            trace_hash = hashlib.sha256(f"{session_id}:{message}".encode("utf-8")).hexdigest()
            self.traces[trace_id] = {
                "追跡ID": trace_id,
                "セッションID": session_id,
                "ルートハッシュ": trace_hash,
            }
            return trace_id, trace_hash

    def record_http(self, path: str) -> None:
        with self.lock:
            self.http_request_count += 1
            self.http_paths[path] = self.http_paths.get(path, 0) + 1

    def record_http_result(self, path: str, status: int, detail: str = "") -> None:
        with self.lock:
            self.http_results.append({"path": path, "status": status, "detail": detail})
            if len(self.http_results) > 32:
                del self.http_results[:-32]


class ReusableHTTPServer(http.server.ThreadingHTTPServer):
    allow_reuse_address = True
    daemon_threads = True
    runtime_state: RuntimeState


class RuntimeHandler(http.server.BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"
    server: ReusableHTTPServer

    def _state(self) -> RuntimeState:
        return self.server.runtime_state

    def _send(self, body: dict[str, Any], status: int = 200) -> None:
        encoded = json_bytes(body)
        self._state().record_http_result(self.path, status)
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(encoded)))
        self.send_header("Connection", "close")
        self.end_headers()
        self.wfile.write(encoded)
        self.wfile.flush()
        time.sleep(0.005)

    def do_GET(self) -> None:
        state = self._state()
        state.record_http(self.path)
        if self.path == "/health":
            self._send({"ok": True, "api_version": "MINIDORA-PRODUCT-API-v1"})
            return
        if self.path == "/api/capabilities":
            self._send({"capabilities": ["chat", "trace"]})
            return
        if self.path.startswith("/api/trace/"):
            trace_id = self.path.rsplit("/", 1)[-1]
            with state.lock:
                trace = state.traces.get(trace_id)
            if trace is None:
                self._send({"valid": False}, status=404)
            else:
                self._send({"valid": True, "trace": trace})
            return
        self._send({"error": "not_found"}, status=404)

    def do_POST(self) -> None:
        self._state().record_http(self.path)
        if self.path != "/api/chat":
            self._send({"error": "not_found"}, status=404)
            return
        try:
            length = int(self.headers.get("Content-Length", "-1"))
            raw = self.rfile.read(length)
            request = json.loads(raw.decode("utf-8"))
            session_id = request["session_id"]
            message = request["message"]
            if not isinstance(session_id, str) or not isinstance(message, str):
                raise ValueError
        except (KeyError, TypeError, ValueError, UnicodeDecodeError):
            self._send({"error": "invalid_request"}, status=400)
            return
        trace_id, trace_hash = self._state().next_trace(session_id, message)
        self._send(
            {
                "session_id": session_id,
                "status": "合格",
                "response": f"long-run response {len(message)}",
                "route": "development-long-run-fixture",
                "capabilities": ["chat", "trace"],
                "trace_id": trace_id,
                "trace_hash": trace_hash,
                "sources": [],
            }
        )

    def log_message(self, _format: str, *_args: Any) -> None:
        return


class RuntimeServer:
    def __init__(self) -> None:
        self.state = RuntimeState()
        self.httpd: ReusableHTTPServer | None = None
        self.thread: threading.Thread | None = None
        self.port: int | None = None

    def start(self) -> None:
        if self.httpd is not None:
            raise RuntimeError("Runtime fixtureが起動済み")
        server = ReusableHTTPServer(("127.0.0.1", self.port or 0), RuntimeHandler)
        server.runtime_state = self.state
        self.port = int(server.server_address[1])
        self.httpd = server
        self.thread = threading.Thread(target=server.serve_forever, name="c28-runtime", daemon=True)
        self.thread.start()

    def stop(self) -> None:
        if self.httpd is None:
            return
        server, thread = self.httpd, self.thread
        self.httpd = None
        self.thread = None
        server.shutdown()
        server.server_close()
        if thread is not None:
            thread.join(timeout=5)

    def probe(self) -> None:
        if self.port is None:
            raise RuntimeError("Runtime fixtureのportが未設定")
        with socket.create_connection(("127.0.0.1", self.port), timeout=2) as sock:
            sock.sendall(b"GET /health HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
            response = sock.recv(4096)
        if b"200" not in response:
            raise RuntimeError(f"Runtime fixture health probeが不正: {response!r}")
        time.sleep(0.02)


class BrokerClient:
    def __init__(self, process: subprocess.Popen[bytes], endpoint: dict[str, Any], prefix: str) -> None:
        self.process = process
        self.endpoint = endpoint
        self.prefix = prefix
        self.counter = 0

    def request(self, operation: str, payload: Any = None) -> dict[str, Any]:
        self.counter += 1
        request_token = f"{self.prefix}-{uuid.uuid4().hex}"
        request: dict[str, Any] = {
            "request_id": request_token,
            "session_id": self.endpoint["session_id"],
            "operation": operation,
            "payload_hash": payload_hash(payload),
            "nonce": f"{request_token}-nonce",
            "issued_at": now_text(),
            "metadata": {"client": "c28_long_run_validation"},
        }
        if payload is not None:
            request["payload"] = payload
        with socket.create_connection(
            (self.endpoint["host"], self.endpoint["port"]), timeout=10
        ) as sock:
            sock.sendall(self.endpoint["session_secret"].encode("utf-8") + b"\n")
            sock.sendall(json_bytes(request) + b"\n")
            line = sock.makefile("rb").readline()
        if not line:
            raise RuntimeError(f"Brokerが空応答を返した: {operation}")
        return json.loads(line.decode("utf-8"))

    def accepted(self, operation: str, payload: Any = None) -> dict[str, Any]:
        response = self.request(operation, payload)
        if response.get("status") != "accepted":
            raise RuntimeError(f"Brokerが{operation}を拒否: {response}")
        return response

    def shutdown(self) -> None:
        try:
            self.request("shutdown")
        except (OSError, RuntimeError, json.JSONDecodeError):
            pass


@dataclass
class Statistics:
    dialogue_success: int = 0
    dialogue_failure: int = 0
    expected_disconnect_failures: int = 0
    history_reads: int = 0
    lifecycle_restarts: int = 0
    broker_restarts: int = 0
    runtime_restarts: int = 0
    reconnects: int = 0
    latency_count: int = 0
    latency_total_ms: float = 0.0
    latency_max_ms: float = 0.0
    samples: list[dict[str, Any]] = field(default_factory=list)

    def add_latency(self, elapsed_ms: float) -> None:
        self.latency_count += 1
        self.latency_total_ms += elapsed_ms
        self.latency_max_ms = max(self.latency_max_ms, elapsed_ms)

    def as_dict(self) -> dict[str, Any]:
        rss = [
            sample["Broker working set bytes"]
            for sample in self.samples
            if isinstance(sample.get("Broker working set bytes"), int)
        ]
        stores = [
            sample["store bytes"]
            for sample in self.samples
            if isinstance(sample.get("store bytes"), int)
        ]
        return {
            "対話成功数": self.dialogue_success,
            "対話失敗数": self.dialogue_failure,
            "接続断で想定した失敗数": self.expected_disconnect_failures,
            "履歴bounded読取数": self.history_reads,
            "Runtime相当再起動数": self.runtime_restarts,
            "Runtime lifecycle再起動数": self.lifecycle_restarts,
            "Broker再起動数": self.broker_restarts,
            "再接続数": self.reconnects,
            "対話応答平均Millis": round(self.latency_total_ms / self.latency_count, 2)
            if self.latency_count
            else None,
            "対話応答最大Millis": round(self.latency_max_ms, 2) if self.latency_count else None,
            "Broker working set最小bytes": min(rss) if rss else None,
            "Broker working set最大bytes": max(rss) if rss else None,
            "store bytes最小": min(stores) if stores else None,
            "store bytes最大": max(stores) if stores else None,
            "資源sample": self.samples,
        }


class Report:
    def __init__(self, output: Path | None) -> None:
        self.output = output
        self.started_at = now_text()
        self.status = "running"
        self.error: str | None = None
        self.body: dict[str, Any] = {}

    def write(self) -> None:
        if self.output is None:
            return
        self.output.parent.mkdir(parents=True, exist_ok=True)
        temporary = self.output.with_suffix(self.output.suffix + ".tmp")
        temporary.write_text(json.dumps(self.body, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        temporary.replace(self.output)


def build_helper() -> None:
    result = subprocess.run(
        ["cargo", "build", "--locked", "--quiet"],
        cwd=HELPER_ROOT,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        check=False,
    )
    if result.returncode != 0 or not HELPER_BINARY.exists():
        detail = result.stderr.decode("utf-8", errors="replace")
        raise RuntimeError(f"Rust helper buildが失敗した: {result.returncode}: {detail}")


class LongRun:
    def __init__(self, duration: float, interval: float, report: Report) -> None:
        self.duration = duration
        self.interval = interval
        self.report = report
        self.runtime = RuntimeServer()
        self.temp_root = Path(tempfile.mkdtemp(prefix="gui-shell-c28-"))
        self.store = self.temp_root / "store"
        self.session = self.temp_root / "broker.json"
        self.owner_session = self.temp_root / "owner.json"
        self.stderr = self.temp_root / "broker.stderr"
        self.broker: BrokerClient | None = None
        self.stats = Statistics()
        self.request_prefix = f"c28-{uuid.uuid4().hex}"
        self.started_monotonic = 0.0
        self.next_sample = 0.0
        self.last_phase = "準備"

    def _paths_for_restart(self) -> None:
        for path in (self.session, self.owner_session):
            try:
                path.unlink()
            except FileNotFoundError:
                pass

    def start_broker(self) -> None:
        self._paths_for_restart()
        stderr = self.stderr.open("ab")
        process = subprocess.Popen(
            [
                str(HELPER_BINARY),
                "broker-server",
                "--store-dir",
                str(self.store),
                "--session-file",
                str(self.session),
                "--owner-session-file",
                str(self.owner_session),
                "--minidora-runtime",
                f"long-run-runtime=127.0.0.1:{self.runtime.port}",
                "--enable-development-lifecycle-fixture",
                "--max-request-bytes",
                str(64 * 1024),
            ],
            cwd=HELPER_ROOT,
            stdout=subprocess.DEVNULL,
            stderr=stderr,
        )
        try:
            endpoint = require_endpoint(wait_for_file(self.session, process))
            require_endpoint(wait_for_file(self.owner_session, process))
        except BaseException:
            if process.poll() is None:
                process.kill()
                process.wait(timeout=5)
            stderr.close()
            raise
        stderr.close()
        self.broker = BrokerClient(process, endpoint, self.request_prefix)

    def stop_broker(self) -> None:
        broker = self.broker
        self.broker = None
        if broker is None:
            return
        broker.shutdown()
        try:
            broker.process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            broker.process.kill()
            broker.process.wait(timeout=5)

    def owner_endpoint(self) -> dict[str, Any]:
        return require_endpoint(json.loads(self.owner_session.read_text(encoding="utf-8")))

    def owner_request(self, operation: str, payload: Any = None) -> dict[str, Any]:
        endpoint = self.owner_endpoint()
        request_id = f"c28-owner-{int(time.monotonic_ns())}"
        request: dict[str, Any] = {
            "request_id": request_id,
            "session_id": endpoint["session_id"],
            "operation": operation,
            "payload_hash": payload_hash(payload),
            "nonce": f"{request_id}-nonce",
            "issued_at": now_text(),
            "metadata": {"client": "c28_long_run_validation_owner"},
        }
        if payload is not None:
            request["payload"] = payload
        with socket.create_connection((endpoint["host"], endpoint["port"]), timeout=10) as sock:
            sock.sendall(endpoint["session_secret"].encode("utf-8") + b"\n")
            sock.sendall(json_bytes(request) + b"\n")
            line = sock.makefile("rb").readline()
        return json.loads(line.decode("utf-8"))

    def owner_lifecycle_approve(self, pending: dict[str, Any]) -> None:
        result = subprocess.run(
            [
                str(HELPER_BINARY),
                "実行系ライフサイクル承認",
                "--session-file",
                str(self.owner_session),
                "承認",
                str(pending["承認ID"]),
                str(pending["承認hash"]),
            ],
            cwd=HELPER_ROOT,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
        if result.returncode != 0:
            detail = result.stderr.decode("utf-8", errors="replace")
            raise RuntimeError(f"owner lifecycle承認が失敗した: {detail}")
        approved = json.loads(result.stdout.decode("utf-8"))
        if approved.get("状態") != "approved":
            raise RuntimeError("owner lifecycle承認がapprovedにならない")

    def lifecycle_restart(self) -> None:
        broker = self._broker()
        status = broker.accepted(
            "実行系ライフサイクル状態",
            {"版": 1, "実行系ID": "development-lifecycle-fixture"},
        )["body"].get("状態")
        if status == "stopped":
            action = "start"
        elif status == "paused":
            action = "resume"
        elif status == "ready":
            action = "restart"
        else:
            raise RuntimeError(f"Runtime lifecycle状態が再起動可能でない: {status}")
        pending_response = broker.accepted(
            "実行系ライフサイクル承認要求",
            {"版": 1, "実行系ID": "development-lifecycle-fixture", "操作": action},
        )
        pending = pending_response["body"]
        self.owner_lifecycle_approve(pending)
        result = broker.accepted(
            "実行系ライフサイクル操作",
            {
                "版": 1,
                "実行系ID": "development-lifecycle-fixture",
                "操作": action,
                "承認ID": pending["承認ID"],
            },
        )
        if result["body"].get("統治", {}).get("承認状態") != "consumed":
            raise RuntimeError("Runtime lifecycle再起動の承認がconsumedにならない")
        if action == "restart":
            self.stats.lifecycle_restarts += 1

    def _broker(self) -> BrokerClient:
        if self.broker is None:
            raise RuntimeError("Brokerが起動していない")
        if self.broker.process.poll() is not None:
            raise RuntimeError(f"Brokerが予期せず終了した: {self.broker.process.returncode}")
        return self.broker

    def dialogue(self, expected_failure: bool = False) -> bool:
        broker = self._broker()
        started = time.perf_counter()
        session_response = broker.accepted("対話開始", {"実行系ID": "long-run-runtime"})
        session_id = session_response["body"]["対話セッションID"]
        message = f"C28 long-run dialogue {self.stats.dialogue_success + self.stats.dialogue_failure + 1:08d}"
        submitted = broker.accepted(
            "対話送信", {"対話セッションID": session_id, "入力": message}
        )
        body = submitted["body"]
        request_id = body["要求ID"]
        request_hash = body["要求hash"]
        approval = self.owner_request(
            "対話承認",
            {"要求ID": request_id, "要求hash": request_hash, "表示範囲": "summary"},
        )
        if approval.get("status") != "accepted":
            raise RuntimeError(f"owner対話承認が拒否された: {approval}")
        result: dict[str, Any] | None = None
        deadline = time.monotonic() + 20
        while time.monotonic() < deadline:
            response = broker.accepted("対話取得", {"要求ID": request_id})
            state = response["body"].get("状態")
            if state == "完了":
                result = response["body"]
                break
            time.sleep(0.02)
        if result is None:
            raise TimeoutError("長時間試験の対話完了待機がtimeoutした")
        broker.accepted("対話終了", {"対話セッションID": session_id})
        success = result.get("結果", {}).get("状態") == "成功"
        elapsed_ms = (time.perf_counter() - started) * 1000
        if not expected_failure and not success:
            raise RuntimeError(f"対話が成功にならない: {json.dumps(result, ensure_ascii=False)}")
        if expected_failure and success:
            raise RuntimeError("接続断中の対話が成功へ昇格した")
        if success:
            self.stats.dialogue_success += 1
            self.stats.add_latency(elapsed_ms)
        else:
            self.stats.dialogue_failure += 1
            if expected_failure:
                self.stats.expected_disconnect_failures += 1
        return success

    def history_read(self) -> None:
        response = self.owner_request("対話履歴一覧", {"after": 0, "limit": 100})
        if response.get("status") != "accepted":
            raise RuntimeError(f"履歴bounded読取が拒否された: {response}")
        entries = response.get("body", {}).get("entries", [])
        if not isinstance(entries, list) or len(entries) > 100:
            raise RuntimeError("履歴bounded読取の上限が守られない")
        self.stats.history_reads += 1

    def sample(self, phase: str) -> None:
        broker_pid = self.broker.process.pid if self.broker is not None else None
        rss = process_rss_bytes(broker_pid) if broker_pid else None
        sample = {
            "時刻": now_text(),
            "経過Millis": round((time.monotonic() - self.started_monotonic) * 1000),
            "段階": phase,
            "Broker PID": broker_pid,
            "Broker working set bytes": rss,
            "store bytes": directory_bytes(self.store),
            "Broker process alive": bool(self.broker and self.broker.process.poll() is None),
            "Runtime HTTP request count": self.runtime.state.http_request_count,
            "Runtime HTTP paths": dict(self.runtime.state.http_paths),
            "Runtime HTTP results": list(self.runtime.state.http_results),
        }
        if len(self.stats.samples) < MAX_REPORT_SAMPLES:
            self.stats.samples.append(sample)
        else:
            self.stats.samples[-1] = sample

    def maybe_sample(self, phase: str) -> None:
        now = time.monotonic()
        if now >= self.next_sample:
            self.sample(phase)
            self.next_sample = now + max(1.0, min(60.0, self.interval * 10))
            self.refresh_report()

    def refresh_report(self) -> None:
        self.report.body = {
            "版": 1,
            "状態": self.report.status,
            "証拠範囲": ["LIVE_RUNTIME", "FIXTURE", "INTERNAL_STATE"],
            "Runtime実装": "ローカルMINIDORA API fixture",
            "開始時刻": self.report.started_at,
            "更新時刻": now_text(),
            "経過秒": round(max(0.0, time.monotonic() - self.started_monotonic), 3)
            if self.started_monotonic
            else 0,
            "統計": self.stats.as_dict(),
            "段階": self.last_phase,
            "エラー": self.report.error,
            "installed製品の証明ではない": True,
            "Runtime fixture観測": {
                "HTTP request count": self.runtime.state.http_request_count,
                "HTTP paths": dict(self.runtime.state.http_paths),
                "HTTP results": list(self.runtime.state.http_results),
            },
        }
        self.report.write()

    def run(self) -> None:
        build_helper()
        self.runtime.start()
        self.runtime.probe()
        self.start_broker()
        self.started_monotonic = time.monotonic()
        self.next_sample = self.started_monotonic
        deadline = self.started_monotonic + self.duration
        self.refresh_report()

        self.last_phase = "Runtime lifecycle start"
        broker = self._broker()
        pending = broker.accepted(
            "実行系ライフサイクル承認要求",
            {"版": 1, "実行系ID": "development-lifecycle-fixture", "操作": "start"},
        )["body"]
        self.owner_lifecycle_approve(pending)
        broker.accepted(
            "実行系ライフサイクル操作",
            {
                "版": 1,
                "実行系ID": "development-lifecycle-fixture",
                "操作": "start",
                "承認ID": pending["承認ID"],
            },
        )

        phase_actions = [
            ("大量対話", lambda: self.dialogue()),
            ("Runtime相当再起動", self.restart_runtime_adapter),
            ("Runtime lifecycle再起動", self.lifecycle_restart),
            ("Broker再起動", self.restart_broker),
            ("network切断", self.disconnect_and_reconnect),
        ]
        action_index = 0
        while time.monotonic() < deadline:
            self.last_phase = phase_actions[action_index % len(phase_actions)][0]
            action = phase_actions[action_index % len(phase_actions)][1]
            action()
            action_index += 1
            if self.stats.dialogue_success and self.stats.dialogue_success % 10 == 0:
                self.history_read()
            self.maybe_sample(self.last_phase)
            remaining = deadline - time.monotonic()
            if remaining > 0:
                time.sleep(min(self.interval, remaining))
        self.last_phase = "完了確認"
        self.history_read()
        self.sample(self.last_phase)
        self.report.status = "passed"
        self.refresh_report()

    def restart_runtime_adapter(self) -> None:
        self.runtime.stop()
        time.sleep(0.05)
        self.runtime.start()
        self.runtime.probe()
        self.stats.runtime_restarts += 1
        self.stats.reconnects += 1
        self.dialogue()

    def restart_broker(self) -> None:
        self.stop_broker()
        time.sleep(0.05)
        self.start_broker()
        self.runtime.probe()
        self.stats.broker_restarts += 1
        self.dialogue()

    def disconnect_and_reconnect(self) -> None:
        self.runtime.stop()
        try:
            self.dialogue(expected_failure=True)
        finally:
            self.runtime.start()
            self.runtime.probe()
        self.stats.reconnects += 1
        self.dialogue()

    def close(self) -> None:
        try:
            self.stop_broker()
        finally:
            self.runtime.stop()
            shutil.rmtree(self.temp_root, ignore_errors=True)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="C28長時間運用の開発用検証")
    parser.add_argument("--duration-seconds", type=float, default=30.0)
    parser.add_argument("--duration-hours", type=float)
    parser.add_argument("--interval-seconds", type=float, default=1.0)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    if args.duration_hours is not None:
        if args.duration_hours <= 0:
            parser.error("--duration-hoursは正数が必要")
        args.duration_seconds = args.duration_hours * 3600
    if args.duration_seconds < 5:
        parser.error("長時間試験の最短時間は5秒")
    if args.interval_seconds <= 0:
        parser.error("--interval-secondsは正数が必要")
    return args


def main() -> int:
    args = parse_args()
    report = Report(args.output)
    run = LongRun(args.duration_seconds, args.interval_seconds, report)
    try:
        run.run()
        print(json.dumps(compact_report(report.body), ensure_ascii=False, indent=2))
        return 0
    except BaseException as error:
        report.status = "failed"
        detail = ""
        if run.stderr.exists():
            detail = run.stderr.read_text(encoding="utf-8", errors="replace")[-4000:]
        report.error = str(error) + (f" / Broker stderr: {detail}" if detail else "")
        run.last_phase = run.last_phase or "失敗"
        run.sample(run.last_phase)
        run.refresh_report()
        print(json.dumps(compact_report(report.body), ensure_ascii=False, indent=2), file=sys.stderr)
        return 1
    finally:
        run.close()


def compact_report(body: dict[str, Any]) -> dict[str, Any]:
    compact = dict(body)
    statistics = dict(compact.get("統計", {}))
    statistics.pop("資源sample", None)
    compact["統計"] = statistics
    fixture = dict(compact.get("Runtime fixture観測", {}))
    fixture.pop("HTTP results", None)
    compact["Runtime fixture観測"] = fixture
    return compact


if __name__ == "__main__":
    raise SystemExit(main())
