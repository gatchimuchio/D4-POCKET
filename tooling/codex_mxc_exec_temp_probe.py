"""実Codex CLI/MxCの一時領域と合成filesystem境界を調べるdevelopment-only probe。

loopbackの偽Responses APIから固定exec_commandを一度だけ返す。実モデル、資格情報、
Broker、Owner Approval、製品Task経路は使わず、task_executionを有効化しない。
"""

from __future__ import annotations

import argparse
import json
import os
import re
import stat
import subprocess
import tempfile
import threading
import uuid
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any
from urllib.parse import urlsplit


RUN_TIMEOUT_SECONDS = 60
MAX_RUNS = 3
MOCK_MODEL = "d4p-local-probe"

# ここはCodex CLIへ渡す機械設定と、隔離したshell child内だけで動く固定命令。
_BASE_OVERRIDES = (
    f'model="{MOCK_MODEL}"',
    'model_provider="d4p_loopback_probe"',
    'model_providers.d4p_loopback_probe.name="D4 Pocket loopback probe"',
    'model_providers.d4p_loopback_probe.base_url="__D4P_BASE_URL__"',
    'model_providers.d4p_loopback_probe.wire_api="responses"',
    "model_providers.d4p_loopback_probe.requires_openai_auth=false",
    "model_providers.d4p_loopback_probe.request_max_retries=0",
    "model_providers.d4p_loopback_probe.stream_max_retries=0",
    'approval_policy="never"',
    'default_permissions="d4p-agent-task"',
    'windows.sandbox="mxc"',
    'permissions.d4p-agent-task.extends=":workspace"',
    (
        'permissions.d4p-agent-task.filesystem={":root"="deny",":minimal"="read",'
        '":workspace_roots"={"**/*.env"="deny","**/.env.*"="deny",'
        '"**/.ssh/**"="deny","**/secrets/**"="deny",'
        '"private/registered-marker.txt"="deny",'
        '"private/registered-write-target.txt"="deny",'
        '"private/registered-directory"="deny",'
        '"private/registered-directory/**"="deny"},"glob_scan_max_depth"=32}'
    ),
    "permissions.d4p-agent-task.network.enabled=false",
)
_TASK_PROBE_COMMAND_TEMPLATE = (
    "$ErrorActionPreference='Stop'; "
    "$scratch=[IO.Path]::GetFullPath((Join-Path (Get-Location).Path '.d4p-tmp-probe')); "
    "$workspace=[IO.Path]::GetFullPath((Get-Location).Path); "
    "$outsideRoot=[IO.Directory]::GetParent($workspace).FullName; "
    "$tempPath=[IO.Path]::GetFullPath($env:TEMP); "
    "$tmpPath=[IO.Path]::GetFullPath($env:TMP); "
    "$tempMatches=[string]::Equals($tempPath,$scratch,[StringComparison]::OrdinalIgnoreCase); "
    "$tmpMatches=[string]::Equals($tmpPath,$scratch,[StringComparison]::OrdinalIgnoreCase); "
    "[IO.Directory]::CreateDirectory($scratch) | Out-Null; "
    "[IO.File]::WriteAllText((Join-Path $scratch 'scratch-marker.txt'),'synthetic'); "
    "$readProbe={param($path) try {$null=[IO.File]::ReadAllText($path); 'allowed'} "
    "catch {'blocked:'+$_.Exception.GetType().Name}}; "
    "$writeProbe={param($path) try {[IO.File]::WriteAllText($path,'synthetic-write'); 'allowed'} "
    "catch {'blocked:'+$_.Exception.GetType().Name}}; "
    "$probeStatuses=@{workspaceControlRead=(& $readProbe (Join-Path $workspace 'probe-inputs\\ordinary-marker.txt')); "
    "envRead=(& $readProbe (Join-Path $workspace 'probe-inputs\\config.env')); "
    "dotenvRead=(& $readProbe (Join-Path $workspace 'probe-inputs\\.env.production')); "
    "sshRead=(& $readProbe (Join-Path $workspace '.ssh\\id_ed25519')); "
    "secretsRead=(& $readProbe (Join-Path $workspace 'secrets\\secret.txt')); "
    "registeredFileRead=(& $readProbe (Join-Path $workspace 'private\\registered-marker.txt')); "
    "registeredDirectoryRead=(& $readProbe (Join-Path $workspace 'private\\registered-directory\\read-marker.txt')); "
    "outsideWorkspaceRead=(& $readProbe (Join-Path $outsideRoot 'outside-read-marker.txt')); "
    "envWrite=(& $writeProbe (Join-Path $workspace 'probe-inputs\\write-probe.env')); "
    "dotenvWrite=(& $writeProbe (Join-Path $workspace 'probe-inputs\\.env.write-probe')); "
    "sshWrite=(& $writeProbe (Join-Path $workspace '.ssh\\write-probe')); "
    "secretsWrite=(& $writeProbe (Join-Path $workspace 'secrets\\write-probe')); "
    "registeredFileWrite=(& $writeProbe (Join-Path $workspace 'private\\registered-write-target.txt')); "
    "registeredDirectoryWrite=(& $writeProbe (Join-Path $workspace 'private\\registered-directory\\write-probe.txt')); "
    "outsideWorkspaceWrite=(& $writeProbe (Join-Path $outsideRoot 'outside-write-marker.txt'))}; "
    "$hardlinkAlias=Join-Path $workspace 'private\\registered-hardlink-alias.txt'; "
    "$hardlinkCreateStatus='not_attempted'; "
    "try {$null=New-Item -ItemType HardLink -Path $hardlinkAlias "
    "-Target (Join-Path $workspace 'private\\registered-marker.txt') -ErrorAction Stop; "
    "$hardlinkCreateStatus='created'} catch {$hardlinkError=$_.Exception; "
    "$hardlinkHresult=[Convert]::ToString([BitConverter]::ToUInt32("
    "[BitConverter]::GetBytes([int]$hardlinkError.HResult),0),16).PadLeft(8,'0').ToUpperInvariant(); "
    "$hardlinkNativeCode=''; if ($hardlinkError -is [ComponentModel.Win32Exception]) "
    "{$hardlinkNativeCode=':native:'+$hardlinkError.NativeErrorCode}; "
    "$hardlinkCreateStatus='blocked:'+$hardlinkError.GetType().Name+':0x'+$hardlinkHresult+$hardlinkNativeCode}; "
    "if ($hardlinkCreateStatus -eq 'created') {try {$null=[IO.File]::ReadAllText($hardlinkAlias); "
    "$hardlinkCreateStatus='created:read_allowed'} catch {$hardlinkCreateStatus='created:read_blocked'}}; "
    "$markerPath=Join-Path $tempPath 'd4p-probe-__D4P_NONCE__.tmp'; "
    "$reportPath=Join-Path (Get-Location).Path 'probe-report.json'; "
    "$report=@{tempPath=$tempPath;tmpPath=$tmpPath;markerPath=$markerPath;"
    "tempMatchesScratch=$tempMatches;tmpMatchesScratch=$tmpMatches;"
    "probeStatuses=$probeStatuses;hardlinkCreateStatus=$hardlinkCreateStatus;"
    "scratchMarkerExists=[IO.File]::Exists((Join-Path $scratch 'scratch-marker.txt'))}; "
    "[IO.File]::WriteAllText($reportPath,($report | ConvertTo-Json -Compress)); "
    "$markerWrite='failed'; try {[IO.File]::WriteAllText($markerPath,'codex-exec-temp-probe'); "
    "$markerWrite='written'} catch {$markerWrite=$_.Exception.GetType().Name}; "
    "$markerVisible=[IO.File]::Exists($markerPath); "
    "[IO.File]::WriteAllText((Join-Path (Get-Location).Path 'temp-write-status.txt'),"
    "($markerWrite+'|'+$markerVisible)); exit 0"
)


