"""Broker Export Receiptからunsigned macOS App Bundle／pkg候補を構築する開発tool。"""

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


def _patch_macos_export_signing(source_root: Path) -> None:
    project = source_root / "apps/desktop_flutter/macos/Runner.xcodeproj/project.pbxproj"
    raw = _read_regular_file(project, 2 * 1024 * 1024, "macOS Runner project").decode("utf-8")
    configuration_count = raw.count("isa = XCBuildConfiguration;")
    settings_count = raw.count("buildSettings = {")
    if configuration_count == 0 or configuration_count != settings_count:
        raise ValueError("macOS Xcode build configurationの構造が想定と一致しない")
    raw, patched_count = re.subn(
        r"\bbuildSettings = \{",
        "buildSettings = { CODE_SIGNING_ALLOWED = NO; CODE_SIGNING_REQUIRED = NO;",
        raw,
    )
    if patched_count != configuration_count:
        raise ValueError("macOS Export用の署名無効設定を全configurationへ適用できない")
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


def _macos_code_objects(bundle: Path) -> list[Path]:
    if bundle.is_symlink() or not bundle.is_dir():
        raise ValueError("署名検査対象のmacOS App Bundleが通常directoryではない")
    objects = [bundle]
    for path in sorted(bundle.rglob("*"), key=lambda item: item.as_posix()):
        metadata = path.lstat()
        if stat.S_ISLNK(metadata.st_mode):
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


def _require_unsigned_macos_code(
    bundle: Path, *, codesign: str, environment: dict[str, str]
) -> dict[str, Any]:
    code_objects = _macos_code_objects(bundle)
    if len(code_objects) < 2:
        raise ValueError("macOS App Bundle内の実行codeを署名検査できない")
    probe_environment = dict(environment)
    probe_environment.update({"LANG": "C", "LC_ALL": "C"})
    for path in code_objects:
        result = subprocess.run(
            [codesign, "--display", "--verbose=2", str(path)],
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            encoding="utf-8",
            errors="replace",
            env=probe_environment,
            check=False,
        )
        if result.returncode == 0:
            raise ValueError("macOS Export candidateに署名済みcode objectがある")
        if "code object is not signed at all" not in result.stdout.casefold():
            raise ValueError("macOS code signing状態をunsignedとして確定できない")
    return {"status": "passed_unsigned", "code_objects_checked": len(code_objects)}


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
            _patch_macos_export_signing(source_root)

            toolchain = {
                "architecture": platform.machine().lower(),
                "flutter": _tool_version([tools["flutter"], "--version"], cwd=flutter_project, environment=environment),
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
            _require_file(built_app / "Contents/Info.plist", "macOS Info.plist")
            built_bundle_id = _run(
                [tools["plutil"], "-extract", "CFBundleIdentifier", "raw", "-o", "-", str(built_app / "Contents/Info.plist")],
                cwd=source_root,
                environment=environment,
            ).strip()
            if built_bundle_id != bundle_id:
                raise ValueError("built App Bundle identifierがExport App identityと一致しない")
            built_helper = built_app / "Contents/MacOS/gui_shell_macos_broker"
            _require_file(built_helper, "macOS Broker helper")
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
            app_code_signature_scan = _require_unsigned_macos_code(
                app_bundle, codesign=tools["codesign"], environment=environment
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
            "version": 1,
            "evidence_type": "d4_pocket_macos_export_build",
            "evidence_source": "INTERNAL_STATE",
            "product": "D4 Pocket",
            "target_platform": "macos",
            "build_mode": "release_unsigned_candidate",
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
            "app_signed": False,
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
        _validate_build_evidence(evidence, staging_root, app_bundle, manifest_raw)
        (staging_root / RECEIPT_NAME).write_text(
            json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
        )
        if output.exists() or output.is_symlink():
            raise ValueError("build中にoutput directoryが作成されたため公開を停止した")
        os.rename(staging_root, output)
    return evidence


def _validate_build_evidence(
    evidence: dict[str, Any], root: Path, app_bundle: Path, manifest_raw: bytes
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
        or evidence.get("app_signed") is not False
        or evidence.get("app_code_signature_scan", {}).get("status") != "passed_unsigned"
        or evidence.get("app_code_signature_scan", {}).get("code_objects_checked", 0) < 2
        or evidence.get("product_manifest_copied_byte_for_byte") is not True
        or (app_bundle / "Contents/Resources/product_manifest.json").read_bytes() != manifest_raw
    ):
        raise ValueError("macOS Export build evidenceが候補の未署名／identity条件に適合しない")
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
        description="Broker Receipt／Manifestからunsigned macOS .app／.pkg候補を構築する"
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
