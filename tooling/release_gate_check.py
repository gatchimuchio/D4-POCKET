from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RELEASE_BLOCKERS_REGISTRY = ROOT / "release_blockers.registry.json"

SCAN_FILES = [
    "README.md",
    "CLAIM.md",
    "RELEASE_CHECKLIST.md",
    "CONFORMANCE_REPORT.md",
    "SECURITY_REVIEW.md",
    "AUDIT_EVIDENCE.md",
    "INSTALLER_STATUS.md",
    "MOBILE_STATUS.md",
    "COMPATIBILITY_MATRIX.md",
    "VALIDATION.txt",
    "docs/DESKTOP_PLATFORM_MATRIX.md",
    "docs/GUI_OPERATION_SURFACES.md",
    "docs/security/IPC_THREAT_MODEL.md",
    "docs/WINDOWS_RELEASE_PLAN.md",
    "docs/PRODUCT_COMPLETION_PLAN.md",
    "docs/RELEASE_VALIDATION.md",
    "docs/WINDOWS_RELEASE_EVIDENCE.md",
    "docs/STRATEGY.md",
]

CURRENT_FACING_RELEASE_DOCS = [
    "README.md",
    "CLAIM.md",
    "RELEASE_CHECKLIST.md",
    "AUDIT_EVIDENCE.md",
    "SECURITY_REVIEW.md",
    "INSTALLER_STATUS.md",
    "MOBILE_STATUS.md",
    "COMPATIBILITY_MATRIX.md",
    "docs/security/IPC_THREAT_MODEL.md",
    "docs/WINDOWS_RELEASE_PLAN.md",
    "docs/WINDOWS_RELEASE_EVIDENCE.md",
    "docs/PRODUCT_COMPLETION_PLAN.md",
    "docs/RELEASE_VALIDATION.md",
]

PATTERNS = [
    "not run",
    "not verified",
    "not implemented",
    "not complete",
    "still needed",
    "still required",
    "remaining",
    "TODO",
    "future work",
    "skeleton only",
    "mock only",
    "unavailable",
    "not found",
    "not tested",
    "pending",
    "incomplete",
    "placeholder",
    "scaffold",
    "stub",
]

CLASSIFICATIONS = ["release_blocker", "post_v1_scope", "known_limitation", "required_for_v1"]
DOC_SYNC_MARKERS = ["registry_id", "aggregate_of", "superseded_by", "historical", "example"]

MACOS_CLAIM_PATTERNS = [
    r"\bmacos\b.{0,80}\b(verified|supported|ready|complete|release-ready)\b",
    r"\b(verified|supported|ready|complete|release-ready)\b.{0,80}\bmacos\b",
]

MACOS_NEGATION_HINTS = [
    "does not claim",
    "must not be advertised",
    "before claiming",
    "unverified",
    "known_limitation",
    "planned portability",
    "no macos validation environment",
]


def classified_near(lines: list[str], index: int) -> bool:
    start = max(0, index - 2)
    end = min(len(lines), index + 4)
    window = "\n".join(lines[start:end]).lower()
    return any(token in window for token in CLASSIFICATIONS)


def scan_file(path: Path) -> list[str]:
    if not path.exists():
        return [f"{path.relative_to(ROOT)} missing"]
    lines = path.read_text(encoding="utf-8").splitlines()
    errors = []
    for index, line in enumerate(lines):
        lowered = line.lower()
        for pattern in PATTERNS:
            if pattern.lower() in lowered and not classified_near(lines, index):
                errors.append(f"{path.relative_to(ROOT)}:{index + 1}: unclassified unfinished item: {pattern}")
    return errors


def release_claim_exists_without_classification(text: str) -> bool:
    lower = text.lower()
    release_claim = re.search(r"\brelease\b", lower) and "completed product release" in lower
    blocker = "release_blocker" in lower
    return bool(release_claim and blocker and "not yet a completed product release" not in lower)


