"""実Linux GTK sessionでBroker-installed productのRollback／UI／Uninstallを検証する。"""

from __future__ import annotations

import argparse
from collections.abc import Callable
import hashlib
import hmac
import json
import os
import re
import secrets
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
UPDATE_ID = "linux-installed-catalog-display"
UPDATE_VERSION = "0.1.2"
WINDOW_TITLE = "D4 Pocket powered by GUI Shell"
OWNER_DIALOG_TITLE = "D4 Pocket Owner確認"
UNINSTALL_BUTTON = "D4 Pocketをアンインストール"
OWNER_DIALOG_TIMEOUT = 15.0
RUST_CATALOG_RELAY_TIMEOUT_TEST = (
    "linux_desktop_worker::tests::catalog_relay_wait_matches_broker_fetch_deadline_without_extending_other_operations"
)
RUST_HELPER_EXIT_CLASS_TEST = (
    "linux_desktop_worker::tests::helper_exit_code_exposes_only_a_bounded_io_error_class"
)
RUST_CATALOG_TLS_PROVIDER_TEST = (
    "broker::update_download::tests::catalog_client_builds_with_the_explicit_ring_crypto_provider"
)
RUST_STAGE_TEST = (
    "broker::update_center::tests::linux_installed_acceptance_stages_real_export_and_registers_entry"
)
RUST_ROLLBACK_SEED_TEST = (
    "broker::update_center::tests::linux_installed_acceptance_seeds_verified_rollback_target"
)
RUST_SECRET_SERVICE_TEST = (
    "broker::credential_storage::tests::linux_secret_service_actual_store_is_create_only_namespaced_and_cleaned"
)
RUST_AUDIT_TEST = (
    "broker::update_center::tests::linux_installed_acceptance_runtime_audit_is_durable"
)

if __package__:
    from .linux_gtk_bundle_smoke import (
        _request_x11_window_manager_close,
        _start_wayland_compositor,
        _terminate_group,
    )
else:
    from linux_gtk_bundle_smoke import (
        _request_x11_window_manager_close,
        _start_wayland_compositor,
        _terminate_group,
    )


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


def _catalog_failure_diagnostics(
    helper: Path, preexisting: set[int], audit_path: Path
) -> dict[str, object]:
    helper_pids = _processes_using({helper.resolve()}) - preexisting
    recent_audit: list[dict[str, object]] = []
    try:
        lines = audit_path.read_text(encoding="utf-8").splitlines()[-6:]
        for line in lines:
            try:
                event = json.loads(line)
            except json.JSONDecodeError:
                continue
            if not isinstance(event, dict):
                continue
            recent_audit.append(
                {
                    key: event[key]
                    for key in ("event_id", "operation", "decision", "evidence_source")
                    if isinstance(event.get(key), str)
                }
            )
    except (OSError, UnicodeError):
        pass
    return {
        "helper_alive": bool(helper_pids),
        "helper_count": len(helper_pids),
        "recent_audit": recent_audit,
    }


def _helper_exit_diagnostics(path: Path) -> dict[str, list[int]]:
    try:
        content = path.read_text(encoding="utf-8", errors="replace")[-4000:]
    except OSError:
        return {"exit_codes": [], "signals": []}
    exit_codes = [
        int(match.group(1))
        for match in re.finditer(
            r"Linux Broker helper exited with classified code ([0-9]{1,3})",
            content,
        )
    ]
    signals = [
        int(match.group(1))
        for match in re.finditer(
            r"Linux Broker helper terminated by signal ([0-9]{1,3})", content
        )
    ]
    return {
        "exit_codes": [value for value in exit_codes if 0 <= value <= 255][-4:],
        "signals": [value for value in signals if 1 <= value <= 255][-4:],
    }


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


def _wayland_launch_environment(
    environment: dict[str, str], display: str
) -> dict[str, str]:
    result = environment.copy()
    for name in ("DISPLAY", "XAUTHORITY"):
        result.pop(name, None)
    result.update(
        {
            "GDK_BACKEND": "wayland",
            "XDG_SESSION_TYPE": "wayland",
            "WAYLAND_DISPLAY": display,
        }
    )
    return result


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


def _wait_for_catalog_fetch_status(root, *, timeout: float = 60.0) -> str:
    success_prefix = "Brokerが署名検証済み候補を取得しました（"
    failure_prefix = "更新候補の取得に失敗しました:"
    unconfigured = "Brokerに署名済み配布元が構成されていません。"
    expected_success = f"{success_prefix}1件）。"
    deadline = time.monotonic() + timeout
    observed: list[str] = []
    while time.monotonic() < deadline:
        names = [name for _, name in _accessible_nodes(root)]
        statuses = [
            name
            for name in names
            if name.startswith((success_prefix, failure_prefix)) or name == unconfigured
        ]
        observed.extend(name for name in names if "候補取得" in name)
        observed[:] = observed[-24:]
        if statuses:
            status = statuses[-1]
            if status.startswith(failure_prefix):
                raise RuntimeError(f"installed UI catalog fetch failed: {status[:400]}")
            if status == unconfigured:
                raise RuntimeError(f"installed UI catalog trust is unconfigured: {status}")
            if status != expected_success:
                raise RuntimeError(
                    f"installed UI catalog returned an unexpected result: {status[:400]}"
                )
            return status
        time.sleep(0.15)
    observed_text = ", ".join(dict.fromkeys(observed[-6:]))
    raise RuntimeError(
        "installed UI catalog fetch did not expose a terminal status"
        + (f": {observed_text}" if observed_text else "")
    )


def _actionable_node(node, *, max_ancestors: int = 5):
    """Flutterの可視label leafからAT-SPI actionを持つ親buttonを解決する。"""
    current = node
    for depth in range(max_ancestors + 1):
        try:
            actions = current.queryAction()
            for index in range(actions.nActions):
                if actions.getName(index).lower() in {
                    "click",
                    "press",
                    "activate",
                    "tap",
                }:
                    return current
        except Exception:
            pass
        if depth == max_ancestors:
            break
        current = _accessible_parent(current)
        if current is None:
            break
    return None


def _accessible_parent(node):
    """pyatspi／GIの実APIを優先し、legacy propertyにも対応する。"""
    for method_name in ("get_parent", "getParent"):
        try:
            getter = getattr(node, method_name)
        except Exception:
            continue
        if callable(getter):
            try:
                return getter()
            except Exception:
                continue
    try:
        return node.parent
    except Exception:
        return None


def _component_center(node) -> tuple[int, int] | None:
    try:
        import pyatspi

        bounds = node.queryComponent().getExtents(pyatspi.XY_SCREEN)
    except Exception:
        return None
    x, y = int(bounds.x), int(bounds.y)
    width, height = int(bounds.width), int(bounds.height)
    if width <= 0 or height <= 0 or width > 4096 or height > 4096:
        return None
    if abs(x) > 8192 or abs(y) > 8192:
        return None
    return x + width // 2, y + height // 2


def _accessibility_debug(node, *, max_ancestors: int = 5) -> str:
    try:
        import pyatspi
    except ImportError:
        return "pyatspi_unavailable"
    details = []
    current = node
    for depth in range(max_ancestors + 1):
        try:
            role = current.getRoleName()
        except Exception:
            role = "unknown"
        try:
            actions = current.queryAction()
            action_names = [
                actions.getName(index) for index in range(min(actions.nActions, 8))
            ]
        except Exception:
            action_names = []
        try:
            showing = current.getState().contains(pyatspi.STATE_SHOWING)
        except Exception:
            showing = "unknown"
        try:
            bounds = current.queryComponent().getExtents(pyatspi.DESKTOP_COORDS)
            extents = f"{bounds.x},{bounds.y},{bounds.width},{bounds.height}"
        except Exception as error:
            extents = f"unavailable:{type(error).__name__}"
        details.append(
            f"depth={depth},role={role},showing={showing},bounds={extents},actions={action_names}"
        )
        current = _accessible_parent(current)
        if current is None:
            break
    return "; ".join(details)


def _find_operable_accessible(name: str, *, root=None, timeout: float = 10.0):
    try:
        import pyatspi
    except ImportError as error:
        raise RuntimeError("AT-SPI UI操作にpython3-pyatspiが必要です") from error

    deadline = time.monotonic() + timeout
    last_debug = "no matching AT-SPI node"
    while time.monotonic() < deadline:
        search_root = root if root is not None else pyatspi.Registry.getDesktop(0)
        matches = [
            node
            for node, accessible_name in _accessible_nodes(search_root)
            if name in accessible_name.strip()
        ]
        for node in matches:
            actionable = _actionable_node(node)
            if actionable is not None:
                return actionable
            if _component_center(node) is not None:
                return node
            last_debug = _accessibility_debug(node)
        time.sleep(0.15)
    raise RuntimeError(
        f"AT-SPIで操作可能なGTK controlを確認できません: {name}; {last_debug}"
    )


def _activate_accessible(node, name: str, environment: dict[str, str]) -> None:
    try:
        actions = node.queryAction()
        for index in range(actions.nActions):
            action_name = actions.getName(index).lower()
            if action_name in {"click", "press", "activate", "tap"}:
                if actions.doAction(index):
                    return
    except Exception:
        pass
    center = _component_center(node)
    if center is None:
        raise RuntimeError(f"AT-SPI action／可視boundsがありません: {name}")
    clicked = subprocess.run(
        ["xdotool", "mousemove", "--sync", str(center[0]), str(center[1]), "click", "1"],
        env=environment,
        capture_output=True,
        check=False,
        timeout=5,
    )
    if clicked.returncode != 0:
        raise RuntimeError(f"GTK controlのAT-SPI座標clickに失敗しました: {name}")


def _accessible_descendants(root) -> list[tuple[object, str]]:
    return list(_accessible_nodes(root))


def _accessible_text(node) -> str:
    """GTK labels may expose their body through AT-SPI Text, not Accessible.name."""
    try:
        text = node.queryText()
        value = text.getText(0, -1)
        if not value:
            length = int(text.characterCount)
            if length < 0 or length > 8192:
                return ""
            value = text.getText(0, length)
        if len(value) > 8192:
            return ""
        return value if isinstance(value, str) else ""
    except Exception:
        return ""


