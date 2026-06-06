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
        errors.append("visible surfaces evidence detected or failed to rule out aggregate surface shortcut")
    if surface_evidence.get("surface_match_requirements_met") is not True:
        errors.append("visible surfaces evidence did not confirm individual surface match requirements")

    surface_matches = surface_evidence.get("surface_matches")
    if not isinstance(surface_matches, dict):
        errors.append("visible surfaces evidence surface_matches missing")
        return errors

    matched_element_keys: list[str] = []
    aggregate_text = _normalised_text(AGGREGATE_SURFACE_TEXT).casefold()
    for label in sorted(REQUIRED_VISIBLE_SURFACES):
        match = surface_matches.get(label)
        if not isinstance(match, dict):
            errors.append(f"{label} surface match evidence missing")
            continue
        if match.get("matched") is not True:
            errors.append(f"{label} surface match must be true")
        name = str(match.get("name") or "")
        automation_id = str(match.get("automation_id") or "")
        control_type = str(match.get("control_type") or "")
        element_key = str(match.get("element_key") or "")
        if not element_key:
            errors.append(f"{label} surface match element_key missing")
        else:
            matched_element_keys.append(element_key)
        if not control_type:
            errors.append(f"{label} surface match control_type missing")
        if not (_contains_surface_label(name, label) or _contains_surface_label(automation_id, label)):
            errors.append(f"{label} surface match name or automation_id must contain the surface label")

        element_text = _normalised_text(f"{name} {automation_id}")
        if _contains_all_required_surfaces(element_text):
            errors.append(f"{label} surface match uses one aggregate element containing all required labels")
        if aggregate_text and aggregate_text in element_text.casefold():
            errors.append(f"{label} surface match uses the forbidden aggregate native surface title")

    if (
        len(matched_element_keys) == len(REQUIRED_VISIBLE_SURFACES)
        and len(set(matched_element_keys)) == 1
    ):
        errors.append("all required surfaces rely on a single automation element")
    diagnostic_tree = surface_evidence.get("diagnostic_tree")
    if not isinstance(diagnostic_tree, dict):
        errors.append("visible surfaces evidence diagnostic_tree missing")
    else:
        observed = diagnostic_tree.get("observed_elements")
        if not isinstance(observed, list) or not observed:
            errors.append("visible surfaces diagnostic_tree.observed_elements missing")
        else:
            required_keys = {"element_key", "runtime_id", "parent_runtime_id", "name", "automation_id", "control_type", "class_name", "framework_id", "supported_patterns"}
            for index, element in enumerate(observed[: min(len(observed), 20)]):
                if not isinstance(element, dict):
                    errors.append(f"visible surfaces observed element {index} is not an object")
                    continue
                missing = sorted(required_keys - set(element))
                if missing:
                    errors.append(f"visible surfaces observed element {index} missing diagnostic keys: {', '.join(missing)}")
                    break
        tree_edges = diagnostic_tree.get("tree_edges")
        if not isinstance(tree_edges, list):
            errors.append("visible surfaces diagnostic_tree.tree_edges missing")
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
        return ["field_provenance object missing"]
    for group, (source_type, evidence_classes) in required_field_provenance.items():
        entry = provenance.get(group)
        if not isinstance(entry, dict):
            errors.append(f"field_provenance.{group} missing")
            continue
        if entry.get("source_type") != source_type:
            errors.append(f"field_provenance.{group}.source_type must be {source_type}")
        observed_classes = entry.get("evidence_class")
        if isinstance(observed_classes, str):
            observed = {observed_classes}
        elif isinstance(observed_classes, list):
            observed = {item for item in observed_classes if isinstance(item, str)}
        else:
            observed = set()
        if not observed or observed.isdisjoint(evidence_classes):
            errors.append(
                f"field_provenance.{group}.evidence_class must include one of {', '.join(sorted(evidence_classes))}"
            )
        if entry.get("formal_release_input") is not True:
            errors.append(f"field_provenance.{group}.formal_release_input must be true")
    if check_unsupported_claims:
        unsupported = provenance.get("unsupported_claims")
        if unsupported not in (None, []):
            errors.append("field_provenance.unsupported_claims must be empty for strict Windows evidence")
    return errors


def _path_contains_run_id(path_value: Any, run_id: str) -> bool:
    path = str(path_value or "").replace("/", "\\").casefold()
    return bool(run_id) and run_id.casefold() in path


