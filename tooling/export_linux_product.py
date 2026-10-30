"""Broker Export Manifestから未署名Linux Release bundle／D4PKG01を組み立てる開発tool。"""

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
import tempfile
import time
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from tooling.build_module_pruned_windows import dart_defines
from tooling.export_windows_product import (
    CREDENTIAL_SCAN_STATUS,
    MAX_INPUT_BYTES,
    MAX_MANIFEST_BYTES,
    _artifact_inventory,
    _parse_json_bytes,
    _read_regular_file,
    _resolve_build_temp_root,
    _resolve_output_directory,
    _safe_extract_source,
    _scan_credential_inventory,
    _schema,
    _update_trust_summary,
    product_version_from_source,
    validate_export_inputs,
)
from tooling.package_linux_product import write_product_package
from tooling.schema_check.check_schemas import validate_instance


RECEIPT_NAME = "d4_pocket_linux_export_build.json"
BUILD_SCHEMA_NAME = "gui_shell_linux_export_build.schema.json"
BUILD_TEMP_PREFIX = "d4p-linux-build-"
OUTPUT_TEMP_PREFIX = "d4p-linux-output-"
ISOLATED_ENVIRONMENT_ALLOWLIST = {
    "PATH",
    "LANG",
    "LC_ALL",
    "TZ",
    "FLUTTER_ROOT",
    "RUSTUP_HOME",
    "PKG_CONFIG_PATH",
    "PKG_CONFIG_LIBDIR",
    "PKG_CONFIG_SYSROOT_DIR",
    "CC",
    "CXX",
}


def _git_value(*arguments: str) -> str:
    completed = subprocess.run(
        ["git", "-C", str(ROOT), *arguments],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        env=_linux_build_environment({}),
    )
    return completed.stdout.strip()


def _require_clean_source(*, allow_verification_branch: bool = False) -> tuple[str, str]:
    branch = _git_value("branch", "--show-current")
    if not branch or (branch != "main" and not allow_verification_branch):
        raise ValueError("Linux Export buildはmainのclean commitから実行する")
    if branch != "main" and (
        re.fullmatch(r"codex/[A-Za-z0-9._/-]+", branch) is None
        or ".." in branch
        or "//" in branch
        or branch.endswith(("/", "."))
    ):
        raise ValueError("手動検証branchはcodex/配下の安全な名前に限る")
    if _git_value("status", "--porcelain", "--untracked-files=all"):
        raise ValueError("Linux Export buildはcleanなcommit済みsourceから実行する")
    commit = _git_value("rev-parse", "HEAD")
    remote_ref = f"refs/heads/{branch}"
    remote_refs = _git_value("ls-remote", "origin", remote_ref).splitlines()
    remote_parts = remote_refs[0].split() if len(remote_refs) == 1 else []
    if len(remote_parts) != 2 or remote_parts[0] != commit or remote_parts[1] != remote_ref:
        raise ValueError("Linux Export build sourceとGitHub remote branch HEADが一致しない")
    return branch, commit


def _linux_build_environment(
    overrides: dict[str, str], *, inherited: dict[str, str] | None = None
) -> dict[str, str]:
    source = os.environ if inherited is None else inherited
    result = {
        name: value
        for name, value in source.items()
        if name.upper() in ISOLATED_ENVIRONMENT_ALLOWLIST
    }
    for name, value in overrides.items():
        for existing in tuple(result):
            if existing.upper() == name.upper():
                del result[existing]
        result[name] = value
    return result


def _flutter_build_environment(environment: dict[str, str]) -> dict[str, str]:
    # Flutter's first-run analytics notice is appended to stdout unless it detects CI;
    # that breaks the machine-readable JSON emitted by `flutter --version --machine`.
    result = {name: value for name, value in environment.items() if name.upper() != "CI"}
    result["CI"] = "true"
    return result


def _rustup_home() -> str:
    configured = os.environ.get("RUSTUP_HOME", "").strip()
    if configured:
        return str(Path(configured).expanduser().resolve(strict=True))
    rustup = shutil.which("rustup")
    if rustup is None:
        raise ValueError("Rust toolchainのRUSTUP_HOMEを特定できない")
    completed = subprocess.run(
        [rustup, "show", "home"],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        env=_linux_build_environment({}),
    )
    value = completed.stdout.strip()
    if not value:
        raise ValueError("rustupがhome pathを返さなかった")
    return str(Path(value).resolve(strict=True))


