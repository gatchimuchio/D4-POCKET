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
from packages.shell_core.permission_ledger import NON_AUTHORITY_SOURCES, PermissionLedger
from packages.shell_core.policy_evaluator import PolicyEvaluator
from packages.shell_core.runtime_state import RuntimeState
from packages.shell_core.release_smoke import run_shell_core_release_smoke
from packages.shell_core.sensitive_action_router import SensitiveActionRouter
from packages.shell_core.state_snapshot import create_state_snapshot, deterministic_snapshot_json
from packages.blue_tanuki_adapter.adapter import BlueTanukiAdapter
from packages.blue_tanuki_adapter.approvals import normalize_approval, projected_approval
from packages.blue_tanuki_adapter.authority_trace import metadata_attempts_authority
from packages.blue_tanuki_adapter.recovery import recovery_candidates
from packages.agent_runtime import AgentRuntimeContract
from packages.runtime_catalog import RuntimeCatalog
from packages.shell_core.audit_chain import chain_event, verify_audit_chain
from tooling.schema_check.check_schemas import validate_instance
from tooling.release_smoke import run_release_smokes
from tooling.evidence_bundle import build_evidence_bundle, validate_evidence_bundle
from tooling.manifest import build_manifest, matches_forbidden
from tooling.shell_snapshot import build_shell_snapshot
from tooling.validate_all import ValidationStep, build_steps as build_validation_steps, run_step
from tooling.windows_release_evidence import validate_windows_release_evidence
from tooling.broker_parity.run_authority_parity import DEFAULT_BROKER_START_TIMEOUT_SECONDS

REQUIRED_SCHEMA_NAMES = {
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
    # Adapter metadata is descriptive only. Permission-like metadata must be ignored.
    return list(adapter.get("declared_capabilities", []))


def can_create_authority_context(source: str, runtime_allowed: bool) -> bool:
    return source == "runtime" and runtime_allowed


def source_can_grant_authority(source: str) -> bool:
    return source not in NON_AUTHORITY_SOURCES


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
    raise ValueError(f"unknown content visibility: {visibility}")


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
        "adapter-conformance.md",
        "agent-runtime.md",
        "authority-strip-conformance.md",
        "content-exposure-policy.md",
        "approval-visibility-boundary.md",
        "runtime-catalog.md",
    }
    existing = {path.name for path in DOC_SPECS.glob("*.md")}
    for missing in sorted(required_docs - existing):
        errors.append(f"docs/specs/{missing} missing")
    return errors


def test_contract_fixtures_are_available() -> list[str]:
    errors = []
    expected = {f"{name}.valid.json" for name in REQUIRED_SCHEMA_NAMES}
    existing = {path.name for path in CONTRACT_EXAMPLES.glob("*.valid.json")}
    for missing in sorted(expected - existing):
        errors.append(f"examples/contracts/{missing} missing")
    for name in sorted(expected & existing):
        try:
            fixture = load_contract_fixture(name)
        except Exception as exc:
            errors.append(f"examples/contracts/{name} did not parse: {exc}")
            continue
        if not isinstance(fixture, dict):
            errors.append(f"examples/contracts/{name} must be a JSON object")
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
        errors.append(f"examples/contracts/invalid missing negative fixture for {missing}")
    if len(invalid_paths) < len(REQUIRED_SCHEMA_NAMES):
        errors.append("negative contract fixtures must cover every schema")
    return errors


def test_adapter_authority_strip_schema() -> list[str]:
    errors = []
    schema = load_schema("adapter.schema.json")
    required = set(schema.get("required", []))
    authority_strip = schema["properties"].get("authority_strip", {})
    if "authority_strip" not in required:
        errors.append("adapter.schema.json must require authority_strip")
    if authority_strip.get("const") is not True:
        errors.append("adapter.schema.json must require authority_strip=true")
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
            errors.append(f"inbound authority key was not stripped: {key}")
    if stripped["payload"]["nested"].get("value") != 1:
        errors.append("authority strip removed safe nested payload")
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
            errors.append(f"authority value survived adapter metadata strip: {forbidden}")
    for key in AUTHORITY_KEYS:
        if f'"{key}"' in encoded:
            errors.append(f"authority key survived adapter metadata strip: {key}")
    if record.metadata != {"safe_label": "reference"}:
        errors.append("adapter metadata strip removed safe metadata or left authority metadata")
    if record.effective_capabilities() != ("filesystem.read",):
        errors.append("adapter metadata changed effective capabilities")
    return errors


def test_adapter_loader_rejects_value_only_authority_metadata() -> list[str]:
    adapter = load_contract_fixture("adapter.valid.json")
    adapter["metadata"] = {"safe_label": "root"}
    try:
        load_adapter(adapter)
    except ValueError:
        return []
    return ["adapter loader accepted value-only authority metadata"]


def test_runtime_state_adapter_registration_uses_loader_boundary() -> list[str]:
    state = RuntimeState()
    adapter = load_contract_fixture("adapter.valid.json")
    adapter["metadata"] = {"permissionGrant": "all", "safe_label": "reference"}
    state.register_adapter(adapter)
    stored = state.adapters.get(adapter["adapter_id"], {})
    errors = []
    if "permission_grant" in json.dumps(stored.get("metadata", {}), sort_keys=True):
        errors.append("RuntimeState.register_adapter stored stripped authority metadata")
    if stored.get("metadata") != {"safe_label": "reference"}:
        errors.append("RuntimeState.register_adapter did not preserve safe stripped metadata")
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
            errors.append(f"normalization did not canonicalize key: {key}")
    if normalized["quarantined"] is not True:
        errors.append("normalization firewall did not quarantine authority-bearing payload")
    if not normalized["audit_event"].get("raw_payload_preserved"):
        errors.append("normalization firewall did not preserve raw payload for audit")
    stripped = json.dumps(normalized["stripped_payload"], sort_keys=True)
    for forbidden in ["trust_level", "permission_grant", "admin_context", "authority"]:
        if forbidden in stripped:
            errors.append(f"authority key survived normalization strip: {forbidden}")
    return errors


def test_normalization_firewall_detects_value_only_escalation() -> list[str]:
    payload = {"metadata": {"label": "root", "safe_note": "operator visible"}}
    normalized = normalize_inbound_payload(payload)
    errors = []
    if normalized["quarantined"] is not True:
        errors.append("normalization firewall did not quarantine value-only authority attempt")
    if not normalized["authority_value_findings"]:
        errors.append("normalization firewall did not record authority value finding")
    if normalized["stripped_payload"].get("metadata", {}).get("safe_note") != "operator visible":
        errors.append("normalization firewall removed safe metadata while detecting value-only escalation")
    return errors


def test_normalization_firewall_detects_key_collisions() -> list[str]:
    normalized = normalize_inbound_payload({"safeLabel": "first", "safe_label": "second"})
    errors = []
    if normalized["quarantined"] is not True:
        errors.append("normalization firewall did not quarantine normalized key collision")
    if not normalized.get("normalization_collision_findings"):
        errors.append("normalization firewall did not record normalized key collision")
    if normalized["audit_event"].get("normalization_collision_count") != 1:
        errors.append("normalization firewall did not count normalized key collision")
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
        return ["adapter metadata escalated effective permissions"]
    return []


