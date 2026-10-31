"""実Linux GTK sessionでBroker-installed productの導入とruntime provenanceを検証する。"""

from __future__ import annotations

import argparse
from collections.abc import Callable
import copy
import hashlib
import hmac
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
import posixpath
import re
import secrets
import shutil
import signal
import stat
import subprocess
import sys
import threading
import time
import traceback
import uuid
from pathlib import Path, PurePosixPath

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
MCP_PUBLIC_INPUT = "installed-mcp-public-input"
MCP_RESULT_PREFIX = "D4P_MCP_RESULT_PROBE_"
MCP_OWNER_WORKSPACE_LABEL = "Workspace: "
MCP_OWNER_ARGUMENT_HASH_MARKER = "引数hash:"
MCP_OWNER_NO_CREDENTIAL_MARKER = "Credential注入: なし"
LINUX_AGENT_TASK_FIXTURE_MODEL_ID = "o3"
# Keep the simulated Broker deadline shorter than the first real CLI tool wait,
# so the deadline scenario cannot receive the fixture's final model response first.
LINUX_AGENT_TASK_INITIAL_TOOL_YIELD_MS = 30_000
LINUX_AGENT_TASK_DEADLINE_TEST_LIMIT_MS = 20_000
RUST_AUDIT_TEST = (
    "broker::update_center::tests::linux_installed_acceptance_runtime_audit_is_durable"
)

if __package__:
    from .linux_gtk_bundle_smoke import (
        _capture_weston_frame,
        _different_pixel_count,
        _request_x11_window_manager_close,
        _start_sway_compositor,
        _start_wayland_compositor,
        _terminate_group,
    )