def _tool_paths() -> dict[str, str]:
    names = ("flutter", "cargo", "rustc", "cmake", "ninja", "pkg-config", "clang", "clang++")
    paths = {name: shutil.which(name) for name in names}
    if any(value is None for value in paths.values()):
        missing = [name for name, value in paths.items() if value is None]
        raise ValueError("Linux Export toolchain commandがPATHにない: " + ", ".join(missing))
    return {name: str(value) for name, value in paths.items()}


def _tool_version(command: list[str], environment: dict[str, str]) -> str:
    completed = subprocess.run(
        command,
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        env=environment,
    )
    lines = completed.stdout.splitlines()
    if not lines or not lines[0].strip():
        raise ValueError("Linux build toolがversionを返さなかった")
    return lines[0].strip()


def _toolchain(paths: dict[str, str], environment: dict[str, str]) -> dict[str, str]:
    flutter_environment = _flutter_build_environment(environment)
    flutter_result = subprocess.run(
        [paths["flutter"], "--version", "--machine"],
        cwd=ROOT / "apps" / "desktop_flutter",
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        env=flutter_environment,
    )
    flutter_value = json.loads(flutter_result.stdout)
    versions = {
        "flutter": str(flutter_value.get("frameworkVersion", "unknown")),
        "dart": str(flutter_value.get("dartSdkVersion", "unknown")),
        "framework_revision": str(flutter_value.get("frameworkRevision", "unknown")),
    }
    versions.update(
        {
            "cargo": _tool_version([paths["cargo"], "--version"], environment),
            "rustc": _tool_version([paths["rustc"], "--version"], environment),
            "cmake": _tool_version([paths["cmake"], "--version"], environment),
            "ninja": _tool_version([paths["ninja"], "--version"], environment),
            "pkg_config": _tool_version([paths["pkg-config"], "--version"], environment),
            "clang": _tool_version([paths["clang"], "--version"], environment),
            "clangxx": _tool_version([paths["clang++"], "--version"], environment),
            "architecture": platform.machine() or "unknown",
        }
    )
    if hasattr(platform, "freedesktop_os_release"):
        release = platform.freedesktop_os_release()
        versions["distribution_id"] = release.get("ID", "unknown")
        versions["distribution_version"] = release.get("VERSION_ID", "unknown")
    else:
        versions["distribution_id"] = "unknown"
        versions["distribution_version"] = "unknown"
    if any(not value or value == "unknown" for value in versions.values()):
        raise ValueError("Linux build toolchain／host identityのversion情報が不足している")
    return versions


def _sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _write_source_archive(commit: str, archive_path: Path) -> str:
    with archive_path.open("xb") as archive:
        subprocess.run(
            ["git", "-C", str(ROOT), "archive", "--format=tar", commit],
            check=True,
            stdout=archive,
            env=_linux_build_environment({}),
        )
    return _sha256_file(archive_path)


