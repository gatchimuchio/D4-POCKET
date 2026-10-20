"""Linux Release bundleのFlutter GTK runnerを隔離X11／Wayland sessionで確認する。"""

from __future__ import annotations

import argparse
import os
import re
import signal
import stat
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


def _owned_product_groups(
    launcher: subprocess.Popen[bytes],
    runner: Path,
    helper: Path,
    preexisting_runners: set[int],
    preexisting_helpers: set[int],
) -> set[int]:
    groups = {launcher.pid} if launcher.poll() is None else set()
    owned_pids = (_processes_using(runner) - preexisting_runners) | (
        _processes_using(helper) - preexisting_helpers
    )
    for pid in owned_pids:
        try:
            groups.add(os.getpgid(pid))
        except (ProcessLookupError, PermissionError):
            continue
    return groups


def _signal_owned_product_groups(
    launcher: subprocess.Popen[bytes],
    runner: Path,
    helper: Path,
    preexisting_runners: set[int],
    preexisting_helpers: set[int],
    sig: signal.Signals,
) -> None:
    for process_group in _owned_product_groups(
        launcher, runner, helper, preexisting_runners, preexisting_helpers
    ):
        try:
            os.killpg(process_group, sig)
        except ProcessLookupError:
            continue


def _failure_with_runner_output(message: str, app_log: Path, isolated_root: Path) -> str:
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
    return f"{message}\nGTK runner診断（末尾2048文字以内）:\n{output}"