def _bounded_owner_text_diagnostics(values: list[str]) -> str:
    observed = []
    for value in values:
        if not value.strip():
            continue
        value = re.sub(r"\b[A-Za-z]:\\[^\s]+", "<path>", value)
        value = re.sub(r"(?<![\w:/])/(?:[^\s]+)", "<path>", value)
        observed.append(value[:240])
        if len(observed) == 12:
            break
    return " | ".join(observed)[:1800] or "(no accessible text)"


def _trigger_export_from_installed_ui(environment: dict[str, str], window_id: str) -> None:
    window = _find_accessible(WINDOW_TITLE, timeout=10)
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
    _find_accessible("コマンドパレット", root=window, timeout=10)
    subprocess.run(
        ["xdotool", "type", "--clearmodifiers", "--delay", "30", "export"],
        env=environment,
        check=True,
        timeout=5,
    )
    command = _find_operable_accessible(
        "独立Appを書き出す", root=window, timeout=10
    )
    _activate_accessible(command, "独立Appを書き出す", environment)
    _activate_named_control_by_keyboard(
        "Owner確認してManifest fileを生成", environment, window_id
    )


def _open_update_center_from_installed_ui(
    environment: dict[str, str], window_id: str
):
    window = _find_accessible(WINDOW_TITLE, timeout=10)
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
    _find_accessible("コマンドパレット", root=window, timeout=10)
    subprocess.run(
        ["xdotool", "type", "--clearmodifiers", "--delay", "30", "update"],
        env=environment,
        check=True,
        timeout=5,
    )
    command = _find_operable_accessible("更新確認", root=window, timeout=10)
    _activate_accessible(command, "更新確認", environment)
    _find_accessible("更新センター", root=window, timeout=10)
    return window


def _open_mcp_center_from_installed_ui(
    environment: dict[str, str], window_id: str
):
    window = _find_accessible(WINDOW_TITLE, timeout=10)
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
    _find_accessible("コマンドパレット", root=window, timeout=10)
    subprocess.run(
        ["xdotool", "type", "--clearmodifiers", "--delay", "30", "mcp"],
        env=environment,
        check=True,
        timeout=5,
    )
    # Navigation also exposes an "MCP接続" item; target the palette row by its
    # unique subtitle so this activates the command rather than the nav item.
    _find_operable_accessible(
        "MCP接続一覧とOwner確認付き切断を開く", root=window, timeout=10
    )
    _activate_named_control_by_keyboard(
        "MCP接続一覧とOwner確認付き切断を開く",
        environment,
        window_id,
        context_marker="コマンドパレット",
    )
    # Dismissing the command-palette dialog can invalidate its old AT-SPI
    # subtree. Resolve the destination from the desktop, then reacquire the
    # product window before returning it to callers that inspect its controls.
    _find_accessible("MCP接続センター", timeout=10)
    window = _find_accessible(WINDOW_TITLE, timeout=10)
    _find_accessible("MCP接続センター", root=window, timeout=10)
    return window


def _set_public_text_by_keyboard(
    root,
    label: str,
    value: str,
    environment: dict[str, str],
    window_id: str,
) -> None:
    if (
        not value
        or len(value) > 256
        or any(ord(character) < 0x20 for character in value)
    ):
        raise RuntimeError("public GTK field value is empty or outside its bounded format")
    node = _find_accessible(label, root=root, timeout=10)
    editable_node = None
    for _ in range(6):
        try:
            node.queryEditableText()
            editable_node = node
            break
        except Exception:
            node = _accessible_parent(node)
            if node is None:
                break
    if editable_node is None:
        raise RuntimeError("GTK public text field is not keyboard-accessible")

    subprocess.run(
        ["xdotool", "windowactivate", "--sync", window_id],
        env=environment,
        check=True,
        timeout=5,
    )
    focused = None
    for index in range(97):
        focused = _focused_named_accessible(label, root=root)
        if focused is not None:
            break
        if index == 96:
            break
        subprocess.run(
            ["xdotool", "key", "--clearmodifiers", "Tab"],
            env=environment,
            check=True,
            timeout=5,
        )
        time.sleep(0.05)
    if focused is None:
        raise RuntimeError("GTK public text field did not receive keyboard focus")
    subprocess.run(
        ["xdotool", "key", "--clearmodifiers", "ctrl+a"],
        env=environment,
        check=True,
        timeout=5,
    )
    subprocess.run(
        ["xdotool", "type", "--clearmodifiers", "--delay", "20", value],
        env=environment,
        check=True,
        timeout=5,
    )
    deadline = time.monotonic() + 2
    while time.monotonic() < deadline:
        if _accessible_text(editable_node) == value:
            return
        time.sleep(0.05)
    raise RuntimeError("GTK public text field did not reflect the typed identifier")


def _enter_native_credential_secret(dialog, secret: str) -> None:
    editable = []
    for node, _ in _accessible_descendants(dialog):
        try:
            role = node.getRoleName().lower()
            node.queryEditableText()
            if "password" in role or "entry" in role or "text" in role:
                editable.append(node)
        except Exception:
            continue
    if len(editable) != 1:
        raise RuntimeError("native GTK secret entry is not uniquely accessible")
    try:
        if editable[0].queryEditableText().setTextContents(secret) is False:
            raise RuntimeError("native GTK secret entry rejected input")
    except Exception:
        raise RuntimeError("native GTK secret entry could not accept test input") from None


def _enter_credential_and_answer_owner(
    environment: dict[str, str],
    secret: str,
    decision: str,
    *,
    target_server_id: str,
    operation: str,
) -> None:
    dialog = _find_accessible("D4 Pocket 資格情報入力", timeout=10)
    _enter_native_credential_secret(dialog, secret)
    accept = _find_operable_accessible("入力して続行", root=dialog, timeout=10)
    _activate_accessible(accept, "入力して続行", environment)
    if operation == "登録":
        markers = (
            "Linux Secret Service default collectionへ登録します。",
            f"接続対象: {target_server_id}",
            "公開要求hash:",
        )
    else:
        markers = ("失効", target_server_id)
    _answer_owner_dialog(environment, decision, expected_summary_markers=markers)
    _wait_for_owner_dialog_closed(environment)


def _secret_service_item_labels() -> list[str]:
    try:
        import dbus
    except ImportError as error:
        raise RuntimeError(
            "Secret Service metadata inspection requires python3-dbus"
        ) from error

    try:
        bus = dbus.SessionBus()
        service_object = bus.get_object(
            "org.freedesktop.secrets", "/org/freedesktop/secrets"
        )
        service = dbus.Interface(
            service_object, "org.freedesktop.Secret.Service"
        )
        attributes = dbus.Dictionary(
            {"application": "d4-pocket"}, signature="ss"
        )
        unlocked, locked = service.SearchItems(attributes)
        labels: list[str] = []
        seen_paths: set[str] = set()
        for object_path in (*unlocked, *locked):
            path = str(object_path)
            if path in seen_paths:
                continue
            seen_paths.add(path)
            item_object = bus.get_object("org.freedesktop.secrets", path)
            properties = dbus.Interface(
                item_object, "org.freedesktop.DBus.Properties"
            )
            label = properties.Get("org.freedesktop.Secret.Item", "Label")
            if (
                not isinstance(label, str)
                or not label
                or len(label) > 256
                or any(ord(character) < 0x20 for character in label)
            ):
                raise RuntimeError("Secret Service item metadata is malformed")
            labels.append(label)
        return labels
    except dbus.exceptions.DBusException:
        raise RuntimeError("Secret Service metadata search failed") from None


def _start_isolated_secret_service(
    environment: dict[str, str], control_directory: Path, log_path: Path
) -> subprocess.Popen[bytes]:
    if control_directory.exists() or control_directory.is_symlink():
        raise RuntimeError("run固有Secret Service control directoryは未使用でなければなりません")
    control_directory.mkdir(mode=0o700)
    if control_directory.stat().st_mode & 0o077:
        raise RuntimeError("Secret Service control directoryがprivateではありません")
    process: subprocess.Popen[bytes] | None = None
    try:
        with log_path.open("xb") as log:
            process = subprocess.Popen(
                [
                    "gnome-keyring-daemon",
                    "--foreground",
                    "--unlock",
                    "--components=secrets",
                    "--control-directory",
                    str(control_directory),
                ],
                env=environment,
                stdin=subprocess.PIPE,
                stdout=log,
                stderr=subprocess.STDOUT,
                start_new_session=True,
            )
        if process.stdin is None:
            raise RuntimeError("Secret Service password stdin is unavailable")
        process.stdin.write(b"\n")
        process.stdin.close()
        environment["GNOME_KEYRING_CONTROL"] = str(control_directory)
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline:
            if process.poll() is not None:
                raise RuntimeError("isolated Secret Service exited during startup")
            owner = subprocess.run(
                [
                    "gdbus",
                    "call",
                    "--session",
                    "--dest",
                    "org.freedesktop.DBus",
                    "--object-path",
                    "/org/freedesktop/DBus",
                    "--method",
                    "org.freedesktop.DBus.NameHasOwner",
                    "org.freedesktop.secrets",
                ],
                env=environment,
                capture_output=True,
                text=True,
                check=False,
                timeout=5,
            )
            if owner.returncode == 0 and "(true,)" in owner.stdout:
                return process
            time.sleep(0.2)
        raise RuntimeError("isolated user-session Secret Service did not become available")
    except Exception:
        if process is not None and process.poll() is None:
            _terminate_group(process)
        raise