def _event(event_type: str, response_id: str, item: dict[str, Any] | None = None) -> dict[str, Any]:
    if item is not None:
        return {"type": "response.output_item.done", "item": item}
    if event_type == "response.created":
        return {"type": event_type, "response": {"id": response_id}}
    return {
        "type": "response.completed",
        "response": {
            "id": response_id,
            "usage": {
                "input_tokens": 1,
                "input_tokens_details": None,
                "output_tokens": 1,
                "output_tokens_details": None,
                "total_tokens": 2,
            },
        },
    }


def _sse(events: list[dict[str, Any]]) -> bytes:
    return "".join(
        f"event: {event['type']}\ndata: {json.dumps(event, ensure_ascii=False)}\n\n"
        for event in events
    ).encode("utf-8")


class _MockState:
    def __init__(self) -> None:
        self.lock = threading.Lock()
        self.request_count = 0
        self.blocked_external_requests = 0
        self.blocked_external_categories: dict[str, int] = {}
        self.response_write_completed = 0
        self.response_write_errors = 0
        self.exec_command_exposed = False
        self.command = ""
        self.workdir = ""

    def begin_run(self, command: str, workdir: Path) -> None:
        with self.lock:
            self.request_count = 0
            self.exec_command_exposed = False
            self.command = command
            self.workdir = str(workdir)
            self.response_write_completed = 0
            self.response_write_errors = 0


