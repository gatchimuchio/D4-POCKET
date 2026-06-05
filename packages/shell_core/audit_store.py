import copy
import hashlib
import json


class AuditStore:
    def __init__(self):
        self._events: list[dict] = []

    def append(self, event: dict) -> dict:
        event_id = event.get("event_id")
        if not event_id:
            raise ValueError("audit event_id is required")
        if any(stored.get("event_id") == event_id for stored in self._events):
            raise ValueError(f"duplicate audit event_id: {event_id}")
        previous_hash = self._events[-1]["event_hash"] if self._events else None
        stored = copy.deepcopy(event)
        stored["previous_event_hash"] = previous_hash
        encoded = json.dumps(stored, sort_keys=True, separators=(",", ":")).encode("utf-8")
        stored["event_hash"] = "sha256:" + hashlib.sha256(encoded).hexdigest()
        self._events.append(stored)
        return copy.deepcopy(stored)

    def events(self) -> list[dict]:
        return copy.deepcopy(self._events)