def _verify_credential_audit(store: Path, secret: str) -> dict[str, int]:
    audit_path = store / "audit.jsonl"
    anchor_path = store / "audit_anchor.json"
    key_path = store / "audit_anchor.key"
    audit_bytes = audit_path.read_bytes()
    if secret.encode("utf-8") in audit_bytes:
        raise RuntimeError("Credential secret leaked into durable Audit")
    events = [json.loads(line) for line in audit_bytes.decode("utf-8").splitlines() if line]
    if not events:
        raise RuntimeError("Credential acceptance produced no durable Audit")
    previous_hash: str | None = None
    for index, event in enumerate(events, start=1):
        if (
            event.get("event_id") != f"broker-audit-{index}"
            or event.get("previous_event_hash") != previous_hash
        ):
            raise RuntimeError("Credential Audit event sequence is invalid")
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
        previous_hash = "sha256:" + hashlib.sha256(hash_input.encode("utf-8")).hexdigest()
        if not hmac.compare_digest(event.get("event_hash", ""), previous_hash):
            raise RuntimeError("Credential Audit event hash is invalid")
    anchor = json.loads(anchor_path.read_text(encoding="utf-8"))
    key = bytes.fromhex(key_path.read_text(encoding="ascii").strip())
    anchor_input = (
        f"version=1|event_count={len(events)}|head_event_hash={previous_hash or ''}"
    ).encode("utf-8")
    expected = "sha256:" + hmac.new(key, anchor_input, hashlib.sha256).hexdigest()
    if (
        len(key) != 32
        or anchor.get("version") != 1
        or anchor.get("event_count") != len(events)
        or anchor.get("head_event_hash") != previous_hash
        or not hmac.compare_digest(anchor.get("anchor_hmac", ""), expected)
    ):
        raise RuntimeError("Credential Audit anchor／HMAC is invalid")
    operations = [event.get("operation", "") for event in events]
    if not any("資格情報登録" in operation for operation in operations):
        raise RuntimeError("Credential registration Audit is missing")
    if not any("資格情報失効" in operation for operation in operations):
        raise RuntimeError("Credential revocation Audit is missing")
    for operation_name in ("資格情報登録", "資格情報失効"):
        decisions = {
            event.get("decision")
            for event in events
            if event.get("operation") == operation_name
        }
        if not {"rejected", "accepted"}.issubset(decisions):
            raise RuntimeError(f"{operation_name} Audit lacks separate reject and accept decisions")
    return {
        "events": len(events),
        "registration_events": sum(operation == "資格情報登録" for operation in operations),
        "revocation_events": sum(operation == "資格情報失効" for operation in operations),
    }


_CREDENTIAL_SCAN_CHUNK_BYTES = 1024 * 1024
_CREDENTIAL_SCAN_MAX_BYTES = 512 * 1024 * 1024


def _assert_secret_absent_from_files(
    root: Path, secret: str, *, excluded_roots: tuple[Path, ...] = ()
) -> None:
    marker = secret.encode("utf-8")
    excluded = tuple(path.resolve(strict=True) for path in excluded_roots)
    remaining = _CREDENTIAL_SCAN_MAX_BYTES
    for path in root.rglob("*"):
        if any(path.is_relative_to(excluded_root) for excluded_root in excluded):
            continue
        try:
            metadata = path.lstat()
            if not stat.S_ISREG(metadata.st_mode):
                continue
            if metadata.st_size > remaining:
                raise RuntimeError("Credential acceptance scan exceeds its bounded byte budget")
            with path.open("rb") as stream:
                overlap = b""
                while chunk := stream.read(min(_CREDENTIAL_SCAN_CHUNK_BYTES, remaining + 1)):
                    remaining -= len(chunk)
                    if remaining < 0:
                        raise RuntimeError(
                            "Credential acceptance scan exceeds its bounded byte budget"
                        )
                    content = overlap + chunk
                    if marker in content:
                        raise RuntimeError(
                            "Credential secret leaked into an acceptance file or log"
                        )
                    overlap = content[-(len(marker) - 1) :] if len(marker) > 1 else b""
        except OSError:
            raise RuntimeError("Credential acceptance file scan could not read a test file") from None


def _accessible_snapshot(root) -> str:
    values: list[str] = []
    for node, name in _accessible_nodes(root):
        if name:
            values.append(name)
        text = _accessible_text(node)
        if text and text != name:
            values.append(text)
    return "\n".join(values)


def _run_installed_credential_vault_acceptance(
    environment: dict[str, str],
    window_id: str,
    *,
    target_server_id: str,
    audit_store: Path,
    acceptance_root: Path,
    restart_application: Callable[[str], str],
) -> dict[str, object]:
    window = _open_mcp_center_from_installed_ui(environment, window_id)
    secret = secrets.token_urlsafe(36)
    _set_public_text_by_keyboard(
        window, "サーバー識別子", target_server_id, environment, window_id
    )
    _activate_named_control_by_keyboard("native入力で資格情報を登録", environment, window_id)
    _enter_credential_and_answer_owner(
        environment,
        secret,
        "拒否",
        target_server_id=target_server_id,
        operation="登録",
    )
    _activate_named_control_by_keyboard(
        "対象ServerのCredential metadata一覧を取得", environment, window_id
    )
    _find_accessible(
        "このServer向けに利用可能なCredential metadataはありません。値は読み出していません。",
        root=window,
        timeout=10,
    )
    if _secret_service_item_labels():
        raise RuntimeError("Credential Owner拒否後にSecret Service itemが作成されました")

    _activate_named_control_by_keyboard("native入力で資格情報を登録", environment, window_id)
    _enter_credential_and_answer_owner(
        environment,
        secret,
        "許可",
        target_server_id=target_server_id,
        operation="登録",
    )
    _find_accessible("Linux Secret Service登録後のmetadataを取得しました。秘密値は取得していません。", root=window, timeout=15)
    _find_accessible("api_key ・ 有効", root=window, timeout=10)
    names = _accessible_snapshot(window)
    credential_ids = set(re.findall(r"\b[0-9a-f]{32}\b", names))
    if len(credential_ids) != 1:
        raise RuntimeError("installed UI did not expose exactly one public Credential ID")
    credential_id = next(iter(credential_ids))
    labels = _secret_service_item_labels()
    if sorted(labels) != ["D4 Pocket credential ciphertext", "D4 Pocket credential key"]:
        raise RuntimeError("Secret Service did not contain exactly the separate key and ciphertext items")
    if secret in names:
        raise RuntimeError("Credential secret leaked into the installed UI accessibility tree")

    window_id = restart_application(window_id)
    if _secret_service_item_labels() != labels:
        raise RuntimeError("Broker restart changed the run-owned Secret Service items")
    window = _open_mcp_center_from_installed_ui(environment, window_id)
    _set_public_text_by_keyboard(
        window, "サーバー識別子", target_server_id, environment, window_id
    )
    _activate_named_control_by_keyboard(
        "対象ServerのCredential metadata一覧を取得", environment, window_id
    )
    _find_accessible(
        "1件の対象Credential metadataを取得しました。秘密値は取得していません。",
        root=window,
        timeout=15,
    )
    restarted_names = _accessible_snapshot(window)
    if credential_id not in restarted_names or "api_key ・ 有効" not in restarted_names:
        raise RuntimeError("Broker restart後に同じ有効Credential metadataを読戻せません")
    if secret in restarted_names:
        raise RuntimeError("Broker restart後のUIへCredential secretが露出しました")

    _activate_named_control_by_keyboard("資格情報を失効", environment, window_id)
    _answer_owner_dialog(
        environment,
        "拒否",
        expected_summary_markers=(credential_id, target_server_id, "失効"),
    )
    _wait_for_owner_dialog_closed(environment)
    _find_accessible("api_key ・ 有効", root=window, timeout=10)
    if _secret_service_item_labels() != labels:
        raise RuntimeError("Credential revocation rejection changed Secret Service items")

    _activate_named_control_by_keyboard("資格情報を失効", environment, window_id)
    _answer_owner_dialog(
        environment,
        "許可",
        expected_summary_markers=(credential_id, target_server_id, "失効"),
    )
    _wait_for_owner_dialog_closed(environment)
    _find_accessible("Credentialを論理失効しました。以後の使用を拒否し、Secret Service itemは保持します。", root=window, timeout=15)
    _find_accessible("api_key ・ 失効", root=window, timeout=10)
    if _secret_service_item_labels() != labels:
        raise RuntimeError("Logical revocation unexpectedly removed Secret Service items")

    audit_evidence = _verify_credential_audit(audit_store, secret)
    _assert_secret_absent_from_files(
        acceptance_root,
        secret,
        excluded_roots=(acceptance_root / "home/.local/share/apps",),
    )
    return {
        "target_server_id": target_server_id,
        "credential_id": credential_id,
        "owner_reject_then_allow": True,
        "secret_service_items_after_logical_revocation": labels,
        "metadata_after_registration": "active",
        "broker_restarted_and_metadata_reloaded": True,
        "metadata_after_revocation": "revoked",
        "secret_in_flutter_a11y_audit_logs_or_files": False,
        "audit": audit_evidence,
    }


def _restart_installed_product(
    environment: dict[str, str],
    window_id: str,
    launcher: Path,
    desktop_entry: Path,
    stage: Path,
    log_path: Path,
    product_paths: set[Path],
    preexisting: set[int],
    current_process: subprocess.Popen[bytes],
) -> tuple[subprocess.Popen[bytes], str]:
    if not _desktop_entry_targets_fixed_launcher(desktop_entry, launcher):
        raise RuntimeError("Broker restart前に固定XDG desktop entryがinstalled launcherから変化しました")
    _request_x11_window_manager_close(window_id, environment)
    deadline = time.monotonic() + 60
    while time.monotonic() < deadline:
        if not (_processes_using(product_paths) - preexisting):
            break
        time.sleep(0.2)
    if _processes_using(product_paths) - preexisting:
        raise RuntimeError("Broker restart前の正常終了後にowned installed processが残留しています")
    if current_process.poll() is None:
        current_process.wait(timeout=10)
    if current_process.returncode not in (0, None):
        raise RuntimeError("Broker restart前のinstalled launcher終了codeが不正です")

    with log_path.open("xb") as log:
        restarted_process = subprocess.Popen(
            ["gio", "launch", str(desktop_entry)],
            cwd=stage,
            env=environment,
            stdin=subprocess.DEVNULL,
            stdout=log,
            stderr=subprocess.STDOUT,
            start_new_session=True,
        )
    deadline = time.monotonic() + 90
    while time.monotonic() < deadline:
        restarted_window = _visible_product_window(environment)
        running_products = _processes_using(product_paths) - preexisting
        if restarted_window is not None and running_products:
            return restarted_process, restarted_window
        if restarted_process.poll() is not None and restarted_process.returncode != 0:
            raise RuntimeError("固定desktop entryからのBroker restartが失敗しました")
        time.sleep(0.25)
    raise RuntimeError("固定desktop entryからBroker restart後のinstalled UIを確認できません")


