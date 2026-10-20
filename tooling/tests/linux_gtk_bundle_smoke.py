"""Linux Release bundleを隔離X11 desktop上で起動し、正常終了を確認する。"""

from __future__ import annotations

import argparse
import os
import re
import signal
import subprocess
import tempfile
import time
from pathlib import Path

PRODUCT_WINDOW_TITLE = "D4 Pocket powered by GUI Shell"


def _processes_using(executable: Path) -> set[int]:
    result: set[int] = set()
    for entry in Path("/proc").iterdir():
        if not entry.name.isdecimal():
            continue
        try:
            if Path(os.readlink(entry / "exe")) == executable:
                result.add(int(entry.name))
        except (FileNotFoundError, PermissionError, ProcessLookupError):
            continue
    return result


def _failure_with_launcher_output(message: str, app_log: Path, isolated_root: Path) -> str:
    try:
        output = app_log.read_bytes()[-4096:].decode("utf-8", errors="replace")
    except OSError:
        output = ""
    output = output.replace(str(isolated_root), "<隔離root>")
    output = re.sub(
        r"(?i)\b(authorization|password|passwd|token|secret|api[_-]?key)(\s*[:=]\s*)[^\s,;]+",
        r"\1\2[伏字]",
        output,
    )
    output = output.strip()
    if len(output) > 2048:
        output = output[-2048:]
    if not output:
        return message
    return f"{message}\nGTK launcher診断（末尾2048文字以内）:\n{output}"


def _terminate_group(process: subprocess.Popen[bytes]) -> None:
    if process.poll() is not None:
        return
    try:
        os.killpg(process.pid, signal.SIGTERM)
        process.wait(timeout=5)
    except (ProcessLookupError, subprocess.TimeoutExpired):
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            pass


