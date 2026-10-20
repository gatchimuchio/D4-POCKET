"""実Linux GTK sessionでBroker-installed desktop entryを起動・終了する。"""

from __future__ import annotations

import argparse
import hashlib
import hmac
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


def _accessible_nodes(root):
    """Installed UIの現在のAT-SPI treeをboundedに列挙する。"""
    pending = [root]
    visited = 0
    while pending and visited < 12000:
        node = pending.pop()
        visited += 1
        try:
            name = node.name or ""
        except Exception:
            name = ""
        yield node, name
        try:
            count = min(node.childCount, 12000 - visited)
            for index in reversed(range(count)):
                try:
                    pending.append(node.getChildAtIndex(index))
                except Exception:
                    continue
        except Exception:
            continue


def _find_accessible(name: str, *, root=None, timeout: float = 10.0):
    try:
        import pyatspi
    except ImportError as error:
        raise RuntimeError("AT-SPI UI操作にpython3-pyatspiが必要です") from error

    deadline = time.monotonic() + timeout
    last_names: list[str] = []
    while time.monotonic() < deadline:
        search_root = root if root is not None else pyatspi.Registry.getDesktop(0)
        for node, accessible_name in _accessible_nodes(search_root):
            if accessible_name == name or name in accessible_name:
                return node
            if accessible_name and any(
                token in accessible_name
                for token in ("D4 Pocket", "Owner確認", "Manifest file")
            ):
                last_names.append(accessible_name[:160])
        time.sleep(0.15)
    observed = ", ".join(dict.fromkeys(last_names[-12:]))
    raise RuntimeError(f"AT-SPI要素を確認できません: {name}; observed={observed or '(none)'}")


def _activate_accessible(node, name: str) -> None:
    try:
        actions = node.queryAction()
        for index in range(actions.nActions):
            action_name = actions.getName(index).lower()
            if action_name in {"click", "press", "activate"}:
                if actions.doAction(index):
                    return
    except Exception as error:
        raise RuntimeError(f"AT-SPI要素を操作できません: {name}") from error
    raise RuntimeError(f"AT-SPI要素に操作可能なactionがありません: {name}")


def _accessible_descendants(root) -> list[tuple[object, str]]:
    return list(_accessible_nodes(root))


def _trigger_export_from_installed_ui(environment: dict[str, str], window_id: str) -> None:
    subprocess.run(
        ["xdotool", "windowactivate", "--sync", window_id],
        env=environment,
        check=True,
        timeout=5,
    )
    subprocess.run(
        ["xdotool", "key", "--clearmodifiers", "ctrl+k"],
        env=environment,
        check=True,
        timeout=5,
    )
    _find_accessible("コマンドパレット", timeout=10)
    subprocess.run(
        ["xdotool", "type", "--clearmodifiers", "--delay", "30", "export"],
        env=environment,
        check=True,
        timeout=5,
    )
    subprocess.run(
        ["xdotool", "key", "--clearmodifiers", "Return"],
        env=environment,
        check=True,
        timeout=5,
    )
    button = _find_accessible("Owner確認してManifest fileを生成", timeout=20)
    _activate_accessible(button, "Owner確認してManifest fileを生成")


def _answer_owner_dialog(environment: dict[str, str], decision: str) -> None:
    if decision not in {"許可", "拒否"}:
        raise ValueError("Owner確認の回答値が不正")
    result = subprocess.run(
        ["xdotool", "search", "--onlyvisible", "--name", "^D4 Pocket Owner確認$"],
        env=environment,
        capture_output=True,
        check=False,
        timeout=5,
    )
    if result.returncode != 0:
        raise RuntimeError("GTK native Owner確認dialogが表示されません")
    dialog_ids = result.stdout.decode("ascii", errors="ignore").splitlines()
    if len(dialog_ids) != 1:
        raise RuntimeError("native Owner確認dialogが一意ではありません")
    dialog = _find_accessible("D4 Pocket Owner確認", timeout=10)
    descendants = _accessible_descendants(dialog)
    names = [accessible_name for _, accessible_name in descendants]
    if not any("対象platform: Linux" in value for value in names):
        raise RuntimeError("Owner確認summaryにLinux対象platformがありません")
    if not any("要求hash:" in value or "payload hash:" in value for value in names):
        raise RuntimeError("Owner確認summaryに要求hashがありません")
    button = next(
        (
            node
            for node, accessible_name in descendants
            if accessible_name.strip() == decision
        ),
        None,
    )
    if button is None:
        raise RuntimeError(f"native Owner確認dialogに{decision}操作がありません")
    _activate_accessible(button, decision)


