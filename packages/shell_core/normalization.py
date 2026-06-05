from __future__ import annotations

import copy
import re
import unicodedata

from .authority_keys import AUTHORITY_KEYS


ZERO_WIDTH_CHARACTERS = {
    "\u200b",
    "\u200c",
    "\u200d",
    "\ufeff",
}

AUTHORITY_KEY_ALIASES = {
    "admin_context": "authority_context",
    "elevated": "scope_escalation",
    "grant": "permission_grant",
    "grants": "permission_grant",
    "permission": "permission_grant",
    "permissions": "permission_grant",
    "permissions_granted": "permission_grant",
    "privilege": "permission_grant",
    "privileges": "permission_grant",
    "permissiongrant": "permission_grant",
}

AUTHORITY_VALUES = frozenset({"admin", "all", "approved", "elevated", "root"})


def normalize_key(key: object) -> str:
    normalized = unicodedata.normalize("NFKC", str(key))
    for character in ZERO_WIDTH_CHARACTERS:
        normalized = normalized.replace(character, "")
    normalized = normalized.strip()
    normalized = re.sub(r"(?<=[a-z0-9])(?=[A-Z])", "_", normalized)
    normalized = normalized.casefold()
    normalized = re.sub(r"[\s\-]+", "_", normalized)
    normalized = re.sub(r"_+", "_", normalized)
    return normalized


def canonical_authority_key(key: object) -> str | None:
    normalized = normalize_key(key)
    if normalized in AUTHORITY_KEYS:
        return normalized
    return AUTHORITY_KEY_ALIASES.get(normalized)


def normalize_payload(value):
    if isinstance(value, dict):
        return {normalize_key(key): normalize_payload(item) for key, item in value.items()}
    if isinstance(value, list):
        return [normalize_payload(item) for item in value]
    if isinstance(value, str):
        return unicodedata.normalize("NFKC", value).strip()
    return copy.deepcopy(value)


def normalization_collisions_in(value, *, path: str = "") -> list[dict]:
    findings: list[dict] = []
    if isinstance(value, dict):
        seen: dict[str, str] = {}
        for key, item in value.items():
            normalized = normalize_key(key)
            child_path = f"{path}.{normalized}" if path else normalized
            if normalized in seen and seen[normalized] != str(key):
                findings.append(
                    {
                        "path": child_path,
                        "normalized_key": normalized,
                        "first_key": seen[normalized],
                        "colliding_key": str(key),
                    }
                )
            else:
                seen[normalized] = str(key)
            findings.extend(normalization_collisions_in(item, path=child_path))
    elif isinstance(value, list):
        for index, item in enumerate(value):
            findings.extend(normalization_collisions_in(item, path=f"{path}[{index}]"))
    return findings


def authority_keys_in(value, *, path: str = "") -> list[dict]:
    findings: list[dict] = []
    if isinstance(value, dict):
        for key, item in value.items():
            canonical = canonical_authority_key(key)
            child_path = f"{path}.{normalize_key(key)}" if path else normalize_key(key)
            if canonical is not None:
                findings.append({"path": child_path, "key": str(key), "canonical_key": canonical})
            findings.extend(authority_keys_in(item, path=child_path))
    elif isinstance(value, list):
        for index, item in enumerate(value):
            findings.extend(authority_keys_in(item, path=f"{path}[{index}]"))
    return findings


def authority_values_in(value, *, path: str = "") -> list[dict]:
    findings: list[dict] = []
    if isinstance(value, dict):
        for key, item in value.items():
            child_path = f"{path}.{normalize_key(key)}" if path else normalize_key(key)
            findings.extend(authority_values_in(item, path=child_path))
    elif isinstance(value, list):
        for index, item in enumerate(value):
            findings.extend(authority_values_in(item, path=f"{path}[{index}]"))
    elif isinstance(value, str):
        normalized = normalize_key(value)
        if normalized in AUTHORITY_VALUES:
            findings.append({"path": path, "value": value, "canonical_value": normalized})
    return findings


def strip_authority_keys(value):
    if isinstance(value, dict):
        return {
            normalize_key(key): strip_authority_keys(item)
            for key, item in value.items()
            if canonical_authority_key(key) is None
        }
    if isinstance(value, list):
        return [strip_authority_keys(item) for item in value]
    return copy.deepcopy(value)


def normalize_inbound_payload(payload: dict) -> dict:
    raw_payload = copy.deepcopy(payload)
    normalized_payload = normalize_payload(payload)
    stripped_payload = strip_authority_keys(payload)
    key_findings = authority_keys_in(payload)
    value_findings = authority_values_in(stripped_payload)
    collision_findings = normalization_collisions_in(payload)
    quarantined = bool(key_findings or value_findings or collision_findings)
    return {
        "raw_payload": raw_payload,
        "normalized_payload": normalized_payload,
        "stripped_payload": stripped_payload,
        "quarantined": quarantined,
        "authority_key_findings": key_findings,
        "authority_value_findings": value_findings,
        "normalization_collision_findings": collision_findings,
        "audit_event": {
            "event_type": "normalization.quarantine" if quarantined else "normalization.pass",
            "authority_key_count": len(key_findings),
            "authority_value_count": len(value_findings),
            "normalization_collision_count": len(collision_findings),
            "raw_payload_preserved": True,
        },
    }
