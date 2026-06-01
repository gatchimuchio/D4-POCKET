from __future__ import annotations

import argparse
import hashlib
import json
from dataclasses import dataclass
from pathlib import Path, PurePosixPath


ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "MANIFEST.sha256.json"

GLOB_PATTERNS = [
    "packages/shell_core/**/*.py",
    "tooling/**/*.py",
    "schemas/**/*.json",
    "specs/**/*.json",
    "examples/**/*.json",
    "apps/desktop_flutter/lib/**/*.dart",
    "apps/desktop_flutter/test/**/*.dart",
    "native/rust_helper/src/**/*.rs",
    "docs/**/*.md",
]

EXACT_FILES = [
    "AGENTS.md",
    "ROADMAP.md",
    "CONFORMANCE_REPORT.md",
    "COMPATIBILITY_MATRIX.md",
    "apps/desktop_flutter/pubspec.yaml",
    "README.md",
    "CLAIM.md",
    "RELEASE_CHECKLIST.md",
    "VALIDATION.txt",
    "AUDIT_EVIDENCE.md",
    "INSTALLER_STATUS.md",
    "SECURITY_REVIEW.md",
    "MOBILE_STATUS.md",
]

EXCLUDED_PATHS = {
    "MANIFEST.sha256.json",
    "release_evidence/windows_installed_smoke.json",
    "apps/mobile_flutter/pubspec.lock",
}

EXCLUDED_PARTS = {
    ".git",
    ".dart_tool",
    "build",
    "target",
    "__pycache__",
}


@dataclass(frozen=True)
class ManifestEntry:
    path: str
    sha256: str


def relative(path: Path) -> str:
    return path.relative_to(ROOT).as_posix()


def is_excluded(path: Path) -> bool:
    return is_excluded_relative(relative(path))


def matches_forbidden(path: str) -> bool:
    return is_excluded_relative(path)


def is_excluded_relative(path: str) -> bool:
    normalized = path.replace("\\", "/")
    parts = PurePosixPath(normalized).parts
    return (
        not normalized
        or normalized.startswith("/")
        or normalized in EXCLUDED_PATHS
        or ".." in parts
        or any(part in EXCLUDED_PARTS for part in parts)
    )


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def expected_files() -> tuple[list[Path], list[str]]:
    files: set[Path] = set()
    errors: list[str] = []

    for pattern in GLOB_PATTERNS:
        for path in ROOT.glob(pattern):
            if path.is_file() and not is_excluded(path):
                files.add(path)

    for name in EXACT_FILES:
        path = ROOT / name
        if not path.exists():
            errors.append(f"required file missing from workspace: {name}")
        elif path.is_file() and not is_excluded(path):
            files.add(path)

    shell_core_files = sorted((ROOT / "packages" / "shell_core").glob("**/*.py"))
    if not shell_core_files:
        errors.append("core Shell Core files are absent")

    return sorted(files, key=relative), errors


def build_manifest() -> tuple[dict, list[str]]:
    files, errors = expected_files()
    entries = [ManifestEntry(relative(path), sha256(path)).__dict__ for path in files]
    return {
        "version": 1,
        "hash": "sha256",
        "self_reference_policy": "MANIFEST.sha256.json is excluded from its own file list.",
        "coverage": {
            "glob_patterns": GLOB_PATTERNS,
            "exact_files": EXACT_FILES,
            "excluded_paths": sorted(EXCLUDED_PATHS),
            "excluded_directory_names": sorted(EXCLUDED_PARTS),
        },
        "files": entries,
    }, errors


def load_manifest() -> tuple[dict | None, list[str]]:
    try:
        data = json.loads(MANIFEST.read_text(encoding="utf-8"))
    except FileNotFoundError:
        return None, ["MANIFEST.sha256.json missing"]
    except json.JSONDecodeError as exc:
        return None, [f"MANIFEST.sha256.json is invalid JSON: {exc}"]
    if not isinstance(data, dict):
        return None, ["MANIFEST.sha256.json must contain a JSON object"]
    if not isinstance(data.get("files"), list):
        return None, ["MANIFEST.sha256.json must contain a files list"]
    return data, []


def check_manifest() -> list[str]:
    expected, expected_errors = build_manifest()
    actual, load_errors = load_manifest()
    if actual is None:
        return expected_errors + load_errors

    errors = expected_errors + load_errors
    expected_by_path = {entry["path"]: entry["sha256"] for entry in expected["files"]}
    actual_by_path: dict[str, str] = {}

    for index, entry in enumerate(actual["files"]):
        if not isinstance(entry, dict):
            errors.append(f"manifest entry {index} is not an object")
            continue
        path = entry.get("path")
        digest = entry.get("sha256")
        if not isinstance(path, str) or not isinstance(digest, str):
            errors.append(f"manifest entry {index} must contain string path and sha256")
            continue
        if path in actual_by_path:
            errors.append(f"duplicate manifest entry: {path}")
        actual_by_path[path] = digest
        if matches_forbidden(path):
            errors.append(f"forbidden generated or self-referenced file listed: {path}")
        file_path = ROOT / path
        if not file_path.exists():
            errors.append(f"listed file missing: {path}")
        elif file_path.is_file():
            actual_digest = sha256(file_path)
            if actual_digest != digest:
                errors.append(f"hash mismatch: {path}")

    for path in sorted(set(expected_by_path) - set(actual_by_path)):
        errors.append(f"required source file missing from manifest: {path}")

    shell_core_listed = [path for path in actual_by_path if path.startswith("packages/shell_core/") and path.endswith(".py")]
    if not shell_core_listed:
        errors.append("core Shell Core files are absent from manifest")

    return errors


def write_manifest() -> int:
    manifest, errors = build_manifest()
    if errors:
        for error in errors:
            print(f"manifest generation failed: {error}")
        return 1
    MANIFEST.write_text(json.dumps(manifest, indent=2, sort_keys=False) + "\n", encoding="utf-8")
    print(f"wrote {MANIFEST.relative_to(ROOT)} with {len(manifest['files'])} files")
    return 0


def run_check() -> int:
    errors = check_manifest()
    if errors:
        print("manifest check failed:")
        for error in errors:
            print(f"  - {error}")
        return 1
    print("manifest check passed")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--write", action="store_true")
    group.add_argument("--check", action="store_true")
    args = parser.parse_args()

    if args.write:
        return write_manifest()
    return run_check()


if __name__ == "__main__":
    raise SystemExit(main())
