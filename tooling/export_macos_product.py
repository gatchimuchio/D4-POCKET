"""Broker Export ReceiptからPublisher identityなしのmacOS App Bundle／pkg候補を構築する開発tool。"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import re
import shutil
import stat
import subprocess
import sys
import tarfile
import tempfile
import time
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from tooling.build_module_pruned_windows import _load_catalog, dart_defines
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
# These modes and exclusions mirror the pinned Flutter macOS unpack target.
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


def _macos_code_objects(bundle: Path, *, collapsed_bundle: Path) -> list[Path]:
    if bundle.is_symlink() or not bundle.is_dir():
        raise ValueError("署名検査対象のmacOS App Bundleが通常directoryではない")
    objects = [bundle]
    for path in sorted(bundle.rglob("*"), key=lambda item: item.as_posix()):
        metadata = path.lstat()
        if stat.S_ISLNK(metadata.st_mode):
            continue
        if path != collapsed_bundle and path.is_relative_to(collapsed_bundle):
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


def _flutter_framework_copy_exclusions(
    source_entries: dict[str, dict[str, Any]],
) -> set[str]:
    excluded: set[str] = set()
    excluded_directories = {
        path
        for path, entry in source_entries.items()
        if entry["kind"] == "directory" and PurePosixPath(path).name == ".DS_Store"
    }
    for directory in excluded_directories:
        excluded.update(
            path
            for path in source_entries
            if path == directory or path.startswith(directory + "/")
        )
    excluded.update(
        path
        for path, entry in source_entries.items()
        if entry["kind"] == "file"
        and PurePosixPath(path).name in FLUTTER_MACOS_FRAMEWORK_DENYLISTED_FILES
    )
    return excluded


def _verify_flutter_framework_copy_entries(
    source_framework: Path, embedded_framework: Path
) -> tuple[str, str]:
    source_entries = _macos_bundle_entries(source_framework)
    embedded_entries = _macos_bundle_entries(embedded_framework)
    exclusions = _flutter_framework_copy_exclusions(source_entries)
    expected_paths = set(source_entries) - exclusions
    if set(embedded_entries) != expected_paths:
        raise ValueError(
            "embedded FlutterMacOS.frameworkのfile layoutが固定Flutter copy規則と一致しない"
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


def _verify_pinned_flutter_framework_signature(
    framework: Path, *, codesign: str, environment: dict[str, str]
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
        raise ValueError("pinned FlutterMacOS.frameworkのstrict code signature検証に失敗した")
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
        source_path, codesign=codesign, environment=environment
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
        reference.source_path, codesign=codesign, environment=environment
    )
    if source_facts != reference.signature_facts:
        raise ValueError("Flutter SDK cache内Frameworkの署名metadataがbuild中に変化した")
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
        framework, codesign=codesign, environment=environment
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
    code_objects = _macos_code_objects(bundle, collapsed_bundle=framework_path)
    if len(code_objects) < 2:
        raise ValueError("macOS App Bundle内の実行codeを署名検査できない")
    probe_environment = dict(environment)
    probe_environment.update({"LANG": "C", "LC_ALL": "C"})
    unsigned_code_objects = 0
    adhoc_executable_signatures = 0
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
    return {
        "status": "passed_no_d4_publisher_identity",
        "code_objects_checked": len(code_objects),
        "unsigned_code_objects": unsigned_code_objects,
        "adhoc_executable_signatures": adhoc_executable_signatures,
        "verified_toolchain_framework_signatures": verified_toolchain_framework_signatures,
        "d4_publisher_identity_signatures": 0,
        "d4_resource_sealed_bundle_signatures": 0,
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
    tools = _tool_paths()
    target_architecture = _darwin_build_architecture()
    defines = dart_defines(plan)
    started = time.perf_counter()
    with tempfile.TemporaryDirectory(prefix=OUTPUT_TEMP_PREFIX, dir=output.parent) as output_temp:
        staging_root = Path(output_temp) / "product"
        staging_root.mkdir(mode=0o700)
        with tempfile.TemporaryDirectory(
            prefix=BUILD_TEMP_PREFIX, dir=_resolve_build_temp_root()
        ) as build_temp:
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
            flutter_arguments = [
                "build", "macos", "--release", "--no-pub",
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
            app_code_signature_scan = _require_macos_code_without_publisher_identity(
                app_bundle,
                flutter_framework=flutter_framework,
                codesign=tools["codesign"],
                lipo=tools["lipo"],
                architecture=target_architecture,
                verification_directory=temporary_root,
                environment=environment,
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
            "version": 3,
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
                "build", "macos", "--release", "--no-pub",
                *[f"--dart-define={name}={str(enabled).lower()}" for name, enabled in defines.items()],
                f"--dart-define=GUI_SHELL_PRODUCT_VERSION={version}",
            ],
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
) -> None:
    schema = json.loads(
        (ROOT / "specs/gui_shell_macos_export_build.schema.json").read_text(encoding="utf-8")
    )
    if validate_instance(evidence, schema):
        raise ValueError("macOS Export build evidenceが専用Schemaに適合しない")
    manifest = _parse_json_bytes(manifest_raw)
    expected_app_id = manifest["manifest"]["app_identity"]["app_id"]
    expected_audit_id = manifest["manifest"]["audit_store"]["store_id"]
    expected_manifest_hash = hashlib.sha256(manifest_raw).hexdigest()
    expected_bundle_id = bundle_identifier_for_app(expected_app_id)
    expected_bundle_name = f"D4Pocket-{expected_app_id.removeprefix('d4-pocket-app-')[:12]}.app"
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
        evidence.get("evidence_type") != "d4_pocket_macos_export_build"
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
            + signature_scan.get("verified_toolchain_framework_signatures", -1)
            != signature_scan.get("code_objects_checked", -2)
        )
        or signature_scan.get("verified_toolchain_framework_signatures") != 1
        or signature_scan.get("d4_publisher_identity_signatures") != 0
        or signature_scan.get("d4_resource_sealed_bundle_signatures") != 0
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
    archive = root / evidence["app_archive"]["file_name"]
    if (
        _sha256_file(package) != evidence["package"]["sha256"]
        or package.stat().st_size != evidence["package"]["byte_length"]
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
        message = f": {error}" if isinstance(error, ValueError) else ""
        print(f"macOS Export buildを停止した: {type(error).__name__}{message}", file=sys.stderr)
        return 1
    print(json.dumps({
        "output_dir": str(args.output_dir.resolve()),
        "source_commit": evidence["source_commit"],
        "app_id": evidence["app_id"],
        "audit_store_id": evidence["audit_store_id"],
        "bundle_identifier": evidence["bundle_identifier"],
        "package_sha256": evidence["package"]["sha256"],
        "artifact_tree_sha256": evidence["artifact_tree_sha256"],
        "signed": False,
        "installed_product_verified": False,
    }, ensure_ascii=False, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
