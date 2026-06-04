from __future__ import annotations

import argparse
import json
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_EVIDENCE_PATH = ROOT / "release_evidence" / "windows_installed_smoke.json"
SHA256_RE = re.compile(r"^sha256:[0-9a-f]{64}$")
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
    return errors


def load_evidence(path: Path = DEFAULT_EVIDENCE_PATH) -> tuple[dict[str, Any] | None, str | None]:
    if not path.exists():
        return None, f"{path.relative_to(ROOT)} missing"
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
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
        if surface_evidence.get("source") not in {"uiautomation", "accessibility_tree"}:
            errors.append("visible surfaces evidence source must be uiautomation or accessibility_tree")
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
        if setup.get("status") not in ("pass", "warning"):
            errors.append("setup_doctor.status must be pass or warning")
        evidence_source = setup.get("evidence_source")
        if not isinstance(evidence_source, dict):
            errors.append("Setup Doctor evidence_source missing")
        else:
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
            "Run Setup Doctor from the installed Windows app path and record valid diagnostics evidence.",
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
        for field in sorted(REQUIRED_BROKER_TRUE_FIELDS):
            if broker.get(field) is not True:
                errors.append(f"broker.{field} must be true")
        if broker.get("endpoint_host") != "127.0.0.1":
            errors.append("broker endpoint_host must be 127.0.0.1")
        if broker.get("replay_error_code") != "broker_replay_detected":
            errors.append("broker replay_error_code must be broker_replay_detected")
        if broker.get("python_runtime_required_for_authority") is not False:
            errors.append("broker python_runtime_required_for_authority must be false")
        if broker.get("flutter_rust_ffi_authority_bridge") is not False:
            errors.append("broker flutter_rust_ffi_authority_bridge must be false")
        broker_errors = broker.get("errors")
        if broker_errors not in (None, []) and not (isinstance(broker_errors, list) and len(broker_errors) == 0):
            errors.append("broker errors must be empty")
    if errors:
        return _failed(
            "windows_broker_installed_smoke",
            "; ".join(errors),
            "Run installer/windows/collect_broker_smoke.ps1 against the installed Rust broker helper and include the result in release_evidence/windows_installed_smoke.json.",
        )
    return _passed(
        "windows_broker_installed_smoke",
        "Windows installed-path broker launch/connect/restart/crash/no-Python/no-FFI evidence passed machine validation.",
        "Keep broker installed-path smoke evidence current for release candidates.",
    )


def validate_windows_release_evidence(path: Path = DEFAULT_EVIDENCE_PATH) -> list[EvidenceResult]:
    data, error = load_evidence(path)
    if data is None:
        return [
            _failed(
                "windows_installer_first_run_smoke",
                error or "Windows installed smoke evidence missing",
                "Create release_evidence/windows_installed_smoke.json from a native Windows installed-app smoke with broker-mediated Flutter .exe launch, -NoPythonRuntime launch, measured window, visible-surface, config, and audit probe evidence.",
            ),
            _failed(
                "windows_setup_doctor_smoke",
                error or "Windows Setup Doctor evidence missing",
                "Run Setup Doctor from the installed Windows app path and record non-synthetic required diagnostics evidence.",
            ),
            _failed(
                "windows_broker_installed_smoke",
                error or "Windows broker installed smoke evidence missing",
                "Run installer/windows/collect_broker_smoke.ps1 and include broker evidence in release_evidence/windows_installed_smoke.json.",
            ),
        ]
    return [validate_installer_first_run(data), validate_setup_doctor(data), validate_broker_smoke(data)]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--evidence", type=Path, default=DEFAULT_EVIDENCE_PATH)
    args = parser.parse_args()
    results = validate_windows_release_evidence(args.evidence)
    print(json.dumps([result.__dict__ for result in results], indent=2, sort_keys=True))
    return 1 if any(result.classification == "release_blocker" for result in results) else 0


if __name__ == "__main__":
    raise SystemExit(main())