def _reenter_update_center_for_fresh_broker_state(
    environment: dict[str, str], window_id: str
):
    window = _find_accessible(WINDOW_TITLE, timeout=10)
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
    _find_accessible("コマンドパレット", root=window, timeout=10)
    subprocess.run(
        ["xdotool", "type", "--clearmodifiers", "--delay", "30", "runtime"],
        env=environment,
        check=True,
        timeout=5,
    )
    runtime_command = _find_operable_accessible(
        "Runtimeを開く", root=window, timeout=10
    )
    _activate_accessible(runtime_command, "Runtimeを開く", environment)
    time.sleep(0.25)
    return _open_update_center_from_installed_ui(environment, window_id)


def _trigger_rollback_from_installed_ui(
    environment: dict[str, str], window_id: str
) -> None:
    window = _open_update_center_from_installed_ui(environment, window_id)
    _find_accessible("直前版へ戻す", root=window, timeout=10)
    _activate_named_control_by_keyboard("直前版へ戻す", environment, window_id)


def _trigger_update_action_from_installed_ui(
    environment: dict[str, str],
    window_id: str,
    action: str,
    *,
    row_marker: str | None = None,
):
    window = _open_update_center_from_installed_ui(environment, window_id)
    # Flutter may expose a TextButton label without an AT-SPI action/bounds.
    # Bind repeated action labels to the candidate row before keyboard activation.
    _find_accessible(action, root=window, timeout=10)
    if row_marker is not None:
        _find_accessible(row_marker, root=window, timeout=10)
    _activate_named_control_by_keyboard(
        action, environment, window_id, context_marker=row_marker
    )
    return window


def _trigger_catalog_fetch_from_installed_ui(
    environment: dict[str, str], window_id: str
) -> dict[str, str]:
    window = _open_update_center_from_installed_ui(environment, window_id)
    command = _find_operable_accessible("配布元から候補取得", root=window, timeout=10)
    _activate_accessible(command, "配布元から候補取得", environment)
    _wait_for_catalog_fetch_status(window)
    candidate = _find_accessible(
        "0.1.2 (linux-installed-catalog-display)", root=window, timeout=10
    )
    _find_accessible(
        "現在のBroker trustで検証済み", root=window, timeout=10
    )
    _find_accessible("配布元=raw.githubusercontent.com", root=window, timeout=10)
    _find_accessible("Brokerが署名検証済み候補を取得しました（1件）。", root=window, timeout=10)
    if not candidate.name:
        raise RuntimeError("Update Centerに候補表示がありません")
    return {
        "candidate": candidate.name,
        "signature": "verified_by_current_broker_trust",
        "source": "raw.githubusercontent.com",
        "package_downloaded": "download-and-apply-follows-in-this-acceptance",
    }


def _trigger_uninstall_from_installed_ui(
    environment: dict[str, str], window_id: str
) -> None:
    window = _open_update_center_from_installed_ui(environment, window_id)
    _find_accessible(UNINSTALL_BUTTON, root=window, timeout=10)
    _activate_named_control_by_keyboard(UNINSTALL_BUTTON, environment, window_id)


def _activate_named_control_by_keyboard(
    name: str,
    environment: dict[str, str],
    window_id: str,
    *,
    max_tabs: int = 96,
    context_marker: str | None = None,
) -> None:
    """可視かつ正確なAccessible名にfocusがある場合だけEnterを送る。"""
    subprocess.run(
        ["xdotool", "windowactivate", "--sync", window_id],
        env=environment,
        check=True,
        timeout=5,
    )
    window = _find_accessible(WINDOW_TITLE, timeout=10)
    context_node = (
        _find_accessible(context_marker, root=window, timeout=10)
        if context_marker is not None
        else None
    )
    for index in range(max_tabs + 1):
        if (
            _focused_named_accessible(
                name, root=window, context_node=context_node
            )
            is not None
        ):
            subprocess.run(
                ["xdotool", "key", "--clearmodifiers", "Return"],
                env=environment,
                check=True,
                timeout=5,
            )
            return
        if index == max_tabs:
            break
        subprocess.run(
            ["xdotool", "key", "--clearmodifiers", "Tab"],
            env=environment,
            check=True,
            timeout=5,
        )
        time.sleep(0.05)
    target = _find_accessible(name, timeout=2)
    raise RuntimeError(
        f"GTK keyboard traversalで可視focusを確認できません: {name}; "
        f"{_accessibility_debug(target)}"
    )


def _focused_named_accessible(name: str, *, root=None, context_node=None):
    try:
        import pyatspi
    except ImportError:
        return None
    search_root = root if root is not None else pyatspi.Registry.getDesktop(0)
    for node, accessible_name in _accessible_nodes(search_root):
        if name not in accessible_name.strip():
            continue
        if context_node is not None and not _focused_node_matches_context(
            node, name, context_node
        ):
            continue
        try:
            state = node.getState()
            if state.contains(pyatspi.STATE_FOCUSED) and state.contains(
                pyatspi.STATE_SHOWING
            ):
                return node
        except Exception:
            continue
    return None


def _focused_node_matches_context(node, name: str, context_node) -> bool:
    """候補とfocus controlを含む最小rowだけに同名actionが一つある場合に一致。"""
    scope = context_node
    for _ in range(32):
        if _accessible_contains(scope, node):
            matches = [
                _accessible_identity(candidate)
                for candidate, accessible_name in _accessible_nodes(scope)
                if accessible_name.strip() == name
            ]
            if len(set(matches)) == 1:
                return True
        scope = _accessible_parent(scope)
        if scope is None:
            break
    return False


def _accessible_contains(root, target) -> bool:
    current = target
    for _ in range(32):
        if _accessible_identity(current) == _accessible_identity(root):
            return True
        current = _accessible_parent(current)
        if current is None:
            break
    return False


def _accessible_identity(node):
    for attribute in ("path", "object_path", "objectPath"):
        try:
            value = getattr(node, attribute)
        except Exception:
            continue
        if value is not None:
            return (attribute, str(value))
    for method_name in ("getPath", "get_path"):
        try:
            method = getattr(node, method_name)
            value = method() if callable(method) else None
        except Exception:
            continue
        if value is not None:
            return ("path", str(value))
    try:
        return ("proxy", hash(node))
    except Exception:
        return ("object", id(node))


def _answer_owner_dialog(
    environment: dict[str, str],
    decision: str,
    *,
    expected_summary_markers: tuple[str, ...],
) -> None:
    if decision not in {"許可", "拒否"}:
        raise ValueError("Owner確認の回答値が不正")
    _wait_for_owner_dialog_window(environment)
    dialog = _find_accessible(OWNER_DIALOG_TITLE, timeout=10)
    descendants = _accessible_descendants(dialog)
    names = []
    for node, accessible_name in descendants:
        names.append(accessible_name)
        text = _accessible_text(node)
        if text and text != accessible_name:
            names.append(text)
    normalized_summary = " ".join(" ".join(names).split())
    missing_markers = [
        marker
        for marker in expected_summary_markers
        if " ".join(marker.split()) not in normalized_summary
    ]
    if missing_markers:
        raise RuntimeError(
            "Owner確認summaryに操作固有の必須情報がありません: "
            + "、".join(missing_markers)
            + "; bounded accessible text="
            + _bounded_owner_text_diagnostics(names)
        )
    if not any("要求hash:" in value or "payload hash:" in value for value in names):
        raise RuntimeError("Owner確認summaryに要求hashがありません")
    button = _find_operable_accessible(decision, root=dialog, timeout=10)
    _activate_accessible(button, decision, environment)


def _wait_for_owner_dialog_closed(
    environment: dict[str, str], *, timeout: float = 15.0
) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        result = subprocess.run(
            [
                "xdotool",
                "search",
                "--onlyvisible",
                "--name",
                f"^{re.escape(OWNER_DIALOG_TITLE)}$",
            ],
            env=environment,
            capture_output=True,
            check=False,
            timeout=5,
        )
        if result.returncode != 0:
            return
        time.sleep(0.1)
    raise RuntimeError("native Owner確認dialogが閉じませんでした")


def _wait_for_audit_event(
    audit_path: Path,
    operation: str,
    decision: str,
    *,
    evidence_source: str | None = None,
    timeout: float = 90.0,
) -> dict[str, object]:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        try:
            events = [
                json.loads(line)
                for line in audit_path.read_text(encoding="utf-8").splitlines()
                if line
            ]
        except (OSError, UnicodeError, json.JSONDecodeError):
            events = []
        for event in reversed(events):
            if (
                event.get("operation") == operation
                and event.get("decision") == decision
                and (
                    evidence_source is None
                    or event.get("evidence_source") == evidence_source
                )
            ):
                return event
        time.sleep(0.2)
    raise RuntimeError(
        f"installed Auditにterminal eventがありません: {operation}/{decision}"
    )