def validate_provenance_and_isolation(data: dict[str, Any], path: Path = DEFAULT_EVIDENCE_PATH) -> EvidenceResult:
    errors: list[str] = []
    provenance = data.get("provenance")
    if not isinstance(provenance, dict):
        errors.append("provenance object missing")
    else:
        run_id = str(provenance.get("run_id") or "")
        if not run_id:
            errors.append("provenance.run_id missing")
        source_commit = str(provenance.get("source_commit") or "")
        if not SOURCE_COMMIT_RE.match(source_commit):
            errors.append("provenance.source_commit must be a 40-character commit SHA")
        if provenance.get("source_worktree_clean") is not True:
            errors.append("provenance.source_worktree_clean must be true")
        if str(provenance.get("source_status_porcelain") or "") != "":
            errors.append("provenance.source_status_porcelain must be empty")
        for field in ("build_command", "build_timestamp", "staged_manifest_path"):
            if not provenance.get(field):
                errors.append(f"provenance.{field} missing")
        for field in ("installed_manifest_sha256", "app_artifact_sha256", "broker_artifact_sha256", "evidence_bundle_sha256"):
            if not _is_sha256_tag(provenance.get(field)):
                errors.append(f"provenance.{field} must be tagged sha256")
        artifact_hash = _get(data, "artifact.sha256")
        if _is_sha256_tag(artifact_hash) and provenance.get("app_artifact_sha256") != artifact_hash:
            errors.append("provenance.app_artifact_sha256 must match artifact.sha256")

        isolation = provenance.get("isolation")
        if not isinstance(isolation, dict):
            errors.append("provenance.isolation object missing")
        else:
            if isolation.get("uses_shared_fixed_install_root") is not False:
                errors.append("provenance.isolation.uses_shared_fixed_install_root must be false")
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
                    errors.append(f"provenance.isolation.{field} missing")
                elif not _path_contains_run_id(value, run_id):
                    errors.append(f"provenance.isolation.{field} must contain run_id")
            install_root = str(isolation.get("isolated_install_root") or "").replace("/", "\\").casefold()
            if install_root.endswith(LEGACY_FIXED_INSTALL_ROOT_SUFFIX):
                errors.append("provenance.isolation.isolated_install_root uses legacy shared fixed install root")

        bundle_files = provenance.get("evidence_bundle_files")
        if not isinstance(bundle_files, list) or not bundle_files:
            errors.append("provenance.evidence_bundle_files missing")
        else:
            kinds = set()
            for index, item in enumerate(bundle_files):
                if not isinstance(item, dict):
                    errors.append(f"provenance.evidence_bundle_files[{index}] must be an object")
                    continue
                kind = item.get("kind")
                if isinstance(kind, str):
                    kinds.add(kind)
                if not item.get("path"):
                    errors.append(f"provenance.evidence_bundle_files[{index}].path missing")
                if not _is_sha256_tag(item.get("sha256")):
                    errors.append(f"provenance.evidence_bundle_files[{index}].sha256 must be tagged sha256")
            missing = BASE_REQUIRED_EVIDENCE_BUNDLE_KINDS - kinds
            if missing:
                errors.append(f"provenance.evidence_bundle_files missing kinds: {', '.join(sorted(missing))}")

        errors.extend(_validate_field_provenance(data))

        if path.exists():
            actual_hash = _hash_file(path)
            if _is_sha256_tag(provenance.get("final_evidence_sha256")) and provenance.get("final_evidence_sha256") != actual_hash:
                errors.append("provenance.final_evidence_sha256 does not match this evidence file")

    if errors:
        return _failed(
            "windows_evidence_provenance_isolation",
            "; ".join(errors),
            "Collect Windows evidence from a unique staged run with source commit, clean worktree, artifact hashes, isolated install/runtime/config/audit/store paths, field provenance, and evidence bundle hashes.",
        )
    return _passed(
        "windows_evidence_provenance_isolation",
        "Windows installed evidence is tied to a clean source commit, isolated run paths, artifact hashes, and field-level evidence provenance.",
        "Keep every Windows formal evidence run isolated and commit-linked.",
    )


def load_evidence(path: Path = DEFAULT_EVIDENCE_PATH) -> tuple[dict[str, Any] | None, str | None]:
    if not path.exists():
        return None, f"{path.relative_to(ROOT)} missing"
    try:
        payload = json.loads(path.read_text(encoding="utf-8-sig"))
    except json.JSONDecodeError as exc:
        return None, f"{path.relative_to(ROOT)} is invalid JSON: {exc}"
    if not isinstance(payload, dict):
        return None, f"{path.relative_to(ROOT)} must contain a JSON object"
    return payload, None


