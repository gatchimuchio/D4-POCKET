import copy
import hmac
import json
import secrets
from pathlib import Path

from .audit_chain import chain_event, verify_audit_chain
from .runtime_state import RuntimeState
from .state_store import load_snapshot, save_snapshot


class JsonPersistence:
    def __init__(self, root: Path):
        self.root = root
        self.audit_path = root / "audit.jsonl"
        self.audit_anchor_path = root / "audit_anchor.json"
        self.audit_anchor_key_path = root / "audit_anchor.key"
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
        self._write_audit_anchor(events + [chained])
        return copy.deepcopy(chained)

    def audit_events(self) -> list[dict]:
        report = self.audit_events_report()
        if report["errors"]:
            raise ValueError("; ".join(report["errors"]))
        return copy.deepcopy(report["events"])

    def audit_events_report(self) -> dict:
        if not self.audit_path.exists():
            return {"events": [], "errors": []}
        events = []
        errors = []
        for index, line in enumerate(self.audit_path.read_text(encoding="utf-8").splitlines(), start=1):
            if not line.strip():
                continue
            try:
                parsed = json.loads(line)
            except json.JSONDecodeError as exc:
                errors.append(f"corrupt audit JSONL line {index}: {exc.msg}")
                continue
            if not isinstance(parsed, dict):
                errors.append(f"corrupt audit JSONL line {index}: event is not an object")
                continue
            events.append(parsed)
        return {"events": events, "errors": errors}

    def verify_audit_chain(self) -> dict:
        report = self.audit_events_report()
        result = verify_audit_chain(report["events"])
        anchor = self._verify_audit_anchor(report["events"])
        if report["errors"]:
            result["ok"] = False
            result["errors"] = report["errors"] + result["errors"]
        if not anchor["ok"]:
            result["ok"] = False
            result["errors"] = result["errors"] + anchor["errors"]
        result["anchor_verified"] = anchor["ok"]
        result["anchor_evidence_source"] = "INTERNAL_STATE"
        return result

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

    def _audit_anchor_key(self) -> bytes:
        self.root.mkdir(parents=True, exist_ok=True)
        if self.audit_anchor_key_path.exists():
            return bytes.fromhex(self.audit_anchor_key_path.read_text(encoding="utf-8").strip())
        key = secrets.token_bytes(32)
        self.audit_anchor_key_path.write_text(key.hex(), encoding="utf-8")
        return key

    def _anchor_record(self, events: list[dict]) -> dict:
        latest = events[-1].get("event_hash") if events else None
        body = {
            "version": 1,
            "event_count": len(events),
            "head_event_hash": latest,
        }
        message = json.dumps(body, sort_keys=True, separators=(",", ":")).encode("utf-8")
        body["anchor_hmac"] = "sha256:" + hmac.new(
            self._audit_anchor_key(),
            message,
            "sha256",
        ).hexdigest()
        return body

    def _write_audit_anchor(self, events: list[dict]) -> None:
        record = self._anchor_record(events)
        temporary = self.audit_anchor_path.with_suffix(".json.tmp")
        temporary.write_text(json.dumps(record, sort_keys=True, indent=2) + "\n", encoding="utf-8")
        temporary.replace(self.audit_anchor_path)

    def _verify_audit_anchor(self, events: list[dict]) -> dict:
        if not events and not self.audit_anchor_path.exists():
            return {"ok": True, "errors": []}
        if events and not self.audit_anchor_path.exists():
            return {
                "ok": False,
                "errors": ["audit anchor missing for non-empty audit chain"],
            }
        try:
            stored = json.loads(self.audit_anchor_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as exc:
            return {"ok": False, "errors": [f"audit anchor unreadable: {exc}"]}
        expected = self._anchor_record(events)
        if stored != expected:
            return {
                "ok": False,
                "errors": ["audit anchor HMAC does not match audit chain head"],
            }
        return {"ok": True, "errors": []}