def _installed_update_download_stage_activate(
    environment: dict[str, str],
    window_id: str,
    *,
    product_root: Path,
    record_path: Path,
    desktop_entry: Path,
    audit_path: Path,
    product_paths: set[Path],
) -> dict[str, object]:
    fixture_package = (
        ROOT
        / "tooling/tests/fixtures/linux-update-catalog"
        / f"{UPDATE_ID}.pkg"
    ).resolve(strict=True)
    package_bytes = fixture_package.read_bytes()
    package_hash = hashlib.sha256(package_bytes).hexdigest()
    store_root = audit_path.parent
    package_store = store_root / "update_packages"
    downloaded_package = package_store / f"{package_hash}.pkg"
    record_before = record_path.read_bytes()
    entry_before = desktop_entry.read_bytes()
    current = json.loads(record_before)
    if current.get("product_version") != "0.1.1":
        raise RuntimeError("remote update開始時のactive versionが0.1.1ではありません")

    download_markers = (
        "Linux製品の署名検査済み更新packageを取得しますか？",
        f"更新ID: {UPDATE_ID}",
        f"提供版: {UPDATE_VERSION}",
        "取得host: raw.githubusercontent.com",
        package_hash,
        str(len(package_bytes)),
    )
    candidate_row = f"{UPDATE_VERSION} ({UPDATE_ID})"
    _trigger_update_action_from_installed_ui(
        environment, window_id, "download", row_marker=candidate_row
    )
    _answer_owner_dialog(
        environment, "拒否", expected_summary_markers=download_markers
    )
    _wait_for_owner_dialog_closed(environment)
    if downloaded_package.exists() or (
        package_store.exists() and list(package_store.glob("*.part"))
    ):
        raise RuntimeError("download Owner拒否後にpackageまたはpartial fileが作成されました")
    if record_path.read_bytes() != record_before or desktop_entry.read_bytes() != entry_before:
        raise RuntimeError("download Owner拒否がinstalled product状態を変更しました")

    _trigger_update_action_from_installed_ui(
        environment, window_id, "download", row_marker=candidate_row
    )
    _answer_owner_dialog(
        environment, "許可", expected_summary_markers=download_markers
    )
    _find_accessible("download jobを開始しました", timeout=15)
    _wait_for_audit_event(
        audit_path,
        "更新download要求",
        "completed",
        evidence_source="LIVE_RUNTIME",
    )
    package_metadata = downloaded_package.lstat()
    if (
        not stat.S_ISREG(package_metadata.st_mode)
        or downloaded_package.is_symlink()
        or package_metadata.st_nlink != 1
        or package_metadata.st_mode & 0o022
        or downloaded_package.read_bytes() != package_bytes
    ):
        raise RuntimeError("Broker固定store上のremote download byte/hash/identityが一致しません")

    window = _reenter_update_center_for_fresh_broker_state(environment, window_id)
    _find_accessible("download済み（installは別途保留）", root=window, timeout=15)
    stage_directory = product_root / "versions" / f"{UPDATE_VERSION}-{package_hash}"
    stage_markers = (
        "Linux製品の署名検査済みpackageを固定導入先へ展開しますか？",
        f"更新ID: {UPDATE_ID}",
        f"提供版: {UPDATE_VERSION}",
        f"package SHA-256: {package_hash}",
        APP_ID,
        AUDIT_ID,
        str(stage_directory),
    )
    _trigger_update_action_from_installed_ui(
        environment,
        window_id,
        "未起動版へ展開",
        row_marker=candidate_row,
    )
    _answer_owner_dialog(environment, "拒否", expected_summary_markers=stage_markers)
    _wait_for_owner_dialog_closed(environment)
    if (
        stage_directory.exists()
        or record_path.read_bytes() != record_before
        or desktop_entry.read_bytes() != entry_before
        or not downloaded_package.is_file()
    ):
        raise RuntimeError("stage Owner拒否がpackage／installed product状態を変更しました")

    _trigger_update_action_from_installed_ui(
        environment,
        window_id,
        "未起動版へ展開",
        row_marker=candidate_row,
    )
    _answer_owner_dialog(environment, "許可", expected_summary_markers=stage_markers)
    _find_accessible(
        "署名済みpackageを未起動versionとして固定導入先へ展開しました。",
        timeout=30,
    )
    package_document = json.loads(
        (ROOT / "tooling/tests/fixtures/linux-update-catalog/updates.json").read_text(
            encoding="utf-8"
        )
    )["候補"][0]
    signed_bytes = bytes.fromhex(package_document["署名対象"])
    signed_candidate = json.loads(signed_bytes)
    stage_manifest_length = int.from_bytes(package_bytes[8:12], "little")
    stage_manifest = json.loads(package_bytes[12 : 12 + stage_manifest_length])
    expected_stage_files = {entry["path"] for entry in stage_manifest["files"]}
    actual_stage_files = {
        path.relative_to(stage_directory).as_posix()
        for path in stage_directory.rglob("*")
        if path.is_file() and not path.is_symlink()
    }
    if (
        not stage_directory.is_dir()
        or stage_directory.is_symlink()
        or actual_stage_files != expected_stage_files
        or any(path.is_symlink() for path in stage_directory.rglob("*"))
        or signed_candidate["package_sha256"] != package_hash
        or signed_candidate["package_size_bytes"] != len(package_bytes)
    ):
        raise RuntimeError("Owner-confirmed stageが署名packageの固定inventoryと一致しません")
    product_paths.update(
        (stage_directory / relative).resolve()
        for relative in expected_stage_files
        if relative
        in {
            "gui_shell_desktop",
            "gui_shell_desktop_launcher",
            "libexec/gui_shell_linux_broker",
        }
    )
    _wait_for_audit_event(audit_path, "更新適用要求", "completed")

    stage_record_before_activation = record_path.read_bytes()
    active_pids = _processes_using(product_paths)
    active_groups = _owned_groups(active_pids)
    activation_markers = (
        "Linux製品の署名検査済みstageを有効版に切り替え",
        f"更新ID: {UPDATE_ID}",
        f"提供版: {UPDATE_VERSION}",
        f"package SHA-256: {package_hash}",
        APP_ID,
        AUDIT_ID,
        str(stage_directory),
        str(desktop_entry),
    )
    _trigger_update_action_from_installed_ui(
        environment,
        window_id,
        "更新を有効化",
        row_marker=candidate_row,
    )
    _answer_owner_dialog(
        environment, "拒否", expected_summary_markers=activation_markers
    )
    _wait_for_owner_dialog_closed(environment)
    if (
        record_path.read_bytes() != stage_record_before_activation
        or desktop_entry.read_bytes() != entry_before
        or not stage_directory.is_dir()
        or not downloaded_package.is_file()
        or _processes_using(product_paths) != active_pids
        or not all(_group_exists(group) for group in active_groups)
    ):
        raise RuntimeError("activation Owner拒否がstage／record／entry／processを変更しました")

    _trigger_update_action_from_installed_ui(
        environment,
        window_id,
        "更新を有効化",
        row_marker=candidate_row,
    )
    _answer_owner_dialog(
        environment, "許可", expected_summary_markers=activation_markers
    )
    _find_accessible(
        "有効版を切り替え、OS起動項目へ登録しました。",
        timeout=30,
    )
    activated_record = json.loads(record_path.read_text(encoding="utf-8"))
    if (
        activated_record.get("product_version") != UPDATE_VERSION
        or activated_record.get("package_sha256") != package_hash
        or not isinstance(activated_record.get("previous"), dict)
        or activated_record["previous"].get("product_version") != current["product_version"]
        or desktop_entry.read_bytes() != entry_before
        or not _desktop_entry_targets_fixed_launcher(
            desktop_entry, product_root / "gui_shell_desktop_launcher"
        )
        or not stage_directory.is_dir()
        or downloaded_package.exists()
        or _processes_using(product_paths) != active_pids
        or not all(_group_exists(group) for group in active_groups)
    ):
        raise RuntimeError("Owner-confirmed activation record／entry／process／cleanupが不正です")
    _wait_for_audit_event(audit_path, "更新有効版切替要求", "completed")
    return {
        "active_record": activated_record,
        "package_sha256": package_hash,
        "package_size_bytes": len(package_bytes),
        "staged_file_count": len(expected_stage_files),
        "download_rejected_then_owner_approved": True,
        "stage_rejected_then_owner_approved": True,
        "activation_rejected_then_owner_approved": True,
        "installed_process_preserved": True,
    }


def _wait_for_owner_dialog_window(
    environment: dict[str, str], *, timeout: float = OWNER_DIALOG_TIMEOUT
) -> str:
    deadline = time.monotonic() + timeout
    while True:
        result = subprocess.run(
            [
                "xdotool",
                "search",
                "--onlyvisible",
                "--name",
                f"^{re.escape(OWNER_DIALOG_TITLE)}$",
            ],
            env=environment,
            capture_output=True,
            check=False,
            timeout=5,
        )
        if result.returncode == 0:
            dialog_ids = result.stdout.decode("ascii", errors="ignore").splitlines()
            if len(dialog_ids) != 1:
                raise RuntimeError("native Owner確認dialogが一意ではありません")
            return dialog_ids[0]
        if time.monotonic() >= deadline:
            raise RuntimeError("GTK native Owner確認dialogが期限内に表示されません")
        time.sleep(0.1)


