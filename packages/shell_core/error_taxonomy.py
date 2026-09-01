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
    UNKNOWN_RUNTIME: "この操作を route する前に runtime を登録してください。",
    UNKNOWN_CAPABILITY: "利用前に capability を Shell Core へ登録してください。",
    UNKNOWN_PERMISSION: "利用前に明示的な permission 判定を記録してください。",
    PERMISSION_DENIED: "権限源を通じて permission を要求または付与してください。",
    RELATION_MISMATCH: "broker が所有する一つの action 関係から runtime、capability、permission、approval、recovery、target scope を解決してください。",
    PAYLOAD_HASH_MISMATCH: "正規 payload hash を再計算し、approval/audit を送信 payload へ結び付けてください。",
    APPROVAL_MISSING: "approval を作成し、approved 状態を待ってください。",
    APPROVAL_NOT_VALID: "現在の approval 要求を再検証または承認してください。",
    AUDIT_MAPPING_MISSING: "event_id と必要な payload_hash を持つ AuditEvent を付与してください。",
    RECOVERY_MAPPING_MISSING: "recovery_id を持つ RecoveryAction を付与してください。",
    CONTENT_VISIBILITY_VIOLATION: "content_visibility に従って approval 内容を射影してください。",
    ADAPTER_METADATA_ESCALATION_ATTEMPT: "adapter metadata から権限主張を削除してください。",
    NON_AUTHORITY_SOURCE_ATTEMPT: "権限源を利用してください。memory、cache、previous_state、local_ui_state は権限を与えられません。",
    UPDATE_SIGNATURE_REQUIRED: "update 操作の前に有効な update 署名を必須にしてください。",
    SCHEMA_CONTRACT_MISSING: "検証前に必要な schema 契約を読み込んでください。",
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
