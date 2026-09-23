"""実物Agent CLIのinterfaceだけを確認するdevelopment-only probe。

このprobeはversion/helpだけを呼び出し、prompt、credential、workspace、taskを渡さない。
Agent起動やcommand dispatchはBrokerのproduction経路へ実装しない。
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import re
import shutil
import subprocess
from pathlib import Path


PROBE_TIMEOUT_SECONDS = 5


def _platform_name() -> str:
    return {
        "Windows": "windows",
        "Darwin": "macos",
        "Linux": "linux",
    }.get(platform.system(), "unknown")


def _safe_environment() -> dict[str, str]:
    allowed = {"PATH", "SystemRoot", "WINDIR", "TEMP", "TMP"}
    return {key: value for key, value in os.environ.items() if key in allowed}


def _run_read_only(executable: str, arguments: list[str]) -> tuple[int, str]:
    try:
        completed = subprocess.run(
            [executable, *arguments],
            cwd=Path.cwd(),
            env=_safe_environment(),
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            timeout=PROBE_TIMEOUT_SECONDS,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired):
        return -1, ""
    return completed.returncode, completed.stdout[:4096]


def _version_text(output: str) -> str:
    match = re.search(r"\bcodex-cli\s+([^\s]+)", output, flags=re.IGNORECASE)
    return match.group(1) if match else "unknown"


def _support(status: str, reason: str) -> dict[str, str]:
    return {"status": status, "reason": reason}


def build_adapter_record(
    executable: str | None,
    version: str,
    version_ok: bool,
    help_ok: bool,
) -> dict[str, object]:
    current_platform = _platform_name()
    if executable is None:
        status = "unavailable"
        status_reason = "codex CLIがPATHに存在しない"
    elif not version_ok or not help_ok:
        status = "unsupported"
        status_reason = "codex CLIのversion/help interfaceを確認できない"
    else:
        status = "degraded"
        status_reason = "interfaceは確認済みだがBrokerのcommand dispatchが停止中である"
    return {
        "adapter_id": "codex-cli",
        "agent_id": "codex",
        "provider": "OpenAI",
        "version": version if version_ok else "unknown",
        "model": "unknown",
        "status": status,
        "capabilities": [
            {
                "capability_id": "task_execution",
                "support": _support("unknown", "help interfaceだけを確認し、実taskは実行していない"),
            },
            {
                "capability_id": "session_control",
                "support": _support("unknown", "exec helpにresume/fork表記はあるが、セッション操作の実動作は確認していない"),
            },
        ],
        "workspace_requirements": {
            "mode": "required",
            "boundary_policy": "deny_outside_workspace",
            "secret_paths": [".env", ".ssh", "secrets/"],
        },
        "tool_support": _support("unknown", "実taskを実行してtool経路を確認していない"),
        "mcp_support": _support("unknown", "help interfaceだけではMCP接続可否を確定できない"),
        "session_support": _support("unknown", "exec helpにsession操作表記はあるが、セッション操作の実動作は確認していない"),
        "cancellation_support": _support("unknown", "process終了を実行していない"),
        "usage_metrics_support": _support("unknown", "実taskのmetricsを取得していない"),
        "cost_metrics_support": _support("unknown", "cost情報を取得していない"),
        "authentication": {
            "method": "unknown",
            "secret_value_present": False,
        },
        "host_requirements": {
            "platforms": [current_platform],
            "network_scope": "unknown",
            "process_spawn": _support("unsupported", "Brokerのcommand dispatchが停止中である"),
        },
        "evidence_source": "LIVE_RUNTIME" if executable and version_ok and help_ok else "CONFIG",
        "evidence_reason": status_reason,
    }


def probe_codex_cli() -> dict[str, object]:
    executable = shutil.which("codex")
    if executable is None:
        return build_adapter_record(None, "unknown", False, False)
    version_code, version_output = _run_read_only(executable, ["--version"])
    help_code, help_output = _run_read_only(executable, ["exec", "--help"])
    help_ok = help_code == 0 and "codex exec" in help_output
    return build_adapter_record(
        executable,
        _version_text(version_output),
        version_code == 0,
        help_ok,
    )


def main() -> int:
    parser = argparse.ArgumentParser(description="Agent CLIの読み取り専用interface probe")
    parser.add_argument("--check", action="store_true", help="probe結果をJSONで出力する")
    args = parser.parse_args()
    if not args.check:
        parser.error("--checkが必要です")
    print(json.dumps(probe_codex_cli(), ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