def _verify_live_audit(
    store: Path,
    *,
    require_uninstall: bool = False,
    require_rollback: bool = False,
    require_catalog: bool = False,
    require_update_lifecycle: bool = False,
) -> dict[str, int]:
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
    try:
        key = bytes.fromhex(key_path.read_text(encoding="ascii").strip())
    except (OSError, UnicodeError, ValueError) as error:
        raise RuntimeError("installed product Audit anchor keyが不正です") from error
    if len(key) != 32:
        raise RuntimeError("installed product Audit anchor key長が不正です")
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

    exports = [
        event
        for event in events
        if event.get("operation") == "GUI Shell書出し"
        and event.get("evidence_source") == "INTERNAL_STATE"
    ]
    accepted = sum(event.get("decision") == "accepted" for event in exports)
    rejected = sum(event.get("decision") == "rejected" for event in exports)
    if accepted < 1 or rejected < 1:
        raise RuntimeError("GTK Owner拒否／許可の両方がINTERNAL_STATE Auditにありません")
    uninstall = [
        event
        for event in events
        if event.get("operation") == "製品アンインストール要求"
    ]
    uninstall_accepted = sum(
        event.get("decision") == "accepted"
        and event.get("evidence_source") == "LIVE_RUNTIME"
        for event in uninstall
    )
    uninstall_rejected = sum(
        event.get("decision") == "rejected"
        and event.get("reason") == "desktop_native_owner_confirmation_required"
        and event.get("evidence_source") == "INTERNAL_STATE"
        for event in uninstall
    )
    uninstall_completed = sum(
        event.get("operation") == "製品アンインストール完了"
        and event.get("decision") == "completed"
        and event.get("evidence_source") == "LIVE_RUNTIME"
        for event in events
    )
    if require_uninstall and (
        uninstall_accepted < 1 or uninstall_rejected < 1 or uninstall_completed < 1
    ):
        raise RuntimeError(
            "Uninstall拒否(INTERNAL_STATE)／許可・完了(LIVE_RUNTIME) Auditが不足しています"
        )
    rollback = [
        event for event in events if event.get("operation") == "更新rollback要求"
    ]
    rollback_rejected = sum(
        event.get("decision") == "rejected"
        and event.get("reason") == "desktop_native_owner_confirmation_required"
        and event.get("evidence_source") == "INTERNAL_STATE"
        for event in rollback
    )
    rollback_queued = sum(
        event.get("decision") == "queued"
        and event.get("evidence_source") == "INTERNAL_STATE"
        for event in rollback
    )
    rollback_completed = sum(
        event.get("decision") == "completed"
        and event.get("evidence_source") == "INTERNAL_STATE"
        for event in rollback
    )
    if require_rollback and (
        rollback_rejected < 1 or rollback_queued < 1 or rollback_completed < 1
    ):
        raise RuntimeError(
            "Rollback拒否／fresh Owner承認後のintent・完了(INTERNAL_STATE) Auditが不足しています"
        )
    catalog_fetch = [
        event for event in events if event.get("operation") == "更新候補取得"
    ]
    catalog_accepted = sum(
        event.get("decision") == "accepted"
        and event.get("evidence_source") == "EXTERNAL_EVIDENCE"
        for event in catalog_fetch
    )
    if require_catalog and catalog_accepted < 1:
        raise RuntimeError(
            "installed UIからの更新候補取得がEXTERNAL_EVIDENCEとしてAuditにありません"
        )
    update_download = [
        event for event in events if event.get("operation") == "更新download要求"
    ]
    update_apply = [event for event in events if event.get("operation") == "更新適用要求"]
    update_activation = [
        event for event in events if event.get("operation") == "更新有効版切替要求"
    ]
    download_rejected = sum(
        event.get("decision") == "rejected"
        and event.get("reason") == "desktop_native_owner_confirmation_required"
        and event.get("evidence_source") == "INTERNAL_STATE"
        for event in update_download
    )
    download_completed = sum(
        event.get("decision") == "completed"
        and event.get("evidence_source") == "LIVE_RUNTIME"
        for event in update_download
    )
    apply_rejected = sum(
        event.get("decision") == "rejected"
        and event.get("reason") == "desktop_native_owner_confirmation_required"
        and event.get("evidence_source") == "INTERNAL_STATE"
        for event in update_apply
    )
    apply_completed = sum(
        event.get("decision") == "completed"
        and event.get("evidence_source") == "INTERNAL_STATE"
        for event in update_apply
    )
    activation_rejected = sum(
        event.get("decision") == "rejected"
        and event.get("reason") == "desktop_native_owner_confirmation_required"
        and event.get("evidence_source") == "INTERNAL_STATE"
        for event in update_activation
    )
    activation_completed = sum(
        event.get("decision") == "completed"
        and event.get("evidence_source") == "INTERNAL_STATE"
        for event in update_activation
    )
    if require_update_lifecycle and not all(
        (
            download_rejected >= 1,
            download_completed >= 1,
            apply_rejected >= 1,
            apply_completed >= 1,
            activation_rejected >= 1,
            activation_completed >= 1,
        )
    ):
        raise RuntimeError(
            "installed Update download／stage／activationのfresh Owner拒否・許可Auditが不足しています"
        )
    return {
        "events": len(events),
        "export_accepted": accepted,
        "export_rejected": rejected,
        "uninstall_accepted": uninstall_accepted,
        "uninstall_rejected": uninstall_rejected,
        "uninstall_completed": uninstall_completed,
        "rollback_rejected": rollback_rejected,
        "rollback_queued": rollback_queued,
        "rollback_completed": rollback_completed,
        "catalog_fetch_external_accepted": catalog_accepted,
        "update_download_owner_rejected": download_rejected,
        "update_download_live_completed": download_completed,
        "update_stage_owner_rejected": apply_rejected,
        "update_stage_completed": apply_completed,
        "update_activation_owner_rejected": activation_rejected,
        "update_activation_completed": activation_completed,
    }


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


def _verify_uninstalled_state(
    product_root: Path,
    desktop_entry: Path,
    audit_store: Path,
    preserved_user_data: Path,
) -> dict[str, str]:
    if product_root.exists() or product_root.is_symlink():
        raise RuntimeError("Uninstall後も固定製品rootが残っています")
    if desktop_entry.exists() or desktop_entry.is_symlink():
        raise RuntimeError("Uninstall後も製品desktop entryが残っています")
    if product_root == audit_store or product_root in audit_store.parents:
        raise RuntimeError("Audit Storeが削除対象の製品root内にあります")
    if not audit_store.is_dir() or audit_store.is_symlink():
        raise RuntimeError("Uninstall後にBroker Audit Storeが保持されていません")
    audit_path = audit_store / "audit.jsonl"
    anchor_path = audit_store / "audit_anchor.json"
    if not audit_path.is_file() or audit_path.is_symlink():
        raise RuntimeError("Uninstall後のfile-backed Auditがありません")
    if not anchor_path.is_file() or anchor_path.is_symlink():
        raise RuntimeError("Uninstall後のAudit anchorがありません")
    if (
        not preserved_user_data.is_file()
        or preserved_user_data.is_symlink()
        or preserved_user_data.read_bytes() != b"D4P uninstall scope sentinel\n"
    ):
        raise RuntimeError("Uninstall後に製品root外の利用者dataが保持されていません")
    return {
        "product_root": "removed",
        "desktop_entry": "removed",
        "audit_store": "retained_outside_product_root",
        "user_data": "retained_outside_product_root",
    }


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
    _stop_owned_groups(groups)
    if extra_process is not None:
        try:
            extra_process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            pass


def _stop_owned_groups(groups: set[int]) -> None:
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


def _group_exists(group: int) -> bool:
    try:
        os.killpg(group, 0)
        return True
    except ProcessLookupError:
        return False
    except PermissionError:
        return True


def _run_rust_test(
    name: str, environment: dict[str, str], private_key_path: Path
) -> None:
    test_environment = environment | {
        "D4P_LINUX_ACCEPTANCE_PRIVATE_KEY": str(private_key_path)
    }
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
        env=test_environment,
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