def test_gui_input_cannot_create_runtime_disallowed_authority_context() -> list[str]:
    errors = []
    if can_create_authority_context("gui", runtime_allowed=True):
        errors.append("GUI input created authority context")
    if can_create_authority_context("adapter", runtime_allowed=True):
        errors.append("adapter input created authority context directly")
    if can_create_authority_context("runtime", runtime_allowed=False):
        errors.append("runtime-disallowed authority context was created")
    if not can_create_authority_context("runtime", runtime_allowed=True):
        errors.append("runtime-allowed authority context was denied")
    return errors


def test_memory_cache_previous_state_cannot_grant_authority() -> list[str]:
    errors = []
    for source in sorted(NON_AUTHORITY_SOURCES):
        if source_can_grant_authority(source):
            errors.append(f"{source} granted authority")
    return errors


def test_content_exposure_contract() -> list[str]:
    schema = load_schema("content_exposure.schema.json")
    fixture = load_contract_fixture("content_exposure.valid.json")
    errors = []
    default_visibility = schema["properties"]["default_visibility"]
    if default_visibility.get("const") != "none":
        errors.append("content exposure default_visibility must be const none")
    if fixture.get("default_visibility") != "none":
        errors.append("content exposure valid fixture default_visibility must be none")
    enum = schema["properties"]["allowed_visibility"]["items"]["enum"]
    if enum != VISIBILITY_VALUES:
        errors.append("content exposure allowed_visibility enum must match locked order")
    return errors


def test_full_content_only_visible_when_full() -> list[str]:
    base = load_contract_fixture("approval.valid.json")
    errors = []
    for visibility in VISIBILITY_VALUES:
        rendered = render_approval_content({**base, "content_visibility": visibility})
        if visibility != "full" and "full_payload" in rendered:
            errors.append(f"full payload rendered for content_visibility={visibility}")
        if visibility == "hash_only" and set(rendered) != {"payload_hash"}:
            errors.append("hash_only rendered more than payload_hash")
        if visibility == "none" and rendered:
            errors.append("none visibility rendered content")
    return errors


def test_approval_schema_has_protected_field_sets() -> list[str]:
    schema = load_schema("approval.schema.json")
    properties = schema.get("properties", {})
    errors = []
    for field in ["authority_fields", "sealed_fields", "hidden_fields", "sacred_fields"]:
        if field not in properties:
            errors.append(f"approval.schema.json missing {field}")
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
            errors.append(f"protected approval field was editable: {field}")
        before = queue.get(approval["approval_id"])
        try:
            queue.edit(approval["approval_id"], field, "mutated")
        except ValueError:
            pass
        else:
            errors.append(f"protected approval field was written: {field}")
        after = queue.get(approval["approval_id"])
        if after != before:
            errors.append(f"protected approval mutation changed queued approval: {field}")
    if not queue.can_edit(approval["approval_id"], "path"):
        errors.append("allowed non-protected approval field was not editable")
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
        errors.append("approval edit did not change payload_hash")
    if edited["payload_hash"] != canonical_hash({"allowed_note": "after"}):
        errors.append("approval edit payload_hash was not canonical")
    if edited["status"] != "requires_validation":
        errors.append("approval edit did not require revalidation")
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
        errors.append("complete sensitive action mapping was rejected")
    if sensitive_action_mapping_is_complete(incomplete):
        errors.append("sensitive action mapping passed without RecoveryAction")
    return errors


def test_hash_patterns_are_tagged_sha256() -> list[str]:
    errors = []
    sample = sha256_tagged(b"approval")
    if not sample.startswith("sha256:") or len(sample) != 71:
        errors.append("sha256_tagged helper invariant failed")
    for schema_name in ["approval.schema.json", "audit.schema.json"]:
        schema = load_schema(schema_name)
        pattern = schema["properties"]["payload_hash"].get("pattern", "")
        if "sha256:" not in pattern:
            errors.append(f"{schema_name} payload_hash must use tagged sha256 pattern")
    return errors


def test_framework_risk_profile_exists() -> list[str]:
    path = SPECS / "framework_risk_profile.schema.json"
    if not path.exists():
        return ["framework_risk_profile.schema.json missing"]
    return []


def test_update_fixture_requires_signature() -> list[str]:
    update = load_contract_fixture("update.valid.json")
    if update.get("signature_required") is not True:
        return ["update valid fixture does not require signatures"]
    return []


def test_shell_contracts_load_required_schemas() -> list[str]:
    catalog = load_default_catalog()
    expected = {f"{name}.schema.json" for name in REQUIRED_SCHEMA_NAMES}
    loaded = set(catalog.names())
    missing = sorted(expected - loaded)
    if missing:
        return [f"shell_contracts catalog missing schema: {name}" for name in missing]
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
        return ["Shell Core trusted adapter metadata for permissions"]
    return []


def test_shell_core_non_authority_sources_do_not_grant_authority() -> list[str]:
    ledger = PermissionLedger()
    errors = []
    for source in sorted(NON_AUTHORITY_SOURCES):
        if ledger.can_grant_authority_from_source(source):
            errors.append(f"Shell Core treated {source} as authority")
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
        errors.append("Shell Core routed sensitive action without evaluator state")
    if "schema_contract_missing" not in error_codes(routed.get("policy_result", {})):
        errors.append("Shell Core did not expose fail-closed missing evaluator result")
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
    errors.append("Shell Core routed sensitive action without RecoveryAction")
    return errors


def test_shell_core_content_projection_hides_full_payload_until_full() -> list[str]:
    approval = load_contract_fixture("approval.valid.json")
    errors = []
    for visibility in VISIBILITY_VALUES:
        projected = project_approval_content({**approval, "content_visibility": visibility})
        if visibility != "full" and "full_payload" in projected:
            errors.append(f"Shell Core projected full_payload for {visibility}")
    return errors


def test_shell_core_has_no_flutter_imports() -> list[str]:
    errors = []
    for path in sorted(SHELL_CORE.glob("*.py")):
        text = path.read_text(encoding="utf-8")
        for line_number, line in enumerate(text.splitlines(), start=1):
            normalized = line.strip().lower()
            if normalized.startswith("import flutter") or normalized.startswith("from flutter"):
                errors.append(f"{path}:{line_number} imports Flutter")
    return errors


def test_shell_core_has_no_blue_tanuki_internal_imports() -> list[str]:
    errors = []
    for path in sorted(SHELL_CORE.glob("*.py")):
        text = path.read_text(encoding="utf-8")
        for line_number, line in enumerate(text.splitlines(), start=1):
            normalized = line.strip().lower()
            if normalized.startswith("import blue_tanuki") or normalized.startswith("from blue_tanuki"):
                errors.append(f"{path}:{line_number} imports BLUE-TANUKI internals")
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
        return ["PolicyEvaluator did not reject unknown capability"]
    return []


def test_policy_evaluator_returns_structured_errors() -> list[str]:
    state = build_policy_state()
    action = build_sensitive_action()
    action["capability_id"] = "unknown.capability"
    result = PolicyEvaluator(state).evaluate(action)
    errors = []
    required = {"code", "message", "operation", "recoverable"}
    if not result["errors"]:
        return ["PolicyEvaluator returned no error for invalid action"]
    for error in result["errors"]:
        missing = sorted(required - set(error))
        if missing:
            errors.append(f"PolicyEvaluator error missing fields: {', '.join(missing)}")
        if error.get("code") == "unknown_capability" and "recovery_hint" not in error:
            errors.append("PolicyEvaluator recoverable error missing recovery_hint")
    return errors


