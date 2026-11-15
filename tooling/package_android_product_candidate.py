"""Package Android debug and unsigned release APKs with bounded provenance."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import stat
import subprocess
import sys
import tempfile
import zipfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
SCHEMA_PATH = ROOT / "specs" / "gui_shell_android_product_candidate.schema.json"
MAX_APK_BYTES = 800_000_000
MAX_APK_ENTRIES = 100_000
COMMIT_PATTERN = re.compile(r"^[0-9a-f]{40}$")
APP_ID_PATTERN = re.compile(r"^[A-Za-z][A-Za-z0-9_]*(?:\.[A-Za-z][A-Za-z0-9_]*)+$")
VERSION_PATTERN = re.compile(r"^[0-9][A-Za-z0-9.+_-]{0,63}$")
PACKAGE_LINE = re.compile(
    r"(?m)^package: name='([^']+)' versionCode='([0-9]+)' versionName='([^']*)'"
)
DEBUG_CERTIFICATE_DN = frozenset({"cn=androiddebug", "o=android", "c=us"})
APK_SIGNING_BLOCK_MAGIC = b"APK Sig Block 42"
UNSIGNED_APK_DIAGNOSTIC = re.compile(
    r"(?mi)^ERROR:\s*No signatures found in APK\s*$"
)

if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from tooling.schema_check.check_schemas import validate_instance  # noqa: E402


@dataclass(frozen=True)
class ApkFacts:
    path: Path
    sha256: str
    size_bytes: int
    application_id: str
    version_code: int
    version_name: str
    native_abis: tuple[str, ...]
    signature_state: str


def _is_reparse_point(metadata: os.stat_result) -> bool:
    flag = getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0x400)
    return bool(getattr(metadata, "st_file_attributes", 0) & flag)


def _path_is_within(path: Path, parent: Path) -> bool:
    try:
        path.resolve(strict=False).relative_to(parent.resolve(strict=False))
        return True
    except ValueError:
        return False


def _assert_no_reparse_ancestors(path: Path) -> None:
    current = path.absolute()
    while True:
        try:
            metadata = current.lstat()
        except OSError as error:
            raise ValueError("候補pathのdirectory境界を確認できない") from error
        if stat.S_ISLNK(metadata.st_mode) or _is_reparse_point(metadata):
            raise ValueError("候補pathにsymbolic linkまたはreparse pointがある")
        if current.parent == current:
            return
        current = current.parent


def _one_drive_roots() -> tuple[Path, ...]:
    roots: set[Path] = set()
    for name in ("OneDrive", "OneDriveCommercial", "OneDriveConsumer"):
        value = os.environ.get(name)
        if value:
            roots.add(Path(value).expanduser().resolve(strict=False))
    return tuple(roots)


def _reject_repository_or_sync_path(path: Path, description: str) -> Path:
    resolved = path.expanduser().resolve(strict=False)
    if _path_is_within(resolved, ROOT.resolve()):
        raise ValueError(f"{description}はRepository外でなければならない")
    if any(_path_is_within(resolved, root) for root in _one_drive_roots()):
        raise ValueError(f"{description}はOneDrive同期領域外でなければならない")
    return resolved


def _output_target(path: Path) -> Path:
    candidate = path.expanduser()
    if not candidate.is_absolute():
        candidate = ROOT / candidate
    if candidate.name in ("", ".", ".."):
        raise ValueError("Android candidate output directory名が不正")
    _assert_no_reparse_ancestors(candidate.parent)
    parent = candidate.parent.resolve(strict=True)
    if not parent.is_dir():
        raise ValueError("Android candidate outputの親は通常directoryでなければならない")
    resolved = parent / candidate.name
    if resolved.exists() or resolved.is_symlink():
        raise ValueError("Android candidate output directoryが既に存在する。上書きしない")
    return _reject_repository_or_sync_path(resolved, "Android candidate output")


def _require_clean_source(expected_commit: str) -> str:
    if COMMIT_PATTERN.fullmatch(expected_commit) is None:
        raise ValueError("source commitは40桁のlowercase SHA-1でなければならない")
    try:
        head = subprocess.run(
            ["git", "-C", str(ROOT), "rev-parse", "HEAD"],
            check=True,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=15,
        ).stdout.strip()
        dirty = subprocess.run(
            ["git", "-C", str(ROOT), "status", "--porcelain", "--untracked-files=all"],
            check=True,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=15,
        ).stdout
    except (OSError, subprocess.SubprocessError) as error:
        raise ValueError("source commitとworking tree状態を確認できない") from error
    if head != expected_commit or dirty:
        raise ValueError("candidate source commitがHEADと不一致、またはsourceに未commit差分がある")
    return head


def _application_identity() -> tuple[str, str, int]:
    gradle = ROOT / "apps/mobile_flutter/android/app/build.gradle.kts"
    pubspec = ROOT / "apps/mobile_flutter/pubspec.yaml"
    try:
        gradle_text = gradle.read_text(encoding="utf-8")
        pubspec_text = pubspec.read_text(encoding="utf-8")
    except OSError as error:
        raise ValueError("Android application identity sourceを読めない") from error
    ids = re.findall(r"(?m)^\s*applicationId\s*=\s*\"([^\"]+)\"\s*$", gradle_text)
    versions = re.findall(r"(?m)^version:\s*([A-Za-z0-9.+_-]+)\s*$", pubspec_text)
    if len(ids) != 1 or APP_ID_PATTERN.fullmatch(ids[0]) is None or len(versions) != 1:
        raise ValueError("Android sourceのapplication IDまたはversionを一意に確定できない")
    version = versions[0]
    if VERSION_PATTERN.fullmatch(version) is None:
        raise ValueError("pubspec versionの形式が不正")
    name, plus, build = version.partition("+")
    if VERSION_PATTERN.fullmatch(name) is None or (plus and not build.isdecimal()):
        raise ValueError("pubspec versionのname/build numberが不正")
    return ids[0], name, int(build) if plus else 1


def _require_build_apk(
    path: Path,
    description: str,
    *,
    expected_names: frozenset[str],
) -> Path:
    candidate = path.expanduser().absolute()
    expected_root = (ROOT / "apps/mobile_flutter/build/app/outputs/flutter-apk").resolve(
        strict=False
    )
    _assert_no_reparse_ancestors(candidate)
    metadata = candidate.lstat()
    if (
        not stat.S_ISREG(metadata.st_mode)
        or candidate.is_symlink()
        or _is_reparse_point(metadata)
        or metadata.st_size < 1
        or metadata.st_size > MAX_APK_BYTES
    ):
        raise ValueError(f"{description}はsize範囲内の通常APK fileではない")
    if candidate.name not in expected_names:
        raise ValueError(f"{description}が期待するFlutter APK output名ではない")
    resolved = candidate.resolve(strict=True)
    if not _path_is_within(resolved, expected_root):
        raise ValueError(f"{description}がFlutter Android build output外にある")
    return resolved


def _find_android_tool(name: str, explicit: Path | None) -> Path:
    candidates: list[Path] = []
    if explicit is not None:
        candidates.append(explicit.expanduser())
    located = shutil.which(name)
    if located:
        candidates.append(Path(located))
    sdk = os.environ.get("ANDROID_HOME") or os.environ.get("ANDROID_SDK_ROOT")
    if sdk:
        build_tools = Path(sdk) / "build-tools"
        try:
            versions = sorted(
                (p for p in build_tools.iterdir() if p.is_dir()),
                key=lambda p: tuple(int(x) if x.isdecimal() else -1 for x in p.name.split(".")),
                reverse=True,
            )
        except OSError:
            versions = []
        suffixes = (".exe", ".bat", "") if os.name == "nt" else ("",)
        candidates.extend(version / f"{name}{suffix}" for version in versions for suffix in suffixes)
    for candidate in candidates:
        if os.name == "nt" and name == "apksigner" and candidate.suffix.lower() in {".bat", ".cmd"}:
            candidate = candidate.parent / "lib" / "apksigner.jar"
        try:
            resolved = candidate.resolve(strict=True)
            metadata = resolved.lstat()
        except OSError:
            continue
        if stat.S_ISREG(metadata.st_mode) and not resolved.is_symlink() and not _is_reparse_point(metadata):
            return resolved
    raise ValueError(f"Android SDK {name} toolが見つからない")


def _tool_build_tools_version(aapt: Path, apksigner: Path, expected: str) -> None:
    apksigner_home = apksigner.parent.parent if apksigner.name == "apksigner.jar" else apksigner.parent
    if aapt.parent != apksigner_home or aapt.parent.name != expected:
        raise ValueError("aapt/apksignerが指定Android Build Tools versionと一致しない")


def _certificate_dn_parts(value: str) -> frozenset[str]:
    return frozenset(
        part.strip().replace(" ", "").lower()
        for part in value.split(",")
        if part.strip()
    )


def _run_tool(command: list[str], *, label: str) -> subprocess.CompletedProcess[str]:
    executable = Path(command[0])
    if executable.suffix.lower() == ".jar":
        java = shutil.which("java")
        if not java:
            raise ValueError(f"{label}のJAR起動に必要なJavaがない")
        command = [java, "-jar", *command]
    try:
        return subprocess.run(
            command,
            check=False,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=45,
        )
    except (OSError, subprocess.SubprocessError) as error:
        raise ValueError(f"{label}を実行できない") from error


def _apk_digest(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as apk:
        for chunk in iter(lambda: apk.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _signature_metadata(archive: zipfile.ZipFile, path: Path) -> bool:
    for info in archive.infolist():
        name = info.filename.replace("\\", "/")
        if name.upper().startswith("META-INF/") and Path(name).suffix.upper() in {
            ".SF",
            ".RSA",
            ".DSA",
            ".EC",
        }:
            return True
    try:
        with path.open("rb") as source:
            source.seek(archive.start_dir - len(APK_SIGNING_BLOCK_MAGIC))
            return source.read(len(APK_SIGNING_BLOCK_MAGIC)) == APK_SIGNING_BLOCK_MAGIC
    except (OSError, ValueError):
        return True


def _archive_entries(archive: zipfile.ZipFile) -> tuple[set[str], tuple[str, ...]]:
    entries: set[str] = set()
    abis: set[str] = set()
    for info in archive.infolist():
        name = info.filename
        if (
            not name
            or name.startswith(("/", "\\"))
            or "\\" in name
            or len(name.encode("utf-8")) > 1024
            or any(part in {"", ".", ".."} for part in name.rstrip("/").split("/"))
            or name in entries
        ):
            raise ValueError("APK内に重複または不正なentry pathがある")
        entries.add(name)
        parts = name.split("/")
        if len(parts) >= 3 and parts[0] == "lib" and parts[1]:
            abis.add(parts[1])
    if len(entries) > MAX_APK_ENTRIES:
        raise ValueError("APK entry数が上限を超過")
    if not {"AndroidManifest.xml", "classes.dex"}.issubset(entries):
        raise ValueError("APKにcompiled Android ManifestまたはDEXがない")
    return entries, tuple(sorted(abis))


def _inspect_apk(
    path: Path,
    *,
    variant: str,
    expected_app_id: str,
    expected_version_name: str,
    expected_version_code: int,
    aapt: Path,
    apksigner: Path,
) -> ApkFacts:
    metadata = path.lstat()
    if (
        not stat.S_ISREG(metadata.st_mode)
        or path.is_symlink()
        or _is_reparse_point(metadata)
        or metadata.st_size < 1
        or metadata.st_size > MAX_APK_BYTES
    ):
        raise ValueError(f"{variant} APKが通常fileまたはsize上限に適合しない")
    try:
        with zipfile.ZipFile(path) as archive:
            _, native_abis = _archive_entries(archive)
            if archive.testzip() is not None:
                raise ValueError(f"{variant} APK ZIP CRC検査に失敗")
            has_signature = _signature_metadata(archive, path)
    except (OSError, zipfile.BadZipFile, zipfile.LargeZipFile) as error:
        raise ValueError(f"{variant} APK archiveを検証できない") from error

    package_result = _run_tool([str(aapt), "dump", "badging", str(path)], label="aapt")
    matches = PACKAGE_LINE.findall(package_result.stdout) if package_result.returncode == 0 else []
    if len(matches) != 1:
        raise ValueError(f"{variant} APKのapplication identityを読めない")
    application_id, version_code_text, version_name = matches[0]
    if (
        application_id != expected_app_id
        or not version_code_text.isdecimal()
        or int(version_code_text) != expected_version_code
        or version_name != expected_version_name
    ):
        raise ValueError(f"{variant} APK identity/versionがAndroid sourceと一致しない")

    signature = _run_tool(
        [str(apksigner), "verify", "--verbose", "--print-certs", str(path)],
        label="apksigner",
    )
    combined = f"{signature.stdout}\n{signature.stderr}"
    if variant == "debug":
        signer_dns = re.findall(
            r"(?mi)^(?:Signer #[0-9]+\s+|V[0-9]+(?:\.[0-9]+)? Signer:\s+)"
            r"certificate DN:\s*(.+?)\s*$",
            combined,
        )
        normalized_dns = [_certificate_dn_parts(dn) for dn in signer_dns]
        if signature.returncode != 0 or normalized_dns != [DEBUG_CERTIFICATE_DN]:
            raise ValueError("Emulator APKがAndroid debug signingとして検証できない")
        signature_state = "debug_signed"
    else:
        diagnostic = f"{signature.stdout}\n{signature.stderr}"
        if has_signature or signature.returncode == 0:
            raise ValueError("Release candidateは署名なしでなければならない")
        if not UNSIGNED_APK_DIAGNOSTIC.search(diagnostic):
            raise ValueError("Release APKの署名状態を安全に分類できない")
        idsig = path.with_name(path.name + ".idsig")
        if idsig.exists() or idsig.is_symlink():
            raise ValueError("Release candidateに署名sidecarがある")
        signature_state = "unsigned"

    return ApkFacts(
        path=path,
        sha256=_apk_digest(path),
        size_bytes=metadata.st_size,
        application_id=application_id,
        version_code=int(version_code_text),
        version_name=version_name,
        native_abis=native_abis,
        signature_state=signature_state,
    )


def _copy_stable(source: Path, destination: Path) -> None:
    before = source.lstat()
    digest = hashlib.sha256()
    size = 0
    with source.open("rb") as input_file, destination.open("xb") as output_file:
        for chunk in iter(lambda: input_file.read(1024 * 1024), b""):
            output_file.write(chunk)
            digest.update(chunk)
            size += len(chunk)
        output_file.flush()
        os.fsync(output_file.fileno())
    after = source.lstat()
    stable_before = (before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns)
    stable_after = (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns)
    if stable_before != stable_after or size != before.st_size or _apk_digest(source) != digest.hexdigest():
        raise ValueError("APK build outputがcopy中に変化した")


def package_android_product_candidate(
    debug_apk: Path,
    release_apk: Path,
    output_dir: Path,
    *,
    source_commit: str,
    flutter_version: str,
    java_version: str,
    android_build_tools_version: str,
    aapt_path: Path | None = None,
    apksigner_path: Path | None = None,
) -> dict[str, Any]:
    source = _require_clean_source(source_commit)
    debug_source = _require_build_apk(
        debug_apk,
        "Emulator Debug APK",
        expected_names=frozenset({"app-debug.apk"}),
    )
    release_source = _require_build_apk(
        release_apk,
        "Release APK",
        expected_names=frozenset({"app-release.apk", "app-release-unsigned.apk"}),
    )
    if debug_source == release_source:
        raise ValueError("Debug APKとRelease APKが同一file")
    output = _output_target(output_dir)
    app_id, version_name, version_code = _application_identity()
    for label, value in (
        ("Flutter", flutter_version),
        ("Java", java_version),
        ("Android Build Tools", android_build_tools_version),
    ):
        if not value or len(value) > 128 or any(ord(char) < 32 for char in value):
            raise ValueError(f"{label} version evidenceが不正")
    aapt = _find_android_tool("aapt", aapt_path)
    apksigner = _find_android_tool("apksigner", apksigner_path)
    _tool_build_tools_version(aapt, apksigner, android_build_tools_version)

    with tempfile.TemporaryDirectory(prefix="d4p-android-candidate-", dir=output.parent) as temporary:
        stage = Path(temporary) / "published"
        stage.mkdir()
        staged_debug = stage / "d4-pocket-android-emulator-debug.apk"
        staged_release = stage / "d4-pocket-android-release-unsigned.apk"
        _copy_stable(debug_source, staged_debug)
        _copy_stable(release_source, staged_release)
        debug = _inspect_apk(
            staged_debug,
            variant="debug",
            expected_app_id=app_id,
            expected_version_name=version_name,
            expected_version_code=version_code,
            aapt=aapt,
            apksigner=apksigner,
        )
        release = _inspect_apk(
            staged_release,
            variant="release",
            expected_app_id=app_id,
            expected_version_name=version_name,
            expected_version_code=version_code,
            aapt=aapt,
            apksigner=apksigner,
        )
        if debug.sha256 == release.sha256:
            raise ValueError("Debug APKとRelease APKのhashが同じ")
        if debug.application_id != release.application_id:
            raise ValueError("Debug APKとRelease APKのapplication IDが異なる")
        if _require_clean_source(source_commit) != source:
            raise ValueError("APK検査中にsource identityが変化した")

        record: dict[str, Any] = {
            "version": 1,
            "evidence_type": "d4_pocket_android_product_candidate",
            "evidence_source": "INTERNAL_STATE",
            "source_commit": source,
            "source_clean": True,
            "application_id": app_id,
            "application_identity_confirmed": False,
            "version_code": version_code,
            "version_name": version_name,
            "toolchain": {
                "flutter": flutter_version,
                "java": java_version,
                "android_build_tools": android_build_tools_version,
            },
            "artifacts": {
                "emulator_debug": {
                    "file_name": staged_debug.name,
                    "sha256": debug.sha256,
                    "size_bytes": debug.size_bytes,
                    "build_configuration": "debug",
                    "signature_state": debug.signature_state,
                    "native_abis": list(debug.native_abis),
                },
                "release_candidate": {
                    "file_name": staged_release.name,
                    "sha256": release.sha256,
                    "size_bytes": release.size_bytes,
                    "build_configuration": "release",
                    "signature_state": release.signature_state,
                    "native_abis": list(release.native_abis),
                },
            },
            "distribution_ready": False,
            "formal_distribution_claimed": False,
            "publisher_identity_included": False,
            "credential_included": False,
            "installed_product_verified": False,
        }
        errors = validate_instance(record, json.loads(SCHEMA_PATH.read_text(encoding="utf-8")))
        if errors:
            raise ValueError("Android product candidate evidenceが現行Schemaに適合しない")
        evidence_path = stage / "android-product-candidate.json"
        with evidence_path.open("xb") as evidence:
            evidence.write((json.dumps(record, ensure_ascii=False, indent=2) + "\n").encode("utf-8"))
            evidence.flush()
            os.fsync(evidence.fileno())
        os.rename(stage, output)

    return {
        "source_commit": source,
        "application_id": app_id,
        "version_name": version_name,
        "debug_apk_sha256": debug.sha256,
        "release_apk_sha256": release.sha256,
        "release_signature_state": release.signature_state,
        "distribution_ready": False,
        "formal_distribution_claimed": False,
    }


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Flutter Android Debug／署名なしRelease APKを分離して候補化する。"
    )
    parser.add_argument("--debug-apk", type=Path, required=True)
    parser.add_argument("--release-apk", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--flutter-version", required=True)
    parser.add_argument("--java-version", required=True)
    parser.add_argument("--android-build-tools-version", required=True)
    parser.add_argument("--aapt", type=Path)
    parser.add_argument("--apksigner", type=Path)
    args = parser.parse_args()
    try:
        result = package_android_product_candidate(
            args.debug_apk,
            args.release_apk,
            args.output_dir,
            source_commit=args.source_commit,
            flutter_version=args.flutter_version,
            java_version=args.java_version,
            android_build_tools_version=args.android_build_tools_version,
            aapt_path=args.aapt,
            apksigner_path=args.apksigner,
        )
    except (OSError, ValueError, subprocess.SubprocessError, zipfile.BadZipFile) as error:
        print(f"Android product candidateの公開を停止した: {error}", file=sys.stderr)
        return 1
    print(json.dumps(result, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
