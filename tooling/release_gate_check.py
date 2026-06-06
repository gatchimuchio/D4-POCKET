from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]

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

    if args.strict_release and "release_blocker" in combined:
        errors.append("strict release mode found release_blocker classifications")
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
