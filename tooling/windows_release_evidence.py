from __future__ import annotations

import argparse
import base64
import hashlib
import ntpath
import subprocess
import sys
import json
import re
import math
from dataclasses import dataclass
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))
DEFAULT_EVIDENCE_PATH = ROOT / "release_evidence" / "windows_installed_smoke.json"
SHA256_RE = re.compile(r"^sha256:[0-9a-f]{64}$")
SOURCE_COMMIT_RE = re.compile(r"^[0-9a-f]{40}$")
REQUIRED_VISIBLE_SURFACES = {"Dashboard", "NavigationRail", "Runtime Status", "Invariant Status"}
REQUIRED_SETUP_CHECKS = {
    "setup_doctor.ran_from_installed_app_path",
    "setup_doctor.runtime_connection",
    "setup_doctor.authority_boundary",
    "setup_doctor.network_public_bind",
    "setup_doctor.recovery_instruction",
    "setup_doctor.audit_storage",
    "setup_doctor.config_created",
}
REQUIRED_BROKER_TRUE_FIELDS = {
    "helper_exe_exists",
    "session_file_created",
    "session_file_removed_after_collection",
    "normal_endpoint_credential_role_verified",
    "restricted_loopback_bind",
    "authenticated_ipc_connection",
    "durable_store_ready",
    "normal_ipc_agent_task_workspace_permission_denied",
    "normal_ipc_agent_task_owner_approval_denied",
    "restart_replay_rejected",
    "fresh_health_after_restart",
    "crash_fail_closed",
}
AGGREGATE_SURFACE_TEXT = "GUI Shell Dashboard NavigationRail Runtime Status Invariant Status"
BASE_REQUIRED_EVIDENCE_BUNDLE_KINDS = {
    "setup_doctor",
    "setup_doctor_operator_readability",
    "broker_smoke",
    "broker_lifecycle_audit",
    "first_run_configuration",
    "visible_surfaces",
    "runtime_assertions",
}
AUDIT_REQUIRED_EVIDENCE_BUNDLE_KINDS = {
    "audit_anchor_external_tamper_evidence",
}
BASE_REQUIRED_FIELD_PROVENANCE = {
    "artifact": ("directly_measured", {"EXTERNAL_EVIDENCE"}),
    "first_run.process": ("directly_measured", {"LIVE_RUNTIME", "EXTERNAL_EVIDENCE"}),
    "first_run.visible_surfaces": ("directly_measured", {"LIVE_RUNTIME", "EXTERNAL_EVIDENCE"}),
    "first_run.broker_lifecycle_audit": ("directly_measured", {"LIVE_RUNTIME", "EXTERNAL_EVIDENCE"}),
    "first_run.broker_health_request": ("directly_measured", {"LIVE_RUNTIME"}),
    "first_run.config_audit": ("directly_measured", {"LIVE_RUNTIME", "EXTERNAL_EVIDENCE"}),
    "first_run.installer_authority_boundary": ("static_assertion", {"CONFIG"}),
    "setup_doctor": ("product_export", {"LIVE_RUNTIME", "EXTERNAL_EVIDENCE"}),
    "broker.ipc_restart_crash": ("directly_measured", {"LIVE_RUNTIME", "EXTERNAL_EVIDENCE"}),
    "release_runtime_assertions": ("static_assertion", {"CONFIG", "FIXTURE"}),
}
AUDIT_REQUIRED_FIELD_PROVENANCE = {
    "audit_anchor.external_tamper_evidence": (
        "directly_measured",
        {"LIVE_RUNTIME", "EXTERNAL_EVIDENCE"},
    ),
}
LEGACY_FIXED_INSTALL_ROOT_SUFFIX = "\\gui-shell\\installed"


@dataclass(frozen=True)
class EvidenceResult:
    name: str
    status: str
    classification: str
    blocks_release: str
    reason: str
    required_action: str


def _failed(name: str, reason: str, required_action: str) -> EvidenceResult:
    return EvidenceResult(name, "failed", "release_blocker", "yes", reason, required_action)


def _passed(name: str, reason: str, required_action: str) -> EvidenceResult:
    return EvidenceResult(name, "passed", "none", "no", reason, required_action)


def _get(data: dict[str, Any], dotted: str) -> Any:
    value: Any = data
    for part in dotted.split("."):
        if not isinstance(value, dict) or part not in value:
            return None
        value = value[part]
    return value


def _is_false(data: dict[str, Any], dotted: str) -> bool:
    return _get(data, dotted) is False


def _is_true(data: dict[str, Any], dotted: str) -> bool:
    return _get(data, dotted) is True


def _is_sha256_tag(value: Any) -> bool:
    return bool(SHA256_RE.match(str(value or "")))


def _hash_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return f"sha256:{digest.hexdigest()}"


def _normalised_text(value: Any) -> str:
    return re.sub(r"\s+", " ", str(value or "")).strip()


SURFACE_NAMES = {
    "Dashboard": ("概要", "gui_shell.surface.dashboard"),
    "NavigationRail": ("ナビゲーション", "gui_shell.surface.navigation_rail"),
    "Runtime Status": ("実行系状態", "gui_shell.surface.runtime_status"),
    "Invariant Status": ("不変条件状態", "gui_shell.surface.invariant_status"),
}


def _contains_surface_label(value: Any, label: str) -> bool:
    japanese, identifier = SURFACE_NAMES[label]
    normalized = _normalised_text(value)
    return normalized in (japanese, japanese + " " + japanese, identifier) or bool(
        re.search(r"(?<![a-z0-9_.])" + re.escape(label) + r"(?![a-z0-9_.])", normalized, flags=re.IGNORECASE))


def _contains_all_required_surfaces(value: Any) -> bool:
    return all(_contains_surface_label(value, label) for label in REQUIRED_VISIBLE_SURFACES)


def _finite_coordinate(value: Any) -> bool:
    try:
        return type(value) in (int, float) and math.isfinite(value)
    except OverflowError:
        return False


def _surface_tree_errors(tree: dict[str, Any], matches: dict[str, Any]) -> list[str]:
    """収集済みtreeの同一性と表示領域を検証する。pixelや遮蔽の証明ではない。"""
    nodes = tree.get("observed_elements")
    if not isinstance(nodes, list) or not 1 <= len(nodes) <= 10000:
        return ["可視surfaceの観測要素数が不正"]
    if type(tree.get("observed_element_count")) is not int or tree["observed_element_count"] != len(nodes):
        return ["可視surfaceの観測件数が不一致"]
    required = {"element_key", "runtime_id", "parent_runtime_id", "name", "automation_id",
                "control_type", "class_name", "framework_id", "supported_patterns",
                "is_root", "is_native_container"}
    by_id: dict[str, dict[str, Any]] = {}
    by_key: dict[str, dict[str, Any]] = {}
    for node in nodes:
        if not isinstance(node, dict) or not required <= node.keys():
            return ["可視surfaceの観測要素に診断項目が欠落"]
        key, rid = node["element_key"], node["runtime_id"]
        if not isinstance(key, str) or not key or not isinstance(rid, str) or not rid:
            return ["可視surfaceの観測識別子が不正"]
        if key in by_key or rid in by_id:
            return ["可視surfaceの観測識別子が重複"]
        if not isinstance(node["parent_runtime_id"], str):
            return ["可視surfaceの親識別子が不正"]
        by_key[key], by_id[rid] = node, node
    roots = [n for n in nodes if n["is_root"] is True and n["parent_runtime_id"] == ""]
    if len(roots) != 1:
        return ["可視surfaceのrootが一意でない"]
    root = roots[0]
    expected_edges = []
    for node in nodes:
        if node is not root:
            if node["is_root"] is not False or node["parent_runtime_id"] not in by_id:
                return ["可視surfaceの親が欠落またはroot分類が不正"]
            expected_edges.append({"child_runtime_id": node["runtime_id"],
                                   "parent_runtime_id": node["parent_runtime_id"],
                                   "child_element_key": node["element_key"]})
        seen: set[str] = set()
        cursor = node
        while cursor is not root:
            if cursor["runtime_id"] in seen or len(seen) >= 128:
                return ["可視surfaceの親子関係が循環または深すぎる"]
            seen.add(cursor["runtime_id"])
            cursor = by_id.get(cursor["parent_runtime_id"])
            if cursor is None:
                return ["可視surfaceがrootへ到達しない"]
    edges = tree.get("tree_edges")
    if not isinstance(edges, list) or len(edges) != len(expected_edges) or any(e not in edges for e in expected_edges):
        return ["可視surfaceの親子edgeが観測要素と不一致"]

    errors = []
    projection = ("name", "automation_id", "control_type", "class_name", "framework_id", "is_root", "is_native_container")
    for label, match in matches.items():
        if not isinstance(match, dict):
            continue
        node = by_key.get(str(match.get("element_key", "")))
        if node is None or any(node.get(k) != match.get(k) for k in projection):
            errors.append(f"{label} のmatchが実観測要素と不一致")
            continue
        if node["is_root"] is not False or node["is_native_container"] is not False:
            errors.append(f"{label} はrootまたはnative containerを流用している")
            continue
        intersection = None
        cursor = node
        while True:
            rect = cursor.get("bounding_rectangle")
            values = [rect.get(k) for k in ("x", "y", "width", "height")] if isinstance(rect, dict) else []
            if (cursor.get("is_offscreen") is not False or len(values) != 4
                    or any(not _finite_coordinate(v) for v in values)
                    or values[2] <= 0 or values[3] <= 0):
                errors.append(f"{label} の要素または親の表示状態・座標を確認できない")
                break
            x, y, w, h = values
            bounds = (x, y, x + w, y + h)
            if not all(math.isfinite(v) for v in bounds):
                errors.append(f"{label} の矩形端点が非有限")
                break
            intersection = bounds if intersection is None else (
                max(intersection[0], bounds[0]), max(intersection[1], bounds[1]),
                min(intersection[2], bounds[2]), min(intersection[3], bounds[3]))
            if intersection[2] <= intersection[0] or intersection[3] <= intersection[1]:
                errors.append(f"{label} が表示領域外")
                break
            if cursor is root:
                break
            cursor = by_id[cursor["parent_runtime_id"]]
    return errors