def run_installed_smoke(
    package: Path,
    temp_root: Path,
    acceptance_root: Path,
    private_key_path: Path,
    update_package: Path,
    acceptance_scope: str = "update_lifecycle",
) -> None:
    if not os.environ.get("DISPLAY") or not os.environ.get("DBUS_SESSION_BUS_ADDRESS"):
        raise RuntimeError("隔離X11 displayとD-Bus sessionが必要です")
    if temp_root.is_symlink() or not temp_root.is_dir():
        raise RuntimeError("run固有一時rootが通常directoryではありません")
    temp_root = temp_root.resolve(strict=True)
    package = _resolve_regular_package(package)
    update_package = _resolve_regular_package(update_package)
    if temp_root not in update_package.parents:
        raise RuntimeError("rollback fixture packageがrun固有一時root外です")
    if acceptance_scope not in {"update_lifecycle", "credential_vault"}:
        raise RuntimeError("installed acceptance scopeが未対応です")
    resolved_private_key = private_key_path.resolve(strict=True)
    if (
        resolved_private_key != private_key_path
        or private_key_path.is_symlink()
        or not private_key_path.is_file()
        or temp_root not in private_key_path.parents
        or private_key_path.stat().st_mode & 0o077
    ):
        raise RuntimeError("test署名鍵がacceptance root配下のprivate通常fileではありません")
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
    preserved_user_data = home / ".config/owner-data-sentinel/uninstall-preserved.marker"
    preserved_user_data.parent.mkdir(mode=0o700, exist_ok=False)
    preserved_user_data.write_bytes(b"D4P uninstall scope sentinel\n")

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
            "D4P_LINUX_ACCEPTANCE_UPDATE_PACKAGE": str(update_package),
            "D4P_LINUX_ACCEPTANCE_ROOT": str(acceptance_root),
            "CARGO_TARGET_DIR": str(temp_root / "linux-installed-cargo-target"),
        }
    )

    window_manager: subprocess.Popen[bytes] | None = None
    desktop_process: subprocess.Popen[bytes] | None = None
    wayland_compositor: subprocess.Popen[bytes] | None = None
    wayland_process: subprocess.Popen[bytes] | None = None
    uninstall_process: subprocess.Popen[bytes] | None = None
    secret_service_process: subprocess.Popen[bytes] | None = None
    uninstall_groups: set[int] = set()
    product_paths: set[Path] = set()
    preexisting: set[int] = set()
    app_log = acceptance_root / "installed-app.log"
    try:
        if acceptance_scope == "credential_vault":
            secret_service_process = _start_isolated_secret_service(
                environment,
                acceptance_root / "keyring-control",
                acceptance_root / "gnome-keyring.log",
            )
            _run_rust_test(RUST_SECRET_SERVICE_TEST, environment, private_key_path)
        else:
            _run_rust_test(RUST_CATALOG_RELAY_TIMEOUT_TEST, environment, private_key_path)
            _run_rust_test(RUST_HELPER_EXIT_CLASS_TEST, environment, private_key_path)
            _run_rust_test(RUST_CATALOG_TLS_PROVIDER_TEST, environment, private_key_path)
        _run_rust_test(RUST_STAGE_TEST, environment, private_key_path)
        if acceptance_scope == "update_lifecycle":
            _run_rust_test(RUST_ROLLBACK_SEED_TEST, environment, private_key_path)
        for test_only_name in (
            "D4P_LINUX_ACCEPTANCE_PACKAGE",
            "D4P_LINUX_ACCEPTANCE_UPDATE_PACKAGE",
            "D4P_LINUX_ACCEPTANCE_ROOT",
        ):
            environment.pop(test_only_name, None)
        product_root = xdg_data_home / "apps/d4-pocket" / APP_ID
        fixed_launcher = product_root / "gui_shell_desktop_launcher"
        record_path = product_root / "active_version.json"
        record = json.loads(record_path.read_text(encoding="utf-8"))
        stage = product_root / "versions" / f"{record['product_version']}-{record['package_sha256']}"
        previous_record = record.get("previous")
        if acceptance_scope == "update_lifecycle" and not isinstance(previous_record, dict):
            raise RuntimeError("Rollback受入れ前のactive recordに直前版がありません")
        previous_stage = (
            product_root / "versions" / (
                f"{previous_record['product_version']}-{previous_record['package_sha256']}"
            )
            if isinstance(previous_record, dict)
            else None
        )
        active_launcher = stage / "gui_shell_desktop_launcher"
        runner = stage / "gui_shell_desktop"
        helper = stage / "libexec/gui_shell_linux_broker"
        if not all(path.is_file() and not path.is_symlink() for path in (fixed_launcher, active_launcher, runner, helper)):
            raise RuntimeError("Brokerが有効化したinstalled bundleの固定fileが不足しています")
        applications = xdg_data_home / "applications"
        entries = list(applications.glob("*.desktop"))
        if len(entries) != 1 or not _desktop_entry_targets_fixed_launcher(entries[0], fixed_launcher):
            raise RuntimeError("登録desktop entryが固定installed launcherだけを指していません")
        previous_binaries = (
            {
                previous_stage / "gui_shell_desktop_launcher",
                previous_stage / "gui_shell_desktop",
                previous_stage / "libexec/gui_shell_linux_broker",
            }
            if previous_stage is not None
            else set()
        )
        if previous_binaries and not all(
            path.is_file() and not path.is_symlink() for path in previous_binaries
        ):
            raise RuntimeError("Rollback target stageの固定binaryが不足しています")
        product_paths = {
            fixed_launcher.resolve(),
            active_launcher.resolve(),
            runner.resolve(),
            helper.resolve(),
            *(path.resolve() for path in previous_binaries),
        }
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

            if acceptance_scope == "credential_vault":
                if _secret_service_item_labels():
                    raise RuntimeError("Credential acceptance開始前のrun固有Secret Serviceが空ではありません")

                def restart_application(old_window_id: str) -> str:
                    nonlocal desktop_process
                    desktop_process, restarted_window_id = _restart_installed_product(
                        environment,
                        old_window_id,
                        fixed_launcher,
                        entries[0],
                        stage,
                        acceptance_root / "installed-app-after-broker-restart.log",
                        product_paths,
                        preexisting,
                        desktop_process,
                    )
                    return restarted_window_id

                credential_evidence = _run_installed_credential_vault_acceptance(
                    environment,
                    window_id,
                    target_server_id="installed-secret-service-server",
                    audit_store=broker_state / "store",
                    acceptance_root=acceptance_root,
                    restart_application=restart_application,
                )
                current_window_id = _visible_product_window(environment)
                if current_window_id is None:
                    raise RuntimeError("Credential受入れ後にinstalled GTK windowが見つかりません")
                _request_x11_window_manager_close(current_window_id, environment)
                deadline = time.monotonic() + 60
                while time.monotonic() < deadline:
                    if not (_processes_using(product_paths) - preexisting):
                        break
                    time.sleep(0.2)
                if _processes_using(product_paths) - preexisting:
                    raise RuntimeError("Credential受入れ後の正常終了でowned processが残留しています")
                if desktop_process.poll() is None:
                    desktop_process.wait(timeout=10)
                if desktop_process.returncode not in (0, None):
                    raise RuntimeError("Credential受入れ後のdesktop launcher終了codeが不正です")
                print(
                    json.dumps(
                        {
                            "acceptance_scope": acceptance_scope,
                            "installed_entry": str(entries[0]),
                            "product_version": record["product_version"],
                            "package_sha256": record["package_sha256"],
                            "credential_vault": credential_evidence,
                            "installed_app_exit": "normal_window_close_and_owned_process_cleanup",
                        },
                        ensure_ascii=False,
                        separators=(",", ":"),
                    )
                )
                return

            _trigger_export_from_installed_ui(environment, window_id)
            _answer_owner_dialog(
                environment,
                "拒否",
                expected_summary_markers=("対象platform: Linux",),
            )
            _find_accessible("GUI Shell独立App書出しは受理されませんでした", timeout=20)
            if any(export_root.iterdir()):
                raise RuntimeError("Owner拒否後にManifestまたは一時fileが作成されました")

            before_catalog = _catalog_failure_diagnostics(
                helper, preexisting, broker_state / "store/audit.jsonl"
            )
            print(
                "catalog transport immediately before fetch="
                + json.dumps(before_catalog, ensure_ascii=False, separators=(",", ":"))
            )
            try:
                catalog_evidence = _trigger_catalog_fetch_from_installed_ui(
                    environment, window_id
                )
            except RuntimeError as error:
                diagnostics = _catalog_failure_diagnostics(
                    helper, preexisting, broker_state / "store/audit.jsonl"
                )
                diagnostics["helper_exit"] = _helper_exit_diagnostics(app_log)
                raise RuntimeError(
                    f"{error}; catalog transport diagnostics="
                    + json.dumps(diagnostics, ensure_ascii=False, separators=(",", ":"))
                ) from error

            update_evidence = _installed_update_download_stage_activate(
                environment,
                window_id,
                product_root=product_root,
                record_path=record_path,
                desktop_entry=entries[0],
                audit_path=broker_state / "store/audit.jsonl",
                product_paths=product_paths,
            )
            record = update_evidence.get("active_record")
            if not isinstance(record, dict):
                raise RuntimeError("installed Update受入れがactive recordを返しません")
            previous_record = record.get("previous")
            if not isinstance(previous_record, dict):
                raise RuntimeError("remote Update後のactive recordにprevious versionがありません")
            stage = product_root / "versions" / (
                f"{record['product_version']}-{record['package_sha256']}"
            )
            previous_stage = product_root / "versions" / (
                f"{previous_record['product_version']}-{previous_record['package_sha256']}"
            )

            rollback_entry_before = entries[0].read_bytes()
            rollback_record_before = record_path.read_bytes()
            rollback_pids = _processes_using(product_paths) - preexisting
            rollback_groups = _owned_groups(rollback_pids)
            if not rollback_groups:
                raise RuntimeError("Rollback開始時にinstalled productの所有process groupがありません")
            current_version = record["product_version"]
            target_version = previous_record["product_version"]
            if current_version != UPDATE_VERSION or target_version != "0.1.1":
                raise RuntimeError("Rollback acceptanceの現在版／対象版が期待値と一致しません")

            _trigger_rollback_from_installed_ui(environment, window_id)
            rollback_markers = (
                "Linux製品を直前の署名済み有効版へrollbackしますか？",
                f"現行版: {current_version}",
                f"復元版: {target_version}",
                APP_ID,
                AUDIT_ID,
            )
            _answer_owner_dialog(
                environment,
                "拒否",
                expected_summary_markers=rollback_markers,
            )
            if (
                record_path.read_bytes() != rollback_record_before
                or entries[0].read_bytes() != rollback_entry_before
                or not _desktop_entry_targets_fixed_launcher(entries[0], fixed_launcher)
                or not stage.is_dir()
                or not previous_stage.is_dir()
                or (_processes_using(product_paths) - preexisting) != rollback_pids
                or not all(_group_exists(group) for group in rollback_groups)
            ):
                raise RuntimeError("Rollback Owner拒否後にrecord／entry／processが保持されませんでした")

            _trigger_rollback_from_installed_ui(environment, window_id)
            _answer_owner_dialog(
                environment,
                "許可",
                expected_summary_markers=rollback_markers,
            )
            rollback_deadline = time.monotonic() + 20
            while time.monotonic() < rollback_deadline:
                if record_path.read_bytes() != rollback_record_before:
                    break
                time.sleep(0.1)
            record = json.loads(record_path.read_text(encoding="utf-8"))
            if (
                record.get("product_version") != target_version
                or not isinstance(record.get("previous"), dict)
                or record["previous"].get("product_version") != current_version
                or entries[0].read_bytes() != rollback_entry_before
                or not _desktop_entry_targets_fixed_launcher(entries[0], fixed_launcher)
                or not stage.is_dir()
                or not previous_stage.is_dir()
                or (_processes_using(product_paths) - preexisting) != rollback_pids
                or not all(_group_exists(group) for group in rollback_groups)
            ):
                raise RuntimeError("fresh Owner許可後のRollback状態遷移または隔離が不正です")
            stage = previous_stage
            _find_accessible(
                "有効版切替は次回の固定root起動時に反映されます",
                root=_find_accessible(WINDOW_TITLE, timeout=10),
                timeout=10,
            )

            _trigger_export_from_installed_ui(environment, window_id)
            _answer_owner_dialog(
                environment,
                "許可",
                expected_summary_markers=("対象platform: Linux",),
            )
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

        wayland_compositor, wayland_display = _start_wayland_compositor(
            runtime, environment, acceptance_root / "wayland-compositor.log"
        )
        wayland_environment = _wayland_launch_environment(environment, wayland_display)
        wayland_log = acceptance_root / "installed-wayland-app.log"
        with wayland_log.open("xb") as log:
            wayland_process = subprocess.Popen(
                ["gio", "launch", str(entries[0])],
                cwd=stage,
                env=wayland_environment,
                stdin=subprocess.DEVNULL,
                stdout=log,
                stderr=subprocess.STDOUT,
                start_new_session=True,
            )
            try:
                wayland_window = _find_accessible(WINDOW_TITLE, timeout=60)
            except RuntimeError as error:
                tail = _safe_log_tail(wayland_log, (acceptance_root, temp_root))
                raise RuntimeError(
                    f"installed Wayland GTK windowがAT-SPIへ現れませんでした\n{tail}"
                ) from error
            try:
                import pyatspi

                if not wayland_window.getState().contains(pyatspi.STATE_SHOWING):
                    raise RuntimeError("installed Wayland GTK top-levelがshowingではありません")
            except RuntimeError:
                raise
            except Exception as error:
                raise RuntimeError(
                    "installed Wayland GTK window visibilityを取得できません"
                ) from error
            running_wayland = _processes_using(product_paths) - preexisting
            if not running_wayland or wayland_process.poll() not in (None, 0):
                raise RuntimeError("Wayland GTK window観測時にinstalled processが稼働していません")
            _stop_owned_processes(running_wayland, wayland_process)
            if _processes_using(product_paths) - preexisting:
                raise RuntimeError("Wayland installed productの所有process cleanupが残留しました")
            if wayland_process.poll() is None:
                wayland_process.wait(timeout=10)
            if wayland_process.returncode not in (0, -signal.SIGTERM, -signal.SIGKILL, None):
                raise RuntimeError(
                    f"Wayland installed launcher終了codeが想定外です: {wayland_process.returncode}"
                )
        _terminate_group(wayland_compositor)
        wayland_compositor = None

        audit_evidence = _verify_live_audit(
            broker_state / "store",
            require_rollback=True,
            require_catalog=True,
            require_update_lifecycle=True,
        )
        _run_rust_test(RUST_AUDIT_TEST, environment, private_key_path)

        audit_store = broker_state / "store"
        active_record_before_uninstall = record_path.read_bytes()
        uninstall_log = acceptance_root / "installed-uninstall-app.log"
        with uninstall_log.open("xb") as log:
            uninstall_process = subprocess.Popen(
                ["gio", "launch", str(entries[0])],
                cwd=stage,
                env=environment,
                stdin=subprocess.DEVNULL,
                stdout=log,
                stderr=subprocess.STDOUT,
                start_new_session=True,
            )
            deadline = time.monotonic() + 90
            uninstall_window_id: str | None = None
            while time.monotonic() < deadline:
                uninstall_window_id = _visible_product_window(environment)
                uninstall_pids = _processes_using(product_paths) - preexisting
                if uninstall_window_id is not None and uninstall_pids:
                    uninstall_groups = _owned_groups(uninstall_pids)
                    break
                if uninstall_process.poll() is not None and uninstall_process.returncode != 0:
                    tail = _safe_log_tail(uninstall_log, (acceptance_root, temp_root))
                    raise RuntimeError(
                        f"Uninstall確認用installed UI起動失敗: {uninstall_process.returncode}\n{tail}"
                    )
                time.sleep(0.25)
            if uninstall_window_id is None or not uninstall_groups:
                tail = _safe_log_tail(uninstall_log, (acceptance_root, temp_root))
                raise RuntimeError(f"Uninstall確認用installed UI／process groupがありません\n{tail}")

            _trigger_uninstall_from_installed_ui(environment, uninstall_window_id)
            _answer_owner_dialog(
                environment,
                "拒否",
                expected_summary_markers=(
                    "D4 Pocketを終了後にこの利用者から削除しますか？",
                    "固定製品root:",
                    "固定XDG desktop entry:",
                    "XDG state内のRuntime設定、Credential、Audit Storeは保持します。",
                ),
            )
            if not _desktop_entry_targets_fixed_launcher(entries[0], fixed_launcher):
                raise RuntimeError("Uninstall Owner拒否後にdesktop entryが変化しました")
            if (
                not product_root.is_dir()
                or product_root.is_symlink()
                or not record_path.is_file()
                or record_path.is_symlink()
                or record_path.read_bytes() != active_record_before_uninstall
                or not all(_group_exists(group) for group in uninstall_groups)
            ):
                raise RuntimeError("Uninstall Owner拒否後にproduct root／active record／processが保持されませんでした")

            reject_window_deadline = time.monotonic() + 10
            uninstall_window_id = None
            while time.monotonic() < reject_window_deadline:
                uninstall_window_id = _visible_product_window(environment)
                if uninstall_window_id is not None:
                    break
                time.sleep(0.2)
            reject_window_reopened = uninstall_window_id is not None
            if uninstall_window_id is None:
                # Owner拒否後の非作用を確認済み。再試行用にこのtestが所有する
                # process groupだけを停止し、固定desktop entryから新しいUIを起動する。
                _stop_owned_groups(uninstall_groups)
                if _processes_using(product_paths) - preexisting:
                    raise RuntimeError("Owner拒否後のtest-owned process groupを停止できません")
                uninstall_groups.clear()
                if uninstall_process.poll() is None:
                    uninstall_process.wait(timeout=10)

                uninstall_retry_log = acceptance_root / "installed-uninstall-retry-app.log"
                with uninstall_retry_log.open("xb") as log:
                    uninstall_process = subprocess.Popen(
                        ["gio", "launch", str(entries[0])],
                        cwd=stage,
                        env=environment,
                        stdin=subprocess.DEVNULL,
                        stdout=log,
                        stderr=subprocess.STDOUT,
                        start_new_session=True,
                    )
                    deadline = time.monotonic() + 90
                    uninstall_window_id = None
                    uninstall_pids: set[int] = set()
                    while time.monotonic() < deadline:
                        uninstall_window_id = _visible_product_window(environment)
                        uninstall_pids = _processes_using(product_paths) - preexisting
                        if uninstall_window_id is not None and uninstall_pids:
                            uninstall_groups = _owned_groups(uninstall_pids)
                            break
                        if (
                            uninstall_process.poll() is not None
                            and uninstall_process.returncode != 0
                        ):
                            tail = _safe_log_tail(
                                uninstall_retry_log, (acceptance_root, temp_root)
                            )
                            raise RuntimeError(
                                "Owner拒否後のinstalled UI再起動に失敗: "
                                f"{uninstall_process.returncode}\n{tail}"
                            )
                        time.sleep(0.25)
                    if uninstall_window_id is None or not uninstall_groups:
                        tail = _safe_log_tail(
                            uninstall_retry_log, (acceptance_root, temp_root)
                        )
                        raise RuntimeError(
                            "Owner拒否後に固定desktop entryからinstalled UIを再起動できません\n"
                            f"{tail}"
                        )
            _trigger_uninstall_from_installed_ui(environment, uninstall_window_id)
            _answer_owner_dialog(
                environment,
                "許可",
                expected_summary_markers=(
                    "D4 Pocketを終了後にこの利用者から削除しますか？",
                    "固定製品root:",
                    "固定XDG desktop entry:",
                    "XDG state内のRuntime設定、Credential、Audit Storeは保持します。",
                ),
            )

            deadline = time.monotonic() + 90
            while time.monotonic() < deadline:
                if not any(_group_exists(group) for group in uninstall_groups):
                    break
                time.sleep(0.2)
            if any(_group_exists(group) for group in uninstall_groups):
                raise RuntimeError("Uninstall許可後もowned product process groupが残留しています")
            uninstall_groups.clear()
            if uninstall_process.poll() is None:
                uninstall_process.wait(timeout=10)
            if uninstall_process.returncode not in (0, None):
                raise RuntimeError(
                    f"installed Uninstall launcher終了codeが想定外です: {uninstall_process.returncode}"
                )

        uninstall_evidence = _verify_uninstalled_state(
            product_root, entries[0], audit_store, preserved_user_data
        )
        audit_evidence = _verify_live_audit(
            audit_store,
            require_uninstall=True,
            require_rollback=True,
            require_catalog=True,
            require_update_lifecycle=True,
        )
        print(
            json.dumps(
                {
                    "installed_entry": str(entries[0]),
                    "active_version": record["product_version"],
                    "package_sha256": record["package_sha256"],
                    "gtk_window": "opened_and_normally_closed",
                    "wayland_installed_window": "AT-SPI showing; owner process group stopped; surface rendering not established",
                    "owned_processes": "all_stopped",
                    "installed_ui_owner_confirmation": "native_AT-SPI_rejected_then_approved_through_GTK_runner_private_pipe",
                    "uninstall_rejection_ui": (
                        "same_window_remained_visible"
                        if reject_window_reopened
                        else "restarted_from_fixed_desktop_entry_for_fresh_approval"
                    ),
                    "manifest": manifest_evidence,
                    "update_catalog": catalog_evidence,
                    "update_lifecycle": update_evidence,
                    "uninstall": uninstall_evidence,
                    "live_audit": audit_evidence,
                    "runtime_audit": "durable_after_broker_restart",
                },
                ensure_ascii=False,
                separators=(",", ":"),
            )
        )
    finally:
        if uninstall_groups:
            _stop_owned_groups(uninstall_groups)
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
        if wayland_process is not None and wayland_process.poll() is None:
            _stop_owned_processes(
                _processes_using(product_paths) - preexisting, wayland_process
            )
        if wayland_compositor is not None and wayland_compositor.poll() is None:
            _terminate_group(wayland_compositor)
        if uninstall_process is not None and uninstall_process.poll() is None:
            _stop_owned_processes(set(), uninstall_process)
        if secret_service_process is not None and secret_service_process.poll() is None:
            _terminate_group(secret_service_process)
        if secret_service_process is not None and secret_service_process.poll() is None:
            raise RuntimeError("run固有Secret Service processを停止できません")
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
    parser.add_argument("--private-key", type=Path, required=True)
    parser.add_argument("--update-package", type=Path, required=True)
    parser.add_argument(
        "--acceptance-scope",
        choices=("update_lifecycle", "credential_vault"),
        default="update_lifecycle",
    )
    args = parser.parse_args()
    try:
        run_installed_smoke(
            args.package,
            args.temp_root,
            args.acceptance_root,
            args.private_key,
            args.update_package,
            args.acceptance_scope,
        )
    except Exception as error:
        print(f"Linux installed product GTK受入れ 不合格: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