class _MockResponsesServer(ThreadingHTTPServer):
    daemon_threads = True
    allow_reuse_address = True

    def __init__(self) -> None:
        super().__init__(("127.0.0.1", 0), _MockResponsesHandler)
        self.state = _MockState()


class _MockResponsesHandler(BaseHTTPRequestHandler):
    server: _MockResponsesServer

    def log_message(self, _format: str, *_args: object) -> None:
        return None

    def _is_local_mock_request(self) -> bool:
        if not self.path.startswith("/") or self.path.startswith("//"):
            return False
        try:
            authority = urlsplit(f"//{self.headers.get('host', '')}")
            return (
                authority.hostname in {"127.0.0.1", "localhost"}
                and authority.port == self.server.server_address[1]
            )
        except ValueError:
            return False

    def do_CONNECT(self) -> None:
        self._block_external()

    def do_GET(self) -> None:
        if not self._is_local_mock_request():
            self._block_external()
            return
        if self.path.endswith("/v1/models") or self.path.endswith("/models"):
            self._send_json(
                {
                    "object": "list",
                    "data": [
                        {
                            "id": MOCK_MODEL,
                            "object": "model",
                            "created": 0,
                            "owned_by": "local-probe",
                        }
                    ],
                }
            )
            return
        self._block_external()

    def do_POST(self) -> None:
        if not self._is_local_mock_request():
            self._block_external()
            return
        body = self.rfile.read(int(self.headers.get("content-length", "0")))
        if not self.path.endswith("/v1/responses") and not self.path.endswith("/responses"):
            self._block_external()
            return

        with self.server.state.lock:
            self.server.state.request_count += 1
            request_number = self.server.state.request_count
            if request_number == 1:
                try:
                    request = json.loads(body)
                    tools = request.get("tools", [])
                    self.server.state.exec_command_exposed = any(
                        isinstance(tool, dict) and tool.get("name") == "exec_command"
                        for tool in tools
                    )
                except (json.JSONDecodeError, TypeError):
                    self.server.state.exec_command_exposed = False
            can_call_tool = self.server.state.exec_command_exposed
            command = self.server.state.command
            workdir = self.server.state.workdir

        response_id = f"d4p-probe-{request_number}"
        events = [_event("response.created", response_id)]
        if request_number == 1 and can_call_tool:
            arguments = {
                "cmd": command,
                "workdir": workdir,
                "shell": "powershell.exe",
                "yield_time_ms": 30000,
            }
            events.append(
                _event(
                    "response.output_item.done",
                    response_id,
                    {
                        "type": "function_call",
                        "call_id": "d4p-temp-probe-call",
                        "name": "exec_command",
                        "arguments": json.dumps(arguments, ensure_ascii=False),
                    },
                )
            )
        else:
            events.append(
                _event(
                    "response.output_item.done",
                    response_id,
                    {
                        "type": "message",
                        "role": "assistant",
                        "id": f"d4p-probe-message-{request_number}",
                        "content": [{"type": "output_text", "text": "probe complete"}],
                    },
                )
            )
        events.append(_event("response.completed", response_id))
        payload = _sse(events)
        self.send_response(200)
        self.send_header("content-type", "text/event-stream")
        self.send_header("content-length", str(len(payload)))
        self.end_headers()
        try:
            self.wfile.write(payload)
            self.wfile.flush()
        except OSError:
            with self.server.state.lock:
                self.server.state.response_write_errors += 1
            return
        with self.server.state.lock:
            self.server.state.response_write_completed += 1

    def _block_external(self) -> None:
        if self.command.upper() == "CONNECT":
            category = "proxy_connect"
        else:
            path = urlsplit(self.path).path.casefold()
            if path.endswith("/analytics/codex/turn-costs"):
                category = "turn_costs_route"
            elif path.endswith("/responses"):
                category = "responses_route"
            elif path.endswith("/models"):
                category = "models_route"
            else:
                category = "other_http_route"
        with self.server.state.lock:
            self.server.state.blocked_external_requests += 1
            self.server.state.blocked_external_categories[category] = (
                self.server.state.blocked_external_categories.get(category, 0) + 1
            )
        self.send_response(403)
        self.send_header("content-length", "0")
        self.end_headers()

    def _send_json(self, payload: dict[str, Any]) -> None:
        body = json.dumps(payload).encode("utf-8")
        self.send_response(200)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


