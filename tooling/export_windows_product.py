"""Broker Export Manifestから未署名Windows portable bundleを組み立てる開発用tool。"""

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
import tarfile
import tempfile
import time
from pathlib import Path, PurePosixPath
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from tooling.build_module_pruned_windows import (
    _flutter_executable,
    _flutter_versions,
    _parse_json_bytes,
    _validate_catalog,
    dart_defines,
    resolve_module_plan,
)
from tooling.schema_check.check_schemas import validate_instance


RECEIPT_SCHEMA = ROOT / "specs" / "gui_shell_export_receipt.schema.json"
MANIFEST_SCHEMA = ROOT / "specs" / "gui_shell_export_manifest.schema.json"
BUILD_SCHEMA = ROOT / "specs" / "gui_shell_windows_export_build.schema.json"
CATALOG_PATH = ROOT / "specs" / "gui_shell_module_catalog.json"
MAX_INPUT_BYTES = 1_000_000
MAX_MANIFEST_BYTES = 65_536
RECEIPT_NAME = "d4_pocket_build_receipt.json"
MAX_CARGO_LINKER_PATH_CHARS = 240
CREDENTIAL_SCAN_CHUNK_BYTES = 1024 * 1024
CREDENTIAL_SCAN_OVERLAP_BYTES = 1024
CREDENTIAL_SCAN_STATUS = "passed_known_patterns"
CREDENTIAL_PATTERNS = (
    ("aws_access_key_id", re.compile(rb"(?:AKIA|ASIA)[A-Z0-9]{16}")),
    (
        "github_token",
        re.compile(
            rb"(?:gh[pour]_[A-Za-z0-9_]{20,255}|ghs_[A-Za-z0-9_.-]{12,512}|"
            rb"github_pat_[A-Za-z0-9_]{20,255})"
        ),
    ),
    ("google_api_key", re.compile(rb"AIza[0-9A-Za-z_-]{16,128}")),
    (
        "slack_token",
        re.compile(
            rb"(?:xox[bacprs]-[0-9A-Za-z-]{10,128}|xapp-[0-9A-Za-z-]{10,128}|"
            rb"xwfp-[0-9A-Za-z-]{10,128})"
        ),
    ),
    (
        "private_key_marker",
        re.compile(
            rb"-----BEGIN (?:(?:RSA |DSA |EC |OPENSSH |ENCRYPTED )?PRIVATE KEY|"
            rb"PGP PRIVATE KEY BLOCK)-----"
        ),
    ),
    (
        "credential_assignment",
        re.compile(
            rb"(?i)\b(?P<field>api[_-]?key|access[_-]?token|aws[_-]?secret[_-]?access[_-]?key|"
            rb"client[_-]?secret|password|session[_-]?secret)\b"
            rb"\s*[:=]\s*[\"']?(?P<value>[A-Za-z0-9/+=._-]{12,256})"
        ),
    ),
    (
        "bearer_authorization",
        re.compile(rb"(?i)authorization\s*:\s*bearer\s+[A-Za-z0-9._~+/-]{20,512}"),
    ),
)
SENSITIVE_ARTIFACT_NAMES = {
    ".env",
    ".env.local",
    "credentials",
    "credentials.json",
    "id_ed25519",
    "id_rsa",
    "secrets.json",
    "token.json",
}
BUILD_ENVIRONMENT_ALLOWLIST = {
    "APPDATA",
    "COMSPEC",
    "FLUTTER_ROOT",
    "HOMEDRIVE",
    "HOMEPATH",
    "INCLUDE",
    "LIB",
    "LIBPATH",
    "LOCALAPPDATA",
    "NUMBER_OF_PROCESSORS",
    "OS",
    "PATH",
    "PATHEXT",
    "PROCESSOR_ARCHITECTURE",
    "PROCESSOR_IDENTIFIER",
    "PROCESSOR_LEVEL",
    "PROCESSOR_REVISION",
    "PROGRAMDATA",
    "PROGRAMFILES",
    "PROGRAMFILES(X86)",
    "PUBLIC",
    "SYSTEMDRIVE",
    "SYSTEMROOT",
    "TEMP",
    "TMP",
    "UCRTVERSION",
    "UNIVERSALCRTSDKDIR",
    "USERPROFILE",
    "VCINSTALLDIR",
    "VCTOOLSINSTALLDIR",
    "VISUALSTUDIOVERSION",
    "VSCMD_ARG_HOST_ARCH",
    "VSCMD_ARG_TGT_ARCH",
    "VSCMD_VER",
    "WINDIR",
    "WINDOWSSDKDIR",
    "WINDOWSSDKVERSION",
    "RUSTUP_HOME",
}


