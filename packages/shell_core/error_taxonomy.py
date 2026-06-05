from dataclasses import dataclass


UNKNOWN_RUNTIME = "unknown_runtime"
UNKNOWN_CAPABILITY = "unknown_capability"
UNKNOWN_PERMISSION = "unknown_permission"
PERMISSION_DENIED = "permission_denied"
RELATION_MISMATCH = "relation_mismatch"
PAYLOAD_HASH_MISMATCH = "payload_hash_mismatch"
APPROVAL_MISSING = "approval_missing"
APPROVAL_NOT_VALID = "approval_not_valid"
AUDIT_MAPPING_MISSING = "audit_mapping_missing"
RECOVERY_MAPPING_MISSING = "recovery_mapping_missing"
CONTENT_VISIBILITY_VIOLATION = "content_visibility_violation"
ADAPTER_METADATA_ESCALATION_ATTEMPT = "adapter_metadata_escalation_attempt"
NON_AUTHORITY_SOURCE_ATTEMPT = "non_authority_source_attempt"
UPDATE_SIGNATURE_REQUIRED = "update_signature_required"
SCHEMA_CONTRACT_MISSING = "schema_contract_missing"


RECOVERY_HINTS = {
    UNKNOWN_RUNTIME: "Register the runtime before routing this operation.",
    UNKNOWN_CAPABILITY: "Register the capability in Shell Core before use.",
    UNKNOWN_PERMISSION: "Record an explicit permission decision before use.",
    PERMISSION_DENIED: "Request or grant permission through an authority source.",
    RELATION_MISMATCH: "Resolve runtime, capability, permission, approval, recovery, and target scope from one broker-owned action relation.",
    PAYLOAD_HASH_MISMATCH: "Recompute the canonical payload hash and bind approval/audit to the submitted payload.",
    APPROVAL_MISSING: "Create an approval and wait for an approved state.",
    APPROVAL_NOT_VALID: "Revalidate or approve the current approval request.",
    AUDIT_MAPPING_MISSING: "Attach an AuditEvent with an event_id and required payload_hash.",
    RECOVERY_MAPPING_MISSING: "Attach a RecoveryAction with a recovery_id.",
    CONTENT_VISIBILITY_VIOLATION: "Project approval content according to content_visibility.",
    ADAPTER_METADATA_ESCALATION_ATTEMPT: "Remove authority claims from adapter metadata.",
    NON_AUTHORITY_SOURCE_ATTEMPT: "Use an authority source; memory, cache, previous_state, and local_ui_state cannot grant authority.",
    UPDATE_SIGNATURE_REQUIRED: "Require a valid update signature before update operations.",
    SCHEMA_CONTRACT_MISSING: "Load the required schema contract before validation.",
}


@dataclass(frozen=True)
class ShellCoreError:
    code: str
    message: str
    operation: str
    recoverable: bool
    recovery_hint: str | None = None

    def to_dict(self) -> dict:
        result = {
            "code": self.code,
            "message": self.message,
            "operation": self.operation,
            "recoverable": self.recoverable,
        }
        if self.recovery_hint:
            result["recovery_hint"] = self.recovery_hint
        return result


def shell_error(
    code: str,
    message: str,
    operation: str,
    *,
    recoverable: bool = True,
    recovery_hint: str | None = None,
) -> dict:
    return ShellCoreError(
        code=code,
        message=message,
        operation=operation,
        recoverable=recoverable,
        recovery_hint=recovery_hint if recovery_hint is not None else RECOVERY_HINTS.get(code),
    ).to_dict()