else:
    from linux_gtk_bundle_smoke import (
        _capture_weston_frame,
        _different_pixel_count,
        _request_x11_window_manager_close,
        _start_sway_compositor,
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


def _linux_runtime_artifact_inventory(root: Path) -> tuple[list[dict[str, object]], int, str]:
    metadata = root.lstat()
    if not stat.S_ISDIR(metadata.st_mode) or stat.S_ISLNK(metadata.st_mode):
        raise RuntimeError("runtime provenance artifact rootが通常directoryではありません")
    resolved_root = root.resolve(strict=True)
    records: list[dict[str, object]] = []
    total_bytes = 0
    tree_hash = hashlib.sha256()
    seen: set[str] = set()
    for path in sorted(
        root.rglob("*"),
        key=lambda item: item.relative_to(root).as_posix().casefold(),
    ):
        item = path.lstat()
        if stat.S_ISLNK(item.st_mode):
            raise RuntimeError("runtime provenance artifactにsymlinkがあります")
        if stat.S_ISDIR(item.st_mode):
            continue
        if not stat.S_ISREG(item.st_mode):
            raise RuntimeError("runtime provenance artifactに通常file以外があります")
        relative = path.relative_to(root).as_posix()
        relative_path = PurePosixPath(relative)
        if (
            relative_path.is_absolute()
            or ".." in relative_path.parts
            or not path.resolve(strict=True).is_relative_to(resolved_root)
            or relative.casefold() in seen
        ):
            raise RuntimeError("runtime provenance artifact pathが不正です")
        seen.add(relative.casefold())
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        records.append(
            {"path": relative, "size_bytes": item.st_size, "sha256": digest}
        )
        total_bytes += item.st_size
        tree_hash.update(
            f"{relative}\0{item.st_size}\0{digest}\n".encode("utf-8")
        )
    if not records:
        raise RuntimeError("runtime provenance artifact inventoryが空です")
    return records, total_bytes, tree_hash.hexdigest()


def _verify_linux_installed_artifact_provenance(
    *,
    build_evidence_path: Path,
    bundle_root: Path,
    package_path: Path,
    source_commit: str,
    product_root: Path,
    stage: Path,
    xdg_data_home: Path,
    xdg_state_home: Path,
    active_record: dict[str, object],
    app_id: str,
    audit_store_id: str,
) -> dict[str, object]:
    try:
        evidence_metadata = build_evidence_path.lstat()
        if not stat.S_ISREG(evidence_metadata.st_mode) or stat.S_ISLNK(
            evidence_metadata.st_mode
        ):
            raise RuntimeError("Linux build evidenceが通常fileではありません")
        evidence = json.loads(build_evidence_path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise RuntimeError("Linux build evidenceを読み取れません") from error
    if not isinstance(evidence, dict):
        raise RuntimeError("Linux build evidenceのroot objectが不正です")
    if (
        evidence.get("target_platform") != "linux"
        or evidence.get("source_commit") != source_commit
        or not re.fullmatch(r"[0-9a-f]{40}", source_commit)
        or evidence.get("signed") is not False
        or evidence.get("formal_distribution_claimed") is not False
    ):
        raise RuntimeError("Linux build evidenceのtarget／source／unsigned scopeが不一致です")
    expected_identity = {
        "source": "schema_validated_manifest",
        "app_id": app_id,
        "audit_store_id": audit_store_id,
    }
    if (
        evidence.get("app_id") != app_id
        or evidence.get("audit_store_id") != audit_store_id
        or evidence.get("compile_time_identity") != expected_identity
    ):
        raise RuntimeError("compile-time product identityがbuild evidenceと不一致です")

    bundle_records, bundle_bytes, bundle_tree = _linux_runtime_artifact_inventory(
        bundle_root
    )
    installed_records, installed_bytes, installed_tree = (
        _linux_runtime_artifact_inventory(stage)
    )
    if (
        evidence.get("artifact_files") != bundle_records
        or evidence.get("artifact_total_bytes") != bundle_bytes
        or evidence.get("artifact_tree_sha256") != bundle_tree
    ):
        raise RuntimeError("build evidenceとexport bundle inventoryが一致しません")
    if installed_records != bundle_records or installed_bytes != bundle_bytes:
        raise RuntimeError("installed stageのfile／hash inventoryがexport bundleと不一致です")

    package_metadata = package_path.lstat()
    if not stat.S_ISREG(package_metadata.st_mode) or stat.S_ISLNK(
        package_metadata.st_mode
    ):
        raise RuntimeError("runtime provenance packageが通常fileではありません")
    package_hash = hashlib.sha256(package_path.read_bytes()).hexdigest()
    package_record = evidence.get("package")
    if not isinstance(package_record, dict) or package_record != {
        "file_name": package_path.name,
        "format": "D4PKG01",
        "byte_length": package_metadata.st_size,
        "sha256": package_hash,
        "signed": False,
        "installed_product_verified": False,
    }:
        raise RuntimeError("package byte length／hashがbuild evidenceと一致しません")

    try:
        manifest = json.loads((stage / "product_manifest.json").read_text(encoding="utf-8"))
        manifest_body = manifest["manifest"]
        manifest_app_id = manifest_body["app_identity"]["app_id"]
        manifest_audit_store_id = manifest_body["audit_store"]["store_id"]
        manifest_platform = manifest_body["app_identity"]["target_platform"]
    except (OSError, UnicodeError, json.JSONDecodeError, KeyError, TypeError) as error:
        raise RuntimeError("installed Product Manifestのidentityを確認できません") from error
    if (
        manifest_app_id != app_id
        or manifest_audit_store_id != audit_store_id
        or manifest_platform != "linux"
    ):
        raise RuntimeError("installed Product Manifest identityがbuildと不一致です")

    product_version = active_record.get("product_version")
    if not isinstance(product_version, str) or not re.fullmatch(
        r"[0-9]+\.[0-9]+\.[0-9]+", product_version
    ):
        raise RuntimeError("active versionの製品版が不正です")
    if (
        active_record.get("version") != 1
        or active_record.get("product") != "D4 Pocket"
        or active_record.get("app_id") != app_id
        or active_record.get("audit_store_id") != audit_store_id
        or active_record.get("package_sha256") != package_hash
    ):
        raise RuntimeError("active version record identity／packageが不一致です")

    evidence_files = {
        item["path"]: item
        for item in bundle_records
        if isinstance(item.get("path"), str)
    }
    component_paths = {
        "launcher": "gui_shell_desktop_launcher",
        "runner": "gui_shell_desktop",
        "broker": "libexec/gui_shell_linux_broker",
        "product_manifest": "product_manifest.json",
    }
    if set(component_paths.values()) - evidence_files.keys():
        raise RuntimeError("必須installed componentがbuild inventoryにありません")
    component_hashes = {
        name: evidence_files[relative]["sha256"]
        for name, relative in component_paths.items()
    }
    if (
        active_record.get("launcher_sha256") != component_hashes["launcher"]
        or active_record.get("product_manifest_sha256")
        != component_hashes["product_manifest"]
    ):
        raise RuntimeError("active versionのlauncher／Manifest hashが不一致です")

    expected_product_root = xdg_data_home / "apps" / "d4-pocket" / app_id
    expected_stage = expected_product_root / "versions" / f"{product_version}-{package_hash}"
    if (
        product_root.resolve(strict=True) != expected_product_root.resolve(strict=True)
        or stage.resolve(strict=True) != expected_stage.resolve(strict=True)
    ):
        raise RuntimeError("installed product root／active stageが固定XDG pathと不一致です")
    fixed_launcher = product_root / "gui_shell_desktop_launcher"
    active_launcher = stage / "gui_shell_desktop_launcher"
    try:
        fixed_metadata = fixed_launcher.lstat()
        if (
            not stat.S_ISREG(fixed_metadata.st_mode)
            or stat.S_ISLNK(fixed_metadata.st_mode)
            or not os.path.samefile(fixed_launcher, active_launcher)
        ):
            raise RuntimeError("固定root launcherがactive stage launcherと同一fileではありません")
    except OSError as error:
        raise RuntimeError("固定root launcherを確認できません") from error

    audit_store = xdg_state_home / "d4-pocket" / "linux-broker" / "store"
    return {
        "evidence_source": "LIVE_RUNTIME",
        "source_commit": source_commit,
        "package_sha256": package_hash,
        "package_byte_length": package_metadata.st_size,
        "artifact_tree_sha256": bundle_tree,
        "installed_artifact_tree_sha256": installed_tree,
        "artifact_count": len(bundle_records),
        "runtime_identity": {
            "app_id": app_id,
            "audit_store_id": audit_store_id,
            "source": "compile_time_identity_and_installed_product_manifest",
        },
        "installed_root": str(product_root.resolve(strict=True)),
        "active_stage": str(stage.resolve(strict=True)),
        "component_sha256": component_hashes,
        "active_record_sha256": hashlib.sha256(
            (product_root / "active_version.json").read_bytes()
        ).hexdigest(),
        "audit_store_identity": {
            "audit_store_id": audit_store_id,
            "path": str(audit_store),
            "path_contract": "XDG_STATE_HOME/d4-pocket/linux-broker/store",
        },
    }


def _observe_linux_runtime_executables(
    expected: dict[str, tuple[Path, str]],
) -> dict[str, dict[str, object]]:
    observed: dict[str, list[str]] = {role: [] for role in expected}
    for process in Path("/proc").iterdir():
        if not process.name.isdecimal():
            continue
        try:
            executable = Path(os.readlink(process / "exe"))
        except (FileNotFoundError, PermissionError, ProcessLookupError):
            continue
        for role, (expected_path, expected_hash) in expected.items():
            if executable != expected_path.resolve(strict=True):
                continue
            try:
                digest = hashlib.sha256((process / "exe").read_bytes()).hexdigest()
            except (OSError, ProcessLookupError):
                continue
            if digest != expected_hash:
                raise RuntimeError("実行中process image hashがinstalled artifactと不一致です")
            observed[role].append(digest)
    result: dict[str, dict[str, object]] = {}
    for role, digests in observed.items():
        if not digests or len(set(digests)) != 1:
            raise RuntimeError("installed productのlive executableを確認できません")
        result[role] = {"process_count": len(digests), "sha256": digests[0]}
    return result


def _observe_linux_audit_store(store: Path) -> dict[str, object]:
    metadata = store.lstat()
    if not stat.S_ISDIR(metadata.st_mode) or stat.S_ISLNK(metadata.st_mode):
        raise RuntimeError("installed BrokerのAudit Store rootが通常directoryではありません")
    observations: dict[str, object] = {"path": str(store.resolve(strict=True))}
    for name in ("audit.jsonl", "audit_anchor.json"):
        path = store / name
        item = path.lstat()
        if not stat.S_ISREG(item.st_mode) or stat.S_ISLNK(item.st_mode):
            raise RuntimeError("installed BrokerのAudit evidence fileが通常fileではありません")
        observations[name] = {
            "byte_length": item.st_size,
            "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
        }
    key_path = store / "audit_anchor.key"
    key_metadata = key_path.lstat()
    if (
        not stat.S_ISREG(key_metadata.st_mode)
        or stat.S_ISLNK(key_metadata.st_mode)
        or key_metadata.st_mode & 0o077
    ):
        raise RuntimeError("Audit anchor keyのprivate regular-file境界が不成立です")
    observations["anchor_key_private_file"] = True
    return observations


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


def _scroll_product_page_down(environment: dict[str, str], window_id: str) -> None:
    if re.fullmatch(r"(?:0x)?[0-9a-fA-F]+", window_id) is None:
        raise RuntimeError("製品window IDが不正なため画面をscrollできません")
    activated = subprocess.run(
        ["xdotool", "windowactivate", "--sync", window_id],
        env=environment,
        capture_output=True,
        check=False,
        timeout=5,
    )
    if activated.returncode != 0:
        raise RuntimeError("製品windowを有効化できずWorkspace画面をscrollできません")
    geometry = subprocess.run(
        ["xdotool", "getwindowgeometry", "--shell", window_id],
        env=environment,
        capture_output=True,
        check=False,
        timeout=5,
    )
    if geometry.returncode != 0:
        raise RuntimeError("製品windowの表示範囲を取得できません")
    fields: dict[str, int] = {}
    for line in geometry.stdout.decode("ascii", errors="ignore").splitlines():
        key, separator, value = line.partition("=")
        if separator and key in {"WIDTH", "HEIGHT"}:
            try:
                fields[key] = int(value)
            except ValueError:
                continue
    if set(fields) != {"WIDTH", "HEIGHT"} or not (
        320 <= fields["WIDTH"] <= 4096 and 240 <= fields["HEIGHT"] <= 4096
    ):
        raise RuntimeError("製品windowの表示範囲がscroll可能な範囲ではありません")
    x, y = fields["WIDTH"] * 3 // 4, fields["HEIGHT"] * 3 // 4
    scrolled = subprocess.run(
        [
            "xdotool",
            "mousemove",
            "--window",
            window_id,
            str(x),
            str(y),
            "click",
            "5",
        ],
        env=environment,
        capture_output=True,
        check=False,
        timeout=5,
    )
    if scrolled.returncode != 0:
        raise RuntimeError("製品のWorkspace画面をscrollできません")


def _atspi_desktop_failure_summary(
    environment: dict[str, str], workspace_id: str
) -> str:
    """AT-SPI desktopの構造だけを別processで短時間・限定取得する。"""
    targets = json.dumps(
        [WINDOW_TITLE, "作業領域インスペクタ", workspace_id], ensure_ascii=True
    )
    script = f"""
import json, time
targets = {targets}
started = time.monotonic()
flags = [False, False, False]
nodes = []
status = "ok"
try:
    import pyatspi
    pending = [(pyatspi.Registry.getDesktop(0), 0)]
    while pending and len(nodes) < 64 and time.monotonic() - started < 2.0:
        node, depth = pending.pop(0)
        try:
            name = node.name or ""
        except Exception:
            name = ""
        for index, target in enumerate(targets):
            flags[index] = flags[index] or name == target
        try:
            role = str(node.getRoleName())[:40]
        except Exception:
            role = "unknown"
        try:
            child_count = max(0, min(int(node.childCount), 64))
        except Exception:
            child_count = 0
        nodes.append({{"depth": depth, "role": role, "children": child_count}})
        if depth < 3:
            for index in range(min(child_count, 64 - len(nodes))):
                if time.monotonic() - started >= 2.0:
                    break
                try:
                    pending.append((node.getChildAtIndex(index), depth + 1))
                except Exception:
                    continue
    if pending:
        status = "bounded"
except Exception:
    status = "error"
    nodes = []
print(json.dumps({{"status": status, "elapsed_ms": int((time.monotonic()-started)*1000),
                  "node_count": len(nodes), "targets": flags, "nodes": nodes}}))
"""
    try:
        result = subprocess.run(
            [sys.executable, "-c", script],
            env=environment,
            capture_output=True,
            check=False,
            timeout=3,
        )
    except subprocess.TimeoutExpired:
        return json.dumps({"status": "process_timeout"}, separators=(",", ":"))
    if result.returncode != 0 or len(result.stdout) > 8192:
        return json.dumps({"status": "process_error"}, separators=(",", ":"))
    try:
        payload = json.loads(result.stdout.decode("utf-8"))
        if not isinstance(payload, dict):
            raise ValueError("invalid payload")
        status = payload.get("status")
        if status not in {"ok", "bounded", "error"}:
            raise ValueError("invalid status")
        raw_nodes = payload.get("nodes", [])
        if not isinstance(raw_nodes, list):
            raise ValueError("invalid nodes")
        nodes = []
        for raw_node in raw_nodes[:64]:
            if not isinstance(raw_node, dict):
                continue
            role = raw_node.get("role")
            depth = raw_node.get("depth")
            children = raw_node.get("children")
            if (
                isinstance(role, str)
                and isinstance(depth, int)
                and 0 <= depth <= 3
                and isinstance(children, int)
                and 0 <= children <= 64
            ):
                nodes.append(
                    {"depth": depth, "role": role[:40], "children": children}
                )
        flags = payload.get("targets", [])
        if not isinstance(flags, list) or len(flags) != 3:
            flags = [False, False, False]
        sanitized = {
            "status": status,
            "elapsed_ms": min(max(int(payload.get("elapsed_ms", 0)), 0), 3000),
            "node_count": min(max(int(payload.get("node_count", 0)), 0), 64),
            "targets": {
                "product_window": flags[0] is True,
                "inspector_heading": flags[1] is True,
                "workspace_id": flags[2] is True,
            },
            "nodes": nodes,
        }
        return json.dumps(sanitized, ensure_ascii=True, separators=(",", ":"))
    except (UnicodeError, json.JSONDecodeError, TypeError, ValueError):
        return json.dumps({"status": "invalid_response"}, separators=(",", ":"))


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


def _publish_wayland_dbus_activation_environment(
    environment: dict[str, str],
) -> None:
    """Pass the compositor session to D-Bus-activated GTK portal services."""
    names = (
        "WAYLAND_DISPLAY",
        "XDG_RUNTIME_DIR",
        "XDG_SESSION_TYPE",
        "XDG_CURRENT_DESKTOP",
        "GDK_BACKEND",
    )
    if any(not environment.get(name) for name in names):
        raise RuntimeError("Wayland portal用のD-Bus session environmentが不足しています")
    try:
        result = subprocess.run(
            ["dbus-update-activation-environment", *names],
            env=environment,
            capture_output=True,
            check=False,
            timeout=10,
        )
    except (OSError, subprocess.TimeoutExpired):
        raise RuntimeError(
            "Wayland portal用D-Bus activation environmentを更新できません"
        ) from None
    if result.returncode != 0:
        raise RuntimeError(
            "Wayland portal用D-Bus activation environment更新が失敗しました "
            f"(exit={result.returncode})"
        )


def _parse_xdg_document_portal_mount_points(
    runtime_path: str, mount_info_text: str
) -> list[str]:
    runtime = PurePosixPath(runtime_path)
    if not runtime.is_absolute() or ".." in runtime.parts:
        raise RuntimeError("XDG Document Portal cleanup rootが不正です")
    portal_root = runtime / "doc"
    mounts: set[str] = set()
    for line in mount_info_text.splitlines():
        before_separator, separator, _ = line.partition(" - ")
        fields = before_separator.split()
        if not separator or len(fields) < 5:
            continue
        mount_path = re.sub(
            r"\\([0-7]{3})",
            lambda match: chr(int(match.group(1), 8)),
            fields[4],
        )
        candidate = PurePosixPath(mount_path)
        if candidate == portal_root or portal_root in candidate.parents:
            mounts.add(str(candidate))
    return sorted(mounts, key=lambda value: len(PurePosixPath(value).parts), reverse=True)


def _cleanup_xdg_document_portal_mounts(
    runtime: Path, temp_root: Path, environment: dict[str, str]
) -> None:
    if (
        not runtime.is_absolute()
        or runtime.is_symlink()
        or not runtime.is_dir()
        or runtime.resolve(strict=True) != runtime
        or temp_root not in runtime.parents
        or environment.get("XDG_RUNTIME_DIR") != str(runtime)
    ):
        raise RuntimeError("XDG Document Portal cleanup rootがrun固有runtime外です")
    mount_info_path = Path("/proc/self/mountinfo")
    try:
        mount_info = mount_info_path.read_text(encoding="utf-8")
    except OSError as error:
        raise RuntimeError(
            f"XDG Document Portal mount tableを読めません ({type(error).__name__})"
        ) from None
    mount_points = _parse_xdg_document_portal_mount_points(
        str(runtime), mount_info
    )
    if not mount_points:
        return
    fusermount = shutil.which("fusermount3")
    if fusermount is None:
        raise RuntimeError("XDG Document Portal mount解放にfusermount3がありません")
    for mount_point in mount_points:
        try:
            result = subprocess.run(
                [fusermount, "-u", mount_point],
                env=environment,
                capture_output=True,
                check=False,
                timeout=10,
            )
        except (OSError, subprocess.TimeoutExpired) as error:
            raise RuntimeError(
                f"run所有XDG Document Portal mountを解放できません ({type(error).__name__})"
            ) from None
        if result.returncode != 0:
            raise RuntimeError(
                "run所有XDG Document Portal mount解放が失敗しました "
                f"(exit={result.returncode})"
            )
        try:
            remaining = _parse_xdg_document_portal_mount_points(
                str(runtime), mount_info_path.read_text(encoding="utf-8")
            )
        except OSError as error:
            raise RuntimeError(
                f"XDG Document Portal mount解放後の確認ができません ({type(error).__name__})"
            ) from None
        if mount_point in remaining:
            raise RuntimeError("XDG Document Portal mountが解放後も残っています")


def _installed_acceptance_runtime_directory(
    acceptance_scope: str, acceptance_root: Path, temp_root: Path
) -> Path:
    if acceptance_scope == "wayland_workspace_selection":
        return temp_root / "sway-runtime"
    return acceptance_root / "runtime"


def _cleanup_owned_sway_runtime(runtime: Path, temp_root: Path) -> None:
    expected = temp_root / "sway-runtime"
    if (
        runtime != expected
        or not temp_root.is_absolute()
        or temp_root.is_symlink()
        or not temp_root.is_dir()
        or temp_root.resolve(strict=True) != temp_root
    ):
        raise RuntimeError("Sway runtime cleanup対象がrun所有pathではありません")
    if not runtime.exists() and not runtime.is_symlink():
        return
    try:
        metadata = runtime.lstat()
    except OSError as error:
        raise RuntimeError(
            f"Sway runtime cleanup対象を確認できません ({type(error).__name__})"
        ) from None
    if (
        not stat.S_ISDIR(metadata.st_mode)
        or metadata.st_uid != os.getuid()
        or metadata.st_mode & 0o077
        or runtime.resolve(strict=True) != runtime
    ):
        raise RuntimeError("Sway runtime cleanup対象がprivateなrun所有directoryではありません")
    shutil.rmtree(runtime)
    if runtime.exists() or runtime.is_symlink():
        raise RuntimeError("Sway runtime directoryをcleanupできません")


def _visible_wayland_window(window) -> bool:
    try:
        import pyatspi

        return window.getState().contains(pyatspi.STATE_SHOWING)
    except Exception as error:
        raise RuntimeError("導入済みWayland GTK windowの表示状態を取得できません") from error


def _is_wayland_environment(environment: dict[str, str]) -> bool:
    return bool(environment.get("WAYLAND_DISPLAY")) and environment.get(
        "XDG_SESSION_TYPE"
    ) == "wayland"


def _wayland_named_window_is_visible(name: str, *, timeout: float = 0.2) -> bool:
    try:
        window = _find_accessible(name, timeout=timeout)
    except RuntimeError:
        return False
    return _visible_wayland_window(window)


def _accessible_nodes(
    root, *, deadline: float | None = None, max_depth: int | None = None
):
    """Installed UIの現在のAT-SPI treeをboundedに列挙する。"""
    pending = [(root, 0)]
    visited = 0
    while pending and visited < 12000:
        if deadline is not None and time.monotonic() >= deadline:
            return
        node, depth = pending.pop()
        visited += 1
        try:
            name = node.name or ""
        except Exception:
            name = ""
        yield node, name
        if deadline is not None and time.monotonic() >= deadline:
            return
        if max_depth is not None and depth >= max_depth:
            continue
        try:
            count = min(node.childCount, 12000 - visited)
            for index in reversed(range(count)):
                if deadline is not None and time.monotonic() >= deadline:
                    return
                try:
                    pending.append((node.getChildAtIndex(index), depth + 1))
                except Exception:
                    continue
        except Exception:
            continue


def _find_accessible(
    name: str,
    *,
    root=None,
    timeout: float = 10.0,
    max_depth: int | None = None,
):
    try:
        import pyatspi
    except ImportError as error:
        raise RuntimeError("AT-SPI UI操作にpython3-pyatspiが必要です") from error

    deadline = time.monotonic() + timeout
    last_names: list[str] = []
    while time.monotonic() < deadline:
        search_root = root if root is not None else pyatspi.Registry.getDesktop(0)
        for node, accessible_name in _accessible_nodes(
            search_root, deadline=deadline, max_depth=max_depth
        ):
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
                raise RuntimeError(f"導入済みUIからの一覧取得に失敗しました: {status[:400]}")
            if status == unconfigured:
                raise RuntimeError(f"導入済みUIの一覧信頼設定がありません: {status}")
            if status != expected_success:
                raise RuntimeError(
                    f"導入済みUI一覧から想定外の結果が返りました: {status[:400]}"
                )
            return status
        time.sleep(0.15)
    observed_text = ", ".join(dict.fromkeys(observed[-6:]))
    raise RuntimeError(
        "導入済みUI一覧取得の終端状態が示されませんでした"
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


def _accessible_scope_with_named_descendant(
    node,
    descendant_name: str,
    *,
    max_ancestors: int = 16,
    deadline: float | None = None,
):
    """見出しから昇り、指定要素を含む最小のAT-SPI部分木を返す。"""
    current = node
    for _ in range(max_ancestors + 1):
        found = False
        for _, accessible_name in _accessible_nodes(current, deadline=deadline):
            if descendant_name in accessible_name:
                found = True
                break
        if found:
            return current
        if deadline is not None and time.monotonic() >= deadline:
            raise RuntimeError(
                f"AT-SPI部分木の検索期限切れ: {descendant_name}"
            )
        parent = _accessible_parent(current)
        if parent is None or parent is current:
            break
        current = parent
    raise RuntimeError(
        f"AT-SPI見出しの親範囲に対象要素がありません: {descendant_name}"
    )


def _editable_text_ancestor(node, *, max_depth: int = 16):
    current = node
    for _ in range(max_depth + 1):
        if current is None:
            return None
        try:
            current.queryEditableText()
            return current
        except Exception:
            current = _accessible_parent(current)
    return None


def _focus_accessible_text(editable) -> bool:
    """AT-SPI Componentまたは同じText欄のfocus/click Actionで焦点を要求する。"""
    try:
        if editable.queryComponent().grabFocus() is True:
            return True
    except Exception:
        pass
    try:
        actions = editable.queryAction()
        action_names = [
            actions.getName(index).strip().lower()
            for index in range(actions.nActions)
        ]
        for expected in (
            "focus",
            "set focus",
            "tap",
            "click",
            "press",
            "activate",
        ):
            for index, action_name in enumerate(action_names):
                if action_name == expected and actions.doAction(index):
                    return True
    except Exception:
        pass
    return False


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


def _find_operable_accessible(
    name: str, *, root=None, timeout: float = 10.0, exact: bool = False
):
    try:
        import pyatspi
    except ImportError as error:
        raise RuntimeError("AT-SPI UI操作にpython3-pyatspiが必要です") from error

    deadline = time.monotonic() + timeout
    last_debug = "no matching AT-SPI node"
    while time.monotonic() < deadline:
        search_root = root if root is not None else pyatspi.Registry.getDesktop(0)
        matches = []
        for node, accessible_name in _accessible_nodes(
            search_root, deadline=deadline
        ):
            if (
                accessible_name.strip() == name
                if exact
                else name in accessible_name.strip()
            ):
                matches.append(node)
        if not matches and time.monotonic() >= deadline:
            last_debug = "AT-SPI tree traversal reached its deadline"
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


def _find_workspace_inspector_control(
    name: str,
    workspace_id: str,
    environment: dict[str, str],
    *,
    timeout: float = 60.0,
    accessibility_root=None,
    diagnostic_screenshot_path: Path | None = None,
):
    deadline = time.monotonic() + timeout
    last_error = "not observed"
    scroll_steps = 0
    window_lookups = 0
    while scroll_steps <= 48:
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            break
        try:
            window_lookups += 1
            _, inspector = _workspace_inspector_scope(
                workspace_id,
                timeout=min(3.0, remaining),
                root=accessibility_root,
            )
            return _find_operable_accessible(
                name, root=inspector, timeout=min(0.5, remaining)
            )
        except RuntimeError as error:
            last_error = str(error)[:400]

        remaining = deadline - time.monotonic()
        if remaining <= 0:
            break
        window_id = _visible_product_window(environment)
        if window_id is not None:
            _scroll_product_page_down(environment, window_id)
            scroll_steps += 1
        time.sleep(min(0.12, remaining))

    visible_window_id = _visible_product_window(environment)
    screenshot_captured = (
        _capture_gtk_dialog_screenshot(
            environment, WINDOW_TITLE, diagnostic_screenshot_path
        )
        if diagnostic_screenshot_path is not None
        else False
    )
    atspi_summary = _atspi_desktop_failure_summary(environment, workspace_id)
    raise RuntimeError(
        "Workspace Inspector control did not become operable after bounded "
        f"page scrolling: control={name}; workspace_id={workspace_id}; "
        f"window_lookups={window_lookups}; scroll_steps={scroll_steps}; "
        f"x11_window_visible={visible_window_id is not None}; "
        f"screenshot_captured={screenshot_captured}; "
        f"atspi_desktop={atspi_summary}; last_error={last_error}"
    )


def _workspace_inspector_scope(
    workspace_id: str, *, timeout: float = 15.0, root=None
):
    deadline = time.monotonic() + timeout
    if root is not None:
        registration = _find_accessible(
            workspace_id,
            root=root,
            timeout=min(3.0, timeout),
            max_depth=20,
        )
        inspector = _accessible_scope_with_named_descendant(
            registration, "登録状態を更新", deadline=deadline
        )
        return root, inspector

    try:
        # アプリ名のAT-SPI検索が停止する画面では、可視の意味見出しを
        # 先に探し、その配下に対象Workspaceがあることを確認する。
        heading = _find_accessible(
            "作業領域インスペクタ",
            timeout=min(3.0, timeout),
            max_depth=20,
        )
        inspector = _accessible_scope_with_named_descendant(
            heading, workspace_id, deadline=deadline
        )
        return None, inspector
    except RuntimeError:
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise

    window = None
    try:
        window = _find_accessible(
            WINDOW_TITLE, timeout=min(2.0, remaining), max_depth=4
        )
        heading = _find_accessible(
            "作業領域インスペクタ",
            root=window,
            timeout=max(0.01, deadline - time.monotonic()),
            max_depth=12,
        )
    except RuntimeError:
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise
        heading = _find_accessible(
            "作業領域インスペクタ",
            timeout=remaining,
            max_depth=20,
        )
    inspector = _accessible_scope_with_named_descendant(
        heading, workspace_id, deadline=deadline
    )
    return window, inspector


def _select_workspace_inspector_visibility(
    workspace_id: str,
    visibility: str,
    environment: dict[str, str],
    *,
    diagnostic_screenshot_path: Path | None = None,
) -> None:
    """現在値のDropdownを開いてから、製品window内の選択肢を選ぶ。"""
    _, inspector = _workspace_inspector_scope(workspace_id, timeout=10)
    current_value = _find_operable_accessible(
        "hash_only", root=inspector, timeout=10, exact=True
    )
    _activate_accessible(current_value, "hash_only", environment)

    try:
        # Dropdown menuはInspectorの子ではなくOverlayとして公開されるため、
        # 開いた後に製品windowの現行AT-SPI treeから選択肢を取り直す。
        product_window = _find_accessible(WINDOW_TITLE, timeout=10)
        option = _find_operable_accessible(
            visibility, root=product_window, timeout=10, exact=True
        )
    except RuntimeError:
        if diagnostic_screenshot_path is not None:
            _capture_gtk_dialog_screenshot(
                environment, WINDOW_TITLE, diagnostic_screenshot_path
            )
        raise
    _activate_accessible(option, visibility, environment)


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
    if _is_wayland_environment(environment):
        raise RuntimeError(
            f"Waylandでは座標入力へfallbackせず、AT-SPI actionを要求します: {name}"
        )
    _click_accessible_by_coordinates(node, name, environment)


def _click_accessible_by_coordinates(
    node, name: str, environment: dict[str, str]
) -> None:
    center = _component_center(node)
    if center is None:
        raise RuntimeError(f"AT-SPI controlに可視boundsがありません: {name}")
    clicked = subprocess.run(
        ["xdotool", "mousemove", "--sync", str(center[0]), str(center[1]), "click", "1"],
        env=environment,
        capture_output=True,
        check=False,
        timeout=5,
    )
    if clicked.returncode != 0:
        raise RuntimeError(f"GTK controlのAT-SPI座標clickに失敗しました: {name}")


def _double_click_accessible_by_coordinates(
    node, name: str, environment: dict[str, str]
) -> None:
    center = _component_center(node)
    if center is None:
        raise RuntimeError(f"AT-SPI controlに可視boundsがありません: {name}")
    activated = subprocess.run(
        [
            "xdotool",
            "mousemove",
            "--sync",
            str(center[0]),
            str(center[1]),
            "click",
            "--repeat",
            "2",
            "--delay",
            "100",
            "1",
        ],
        env=environment,
        capture_output=True,
        check=False,
        timeout=5,
    )
    if activated.returncode != 0:
        raise RuntimeError(f"GTK folder rowのAT-SPI座標double-clickに失敗しました: {name}")


def _accessible_descendants(root) -> list[tuple[object, str]]:
    return list(_accessible_nodes(root))


def _accessible_text(node) -> str:
    """GTKラベルは本文をAT-SPIのTextとして公開し、Accessible.nameは使わない場合があります。"""
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
        [
            "xdotool",
            "type",
            "--clearmodifiers",
            "--delay",
            "30",
            "mcp",
        ],
        env=environment,
        check=True,
        timeout=5,
    )
    _find_accessible(
        "MCP接続一覧とOwner確認付き切断を開く",
        root=window,
        timeout=10,
    )
    # 対応済みのTextField経由でASCII文字のMCP問合せを送信する
    # onSubmitted経路を使う。GTKのAT-SPI action nodeには位置情報がない場合があり、xdotoolでは
    # 日本語を入力できない。最初のMCP結果から同じ画面を開ける。
    subprocess.run(
        ["xdotool", "key", "--clearmodifiers", "Return"],
        env=environment,
        check=True,
        timeout=5,
    )
    # コマンドパレットのdialogを閉じると、古いAT-SPI
    # 部分木が無効になることがある。desktopから移動先を特定してから、
    # controlを調べる呼出元へ返す前に製品windowを再取得する。
    _find_accessible("MCP接続センター", timeout=10)
    window = _find_accessible(WINDOW_TITLE, timeout=10)
    _find_accessible("MCP接続センター", root=window, timeout=10)
    return window


def _set_public_text_by_keyboard(
    root,
    label: str,
    value: str,
    environment: dict[str, str],
    window_id: str | None,
) -> None:
    if (
        not value
        or len(value) > 256
        or any(
            ord(character) < 0x20 and character != "\n"
            for character in value
        )
    ):
        raise RuntimeError("公開GTK欄の値が空か、許容形式を超えています")
    if _is_wayland_environment(environment):
        _set_public_text_by_wayland_keyboard(root, label, value, environment)
        return
    node = _find_accessible(label, root=root, timeout=10)
    editable_node = _editable_text_ancestor(node)

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
        raise RuntimeError("公開GTKテキスト欄にキーボード焦点が移りませんでした")
    if editable_node is None:
        editable_node = _editable_text_ancestor(focused)
    if editable_node is None:
        raise RuntimeError(f"公開GTKテキスト欄をキーボードで操作できません: {label}")
    subprocess.run(
        ["xdotool", "key", "--clearmodifiers", "ctrl+a"],
        env=environment,
        check=True,
        timeout=5,
    )
    lines = value.split("\n")
    for index, line in enumerate(lines):
        if line:
            subprocess.run(
                ["xdotool", "type", "--clearmodifiers", "--delay", "20", line],
                env=environment,
                check=True,
                timeout=5,
            )
        if index < len(lines) - 1:
            subprocess.run(
                ["xdotool", "key", "--clearmodifiers", "Return"],
                env=environment,
                check=True,
                timeout=5,
            )
    deadline = time.monotonic() + 2
    while time.monotonic() < deadline:
        if _accessible_text(editable_node) == value:
            return
        time.sleep(0.05)
    raise RuntimeError(f"公開GTKテキスト欄{label!r}に入力した識別子が反映されませんでした")


def _set_public_text_by_wayland_keyboard(
    root, label: str, value: str, environment: dict[str, str]
) -> None:
    """AT-SPIで公開欄へfocusし、Wayland仮想keyboardで実キー入力する。"""
    editable_candidates = []
    for node, accessible_name in _accessible_nodes(root, deadline=time.monotonic() + 5):
        if label not in accessible_name:
            continue
        editable = _editable_text_ancestor(node)
        if editable is not None and all(
            _accessible_identity(editable) != _accessible_identity(candidate)
            for candidate in editable_candidates
        ):
            editable_candidates.append(editable)
    if len(editable_candidates) != 1:
        raise RuntimeError(
            f"公開Wayland欄のAT-SPI EditableTextが一意ではありません: {label}"
        )
    editable = editable_candidates[0]
    focus_requested = _focus_accessible_text(editable)
    if not focus_requested:
        raise RuntimeError(
            f"公開Wayland欄へkeyboard focusを設定できません: {label}"
        )

    def send_keys(arguments: list[str], *, text: str | None = None) -> None:
        timeout_seconds = 15
        if text is not None and "-d" in arguments:
            delay_index = arguments.index("-d") + 1
            delay_ms = int(arguments[delay_index])
            timeout_seconds = max(
                timeout_seconds, len(text) * delay_ms / 1000 + 5
            )
        try:
            result = subprocess.run(
                ["wtype", *arguments],
                env=environment,
                input=text,
                text=True,
                capture_output=True,
                check=False,
                timeout=timeout_seconds,
            )
        except (OSError, subprocess.TimeoutExpired):
            raise RuntimeError("Wayland仮想keyboard入力を実行できません") from None
        if result.returncode != 0:
            raise RuntimeError(
                "Wayland仮想keyboard入力が失敗しました "
                f"(exit={result.returncode})"
            )

    observed_length = 0
    candidate_count = 0
    initial_delay_ms = 100 if len(value) > 30 else 40
    delay_attempts = (initial_delay_ms, max(initial_delay_ms, 160))
    for attempt, delay_ms in enumerate(delay_attempts):
        if attempt and not _focus_accessible_text(editable):
            break
        send_keys(["-M", "ctrl", "-k", "a", "-m", "ctrl"])
        lines = value.split("\n")
        for index, line in enumerate(lines):
            if line:
                send_keys(["-d", str(delay_ms), "-"], text=line)
            if index < len(lines) - 1:
                send_keys(["-k", "Return"])
        deadline = time.monotonic() + 2
        while time.monotonic() < deadline:
            direct_text = _accessible_text(editable)
            observed_length = len(direct_text)
            if direct_text == value:
                return
            current_candidates = []
            for node, accessible_name in _accessible_nodes(
                root, deadline=min(deadline, time.monotonic() + 0.25)
            ):
                if label not in accessible_name:
                    continue
                current = _editable_text_ancestor(node)
                if current is not None and all(
                    _accessible_identity(current) != _accessible_identity(candidate)
                    for candidate in current_candidates
                ):
                    current_candidates.append(current)
            candidate_count = len(current_candidates)
            if candidate_count == 1:
                current_text = _accessible_text(current_candidates[0])
                observed_length = len(current_text)
                if current_text == value:
                    return
            time.sleep(0.05)
    raise RuntimeError(
        f"公開GTK欄{label!r}に設定した値が反映されませんでした "
        f"(focus_requested={'yes' if focus_requested else 'no'}, "
        f"editable_candidates={candidate_count}, observed_length={observed_length}, "
        f"expected_length={len(value)}, attempts={len(delay_attempts)}; "
        f"{_accessibility_debug(editable)})"
    )


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
        raise RuntimeError("native GTK秘密入力欄を一意に特定できません")
    try:
        if editable[0].queryEditableText().setTextContents(secret) is False:
            raise RuntimeError("native GTK秘密入力欄が入力を拒否しました")
    except Exception:
        raise RuntimeError("native GTK秘密入力欄が試験入力を受け付けませんでした") from None


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
            "秘密保管サービスのメタデータ検査にはpython3-dbusが必要です"
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
                raise RuntimeError("秘密保管サービス項目のメタデータが不正です")
            labels.append(label)
        return labels
    except dbus.exceptions.DBusException:
        raise RuntimeError("秘密保管サービスのメタデータ検索に失敗しました") from None


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
            raise RuntimeError("秘密保管サービスへ渡すパスワード用標準入力を利用できません")
        process.stdin.write(b"\n")
        process.stdin.close()
        environment["GNOME_KEYRING_CONTROL"] = str(control_directory)
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline:
            if process.poll() is not None:
                raise RuntimeError("隔離した秘密保管サービスが起動中に終了しました")
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
        raise RuntimeError("隔離した利用者sessionの秘密保管サービスが利用可能になりませんでした")
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
        raise RuntimeError("資格情報の秘密が永続監査に漏えいしました")
    events = [json.loads(line) for line in audit_bytes.decode("utf-8").splitlines() if line]
    if not events:
        raise RuntimeError("資格情報受入れで永続監査が生成されませんでした")
    previous_hash: str | None = None
    for index, event in enumerate(events, start=1):
        if (
            event.get("event_id") != f"broker-audit-{index}"
            or event.get("previous_event_hash") != previous_hash
        ):
            raise RuntimeError("資格情報の監査事象順序が不正です")
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
            raise RuntimeError("資格情報の監査事象ハッシュが不正です")
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
        raise RuntimeError("資格情報の監査アンカー／HMACが不正です")
    operations = [event.get("operation", "") for event in events]
    if not any("資格情報登録" in operation for operation in operations):
        raise RuntimeError("資格情報登録の監査記録がありません")
    if not any("資格情報失効" in operation for operation in operations):
        raise RuntimeError("資格情報失効の監査記録がありません")
    for operation_name in ("資格情報登録", "資格情報失効"):
        decisions = {
            event.get("decision")
            for event in events
            if event.get("operation") == operation_name
        }
        if not {"rejected", "accepted"}.issubset(decisions):
            raise RuntimeError(f"{operation_name} 拒否と許可の判断が別々に監査記録されていません")
    return {
        "events": len(events),
        "registration_events": sum(operation == "資格情報登録" for operation in operations),
        "revocation_events": sum(operation == "資格情報失効" for operation in operations),
    }


_CREDENTIAL_SCAN_CHUNK_BYTES = 1024 * 1024
# ホスト型の導入済みGTK受入れ試験で10,677件を走査中に512 MiBを超えました。
# 通常の受入れfile（合計572,613,198 byte以上）です。走査を完全に保ちつつ
# 固定1 MiBメモリ単位を使い、合計byte数の上限を1 GiBに保ちます。
_CREDENTIAL_SCAN_MAX_BYTES = 1024 * 1024 * 1024


def _credential_scan_file_role(path: Path, root: Path) -> str:
    parts = path.relative_to(root).parts
    if parts[:4] == ("home", ".local", "share", "keyrings"):
        return "secret_service_store"
    if parts[:2] == ("home", ".cache"):
        return "user_cache"
    if parts[:3] == ("home", ".local", "state"):
        return "user_state"
    if parts[:2] == ("home", ".config"):
        return "user_config"
    if parts[0] in {"tmp", "runtime"}:
        return "temporary_or_runtime"
    if parts[:3] == ("home", ".local", "share"):
        return "user_data"
    if path.suffix.lower() in {".log", ".txt", ".json", ".jsonl", ".trace"}:
        return "diagnostic_or_text"
    if parts[0] == "home":
        return "home_other"
    return "other_regular_file"


def _credential_scan_role_summary(
    totals: dict[str, tuple[int, int]],
) -> str:
    return ",".join(
        f"{role}(files={file_count},bytes={byte_count})"
        for role, (file_count, byte_count) in sorted(totals.items())
    ) or "none"


def _assert_secret_absent_from_files(
    root: Path, secret: str, *, excluded_roots: tuple[Path, ...] = ()
) -> None:
    marker = secret.encode("utf-8")
    excluded = tuple(path.resolve(strict=True) for path in excluded_roots)
    remaining = _CREDENTIAL_SCAN_MAX_BYTES
    files_scanned = 0
    role_totals: dict[str, tuple[int, int]] = {}

    def budget_error(
        *,
        next_file_bytes: int | None = None,
        next_file_role: str | None = None,
    ) -> RuntimeError:
        next_file = (
            f", next_file_bytes={next_file_bytes}, next_file_role={next_file_role}"
            if next_file_bytes is not None and next_file_role is not None
            else ""
        )
        return RuntimeError(
            "資格情報の受入れファイル走査が上限バイト数を超えました "
            f"(files_scanned={files_scanned}, "
            f"scanned_bytes={_CREDENTIAL_SCAN_MAX_BYTES - remaining}"
            f"{next_file}, scanned_role_totals={_credential_scan_role_summary(role_totals)})"
        )

    for path in root.rglob("*"):
        if any(path.is_relative_to(excluded_root) for excluded_root in excluded):
            continue
        try:
            metadata = path.lstat()
            if not stat.S_ISREG(metadata.st_mode):
                continue
            role = _credential_scan_file_role(path, root)
            if metadata.st_size > remaining:
                raise budget_error(
                    next_file_bytes=metadata.st_size,
                    next_file_role=role,
                )
            role_file_count, role_byte_count = role_totals.get(role, (0, 0))
            role_totals[role] = (role_file_count + 1, role_byte_count)
            with path.open("rb") as stream:
                overlap = b""
                while chunk := stream.read(min(_CREDENTIAL_SCAN_CHUNK_BYTES, remaining + 1)):
                    remaining -= len(chunk)
                    role_totals[role] = (
                        role_totals[role][0],
                        role_totals[role][1] + len(chunk),
                    )
                    if remaining < 0:
                        raise budget_error(
                            next_file_bytes=metadata.st_size,
                            next_file_role=role,
                        )
                    content = overlap + chunk
                    if marker in content:
                        raise RuntimeError(
                            "資格情報の秘密が受入れファイルまたはログへ漏えいしました"
                        )
                    overlap = content[-(len(marker) - 1) :] if len(marker) > 1 else b""
            files_scanned += 1
        except OSError:
            raise RuntimeError("資格情報の受入れファイル走査で試験ファイルを読めませんでした") from None


def _accessible_snapshot(root, *, deadline: float | None = None) -> str:
    values: list[str] = []
    for node, name in _accessible_nodes(root, deadline=deadline):
        if deadline is not None and time.monotonic() >= deadline:
            break
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
        raise RuntimeError("導入済みUIに公開資格情報IDが一つだけ表示されませんでした")
    credential_id = next(iter(credential_ids))
    labels = _secret_service_item_labels()
    if sorted(labels) != ["D4 Pocket credential ciphertext", "D4 Pocket credential key"]:
        raise RuntimeError("秘密保管サービスに鍵と暗号文が別々の項目として一つずつ存在しません")
    if secret in names:
        raise RuntimeError("資格情報の秘密が導入済みUIのアクセシビリティ木へ漏えいしました")

    window_id = restart_application(window_id)
    if _secret_service_item_labels() != labels:
        raise RuntimeError("Broker再起動で実行単位が所有する秘密保管サービス項目が変化しました")
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
        raise RuntimeError("資格情報失効の拒否で秘密保管サービス項目が変化しました")

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
        raise RuntimeError("論理失効で秘密保管サービス項目が予期せず削除されました")

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


def _write_linux_mcp_fixture(
    temp_root: Path, acceptance_root: Path
) -> tuple[Path, Path, str, str]:
    workspace = acceptance_root / "mcp-workspace"
    workspace.mkdir(mode=0o700)
    metadata = workspace.lstat()
    if (
        workspace.is_symlink()
        or not stat.S_ISDIR(metadata.st_mode)
        or metadata.st_uid != os.getuid()
        or metadata.st_mode & 0o077
    ):
        raise RuntimeError("導入済みMCPの試験用作業領域が非公開で所有者に束縛されていません")

    fixture = temp_root / "linux-installed-mcp-server.py"
    if fixture.exists() or fixture.is_symlink():
        raise RuntimeError("実行単位所有のMCP試験用pathが既に使われています")
    source = '''import json
import os
import sys

seed = sys.argv[1]

def write_private(name, value):
    fd = os.open(name, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(fd, "w", encoding="utf-8") as stream:
        json.dump(value, stream, separators=(",", ":"))
        stream.flush()
        os.fsync(stream.fileno())

write_private("mcp-child-started.json", {"pid": os.getpid(), "pgid": os.getpgrp()})

for line in sys.stdin:
    request = json.loads(line)
    method = request.get("method")
    request_id = request.get("id")
    if request_id is None:
        continue
    if method == "server/discover":
        response = {
            "jsonrpc": "2.0",
            "id": request_id,
            "result": {
                "resultType": "complete",
                "supportedVersions": ["2026-07-28"],
                "capabilities": {"tools": {}},
            },
            "_meta": {"io.modelcontextprotocol/serverInfo": {"name": "installed-fixture", "version": "1"}},
        }
    elif method == "tools/list":
        response = {
            "jsonrpc": "2.0",
            "id": request_id,
            "result": {
                "resultType": "complete",
                "tools": [{
                    "name": "echo",
                    "inputSchema": {
                        "type": "object",
                        "properties": {"text": {"type": "string"}},
                        "required": ["text"],
                        "additionalProperties": False,
                    },
                }],
            },
        }
    elif method == "tools/call":
        arguments = request.get("params", {}).get("arguments", {})
        if request.get("params", {}).get("name") != "echo" or arguments != {"text": "installed-mcp-public-input"}:
            raise SystemExit(73)
        write_private("mcp-tool-called.json", {"method": method, "count": 1})
        response = {
            "jsonrpc": "2.0",
            "id": request_id,
            "result": {
                "resultType": "complete",
                "content": [{"type": "text", "text": "D4P_MCP_RESULT_PROBE_" + seed}],
                "isError": False,
            },
        }
    else:
        response = {"jsonrpc": "2.0", "id": request_id, "error": {"code": -32601, "message": "method not found"}}
    sys.stdout.write(json.dumps(response, separators=(",", ":")) + "\\n")
    sys.stdout.flush()
'''
    with fixture.open("x", encoding="utf-8", newline="\n") as stream:
        stream.write(source)
    fixture.chmod(0o600)
    fixture_metadata = fixture.lstat()
    if (
        fixture.is_symlink()
        or not stat.S_ISREG(fixture_metadata.st_mode)
        or fixture_metadata.st_nlink != 1
        or fixture_metadata.st_uid != os.getuid()
        or fixture_metadata.st_mode & 0o077
        or temp_root.resolve(strict=True) not in fixture.resolve(strict=True).parents
    ):
        raise RuntimeError("実行単位所有のMCP試験用fileが非公開の通常fileではありません")

    seed = secrets.token_hex(16)
    result_marker = MCP_RESULT_PREFIX + seed
    return workspace, fixture, seed, result_marker


def _read_mcp_child_identity(workspace: Path) -> tuple[int, int]:
    marker = workspace / "mcp-child-started.json"
    metadata = marker.lstat()
    if (
        marker.is_symlink()
        or not stat.S_ISREG(metadata.st_mode)
        or metadata.st_nlink != 1
        or metadata.st_uid != os.getuid()
        or metadata.st_mode & 0o077
    ):
        raise RuntimeError("導入済みMCP子processの識別markerが非公開の通常fileではありません")
    try:
        document = json.loads(marker.read_text(encoding="utf-8"))
        pid = int(document["pid"])
        pgid = int(document["pgid"])
    except (OSError, UnicodeError, json.JSONDecodeError, KeyError, TypeError, ValueError) as error:
        raise RuntimeError("導入済みMCP子processの識別markerの形式が不正です") from error
    if pid <= 1 or pgid <= 1 or pid == pgid or pgid == os.getpgrp():
        raise RuntimeError("導入済みMCP子processが隔離された所有process群に属していません")
    return pid, pgid


def _mcp_process_group_is_fixture_owned(
    pid: int, pgid: int, *, helper: Path, fixture: Path
) -> bool:
    if pgid <= 1 or pgid == os.getpgrp():
        return False
    try:
        leader_executable = Path(os.readlink(f"/proc/{pgid}/exe")).resolve(strict=True)
        leader_arguments = Path(f"/proc/{pgid}/cmdline").read_bytes().split(b"\0")
        if (
            leader_executable != helper.resolve(strict=True)
            or b"--internal-process-group-guardian-v1" not in leader_arguments
        ):
            return False
        try:
            child_executable = Path(os.readlink(f"/proc/{pid}/exe")).resolve(strict=True)
            child_arguments = Path(f"/proc/{pid}/cmdline").read_bytes().split(b"\0")
        except (FileNotFoundError, ProcessLookupError):
            return True
        return (
            os.getpgid(pid) == pgid
            and fixture.as_posix().encode() in child_arguments
            and child_executable.is_file()
        )
    except (OSError, ProcessLookupError, PermissionError):
        return False


def _wait_for_mcp_group_state(pgid: int, should_exist: bool, *, timeout: float = 15.0) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if _group_exists(pgid) == should_exist:
            return
        time.sleep(0.1)
    expected = "alive" if should_exist else "stopped"
    raise RuntimeError(f"導入済みMCP所有process群の状態が {expected}")


def _cleanup_mcp_fixture_group(workspace: Path, *, helper: Path, fixture: Path) -> None:
    marker = workspace / "mcp-child-started.json"
    if not marker.exists() and not marker.is_symlink():
        return
    _, pgid = _read_mcp_child_identity(workspace)
    if not _group_exists(pgid):
        return
    if not _mcp_process_group_is_fixture_owned(
        *_read_mcp_child_identity(workspace), helper=helper, fixture=fixture
    ):
        raise RuntimeError("実行単位が所有すると証明できないMCP process群の清掃を拒否します")
    _stop_owned_groups({pgid})
    _wait_for_mcp_group_state(pgid, False)


def _expand_single_installed_mcp_tool_list(
    environment: dict[str, str], window: object
) -> None:
    label = "Tool一覧：1件"
    tool_list = _find_operable_accessible(label, root=window, timeout=15)
    _activate_accessible(tool_list, label, environment)
    _find_accessible("echo", root=window, timeout=15)


def _enter_single_installed_mcp_tool_arguments(
    environment: dict[str, str], window: object, window_id: str
) -> None:
    _find_accessible("echo のarguments", root=window, timeout=10)
    arguments = json.dumps({"text": MCP_PUBLIC_INPUT}, ensure_ascii=False)
    _set_public_text_by_keyboard(
        window,
        "JSON形式のobject",
        arguments,
        environment,
        window_id,
    )
    confirm_arguments = _find_operable_accessible(
        "入力内容を確認", root=window, timeout=10
    )
    _activate_accessible(confirm_arguments, "入力内容を確認", environment)


def _review_single_installed_mcp_tool_arguments(
    environment: dict[str, str], window: object
) -> None:
    _find_accessible("送信する引数を確認", root=window, timeout=10)
    preview = _accessible_snapshot(window)
    if MCP_PUBLIC_INPUT not in preview:
        raise RuntimeError(
            "導入済みMCPの公開引数が製品UIのアクセシビリティ木に表示されていません"
        )
    if "Tool結果本文は返却されず" not in preview:
        raise RuntimeError(
            "導入済みMCP結果の露出範囲表示が製品UIにありません"
        )
    proceed = _find_operable_accessible("Linux確認へ進む", root=window, timeout=10)
    _activate_accessible(proceed, "Linux確認へ進む", environment)


WORKSPACE_PUBLIC_CONTENT = "D4P_WORKSPACE_PUBLIC_PROJECTION_OK"
WORKSPACE_ENV_SECRET = "D4P_WORKSPACE_ENV_SECRET_CANARY_71A9"
WORKSPACE_REGISTERED_SECRET = "D4P_WORKSPACE_REGISTERED_SECRET_CANARY_4F2C"
WORKSPACE_OUTSIDE_SECRET = "D4P_WORKSPACE_OUTSIDE_SECRET_CANARY_93D1"


def _gtk_dialog_window_id(
    environment: dict[str, str], title: str
) -> str | None:
    if _is_wayland_environment(environment):
        _find_accessible(title, timeout=10)
        return None
    result = subprocess.run(
        ["xdotool", "search", "--onlyvisible", "--name", f"^{re.escape(title)}$"],
        env=environment,
        capture_output=True,
        check=False,
        timeout=5,
    )
    if result.returncode != 0:
        raise RuntimeError("GTK OS選択dialogの表示を確認できません")
    window_ids = result.stdout.decode("ascii", errors="ignore").splitlines()
    if len(window_ids) != 1 or not window_ids[0].isdecimal():
        raise RuntimeError("GTK OS選択dialogが一意ではありません")
    return window_ids[0]


def _assert_gtk_chooser_open(
    environment: dict[str, str], window_id: str | None
) -> None:
    if _is_wayland_environment(environment):
        _find_accessible("D4 Pocket 作業領域の選択", timeout=2)
        return
    visible = subprocess.run(
        [
            "xdotool",
            "search",
            "--onlyvisible",
            "--name",
            "^D4 Pocket 作業領域の選択$",
        ],
        env=environment,
        capture_output=True,
        check=False,
        timeout=5,
    )
    window_ids = visible.stdout.decode("ascii", errors="ignore").splitlines()
    if visible.returncode != 0 or window_ids != [window_id]:
        raise RuntimeError(
            "フォルダー移動中にGTKの選択画面が閉じたか、別ウィンドウへ切り替わりました"
        )


def _capture_gtk_dialog_screenshot(
    environment: dict[str, str], title: str, destination: Path
) -> bool:
    try:
        if destination.exists() or destination.is_symlink():
            return False
        search = subprocess.run(
            ["xdotool", "search", "--onlyvisible", "--name", f"^{re.escape(title)}$"],
            env=environment,
            capture_output=True,
            check=False,
            timeout=5,
        )
        if search.returncode != 0:
            return False
        window_ids = search.stdout.decode("ascii", errors="ignore").splitlines()
        if len(window_ids) != 1 or not window_ids[0].isdecimal():
            return False
        geometry = subprocess.run(
            ["xdotool", "getwindowgeometry", "--shell", window_ids[0]],
            env=environment,
            capture_output=True,
            check=False,
            timeout=5,
        )
        if geometry.returncode != 0:
            return False
        fields = {}
        for line in geometry.stdout.decode("ascii", errors="ignore").splitlines():
            key, separator, value = line.partition("=")
            if separator and key in {"X", "Y", "WIDTH", "HEIGHT"}:
                fields[key] = int(value)
        if set(fields) != {"X", "Y", "WIDTH", "HEIGHT"}:
            return False
        x, y, width, height = (
            fields["X"],
            fields["Y"],
            fields["WIDTH"],
            fields["HEIGHT"],
        )
        if (
            width <= 0
            or height <= 0
            or width > 4096
            or height > 4096
            or abs(x) > 8192
            or abs(y) > 8192
        ):
            return False
        import gi

        gi.require_version("Gdk", "3.0")
        from gi.repository import Gdk

        display = Gdk.Display.get_default()
        screen = display.get_default_screen() if display is not None else None
        root = screen.get_root_window() if screen is not None else None
        pixbuf = (
            Gdk.pixbuf_get_from_window(root, x, y, width, height)
            if root is not None
            else None
        )
        if pixbuf is None:
            return False
        old_umask = os.umask(0o077)
        try:
            pixbuf.savev(str(destination), "png", [], [])
        finally:
            os.umask(old_umask)
        metadata = destination.lstat()
        if not stat.S_ISREG(metadata.st_mode) or metadata.st_nlink != 1:
            destination.unlink(missing_ok=True)
            return False
        destination.chmod(0o600)
        return True
    except Exception:
        return False


def _wait_for_gtk_dialog_closed(
    environment: dict[str, str],
    title: str,
    *,
    action: str,
    timeout: float = 10.0,
) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if _is_wayland_environment(environment):
            if not _wayland_named_window_is_visible(
                title, timeout=min(0.2, deadline - time.monotonic())
            ):
                return
            time.sleep(0.1)
            continue
        result = subprocess.run(
            ["xdotool", "search", "--onlyvisible", "--name", f"^{re.escape(title)}$"],
            env=environment,
            capture_output=True,
            check=False,
            timeout=5,
        )
        if result.returncode != 0:
            return
        time.sleep(0.1)
    raise RuntimeError(f"GTK OS選択dialogが{action}後も閉じませんでした")


def _gtk_location_entry_candidates(chooser):
    candidates = []
    for node, _ in _accessible_nodes(chooser):
        try:
            node.queryEditableText()
        except Exception:
            continue
        bounds = _accessible_component_bounds(node)
        value = _accessible_text(node).strip()
        if (
            bounds is None
            or bounds[1] >= 120
            or bounds[2] < 100
            or not value.startswith("/")
            or len(value) > 4096
        ):
            continue
        candidates.append((bounds[2], value, node))
    return candidates


def _gtk_location_entry_value(chooser) -> str | None:
    """GTK chooser上部の編集可能な絶対path欄を一意に読む。"""
    candidates = _gtk_location_entry_candidates(chooser)
    if not candidates:
        return None
    largest_width = max(width for width, _, _ in candidates)
    widest = {value for width, value, _ in candidates if width == largest_width}
    return next(iter(widest)) if len(widest) == 1 else None


def _normalize_gtk_path(value: str) -> str:
    return posixpath.normpath(value.replace("\\", "/"))


def _wait_for_gtk_location(chooser, folder: Path, timeout: float = 2) -> bool:
    expected = _normalize_gtk_path(str(folder))
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        value = _gtk_location_entry_value(chooser)
        if value is not None:
            actual = _normalize_gtk_path(value)
            if actual == expected:
                return True
        time.sleep(0.1)
    return False


def _workspace_root_field_value(*, timeout: float = 10.0) -> str:
    window = _find_accessible(WINDOW_TITLE, timeout=timeout)
    node = _find_accessible(
        "Workspace rootの絶対path", root=window, timeout=timeout
    )
    for _ in range(7):
        try:
            node.queryEditableText()
            return _accessible_text(node)
        except Exception:
            node = _accessible_parent(node)
            if node is None:
                break
    raise RuntimeError("Workspace root入力fieldの現在値を確認できません")


def _wait_for_workspace_root_value(expected: str, *, timeout: float = 5.0) -> None:
    deadline = time.monotonic() + timeout
    observed: str | None = None
    read_error: str | None = None
    while True:
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            break
        try:
            observed = _workspace_root_field_value(timeout=min(0.5, remaining))
            read_error = None
            if observed == expected:
                return
        except RuntimeError as error:
            read_error = type(error).__name__
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            break
        time.sleep(min(0.1, remaining))

    observed_basename = (
        Path(observed).name if observed and observed.startswith("/") else "unavailable"
    )
    raise RuntimeError(
        "GTK chooser selection did not update the Workspace input after bounded "
        "fresh-window checks; "
        f"expected_basename={Path(expected).name}; "
        f"observed_basename={observed_basename}; "
        f"field_read_error={read_error or 'none'}"
    )


def _open_workspace_folder_chooser(
    environment: dict[str, str], dialog
) -> str | None:
    button = _find_operable_accessible("OSで作業領域を選択", root=dialog, timeout=10)
    _activate_accessible(button, "OSで作業領域を選択", environment)
    _find_accessible("D4 Pocket 作業領域の選択", timeout=10)
    return _gtk_dialog_window_id(environment, "D4 Pocket 作業領域の選択")


def _select_folder_in_gtk_chooser(
    environment: dict[str, str],
    chooser_window_id: str | None,
    folder: Path,
    *,
    screenshot_before_selection_path: Path | None = None,
    screenshot_path: Path | None = None,
) -> None:
    if not folder.is_absolute() or not folder.is_dir() or folder.is_symlink():
        raise RuntimeError("GTK chooserへ渡すtest directoryが不正です")
    home_root = Path(environment.get("HOME", ""))
    try:
        resolved_home = home_root.resolve(strict=True)
        resolved_picker_root = folder.parent.parent.resolve(strict=True)
    except (OSError, RuntimeError):
        raise RuntimeError(
            "GTKの選択試験用directoryが隔離HOME外です"
        ) from None
    if (
        not home_root.is_absolute()
        or home_root.is_symlink()
        or resolved_home != home_root
        or resolved_picker_root != resolved_home
    ):
        raise RuntimeError(
            "GTKの選択試験用directoryは隔離HOME直下のpicker直下workspaceに限ります"
        )

    if _is_wayland_environment(environment):
        _select_folder_in_wayland_gtk_chooser(
            environment,
            folder,
            screenshot_before_selection_path=screenshot_before_selection_path,
            screenshot_path=screenshot_path,
        )
        return

    if chooser_window_id is None:
        raise RuntimeError("X11 GTK chooserのwindow IDがありません")

    subprocess.run(
        ["xdotool", "windowactivate", "--sync", chooser_window_id],
        env=environment,
        check=True,
        timeout=5,
    )
    chooser = _find_accessible("D4 Pocket 作業領域の選択", timeout=10)

    home_control = _find_operable_accessible(
        "Home", root=chooser, timeout=10, exact=True
    )
    _activate_accessible(home_control, "Home", environment)
    _assert_gtk_chooser_open(environment, chooser_window_id)

    try:
        picker_row = _wait_for_gtk_folder_row(chooser, folder.parent)
    except RuntimeError as error:
        diagnostics = _gtk_folder_chooser_diagnostics(chooser, folder)
        diagnostics["navigation_stage"] = "home_to_picker_root"
        screenshot_captured = (
            _capture_gtk_dialog_screenshot(
                environment,
                "D4 Pocket 作業領域の選択",
                screenshot_path,
            )
            if screenshot_path is not None
            else False
        )
        raise RuntimeError(
            f"{error}; bounded_state={diagnostics}; screenshot_captured={screenshot_captured}"
        ) from None

    _double_click_accessible_by_coordinates(
        picker_row, "Workspace picker root row", environment
    )
    _assert_gtk_chooser_open(environment, chooser_window_id)

    try:
        row = _wait_for_gtk_folder_row(chooser, folder)
    except RuntimeError as error:
        diagnostics = _gtk_folder_chooser_diagnostics(chooser, folder)
        diagnostics["navigation_stage"] = "picker_root_to_workspace"
        screenshot_captured = (
            _capture_gtk_dialog_screenshot(
                environment,
                "D4 Pocket 作業領域の選択",
                screenshot_path,
            )
            if screenshot_path is not None
            else False
        )
        raise RuntimeError(
            f"{error}; bounded_state={diagnostics}; screenshot_captured={screenshot_captured}"
        ) from None

    screenshot_before_selection_captured = (
        _capture_gtk_dialog_screenshot(
            environment,
            "D4 Pocket 作業領域の選択",
            screenshot_before_selection_path,
        )
        if screenshot_before_selection_path is not None
        else False
    )
    _click_accessible_by_coordinates(row, "Workspace folder row", environment)
    _wait_for_gtk_folder_row_selected(chooser, folder)
    accept = _find_operable_accessible("選択", root=chooser, timeout=10, exact=True)
    _activate_accessible(accept, "選択", environment)
    coordinate_fallback_used = False
    try:
        _wait_for_gtk_dialog_closed(
            environment,
            "D4 Pocket 作業領域の選択",
            action="AT-SPI exact accept action",
            timeout=2,
        )
        return
    except RuntimeError:
        coordinate_fallback_used = True
        _click_accessible_by_coordinates(accept, "選択", environment)
    try:
        _wait_for_gtk_dialog_closed(
            environment,
            "D4 Pocket 作業領域の選択",
            action="AT-SPI action and exact-control coordinate fallback",
        )
    except RuntimeError:
        diagnostics = _gtk_folder_chooser_diagnostics(chooser, folder)
        diagnostics["exact_accept_coordinate_fallback_used"] = (
            coordinate_fallback_used
        )
        diagnostics["selected_workspace_row_confirmed"] = True
        screenshot_captured = (
            _capture_gtk_dialog_screenshot(
                environment,
                "D4 Pocket 作業領域の選択",
                screenshot_path,
            )
            if screenshot_path is not None
            else False
        )
        diagnostics.update(
            {
                "selection_target_source": "verified_home_navigation_and_exact_visible_row",
                "screenshot_before_selection_captured": (
                    screenshot_before_selection_captured
                ),
                "screenshot_captured": screenshot_captured,
            }
        )
        raise RuntimeError(
            "GTK folder selection did not close after AT-SPI action and exact-control coordinate fallback; "
            f"bounded_state={diagnostics}"
        ) from None


def _select_folder_in_wayland_gtk_chooser(
    environment: dict[str, str],
    folder: Path,
    *,
    screenshot_before_selection_path: Path | None,
    screenshot_path: Path | None,
) -> None:
    chooser = _find_accessible("D4 Pocket 作業領域の選択", timeout=10)
    candidates = _gtk_location_entry_candidates(chooser)
    if not candidates:
        # GTK file choosers hide the location entry until the standard Ctrl+L
        # shortcut is used. Keep this as real Wayland keyboard input: the
        # acceptance must exercise the installed chooser, not mutate its
        # accessibility model.
        try:
            result = subprocess.run(
                ["wtype", "-M", "ctrl", "-k", "l", "-m", "ctrl"],
                env=environment,
                capture_output=True,
                check=False,
                timeout=5,
            )
        except (OSError, subprocess.TimeoutExpired):
            raise RuntimeError(
                "Wayland GTK chooserの場所入力を開けませんでした"
            ) from None
        if result.returncode != 0:
            raise RuntimeError(
                "Wayland GTK chooserの場所入力を開けませんでした "
                f"(exit={result.returncode})"
            )

    else:
        widest = max(width for width, _, _ in candidates)
        location_nodes = [node for width, _, node in candidates if width == widest]
        if len(location_nodes) != 1:
            raise RuntimeError("Wayland GTK chooserのpath欄を一意に特定できません")
        location = location_nodes[0]
        if not _focus_accessible_text(location):
            raise RuntimeError("Wayland GTK chooserの場所入力へfocusできません")

    def send_keys(arguments: list[str], *, text: str | None = None) -> None:
        timeout_seconds = 15
        if text is not None and "-d" in arguments:
            delay_index = arguments.index("-d") + 1
            delay_ms = int(arguments[delay_index])
            timeout_seconds = max(
                timeout_seconds, len(text) * delay_ms / 1000 + 5
            )
        try:
            result = subprocess.run(
                ["wtype", *arguments],
                env=environment,
                input=text,
                text=True,
                capture_output=True,
                check=False,
                timeout=timeout_seconds,
            )
        except (OSError, subprocess.TimeoutExpired):
            raise RuntimeError("Wayland GTK chooserへの実キー入力に失敗しました") from None
        if result.returncode != 0:
            raise RuntimeError(
                "Wayland GTK chooserへの実キー入力に失敗しました "
                f"(exit={result.returncode})"
            )

    initial_delay_ms = 100 if len(str(folder)) > 30 else 40
    delay_attempts = (initial_delay_ms, max(initial_delay_ms, 160))
    exact_folder_visible = False
    for attempt, delay_ms in enumerate(delay_attempts):
        if attempt:
            # Reopen/focus GTK's native location prompt, then replace the
            # incomplete value. Never infer selection from the typed path.
            send_keys(["-M", "ctrl", "-k", "l", "-m", "ctrl"])
        send_keys(["-M", "ctrl", "-k", "a", "-m", "ctrl"])
        send_keys(["-d", str(delay_ms), "-"], text=str(folder))
        send_keys(["-k", "Return"])

        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            chooser = _find_accessible("D4 Pocket 作業領域の選択", timeout=1)
            if (
                _normalize_gtk_path(_gtk_location_entry_value(chooser) or "")
                == _normalize_gtk_path(str(folder))
            ):
                exact_folder_visible = True
                break
            time.sleep(0.1)
        if exact_folder_visible:
            break

    if not exact_folder_visible:
        diagnostics = _gtk_folder_chooser_diagnostics(chooser, folder)
        raise RuntimeError(
            "Wayland GTK chooser did not select the exact isolated Workspace path; "
            f"attempts={len(delay_attempts)}; bounded_state={diagnostics}"
        )

    accept = _find_operable_accessible("選択", root=chooser, timeout=10, exact=True)
    _activate_accessible(accept, "選択", environment)
    _wait_for_gtk_dialog_closed(
        environment,
        "D4 Pocket 作業領域の選択",
        action="Wayland GTK exact Workspace path and accept action",
    )


def _gtk_file_list_region(chooser) -> tuple[int, int, int, int, int] | None:
    table_bounds = []
    header_bottoms = []
    for node, _ in _accessible_nodes(chooser):
        try:
            role = str(node.getRoleName())
        except Exception:
            continue
        bounds = _accessible_component_bounds(node)
        if bounds is None:
            continue
        if role == "table":
            table_bounds.append(bounds)
        elif role == "table column header":
            header_bottoms.append(bounds[1] + bounds[3])
    if len(table_bounds) != 1 or not header_bottoms:
        return None
    x, y, width, height = table_bounds[0]
    rows_top = max(y, max(header_bottoms))
    if width <= 0 or height <= 0 or rows_top >= y + height:
        return None
    return (x, y, width, height, rows_top)


def _gtk_bounds_are_in_file_list(
    bounds: tuple[int, int, int, int], region: tuple[int, int, int, int, int] | None
) -> bool:
    if region is None:
        return False
    x, y, width, height = bounds
    table_x, table_y, table_width, table_height, rows_top = region
    center_x = x + width // 2
    center_y = y + height // 2
    return (
        table_x <= center_x < table_x + table_width
        and rows_top <= center_y < table_y + table_height
    )


def _gtk_folder_row_candidates(chooser, folder: Path) -> list[object]:
    role_priority = {"table cell": 0, "list item": 1, "icon": 2, "text": 3}
    region = _gtk_file_list_region(chooser)
    if region is None:
        return []
    candidates = []
    for node, accessible_name in _accessible_nodes(chooser):
        try:
            role = str(node.getRoleName())
        except Exception:
            continue
        if role not in role_priority:
            continue
        bounds = _accessible_component_bounds(node)
        if bounds is None or not _gtk_bounds_are_in_file_list(bounds, region):
            continue
        text = _accessible_text(node).strip()
        if accessible_name.strip() != folder.name and text != folder.name:
            continue
        try:
            import pyatspi

            if not node.getState().contains(pyatspi.STATE_SHOWING):
                continue
        except Exception:
            continue
        candidates.append(
            (
                0 if accessible_name.strip() == folder.name else 1,
                role_priority[role],
                bounds,
                node,
            )
        )
    if not candidates:
        return []
    candidates.sort(key=lambda item: (item[0], item[1]))
    best_score = candidates[0][:2]
    best = [item for item in candidates if item[:2] == best_score]
    unique_bounds = {item[2] for item in best}
    if len(unique_bounds) != 1:
        return []
    return [best[0][3]]


def _wait_for_gtk_folder_row(
    chooser, folder: Path, *, timeout: float = 10.0
):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        candidates = _gtk_folder_row_candidates(chooser, folder)
        if candidates:
            return candidates[0]
        time.sleep(0.15)
    diagnostics = _gtk_folder_chooser_diagnostics(chooser, folder)
    raise RuntimeError(
        "GTK Workspace parent listing did not expose one exact visible folder row; "
        f"bounded_state={diagnostics}"
    )


def _gtk_accessible_or_parent_selected(node, *, max_ancestors: int = 5) -> bool:
    try:
        import pyatspi
    except ImportError:
        return False
    current = node
    for _ in range(max_ancestors + 1):
        if current is None:
            break
        try:
            if current.getState().contains(pyatspi.STATE_SELECTED):
                return True
        except Exception:
            pass
        current = _accessible_parent(current)
    return False


def _wait_for_gtk_folder_row_selected(
    chooser, folder: Path, *, timeout: float = 10.0
) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        candidates = _gtk_folder_row_candidates(chooser, folder)
        if candidates and _gtk_accessible_or_parent_selected(candidates[0]):
            return
        time.sleep(0.15)
    diagnostics = _gtk_folder_chooser_diagnostics(chooser, folder)
    raise RuntimeError(
        "GTK Workspace row did not enter the selected state; "
        f"bounded_state={diagnostics}"
    )


def _gtk_folder_chooser_diagnostics(chooser, folder: Path) -> dict[str, object]:
    snapshot = _accessible_snapshot(chooser)
    location_value = _gtk_location_entry_value(chooser)
    expected_parent = _normalize_gtk_path(str(folder.parent))
    normalized_location = (
        _normalize_gtk_path(location_value) if location_value is not None else None
    )
    location_matches_parent = normalized_location == expected_parent
    location_is_autocompleted_child = (
        normalized_location is not None
        and posixpath.dirname(normalized_location) == expected_parent
    )
    try:
        import pyatspi
    except ImportError:
        pyatspi = None
    role_counts: dict[str, int] = {}
    selected_count = 0
    for node, _ in _accessible_nodes(chooser):
        try:
            role = str(node.getRoleName())
        except Exception:
            role = "unknown"
        role_counts[role] = role_counts.get(role, 0) + 1
        if pyatspi is not None:
            try:
                if node.getState().contains(pyatspi.STATE_SELECTED):
                    selected_count += 1
            except Exception:
                pass
    common_roles = sorted(role_counts.items(), key=lambda item: (-item[1], item[0]))[:12]
    focused_roles = _gtk_focused_accessible_roles(chooser)
    header_bounds = []
    table_bounds = []
    target_entries = []
    visible_row_entries = []
    file_list_region = _gtk_file_list_region(chooser)
    for node, accessible_name in _accessible_nodes(chooser):
        try:
            role = str(node.getRoleName())
        except Exception:
            role = "unknown"
        bounds = _accessible_component_bounds(node)
        if role == "table column header" and bounds is not None and len(header_bounds) < 5:
            header_bounds.append(bounds)
        if role == "table" and bounds is not None and len(table_bounds) < 3:
            table_bounds.append(bounds)
        text = _accessible_text(node)
        if (
            role in {"table cell", "list item", "icon", "text"}
            and bounds is not None
            and _gtk_bounds_are_in_file_list(bounds, file_list_region)
            and len(visible_row_entries) < 20
        ):
            visible_row_entries.append(
                {
                    "role": role,
                    "name_exact": accessible_name.strip() == folder.name,
                    "text_exact": text.strip() == folder.name,
                    "name_contains": folder.name in accessible_name,
                    "text_contains": folder.name in text,
                    "bounds": bounds,
                }
            )
        if (
            role in {"table cell", "list item", "icon", "text"}
            and bounds is not None
            and _gtk_bounds_are_in_file_list(bounds, file_list_region)
            and (
                accessible_name.strip() == folder.name
                or text.strip() == folder.name
                or folder.name in accessible_name
                or folder.name in text
            )
            and len(target_entries) < 4
        ):
            parent = _accessible_parent(node)
            try:
                parent_role = str(parent.getRoleName()) if parent is not None else "none"
            except Exception:
                parent_role = "unknown"
            target_entries.append(
                {
                    "role": role,
                    "name_exact": accessible_name.strip() == folder.name,
                    "text_exact": text.strip() == folder.name,
                    "name_contains": folder.name in accessible_name,
                    "text_contains": folder.name in text,
                    "bounds": bounds,
                    "parent_role": parent_role,
                    "parent_bounds": _accessible_component_bounds(parent)
                    if parent is not None
                    else None,
                    "selected": _gtk_accessible_or_parent_selected(node),
                }
            )
    return {
        "workspace_parent_path_visible": str(folder.parent) in snapshot,
        "location_entry_matches_workspace_parent": location_matches_parent,
        "location_entry_is_autocompleted_workspace_child": (
            location_is_autocompleted_child
        ),
        "workspace_folder_row_visible": bool(target_entries),
        "workspace_folder_row_selected": any(
            entry["selected"] for entry in target_entries
        ),
        "selected_accessible_count": selected_count,
        "common_roles": common_roles,
        "focused_roles": focused_roles,
        "visible_header_bounds": header_bounds,
        "visible_table_bounds": table_bounds,
        "visible_row_entries": visible_row_entries,
        "target_entries": target_entries,
    }


def _accessible_component_bounds(node) -> tuple[int, int, int, int] | None:
    if node is None:
        return None
    try:
        import pyatspi

        bounds = node.queryComponent().getExtents(pyatspi.XY_SCREEN)
        values = tuple(int(value) for value in (bounds.x, bounds.y, bounds.width, bounds.height))
    except Exception:
        return None
    x, y, width, height = values
    if (
        width <= 0
        or height <= 0
        or width > 4096
        or height > 4096
        or abs(x) > 8192
        or abs(y) > 8192
    ):
        return None
    return values


def _gtk_focused_accessible_roles(chooser) -> list[tuple[str, int]]:
    try:
        import pyatspi
    except ImportError:
        return []
    roles: dict[str, int] = {}
    for node, _ in _accessible_nodes(chooser):
        try:
            if node.getState().contains(pyatspi.STATE_FOCUSED):
                role = str(node.getRoleName())
                roles[role] = roles.get(role, 0) + 1
        except Exception:
            continue
    return sorted(roles.items())


def _cancel_gtk_folder_chooser(
    environment: dict[str, str], chooser_window_id: str | None
) -> None:
    if not _is_wayland_environment(environment):
        if chooser_window_id is None:
            raise RuntimeError("X11 GTK chooserのwindow IDがありません")
        subprocess.run(
            ["xdotool", "windowactivate", "--sync", chooser_window_id],
            env=environment,
            check=True,
            timeout=5,
        )
    chooser = _find_accessible("D4 Pocket 作業領域の選択", timeout=10)
    cancel = _find_operable_accessible("取消", root=chooser, timeout=10)
    if _is_wayland_environment(environment):
        _activate_accessible(cancel, "取消", environment)
    else:
        _click_accessible_by_coordinates(cancel, "取消", environment)
    _wait_for_gtk_dialog_closed(
        environment,
        "D4 Pocket 作業領域の選択",
        action=(
            "AT-SPI cancel control activation"
            if _is_wayland_environment(environment)
            else "cancel control click"
        ),
    )


def _verify_workspace_projection(
    snapshot: str,
    *,
    public_markers: tuple[str, ...],
    forbidden_markers: tuple[str, ...],
) -> dict[str, int]:
    if not public_markers or any(marker not in snapshot for marker in public_markers):
        raise RuntimeError("approved public Workspace content is not visible")
    if any(marker and marker in snapshot for marker in forbidden_markers):
        raise RuntimeError("Workspace Inspector exposed secret or external content")
    return {
        "public_markers_visible": len(public_markers),
        "secret_and_external_markers_absent": len(forbidden_markers),
    }


def _wait_for_workspace_projection(
    snapshot: Callable[[], str],
    *,
    public_markers: tuple[str, ...],
    forbidden_markers: tuple[str, ...],
    timeout: float = 15.0,
) -> tuple[str, dict[str, int]]:
    """AT-SPIのAccessible.nameとText interfaceを使い、boundedに表示を待つ。"""
    deadline = time.monotonic() + timeout
    latest_snapshot = ""
    snapshot_observed = False
    while True:
        latest_snapshot = snapshot()
        snapshot_observed = True
        if any(marker and marker in latest_snapshot for marker in forbidden_markers):
            raise RuntimeError("Workspace Inspector exposed secret or external content")
        if all(marker in latest_snapshot for marker in public_markers):
            return latest_snapshot, _verify_workspace_projection(
                latest_snapshot,
                public_markers=public_markers,
                forbidden_markers=forbidden_markers,
            )
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            statuses = (
                "取得時点の表示です。承認状態を定期確認しています。",
                "取得または現在の承認を確認できません。登録状態を更新してください。",
                "承認済み範囲を取得中",
            )
            read_status = next(
                (status for status in statuses if status in latest_snapshot),
                "unknown",
            )
            raise RuntimeError(
                "approved public Workspace content is not visible; "
                f"snapshot_observed={snapshot_observed}; "
                f"read_status={read_status}"
            )
        time.sleep(min(0.15, remaining))


def _write_linux_workspace_acceptance_fixture(
    acceptance_root: Path,
    isolated_home: Path,
) -> tuple[Path, Path, Path, Path, Path]:
    resolved_acceptance_root = acceptance_root.resolve(strict=True)
    resolved_home = isolated_home.resolve(strict=True)
    if (
        isolated_home.is_symlink()
        or resolved_home != isolated_home
        or resolved_home.parent != resolved_acceptance_root
    ):
        raise RuntimeError("Workspace acceptance HOME must be a direct isolated child")
    fixture_root = isolated_home / "workspace-picker"
    workspace = fixture_root / "selected-workspace"
    outside_root = fixture_root / "outside-root"
    decoy_workspace = fixture_root / "decoy-workspace"
    fixture_root.mkdir(mode=0o700)
    workspace.mkdir(mode=0o700)
    outside_root.mkdir(mode=0o700)
    decoy_workspace.mkdir(mode=0o700)
    (workspace / "public.txt").write_text(WORKSPACE_PUBLIC_CONTENT, encoding="utf-8")
    (workspace / ".env").write_text(WORKSPACE_ENV_SECRET, encoding="utf-8")
    registered_secret_root = workspace / "private"
    registered_secret_root.mkdir(mode=0o700)
    (registered_secret_root / "token.txt").write_text(
        WORKSPACE_REGISTERED_SECRET, encoding="utf-8"
    )
    outside_file = outside_root / "external.txt"
    outside_file.write_text(WORKSPACE_OUTSIDE_SECRET, encoding="utf-8")
    (workspace / "external-alias.txt").symlink_to(outside_file)

    cli_marker = acceptance_root / "workspace-cli-probe.jsonl"
    cli_fixture = acceptance_root / "workspace-codex-cli-fixture"
    marker_literal = json.dumps(str(cli_marker))
    cli_fixture.write_text(
        "#!/usr/bin/python3\n"
        "import json, pathlib, sys\n"
        f"marker = pathlib.Path({marker_literal})\n"
        "with marker.open('a', encoding='utf-8') as stream:\n"
        "    stream.write(json.dumps(sys.argv[1:]) + '\\n')\n"
        "args = sys.argv[1:]\n"
        "if args == ['--version']:\n"
        "    print('codex-cli 0.1.0')\n"
        "elif args == ['exec', '--help']:\n"
        "    print('codex exec --sandbox --cd --json --ephemeral --ignore-user-config --skip-git-repo-check --model')\n"
        "else:\n"
        "    raise SystemExit(2)\n",
        encoding="utf-8",
    )
    cli_fixture.chmod(0o700)
    return fixture_root, workspace, decoy_workspace, cli_fixture, cli_marker


def _open_agent_center_from_installed_ui(
    environment: dict[str, str], window_id: str | None
):
    window = _find_accessible(WINDOW_TITLE, timeout=10)
    if _is_wayland_environment(environment):
        palette_button = _find_operable_accessible(
            "コマンドパレット", root=window, timeout=10
        )
        _activate_accessible(palette_button, "コマンドパレット", environment)
        _find_accessible("コマンドパレット", root=window, timeout=10)
        # The product lists page commands before any query is entered. Select
        # the exact visible action directly; the search field shares its
        # accessible label with the launcher button and is ambiguous in some
        # GTK/AT-SPI trees.
        command = _find_operable_accessible(
            "エージェントセンターを開く", root=window, timeout=10
        )
        _activate_accessible(command, "エージェントセンターを開く", environment)
        _find_accessible("エージェントセンター", timeout=10)
        window = _find_accessible(WINDOW_TITLE, timeout=10)
        _find_accessible("エージェントセンター", root=window, timeout=10)
        return window
    if window_id is None:
        raise RuntimeError("Agent Centerへ移動するX11 window IDがありません")
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
    command = _find_operable_accessible(
        "エージェントセンターを開く", root=window, timeout=10
    )
    _activate_accessible(command, "エージェントセンターを開く", environment)
    _find_accessible("エージェントセンター", timeout=10)
    window = _find_accessible(WINDOW_TITLE, timeout=10)
    _find_accessible("エージェントセンター", root=window, timeout=10)
    return window


def _open_installed_codex_registration(
    environment: dict[str, str], *, window_id: str | None = None
):
    current_window_id = (
        window_id
        if _is_wayland_environment(environment)
        else (_visible_product_window(environment) or window_id)
    )
    if current_window_id is None and not _is_wayland_environment(environment):
        raise RuntimeError("Agent Centerへ移動するinstalled product windowがありません")
    window = _open_agent_center_from_installed_ui(environment, current_window_id)
    try:
        button = _find_operable_accessible("登録を開始", root=window, timeout=10)
    except RuntimeError as error:
        snapshot = _accessible_snapshot(window)
        if (
            "broker_unavailable" in snapshot
            or "RustブローカーIPCを利用できないか、接続が拒否されました。" in snapshot
        ):
            raise RuntimeError(
                "installed Agent Center reports broker_unavailable; registration remained fail-closed"
            ) from error
        raise RuntimeError(
            "Agent Center registration control was not exposed and no bounded broker-unavailable marker was found"
        ) from error
    _activate_accessible(button, "登録を開始", environment)
    _find_accessible("Codex実行系をBrokerへ登録", root=window, timeout=10)
    return window


def _add_responses_sequence_numbers(
    events: list[dict[str, object]],
) -> list[dict[str, object]]:
    for sequence_number, event in enumerate(events):
        event["sequence_number"] = sequence_number
    return events


def _responses_lifecycle_event(
    event_type: str,
    response_id: str,
    status: str,
    output: list[dict[str, object]],
) -> dict[str, object]:
    completed = status == "completed"
    return {
        "type": event_type,
        "response": {
            "id": response_id,
            "object": "response",
            "access_programs": None,
            "status": status,
            "output": output,
            "created_at": 1,
            "completed_at": 1 if completed else None,
            "background": False,
            "error": None,
            "incomplete_details": None,
            "instructions": None,
            "max_output_tokens": None,
            "max_tool_calls": None,
            "model": LINUX_AGENT_TASK_FIXTURE_MODEL_ID,
            "parallel_tool_calls": True,
            "previous_response_id": None,
            "reasoning": {"effort": None, "summary": None},
            "service_tier": "default",
            "store": True,
            "temperature": 1.0,
            "text": {"format": {"type": "text"}},
            "tool_choice": "auto",
            "tools": [],
            "top_p": 1.0,
            "truncation": "disabled",
            "usage": (
                {
                    "input_tokens": 1,
                    "input_tokens_details": {"cached_tokens": 0, "cache_write_tokens": 0},
                    "output_tokens": 1,
                    "output_tokens_details": {"reasoning_tokens": 0},
                    "total_tokens": 2,
                }
                if completed
                else None
            ),
            "user": None,
            "metadata": {},
        },
    }


def _responses_final_message_events(request_number: int) -> list[dict[str, object]]:
    response_id = f"resp_d4p_linux_agent_task_{request_number}"
    message_id = f"msg_d4p_linux_agent_task_{request_number}"
    text = "合成installed Agent Task fixtureが完了しました。"
    started = {
        "type": "message",
        "role": "assistant",
        "id": message_id,
        "status": "in_progress",
        "content": [],
    }
    completed = {
        "type": "message",
        "role": "assistant",
        "id": message_id,
        "status": "completed",
        "content": [{"type": "output_text", "text": text, "annotations": [], "logprobs": []}],
    }
    events: list[dict[str, object]] = [
        _responses_lifecycle_event("response.created", response_id, "in_progress", []),
        _responses_lifecycle_event("response.in_progress", response_id, "in_progress", []),
        {
            "type": "response.output_item.added",
            "response_id": response_id,
            "output_index": 0,
            "item": started,
        },
        {
            "type": "response.content_part.added",
            "response_id": response_id,
            "item_id": message_id,
            "output_index": 0,
            "content_index": 0,
            "part": {"type": "output_text", "text": "", "annotations": [], "logprobs": []},
        },
        {
            "type": "response.output_text.delta",
            "response_id": response_id,
            "item_id": message_id,
            "output_index": 0,
            "content_index": 0,
            "delta": text,
            "logprobs": [],
        },
        {
            "type": "response.output_text.done",
            "response_id": response_id,
            "item_id": message_id,
            "output_index": 0,
            "content_index": 0,
            "text": text,
            "logprobs": [],
        },
        {
            "type": "response.content_part.done",
            "response_id": response_id,
            "item_id": message_id,
            "output_index": 0,
            "content_index": 0,
            "part": {"type": "output_text", "text": text, "annotations": [], "logprobs": []},
        },
        {
            "type": "response.output_item.done",
            "response_id": response_id,
            "output_index": 0,
            "item": completed,
        },
        _responses_lifecycle_event("response.completed", response_id, "completed", [completed]),
    ]
    return _add_responses_sequence_numbers(events)


def _normalize_r2_e2e_diagnostic(payload: object) -> dict[str, object] | None:
    if not isinstance(payload, dict) or not isinstance(payload.get("kind"), str):
        return None
    kind = payload["kind"]
    stages = {
        "secret_path_validation",
        "scratch_create",
        "temporary_environment_bind",
        "codex_task_execution",
        "scratch_cleanup",
    }
    failure_classes = {
        "要求不正",
        "実行系不在",
        "権限拒否",
        "セッション不一致",
        "通信失敗",
        "期限超過",
        "応答不正",
        "監査失敗",
        "取消",
        "実行系隔離済み",
        "作業領域不在",
        "AgentTask実行非対応",
    }
    error_kinds = {
        "NotFound",
        "PermissionDenied",
        "ConnectionRefused",
        "ConnectionReset",
        "ConnectionAborted",
        "NotConnected",
        "AddrInUse",
        "AddrNotAvailable",
        "BrokenPipe",
        "AlreadyExists",
        "WouldBlock",
        "InvalidInput",
        "InvalidData",
        "TimedOut",
        "WriteZero",
        "Interrupted",
        "UnexpectedEof",
        "OutOfMemory",
        "Other",
        "Unsupported",
    }
    if kind == "task_failure":
        stage, failure_class = payload.get("stage"), payload.get("failure_class")
        if set(payload) != {"kind", "stage", "failure_class"}:
            return None
        if (
            not isinstance(stage, str)
            or not isinstance(failure_class, str)
            or stage not in stages
            or failure_class not in failure_classes
        ):
            return None
        return {"stage": stage, "failure_class": failure_class}
    if kind == "os_failure":
        stage, error_kind = payload.get("stage"), payload.get("os_error_kind")
        if set(payload) != {"kind", "stage", "os_error_kind"}:
            return None
        if not isinstance(stage, str) or stage not in {
            "codex_process_spawn",
            "codex_stdin_write",
        }:
            return None
        if not isinstance(error_kind, str) or error_kind not in error_kinds:
            return None
        return {"stage": stage, "os_error_kind": error_kind}
    if kind == "missing_stdin_pipe":
        if set(payload) != {"kind"}:
            return None
        return {"stage": "codex_stdin_pipe_missing"}
    if kind != "cli_failure" or set(payload) != {"kind", "summary"}:
        return None
    summary = payload.get("summary")
    if not isinstance(summary, str):
        return None
    match = re.fullmatch(
        r'exit_code=(Some\(-?\d+\)|None); '
        r'stdout_event_types=(\[(?:"(?:thread\.started|turn\.started|turn\.completed|'
        r'turn\.failed|error|item\.started|item\.completed|other)"(?:, )?)*\]); '
        r'stdout_error_class=(none|authentication|model_unavailable|rate_limited|'
        r'invalid_request|timeout|transport|service_error|other); '
        r'stdout_http_status=(none|[45]\d\d); '
        r'stderr_class=(empty|sandbox_setup|authentication|configuration|permission|'
        r'transport|missing_dependency|other); stderr_bytes=(\d+)',
        summary,
    )
    if match is None:
        return None
    exit_code, event_json, stdout_error_class, http_status, stderr_class, stderr_bytes = (
        match.groups()
    )
    try:
        event_types = json.loads(event_json)
        byte_count = int(stderr_bytes)
    except (json.JSONDecodeError, ValueError):
        return None
    if not isinstance(event_types, list) or len(event_types) > 32 or byte_count > 1_048_576:
        return None
    return {
        "stage": "codex_cli_exit",
        "exit_code": exit_code,
        "stdout_event_types": event_types,
        "stdout_error_class": stdout_error_class,
        "stdout_http_status": None if http_status == "none" else int(http_status),
        "stderr_class": stderr_class,
        "stderr_bytes": byte_count,
    }


class _LinuxAgentTaskResponsesServer:
    """Codex Responses wire fixture bound to loopback and one active Task at a time."""

    def __init__(self) -> None:
        fixture = self

        class Handler(BaseHTTPRequestHandler):
            protocol_version = "HTTP/1.1"

            def log_message(self, _format: str, *_args: object) -> None:
                return

            def _send(self, status: int, content_type: str, body: bytes) -> None:
                self.close_connection = True
                self.send_response(status)
                self.send_header("Content-Type", content_type)
                self.send_header("Content-Length", str(len(body)))
                self.send_header("Connection", "close")
                self.end_headers()
                self.wfile.write(body)

            def do_GET(self) -> None:
                if self.headers.get("Authorization") is not None:
                    fixture._reject_request("unexpected_authorization_header")
                    self._send(401, "application/json", b"{}")
                    return
                if self.path.split("?", 1)[0] not in {"/v1/models", "/models"}:
                    fixture._reject_request("unexpected_get")
                    self._send(404, "application/json", b"{}")
                    return
                fixture._record_model_request()
                body = json.dumps(
                    {
                        "object": "list",
                        "data": [
                            {
                                "id": LINUX_AGENT_TASK_FIXTURE_MODEL_ID,
                                "object": "model",
                                "created": 0,
                                "owned_by": "d4-pocket-local-fixture",
                            }
                        ],
                    },
                    separators=(",", ":"),
                ).encode("utf-8")
                self._send(200, "application/json", body)

            def do_POST(self) -> None:
                if self.headers.get("Authorization") is not None:
                    fixture._reject_request("unexpected_authorization_header")
                    self._send(401, "application/json", b"{}")
                    return
                if self.path.split("?", 1)[0] == "/__d4p_r2_e2e/diagnostics":
                    try:
                        length = int(self.headers.get("Content-Length", "-1"))
                        if length < 0 or length > 4096:
                            raise ValueError
                        payload = json.loads(self.rfile.read(length))
                    except (OSError, UnicodeError, ValueError, json.JSONDecodeError):
                        fixture._reject_request("malformed_diagnostic")
                        self._send(400, "application/json", b"{}")
                        return
                    diagnostic = _normalize_r2_e2e_diagnostic(payload)
                    if diagnostic is None:
                        fixture._reject_request("unsafe_diagnostic_rejected")
                        self._send(400, "application/json", b"{}")
                        return
                    fixture._record_diagnostic(diagnostic)
                    self._send(200, "application/json", b"{}")
                    return
                if self.path.split("?", 1)[0] not in {"/v1/responses", "/responses"}:
                    fixture._reject_request("unexpected_post")
                    self._send(404, "application/json", b"{}")
                    return
                try:
                    length = int(self.headers.get("Content-Length", "-1"))
                    if length < 0 or length > 2 * 1024 * 1024:
                        raise ValueError
                    request = json.loads(self.rfile.read(length))
                    if not isinstance(request, dict):
                        raise ValueError
                except (OSError, UnicodeError, ValueError, json.JSONDecodeError):
                    fixture._reject_request("malformed_request")
                    self._send(400, "application/json", b"{}")
                    return
                try:
                    events = fixture._response_events(request)
                except RuntimeError:
                    self._send(409, "application/json", b"{}")
                    return
                payload = b"".join(
                    b"data: "
                    + json.dumps(event, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
                    + b"\n\n"
                    for event in events
                ) + b"data: [DONE]\n\n"
                self._send(200, "text/event-stream", payload)

            def do_CONNECT(self) -> None:
                fixture._reject_request("blocked_connect_request")
                self._send(403, "text/plain", b"")

            def do_PUT(self) -> None:
                fixture._reject_request("unexpected_put")
                self._send(403, "text/plain", b"")

            def do_DELETE(self) -> None:
                fixture._reject_request("unexpected_delete")
                self._send(403, "text/plain", b"")

        self._server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self._server.daemon_threads = True
        self._server.timeout = 0.2
        self._lock = threading.Lock()
        self._scenario: dict[str, object] | None = None
        self._history: list[dict[str, object]] = []
        self._model_requests = 0
        self._rejections: list[str] = []
        self._diagnostics: list[dict[str, object]] = []
        self._thread: threading.Thread | None = None

    @property
    def port(self) -> int:
        return int(self._server.server_address[1])

    def start(self) -> None:
        if self._thread is not None:
            raise RuntimeError("loopback fixtureは既に起動しています")
        self._thread = threading.Thread(
            target=self._server.serve_forever,
            kwargs={"poll_interval": 0.1},
            name="d4p-agent-task-loopback-responses",
            daemon=True,
        )
        self._thread.start()

    def close(self) -> None:
        if self._thread is not None:
            self._server.shutdown()
            self._thread.join(timeout=5)
            self._thread = None
        self._server.server_close()

    def configure(self, scenario: str, command: str, workspace: Path) -> None:
        if scenario not in {"complete", "cancel", "deadline"}:
            raise ValueError("Agent Task loopback scenario is not supported")
        with self._lock:
            self._scenario = {
                "name": scenario,
                "command": command,
                "workspace": str(workspace),
                "posts": 0,
                "tool_calls": 0,
                "tool_outputs": 0,
                "tool_output_sha256": None,
                "call_id": None,
                "request_shapes": [],
            }
            self._history.append(self._scenario)

    def snapshot(self) -> dict[str, object]:
        with self._lock:
            return {
                "model_requests": self._model_requests,
                "rejections": list(self._rejections),
                "agent_task_diagnostics": copy.deepcopy(self._diagnostics),
                "scenarios": [
                    {
                        "name": row["name"],
                        "posts": row["posts"],
                        "tool_calls": row["tool_calls"],
                        "tool_outputs": row["tool_outputs"],
                        "tool_output_sha256": row["tool_output_sha256"],
                        "request_shapes": copy.deepcopy(row["request_shapes"]),
                    }
                    for row in self._history
                ],
            }

    def _record_model_request(self) -> None:
        with self._lock:
            self._model_requests += 1

    def _reject_request(self, reason: str) -> None:
        with self._lock:
            self._rejections.append(reason)

    def _record_diagnostic(self, diagnostic: dict[str, object]) -> None:
        with self._lock:
            if len(self._diagnostics) < 32:
                self._diagnostics.append(copy.deepcopy(diagnostic))
            else:
                self._rejections.append("diagnostic_limit_exceeded")

    def _response_events(self, request: dict[str, object]) -> list[dict[str, object]]:
        with self._lock:
            scenario = self._scenario
            if scenario is None:
                self._rejections.append("request_without_active_task")
                raise RuntimeError("no active Task")
            posts = int(scenario["posts"]) + 1
            scenario["posts"] = posts
            input_items = request.get("input")
            if not isinstance(input_items, list):
                self._rejections.append("responses_input_not_array")
                raise RuntimeError("invalid input")
            tool_outputs = [
                item
                for item in input_items
                if isinstance(item, dict) and item.get("type") == "function_call_output"
            ]
            tools = request.get("tools")
            offers_exec = isinstance(tools, list) and any(
                isinstance(tool, dict) and tool.get("name") == "exec_command"
                for tool in tools
            )
            cast_shapes = scenario["request_shapes"]
            assert isinstance(cast_shapes, list)
            cast_shapes.append(
                {
                    "input_items": len(input_items),
                    "function_call_output_items": len(tool_outputs),
                    "exec_command_offered": offers_exec,
                }
            )
            if posts == 1 and not tool_outputs and offers_exec:
                scenario["tool_calls"] = 1
                call_id = "call_d4p_linux_agent_task_1"
                item_id = "fc_d4p_linux_agent_task_1"
                scenario["call_id"] = call_id
                arguments = json.dumps(
                    {
                        "cmd": scenario["command"],
                        "workdir": scenario["workspace"],
                        "shell": "/bin/bash",
                        "yield_time_ms": LINUX_AGENT_TASK_INITIAL_TOOL_YIELD_MS,
                    },
                    separators=(",", ":"),
                )
                response_id = f"resp_d4p_linux_agent_task_{posts}"
                started = {
                    "type": "function_call",
                    "id": item_id,
                    "call_id": call_id,
                    "name": "exec_command",
                    "status": "in_progress",
                    "arguments": "",
                }
                completed = {**started, "status": "completed", "arguments": arguments}
                events: list[dict[str, object]] = [
                    _responses_lifecycle_event("response.created", response_id, "in_progress", []),
                    _responses_lifecycle_event("response.in_progress", response_id, "in_progress", []),
                    {
                        "type": "response.output_item.added",
                        "response_id": response_id,
                        "output_index": 0,
                        "item": started,
                    },
                    {
                        "type": "response.function_call_arguments.delta",
                        "response_id": response_id,
                        "item_id": item_id,
                        "output_index": 0,
                        "delta": arguments,
                    },
                    {
                        "type": "response.function_call_arguments.done",
                        "response_id": response_id,
                        "item_id": item_id,
                        "output_index": 0,
                        "arguments": arguments,
                    },
                    {
                        "type": "response.output_item.done",
                        "response_id": response_id,
                        "output_index": 0,
                        "item": completed,
                    },
                    _responses_lifecycle_event("response.completed", response_id, "completed", [completed]),
                ]
                return _add_responses_sequence_numbers(events)
            if posts == 2 and len(tool_outputs) == 1 and scenario.get("call_id"):
                output = tool_outputs[0]
                assert isinstance(output, dict)
                if output.get("call_id") != scenario["call_id"]:
                    self._rejections.append("tool_output_call_id_mismatch")
                    raise RuntimeError("tool output id mismatch")
                output_text = output.get("output")
                if not isinstance(output_text, str):
                    self._rejections.append("tool_output_not_text")
                    raise RuntimeError("tool output missing")
                if "D4P_SYNTHETIC_SECRET_CANARY" in output_text:
                    self._rejections.append("synthetic_secret_canary_in_tool_output")
                    raise RuntimeError("secret canary exposed")
                scenario["tool_outputs"] = 1
                scenario["tool_output_sha256"] = hashlib.sha256(
                    output_text.encode("utf-8", errors="replace")
                ).hexdigest()
                return _responses_final_message_events(posts)
            self._rejections.append("unexpected_response_request_shape")
            raise RuntimeError("unexpected request sequence")


def _fill_linux_workspace_registration(
    environment: dict[str, str],
    dialog,
    *,
    cli_fixture: Path,
    workspace_id: str,
    workspace_root: Path,
    secret_paths: str,
    runtime_id: str = "runtime-linux-workspace-acceptance",
    model_id: str = "model-linux-workspace-acceptance",
) -> None:
    window_id = (
        None
        if _is_wayland_environment(environment)
        else _visible_product_window(environment)
    )
    if window_id is None and not _is_wayland_environment(environment):
        raise RuntimeError("Workspace registration中にinstalled product windowがありません")
    for label, value in (
        ("模型識別子", model_id),
        ("実行系ID（Runtime ID）", runtime_id),
        ("Codex CLI実行fileの絶対path", str(cli_fixture)),
        ("作業領域ID（Workspace ID）", workspace_id),
        ("Workspace rootの絶対path", str(workspace_root)),
        ("除外する秘密path（相対path、1行に1件）", secret_paths),
    ):
        _set_public_text_by_keyboard(dialog, label, value, environment, window_id)


def _write_linux_agent_task_fixture(acceptance_root: Path) -> tuple[Path, Path, Path]:
    fixture_root = acceptance_root / "agent-task-fixture"
    workspace = fixture_root / "workspace"
    outside_root = fixture_root / "outside"
    for directory in (fixture_root, workspace, outside_root, workspace / "private"):
        directory.mkdir(mode=0o700, parents=True, exist_ok=False)
    (workspace / ".env").write_text(
        "D4P_SYNTHETIC_SECRET_CANARY=must-not-be-readable\n", encoding="utf-8"
    )
    (workspace / "private/token.txt").write_text(
        "D4P_SYNTHETIC_SECRET_CANARY=must-not-be-readable\n", encoding="utf-8"
    )
    outside_file = outside_root / "outside.txt"
    outside_file.write_text("D4P_SYNTHETIC_SECRET_CANARY=outside-workspace\n", encoding="utf-8")
    outside_link = workspace / "outside-link.txt"
    outside_link.symlink_to(outside_file)
    (workspace / "task_fixture_descendant.py").write_text(
        "import sys, time\n"
        "nonce = sys.argv[1]\n"
        "if not nonce.startswith('d4p-task-descendant-'):\n"
        "    raise SystemExit(4)\n"
        "while True:\n"
        "    time.sleep(300)\n",
        encoding="utf-8",
    )
    (workspace / "task_fixture.py").write_text(
        "import json, os, pathlib, subprocess, sys, time, uuid\n"
        "workspace = pathlib.Path(__file__).resolve().parent\n"
        "fixture_root = workspace.parent\n"
        "scratch = pathlib.Path(os.environ['TMPDIR']).resolve()\n"
        "scratch_workspace_bound = scratch.parent == workspace\n"
        "def read_allowed(path):\n"
        "    try:\n"
        "        path.read_text(encoding='utf-8')\n"
        "        return True\n"
        "    except OSError:\n"
        "        return False\n"
        "def write_allowed(path):\n"
        "    try:\n"
        "        fd = os.open(path, os.O_WRONLY | getattr(os, 'O_CLOEXEC', 0))\n"
        "        os.close(fd)\n"
        "        return True\n"
        "    except OSError:\n"
        "        return False\n"
        "read_denied = not any((read_allowed(workspace / '.env'), read_allowed(workspace / 'private/token.txt'), read_allowed(workspace / 'outside-link.txt'), read_allowed(fixture_root / 'outside/outside.txt')))\n"
        "write_denied = not any((write_allowed(workspace / '.env'), write_allowed(workspace / 'private/token.txt'), write_allowed(fixture_root / 'outside/outside.txt')))\n"
        "(scratch / 'fixture-scratch-marker').write_text('owned-task-scratch\\n', encoding='utf-8')\n"
        "mode = sys.argv[1]\n"
        "if mode == 'complete':\n"
        "    report = {'mode': mode, 'secret_and_outside_reads_denied': read_denied, 'secret_and_outside_writes_denied': write_denied, 'workspace_write': True, 'scratch_marker': True, 'scratch_workspace_bound': scratch_workspace_bound, 'scratch_name': scratch.name}\n"
        "    (workspace / 'agent-task-result.json').write_text(json.dumps(report, sort_keys=True) + '\\n', encoding='utf-8')\n"
        "    print('D4P_AGENT_TASK_FIXTURE_RESULT ' + json.dumps(report, sort_keys=True))\n"
        "elif mode in {'cancel', 'deadline'}:\n"
        "    nonce = 'd4p-task-descendant-' + uuid.uuid4().hex\n"
        "    child = subprocess.Popen([sys.executable, str(workspace / 'task_fixture_descendant.py'), nonce], stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)\n"
        "    report = {'mode': mode, 'secret_and_outside_reads_denied': read_denied, 'secret_and_outside_writes_denied': write_denied, 'scratch_marker': True, 'scratch_workspace_bound': scratch_workspace_bound, 'scratch_name': scratch.name, 'descendant_nonce': nonce}\n"
        "    (workspace / f'agent-task-{mode}-active.json').write_text(json.dumps(report, sort_keys=True) + '\\n', encoding='utf-8')\n"
        "    while True:\n"
        "        time.sleep(0.1)\n"
        "else:\n"
        "    raise SystemExit(3)\n",
        encoding="utf-8",
    )
    return fixture_root, workspace, outside_file


def _linux_proc_identity(pid: int) -> tuple[int, int, str] | None:
    try:
        raw = Path(f"/proc/{pid}/stat").read_text(encoding="utf-8")
    except (FileNotFoundError, ProcessLookupError, PermissionError, OSError):
        return None
    close = raw.rfind(")")
    if close < 0:
        raise RuntimeError("Linux process statの形が不正です")
    fields = raw[close + 2 :].split()
    if len(fields) <= 19:
        raise RuntimeError("Linux process statのfieldが不足しています")
    return int(fields[2]), int(fields[19]), fields[0]


def _linux_process_group_members(pgid: int) -> list[tuple[int, int, str]]:
    members: list[tuple[int, int, str]] = []
    for entry in Path("/proc").iterdir():
        if not entry.name.isascii() or not entry.name.isdigit():
            continue
        identity = _linux_proc_identity(int(entry.name))
        if identity is not None and identity[0] == pgid and identity[2] != "Z":
            members.append((int(entry.name), identity[1], identity[2]))
    return members


def _linux_processes_with_task_nonce(nonce: str) -> list[tuple[int, int, int]]:
    if not re.fullmatch(r"d4p-task-descendant-[a-f0-9]{32}", nonce):
        raise ValueError("Agent Task descendant nonceが不正です")
    encoded_nonce = nonce.encode("ascii")
    matches: list[tuple[int, int, int]] = []
    for entry in Path("/proc").iterdir():
        if not entry.name.isascii() or not entry.name.isdigit():
            continue
        try:
            pid = int(entry.name)
            if encoded_nonce not in (entry / "cmdline").read_bytes().split(b"\0"):
                continue
            identity = _linux_proc_identity(pid)
            if identity is None or identity[2] == "Z":
                continue
            confirmed = (entry / "cmdline").read_bytes().split(b"\0")
            current_identity = _linux_proc_identity(pid)
        except OSError:
            continue
        if (
            encoded_nonce in confirmed
            and current_identity == identity
            and identity is not None
        ):
            matches.append((pid, identity[0], identity[1]))
    return matches


def _wait_for_linux_process_group_exit(
    pgid: int, *, timeout: float = 12.0
) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if not _linux_process_group_members(pgid):
            return
        time.sleep(0.1)
    raise RuntimeError("Agent Task終了後も所有process groupのlive processが残っています")


def _stop_linux_agent_task_process_group(
    pgid: int, anchor_pid: int, anchor_start_time: int, *, timeout: float = 3.0
) -> None:
    if (
        pgid <= 1
        or anchor_pid <= 1
        or anchor_start_time <= 0
        or pgid == os.getpgrp()
    ):
        raise RuntimeError("試験runner自身または不正なAgent Task process groupは停止しません")
    if not _linux_process_group_members(pgid):
        return
    identity = _linux_proc_identity(anchor_pid)
    if identity is None or (identity[0], identity[1]) != (pgid, anchor_start_time):
        raise RuntimeError("Agent Task process groupの所有identityを再確認できず停止を拒否しました")
    try:
        os.killpg(pgid, signal.SIGTERM)
    except ProcessLookupError:
        if not _linux_process_group_members(pgid):
            return
        raise RuntimeError("Agent Task process groupへのTERM送信を確認できません")
    try:
        _wait_for_linux_process_group_exit(pgid, timeout=timeout)
        return
    except RuntimeError:
        if not _linux_process_group_members(pgid):
            return
    try:
        os.killpg(pgid, signal.SIGKILL)
    except ProcessLookupError:
        if not _linux_process_group_members(pgid):
            return
        raise RuntimeError("Agent Task process groupへのKILL送信を確認できません")
    _wait_for_linux_process_group_exit(pgid, timeout=timeout)


def _linux_agent_task_cleanup_failure_code(error: Exception) -> str:
    safe_messages = {
        "試験runner自身または不正なAgent Task process groupは停止しません": "unsafe_process_group",
        "Agent Task process groupの所有identityを再確認できず停止を拒否しました": "identity_mismatch",
        "Agent Task process groupへのTERM送信を確認できません": "term_unconfirmed",
        "Agent Task process groupへのKILL送信を確認できません": "kill_unconfirmed",
        "Agent Task終了後も所有process groupのlive processが残っています": "live_process_remains",
        "Agent Task cleanup rootがsymlinkです": "acceptance_root_symlink",
        "Agent Task cleanup fixture rootがsymlinkです": "fixture_root_symlink",
        "Agent Task cleanup Workspaceがsymlinkです": "workspace_symlink",
        "Agent Task cleanup Workspaceがdirectoryではありません": "workspace_not_directory",
        "Agent Task cleanup markerがsymlinkです": "marker_symlink",
        "Agent Task cleanup markerが通常fileまたはsize上限に違反します": "marker_shape_invalid",
    }
    known_message = safe_messages.get(str(error))
    if known_message is not None:
        return known_message
    if isinstance(error, (json.JSONDecodeError, UnicodeError, KeyError, TypeError, ValueError)):
        return "marker_identity_invalid"
    if isinstance(error, OSError):
        return f"os_error_{error.errno}" if isinstance(error.errno, int) else "os_error"
    return "cleanup_internal_error"


def _cleanup_linux_agent_task_process_groups(acceptance_root: Path) -> None:
    if acceptance_root.is_symlink():
        raise RuntimeError("Agent Task cleanup rootがsymlinkです")
    fixture_root = acceptance_root / "agent-task-fixture"
    if fixture_root.is_symlink():
        raise RuntimeError("Agent Task cleanup fixture rootがsymlinkです")
    if not fixture_root.exists():
        return
    workspace = fixture_root / "workspace"
    if workspace.is_symlink():
        raise RuntimeError("Agent Task cleanup Workspaceがsymlinkです")
    if not workspace.exists():
        return
    if not workspace.is_dir():
        raise RuntimeError("Agent Task cleanup Workspaceがdirectoryではありません")
    first_failure: dict[str, str] | None = None
    first_failure_cause: Exception | None = None
    for scenario in ("cancel", "deadline"):
        try:
            marker = workspace / f"agent-task-{scenario}-active.json"
            if marker.is_symlink():
                raise RuntimeError("Agent Task cleanup markerがsymlinkです")
            if not marker.exists():
                continue
            if not marker.is_file() or marker.stat().st_size > 4096:
                raise RuntimeError(
                    "Agent Task cleanup markerが通常fileまたはsize上限に違反します"
                )
            value = json.loads(marker.read_text(encoding="utf-8"))
            if not isinstance(value, dict) or value.get("mode") != scenario:
                raise ValueError
            nonce = value["descendant_nonce"]
            if not isinstance(nonce, str):
                raise ValueError
            owned_groups: dict[int, tuple[int, int, int]] = {}
            for pid, pgid, start_time in _linux_processes_with_task_nonce(nonce):
                owned_groups.setdefault(pgid, (pid, pgid, start_time))
            group_failure: Exception | None = None
            for pid, pgid, start_time in owned_groups.values():
                try:
                    _stop_linux_agent_task_process_group(pgid, pid, start_time)
                except Exception as error:
                    if group_failure is None:
                        group_failure = error
            if group_failure is not None:
                raise group_failure
        except Exception as error:
            if first_failure is None:
                first_failure = {
                    "scenario": scenario,
                    "error_type": type(error).__name__,
                    "failure_code": _linux_agent_task_cleanup_failure_code(error),
                }
                first_failure_cause = error
    if first_failure is not None:
        raise RuntimeError(
            "Agent Task process group cleanupで一つ以上の失敗を確認しました: "
            + json.dumps(first_failure, ensure_ascii=False, sort_keys=True)
        ) from first_failure_cause


def _read_task_scratch_recovery_state(store: Path) -> dict[str, object]:
    path = store / "agent_task_scratch_recovery.json"
    if path.is_symlink() or not path.is_file():
        raise RuntimeError("WorkspaceTaskScratch recovery journalが通常fileとしてありません")
    try:
        wrapper = json.loads(path.read_text(encoding="utf-8"))
        state = wrapper["state"]
        entries = state["entries"]
    except (OSError, UnicodeError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise RuntimeError("WorkspaceTaskScratch recovery journalを解析できません") from error
    if wrapper.get("version") != 1 or state.get("version") != 1 or not isinstance(entries, list):
        raise RuntimeError("WorkspaceTaskScratch recovery journalのversionまたは形が不正です")
    if entries:
        raise RuntimeError("終端TaskのWorkspaceTaskScratch recovery journalに残存entryがあります")
    hmac_value = wrapper.get("hmac")
    key_path = store / "audit_anchor.key"
    if (
        not isinstance(hmac_value, str)
        or not key_path.is_file()
        or key_path.is_symlink()
    ):
        raise RuntimeError("WorkspaceTaskScratch recovery journalのHMAC keyまたはHMACがありません")
    try:
        key = bytes.fromhex(key_path.read_text(encoding="ascii").strip())
    except (OSError, UnicodeError, ValueError) as error:
        raise RuntimeError("WorkspaceTaskScratch recovery journalのHMAC keyが不正です") from error
    if len(key) != 32:
        raise RuntimeError("WorkspaceTaskScratch recovery journalのHMAC key長が不正です")
    normalized_state = json.dumps(
        state, ensure_ascii=False, sort_keys=True, separators=(",", ":")
    )
    expected_hmac = "sha256:" + hmac.new(
        key,
        f"version=1|state={normalized_state}".encode("utf-8"),
        hashlib.sha256,
    ).hexdigest()
    if not hmac.compare_digest(hmac_value, expected_hmac):
        raise RuntimeError("WorkspaceTaskScratch recovery journalのHMACが一致しません")
    return {"entries": len(entries), "hmac_present": True}


def _linux_installed_task_status_observation(
    task_id: str,
    expected_status: str,
    *,
    deadline: float,
    accessibility_root=None,
) -> tuple[str | None, str | None, list[str]]:
    try:
        import pyatspi
    except ImportError as error:
        raise RuntimeError("AT-SPI UI操作にpython3-pyatspiが必要です") from error

    task_id_label = f"Task識別子: {task_id}"
    visible_statuses: list[str] = []
    current_task_status: str | None = None
    desktop = (
        accessibility_root
        if accessibility_root is not None
        else pyatspi.Registry.getDesktop(0)
    )
    status_nodes: list[tuple[object, str]] = []
    for node, name in _accessible_nodes(desktop, deadline=deadline):
        # GTK may expose a visible label through AT-SPI Text while Accessible.name
        # remains empty. Read Text for nodes whose accessible name does not already
        # carry one of the Task correlation fields.
        content = name
        if not any(
            marker in name
            for marker in ("Broker Task状態", "Task識別子:", "Task状態:")
        ):
            text = _accessible_text(node)
            if text and text != name:
                content = f"{name}\n{text}" if name else text
        match = re.search(
            r"Task状態:\s*(pending|running|blocked|completed|failed|cancelled|quarantined)\b",
            content,
        )
        if match is not None:
            status_nodes.append((node, match.group(1)))
            status = match.group(1)
            if status not in visible_statuses:
                visible_statuses.append(status)
                visible_statuses[:] = visible_statuses[-6:]
        if "Broker Task状態" in content:
            if time.monotonic() >= deadline:
                return None, current_task_status, visible_statuses
            snapshot = _accessible_snapshot(node, deadline=deadline)
            if time.monotonic() >= deadline:
                return None, current_task_status, visible_statuses
            scope_task_ids = set(
                re.findall(r"Task識別子:\s*([A-Za-z0-9_.:-]+)", snapshot)
            )
            if scope_task_ids == {task_id} and task_id_label in snapshot:
                task_status_match = re.search(
                    r"Task状態:\s*(pending|running|blocked|completed|failed|cancelled|quarantined)\b",
                    snapshot,
                )
                if task_status_match is None:
                    return None, current_task_status, visible_statuses
                current_task_status = task_status_match.group(1)
                if current_task_status not in visible_statuses:
                    visible_statuses.append(current_task_status)
                    visible_statuses[:] = visible_statuses[-6:]
                if current_task_status == expected_status:
                    return snapshot, current_task_status, visible_statuses
                return None, current_task_status, visible_statuses

    for node, observed_status in status_nodes:
        if time.monotonic() >= deadline:
            return None, current_task_status, visible_statuses
        scope = node
        task_scope = None
        for _ in range(25):
            if time.monotonic() >= deadline:
                return None, current_task_status, visible_statuses
            scope_text = _accessible_snapshot(scope, deadline=deadline)
            if time.monotonic() >= deadline:
                return None, current_task_status, visible_statuses
            scope_task_ids = set(
                re.findall(r"Task識別子:\s*([A-Za-z0-9_.:-]+)", scope_text)
            )
            if scope_task_ids:
                if scope_task_ids == {task_id} and task_id_label in scope_text:
                    task_scope = scope
                break
            parent = _accessible_parent(scope)
            if parent is None or parent is scope:
                break
            scope = parent
        if task_scope is None:
            continue
        snapshot = _accessible_snapshot(task_scope, deadline=deadline)
        if time.monotonic() >= deadline:
            return None, current_task_status, visible_statuses
        snapshot_task_ids = set(
            re.findall(r"Task識別子:\s*([A-Za-z0-9_.:-]+)", snapshot)
        )
        if snapshot_task_ids != {task_id} or task_id_label not in snapshot:
            continue
        task_status_match = re.search(
            r"Task状態:\s*(pending|running|blocked|completed|failed|cancelled|quarantined)\b",
            snapshot,
        )
        if task_status_match is None:
            continue
        current_task_status = task_status_match.group(1)
        if current_task_status == expected_status:
            return snapshot, current_task_status, visible_statuses
        if current_task_status != observed_status:
            return None, current_task_status, visible_statuses
    return None, current_task_status, visible_statuses


def _wait_installed_task_status(
    environment: dict[str, str],
    window,
    expected_status: str,
    *,
    task_id: str,
    timeout: float,
) -> str:
    deadline = time.monotonic() + timeout
    last_snapshot = ""
    visible_statuses: list[str] = []
    current_task_status: str | None = None
    window_lookup_failures = 0
    refresh_control_seen = False
    refresh_attempts = 0
    while time.monotonic() < deadline:
        remaining = max(0.01, deadline - time.monotonic())
        try:
            current_window = _find_accessible(
                WINDOW_TITLE, timeout=min(0.2, remaining)
            )
            window = current_window
        except RuntimeError:
            window_lookup_failures += 1
        task_snapshot, current_task_status, observed_statuses = (
            _linux_installed_task_status_observation(
                task_id,
                expected_status,
                deadline=min(deadline, time.monotonic() + min(0.2, remaining)),
                accessibility_root=window,
            )
        )
        visible_statuses.extend(observed_statuses)
        visible_statuses[:] = list(dict.fromkeys(visible_statuses))[-6:]
        if task_snapshot is not None:
            return task_snapshot
        last_snapshot = _accessible_snapshot(window)
        visible_statuses.extend(
            re.findall(
                r"Task状態:\s*(pending|running|blocked|completed|failed|cancelled|quarantined)\b",
                last_snapshot,
            )
        )
        visible_statuses[:] = list(dict.fromkeys(visible_statuses))[-6:]
        try:
            refresh = _find_operable_accessible(
                "Task状態を更新",
                root=window,
                timeout=min(0.2, max(0.01, deadline - time.monotonic())),
            )
            refresh_control_seen = True
            refresh_attempts += 1
            _activate_accessible(refresh, "Task状態を更新", environment)
        except RuntimeError:
            pass
        time.sleep(0.25)
    diagnostics = {
        "visible_task_statuses": visible_statuses,
        "current_task_status": current_task_status,
        "task_id_was_visible_in_last_snapshot": f"Task識別子: {task_id}" in last_snapshot,
        "product_window_lookup_failures": window_lookup_failures,
        "last_snapshot_nonempty": bool(last_snapshot),
        "refresh_control_seen": refresh_control_seen,
        "refresh_attempts": refresh_attempts,
    }
    raise RuntimeError(
        f"installed Agent Taskが{expected_status}へ遷移しませんでした: "
        + json.dumps(diagnostics, ensure_ascii=False, sort_keys=True)
    )


def _linux_installed_visible_task_ids(*, deadline: float) -> set[str]:
    try:
        import pyatspi
    except ImportError as error:
        raise RuntimeError("AT-SPI UI操作にpython3-pyatspiが必要です") from error

    task_ids: set[str] = set()
    desktop = pyatspi.Registry.getDesktop(0)
    for node, name in _accessible_nodes(desktop, deadline=deadline, max_depth=20):
        for value in (name, _accessible_text(node)):
            task_ids.update(
                re.findall(r"Task識別子:\s*([A-Za-z0-9_.:-]+)", value)
            )
    return task_ids


def _wait_for_installed_task_id(
    window, *, previously_visible_task_ids: set[str], timeout: float
) -> str:
    deadline = time.monotonic() + timeout
    last_statuses: list[str] = []
    last_new_task_count = 0
    observation_errors = 0
    while time.monotonic() < deadline:
        current_task_ids: set[str] = set()
        try:
            current_task_ids.update(
                _linux_installed_visible_task_ids(
                    deadline=min(deadline, time.monotonic() + 1.0)
                )
            )
        except RuntimeError:
            observation_errors += 1
        snapshot = _accessible_snapshot(window)
        current_task_ids.update(
            re.findall(r"Task識別子:\s*([A-Za-z0-9_.:-]+)", snapshot)
        )
        new_task_ids = current_task_ids - previously_visible_task_ids
        last_new_task_count = len(new_task_ids)
        if len(new_task_ids) == 1:
            return next(iter(new_task_ids))
        if len(new_task_ids) > 1:
            raise RuntimeError(
                "installed Agent Centerで新規Task IDを一意に特定できません: "
                + json.dumps(
                    {"new_task_count": len(new_task_ids)}, ensure_ascii=False
                )
            )
        last_statuses = re.findall(
            r"Task状態:\s*(pending|running|blocked|completed|failed|cancelled|quarantined)\b",
            snapshot,
        )[-4:]
        time.sleep(0.1)
    raise RuntimeError(
        "installed Agent Centerに開始Task IDが表示されません: "
        + json.dumps(
            {
                "visible_task_statuses": last_statuses,
                "new_task_count": last_new_task_count,
                "observation_errors": observation_errors,
            },
            ensure_ascii=False,
            sort_keys=True,
        )
    )


def _wait_for_installed_task_result_or_terminal(
    environment: dict[str, str], window, result_file: Path, *, timeout: float
) -> str:
    terminal_statuses = {"completed", "failed", "cancelled", "quarantined"}
    deadline = time.monotonic() + timeout
    next_refresh = 0.0
    while time.monotonic() < deadline:
        if result_file.is_symlink():
            return "result_symlink"
        if result_file.is_file():
            return "result_present"
        now = time.monotonic()
        if now >= next_refresh:
            try:
                refresh = _find_operable_accessible(
                    "Task状態を更新", root=window, timeout=1
                )
                _activate_accessible(refresh, "Task状態を更新", environment)
            except RuntimeError:
                pass
            next_refresh = now + 1.0
            snapshot = _accessible_snapshot(window)
            statuses = re.findall(
                r"Task状態:\s*(pending|running|blocked|completed|failed|cancelled|quarantined)\b",
                snapshot,
            )
            if statuses and statuses[-1] in terminal_statuses:
                return statuses[-1]
        time.sleep(0.1)
    return "timeout"


def _installed_task_history_diagnostics(store: Path, task_id: str) -> dict[str, object]:
    audit_path = store / "audit.jsonl"
    terminal_statuses = {"completed", "failed", "cancelled", "quarantined"}
    try:
        if audit_path.is_symlink() or not audit_path.is_file():
            return {"state": "absent", "terminal_records": []}
        if audit_path.stat().st_size > 8 * 1024 * 1024:
            return {"state": "over_limit", "terminal_records": []}
        lines = audit_path.read_text(encoding="utf-8").splitlines()
    except (OSError, UnicodeError):
        return {"state": "unreadable", "terminal_records": []}

    failure_classes = {
        "要求不正",
        "実行系不在",
        "権限拒否",
        "セッション不一致",
        "通信失敗",
        "期限超過",
        "応答不正",
        "監査失敗",
        "取消",
        "実行系隔離済み",
        "作業領域不在",
        "AgentTask実行非対応",
    }
    terminal_records: list[dict[str, str]] = []
    for line in lines:
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            return {"state": "invalid", "terminal_records": []}
        if not isinstance(event, dict) or event.get("request_id") != task_id:
            continue
        operation = event.get("operation")
        if not isinstance(operation, str) or operation not in {
            "AgentTask実行",
            "AgentTask状態",
        }:
            continue
        reason = event.get("reason")
        if not isinstance(reason, str) or not reason.startswith("AgentTask履歴:"):
            continue
        try:
            record = json.loads(reason.removeprefix("AgentTask履歴:"))
        except json.JSONDecodeError:
            return {"state": "invalid", "terminal_records": []}
        if (
            not isinstance(record, dict)
            or record.get("task_id") != task_id
            or record.get("stage") != "terminal"
        ):
            continue
        status = record.get("status")
        failure_class = record.get("failure_class")
        terminal_records.append(
            {
                "status": (
                    status
                    if isinstance(status, str) and status in terminal_statuses
                    else "unknown"
                ),
                "failure_class": (
                    failure_class
                    if isinstance(failure_class, str)
                    and failure_class in failure_classes
                    else "unknown"
                ),
            }
        )
    return {"state": "read", "terminal_records": terminal_records[-1:]}


def _installed_task_write_failure_diagnostics(
    window,
    task_id: str,
    result_file: Path,
    workspace: Path,
    responses_server: _LinuxAgentTaskResponsesServer,
    audit_store: Path,
) -> dict[str, object]:
    snapshot = _accessible_snapshot(window)
    loopback = responses_server.snapshot()
    statuses = re.findall(
        r"Task状態:\s*(pending|running|blocked|completed|failed|cancelled|quarantined)\b",
        snapshot,
    )
    task_ids = re.findall(r"Task識別子:\s*([A-Za-z0-9_.:-]+)", snapshot)
    scratch_paths = [
        path
        for path in workspace.glob(".d4p-tmp-*")
        if path.is_dir() and not path.is_symlink()
    ]
    return {
        "visible_task_statuses": statuses[-4:],
        "requested_task_visible": task_id in task_ids,
        "result_file_exists": result_file.is_file() and not result_file.is_symlink(),
        "result_file_is_symlink": result_file.is_symlink(),
        "scratch_directory_count": len(scratch_paths),
        "scratch_marker_count": sum(
            (path / "fixture-scratch-marker").is_file() for path in scratch_paths
        ),
        "loopback": loopback,
        "task_history": _installed_task_history_diagnostics(audit_store, task_id),
        "runtime_failure_diagnostics": {
            "state": "read",
            "records": loopback["agent_task_diagnostics"],
        },
    }


def _prepare_and_start_installed_agent_task(
    environment: dict[str, str],
    window,
    *,
    runtime_id: str,
    workspace_id: str,
    instruction: str,
) -> None:
    inspect = _find_operable_accessible(
        "Task実行能力を事前検査（実行なし）", root=window, timeout=15
    )
    _activate_accessible(inspect, "Task実行能力を事前検査（実行なし）", environment)
    dialog_title = _find_accessible("Agent Task能力の事前検査", timeout=10)
    dialog = _accessible_scope_with_named_descendant(
        dialog_title, "Task指示", deadline=time.monotonic() + 10
    )
    window_id = _visible_product_window(environment)
    if window_id is None:
        raise RuntimeError("Task instruction dialog中にinstalled product windowがありません")
    _set_public_text_by_keyboard(dialog, "Task指示", instruction, environment, window_id)
    submit = _find_operable_accessible("Broker事前検査", root=dialog, timeout=10)
    _activate_accessible(submit, "Broker事前検査", environment)
    _find_accessible("Broker事前検査済み", root=window, timeout=15)

    permission = _find_operable_accessible(
        "Workspace PermissionのOwner確認", root=window, timeout=10
    )
    _activate_accessible(permission, "Workspace PermissionのOwner確認", environment)
    _answer_owner_dialog(
        environment,
        "許可",
        context="Agent Task Workspace Permission",
        expected_summary_markers=(
            "このAgent Sessionに限定した、一回限りのTask用Workspace Permissionを発行しますか？",
            f"Runtime ID: {runtime_id}",
            f"Workspace ID: {workspace_id}",
            "許可operation: agent_task.execute",
            "Task本文と実行条件を示す別個のOwner Approvalは必要です",
        ),
    )
    _wait_for_owner_dialog_closed(environment)

    approval = _find_operable_accessible(
        "Task一回ApprovalのOwner確認", root=window, timeout=10
    )
    _activate_accessible(approval, "Task一回ApprovalのOwner確認", environment)
    _answer_owner_dialog(
        environment,
        "許可",
        context="Agent Task一回Owner Approval",
        expected_summary_markers=(
            "このAgent Taskに一回限りのOwner Approvalを発行しますか？",
            f"Runtime ID: {runtime_id}",
            "Session ID:",
            f"Workspace ID: {workspace_id}",
            "指示hash:",
            "実行上限: 最大15分",
            "Approvalは本文hashと現在の実行条件に束縛され",
        ),
    )
    _wait_for_owner_dialog_closed(environment)
    start = _find_operable_accessible("Taskを一回実行", root=window, timeout=10)
    _activate_accessible(start, "Taskを一回実行", environment)


def _run_installed_agent_task_acceptance(
    environment: dict[str, str],
    window_id: str,
    *,
    codex_cli: Path,
    responses_server: _LinuxAgentTaskResponsesServer,
    audit_store: Path,
    acceptance_root: Path,
    task_execution_limit_ms: int,
) -> dict[str, object]:
    if not 0 < task_execution_limit_ms < LINUX_AGENT_TASK_INITIAL_TOOL_YIELD_MS:
        raise RuntimeError(
            "deadline acceptanceにはAgent初回tool待機より短いBroker期限が必要です"
        )
    fixture_root, workspace, outside_file = _write_linux_agent_task_fixture(acceptance_root)
    workspace_id = "workspace-linux-installed-agent-task"
    runtime_id = "runtime-linux-installed-agent-task"
    secret_paths = ".env\nprivate/token.txt"
    registration = _open_installed_codex_registration(environment, window_id=window_id)
    _fill_linux_workspace_registration(
        environment,
        registration,
        cli_fixture=codex_cli,
        workspace_id=workspace_id,
        workspace_root=workspace,
        secret_paths=secret_paths,
        runtime_id=runtime_id,
        # Real Codex CLI receives this value as --model; it must exist in the loopback catalog.
        model_id=LINUX_AGENT_TASK_FIXTURE_MODEL_ID,
    )
    continue_button = _find_operable_accessible(
        "native Owner確認へ進む", root=registration, timeout=10
    )
    _activate_accessible(continue_button, "native Owner確認へ進む", environment)
    _answer_owner_dialog(
        environment,
        "許可",
        context="Linux installed Codex Runtime／Workspace registration",
        expected_summary_markers=(
            "Agent CLI実行系とWorkspaceを、このBroker起動中だけ登録します。",
            f"Runtime: {runtime_id}",
            f"CLI実行file: {codex_cli}",
            "認証方式: Codex CLI管理設定（登録時は認証しません）",
            "Permission・Approval・Trust・Credentialを生成しません。",
            "LinuxのWorkspace rootがBroker内部の保護領域と重ならないことを確認します。",
            f"Workspace ID: {workspace_id}",
            f"Workspace root: {workspace}",
        ),
    )
    _wait_for_owner_dialog_closed(environment)
    window = _find_accessible(WINDOW_TITLE, timeout=10)
    _find_accessible("Task実行能力: supported", root=window, timeout=20)
    session_start = _find_operable_accessible(
        "登録Workspaceで対話Sessionを開始", root=window, timeout=10
    )
    _activate_accessible(session_start, "登録Workspaceで対話Sessionを開始", environment)
    _find_accessible("Task実行能力を事前検査（実行なし）", root=window, timeout=20)

    scenarios = (
        ("complete", "completed", 90.0),
        ("cancel", "cancelled", 30.0),
        ("deadline", "failed", task_execution_limit_ms / 1000 + 30.0),
    )
    task_ids: list[str] = []
    completed_reports: list[dict[str, object]] = []
    process_groups: list[int] = []
    positive_result_hash: str | None = None
    for scenario_name, expected_status, wait_seconds in scenarios:
        command = f"/usr/bin/python3 {workspace / 'task_fixture.py'} {scenario_name}"
        responses_server.configure(scenario_name, command, workspace)
        instruction = (
            f"D4 Pocket installed Linux acceptance fixture: execute the provided "
            f"workspace task_fixture.py in {scenario_name} mode exactly once. "
            "Do not use network access or credentials."
        )
        previously_visible_task_ids = _linux_installed_visible_task_ids(
            deadline=time.monotonic() + 5.0
        )
        previously_visible_task_ids.update(task_ids)
        _prepare_and_start_installed_agent_task(
            environment,
            window,
            runtime_id=runtime_id,
            workspace_id=workspace_id,
            instruction=instruction,
        )
        task_ids.append(
            _wait_for_installed_task_id(
                window,
                previously_visible_task_ids=previously_visible_task_ids,
                timeout=15.0,
            )
        )

        active_marker = workspace / f"agent-task-{scenario_name}-active.json"
        result_file = workspace / "agent-task-result.json"
        if scenario_name == "complete":
            wait_outcome = _wait_for_installed_task_result_or_terminal(
                environment, window, result_file, timeout=wait_seconds
            )
            if result_file.is_symlink() or not result_file.is_file():
                diagnostics = _installed_task_write_failure_diagnostics(
                    window,
                    task_ids[-1],
                    result_file,
                    workspace,
                    responses_server,
                    audit_store,
                )
                diagnostics["wait_outcome"] = wait_outcome
                raise RuntimeError(
                    "installed Agent Taskが許可Workspaceへ結果fileを書き込みませんでした: "
                    + json.dumps(diagnostics, ensure_ascii=False, sort_keys=True)
                )
            try:
                report = json.loads(result_file.read_text(encoding="utf-8"))
            except (OSError, UnicodeError, json.JSONDecodeError) as error:
                raise RuntimeError("Agent Task fixture結果がJSONとして読めません") from error
            expected_report_fields = {
                "mode",
                "secret_and_outside_reads_denied",
                "secret_and_outside_writes_denied",
                "workspace_write",
                "scratch_marker",
                "scratch_workspace_bound",
                "scratch_name",
            }
            report_is_object = isinstance(report, dict)
            report_fields_match = report_is_object and set(report) == expected_report_fields
            failed_report_checks = (
                [
                    name
                    for name in (
                        "secret_and_outside_reads_denied",
                        "secret_and_outside_writes_denied",
                        "workspace_write",
                        "scratch_marker",
                        "scratch_workspace_bound",
                    )
                    if report.get(name) is not True
                ]
                if report_is_object
                else ["report_not_object"]
            )
            mode_matches = report_is_object and report.get("mode") == "complete"
            scratch_name_matches = (
                report_is_object
                and isinstance(report.get("scratch_name"), str)
                and report["scratch_name"].startswith(".d4p-tmp-")
            )
            if (
                not report_fields_match
                or failed_report_checks
                or not mode_matches
                or not scratch_name_matches
            ):
                report_diagnostics = {
                    "report_fields_match": report_fields_match,
                    "missing_fields": (
                        sorted(expected_report_fields - set(report))
                        if report_is_object
                        else []
                    ),
                    "unexpected_fields": (
                        sorted(set(report) - expected_report_fields)
                        if report_is_object
                        else []
                    ),
                    "failed_checks": failed_report_checks,
                    "mode_matches": mode_matches,
                    "scratch_name_matches": scratch_name_matches,
                }
                raise RuntimeError(
                    "Agent Taskのsecret／Workspace境界またはscratch fixtureが不成立です: "
                    + json.dumps(report_diagnostics, ensure_ascii=False, sort_keys=True)
                )
            completed_snapshot = _wait_installed_task_status(
                environment,
                window,
                expected_status,
                task_id=task_ids[-1],
                timeout=wait_seconds,
            )
            result_hash_match = re.search(
                r"結果hash:\s*(sha256:[0-9a-f]{64})", completed_snapshot
            )
            if result_hash_match is None:
                raise RuntimeError("completed Agent Taskに結果hashがありません")
            positive_result_hash = result_hash_match.group(1)
            completed_reports.append(report)
            if list(workspace.glob(".d4p-tmp-*")):
                raise RuntimeError("positive Task終了後にWorkspaceTaskScratchが残っています")
            _read_task_scratch_recovery_state(audit_store)
        else:
            deadline = time.monotonic() + 25
            while not active_marker.is_file() and time.monotonic() < deadline:
                time.sleep(0.1)
            if active_marker.is_symlink() or not active_marker.is_file():
                raise RuntimeError(f"installed Agent Taskの{scenario_name} process markerがありません")
            try:
                active = json.loads(active_marker.read_text(encoding="utf-8"))
                nonce = active["descendant_nonce"]
            except (OSError, UnicodeError, KeyError, TypeError, ValueError, json.JSONDecodeError) as error:
                raise RuntimeError("Agent Task descendant identity markerが不正です") from error
            if (
                active.get("mode") != scenario_name
                or active.get("secret_and_outside_reads_denied") is not True
                or active.get("secret_and_outside_writes_denied") is not True
                or active.get("scratch_workspace_bound") is not True
                or not isinstance(active.get("scratch_name"), str)
                or not active["scratch_name"].startswith(".d4p-tmp-")
                or not isinstance(nonce, str)
                or not re.fullmatch(r"d4p-task-descendant-[a-f0-9]{32}", nonce)
            ):
                raise RuntimeError("active Agent Taskのsecret／Workspace境界が不成立です")
            descendant_matches: list[tuple[int, int, int]] = []
            descendant_deadline = time.monotonic() + 5
            while time.monotonic() < descendant_deadline:
                descendant_matches = _linux_processes_with_task_nonce(nonce)
                if descendant_matches:
                    break
                time.sleep(0.05)
            if len(descendant_matches) != 1:
                raise RuntimeError(
                    "Agent Task descendant nonceからhost process identityを一意に特定できません"
                )
            pid, pgid, start_time = descendant_matches[0]
            if pgid == os.getpgrp():
                raise RuntimeError("Agent Task process groupが受入れrunner自身と共有されています")
            if not any(
                member[0] == pid and member[1] == start_time
                for member in _linux_process_group_members(pgid)
            ):
                raise RuntimeError("Agent Task descendantが監督対象process groupで実行中ではありません")
            scratch_candidates = [
                path
                for path in workspace.glob(".d4p-tmp-*")
                if path.is_dir() and not path.is_symlink()
            ]
            if len(scratch_candidates) != 1:
                raise RuntimeError("active Taskに束縛されたWorkspaceTaskScratchが一つではありません")
            scratch_path = scratch_candidates[0]
            if scratch_path.name != active["scratch_name"]:
                raise RuntimeError("active TaskのTMPDIRとWorkspaceTaskScratchが一致しません")
            if not (scratch_path / "fixture-scratch-marker").is_file():
                raise RuntimeError("active Agent TaskのTMPDIRがWorkspaceTaskScratchへ結合していません")
            if scenario_name == "cancel":
                cancel = _find_operable_accessible("Task停止を要求", root=window, timeout=10)
                _activate_accessible(cancel, "Task停止を要求", environment)
                _find_accessible("取消要求を記録しました", root=window, timeout=10)
            else:
                # The Broker owns the deadline; wait for its terminal transition without sending Cancel.
                try:
                    _wait_installed_task_status(
                        environment,
                        window,
                        expected_status,
                        task_id=task_ids[-1],
                        timeout=wait_seconds,
                    )
                except RuntimeError as error:
                    raise RuntimeError(
                        "installed deadline Taskの終端状態がAcceptanceと一致しません: "
                        + json.dumps(
                            {
                                "status_wait": str(error),
                                "responses_fixture": responses_server.snapshot(),
                                "task_history": _installed_task_history_diagnostics(
                                    audit_store, task_ids[-1]
                                ),
                            },
                            ensure_ascii=False,
                            sort_keys=True,
                        )
                    ) from None
            _wait_installed_task_status(
                environment,
                window,
                expected_status,
                task_id=task_ids[-1],
                timeout=wait_seconds if scenario_name == "cancel" else 2.0,
            )
            current_identity = _linux_proc_identity(pid)
            if (
                current_identity is not None
                and current_identity[1] == start_time
                and current_identity[2] != "Z"
            ):
                raise RuntimeError("Task terminal化後も同一descendant processが残っています")
            process_groups.append(pgid)
            _wait_for_linux_process_group_exit(pgid)
            if scratch_path.exists() or scratch_path.is_symlink():
                raise RuntimeError(f"{scenario_name}終了後にWorkspaceTaskScratchが残っています")
            _read_task_scratch_recovery_state(audit_store)

    if positive_result_hash is None:
        raise RuntimeError("positive Agent Taskの結果hashがAgent Centerに表示されません")
    if len(task_ids) != 3 or len(set(task_ids)) != 3:
        raise RuntimeError("各Agent Taskが独立した一意Task IDを持っていません")
    audit = _verify_live_audit(
        audit_store,
        require_export=False,
        require_agent_task=True,
    )
    scratch_state = _read_task_scratch_recovery_state(audit_store)
    loopback = responses_server.snapshot()
    expected_scenarios = {
        "complete": (2, 1, 1),
        "cancel": (1, 1, 0),
        "deadline": (1, 1, 0),
    }
    observed_scenarios = loopback.get("scenarios")
    if not isinstance(observed_scenarios, list) or len(observed_scenarios) != 3:
        raise RuntimeError("loopback Responses fixtureのTask数が期待値と一致しません")
    for row in observed_scenarios:
        if not isinstance(row, dict) or row.get("name") not in expected_scenarios:
            raise RuntimeError("loopback Responses fixtureに未知のTaskがあります")
        expected_counts = expected_scenarios[row["name"]]
        actual_counts = (row.get("posts"), row.get("tool_calls"), row.get("tool_outputs"))
        if actual_counts != expected_counts:
            raise RuntimeError("loopback Responses fixtureの呼出し・tool結果数が不正です")
    if loopback.get("rejections"):
        raise RuntimeError("loopback Responses fixtureが不正・外部要求を観測しました")
    if {row["name"] for row in observed_scenarios} != set(expected_scenarios):
        raise RuntimeError("loopback Responses fixtureで必須Task scenarioが揃っていません")
    if outside_file.is_symlink() or outside_file.read_bytes() != b"D4P_SYNTHETIC_SECRET_CANARY=outside-workspace\n":
        raise RuntimeError("Workspace外のsynthetic fileが変更または置換されました")
    for path, expected in (
        (workspace / ".env", b"D4P_SYNTHETIC_SECRET_CANARY=must-not-be-readable\n"),
        (workspace / "private/token.txt", b"D4P_SYNTHETIC_SECRET_CANARY=must-not-be-readable\n"),
    ):
        if path.is_symlink() or path.read_bytes() != expected:
            raise RuntimeError("registered secret fixtureがTaskから変更されました")
    if any(
        path.exists() or path.is_symlink()
        for path in (
            workspace / "private/write-probe.txt",
            fixture_root / "outside/write-probe.txt",
        )
    ):
        raise RuntimeError("Agent Taskがsecret／Workspace外へwrite probeを作成しました")
    for canary in (
        "D4P_SYNTHETIC_SECRET_CANARY=must-not-be-readable",
        "D4P_SYNTHETIC_SECRET_CANARY=outside-workspace",
    ):
        _assert_secret_absent_from_files(
            audit_store,
            canary,
        )
        codex_home = Path(environment["GUI_SHELL_R2_E2E_CODEX_HOME"])
        _assert_secret_absent_from_files(codex_home, canary)
        app_log = acceptance_root / "installed-app.log"
        if app_log.is_file() and canary.encode("utf-8") in app_log.read_bytes():
            raise RuntimeError("synthetic secret canaryがinstalled product logへ漏えいしました")
    return {
        "task_scenarios": ["complete", "cancel", "deadline"],
        "terminal_statuses": ["completed", "cancelled", "failed"],
        "workspace_write": (workspace / "agent-task-result.json").is_file(),
        "secret_and_outside_reads_denied": True,
        "descendant_groups_stopped": len(process_groups) == 2,
        "workspace_task_scratch_removed": not list(workspace.glob(".d4p-tmp-*")),
        "scratch_recovery_journal": scratch_state,
        "task_id_count": len(task_ids),
        "completed_fixture_reports": len(completed_reports),
        "positive_result_hash": positive_result_hash,
        "loopback": loopback,
        "live_audit": audit,
    }


def _open_installed_host_operation_center(
    environment: dict[str, str], window_id: str | None
):
    if _is_wayland_environment(environment):
        raise RuntimeError("Host登録acceptanceはX11 GTK scopeだけを対象にします")
    if window_id is None:
        raise RuntimeError("Host操作面へ移動するinstalled product windowがありません")
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
    host_query = "Host"
    subprocess.run(
        ["xdotool", "type", "--clearmodifiers", "--delay", "30", host_query],
        env=environment,
        check=True,
        timeout=5,
    )
    command = _find_operable_accessible("Host操作面を開く", root=window, timeout=10)
    _activate_accessible(command, "Host操作面を開く", environment)
    _find_accessible("D4 Pocket Host操作面", timeout=10)
    window = _find_accessible(WINDOW_TITLE, timeout=10)
    _find_accessible("D4 Pocket Host操作面", root=window, timeout=10)
    return window


def _open_installed_host_registration_dialog(
    environment: dict[str, str], window_id: str | None
):
    window = _open_installed_host_operation_center(environment, window_id)
    register = _find_operable_accessible("Hostを登録", root=window, timeout=10)
    _activate_accessible(register, "Hostを登録", environment)
    dialog_title = _find_accessible(
        "公開Host metadata登録", root=window, timeout=10
    )
    dialog = _accessible_scope_with_named_descendant(
        dialog_title, "Host識別子", deadline=time.monotonic() + 10
    )
    return window, dialog


def _fill_installed_host_registration(
    environment: dict[str, str],
    dialog,
    window_id: str | None,
    *,
    host_id: str,
    display_name: str,
    identity_hash: str,
) -> None:
    for label, value in (
        ("Host識別子", host_id),
        ("Host表示名", display_name),
        ("公開identity hash", identity_hash),
        ("申告Runtime件数", "2"),
        ("申告Agent件数", "1"),
    ):
        _set_public_text_by_keyboard(dialog, label, value, environment, window_id)
    continue_button = _find_operable_accessible(
        "Owner確認して登録", root=dialog, timeout=10
    )
    _activate_accessible(continue_button, "Owner確認して登録", environment)


def _run_installed_host_registration_acceptance(
    environment: dict[str, str],
    window_id: str | None,
    *,
    audit_store: Path,
    acceptance_root: Path,
) -> dict[str, object]:
    audit_path = audit_store / "audit.jsonl"
    rejected_host_id = f"linux-host-rejected-{uuid.uuid4().hex[:12]}"
    accepted_host_id = f"linux-host-approved-{uuid.uuid4().hex[:12]}"
    rejected_hash = "sha256:" + "a" * 64
    accepted_hash = "sha256:" + "b" * 64
    def request_registration(host_id: str, display_name: str, identity_hash: str):
        window, dialog = _open_installed_host_registration_dialog(
            environment, window_id
        )
        _fill_installed_host_registration(
            environment,
            dialog,
            window_id,
            host_id=host_id,
            display_name=display_name,
            identity_hash=identity_hash,
        )
        _wait_for_owner_dialog_window(
            environment,
            context="Linux installed Host metadata登録",
            diagnostic_root=acceptance_root,
            diagnostic_audit_path=audit_path,
        )
        return window

    rejected_window = request_registration(
        rejected_host_id, "Rejected Linux Host metadata", rejected_hash
    )
    common_owner_markers = (
        "Host metadataを未審査で登録します。",
        "Platform: linux",
        "申告Runtime件数: 2 / Agent件数: 1",
        "remote接続・Trust・Permission・Approval・Credential・Task実行権限を生成せず",
        "表示切替だけを可能にします。",
    )
    _answer_owner_dialog(
        environment,
        "拒否",
        context="Linux Host登録のOwner拒否",
        diagnostic_root=acceptance_root,
        diagnostic_audit_path=audit_path,
        expected_summary_markers=(
            *common_owner_markers,
            f"Host ID: {rejected_host_id}",
            "表示名: Rejected Linux Host metadata",
            f"identity hash: {rejected_hash}",
        ),
    )
    _wait_for_owner_dialog_closed(environment)
    rejected_window = _find_accessible(WINDOW_TITLE, timeout=10)
    _find_accessible(
        "Host登録または一覧更新は未成立です。",
        root=rejected_window,
        timeout=20,
    )
    rejected_event = _wait_for_audit_event(
        audit_path,
        "Host登録",
        "rejected",
        evidence_source="INTERNAL_STATE",
    )
    if rejected_event.get("reason") != "desktop_native_owner_confirmation_required":
        raise RuntimeError("Host登録のOwner拒否が別の理由で記録されました")

    accepted_window = request_registration(
        accepted_host_id, "Approved Linux Host metadata", accepted_hash
    )
    _answer_owner_dialog(
        environment,
        "許可",
        context="Linux Host登録の独立Owner許可",
        diagnostic_root=acceptance_root,
        diagnostic_audit_path=audit_path,
        expected_summary_markers=(
            *common_owner_markers,
            f"Host ID: {accepted_host_id}",
            "表示名: Approved Linux Host metadata",
            f"identity hash: {accepted_hash}",
        ),
    )
    _wait_for_owner_dialog_closed(environment)
    accepted_window = _find_accessible(WINDOW_TITLE, timeout=10)
    _find_accessible(
        "Host metadataを登録しました。未審査のままです。Audit=",
        root=accepted_window,
        timeout=20,
    )
    _find_accessible(accepted_host_id, root=accepted_window, timeout=10)
    projection = _accessible_snapshot(accepted_window)
    for marker in (
        "基盤: linux",
        "接続: pending_review",
        "登録接続状態: pending_review",
        "信頼: pending_review",
        "公開範囲: metadata_only",
        "Runtime summary: 2件 / Agent summary: 1件",
        "別登録Host（remote接続・個別状態は未観測）",
        "remote接続やTrust・権限は生成しません。",
    ):
        if marker not in projection:
            raise RuntimeError(f"Host registration projectionが未審査境界を表示しません: {marker}")
    if rejected_host_id in projection:
        raise RuntimeError("Owner拒否したHostが登録一覧へ現れました")

    _wait_for_audit_event(
        audit_path,
        "Host登録",
        "accepted",
        evidence_source="INTERNAL_STATE",
    )
    audit_text = audit_path.read_text(encoding="utf-8")
    if any(
        marker in audit_text
        for marker in (
            rejected_host_id,
            "Rejected Linux Host metadata",
            rejected_hash,
            accepted_hash,
        )
    ):
        raise RuntimeError("Host登録Auditへraw payload／identity hashが露出しました")
    audit_events = [json.loads(line) for line in audit_text.splitlines() if line]
    audit_projection = _host_registration_audit_evidence(audit_events)
    return {
        "owner_rejected_host_id": rejected_host_id,
        "owner_approved_host_id": accepted_host_id,
        "distinct_request_payload_hashes": audit_projection[
            "host_registration_distinct_request_payload_hashes"
        ],
        "audit_projection": "raw input and identity hashes absent",
        "metadata_projection": "pending_review / metadata_only / no remote observation",
        "authority": "no Permission, Approval, Credential, Trust, or Task authority generated",
    }


def _run_installed_workspace_selection_acceptance(
    environment: dict[str, str],
    window_id: str | None,
    *,
    audit_store: Path,
    acceptance_root: Path,
    screenshot_before_selection_path: Path,
    screenshot_path: Path,
    inspector_failure_screenshot_path: Path,
    registration_only: bool = False,
) -> dict[str, object]:
    fixture_root, workspace, decoy_workspace, cli_fixture, cli_marker = (
        _write_linux_workspace_acceptance_fixture(
            acceptance_root, Path(environment["HOME"])
        )
    )
    workspace_id = "workspace-linux-installed-acceptance"
    secret_paths = "private/token.txt"
    expected_owner_markers = (
        "Agent CLI実行系とWorkspaceを、このBroker起動中だけ登録します。",
        "Runtime: runtime-linux-workspace-acceptance",
        f"CLI実行file: {cli_fixture}",
        f"Workspace ID: {workspace_id}",
        f"Workspace root: {workspace}",
        f"除外する秘密path: {secret_paths}",
        "Task実行能力はunsupportedのままです。",
    )

    dialog = _open_installed_codex_registration(environment, window_id=window_id)
    _fill_linux_workspace_registration(
        environment,
        dialog,
        cli_fixture=cli_fixture,
        workspace_id=workspace_id,
        workspace_root=decoy_workspace,
        secret_paths=secret_paths,
    )
    chooser_window_id = _open_workspace_folder_chooser(environment, dialog)
    _cancel_gtk_folder_chooser(environment, chooser_window_id)
    _wait_for_workspace_root_value(str(decoy_workspace))
    _find_accessible(
        "OS選択を取り消しました。入力は変更していません。",
        root=dialog,
        timeout=10,
    )
    chooser_window_id = _open_workspace_folder_chooser(environment, dialog)
    _select_folder_in_gtk_chooser(
        environment,
        chooser_window_id,
        workspace,
        screenshot_before_selection_path=screenshot_before_selection_path,
        screenshot_path=screenshot_path,
    )
    _wait_for_workspace_root_value(str(workspace))
    for diagnostic_path in (screenshot_before_selection_path, screenshot_path):
        if diagnostic_path.is_symlink():
            raise RuntimeError("successful GTK chooser left a symlink diagnostic image")
        diagnostic_path.unlink(missing_ok=True)
    _find_accessible(
        "OS選択済み（選択結果のみ）。登録・Permission・Approvalは別です。",
        root=dialog,
        timeout=10,
    )
    continue_button = _find_operable_accessible(
        "native Owner確認へ進む", root=dialog, timeout=10
    )
    _activate_accessible(continue_button, "native Owner確認へ進む", environment)
    _wait_for_owner_dialog_window(environment, context="Linux Workspace登録Owner拒否")
    if cli_marker.exists():
        raise RuntimeError("CLI probe started before native Owner approval")
    _answer_owner_dialog(
        environment,
        "拒否",
        context="Linux Workspace登録の初回拒否",
        expected_summary_markers=expected_owner_markers,
    )
    _wait_for_owner_dialog_closed(environment)
    if cli_marker.exists():
        raise RuntimeError("native Owner rejection launched the registered CLI")

    dialog = _open_installed_codex_registration(environment)
    _fill_linux_workspace_registration(
        environment,
        dialog,
        cli_fixture=cli_fixture,
        workspace_id=workspace_id,
        workspace_root=decoy_workspace,
        secret_paths=secret_paths,
    )
    chooser_window_id = _open_workspace_folder_chooser(environment, dialog)
    _select_folder_in_gtk_chooser(
        environment,
        chooser_window_id,
        workspace,
        screenshot_before_selection_path=screenshot_before_selection_path,
        screenshot_path=screenshot_path,
    )
    _wait_for_workspace_root_value(str(workspace))
    continue_button = _find_operable_accessible(
        "native Owner確認へ進む", root=dialog, timeout=10
    )
    _activate_accessible(continue_button, "native Owner確認へ進む", environment)
    _answer_owner_dialog(
        environment,
        "許可",
        context="Linux Workspace登録の独立Owner許可",
        expected_summary_markers=expected_owner_markers,
    )
    _wait_for_owner_dialog_closed(environment)
    _find_accessible("Task実行能力: unsupported", timeout=20)
    if not cli_marker.is_file() or cli_marker.is_symlink():
        raise RuntimeError("Owner-approved Workspace registration did not run its CLI interface probe")
    try:
        probe_calls = [
            json.loads(line)
            for line in cli_marker.read_text(encoding="utf-8").splitlines()
            if line
        ]
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise RuntimeError("run-owned Codex CLI probe evidence is malformed") from error
    if probe_calls != [["--version"], ["exec", "--help"]]:
        raise RuntimeError("Workspace registration launched an unexpected CLI probe")

    window = _find_accessible(WINDOW_TITLE, timeout=10)
    _find_accessible("Permission／Approval／Credential: 生成なし", root=window, timeout=10)
    if registration_only:
        audit_path = audit_store / "audit.jsonl"
        try:
            audit_text = audit_path.read_text(encoding="utf-8")
            audit_events = [
                json.loads(line)
                for line in audit_text.splitlines()
                if line
            ]
        except (OSError, UnicodeError, json.JSONDecodeError) as error:
            raise RuntimeError(
                "Wayland Workspace登録のdurable Auditが利用できません"
            ) from error
        accepted_registration = [
            event
            for event in audit_events
            if event.get("operation") == "AgentCLI実行系作業領域登録"
            and event.get("decision") == "accepted"
            and event.get("evidence_source") == "LIVE_RUNTIME"
        ]
        if len(accepted_registration) != 1:
            raise RuntimeError(
                "Wayland Workspace登録のLIVE_RUNTIME accepted Auditが一意ではありません"
            )
        if any(
            event.get("operation")
            in {
                "AgentTaskWorkspacePermissionGrant",
                "AgentTaskOwnerApprovalGrant",
            }
            for event in audit_events
        ):
            raise RuntimeError(
                "Wayland登録でTask用PermissionまたはApprovalが生成されました"
            )
        if any(
            marker in audit_text
            for marker in (
                str(workspace),
                secret_paths,
                WORKSPACE_ENV_SECRET,
                WORKSPACE_REGISTERED_SECRET,
                WORKSPACE_OUTSIDE_SECRET,
            )
        ):
            raise RuntimeError(
                "Wayland Workspace登録rootまたはsecret markerがdurable Auditへ露出しました"
            )
        for secret in (
            WORKSPACE_ENV_SECRET,
            WORKSPACE_REGISTERED_SECRET,
            WORKSPACE_OUTSIDE_SECRET,
        ):
            _assert_secret_absent_from_files(
                acceptance_root, secret, excluded_roots=(fixture_root,)
            )
        return {
            "workspace_selection": "Wayland GTK native chooser selected and cancellation preserved prior input",
            "registration": "native Owner rejected before CLI probe, then independently approved",
            "cli_interface_probe_calls": len(probe_calls),
            "workspace_read_permission": "not requested",
            "task_execution": "unsupported",
            "audit_secret_and_root_exposure": "absent",
            "accepted_registration_audit": "one LIVE_RUNTIME event",
        }
    row = _find_workspace_inspector_control(
        workspace_id,
        workspace_id,
        environment,
        timeout=20,
        diagnostic_screenshot_path=inspector_failure_screenshot_path,
    )
    _activate_accessible(row, workspace_id, environment)
    _select_workspace_inspector_visibility(
        workspace_id,
        "full",
        environment,
        diagnostic_screenshot_path=inspector_failure_screenshot_path,
    )
    approve = _find_workspace_inspector_control(
        "native Owner確認でWorkspace読取を許可",
        workspace_id,
        environment,
        timeout=10,
    )
    _activate_accessible(approve, "native Owner確認でWorkspace読取を許可", environment)
    _answer_owner_dialog(
        environment,
        "許可",
        context="Linux Workspace内容露出の独立Approval",
        expected_summary_markers=(
            "Workspace操作: 作業領域承認",
            f"Workspace ID: {workspace_id}",
            "内容露出: full",
            "fullは登録済みsecret除外以外の本文を画面へ返せます。",
        ),
    )
    _wait_for_owner_dialog_closed(environment)
    _, inspector = _workspace_inspector_scope(workspace_id)
    try:
        _find_accessible(
            "現在のWorkspace読取範囲: full・Task実行／書込権は別です。",
            root=inspector,
            timeout=15,
        )
    except RuntimeError as error:
        screenshot_captured = _capture_gtk_dialog_screenshot(
            environment, WINDOW_TITLE, inspector_failure_screenshot_path
        )
        safe_status_names = []
        for _, accessible_name in _accessible_nodes(
            inspector, deadline=time.monotonic() + 2, max_depth=12
        ):
            if (
                accessible_name == workspace_id
                or accessible_name.startswith(
                    (
                        f"{workspace_id}・",
                        "現在のWorkspace読取範囲:",
                        "native Owner確認でWorkspace読取を許可",
                        "Workspace読取Approvalを確認できません。",
                        "現在の登録・承認状態を確認中",
                        "登録状態を確認できないため表示を停止しました。",
                        "読取には所有者による現在登録の承認が必要です。",
                    )
                )
            ):
                safe_status_names.append(accessible_name[:180])
        raise RuntimeError(
            f"{error}; inspector_status={list(dict.fromkeys(safe_status_names))}; "
            f"screenshot_captured={screenshot_captured}"
        ) from error
    row = _find_workspace_inspector_control(
        workspace_id,
        workspace_id,
        environment,
        timeout=10,
    )
    _activate_accessible(row, workspace_id, environment)
    public_file = _find_workspace_inspector_control(
        "public.txt",
        workspace_id,
        environment,
        timeout=15,
    )
    _, inspector = _workspace_inspector_scope(workspace_id)
    initial_projection = _accessible_snapshot(inspector)
    tree_projection_evidence = _verify_workspace_projection(
        initial_projection,
        public_markers=("public.txt",),
        forbidden_markers=(
            ".env",
            "private/token.txt",
            "external-alias.txt",
            WORKSPACE_ENV_SECRET,
            WORKSPACE_REGISTERED_SECRET,
            WORKSPACE_OUTSIDE_SECRET,
        ),
    )
    _activate_accessible(public_file, "public.txt", environment)
    projection, projection_evidence = _wait_for_workspace_projection(
        lambda: _accessible_snapshot(
            _workspace_inspector_scope(workspace_id, timeout=3)[1]
        ),
        public_markers=("public.txt", WORKSPACE_PUBLIC_CONTENT),
        forbidden_markers=(
            ".env",
            "private/token.txt",
            "external-alias.txt",
            WORKSPACE_ENV_SECRET,
            WORKSPACE_REGISTERED_SECRET,
            WORKSPACE_OUTSIDE_SECRET,
        ),
    )

    audit_path = audit_store / "audit.jsonl"
    try:
        audit_text = audit_path.read_text(encoding="utf-8")
    except (OSError, UnicodeError) as error:
        raise RuntimeError("Workspace acceptance durable Audit is unavailable") from error
    if any(
        marker in audit_text
        for marker in (
            str(workspace),
            secret_paths,
            ".env",
            WORKSPACE_ENV_SECRET,
            WORKSPACE_REGISTERED_SECRET,
            WORKSPACE_OUTSIDE_SECRET,
        )
    ):
        raise RuntimeError("Workspace root or secret content leaked into durable Audit")
    for secret in (
        WORKSPACE_ENV_SECRET,
        WORKSPACE_REGISTERED_SECRET,
        WORKSPACE_OUTSIDE_SECRET,
    ):
        _assert_secret_absent_from_files(
            acceptance_root, secret, excluded_roots=(fixture_root,)
        )
    return {
        "workspace_selection": "GTK native chooser selected and cancellation preserved prior input",
        "registration": "native Owner rejected before CLI probe, then independently approved",
        "cli_interface_probe_calls": len(probe_calls),
        "workspace_read_approval": "separate native Owner approval for full content projection",
        "workspace_tree_projection": tree_projection_evidence,
        "workspace_projection": projection_evidence,
        "task_execution": "unsupported",
        "audit_secret_and_root_exposure": "absent",
    }


def _run_installed_mcp_tool_acceptance(
    environment: dict[str, str],
    window_id: str,
    *,
    helper: Path,
    audit_store: Path,
    temp_root: Path,
    acceptance_root: Path,
) -> dict[str, object]:
    workspace, fixture, seed, result_marker = _write_linux_mcp_fixture(
        temp_root, acceptance_root
    )
    python_executable = Path("/usr/bin/python3")
    if not python_executable.is_file() or not os.access(python_executable, os.X_OK):
        raise RuntimeError("導入済みMCP受入れには実行runnerが所有する/usr/bin/python3が必要です")
    server_id = "linux-installed-mcp-fixture"
    start_marker = workspace / "mcp-child-started.json"
    call_marker = workspace / "mcp-tool-called.json"
    window = _open_mcp_center_from_installed_ui(environment, window_id)

    for label, value in (
        ("サーバー識別子", server_id),
        ("実行ファイルの絶対パス", str(python_executable)),
        ("作業フォルダーの絶対パス", str(workspace)),
        ("起動引数（1行に1項目、最大32項目）", f"{fixture}\n{seed}"),
    ):
        _set_public_text_by_keyboard(window, label, value, environment, window_id)

    connect = _find_operable_accessible("MCP接続を開始", root=window, timeout=10)
    _activate_accessible(connect, "MCP接続を開始", environment)
    _answer_owner_dialog(
        environment,
        "拒否",
        context="MCP接続の初回拒否",
        diagnostic_root=window,
        diagnostic_audit_path=audit_store / "audit.jsonl",
        expected_summary_markers=(
            f"Server ID: {server_id}",
            f"実行file: {python_executable}",
            f"{MCP_OWNER_WORKSPACE_LABEL}{workspace}",
            MCP_OWNER_NO_CREDENTIAL_MARKER,
        ),
    )
    _wait_for_owner_dialog_closed(environment)
    _find_accessible("MCP接続は確定していません", root=window, timeout=15)
    if start_marker.exists() or call_marker.exists():
        raise RuntimeError("導入済みMCPのnative所有者拒否後に子processまたはツールが起動しました")

    connect = _find_operable_accessible("MCP接続を開始", root=window, timeout=10)
    _activate_accessible(connect, "MCP接続を開始", environment)
    _answer_owner_dialog(
        environment,
        "許可",
        context="MCP接続の独立許可",
        diagnostic_root=window,
        diagnostic_audit_path=audit_store / "audit.jsonl",
        expected_summary_markers=(
            f"Server ID: {server_id}",
            f"実行file: {python_executable}",
            f"{MCP_OWNER_WORKSPACE_LABEL}{workspace}",
            MCP_OWNER_NO_CREDENTIAL_MARKER,
        ),
    )
    _wait_for_owner_dialog_closed(environment)
    _find_accessible("接続receiptを受理しました", root=window, timeout=20)
    deadline = time.monotonic() + 10
    while not start_marker.exists() and time.monotonic() < deadline:
        time.sleep(0.1)
    pid, pgid = _read_mcp_child_identity(workspace)
    if not _mcp_process_group_is_fixture_owned(
        pid, pgid, helper=helper, fixture=fixture
    ):
        raise RuntimeError("導入済みMCP子processは導入済みBrokerの監督下にありません")
    _wait_for_mcp_group_state(pgid, True)

    _expand_single_installed_mcp_tool_list(environment, window)
    run_tool = _find_operable_accessible("確認して実行", root=window, timeout=10)
    _activate_accessible(run_tool, "確認して実行", environment)
    _enter_single_installed_mcp_tool_arguments(environment, window, window_id)
    _review_single_installed_mcp_tool_arguments(environment, window)
    _answer_owner_dialog(
        environment,
        "許可",
        context="MCP Toolの一回実行許可",
        diagnostic_root=window,
        diagnostic_audit_path=audit_store / "audit.jsonl",
        expected_summary_markers=(
            f"Server ID: {server_id}",
            "Tool: echo",
            MCP_OWNER_ARGUMENT_HASH_MARKER,
        ),
    )
    _wait_for_owner_dialog_closed(environment)
    _find_accessible("Tool応答receiptを受理しました", root=window, timeout=20)
    _find_accessible("result hash: sha256:", root=window, timeout=10)
    _find_accessible("content: text", root=window, timeout=10)
    if not call_marker.is_file() or call_marker.is_symlink():
        raise RuntimeError("導入済みMCP子processがtools/callを一回だけ受け取りませんでした")
    try:
        call_document = json.loads(call_marker.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise RuntimeError("導入済みMCPのtools/call markerの形式が不正です") from error
    if call_document != {"method": "tools/call", "count": 1}:
        raise RuntimeError("導入済みMCP子processが想定したtools/call一回を受け取りませんでした")

    snapshot_after_tool = _accessible_snapshot(window)
    if result_marker in snapshot_after_tool:
        raise RuntimeError("導入済みMCPツール結果本文がUIのアクセシビリティ木へ漏えいしました")
    result_hashes = re.findall(r"result hash: (sha256:[0-9a-f]{64})", snapshot_after_tool)
    if len(result_hashes) != 1:
        raise RuntimeError("導入済みMCPツールUIにハッシュのみの結果受領票が一つだけ表示されませんでした")

    _activate_named_control_by_keyboard(
        "切断", environment, window_id, context_marker=server_id
    )
    _answer_owner_dialog(
        environment,
        "拒否",
        context="MCP切断の初回拒否",
        diagnostic_root=window,
        diagnostic_audit_path=audit_store / "audit.jsonl",
        expected_summary_markers=(f"Server ID: {server_id}",),
    )
    _wait_for_owner_dialog_closed(environment)
    _find_accessible("切断は確定していません", root=window, timeout=15)
    if not _group_exists(pgid):
        raise RuntimeError("MCP切断の所有者拒否が登録中の子processを停止させました")

    _activate_named_control_by_keyboard(
        "切断", environment, window_id, context_marker=server_id
    )
    _answer_owner_dialog(
        environment,
        "許可",
        context="MCP切断の独立許可",
        diagnostic_root=window,
        diagnostic_audit_path=audit_store / "audit.jsonl",
        expected_summary_markers=(f"Server ID: {server_id}",),
    )
    _wait_for_owner_dialog_closed(environment)
    _find_accessible("切断receiptを受理しました", root=window, timeout=20)
    _wait_for_mcp_group_state(pgid, False)
    _find_accessible("Brokerが保持するMCP接続はありません。", root=window, timeout=10)

    audit = _verify_live_audit(
        audit_store,
        require_export=False,
        require_mcp=True,
    )
    audit_events = [
        json.loads(line)
        for line in (audit_store / "audit.jsonl").read_text(encoding="utf-8").splitlines()
        if line
    ]
    tool_events = [
        event
        for event in audit_events
        if event.get("operation") == "MCP Tool実行"
        and event.get("decision") == "accepted"
        and event.get("evidence_source") == "LIVE_RUNTIME"
    ]
    if len(tool_events) != 1 or result_hashes[0] not in tool_events[0].get("reason", ""):
        raise RuntimeError("導入済みMCP UIの結果ハッシュが永続LIVE_RUNTIME監査に結合されていません")
    _assert_secret_absent_from_files(
        acceptance_root,
        result_marker,
        excluded_roots=(acceptance_root / "home/.local/share/apps",),
    )
    return {
        "server_id": server_id,
        "installed_python_child_started_after_owner_allow": True,
        "connection_owner_rejected_then_allowed": True,
        "tool_call_owner_approved_once": True,
        "tool_result": "hash_only_live_runtime_receipt",
        "result_hash_matches_durable_audit": True,
        "result_body_absent_from_ui_audit_logs_and_acceptance_files": True,
        "disconnect_owner_rejected_then_allowed": True,
        "owned_process_group_stopped": True,
        "audit": audit,
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
    # FlutterはAT-SPIの操作機能／位置情報を持たないTextButton表示名を公開する場合があります。
    # キーボードで操作する前に、重複する操作名を候補行へ束縛します。
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
    context: str = "Owner確認",
    diagnostic_root=None,
    diagnostic_audit_path: Path | None = None,
    expected_summary_markers: tuple[str, ...],
) -> None:
    if decision not in {"許可", "拒否"}:
        raise ValueError("Owner確認の回答値が不正")
    _wait_for_owner_dialog_window(
        environment,
        context=context,
        diagnostic_root=diagnostic_root,
        diagnostic_audit_path=diagnostic_audit_path,
    )
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
            + "; 範囲を限定したアクセシビリティ表示文字列="
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
        if _is_wayland_environment(environment):
            if not _wayland_named_window_is_visible(
                OWNER_DIALOG_TITLE,
                timeout=min(0.2, deadline - time.monotonic()),
            ):
                return
            time.sleep(0.1)
            continue
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
    environment: dict[str, str],
    *,
    timeout: float = OWNER_DIALOG_TIMEOUT,
    context: str = "Owner確認",
    diagnostic_root=None,
    diagnostic_audit_path: Path | None = None,
) -> str | None:
    deadline = time.monotonic() + timeout
    while True:
        if _is_wayland_environment(environment):
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise RuntimeError(
                    "GTK native Owner確認dialogが期限内に表示されません; "
                    f"context={context}; "
                    + _owner_dialog_timeout_diagnostics(
                        environment, diagnostic_root, diagnostic_audit_path
                    )
                )
            if _wayland_named_window_is_visible(
                OWNER_DIALOG_TITLE, timeout=min(0.2, remaining)
            ):
                return None
            time.sleep(min(0.1, remaining))
            continue
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
            raise RuntimeError(
                "GTK native Owner確認dialogが期限内に表示されません; "
                f"context={context}; "
                + _owner_dialog_timeout_diagnostics(
                    environment, diagnostic_root, diagnostic_audit_path
                )
            )
        time.sleep(0.1)


def _owner_dialog_timeout_diagnostics(
    environment: dict[str, str],
    diagnostic_root=None,
    diagnostic_audit_path: Path | None = None,
) -> str:
    if _is_wayland_environment(environment):
        try:
            import pyatspi

            desktop = pyatspi.Registry.getDesktop(0)
            names = [
                name
                for _, name in _accessible_nodes(
                    desktop, deadline=time.monotonic() + 1, max_depth=8
                )
                if name
            ]
            return _bounded_owner_text_diagnostics(names)
        except Exception:
            return "Wayland AT-SPI desktop unavailable"
    visible = subprocess.run(
        ["xdotool", "search", "--onlyvisible", "--name", ".*"],
        env=environment,
        capture_output=True,
        check=False,
        timeout=5,
    )
    window_ids = (
        visible.stdout.decode("ascii", errors="ignore").splitlines()
        if visible.returncode == 0
        else []
    )
    kinds = {"product": 0, "owner": 0, "credential": 0, "workspace": 0, "other": 0}
    expected_titles = {
        WINDOW_TITLE: "product",
        OWNER_DIALOG_TITLE: "owner",
        "D4 Pocket 資格情報入力": "credential",
        "D4 Pocket 作業領域の選択": "workspace",
    }
    for window_id in window_ids[:32]:
        title = subprocess.run(
            ["xdotool", "getwindowname", window_id],
            env=environment,
            capture_output=True,
            check=False,
            timeout=5,
        )
        name = title.stdout.decode("utf-8", errors="replace").strip()
        kinds[expected_titles.get(name, "other")] += 1

    statuses: list[str] = []
    if diagnostic_root is not None:
        allowed_statuses = (
            "MCP接続は確定していません",
            "接続receiptを受理しました",
            "MCP接続設定の識別子または絶対pathが不正です",
            "安全Brokerとの通信に失敗しました",
        )
        try:
            for _, name in _accessible_nodes(diagnostic_root):
                normalized = " ".join(name.split())
                if any(normalized.startswith(value) for value in allowed_statuses):
                    statuses.append(normalized[:160])
        except Exception:
            statuses.append("<accessible-state-unavailable>")
    audit_status = "<unavailable>"
    if diagnostic_audit_path is not None:
        try:
            matching_events = []
            for line in diagnostic_audit_path.read_text(encoding="utf-8").splitlines():
                if not line:
                    continue
                event = json.loads(line)
                if isinstance(event, dict) and event.get("operation") == "MCP接続":
                    matching_events.append(event)
            if matching_events:
                event = matching_events[-1]
                decision = event.get("decision")
                decision = (
                    decision
                    if decision in {"accepted", "rejected", "denied"}
                    else "other"
                )
                reason = event.get("reason")
                reason_categories = {
                    "MCP接続にはowner controlが必要": "owner_required",
                    "MCP接続には永続Auditが必要": "audit_unavailable",
                    "MCP接続の受信Auditを確定できない": "receive_audit_failed",
                    "MCP接続の受理Auditを確定できない": "acceptance_audit_failed",
                }
                audit_status = (
                    f"decision:{decision},category:{reason_categories.get(reason, 'other')}"
                )
            else:
                audit_status = "<no_mcp_connect_event>"
        except (OSError, UnicodeError, json.JSONDecodeError, TypeError):
            audit_status = "<unavailable_or_malformed>"
    return (
        "visible_window_kinds="
        + ",".join(f"{name}:{count}" for name, count in kinds.items())
        + "; product_status="
        + ("|".join(dict.fromkeys(statuses[-4:])) if statuses else "<none-observed>")
        + "; mcp_connect_audit="
        + audit_status
    )


def _agent_task_audit_acceptance_diagnostics(
    task_starts: list[dict[str, object]],
    task_terminals: list[dict[str, object]],
    events: list[dict[str, object]],
    scratch_mediation: list[dict[str, object]],
) -> dict[str, object]:
    start_task_ids = [record.get("task_id") for record in task_starts]
    terminal_task_ids = [record.get("task_id") for record in task_terminals]
    request_hashes = [record.get("request_hash") for record in task_starts]
    valid_start_task_ids = [value for value in start_task_ids if isinstance(value, str)]
    valid_terminal_task_ids = [
        value for value in terminal_task_ids if isinstance(value, str)
    ]
    valid_request_hashes = [value for value in request_hashes if isinstance(value, str)]
    statuses = [record.get("status") for record in task_terminals]
    expected_statuses = ("cancelled", "completed", "failed")
    status_counts = {
        status: sum(value == status for value in statuses)
        for status in expected_statuses
    }
    unknown_status_count = sum(
        not isinstance(value, str) or value not in expected_statuses
        for value in statuses
    )
    scratch_cleanup_confirmed = sum(
        item.get("scratch_cleanup_completed") is True for item in scratch_mediation
    )
    permission_operation = "AgentTaskWorkspacePermissionGrant"
    approval_operation = "AgentTaskOwnerApprovalGrant"
    permission_issue_reason = (
        "Agent Task用Workspace Permission発行（native Owner確認・Task未実行）"
    )
    approval_issue_reason = (
        "Agent Task本文hash・実行条件hashへのOwner Approval発行（native確認・Task未実行）"
    )

    def operation_audit_counts(
        operation: str, issue_reason: str
    ) -> dict[str, int]:
        operation_events = [
            event for event in events if event.get("operation") == operation
        ]
        request_receipts = [
            event
            for event in operation_events
            if event.get("decision") == "received"
            and event.get("reason") == "対話操作を受信"
        ]
        issue_records = [
            event
            for event in operation_events
            if event.get("decision") == "recorded"
            and event.get("reason") == issue_reason
        ]
        request_ids = [
            event.get("request_id")
            for event in request_receipts
            if isinstance(event.get("request_id"), str)
        ]
        issue_hashes = [
            event.get("payload_hash")
            for event in issue_records
            if isinstance(event.get("payload_hash"), str)
        ]
        return {
            "operation_events": len(operation_events),
            "request_receipts": len(request_receipts),
            "unique_request_ids": len(set(request_ids)),
            "issue_records": len(issue_records),
            "unique_issue_hashes": len(set(issue_hashes)),
            "unclassified_operation_events": len(operation_events)
            - len(request_receipts)
            - len(issue_records),
        }

    permission_audit = operation_audit_counts(
        permission_operation, permission_issue_reason
    )
    approval_audit = operation_audit_counts(approval_operation, approval_issue_reason)
    checks = {
        "three_task_start_records": len(task_starts) == 3,
        "three_task_terminal_records": len(task_terminals) == 3,
        "three_unique_task_ids": len(set(valid_start_task_ids)) == 3
        and len(valid_start_task_ids) == len(start_task_ids),
        "terminal_task_ids_match": len(valid_terminal_task_ids)
        == len(terminal_task_ids)
        and set(valid_start_task_ids) == set(valid_terminal_task_ids),
        "three_unique_request_hashes": len(set(valid_request_hashes)) == 3
        and len(valid_request_hashes) == len(request_hashes),
        "expected_terminal_statuses": unknown_status_count == 0
        and all(status_counts[status] == 1 for status in expected_statuses),
        "three_workspace_permission_requests": permission_audit["request_receipts"]
        == 3,
        "three_unique_workspace_permission_request_ids": permission_audit[
            "unique_request_ids"
        ]
        == 3,
        "three_workspace_permission_grants": permission_audit["issue_records"] == 3,
        "three_unique_workspace_permission_issuance_hashes": permission_audit[
            "unique_issue_hashes"
        ]
        == 3,
        "three_owner_approval_requests": approval_audit["request_receipts"] == 3,
        "three_unique_owner_approval_request_ids": approval_audit[
            "unique_request_ids"
        ]
        == 3,
        "three_owner_approval_grants": approval_audit["issue_records"] == 3,
        "three_unique_owner_approval_issuance_hashes": approval_audit[
            "unique_issue_hashes"
        ]
        == 3,
        "one_scratch_cleanup_confirmation": scratch_cleanup_confirmed == 1,
    }
    return {
        "checks": checks,
        "failed_checks": [name for name, passed in checks.items() if not passed],
        "counts": {
            "task_start_records": len(task_starts),
            "task_terminal_records": len(task_terminals),
            "unique_start_task_ids": len(set(valid_start_task_ids)),
            "terminal_task_ids_match": checks["terminal_task_ids_match"],
            "unique_start_request_hashes": len(set(valid_request_hashes)),
            "terminal_status_counts": status_counts,
            "unknown_terminal_status_count": unknown_status_count,
            "workspace_permission_audit": permission_audit,
            "owner_approval_audit": approval_audit,
            "scratch_mediation_records": len(scratch_mediation),
            "scratch_cleanup_confirmations": scratch_cleanup_confirmed,
        },
    }


def _host_registration_audit_evidence(
    events: list[dict[str, object]],
) -> dict[str, int]:
    registrations = [
        event for event in events if event.get("operation") == "Host登録"
    ]
    rejected = [
        event
        for event in registrations
        if event.get("decision") == "rejected"
        and event.get("reason") == "desktop_native_owner_confirmation_required"
        and event.get("evidence_source") == "INTERNAL_STATE"
    ]
    accepted = [
        event
        for event in registrations
        if event.get("decision") == "accepted"
        and event.get("evidence_source") == "INTERNAL_STATE"
    ]
    received = [
        event
        for event in registrations
        if event.get("decision") == "received"
        and event.get("evidence_source") == "INTERNAL_STATE"
    ]
    terminal = [*rejected, *accepted]
    if (
        len(rejected) != 1
        or len(accepted) != 1
        or len(received) != 1
        or len(terminal) != 2
        or received[0].get("request_id") != accepted[0].get("request_id")
    ):
        raise RuntimeError("Host登録のfresh Owner拒否／許可Auditが一意ではありません")
    request_ids = [rejected[0].get("request_id"), received[0].get("request_id")]
    payload_hashes = [rejected[0].get("payload_hash"), received[0].get("payload_hash")]
    audit_hashes = [event.get("payload_hash") for event in terminal]
    if (
        not all(isinstance(value, str) and value for value in request_ids)
        or len(set(request_ids)) != 2
        or not all(
            isinstance(value, str)
            and re.fullmatch(r"sha256:[0-9a-f]{64}", value)
            for value in [*payload_hashes, *audit_hashes]
        )
        or len(set(payload_hashes)) != 2
    ):
        raise RuntimeError("Host登録の拒否／許可要求が独立hashへ束縛されていません")
    authority_terms = ("permission", "approval", "credential", "trust", "task")
    forbidden_authority_events = [
        event
        for event in events
        if any(
            term in str(event.get("operation", "")).casefold()
            for term in authority_terms
        )
    ]
    if forbidden_authority_events:
        raise RuntimeError(
            "Host登録acceptanceでPermission／Approval／Credential／Trust／Task authorityが生成されました"
        )
    return {
        "host_registration_owner_rejected": len(rejected),
        "host_registration_internal_accepted": len(accepted),
        "host_registration_distinct_requests": len(set(request_ids)),
        "host_registration_distinct_request_payload_hashes": len(set(payload_hashes)),
        "host_registration_forbidden_authority_events": len(
            forbidden_authority_events
        ),
    }


def _verify_live_audit(
    store: Path,
    *,
    require_export: bool = True,
    require_mcp: bool = False,
    require_workspace: bool = False,
    require_uninstall: bool = False,
    require_rollback: bool = False,
    require_catalog: bool = False,
    require_update_lifecycle: bool = False,
    require_agent_task: bool = False,
    require_host_registration: bool = False,
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
            raise RuntimeError("導入済み製品の監査事象ハッシュが不正です")
        previous_hash = expected

    anchor = json.loads(anchor_path.read_text(encoding="utf-8"))
    try:
        key = bytes.fromhex(key_path.read_text(encoding="ascii").strip())
    except (OSError, UnicodeError, ValueError) as error:
        raise RuntimeError("導入済み製品の監査アンカー鍵が不正です") from error
    if len(key) != 32:
        raise RuntimeError("導入済み製品の監査アンカー鍵長が不正です")
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
        raise RuntimeError("導入済み製品の監査アンカー／HMACが不正です")

    exports = [
        event
        for event in events
        if event.get("operation") == "GUI Shell書出し"
        and event.get("evidence_source") == "INTERNAL_STATE"
    ]
    accepted = sum(event.get("decision") == "accepted" for event in exports)
    rejected = sum(event.get("decision") == "rejected" for event in exports)
    if require_export and (accepted < 1 or rejected < 1):
        raise RuntimeError("GTK Owner拒否／許可の両方がINTERNAL_STATE Auditにありません")
    mcp_connect = [event for event in events if event.get("operation") == "MCP接続"]
    mcp_tool = [event for event in events if event.get("operation") == "MCP Tool実行"]
    mcp_disconnect = [event for event in events if event.get("operation") == "MCP切断"]
    mcp_connect_rejected = sum(
        event.get("decision") == "rejected"
        and event.get("reason") == "desktop_native_owner_confirmation_required"
        and event.get("evidence_source") == "INTERNAL_STATE"
        for event in mcp_connect
    )
    mcp_connect_accepted = sum(
        event.get("decision") == "accepted"
        and event.get("evidence_source") == "INTERNAL_STATE"
        for event in mcp_connect
    )
    mcp_tool_approved = sum(
        event.get("decision") == "approved"
        and event.get("evidence_source") == "INTERNAL_STATE"
        for event in mcp_tool
    )
    mcp_tool_started = sum(
        event.get("decision") == "started"
        and event.get("evidence_source") == "INTERNAL_STATE"
        and "consumed=true" in event.get("reason", "")
        for event in mcp_tool
    )
    mcp_tool_live_completed = sum(
        event.get("decision") == "accepted"
        and event.get("evidence_source") == "LIVE_RUNTIME"
        and "ハッシュだけを含む受領票" in event.get("reason", "")
        for event in mcp_tool
    )
    mcp_disconnect_rejected = sum(
        event.get("decision") == "rejected"
        and event.get("reason") == "desktop_native_owner_confirmation_required"
        and event.get("evidence_source") == "INTERNAL_STATE"
        for event in mcp_disconnect
    )
    mcp_disconnect_live_completed = sum(
        event.get("decision") == "accepted"
        and event.get("evidence_source") == "LIVE_RUNTIME"
        for event in mcp_disconnect
    )
    if require_mcp and not all(
        (
            mcp_connect_rejected >= 1,
            mcp_connect_accepted >= 1,
            mcp_tool_approved == 1,
            mcp_tool_started == 1,
            mcp_tool_live_completed == 1,
            mcp_disconnect_rejected >= 1,
            mcp_disconnect_live_completed >= 1,
        )
    ):
        raise RuntimeError(
            "installed MCP connect／Tool one-shot Permission／disconnectのOwner・LIVE_RUNTIME Auditが不足しています"
        )
    workspace_selection = [
        event for event in events if event.get("operation") == "作業領域OS選択"
    ]
    workspace_registration = [
        event
        for event in events
        if event.get("operation") == "AgentCLI実行系作業領域登録"
    ]
    workspace_approval = [
        event for event in events if event.get("operation") == "作業領域承認"
    ]
    workspace_tree = [
        event for event in events if event.get("operation") == "作業領域ツリー"
    ]
    workspace_selection_accepted = sum(
        event.get("decision") == "accepted"
        and event.get("evidence_source") == "INTERNAL_STATE"
        for event in workspace_selection
    )
    workspace_registration_rejected = sum(
        event.get("decision") == "rejected"
        and event.get("reason") == "desktop_native_owner_confirmation_required"
        and event.get("evidence_source") == "INTERNAL_STATE"
        for event in workspace_registration
    )
    workspace_registration_accepted = sum(
        event.get("decision") == "accepted"
        and event.get("evidence_source") == "LIVE_RUNTIME"
        for event in workspace_registration
    )
    workspace_approval_recorded = sum(
        event.get("decision") == "recorded"
        and event.get("reason") == "ownerが現在登録の読取を承認"
        and event.get("evidence_source") == "INTERNAL_STATE"
        for event in workspace_approval
    )
    workspace_approval_accepted = sum(
        event.get("decision") == "accepted"
        and event.get("evidence_source") == "INTERNAL_STATE"
        for event in workspace_approval
    )
    workspace_tree_accepted = sum(
        event.get("decision") == "accepted"
        and event.get("evidence_source") == "LIVE_RUNTIME"
        for event in workspace_tree
    )
    task_permission_events = sum(
        event.get("operation")
        in {"AgentTaskWorkspacePermissionGrant", "AgentTaskOwnerApprovalGrant"}
        for event in events
    )
    task_records: list[dict[str, object]] = []
    scratch_mediation: list[dict[str, object]] = []
    for event in events:
        reason = event.get("reason")
        if not isinstance(reason, str):
            continue
        if event.get("operation") in {"AgentTask実行", "AgentTask状態"} and reason.startswith(
            "AgentTask履歴:"
        ):
            try:
                record = json.loads(reason.removeprefix("AgentTask履歴:"))
            except json.JSONDecodeError as error:
                raise RuntimeError("LIVE_RUNTIME Agent Task履歴Auditが不正JSONです") from error
            if not isinstance(record, dict):
                raise RuntimeError("LIVE_RUNTIME Agent Task履歴Auditがobjectではありません")
            task_records.append(record)
        if reason.startswith("AgentTask調停作用:"):
            try:
                evidence = json.loads(reason.removeprefix("AgentTask調停作用:"))
            except json.JSONDecodeError as error:
                raise RuntimeError("LIVE_RUNTIME Agent Task scratch調停Auditが不正JSONです") from error
            if isinstance(evidence, dict):
                scratch_mediation.append(evidence)
    task_starts = [record for record in task_records if record.get("stage") == "start"]
    task_terminals = [record for record in task_records if record.get("stage") == "terminal"]
    task_statuses = [record.get("status") for record in task_terminals]
    scratch_cleanup_confirmed = sum(
        item.get("scratch_cleanup_completed") is True for item in scratch_mediation
    )
    if require_agent_task:
        acceptance_diagnostics = _agent_task_audit_acceptance_diagnostics(
            task_starts,
            task_terminals,
            events,
            scratch_mediation,
        )
        permission_audit = acceptance_diagnostics["counts"][
            "workspace_permission_audit"
        ]
        approval_audit = acceptance_diagnostics["counts"]["owner_approval_audit"]
        task_permission_grants = permission_audit["issue_records"]
        task_owner_approvals = approval_audit["issue_records"]
        if acceptance_diagnostics["failed_checks"]:
            raise RuntimeError(
                "installed Agent Task Auditの有限受入れ条件が不成立です: "
                + json.dumps(acceptance_diagnostics, ensure_ascii=False, sort_keys=True)
            )
        deadline_records = [
            record
            for record in task_terminals
            if record.get("status") == "failed"
            and isinstance(record.get("failure_class"), str)
            and "期限" in record["failure_class"]
        ]
        cancelled_records = [
            record for record in task_terminals if record.get("status") == "cancelled"
        ]
        completed_records = [
            record for record in task_terminals if record.get("status") == "completed"
        ]
        if not all((len(deadline_records) == 1, len(cancelled_records) == 1, len(completed_records) == 1)):
            raise RuntimeError(
                "deadline／cancel／positive Task terminal Auditの対応が不正です: "
                + json.dumps(
                    {
                        "deadline_failure_records": len(deadline_records),
                        "cancelled_records": len(cancelled_records),
                        "completed_records": len(completed_records),
                    },
                    ensure_ascii=False,
                    sort_keys=True,
                )
            )
    if require_workspace and not all(
        (
            workspace_selection_accepted >= 2,
            workspace_registration_rejected >= 1,
            workspace_registration_accepted == 1,
            workspace_approval_recorded >= 1,
            workspace_approval_accepted >= 1,
            workspace_tree_accepted >= 1,
            task_permission_events == 0,
        )
    ):
        raise RuntimeError(
            "installed Workspace selection／registration／read approval／tree Audit evidence is incomplete"
        )
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
    host_registration_evidence = (
        _host_registration_audit_evidence(events)
        if require_host_registration
        else None
    )
    audit_evidence: dict[str, int] = {
        "events": len(events),
        "export_accepted": accepted,
        "export_rejected": rejected,
        "mcp_connect_owner_rejected": mcp_connect_rejected,
        "mcp_connect_internal_accepted": mcp_connect_accepted,
        "mcp_tool_approved": mcp_tool_approved,
        "mcp_tool_permission_consumed": mcp_tool_started,
        "mcp_tool_live_hash_only_completed": mcp_tool_live_completed,
        "mcp_disconnect_owner_rejected": mcp_disconnect_rejected,
        "mcp_disconnect_live_completed": mcp_disconnect_live_completed,
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
    if host_registration_evidence is not None:
        audit_evidence.update(host_registration_evidence)
    if require_workspace:
        audit_evidence.update(
            {
                "workspace_selection_accepted": workspace_selection_accepted,
                "workspace_registration_owner_rejected": workspace_registration_rejected,
                "workspace_registration_live_accepted": workspace_registration_accepted,
                "workspace_read_approval_recorded": workspace_approval_recorded,
                "workspace_read_approval_accepted": workspace_approval_accepted,
                "workspace_tree_live_accepted": workspace_tree_accepted,
                "agent_task_permission_or_approval_events": task_permission_events,
            }
        )
    if require_agent_task:
        audit_evidence.update(
            {
                "agent_task_started": len(task_starts),
                "agent_task_completed": task_statuses.count("completed"),
                "agent_task_cancelled": task_statuses.count("cancelled"),
                "agent_task_deadline_failed": task_statuses.count("failed"),
                "agent_task_permission_grants": task_permission_grants,
                "agent_task_owner_approvals": task_owner_approvals,
                "agent_task_scratch_cleanup_confirmed": scratch_cleanup_confirmed,
            }
        )
    return audit_evidence


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
    runner_home = Path(os.environ.get("HOME") or Path.home())
    test_environment = environment | {
        "D4P_LINUX_ACCEPTANCE_PRIVATE_KEY": str(private_key_path)
    }
    test_environment.setdefault(
        "CARGO_HOME", os.environ.get("CARGO_HOME", str(runner_home / ".cargo"))
    )
    test_environment.setdefault(
        "RUSTUP_HOME", os.environ.get("RUSTUP_HOME", str(runner_home / ".rustup"))
    )
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
        raise RuntimeError(f"導入済み受入れRust試験に失敗しました: {name}")


def run_installed_smoke(
    package: Path,
    temp_root: Path,
    acceptance_root: Path,
    private_key_path: Path,
    update_package: Path,
    acceptance_scope: str = "update_lifecycle",
    codex_cli: Path | None = None,
    build_evidence_path: Path | None = None,
    bundle_root: Path | None = None,
    source_commit: str | None = None,
) -> None:
    needs_x11 = acceptance_scope not in {
        "wayland_rendering",
        "wayland_workspace_selection",
    }
    if (needs_x11 and not os.environ.get("DISPLAY")) or not os.environ.get(
        "DBUS_SESSION_BUS_ADDRESS"
    ):
        raise RuntimeError("対象acceptanceに必要な隔離displayまたはD-Bus sessionがありません")
    if temp_root.is_symlink() or not temp_root.is_dir():
        raise RuntimeError("run固有一時rootが通常directoryではありません")
    temp_root = temp_root.resolve(strict=True)
    package = _resolve_regular_package(package)
    update_package = _resolve_regular_package(update_package)
    if temp_root not in update_package.parents:
        raise RuntimeError("rollback fixture packageがrun固有一時root外です")
    if acceptance_scope not in {
        "update_lifecycle",
        "credential_vault",
        "mcp_tool_execution",
        "host_registration",
        "workspace_selection",
        "agent_task_supervision",
        "runtime_provenance",
        "wayland_rendering",
        "wayland_workspace_selection",
    }:
        raise RuntimeError("installed acceptance scopeが未対応です")
    if acceptance_scope == "runtime_provenance":
        if build_evidence_path is None or bundle_root is None or source_commit is None:
            raise RuntimeError("runtime provenanceにbuild evidence／bundle／source commitが必要です")
        for source in (build_evidence_path, bundle_root):
            resolved_source = source.resolve(strict=True)
            if temp_root not in resolved_source.parents or source.is_symlink():
                raise RuntimeError("runtime provenance入力がrun固有temporary root外です")
    if acceptance_scope in {"wayland_rendering", "wayland_workspace_selection"} and (
        source_commit is None or re.fullmatch(r"[0-9a-f]{40}", source_commit) is None
    ):
        raise RuntimeError("Wayland installed acceptanceに固定source commitがありません")
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
    if acceptance_scope == "agent_task_supervision":
        if codex_cli is None:
            raise RuntimeError("Agent Task acceptanceに固定Codex CLIが必要です")
        resolved_codex_cli = codex_cli.resolve(strict=True)
        if (
            resolved_codex_cli != codex_cli
            or codex_cli.is_symlink()
            or not codex_cli.is_file()
            or temp_root not in codex_cli.parents
            or not os.access(codex_cli, os.X_OK)
        ):
            raise RuntimeError("固定Codex CLIがrun固有一時root配下の実行fileではありません")
    else:
        resolved_codex_cli = None
    acceptance_root.mkdir(mode=0o700)
    home = acceptance_root / "home"
    runtime = _installed_acceptance_runtime_directory(
        acceptance_scope, acceptance_root, temp_root
    )
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
    installed_helper: Path | None = None
    responses_server: _LinuxAgentTaskResponsesServer | None = None
    task_group_cleanup_error: Exception | None = None
    task_execution_limit_ms = LINUX_AGENT_TASK_DEADLINE_TEST_LIMIT_MS
    app_log = acceptance_root / "installed-app.log"
    try:
        if acceptance_scope == "agent_task_supervision":
            if resolved_codex_cli is None:
                raise RuntimeError("Agent Task acceptanceのCodex CLI pathがありません")
            synthetic_root = acceptance_root / "D4Pocket-R2-E2E-SYNTHETIC"
            synthetic_root.mkdir(mode=0o700)
            (synthetic_root / "OWNER-APPROVED-SYNTHETIC.txt").write_text(
                "Run-owned test identity only; no provider credentials.\n",
                encoding="utf-8",
            )
            agent_task_codex_home = synthetic_root / "codex-home"
            agent_task_codex_home.mkdir(mode=0o700)
            responses_server = _LinuxAgentTaskResponsesServer()
            responses_server.start()
            environment.update(
                {
                    "GUI_SHELL_R2_E2E_RESPONSES_PORT": str(responses_server.port),
                    "GUI_SHELL_R2_E2E_CODEX_HOME": str(agent_task_codex_home),
                    "GUI_SHELL_R2_E2E_TASK_EXECUTION_LIMIT_MS": str(task_execution_limit_ms),
                }
            )
        if acceptance_scope == "credential_vault":
            secret_service_process = _start_isolated_secret_service(
                environment,
                acceptance_root / "keyring-control",
                acceptance_root / "gnome-keyring.log",
            )
            _run_rust_test(RUST_SECRET_SERVICE_TEST, environment, private_key_path)
        elif acceptance_scope in {
            "mcp_tool_execution",
            "host_registration",
            "workspace_selection",
            "agent_task_supervision",
            "runtime_provenance",
            "wayland_rendering",
            "wayland_workspace_selection",
        }:
            pass
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
        installed_helper = helper
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

        if acceptance_scope == "wayland_rendering":
            wayland_compositor, wayland_display = _start_wayland_compositor(
                runtime,
                environment,
                acceptance_root / "wayland-compositor.log",
                enable_output_capture=True,
            )
            wayland_environment = _wayland_launch_environment(
                environment, wayland_display
            )
            time.sleep(0.5)
            baseline_frame, baseline_png_sha256 = _capture_weston_frame(
                wayland_environment, acceptance_root / "wayland-frame-before"
            )

            with app_log.open("xb") as log:
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
                if not _visible_wayland_window(wayland_window):
                    raise RuntimeError(
                        "導入済みWayland GTK windowがAT-SPIで表示状態ではありません"
                    )
                live_runner = _observe_linux_runtime_executables(
                    {
                        "runner": (
                            runner,
                            hashlib.sha256(runner.read_bytes()).hexdigest(),
                        )
                    }
                )
                changed_pixels = 0
                rendered_png_sha256 = ""
                rendered_frame: tuple[int, int, bytes] | None = None
                required_changed_pixels = max(
                    4096, baseline_frame[0] * baseline_frame[1] // 100
                )
                deadline = time.monotonic() + 20
                sample_number = 0
                while time.monotonic() < deadline:
                    if not (_processes_using(product_paths) - preexisting):
                        raise RuntimeError(
                            "Weston出力取得前に導入済みWayland製品processが停止しました"
                        )
                    sample_number += 1
                    rendered_frame, rendered_png_sha256 = _capture_weston_frame(
                        wayland_environment,
                        acceptance_root / f"wayland-frame-after-{sample_number:02d}",
                    )
                    changed_pixels = _different_pixel_count(
                        baseline_frame, rendered_frame
                    )
                    if changed_pixels >= required_changed_pixels:
                        break
                    time.sleep(0.25)
                if rendered_frame is None or changed_pixels < required_changed_pixels:
                    raise RuntimeError(
                        "導入済みWayland surfaceのcompositor画面変化が受入れ下限未満です"
                    )

                running_wayland = _processes_using(product_paths) - preexisting
                _stop_owned_processes(running_wayland, wayland_process)
                if _processes_using(product_paths) - preexisting:
                    raise RuntimeError(
                        "Wayland描画受入れ後にinstalled product processが残留しました"
                    )
                if wayland_process.poll() is None:
                    wayland_process.wait(timeout=10)
                if wayland_process.returncode not in (
                    0,
                    -signal.SIGTERM,
                    -signal.SIGKILL,
                    None,
                ):
                    raise RuntimeError(
                        "Wayland描画受入れ後のinstalled launcher終了codeが不正です"
                    )

                _terminate_group(wayland_compositor)
                if wayland_compositor.poll() is None:
                    raise RuntimeError("Wayland描画用isolated compositorが停止しませんでした")
                wayland_compositor = None
                print(
                    json.dumps(
                        {
                            "acceptance_scope": acceptance_scope,
                            "evidence_class": "LIVE_RUNTIME",
                            "source_commit": source_commit,
                            "product_version": record["product_version"],
                            "installed_package_sha256": record["package_sha256"],
                            "display_server": "isolated Weston headless/Pixman",
                            "visible_window": "AT-SPI SHOWING",
                            "compositor_output": {
                                "width": rendered_frame[0],
                                "height": rendered_frame[1],
                                "changed_pixels": changed_pixels,
                                "required_changed_pixels": required_changed_pixels,
                                "baseline_png_sha256": baseline_png_sha256,
                                "rendered_png_sha256": rendered_png_sha256,
                                "image_files_retained": False,
                            },
                            "live_installed_runner": live_runner,
                            "owned_process_cleanup": "PASS",
                        },
                        ensure_ascii=False,
                        separators=(",", ":"),
                        sort_keys=True,
                    )
                )
            finally:
                if wayland_process is not None and wayland_process.poll() is None:
                    _stop_owned_processes(
                        _processes_using(product_paths) - preexisting,
                        wayland_process,
                    )
                if wayland_compositor is not None and wayland_compositor.poll() is None:
                    _terminate_group(wayland_compositor)
            return

        if acceptance_scope == "wayland_workspace_selection":
            wayland_compositor, wayland_display = _start_sway_compositor(
                runtime,
                environment,
                acceptance_root / "wayland-compositor.log",
            )
            wayland_environment = _wayland_launch_environment(
                environment, wayland_display
            )
            wayland_environment.pop("DISPLAY", None)
            wayland_environment.pop("XAUTHORITY", None)
            wayland_environment["XDG_CURRENT_DESKTOP"] = "sway"
            _publish_wayland_dbus_activation_environment(wayland_environment)
            with app_log.open("xb") as log:
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
                if not _visible_wayland_window(wayland_window):
                    raise RuntimeError(
                        "導入済みWayland GTK windowがAT-SPIで表示状態ではありません"
                    )
                running_wayland = _processes_using(product_paths) - preexisting
                if not running_wayland:
                    raise RuntimeError(
                        "Wayland UIのAT-SPI表示中にinstalled product processがありません"
                    )
                live_runner = _observe_linux_runtime_executables(
                    {
                        "runner": (
                            runner,
                            hashlib.sha256(runner.read_bytes()).hexdigest(),
                        )
                    }
                )
                workspace_evidence = _run_installed_workspace_selection_acceptance(
                    wayland_environment,
                    None,
                    audit_store=broker_state / "store",
                    acceptance_root=acceptance_root,
                    screenshot_before_selection_path=(
                        temp_root / "wayland-workspace-before-selection.png"
                    ),
                    screenshot_path=(
                        temp_root / "wayland-workspace-after-selection.png"
                    ),
                    inspector_failure_screenshot_path=(
                        temp_root / "wayland-workspace-inspector-failure.png"
                    ),
                    registration_only=True,
                )
                _stop_owned_processes(
                    _processes_using(product_paths) - preexisting,
                    wayland_process,
                )
                if _processes_using(product_paths) - preexisting:
                    raise RuntimeError(
                        "Wayland Workspace acceptance後にinstalled product processが残留しました"
                    )
                if wayland_process.poll() is None:
                    wayland_process.wait(timeout=10)
                if wayland_process.returncode not in (
                    0,
                    -signal.SIGTERM,
                    -signal.SIGKILL,
                    None,
                ):
                    raise RuntimeError(
                        "Wayland Workspace acceptance後のlauncher終了codeが不正です"
                    )
                _terminate_group(wayland_compositor)
                if wayland_compositor.poll() is None:
                    raise RuntimeError(
                        "Wayland Workspace acceptance用compositorが停止しませんでした"
                    )
                wayland_compositor = None
                print(
                    json.dumps(
                        {
                            "acceptance_scope": acceptance_scope,
                            "evidence_class": "LIVE_RUNTIME",
                            "source_commit": source_commit,
                            "product_version": record["product_version"],
                            "installed_package_sha256": record["package_sha256"],
                            "display_server": "隔離Sway headless/Pixman；X11 DISPLAY未設定",
                            "keyboard_input": "AT-SPI focus後にwtypeで実キー入力（30文字超は100ms間隔、その他40ms。実値不一致時は1回だけ160msで再入力）",
                            "visible_window": "AT-SPI SHOWING",
                            "workspace_selection": workspace_evidence,
                            "live_installed_runner": live_runner,
                            "owned_process_cleanup": "PASS",
                        },
                        ensure_ascii=False,
                        separators=(",", ":"),
                        sort_keys=True,
                    )
                )
            finally:
                if wayland_process is not None and wayland_process.poll() is None:
                    _stop_owned_processes(
                        _processes_using(product_paths) - preexisting,
                        wayland_process,
                    )
                if wayland_compositor is not None and wayland_compositor.poll() is None:
                    _terminate_group(wayland_compositor)
            return

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
                            f"gioによる起動に失敗しました: {desktop_process.returncode}\n{tail or '(no launcher output)'}"
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

            if acceptance_scope == "mcp_tool_execution":
                mcp_evidence = _run_installed_mcp_tool_acceptance(
                    environment,
                    window_id,
                    helper=helper,
                    audit_store=broker_state / "store",
                    temp_root=temp_root,
                    acceptance_root=acceptance_root,
                )
                current_window_id = _visible_product_window(environment)
                if current_window_id is None:
                    raise RuntimeError("MCP acceptance後にinstalled GTK windowが見つかりません")
                _request_x11_window_manager_close(current_window_id, environment)
                deadline = time.monotonic() + 60
                while time.monotonic() < deadline:
                    if not (_processes_using(product_paths) - preexisting):
                        break
                    time.sleep(0.2)
                if _processes_using(product_paths) - preexisting:
                    raise RuntimeError("MCP acceptance後の正常終了でowned product processが残留しています")
                if desktop_process.poll() is None:
                    desktop_process.wait(timeout=10)
                if desktop_process.returncode not in (0, None):
                    raise RuntimeError("MCP acceptance後のdesktop launcher終了codeが不正です")
                print(
                    json.dumps(
                        {
                            "acceptance_scope": acceptance_scope,
                            "installed_entry": str(entries[0]),
                            "product_version": record["product_version"],
                            "package_sha256": record["package_sha256"],
                            "mcp_tool_execution": mcp_evidence,
                            "installed_app_exit": "normal_window_close_and_owned_process_cleanup",
                        },
                        ensure_ascii=False,
                        separators=(",", ":"),
                    )
                )
                return

            if acceptance_scope == "host_registration":
                host_evidence = _run_installed_host_registration_acceptance(
                    environment,
                    window_id,
                    audit_store=broker_state / "store",
                    acceptance_root=acceptance_root,
                )
                current_window_id = _visible_product_window(environment)
                if current_window_id is None:
                    raise RuntimeError("Host registration後にinstalled GTK windowがありません")
                _request_x11_window_manager_close(current_window_id, environment)
                deadline = time.monotonic() + 60
                while time.monotonic() < deadline:
                    if not (_processes_using(product_paths) - preexisting):
                        break
                    time.sleep(0.2)
                if _processes_using(product_paths) - preexisting:
                    raise RuntimeError("Host registration後にowned product processが残留しています")
                if desktop_process.poll() is None:
                    desktop_process.wait(timeout=10)
                if desktop_process.returncode not in (0, None):
                    raise RuntimeError("Host registration後のdesktop launcher終了codeが不正です")
                durable_audit = _verify_live_audit(
                    broker_state / "store",
                    require_export=False,
                    require_host_registration=True,
                )
                print(
                    json.dumps(
                        {
                            "acceptance_scope": acceptance_scope,
                            "installed_entry": str(entries[0]),
                            "product_version": record["product_version"],
                            "package_sha256": record["package_sha256"],
                            "host_registration": host_evidence,
                            "durable_audit_after_normal_exit": durable_audit,
                            "installed_app_exit": "normal_window_close_and_owned_process_cleanup",
                        },
                        ensure_ascii=False,
                        separators=(",", ":"),
                    )
                )
                return

            if acceptance_scope == "workspace_selection":
                workspace_evidence = _run_installed_workspace_selection_acceptance(
                    environment,
                    window_id,
                    audit_store=broker_state / "store",
                    acceptance_root=acceptance_root,
                    screenshot_before_selection_path=(
                        temp_root / "workspace-chooser-before-selection.png"
                    ),
                    screenshot_path=(
                        temp_root / "workspace-chooser-after-selection.png"
                    ),
                    inspector_failure_screenshot_path=(
                        temp_root / "workspace-inspector-failure.png"
                    ),
                )
                current_window_id = _visible_product_window(environment)
                if current_window_id is None:
                    raise RuntimeError(
                        "Workspace acceptance後にinstalled GTK windowが見つかりません"
                    )
                _request_x11_window_manager_close(current_window_id, environment)
                deadline = time.monotonic() + 60
                while time.monotonic() < deadline:
                    if not (_processes_using(product_paths) - preexisting):
                        break
                    time.sleep(0.2)
                if _processes_using(product_paths) - preexisting:
                    raise RuntimeError(
                        "Workspace acceptance後の正常終了でowned product processが残留しています"
                    )
                if desktop_process.poll() is None:
                    desktop_process.wait(timeout=10)
                if desktop_process.returncode not in (0, None):
                    raise RuntimeError(
                        "Workspace acceptance後のdesktop launcher終了codeが不正です"
                    )
                audit_evidence = _verify_live_audit(
                    broker_state / "store",
                    require_export=False,
                    require_workspace=True,
                )
                print(
                    json.dumps(
                        {
                            "acceptance_scope": acceptance_scope,
                            "product_version": record["product_version"],
                            "package_sha256": record["package_sha256"],
                            "workspace_selection": workspace_evidence,
                            "live_audit": audit_evidence,
                            "installed_app_exit": "normal_window_close_and_owned_process_cleanup",
                        },
                        ensure_ascii=False,
                        separators=(",", ":"),
                    )
                )
                return

            if acceptance_scope == "agent_task_supervision":
                if resolved_codex_cli is None or responses_server is None:
                    raise RuntimeError("installed Agent Task acceptance fixtureの初期化が不足しています")
                agent_task_evidence = _run_installed_agent_task_acceptance(
                    environment,
                    window_id,
                    codex_cli=resolved_codex_cli,
                    responses_server=responses_server,
                    audit_store=broker_state / "store",
                    acceptance_root=acceptance_root,
                    task_execution_limit_ms=task_execution_limit_ms,
                )
                current_window_id = _visible_product_window(environment)
                if current_window_id is None:
                    raise RuntimeError("Agent Task acceptance後にinstalled GTK windowがありません")
                _request_x11_window_manager_close(current_window_id, environment)
                deadline = time.monotonic() + 60
                while time.monotonic() < deadline:
                    if not (_processes_using(product_paths) - preexisting):
                        break
                    time.sleep(0.2)
                if _processes_using(product_paths) - preexisting:
                    raise RuntimeError("Agent Task acceptance後にowned product processが残留しています")
                if desktop_process.poll() is None:
                    desktop_process.wait(timeout=10)
                if desktop_process.returncode not in (0, None):
                    raise RuntimeError("Agent Task acceptance後のdesktop launcher終了codeが不正です")
                durable_audit = _verify_live_audit(
                    broker_state / "store",
                    require_export=False,
                    require_agent_task=True,
                )
                print(
                    json.dumps(
                        {
                            "acceptance_scope": acceptance_scope,
                            "installed_entry": str(entries[0]),
                            "product_version": record["product_version"],
                            "package_sha256": record["package_sha256"],
                            "agent_task_supervision": agent_task_evidence,
                            "durable_audit_after_normal_exit": durable_audit,
                            "installed_app_exit": "normal_window_close_and_owned_process_cleanup",
                        },
                        ensure_ascii=False,
                        separators=(",", ":"),
                    )
                )
                return

            if acceptance_scope == "runtime_provenance":
                if source_commit is None or build_evidence_path is None or bundle_root is None:
                    raise RuntimeError("runtime provenance入力が初期化されていません")
                provenance = _verify_linux_installed_artifact_provenance(
                    build_evidence_path=build_evidence_path,
                    bundle_root=bundle_root,
                    package_path=package,
                    source_commit=source_commit,
                    product_root=product_root,
                    stage=stage,
                    xdg_data_home=xdg_data_home,
                    xdg_state_home=xdg_state_home,
                    active_record=record,
                    app_id=APP_ID,
                    audit_store_id=AUDIT_ID,
                )
                provenance["live_processes"] = _observe_linux_runtime_executables(
                    {
                        "runner": (runner, provenance["component_sha256"]["runner"]),
                        "broker": (helper, provenance["component_sha256"]["broker"]),
                    }
                )
                current_window_id = _visible_product_window(environment)
                if current_window_id is None:
                    raise RuntimeError("runtime provenance受入れ中にinstalled GTK windowがありません")
                _request_x11_window_manager_close(current_window_id, environment)
                deadline = time.monotonic() + 60
                while time.monotonic() < deadline:
                    if not (_processes_using(product_paths) - preexisting):
                        break
                    time.sleep(0.2)
                if _processes_using(product_paths) - preexisting:
                    raise RuntimeError("runtime provenance受入れ後にowned product processが残留しています")
                if desktop_process.poll() is None:
                    desktop_process.wait(timeout=10)
                if desktop_process.returncode not in (0, None):
                    raise RuntimeError("runtime provenance受入れ後のdesktop launcher終了codeが不正です")
                provenance["audit_store_observation"] = _observe_linux_audit_store(
                    broker_state / "store"
                )
                provenance["installed_app_exit"] = (
                    "normal_window_close_and_owned_process_cleanup"
                )
                print(
                    json.dumps(
                        {"acceptance_scope": acceptance_scope, **provenance},
                        ensure_ascii=False,
                        separators=(",", ":"),
                        sort_keys=True,
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
                "一覧取得直前の通信状態="
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
                    f"{error}; 一覧取得の通信診断="
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
                raise RuntimeError(f"gioによる起動が想定外に終了しました: {desktop_process.returncode}")

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
                    raise RuntimeError("導入済みWayland GTK最上位windowが表示状態ではありません")
            except RuntimeError:
                raise
            except Exception as error:
                raise RuntimeError(
                    "導入済みWayland GTK windowの表示状態を取得できません"
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
                    "wayland_installed_window": "AT-SPI表示確認済み; 所有process群停止済み; 画面描画は未確認",
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
        primary_acceptance_error = sys.exc_info()[1]
        if acceptance_scope == "agent_task_supervision":
            try:
                _cleanup_linux_agent_task_process_groups(acceptance_root)
            except Exception as error:
                task_group_cleanup_error = error
        if uninstall_groups:
            _stop_owned_groups(uninstall_groups)
        if product_paths:
            _stop_owned_processes(_processes_using(product_paths) - preexisting, desktop_process)
        elif desktop_process is not None and desktop_process.poll() is None:
            _stop_owned_processes(set(), desktop_process)
        if responses_server is not None:
            responses_server.close()
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
        if (
            acceptance_scope == "mcp_tool_execution"
            and installed_helper is not None
            and acceptance_root.exists()
            and (acceptance_root / "mcp-workspace").is_dir()
        ):
            _cleanup_mcp_fixture_group(
                acceptance_root / "mcp-workspace",
                helper=installed_helper,
                fixture=temp_root / "linux-installed-mcp-server.py",
            )
        if task_group_cleanup_error is not None:
            primary_summary: dict[str, object] = {
                "error_type": (
                    type(primary_acceptance_error).__name__
                    if primary_acceptance_error is not None
                    else "none"
                )
            }
            primary_message = str(primary_acceptance_error or "")
            report_failure_prefix = (
                "Agent Taskのsecret／Workspace境界またはscratch fixtureが不成立です: "
            )
            if primary_message.startswith(report_failure_prefix):
                try:
                    report_diagnostics = json.loads(
                        primary_message[len(report_failure_prefix) :]
                    )
                except json.JSONDecodeError:
                    report_diagnostics = None
                if isinstance(report_diagnostics, dict):
                    allowed_check_names = {
                        "secret_and_outside_reads_denied",
                        "secret_and_outside_writes_denied",
                        "workspace_write",
                        "scratch_marker",
                        "scratch_workspace_bound",
                    }
                    primary_summary["failure_stage"] = "fixture_report_validation"
                    primary_summary["diagnostics"] = {
                        "report_fields_match": report_diagnostics.get(
                            "report_fields_match"
                        )
                        is True,
                        "missing_fields": [
                            field
                            for field in report_diagnostics.get("missing_fields", [])
                            if isinstance(field, str)
                            and field
                            in {
                                "mode",
                                "secret_and_outside_reads_denied",
                                "secret_and_outside_writes_denied",
                                "workspace_write",
                                "scratch_marker",
                                "scratch_workspace_bound",
                                "scratch_name",
                            }
                        ],
                        "unexpected_fields": [
                            field
                            for field in report_diagnostics.get("unexpected_fields", [])
                            if isinstance(field, str)
                            and re.fullmatch(r"[A-Za-z_]{1,64}", field)
                        ],
                        "failed_checks": [
                            name
                            for name in report_diagnostics.get("failed_checks", [])
                            if isinstance(name, str) and name in allowed_check_names
                        ],
                        "mode_matches": report_diagnostics.get("mode_matches") is True,
                        "scratch_name_matches": report_diagnostics.get(
                            "scratch_name_matches"
                        )
                        is True,
                    }
            cleanup_summary: dict[str, object] = {
                "error_type": type(task_group_cleanup_error).__name__,
                "failure_code": _linux_agent_task_cleanup_failure_code(
                    task_group_cleanup_error
                ),
            }
            cleanup_message = str(task_group_cleanup_error)
            cleanup_prefix = (
                "Agent Task process group cleanupで一つ以上の失敗を確認しました: "
            )
            if cleanup_message.startswith(cleanup_prefix):
                try:
                    parsed_cleanup = json.loads(cleanup_message[len(cleanup_prefix) :])
                except json.JSONDecodeError:
                    parsed_cleanup = None
                if isinstance(parsed_cleanup, dict):
                    scenario = parsed_cleanup.get("scenario")
                    error_type = parsed_cleanup.get("error_type")
                    failure_code = parsed_cleanup.get("failure_code")
                    if scenario in {"cancel", "deadline"}:
                        cleanup_summary["scenario"] = scenario
                    if isinstance(error_type, str) and re.fullmatch(
                        r"[A-Za-z]{1,64}", error_type
                    ):
                        cleanup_summary["error_type"] = error_type
                    if isinstance(failure_code, str) and re.fullmatch(
                        r"[a-z0-9_]{1,64}", failure_code
                    ):
                        cleanup_summary["failure_code"] = failure_code
            raise RuntimeError(
                "Agent Task acceptance後のprocess group cleanupが成立しません: "
                + json.dumps(
                    {
                        "primary_failure": primary_summary,
                        "cleanup_failure": cleanup_summary,
                    },
                    ensure_ascii=False,
                    sort_keys=True,
                )
            ) from task_group_cleanup_error
        cleanup_operation = "acceptance_root_validation"
        try:
            if acceptance_scope in {
                "workspace_selection",
                "wayland_workspace_selection",
            }:
                cleanup_operation = "xdg_document_portal_mounts"
                _cleanup_xdg_document_portal_mounts(
                    runtime, temp_root, environment
                )
            if acceptance_scope == "wayland_workspace_selection":
                cleanup_operation = "sway_runtime_removal"
                _cleanup_owned_sway_runtime(runtime, temp_root)
            if acceptance_root.exists():
                cleanup_operation = "acceptance_root_removal"
                resolved = acceptance_root.resolve(strict=True)
                if (
                    resolved == temp_root
                    or temp_root not in resolved.parents
                    or acceptance_root.is_symlink()
                ):
                    raise RuntimeError(
                        "cleanup対象がrun固有test root外またはsymlinkです"
                    )
                shutil.rmtree(resolved)
            if acceptance_root.exists() or acceptance_root.is_symlink():
                cleanup_operation = "acceptance_root_verification"
                raise RuntimeError("installed acceptance test rootをcleanupできません")
        except Exception as cleanup_error:
            if primary_acceptance_error is not None:
                cleanup_summary: dict[str, object] = {
                    "primary_error_type": type(primary_acceptance_error).__name__,
                    "primary_call_path": _safe_exception_call_path(
                        primary_acceptance_error
                    ),
                    "cleanup_error_type": type(cleanup_error).__name__,
                    "cleanup_operation": cleanup_operation,
                }
                cleanup_errno = getattr(cleanup_error, "errno", None)
                if isinstance(cleanup_errno, int):
                    cleanup_summary["cleanup_errno"] = cleanup_errno
                raise RuntimeError(
                    "installed acceptanceのprimary failureとcleanup failure: "
                    + json.dumps(cleanup_summary, ensure_ascii=True, sort_keys=True)
                ) from cleanup_error
            raise


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package", type=Path, required=True)
    parser.add_argument("--temp-root", type=Path, required=True)
    parser.add_argument("--acceptance-root", type=Path, required=True)
    parser.add_argument("--private-key", type=Path, required=True)
    parser.add_argument("--update-package", type=Path, required=True)
    parser.add_argument(
        "--acceptance-scope",
        choices=(
            "update_lifecycle",
            "credential_vault",
            "mcp_tool_execution",
            "host_registration",
            "workspace_selection",
            "agent_task_supervision",
            "runtime_provenance",
            "wayland_rendering",
            "wayland_workspace_selection",
        ),
        default="update_lifecycle",
    )
    parser.add_argument("--codex-cli", type=Path)
    parser.add_argument("--build-evidence", type=Path)
    parser.add_argument("--bundle-root", type=Path)
    parser.add_argument("--source-commit")
    args = parser.parse_args()
    try:
        run_installed_smoke(
            args.package,
            args.temp_root,
            args.acceptance_root,
            args.private_key,
            args.update_package,
            args.acceptance_scope,
            args.codex_cli,
            args.build_evidence,
            args.bundle_root,
            args.source_commit,
        )
    except Exception as error:
        call_path = _safe_exception_call_path(error)
        print(
            "Linux installed product GTK受入れ 不合格: "
            f"{type(error).__name__}; call_path={call_path}; {error}",
            file=sys.stderr,
        )
        return 1
    return 0


def _safe_exception_call_path(error: BaseException) -> str:
    return " -> ".join(
        f"{frame.name}:{frame.lineno}"
        for frame in traceback.extract_tb(error.__traceback__)[-8:]
    )


if __name__ == "__main__":
    raise SystemExit(main())