def _verify_live_audit(store: Path) -> dict[str, int]:
    audit_path = store / "audit.jsonl"
    anchor_path = store / "audit_anchor.json"
    key_path = store / "audit_anchor.key"
    events = [
        json.loads(line)
        for line in audit_path.read_text(encoding="utf-8").splitlines()
        if line
    ]
    if not events:
        raise RuntimeError("installed productのLIVE_RUNTIME Auditが空です")
    previous_hash: str | None = None
    for index, event in enumerate(events, start=1):
        if (
            event.get("event_id") != f"broker-audit-{index}"
            or event.get("previous_event_hash") != previous_hash
        ):
            raise RuntimeError("installed product Auditのevent IDまたはprevious linkが不正です")
        hash_input = "|".join(
            (
                event["event_id"],
                event["request_id"],
                event["operation"],
                event["decision"],
                event["reason"],
                event["evidence_source"],
                event["payload_hash"],
                previous_hash or "",
            )
        )
        expected = "sha256:" + hashlib.sha256(hash_input.encode("utf-8")).hexdigest()
        if not hmac.compare_digest(event.get("event_hash", ""), expected):
            raise RuntimeError("installed product Audit event hashが不正です")
        previous_hash = expected

    anchor = json.loads(anchor_path.read_text(encoding="utf-8"))
    key = key_path.read_bytes()
    anchor_input = (
        f"version=1|event_count={len(events)}|head_event_hash={previous_hash or ''}"
    ).encode("utf-8")
    expected_hmac = "sha256:" + hmac.new(key, anchor_input, hashlib.sha256).hexdigest()
    if (
        anchor.get("version") != 1
        or anchor.get("event_count") != len(events)
        or anchor.get("head_event_hash") != previous_hash
        or not hmac.compare_digest(anchor.get("anchor_hmac", ""), expected_hmac)
    ):
        raise RuntimeError("installed product Audit anchor／HMACが不正です")

    exports = [event for event in events if event.get("operation") == "GUI Shell書出し"]
    accepted = sum(event.get("decision") == "accepted" for event in exports)
    rejected = sum(event.get("decision") == "rejected" for event in exports)
    if accepted < 1 or rejected < 1:
        raise RuntimeError("GTK Owner拒否／許可の両方がLIVE_RUNTIME Auditにありません")
    return {"events": len(events), "export_accepted": accepted, "export_rejected": rejected}


def _verify_export_manifest(export_root: Path) -> dict[str, str]:
    files = list(export_root.glob("d4-pocket-app-*.json"))
    if len(files) != 1:
        raise RuntimeError(
            f"Broker Export directoryの新規Manifest数が1ではありません: {len(files)}"
        )
    path = files[0]
    metadata = path.lstat()
    if not stat.S_ISREG(metadata.st_mode) or metadata.st_nlink != 1:
        raise RuntimeError("Broker Export結果がprivate regular fileではありません")
    document = json.loads(path.read_text(encoding="utf-8"))
    manifest = document.get("manifest", {})
    app_id = manifest.get("app_identity", {}).get("app_id", "")
    audit_id = manifest.get("audit_store", {}).get("store_id", "")
    if (
        document.get("version") != 1
        or document.get("product") != "D4 Pocket"
        or manifest.get("app_identity", {}).get("target_platform") != "linux"
        or not re.fullmatch(r"d4-pocket-app-[0-9a-f]{32}", app_id)
        or not re.fullmatch(r"audit-store-[0-9a-f]{32}", audit_id)
        or app_id == APP_ID
        or audit_id == AUDIT_ID
        or manifest.get("audit_store", {}).get("inherited") is not False
        or set(manifest.get("inheritance_policy", {}).values()) != {"none"}
        or manifest.get("distribution_metadata", {}).get("signed") is not False
    ):
        raise RuntimeError("実UI Export結果のidentity／継承禁止／unsigned条件が不正です")
    return {"file": path.name, "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}


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
        broker_state = xdg_state_home / "d4-pocket/linux-broker"
        export_root = broker_state / "exports"
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
            deadline = time.monotonic() + 10
            while not export_root.exists() and time.monotonic() < deadline:
                time.sleep(0.1)
            if export_root.is_symlink() or not export_root.is_dir():
                raise RuntimeError("installed Brokerの固定Export directoryがありません")
            metadata = export_root.lstat()
            if not stat.S_ISDIR(metadata.st_mode) or metadata.st_uid != os.getuid() or metadata.st_mode & 0o077:
                raise RuntimeError("installed BrokerのExport directoryがprivate／owner-boundではありません")
            if any(export_root.iterdir()):
                raise RuntimeError("installed BrokerのExport directoryが受入れ前から空ではありません")

            _trigger_export_from_installed_ui(environment, window_id)
            _answer_owner_dialog(environment, "拒否")
            _find_accessible("GUI Shell独立App書出しは受理されませんでした", timeout=20)
            if any(export_root.iterdir()):
                raise RuntimeError("Owner拒否後にManifestまたは一時fileが作成されました")

            _trigger_export_from_installed_ui(environment, window_id)
            _answer_owner_dialog(environment, "許可")
            _find_accessible("独立Manifest fileを保存しました", timeout=30)
            manifest_evidence = _verify_export_manifest(export_root)

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

        audit_evidence = _verify_live_audit(broker_state / "store")
        _run_rust_test(RUST_TESTS[1], environment)
        print(
            json.dumps(
                {
                    "installed_entry": str(entries[0]),
                    "active_version": record["product_version"],
                    "package_sha256": record["package_sha256"],
                    "gtk_window": "opened_and_normally_closed",
                    "owned_processes": "all_stopped",
                    "installed_ui_owner_confirmation": "native_AT-SPI_rejected_then_approved_through_GTK_runner_private_pipe",
                    "manifest": manifest_evidence,
                    "live_audit": audit_evidence,
                    "runtime_audit": "durable_after_broker_restart",
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
