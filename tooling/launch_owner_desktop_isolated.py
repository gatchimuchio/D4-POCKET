from __future__ import annotations

import json
import os
import shutil
import signal
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from tooling.flutter_workspace import IsolatedFlutterWorkspace


BROKER_READY_TIMEOUT_SECONDS = 5.0
PROCESS_STOP_TIMEOUT_SECONDS = 5.0


def _find_tool(name: str) -> str | None:
    resolved = shutil.which(name)
    if resolved is not None:
        return resolved
    if os.name == "nt":
        for suffix in (".exe", ".bat", ".cmd"):
            resolved = shutil.which(f"{name}{suffix}")
            if resolved is not None:
                return resolved
    return None


def _desktop_device() -> str:
    if sys.platform == "win32":
        return "windows"
    if sys.platform.startswith("linux"):
        return "linux"
    raise RuntimeError("Owner用開発ランチャーの対象Desktop OSではありません")


def _process_options(*, hidden: bool = False) -> dict[str, object]:
    if os.name == "nt":
        flags = getattr(subprocess, "CREATE_NEW_PROCESS_GROUP", 0)
        if hidden:
            flags |= getattr(subprocess, "CREATE_NO_WINDOW", 0)
        return {"creationflags": flags}
    return {"start_new_session": True}


def _runtime_environment(workspace: IsolatedFlutterWorkspace) -> dict[str, str]:
    environment = workspace.environment.copy()
    for name in ("APPDATA", "XDG_CONFIG_HOME"):
        if name in os.environ:
            environment[name] = os.environ[name]
        else:
            environment.pop(name, None)
    environment.pop("GUI_SHELL_BROKER_ENDPOINT_JSON", None)
    return environment


def _ensure_repository_directory(root: Path, relative: Path) -> Path:
    current = root
    for part in relative.parts:
        current = current / part
        try:
            metadata = current.lstat()
        except FileNotFoundError:
            current.mkdir()
            metadata = current.lstat()
        attributes = getattr(metadata, "st_file_attributes", 0)
        if current.is_symlink() or (os.name == "nt" and attributes & 0x400):
            raise RuntimeError("Broker永続store pathにsymlinkまたはreparse pointがあります")
        if not current.is_dir():
            raise RuntimeError("Broker永続store pathの構成要素がdirectoryではありません")
    resolved = current.resolve(strict=True)
    if resolved != root.resolve(strict=True) / relative:
        raise RuntimeError("Broker永続store pathがRepository rootから逸脱しています")
    return resolved


def _validate_session_file(path: Path) -> None:
    try:
        endpoint = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise RuntimeError("Broker session fileを読み取れないかJSONが不正です") from error
    if not isinstance(endpoint, dict):
        raise RuntimeError("Broker session fileのrootがobjectではありません")
    if (
        endpoint.get("credential_role") != "normal"
        or endpoint.get("host") != "127.0.0.1"
        or endpoint.get("transport") != "authenticated_loopback_tcp"
    ):
        raise RuntimeError("Flutterへ渡すBroker通常接続資格のmetadataが不正です")


def _wait_for_session(process: subprocess.Popen, session_file: Path) -> None:
    deadline = time.monotonic() + BROKER_READY_TIMEOUT_SECONDS
    while time.monotonic() < deadline:
        if session_file.is_file():
            _validate_session_file(session_file)
            exit_code = process.poll()
            if exit_code is not None:
                raise RuntimeError(f"Rust Brokerがsession準備後に終了しました: {exit_code}")
            return
        exit_code = process.poll()
        if exit_code is not None:
            raise RuntimeError(f"Rust Brokerがendpoint準備前に終了しました: {exit_code}")
        time.sleep(0.05)
    raise RuntimeError("Rust Broker endpoint fileが期限内に作成されませんでした")


def _kill_windows_process_tree(process: subprocess.Popen) -> None:
    subprocess.run(
        ["taskkill.exe", "/PID", str(process.pid), "/T", "/F"],
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    )


