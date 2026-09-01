from __future__ import annotations

import argparse
import hashlib
import json
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_EVIDENCE_PATH = ROOT / "release_evidence" / "windows_installed_smoke.json"
SHA256_RE = re.compile(r"^sha256:[0-9a-f]{64}$")
SOURCE_COMMIT_RE = re.compile(r"^[0-9a-f]{40}$")
REQUIRED_VISIBLE_SURFACES = {"Dashboard", "NavigationRail", "Runtime Status", "Invariant Status"}
REQUIRED_SETUP_CHECKS = {
    "windows.installed_app_path",
    "windows.artifact_hash",
    "first_run.config_created",
    "first_run.audit_dir_writable",
    "setup_doctor.ran_from_installed_app_path",
    "setup_doctor.runtime_connection",
    "setup_doctor.authority_boundary",
    "setup_doctor.network_public_bind",
    "setup_doctor.recovery_instruction",
    "setup_doctor.audit_storage",
}
REQUIRED_BROKER_TRUE_FIELDS = {
    "helper_exe_exists",
    "session_file_created",
    "restricted_loopback_bind",
    "authenticated_ipc_connection",
    "durable_store_ready",
    "restart_replay_rejected",
    "fresh_health_after_restart",
    "crash_fail_closed",
}
AGGREGATE_SURFACE_TEXT = "GUI Shell Dashboard NavigationRail Runtime Status Invariant Status"
BASE_REQUIRED_EVIDENCE_BUNDLE_KINDS = {
    "setup_doctor",
    "broker_smoke",
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


def _contains_surface_label(value: Any, label: str) -> bool:
    return bool(re.search(re.escape(label), str(value or ""), flags=re.IGNORECASE))


def _contains_all_required_surfaces(value: Any) -> bool:
    return all(_contains_surface_label(value, label) for label in REQUIRED_VISIBLE_SURFACES)


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
    if not isinstance(diagnostic_tree, dict):
        errors.append("可視 surface 証拠に diagnostic_tree がない")
    else:
        observed = diagnostic_tree.get("observed_elements")
        if not isinstance(observed, list) or not observed:
            errors.append("可視 surface の diagnostic_tree.observed_elements がない")
        else:
            required_keys = {"element_key", "runtime_id", "parent_runtime_id", "name", "automation_id", "control_type", "class_name", "framework_id", "supported_patterns"}
            for index, element in enumerate(observed[: min(len(observed), 20)]):
                if not isinstance(element, dict):
                    errors.append(f"可視 surface の観測要素 {index} が object ではない")
                    continue
                missing = sorted(required_keys - set(element))
                if missing:
                    errors.append(f"可視 surface の観測要素 {index} に診断 key がない: {', '.join(missing)}")
                    break
        tree_edges = diagnostic_tree.get("tree_edges")
        if not isinstance(tree_edges, list):
            errors.append("可視 surface の diagnostic_tree.tree_edges がない")
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
        for field in ("installed_manifest_sha256", "app_artifact_sha256", "broker_artifact_sha256", "evidence_bundle_sha256"):
            if not _is_sha256_tag(provenance.get(field)):
                errors.append(f"provenance.{field} には sha256 tag が必要である")
        artifact_hash = _get(data, "artifact.sha256")
        if _is_sha256_tag(artifact_hash) and provenance.get("app_artifact_sha256") != artifact_hash:
            errors.append("provenance.app_artifact_sha256 は artifact.sha256 と一致しなければならない")

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
                "isolated_config_dir",
                "isolated_audit_dir",
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
    if _get(data, "first_run.status") != "passed":
        errors.append("first_run.status は passed でなければならない")
    if not _is_true(data, "first_run.launched_from_installed_path"):
        errors.append("first run が installed app path から起動しなかった")
    if not _is_true(data, "first_run.process_running_after_launch"):
        errors.append("起動後に process が動作していることを確認できなかった")
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
    if _get(data, "first_run.broker_transport") != "authenticated_loopback_tcp":
        errors.append("first_run.broker_transport は authenticated_loopback_tcp でなければならない")
    if not _is_true(data, "first_run.no_python_runtime_requested"):
        errors.append("first run が no-Python runtime evidence mode を要求しなかった")
    if not _is_true(data, "first_run.python_runtime_path_scrubbed"):
        errors.append("first-run 起動前に Python runtime PATH scrub が適用されなかった")
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
            "flutter_semantics_runtime_export",
        }:
            errors.append("可視 surface の evidence source は uiautomation、accessibility_tree、flutter_semantics_runtime_export のいずれかでなければならない")
        if not surface_evidence.get("path"):
            errors.append("可視 surface の evidence path がない")
        errors.extend(_validate_surface_match_evidence(surface_evidence))
    if not _is_true(data, "first_run.config_created"):
        errors.append("first-run config の作成を確認できなかった")
    if not _is_true(data, "first_run.config_json_valid"):
        errors.append("first-run config JSON の妥当性を確認できなかった")
    if not _get(data, "first_run.config_path"):
        errors.append("first-run config path がない")
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
            "-BrokerHelperExe と -NoPythonRuntime を指定して Windows installed first-run smoke を実行し、妥当な release_evidence/windows_installed_smoke.json を記録する。",
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
        if setup.get("status") not in ("pass", "warning"):
            errors.append("setup_doctor.status は pass または warning でなければならない")
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
        if not setup.get("ran_from_installed_app_path"):
            errors.append("Setup Doctor が installed app path から実行されなかった")
        if not setup.get("operator_readable"):
            errors.append("Setup Doctor の operator readability を確認できなかった")
        if setup.get("installer_grants_authority") is not False:
            errors.append("Setup Doctor の installer_grants_authority は false でなければならない")
        if setup.get("installer_silently_approves_permissions") is not False:
            errors.append("Setup Doctor の installer_silently_approves_permissions は false でなければならない")
        checks = setup.get("checks")
        if not isinstance(checks, list) or not checks:
            errors.append("Setup Doctor の checks は空でない list でなければならない")
        else:
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
            if not source.get("collector_version"):
                errors.append("broker の evidence_source.collector_version がない")
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
        if broker.get("replay_error_code") != "broker_replay_detected":
            errors.append("broker の replay_error_code は broker_replay_detected でなければならない")
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
            "installed Rust broker helper に対して installer/windows/collect_broker_smoke.ps1 を実行し、測定済みの IPC/restart/crash field provenance だけを release_evidence/windows_installed_smoke.json に含める。",
        )
    return _passed(
        "windows_broker_installed_smoke",
        "Windows installed-path broker の launch/connect/restart/crash evidence が機械検証に合格した。no-Python/no-FFI は個別に分類された static evidence または installed-launch evidence のままである。",
        "release candidate ごとに broker の installed-path smoke evidence を最新に保つ。",
    )


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
            "product release claim の前に、audit_anchor.key、audit_anchor.json、audit.jsonl に対する Windows installed-path の ACL/DPAPI、external-anchor、signed-evidence のいずれかの証明を収集する。",
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
                "product release claim の前に、audit_anchor.key、audit_anchor.json、audit.jsonl に対する Windows installed-path の ACL/DPAPI、external-anchor、signed-evidence のいずれかの証明を収集する。",
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