def _validate_surface_match_evidence(surface_evidence: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    if surface_evidence.get("aggregate_surface_shortcut_detected") is not False:
        errors.append("可視 surface 証拠が集約 surface shortcut を検出した、または不使用を確認できなかった")
    if surface_evidence.get("surface_match_requirements_met") is not True:
        errors.append("可視 surface 証拠で個別 surface の一致要件を確認できなかった")

    surface_matches = surface_evidence.get("surface_matches")
    if not isinstance(surface_matches, dict):
        errors.append("可視 surface 証拠に surface_matches がない")
        return errors

    matched_element_keys: list[str] = []
    aggregate_text = _normalised_text(AGGREGATE_SURFACE_TEXT).casefold()
    for label in sorted(REQUIRED_VISIBLE_SURFACES):
        match = surface_matches.get(label)
        if not isinstance(match, dict):
            errors.append(f"{label} の surface 一致証拠がない")
            continue
        if match.get("matched") is not True:
            errors.append(f"{label} の surface match は true でなければならない")
        name = str(match.get("name") or "")
        automation_id = str(match.get("automation_id") or "")
        control_type = str(match.get("control_type") or "")
        element_key = str(match.get("element_key") or "")
        if not element_key:
            errors.append(f"{label} の surface match に element_key がない")
        else:
            matched_element_keys.append(element_key)
        if not control_type:
            errors.append(f"{label} の surface match に control_type がない")
        if not (_contains_surface_label(name, label) or _contains_surface_label(automation_id, label)):
            errors.append(f"{label} の surface match にある name または automation_id は surface label を含まなければならない")

        element_text = _normalised_text(f"{name} {automation_id}")
        if _contains_all_required_surfaces(element_text):
            errors.append(f"{label} の surface match が必須 label をすべて含む単一の集約要素を使用している")
        if aggregate_text and aggregate_text in element_text.casefold():
            errors.append(f"{label} の surface match が禁止された集約 native surface title を使用している")

    if (
        len(matched_element_keys) == len(REQUIRED_VISIBLE_SURFACES)
        and len(set(matched_element_keys)) == 1
    ):
        errors.append("必須 surface のすべてが単一の automation 要素に依存している")
    diagnostic_tree = surface_evidence.get("diagnostic_tree")
    if isinstance(diagnostic_tree, dict) and diagnostic_tree.get("mode") == "flutter_dart_surface_semantics_runtime_export":
        errors.append("Flutter build registryのINTERNAL_STATEを可視surfaceのLIVE_RUNTIME証拠として受理しない")
    if not isinstance(diagnostic_tree, dict):
        errors.append("可視 surface 証拠に diagnostic_tree がない")
    else:
        if surface_evidence.get("source") == "uiautomation":
            if diagnostic_tree.get("mode") != "full_uiautomation_tree_projection":
                errors.append("可視surfaceの観測方式がWindows UI Automation treeではない")
            if diagnostic_tree.get("tree_view") != "control":
                errors.append("可視surfaceの観測treeがControl Viewではない")
            if diagnostic_tree.get("capture_limit") != "none":
                errors.append("可視surfaceのUI Automation treeが上限または重複IDで不完全である")
        errors.extend(_surface_tree_errors(diagnostic_tree, surface_matches))
    return errors


def _validate_field_provenance(
    data: dict[str, Any],
    required_field_provenance: dict[str, tuple[str, set[str]]] = BASE_REQUIRED_FIELD_PROVENANCE,
    *,
    check_unsupported_claims: bool = True,
) -> list[str]:
    errors: list[str] = []
    provenance = data.get("field_provenance")
    if not isinstance(provenance, dict):
        return ["field_provenance object がない"]
    for group, (source_type, evidence_classes) in required_field_provenance.items():
        entry = provenance.get(group)
        if not isinstance(entry, dict):
            errors.append(f"field_provenance.{group} がない")
            continue
        if entry.get("source_type") != source_type:
            errors.append(f"field_provenance.{group}.source_type は {source_type} でなければならない")
        observed_classes = entry.get("evidence_class")
        if isinstance(observed_classes, str):
            observed = {observed_classes}
        elif isinstance(observed_classes, list):
            observed = {item for item in observed_classes if isinstance(item, str)}
        else:
            observed = set()
        if not observed or observed.isdisjoint(evidence_classes):
            errors.append(
                f"field_provenance.{group}.evidence_class は {', '.join(sorted(evidence_classes))} のいずれかを含まなければならない"
            )
        if entry.get("formal_release_input") is not True:
            errors.append(f"field_provenance.{group}.formal_release_input は true でなければならない")
    if check_unsupported_claims:
        unsupported = provenance.get("unsupported_claims")
        if unsupported not in (None, []):
            errors.append("厳格な Windows 証拠では field_provenance.unsupported_claims は空でなければならない")
    return errors


def _path_contains_run_id(path_value: Any, run_id: str) -> bool:
    path = str(path_value or "").replace("/", "\\").casefold()
    return bool(run_id) and run_id.casefold() in path


def _windows_path_is_within(path_value: Any, root_value: Any) -> bool:
    path = ntpath.normcase(ntpath.normpath(str(path_value or "")))
    root = ntpath.normcase(ntpath.normpath(str(root_value or "")))
    if not path or not root:
        return False
    try:
        return ntpath.commonpath((path, root)) == root
    except ValueError:
        return False


def validate_provenance_and_isolation(data: dict[str, Any], path: Path = DEFAULT_EVIDENCE_PATH) -> EvidenceResult:
    errors: list[str] = []
    provenance = data.get("provenance")
    if not isinstance(provenance, dict):
        errors.append("provenance object がない")
    else:
        run_id = str(provenance.get("run_id") or "")
        if not run_id:
            errors.append("provenance.run_id がない")
        source_commit = str(provenance.get("source_commit") or "")
        if not SOURCE_COMMIT_RE.match(source_commit):
            errors.append("provenance.source_commit は40文字の commit SHA でなければならない")
        if provenance.get("source_worktree_clean") is not True:
            errors.append("provenance.source_worktree_clean は true でなければならない")
        if str(provenance.get("source_status_porcelain") or "") != "":
            errors.append("provenance.source_status_porcelain は空でなければならない")
        for field in ("build_command", "build_timestamp", "staged_manifest_path"):
            if not provenance.get(field):
                errors.append(f"provenance.{field} がない")
        for field in ("installed_manifest_sha256", "app_artifact_sha256", "launcher_artifact_sha256", "broker_artifact_sha256", "evidence_bundle_sha256"):
            if not _is_sha256_tag(provenance.get(field)):
                errors.append(f"provenance.{field} には sha256 tag が必要である")
        artifact_hash = _get(data, "artifact.sha256")
        if _is_sha256_tag(artifact_hash) and provenance.get("app_artifact_sha256") != artifact_hash:
            errors.append("provenance.app_artifact_sha256 は artifact.sha256 と一致しなければならない")
        launcher_hash = _get(data, "artifact.desktop_launcher_sha256")
        if _is_sha256_tag(launcher_hash) and provenance.get("launcher_artifact_sha256") != launcher_hash:
            errors.append("provenance.launcher_artifact_sha256 は artifact.desktop_launcher_sha256 と一致しなければならない")

        isolation = provenance.get("isolation")
        if not isinstance(isolation, dict):
            errors.append("provenance.isolation object がない")
        else:
            if isolation.get("uses_shared_fixed_install_root") is not False:
                errors.append("provenance.isolation.uses_shared_fixed_install_root は false でなければならない")
            path_fields = [
                "isolated_install_root",
                "isolated_runtime_dir",
                "isolated_store_dir",
                "isolated_audit_dir",
                "isolated_localappdata",
            ]
            for field in path_fields:
                value = isolation.get(field)
                if not value:
                    errors.append(f"provenance.isolation.{field} がない")
                elif not _path_contains_run_id(value, run_id):
                    errors.append(f"provenance.isolation.{field} は run_id を含まなければならない")
            install_root = str(isolation.get("isolated_install_root") or "").replace("/", "\\").casefold()
            if install_root.endswith(LEGACY_FIXED_INSTALL_ROOT_SUFFIX):
                errors.append("provenance.isolation.isolated_install_root が旧式の共有固定 install root を使用している")
            if isolation.get("separate_windows_user_profile") is not True:
                errors.append("provenance.isolation.separate_windows_user_profile を実証できなかった")
            local_appdata = isolation.get("isolated_localappdata")
            runtime_dir = isolation.get("isolated_runtime_dir")
            config_dir = isolation.get("isolated_config_dir")
            runtime_identity = isolation.get("runtime_identity")
            if not _windows_path_is_within(runtime_dir, local_appdata):
                errors.append("起動器runtimeが実行user profileのisolated LOCALAPPDATA外にある")
            if not isinstance(runtime_identity, dict):
                errors.append("provenance.isolation.runtime_identity object がない")
            else:
                identity_kind = runtime_identity.get("kind")
                runtime_normalized = str(runtime_dir or "").replace("/", "\\").rstrip("\\").casefold()
                if runtime_identity.get("evidence_class") != "CONFIG":
                    errors.append("起動器runtime identityはCONFIG証拠として分類しなければならない")
                if identity_kind == "gui_shell":
                    if (
                        runtime_identity.get("app_id") is not None
                        or runtime_identity.get("audit_store_id") is not None
                        or runtime_identity.get("product_manifest_path") is not None
                        or runtime_identity.get("product_manifest_sha256") is not None
                        or not runtime_normalized.endswith("\\gui-shell\\broker\\desktop")
                    ):
                        errors.append("generic GUI Shell runtime identityと実runtime pathが一致しない")
                elif identity_kind == "d4_pocket_product":
                    app_id = runtime_identity.get("app_id")
                    audit_store_id = runtime_identity.get("audit_store_id")
                    product_manifest_path = str(runtime_identity.get("product_manifest_path") or "")
                    staged_install_root = ntpath.dirname(str(provenance.get("staged_manifest_path") or ""))
                    expected_product_manifest_path = ntpath.join(staged_install_root, "product_manifest.json")
                    if (
                        not isinstance(app_id, str)
                        or re.fullmatch(r"d4-pocket-app-[0-9a-f]{32}", app_id) is None
                        or not isinstance(audit_store_id, str)
                        or re.fullmatch(r"audit-store-[0-9a-f]{32}", audit_store_id) is None
                        or not runtime_normalized.endswith(
                            f"\\d4pocket\\apps\\{app_id.casefold()}\\stores\\{audit_store_id.casefold()}"
                        )
                        or not _is_sha256_tag(runtime_identity.get("product_manifest_sha256"))
                        or ntpath.normcase(ntpath.normpath(product_manifest_path))
                        != ntpath.normcase(ntpath.normpath(expected_product_manifest_path))
                    ):
                        errors.append("D4 Pocket runtime identity、installed Product Manifestまたは実runtime pathが一致しない")
                else:
                    errors.append("起動器runtime identity kindが未知です")
                first_run_identity = _get(data, "first_run.launcher_runtime_identity")
                if not isinstance(first_run_identity, dict) or any(
                    first_run_identity.get(key) != runtime_identity.get(key)
                    for key in (
                        "kind",
                        "app_id",
                        "audit_store_id",
                        "product_manifest_path",
                        "product_manifest_sha256",
                        "evidence_class",
                    )
                ):
                    errors.append("first_run.runtime identityがprovenanceと一致しない")
            if config_dir and not _windows_path_is_within(config_dir, local_appdata):
                errors.append("製品config directoryが実行user profileのisolated LOCALAPPDATA外にある")
            config_path = _get(data, "first_run.config_path")
            if config_path and not _windows_path_is_within(config_path, local_appdata):
                errors.append("初回config pathが実行user profileのisolated LOCALAPPDATA外にある")
            if str(_get(data, "first_run.launcher_runtime_dir") or "") != str(isolation.get("isolated_runtime_dir") or ""):
                errors.append("実起動器runtime pathがprovenance.isolationの実runtime pathと一致しない")
            if not _windows_path_is_within(_get(data, "first_run.broker_endpoint_file"), isolation.get("isolated_runtime_dir")):
                errors.append("Broker endpoint pathが起動器runtime外にある")
            if not _windows_path_is_within(_get(data, "first_run.broker_lifecycle_audit.path"), isolation.get("isolated_store_dir")):
                errors.append("Broker lifecycle Audit pathが起動器の実Store外にある")

        bundle_files = provenance.get("evidence_bundle_files")
        if not isinstance(bundle_files, list) or not bundle_files:
            errors.append("provenance.evidence_bundle_files がない")
        else:
            kinds = set()
            for index, item in enumerate(bundle_files):
                if not isinstance(item, dict):
                    errors.append(f"provenance.evidence_bundle_files[{index}] は object でなければならない")
                    continue
                kind = item.get("kind")
                if isinstance(kind, str):
                    kinds.add(kind)
                if not item.get("path"):
                    errors.append(f"provenance.evidence_bundle_files[{index}].path がない")
                if not _is_sha256_tag(item.get("sha256")):
                    errors.append(f"provenance.evidence_bundle_files[{index}].sha256 には sha256 tag が必要である")
            missing = BASE_REQUIRED_EVIDENCE_BUNDLE_KINDS - kinds
            if missing:
                errors.append(f"provenance.evidence_bundle_files に次の kind がない: {', '.join(sorted(missing))}")
            runtime_identity = isolation.get("runtime_identity")
            product_manifest_records = [
                item for item in bundle_files
                if isinstance(item, dict) and item.get("kind") == "product_manifest"
            ]
            if isinstance(runtime_identity, dict) and runtime_identity.get("kind") == "d4_pocket_product":
                if (
                    len(product_manifest_records) != 1
                    or product_manifest_records[0].get("path") != runtime_identity.get("product_manifest_path")
                    or product_manifest_records[0].get("sha256") != runtime_identity.get("product_manifest_sha256")
                ):
                    errors.append("D4 Pocket Product Manifestが実行器runtime identityへhash結合されていない")
            elif product_manifest_records:
                errors.append("generic GUI Shell evidenceに未宣言のProduct Manifestが含まれる")

        errors.extend(_validate_field_provenance(data))

        if path.exists():
            actual_hash = _hash_file(path)
            if _is_sha256_tag(provenance.get("final_evidence_sha256")) and provenance.get("final_evidence_sha256") != actual_hash:
                errors.append("provenance.final_evidence_sha256 がこの evidence file と一致しない")

    if errors:
        return _failed(
            "windows_evidence_provenance_isolation",
            "; ".join(errors),
            "一意に stage した Windows run から、source commit、clean worktree、artifact hash、分離した install/runtime/config/audit/store path、field provenance、evidence bundle hash を収集する。",
        )
    return _passed(
        "windows_evidence_provenance_isolation",
        "Windows installed 証拠は、clean source commit、分離した run path、artifact hash、field 単位の evidence provenance に結び付いている。",
        "Windows の正式 evidence run をすべて分離し、commit との結び付きを維持する。",
    )


def load_evidence(path: Path = DEFAULT_EVIDENCE_PATH) -> tuple[dict[str, Any] | None, str | None]:
    if not path.exists():
        return None, f"{path.relative_to(ROOT)} がない"
    try:
        payload = json.loads(path.read_text(encoding="utf-8-sig"))
    except json.JSONDecodeError as exc:
        return None, f"{path.relative_to(ROOT)} は無効な JSON である: {exc}"
    if not isinstance(payload, dict):
        return None, f"{path.relative_to(ROOT)} は JSON object を含まなければならない"
    return payload, None


def validate_installer_first_run(data: dict[str, Any]) -> EvidenceResult:
    errors: list[str] = []
    if data.get("platform") != "windows":
        errors.append("platform は windows でなければならない")
    source = _get(data, "evidence_source")
    if not isinstance(source, dict):
        errors.append("evidence_source object がない")
    else:
        if source.get("collector") != "installer/windows/collect_installed_smoke.ps1":
            errors.append("evidence_source.collector は Windows installed smoke collector でなければならない")
        if not source.get("collector_version"):
            errors.append("evidence_source.collector_version がない")
        if source.get("manual_confirmation") is not False:
            errors.append("厳格な release では手動確認の evidence を受理しない")
    if not SHA256_RE.match(str(_get(data, "artifact.sha256") or "")):
        errors.append("artifact.sha256 には sha256 tag が必要である")
    if not _is_true(data, "artifact.installed_exe_exists"):
        errors.append("installed executable の存在を確認できなかった")
    installed_exe_path = str(_get(data, "artifact.installed_exe_path") or "")
    if not installed_exe_path.lower().endswith(".exe"):
        errors.append("artifact.installed_exe_path は installed Flutter executable を指さなければならない")
    launcher_path = str(_get(data, "artifact.desktop_launcher_path") or "")
    if not launcher_path.lower().endswith("gui_shell_desktop_launcher.exe"):
        errors.append("artifact.desktop_launcher_path はRust Desktop起動器を指さなければならない")
    if not SHA256_RE.match(str(_get(data, "artifact.desktop_launcher_sha256") or "")):
        errors.append("artifact.desktop_launcher_sha256 にはsha256 tagが必要である")
    if not SHA256_RE.match(str(_get(data, "artifact.broker_helper_sha256") or "")):
        errors.append("artifact.broker_helper_sha256 にはsha256 tagが必要である")
    if _get(data, "first_run.status") != "passed":
        errors.append("first_run.status は passed でなければならない")
    if not _is_true(data, "first_run.launched_from_installed_path"):
        errors.append("first run が installed app path から起動しなかった")
    if not _is_true(data, "first_run.launched_via_rust_desktop_launcher"):
        errors.append("first run がRust Desktop起動器を経由しなかった")
    launcher_pid = _get(data, "first_run.launcher_process_id")
    app_pid = _get(data, "first_run.process_id")
    if not isinstance(launcher_pid, int) or not isinstance(app_pid, int) or launcher_pid == app_pid:
        errors.append("Rust起動器とFlutter画面のprocessを別々に観測できなかった")
    process_identity = _get(data, "first_run.process_identity")
    if not isinstance(process_identity, dict):
        errors.append("起動器childの親PID・実行image hash証拠がない")
    else:
        if process_identity.get("method") != "direct_parent_pid_and_sha256":
            errors.append("起動器child identityは親PIDとSHA-256の直接観測でなければならない")
        if process_identity.get("evidence_class") != "LIVE_RUNTIME":
            errors.append("起動器child identity証拠がLIVE_RUNTIMEではない")
        if process_identity.get("direct_child_observed") is not True:
            errors.append("Flutter processがRust起動器の直接childであることを確認できなかった")
        if not isinstance(launcher_pid, int) or process_identity.get("parent_process_id") != launcher_pid:
            errors.append("Flutter processのobserved parent PIDがRust起動器PIDと一致しない")
        if process_identity.get("same_windows_session") is not True:
            errors.append("Flutter childとRust起動器が同じWindows sessionであることを確認できなかった")
        if process_identity.get("started_after_launcher") is not True:
            errors.append("Flutter childがRust起動器起動後に作成されたことを確認できなかった")
        if process_identity.get("image_sha256") != _get(data, "artifact.sha256"):
            errors.append("Flutter processのimage SHA-256がstaged artifactと一致しない")
        if process_identity.get("image_matches_manifest") is not True:
            errors.append("Flutter processのimage hashをstaged manifestと照合できなかった")
    if not _is_true(data, "first_run.profile_identity_isolated_from_staging_user"):
        errors.append("stage作業者と異なるWindows user profileでの実行を確認できなかった")
    if not _get(data, "first_run.launcher_runtime_dir"):
        errors.append("実起動器が使用したBroker runtime pathがない")
    if not _is_true(data, "first_run.launcher_exited_after_frontend") or _get(data, "first_run.launcher_exit_code") != 0:
        errors.append("Flutter終了後にRust Desktop起動器が正常終了したことを確認できなかった")
    if not _is_true(data, "first_run.process_running_after_launch"):
        errors.append("起動後に process が動作していることを確認できなかった")
    if not _is_true(data, "first_run.frontend_exit_menu_invoked"):
        errors.append("製品の通知領域『終了』操作を実行したことを確認できなかった")
    if not _is_false(data, "first_run.frontend_forced_to_exit"):
        errors.append("Flutter画面を強制終了せず通常終了したことを確認できなかった")
    if not _is_false(data, "first_run.frontend_cleanup_error"):
        errors.append("Flutter画面の通常終了・cleanupでerrorが発生した")
    if not isinstance(_get(data, "first_run.process_id"), int):
        errors.append("first_run.process_id がない")
    if not isinstance(_get(data, "first_run.main_window_handle"), int) or _get(data, "first_run.main_window_handle") == 0:
        errors.append("MainWindowHandle を確認できなかった")
    if not _is_true(data, "first_run.first_window_visible"):
        errors.append("最初の window が可視であることを確認できなかった")
    if not _is_true(data, "first_run.broker_mediated_launch"):
        errors.append("first run が Rust broker を介して起動されなかった")
    if not _get(data, "first_run.broker_helper_path"):
        errors.append("first_run.broker_helper_path がない")
    if not _get(data, "first_run.broker_endpoint_file"):
        errors.append("first_run.broker_endpoint_file がない")
    if not _is_true(data, "first_run.broker_endpoint_created"):
        errors.append("first run で broker endpoint file の作成を確認できなかった")
    if not _is_true(data, "first_run.broker_endpoint_removed_after_shutdown"):
        errors.append("起動器終了後に同一実行のBroker endpoint cleanupを確認できなかった")
    if _get(data, "first_run.broker_transport") != "authenticated_loopback_tcp":
        errors.append("first_run.broker_transport は authenticated_loopback_tcp でなければならない")
    if _get(data, "first_run.broker_endpoint_credential_role") != "normal":
        errors.append("first_run.broker_endpoint_credential_role は normal でなければならない")
    if not _is_true(data, "first_run.normal_endpoint_credential_role_verified"):
        errors.append("first run で broker通常接続資格のroleを確認できなかった")
    health_request = _get(data, "first_run.broker_health_request")
    if not isinstance(health_request, dict):
        errors.append("Brokerのhealth要求受理を示すAudit evidenceがない")
    else:
        accepted_event_count = health_request.get("accepted_event_count")
        if not _is_true(health_request, "accepted"):
            errors.append("通常資格Brokerがhealth要求を受理したLIVE_RUNTIME AuditEventがない")
        if type(accepted_event_count) is not int or accepted_event_count < 1:
            errors.append("Broker health受理AuditEvent数が1件以上の実測値ではない")
        if not re.fullmatch(r"broker-audit-[1-9][0-9]*", str(health_request.get("first_audit_event_id") or "")):
            errors.append("Broker health受理AuditEventのevent_idがない")
        if health_request.get("evidence_class") != "LIVE_RUNTIME":
            errors.append("Broker health受理AuditEventの証拠種別がLIVE_RUNTIMEではない")
        if health_request.get("caller_process_attributed") is not False:
            errors.append("Broker Auditにない呼出し元process attributionを主張している")
        if health_request.get("client_response_receipt_observed") is not False:
            errors.append("Broker Auditが証明しないclient応答受信を主張している")
    lifecycle_audit = _get(data, "first_run.broker_lifecycle_audit")
    if not isinstance(lifecycle_audit, dict):
        errors.append("起動器管理Brokerのlifecycle Audit evidenceがない")
    else:
        if lifecycle_audit.get("evidence_class") != "LIVE_RUNTIME":
            errors.append("Broker lifecycle Audit evidenceがLIVE_RUNTIMEではない")
        if not _is_true(lifecycle_audit, "startup_event_recorded"):
            errors.append("Broker startup AuditEventを確認できなかった")
        if not _is_true(lifecycle_audit, "shutdown_event_recorded"):
            errors.append("Broker shutdown AuditEventを確認できなかった")
        if not isinstance(lifecycle_audit.get("startup_event_count"), int) or lifecycle_audit["startup_event_count"] < 1:
            errors.append("Broker startup AuditEvent countが不正")
        if not isinstance(lifecycle_audit.get("shutdown_event_count"), int) or lifecycle_audit["shutdown_event_count"] < 1:
            errors.append("Broker shutdown AuditEvent countが不正")
        if not SHA256_RE.match(str(lifecycle_audit.get("sha256") or "")):
            errors.append("Brokerの起動・終了監査記録ファイルにSHA-256値がない")
    if not _is_true(data, "first_run.no_python_runtime_requested"):
        errors.append("初回起動でPython実行系なしの実測モードを要求していなかった")
    if not _is_true(data, "first_run.python_runtime_path_scrubbed"):
        errors.append("初回起動前にPATHからPython実行系を除外していなかった")
    if _get(data, "first_run.python_path_entries_remaining_count") != 0:
        errors.append("first-run 起動前に Python PATH entry が可視のまま残った")
    python_commands = _get(data, "first_run.python_commands_visible_after_scrub")
    if not isinstance(python_commands, list):
        errors.append("first_run.python_commands_visible_after_scrub は list でなければならない")
    elif python_commands:
        errors.append("first-run 起動前に Python command が可視のまま残った")
    if not _is_true(data, "first_run.visible_surfaces_complete"):
        errors.append("first_run.visible_surfaces_complete は true でなければならない")
    visible = set(_get(data, "first_run.visible_surfaces") or [])
    for label in sorted(REQUIRED_VISIBLE_SURFACES):
        if label not in visible:
            errors.append(f"{label} が可視として記録されていない")
    surface_evidence = _get(data, "first_run.visible_surfaces_evidence")
    if not isinstance(surface_evidence, dict):
        errors.append("可視 surface の evidence がない")
    else:
        if surface_evidence.get("source") not in {
            "uiautomation",
            "accessibility_tree",
        }:
            errors.append("可視 surface の evidence source は uiautomation または accessibility_tree でなければならない。Flutter build registry は可視性を証明しない")
        if not surface_evidence.get("path"):
            errors.append("可視 surface の evidence path がない")
        errors.extend(_validate_surface_match_evidence(surface_evidence))
    if not _is_true(data, "first_run.config_created"):
        errors.append("first-run config の作成を確認できなかった")
    if not _is_false(data, "first_run.config_existed_before_launch"):
        errors.append("first-run config は起動前に存在してはならず、その後の生成を確認する必要がある")
    if not _is_true(data, "first_run.config_json_valid"):
        errors.append("first-run config JSON の妥当性を確認できなかった")
    if not _get(data, "first_run.config_path"):
        errors.append("first-run config path がない")
    config_audit = _get(data, "first_run.config_audit")
    if not isinstance(config_audit, dict):
        errors.append("first-run configとBroker Auditを結ぶ証拠がない")
    else:
        if config_audit.get("evidence_class") != "LIVE_RUNTIME":
            errors.append("first-run config Audit evidenceはLIVE_RUNTIMEでなければならない")
        if config_audit.get("accepted_event_count") != 1:
            errors.append("first-run configのaccepted AuditEventは一件でなければならない")
        config_hash = config_audit.get("config_sha256")
        if not _is_sha256_tag(config_hash):
            errors.append("first-run configのSHA-256がない")
        event = config_audit.get("accepted_event")
        if not isinstance(event, dict):
            errors.append("first-run configのaccepted Broker AuditEventがない")
        else:
            if event.get("operation") != "初回設定取得" or event.get("decision") != "accepted":
                errors.append("first-run config AuditEventが初回設定取得のaccepted eventではない")
            if event.get("reason") != "初回UI設定を固定Broker storeからprojection。authorityは生成しない":
                errors.append("first-run config AuditEventのreasonが一致しない")
            if event.get("evidence_source") != "LIVE_RUNTIME":
                errors.append("first-run config AuditEventの証拠種別がLIVE_RUNTIMEではない")
            if not re.fullmatch(r"broker-audit-[1-9][0-9]*", str(event.get("event_id") or "")):
                errors.append("first-run config AuditEventのevent_idが不正")
            if not event.get("request_id"):
                errors.append("first-run config AuditEventのrequest_idがない")
            if not _is_sha256_tag(event.get("event_hash")):
                errors.append("first-run config AuditEventのevent_hashがない")
            if event.get("payload_hash") != config_hash:
                errors.append("first-run config AuditEvent payload_hashがconfig file hashと一致しない")
        bundle_files = _get(data, "provenance.evidence_bundle_files")
        config_files = [
            item for item in bundle_files or []
            if isinstance(item, dict) and item.get("kind") == "first_run_configuration"
        ] if isinstance(bundle_files, list) else []
        if len(config_files) != 1:
            errors.append("evidence bundleにfirst-run config fileが一件だけ含まれていない")
        elif (
            config_files[0].get("path") != _get(data, "first_run.config_path")
            or config_files[0].get("sha256") != config_hash
            or config_files[0].get("exists") is not True
        ):
            errors.append("evidence bundleのfirst-run config path/hashがAuditと一致しない")
    if not _is_true(data, "first_run.audit_dir_writable"):
        errors.append("audit directory の書込み可能性を確認できなかった")
    probe = _get(data, "first_run.audit_write_probe")
    if not isinstance(probe, dict):
        errors.append("audit write probe がない")
    elif not all(probe.get(key) is True for key in ("attempted", "write", "read", "delete")):
        errors.append("audit の write/read/delete probe が合格しなかった")
    if not _get(data, "first_run.audit_dir"):
        errors.append("audit dir path がない")
    if not _is_false(data, "first_run.installer_grants_authority"):
        errors.append("installer authority boundary が false であることを確認できなかった")
    if not _is_false(data, "first_run.installer_silently_approves_permissions"):
        errors.append("silent approval boundary が false であることを確認できなかった")
    if errors:
        return _failed(
            "windows_installer_first_run_smoke",
            "; ".join(errors),
            "Rust Desktop起動器、分離Windows user profile、実runtime lifecycle／health受理Audit、Setup Doctor／config製品証拠を同一clean-source runから収集する。",
        )
    return _passed(
        "windows_installer_first_run_smoke",
        "Windows installed executable の first-run smoke evidence が機械検証に合格した。",
        "release candidate ごとに Windows installed first-run evidence を最新に保つ。",
    )


def validate_setup_doctor(data: dict[str, Any]) -> EvidenceResult:
    errors: list[str] = []
    setup = _get(data, "setup_doctor")
    if not isinstance(setup, dict):
        errors.append("setup_doctor object がない")
    else:
        if setup.get("formal_product_evidence") is not True:
            errors.append("Setup Doctor は外部 probe ではなく installed-app product evidence でなければならない")
        if setup.get("status") != "pass":
            errors.append("strict releaseではSetup Doctor全checkがpassでなければならない")
        evidence_source = setup.get("evidence_source")
        if not isinstance(evidence_source, dict):
            errors.append("Setup Doctor の evidence_source がない")
        else:
            if evidence_source.get("source_kind") != "installed_app_machine_readable_export":
                errors.append("Setup Doctor の evidence_source.source_kind は installed_app_machine_readable_export でなければならない")
            if evidence_source.get("product_generated") is not True:
                errors.append("Setup Doctor evidence は installed app が生成しなければならない")
            if evidence_source.get("collector_derives_checks") is not False:
                errors.append("Setup Doctor collector は product diagnostic check を導出してはならない")
            if evidence_source.get("synthetic") is not False:
                errors.append("合成した Setup Doctor evidence は受理しない")
            if not evidence_source.get("command"):
                errors.append("Setup Doctor の evidence_source.command がない")
        encoded_report = setup.get("serialized_report_base64")
        report_hash = setup.get("report_sha256")
        product_report = setup.get("product_report")
        accepted_audit = setup.get("accepted_audit_event")
        report_bytes: bytes | None = None
        if not isinstance(encoded_report, str) or len(encoded_report) > 96 * 1024:
            errors.append("Setup Doctorの保存済みreport byte列がない、またはsize上限を超える")
        else:
            try:
                report_bytes = base64.b64decode(encoded_report, validate=True)
            except (ValueError, base64.binascii.Error):
                errors.append("Setup Doctorの保存済みreport byte列がbase64として不正")
        if report_bytes is not None:
            if not report_bytes or len(report_bytes) > 64 * 1024:
                errors.append("Setup Doctor report byte列が許容size外")
            actual_report_hash = "sha256:" + hashlib.sha256(report_bytes).hexdigest()
            if report_hash != actual_report_hash:
                errors.append("Setup Doctor report hashが保存byte列と一致しない")
            if not isinstance(product_report, dict):
                errors.append("Setup Doctorのproduct_report objectがない")
            else:
                try:
                    decoded_report = json.loads(report_bytes.decode("utf-8", errors="strict"))
                    from tooling.schema_check.check_schemas import validate_instance
                    schema = json.loads((ROOT / "specs" / "setup_doctor_report.schema.json").read_text(encoding="utf-8"))
                    if not isinstance(decoded_report, dict) or decoded_report != product_report:
                        errors.append("Setup Doctor product_reportが保存されたUTF-8 reportと一致しない")
                    elif validate_instance(decoded_report, schema):
                        errors.append("Setup Doctor product reportが正本Schemaに適合しない")
                    elif decoded_report.get("status") != setup.get("status") or decoded_report.get("checks") != setup.get("checks"):
                        errors.append("Setup Doctorの表示・collector値がBroker reportと一致しない")
                except (OSError, UnicodeError, json.JSONDecodeError, ValueError, TypeError) as error:
                    errors.append(f"Setup Doctor product reportを検証できない: {error}")
        if not isinstance(accepted_audit, dict):
            errors.append("Setup Doctorのaccepted AuditEventがない")
        else:
            expected_report_hash = report_hash if isinstance(report_hash, str) else ""
            if (
                accepted_audit.get("operation") != "Setup Doctor報告取得"
                or accepted_audit.get("decision") != "accepted"
                or accepted_audit.get("reason") != "setup_doctor_report_exported"
                or accepted_audit.get("evidence_source") != "LIVE_RUNTIME"
                or accepted_audit.get("payload_hash") != expected_report_hash
                or not re.fullmatch(r"broker-audit-[1-9][0-9]*", str(accepted_audit.get("event_id", "")))
                or not SHA256_RE.fullmatch(str(accepted_audit.get("event_hash", "")))
                or not accepted_audit.get("request_id")
            ):
                errors.append("Setup Doctor accepted AuditEventがproduct reportのLIVE_RUNTIME hashと一致しない")
            if evidence_source and evidence_source.get("accepted_audit_event_id") != accepted_audit.get("event_id"):
                errors.append("Setup Doctor evidence_sourceのaccepted AuditEvent IDが一致しない")
            if evidence_source and evidence_source.get("accepted_audit_event_payload_hash") != expected_report_hash:
                errors.append("Setup Doctor evidence_sourceのAudit payload hashが一致しない")
        provenance = data.get("provenance")
        bundle_files = provenance.get("evidence_bundle_files") if isinstance(provenance, dict) else None
        setup_file = next((item for item in bundle_files or [] if isinstance(item, dict) and item.get("kind") == "setup_doctor"), None)
        if (
            not isinstance(setup_file, dict)
            or setup_file.get("exists") is not True
            or setup_file.get("sha256") != report_hash
            or setup_file.get("path") != setup.get("report_path")
        ):
            errors.append("Setup Doctor reportの固定store file記録/hashがevidence bundleと一致しない")
        isolation = _get(data, "provenance.isolation")
        if isinstance(isolation, dict) and setup.get("report_path"):
            expected_report_path = ntpath.normcase(ntpath.normpath(
                ntpath.join(str(isolation.get("isolated_store_dir") or ""), "setup_doctor_report.json")
            ))
            if ntpath.normcase(ntpath.normpath(str(setup.get("report_path")))) != expected_report_path:
                errors.append("Setup Doctor reportは固定Broker Store path以外から収集してはならない")
        if not setup.get("ran_from_installed_app_path"):
            errors.append("Setup Doctor が installed app path から実行されなかった")
        if not setup.get("operator_readable"):
            errors.append("Setup Doctor の operator readability を確認できなかった")
        errors.extend(_validate_setup_doctor_operator_readability(data, setup))
        if setup.get("installer_grants_authority") is not False:
            errors.append("Setup Doctor の installer_grants_authority は false でなければならない")
        if setup.get("installer_silently_approves_permissions") is not False:
            errors.append("Setup Doctor の installer_silently_approves_permissions は false でなければならない")
        checks = setup.get("checks")
        if not isinstance(checks, list) or not checks:
            errors.append("Setup Doctor の checks は空でない list でなければならない")
        else:
            if any(check.get("status") != "pass" for check in checks if isinstance(check, dict)):
                errors.append("strict releaseではunknown、warning、failを含むSetup Doctor checkを受理しない")
            if any(check.get("status") == "fail" for check in checks if isinstance(check, dict)):
                errors.append("Setup Doctor に失敗した check が含まれる")
            if any(check.get("grants_authority") is not False for check in checks if isinstance(check, dict)):
                errors.append("Setup Doctor check が権限を付与している、または grants_authority=false がない")
            check_ids = {
                check.get("check_id")
                for check in checks
                if isinstance(check, dict) and isinstance(check.get("check_id"), str)
            }
            missing = REQUIRED_SETUP_CHECKS - check_ids
            if missing:
                errors.append(f"Setup Doctor に次の必須 check がない: {', '.join(sorted(missing))}")
            if any(not check.get("recovery_instruction") for check in checks if isinstance(check, dict)):
                errors.append("Setup Doctor の checks は recovery instruction を含まなければならない")
    if errors:
        return _failed(
            "windows_setup_doctor_smoke",
            "; ".join(errors),
            "Windows の native installed smoke を実行し、installed app に機械可読な Setup Doctor の product evidence を書き出させる。外部 collector probe の出力は product evidence として受理しない。",
        )
    return _passed(
        "windows_setup_doctor_smoke",
        "Windows installed-path の Setup Doctor evidence が機械検証に合格した。",
        "release candidate ごとに Windows Setup Doctor evidence を最新に保つ。",
    )


def validate_broker_smoke(data: dict[str, Any]) -> EvidenceResult:
    errors: list[str] = []
    broker = _get(data, "broker")
    if not isinstance(broker, dict):
        errors.append("broker evidence object がない")
    else:
        if broker.get("status") != "passed":
            errors.append("broker.status は passed でなければならない")
        source = broker.get("evidence_source")
        if not isinstance(source, dict):
            errors.append("broker の evidence_source がない")
        else:
            if source.get("collector") != "installer/windows/collect_broker_smoke.ps1":
                errors.append("broker の evidence_source.collector は installer/windows/collect_broker_smoke.ps1 でなければならない")
            if source.get("collector_version") != "6":
                errors.append("broker の evidence_source.collector_version は6でなければならない")
            if source.get("synthetic") is not False:
                errors.append("合成した broker evidence は受理しない")
            if not source.get("command"):
                errors.append("broker の evidence_source.command がない")
        field_provenance = broker.get("field_provenance")
        if not isinstance(field_provenance, dict):
            errors.append("broker の field_provenance がない")
        for field in sorted(REQUIRED_BROKER_TRUE_FIELDS):
            if broker.get(field) is not True:
                errors.append(f"broker.{field} は true でなければならない")
            if isinstance(field_provenance, dict):
                entry = field_provenance.get(field)
                if not isinstance(entry, dict):
                    errors.append(f"broker の field_provenance.{field} がない")
                else:
                    if entry.get("source_type") != "directly_measured":
                        errors.append(f"broker の field_provenance.{field}.source_type は directly_measured でなければならない")
                    evidence_class = entry.get("evidence_class")
                    if evidence_class not in ("LIVE_RUNTIME", "EXTERNAL_EVIDENCE"):
                        errors.append(f"broker の field_provenance.{field}.evidence_class は LIVE_RUNTIME または EXTERNAL_EVIDENCE でなければならない")
        if broker.get("endpoint_host") != "127.0.0.1":
            errors.append("broker の endpoint_host は 127.0.0.1 でなければならない")
        if broker.get("endpoint_credential_role") != "normal":
            errors.append("broker の endpoint_credential_role は normal でなければならない")
        if broker.get("replay_error_code") != "broker_replay_detected":
            errors.append("broker の replay_error_code は broker_replay_detected でなければならない")
        if broker.get("agent_task_workspace_permission_error_code") != "desktop_native_owner_confirmation_required":
            errors.append("通常Broker IPCのAgent Task Workspace Permissionはnative Owner確認必須として拒否されなければならない")
        if broker.get("agent_task_owner_approval_error_code") != "desktop_native_owner_confirmation_required":
            errors.append("通常Broker IPCのAgent Task Owner Approvalはnative Owner確認必須として拒否されなければならない")
        if "python_runtime_required_for_authority" in broker:
            errors.append("broker の top-level declaration python_runtime_required_for_authority は測定済み broker evidence として受理しない")
        if "flutter_rust_ffi_authority_bridge" in broker:
            errors.append("broker の top-level declaration flutter_rust_ffi_authority_bridge は測定済み broker evidence として受理しない")
        declarations = broker.get("unmeasured_declarations")
        if declarations not in (None, {}):
            if not isinstance(declarations, dict):
                errors.append("broker の unmeasured_declarations は、存在する場合 object でなければならない")
            else:
                for key, value in declarations.items():
                    if not isinstance(value, dict):
                        errors.append(f"broker の unmeasured_declarations.{key} は object でなければならない")
                        continue
                    if value.get("formal_runtime_proof") is not False:
                        errors.append(f"broker の unmeasured_declarations.{key}.formal_runtime_proof は false でなければならない")
        broker_errors = broker.get("errors")
        if broker_errors not in (None, []) and not (isinstance(broker_errors, list) and len(broker_errors) == 0):
            errors.append("broker の errors は空でなければならない")
    if errors:
        return _failed(
            "windows_broker_installed_smoke",
            "; ".join(errors),
            "installed Rust broker helper に対して installer/windows/collect_broker_smoke.ps1 を実行し、測定済みの IPC/restart/crash と一時資格file削除の provenance だけを release_evidence/windows_installed_smoke.json に含める。",
        )
    return _passed(
        "windows_broker_installed_smoke",
        "Windows installed-path broker の launch/connect/restart/crash と一時資格file削除 evidence が機械検証に合格した。no-Python/no-FFI は個別に分類された static evidence または installed-launch evidence のままである。",
        "release candidate ごとに broker の installed-path smoke evidence を最新に保つ。",
    )


def _setup_doctor_status_label(value: Any) -> str:
    return {
        "pass": "正常",
        "warning": "確認が必要",
        "fail": "問題あり",
    }.get(str(value), "不明")


def _setup_doctor_check_title(value: Any) -> str:
    return {
        "setup_doctor.ran_from_installed_app_path": "製品配置",
        "setup_doctor.runtime_connection": "Broker接続",
        "setup_doctor.authority_boundary": "権限境界",
        "setup_doctor.network_public_bind": "通信範囲",
        "setup_doctor.recovery_instruction": "復旧案内",
        "setup_doctor.audit_storage": "監査保存",
        "setup_doctor.config_created": "初回設定",
    }.get(str(value), "その他の診断項目")


def _rect_intersects_window(rect: Any, window: Any) -> bool:
    required = ("x", "y", "width", "height")
    if not isinstance(rect, dict) or not isinstance(window, dict):
        return False
    values = [rect.get(key) for key in required]
    window_values = [window.get(key) for key in required]
    if any(type(value) not in (int, float) or not math.isfinite(value) for value in values + window_values):
        return False
    x, y, width, height = values
    wx, wy, wwidth, wheight = window_values
    if width <= 0 or height <= 0 or wwidth <= 0 or wheight <= 0:
        return False
    return min(x + width, wx + wwidth) > max(x, wx) and min(y + height, wy + wheight) > max(y, wy)


def _point_inside_rect(point: Any, rect: Any) -> bool:
    if not isinstance(point, dict) or not isinstance(rect, dict):
        return False
    x, y = point.get("x"), point.get("y")
    rx, ry, width, height = (rect.get(key) for key in ("x", "y", "width", "height"))
    values = (x, y, rx, ry, width, height)
    if any(type(value) not in (int, float) or not math.isfinite(value) for value in values):
        return False
    return width > 0 and height > 0 and rx <= x < rx + width and ry <= y < ry + height


def _validate_setup_doctor_operator_readability(data: dict[str, Any], setup: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    proof = setup.get("operator_readability_proof")
    reference = setup.get("operator_readability_evidence")
    if setup.get("operator_readable") is not True:
        errors.append("Setup Doctor の operator_readable は実画面証拠に基づき true でなければならない")
    if not isinstance(proof, dict):
        return errors + ["Setup Doctor の実UIAutomation operator-readability proof がない"]
    if not isinstance(reference, dict):
        errors.append("Setup Doctor の operator-readability evidence参照がない")
    else:
        for key, expected in (
            ("source", "uiautomation"),
            ("evidence_class", "LIVE_RUNTIME"),
            ("status", "passed"),
            ("process_id", _get(data, "first_run.process_id")),
            ("run_id", _get(data, "provenance.isolation.run_id")),
            ("report_sha256", setup.get("report_sha256")),
        ):
            if reference.get(key) != expected:
                errors.append(f"Setup Doctor操作者向け可読性証拠の{key}が検証済みruntime／reportと一致しない")
        if not _is_sha256_tag(reference.get("sha256")) or not reference.get("path"):
            errors.append("Setup Doctor operator-readability sidecarのpathまたはSHA-256がない")
        bundle_files = _get(data, "provenance.evidence_bundle_files")
        records = [
            item for item in bundle_files or []
            if isinstance(item, dict) and item.get("kind") == "setup_doctor_operator_readability"
        ] if isinstance(bundle_files, list) else []
        if (
            len(records) != 1
            or records[0].get("exists") is not True
            or records[0].get("path") != reference.get("path")
            or records[0].get("sha256") != reference.get("sha256")
        ):
            errors.append("Setup Doctor operator-readability sidecarが同一evidence bundleへhash結合されていない")
    if proof.get("evidence_version") != 1 or proof.get("status") != "passed":
        errors.append("Setup Doctor operator-readability proofはversion 1のpassedでなければならない")
    if (
        proof.get("source") != "uiautomation"
        or proof.get("evidence_class") != "LIVE_RUNTIME"
        or proof.get("measurement_scope") != "前景の可視UI Automation文字と最前面点の観測"
        or proof.get("visual_contrast_measured") is not False
        or proof.get("screen_reader_executed") is not False
        or proof.get("collector") != "installer/windows/collect_installed_smoke.ps1"
        or proof.get("collector_version") != "17"
        or proof.get("process_id") != _get(data, "first_run.process_id")
        or proof.get("run_id") != _get(data, "provenance.isolation.run_id")
        or proof.get("report_sha256") != setup.get("report_sha256")
    ):
        errors.append("Setup Doctor operator-readability proofのsource／process／run／report結合が不正")
    if proof.get("errors") != []:
        errors.append("Setup Doctor operator-readability proofに収集失敗がある")
    if type(proof.get("scroll_passes")) is not int or not 1 <= proof["scroll_passes"] <= 17:
        errors.append("Setup Doctor画面の移動回数が範囲外")

    window = proof.get("window")
    if (
        not isinstance(window, dict)
        or window.get("process_id") != proof.get("process_id")
        or window.get("control_type") != "ControlType.Window"
        or not isinstance(window.get("runtime_id"), str)
        or not window.get("runtime_id")
        or type(window.get("native_window_handle")) is not int
        or window.get("native_window_handle", 0) <= 0
        or type(proof.get("main_window_handle")) is not int
        or proof.get("main_window_handle") != window.get("native_window_handle")
        or proof.get("main_window_handle") != _get(data, "first_run.main_window_handle")
        or not isinstance(window.get("bounding_rectangle"), dict)
        or not _rect_intersects_window(window.get("bounding_rectangle"), window.get("bounding_rectangle"))
    ):
        errors.append("Setup Doctor operator-readability windowは同じFrontendの実UIA windowでない")
        window_rect = None
    else:
        window_rect = window["bounding_rectangle"]

    navigation = proof.get("navigation")
    nav_observation = navigation.get("observation") if isinstance(navigation, dict) else None
    if (
        not isinstance(navigation, dict)
        or navigation.get("label") != "診断"
        or navigation.get("action") != "visible_uia_element_pointer_click"
        or navigation.get("matched") is not True
        or not isinstance(nav_observation, dict)
        or nav_observation.get("process_id") != proof.get("process_id")
        or nav_observation.get("window_runtime_id") != (window.get("runtime_id") if isinstance(window, dict) else None)
        or nav_observation.get("control_type") != "ControlType.Text"
        or str(nav_observation.get("name", "")).splitlines()[0].strip() != "診断"
        or nav_observation.get("is_offscreen") is not False
        or not isinstance(nav_observation.get("runtime_id"), str)
        or not nav_observation.get("runtime_id")
        or nav_observation.get("runtime_id") == (window.get("runtime_id") if isinstance(window, dict) else None)
        or not _rect_intersects_window(nav_observation.get("bounding_rectangle"), window_rect)
        or not _point_inside_rect(nav_observation.get("visible_sample"), nav_observation.get("bounding_rectangle"))
        or not _point_inside_rect(nav_observation.get("visible_sample"), window_rect)
        or not isinstance(nav_observation.get("visible_sample"), dict)
        or nav_observation["visible_sample"].get("topmost_native_window_handle") != proof.get("main_window_handle")
    ):
        errors.append("Setup Doctorへの遷移が同じFrontendの可視『診断』UI操作として観測されていない")

    expected: dict[str, str] = {
        "page_title": "環境診断",
        "status_summary": "診断状態: " + _setup_doctor_status_label(setup.get("status")),
        "scope_notice": "この診断はPermissionやApprovalを作らず、製品リリースの完成判定にも使いません。",
        "authority_notice": "インストーラーは権限を付与しません。",
        "approval_notice": "インストーラーはPermissionを自動承認しません。",
    }
    checks = setup.get("checks")
    if isinstance(checks, list):
        seen_check_ids: set[str] = set()
        for check in checks:
            if not isinstance(check, dict) or not isinstance(check.get("check_id"), str):
                errors.append("Setup Doctor UI表示と対応づけるcheck recordが不正")
                continue
            if check["check_id"] in seen_check_ids:
                errors.append("Setup Doctor UI表示と対応づけるcheck_idが重複")
            seen_check_ids.add(check["check_id"])
            prefix = f"check:{check['check_id']}"
            expected[f"{prefix}:title"] = _setup_doctor_check_title(check["check_id"])
            expected[f"{prefix}:status"] = _setup_doctor_status_label(check.get("status"))
            message = check.get("message")
            if not isinstance(message, str) or not message.strip():
                errors.append(f"Setup Doctor {check['check_id']} messageが空でUI表示を証明できない")
            expected[f"{prefix}:message"] = message if isinstance(message, str) else ""
            if check.get("status") != "pass":
                recovery = check.get("recovery_instruction")
                if not isinstance(recovery, str) or not recovery.strip():
                    errors.append(f"Setup Doctor {check['check_id']} recovery_instructionが空である")
                expected[f"{prefix}:recovery_heading"] = "次に行うこと"
                expected[f"{prefix}:recovery"] = recovery if isinstance(recovery, str) else ""
    else:
        errors.append("Setup Doctor UIとの照合対象checksがlistでない")

    actual = proof.get("required_elements")
    if not isinstance(actual, list) or len(actual) != len(expected):
        errors.append("Setup Doctor operator-readability proofのrequired element数が期待値と一致しない")
        actual = actual if isinstance(actual, list) else []
    by_key = {item.get("key"): item for item in actual if isinstance(item, dict) and isinstance(item.get("key"), str)}
    if len(by_key) != len(actual):
        errors.append("Setup Doctor operator-readability proofに不正または重複element keyがある")
    if set(by_key) != set(expected):
        errors.append("Setup Doctor operator-readability proofの要求表示項目がreportと一致しない")

    used_runtime_ids: set[str] = set()
    if isinstance(nav_observation, dict) and isinstance(nav_observation.get("runtime_id"), str):
        used_runtime_ids.add(nav_observation["runtime_id"])
    for key, text in expected.items():
        item = by_key.get(key)
        observation = item.get("observation") if isinstance(item, dict) else None
        if (
            not isinstance(item, dict)
            or item.get("expected") != text
            or item.get("matched") is not True
            or type(item.get("scroll_pass")) is not int
            or not 1 <= item["scroll_pass"] <= (proof.get("scroll_passes") if type(proof.get("scroll_passes")) is int else 0)
            or not isinstance(observation, dict)
        ):
            errors.append(f"Setup Doctor画面要素{key}がreportの正確な表示証拠として不完全")
            continue
        runtime_id = observation.get("runtime_id")
        observed_name = _normalised_text(observation.get("name"))
        if (
            observation.get("process_id") != proof.get("process_id")
            or observation.get("window_runtime_id") != (window.get("runtime_id") if isinstance(window, dict) else None)
            or observation.get("is_offscreen") is not False
            or observation.get("control_type") not in {"ControlType.Text", "ControlType.Custom"}
            or not isinstance(runtime_id, str)
            or not runtime_id
            or runtime_id in used_runtime_ids
            or observed_name != _normalised_text(text)
            or not _rect_intersects_window(observation.get("bounding_rectangle"), window_rect)
            or not _point_inside_rect(observation.get("visible_sample"), observation.get("bounding_rectangle"))
            or not _point_inside_rect(observation.get("visible_sample"), window_rect)
            or not isinstance(observation.get("visible_sample"), dict)
            or observation["visible_sample"].get("topmost_native_window_handle") != proof.get("main_window_handle")
            or not isinstance(observation.get("parent_runtime_id"), str)
            or not observation.get("parent_runtime_id")
        ):
            errors.append(f"Setup Doctor画面要素{key}は固有かつ可視の個別UI Automation要素ではない")
        if isinstance(runtime_id, str):
            used_runtime_ids.add(runtime_id)
    return errors


def validate_audit_anchor_external_tamper_evidence(data: dict[str, Any]) -> EvidenceResult:
    errors: list[str] = []
    provenance = data.get("provenance")
    if not isinstance(provenance, dict):
        errors.append("audit anchor evidence の provenance object がない")
    else:
        bundle_files = provenance.get("evidence_bundle_files")
        if not isinstance(bundle_files, list) or not bundle_files:
            errors.append("provenance.evidence_bundle_files に audit anchor evidence がない")
        else:
            kinds = {item.get("kind") for item in bundle_files if isinstance(item, dict)}
            missing = AUDIT_REQUIRED_EVIDENCE_BUNDLE_KINDS - {kind for kind in kinds if isinstance(kind, str)}
            if missing:
                errors.append(f"provenance.evidence_bundle_files に次の kind がない: {', '.join(sorted(missing))}")
    errors.extend(
        _validate_field_provenance(
            data,
            AUDIT_REQUIRED_FIELD_PROVENANCE,
            check_unsupported_claims=False,
        )
    )
    evidence = data.get("audit_anchor_external_tamper_evidence")
    if not isinstance(evidence, dict):
        errors.append("audit_anchor_external_tamper_evidence object がない")
    else:
        try:
            from tooling.audit_checkpoint_verifier import verify_collected
            verify_collected(evidence)
        except (ValueError, OSError, KeyError, TypeError, subprocess.SubprocessError) as error:
            errors.append("署名checkpointの独立した実検証が失敗: " + str(error))
        if evidence.get("status") != "passed":
            errors.append("audit_anchor_external_tamper_evidence.status は passed でなければならない")
        if evidence.get("installed_path_verified") is not True:
            errors.append("audit anchor evidence は installed app path から測定しなければならない")
        if evidence.get("key_anchor_log_same_user_rewrite_mitigated") is not True:
            errors.append("同一 user による key+anchor+log rewrite の緩和策を検証しなければならない")
        protection_checks = [
            "windows_acl_verified",
            "dpapi_verified",
            "external_anchor_verified",
            "signed_evidence_verified",
        ]
        if not any(evidence.get(field) is True for field in protection_checks):
            errors.append("audit anchor evidence は Windows ACL、DPAPI、external anchor、signed evidence のいずれかを検証しなければならない")
        if evidence.get("administrator_root_resistance_claimed") is True and not (
            evidence.get("external_anchor_verified") is True
            or evidence.get("signed_evidence_verified") is True
        ):
            errors.append("administrator/root resistance には external anchor または signed evidence が必要である")

        source = evidence.get("evidence_source")
        if not isinstance(source, dict):
            errors.append("audit anchor の evidence_source がない")
        else:
            if source.get("source_kind") not in {
                "windows_acl_dpapi_probe",
                "external_anchor",
                "signed_evidence",
            }:
                errors.append("audit anchor の evidence_source.source_kind は windows_acl_dpapi_probe、external_anchor、signed_evidence のいずれかでなければならない")
            evidence_class = source.get("evidence_class")
            if evidence_class not in ("LIVE_RUNTIME", "EXTERNAL_EVIDENCE"):
                errors.append("audit anchor の evidence_source.evidence_class は LIVE_RUNTIME または EXTERNAL_EVIDENCE でなければならない")
            if source.get("synthetic") is not False:
                errors.append("合成した audit anchor evidence は受理しない")
            if not source.get("command"):
                errors.append("audit anchor の evidence_source.command がない")
            if not source.get("path"):
                errors.append("audit anchor の evidence_source.path がない")
            if not _is_sha256_tag(source.get("sha256")):
                errors.append("audit anchor の evidence_source.sha256 には sha256 tag が必要である")
    if errors:
        return _failed(
            "audit_anchor_external_tamper_evidence_proof",
            "; ".join(errors),
            "ownerの公開鍵fingerprintと外部継続性記録を固定し、現在installed状態に結合したオフライン署名checkpointを収集・再検証する。",
        )
    return _passed(
        "audit_anchor_external_tamper_evidence_proof",
        "Windows installed-path の audit anchor に対する external tamper-evidence proof が機械検証に合格した。",
        "release candidate ごとに audit anchor key-protection または external-anchor evidence を最新に保つ。",
    )


def validate_windows_release_evidence(path: Path = DEFAULT_EVIDENCE_PATH) -> list[EvidenceResult]:
    data, error = load_evidence(path)
    if data is None:
        return [
            _failed(
                "windows_evidence_provenance_isolation",
                error or "Windows installed smoke evidence がない",
                "正確な source commit と artifact hash に結び付いた分離済み native Windows run から release_evidence/windows_installed_smoke.json を作成する。",
            ),
            _failed(
                "windows_installer_first_run_smoke",
                error or "Windows installed smoke evidence がない",
                "broker を介した Flutter .exe 起動、-NoPythonRuntime 起動、測定済み window、visible-surface、config、audit probe evidence を含む native Windows installed-app smoke から release_evidence/windows_installed_smoke.json を作成する。",
            ),
            _failed(
                "windows_setup_doctor_smoke",
                error or "Windows Setup Doctor evidence がない",
                "Windows の native installed smoke を実行し、installed app に機械可読な Setup Doctor の product evidence を書き出させる。外部 collector probe の出力は product evidence として受理しない。",
            ),
            _failed(
                "windows_broker_installed_smoke",
                error or "Windows broker の installed smoke evidence がない",
                "installer/windows/collect_broker_smoke.ps1 を実行し、broker evidence を release_evidence/windows_installed_smoke.json に含める。",
            ),
            _failed(
                "audit_anchor_external_tamper_evidence_proof",
                error or "Audit anchor の external tamper-evidence proof がない",
                "ownerの公開鍵fingerprintと外部継続性記録を固定し、現在installed状態に結合したオフライン署名checkpointを収集・再検証する。",
            ),
        ]
    return [
        validate_provenance_and_isolation(data, path),
        validate_installer_first_run(data),
        validate_setup_doctor(data),
        validate_broker_smoke(data),
        validate_audit_anchor_external_tamper_evidence(data),
    ]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--evidence", type=Path, default=DEFAULT_EVIDENCE_PATH)
    args = parser.parse_args()
    results = validate_windows_release_evidence(args.evidence)
    print(json.dumps([result.__dict__ for result in results], indent=2, sort_keys=True))
    return 1 if any(result.classification == "release_blocker" for result in results) else 0


if __name__ == "__main__":
    raise SystemExit(main())