def _stop_process(process: subprocess.Popen | None, *, process_tree: bool = False) -> None:
    if process is None or process.poll() is not None:
        return

    if os.name == "nt":
        if process_tree:
            _kill_windows_process_tree(process)
        else:
            process.terminate()
    elif process_tree:
        try:
            os.killpg(process.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
    else:
        process.terminate()

    try:
        process.wait(timeout=PROCESS_STOP_TIMEOUT_SECONDS)
    except subprocess.TimeoutExpired:
        if os.name == "nt":
            if process_tree:
                _kill_windows_process_tree(process)
            else:
                process.kill()
        elif process_tree:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
        else:
            process.kill()
        process.wait(timeout=PROCESS_STOP_TIMEOUT_SECONDS)


def _run_owner_desktop(source_root: Path) -> int:
    device = _desktop_device()
    cargo = _find_tool("cargo")
    flutter = _find_tool("flutter")
    if cargo is None or flutter is None:
        missing = ", ".join(name for name, value in (("cargo", cargo), ("flutter", flutter)) if value is None)
        print(f"必要な開発toolがありません: {missing}", file=sys.stderr)
        return 2

    with IsolatedFlutterWorkspace(source_root) as workspace:
        assert workspace.root is not None
        assert workspace.temporary_root is not None
        snapshot_root = workspace.root
        helper_name = "gui_shell_rust_helper.exe" if os.name == "nt" else "gui_shell_rust_helper"
        helper = snapshot_root / "native" / "rust_helper" / "target" / "debug" / helper_name
        cargo_environment = workspace.environment.copy()
        cargo_environment.pop("GUI_SHELL_BROKER_ENDPOINT_JSON", None)
        cargo_target = Path(cargo_environment.get("CARGO_TARGET_DIR", "")).resolve(strict=False)
        if not cargo_target.is_relative_to(snapshot_root.resolve(strict=True)):
            raise RuntimeError("Cargo targetが一時source snapshot外を指しています")

        print("Git source snapshotをRepository／OneDrive外で起動します。生成物は終了後に回収します。")
        cargo_process = subprocess.Popen(
            [cargo, "build", "--locked", "--manifest-path", "native/rust_helper/Cargo.toml"],
            cwd=snapshot_root,
            env=cargo_environment,
            **_process_options(),
        )
        try:
            cargo_exit = cargo_process.wait()
        finally:
            _stop_process(cargo_process, process_tree=True)
        if cargo_exit != 0:
            return cargo_exit
        if not helper.is_file():
            raise RuntimeError("isolated Cargo build後にRust helperがありません")

        broker_root = _ensure_repository_directory(
            source_root.resolve(strict=True), Path(".gui_shell/broker/store")
        )
        session_file = workspace.temporary_root / "broker_session.json"
        broker_process: subprocess.Popen | None = None
        flutter_process: subprocess.Popen | None = None
        try:
            broker_process = subprocess.Popen(
                [
                    str(helper),
                    "broker-server",
                    "--store-dir",
                    str(broker_root),
                    "--session-file",
                    str(session_file),
                ],
                cwd=snapshot_root,
                env=_runtime_environment(workspace),
                stdin=subprocess.DEVNULL,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                **_process_options(hidden=True),
            )
            _wait_for_session(broker_process, session_file)

            flutter_environment = _runtime_environment(workspace)
            flutter_environment["GUI_SHELL_BROKER_ENDPOINT_JSON"] = str(session_file)
            flutter_process = subprocess.Popen(
                [flutter, "run", "-d", device],
                cwd=snapshot_root / "apps" / "desktop_flutter",
                env=flutter_environment,
                **_process_options(),
            )
            return flutter_process.wait()
        finally:
            cleanup_errors: list[Exception] = []
            try:
                _stop_process(flutter_process, process_tree=True)
            except (OSError, subprocess.SubprocessError) as error:
                cleanup_errors.append(error)
            try:
                _stop_process(broker_process)
            except (OSError, subprocess.SubprocessError) as error:
                cleanup_errors.append(error)
            if cleanup_errors:
                raise RuntimeError("Owner用起動processを停止できませんでした") from cleanup_errors[0]


def run_owner_desktop(source_root: Path = ROOT) -> int:
    try:
        return _run_owner_desktop(source_root)
    except KeyboardInterrupt:
        return 130
    except (OSError, RuntimeError, subprocess.SubprocessError) as error:
        print(f"Owner用Desktop開発起動に失敗しました: {error}", file=sys.stderr)
        return 2


def main() -> int:
    return run_owner_desktop()


if __name__ == "__main__":
    raise SystemExit(main())