def run_smoke(launcher: Path, temp_root: Path) -> None:
    if launcher.is_symlink():
        raise RuntimeError("GTK launcherがsymlinkです")
    if temp_root.is_symlink():
        raise RuntimeError("smoke用一時rootがsymlinkです")
    launcher = launcher.resolve(strict=True)
    if not launcher.is_file() or not os.access(launcher, os.X_OK):
        raise RuntimeError("GTK launcherが通常実行fileではありません")
    helper = launcher.parent / "libexec" / "gui_shell_linux_broker"
    if helper.is_symlink():
        raise RuntimeError("Broker helperがsymlinkです")
    helper = helper.resolve(strict=True)
    temp_root = temp_root.resolve(strict=True)
    if not temp_root.is_dir():
        raise RuntimeError("smoke用一時rootが通常directoryではありません")
    if not os.environ.get("DISPLAY") or not os.environ.get("DBUS_SESSION_BUS_ADDRESS"):
        raise RuntimeError("X11 displayまたは一時D-Bus sessionがありません")

    with tempfile.TemporaryDirectory(prefix="d4p-linux-gtk-smoke-", dir=temp_root) as raw:
        root = Path(raw)
        os.chmod(root, 0o700)
        home = root / "home"
        runtime = root / "runtime"
        temporary = root / "tmp"
        for directory in (home, runtime, temporary):
            directory.mkdir(mode=0o700)
        env = {
            key: os.environ[key]
            for key in (
                "PATH",
                "DISPLAY",
                "XAUTHORITY",
                "DBUS_SESSION_BUS_ADDRESS",
                "LANG",
                "LC_ALL",
                "LC_CTYPE",
            )
            if os.environ.get(key)
        }
        env.update(
            {
                "HOME": str(home),
                "TMPDIR": str(temporary),
                "XDG_CONFIG_HOME": str(home / ".config"),
                "XDG_DATA_HOME": str(home / ".local" / "share"),
                "XDG_CACHE_HOME": str(home / ".cache"),
                "XDG_STATE_HOME": str(home / ".local" / "state"),
                "XDG_RUNTIME_DIR": str(runtime),
                "XDG_DATA_DIRS": "/usr/local/share:/usr/share",
                "GDK_BACKEND": "x11",
            }
        )
        for name in (".config", ".local", ".local/share", ".local/state", ".cache"):
            (home / name).mkdir(mode=0o700, parents=True, exist_ok=True)

        preexisting_helpers = _processes_using(helper)
        window_manager = subprocess.Popen(
            ["openbox", "--sm-disable"],
            env=env,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            start_new_session=True,
        )
        app_log = root / "app-output.log"
        launcher_process: subprocess.Popen[bytes] | None = None
        try:
            time.sleep(0.5)
            if window_manager.poll() is not None:
                raise RuntimeError("隔離X11 window managerが起動しませんでした")
            with app_log.open("xb") as log:
                launcher_process = subprocess.Popen(
                    [str(launcher)],
                    cwd=launcher.parent,
                    env=env,
                    stdin=subprocess.DEVNULL,
                    stdout=log,
                    stderr=subprocess.STDOUT,
                    start_new_session=True,
                )

            window_id: str | None = None
            deadline = time.monotonic() + 60
            pattern = f"^{re.escape(PRODUCT_WINDOW_TITLE)}$"
            while time.monotonic() < deadline:
                if launcher_process.poll() is not None:
                    raise RuntimeError(
                        _failure_with_launcher_output(
                            "GTK launcherがwindow表示前に終了しました: "
                            f"{launcher_process.returncode}",
                            app_log,
                            root,
                        )
                    )
                search = subprocess.run(
                    ["xdotool", "search", "--onlyvisible", "--name", pattern],
                    env=env,
                    capture_output=True,
                    check=False,
                    timeout=5,
                )
                if search.returncode == 0:
                    candidate = search.stdout.decode("ascii", errors="ignore").splitlines()
                    if candidate:
                        window_id = candidate[0]
                        break
                time.sleep(0.25)
            if window_id is None:
                raise RuntimeError(
                    _failure_with_launcher_output(
                        "D4 PocketのGTK window titleを検出できませんでした",
                        app_log,
                        root,
                    )
                )

            helper_seen = False
            for _ in range(20):
                if _processes_using(helper) - preexisting_helpers:
                    helper_seen = True
                    break
                time.sleep(0.1)

            close = subprocess.run(
                ["xdotool", "windowclose", window_id],
                env=env,
                capture_output=True,
                check=False,
                timeout=10,
            )
            if close.returncode != 0:
                raise RuntimeError("D4 Pocket GTK windowへ正常終了要求を送れませんでした")
            try:
                exit_code = launcher_process.wait(timeout=45)
            except subprocess.TimeoutExpired as error:
                raise RuntimeError("GTK window close後にlauncherが終了しませんでした") from error
            if exit_code != 0:
                raise RuntimeError(
                    _failure_with_launcher_output(
                        f"GTK launcherが異常終了しました: {exit_code}", app_log, root
                    )
                )

            cleanup_deadline = time.monotonic() + 10
            remaining: set[int] = set()
            while time.monotonic() < cleanup_deadline:
                remaining = _processes_using(helper) - preexisting_helpers
                if not remaining:
                    break
                time.sleep(0.2)
            if remaining:
                raise RuntimeError("GTK正常終了後にBroker helper processが残留しました")
            print(
                "Linux GTK製品画面試験 合格: 製品画面を表示し正常終了; "
                f"Broker helper観測済み={str(helper_seen).lower()}"
            )
        finally:
            if launcher_process is not None:
                _terminate_group(launcher_process)
            _terminate_group(window_manager)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--launcher", type=Path, required=True)
    parser.add_argument("--temp-root", type=Path, required=True)
    args = parser.parse_args()
    try:
        run_smoke(args.launcher, args.temp_root)
    except Exception as error:
        print(f"Linux GTK製品画面試験 不合格: {error}")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
