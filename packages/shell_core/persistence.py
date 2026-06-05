import copy
import json
from pathlib import Path

from .audit_chain import chain_event, verify_audit_chain
from .runtime_state import RuntimeState
from .state_store import load_snapshot, save_snapshot


class JsonPersistence:
    def __init__(self, root: Path):
        self.root = root
        self.audit_path = root / "audit.jsonl"
        self.snapshot_path = root / "state_snapshot.json"

    def append_audit_event(self, event: dict) -> dict:
        self.root.mkdir(parents=True, exist_ok=True)
        event_id = event.get("event_id")
        if not event_id:
            raise ValueError("audit event_id is required")
        events = self.audit_events()
        verification = verify_audit_chain(events)
        if verification["ok"] is not True:
            raise ValueError(
                "cannot append to invalid audit chain: "
                + "; ".join(verification.get("errors", []))
            )
        if any(stored.get("event_id") == event_id for stored in events):
            raise ValueError(f"duplicate audit event_id: {event_id}")
        previous = events[-1].get("event_hash") if events else None
        chained = chain_event(event, previous)
        with self.audit_path.open("a", encoding="utf-8") as handle:
            handle.write(json.dumps(chained, sort_keys=True, separators=(",", ":")) + "\n")
        return copy.deepcopy(chained)

    def audit_events(self) -> list[dict]:
        if not self.audit_path.exists():
            return []
        return [json.loads(line) for line in self.audit_path.read_text(encoding="utf-8").splitlines() if line.strip()]

    def verify_audit_chain(self) -> dict:
        return verify_audit_chain(self.audit_events())

    def export_audit(self) -> list[dict]:
        return copy.deepcopy(self.audit_events())

    def detect_tamper(self) -> bool:
        return not self.verify_audit_chain()["ok"]

    def save_snapshot(self, state: RuntimeState) -> dict:
        return save_snapshot(state, self.snapshot_path)

    def load_snapshot(self) -> dict:
        return load_snapshot(self.snapshot_path)

    def _latest_event_hash(self) -> str | None:
        events = self.audit_events()
        return events[-1].get("event_hash") if events else None
