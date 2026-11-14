"""Broker Export ReceiptからPublisher identityなしのmacOS App Bundle／pkg候補を構築する開発tool。"""

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
import tarfile
import tempfile
import time
import traceback
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from tooling.build_module_pruned_windows import (
    MODULE_DEFINE_NAMES,
    _load_catalog,
    dart_defines,
)
from tooling.export_windows_product import (
    CREDENTIAL_PATTERNS,
    CREDENTIAL_SCAN_CHUNK_BYTES,
    CREDENTIAL_SCAN_OVERLAP_BYTES,
    CREDENTIAL_SCAN_STATUS,
    MAX_INPUT_BYTES,
    MAX_MANIFEST_BYTES,
    SENSITIVE_ARTIFACT_NAMES,
    _parse_json_bytes,
    _read_regular_file,
    _resolve_build_temp_root,
    _resolve_output_directory,
    _safe_extract_source,
    _sha256_file,
    _update_trust_summary,
    _write_source_archive,
    product_version_from_source,
    validate_export_inputs,
)
from tooling.schema_check.check_schemas import validate_instance

RECEIPT_NAME = "d4_pocket_macos_export_build.json"
BUILD_TEMP_PREFIX = "d4p-macos-build-"
OUTPUT_TEMP_PREFIX = "d4p-macos-output-"
MACOS_RUNNER_ENTITLEMENTS = Path("apps/desktop_flutter/macos/Runner/Release.entitlements")
MACOS_BROKER_HELPER_ENTITLEMENTS = Path(
    "apps/desktop_flutter/macos/Runner/BrokerHelper.entitlements"
)
EXPECTED_MACOS_APP_ENTITLEMENTS = {
    "com.apple.security.app-sandbox": True,
    "com.apple.security.files.user-selected.read-write": True,
    "com.apple.security.network.client": True,
    "com.apple.security.network.server": True,
}
EXPECTED_MACOS_BROKER_HELPER_ENTITLEMENTS = {
    "com.apple.security.app-sandbox": True,
    "com.apple.security.inherit": True,
}
MACOS_AOT_REPORT_ROOT = "App (Dart AOT)"
MACOS_AOT_IMAGE_RELATIVE_PATH = PurePosixPath(
    "Contents/Frameworks/App.framework/Versions/A/App"
)
MACOS_AOT_FRAMEWORK_RELATIVE_PATH = PurePosixPath(
    "Contents/Frameworks/App.framework"
)
MACOS_AOT_REPORT_SENTINEL = (
    "A summary of your macOS bundle analysis can be found at:"
)
MAX_SIZE_ANALYSIS_BYTES = 50_000_000
RUST_TOOLCHAIN = "1.95.0"
PINNED_FLUTTER_REVISION = "559ffa3f75e7402d65a8def9c28389a9b2e6fe42"
FLUTTER_ENGINE_FRAMEWORK_RELATIVE_PATH = PurePosixPath(
    "bin/cache/artifacts/engine/darwin-x64-release/FlutterMacOS.xcframework/"
    "macos-arm64_x86_64/FlutterMacOS.framework"
)
APP_FLUTTER_FRAMEWORK_RELATIVE_PATH = PurePosixPath(
    "Contents/Frameworks/FlutterMacOS.framework"
)
FLUTTER_MACOS_FRAMEWORK_BINARY_RELATIVE_PATH = PurePosixPath(
    "Versions/A/FlutterMacOS"
)
FLUTTER_MACOS_FRAMEWORK_DENYLISTED_FILES = frozenset(
    {"entitlements.txt", "without_entitlements.txt", "unsigned_binaries.txt"}
)
# Pinned xcode_backend.dart embed filters these exact entry names at every depth.
FLUTTER_MACOS_FRAMEWORK_EMBED_FILTERED_BASENAMES = frozenset(
    {".DS_Store", "Headers", "Modules"}
)
# The pinned Flutter macOS unpack target normalizes copied entry modes.
FLUTTER_RSYNC_DIRECTORY_MODE = 0o755
FLUTTER_RSYNC_FILE_MODE = 0o644
MACOS_ENVIRONMENT_ALLOWLIST = {
    "CARGO_HOME",
    "DEVELOPER_DIR",
    "FLUTTER_ROOT",
    "HOME",
    "LANG",
    "LC_ALL",
    "PATH",
    "RUSTUP_HOME",
    "SDKROOT",
    "TMPDIR",
}


@dataclass(frozen=True)
class FlutterFrameworkReference:
    source_path: Path
    engine_revision: str
    tree_sha256: str
    signature_facts: dict[str, bool]


MACOS_CODE_BUNDLE_SUFFIXES = {
    ".app",
    ".appex",
    ".bundle",
    ".framework",
    ".kext",
    ".plugin",
    ".systemextension",
    ".xpc",
}
MACH_O_MAGICS = {
    b"\xfe\xed\xfa\xce",
    b"\xce\xfa\xed\xfe",
    b"\xfe\xed\xfa\xcf",
    b"\xcf\xfa\xed\xfe",
    b"\xca\xfe\xba\xbe",
    b"\xbe\xba\xfe\xca",
    b"\xca\xfe\xba\xbf",
    b"\xbf\xba\xfe\xca",
}
DARWIN_ARCHITECTURES = {
    "aarch64": "arm64",
    "arm64": "arm64",
    "amd64": "x86_64",
    "x86_64": "x86_64",
}


def _git_value(*arguments: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(ROOT), *arguments],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
    )
    return result.stdout.strip()


def _require_clean_source(*, allow_verification_branch: bool) -> tuple[str, str, str]:
    branch = _git_value("branch", "--show-current")
    if not branch and allow_verification_branch:
        github_ref = os.environ.get("GITHUB_REF", "")
        if github_ref.startswith("refs/heads/"):
            branch = github_ref.removeprefix("refs/heads/")
    if not branch or (branch != "main" and not allow_verification_branch):
        raise ValueError("macOS Exportはmainまたは明示的な手動検証branchから実行する")
    if branch != "main" and (
        re.fullmatch(r"codex/[A-Za-z0-9._/-]+", branch) is None
        or ".." in branch
        or "//" in branch
        or branch.endswith(("/", "."))
    ):
        raise ValueError("手動検証branchはcodex/配下の安全な名前に限る")
    if _git_value("status", "--porcelain", "--untracked-files=all"):
        raise ValueError("macOS Exportはcleanなcommit済みsourceから実行する")
    commit = _git_value("rev-parse", "HEAD")
    remote_ref = f"refs/heads/{branch}"
    remote_lines = _git_value("ls-remote", "origin", remote_ref).splitlines()
    remote_parts = remote_lines[0].split() if len(remote_lines) == 1 else []
    if (
        len(remote_parts) != 2
        or remote_parts[0] != commit
        or remote_parts[1] != remote_ref
    ):
        raise ValueError("macOS Export sourceとGitHub remote branch HEADが一致しない")
    return branch, remote_ref, commit


def bundle_identifier_for_app(app_id: str) -> str:
    prefix = "d4-pocket-app-"
    suffix = app_id.removeprefix(prefix)
    if (
        not app_id.startswith(prefix)
        or len(suffix) != 32
        or any(character not in "0123456789abcdef" for character in suffix)
    ):
        raise ValueError("App IDからmacOS bundle identifierを構成できない")
    return f"com.d4p.export.{suffix}"


def uninstaller_package_identifier_for_app(bundle_identifier: str) -> str:
    if re.fullmatch(r"com\.d4p\.export\.[0-9a-f]{32}", bundle_identifier) is None:
        raise ValueError("macOS uninstaller package identityがExport Appに束縛されていない")
    return f"{bundle_identifier}.uninstaller"


def render_macos_uninstaller_postinstall(
    template: str, *, bundle_name: str, bundle_identifier: str
) -> str:
    match = re.fullmatch(r"com\.d4p\.export\.([0-9a-f]{32})", bundle_identifier)
    if match is None or bundle_name != f"D4Pocket-{match.group(1)[:12]}.app":
        raise ValueError("macOS uninstaller targetがExport App identityと一致しない")
    replacements = {
        "@D4P_APP_BUNDLE_NAME@": bundle_name,
        "@D4P_BUNDLE_IDENTIFIER@": bundle_identifier,
    }
    for token, value in replacements.items():
        if template.count(token) != 1:
            raise ValueError("macOS uninstaller template tokenが一意でない")
        template = template.replace(token, value)
    if "@D4P_" in template:
        raise ValueError("macOS uninstaller templateに未解決tokenがある")
    return template


def _darwin_build_architecture() -> str:
    machine = platform.machine().lower()
    architecture = DARWIN_ARCHITECTURES.get(machine)
    if architecture is None:
        raise ValueError("macOS host architectureが未対応である")
    return architecture


def _macos_environment(temp_root: Path, *, flutter_root: Path) -> dict[str, str]:
    environment = {
        key: value
        for key, value in os.environ.items()
        if key.upper() in MACOS_ENVIRONMENT_ALLOWLIST
    }
    if not environment.get("PATH") or not environment.get("RUSTUP_HOME"):
        raise ValueError("Flutter／Rust固定toolchain環境が不足している")
    for directory in ("home", "cargo", "pub", "tmp"):
        (temp_root / directory).mkdir(mode=0o700)
    environment.update(
        {
            "HOME": str(temp_root / "home"),
            "CARGO_HOME": str(temp_root / "cargo"),
            "PUB_CACHE": str(temp_root / "pub"),
            "TMPDIR": str(temp_root / "tmp"),
            "FLUTTER_ROOT": str(flutter_root),
            "CI": "true",
            "FLUTTER_SUPPRESS_ANALYTICS": "true",
            "FLUTTER_XCODE_CODE_SIGNING_ALLOWED": "NO",
            "FLUTTER_XCODE_CODE_SIGNING_REQUIRED": "NO",
            "FLUTTER_XCODE_CODE_SIGN_IDENTITY": "",
        }
    )
    return environment


def _tool_paths() -> dict[str, str]:
    if sys.platform != "darwin":
        raise ValueError("macOS Product ExportはmacOS native host専用である")
    if platform.machine().lower() not in {"arm64", "aarch64", "x86_64", "amd64"}:
        raise ValueError("macOS host architectureが未対応である")
    tools = (
        "flutter",
        "cargo",
        "rustc",
        "xcodebuild",
        "codesign",
        "lipo",
        "pkgbuild",
        "ditto",
        "plutil",
    )
    result: dict[str, str] = {}
    for name in tools:
        path = shutil.which(name)
        if path is None:
            raise ValueError(f"macOS build toolがない: {name}")
        result[name] = path
    return result


def _run(command: list[str], *, cwd: Path, environment: dict[str, str]) -> str:
    completed = subprocess.run(
        command,
        cwd=cwd,
        env=environment,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        encoding="utf-8",
        errors="replace",
        check=False,
    )
    if completed.stdout:
        sys.stdout.write(completed.stdout)
        sys.stdout.flush()
    if completed.returncode != 0:
        raise subprocess.CalledProcessError(completed.returncode, command)
    return completed.stdout.strip()


def _validated_macos_entitlements_source(
    source_root: Path, relative_path: Path, expected: dict[str, bool], label: str
) -> tuple[Path, str]:
    path = source_root / relative_path
    raw = _read_regular_file(path, 64 * 1024, label)
    try:
        entitlements = plistlib.loads(raw)
    except (plistlib.InvalidFileException, ValueError, TypeError) as error:
        raise ValueError(f"{label}がproperty listとして読めない") from error
    if (
        not isinstance(entitlements, dict)
        or entitlements != expected
        or any(type(value) is not bool for value in entitlements.values())
    ):
        raise ValueError(f"{label}が許可済みの固定entitlement集合と一致しない")
    canonical = plistlib.dumps(entitlements, fmt=plistlib.FMT_XML, sort_keys=True)
    return path, hashlib.sha256(canonical).hexdigest()


