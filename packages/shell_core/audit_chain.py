import copy
import hashlib
import json


def canonical_hash(payload: object) -> str:
    encoded = json.dumps(payload, sort_keys=True, separators=(",", ":")).encode("utf-8")
    return "sha256:" + hashlib.sha256(encoded).hexdigest()


def chain_event(event: dict, previous_event_hash: str | None) -> dict:
    chained = copy.deepcopy(event)
    chained["previous_event_hash"] = previous_event_hash
    chained.pop("event_hash", None)
    chained["event_hash"] = canonical_hash(chained)
    return chained


def verify_audit_chain(events: list[dict]) -> dict:
    previous = None
    errors = []
    seen_event_ids = set()
    for index, event in enumerate(events):
        event_id = event.get("event_id")
        if not event_id:
            errors.append(f"event {index} に event_id がありません")
        elif event_id in seen_event_ids:
            errors.append(f"event {index} の event_id {event_id} が重複しています")
        else:
            seen_event_ids.add(event_id)
        expected = chain_event({key: value for key, value in event.items() if key != "event_hash"}, previous)
        if event.get("previous_event_hash") != previous:
            errors.append(f"event {index} の previous hash が一致しません")
        if event.get("event_hash") != expected["event_hash"]:
            errors.append(f"event {index} の hash が一致しません")
        previous = event.get("event_hash")
    return {
        "ok": not errors,
        "event_count": len(events),
        "latest_event_hash": previous,
        "errors": errors,
    }
