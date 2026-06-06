from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from tooling.manifest import expected_files, relative


def portable_path_errors(paths: list[Path]) -> list[str]:
    errors: list[str] = []
    for path in paths:
        rel = relative(path)
        try:
            rel.encode("ascii")
        except UnicodeEncodeError:
            errors.append(f"non-ASCII packaged path: {rel}")
        if any(ord(character) < 32 or ord(character) >= 127 for character in rel):
            errors.append(f"non-portable packaged path: {rel}")
    return errors


def run_check(cwd: Path, command: list[str]) -> list[str]:
    env = os.environ.copy()
    env["PYTHONDONTWRITEBYTECODE"] = "1"
    completed = subprocess.run(
        command,
        cwd=cwd,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
        env=env,
    )
    if completed.returncode == 0:
        return []
    output = completed.stdout.strip()
    if not output:
        output = "no output"
    return [f"{' '.join(command)} failed: {output}"]


def main() -> int:
    source_files, errors = expected_files()
    errors.extend(portable_path_errors(source_files))
    unzip = shutil.which("unzip")
    if unzip is None:
        errors.append("unzip not found on PATH")
    if errors:
        print("packaging portability check failed:")
        for error in errors:
            print(f"  - {error}")
        return 1

    with tempfile.TemporaryDirectory() as raw_tmp:
        tmp = Path(raw_tmp)
        archive = tmp / "gui_shell_source.zip"
        extract_root = tmp / "extract"
        with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as handle:
            for path in source_files:
                handle.write(path, relative(path))
            manifest_path = ROOT / "MANIFEST.sha256.json"
            if manifest_path.exists():
                handle.write(manifest_path, "MANIFEST.sha256.json")
        env = os.environ.copy()
        env["LC_ALL"] = "C"
        env["LANG"] = "C"
        completed = subprocess.run(
            [unzip, "-qq", str(archive), "-d", str(extract_root)],
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
            env=env,
        )
        if completed.returncode != 0:
            print("packaging portability check failed:")
            print(f"  - unzip extraction failed: {completed.stdout.strip()}")
            return 1

        errors = []
        errors.extend(run_check(extract_root, [sys.executable, "tooling/manifest.py", "--check"]))
        errors.extend(run_check(extract_root, [sys.executable, "tooling/conformance_tests/run_conformance_skeleton.py"]))
        errors.extend(run_check(extract_root, [sys.executable, "tooling/release_gate_check.py"]))
        if errors:
            print("packaging portability check failed:")
            for error in errors:
                print(f"  - {error}")
            return 1
    print("packaging portability check passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