def validate_installer_first_run(data: dict[str, Any]) -> EvidenceResult:
    errors: list[str] = []
    if data.get("platform") != "windows":
        errors.append("platform must be windows")
    source = _get(data, "evidence_source")
    if not isinstance(source, dict):
        errors.append("evidence_source object missing")
    else:
        if source.get("collector") != "installer/windows/collect_installed_smoke.ps1":
            errors.append("evidence_source.collector must be the Windows installed smoke collector")
        if not source.get("collector_version"):
            errors.append("evidence_source.collector_version missing")
        if source.get("manual_confirmation") is not False:
            errors.append("manual confirmation evidence is not accepted for strict release")
    if not SHA256_RE.match(str(_get(data, "artifact.sha256") or "")):
        errors.append("artifact.sha256 must be tagged sha256")
    if not _is_true(data, "artifact.installed_exe_exists"):
        errors.append("installed executable existence was not confirmed")
    installed_exe_path = str(_get(data, "artifact.installed_exe_path") or "")
    if not installed_exe_path.lower().endswith(".exe"):
        errors.append("artifact.installed_exe_path must point to the installed Flutter executable")
    if _get(data, "first_run.status") != "passed":
        errors.append("first_run.status must be passed")
    if not _is_true(data, "first_run.launched_from_installed_path"):
        errors.append("first run did not launch from installed app path")
    if not _is_true(data, "first_run.process_running_after_launch"):
        errors.append("process was not confirmed running after launch")
    if not isinstance(_get(data, "first_run.process_id"), int):
        errors.append("first_run.process_id missing")
    if not isinstance(_get(data, "first_run.main_window_handle"), int) or _get(data, "first_run.main_window_handle") == 0:
        errors.append("MainWindowHandle was not confirmed")
    if not _is_true(data, "first_run.first_window_visible"):
        errors.append("first window visibility was not confirmed")
    if not _is_true(data, "first_run.broker_mediated_launch"):
        errors.append("first run was not launched through the Rust broker")
    if not _get(data, "first_run.broker_helper_path"):
        errors.append("first_run.broker_helper_path missing")
    if not _get(data, "first_run.broker_endpoint_file"):
        errors.append("first_run.broker_endpoint_file missing")
    if not _is_true(data, "first_run.broker_endpoint_created"):
        errors.append("broker endpoint file creation was not confirmed for first run")
    if _get(data, "first_run.broker_transport") != "authenticated_loopback_tcp":
        errors.append("first_run.broker_transport must be authenticated_loopback_tcp")
    if not _is_true(data, "first_run.no_python_runtime_requested"):
        errors.append("first run did not request no-Python runtime evidence mode")
    if not _is_true(data, "first_run.python_runtime_path_scrubbed"):
        errors.append("Python runtime PATH scrub was not applied before first-run launch")
    if _get(data, "first_run.python_path_entries_remaining_count") != 0:
        errors.append("Python PATH entries remained visible before first-run launch")
    python_commands = _get(data, "first_run.python_commands_visible_after_scrub")
    if not isinstance(python_commands, list):
        errors.append("first_run.python_commands_visible_after_scrub must be a list")
    elif python_commands:
        errors.append("Python commands remained visible before first-run launch")
    if not _is_true(data, "first_run.visible_surfaces_complete"):
        errors.append("first_run.visible_surfaces_complete must be true")
    visible = set(_get(data, "first_run.visible_surfaces") or [])
    for label in sorted(REQUIRED_VISIBLE_SURFACES):
        if label not in visible:
            errors.append(f"{label} was not recorded as visible")
    surface_evidence = _get(data, "first_run.visible_surfaces_evidence")
    if not isinstance(surface_evidence, dict):
        errors.append("visible surfaces evidence missing")
    else:
        if surface_evidence.get("source") not in {
            "uiautomation",
            "accessibility_tree",
            "flutter_semantics_runtime_export",
        }:
            errors.append("visible surfaces evidence source must be uiautomation, accessibility_tree, or flutter_semantics_runtime_export")
        if not surface_evidence.get("path"):
            errors.append("visible surfaces evidence path missing")
        errors.extend(_validate_surface_match_evidence(surface_evidence))
    if not _is_true(data, "first_run.config_created"):
        errors.append("first-run config creation was not confirmed")
    if not _is_true(data, "first_run.config_json_valid"):
        errors.append("first-run config JSON validity was not confirmed")
    if not _get(data, "first_run.config_path"):
        errors.append("first-run config path missing")
    if not _is_true(data, "first_run.audit_dir_writable"):
        errors.append("audit directory writability was not confirmed")
    probe = _get(data, "first_run.audit_write_probe")
    if not isinstance(probe, dict):
        errors.append("audit write probe missing")
    elif not all(probe.get(key) is True for key in ("attempted", "write", "read", "delete")):
        errors.append("audit write/read/delete probe did not pass")
    if not _get(data, "first_run.audit_dir"):
        errors.append("audit dir path missing")
    if not _is_false(data, "first_run.installer_grants_authority"):
        errors.append("installer authority boundary was not confirmed false")
    if not _is_false(data, "first_run.installer_silently_approves_permissions"):
        errors.append("silent approval boundary was not confirmed false")
    if errors:
        return _failed(
            "windows_installer_first_run_smoke",
            "; ".join(errors),
            "Run the Windows installed first-run smoke with -BrokerHelperExe and -NoPythonRuntime and record valid release_evidence/windows_installed_smoke.json.",
        )
    return _passed(
        "windows_installer_first_run_smoke",
        "Windows installed executable first-run smoke evidence passed machine validation.",
        "Keep Windows installed first-run evidence current for release candidates.",
    )