def _safe_environment(server_port: int, codex_home: Path, scratch: Path) -> dict[str, str]:
    environment = _credential_free_environment()
    proxy = f"http://127.0.0.1:{server_port}"
    environment.update(
        {
            "CODEX_HOME": str(codex_home),
            "TEMP": str(scratch),
            "TMP": str(scratch),
            "HTTP_PROXY": proxy,
            "HTTPS_PROXY": proxy,
            "ALL_PROXY": proxy,
            "NO_PROXY": "127.0.0.1,localhost",
            "http_proxy": proxy,
            "https_proxy": proxy,
            "all_proxy": proxy,
            "no_proxy": "127.0.0.1,localhost",
            "RUST_LOG": "warn",
        }
    )
    return environment


def _credential_free_environment() -> dict[str, str]:
    sensitive_markers = ("API_KEY", "TOKEN", "SECRET", "PASSWORD", "CREDENTIAL")
    blocked_prefixes = ("OPENAI_", "AZURE_OPENAI_", "ANTHROPIC_", "CODEX_", "GITHUB_", "GH_")
    return {
        key: value
        for key, value in os.environ.items()
        if not key.upper().startswith(blocked_prefixes)
        and not any(marker in key.upper() for marker in sensitive_markers)
    }


def _overrides(base_url: str, scratch: Path) -> list[str]:
    overrides = [value.replace("__D4P_BASE_URL__", base_url) for value in _BASE_OVERRIDES]
    scratch_value = json.dumps(str(scratch).replace("\\", "/"), ensure_ascii=False)
    overrides.append(
        f"shell_environment_policy.set={{TEMP={scratch_value},TMP={scratch_value}}}"
    )
    return overrides


def _task_probe_command(nonce: str) -> str:
    return _TASK_PROBE_COMMAND_TEMPLATE.replace("__D4P_NONCE__", nonce)


def _kill_process_tree(process: subprocess.Popen[bytes], environment: dict[str, str]) -> None:
    try:
        subprocess.run(
            ["taskkill.exe", "/PID", str(process.pid), "/T", "/F"],
            env=environment,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            timeout=10,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired):
        process.kill()
    try:
        process.wait(timeout=10)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait(timeout=10)


def _remove_exact_marker(report: dict[str, Any], nonce: str) -> str:
    marker_value = report.get("markerPath")
    temp_value = report.get("tempPath")
    if not isinstance(marker_value, str) or not isinstance(temp_value, str):
        return "invalid_report_path"
    marker = Path(marker_value)
    temp_path = Path(temp_value)
    if marker.name != f"d4p-probe-{nonce}.tmp":
        return "unexpected_marker_name"
    try:
        marker_stat = marker.lstat()
    except FileNotFoundError:
        return "not_host_visible_after_cli_exit"
    except OSError:
        return "not_host_accessible"
    if not stat.S_ISREG(marker_stat.st_mode):
        return "not_a_regular_file"
    try:
        if marker.resolve(strict=True).parent != temp_path.resolve(strict=True):
            return "marker_outside_reported_temp"
        if marker.read_text(encoding="utf-8") != "codex-exec-temp-probe":
            return "unexpected_marker_content"
        os.remove(marker)
    except FileNotFoundError:
        return "not_host_visible_after_cli_exit"
    except OSError:
        return "not_host_accessible"
    return "present_after_cli_exit_removed_exact_file"