def test_policy_evaluator_rejects_unknown_permission() -> list[str]:
    state = build_policy_state()
    action = build_sensitive_action()
    action["permission_id"] = "unknown.permission"
    result = PolicyEvaluator(state).evaluate(action)
    if result["allowed"] or "unknown_permission" not in error_codes(result):
        return ["PolicyEvaluator did not reject unknown permission"]
    return []


def test_policy_evaluator_rejects_denied_permission() -> list[str]:
    state = build_policy_state(permission_decision="deny")
    result = PolicyEvaluator(state).evaluate(build_sensitive_action())
    if result["allowed"] or "permission_denied" not in error_codes(result):
        return ["PolicyEvaluator did not reject denied permission"]
    return []


def test_policy_evaluator_rejects_missing_approval() -> list[str]:
    state = build_policy_state()
    action = build_sensitive_action()
    action.pop("approval_state")
    action.pop("approval_id")
    result = PolicyEvaluator(state).evaluate(action)
    if result["allowed"] or "approval_missing" not in error_codes(result):
        return ["PolicyEvaluator did not reject missing approval"]
    return []


def test_policy_evaluator_rejects_self_reported_approval_without_approval_id() -> list[str]:
    state = build_policy_state()
    action = build_sensitive_action()
    action.pop("approval_id")
    action["approval_state"] = "approved"
    result = PolicyEvaluator(state).evaluate(action)
    if result["allowed"] or "approval_missing" not in error_codes(result):
        return ["PolicyEvaluator trusted self-reported approval_state without approval_id"]
    return []


def test_policy_evaluator_rejects_unknown_approval_id() -> list[str]:
    state = build_policy_state()
    action = build_sensitive_action()
    action["approval_id"] = "approval-does-not-exist"
    action["approval_state"] = "approved"
    result = PolicyEvaluator(state).evaluate(action)
    if result["allowed"] or "approval_missing" not in error_codes(result):
        return ["PolicyEvaluator accepted unknown approval_id"]
    return []


def test_policy_evaluator_uses_runtime_state_approval_status() -> list[str]:
    state = build_policy_state(approval_status="approved")
    action = build_sensitive_action()
    action["approval_state"] = "pending"
    result = PolicyEvaluator(state).evaluate(action)
    if not result["allowed"]:
        return ["PolicyEvaluator rejected approved RuntimeState approval because action self-report differed"]
    return []


def test_policy_evaluator_rejects_unapproved_runtime_state_approval() -> list[str]:
    state = build_policy_state(approval_status="requires_validation")
    action = build_sensitive_action()
    action["approval_state"] = "approved"
    result = PolicyEvaluator(state).evaluate(action)
    if result["allowed"] or "approval_not_valid" not in error_codes(result):
        return ["PolicyEvaluator trusted action approval_state over RuntimeState approval status"]
    return []


def test_policy_evaluator_rejects_missing_audit_event() -> list[str]:
    state = build_policy_state()
    action = build_sensitive_action()
    action.pop("audit_event")
    result = PolicyEvaluator(state).evaluate(action)
    if result["allowed"] or "audit_mapping_missing" not in error_codes(result):
        return ["PolicyEvaluator did not reject missing audit event"]
    return []


def test_policy_evaluator_rejects_missing_recovery_action() -> list[str]:
    state = build_policy_state()
    action = build_sensitive_action()
    action.pop("recovery_action")
    result = PolicyEvaluator(state).evaluate(action)
    if result["allowed"] or "recovery_mapping_missing" not in error_codes(result):
        return ["PolicyEvaluator did not reject missing recovery action"]
    return []


def test_policy_evaluator_rejects_unknown_recovery_id() -> list[str]:
    state = build_policy_state()
    action = build_sensitive_action()
    action["recovery_action"] = {"recovery_id": "recover-does-not-exist"}
    result = PolicyEvaluator(state).evaluate(action)
    if result["allowed"] or "recovery_mapping_missing" not in error_codes(result):
        return ["PolicyEvaluator accepted unknown recovery_id"]
    return []


def test_policy_evaluator_accepts_known_recovery_id() -> list[str]:
    state = build_policy_state()
    result = PolicyEvaluator(state).evaluate(build_sensitive_action())
    if not result["allowed"]:
        return ["PolicyEvaluator rejected known recovery_id with otherwise valid action"]
    return []


def test_policy_evaluator_ignores_adapter_metadata_authority() -> list[str]:
    state = build_policy_state()
    action = build_sensitive_action()
    action["adapter_metadata"] = {"permissions": ["filesystem.write"], "trust_level": "root"}
    result = PolicyEvaluator(state).evaluate(action)
    if "adapter_metadata_escalation_attempt" not in error_codes(result):
        return ["PolicyEvaluator did not flag adapter metadata authority claims"]
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
            errors.append(f"PolicyEvaluator did not normalize and reject adapter metadata: {metadata}")
    return errors


def test_policy_evaluator_rejects_non_authority_source() -> list[str]:
    state = build_policy_state()
    errors = []
    for source in sorted(NON_AUTHORITY_SOURCES):
        action = build_sensitive_action()
        action["authority_source"] = source
        result = PolicyEvaluator(state).evaluate(action)
        if result["allowed"] or "non_authority_source_attempt" not in error_codes(result):
            errors.append(f"PolicyEvaluator allowed non-authority source: {source}")
    return errors


def test_policy_evaluator_enforces_action_envelope_relations() -> list[str]:
    errors = []

    def assert_error(label: str, mutate, expected_code: str) -> None:
        state = build_policy_state(approval_status="approved")
        action = build_sensitive_action()
        mutate(state, action)
        result = PolicyEvaluator(state).evaluate(action)
        if result["allowed"] or expected_code not in error_codes(result):
            errors.append(f"PolicyEvaluator did not reject {label}: {result}")

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
        errors.append(f"PolicyEvaluator rejected valid ActionEnvelope relation: {allowed}")
    if not allowed.get("action_envelope"):
        errors.append("PolicyEvaluator did not return validated action_envelope")
    return errors


def test_sensitive_action_router_uses_policy_evaluator_when_state_is_provided() -> list[str]:
    state = build_policy_state()
    routed = SensitiveActionRouter(state).route(build_sensitive_action())
    errors = []
    if routed.get("routed") is not True:
        errors.append("policy-backed SensitiveActionRouter did not route allowed action")
    if routed.get("policy_result", {}).get("allowed") is not True:
        errors.append("policy-backed SensitiveActionRouter missing allowed policy result")
    return errors


def test_sensitive_action_router_blocks_policy_denied_action() -> list[str]:
    state = build_policy_state(permission_decision="deny")
    routed = SensitiveActionRouter(state).route(build_sensitive_action())
    errors = []
    if routed.get("routed") is not False:
        errors.append("policy-backed SensitiveActionRouter routed denied action")
    if "permission_denied" not in error_codes(routed.get("policy_result", {})):
        errors.append("policy-backed SensitiveActionRouter did not expose permission_denied")
    return errors


