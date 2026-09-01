import copy
import hashlib
import json


PROTECTED_EDIT_FIELDS = {
    "runtime_id",
    "permission_id",
    "audit_id",
    "audit_event_id",
    "payload_hash",
}


def canonical_hash(payload: object) -> str:
    encoded = json.dumps(payload, sort_keys=True, separators=(",", ":")).encode("utf-8")
    return "sha256:" + hashlib.sha256(encoded).hexdigest()


def protected_fields(approval: dict) -> set[str]:
    return (
        set(approval.get("authority_fields", []))
        | set(approval.get("sealed_fields", []))
        | set(approval.get("hidden_fields", []))
        | set(approval.get("sacred_fields", []))
        | PROTECTED_EDIT_FIELDS
    )


class ApprovalQueue:
    def __init__(self):
        self._approvals: dict[str, dict] = {}

    def enqueue(self, approval: dict) -> None:
        self._approvals[approval["approval_id"]] = copy.deepcopy(approval)

    def get(self, approval_id: str) -> dict:
        if approval_id not in self._approvals:
            raise KeyError(f"queue にない approval です: {approval_id}")
        return copy.deepcopy(self._approvals[approval_id])

    def can_edit(self, approval_id: str, field: str) -> bool:
        approval = self.get(approval_id)
        return field in set(approval.get("editable_fields", [])) and field not in protected_fields(approval)

    def edit(self, approval_id: str, field: str, value) -> dict:
        if not self.can_edit(approval_id, field):
            raise ValueError(f"編集できない field です: {field}")
        approval = self.get(approval_id)
        payload = copy.deepcopy(approval.get("full_payload", {}))
        payload[field] = value
        approval["full_payload"] = payload
        approval["payload_hash"] = canonical_hash(payload)
        approval["status"] = "requires_validation"
        self._approvals[approval_id] = copy.deepcopy(approval)
        return approval
