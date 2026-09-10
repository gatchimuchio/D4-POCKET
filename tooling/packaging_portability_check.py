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
UTF8_GOVERNANCE_PATH_ALLOWLIST = frozenset(
    {
        "規定/00_日本語基底規定.md",
        "規定/正本索引.json",
        "規定/日本語基底例外.json",
        "tooling/日本語基底監査.py",
    }
)


def portable_path_errors(paths: list[Path]) -> list[str]:
    errors: list[str] = []
    for path in paths:
        rel = relative(path)
        allowlisted_utf8_path = rel in UTF8_GOVERNANCE_PATH_ALLOWLIST
        try:
            rel.encode("ascii")
        except UnicodeEncodeError:
            if not allowlisted_utf8_path:
                errors.append(f"非ASCIIのpackaged path: {rel}")
        if any(ord(character) < 32 for character in rel) or (
            not allowlisted_utf8_path
            and any(ord(character) >= 127 for character in rel)
        ):
            errors.append(f"portableではないpackaged path: {rel}")
    return errors


def command_label(command: list[str]) -> str:
    return " ".join(str(part) for part in command)


def timeout_output(exc: subprocess.TimeoutExpired) -> str:
    output = exc.output or ""
    if isinstance(output, bytes):
        output = output.decode("utf-8", errors="replace")
    output = str(output).strip()
    return output or "部分出力なし"


def 展開命令(書庫: Path, 展開先: Path) -> list[str]:
    """日本語名を保持する OS 標準展開器を使い、不在を隠さない。"""
    if sys.platform == "win32":
        展開器 = Path(os.environ.get("SystemRoot", r"C:\Windows")) / "System32" / "tar.exe"
        if not 展開器.is_file():
            raise FileNotFoundError("Windows 標準 tar.exe が見つからない")
        return [str(展開器), "-xf", str(書庫), "-C", str(展開先)]
    展開器 = shutil.which("unzip")
    if 展開器 is None:
        raise FileNotFoundError("unzip が PATH に見つからない")
    return [展開器, "-qq", str(書庫), "-d", str(展開先)]


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
            f"{command_label(command)}が{timeout_seconds}s後にtimeout: "
            f"{timeout_output(exc)}"
        ]
    if completed.returncode == 0:
        return []
    output = completed.stdout.strip()
    if not output:
        output = "出力なし"
    return [f"{command_label(command)} が失敗: {output}"]


def main() -> int:
    source_files, errors = expected_files()
    errors.extend(portable_path_errors(source_files))
    if errors:
        print("packaging portability checkが失敗:")
        for error in errors:
            print(f"  - {error}")
        return 1

    with tempfile.TemporaryDirectory() as raw_tmp:
        tmp = Path(raw_tmp)
        archive = tmp / "gui_shell_source.zip"
        extract_root = tmp / "extract"
        extract_root.mkdir()
        with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as handle:
            for path in source_files:
                handle.write(path, relative(path))
            manifest_path = ROOT / "MANIFEST.sha256.json"
            if manifest_path.exists():
                handle.write(manifest_path, "MANIFEST.sha256.json")
        env = os.environ.copy()
        env["LC_ALL"] = "C"
        env["LANG"] = "C"
        try:
            unzip_command = 展開命令(archive, extract_root)
        except FileNotFoundError as exc:
            print(f"packaging portability checkが失敗: {exc}")
            return 1
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
            print("packaging portability checkが失敗:")
            print(
                "  - タイムアウト: "
                f"{command_label(unzip_command)} が次の秒数後にタイムアウト: "
                f"{DEFAULT_SUBPROCESS_TIMEOUT_SECONDS}s: {timeout_output(exc)}"
            )
            return 1
        except OSError as exc:
            print(f"packaging portability checkが失敗: 展開器を起動できない: {exc}")
            return 1
        if completed.returncode != 0:
            print("packaging portability checkが失敗:")
            print(f"  - 展開に失敗: {completed.stdout.strip()}")
            return 1

        errors = []
        errors.extend(run_check(extract_root, [sys.executable, "tooling/manifest.py", "--check"]))
        errors.extend(run_check(extract_root, [sys.executable, "tooling/conformance_tests/run_conformance_skeleton.py"]))
        errors.extend(run_check(extract_root, [sys.executable, "tooling/release_gate_check.py"]))
        if errors:
            print("packaging portability checkが失敗:")
            for error in errors:
                print(f"  - {error}")
            return 1
    print("packaging portability checkが合格")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