def test_state_snapshot_is_deterministic() -> list[str]:
    state = build_policy_state()
    first = deterministic_snapshot_json(state)
    second = deterministic_snapshot_json(state.clone())
    if first != second:
        return ["state snapshot was not deterministic"]
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
            errors.append(f"state snapshot invariant flag missing or not false: {flag}")
    return errors


def test_shell_core_integrated_release_smoke() -> list[str]:
    with tempfile.TemporaryDirectory() as tmp:
        result = run_shell_core_release_smoke(Path(tmp))
    if not result["ok"]:
        return [f"Shell Core release smoke failed: {error}" for error in result["errors"]]
    errors = []
    if result["snapshot_saved"] is not True:
        errors.append("Shell Core release smoke did not save snapshot")
    if result["audit_chain_verified"] is not True:
        errors.append("Shell Core release smoke did not verify audit chain")
    if result["tamper_detected"] is not True:
        errors.append("Shell Core release smoke did not detect tamper")
    if result["approval_revalidation_required"] is not True:
        errors.append("Shell Core release smoke did not require approval revalidation")
    if result["recovery_id_verified"] is not True:
        errors.append("Shell Core release smoke did not verify recovery mapping")
    return errors


def test_release_smoke_runs_first_run_and_setup_doctor() -> list[str]:
    result = run_release_smokes()
    if not result["ok"]:
        return [f"release smoke failed: {error}" for error in result["errors"]]
    first_run = result["first_run"]
    errors = []
    if first_run["config_created"] is not True:
        errors.append("first-run smoke did not create config")
    if first_run["audit_dir_writable"] is not True:
        errors.append("first-run smoke did not verify audit dir writability")
    if first_run["installer_grants_authority"] is not False:
        errors.append("first-run smoke grants authority")
    if first_run["installer_silently_approves_permissions"] is not False:
        errors.append("first-run smoke silently approves permissions")
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
            errors.append(f"shell snapshot missing GUI operation state: {key}")
    if snapshot.get("installer_grants_authority") is not False:
        errors.append("shell snapshot grants installer authority")
    if snapshot.get("installer_silently_approves_permissions") is not False:
        errors.append("shell snapshot silently approves installer permissions")
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
            return [f"shell snapshot generator failed: {result.stderr or result.stdout}"]
        if not output.exists():
            return ["shell snapshot generator did not write output"]
        snapshot = json.loads(output.read_text(encoding="utf-8"))
    errors = []
    if snapshot.get("phase_status", {}).get("phase_a_status") != "complete":
        errors.append("generated snapshot does not mark Phase A complete")
    if snapshot.get("phase_status", {}).get("phase_b_status") != "complete":
        errors.append("generated snapshot does not mark Phase B complete")
    if snapshot.get("operation_status", {}).get("release_state") != "not claimed":
        errors.append("generated snapshot claimed release readiness")
    if not any(problem.get("classification") == "release_blocker" for problem in snapshot.get("problems", [])):
        errors.append("generated snapshot lacks expected release blockers")
    if release_evidence.exists() != existed_before:
        errors.append("shell snapshot generator created or removed Windows release evidence")
    return errors


def test_evidence_bundle_is_development_classified_and_non_authoritative() -> list[str]:
    bundle = build_evidence_bundle()
    errors = validate_evidence_bundle(bundle)
    if bundle.get("release_ready") is not False:
        errors.append("evidence bundle claimed release readiness")
    if bundle.get("classification") != "development_evidence":
        errors.append("evidence bundle is not classified as development_evidence")
    if not bundle.get("blockers"):
        errors.append("evidence bundle did not preserve Windows installed-path blockers")
    if bundle.get("authority_boundary", {}).get("flutter_owns_authority") is not False:
        errors.append("evidence bundle made Flutter authoritative")
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
    return {
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
    }


def test_windows_release_evidence_validator_accepts_valid_installed_smoke() -> list[str]:
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "windows_installed_smoke.json"
        path.write_text(json.dumps(_valid_windows_installed_evidence()), encoding="utf-8")
        results = validate_windows_release_evidence(path)
    errors = []
    for result in results:
        if result.status != "passed":
            errors.append(f"{result.name} rejected valid Windows evidence: {result.reason}")
        if result.classification == "release_blocker":
            errors.append(f"{result.name} classified valid Windows evidence as release_blocker")
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
        return ["Windows evidence validator accepted missing provenance/isolation"]
    return []


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
        errors.append("Windows first-run evidence validator accepted missing installed path or installer authority")
    if result_by_name["windows_setup_doctor_smoke"].classification != "release_blocker":
        errors.append("Windows Setup Doctor evidence validator accepted authority-granting check")
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
        errors.append("Windows Setup Doctor validator accepted external probe as formal product evidence")
    if result_by_name["windows_evidence_provenance_isolation"].classification != "release_blocker":
        errors.append("Windows provenance validator accepted external Setup Doctor provenance as product export")
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
        errors.append("Windows first-run evidence validator accepted unmeasured/manual evidence")
    if result_by_name["windows_setup_doctor_smoke"].classification != "release_blocker":
        errors.append("Windows Setup Doctor evidence validator accepted synthetic or shallow evidence")
    if result_by_name["windows_broker_installed_smoke"].classification != "release_blocker":
        errors.append("Windows broker evidence validator accepted synthetic, replay-unsafe, or Python-required evidence")
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
        return ["Windows broker validator accepted top-level unmeasured authority declarations"]
    return []


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
        return ["Windows first-run evidence validator accepted missing per-surface UIAutomation matches"]
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
        return ["Windows first-run evidence validator accepted screenshot as strict visible-surface source"]
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
        return ["Windows first-run evidence validator accepted one aggregate root automation element"]
    return []


def test_validate_all_subprocess_start_failure_is_structured() -> list[str]:
    step = ValidationStep(
        "missing_executable_probe",
        ["gui-shell-definitely-missing-validator-command"],
        ROOT,
    )
    result = run_step(step, strict_release=True, desktop_platform="windows")
    errors = []
    if result.get("status") != "not_run":
        errors.append("validate_all run_step did not return not_run for subprocess start failure")
    if result.get("classification") != "release_blocker":
        errors.append("validate_all run_step did not classify subprocess start failure as release_blocker")
    if "FileNotFoundError" not in result.get("stderr", ""):
        errors.append("validate_all run_step did not preserve subprocess start failure stack trace")
    return errors


def test_invariant_evaluator_detects_intentional_import_violation() -> list[str]:
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        shell_core = root / "packages" / "shell_core"
        shell_core.mkdir(parents=True)
        (shell_core / "bad.py").write_text("import flutter\n", encoding="utf-8")
        if not InvariantEvaluator(root).shell_core_imports_forbidden("flutter"):
            return ["InvariantEvaluator did not detect forbidden Flutter import"]
    return []