def validate_build_evidence(
    evidence: dict[str, Any], bundle: Path, package_path: Path, manifest_raw: bytes
) -> None:
    schema = _schema(BUILD_SCHEMA_NAME)
    errors = validate_instance(evidence, schema)
    if errors:
        raise ValueError("Linux Export build evidenceがSchemaに適合しない")
    app_id = evidence["app_id"]
    audit_store_id = evidence["audit_store_id"]
    if evidence["compile_time_identity"] != {
        "source": "schema_validated_manifest",
        "app_id": app_id,
        "audit_store_id": audit_store_id,
    }:
        raise ValueError("compile-time identityがBroker Export Manifestと一致しない")
    records, total_bytes, tree_hash = _artifact_inventory(bundle)
    if (
        records != evidence["artifact_files"]
        or total_bytes != evidence["artifact_total_bytes"]
        or tree_hash != evidence["artifact_tree_sha256"]
    ):
        raise ValueError("Linux Export evidenceとbundle inventoryが一致しない")
    product_manifest = _read_regular_file(
        bundle / "product_manifest.json", MAX_MANIFEST_BYTES, "Linux Product Manifest"
    )
    if product_manifest != manifest_raw:
        raise ValueError("Linux Product Manifestのbyte列がBroker入力から変化した")
    parsed_manifest = _parse_json_bytes(product_manifest)
    if (
        parsed_manifest["manifest"]["app_identity"]["app_id"] != app_id
        or parsed_manifest["manifest"]["audit_store"]["store_id"] != audit_store_id
        or parsed_manifest["manifest"]["app_identity"]["target_platform"] != "linux"
    ):
        raise ValueError("bundle Manifest identity／targetがbuild evidenceと一致しない")
    if _update_trust_summary(parsed_manifest) != evidence["compile_time_update_trust"]:
        raise ValueError("Rust compile-time update trustがManifestと一致しない")
    scan = _scan_credential_inventory(bundle, records, total_bytes, tree_hash)
    if (
        scan != evidence["credential_artifact_scan"]
        or evidence["authority_boundary"]["credential_artifact_scan_status"] != scan["status"]
    ):
        raise ValueError("bounded Credential scanがLinux bundle hashへ結合されない")
    metadata = package_path.lstat()
    if stat.S_ISLNK(metadata.st_mode) or not stat.S_ISREG(metadata.st_mode):
        raise ValueError("Linux D4PKG01 outputが通常fileではない")
    package_size = metadata.st_size
    package_hash = _sha256_file(package_path)
    package_record = evidence["package"]
    if (
        package_record["byte_length"] != package_size
        or package_record["sha256"] != package_hash
        or package_record["file_name"] != package_path.name
    ):
        raise ValueError("Linux D4PKG01がbuild evidenceと一致しない")


