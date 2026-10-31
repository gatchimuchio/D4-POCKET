"""Linux Release bundleのFlutter GTK runnerを隔離X11／Wayland sessionで確認する。"""

from __future__ import annotations

import argparse
import hashlib
import os
import re
import shutil
import signal
import stat
import struct
import subprocess
import tempfile
import time
import zlib
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


def _weston_arguments(
    backend_argument: str,
    renderer_argument: str,
    display_name: str,
    *,
    enable_output_capture: bool,
) -> list[str]:
    arguments = ["weston"]
    if enable_output_capture:
        # This exposes Weston’s output-capture debug protocol. It is enabled only
        # for a private, run-owned compositor with no untrusted clients.
        arguments.append("--debug")
    arguments.extend(
        (backend_argument, renderer_argument, f"--socket={display_name}")
    )
    return arguments


def _sway_arguments(config_path: Path) -> list[str]:
    return ["sway", "--debug", "--config", str(config_path)]


def _decode_png_rgba(image: bytes) -> tuple[int, int, bytes]:
    signature = b"\x89PNG\r\n\x1a\n"
    if len(image) > 64 * 1024 * 1024 or not image.startswith(signature):
        raise RuntimeError("Westonの出力画像がPNG形式または上限内ではありません")

    offset = len(signature)
    width = height = bit_depth = color_type = None
    compressed = bytearray()
    saw_idat = False
    idat_sequence_ended = False
    saw_iend = False
    while offset < len(image):
        if offset + 12 > len(image):
            raise RuntimeError("PNG chunkが途中で切れています")
        length = struct.unpack_from(">I", image, offset)[0]
        end = offset + 12 + length
        if end > len(image):
            raise RuntimeError("PNG chunk長がfile境界を越えています")
        chunk_type = image[offset + 4 : offset + 8]
        chunk_data = image[offset + 8 : offset + 8 + length]
        expected_crc = struct.unpack_from(">I", image, offset + 8 + length)[0]
        if len(chunk_type) != 4 or any(
            not (65 <= byte <= 90 or 97 <= byte <= 122) for byte in chunk_type
        ):
            raise RuntimeError("PNG chunk名が不正です")
        if zlib.crc32(chunk_type + chunk_data) & 0xFFFFFFFF != expected_crc:
            raise RuntimeError("PNG chunkのCRCが一致しません")
        if offset == len(signature):
            if chunk_type != b"IHDR" or length != 13:
                raise RuntimeError("PNG先頭に正しいIHDRがありません")
            width, height, bit_depth, color_type, compression, filtering, interlace = (
                struct.unpack(">IIBBBBB", chunk_data)
            )
            if (
                width < 1
                or height < 1
                or width * height > 16_777_216
                or bit_depth != 8
                or color_type not in (2, 6)
                or compression != 0
                or filtering != 0
                or interlace != 0
            ):
                raise RuntimeError("PNGの画像形式または寸法が対応範囲外です")
        elif chunk_type == b"IHDR":
            raise RuntimeError("PNGに重複IHDRがあります")
        elif chunk_type not in (b"PLTE", b"IDAT", b"IEND") and not (
            chunk_type[0] & 0x20
        ):
            raise RuntimeError("PNGに未対応のcritical chunkがあります")

        if chunk_type == b"IDAT":
            if idat_sequence_ended:
                raise RuntimeError("PNGのIDAT chunkが連続していません")
            saw_idat = True
            compressed.extend(chunk_data)
        elif chunk_type == b"IEND":
            if length != 0 or not saw_idat:
                raise RuntimeError("PNGのIENDまたはIDATが不正です")
            saw_iend = True
            offset = end
            break
        elif saw_idat:
            idat_sequence_ended = True
        offset = end

    if not saw_iend or offset != len(image) or width is None or height is None:
        raise RuntimeError("PNG終端が欠落しているか余分なdataがあります")

    channels = 4 if color_type == 6 else 3
    row_length = width * channels
    decoded_length = height * (row_length + 1)
    if decoded_length > 64 * 1024 * 1024:
        raise RuntimeError("PNG展開後の画像が上限を超えます")
    inflater = zlib.decompressobj()
    filtered = inflater.decompress(bytes(compressed), decoded_length + 1)
    if len(filtered) > decoded_length or inflater.unconsumed_tail:
        raise RuntimeError("PNG展開後の画像が宣言寸法を超えます")
    filtered += inflater.flush()
    if (
        len(filtered) != decoded_length
        or not inflater.eof
        or inflater.unused_data
    ):
        raise RuntimeError("PNG展開dataの長さまたは終端が不正です")

    rgba = bytearray(width * height * 4)
    previous = bytearray(row_length)
    source_offset = 0
    pixel_offset = 0

    def paeth(left: int, above: int, upper_left: int) -> int:
        estimate = left + above - upper_left
        distances = (
            abs(estimate - left),
            abs(estimate - above),
            abs(estimate - upper_left),
        )
        return (left, above, upper_left)[distances.index(min(distances))]

    for _ in range(height):
        filter_type = filtered[source_offset]
        source_offset += 1
        encoded_row = filtered[source_offset : source_offset + row_length]
        source_offset += row_length
        row = bytearray(row_length)
        if filter_type > 4:
            raise RuntimeError("PNGに未対応の行filterがあります")
        for index, value in enumerate(encoded_row):
            left = row[index - channels] if index >= channels else 0
            above = previous[index]
            upper_left = previous[index - channels] if index >= channels else 0
            if filter_type == 1:
                value += left
            elif filter_type == 2:
                value += above
            elif filter_type == 3:
                value += (left + above) // 2
            elif filter_type == 4:
                value += paeth(left, above, upper_left)
            row[index] = value & 0xFF

        if channels == 4:
            rgba[pixel_offset : pixel_offset + row_length] = row
            pixel_offset += row_length
        else:
            for index in range(0, row_length, 3):
                rgba[pixel_offset : pixel_offset + 3] = row[index : index + 3]
                rgba[pixel_offset + 3] = 255
                pixel_offset += 4
        previous = row

    return width, height, bytes(rgba)