def validate_setup_doctor(data: dict[str, Any]) -> EvidenceResult:
    errors: list[str] = []
    setup = _get(data, "setup_doctor")
    if not isinstance(setup, dict):
        errors.append("setup_doctor object missing")
    else:
        if setup.get("formal_product_evidence") is not True:
            errors.append("Setup Doctor must be installed-app product evidence, not an external probe")
        if setup.get("status") not in ("pass", "warning"):
            errors.append("setup_doctor.status must be pass or warning")
        evidence_source = setup.get("evidence_source")
        if not isinstance(evidence_source, dict):
            errors.append("Setup Doctor evidence_source missing")
        else:
            if evidence_source.get("source_kind") != "installed_app_machine_readable_export":
                errors.append("Setup Doctor evidence_source.source_kind must be installed_app_machine_readable_export")
            if evidence_source.get("product_generated") is not True:
                errors.append("Setup Doctor evidence must be generated by the installed app")
            if evidence_source.get("collector_derives_checks") is not False:
                errors.append("Setup Doctor collector must not derive product diagnostic checks")
            if evidence_source.get("synthetic") is not False:
                errors.append("synthetic Setup Doctor evidence is not accepted")
            if not evidence_source.get("command"):
                errors.append("Setup Doctor evidence_source.command missing")
        if not setup.get("ran_from_installed_app_path"):
            errors.append("Setup Doctor did not run from installed app path")
        if not setup.get("operator_readable"):
            errors.append("Setup Doctor operator readability was not confirmed")
        if setup.get("installer_grants_authority") is not False:
            errors.append("Setup Doctor installer_grants_authority must be false")
        if setup.get("installer_silently_approves_permissions") is not False:
            errors.append("Setup Doctor installer_silently_approves_permissions must be false")
        checks = setup.get("checks")
        if not isinstance(checks, list) or not checks:
            errors.append("Setup Doctor checks must be a non-empty list")
        else:
            if any(check.get("status") == "fail" for check in checks if isinstance(check, dict)):
                errors.append("Setup Doctor contains failing checks")
            if any(check.get("grants_authority") is not False for check in checks if isinstance(check, dict)):
                errors.append("Setup Doctor check grants authority or lacks grants_authority=false")
            check_ids = {
                check.get("check_id")
                for check in checks
                if isinstance(check, dict) and isinstance(check.get("check_id"), str)
            }
            missing = REQUIRED_SETUP_CHECKS - check_ids
            if missing:
                errors.append(f"Setup Doctor missing required checks: {', '.join(sorted(missing))}")
            if any(not check.get("recovery_instruction") for check in checks if isinstance(check, dict)):
                errors.append("Setup Doctor checks must include recovery instructions")
    if errors:
        return _failed(
            "windows_setup_doctor_smoke",
            "; ".join(errors),
            "Run native Windows installed smoke so the installed app writes machine-readable Setup Doctor product evidence; external collector probe output is not accepted as product evidence.",
        )
    return _passed(
        "windows_setup_doctor_smoke",
        "Windows installed-path Setup Doctor evidence passed machine validation.",
        "Keep Windows Setup Doctor evidence current for release candidates.",
    )