def _sign_macos_export_bundle(
    bundle: Path,
    *,
    source_root: Path,
    flutter_framework: FlutterFrameworkReference,
    codesign: str,
    lipo: str,
    architecture: str,
    verification_directory: Path,
    environment: dict[str, str],
) -> dict[str, str]:
    app_entitlements_path, app_entitlements_sha256 = _validated_macos_entitlements_source(
        source_root,
        MACOS_RUNNER_ENTITLEMENTS,
        EXPECTED_MACOS_APP_ENTITLEMENTS,
        "macOS Runner entitlement source",
    )
    helper_entitlements_path, helper_entitlements_sha256 = _validated_macos_entitlements_source(
        source_root,
        MACOS_BROKER_HELPER_ENTITLEMENTS,
        EXPECTED_MACOS_BROKER_HELPER_ENTITLEMENTS,
        "macOS Broker helper entitlement source",
    )
    aot_framework = bundle.joinpath(*MACOS_AOT_FRAMEWORK_RELATIVE_PATH.parts)
    try:
        aot_metadata = aot_framework.lstat()
    except OSError as error:
        raise ValueError("macOS AOT App.frameworkが存在しない") from error
    if not stat.S_ISDIR(aot_metadata.st_mode) or stat.S_ISLNK(aot_metadata.st_mode):
        raise ValueError("macOS AOT App.frameworkが通常directoryではない")
    helper = bundle / "Contents/MacOS/gui_shell_macos_broker"
    _require_file(helper, "macOS Broker helper for sandbox signing")

    # Flutter's pinned embed filter can omit resources covered by the upstream
    # FlutterMacOS.framework seal. Restore and strictly verify them before the
    # root App signature seals the bundle's resource envelope.
    _verify_embedded_flutter_framework(
        bundle,
        flutter_framework,
        codesign=codesign,
        lipo=lipo,
        architecture=architecture,
        verification_directory=verification_directory,
        environment=environment,
    )

    # Sign nested code individually, from the AOT framework outward. Frameworks receive
    # no entitlements; the helper and root App receive their distinct fixed contracts.
    _run(
        [codesign, "--force", "--sign", "-", str(aot_framework)],
        cwd=source_root,
        environment=environment,
    )
    _run(
        [
            codesign,
            "--verify",
            "--strict",
            "--all-architectures",
            "--verbose=2",
            str(aot_framework),
        ],
        cwd=source_root,
        environment=environment,
    )
    _run(
        [
            codesign,
            "--force",
            "--sign",
            "-",
            "--entitlements",
            str(helper_entitlements_path),
            "--generate-entitlement-der",
            str(helper),
        ],
        cwd=source_root,
        environment=environment,
    )
    _run(
        [
            codesign,
            "--verify",
            "--strict",
            "--all-architectures",
            "--verbose=2",
            str(helper),
        ],
        cwd=source_root,
        environment=environment,
    )
    _run(
        [
            codesign,
            "--force",
            "--sign",
            "-",
            "--entitlements",
            str(app_entitlements_path),
            "--generate-entitlement-der",
            str(bundle),
        ],
        cwd=source_root,
        environment=environment,
    )
    _run(
        [
            codesign,
            "--verify",
            "--deep",
            "--strict",
            "--all-architectures",
            "--verbose=2",
            str(bundle),
        ],
        cwd=source_root,
        environment=environment,
    )
    return {
        "app_entitlements_sha256": app_entitlements_sha256,
        "helper_entitlements_sha256": helper_entitlements_sha256,
    }


def _read_signed_macos_entitlements(
    codesign: str, code_path: Path, *, environment: dict[str, str]
) -> dict[str, Any]:
    result = subprocess.run(
        [codesign, "--display", "--entitlements", ":-", "--xml", str(code_path)],
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        encoding="utf-8",
        errors="replace",
        env=environment,
        check=False,
    )
    if result.returncode != 0:
        raise ValueError("macOS code signatureからentitlementを取得できない")
    output = result.stdout
    start = output.find("<?xml")
    if start < 0:
        start = output.find("<plist")
    end = output.rfind("</plist>")
    if start < 0 or end < start:
        raise ValueError("macOS code signatureのentitlement plist境界が不正である")
    payload = output[start : end + len("</plist>")].encode("utf-8")
    try:
        entitlements = plistlib.loads(payload)
    except (plistlib.InvalidFileException, ValueError, TypeError) as error:
        raise ValueError("macOS code signatureのentitlement plistが不正である") from error
    if not isinstance(entitlements, dict):
        raise ValueError("macOS code signatureのentitlementがobjectではない")
    return entitlements


def _tool_version(command: list[str], *, cwd: Path, environment: dict[str, str]) -> str:
    output = _run(command, cwd=cwd, environment=environment)
    first_line = next((line.strip() for line in output.splitlines() if line.strip()), "")
    if not first_line:
        raise ValueError("build toolがversionを返さなかった")
    return first_line


def _patch_app_identity(source_root: Path, bundle_id: str) -> None:
    config = source_root / "apps/desktop_flutter/macos/Runner/Configs/AppInfo.xcconfig"
    raw = _read_regular_file(config, 128 * 1024, "macOS AppInfo.xcconfig").decode("utf-8")
    lines = raw.splitlines(keepends=True)
    bundle_pattern = re.compile(r"^([ \t]*)PRODUCT_BUNDLE_IDENTIFIER[ \t]*=[ \t]*[^\r\n]*$")
    bundle_matches: list[int] = []
    for index, line in enumerate(lines):
        body = line.rstrip("\r\n")
        if bundle_pattern.fullmatch(body):
            bundle_matches.append(index)
    if len(bundle_matches) != 1:
        raise ValueError("macOS AppInfo.xcconfigの固定identity設定が一意でない")
    index = bundle_matches[0]
    line = lines[index]
    ending = line[len(line.rstrip("\r\n")) :]
    match = bundle_pattern.fullmatch(line.rstrip("\r\n"))
    if match is None:
        raise ValueError("macOS AppInfo.xcconfig identity設定が再読時に変化した")
    lines[index] = f"{match.group(1)}PRODUCT_BUNDLE_IDENTIFIER = {bundle_id}{ending}"
    config.write_text("".join(lines), encoding="utf-8", newline="")


def _patch_macos_export_build_settings(source_root: Path, *, architecture: str) -> None:
    if architecture not in {"arm64", "x86_64"}:
        raise ValueError("macOS Export build architectureが未対応である")
    project = source_root / "apps/desktop_flutter/macos/Runner.xcodeproj/project.pbxproj"
    raw = _read_regular_file(project, 2 * 1024 * 1024, "macOS Runner project").decode("utf-8")
    configuration_count = raw.count("isa = XCBuildConfiguration;")
    settings_count = raw.count("buildSettings = {")
    if configuration_count == 0 or configuration_count != settings_count:
        raise ValueError("macOS Xcode build configurationの構造が想定と一致しない")
    raw, patched_count = re.subn(
        r"\bbuildSettings = \{",
        'buildSettings = { CODE_SIGNING_ALLOWED = NO; CODE_SIGNING_REQUIRED = NO; CODE_SIGN_IDENTITY = ""; '
        f"ARCHS = {architecture};",
        raw,
    )
    if patched_count != configuration_count:
        raise ValueError("macOS Export用の署名無効／architecture設定を全configurationへ適用できない")
    default_identity = 'CODE_SIGN_IDENTITY = "-";'
    if default_identity not in raw:
        raise ValueError("macOS Xcodeのad-hoc署名identity設定が想定と一致しない")
    raw = raw.replace(default_identity, 'CODE_SIGN_IDENTITY = "";')
    helper_sign_command = (
        r'/usr/bin/codesign --force --sign \"${EXPANDED_CODE_SIGN_IDENTITY:--}\" '
        r'--entitlements \"$PROJECT_DIR/Runner/BrokerHelper.entitlements\" \"$destination\"\n'
    )
    if raw.count(helper_sign_command) != 1:
        raise ValueError("macOS Broker helper署名工程が一意に確認できない")
    raw = raw.replace(helper_sign_command, "")
    if "/usr/bin/codesign --force --sign" in raw:
        raise ValueError("Export用sourceに独自codesign工程が残っている")
    project.write_text(raw, encoding="utf-8", newline="")


def _require_macos_binary_architecture(
    binaries: list[Path], *, lipo: str, architecture: str, environment: dict[str, str]
) -> dict[str, Any]:
    if architecture not in {"arm64", "x86_64"} or len(binaries) != 2:
        raise ValueError("macOS binary architecture検査条件が不正である")
    for binary in binaries:
        metadata = binary.lstat()
        if not stat.S_ISREG(metadata.st_mode) or stat.S_ISLNK(metadata.st_mode):
            raise ValueError("macOS architecture検査対象binaryが通常fileではない")
        result = subprocess.run(
            [lipo, "-archs", str(binary)],
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            encoding="utf-8",
            errors="replace",
            env=environment,
            check=False,
        )
        found = result.stdout.strip().split()
        if result.returncode != 0 or found != [architecture]:
            raise ValueError("macOS App／Broker binary architectureがbuild hostと一致しない")
    return {
        "status": "passed_host_architecture",
        "architecture": architecture,
        "binary_files_checked": len(binaries),
    }


def _macos_code_objects(bundle: Path, *, collapsed_bundles: tuple[Path, ...]) -> list[Path]:
    if bundle.is_symlink() or not bundle.is_dir():
        raise ValueError("署名検査対象のmacOS App Bundleが通常directoryではない")
    objects = [bundle]
    for path in sorted(bundle.rglob("*"), key=lambda item: item.as_posix()):
        metadata = path.lstat()
        if stat.S_ISLNK(metadata.st_mode):
            continue
        if any(path != collapsed and path.is_relative_to(collapsed) for collapsed in collapsed_bundles):
            continue
        if stat.S_ISDIR(metadata.st_mode):
            if path.suffix.casefold() in MACOS_CODE_BUNDLE_SUFFIXES:
                objects.append(path)
        elif stat.S_ISREG(metadata.st_mode):
            with path.open("rb") as source:
                if source.read(4) in MACH_O_MAGICS:
                    objects.append(path)
    unique: dict[str, Path] = {}
    for path in objects:
        unique[str(path.resolve(strict=True))] = path
    return list(unique.values())


def _macos_main_executable_path(bundle: Path) -> Path:
    """Resolve the app's primary executable from its bounded bundle metadata."""
    contents = bundle / "Contents"
    macos_directory = contents / "MacOS"
    for directory, label in (
        (contents, "macOS App Contents"),
        (macos_directory, "macOS App executable directory"),
    ):
        try:
            metadata = directory.lstat()
        except OSError as error:
            raise ValueError(f"{label}が一意に存在しない") from error
        if stat.S_ISLNK(metadata.st_mode) or not stat.S_ISDIR(metadata.st_mode):
            raise ValueError(f"{label}が通常directoryではない")

    info_path = contents / "Info.plist"
    try:
        info_raw = _read_regular_file(info_path, 1024 * 1024, "macOS App Info.plist")
        info = plistlib.loads(info_raw)
    except (OSError, ValueError, plistlib.InvalidFileException) as error:
        raise ValueError("macOS App Info.plistを安全に読めない") from error
    executable_name = info.get("CFBundleExecutable") if isinstance(info, dict) else None
    if not isinstance(executable_name, str) or re.fullmatch(
        r"[A-Za-z0-9._-]{1,128}", executable_name
    ) is None:
        raise ValueError("macOS App Info.plistのCFBundleExecutableが不正")

    executable = macos_directory / executable_name
    try:
        metadata = executable.lstat()
    except OSError as error:
        raise ValueError("macOS App主実行fileが一意に存在しない") from error
    if (
        stat.S_ISLNK(metadata.st_mode)
        or not stat.S_ISREG(metadata.st_mode)
        or metadata.st_size == 0
        or bool(getattr(metadata, "st_file_attributes", 0) & 0x400)
    ):
        raise ValueError("macOS App主実行fileが通常の非empty fileではない")
    resolved_bundle = bundle.resolve(strict=True)
    resolved_executable = executable.resolve(strict=True)
    if not resolved_executable.is_relative_to(resolved_bundle):
        raise ValueError("macOS App主実行fileがBundle外を指している")
    return resolved_executable


def _is_unsealed_adhoc_executable_signature(output: str) -> bool:
    """Accept only identity-free, resource-unsealed ad-hoc executable signatures."""
    if re.search(r"(?im)^\s*Signature=adhoc\s*$", output) is None:
        return False
    code_directories = re.findall(
        r"(?im)^\s*CodeDirectory\b.*?\bflags=(0x[0-9a-f]+)\(([^)]*)\)", output
    )
    if not code_directories:
        return False
    for raw_flags_value, raw_flags in code_directories:
        try:
            flags_value = int(raw_flags_value, 16)
        except ValueError:
            return False
        flags = frozenset(flag.strip().casefold() for flag in raw_flags.split(","))
        if (flags_value, flags) not in {
            (0x2, frozenset({"adhoc"})),
            (0x20002, frozenset({"adhoc", "linker-signed"})),
        }:
            return False
    if re.search(r"(?im)^\s*Authority=", output):
        return False
    team_identifiers = re.findall(r"(?im)^\s*TeamIdentifier=(.*)$", output)
    if any(value.strip().casefold() not in {"", "not set"} for value in team_identifiers):
        return False
    if re.search(r"(?im)^\s*(?:Signed Time|Timestamp)=", output):
        return False
    if re.search(r"(?im)^\s*Sealed Resources version=", output):
        return False
    return True


def _is_identity_free_adhoc_sealed_bundle_signature(output: str) -> bool:
    """Accept an ad-hoc resource-sealed bundle without Publisher identity."""
    if re.search(r"(?im)^\s*Signature=adhoc\s*$", output) is None:
        return False
    code_directories = re.findall(
        r"(?im)^\s*CodeDirectory\b.*?\bflags=(0x[0-9a-f]+)\(([^)]*)\)", output
    )
    if not code_directories:
        return False
    for raw_flags_value, raw_flags in code_directories:
        try:
            flags_value = int(raw_flags_value, 16)
        except ValueError:
            return False
        flags = frozenset(flag.strip().casefold() for flag in raw_flags.split(","))
        if (flags_value, flags) != (0x2, frozenset({"adhoc"})):
            return False
    if re.search(r"(?im)^\s*Authority=", output):
        return False
    team_identifiers = re.findall(r"(?im)^\s*TeamIdentifier=(.*)$", output)
    if any(value.strip().casefold() not in {"", "not set"} for value in team_identifiers):
        return False
    if re.search(r"(?im)^\s*(?:Signed Time|Timestamp)=", output):
        return False
    return re.search(r"(?im)^\s*Sealed Resources version=", output) is not None