def _different_pixel_count(
    first: tuple[int, int, bytes], second: tuple[int, int, bytes]
) -> int:
    first_width, first_height, first_pixels = first
    second_width, second_height, second_pixels = second
    if (
        first_width != second_width
        or first_height != second_height
        or len(first_pixels) != len(second_pixels)
    ):
        raise RuntimeError("Westonの比較画像寸法が一致しません")
    return sum(
        first_pixels[offset : offset + 4] != second_pixels[offset : offset + 4]
        for offset in range(0, len(first_pixels), 4)
    )


def _capture_weston_frame(
    environment: dict[str, str], pictures_dir: Path
) -> tuple[tuple[int, int, bytes], str]:
    if pictures_dir.exists() or pictures_dir.is_symlink():
        raise RuntimeError("Weston画面取得用directoryは未使用でなければなりません")
    parent = pictures_dir.parent
    if parent.is_symlink() or not parent.is_dir():
        raise RuntimeError("Weston画面取得rootが通常directoryではありません")
    pictures_dir.mkdir(mode=0o700)
    if os.name == "posix" and stat.S_IMODE(pictures_dir.stat().st_mode) & 0o077:
        raise RuntimeError("Weston画面取得directoryがprivateではありません")

    capture_environment = environment.copy()
    capture_environment["XDG_PICTURES_DIR"] = str(pictures_dir)
    image_path: Path | None = None
    try:
        if shutil.which("weston-screenshooter", path=capture_environment.get("PATH")) is None:
            raise RuntimeError("Wayland出力取得用weston-screenshooterがありません")
        result = subprocess.run(
            ["weston-screenshooter"],
            env=capture_environment,
            capture_output=True,
            check=False,
            timeout=20,
        )
        if result.returncode != 0:
            raise RuntimeError(
                f"Wayland compositorの出力取得に失敗しました: exit={result.returncode}"
            )
        candidates = list(pictures_dir.glob("wayland-screenshot-*.png"))
        if len(candidates) != 1:
            raise RuntimeError("Weston出力取得fileが一意に生成されませんでした")
        image_path = candidates[0]
        metadata = image_path.lstat()
        if (
            not stat.S_ISREG(metadata.st_mode)
            or stat.S_ISLNK(metadata.st_mode)
            or metadata.st_size < 1
            or metadata.st_size > 64 * 1024 * 1024
        ):
            raise RuntimeError("Weston出力取得fileが通常fileまたはsize上限を満たしません")
        os.chmod(image_path, 0o600)
        image = image_path.read_bytes()
        frame = _decode_png_rgba(image)
        return frame, hashlib.sha256(image).hexdigest()
    finally:
        if image_path is not None and image_path.exists() and not image_path.is_symlink():
            image_path.unlink()
        if pictures_dir.exists() and not pictures_dir.is_symlink():
            pictures_dir.rmdir()


def _weston_backend_is_unsupported(backend_argument: str, output: str) -> bool:
    return (
        backend_argument == "--backend=headless"
        and 'Error: unknown backend "headless"' in output
    )


def _weston_renderer_is_unsupported(renderer_argument: str, output: str) -> bool:
    return (
        renderer_argument == "--renderer=pixman"
        and "fatal: unhandled option: --renderer=pixman" in output
    )


