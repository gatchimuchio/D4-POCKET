"""C29 障害注入の開発用検証器。\n\n実Broker IPCと、現行Brokerへ接続可能なMCP stdio／A2A loopback fixtureを使い、\n障害を成功・権限・監査・復旧へ昇格させないことを確認する。installed製品、\n外部Runtime、外部MCP／A2Aの障害証拠へ、この検証器の結果を昇格させてはならない。\n"""

from __future__ import annotations

import http.server
import json
import shutil
import socketserver
import sys
import threading
import time
from pathlib import Path
from typing import Any, Callable

from long_run_validation import LongRun, Report, build_helper, now_text

ROOT = Path(__file__).resolve().parents[1]
MCP_TIMEOUT_SECONDS = 5.0


class StallHTTPServer(socketserver.ThreadingTCPServer):
    allow_reuse_address = True
    daemon_threads = True
    started: threading.Event


class StallHandler(http.server.BaseHTTPRequestHandler):
    """A2Aの応答期限を超過させるloopback fixture。"""

    server: StallHTTPServer

    def do_GET(self) -> None:
        self.server.started.set()
        time.sleep(MCP_TIMEOUT_SECONDS + 1.0)

    def log_message(self, _format: str, *_args: Any) -> None:
        return


class StallServer:
    """外部A2A相当の応答停止をboundedに提供する。"""

    def __init__(self) -> None:
        self.httpd = StallHTTPServer(("127.0.0.1", 0), StallHandler)
        self.started = threading.Event()
        self.thread = threading.Thread(
            target=self.httpd.serve_forever,
            name="c29-a2a-stall",
            daemon=True,
        )
        self.httpd.started = self.started
        self.thread.start()

    @property
    def port(self) -> int:
        return int(self.httpd.server_address[1])

    def close(self) -> None:
        self.httpd.shutdown()
        self.httpd.server_close()
        self.thread.join(timeout=5)

def error_code(response: dict[str, Any]) -> str | None:
    error = response.get("error")
    return error.get("code") if isinstance(error, dict) else None


def credential_ref(required: bool = False) -> dict[str, Any]:
    return {
        "credential_id": "a" * 32,
        "purpose": "C29障害注入",
        "target": "development-fixture",
        "required": required,
        "status": "missing",
    }


def mcp_payload(executable: Path, fixture: Path, required: bool = False) -> dict[str, Any]:
    return {
        "版": 1,
        "操作": "接続",
        "ServerID": "c29-mcp-timeout" if not required else "c29-mcp-credential",
        "実行file": str(executable),
        "引数": [str(fixture), "timeout"],
        "workspace": str(fixture.parent),
        "Transport": "stdio",
        "Credential ref": credential_ref(required),
    }


def a2a_payload(port: int) -> dict[str, Any]:
    return {
        "版": 1,
        "操作": "接続",
        "AgentID": "c29-a2a-timeout",
        "Agent Card URI": f"http://127.0.0.1:{port}/.well-known/agent-card.json",
        "protocol_version": "1.0",
        "Transport": "http",
        "Credential ref": credential_ref(False),
    }


def start_harness() -> LongRun:
    harness = LongRun(1.0, 1.0, Report(None))
    harness.runtime.start()
    harness.runtime.probe()
    harness.start_broker()
    return harness


def close_harness(harness: LongRun | None) -> None:
    if harness is None:
        return
    try:
        harness.close()
    except (OSError, RuntimeError, TimeoutError):
        harness.runtime.stop()


def case_runtime_crash() -> dict[str, Any]:
    harness: LongRun | None = None
    try:
        harness = start_harness()
        harness.runtime.stop()
        if harness.dialogue(expected_failure=True):
            raise RuntimeError("Runtime停止中の対話が成功した")
        harness.runtime.start()
        harness.runtime.probe()
        harness.dialogue()
        return {"観測": "通信失敗後の再接続成功", "証拠": "LIVE_RUNTIME/FIXTURE"}
    finally:
        close_harness(harness)


def case_broker_crash() -> dict[str, Any]:
    harness: LongRun | None = None
    try:
        harness = start_harness()
        broker = harness.broker
        if broker is None:
            raise RuntimeError("Brokerがない")
        broker.process.kill()
        broker.process.wait(timeout=5)
        harness.broker = None
        try:
            broker.request("health")
        except (ConnectionError, OSError, RuntimeError, json.JSONDecodeError):
            pass
        else:
            raise RuntimeError("Broker crash後のIPCが成功した")
        try:
            harness.start_broker()
        except RuntimeError:
            return {"観測": "Broker crash後のactive session復旧を拒否", "証拠": "LIVE_RUNTIME/INTERNAL_STATE"}
        response = harness.broker.request("health") if harness.broker else {}
        if response.get("status") != "accepted":
            raise RuntimeError("Broker crash後の明示再起動healthが成功しない")
        return {"観測": "Broker crash後のIPC断と明示再起動health", "証拠": "LIVE_RUNTIME/INTERNAL_STATE"}
    finally:
        close_harness(harness)


def write_mcp_fixture(path: Path) -> None:
    path.write_text(
        """import json
import sys
import time

for line in sys.stdin:
    request = json.loads(line)
    if request.get("method") == "server/discover":
        response = {
            "jsonrpc": "2.0",
            "id": request.get("id"),
            "result": {
                "supportedVersions": ["2026-07-28"],
                "capabilities": {"tools": {}},
            },
        }
        sys.stdout.write(json.dumps(response) + "\\n")
        sys.stdout.flush()
    else:
        time.sleep(6)
""",
        encoding="utf-8",
    )