def _terminate_group(process: subprocess.Popen[bytes]) -> None:
    try:
        os.killpg(process.pid, signal.SIGTERM)
    except ProcessLookupError:
        return
    try:
        process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            pass
        return
    try:
        os.killpg(process.pid, 0)
    except ProcessLookupError:
        return
    try:
        os.killpg(process.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    try:
        process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        pass


def _request_owned_product_sigterm(
    launcher: subprocess.Popen[bytes],
    runner: Path,
    helper: Path,
    preexisting_runners: set[int],
    preexisting_helpers: set[int],
) -> None:
    _signal_owned_product_groups(
        launcher,
        runner,
        helper,
        preexisting_runners,
        preexisting_helpers,
        signal.SIGTERM,
    )
    try:
        launcher.wait(timeout=10)
    except subprocess.TimeoutExpired as error:
        raise RuntimeError("Wayland smoke後に所有launcherがSIGTERMで停止しませんでした") from error


def _terminate_owned_product(
    launcher: subprocess.Popen[bytes],
    runner: Path,
    helper: Path,
    preexisting_runners: set[int],
    preexisting_helpers: set[int],
) -> None:
    _signal_owned_product_groups(
        launcher,
        runner,
        helper,
        preexisting_runners,
        preexisting_helpers,
        signal.SIGTERM,
    )
    try:
        launcher.wait(timeout=5)
    except subprocess.TimeoutExpired:
        _signal_owned_product_groups(
            launcher,
            runner,
            helper,
            preexisting_runners,
            preexisting_helpers,
            signal.SIGKILL,
        )
        try:
            launcher.wait(timeout=5)
        except subprocess.TimeoutExpired:
            pass
    remaining_runners = _processes_using(runner) - preexisting_runners
    remaining_helpers = _processes_using(helper) - preexisting_helpers
    if remaining_runners or remaining_helpers:
        _signal_owned_product_groups(
            launcher,
            runner,
            helper,
            preexisting_runners,
            preexisting_helpers,
            signal.SIGKILL,
        )


def _wayland_toplevel_ready(output: str) -> bool:
    title = re.escape(PRODUCT_WINDOW_TITLE)
    return bool(
        re.search(r"xdg_toplevel@\d+\.set_title\(\"" + title + r"\"\)", output)
        and re.search(r"wl_surface@\d+\.commit\(\)", output)
    )


def _start_wayland_compositor(
    runtime: Path, environment: dict[str, str], log_path: Path
) -> tuple[subprocess.Popen[bytes], str]:
    display_name = "wayland-d4p-smoke"
    socket_path = runtime / display_name
    if socket_path.exists() or socket_path.is_symlink():
        raise RuntimeError("隔離Wayland socketが事前に存在しています")

    compositor_environment = environment.copy()
    for name in (
        "DISPLAY",
        "XAUTHORITY",
        "WAYLAND_DISPLAY",
        "GDK_BACKEND",
        "WAYLAND_DEBUG",
        "XDG_SESSION_TYPE",
    ):
        compositor_environment.pop(name, None)
    with log_path.open("xb") as log:
        compositor = subprocess.Popen(
            [
                "weston",
                "--backend=headless",
                "--renderer=pixman",
                f"--socket={display_name}",
            ],
            env=compositor_environment,
            stdin=subprocess.DEVNULL,
            stdout=log,
            stderr=subprocess.STDOUT,
            start_new_session=True,
        )

    deadline = time.monotonic() + 20
    while time.monotonic() < deadline:
        if compositor.poll() is not None:
            raise RuntimeError(
                _failure_with_runner_output(
                    f"隔離Weston compositorが起動前に終了しました: {compositor.returncode}",
                    log_path,
                    runtime.parent,
                )
            )
        try:
            socket_info = socket_path.lstat()
        except FileNotFoundError:
            time.sleep(0.1)
            continue
        if stat.S_ISSOCK(socket_info.st_mode):
            return compositor, display_name
        _terminate_group(compositor)
        raise RuntimeError("Westonが通常Unix socket以外のWayland endpointを作成しました")

    _terminate_group(compositor)
    raise RuntimeError(
        _failure_with_runner_output(
            "隔離Weston compositorのWayland socketが時間内に作成されませんでした",
            log_path,
            runtime.parent,
        )
    )


def run_smoke(launcher: Path, temp_root: Path, backend: str) -> None:
    if backend not in {"x11", "wayland"}:
        raise RuntimeError("表示backendが許可集合外です")
    if launcher.is_symlink():
        raise RuntimeError("Linux製品launcherがsymlinkです")
    if temp_root.is_symlink():
        raise RuntimeError("smoke用一時rootがsymlinkです")
    launcher = launcher.resolve(strict=True)
    if launcher.name != "gui_shell_desktop_launcher":
        raise RuntimeError("Linux製品launcherの固定file名が一致しません")
    if not launcher.is_file() or not os.access(launcher, os.X_OK):
        raise RuntimeError("Linux製品launcherが通常実行fileではありません")
    bundle_root = launcher.parent
    runner = bundle_root / "gui_shell_desktop"
    if runner.is_symlink():
        raise RuntimeError("Flutter GTK runnerがsymlinkです")
    runner = runner.resolve(strict=True)
    if not runner.is_file() or not os.access(runner, os.X_OK):
        raise RuntimeError("Flutter GTK runnerが通常実行fileではありません")
    helper = bundle_root / "libexec" / "gui_shell_linux_broker"
    if helper.is_symlink():
        raise RuntimeError("Broker helperがsymlinkです")
    helper = helper.resolve(strict=True)
    temp_root = temp_root.resolve(strict=True)
    if not temp_root.is_dir():
        raise RuntimeError("smoke用一時rootが通常directoryではありません")
    if backend == "x11" and not os.environ.get("DISPLAY"):
        raise RuntimeError("X11 displayがありません")
    if not os.environ.get("DBUS_SESSION_BUS_ADDRESS"):
        raise RuntimeError("一時D-Bus sessionがありません")

    with tempfile.TemporaryDirectory(prefix="d4p-linux-gtk-smoke-", dir=temp_root) as raw:
        root = Path(raw)
        os.chmod(root, 0o700)
        home = root / "home"
        runtime = root / "runtime"
        temporary = root / "tmp"
        xdg_data_home = home / ".local" / "share"
        for directory in (home, runtime, temporary):
            directory.mkdir(mode=0o700)
        if xdg_data_home.exists() or xdg_data_home.is_symlink():
            raise RuntimeError("初回起動前にXDG_DATA_HOMEが存在しています")
        env = {
            key: os.environ[key]
            for key in ("PATH", "DBUS_SESSION_BUS_ADDRESS", "LANG", "LC_ALL", "LC_CTYPE")
            if os.environ.get(key)
        }
        if backend == "x11":
            env["DISPLAY"] = os.environ["DISPLAY"]
            if os.environ.get("XAUTHORITY"):
                env["XAUTHORITY"] = os.environ["XAUTHORITY"]
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
                "GDK_BACKEND": backend,
            }
        )
        if backend == "wayland":
            env["XDG_SESSION_TYPE"] = "wayland"
            env["WAYLAND_DEBUG"] = "client"
        for name in (".config", ".local/state", ".cache"):
            (home / name).mkdir(mode=0o700, parents=True, exist_ok=True)

        preexisting_helpers = _processes_using(helper)
        preexisting_runners = _processes_using(runner)
        window_manager: subprocess.Popen[bytes] | None = None
        compositor: subprocess.Popen[bytes] | None = None
        if backend == "x11":
            window_manager = subprocess.Popen(
                ["openbox", "--sm-disable"],
                env=env,
                stdin=subprocess.DEVNULL,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                start_new_session=True,
            )
        else:
            compositor, wayland_display = _start_wayland_compositor(
                runtime, env, root / "weston-output.log"
            )
            env["WAYLAND_DISPLAY"] = wayland_display
        app_log = root / "app-output.log"
        runner_process: subprocess.Popen[bytes] | None = None
        try:
            if backend == "x11":
                time.sleep(0.5)
                if window_manager is None or window_manager.poll() is not None:
                    raise RuntimeError("隔離X11 window managerが起動しませんでした")
            with app_log.open("xb") as log:
                runner_process = subprocess.Popen(
                    [str(launcher)],
                    cwd=bundle_root,
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
                if runner_process.poll() is not None:
                    raise RuntimeError(
                        _failure_with_runner_output(
                            "Flutter GTK runnerがwindow表示前に終了しました: "
                            f"{runner_process.returncode}",
                            app_log,
                            root,
                        )
                    )
                if backend == "x11":
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
                else:
                    output = app_log.read_bytes()[-256 * 1024 :].decode(
                        "utf-8", errors="replace"
                    )
                    if _wayland_toplevel_ready(output):
                        break
                time.sleep(0.25)
            wayland_ready = backend == "wayland" and _wayland_toplevel_ready(
                app_log.read_bytes()[-256 * 1024 :].decode("utf-8", errors="replace")
            )
            if (backend == "x11" and window_id is None) or (
                backend == "wayland" and not wayland_ready
            ):
                raise RuntimeError(
                    _failure_with_runner_output(
                        f"D4 PocketのGTK {backend} toplevel／surfaceを検出できませんでした",
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

            if backend == "x11":
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
                    exit_code = runner_process.wait(timeout=45)
                except subprocess.TimeoutExpired as error:
                    raise RuntimeError("GTK window close後にFlutter GTK runnerが終了しませんでした") from error
                if exit_code != 0:
                    raise RuntimeError(
                        _failure_with_runner_output(
                            f"Flutter GTK runnerが異常終了しました: {exit_code}",
                            app_log,
                            root,
                        )
                    )
            else:
                _request_owned_product_sigterm(
                    runner_process,
                    runner,
                    helper,
                    preexisting_runners,
                    preexisting_helpers,
                )
                if runner_process.poll() is None:
                    raise RuntimeError("Wayland smoke後にFlutter GTK launcher processが停止しませんでした")

            if xdg_data_home.exists() or xdg_data_home.is_symlink():
                raise RuntimeError("未導入状態のportable起動がXDG_DATA_HOMEを作成しました")

            cleanup_deadline = time.monotonic() + 10
            remaining: set[int] = set()
            remaining_runners: set[int] = set()
            while time.monotonic() < cleanup_deadline:
                remaining = _processes_using(helper) - preexisting_helpers
                remaining_runners = _processes_using(runner) - preexisting_runners
                if not remaining and not remaining_runners:
                    break
                time.sleep(0.2)
            if remaining or remaining_runners:
                raise RuntimeError(
                    "GTK終了後にBroker helper／Flutter runner processが残留しました"
                )
            stop_mode = "window-close" if backend == "x11" else "owned-process-group SIGTERM"
            print(
                f"Linux GTK {backend} bundle runtime試験 合格: 製品画面作成を確認; "
                f"終了方式={stop_mode}; "
                f"Broker helper観測済み={str(helper_seen).lower()}"
            )
        finally:
            if runner_process is not None:
                _terminate_owned_product(
                    runner_process,
                    runner,
                    helper,
                    preexisting_runners,
                    preexisting_helpers,
                )
            if window_manager is not None:
                _terminate_group(window_manager)
            if compositor is not None:
                _terminate_group(compositor)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--launcher", type=Path, required=True)
    parser.add_argument("--temp-root", type=Path, required=True)
    parser.add_argument("--backend", choices=("x11", "wayland"), default="x11")
    args = parser.parse_args()
    try:
        run_smoke(args.launcher, args.temp_root, args.backend)
    except Exception as error:
        print(f"Linux GTK bundle runtime試験 不合格: {error}")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