def validate_broker_smoke(data: dict[str, Any]) -> EvidenceResult:
    errors: list[str] = []
    broker = _get(data, "broker")
    if not isinstance(broker, dict):
        errors.append("broker evidence object missing")
    else:
        if broker.get("status") != "passed":
            errors.append("broker.status must be passed")
        source = broker.get("evidence_source")
        if not isinstance(source, dict):
            errors.append("broker evidence_source missing")
        else:
            if source.get("collector") != "installer/windows/collect_broker_smoke.ps1":
                errors.append("broker evidence_source.collector must be installer/windows/collect_broker_smoke.ps1")
            if not source.get("collector_version"):
                errors.append("broker evidence_source.collector_version missing")
            if source.get("synthetic") is not False:
                errors.append("synthetic broker evidence is not accepted")
            if not source.get("command"):
                errors.append("broker evidence_source.command missing")
        field_provenance = broker.get("field_provenance")
        if not isinstance(field_provenance, dict):
            errors.append("broker field_provenance missing")
        for field in sorted(REQUIRED_BROKER_TRUE_FIELDS):
            if broker.get(field) is not True:
                errors.append(f"broker.{field} must be true")
            if isinstance(field_provenance, dict):
                entry = field_provenance.get(field)
                if not isinstance(entry, dict):
                    errors.append(f"broker field_provenance.{field} missing")
                else:
                    if entry.get("source_type") != "directly_measured":
                        errors.append(f"broker field_provenance.{field}.source_type must be directly_measured")
                    evidence_class = entry.get("evidence_class")
                    if evidence_class not in ("LIVE_RUNTIME", "EXTERNAL_EVIDENCE"):
                        errors.append(f"broker field_provenance.{field}.evidence_class must be LIVE_RUNTIME or EXTERNAL_EVIDENCE")
        if broker.get("endpoint_host") != "127.0.0.1":
            errors.append("broker endpoint_host must be 127.0.0.1")
        if broker.get("replay_error_code") != "broker_replay_detected":
            errors.append("broker replay_error_code must be broker_replay_detected")
        if "python_runtime_required_for_authority" in broker:
            errors.append("broker python_runtime_required_for_authority top-level declaration is not accepted as measured broker evidence")
        if "flutter_rust_ffi_authority_bridge" in broker:
            errors.append("broker flutter_rust_ffi_authority_bridge top-level declaration is not accepted as measured broker evidence")
        declarations = broker.get("unmeasured_declarations")
        if declarations not in (None, {}):
            if not isinstance(declarations, dict):
                errors.append("broker unmeasured_declarations must be an object when present")
            else:
                for key, value in declarations.items():
                    if not isinstance(value, dict):
                        errors.append(f"broker unmeasured_declarations.{key} must be an object")
                        continue
                    if value.get("formal_runtime_proof") is not False:
                        errors.append(f"broker unmeasured_declarations.{key}.formal_runtime_proof must be false")
        broker_errors = broker.get("errors")
        if broker_errors not in (None, []) and not (isinstance(broker_errors, list) and len(broker_errors) == 0):
            errors.append("broker errors must be empty")
    if errors:
        return _failed(
            "windows_broker_installed_smoke",
            "; ".join(errors),
            "Run installer/windows/collect_broker_smoke.ps1 against the installed Rust broker helper and include only measured IPC/restart/crash field provenance in release_evidence/windows_installed_smoke.json.",
        )
    return _passed(
        "windows_broker_installed_smoke",
        "Windows installed-path broker launch/connect/restart/crash evidence passed machine validation; no-Python/no-FFI remain separately classified static or installed-launch evidence.",
        "Keep broker installed-path smoke evidence current for release candidates.",
    )


