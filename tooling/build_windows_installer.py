"""Export Receiptに固定されたWindows D4 Pocket bootstrap Setupを生成する。"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import stat
import struct
import subprocess
import sys
import tempfile
from pathlib import Path, PurePosixPath
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from tooling.export_windows_product import (  # noqa: E402
    MANIFEST_SCHEMA,
    MAX_INPUT_BYTES,
    RECEIPT_NAME,
    _is_one_drive_path,
    _is_reparse_point,
    _read_regular_file,
    _require_clean_source,
    _schema,
    product_version_from_source,
    validate_build_evidence,
)
from tooling.schema_check.check_schemas import validate_instance  # noqa: E402

APP_ID_PATTERN = re.compile(r"^d4-pocket-app-[0-9a-f]{32}$")
AUDIT_STORE_ID_PATTERN = re.compile(r"^audit-store-[0-9a-f]{32}$")
EMBEDDED_APP_ID_PATTERN = re.compile(rb"d4-pocket-app-[0-9a-f]{32}")
EMBEDDED_AUDIT_STORE_ID_PATTERN = re.compile(rb"audit-store-[0-9a-f]{32}")
MAX_LAUNCHER_BYTES = 128 * 1024 * 1024
MAX_PRODUCT_MANIFEST_BYTES = 64 * 1024
WINDOWS_PAYLOAD_COMPONENTS = (
    ("flutter_application", "app/gui_shell_desktop.exe"),
    ("rust_broker", "broker/gui_shell_rust_helper.exe"),
    ("rust_launcher", "gui_shell_desktop_launcher.exe"),
)
PE_ARCHITECTURES = {
    0x014C: "x86",
    0x8664: "x64",
    0xAA64: "arm64",
}
ISS_SCRIPT = ROOT / "installer" / "windows" / "d4_pocket_bootstrap_setup.iss"


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("JSONに重複keyがある")
        result[key] = value
    return result


def _read_json(path: Path, max_bytes: int, label: str) -> tuple[dict[str, Any], bytes]:
    metadata = path.lstat()
    if (
        not stat.S_ISREG(metadata.st_mode)
        or path.is_symlink()
        or _is_reparse_point(metadata)
        or metadata.st_size < 1
        or metadata.st_size > max_bytes
    ):
        raise ValueError(f"{label}はsize上限内の通常fileでなければならない")
    raw = _read_regular_file(path, max_bytes, label)
    try:
        value = json.loads(raw.decode("utf-8"), object_pairs_hook=_unique_object)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ValueError(f"{label}は重複keyのないUTF-8 JSONでなければならない") from error
    if not isinstance(value, dict):
        raise ValueError(f"{label}はJSON objectでなければならない")
    return value, raw


def _regular_file(path: Path, label: str, *, max_bytes: int | None = None) -> bytes:
    metadata = path.lstat()
    if (
        not stat.S_ISREG(metadata.st_mode)
        or path.is_symlink()
        or _is_reparse_point(metadata)
        or metadata.st_size < 1
        or (max_bytes is not None and metadata.st_size > max_bytes)
    ):
        raise ValueError(f"{label}が通常fileではない、または許容size外である")
    with path.open("rb") as source:
        return source.read()


def _hash_regular_file(path: Path, label: str) -> tuple[str, int]:
    metadata = path.lstat()
    if (
        not stat.S_ISREG(metadata.st_mode)
        or path.is_symlink()
        or _is_reparse_point(metadata)
        or metadata.st_size < 1
    ):
        raise ValueError(f"{label}が通常fileではない")
    digest = hashlib.sha256()
    byte_count = 0
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
            byte_count += len(chunk)
    if byte_count != metadata.st_size:
        raise ValueError(f"{label}の読取中にsizeが変化した")
    return digest.hexdigest(), byte_count


def _pe_architecture(path: Path) -> str:
    metadata = path.lstat()
    if (
        not stat.S_ISREG(metadata.st_mode)
        or path.is_symlink()
        or _is_reparse_point(metadata)
        or metadata.st_size < 64
    ):
        raise ValueError("Windows executableが通常PE fileではない")
    with path.open("rb") as source:
        dos_header = source.read(64)
        if len(dos_header) != 64 or dos_header[:2] != b"MZ":
            raise ValueError("Windows executableのDOS headerが不正である")
        pe_offset = struct.unpack_from("<I", dos_header, 0x3C)[0]
        if pe_offset < 64 or pe_offset > metadata.st_size - 6:
            raise ValueError("Windows executableのPE offsetがfile範囲外である")
        source.seek(pe_offset)
        pe_header = source.read(6)
    if len(pe_header) != 6 or pe_header[:4] != b"PE\0\0":
        raise ValueError("Windows executableのPE signatureが不正である")
    machine = struct.unpack_from("<H", pe_header, 4)[0]
    try:
        return PE_ARCHITECTURES[machine]
    except KeyError as error:
        raise ValueError("Windows executableのmachine architectureが未対応である") from error


def _windows_powershell_path() -> Path:
    powershell = (
        Path(os.environ.get("SystemRoot", r"C:\Windows"))
        / "System32"
        / "WindowsPowerShell"
        / "v1.0"
        / "powershell.exe"
    )
    if not powershell.is_file():
        raise ValueError("Windows標準PowerShell 5.1が見つからない")
    return powershell


def _authenticode_info(path: Path) -> dict[str, str | None]:
    powershell = _windows_powershell_path()
    script = (
        "$ErrorActionPreference = 'Stop'; "
        "Import-Module Microsoft.PowerShell.Security -ErrorAction Stop; "
        "$signature = Get-AuthenticodeSignature -LiteralPath $env:D4P_AUTHENTICODE_TARGET; "
        "$thumbprint = $null; "
        "if ($null -ne $signature.SignerCertificate) { "
        "$thumbprint = $signature.SignerCertificate.Thumbprint }; "
        "$result = [ordered]@{ status = $signature.Status.ToString(); "
        "signer_thumbprint = $thumbprint }; "
        "[Console]::Out.Write(($result | ConvertTo-Json -Compress))"
    )
    environment = os.environ.copy()
    environment["D4P_AUTHENTICODE_TARGET"] = str(path)
    system_modules = (
        Path(os.environ.get("SystemRoot", r"C:\Windows"))
        / "System32"
        / "WindowsPowerShell"
        / "v1.0"
        / "Modules"
    )
    security_module = (
        system_modules
        / "Microsoft.PowerShell.Security"
        / "Microsoft.PowerShell.Security.psd1"
    )
    if not security_module.is_file():
        raise ValueError("Windows標準Authenticode moduleが見つからない")
    environment["PSModulePath"] = str(system_modules)
    try:
        completed = subprocess.run(
            [str(powershell), "-NoProfile", "-NonInteractive", "-Command", script],
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding="utf-8",
            errors="replace",
            env=environment,
            timeout=30,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        raise ValueError("Authenticode確認を完了できない") from error
    if completed.returncode != 0:
        detail = " ".join(completed.stderr.split())[-1200:]
        raise ValueError(f"Authenticode確認が失敗した (exit={completed.returncode}): {detail}")
    try:
        result = json.loads(completed.stdout)
    except json.JSONDecodeError as error:
        raise ValueError("Authenticode確認結果を読み取れない") from error
    if not isinstance(result, dict) or set(result) != {"status", "signer_thumbprint"}:
        raise ValueError("Authenticode確認結果の形式が不正である")
    status = result.get("status")
    thumbprint = result.get("signer_thumbprint")
    if not isinstance(status, str) or status not in {"Valid", "NotSigned"}:
        raise ValueError("Authenticode署名が無効、または信頼状態を確認できない")
    if status == "Valid":
        if not isinstance(thumbprint, str) or not re.fullmatch(r"[0-9A-Fa-f]{40,64}", thumbprint):
            raise ValueError("有効署名のcertificate thumbprintを確認できない")
        thumbprint = thumbprint.upper()
    elif thumbprint is not None:
        raise ValueError("未署名fileの署名者情報が整合しない")
    return {"status": status, "signer_thumbprint": thumbprint}


def _inspect_payload_components(
    bundle_dir: Path, artifact_files: list[dict[str, Any]]
) -> list[dict[str, Any]]:
    records: dict[str, dict[str, Any]] = {}
    for record in artifact_files:
        if not isinstance(record, dict):
            raise ValueError("Export Receipt artifact recordが不正である")
        relative = record.get("path")
        if not isinstance(relative, str) or relative in records:
            raise ValueError("Export Receipt artifact pathが不正または重複している")
        records[relative] = record

    inspected: list[dict[str, Any]] = []
    for role, relative in WINDOWS_PAYLOAD_COMPONENTS:
        record = records.get(relative)
        if record is None:
            raise ValueError(f"Export Receiptに必須Windows componentがない: {relative}")
        expected_size = record.get("size_bytes")
        expected_hash = record.get("sha256")
        if (
            type(expected_size) is not int
            or expected_size < 1
            or not isinstance(expected_hash, str)
            or not re.fullmatch(r"[0-9a-f]{64}", expected_hash)
        ):
            raise ValueError(f"Export Receipt component recordが不正である: {relative}")
        source = bundle_dir.joinpath(*PurePosixPath(relative).parts)
        digest, size = _hash_regular_file(source, f"Windows component {relative}")
        if size != expected_size or digest != expected_hash:
            raise ValueError(f"Windows componentがExport Receiptのhash／sizeと一致しない: {relative}")
        architecture = _pe_architecture(source)
        if architecture != "x64":
            raise ValueError(f"Windows product payload architectureはx64必須: {relative}")
        signature = _authenticode_info(source)
        verified_hash, verified_size = _hash_regular_file(
            source, f"Windows component {relative} after Authenticode check"
        )
        if verified_hash != digest or verified_size != size:
            raise ValueError(f"Authenticode確認中にWindows componentが変化した: {relative}")
        inspected.append(
            {
                "role": role,
                "path": relative,
                "size_bytes": size,
                "sha256": digest,
                "architecture": architecture,
                "authenticode_status": signature["status"],
                "signer_thumbprint": signature["signer_thumbprint"],
            }
        )
    return inspected


def _outside_source_roots(path: Path, label: str) -> Path:
    metadata = path.lstat()
    if not stat.S_ISDIR(metadata.st_mode) or path.is_symlink() or _is_reparse_point(metadata):
        raise ValueError(f"{label}は通常directoryでなければならない")
    resolved = path.resolve(strict=True)
    if (
        resolved == ROOT
        or resolved.is_relative_to(ROOT)
        or _is_one_drive_path(resolved)
        or any(
            (parent / ".git").exists()
            for parent in (resolved, *resolved.parents)
        )
    ):
        raise ValueError(f"{label}はRepository／OneDriveの外でなければならない")
    return resolved


def _snapshot_export_bundle(
    source_root: Path,
    snapshot_root: Path,
    artifact_files: list[dict[str, Any]],
    receipt_raw: bytes,
) -> None:
    snapshot_root.mkdir(parents=False, exist_ok=False)
    for record in artifact_files:
        relative = PurePosixPath(record["path"])
        if relative.is_absolute() or ".." in relative.parts or not relative.parts:
            raise ValueError("Export Receiptに安全でないartifact pathがある")
        source = source_root.joinpath(*relative.parts)
        metadata = source.lstat()
        if (
            not stat.S_ISREG(metadata.st_mode)
            or source.is_symlink()
            or _is_reparse_point(metadata)
            or metadata.st_size != record["size_bytes"]
        ):
            raise ValueError(
                f"Export artifactが通常fileまたはReceipt sizeと一致しない: {relative}"
            )
        destination = snapshot_root.joinpath(*relative.parts)
        destination.parent.mkdir(parents=True, exist_ok=True)
        digest = hashlib.sha256()
        byte_count = 0
        with source.open("rb") as input_file, destination.open("xb") as output_file:
            for chunk in iter(lambda: input_file.read(1024 * 1024), b""):
                output_file.write(chunk)
                digest.update(chunk)
                byte_count += len(chunk)
        if byte_count != record["size_bytes"] or digest.hexdigest() != record["sha256"]:
            raise ValueError(f"Export artifactがReceipt hash／sizeから変化した: {relative}")

    receipt_target = snapshot_root / RECEIPT_NAME
    with receipt_target.open("xb") as output_file:
        output_file.write(receipt_raw)
        output_file.flush()


def _embedded_identity(launcher_bytes: bytes) -> tuple[set[str], set[str]]:
    app_ids = {
        value.decode("ascii") for value in EMBEDDED_APP_ID_PATTERN.findall(launcher_bytes)
    }
    audit_ids = {
        value.decode("ascii")
        for value in EMBEDDED_AUDIT_STORE_ID_PATTERN.findall(launcher_bytes)
    }
    return app_ids, audit_ids


def validate_export_bundle(bundle_dir: Path) -> tuple[str, str, str, str]:
    """Broker Export Receiptと現行mainに束縛されたbundleだけを許可する。"""
    bundle = _outside_source_roots(bundle_dir, "Export bundle")
    if not bundle.is_dir():
        raise ValueError("Export bundleは通常directoryでなければならない")

    receipt, _ = _read_json(bundle / RECEIPT_NAME, MAX_INPUT_BYTES, "Windows Export Receipt")
    validate_build_evidence(receipt, bundle)
    current_commit = _require_clean_source()
    if receipt.get("source_commit") != current_commit:
        raise ValueError("Export Receiptのsource commitが現在のGitHub mainと一致しない")

    manifest_path = bundle / "product_manifest.json"
    manifest, _ = _read_json(
        manifest_path, MAX_PRODUCT_MANIFEST_BYTES, "D4 Pocket Product Manifest"
    )
    if validate_instance(manifest, _schema(MANIFEST_SCHEMA.name)):
        raise ValueError("Product Manifestが現行Schemaに適合しない")
    if manifest.get("product") != "D4 Pocket" or manifest.get("version") != 1:
        raise ValueError("Product Manifestの製品識別が一致しない")
    export_manifest = manifest.get("manifest")
    app_identity = (
        export_manifest.get("app_identity") if isinstance(export_manifest, dict) else None
    )
    audit_store = export_manifest.get("audit_store") if isinstance(export_manifest, dict) else None
    app_id = app_identity.get("app_id") if isinstance(app_identity, dict) else None
    audit_store_id = audit_store.get("store_id") if isinstance(audit_store, dict) else None
    if not isinstance(app_id, str) or not APP_ID_PATTERN.fullmatch(app_id):
        raise ValueError("Product ManifestのApp IDが不正である")
    if (
        not isinstance(audit_store_id, str)
        or not AUDIT_STORE_ID_PATTERN.fullmatch(audit_store_id)
    ):
        raise ValueError("Product ManifestのAudit Store IDが不正である")
    if app_id != receipt.get("app_id") or audit_store_id != receipt.get("audit_store_id"):
        raise ValueError("Product Manifest identityがExport Receiptと一致しない")

    launcher = _regular_file(
        bundle / "gui_shell_desktop_launcher.exe",
        "Rust Desktop Launcher",
        max_bytes=MAX_LAUNCHER_BYTES,
    )
    embedded_app_ids, embedded_audit_ids = _embedded_identity(launcher)
    if embedded_app_ids != {app_id} or embedded_audit_ids != {audit_store_id}:
        raise ValueError("Rust Desktop Launcherのcompile-time identityがManifestと一致しない")
    version = product_version_from_source(ROOT)
    if not re.fullmatch(r"[0-9][A-Za-z0-9.+_-]{0,63}", version):
        raise ValueError("製品版のSetup出力名が不正である")
    return current_commit, app_id, audit_store_id, version


def _find_iscc(explicit: Path | None) -> Path:
    candidates: list[Path] = []
    if explicit is not None:
        candidates.append(explicit)
    located = shutil.which("ISCC.exe") or shutil.which("iscc")
    if located:
        candidates.append(Path(located))
    for variable in ("LOCALAPPDATA", "ProgramFiles(x86)", "ProgramFiles"):
        base = os.environ.get(variable)
        if base:
            candidates.append(Path(base) / "Programs" / "Inno Setup 6" / "ISCC.exe")
            candidates.append(Path(base) / "Inno Setup 6" / "ISCC.exe")
    for candidate in candidates:
        try:
            resolved = candidate.resolve(strict=True)
            if (
                resolved.is_file()
                and not resolved.is_symlink()
                and not _is_reparse_point(resolved.lstat())
            ):
                return resolved
        except OSError:
            continue
    raise ValueError("Inno Setup 6 ISCC.exeが見つからない。--isccでcompiler pathを指定する")


def _ispp_string(value: str) -> str:
    if '"' in value or "\r" in value or "\n" in value:
        raise ValueError("Inno Setup defineに不正な文字がある")
    return '"' + value + '"'


def build_windows_installer(
    bundle_dir: Path,
    output_dir: Path,
    *,
    iscc_path: Path | None = None,
) -> dict[str, Any]:
    if sys.platform != "win32":
        raise ValueError("Windows Setupの生成はWindows上で実行する")
    if (
        not ISS_SCRIPT.is_file()
        or ISS_SCRIPT.is_symlink()
        or _is_reparse_point(ISS_SCRIPT.lstat())
    ):
        raise ValueError("tracked Inno Setup scriptが通常fileではない")

    source_commit, app_id, audit_store_id, product_version = validate_export_bundle(bundle_dir)
    source_bundle = bundle_dir.resolve(strict=True)
    output = _outside_source_roots(output_dir, "Setup output directory")
    if (
        output == source_bundle
        or output.is_relative_to(source_bundle)
        or source_bundle.is_relative_to(output)
    ):
        raise ValueError("Setup output directoryとExport bundleは重複してはならない")
    if any(output.iterdir()):
        raise ValueError("Setup output directoryは空でなければならない")
    setup_path = output / f"D4Pocket-Setup-{product_version}.exe"
    if setup_path.exists() or setup_path.is_symlink():
        raise ValueError("Setup output fileがすでに存在する。上書きしない")

    compiler = _find_iscc(iscc_path)
    with tempfile.TemporaryDirectory(prefix="d4p-setup-script-") as temporary:
        temporary_root = Path(temporary)
        receipt, receipt_raw = _read_json(
            source_bundle / RECEIPT_NAME, MAX_INPUT_BYTES, "Windows Export Receipt"
        )
        bundle = temporary_root / "export-bundle"
        _snapshot_export_bundle(source_bundle, bundle, receipt["artifact_files"], receipt_raw)
        if validate_export_bundle(bundle) != (
            source_commit,
            app_id,
            audit_store_id,
            product_version,
        ):
            raise ValueError("一時snapshotのReceipt／Manifest identityが入力bundleと一致しない")
        payload_components = _inspect_payload_components(bundle, receipt["artifact_files"])
        wrapper = temporary_root / "D4Pocket-Setup.iss"
        wrapper.write_text(
            "\n".join(
                (
                    f"#define BundleDir {_ispp_string(str(bundle))}",
                    f"#define ProductAppId {_ispp_string(app_id)}",
                    f"#define ProductVersion {_ispp_string(product_version)}",
                    f'#include "{ISS_SCRIPT.as_posix()}"',
                    "",
                )
            ),
            encoding="utf-8",
            newline="\n",
        )
        command = [str(compiler), f"-O{output}", str(wrapper)]
        completed = subprocess.run(
            command,
            cwd=ROOT,
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            encoding="utf-8",
            errors="replace",
        )
    if completed.returncode != 0:
        raise RuntimeError(
            f"ISCC failed (exit={completed.returncode})\n{completed.stdout[-12000:]}"
        )
    metadata = setup_path.lstat()
    if not stat.S_ISREG(metadata.st_mode) or _is_reparse_point(metadata) or metadata.st_size < 1:
        raise RuntimeError("ISCCの出力Setup.exeを通常fileとして検証できない")
    setup_sha256, setup_size_bytes = _hash_regular_file(setup_path, "Windows Setup.exe")
    setup_architecture = _pe_architecture(setup_path)
    setup_authenticode = _authenticode_info(setup_path)
    if setup_authenticode["status"] != "NotSigned":
        raise ValueError("Setup candidateは署名しないrecipeの出力として不整合である")
    verified_setup_hash, verified_setup_size = _hash_regular_file(
        setup_path, "Windows Setup.exe after Authenticode check"
    )
    if (
        setup_size_bytes != metadata.st_size
        or verified_setup_hash != setup_sha256
        or verified_setup_size != setup_size_bytes
    ):
        raise ValueError("Authenticode確認中にWindows Setup.exeが変化した")
    return {
        "source_commit": source_commit,
        "app_id": app_id,
        "audit_store_id": audit_store_id,
        "product_version": product_version,
        "setup_path": str(setup_path),
        "setup_size_bytes": setup_size_bytes,
        "setup_sha256": setup_sha256,
        "authenticode": "not_signed_by_this_build_recipe",
        "setup_architecture": setup_architecture,
        "setup_authenticode_status": setup_authenticode["status"],
        "setup_signer_thumbprint": setup_authenticode["signer_thumbprint"],
        "payload_components": payload_components,
        "authenticode_evidence_source": {
            "evidence_class": "EXTERNAL_EVIDENCE",
            "tool": "PowerShell Get-AuthenticodeSignature",
        },
        "product_installed_runtime": "not_exercised",
    }


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Broker Export Receiptに束縛したWindows per-user bootstrap Setupを生成する。"
    )
    parser.add_argument(
        "--source-dir", type=Path, required=True, help="検証済みWindows Export bundle"
    )
    parser.add_argument(
        "--output-dir", type=Path, required=True, help="事前作成済みの外部output directory"
    )
    parser.add_argument(
        "--iscc", type=Path, help="Inno Setup 6 ISCC.exe。省略時はPATH等から検出"
    )
    args = parser.parse_args()
    try:
        result = build_windows_installer(args.source_dir, args.output_dir, iscc_path=args.iscc)
    except (OSError, ValueError, RuntimeError, json.JSONDecodeError) as error:
        print(f"D4 Pocket Setup生成を停止した: {error}", file=sys.stderr)
        return 1
    print(json.dumps(result, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
