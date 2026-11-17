"""Build reproducible iOS Simulator and non-distribution device-app archives for QA.

This developer tool does not create a signed IPA or a distribution-ready app.
It records two distinct Apple platform builds from one source commit while
keeping publisher identity and signing credentials out of the repository.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import plistlib
import re
import shutil
import stat
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
SCHEMA_PATH = ROOT / "specs" / "gui_shell_ios_product_candidate.schema.json"
EXPECTED_BUNDLE_NAME = "Runner.app"
MAX_BUNDLE_BYTES = 2_000_000_000
MAX_BUNDLE_ENTRIES = 100_000
COMMIT_PATTERN = re.compile(r"^[0-9a-f]{40}$")
VERSION_PATTERN = re.compile(r"^[A-Za-z0-9][A-Za-z0-9.+_-]{0,63}$")

if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from tooling.schema_check.check_schemas import validate_instance  # noqa: E402
from tooling.mobile_product_candidate_metadata import device_link_schema_evidence  # noqa: E402


@dataclass(frozen=True)
class BundleFacts:
    path: Path
    bundle_identifier: str
    version_name: str
    version_code: str
    executable_sha256: str
    tree_sha256: str
    entry_count: int
    total_bytes: int
    architectures: tuple[str, ...]
    platform: str
    build_configuration: str
    signature_state: str
    build_settings: dict[str, Any]
    privacy_usage_keys: tuple[str, ...]


def _sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _require_clean_source(expected_commit: str) -> str:
    """Bind source inputs while allowing only untracked Apple workflow evidence files."""
    if COMMIT_PATTERN.fullmatch(expected_commit) is None:
        raise ValueError("iOS candidate source commitがfull SHA-1ではない")
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
        status_output = subprocess.run(
            [
                "git",
                "-C",
                str(ROOT),
                "status",
                "--porcelain=v1",
                "-z",
                "--untracked-files=all",
            ],
            check=True,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=15,
        ).stdout
    except (OSError, subprocess.SubprocessError) as error:
        raise ValueError("iOS candidate source commitとworking tree状態を確認できない") from error
    dirty_entries = []
    for entry in status_output.split("\0"):
        if not entry:
            continue
        status = entry[:2]
        path = entry[3:] if len(entry) > 3 else ""
        if status == "??" and path.replace("\\", "/").startswith("evidence/"):
            continue
        dirty_entries.append(entry)
    if head != expected_commit or dirty_entries:
        raise ValueError("iOS candidate source commitがHEADと不一致、またはsourceに未commit差分がある")
    return head


def _source_entitlement_declarations() -> dict[str, Any]:
    path = ROOT / "apps/mobile_flutter/ios/Runner/Runner.entitlements"
    try:
        metadata = path.lstat()
    except OSError as error:
        raise ValueError("iOS entitlement sourceを確認できない") from error
    if not stat.S_ISREG(metadata.st_mode) or stat.S_ISLNK(metadata.st_mode) or metadata.st_size > 64_000:
        raise ValueError("iOS entitlement sourceが通常fileまたはsize上限に適合しない")
    try:
        with path.open("rb") as handle:
            value = plistlib.load(handle)
    except (OSError, plistlib.InvalidFileException) as error:
        raise ValueError("iOS entitlement sourceを読めない") from error
    if not isinstance(value, dict) or any(
        not isinstance(key, str) or re.fullmatch(r"[A-Za-z0-9.-]{1,128}", key) is None
        for key in value
    ):
        raise ValueError("iOS entitlement sourceの構造が不正")
    return {
        "source_sha256": _sha256_file(path),
        "declared_keys": sorted(value),
    }


def _inspect_app_bundle_signature(bundle: Path) -> str:
    codesign = shutil.which("codesign")
    if not codesign:
        raise ValueError("iOS candidate確認に必要なmacOS codesignがない")
    result = subprocess.run(
        [codesign, "--display", "--verbose=4", str(bundle)],
        check=False,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    diagnostic = f"{result.stdout}\n{result.stderr}".lower()
    if result.returncode == 0:
        ad_hoc_signature = re.search(r"(?m)^signature=adhoc\s*$", diagnostic)
        no_team_identity = re.search(r"(?m)^teamidentifier=not set\s*$", diagnostic)
        has_authority = re.search(r"(?m)^authority=", diagnostic)
        if ad_hoc_signature and no_team_identity and not has_authority:
            return "adhoc_no_identity"
        raise ValueError("iOS app bundleにPublisher/developer identityまたは未分類signatureがある")
    if "code object is not signed at all" not in diagnostic:
        raise ValueError("iOS app bundleのcode signature状態を分類できない")
    return "unsigned"


def _safe_app_bundle(
    path: Path,
    *,
    expected_platform: str,
    build_configuration: str,
) -> BundleFacts:
    input_metadata = path.lstat()
    if not stat.S_ISDIR(input_metadata.st_mode) or stat.S_ISLNK(input_metadata.st_mode):
        raise ValueError("iOS app bundle rootが通常directoryではない")
    bundle = path.resolve(strict=True)
    metadata = bundle.lstat()
    if not stat.S_ISDIR(metadata.st_mode) or stat.S_ISLNK(metadata.st_mode):
        raise ValueError("iOS app bundle rootが通常directoryではない")
    if bundle.name != EXPECTED_BUNDLE_NAME:
        raise ValueError("iOS product targetのapp bundle名がRunner.appではない")
    signature_state = _inspect_app_bundle_signature(bundle)

    info_path = bundle / "Info.plist"
    info_metadata = info_path.lstat()
    if not stat.S_ISREG(info_metadata.st_mode) or stat.S_ISLNK(info_metadata.st_mode):
        raise ValueError("iOS app Info.plistが通常fileではない")
    try:
        with info_path.open("rb") as handle:
            info = plistlib.load(handle)
    except (OSError, plistlib.InvalidFileException) as error:
        raise ValueError("iOS app Info.plistを読めない") from error
    if not isinstance(info, dict):
        raise ValueError("iOS app Info.plistのrootがobjectではない")
    bundle_identifier = info.get("CFBundleIdentifier")
    executable_name = info.get("CFBundleExecutable")
    version_name = info.get("CFBundleShortVersionString")
    version_code = info.get("CFBundleVersion")
    if (
        not isinstance(bundle_identifier, str)
        or re.fullmatch(r"[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+){1,7}", bundle_identifier)
        is None
        or not isinstance(executable_name, str)
        or re.fullmatch(r"[A-Za-z0-9._-]{1,128}", executable_name) is None
        or not isinstance(version_name, str)
        or VERSION_PATTERN.fullmatch(version_name) is None
        or not isinstance(version_code, str)
        or VERSION_PATTERN.fullmatch(version_code) is None
    ):
        raise ValueError("iOS app identity、versionまたは実行file名が不正")

    platform_name = info.get("DTPlatformName")
    sdk_name = info.get("DTSDKName")
    sdk_version = info.get("DTPlatformVersion")
    minimum_os_version = info.get("MinimumOSVersion")
    supported_platforms = info.get("CFBundleSupportedPlatforms")
    device_families = info.get("UIDeviceFamily")
    expected_platform_name = "iphonesimulator" if expected_platform == "IOSSIMULATOR" else "iphoneos"
    expected_supported_platform = "iPhoneSimulator" if expected_platform == "IOSSIMULATOR" else "iPhoneOS"
    if (
        platform_name != expected_platform_name
        or not isinstance(sdk_name, str)
        or re.fullmatch(r"(?:iphoneos|iphonesimulator)[0-9]+(?:\.[0-9]+){0,2}", sdk_name) is None
        or not sdk_name.startswith(expected_platform_name)
        or not isinstance(sdk_version, str)
        or re.fullmatch(r"[0-9]+(?:\.[0-9]+){0,2}", sdk_version) is None
        or not isinstance(minimum_os_version, str)
        or re.fullmatch(r"[0-9]+(?:\.[0-9]+){0,2}", minimum_os_version) is None
        or not isinstance(supported_platforms, list)
        or supported_platforms != [expected_supported_platform]
        or not isinstance(device_families, list)
        or not device_families
        or any(type(value) is not int or value not in {1, 2} for value in device_families)
        or len(set(device_families)) != len(device_families)
    ):
        raise ValueError("iOS artifactの実測build settingsがplatform契約に適合しない")
    privacy_usage_keys: list[str] = []
    for key, value in info.items():
        if isinstance(key, str) and key.startswith("NS") and key.endswith("UsageDescription"):
            if not isinstance(value, str) or not value.strip():
                raise ValueError("iOS native permissionのusage descriptionが空または不正")
            privacy_usage_keys.append(key)
    executable = bundle / executable_name
    executable_metadata = executable.lstat()
    if not stat.S_ISREG(executable_metadata.st_mode) or stat.S_ISLNK(executable_metadata.st_mode):
        raise ValueError("iOS app executableが通常fileではない")

    xcrun = shutil.which("xcrun")
    if not xcrun:
        raise ValueError("iOS candidate確認に必要なXcode xcrunがない")
    result = subprocess.run(
        [xcrun, "lipo", "-archs", str(executable)],
        check=True,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    architectures = tuple(sorted(set(result.stdout.strip().split())))
    if not architectures or "arm64" not in architectures:
        raise ValueError("iOS app executableにarm64 sliceがない")
    platform_result = subprocess.run(
        [xcrun, "vtool", "-show-build", str(executable)],
        check=True,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    platform_lines = [
        line.strip()
        for line in platform_result.stdout.splitlines()
        if re.search(r"\bplatform\b", line, flags=re.IGNORECASE)
    ]
    platforms = [
        value.upper()
        for value in re.findall(r"(?mi)^\s*platform\s+([A-Za-z0-9_]+)", platform_result.stdout)
    ]
    observed_platforms = sorted(set(platforms))
    if not observed_platforms or observed_platforms != [expected_platform]:
        observed = ",".join(observed_platforms) if observed_platforms else "unrecognized"
        safe_lines = [
            re.sub(r"[^A-Za-z0-9_ .()=:-]", "?", line)[:160]
            for line in platform_lines[:8]
        ]
        detail = "|".join(safe_lines) if safe_lines else "no platform lines"
        raise ValueError(
            f"iOS app executable platformが不一致: expected {expected_platform}; "
            f"observed={observed}; output={detail}"
        )

    build_settings: dict[str, Any] = {
        "target": "Runner",
        "configuration": build_configuration,
        "platform": expected_platform,
        "bundle_identifier": bundle_identifier,
        "version_name": version_name,
        "version_code": version_code,
        "sdk": sdk_name,
        "sdk_version": sdk_version,
        "minimum_os_version": minimum_os_version,
        "device_families": device_families,
        "architectures": list(architectures),
    }
    build_settings["sha256"] = hashlib.sha256(
        json.dumps(build_settings, sort_keys=True, separators=(",", ":")).encode("ascii")
    ).hexdigest()

    digest = hashlib.sha256()
    entry_count = 0
    total_bytes = 0
    for current, directories, files in os.walk(bundle, topdown=True, followlinks=False):
        current_path = Path(current)
        directories.sort()
        files.sort()
        for name in tuple(directories):
            candidate = current_path / name
            if candidate.is_symlink():
                directories.remove(name)
                _hash_symlink(candidate, bundle, digest)
                entry_count += 1
        for name in files:
            candidate = current_path / name
            relative = candidate.relative_to(bundle).as_posix()
            if name == "embedded.mobileprovision":
                raise ValueError("iOS non-distribution candidateにprovisioning profileがある")
            metadata = candidate.lstat()
            if stat.S_ISLNK(metadata.st_mode):
                _hash_symlink(candidate, bundle, digest)
            elif stat.S_ISREG(metadata.st_mode):
                total_bytes += metadata.st_size
                digest.update(b"F\0" + relative.encode("utf-8") + b"\0")
                digest.update(bytes.fromhex(_sha256_file(candidate)))
            else:
                raise ValueError("iOS app bundleにspecial fileがある")
            entry_count += 1
            if entry_count > MAX_BUNDLE_ENTRIES or total_bytes > MAX_BUNDLE_BYTES:
                raise ValueError("iOS app bundleがfile数または容量上限を超過")

    if entry_count == 0 or total_bytes == 0:
        raise ValueError("iOS app bundleが空または実行fileを含まない")
    return BundleFacts(
        path=bundle,
        bundle_identifier=bundle_identifier,
        version_name=version_name,
        version_code=version_code,
        executable_sha256=_sha256_file(executable),
        tree_sha256=digest.hexdigest(),
        entry_count=entry_count,
        total_bytes=total_bytes,
        architectures=architectures,
        platform=expected_platform,
        build_configuration=build_configuration,
        signature_state=signature_state,
        build_settings=build_settings,
        privacy_usage_keys=tuple(sorted(privacy_usage_keys)),
    )


def _hash_symlink(path: Path, root: Path, digest: Any) -> None:
    relative = path.relative_to(root).as_posix()
    target = os.readlink(path)
    if Path(target).is_absolute():
        raise ValueError("iOS app bundleにabsolute symlinkがある")
    try:
        resolved = (path.parent / target).resolve(strict=True)
    except OSError as error:
        raise ValueError("iOS app bundle symlinkの参照先を解決できない") from error
    try:
        resolved.relative_to(root)
    except ValueError as error:
        raise ValueError("iOS app bundle symlinkがroot外を指す") from error
    digest.update(b"L\0" + relative.encode("utf-8") + b"\0")
    digest.update(target.encode("utf-8"))


def _archive_app(bundle: BundleFacts, destination: Path) -> None:
    ditto = shutil.which("ditto")
    if not ditto:
        raise ValueError("iOS candidate archiveに必要なmacOS dittoがない")
    subprocess.run(
        [ditto, "-c", "-k", "--sequesterRsrc", "--keepParent", str(bundle.path), str(destination)],
        check=True,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    if not destination.is_file() or destination.stat().st_size == 0:
        raise ValueError("iOS app bundle archiveを生成できない")


def _artifact_record(path: Path, bundle: BundleFacts) -> dict[str, Any]:
    return {
        "file_name": path.name,
        "sha256": _sha256_file(path),
        "size_bytes": path.stat().st_size,
        "bundle_identifier": bundle.bundle_identifier,
        "version_name": bundle.version_name,
        "version_code": bundle.version_code,
        "main_executable_sha256": bundle.executable_sha256,
        "bundle_tree_sha256": bundle.tree_sha256,
        "bundle_file_count": bundle.entry_count,
        "bundle_uncompressed_bytes": bundle.total_bytes,
        "architectures": list(bundle.architectures),
        "platform": bundle.platform,
        "build_configuration": bundle.build_configuration,
        "build_settings": bundle.build_settings,
        "privacy_usage_keys": list(bundle.privacy_usage_keys),
        "app_bundle_signature_state": bundle.signature_state,
        "signature_inspection": "codesign_display",
        "provisioning_profile_present": False,
    }


def _validate_output_directory(output_dir: Path) -> tuple[Path, Path]:
    output = output_dir.absolute()
    parent = output.parent.resolve(strict=True)
    try:
        output.resolve(strict=False).relative_to(ROOT)
    except ValueError:
        pass
    else:
        raise ValueError("iOS candidate outputはRepository外に置く")
    if output.exists() or output.is_symlink():
        raise ValueError("iOS candidate output directoryが既に存在する")
    return output, parent


def build_ios_product_candidate(
    simulator_app: Path,
    device_app: Path,
    output_dir: Path,
    *,
    source_commit: str,
    flutter_version: str,
    xcode_version: str,
) -> dict[str, Any]:
    if platform.system() != "Darwin":
        raise ValueError("iOS product candidate生成にはmacOS native build hostが必要")
    source = _require_clean_source(source_commit)
    if VERSION_PATTERN.fullmatch(flutter_version) is None or VERSION_PATTERN.fullmatch(xcode_version) is None:
        raise ValueError("iOS candidate toolchain versionが不正")

    output, parent = _validate_output_directory(output_dir)
    simulator = _safe_app_bundle(
        simulator_app,
        expected_platform="IOSSIMULATOR",
        build_configuration="debug",
    )
    device = _safe_app_bundle(
        device_app,
        expected_platform="IOS",
        build_configuration="release",
    )
    if simulator.bundle_identifier != device.bundle_identifier:
        raise ValueError("Simulatorとdevice appのBundle Identifierが異なる")
    if (simulator.version_name, simulator.version_code) != (device.version_name, device.version_code):
        raise ValueError("Simulatorとdevice appのversionが異なる")
    if simulator.executable_sha256 == device.executable_sha256:
        raise ValueError("Simulatorとdevice appの実行binaryが同一")

    stage = Path(tempfile.mkdtemp(prefix="d4p-ios-candidate-", dir=parent))
    try:
        simulator_archive = stage / "d4-pocket-ios-simulator-app.zip"
        device_archive = stage / "d4-pocket-ios-device-app.zip"
        _archive_app(simulator, simulator_archive)
        _archive_app(device, device_archive)
        simulator_artifact = _artifact_record(simulator_archive, simulator)
        device_artifact = _artifact_record(device_archive, device)
        if (
            simulator_artifact["sha256"] == device_artifact["sha256"]
            or simulator.executable_sha256 == device.executable_sha256
        ):
            raise ValueError("iOS Simulatorとdevice artifactのbinary identityが分離されていない")
        entitlement_declarations = _source_entitlement_declarations()
        device_link_contract = device_link_schema_evidence(ROOT)
        evidence: dict[str, Any] = {
            "schema_version": 1,
            "evidence_type": "d4_pocket_ios_product_build_candidate",
            "evidence_source": "INTERNAL_STATE",
            "source_commit": source,
            "source_clean": True,
            "channel": "qa_candidate",
            "toolchain": {
                "flutter_version": flutter_version,
                "xcode_version": xcode_version,
                "host_platform": "macos",
            },
            "bundle_identity": {
                "bundle_identifier": simulator.bundle_identifier,
                "version_name": simulator.version_name,
                "version_code": simulator.version_code,
                "owner_confirmed": False,
                "distribution_identity": False,
            },
            "native_permissions": {
                "evidence_source": "built_info_plist_and_source_entitlements",
                "simulator_usage_keys": list(simulator.privacy_usage_keys),
                "device_usage_keys": list(device.privacy_usage_keys),
                "source_entitlements_sha256": entitlement_declarations["source_sha256"],
                "source_entitlement_keys": entitlement_declarations["declared_keys"],
                "applied_entitlements_observed": False,
            },
            "device_link_contract": device_link_contract,
            "simulator_artifact": simulator_artifact,
            "device_artifact": device_artifact,
            "relationship": {
                "same_source_commit": True,
                "same_bundle_identifier": True,
                "distinct_artifact_hashes": True,
                "distinct_main_executable_hashes": True,
            },
            "distribution_ready": False,
            "formal_distribution_claimed": False,
            "installed_product_verified": False,
        }
        errors = validate_instance(evidence, json.loads(SCHEMA_PATH.read_text(encoding="utf-8")))
        if errors:
            raise ValueError("iOS candidate evidenceがSchema不適合: " + "; ".join(errors[:8]))
        if _require_clean_source(source_commit) != source:
            raise ValueError("iOS candidate source identity changed during packaging")
        (stage / "ios-product-candidate.json").write_text(
            json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
        )
        os.rename(stage, output)
        return evidence
    except BaseException:
        shutil.rmtree(stage, ignore_errors=True)
        raise


def main() -> int:
    parser = argparse.ArgumentParser(description="iOS Simulator／非配布device buildを別artifactとして記録する")
    parser.add_argument("--simulator-app", type=Path, required=True)
    parser.add_argument("--device-app", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--source-commit", default=os.environ.get("GITHUB_SHA", ""))
    parser.add_argument("--flutter-version", required=True)
    parser.add_argument("--xcode-version", required=True)
    args = parser.parse_args()
    try:
        evidence = build_ios_product_candidate(
            args.simulator_app,
            args.device_app,
            args.output_dir,
            source_commit=args.source_commit,
            flutter_version=args.flutter_version,
            xcode_version=args.xcode_version,
        )
    except (OSError, ValueError, subprocess.CalledProcessError, json.JSONDecodeError) as error:
        print(f"iOS product candidateを停止した: {type(error).__name__}: {error}", file=sys.stderr)
        return 1
    print(json.dumps({
        "source_commit": evidence["source_commit"],
        "simulator_artifact_sha256": evidence["simulator_artifact"]["sha256"],
        "device_artifact_sha256": evidence["device_artifact"]["sha256"],
        "bundle_identifier": evidence["bundle_identity"]["bundle_identifier"],
        "distribution_ready": False,
        "formal_distribution_claimed": False,
    }, ensure_ascii=False, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