def _start_wayland_compositor(
    runtime: Path,
    environment: dict[str, str],
    log_path: Path,
    *,
    enable_output_capture: bool = False,
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
    renderer_arguments = ("--renderer=pixman", "--use-pixman")
    attempt = 0
    for backend_argument in backend_arguments:
        for renderer_argument in renderer_arguments:
            with log_path.open("xb" if attempt == 0 else "ab") as log:
                compositor = subprocess.Popen(
                    _weston_arguments(
                        backend_argument,
                        renderer_argument,
                        display_name,
                        enable_output_capture=enable_output_capture,
                    ),
                    env=compositor_environment,
                    stdin=subprocess.DEVNULL,
                    stdout=log,
                    stderr=subprocess.STDOUT,
                    start_new_session=True,
                )
            attempt += 1

            deadline = time.monotonic() + 20
            backend_unsupported = False
            renderer_unsupported = False
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
                        backend_unsupported = True
                        break
                    if _weston_renderer_is_unsupported(renderer_argument, output):
                        if socket_path.exists() or socket_path.is_symlink():
                            raise RuntimeError(
                                "Westonのrenderer拒否後にWayland endpointが残りました"
                            )
                        renderer_unsupported = True
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
                    print(
                        "Weston headless options: "
                        f"{backend_argument}, {renderer_argument}"
                    )
                    return compositor, display_name
                _terminate_group(compositor)
                raise RuntimeError(
                    "Westonが通常Unix socket以外のWayland endpointを作成しました"
                )

            else:
                _terminate_group(compositor)
                raise RuntimeError(
                    _failure_with_runner_output(
                        "隔離Weston compositorのWayland socketが時間内に作成されませんでした",
                        log_path,
                        runtime.parent,
                    )
                )
            if backend_unsupported:
                break
            if renderer_unsupported:
                continue

    raise RuntimeError(
        _failure_with_runner_output(
            "Westonのheadless backend指定が全て拒否されました",
            log_path,
            runtime.parent,
        )
    )


def _start_sway_compositor(
    runtime: Path,
    environment: dict[str, str],
    log_path: Path,
) -> tuple[subprocess.Popen[bytes], str]:
    """検査専用の隔離Sway headless Wayland sessionを起動する。"""
    if not runtime.is_dir() or runtime.is_symlink():
        raise RuntimeError("隔離Sway runtimeが通常directoryではありません")
    if log_path.exists() or log_path.is_symlink():
        raise RuntimeError("隔離Sway logが事前に存在しています")
    config_path = log_path.with_name("sway.conf")
    if config_path.exists() or config_path.is_symlink():
        raise RuntimeError("隔離Sway configが事前に存在しています")

    existing_sockets = [
        path
        for path in runtime.glob("wayland-*")
        if stat.S_ISSOCK(path.lstat().st_mode)
    ]
    if existing_sockets:
        raise RuntimeError("隔離Sway runtimeに既存Wayland socketがあります")

    with config_path.open("x", encoding="utf-8") as config:
        config.write("xwayland disable\n")
    compositor_environment = environment.copy()
    for name in (
        "DISPLAY",
        "XAUTHORITY",
        "WAYLAND_DISPLAY",
        "GDK_BACKEND",
        "SWAYSOCK",
    ):
        compositor_environment.pop(name, None)
    compositor_environment.update(
        {
            "XDG_CURRENT_DESKTOP": "sway",
            "XDG_SESSION_TYPE": "wayland",
            "WLR_BACKENDS": "headless",
            "WLR_RENDERER": "pixman",
            "WLR_LIBINPUT_NO_DEVICES": "1",
        }
    )
    with log_path.open("xb") as log:
        compositor = subprocess.Popen(
            _sway_arguments(config_path),
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
                    f"隔離Sway compositorが起動前に終了しました: {compositor.returncode}",
                    log_path,
                    runtime.parent,
                )
            )
        sockets = [
            path
            for path in runtime.glob("wayland-*")
            if stat.S_ISSOCK(path.lstat().st_mode)
        ]
        if len(sockets) == 1:
            display_name = sockets[0].name
            print(
                "検査用Sway: WLR_BACKENDS=headless、WLR_RENDERER=pixman、"
                "wtype仮想keyboard入力を有効化"
            )
            return compositor, display_name
        if len(sockets) > 1:
            _terminate_group(compositor)
            raise RuntimeError("隔離Sway runtimeに複数Wayland socketが作られました")
        time.sleep(0.1)

    _terminate_group(compositor)
    raise RuntimeError(
        _failure_with_runner_output(
            "隔離Sway compositorのWayland socketが時間内に作成されませんでした",
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