def build_linux_product(
    receipt_path: Path,
    manifest_path: Path,
    output_dir: Path,
    *,
    allow_verification_branch: bool = False,
    r2_e2e_test_build: bool = False,
) -> dict[str, Any]:
    if not sys.platform.startswith("linux") or not hasattr(os, "O_NOFOLLOW"):
        raise ValueError("Linux Export buildはLinux host専用である")
    source_branch, commit = _require_clean_source(
        allow_verification_branch=allow_verification_branch
    )
    output = _resolve_output_directory(output_dir)
    receipt_raw = _read_regular_file(receipt_path, MAX_INPUT_BYTES, "Export Receipt")
    manifest_raw = _read_regular_file(manifest_path, MAX_MANIFEST_BYTES, "Export Manifest file")
    receipt, manifest_file, plan, receipt_hash, manifest_hash = validate_export_inputs(
        receipt_raw, manifest_raw, target_platform="linux"
    )
    if manifest_path.name != receipt["manifest_file"]["file_name"]:
        raise ValueError("入力Manifest file名がBroker Receiptと一致しない")
    app_id = manifest_file["manifest"]["app_identity"]["app_id"]
    audit_store_id = manifest_file["manifest"]["audit_store"]["store_id"]
    if r2_e2e_test_build:
        if not allow_verification_branch:
            raise ValueError("R2 E2E buildは明示された手動検証buildだけで有効化できる")
        if (
            app_id != "d4-pocket-app-11111111111111111111111111111111"
            or audit_store_id != "audit-store-22222222222222222222222222222222"
        ):
            raise ValueError("R2 E2E buildは固定test identityだけを受け付ける")
    version = product_version_from_source(ROOT)
    paths = _tool_paths()
    rustup_home = _rustup_home()
    defines = dart_defines(plan)

    build_started = time.perf_counter()
    with tempfile.TemporaryDirectory(prefix=OUTPUT_TEMP_PREFIX, dir=output.parent) as output_temp:
        staging_root = Path(output_temp) / "product"
        staging_root.mkdir()
        bundle = staging_root / "bundle"
        package_path = staging_root / "D4Pocket-linux.d4pkg"
        with tempfile.TemporaryDirectory(
            prefix=BUILD_TEMP_PREFIX, dir=_resolve_build_temp_root()
        ) as build_temp:
            temporary_root = Path(build_temp)
            isolated_home = temporary_root / "home"
            isolated_home.mkdir(mode=0o700)
            source_root = temporary_root / "source"
            source_root.mkdir()
            archive_path = temporary_root / "source.tar"
            archive_hash = _write_source_archive(commit, archive_path)
            _safe_extract_source(archive_path, source_root)

            flutter_project = source_root / "apps" / "desktop_flutter"
            build_dir = flutter_project / "build" / "linux" / "x64" / "release"
            product_version = product_version_from_source(source_root)
            update_trust = manifest_file.get("update_trust")
            build_overrides = {
                "HOME": str(isolated_home),
                "XDG_CACHE_HOME": str(temporary_root / "cache"),
                "TMPDIR": str(temporary_root / "tmp"),
                "PUB_CACHE": str(temporary_root / "pub"),
                "CARGO_HOME": str(temporary_root / "cargo"),
                "RUSTUP_HOME": rustup_home,
                "GUI_SHELL_UPDATE_TRUST_JSON": json.dumps(
                    update_trust, ensure_ascii=False, sort_keys=True, separators=(",", ":")
                ) if update_trust is not None else "",
            }
            (temporary_root / "cache").mkdir()
            (temporary_root / "tmp").mkdir()
            (temporary_root / "pub").mkdir()
            (temporary_root / "cargo").mkdir()
            if update_trust is None:
                build_overrides.pop("GUI_SHELL_UPDATE_TRUST_JSON")
            child_environment = _linux_build_environment(build_overrides)
            flutter_environment = _flutter_build_environment(child_environment)
            toolchain = _toolchain(paths, child_environment)
            pub_arguments = ["pub", "get", "--enforce-lockfile"]
            subprocess.run(
                [paths["flutter"], *pub_arguments],
                cwd=flutter_project,
                env=flutter_environment,
                check=True,
            )
            flutter_arguments = [
                "build",
                "linux",
                "--release",
                "--no-pub",
                "--target-platform=linux-x64",
                "--config-only",
                *[
                    f"--dart-define={name}={str(enabled).lower()}"
                    for name, enabled in defines.items()
                ],
                f"--dart-define=GUI_SHELL_PRODUCT_VERSION={product_version}",
            ]
            subprocess.run(
                [paths["flutter"], *flutter_arguments],
                cwd=flutter_project,
                env=flutter_environment,
                check=True,
            )

            cmake_configure_arguments = [
                "-S", str(flutter_project / "linux"),
                "-B", str(build_dir),
                "-G", "Ninja",
                "-DCMAKE_BUILD_TYPE=Release",
                "-DFLUTTER_TARGET_PLATFORM=linux-x64",
                f"-DCMAKE_INSTALL_PREFIX={bundle}",
                f"-DD4_PRODUCT_APP_ID={app_id}",
                f"-DD4_PRODUCT_AUDIT_STORE_ID={audit_store_id}",
                f"-DD4_LINUX_R2_E2E_TEST_BUILD={'ON' if r2_e2e_test_build else 'OFF'}",
            ]
            subprocess.run(
                [paths["cmake"], *cmake_configure_arguments],
                cwd=source_root,
                env={**child_environment, "CC": "clang", "CXX": "clang++"},
                check=True,
            )
            cmake_build_arguments = ["--build", str(build_dir), "--target", "install"]
            subprocess.run(
                [paths["cmake"], *cmake_build_arguments],
                cwd=source_root,
                env=child_environment,
                check=True,
            )
            if not bundle.is_dir() or bundle.is_symlink():
                raise ValueError("CMake installが固定Linux bundleを生成しなかった")
            manifest_target = bundle / "product_manifest.json"
            if manifest_target.exists() or manifest_target.is_symlink():
                raise ValueError("build bundleに入力Manifest名と衝突するfileがある")
            with manifest_target.open("xb") as destination:
                destination.write(manifest_raw)
                destination.flush()
                os.fsync(destination.fileno())

            package_result = write_product_package(
                bundle, package_path, product_version=product_version
            )
            artifact_files, total_bytes, tree_hash = _artifact_inventory(bundle)
            credential_scan = _scan_credential_inventory(
                bundle, artifact_files, total_bytes, tree_hash
            )
            elapsed_ms = round((time.perf_counter() - build_started) * 1000)
            evidence = {
                "version": 1,
                "evidence_type": "d4_pocket_linux_export_build",
                "evidence_source": "INTERNAL_STATE",
                "product": "D4 Pocket",
                "target_platform": "linux",
                "build_mode": "release",
                "source_ref": f"refs/heads/{source_branch}",
                "source_commit": commit,
                "source_archive_sha256": archive_hash,
                "source_receipt_sha256": receipt_hash,
                "source_manifest_sha256": manifest_hash,
                "export_id": receipt["export_id"],
                "app_id": app_id,
                "audit_store_id": audit_store_id,
                "authority_boundary": {
                    "source_authority_verified": False,
                    "owner_authorization_verified": False,
                    "authority_strip": True,
                    "credential_values_explicitly_supplied": False,
                    "credential_artifact_scan_status": CREDENTIAL_SCAN_STATUS,
                    "credential_inherited": False,
                    "permission_inherited": False,
                    "approval_inherited": False,
                    "audit_chain_inherited": False,
                },
                "module_plan": {
                    "catalog_version": 1,
                    "selection_mode": plan.selection_mode,
                    "requested_optional_module_ids": list(plan.requested_optional_ids),
                    "unprunable_core_ids": receipt["export_manifest"]["module_plan"]["unprunable_core_ids"],
                    "included_module_ids": list(plan.included_module_ids),
                    "excluded_optional_module_ids": list(plan.excluded_optional_ids),
                },
                "toolchain": toolchain,
                "pub_get_arguments": pub_arguments,
                "flutter_build_arguments": flutter_arguments,
                "cmake_configure_arguments": cmake_configure_arguments,
                "cmake_build_arguments": cmake_build_arguments,
                "compile_time_identity": {
                    "source": "schema_validated_manifest",
                    "app_id": app_id,
                    "audit_store_id": audit_store_id,
                },
                "compile_time_update_trust": _update_trust_summary(manifest_file),
                "build_duration_ms": elapsed_ms,
                "runtime_storage_root_template": "$XDG_DATA_HOME/apps/d4-pocket/<App ID>/stores/<Audit store ID>",
                "portable_bundle_assembled": True,
                "manifest_copied_byte_for_byte": True,
                "runtime_manifest_consumed": False,
                "binary_pruning_verified": False,
                "formal_distribution_claimed": False,
                "signed": False,
                "installer_status": "not_started",
                "launch_status": "not_verified",
                "artifact_root": "bundle",
                "artifact_files": artifact_files,
                "artifact_total_bytes": total_bytes,
                "artifact_tree_sha256": tree_hash,
                "package": {
                    "file_name": package_path.name,
                    "format": "D4PKG01",
                    "byte_length": package_result["package_size_bytes"],
                    "sha256": package_result["package_sha256"],
                    "signed": False,
                    "installed_product_verified": False,
                },
                "credential_artifact_scan": credential_scan,
            }
            validate_build_evidence(evidence, bundle, package_path, manifest_raw)
            (staging_root / RECEIPT_NAME).write_text(
                json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
            )
        if output.exists() or output.is_symlink():
            raise ValueError("build中にoutput directoryが作成されたため公開を停止した")
        os.rename(staging_root, output)
    return evidence


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Broker Receipt／ManifestからLinux Release bundle、D4PKG01、build provenanceを作る"
    )
    parser.add_argument("--receipt", type=Path, required=True, help="Brokerが返したExport Receipt")
    parser.add_argument("--manifest-file", type=Path, required=True, help="Receiptが束縛するManifest file")
    parser.add_argument("--output-dir", type=Path, required=True, help="Repository／OneDrive外の新規出力root")
    parser.add_argument(
        "--allow-verification-branch",
        action="store_true",
        help="cleanでremote HEAD一致のcodex/* branchを手動検証buildに限り許可する",
    )
    parser.add_argument(
        "--r2-e2e-test-build",
        action="store_true",
        help="固定test identity限定でLinux installed Agent Task acceptance buildを生成する",
    )
    args = parser.parse_args()
    try:
        evidence = build_linux_product(
            args.receipt,
            args.manifest_file,
            args.output_dir,
            allow_verification_branch=args.allow_verification_branch,
            r2_e2e_test_build=args.r2_e2e_test_build,
        )
    except Exception as error:
        print(f"Linux Export buildを停止した: {type(error).__name__}", file=sys.stderr)
        return 1
    print(
        json.dumps(
            {
                "output_dir": str(args.output_dir.resolve()),
                "bundle": str((args.output_dir / "bundle").resolve()),
                "package_sha256": evidence["package"]["sha256"],
                "artifact_tree_sha256": evidence["artifact_tree_sha256"],
                "source_commit": evidence["source_commit"],
                "signed": False,
                "installed_product_verified": False,
                "r2_e2e_test_build": args.r2_e2e_test_build,
            },
            ensure_ascii=False,
            separators=(",", ":"),
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