def case_mcp_timeout() -> dict[str, Any]:
    harness: LongRun | None = None
    try:
        harness = start_harness()
        fixture = harness.temp_root / "mcp_timeout.py"
        write_mcp_fixture(fixture)
        response = harness.owner_request(
            "MCP接続", mcp_payload(Path(sys.executable), fixture)
        )
        if response.get("status") != "rejected" or error_code(response) != "mcp_timeout":
            raise RuntimeError("MCP timeoutが拒否へ射影されない")
        return {"観測": "MCP timeoutをrejectedへ射影", "証拠": "LIVE_RUNTIME/FIXTURE"}
    finally:
        close_harness(harness)


def case_a2a_timeout() -> dict[str, Any]:
    harness: LongRun | None = None
    server: StallServer | None = None
    try:
        harness = start_harness()
        server = StallServer()
        response = harness.owner_request("A2A接続", a2a_payload(server.port))
        if response.get("status") != "rejected" or error_code(response) != "a2a_timeout":
            raise RuntimeError("A2A timeoutが拒否へ射影されない")
        return {"観測": "A2A timeoutをrejectedへ射影", "証拠": "LIVE_RUNTIME/FIXTURE"}
    finally:
        if server is not None:
            server.close()
        close_harness(harness)


def case_credential_unavailable() -> dict[str, Any]:
    harness: LongRun | None = None
    try:
        harness = start_harness()
        fixture = harness.temp_root / "mcp_credential.py"
        fixture.write_text("", encoding="utf-8")
        response = harness.owner_request(
            "MCP接続", mcp_payload(Path(sys.executable), fixture, required=True)
        )
        if response.get("status") != "rejected" or error_code(response) != "mcp_credential_unavailable":
            raise RuntimeError("credential unavailableが拒否へ射影されない")
        return {"観測": "Credential unavailableを拒否", "証拠": "LIVE_RUNTIME/INTERNAL_STATE"}
    finally:
        close_harness(harness)


def case_disk_full_simulation() -> dict[str, Any]:
    harness: LongRun | None = None
    try:
        harness = LongRun(1.0, 1.0, Report(None))
        harness.runtime.start()
        harness.runtime.probe()
        harness.store.write_text("storage path is unavailable", encoding="utf-8")
        try:
            harness.start_broker()
        except RuntimeError:
            return {
                "観測": "store path書込不能を起動拒否",
                "証拠": "LIVE_RUNTIME/INTERNAL_STATE",
                "シミュレーション": "disk fullそのものではなくregular file置換",
            }
        raise RuntimeError("書込不能storeでBrokerが起動した")
    finally:
        close_harness(harness)


def case_audit_failure() -> dict[str, Any]:
    harness: LongRun | None = None
    restored = False
    try:
        harness = start_harness()
        audit_path = harness.store / "audit.jsonl"
        backup_path = harness.store / "audit.jsonl.c29-backup"
        audit_path.rename(backup_path)
        audit_path.mkdir()
        response = harness.broker.request("health") if harness.broker else {}
        if response.get("status") != "suspended" or error_code(response) != "broker_audit_append_failed":
            raise RuntimeError("監査書込失敗がsuspendedへ射影されない")
        shutil.rmtree(audit_path)
        backup_path.rename(audit_path)
        restored = True
        return {"観測": "Audit書込失敗をsuspendedへ射影", "証拠": "LIVE_RUNTIME/INTERNAL_STATE"}
    finally:
        if harness is not None and not restored:
            audit_path = harness.store / "audit.jsonl"
            backup_path = harness.store / "audit.jsonl.c29-backup"
            if audit_path.is_dir():
                shutil.rmtree(audit_path)
            if backup_path.exists():
                backup_path.rename(audit_path)
        close_harness(harness)


def case_corrupt_state() -> dict[str, Any]:
    harness: LongRun | None = None
    try:
        harness = start_harness()
        harness.stop_broker()
        harness._paths_for_restart()
        (harness.store / "a2a_connections.json").write_text("{malformed", encoding="utf-8")
        try:
            harness.start_broker()
        except RuntimeError:
            return {"観測": "malformed stateのBroker再起動を拒否", "証拠": "LIVE_RUNTIME/INTERNAL_STATE"}
        raise RuntimeError("malformed stateでBrokerが起動した")
    finally:
        close_harness(harness)


def run_case(name: str, action: Callable[[], dict[str, Any]]) -> dict[str, Any]:
    started = time.perf_counter()
    try:
        detail = action()
        return {
            "名称": name,
            "状態": "passed",
            "経過Millis": round((time.perf_counter() - started) * 1000, 2),
            **detail,
        }
    except Exception as error:
        return {
            "名称": name,
            "状態": "failed",
            "経過Millis": round((time.perf_counter() - started) * 1000, 2),
            "エラー種別": type(error).__name__,
        }


def main() -> int:
    build_helper()
    cases = [
        ("Runtime crash相当", case_runtime_crash),
        ("Broker crash", case_broker_crash),
        ("MCP timeout", case_mcp_timeout),
        ("A2A timeout", case_a2a_timeout),
        ("credential unavailable", case_credential_unavailable),
        ("disk full simulation", case_disk_full_simulation),
        ("audit failure", case_audit_failure),
        ("corrupt state", case_corrupt_state),
    ]
    results = [run_case(name, action) for name, action in cases]
    failed = [result for result in results if result["状態"] != "passed"]
    body = {
        "版": 1,
        "状態": "failed" if failed else "passed",
        "証拠範囲": ["LIVE_RUNTIME", "FIXTURE", "INTERNAL_STATE"],
        "開始時刻": now_text(),
        "更新時刻": now_text(),
        "試験": results,
        "installed製品の証明ではない": True,
        "外部Runtime・外部MCP・外部A2Aの障害証拠ではない": True,
    }
    print(json.dumps(body, ensure_ascii=False, indent=2))
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