def _is_sandboxed_adhoc_app_bundle_signature(output: str) -> bool:
    """Accept only the local ad-hoc signature used to enforce the fixed App Sandbox."""
    return _is_identity_free_adhoc_sealed_bundle_signature(output)


def _macos_signature_diagnostic(output: str) -> str:
    """Return signature facts without exposing certificate or team identifier values."""
    facts = _macos_signature_facts(output)
    return "; ".join(
        f"{key}={value}" for key, value in facts.items() if key != "adhoc_code_directory"
    )


def _macos_signature_facts(output: str) -> dict[str, Any]:
    signature_values = re.findall(r"(?im)^\s*Signature=(.*?)\s*$", output)
    if signature_values and all(value.strip().casefold() == "adhoc" for value in signature_values):
        signature_class = "adhoc"
    elif signature_values:
        signature_class = "other"
    else:
        signature_class = "unknown"

    code_directory_flags: list[str] = []
    for raw_value, raw_flags in re.findall(
        r"(?im)^\s*CodeDirectory\b.*?\bflags=(0x[0-9a-f]+)\(([^)]*)\)", output
    ):
        safe_flags = [
            flag.strip().casefold()
            for flag in raw_flags.split(",")
            if re.fullmatch(r"[A-Za-z0-9_-]{1,40}", flag.strip())
        ]
        code_directory_flags.append(f"{raw_value.casefold()}({','.join(safe_flags)})")

    team_identifiers = re.findall(r"(?im)^\s*TeamIdentifier=(.*)$", output)
    team_identifier_set = any(
        value.strip().casefold() not in {"", "not set"} for value in team_identifiers
    )
    code_directory_values = re.findall(
        r"(?im)^\s*CodeDirectory\b.*?\bflags=(0x[0-9a-f]+)\(([^)]*)\)", output
    )
    return {
        "signature_class": signature_class,
        "code_directory_flags": "|".join(code_directory_flags) or "none",
        "adhoc_code_directory": any(int(value, 16) & 0x2 for value, _flags in code_directory_values),
        "authority_present": bool(re.search(r"(?im)^\s*Authority=", output)),
        "team_identifier_set": team_identifier_set,
        "signed_time_present": bool(re.search(r"(?im)^\s*Signed Time=", output)),
        "timestamp_present": bool(re.search(r"(?im)^\s*Timestamp=", output)),
        "cms_digest_present": bool(re.search(r"(?im)^\s*CMSDigest=", output)),
        "sealed_resources_present": bool(re.search(r"(?im)^\s*Sealed Resources version=", output)),
    }


def _macos_bundle_tree_sha256(bundle: Path) -> str:
    _validate_bundle_symlinks(bundle)
    digest = hashlib.sha256()
    entries = [
        bundle,
        *sorted(bundle.rglob("*"), key=lambda item: item.relative_to(bundle).as_posix()),
    ]
    for path in entries:
        metadata = path.lstat()
        relative = "." if path == bundle else path.relative_to(bundle).as_posix()
        mode = stat.S_IMODE(metadata.st_mode)
        if stat.S_ISLNK(metadata.st_mode):
            digest.update(
                f"link\0{relative}\0{mode:o}\0{os.readlink(path)}\n".encode("utf-8")
            )
        elif stat.S_ISDIR(metadata.st_mode):
            digest.update(f"directory\0{relative}\0{mode:o}\n".encode("utf-8"))
        elif stat.S_ISREG(metadata.st_mode):
            digest.update(
                f"file\0{relative}\0{mode:o}\0{metadata.st_size}\0{_sha256_file(path)}\n".encode("utf-8")
            )
        else:
            raise ValueError("Flutter Framework treeに通常file／directory以外がある")
    return digest.hexdigest()


def _macos_bundle_entries(bundle: Path) -> dict[str, dict[str, Any]]:
    _validate_bundle_symlinks(bundle)
    entries: dict[str, dict[str, Any]] = {}
    paths = [
        bundle,
        *sorted(bundle.rglob("*"), key=lambda item: item.relative_to(bundle).as_posix()),
    ]
    for path in paths:
        metadata = path.lstat()
        relative = "." if path == bundle else path.relative_to(bundle).as_posix()
        mode = stat.S_IMODE(metadata.st_mode)
        if stat.S_ISLNK(metadata.st_mode):
            entries[relative] = {
                "kind": "symlink",
                "mode": mode,
                "target": os.readlink(path),
            }
        elif stat.S_ISDIR(metadata.st_mode):
            entries[relative] = {"kind": "directory", "mode": mode}
        elif stat.S_ISREG(metadata.st_mode):
            entries[relative] = {
                "kind": "file",
                "mode": mode,
                "size_bytes": metadata.st_size,
                "sha256": _sha256_file(path),
            }
        else:
            raise ValueError("Flutter Framework treeに通常file／directory以外がある")
    return entries


def _flutter_framework_embed_filtered_paths(
    source_entries: dict[str, dict[str, Any]],
) -> set[str]:
    embed_filtered_paths = {
        path
        for path in source_entries
        if PurePosixPath(path).name in FLUTTER_MACOS_FRAMEWORK_EMBED_FILTERED_BASENAMES
    }
    filtered: set[str] = set()
    for directory in embed_filtered_paths:
        filtered.update(
            path
            for path in source_entries
            if path == directory or path.startswith(directory + "/")
        )
    return filtered


def _flutter_framework_denylisted_paths(
    source_entries: dict[str, dict[str, Any]],
) -> set[str]:
    return {
        path
        for path, entry in source_entries.items()
        if entry["kind"] == "file"
        and PurePosixPath(path).name in FLUTTER_MACOS_FRAMEWORK_DENYLISTED_FILES
    }


def _flutter_framework_copy_exclusions(
    source_entries: dict[str, dict[str, Any]],
) -> set[str]:
    excluded = _flutter_framework_embed_filtered_paths(source_entries)
    excluded.update(_flutter_framework_denylisted_paths(source_entries))
    return excluded


def _flutter_framework_export_exclusions(
    source_entries: dict[str, dict[str, Any]],
) -> set[str]:
    return _flutter_framework_denylisted_paths(source_entries)


def _restore_pinned_flutter_framework_embed_filtered_resources(
    source_framework: Path, embedded_framework: Path
) -> int:
    source_entries = _macos_bundle_entries(source_framework)
    if embedded_framework.is_symlink() or not embedded_framework.is_dir():
        raise ValueError("Export App Bundle内FlutterMacOS.frameworkが通常directoryではない")
    embedded_entries = _macos_bundle_entries(embedded_framework)
    restore_paths = _flutter_framework_embed_filtered_paths(source_entries)
    restore_paths.difference_update(_flutter_framework_denylisted_paths(source_entries))
    restored_count = 0

    for relative in sorted(restore_paths, key=lambda value: (value.count("/"), value)):
        source_entry = source_entries[relative]
        source_path = source_framework.joinpath(*PurePosixPath(relative).parts)
        target_path = embedded_framework.joinpath(*PurePosixPath(relative).parts)
        parent = embedded_framework
        for part in PurePosixPath(relative).parts[:-1]:
            parent = parent / part
            try:
                parent_metadata = parent.lstat()
            except FileNotFoundError as error:
                raise ValueError("Flutter Framework復元先parentが存在しない") from error
            if stat.S_ISLNK(parent_metadata.st_mode) or not stat.S_ISDIR(parent_metadata.st_mode):
                raise ValueError("Flutter Framework復元先parentが通常directoryではない")
        try:
            target_metadata = target_path.lstat()
        except FileNotFoundError:
            target_metadata = None

        if target_metadata is not None:
            target_entry = embedded_entries.get(relative)
            if target_entry is None:
                raise ValueError("Flutter Framework復元対象をExport Bundleから安全に読めない")
            source_content = {
                key: value for key, value in source_entry.items() if key != "mode"
            }
            target_content = {
                key: value for key, value in target_entry.items() if key != "mode"
            }
            if source_content != target_content:
                raise ValueError("Export Bundle内filtered Flutter resourceがpinned sourceと異なる")
            continue

        if source_entry["kind"] == "directory":
            target_path.mkdir(mode=source_entry["mode"])
            os.chmod(target_path, source_entry["mode"])
            embedded_entries[relative] = {
                "kind": "directory",
                "mode": stat.S_IMODE(target_path.lstat().st_mode),
            }
        elif source_entry["kind"] == "file":
            with source_path.open("rb") as source, target_path.open("xb") as target:
                shutil.copyfileobj(source, target, length=1024 * 1024)
            os.chmod(target_path, source_entry["mode"])
            target_metadata = target_path.lstat()
            copied_sha256 = _sha256_file(target_path)
            if (
                not stat.S_ISREG(target_metadata.st_mode)
                or stat.S_ISLNK(target_metadata.st_mode)
                or target_metadata.st_size != source_entry["size_bytes"]
                or copied_sha256 != source_entry["sha256"]
            ):
                raise ValueError("pinned Flutter resource copy後のfile hash照合に失敗した")
            embedded_entries[relative] = {
                "kind": "file",
                "mode": stat.S_IMODE(target_metadata.st_mode),
                "size_bytes": target_metadata.st_size,
                "sha256": copied_sha256,
            }
        elif source_entry["kind"] == "symlink":
            link_target = os.readlink(source_path)
            os.symlink(link_target, target_path)
            embedded_entries[relative] = {
                "kind": "symlink",
                "mode": stat.S_IMODE(target_path.lstat().st_mode),
                "target": os.readlink(target_path),
            }
        else:
            raise ValueError("pinned Flutter filtered resource typeが未対応である")
        restored_count += 1

    _validate_bundle_symlinks(embedded_framework)
    return restored_count


def _verify_flutter_framework_copy_entries(
    source_framework: Path, embedded_framework: Path
) -> tuple[str, str]:
    source_entries = _macos_bundle_entries(source_framework)
    embedded_entries = _macos_bundle_entries(embedded_framework)
    exclusions = _flutter_framework_export_exclusions(source_entries)
    expected_paths = set(source_entries) - exclusions
    embedded_paths = set(embedded_entries)
    if embedded_paths != expected_paths:
        missing_paths = sorted(expected_paths - embedded_paths)
        unexpected_paths = sorted(embedded_paths - expected_paths)
        raise ValueError(
            "embedded FlutterMacOS.frameworkのfile layoutが固定Flutter copy規則と一致しない: "
            f"missing_count={len(missing_paths)}, missing={missing_paths[:12]!r}, "
            f"unexpected_count={len(unexpected_paths)}, unexpected={unexpected_paths[:12]!r}"
        )

    binary_relative = FLUTTER_MACOS_FRAMEWORK_BINARY_RELATIVE_PATH.as_posix()
    source_binary = source_entries.get(binary_relative)
    embedded_binary = embedded_entries.get(binary_relative)
    if (
        source_binary is None
        or embedded_binary is None
        or source_binary.get("kind") != "file"
        or embedded_binary.get("kind") != "file"
    ):
        raise ValueError("FlutterMacOS.framework内の固定Mach-O binaryが一意でない")

    for relative in sorted(expected_paths):
        source_entry = source_entries[relative]
        embedded_entry = embedded_entries[relative]
        if source_entry["kind"] != embedded_entry["kind"]:
            raise ValueError(
                f"embedded FlutterMacOS.framework entry typeが固定Flutterと異なる: {relative}"
            )
        kind = source_entry["kind"]
        allowed_modes = {source_entry["mode"]}
        if kind == "directory":
            allowed_modes.add(FLUTTER_RSYNC_DIRECTORY_MODE)
        elif kind == "file":
            allowed_modes.add(FLUTTER_RSYNC_FILE_MODE)
        elif kind == "symlink":
            allowed_modes.add(0o777)
        if embedded_entry["mode"] not in allowed_modes:
            raise ValueError(
                f"embedded FlutterMacOS.framework modeが固定Flutter copy規則と異なる: {relative}"
            )
        if relative == binary_relative:
            continue
        source_content = {key: value for key, value in source_entry.items() if key != "mode"}
        embedded_content = {
            key: value for key, value in embedded_entry.items() if key != "mode"
        }
        if source_content != embedded_content:
            raise ValueError(
                f"embedded FlutterMacOS.framework contentが固定Flutterと異なる: {relative}"
            )

    return source_binary["sha256"], embedded_binary["sha256"]


