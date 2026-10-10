import base64
import copy
import hashlib
import io
import json
import re
import subprocess
import sys
import tarfile
import tempfile
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))
SPECS = ROOT / "specs"
DOC_SPECS = ROOT / "docs" / "specs"
CONTRACT_EXAMPLES = ROOT / "examples" / "contracts"
INVALID_CONTRACT_EXAMPLES = CONTRACT_EXAMPLES / "invalid"
SHELL_CORE = ROOT / "packages" / "shell_core"
RUST_HELPER = ROOT / "native" / "rust_helper"
DESKTOP_FLUTTER = ROOT / "apps" / "desktop_flutter"
MOBILE_FLUTTER = ROOT / "apps" / "mobile_flutter"
INSTALLER = ROOT / "installer"

from packages.shell_contracts import load_default_catalog
from packages.shell_core.adapter_loader import load_adapter, strip_authority_keys
from packages.shell_core.approval_queue import ApprovalQueue, canonical_hash
from packages.shell_core.authority_keys import AUTHORITY_KEYS
from packages.shell_core.content_exposure import project_approval_content
from packages.shell_core.invariant_evaluator import InvariantEvaluator
from packages.shell_core.normalization import normalize_inbound_payload, normalize_key
from packages.shell_core.permission_ledger import AUTHORITY_SOURCES, NON_AUTHORITY_SOURCES, PermissionLedger
from packages.shell_core.policy_evaluator import PolicyEvaluator
from packages.shell_core.persistence import JsonPersistence
from packages.shell_core.runtime_state import RuntimeState
from packages.shell_core.release_smoke import run_shell_core_release_smoke
from packages.shell_core.sensitive_action_router import SensitiveActionRouter
from packages.shell_core.state_snapshot import create_state_snapshot, deterministic_snapshot_json
from packages.shell_core.update_policy_store import UpdatePolicyStore
from packages.blue_tanuki_adapter.adapter import BlueTanukiAdapter
from packages.blue_tanuki_adapter.approvals import normalize_approval, projected_approval
from packages.blue_tanuki_adapter.authority_trace import metadata_attempts_authority
from packages.blue_tanuki_adapter.recovery import recovery_candidates
from packages.agent_runtime import AgentAdapterContract, AgentRuntimeContract
from tooling.agent_adapter_probe import build_adapter_record
from packages.runtime_catalog import RuntimeCatalog
from packages.shell_core.audit_chain import chain_event, verify_audit_chain
from tooling.schema_check.check_schemas import parse_json_text, validate_instance
from tooling.release_smoke import run_release_smokes
from tooling.evidence_bundle import build_evidence_bundle, validate_evidence_bundle
from tooling.manifest import build_manifest, matches_forbidden, working_tree_eol_errors
from tooling.packaging_portability_check import portable_path_errors, run_check
from tooling.release_gate_check import (
    BLOCKER_CAUSE_CATEGORIES,
    CURRENT_FACING_RELEASE_DOCS,
    RELEASE_TRACKS,
    blocker_cause_category_errors,
    blocker_scope_errors,
    release_blocker_doc_sync_errors,
    registry_blocker_names,
    unresolved_active_blockers,
    unresolved_windows_technical_complete_blockers,
)
from tooling.shell_snapshot import build_shell_snapshot
from tooling.validate_all import (
    ValidationStep,
    build_steps as build_validation_steps,
    python_step,
    run_step,
)
from tooling.windows_release_evidence import validate_windows_release_evidence
from tooling.broker_parity.run_authority_parity import DEFAULT_BROKER_START_TIMEOUT_SECONDS
from tooling.build_module_pruned_windows import (
    EXPECTED_CORE_IDS,
    EXPECTED_OPTIONAL_IDS,
    EXPECTED_REQUIRED_IDS,
    _validate_catalog,
    _flutter_candidates,
    dart_defines,
    resolve_module_plan,
)
from tooling.compare_module_builds_windows import (
    _safe_console_text,
    all_optional_defines,
    comparison_summary,
    validate_comparison_evidence,
)
from tooling.export_windows_product import (
    _build_child_environment,
    _cargo_target_directory,
    _require_clean_source,
    _artifact_inventory,
    _copy_bundle,
    _resolve_output_directory,
    _safe_extract_source,
    _validate_cargo_target_path,
    product_version_from_source,
    scan_credential_artifacts,
    validate_build_evidence,
    validate_export_inputs,
)
import tooling.export_windows_product as windows_export
import tooling.package_windows_product as windows_product_package
import tooling.package_windows_portable as windows_portable_package

REQUIRED_SCHEMA_NAMES = {
    "buffer_intervention_record",
    "workspace_diff",
    "workspace_inspection_request",
    "workspace_startup",
    "workspace_inspection_response",
    "runtime_dialogue_operation",
    "runtime_dialogue_request",
    "runtime_dialogue_submission_receipt",
    "runtime_dialogue_session",
    "runtime_dialogue_session_list",
    "runtime_dialogue_response",
    "runtime_dialogue_comparison",
    "evaluation_dataset",
    "evaluation_case",
    "evaluation_evaluator",
    "evaluation_dataset_registration",
    "regression_case_registration",
    "regression_case_receipt",
    "regression_case_list_request",
    "regression_case_list",
    "regression_case_delete_request",
    "regression_case_delete_receipt",
    "regression_case_delete_recovery_request",
    "regression_case_delete_recovery_receipt",
    "mcp_contract",
    "a2a_contract",
    "mcp_connection",
    "mcp_connection_receipt",
    "mcp_tool_call",
    "mcp_tool_call_receipt",
    "mcp_connection_list",
    "mcp_disconnect",
    "mcp_disconnect_receipt",
    "a2a_connection",
    "a2a_connection_receipt",
    "a2a_connection_list",
    "host_registration",
    "host_receipt",
    "host_list",
    "profile",
    "profile_receipt",
    "profile_list",
    "evaluation_experiment",
    "evaluation_result",
    "evaluation_comparison",
    "runtime_resource_query",
    "runtime_resource_observation",
    "runtime_lifecycle_request",
    "runtime_lifecycle_result",

    "action_envelope",
    "runtime",
    "adapter",
    "capability",
    "permission",
    "approval",
    "audit",
    "recovery",
    "diagnostic",
    "update",
    "update_candidate",
    "update_catalog",
    "update_receipt",
    "update_list",
    "update_trust",
    "notification",
    "notification_list",
    "notification_action",
    "observation_span",
    "observation_trace",
    "observation_metric",
    "observation_list",
    "content_exposure",
    "framework_risk_profile",
    "runtime_manifest",
    "adapter_manifest",
    "agent_runtime",
    "agent_adapter",
    "agent_session",
    "agent_workspace",
    "agent_task",
    "agent_task_scratch_recovery",
    "agent_task_request",
    "agent_cli_runtime_workspace_registration",
    "provider_model_selection",
    "agent_task_workspace_permission_request",
    "agent_task_workspace_permission",
    "agent_tool_call",
    "agent_diff",
    "agent_comparison",
    "agent_handoff",
    "agent_handoff_package",
    "gui_shell_compose",
    "gui_shell_compose_receipt",
    "gui_shell_preview",
    "gui_shell_preview_receipt",
    "gui_shell_edit_proposal",
    "gui_shell_edit_proposal_receipt",
    "gui_shell_export",
    "gui_shell_export_receipt",
    "gui_shell_export_manifest",
    "gui_shell_module_catalog",
    "gui_shell_module_build_evidence",
    "gui_shell_module_comparison_evidence",
    "gui_shell_windows_export_build",
    "ipc_request",
    "ipc_response",
    "broker_error",
    "broker_session",
    "broker_health",
    "broker_command_envelope",
    "desktop_broker_channel_request",
    "setup_doctor_report",
    "first_run_configuration",
    "first_run_configuration_request",
    "mobile_device_link_channel_request",
    "mobile_local_recovery_audit",
    "adapter_management_manifest",
    "adapter_management_request",
    "adapter_management_receipt",
    "adapter_management_list",
    "tray_stop_request",
    "tray_stop_response",
    "windows_tray_projection",
}

VISIBILITY_VALUES = ["none", "hash_only", "summary", "redacted", "full"]
BOUNDED_EXTENSION_FIXTURE = "llm_bounded_extension.valid.json"
BOUNDED_EXTENSION_RECORD_SCHEMAS = {
    "runtime": "runtime.schema.json",
    "adapter": "adapter.schema.json",
    "runtime_manifest": "runtime_manifest.schema.json",
    "adapter_manifest": "adapter_manifest.schema.json",
    "capability": "capability.schema.json",
    "permission": "permission.schema.json",
    "approval": "approval.schema.json",
    "audit_event": "audit.schema.json",
    "recovery_action": "recovery.schema.json",
    "content_exposure_policy": "content_exposure.schema.json",
    "update_policy": "update.schema.json",
}
RUST_HELPER_REQUIRED_SOURCES = {
    "lib.rs",
    "process.rs",
    "filesystem.rs",
    "network.rs",
    "diagnostics.rs",
    "update_verification.rs",
    "audit_hash.rs",
    "ipc.rs",
    "main.rs",
}
BROKER_REQUIRED_SOURCES = {
    "mod.rs",
    "protocol.rs",
    "audit.rs",
    "regression_case.rs",
    "mcp_center.rs",
    "a2a_center.rs",
    "host_center.rs",
    "profile_center.rs",
    "update_center.rs",
    "adapter_center.rs",
    "notification_center.rs",
    "observation_center.rs",
    "compose_center.rs",
    "ai_edit_center.rs",
    "export_center.rs",
}
BROKER_REQUIRED_SCHEMAS = {
    "ipc_request.schema.json",
    "ipc_response.schema.json",
    "agent_cli_runtime_workspace_registration.schema.json",
    "provider_model_selection.schema.json",
    "broker_error.schema.json",
    "broker_endpoint.schema.json",
    "broker_session.schema.json",
    "broker_health.schema.json",
    "broker_command_envelope.schema.json",
    "desktop_broker_channel_request.schema.json",
    "setup_doctor_report.schema.json",
    "first_run_configuration.schema.json",
    "first_run_configuration_request.schema.json",
    "profile.schema.json",
    "profile_receipt.schema.json",
    "profile_list.schema.json",
    "update_candidate.schema.json",
    "update_catalog.schema.json",
    "update_receipt.schema.json",
    "update_list.schema.json",
    "update_download_job.schema.json",
    "update_trust.schema.json",
    "notification.schema.json",
    "notification_list.schema.json",
    "notification_action.schema.json",
    "observation_span.schema.json",
    "observation_trace.schema.json",
    "observation_metric.schema.json",
    "observation_list.schema.json",
    "a2a_connection.schema.json",
    "a2a_connection_receipt.schema.json",
    "a2a_connection_list.schema.json",
    "host_registration.schema.json",
    "host_receipt.schema.json",
    "host_list.schema.json",
    "adapter_management_manifest.schema.json",
    "adapter_management_request.schema.json",
    "adapter_management_receipt.schema.json",
    "adapter_management_list.schema.json",
    "tray_stop_request.schema.json",
    "tray_stop_response.schema.json",
    "windows_tray_projection.schema.json",
    "gui_shell_compose.schema.json",
    "gui_shell_compose_receipt.schema.json",
    "gui_shell_preview.schema.json",
    "gui_shell_preview_receipt.schema.json",
    "gui_shell_edit_proposal.schema.json",
    "gui_shell_edit_proposal_receipt.schema.json",
    "gui_shell_export.schema.json",
    "gui_shell_export_receipt.schema.json",
    "gui_shell_export_manifest.schema.json",
    "gui_shell_module_catalog.schema.json",
    "gui_shell_windows_export_build.schema.json",
}
DESKTOP_FLUTTER_REQUIRED_FILES = {
    "lib/main.dart",
    "lib/screens/dashboard.dart",
    "lib/screens/trust_center.dart",
    "lib/screens/authority_map.dart",
    "lib/screens/setup_doctor.dart",
    "lib/screens/runtime_center.dart",
    "lib/screens/agent_center.dart",
    "lib/screens/permission_center.dart",
    "lib/screens/approval_center.dart",
    "lib/screens/audit_viewer.dart",
    "lib/screens/recovery_center.dart",
    "lib/screens/settings.dart",
    "lib/services/shell_core_client.dart",
    "lib/services/agent_coordination.dart",
    "lib/services/compose_client.dart",
    "lib/services/global_search_index.dart",
    "lib/services/windows_tray_client.dart",
    "windows/runner/tray_controller.cpp",
    "windows/runner/tray_controller.h",
    "lib/services/profile_client.dart",
    "lib/services/update_client.dart",
    "lib/services/observation_client.dart",
    "lib/screens/observability_center.dart",
    "lib/screens/trace_inspector.dart",
    "lib/models/generated_contracts.dart",
}
MOBILE_FLUTTER_REQUIRED_FILES = {
    "lib/main.dart",
    "lib/screens/mobile_dashboard.dart",
    "lib/screens/approval_review.dart",
    "lib/screens/notifications.dart",
    "lib/screens/runtime_status.dart",
    "lib/screens/emergency_stop.dart",
    "lib/screens/recovery_instruction.dart",
    "lib/services/device_link_client.dart",
    "lib/services/device_link_controller.dart",
    "lib/screens/device_connection.dart",
    "test/device_link_test.dart",
    "android/app/src/main/kotlin/com/example/gui_shell_mobile/MainActivity.kt",
    "android/app/src/main/kotlin/com/example/gui_shell_mobile/DeviceLinkModels.kt",
    "android/app/src/main/kotlin/com/example/gui_shell_mobile/DeviceLinkNativeService.kt",
    "android/app/src/main/kotlin/com/example/gui_shell_mobile/DeviceLinkNativeStore.kt",
    "android/app/src/main/kotlin/com/example/gui_shell_mobile/DeviceLinkTlsClient.kt",
    "android/app/src/main/kotlin/com/example/gui_shell_mobile/StrictJson.kt",
    "android/app/src/test/kotlin/com/example/gui_shell_mobile/DeviceLinkPolicyTest.kt",
    "android/app/src/test/kotlin/com/example/gui_shell_mobile/StrictJsonTest.kt",
    "ios/Runner/DeviceLinkModels.swift",
    "ios/Runner/DeviceLinkNativeService.swift",
    "ios/Runner/DeviceLinkNativeStore.swift",
    "ios/RunnerTests/RunnerTests.swift",
}
RELEASE_HARDENING_FILES = {
    "RELEASE_CHECKLIST.md",
    "SECURITY_REVIEW.md",
    "COMPATIBILITY_MATRIX.md",
    "CONFORMANCE_REPORT.md",
    "AUDIT_EVIDENCE.md",
    "INSTALLER_STATUS.md",
    "MOBILE_STATUS.md",
}
CLAIM_REVIEW_FILES = {
    "README.md",
    "CLAIM.md",
    "QUICKSTART.md",
    "ROADMAP.md",
    "VALIDATION.txt",
    "CONFORMANCE_REPORT.md",
    "docs/OPERATING_MODEL.md",
    "docs/COMPLETION_STRATEGY_INSTRUCTION.md",
}


def load_schema(name: str) -> dict:
    return json.loads((SPECS / name).read_text(encoding="utf-8"))


def load_contract_fixture(name: str) -> dict:
    return json.loads((CONTRACT_EXAMPLES / name).read_text(encoding="utf-8"))


def sha256_tagged(payload: bytes) -> str:
    return "sha256:" + hashlib.sha256(payload).hexdigest()


def metadata_permissions(adapter: dict) -> list[str]:
    # Adapterのmetadataは説明用に限り、permissionのようなmetadataは無視しなければならない。
    return list(adapter.get("declared_capabilities", []))


def can_create_authority_context(source: str, runtime_allowed: bool) -> bool:
    return source == "runtime" and runtime_allowed


def source_can_grant_authority(source: str) -> bool:
    return source in AUTHORITY_SOURCES


def render_approval_content(approval: dict) -> dict:
    visibility = approval["content_visibility"]
    if visibility == "none":
        return {}
    if visibility == "hash_only":
        return {"payload_hash": approval["payload_hash"]}
    if visibility == "summary":
        return {"summary": approval.get("summary", "")}
    if visibility == "redacted":
        return {"redacted_payload": approval.get("redacted_payload", {})}
    if visibility == "full":
        return {"full_payload": approval.get("full_payload", {})}
    raise ValueError(f"未知のcontent visibility: {visibility}")


def sensitive_action_mapping_is_complete(action: dict) -> bool:
    required = {
        "capability_id",
        "permission_id",
        "approval_state",
        "audit_event",
        "recovery_action",
    }
    if not required.issubset(action):
        return False
    audit_event = action["audit_event"]
    recovery_action = action["recovery_action"]
    return bool(audit_event.get("event_id")) and bool(recovery_action.get("recovery_id"))


def test_required_docs_exist() -> list[str]:
    errors = []
    required_docs = {
        "gui-shell-spec-v1.md",
        "adapter-conformance.md",
        "agent-runtime.md",
        "authority-strip-conformance.md",
        "content-exposure-policy.md",
        "approval-visibility-boundary.md",
        "a2a-contract.md",
        "a2a-connection-center.md",
        "runtime-catalog.md",
    }
    existing = {path.name for path in DOC_SPECS.glob("*.md")}
    for missing in sorted(required_docs - existing):
        errors.append(f"docs/specs/{missing} が存在しない")
    phase_mapping = ROOT / "docs" / "D4_POCKET_PHASE_MAPPING.md"
    if not phase_mapping.is_file():
        errors.append("docs/D4_POCKET_PHASE_MAPPING.md が存在しない")
    else:
        mapping_text = phase_mapping.read_text(encoding="utf-8")
        required_mappings = [
            *(f"| `Phase {phase}` |" for phase in range(46)),
            *(f"| `C{phase}` |" for phase in range(35)),
        ]
        errors.extend(
            f"D4 Phase／rev1 C対応表に行がない: {row}"
            for row in required_mappings
            if row not in mapping_text
        )
    return errors


def test_gui_shell_spec_v1_declares_core_boundaries() -> list[str]:
    path = DOC_SPECS / "gui-shell-spec-v1.md"
    if not path.exists():
        return ["docs/specs/gui-shell-spec-v1.md が存在しない"]
    text = path.read_text(encoding="utf-8")
    required_tokens = [
        "Runtime Operation Shell",
        "not a BLUE-TANUKI-specific GUI",
        "BLUE-TANUKI is the first reference runtime",
        "Flutter UI layer",
        "Shell Core",
        "Adapter",
        "Permission Ledger",
        "Approval Queue",
        "Audit Store",
        "Recovery Center",
        "Rust Native Helper",
        "Content Exposure Boundary",
        "Authority Strip",
        "Windows installed smoke tests",
        "explicit owner GO",
    ]
    return [
        f"docs/specs/gui-shell-spec-v1.md に必須tokenがない: {token}"
        for token in required_tokens
        if token not in text
    ]


def test_contract_fixtures_are_available() -> list[str]:
    errors = []
    expected = {f"{name}.valid.json" for name in REQUIRED_SCHEMA_NAMES}
    existing = {path.name for path in CONTRACT_EXAMPLES.glob("*.valid.json")}
    for missing in sorted(expected - existing):
        errors.append(f"examples/contracts/{missing} が存在しない")
    for name in sorted(expected & existing):
        try:
            fixture = load_contract_fixture(name)
        except Exception as exc:
            errors.append(f"examples/contracts/{name} を解析できない: {exc}")
            continue
        if not isinstance(fixture, dict):
            errors.append(f"examples/contracts/{name} はJSON objectでなければならない")
    return errors


def test_setup_doctor_report_schema_preserves_unknown_and_denies_authority() -> list[str]:
    schema = load_schema("setup_doctor_report.schema.json")
    valid = load_contract_fixture("setup_doctor_report.valid.json")
    invalid = load_contract_fixture("invalid/setup_doctor_report_authority_escalation.invalid.json")
    errors = validate_instance(valid, schema)
    if errors:
        return [f"Setup Doctorの正常報告例が契約に適合しない: {errors}"]
    if valid.get("status") != "warning" or valid["checks"][-1].get("status") != "unknown":
        return ["初回設定が未実装なら状態を不明のまま保持しなければならない"]
    if validate_instance(invalid, schema) == []:
        return ["環境診断報告のSchemaが権限昇格を拒否しない"]
    return []


def test_first_run_configuration_schema_is_fixed_and_non_authoritative() -> list[str]:
    schema = load_schema("first_run_configuration.schema.json")
    valid = load_contract_fixture("first_run_configuration.valid.json")
    invalid_authority = load_contract_fixture(
        "invalid/first_run_configuration_authority_escalation.invalid.json"
    )
    invalid_preferences = load_contract_fixture(
        "invalid/first_run_configuration_preference_escalation.invalid.json"
    )
    errors = validate_instance(valid, schema)
    if errors:
        return [f"初回UI設定の正常例が契約に適合しない: {errors}"]
    if valid["ui_preferences"] != {
        "theme": "system",
        "density": "compact",
        "locale": "ja-JP",
    }:
        return ["初回UI設定の既定値が現行Desktop UIと一致しない"]
    if validate_instance(invalid_authority, schema) == []:
        return ["初回UI設定Schemaが未知のauthority/permission fieldを拒否しない"]
    if validate_instance(invalid_preferences, schema) == []:
        return ["初回UI設定Schemaが未契約preferenceへの差し替えを拒否しない"]
    return []


def test_first_run_configuration_request_rejects_caller_paths_and_authority() -> list[str]:
    schema = load_schema("first_run_configuration_request.schema.json")
    valid = load_contract_fixture("first_run_configuration_request.valid.json")
    invalid = load_contract_fixture(
        "invalid/first_run_configuration_request_path.invalid.json"
    )
    errors = validate_instance(valid, schema)
    if errors:
        return [f"初回設定取得要求の正常例が契約に適合しない: {errors}"]
    if validate_instance(invalid, schema) == []:
        return ["初回設定取得要求Schemaが任意pathを拒否しない"]
    return []


def test_buffer_intervention_record_is_non_authoritative_and_evidence_bound() -> list[str]:
    schema = load_schema("buffer_intervention_record.schema.json")
    valid = load_contract_fixture("buffer_intervention_record.valid.json")
    invalid_authority = load_contract_fixture(
        "invalid/buffer_intervention_record_authority.invalid.json"
    )
    invalid_unproven = load_contract_fixture(
        "invalid/buffer_intervention_record_unproven_absorption.invalid.json"
    )
    errors = validate_instance(valid, schema)
    if errors:
        return [f"緩衝調停の正常な候補記録が契約に適合しない: {errors}"]
    if validate_instance(invalid_authority, schema) == []:
        return ["緩衝調停記録がAuthority field注入を拒否しない"]
    if validate_instance(invalid_unproven, schema) == []:
        return ["実作用・元Tool成功・独立照合なしの吸収成立をSchemaが拒否しない"]
    candidate_with_action = copy.deepcopy(valid)
    candidate_with_action["実作用"] = {
        "作用参照": "evidence:action-1",
        "作用種別": "環境識別固定",
        "変更前状態hash": "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
        "変更後状態hash": "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
        "復旧状態": "不要",
    }
    if validate_instance(candidate_with_action, schema) == []:
        return ["候補のみの記録が実作用を併記して誤昇格できる"]
    evidenced_absorption = copy.deepcopy(valid)
    evidenced_absorption.update(
        {
            "判定": "吸収成立",
            "理由コード": "独立照合済み",
            "未解決": [],
            "実作用": candidate_with_action["実作用"],
            "元Tool結果": {
                "状態": "成功",
                "終了値": 0,
                "結果hash": "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "証拠参照": "evidence:tool-result-1",
            },
            "独立照合": {
                "結果": "PASS",
                "成功条件ID": "BUF-T02",
                "証拠参照": "evidence:verify-1",
            },
            "比較証拠": {
                "対照証拠参照": [
                    "evidence:baseline-run-1",
                    "evidence:mediated-run-1",
                ],
                "条件一致hash": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "要求意味一致": True,
                "副作用照合": "PASS",
            },
        }
    )
    if validate_instance(evidenced_absorption, schema):
        return ["実作用・元Tool成功・独立照合を持つ記録が契約に適合しない"]
    incomplete_recovery = copy.deepcopy(evidenced_absorption)
    incomplete_recovery["実作用"]["復旧状態"] = "未完了"
    if validate_instance(incomplete_recovery, schema) == []:
        return ["復旧未完了の作用が吸収成立として通過する"]
    failed_side_effect_check = copy.deepcopy(evidenced_absorption)
    failed_side_effect_check["比較証拠"]["副作用照合"] = "FAIL"
    if validate_instance(failed_side_effect_check, schema) == []:
        return ["副作用照合FAILの介入が吸収成立として通過する"]
    return []


def schema_name_from_invalid_fixture(path: Path) -> str:
    stem = path.name.removesuffix(".invalid.json")
    schema_bases = sorted(REQUIRED_SCHEMA_NAMES, key=len, reverse=True)
    for base in schema_bases:
        if stem == base or stem.startswith(f"{base}_"):
            return base
    return stem.split("_", 1)[0]


def test_negative_contract_fixtures_cover_all_schemas() -> list[str]:
    invalid_paths = sorted(INVALID_CONTRACT_EXAMPLES.glob("*.invalid.json"))
    covered = {schema_name_from_invalid_fixture(path) for path in invalid_paths}
    errors = []
    for missing in sorted(REQUIRED_SCHEMA_NAMES - covered):
        errors.append(f"examples/contracts/invalid に{missing}用のnegative fixtureがない")
    if len(invalid_paths) < len(REQUIRED_SCHEMA_NAMES):
        errors.append("negative contract fixtureはすべてのschemaを対象にしなければならない")
    return errors


def test_adapter_authority_strip_schema() -> list[str]:
    errors = []
    schema = load_schema("adapter.schema.json")
    required = set(schema.get("required", []))
    authority_strip = schema["properties"].get("authority_strip", {})
    if "authority_strip" not in required:
        errors.append("adapter.schema.jsonはauthority_stripを必須にしなければならない")
    if authority_strip.get("const") is not True:
        errors.append("adapter.schema.jsonはauthority_strip=trueを必須にしなければならない")
    return errors


def test_inbound_authority_keys_are_stripped() -> list[str]:
    inbound = {
        "operation": "runtime.snapshot",
        "authority": "admin",
        "payload": {
            "message": "safe",
            "permission_grant": "fs:write",
            "nested": {"trust_level": "root", "value": 1},
        },
        "metadata": {"role": "owner", "source": "adapter"},
    }
    stripped = strip_authority_keys(inbound)
    encoded = json.dumps(stripped, sort_keys=True)
    errors = []
    for key in AUTHORITY_KEYS:
        if f'"{key}"' in encoded:
            errors.append(f"入力側のauthority keyが除去されていない: {key}")
    if stripped["payload"]["nested"].get("value") != 1:
        errors.append("authority除去が安全なnested payloadまで除去した")
    return errors


def test_adapter_loader_strips_authority_metadata_from_effective_payload() -> list[str]:
    adapter = {
        "adapter_id": "bad_adapter",
        "runtime_id": "blue_tanuki",
        "contract_version": "1.0.0",
        "authority_strip": True,
        "declared_capabilities": ["filesystem.read"],
        "metadata": {
            "authority": "admin",
            "permission_grant": "all",
            "approval_state": "approved",
            "trust_level": "root",
            "safe_label": "reference",
        },
    }
    record = load_adapter(adapter)
    encoded = json.dumps(record.metadata, sort_keys=True)
    errors = []
    for forbidden in ["admin", "all", "approved", "root"]:
        if forbidden in encoded:
            errors.append(f"authority valueがadapter metadata除去後も残った: {forbidden}")
    for key in AUTHORITY_KEYS:
        if f'"{key}"' in encoded:
            errors.append(f"authority keyがadapter metadata除去後も残った: {key}")
    if record.metadata != {"safe_label": "reference"}:
        errors.append("adapter metadata除去が安全なmetadataを除去したか、authority metadataを残した")
    if record.effective_capabilities() != ("filesystem.read",):
        errors.append("adapter metadataがeffective capabilitiesを変更した")
    return errors


def test_adapter_loader_rejects_value_only_authority_metadata() -> list[str]:
    adapter = load_contract_fixture("adapter.valid.json")
    adapter["metadata"] = {"safe_label": "root"}
    try:
        load_adapter(adapter)
    except ValueError:
        return []
    return ["adapter loaderがvalue-only authority metadataを受け入れた"]


def test_runtime_state_adapter_registration_uses_loader_boundary() -> list[str]:
    state = RuntimeState()
    adapter = load_contract_fixture("adapter.valid.json")
    adapter["metadata"] = {"permissionGrant": "all", "safe_label": "reference"}
    state.register_adapter(adapter)
    stored = state.adapters.get(adapter["adapter_id"], {})
    errors = []
    if "permission_grant" in json.dumps(stored.get("metadata", {}), sort_keys=True):
        errors.append("RuntimeState.register_adapterが除去対象のauthority metadataを保存した")
    if stored.get("metadata") != {"safe_label": "reference"}:
        errors.append("RuntimeState.register_adapterが除去処理後の安全なmetadataを保持しなかった")
    return errors


def test_normalization_firewall_rejects_authority_aliases() -> list[str]:
    payload = {
        "Trust_Level": "root",
        "ｔｒｕｓｔ＿ｌｅｖｅｌ": "admin",
        "trust\u200b_level": "elevated",
        "permissionGrant": "all",
        "admin_context": "owner",
        "frame": {"metadata": {"authority": "admin"}},
        "safe_label": "reference",
    }
    normalized = normalize_inbound_payload(payload)
    errors = []
    for key in ["Trust_Level", "ｔｒｕｓｔ＿ｌｅｖｅｌ", "trust\u200b_level"]:
        if normalize_key(key) != "trust_level":
            errors.append(f"normalizationがkeyを正規化しなかった: {key}")
    if normalized["quarantined"] is not True:
        errors.append("normalization firewallがauthorityを含むpayloadを隔離しなかった")
    if not normalized["audit_event"].get("raw_payload_preserved"):
        errors.append("normalization firewallがaudit用のraw payloadを保持しなかった")
    stripped = json.dumps(normalized["stripped_payload"], sort_keys=True)
    for forbidden in ["trust_level", "permission_grant", "admin_context", "authority"]:
        if forbidden in stripped:
            errors.append(f"authority keyがnormalization除去後も残った: {forbidden}")
    return errors


def test_normalization_firewall_detects_value_only_escalation() -> list[str]:
    payload = {"metadata": {"label": "root", "safe_note": "operator visible"}}
    normalized = normalize_inbound_payload(payload)
    errors = []
    if normalized["quarantined"] is not True:
        errors.append("normalization firewallがvalue-only authority試行を隔離しなかった")
    if not normalized["authority_value_findings"]:
        errors.append("normalization firewallがauthority valueの検出を記録しなかった")
    if normalized["stripped_payload"].get("metadata", {}).get("safe_note") != "operator visible":
        errors.append("normalization firewallがvalue-only escalation検出時に安全なmetadataまで除去した")
    return errors


def test_normalization_firewall_detects_key_collisions() -> list[str]:
    normalized = normalize_inbound_payload({"safeLabel": "first", "safe_label": "second"})
    errors = []
    if normalized["quarantined"] is not True:
        errors.append("normalization firewallがnormalized keyの衝突を隔離しなかった")
    if not normalized.get("normalization_collision_findings"):
        errors.append("normalization firewallがnormalized keyの衝突を記録しなかった")
    if normalized["audit_event"].get("normalization_collision_count") != 1:
        errors.append("normalization firewallがnormalized keyの衝突を計数しなかった")
    return errors


def test_external_metadata_cannot_escalate_authority() -> list[str]:
    adapter = load_contract_fixture("adapter.valid.json")
    adapter["metadata"] = {
        "permission_grant": "all",
        "permission_override": "fs:write",
        "trust_level": "root",
    }
    effective = metadata_permissions(adapter)
    if effective != adapter["declared_capabilities"]:
        return ["adapter metadataがeffective permissionsを昇格させた"]
    return []


def test_gui_input_cannot_create_runtime_disallowed_authority_context() -> list[str]:
    errors = []
    if can_create_authority_context("gui", runtime_allowed=True):
        errors.append("GUI inputがauthority contextを作成した")
    if can_create_authority_context("adapter", runtime_allowed=True):
        errors.append("adapter inputがauthority contextを直接作成した")
    if can_create_authority_context("runtime", runtime_allowed=False):
        errors.append("runtimeで禁止されたauthority contextが作成された")
    if not can_create_authority_context("runtime", runtime_allowed=True):
        errors.append("runtimeで許可されたauthority contextが拒否された")
    return errors


def test_memory_cache_previous_state_cannot_grant_authority() -> list[str]:
    errors = []
    for source in sorted(NON_AUTHORITY_SOURCES):
        if source_can_grant_authority(source):
            errors.append(f"{source} がauthorityを付与した")
    if source_can_grant_authority("unknown_future_source"):
        errors.append("未知のsourceがauthorityを付与した")
    return errors


def test_content_exposure_contract() -> list[str]:
    schema = load_schema("content_exposure.schema.json")
    fixture = load_contract_fixture("content_exposure.valid.json")
    errors = []
    default_visibility = schema["properties"]["default_visibility"]
    if default_visibility.get("const") != "none":
        errors.append("content exposureのdefault_visibilityはconst noneでなければならない")
    if fixture.get("default_visibility") != "none":
        errors.append("content exposureのvalid fixtureではdefault_visibilityがnoneでなければならない")
    enum = schema["properties"]["allowed_visibility"]["items"]["enum"]
    if enum != VISIBILITY_VALUES:
        errors.append("content exposureのallowed_visibility enumは固定順序と一致しなければならない")
    return errors


def test_full_content_only_visible_when_full() -> list[str]:
    base = load_contract_fixture("approval.valid.json")
    errors = []
    for visibility in VISIBILITY_VALUES:
        rendered = render_approval_content({**base, "content_visibility": visibility})
        if visibility != "full" and "full_payload" in rendered:
            errors.append(f"content_visibility={visibility}でfull payloadが描画された")
        if visibility == "hash_only" and set(rendered) != {"payload_hash"}:
            errors.append("hash_onlyがpayload_hashを超える内容を描画した")
        if visibility == "none" and rendered:
            errors.append("none visibilityでcontentが描画された")
    return errors


def test_approval_schema_has_protected_field_sets() -> list[str]:
    schema = load_schema("approval.schema.json")
    properties = schema.get("properties", {})
    errors = []
    for field in ["authority_fields", "sealed_fields", "hidden_fields", "sacred_fields"]:
        if field not in properties:
            errors.append(f"approval.schema.jsonに{field}がない")
    return errors


def test_protected_approval_fields_cannot_be_edited() -> list[str]:
    approval = load_contract_fixture("approval.valid.json")
    approval["editable_fields"] = [
        "path",
        "authority_context",
        "runtime_id",
        "credential",
        "permission_id",
        "payload_hash",
    ]
    queue = ApprovalQueue()
    queue.enqueue(approval)
    errors = []
    for field in ["authority_context", "runtime_id", "credential", "permission_id", "payload_hash"]:
        if queue.can_edit(approval["approval_id"], field):
            errors.append(f"保護されたapproval fieldが編集可能だった: {field}")
        before = queue.get(approval["approval_id"])
        try:
            queue.edit(approval["approval_id"], field, "mutated")
        except ValueError:
            pass
        else:
            errors.append(f"保護されたapproval fieldが書き込まれた: {field}")
        after = queue.get(approval["approval_id"])
        if after != before:
            errors.append(f"保護されたapprovalの変更がqueued approvalを変更した: {field}")
    if not queue.can_edit(approval["approval_id"], "path"):
        errors.append("許可された非保護のapproval fieldが編集できなかった")
    return errors


def test_approval_edits_are_rehashed_and_revalidated() -> list[str]:
    approval = {
        "status": "pending",
        "payload_hash": canonical_hash({"allowed_note": "before"}),
        "editable_fields": ["allowed_note"],
        "authority_fields": [],
        "sealed_fields": [],
        "hidden_fields": [],
        "sacred_fields": [],
        "full_payload": {"allowed_note": "before"},
    }
    queue = ApprovalQueue()
    queue.enqueue({"approval_id": "approval-edit-1", **approval})
    edited = queue.edit("approval-edit-1", "allowed_note", "after")
    errors = []
    if edited["payload_hash"] == approval["payload_hash"]:
        errors.append("approval編集がpayload_hashを変更しなかった")
    if edited["payload_hash"] != canonical_hash({"allowed_note": "after"}):
        errors.append("approval編集後のpayload_hashがcanonicalではなかった")
    if edited["status"] != "requires_validation":
        errors.append("approval編集が再検証を必須にしなかった")
    return errors


def test_sensitive_actions_map_to_audit_and_recovery() -> list[str]:
    capability = load_contract_fixture("capability.valid.json")
    permission = load_contract_fixture("permission.valid.json")
    audit_event = load_contract_fixture("audit.valid.json")
    recovery_action = load_contract_fixture("recovery.valid.json")
    complete = {
        "capability_id": capability["capability_id"],
        "permission_id": permission["permission_id"],
        "approval_state": "approved",
        "audit_event": audit_event,
        "recovery_action": recovery_action,
    }
    incomplete = {
        "capability_id": "filesystem.write",
        "permission_id": "permission.fs.write.workspace",
        "approval_state": "approved",
        "audit_event": {"event_id": "audit-1"},
    }
    errors = []
    if not sensitive_action_mapping_is_complete(complete):
        errors.append("完全なsensitive action mappingが拒否された")
    if sensitive_action_mapping_is_complete(incomplete):
        errors.append("sensitive action mappingがRecoveryActionなしで通過した")
    return errors


def test_hash_patterns_are_tagged_sha256() -> list[str]:
    errors = []
    sample = sha256_tagged(b"approval")
    if not sample.startswith("sha256:") or len(sample) != 71:
        errors.append("sha256_tagged helperのinvariantが失敗した")
    for schema_name in ["approval.schema.json", "audit.schema.json"]:
        schema = load_schema(schema_name)
        pattern = schema["properties"]["payload_hash"].get("pattern", "")
        if "sha256:" not in pattern:
            errors.append(f"{schema_name} のpayload_hashはtagged sha256 patternを使わなければならない")
    return errors


def test_framework_risk_profile_exists() -> list[str]:
    path = SPECS / "framework_risk_profile.schema.json"
    if not path.exists():
        return ["framework_risk_profile.schema.jsonが存在しない"]
    return []


def test_update_fixture_requires_signature() -> list[str]:
    update = load_contract_fixture("update.valid.json")
    if update.get("signature_required") is not True:
        return ["updateのvalid fixtureがsignatureを必須にしていない"]
    return []


def test_update_policy_unsigned_rejection_uses_taxonomy() -> list[str]:
    store = UpdatePolicyStore()
    try:
        store.register({"policy_id": "unsigned-policy", "signature_required": False})
    except ValueError as exc:
        if "update_signature_required" not in str(exc):
            return ["UpdatePolicyStoreのunsigned拒否がupdate_signature_required taxonomyを使っていない"]
        return []
    return ["UpdatePolicyStoreがunsigned update policyを受け入れた"]


def test_update_center_contract_and_execution_boundary() -> list[str]:
    errors = []
    candidate = load_contract_fixture("update_candidate.valid.json")
    catalog = load_contract_fixture("update_catalog.valid.json")
    receipt = load_contract_fixture("update_receipt.valid.json")
    listing = load_contract_fixture("update_list.valid.json")
    trust = load_contract_fixture("update_trust.valid.json")
    download_job = load_contract_fixture("update_download_job.valid.json")
    for name, value in (("update_candidate", candidate), ("update_catalog", catalog), ("update_receipt", receipt), ("update_list", listing), ("update_trust", trust)):
        errors.extend(validate_instance(value, load_schema(f"{name}.schema.json")))
    if validate_instance(
        load_contract_fixture("invalid/update_catalog.invalid.json"),
        load_schema("update_catalog.schema.json"),
    ) == []:
        errors.append("署名更新候補一覧が未知の配布pathを受け入れた")
    update_list_schema = load_schema("update_list.schema.json")
    missing_download_job = copy.deepcopy(listing)
    missing_download_job.pop("download_job", None)
    if validate_instance(missing_download_job, update_list_schema) == []:
        errors.append("更新一覧がdownload job状態の欠落を受理した")
    errors.extend(validate_instance(download_job, load_schema("update_download_job.schema.json")))
    download_job_schema = load_schema("update_download_job.schema.json")
    if validate_instance(
        load_contract_fixture("invalid/update_download_job_path_leak.invalid.json"),
        download_job_schema,
    ) == []:
        errors.append("UpdateDownloadJobがpackage pathの露出を受け入れた")
    for invalid_job in (
        {**download_job, "状態": "trusted"},
        {**download_job, "受信byte数": 4 * 1024 * 1024 * 1024 + 1},
        {**download_job, "local_path": "C:/private/update.pkg"},
        {**download_job, "失敗code": "network failure at https://secret.example/token"},
        {**download_job, "状態": "downloaded", "失敗code": "update_download_network_failed"},
    ):
        if validate_instance(invalid_job, load_schema("update_download_job.schema.json")) == []:
            errors.append("UpdateDownloadJobが未知状態・容量超過・path／詳細漏えい・矛盾状態を受理した")
    trust_schema = load_schema("update_trust.schema.json")
    configured_trust = {
        **trust,
        "public_key_der_hex": "00" * 44,
        "public_key_fingerprint": "sha256:" + "a" * 64,
    }
    source_configured_trust = {
        **configured_trust,
        "package_sources": [{
            "channel": "stable",
            "base_url": "https://updates.example.invalid/d4/stable",
        }],
    }
    if validate_instance(source_configured_trust, trust_schema):
        errors.append("Broker所有の版2 update trustが有効な配布元設定を拒否した")
    duplicate_channel_trust = {
        **configured_trust,
        "package_sources": [
            {"channel": "stable", "base_url": "https://updates.example.invalid/d4/stable"},
            {"channel": "stable", "base_url": "https://mirror.example.invalid/d4/stable"},
        ],
    }
    if validate_instance(duplicate_channel_trust, trust_schema) == []:
        errors.append("update trust schemaが同じchannelの複数配布元を受理した")
    legacy_trust = {key: value for key, value in trust.items() if key != "package_sources"}
    legacy_trust["版"] = 1
    if validate_instance(legacy_trust, trust_schema):
        errors.append("旧版1 update trustの互換読取り契約が失われた")
    if validate_instance({**legacy_trust, "package_sources": []}, trust_schema) == []:
        errors.append("旧版1 update trustが版2専用配布元fieldを受理した")
    for base_url in (
        "http://updates.example.invalid/d4/stable",
        "https://user@updates.example.invalid/d4/stable",
        "https://updates.example.invalid:443/d4/stable",
        "https://updates.example.invalid/d4/stable?channel=beta",
        "https://updates.example.invalid/d4/stable#fragment",
        "https://Updates.example.invalid/d4/stable",
    ):
        invalid_trust = {
            **configured_trust,
            "package_sources": [{"channel": "stable", "base_url": base_url}],
        }
        if validate_instance(invalid_trust, trust_schema) == []:
            errors.append("update trust schemaが不正な配布元URLを受理した")
    unsupported_channel_trust = {
        **configured_trust,
        "package_sources": [{
            "channel": "preview",
            "base_url": "https://updates.example.invalid/d4/preview",
        }],
    }
    if validate_instance(unsupported_channel_trust, trust_schema) == []:
        errors.append("update trust schemaが未対応channelを受理した")
    if validate_instance({**trust, "package_sources": [{
        "channel": "stable",
        "base_url": "https://updates.example.invalid/d4/stable",
    }]}, trust_schema) == []:
        errors.append("未構成の署名鍵に配布元を結合したupdate trustを受理した")
    if validate_instance({**candidate, "public_key_der_hex": "00"}, load_schema("update_candidate.schema.json")) == []:
        errors.append("更新候補へBroker外部公開鍵を混入できた")
    if validate_instance({**candidate, "package_path": "C:/untrusted/package.zip"}, load_schema("update_candidate.schema.json")) == []:
        errors.append("更新候補が外部package pathを受け入れた")
    if validate_instance({**candidate, "package_url": "https://attacker.example/update-1.pkg"}, load_schema("update_candidate.schema.json")) == []:
        errors.append("更新候補が外部package URLを受け入れた")
    projected_item = listing["更新一覧"][0]
    missing_source = copy.deepcopy(listing)
    del missing_source["更新一覧"][0]["取得元"]
    if validate_instance(missing_source, load_schema("update_list.schema.json")) == []:
        errors.append("更新一覧がBroker導出の取得元状態を欠いたreceiptを受理した")
    mismatched_source = copy.deepcopy(listing)
    mismatched_source["更新一覧"][0]["取得元"] = {"状態": "configured", "URL": None}
    if validate_instance(mismatched_source, load_schema("update_list.schema.json")) == []:
        errors.append("配布元設定済み状態とURLの不一致を受理した")
    for unsafe_url in (
        "https://updates.example.invalid/d4/../update-1.pkg",
        "https://127.0.0.1/d4/stable/update-1.pkg",
        "https://Updates.example.invalid/d4/stable/update-1.pkg",
    ):
        unsafe_source = copy.deepcopy(listing)
        unsafe_source["更新一覧"][0]["取得元"]["URL"] = unsafe_url
        if validate_instance(unsafe_source, load_schema("update_list.schema.json")) == []:
            errors.append("Broker導出取得先Schemaが非正規URLを受理した")
    if projected_item["取得元"]["URL"] != "https://updates.example.invalid/d4/stable/update-1.pkg":
        errors.append("更新一覧fixtureの取得先が更新ID付きBroker導出URLと一致しない")
    if candidate.get("版") != 2 or not candidate.get("package_sha256") or candidate.get("package_size_bytes", 0) <= 0:
        errors.append("更新候補が署名対象packageのhash・byte長を持たない")
    if validate_instance({**candidate, "package_sha256": "g" * 64}, load_schema("update_candidate.schema.json")) == []:
        errors.append("更新候補が不正なpackage hashを受理した")
    if validate_instance({**candidate, "package_size_bytes": 0}, load_schema("update_candidate.schema.json")) == []:
        errors.append("更新候補が空packageを受理した")
    legacy_receipt = {key: value for key, value in receipt.items() if key not in {"package_sha256", "package_size_bytes"}}
    legacy_receipt["版"] = 1
    legacy_receipt["署名状態"] = "legacy_unbound"
    errors.extend(validate_instance(legacy_receipt, load_schema("update_receipt.schema.json")))
    legacy_verified_receipt = dict(legacy_receipt)
    legacy_verified_receipt["署名状態"] = "verified"
    if validate_instance(legacy_verified_receipt, load_schema("update_receipt.schema.json")) == []:
        errors.append("package未結合の旧更新receiptをverifiedとして表示できた")
    legacy_stale_receipt = dict(legacy_receipt)
    legacy_stale_receipt["署名状態"] = "verification_stale"
    if validate_instance(legacy_stale_receipt, load_schema("update_receipt.schema.json")) == []:
        errors.append("package未結合の旧更新receiptが現行trust再検証状態を主張できた")
    current_stale_receipt = dict(receipt)
    current_stale_receipt["署名状態"] = "verification_stale"
    errors.extend(validate_instance(current_stale_receipt, load_schema("update_receipt.schema.json")))
    rollback_available = listing["rollback状態"]["状態"] == "available"
    available_without_descriptors = copy.deepcopy(listing)
    available_without_descriptors["rollback実行"] = "available"
    available_without_descriptors["rollback状態"]["状態"] = "available"
    if validate_instance(available_without_descriptors, update_list_schema) == []:
        errors.append("更新一覧が対象版のないrollback available状態を受理した")
    unavailable_with_descriptor = copy.deepcopy(listing)
    unavailable_with_descriptor["rollback状態"]["現在版"] = {
        "更新ID": "update-1",
        "候補hash": "sha256:" + "a" * 64,
        "提供版": "1.1.0",
        "package_sha256": "b" * 64,
    }
    if validate_instance(unavailable_with_descriptor, update_list_schema) == []:
        errors.append("更新一覧がrollback不可状態で版descriptorを露出した")
    if (
        listing["download実行"] not in ("available", "suspended")
        or listing["適用実行"] != "suspended"
        or listing["rollback実行"] != ("available" if rollback_available else "suspended")
        or (rollback_available and (listing["rollback状態"]["現在版"] is None or listing["rollback状態"]["対象版"] is None))
        or (not rollback_available and (listing["rollback状態"]["現在版"] is not None or listing["rollback状態"]["対象版"] is not None))
    ):
        errors.append("更新実行状態とBroker由来rollback先projectionが一致しない")
    for name in ("ipc_request", "ipc_response"):
        operations = load_schema(f"{name}.schema.json")["properties"]["operation"]["enum"]
        for operation in ("更新一覧", "更新候補取得", "更新確認", "更新署名検査", "更新download要求", "更新適用要求", "更新延期", "更新rollback要求", "製品起動項目修復要求", "製品アンインストール要求"):
            if operation not in operations:
                errors.append(f"{name}に更新操作がない: {operation}")
    update_center = (RUST_HELPER / "src" / "broker" / "update_center.rs").read_text(
        encoding="utf-8"
    )
    for token in (
        'const OP_FETCH_CATALOG: &str = "更新候補取得"',
        "fn fetch_catalog_with<",
        "fn catalog_audit_reason(",
        "Capability=更新候補取得",
        "Permission=Broker所有trust.package_sources",
        "Approval=明示操作によるread-only公開metadata取得はnot_required",
        "AuditEvent=配布元件数・catalog hash・候補件数だけ",
        "RecoveryAction=失敗時は既存候補を保持",
    ):
        if token not in update_center:
            errors.append(f"更新候補取得のBroker／Authority／Recovery contractがない: {token}")
    return errors


def test_notification_center_contract_and_navigation_boundary() -> list[str]:
    errors = []
    notification = load_contract_fixture("notification.valid.json")
    listing = load_contract_fixture("notification_list.valid.json")
    action = load_contract_fixture("notification_action.valid.json")
    for name, value in (
        ("notification", notification),
        ("notification_list", listing),
        ("notification_action", action),
    ):
        errors.extend(validate_instance(value, load_schema(f"{name}.schema.json")))
    if listing["操作"] != "navigation_only" or listing["権限生成"] != "なし":
        errors.append("通知一覧がnavigation-only境界を宣言していない")
    if notification["表示範囲"] != "summary":
        errors.append("通知summaryがsummary表示範囲ではない")
    if "payload" in notification or "reason" in notification:
        errors.append("通知summaryへ監査reasonまたはraw payloadが露出している")
    for name in ("ipc_request", "ipc_response"):
        operations = load_schema(f"{name}.schema.json")["properties"]["operation"]["enum"]
        for operation in ("通知一覧", "通知既読", "通知破棄", "通知全既読"):
            if operation not in operations:
                errors.append(f"{name}に通知操作がない: {operation}")
    source = (RUST_HELPER / "src" / "broker" / "notification_center.rs").read_text(encoding="utf-8")
    if 'const OP_REGISTER: &str = "通知登録"' in source or '"通知登録"' in source:
        errors.append("通知登録というcaller由来の生成経路を追加してはならない")
    if '"操作": "navigation_only"' not in source:
        errors.append("通知Broker応答がnavigation_onlyを明示していない")
    return errors


def test_observation_center_contract_and_audit_separation() -> list[str]:
    errors = []
    span = load_contract_fixture("observation_span.valid.json")
    trace = load_contract_fixture("observation_trace.valid.json")
    metric = load_contract_fixture("observation_metric.valid.json")
    listing = load_contract_fixture("observation_list.valid.json")
    for name, value in (
        ("observation_span", span),
        ("observation_trace", trace),
        ("observation_metric", metric),
        ("observation_list", listing),
    ):
        errors.extend(validate_instance(value, load_schema(f"{name}.schema.json")))
    if listing["証拠種別"] != "INTERNAL_STATE":
        errors.append("観測一覧が内部状態の証拠範囲を宣言していない")
    if listing["権限生成"] != "なし":
        errors.append("観測一覧が権限を生成しないことを宣言していない")
    if listing["OpenTelemetry export"] != "unsupported":
        errors.append("観測一覧が未対応のOpenTelemetry exportを実行可能と宣言した")
    for name in ("ipc_request", "ipc_response"):
        operations = load_schema(f"{name}.schema.json")["properties"]["operation"]["enum"]
        if "観測一覧" not in operations:
            errors.append(f"{name}に観測一覧操作がない")
    source = (RUST_HELPER / "src" / "broker" / "observation_center.rs").read_text(encoding="utf-8")
    if '"reason"' in source or '"payload_hash"' in source:
        errors.append("観測応答へ監査内容の生フィールドを投影している")
    if '"証拠種別": EVIDENCE_SOURCE_INTERNAL_STATE' not in source:
        errors.append("観測応答の証拠種別が内部状態に固定されていない")
    if '"権限生成": "なし"' not in source:
        errors.append("観測応答が権限非生成を明示していない")
    if '"OpenTelemetry export": "unsupported"' not in source:
        errors.append("OpenTelemetry exportの未対応境界がない")
    if '"観測登録"' in source:
        errors.append("caller由来の観測登録経路を追加してはならない")
    if "MAX_SPANS: usize = 1024" not in source or "MAX_RESPONSE_SPANS: usize = 256" not in source:
        errors.append("観測保持または応答のbounded上限が宣言されていない")
    return errors


def test_trace_inspector_surface_and_evidence_boundary() -> list[str]:
    errors = []
    source = (DESKTOP_FLUTTER / "lib" / "screens" / "trace_inspector.dart").read_text(
        encoding="utf-8"
    )
    for term in (
        "処理時間 waterfall",
        "開始EpochMillis",
        "終了EpochMillis",
        "所要Millis",
        "親SpanID",
        "エラー分類",
        "現在の実測対象: Broker",
        "Runtime、Adapter、Tool、外部通信",
        "snapshotをTraceの根拠にはしません",
        "Permission、Approval、Authority、Capability、Credential",
    ):
        if term not in source:
            errors.append(f"Trace Inspectorに必要な境界または表示項目がない: {term}")
    return errors


def test_shell_contracts_load_required_schemas() -> list[str]:
    catalog = load_default_catalog()
    expected = {f"{name}.schema.json" for name in REQUIRED_SCHEMA_NAMES}
    loaded = set(catalog.names())
    missing = sorted(expected - loaded)
    if missing:
        return [f"shell_contracts catalogにschemaがない: {name}" for name in missing]
    return []


def test_shell_core_ignores_adapter_metadata_permissions() -> list[str]:
    adapter = load_contract_fixture("adapter.valid.json")
    adapter["metadata"] = {
        "permissions": ["filesystem.write"],
        "grants": ["all"],
        "trust_level": "root",
    }
    try:
        record = load_adapter(adapter)
    except ValueError:
        return []
    if record.effective_capabilities() != tuple(adapter["declared_capabilities"]):
        return ["Shell Coreがpermissionのadapter metadataを信頼した"]
    return []


def test_shell_core_non_authority_sources_do_not_grant_authority() -> list[str]:
    ledger = PermissionLedger()
    errors = []
    for source in sorted(NON_AUTHORITY_SOURCES):
        if ledger.can_grant_authority_from_source(source):
            errors.append(f"Shell Coreが{source}をauthorityとして扱った")
    if ledger.can_grant_authority_from_source("unknown_future_source"):
        errors.append("Shell Coreが未知のsourceをauthorityとして扱った")
    return errors


def test_shell_core_routes_sensitive_actions_through_required_mapping() -> list[str]:
    router = SensitiveActionRouter()
    capability = load_contract_fixture("capability.valid.json")
    permission = load_contract_fixture("permission.valid.json")
    audit_event = load_contract_fixture("audit.valid.json")
    recovery_action = load_contract_fixture("recovery.valid.json")
    routed = router.route(
        {
            "runtime_id": "blue_tanuki",
            "operation": capability["capability_id"],
            "capability_id": capability["capability_id"],
            "permission_id": permission["permission_id"],
            "approval_id": load_contract_fixture("approval.valid.json")["approval_id"],
            "target_scope": permission["target_scope"],
            "audit_event": audit_event,
            "recovery_action": recovery_action,
        }
    )
    errors = []
    if routed.get("routed") is not False:
        errors.append("Shell Coreがevaluator stateなしでsensitive actionをroutedした")
    if "schema_contract_missing" not in error_codes(routed.get("policy_result", {})):
        errors.append("Shell Coreがevaluator result欠落時のfail-closedを公開しなかった")
    try:
        router.route(
            {
                "runtime_id": "blue_tanuki",
                "operation": capability["capability_id"],
                "capability_id": capability["capability_id"],
                "permission_id": permission["permission_id"],
                "approval_id": load_contract_fixture("approval.valid.json")["approval_id"],
                "target_scope": permission["target_scope"],
                "audit_event": audit_event,
            }
        )
    except ValueError:
        return errors
    errors.append("Shell CoreがRecoveryActionなしでsensitive actionをroutedした")
    return errors


def test_shell_core_content_projection_hides_full_payload_until_full() -> list[str]:
    approval = load_contract_fixture("approval.valid.json")
    errors = []
    for visibility in VISIBILITY_VALUES:
        projected = project_approval_content({**approval, "content_visibility": visibility})
        if visibility != "full" and "full_payload" in projected:
            errors.append(f"Shell Coreが{visibility}でfull_payloadを射影した")
    return errors


def test_content_projection_missing_visibility_fails_closed() -> list[str]:
    approval = load_contract_fixture("approval.valid.json")
    approval.pop("content_visibility")
    projected = project_approval_content(approval)
    error = projected.get("error", {})
    if error.get("code") != "content_visibility_violation":
        return ["content_visibility欠落が構造化されたfail-closedのprojection errorを返さなかった"]
    if "full_payload" in projected:
        return ["content_visibility欠落時にfull_payloadが公開された"]
    return []


def test_shell_core_has_no_flutter_imports() -> list[str]:
    errors = []
    for path in sorted(SHELL_CORE.rglob("*.py")):
        if "__pycache__" in path.parts:
            continue
        text = path.read_text(encoding="utf-8")
        for line_number, line in enumerate(text.splitlines(), start=1):
            normalized = line.strip().lower()
            if normalized.startswith("import flutter") or normalized.startswith("from flutter"):
                errors.append(f"{path}:{line_number} がFlutterをimportしている")
    return errors


def test_shell_core_has_no_blue_tanuki_internal_imports() -> list[str]:
    errors = []
    for path in sorted(SHELL_CORE.rglob("*.py")):
        if "__pycache__" in path.parts:
            continue
        text = path.read_text(encoding="utf-8")
        for line_number, line in enumerate(text.splitlines(), start=1):
            normalized = line.strip().lower()
            if normalized.startswith("import blue_tanuki") or normalized.startswith("from blue_tanuki"):
                errors.append(f"{path}:{line_number} がBLUE-TANUKI内部をimportしている")
    return errors


def build_policy_state(*, permission_decision: str = "allow", approval_status: str = "approved") -> RuntimeState:
    state = RuntimeState()
    state.register_runtime(load_contract_fixture("runtime.valid.json"))
    state.register_adapter(load_contract_fixture("adapter.valid.json"))
    state.register_capability(load_contract_fixture("capability.valid.json"))
    permission = load_contract_fixture("permission.valid.json")
    permission["decision"] = permission_decision
    state.record_permission(permission)
    approval = load_contract_fixture("approval.valid.json")
    approval["status"] = approval_status
    state.enqueue_approval(approval)
    state.append_audit_event(load_contract_fixture("audit.valid.json"))
    state.register_recovery_action(load_contract_fixture("recovery.valid.json"))
    state.register_update_policy(load_contract_fixture("update.valid.json"))
    return state


def build_sensitive_action() -> dict:
    capability = load_contract_fixture("capability.valid.json")
    permission = load_contract_fixture("permission.valid.json")
    approval = load_contract_fixture("approval.valid.json")
    return {
        "runtime_id": "blue_tanuki",
        "operation": capability["capability_id"],
        "capability_id": capability["capability_id"],
        "permission_id": permission["permission_id"],
        "approval_id": approval["approval_id"],
        "approval_state": "approved",
        "target_scope": permission.get("target_scope", "workspace"),
        "payload": approval["full_payload"],
        "audit_event": load_contract_fixture("audit.valid.json"),
        "recovery_action": load_contract_fixture("recovery.valid.json"),
    }


def error_codes(result: dict) -> set[str]:
    return {error["code"] for error in result["errors"]}


def test_policy_evaluator_rejects_unknown_capability() -> list[str]:
    state = build_policy_state()
    action = build_sensitive_action()
    action["capability_id"] = "unknown.capability"
    result = PolicyEvaluator(state).evaluate(action)
    if result["allowed"] or "unknown_capability" not in error_codes(result):
        return ["PolicyEvaluatorが未知のcapabilityを拒否しなかった"]
    return []


def test_policy_evaluator_returns_structured_errors() -> list[str]:
    state = build_policy_state()
    action = build_sensitive_action()
    action["capability_id"] = "unknown.capability"
    result = PolicyEvaluator(state).evaluate(action)
    errors = []
    required = {"code", "message", "operation", "recoverable"}
    if not result["errors"]:
        return ["PolicyEvaluatorが無効なactionでerrorを返さなかった"]
    for error in result["errors"]:
        missing = sorted(required - set(error))
        if missing:
            errors.append(f"PolicyEvaluatorのerrorにfieldがない: {', '.join(missing)}")
        if error.get("code") == "unknown_capability" and "recovery_hint" not in error:
            errors.append("PolicyEvaluatorのrecoverable errorにrecovery_hintがない")
    return errors


def test_policy_evaluator_rejects_unknown_permission() -> list[str]:
    state = build_policy_state()
    action = build_sensitive_action()
    action["permission_id"] = "unknown.permission"
    result = PolicyEvaluator(state).evaluate(action)
    if result["allowed"] or "unknown_permission" not in error_codes(result):
        return ["PolicyEvaluatorが未知のpermissionを拒否しなかった"]
    return []


def test_policy_evaluator_rejects_denied_permission() -> list[str]:
    state = build_policy_state(permission_decision="deny")
    result = PolicyEvaluator(state).evaluate(build_sensitive_action())
    if result["allowed"] or "permission_denied" not in error_codes(result):
        return ["PolicyEvaluatorが拒否されたpermissionを拒否しなかった"]
    return []


def test_policy_evaluator_rejects_missing_approval() -> list[str]:
    state = build_policy_state()
    action = build_sensitive_action()
    action.pop("approval_state")
    action.pop("approval_id")
    result = PolicyEvaluator(state).evaluate(action)
    if result["allowed"] or "approval_missing" not in error_codes(result):
        return ["PolicyEvaluatorが欠落したapprovalを拒否しなかった"]
    return []


def test_policy_evaluator_rejects_self_reported_approval_without_approval_id() -> list[str]:
    state = build_policy_state()
    action = build_sensitive_action()
    action.pop("approval_id")
    action["approval_state"] = "approved"
    result = PolicyEvaluator(state).evaluate(action)
    if result["allowed"] or "approval_missing" not in error_codes(result):
        return ["PolicyEvaluatorがapproval_idなしの自己申告approval_stateを信頼した"]
    return []


def test_policy_evaluator_rejects_unknown_approval_id() -> list[str]:
    state = build_policy_state()
    action = build_sensitive_action()
    action["approval_id"] = "approval-does-not-exist"
    action["approval_state"] = "approved"
    result = PolicyEvaluator(state).evaluate(action)
    if result["allowed"] or "approval_missing" not in error_codes(result):
        return ["PolicyEvaluatorが未知のapproval_idを受け入れた"]
    return []


def test_policy_evaluator_uses_runtime_state_approval_status() -> list[str]:
    state = build_policy_state(approval_status="approved")
    action = build_sensitive_action()
    action["approval_state"] = "pending"
    result = PolicyEvaluator(state).evaluate(action)
    if not result["allowed"]:
        return ["PolicyEvaluatorがactionの自己申告差だけを理由に、RuntimeStateでapprovedのapprovalを拒否した"]
    return []


def test_policy_evaluator_rejects_unapproved_runtime_state_approval() -> list[str]:
    state = build_policy_state(approval_status="requires_validation")
    action = build_sensitive_action()
    action["approval_state"] = "approved"
    result = PolicyEvaluator(state).evaluate(action)
    if result["allowed"] or "approval_not_valid" not in error_codes(result):
        return ["PolicyEvaluatorがRuntimeStateのapproval statusよりactionのapproval_stateを信頼した"]
    return []


def test_policy_evaluator_rejects_missing_audit_event() -> list[str]:
    state = build_policy_state()
    action = build_sensitive_action()
    action.pop("audit_event")
    result = PolicyEvaluator(state).evaluate(action)
    if result["allowed"] or "audit_mapping_missing" not in error_codes(result):
        return ["PolicyEvaluatorが欠落したaudit eventを拒否しなかった"]
    return []


def test_policy_evaluator_rejects_missing_recovery_action() -> list[str]:
    state = build_policy_state()
    action = build_sensitive_action()
    action.pop("recovery_action")
    result = PolicyEvaluator(state).evaluate(action)
    if result["allowed"] or "recovery_mapping_missing" not in error_codes(result):
        return ["PolicyEvaluatorが欠落したrecovery actionを拒否しなかった"]
    return []


def test_policy_evaluator_rejects_unknown_recovery_id() -> list[str]:
    state = build_policy_state()
    action = build_sensitive_action()
    action["recovery_action"] = {"recovery_id": "recover-does-not-exist"}
    result = PolicyEvaluator(state).evaluate(action)
    if result["allowed"] or "recovery_mapping_missing" not in error_codes(result):
        return ["PolicyEvaluatorが未知のrecovery_idを受け入れた"]
    return []


def test_policy_evaluator_accepts_known_recovery_id() -> list[str]:
    state = build_policy_state()
    result = PolicyEvaluator(state).evaluate(build_sensitive_action())
    if not result["allowed"]:
        return ["PolicyEvaluatorがその他は有効なactionで既知のrecovery_idを拒否した"]
    return []


def test_policy_evaluator_ignores_adapter_metadata_authority() -> list[str]:
    state = build_policy_state()
    action = build_sensitive_action()
    action["adapter_metadata"] = {"permissions": ["filesystem.write"], "trust_level": "root"}
    result = PolicyEvaluator(state).evaluate(action)
    if "adapter_metadata_escalation_attempt" not in error_codes(result):
        return ["PolicyEvaluatorがadapter metadataのauthority claimを検出しなかった"]
    return []


def test_policy_evaluator_normalizes_adapter_metadata_authority() -> list[str]:
    state = build_policy_state()
    cases = [
        {"Trust_Level": "root"},
        {"permissionGrant": "all"},
        {"trust\u200b_level": "root"},
        {"ｔｒｕｓｔ＿ｌｅｖｅｌ": "root"},
        {"safe_label": "root"},
    ]
    errors = []
    for metadata in cases:
        action = build_sensitive_action()
        action["adapter_metadata"] = metadata
        result = PolicyEvaluator(state).evaluate(action)
        if result["allowed"] or "adapter_metadata_escalation_attempt" not in error_codes(result):
            errors.append(f"PolicyEvaluatorがadapter metadataをnormalizeして拒否しなかった: {metadata}")
    return errors


def test_policy_evaluator_rejects_non_authority_source() -> list[str]:
    state = build_policy_state()
    errors = []
    for source in sorted(NON_AUTHORITY_SOURCES):
        action = build_sensitive_action()
        action["authority_source"] = source
        result = PolicyEvaluator(state).evaluate(action)
        if result["allowed"] or "non_authority_source_attempt" not in error_codes(result):
            errors.append(f"PolicyEvaluatorが非authorityのsourceを許可した: {source}")
    return errors


def test_policy_evaluator_enforces_action_envelope_relations() -> list[str]:
    errors = []

    def assert_error(label: str, mutate, expected_code: str) -> None:
        state = build_policy_state(approval_status="approved")
        action = build_sensitive_action()
        mutate(state, action)
        result = PolicyEvaluator(state).evaluate(action)
        if result["allowed"] or expected_code not in error_codes(result):
            errors.append(f"PolicyEvaluatorが{label}を拒否しなかった: {result}")

    assert_error(
        "runtime capability mismatch",
        lambda state, action: state.capabilities[action["capability_id"]].update({"runtime_id": "other-runtime"}),
        "relation_mismatch",
    )
    assert_error(
        "operation capability mismatch",
        lambda state, action: state.capabilities[action["capability_id"]].update({"operations": ["filesystem.read"]}),
        "relation_mismatch",
    )
    assert_error(
        "permission runtime mismatch",
        lambda state, action: state.permissions[action["permission_id"]].update({"runtime_id": "other-runtime"}),
        "relation_mismatch",
    )
    assert_error(
        "approval payload hash mismatch",
        lambda state, action: action.update({"payload": {"path": "notes/today.md", "content": "tampered"}}),
        "payload_hash_mismatch",
    )
    assert_error(
        "fabricated audit mapping",
        lambda state, action: action.update({"audit_event": {**action["audit_event"], "event_id": "fabricated-audit"}}),
        "audit_mapping_missing",
    )

    allowed = PolicyEvaluator(build_policy_state(approval_status="approved")).evaluate(build_sensitive_action())
    if allowed["allowed"] is not True:
        errors.append(f"PolicyEvaluatorが有効なActionEnvelope relationを拒否した: {allowed}")
    if not allowed.get("action_envelope"):
        errors.append("PolicyEvaluatorが検証済みaction_envelopeを返さなかった")
    return errors


def test_sensitive_action_router_uses_policy_evaluator_when_state_is_provided() -> list[str]:
    state = build_policy_state()
    routed = SensitiveActionRouter(state).route(build_sensitive_action())
    errors = []
    if routed.get("routed") is not True:
        errors.append("policy-backed SensitiveActionRouterが許可済みactionをrouteしなかった")
    if routed.get("policy_result", {}).get("allowed") is not True:
        errors.append("policy-backed SensitiveActionRouterに許可済みpolicy resultがない")
    return errors


def test_sensitive_action_router_blocks_policy_denied_action() -> list[str]:
    state = build_policy_state(permission_decision="deny")
    routed = SensitiveActionRouter(state).route(build_sensitive_action())
    errors = []
    if routed.get("routed") is not False:
        errors.append("policy-backed SensitiveActionRouterが拒否済みactionをrouteした")
    if "permission_denied" not in error_codes(routed.get("policy_result", {})):
        errors.append("policy-backed SensitiveActionRouterがpermission_deniedを公開しなかった")
    return errors


def test_state_snapshot_is_deterministic() -> list[str]:
    state = build_policy_state()
    first = deterministic_snapshot_json(state)
    second = deterministic_snapshot_json(state.clone())
    if first != second:
        return ["state snapshotがdeterministicではなかった"]
    return []


def test_state_snapshot_reports_invariant_flags() -> list[str]:
    snapshot = create_state_snapshot(build_policy_state())
    flags = snapshot.get("invariant_flags", {})
    required = {
        "flutter_imported_by_shell_core",
        "blue_tanuki_imported_by_shell_core",
        "adapter_metadata_can_escalate_authority",
        "memory_cache_previous_state_can_grant_authority",
        "full_payload_projected_without_full_visibility",
        "installer_setup_state_can_grant_authority",
        "mobile_device_state_can_grant_authority",
    }
    errors = []
    for flag in sorted(required):
        if flags.get(flag) is not False:
            errors.append(f"state snapshotのinvariant flagがないかfalseではない: {flag}")
    return errors


def test_invariant_evaluator_scans_nested_shell_core_python() -> list[str]:
    with tempfile.TemporaryDirectory(prefix="gui-shell-invariant-recursive-") as directory:
        root = Path(directory)
        nested = root / "packages" / "shell_core" / "nested"
        nested.mkdir(parents=True)
        (nested / "bad.py").write_text("import flutter\n", encoding="utf-8")
        if not InvariantEvaluator(root).shell_core_imports_forbidden("flutter"):
            return ["InvariantEvaluatorがnested Shell CoreのPython fileをscanしなかった"]
    return []


def test_shell_core_integrated_release_smoke() -> list[str]:
    with tempfile.TemporaryDirectory() as tmp:
        result = run_shell_core_release_smoke(Path(tmp))
    if not result["ok"]:
        return [f"Shell Coreのrelease smokeが失敗: {error}" for error in result["errors"]]
    errors = []
    if result["snapshot_saved"] is not True:
        errors.append("Shell Coreのrelease smokeがsnapshotを保存しなかった")
    if result["audit_chain_verified"] is not True:
        errors.append("Shell Coreのrelease smokeがaudit chainを検証しなかった")
    if result.get("audit_anchor_verified") is not True:
        errors.append("Shell Coreのrelease smokeがaudit HMAC anchorを検証しなかった")
    if result["tamper_detected"] is not True:
        errors.append("Shell Coreのrelease smokeがtamperを検出しなかった")
    if result["approval_revalidation_required"] is not True:
        errors.append("Shell Coreのrelease smokeがapprovalの再検証を必須にしなかった")
    if result["recovery_id_verified"] is not True:
        errors.append("Shell Coreのrelease smokeがrecovery mappingを検証しなかった")
    return errors


def test_json_persistence_rejects_truncated_audit_anchor() -> list[str]:
    with tempfile.TemporaryDirectory(prefix="gui-shell-audit-anchor-") as directory:
        persistence = JsonPersistence(Path(directory))
        first = persistence.append_audit_event(
            {
                "event_id": "audit-1",
                "action": "approval.requested",
                "result": "success",
                "payload_hash": canonical_hash({"approval_id": "approval-1"}),
            }
        )
        persistence.append_audit_event(
            {
                "event_id": "audit-2",
                "action": "approval.validated",
                "result": "success",
                "payload_hash": canonical_hash({"approval_id": "approval-1", "status": "approved"}),
            }
        )
        persistence.audit_path.write_text(
            json.dumps(first, sort_keys=True, separators=(",", ":")) + "\n",
            encoding="utf-8",
        )
        verification = persistence.verify_audit_chain()
        if verification["ok"] is not False:
            return ["JsonPersistenceがstale HMAC anchorを伴うtruncated audit logを受け入れた"]
        if "audit anchor HMAC" not in " ".join(verification.get("errors", [])):
            return ["JsonPersistenceのtruncate失敗がaudit anchor HMACを示さなかった"]
    return []


def test_release_smoke_runs_first_run_and_setup_doctor() -> list[str]:
    result = run_release_smokes()
    if not result["ok"]:
        return [f"release smokeが失敗: {error}" for error in result["errors"]]
    first_run = result["first_run"]
    errors = []
    if first_run["config_created"] is not True:
        errors.append("first-run smokeがconfigを作成しなかった")
    if first_run["audit_dir_writable"] is not True:
        errors.append("first-run smokeがaudit dirの書き込み可能性を検証しなかった")
    if first_run["installer_grants_authority"] is not False:
        errors.append("first-run smokeがauthorityを付与する")
    if first_run["installer_silently_approves_permissions"] is not False:
        errors.append("first-run smokeがpermissionを黙示承認する")
    return errors


def test_shell_snapshot_contains_gui_operation_state() -> list[str]:
    snapshot = build_shell_snapshot()
    errors = []
    for key in [
        "trust_records",
        "authority_map",
        "adapter_catalog",
        "permission_diffs",
        "problems",
        "evidence",
        "settings",
    ]:
        if not snapshot.get(key):
            errors.append(f"shell snapshotにGUI operation stateがない: {key}")
    if snapshot.get("installer_grants_authority") is not False:
        errors.append("shell snapshotがinstaller authorityを付与する")
    if snapshot.get("installer_silently_approves_permissions") is not False:
        errors.append("shell snapshotがinstaller permissionを黙示承認する")
    return errors


def test_shell_snapshot_generator_writes_phase_b_local_snapshot() -> list[str]:
    with tempfile.TemporaryDirectory() as tmp:
        output = Path(tmp) / ".gui_shell" / "shell_snapshot.json"
        release_evidence = ROOT / "release_evidence" / "windows_installed_smoke.json"
        existed_before = release_evidence.exists()
        result = subprocess.run(
            [sys.executable, "tooling/shell_snapshot.py", "--write", str(output)],
            cwd=ROOT,
            text=True,
            capture_output=True,
            check=False,
        )
        if result.returncode != 0:
            return [f"shell snapshot generatorが失敗: {result.stderr or result.stdout}"]
        if not output.exists():
            return ["shell snapshot generatorがoutputを書き込まなかった"]
        snapshot = json.loads(output.read_text(encoding="utf-8"))
    errors = []
    if snapshot.get("phase_status", {}).get("phase_a_status") != "complete":
        errors.append("生成されたsnapshotがPhase A完了を示していない")
    if snapshot.get("phase_status", {}).get("phase_b_status") != "complete":
        errors.append("生成されたsnapshotがPhase B完了を示していない")
    if snapshot.get("operation_status", {}).get("release_state") != "not claimed":
        errors.append("生成されたsnapshotがrelease readinessを主張した")
    if not any(problem.get("classification") == "release_blocker" for problem in snapshot.get("problems", [])):
        errors.append("生成されたsnapshotに想定されるrelease blockerがない")
    problem_ids = {problem.get("problem_id") for problem in snapshot.get("problems", []) if isinstance(problem, dict)}
    if "audit-anchor-external-tamper-evidence-missing" not in problem_ids:
        errors.append("生成されたsnapshotにaudit anchorの外部tamper-evidence blockerがない")
    computed_blockers = sum(
        1
        for problem in snapshot.get("problems", [])
        if isinstance(problem, dict) and problem.get("classification") == "release_blocker"
    )
    if snapshot.get("release_blocker_count") != computed_blockers:
        errors.append("生成されたsnapshotのrelease_blocker_countがproblemsから計算されていない")
    playbook_ids = {item.get("recovery_id") for item in snapshot.get("recovery_playbook", []) if isinstance(item, dict)}
    if "recover-audit-anchor-external-proof" not in playbook_ids:
        errors.append("生成されたsnapshotにaudit anchor外部recovery playbookの項目がない")
    if release_evidence.exists() != existed_before:
        errors.append("shell snapshot generatorがWindows release evidenceを作成または削除した")
    return errors


def test_evidence_bundle_is_development_classified_and_non_authoritative() -> list[str]:
    bundle = build_evidence_bundle()
    errors = validate_evidence_bundle(bundle)
    if bundle.get("release_ready") is not False:
        errors.append("evidence bundleがrelease readinessを主張した")
    if bundle.get("classification") != "development_evidence":
        errors.append("evidence bundleがdevelopment_evidenceに分類されていない")
    if not bundle.get("blockers"):
        evidence_path = ROOT / "release_evidence" / "windows_installed_smoke.json"
        if not evidence_path.exists():
            errors.append("evidence bundleがWindows installed-path欠落のblockerを保持しなかった")
        else:
            windows_results = validate_windows_release_evidence(evidence_path)
            if any(result.classification == "release_blocker" for result in windows_results):
                errors.append("evidence bundleが失敗中のWindows installed-path blockerを落とした")
    blocker_names = {blocker.get("name") for blocker in bundle.get("blockers", []) if isinstance(blocker, dict)}
    evidence_path = ROOT / "release_evidence" / "windows_installed_smoke.json"
    if not evidence_path.exists() and "audit_anchor_external_tamper_evidence_proof" not in blocker_names:
        errors.append("evidence bundleがaudit anchorの外部tamper-evidence blockerを保持しなかった")
    for index, blocker in enumerate(bundle.get("blockers", [])):
        if not isinstance(blocker, dict):
            errors.append(f"evidence bundleのblocker {index} が構造化metadataではない")
            continue
        for key in ["name", "status", "classification", "blocks_release", "reason", "required_action"]:
            if key not in blocker:
                errors.append(f"evidence bundleのblocker {index} に{key}がない")
        if blocker.get("classification") != "release_blocker":
            errors.append(f"evidence bundleのblocker {index} がrelease_blockerに分類されていない")
        if blocker.get("blocks_release") is not True:
            errors.append(f"evidence bundleのblocker {index} がreleaseを阻止していない")
    if bundle.get("authority_boundary", {}).get("flutter_owns_authority") is not False:
        errors.append("evidence bundleがFlutterを権威化した")
    return errors


def _valid_windows_installed_evidence() -> dict:
    setup_checks = []
    for check_id, evidence_class in [
        ("setup_doctor.ran_from_installed_app_path", "LIVE_RUNTIME"),
        ("setup_doctor.runtime_connection", "LIVE_RUNTIME"),
        ("setup_doctor.authority_boundary", "CONFIG"),
        ("setup_doctor.network_public_bind", "LIVE_RUNTIME"),
        ("setup_doctor.recovery_instruction", "CONFIG"),
        ("setup_doctor.audit_storage", "LIVE_RUNTIME"),
        ("setup_doctor.config_created", "CONFIG"),
    ]:
        setup_checks.append(
            {
                "check_id": check_id,
                "status": "pass",
                "message": f"{check_id} passed",
                "recovery_instruction": "Rerun installed-path smoke after remediation.",
                "evidence_class": evidence_class,
                "grants_authority": False,
            }
        )
    setup_report = {
        "version": 1,
        "report_id": "setup-doctor-" + "c" * 64,
        "generated_at": "2026-06-05T00:00:00Z",
        "status": "pass",
        "evidence_source": "LIVE_RUNTIME",
        "checks": setup_checks,
        "installer_grants_authority": False,
        "installer_silently_approves_permissions": False,
    }
    setup_report_bytes = json.dumps(setup_report, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
    setup_report_sha256 = "sha256:" + hashlib.sha256(setup_report_bytes).hexdigest()
    setup_report_path = r"C:\Users\test\AppData\Local\D4Pocket-installed-smoke-run-20260605T000000Z-a1b2c3d4-smoke-0123\GUI-Shell\broker\desktop\store\setup_doctor_report.json"
    setup_audit_event = {
        "event_id": "broker-audit-42",
        "request_id": "setup-report-request-1",
        "operation": "Setup Doctor報告取得",
        "decision": "accepted",
        "reason": "setup_doctor_report_exported",
        "evidence_source": "LIVE_RUNTIME",
        "payload_hash": setup_report_sha256,
        "event_hash": "sha256:" + "d" * 64,
    }
    ui_window_rect = {"x": 0, "y": 0, "width": 1920, "height": 1080}
    ui_required_elements = []
    ui_y = 160
    def add_ui_observation(key: str, expected: str) -> None:
        nonlocal ui_y
        index = len(ui_required_elements)
        ui_required_elements.append({
            "key": key,
            "expected": expected,
            "matched": True,
            "scroll_pass": 1,
            "observation": {
                "process_id": 1234,
                "window_runtime_id": "window-1",
                "runtime_id": f"ui-{index}",
                "parent_runtime_id": "window-1",
                "name": expected,
                "control_type": "ControlType.Text",
                "is_offscreen": False,
                "bounding_rectangle": {"x": 260, "y": ui_y, "width": 800, "height": 22},
                "visible_sample": {"x": 300, "y": ui_y + 10, "topmost_native_window_handle": 100},
            },
        })
        ui_y += 28
    add_ui_observation("page_title", "環境診断")
    add_ui_observation("status_summary", "診断状態: 正常")
    add_ui_observation("scope_notice", "この診断はPermissionやApprovalを作らず、製品リリースの完成判定にも使いません。")
    add_ui_observation("authority_notice", "インストーラーは権限を付与しません。")
    add_ui_observation("approval_notice", "インストーラーはPermissionを自動承認しません。")
    status_labels = {"pass": "正常", "warning": "確認が必要", "fail": "問題あり"}
    check_titles = {
        "setup_doctor.ran_from_installed_app_path": "製品配置",
        "setup_doctor.runtime_connection": "Broker接続",
        "setup_doctor.authority_boundary": "権限境界",
        "setup_doctor.network_public_bind": "通信範囲",
        "setup_doctor.recovery_instruction": "復旧案内",
        "setup_doctor.audit_storage": "監査保存",
        "setup_doctor.config_created": "初回設定",
    }
    for check in setup_checks:
        prefix = f"check:{check['check_id']}"
        add_ui_observation(f"{prefix}:title", check_titles[check["check_id"]])
        add_ui_observation(f"{prefix}:status", status_labels[check["status"]])
        add_ui_observation(f"{prefix}:message", check["message"])
        if check["status"] != "pass":
            add_ui_observation(f"{prefix}:recovery", check["recovery_instruction"])
    ui_proof = {
        "evidence_version": 1,
        "status": "passed",
        "source": "uiautomation",
        "evidence_class": "LIVE_RUNTIME",
        "measurement_scope": "前景の可視UI Automation文字と最前面点の観測",
        "visual_contrast_measured": False,
        "screen_reader_executed": False,
        "collector": "installer/windows/collect_installed_smoke.ps1",
        "collector_version": "17",
        "captured_at": "2026-06-05T00:00:00Z",
        "run_id": "run-20260605T000000Z-a1b2c3d4-smoke-0123",
        "process_id": 1234,
        "main_window_handle": 100,
        "report_sha256": setup_report_sha256,
        "navigation": {
            "label": "診断",
            "action": "visible_uia_element_pointer_click",
            "matched": True,
            "observation": {
                "process_id": 1234,
                "window_runtime_id": "window-1",
                "runtime_id": "nav-1",
                "name": "診断\nタブ: 2/20",
                "control_type": "ControlType.Text",
                "is_offscreen": False,
                "bounding_rectangle": {"x": 30, "y": 300, "width": 100, "height": 48},
                "visible_sample": {"x": 50, "y": 320, "topmost_native_window_handle": 100},
            },
        },
        "window": {
            "runtime_id": "window-1",
            "native_window_handle": 100,
            "parent_runtime_id": "",
            "process_id": 1234,
            "name": "D4 Pocket",
            "control_type": "ControlType.Window",
            "is_offscreen": False,
            "bounding_rectangle": ui_window_rect,
        },
        "scroll_passes": 1,
        "required_elements": ui_required_elements,
        "errors": [],
    }
    ui_evidence_path = r"C:\evidence\setup-doctor-ui.json"
    ui_evidence_sha256 = "sha256:" + "5" * 64
    data = {
        "platform": "windows",
        "provenance": {
            "evidence_contract_version": 2,
            "run_id": "run-20260605T000000Z-a1b2c3d4",
            "source_commit": "a" * 40,
            "source_worktree_clean": True,
            "source_status_porcelain": "",
            "build_command": "flutter build windows --release; cargo build --release",
            "build_timestamp": "2026-06-05T00:00:00Z",
            "staged_manifest_path": r"C:\Users\owner\AppData\Local\GUI-Shell\installed-runs\run-20260605T000000Z-a1b2c3d4\installed_manifest.json",
            "installed_manifest_sha256": "sha256:" + "2" * 64,
            "app_artifact_sha256": "sha256:" + "1" * 64,
            "launcher_artifact_sha256": "sha256:" + "b" * 64,
            "broker_artifact_sha256": "sha256:" + "3" * 64,
            "isolation": {
                "uses_shared_fixed_install_root": False,
                "isolated_install_root": r"C:\Users\owner\AppData\Local\GUI-Shell\installed-runs\run-20260605T000000Z-a1b2c3d4",
                "isolated_localappdata": r"C:\Users\test\AppData\Local\D4Pocket-installed-smoke-run-20260605T000000Z-a1b2c3d4-smoke-0123",
                "isolated_runtime_dir": r"C:\Users\test\AppData\Local\D4Pocket-installed-smoke-run-20260605T000000Z-a1b2c3d4-smoke-0123\GUI-Shell\broker\desktop",
                "isolated_store_dir": r"C:\Users\test\AppData\Local\D4Pocket-installed-smoke-run-20260605T000000Z-a1b2c3d4-smoke-0123\GUI-Shell\broker\desktop\store",
                "isolated_config_dir": None,
                "isolated_audit_dir": r"C:\Users\test\AppData\Local\D4Pocket-installed-smoke-run-20260605T000000Z-a1b2c3d4-smoke-0123\GUI-Shell\broker\desktop\store",
                "runtime_identity": {
                    "kind": "gui_shell",
                    "app_id": None,
                    "audit_store_id": None,
                    "product_manifest_path": None,
                    "product_manifest_sha256": None,
                    "evidence_class": "CONFIG",
                },
                "run_id": "run-20260605T000000Z-a1b2c3d4-smoke-0123",
                "separate_windows_user_profile": True,
            },
            "evidence_bundle_sha256": "sha256:" + "4" * 64,
            "evidence_bundle_files": [
                {"kind": "setup_doctor", "path": setup_report_path, "exists": True, "sha256": setup_report_sha256},
                {"kind": "setup_doctor_operator_readability", "path": ui_evidence_path, "exists": True, "sha256": ui_evidence_sha256},
                {"kind": "broker_smoke", "path": r"C:\evidence\broker.json", "sha256": "sha256:" + "6" * 64},
                {"kind": "broker_lifecycle_audit", "path": r"C:\evidence\broker-audit.jsonl", "sha256": "sha256:" + "a" * 64},
                {"kind": "first_run_configuration", "path": r"C:\Users\test\AppData\Local\D4Pocket-installed-smoke-run-20260605T000000Z-a1b2c3d4-smoke-0123\GUI-Shell\broker\desktop\store\first_run_configuration.json", "exists": True, "sha256": "sha256:" + "c" * 64},
                {"kind": "visible_surfaces", "path": r"C:\evidence\visible_surfaces.json", "sha256": "sha256:" + "7" * 64},
                {"kind": "runtime_assertions", "path": r"C:\evidence\runtime_assertions.json", "sha256": "sha256:" + "8" * 64},
                {"kind": "audit_anchor_external_tamper_evidence", "path": r"C:\evidence\audit_anchor_external.json", "sha256": "sha256:" + "9" * 64},
            ],
        },
        "field_provenance": {
            "artifact": {"source_type": "directly_measured", "evidence_class": "EXTERNAL_EVIDENCE", "formal_release_input": True},
            "first_run.process": {"source_type": "directly_measured", "evidence_class": "LIVE_RUNTIME", "formal_release_input": True},
            "first_run.visible_surfaces": {"source_type": "directly_measured", "evidence_class": "LIVE_RUNTIME", "formal_release_input": True},
            "first_run.config_audit": {"source_type": "directly_measured", "evidence_class": "LIVE_RUNTIME", "formal_release_input": True},
            "first_run.broker_lifecycle_audit": {"source_type": "directly_measured", "evidence_class": "LIVE_RUNTIME", "formal_release_input": True},
            "first_run.broker_health_request": {"source_type": "directly_measured", "evidence_class": "LIVE_RUNTIME", "formal_release_input": True},
            "first_run.launcher_runtime_identity": {"source_type": "static_assertion", "evidence_class": "CONFIG", "formal_release_input": True},
            "first_run.installer_authority_boundary": {"source_type": "static_assertion", "evidence_class": "CONFIG", "formal_release_input": True},
            "setup_doctor": {"source_type": "product_export", "evidence_class": "LIVE_RUNTIME", "formal_release_input": True},
            "broker.ipc_restart_crash": {"source_type": "directly_measured", "evidence_class": "LIVE_RUNTIME", "formal_release_input": True},
            "audit_anchor.external_tamper_evidence": {"source_type": "directly_measured", "evidence_class": "EXTERNAL_EVIDENCE", "formal_release_input": True},
            "release_runtime_assertions": {"source_type": "static_assertion", "evidence_class": ["CONFIG", "FIXTURE"], "formal_release_input": True},
            "unsupported_claims": [],
        },
        "evidence_source": {
            "collector": "installer/windows/collect_installed_smoke.ps1",
            "collector_version": "17",
            "manual_confirmation": False,
            "screenshot_path": r"C:\ProgramData\GUI-Shell\evidence\first-window.png",
        },
        "artifact": {
            "installed_exe_path": r"C:\Users\owner\AppData\Local\GUI-Shell\installed-runs\run-20260605T000000Z-a1b2c3d4\app\gui_shell_desktop.exe",
            "installed_exe_exists": True,
            "sha256": "sha256:" + "1" * 64,
            "desktop_launcher_path": r"C:\Users\owner\AppData\Local\GUI-Shell\installed-runs\run-20260605T000000Z-a1b2c3d4\gui_shell_desktop_launcher.exe",
            "desktop_launcher_sha256": "sha256:" + "b" * 64,
            "broker_helper_path": r"C:\Users\owner\AppData\Local\GUI-Shell\installed-runs\run-20260605T000000Z-a1b2c3d4\broker\gui_shell_rust_helper.exe",
            "broker_helper_sha256": "sha256:" + "3" * 64,
        },
        "first_run": {
            "status": "passed",
            "command": r".\gui_shell_desktop.exe",
            "launched_from_installed_path": True,
            "launched_via_rust_desktop_launcher": True,
            "launcher_process_id": 4321,
            "launcher_exited_after_frontend": True,
            "launcher_exit_code": 0,
            "launcher_runtime_dir": r"C:\Users\test\AppData\Local\D4Pocket-installed-smoke-run-20260605T000000Z-a1b2c3d4-smoke-0123\GUI-Shell\broker\desktop",
            "launcher_runtime_identity": {
                "kind": "gui_shell",
                "app_id": None,
                "audit_store_id": None,
                "product_manifest_path": None,
                "product_manifest_sha256": None,
                "evidence_class": "CONFIG",
            },
            "profile_identity_isolated_from_staging_user": True,
            "profile_identity_sid_exposed": False,
            "config_existed_before_launch": False,
            "process_id": 1234,
            "process_identity": {
                "method": "direct_parent_pid_and_sha256",
                "evidence_class": "LIVE_RUNTIME",
                "direct_child_observed": True,
                "parent_process_id": 4321,
                "same_windows_session": True,
                "started_after_launcher": True,
                "image_sha256": "sha256:" + "1" * 64,
                "image_matches_manifest": True,
            },
            "process_running_after_launch": True,
            "frontend_exit_menu_invoked": True,
            "frontend_forced_to_exit": False,
            "frontend_cleanup_error": False,
            "main_window_handle": 100,
            "window_title": "GUI-Shell",
            "first_window_visible": True,
            "broker_mediated_launch": True,
            "broker_helper_path": r"C:\Program Files\GUI-Shell\broker\gui_shell_rust_helper.exe",
            "broker_endpoint_file": r"C:\Users\test\AppData\Local\D4Pocket-installed-smoke-run-20260605T000000Z-a1b2c3d4-smoke-0123\GUI-Shell\broker\desktop\broker_session.json",
            "broker_endpoint_created": True,
            "broker_endpoint_removed_after_shutdown": True,
            "broker_transport": "authenticated_loopback_tcp",
            "broker_endpoint_credential_role": "normal",
            "normal_endpoint_credential_role_verified": True,
            "broker_health_request": {
                "accepted": True,
                "accepted_event_count": 1,
                "first_audit_event_id": "broker-audit-2",
                "evidence_class": "LIVE_RUNTIME",
                "caller_process_attributed": False,
                "client_response_receipt_observed": False,
            },
            "broker_lifecycle_audit": {
                "path": r"C:\Users\test\AppData\Local\D4Pocket-installed-smoke-run-20260605T000000Z-a1b2c3d4-smoke-0123\GUI-Shell\broker\desktop\store\audit.jsonl",
                "sha256": "sha256:" + "c" * 64,
                "startup_event_count": 1,
                "startup_event_recorded": True,
                "shutdown_event_count": 1,
                "shutdown_event_recorded": True,
                "evidence_class": "LIVE_RUNTIME",
            },
            "no_python_runtime_requested": True,
            "python_runtime_path_scrubbed": True,
            "python_path_entries_removed_count": 2,
            "python_path_entries_remaining_count": 0,
            "python_commands_visible_after_scrub": [],
            "visible_surfaces_complete": True,
            "visible_surfaces": ["Dashboard", "NavigationRail", "Runtime Status", "Invariant Status"],
            "visible_surfaces_evidence": {
                "source": "uiautomation",
                "path": r"C:\ProgramData\GUI-Shell\evidence\visible-surfaces.json",
                "captured_at": "2026-05-26T00:00:00Z",
                "surface_matches": {
                    "Dashboard": {
                        "matched": True,
                        "name": "Dashboard",
                        "automation_id": "",
                        "control_type": "ControlType.Text",
                        "class_name": "",
                        "framework_id": "Flutter",
                        "element_key": "descendant:1",
                        "is_root": False,
                        "is_native_container": False,
                        "surfaces_present": ["Dashboard"],
                    },
                    "NavigationRail": {
                        "matched": True,
                        "name": "NavigationRail",
                        "automation_id": "",
                        "control_type": "ControlType.Group",
                        "class_name": "",
                        "framework_id": "Flutter",
                        "element_key": "descendant:2",
                        "is_root": False,
                        "is_native_container": False,
                        "surfaces_present": ["NavigationRail"],
                    },
                    "Runtime Status": {
                        "matched": True,
                        "name": "Runtime Status",
                        "automation_id": "",
                        "control_type": "ControlType.Text",
                        "class_name": "",
                        "framework_id": "Flutter",
                        "element_key": "descendant:3",
                        "is_root": False,
                        "is_native_container": False,
                        "surfaces_present": ["Runtime Status"],
                    },
                    "Invariant Status": {
                        "matched": True,
                        "name": "Invariant Status",
                        "automation_id": "",
                        "control_type": "ControlType.Text",
                        "class_name": "",
                        "framework_id": "Flutter",
                        "element_key": "descendant:4",
                        "is_root": False,
                        "is_native_container": False,
                        "surfaces_present": ["Invariant Status"],
                    },
                },
                "aggregate_surface_shortcut_detected": False,
                "surface_match_requirements_met": True,
                "diagnostic_tree": {
                    "mode": "full_uiautomation_tree_projection",
                    "tree_view": "control",
                    "observed_element_count": 5,
                    "observed_elements": [
                        {
                            "element_key": "root",
                            "runtime_id": "1.2",
                            "parent_runtime_id": "",
                            "name": "GUI Shell",
                            "automation_id": "",
                            "control_type": "ControlType.Window",
                            "class_name": "FlutterView",
                            "framework_id": "Win32",
                            "supported_patterns": ["WindowPatternIdentifiers.Pattern"],
                        },
                        {
                            "element_key": "descendant:1",
                            "runtime_id": "1.2.1",
                            "parent_runtime_id": "1.2",
                            "name": "Dashboard",
                            "automation_id": "",
                            "control_type": "ControlType.Text",
                            "class_name": "",
                            "framework_id": "Flutter",
                            "supported_patterns": [],
                        },
                    ],
                    "tree_edges": [
                        {"child_runtime_id": "1.2.1", "parent_runtime_id": "1.2", "child_element_key": "descendant:1"}
                    ],
                    "capture_limit": "none",
                },
            },
            "config_path": r"C:\Users\test\AppData\Local\D4Pocket-installed-smoke-run-20260605T000000Z-a1b2c3d4-smoke-0123\GUI-Shell\broker\desktop\store\first_run_configuration.json",
            "config_created": True,
            "config_json_valid": True,
            "config_audit": {
                "evidence_class": "LIVE_RUNTIME",
                "accepted_event_count": 1,
                "config_sha256": "sha256:" + "c" * 64,
                "accepted_event": {
                    "event_id": "broker-audit-12",
                    "request_id": "first-run-config-request",
                    "operation": "初回設定取得",
                    "decision": "accepted",
                    "reason": "初回UI設定を固定Broker storeからprojection。authorityは生成しない",
                    "evidence_source": "LIVE_RUNTIME",
                    "payload_hash": "sha256:" + "c" * 64,
                    "event_hash": "sha256:" + "d" * 64,
                },
            },
            "audit_dir": r"C:\ProgramData\GUI-Shell\audit",
            "audit_dir_writable": True,
            "audit_write_probe": {
                "attempted": True,
                "write": True,
                "read": True,
                "delete": True,
                "probe_path": r"C:\ProgramData\GUI-Shell\audit\.gui-shell-write-probe",
            },
            "installer_grants_authority": False,
            "installer_silently_approves_permissions": False,
        },
        "setup_doctor": {
            "status": "pass",
            "formal_product_evidence": True,
            "report_path": setup_report_path,
            "report_sha256": setup_report_sha256,
            "serialized_report_base64": base64.b64encode(setup_report_bytes).decode("ascii"),
            "product_report": setup_report,
            "accepted_audit_event": setup_audit_event,
            "evidence_source": {
                "source_kind": "installed_app_machine_readable_export",
                "product_generated": True,
                "collector_derives_checks": False,
                "synthetic": False,
                "command": "通常起動したD4 Pocket UIが認証済みBroker IPCのSetup Doctor報告取得を呼び出し",
                "accepted_audit_event_id": setup_audit_event["event_id"],
                "accepted_audit_event_payload_hash": setup_report_sha256,
            },
            "ran_from_installed_app_path": True,
            "operator_readable": True,
            "operator_readability_evidence": {
                "source": "uiautomation",
                "evidence_class": "LIVE_RUNTIME",
                "status": "passed",
                "path": ui_evidence_path,
                "sha256": ui_evidence_sha256,
                "process_id": 1234,
                "run_id": ui_proof["run_id"],
                "report_sha256": setup_report_sha256,
            },
            "operator_readability_proof": ui_proof,
            "installer_grants_authority": False,
            "installer_silently_approves_permissions": False,
            "checks": setup_checks,
        },
        "broker": {
            "status": "passed",
            "evidence_source": {
                "collector": "installer/windows/collect_broker_smoke.ps1",
                "collector_version": "6",
                "synthetic": False,
                "command": r"powershell -ExecutionPolicy Bypass -File installer\windows\collect_broker_smoke.ps1",
            },
            "helper_exe_path": r"C:\Program Files\GUI-Shell\broker\gui_shell_rust_helper.exe",
            "helper_exe_exists": True,
            "session_file": r"C:\ProgramData\GUI-Shell\broker\broker_session.json",
            "session_file_created": True,
            "session_file_removed_after_collection": True,
            "store_dir": r"C:\ProgramData\GUI-Shell\broker\store",
            "endpoint_host": "127.0.0.1",
            "endpoint_port": 49152,
            "endpoint_credential_role": "normal",
            "normal_endpoint_credential_role_verified": True,
            "restricted_loopback_bind": True,
            "authenticated_ipc_connection": True,
            "durable_store_ready": True,
            "normal_ipc_agent_task_workspace_permission_denied": True,
            "agent_task_workspace_permission_error_code": "desktop_native_owner_confirmation_required",
            "normal_ipc_agent_task_owner_approval_denied": True,
            "agent_task_owner_approval_error_code": "desktop_native_owner_confirmation_required",
            "replay_nonce": "windows-installed-replay-nonce",
            "restart_replay_rejected": True,
            "replay_error_code": "broker_replay_detected",
            "fresh_health_after_restart": True,
            "crash_fail_closed": True,
            "field_provenance": {
                "helper_exe_exists": {"source_type": "directly_measured", "evidence_class": "EXTERNAL_EVIDENCE"},
                "session_file_created": {"source_type": "directly_measured", "evidence_class": "LIVE_RUNTIME"},
                "session_file_removed_after_collection": {"source_type": "directly_measured", "evidence_class": "EXTERNAL_EVIDENCE"},
                "normal_endpoint_credential_role_verified": {"source_type": "directly_measured", "evidence_class": "LIVE_RUNTIME"},
                "restricted_loopback_bind": {"source_type": "directly_measured", "evidence_class": "LIVE_RUNTIME"},
                "authenticated_ipc_connection": {"source_type": "directly_measured", "evidence_class": "LIVE_RUNTIME"},
                "durable_store_ready": {"source_type": "directly_measured", "evidence_class": "LIVE_RUNTIME"},
                "normal_ipc_agent_task_workspace_permission_denied": {"source_type": "directly_measured", "evidence_class": "LIVE_RUNTIME"},
                "normal_ipc_agent_task_owner_approval_denied": {"source_type": "directly_measured", "evidence_class": "LIVE_RUNTIME"},
                "restart_replay_rejected": {"source_type": "directly_measured", "evidence_class": "LIVE_RUNTIME"},
                "fresh_health_after_restart": {"source_type": "directly_measured", "evidence_class": "LIVE_RUNTIME"},
                "crash_fail_closed": {"source_type": "directly_measured", "evidence_class": "LIVE_RUNTIME"},
            },
            "unmeasured_declarations": {
                "python_runtime_required_for_authority": {
                    "value": False,
                    "source_type": "static_assertion",
                    "evidence_class": "CONFIG",
                    "formal_runtime_proof": False,
                },
                "flutter_rust_ffi_authority_bridge": {
                    "value": False,
                    "source_type": "static_assertion",
                    "evidence_class": "CONFIG",
                    "formal_runtime_proof": False,
                },
            },
            "errors": [],
        },
        "audit_anchor_external_tamper_evidence": {
            "status": "passed",
            "installed_path_verified": True,
            "key_anchor_log_same_user_rewrite_mitigated": True,
            "windows_acl_verified": True,
            "dpapi_verified": True,
            "external_anchor_verified": False,
            "signed_evidence_verified": False,
            "administrator_root_resistance_claimed": False,
            "evidence_source": {
                "source_kind": "windows_acl_dpapi_probe",
                "evidence_class": "EXTERNAL_EVIDENCE",
                "synthetic": False,
                "command": r"powershell -ExecutionPolicy Bypass -File installer\windows\collect_audit_anchor_proof.ps1",
                "path": r"C:\evidence\audit_anchor_external.json",
                "sha256": "sha256:" + "9" * 64,
            },
        },
    }

    surface = data["first_run"]["visible_surfaces_evidence"]
    tree = surface["diagnostic_tree"]
    root = tree["observed_elements"][0]
    root.update(is_root=True, is_native_container=True, is_offscreen=False,
                bounding_rectangle={"x": 0, "y": 0, "width": 1000, "height": 800})
    tree["observed_elements"] = [root]
    tree["tree_edges"] = []
    for index, match in enumerate(surface["surface_matches"].values(), 1):
        node = {k: v for k, v in match.items() if k != "matched"}
        node.update(runtime_id=f"1.2.{index}", parent_runtime_id="1.2",
                    supported_patterns=[], is_offscreen=False,
                    bounding_rectangle={"x": 10, "y": index * 50, "width": 100, "height": 30})
        tree["observed_elements"].append(node)
        tree["tree_edges"].append({"child_runtime_id": node["runtime_id"],
                                   "parent_runtime_id": "1.2", "child_element_key": node["element_key"]})
    return data


def test_windows_release_evidence_validator_accepts_other_checks_but_rejects_unbound_anchor() -> list[str]:
    errors = []
    # 他の製品経路の正常fixtureは維持し、未接続のアンカー宣言だけを拒否する。
    for source_kind in ("windows_acl_dpapi_probe", "external_anchor", "signed_evidence"):
        data = _valid_windows_installed_evidence()
        anchor = data["audit_anchor_external_tamper_evidence"]
        anchor["evidence_source"]["source_kind"] = source_kind
        anchor["external_anchor_verified"] = source_kind == "external_anchor"
        anchor["signed_evidence_verified"] = source_kind == "signed_evidence"
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "windows_installed_smoke.json"
            path.write_text(json.dumps(data), encoding="utf-8")
            results = validate_windows_release_evidence(path)
        for result in results:
            if result.name == "audit_anchor_external_tamper_evidence_proof":
                if result.status == "passed" or result.classification != "release_blocker":
                    errors.append(f"{source_kind}の未結合アンカー宣言を受理した")
            elif result.status != "passed" or result.classification == "release_blocker":
                errors.append(f"{result.name} が他の有効なWindows evidenceを拒否した: {result.reason}")
    return errors


def test_windows_release_evidence_validator_rejects_missing_provenance() -> list[str]:
    bad = _valid_windows_installed_evidence()
    bad.pop("provenance")
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "windows_installed_smoke.json"
        path.write_text(json.dumps(bad), encoding="utf-8")
        results = validate_windows_release_evidence(path)
    result_by_name = {result.name: result for result in results}
    if result_by_name["windows_evidence_provenance_isolation"].classification != "release_blocker":
        return ["Windows evidence validatorがprovenance/isolation欠落を受け入れた"]
    return []


def test_windows_release_evidence_validator_preserves_audit_anchor_external_blocker() -> list[str]:
    bad = _valid_windows_installed_evidence()
    bad.pop("audit_anchor_external_tamper_evidence")
    bad["field_provenance"].pop("audit_anchor.external_tamper_evidence")
    bad["provenance"]["evidence_bundle_files"] = [
        item
        for item in bad["provenance"]["evidence_bundle_files"]
        if item.get("kind") != "audit_anchor_external_tamper_evidence"
    ]
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "windows_installed_smoke.json"
        path.write_text(json.dumps(bad), encoding="utf-8")
        results = validate_windows_release_evidence(path)
    result_by_name = {result.name: result for result in results}
    errors = []
    if result_by_name["audit_anchor_external_tamper_evidence_proof"].classification != "release_blocker":
        errors.append("Windows evidence validatorがaudit anchorの外部tamper-evidence release blockerを落とした")
    if result_by_name["windows_evidence_provenance_isolation"].classification == "release_blocker":
        errors.append("Windows provenance validatorがaudit anchor evidence bundle/provenance欠落の責任を依然として持っている")
    return errors


def test_windows_release_evidence_validator_rejects_authority_and_missing_installed_path() -> list[str]:
    bad = _valid_windows_installed_evidence()
    bad["artifact"]["installed_exe_exists"] = False
    bad["first_run"]["installer_grants_authority"] = True
    bad["setup_doctor"]["checks"][0]["grants_authority"] = True
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "windows_installed_smoke.json"
        path.write_text(json.dumps(bad), encoding="utf-8")
        results = validate_windows_release_evidence(path)
    result_by_name = {result.name: result for result in results}
    errors = []
    if result_by_name["windows_installer_first_run_smoke"].classification != "release_blocker":
        errors.append("Windows first-run evidence validatorがinstalled pathまたはinstaller authorityの欠落を受け入れた")
    if result_by_name["windows_setup_doctor_smoke"].classification != "release_blocker":
        errors.append("Windows Setup Doctorのevidence validatorがauthorityを付与するcheckを受け入れた")
    return errors


def test_windows_release_evidence_validator_rejects_preexisting_first_run_config() -> list[str]:
    bad = _valid_windows_installed_evidence()
    bad["first_run"]["config_existed_before_launch"] = True
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "windows_installed_smoke.json"
        path.write_text(json.dumps(bad), encoding="utf-8")
        results = validate_windows_release_evidence(path)
    result_by_name = {result.name: result for result in results}
    if result_by_name["windows_installer_first_run_smoke"].classification != "release_blocker":
        return ["Windows first-run validatorが起動前から存在するconfigを初回生成として受け入れた"]
    return []


def test_windows_release_evidence_rejects_unbound_first_run_config_audit() -> list[str]:
    errors: list[str] = []
    cases = (
        (
            "AuditEvent payload hashがconfigと異なる",
            lambda item: item["first_run"]["config_audit"]["accepted_event"].update(
                payload_hash="sha256:" + "e" * 64
            ),
        ),
        (
            "config fileがevidence bundleにない",
            lambda item: item["provenance"]["evidence_bundle_files"].__setitem__(
                slice(None),
                [
                    record for record in item["provenance"]["evidence_bundle_files"]
                    if record["kind"] != "first_run_configuration"
                ],
            ),
        ),
    )
    for label, mutate in cases:
        evidence = _valid_windows_installed_evidence()
        mutate(evidence)
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "windows_installed_smoke.json"
            path.write_text(json.dumps(evidence), encoding="utf-8")
            results = validate_windows_release_evidence(path)
        result_by_name = {result.name: result for result in results}
        if result_by_name["windows_installer_first_run_smoke"].classification != "release_blocker":
            errors.append(f"Windows first-run validatorが{label}を受け入れた")
    return errors


def test_windows_release_evidence_validator_rejects_external_setup_probe_as_product_evidence() -> list[str]:
    bad = _valid_windows_installed_evidence()
    bad["setup_doctor"]["formal_product_evidence"] = False
    bad["setup_doctor"]["evidence_source"]["source_kind"] = "external_installer_config_broker_probe"
    bad["setup_doctor"]["evidence_source"]["product_generated"] = False
    bad["setup_doctor"]["evidence_source"]["collector_derives_checks"] = True
    bad["field_provenance"]["setup_doctor"]["source_type"] = "external_probe"
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "windows_installed_smoke.json"
        path.write_text(json.dumps(bad), encoding="utf-8")
        results = validate_windows_release_evidence(path)
    result_by_name = {result.name: result for result in results}
    errors = []
    if result_by_name["windows_setup_doctor_smoke"].classification != "release_blocker":
        errors.append("Windows Setup Doctor validatorがexternal probeをformal product evidenceとして受け入れた")
    if result_by_name["windows_evidence_provenance_isolation"].classification != "release_blocker":
        errors.append("Windows provenance validatorがexternal Setup Doctor provenanceをproduct exportとして受け入れた")
    return errors


def test_windows_setup_doctor_does_not_promote_unknown_to_release_pass() -> list[str]:
    bad = _valid_windows_installed_evidence()
    setup = bad["setup_doctor"]
    setup["status"] = "warning"
    setup["product_report"]["status"] = "warning"
    setup["product_report"]["checks"][-1]["status"] = "unknown"
    report_bytes = json.dumps(
        setup["product_report"], ensure_ascii=False, separators=(",", ":")
    ).encode("utf-8")
    report_hash = "sha256:" + hashlib.sha256(report_bytes).hexdigest()
    setup["serialized_report_base64"] = base64.b64encode(report_bytes).decode("ascii")
    setup["report_sha256"] = report_hash
    setup["accepted_audit_event"]["payload_hash"] = report_hash
    setup["evidence_source"]["accepted_audit_event_payload_hash"] = report_hash
    next(item for item in bad["provenance"]["evidence_bundle_files"] if item["kind"] == "setup_doctor")["sha256"] = report_hash
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "windows_installed_smoke.json"
        path.write_text(json.dumps(bad), encoding="utf-8")
        results = validate_windows_release_evidence(path)
    result_by_name = {result.name: result for result in results}
    if result_by_name["windows_setup_doctor_smoke"].classification != "release_blocker":
        return ["strict Windows Setup Doctor validatorがunknown checkをrelease証拠として受け入れた"]
    return []


def test_windows_setup_doctor_readability_requires_bound_visible_uia_evidence() -> list[str]:
    mutations = (
        ("UIA proof欠落", lambda data: data["setup_doctor"].pop("operator_readability_proof")),
        ("別Frontend PID", lambda data: data["setup_doctor"]["operator_readability_proof"].update(process_id=9999)),
        ("証拠範囲の改変", lambda data: data["setup_doctor"]["operator_readability_proof"].update(visual_contrast_measured=True)),
        ("別主画面Handle", lambda data: data["setup_doctor"]["operator_readability_proof"].update(main_window_handle=12345)),
        ("無関係なUIA主画面", lambda data: data["setup_doctor"]["operator_readability_proof"]["required_elements"][0]["observation"].update(window_runtime_id="foreign-window")),
        ("別窓に覆われた表示点", lambda data: data["setup_doctor"]["operator_readability_proof"]["required_elements"][0]["observation"]["visible_sample"].update(topmost_native_window_handle=777)),
        ("表示点が要素外", lambda data: data["setup_doctor"]["operator_readability_proof"]["required_elements"][0]["observation"]["visible_sample"].update(x=1800)),
        ("画面外要素", lambda data: data["setup_doctor"]["operator_readability_proof"]["required_elements"][0]["observation"].update(is_offscreen=True)),
        ("画面範囲外座標", lambda data: data["setup_doctor"]["operator_readability_proof"]["required_elements"][0]["observation"]["bounding_rectangle"].update(x=3000)),
        ("集約された偽表示", lambda data: data["setup_doctor"]["operator_readability_proof"]["required_elements"][0]["observation"].update(name="環境診断 診断状態: 正常")),
        ("重複ラベルの偽表示", lambda data: data["setup_doctor"]["operator_readability_proof"]["required_elements"][0]["observation"].update(name="環境診断 環境診断")),
        ("空のreport message", lambda data: data["setup_doctor"]["checks"][0].update(message="")),
        ("同一runtime ID再利用", lambda data: data["setup_doctor"]["operator_readability_proof"]["required_elements"][1]["observation"].update(runtime_id=data["setup_doctor"]["operator_readability_proof"]["required_elements"][0]["observation"]["runtime_id"])),
        ("sidecar未結合", lambda data: data["provenance"]["evidence_bundle_files"].__setitem__(slice(None), [record for record in data["provenance"]["evidence_bundle_files"] if record["kind"] != "setup_doctor_operator_readability"])),
    )
    errors = []
    for label, mutate in mutations:
        bad = _valid_windows_installed_evidence()
        mutate(bad)
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "windows_installed_smoke.json"
            path.write_text(json.dumps(bad), encoding="utf-8")
            results = validate_windows_release_evidence(path)
        result = {item.name: item for item in results}["windows_setup_doctor_smoke"]
        if result.classification != "release_blocker":
            errors.append(f"Setup Doctor operator-readability validatorが{label}を受け入れた")
    return errors


def test_windows_release_evidence_requires_accepted_broker_health_audit() -> list[str]:
    errors = []
    for mutate, label, expected_reason in (
        (
            lambda item: item["first_run"].update(broker_health_request={"accepted": False}),
            "endpoint role metadata only",
            "health要求",
        ),
        (
            lambda item: item["first_run"]["broker_health_request"].update(accepted_event_count=0),
            "accepted flag without an observed event",
            "AuditEvent数",
        ),
        (
            lambda item: item["first_run"]["broker_health_request"].update(client_response_receipt_observed=True),
            "unsupported response-receipt claim",
            "応答受信",
        ),
    ):
        evidence = _valid_windows_installed_evidence()
        evidence["first_run"]["normal_endpoint_credential_role_verified"] = True
        mutate(evidence)
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "windows_installed_smoke.json"
            path.write_text(json.dumps(evidence), encoding="utf-8")
            results = validate_windows_release_evidence(path)
        result_by_name = {result.name: result for result in results}
        if result_by_name["windows_installer_first_run_smoke"].classification != "release_blocker":
            errors.append(f"{label}だけでBroker health受理のLIVE_RUNTIME evidenceを代替できている")
        if expected_reason not in result_by_name["windows_installer_first_run_smoke"].reason:
            errors.append(f"{label}に対するhealth Audit validatorの理由が出力に含まれない")
    return errors


def test_windows_release_evidence_requires_verified_launcher_child() -> list[str]:
    errors = []
    for mutate, label, expected_reason in (
        (
            lambda item: item["first_run"].pop("process_identity"),
            "missing process identity",
            "childの親PID・実行image hash証拠がない",
        ),
        (
            lambda item: item["first_run"]["process_identity"].update(direct_child_observed=False),
            "non-child process",
            "直接childであることを確認できなかった",
        ),
        (
            lambda item: item["first_run"]["process_identity"].update(parent_process_id=9876),
            "mismatched parent PID",
            "observed parent PID",
        ),
        (
            lambda item: item["first_run"]["process_identity"].update(same_windows_session=False),
            "different Windows session",
            "同じWindows session",
        ),
        (
            lambda item: item["first_run"]["process_identity"].update(started_after_launcher=False),
            "pre-existing frontend process",
            "起動器起動後に作成",
        ),
        (
            lambda item: item["first_run"]["process_identity"].update(image_sha256="sha256:" + "f" * 64),
            "mismatched image hash",
            "image SHA-256",
        ),
        (
            lambda item: item["first_run"]["process_identity"].update(image_matches_manifest=False),
            "unverified image hash",
            "image hashをstaged manifestと照合できなかった",
        ),
    ):
        evidence = _valid_windows_installed_evidence()
        mutate(evidence)
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "windows_installed_smoke.json"
            path.write_text(json.dumps(evidence), encoding="utf-8")
            results = validate_windows_release_evidence(path)
        result_by_name = {result.name: result for result in results}
        if result_by_name["windows_installer_first_run_smoke"].classification != "release_blocker":
            errors.append(f"{label}をWindows installed smoke validatorが受け入れた")
        if expected_reason not in result_by_name["windows_installer_first_run_smoke"].reason:
            errors.append(f"{label}のprocess identity不備をvalidatorが具体的に指摘しなかった")
    return errors


def test_windows_release_evidence_requires_normal_frontend_exit() -> list[str]:
    errors = []
    for mutate, label, expected_reason in (
        (
            lambda item: item["first_run"].update(frontend_exit_menu_invoked=False),
            "missing product exit action",
            "通知領域『終了』操作",
        ),
        (
            lambda item: item["first_run"].update(frontend_forced_to_exit=True),
            "forced frontend termination",
            "強制終了せず通常終了",
        ),
        (
            lambda item: item["first_run"].update(frontend_cleanup_error=True),
            "frontend cleanup error",
            "cleanupでerror",
        ),
    ):
        evidence = _valid_windows_installed_evidence()
        mutate(evidence)
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "windows_installed_smoke.json"
            path.write_text(json.dumps(evidence), encoding="utf-8")
            results = validate_windows_release_evidence(path)
        result_by_name = {result.name: result for result in results}
        if result_by_name["windows_installer_first_run_smoke"].classification != "release_blocker":
            errors.append(f"{label}をWindows installed smoke validatorが受け入れた")
        if expected_reason not in result_by_name["windows_installer_first_run_smoke"].reason:
            errors.append(f"{label}をvalidatorが具体的に指摘しなかった")
    return errors


def test_windows_release_evidence_rejects_staged_runtime_scratch_path() -> list[str]:
    evidence = _valid_windows_installed_evidence()
    evidence["first_run"]["broker_endpoint_file"] = (
        evidence["provenance"]["isolation"]["isolated_install_root"] + r"\runtime\broker_session.json"
    )
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "windows_installed_smoke.json"
        path.write_text(json.dumps(evidence), encoding="utf-8")
        results = validate_windows_release_evidence(path)
    result_by_name = {result.name: result for result in results}
    if result_by_name["windows_evidence_provenance_isolation"].classification != "release_blocker":
        return ["stage manifestのcollector scratch endpoint pathを起動器runtimeとして受け入れた"]
    return []


def test_windows_release_evidence_rejects_malformed_product_runtime_identity() -> list[str]:
    evidence = _valid_windows_installed_evidence()
    app_id = "d4-pocket-app-" + "1" * 32
    audit_store_id = "audit-store-" + "2" * 32
    runtime_root = (
        r"C:\Users\test\AppData\Local\D4Pocket-installed-smoke-run-20260605T000000Z-a1b2c3d4-smoke-0123"
        + rf"\D4Pocket\apps\{app_id}\stores\{audit_store_id}"
    )
    product_manifest_path = r"C:\Users\owner\AppData\Local\GUI-Shell\installed-runs\run-20260605T000000Z-a1b2c3d4\product_manifest.json"
    identity = {
        "kind": "d4_pocket_product",
        "app_id": app_id,
        "audit_store_id": audit_store_id,
        "product_manifest_path": product_manifest_path,
        "product_manifest_sha256": "sha256:" + "b" * 64,
        "evidence_class": "CONFIG",
    }
    isolation = evidence["provenance"]["isolation"]
    isolation["isolated_runtime_dir"] = runtime_root
    isolation["isolated_store_dir"] = runtime_root + r"\store"
    isolation["isolated_audit_dir"] = runtime_root + r"\store"
    isolation["runtime_identity"] = identity
    evidence["first_run"]["launcher_runtime_dir"] = runtime_root
    evidence["first_run"]["broker_endpoint_file"] = runtime_root + r"\broker_session.json"
    evidence["first_run"]["broker_lifecycle_audit"]["path"] = runtime_root + r"\store\audit.jsonl"
    evidence["first_run"]["launcher_runtime_identity"] = dict(identity)
    evidence["provenance"]["evidence_bundle_files"].append(
        {
            "kind": "product_manifest",
            "path": product_manifest_path,
            "exists": True,
            "sha256": identity["product_manifest_sha256"],
        }
    )
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "windows_installed_smoke.json"
        path.write_text(json.dumps(evidence), encoding="utf-8")
        results = validate_windows_release_evidence(path)
    result_by_name = {result.name: result for result in results}
    if result_by_name["windows_evidence_provenance_isolation"].status != "passed":
        return ["一致するD4 Pocket runtime identityとstage済みProduct Manifestを受け入れなかった: "
                + result_by_name["windows_evidence_provenance_isolation"].reason]

    outside_manifest_path = r"C:\evidence\product_manifest.json"
    identity_with_unbound_manifest = dict(identity)
    identity_with_unbound_manifest["product_manifest_path"] = outside_manifest_path
    isolation["runtime_identity"] = identity_with_unbound_manifest
    evidence["first_run"]["launcher_runtime_identity"] = dict(identity_with_unbound_manifest)
    product_manifest_record = next(
        item for item in evidence["provenance"]["evidence_bundle_files"]
        if item["kind"] == "product_manifest"
    )
    product_manifest_record["path"] = outside_manifest_path
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "windows_installed_smoke.json"
        path.write_text(json.dumps(evidence), encoding="utf-8")
        results = validate_windows_release_evidence(path)
    result_by_name = {result.name: result for result in results}
    result = result_by_name["windows_evidence_provenance_isolation"]
    if result.classification != "release_blocker" or "installed Product Manifest" not in result.reason:
        return ["stage済みinstalled manifest外のProduct ManifestをWindows evidenceとして拒否しなかった"]

    malformed_identity = dict(identity)
    malformed_identity["app_id"] = "d4-pocket-app-invalid"
    isolation["runtime_identity"] = malformed_identity
    evidence["first_run"]["launcher_runtime_identity"] = dict(malformed_identity)
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "windows_installed_smoke.json"
        path.write_text(json.dumps(evidence), encoding="utf-8")
        results = validate_windows_release_evidence(path)
    result_by_name = {result.name: result for result in results}
    result = result_by_name["windows_evidence_provenance_isolation"]
    if result.classification != "release_blocker":
        return ["不正なD4 Pocket runtime identityをWindows installed evidenceとして受け入れた"]
    if "D4 Pocket runtime identity" not in result.reason:
        return ["不正なD4 Pocket runtime identityを具体的に拒否しなかった"]
    return []


def test_windows_release_evidence_rejects_runtime_or_config_outside_isolated_profile() -> list[str]:
    errors: list[str] = []
    for field, value, label in (
        (
            "isolated_runtime_dir",
            r"C:\Users\owner\AppData\Local\GUI-Shell\broker\desktop",
            "起動器runtime",
        ),
        (
            "isolated_config_dir",
            r"C:\Users\owner\AppData\Local\GUI-Shell\installed-runs\run-20260605T000000Z-a1b2c3d4\runtime\config",
            "stage collector scratch config directory",
        ),
        (
            "first_run.config_path",
            r"C:\ProgramData\GUI-Shell\config\gui_shell.json",
            "初回config",
        ),
    ):
        evidence = _valid_windows_installed_evidence()
        if field == "first_run.config_path":
            evidence["first_run"]["config_path"] = value
        elif field == "isolated_runtime_dir":
            evidence["provenance"]["isolation"][field] = value
            evidence["first_run"]["launcher_runtime_dir"] = value
            evidence["first_run"]["broker_endpoint_file"] = value + r"\broker_session.json"
        else:
            evidence["provenance"]["isolation"][field] = value
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "windows_installed_smoke.json"
            path.write_text(json.dumps(evidence), encoding="utf-8")
            results = validate_windows_release_evidence(path)
        result_by_name = {result.name: result for result in results}
        if result_by_name["windows_evidence_provenance_isolation"].classification != "release_blocker":
            errors.append(f"isolated LOCALAPPDATA外の{label}を正式evidenceとして受け入れた")
    return errors


def test_windows_release_evidence_validator_rejects_unmeasured_or_synthetic_evidence() -> list[str]:
    bad = _valid_windows_installed_evidence()
    bad["evidence_source"]["manual_confirmation"] = True
    bad["first_run"]["main_window_handle"] = 0
    bad["first_run"]["visible_surfaces_evidence"] = {"source": "manual", "path": ""}
    bad["first_run"]["config_json_valid"] = False
    bad["first_run"]["audit_write_probe"]["read"] = False
    bad["first_run"]["broker_mediated_launch"] = False
    bad["first_run"]["python_commands_visible_after_scrub"] = ["python"]
    bad["first_run"]["visible_surfaces_complete"] = False
    bad["setup_doctor"]["evidence_source"]["synthetic"] = True
    bad["setup_doctor"]["checks"] = bad["setup_doctor"]["checks"][:1]
    bad["broker"]["evidence_source"]["synthetic"] = True
    bad["broker"]["restricted_loopback_bind"] = False
    bad["broker"]["restart_replay_rejected"] = False
    bad["broker"]["session_file_removed_after_collection"] = False
    bad["broker"]["python_runtime_required_for_authority"] = True
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "windows_installed_smoke.json"
        path.write_text(json.dumps(bad), encoding="utf-8")
        results = validate_windows_release_evidence(path)
    result_by_name = {result.name: result for result in results}
    errors = []
    if result_by_name["windows_installer_first_run_smoke"].classification != "release_blocker":
        errors.append("Windows first-run evidence validatorがunmeasured/manual evidenceを受け入れた")
    if result_by_name["windows_setup_doctor_smoke"].classification != "release_blocker":
        errors.append("Windows Setup Doctorのevidence validatorがsyntheticまたはshallowのevidenceを受け入れた")
    if result_by_name["windows_broker_installed_smoke"].classification != "release_blocker":
        errors.append("Windows brokerのevidence validatorがsynthetic、replay-unsafe、またはPython-requiredのevidenceを受け入れた")
    return errors


def test_windows_release_evidence_validator_rejects_broker_top_level_unmeasured_declarations() -> list[str]:
    bad = _valid_windows_installed_evidence()
    bad["broker"]["python_runtime_required_for_authority"] = False
    bad["broker"]["flutter_rust_ffi_authority_bridge"] = False
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "windows_installed_smoke.json"
        path.write_text(json.dumps(bad), encoding="utf-8")
        results = validate_windows_release_evidence(path)
    result_by_name = {result.name: result for result in results}
    if result_by_name["windows_broker_installed_smoke"].classification != "release_blocker":
        return ["Windows broker validatorがtop-levelのunmeasured authority declarationを受け入れた"]
    return []


def test_windows_broker_evidence_requires_native_owner_denial_for_agent_task_grants() -> list[str]:
    errors = []
    for field, error_code in (
        ("normal_ipc_agent_task_workspace_permission_denied", "agent_task_workspace_permission_error_code"),
        ("normal_ipc_agent_task_owner_approval_denied", "agent_task_owner_approval_error_code"),
    ):
        bad = _valid_windows_installed_evidence()
        bad["broker"][field] = False
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "windows_installed_smoke.json"
            path.write_text(json.dumps(bad), encoding="utf-8")
            results = validate_windows_release_evidence(path)
        result_by_name = {result.name: result for result in results}
        if result_by_name["windows_broker_installed_smoke"].classification != "release_blocker":
            errors.append(f"Windows broker validatorが{field}=falseを受け入れた")

        bad = _valid_windows_installed_evidence()
        bad["broker"][error_code] = "accepted"
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "windows_installed_smoke.json"
            path.write_text(json.dumps(bad), encoding="utf-8")
            results = validate_windows_release_evidence(path)
        result_by_name = {result.name: result for result in results}
        if result_by_name["windows_broker_installed_smoke"].classification != "release_blocker":
            errors.append(f"Windows broker validatorが{error_code}の不一致を受け入れた")

    bad = _valid_windows_installed_evidence()
    bad["broker"]["evidence_source"]["collector_version"] = "5"
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "windows_installed_smoke.json"
        path.write_text(json.dumps(bad), encoding="utf-8")
        results = validate_windows_release_evidence(path)
    result_by_name = {result.name: result for result in results}
    if result_by_name["windows_broker_installed_smoke"].classification != "release_blocker":
        errors.append("Windows broker validatorが旧collector version 5を受け入れた")
    return errors


def test_windows_japanese_surface_labels() -> list[str]:
    from tooling.windows_release_evidence import _validate_surface_match_evidence, _contains_surface_label, SURFACE_NAMES
    errors = []
    for label, (name, identifier) in SURFACE_NAMES.items():
        for accepted in (name, name + "\n" + name, identifier):
            if not _contains_surface_label(accepted, label):
                errors.append(f"{label}の固定対応を拒否した")
        for rejected in (name + " Tab 1 of 13", "説明 " + name, identifier + ".extra"):
            if _contains_surface_label(rejected, label):
                errors.append(f"{label}を部分一致で誤認した")
    for use_identifier in (False, True):
        surface = _valid_windows_installed_evidence()["first_run"]["visible_surfaces_evidence"]
        nodes = {n["element_key"]: n for n in surface["diagnostic_tree"]["observed_elements"]}
        for label, match in surface["surface_matches"].items():
            name, identifier = SURFACE_NAMES[label]
            for target in (match, nodes[match["element_key"]]):
                target["name"] = name if not use_identifier else ""
                target["automation_id"] = identifier if use_identifier else ""
        errors.extend(_validate_surface_match_evidence(surface))
    return errors


def test_windows_surface_geometry_and_identity() -> list[str]:
    from tooling.windows_release_evidence import _validate_surface_match_evidence
    surface = _valid_windows_installed_evidence()["first_run"]["visible_surfaces_evidence"]
    if _validate_surface_match_evidence(surface):
        return ["完全な観測treeの正常surfaceを拒否した"]
    cases = {
        "観測方式": lambda s, ns: s["diagnostic_tree"].update(tree_view="raw"),
        "観測上限": lambda s, ns: s["diagnostic_tree"].update(capture_limit="max_10000_elements"),
        "画面外": lambda s, ns: ns[1]["bounding_rectangle"].update(x=2000),
        "非表示": lambda s, ns: ns[1].update(is_offscreen=True),
        "状態欠落": lambda s, ns: ns[1].pop("is_offscreen"),
        "座標欠落": lambda s, ns: ns[1].pop("bounding_rectangle"),
        "ゼロ面積": lambda s, ns: ns[1]["bounding_rectangle"].update(width=0),
        "非有限": lambda s, ns: ns[1]["bounding_rectangle"].update(x=float("nan")),
        "過大整数": lambda s, ns: ns[1]["bounding_rectangle"].update(x=10**400),
        "親領域外": lambda s, ns: ns[0]["bounding_rectangle"].update(height=40),
        "親非表示": lambda s, ns: ns[0].update(is_offscreen=True),
        "存在しない親": lambda s, ns: ns[1].update(parent_runtime_id="missing"),
        "循環": lambda s, ns: ns[1].update(parent_runtime_id=ns[1]["runtime_id"]),
        "重複ID": lambda s, ns: ns[2].update(runtime_id=ns[1]["runtime_id"]),
        "重複key": lambda s, ns: ns[2].update(element_key=ns[1]["element_key"]),
        "宣言の差替え": lambda s, ns: ns[1].update(name="別の要素"),
        "未観測match": lambda s, ns: ns.pop(),
        "件数不一致": lambda s, ns: s["diagnostic_tree"].update(observed_element_count=9),
        "edge欠落": lambda s, ns: s["diagnostic_tree"]["tree_edges"].pop(),
        "container流用": lambda s, ns: ns[1].update(is_native_container=True),
    }
    errors = []
    for name, mutate in cases.items():
        bad = json.loads(json.dumps(surface))
        mutate(bad, bad["diagnostic_tree"]["observed_elements"])
        if not _validate_surface_match_evidence(bad):
            errors.append(f"可視surfaceの{name}を受理した")
    if sys.platform == "win32":
        cases_for_collector = [{"name": "正常", "nodes": surface["diagnostic_tree"]["observed_elements"], "expected": True}]
        for name in ("画面外", "非表示", "状態欠落", "座標欠落", "ゼロ面積", "親領域外", "親非表示", "存在しない親", "循環", "重複ID", "container流用"):
            bad = json.loads(json.dumps(surface))
            nodes = bad["diagnostic_tree"]["observed_elements"]
            cases[name](bad, nodes)
            cases_for_collector.append({"name": name, "nodes": nodes, "expected": False})
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "cases.json"
            path.write_text(json.dumps(cases_for_collector, ensure_ascii=False), encoding="utf-8")
            import base64
            def ps_literal(value):
                return "'" + str(value).replace("'", "''") + "'"
            # Windows PowerShell 5.1でも日本語sourceをUTF-8として読む試験用入口。
            script = ROOT / "tooling/conformance_tests/windows_surface_geometry.ps1"
            command = ("& ([scriptblock]::Create([IO.File]::ReadAllText(" + ps_literal(script)
                       + ", [Text.Encoding]::UTF8))) -CollectorPath "
                       + ps_literal(INSTALLER / "windows/collect_installed_smoke.ps1")
                       + " -CasesJson " + ps_literal(path))
            result = subprocess.run(["powershell.exe", "-NoProfile", "-EncodedCommand",
                base64.b64encode(command.encode("utf-16-le")).decode("ascii")],
                capture_output=True, timeout=30)
            if result.returncode != 0:
                errors.append("実collectorの可視判定回帰試験が失敗: " + result.stderr.decode(errors="replace"))
    return errors


def test_windows_release_evidence_validator_rejects_missing_surface_matches() -> list[str]:
    bad = _valid_windows_installed_evidence()
    bad["first_run"]["visible_surfaces_evidence"].pop("surface_matches")
    bad["first_run"]["visible_surfaces_evidence"].pop("aggregate_surface_shortcut_detected")
    bad["first_run"]["visible_surfaces_evidence"].pop("surface_match_requirements_met")
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "windows_installed_smoke.json"
        path.write_text(json.dumps(bad), encoding="utf-8")
        results = validate_windows_release_evidence(path)
    result_by_name = {result.name: result for result in results}
    if result_by_name["windows_installer_first_run_smoke"].classification != "release_blocker":
        return ["Windows first-run evidence validatorがper-surface UIAutomation matchの欠落を受け入れた"]
    return []


def test_windows_release_evidence_validator_rejects_screenshot_surface_source() -> list[str]:
    bad = _valid_windows_installed_evidence()
    bad["first_run"]["visible_surfaces_evidence"]["source"] = "screenshot"
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "windows_installed_smoke.json"
        path.write_text(json.dumps(bad), encoding="utf-8")
        results = validate_windows_release_evidence(path)
    result_by_name = {result.name: result for result in results}
    if result_by_name["windows_installer_first_run_smoke"].classification != "release_blocker":
        return ["Windows first-run evidence validatorがscreenshotを厳密なvisible-surface sourceとして受け入れた"]
    return []


def test_windows_release_evidence_validator_rejects_flutter_build_registry_as_visibility() -> list[str]:
    for source in ("flutter_semantics_runtime_export", "uiautomation", "accessibility_tree"):
        bad = _valid_windows_installed_evidence()
        surface = bad["first_run"]["visible_surfaces_evidence"]
        surface["source"] = source
        surface["diagnostic_tree"]["mode"] = "flutter_dart_surface_semantics_runtime_export"
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "windows_installed_smoke.json"
            path.write_text(json.dumps(bad), encoding="utf-8")
            results = validate_windows_release_evidence(path)
        result_by_name = {result.name: result for result in results}
        if result_by_name["windows_installer_first_run_smoke"].classification != "release_blocker":
            return [f"Windows first-run evidence validatorがbuild registryを可視証拠として受け入れた: {source}"]
    return []


def test_windows_release_evidence_validator_rejects_aggregate_surface_root_match() -> list[str]:
    bad = _valid_windows_installed_evidence()
    aggregate_match = {
        "matched": True,
        "name": "GUI Shell Dashboard NavigationRail Runtime Status Invariant Status",
        "automation_id": "",
        "control_type": "ControlType.Window",
        "class_name": "FlutterView",
        "framework_id": "Win32",
        "element_key": "root",
        "is_root": True,
        "is_native_container": True,
        "surfaces_present": ["Dashboard", "NavigationRail", "Runtime Status", "Invariant Status"],
    }
    bad["first_run"]["visible_surfaces_evidence"]["surface_matches"] = {
        label: copy.deepcopy(aggregate_match)
        for label in ["Dashboard", "NavigationRail", "Runtime Status", "Invariant Status"]
    }
    bad["first_run"]["visible_surfaces_evidence"]["aggregate_surface_shortcut_detected"] = True
    bad["first_run"]["visible_surfaces_evidence"]["surface_match_requirements_met"] = False
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "windows_installed_smoke.json"
        path.write_text(json.dumps(bad), encoding="utf-8")
        results = validate_windows_release_evidence(path)
    result_by_name = {result.name: result for result in results}
    if result_by_name["windows_installer_first_run_smoke"].classification != "release_blocker":
        return ["Windows first-run evidence validatorが1つのaggregate root automation elementを受け入れた"]
    return []


def test_flutter_setup_doctor_has_no_filesystem_export_path() -> list[str]:
    export = DESKTOP_FLUTTER / "lib" / "services" / "setup_doctor_export.dart"
    main = DESKTOP_FLUTTER / "lib" / "main.dart"
    collector = INSTALLER / "windows" / "collect_installed_smoke.ps1"
    external_probe = INSTALLER / "windows" / "collect_setup_doctor.ps1"
    main_text = main.read_text(encoding="utf-8")
    collector_text = collector.read_text(encoding="utf-8")
    shell_core_text = (DESKTOP_FLUTTER / "lib" / "services" / "shell_core_client.dart").read_text(encoding="utf-8")
    errors = []
    if export.exists():
        errors.append("Flutter内Setup Doctor filesystem export helperが残っている")
    for path in (DESKTOP_FLUTTER / "lib").rglob("*.dart"):
        source = path.read_text(encoding="utf-8")
        if "dart:io" in source:
            errors.append(f"Flutter production sourceにdart:ioが残っている: {path.relative_to(ROOT)}")
    if "setup_doctor_export" in main_text or "writeSetupDoctorProductExportIfRequested" in main_text:
        errors.append("desktop mainがFlutter Setup Doctor filesystem exportを呼び出している")
    for token in ("GUI_SHELL_SETUP_DOCTOR_EXPORT_JSON", "GUI_SHELL_SETUP_DOCTOR_CONTEXT_JSON", "setup_doctor_context"):
        if token in collector_text:
            errors.append(f"installed smoke collectorがFlutter Setup Doctor export経路を使っている: {token}")
    if "collect_setup_doctor.ps1" in collector_text:
        errors.append("installed smoke collectorがstage scratch pathを使う外部Setup Doctor probeを実行している")
    if "config_created = $configCreated" not in collector_text or "$configCreated = $false" not in collector_text:
        errors.append("installed smoke collectorが製品config export未観測を未成立として記録しない")
    for token in (
        'Join-Path $brokerStoreDir "first_run_configuration.json"',
        'config_path = $firstRunConfigurationPath',
        'New-EvidenceFileRecord -Kind "first_run_configuration" -Path $firstRunConfigurationPath',
        '"first_run.config_audit"',
        'configAuditMatched = ($configFileObserved -and $configJsonValid -and $configAuditAcceptedMatchCount -eq 1)',
        'config path/hashがAuditと一致しない',
    ):
        if token not in collector_text and token not in (ROOT / "tooling" / "windows_release_evidence.py").read_text(encoding="utf-8"):
            errors.append(f"installed smoke collectorの初回config/Audit証拠契約が欠落: {token}")
    external_probe_text = external_probe.read_text(encoding="utf-8")
    if (
        "formal_product_evidence = $false" not in external_probe_text
        or "product_generated = $false" not in external_probe_text
        or "collector_derives_checks = $true" not in external_probe_text
        or "synthetic = $false" not in external_probe_text
        or "setup_doctor.ran_from_installed_app_path" not in external_probe_text
    ):
        errors.append("独立Setup Doctor probeの外部由来・非製品生成provenanceが明確でない")
    if '-Status "warning" `' not in external_probe_text or "アプリ自身が生成する環境診断の製品出力は観測していません" not in external_probe_text:
        errors.append("外部probeが観測していないproduct Setup Doctorを合格扱いする")
    first_run_config_check = external_probe_text.split('-CheckId "first_run.config_created"', 1)
    if len(first_run_config_check) != 2 or '-Status "warning"' not in first_run_config_check[1].split("  New-DoctorCheck", 1)[0]:
        errors.append("外部probeが起動中の生成を観測していないconfigを初回生成合格として扱う")
    for token in (
        "Setup Doctor報告取得",
        "_validatedSetupDoctorReport",
        "_unknownSetupDoctorReport",
        "setup_doctor_checks': setupDoctorReport['checks']",
    ):
        if token not in shell_core_text:
            errors.append(f"Flutter Setup DoctorがBroker reportを表示するread-only経路を持たない: {token}")
    for token in (
        'Join-Path $brokerStoreDir "setup_doctor_report.json"',
        "$setupDoctorAuditMatched",
        '"setup_doctor_report_exported"',
        "serialized_report_base64",
        "accepted_audit_event = $setupDoctorAuditEvent",
        'source_type = $(if ($setupDoctorAuditMatched) { "product_export" } else { "unsupported_claim" })',
    ):
        if token not in collector_text:
            errors.append(f"installed collectorがBroker Setup Doctor reportとAuditを照合しない: {token}")
    if "[string]$SetupDoctorJson" in collector_text:
        errors.append("installed smoke collectorが未検証Setup Doctor JSONをproduct evidenceへ取り込める")
    return errors


def test_setup_doctor_filesystem_probe_is_bounded_and_non_destructive() -> list[str]:
    external_probe = (INSTALLER / "windows" / "collect_setup_doctor.ps1").read_text(encoding="utf-8")
    installed_smoke = (INSTALLER / "windows" / "collect_installed_smoke.ps1").read_text(encoding="utf-8")
    errors = []
    if "[System.IO.FileMode]::CreateNew" not in external_probe or "[guid]::NewGuid()" not in external_probe:
        errors.append("Setup Doctor external probeが衝突しないexclusive temporary fileを使わない")
    if "[System.IO.FileMode]::CreateNew" not in installed_smoke or "[guid]::NewGuid()" not in installed_smoke:
        errors.append("installed smoke audit probeが衝突しないexclusive temporary fileを使わない")
    if "New-Item -ItemType Directory -Force -Path $Path" in external_probe:
        errors.append("Setup Doctor external probeが未存在のaudit directoryを作って成功を捏造する")
    if '".gui-shell-setup-doctor-probe"' in external_probe or '".gui-shell-write-probe"' in installed_smoke:
        errors.append("audit probeが固定名fileを上書きまたは削除する可能性がある")
    return errors


def test_windows_stage_installer_powershell_boolean_grouping() -> list[str]:
    text = (INSTALLER / "windows" / "stage_installed_app.ps1").read_text(encoding="utf-8")
    errors = []
    if "Test-Path $InstallRoot -and" in text:
        errors.append("stage_installed_app.ps1が-andをTest-Pathのargumentとして渡している")
    if "if ((Test-Path $InstallRoot) -and" not in text:
        errors.append("stage_installed_app.ps1にgroup化されたTest-Path boolean conditionがない")
    return errors


def test_windows_stage_uses_terminal_free_native_launcher() -> list[str]:
    stage = (INSTALLER / "windows" / "stage_installed_app.ps1").read_text(encoding="utf-8")
    broker_smoke = (INSTALLER / "windows" / "collect_broker_smoke.ps1").read_text(encoding="utf-8")
    launcher = (ROOT / "native" / "rust_helper" / "src" / "desktop_launcher.rs").read_text(encoding="utf-8")
    broker_server = (ROOT / "native" / "rust_helper" / "src" / "broker" / "ipc_server.rs").read_text(encoding="utf-8")
    product_install = (ROOT / "native" / "rust_helper" / "src" / "broker" / "product_install.rs").read_text(encoding="utf-8")
    update_center = (ROOT / "native" / "rust_helper" / "src" / "broker" / "update_center.rs").read_text(encoding="utf-8")
    launcher_doc = (ROOT / "docs" / "specs" / "windows-desktop-launcher.md").read_text(encoding="utf-8")
    broker_channel_doc = (ROOT / "docs" / "specs" / "desktop-broker-channel.md").read_text(encoding="utf-8")
    regression_case_doc = (ROOT / "docs" / "specs" / "regression-case.md").read_text(encoding="utf-8")
    errors = []
    for token in ["[string]$DesktopLauncherExe", "gui_shell_desktop_launcher.exe", "launcher_exe =", "launcher_artifact_sha256", "launcher_runtime = [ordered]@{", "formal_runtime_proof = $false", "staging_user_identity = [ordered]@{", "salted_hash = $stagingUserIdentityHash"]:
        if token not in stage:
            errors.append(f"staged installがnative起動器を配置・hash結合しない: {token}")
    for token in (
        "[string]$ProductManifestJson",
        "manifest_version = 3",
        "identity_kind = $launcherRuntimeIdentityKind",
        "app_id = $productAppId",
        "audit_store_id = $productAuditStoreId",
        "Copy-Item -LiteralPath $productManifestPath",
        "embeddedAppIds[0] -cne $productAppId",
        'Join-Path $env:LOCALAPPDATA (Join-Path "D4Pocket\\apps"',
        'session_file = (Join-Path $launcherRuntimeDir "broker_session.json")',
        '$sessionFile = Join-Path $runtimeDir.FullName "broker_session.json"',
        "broker_session_file = $sessionFile",
    ):
        if token not in stage:
            errors.append(f"staged installがD4 Pocket runtime identityをManifest／起動器へ結合しない: {token}")
    if 'Join-Path $env:LOCALAPPDATA "GUI-Shell\\broker\\desktop"' not in stage:
        errors.append("staged manifestが起動器のper-user runtime rootを示さない")
    if 'Remove-Item -Force -Path $SessionFile' not in broker_smoke:
        errors.append("独立Broker smokeのscratch session file cleanup境界を確認できない")
    if 'broker_session_file = $sessionFile' not in stage or '$sessionFile = Join-Path $runtimeDir.FullName "broker_session.json"' not in stage:
        errors.append("Broker単体smokeが製品runtime session fileを触れないようscratch pathを分離していない")
    for token in [
        "run_loopback_server_cancellable",
        "BrokerCredentialRole::Normal",
        "authenticated_loopback_tcp",
        "configure_frontend_environment(",
        ".env_clear()",
        '"GUI_SHELL_BROKER_CHANNEL_PIPE"',
    ]:
        if token not in launcher:
            errors.append(f"Rust Desktop起動器の既存Broker境界が欠落: {token}")
    for token in [
        "runtime_directory_with_identity(",
        'option_env!("GUI_SHELL_PRODUCT_APP_ID")',
        'option_env!("GUI_SHELL_PRODUCT_AUDIT_STORE_ID")',
        '"D4Pocket".to_string()',
        '"stores".to_string()',
        "canonical.starts_with(&root)",
        "ensure_store_directory",
        "PRODUCT_RUNTIME_IDENTITY_INVALID",
        "product_runtime_identity_requires_both_generated_ids",
        "invalid_product_identity_is_rejected_before_creating_user_data",
        "exported_products_get_distinct_app_and_audit_store_directories",
        "product_runtime_directory_rejects_reparse_identity_parent",
        "broker_store_directory_rejects_junction_outside_runtime_root",
        "flutter_child_environment_is_allowlisted",
        "GUI_SHELL_PRODUCT_APP_ID",
        "GUI_SHELL_PRODUCT_AUDIT_STORE_ID",
    ]:
        if token not in launcher:
            errors.append(f"Rust Desktop起動器のユーザー保存先reparse境界が欠落: {token}")
    for token in ["D4 Pocket Desktop起動", "D4 Pocket Desktop終了", "Capability=desktop.launch", "RecoveryAction=", "LIVE_RUNTIME"]:
        if token not in broker_server:
            errors.append(f"Desktop起動器のBroker lifecycle監査対応が欠落: {token}")
    if "Command::new(&layout.broker_exe)" in launcher or "owner-session-file" in launcher:
        errors.append("Desktop起動器がBrokerを別権限経路・Owner資格で起動する")
    if any(token in stage for token in ["GUI-Shell.brokered.ps1", "GUI-Shell.brokered.cmd", "Start-Process", "powershell -ExecutionPolicy"]):
        errors.append("staged product rootがscript/terminal launcherを生成する")
    if "Owner資格を作成・読み込み・Flutterへ渡さない" not in launcher_doc:
        errors.append("Windows起動仕様にOwner資格境界がない")
    for token in (
        "GUI_SHELL_PRODUCT_APP_ID",
        "GUI_SHELL_PRODUCT_AUDIT_STORE_ID",
        "%LOCALAPPDATA%\\D4Pocket\\apps\\<App ID>\\stores\\<Audit store ID>",
        "旧runtimeからのcopy／migration／継承は行わない",
        "Developer専用の`tooling/export_windows_product.py`はSchema検証済みManifestから両IDをRust Release buildのcompile-time入力へ渡し",
        "このbuild補助はruntime Manifest消費やOwner承認済みproduction Export pathではない",
        "現行Developer専用Export build toolは識別子ごとに分離したCargo target directoryを使い",
        "compile-time identityはFlutter childへ渡さない",
    ):
        if token not in launcher_doc:
            errors.append(f"Windows起動仕様にExport runtime identity境界がない: {token}")
    for token in (
        "### 製品アンインストール",
        "Brokerは削除意図をdurable Auditへ確定した後、一回限りticketを起動器へ返す",
        "Credential、設定、Workspace、AppData内のAudit Storeは削除しない",
        "finalizer開始後の失敗・再起動では同ticketを再利用できず",
        "finalizer用の一時copy残存を含むcleanup確認は最終統合QAで扱う",
    ):
        if token not in launcher_doc:
            errors.append(f"Windows Uninstaller正本に必要な境界がない: {token}")
    uninstall_sources = launcher + broker_server + product_install + update_center
    for token in (
        "stage_uninstall_finalizer(",
        "run_product_uninstall_finalizer(",
        "begin_product_uninstall_finalization(",
        "remove_installed_product(",
        "uninstall_ticket_is_removed_before_the_flutter_response_is_serialized",
        "uninstall_owner_confirmation_names_fixed_root_and_preserved_user_data",
        "product_uninstall_requires_owner_and_consumes_durable_ticket_once",
        "uninstall_removes_only_fixed_product_and_matching_shortcut",
        "uninstall_conflicting_shortcut_preserves_product_root",
    ):
        if token not in uninstall_sources:
            errors.append(f"Windows Uninstallerの実装・正常／境界試験が欠落: {token}")
    for token in (
        "%LOCALAPPDATA%\\GUI-Shell\\broker\\desktop\\store",
        "%LOCALAPPDATA%\\D4Pocket\\apps\\<App ID>\\stores\\<Audit store ID>\\store",
    ):
        if token not in broker_channel_doc:
            errors.append(f"Broker channel仕様にidentity別の初回設定保存pathがない: {token}")
    for token in (
        "%LOCALAPPDATA%\\GUI-Shell\\broker\\desktop\\protected",
        "%LOCALAPPDATA%\\D4Pocket\\apps\\<App ID>\\stores\\<Audit store ID>\\protected",
    ):
        if token not in regression_case_doc:
            errors.append(f"Regression Case仕様にidentity別ProtectedStore pathがない: {token}")
    return errors


def test_windows_installed_smoke_preserves_trap_failure() -> list[str]:
    text = (INSTALLER / "windows" / "collect_installed_smoke.ps1").read_text(encoding="utf-8")
    errors = []
    if "trap {\n  $failure = $_" not in text:
        errors.append("collect_installed_smoke.ps1のtrapが元の失敗を保持していない")
    if "\n  throw\n" in text:
        errors.append("collect_installed_smoke.ps1がbare throwを使いdiagnostic causeを失う")
    if "throw $failure" not in text:
        errors.append("collect_installed_smoke.ps1がcaptureした失敗をrethrowしない")
    return errors


def test_windows_broker_smoke_keeps_full_duplex_response() -> list[str]:
    text = (INSTALLER / "windows" / "collect_broker_smoke.ps1").read_text(encoding="utf-8")
    errors = []
    if "Shutdown([System.Net.Sockets.SocketShutdown]::Send)" in text:
        errors.append("collect_broker_smoke.ps1がWindows応答前に送信側socketをshutdownしている")
    if "$raw = $reader.ReadLine()" not in text:
        errors.append("collect_broker_smoke.ps1がBrokerの改行区切り応答を1フレームとして読んでいない")
    if 'collector_version = "6"' not in text:
        errors.append("collect_broker_smoke.ps1のcollector versionがAgent Task権限否定検査を反映していない")
    if "$sessionFileCreated = Test-Path -LiteralPath $SessionFile" not in text:
        errors.append("collect_broker_smoke.ps1がcleanup前にendpoint file生成を観測していない")
    if "$sessionFileRemovedAfterCollection = !(Test-Path -LiteralPath $SessionFile)" not in text:
        errors.append("collect_broker_smoke.ps1がendpoint資格file削除後の状態を測定していない")
    if "session_file_removed_after_collection = $sessionFileRemovedAfterCollection" not in text:
        errors.append("collect_broker_smoke.ps1が資格file cleanupをevidenceへ結合していない")
    return errors


def test_windows_broker_smoke_checks_agent_task_grants_through_normal_ipc() -> list[str]:
    text = (INSTALLER / "windows" / "collect_broker_smoke.ps1").read_text(encoding="utf-8")
    errors = []
    for operation in ("AgentTaskWorkspacePermissionGrant", "AgentTaskOwnerApprovalGrant"):
        if f'-Operation "{operation}"' not in text:
            errors.append(f"collect_broker_smoke.ps1に{operation}の通常IPC probeがない")
    for field in (
        "normal_ipc_agent_task_workspace_permission_denied",
        "agent_task_workspace_permission_error_code",
        "normal_ipc_agent_task_owner_approval_denied",
        "agent_task_owner_approval_error_code",
    ):
        if f"{field} =" not in text:
            errors.append(f"collect_broker_smoke.ps1の結果に{field}がない")
    if text.count("desktop_native_owner_confirmation_required") < 2:
        errors.append("collect_broker_smoke.ps1が両grant操作のnative Owner確認拒否codeを照合しない")
    return errors


def test_windows_installed_smoke_reads_json_as_utf8() -> list[str]:
    text = (INSTALLER / "windows" / "collect_installed_smoke.ps1").read_text(encoding="utf-8")
    errors = []
    if "function Read-Utf8Json" not in text:
        errors.append("collect_installed_smoke.ps1にUTF-8 JSON readerがない")
    if "[System.IO.File]::ReadAllText($resolved.Path, [System.Text.Encoding]::UTF8)" not in text:
        errors.append("collect_installed_smoke.ps1のJSON readerがUTF-8明示読取りではない")
    if 'collector_version = "17"' not in text:
        errors.append("collect_installed_smoke.ps1の版識別子がruntime identity-aware installed evidenceを表さない")
    for token in (
        "function Collect-SetupDoctorOperatorReadability",
        "visible_uia_element_pointer_click",
        "operator_readability_proof",
        'New-EvidenceFileRecord -Kind "setup_doctor_operator_readability"',
        "Get-SetupDoctorVisibleSample",
        "WindowRootHandleAt",
        "WindowProcessIdAt",
        "NativeWindowHandle",
        "window_runtime_id = $state.root_runtime_id",
    ):
        if token not in text:
            errors.append(f"collect_installed_smoke.ps1に実画面可読性証拠の境界がない: {token}")
    return errors


def test_windows_installed_smoke_uses_launcher_owned_runtime() -> list[str]:
    text = (INSTALLER / "windows" / "collect_installed_smoke.ps1").read_text(encoding="utf-8")
    errors = []
    required = [
        "[string]$DesktopLauncherExe",
        "Start-Process -FilePath $launcher.Path -PassThru",
        "Find-InstalledFrontendProcess -LauncherProcess $launcherProcess -ExpectedPath $exe.Path",
        "Get-VerifiedInstalledFrontendChildren",
        "-ExpectedSha256 $hash",
        "Get-FileHash -LiteralPath $child.ExecutablePath -Algorithm SHA256",
        "$candidate.SessionId -ne $launcherSessionId",
        "$candidate.StartTime -lt $launcherStartTime",
        'process_identity = [ordered]@{',
        'method = "direct_parent_pid_and_sha256"',
        "function Request-InstalledFrontendExit",
        "if ($Frontend.HasExited) { return $false }",
        '[System.Windows.Automation.ControlType]::MenuItem',
        '$item.Current.Name -eq "終了"',
        "frontend_exit_menu_invoked = $frontendExitMenuInvoked",
        "same_windows_session = $true",
        "started_after_launcher = $true",
        'SetEnvironmentVariable("LOCALAPPDATA", $localAppDataRoot, "Process")',
        'Join-Path $LocalAppDataRoot "GUI-Shell\\broker\\desktop"',
        "function Resolve-InstalledRuntimeIdentity",
        '"D4Pocket\\apps\\$appId\\stores\\$auditStoreId"',
        'Get-TaggedSha256 -Path $manifestPath',
        'New-EvidenceFileRecord -Kind "product_manifest"',
        'runtime_identity = [ordered]@{',
        '"first_run.launcher_runtime_identity" = [ordered]@{ source_type = "static_assertion"; evidence_class = "CONFIG"; formal_release_input = $true }',
        "isolated_runtime_dir = $brokerRuntimeRoot",
        "D4 Pocket Desktop起動",
        "D4 Pocket Desktop終了",
        "broker_health_request = [ordered]@{",
        "accepted_event_count = $normalBrokerHealthEventCount",
        "first_audit_event_id = $normalBrokerHealthFirstEventId",
        "caller_process_attributed = $false",
        "client_response_receipt_observed = $false",
        "profile_identity_isolated_from_staging_user = $separateWindowsProfileVerified",
    ]
    for token in required:
        if token not in text:
            errors.append(f"installed smoke collectorにRust起動器・実runtimeの観測tokenがない: {token}")
    forbidden = [
        "Start-SmokeBroker",
        "Start-Process -FilePath $exe",
        "$env:GUI_SHELL_BROKER_ENDPOINT_JSON =",
        "[string]$BrokerHelperExe",
    ]
    for token in forbidden:
        if token in text:
            errors.append(f"installed smoke collectorが直接Flutter起動または独立Broker経路を残している: {token}")
    if 'New-EvidenceFileRecord -Kind "broker_lifecycle_audit" -Path $brokerAuditPath' not in text:
        errors.append("installed smoke collectorがBroker lifecycle Audit fileをevidence bundleへ結合しない")
    for token in [
        'operation -eq "health"',
        'decision -eq "accepted"',
        'reason -eq "health status returned"',
        'evidence_source -eq "LIVE_RUNTIME"',
        'throw "Broker lifecycle Auditに不正なJSON行があります。"',
    ]:
        if token not in text:
            errors.append(f"installed smoke collectorがauthenticated health Auditを厳格に観測しない: {token}")
    return errors


def test_windows_installed_smoke_exit_matches_native_tray_contract() -> list[str]:
    collector = (INSTALLER / "windows" / "collect_installed_smoke.ps1").read_text(encoding="utf-8")
    tray = (ROOT / "apps" / "desktop_flutter" / "windows" / "runner" / "tray_controller.cpp").read_text(encoding="utf-8")
    window = (ROOT / "apps" / "desktop_flutter" / "windows" / "runner" / "flutter_window.cpp").read_text(encoding="utf-8")
    errors = []
    for token in (
        'constexpr UINT kTrayCallbackMessage = WM_APP + 41;',
        'if (lparam == WM_RBUTTONUP)',
        r'AppendMenuW(menu, MF_STRING, kTrayExitCommand, L"\u7d42\u4e86");',
        'if (command == kTrayExitCommand) RequestExit();',
    ):
        if token not in tray:
            errors.append(f"native tray exit contractに必要な実装がない: {token}")
    for token in (
        'if (message == WM_CLOSE && tray_controller_ &&',
        '!tray_controller_->ExitRequested()',
        'ShowWindow(hwnd, SW_HIDE);',
    ):
        if token not in window:
            errors.append(f"Desktop WM_CLOSE hide contractに必要な実装がない: {token}")
    for token in (
        '[uint32]0x8029',
        '[IntPtr]0x0205',
        '$item.Current.ProcessId -eq $Frontend.Id',
        '[System.Windows.Automation.TreeWalker]::ControlViewWalker',
        '$topLevelProcessId = [int]$topLevel.Current.ProcessId',
        'if ($topLevelProcessId -eq $Frontend.Id)',
    ):
        if token not in collector:
            errors.append(f"installed collectorがnative tray exit callbackを正確に操作しない: {token}")
    return errors


def test_windows_installed_smoke_uses_bounded_control_uia_tree() -> list[str]:
    text = (INSTALLER / "windows" / "collect_installed_smoke.ps1").read_text(encoding="utf-8")
    errors = []
    required_tokens = [
        "function Get-ControlViewDescendants",
        "$walker = [System.Windows.Automation.TreeWalker]::ControlViewWalker",
        "Get-ControlViewDescendants -RootElement $RootElement",
        "Get-ControlViewDescendants -RootElement $window",
        "[int]$MaximumElements = 10000",
        "capture_limit = $(if ($script:surfaceTreeCaptureLimitReached)",
        'tree_view = "control"',
    ]
    for token in required_tokens:
        if token not in text:
            errors.append(f"collect_installed_smoke.ps1がbounded Control View UIAutomation tokenを欠いている: {token}")
    if ".FindAll(" in text:
        errors.append("collect_installed_smoke.ps1がtree walkerを迂回するFindAll treeを使用している")
    if "::RawViewWalker" in text:
        errors.append("collect_installed_smoke.ps1が可視surfaceにRaw Viewだけを使用する")
    return errors


def test_windows_installed_smoke_automation_names_are_materialized() -> list[str]:
    text = (INSTALLER / "windows" / "collect_installed_smoke.ps1").read_text(encoding="utf-8")
    errors = []
    if "automation_names = @($names | Select-Object" in text:
        errors.append("collect_installed_smoke.ps1がUIAutomation nameをJSON evidenceへ直接pipeline処理する")
    if "$automationNames = New-Object System.Collections.Generic.List[string]" not in text:
        errors.append("collect_installed_smoke.ps1がJSON evidence化の前にautomation nameをmaterializeしない")
    if "automation_names = @($automationNameValues)" not in text:
        errors.append("collect_installed_smoke.ps1がmaterialize済みのautomation name一覧をserializeしない")
    return errors


def test_windows_installed_smoke_uia_properties_are_stringified() -> list[str]:
    text = (INSTALLER / "windows" / "collect_installed_smoke.ps1").read_text(encoding="utf-8")
    errors = []
    direct_tokens = [
        "$element.Current.Name",
        "$element.Current.AutomationId",
        "$element.Current.ControlType.ProgrammaticName",
        "$window.Current.Name",
    ]
    for token in direct_tokens:
        if token in text:
            errors.append(f"collect_installed_smoke.ps1がevidence projectionでraw UIAutomation propertyを使う: {token}")
    if "$rootWindowTitle = \"\"" not in text:
        errors.append("collect_installed_smoke.ps1がroot window titleをmaterializeしない")
    if "window_title = $rootWindowTitle" not in text:
        errors.append("collect_installed_smoke.ps1がmaterialize済みのroot window titleをserializeしない")
    for token in [
        "$windowFound = [bool]($observedElements.Count -gt 0)",
        "$automationNameValues = @($automationNames.ToArray())",
        "$observedElementValues = @($observedElements.ToArray())",
        "$evidenceBundleFileValues = @($evidenceBundleFiles.ToArray())",
        "window_found = $windowFound",
        "observed_elements = @($observedElementValues)",
        "evidence_bundle_files = @($evidenceBundleFileValues)",
    ]:
        if token not in text:
            errors.append(f"collect_installed_smoke.ps1にmaterialize済みUIAutomation evidence tokenがない: {token}")
    for token in ["surface_semantics_export.json", "$surfaceSemanticsPath"]:
        if token in text:
            errors.append(f"collect_installed_smoke.ps1がFlutter内部Semantics fileへ依存する: {token}")
    if "$env:GUI_SHELL_SURFACE_SEMANTICS_EXPORT_JSON =" in text:
        errors.append("collect_installed_smoke.ps1がFlutterへ内部Semantics file出力先を渡す")
    if "Collect-VisibleSurfaces `" not in text:
        errors.append("collect_installed_smoke.ps1が外部UIAutomation treeを収集しない")
    return errors


def test_windows_audit_anchor_proof_collector_is_connected() -> list[str]:
    collector = INSTALLER / "windows" / "collect_audit_anchor_proof.ps1"
    installed_smoke = INSTALLER / "windows" / "collect_installed_smoke.ps1"
    errors: list[str] = []
    if not collector.exists():
        return ["collect_audit_anchor_proof.ps1が存在しない"]
    collector_text = collector.read_text(encoding="utf-8")
    installed_text = installed_smoke.read_text(encoding="utf-8")
    for token in [
        "audit_anchor_external_tamper_evidence.json",
        "key_anchor_log_same_user_rewrite_mitigated",
        "windows_acl_verified",
        "dpapi_verified",
        "external_anchor_verified",
        "signed_evidence_verified",
        "administrator_root_resistance_claimed = $false",
        "source_kind = $sourceKind",
        "sha256_scope = \"probe_material_without_self_reference\"",
        "[System.Text.UTF8Encoding]::new($false)",
    ]:
        if token not in collector_text:
            errors.append(f"audit anchor proof collectorにtokenがない: {token}")
    for token in [
        "[string]$AuditAnchorEvidenceJson",
        "audit_anchor_external_tamper_evidence",
        "field_provenance\"][\"audit_anchor.external_tamper_evidence",
        "New-EvidenceFileRecord -Kind \"audit_anchor_external_tamper_evidence\"",
    ]:
        if token not in installed_text:
            errors.append(f"installed smoke collectorにaudit anchor integration tokenがない: {token}")
    if sys.platform == "win32":
        measured = subprocess.run(
            [sys.executable, "-m", "unittest", "tooling.conformance_tests.test_windows_anchor_collector"],
            cwd=ROOT, capture_output=True, timeout=150,
        )
        if measured.returncode != 0:
            errors.append("Windows監査アンカー収集器の実行回帰が失敗: " + measured.stderr.decode(errors="replace"))
    return errors


def test_validate_all_subprocess_start_failure_is_structured() -> list[str]:
    step = ValidationStep(
        "missing_executable_probe",
        ["gui-shell-definitely-missing-validator-command"],
        ROOT,
    )
    result = run_step(step, strict_release=True, desktop_platform="windows")
    errors = []
    if result.get("status") != "not_run":
        errors.append("validate_allのrun_stepがsubprocess起動失敗でnot_runを返さなかった")
    if result.get("classification") != "release_blocker":
        errors.append("validate_allのrun_stepがsubprocess起動失敗をrelease_blockerに分類しなかった")
    if "FileNotFoundError" not in result.get("stderr", ""):
        errors.append("validate_allのrun_stepがsubprocess起動失敗のstack traceを保持しなかった")
    return errors


def test_validate_all_strict_release_runs_release_gate_strict_scan() -> list[str]:
    step = ValidationStep("release_gate_check", python_step("tooling/release_gate_check.py"), ROOT)
    result = run_step(step, strict_release=True, desktop_platform="windows")
    errors = []
    if "--strict-release" not in result.get("command", ""):
        errors.append("validate_allのstrict Windows releaseが--strict-releaseをrelease_gate_checkへ渡さなかった")
    if "--release-track windows_v1" not in result.get("command", ""):
        errors.append("validate_allのstrict Windows releaseがWindows 1.0 trackを指定しなかった")
    if result.get("status") != "failed":
        errors.append("activeなrelease blockerが残る間、validate_allのstrict release gate scanは失敗しなければならない")
    if result.get("classification") != "release_blocker":
        errors.append("validate_allのstrict release gate scanがrelease_blockerに分類されていない")
    if "strict release gate が未解決の有効な structured release blocker を検出した" not in result.get("reason", ""):
        errors.append("validate_allのstrict release gate scanが構造化されたrelease_blocker reasonを報告しなかった")
    return errors


def test_release_blocker_registry_controls_strict_release() -> list[str]:
    registry = ROOT / "release_blockers.registry.json"
    if not registry.exists():
        return ["release_blockers.registry.jsonが存在しない"]
    data = json.loads(registry.read_text(encoding="utf-8"))
    blockers = data.get("blockers")
    if not isinstance(blockers, list):
        return ["release blocker registryにblocker一覧がない"]
    errors = []
    active = [
        blocker
        for blocker in blockers
        if isinstance(blocker, dict)
        and blocker.get("active") is True
        and blocker.get("status") == "unresolved"
    ]
    if not active:
        errors.append("release blocker registryにactiveかunresolvedのblockerがない")
    for index, blocker in enumerate(blockers):
        if not isinstance(blocker, dict):
            continue
        errors.extend(blocker_cause_category_errors(blocker))
        category = blocker.get("cause_category")
        if not isinstance(category, str) or category not in BLOCKER_CAUSE_CATEGORIES:
            errors.append(f"blocker {blocker.get('name', index)}にrev3原因分類がない")
    if not blocker_cause_category_errors({"name": "missing-cause-category"}):
        errors.append("原因分類のないblockerをvalidatorが受理する")
    if not blocker_cause_category_errors(
        {"name": "unknown-cause-category", "cause_category": "owner_wait"}
    ):
        errors.append("未知の原因分類をvalidatorが受理する")
    if blocker_cause_category_errors(
        {"name": "valid-cause-category", "cause_category": "technical_blocker"}
    ):
        errors.append("有効なrev3原因分類をvalidatorが拒否する")
    for index, blocker in enumerate(blockers):
        if isinstance(blocker, dict):
            errors.extend(blocker_scope_errors(blocker))
            if not isinstance(blocker.get("blocks_windows_technical_complete"), bool):
                errors.append(f"blocker {blocker.get('name', index)}のTechnical Complete scopeがbooleanでない")
    if blocker_scope_errors(
        {
            "name": "valid-scope",
            "release_tracks": ["windows_v1"],
            "blocks_windows_technical_complete": True,
        }
    ):
        errors.append("有効なrelease track scopeをvalidatorが拒否する")
    if not blocker_scope_errors(
        {
            "name": "unknown-scope",
            "release_tracks": ["windows_mobile"],
            "blocks_windows_technical_complete": False,
        }
    ):
        errors.append("未知のrelease trackをvalidatorが受理する")
    if not blocker_scope_errors(
        {
            "name": "invalid-technical-complete-flag",
            "release_tracks": ["windows_v1"],
            "blocks_windows_technical_complete": "false",
        }
    ):
        errors.append("booleanでないTechnical Complete scopeをvalidatorが受理する")
    windows_release_names = {item["name"] for item in unresolved_active_blockers("windows_v1")}
    mobile_release_names = {item["name"] for item in unresolved_active_blockers("mobile")}
    windows_technical_names = {
        item["name"] for item in unresolved_windows_technical_complete_blockers()
    }
    mobile_only = {
        "rev2_mobile_device_evidence",
        "rev2_mobile_distribution",
        "rev2_mobile_flutter_native_device_link_boundary",
    }
    if windows_release_names & mobile_only:
        errors.append("Mobile専用blockerがWindows 1.0 release判定へ混入する")
    if not mobile_only.issubset(mobile_release_names):
        errors.append("Mobile専用blockerがMobile release scopeから欠落している")
    if not {
        "comprehensive_extension_rev1_completion",
        "windows_installer_first_run_smoke",
    }.issubset(windows_technical_names):
        errors.append("Windows Technical Complete gateから技術／Windows証拠blockerが欠落している")
    if windows_technical_names & {
        "owner_go",
        "audit_anchor_external_tamper_evidence_proof",
        "windows_distribution_identity",
        *mobile_only,
    }:
        errors.append("Owner／production identity／Mobile blockerがWindows Technical Complete gateへ混入する")
    if RELEASE_TRACKS != {"windows_v1", "mobile", "non_windows"}:
        errors.append("release gateのrev3 platform track定義が想定と異なる")
    for blocker in active:
        if blocker.get("classification") != "release_blocker":
            errors.append(f"active blocker {blocker.get('name')} がrelease_blockerに分類されていない")
        if blocker.get("blocks_release") is not True:
            errors.append(f"active blocker {blocker.get('name')} がreleaseを阻止していない")
    release_gate = (ROOT / "tooling" / "release_gate_check.py").read_text(encoding="utf-8")
    for token in [
        "RELEASE_BLOCKERS_REGISTRY",
        "unresolved_active_blockers",
        "strict releaseのactive blockerが未解決",
        "--release-track",
        "blocks_windows_technical_complete",
    ]:
        if token not in release_gate:
            errors.append(f"release_gate_check.pyに構造化registry tokenがない: {token}")
    if '"release_blocker" in combined' in release_gate:
        errors.append("release_gate_check.pyがraw release_blocker textをstrict release blockerとしてまだ使っている")
    return errors


def test_release_facing_docs_sync_release_blockers_to_registry() -> list[str]:
    errors = release_blocker_doc_sync_errors()
    registry_names = registry_blocker_names()
    for expected in [
        "windows_evidence_provenance_isolation",
        "windows_installer_first_run_smoke",
        "windows_setup_doctor_smoke",
        "windows_broker_installed_smoke",
        "audit_anchor_external_tamper_evidence_proof",
        "owner_go",
        "windows_distribution_identity",
    ]:
        if expected not in registry_names:
            errors.append(f"release blocker registryにcanonical blockerがない: {expected}")
    for relative in CURRENT_FACING_RELEASE_DOCS:
        if not (ROOT / relative).exists():
            errors.append(f"release-facing docがsync scanの対象にない: {relative}")
    return errors


def test_release_gate_scans_ipc_threat_model() -> list[str]:
    text = (ROOT / "tooling" / "release_gate_check.py").read_text(encoding="utf-8")
    if "docs/security/IPC_THREAT_MODEL.md" not in text:
        return ["release_gate_check.pyがIPC threat modelのrelease blockerをscanしていない"]
    return []


def test_packaging_portability_checker_exists() -> list[str]:
    checker = ROOT / "tooling" / "packaging_portability_check.py"
    validate_all = ROOT / "tooling" / "validate_all.py"
    errors = []
    if not checker.exists():
        errors.append("tooling/packaging_portability_check.pyが存在しない")
    else:
        text = checker.read_text(encoding="utf-8")
        for token in [
            "DEFAULT_SUBPROCESS_TIMEOUT_SECONDS = 120",
            "UTF8_GOVERNANCE_PATH_ALLOWLIST",
            "timeout=timeout_seconds",
            "subprocess.TimeoutExpired",
            "が次の秒数後にタイムアウト",
            "unzip",
            "LC_ALL",
            "tooling/manifest.py",
            "run_conformance_skeleton.py",
            "release_gate_check.py",
        ]:
            if token not in text:
                errors.append(f"packaging portability checkerにtokenがない: {token}")
    tracked_paths = [ROOT / path for path in subprocess.run(
        ["git", "ls-files"],
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        check=False,
    ).stdout.splitlines()]
    errors.extend(portable_path_errors([path for path in tracked_paths if path.exists()]))
    if "packaging_portability_check" not in validate_all.read_text(encoding="utf-8"):
        errors.append("validate_all.pyがpackaging portability checkを実行していない")
    return errors


def test_packaging_portability_subprocess_output_is_utf8_safe() -> list[str]:
    errors = run_check(
        ROOT,
        [
            sys.executable,
            "-c",
            "import sys; sys.stdout.buffer.write('日本語出力'.encode('utf-8')); sys.exit(1)",
        ],
    )
    if len(errors) != 1 or "日本語出力" not in errors[0]:
        return ["packaging portability checkerがUTF-8 subprocess出力を保持しない"]
    release_gate = (ROOT / "tooling" / "release_gate_check.py").read_text(encoding="utf-8")
    for token in (
        'env["PYTHONUTF8"] = "1"',
        'env["PYTHONIOENCODING"] = "utf-8"',
        'encoding="utf-8"',
        'errors="replace"',
    ):
        if token not in release_gate:
            return [f"release gateのmanifest subprocessがUTF-8境界を固定しない: {token}"]
    return []


def 端末契約の構造と禁止操作を検査する() -> list[str]:
    errors = []
    for name in ("device_link_invitation", "device_link_credential", "device_link_request"):
        schema = load_schema(name + ".schema.json")
        sample = load_contract_fixture(name + ".valid.json")
        errors.extend(validate_instance(sample, schema))
        for key in sample:
            missing = dict(sample)
            del missing[key]
            if not validate_instance(missing, schema):
                errors.append("端末契約が必須field欠落を受理: " + key)
        for change in ({"authority": "owner"}, {"HostID": "a" * 32 + "\n"}, {"端末ID": ""}, {"版": True}):
            if not validate_instance({**sample, **change}, schema):
                errors.append("端末契約が不正構造を受理: " + name)
        if name != "device_link_request":
            for port in (0, 65536, True):
                if not validate_instance({**sample, "port": port}, schema):
                    errors.append("端末契約がport境界を受理")
    schema = load_schema("device_link_request.schema.json")
    sample = load_contract_fixture("device_link_request.valid.json")
    rust_source = (RUST_HELPER / "src" / "broker" / "device_link.rs").read_text(encoding="utf-8")
    allowlist = re.search(r"pub\(crate\) const 許可操作: &\[&str\] = &\[(.*?)\];", rust_source, re.S)
    schema_operations = schema.get("properties", {}).get("操作", {}).get("enum", [])
    if allowlist is None:
        errors.append("Rust Device Link通常資格allowlistを抽出できない")
    else:
        rust_operations = re.findall(r'"([^"\\]+)"', allowlist.group(1))
        schema_runtime_operations = set(schema_operations) - {"端末結合"}
        if len(rust_operations) != len(set(rust_operations)) or set(rust_operations) != schema_runtime_operations:
            errors.append("Device Link Schemaの通常操作集合がRust Broker allowlistと一致しない")
    operations = {"端末結合": {}, "端末確認": {}, "端末離脱": {}, "実行系列挙": {},
                  "Agent一覧": {}, "作業領域一覧": {},
                  "実行系ライフサイクル状態": {"版": 1, "実行系ID": "local"},
                  "実行系資源観測": {"版": 1, "実行系ID": "local"},
                  "通知一覧": {"版": 1, "未読のみ": False, "上限": 64},
                  "全Runtime停止要求": {"版": 1},
                  "対話開始": {"実行系ID": "local"}, "対話送信": {"対話セッションID": "a" * 32, "入力": "こんにちは"},
                  "対話取得": {"要求ID": "b" * 32}, "対話中止": {"要求ID": "b" * 32}, "対話終了": {"対話セッションID": "a" * 32},
                  "対話履歴閲覧状態": {},
                  "対話履歴閲覧": {"approval_id": "a" * 32, "query": {"after": 0, "limit": 20, "filter": {"実行系ID": "local"}}}}
    for operation, payload in operations.items():
        errors.extend(validate_instance({**sample, "操作": operation, "内容": payload}, schema))
        if not validate_instance({**sample, "操作": operation, "内容": {**payload, "owner": True}}, schema):
            errors.append("端末操作が権限fieldを受理")
    for operation in ("対話承認", "対話承認待ち", "対話セッション一覧", "shutdown", "command_envelope", "端末招待", "端末失効"):
        if not validate_instance({**sample, "操作": operation, "内容": {}}, schema):
            errors.append("端末経路が禁止操作を受理: " + operation)
    if not validate_instance({**sample, "操作": "対話取得", "内容": {"対話セッションID": "a" * 32}}, schema):
        errors.append("端末要求が操作と内容の不一致を受理")
    invalid_payloads = (
        ("実行系ライフサイクル状態", {"版": 2, "実行系ID": "local"}),
        ("実行系資源観測", {"版": 1, "実行系ID": "../runtime"}),
        ("通知一覧", {"版": 1, "上限": 257}),
        ("作業領域一覧", {"実行系ID": "local"}),
        ("全Runtime停止要求", {"版": 1, "owner": True}),
        ("対話履歴閲覧状態", {"approval_id": "a" * 32}),
        ("対話履歴閲覧", {"approval_id": "a" * 32, "query": {"after": 0, "limit": 20, "filter": {"owner": "true"}}}),
    )
    for operation, payload in invalid_payloads:
        if not validate_instance({**sample, "操作": operation, "内容": payload}, schema):
            errors.append("端末操作が境界外payloadを受理: " + operation)
    return errors


def Mobile_native_Device_Link_channelを秘密非通過に制限する() -> list[str]:
    schema = load_schema("mobile_device_link_channel_request.schema.json")
    sample = load_contract_fixture("mobile_device_link_channel_request.valid.json")
    errors = validate_instance(sample, schema)
    valid = [
        {"version": 1, "method": "read_state"},
        {"version": 1, "method": "read_recovery_audit"},
        {"version": 1, "method": "pair"},
        {"version": 1, "method": "broker_request", "broker_operation": "Agent一覧", "payload": {}},
        {"version": 1, "method": "broker_request", "broker_operation": "作業領域一覧", "payload": {}},
        {"version": 1, "method": "broker_request", "broker_operation": "対話履歴閲覧", "payload": {"approval_id": "a" * 32, "query": {"after": 0, "limit": 50, "latest_per_request": True, "include_audit_context": True, "include_result_evidence": True, "include_content_receipt": True, "filter": {"実行系ID": "local"}}}},
        {"version": 1, "method": "disconnect"},
        {"version": 1, "method": "local_delete"},
    ]
    for candidate in valid:
        if validate_instance(candidate, schema):
            errors.append(
                "Mobile native channelの許可requestを拒否: "
                + candidate["method"]
                + "/"
                + str(candidate.get("broker_operation"))
            )
    invalid = [
        {**sample, "招待秘密": "e" * 64},
        {"version": 1, "method": "pair", "invitation": {"招待ID": "d" * 32}},
        {"version": 1, "method": "broker_request", "broker_operation": "端末招待", "payload": {}},
        {"version": 1, "method": "broker_request", "broker_operation": "対話セッション一覧", "payload": {}},
        {"version": 1, "method": "broker_request", "broker_operation": "対話送信", "payload": {"内容": {"端末秘密": "e" * 64}}},
        {"version": 1, "method": "broker_request", "broker_operation": "対話送信", "payload": {"内容": {"credential": {"value": "e" * 64}}}},
        {"version": 1, "method": "broker_request", "broker_operation": "対話送信", "payload": {"内容": {"authority": "owner"}}},
        {"version": 1, "method": "broker_request", "broker_operation": "Agent一覧", "payload": {"approval_id": "a" * 32}},
        {"version": 1, "method": "broker_request", "broker_operation": "作業領域一覧", "payload": {"実行系ID": "local"}},
        {"version": 1, "method": "broker_request", "broker_operation": "対話履歴閲覧", "payload": {"approval_id": "a" * 32, "query": {"after": 0, "limit": 50, "owner": True}}},
        {"version": 1, "method": "broker_request", "broker_operation": "対話履歴閲覧", "payload": {"approval_id": "a" * 32, "query": {"after": 0, "limit": 50, "filter": {"authority": "owner"}}}},
        {"version": 1, "method": "pair", "authority": "owner"},
        {"version": 1, "method": "set_foreground", "foreground": False},
        {"version": 1, "method": "set_foreground", "foreground": True},
        {"version": 1, "method": "unknown"},
    ]
    for candidate in invalid:
        if not validate_instance(candidate, schema):
            errors.append(
                "Mobile native channelが招待・資格・権限または禁止operationを受理: "
                + str(candidate.get("method"))
                + "/"
                + str(candidate.get("broker_operation"))
            )
    return errors


def Mobile_local_deleteを有界の非権威回復記録へ閉じる() -> list[str]:
    schema = load_schema("mobile_local_recovery_audit.schema.json")
    event = load_contract_fixture("mobile_local_recovery_audit.valid.json")
    errors = validate_instance(event, schema)
    if validate_instance(event, load_schema("audit.schema.json")):
        errors.append("端末内回復記録が共通AuditEvent形式に適合しない")
    milliseconds_event = copy.deepcopy(event)
    milliseconds_event["timestamp"] = "2026-09-25T12:00:00.123Z"
    if validate_instance(milliseconds_event, schema):
        errors.append("端末内回復記録schemaが正規UTCミリ秒日時を受理しない")
    if event.get("payload_hash") != "sha256:" + hashlib.sha256(
        b"gui-shell/mobile/local-credential-delete/v1"
    ).hexdigest():
        errors.append("端末内回復記録hashが固定の公開操作識別子と一致しない")

    changed_hash = copy.deepcopy(event)
    changed_hash["payload_hash"] = "sha256:" + "0" * 64
    if not validate_instance(changed_hash, schema):
        errors.append("端末内回復記録schemaが固定操作hash以外を拒否しない")

    secret_event = {**event, "credential_secret": "not-allowed"}
    if not validate_instance(secret_event, schema):
        errors.append("端末内回復記録schemaが資格秘密を拒否しない")
    forged_revocation = copy.deepcopy(event)
    forged_revocation["metadata"]["desktop_revocation"] = "confirmed"
    if not validate_instance(forged_revocation, schema):
        errors.append("端末内回復記録がDesktop失効の虚偽確認を拒否しない")
    for timestamp in (
        "2026-02-30T12:00:00Z",
        "2026-09-25T12:00:00+00:00",
        "2026-09-25T12:00:00Z\n",
        "2026-09-25T12:00:00.1Z",
        "2026-09-25T12:00:00.1234Z",
    ):
        malformed_time = copy.deepcopy(event)
        malformed_time["timestamp"] = timestamp
        if not validate_instance(malformed_time, schema):
            errors.append(f"端末内回復記録schemaが非正規UTC日時を拒否しない: {timestamp!r}")

    android_store = (MOBILE_FLUTTER / "android/app/src/main/kotlin/com/example/gui_shell_mobile/DeviceLinkNativeStore.kt").read_text(encoding="utf-8")
    android_service = (MOBILE_FLUTTER / "android/app/src/main/kotlin/com/example/gui_shell_mobile/DeviceLinkNativeService.kt").read_text(encoding="utf-8")
    android_models = (MOBILE_FLUTTER / "android/app/src/main/kotlin/com/example/gui_shell_mobile/DeviceLinkModels.kt").read_text(encoding="utf-8")
    ios_store = (MOBILE_FLUTTER / "ios/Runner/DeviceLinkNativeStore.swift").read_text(encoding="utf-8")
    ios_service = (MOBILE_FLUTTER / "ios/Runner/DeviceLinkNativeService.swift").read_text(encoding="utf-8")
    ios_models = (MOBILE_FLUTTER / "ios/Runner/DeviceLinkModels.swift").read_text(encoding="utf-8")
    for platform, store, service, models, limit in (
        ("Android", android_store, android_service, android_models, "MAX_EVENTS = 32"),
        ("iOS", ios_store, ios_service, ios_models, "maximumEvents = 32"),
    ):
        if "deleteCredentialWithLocalRecoveryAudit" not in store or "deleteCredentialWithLocalRecoveryAudit" not in service:
            errors.append(f"{platform}のlocal_deleteが資格削除と回復記録の統合保存を使わない")
        if limit not in models:
            errors.append(f"{platform}の回復記録保持上限が32件でない")
        if "mobile_local_credential_delete" not in models or "INTERNAL_STATE" not in models:
            errors.append(f"{platform}の回復記録が操作と証拠範囲を固定しない")
        if "local_recovery_audit" not in store or '"credential"' not in store:
            errors.append(f"{platform}が資格状態と回復記録を同じ保護storeへ保存しない")
        if "store.deleteCredential()" not in service:
            errors.append(f"{platform}通常disconnectのDesktop失効後readback経路が保持されない")
        if "Desktop側の失効は未確認" not in service:
            errors.append(f"{platform}のnative表示がDesktop未失効を示さない")
    if "1L ->" not in android_store or "2L ->" not in android_store:
        errors.append("Android OS保護storeが旧版読取と現行版検査を維持しない")
    if "version == 1" not in ios_store or "version == 2" not in ios_store:
        errors.append("iOS OS保護storeが旧版読取と現行版検査を維持しない")
    if "read_recovery_audit" not in android_service or "readLocalRecoveryAudit" not in android_store:
        errors.append("Androidがnativeの読み取り専用回復記録経路を持たない")
    if "read_recovery_audit" not in ios_service or "readLocalRecoveryAudit" not in ios_store:
        errors.append("iOSがnativeの読み取り専用回復記録経路を持たない")
    mobile_client = (MOBILE_FLUTTER / "lib/services/device_link_client.dart").read_text(encoding="utf-8")
    mobile_screen = (MOBILE_FLUTTER / "lib/screens/device_connection.dart").read_text(encoding="utf-8")
    if (
        "read_recovery_audit" not in mobile_client
        or "eventKeys.containsAll(value.keys)" not in mobile_client
        or "metadataKeys.containsAll(metadataValue.keys)" not in mobile_client
        or "_isCanonicalUtcTimestamp" not in mobile_client
    ):
        errors.append("Flutterの回復記録読取経路または秘密field拒否がない")
    if "Desktopの監査連鎖とは別" not in mobile_screen or "端末内回復記録を確認" not in mobile_screen:
        errors.append("Mobile回復記録画面がDesktop Auditとの非同一性を説明しない")
    return errors


def test_schema_validator_supports_composition_keywords() -> list[str]:
    errors = []
    any_of = {"anyOf": [{"type": "string"}, {"type": "integer"}]}
    if validate_instance("ok", any_of) or validate_instance(3, any_of):
        errors.append("schema validatorのanyOfが適合値を拒否した")
    if not validate_instance(True, any_of):
        errors.append("schema validatorのanyOfが型不一致を拒否しなかった")

    not_schema = {"not": {"enum": ["forbidden"]}}
    if validate_instance("allowed", not_schema):
        errors.append("schema validatorのnotが許可値を拒否した")
    if not validate_instance("forbidden", not_schema):
        errors.append("schema validatorのnotが禁止値を拒否しなかった")

    property_names = {
        "type": "object",
        "propertyNames": {"not": {"enum": ["secret"]}},
    }
    if validate_instance({"ordinary": 1}, property_names):
        errors.append("schema validatorのpropertyNamesが通常fieldを拒否した")
    if not validate_instance({"secret": 1}, property_names):
        errors.append("schema validatorのpropertyNamesが禁止fieldを拒否しなかった")

    contains = {"type": "array", "contains": {"pattern": "^--dart-define="}}
    if validate_instance(["flutter", "--dart-define=GUI_SHELL_MODULE_TEST=false"], contains):
        errors.append("schema validatorのcontainsが一致要素を拒否した")
    if not validate_instance(["flutter", "build", "windows"], contains):
        errors.append("schema validatorのcontainsが不一致配列を拒否しなかった")
    return errors


def test_schema_validator_enforces_date_time_format() -> list[str]:
    schema = {"type": "string", "format": "date-time"}
    errors = []
    for value in (
        "2026-09-25T12:00:00Z",
        "2026-09-25T12:00:00.123456789Z",
        "2026-09-25T12:00:00+09:00",
        "2026-09-25t12:00:00z",
    ):
        if validate_instance(value, schema):
            errors.append(f"schema validatorが有効なdate-timeを拒否した: {value!r}")
    for value in (
        "2026-02-30T12:00:00Z",
        "2026-09-25T12:00:00",
        "2026-09-25T12:00:00+24:00",
        "2026-09-25T12:00:00Z\n",
    ):
        if not validate_instance(value, schema):
            errors.append(f"schema validatorが不正なdate-timeを受理した: {value!r}")
    return errors


def 対話操作の分岐と未知fieldを検査する() -> list[str]:
    schema = load_schema("runtime_dialogue_operation.schema.json")
    errors = []
    samples = {
        "実行系列挙": {}, "対話セッション一覧": {}, "対話承認待ち": {}, "対話開始": {"実行系ID": "local"},
        "対話送信": {"対話セッションID": "a" * 32, "入力": "こんにちは"},
        "対話取得": {"要求ID": "b" * 32}, "対話中止": {"要求ID": "b" * 32},
        "対話終了": {"対話セッションID": "a" * 32},
        "対話承認": {"要求ID": "b" * 32, "要求hash": "sha256:" + "c" * 64, "表示範囲": "full"},
    }
    for operation, payload in samples.items():
        errors.extend(validate_instance({"operation": operation, "payload": payload}, schema))
        if not validate_instance({"operation": operation, "payload": {**payload, "authority": "owner"}}, schema):
            errors.append("対話操作が未知の権限fieldを受理した")
        if payload and not validate_instance({"operation": operation, "payload": {}}, schema):
            errors.append("対話操作が必須fieldの欠落を受理した")
    bound_start = {"operation": "対話開始", "payload": {"実行系ID": "codex-local", "作業領域ID": "workspace-local"}}
    if validate_instance(bound_start, schema):
        errors.append("対話開始が明示Workspace参照を受理しない")
    invalid_bound_start = {"operation": "対話開始", "payload": {"実行系ID": "codex-local", "作業領域ID": "../workspace"}}
    if validate_instance(invalid_bound_start, schema) == []:
        errors.append("対話開始が不正Workspace識別子を拒否しない")
    for value in ({"operation": "未知操作", "payload": {}}, {"operation": "対話取得", "payload": {"実行系ID": "local"}}):
        if not validate_instance(value, schema):
            errors.append("操作とpayloadの不整合を受理した")
    if not validate_instance({}, {"oneOf": [{"type": "object"}, {"type": "object"}]}):
        errors.append("oneOfの複数一致を拒否しなかった")
    return errors


def 対話セッション一覧の有界投影を検査する() -> list[str]:
    schema = load_schema("runtime_dialogue_session_list.schema.json")
    fixture = load_contract_fixture("runtime_dialogue_session_list.valid.json")
    negative = load_contract_fixture("invalid/runtime_dialogue_session_list_content.invalid.json")
    errors = validate_instance(fixture, schema)
    if validate_instance(negative, schema) == []:
        errors.append("対話セッション一覧がTask本文を含む未知fieldを拒否しない")

    sessions = fixture.get("対話セッション", [])
    identifiers = [item.get("対話セッションID") for item in sessions if isinstance(item, dict)]
    if len(identifiers) != len(set(identifiers)):
        errors.append("正常fixtureの対話セッションIDが重複している")

    duplicate = {
        "版": 1,
        "対話セッション": [fixture["対話セッション"][0], fixture["対話セッション"][0]],
    }
    def 識別子一意(value: dict) -> bool:
        items = value.get("対話セッション")
        if not isinstance(items, list):
            return False
        identifiers = [item.get("対話セッションID") for item in items if isinstance(item, dict)]
        return len(identifiers) == len(items) == len(set(identifiers))

    if not 識別子一意(fixture) or 識別子一意(duplicate):
        errors.append("重複対話セッションIDを意味検査で拒否できない")

    bounded = {
        "版": 1,
        "対話セッション": [
            {
                "対話セッションID": f"{index:032x}",
                "実行系ID": f"runtime-{index}",
                "状態": "利用中",
                "作成監査ID": f"audit-dialogue-{index}",
                "作業領域ID": f"workspace-{index}",
                "作業領域結合監査ID": f"audit-workspace-{index}",
            }
            for index in range(64)
        ],
    }
    if validate_instance(bounded, schema):
        errors.append("上限64件の対話セッション一覧を受理しない")
    over_bound = {**bounded, "対話セッション": [*bounded["対話セッション"], bounded["対話セッション"][0]]}
    if validate_instance(over_bound, schema) == []:
        errors.append("上限超過の対話セッション一覧を拒否しない")

    for invalid in (
        {**fixture, "版": 2},
        {**fixture, "authority": "owner"},
        {"版": 1, "対話セッション": [{**fixture["対話セッション"][0], "Workspace": "workspace-a"}]},
        {"版": 1, "対話セッション": [{key: value for key, value in fixture["対話セッション"][0].items() if key != "作業領域ID"}]},
        {"版": 1, "対話セッション": [{key: value for key, value in fixture["対話セッション"][0].items() if key != "作業領域結合監査ID"}]},
        {"版": 1, "対話セッション": [{**fixture["対話セッション"][0], "状態": "trusted"}]},
        {"版": 1, "対話セッション": [{**fixture["対話セッション"][0], "実行系ID": "../runtime"}]},
        {"版": 1, "対話セッション": [{**fixture["対話セッション"][0], "作成監査ID": ""}]},
    ):
        if validate_instance(invalid, schema) == []:
            errors.append("版違い・Authority・未検証Workspaceを拒否しない")
    return errors


def 履歴入力概要のhash_only境界を検査する() -> list[str]:
    surfaces = [
        (
            "runtime_execution_history",
            load_schema("runtime_execution_history.schema.json"),
            load_contract_fixture("runtime_execution_history.valid.json"),
            lambda value: value,
        ),
        (
            "runtime_execution_history_page",
            load_schema("runtime_execution_history_page.schema.json"),
            load_contract_fixture("runtime_execution_history_page.valid.json"),
            lambda value: value["entries"][0]["record"],
        ),
    ]
    access_page = load_contract_fixture("runtime_execution_history_page.valid.json")
    surfaces.append((
        "runtime_history_access",
        load_schema("runtime_history_access.schema.json"),
        {
            "grant": {
                "approval_id": "d" * 32,
                "runtime_id": "local",
                "expires_at": 1,
            },
            "page": access_page,
        },
        lambda value: value["page"]["entries"][0]["record"],
    ))
    errors = []
    for name, schema, valid, record_of in surfaces:
        errors.extend(validate_instance(valid, schema))
        legacy = copy.deepcopy(valid)
        legacy_record = record_of(legacy)
        legacy_record["版"] = 1
        legacy_record.pop("入力概要")
        errors.extend(validate_instance(legacy, schema))

        invalid_cases = {
            "入力概要欠落": lambda record: record.pop("入力概要"),
            "表示範囲昇格": lambda record: record["入力概要"].update({"表示範囲": "full"}),
            "追加metadata": lambda record: record["入力概要"].update({"文字数": 4}),
            "hash不正": lambda record: record["入力概要"].update({"入力hash": "sha256:invalid"}),
            "本文混入": lambda record: record["入力概要"].update({"本文": "漏洩"}),
            "未知版": lambda record: record.update({"版": 3}),
            "型違い版": lambda record: record.update({"版": "2"}),
        }
        for label, alter in invalid_cases.items():
            candidate = copy.deepcopy(valid)
            alter(record_of(candidate))
            if not validate_instance(candidate, schema):
                errors.append(f"{name}が版2入力概要の{label}を受理した")

        v1_with_summary = copy.deepcopy(legacy)
        v1_record = record_of(v1_with_summary)
        v1_record["入力概要"] = copy.deepcopy(record_of(valid)["入力概要"])
        if not validate_instance(v1_with_summary, schema):
            errors.append(f"{name}が版1への入力概要追加を受理した")
    return errors


def AgentTask履歴のContractと現在承認経路を検査する() -> list[str]:
    errors: list[str] = []
    for schema_name, fixture_name in (
        ("agent_task_history_page", "agent_task_history_page.valid.json"),
        ("runtime_agent_task_history_access", "runtime_agent_task_history_access.valid.json"),
        ("runtime_agent_task_history_request", "runtime_agent_task_history_request.valid.json"),
    ):
        errors.extend(validate_instance(
            load_contract_fixture(fixture_name), load_schema(schema_name + ".schema.json")
        ))
    invalid_request = load_contract_fixture("invalid/runtime_agent_task_history_request_authority.invalid.json")
    if not validate_instance(invalid_request, load_schema("runtime_agent_task_history_request.schema.json")):
        errors.append("Agent Task履歴要求がRuntimeまたはAuthority注入を受理")
    invalid_page = load_contract_fixture("invalid/agent_task_history_page_content.invalid.json")
    if not validate_instance(invalid_page, load_schema("agent_task_history_page.schema.json")):
        errors.append("Agent Task履歴projectionがTask本文を受理")
    for schema_name in ("ipc_request", "ipc_response"):
        operations = load_schema(schema_name + ".schema.json")["properties"]["operation"]["enum"]
        if "AgentTask履歴閲覧" not in operations:
            errors.append(f"{schema_name}にAgent Task履歴操作がない")
    protocol = (RUST_HELPER / "src" / "broker" / "protocol.rs").read_text(encoding="utf-8")
    access = (RUST_HELPER / "src" / "broker" / "history_access.rs").read_text(encoding="utf-8")
    audit_projection = (RUST_HELPER / "src" / "broker" / "execution_history.rs").read_text(encoding="utf-8")
    task_runtime = (RUST_HELPER / "src" / "broker" / "dialogue.rs").read_text(encoding="utf-8")
    client = (ROOT / "packages" / "gui_shell_ui" / "lib" / "src" / "history_client.dart").read_text(encoding="utf-8")
    screen = (DESKTOP_FLUTTER / "lib" / "screens" / "history_screen.dart").read_text(encoding="utf-8")
    for token, source in (
        ("AgentTask履歴閲覧", protocol),
        ("履歴閲覧処理", protocol),
        ("agent_task_page(log, &g.runtime", access),
        ("INTERNAL_STATE", audit_projection),
        ("AGENT_TASK_HISTORY_PREFIX", audit_projection),
        ("encode_agent_task_audit", task_runtime),
        ("AgentTask履歴閲覧", client),
        ("Agent Task履歴", screen),
    ):
        if token not in source:
            errors.append(f"Agent Task履歴のproduction接続または境界がない: {token}")
    page_schema = load_schema("agent_task_history_page.schema.json")
    entry_properties = page_schema["properties"]["entries"]["items"]["properties"]
    record_properties = entry_properties["record"]["properties"]
    exposed_names = set(entry_properties) | set(record_properties)
    for forbidden in ("result_body", "instruction", "credential", "approval_id", "authority"):
        if forbidden in exposed_names:
            errors.append(f"Agent Task履歴Schemaが非公開fieldを投影: {forbidden}")
    return errors


def 実行系資源観測の証拠境界を検査する() -> list[str]:
    query_schema = load_schema("runtime_resource_query.schema.json")
    observation_schema = load_schema("runtime_resource_observation.schema.json")
    ipc_request_schema = load_schema("ipc_request.schema.json")
    ipc_response_schema = load_schema("ipc_response.schema.json")
    query = load_contract_fixture("runtime_resource_query.valid.json")
    observation = load_contract_fixture("runtime_resource_observation.valid.json")
    errors = []
    errors.extend(validate_instance(query, query_schema))
    errors.extend(validate_instance(observation, observation_schema))

    for token in ("NaN", "Infinity", "-Infinity", "1e400"):
        try:
            parse_json_text('{"値": ' + token + "}")
        except ValueError:
            pass
        else:
            errors.append(f"JSON readerが非有限数{token}を受理する")
    for nonfinite in (float("nan"), float("inf"), -float("inf")):
        candidate = copy.deepcopy(observation)
        candidate["計測"]["CPU利用率Percent"]["値"] = nonfinite
        if not validate_instance(candidate, observation_schema):
            errors.append("C3 schema validatorが非有限CPU利用率を受理する")

    def c3_metric_boundary_errors(candidate: dict) -> list[str]:
        binding = candidate.get("結合")
        metrics = candidate.get("計測")
        history = candidate.get("短期履歴")
        boundary_errors = []
        if not isinstance(binding, dict) or not isinstance(metrics, dict) or not isinstance(history, list):
            return ["C3資源観測の状態境界を評価できない"]

        def metric_is(key: str, state: str, evidence_source: str, source: dict = metrics) -> bool:
            metric = source.get(key)
            return (
                isinstance(metric, dict)
                and metric.get("状態") == state
                and metric.get("証拠種別") == evidence_source
            )

        def metric_source_is(key: str, evidence_source: str, source: dict = metrics) -> bool:
            metric = source.get(key)
            return isinstance(metric, dict) and metric.get("証拠種別") == evidence_source

        if binding.get("状態") != "bound":
            if history:
                boundary_errors.append("非結合C3観測が短期履歴を返す")
            for key in (
                "稼働時間Millis",
                "CPU累積時間Millis",
                "CPU利用率Percent",
                "RAMWorkingSetBytes",
                "RAMPrivateBytes",
                "DiskIOBytes",
                "NetworkIOBytes",
                "GPU利用率Percent",
                "VRAMBytes",
                "処理中要求数",
                "平均応答Millis",
                "失敗要求数",
            ):
                if not metric_is(key, "unknown", "INTERNAL_STATE"):
                    boundary_errors.append(f"非結合C3観測の{key}がunknown INTERNAL_STATEでない")
            return boundary_errors

        for key in ("RAMWorkingSetBytes", "RAMPrivateBytes"):
            if not metric_is(key, "measured", "LIVE_RUNTIME"):
                boundary_errors.append(f"結合C3観測の{key}がmeasured LIVE_RUNTIMEでない")
        for key in ("稼働時間Millis", "CPU累積時間Millis", "CPU利用率Percent"):
            if not metric_source_is(key, "LIVE_RUNTIME"):
                boundary_errors.append(f"結合C3観測の{key}がLIVE_RUNTIMEでない")
        for key in ("DiskIOBytes", "NetworkIOBytes", "GPU利用率Percent", "VRAMBytes", "平均応答Millis"):
            if not metric_is(key, "unknown", "INTERNAL_STATE"):
                boundary_errors.append(f"未接続C3 metric {key}がunknown INTERNAL_STATEでない")
        for key in ("処理中要求数", "失敗要求数"):
            if not metric_is(key, "measured", "INTERNAL_STATE"):
                boundary_errors.append(f"Broker集計 {key}がmeasured INTERNAL_STATEでない")
        for index, sample in enumerate(history):
            if not isinstance(sample, dict):
                boundary_errors.append(f"短期履歴{index}がobjectでない")
                continue
            if not metric_source_is("CPU利用率Percent", "LIVE_RUNTIME", sample):
                boundary_errors.append(f"短期履歴{index}のCPU利用率PercentがLIVE_RUNTIMEでない")
            if not metric_is("RAMWorkingSetBytes", "measured", "LIVE_RUNTIME", sample):
                boundary_errors.append(f"短期履歴{index}のRAMWorkingSetBytesがmeasured LIVE_RUNTIMEでない")
            if not metric_is("NetworkIOBytes", "unknown", "INTERNAL_STATE", sample):
                boundary_errors.append(f"短期履歴{index}のNetworkIOBytesがunknown INTERNAL_STATEでない")
            if not metric_is("平均応答Millis", "unknown", "INTERNAL_STATE", sample):
                boundary_errors.append(f"短期履歴{index}の平均応答Millisがunknown INTERNAL_STATEでない")
            if not metric_source_is("エラー率Percent", "INTERNAL_STATE", sample):
                boundary_errors.append(f"短期履歴{index}のエラー率PercentがINTERNAL_STATEでない")
        return boundary_errors

    errors.extend(c3_metric_boundary_errors(observation))

    for label, alter in (
        ("PID入力", lambda value: value.update({"PID": 4321})),
        ("接続先入力", lambda value: value.update({"endpoint": "127.0.0.1:8080"})),
        ("未知版", lambda value: value.update({"版": 2})),
    ):
        candidate = copy.deepcopy(query)
        alter(candidate)
        if not validate_instance(candidate, query_schema):
            errors.append(f"実行系資源観測要求が{label}を受理した")

    invalid_metrics = (
        ("unknownを0へ置換", lambda value: value["計測"]["DiskIOBytes"].update({"値": 0})),
        ("unknownの理由欠落", lambda value: value["計測"]["DiskIOBytes"].pop("理由")),
        ("measuredへの理由混入", lambda value: value["計測"]["失敗要求数"].update({"理由": "不要"})),
        ("CPU百分率上限超過", lambda value: value["計測"]["CPU利用率Percent"].update({"値": 101})),
    )
    for label, alter in invalid_metrics:
        candidate = copy.deepcopy(observation)
        alter(candidate)
        if not validate_instance(candidate, observation_schema):
            errors.append(f"実行系資源観測がmetricの{label}を受理した")

    semantic_metric_mutations = (
        (
            "未接続Disk I/Oのmeasured",
            lambda value: value["計測"].__setitem__(
                "DiskIOBytes",
                {"状態": "measured", "値": 1, "証拠種別": "INTERNAL_STATE"},
            ),
        ),
        (
            "未接続Network I/OのLIVE_RUNTIME",
            lambda value: value["計測"]["NetworkIOBytes"].update({"証拠種別": "LIVE_RUNTIME"}),
        ),
        (
            "CPUのINTERNAL_STATE",
            lambda value: value["計測"]["CPU利用率Percent"].update({"証拠種別": "INTERNAL_STATE"}),
        ),
        (
            "要求数のLIVE_RUNTIME",
            lambda value: value["計測"]["処理中要求数"].update({"証拠種別": "LIVE_RUNTIME"}),
        ),
        (
            "平均応答のmeasured",
            lambda value: value["計測"].__setitem__(
                "平均応答Millis",
                {"状態": "measured", "値": 1, "証拠種別": "INTERNAL_STATE"},
            ),
        ),
        (
            "履歴Network I/OのLIVE_RUNTIME",
            lambda value: value["短期履歴"][0]["NetworkIOBytes"].update({"証拠種別": "LIVE_RUNTIME"}),
        ),
        (
            "履歴平均応答のmeasured",
            lambda value: value["短期履歴"][0].__setitem__(
                "平均応答Millis",
                {"状態": "measured", "値": 1, "証拠種別": "INTERNAL_STATE"},
            ),
        ),
    )
    for label, alter in semantic_metric_mutations:
        candidate = copy.deepcopy(observation)
        alter(candidate)
        if not c3_metric_boundary_errors(candidate):
            errors.append(f"C3 metric境界が{label}を受理する")

    invalid_binding = copy.deepcopy(observation)
    invalid_binding["結合"]["PID"] = 0
    if not validate_instance(invalid_binding, observation_schema):
        errors.append("結合済み観測がPID=0を受理した")
    invalid_binding = copy.deepcopy(observation)
    invalid_binding["結合"]["endpoint"] = "127.0.0.1:8080"
    if not validate_instance(invalid_binding, observation_schema):
        errors.append("結合情報が接続先を公開した")

    history_overflow = copy.deepcopy(observation)
    history_overflow["短期履歴"] = [copy.deepcopy(observation["短期履歴"][0]) for _ in range(61)]
    if not validate_instance(history_overflow, observation_schema):
        errors.append("実行系資源観測が短期履歴61件を受理した")
    invalid_governance = copy.deepcopy(observation)
    invalid_governance["統治"]["承認状態"] = "approved"
    if not validate_instance(invalid_governance, observation_schema):
        errors.append("実行系資源観測が統治fieldの自己変更を受理した")

    def unknown_metric(evidence_source: str) -> dict:
        return {
            "状態": "unknown",
            "値": None,
            "証拠種別": evidence_source,
            "理由": "結合を再確認する必要がある",
        }

    bound_safe_unknown = copy.deepcopy(observation)
    for name in ("稼働時間Millis", "CPU累積時間Millis", "CPU利用率Percent"):
        bound_safe_unknown["計測"][name] = unknown_metric("LIVE_RUNTIME")
    errors.extend(validate_instance(bound_safe_unknown, observation_schema))
    if c3_metric_boundary_errors(bound_safe_unknown):
        errors.append("結合済みC3観測が安全なLIVE_RUNTIME unknownを拒否する")

    unavailable = copy.deepcopy(observation)
    unavailable["結合"] = {
        "状態": "binding_mismatch",
        "根拠": "loopback_tcp_listener_owner_pid",
        "PID": None,
        "PID作成時刻UnixMillis": None,
        "登録時刻UnixMillis": observation["結合"]["登録時刻UnixMillis"],
        "登録監査ID": observation["結合"]["登録監査ID"],
        "理由": "PID作成時刻が登録時と一致しない",
    }
    for name in unavailable["計測"]:
        unavailable["計測"][name] = unknown_metric("INTERNAL_STATE")
    unavailable["短期履歴"] = []
    errors.extend(validate_instance(unavailable, observation_schema))
    errors.extend(c3_metric_boundary_errors(unavailable))
    unavailable_with_pid = copy.deepcopy(unavailable)
    unavailable_with_pid["結合"]["PID"] = 4321
    if not validate_instance(unavailable_with_pid, observation_schema):
        errors.append("PID不一致の結合がPIDを返した")
    unavailable_with_value = copy.deepcopy(unavailable)
    unavailable_with_value["計測"]["失敗要求数"] = copy.deepcopy(observation["計測"]["失敗要求数"])
    if not validate_instance(unavailable_with_value, observation_schema):
        errors.append("PID不一致の結合が実測値を返した")
    unavailable_with_history = copy.deepcopy(unavailable)
    unavailable_with_history["短期履歴"] = copy.deepcopy(observation["短期履歴"])
    if not validate_instance(unavailable_with_history, observation_schema):
        errors.append("PID不一致の結合が過去短期履歴を返した")

    unavailable_wrong_source = copy.deepcopy(unavailable)
    unavailable_wrong_source["計測"]["CPU利用率Percent"]["証拠種別"] = "LIVE_RUNTIME"
    if not c3_metric_boundary_errors(unavailable_wrong_source):
        errors.append("非結合C3観測がLIVE_RUNTIMEのunknown metricを受理する")

    ipc_request = {
        "request_id": "resource-observation-request",
        "operation": "実行系資源観測",
        "payload_hash": "sha256:" + "0" * 64,
        "nonce": "resource-observation-nonce",
        "issued_at": "1789393500000",
        "metadata": {},
        "payload": query,
    }
    errors.extend(validate_instance(ipc_request, ipc_request_schema))

    def ipc_relation_errors(response: dict) -> list[str]:
        body = response.get("body")
        if not isinstance(body, dict):
            return ["実行系資源観測IPC bodyがobjectでない"]
        if response.get("audit_event_id") != body.get("観測監査ID"):
            return ["実行系資源観測IPC監査IDがbodyと一致しない"]
        binding = body.get("結合")
        if not isinstance(binding, dict):
            return ["実行系資源観測IPC bodyに結合がない"]
        expected_evidence = (
            "LIVE_RUNTIME"
            if binding.get("状態") == "bound"
            else "INTERNAL_STATE"
        )
        if response.get("evidence_source") != expected_evidence:
            return ["実行系資源観測IPC証拠種別が結合状態と一致しない"]
        return []

    ipc_response = {
        "request_id": ipc_request["request_id"],
        "operation": "実行系資源観測",
        "status": "accepted",
        "evidence_source": "LIVE_RUNTIME",
        "audit_event_id": observation["観測監査ID"],
        "error": None,
        "health": None,
        "body": observation,
        "shutdown_requested": False,
    }
    errors.extend(validate_instance(ipc_response, ipc_response_schema))
    errors.extend(ipc_relation_errors(ipc_response))
    invalid_audit_response = {**ipc_response, "audit_event_id": "other-audit"}
    if not ipc_relation_errors(invalid_audit_response):
        errors.append("実行系資源観測IPCが監査ID不一致を受理した")
    invalid_evidence_response = {**ipc_response, "evidence_source": "INTERNAL_STATE"}
    if not ipc_relation_errors(invalid_evidence_response):
        errors.append("結合済み観測IPCがINTERNAL_STATEを受理した")
    unavailable_response = {
        **ipc_response,
        "body": unavailable,
        "audit_event_id": unavailable["観測監査ID"],
        "evidence_source": "INTERNAL_STATE",
    }
    errors.extend(validate_instance(unavailable_response, ipc_response_schema))
    errors.extend(ipc_relation_errors(unavailable_response))
    invalid_unavailable_evidence = {**unavailable_response, "evidence_source": "LIVE_RUNTIME"}
    if not ipc_relation_errors(invalid_unavailable_evidence):
        errors.append("非結合観測IPCがLIVE_RUNTIMEを受理した")
    for schema_name, schema in (("ipc_request", ipc_request_schema), ("ipc_response", ipc_response_schema)):
        operations = schema["properties"]["operation"]["enum"]
        if "実行系資源観測" not in operations:
            errors.append(f"{schema_name}が実行系資源観測operationを公開していない")
    return errors


def 実行系ライフサイクルの契約と統治境界を検査する() -> list[str]:
    request_schema = load_schema("runtime_lifecycle_request.schema.json")
    result_schema = load_schema("runtime_lifecycle_result.schema.json")
    runtime_schema = load_schema("runtime.schema.json")
    ipc_request_schema = load_schema("ipc_request.schema.json")
    ipc_response_schema = load_schema("ipc_response.schema.json")
    command_schema = load_schema("broker_command_envelope.schema.json")
    request = load_contract_fixture("runtime_lifecycle_request.valid.json")
    transition = load_contract_fixture("runtime_lifecycle_result.valid.json")
    errors = []

    errors.extend(validate_instance(request, request_schema))
    errors.extend(validate_instance(transition, result_schema))

    expected_operations = ("start", "stop", "restart", "pause", "resume", "quarantine")
    request_shapes = {
        "実行系ライフサイクル状態": {"版", "実行系ID"},
        "実行系ライフサイクル承認要求": {"版", "実行系ID", "操作"},
        "実行系ライフサイクル承認": {"版", "承認ID", "承認hash"},
        "実行系ライフサイクル操作": {"版", "実行系ID", "操作", "承認ID"},
    }
    request_payloads = {
        "実行系ライフサイクル状態": {"版": 1, "実行系ID": "fixture-runtime"},
        "実行系ライフサイクル承認要求": {
            "版": 1,
            "実行系ID": "fixture-runtime",
            "操作": "pause",
        },
        "実行系ライフサイクル承認": {
            "版": 1,
            "承認ID": "lifecycle-approval-42",
            "承認hash": "sha256:" + "a" * 64,
        },
        "実行系ライフサイクル操作": request,
    }
    for operation, payload in request_payloads.items():
        errors.extend(validate_instance(payload, request_schema))
        if set(payload) != request_shapes[operation]:
            errors.append(f"{operation}のpayload field集合が固定されていない")

    def lifecycle_request_relation_errors(operation: str, payload: object) -> list[str]:
        relation_errors = []
        expected_fields = request_shapes.get(operation)
        if expected_fields is None:
            return ["未知のlifecycle IPC operation"]
        if not isinstance(payload, dict):
            return ["lifecycle IPC payloadがobjectでない"]
        if set(payload) != expected_fields:
            relation_errors.append("lifecycle IPC operationとpayload field集合が一致しない")
        if validate_instance(payload, request_schema):
            relation_errors.append("lifecycle IPC payloadがrequest schemaに適合しない")
        return relation_errors

    for operation, payload in request_payloads.items():
        errors.extend(
            f"{operation}: {failure}"
            for failure in lifecycle_request_relation_errors(operation, payload)
        )
    if not lifecycle_request_relation_errors(
        "実行系ライフサイクル操作",
        request_payloads["実行系ライフサイクル状態"],
    ):
        errors.append("lifecycle IPCが状態payloadを実行操作として受理する")

    for label, field, value in (
        ("PID", "PID", 4321),
        ("endpoint", "endpoint", "127.0.0.1:8080"),
        ("command", "command", "cmd.exe"),
        ("argv", "argv", ["--unsafe"]),
        ("env", "env", {"PATH": "unsafe"}),
        ("Capability ID", "能力ID", "runtime.lifecycle.control"),
        ("Permission ID", "権限ID", "permission.runtime.lifecycle.pause"),
        ("Audit ID", "監査ID", "forged-audit"),
        ("Recovery ID", "復旧ID", "forged-recovery"),
        ("caller state", "状態", "ready"),
        ("authority", "authority", "root"),
        ("metadata", "metadata", {"trust_level": "root"}),
    ):
        candidate = copy.deepcopy(request)
        candidate[field] = value
        if not validate_instance(candidate, request_schema):
            errors.append(f"実行系ライフサイクル操作が{label}を受理した")

    for label, candidate in (
        (
            "未知操作",
            {
                "版": 1,
                "実行系ID": "fixture-runtime",
                "操作": "kill",
            },
        ),
        (
            "未知版",
            {
                "版": 2,
                "実行系ID": "fixture-runtime",
            },
        ),
        (
            "owner承認の実行系ID注入",
            {
                "版": 1,
                "承認ID": "lifecycle-approval-42",
                "承認hash": "sha256:" + "a" * 64,
                "実行系ID": "fixture-runtime",
            },
        ),
    ):
        if not validate_instance(candidate, request_schema):
            errors.append(f"実行系ライフサイクル要求が{label}を受理した")

    for status in ("paused", "quarantined"):
        runtime = load_contract_fixture("runtime.valid.json")
        runtime["status"] = status
        errors.extend(validate_instance(runtime, runtime_schema))
    runtime = load_contract_fixture("runtime.valid.json")
    runtime["status"] = "unsupported"
    if not validate_instance(runtime, runtime_schema):
        errors.append("Runtime schemaがunsupportedを受理した")

    def governance(operation: str, approval_id: str, approval_status: str) -> dict:
        return {
            "能力ID": f"runtime.lifecycle.{operation}",
            "権限ID": f"permission.runtime.lifecycle.{operation}",
            "承認ID": approval_id,
            "承認状態": approval_status,
            "復旧ID": f"recover-runtime-lifecycle-{operation}",
        }

    executable_in_ready = {"stop", "restart", "pause", "quarantine"}
    status_result = {
        "版": 1,
        "対応": True,
        "実行系ID": "fixture-runtime",
        "状態": "ready",
        "証拠種別": "INTERNAL_STATE",
        "操作一覧": [
            {
                "操作": operation,
                "能力ID": f"runtime.lifecycle.{operation}",
                "権限ID": f"permission.runtime.lifecycle.{operation}",
                "承認必要": True,
                "復旧ID": f"recover-runtime-lifecycle-{operation}",
                "実行可能": operation in executable_in_ready,
            }
            for operation in expected_operations
        ],
        "承認一覧": [
            {
                "版": 1,
                "承認ID": "lifecycle-approval-42",
                "承認hash": "sha256:" + "a" * 64,
                "実行系ID": "fixture-runtime",
                "操作": "pause",
                "状態": "approved",
                "有効期限UnixSeconds": 4102444800,
                "統治": governance("pause", "lifecycle-approval-42", "approved"),
            }
        ],
    }
    unsupported_result = {
        "版": 1,
        "対応": False,
        "実行系ID": "minidora",
        "状態": "not_supported",
        "証拠種別": "INTERNAL_STATE",
        "操作一覧": [],
        "承認一覧": [],
    }
    quarantined_result = {
        "版": 1,
        "対応": True,
        "実行系ID": "fixture-runtime",
        "状態": "quarantined",
        "証拠種別": "LIVE_RUNTIME",
        "操作一覧": [],
        "承認一覧": [],
    }
    for label, result in (
        ("対応実行系状態", status_result),
        ("Capability未宣言状態", unsupported_result),
        ("隔離状態", quarantined_result),
    ):
        failures = validate_instance(result, result_schema)
        errors.extend(f"{label}: {failure}" for failure in failures)

    def state_result_errors(result: dict) -> list[str]:
        state_errors = []
        if result.get("対応") is False:
            if result.get("状態") != "not_supported":
                state_errors.append("Capability未宣言状態がnot_supportedでない")
            if result.get("操作一覧") or result.get("承認一覧"):
                state_errors.append("Capability未宣言状態が操作または承認を返す")
        if result.get("状態") == "quarantined":
            if result.get("操作一覧") or result.get("承認一覧"):
                state_errors.append("隔離後の通常操作または承認が残る")
        operations = result.get("操作一覧", [])
        if result.get("対応") is True and result.get("状態") != "quarantined":
            names = [item.get("操作") for item in operations if isinstance(item, dict)]
            if (
                not names
                or not set(names).issubset(set(expected_operations))
                or len(names) != len(set(names))
            ):
                state_errors.append("対応実行系のBroker操作一覧がCapability宣言集合になっていない")
            executable_by_state = {
                "stopped": {"start", "quarantine"},
                "ready": {"stop", "restart", "pause", "quarantine"},
                "paused": {"stop", "resume", "quarantine"},
                "unknown": set(),
            }
            allowed = executable_by_state.get(result.get("状態"), set())
            for item in operations:
                if not isinstance(item, dict):
                    state_errors.append("操作一覧の要素がobjectでない")
                    continue
                operation = item.get("操作")
                if (
                    item.get("能力ID") != f"runtime.lifecycle.{operation}"
                    or item.get("権限ID") != f"permission.runtime.lifecycle.{operation}"
                    or item.get("承認必要") is not True
                    or item.get("復旧ID") != f"recover-runtime-lifecycle-{operation}"
                ):
                    state_errors.append("操作一覧にBroker生成統治対応がない")
                if item.get("実行可能") is True and operation not in allowed:
                    state_errors.append("現在状態から不可能な操作を実行可能としている")
            for approval in result.get("承認一覧", []):
                if not isinstance(approval, dict):
                    state_errors.append("承認一覧の要素がobjectでない")
                    continue
                approval_governance = approval.get("統治")
                if (
                    approval.get("実行系ID") != result.get("実行系ID")
                    or approval.get("操作") not in names
                    or not isinstance(approval_governance, dict)
                    or approval_governance.get("承認ID") != approval.get("承認ID")
                    or approval_governance.get("承認状態") != approval.get("状態")
                ):
                    state_errors.append("承認一覧が状態表示のRuntime、操作、統治対応と一致しない")
        return state_errors

    errors.extend(state_result_errors(status_result))
    errors.extend(state_result_errors(unsupported_result))
    errors.extend(state_result_errors(quarantined_result))
    invalid_unsupported = copy.deepcopy(unsupported_result)
    invalid_unsupported["操作一覧"] = [copy.deepcopy(status_result["操作一覧"][0])]
    if not validate_instance(invalid_unsupported, result_schema):
        errors.append("Capability未宣言状態が通常操作を返す")
    invalid_quarantine = copy.deepcopy(quarantined_result)
    invalid_quarantine["承認一覧"] = copy.deepcopy(status_result["承認一覧"])
    if not validate_instance(invalid_quarantine, result_schema):
        errors.append("隔離状態が通常承認を返す")
    invalid_operable = copy.deepcopy(status_result)
    invalid_operable["操作一覧"][0]["実行可能"] = True
    if not state_result_errors(invalid_operable):
        errors.append("現在状態から不可能な操作を実行可能にできる")
    invalid_state_pid = copy.deepcopy(status_result)
    invalid_state_pid["PID"] = 4321
    if not validate_instance(invalid_state_pid, result_schema):
        errors.append("ライフサイクル状態結果がPIDを公開する")

    approval_result = {
        "版": 1,
        "承認ID": "lifecycle-approval-42",
        "承認hash": "sha256:" + "a" * 64,
        "実行系ID": "fixture-runtime",
        "操作": "pause",
        "状態": "pending",
        "有効期限UnixSeconds": 4102444800,
        "統治": governance("pause", "lifecycle-approval-42", "pending"),
    }
    approved_result = copy.deepcopy(approval_result)
    approved_result["状態"] = "approved"
    approved_result["統治"]["承認状態"] = "approved"
    for label, result in (("承認要求", approval_result), ("owner承認", approved_result)):
        failures = validate_instance(result, result_schema)
        errors.extend(f"{label}: {failure}" for failure in failures)
        if (
            result["状態"] != result["統治"]["承認状態"]
            or result["承認ID"] != result["統治"]["承認ID"]
        ):
            errors.append(f"{label}の承認状態が統治対応と一致しない")

    expected_transitions = {
        "start": (("stopped", "ready"),),
        "stop": (("ready", "stopped"), ("paused", "stopped")),
        "restart": (("ready", "ready"),),
        "pause": (("ready", "paused"),),
        "resume": (("paused", "ready"),),
        "quarantine": (
            ("stopped", "quarantined"),
            ("ready", "quarantined"),
            ("paused", "quarantined"),
        ),
    }

    def transition_errors(result: dict) -> list[str]:
        transition_errors = []
        if "ライフサイクル監査ID" not in result:
            return ["遷移結果にライフサイクル監査IDがない"]
        operation = result.get("操作")
        expected = expected_transitions.get(operation)
        if expected is None:
            transition_errors.append("未知のlifecycle操作が遷移結果にある")
        elif (result.get("遷移前状態"), result.get("遷移後状態")) not in expected:
            transition_errors.append("操作とライフサイクル状態遷移が一致しない")
        if result.get("証拠種別") != "LIVE_RUNTIME":
            transition_errors.append("成功遷移がLIVE_RUNTIMEでない")
        governance_value = result.get("統治")
        if (
            not isinstance(governance_value, dict)
            or governance_value.get("能力ID") != f"runtime.lifecycle.{operation}"
            or governance_value.get("権限ID") != f"permission.runtime.lifecycle.{operation}"
            or governance_value.get("復旧ID") != f"recover-runtime-lifecycle-{operation}"
            or governance_value.get("承認状態") != "consumed"
        ):
            transition_errors.append("成功遷移に消費済みの統治対応がない")
        return transition_errors

    for operation, transitions in expected_transitions.items():
        before, after = transitions[0]
        result = copy.deepcopy(transition)
        result["操作"] = operation
        result["遷移前状態"] = before
        result["遷移後状態"] = after
        result["統治"] = governance(operation, "lifecycle-approval-42", "consumed")
        result["ライフサイクル監査ID"] = f"lifecycle-transition-{operation}"
        failures = validate_instance(result, result_schema)
        errors.extend(f"{operation}遷移: {failure}" for failure in failures)
        errors.extend(f"{operation}遷移: {failure}" for failure in transition_errors(result))

    invalid_transition = copy.deepcopy(transition)
    invalid_transition["操作"] = "start"
    invalid_transition["遷移前状態"] = "ready"
    invalid_transition["遷移後状態"] = "ready"
    invalid_transition["統治"] = governance("start", "lifecycle-approval-42", "consumed")
    if not transition_errors(invalid_transition):
        errors.append("不正なlifecycle状態遷移を受理する")
    invalid_transition = copy.deepcopy(transition)
    invalid_transition["統治"]["承認状態"] = "pending"
    if not validate_instance(invalid_transition, result_schema) or not transition_errors(invalid_transition):
        errors.append("未承認lifecycle遷移を受理する")

    ipc_request = {
        "request_id": "lifecycle-request-42",
        "session_id": "current-session",
        "operation": "実行系ライフサイクル操作",
        "payload_hash": "sha256:" + "0" * 64,
        "nonce": "lifecycle-nonce-42",
        "issued_at": "1789393500000",
        "metadata": {},
        "payload": request,
    }
    ipc_response = {
        "request_id": ipc_request["request_id"],
        "operation": "実行系ライフサイクル操作",
        "status": "accepted",
        "evidence_source": "LIVE_RUNTIME",
        "audit_event_id": transition["ライフサイクル監査ID"],
        "error": None,
        "health": None,
        "body": transition,
        "shutdown_requested": False,
    }
    errors.extend(validate_instance(ipc_request, ipc_request_schema))
    errors.extend(validate_instance(ipc_response, ipc_response_schema))

    def ipc_relation_errors(request_value: dict, response_value: dict) -> list[str]:
        relation_errors = []
        if request_value.get("request_id") != response_value.get("request_id"):
            relation_errors.append("lifecycle IPCのrequest_idが一致しない")
        if request_value.get("operation") != response_value.get("operation"):
            relation_errors.append("lifecycle IPCのoperationが一致しない")
        if response_value.get("operation") == "実行系ライフサイクル操作":
            body = response_value.get("body")
            if not isinstance(body, dict) or "ライフサイクル監査ID" not in body:
                relation_errors.append("lifecycle操作成功応答に遷移結果がない")
                return relation_errors
            payload = request_value.get("payload")
            if not isinstance(payload, dict):
                relation_errors.append("lifecycle操作要求payloadがobjectでない")
                return relation_errors
            if body.get("実行系ID") != payload.get("実行系ID") or body.get("操作") != payload.get("操作"):
                relation_errors.append("lifecycle操作結果が要求Runtimeまたは操作と一致しない")
            if response_value.get("audit_event_id") != body.get("ライフサイクル監査ID"):
                relation_errors.append("lifecycle操作IPC監査IDが遷移結果と一致しない")
            if response_value.get("evidence_source") != body.get("証拠種別"):
                relation_errors.append("lifecycle操作IPC証拠種別が遷移結果と一致しない")
        return relation_errors

    errors.extend(ipc_relation_errors(ipc_request, ipc_response))
    invalid_ipc_response = copy.deepcopy(ipc_response)
    invalid_ipc_response["audit_event_id"] = "different-audit"
    if not ipc_relation_errors(ipc_request, invalid_ipc_response):
        errors.append("lifecycle操作IPCが監査ID不一致を受理する")
    invalid_ipc_response = copy.deepcopy(ipc_response)
    invalid_ipc_response["evidence_source"] = "INTERNAL_STATE"
    if not ipc_relation_errors(ipc_request, invalid_ipc_response):
        errors.append("lifecycle操作IPCが成功遷移の証拠種別不一致を受理する")

    for schema_name, schema in (
        ("ipc_request", ipc_request_schema),
        ("ipc_response", ipc_response_schema),
    ):
        operations = schema["properties"]["operation"]["enum"]
        missing = sorted(set(request_shapes) - set(operations))
        if missing:
            errors.append(f"{schema_name}がlifecycle operationを公開していない: {', '.join(missing)}")

    if command_schema["properties"]["dispatch_enabled"].get("const") is not False:
        errors.append("C4がgeneric command dispatchを有効化している")

    for path in (
        ROOT / "examples" / "adapter" / "blue_tanuki_reference_adapter.json",
        CONTRACT_EXAMPLES / "adapter.valid.json",
    ):
        adapter = json.loads(path.read_text(encoding="utf-8"))
        capabilities = adapter.get("declared_capabilities", [])
        if any("lifecycle" in capability.lower() for capability in capabilities if isinstance(capability, str)):
            errors.append(f"{path.name}がC4のためにlifecycle Capabilityを追加している")
    minidora_source = (RUST_HELPER / "src" / "adapters" / "minidora.rs").read_text(encoding="utf-8")
    if "RuntimeLifecycle" in minidora_source or "実行系ライフサイクル" in minidora_source:
        errors.append("MINIDORA Adapterがlifecycle Capabilityまたは制御経路を実装している")

    lifecycle_doc = DOC_SPECS / "runtime-lifecycle.md"
    if not lifecycle_doc.exists():
        errors.append("実行系ライフサイクルの仕様文書がない")
    else:
        text = lifecycle_doc.read_text(encoding="utf-8")
        for token in (
            "実行系ライフサイクル状態",
            "実行系ライフサイクル承認要求",
            "実行系ライフサイクル承認",
            "実行系ライフサイクル操作",
            "quarantined",
            "MINIDORA",
            "generic dispatch",
        ):
            if token not in text:
                errors.append(f"実行系ライフサイクル仕様に必須境界がない: {token}")
    return errors


def 対話契約の関係と表示境界を検査する() -> list[str]:
    from tooling.dialogue_contract_check import 要求関係検査, 応答関係検査, 比較関係検査

    要求 = load_contract_fixture("runtime_dialogue_request.valid.json")
    送信受付 = load_contract_fixture("runtime_dialogue_submission_receipt.valid.json")
    送信受付schema = load_schema("runtime_dialogue_submission_receipt.schema.json")
    セッション = load_contract_fixture("runtime_dialogue_session.valid.json")
    応答 = load_contract_fixture("runtime_dialogue_response.valid.json")
    応答schema = load_schema("runtime_dialogue_response.schema.json")
    不整合 = 要求関係検査(要求, セッション) + 応答関係検査(要求, 応答, "none")
    workspace_rejection = {
        **応答,
        "状態": "失敗",
        "失敗分類": "作業領域不在",
        "復旧": "RuntimeとWorkspaceの登録対応を再確認",
    }
    if validate_instance(workspace_rejection, 応答schema):
        不整合.append("Workspace結合拒否の失敗・復旧分類がresponse schemaにない")
    if validate_instance(送信受付, 送信受付schema):
        不整合.append("Broker送信受付receiptのvalid fixtureがschemaに適合しない")
    for 変更 in (
        {"要求ID": "not-an-id"},
        {"要求hash": "sha256:stale"},
        {"状態": "成功"},
        {"owner": True},
    ):
        if not validate_instance({**送信受付, **変更}, 送信受付schema):
            不整合.append("送信受付receiptが不正hash・状態またはauthority fieldを拒否しない")
    for 変更 in ({"入力": " "}, {"入力": "あ" * 4097}, {"実行系ID": "別実行系"},
               {"要求ID": "1" * 32 + "\n"}, {"実行系ID": "runtime-a\n"},
               {"対話セッションID": "3" * 32}, {"authority_source": "gui_state"}):
        if not 要求関係検査({**要求, **変更}, セッション):
            不整合.append("不正・越境した要求が許可された")
    if 要求関係検査({**要求, "入力": "あ" * 4096}, セッション):
        不整合.append("入力長上限の要求が拒否された")
    for 状態 in ("終了", "中止後隔離"):
        if not 要求関係検査(要求, {**セッション, "状態": 状態}):
            不整合.append("閉じたセッションが再利用された")
    for 長さ in (65536, 65537):
        結果 = 応答関係検査(要求, {**応答, "表示範囲": "full", "本文": "あ" * 長さ}, "full")
        if bool(結果) != (長さ > 65536):
            不整合.append("応答本文の長さ境界が不正")
    for 件数 in (64, 65):
        結果 = 応答関係検査(要求, {**応答, "表示範囲": "full", "参照": ["公開参照"] * 件数}, "full")
        if bool(結果) != (件数 > 64):
            不整合.append("参照数の上限境界が不正")
    for 鍵 in ("要求ID", "実行系ID", "対話セッションID"):
        if not 応答関係検査(要求, {**応答, 鍵: "3" * 32}, "none"):
            不整合.append("他要求の応答が採用された")
    for 範囲 in ("none", "hash_only", "summary", "redacted"):
        for 鍵, 値 in (("本文", "秘密"), ("参照", ["秘密"]), ("能力", ["秘密"]), ("経路", "秘密")):
            if not 応答関係検査(要求, {**応答, "表示範囲": 範囲, 鍵: 値}, 範囲):
                不整合.append("非全文表示へ未承認の内容が漏れた")
    if not 応答関係検査(要求, {**応答, "表示範囲": "full", "本文": "全文"}, "none"):
        不整合.append("表示範囲が自己昇格された")
    if 応答関係検査(要求, {**応答, "表示範囲": "full", "本文": "全文"}, "full"):
        不整合.append("許可済み全文の契約が拒否された")
    if not 応答関係検査(要求, {**応答, "状態": "失敗"}, "none"):
        不整合.append("復旧のない失敗結果が許可された")
    return 不整合


def 二実行系比較の非混線を検査する() -> list[str]:
    from tooling.dialogue_contract_check import 比較関係検査

    左要求 = load_contract_fixture("runtime_dialogue_request.valid.json")
    右要求 = {**左要求, "要求ID": "4" * 32, "実行系ID": "runtime-b", "対話セッションID": "5" * 32}
    左応答 = load_contract_fixture("runtime_dialogue_response.valid.json")
    右応答 = {**左応答, **{鍵: 右要求[鍵] for 鍵 in ("要求ID", "実行系ID", "対話セッションID")}}
    比較 = load_contract_fixture("runtime_dialogue_comparison.valid.json")
    比較["入力hash"] = "sha256:" + hashlib.sha256(左要求["入力"].encode("utf-8")).hexdigest()
    不整合 = []
    for 左状態, 右状態 in (("成功", "成功"), ("失敗", "成功"), ("成功", "失敗"), ("失敗", "失敗")):
        左 = {**左応答, "状態": 左状態, "失敗分類": "通信失敗" if 左状態 == "失敗" else "", "復旧": "接続再確認" if 左状態 == "失敗" else ""}
        右 = {**右応答, "状態": 右状態, "失敗分類": "通信失敗" if 右状態 == "失敗" else "", "復旧": "接続再確認" if 右状態 == "失敗" else ""}
        記録 = {**比較, "左状態": 左状態, "右状態": 右状態}
        if 比較関係検査(記録, 左要求, 右要求, 左, 右, "none", "none"):
            不整合.append("比較の独立した成功・失敗が拒否された")
        if not 比較関係検査(記録, 左要求, 右要求, 右, 左, "none", "none"):
            不整合.append("左右の応答が混線した")
    記録 = {**比較, "左状態": "成功", "右状態": "成功"}
    空白記録 = {**記録, "入力hash": "sha256:" + hashlib.sha256(b" ").hexdigest()}
    if not 比較関係検査(空白記録, {**左要求, "入力": " "}, {**右要求, "入力": " "}, 左応答, 右応答, "none", "none"):
        不整合.append("比較側で空白だけの入力が採用された")
    if not 比較関係検査(記録, 左要求, {**右要求, "入力": "別入力"}, 左応答, 右応答, "none", "none"):
        不整合.append("異なる比較入力が許可された")
    if not 比較関係検査(記録, 左要求, {**右要求, "対話セッションID": 左要求["対話セッションID"]}, 左応答, 右応答, "none", "none"):
        不整合.append("比較で同じセッションが共有された")
    if not 比較関係検査(記録, 左要求, 右要求, {**左応答, "表示範囲": "full", "本文": "全文"}, 右応答, "none", "full"):
        不整合.append("右側の表示許可が左へ転用された")
    return 不整合


def 評価ラボの契約と境界を検査する() -> list[str]:
    from tooling.evaluation_contract_check import (
        Dataset改版列検査,
        公開projection検査,
        実験検査,
        公開結果検査,
        比較検査,
        登録検査,
        結果検査,
    )

    不整合: list[str] = []
    specification = DOC_SPECS / "evaluation-lab.md"
    if not specification.exists():
        return ["C5評価ラボの日本語意味正本がない"]
    specification_text = specification.read_text(encoding="utf-8")
    try:
        canonical_index = json.loads((ROOT / "規定" / "正本索引.json").read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        不整合.append("C5評価ラボの日本語意味正本索引を読めない")
    else:
        current_sources = canonical_index.get("現行正本", [])
        if not any(item.get("path") == "docs/specs/evaluation-lab.md" for item in current_sources if isinstance(item, dict)):
            不整合.append("C5評価ラボの日本語意味正本が正本索引へ登録されていない")
    for token in (
        "運用観測",
        "Authority",
        "Permission",
        "Approval",
        "Audit",
        "release",
        "security",
        "LLM-as-judge",
        "normal IPC",
        "raw input",
        "immutable revision",
        "Dataset改版列検査",
        "計画監査ID",
        "異なるExperimentIDを混在",
        "結果状態=中止",
        "C6",
        "scope外",
    ):
        if token not in specification_text:
            不整合.append(f"C5評価ラボ仕様に必須境界がない: {token}")
    fixtures = {
        "dataset": load_contract_fixture("evaluation_dataset.valid.json"),
        "case": load_contract_fixture("evaluation_case.valid.json"),
        "evaluator": load_contract_fixture("evaluation_evaluator.valid.json"),
        "registration": load_contract_fixture("evaluation_dataset_registration.valid.json"),
        "experiment": load_contract_fixture("evaluation_experiment.valid.json"),
        "result": load_contract_fixture("evaluation_result.valid.json"),
        "public_result": load_contract_fixture("evaluation_public_result.valid.json"),
        "comparison": load_contract_fixture("evaluation_comparison.valid.json"),
    }
    schemas = {
        name: load_schema(f"evaluation_{name}.schema.json")
        for name in (
            "dataset",
            "case",
            "evaluator",
            "dataset_registration",
            "experiment",
            "result",
            "public_result",
            "comparison",
        )
    }
    schema_fixture_names = {
        "dataset": "dataset",
        "case": "case",
        "evaluator": "evaluator",
        "dataset_registration": "registration",
        "experiment": "experiment",
        "result": "result",
        "public_result": "public_result",
        "comparison": "comparison",
    }
    for schema_name, fixture_name in schema_fixture_names.items():
        failures = validate_instance(fixtures[fixture_name], schemas[schema_name])
        if failures:
            不整合.extend(f"{schema_name}のvalid fixtureが拒否された: {failure}" for failure in failures)
    expected_config_fields = {
        "exact_config": {"期待本文"},
        "contains_config": {"期待断片"},
        "regex_config": {"パターン"},
        "json_schema_config": {"期待Schema"},
        "reference_count_config": {"最小数", "最大数"},
        "route_config": {"期待経路"},
        "status_config": {"期待状態"},
        "capability_config": {"必要能力一覧"},
        "latency_threshold_config": {"最大Millis"},
    }
    registration_defs = schemas["dataset_registration"].get("$defs", {})
    for config_name, expected_fields in expected_config_fields.items():
        config_schema = registration_defs.get(config_name, {})
        if config_schema.get("additionalProperties") is not False:
            不整合.append(f"{config_name}がstrict additionalProperties:falseではない")
        if set(config_schema.get("properties", {})) != expected_fields:
            不整合.append(f"{config_name}のprivate設定keyがEvaluator実装と一致しない")
    runtime_case_summary = (
        schemas["dataset"]
        .get("properties", {})
        .get("Case一覧", {})
        .get("items", {})
    )
    if set(runtime_case_summary.get("properties", {})) != {"評価CaseID", "定義hash"}:
        不整合.append("normal IPCのDataset Case一覧がCaseIDと定義hashだけに限定されていない")
    if runtime_case_summary.get("additionalProperties") is not False:
        不整合.append("normal IPCのDataset Case一覧がstrictではない")

    public_values = (
        fixtures["dataset"],
        fixtures["case"],
        fixtures["evaluator"],
        fixtures["experiment"],
        fixtures["result"],
        fixtures["comparison"],
    )
    if 公開projection検査(*public_values):
        不整合.append("validな公開evaluation projectionにraw field検査の誤検知がある")
    for index, value in enumerate(public_values):
        raw = copy.deepcopy(value)
        raw["本文"] = "公開してはならない値"
        if not validate_instance(raw, schemas[("dataset", "case", "evaluator", "experiment", "result", "comparison")[index]]):
            不整合.append(f"公開projection[{index}]がraw本文をschemaで拒否しない")
        if not 公開projection検査(raw):
            不整合.append(f"公開projection[{index}]がraw本文を関係検査で拒否しない")

    registration = fixtures["registration"]
    if 登録検査(registration):
        不整合.append("validなowner評価Dataset登録が関係検査で拒否された")
    next_registration = copy.deepcopy(registration)
    next_registration["revision"] = 2
    if Dataset改版列検査([registration, next_registration]):
        不整合.append("同一DatasetIDの増加revision列が関係検査で拒否された")
    duplicate_registration_revision = copy.deepcopy(registration)
    if not Dataset改版列検査([registration, duplicate_registration_revision]):
        不整合.append("同じDatasetID/revisionの再登録が許可された")
    rollback_registration_revision = copy.deepcopy(registration)
    rollback_registration_revision["revision"] = 3
    if not Dataset改版列検査([rollback_registration_revision, registration]):
        不整合.append("同一DatasetIDのrevision後退が許可された")
    separate_dataset = copy.deepcopy(registration)
    separate_dataset["評価DatasetID"] = "3" * 32
    if Dataset改版列検査([next_registration, separate_dataset]):
        不整合.append("別DatasetIDの独立したrevision列が関係検査で拒否された")
    incompatible_config = copy.deepcopy(registration)
    incompatible_config["非公開CasePayload一覧"][0]["評価器設定"][0]["種類"] = "contains"
    if not 登録検査(incompatible_config):
        不整合.append("Evaluator種類とprivate設定の不一致が許可された")
    invalid_reference_range = copy.deepcopy(registration)
    reference_config = invalid_reference_range["非公開CasePayload一覧"][0]["評価器設定"][4]["設定"]
    reference_config["最小数"] = 3
    reference_config["最大数"] = 2
    if not 登録検査(invalid_reference_range):
        不整合.append("reference_countの逆転した上下限が許可された")
    external_ref = copy.deepcopy(registration)
    external_ref["非公開CasePayload一覧"][0]["評価器設定"][3]["設定"]["期待Schema"]["$ref"] = "https://example.invalid/schema.json"
    if not 登録検査(external_ref):
        不整合.append("json_schemaの外部refが許可された")

    invalid_llm_authority = copy.deepcopy(registration)
    invalid_llm_authority["評価方式"] = "LLM-as-judge"
    invalid_llm_authority["評価用途"] = "Authority"
    if not validate_instance(invalid_llm_authority, schemas["dataset_registration"]):
        不整合.append("LLM-as-judgeまたはAuthority用途がSchemaで許可された")
    if not 登録検査(invalid_llm_authority):
        不整合.append("LLM-as-judgeまたはAuthority用途が関係検査で許可された")

    unknown_evaluator = copy.deepcopy(fixtures["evaluator"])
    unknown_evaluator["種類"] = "llm_as_judge"
    if not validate_instance(unknown_evaluator, schemas["evaluator"]):
        不整合.append("未登録のEvaluator種類がSchemaで許可された")

    experiment = fixtures["experiment"]
    if 実験検査(experiment):
        不整合.append("validなExperimentが関係検査で拒否された")
    missing_plan_audit = copy.deepcopy(experiment)
    missing_plan_audit.pop("計画監査ID")
    if not validate_instance(missing_plan_audit, schemas["experiment"]):
        不整合.append("Experimentの計画監査IDがSchemaで必須ではない")
    if not 実験検査(missing_plan_audit):
        不整合.append("Experimentの計画監査ID欠落が関係検査で許可された")
    duplicate_runtime = copy.deepcopy(experiment)
    duplicate_runtime["対象Runtime一覧"] = ["runtime-a", "runtime-a"]
    if not 実験検査(duplicate_runtime):
        不整合.append("Experimentの同一Runtime重複が許可された")
    malformed_runtime = copy.deepcopy(experiment)
    malformed_runtime["対象Runtime一覧"][0] = {}
    if not 実験検査(malformed_runtime):
        不整合.append("Experimentのobject実行系IDが関係検査で許可された")
    incomplete_complete = copy.deepcopy(experiment)
    incomplete_complete["結果数"] = 1
    if not 実験検査(incomplete_complete):
        不整合.append("完了Experimentの不足Result数が許可された")
    overflow_complete = copy.deepcopy(experiment)
    overflow_complete["結果数"] = 3
    if not 実験検査(overflow_complete):
        不整合.append("完了Experimentの超過Result数が許可された")
    complete_without_finished_at = copy.deepcopy(experiment)
    complete_without_finished_at["終了時刻UnixMillis"] = None
    if not validate_instance(complete_without_finished_at, schemas["experiment"]):
        不整合.append("完了Experimentの終了時刻nullがSchemaで許可された")
    if not 実験検査(complete_without_finished_at):
        不整合.append("完了Experimentの終了時刻nullが関係検査で許可された")
    complete_without_started_at = copy.deepcopy(experiment)
    complete_without_started_at["開始時刻UnixMillis"] = None
    if not validate_instance(complete_without_started_at, schemas["experiment"]):
        不整合.append("完了Experimentの開始時刻nullがSchemaで許可された")
    if not 実験検査(complete_without_started_at):
        不整合.append("完了Experimentの開始時刻nullが関係検査で許可された")
    reversed_timestamps = copy.deepcopy(experiment)
    reversed_timestamps["開始時刻UnixMillis"] = 1726300000200
    reversed_timestamps["終了時刻UnixMillis"] = 1726300000100
    if not 実験検査(reversed_timestamps):
        不整合.append("Experimentの終了時刻が開始時刻より前でも関係検査で許可された")
    two_case_complete = copy.deepcopy(experiment)
    two_case_complete["計画Case数"] = 2
    two_case_complete["結果数"] = 4
    if 実験検査(two_case_complete):
        不整合.append("Case数2とRuntime数2の完了Experimentが関係検査で拒否された")
    two_case_incomplete = copy.deepcopy(two_case_complete)
    two_case_incomplete["結果数"] = 2
    if not 実験検査(two_case_incomplete):
        不整合.append("Case数2とRuntime数2の完了Experiment不足Result数が許可された")

    result = fixtures["result"]
    if 結果検査(result):
        不整合.append("validなResultが関係検査で拒否された")
    success_without_response_hash = copy.deepcopy(result)
    success_without_response_hash["応答hash"] = None
    if not validate_instance(success_without_response_hash, schemas["result"]):
        不整合.append("成功Resultが応答hashなしでもSchemaで許可された")
    if not 結果検査(success_without_response_hash):
        不整合.append("成功Resultが応答hashなしでも関係検査で許可された")
    non_success_empty_response_hash = copy.deepcopy(result)
    non_success_empty_response_hash["結果状態"] = "中止"
    non_success_empty_response_hash["判定"] = "中断"
    non_success_empty_response_hash["応答hash"] = ""
    if not validate_instance(non_success_empty_response_hash, schemas["result"]):
        不整合.append("非成功Resultが空の応答hashでもSchemaで許可された")
    if not 結果検査(non_success_empty_response_hash):
        不整合.append("非成功Resultが空の応答hashでも関係検査で許可された")
    non_abort_with_quarantine_reservation = copy.deepcopy(result)
    non_abort_with_quarantine_reservation["実行系隔離予約監査ID"] = "audit.lifecycle.quarantine.1"
    if not validate_instance(non_abort_with_quarantine_reservation, schemas["result"]):
        不整合.append("中止以外のResultが実行系隔離予約監査IDを持ってもSchemaで許可された")
    if not 結果検査(non_abort_with_quarantine_reservation):
        不整合.append("中止以外のResultが実行系隔離予約監査IDを持っても関係検査で許可された")
    unstarted_abort = copy.deepcopy(result)
    unstarted_abort["結果状態"] = "中止"
    unstarted_abort["判定"] = "中断"
    unstarted_abort["開始監査ID"] = None
    unstarted_abort["終了監査ID"] = None
    unstarted_abort["LatencyMillis"] = None
    if validate_instance(unstarted_abort, schemas["result"]) or 結果検査(unstarted_abort):
        不整合.append("未送信中断のnull相関を表せない")
    abort_with_wrong_determination = copy.deepcopy(result)
    abort_with_wrong_determination["結果状態"] = "中止"
    abort_with_wrong_determination["判定"] = "評価不能"
    if not validate_instance(abort_with_wrong_determination, schemas["result"]):
        不整合.append("中止結果が中断以外の判定でもSchemaで許可された")
    if not 結果検査(abort_with_wrong_determination):
        不整合.append("中止結果が中断以外の判定でも許可された")
    indeterminate_with_wrong_determination = copy.deepcopy(result)
    indeterminate_with_wrong_determination["結果状態"] = "評価不能"
    indeterminate_with_wrong_determination["判定"] = "中断"
    if not validate_instance(indeterminate_with_wrong_determination, schemas["result"]):
        不整合.append("評価不能結果が評価不能以外の判定でもSchemaで許可された")
    if not 結果検査(indeterminate_with_wrong_determination):
        不整合.append("評価不能結果が評価不能以外の判定でも許可された")
    for nonfinite in (float("nan"), float("inf")):
        invalid_latency = copy.deepcopy(result)
        invalid_latency["LatencyMillis"] = nonfinite
        if not validate_instance(invalid_latency, schemas["result"]):
            不整合.append("Resultの非有限LatencyMillisがSchemaで許可された")
        if not 結果検査(invalid_latency):
            不整合.append("Resultの非有限LatencyMillisが関係検査で許可された")

    public_result = fixtures["public_result"]
    if 公開結果検査(public_result):
        不整合.append("validな公開Resultが関係検査で拒否された")
    internal_reservation_in_public = copy.deepcopy(public_result)
    internal_reservation_in_public["実行系隔離予約監査ID"] = "audit.lifecycle.quarantine.1"
    if not validate_instance(internal_reservation_in_public, schemas["public_result"]):
        不整合.append("公開Resultへ内部隔離予約監査IDを混入できた")
    if not 公開結果検査(internal_reservation_in_public):
        不整合.append("公開Resultへ内部隔離予約監査IDを混入しても関係検査で許可された")

    comparison = fixtures["comparison"]
    if 比較検査(comparison):
        不整合.append("validなComparisonが関係検査で拒否された")
    mixed_experiment_ids = copy.deepcopy(comparison)
    mixed_experiment_ids["実験一覧"][1]["評価ExperimentID"] = "5" * 32
    if validate_instance(mixed_experiment_ids, schemas["comparison"]):
        不整合.append("異なるExperimentIDのComparisonがSchema形状として不正")
    if not 比較検査(mixed_experiment_ids):
        不整合.append("異なるExperimentIDを混在したComparisonが許可された")
    mismatched_dataset = copy.deepcopy(comparison)
    mismatched_dataset["実験一覧"][1]["Dataset定義hash"] = "sha256:" + "f" * 64
    if not 比較検査(mismatched_dataset):
        不整合.append("ComparisonのDataset定義hash不一致が許可された")
    duplicate_comparison_runtime = copy.deepcopy(comparison)
    duplicate_comparison_runtime["実験一覧"][1]["実行系ID"] = "runtime-a"
    if not 比較検査(duplicate_comparison_runtime):
        不整合.append("Comparisonの同一Runtime重複が許可された")
    malformed_comparison_runtime = copy.deepcopy(comparison)
    malformed_comparison_runtime["実験一覧"][0]["実行系ID"] = {}
    if not 比較検査(malformed_comparison_runtime):
        不整合.append("Comparisonのobject実行系IDが関係検査で許可された")
    entry_count_overflow = copy.deepcopy(comparison)
    entry_count_overflow["実験一覧"][0]["成立数"] = 2
    entry_count_overflow["実験一覧"][1]["成立数"] = 0
    if not 比較検査(entry_count_overflow):
        不整合.append("ComparisonのRuntime entry四分類超過が許可された")
    entry_count_underflow = copy.deepcopy(comparison)
    entry_count_underflow["実験一覧"][0]["成立数"] = 0
    entry_count_underflow["成立数"] = 1
    if not 比較検査(entry_count_underflow):
        不整合.append("ComparisonのRuntime entry四分類不足が許可された")
    two_case_comparison = copy.deepcopy(comparison)
    two_case_comparison["計画Case数"] = 2
    two_case_comparison["実験一覧"][0]["成立数"] = 2
    two_case_comparison["実験一覧"][1]["成立数"] = 2
    two_case_comparison["成立数"] = 4
    if 比較検査(two_case_comparison):
        不整合.append("Case数2のComparisonが関係検査で拒否された")
    two_case_underflow = copy.deepcopy(two_case_comparison)
    two_case_underflow["実験一覧"][0]["成立数"] = 1
    two_case_underflow["成立数"] = 3
    if not 比較検査(two_case_underflow):
        不整合.append("Case数2のComparison Runtime entry四分類不足が許可された")
    root_count_mismatch = copy.deepcopy(comparison)
    root_count_mismatch["成立数"] = 0
    if not 比較検査(root_count_mismatch):
        不整合.append("Comparisonのroot分類数とRuntime entry合計の不一致が許可された")
    nonfinite_average = copy.deepcopy(comparison)
    nonfinite_average["実験一覧"][0]["平均LatencyMillis"] = float("nan")
    if not validate_instance(nonfinite_average, schemas["comparison"]):
        不整合.append("Comparisonの非有限平均LatencyMillisがSchemaで許可された")
    if not 比較検査(nonfinite_average):
        不整合.append("Comparisonの非有限平均LatencyMillisが関係検査で許可された")
    return 不整合


def 回帰Caseの契約と境界を検査する() -> list[str]:
    不整合: list[str] = []
    specification = DOC_SPECS / "regression-case.md"
    if not specification.exists():
        return ["C6回帰Caseの日本語意味正本がない"]
    specification_text = specification.read_text(encoding="utf-8")
    try:
        canonical_index = json.loads((ROOT / "規定" / "正本索引.json").read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        不整合.append("C6回帰Caseの日本語意味正本索引を読めない")
    else:
        current_sources = canonical_index.get("現行正本", [])
        if not any(item.get("path") == "docs/specs/regression-case.md" for item in current_sources if isinstance(item, dict)):
            不整合.append("C6回帰Caseの日本語意味正本が正本索引へ登録されていない")
    for token in (
        "owner",
        "private",
        "ProtectedStore::Purpose::Regression",
        "hash_only",
        "要求hash",
        "結果証跡",
        "終了監査ID",
        "秘密",
        "C5",
        "Approval",
        "Audit",
        "RecoveryAction",
        "normal IPC",
        "自動コピー",
        "metadata_only",
        "通常IPC",
        "cursor",
        "削除承認Audit",
        "回帰Case削除中断確認",
        "物理消去",
        "LIVE_RUNTIME",
        "暗号文不在・中断照合済み",
        "暗号文残存・再試行可能",
    ):
        if token not in specification_text:
            不整合.append(f"C6回帰Case仕様に必須境界がない: {token}")

    registration = load_contract_fixture("regression_case_registration.valid.json")
    receipt = load_contract_fixture("regression_case_receipt.valid.json")
    listing = load_contract_fixture("regression_case_list.valid.json")
    registration_schema = load_schema("regression_case_registration.schema.json")
    receipt_schema = load_schema("regression_case_receipt.schema.json")
    list_request_schema = load_schema("regression_case_list_request.schema.json")
    list_schema = load_schema("regression_case_list.schema.json")
    for name, value, schema in (
        ("registration", registration, registration_schema),
        ("receipt", receipt, receipt_schema),
        ("metadata list", listing, list_schema),
    ):
        failures = validate_instance(value, schema)
        if failures:
            不整合.extend(f"C6 {name} valid fixtureが拒否された: {failure}" for failure in failures)

    if any(key in registration for key in ("permission_id", "approval_id", "authority", "credential")):
        不整合.append("C6登録payloadへ権限fieldが混入している")
    if any(key in receipt for key in ("入力", "本文", "必要条件", "禁止条件", "必要参照", "期待経路")):
        不整合.append("C6公開receiptへprivate本文が混入している")

    auto_copy = copy.deepcopy(registration)
    auto_copy["入力方式"] = "auto_from_dialogue"
    if not validate_instance(auto_copy, registration_schema):
        不整合.append("C6が対話自動コピー方式をschemaで拒否しない")
    authority = copy.deepcopy(registration)
    authority["approval_id"] = "owner-injected"
    if not validate_instance(authority, registration_schema):
        不整合.append("C6登録payloadのapproval_id混入を拒否しない")
    raw_receipt = copy.deepcopy(receipt)
    raw_receipt["入力"] = {"本文": "公開してはならない"}
    if not validate_instance(raw_receipt, receipt_schema):
        不整合.append("C6公開receiptのraw入力混入を拒否しない")

    for request in (
        {"版": 1, "after": 0, "limit": 100},
        {"版": 1, "after": 12, "limit": 1},
    ):
        if validate_instance(request, list_request_schema):
            不整合.append("C6ページ要求の正常なcursor/limitを拒否した")
    for request in (
        {"版": 1, "after": -1, "limit": 10},
        {"版": 1, "after": 0, "limit": 101},
        {"版": 1, "after": 0, "limit": 10, "approval_id": "injected"},
    ):
        if not validate_instance(request, list_request_schema):
            不整合.append("C6ページ要求が負数・上限超過・権限fieldを許可した")
    private_listing = copy.deepcopy(listing)
    private_listing["回帰Case一覧"][0]["期待経路"] = "must-not-be-returned"
    if not validate_instance(private_listing, list_schema):
        不整合.append("C6 metadata一覧がprivate fieldの混入を許可した")

    def list_page_matches(request: dict, response: dict) -> bool:
        items = response.get("回帰Case一覧")
        if not isinstance(items, list):
            return False
        after = request.get("after")
        limit = request.get("limit")
        count = response.get("件数")
        total = response.get("合計件数")
        if not all(isinstance(value, int) and not isinstance(value, bool) for value in (after, limit, count, total)):
            return False
        if count != len(items) or count > limit or after + count > total:
            return False
        expected_next = after + count if after + count < total else None
        return response.get("次cursor") == expected_next

    list_request = {"版": 1, "after": 0, "limit": 100}
    if not list_page_matches(list_request, listing):
        不整合.append("C6 metadata一覧の件数またはcursor関係が不正")
    count_mismatch = copy.deepcopy(listing)
    count_mismatch["件数"] = 0
    if list_page_matches(list_request, count_mismatch):
        不整合.append("C6 metadata一覧の件数不一致を関係検査が許可した")
    next_cursor_mismatch = copy.deepcopy(listing)
    next_cursor_mismatch["合計件数"] = 2
    next_cursor_mismatch["次cursor"] = None
    if list_page_matches(list_request, next_cursor_mismatch):
        不整合.append("C6 metadata一覧の次cursor欠落を関係検査が許可した")

    deletion_request = load_contract_fixture("regression_case_delete_request.valid.json")
    deletion_receipt = load_contract_fixture("regression_case_delete_receipt.valid.json")
    recovery_request = load_contract_fixture("regression_case_delete_recovery_request.valid.json")
    recovery_receipt = load_contract_fixture("regression_case_delete_recovery_receipt.valid.json")
    deletion_request_schema = load_schema("regression_case_delete_request.schema.json")
    deletion_receipt_schema = load_schema("regression_case_delete_receipt.schema.json")
    recovery_request_schema = load_schema("regression_case_delete_recovery_request.schema.json")
    recovery_receipt_schema = load_schema("regression_case_delete_recovery_receipt.schema.json")
    for name, value, schema in (
        ("deletion request", deletion_request, deletion_request_schema),
        ("deletion receipt", deletion_receipt, deletion_receipt_schema),
        ("recovery request", recovery_request, recovery_request_schema),
        ("recovery receipt", recovery_receipt, recovery_receipt_schema),
    ):
        failures = validate_instance(value, schema)
        if failures:
            不整合.extend(f"C6 {name} valid fixtureが拒否された: {failure}" for failure in failures)

    for invalid_name, schema in (
        ("regression_case_delete_request_authority.invalid.json", deletion_request_schema),
        ("regression_case_delete_receipt_evidence.invalid.json", deletion_receipt_schema),
        ("regression_case_delete_receipt_physical_erasure.invalid.json", deletion_receipt_schema),
        ("regression_case_delete_recovery_request_authority.invalid.json", recovery_request_schema),
        ("regression_case_delete_recovery_request_stale_approval.invalid.json", recovery_request_schema),
        ("regression_case_delete_recovery_receipt_evidence.invalid.json", recovery_receipt_schema),
        ("regression_case_list_deleted_claim.invalid.json", list_schema),
    ):
        invalid = json.loads((INVALID_CONTRACT_EXAMPLES / invalid_name).read_text(encoding="utf-8"))
        if not validate_instance(invalid, schema):
            不整合.append(f"C6否定fixtureをSchemaが許可した: {invalid_name}")

    false_physical_erasure = copy.deepcopy(deletion_receipt)
    false_physical_erasure["物理消去"] = True
    if not validate_instance(false_physical_erasure, deletion_receipt_schema):
        不整合.append("C6削除結果が物理消去主張を拒否しない")
    authority_deletion = copy.deepcopy(deletion_request)
    authority_deletion["approval_id"] = "injected"
    if not validate_instance(authority_deletion, deletion_request_schema):
        不整合.append("C6削除要求がapproval_idを許可した")

    request_contract = json.loads((SPECS / "ipc_request.schema.json").read_text(encoding="utf-8"))
    response_contract = json.loads((SPECS / "ipc_response.schema.json").read_text(encoding="utf-8"))
    for operation in ("回帰Case一覧", "回帰Case削除", "回帰Case削除中断確認"):
        if operation not in request_contract["properties"]["operation"]["enum"]:
            不整合.append(f"C6 {operation} operationがIPC request contractへ未接続")
        if operation not in response_contract["properties"]["operation"]["enum"]:
            不整合.append(f"C6 {operation} operationがIPC response contractへ未接続")

    rust = (RUST_HELPER / "src" / "broker" / "regression_case.rs").read_text(encoding="utf-8")
    owner_cli = (RUST_HELPER / "src" / "owner_cli.rs").read_text(encoding="utf-8")
    dialogue = (RUST_HELPER / "src" / "broker" / "dialogue.rs").read_text(encoding="utf-8")
    protected = (RUST_HELPER / "src" / "protected_store.rs").read_text(encoding="utf-8")
    for token, source in (
        ("Purpose::Regression", rust),
        ("回帰Case情報", rust + dialogue),
        ("保存済み結果証跡", rust + dialogue),
        ("終了監査ID", rust + dialogue),
        ("hash_only", rust),
        ("秘密候補入力", rust),
        ("回帰Case一覧処理", rust),
        ("stored_regression_cases", rust),
        ("metadata_projection", rust),
        ("store.inspect", rust),
        ("回帰Case削除処理", rust),
        ("回帰Case削除中断確認処理", rust),
        ("stored_regression_case_deletions", rust),
        ("prepare_delete", rust),
        ("prepared.commit()", rust),
        ("回帰Case削除", owner_cli),
        ("回帰Case削除中断確認", owner_cli),
        ("Regression", protected),
    ):
        if token not in source:
            不整合.append(f"C6実装に統治境界tokenがない: {token}")
    if "Purpose::Evaluation" in rust:
        不整合.append("C6がC5 Evaluation purposeへ混入している")
    if "unprotect(" in rust:
        不整合.append("C6 metadata一覧経路がprivate定義を復号している")
    return 不整合


def 資格情報保管庫の契約と境界を検査する() -> list[str]:
    不整合: list[str] = []
    specification = DOC_SPECS / "credential-vault.md"
    if not specification.exists():
        return ["C7資格情報保管庫の日本語意味正本がない"]
    specification_text = specification.read_text(encoding="utf-8")
    try:
        canonical_index = json.loads((ROOT / "規定" / "正本索引.json").read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        不整合.append("C7資格情報保管庫の日本語意味正本索引を読めない")
    else:
        current_sources = canonical_index.get("現行正本", [])
        if not any(item.get("path") == "docs/specs/credential-vault.md" for item in current_sources if isinstance(item, dict)):
            不整合.append("C7資格情報保管庫の日本語意味正本が正本索引へ登録されていない")
    for token in (
        "owner control",
        "ProtectedStore / DPAPI",
        "Purpose::Credential",
        "秘密値",
        "metadata_only",
        "normal IPC",
        "Capability",
        "Permission",
        "Approval",
        "Audit",
        "Recovery",
        "更新",
        "失効",
        "削除",
        "接続先変更",
        "release_blocker",
    ):
        if token not in specification_text:
            不整合.append(f"C7資格情報保管庫仕様に必須境界がない: {token}")

    registration = load_contract_fixture("credential_registration.valid.json")
    revocation = load_contract_fixture("credential_revocation_request.valid.json")
    receipt = load_contract_fixture("credential_receipt.valid.json")
    listing = load_contract_fixture("credential_list.valid.json")
    registration_schema = load_schema("credential_registration.schema.json")
    revocation_schema = load_schema("credential_revocation_request.schema.json")
    receipt_schema = load_schema("credential_receipt.schema.json")
    list_schema = load_schema("credential_list.schema.json")
    for name, value, schema in (
        ("registration", registration, registration_schema),
        ("revocation", revocation, revocation_schema),
        ("receipt", receipt, receipt_schema),
        ("list", listing, list_schema),
    ):
        failures = validate_instance(value, schema)
        if failures:
            不整合.extend(f"C7 {name} valid fixtureが拒否された: {failure}" for failure in failures)
    if any(key in registration for key in ("permission_id", "approval_id", "authority", "capability_id")):
        不整合.append("C7登録payloadへ権限fieldが混入している")
    if any(key in receipt for key in ("秘密値", "secret", "token", "password", "credential_value")):
        不整合.append("C7公開receiptへ秘密値が混入している")
    if not validate_instance({**registration, "permission_id": "injected"}, registration_schema):
        不整合.append("C7登録payloadの権限field混入を拒否しない")
    if not validate_instance({**receipt, "秘密値": "must-not-be-projected"}, receipt_schema):
        不整合.append("C7公開receiptの秘密値混入を拒否しない")
    if any(key in revocation for key in ("秘密値", "credential", "authority", "permission_id", "approval_id")):
        不整合.append("C7失効要求へ秘密値または権限fieldが混入している")
    if not validate_instance({**revocation, "秘密値": "must-not-enter-request"}, revocation_schema):
        不整合.append("C7失効要求の秘密値混入を拒否しない")
    revoked_receipt = {**receipt, "状態": "失効", "失効時刻UnixMillis": 1780000000001}
    if validate_instance(revoked_receipt, receipt_schema):
        不整合.append("C7失効済みreceiptをmetadata schemaが受理しない")
    if not validate_instance({**revoked_receipt, "失効時刻UnixMillis": None}, receipt_schema):
        不整合.append("C7失効済みreceiptの失効時刻欠落を拒否しない")
    if listing.get("件数") != len(listing.get("資格情報一覧", [])):
        不整合.append("C7一覧valid fixtureの件数関係が不正")

    rust = (RUST_HELPER / "src" / "broker" / "credential_vault.rs").read_text(encoding="utf-8")
    protocol = (RUST_HELPER / "src" / "broker" / "protocol.rs").read_text(encoding="utf-8")
    launcher = (RUST_HELPER / "src" / "desktop_launcher.rs").read_text(encoding="utf-8")
    owner_cli = (RUST_HELPER / "src" / "owner_cli.rs").read_text(encoding="utf-8")
    broker_client = (DESKTOP_FLUTTER / "lib" / "services" / "broker_client.dart").read_text(encoding="utf-8")
    credential_storage = (RUST_HELPER / "src" / "broker" / "credential_storage.rs").read_text(encoding="utf-8")
    for token, source in (
        ("Purpose::Credential", credential_storage),
        ("資格情報保存先()", rust),
        (".読取", rust),
        ("credential_owner_required", rust),
        ("credential_normal_channel_required", rust),
        ("metadata_only", rust),
        ("秘密値", rust + owner_cli),
        ("新規資格情報暗号文を破棄", rust),
        ("credential_storage_missing", rust),
        ("credential_storage_changed", rust),
        ("credential_ledger", rust),
        ("資格情報失効処理", rust + protocol),
        ("desktop_native_owner_confirmation_required", protocol),
        ("CredentialRevocation", launcher),
        ("DEFBUTTON2", launcher),
        ("資格情報失効", broker_client),
        ("資格情報公開receiptへ秘密値が混入している", owner_cli),
    ):
        if token not in source:
            不整合.append(f"C7実装に統治境界tokenがない: {token}")
    if 'println!("{}", registration.secret' in rust:
        不整合.append("C7が秘密値を直接表示している")
    return 不整合


def MCP外部概念射影の契約と境界を検査する() -> list[str]:
    不整合: list[str] = []
    specification = DOC_SPECS / "mcp-contract.md"
    if not specification.exists():
        return ["C8 MCP外部概念射影の日本語意味正本がない"]
    specification_text = specification.read_text(encoding="utf-8")
    required_tokens = (
        "Server",
        "Tool",
        "Resource",
        "Prompt",
        "Transport",
        "Credential ref",
        "Trust",
        "Capability diff",
        "MCP metadata ≠ Authority",
        "Tool description ≠ Permission",
        "Trust ≠ Approval",
        "Capability diff ≠ Permission grant",
        "Credential ref ≠ Credential value",
        "権限生成",
        "metadata_only",
        "release_blocker",
    )
    for token in required_tokens:
        if token not in specification_text:
            不整合.append(f"C8 MCP契約の日本語意味正本に必須境界がない: {token}")
    try:
        canonical_index = json.loads((ROOT / "規定" / "正本索引.json").read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        不整合.append("C8 MCP契約の日本語意味正本索引を読めない")
    else:
        current_sources = canonical_index.get("現行正本", [])
        if not any(item.get("path") == "docs/specs/mcp-contract.md" for item in current_sources if isinstance(item, dict)):
            不整合.append("C8 MCP契約の日本語意味正本が正本索引へ登録されていない")

    schema = load_schema("mcp_contract.schema.json")
    valid = load_contract_fixture("mcp_contract.valid.json")
    failures = validate_instance(valid, schema)
    if failures:
        不整合.extend(f"C8 valid fixtureが拒否された: {failure}" for failure in failures)
    if valid.get("権限生成") != "なし" or valid.get("公開範囲") != "metadata_only":
        不整合.append("C8 valid fixtureがmetadata_onlyかつ権限生成なしではない")
    if valid.get("Trust", {}).get("state") == "verified":
        不整合.append("C8 valid fixtureがMCP metadataからverified Trustを主張している")
    capability_diff = valid.get("Capability diff", {})
    if capability_diff.get("status") in {"added", "changed", "removed"} and not capability_diff.get("requires_operator_review"):
        不整合.append("C8 Capability diffの追加・変更・削除にoperator reviewが必要")
    credential = valid.get("Credential ref", {})
    if any(key in credential for key in ("secret", "secret_value", "token", "password", "credential_value")):
        不整合.append("C8 Credential refへ実値が混入している")

    invalid_names = (
        "mcp_contract_tool_permission.invalid.json",
        "mcp_contract_credential_secret.invalid.json",
        "mcp_contract_authority.invalid.json",
    )
    for name in invalid_names:
        try:
            invalid = load_contract_fixture(f"invalid/{name}")
        except (OSError, json.JSONDecodeError) as exc:
            不整合.append(f"C8 {name}を読めない: {exc}")
            continue
        if not validate_instance(invalid, schema):
            不整合.append(f"C8 {name}を受理している")
    return 不整合


def A2A外部概念射影の契約と境界を検査する() -> list[str]:
    不整合: list[str] = []
    specification = DOC_SPECS / "a2a-contract.md"
    if not specification.exists():
        return ["C15 A2A外部概念射影の日本語意味正本がない"]
    specification_text = specification.read_text(encoding="utf-8")
    required_tokens = (
        "Agent Card",
        "Task",
        "Message",
        "Artifact",
        "Stream",
        "Agent Card ≠ Trust",
        "A2A Task ≠ Approval",
        "Message ≠ Permission",
        "Artifact ≠ Authority",
        "Stream ≠ Capability grant",
        "authority_strip=true",
        "権限生成=なし",
        "metadata_only",
        "FIXTURE",
        "release_blocker",
    )
    for token in required_tokens:
        if token not in specification_text:
            不整合.append(f"C15 A2A契約の日本語意味正本に必須境界がない: {token}")

    try:
        canonical_index = json.loads((ROOT / "規定" / "正本索引.json").read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        不整合.append("C15 A2A契約の日本語意味正本索引を読めない")
    else:
        current_sources = canonical_index.get("現行正本", [])
        if not any(item.get("path") == "docs/specs/a2a-contract.md" for item in current_sources if isinstance(item, dict)):
            不整合.append("C15 A2A契約の日本語意味正本が正本索引へ登録されていない")

    schema = load_schema("a2a_contract.schema.json")
    valid = load_contract_fixture("a2a_contract.valid.json")
    failures = validate_instance(valid, schema)
    if failures:
        不整合.extend(f"C15 valid fixtureが拒否された: {failure}" for failure in failures)
    if valid.get("権限生成") != "なし" or valid.get("authority_strip") is not True:
        不整合.append("C15 valid fixtureが権限非生成またはauthority_strip=trueではない")
    if valid.get("公開範囲") != "metadata_only" or valid.get("証拠種別") != "FIXTURE":
        不整合.append("C15 valid fixtureの公開範囲または証拠種別が境界外である")
    if valid.get("Trust", {}).get("state") == "verified":
        不整合.append("C15 Agent Cardがoperator review前にverified Trustを主張している")
    if valid.get("Trust", {}).get("requires_operator_review") is not True:
        不整合.append("C15 Trustにoperator review要求がない")
    card = valid.get("Agent Card", {})
    if "endpoint" in card or "endpoint_url" in card:
        不整合.append("C15 Agent Cardがendpoint実値を保持している")
    authentication = card.get("authentication", {})
    if authentication.get("secret_value_present") is not False:
        不整合.append("C15 Agent Cardが秘密値の存在を許可している")
    for concept in ("Task", "Message", "Artifact", "Stream"):
        records = valid.get(concept, [])
        if not isinstance(records, list):
            不整合.append(f"C15 {concept}がbounded arrayではない")
            continue
        for record in records:
            if record.get("authority_strip") is not True:
                不整合.append(f"C15 {concept}にauthority_strip=trueがない")

    candidates = []
    authority_candidate = copy.deepcopy(valid)
    authority_candidate["権限生成"] = "filesystem.write"
    candidates.append(("権限生成", authority_candidate))
    raw_candidate = copy.deepcopy(valid)
    raw_candidate["Message"][0]["raw_content"] = "secret raw content"
    candidates.append(("raw_content", raw_candidate))
    secret_candidate = copy.deepcopy(valid)
    secret_candidate["Agent Card"]["authentication"]["secret_value_present"] = True
    candidates.append(("secret_value_present", secret_candidate))
    for name, candidate in candidates:
        if not validate_instance(candidate, schema):
            不整合.append(f"C15 {name}混入をschemaが受理している")

    invalid_names = (
        "a2a_contract_authority_escalation.invalid.json",
        "a2a_contract_raw_content.invalid.json",
        "a2a_contract_secret.invalid.json",
    )
    for name in invalid_names:
        try:
            invalid = load_contract_fixture(f"invalid/{name}")
        except (OSError, json.JSONDecodeError) as exc:
            不整合.append(f"C15 {name}を読めない: {exc}")
            continue
        if not validate_instance(invalid, schema):
            不整合.append(f"C15 {name}を受理している")
    return 不整合


def MCP接続センターの統治経路と境界を検査する() -> list[str]:
    不整合: list[str] = []
    specification = DOC_SPECS / "mcp-connection-center.md"
    if not specification.exists():
        return ["C9 MCP接続センターの日本語意味正本がない"]
    specification_text = specification.read_text(encoding="utf-8")
    required_tokens = (
        "server/discover",
        "initialize",
        "stdio",
        "owner control",
        "metadata_only",
        "Credential実値",
        "Tool実行",
        "MCP接続一覧",
        "MCP切断",
        "Windows Job Object",
        "LIVE_RUNTIME",
        "mcp_process_tree_supervision_unsupported",
        "mcp_server_unavailable",
        "mcp_timeout",
        "改行到着前から256 KiBを上限として逐次読取り",
        "Tool引数の検査と呼出し",
        "142桁",
        "32 KiB以下・2048 node以下・深さ32以下",
        "Rust Desktop native Owner確認",
        "tools/call",
        "一回限りPermission",
        "hash-only",
        "quarantined",
        "release_blocker",
    )
    for token in required_tokens:
        if token not in specification_text:
            不整合.append(f"C9 MCP接続センター正本に必須境界がない: {token}")
    try:
        canonical_index = json.loads((ROOT / "規定" / "正本索引.json").read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        不整合.append("C9 MCP接続センター正本索引を読めない")
    else:
        current_sources = canonical_index.get("現行正本", [])
        if not any(item.get("path") == "docs/specs/mcp-connection-center.md" for item in current_sources if isinstance(item, dict)):
            不整合.append("C9 MCP接続センター正本が正本索引へ登録されていない")

    for name in (
        "mcp_connection", "mcp_connection_receipt", "mcp_connection_list",
        "mcp_disconnect", "mcp_disconnect_receipt", "mcp_tool_call",
        "mcp_tool_call_receipt",
    ):
        schema = load_schema(name + ".schema.json")
        valid = load_contract_fixture(name + ".valid.json")
        failures = validate_instance(valid, schema)
        if failures:
            不整合.extend(f"C9 {name} valid fixtureが拒否された: {failure}" for failure in failures)
    configured_credential = load_contract_fixture("mcp_connection_credential.valid.json")
    if validate_instance(configured_credential, load_schema("mcp_connection.schema.json")):
        不整合.append("C9 configured Credential接続要求fixtureが拒否された")
    invalid_names = (
        "mcp_connection_unknown_authority.invalid.json",
        "mcp_connection_credential_secret.invalid.json",
        "mcp_connection_credential_missing_environment.invalid.json",
        "mcp_connection_tool_unknown_field.invalid.json",
        "mcp_connection_resource_uri_leak.invalid.json",
        "mcp_connection_prompt_description.invalid.json",
        "mcp_connection_receipt_full_content.invalid.json",
        "mcp_connection_list_wrong_evidence.invalid.json",
        "mcp_connection_list_entry_live_evidence.invalid.json",
        "mcp_disconnect_authority.invalid.json",
        "mcp_disconnect_receipt_not_stopped.invalid.json",
        "mcp_disconnect_receipt_wrong_evidence.invalid.json",
        "mcp_tool_call_authority.invalid.json",
        "mcp_tool_call_bad_tool_id.invalid.json",
        "mcp_tool_call_unknown_field.invalid.json",
        "mcp_tool_call_receipt_full_content.invalid.json",
    )
    for name in invalid_names:
        invalid = load_contract_fixture("invalid/" + name)
        schema_name = (
            "mcp_connection" if name.startswith(("mcp_connection_unknown", "mcp_connection_credential")) else
            "mcp_connection_receipt" if name.startswith("mcp_connection_receipt") else
            "mcp_connection_receipt" if name.startswith(("mcp_connection_tool", "mcp_connection_resource", "mcp_connection_prompt")) else
            "mcp_connection_list" if name.startswith("mcp_connection_list") else
            "mcp_disconnect" if name.startswith("mcp_disconnect_authority") else
            "mcp_tool_call" if name.startswith("mcp_tool_call_") and not name.startswith("mcp_tool_call_receipt") else
            "mcp_tool_call_receipt" if name.startswith("mcp_tool_call_receipt") else
            "mcp_disconnect_receipt"
        )
        if not validate_instance(invalid, load_schema(schema_name + ".schema.json")):
            不整合.append(f"C9 {name}を受理している")

    receipt = load_contract_fixture("mcp_connection_receipt.valid.json")
    if receipt.get("権限生成") != "なし" or receipt.get("公開範囲") != "metadata_only":
        不整合.append("C9 receiptが権限非生成またはmetadata_onlyではない")
    if receipt.get("承認状態") != "owner_control_approved" or receipt.get("接続状態") != "connected":
        不整合.append("C9 receiptのBroker統治状態が固定されていない")
    encoded_receipt = json.dumps(receipt, ensure_ascii=False)
    if any(token in encoded_receipt for token in ("secret_value", "credential_value", "password", "token")):
        不整合.append("C9 receiptへCredential実値が混入している")
    tool_call_request = load_contract_fixture("mcp_tool_call.valid.json")
    if (
        not isinstance(tool_call_request.get("ToolID"), str)
        or len(tool_call_request["ToolID"]) != 147
        or not tool_call_request["ToolID"].startswith("tool-")
    ):
        不整合.append("C9 MCP Tool実行要求のTool ID形式が現行Catalog生成形式と一致しない")
    tools = receipt.get("Tool")
    if not isinstance(tools, list) or any(
        not isinstance(tool, dict)
        or tool.get("description_summary") != ""
        or tool.get("risk") != "unknown"
        or set(tool) != {"tool_id", "name", "description_summary", "input_schema_hash", "risk", "status"}
        for tool in tools
    ):
        不整合.append("C9 Tool receiptがmetadata-only固定fieldではない")
    resources = receipt.get("Resource")
    if not isinstance(resources, list) or any(
        not isinstance(resource, dict)
        or set(resource) != {"resource_id", "name", "uri_template_hash", "mime_type", "status"}
        or "uri" in resource
        for resource in resources
    ):
        不整合.append("C9 Resource receiptへURI実値または未知fieldが混入している")
    prompts = receipt.get("Prompt")
    if not isinstance(prompts, list) or any(
        not isinstance(prompt, dict)
        or set(prompt) != {"prompt_id", "name", "description_summary", "argument_schema_hash", "status"}
        or prompt.get("description_summary") != ""
        for prompt in prompts
    ):
        不整合.append("C9 Prompt receiptへ説明本文または未知fieldが混入している")

    disconnect = load_contract_fixture("mcp_disconnect_receipt.valid.json")
    if disconnect.get("証拠種別") != "LIVE_RUNTIME" or disconnect.get("権限生成") != "なし":
        不整合.append("C9 MCP切断receiptがLIVE_RUNTIMEまたは権限非生成ではない")
    if any(token in json.dumps(disconnect, ensure_ascii=False) for token in ("実行file", "workspace", "credential_value", "secret_value")):
        不整合.append("C9 MCP切断receiptへ接続設定または秘密値が混入している")

    connection_list = load_contract_fixture("mcp_connection_list.valid.json")
    listed_connections = connection_list.get("MCP接続一覧")
    if (
        connection_list.get("証拠種別") != "INTERNAL_STATE"
        or not isinstance(listed_connections, list)
        or len(listed_connections) != connection_list.get("件数")
        or any(item.get("証拠種別") != "INTERNAL_STATE" for item in listed_connections)
    ):
        不整合.append("C9 MCP接続一覧の保存状態証拠と件数が正本に一致しない")

    rust_mcp = (RUST_HELPER / "src" / "mcp.rs").read_text(encoding="utf-8")
    rust_stdio = (RUST_HELPER / "src" / "adapters" / "mcp_stdio.rs").read_text(encoding="utf-8")
    rust_center = (RUST_HELPER / "src" / "broker" / "mcp_center.rs").read_text(encoding="utf-8")
    for token, source in (
        ("server/discover", rust_mcp + rust_stdio),
        ("mcp_metadata_authority_injection", rust_mcp),
        ("mcp_list_pagination_unhandled", rust_mcp),
        ("validate_tool_call", rust_mcp),
        ("MAX_TOOL_ARGUMENT_BYTES", rust_mcp),
        ("MAX_TOOL_ARGUMENT_NODES", rust_mcp),
        ("MAX_TOOL_ARGUMENT_DEPTH", rust_mcp),
        ("mcp_tool_arguments_schema_invalid", rust_mcp),
        ("validate_tool_identity_and_call", rust_mcp),
        ("mcp_tool_identity_mismatch", rust_mcp),
        ("summarize_tool_call_result", rust_mcp),
        ("mcp_tool_result_type_unsupported", rust_mcp),
        ("validator.is_valid(arguments)", rust_mcp),
        ("env_clear", rust_stdio),
        ("mcp_timeout", rust_stdio),
        ("read_bounded_line", rust_stdio),
        ("fill_buf", rust_stdio),
        ("mcp_wire_oversized", rust_stdio),
        ("McpProtocolEra::Legacy", rust_stdio),
        ("mcp_owner_required", rust_center),
        ("mcp_normal_channel_required", rust_center),
        ("mcp_credential_unavailable", rust_center),
        ("資格情報MCP使用処理", rust_center),
        ("資格情報MCP使用確定処理", rust_center),
        ("safe_credential_environment_name", rust_center),
        ("connect_with_environment", rust_center),
        ("append_audit", rust_center),
        ("metadata_only", rust_center),
        ("super::process_tree::spawn", rust_stdio),
        ("terminate_tree", rust_stdio),
        ("mcp_process_termination_failed", rust_stdio),
        ("OP_DISCONNECT", rust_center),
        ("mcp_process_tree_supervision_unsupported", rust_center),
        ("EVIDENCE_SOURCE_LIVE_RUNTIME", rust_center),
        ("terminate()", rust_center),
        ("mcp_connections.remove", rust_center),
        ("MCP tools/callを同一接続へ一度だけ送信", rust_center),
        ("mcp_tool_result_unknown", rust_center),
        ("quarantine(entry)", rust_center),
    ):
        if token not in source:
            不整合.append(f"C9実装に統治境界tokenがない: {token}")
    if "read_until(b'\\n'" in rust_stdio or 'read_until(b"\\n"' in rust_stdio:
        不整合.append("C9 MCP stdio readerが行上限前に無制限蓄積するread_untilを使用している")
    rust_protocol = (RUST_HELPER / "src" / "broker" / "protocol.rs").read_text(encoding="utf-8")
    owner_cli = (RUST_HELPER / "src" / "owner_cli.rs").read_text(encoding="utf-8")
    main = (RUST_HELPER / "src" / "main.rs").read_text(encoding="utf-8")
    desktop_launcher = (RUST_HELPER / "src" / "desktop_launcher.rs").read_text(encoding="utf-8")
    desktop_broker_client = (ROOT / "apps" / "desktop_flutter" / "lib" / "services" / "broker_client.dart").read_text(encoding="utf-8")
    mcp_client = (ROOT / "apps" / "desktop_flutter" / "lib" / "services" / "mcp_connection_client.dart").read_text(encoding="utf-8")
    mcp_screen = (ROOT / "apps" / "desktop_flutter" / "lib" / "screens" / "mcp_connection_center.dart").read_text(encoding="utf-8")
    for token, source in (
        ("BrokerOperation::MCP切断", rust_protocol),
        ("mcp_center::disconnect", rust_protocol),
        ("BrokerOperation::MCPTool実行", rust_protocol),
        ("mcp_center::call_tool", rust_protocol),
        ("pub fn MCP切断", owner_cli),
        ("owner_cli::MCP切断", main),
        ("BrokerOperation::MCP接続", desktop_launcher),
        ("BrokerOperation::MCPTool実行", desktop_launcher),
        ("McpToolCallOwnerRequest", desktop_launcher),
        ("McpConnectOwnerRequest", desktop_launcher),
        ("'MCP接続',", desktop_broker_client),
        ("'MCP Tool実行',", desktop_broker_client),
        ("Future<McpConnectionSummary> connect", mcp_client),
        ("Future<List<McpCredentialSummary>> listCredentials", mcp_client),
        ("Future<McpToolCallReceipt> callTool", mcp_client),
        ("class McpToolSummary", mcp_client),
        ("_toolSummaries", mcp_client),
        ("class McpResourceSummary", mcp_client),
        ("_resourceSummaries", mcp_client),
        ("class McpPromptSummary", mcp_client),
        ("_promptSummaries", mcp_client),
        ("MCP接続を開始", mcp_screen),
        ("対象ServerのCredential metadata一覧を取得", mcp_screen),
        ("確認して実行", mcp_screen),
        ("Tool一覧：", mcp_screen),
        ("入力仕様hash:", mcp_screen),
        ("Resource一覧：", mcp_screen),
        ("Prompt一覧：", mcp_screen),
    ):
        if token not in source:
            不整合.append(f"C9 MCP接続／切断の統治済み経路がない: {token}")
    return 不整合


def A2A接続センターの統治経路と境界を検査する() -> list[str]:
    不整合: list[str] = []
    specification = DOC_SPECS / "a2a-connection-center.md"
    if not specification.exists():
        return ["C16 A2A接続センターの日本語意味正本がない"]
    specification_text = specification.read_text(encoding="utf-8")
    required_tokens = (
        "A2A接続",
        "A2A接続一覧",
        "Agent Card",
        "metadata_only",
        "LIVE_RUNTIME",
        "INTERNAL_STATE",
        "owner control",
        "通常認証済みIPC",
        "HTTPS",
        "Task送信",
        "Credential実値",
        "release_blocker",
    )
    for token in required_tokens:
        if token not in specification_text:
            不整合.append(f"C16 A2A接続センター正本に必須境界がない: {token}")

    try:
        canonical_index = json.loads((ROOT / "規定" / "正本索引.json").read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        不整合.append("C16 A2A接続センター正本索引を読めない")
    else:
        current_sources = canonical_index.get("現行正本", [])
        if not any(item.get("path") == "docs/specs/a2a-connection-center.md" for item in current_sources if isinstance(item, dict)):
            不整合.append("C16 A2A接続センター正本が正本索引へ登録されていない")

    for name in ("a2a_connection", "a2a_connection_receipt", "a2a_connection_list"):
        schema = load_schema(name + ".schema.json")
        valid = load_contract_fixture(name + ".valid.json")
        failures = validate_instance(valid, schema)
        if failures:
            不整合.extend(f"C16 {name} valid fixtureが拒否された: {failure}" for failure in failures)

    invalid_names = (
        "a2a_connection_authority.invalid.json",
        "a2a_connection_receipt_raw_endpoint.invalid.json",
        "a2a_connection_list_wrong_evidence.invalid.json",
    )
    invalid_schema_names = {
        invalid_names[0]: "a2a_connection",
        invalid_names[1]: "a2a_connection_receipt",
        invalid_names[2]: "a2a_connection_list",
    }
    for name in invalid_names:
        try:
            invalid = load_contract_fixture("invalid/" + name)
        except (OSError, json.JSONDecodeError) as exc:
            不整合.append(f"C16 {name}を読めない: {exc}")
            continue
        if not validate_instance(invalid, load_schema(invalid_schema_names[name] + ".schema.json")):
            不整合.append(f"C16 {name}を受理している")

    request = load_contract_fixture("a2a_connection.valid.json")
    if request.get("Transport") != "http" or request.get("Credential ref", {}).get("required") is not False:
        不整合.append("C16接続要求が現行loopback HTTP／credential ref境界ではない")
    receipt = load_contract_fixture("a2a_connection_receipt.valid.json")
    if (
        receipt.get("権限生成") != "なし"
        or receipt.get("authority_strip") is not True
        or receipt.get("公開範囲") != "metadata_only"
        or receipt.get("証拠種別") != "LIVE_RUNTIME"
        or receipt.get("Trust", {}).get("requires_operator_review") is not True
    ):
        不整合.append("C16 receiptのAuthority、公開範囲、証拠種別、Trust境界が不正")
    card = receipt.get("Agent Card", {})
    if "endpoint" in card or "uri" in card or "Agent Card URI" in card:
        不整合.append("C16 receiptがAgent Card endpoint実値を保持している")
    authentication = card.get("authentication", {})
    if authentication.get("secret_value_present") is not False:
        不整合.append("C16 receiptがcredential実値の存在を許可している")

    rust_a2a = (RUST_HELPER / "src" / "a2a.rs").read_text(encoding="utf-8")
    rust_center = (RUST_HELPER / "src" / "broker" / "a2a_center.rs").read_text(encoding="utf-8")
    owner_cli = (RUST_HELPER / "src" / "owner_cli.rs").read_text(encoding="utf-8")
    protocol = (RUST_HELPER / "src" / "broker" / "protocol.rs").read_text(encoding="utf-8")
    store = (RUST_HELPER / "src" / "broker" / "store.rs").read_text(encoding="utf-8")
    ipc_request = (SPECS / "ipc_request.schema.json").read_text(encoding="utf-8")
    ipc_response = (SPECS / "ipc_response.schema.json").read_text(encoding="utf-8")
    for token, source in (
        ("a2a_https_unavailable", rust_a2a),
        ("a2a_http_non_loopback", rust_a2a),
        ("Content-Length", rust_a2a),
        ("secret_value_present", rust_a2a),
        ("supportedInterfaces", rust_a2a),
        ("a2a_owner_required", rust_center),
        ("a2a_normal_channel_required", rust_center),
        ("recover-a2a-connection", rust_center),
        ("metadata_only", rust_center),
        ("Task本文", rust_center),
        ("A2A接続公開投影", owner_cli),
        ("A2A接続設定に禁止fieldがある", owner_cli),
        ("A2A接続", protocol + ipc_request + ipc_response),
        ("A2A接続一覧", protocol + ipc_request + ipc_response),
        ("restored_pending_review", rust_center),
        ("owner_reapproval_required", rust_center),
        ("load_persistent_connections", rust_center + protocol),
        ("a2a_connections.json", store),
        ("MalformedA2aState", store + rust_center),
    ):
        if token not in source:
            不整合.append(f"C16実装に統治境界tokenがない: {token}")
    return 不整合


def 複数Host_registryの統治経路と境界を検査する() -> list[str]:
    不整合: list[str] = []
    specification = DOC_SPECS / "host-registry.md"
    if not specification.exists():
        return ["C17 Host registryの日本語意味正本がない"]
    specification_text = specification.read_text(encoding="utf-8")
    required_tokens = (
        "Host登録",
        "Host一覧",
        "owner control",
        "通常認証済みIPC",
        "metadata-only",
        "pending_review",
        "INTERNAL_STATE",
        "identity",
        "Permission",
        "Approval",
        "Authority",
        "Host切替",
        "release_blocker",
    )
    for token in required_tokens:
        if token not in specification_text:
            不整合.append(f"C17 Host registry正本に必須境界がない: {token}")

    try:
        canonical_index = json.loads((ROOT / "規定" / "正本索引.json").read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        不整合.append("C17 Host registry正本索引を読めない")
    else:
        current_sources = canonical_index.get("現行正本", [])
        if not any(item.get("path") == "docs/specs/host-registry.md" for item in current_sources if isinstance(item, dict)):
            不整合.append("C17 Host registry正本が正本索引へ登録されていない")

    for name in ("host_registration", "host_receipt", "host_list", "host_switch", "host_switch_receipt"):
        schema = load_schema(name + ".schema.json")
        valid = load_contract_fixture(name + ".valid.json")
        failures = validate_instance(valid, schema)
        if failures:
            不整合.extend(f"C17 {name} valid fixtureが拒否された: {failure}" for failure in failures)
    invalid_names = (
        "host_registration_authority.invalid.json",
        "host_receipt_verified.invalid.json",
        "host_list_wrong_evidence.invalid.json",
        "host_switch_authority.invalid.json",
        "host_switch_receipt_authority.invalid.json",
    )
    invalid_schema_names = {
        invalid_names[0]: "host_registration",
        invalid_names[1]: "host_receipt",
        invalid_names[2]: "host_list",
        invalid_names[3]: "host_switch",
        invalid_names[4]: "host_switch_receipt",
    }
    for name in invalid_names:
        try:
            invalid = load_contract_fixture("invalid/" + name)
        except (OSError, json.JSONDecodeError) as exc:
            不整合.append(f"C17 {name}を読めない: {exc}")
            continue
        if not validate_instance(invalid, load_schema(invalid_schema_names[name] + ".schema.json")):
            不整合.append(f"C17 {name}を受理している")

    center = (RUST_HELPER / "src" / "broker" / "host_center.rs").read_text(encoding="utf-8")
    owner_cli = (RUST_HELPER / "src" / "owner_cli.rs").read_text(encoding="utf-8")
    protocol = (RUST_HELPER / "src" / "broker" / "protocol.rs").read_text(encoding="utf-8")
    store = (RUST_HELPER / "src" / "broker" / "store.rs").read_text(encoding="utf-8")
    ipc_request = (SPECS / "ipc_request.schema.json").read_text(encoding="utf-8")
    ipc_response = (SPECS / "ipc_response.schema.json").read_text(encoding="utf-8")
    for token, source in (
        ("host_owner_required", center),
        ("host_normal_channel_required", center),
        ("pending_review", center),
        ("metadata_only", center),
        ("authority_strip", center),
        ("host.registry.register", center),
        ("load_persistent_hosts", center + protocol),
        ("hosts.json", store),
        ("MalformedHostState", store + center),
        ("Host登録", protocol + ipc_request + ipc_response),
        ("Host一覧", protocol + ipc_request + ipc_response),
        ("Host切替", protocol + ipc_request + ipc_response),
        ("not_reused", center),
        ("再利用禁止", center),
        ("Host登録公開投影", owner_cli),
        ("Host登録設定に禁止fieldがある", owner_cli),
    ):
        if token not in source:
            不整合.append(f"C17 Host registry実装に統治境界tokenがない: {token}")
    return 不整合


def Host操作面の観測境界とHost間非混線を検査する() -> list[str]:
    不整合: list[str] = []
    specification = DOC_SPECS / "host-operation-surface.md"
    if not specification.exists():
        return ["C18 Host操作面の日本語意味正本がない"]
    text = specification.read_text(encoding="utf-8")
    for token in (
        "Host一覧",
        "Host切替",
        "Runtime／Agent",
        "未観測",
        "Permission",
        "Approval",
        "Authority",
        "release_blocker",
    ):
        if token not in text:
            不整合.append(f"C18 Host操作面正本に必須境界がない: {token}")

    center = (RUST_HELPER / "src" / "broker" / "host_center.rs").read_text(encoding="utf-8")
    desktop = (DESKTOP_FLUTTER / "lib" / "screens" / "host_operation_center.dart").read_text(encoding="utf-8")
    shell_core = (DESKTOP_FLUTTER / "lib" / "services" / "shell_core_client.dart").read_text(encoding="utf-8")
    for token, source in (
        ("Host切替", center + desktop + shell_core),
        ("not_reused", center + desktop + shell_core),
        ("authority_strip", center + desktop + shell_core),
        ("Runtime一覧", desktop),
        ("Agent一覧", desktop),
        ("未観測", desktop),
        ("Host一覧", shell_core),
    ):
        if token not in source:
            不整合.append(f"C18 Host操作面実装に統治境界tokenがない: {token}")
    return 不整合


def Adapter管理操作の統治境界を検査する() -> list[str]:
    不整合: list[str] = []
    specification = DOC_SPECS / "adapter-management-surface.md"
    if not specification.exists():
        return ["C19 Adapter管理操作の日本語意味正本がない"]
    text = specification.read_text(encoding="utf-8")
    for token in (
        "アダプター導入",
        "アダプター検証",
        "アダプター有効化",
        "アダプター無効化",
        "アダプター隔離",
        "アダプター更新",
        "アダプター削除",
        "owner_reapproval_required",
        "metadata_only",
        "filesystem",
        "process",
        "authority_strip",
        "Ed25519",
        "release_blocker",
    ):
        if token not in text:
            不整合.append(f"C19 Adapter管理操作正本に必須境界がない: {token}")

    center = (RUST_HELPER / "src" / "broker" / "adapter_center.rs").read_text(encoding="utf-8")
    protocol = (RUST_HELPER / "src" / "broker" / "protocol.rs").read_text(encoding="utf-8")
    store = (RUST_HELPER / "src" / "broker" / "store.rs").read_text(encoding="utf-8")
    desktop = (DESKTOP_FLUTTER / "lib" / "screens" / "runtime_center.dart").read_text(encoding="utf-8")
    client = (DESKTOP_FLUTTER / "lib" / "services" / "shell_core_client.dart").read_text(encoding="utf-8")
    launcher = (RUST_HELPER / "src" / "desktop_launcher.rs").read_text(encoding="utf-8")
    broker_client = (DESKTOP_FLUTTER / "lib" / "services" / "broker_client.dart").read_text(encoding="utf-8")
    pipe_controller = (DESKTOP_FLUTTER / "windows" / "runner" / "broker_pipe_controller.cpp").read_text(encoding="utf-8")
    for token, source in (
        ("owner_reapproval_required", center + desktop + client),
        ("runtime_is_quarantined", center + protocol),
        ("adapter_quarantined", protocol),
        ("authority_strip", center + desktop + client),
        ("metadata_only", center + desktop + client),
        ("adapters.json", store),
        ("MalformedAdapterState", store + center),
        ("アダプター一覧", center + protocol + desktop + client),
        ("アダプター隔離", center + protocol + desktop + client),
        ("アダプター削除", center + protocol + desktop + client),
        ("owner_manifest_confirmation_summary", center + launcher),
        ("AdapterManifestManagement", launcher),
        ("currentAdapterHash", client),
        ("アダプター導入", protocol + launcher + desktop + client + broker_client),
        ("アダプター更新", protocol + launcher + desktop + client + broker_client),
        ("アダプター一覧", client),
        (r"\xE3\x82\xA2\xE3\x83\x80\xE3\x83\x97\xE3\x82\xBF\xE3\x83\xBC\xE5\xB0\x8E\xE5\x85\xA5", pipe_controller),
    ):
        if token not in source:
            不整合.append(f"C19 Adapter管理操作実装に統治境界tokenがない: {token}")

    for name in (
        "adapter_management_request",
        "adapter_management_receipt",
    ):
        schema = load_schema(name + ".schema.json")
        valid = load_contract_fixture(name + ".valid.json")
        failures = validate_instance(valid, schema)
        if failures:
            不整合.extend(f"C19 {name} valid fixtureが拒否された: {failure}" for failure in failures)
    update_valid = load_contract_fixture("adapter_management_update.valid.json")
    failures = validate_instance(update_valid, load_schema("adapter_management_request.schema.json"))
    if failures:
        不整合.extend(f"C19 adapter_management_update valid fixtureが拒否された: {failure}" for failure in failures)
    list_schema = load_schema("adapter_management_list.schema.json")
    receipt = load_contract_fixture("adapter_management_receipt.valid.json")
    list_fixture = {
        "版": 1,
        "Adapter一覧": [receipt],
        "件数": 1,
        "公開範囲": "metadata_only",
        "証拠種別": "INTERNAL_STATE",
        "権限生成": "なし",
        "authority_strip": True,
    }
    failures = validate_instance(list_fixture, list_schema)
    if failures:
        不整合.extend(f"C19 adapter_management_list fixtureが拒否された: {failure}" for failure in failures)
    for file_name, schema_name in (
        ("adapter_management_authority.invalid.json", "adapter_management_request"),
        ("adapter_management_unknown_field.invalid.json", "adapter_management_request"),
        ("adapter_management_update_missing_hash.invalid.json", "adapter_management_request"),
        ("adapter_management_install_with_hash.invalid.json", "adapter_management_request"),
    ):
        invalid = load_contract_fixture("invalid/" + file_name)
        if not validate_instance(invalid, load_schema(schema_name + ".schema.json")):
            不整合.append(f"C19 {file_name}を受理している")
    return 不整合


def Windows常駐トレイ操作面の統治境界を検査する() -> list[str]:
    不整合: list[str] = []
    specification = DOC_SPECS / "windows-tray-surface.md"
    if not specification.exists():
        return ["C20 Windows常駐トレイの日本語意味正本がない"]
    text = specification.read_text(encoding="utf-8")
    for token in (
        "全Runtime停止要求",
        "owner_reapproval_required",
        "停止実行済み=false",
        "権限生成=なし",
        "Broker IPC",
        "直接kill",
        "release_blocker",
    ):
        if token not in text:
            不整合.append(f"C20 Windows常駐トレイ正本に必須境界がない: {token}")

    protocol = (RUST_HELPER / "src" / "broker" / "protocol.rs").read_text(encoding="utf-8")
    lifecycle = (RUST_HELPER / "src" / "broker" / "runtime_lifecycle.rs").read_text(encoding="utf-8")
    tray_client = (DESKTOP_FLUTTER / "lib" / "services" / "windows_tray_client.dart").read_text(encoding="utf-8")
    shell_client = (DESKTOP_FLUTTER / "lib" / "services" / "shell_core_client.dart").read_text(encoding="utf-8")
    main = (DESKTOP_FLUTTER / "lib" / "main.dart").read_text(encoding="utf-8")
    native = (DESKTOP_FLUTTER / "windows" / "runner" / "tray_controller.cpp").read_text(encoding="utf-8")
    for token, source in (
        ("全Runtime停止要求", protocol + lifecycle + shell_client),
        ("all_stop_request_body", lifecycle + protocol),
        ("owner_reapproval_required", lifecycle + shell_client + tray_client),
        ("停止実行済み", lifecycle + shell_client),
        ("gui_shell/tray", tray_client + native),
        ("stop_request", main + tray_client + native),
        ("Shell_NotifyIconW", native),
        ("NIM_ADD", native),
        ("NIM_DELETE", native),
        ("PostMessageW", native),
    ):
        if token not in source:
            不整合.append(f"C20 Windows常駐トレイ実装に統治境界tokenがない: {token}")

    for name in ("tray_stop_request", "tray_stop_response", "windows_tray_projection"):
        schema = load_schema(name + ".schema.json")
        valid = load_contract_fixture(name + ".valid.json")
        failures = validate_instance(valid, schema)
        if failures:
            不整合.extend(f"C20 {name} valid fixtureが拒否された: {failure}" for failure in failures)
    for file_name, schema_name in (
        ("tray_stop_request_unknown_field.invalid.json", "tray_stop_request"),
        ("tray_stop_response_authority.invalid.json", "tray_stop_response"),
        ("windows_tray_projection_zero_unknown.invalid.json", "windows_tray_projection"),
    ):
        invalid = load_contract_fixture("invalid/" + file_name)
        if not validate_instance(invalid, load_schema(schema_name + ".schema.json")):
            不整合.append(f"C20 {file_name}を受理している")
    return 不整合


def コマンドパレット拡張の統治境界を検査する() -> list[str]:
    不整合: list[str] = []
    specification = DOC_SPECS / "command-palette-surface.md"
    if not specification.exists():
        return ["C21コマンドパレットの日本語意味正本がない"]
    specification_text = specification.read_text(encoding="utf-8")
    main = (DESKTOP_FLUTTER / "lib" / "main.dart").read_text(encoding="utf-8")
    start = main.find("List<_CommandEntry> _featureCommandEntries()")
    end = main.find("\n  }\n}", start)
    if start == -1 or end == -1:
        return ["C21コマンドパレットの新機能登録実装がない"]
    feature_commands = main[start:end]
    for token in (
        "Runtimeを開く",
        "Agentを開く",
        "履歴検索",
        "評価実行",
        "MCP接続",
        "通知表示",
        "資源監視",
        "資格情報",
        "更新確認",
        "Host切替",
        "全Runtime停止要求を確認",
    ):
        if token not in feature_commands or token not in specification_text:
            不整合.append(f"C21コマンドパレットに必須コマンドがない: {token}")
    for forbidden in ("brokerTransport", "transport.request", "BrokerClient", "requestAllRuntimeStop"):
        if forbidden in feature_commands:
            不整合.append(f"C21コマンドパレットが直接権限経路へ到達している: {forbidden}")
    for token in ("Ctrl+K", "Ctrl+P", "最大30件", "画面遷移だけ"):
        if token not in specification_text:
            不整合.append(f"C21コマンドパレット正本にbounded／keyboard境界がない: {token}")
    return 不整合


def グローバル検索の統治境界を検査する() -> list[str]:
    不整合: list[str] = []
    specification = DOC_SPECS / "global-search-surface.md"
    index = DESKTOP_FLUTTER / "lib" / "services" / "global_search_index.dart"
    main = (DESKTOP_FLUTTER / "lib" / "main.dart").read_text(encoding="utf-8")
    if not specification.exists() or not index.exists():
        return ["C22グローバル検索の正本または実装がない"]
    specification_text = specification.read_text(encoding="utf-8")
    index_text = index.read_text(encoding="utf-8")
    required = (
        ("Runtime", "実行系"),
        ("Agent", "エージェント"),
        ("Session", "対話セッション"),
        ("Permission", "権限"),
        ("Approval", "承認"),
        ("Audit", "監査"),
        ("Recovery", "復旧"),
        ("Problem", "問題"),
        ("Evidence", "証拠"),
        ("MCP", "MCP"),
        ("A2A", "A2A"),
        ("Host", "接続先"),
        ("Adapter", "アダプター"),
        ("Profile", "プロファイル"),
        ("Evaluation", "評価"),
        ("Notification", "通知"),
    )
    for token, japanese in required:
        if (
            token not in specification_text
            and japanese not in specification_text
        ) or (token not in index_text and japanese not in index_text):
            不整合.append(f"C22グローバル検索に検索対象がない: {token}")
    for token in (
        "GlobalSearchIndex.maxQueryLength",
        "GlobalSearchIndex.maxResults",
        "Ctrl+Shift+F",
        "検索結果は表示専用",
    ):
        if (
            token not in specification_text
            and token not in main
            and token not in index_text
        ):
            不整合.append(f"C22グローバル検索のbounded／keyboard境界がない: {token}")
    for forbidden in (
        "BrokerClient",
        "transport.request",
        "ShellCoreClient.product",
        "Clipboard.setData",
        "Process.run",
        "Process.start",
    ):
        if forbidden in index_text:
            不整合.append(f"C22グローバル検索が権限・外部作用へ到達している: {forbidden}")
    start = main.find("Future<void> _openGlobalSearch")
    end = main.find("\n  List<_CommandEntry> _commandEntries", start)
    if start == -1 or end == -1:
        不整合.append("C22グローバル検索の画面遷移経路がない")
    else:
        navigation = main[start:end]
        for forbidden in ("BrokerClient", "transport.request", "Clipboard.setData", "Process."):
            if forbidden in navigation:
                不整合.append(f"C22グローバル検索の選択操作が直接作用を持つ: {forbidden}")
    return 不整合


def Desktop_UX統合の表示境界を検査する() -> list[str]:
    不整合: list[str] = []
    specification = DOC_SPECS / "desktop-ux-integration.md"
    main_path = DESKTOP_FLUTTER / "lib" / "main.dart"
    if not specification.exists() or not main_path.exists():
        return ["C23 Desktop UX統合の正本または実装がない"]
    specification_text = specification.read_text(encoding="utf-8")
    main = main_path.read_text(encoding="utf-8")
    for token in (
        "運用",
        "安全",
        "開発",
        "設定",
        "すべて",
        "NavigationRail",
    ):
        if token not in specification_text or token not in main:
            不整合.append(f"C23 Desktop UX統合の境界tokenがない: {token}")
    for token in (
        "_ShellNavigationGroup",
        "navigationGroup",
        "操作グループ選択",
    ):
        if token not in main:
            不整合.append(f"C23 Desktop UX統合の実装tokenがない: {token}")
    page_indices = {
        int(index)
        for index in re.findall(r"_ShellPageEntry\(\s*(\d+)\s*,", main)
    }
    if page_indices != set(range(21)):
        不整合.append(
            "C23 Desktop UX統合が現行21画面のindexを保持していない: "
            f"{sorted(page_indices)}"
        )
    for forbidden in (
        "BrokerClient",
        "transport.request",
        "Process.run",
        "Process.start",
        "Clipboard.setData",
    ):
        start = main.find("class _ShellNavigationGroup")
        end = main.find("class _ShellViewMode", start)
        navigation_group_section = main[start:end]
        if forbidden in navigation_group_section:
            不整合.append(f"C23 Desktop UX統合が直接作用へ到達している: {forbidden}")
    return 不整合


def Mobile投影の統治境界を検査する() -> list[str]:
    不整合: list[str] = []
    specification = DOC_SPECS / "mobile-surface.md"
    main = MOBILE_FLUTTER / "lib" / "main.dart"
    projection = MOBILE_FLUTTER / "lib" / "services" / "mobile_projection_client.dart"
    link = MOBILE_FLUTTER / "lib" / "services" / "device_link_client.dart"
    device_link = RUST_HELPER / "src" / "broker" / "device_link.rs"
    protocol = RUST_HELPER / "src" / "broker" / "protocol.rs"
    required_files = (
        specification,
        main,
        projection,
        MOBILE_FLUTTER / "lib" / "screens" / "agent_status.dart",
        MOBILE_FLUTTER / "lib" / "screens" / "resource_overview.dart",
        MOBILE_FLUTTER / "lib" / "screens" / "history.dart",
        MOBILE_FLUTTER / "lib" / "screens" / "mcp_status.dart",
    )
    if any(not path.exists() for path in required_files):
        return ["C24 Mobile投影の正本または実装がない"]
    specification_text = specification.read_text(encoding="utf-8")
    main_text = main.read_text(encoding="utf-8")
    projection_text = projection.read_text(encoding="utf-8")
    link_text = link.read_text(encoding="utf-8")
    device_link_text = device_link.read_text(encoding="utf-8")
    protocol_text = protocol.read_text(encoding="utf-8")
    for token in (
        "通知",
        "Agent状態",
        "Approval",
        "Runtime状態",
        "資源概要",
        "履歴",
        "Host",
        "MCP",
        "緊急停止要求",
        "Recovery",
        "未観測",
        "unknown",
    ):
        if token not in specification_text:
            不整合.append(f"C24正本にMobile対象境界がない: {token}")
    for token in ("Agent", "資源", "履歴", "MCP", "MobileAgentStatus", "ResourceOverview", "MobileHistory", "MobileMcpStatus"):
        if token not in main_text:
            不整合.append(f"C24 Mobile実装に投影surfaceがない: {token}")
    for token in ("Agent一覧", "secret_value_present", "通知一覧", "全Runtime停止要求", "INTERNAL_STATE", "権限生成", "owner_reapproval_required"):
        if token not in projection_text:
            不整合.append(f"C24 Mobile投影clientの境界がない: {token}")
    for token in (
        "Agent一覧",
        "実行系ライフサイクル状態",
        "実行系資源観測",
        "通知一覧",
        "全Runtime停止要求",
        "対話履歴閲覧状態",
        "対話履歴閲覧",
    ):
        if token not in link_text or token not in device_link_text or token not in protocol_text:
            不整合.append(f"C24 Device Linkの読み取り操作が接続されていない: {token}")
    agent_screen = (MOBILE_FLUTTER / "lib" / "screens" / "agent_status.dart").read_text(encoding="utf-8")
    for token in ("実taskの実行可否", "Permission", "Approval", "Trust", "未観測"):
        if token not in agent_screen:
            不整合.append(f"D4 Mobile Agent statusがmetadata境界を示さない: {token}")
    for forbidden in (
        "対話履歴承認",
        "対話内容閲覧",
        "MCP接続",
        "資格情報一覧",
        "Process.start",
        "Process.run",
    ):
        if forbidden in projection_text:
            不整合.append(f"C24 Mobile投影clientが禁止操作へ到達している: {forbidden}")
    for forbidden in ("端末秘密", "招待秘密", "Credential実値", "payload_hash"):
        if forbidden in projection_text:
            不整合.append(f"C24 Mobile投影clientが秘密またはauthority値を扱っている: {forbidden}")
    return 不整合


def 書庫展開で日本語名と内容を保持する() -> list[str]:
    import hashlib
    import os
    import zipfile
    from unittest.mock import patch
    from tooling.packaging_portability_check import 展開命令

    不整合 = []
    with tempfile.TemporaryDirectory() as 場所:
        ルート = Path(場所)
        書庫 = ルート / "source.zip"
        展開先 = ルート / "展開 先"
        展開先.mkdir()
        名前 = "規定/日本語 名前.txt"
        内容 = "本文と改行\n".encode("utf-8")
        with zipfile.ZipFile(書庫, "w", zipfile.ZIP_DEFLATED) as 保存:
            保存.writestr(名前, 内容)
            保存.writestr("ascii.txt", b"ascii")
        環境 = os.environ.copy()
        環境.update(LC_ALL="C", LANG="C")
        結果 = subprocess.run(展開命令(書庫, 展開先), env=環境, capture_output=True, timeout=30)
        if 結果.returncode != 0 or not (展開先 / 名前).is_file():
            不整合.append("標準展開器が日本語名を保持しなかった")
        elif hashlib.sha256((展開先 / 名前).read_bytes()).digest() != hashlib.sha256(内容).digest():
            不整合.append("標準展開器が内容を変更した")
        書庫.write_bytes(b"broken archive")
        結果 = subprocess.run(展開命令(書庫, 展開先), env=環境, capture_output=True, timeout=30)
        if 結果.returncode == 0:
            不整合.append("破損した書庫が成功と判定された")
        with patch("tooling.packaging_portability_check.sys.platform", "win32"), patch.dict(os.environ, {"SystemRoot": str(ルート / "不在")}):
            try:
                展開命令(書庫, 展開先)
                不整合.append("不在の Windows 展開器が許可された")
            except FileNotFoundError:
                pass
        with patch("tooling.packaging_portability_check.sys.platform", "linux"), patch("tooling.packaging_portability_check.shutil.which", return_value=None):
            try:
                展開命令(書庫, 展開先)
                不整合.append("不在の POSIX 展開器が許可された")
            except FileNotFoundError:
                pass
    return 不整合


def test_packaging_portability_utf8_governance_allowlist_is_exact() -> list[str]:
    errors = []
    allowlisted_paths = [
        ROOT / "規定" / "00_日本語基底規定.md",
        ROOT / "規定" / "正本索引.json",
        ROOT / "規定" / "日本語基底例外.json",
        ROOT / "tooling" / "日本語基底監査.py",
    ]
    allowlisted_errors = portable_path_errors(allowlisted_paths)
    if allowlisted_errors:
        errors.append(
            "exact UTF-8 governance allowlistのpathが拒否された: "
            + "; ".join(allowlisted_errors)
        )

    rev1_document_errors = portable_path_errors(
        [ROOT / "docs" / "総合機能拡張_rev1" / "実装仕様書.md"]
    )
    if rev1_document_errors:
        errors.append(
            "rev1日本語文書pathが正本資料のallowlistから外れている: "
            + "; ".join(rev1_document_errors)
        )

    unregistered_errors = portable_path_errors([ROOT / "規定" / "未登録規定.md"])
    if not unregistered_errors:
        errors.append("未登録の非ASCII pathが許可された")

    control_errors = portable_path_errors([ROOT / "docs" / "control\npath.md"])
    if not control_errors:
        errors.append("control characterを含むpathが許可された")
    return errors


def test_invariant_evaluator_detects_intentional_import_violation() -> list[str]:
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        shell_core = root / "packages" / "shell_core"
        shell_core.mkdir(parents=True)
        (shell_core / "bad.py").write_text("import flutter\n", encoding="utf-8")
        if not InvariantEvaluator(root).shell_core_imports_forbidden("flutter"):
            return ["InvariantEvaluatorが禁止されたFlutter importを検出しなかった"]
    return []


def test_invariant_evaluator_detects_live_authority_invariants() -> list[str]:
    flags = InvariantEvaluator().evaluate()
    errors = []
    if flags["adapter_metadata_can_escalate_authority"]:
        errors.append("InvariantEvaluatorがadapter metadataのauthority escalationを計測した")
    if flags["memory_cache_previous_state_can_grant_authority"]:
        errors.append("InvariantEvaluatorが非authority sourceからのauthority付与を計測した")
    if flags["full_payload_projected_without_full_visibility"]:
        errors.append("InvariantEvaluatorがfull visibilityなしのfull payload projectionを計測した")
    if flags["installer_setup_state_can_grant_authority"]:
        errors.append("InvariantEvaluatorがinstaller/setupのauthority付与を計測した")
    if flags["mobile_device_state_can_grant_authority"]:
        errors.append("InvariantEvaluatorがmobile/deviceのauthority付与を計測した")
    return errors


def test_rust_helper_required_sources_exist() -> list[str]:
    existing = {path.name for path in (RUST_HELPER / "src").glob("*.rs")}
    errors = []
    for missing in sorted(RUST_HELPER_REQUIRED_SOURCES - existing):
        errors.append(f"native/rust_helper/src/{missing} が存在しない")
    return errors


def test_rust_helper_contract_shape_exists() -> list[str]:
    lib_rs = (RUST_HELPER / "src" / "lib.rs").read_text(encoding="utf-8")
    errors = []
    for token in ["HelperResponse", "HelperError", "ok", "operation", "result", "diagnostics", "error"]:
        if token not in lib_rs:
            errors.append(f"Rust helper contractにtokenがない: {token}")
    return errors


def test_rust_helper_does_not_expose_hidden_authority_paths() -> list[str]:
    forbidden = [
        "std::process::Command",
        "std::fs::read_to_string",
        "std::fs::read(",
        "std::fs::write",
        "reqwest::",
        "ureq::",
    ]
    errors = []
    for path in sorted((RUST_HELPER / "src").rglob("*.rs")):
        text = path.read_text(encoding="utf-8").split("#[cfg(test)]", 1)[0]
        for pattern in forbidden:
            if pattern == "reqwest::" and path.as_posix().endswith(
                (
                    "native/rust_helper/src/broker/update_download.rs",
                    "native/rust_helper/src/broker/update_center.rs",
                    "native/rust_helper/src/desktop_launcher.rs",
                )
            ):
                if path.name != "update_download.rs" and "reqwest::" in re.sub(
                    r"reqwest::Url::parse", "", text
                ):
                    errors.append(f"{path} はURL解析以外のreqwest経路を追加した")
                continue
            if pattern in text:
                errors.append(f"{path} が禁止されたhelper authority patternを使う: {pattern}")
    return errors


def test_update_download_transport_is_broker_owned_and_bounded() -> list[str]:
    download = (RUST_HELPER / "src" / "broker" / "update_download.rs").read_text(
        encoding="utf-8"
    ).split("#[cfg(test)]", 1)[0]
    center = (RUST_HELPER / "src" / "broker" / "update_center.rs").read_text(
        encoding="utf-8"
    ).split("#[cfg(test)]\nmod tests", 1)[0]
    required_download = (
        "parse_source_url(url_text)?",
        "resolve_public_addresses(host,443,cancel,deadline)?",
        "const DNS_RESOLUTION_TIMEOUT: Duration = Duration::from_secs(15);",
        "#[cfg(windows)]",
        ".https_only(true)",
        ".no_proxy()",
        ".redirect(redirect::Policy::none())",
        ".retry(retry::never())",
        ".timeout(timeout)",
        ".read_timeout(READ_TIMEOUT)",
        ".resolve_to_addrs(host, addresses)",
        "validate_response_headers(response.headers(), expected_bytes)?",
        "chunk.chunks(COPY_BUFFER_BYTES)",
        "consume_chunk(",
        ".hard_link(temporary_name, directory, final_name)",
        "ExistingPackageState::Corrupt",
        "PackageDisposition::RepairedCorrupt",
        ".rename(temporary_name, directory, final_name)",
        "fn package_entry_is_replaceable(",
        "build_download_client(host, &addresses, &[], MAX_DOWNLOAD_TIME)?",
        "let remaining = deadline.saturating_duration_since(std::time::Instant::now());",
        "if remaining.is_zero()",
        "build_download_client(host, &addresses, &[], remaining)?",
    )
    errors = [
        f"Broker download transportの必須境界がない: {token}"
        for token in required_download
        if re.sub(r"\s+", "", token) not in re.sub(r"\s+", "", download)
    ]
    resolver_match = re.search(
        r"(?ms)^fn resolve_public_addresses\(.*?^\}\s*$",
        (RUST_HELPER / "src" / "broker" / "update_download.rs").read_text(
            encoding="utf-8"
        ),
    )
    if resolver_match is None:
        errors.append("Broker DNS resolver関数がない")
    else:
        resolver = re.sub(r"\s+", "", resolver_match.group(0))
        required_resolver = (
            "deadline.min(std::time::Instant::now()+DNS_RESOLUTION_TIMEOUT)",
            "windows_dns::resolve(",
            "to_socket_addrs()",
        )
        errors.extend(
            f"Broker DNS resolverの期限・platform境界がない: {token}"
            for token in required_resolver
            if token not in resolver
        )
    dns_helper = (ROOT / "native" / "windows_dns" / "src" / "lib.rs").read_text(
        encoding="utf-8"
    ).split("#[cfg(test)]", 1)[0]
    required_dns = (
        "DnsQueryEx(",
        "DnsCancelQuery(",
        "recv_timeout(CANCEL_CALLBACK_TIMEOUT)",
        "PENDING_QUERY.compare_exchange(",
        "Arc::into_raw(Arc::clone(&context))",
    )
    errors.extend(
        f"Windows DNS resolverの期限・取消境界がない: {token}"
        for token in required_dns
        if re.sub(r"\s+", "", token) not in re.sub(r"\s+", "", dns_helper)
    )
    if "OwnerConfirmationSource::DesktopNativeConfirmation" not in center:
        errors.append("更新downloadがDesktop native Owner確認へ結合されない")
    if "confirmation.payload_hash != hash" not in center:
        errors.append("更新downloadが確認済み要求hashを実行直前に再照合しない")
    if '"recovered"' not in center or "検証済みpackageへの置換" not in center:
        errors.append("破損package修復のAudit結果が回復として記録されない")
    if "start(directory,&request,&current_record,source_url)" not in re.sub(
        r"\s+", "", center
    ):
        errors.append("package修復の実行経路がnative Owner確認付きdownload jobから外れている")
    return errors


def test_codex_cli_adapter_is_broker_governed_and_bounded() -> list[str]:
    adapter_path = RUST_HELPER / "src" / "adapters" / "codex_cli.rs"
    cargo_manifest_path = RUST_HELPER / "Cargo.toml"
    broker_path = RUST_HELPER / "src" / "broker" / "ipc_server.rs"
    workspace_path = RUST_HELPER / "src" / "broker" / "workspace.rs"
    dialogue_path = RUST_HELPER / "src" / "broker" / "dialogue.rs"
    scratch_path = RUST_HELPER / "src" / "broker" / "agent_task_scratch.rs"
    reader_path = RUST_HELPER / "src" / "workspace_reader.rs"
    if not adapter_path.is_file():
        return ["Codex CLI Adapter sourceが存在しない"]
    adapter = adapter_path.read_text(encoding="utf-8")
    cargo_manifest = cargo_manifest_path.read_text(encoding="utf-8")
    broker = broker_path.read_text(encoding="utf-8")
    workspace = workspace_path.read_text(encoding="utf-8")
    dialogue = dialogue_path.read_text(encoding="utf-8")
    scratch = scratch_path.read_text(encoding="utf-8")
    reader = reader_path.read_text(encoding="utf-8")
    required = [
        "--json",
        "--ephemeral",
        "--sandbox",
        "--skip-git-repo-check",
        "read-only",
        "env_clear",
        "SAFE_ENVIRONMENT",
        "MAX_OUTPUT_BYTES",
        "turn.completed",
        "turn.failed",
        "AtomicBool",
        "kill",
        "secret_component",
    ]
    errors = [
        f"Codex Adapterに安全境界tokenがない: {token}"
        for token in required
        if token not in adapter
    ]
    task_required = (
        "workspace-write",
        "d4p-agent-task",
        r'windows.sandbox=\"mxc\"',
        "**/*.env",
        "**/.env.*",
        "**/.ssh/**",
        "**/secrets/**",
        r'\":minimal\"=\"read\"',
        r'\":root\"=\"deny\"',
        r'\"glob_scan_max_depth\"=32',
        "network.enabled=false",
        "TASK_BASE_DENY_GLOBS",
        "task_filesystem_override(secret_paths)?",
        "MAX_TASK_FILESYSTEM_OVERRIDE_BYTES",
        "MAX_REGISTERED_SECRET_PATHS",
        "escape_codex_glob_literal",
        'format!("{literal}/**")',
        "WorkspaceTaskScratch",
        "exec_interface_present",
        "workspace_write_interface_present",
        "remove_open_dir_all",
    )
    for token in task_required:
        if token not in adapter:
            errors.append(f"Codex Agent Task境界または未検証gateがない: {token}")
    explicit_unsupported_gate = 'task_execution", "support": {"status": "unsupported"' in adapter
    isolated_e2e_unsupported_gate = (
        '[features]\ndefault = []\n# Local-only integrated R2 test build. Never use for product distribution.\nr2-e2e = []'
        in cargo_manifest
        and '#[cfg(not(all(feature = "r2-e2e", not(test))))]' in adapter
        and 'let task_execution_capability = json!({' in adapter
        and '"status": "unsupported"' in adapter
    )
    if not explicit_unsupported_gate and not isolated_e2e_unsupported_gate:
        errors.append("Codex Agent Taskの通常Release unsupported gateが既定のまま保たれない")
    if adapter.count("task_filesystem_override(secret_paths)?") != 1:
        errors.append("Codex Taskが登録secretから単一のfilesystem overrideを生成しない")
    if "permissions.d4p-agent-task.filesystem.glob_scan_max_depth=" in adapter:
        errors.append("後続filesystem table overrideで走査深度を上書きするCodex設定を受理した")
    propagation = (
        "registered_secret_paths",
        "secret_paths: entry.reader.registered_secret_paths().to_vec()",
        "pub(crate) fn secret_paths(&self) -> &[String]",
    )
    for token in propagation:
        if token not in workspace:
            errors.append(f"Workspace登録除外pathのbinding接続がない: {token}")
    if "secret_paths: binding.secret_paths().to_vec()" not in dialogue:
        errors.append("Task contextへ現在のWorkspace登録除外pathを渡さない")
    if "pub(crate) secret_paths: Vec<String>" not in scratch or "secret_path_count" not in scratch:
        errors.append("Task contextがsecret pathをredacted in-memory fieldとして保持しない")
    if "validate_registered_secret_path" not in reader:
        errors.append("Task sandboxへ渡すWorkspace相対pathの再検証がない")
    forbidden = [
        "--dangerously-bypass-approvals-and-sandbox",
        "--worktree",
        "--add-dir",
        "command_envelope",
    ]
    for token in forbidden:
        if token in adapter:
            errors.append(f"Codex Adapterに禁止された実行境界tokenがある: {token}")
    for required in (
        '.env("OPENAI_API_KEY", credential)',
        '"shell_environment_policy.ignore_default_excludes=false"',
        '"shell_environment_policy.exclude=[\\"OPENAI_API_KEY\\",\\"CODEX_API_KEY\\",\\"CODEX_ACCESS_TOKEN\\"]"',
        '"CODEX_WINDOWS_SANDBOX_PACKAGE_FAMILY"',
        '"local_credential_reference"',
    ):
        if required not in adapter:
            errors.append(f"Codex Provider credentialの短命bindingまたは子process除外がない: {required}")
    if "codex_runtimes" not in broker or "CodexCliAdapter" not in broker:
        errors.append("Codex AdapterがBrokerの明示登録経路へ接続されていない")
    return errors


def test_broker_ipc_contract_schemas_exist() -> list[str]:
    existing = {path.name for path in SPECS.glob("*.schema.json")}
    errors = []
    for missing in sorted(BROKER_REQUIRED_SCHEMAS - existing):
        errors.append(f"broker IPC schemaがない: {missing}")
    for name in sorted(BROKER_REQUIRED_SCHEMAS & existing):
        schema = load_schema(name)
        if schema.get("type") != "object":
            errors.append(f"{name} はobject contractを定義しなければならない")
        if "additionalProperties" not in schema:
            errors.append(f"{name} はadditionalProperties policyを定義しなければならない")
    return errors


def test_broker_boundary_docs_exist() -> list[str]:
    required = {
        "docs/security/IPC_THREAT_MODEL.md",
        "docs/architecture/RUST_BROKER_IPC_PROTOCOL.md",
        "docs/implementation/RUST_SECURITY_BROKER_MIGRATION_PLAN.md",
        "docs/specs/desktop-broker-channel.md",
    }
    errors = []
    for relative in sorted(required):
        path = ROOT / relative
        if not path.exists():
            errors.append(f"{relative} が存在しない")
            continue
        text = path.read_text(encoding="utf-8")
        for token in ["Rust Security Broker", "release_blocker", "FFI"]:
            if token not in text:
                errors.append(f"{relative} にbroker governance tokenがない: {token}")
    return errors


def test_desktop_broker_channel_contract() -> list[str]:
    schema = load_schema("desktop_broker_channel_request.schema.json")
    sample = load_contract_fixture("desktop_broker_channel_request.valid.json")
    errors = [
        f"Desktop Broker channel正常fixtureが不正: {failure}"
        for failure in validate_instance(sample, schema)
    ]
    for field, value in {
        "session_id": "broker-session-forged",
        "session_secret": "0" * 64,
        "credential_role": "owner",
        "owner": True,
        "authority": "owner",
        "endpoint": {"host": "127.0.0.1", "port": 1},
    }.items():
        if not validate_instance({**sample, field: value}, schema):
            errors.append(f"Desktop Broker channelが資格・権限fieldを受理した: {field}")

    # metadataは信頼しない説明dataであり、Schema層で権限を解釈しない。
    metadata_attempt = {
        **sample,
        "metadata": {
            "role": "owner",
            "permission": "all",
            "authority_context": {"trusted": True},
        },
    }
    if validate_instance(metadata_attempt, schema):
        errors.append("Desktop Broker channelが不透明metadataを許可しなかった")

    spec = (ROOT / "docs" / "specs" / "desktop-broker-channel.md").read_text(
        encoding="utf-8"
    )
    for token in (
        "既存authenticated loopback TCP Broker経路",
        "同じ起動器が起動したFlutter child processのPID",
        "Owner資格",
        "fallbackしない",
        "親processの残りの環境変数を継承しない",
    ):
        if token not in spec:
            errors.append(f"Desktop Broker channel契約に境界の説明がない: {token}")

    launcher_source = (
        RUST_HELPER / "src" / "desktop_launcher.rs"
    ).read_text(encoding="utf-8")
    environment_allowlist = launcher_source.split(
        "const FRONTEND_ENVIRONMENT_ALLOWLIST:", 1
    )[1].split("];", 1)[0]
    actual_allowlist = set(re.findall(r'"([A-Z0-9_]+)"', environment_allowlist))
    expected_allowlist = {
        "APPDATA",
        "LOCALAPPDATA",
        "PROGRAMDATA",
        "SYSTEMDRIVE",
        "SYSTEMROOT",
        "TEMP",
        "TMP",
        "USERPROFILE",
        "WINDIR",
    }
    if actual_allowlist != expected_allowlist:
        errors.append(
            "Flutter childのOS environment allowlistが正本と一致しない: "
            f"missing={sorted(expected_allowlist - actual_allowlist)}, "
            f"extra={sorted(actual_allowlist - expected_allowlist)}"
        )
    frontend_launch = launcher_source.split("fn launch_frontend(", 1)[1].split(
        "pub fn run()", 1
    )[0]
    frontend_environment = launcher_source.split(
        "fn configure_frontend_environment(", 1
    )[1].split("\n}", 1)[0]
    for token in (
        "std::env::vars_os()",
        "configure_frontend_environment(",
        "OsStr::new(&broker.channel_pipe_name)",
    ):
        if token not in frontend_launch:
            errors.append(f"Desktop起動器のFlutter起動経路に環境境界がない: {token}")
    for token in (
        ".env_clear()",
        ".envs(filtered_frontend_environment(inherited))",
        '.env("GUI_SHELL_BROKER_CHANNEL_PIPE", channel_pipe_name)',
    ):
        if token not in frontend_environment:
            errors.append(f"Desktop起動器がFlutter環境を限定していない: {token}")
    for token in (
        '"OPENAI_API_KEY"',
        '"CODEX_HOME"',
        '"GUI_SHELL_BROKER_RUNTIME_DIR"',
        '"GUI_SHELL_BROKER_ENDPOINT_JSON"',
        '"GUI_SHELL_BROKER_SESSION_JSON"',
        '"PATH"',
    ):
        if token not in launcher_source:
            errors.append(f"Flutter子process環境分離のnegative試験にcaseがない: {token}")

    launcher_spec = (ROOT / "docs" / "specs" / "windows-desktop-launcher.md").read_text(
        encoding="utf-8"
    )
    operation_surfaces = (ROOT / "docs" / "GUI_OPERATION_SURFACES.md").read_text(
        encoding="utf-8"
    )
    roadmap = (ROOT / "ROADMAP.md").read_text(encoding="utf-8")
    for document, stale_claim in (
        (launcher_spec, "Flutterは既存`BrokerClient`の認証付き`127.0.0.1`接続を使う。"),
        (launcher_spec, "Flutterへ渡す環境変数はendpoint file pathとBroker runtime directoryに限る。"),
        (operation_surfaces, "現行Dart BrokerClientのfile／session secret／loopback socket責務は未解消。"),
        (roadmap, "現行Flutter BrokerClientのDart内file／session-secret／loopback socket処理もD4責務境界とのGap"),
    ):
        if stale_claim in document:
            errors.append(f"Desktop Broker channel現行実装と文書記述が不一致: {stale_claim}")
    for token in (
        "gui_shell/broker",
        "PID照合",
        "GUI_SHELL_BROKER_CHANNEL_PIPE",
        "親processの残りの環境変数を継承しない",
    ):
        if token not in launcher_spec:
            errors.append(f"Windows Desktop起動器正本に現行境界がない: {token}")
    return errors


def test_rust_broker_skeleton_exists() -> list[str]:
    broker_src = RUST_HELPER / "src" / "broker"
    existing = {path.name for path in broker_src.glob("*.rs")}
    errors = []
    for missing in sorted(BROKER_REQUIRED_SOURCES - existing):
        errors.append(f"native/rust_helper/src/broker/{missing} が存在しない")
    main_rs = RUST_HELPER / "src" / "main.rs"
    lib_rs = RUST_HELPER / "src" / "lib.rs"
    for path in [main_rs, lib_rs]:
        text = path.read_text(encoding="utf-8")
        if "#![forbid(unsafe_code)]" not in text:
            errors.append(f"{path.relative_to(ROOT)} はunsafe codeを禁止しなければならない")
    return errors


def test_process_supervision_unsafe_exception_is_narrow() -> list[str]:
    source_root = ROOT / "native" / "process_supervision" / "src"
    source_files = sorted(source_root.rglob("*.rs"))
    unsafe_files = [
        path.relative_to(ROOT).as_posix()
        for path in source_files
        if re.search(r"\bunsafe\s*\{", path.read_text(encoding="utf-8"))
    ]
    allowed_file = "native/process_supervision/src/windows_job.rs"
    errors = []
    if unsafe_files != [allowed_file]:
        errors.append(f"process supervisionのunsafe境界が指定file外へ拡張された: {unsafe_files}")

    crate_lib = ROOT / "native" / "process_supervision" / "src" / "lib.rs"
    if not crate_lib.is_file() or "#![deny(unsafe_op_in_unsafe_fn)]" not in crate_lib.read_text(encoding="utf-8"):
        errors.append("process supervision crateはunsafe_op_in_unsafe_fnをdenyしなければならない")

    broker_lib = RUST_HELPER / "src" / "lib.rs"
    if "#![forbid(unsafe_code)]" not in broker_lib.read_text(encoding="utf-8"):
        errors.append("process supervision例外によってRust Brokerのunsafe禁止を弱めてはならない")

    if unsafe_files == [allowed_file]:
        source = (ROOT / allowed_file).read_text(encoding="utf-8")
        unsafe_blocks = len(re.findall(r"\bunsafe\s*\{", source))
        safety_reasons = len(re.findall(r"(?m)^\s*//\s*SAFETY:", source))
        if unsafe_blocks != safety_reasons:
            errors.append(
                f"process supervisionのunsafe block {unsafe_blocks}件に対しSAFETY根拠は{safety_reasons}件"
            )

    contract = DOC_SPECS / "process-supervision.md"
    if not contract.is_file():
        errors.append("process supervisionの明示レビュー契約がない")
    else:
        contract_text = contract.read_text(encoding="utf-8")
        for required in ("KILL_ON_JOB_CLOSE", "task_execution=unsupported", "sandbox"):
            if required not in contract_text:
                errors.append(f"process supervision契約に必須境界がない: {required}")
    return errors


def test_rust_broker_rejection_audit_contract_shape() -> list[str]:
    protocol_rs = (RUST_HELPER / "src" / "broker" / "protocol.rs").read_text(encoding="utf-8")
    audit_rs = (RUST_HELPER / "src" / "broker" / "audit.rs").read_text(encoding="utf-8")
    errors = []
    parser_path = RUST_HELPER / "src" / "broker" / "json_input.rs"
    if not parser_path.is_file():
        errors.append("一意JSON parserが存在しない")
    else:
        parser_text = parser_path.read_text(encoding="utf-8")
        for token in ("serde_json::from_str(raw)?", "serde_json::from_value(value.0)", "v.insert(k, x.0).is_some()", "重複field"):
            if token not in parser_text:
                errors.append(f"一意JSON parserに必須経路がない: {token}")
    required_protocol_tokens = [
        "BrokerRequestEnvelope",
        "BrokerResponse",
        "BrokerStatus::Rejected",
        "BrokerStatus::Suspended",
        "from_json_str",
        "handle_json",
        "to_json_string",
        "super::json_input::read_unique",
        "broker_request_malformed",
        "broker_payload_hash_invalid",
        "broker_payload_hash_mismatch",
        "canonical_payload_hash",
        "broker_issued_at_invalid",
        "broker_persistence_unavailable",
        "broker_stale_session",
        "broker_replay_detected",
        "broker_authority_metadata_rejected",
        "broker_command_dispatch_disabled",
        "metadata_attempts_authority",
        "normalize_key",
        "UnicodeNormalization",
        "nfkc",
        "boundary_role",
        "authority_cutover_status",
        "BrokerPersistenceMode",
        "BrokerStateStore",
        "in_memory_skeleton",
        "in_memory_session_only",
        "session_persistence",
        "persistence_required",
        "persistence_ready",
        "persistent audit, replay, and session state",
        "REQUEST_FRESHNESS_WINDOW_SECONDS",
        "parse_issued_at_epoch_seconds",
    ]
    for token in required_protocol_tokens:
        if token not in protocol_rs:
            errors.append(f"broker protocolにtokenがない: {token}")
    for token in ["BrokerAuditLog", "append", "previous_event_hash", "event_hash", "payload_hash"]:
        if token not in audit_rs:
            errors.append(f"broker auditにtokenがない: {token}")
    return errors


def test_rust_broker_audit_anchor_and_nonce_compaction_present() -> list[str]:
    store_rs = (RUST_HELPER / "src" / "broker" / "store.rs").read_text(encoding="utf-8")
    audit_hash_rs = (RUST_HELPER / "src" / "audit_hash.rs").read_text(encoding="utf-8")
    errors = []
    for token in [
        "audit_anchor.json",
        "audit_anchor.key",
        "AuditAnchorRecord",
        "anchor_hmac",
        "verify_audit_anchor",
        "write_audit_anchor",
        "recorded_at_epoch_seconds",
        "REPLAY_NONCE_RETENTION_SECONDS",
        "MAX_REPLAY_NONCE_RECORDS",
        "compact_replay_nonces",
    ]:
        if token not in store_rs:
            errors.append(f"broker storeにaudit anchor/nonce tokenがない: {token}")
    if "hmac_sha256_tagged" not in audit_hash_rs:
        errors.append("audit_hash.rsにHMAC helperがない")
    return errors


def test_rust_filesystem_diagnostic_detects_secret_symlink() -> list[str]:
    filesystem_rs = (RUST_HELPER / "src" / "filesystem.rs").read_text(encoding="utf-8")
    errors = []
    for token in [
        "symlink_metadata",
        "canonicalize",
        "secret_path_detected",
        "filesystem_secret_path_diagnostic_blocked",
        "filesystem_diagnostic_detects_symlink_to_secret",
    ]:
        if token not in filesystem_rs:
            errors.append(f"filesystem diagnosticにsymlink secret tokenがない: {token}")
    return errors


def test_desktop_flutter_does_not_spawn_python_or_use_ffi_authority_bridge() -> list[str]:
    forbidden = [
        "Process.run",
        "Process.start",
        "Process.killPid",
        "dart:ffi",
        "flutter_rust_bridge",
    ]
    errors = []
    for path in sorted((DESKTOP_FLUTTER / "lib").rglob("*.dart")):
        text = path.read_text(encoding="utf-8")
        for token in forbidden:
            if token in text:
                errors.append(f"{path.relative_to(ROOT)} が禁止されたruntime bridge tokenを含む: {token}")
        if "MethodChannel(" in text:
            allowed_tray = (
                path == DESKTOP_FLUTTER / "lib" / "services" / "windows_tray_client.dart"
                and "MethodChannel('gui_shell/tray')" in text
                and "requestAllRuntimeStop" not in text
                and "Process.run" not in text
                and "Process.start" not in text
                and "dart:ffi" not in text
                and "flutter_rust_bridge" not in text
            )
            allowed_broker_transport = (
                path == DESKTOP_FLUTTER / "lib" / "services" / "broker_client.dart"
                and "MethodChannel('gui_shell/broker')" in text
                and text.count("MethodChannel(") == 1
                and "invokeMethod<String>('request'" in text
                and "Socket.connect" not in text
                and "dart:io" not in text
                and "session_secret" not in text
                and "sessionSecret" not in text
                and "Process.run" not in text
                and "Process.start" not in text
                and "dart:ffi" not in text
                and "flutter_rust_bridge" not in text
            )
            if not (allowed_tray or allowed_broker_transport):
                errors.append(f"{path.relative_to(ROOT)} が許可外のMethodChannel(を含む")
    return errors


def test_release_docs_declare_language_policy_runtime_blockers() -> list[str]:
    required_token_groups = [
        ("rust security broker",),
        ("release_blocker",),
        ("production ipc", "製品 ipc"),
        ("no-python-runtime",),
        ("no-ffi-authority",),
    ]
    required_docs = [
        "ROADMAP.md",
        "CLAIM.md",
        "RELEASE_CHECKLIST.md",
        "docs/PHASE_STRATEGY.md",
        "docs/implementation/LANGUAGE_POLICY_GAP_ANALYSIS.md",
        "docs/implementation/RUNTIME_OWNERSHIP_AUDIT.md",
    ]
    errors = []
    for relative in required_docs:
        text = (ROOT / relative).read_text(encoding="utf-8").lower()
        for token_group in required_token_groups:
            if not any(token in text for token in token_group):
                errors.append(
                    f"{relative} にlanguage-policy blocker tokenがない: "
                    + " / ".join(token_group)
                )
    return errors


def test_blue_tanuki_adapter_runtime_output_validates_against_generic_schema() -> list[str]:
    adapter = BlueTanukiAdapter()
    runtime = adapter.runtime_snapshot()
    diagnostic = adapter.diagnostics_export()
    recovery = adapter.recovery_actions()[0]
    audit = adapter.audit_events()[0]
    approval = adapter.approvals()[0]
    schema_pairs = [
        ("runtime.schema.json", runtime),
        ("diagnostic.schema.json", diagnostic),
        ("recovery.schema.json", recovery),
        ("audit.schema.json", audit),
        ("approval.schema.json", approval),
    ]
    errors = []
    for schema_name, value in schema_pairs:
        schema = load_schema(schema_name)
        for failure in validate_instance(value, schema):
            errors.append(f"BLUE-TANUKI adapter {schema_name} のvalidationが失敗: {failure}")
    return errors


def test_blue_tanuki_adapter_metadata_cannot_escalate_authority() -> list[str]:
    metadata = {"permissions": ["filesystem.write"], "trust_level": "root"}
    trace = BlueTanukiAdapter().authority_trace()
    errors = []
    if not metadata_attempts_authority(metadata):
        errors.append("BLUE-TANUKI adapterがauthority-like metadataを検出しなかった")
    if trace.get("metadata_trusted") is not False:
        errors.append("BLUE-TANUKI adapterがmetadataを信頼した")
    if trace.get("adapter_can_grant_permission") is not False:
        errors.append("BLUE-TANUKI adapterがpermissionを付与できる")
    return errors


def test_blue_tanuki_adapter_cannot_expose_full_payload_unless_visibility_full() -> list[str]:
    errors = []
    for visibility in ["none", "hash_only", "summary", "redacted"]:
        projected = projected_approval({"content_visibility": visibility})
        if "full_payload" in projected:
            errors.append(f"BLUE-TANUKI adapterが{visibility}でfull payloadを公開した")
    if "full_payload" not in projected_approval({"content_visibility": "full"}):
        errors.append("BLUE-TANUKI adapterがvisibilityがfullのときfull payloadを公開しなかった")
    return errors


def test_blue_tanuki_adapter_cannot_mark_approvals_approved_by_itself() -> list[str]:
    approval = normalize_approval({"status": "approved", "approved_by": "adapter", "adapter_approved": True})
    if approval["status"] == "approved":
        return ["BLUE-TANUKI adapterがapprovalを自己承認した"]
    return []


def test_blue_tanuki_adapter_failures_map_to_recovery_actions() -> list[str]:
    candidates = recovery_candidates("runtime_down")
    if not candidates:
        return ["BLUE-TANUKI adapterの失敗がRecoveryAction candidateを生成しなかった"]
    schema = load_schema("recovery.schema.json")
    errors = []
    for candidate in candidates:
        errors.extend(validate_instance(candidate, schema))
    return [f"BLUE-TANUKI adapterのrecovery validationが失敗: {error}" for error in errors]


def test_desktop_flutter_required_files_exist() -> list[str]:
    errors = []
    for relative in sorted(DESKTOP_FLUTTER_REQUIRED_FILES):
        if not (DESKTOP_FLUTTER / relative).exists():
            errors.append(f"apps/desktop_flutter/{relative} が存在しない")
    return errors


def test_desktop_flutter_keeps_authority_in_shell_core_client() -> list[str]:
    errors = []
    dart_files = sorted((DESKTOP_FLUTTER / "lib").glob("**/*.dart"))
    forbidden_assignments = [
        "adapter_can_grant_permission: true",
        "adapter_can_approve: true",
        "metadata_trusted: true",
        "'full_payload'",
    ]
    for path in dart_files:
        text = path.read_text(encoding="utf-8")
        for pattern in forbidden_assignments:
            if pattern in text:
                errors.append(f"{path} が禁止されたUI authority patternを含む: {pattern}")
    client = (DESKTOP_FLUTTER / "lib" / "services" / "shell_core_client.dart").read_text(encoding="utf-8")
    if "full_payload_projected_without_full_visibility': false" not in client:
        errors.append("desktop Flutterのmock clientがShell Coreのinvariant statusを公開していない")
    if "ShellSnapshot? snapshot" not in client:
        errors.append("desktop Flutterのlocal clientが明示注入されたsnapshotを受け取らない")
    forbidden_local_io = [
        "dart:io",
        "File(",
        "Directory(",
        "Platform.environment",
        "readAsString",
        "existsSync(",
        "LOCALAPPDATA",
        "GUI_SHELL_SNAPSHOT_JSON",
    ]
    for token in forbidden_local_io:
        if token in client:
            errors.append(f"desktop Flutterのlocal clientが直接I/O参照を含む: {token}")
    if "completedProductReleaseClaimed: false" not in client:
        errors.append("desktop Flutterのlocal clientが注入データのrelease claimを抑止しない")
    if "releaseState: 'not claimed'" not in client:
        errors.append("desktop Flutterのlocal clientが注入データのrelease stateを抑止しない")
    return errors


def test_desktop_flutter_windows_runner_rejects_native_surface_aggregate_injection() -> list[str]:
    runner = DESKTOP_FLUTTER / "windows" / "runner" / "flutter_window.cpp"
    if not runner.exists():
        return ["Windows Flutter runnerがない: apps/desktop_flutter/windows/runner/flutter_window.cpp"]
    text = runner.read_text(encoding="utf-8")
    required_labels = ["Dashboard", "NavigationRail", "Runtime Status", "Invariant Status"]
    aggregate = "GUI Shell Dashboard NavigationRail Runtime Status Invariant Status"
    errors = []
    if aggregate in text:
        errors.append("Windows runnerが禁止されたaggregate native surface titleを含む")
    set_window_text_blocks = re.findall(r"SetWindowText\s*\([^;]*;", text, flags=re.DOTALL)
    for block in set_window_text_blocks:
        labels = [label for label in required_labels if label in block]
        if labels:
            errors.append(
                "Windows runnerのSetWindowTextが必須のsurface labelを含む: "
                + ", ".join(labels)
            )
    return errors


def test_desktop_flutter_windows_uses_platform_accessibility_lifecycle() -> list[str]:
    runner_main = DESKTOP_FLUTTER / "windows" / "runner" / "main.cpp"
    dart_main = DESKTOP_FLUTTER / "lib" / "main.dart"
    if not runner_main.exists() or not dart_main.exists():
        return ["Desktop Windows accessibilityの検査対象fileがない"]
    runner_text = runner_main.read_text(encoding="utf-8")
    dart_text = dart_main.read_text(encoding="utf-8")
    project = 'flutter::DartProject project(L"data");'
    mode = "project.set_accessibility_mode(flutter::AccessibilityMode::IAccessibleEx);"
    window = "FlutterWindow window(project);"
    errors = []
    if project not in runner_text or mode not in runner_text or window not in runner_text:
        errors.append("Windows runnerがFlutter IAccessibleEx providerを選択しない")
    elif not runner_text.index(project) < runner_text.index(mode) < runner_text.index(window):
        errors.append("Windows runnerがFlutterView生成前にIAccessibleEx providerを選択しない")
    if "SemanticsBinding.instance.ensureSemantics()" in dart_text:
        errors.append("Desktop Dart rootがWindows OS要求前にSemantics treeを常時強制する")
    return errors


def test_desktop_flutter_exposes_individual_surface_semantics_identifiers() -> list[str]:
    shared = (DESKTOP_FLUTTER / "lib" / "screens" / "shared.dart").read_text(encoding="utf-8")
    main = (DESKTOP_FLUTTER / "lib" / "main.dart").read_text(encoding="utf-8")
    widget_test = (DESKTOP_FLUTTER / "test" / "widget_test.dart").read_text(encoding="utf-8")
    required = {
        "Dashboard": "gui_shell.surface.dashboard",
        "NavigationRail": "gui_shell.surface.navigation_rail",
        "Runtime Status": "gui_shell.surface.runtime_status",
        "Invariant Status": "gui_shell.surface.invariant_status",
    }
    errors = []
    if "writeSurfaceSemanticsExportIfRequested" in main or "surface_semantics_export.dart" in main:
        errors.append("desktop Flutter mainが内部Semantics registryをfileへ書き出す")
    for label, identifier in required.items():
        if identifier not in shared:
            errors.append(f"desktop Flutterのsurface semantics identifierが{label}にない")
        if label not in shared and label not in main:
            errors.append(f"desktop Flutterのsurface labelがない: {label}")
    if "SurfaceSemantics(" not in main or "evidenceLabel: 'NavigationRail'" not in main:
        errors.append("NavigationRailの安定証拠IDがSurfaceSemanticsから公開されていない")
    if "bySemanticsIdentifier(surfaceSemanticsIdentifier(label))" not in widget_test and (
        "surfaceSemanticsIdentifier(label)" not in widget_test
        or "properties.identifier == identifier" not in widget_test
    ):
        errors.append("desktop Flutterのwidget testがper-surface semantics identifierを検証しない")
    return errors


def test_desktop_flutter_product_baseline_chrome_exists() -> list[str]:
    main = (DESKTOP_FLUTTER / "lib" / "main.dart").read_text(encoding="utf-8")
    windows_main = (DESKTOP_FLUTTER / "windows" / "runner" / "main.cpp").read_text(encoding="utf-8")
    win32 = (DESKTOP_FLUTTER / "windows" / "runner" / "win32_window.cpp").read_text(encoding="utf-8")
    linux = (DESKTOP_FLUTTER / "linux" / "runner" / "my_application.cc").read_text(encoding="utf-8")
    widget_test = (DESKTOP_FLUTTER / "test" / "widget_test.dart").read_text(encoding="utf-8")
    errors = []
    for token in [
        "runZonedGuarded",
        "PlatformDispatcher.instance.onError",
        "ErrorWidget.builder",
        "GuiShellFatalErrorScreen",
        "themeMode: switch (initialConfiguration.theme)",
        "darkTheme: _buildShellTheme(",
        "localizationsDelegates: GlobalMaterialLocalizations.delegates",
        "kGuiShellProductTitle",
    ]:
        if token not in main:
            errors.append(f"desktop Flutterのproduct baselineにtokenがない: {token}")
    if 'window.Create(L"D4 Pocket", origin, size)' not in windows_main:
        errors.append("Windows runnerのproduct window titleがD4 Pocketではない")
    if "Win32Window::Size size(1280, 800)" not in windows_main:
        errors.append("Windows runnerのdefault product window sizeが1280x800に固定されていない")
    for token in ["WM_GETMINMAXINFO", "kMinWindowWidth = 1024", "kMinWindowHeight = 640"]:
        if token not in win32:
            errors.append(f"Windows runnerのminimum-size guardにtokenがない: {token}")
    for token in [
        'gtk_header_bar_set_title(header_bar, "GUI Shell")',
        'gtk_window_set_title(window, "GUI Shell")',
        "gtk_window_set_default_size(window, 1280, 800)",
        "gtk_widget_set_size_request(GTK_WIDGET(window), 1024, 640)",
    ]:
        if token not in linux:
            errors.append(f"Linux runnerのproduct window baselineにtokenがない: {token}")
    for token in ["GUI Shellデスクトップアプリが製品基準の外枠を持つ", "ThemeMode.system"]:
        if token not in widget_test:
            errors.append(f"desktop Flutterのwidget baseline testにtokenがない: {token}")
    return errors


def test_validate_all_uses_running_python_interpreter_for_python_steps() -> list[str]:
    errors = []
    steps = build_validation_steps(False, "windows", python_only=True)
    names = {step.name for step in steps}
    for step in steps:
        if step.command[0] != sys.executable:
            errors.append(f"{step.name} がsys.executableを使っていない")
        if step.required_tool is not None:
            errors.append(f"{step.name} がPATH上のPython tool必須をまだ宣言している")
    if "broker_authority_parity" in names:
        errors.append("validate_all --python-onlyがcargo-backed broker_authority_parityをまだ含む")
    return errors


def test_desktop_flutter_exposes_operation_surfaces() -> list[str]:
    main = (DESKTOP_FLUTTER / "lib" / "main.dart").read_text(encoding="utf-8")
    dashboard = (DESKTOP_FLUTTER / "lib" / "screens" / "dashboard.dart").read_text(encoding="utf-8")
    runtime = (DESKTOP_FLUTTER / "lib" / "screens" / "runtime_center.dart").read_text(encoding="utf-8")
    settings = (DESKTOP_FLUTTER / "lib" / "screens" / "settings.dart").read_text(encoding="utf-8")
    audit = (DESKTOP_FLUTTER / "lib" / "screens" / "audit_viewer.dart").read_text(encoding="utf-8")
    recovery = (DESKTOP_FLUTTER / "lib" / "screens" / "recovery_center.dart").read_text(encoding="utf-8")
    trust = (DESKTOP_FLUTTER / "lib" / "screens" / "trust_center.dart").read_text(encoding="utf-8")
    authority = (DESKTOP_FLUTTER / "lib" / "screens" / "authority_map.dart").read_text(encoding="utf-8")
    problems = (DESKTOP_FLUTTER / "lib" / "screens" / "problems_panel.dart").read_text(encoding="utf-8")
    evidence = (DESKTOP_FLUTTER / "lib" / "screens" / "evidence_center.dart").read_text(encoding="utf-8")
    combined = "\n".join([main, dashboard, runtime, settings, audit, recovery, trust, authority, problems, evidence])
    required = [
        "TrustCenter",
        "AuthorityMap",
        "アダプター台帳",
        "許可差分",
        "問題一覧",
        "証拠センター",
        "コマンドパレット",
        "事象をコピー／JSONLを書き出し／鎖を検証",
        "事前確認",
        "ShellStatusBar",
    ]
    return [f"desktop Flutterのoperation surfaceがない: {token}" for token in required if token not in combined]


def test_installer_setup_doctor_reports_structured_status_without_authority() -> list[str]:
    from installer.setup_doctor import setup_doctor_report

    report = setup_doctor_report()
    errors = []
    for key in ["status", "checks", "installer_grants_authority", "installer_silently_approves_permissions"]:
        if key not in report:
            errors.append(f"Setup Doctor reportに{key}がない")
    if report.get("installer_grants_authority") is not False:
        errors.append("installerがauthorityを付与する")
    if report.get("installer_silently_approves_permissions") is not False:
        errors.append("installerがpermissionを黙示承認する")
    for check in report.get("checks", []):
        if check.get("grants_authority") is not False:
            errors.append(f"Setup Doctorのcheckがauthorityを付与する: {check.get('check_id')}")
        if check.get("status") in {"fail", "warning"} and not check.get("recovery_instruction"):
            errors.append(f"Setup Doctorのcheckにrecovery instructionがない: {check.get('check_id')}")
    return errors


def test_installer_boundary_docs_exist() -> list[str]:
    required = ["FIRST_RUN.md", "SETUP_DOCTOR.md", "INSTALLER_BOUNDARY.md"]
    errors = []
    for name in required:
        if not (ROOT / "docs" / name).exists():
            errors.append(f"docs/{name} が存在しない")
    if not (INSTALLER / "setup_doctor.py").exists():
        errors.append("installer/setup_doctor.pyが存在しない")
    return errors


def test_mobile_flutter_required_files_exist() -> list[str]:
    errors = []
    for relative in sorted(MOBILE_FLUTTER_REQUIRED_FILES):
        if not (MOBILE_FLUTTER / relative).exists():
            errors.append(f"apps/mobile_flutter/{relative} が存在しない")
    return errors


def test_mobile_flutter_cannot_create_hidden_authority() -> list[str]:
    required_terms = ["device_id", "pairing_id", "操作者が照合", "監査事象", "取消し", "復旧経路"]
    dart_sources = sorted((MOBILE_FLUTTER / "lib").glob("**/*.dart"))
    native_sources = sorted((MOBILE_FLUTTER / "android/app/src/main/kotlin").glob("**/*.kt"))
    combined = "\n".join(path.read_text(encoding="utf-8") for path in [*dart_sources, *native_sources])
    errors = []
    for term in required_terms:
        if term not in combined:
            errors.append(f"mobile pairing contractのtermがない: {term}")
    dart_only = "\n".join(path.read_text(encoding="utf-8") for path in dart_sources)
    for marker in ("import 'dart:io'", "SecureSocket", "flutter_secure_storage", "class DeviceCredential", "class SecureDeviceStore"):
        if marker in dart_only:
            errors.append("Mobile Dartが資格・network・安全保管を直接扱う: " + marker)
    main_activity = (MOBILE_FLUTTER / "android/app/src/main/kotlin/com/example/gui_shell_mobile/MainActivity.kt").read_text(encoding="utf-8")
    native_service = (MOBILE_FLUTTER / "android/app/src/main/kotlin/com/example/gui_shell_mobile/DeviceLinkNativeService.kt").read_text(encoding="utf-8")
    if "onResume()" not in main_activity or "setForeground(true)" not in main_activity or "onPause()" not in main_activity or "setForeground(false)" not in main_activity:
        errors.append("Android native foregroundの正本がActivity lifecycleへ接続されていない")
    if '"set_foreground"' in native_service:
        errors.append("FlutterからAndroid native foregroundを設定できるchannel methodが残っている")
    if "pairingInProgress.compareAndSet(false, true)" not in native_service:
        errors.append("Android native pairingが重複開始を排他していない")
    if native_service.count("check(!pairingInProgress.get())") < 3:
        errors.append("Android native pairing中に通常要求・解除が許可される可能性がある")
    forbidden = ["independent authority: true", "silently pair", "'full_payload'", "hidden payload available"]
    for pattern in forbidden:
        if pattern in combined:
            errors.append(f"mobile Flutterが禁止されたauthority patternを含む: {pattern}")
    return errors


def test_release_hardening_files_exist() -> list[str]:
    errors = []
    for relative in sorted(RELEASE_HARDENING_FILES):
        if not (ROOT / relative).exists():
            errors.append(f"{relative} が存在しない")
    return errors


def test_release_hardening_does_not_overclaim_readiness() -> list[str]:
    errors = []
    forbidden_claims = [
        "production ready",
        "installer ready",
        "mobile ready",
        "stable runtime support",
        "security complete",
    ]
    for relative in sorted(RELEASE_HARDENING_FILES):
        text = (ROOT / relative).read_text(encoding="utf-8").lower()
        for claim in forbidden_claims:
            if claim in text and "not " + claim not in text:
                errors.append(f"{relative} overclaims {claim}")
    return errors


def test_validation_reporter_exists() -> list[str]:
    path = ROOT / "tooling" / "validate_all.py"
    if not path.exists():
        return ["tooling/validate_all.pyが存在しない"]
    text = path.read_text(encoding="utf-8")
    errors = []
    for token in ["schema_check", "conformance_skeleton", "manifest_check", "release_gate_check", "rust_helper_cargo_test", "desktop_flutter_analyze", "desktop_flutter_test", "desktop_flutter_build_linux", "mobile_flutter_analyze", "strict-release", "desktop-platform", "windows", "linux", "macos"]:
        if token not in text:
            errors.append(f"validate_all.pyにvalidation tokenがない: {token}")
    return errors


def test_validate_all_resolves_windows_batch_commands() -> list[str]:
    text = (ROOT / "tooling" / "validate_all.py").read_text(encoding="utf-8")
    errors = []
    for token in [
        "def find_tool(",
        "def resolve_step_command(",
        '".exe"',
        '".bat"',
        '".cmd"',
        "step_command = resolve_step_command(step.command)",
        "find_tool(step.required_tool)",
    ]:
        if token not in text:
            errors.append(f"validate_all.pyにWindows command resolution tokenがない: {token}")
    return errors


def test_manifest_integrity_tooling_exists() -> list[str]:
    errors = []
    required_paths = {
        "AGENTS.md",
        "ROADMAP.md",
        "CONFORMANCE_REPORT.md",
        "COMPATIBILITY_MATRIX.md",
        "apps/mobile_flutter/lib/main.dart",
        "apps/desktop_flutter/windows/runner/main.cpp",
        "docs/specs/gui-shell-spec-v1.md",
        "docs/LANGUAGE_POLICY.md",
        "packages/agent_runtime/contract.py",
        "packages/runtime_catalog/catalog.py",
        "packages/shell_contracts/schema_loader.py",
        "packages/blue_tanuki_adapter/adapter.py",
        "tooling/manifest.py",
        "tooling/conformance_tests/run_conformance_skeleton.py",
    }
    manifest, manifest_errors = build_manifest()
    errors.extend(manifest_errors)
    listed = {entry["path"] for entry in manifest["files"]}
    for path in sorted(required_paths - listed):
        errors.append(f"manifestが期待するsourceが生成file一覧にない: {path}")

    forbidden_paths = [
        "MANIFEST.sha256.json",
        "build/out.txt",
        "target/out.txt",
        ".dart_tool/cache",
        "__pycache__/x.pyc",
        "apps/desktop_flutter/build/out.txt",
        "native/rust_helper/target/out.txt",
        "apps/mobile_flutter/pubspec.lock",
        "../outside.txt",
        "/tmp/out.txt",
    ]
    for path in forbidden_paths:
        if not matches_forbidden(path):
            errors.append(f"manifestで禁止されたpathが受け入れられた: {path}")

    allowed_paths = [
        "docs/LANGUAGE_POLICY.md",
        "packages/shell_core/runtime_state.py",
        "tooling/manifest.py",
    ]
    for path in allowed_paths:
        if matches_forbidden(path):
            errors.append(f"manifestのsource pathが拒否された: {path}")
    return errors


def test_manifest_rejects_working_tree_eol_mismatch() -> list[str]:
    errors = []
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        subprocess.run(["git", "init", "--quiet", str(root)], check=True)
        (root / ".gitattributes").write_bytes(b"* text=auto eol=lf\n*.bat text eol=crlf\n*.png binary\n")
        text = root / "source.txt"
        text.write_bytes(b"first\nsecond\n")
        (root / "run.bat").write_bytes(b"@echo off\r\n")
        (root / "image.png").write_bytes(b"\x00\r\n\xff")
        subprocess.run(["git", "add", "."], cwd=root, check=True, capture_output=True)
        if working_tree_eol_errors(root):
            errors.append("規定LF・明示CRLF・binaryを誤拒否した")
        for invalid in (b"first\r\nsecond\r\n", b"first\nsecond\r\n"):
            text.write_bytes(invalid)
            rejected = working_tree_eol_errors(root)
            if len(rejected) != 1 or "source.txt" not in rejected[0]:
                errors.append("CRLFまたは混在改行の生成を拒否しなかった")
        text.write_bytes(b"first\nsecond\n")
        if working_tree_eol_errors(root):
            errors.append("規定改行への修復後も拒否した")
    return errors


def test_claim_documents_do_not_contain_stale_phase_or_check_counts() -> list[str]:
    stale_patterns = ["23 checks", "49 checks", "51 checks", "53 checks", "55 checks", "Phase 0 / Phase 1"]
    errors = []
    for relative in sorted(CLAIM_REVIEW_FILES):
        text = (ROOT / relative).read_text(encoding="utf-8")
        for pattern in stale_patterns:
            if pattern in text:
                errors.append(f"{relative} がstaleのclaim textを含む: {pattern}")
    return errors


def test_runtime_manifest_invalid_fixture_rejected() -> list[str]:
    schema = load_schema("runtime_manifest.schema.json")
    invalid = json.loads((INVALID_CONTRACT_EXAMPLES / "runtime_manifest_unsigned.invalid.json").read_text(encoding="utf-8"))
    if not validate_instance(invalid, schema):
        return ["runtime manifestのinvalid fixtureが受け入れられた"]
    return []


def test_adapter_manifest_authority_escalation_rejected() -> list[str]:
    schema = load_schema("adapter_manifest.schema.json")
    invalid = json.loads((INVALID_CONTRACT_EXAMPLES / "adapter_manifest_authority_escalation.invalid.json").read_text(encoding="utf-8"))
    errors = []
    if not validate_instance(invalid, schema):
        errors.append("adapter manifestのauthority escalation fixtureが受け入れられた")
    catalog = RuntimeCatalog()
    if not catalog.metadata_attempts_authority(invalid.get("metadata", {})):
        errors.append("RuntimeCatalogがadapter manifest metadataのauthority試行を検出しなかった")
    return errors


def test_runtime_catalog_cannot_grant_authority() -> list[str]:
    catalog = RuntimeCatalog()
    manifest = load_contract_fixture("runtime_manifest.valid.json")
    catalog.register_runtime_manifest(manifest)
    if catalog.can_grant_authority(manifest):
        return ["RuntimeCatalogがmanifestからauthorityを付与した"]
    return []


def 作業領域応答の露出境界を検査する() -> list[str]:
    schema = load_schema("workspace_inspection_response.schema.json")
    base = load_contract_fixture("workspace_inspection_response.valid.json")
    errors = validate_instance(base, schema)
    for visibility, projection in [("none", None), ("hash_only", {"sha256": "sha256:" + "a" * 64}), ("summary", {"説明": "この表示範囲に提供できる承認済み内容はありません"}), ("redacted", {"説明": "この表示範囲に提供できる承認済み内容はありません"})]:
        value = dict(base, 表示範囲=visibility, projection=projection)
        errors.extend(validate_instance(value, schema))
        if not validate_instance(dict(value, projection=base["projection"]), schema):
            errors.append("制限付き応答に全文を混入できた")
    for field in ("登録hash", "approval_id", "有効期限", "operation", "要求hash"):
        wrong = dict(base)
        del wrong[field]
        if not validate_instance(wrong, schema):
            errors.append("応答結合fieldの欠落を受理した")
    if not validate_instance(dict(base, projection=dict(base["projection"], binary=True)), schema):
        errors.append("binary本文を受理した")
    for kind, size in (("directory", None), ("file", 0)):
        entry = {"path": "entry", "kind": kind, "bytes": size}
        tree = dict(base, operation="作業領域ツリー", projection={"entries": [entry]})
        errors.extend(validate_instance(tree, schema))
        wrong = dict(tree, projection={"entries": [dict(entry, bytes=0 if size is None else None)]})
        if not validate_instance(wrong, schema):
            errors.append("fileとdirectoryのサイズ表現を混同した")
    for name in ("ipc_request", "ipc_response"):
        operations = load_schema(name + ".schema.json")["properties"]["operation"]["enum"]
        for operation in ("作業領域一覧", "作業領域承認", "作業領域失効", "作業領域ツリー", "作業領域読取"):
            if operation not in operations:
                errors.append("通常IPCの作業領域操作が欠落")
    return errors


def test_host_capability_is_observation_not_authority() -> list[str]:
    schema = load_schema("host_capability.schema.json")
    valid = load_contract_fixture("host_capability.valid.json")
    errors = validate_instance(valid, schema)
    if errors:
        return [f"host capabilityの正常fixtureが不正: {errors}"]
    invalid = json.loads(
        (INVALID_CONTRACT_EXAMPLES / "host_capability_authority.invalid.json").read_text(
            encoding="utf-8"
        )
    )
    if not validate_instance(invalid, schema):
        return ["host capabilityがPermissionをauthorityとして受け入れた"]
    if any("Permission" in item or "Approval" in item for item in valid["能力"]):
        return ["host capabilityの正常fixtureにauthority fieldが混入している"]
    return []


def 作業領域検査要求の分岐を検査する() -> list[str]:
    schema = load_schema("workspace_inspection_request.schema.json")
    samples = {
        "作業領域一覧": {},
        "作業領域比較範囲": {"作業領域ID":"workspace-a", "相対path":""},
        "作業領域基準点保存": {"作業領域ID":"workspace-a", "登録hash":"sha256:" + "a" * 64, "相対paths":["file.txt"]},
        "作業領域差分": {"作業領域ID":"workspace-a", "相対path":"file.txt", "基準点hash":"sha256:" + "a" * 64},
        "作業領域承認": {"作業領域ID": "workspace-a", "登録hash": "sha256:" + "a" * 64, "表示範囲": "full"},
        "作業領域失効": {"作業領域ID": "workspace-a", "登録hash": "sha256:" + "a" * 64},
        "作業領域ツリー": {"作業領域ID": "workspace-a", "相対path": ""},
        "作業領域読取": {"作業領域ID": "workspace-a", "相対path": "file.txt"},
    }
    errors = []
    for operation, payload in samples.items():
        errors.extend(validate_instance({"operation": operation, "payload": payload}, schema))
        for forbidden in ("root", "authority", "permission", "approval_id", "表示範囲追加"):
            if not validate_instance({"operation": operation, "payload": {**payload, forbidden: "full"}}, schema):
                errors.append("作業領域要求が未知fieldを受理した")
        if payload and not validate_instance({"operation": operation, "payload": {}}, schema):
            errors.append("作業領域要求が必須field欠落を受理した")
    if not validate_instance({"operation": "作業領域読取", "payload": {"作業領域ID": "workspace-a", "相対path": ""}}, schema):
        errors.append("file読取が空pathを受理した")
    return errors


def test_workspace_diff_content_shape() -> list[str]:
    schema = json.loads((SPECS / "workspace_diff.schema.json").read_text(encoding="utf-8"))
    base = {"version": 1, "kind": "binary", "before": None,
            "after": {"bytes": 1, "sha256": "sha256:" + "0" * 64}, "unified": None, "rows": []}
    for kind in ["binary", "oversized", "unchanged"]:
        item = dict(base, kind=kind)
        if validate_instance(item, schema):
            return ["metadataだけの差分構造が拒否された"]
        for injected in [dict(item, unified="本文"), dict(item, rows=[{"kind": "added", "before": None, "after": {"number": 1, "text": "本文", "newline": True}}])]:
            if not validate_instance(injected, schema):
                return ["本文禁止の差分へ内容を混入できた"]
    item = dict(base, kind="text", unified="--- /dev/null\n+++ b/file\n@@ -0,0 +1,1 @@\n+追加\n",
                rows=[{"kind": "added", "before": None, "after": {"number": 1, "text": "追加", "newline": True}}])
    if validate_instance(item, schema):
        return ["text差分の構造が拒否された"]
    for kind in ["same", "changed", "deleted"]:
        wrong = copy.deepcopy(item)
        wrong["rows"][0]["kind"] = kind
        if not validate_instance(wrong, schema):
            return ["左右行と変更種類の矛盾を受理した"]
    return []


def test_agent_workspace_outside_access_default_deny() -> list[str]:
    workspace = load_contract_fixture("agent_workspace.valid.json")
    contract = AgentRuntimeContract(workspace)
    if contract.path_allowed("/outside/project/file.txt"):
        return ["agent runtimeがworkspace外へのaccessをdefaultで許可した"]
    return []


def test_agent_task_request_cannot_carry_authority_or_dialogue_approval() -> list[str]:
    schema = load_schema("agent_task_request.schema.json")
    valid = load_contract_fixture("agent_task_request.valid.json")
    errors = validate_instance(valid, schema)
    if errors:
        return [f"Agent作業要求の正常例が契約に適合しない: {errors}"]

    expected_fields = {"agent_runtime_id", "session_id", "workspace_id", "instruction"}
    if set(valid) != expected_fields:
        return ["Agent作業要求fixtureのfield集合が固定されていない"]

    invalid = load_contract_fixture(
        "invalid/agent_task_request_authority_escalation.invalid.json"
    )
    if validate_instance(invalid, schema) == []:
        errors.append("Agent作業要求がPermission／Approval／Audit識別子を受け入れた")

    forbidden_fields = (
        "permission_id",
        "approval_id",
        "audit_event_id",
        "authority",
        "sandbox",
        "workspace_path",
        "cwd",
        "executable",
        "command",
        "credential",
    )
    for field in forbidden_fields:
        candidate = {**valid, field: "untrusted"}
        if validate_instance(candidate, schema) == []:
            errors.append(f"Agent作業要求が禁止fieldを受け入れた: {field}")

    for instruction in ("", " \n\t ", "x" * 32769):
        candidate = {**valid, "instruction": instruction}
        if validate_instance(candidate, schema) == []:
            errors.append("Agent作業要求の空白または上限超過instructionを拒否しない")

    contract_text = (DOC_SPECS / "agent-runtime.md").read_text(encoding="utf-8")
    if "対話Approval" not in contract_text:
        errors.append("Agent Runtime正本が対話Approvalと作業Task Approvalの分離を定義しない")
    if "Agent TaskのBroker実行Consumer" not in contract_text:
        errors.append("Agent Runtime正本がTask requestのBroker consumerを示さない")
    return errors


def test_agent_broker_operations_are_declared_in_ipc_contracts() -> list[str]:
    request_schema = load_schema("ipc_request.schema.json")
    response_schema = load_schema("ipc_response.schema.json")
    request_operations = set(request_schema["properties"]["operation"]["enum"])
    response_operations = set(response_schema["properties"]["operation"]["enum"])
    broker_source = (ROOT / "native/rust_helper/src/broker/protocol.rs").read_text(
        encoding="utf-8"
    )
    required_operations = (
        "Agent一覧",
        "AgentCLI実行系作業領域登録",
        "Agent作業要求検査",
        "AgentTaskWorkspacePermissionGrant",
        "AgentTaskOwnerApprovalGrant",
        "AgentTask実行",
        "AgentTask状態",
        "AgentTask取消",
    )
    errors: list[str] = []
    for operation in required_operations:
        if operation not in request_operations:
            errors.append(f"Agent Broker操作がIPC要求Schemaにない: {operation}")
        if operation not in response_operations:
            errors.append(f"Agent Broker操作がIPC応答Schemaにない: {operation}")
        if operation not in broker_source:
            errors.append(f"Agent Broker操作がRust protocolへ接続されていない: {operation}")
    return errors


def test_agent_cli_runtime_registration_is_owner_scoped_and_authority_free() -> list[str]:
    schema = load_schema("agent_cli_runtime_workspace_registration.schema.json")
    valid = load_contract_fixture("agent_cli_runtime_workspace_registration.valid.json")
    invalid = load_contract_fixture(
        "invalid/agent_cli_runtime_workspace_registration_authority.invalid.json"
    )
    errors: list[str] = []
    if validate_instance(valid, schema):
        errors.append("Agent CLI Runtime／Workspace登録の正常例が契約に適合しない")
    if validate_instance(invalid, schema) == []:
        errors.append("Agent CLI Runtime／Workspace登録要求がAuthority fieldを受け入れた")
    for field in ("permission", "approval_id", "credential_value", "owner"):
        if validate_instance({**valid, field: "untrusted"}, schema) == []:
            errors.append(f"Agent CLI Runtime／Workspace登録が禁止fieldを受け入れた: {field}")

    protocol = (ROOT / "native/rust_helper/src/broker/protocol.rs").read_text(
        encoding="utf-8"
    )
    launcher = (ROOT / "native/rust_helper/src/desktop_launcher.rs").read_text(
        encoding="utf-8"
    )
    broker_client = (ROOT / "apps/desktop_flutter/lib/services/broker_client.dart").read_text(
        encoding="utf-8"
    )
    adapter_registry = (ROOT / "native/rust_helper/src/adapters/mod.rs").read_text(
        encoding="utf-8"
    )
    for required in (
        "desktop_native_owner_confirmation_required",
        "desktop_install_path_verified",
        "self.対話.登録件数() >= 8",
        'let task_execution = if self.対話.AgentTask能力対応(&request.runtime_id)',
        '"task_execution": task_execution',
        "rollback_agent_cli_registration",
    ):
        if required not in protocol:
            errors.append(f"Agent CLI登録Broker境界が不足: {required}")
    for required in (
        "Agent CLI実行系と作業領域を、このDesktop起動中だけBrokerへ登録",
        "Adapter ID: {}",
        "cli_path",
        "workspace_root",
        "secret_paths",
    ):
        if required not in launcher:
            errors.append(f"native Owner確認が登録範囲を示さない: {required}")
    if "AgentCLI実行系作業領域登録" not in broker_client:
        errors.append("登録要求のnative Owner確認待ちがDesktop clientにない")
    native_owner_e2e_name = (
        "fn 登録CodexへのnativeOwner確認後もAgentTask非対応gateを維持する()"
    )
    native_owner_e2e_start = launcher.find(native_owner_e2e_name)
    native_owner_e2e_end = launcher.find("\n    #[test]", native_owner_e2e_start)
    native_owner_e2e = (
        launcher[native_owner_e2e_start:native_owner_e2e_end]
        if native_owner_e2e_start >= 0 and native_owner_e2e_end >= 0
        else ""
    )
    for required in (
        '"AgentCLI実行系作業領域登録"',
        "automate_native_owner_confirmation(",
        '"Agent作業要求検査"',
        '"AgentTask実行"',
        '"unsupported"',
        "synthetic-secret-content-never-returned",
    ):
        if required not in native_owner_e2e:
            errors.append(f"native Owner付きAgent CLI負のE2Eが境界を検査しない: {required}")
    if "supports_cli_adapter" not in adapter_registry or '"codex-cli"' not in adapter_registry:
        errors.append("CLI Adapter選択をBroker CoreではなくAdapter層で解決しない")
    if "crate::adapters::codex_cli::CodexCliAdapter" in protocol:
        errors.append("Broker CoreがCodex固有Adapterを直接生成する")
    return errors


def test_provider_model_selection_is_registered_and_non_authoritative() -> list[str]:
    schema = load_schema("provider_model_selection.schema.json")
    valid = load_contract_fixture("provider_model_selection.valid.json")
    vault_valid = load_contract_fixture("provider_model_selection_vault.valid.json")
    invalid = load_contract_fixture(
        "invalid/provider_model_selection_unknown_provider.invalid.json"
    )
    invalid_missing_credential = load_contract_fixture(
        "invalid/provider_model_selection_vault_missing_id.invalid.json"
    )
    invalid_credential_id = load_contract_fixture(
        "invalid/provider_model_selection_vault_malformed_id.invalid.json"
    )
    errors: list[str] = []
    if validate_instance(valid, schema):
        errors.append("Provider／Model選択の正常例が契約に適合しない")
    if validate_instance(vault_valid, schema):
        errors.append("Broker資格情報参照を使うProvider／Model選択例が契約に適合しない")
    for negative, label in (
        (invalid, "未登録Provider"),
        (invalid_missing_credential, "Credential ID欠落"),
        (invalid_credential_id, "不正Credential ID"),
    ):
        if validate_instance(negative, schema) == []:
            errors.append(f"Provider／Model選択が{label}を受け入れた")
    for changed in (
        {**valid, "automatic_fallback": True},
        {**valid, "model_id": "--dangerous"},
        {**valid, "model_id": "model\n"},
        {**valid, "model_id": "模型"},
        {**valid, "credential_value": "secret"},
        {**valid, "capabilities": ["task_execution"]},
    ):
        if validate_instance(changed, schema) == []:
            errors.append("Provider／Model選択が禁止値または権限類似fieldを受け入れた")
    registration = load_schema("agent_cli_runtime_workspace_registration.schema.json")
    if "provider_model_selection" not in registration.get("required", []):
        errors.append("Agent CLI登録がProvider／Model選択を必須にしていない")
    if registration.get("properties", {}).get("provider_model_selection", {}).get(
        "$ref"
    ) != "provider_model_selection.schema.json":
        errors.append("Agent CLI登録がProvider／Model選択Schemaを参照しない")
    adapter = (ROOT / "native/rust_helper/src/adapters/codex_cli.rs").read_text(
        encoding="utf-8"
    )
    fake_cli = (ROOT / "native/rust_helper/tests/fixtures/fake_codex_cli.rs").read_text(
        encoding="utf-8"
    )
    agent_center = (ROOT / "apps/desktop_flutter/lib/screens/agent_center.dart").read_text(
        encoding="utf-8"
    )
    for required in (
        '"--model", model_id',
        '"provider_id": "openai_codex_cli"',
        '"provider_health": {"status": "unknown"',
        '"automatic_fallback": false',
        '"method": if self.provider_credential_id.is_some() { "local_credential_reference" } else { "codex_cli_managed" }',
    ):
        if required not in adapter:
            errors.append(f"Codex Adapterが選択Model／状態契約を反映しない: {required}")
    for required in (
        "fn 選択Modelが偽CodexCLIのTask実行へ届く()",
        '"model-fixture-v1"',
        '"fixture-task-completed:model-fixture-v1"',
    ):
        if required not in adapter:
            errors.append(f"Provider／Modelの偽CLI正常経路試験がない: {required}")
    for required in (
        'argument_after(&arguments, "--model")',
        "fixture-task-completed:{model}",
        "--model MODEL",
    ):
        if required not in fake_cli:
            errors.append(f"偽Codex CLIがModel指定を検証しない: {required}")
    for required in (
        "'provider_model_selection': {",
        "'model_id': modelId",
        "'authentication_source': authenticationSource",
        "if (credentialId != null) 'credential_id': credentialId",
        "listProviderCredentials()",
        "'automatic_fallback': false",
        "提供元接続・模型利用可否: 不明（CLI接続面のみ確認）",
        "自動代替実行: 無効",
    ):
        if required not in agent_center:
            errors.append(f"Agent CenterがProvider／Model状態を表示・送信しない: {required}")
    return errors


def test_agent_task_owner_confirmation_wait_uses_native_operation_timeout() -> list[str]:
    broker_client = (ROOT / "apps/desktop_flutter/lib/services/broker_client.dart").read_text(
        encoding="utf-8"
    )
    runner = (ROOT / "apps/desktop_flutter/windows/runner/broker_pipe_controller.cpp").read_text(
        encoding="utf-8"
    )
    errors: list[str] = []
    owner_operations = (
        "GUI Shell書出し",
        "回帰Case登録",
        "回帰Case削除",
        "回帰Case削除中断確認",
        "資格情報失効",
        "A2A接続",
        "MCP接続",
        "MCP切断",
        "MCP Tool実行",
        "更新download要求",
        "更新有効版切替要求",
        "製品起動項目修復要求",
        "製品アンインストール要求",
        "AgentTaskWorkspacePermissionGrant",
        "AgentTaskOwnerApprovalGrant",
        "AgentTask結果表示承認",
        "作業領域承認",
        "作業領域失効",
        "作業領域全体基準点保存",
        "AgentCLI実行系作業領域登録",
        "アダプター導入",
        "アダプター更新",
        "アダプター検証",
        "アダプター有効化",
        "アダプター無効化",
        "アダプター隔離",
        "アダプター削除",
    )
    owner_set_match = re.search(
        r"const _nativeOwnerConfirmationOperations = <String>\{(.*?)\};",
        broker_client,
        re.DOTALL,
    )
    if owner_set_match is None:
        errors.append("Flutter native Owner確認operation集合を特定できない")
    else:
        client_operations = re.findall(r"'([^']+)'", owner_set_match.group(1))
        if set(client_operations) != set(owner_operations):
            errors.append("FlutterとWindows Runnerのnative Owner確認operation集合が一致しない")
    for operation in owner_operations:
        if operation not in broker_client:
            errors.append(f"Agent Task native Owner確認operationをDesktop待機対象に含めない: {operation}")
        encoded_operation = "".join(f"\\x{byte:02X}" for byte in operation.encode("utf-8"))
        if encoded_operation not in runner:
            errors.append(f"Windows Runner pipe timeoutがOwner確認operationを待機対象に含めない: {operation}")
    if "brokerRequestTimeoutForOperation(operation)" not in broker_client:
        errors.append("Desktop Broker要求がoperation別応答待ちpolicyを使わない")
    if "Duration(seconds: 305)" not in broker_client or "Duration(seconds: 5)" not in broker_client:
        errors.append("Desktop Broker要求のnative Owner確認待ちと通常待ちを区別しない")
    if (
        "RequiresOwnerConfirmationPipeTimeout(request)" not in runner
        or 'request.find("\\\"operation\\\":")' not in runner
        or "request.compare(operation_value" not in runner
    ):
        errors.append("Windows Runner pipeがOwner確認要求別の応答待ちpolicyを使わない")
    if "kOwnerConfirmationPipeTimeoutMs = 310000" not in runner or "kDefaultPipeTimeoutMs = 5000" not in runner:
        errors.append("Windows Runner pipeのOwner確認／通常要求timeout境界が不正")
    return errors


def test_agent_task_id_operations_are_content_free_and_declared() -> list[str]:
    schema = load_schema("agent_task_id_request.schema.json")
    valid = load_contract_fixture("agent_task_id_request.valid.json")
    errors = validate_instance(valid, schema)
    escalation = load_contract_fixture(
        "invalid/agent_task_id_request_escalation.invalid.json"
    )
    if validate_instance(escalation, schema) == []:
        errors.append("Agent Task識別子要求が追加のscope fieldを受け入れる")
    broker = (ROOT / "native/rust_helper/src/broker/dialogue.rs").read_text(
        encoding="utf-8"
    )
    for required in (
        "AgentTask実行対応",
        "AgentTask実行",
        "AgentTask進捗反映",
        "AgentTask結果hash化",
        "AGENT_TASK_RECORD_LIMIT",
        "AgentTask状態射影",
    ):
        if required not in broker:
            errors.append(f"Agent Task consumerが必須のbounded経路を欠く: {required}")
    if '"description": "Agent作業Task（結果本文とWorkspace差分は別の権限経路）"' not in broker:
        errors.append("Agent Task状態projectionのdescriptionが固定labelではない")
    return errors


def test_agent_task_workspace_permission_is_owner_scoped_and_one_use() -> list[str]:
    request_schema = load_schema("agent_task_workspace_permission_request.schema.json")
    permission_schema = load_schema("agent_task_workspace_permission.schema.json")
    request = load_contract_fixture("agent_task_workspace_permission_request.valid.json")
    permission = load_contract_fixture("agent_task_workspace_permission.valid.json")
    errors = validate_instance(request, request_schema)
    errors.extend(validate_instance(permission, permission_schema))

    if set(request) != {"agent_runtime_id", "session_id", "workspace_id"}:
        errors.append("Workspace Permission要求が識別子以外を受け付ける")
    for field in ("permission_id", "approval_id", "expires_at", "workspace_path", "sandbox", "command"):
        if validate_instance({**request, field: "untrusted"}, request_schema) == []:
            errors.append(f"Workspace Permission要求が禁止fieldを受け付けた: {field}")

    invalid_request = load_contract_fixture(
        "invalid/agent_task_workspace_permission_request_escalation.invalid.json"
    )
    invalid_permission = load_contract_fixture(
        "invalid/agent_task_workspace_permission_escalation.invalid.json"
    )
    if validate_instance(invalid_request, request_schema) == []:
        errors.append("Workspace Permission要求がrequest側Authorityを受け付ける")
    if validate_instance(invalid_permission, permission_schema) == []:
        errors.append("Workspace Permissionがscope／operation／source escalationを受け付ける")

    if permission.get("use_limit") != 1 or permission.get("uses_remaining") != 1:
        errors.append("Workspace Permission正常例が一回限りではない")
    contract_text = (DOC_SPECS / "agent-runtime.md").read_text(encoding="utf-8")
    for required in ("Task固有のOwner Approvalではない", "5分期限", "Task保存、process起動"):
        if required not in contract_text:
            errors.append(f"Agent Runtime正本にWorkspace Permission境界がない: {required}")
    return errors


def test_agent_task_owner_approval_receipt_is_hash_bound_and_unconsumed() -> list[str]:
    schema = load_schema("agent_task_owner_approval.schema.json")
    receipt = load_contract_fixture("agent_task_owner_approval.valid.json")
    errors = validate_instance(receipt, schema)
    stale_policy = load_contract_fixture(
        "invalid/agent_task_owner_approval_stale_policy.invalid.json"
    )
    if validate_instance(stale_policy, schema) == []:
        errors.append("Owner Approval receiptが期限上限のない旧実行ポリシー識別子を受け入れる")
    execution_claim = load_contract_fixture(
        "invalid/agent_task_owner_approval_execution_claim.invalid.json"
    )
    if validate_instance(execution_claim, schema) == []:
        errors.append("Owner Approval receiptがTask実行済み状態を受け入れる")
    for field in ("approval_id", "permission_id", "instruction", "command", "workspace_path"):
        if validate_instance({**receipt, field: "untrusted"}, schema) == []:
            errors.append(f"Owner Approval receiptが秘密／権限／実行fieldを受け入れる: {field}")
    if receipt.get("use_limit") != 1 or receipt.get("uses_remaining") != 1:
        errors.append("Owner Approval receiptが未消費一回限りを示さない")
    policy_id = "gui-shell-agent-task-sandbox-v1-max-runtime-900s"
    if schema["properties"]["適用ポリシー"].get("const") != policy_id:
        errors.append("Owner Approval Schemaが固定実行ポリシー識別子と一致しない")
    if receipt.get("適用ポリシー") != policy_id:
        errors.append("Owner Approval正常例が固定実行ポリシー識別子と一致しない")
    broker = (ROOT / "native/rust_helper/src/broker/dialogue.rs").read_text(
        encoding="utf-8"
    )
    launcher = (ROOT / "native/rust_helper/src/desktop_launcher.rs").read_text(
        encoding="utf-8"
    )
    protocol = (ROOT / "native/rust_helper/src/broker/protocol.rs").read_text(
        encoding="utf-8"
    )
    for required in (
        "AgentTaskOwnerApprovalGrant",
        "AgentTask実行条件hash",
        "agent_task_owner_approval_is_active",
        "実行条件hash",
    ):
        if required not in broker:
            errors.append(f"Owner Approval Broker経路に必須結合がない: {required}")
    for required in ("AgentTaskOwnerApprovalGrant", "指示hash"):
        if required not in launcher:
            errors.append(f"Owner Approval native確認経路に必須表示／制限がない: {required}")
    if "AgentTaskOwnerApprovalGrant" not in protocol or "DesktopNativeConfirmation" not in protocol:
        errors.append("Owner Approval operationがRust Desktop native confirmationへ限定されない")
    for source, text in (("Broker", broker), ("native確認画面", launcher)):
        if policy_id not in text:
            errors.append(f"Owner Approval {source}がSchemaと同じ固定実行ポリシーを使わない")
    return errors


def test_agent_task_state_record_is_versioned_scoped_and_content_free() -> list[str]:
    schema = load_schema("agent_task.schema.json")
    current = load_contract_fixture("agent_task.valid.json")
    errors = validate_instance(current, schema)
    expected = {
        "task_id",
        "record_version",
        "agent_runtime_id",
        "session_id",
        "workspace_id",
        "description",
        "instruction_hash",
        "status",
        "audit_event_id",
        "result_content_available",
    }
    if set(current) != expected:
        errors.append("AgentTask版2のfield集合が実行範囲へ過不足なく固定されていない")

    legacy = {
        "task_id": "legacy-task",
        "session_id": "legacy-session",
        "description": "旧形式の記録",
        "status": "running",
        "audit_event_id": "legacy-audit",
    }
    if validate_instance(legacy, schema):
        errors.append("版なしAgentTaskの履歴互換形を読み取れない")

    for field in ("agent_runtime_id", "workspace_id", "instruction_hash", "result_content_available"):
        candidate = {**current}
        candidate.pop(field)
        if validate_instance(candidate, schema) == []:
            errors.append(f"AgentTask版2が必須の結合field欠落を受理した: {field}")

    for field in (
        "instruction",
        "agent_output",
        "permission_id",
        "approval_id",
        "workspace_path",
        "command",
        "credential",
    ):
        if validate_instance({**current, field: "untrusted"}, schema) == []:
            errors.append(f"AgentTask状態recordが本文・権限・実行情報を受理した: {field}")

    for status in ("approved", "executed", "success"):
        if validate_instance({**current, "status": status}, schema) == []:
            errors.append(f"AgentTask状態recordが許可外statusを受理した: {status}")
    if validate_instance({**current, "result_hash": "raw-output"}, schema) == []:
        errors.append("AgentTask状態recordが不正なresult hashを受理した")
    if validate_instance({**current, "result_content_available": "yes"}, schema) == []:
        errors.append("AgentTask状態recordが不正な結果本文保持状態を受理した")
    if validate_instance({**current, "description": "x" * 161}, schema) == []:
        errors.append("AgentTask状態recordが表示labelの上限超過を受理した")

    contract_text = (DOC_SPECS / "agent-runtime.md").read_text(encoding="utf-8")
    for required in ("履歴互換", "実行状態の証拠", "Content Exposure Boundary"):
        if required not in contract_text:
            errors.append(f"Agent Runtime正本が版2／履歴投影の境界を定義しない: {required}")
    return errors


def test_agent_task_result_content_exposure_is_separate_and_bounded() -> list[str]:
    schema = load_schema("agent_task_content_exposure.schema.json")
    task_id = "a" * 32
    result_hash = "sha256:" + "b" * 64
    valid_grant = {
        "operation": "AgentTask結果表示承認",
        "payload": {
            "task_id": task_id,
            "result_hash": result_hash,
            "content_visibility": "full",
        },
    }
    valid_read = {
        "operation": "AgentTask結果取得",
        "payload": {"task_id": task_id, "approval_id": "c" * 32},
    }
    errors = []
    for request in (valid_grant, valid_read):
        if validate_instance(request, schema):
            errors.append("Agent Task結果Content Exposureの正規要求を受理できない")
    for visibility in ("none", "hash_only", "summary", "redacted", "full"):
        candidate = {
            "operation": "AgentTask結果表示承認",
            "payload": {
                "task_id": task_id,
                "result_hash": result_hash,
                "content_visibility": visibility,
            },
        }
        if validate_instance(candidate, schema):
            errors.append(f"Agent Task結果Content Exposureの範囲を受理できない: {visibility}")
    result_receipt_schema = {
        "$schema": schema["$schema"],
        "$defs": schema["$defs"],
        "$ref": "#/$defs/result_receipt",
    }
    valid_projections = {
        "none": None,
        "hash_only": {"result_hash": result_hash},
        "summary": {"説明": "この表示範囲に提供できる承認済み内容はありません"},
        "redacted": {"説明": "この表示範囲に提供できる承認済み内容はありません"},
        "full": {"result_hash": result_hash, "text": "合成結果"},
    }
    for visibility, projection in valid_projections.items():
        receipt = {
            "task_id": task_id,
            "result_hash": result_hash,
            "content_visibility": visibility,
            "projection": projection,
        }
        if validate_instance(receipt, result_receipt_schema):
            errors.append(f"Agent Task結果receiptの正規projectionを受理できない: {visibility}")
    mismatched_receipts = [
        {
            "task_id": task_id,
            "result_hash": result_hash,
            "content_visibility": "hash_only",
            "projection": {"result_hash": result_hash, "text": "unexpected"},
        },
        {
            "task_id": task_id,
            "result_hash": result_hash,
            "content_visibility": "full",
            "projection": None,
        },
    ]
    if any(
        validate_instance(receipt, result_receipt_schema) == []
        for receipt in mismatched_receipts
    ):
        errors.append("Agent Task結果receiptが要求visibilityと不一致のprojectionを拒否しない")
    invalid_grants = [
        {"operation": "AgentTask結果表示承認", "payload": {"task_id": task_id, "result_hash": result_hash}},
        {"operation": "AgentTask結果表示承認", "payload": {"task_id": task_id, "result_hash": result_hash, "content_visibility": "full", "approval_id": "d" * 32}},
        {"operation": "AgentTask結果表示承認", "payload": {"task_id": task_id, "result_hash": "raw", "content_visibility": "full"}},
        {"operation": "AgentTask結果取得", "payload": {"task_id": task_id, "approval_id": "d" * 32, "content_visibility": "full"}},
    ]
    if any(validate_instance(candidate, schema) == [] for candidate in invalid_grants):
        errors.append("Agent Task結果Content Exposure要求が欠落・未知・不正fieldを拒否しない")

    root = Path(__file__).resolve().parents[2]
    dialogue = (root / "native/rust_helper/src/broker/dialogue.rs").read_text(encoding="utf-8")
    protocol = (root / "native/rust_helper/src/broker/protocol.rs").read_text(encoding="utf-8")
    launcher = (root / "native/rust_helper/src/desktop_launcher.rs").read_text(encoding="utf-8")
    flutter = (root / "apps/desktop_flutter/lib/services/broker_client.dart").read_text(encoding="utf-8")
    workspace_client = (root / "apps/desktop_flutter/lib/services/workspace_client.dart").read_text(encoding="utf-8")
    workspace_screen = (root / "apps/desktop_flutter/lib/screens/workspace_inspector.dart").read_text(encoding="utf-8")
    for operation in ("AgentTask結果表示承認", "AgentTask結果取得"):
        if operation not in dialogue or operation not in protocol:
            errors.append(f"Agent Task結果Content Exposureが統治済みBroker経路へ未接続: {operation}")
    if "DesktopNativeConfirmation" not in protocol or "AgentTask結果表示承認" not in launcher or "AgentTask結果表示承認" not in flutter:
        errors.append("Agent Task結果表示承認がnative Owner確認経路へ限定されない")
    for operation in ("作業領域承認", "作業領域失効", "作業領域全体基準点保存"):
        if (
            f"BrokerOperation::{operation}" not in protocol
            or operation not in launcher
            or operation not in workspace_client
        ):
            errors.append(f"Workspace内容露出操作がnative Owner／Agent Center経路へ未接続: {operation}")
    if (
        "native Owner確認でWorkspace読取を許可" not in workspace_screen
        or "native Owner確認で比較baselineを保存" not in workspace_screen
        or "baseline以降の変更file／差分を表示" not in workspace_screen
    ):
        errors.append("Agent Center内Workspace Inspectorに内容Approval／差分導線がない")
    if "作業領域基準点保存" not in protocol or "DesktopNativeConfirmation" not in protocol:
        errors.append("Workspace比較基準点の部分保存をnative Owner確認で拘束しない")
    if "Task IDには結合されない" not in launcher:
        errors.append("Workspace単一baselineがTask固有証拠ではないことをnative確認で示さない")
    for required in ("1_048_576", "AGENT_TASK_RESULT_BODY_LIMIT", "Duration::from_secs(900)", "content_visibility"):
        if required not in dialogue:
            errors.append(f"Agent Task結果のbounded保持・可視性境界が欠落: {required}")
    contract = (root / "docs/specs/agent-runtime.md").read_text(encoding="utf-8")
    for required in ("15分", "native Owner確認", "Task実行Permission", "Agentの自己申告", "unknown"):
        if required not in contract:
            errors.append(f"Agent Task結果露出の意味境界が日本語正本にない: {required}")
    return errors


def test_agent_task_scratch_recovery_journal_is_bounded_and_content_free() -> list[str]:
    schema = load_schema("agent_task_scratch_recovery.schema.json")
    valid = load_contract_fixture("agent_task_scratch_recovery.valid.json")
    invalid = load_contract_fixture(
        "invalid/agent_task_scratch_recovery_absolute_path.invalid.json"
    )
    errors = validate_instance(valid, schema)
    if validate_instance(invalid, schema) == []:
        errors.append("Agent Task scratch回復Schemaが絶対pathを拒否しない")
    entry = valid["entries"][0]
    for field in (
        "instruction",
        "agent_output",
        "credential",
        "permission_id",
        "approval_id",
        "workspace_path",
    ):
        if validate_instance({**valid, "entries": [{**entry, field: "untrusted"}]}, schema) == []:
            errors.append(f"Agent Task scratch回復記録が秘密／authority fieldを受理した: {field}")
    active_without_identity = {**entry, "scratch_file_id": None}
    if validate_instance({**valid, "entries": [active_without_identity]}, schema) == []:
        errors.append("active scratch記録が実体identity欠落を受理した")
    if validate_instance({**valid, "entries": valid["entries"] * 129}, schema) == []:
        errors.append("Agent Task scratch回復記録が256件上限を超過した")

    contract = (DOC_SPECS / "agent-runtime.md").read_text(encoding="utf-8")
    journal_source = (RUST_HELPER / "src" / "broker" / "agent_task_scratch.rs").read_text(
        encoding="utf-8"
    )
    broker_source = (RUST_HELPER / "src" / "broker" / "protocol.rs").read_text(
        encoding="utf-8"
    )
    server_source = (RUST_HELPER / "src" / "broker" / "ipc_server.rs").read_text(
        encoding="utf-8"
    )
    for required in ("recovery_binding_hash", "reserved", "active", "HMAC", "unsupported"):
        if required not in contract:
            errors.append(f"Agent Runtime正本がscratch回復境界を規定しない: {required}")
    for required in ("recover_workspace", "remove_open_dir_all", "open_dir_nofollow"):
        if required not in journal_source:
            errors.append(f"scratch回復journalが必要な限定処理を持たない: {required}")
    if "self.recover_agent_task_scratch(config, protected)?" not in broker_source:
        errors.append("Workspace起動登録がBroker scratch reaperへ接続されない")
    for required in (
        "fn broker強制終了後の別process起動登録で永続scratchを監査付き回収する",
        "current_exe()",
        "child.0.kill()",
    ):
        if required not in broker_source:
            errors.append(f"Broker scratch回復にprocess境界の強制終了試験がない: {required}")
    compact_server_source = re.sub(r"\s+", "", server_source)
    startup_registration = compact_server_source.find(
        "broker.作業領域起動登録(workspace,&protected)"
    )
    listener_start = compact_server_source.find("TcpListener::bind(bind_addr)")
    if startup_registration < 0 or listener_start < 0 or startup_registration > listener_start:
        errors.append("BrokerはWorkspace scratch回復をIPC受付開始前に完了しない")
    return errors


def test_agent_secret_path_read_default_deny() -> list[str]:
    workspace = load_contract_fixture("agent_workspace.valid.json")
    contract = AgentRuntimeContract(workspace)
    if contract.path_allowed("/workspace/project/.env"):
        return ["agent runtimeがsecret pathのreadをdefaultで許可した"]
    return []


def test_agent_secret_path_symlink_default_deny() -> list[str]:
    with tempfile.TemporaryDirectory(prefix="gui-shell-agent-runtime-") as directory:
        root = Path(directory)
        workspace_root = root / "workspace"
        workspace_root.mkdir()
        secret = workspace_root / ".env"
        secret.write_text("TOKEN=secret\n", encoding="utf-8")
        public = workspace_root / "public"
        public.mkdir()
        symlink = public / "linked-config"
        try:
            symlink.symlink_to(secret)
        except OSError:
            return []
        contract = AgentRuntimeContract(
            {
                "workspace_id": "workspace-symlink",
                "root_path": str(workspace_root),
                "boundary_policy": "deny_outside_workspace",
                "secret_paths": [".env", "secrets/"],
                "outside_access_default": "deny",
            }
        )
        if contract.path_allowed(str(symlink)):
            return ["agent runtimeがsecret fileへresolveするsymlink pathを許可した"]
    return []


def test_agent_shell_command_requires_permission_mapping() -> list[str]:
    workspace = load_contract_fixture("agent_workspace.valid.json")
    contract = AgentRuntimeContract(workspace)
    allowed = contract.shell_command_requires_permission(load_contract_fixture("agent_tool_call.valid.json"))
    denied = contract.shell_command_requires_permission({"tool_name": "shell.command"})
    if not allowed or denied:
        return ["agent shell commandのpermission mapping checkが失敗した"]
    return []


def test_agent_git_push_requires_explicit_approval() -> list[str]:
    contract = AgentRuntimeContract(load_contract_fixture("agent_workspace.valid.json"))
    if not contract.git_push_requires_explicit_approval({"tool_name": "git.push", "permission_id": "permission.git.push", "approval_required": True}):
        return ["agentのgit pushに対するexplicit approvalが拒否された"]
    if contract.git_push_requires_explicit_approval({"tool_name": "git.push", "permission_id": "permission.git.push", "approval_required": False}):
        return ["agentのgit pushがexplicit approvalを必須にしなかった"]
    return []


def test_agent_generated_diff_must_be_auditable() -> list[str]:
    contract = AgentRuntimeContract(load_contract_fixture("agent_workspace.valid.json"))
    if not contract.diff_is_auditable(load_contract_fixture("agent_diff.valid.json")):
        return ["agentがaudit evidence付きで生成したdiffが拒否された"]
    if contract.diff_is_auditable({"diff_id": "diff-1", "payload_hash": "sha256:" + "5" * 64}):
        return ["agentがauditなしで生成したdiffが受け入れられた"]
    return []


def test_agent_auto_permission_is_advisory_only() -> list[str]:
    contract = AgentRuntimeContract(load_contract_fixture("agent_workspace.valid.json"))
    if not contract.auto_permission_is_advisory_only(load_contract_fixture("agent_runtime.valid.json")):
        return ["agentのadvisory auto-permission modeが拒否された"]
    if contract.auto_permission_is_advisory_only({"auto_permission_mode": "authority"}):
        return ["agentのauto-permission authority modeが受け入れられた"]
    return []


def test_agent_adapter_is_declaration_only_and_unsupported_is_explicit() -> list[str]:
    adapter = load_contract_fixture("agent_adapter.valid.json")
    contract = AgentAdapterContract(adapter)
    schema = load_schema("agent_adapter.schema.json")
    if validate_instance(adapter, schema):
        return ["agent adapterの正常fixtureがSchemaに適合しない"]
    if not contract.is_declaration_only():
        return ["agent adapterの宣言へauthority fieldが混入した"]
    if not contract.unsupported_features_are_explicit():
        return ["agent adapterのunsupported／unknownに理由がない"]

    forged = json.loads(
        (INVALID_CONTRACT_EXAMPLES / "agent_adapter_authority.invalid.json").read_text(
            encoding="utf-8"
        )
    )
    if not validate_instance(forged, schema):
        return ["agent adapterへpermission_idを混入したfixtureが拒否されない"]
    incomplete = copy.deepcopy(adapter)
    incomplete["tool_support"] = {"status": "unsupported", "reason": ""}
    if contract.unsupported_features_are_explicit(incomplete):
        return ["agent adapterの空reasonをunsupportedとして受理した"]
    return []


def test_broker_agent_metadata_is_validated_before_projection() -> list[str]:
    schema = load_schema("agent_adapter.schema.json")
    authentication = schema.get("properties", {}).get("authentication", {})
    secret_flag = authentication.get("properties", {}).get("secret_value_present", {})
    if secret_flag.get("const") is not False:
        return ["Agent Adapter Schemaが秘密値の非保持を固定していない"]

    dialogue = (RUST_HELPER / "src" / "broker" / "dialogue.rs").read_text(
        encoding="utf-8"
    )
    protocol = (RUST_HELPER / "src" / "broker" / "protocol.rs").read_text(
        encoding="utf-8"
    )
    contract = (ROOT / "docs" / "specs" / "agent-coordination.md").read_text(
        encoding="utf-8"
    )
    for token in (
        "struct AgentAdapterMetadata",
        "deny_unknown_fields",
        "metadata_attempts_authority_value",
        "agent_metadata_contains_credential_marker",
        "secret_value_present",
        "agent_metadata_projection",
    ):
        if token not in dialogue:
            return [f"BrokerのAgent metadata検証経路が不足: {token}"]
    if "agent_list_rejects_untrusted_adapter_metadata_with_audit_and_no_leak" not in protocol:
        return ["Agent metadataのAuthority／secret拒否をBroker実監査経路で試験していない"]
    for token in (
        "一覧全体を`応答不正`として拒否",
        "Broker Audit",
        "Permission",
        "既知markerに限り",
    ):
        if token not in contract:
            return [f"Agent metadata意味正本に拒否境界がない: {token}"]
    return []


def test_agent_adapter_probe_is_read_only_and_fail_closed() -> list[str]:
    adapter = build_adapter_record("codex", "0.155.0-alpha.16", True, True)
    schema = load_schema("agent_adapter.schema.json")
    if validate_instance(adapter, schema):
        return ["Agent CLI probeのfixture結果がAgent Adapter Schemaに適合しない"]
    if adapter["status"] != "degraded" or adapter["evidence_source"] != "LIVE_RUNTIME":
        return ["interface確認済みAgentをBroker dispatch停止中のdegradedとして表現しなかった"]
    if adapter["authentication"]["secret_value_present"] is not False:
        return ["Agent CLI probeがsecret実値の存在を許可した"]
    if (
        adapter.get("provider_id") != "openai_codex_cli"
        or adapter.get("provider_health", {}).get("status") != "unknown"
        or adapter.get("automatic_fallback") is not False
        or adapter["authentication"].get("method") != "codex_cli_managed"
    ):
        return ["Agent CLI probeがProvider／Model選択とunknown／fallback状態を正確に投影しない"]
    if validate_instance(adapter, schema):
        return ["Provider／Model状態を含むAgent CLI probeがAgent Adapter Schemaに適合しない"]
    unavailable = build_adapter_record(None, "unknown", False, False)
    if unavailable["status"] != "unavailable" or unavailable["evidence_source"] != "CONFIG":
        return ["未導入Agent CLIを利用可能として扱った"]
    return []


def agent_comparison_projection_errors(record: dict) -> list[str]:
    errors: list[str] = []
    entries = record.get("entries")
    if not isinstance(entries, list) or len(entries) < 2:
        errors.append("agent comparisonは二つ以上のentryを必要とする")
        return errors
    session_ids = [entry.get("session_id") for entry in entries]
    agent_runtime_ids = [entry.get("agent_runtime_id") for entry in entries]
    workspace_ids = [entry.get("workspace_id") for entry in entries]
    if len(set(session_ids)) != len(session_ids):
        errors.append("agent comparisonは同じsessionを重複して比較してはならない")
    if len(set(agent_runtime_ids)) != len(agent_runtime_ids):
        errors.append("agent comparisonは同じAgent runtimeを別Agentとして比較してはならない")
    if len(set(workspace_ids)) != len(workspace_ids):
        errors.append("agent comparisonは同一Workspaceを比較へ混在させてはならない")
    if record.get("workspace_isolation") != "passed":
        errors.append("agent comparisonのWorkspace隔離はpassedでなければならない")
    if record.get("authority_reused") is not False:
        errors.append("agent comparisonはAuthorityを再利用してはならない")
    if record.get("approval_reused") is not False:
        errors.append("agent comparisonはApprovalを再利用してはならない")
    if record.get("credential_included") is not False:
        errors.append("agent comparisonはCredentialを含めてはならない")
    for entry in entries:
        for metric_name in ("duration_ms", "token_count", "cost", "resource"):
            metric = entry.get(metric_name, {})
            if metric.get("status") != "known" and metric.get("value") is not None:
                errors.append(f"agent comparisonの{metric_name} unknown値を0等へ補完してはならない")
    return errors


def test_agent_comparison_projection_is_isolated_and_non_authoritative() -> list[str]:
    comparison = load_contract_fixture("agent_comparison.valid.json")
    schema = load_schema("agent_comparison.schema.json")
    if validate_instance(comparison, schema):
        return ["agent comparisonの正常fixtureがSchemaに適合しない"]
    errors = agent_comparison_projection_errors(comparison)
    if errors:
        return errors

    same_workspace = copy.deepcopy(comparison)
    same_workspace["entries"][1]["workspace_id"] = same_workspace["entries"][0]["workspace_id"]
    if not agent_comparison_projection_errors(same_workspace):
        return ["agent comparisonが同一Workspaceのentryを受け入れた"]

    same_agent_runtime = copy.deepcopy(comparison)
    same_agent_runtime["entries"][1]["agent_runtime_id"] = (
        same_agent_runtime["entries"][0]["agent_runtime_id"]
    )
    if not agent_comparison_projection_errors(same_agent_runtime):
        return ["agent comparisonが同じAgent runtimeを複数Agentとして受け入れた"]

    unknown_metric = copy.deepcopy(comparison)
    unknown_metric["entries"][0]["token_count"] = {"status": "unknown", "value": 0}
    if not agent_comparison_projection_errors(unknown_metric):
        return ["agent comparisonが取得不能なmetricを0として受け入れた"]

    authority = copy.deepcopy(comparison)
    authority["authority_reused"] = True
    if not agent_comparison_projection_errors(authority):
        return ["agent comparisonがAuthority再利用を受け入れた"]
    if not validate_instance(
        load_contract_fixture("invalid/agent_comparison_authority.invalid.json"), schema
    ):
        return ["agent comparisonのAuthority混入negativeが拒否されない"]
    return []


def agent_handoff_projection_errors(record: dict) -> list[str]:
    errors: list[str] = []
    if record.get("source_agent_runtime_id") == record.get("target_agent_runtime_id"):
        errors.append("agent handoffは同一AgentへのAuthority付き再利用を表してはならない")
    if record.get("authority_reassessment_required") is not True:
        errors.append("agent handoffはtarget条件でのAuthority再評価を必須にしなければならない")
    for key in (
        "permission_reused",
        "approval_reused",
        "credential_included",
        "hidden_context_included",
    ):
        if record.get(key) is not False:
            errors.append(f"agent handoffは{key}をfalseで固定しなければならない")
    summary = record.get("public_execution_summary", "")
    if any(marker in summary.lower() for marker in ("secret=", "password=", "credential-value")):
        errors.append("agent handoffの公開実行概要へ秘密値を含めてはならない")
    return errors


def test_agent_handoff_projection_requires_reassessment_and_redaction() -> list[str]:
    handoff = load_contract_fixture("agent_handoff.valid.json")
    schema = load_schema("agent_handoff.schema.json")
    if validate_instance(handoff, schema):
        return ["agent handoffの正常fixtureがSchemaに適合しない"]
    errors = agent_handoff_projection_errors(handoff)
    if errors:
        return errors

    same_agent = copy.deepcopy(handoff)
    same_agent["target_agent_runtime_id"] = same_agent["source_agent_runtime_id"]
    if not agent_handoff_projection_errors(same_agent):
        return ["agent handoffが同一AgentをAuthority付きで受け入れた"]

    secret_summary = copy.deepcopy(handoff)
    secret_summary["public_execution_summary"] = "secret=credential-value"
    if not agent_handoff_projection_errors(secret_summary):
        return ["agent handoffが公開概要の秘密値を受け入れた"]
    if not validate_instance(
        load_contract_fixture("invalid/agent_handoff_authority.invalid.json"), schema
    ):
        return ["agent handoffのAuthority・秘密値混入negativeが拒否されない"]

    package = load_contract_fixture("agent_handoff_package.valid.json")
    package_schema = load_schema("agent_handoff_package.schema.json")
    if validate_instance(package, package_schema):
        return ["agent handoff packageの正常fixtureがSchemaに適合しない"]
    if package.get("source_session_id") == package.get("target_session_id") or \
            package.get("source_agent_runtime_id") == package.get("target_agent_runtime_id") or \
            package.get("source_workspace_id") == package.get("target_workspace_id"):
        return ["agent handoff packageの送信元・受信先が独立していない"]
    if not validate_instance(
        load_contract_fixture("invalid/agent_handoff_package_authority.invalid.json"),
        package_schema,
    ):
        return ["agent handoff packageのAuthority混入negativeが拒否されない"]
    same_session = copy.deepcopy(package)
    same_session["target_session_id"] = same_session["source_session_id"]
    if not agent_handoff_package_errors(same_session):
        return ["agent handoff packageが同じSessionを受入れた"]
    unsafe_path = copy.deepcopy(package)
    unsafe_path["changed_files"] = ["../secrets/token.txt"]
    if not validate_instance(unsafe_path, package_schema):
        return ["agent handoff packageがWorkspace外相対pathを受け入れた"]
    return []


def agent_handoff_package_errors(package: dict) -> list[str]:
    errors = []
    for source_key, target_key in (
        ("source_session_id", "target_session_id"),
        ("source_agent_runtime_id", "target_agent_runtime_id"),
        ("source_workspace_id", "target_workspace_id"),
    ):
        if package.get(source_key) == package.get(target_key):
            errors.append(f"handoff packageの{source_key}/{target_key}が同一")
    for key in (
        "authority_reassessment_required",
        "permission_reused",
        "approval_reused",
        "credential_included",
        "hidden_context_included",
    ):
        expected = key == "authority_reassessment_required"
        if package.get(key) is not expected:
            errors.append(f"handoff packageの{key}が不正")
    return errors


def test_gui_shell_compose_is_manifest_only_and_non_inheriting() -> list[str]:
    compose = load_contract_fixture("gui_shell_compose.valid.json")
    receipt = load_contract_fixture("gui_shell_compose_receipt.valid.json")
    compose_schema = load_schema("gui_shell_compose.schema.json")
    receipt_schema = load_schema("gui_shell_compose_receipt.schema.json")
    errors = []
    errors.extend(validate_instance(compose, compose_schema))
    errors.extend(validate_instance(receipt, receipt_schema))
    if compose.get("output_mode") != "manifest_only":
        errors.append("GUI Shell構成がManifest-onlyではない")
    policy = compose.get("inheritance_policy", {})
    if any(
        policy.get(key) != "none"
        for key in ("authority", "permission", "approval", "credential", "audit_chain")
    ):
        errors.append("GUI Shell構成が権限・資格・監査chainを継承可能にしている")
    if (
        receipt.get("permission_generated") is not False
        or receipt.get("app_identity_status") != "not_generated"
    ):
        errors.append("GUI Shell構成receiptがPermission生成またはApp identity生成を主張している")
    invalid_compose = json.loads(
        (INVALID_CONTRACT_EXAMPLES / "gui_shell_compose_authority.invalid.json").read_text(
            encoding="utf-8"
        )
    )
    invalid_receipt = json.loads(
        (
            INVALID_CONTRACT_EXAMPLES
            / "gui_shell_compose_receipt_authority.invalid.json"
        ).read_text(encoding="utf-8")
    )
    if not validate_instance(invalid_compose, compose_schema):
        errors.append("GUI Shell構成が権限継承fieldを受け入れた")
    if not validate_instance(invalid_receipt, receipt_schema):
        errors.append("GUI Shell構成receiptがPermission生成を受け入れた")
    for name in ("ipc_request", "ipc_response"):
        operations = load_schema(f"{name}.schema.json")["properties"]["operation"]["enum"]
        if "GUI Shell構成" not in operations:
            errors.append(f"{name}にGUI Shell構成操作がない")
    source = (RUST_HELPER / "src" / "broker" / "compose_center.rs").read_text(
        encoding="utf-8"
    )
    for token in (
        "manifest_only",
        "not_started",
        "not_generated",
        "permission_generated",
        "authority_strip",
    ):
        if token not in source:
            errors.append(f"GUI Shell構成Broker経路に境界tokenがない: {token}")
    return errors


def test_gui_shell_preview_is_read_only_and_non_rollback() -> list[str]:
    request = load_contract_fixture("gui_shell_preview.valid.json")
    receipt = load_contract_fixture("gui_shell_preview_receipt.valid.json")
    request_schema = load_schema("gui_shell_preview.schema.json")
    receipt_schema = load_schema("gui_shell_preview_receipt.schema.json")
    errors = []
    errors.extend(validate_instance(request, request_schema))
    errors.extend(validate_instance(receipt, receipt_schema))
    if request.get("preview_mode") != "version_rollback":
        errors.append("GUI Shell Previewの版・rollbackモードが固定されていない")
    if receipt.get("build_status") != "not_started" or receipt.get("export_status") != "not_started":
        errors.append("GUI Shell PreviewがbuildまたはExportを完了扱いにした")
    if receipt.get("version_preview", {}).get("rollback_available") is not False:
        errors.append("GUI Shell Previewがrollback実行可能性を生成した")
    if any(
        item.get("permission_status") != "not_generated"
        or item.get("approval_status") != "not_requested"
        for item in receipt.get("permission_requirements", [])
    ):
        errors.append("GUI Shell PreviewがCapability requirementからPermissionまたはApprovalを生成した")
    invalid = json.loads(
        (INVALID_CONTRACT_EXAMPLES / "gui_shell_preview_authority.invalid.json").read_text(
            encoding="utf-8"
        )
    )
    if not validate_instance(invalid, request_schema):
        errors.append("GUI Shell PreviewがPermission継承を受け入れた")
    invalid_receipt = json.loads(
        (
            INVALID_CONTRACT_EXAMPLES
            / "gui_shell_preview_receipt_authority.invalid.json"
        ).read_text(encoding="utf-8")
    )
    if not validate_instance(invalid_receipt, receipt_schema):
        errors.append("GUI Shell Preview ReceiptがAuthority strip無効化を受け入れた")
    for name in ("ipc_request", "ipc_response"):
        operations = load_schema(f"{name}.schema.json")["properties"]["operation"]["enum"]
        if "GUI Shell構成Preview" not in operations:
            errors.append(f"{name}にGUI Shell構成Preview操作がない")
    source = (RUST_HELPER / "src" / "broker" / "compose_center.rs").read_text(
        encoding="utf-8"
    )
    for token in ("pub(super) fn preview", "not_executable", "permission_status"):
        if token not in source:
            errors.append(f"GUI Shell Preview Broker経路に境界tokenがない: {token}")
    return errors


def test_gui_shell_edit_proposal_is_owner_review_only() -> list[str]:
    proposal = load_contract_fixture("gui_shell_edit_proposal.valid.json")
    receipt = load_contract_fixture("gui_shell_edit_proposal_receipt.valid.json")
    proposal_schema = load_schema("gui_shell_edit_proposal.schema.json")
    receipt_schema = load_schema("gui_shell_edit_proposal_receipt.schema.json")
    errors = []
    errors.extend(validate_instance(proposal, proposal_schema))
    errors.extend(validate_instance(receipt, receipt_schema))
    if proposal.get("self_approval") is not False:
        errors.append("GUI Shell編集提案が自己承認を許可した")
    if proposal.get("execution_mode") != "proposal_only":
        errors.append("GUI Shell編集提案が自動適用を許可した")
    if (
        receipt.get("review_required") is not True
        or receipt.get("files_written") is not False
        or receipt.get("permission_generated") is not False
        or receipt.get("approval_state") != "owner_review_required"
    ):
        errors.append("GUI Shell編集提案Receiptが審査待ち境界を満たさない")
    invalid = json.loads(
        (
            INVALID_CONTRACT_EXAMPLES
            / "gui_shell_edit_proposal_self_approval.invalid.json"
        ).read_text(encoding="utf-8")
    )
    if not validate_instance(invalid, proposal_schema):
        errors.append("GUI Shell編集提案がself_approval=trueを受け入れた")
    invalid_receipt = json.loads(
        (
            INVALID_CONTRACT_EXAMPLES
            / "gui_shell_edit_proposal_receipt_apply.invalid.json"
        ).read_text(encoding="utf-8")
    )
    if not validate_instance(invalid_receipt, receipt_schema):
        errors.append("GUI Shell編集提案Receiptがapply完了を受け入れた")
    for name in ("ipc_request", "ipc_response"):
        operations = load_schema(f"{name}.schema.json")["properties"]["operation"]["enum"]
        if "GUI Shell編集提案" not in operations:
            errors.append(f"{name}にGUI Shell編集提案操作がない")
    source = (RUST_HELPER / "src" / "broker" / "ai_edit_center.rs").read_text(
        encoding="utf-8"
    )
    for token in ("owner_required", "proposal_only", "files_written", "permission_generated"):
        if token not in source:
            errors.append(f"GUI Shell編集提案Broker経路に境界tokenがない: {token}")
    return errors


def test_gui_shell_export_is_new_identity_and_non_inheriting() -> list[str]:
    request = load_contract_fixture("gui_shell_export.valid.json")
    receipt = load_contract_fixture("gui_shell_export_receipt.valid.json")
    manifest_file = load_contract_fixture("gui_shell_export_manifest.valid.json")
    request_schema = load_schema("gui_shell_export.schema.json")
    receipt_schema = load_schema("gui_shell_export_receipt.schema.json")
    manifest_file_schema = load_schema("gui_shell_export_manifest.schema.json")
    errors = []
    errors.extend(validate_instance(request, request_schema))
    errors.extend(validate_instance(receipt, receipt_schema))
    errors.extend(validate_instance(manifest_file, manifest_file_schema))
    if request.get("target_platform") != "windows" or request.get("export_mode") != "manifest_file":
        errors.append("GUI Shell書出しがWindows Manifest file生成契約でない")
    if (
        receipt.get("build_status") != "not_started"
        or receipt.get("artifact_status") != "not_built"
        or receipt.get("manifest_file_status") != "written"
        or receipt.get("authority_strip") is not True
    ):
        errors.append("GUI Shell書出しがManifest file生成と実行可能artifactの状態を分離していない")
    if (
        manifest_file.get("export_id") != receipt.get("export_id")
        or manifest_file.get("manifest") != receipt.get("export_manifest")
    ):
        errors.append("実生成Manifest fileとBroker Receiptの内容が一致しない")
    file_record = receipt.get("manifest_file", {})
    app_id = receipt.get("export_manifest", {}).get("app_identity", {}).get("app_id")
    if (
        file_record.get("file_name") != f"{app_id}.json"
        or not re.fullmatch(r"sha256:[0-9a-f]{64}", file_record.get("sha256", ""))
        or not isinstance(file_record.get("byte_length"), int)
        or file_record.get("byte_length", 0) <= 0
        or file_record.get("temporary_file_status") != "removed"
        or file_record.get("temporary_file_name")
        != f".{app_id.removeprefix('d4-pocket-app-')}.manifest.tmp"
        or file_record.get("recovery_action") != "none"
    ):
        errors.append("Manifest file Receiptの名前、hash、size、一時file後処理が実体検証可能でない")
    pending_cleanup = copy.deepcopy(receipt)
    pending_cleanup["manifest_file"].update(
        {
            "temporary_file_status": "cleanup_pending",
            "recovery_action": "owner_review_and_remove_after_hash_verification",
        }
    )
    if validate_instance(pending_cleanup, receipt_schema):
        errors.append("一時file後処理待ちReceiptが明示RecoveryAction付きで受理されない")
    pending_cleanup["manifest_file"]["recovery_action"] = "none"
    if not validate_instance(pending_cleanup, receipt_schema):
        errors.append("一時file後処理待ちReceiptがRecoveryActionなしで受理された")
    if any(
        receipt.get(key) is not False
        for key in (
            "credential_inherited",
            "permission_inherited",
            "approval_inherited",
            "audit_chain_inherited",
        )
    ):
        errors.append("GUI Shell書出しが資格・権限・承認・監査chainを継承した")
    export_manifest = receipt.get("export_manifest", {})
    if export_manifest.get("audit_store", {}).get("inherited") is not False:
        errors.append("GUI Shell書出しの監査storeが新規化されていない")
    if not export_manifest.get("app_identity", {}).get("app_id"):
        errors.append("GUI Shell書出しの新規App identityが空である")
    for target_name, target, schema in (
        ("Receipt", receipt, receipt_schema),
        ("Manifest file", manifest_file, manifest_file_schema),
    ):
        invalid_app_identity = copy.deepcopy(target)
        app_identity = (
            invalid_app_identity.get("export_manifest", {}).get("app_identity")
            if target_name == "Receipt"
            else invalid_app_identity.get("manifest", {}).get("app_identity")
        )
        if isinstance(app_identity, dict):
            app_identity["app_id"] = "d4-pocket-app-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
        if not validate_instance(invalid_app_identity, schema):
            errors.append(f"GUI Shell書出し{target_name}Schemaが大文字App IDを拒否しない")

        invalid_audit_store = copy.deepcopy(target)
        audit_store = (
            invalid_audit_store.get("export_manifest", {}).get("audit_store")
            if target_name == "Receipt"
            else invalid_audit_store.get("manifest", {}).get("audit_store")
        )
        if isinstance(audit_store, dict):
            audit_store["store_id"] = "audit-store-example"
        if not validate_instance(invalid_audit_store, schema):
            errors.append(f"GUI Shell書出し{target_name}Schemaが不定形式Audit store IDを拒否しない")
    module_catalog = load_contract_fixture("gui_shell_module_catalog.valid.json")
    module_schema = load_schema("gui_shell_module_catalog.schema.json")
    errors.extend(validate_instance(module_catalog, module_schema))
    authoritative_catalog = json.loads(
        (SPECS / "gui_shell_module_catalog.json").read_text(encoding="utf-8")
    )
    if module_catalog != authoritative_catalog:
        errors.append("Module一覧exampleが機械可読正本と一致しない")
    optional_ids = [item["module_id"] for item in module_catalog.get("optional_modules", [])]
    required_ids = module_catalog.get("required_module_ids", [])
    core_ids = module_catalog.get("unprunable_core_ids", [])
    expected_core_ids = [
        "core.security_broker",
        "core.cryptography",
        "core.audit_finality",
        "core.credentials",
        "core.os_controls",
        "core.permission_enforcement",
        "core.approval_enforcement",
        "core.content_exposure",
    ]
    expected_required_ids = [
        "shell.dashboard",
        "shell.runtime_operation",
        "shell.agent_operation",
        "shell.authority",
        "shell.approval",
        "shell.audit",
        "shell.recovery",
        "shell.problems",
        "shell.evidence",
        "shell.settings",
    ]
    if core_ids != expected_core_ids:
        errors.append("Module一覧が固定された除去禁止Core安全境界と一致しない")
    if required_ids != expected_required_ids:
        errors.append("Module一覧が固定された必須画面一覧と一致しない")
    if len(set(optional_ids)) != len(optional_ids):
        errors.append("Module一覧で任意Module IDが重複している")
    if set(optional_ids) & (set(required_ids) | set(core_ids)):
        errors.append("Module一覧で任意Moduleが必須境界と重複している")
    request_module_ids = request_schema["properties"]["module_selection"]["properties"]["optional_module_ids"]["items"]["enum"]
    catalog_module_ids = module_schema["properties"]["optional_modules"]["items"]["properties"]["module_id"]["enum"]
    if request_module_ids != optional_ids or catalog_module_ids != optional_ids:
        errors.append("書出し要求／Module一覧Schemaと任意Module一覧が同期していない")
    receipt_plan_schema = receipt_schema["properties"]["export_manifest"]["properties"]["module_plan"]["properties"]
    if receipt_plan_schema["unprunable_core_ids"]["items"]["enum"] != core_ids:
        errors.append("Receipt Schemaの除去禁止Core一覧が正本と同期していない")
    if receipt_plan_schema["mandatory_module_ids"]["items"]["enum"] != required_ids:
        errors.append("Receipt Schemaの必須画面Module一覧が正本と同期していない")
    if not set(core_ids).isdisjoint(required_ids):
        errors.append("除去禁止Core識別子と必須画面識別子が分離されていない")
    module_plan = export_manifest.get("module_plan", {})
    unprunable_core = core_ids
    mandatory = required_ids
    optional = optional_ids
    included = module_plan.get("included_module_ids", [])
    if module_plan.get("mandatory_module_ids") != mandatory:
        errors.append("Module計画が必須Module一覧を保持しない")
    if module_plan.get("unprunable_core_ids") != unprunable_core:
        errors.append("Module計画が除去禁止のCore安全境界を保持しない")
    if not set(unprunable_core + mandatory).issubset(included):
        errors.append("Module計画から必須画面または除去禁止安全境界が欠落している")
    if module_plan.get("binary_pruning_status") != "not_applied":
        errors.append("Manifest-only Module計画がbinary除去済みと誤表示している")
    requested = set(module_plan.get("requested_optional_module_ids", []))
    expected_included = set(unprunable_core) | set(mandatory) | requested
    dependency_map = {
        item["module_id"]: item.get("depends_on", [])
        for item in module_catalog.get("optional_modules", [])
    }
    previous_count = -1
    while len(expected_included) != previous_count:
        previous_count = len(expected_included)
        for module_id in list(expected_included):
            expected_included.update(dependency_map.get(module_id, []))
    if set(included) != expected_included:
        errors.append("Module計画が選択Moduleの依存閉包と一致しない")
    if set(module_plan.get("excluded_optional_module_ids", [])) != set(optional) - expected_included:
        errors.append("Module計画の除外一覧が選択結果と一致しない")
    for module in module_catalog.get("optional_modules", []):
        for surface in module.get("source_surfaces", []):
            if ".." in surface or surface.startswith(("/", "\\")):
                errors.append(f"Module一覧がroot外pathを参照: {surface}")
            elif not (ROOT / surface).is_file():
                errors.append(f"Module一覧が存在しない画面pathを参照: {surface}")
    for module in module_catalog.get("optional_modules", []):
        for dependency in module.get("depends_on", []):
            if dependency not in set(optional) | set(mandatory):
                errors.append(f"Module依存先が必須／任意一覧にない: {dependency}")

    graph = {
        module["module_id"]: module.get("depends_on", [])
        for module in module_catalog.get("optional_modules", [])
    }
    def has_cycle(module_id: str, active: set[str], complete: set[str]) -> bool:
        if module_id in active:
            return True
        if module_id in complete:
            return False
        active.add(module_id)
        if any(has_cycle(dep, active, complete) for dep in graph.get(module_id, [])):
            return True
        active.remove(module_id)
        complete.add(module_id)
        return False
    completed: set[str] = set()
    if any(has_cycle(module_id, set(), completed) for module_id in graph):
        errors.append("Module一覧の依存関係に循環がある")
    invalid_module_receipt = json.loads(
        (INVALID_CONTRACT_EXAMPLES / "gui_shell_export_receipt_pruning_claim.invalid.json")
        .read_text(encoding="utf-8")
    )
    if not validate_instance(invalid_module_receipt, receipt_schema):
        errors.append("実build前のReceiptがbinary Module除去済みを主張できる")
    invalid_pruning_claim = copy.deepcopy(receipt)
    invalid_pruning_claim["export_manifest"]["module_plan"]["binary_pruning_status"] = "applied"
    if not validate_instance(invalid_pruning_claim, receipt_schema):
        errors.append("Manifest file Receiptが実binary Module除去済みを主張できる")
    invalid_request = json.loads(
        (INVALID_CONTRACT_EXAMPLES / "gui_shell_export_credential.invalid.json").read_text(
            encoding="utf-8"
        )
    )
    if not validate_instance(invalid_request, request_schema):
        errors.append("GUI Shell書出しがCredential継承要求を受け入れた")
    invalid_receipt = json.loads(
        (
            INVALID_CONTRACT_EXAMPLES
            / "gui_shell_export_receipt_inherited.invalid.json"
        ).read_text(encoding="utf-8")
    )
    if not validate_instance(invalid_receipt, receipt_schema):
        errors.append("GUI Shell書出しReceiptが継承済み状態を受け入れた")
    invalid_inheritance_claim = copy.deepcopy(receipt)
    invalid_inheritance_claim["credential_inherited"] = True
    if not validate_instance(invalid_inheritance_claim, receipt_schema):
        errors.append("GUI Shell書出しReceiptがcredential継承を受け入れた")
    invalid_manifest_file = copy.deepcopy(manifest_file)
    invalid_manifest_file["manifest"]["inheritance_policy"]["credential"] = "credential.value"
    if not validate_instance(invalid_manifest_file, manifest_file_schema):
        errors.append("Manifest fileがcredential継承を受け入れた")
    for name in ("ipc_request", "ipc_response"):
        operations = load_schema(f"{name}.schema.json")["properties"]["operation"]["enum"]
        if "GUI Shell書出し" not in operations:
            errors.append(f"{name}にGUI Shell書出し操作がない")
    source = (RUST_HELPER / "src" / "broker" / "export_center.rs").read_text(
        encoding="utf-8"
    )
    for token in (
        "d4-pocket-app-",
        "audit-store-",
        "credential_inherited",
        "permission_inherited",
        "artifact_status",
        "target_platform",
        "resolve_module_plan",
        "binary_pruning_status",
        "write_manifest_file",
        "MAX_MANIFEST_FILE_BYTES",
        "hard_link",
        "desktop_export_root",
        "temporary_file_status",
        "owner_review_and_remove_after_hash_verification",
        "with_temporary_cleanup_hint",
    ):
        if token not in source:
            errors.append(f"GUI Shell書出しBroker経路に境界tokenがない: {token}")
    protocol_source = (RUST_HELPER / "src" / "broker" / "protocol.rs").read_text(
        encoding="utf-8"
    )
    compact_protocol = re.sub(r"\s+", "", protocol_source)
    native_export_gate = (
        "ifowner&&envelope.operation==Some(BrokerOperation::GuiShell書出し)"
        "&&export_confirmation!=OwnerConfirmationSource::DesktopNativeConfirmation"
    )
    export_dispatch = "BrokerOperation::GuiShell書出し=>super::export_center::export"
    native_export_gate_index = compact_protocol.find(native_export_gate)
    export_dispatch_index = compact_protocol.find(export_dispatch)
    if (
        native_export_gate_index < 0
        or export_dispatch_index < 0
        or native_export_gate_index > export_dispatch_index
        or "desktop_native_owner_confirmation_required" not in protocol_source
    ):
        errors.append("GUI Shell書出しが通常Owner資格を拒否しnative Owner確認だけをBroker dispatchへ通さない")
    if "Owner資格だけのExport要求はManifestを作らずnative確認必須として拒否する" not in source:
        errors.append("通常Owner資格だけのExport拒否とManifest非生成のRust回帰試験がない")
    launcher_source = (RUST_HELPER / "src" / "desktop_launcher.rs").read_text(
        encoding="utf-8"
    )
    if "Dir::open_ambient_dir" not in launcher_source or "Some((export_dir, export_root))" not in launcher_source:
        errors.append("GUI Shell書出しが検査済み固定保存先のDirectory handleをBrokerへ渡さない")
    if (
        "nativeOwner確認dialogのExportNoYesをBrokerReceiptManifestまで接続する"
        not in launcher_source
        or '"kind": "gui_shell_export"' not in launcher_source
        or "GUI_SHELL_EXPORT_E2E_CAPTURE_DIR" not in launcher_source
    ):
        errors.append("GUI Shell書出しのWin32 Owner dialogからBroker Receipt／Manifestへ接続する実試験がない")
    return errors


def test_gui_shell_module_build_is_untrusted_ui_only_selection() -> list[str]:
    evidence = load_contract_fixture("gui_shell_module_build_evidence.valid.json")
    schema = load_schema("gui_shell_module_build_evidence.schema.json")
    errors = validate_instance(evidence, schema)
    if _flutter_candidates("win32") != ("flutter.bat", "flutter.cmd", "flutter.exe"):
        errors.append("WindowsのFlutter実行fileをPATH上の実体から解決しない")
    if _flutter_candidates("linux") != ("flutter",):
        errors.append("非WindowsのFlutter executable名が未対応である")
    artifact_paths = [item.get("path") for item in evidence.get("artifact_files", [])]
    if len(artifact_paths) != len(set(artifact_paths)):
        errors.append("Module build証拠のartifact pathが重複している")
    if (
        evidence.get("product_artifact_claimed") is not False
        or evidence.get("standalone_app_claimed") is not False
        or evidence.get("authority_verified") is not False
        or evidence.get("selection_input_trust")
        != "unverified_receipt_json_selection_only"
        or evidence.get("binary_pruning_verified") is not False
    ):
        errors.append("開発build証拠が製品・Owner権限・binary除去を主張している")

    receipt = load_contract_fixture("gui_shell_export_receipt.valid.json")
    catalog = None
    try:
        catalog = _validate_catalog(
            json.loads((SPECS / "gui_shell_module_catalog.json").read_text(encoding="utf-8"))
        )
        plan = resolve_module_plan(receipt, catalog)
        defines = dart_defines(plan, catalog)
        expected = {
            "GUI_SHELL_MODULE_SETUP_DOCTOR": False,
            "GUI_SHELL_MODULE_HISTORY": False,
            "GUI_SHELL_MODULE_EVALUATION_LAB": False,
            "GUI_SHELL_MODULE_HOST_CAPABILITIES": False,
            "GUI_SHELL_MODULE_NOTIFICATIONS": False,
            "GUI_SHELL_MODULE_OBSERVABILITY": True,
            "GUI_SHELL_MODULE_TRACE_INSPECTOR": True,
            "GUI_SHELL_MODULE_HOST_OPERATIONS": False,
        }
        if defines != expected:
            errors.append("Manifest選択・依存閉包とFlutter compile-time defineが一致しない")
        if list(plan.included_module_ids) != evidence.get("included_module_ids"):
            errors.append("Module build証拠の包含ModuleがReceipt選択と一致しない")
    except (OSError, ValueError, KeyError, TypeError) as exc:
        errors.append(f"Module buildのManifest選択検査が失敗した: {exc}")

    authority_claim = json.loads(
        (INVALID_CONTRACT_EXAMPLES / "gui_shell_module_build_evidence_authority_claim.invalid.json")
        .read_text(encoding="utf-8")
    )
    product_claim = json.loads(
        (INVALID_CONTRACT_EXAMPLES / "gui_shell_module_build_evidence_product_claim.invalid.json")
        .read_text(encoding="utf-8")
    )
    if not validate_instance(authority_claim, schema):
        errors.append("開発build証拠がOwner Authority検証を主張できる")
    if not validate_instance(product_claim, schema):
        errors.append("開発build証拠が完成製品artifactを主張できる")

    if catalog is not None:
        cyclic_catalog = copy.deepcopy(catalog)
        by_id = {item["module_id"]: item for item in cyclic_catalog["optional_modules"]}
        by_id["shell.history"]["depends_on"] = ["shell.evaluation_lab"]
        by_id["shell.evaluation_lab"]["depends_on"] = ["shell.history"]
        try:
            _validate_catalog(cyclic_catalog)
            errors.append("循環Module依存をbuild計画が受け入れた")
        except ValueError:
            pass

    desktop_source = (DESKTOP_FLUTTER / "lib" / "main.dart").read_text(encoding="utf-8")
    command_start = desktop_source.index("List<_CommandEntry> _featureCommandEntries")
    command_block = desktop_source[command_start : desktop_source.index("class _OpenCommandPaletteIntent")]
    if not re.search(
        r"if\s*\(kGuiShellModuleHostOperations\)\s*const _CommandEntry\(\s*title: 'Host切替'",
        command_block,
    ):
        errors.append("除外可能なHost操作Moduleへのコマンドパレット導線が未連動")

    if set(EXPECTED_CORE_IDS) != set(catalog["unprunable_core_ids"]):
        errors.append("開発build計画が除去禁止Core境界を保持しない")
    if set(EXPECTED_REQUIRED_IDS) != set(catalog["required_module_ids"]):
        errors.append("開発build計画が必須画面Moduleを保持しない")
    if set(EXPECTED_OPTIONAL_IDS) != {item["module_id"] for item in catalog["optional_modules"]}:
        errors.append("開発build計画の任意Module一覧が固定surfaceと一致しない")
    return errors


def test_gui_shell_module_comparison_is_same_commit_and_non_authoritative() -> list[str]:
    evidence = load_contract_fixture("gui_shell_module_comparison_evidence.valid.json")
    schema = load_schema("gui_shell_module_comparison_evidence.schema.json")
    receipt = load_contract_fixture("gui_shell_export_receipt.valid.json")
    catalog = _validate_catalog(
        json.loads((SPECS / "gui_shell_module_catalog.json").read_text(encoding="utf-8"))
    )
    errors = validate_comparison_evidence(
        evidence, schema, receipt=receipt, catalog=catalog
    )
    if evidence.get("baseline_selection") != "all_optional_default_enabled":
        errors.append("all-enabled baselineの選択条件が固定されていない")
    if evidence.get("binary_pruning_verified") is not False:
        errors.append("size deltaだけでbinary／semantic pruningを完了扱いしている")
    if evidence.get("cold_start_status") != "not_measured" or evidence.get(
        "resource_comparison_status"
    ) != "not_measured":
        errors.append("build比較が未測定のstartup／resource値を主張している")
    if evidence.get("baseline", {}).get("effective_dart_defines") != all_optional_defines():
        errors.append("baselineの有効defineが全任意Module有効状態ではない")
    if evidence.get("comparison") != comparison_summary(
        evidence["baseline"]["artifact_files"], evidence["selected"]["artifact_files"]
    ):
        errors.append("比較summaryがartifact recordから再計算できない")
    if len(evidence["baseline"]["aot_surface_libraries"]) != len(EXPECTED_OPTIONAL_IDS):
        errors.append("all-enabled baselineのFlutter AOT surface libraryが揃っていない")
    if len(evidence["selected"]["aot_surface_libraries"]) != 2:
        errors.append("選択buildのFlutter AOT surface libraryがReceiptの選択数と一致しない")
    try:
        _safe_console_text("░", "cp932").encode("cp932")
    except (LookupError, UnicodeEncodeError):
        errors.append("比較buildのWindows consoleで未対応文字を安全に出力できない")

    bad_claim = copy.deepcopy(evidence)
    bad_claim["binary_pruning_verified"] = True
    if not validate_comparison_evidence(bad_claim, schema, receipt=receipt, catalog=catalog):
        errors.append("byte差分だけでbinary pruning完了を主張できる")

    bad_define = copy.deepcopy(evidence)
    bad_define["selected"]["build_command"] = [
        item
        for item in bad_define["selected"]["build_command"]
        if item != "--dart-define=GUI_SHELL_MODULE_TRACE_INSPECTOR=true"
    ]
    if not validate_comparison_evidence(bad_define, schema, receipt=receipt, catalog=catalog):
        errors.append("記録済みModule defineと実build commandの不一致を受け入れた")

    bad_summary = copy.deepcopy(evidence)
    bad_summary["comparison"]["total_bytes_reduced"] += 1
    if not validate_comparison_evidence(bad_summary, schema, receipt=receipt, catalog=catalog):
        errors.append("artifact byte数と矛盾する比較結果を受け入れた")

    bad_surface_retained = copy.deepcopy(evidence)
    bad_surface_retained["selected"]["aot_surface_libraries"].append(
        "apps/desktop_flutter/lib/screens/setup_doctor.dart"
    )
    bad_surface_retained["selected"]["aot_surface_libraries"].sort()
    if not validate_comparison_evidence(
        bad_surface_retained, schema, receipt=receipt, catalog=catalog
    ):
        errors.append("選択外Setup Doctor libraryがFlutter AOT reportに残る結果を受け入れた")

    bad_surface_missing = copy.deepcopy(evidence)
    bad_surface_missing["selected"]["aot_surface_libraries"].remove(
        "apps/desktop_flutter/lib/screens/trace_inspector.dart"
    )
    if not validate_comparison_evidence(
        bad_surface_missing, schema, receipt=receipt, catalog=catalog
    ):
        errors.append("選択済みTrace Inspector libraryがFlutter AOT reportにない結果を受け入れた")
    return errors


def test_windows_product_package_writer_is_bounded_and_hash_bound() -> list[str]:
    errors: list[str] = []
    app_id = "d4-pocket-app-11111111111111111111111111111111"
    audit_store_id = "audit-store-22222222222222222222222222222222"
    payloads = {
        "app/data/app.so": b"app-so",
        "app/data/icudtl.dat": b"icu-data",
        "app/flutter_windows.dll": b"flutter",
        "app/gui_shell_desktop.exe": b"desktop",
        "broker/gui_shell_rust_helper.exe": b"broker",
        "gui_shell_desktop_launcher.exe": b"launcher",
        "product_manifest.json": json.dumps({
            "version": 1,
            "product": "D4 Pocket",
            "export_id": "export-test",
            "manifest": {
                "app_identity": {"app_id": app_id},
                "audit_store": {"store_id": audit_store_id, "chain_status": "new", "inherited": False},
                "inheritance_policy": {
                    "authority": "none", "permission": "none", "approval": "none",
                    "credential": "none", "audit_chain": "none",
                },
            },
        }, separators=(",", ":")).encode("utf-8"),
    }
    with tempfile.TemporaryDirectory(prefix="d4p-product-package-") as temporary:
        root = Path(temporary)
        source = root / "bundle"
        source.mkdir()
        for relative, payload in payloads.items():
            target = source / Path(relative)
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(payload)
        output = root / "D4Pocket.pkg"
        record = windows_product_package.write_product_package(
            source,
            output,
            product_version="1.2.3",
            app_id=app_id,
            audit_store_id=audit_store_id,
        )
        package = output.read_bytes()
        if package[:8] != windows_product_package.MAGIC:
            errors.append("D4PKG01 magicが固定されていない")
        if len(package) < 12:
            return errors + ["D4PKG01 headerが不完全"]
        manifest_length = int.from_bytes(package[8:12], "little")
        manifest_end = 12 + manifest_length
        try:
            manifest = json.loads(package[12:manifest_end].decode("utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError):
            return errors + ["D4PKG01 manifestがUTF-8 JSONではない"]
        schema_errors = validate_instance(
            manifest, load_schema("d4_pocket_product_package_manifest.schema.json")
        )
        if schema_errors:
            errors.append("D4PKG01 manifestが正本Schemaに適合しない")
        payload_offset = manifest_end
        for entry in manifest.get("files", []):
            byte_length = entry.get("byte_length")
            content = package[payload_offset : payload_offset + byte_length]
            if len(content) != byte_length or hashlib.sha256(content).hexdigest() != entry.get("sha256"):
                errors.append("D4PKG01 file inventoryのbyte長／SHA-256が一致しない")
                break
            payload_offset += byte_length
        if payload_offset != len(package):
            errors.append("D4PKG01 file payloadに欠落またはtrailing byteがある")
        if record["package_sha256"] != hashlib.sha256(package).hexdigest():
            errors.append("D4PKG01 receipt SHA-256が生成packageと一致しない")
        if record["file_count"] != len(payloads):
            errors.append("D4PKG01 receipt file countが一致しない")
        try:
            (source / "product_manifest.json").write_text(
                json.dumps({
                    "version": 1,
                    "product": "D4 Pocket",
                    "export_id": "export-test",
                    "manifest": {"app_identity": {"app_id": "d4-pocket-app-33333333333333333333333333333333"}},
                }),
                encoding="utf-8",
            )
            windows_product_package.write_product_package(
                source,
                root / "mismatched.pkg",
                product_version="1.2.3",
                app_id=app_id,
                audit_store_id=audit_store_id,
            )
            errors.append("Product Manifestとpackage identityの不一致を拒否しない")
        except ValueError:
            pass
        try:
            windows_product_package.write_product_package(
                source,
                source / "inside.pkg",
                product_version="1.2.3",
                app_id=app_id,
                audit_store_id=audit_store_id,
            )
            errors.append("source bundle内へのpackage再帰書込みを拒否しない")
        except ValueError:
            pass
    return errors


def test_d4_pocket_active_version_record_is_fixed_and_path_free() -> list[str]:
    schema = load_schema("d4_pocket_active_version.schema.json")
    valid = load_contract_fixture("d4_pocket_active_version.valid.json")
    errors: list[str] = []
    if validate_instance(valid, schema):
        errors.append("Bootstrapper有効版記録の正常例が正本Schemaに適合しない")
    invalid = load_contract_fixture(
        "invalid/d4_pocket_active_version_arbitrary_path.invalid.json"
    )
    if not validate_instance(invalid, schema):
        errors.append("有効版記録から任意executable pathを受け入れた")
    for field, value in (
        ("product_version", "../outside"),
        ("product_version", "release"),
        ("package_sha256", "A" * 64),
        ("launcher_sha256", "../payload"),
        ("audit_store_id", "audit-store-invalid"),
    ):
        mutated = copy.deepcopy(valid)
        mutated[field] = value
        if not validate_instance(mutated, schema):
            errors.append(f"有効版記録の不正field {field} を拒否しない")
    return errors


def test_gui_shell_windows_export_build_is_hash_bound_and_non_authoritative() -> list[str]:
    evidence = load_contract_fixture("gui_shell_windows_export_build.valid.json")
    schema = load_schema("gui_shell_windows_export_build.schema.json")
    errors: list[str] = []
    with tempfile.TemporaryDirectory(prefix="d4p-product-version-") as temporary:
        source_root = Path(temporary)
        pubspec = source_root / "apps" / "desktop_flutter" / "pubspec.yaml"
        pubspec.parent.mkdir(parents=True)
        pubspec.write_text("name: product\nversion: 2.4.1+17\n", encoding="utf-8")
        if product_version_from_source(source_root) != "2.4.1+17":
            errors.append("Product buildがsourceのpubspec版を固定できない")
        pubspec.write_text("name: product\nversion: invalid\n", encoding="utf-8")
        try:
            product_version_from_source(source_root)
            errors.append("不正なsource製品版を拒否しない")
        except ValueError:
            pass
    if validate_instance(evidence, schema):
        errors.append("Windows Export buildの正常証拠がSchemaに適合しない")
    scan_record = evidence.get("credential_artifact_scan", {})
    if (
        evidence.get("authority_boundary", {}).get("credential_artifact_scan_status")
        != "passed_known_patterns"
        or scan_record.get("scope") != "runtime_artifact_files"
        or scan_record.get("coverage") != "known_patterns_only"
        or scan_record.get("findings") != 0
        or scan_record.get("artifact_tree_sha256") != evidence.get("artifact_tree_sha256")
    ):
        errors.append("Windows Export build証拠が限定Credential scanをartifact hashへ結合しない")

    original_git_value = windows_export._git_value
    commit = "a" * 40
    git_values = {
        ("branch", "--show-current"): "main",
        ("status", "--porcelain", "--untracked-files=all"): "",
        ("rev-parse", "HEAD"): commit,
        ("ls-remote", "origin", "refs/heads/main"): f"{commit}\trefs/heads/main",
    }
    windows_export._git_value = lambda *arguments: git_values[arguments]
    try:
        if _require_clean_source() != commit:
            errors.append("clean main／GitHub同一commitをsource固定点にできない")
        git_values[("ls-remote", "origin", "refs/heads/main")] = f"{'b' * 40}\trefs/heads/main"
        try:
            _require_clean_source()
            errors.append("origin/mainと異なるsource commitでbuildを開始できる")
        except ValueError:
            pass
        git_values[("status", "--porcelain", "--untracked-files=all")] = " M dirty.txt"
        try:
            _require_clean_source()
            errors.append("dirty working treeからExport buildを開始できる")
        except ValueError:
            pass
    finally:
        windows_export._git_value = original_git_value

    inherited = {
        "PATH": "toolchain-path",
        "SystemRoot": "windows-root",
        "TEMP": "temporary-root",
        "GITHUB_TOKEN": "secret-marker",
        "AWS_SECRET_ACCESS_KEY": "secret-marker",
        "GUI_SHELL_PRODUCT_APP_ID": "inherited-product-id",
        "GUI_SHELL_UPDATE_TRUST_JSON": "inherited-update-trust",
        "CARGO_HOME": "credential-bearing-user-cache",
    }
    flutter_environment = _build_child_environment(
        {"PUB_CACHE": "isolated-pub-cache"}, inherited=inherited
    )
    if (
        flutter_environment.get("PATH") != "toolchain-path"
        or flutter_environment.get("SystemRoot") != "windows-root"
        or "GITHUB_TOKEN" in flutter_environment
        or "AWS_SECRET_ACCESS_KEY" in flutter_environment
        or "GUI_SHELL_PRODUCT_APP_ID" in flutter_environment
        or "CARGO_HOME" in flutter_environment
        or flutter_environment.get("PUB_CACHE") != "isolated-pub-cache"
    ):
        errors.append("Flutter build childへ必要最小限以外の親環境を渡す")
    rust_environment = _build_child_environment(
        {
            "CARGO_HOME": "isolated-cargo-home",
            "GUI_SHELL_PRODUCT_APP_ID": "d4-pocket-app-" + "1" * 32,
            "GUI_SHELL_PRODUCT_AUDIT_STORE_ID": "audit-store-" + "2" * 32,
            "GUI_SHELL_UPDATE_TRUST_JSON": '{"test":true}',
        },
        inherited=inherited,
    )
    if (
        rust_environment.get("CARGO_HOME") != "isolated-cargo-home"
        or rust_environment.get("GUI_SHELL_PRODUCT_APP_ID") != "d4-pocket-app-" + "1" * 32
        or rust_environment.get("GUI_SHELL_PRODUCT_AUDIT_STORE_ID") != "audit-store-" + "2" * 32
        or rust_environment.get("GUI_SHELL_UPDATE_TRUST_JSON") != '{"test":true}'
        or "GITHUB_TOKEN" in rust_environment
        or "AWS_SECRET_ACCESS_KEY" in rust_environment
    ):
        errors.append("Rust build childへcredential環境を残すか、compile-time product identity／trustを分離しない")
    no_trust_environment = _build_child_environment(
        {
            "CARGO_HOME": "isolated-cargo-home",
            "GUI_SHELL_PRODUCT_APP_ID": "d4-pocket-app-" + "1" * 32,
            "GUI_SHELL_PRODUCT_AUDIT_STORE_ID": "audit-store-" + "2" * 32,
        },
        inherited=inherited,
    )
    if "GUI_SHELL_UPDATE_TRUST_JSON" in no_trust_environment:
        errors.append("未構成Productへ親環境のupdate trustを継承する")

    with tempfile.TemporaryDirectory(prefix="d4b-") as temporary:
        temporary_root = Path(temporary)
        app_id = "d4-pocket-app-" + "1" * 32
        audit_store_id = "audit-store-" + "2" * 32
        cargo_target = _cargo_target_directory(temporary_root, app_id, audit_store_id)
        distinct_target = _cargo_target_directory(
            temporary_root, app_id, "audit-store-" + "3" * 32
        )
        if cargo_target == distinct_target or app_id in str(cargo_target) or audit_store_id in str(cargo_target):
            errors.append("Cargo target directoryがApp／Audit identity pairごとに短く分離されない")
        try:
            _validate_cargo_target_path(cargo_target)
        except ValueError as exc:
            errors.append(f"通常の短いWindows TEMPでもCargo linker pathを許可しない: {exc}")
        try:
            _validate_cargo_target_path(Path("C:/") / ("x" * 150) / "c" / ("a" * 64))
            errors.append("Windows linker path budgetを超えるCargo targetを拒否しない")
        except ValueError:
            pass

    for path, value, label in (
        (("authority_boundary", "source_authority_verified"), True, "source authority"),
        (("authority_boundary", "owner_authorization_verified"), True, "Owner authorization"),
        (("authority_boundary", "credential_values_explicitly_supplied"), True, "credential build input"),
        (("authority_boundary", "credential_inherited"), True, "credential inheritance"),
        (("authority_boundary", "permission_inherited"), True, "Permission inheritance"),
        (("authority_boundary", "approval_inherited"), True, "Approval inheritance"),
        (("authority_boundary", "audit_chain_inherited"), True, "Audit inheritance"),
        (("standalone_app_verified",), True, "standalone verification"),
        (("runtime_manifest_consumed",), True, "runtime Manifest consumption"),
        (("binary_pruning_verified",), True, "binary pruning"),
        (("formal_distribution_claimed",), True, "formal distribution"),
        (("signed",), True, "signature"),
        (("launch_status",), "verified", "live launch"),
    ):
        candidate = copy.deepcopy(evidence)
        target = candidate
        for key in path[:-1]:
            target = target[key]
        target[path[-1]] = value
        if not validate_instance(candidate, schema):
            errors.append(f"Windows Export build証拠が未成立の{label}を主張できる")

    source_manifest_path = CONTRACT_EXAMPLES / "gui_shell_export_manifest.valid.json"
    source_manifest = load_contract_fixture("gui_shell_export_manifest.valid.json")
    manifest_raw = source_manifest_path.read_bytes()
    source_receipt = load_contract_fixture("gui_shell_export_receipt.valid.json")
    receipt_raw = (CONTRACT_EXAMPLES / "gui_shell_export_receipt.valid.json").read_bytes()
    test_public_key = bytes.fromhex(
        "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"
    )
    configured_trust = {
        "版": 2,
        "algorithm": "Ed25519",
        "public_key_der_hex": (
            bytes.fromhex("302a300506032b6570032100") + test_public_key
        ).hex(),
        "public_key_fingerprint": "sha256:" + hashlib.sha256(test_public_key).hexdigest(),
        "package_sources": [
            {"channel": "stable", "base_url": "https://updates.example.invalid/d4/stable"}
        ],
    }
    configured_manifest = copy.deepcopy(source_manifest)
    configured_manifest["update_trust"] = configured_trust
    configured_manifest_raw = json.dumps(
        configured_manifest, ensure_ascii=False, separators=(",", ":")
    ).encode("utf-8")
    configured_receipt = copy.deepcopy(source_receipt)
    configured_receipt["manifest_file"]["byte_length"] = len(configured_manifest_raw)
    configured_receipt["manifest_file"]["sha256"] = (
        "sha256:" + hashlib.sha256(configured_manifest_raw).hexdigest()
    )
    try:
        validate_export_inputs(
            json.dumps(configured_receipt, ensure_ascii=False).encode("utf-8"),
            configured_manifest_raw,
        )
    except (OSError, ValueError, KeyError, TypeError) as exc:
        errors.append(f"Schema適合する製品update trustをExport build入力へ結合できない: {exc}")
    bad_trust_manifest = copy.deepcopy(configured_manifest)
    bad_trust_manifest["update_trust"]["package_sources"] = []
    if not validate_instance(bad_trust_manifest, load_schema("gui_shell_export_manifest.schema.json")):
        errors.append("配布元なしの製品update trustをExport Manifest Schemaが拒否しない")
    bad_trust_receipt = copy.deepcopy(configured_receipt)
    bad_fingerprint_manifest = copy.deepcopy(configured_manifest)
    bad_fingerprint_manifest["update_trust"]["public_key_fingerprint"] = "sha256:" + "0" * 64
    bad_fingerprint_raw = json.dumps(
        bad_fingerprint_manifest, ensure_ascii=False, separators=(",", ":")
    ).encode("utf-8")
    bad_trust_receipt["manifest_file"]["byte_length"] = len(bad_fingerprint_raw)
    bad_trust_receipt["manifest_file"]["sha256"] = (
        "sha256:" + hashlib.sha256(bad_fingerprint_raw).hexdigest()
    )
    try:
        validate_export_inputs(
            json.dumps(bad_trust_receipt, ensure_ascii=False).encode("utf-8"),
            bad_fingerprint_raw,
        )
        errors.append("Ed25519公開鍵とfingerprintが不一致のExport Manifestを受理する")
    except ValueError:
        pass
    try:
        validate_export_inputs(receipt_raw, manifest_raw)
    except (OSError, ValueError, KeyError, TypeError) as exc:
        errors.append(f"一致するReceipt／Manifest fixtureをbuild入力として検査できない: {exc}")
    else:
        bad_receipt = copy.deepcopy(source_receipt)
        bad_receipt["manifest_file"]["sha256"] = "sha256:" + "0" * 64
        try:
            validate_export_inputs(
                json.dumps(bad_receipt, ensure_ascii=False).encode("utf-8"), manifest_raw
            )
            errors.append("Manifest hashが一致しないReceiptをbuild入力として受理する")
        except ValueError:
            pass
        bad_name = copy.deepcopy(source_receipt)
        bad_name["manifest_file"]["file_name"] = "other-product.json"
        try:
            validate_export_inputs(
                json.dumps(bad_name, ensure_ascii=False).encode("utf-8"), manifest_raw
            )
            errors.append("App ID由来でないManifest file名をbuild入力として受理する")
        except ValueError:
            pass
        duplicate_key_receipt = receipt_raw.replace(
            b'"version": 1', b'"version": 1, "version": 1', 1
        )
        try:
            validate_export_inputs(duplicate_key_receipt, manifest_raw)
            errors.append("duplicate JSON keyを持つReceiptをbuild入力として受理する")
        except ValueError:
            pass

    with tempfile.TemporaryDirectory(prefix="d4pocket-export-conformance-") as temporary:
        bundle = Path(temporary) / "bundle"
        for relative, payload in (
            ("app/gui_shell_desktop.exe", b"flutter-app"),
            ("app/flutter_windows.dll", b"flutter-engine"),
            ("app/data/app.so", b"aot-app"),
            ("app/data/icudtl.dat", b"icu-data"),
            ("broker/gui_shell_rust_helper.exe", b"rust-broker"),
            ("gui_shell_desktop_launcher.exe", b"rust-launcher"),
            ("product_manifest.json", manifest_raw),
        ):
            path = bundle / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(payload)
        (bundle / "app" / "data" / "flutter_assets").mkdir(parents=True, exist_ok=True)
        (bundle / "app" / "data" / "flutter_assets" / "AssetManifest.bin").write_bytes(b"assets")
        good = copy.deepcopy(evidence)
        good["source_manifest_sha256"] = hashlib.sha256(manifest_raw).hexdigest()
        good["artifact_files"], good["artifact_total_bytes"], good["artifact_tree_sha256"] = _artifact_inventory(bundle)
        good["credential_artifact_scan"] = scan_credential_artifacts(
            bundle, expected_tree_sha256=good["artifact_tree_sha256"]
        )
        good["authority_boundary"]["credential_artifact_scan_status"] = (
            good["credential_artifact_scan"]["status"]
        )
        good["app_id"] = source_manifest["manifest"]["app_identity"]["app_id"]
        good["audit_store_id"] = source_manifest["manifest"]["audit_store"]["store_id"]
        good["rust_compile_time_identity"]["app_id"] = good["app_id"]
        good["rust_compile_time_identity"]["audit_store_id"] = good["audit_store_id"]
        product_version = product_version_from_source(ROOT)
        good["flutter_build_arguments"].append(
            f"--dart-define=GUI_SHELL_PRODUCT_VERSION={product_version}"
        )
        validate_build_evidence(good, bundle)

        product_manifest_path = bundle / "product_manifest.json"
        configured_summary = {
            "source": "schema_validated_manifest",
            "configured": True,
            "public_key_fingerprint": configured_trust["public_key_fingerprint"],
            "package_source_count": 1,
        }
        configured_good = copy.deepcopy(good)
        configured_good["rust_compile_time_update_trust"] = configured_summary
        product_manifest_path.write_bytes(configured_manifest_raw)
        configured_good["source_manifest_sha256"] = hashlib.sha256(
            configured_manifest_raw
        ).hexdigest()
        (
            configured_good["artifact_files"],
            configured_good["artifact_total_bytes"],
            configured_good["artifact_tree_sha256"],
        ) = _artifact_inventory(bundle)
        configured_good["credential_artifact_scan"] = scan_credential_artifacts(
            bundle, expected_tree_sha256=configured_good["artifact_tree_sha256"]
        )
        configured_good["authority_boundary"]["credential_artifact_scan_status"] = (
            configured_good["credential_artifact_scan"]["status"]
        )
        try:
            validate_build_evidence(configured_good, bundle)
        except (OSError, ValueError, KeyError, TypeError) as exc:
            errors.append(f"compile-time update trustのManifest／evidence結合が成立しない: {exc}")
        forged_trust_evidence = copy.deepcopy(configured_good)
        forged_trust_evidence["rust_compile_time_update_trust"]["package_source_count"] = 0
        try:
            validate_build_evidence(forged_trust_evidence, bundle)
            errors.append("Manifestと異なるcompile-time update trust evidenceを受理する")
        except ValueError:
            pass
        product_manifest_path.write_bytes(manifest_raw)

        receipt_path = bundle / windows_export.RECEIPT_NAME
        receipt_raw = (json.dumps(good, ensure_ascii=False, indent=2) + "\n").encode("utf-8")
        receipt_path.write_bytes(receipt_raw)
        portable_zip = Path(temporary) / "D4 Pocket portable.zip"
        original_portable_sys = windows_portable_package.sys
        original_portable_clean_source = windows_portable_package._require_clean_source
        original_portable_version = windows_portable_package.product_version_from_source
        try:
            windows_portable_package.sys = type("WindowsTestHost", (), {"platform": "win32"})()
            windows_portable_package._require_clean_source = lambda: good["source_commit"]
            windows_portable_package.product_version_from_source = lambda _source: product_version
            portable_result = windows_portable_package.build_export_portable_zip(bundle, portable_zip)
            with windows_portable_package.zipfile.ZipFile(portable_zip, "r") as archive:
                zip_names = set(archive.namelist())
                expected_names = {
                    f"{windows_portable_package.PORTABLE_ROOT}/{record['path']}"
                    for record in good["artifact_files"]
                } | {
                    f"{windows_portable_package.PORTABLE_ROOT}/{windows_export.RECEIPT_NAME}",
                    f"{windows_portable_package.PORTABLE_ROOT}/{windows_portable_package.GUIDE_NAME}",
                }
                if zip_names != expected_names or archive.testzip() is not None:
                    errors.append("portable ZIPのroot／全payload／Receipt／guideが正確に一致しない")
                if archive.read(
                    f"{windows_portable_package.PORTABLE_ROOT}/{windows_export.RECEIPT_NAME}"
                ) != receipt_raw:
                    errors.append("portable ZIPが検証済みbuild receiptをbyte-for-byte保持しない")
                guide = archive.read(
                    f"{windows_portable_package.PORTABLE_ROOT}/{windows_portable_package.GUIDE_NAME}"
                ).decode("utf-8")
                if "保護設定を変更せず" not in guide or "正式配布物ではありません" not in guide:
                    errors.append("portable guideが未署名／セキュリティ保護の注意を明示しない")
            if (
                portable_result["signed"]
                or portable_result["formal_distribution_claimed"]
                or portable_result["standalone_app_verified"]
                or portable_result["archive_sha256"] != hashlib.sha256(portable_zip.read_bytes()).hexdigest()
            ):
                errors.append("portable ZIPの結果が未署名／非正式配布／未起動を正確に保持しない")
        except (OSError, ValueError, KeyError, zipfile.BadZipFile) as exc:
            errors.append(f"有効なExport bundleからportable ZIPを作成・再検証できない: {exc}")
        finally:
            windows_portable_package.sys = original_portable_sys
            windows_portable_package._require_clean_source = original_portable_clean_source
            windows_portable_package.product_version_from_source = original_portable_version

        try:
            windows_portable_package._resolve_output_file(portable_zip, bundle)
            errors.append("既存portable ZIP outputの上書きを拒否しない")
        except ValueError:
            pass
        try:
            windows_portable_package._resolve_output_file(bundle / "inside.zip", bundle)
            errors.append("input bundle内へのportable ZIP outputを拒否しない")
        except ValueError:
            pass

        tampered = bundle / "app" / "flutter_windows.dll"
        original_tampered = tampered.read_bytes()
        tampered.write_bytes(original_tampered + b"changed")
        tampered_output = Path(temporary) / "tampered.zip"
        try:
            windows_portable_package.sys = type("WindowsTestHost", (), {"platform": "win32"})()
            windows_portable_package._require_clean_source = lambda: good["source_commit"]
            windows_portable_package.product_version_from_source = lambda _source: product_version
            windows_portable_package.build_export_portable_zip(bundle, tampered_output)
            errors.append("receipt inventoryと異なるbundleからportable ZIPを作成できる")
        except ValueError:
            pass
        finally:
            windows_portable_package.sys = original_portable_sys
            windows_portable_package._require_clean_source = original_portable_clean_source
            windows_portable_package.product_version_from_source = original_portable_version
            tampered.write_bytes(original_tampered)
        if tampered_output.exists():
            errors.append("不一致bundle拒否時にportable ZIP outputが残る")

        probe = bundle / "app" / "data" / "flutter_assets" / "credential-probe.bin"
        probe.write_bytes(
            b"x" * (windows_export.CREDENTIAL_SCAN_CHUNK_BYTES - 6)
            + b"AKIA1234567890ABCDEF"
        )
        try:
            scan_credential_artifacts(bundle)
            errors.append("chunk境界をまたぐ既知AWS access-key markerをCredential scanが拒否しない")
        except ValueError:
            pass
        credential_field = "access_token"
        credential_value = "synthetic-not-a-real-secret"
        probe.write_bytes(f"{credential_field}={credential_value}".encode("ascii"))
        try:
            scan_credential_artifacts(bundle)
            errors.append("Credential名に結び付いたsynthetic値をscanが拒否しない")
        except ValueError as error:
            diagnostic = str(error)
            if (
                f"field={credential_field}" not in diagnostic
                or f"value_length={len(credential_value)}" not in diagnostic
                or credential_value in diagnostic
            ):
                errors.append("Credential scan診断がfield／長さだけを示し、候補値を秘匿しない")
        probe.write_bytes(b"synthetic private-key boundary: -----BEGIN PRIVATE KEY-----")
        try:
            scan_credential_artifacts(bundle)
            errors.append("private-key markerを含むportable bundleをCredential scanが拒否しない")
        except ValueError:
            pass
        for label, marker in (
            ("GitHub", b"ghp_" + b"a" * 40),
            ("GitHub stateless", b"ghs_APPID_" + b"a" * 48 + b"." + b"b" * 32),
            ("Google", b"AIza" + b"A" * 35),
            ("Slack", b"xoxb-" + b"A" * 16),
            ("Slack app-level", b"xapp-" + b"A" * 16),
            ("Slack workflow", b"xwfp-" + b"A" * 16),
            ("Bearer", b"Authorization: Bearer " + b"A" * 32),
        ):
            probe.write_bytes(marker)
            try:
                scan_credential_artifacts(bundle)
                errors.append(f"{label}のsynthetic credential markerをscanが拒否しない")
            except ValueError:
                pass
        probe.unlink()
        named_secret = bundle / "app" / "data" / "flutter_assets" / "credentials.json"
        named_secret.write_bytes(b"synthetic")
        try:
            scan_credential_artifacts(bundle)
            errors.append("Credential固定file名を含むportable bundleをscanが拒否しない")
        except ValueError:
            pass
        named_secret.unlink()
        rescan = scan_credential_artifacts(bundle)
        if (
            rescan["findings"] != 0
            or rescan["artifact_tree_sha256"] != good["artifact_tree_sha256"]
        ):
            errors.append("Credential scanのnegative probe後に元のartifact treeへ戻らない")
        forged_scan = copy.deepcopy(good)
        forged_scan["credential_artifact_scan"]["files_scanned"] += 1
        try:
            validate_build_evidence(forged_scan, bundle)
            errors.append("portable bundleと一致しないCredential scan件数をbuild evidenceが受け入れる")
        except ValueError:
            pass

        mismatch = copy.deepcopy(good)
        mismatch["rust_compile_time_identity"]["app_id"] = "d4-pocket-app-" + "9" * 32
        try:
            validate_build_evidence(mismatch, bundle)
            errors.append("Rust compile-time App IDとManifestの不一致を受理する")
        except ValueError:
            pass

        manifest_mismatch = copy.deepcopy(good)
        manifest_mismatch["app_id"] = "d4-pocket-app-" + "9" * 32
        manifest_mismatch["rust_compile_time_identity"]["app_id"] = manifest_mismatch["app_id"]
        try:
            validate_build_evidence(manifest_mismatch, bundle)
            errors.append("Manifest内App IDとExport build evidenceの不一致を受理する")
        except ValueError:
            pass

        (bundle / "app" / "gui_shell_desktop.exe").write_bytes(b"tampered")
        try:
            validate_build_evidence(good, bundle)
            errors.append("変更後artifactを古いbuild inventoryで受理する")
        except ValueError:
            pass

    with tempfile.TemporaryDirectory(prefix="d4pocket-export-location-") as temporary:
        temporary_path = Path(temporary)
        one_drive_root = temporary_path / "OneDrive - Test"
        one_drive_root.mkdir()
        try:
            _resolve_output_directory(one_drive_root / "bundle")
            errors.append("OneDrive同期directoryへのExport build出力を許可する")
        except ValueError:
            pass
        try:
            accepted_output = _resolve_output_directory(temporary_path / "bundle")
            if accepted_output != temporary_path / "bundle":
                errors.append("外部の短いoutput pathを安定して解決しない")
        except (OSError, ValueError) as exc:
            errors.append(f"OneDrive外のoutput directoryを解決できない: {exc}")

    with tempfile.TemporaryDirectory(prefix="d4pocket-export-copy-") as temporary:
        root = Path(temporary)
        flutter_release = root / "flutter-release"
        for relative, payload in (
            ("gui_shell_desktop.exe", b"flutter-app"),
            ("flutter_windows.dll", b"flutter-engine"),
            ("data/app.so", b"aot-app"),
            ("data/icudtl.dat", b"icu-data"),
            ("data/flutter_assets/AssetManifest.bin", b"assets"),
        ):
            path = flutter_release / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(payload)
        helper = root / "gui_shell_rust_helper.exe"
        helper.write_bytes(b"rust-broker")
        launcher = root / "gui_shell_desktop_launcher.exe"
        launcher.write_bytes(b"rust-launcher")
        copied = root / "copied-bundle"
        _copy_bundle(flutter_release, helper, launcher, manifest_raw, copied)
        if (copied / "product_manifest.json").read_bytes() != manifest_raw:
            errors.append("portable bundleが入力Manifestをbyte-for-byte保持しない")
        (flutter_release / "data" / "icudtl.dat").unlink()
        incomplete = root / "incomplete-bundle"
        try:
            _copy_bundle(flutter_release, helper, launcher, manifest_raw, incomplete)
            errors.append("Flutter locale data欠落bundleをExport buildが完成扱いする")
        except ValueError:
            pass

    def archive_bytes(entries: list[tuple[str, bytes | None, bytes]]) -> bytes:
        output = io.BytesIO()
        with tarfile.open(fileobj=output, mode="w") as archive:
            for name, kind, payload in entries:
                member = tarfile.TarInfo(name)
                if kind is None:
                    member.type = tarfile.REGTYPE
                    member.size = len(payload)
                    archive.addfile(member, io.BytesIO(payload))
                else:
                    member.type = kind
                    archive.addfile(member)
        return output.getvalue()

    with tempfile.TemporaryDirectory(prefix="d4pocket-source-extract-") as temporary:
        temporary_path = Path(temporary)
        safe_archive = temporary_path / "safe.tar"
        safe_archive.write_bytes(archive_bytes([("src/file.txt", None, b"safe")]))
        safe_root = temporary_path / "safe-root"
        safe_root.mkdir()
        _safe_extract_source(safe_archive, safe_root)
        if (safe_root / "src" / "file.txt").read_bytes() != b"safe":
            errors.append("許可されたGit archive fileをsource snapshotへ展開できない")

        for name, kind, label in (
            ("../escape.txt", None, "root traversal"),
            ("bad-link", tarfile.SYMTYPE, "symlink"),
            ("Case.txt", None, "case-fold collision"),
        ):
            archive_path = temporary_path / f"{label.replace(' ', '-')}.tar"
            entries = [(name, kind, b"payload")]
            if label == "case-fold collision":
                entries = [("Case.txt", None, b"one"), ("case.txt", None, b"two")]
            archive_path.write_bytes(archive_bytes(entries))
            destination = temporary_path / f"extract-{label.replace(' ', '-')}"
            destination.mkdir()
            try:
                _safe_extract_source(archive_path, destination)
                errors.append(f"Git source archiveの{label}を拒否しない")
            except ValueError:
                pass
    return errors


def load_bounded_extension_fixture() -> dict:
    return load_contract_fixture(BOUNDED_EXTENSION_FIXTURE)


def bounded_extension_validation_errors(extension: dict) -> list[str]:
    errors = []
    for key, schema_name in BOUNDED_EXTENSION_RECORD_SCHEMAS.items():
        record = extension.get(key)
        if not isinstance(record, dict):
            errors.append(f"bounded extensionにobject recordがない: {key}")
            continue
        for failure in validate_instance(record, load_schema(schema_name)):
            errors.append(f"bounded extension {key} が{schema_name}で失敗: {failure}")

    runtime = extension.get("runtime", {})
    adapter = extension.get("adapter", {})
    runtime_manifest = extension.get("runtime_manifest", {})
    adapter_manifest = extension.get("adapter_manifest", {})
    capability = extension.get("capability", {})
    permission = extension.get("permission", {})
    approval = extension.get("approval", {})
    audit_event = extension.get("audit_event", {})
    recovery_action = extension.get("recovery_action", {})
    content_policy = extension.get("content_exposure_policy", {})

    runtime_id = runtime.get("runtime_id")
    adapter_id = adapter.get("adapter_id")
    capability_id = capability.get("capability_id")
    permission_id = permission.get("permission_id")

    if extension.get("evidence_classification") != "contract_conformance":
        errors.append("bounded extensionはcontract_conformanceに分類されなければならない")
    required_non_claims = {
        "not_installed_product_evidence",
        "not_windows_release_evidence",
        "not_cross_agent_reproduction_evidence",
        "not_public_standard_adoption_evidence",
    }
    missing_non_claims = required_non_claims - set(extension.get("non_claims", []))
    if missing_non_claims:
        errors.append(f"bounded extensionにnon-claimがない: {', '.join(sorted(missing_non_claims))}")

    if runtime_id != runtime_manifest.get("runtime_id"):
        errors.append("bounded extensionのruntime_manifest runtime_idがruntimeと一致しない")
    if runtime_id != adapter.get("runtime_id") or runtime_id != adapter_manifest.get("runtime_id"):
        errors.append("bounded extensionのadapter runtime_idがruntimeと一致しない")
    if runtime.get("adapter_id") != adapter_id or adapter_manifest.get("adapter_id") != adapter_id:
        errors.append("bounded extensionのadapter_id linkageが不整合である")
    if runtime_manifest.get("runtime_type") != "tool_runtime" or runtime.get("kind") != "tool":
        errors.append("bounded extensionのreferenceはtool runtimeのままでなければならない")
    if adapter.get("transport") != "mock" or adapter_manifest.get("transport") != "mock":
        errors.append("bounded extensionのreferenceはprivileged transportを必須にしてはならない")
    if adapter.get("authority_strip") is not True or adapter_manifest.get("authority_strip") is not True:
        errors.append("bounded extensionのadapterはauthority_strip=trueを必須にしなければならない")
    if runtime_manifest.get("signed_manifest") is not True or adapter_manifest.get("signed_manifest") is not True:
        errors.append("bounded extensionのmanifestはsignedでなければならない")

    for record_name, capabilities in {
        "runtime": runtime.get("capabilities", []),
        "runtime_manifest": runtime_manifest.get("capabilities", []),
        "adapter": adapter.get("declared_capabilities", []),
        "adapter_manifest": adapter_manifest.get("declared_capabilities", []),
    }.items():
        if capability_id not in capabilities:
            errors.append(f"bounded extension {record_name} がcapability {capability_id}を宣言していない")
    if permission_id not in runtime_manifest.get("permissions", []):
        errors.append("bounded extensionのruntime_manifestがpermissionを宣言していない")
    if permission.get("capability_id") != capability_id:
        errors.append("bounded extensionのpermissionがcapabilityへ対応付けられていない")
    if permission.get("source") != "policy":
        errors.append("bounded extensionのpermission sourceはruntimeまたはmetadataではなくpolicyでなければならない")
    if permission.get("decision") != "allow":
        errors.append("bounded extensionのpositive fixtureのpermissionはallowでなければならない")

    if approval.get("runtime_id") != runtime_id or approval.get("operation") != capability_id:
        errors.append("bounded extensionのapprovalがruntime capabilityへ対応付けられていない")
    if approval.get("status") != "approved":
        errors.append("bounded extensionのpositive fixtureのapprovalはapprovedでなければならない")
    if audit_event.get("payload_hash") != approval.get("payload_hash"):
        errors.append("bounded extensionのaudit payload_hashがapproval payload_hashと一致しない")
    if audit_event.get("action") != capability_id or audit_event.get("target") != runtime_id:
        errors.append("bounded extensionのaudit target/actionがruntime capabilityへ対応付けられていない")
    if not recovery_action.get("recovery_id"):
        errors.append("bounded extensionのrecovery mappingがない")

    if content_policy.get("default_visibility") != "none":
        errors.append("bounded extensionのcontent exposure defaultはnoneでなければならない")
    if "full" in content_policy.get("allowed_visibility", []):
        errors.append("bounded extensionのcontent exposure policyはfull payloadを許可してはならない")
    if approval.get("content_visibility") not in content_policy.get("allowed_visibility", []):
        errors.append("bounded extensionのapproval visibilityがcontent policyの範囲外である")

    return errors


def build_bounded_extension_state(extension: dict) -> RuntimeState:
    state = RuntimeState()
    state.register_runtime(extension["runtime"])
    state.register_adapter(extension["adapter"])
    state.register_capability(extension["capability"])
    state.record_permission(extension["permission"])
    state.enqueue_approval(extension["approval"])
    state.append_audit_event(extension["audit_event"])
    state.register_recovery_action(extension["recovery_action"])
    state.register_update_policy(extension["update_policy"])
    return state


def build_bounded_extension_action(extension: dict) -> dict:
    approval = extension["approval"]
    return {
        "runtime_id": extension["runtime"]["runtime_id"],
        "operation": extension["capability"]["capability_id"],
        "capability_id": extension["capability"]["capability_id"],
        "permission_id": extension["permission"]["permission_id"],
        "approval_id": approval["approval_id"],
        "approval_state": "approved",
        "target_scope": extension["permission"].get("target_scope", "diagnostic_summary"),
        "payload": approval["redacted_payload"],
        "audit_event": extension["audit_event"],
        "recovery_action": extension["recovery_action"],
        "adapter_metadata": extension["adapter"].get("metadata", {}),
    }


def test_l3_bounded_reference_extension_uses_existing_contracts() -> list[str]:
    extension = load_bounded_extension_fixture()
    errors = bounded_extension_validation_errors(extension)

    catalog = RuntimeCatalog()
    try:
        catalog.register_runtime_manifest(extension["runtime_manifest"])
        catalog.register_adapter_manifest(extension["adapter_manifest"])
    except ValueError as exc:
        errors.append(f"bounded extensionのmanifest registrationが失敗: {exc}")
    if catalog.can_grant_authority(extension["runtime_manifest"]):
        errors.append("bounded extensionのruntime manifestがauthorityを付与した")
    if catalog.metadata_attempts_authority(extension["adapter_manifest"].get("metadata", {})):
        errors.append("bounded extensionのadapter manifest metadataがauthorityを試行した")

    try:
        adapter_record = load_adapter(extension["adapter"])
    except ValueError as exc:
        errors.append(f"bounded extensionのadapter loadが失敗: {exc}")
    else:
        if adapter_record.effective_capabilities() != tuple(extension["adapter"]["declared_capabilities"]):
            errors.append("bounded extensionのadapter metadataがeffective capabilitiesを変更した")

    marker = extension["runtime"]["runtime_id"]
    for path in sorted(SHELL_CORE.glob("*.py")):
        if marker in path.read_text(encoding="utf-8"):
            errors.append(f"bounded extensionのruntime固有markerがShell Coreへ漏れた: {path}")
    return errors


def test_l3_bounded_reference_extension_governed_path_accepts_declared_mapping() -> list[str]:
    extension = load_bounded_extension_fixture()
    state = build_bounded_extension_state(extension)
    result = PolicyEvaluator(state).evaluate(build_bounded_extension_action(extension))
    errors = []
    if not result["allowed"]:
        errors.append(f"bounded extensionの宣言済みmappingが拒否された: {result['errors']}")
    if result.get("audit_required") is not True:
        errors.append("bounded extensionのpolicy resultがauditを必須にしなかった")

    projected = project_approval_content(extension["approval"])
    if "full_payload" in projected:
        errors.append("bounded extensionがfull visibilityなしでfull payloadを射影した")
    if "redacted_payload" not in projected:
        errors.append("bounded extensionが宣言済みのredacted payloadを射影しなかった")
    return errors


def test_l3_bounded_reference_extension_negative_cases_fail_closed() -> list[str]:
    extension = load_bounded_extension_fixture()
    errors = []

    def assert_policy_error(label: str, mutate, expected_code: str) -> None:
        state = build_bounded_extension_state(extension)
        action = build_bounded_extension_action(extension)
        mutate(state, action)
        result = PolicyEvaluator(state).evaluate(action)
        if result["allowed"] or expected_code not in error_codes(result):
            errors.append(f"bounded extensionのnegative caseが{label}でfail closedにならなかった: {result}")

    assert_policy_error(
        "adapter metadata authority escalation",
        lambda state, action: action.update({"adapter_metadata": {"generated_config": {"permissionGrant": "all"}}}),
        "adapter_metadata_escalation_attempt",
    )
    assert_policy_error(
        "undeclared capability",
        lambda state, action: action.update({"capability_id": "diagnostic.write.undeclared"}),
        "unknown_capability",
    )
    assert_policy_error(
        "undeclared permission",
        lambda state, action: action.update({"permission_id": "permission.diagnostic.write.undeclared"}),
        "unknown_permission",
    )
    assert_policy_error(
        "self-approved action without approval id",
        lambda state, action: action.pop("approval_id"),
        "approval_missing",
    )
    assert_policy_error(
        "missing audit mapping",
        lambda state, action: action.pop("audit_event"),
        "audit_mapping_missing",
    )
    assert_policy_error(
        "missing recovery mapping",
        lambda state, action: action.pop("recovery_action"),
        "recovery_mapping_missing",
    )

    for source in sorted(NON_AUTHORITY_SOURCES):
        assert_policy_error(
            f"{source} authority source",
            lambda state, action, source=source: action.update({"authority_source": source}),
            "non_authority_source_attempt",
        )

    approval = copy.deepcopy(extension["approval"])
    approval["content_visibility"] = "full"
    policy = extension["content_exposure_policy"]
    projected = project_approval_content(approval)
    if approval["content_visibility"] in policy["allowed_visibility"]:
        errors.append("bounded extensionのcontent policyがfull visibilityを許可した")
    if "full_payload" not in projected:
        errors.append("bounded extensionのfull visibility変更がpolicyの拒否が必要な理由を示さなかった")
    return errors


def test_audit_chain_verification_fails_on_tampered_event() -> list[str]:
    event = load_contract_fixture("audit.valid.json")
    first = chain_event(event, None)
    second = chain_event({**event, "event_id": "audit-2", "target": "runtime"}, first["event_hash"])
    valid = verify_audit_chain([first, second])
    tampered = copy.deepcopy(second)
    tampered["target"] = "tampered"
    invalid = verify_audit_chain([first, tampered])
    errors = []
    if valid["ok"] is not True:
        errors.append("有効なaudit chainを検証できなかった")
    if invalid["ok"] is not False:
        errors.append("改ざんされたaudit chainの検証に成功した")
    return errors


def test_audit_chain_rejects_duplicate_event_ids() -> list[str]:
    event = load_contract_fixture("audit.valid.json")
    first = chain_event(event, None)
    duplicate = chain_event({**event, "target": "runtime"}, first["event_hash"])
    invalid = verify_audit_chain([first, duplicate])
    errors = []
    if invalid["ok"] is not False:
        errors.append("重複したaudit event_idのchainの検証に成功した")

    state = RuntimeState()
    state.append_audit_event(event)
    try:
        state.append_audit_event(copy.deepcopy(event))
        errors.append("RuntimeStateが重複したaudit event_idのoverwriteを許可した")
    except ValueError:
        pass

    from packages.shell_core.audit_store import AuditStore
    from packages.shell_core.persistence import JsonPersistence

    store = AuditStore()
    store.append({"event_id": "audit-1", "action": "test", "result": "success"})
    try:
        store.append({"event_id": "audit-1", "action": "test", "result": "success"})
        errors.append("AuditStoreが重複したaudit event_idのappendを許可した")
    except ValueError:
        pass

    with tempfile.TemporaryDirectory(prefix="gui-shell-audit-duplicate-") as directory:
        persistence = JsonPersistence(Path(directory))
        persistence.append_audit_event(event)
        try:
            persistence.append_audit_event(copy.deepcopy(event))
            errors.append("JsonPersistenceが重複したaudit event_idのappendを許可した")
        except ValueError:
            pass
    return errors


def test_json_persistence_reports_corrupt_audit_jsonl() -> list[str]:
    from packages.shell_core.persistence import JsonPersistence

    event = load_contract_fixture("audit.valid.json")
    with tempfile.TemporaryDirectory(prefix="gui-shell-audit-corrupt-") as directory:
        persistence = JsonPersistence(Path(directory))
        persistence.append_audit_event(event)
        with persistence.audit_path.open("a", encoding="utf-8") as handle:
            handle.write("{not-json}\n")
        report = persistence.audit_events_report()
        errors = []
        if not report["errors"]:
            errors.append("JsonPersistenceがcorrupt audit JSONLのlineを報告しなかった")
        if persistence.verify_audit_chain()["ok"] is not False:
            errors.append("JsonPersistenceがcorrupt audit JSONLの検証に成功した")
        try:
            persistence.audit_events()
            errors.append("JsonPersistenceのaudit_eventsがcorrupt JSONLでfail closedにならなかった")
        except ValueError:
            pass
        return errors


def test_platform_hardening_configuration_exists() -> list[str]:
    errors = []
    gitattributes = ROOT / ".gitattributes"
    if not gitattributes.exists():
        errors.append(".gitattributesが存在しない")
    else:
        text = gitattributes.read_text(encoding="utf-8")
        for token in ["* text=auto eol=lf", "*.ps1 text eol=lf", "*.exe binary"]:
            if token not in text:
                errors.append(f".gitattributesにtokenがない: {token}")

    from tooling.manual_workflow_check import 手動補助一覧検査

    errors.extend(手動補助一覧検査(ROOT))

    main_rs = (RUST_HELPER / "src" / "main.rs").read_text(encoding="utf-8")
    if "dev-stdin-smoke" not in main_rs:
        errors.append("Rust helperのdev stdin smokeがexplicit subcommandの後方に隔離されていない")
    if "使用法: gui_shell_rust_helper broker-server" not in main_rs:
        errors.append("Rust helperが未知/引数なしの呼び出しでusageへfail closedにならない")
    return errors


def 手動補助の起動境界を検査する() -> list[str]:
    from tooling.manual_workflow_check import 手動起動検査, 手動補助一覧検査

    不整合 = []
    for 本文 in (
        "on: workflow_dispatch\n",
        "on: [workflow_dispatch]\n",
        "on:\n  workflow_dispatch:\n",
        '\"on\": {workflow_dispatch: {inputs: {target: {type: string}}}}\n',
        "on: workflow_dispatch\njobs: {build: {steps: [{run: 'echo push'}]}}\n",
    ):
        if 手動起動検査(本文):
            不整合.append("手動補助の正規起動が拒否された")
    for 本文 in (
        "", "[]", "on: push", "on: [workflow_dispatch, push]",
        "on: {workflow_dispatch: {}, pull_request: {}}",
        "on: {merge_group: {}}", "on: {schedule: []}",
        "on: {workflow_call: {}}", "on: {repository_dispatch: {}}",
        "on: workflow_dispatch\non: push", "on: push\non: workflow_dispatch",
        "on: {workflow_dispatch: {}, workflow_dispatch: {}}",
        "on: [", "on: workflow_dispatch\n---\non: push",
        "on: {$ref: workflow_dispatch}", "on: {workflow_dispatch: false}",
        "on: workflow_dispatch\n<<: {on: push}",
        "on: workflow_dispatch\njobs: !!python/object/apply:os.system []",
    ):
        if not 手動起動検査(本文):
            不整合.append(f"禁止または不正な起動条件が許可された: {本文}")
    with tempfile.TemporaryDirectory() as 場所:
        ルート = Path(場所)
        if 手動補助一覧検査(ルート):
            不整合.append("workflow 不在が拒否された")
        格納先 = ルート / ".github" / "workflows"
        格納先.mkdir(parents=True)
        (格納先 / "manual.yml").write_text("on: workflow_dispatch", encoding="utf-8")
        if 手動補助一覧検査(ルート):
            不整合.append("手動 workflow ファイルが拒否された")
        (格納先 / "automatic.yaml").write_text("on: push", encoding="utf-8")
        if not 手動補助一覧検査(ルート):
            不整合.append("実ファイル経路で自動起動が見逃された")
    export_workflow = (ROOT / ".github" / "workflows" / "windows-manual-export-build.yml").read_text(encoding="utf-8")
    if "python -X utf8 tooling/package_windows_portable.py" not in export_workflow:
        不整合.append("手動Windows Export workflowが検証済みportable ZIPを生成しない")
    if "actions/upload-artifact" in export_workflow:
        不整合.append("Windows Export workflowが一時fixture ZIPを外部artifactとして残す")
    if not any(token in export_workflow for token in ("'on': workflow_dispatch", '"on": workflow_dispatch', "on: workflow_dispatch")):
        不整合.append("portable ZIP補助がworkflow_dispatch限定ではない")
    return 不整合


def test_macos_workspace_selection_is_native_scoped_and_non_authoritative() -> list[str]:
    schema = json.loads((SPECS / "macos_workspace_selection.schema.json").read_text(encoding="utf-8"))
    errors = []
    if validate_instance({"version": 1}, schema, "workspace-selection"):
        errors.append("OS選択の正規要求が拒否される")
    for key in ("workspace_root", "bookmark", "permission", "approval"):
        if not validate_instance({"version": 1, key: "injected"}, schema, "workspace-selection"):
            errors.append(f"OS選択要求でUIの権限・pathが受理される: {key}")
    protocol = (RUST_HELPER / "src/broker/protocol.rs").read_text(encoding="utf-8")
    worker = (RUST_HELPER / "src/macos_workspace_selection.rs").read_text(encoding="utf-8")
    for text, tokens in ((protocol, ("MacOSWorkspaceSelection", "macos_os_selection_required", '"registration_generated":false')),
                         (worker, ("SCOPE_LIMIT: usize = 8", "seen.len() >= 64", '"selection_request_hash"', '"workspace_root"'))):
        for token in tokens:
            if token not in text.replace('"registration_generated": false', '"registration_generated":false'):
                errors.append(f"OS選択の境界接続がない: {token}")
    for name in ("DebugProfile", "Release"):
        entitlements = (DESKTOP_FLUTTER / f"macos/Runner/{name}.entitlements").read_text(encoding="utf-8")
        if "com.apple.security.files.user-selected.read-write" not in entitlements:
            errors.append("OS chooser用の限定entitlementがない")
    native = (ROOT / "native/macos_owner/src/workspace_transport.rs").read_text(encoding="utf-8")
    runner = (DESKTOP_FLUTTER / "macos/Runner/BrokerProcessChannel.swift").read_text(encoding="utf-8")
    for token in ("d4_workspace_select_and_write", "is_fifo()", "F_SETNOSIGPIPE", "NativeSelection", "bookmark.zeroize()",
                  "beginSheetModalForWindow_completionHandler", "generation != generation", "added_latest", "pending.borrow_mut().take()"):
        if token not in native:
            errors.append(f"親Rust OS選択の限定配送・回収接続がない: {token}")
    for token in ('object["native_workspace_selection"] != nil', "d4WorkspaceReleaseAll()", "d4WorkspaceReleaseLast()"):
        if token not in runner:
            errors.append(f"OS選択transportのprivate入力拒否・回収接続がない: {token}")
    if "workspace-ui-diagnostic" in (ROOT / "native/rust_helper/Cargo.toml").read_text(encoding="utf-8"):
        errors.append("原因特定済みのOS選択診断featureが残っている")
    return errors


def test_macos_agent_cli_selection_is_non_authoritative() -> list[str]:
    errors = []
    request_schema = json.loads((SPECS / "macos_agent_cli_selection.schema.json").read_text(encoding="utf-8"))
    receipt_schema = json.loads((SPECS / "macos_agent_cli_selection_receipt.schema.json").read_text(encoding="utf-8"))
    if validate_instance({"version": 1}, request_schema, "cli-selection"):
        errors.append("CLIの正規OS選択要求が拒否される")
    for key in ("cli_path", "bookmark", "approval", "permission", "mode"):
        if not validate_instance({"version": 1, key: "injected"}, request_schema, "cli-selection"):
            errors.append(f"CLI OS選択がUIのpath・権限・modeを受け取る: {key}")
    receipt = json.loads((ROOT / "examples/contracts/macos_agent_cli_selection_receipt.valid.json").read_text(encoding="utf-8"))
    if validate_instance(receipt, receipt_schema, "cli-receipt"):
        errors.append("CLI選択の非権限投影が拒否される")
    for key in ("approval_generated", "permission_generated", "registration_generated"):
        if not validate_instance({**receipt, key: True}, receipt_schema, "cli-receipt"):
            errors.append(f"CLI選択が権限・登録を生成する: {key}")
    if not validate_instance({**receipt, "cli_path": "/bad\npath"}, receipt_schema, "cli-receipt"):
        errors.append("CLI投影が制御文字を許す")
    protocol = (RUST_HELPER / "src/broker/protocol.rs").read_text(encoding="utf-8")
    if "MacOSAgentCLISelection" not in protocol or "AgentCLI実行fileOS選択" not in protocol:
        errors.append("CLI OS選択の独立sourceとBroker操作が接続されていない")
    return errors


def test_setup_doctor_public_bind_warning_exists() -> list[str]:
    from installer.setup_doctor import setup_doctor_report

    report = setup_doctor_report()
    matches = [check for check in report["checks"] if check["check_id"] == "network.public_bind"]
    if not matches or matches[0].get("status") != "warning" or not matches[0].get("recovery_action"):
        return ["Setup Doctorのpublic bind warningがない"]
    return []


def test_desktop_setup_doctor_ui_does_not_require_development_toolchains() -> list[str]:
    source = (DESKTOP_FLUTTER / "lib" / "screens" / "setup_doctor.dart").read_text(encoding="utf-8")
    errors = []
    for token in (
        "dart:io",
        "Platform.",
        "Flutterツールチェーン",
        "Python: tooling/",
        "Rust helper: validate_all",
        "snapshot.snapshotPath",
        "evidence.path",
    ):
        if token in source:
            errors.append(f"製品Setup Doctor UIに開発環境依存または生path表示が残る: {token}")
    for token in (
        "snapshot.snapshotSource",
        "snapshot.snapshotFreshness",
        "snapshot.networkExposure",
        "snapshot.auditChainStatus",
    ):
        if token not in source:
            errors.append(f"製品Setup Doctor UIにBroker snapshot診断項目がない: {token}")
    return errors


def test_broker_parity_startup_timeout_allows_local_cold_build() -> list[str]:
    if DEFAULT_BROKER_START_TIMEOUT_SECONDS < 60.0:
        return ["broker parityのstartup timeoutがlocal cold Rust buildに対して短すぎる"]
    return []


def test_broker_parity_waits_after_process_kill() -> list[str]:
    text = (ROOT / "tooling" / "broker_parity" / "run_authority_parity.py").read_text(encoding="utf-8")
    errors = []
    for token in [
        "def wait_for_process_exit(",
        "process.kill()",
        "process.wait(timeout=timeout)",
        "finally:\n                wait_for_process_exit(broker.process)",
    ]:
        if token not in text:
            errors.append(f"broker parityのcleanupにtokenがない: {token}")
    return errors


def test_desktop_agent_center_required_surface_exists() -> list[str]:
    path = DESKTOP_FLUTTER / "lib" / "screens" / "agent_center.dart"
    text = path.read_text(encoding="utf-8")
    required = [
        "作業領域",
        "タスク",
        "変更ファイル",
        "道具呼出し",
        "シェルコマンド",
        "試験状態",
        "差分概要",
        "保留中の承認",
        "巻戻し候補",
        "監査リンク",
    ]
    return [f"エージェントセンターにsurfaceがない: {item}" for item in required if item not in text]


def test_c28_harness_regressions_are_registered() -> list[str]:
    result = subprocess.run(
        [sys.executable, "-m", "unittest", "tooling.conformance_tests.test_long_run_validation"],
        cwd=ROOT,
        capture_output=True,
        timeout=30,
    )
    if result.returncode == 0:
        return []
    details = (result.stdout + result.stderr).decode("utf-8", errors="replace")[-4000:]
    return ["C28資格file cleanupの有限再試行testが失敗: " + details]


def test_macos_credential_public_input_is_metadata_only() -> list[str]:
    schema = load_schema("macos_credential_input.schema.json")
    sample = {"版": 1, "資格情報ID": "a" * 32, "用途": "mcp_transport", "接続対象": "server", "種類": "api_key"}
    errors = []
    if validate_instance(sample, schema):
        errors.append("Mac資格情報の公開metadataが拒否された")
    for field in ("秘密値", "Approval", "Permission", "保管方式", "登録者種別", "登録経路"):
        if not validate_instance({**sample, field: "injected"}, schema):
            errors.append(f"Mac資格情報の公開要求に注入できる: {field}")
    return errors


def test_macos_mcp_native_scope_is_explicit() -> list[str]:
    """CONFIG証拠。native実作用は専用手動Mac試験で別途確認する。"""
    required = {
        "docs/specs/macos-mcp-center.md": ["MAC-MCP-1", "MAC-MCP-5", "Credential", "process group"],
        "native/rust_helper/src/macos_desktop_owner.rs": ["macos_owner_summary", "response_timeout", "payload_hash"],
        "native/rust_helper/src/adapters/process_tree.rs": ["process_group(0)", "getpgid", "test_kill_process_group", "self.group = None"],
        ".github/workflows/apple-manual-build.yml": ["macos_mcp_center", "testProductMacMcpCenter"],
    }
    errors = []
    for path, tokens in required.items():
        source = (ROOT / path).read_text(encoding="utf-8")
        errors.extend(f"Mac MCP責任経路がない: {path}: {token}" for token in tokens if token not in source)
    return errors


def macos_mcp資格情報参照の公開境界を検査する() -> list[str]:
    """CONFIG証拠。秘密の実受渡し・Owner確認は専用Mac実行で確認する。"""
    schema = load_schema("mcp_connection.schema.json")
    sample = {"版": 1, "操作": "接続", "ServerID": "mac-credential-server",
              "実行file": "/fixture/server", "引数": [], "workspace": "/fixture/workspace",
              "Transport": "stdio", "Credential ref": {"credential_id": "a" * 32,
              "purpose": "mcp_transport", "target": "mac-credential-server", "required": True,
              "status": "configured", "environment_variable": "MCP_API_KEY"}}
    errors = []
    if validate_instance(sample, schema):
        errors.append("Mac MCPの公開資格情報参照が拒否された")
    for field in ("秘密値", "Approval", "Permission", "保管先"):
        if not validate_instance({**sample, field: "injected"}, schema):
            errors.append(f"Mac MCP参照に非公開fieldを注入できる: {field}")
    return errors


def macos_a2a接続の限定責任経路を検査する() -> list[str]:
    """CONFIG証拠。新native入口の実作用は専用手動Mac試験で確認する。"""
    required = {
        "docs/specs/macos-a2a-center.md": ["MAC-A2A-1", "MAC-A2A-3", "metadata-only", "pending_review"],
        "native/rust_helper/src/macos_desktop_owner.rs": ['operation == "A2A接続"', "a2a_center::owner_confirmation_summary", "macos_a2a_同一要求"],
        "apps/desktop_flutter/macos/Runner/BrokerProcessChannel.swift": ['operation == "A2A接続" ? 315', "result(response)"],
        ".github/workflows/apple-manual-build.yml": ["macos_a2a_center", "testProductMacA2aCenter", "D4_A2A_FIXTURE_SERVED_ONCE"],
    }
    errors = []
    for path, tokens in required.items():
        source = (ROOT / path).read_text(encoding="utf-8")
        errors.extend(f"Mac A2A責任経路がない: {path}: {token}" for token in tokens if token not in source)
    index = (ROOT / "規定/正本索引.json").read_text(encoding="utf-8")
    if "docs/specs/macos-a2a-center.md" not in index:
        errors.append("Mac A2A責任正本が索引にない")
    return errors


def macos_host登録の限定責任経路を検査する() -> list[str]:
    """CONFIG証拠。実native確認と製品操作は専用Mac正常経路だけで確認する。"""
    required = {
        "docs/specs/macos-host-registration.md": ["MAC-HOST-1", "MAC-HOST-3", "metadata", "申告"],
        "native/rust_helper/src/macos_desktop_owner.rs": ['operation == "Host登録"', "host_center::owner_confirmation_summary", "macos_host_同一要求"],
        "native/rust_helper/src/broker/host_center.rs": ["parse_request(payload).ok()?", "件数は実測ではありません"],
        "apps/desktop_flutter/lib/services/host_registration_client.dart": ["Host登録", "Host一覧", "_count", "authority_strip", "pending_review"],
        "apps/desktop_flutter/lib/screens/host_operation_center.dart": ["TargetPlatform.macOS", "client.refresh()", "selectedHost: host"],
        ".github/workflows/apple-manual-build.yml": ["macos_host_registration", "testProductMacHostRegistration"],
    }
    errors = []
    for path, tokens in required.items():
        source = (ROOT / path).read_text(encoding="utf-8")
        errors.extend(f"Mac Host責任経路がない: {path}: {token}" for token in tokens if token not in source)
    if "docs/specs/macos-host-registration.md" not in (ROOT / "規定/正本索引.json").read_text(encoding="utf-8"):
        errors.append("Mac Host責任正本が索引にない")
    return errors


def macos_workspace内容露出の限定責任経路を検査する() -> list[str]:
    """CONFIG証拠。現在のnative確認・APFS読取・製品差分は専用正常試験で確認する。"""
    required = {
        "docs/specs/macos-workspace-inspector.md": ["MAC-INSPECT-1", "MAC-INSPECT-3", "Task", "secret"],
        "native/rust_helper/src/macos_desktop_owner.rs": ["workspace::owner_confirmation_summary", "macos_workspace内容露出_同一要求"],
        "native/rust_helper/src/broker/workspace.rs": ["owner_confirmation_summary", "Task実行・書込権は付与しません"],
        "apps/desktop_flutter/macos/Runner/BrokerProcessChannel.swift": ["作業領域承認", "作業領域失効", "作業領域全体基準点保存"],
        ".github/workflows/apple-manual-build.yml": ["macos_workspace_inspector", "testProductMacWorkspaceInspector"],
    }
    errors = []
    for path, tokens in required.items():
        source = (ROOT / path).read_text(encoding="utf-8")
        errors.extend(f"Mac Workspace内容露出の責任経路がない: {path}: {token}" for token in tokens if token not in source)
    if "docs/specs/macos-workspace-inspector.md" not in (ROOT / "規定/正本索引.json").read_text(encoding="utf-8"):
        errors.append("Mac Workspace内容露出の責任正本が索引にない")
    return errors


def main() -> int:
    tests = [
        test_required_docs_exist,
        test_gui_shell_spec_v1_declares_core_boundaries,
        test_contract_fixtures_are_available,
        test_setup_doctor_report_schema_preserves_unknown_and_denies_authority,
        test_first_run_configuration_schema_is_fixed_and_non_authoritative,
        test_first_run_configuration_request_rejects_caller_paths_and_authority,
        test_buffer_intervention_record_is_non_authoritative_and_evidence_bound,
        test_negative_contract_fixtures_cover_all_schemas,
        test_adapter_authority_strip_schema,
        test_inbound_authority_keys_are_stripped,
        test_adapter_loader_strips_authority_metadata_from_effective_payload,
        test_adapter_loader_rejects_value_only_authority_metadata,
        test_runtime_state_adapter_registration_uses_loader_boundary,
        test_normalization_firewall_rejects_authority_aliases,
        test_normalization_firewall_detects_value_only_escalation,
        test_normalization_firewall_detects_key_collisions,
        test_external_metadata_cannot_escalate_authority,
        test_gui_input_cannot_create_runtime_disallowed_authority_context,
        test_memory_cache_previous_state_cannot_grant_authority,
        test_content_exposure_contract,
        test_full_content_only_visible_when_full,
        test_approval_schema_has_protected_field_sets,
        test_protected_approval_fields_cannot_be_edited,
        test_approval_edits_are_rehashed_and_revalidated,
        test_sensitive_actions_map_to_audit_and_recovery,
        test_hash_patterns_are_tagged_sha256,
        test_framework_risk_profile_exists,
        test_update_fixture_requires_signature,
        test_update_policy_unsigned_rejection_uses_taxonomy,
        test_update_center_contract_and_execution_boundary,
        test_update_download_transport_is_broker_owned_and_bounded,
        test_notification_center_contract_and_navigation_boundary,
        test_observation_center_contract_and_audit_separation,
        test_trace_inspector_surface_and_evidence_boundary,
        test_shell_contracts_load_required_schemas,
        test_shell_core_ignores_adapter_metadata_permissions,
        test_shell_core_non_authority_sources_do_not_grant_authority,
        test_shell_core_routes_sensitive_actions_through_required_mapping,
        test_shell_core_content_projection_hides_full_payload_until_full,
        test_content_projection_missing_visibility_fails_closed,
        test_shell_core_has_no_flutter_imports,
        test_shell_core_has_no_blue_tanuki_internal_imports,
        test_policy_evaluator_rejects_unknown_capability,
        test_policy_evaluator_returns_structured_errors,
        test_policy_evaluator_rejects_unknown_permission,
        test_policy_evaluator_rejects_denied_permission,
        test_policy_evaluator_rejects_missing_approval,
        test_policy_evaluator_rejects_self_reported_approval_without_approval_id,
        test_policy_evaluator_rejects_unknown_approval_id,
        test_policy_evaluator_uses_runtime_state_approval_status,
        test_policy_evaluator_rejects_unapproved_runtime_state_approval,
        test_policy_evaluator_rejects_missing_audit_event,
        test_policy_evaluator_rejects_missing_recovery_action,
        test_policy_evaluator_rejects_unknown_recovery_id,
        test_policy_evaluator_accepts_known_recovery_id,
        test_policy_evaluator_ignores_adapter_metadata_authority,
        test_policy_evaluator_normalizes_adapter_metadata_authority,
        test_policy_evaluator_rejects_non_authority_source,
        test_policy_evaluator_enforces_action_envelope_relations,
        test_sensitive_action_router_uses_policy_evaluator_when_state_is_provided,
        test_sensitive_action_router_blocks_policy_denied_action,
        test_state_snapshot_is_deterministic,
        test_state_snapshot_reports_invariant_flags,
        test_invariant_evaluator_scans_nested_shell_core_python,
        test_shell_core_integrated_release_smoke,
        test_json_persistence_rejects_truncated_audit_anchor,
        test_release_smoke_runs_first_run_and_setup_doctor,
        test_shell_snapshot_contains_gui_operation_state,
        test_shell_snapshot_generator_writes_phase_b_local_snapshot,
        test_evidence_bundle_is_development_classified_and_non_authoritative,
        test_windows_release_evidence_validator_accepts_other_checks_but_rejects_unbound_anchor,
        test_windows_release_evidence_validator_rejects_missing_provenance,
        test_windows_release_evidence_validator_preserves_audit_anchor_external_blocker,
        test_windows_release_evidence_validator_rejects_authority_and_missing_installed_path,
        test_windows_release_evidence_validator_rejects_preexisting_first_run_config,
        test_windows_release_evidence_rejects_unbound_first_run_config_audit,
        test_windows_release_evidence_validator_rejects_external_setup_probe_as_product_evidence,
        test_windows_setup_doctor_does_not_promote_unknown_to_release_pass,
        test_windows_setup_doctor_readability_requires_bound_visible_uia_evidence,
        test_windows_release_evidence_requires_accepted_broker_health_audit,
        test_windows_release_evidence_requires_verified_launcher_child,
        test_windows_release_evidence_requires_normal_frontend_exit,
        test_windows_release_evidence_rejects_staged_runtime_scratch_path,
        test_windows_release_evidence_rejects_malformed_product_runtime_identity,
        test_windows_release_evidence_rejects_runtime_or_config_outside_isolated_profile,
        test_windows_release_evidence_validator_rejects_unmeasured_or_synthetic_evidence,
        test_windows_release_evidence_validator_rejects_broker_top_level_unmeasured_declarations,
        test_windows_broker_evidence_requires_native_owner_denial_for_agent_task_grants,
        test_windows_japanese_surface_labels,
        test_windows_surface_geometry_and_identity,
        test_windows_release_evidence_validator_rejects_missing_surface_matches,
        test_windows_release_evidence_validator_rejects_screenshot_surface_source,
        test_windows_release_evidence_validator_rejects_flutter_build_registry_as_visibility,
        test_windows_release_evidence_validator_rejects_aggregate_surface_root_match,
        test_flutter_setup_doctor_has_no_filesystem_export_path,
        test_setup_doctor_filesystem_probe_is_bounded_and_non_destructive,
        test_windows_stage_installer_powershell_boolean_grouping,
        test_windows_stage_uses_terminal_free_native_launcher,
        test_windows_installed_smoke_preserves_trap_failure,
        test_windows_broker_smoke_keeps_full_duplex_response,
        test_windows_broker_smoke_checks_agent_task_grants_through_normal_ipc,
        test_windows_installed_smoke_reads_json_as_utf8,
        test_windows_installed_smoke_uses_launcher_owned_runtime,
        test_windows_installed_smoke_exit_matches_native_tray_contract,
        test_windows_installed_smoke_uses_bounded_control_uia_tree,
        test_windows_installed_smoke_automation_names_are_materialized,
        test_windows_installed_smoke_uia_properties_are_stringified,
        test_windows_audit_anchor_proof_collector_is_connected,
        test_invariant_evaluator_detects_intentional_import_violation,
        test_invariant_evaluator_detects_live_authority_invariants,
        test_rust_helper_required_sources_exist,
        test_rust_helper_contract_shape_exists,
        test_rust_helper_does_not_expose_hidden_authority_paths,
        test_codex_cli_adapter_is_broker_governed_and_bounded,
        test_broker_ipc_contract_schemas_exist,
        test_broker_boundary_docs_exist,
        test_desktop_broker_channel_contract,
        test_rust_broker_skeleton_exists,
        test_process_supervision_unsafe_exception_is_narrow,
        test_rust_broker_rejection_audit_contract_shape,
        test_rust_broker_audit_anchor_and_nonce_compaction_present,
        test_rust_filesystem_diagnostic_detects_secret_symlink,
        test_desktop_flutter_does_not_spawn_python_or_use_ffi_authority_bridge,
        test_desktop_flutter_windows_runner_rejects_native_surface_aggregate_injection,
        test_desktop_flutter_windows_uses_platform_accessibility_lifecycle,
        test_desktop_flutter_exposes_individual_surface_semantics_identifiers,
        test_desktop_flutter_product_baseline_chrome_exists,
        test_validate_all_uses_running_python_interpreter_for_python_steps,
        test_release_docs_declare_language_policy_runtime_blockers,
        test_blue_tanuki_adapter_runtime_output_validates_against_generic_schema,
        test_blue_tanuki_adapter_metadata_cannot_escalate_authority,
        test_blue_tanuki_adapter_cannot_expose_full_payload_unless_visibility_full,
        test_blue_tanuki_adapter_cannot_mark_approvals_approved_by_itself,
        test_blue_tanuki_adapter_failures_map_to_recovery_actions,
        test_desktop_flutter_required_files_exist,
        test_desktop_flutter_keeps_authority_in_shell_core_client,
        test_desktop_flutter_exposes_operation_surfaces,
        test_installer_setup_doctor_reports_structured_status_without_authority,
        test_installer_boundary_docs_exist,
        test_mobile_flutter_required_files_exist,
        test_mobile_flutter_cannot_create_hidden_authority,
        test_release_hardening_files_exist,
        test_release_hardening_does_not_overclaim_readiness,
        test_validation_reporter_exists,
        test_validate_all_resolves_windows_batch_commands,
        test_validate_all_subprocess_start_failure_is_structured,
        test_validate_all_strict_release_runs_release_gate_strict_scan,
        test_release_blocker_registry_controls_strict_release,
        test_release_facing_docs_sync_release_blockers_to_registry,
        test_release_gate_scans_ipc_threat_model,
        test_packaging_portability_checker_exists,
        test_packaging_portability_subprocess_output_is_utf8_safe,
        test_packaging_portability_utf8_governance_allowlist_is_exact,
        書庫展開で日本語名と内容を保持する,
        対話契約の関係と表示境界を検査する,
        対話操作の分岐と未知fieldを検査する,
        対話セッション一覧の有界投影を検査する,
        履歴入力概要のhash_only境界を検査する,
        AgentTask履歴のContractと現在承認経路を検査する,
        実行系資源観測の証拠境界を検査する,
        実行系ライフサイクルの契約と統治境界を検査する,
        端末契約の構造と禁止操作を検査する,
        Mobile_native_Device_Link_channelを秘密非通過に制限する,
        Mobile_local_deleteを有界の非権威回復記録へ閉じる,
        test_schema_validator_supports_composition_keywords,
        test_schema_validator_enforces_date_time_format,
        二実行系比較の非混線を検査する,
        評価ラボの契約と境界を検査する,
        回帰Caseの契約と境界を検査する,
        資格情報保管庫の契約と境界を検査する,
        MCP外部概念射影の契約と境界を検査する,
        A2A外部概念射影の契約と境界を検査する,
        MCP接続センターの統治経路と境界を検査する,
        A2A接続センターの統治経路と境界を検査する,
        複数Host_registryの統治経路と境界を検査する,
        Host操作面の観測境界とHost間非混線を検査する,
        Adapter管理操作の統治境界を検査する,
        Windows常駐トレイ操作面の統治境界を検査する,
        コマンドパレット拡張の統治境界を検査する,
        グローバル検索の統治境界を検査する,
        Desktop_UX統合の表示境界を検査する,
        Mobile投影の統治境界を検査する,
        test_manifest_integrity_tooling_exists,
        test_manifest_rejects_working_tree_eol_mismatch,
        test_claim_documents_do_not_contain_stale_phase_or_check_counts,
        test_runtime_manifest_invalid_fixture_rejected,
        test_adapter_manifest_authority_escalation_rejected,
        test_runtime_catalog_cannot_grant_authority,
        test_host_capability_is_observation_not_authority,
        作業領域検査要求の分岐を検査する,
        作業領域応答の露出境界を検査する,
        test_workspace_diff_content_shape,
        test_agent_workspace_outside_access_default_deny,
        test_agent_task_request_cannot_carry_authority_or_dialogue_approval,
        test_agent_broker_operations_are_declared_in_ipc_contracts,
        test_agent_cli_runtime_registration_is_owner_scoped_and_authority_free,
        test_provider_model_selection_is_registered_and_non_authoritative,
        test_agent_task_owner_confirmation_wait_uses_native_operation_timeout,
        test_agent_task_id_operations_are_content_free_and_declared,
        test_agent_task_workspace_permission_is_owner_scoped_and_one_use,
        test_agent_task_owner_approval_receipt_is_hash_bound_and_unconsumed,
        test_agent_task_state_record_is_versioned_scoped_and_content_free,
        test_agent_task_scratch_recovery_journal_is_bounded_and_content_free,
        test_agent_secret_path_read_default_deny,
        test_agent_secret_path_symlink_default_deny,
        test_agent_shell_command_requires_permission_mapping,
        test_agent_git_push_requires_explicit_approval,
        test_agent_generated_diff_must_be_auditable,
        test_agent_auto_permission_is_advisory_only,
        test_agent_adapter_is_declaration_only_and_unsupported_is_explicit,
        test_broker_agent_metadata_is_validated_before_projection,
        test_agent_adapter_probe_is_read_only_and_fail_closed,
        test_agent_comparison_projection_is_isolated_and_non_authoritative,
        test_agent_handoff_projection_requires_reassessment_and_redaction,
        test_gui_shell_compose_is_manifest_only_and_non_inheriting,
        test_gui_shell_preview_is_read_only_and_non_rollback,
        test_gui_shell_edit_proposal_is_owner_review_only,
        test_gui_shell_export_is_new_identity_and_non_inheriting,
        test_gui_shell_module_build_is_untrusted_ui_only_selection,
        test_gui_shell_windows_export_build_is_hash_bound_and_non_authoritative,
        test_windows_product_package_writer_is_bounded_and_hash_bound,
        test_d4_pocket_active_version_record_is_fixed_and_path_free,
        test_gui_shell_module_comparison_is_same_commit_and_non_authoritative,
        test_l3_bounded_reference_extension_uses_existing_contracts,
        test_l3_bounded_reference_extension_governed_path_accepts_declared_mapping,
        test_l3_bounded_reference_extension_negative_cases_fail_closed,
        test_audit_chain_verification_fails_on_tampered_event,
        test_audit_chain_rejects_duplicate_event_ids,
        test_json_persistence_reports_corrupt_audit_jsonl,
        test_platform_hardening_configuration_exists,
        手動補助の起動境界を検査する,
        test_macos_workspace_selection_is_native_scoped_and_non_authoritative,
        test_macos_agent_cli_selection_is_non_authoritative,
        test_macos_credential_public_input_is_metadata_only,
        test_macos_mcp_native_scope_is_explicit,
        macos_mcp資格情報参照の公開境界を検査する,
        macos_a2a接続の限定責任経路を検査する,
        macos_host登録の限定責任経路を検査する,
        macos_workspace内容露出の限定責任経路を検査する,
        test_setup_doctor_public_bind_warning_exists,
        test_desktop_setup_doctor_ui_does_not_require_development_toolchains,
        test_broker_parity_startup_timeout_allows_local_cold_build,
        test_broker_parity_waits_after_process_kill,
        test_c28_harness_regressions_are_registered,
        test_desktop_agent_center_required_surface_exists,
    ]
    errors = []
    for test in tests:
        errors.extend(test())

    if errors:
        print("conformance skeletonが失敗:")
        for err in errors:
            print(f"  - {err}")
        return 1

    print(f"conformance skeletonが合格: {len(tests)} 件のcheck")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
