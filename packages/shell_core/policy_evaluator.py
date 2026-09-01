from .error_taxonomy import (
    ADAPTER_METADATA_ESCALATION_ATTEMPT,
    APPROVAL_MISSING,
    APPROVAL_NOT_VALID,
    AUDIT_MAPPING_MISSING,
    NON_AUTHORITY_SOURCE_ATTEMPT,
    PAYLOAD_HASH_MISMATCH,
    PERMISSION_DENIED,
    RECOVERY_MAPPING_MISSING,
    RELATION_MISMATCH,
    UNKNOWN_CAPABILITY,
    UNKNOWN_PERMISSION,
    UNKNOWN_RUNTIME,
    shell_error,
)
from .approval_queue import canonical_hash
from .action_envelope import build_action_envelope
from .permission_ledger import NON_AUTHORITY_SOURCES
from .runtime_state import RuntimeState
from .normalization import authority_keys_in, authority_values_in, strip_authority_keys


ALLOWED_PERMISSION_DECISIONS = {"allow", "approved"}


class PolicyEvaluator:
    def __init__(self, state: RuntimeState):
        self.state = state

    def evaluate(self, action: dict) -> dict:
        operation = action.get("operation", "unknown")
        errors = []

        runtime_id = action.get("runtime_id")
        if not runtime_id:
            errors.append(shell_error(UNKNOWN_RUNTIME, "runtime_id は必須です", operation))
        elif runtime_id not in self.state.runtimes:
            errors.append(shell_error(UNKNOWN_RUNTIME, f"未知の runtime です: {runtime_id}", operation))

        capability_id = action.get("capability_id")
        capability = self.state.capabilities.get(capability_id)
        if capability is None:
            errors.append(shell_error(UNKNOWN_CAPABILITY, f"未知の capability です: {capability_id}", operation))
        else:
            if runtime_id and capability.get("runtime_id") != runtime_id:
                errors.append(
                    shell_error(
                        RELATION_MISMATCH,
                        "capability が要求された runtime に属していません",
                        operation,
                    )
                )
            operations = capability.get("operations")
            if isinstance(operations, list) and operation not in operations:
                errors.append(
                    shell_error(
                        RELATION_MISMATCH,
                        "capability が要求操作を許可していません",
                        operation,
                    )
                )

        permission_id = action.get("permission_id")
        permission = self.state.permissions.get(permission_id)
        if permission is None:
            errors.append(shell_error(UNKNOWN_PERMISSION, f"未知の permission です: {permission_id}", operation))
        elif permission.get("decision") not in ALLOWED_PERMISSION_DECISIONS:
            errors.append(
                shell_error(
                    PERMISSION_DENIED,
                    f"permission 判定が allow または approved ではありません: {permission.get('decision')}",
                    operation,
                )
            )

        if permission is not None:
            if capability_id is not None and permission.get("capability_id") != capability_id:
                errors.append(
                    shell_error(
                        PERMISSION_DENIED,
                        "permission が要求された capability を許可していません",
                        operation,
                    )
                )
            for field in ("runtime_id", "operation", "target_scope"):
                if action.get(field) != permission.get(field):
                    errors.append(
                        shell_error(
                            RELATION_MISMATCH,
                            f"permission {field} が要求 action と一致しません",
                            operation,
                        )
                    )

        approval_id = action.get("approval_id")
        approval = None
        if approval_id is None:
            errors.append(shell_error(APPROVAL_MISSING, "approval_id は必須です", operation))
        else:
            approval = self.state.approvals.get(approval_id)
            if approval is None:
                errors.append(shell_error(APPROVAL_MISSING, f"未知の approval です: {approval_id}", operation))
            elif approval.get("status") != "approved":
                errors.append(shell_error(APPROVAL_NOT_VALID, f"approval が approved ではありません: {approval_id}", operation))
            else:
                for field in ("runtime_id", "operation", "target_scope"):
                    if action.get(field) != approval.get(field):
                        errors.append(
                            shell_error(
                                RELATION_MISMATCH,
                                f"approval {field} が要求 action と一致しません",
                                operation,
                            )
                        )
                if self._has_payload(action):
                    expected_hash = canonical_hash(action.get("payload"))
                    if approval.get("payload_hash") != expected_hash:
                        errors.append(
                            shell_error(
                                PAYLOAD_HASH_MISMATCH,
                                "approval payload_hash が正規 action payload と一致しません",
                                operation,
                            )
                        )

        audit_event = action.get("audit_event")
        if not isinstance(audit_event, dict):
            errors.append(shell_error(AUDIT_MAPPING_MISSING, "audit_event は必須です", operation))
        else:
            if not audit_event.get("event_id"):
                errors.append(shell_error(AUDIT_MAPPING_MISSING, "audit_event.event_id は必須です", operation))
            if self._has_payload(action) and not audit_event.get("payload_hash"):
                errors.append(shell_error(AUDIT_MAPPING_MISSING, "payload がある場合は audit_event.payload_hash が必須です", operation))
            stored_audit = self.state.audit_events.get(audit_event.get("event_id"))
            if stored_audit is None:
                errors.append(shell_error(AUDIT_MAPPING_MISSING, "audit_event がbroker所有のaudit storeに存在しません", operation))
            else:
                expected_hash = canonical_hash(action.get("payload")) if self._has_payload(action) else audit_event.get("payload_hash")
                if audit_event.get("payload_hash") != expected_hash or stored_audit.get("payload_hash") != expected_hash:
                    errors.append(
                        shell_error(
                            PAYLOAD_HASH_MISMATCH,
                            "audit payload_hash が正規 action payload と一致しません",
                            operation,
                        )
                    )
                if stored_audit.get("action") not in {None, operation}:
                    errors.append(shell_error(RELATION_MISMATCH, "audit action が要求操作と一致しません", operation))

        recovery_action = action.get("recovery_action")
        if not isinstance(recovery_action, dict) or not recovery_action.get("recovery_id"):
            errors.append(shell_error(RECOVERY_MAPPING_MISSING, "recovery_action.recovery_id は必須です", operation))
        else:
            stored_recovery = self.state.recovery_actions.get(recovery_action["recovery_id"])
            if stored_recovery is None:
                errors.append(
                    shell_error(
                        RECOVERY_MAPPING_MISSING,
                        f"未知の recovery action です: {recovery_action['recovery_id']}",
                        operation,
                    )
                )
            else:
                for field in ("runtime_id", "operation"):
                    if stored_recovery.get(field) != action.get(field):
                        errors.append(
                            shell_error(
                                RELATION_MISMATCH,
                                f"recovery {field} が要求 action と一致しません",
                                operation,
                            )
                        )

        if self._metadata_claims_authority(action.get("adapter_metadata", {})):
            errors.append(
                shell_error(
                    ADAPTER_METADATA_ESCALATION_ATTEMPT,
                    "adapter metadata が権限を主張しようとしました",
                    operation,
                )
            )

        for source in sorted(NON_AUTHORITY_SOURCES):
            if action.get("authority_source") == source or action.get(f"{source}_grants_authority") is True:
                errors.append(shell_error(NON_AUTHORITY_SOURCE_ATTEMPT, f"{source} は権限を与えられません", operation))

        action_envelope = None
        if not errors:
            try:
                action_envelope = build_action_envelope(action)
            except (KeyError, TypeError, ValueError) as exc:
                errors.append(shell_error(RELATION_MISMATCH, f"action envelope が不正です: {exc}", operation))

        return {
            "allowed": not errors,
            "errors": errors,
            "required_recovery": recovery_action if errors and isinstance(recovery_action, dict) else None,
            "audit_required": True,
            "action_envelope": action_envelope,
        }

    def _has_payload(self, action: dict) -> bool:
        return any(key in action for key in ("payload", "full_payload", "redacted_payload"))

    def _metadata_claims_authority(self, value) -> bool:
        stripped = strip_authority_keys(value)
        return bool(authority_keys_in(value) or authority_values_in(stripped))