def _flutter_framework_architecture_slice_sha256(
    source_framework: Path,
    *,
    lipo: str,
    architecture: str,
    verification_directory: Path,
    environment: dict[str, str],
) -> str:
    if architecture not in {"arm64", "x86_64"}:
        raise ValueError("FlutterMacOS.framework thin architectureが未対応である")
    if verification_directory.is_symlink() or not verification_directory.is_dir():
        raise ValueError("Flutter framework slice検証先が通常directoryではない")
    source_binary = source_framework.joinpath(
        *FLUTTER_MACOS_FRAMEWORK_BINARY_RELATIVE_PATH.parts
    )
    if source_binary.is_symlink() or not source_binary.is_file():
        raise ValueError("pinned FlutterMacOS.framework binaryが通常fileではない")
    probe_environment = dict(environment)
    probe_environment.update(
        {"LANG": "C", "LC_ALL": "C", "TMPDIR": str(verification_directory)}
    )
    with tempfile.TemporaryDirectory(
        prefix="d4p-macos-framework-slice-", dir=verification_directory
    ) as temp:
        output = Path(temp) / "FlutterMacOS"
        result = subprocess.run(
            [lipo, "-output", str(output), "-extract", architecture, str(source_binary)],
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            encoding="utf-8",
            errors="replace",
            env=probe_environment,
            check=False,
        )
        if result.returncode != 0:
            raise ValueError(
                "pinned FlutterMacOS.frameworkからtarget architecture sliceを生成できない"
            )
        metadata = output.lstat() if output.exists() or output.is_symlink() else None
        if (
            metadata is None
            or not stat.S_ISREG(metadata.st_mode)
            or stat.S_ISLNK(metadata.st_mode)
        ):
            raise ValueError("pinned FlutterMacOS.framework architecture sliceが通常fileではない")
        return _sha256_file(output)


def _capture_codesign(
    command: list[str], *, environment: dict[str, str]
) -> subprocess.CompletedProcess[str]:
    probe_environment = dict(environment)
    probe_environment.update({"LANG": "C", "LC_ALL": "C"})
    return subprocess.run(
        command,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        encoding="utf-8",
        errors="replace",
        env=probe_environment,
        check=False,
    )


def _codesign_verification_failure_class(output: str) -> str:
    lowered = output.casefold()
    classifications = (
        (
            "sealed_resource_missing_or_invalid",
            ("a sealed resource is missing or invalid",),
        ),
        (
            "resource_envelope_incompatible",
            (
                "code has no resources but signature indicates they must be present",
                "resource envelope is obsolete",
            ),
        ),
        ("code_object_unsigned", ("code object is not signed at all",)),
        (
            "signature_invalid",
            ("invalid signature", "signature verification failed", "failed to verify code signature"),
        ),
        (
            "certificate_validation_failed",
            (
                "certificate has expired",
                "certificate revoked",
                "unable to build chain to self-signed root",
                "cssmerr_tp_cert_expired",
            ),
        ),
    )
    for classification, needles in classifications:
        if any(needle in lowered for needle in needles):
            return classification
    return "unclassified"


def _verify_pinned_flutter_framework_signature(
    framework: Path, *, codesign: str, environment: dict[str, str], scope: str
) -> dict[str, bool]:
    if framework.is_symlink() or not framework.is_dir():
        raise ValueError("pinned FlutterMacOS.frameworkが通常directoryではない")
    _validate_bundle_symlinks(framework)
    verification = _capture_codesign(
        [
            codesign,
            "--verify",
            "--deep",
            "--strict",
            "--all-architectures",
            "--verbose=2",
            str(framework),
        ],
        environment=environment,
    )
    if verification.returncode != 0:
        failure_class = _codesign_verification_failure_class(verification.stdout)
        raise ValueError(
            f"{scope} FlutterMacOS.frameworkのstrict code signature検証に失敗した "
            f"(codesign_failure_class={failure_class})"
        )
    display = _capture_codesign(
        [codesign, "--display", "--verbose=4", str(framework)], environment=environment
    )
    if display.returncode != 0:
        raise ValueError("pinned FlutterMacOS.frameworkのsignature metadataを取得できない")
    facts = _macos_signature_facts(display.stdout)
    required_facts = (
        "authority_present",
        "team_identifier_set",
        "timestamp_present",
        "cms_digest_present",
        "sealed_resources_present",
    )
    if (
        facts["signature_class"] == "adhoc"
        or facts["adhoc_code_directory"]
        or facts["code_directory_flags"] == "none"
        or any(facts[name] is not True for name in required_facts)
    ):
        safe_facts = {name: facts[name] for name in (*required_facts, "code_directory_flags")}
        raise ValueError(
            "pinned FlutterMacOS.frameworkが期待する上流署名条件を満たさない: "
            + "; ".join(f"{key}={value}" for key, value in safe_facts.items())
        )
    return {name: bool(facts[name]) for name in required_facts}


def _prepare_pinned_flutter_framework(
    flutter_root: Path,
    *,
    flutter: str,
    codesign: str,
    environment: dict[str, str],
) -> FlutterFrameworkReference:
    _verify_pinned_flutter_checkout(flutter_root, environment=environment)
    engine_version_file = flutter_root / "bin/internal/engine.version"
    _require_file(engine_version_file, "Flutter engine revision")
    engine_revision = engine_version_file.read_text(encoding="ascii").strip()
    if re.fullmatch(r"[0-9a-f]{40}", engine_revision) is None:
        raise ValueError("Flutter engine revisionが40桁lowercase SHA-1ではない")
    _run([flutter, "precache", "--macos"], cwd=flutter_root, environment=environment)
    _verify_pinned_flutter_checkout(flutter_root, environment=environment)
    source_path = flutter_root.joinpath(*FLUTTER_ENGINE_FRAMEWORK_RELATIVE_PATH.parts)
    current = flutter_root
    for part in FLUTTER_ENGINE_FRAMEWORK_RELATIVE_PATH.parts:
        current = current / part
        if current.is_symlink():
            raise ValueError("pinned Flutter Framework cache pathにsymlinkがある")
    if not source_path.is_dir():
        raise ValueError("pinned FlutterMacOS.frameworkがengine cacheにない")
    tree_sha256 = _macos_bundle_tree_sha256(source_path)
    signature_facts = _verify_pinned_flutter_framework_signature(
        source_path,
        codesign=codesign,
        environment=environment,
        scope="Flutter SDK cache",
    )
    if _macos_bundle_tree_sha256(source_path) != tree_sha256:
        raise ValueError("Flutter SDK cache内Frameworkが署名検証中に変化した")
    return FlutterFrameworkReference(
        source_path=source_path,
        engine_revision=engine_revision,
        tree_sha256=tree_sha256,
        signature_facts=signature_facts,
    )


