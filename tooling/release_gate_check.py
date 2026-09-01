from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
RELEASE_BLOCKERS_REGISTRY = ROOT / "release_blockers.registry.json"
WINDOWS_EVIDENCE_STATUS_SOURCE = "windows_release_evidence"

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
        return [f"{path.relative_to(ROOT)} が存在しない"]
    lines = path.read_text(encoding="utf-8").splitlines()
    errors = []
    for index, line in enumerate(lines):
        lowered = line.lower()
        for pattern in PATTERNS:
            if pattern.lower() in lowered and not classified_near(lines, index):
                errors.append(f"{path.relative_to(ROOT)}:{index + 1}: 未分類の未完了item: {pattern}")
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
                errors.append("validation evidenceなしにmacOS supportを主張しているように見える")
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
        return ["manifest checkが出力なしで失敗"]
    return [f"manifest checkが失敗: {line}" for line in output.splitlines()]


def registry_errors() -> list[str]:
    try:
        registry = json.loads(RELEASE_BLOCKERS_REGISTRY.read_text(encoding="utf-8"))
    except FileNotFoundError:
        return ["release blocker registryがない"]
    except json.JSONDecodeError as exc:
        return [f"release blocker registryが無効なJSON: {exc}"]
    if not isinstance(registry, dict):
        return ["release blocker registryはobjectでなければならない"]
    blockers = registry.get("blockers")
    if not isinstance(blockers, list):
        return ["release blocker registryはblockers listを含まなければならない"]
    errors: list[str] = []
    required_fields = {
        "name",
        "status",
        "status_source",
        "active",
        "classification",
        "blocks_release",
        "reason",
        "required_action",
    }
    names: set[str] = set()
    for index, blocker in enumerate(blockers):
        if not isinstance(blocker, dict):
            errors.append(f"リリース阻止事項の登録項目 {index} がobjectではない")
            continue
        missing = sorted(required_fields - set(blocker))
        if missing:
                errors.append(f"リリース阻止事項の登録項目 {index} にfieldがない: {', '.join(missing)}")
        name = blocker.get("name")
        if not isinstance(name, str) or not name:
            errors.append(f"リリース阻止事項の登録項目 {index} のnameが無効")
        elif name in names:
            errors.append(f"release blocker nameが重複: {name}")
        else:
            names.add(name)
        if blocker.get("classification") != "release_blocker":
            errors.append(f"release blocker {name or index}はrelease_blockerに分類しなければならない")
        if blocker.get("blocks_release") is not True:
            errors.append(f"release blocker {name or index}はblocks_release=trueを設定しなければならない")
        if blocker.get("active") not in (True, False):
            errors.append(f"release blocker {name or index}のactiveはbooleanでなければならない")
        if blocker.get("status") not in {"unresolved", "resolved"}:
            errors.append(f"release blocker {name or index}のstatusはunresolvedまたはresolvedでなければならない")
        status_source = blocker.get("status_source")
        if status_source not in {"manual", WINDOWS_EVIDENCE_STATUS_SOURCE}:
            errors.append(f"release blocker {name or index}のstatus_sourceはmanualまたは{WINDOWS_EVIDENCE_STATUS_SOURCE}でなければならない")
        if status_source == WINDOWS_EVIDENCE_STATUS_SOURCE:
            evidence_result = blocker.get("evidence_result")
            if not isinstance(evidence_result, str) or not evidence_result:
                errors.append(f"release blocker {name or index}に{WINDOWS_EVIDENCE_STATUS_SOURCE}用evidence_resultがない")
        if not isinstance(blocker.get("reason"), str) or not blocker.get("reason"):
            errors.append(f"release blocker {name or index}にreasonがない")
        if not isinstance(blocker.get("required_action"), str) or not blocker.get("required_action"):
            errors.append(f"release blocker {name or index}にrequired_actionがない")
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
    windows_results = _windows_evidence_results_by_name(blockers)
    unresolved: list[dict] = []
    for blocker in blockers:
        if not isinstance(blocker, dict):
            continue
        effective = effective_release_blocker(blocker, windows_results)
        if (
            effective.get("active") is True
            and effective.get("status") == "unresolved"
            and effective.get("classification") == "release_blocker"
            and effective.get("blocks_release") is True
        ):
            unresolved.append(effective)
    return unresolved


def _windows_evidence_results_by_name(blockers: list) -> dict[str, object]:
    if not any(
        isinstance(blocker, dict)
        and blocker.get("status_source") == WINDOWS_EVIDENCE_STATUS_SOURCE
        for blocker in blockers
    ):
        return {}
    from tooling.windows_release_evidence import validate_windows_release_evidence

    return {result.name: result for result in validate_windows_release_evidence()}


def effective_release_blocker(blocker: dict, windows_results: dict[str, object]) -> dict:
    effective = dict(blocker)
    if blocker.get("status_source") != WINDOWS_EVIDENCE_STATUS_SOURCE:
        effective["effective_status_source"] = blocker.get("status_source", "manual")
        return effective

    evidence_result_name = str(blocker.get("evidence_result") or blocker.get("name") or "")
    result = windows_results.get(evidence_result_name)
    effective["effective_status_source"] = WINDOWS_EVIDENCE_STATUS_SOURCE
    if result is None:
        effective["status"] = "unresolved"
        effective["reason"] = f"Windows evidence resultがない: {evidence_result_name}"
        effective["required_action"] = "tooling/windows_release_evidence.pyを実行し、registryのevidence_result nameを同期させる。"
        return effective

    if getattr(result, "status", "") == "passed" and getattr(result, "classification", "") != "release_blocker":
        effective["status"] = "resolved"
        effective["reason"] = f"Windows evidenceにより解決: {getattr(result, 'reason', '')}"
        effective["required_action"] = getattr(result, "required_action", "")
        return effective

    effective["status"] = "unresolved"
    effective["reason"] = getattr(result, "reason", "")
    effective["required_action"] = getattr(result, "required_action", "")
    return effective


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
            errors.append(f"{relative}がrelease blocker文書同期scanにない")
            continue
        lines = path.read_text(encoding="utf-8").splitlines()
        for line_number, block in _release_blocker_doc_blocks(lines):
            if not _has_doc_sync_marker(block):
                errors.append(
                    f"{relative}:{line_number}: release_blocker blockにregistry_id、aggregate_of、superseded_by、historical、example markerのいずれもない"
                )
                continue
            unknown = _unknown_registry_refs(block, names)
            if unknown:
                errors.append(
                    f"{relative}:{line_number}: release_blocker blockが未知のregistry blockerを参照: {', '.join(unknown)}"
                )
            if (
                not _metadata_values(block, "registry_id")
                and not _metadata_values(block, "aggregate_of")
                and not _metadata_values(block, "superseded_by")
                and not _has_truthy_marker(block, "historical")
                and not _has_truthy_marker(block, "example")
            ):
                errors.append(
                    f"{relative}:{line_number}: release_blocker markerは存在するがregistry、aggregate、superseded、historical=true、example=trueのいずれにも結び付かない"
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
            "strict releaseのactive blockerが未解決: "
                f"{blocker['name']} - {blocker['reason']}"
            )
    if release_claim_exists_without_classification(combined):
        errors.append("release_blockerが存在するのにrelease claimがある")
    errors.extend(macos_support_claim_errors(combined))
    errors.extend(manifest_check_errors())

    if errors:
        print("release gate checkが失敗:")
        for error in errors:
            print(f"  - {error}")
        return 1

    print("release gate checkが合格")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
