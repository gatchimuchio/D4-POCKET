from __future__ import annotations

from .approval_queue import canonical_hash
from .schema_validation import validate_contract


def build_action_envelope(action: dict, *, envelope_id: str = "action-envelope-evaluated") -> dict:
    payload = action.get("payload")
    envelope = {
        "envelope_id": envelope_id,
        "runtime_id": action["runtime_id"],
        "operation": action["operation"],
        "capability_id": action["capability_id"],
        "permission_id": action["permission_id"],
        "approval_id": action["approval_id"],
        "recovery_id": action["recovery_action"]["recovery_id"],
        "target_scope": action["target_scope"],
        "payload_hash": canonical_hash(payload),
        "authority_source": "broker_internal",
        "audit_event_required": True,
        "metadata": {"evidence_class": "INTERNAL_STATE"},
    }
    validate_contract(envelope, "action_envelope.schema.json")
    return envelope