def macos_support_claim_errors(text: str) -> list[str]:
    errors: list[str] = []
    lower = text.lower()
    for pattern in MACOS_CLAIM_PATTERNS:
        for match in re.finditer(pattern, lower, flags=re.DOTALL):
            start = max(0, match.start() - 120)
            end = min(len(lower), match.end() + 120)
            window = lower[start:end]
            if not any(hint in window for hint in MACOS_NEGATION_HINTS):
                errors.append("macOS support appears claimed without validation evidence")
                return errors
    return errors


def manifest_check_errors() -> list[str]:
    result = subprocess.run(
        [sys.executable, "tooling/manifest.py", "--check"],
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    if result.returncode == 0:
        return []
    output = result.stdout.strip()
    if not output:
        return ["manifest check failed without output"]
    return [f"manifest check failed: {line}" for line in output.splitlines()]


def registry_errors() -> list[str]:
    try:
        registry = json.loads(RELEASE_BLOCKERS_REGISTRY.read_text(encoding="utf-8"))
    except FileNotFoundError:
        return ["release blockers registry missing"]
    except json.JSONDecodeError as exc:
        return [f"release blockers registry invalid JSON: {exc}"]
    if not isinstance(registry, dict):
        return ["release blockers registry must be an object"]
    blockers = registry.get("blockers")
    if not isinstance(blockers, list):
        return ["release blockers registry must contain a blockers list"]
    errors: list[str] = []
    required_fields = {
        "name",
        "status",
        "active",
        "classification",
        "blocks_release",
        "reason",
        "required_action",
    }
    names: set[str] = set()
    for index, blocker in enumerate(blockers):
        if not isinstance(blocker, dict):
            errors.append(f"release blockers registry entry {index} is not an object")
            continue
        missing = sorted(required_fields - set(blocker))
        if missing:
            errors.append(f"release blockers registry entry {index} missing fields: {', '.join(missing)}")
        name = blocker.get("name")
        if not isinstance(name, str) or not name:
            errors.append(f"release blockers registry entry {index} has invalid name")
        elif name in names:
            errors.append(f"duplicate release blocker name: {name}")
        else:
            names.add(name)
        if blocker.get("classification") != "release_blocker":
            errors.append(f"release blocker {name or index} must be classified release_blocker")
        if blocker.get("blocks_release") is not True:
            errors.append(f"release blocker {name or index} must set blocks_release=true")
        if blocker.get("active") not in (True, False):
            errors.append(f"release blocker {name or index} active must be boolean")
        if blocker.get("status") not in {"unresolved", "resolved"}:
            errors.append(f"release blocker {name or index} status must be unresolved or resolved")
        if not isinstance(blocker.get("reason"), str) or not blocker.get("reason"):
            errors.append(f"release blocker {name or index} reason missing")
        if not isinstance(blocker.get("required_action"), str) or not blocker.get("required_action"):
            errors.append(f"release blocker {name or index} required_action missing")
    return errors


def load_release_blocker_registry() -> dict:
    return json.loads(RELEASE_BLOCKERS_REGISTRY.read_text(encoding="utf-8"))


def registry_blocker_names() -> set[str]:
    registry = load_release_blocker_registry()
    blockers = registry.get("blockers", [])
    return {
        blocker["name"]
        for blocker in blockers
        if isinstance(blocker, dict) and isinstance(blocker.get("name"), str)
    }


def unresolved_active_blockers() -> list[dict]:
    registry = load_release_blocker_registry()
    blockers = registry.get("blockers", [])
    return [
        blocker
        for blocker in blockers
        if isinstance(blocker, dict)
        and blocker.get("active") is True
        and blocker.get("status") == "unresolved"
        and blocker.get("classification") == "release_blocker"
        and blocker.get("blocks_release") is True
    ]


def _metadata_values(block: str, key: str) -> list[str]:
    values: list[str] = []
    for match in re.finditer(rf"(?im)^\s*{re.escape(key)}\s*:\s*(.+?)\s*$", block):
        values.extend(_split_metadata_values(match.group(1)))
    for match in re.finditer(rf"\b{re.escape(key)}=([A-Za-z0-9_,.-]+)", block):
        values.extend(_split_metadata_values(match.group(1)))
    return values


def _split_metadata_values(raw: str) -> list[str]:
    cleaned = raw.strip().strip("[]`")
    cleaned = cleaned.replace("[", "").replace("]", "")
    parts = re.split(r"[\s,]+", cleaned)
    return [part.strip().strip("`'\"") for part in parts if part.strip().strip("`'\"")]


def _has_doc_sync_marker(block: str) -> bool:
    lowered = block.lower()
    return any(f"{marker}:" in lowered or f"{marker}=" in lowered for marker in DOC_SYNC_MARKERS)


def _has_truthy_marker(block: str, key: str) -> bool:
    return any(value.lower() in {"true", "yes"} for value in _metadata_values(block, key))


def _unknown_registry_refs(block: str, names: set[str]) -> list[str]:
    refs: list[str] = []
    for key in ("registry_id", "aggregate_of", "superseded_by"):
        refs.extend(_metadata_values(block, key))
    ignored = {"", "none", "true", "false", "yes", "no"}
    return sorted({ref for ref in refs if ref.lower() not in ignored and ref not in names})


def _release_blocker_doc_blocks(lines: list[str]) -> list[tuple[int, str]]:
    blocks: list[tuple[int, str]] = []
    index = 0
    while index < len(lines):
        line = lines[index]
        item_match = re.match(r"^\s*-\s+item:\s+", line)
        classification_rule_match = re.match(r"^\s*-\s+classification:\s*`?release_blocker`?\s*$", line)
        if item_match or classification_rule_match:
            start = index
            index += 1
            while index < len(lines):
                next_line = lines[index]
                if re.match(r"^\s*-\s+item:\s+", next_line):
                    break
                if classification_rule_match and re.match(r"^\s*-\s+classification:\s+", next_line):
                    break
                if next_line.startswith("#"):
                    break
                index += 1
            block = "\n".join(lines[start:index])
            if re.search(r"(?im)^\s*classification:\s*`?release_blocker`?\s*$", block):
                blocks.append((start + 1, block))
            continue
        if "|" in line and "release_blocker" in line:
            blocks.append((index + 1, line))
        index += 1
    return blocks


def release_blocker_doc_sync_errors() -> list[str]:
    names = registry_blocker_names()
    errors: list[str] = []
    for relative in CURRENT_FACING_RELEASE_DOCS:
        path = ROOT / relative
        if not path.exists():
            errors.append(f"{relative} missing from release blocker doc sync scan")
            continue
        lines = path.read_text(encoding="utf-8").splitlines()
        for line_number, block in _release_blocker_doc_blocks(lines):
            if not _has_doc_sync_marker(block):
                errors.append(
                    f"{relative}:{line_number}: release_blocker block missing registry_id, aggregate_of, superseded_by, historical, or example marker"
                )
                continue
            unknown = _unknown_registry_refs(block, names)
            if unknown:
                errors.append(
                    f"{relative}:{line_number}: release_blocker block references unknown registry blocker: {', '.join(unknown)}"
                )
            if (
                not _metadata_values(block, "registry_id")
                and not _metadata_values(block, "aggregate_of")
                and not _metadata_values(block, "superseded_by")
                and not _has_truthy_marker(block, "historical")
                and not _has_truthy_marker(block, "example")
            ):
                errors.append(
                    f"{relative}:{line_number}: release_blocker marker exists but does not bind to registry, aggregate, superseded, historical=true, or example=true"
                )
    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--strict-release", action="store_true")
    args = parser.parse_args()

    errors: list[str] = []
    combined = ""
    for relative in SCAN_FILES:
        path = ROOT / relative
        if path.exists():
            combined += "\n" + path.read_text(encoding="utf-8")
        errors.extend(scan_file(path))

    errors.extend(registry_errors())
    if not errors:
        errors.extend(release_blocker_doc_sync_errors())
    if args.strict_release and not errors:
        for blocker in unresolved_active_blockers():
            errors.append(
                "strict release active blocker unresolved: "
                f"{blocker['name']} - {blocker['reason']}"
            )
    if release_claim_exists_without_classification(combined):
        errors.append("release claim appears while release_blocker exists")
    errors.extend(macos_support_claim_errors(combined))
    errors.extend(manifest_check_errors())

    if errors:
        print("release gate check failed:")
        for error in errors:
            print(f"  - {error}")
        return 1

    print("release gate check passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