def _sha256(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def _sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _is_reparse_point(metadata: os.stat_result) -> bool:
    return bool(getattr(metadata, "st_file_attributes", 0) & 0x400)


def _read_regular_file(path: Path, maximum_bytes: int, label: str) -> bytes:
    metadata = path.lstat()
    if stat.S_ISLNK(metadata.st_mode) or _is_reparse_point(metadata) or not stat.S_ISREG(metadata.st_mode):
        raise ValueError(f"{label}は通常fileでなければならない")
    if metadata.st_size > maximum_bytes:
        raise ValueError(f"{label}がsize上限を超えている")
    raw = path.read_bytes()
    if len(raw) > maximum_bytes:
        raise ValueError(f"{label}がsize上限を超えている")
    return raw


def _build_child_environment(
    overrides: dict[str, str], *, inherited: dict[str, str] | None = None
) -> dict[str, str]:
    source = os.environ if inherited is None else inherited
    result = {
        name: value
        for name, value in source.items()
        if name.upper() in BUILD_ENVIRONMENT_ALLOWLIST
    }
    for name, value in overrides.items():
        for existing in tuple(result):
            if existing.upper() == name.upper():
                del result[existing]
        result[name] = value
    return result


def _cargo_target_directory(temporary_root: Path, app_id: str, audit_store_id: str) -> Path:
    identity = f"{app_id}\0{audit_store_id}".encode("utf-8")
    identity_key = hashlib.sha256(identity).hexdigest()
    return temporary_root / "c" / identity_key


def _validate_cargo_target_path(target_dir: Path) -> None:
    linker_output = (
        target_dir
        / "release"
        / "build"
        / "windows_x86_64_msvc-0000000000000000"
        / "build_script_build-0000000000000000.exe"
    )
    path_characters = len(str(linker_output).encode("utf-16-le")) // 2
    if path_characters > MAX_CARGO_LINKER_PATH_CHARS:
        raise ValueError(
            "Cargo構築用パスがWindowsのリンク上限を超えた。より短い一時フォルダーを指定する。"
        )


def _is_one_drive_path(path: Path) -> bool:
    resolved = path.resolve(strict=False)
    if any(part.casefold().startswith("onedrive") for part in resolved.parts):
        return True
    for key, raw in os.environ.items():
        if key.lower().startswith("onedrive") and raw.strip():
            sync_root = Path(raw).expanduser().resolve()
            if resolved.is_relative_to(sync_root):
                return True
    return False


def _resolve_build_temp_root() -> Path:
    root = Path(tempfile.gettempdir()).resolve(strict=True)
    if root.is_relative_to(ROOT.resolve()) or _is_one_drive_path(root):
        raise ValueError("sourceとbuild targetの一時領域はRepository／OneDrive外に必要である")
    return root


def _schema(name: str) -> dict[str, Any]:
    value = json.loads((ROOT / "specs" / name).read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{name}がJSON Schema objectではない")
    return value


def validate_export_inputs(
    receipt_raw: bytes, manifest_raw: bytes
) -> tuple[dict[str, Any], dict[str, Any], Any, str, str]:
    if len(receipt_raw) > MAX_INPUT_BYTES or len(manifest_raw) > MAX_MANIFEST_BYTES:
        raise ValueError("Export ReceiptまたはManifest fileがsize上限を超えている")
    receipt = _parse_json_bytes(receipt_raw)
    manifest_file = _parse_json_bytes(manifest_raw)
    if not isinstance(receipt, dict) or not isinstance(manifest_file, dict):
        raise ValueError("Export ReceiptとManifest fileはJSON objectでなければならない")
    receipt_errors = validate_instance(receipt, _schema(RECEIPT_SCHEMA.name))
    manifest_errors = validate_instance(manifest_file, _schema(MANIFEST_SCHEMA.name))
    if receipt_errors:
        raise ValueError("Export ReceiptがSchemaに適合しない")
    if manifest_errors:
        raise ValueError("Export Manifest fileがSchemaに適合しない")

    stored = receipt["manifest_file"]
    exported = receipt["export_manifest"]
    if manifest_file["export_id"] != receipt["export_id"]:
        raise ValueError("ReceiptとManifest fileのExport IDが一致しない")
    if manifest_file["manifest"] != exported:
        raise ValueError("ReceiptとManifest fileの内容が一致しない")
    if manifest_file["manifest"]["app_identity"]["app_id"] != exported["app_identity"]["app_id"]:
        raise ValueError("ReceiptとManifest fileのApp IDが一致しない")
    if manifest_file["manifest"]["audit_store"]["store_id"] != exported["audit_store"]["store_id"]:
        raise ValueError("ReceiptとManifest fileのAudit store IDが一致しない")
    if stored["file_name"] != f"{exported['app_identity']['app_id']}.json":
        raise ValueError("ReceiptのManifest file名がApp IDと一致しない")
    if stored["byte_length"] != len(manifest_raw):
        raise ValueError("ReceiptとManifest fileのbyte長が一致しない")
    if stored["sha256"] != f"sha256:{_sha256(manifest_raw)}":
        raise ValueError("ReceiptとManifest fileのSHA-256が一致しない")

    catalog_raw = _read_regular_file(CATALOG_PATH, MAX_INPUT_BYTES, "Module一覧")
    catalog = _validate_catalog(_parse_json_bytes(catalog_raw))
    plan = resolve_module_plan(receipt, catalog)
    return receipt, manifest_file, plan, _sha256(receipt_raw), _sha256(manifest_raw)


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


def _require_clean_source() -> str:
    if _git_value("branch", "--show-current") != "main":
        raise ValueError("Export buildはmain branchのclean commitから実行する")
    if _git_value("status", "--porcelain", "--untracked-files=all"):
        raise ValueError("Export buildはcleanなcommit済みsourceから実行する")
    commit = _git_value("rev-parse", "HEAD")
    remote_refs = _git_value("ls-remote", "origin", "refs/heads/main").splitlines()
    remote_parts = remote_refs[0].split() if len(remote_refs) == 1 else []
    if len(remote_parts) != 2 or remote_parts[0] != commit or remote_parts[1] != "refs/heads/main":
        raise ValueError("Export buildのmain HEADがGitHub origin/mainと一致しない")
    return commit


def _write_source_archive(commit: str, archive_path: Path) -> str:
    with archive_path.open("xb") as archive:
        subprocess.run(
            ["git", "-C", str(ROOT), "archive", "--format=tar", commit],
            check=True,
            stdout=archive,
        )
    digest = hashlib.sha256()
    with archive_path.open("rb") as archive:
        for chunk in iter(lambda: archive.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _safe_extract_source(archive_path: Path, destination: Path) -> None:
    root = destination.resolve(strict=True)
    seen: set[str] = set()
    member_kinds: dict[str, str] = {}
    with tarfile.open(archive_path, mode="r:") as archive:
        for member in archive.getmembers():
            relative = PurePosixPath(member.name)
            if (
                not member.name
                or not relative.parts
                or relative.is_absolute()
                or ".." in relative.parts
                or "\\" in member.name
                or any(":" in part for part in relative.parts)
            ):
                raise ValueError("Git source archiveにroot外または不正なpathがある")
            normalized = relative.as_posix().casefold()
            if normalized in seen:
                raise ValueError("Git source archiveにWindows上で衝突するpathがある")
            for parent in relative.parents:
                if parent.parts and member_kinds.get(parent.as_posix().casefold()) == "file":
                    raise ValueError("Git source archiveでfileがdirectory pathと衝突する")
            if member.isfile() and any(
                previous.startswith(normalized + "/") for previous in seen
            ):
                raise ValueError("Git source archiveでfileが既存directory pathと衝突する")
            seen.add(normalized)
            target = destination.joinpath(*relative.parts)
            if not target.resolve(strict=False).is_relative_to(root):
                raise ValueError("Git source archive pathが一時source root外を指している")
            if member.isdir():
                member_kinds[normalized] = "directory"
                target.mkdir(parents=True, exist_ok=True)
                continue
            if not member.isfile():
                raise ValueError("Git source archiveのsymlink等の非通常fileを拒否した")
            member_kinds[normalized] = "file"
            target.parent.mkdir(parents=True, exist_ok=True)
            source = archive.extractfile(member)
            if source is None:
                raise ValueError("Git source archive fileを読み取れない")
            with source, target.open("xb") as output:
                shutil.copyfileobj(source, output)


def _tool_version(command: list[str]) -> str:
    result = subprocess.run(
        command,
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
    )
    value = result.stdout.splitlines()[0].strip() if result.stdout.splitlines() else ""
    if not value:
        raise ValueError("build toolがversionを返さなかった")
    return value


def _resolve_output_directory(value: Path) -> Path:
    candidate = value.expanduser()
    if not candidate.is_absolute():
        candidate = ROOT / candidate
    if candidate.name in ("", ".", ".."):
        raise ValueError("output directory名が不正である")
    parent = candidate.parent.resolve(strict=True)
    resolved = parent / candidate.name
    if resolved.exists() or resolved.is_symlink():
        raise ValueError("output directoryが既に存在するため上書きを拒否した")
    root = ROOT.resolve()
    if resolved.is_relative_to(root):
        raise ValueError("Export build artifactをRepository内へ出力できない")
    if _is_one_drive_path(resolved):
        raise ValueError("Export build artifactはOneDrive同期root外へ出力する")
    return resolved


def _artifact_inventory(
    root: Path, *, excluded_paths: frozenset[str] = frozenset()
) -> tuple[list[dict[str, Any]], int, str]:
    root_metadata = root.lstat()
    if (
        not stat.S_ISDIR(root_metadata.st_mode)
        or stat.S_ISLNK(root_metadata.st_mode)
        or _is_reparse_point(root_metadata)
    ):
        raise ValueError("artifact rootが通常directoryではない")
    resolved_root = root.resolve(strict=True)
    records: list[dict[str, Any]] = []
    total_bytes = 0
    tree_hash = hashlib.sha256()
    for path in sorted(root.rglob("*"), key=lambda item: item.relative_to(root).as_posix().casefold()):
        metadata = path.lstat()
        if stat.S_ISLNK(metadata.st_mode) or _is_reparse_point(metadata):
            raise ValueError(f"portable bundleにsymlink／reparse pointがある: {path.relative_to(root)}")
        if stat.S_ISDIR(metadata.st_mode):
            continue
        if not stat.S_ISREG(metadata.st_mode):
            raise ValueError(f"portable bundleに通常file以外がある: {path.relative_to(root)}")
        relative = path.relative_to(root).as_posix()
        if relative in excluded_paths:
            continue
        if ".." in PurePosixPath(relative).parts or not path.resolve(strict=True).is_relative_to(resolved_root):
            raise ValueError("portable bundle fileがartifact root外を指している")
        digest = _sha256_file(path)
        size = metadata.st_size
        records.append({"path": relative, "size_bytes": size, "sha256": digest})
        total_bytes += size
        tree_hash.update(f"{relative}\0{size}\0{digest}\n".encode("utf-8"))
    if not records:
        raise ValueError("portable bundleにfileがない")
    return records, total_bytes, tree_hash.hexdigest()


def scan_credential_artifacts(
    root: Path, *, expected_tree_sha256: str | None = None
) -> dict[str, Any]:
    """既知の資格値形式を全bundle fileから探す。資格不存在の一般証明ではない。"""
    records, total_bytes, tree_hash = _artifact_inventory(
        root, excluded_paths=frozenset({RECEIPT_NAME})
    )
    if expected_tree_sha256 is not None and tree_hash != expected_tree_sha256:
        raise ValueError("Credential検査対象のartifact hashがinventoryと一致しない")
    return _scan_credential_inventory(root, records, total_bytes, tree_hash)


def _scan_credential_inventory(
    root: Path,
    records: list[dict[str, Any]],
    total_bytes: int,
    tree_hash: str,
) -> dict[str, Any]:
    files_scanned = 0
    bytes_scanned = 0
    for record in records:
        relative = record["path"]
        path = root / PurePosixPath(relative)
        artifact_name = PurePosixPath(relative).name.casefold()
        if artifact_name in SENSITIVE_ARTIFACT_NAMES or Path(artifact_name).suffix in {
            ".jks",
            ".key",
            ".keystore",
            ".p12",
            ".pfx",
            ".pem",
        }:
            raise ValueError(f"Credentialらしい固定file名をbundleで検出した: {relative}")

        carry = b""
        with path.open("rb") as source:
            while True:
                chunk = source.read(CREDENTIAL_SCAN_CHUNK_BYTES)
                if not chunk:
                    break
                chunk_offset = bytes_scanned
                bytes_scanned += len(chunk)
                candidate = carry + chunk
                for pattern_id, pattern in CREDENTIAL_PATTERNS:
                    match = pattern.search(candidate)
                    if match is not None:
                        diagnostic = ""
                        if pattern_id == "credential_assignment":
                            field = match.group("field").decode("ascii").lower()
                            value_length = len(match.group("value"))
                            byte_offset = chunk_offset - len(carry) + match.start()
                            diagnostic = (
                                f"; field={field}; byte_offset={byte_offset}; "
                                f"value_length={value_length}"
                            )
                        raise ValueError(
                            f"既知Credential patternをbundleで検出した: {pattern_id} ({relative}){diagnostic}"
                        )
                carry = candidate[-CREDENTIAL_SCAN_OVERLAP_BYTES:]
        files_scanned += 1

    after_records, after_bytes, after_tree_hash = _artifact_inventory(
        root, excluded_paths=frozenset({RECEIPT_NAME})
    )
    if (
        after_records != records
        or after_bytes != total_bytes
        or after_tree_hash != tree_hash
    ):
        raise ValueError("Credential検査中にbundle artifactが変化した")
    return {
        "version": 1,
        "status": CREDENTIAL_SCAN_STATUS,
        "scope": "runtime_artifact_files",
        "coverage": "known_patterns_only",
        "files_scanned": files_scanned,
        "bytes_scanned": bytes_scanned,
        "findings": 0,
        "artifact_tree_sha256": tree_hash,
    }


def validate_build_evidence(evidence: dict[str, Any], artifact_root: Path) -> None:
    errors = validate_instance(evidence, _schema(BUILD_SCHEMA.name))
    if errors:
        raise ValueError("Export build evidenceがSchemaに適合しない: " + "; ".join(errors))
    if evidence["rust_compile_time_identity"] != {
        "source": "schema_validated_manifest",
        "app_id": evidence["app_id"],
        "audit_store_id": evidence["audit_store_id"],
    }:
        raise ValueError("Rust compile-time identityがExport Manifestと一致しない")
    artifact_files, total_bytes, tree_hash = _artifact_inventory(
        artifact_root, excluded_paths=frozenset({RECEIPT_NAME})
    )
    if (
        artifact_files != evidence["artifact_files"]
        or total_bytes != evidence["artifact_total_bytes"]
        or tree_hash != evidence["artifact_tree_sha256"]
    ):
        raise ValueError("Export build evidenceとportable bundle file inventoryが一致しない")
    credential_scan = _scan_credential_inventory(
        artifact_root, artifact_files, total_bytes, tree_hash
    )
    if (
        credential_scan != evidence["credential_artifact_scan"]
        or evidence["authority_boundary"]["credential_artifact_scan_status"]
        != credential_scan["status"]
    ):
        raise ValueError("Credential検査結果がportable bundleに一致しない")
    required_paths = {
        "app/gui_shell_desktop.exe",
        "app/flutter_windows.dll",
        "app/data/app.so",
        "app/data/icudtl.dat",
        "broker/gui_shell_rust_helper.exe",
        "gui_shell_desktop_launcher.exe",
        "product_manifest.json",
    }
    if not required_paths.issubset({item["path"] for item in artifact_files}):
        raise ValueError("portable bundleに必須Flutter／Broker／Manifest fileがない")
    manifest_raw = _read_regular_file(
        artifact_root / "product_manifest.json", MAX_MANIFEST_BYTES, "portable Manifest"
    )
    assets_dir = artifact_root / "app" / "data" / "flutter_assets"
    assets_metadata = assets_dir.lstat()
    if (
        not stat.S_ISDIR(assets_metadata.st_mode)
        or stat.S_ISLNK(assets_metadata.st_mode)
        or _is_reparse_point(assets_metadata)
    ):
        raise ValueError("portable bundleのFlutter assets directoryが不正")
    if _sha256(manifest_raw) != evidence["source_manifest_sha256"]:
        raise ValueError("portable Manifestのbyte列が入力Manifestから変化した")
    manifest_file = _parse_json_bytes(manifest_raw)
    manifest_errors = validate_instance(manifest_file, _schema(MANIFEST_SCHEMA.name))
    if manifest_errors:
        raise ValueError("portable ManifestがSchemaに適合しない")
    if (
        not isinstance(manifest_file, dict)
        or manifest_file.get("manifest", {}).get("app_identity", {}).get("app_id")
        != evidence["app_id"]
        or manifest_file.get("manifest", {}).get("audit_store", {}).get("store_id")
        != evidence["audit_store_id"]
    ):
        raise ValueError("portable Manifestの製品識別子がbuild evidenceと一致しない")


def _copy_bundle(
    flutter_release: Path,
    helper_exe: Path,
    launcher_exe: Path,
    manifest_raw: bytes,
    output: Path,
) -> None:
    _artifact_inventory(flutter_release)
    output.mkdir(parents=False, exist_ok=False)
    shutil.copytree(flutter_release, output / "app", symlinks=False)
    broker = output / "broker"
    broker.mkdir()
    shutil.copy2(helper_exe, broker / "gui_shell_rust_helper.exe")
    shutil.copy2(launcher_exe, output / "gui_shell_desktop_launcher.exe")
    (output / "product_manifest.json").write_bytes(manifest_raw)
    required = (
        output / "app" / "gui_shell_desktop.exe",
        output / "app" / "flutter_windows.dll",
        output / "app" / "data" / "app.so",
        output / "app" / "data" / "icudtl.dat",
        output / "broker" / "gui_shell_rust_helper.exe",
        output / "gui_shell_desktop_launcher.exe",
        output / "product_manifest.json",
    )
    if any(not path.is_file() for path in required) or not (
        output / "app" / "data" / "flutter_assets"
    ).is_dir():
        raise ValueError("portable bundleに起動器、Broker、Flutter ReleaseまたはManifestが揃わない")


def build_portable_bundle(
    receipt_path: Path, manifest_path: Path, output_dir: Path
) -> dict[str, Any]:
    if sys.platform != "win32":
        raise ValueError("このExport buildはWindows host専用である")
    commit = _require_clean_source()
    output = _resolve_output_directory(output_dir)
    receipt_raw = _read_regular_file(receipt_path, MAX_INPUT_BYTES, "Export Receipt")
    manifest_raw = _read_regular_file(manifest_path, MAX_MANIFEST_BYTES, "Export Manifest file")
    receipt, manifest_file, plan, receipt_hash, manifest_hash = validate_export_inputs(
        receipt_raw, manifest_raw
    )
    manifest_info = receipt["manifest_file"]
    if manifest_path.name != manifest_info["file_name"]:
        raise ValueError("入力Manifest file名がBroker Receiptと一致しない")

    app_id = manifest_file["manifest"]["app_identity"]["app_id"]
    audit_store_id = manifest_file["manifest"]["audit_store"]["store_id"]
    define_map = dart_defines(plan)
    flutter = _flutter_executable()
    cargo = shutil.which("cargo")
    rustc = shutil.which("rustc")
    if cargo is None or rustc is None:
        raise ValueError("Export buildに必要なCargo／Rust compilerをPATHから解決できない")
    flutter_versions = _flutter_versions(flutter)
    if any(not value or value == "unknown" for value in flutter_versions.values()):
        raise ValueError("Flutter toolchainのversion情報が不足している")
    cargo_version = _tool_version([cargo, "--version"])
    rustc_version = _tool_version([rustc, "--version"])

    build_started = time.perf_counter()
    with tempfile.TemporaryDirectory(
        prefix="d4p-", dir=output.parent
    ) as staging_parent:
        staging_root = Path(staging_parent) / "bundle"
        with tempfile.TemporaryDirectory(
            prefix="d4b-", dir=_resolve_build_temp_root()
        ) as temporary:
            temporary_root = Path(temporary)
            source_root = temporary_root / "source"
            source_root.mkdir()
            archive_path = temporary_root / "source.tar"
            archive_hash = _write_source_archive(commit, archive_path)
            _safe_extract_source(archive_path, source_root)

            flutter_environment = _build_child_environment(
                {"PUB_CACHE": str(temporary_root / "p")}
            )
            pub_arguments = ["--suppress-analytics", "pub", "get", "--enforce-lockfile"]
            subprocess.run(
                [flutter, *pub_arguments],
                cwd=source_root / "apps" / "desktop_flutter",
                env=flutter_environment,
                check=True,
            )

            flutter_arguments = [
                "--suppress-analytics",
                "build",
                "windows",
                "--release",
                "--no-pub",
                *[
                    f"--dart-define={name}={str(enabled).lower()}"
                    for name, enabled in define_map.items()
                ],
            ]
            flutter_project = source_root / "apps" / "desktop_flutter"
            subprocess.run(
                [flutter, *flutter_arguments],
                cwd=flutter_project,
                env=flutter_environment,
                check=True,
            )
            flutter_release = (
                flutter_project / "build" / "windows" / "x64" / "runner" / "Release"
            )

            target_dir = _cargo_target_directory(temporary_root, app_id, audit_store_id)
            _validate_cargo_target_path(target_dir)
            rust_arguments = [
                "build",
                "--release",
                "--locked",
                "--target-dir",
                str(target_dir),
                "--manifest-path",
                "native/rust_helper/Cargo.toml",
                "--bin",
                "gui_shell_rust_helper",
                "--bin",
                "gui_shell_desktop_launcher",
            ]
            rust_environment = _build_child_environment(
                {
                    "CARGO_HOME": str(temporary_root / "g"),
                    "GUI_SHELL_PRODUCT_APP_ID": app_id,
                    "GUI_SHELL_PRODUCT_AUDIT_STORE_ID": audit_store_id,
                }
            )
            subprocess.run([cargo, *rust_arguments], cwd=source_root, env=rust_environment, check=True)
            helper_exe = target_dir / "release" / "gui_shell_rust_helper.exe"
            launcher_exe = target_dir / "release" / "gui_shell_desktop_launcher.exe"
            if not helper_exe.is_file() or not launcher_exe.is_file():
                raise ValueError("Rust Release buildにBrokerまたはDesktop launcherがない")

            _copy_bundle(
                flutter_release, helper_exe, launcher_exe, manifest_raw, staging_root
            )
        artifact_files, total_bytes, tree_hash = _artifact_inventory(staging_root)
        credential_scan = _scan_credential_inventory(
            staging_root, artifact_files, total_bytes, tree_hash
        )

        elapsed_ms = round((time.perf_counter() - build_started) * 1000)
        catalog = _validate_catalog(
            _parse_json_bytes(_read_regular_file(CATALOG_PATH, MAX_INPUT_BYTES, "Module一覧"))
        )
        evidence = {
            "version": 1,
            "evidence_type": "d4_pocket_windows_export_build",
            "evidence_source": "INTERNAL_STATE",
            "product": "D4 Pocket",
            "target_platform": "windows",
            "build_mode": "release",
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
                "catalog_version": catalog["version"],
                "selection_mode": plan.selection_mode,
                "requested_optional_module_ids": list(plan.requested_optional_ids),
                "unprunable_core_ids": catalog["unprunable_core_ids"],
                "included_module_ids": list(plan.included_module_ids),
                "excluded_optional_module_ids": list(plan.excluded_optional_ids),
            },
            "toolchain": {
                **flutter_versions,
                "cargo": cargo_version,
                "rustc": rustc_version,
            },
            "pub_get_arguments": pub_arguments,
            "flutter_build_arguments": flutter_arguments,
            "rust_build_arguments": [
                "build",
                "--release",
                "--locked",
                "--target-dir",
                "<isolated-target-dir>",
                "--manifest-path",
                "native/rust_helper/Cargo.toml",
                "--bin",
                "gui_shell_rust_helper",
                "--bin",
                "gui_shell_desktop_launcher",
            ],
            "rust_compile_time_identity": {
                "source": "schema_validated_manifest",
                "app_id": app_id,
                "audit_store_id": audit_store_id,
            },
            "build_duration_ms": elapsed_ms,
            "runtime_storage_root_template": "%LOCALAPPDATA%\\D4Pocket\\apps\\<App ID>\\stores\\<Audit store ID>",
            "portable_bundle_assembled": True,
            "manifest_copied_byte_for_byte": True,
            "build_receipt_excluded_from_artifact_inventory": True,
            "standalone_app_verified": False,
            "runtime_manifest_consumed": False,
            "binary_pruning_verified": False,
            "formal_distribution_claimed": False,
            "signed": False,
            "installer_status": "not_started",
            "launch_status": "not_verified",
            "artifact_root": ".",
            "artifact_files": artifact_files,
            "artifact_total_bytes": total_bytes,
            "artifact_tree_sha256": tree_hash,
            "credential_artifact_scan": credential_scan,
        }
        validate_build_evidence(evidence, staging_root)
        (staging_root / RECEIPT_NAME).write_text(
            json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
        )
        if output.exists() or output.is_symlink():
            raise ValueError("build中にoutput directoryが作成されたため公開を停止した")
        os.rename(staging_root, output)
    return evidence


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Broker Export Receipt／Manifestから、Windows用未署名portable bundleを作る開発専用tool。"
    )
    parser.add_argument("--receipt", type=Path, required=True, help="Brokerが返したExport Receipt JSON")
    parser.add_argument("--manifest-file", type=Path, required=True, help="Receiptが指すManifest file")
    parser.add_argument(
        "--output-dir",
        type=Path,
        required=True,
        help="未作成かつRepository／OneDrive同期範囲外の出力directory",
    )
    args = parser.parse_args()
    try:
        evidence = build_portable_bundle(args.receipt, args.manifest_file, args.output_dir)
    except (OSError, ValueError, subprocess.CalledProcessError, json.JSONDecodeError, tarfile.TarError) as exc:
        print(f"Windows Export buildを停止した: {exc}", file=sys.stderr)
        return 1
    print(json.dumps({
        "portable_bundle_assembled": evidence["portable_bundle_assembled"],
        "standalone_app_verified": evidence["standalone_app_verified"],
        "formal_distribution_claimed": evidence["formal_distribution_claimed"],
        "artifact_total_bytes": evidence["artifact_total_bytes"],
        "artifact_tree_sha256": evidence["artifact_tree_sha256"],
    }, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