def _verify_pinned_flutter_checkout(
    flutter_root: Path, *, environment: dict[str, str]
) -> None:
    head = subprocess.run(
        ["git", "-C", str(flutter_root), "rev-parse", "HEAD"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        errors="replace",
        env=environment,
        check=False,
    )
    if head.returncode != 0 or head.stdout.strip() != PINNED_FLUTTER_REVISION:
        raise ValueError("Flutter SDKが固定revisionと一致しない")
    status = subprocess.run(
        ["git", "-C", str(flutter_root), "status", "--porcelain", "--untracked-files=all"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        errors="replace",
        env=environment,
        check=False,
    )
    if status.returncode != 0 or status.stdout.strip():
        raise ValueError("Flutter SDK checkoutに未commit変更がある")


def _verify_embedded_flutter_framework(
    app_bundle: Path,
    reference: FlutterFrameworkReference,
    *,
    codesign: str,
    lipo: str,
    architecture: str,
    verification_directory: Path,
    environment: dict[str, str],
) -> dict[str, Any]:
    if app_bundle.is_symlink() or not app_bundle.is_dir():
        raise ValueError("Export App Bundle rootが通常directoryではない")
    _validate_bundle_symlinks(app_bundle)
    framework = app_bundle.joinpath(*APP_FLUTTER_FRAMEWORK_RELATIVE_PATH.parts)
    if framework.is_symlink() or not framework.is_dir():
        raise ValueError("Export App BundleにFlutterMacOS.frameworkが一意に存在しない")
    matching_frameworks = sorted(
        (path for path in app_bundle.rglob("FlutterMacOS.framework") if path.is_dir()),
        key=lambda path: path.relative_to(app_bundle).as_posix(),
    )
    if matching_frameworks != [framework]:
        raise ValueError("Export App BundleにFlutterMacOS.frameworkが一意に存在しない")
    source_tree_before = _macos_bundle_tree_sha256(reference.source_path)
    if source_tree_before != reference.tree_sha256:
        raise ValueError("Flutter SDK cache内のpinned Frameworkがbuild中に変化した")
    source_facts = _verify_pinned_flutter_framework_signature(
        reference.source_path,
        codesign=codesign,
        environment=environment,
        scope="Flutter SDK cache",
    )
    if source_facts != reference.signature_facts:
        raise ValueError("Flutter SDK cache内Frameworkの署名metadataがbuild中に変化した")
    _restore_pinned_flutter_framework_embed_filtered_resources(
        reference.source_path, framework
    )
    source_tree_after = _macos_bundle_tree_sha256(reference.source_path)
    if source_tree_after != reference.tree_sha256:
        raise ValueError("Flutter SDK cache内のpinned Frameworkが検証中に変化した")
    embedded_tree_before = _macos_bundle_tree_sha256(framework)
    source_binary_sha256, embedded_binary_sha256 = _verify_flutter_framework_copy_entries(
        reference.source_path, framework
    )
    architecture_slice_sha256 = _flutter_framework_architecture_slice_sha256(
        reference.source_path,
        lipo=lipo,
        architecture=architecture,
        verification_directory=verification_directory,
        environment=environment,
    )
    if embedded_binary_sha256 != architecture_slice_sha256:
        raise ValueError(
            "embedded FlutterMacOS.framework binaryがpinned Flutter architecture sliceと一致しない"
        )
    embedded_facts = _verify_pinned_flutter_framework_signature(
        framework,
        codesign=codesign,
        environment=environment,
        scope="Export App Bundle",
    )
    if embedded_facts != reference.signature_facts:
        raise ValueError("Export App Bundle内Flutter Framework署名metadataがsourceと一致しない")
    embedded_tree_sha256 = _macos_bundle_tree_sha256(framework)
    if embedded_tree_sha256 != embedded_tree_before:
        raise ValueError("Export App Bundle内Flutter Frameworkが署名検証中に変化した")
    return {
        "source": "pinned_flutter_release_engine_cache",
        "sdk_revision": PINNED_FLUTTER_REVISION,
        "engine_revision": reference.engine_revision,
        "source_relative_path": FLUTTER_ENGINE_FRAMEWORK_RELATIVE_PATH.as_posix(),
        "embedded_relative_path": APP_FLUTTER_FRAMEWORK_RELATIVE_PATH.as_posix(),
        "source_tree_sha256": reference.tree_sha256,
        "embedded_tree_sha256": embedded_tree_sha256,
        "source_binary_sha256": source_binary_sha256,
        "embedded_binary_sha256": embedded_binary_sha256,
        "architecture_slice_sha256": architecture_slice_sha256,
        "binary_architecture": architecture,
        "binary_architecture_slice_verified": True,
        "non_binary_content_verified": True,
        "framework_copy_transform_verified": True,
        "strict_signature_verified": True,
        **embedded_facts,
    }


def _require_macos_code_without_publisher_identity(
    bundle: Path,
    *,
    flutter_framework: FlutterFrameworkReference,
    codesign: str,
    lipo: str,
    architecture: str,
    verification_directory: Path,
    environment: dict[str, str],
) -> dict[str, Any]:
    framework_path = bundle.joinpath(*APP_FLUTTER_FRAMEWORK_RELATIVE_PATH.parts)
    flutter_framework_evidence = _verify_embedded_flutter_framework(
        bundle,
        flutter_framework,
        codesign=codesign,
        lipo=lipo,
        architecture=architecture,
        verification_directory=verification_directory,
        environment=environment,
    )
    aot_framework_path = bundle.joinpath(*MACOS_AOT_FRAMEWORK_RELATIVE_PATH.parts)
    try:
        aot_framework_metadata = aot_framework_path.lstat()
    except OSError as error:
        raise ValueError("macOS AOT App.frameworkが一意に存在しない") from error
    if not stat.S_ISDIR(aot_framework_metadata.st_mode) or stat.S_ISLNK(aot_framework_metadata.st_mode):
        raise ValueError("macOS AOT App.frameworkが通常directoryではない")
    code_objects = _macos_code_objects(
        bundle, collapsed_bundles=(framework_path, aot_framework_path)
    )
    main_executable_path = _macos_main_executable_path(bundle)
    main_executable_matches = [
        path for path in code_objects if path.resolve(strict=True) == main_executable_path
    ]
    if len(main_executable_matches) != 1:
        raise ValueError("macOS App主実行fileが署名scan対象に一意に含まれない")
    # The App's deep signature verification covers this Mach-O; codesign reports
    # the root bundle's resource seal again when the executable path is queried.
    # Count the App signature once while continuing to inspect every nested code object.
    code_objects = [path for path in code_objects if path != main_executable_matches[0]]
    if len(code_objects) < 2:
        raise ValueError("macOS App Bundle内の実行codeを署名検査できない")
    probe_environment = dict(environment)
    probe_environment.update({"LANG": "C", "LC_ALL": "C"})
    helper_path = bundle / "Contents/MacOS/gui_shell_macos_broker"
    _require_file(helper_path, "macOS Broker helper entitlement verification")
    app_entitlements = _read_signed_macos_entitlements(
        codesign, bundle, environment=probe_environment
    )
    helper_entitlements = _read_signed_macos_entitlements(
        codesign, helper_path, environment=probe_environment
    )
    if app_entitlements != EXPECTED_MACOS_APP_ENTITLEMENTS:
        raise ValueError("macOS App signature entitlementが固定sandbox契約と一致しない")
    if helper_entitlements != EXPECTED_MACOS_BROKER_HELPER_ENTITLEMENTS:
        raise ValueError("macOS Broker helper signature entitlementが継承sandbox契約と一致しない")
    app_entitlements_sha256 = hashlib.sha256(
        plistlib.dumps(app_entitlements, fmt=plistlib.FMT_XML, sort_keys=True)
    ).hexdigest()
    helper_entitlements_sha256 = hashlib.sha256(
        plistlib.dumps(helper_entitlements, fmt=plistlib.FMT_XML, sort_keys=True)
    ).hexdigest()
    unsigned_code_objects = 0
    adhoc_executable_signatures = 0
    sandboxed_adhoc_app_bundle_signatures = 0
    generated_adhoc_aot_framework_signatures = 0
    verified_toolchain_framework_signatures = 0
    for path in code_objects:
        if path == framework_path:
            verified_toolchain_framework_signatures += 1
            continue
        result = subprocess.run(
            [codesign, "--display", "--verbose=4", str(path)],
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            encoding="utf-8",
            errors="replace",
            env=probe_environment,
            check=False,
        )
        relative_path = path.relative_to(bundle).as_posix() or "."
        if result.returncode == 0:
            if path == bundle:
                if _is_sandboxed_adhoc_app_bundle_signature(result.stdout):
                    sandboxed_adhoc_app_bundle_signatures += 1
                    continue
                raise ValueError(
                    "macOS Export candidateにPublisher identity付きまたは許容外署名がある: "
                    f"{relative_path} ({_macos_signature_diagnostic(result.stdout)})"
                )
            if path == aot_framework_path:
                if _is_identity_free_adhoc_sealed_bundle_signature(result.stdout):
                    generated_adhoc_aot_framework_signatures += 1
                    continue
                raise ValueError(
                    "macOS Export candidateにPublisher identity付きまたは許容外署名がある: "
                    f"{relative_path} ({_macos_signature_diagnostic(result.stdout)})"
                )
            if _is_unsealed_adhoc_executable_signature(result.stdout):
                adhoc_executable_signatures += 1
                continue
            raise ValueError(
                "macOS Export candidateにPublisher identity付きまたは許容外署名がある: "
                f"{relative_path} ({_macos_signature_diagnostic(result.stdout)})"
            )
        if "code object is not signed at all" not in result.stdout.casefold():
            raise ValueError(
                "macOS code signature状態を分類できない: "
                f"{relative_path}"
            )
        unsigned_code_objects += 1
    if sandboxed_adhoc_app_bundle_signatures != 1:
        raise ValueError("macOS Export App Bundleに固定sandbox entitlement付きad-hoc署名がない")
    if generated_adhoc_aot_framework_signatures != 1:
        raise ValueError("macOS AOT App.frameworkにad-hoc resource sealが一つない")
    return {
        "status": "passed_no_d4_publisher_identity",
        "code_objects_checked": len(code_objects),
        "unsigned_code_objects": unsigned_code_objects,
        "adhoc_executable_signatures": adhoc_executable_signatures,
        "sandboxed_adhoc_app_bundle_signatures": sandboxed_adhoc_app_bundle_signatures,
        "generated_adhoc_aot_framework_signatures": generated_adhoc_aot_framework_signatures,
        "verified_toolchain_framework_signatures": verified_toolchain_framework_signatures,
        "d4_publisher_identity_signatures": 0,
        "d4_resource_sealed_bundle_signatures": (
            sandboxed_adhoc_app_bundle_signatures + generated_adhoc_aot_framework_signatures
        ),
        "sandbox_entitlements": {
            "status": "passed_exact_expected_entitlements",
            "app_sandbox_entitlements_verified": True,
            "helper_inherited_sandbox_entitlements_verified": True,
            "app_entitlements_sha256": app_entitlements_sha256,
            "helper_entitlements_sha256": helper_entitlements_sha256,
        },
        "flutter_toolchain_framework": flutter_framework_evidence,
    }


def _validate_bundle_symlinks(bundle: Path) -> None:
    root = bundle.resolve(strict=True)
    if not bundle.is_dir() or bundle.is_symlink():
        raise ValueError("macOS App Bundle rootが通常directoryではない")
    for path in bundle.rglob("*"):
        metadata = path.lstat()
        if stat.S_ISLNK(metadata.st_mode):
            target = os.readlink(path)
            if Path(target).is_absolute() or not path.resolve(strict=True).is_relative_to(root):
                raise ValueError("macOS App Bundle symlinkがbundle外またはdanglingを指している")
        elif not stat.S_ISDIR(metadata.st_mode) and not stat.S_ISREG(metadata.st_mode):
            raise ValueError("macOS App Bundleに通常file／directory以外がある")


def _macos_artifact_inventory(
    root: Path, app_bundle: Path
) -> tuple[list[dict[str, Any]], list[dict[str, Any]], int, str]:
    if root.is_symlink() or not root.is_dir():
        raise ValueError("macOS Export artifact rootが通常directoryではない")
    app_root = app_bundle.resolve(strict=True)
    records: list[dict[str, Any]] = []
    regular_files: list[dict[str, Any]] = []
    total_bytes = 0
    tree_hash = hashlib.sha256()
    for path in sorted(root.rglob("*"), key=lambda item: item.relative_to(root).as_posix()):
        metadata = path.lstat()
        relative = path.relative_to(root).as_posix()
        if stat.S_ISLNK(metadata.st_mode):
            if not path.resolve(strict=True).is_relative_to(app_root):
                raise ValueError(f"App Bundle symlinkがbundle外を指している: {relative}")
            target = os.readlink(path)
            record = {"path": relative, "kind": "symlink", "target": target}
            records.append(record)
            tree_hash.update(f"link\0{relative}\0{target}\n".encode("utf-8"))
            continue
        if stat.S_ISDIR(metadata.st_mode):
            continue
        if not stat.S_ISREG(metadata.st_mode) or not path.resolve(strict=True).is_relative_to(root.resolve()):
            raise ValueError(f"macOS artifactに通常file以外またはroot外参照がある: {relative}")
        digest = _sha256_file(path)
        size = metadata.st_size
        record = {"path": relative, "kind": "file", "size_bytes": size, "sha256": digest}
        records.append(record)
        regular_files.append(record)
        total_bytes += size
        tree_hash.update(f"file\0{relative}\0{size}\0{digest}\n".encode("utf-8"))
    if not regular_files:
        raise ValueError("macOS Export artifactに通常fileがない")
    return records, regular_files, total_bytes, tree_hash.hexdigest()


def _scan_macos_artifacts(
    root: Path,
    app_bundle: Path,
    records: list[dict[str, Any]],
    regular_files: list[dict[str, Any]],
    total_bytes: int,
    tree_sha256: str,
) -> dict[str, Any]:
    bytes_scanned = 0
    for record in regular_files:
        relative = record["path"]
        path = root / PurePosixPath(relative)
        metadata = path.lstat()
        if not stat.S_ISREG(metadata.st_mode):
            raise ValueError(f"credential scan対象が通常fileでなくなった: {relative}")
        name = PurePosixPath(relative).name.casefold()
        if name in SENSITIVE_ARTIFACT_NAMES or Path(name).suffix in {
            ".jks", ".key", ".keystore", ".p12", ".pfx", ".pem",
        }:
            raise ValueError(f"Credentialらしい固定file名をbundleで検出した: {relative}")
        carry = b""
        with path.open("rb") as source:
            while True:
                chunk = source.read(CREDENTIAL_SCAN_CHUNK_BYTES)
                if not chunk:
                    break
                bytes_scanned += len(chunk)
                candidate = carry + chunk
                for pattern_id, pattern in CREDENTIAL_PATTERNS:
                    if pattern.search(candidate) is not None:
                        raise ValueError(f"既知Credential patternをbundleで検出した: {pattern_id} ({relative})")
                carry = candidate[-CREDENTIAL_SCAN_OVERLAP_BYTES:]
        if _sha256_file(path) != record["sha256"]:
            raise ValueError("credential scan中にartifact fileが変化した")
    after_records, after_regular, after_bytes, after_hash = _macos_artifact_inventory(root, app_bundle)
    if (after_records, after_regular, after_bytes, after_hash) != (records, regular_files, total_bytes, tree_sha256):
        raise ValueError("credential scan中にmacOS artifactが変化した")
    return {
        "version": 1,
        "status": CREDENTIAL_SCAN_STATUS,
        "scope": "macos_app_regular_files_and_package_archives",
        "coverage": "known_patterns_only",
        "files_scanned": len(regular_files),
        "bytes_scanned": bytes_scanned,
        "findings": 0,
        "artifact_tree_sha256": tree_sha256,
    }


def _require_file(path: Path, label: str) -> None:
    metadata = path.lstat()
    if not stat.S_ISREG(metadata.st_mode) or path.is_symlink() or metadata.st_size == 0:
        raise ValueError(f"{label}が通常の非empty fileではない")


def _safe_export_error_detail(error: BaseException) -> str:
    if not isinstance(error, OSError):
        return ""
    source_frames = [
        frame
        for frame in traceback.extract_tb(error.__traceback__)
        if Path(frame.filename).name == Path(__file__).name
    ]
    detail: list[str] = []
    if source_frames:
        frame = source_frames[-1]
        detail.append(f"{frame.name}:{frame.lineno}")
    if error.errno is not None:
        detail.append(f"errno={error.errno}")
    if error.strerror:
        detail.append(f"reason={error.strerror}")
    return "; ".join(detail)


def _macos_aot_report_root(report: dict[str, Any]) -> dict[str, Any]:
    roots: list[dict[str, Any]] = []
    pending: list[Any] = [report]
    while pending:
        value = pending.pop()
        if isinstance(value, dict):
            if value.get("n") == MACOS_AOT_REPORT_ROOT:
                roots.append(value)
            pending.extend(value.values())
        elif isinstance(value, list):
            pending.extend(value)
    if len(roots) != 1:
        raise ValueError("Flutter macOS size-analysis reportにDart AOT treeが一意に存在しない")
    return roots[0]


def _macos_aot_surface_libraries(
    report: dict[str, Any], catalog: dict[str, Any]
) -> list[str]:
    targets: dict[str, tuple[str, ...]] = {}
    lib_root = PurePosixPath("apps/desktop_flutter/lib")
    package_root = ("package:gui_shell_desktop",)
    for module in catalog["optional_modules"]:
        for surface in module["source_surfaces"]:
            source = PurePosixPath(surface)
            try:
                relative = source.relative_to(lib_root)
            except ValueError as error:
                raise ValueError(
                    f"Catalog surfaceがDesktop Flutter lib外にある: {surface}"
                ) from error
            if not relative.parts or source.suffix != ".dart":
                raise ValueError(f"Catalog surfaceがDart library pathではない: {surface}")
            targets[surface] = (*package_root, *relative.parts)

    found: set[str] = set()
    pending: list[tuple[Any, tuple[str, ...]]] = [(_macos_aot_report_root(report), ())]
    while pending:
        value, ancestors = pending.pop()
        if isinstance(value, dict):
            name = value.get("n")
            path = (*ancestors, name) if isinstance(name, str) else ancestors
            for surface, target in targets.items():
                if len(path) >= len(target) and path[-len(target) :] == target:
                    found.add(surface)
            pending.extend((child, path) for child in value.values())
        elif isinstance(value, list):
            pending.extend((child, ancestors) for child in value)
    return sorted(found)


def _macos_module_source_surfaces(
    catalog: dict[str, Any], included_module_ids: set[str]
) -> tuple[list[str], list[str]]:
    included = sorted(
        surface
        for module in catalog["optional_modules"]
        if module["module_id"] in included_module_ids
        for surface in module["source_surfaces"]
    )
    excluded = sorted(
        surface
        for module in catalog["optional_modules"]
        if module["module_id"] not in included_module_ids
        for surface in module["source_surfaces"]
    )
    return included, excluded


def _macos_aot_report_path(output: str, isolated_home: Path) -> Path:
    paths = [
        line.partition(":")[2].strip()
        for line in output.splitlines()
        if line.startswith(MACOS_AOT_REPORT_SENTINEL)
    ]
    if len(paths) != 1 or not paths[0]:
        raise ValueError("Flutter macOS AOT size-analysis reportの出力pathが一意でない")
    candidate = Path(paths[0])
    if not candidate.is_absolute():
        raise ValueError("Flutter macOS AOT size-analysis report pathが絶対pathではない")
    expected_directory = isolated_home / ".flutter-devtools"
    try:
        directory_metadata = expected_directory.lstat()
        if not stat.S_ISDIR(directory_metadata.st_mode) or stat.S_ISLNK(directory_metadata.st_mode):
            raise ValueError("Flutter macOS AOT report directoryが通常directoryではない")
        resolved_home = isolated_home.resolve(strict=True)
        resolved_directory = expected_directory.resolve(strict=True)
        resolved_directory.relative_to(resolved_home)
        resolved_candidate = candidate.resolve(strict=True)
        resolved_candidate.relative_to(resolved_directory)
    except (OSError, ValueError) as error:
        raise ValueError("Flutter macOS AOT reportが隔離HOME外にある") from error
    if not re.fullmatch(r"macos-code-size-analysis_[0-9]+\.json", candidate.name):
        raise ValueError("Flutter macOS AOT report filenameが固定形式ではない")
    return candidate


def _macos_aot_image(path: Path, label: str) -> tuple[int, str]:
    try:
        metadata = path.lstat()
    except OSError as error:
        raise ValueError(f"{label}が存在しない") from error
    if not stat.S_ISREG(metadata.st_mode) or stat.S_ISLNK(metadata.st_mode):
        raise ValueError(f"{label}が通常fileではない")
    return metadata.st_size, _sha256_file(path)


def _capture_macos_aot_surface_analysis(
    *,
    output: str,
    isolated_home: Path,
    input_directory: Path,
    profiled_app: Path,
    bundle_root: Path,
    staging_root: Path,
    target_architecture: str,
    catalog: dict[str, Any],
    included_module_ids: set[str],
) -> dict[str, Any]:
    if target_architecture not in {"arm64", "x86_64"}:
        raise ValueError("macOS AOT size-analysis architectureが未対応である")
    report_path = _macos_aot_report_path(output, isolated_home)
    report_raw = _read_regular_file(
        report_path, MAX_SIZE_ANALYSIS_BYTES, "Flutter macOS AOT size-analysis report"
    )
    report = _parse_json_bytes(report_raw)
    if not isinstance(report, dict) or report.get("type") != "macos":
        raise ValueError("Flutter size-analysis reportがmacOS形式ではない")
    aot_root = _macos_aot_report_root(report)
    actual_surfaces = _macos_aot_surface_libraries(report, catalog)
    included_surfaces, excluded_surfaces = _macos_module_source_surfaces(
        catalog, included_module_ids
    )
    if actual_surfaces != included_surfaces:
        missing = sorted(set(included_surfaces) - set(actual_surfaces))
        unexpected = sorted(set(actual_surfaces) - set(included_surfaces))
        raise ValueError(
            "Flutter macOS Release AOT libraryがModule選択と一致しない: "
            f"missing={missing}, unexpected={unexpected}"
        )

    input_records: list[dict[str, Any]] = []
    expected_input_names = {
        f"snapshot.{target_architecture}.json",
        f"trace.{target_architecture}.json",
    }
    actual_input_names = {path.name for path in input_directory.iterdir()}
    if actual_input_names != expected_input_names:
        raise ValueError("Flutter macOS AOT input file setが対象architectureと一致しない")
    for name in sorted(expected_input_names):
        input_path = input_directory / name
        raw = _read_regular_file(input_path, MAX_SIZE_ANALYSIS_BYTES, "Flutter macOS AOT input")
        if not isinstance(_parse_json_bytes(raw), dict):
            raise ValueError("Flutter macOS AOT size-analysis inputがJSON objectではない")
        input_records.append(
            {"name": name, "size_bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest()}
        )

    profiled_image = profiled_app.joinpath(*MACOS_AOT_IMAGE_RELATIVE_PATH.parts)
    exported_image = bundle_root.joinpath(*MACOS_AOT_IMAGE_RELATIVE_PATH.parts)
    profiled_size, profiled_hash = _macos_aot_image(
        profiled_image, "Flutter macOS profile AOT image"
    )
    exported_size, exported_hash = _macos_aot_image(
        exported_image, "macOS Export bundle AOT image"
    )
    if (
        not isinstance(aot_root.get("value"), int)
        or isinstance(aot_root.get("value"), bool)
        or aot_root.get("value") != profiled_size
        or profiled_size != exported_size
        or profiled_hash != exported_hash
    ):
        raise ValueError("Flutter AOT report／profile／Export bundle image identityが一致しない")

    analysis_directory = staging_root / "analysis"
    analysis_directory.mkdir(mode=0o700, exist_ok=False)
    report_destination = analysis_directory / "macos-code-size-analysis.json"
    with report_destination.open("xb") as destination:
        destination.write(report_raw)
        destination.flush()
        os.fsync(destination.fileno())
    report_record = {
        "path": "analysis/macos-code-size-analysis.json",
        "size_bytes": len(report_raw),
        "sha256": hashlib.sha256(report_raw).hexdigest(),
    }
    return {
        "scope": "flutter_macos_release_aot_catalog_surfaces",
        "report": report_record,
        "inputs": input_records,
        "aot_surface_libraries": actual_surfaces,
        "included_source_surfaces": included_surfaces,
        "excluded_source_surfaces": excluded_surfaces,
        "profiled_aot_image_size_bytes": profiled_size,
        "exported_aot_image_size_bytes": exported_size,
        "profiled_aot_image_sha256": profiled_hash,
        "exported_aot_image_sha256": exported_hash,
        "image_matches_export": True,
    }


def _validate_macos_aot_surface_evidence(
    evidence: dict[str, Any],
    root: Path,
    profiled_app: Path,
    app_bundle: Path,
    input_directory: Path,
) -> None:
    analysis = evidence["aot_surface_analysis"]
    if analysis["scope"] != "flutter_macos_release_aot_catalog_surfaces":
        raise ValueError("macOS AOT surface evidence scopeが未対応である")
    report_record = analysis["report"]
    if report_record["path"] != "analysis/macos-code-size-analysis.json":
        raise ValueError("macOS AOT report pathが固定evidence位置ではない")
    report_raw = _read_regular_file(
        root / report_record["path"],
        MAX_SIZE_ANALYSIS_BYTES,
        "macOS AOT evidence report",
    )
    if (
        len(report_raw) != report_record["size_bytes"]
        or hashlib.sha256(report_raw).hexdigest() != report_record["sha256"]
    ):
        raise ValueError("macOS AOT report identityがevidenceと一致しない")
    report = _parse_json_bytes(report_raw)
    if not isinstance(report, dict) or report.get("type") != "macos":
        raise ValueError("macOS AOT evidence report typeが不正である")
    aot_root = _macos_aot_report_root(report)

    catalog = _load_catalog()
    optional_ids = {item["module_id"] for item in catalog["optional_modules"]}
    included_ids = set(evidence["module_plan"]["included_module_ids"]) & optional_ids
    actual_surfaces = _macos_aot_surface_libraries(report, catalog)
    included_surfaces, excluded_surfaces = _macos_module_source_surfaces(
        catalog, included_ids
    )
    if (
        actual_surfaces != included_surfaces
        or analysis["aot_surface_libraries"] != actual_surfaces
        or analysis["included_source_surfaces"] != included_surfaces
        or analysis["excluded_source_surfaces"] != excluded_surfaces
    ):
        raise ValueError("macOS AOT surface report／ModulePlan／evidenceが一致しない")

    architecture = evidence["target_architecture"]
    expected_input_names = {
        f"snapshot.{architecture}.json",
        f"trace.{architecture}.json",
    }
    records = analysis["inputs"]
    if {record["name"] for record in records} != expected_input_names or len(records) != 2:
        raise ValueError("macOS AOT input evidenceがtarget architectureと一致しない")
    input_metadata = input_directory.lstat()
    if not stat.S_ISDIR(input_metadata.st_mode) or stat.S_ISLNK(input_metadata.st_mode):
        raise ValueError("macOS AOT input directoryが通常directoryではない")
    if {path.name for path in input_directory.iterdir()} != expected_input_names:
        raise ValueError("macOS AOT input file setがevidence後に変化した")
    for record in records:
        raw = _read_regular_file(
            input_directory / record["name"],
            MAX_SIZE_ANALYSIS_BYTES,
            "macOS AOT input evidence",
        )
        if (
            len(raw) != record["size_bytes"]
            or hashlib.sha256(raw).hexdigest() != record["sha256"]
            or not isinstance(_parse_json_bytes(raw), dict)
        ):
            raise ValueError("macOS AOT input file identityがevidenceと一致しない")

    profiled_size, profiled_hash = _macos_aot_image(
        profiled_app.joinpath(*MACOS_AOT_IMAGE_RELATIVE_PATH.parts),
        "macOS profiled AOT image",
    )
    exported_size, exported_hash = _macos_aot_image(
        app_bundle.joinpath(*MACOS_AOT_IMAGE_RELATIVE_PATH.parts),
        "macOS exported AOT image",
    )
    if (
        not isinstance(aot_root.get("value"), int)
        or isinstance(aot_root.get("value"), bool)
        or aot_root.get("value") != profiled_size
        or profiled_size != exported_size
        or profiled_hash != exported_hash
        or analysis["profiled_aot_image_size_bytes"] != profiled_size
        or analysis["exported_aot_image_size_bytes"] != exported_size
        or analysis["profiled_aot_image_sha256"] != profiled_hash
        or analysis["exported_aot_image_sha256"] != exported_hash
        or analysis["image_matches_export"] is not True
    ):
        raise ValueError("macOS AOT report／profile／bundle image evidenceが一致しない")

    expected_defines = [
        "--dart-define=GUI_SHELL_MODULE_"
        + MODULE_DEFINE_NAMES[module["module_id"]]
        + "="
        + str(module["module_id"] in included_ids).lower()
        for module in catalog["optional_modules"]
    ]
    actual_defines = [
        argument
        for argument in evidence["flutter_build_arguments"]
        if argument.startswith("--dart-define=GUI_SHELL_MODULE_")
    ]
    if (
        actual_defines != expected_defines
        or "--analyze-size" not in evidence["flutter_build_arguments"]
        or "--code-size-directory=<isolated-size-analysis>"
        not in evidence["flutter_build_arguments"]
    ):
        raise ValueError("macOS Flutter build argumentsがModulePlan／AOT analysisと一致しない")


def build_macos_product(
    receipt_path: Path,
    manifest_path: Path,
    output_dir: Path,
    *,
    allow_verification_branch: bool = False,
) -> dict[str, Any]:
    if sys.platform != "darwin":
        raise ValueError("macOS Product ExportはmacOS native host専用である")
    _source_ref_name, source_ref, commit = _require_clean_source(
        allow_verification_branch=allow_verification_branch
    )
    output = _resolve_output_directory(output_dir)
    receipt_raw = _read_regular_file(receipt_path, MAX_INPUT_BYTES, "Export Receipt")
    manifest_raw = _read_regular_file(manifest_path, MAX_MANIFEST_BYTES, "Export Manifest file")
    receipt, manifest_file, plan, receipt_hash, manifest_hash = validate_export_inputs(
        receipt_raw, manifest_raw, target_platform="macos"
    )
    if manifest_path.name != receipt["manifest_file"]["file_name"]:
        raise ValueError("入力Manifest file名がBroker Receiptと一致しない")
    app_id = manifest_file["manifest"]["app_identity"]["app_id"]
    audit_store_id = manifest_file["manifest"]["audit_store"]["store_id"]
    suffix = app_id.removeprefix("d4-pocket-app-")
    bundle_id = bundle_identifier_for_app(app_id)
    bundle_name = f"D4Pocket-{suffix[:12]}.app"
    installer_id = f"{bundle_id}.installer"
    uninstaller_id = uninstaller_package_identifier_for_app(bundle_id)
    tools = _tool_paths()
    target_architecture = _darwin_build_architecture()
    defines = dart_defines(plan)
    started = time.perf_counter()
    # 最終evidence検証でprofiled AppとAOT入力を再読するため、両一時領域を検証後まで保持する。
    with tempfile.TemporaryDirectory(
        prefix=OUTPUT_TEMP_PREFIX, dir=output.parent
    ) as output_temp, tempfile.TemporaryDirectory(
        prefix=BUILD_TEMP_PREFIX, dir=_resolve_build_temp_root()
    ) as build_temp:
        staging_root = Path(output_temp) / "product"
        staging_root.mkdir(mode=0o700)
        temporary_root = Path(build_temp)
        flutter_root = Path(tools["flutter"]).resolve(strict=True).parent.parent
        environment = _macos_environment(temporary_root, flutter_root=flutter_root)
        flutter_framework = _prepare_pinned_flutter_framework(
            flutter_root,
            flutter=tools["flutter"],
            codesign=tools["codesign"],
            environment=environment,
        )
        source_root = temporary_root / "source"
        source_root.mkdir(mode=0o700)
        source_archive_path = temporary_root / "source.tar"
        archive_hash = _write_source_archive(commit, source_archive_path)
        _safe_extract_source(source_archive_path, source_root)
        version = product_version_from_source(source_root)
        if not re.fullmatch(r"[0-9A-Za-z.+_-]{1,64}", version):
            raise ValueError("Product VersionをmacOS packageへ安全に渡せない")
        flutter_project = source_root / "apps/desktop_flutter"
        _patch_app_identity(source_root, bundle_id)
        _patch_macos_export_build_settings(
            source_root, architecture=target_architecture
        )

        toolchain = {
            "architecture": target_architecture,
            "flutter": _tool_version([tools["flutter"], "--version"], cwd=flutter_project, environment=environment),
            "flutter_sdk_revision": PINNED_FLUTTER_REVISION,
            "flutter_engine_revision": flutter_framework.engine_revision,
            "cargo": _tool_version([tools["cargo"], "+" + RUST_TOOLCHAIN, "--version"], cwd=source_root, environment=environment),
            "rustc": _tool_version([tools["rustc"], "+" + RUST_TOOLCHAIN, "--version"], cwd=source_root, environment=environment),
            "xcodebuild": _tool_version([tools["xcodebuild"], "-version"], cwd=source_root, environment=environment),
            "macos_version": _tool_version(["/usr/bin/sw_vers", "-productVersion"], cwd=source_root, environment=environment),
        }
        app_owner_manifest = source_root / "native/macos_owner/Cargo.toml"
        broker_manifest = source_root / "native/rust_helper/Cargo.toml"
        _run(
            [tools["cargo"], "+" + RUST_TOOLCHAIN, "build", "--release", "--locked", "--manifest-path", str(app_owner_manifest), "--lib"],
            cwd=source_root,
            environment=environment,
        )
        rust_environment = dict(environment)
        rust_environment.update(
            {
                "GUI_SHELL_PRODUCT_APP_ID": app_id,
                "GUI_SHELL_PRODUCT_AUDIT_STORE_ID": audit_store_id,
                "GUI_SHELL_PRODUCT_MANIFEST_SHA256": manifest_hash,
            }
        )
        update_trust = manifest_file.get("update_trust")
        if update_trust is not None:
            rust_environment["GUI_SHELL_UPDATE_TRUST_JSON"] = json.dumps(
                update_trust, ensure_ascii=False, sort_keys=True, separators=(",", ":")
            )
        _run(
            [tools["cargo"], "+" + RUST_TOOLCHAIN, "build", "--release", "--locked", "--manifest-path", str(broker_manifest), "--bin", "gui_shell_macos_broker"],
            cwd=source_root,
            environment=rust_environment,
        )
        _run(
            [tools["flutter"], "pub", "get", "--enforce-lockfile"], cwd=flutter_project, environment=environment
        )
        size_analysis_input_directory = temporary_root / "macos-size-analysis-inputs"
        size_analysis_input_directory.mkdir(mode=0o700)
        flutter_arguments = [
            "build", "macos", "--release", "--no-pub", "--analyze-size",
            f"--code-size-directory={size_analysis_input_directory}",
            *[
                f"--dart-define={name}={str(enabled).lower()}"
                for name, enabled in defines.items()
            ],
            f"--dart-define=GUI_SHELL_PRODUCT_VERSION={version}",
        ]
        flutter_output = _run(
            [tools["flutter"], *flutter_arguments],
            cwd=flutter_project,
            environment=environment,
        )
        built_app = flutter_project / "build/macos/Build/Products/Release/gui_shell_desktop.app"
        app_info_plist = built_app / "Contents/Info.plist"
        _require_file(app_info_plist, "macOS Info.plist")
        built_bundle_id = _run(
            [tools["plutil"], "-extract", "CFBundleIdentifier", "raw", "-o", "-", str(built_app / "Contents/Info.plist")],
            cwd=source_root,
            environment=environment,
        ).strip()
        if built_bundle_id != bundle_id:
            raise ValueError("built App Bundle identifierがExport App identityと一致しない")
        built_executable_name = _run(
            [tools["plutil"], "-extract", "CFBundleExecutable", "raw", "-o", "-", str(app_info_plist)],
            cwd=source_root,
            environment=environment,
        ).strip()
        if re.fullmatch(r"[A-Za-z0-9._-]{1,128}", built_executable_name) is None:
            raise ValueError("macOS App executable名が不正である")
        built_executable = built_app / "Contents/MacOS" / built_executable_name
        _require_file(built_executable, "macOS App executable")
        built_helper = built_app / "Contents/MacOS/gui_shell_macos_broker"
        _require_file(built_helper, "macOS Broker helper")
        macos_binary_architecture_scan = _require_macos_binary_architecture(
            [built_executable, built_helper],
            lipo=tools["lipo"],
            architecture=target_architecture,
            environment=environment,
        )
        _validate_bundle_symlinks(built_app)

        app_bundle = staging_root / bundle_name
        shutil.copytree(built_app, app_bundle, symlinks=True)
        resources = app_bundle / "Contents/Resources"
        if resources.exists() or resources.is_symlink():
            resource_metadata = resources.lstat()
            if not stat.S_ISDIR(resource_metadata.st_mode) or stat.S_ISLNK(resource_metadata.st_mode):
                raise ValueError("macOS App Bundle Resourcesが通常directoryではない")
        else:
            resources.mkdir(mode=0o755)
        product_manifest = resources / "product_manifest.json"
        if product_manifest.exists() or product_manifest.is_symlink():
            raise ValueError("build outputに予約済みproduct_manifest.jsonが存在する")
        with product_manifest.open("xb") as target:
            target.write(manifest_raw)
        if product_manifest.read_bytes() != manifest_raw:
            raise ValueError("Export Manifestをbyte-for-byteでbundleへ保持できなかった")
        _validate_bundle_symlinks(app_bundle)
        _sign_macos_export_bundle(
            app_bundle,
            source_root=source_root,
            flutter_framework=flutter_framework,
            codesign=tools["codesign"],
            lipo=tools["lipo"],
            architecture=target_architecture,
            verification_directory=temporary_root,
            environment=environment,
        )
        app_code_signature_scan = _require_macos_code_without_publisher_identity(
            app_bundle,
            flutter_framework=flutter_framework,
            codesign=tools["codesign"],
            lipo=tools["lipo"],
            architecture=target_architecture,
            verification_directory=temporary_root,
            environment=environment,
        )
        catalog = _load_catalog()
        aot_surface_analysis = _capture_macos_aot_surface_analysis(
            output=flutter_output,
            isolated_home=Path(environment["HOME"]),
            input_directory=size_analysis_input_directory,
            profiled_app=built_app,
            bundle_root=app_bundle,
            staging_root=staging_root,
            target_architecture=target_architecture,
            catalog=catalog,
            included_module_ids=set(plan.included_optional_ids),
        )

        app_archive_path = staging_root / f"{bundle_name}.zip"
        _run(
            [tools["ditto"], "-c", "-k", "--sequesterRsrc", "--keepParent", str(app_bundle), str(app_archive_path)],
            cwd=source_root,
            environment=environment,
        )
        _require_file(app_archive_path, "macOS App archive")

        payload_root = temporary_root / "payload"
        install_bundle = payload_root / "Applications" / bundle_name
        install_bundle.parent.mkdir(parents=True, mode=0o700)
        shutil.copytree(app_bundle, install_bundle, symlinks=True)
        if (install_bundle / "Contents/Resources/product_manifest.json").read_bytes() != manifest_raw:
            raise ValueError("package staging App BundleがProduct Manifestを保持していない")
        package_path = staging_root / f"D4Pocket-{suffix[:12]}.pkg"
        _run(
            [tools["pkgbuild"], "--root", str(payload_root), "--identifier", installer_id,
             "--version", version, "--install-location", "/", "--ownership", "preserve",
             str(package_path)],
            cwd=source_root,
            environment=environment,
        )
        _require_file(package_path, "macOS package candidate")

        uninstaller_template_path = source_root / "installer/macos/uninstall/postinstall.sh.in"
        uninstaller_template = _read_regular_file(
            uninstaller_template_path, 32 * 1024, "macOS uninstaller script template"
        ).decode("utf-8")
        uninstaller_script = render_macos_uninstaller_postinstall(
            uninstaller_template,
            bundle_name=bundle_name,
            bundle_identifier=bundle_id,
        )
        uninstaller_scripts = temporary_root / "uninstaller-scripts"
        uninstaller_scripts.mkdir(mode=0o700)
        postinstall_path = uninstaller_scripts / "postinstall"
        postinstall_path.write_bytes(uninstaller_script.encode("utf-8"))
        postinstall_path.chmod(0o755)
        uninstaller_package_path = (
            staging_root / f"D4Pocket-{suffix[:12]}-Uninstaller.pkg"
        )
        _run(
            [tools["pkgbuild"], "--nopayload", "--scripts", str(uninstaller_scripts),
             "--identifier", uninstaller_id, "--version", version,
             str(uninstaller_package_path)],
            cwd=source_root,
            environment=environment,
        )
        _require_file(uninstaller_package_path, "macOS uninstaller package candidate")

        package_manifest = (install_bundle / "Contents/Resources/product_manifest.json").read_bytes()
        if product_manifest.read_bytes() != manifest_raw or package_manifest != manifest_raw:
            raise ValueError("Export Manifestがpackage対象App Bundleから変化した")
        records, regular_files, total_bytes, tree_hash = _macos_artifact_inventory(
            staging_root, app_bundle
        )
        credential_scan = _scan_macos_artifacts(
            staging_root, app_bundle, records, regular_files, total_bytes, tree_hash
        )

        elapsed_ms = round((time.perf_counter() - started) * 1000)
        catalog = _load_catalog()
        evidence = {
            "version": 7,
            "evidence_type": "d4_pocket_macos_export_build",
            "evidence_source": "INTERNAL_STATE",
            "product": "D4 Pocket",
            "target_platform": "macos",
            "build_mode": "release_candidate_no_d4_publisher_identity",
            "source_ref": source_ref,
            "source_commit": commit,
            "source_archive_sha256": archive_hash,
            "source_receipt_sha256": receipt_hash,
            "source_manifest_sha256": manifest_hash,
            "export_id": receipt["export_id"],
            "app_id": app_id,
            "audit_store_id": audit_store_id,
            "bundle_identifier": bundle_id,
            "installer_identifier": installer_id,
            "authority_boundary": {
                "source_authority_verified": False,
                "owner_authorization_verified": False,
                "authority_strip": True,
                "credential_values_explicitly_supplied": False,
                "credential_inherited": False,
                "permission_inherited": False,
                "approval_inherited": False,
                "audit_chain_inherited": False,
            },
            "module_plan": {
                "catalog_version": catalog["version"],
                "selection_mode": plan.selection_mode,
                "requested_optional_module_ids": list(plan.requested_optional_ids),
                "unprunable_core_ids": catalog["unprunable_core_ids"],
                "included_module_ids": list(plan.included_module_ids),
                "excluded_optional_module_ids": list(plan.excluded_optional_ids),
            },
            "toolchain": toolchain,
            "target_architecture": target_architecture,
            "macos_binary_architecture_scan": macos_binary_architecture_scan,
            "flutter_pub_get_arguments": ["pub", "get", "--enforce-lockfile"],
            "flutter_build_arguments": [
                "build", "macos", "--release", "--no-pub", "--analyze-size",
                "--code-size-directory=<isolated-size-analysis>",
                *[f"--dart-define={name}={str(enabled).lower()}" for name, enabled in defines.items()],
                f"--dart-define=GUI_SHELL_PRODUCT_VERSION={version}",
            ],
            "aot_surface_analysis": aot_surface_analysis,
            "compile_time_identity": {
                "source": "schema_validated_manifest",
                "app_id": app_id,
                "audit_store_id": audit_store_id,
                "manifest_sha256": manifest_hash,
            },
            "compile_time_update_trust": _update_trust_summary(manifest_file),
            "build_duration_ms": elapsed_ms,
            "runtime_storage_root_template": "$HOME/Library/Application Support/D4Pocket/apps/<App ID>/stores/<Audit Store ID>",
            "bundle_name": bundle_name,
            "bundle_identifier_verified": True,
            "product_manifest_copied_byte_for_byte": True,
            "runtime_manifest_consumed": False,
            "module_pruning_verified": False,
            "package_signed": False,
            "app_publisher_identity_signed": False,
            "app_code_signature_scan": app_code_signature_scan,
            "launch_status": "not_verified",
            "installed_product_verified": False,
            "formal_distribution_claimed": False,
            "artifact_root": ".",
            "artifact_files": records,
            "artifact_total_bytes": total_bytes,
            "artifact_tree_sha256": tree_hash,
            "package": {
                "file_name": package_path.name,
                "format": "macOS flat pkg",
                "byte_length": package_path.stat().st_size,
                "sha256": _sha256_file(package_path),
                "signed": False,
                "installed_product_verified": False,
            },
            "uninstaller_package": {
                "file_name": uninstaller_package_path.name,
                "identifier": uninstaller_id,
                "format": "macOS flat pkg",
                "byte_length": uninstaller_package_path.stat().st_size,
                "sha256": _sha256_file(uninstaller_package_path),
                "signed": False,
                "removed_scope": "matching_app_bundle_only",
                "user_data_preserved": True,
                "keychain_preserved": True,
            },
            "app_archive": {
                "file_name": app_archive_path.name,
                "byte_length": app_archive_path.stat().st_size,
                "sha256": _sha256_file(app_archive_path),
            },
            "credential_artifact_scan": credential_scan,
        }
        _validate_build_evidence(
            evidence,
            staging_root,
            app_bundle,
            manifest_raw,
            flutter_framework_reference=flutter_framework,
            lipo=tools["lipo"],
            verification_directory=staging_root,
            environment=environment,
            profiled_app=built_app,
            size_analysis_input_directory=size_analysis_input_directory,
        )
        (staging_root / RECEIPT_NAME).write_text(
            json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
        )
        if output.exists() or output.is_symlink():
            raise ValueError("build中にoutput directoryが作成されたため公開を停止した")
        os.rename(staging_root, output)
    return evidence


def _validate_build_evidence(
    evidence: dict[str, Any],
    root: Path,
    app_bundle: Path,
    manifest_raw: bytes,
    *,
    flutter_framework_reference: FlutterFrameworkReference,
    lipo: str,
    verification_directory: Path,
    environment: dict[str, str],
    profiled_app: Path,
    size_analysis_input_directory: Path,
) -> None:
    schema = json.loads(
        (ROOT / "specs/gui_shell_macos_export_build.schema.json").read_text(encoding="utf-8")
    )
    if validate_instance(evidence, schema):
        raise ValueError("macOS Export build evidenceが専用Schemaに適合しない")
    _validate_macos_aot_surface_evidence(
        evidence,
        root,
        profiled_app,
        app_bundle,
        size_analysis_input_directory,
    )
    manifest = _parse_json_bytes(manifest_raw)
    expected_app_id = manifest["manifest"]["app_identity"]["app_id"]
    expected_audit_id = manifest["manifest"]["audit_store"]["store_id"]
    expected_manifest_hash = hashlib.sha256(manifest_raw).hexdigest()
    expected_bundle_id = bundle_identifier_for_app(expected_app_id)
    expected_bundle_name = f"D4Pocket-{expected_app_id.removeprefix('d4-pocket-app-')[:12]}.app"
    expected_uninstaller_name = (
        f"D4Pocket-{expected_app_id.removeprefix('d4-pocket-app-')[:12]}-Uninstaller.pkg"
    )
    expected_uninstaller_id = uninstaller_package_identifier_for_app(expected_bundle_id)
    uninstaller_evidence = evidence.get("uninstaller_package", {})
    if (
        uninstaller_evidence.get("file_name") != expected_uninstaller_name
        or uninstaller_evidence.get("identifier") != expected_uninstaller_id
        or uninstaller_evidence.get("format") != "macOS flat pkg"
        or uninstaller_evidence.get("signed") is not False
        or uninstaller_evidence.get("removed_scope") != "matching_app_bundle_only"
        or uninstaller_evidence.get("user_data_preserved") is not True
        or uninstaller_evidence.get("keychain_preserved") is not True
    ):
        raise ValueError("macOS uninstaller evidenceがExport App identityまたは削除範囲に一致しない")
    records, _regular_files, total_bytes, tree_hash = _macos_artifact_inventory(root, app_bundle)
    signature_scan = evidence.get("app_code_signature_scan", {})
    flutter_framework = signature_scan.get("flutter_toolchain_framework", {})
    embedded_flutter_framework = app_bundle.joinpath(*APP_FLUTTER_FRAMEWORK_RELATIVE_PATH.parts)
    embedded_flutter_binary = embedded_flutter_framework.joinpath(
        *FLUTTER_MACOS_FRAMEWORK_BINARY_RELATIVE_PATH.parts
    )
    try:
        source_framework_tree_hash = _macos_bundle_tree_sha256(
            flutter_framework_reference.source_path
        )
        embedded_flutter_tree_hash = _macos_bundle_tree_sha256(embedded_flutter_framework)
        source_binary_hash, embedded_copy_binary_hash = _verify_flutter_framework_copy_entries(
            flutter_framework_reference.source_path, embedded_flutter_framework
        )
        binary_metadata = embedded_flutter_binary.lstat()
        embedded_flutter_binary_hash = (
            _sha256_file(embedded_flutter_binary)
            if stat.S_ISREG(binary_metadata.st_mode) and not stat.S_ISLNK(binary_metadata.st_mode)
            else ""
        )
        architecture_slice_hash = _flutter_framework_architecture_slice_sha256(
            flutter_framework_reference.source_path,
            lipo=lipo,
            architecture=evidence.get("target_architecture", ""),
            verification_directory=verification_directory,
            environment=environment,
        )
    except (OSError, ValueError):
        source_framework_tree_hash = ""
        embedded_flutter_tree_hash = ""
        source_binary_hash = ""
        embedded_copy_binary_hash = ""
        embedded_flutter_binary_hash = ""
        architecture_slice_hash = ""
    if (
        evidence.get("version") != 7
        or evidence.get("evidence_type") != "d4_pocket_macos_export_build"
        or evidence.get("target_platform") != "macos"
        or evidence.get("app_id") != expected_app_id
        or evidence.get("audit_store_id") != expected_audit_id
        or evidence.get("bundle_identifier") != expected_bundle_id
        or evidence.get("installer_identifier") != f"{expected_bundle_id}.installer"
        or evidence.get("bundle_name") != expected_bundle_name
        or evidence.get("compile_time_identity", {}).get("app_id") != expected_app_id
        or evidence.get("compile_time_identity", {}).get("audit_store_id") != expected_audit_id
        or evidence.get("compile_time_identity", {}).get("manifest_sha256") != expected_manifest_hash
        or evidence.get("source_manifest_sha256") != expected_manifest_hash
        or evidence.get("artifact_files") != records
        or evidence.get("artifact_total_bytes") != total_bytes
        or evidence.get("artifact_tree_sha256") != tree_hash
        or evidence.get("formal_distribution_claimed") is not False
        or evidence.get("installed_product_verified") is not False
        or evidence.get("package_signed") is not False
        or evidence.get("app_publisher_identity_signed") is not False
        or evidence.get("build_mode") != "release_candidate_no_d4_publisher_identity"
        or signature_scan.get("status") != "passed_no_d4_publisher_identity"
        or signature_scan.get("code_objects_checked", 0) < 2
        or (
            signature_scan.get("unsigned_code_objects", -1)
            + signature_scan.get("adhoc_executable_signatures", -1)
            + signature_scan.get("sandboxed_adhoc_app_bundle_signatures", -1)
            + signature_scan.get("generated_adhoc_aot_framework_signatures", -1)
            + signature_scan.get("verified_toolchain_framework_signatures", -1)
            != signature_scan.get("code_objects_checked", -2)
        )
        or signature_scan.get("sandboxed_adhoc_app_bundle_signatures") != 1
        or signature_scan.get("generated_adhoc_aot_framework_signatures") != 1
        or signature_scan.get("verified_toolchain_framework_signatures") != 1
        or signature_scan.get("d4_publisher_identity_signatures") != 0
        or signature_scan.get("d4_resource_sealed_bundle_signatures") != 2
        or signature_scan.get("sandbox_entitlements", {}).get("status")
        != "passed_exact_expected_entitlements"
        or signature_scan.get("sandbox_entitlements", {}).get("app_sandbox_entitlements_verified") is not True
        or signature_scan.get("sandbox_entitlements", {}).get("helper_inherited_sandbox_entitlements_verified") is not True
        or signature_scan.get("sandbox_entitlements", {}).get("app_entitlements_sha256")
        != hashlib.sha256(
            plistlib.dumps(
                EXPECTED_MACOS_APP_ENTITLEMENTS,
                fmt=plistlib.FMT_XML,
                sort_keys=True,
            )
        ).hexdigest()
        or signature_scan.get("sandbox_entitlements", {}).get("helper_entitlements_sha256")
        != hashlib.sha256(
            plistlib.dumps(
                EXPECTED_MACOS_BROKER_HELPER_ENTITLEMENTS,
                fmt=plistlib.FMT_XML,
                sort_keys=True,
            )
        ).hexdigest()
        or evidence.get("toolchain", {}).get("flutter_sdk_revision") != PINNED_FLUTTER_REVISION
        or re.fullmatch(
            r"[0-9a-f]{40}", evidence.get("toolchain", {}).get("flutter_engine_revision", "")
        ) is None
        or flutter_framework.get("source") != "pinned_flutter_release_engine_cache"
        or flutter_framework.get("sdk_revision") != PINNED_FLUTTER_REVISION
        or flutter_framework.get("engine_revision")
        != evidence.get("toolchain", {}).get("flutter_engine_revision")
        or flutter_framework.get("engine_revision")
        != flutter_framework_reference.engine_revision
        or flutter_framework.get("source_relative_path")
        != FLUTTER_ENGINE_FRAMEWORK_RELATIVE_PATH.as_posix()
        or flutter_framework.get("embedded_relative_path")
        != APP_FLUTTER_FRAMEWORK_RELATIVE_PATH.as_posix()
        or source_framework_tree_hash != flutter_framework_reference.tree_sha256
        or flutter_framework.get("source_tree_sha256") != source_framework_tree_hash
        or flutter_framework.get("embedded_tree_sha256") != embedded_flutter_tree_hash
        or flutter_framework.get("source_binary_sha256") != source_binary_hash
        or flutter_framework.get("embedded_binary_sha256") != embedded_copy_binary_hash
        or flutter_framework.get("embedded_binary_sha256") != embedded_flutter_binary_hash
        or flutter_framework.get("architecture_slice_sha256") != architecture_slice_hash
        or flutter_framework.get("embedded_binary_sha256") != architecture_slice_hash
        or flutter_framework.get("binary_architecture") != evidence.get("target_architecture")
        or flutter_framework.get("binary_architecture_slice_verified") is not True
        or flutter_framework.get("non_binary_content_verified") is not True
        or flutter_framework.get("framework_copy_transform_verified") is not True
        or flutter_framework.get("strict_signature_verified") is not True
        or any(
            flutter_framework.get(field) is not True
            for field in (
                "authority_present",
                "team_identifier_set",
                "timestamp_present",
                "cms_digest_present",
                "sealed_resources_present",
            )
        )
        or evidence.get("target_architecture") != evidence.get("toolchain", {}).get("architecture")
        or evidence.get("macos_binary_architecture_scan", {}).get("status") != "passed_host_architecture"
        or evidence.get("macos_binary_architecture_scan", {}).get("architecture") != evidence.get("target_architecture")
        or evidence.get("macos_binary_architecture_scan", {}).get("binary_files_checked") != 2
        or evidence.get("product_manifest_copied_byte_for_byte") is not True
        or (app_bundle / "Contents/Resources/product_manifest.json").read_bytes() != manifest_raw
    ):
        raise ValueError("macOS Export build evidenceが候補の署名／Publisher identity条件に適合しない")
    package = root / evidence["package"]["file_name"]
    uninstaller_package = root / uninstaller_evidence["file_name"]
    archive = root / evidence["app_archive"]["file_name"]
    if (
        _sha256_file(package) != evidence["package"]["sha256"]
        or package.stat().st_size != evidence["package"]["byte_length"]
        or _sha256_file(uninstaller_package) != uninstaller_evidence["sha256"]
        or uninstaller_package.stat().st_size != uninstaller_evidence["byte_length"]
        or _sha256_file(archive) != evidence["app_archive"]["sha256"]
        or archive.stat().st_size != evidence["app_archive"]["byte_length"]
        or evidence["credential_artifact_scan"]["artifact_tree_sha256"]
        != evidence["artifact_tree_sha256"]
    ):
        raise ValueError("macOS Export build evidenceのArtifact identityが一致しない")


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Broker Receipt／ManifestからPublisher identityなしのmacOS .app／.pkg候補を構築する"
    )
    parser.add_argument("--receipt", type=Path, required=True)
    parser.add_argument("--manifest-file", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument(
        "--allow-verification-branch",
        action="store_true",
        help="cleanでremote HEAD一致のcodex/* branchを手動検証buildに限り許可する",
    )
    args = parser.parse_args()
    try:
        evidence = build_macos_product(
            args.receipt,
            args.manifest_file,
            args.output_dir,
            allow_verification_branch=args.allow_verification_branch,
        )
    except (OSError, ValueError, subprocess.CalledProcessError, json.JSONDecodeError, tarfile.TarError) as error:
        if isinstance(error, ValueError):
            message = f": {error}"
        else:
            detail = _safe_export_error_detail(error)
            message = f": {detail}" if detail else ""
        print(f"macOS Export buildを停止した: {type(error).__name__}{message}", file=sys.stderr)
        return 1
    print(json.dumps({
        "output_dir": str(args.output_dir.resolve()),
        "source_commit": evidence["source_commit"],
        "app_id": evidence["app_id"],
        "audit_store_id": evidence["audit_store_id"],
        "bundle_identifier": evidence["bundle_identifier"],
        "package_sha256": evidence["package"]["sha256"],
        "uninstaller_package_sha256": evidence["uninstaller_package"]["sha256"],
        "artifact_tree_sha256": evidence["artifact_tree_sha256"],
        "signed": False,
        "installed_product_verified": False,
    }, ensure_ascii=False, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