def test_invariant_evaluator_detects_live_authority_invariants() -> list[str]:
    flags = InvariantEvaluator().evaluate()
    errors = []
    if flags["adapter_metadata_can_escalate_authority"]:
        errors.append("InvariantEvaluator measured adapter metadata authority escalation")
    if flags["memory_cache_previous_state_can_grant_authority"]:
        errors.append("InvariantEvaluator measured non-authority source authority grant")
    if flags["full_payload_projected_without_full_visibility"]:
        errors.append("InvariantEvaluator measured full payload projection without full visibility")
    if flags["installer_setup_state_can_grant_authority"]:
        errors.append("InvariantEvaluator measured installer/setup authority grant")
    if flags["mobile_device_state_can_grant_authority"]:
        errors.append("InvariantEvaluator measured mobile/device authority grant")
    return errors


def test_rust_helper_required_sources_exist() -> list[str]:
    existing = {path.name for path in (RUST_HELPER / "src").glob("*.rs")}
    errors = []
    for missing in sorted(RUST_HELPER_REQUIRED_SOURCES - existing):
        errors.append(f"native/rust_helper/src/{missing} missing")
    return errors


def test_rust_helper_contract_shape_exists() -> list[str]:
    lib_rs = (RUST_HELPER / "src" / "lib.rs").read_text(encoding="utf-8")
    errors = []
    for token in ["HelperResponse", "HelperError", "ok", "operation", "result", "diagnostics", "error"]:
        if token not in lib_rs:
            errors.append(f"Rust helper contract missing token: {token}")
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
                errors.append(f"{path} uses forbidden helper authority pattern: {pattern}")
    return errors


def test_broker_ipc_contract_schemas_exist() -> list[str]:
    existing = {path.name for path in SPECS.glob("*.schema.json")}
    errors = []
    for missing in sorted(BROKER_REQUIRED_SCHEMAS - existing):
        errors.append(f"broker IPC schema missing: {missing}")
    for name in sorted(BROKER_REQUIRED_SCHEMAS & existing):
        schema = load_schema(name)
        if schema.get("type") != "object":
            errors.append(f"{name} must define an object contract")
        if "additionalProperties" not in schema:
            errors.append(f"{name} must define additionalProperties policy")
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
            errors.append(f"{relative} missing")
            continue
        text = path.read_text(encoding="utf-8")
        for token in ["Rust Security Broker", "release_blocker", "FFI"]:
            if token not in text:
                errors.append(f"{relative} missing broker governance token: {token}")
    return errors


def test_rust_broker_skeleton_exists() -> list[str]:
    broker_src = RUST_HELPER / "src" / "broker"
    existing = {path.name for path in broker_src.glob("*.rs")}
    errors = []
    for missing in sorted(BROKER_REQUIRED_SOURCES - existing):
        errors.append(f"native/rust_helper/src/broker/{missing} missing")
    main_rs = RUST_HELPER / "src" / "main.rs"
    lib_rs = RUST_HELPER / "src" / "lib.rs"
    for path in [main_rs, lib_rs]:
        text = path.read_text(encoding="utf-8")
        if "#![forbid(unsafe_code)]" not in text:
            errors.append(f"{path.relative_to(ROOT)} must forbid unsafe code")
    return errors