def _safe_event_summary(stdout: str, redact_paths: tuple[str, ...] = ()) -> list[dict[str, Any]]:
    summary = []
    for line in stdout.splitlines():
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        if not isinstance(event, dict):
            continue
        item = event.get("item")
        if not isinstance(item, dict):
            item = {}
        details = {
            "event_type": event.get("type"),
            "item_type": item.get("type"),
            "status": item.get("status"),
            "exit_code": item.get("exit_code"),
        }
        error = item.get("error")
        if isinstance(error, dict):
            details["error_code"] = error.get("code")
        if item.get("type") == "error" and isinstance(item.get("message"), str):
            message = item["message"]
            for path in redact_paths:
                message = message.replace(path, "<LOCAL_PATH>")
            message = re.sub(r"https?://\S+", "<URL>", message)
            message = re.sub(
                r"(?i)(authorization|api[_ -]?key|token|secret)(\s*[:=]\s*)\S+",
                r"\1\2<redacted>",
                message,
            )
            details["item_error_message"] = message[:300]
        if event.get("type") == "error" and isinstance(event.get("message"), str):
            message = event["message"]
            for path in redact_paths:
                message = message.replace(path, "<TEMP>")
            message = re.sub(r"https?://\S+", "<URL>", message)
            message = re.sub(
                r"(?i)(authorization|api[_ -]?key|token|secret)(\s*[:=]\s*)\S+",
                r"\1\2<redacted>",
                message,
            )
            details["message"] = message[:300]
        summary.append({key: value for key, value in details.items() if value is not None})
    return summary


