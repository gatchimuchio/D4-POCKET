from pathlib import Path
import copy
import hashlib
import json
import re
import subprocess
import sys
import tempfile

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
from tooling.packaging_portability_check import portable_path_errors
from tooling.release_gate_check import (
    CURRENT_FACING_RELEASE_DOCS,
    release_blocker_doc_sync_errors,
    registry_blocker_names,
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

REQUIRED_SCHEMA_NAMES = {
    "workspace_diff",
    "workspace_inspection_request",
    "workspace_startup",
    "workspace_inspection_response",
    "runtime_dialogue_operation",
    "runtime_dialogue_request",
    "runtime_dialogue_session",
    "runtime_dialogue_response",
    "runtime_dialogue_comparison",
    "evaluation_dataset",
    "evaluation_case",
    "evaluation_evaluator",
    "evaluation_dataset_registration",
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
    "content_exposure",
    "framework_risk_profile",
    "runtime_manifest",
    "adapter_manifest",
    "agent_runtime",
    "agent_adapter",
    "agent_session",
    "agent_workspace",
    "agent_task",
    "agent_tool_call",
    "agent_diff",
    "ipc_request",
    "ipc_response",
    "broker_error",
    "broker_session",
    "broker_health",
    "broker_command_envelope",
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
}
BROKER_REQUIRED_SCHEMAS = {
    "ipc_request.schema.json",
    "ipc_response.schema.json",
    "broker_error.schema.json",
    "broker_endpoint.schema.json",
    "broker_session.schema.json",
    "broker_health.schema.json",
    "broker_command_envelope.schema.json",
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
    "lib/services/surface_semantics_export.dart",
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
        "runtime-catalog.md",
    }
    existing = {path.name for path in DOC_SPECS.glob("*.md")}
    for missing in sorted(required_docs - existing):
        errors.append(f"docs/specs/{missing} が存在しない")
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
    for check_id in [
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
    ]:
        setup_checks.append(
            {
                "check_id": check_id,
                "status": "pass",
                "message": f"{check_id} passed",
                "recovery_instruction": "Rerun installed-path smoke after remediation.",
                "grants_authority": False,
            }
        )
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
            "broker_artifact_sha256": "sha256:" + "3" * 64,
            "isolation": {
                "uses_shared_fixed_install_root": False,
                "isolated_install_root": r"C:\Users\owner\AppData\Local\GUI-Shell\installed-runs\run-20260605T000000Z-a1b2c3d4",
                "isolated_runtime_dir": r"C:\Users\owner\AppData\Local\GUI-Shell\installed-runs\run-20260605T000000Z-a1b2c3d4\runtime",
                "isolated_store_dir": r"C:\Users\owner\AppData\Local\GUI-Shell\installed-runs\run-20260605T000000Z-a1b2c3d4\runtime\broker_store",
                "isolated_config_dir": r"C:\Users\owner\AppData\Local\GUI-Shell\installed-runs\run-20260605T000000Z-a1b2c3d4\runtime\config",
                "isolated_audit_dir": r"C:\Users\owner\AppData\Local\GUI-Shell\installed-runs\run-20260605T000000Z-a1b2c3d4\runtime\audit",
            },
            "evidence_bundle_sha256": "sha256:" + "4" * 64,
            "evidence_bundle_files": [
                {"kind": "setup_doctor", "path": r"C:\evidence\setup_doctor.json", "sha256": "sha256:" + "5" * 64},
                {"kind": "broker_smoke", "path": r"C:\evidence\broker.json", "sha256": "sha256:" + "6" * 64},
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
            "first_run.installer_authority_boundary": {"source_type": "static_assertion", "evidence_class": "CONFIG", "formal_release_input": True},
            "setup_doctor": {"source_type": "product_export", "evidence_class": "LIVE_RUNTIME", "formal_release_input": True},
            "broker.ipc_restart_crash": {"source_type": "directly_measured", "evidence_class": "LIVE_RUNTIME", "formal_release_input": True},
            "audit_anchor.external_tamper_evidence": {"source_type": "directly_measured", "evidence_class": "EXTERNAL_EVIDENCE", "formal_release_input": True},
            "release_runtime_assertions": {"source_type": "static_assertion", "evidence_class": ["CONFIG", "FIXTURE"], "formal_release_input": True},
            "unsupported_claims": [],
        },
        "evidence_source": {
            "collector": "installer/windows/collect_installed_smoke.ps1",
            "collector_version": "6",
            "manual_confirmation": False,
            "screenshot_path": r"C:\ProgramData\GUI-Shell\evidence\first-window.png",
        },
        "artifact": {
            "installed_exe_path": r"C:\Users\owner\AppData\Local\GUI-Shell\installed-runs\run-20260605T000000Z-a1b2c3d4\app\gui_shell_desktop.exe",
            "installed_exe_exists": True,
            "sha256": "sha256:" + "1" * 64,
        },
        "first_run": {
            "status": "passed",
            "command": r".\gui_shell_desktop.exe",
            "launched_from_installed_path": True,
            "process_id": 1234,
            "process_running_after_launch": True,
            "main_window_handle": 100,
            "window_title": "GUI-Shell",
            "first_window_visible": True,
            "broker_mediated_launch": True,
            "broker_helper_path": r"C:\Program Files\GUI-Shell\broker\gui_shell_rust_helper.exe",
            "broker_endpoint_file": r"C:\ProgramData\GUI-Shell\broker\broker_session.json",
            "broker_endpoint_created": True,
            "broker_transport": "authenticated_loopback_tcp",
            "broker_endpoint_credential_role": "normal",
            "normal_endpoint_credential_role_verified": True,
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
                },
            },
            "config_path": r"C:\ProgramData\GUI-Shell\config\gui_shell.json",
            "config_created": True,
            "config_json_valid": True,
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
            "status": "warning",
            "formal_product_evidence": True,
            "evidence_source": {
                "source_kind": "installed_app_machine_readable_export",
                "product_generated": True,
                "collector_derives_checks": False,
                "synthetic": False,
                "command": r".\gui_shell_desktop.exe --setup-doctor --json",
            },
            "ran_from_installed_app_path": True,
            "operator_readable": True,
            "installer_grants_authority": False,
            "installer_silently_approves_permissions": False,
            "checks": setup_checks,
        },
        "broker": {
            "status": "passed",
            "evidence_source": {
                "collector": "installer/windows/collect_broker_smoke.ps1",
                "collector_version": "1",
                "synthetic": False,
                "command": r"powershell -ExecutionPolicy Bypass -File installer\windows\collect_broker_smoke.ps1",
            },
            "helper_exe_path": r"C:\Program Files\GUI-Shell\broker\gui_shell_rust_helper.exe",
            "helper_exe_exists": True,
            "session_file": r"C:\ProgramData\GUI-Shell\broker\broker_session.json",
            "session_file_created": True,
            "store_dir": r"C:\ProgramData\GUI-Shell\broker\store",
            "endpoint_host": "127.0.0.1",
            "endpoint_port": 49152,
            "endpoint_credential_role": "normal",
            "normal_endpoint_credential_role_verified": True,
            "restricted_loopback_bind": True,
            "authenticated_ipc_connection": True,
            "durable_store_ready": True,
            "replay_nonce": "windows-installed-replay-nonce",
            "restart_replay_rejected": True,
            "replay_error_code": "broker_replay_detected",
            "fresh_health_after_restart": True,
            "crash_fail_closed": True,
            "field_provenance": {
                "helper_exe_exists": {"source_type": "directly_measured", "evidence_class": "EXTERNAL_EVIDENCE"},
                "session_file_created": {"source_type": "directly_measured", "evidence_class": "LIVE_RUNTIME"},
                "normal_endpoint_credential_role_verified": {"source_type": "directly_measured", "evidence_class": "LIVE_RUNTIME"},
                "restricted_loopback_bind": {"source_type": "directly_measured", "evidence_class": "LIVE_RUNTIME"},
                "authenticated_ipc_connection": {"source_type": "directly_measured", "evidence_class": "LIVE_RUNTIME"},
                "durable_store_ready": {"source_type": "directly_measured", "evidence_class": "LIVE_RUNTIME"},
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


def test_installed_app_setup_doctor_product_export_contract_exists() -> list[str]:
    export = DESKTOP_FLUTTER / "lib" / "services" / "setup_doctor_export.dart"
    main = DESKTOP_FLUTTER / "lib" / "main.dart"
    collector = INSTALLER / "windows" / "collect_installed_smoke.ps1"
    if not export.exists():
        return ["installed appのSetup Doctor product export helperが存在しない"]
    export_text = export.read_text(encoding="utf-8")
    main_text = main.read_text(encoding="utf-8")
    collector_text = collector.read_text(encoding="utf-8")
    required_export_tokens = [
        "GUI_SHELL_SETUP_DOCTOR_EXPORT_JSON",
        "GUI_SHELL_SETUP_DOCTOR_CONTEXT_JSON",
        "installed_app_machine_readable_export",
        "'formal_product_evidence': true",
        "'product_generated': true",
        "'collector_derives_checks': false",
        "'synthetic': false",
        "'installer_grants_authority': false",
        "'installer_silently_approves_permissions': false",
        "gui_shell_desktop_installed_first_run",
        "_ensureInstalledFirstRunConfig",
    ]
    required_checks = [
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
    ]
    errors = [
        f"Setup Doctorのproduct exportにtokenがない: {token}"
        for token in required_export_tokens
        if token not in export_text
    ]
    errors.extend(
        f"Setup Doctorのproduct exportに必須check idがない: {check_id}"
        for check_id in required_checks
        if check_id not in export_text
    )
    if "writeSetupDoctorProductExportIfRequested(client.getSnapshot())" not in main_text:
        errors.append("desktop mainがinstalled-appのSetup Doctor product export helperを呼び出していない")
    for token in [
        "GUI_SHELL_SETUP_DOCTOR_EXPORT_JSON",
        "GUI_SHELL_SETUP_DOCTOR_CONTEXT_JSON",
        "インストール済み app が環境診断の製品出力を書き出しませんでした",
        "setup_doctor_context",
    ]:
        if token not in collector_text:
            errors.append(f"installed smoke collectorにproduct export tokenがない: {token}")
    external_probe_text = (INSTALLER / "windows" / "collect_setup_doctor.ps1").read_text(encoding="utf-8")
    if (
        "formal_product_evidence" in external_probe_text
        and "formal_product_evidence = $false" not in external_probe_text
    ):
        errors.append("外部Setup Doctor probeが明確に非正式ではなくなった")
    return errors


def test_windows_stage_installer_powershell_boolean_grouping() -> list[str]:
    text = (INSTALLER / "windows" / "stage_installed_app.ps1").read_text(encoding="utf-8")
    errors = []
    if "Test-Path $InstallRoot -and" in text:
        errors.append("stage_installed_app.ps1が-andをTest-Pathのargumentとして渡している")
    if "if ((Test-Path $InstallRoot) -and" not in text:
        errors.append("stage_installed_app.ps1にgroup化されたTest-Path boolean conditionがない")
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
        "GUI_SHELL_SURFACE_SEMANTICS_EXPORT_JSON",
        "surface_semantics_export.json",
    ]:
        if token not in text:
            errors.append(f"collect_installed_smoke.ps1にmaterialize済みUIAutomation evidence tokenがない: {token}")
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
    for blocker in active:
        if blocker.get("classification") != "release_blocker":
            errors.append(f"active blocker {blocker.get('name')} がrelease_blockerに分類されていない")
        if blocker.get("blocks_release") is not True:
            errors.append(f"active blocker {blocker.get('name')} がreleaseを阻止していない")
    release_gate = (ROOT / "tooling" / "release_gate_check.py").read_text(encoding="utf-8")
    for token in ["RELEASE_BLOCKERS_REGISTRY", "unresolved_active_blockers", "strict releaseのactive blockerが未解決"]:
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
    operations = {"端末結合": {}, "端末確認": {}, "端末離脱": {}, "実行系列挙": {},
                  "対話開始": {"実行系ID": "local"}, "対話送信": {"対話セッションID": "a" * 32, "入力": "こんにちは"},
                  "対話取得": {"要求ID": "b" * 32}, "対話中止": {"要求ID": "b" * 32}, "対話終了": {"対話セッションID": "a" * 32}}
    for operation, payload in operations.items():
        errors.extend(validate_instance({**sample, "操作": operation, "内容": payload}, schema))
        if not validate_instance({**sample, "操作": operation, "内容": {**payload, "owner": True}}, schema):
            errors.append("端末操作が権限fieldを受理")
    for operation in ("対話承認", "対話承認待ち", "shutdown", "command_envelope", "端末招待", "端末失効"):
        if not validate_instance({**sample, "操作": operation, "内容": {}}, schema):
            errors.append("端末経路が禁止操作を受理: " + operation)
    if not validate_instance({**sample, "操作": "対話取得", "内容": {"対話セッションID": "a" * 32}}, schema):
        errors.append("端末要求が操作と内容の不一致を受理")
    return errors


def 対話操作の分岐と未知fieldを検査する() -> list[str]:
    schema = load_schema("runtime_dialogue_operation.schema.json")
    errors = []
    samples = {
        "実行系列挙": {}, "対話承認待ち": {}, "対話開始": {"実行系ID": "local"},
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
    for value in ({"operation": "未知操作", "payload": {}}, {"operation": "対話取得", "payload": {"実行系ID": "local"}}):
        if not validate_instance(value, schema):
            errors.append("操作とpayloadの不整合を受理した")
    if not validate_instance({}, {"oneOf": [{"type": "object"}, {"type": "object"}]}):
        errors.append("oneOfの複数一致を拒否しなかった")
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
    セッション = load_contract_fixture("runtime_dialogue_session.valid.json")
    応答 = load_contract_fixture("runtime_dialogue_response.valid.json")
    不整合 = 要求関係検査(要求, セッション) + 応答関係検査(要求, 応答, "none")
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
        text = path.read_text(encoding="utf-8")
        for pattern in forbidden:
            if pattern in text:
                errors.append(f"{path} が禁止されたhelper authority patternを使う: {pattern}")
    return errors


def test_codex_cli_adapter_is_broker_governed_and_bounded() -> list[str]:
    adapter_path = RUST_HELPER / "src" / "adapters" / "codex_cli.rs"
    broker_path = RUST_HELPER / "src" / "broker" / "ipc_server.rs"
    if not adapter_path.is_file():
        return ["Codex CLI Adapter sourceが存在しない"]
    adapter = adapter_path.read_text(encoding="utf-8")
    broker = broker_path.read_text(encoding="utf-8")
    required = [
        "--json",
        "--ephemeral",
        "--sandbox",
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
    forbidden = [
        "--dangerously-bypass-approvals-and-sandbox",
        "--worktree",
        "--add-dir",
        "OPENAI_API_KEY",
        "CODEX_API_KEY",
        "command_envelope",
    ]
    for token in forbidden:
        if token in adapter:
            errors.append(f"Codex Adapterに禁止された実行境界tokenがある: {token}")
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
        "MethodChannel(",
    ]
    errors = []
    for path in sorted((DESKTOP_FLUTTER / "lib").rglob("*.dart")):
        text = path.read_text(encoding="utf-8")
        for token in forbidden:
            if token in text:
                errors.append(f"{path.relative_to(ROOT)} が禁止されたruntime bridge tokenを含む: {token}")
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
    if "factory ShellCoreClient.local() => ShellCoreClient.mock()" in client:
        errors.append("desktop Flutterのlocal clientが依然としてdirect mock aliasである")
    if "ShellSnapshot.fromJson" not in client:
        errors.append("desktop Flutterのlocal clientが構造化snapshot JSONをloadしない")
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
        "themeMode: ThemeMode.system",
        "darkTheme: _buildShellTheme(Brightness.dark)",
        "kGuiShellProductTitle",
    ]:
        if token not in main:
            errors.append(f"desktop Flutterのproduct baselineにtokenがない: {token}")
    if 'window.Create(L"GUI Shell", origin, size)' not in windows_main:
        errors.append("Windows runnerのproduct window titleがGUI Shellではない")
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
    required_terms = ["device_id", "pairing_id", "操作者確認", "監査事象", "取消し", "復旧経路"]
    combined = "\n".join(path.read_text(encoding="utf-8") for path in sorted((MOBILE_FLUTTER / "lib").glob("**/*.dart")))
    errors = []
    for term in required_terms:
        if term not in combined:
            errors.append(f"mobile pairing contractのtermがない: {term}")
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


def test_agent_adapter_probe_is_read_only_and_fail_closed() -> list[str]:
    adapter = build_adapter_record("codex", "0.155.0-alpha.16", True, True)
    schema = load_schema("agent_adapter.schema.json")
    if validate_instance(adapter, schema):
        return ["Agent CLI probeのfixture結果がAgent Adapter Schemaに適合しない"]
    if adapter["status"] != "degraded" or adapter["evidence_source"] != "LIVE_RUNTIME":
        return ["interface確認済みAgentをBroker dispatch停止中のdegradedとして表現しなかった"]
    if adapter["authentication"]["secret_value_present"] is not False:
        return ["Agent CLI probeがsecret実値の存在を許可した"]
    unavailable = build_adapter_record(None, "unknown", False, False)
    if unavailable["status"] != "unavailable" or unavailable["evidence_source"] != "CONFIG":
        return ["未導入Agent CLIを利用可能として扱った"]
    return []


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
    return 不整合


def test_setup_doctor_public_bind_warning_exists() -> list[str]:
    from installer.setup_doctor import setup_doctor_report

    report = setup_doctor_report()
    matches = [check for check in report["checks"] if check["check_id"] == "network.public_bind"]
    if not matches or matches[0].get("status") != "warning" or not matches[0].get("recovery_action"):
        return ["Setup Doctorのpublic bind warningがない"]
    return []


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


def main() -> int:
    tests = [
        test_required_docs_exist,
        test_gui_shell_spec_v1_declares_core_boundaries,
        test_contract_fixtures_are_available,
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
        test_windows_release_evidence_validator_rejects_external_setup_probe_as_product_evidence,
        test_windows_release_evidence_validator_rejects_unmeasured_or_synthetic_evidence,
        test_windows_release_evidence_validator_rejects_broker_top_level_unmeasured_declarations,
        test_windows_japanese_surface_labels,
        test_windows_surface_geometry_and_identity,
        test_windows_release_evidence_validator_rejects_missing_surface_matches,
        test_windows_release_evidence_validator_rejects_screenshot_surface_source,
        test_windows_release_evidence_validator_rejects_flutter_build_registry_as_visibility,
        test_windows_release_evidence_validator_rejects_aggregate_surface_root_match,
        test_installed_app_setup_doctor_product_export_contract_exists,
        test_windows_stage_installer_powershell_boolean_grouping,
        test_windows_installed_smoke_preserves_trap_failure,
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
        test_rust_broker_skeleton_exists,
        test_rust_broker_rejection_audit_contract_shape,
        test_rust_broker_audit_anchor_and_nonce_compaction_present,
        test_rust_filesystem_diagnostic_detects_secret_symlink,
        test_desktop_flutter_does_not_spawn_python_or_use_ffi_authority_bridge,
        test_desktop_flutter_windows_runner_rejects_native_surface_aggregate_injection,
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
        test_packaging_portability_utf8_governance_allowlist_is_exact,
        書庫展開で日本語名と内容を保持する,
        対話契約の関係と表示境界を検査する,
        対話操作の分岐と未知fieldを検査する,
        履歴入力概要のhash_only境界を検査する,
        実行系資源観測の証拠境界を検査する,
        実行系ライフサイクルの契約と統治境界を検査する,
        端末契約の構造と禁止操作を検査する,
        二実行系比較の非混線を検査する,
        評価ラボの契約と境界を検査する,
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
        test_agent_secret_path_read_default_deny,
        test_agent_secret_path_symlink_default_deny,
        test_agent_shell_command_requires_permission_mapping,
        test_agent_git_push_requires_explicit_approval,
        test_agent_generated_diff_must_be_auditable,
        test_agent_auto_permission_is_advisory_only,
        test_agent_adapter_is_declaration_only_and_unsupported_is_explicit,
        test_agent_adapter_probe_is_read_only_and_fail_closed,
        test_l3_bounded_reference_extension_uses_existing_contracts,
        test_l3_bounded_reference_extension_governed_path_accepts_declared_mapping,
        test_l3_bounded_reference_extension_negative_cases_fail_closed,
        test_audit_chain_verification_fails_on_tampered_event,
        test_audit_chain_rejects_duplicate_event_ids,
        test_json_persistence_reports_corrupt_audit_jsonl,
        test_platform_hardening_configuration_exists,
        手動補助の起動境界を検査する,
        test_setup_doctor_public_bind_warning_exists,
        test_broker_parity_startup_timeout_allows_local_cold_build,
        test_broker_parity_waits_after_process_kill,
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