def test_rust_broker_rejection_audit_contract_shape() -> list[str]:
    protocol_rs = (RUST_HELPER / "src" / "broker" / "protocol.rs").read_text(encoding="utf-8")
    audit_rs = (RUST_HELPER / "src" / "broker" / "audit.rs").read_text(encoding="utf-8")
    errors = []
    required_protocol_tokens = [
        "BrokerRequestEnvelope",
        "BrokerResponse",
        "BrokerStatus::Rejected",
        "BrokerStatus::Suspended",
        "from_json_str",
        "handle_json",
        "to_json_string",
        "serde_json::from_str",
        "broker_request_malformed",
        "broker_payload_hash_invalid",
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
            errors.append(f"broker protocol missing token: {token}")
    for token in ["BrokerAuditLog", "append", "previous_event_hash", "event_hash"]:
        if token not in audit_rs:
            errors.append(f"broker audit missing token: {token}")
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
                errors.append(f"{path.relative_to(ROOT)} contains forbidden runtime bridge token: {token}")
    return errors


def test_release_docs_declare_language_policy_runtime_blockers() -> list[str]:
    required_tokens = [
        "rust security broker",
        "release_blocker",
        "production ipc",
        "no-python-runtime",
        "no-ffi-authority",
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
        for token in required_tokens:
            if token not in text:
                errors.append(f"{relative} missing language-policy blocker token: {token}")
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
            errors.append(f"BLUE-TANUKI adapter {schema_name} validation failed: {failure}")
    return errors


def test_blue_tanuki_adapter_metadata_cannot_escalate_authority() -> list[str]:
    metadata = {"permissions": ["filesystem.write"], "trust_level": "root"}
    trace = BlueTanukiAdapter().authority_trace()
    errors = []
    if not metadata_attempts_authority(metadata):
        errors.append("BLUE-TANUKI adapter did not detect authority-like metadata")
    if trace.get("metadata_trusted") is not False:
        errors.append("BLUE-TANUKI adapter trusted metadata")
    if trace.get("adapter_can_grant_permission") is not False:
        errors.append("BLUE-TANUKI adapter can grant permission")
    return errors


def test_blue_tanuki_adapter_cannot_expose_full_payload_unless_visibility_full() -> list[str]:
    errors = []
    for visibility in ["none", "hash_only", "summary", "redacted"]:
        projected = projected_approval({"content_visibility": visibility})
        if "full_payload" in projected:
            errors.append(f"BLUE-TANUKI adapter exposed full payload for {visibility}")
    if "full_payload" not in projected_approval({"content_visibility": "full"}):
        errors.append("BLUE-TANUKI adapter did not expose full payload when visibility was full")
    return errors


def test_blue_tanuki_adapter_cannot_mark_approvals_approved_by_itself() -> list[str]:
    approval = normalize_approval({"status": "approved", "approved_by": "adapter", "adapter_approved": True})
    if approval["status"] == "approved":
        return ["BLUE-TANUKI adapter self-approved an approval"]
    return []


def test_blue_tanuki_adapter_failures_map_to_recovery_actions() -> list[str]:
    candidates = recovery_candidates("runtime_down")
    if not candidates:
        return ["BLUE-TANUKI adapter failure did not produce RecoveryAction candidates"]
    schema = load_schema("recovery.schema.json")
    errors = []
    for candidate in candidates:
        errors.extend(validate_instance(candidate, schema))
    return [f"BLUE-TANUKI adapter recovery validation failed: {error}" for error in errors]


def test_desktop_flutter_required_files_exist() -> list[str]:
    errors = []
    for relative in sorted(DESKTOP_FLUTTER_REQUIRED_FILES):
        if not (DESKTOP_FLUTTER / relative).exists():
            errors.append(f"apps/desktop_flutter/{relative} missing")
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
                errors.append(f"{path} contains forbidden UI authority pattern: {pattern}")
    client = (DESKTOP_FLUTTER / "lib" / "services" / "shell_core_client.dart").read_text(encoding="utf-8")
    if "full_payload_projected_without_full_visibility': false" not in client:
        errors.append("desktop Flutter mock client does not expose Shell Core invariant status")
    if "factory ShellCoreClient.local() => ShellCoreClient.mock()" in client:
        errors.append("desktop Flutter local client is still a direct mock alias")
    if "ShellSnapshot.fromJson" not in client:
        errors.append("desktop Flutter local client does not load structured snapshot JSON")
    return errors


def test_desktop_flutter_windows_runner_rejects_native_surface_aggregate_injection() -> list[str]:
    runner = DESKTOP_FLUTTER / "windows" / "runner" / "flutter_window.cpp"
    if not runner.exists():
        return ["Windows Flutter runner missing: apps/desktop_flutter/windows/runner/flutter_window.cpp"]
    text = runner.read_text(encoding="utf-8")
    required_labels = ["Dashboard", "NavigationRail", "Runtime Status", "Invariant Status"]
    aggregate = "GUI Shell Dashboard NavigationRail Runtime Status Invariant Status"
    errors = []
    if aggregate in text:
        errors.append("Windows runner contains forbidden aggregate native surface title")
    set_window_text_blocks = re.findall(r"SetWindowText\s*\([^;]*;", text, flags=re.DOTALL)
    for block in set_window_text_blocks:
        labels = [label for label in required_labels if label in block]
        if labels:
            errors.append(
                "Windows runner SetWindowText contains required surface labels: "
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
            errors.append(f"desktop Flutter surface semantics identifier missing for {label}")
        if label not in shared and label not in main:
            errors.append(f"desktop Flutter surface label missing: {label}")
    if "SurfaceSemantics(" not in main or "label: 'NavigationRail'" not in main:
        errors.append("NavigationRail is not exposed through SurfaceSemantics")
    if "bySemanticsIdentifier(surfaceSemanticsIdentifier(label))" not in widget_test:
        errors.append("desktop Flutter widget test does not verify per-surface semantics identifiers")
    return errors


def test_validate_all_uses_running_python_interpreter_for_python_steps() -> list[str]:
    errors = []
    steps = build_validation_steps(False, "windows", python_only=True)
    for step in steps:
        if step.command[0] != sys.executable:
            errors.append(f"{step.name} does not use sys.executable")
        if step.required_tool is not None:
            errors.append(f"{step.name} still declares a PATH Python tool requirement")
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
        "Adapter Catalog",
        "Permission Diff",
        "Problems Panel",
        "Evidence Center",
        "Command Palette",
        "copy event / export JSONL / verify chain",
        "pre_check",
        "ShellStatusBar",
    ]
    return [f"desktop Flutter operation surface missing: {token}" for token in required if token not in combined]


def test_installer_setup_doctor_reports_structured_status_without_authority() -> list[str]:
    from installer.setup_doctor import setup_doctor_report

    report = setup_doctor_report()
    errors = []
    for key in ["status", "checks", "installer_grants_authority", "installer_silently_approves_permissions"]:
        if key not in report:
            errors.append(f"Setup Doctor report missing {key}")
    if report.get("installer_grants_authority") is not False:
        errors.append("installer grants authority")
    if report.get("installer_silently_approves_permissions") is not False:
        errors.append("installer silently approves permissions")
    for check in report.get("checks", []):
        if check.get("grants_authority") is not False:
            errors.append(f"Setup Doctor check grants authority: {check.get('check_id')}")
        if check.get("status") in {"fail", "warning"} and not check.get("recovery_instruction"):
            errors.append(f"Setup Doctor check lacks recovery instruction: {check.get('check_id')}")
    return errors


def test_installer_boundary_docs_exist() -> list[str]:
    required = ["FIRST_RUN.md", "SETUP_DOCTOR.md", "INSTALLER_BOUNDARY.md"]
    errors = []
    for name in required:
        if not (ROOT / "docs" / name).exists():
            errors.append(f"docs/{name} missing")
    if not (INSTALLER / "setup_doctor.py").exists():
        errors.append("installer/setup_doctor.py missing")
    return errors


def test_mobile_flutter_required_files_exist() -> list[str]:
    errors = []
    for relative in sorted(MOBILE_FLUTTER_REQUIRED_FILES):
        if not (MOBILE_FLUTTER / relative).exists():
            errors.append(f"apps/mobile_flutter/{relative} missing")
    return errors


def test_mobile_flutter_cannot_create_hidden_authority() -> list[str]:
    required_terms = ["device_id", "pairing_id", "operator confirmation", "audit event", "revocation", "recovery path"]
    combined = "\n".join(path.read_text(encoding="utf-8") for path in sorted((MOBILE_FLUTTER / "lib").glob("**/*.dart")))
    errors = []
    for term in required_terms:
        if term not in combined:
            errors.append(f"mobile pairing contract term missing: {term}")
    forbidden = ["independent authority: true", "silently pair", "'full_payload'", "hidden payload available"]
    for pattern in forbidden:
        if pattern in combined:
            errors.append(f"mobile Flutter contains forbidden authority pattern: {pattern}")
    return errors


def test_release_hardening_files_exist() -> list[str]:
    errors = []
    for relative in sorted(RELEASE_HARDENING_FILES):
        if not (ROOT / relative).exists():
            errors.append(f"{relative} missing")
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
        return ["tooling/validate_all.py missing"]
    text = path.read_text(encoding="utf-8")
    errors = []
    for token in ["schema_check", "conformance_skeleton", "manifest_check", "release_gate_check", "rust_helper_cargo_test", "desktop_flutter_analyze", "desktop_flutter_test", "desktop_flutter_build_linux", "mobile_flutter_analyze", "strict-release", "desktop-platform", "windows", "linux", "macos"]:
        if token not in text:
            errors.append(f"validate_all.py missing validation token: {token}")
    return errors


def test_manifest_integrity_tooling_exists() -> list[str]:
    errors = []
    required_paths = {
        "AGENTS.md",
        "ROADMAP.md",
        "CONFORMANCE_REPORT.md",
        "COMPATIBILITY_MATRIX.md",
        "docs/LANGUAGE_POLICY.md",
        "tooling/manifest.py",
        "tooling/conformance_tests/run_conformance_skeleton.py",
    }
    manifest, manifest_errors = build_manifest()
    errors.extend(manifest_errors)
    listed = {entry["path"] for entry in manifest["files"]}
    for path in sorted(required_paths - listed):
        errors.append(f"manifest expected source missing from generated file list: {path}")

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
            errors.append(f"manifest forbidden path was accepted: {path}")

    allowed_paths = [
        "docs/LANGUAGE_POLICY.md",
        "packages/shell_core/runtime_state.py",
        "tooling/manifest.py",
    ]
    for path in allowed_paths:
        if matches_forbidden(path):
            errors.append(f"manifest source path was rejected: {path}")
    return errors


def test_claim_documents_do_not_contain_stale_phase_or_check_counts() -> list[str]:
    stale_patterns = ["23 checks", "49 checks", "51 checks", "53 checks", "55 checks", "Phase 0 / Phase 1"]
    errors = []
    for relative in sorted(CLAIM_REVIEW_FILES):
        text = (ROOT / relative).read_text(encoding="utf-8")
        for pattern in stale_patterns:
            if pattern in text:
                errors.append(f"{relative} contains stale claim text: {pattern}")
    return errors


def test_runtime_manifest_invalid_fixture_rejected() -> list[str]:
    schema = load_schema("runtime_manifest.schema.json")
    invalid = json.loads((INVALID_CONTRACT_EXAMPLES / "runtime_manifest_unsigned.invalid.json").read_text(encoding="utf-8"))
    if not validate_instance(invalid, schema):
        return ["runtime manifest invalid fixture was accepted"]
    return []


def test_adapter_manifest_authority_escalation_rejected() -> list[str]:
    schema = load_schema("adapter_manifest.schema.json")
    invalid = json.loads((INVALID_CONTRACT_EXAMPLES / "adapter_manifest_authority_escalation.invalid.json").read_text(encoding="utf-8"))
    errors = []
    if not validate_instance(invalid, schema):
        errors.append("adapter manifest authority escalation fixture was accepted")
    catalog = RuntimeCatalog()
    if not catalog.metadata_attempts_authority(invalid.get("metadata", {})):
        errors.append("RuntimeCatalog did not detect adapter manifest metadata authority attempt")
    return errors


def test_runtime_catalog_cannot_grant_authority() -> list[str]:
    catalog = RuntimeCatalog()
    manifest = load_contract_fixture("runtime_manifest.valid.json")
    catalog.register_runtime_manifest(manifest)
    if catalog.can_grant_authority(manifest):
        return ["RuntimeCatalog granted authority from manifest"]
    return []


def test_agent_workspace_outside_access_default_deny() -> list[str]:
    workspace = load_contract_fixture("agent_workspace.valid.json")
    contract = AgentRuntimeContract(workspace)
    if contract.path_allowed("/outside/project/file.txt"):
        return ["agent runtime allowed access outside workspace by default"]
    return []


def test_agent_secret_path_read_default_deny() -> list[str]:
    workspace = load_contract_fixture("agent_workspace.valid.json")
    contract = AgentRuntimeContract(workspace)
    if contract.path_allowed("/workspace/project/.env"):
        return ["agent runtime allowed secret path read by default"]
    return []


def test_agent_shell_command_requires_permission_mapping() -> list[str]:
    workspace = load_contract_fixture("agent_workspace.valid.json")
    contract = AgentRuntimeContract(workspace)
    allowed = contract.shell_command_requires_permission(load_contract_fixture("agent_tool_call.valid.json"))
    denied = contract.shell_command_requires_permission({"tool_name": "shell.command"})
    if not allowed or denied:
        return ["agent shell command permission mapping check failed"]
    return []


def test_agent_git_push_requires_explicit_approval() -> list[str]:
    contract = AgentRuntimeContract(load_contract_fixture("agent_workspace.valid.json"))
    if not contract.git_push_requires_explicit_approval({"tool_name": "git.push", "permission_id": "permission.git.push", "approval_required": True}):
        return ["agent git push explicit approval was rejected"]
    if contract.git_push_requires_explicit_approval({"tool_name": "git.push", "permission_id": "permission.git.push", "approval_required": False}):
        return ["agent git push did not require explicit approval"]
    return []


def test_agent_generated_diff_must_be_auditable() -> list[str]:
    contract = AgentRuntimeContract(load_contract_fixture("agent_workspace.valid.json"))
    if not contract.diff_is_auditable(load_contract_fixture("agent_diff.valid.json")):
        return ["agent generated diff with audit evidence was rejected"]
    if contract.diff_is_auditable({"diff_id": "diff-1", "payload_hash": "sha256:" + "5" * 64}):
        return ["agent generated diff without audit was accepted"]
    return []


def test_agent_auto_permission_is_advisory_only() -> list[str]:
    contract = AgentRuntimeContract(load_contract_fixture("agent_workspace.valid.json"))
    if not contract.auto_permission_is_advisory_only(load_contract_fixture("agent_runtime.valid.json")):
        return ["agent advisory auto-permission mode was rejected"]
    if contract.auto_permission_is_advisory_only({"auto_permission_mode": "authority"}):
        return ["agent auto-permission authority mode was accepted"]
    return []


def load_bounded_extension_fixture() -> dict:
    return load_contract_fixture(BOUNDED_EXTENSION_FIXTURE)


def bounded_extension_validation_errors(extension: dict) -> list[str]:
    errors = []
    for key, schema_name in BOUNDED_EXTENSION_RECORD_SCHEMAS.items():
        record = extension.get(key)
        if not isinstance(record, dict):
            errors.append(f"bounded extension missing object record: {key}")
            continue
        for failure in validate_instance(record, load_schema(schema_name)):
            errors.append(f"bounded extension {key} failed {schema_name}: {failure}")

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
        errors.append("bounded extension must be classified as contract_conformance")
    required_non_claims = {
        "not_installed_product_evidence",
        "not_windows_release_evidence",
        "not_cross_agent_reproduction_evidence",
        "not_public_standard_adoption_evidence",
    }
    missing_non_claims = required_non_claims - set(extension.get("non_claims", []))
    if missing_non_claims:
        errors.append(f"bounded extension missing non-claims: {', '.join(sorted(missing_non_claims))}")

    if runtime_id != runtime_manifest.get("runtime_id"):
        errors.append("bounded extension runtime_manifest runtime_id does not match runtime")
    if runtime_id != adapter.get("runtime_id") or runtime_id != adapter_manifest.get("runtime_id"):
        errors.append("bounded extension adapter runtime_id does not match runtime")
    if runtime.get("adapter_id") != adapter_id or adapter_manifest.get("adapter_id") != adapter_id:
        errors.append("bounded extension adapter_id linkage is inconsistent")
    if runtime_manifest.get("runtime_type") != "tool_runtime" or runtime.get("kind") != "tool":
        errors.append("bounded extension reference must remain a tool runtime")
    if adapter.get("transport") != "mock" or adapter_manifest.get("transport") != "mock":
        errors.append("bounded extension reference must not require privileged transport")
    if adapter.get("authority_strip") is not True or adapter_manifest.get("authority_strip") is not True:
        errors.append("bounded extension adapter must require authority_strip=true")
    if runtime_manifest.get("signed_manifest") is not True or adapter_manifest.get("signed_manifest") is not True:
        errors.append("bounded extension manifests must be signed")

    for record_name, capabilities in {
        "runtime": runtime.get("capabilities", []),
        "runtime_manifest": runtime_manifest.get("capabilities", []),
        "adapter": adapter.get("declared_capabilities", []),
        "adapter_manifest": adapter_manifest.get("declared_capabilities", []),
    }.items():
        if capability_id not in capabilities:
            errors.append(f"bounded extension {record_name} does not declare capability {capability_id}")
    if permission_id not in runtime_manifest.get("permissions", []):
        errors.append("bounded extension runtime_manifest does not declare permission")
    if permission.get("capability_id") != capability_id:
        errors.append("bounded extension permission does not map to capability")
    if permission.get("source") != "policy":
        errors.append("bounded extension permission source must be policy, not runtime or metadata")
    if permission.get("decision") != "allow":
        errors.append("bounded extension positive fixture permission must be allow")

    if approval.get("runtime_id") != runtime_id or approval.get("operation") != capability_id:
        errors.append("bounded extension approval does not map to runtime capability")
    if approval.get("status") != "approved":
        errors.append("bounded extension positive fixture approval must be approved")
    if audit_event.get("payload_hash") != approval.get("payload_hash"):
        errors.append("bounded extension audit payload_hash does not match approval payload_hash")
    if audit_event.get("action") != capability_id or audit_event.get("target") != runtime_id:
        errors.append("bounded extension audit target/action does not map to runtime capability")
    if not recovery_action.get("recovery_id"):
        errors.append("bounded extension recovery mapping is missing")

    if content_policy.get("default_visibility") != "none":
        errors.append("bounded extension content exposure default must be none")
    if "full" in content_policy.get("allowed_visibility", []):
        errors.append("bounded extension content exposure policy must not allow full payload")
    if approval.get("content_visibility") not in content_policy.get("allowed_visibility", []):
        errors.append("bounded extension approval visibility is outside content policy")

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
        errors.append(f"bounded extension manifest registration failed: {exc}")
    if catalog.can_grant_authority(extension["runtime_manifest"]):
        errors.append("bounded extension runtime manifest granted authority")
    if catalog.metadata_attempts_authority(extension["adapter_manifest"].get("metadata", {})):
        errors.append("bounded extension adapter manifest metadata attempted authority")

    try:
        adapter_record = load_adapter(extension["adapter"])
    except ValueError as exc:
        errors.append(f"bounded extension adapter load failed: {exc}")
    else:
        if adapter_record.effective_capabilities() != tuple(extension["adapter"]["declared_capabilities"]):
            errors.append("bounded extension adapter metadata changed effective capabilities")

    marker = extension["runtime"]["runtime_id"]
    for path in sorted(SHELL_CORE.glob("*.py")):
        if marker in path.read_text(encoding="utf-8"):
            errors.append(f"bounded extension runtime-specific marker leaked into Shell Core: {path}")
    return errors


def test_l3_bounded_reference_extension_governed_path_accepts_declared_mapping() -> list[str]:
    extension = load_bounded_extension_fixture()
    state = build_bounded_extension_state(extension)
    result = PolicyEvaluator(state).evaluate(build_bounded_extension_action(extension))
    errors = []
    if not result["allowed"]:
        errors.append(f"bounded extension declared mapping was rejected: {result['errors']}")
    if result.get("audit_required") is not True:
        errors.append("bounded extension policy result did not require audit")

    projected = project_approval_content(extension["approval"])
    if "full_payload" in projected:
        errors.append("bounded extension projected full payload without full visibility")
    if "redacted_payload" not in projected:
        errors.append("bounded extension did not project the declared redacted payload")
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
            errors.append(f"bounded extension negative case did not fail closed for {label}: {result}")

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
        errors.append("bounded extension content policy allowed full visibility")
    if "full_payload" not in projected:
        errors.append("bounded extension full visibility mutation did not expose why policy rejection is required")
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
        errors.append("valid audit chain did not verify")
    if invalid["ok"] is not False:
        errors.append("tampered audit chain verified")
    return errors


def test_setup_doctor_public_bind_warning_exists() -> list[str]:
    from installer.setup_doctor import setup_doctor_report

    report = setup_doctor_report()
    matches = [check for check in report["checks"] if check["check_id"] == "network.public_bind"]
    if not matches or matches[0].get("status") != "warning" or not matches[0].get("recovery_action"):
        return ["Setup Doctor public bind warning missing"]
    return []


def test_broker_parity_startup_timeout_allows_ci_cold_build() -> list[str]:
    if DEFAULT_BROKER_START_TIMEOUT_SECONDS < 60.0:
        return ["broker parity startup timeout is too short for CI cold Rust builds"]
    return []


def test_desktop_agent_center_required_surface_exists() -> list[str]:
    path = DESKTOP_FLUTTER / "lib" / "screens" / "agent_center.dart"
    text = path.read_text(encoding="utf-8")
    required = [
        "Workspace",
        "Task",
        "Changed Files",
        "Tool Calls",
        "Shell Commands",
        "Test Status",
        "Diff Summary",
        "Pending Approvals",
        "Rollback Candidate",
        "Audit Link",
    ]
    return [f"Agent Center missing surface: {item}" for item in required if item not in text]


def main() -> int:
    tests = [
        test_required_docs_exist,
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
        test_shell_contracts_load_required_schemas,
        test_shell_core_ignores_adapter_metadata_permissions,
        test_shell_core_non_authority_sources_do_not_grant_authority,
        test_shell_core_routes_sensitive_actions_through_required_mapping,
        test_shell_core_content_projection_hides_full_payload_until_full,
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
        test_shell_core_integrated_release_smoke,
        test_release_smoke_runs_first_run_and_setup_doctor,
        test_shell_snapshot_contains_gui_operation_state,
        test_shell_snapshot_generator_writes_phase_b_local_snapshot,
        test_evidence_bundle_is_development_classified_and_non_authoritative,
        test_windows_release_evidence_validator_accepts_valid_installed_smoke,
        test_windows_release_evidence_validator_rejects_missing_provenance,
        test_windows_release_evidence_validator_rejects_authority_and_missing_installed_path,
        test_windows_release_evidence_validator_rejects_external_setup_probe_as_product_evidence,
        test_windows_release_evidence_validator_rejects_unmeasured_or_synthetic_evidence,
        test_windows_release_evidence_validator_rejects_broker_top_level_unmeasured_declarations,
        test_windows_release_evidence_validator_rejects_missing_surface_matches,
        test_windows_release_evidence_validator_rejects_screenshot_surface_source,
        test_windows_release_evidence_validator_rejects_aggregate_surface_root_match,
        test_invariant_evaluator_detects_intentional_import_violation,
        test_invariant_evaluator_detects_live_authority_invariants,
        test_rust_helper_required_sources_exist,
        test_rust_helper_contract_shape_exists,
        test_rust_helper_does_not_expose_hidden_authority_paths,
        test_broker_ipc_contract_schemas_exist,
        test_broker_boundary_docs_exist,
        test_rust_broker_skeleton_exists,
        test_rust_broker_rejection_audit_contract_shape,
        test_desktop_flutter_does_not_spawn_python_or_use_ffi_authority_bridge,
        test_desktop_flutter_windows_runner_rejects_native_surface_aggregate_injection,
        test_desktop_flutter_exposes_individual_surface_semantics_identifiers,
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
        test_validate_all_subprocess_start_failure_is_structured,
        test_manifest_integrity_tooling_exists,
        test_claim_documents_do_not_contain_stale_phase_or_check_counts,
        test_runtime_manifest_invalid_fixture_rejected,
        test_adapter_manifest_authority_escalation_rejected,
        test_runtime_catalog_cannot_grant_authority,
        test_agent_workspace_outside_access_default_deny,
        test_agent_secret_path_read_default_deny,
        test_agent_shell_command_requires_permission_mapping,
        test_agent_git_push_requires_explicit_approval,
        test_agent_generated_diff_must_be_auditable,
        test_agent_auto_permission_is_advisory_only,
        test_l3_bounded_reference_extension_uses_existing_contracts,
        test_l3_bounded_reference_extension_governed_path_accepts_declared_mapping,
        test_l3_bounded_reference_extension_negative_cases_fail_closed,
        test_audit_chain_verification_fails_on_tampered_event,
        test_setup_doctor_public_bind_warning_exists,
        test_broker_parity_startup_timeout_allows_ci_cold_build,
        test_desktop_agent_center_required_surface_exists,
    ]
    errors = []
    for test in tests:
        errors.extend(test())

    if errors:
        print("conformance skeleton failed:")
        for err in errors:
            print(f"  - {err}")
        return 1

    print(f"conformance skeleton passed: {len(tests)} checks")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
