"""実Linux GTK sessionでBroker-installed desktop entryを起動・終了する。"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import signal
import stat
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
APP_ID = "d4-pocket-app-11111111111111111111111111111111"
AUDIT_ID = "audit-store-22222222222222222222222222222222"
WINDOW_TITLE = "D4 Pocket powered by GUI Shell"
RUST_TESTS = (
    "broker::update_center::tests::linux_installed_acceptance_stages_real_export_and_registers_entry",
    "broker::update_center::tests::linux_installed_acceptance_runtime_audit_is_durable",
)

if __package__:
    from .linux_gtk_bundle_smoke import _request_x11_window_manager_close
else:
    from linux_gtk_bundle_smoke import _request_x11_window_manager_close


def _processes_using(executables: set[Path]) -> set[int]:
    found: set[int] = set()
    for entry in Path("/proc").iterdir():
        if not entry.name.isdecimal():
            continue
        try:
            running = Path(os.readlink(entry / "exe"))
        except (FileNotFoundError, PermissionError, ProcessLookupError):
            continue
        if running in executables:
            found.add(int(entry.name))
    return found


def _desktop_entry_targets_fixed_launcher(entry: Path, launcher: Path) -> bool:
    try:
        metadata = entry.lstat()
        content = entry.read_text(encoding="utf-8")
    except OSError:
        return False
    if not stat.S_ISREG(metadata.st_mode) or metadata.st_nlink != 1:
        return False
    exec_lines = [line[5:] for line in content.splitlines() if line.startswith("Exec=")]
    escaped = str(launcher).replace("\\", "\\\\").replace('"', '\\"')
    escaped = escaped.replace("`", "\\`").replace("$", "\\$").replace("%", "%%")
    return exec_lines == [f'"{escaped}"']


def _resolve_regular_package(package: Path) -> Path:
    if package.is_symlink() or not package.is_file():
        raise RuntimeError("Linux Release packageが通常fileではありません")
    return package.resolve(strict=True)


def _safe_log_tail(path: Path, roots: tuple[Path, ...]) -> str:
    try:
        content = path.read_text(encoding="utf-8", errors="replace")[-4000:]
    except OSError:
        return ""
    for root in sorted((str(root) for root in roots), key=len, reverse=True):
        content = content.replace(root, "<isolated-root>")
    return content


def _visible_product_window(environment: dict[str, str]) -> str | None:
    result = subprocess.run(
        ["xdotool", "search", "--onlyvisible", "--name", f"^{re.escape(WINDOW_TITLE)}$"],
        env=environment,
        capture_output=True,
        check=False,
        timeout=5,
    )
    if result.returncode != 0:
        return None
    values = result.stdout.decode("ascii", errors="ignore").splitlines()
    return values[0] if values else None


def _owned_groups(pids: set[int]) -> set[int]:
    groups: set[int] = set()
    for pid in pids:
        try:
            group = os.getpgid(pid)
        except (ProcessLookupError, PermissionError):
            continue
        if group != os.getpgrp():
            groups.add(group)
    return groups


def _stop_owned_processes(pids: set[int], extra_process: subprocess.Popen[bytes] | None) -> None:
    groups = _owned_groups(pids)
    if extra_process is not None and extra_process.poll() is None:
        groups.add(extra_process.pid)
    for group in groups:
        try:
            os.killpg(group, signal.SIGTERM)
        except ProcessLookupError:
            continue
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        if not any(_group_exists(group) for group in groups):
            break
        time.sleep(0.1)
    for group in groups:
        if _group_exists(group):
            try:
                os.killpg(group, signal.SIGKILL)
            except ProcessLookupError:
                pass
    if extra_process is not None:
        try:
            extra_process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            pass


def _group_exists(group: int) -> bool:
    try:
        os.killpg(group, 0)
        return True
    except ProcessLookupError:
        return False
    except PermissionError:
        return True


def _run_rust_test(name: str, environment: dict[str, str]) -> None:
    completed = subprocess.run(
        [
            "cargo",
            "+1.95.0",
            "test",
            "--locked",
            "--manifest-path",
            str(ROOT / "native/rust_helper/Cargo.toml"),
            "--lib",
            name,
            "--",
            "--ignored",
            "--exact",
            "--test-threads=1",
        ],
        cwd=ROOT,
        env=environment,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        check=False,
        timeout=900,
    )
    if completed.stdout:
        print(completed.stdout[-12000:], end="")
    if completed.stderr:
        print(completed.stderr[-12000:], file=sys.stderr, end="")
    if completed.returncode != 0:
        raise RuntimeError(f"installed acceptance Rust test failed: {name}")


def run_installed_smoke(package: Path, temp_root: Path, acceptance_root: Path) -> None:
    if not os.environ.get("DISPLAY") or not os.environ.get("DBUS_SESSION_BUS_ADDRESS"):
        raise RuntimeError("隔離X11 displayとD-Bus sessionが必要です")
    if temp_root.is_symlink() or not temp_root.is_dir():
        raise RuntimeError("run固有一時rootが通常directoryではありません")
    temp_root = temp_root.resolve(strict=True)
    package = _resolve_regular_package(package)
    if acceptance_root.exists() or acceptance_root.is_symlink():
        raise RuntimeError("installed acceptance rootは未使用でなければなりません")
    if not acceptance_root.is_absolute() or temp_root not in acceptance_root.parents:
        raise RuntimeError("installed acceptance rootがrun固有一時root外です")
    acceptance_root.mkdir(mode=0o700)
    home = acceptance_root / "home"
    runtime = acceptance_root / "runtime"
    temporary = acceptance_root / "tmp"
    xdg_data_home = home / ".local/share"
    xdg_state_home = home / ".local/state"
    for directory in (
        home,
        runtime,
        temporary,
        home / ".config",
        home / ".cache",
        xdg_data_home,
        xdg_state_home,
    ):
        directory.mkdir(mode=0o700, parents=True, exist_ok=False)

    environment = {
        name: os.environ[name]
        for name in (
            "PATH",
            "DISPLAY",
            "DBUS_SESSION_BUS_ADDRESS",
            "LANG",
            "LC_ALL",
            "LC_CTYPE",
            "XAUTHORITY",
        )
        if os.environ.get(name)
    }
    environment.update(
        {
            "HOME": str(home),
            "TMPDIR": str(temporary),
            "XDG_CONFIG_HOME": str(home / ".config"),
            "XDG_CACHE_HOME": str(home / ".cache"),
            "XDG_DATA_HOME": str(xdg_data_home),
            "XDG_STATE_HOME": str(xdg_state_home),
            "XDG_RUNTIME_DIR": str(runtime),
            "XDG_DATA_DIRS": "/usr/local/share:/usr/share",
            "XDG_SESSION_TYPE": "x11",
            "GDK_BACKEND": "x11",
            "GUI_SHELL_PRODUCT_APP_ID": APP_ID,
            "GUI_SHELL_PRODUCT_AUDIT_STORE_ID": AUDIT_ID,
            "D4P_LINUX_ACCEPTANCE_PACKAGE": str(package),
            "D4P_LINUX_ACCEPTANCE_ROOT": str(acceptance_root),
            "CARGO_TARGET_DIR": str(temp_root / "linux-installed-cargo-target"),
        }
    )

    window_manager: subprocess.Popen[bytes] | None = None
    desktop_process: subprocess.Popen[bytes] | None = None
    product_paths: set[Path] = set()
    preexisting: set[int] = set()
    app_log = acceptance_root / "installed-app.log"
    try:
        _run_rust_test(RUST_TESTS[0], environment)
        product_root = xdg_data_home / "apps/d4-pocket" / APP_ID
        fixed_launcher = product_root / "gui_shell_desktop_launcher"
        record_path = product_root / "active_version.json"
        record = json.loads(record_path.read_text(encoding="utf-8"))
        stage = product_root / "versions" / f"{record['product_version']}-{record['package_sha256']}"
        active_launcher = stage / "gui_shell_desktop_launcher"
        runner = stage / "gui_shell_desktop"
        helper = stage / "libexec/gui_shell_linux_broker"
        if not all(path.is_file() and not path.is_symlink() for path in (fixed_launcher, active_launcher, runner, helper)):
            raise RuntimeError("Brokerが有効化したinstalled bundleの固定fileが不足しています")
        applications = xdg_data_home / "applications"
        entries = list(applications.glob("*.desktop"))
        if len(entries) != 1 or not _desktop_entry_targets_fixed_launcher(entries[0], fixed_launcher):
            raise RuntimeError("登録desktop entryが固定installed launcherだけを指していません")
        product_paths = {fixed_launcher.resolve(), active_launcher.resolve(), runner.resolve(), helper.resolve()}
        preexisting = _processes_using(product_paths)
        if preexisting:
            raise RuntimeError("受入れ開始前からtest product processが存在します")

        window_manager = subprocess.Popen(
            ["openbox", "--sm-disable"],
            env=environment,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            start_new_session=True,
        )
        time.sleep(0.5)
        if window_manager.poll() is not None:
            raise RuntimeError("隔離Openboxが起動しませんでした")

        with app_log.open("xb") as log:
            desktop_process = subprocess.Popen(
                ["gio", "launch", str(entries[0])],
                cwd=stage,
                env=environment,
                stdin=subprocess.DEVNULL,
                stdout=log,
                stderr=subprocess.STDOUT,
                start_new_session=True,
            )
            deadline = time.monotonic() + 90
            window_id: str | None = None
            launch_error_since: float | None = None
            while time.monotonic() < deadline:
                window_id = _visible_product_window(environment)
                running_products = _processes_using(product_paths) - preexisting
                if window_id is not None and running_products:
                    break
                if desktop_process.poll() is not None and desktop_process.returncode != 0:
                    if launch_error_since is None:
                        launch_error_since = time.monotonic()
                    elif time.monotonic() - launch_error_since >= 5:
                        tail = _safe_log_tail(app_log, (acceptance_root, temp_root))
                        raise RuntimeError(
                            f"gio launch failed: {desktop_process.returncode}\n{tail or '(no launcher output)'}"
                        )
                time.sleep(0.25)
            if window_id is None or not (_processes_using(product_paths) - preexisting):
                tail = _safe_log_tail(app_log, (acceptance_root, temp_root))
                raise RuntimeError(f"installed GTK windowを検出できませんでした\n{tail}")

            _request_x11_window_manager_close(window_id, environment)
            deadline = time.monotonic() + 60
            while time.monotonic() < deadline:
                if not (_processes_using(product_paths) - preexisting):
                    break
                time.sleep(0.2)
            remaining = _processes_using(product_paths) - preexisting
            if remaining:
                raise RuntimeError(f"installed終了後にowned product processが残留: {sorted(remaining)}")
            if desktop_process.poll() is None:
                desktop_process.wait(timeout=10)
            if desktop_process.returncode not in (0, None):
                raise RuntimeError(f"gio launch exited unexpectedly: {desktop_process.returncode}")

        _run_rust_test(RUST_TESTS[1], environment)
        print(
            json.dumps(
                {
                    "installed_entry": str(entries[0]),
                    "active_version": record["product_version"],
                    "package_sha256": record["package_sha256"],
                    "gtk_window": "opened_and_normally_closed",
                    "owned_processes": "all_stopped",
                    "runtime_audit": "durable_after_broker_restart",
                    "owner_ui_gesture": "not_simulated; Broker owner-confirmation API used by isolated test fixture",
                },
                ensure_ascii=False,
                separators=(",", ":"),
            )
        )
    finally:
        if product_paths:
            _stop_owned_processes(_processes_using(product_paths) - preexisting, desktop_process)
        elif desktop_process is not None and desktop_process.poll() is None:
            _stop_owned_processes(set(), desktop_process)
        if window_manager is not None and window_manager.poll() is None:
            try:
                os.killpg(window_manager.pid, signal.SIGTERM)
                window_manager.wait(timeout=5)
            except (ProcessLookupError, subprocess.TimeoutExpired):
                try:
                    os.killpg(window_manager.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
        if acceptance_root.exists():
            resolved = acceptance_root.resolve(strict=True)
            if resolved == temp_root or temp_root not in resolved.parents or acceptance_root.is_symlink():
                raise RuntimeError("cleanup対象がrun固有test root外またはsymlinkです")
            shutil.rmtree(resolved)
        if acceptance_root.exists() or acceptance_root.is_symlink():
            raise RuntimeError("installed acceptance test rootをcleanupできません")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package", type=Path, required=True)
    parser.add_argument("--temp-root", type=Path, required=True)
    parser.add_argument("--acceptance-root", type=Path, required=True)
    args = parser.parse_args()
    try:
        run_installed_smoke(args.package, args.temp_root, args.acceptance_root)
    except Exception as error:
        print(f"Linux installed product GTK受入れ 不合格: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