def _run_once(
    executable: Path,
    server: _MockResponsesServer,
    version: str,
) -> dict[str, Any]:
    nonce = uuid.uuid4().hex
    with server.state.lock:
        blocked_categories_before = dict(server.state.blocked_external_categories)
    with tempfile.TemporaryDirectory(prefix="d4p-codex-mxc-probe-") as root_value:
        root = Path(root_value)
        codex_home = root / "codex-home"
        workspace = root / "workspace"
        scratch = workspace / ".d4p-tmp-probe"
        codex_home.mkdir()
        scratch.mkdir(parents=True)
        probe_files = {
            "probe-inputs/ordinary-marker.txt": "ordinary synthetic marker",
            "probe-inputs/config.env": "synthetic env marker",
            "probe-inputs/.env.production": "synthetic dotenv marker",
            ".ssh/id_ed25519": "synthetic ssh marker",
            "secrets/secret.txt": "synthetic secrets marker",
            "private/registered-marker.txt": "synthetic registered secret marker",
            "private/registered-directory/read-marker.txt": "synthetic registered child marker",
        }
        for relative_path, content in probe_files.items():
            path = workspace / relative_path
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(content, encoding="utf-8")
        outside_read_marker = root / "outside-read-marker.txt"
        outside_read_marker.write_text("outside synthetic marker", encoding="utf-8")
        write_probe_paths = {
            "env": workspace / "probe-inputs" / "write-probe.env",
            "dotenv": workspace / "probe-inputs" / ".env.write-probe",
            "ssh": workspace / ".ssh" / "write-probe",
            "secrets": workspace / "secrets" / "write-probe",
            "registered_file": workspace / "private" / "registered-write-target.txt",
            "registered_directory": workspace
            / "private"
            / "registered-directory"
            / "write-probe.txt",
            "registered_hardlink_alias": workspace
            / "private"
            / "registered-hardlink-alias.txt",
            "outside_workspace": root / "outside-write-marker.txt",
        }
        if any(path.exists() for path in write_probe_paths.values()):
            raise RuntimeError("synthetic_write_probe_target_preexists")
        command = _task_probe_command(nonce)
        server.state.begin_run(command, workspace)
        base_url = f"http://127.0.0.1:{server.server_address[1]}/v1"
        environment = _safe_environment(server.server_address[1], codex_home, scratch)
        arguments = [str(executable)]
        for override in _overrides(base_url, scratch):
            arguments.extend(("-c", override))
        arguments.extend(
            (
                "exec",
                "--ignore-user-config",
                "--ephemeral",
                "--json",
                "--skip-git-repo-check",
                "--cd",
                str(workspace),
                "-",
            )
        )
        process = subprocess.Popen(
            arguments,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            env=environment,
            creationflags=getattr(subprocess, "CREATE_NEW_PROCESS_GROUP", 0),
        )
        try:
            stdout, _stderr = process.communicate(
                b"Run the single bounded synthetic TEMP/TMP and filesystem probe.",
                timeout=RUN_TIMEOUT_SECONDS,
            )
        except subprocess.TimeoutExpired as error:
            _kill_process_tree(process, environment)
            report_path = workspace / "probe-report.json"
            if report_path.is_file():
                try:
                    report = json.loads(report_path.read_text(encoding="utf-8"))
                    _remove_exact_marker(report, nonce)
                except (OSError, ValueError, json.JSONDecodeError):
                    pass
            raise RuntimeError("codex_exec_timeout_process_tree_stopped") from error

        report_path = workspace / "probe-report.json"
        if not report_path.is_file():
            with server.state.lock:
                request_count = server.state.request_count
                exec_command_offered = server.state.exec_command_exposed
                response_write_completed = server.state.response_write_completed
                response_write_errors = server.state.response_write_errors
                blocked_categories_after = dict(server.state.blocked_external_categories)
            raise RuntimeError(
                "probe_report_missing="
                + json.dumps(
                    {
                        "codex_exec_exit_code": process.returncode,
                        "responses_api_requests": request_count,
                        "responses_api_stream_writes_completed": response_write_completed,
                        "responses_api_stream_write_errors": response_write_errors,
                        "blocked_non_loopback_request_categories": {
                            category: count - blocked_categories_before.get(category, 0)
                            for category, count in blocked_categories_after.items()
                            if count - blocked_categories_before.get(category, 0) > 0
                        },
                        "exec_command_offered": exec_command_offered,
                        "codex_json_event_summary": _safe_event_summary(
                            stdout.decode("utf-8", errors="replace"), (str(root), str(Path.home()))
                        ),
                    },
                    ensure_ascii=False,
                )
            )
        report = json.loads(report_path.read_text(encoding="utf-8"))
        marker_status = _remove_exact_marker(report, nonce)
        probe_statuses = report.get("probeStatuses")
        expected_probe_keys = {
            "workspaceControlRead",
            "envRead",
            "dotenvRead",
            "sshRead",
            "secretsRead",
            "registeredFileRead",
            "registeredDirectoryRead",
            "outsideWorkspaceRead",
            "envWrite",
            "dotenvWrite",
            "sshWrite",
            "secretsWrite",
            "registeredFileWrite",
            "registeredDirectoryWrite",
            "outsideWorkspaceWrite",
        }
        if not isinstance(probe_statuses, dict) or not expected_probe_keys.issubset(
            probe_statuses
        ):
            raise RuntimeError("filesystem_boundary_probe_report_invalid")
        if any(not isinstance(probe_statuses[key], str) for key in expected_probe_keys):
            raise RuntimeError("filesystem_boundary_probe_status_invalid")
        if any(
            probe_statuses[key] != "allowed"
            and not probe_statuses[key].startswith("blocked:")
            for key in expected_probe_keys
        ):
            raise RuntimeError("filesystem_boundary_probe_status_unclassified")
        hardlink_create_status = report.get("hardlinkCreateStatus")
        if not isinstance(hardlink_create_status, str) or not (
            hardlink_create_status.startswith("blocked:")
            or hardlink_create_status in ("created:read_allowed", "created:read_blocked")
        ):
            raise RuntimeError("hardlink_creation_probe_status_invalid")
        temp_path = Path(report["tempPath"])
        normalized_temp_path = str(report["tempPath"]).replace("/", "\\").casefold()
        scratch_marker = scratch / "scratch-marker.txt"
        event_summary = _safe_event_summary(
            stdout.decode("utf-8", errors="replace"), (str(root), str(Path.home()))
        )
        command_exit_codes = [
            event.get("exit_code")
            for event in event_summary
            if event.get("item_type") == "command_execution" and event.get("status") == "completed"
        ]
        turn_statuses = [
            event["event_type"]
            for event in event_summary
            if event.get("event_type") in ("turn.completed", "turn.failed", "turn.interrupted")
        ]
        try:
            marker_write_status = (workspace / "temp-write-status.txt").read_text(
                encoding="utf-8"
            ).strip()
        except OSError:
            marker_write_status = "status_not_host_visible"
        with server.state.lock:
            request_count_after = server.state.request_count
            exec_command_offered = server.state.exec_command_exposed
            blocked_categories_after = dict(server.state.blocked_external_categories)
        observed = {
            "codex_cli_version": version,
            "codex_exec_exit_code": process.returncode,
            "codex_turn_terminal_event": turn_statuses[-1] if turn_statuses else "not_observed",
            "codex_command_exit_code": command_exit_codes[-1] if command_exit_codes else None,
            "responses_api_requests": request_count_after,
            "blocked_non_loopback_request_categories": {
                category: count - blocked_categories_before.get(category, 0)
                for category, count in blocked_categories_after.items()
                if count - blocked_categories_before.get(category, 0) > 0
            },
            "exec_command_offered": exec_command_offered,
            "shell_environment_policy_temp_override": True,
            "temp_equals_workspace_task_scratch": report.get("tempMatchesScratch"),
            "tmp_equals_workspace_task_scratch": report.get("tmpMatchesScratch"),
            "temp_equals_tmp": str(report.get("tempPath", "")).casefold()
            == str(report.get("tmpPath", "")).casefold(),
            "temp_path_under_packages": "\\packages\\" in normalized_temp_path,
            "temp_path_ends_ac_temp": normalized_temp_path.endswith("\\ac\\temp"),
            "temp_marker_written_inside_mxc": marker_write_status.casefold() == "written|true",
            "temp_marker_after_cli_exit": marker_status,
            "temp_directory_visible_after_cli_exit": temp_path.is_dir(),
            "workspace_scratch_write_succeeded": report.get("scratchMarkerExists") is True
            and scratch_marker.is_file(),
            "filesystem_boundary": {
                "workspace_control_read_allowed": probe_statuses["workspaceControlRead"]
                == "allowed",
                "secret_glob_reads_allowed": {
                    "env": probe_statuses["envRead"] == "allowed",
                    "dotenv": probe_statuses["dotenvRead"] == "allowed",
                    "ssh": probe_statuses["sshRead"] == "allowed",
                    "secrets": probe_statuses["secretsRead"] == "allowed",
                },
                "secret_glob_writes_allowed": {
                    "env": probe_statuses["envWrite"] == "allowed",
                    "dotenv": probe_statuses["dotenvWrite"] == "allowed",
                    "ssh": probe_statuses["sshWrite"] == "allowed",
                    "secrets": probe_statuses["secretsWrite"] == "allowed",
                },
                "registered_secret_path_reads_allowed": {
                    "file": probe_statuses["registeredFileRead"] == "allowed",
                    "directory_child": probe_statuses["registeredDirectoryRead"]
                    == "allowed",
                },
                "registered_secret_path_writes_allowed": {
                    "file": probe_statuses["registeredFileWrite"] == "allowed",
                    "directory_child": probe_statuses["registeredDirectoryWrite"]
                    == "allowed",
                },
                "registered_secret_hardlink_creation_status": hardlink_create_status,
                "registered_secret_hardlink_creation_denied": (
                    hardlink_create_status.endswith(":0x80070005")
                    or hardlink_create_status.endswith(":native:5")
                ),
                "registered_secret_hardlink_alias_read_allowed": hardlink_create_status
                == "created:read_allowed",
                "registered_secret_hardlink_alias_visible_to_host": write_probe_paths[
                    "registered_hardlink_alias"
                ].is_file(),
                "outside_workspace_read_allowed": probe_statuses[
                    "outsideWorkspaceRead"
                ]
                == "allowed",
                "outside_workspace_write_allowed": probe_statuses[
                    "outsideWorkspaceWrite"
                ]
                == "allowed",
                "write_markers_visible_to_host": {
                    name: path.is_file() for name, path in write_probe_paths.items()
                },
            },
            "codex_json_event_summary": event_summary,
        }
        failures = []
        if not observed["exec_command_offered"] or request_count_after != 2:
            failures.append("codex_exec_tool_call_not_completed")
        if observed["codex_exec_exit_code"] != 0:
            failures.append("codex_exec_exit_nonzero")
        if observed["codex_turn_terminal_event"] != "turn.completed":
            failures.append("codex_turn_not_completed")
        if observed["codex_command_exit_code"] != 0:
            failures.append("codex_shell_command_not_successful")
        if not observed["temp_marker_written_inside_mxc"]:
            failures.append("mxc_temp_marker_write_not_observed")
        if marker_status != "not_host_visible_after_cli_exit":
            failures.append(f"mxc_temp_marker_cleanup_{marker_status}")
        if not observed["workspace_scratch_write_succeeded"]:
            failures.append("workspace_scratch_write_not_observed")
        filesystem_boundary = observed["filesystem_boundary"]
        if not filesystem_boundary["workspace_control_read_allowed"]:
            failures.append("workspace_control_read_not_allowed")
        if any(filesystem_boundary["secret_glob_reads_allowed"].values()):
            failures.append("workspace_secret_path_read_allowed")
        if any(filesystem_boundary["registered_secret_path_reads_allowed"].values()):
            failures.append("registered_secret_path_read_allowed")
        if any(filesystem_boundary["registered_secret_path_writes_allowed"].values()):
            failures.append("registered_secret_path_write_allowed")
        if not filesystem_boundary["registered_secret_hardlink_creation_denied"]:
            failures.append("registered_secret_hardlink_creation_not_access_denied")
        if filesystem_boundary["registered_secret_hardlink_alias_read_allowed"]:
            failures.append("registered_secret_hardlink_alias_read_allowed")
        if filesystem_boundary["registered_secret_hardlink_alias_visible_to_host"]:
            failures.append("registered_secret_hardlink_alias_visible_to_host")
        if filesystem_boundary["outside_workspace_read_allowed"]:
            failures.append("outside_workspace_read_allowed")
        if filesystem_boundary["outside_workspace_write_allowed"]:
            failures.append("outside_workspace_write_allowed")
        protected_write_markers = {
            name: filesystem_boundary["write_markers_visible_to_host"][name]
            for name in (
                "registered_file",
                "registered_directory",
                "registered_hardlink_alias",
                "outside_workspace",
            )
        }
        if any(protected_write_markers.values()):
            failures.append("protected_write_marker_visible")
        if failures:
            raise RuntimeError(
                "probe_failed=" + ",".join(failures) + ";観測="
                + json.dumps(observed, ensure_ascii=False)
            )
        return observed


