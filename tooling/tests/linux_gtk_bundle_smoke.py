"""Linux Release bundleのFlutter GTK runnerを隔離X11／Wayland sessionで確認する。"""

from __future__ import annotations

import argparse
import os
import re
import shutil
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


def _request_x11_window_manager_close(window_id: str, environment: dict[str, str]) -> None:
    activated = subprocess.run(
        ["xdotool", "windowactivate", "--sync", window_id],
        env=environment,
        capture_output=True,
        check=False,
        timeout=10,
    )
    if activated.returncode != 0:
        raise RuntimeError("D4 Pocket GTK windowをwindow manager経由でactivateできませんでした")
    closed = subprocess.run(
        ["xdotool", "windowclose", window_id],
        env=environment,
        capture_output=True,
        check=False,
        timeout=10,
    )
    if closed.returncode != 0:
        raise RuntimeError("D4 Pocket GTK windowへWM_DELETE_WINDOWを送れませんでした")


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


def _wayland_socket_connected(output: str, socket_path: Path) -> bool:
    expected_path = f'sun_path="{socket_path.as_posix()}"'
    return any(
        "connect(" in line
        and expected_path in line
        and re.search(r"\)\s+= 0(?:\s|$)", line)
        for line in output.splitlines()
    )


def _weston_backend_is_unsupported(backend_argument: str, output: str) -> bool:
    return (
        backend_argument == "--backend=headless"
        and 'Error: unknown backend "headless"' in output
    )


def _start_wayland_compositor(
    runtime: Path, environment: dict[str, str], log_path: Path
) -> tuple[subprocess.Popen[bytes], str]:
    display_name = "w"
    socket_path = runtime / display_name
    if socket_path.exists() or socket_path.is_symlink():
        raise RuntimeError("隔離Wayland socketが事前に存在しています")

    compositor_environment = environment.copy()
    for name in (
        "DISPLAY",
        "XAUTHORITY",
        "WAYLAND_DISPLAY",
        "GDK_BACKEND",
        "XDG_SESSION_TYPE",
    ):
        compositor_environment.pop(name, None)
    backend_arguments = ("--backend=headless", "--backend=headless-backend.so")
    for attempt, backend_argument in enumerate(backend_arguments):
        with log_path.open("xb" if attempt == 0 else "ab") as log:
            compositor = subprocess.Popen(
                [
                    "weston",
                    backend_argument,
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
                try:
                    output = log_path.read_bytes()[-4096:].decode(
                        "utf-8", errors="replace"
                    )
                except OSError:
                    output = ""
                if _weston_backend_is_unsupported(backend_argument, output):
                    if socket_path.exists() or socket_path.is_symlink():
                        raise RuntimeError(
                            "Westonのbackend拒否後にWayland endpointが残りました"
                        )
                    break
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
                print(f"Weston headless backend option: {backend_argument}")
                return compositor, display_name
            _terminate_group(compositor)
            raise RuntimeError("Westonが通常Unix socket以外のWayland endpointを作成しました")

        else:
            _terminate_group(compositor)
            raise RuntimeError(
                _failure_with_runner_output(
                    "隔離Weston compositorのWayland socketが時間内に作成されませんでした",
                    log_path,
                    runtime.parent,
                )
            )

    raise RuntimeError(
        _failure_with_runner_output(
            "Westonのheadless backend指定が全て拒否されました",
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

    with tempfile.TemporaryDirectory(prefix="d4p-", dir=temp_root) as raw:
        root = Path(raw)
        os.chmod(root, 0o700)
        home = root / "home"
        runtime = root / "r"
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
        wayland_socket: Path | None = None
        if backend == "wayland":
            env["XDG_SESSION_TYPE"] = "wayland"
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
            wayland_socket = runtime / wayland_display
        app_log = root / "app-output.log"
        connect_trace = root / "wayland-connect.log"
        runner_process: subprocess.Popen[bytes] | None = None
        try:
            if backend == "x11":
                time.sleep(0.5)
                if window_manager is None or window_manager.poll() is not None:
                    raise RuntimeError("隔離X11 window managerが起動しませんでした")
            with app_log.open("xb") as log:
                command = [str(launcher)]
                if backend == "wayland":
                    if shutil.which("strace") is None:
                        raise RuntimeError("Wayland接続観測用straceがありません")
                    command = [
                        "strace",
                        "-f",
                        "-e",
                        "trace=connect",
                        "-s",
                        "256",
                        "-o",
                        str(connect_trace),
                        "--",
                        str(launcher),
                    ]
                runner_process = subprocess.Popen(
                    command,
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
                    output = (
                        connect_trace.read_text(encoding="utf-8", errors="replace")
                        if connect_trace.exists()
                        else ""
                    )
                    if wayland_socket is not None and _wayland_socket_connected(
                        output, wayland_socket
                    ):
                        break
                time.sleep(0.25)
            wayland_output = (
                connect_trace.read_text(encoding="utf-8", errors="replace")
                if connect_trace.exists()
                else ""
            )
            wayland_ready = (
                backend == "wayland"
                and wayland_socket is not None
                and _wayland_socket_connected(wayland_output, wayland_socket)
            )
            if (backend == "x11" and window_id is None) or (
                backend == "wayland" and not wayland_ready
            ):
                detail = _failure_with_runner_output(
                    f"D4 PocketのGTK {backend}接続を検出できませんでした",
                    app_log,
                    root,
                )
                if backend == "wayland" and wayland_output:
                    trace_tail = wayland_output[-2048:].replace(str(root), "<隔離root>")
                    detail += f"\nconnect syscall診断（末尾2048文字以内）:\n{trace_tail}"
                raise RuntimeError(detail)
            if backend == "wayland" and runner_process.poll() is not None:
                raise RuntimeError("Wayland接続観測後に製品launcherが終了しました")

            helper_seen = False
            for _ in range(20):
                if _processes_using(helper) - preexisting_helpers:
                    helper_seen = True
                    break
                time.sleep(0.1)

            if backend == "x11":
                # GTKはFlutterの最初のOpenGL描画frameが準備できる前にwindowを画面へ登録します。
                # 初期frameが落ち着いてからX11に描画領域を破棄させます。
                time.sleep(5)
                if runner_process.poll() is not None:
                    raise RuntimeError(
                        _failure_with_runner_output(
                            "Flutter GTK runnerが初回frame表示前に終了しました: "
                            f"{runner_process.returncode}",
                            app_log,
                            root,
                        )
                    )
                visible = subprocess.run(
                    ["xdotool", "search", "--onlyvisible", "--name", pattern],
                    env=env,
                    capture_output=True,
                    check=False,
                    timeout=5,
                )
                visible_ids = visible.stdout.decode("ascii", errors="ignore").splitlines()
                if visible.returncode != 0 or window_id not in visible_ids:
                    raise RuntimeError("GTK windowが初回frame待機中に可視状態でなくなりました")
                _request_x11_window_manager_close(window_id, env)
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
            observation = (
                "製品GTK window close後の終了"
                if backend == "x11"
                else "隔離Wayland socketへの成功接続とlive processを確認"
            )
            print(
                f"Linux GTK {backend} bundle runtime試験 合格: {observation}; 終了方式={stop_mode}; "
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
