from __future__ import annotations

import json
import os
import pathlib
import plistlib
import re
import signal
import subprocess
import sys
import time
from typing import Any


def _parse_signed_entitlements(output: str) -> dict[str, Any]:
    start = output.find("<?xml")
    if start < 0:
        start = output.find("<plist")
    end = output.rfind("</plist>")
    if start < 0 or end < start:
        raise ValueError("署名entitlement plistが見つからない")
    try:
        value = plistlib.loads(output[start : end + len("</plist>")].encode("utf-8"))
    except (plistlib.InvalidFileException, ValueError, TypeError) as error:
        raise ValueError("署名entitlement plistが不正") from error
    if not isinstance(value, dict):
        raise ValueError("署名entitlementがobjectではない")
    return value


def _parse_processes(output: str) -> list[tuple[int, int, str]]:
    processes = []
    for line in output.splitlines():
        fields = line.strip().split(maxsplit=2)
        if len(fields) != 3:
            continue
        try:
            pid, parent = int(fields[0]), int(fields[1])
        except ValueError:
            continue
        executable = fields[2].split(maxsplit=1)[0]
        processes.append((pid, parent, pathlib.Path(executable).name))
    return processes


def _process_snapshot() -> list[tuple[int, int, str]]:
    result = subprocess.run(
        ["/bin/ps", "-ww", "-axo", "pid=,ppid=,command="],
        check=True,
        capture_output=True,
        text=True,
    )
    return _parse_processes(result.stdout)


def _signed_entitlements(code_path: pathlib.Path) -> dict[str, Any]:
    result = subprocess.run(
        ["/usr/bin/codesign", "--display", "--entitlements", ":-", "--xml", str(code_path)],
        check=True,
        capture_output=True,
        text=True,
    )
    return _parse_signed_entitlements(result.stdout + "\n" + result.stderr)


def verify_installed_launch(
    app_path: pathlib.Path, bundle_id: str, evidence_path: pathlib.Path
) -> dict[str, Any]:
    if re.fullmatch(r"com\.d4p\.export\.[0-9a-f]{32}", bundle_id) is None:
        raise ValueError("installed App Bundle IDがExport fixture identityではない")
    if not app_path.is_absolute() or app_path.is_symlink() or not app_path.is_dir():
        raise ValueError("installed App Bundle pathが不正")
    if evidence_path.exists() or evidence_path.is_symlink():
        raise ValueError("installed launch evidence pathが既存である")
    info_path = app_path / "Contents/Info.plist"
    if info_path.is_symlink() or not info_path.is_file():
        raise ValueError("installed App Info.plistが通常fileではない")
    with info_path.open("rb") as stream:
        info = plistlib.load(stream)
    if info.get("CFBundleIdentifier") != bundle_id:
        raise ValueError("installed App Bundle IDが指定値と一致しない")
    executable_name = info.get("CFBundleExecutable")
    if not isinstance(executable_name, str) or re.fullmatch(r"[A-Za-z0-9._-]{1,128}", executable_name) is None:
        raise ValueError("installed App executable名が不正")
    app_executable = app_path / "Contents/MacOS" / executable_name
    helper_executable = app_path / "Contents/MacOS/gui_shell_macos_broker"
    for executable in (app_executable, helper_executable):
        if executable.is_symlink() or not executable.is_file():
            raise ValueError("installed AppまたはBroker helperが通常fileではない")

    subprocess.run(
        ["/usr/bin/codesign", "--verify", "--deep", "--strict", "--all-architectures", str(app_path)],
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        text=True,
    )
    subprocess.run(
        ["/usr/bin/codesign", "--verify", "--strict", "--all-architectures", str(helper_executable)],
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        text=True,
    )
    expected_app = {
        "com.apple.security.app-sandbox": True,
        "com.apple.security.files.user-selected.read-write": True,
        "com.apple.security.network.client": True,
        "com.apple.security.network.server": True,
    }
    expected_helper = {
        "com.apple.security.app-sandbox": True,
        "com.apple.security.inherit": True,
    }
    if _signed_entitlements(app_path) != expected_app:
        raise ValueError("installed App sandbox entitlementが固定値と一致しない")
    if _signed_entitlements(helper_executable) != expected_helper:
        raise ValueError("installed helper sandbox entitlementが固定値と一致しない")

    def named_processes(rows: list[tuple[int, int, str]], name: str) -> list[int]:
        return [pid for pid, _parent, process_name in rows if process_name == name]

    before = _process_snapshot()
    if named_processes(before, executable_name) or named_processes(before, "gui_shell_macos_broker"):
        raise ValueError("installed launch fixture processが起動前から存在する")

    app_pid: int | None = None
    helper_pid: int | None = None
    launch_observed = False
    quit_sent = False
    stopped = False
    forced_cleanup = False
    try:
        subprocess.run(
            ["/usr/bin/open", "-n", str(app_path)],
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
            text=True,
        )
        deadline = time.monotonic() + 60
        while time.monotonic() < deadline:
            rows = _process_snapshot()
            app_pids = named_processes(rows, executable_name)
            if len(app_pids) == 1:
                app_pid = app_pids[0]
                helper_pids = [
                    pid
                    for pid, parent, name in rows
                    if name == "gui_shell_macos_broker" and parent == app_pid
                ]
                if len(helper_pids) == 1:
                    helper_pid = helper_pids[0]
                    break
            time.sleep(0.5)
        launch_observed = app_pid is not None and helper_pid is not None
    finally:
        try:
            quit_result = subprocess.run(
                [
                    "/usr/bin/osascript",
                    "-e",
                    f'tell application id "{bundle_id}" to quit',
                ],
                capture_output=True,
                text=True,
                timeout=20,
                check=False,
            )
            quit_sent = quit_result.returncode == 0
        except (OSError, subprocess.TimeoutExpired):
            quit_sent = False

        def remaining_product_processes() -> list[tuple[int, str]]:
            return [
                (pid, name)
                for pid, _parent, name in _process_snapshot()
                if name in {executable_name, "gui_shell_macos_broker"}
            ]

        deadline = time.monotonic() + 30
        while time.monotonic() < deadline:
            if not remaining_product_processes():
                stopped = True
                break
            time.sleep(0.5)
        if not stopped:
            for pid, _name in remaining_product_processes():
                try:
                    os.kill(pid, signal.SIGTERM)
                    forced_cleanup = True
                except ProcessLookupError:
                    pass
            deadline = time.monotonic() + 15
            while time.monotonic() < deadline:
                if not remaining_product_processes():
                    break
                time.sleep(0.5)

    if not launch_observed or not quit_sent or not stopped or forced_cleanup:
        raise RuntimeError("installed app/helper launchまたはgraceful quit条件が成立しない")
    result = {
        "app_bundle_signature_verified": True,
        "helper_signature_verified": True,
        "app_sandbox_entitlements_verified": True,
        "helper_inherited_sandbox_entitlements_verified": True,
        "app_launched": True,
        "broker_helper_started_as_app_child": True,
        "graceful_quit_request_sent": True,
        "app_process_stopped": True,
        "broker_helper_process_stopped": True,
        "helper_exit_status_observed": False,
    }
    evidence_path.write_text(json.dumps(result, separators=(",", ":")) + "\n", encoding="utf-8")
    return result


def main() -> int:
    if len(sys.argv) != 4:
        raise SystemExit("usage: macos_installed_launch_smoke.py APP_PATH BUNDLE_ID EVIDENCE_PATH")
    result = verify_installed_launch(
        pathlib.Path(sys.argv[1]), sys.argv[2], pathlib.Path(sys.argv[3])
    )
    print(json.dumps(result, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