def _version(executable: Path) -> str:
    with tempfile.TemporaryDirectory(prefix="d4p-codex-version-") as root_value:
        environment = _credential_free_environment()
        environment["CODEX_HOME"] = str(Path(root_value) / "codex-home")
        completed = subprocess.run(
            [str(executable), "--version"],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
            env=environment,
            timeout=RUN_TIMEOUT_SECONDS,
            check=False,
        )
    if completed.returncode != 0:
        raise RuntimeError("codex_version_probe_failed")
    match = re.search(r"\bcodex-cli\s+([^\s]+)", completed.stdout, flags=re.IGNORECASE)
    if match is None:
        raise RuntimeError("codex_cli_version_unrecognized")
    return match.group(1)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="実Codex CLI/MxCのTask shell child一時領域・filesystem挙動をローカル偽APIで観測する"
    )
    parser.add_argument("--exe", required=True, type=Path, help="Ownerが指定する絶対codex.exe path")
    parser.add_argument("--runs", type=int, default=MAX_RUNS, help=f"連続実行数（1..{MAX_RUNS}、既定{MAX_RUNS}）")
    args = parser.parse_args()
    if os.name != "nt":
        parser.error("このprobeはWindows専用です")
    if not args.exe.is_absolute() or not args.exe.is_file():
        parser.error("--exeには存在するcodex.exeの絶対pathを指定してください")
    if not 1 <= args.runs <= MAX_RUNS:
        parser.error(f"--runsは1から{MAX_RUNS}の範囲で指定してください")

    try:
        version = _version(args.exe)
        try:
            import ctypes

            host_elevated = bool(ctypes.windll.shell32.IsUserAnAdmin())
        except (AttributeError, OSError):
            host_elevated = None
        server = _MockResponsesServer()
        thread = threading.Thread(target=server.serve_forever, name="d4p-loopback-responses", daemon=True)
        thread.start()
        try:
            runs = [_run_once(args.exe, server, version) for _ in range(args.runs)]
        finally:
            server.shutdown()
            server.server_close()
            thread.join(timeout=2)
        print(
            json.dumps(
                {
                    "result": "observed",
                    "evidence_source": "LIVE_RUNTIME",
                    "host_process_elevated": host_elevated,
                    "runs": runs,
                    "blocked_non_loopback_requests": server.state.blocked_external_requests,
                    "scope_limit": (
                        "直接codex execと実MxC shell tool childだけ。偽Responses APIを使用し、"
                        "Broker、Owner Approval、実モデル、取消・期限・crash、製品Task lifecycleは通さない。"
                    ),
                },
                ensure_ascii=False,
                indent=2,
            )
        )
        return 0
    except (OSError, subprocess.SubprocessError, RuntimeError, ValueError, json.JSONDecodeError) as error:
        print(json.dumps({"result": "failed", "reason": str(error)}, ensure_ascii=False))
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
