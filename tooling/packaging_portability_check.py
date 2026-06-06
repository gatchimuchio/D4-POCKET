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


DEFAULT_SUBPROCESS_TIMEOUT_SECONDS = 120
WINDOWS_GIT_UNZIP_CANDIDATES = [
    Path(r"C:\Program Files\Git\usr\bin\unzip.exe"),
    Path(r"C:\Program Files (x86)\Git\usr\bin\unzip.exe"),
]


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


def command_label(command: list[str]) -> str:
    return " ".join(str(part) for part in command)


def timeout_output(exc: subprocess.TimeoutExpired) -> str:
    output = exc.output or ""
    if isinstance(output, bytes):
        output = output.decode("utf-8", errors="replace")
    output = str(output).strip()
    return output or "no partial output"


def find_unzip() -> str | None:
    resolved = shutil.which("unzip")
    if resolved is not None:
        return resolved
    if sys.platform == "win32":
        for candidate in WINDOWS_GIT_UNZIP_CANDIDATES:
            if candidate.is_file():
                return str(candidate)
    return None


def run_check(
    cwd: Path,
    command: list[str],
    timeout_seconds: int = DEFAULT_SUBPROCESS_TIMEOUT_SECONDS,
) -> list[str]:
    env = os.environ.copy()
    env["PYTHONDONTWRITEBYTECODE"] = "1"
    try:
        completed = subprocess.run(
            command,
            cwd=cwd,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
            env=env,
            timeout=timeout_seconds,
        )
    except subprocess.TimeoutExpired as exc:
        return [
            f"{command_label(command)} timed out after {timeout_seconds}s: "
            f"{timeout_output(exc)}"
        ]
    if completed.returncode == 0:
        return []
    output = completed.stdout.strip()
    if not output:
        output = "no output"
    return [f"{command_label(command)} failed: {output}"]


def main() -> int:
    source_files, errors = expected_files()
    errors.extend(portable_path_errors(source_files))
    unzip = find_unzip()
    if unzip is None:
        if sys.platform == "win32":
            errors.append(
                "unzip not found on PATH or in Git for Windows at "
                + ", ".join(str(path) for path in WINDOWS_GIT_UNZIP_CANDIDATES)
            )
        else:
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
        unzip_command = [unzip, "-qq", str(archive), "-d", str(extract_root)]
        try:
            completed = subprocess.run(
                unzip_command,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                check=False,
                env=env,
                timeout=DEFAULT_SUBPROCESS_TIMEOUT_SECONDS,
            )
        except subprocess.TimeoutExpired as exc:
            print("packaging portability check failed:")
            print(
                "  - "
                f"{command_label(unzip_command)} timed out after "
                f"{DEFAULT_SUBPROCESS_TIMEOUT_SECONDS}s: {timeout_output(exc)}"
            )
            return 1
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