def validate_audit_anchor_external_tamper_evidence(data: dict[str, Any]) -> EvidenceResult:
    errors: list[str] = []
    provenance = data.get("provenance")
    if not isinstance(provenance, dict):
        errors.append("provenance object missing for audit anchor evidence")
    else:
        bundle_files = provenance.get("evidence_bundle_files")
        if not isinstance(bundle_files, list) or not bundle_files:
            errors.append("provenance.evidence_bundle_files missing audit anchor evidence")
        else:
            kinds = {item.get("kind") for item in bundle_files if isinstance(item, dict)}
            missing = AUDIT_REQUIRED_EVIDENCE_BUNDLE_KINDS - {kind for kind in kinds if isinstance(kind, str)}
            if missing:
                errors.append(f"provenance.evidence_bundle_files missing kinds: {', '.join(sorted(missing))}")
    errors.extend(
        _validate_field_provenance(
            data,
            AUDIT_REQUIRED_FIELD_PROVENANCE,
            check_unsupported_claims=False,
        )
    )
    evidence = data.get("audit_anchor_external_tamper_evidence")
    if not isinstance(evidence, dict):
        errors.append("audit_anchor_external_tamper_evidence object missing")
    else:
        if evidence.get("status") != "passed":
            errors.append("audit_anchor_external_tamper_evidence.status must be passed")
        if evidence.get("installed_path_verified") is not True:
            errors.append("audit anchor evidence must be measured from the installed app path")
        if evidence.get("key_anchor_log_same_user_rewrite_mitigated") is not True:
            errors.append("same-user key+anchor+log rewrite mitigation must be verified")
        protection_checks = [
            "windows_acl_verified",
            "dpapi_verified",
            "external_anchor_verified",
            "signed_evidence_verified",
        ]
        if not any(evidence.get(field) is True for field in protection_checks):
            errors.append("audit anchor evidence must verify Windows ACL, DPAPI, external anchor, or signed evidence")
        if evidence.get("administrator_root_resistance_claimed") is True and not (
            evidence.get("external_anchor_verified") is True
            or evidence.get("signed_evidence_verified") is True
        ):
            errors.append("administrator/root resistance requires external anchor or signed evidence")

        source = evidence.get("evidence_source")
        if not isinstance(source, dict):
            errors.append("audit anchor evidence_source missing")
        else:
            if source.get("source_kind") not in {
                "windows_acl_dpapi_probe",
                "external_anchor",
                "signed_evidence",
            }:
                errors.append("audit anchor evidence_source.source_kind must be windows_acl_dpapi_probe, external_anchor, or signed_evidence")
            evidence_class = source.get("evidence_class")
            if evidence_class not in ("LIVE_RUNTIME", "EXTERNAL_EVIDENCE"):
                errors.append("audit anchor evidence_source.evidence_class must be LIVE_RUNTIME or EXTERNAL_EVIDENCE")
            if source.get("synthetic") is not False:
                errors.append("synthetic audit anchor evidence is not accepted")
            if not source.get("command"):
                errors.append("audit anchor evidence_source.command missing")
            if not source.get("path"):
                errors.append("audit anchor evidence_source.path missing")
            if not _is_sha256_tag(source.get("sha256")):
                errors.append("audit anchor evidence_source.sha256 must be tagged sha256")
    if errors:
        return _failed(
            "audit_anchor_external_tamper_evidence_proof",
            "; ".join(errors),
            "Collect Windows installed-path ACL/DPAPI, external-anchor, or signed-evidence proof for audit_anchor.key, audit_anchor.json, and audit.jsonl before product release claim.",
        )
    return _passed(
        "audit_anchor_external_tamper_evidence_proof",
        "Windows installed-path audit anchor external tamper-evidence proof passed machine validation.",
        "Keep audit anchor key-protection or external-anchor evidence current for release candidates.",
    )


def validate_windows_release_evidence(path: Path = DEFAULT_EVIDENCE_PATH) -> list[EvidenceResult]:
    data, error = load_evidence(path)
    if data is None:
        return [
            _failed(
                "windows_evidence_provenance_isolation",
                error or "Windows installed smoke evidence missing",
                "Create release_evidence/windows_installed_smoke.json from an isolated native Windows run tied to the exact source commit and artifact hashes.",
            ),
            _failed(
                "windows_installer_first_run_smoke",
                error or "Windows installed smoke evidence missing",
                "Create release_evidence/windows_installed_smoke.json from a native Windows installed-app smoke with broker-mediated Flutter .exe launch, -NoPythonRuntime launch, measured window, visible-surface, config, and audit probe evidence.",
            ),
            _failed(
                "windows_setup_doctor_smoke",
                error or "Windows Setup Doctor evidence missing",
                "Run native Windows installed smoke so the installed app writes machine-readable Setup Doctor product evidence; external collector probe output is not accepted as product evidence.",
            ),
            _failed(
                "windows_broker_installed_smoke",
                error or "Windows broker installed smoke evidence missing",
                "Run installer/windows/collect_broker_smoke.ps1 and include broker evidence in release_evidence/windows_installed_smoke.json.",
            ),
            _failed(
                "audit_anchor_external_tamper_evidence_proof",
                error or "Audit anchor external tamper-evidence proof missing",
                "Collect Windows installed-path ACL/DPAPI, external-anchor, or signed-evidence proof for audit_anchor.key, audit_anchor.json, and audit.jsonl before product release claim.",
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
