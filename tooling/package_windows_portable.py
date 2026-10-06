"""検証済みWindows Export bundleを利用者向けportable ZIPへ梱包する開発用tool。"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import stat
import sys
import uuid
import zipfile
from pathlib import Path, PurePosixPath
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from tooling.export_windows_product import (
    RECEIPT_NAME,
    _is_one_drive_path,
    _parse_json_bytes,
    _read_regular_file,
    _require_clean_source,
    _schema,
    _sha256_file,
    product_version_from_source,
    validate_build_evidence,
)
from tooling.package_windows_product import (
    MAX_PRODUCT_MANIFEST_BYTES,
    _inventory_payload,
    _validate_product_manifest,
)
from tooling.schema_check.check_schemas import validate_instance

PORTABLE_ROOT = "D4 Pocket"
GUIDE_NAME = "はじめに.txt"
ZIP_COMPRESSION = zipfile.ZIP_DEFLATED
ZIP_COMPRESSION_LEVEL = 6

GUIDE = """D4 Pocket portable版 — はじめに

起動方法
1. ZIP全体を、書込み可能なローカルフォルダーへ展開してください。
2. 展開後の「D4 Pocket」フォルダー内にある gui_shell_desktop_launcher.exe を起動してください。
3. 起動器、app、broker、product_manifest.jsonを別々に移動・削除しないでください。

この版について
- このZIPは未署名の検証用portable packageです。正式配布物ではありません。
- WindowsのSmart App Controlやセキュリティ機能が起動を止めた場合、保護設定を変更せず、起動を中止してください。
- source commit、build receipt、artifact inventoryは同梱されています。receiptはbuild記録であり、署名・Owner承認・起動成功の証明ではありません。
- 実行時データは展開フォルダーとは別の利用者別領域に保存されます。ZIPや展開物を削除しても利用者データの削除を意味しません。
- このpackage作成はInstaller、Update、Rollback、Repair、署名、正式配布を行いません。
""".replace("\n", "\r\n").encode("utf-8")


def _resolve_output_file(value: Path, source_root: Path) -> Path:
    candidate = value.expanduser()
    if not candidate.is_absolute():
        candidate = ROOT / candidate
    if candidate.name in ("", ".", "..") or candidate.suffix.casefold() != ".zip":
        raise ValueError("portable ZIP outputは.zip拡張子のfileでなければならない")
    parent = candidate.parent.resolve(strict=True)
    output = parent / candidate.name
    if output.exists() or output.is_symlink():
        raise ValueError("portable ZIP outputが既に存在するため上書きを拒否した")
    if output.is_relative_to(ROOT.resolve()) or output.is_relative_to(source_root):
        raise ValueError("portable ZIPはRepository／入力bundleの外へ出力する")
    if _is_one_drive_path(output):
        raise ValueError("portable ZIPはOneDrive同期root外へ出力する")
    return output


def _read_evidence(source_root: Path) -> tuple[dict[str, Any], bytes]:
    receipt_raw = _read_regular_file(
        source_root / RECEIPT_NAME, 4 * 1024 * 1024, "Windows Export build receipt"
    )
    evidence = _parse_json_bytes(receipt_raw)
    if not isinstance(evidence, dict):
        raise ValueError("Windows Export build receiptがJSON objectではない")
    errors = validate_instance(evidence, _schema("gui_shell_windows_export_build.schema.json"))
    if errors:
        raise ValueError("Windows Export build receiptがSchemaに適合しない: " + "; ".join(errors))
    validate_build_evidence(evidence, source_root)
    return evidence, receipt_raw


def _verified_records(source_root: Path, evidence: dict[str, Any]) -> list[dict[str, Any]]:
    records = _inventory_payload(source_root)
    expected = [
        {
            "path": record["path"],
            "byte_length": record["size_bytes"],
            "sha256": record["sha256"],
        }
        for record in evidence["artifact_files"]
    ]
    if records != expected:
        raise ValueError("portable ZIP payload inventoryがExport build receiptと一致しない")
    manifest_raw = _read_regular_file(
        source_root / "product_manifest.json",
        MAX_PRODUCT_MANIFEST_BYTES,
        "portable Product Manifest",
    )
    manifest = _parse_json_bytes(manifest_raw)
    product = manifest.get("manifest", {}) if isinstance(manifest, dict) else {}
    app_id = product.get("app_identity", {}).get("app_id") if isinstance(product, dict) else None
    audit_store_id = product.get("audit_store", {}).get("store_id") if isinstance(product, dict) else None
    if app_id != evidence["app_id"] or audit_store_id != evidence["audit_store_id"]:
        raise ValueError("portable Product Manifestのidentityがbuild receiptと一致しない")
    _validate_product_manifest(source_root, app_id, audit_store_id)
    return records


def _zip_info(name: str) -> zipfile.ZipInfo:
    info = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
    info.compress_type = ZIP_COMPRESSION
    info.create_system = 0
    info.external_attr = 0
    return info


def _hash_zip_entry(archive: zipfile.ZipFile, name: str) -> tuple[int, str]:
    digest = hashlib.sha256()
    length = 0
    with archive.open(name, "r") as source:
        while chunk := source.read(1024 * 1024):
            digest.update(chunk)
            length += len(chunk)
    return length, digest.hexdigest()


def _verify_archive(
    path: Path,
    records: list[dict[str, Any]],
    receipt_raw: bytes,
) -> tuple[int, str]:
    expected: dict[str, tuple[int, str]] = {
        f"{PORTABLE_ROOT}/{record['path']}": (record["byte_length"], record["sha256"])
        for record in records
    }
    expected[f"{PORTABLE_ROOT}/{RECEIPT_NAME}"] = (
        len(receipt_raw), hashlib.sha256(receipt_raw).hexdigest()
    )
    expected[f"{PORTABLE_ROOT}/{GUIDE_NAME}"] = (
        len(GUIDE), hashlib.sha256(GUIDE).hexdigest()
    )
    if len({name.casefold() for name in expected}) != len(expected):
        raise ValueError("portable ZIPにWindows上のcase-insensitive path衝突がある")

    seen: set[str] = set()
    total_uncompressed = 0
    with zipfile.ZipFile(path, "r") as archive:
        for info in archive.infolist():
            name = info.filename
            folded = name.casefold()
            if folded in seen or name not in expected or info.is_dir():
                raise ValueError("portable ZIPに重複・予期しないentryがある")
            seen.add(folded)
            length, digest = _hash_zip_entry(archive, name)
            expected_length, expected_digest = expected[name]
            if length != expected_length or digest != expected_digest:
                raise ValueError("portable ZIP entryがsource inventoryと一致しない")
            total_uncompressed += length
        if seen != {name.casefold() for name in expected}:
            raise ValueError("portable ZIP entryが不足している")
        if archive.testzip() is not None:
            raise ValueError("portable ZIPのCRC検証に失敗した")
    return total_uncompressed, _sha256_file(path)


def _write_portable_zip(
    source_root: Path,
    output: Path,
    evidence: dict[str, Any],
    receipt_raw: bytes,
    product_version: str,
) -> dict[str, Any]:
    records = _verified_records(source_root, evidence)
    expected_version_arg = f"--dart-define=GUI_SHELL_PRODUCT_VERSION={product_version}"
    if expected_version_arg not in evidence["flutter_build_arguments"]:
        raise ValueError("Export receiptの製品版が現在sourceの製品版と一致しない")

    temporary = output.parent / f".{output.name}.{uuid.uuid4().hex}.part"
    try:
        with zipfile.ZipFile(
            temporary,
            mode="x",
            compression=ZIP_COMPRESSION,
            compresslevel=ZIP_COMPRESSION_LEVEL,
            allowZip64=True,
        ) as archive:
            for record in records:
                relative = record["path"]
                source_path = source_root / PurePosixPath(relative)
                name = f"{PORTABLE_ROOT}/{relative}"
                info = _zip_info(name)
                digest = hashlib.sha256()
                copied = 0
                with source_path.open("rb") as source, archive.open(info, "w") as destination:
                    while chunk := source.read(1024 * 1024):
                        destination.write(chunk)
                        digest.update(chunk)
                        copied += len(chunk)
                if copied != record["byte_length"] or digest.hexdigest() != record["sha256"]:
                    raise ValueError("ZIP作成中にportable artifactが変化した")

            for name, content in (
                (f"{PORTABLE_ROOT}/{RECEIPT_NAME}", receipt_raw),
                (f"{PORTABLE_ROOT}/{GUIDE_NAME}", GUIDE),
            ):
                archive.writestr(_zip_info(name), content)

        with temporary.open("rb+") as archive_file:
            os.fsync(archive_file.fileno())
        uncompressed_bytes, archive_sha256 = _verify_archive(temporary, records, receipt_raw)
        current_evidence, current_receipt = _read_evidence(source_root)
        if current_receipt != receipt_raw or current_evidence != evidence:
            raise ValueError("portable ZIP作成中にExport receiptが変化した")
        _verified_records(source_root, current_evidence)
        if output.exists() or output.is_symlink():
            raise ValueError("portable ZIP作成中にoutput pathが作成されたため公開を停止した")
        os.rename(temporary, output)
        return {
            "portable_zip_created": True,
            "product": "D4 Pocket",
            "product_version": product_version,
            "source_commit": evidence["source_commit"],
            "file_count": len(records) + 2,
            "uncompressed_bytes": uncompressed_bytes,
            "archive_bytes": output.stat().st_size,
            "archive_sha256": archive_sha256,
            "signed": False,
            "formal_distribution_claimed": False,
            "standalone_app_verified": False,
            "output_path": str(output),
        }
    except Exception:
        try:
            temporary.unlink(missing_ok=True)
        except OSError:
            pass
        raise


def build_export_portable_zip(source_dir: Path, output_path: Path) -> dict[str, Any]:
    if sys.platform != "win32":
        raise ValueError("Windows Export portable ZIPはWindows host専用である")
    source_metadata = source_dir.lstat()
    if (
        not stat.S_ISDIR(source_metadata.st_mode)
        or stat.S_ISLNK(source_metadata.st_mode)
        or (getattr(source_metadata, "st_file_attributes", 0) & 0x400)
    ):
        raise ValueError("入力portable bundle rootは通常directoryでなければならない")
    source_root = source_dir.resolve(strict=True)
    if source_root.is_relative_to(ROOT.resolve()) or _is_one_drive_path(source_root):
        raise ValueError("入力portable bundleはRepository／OneDrive同期root外に置く")
    output = _resolve_output_file(output_path, source_root)
    source_commit = _require_clean_source()
    evidence, receipt_raw = _read_evidence(source_root)
    if source_commit != evidence["source_commit"]:
        raise ValueError("portable ZIP source commitがExport build receiptと一致しない")
    product_version = product_version_from_source(ROOT)
    return _write_portable_zip(source_root, output, evidence, receipt_raw, product_version)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="検証済みWindows Export配置一式から未署名portable ZIPを作る開発専用tool。"
    )
    parser.add_argument("--source-dir", type=Path, required=True, help="Windows Export後の配置directory")
    parser.add_argument("--output", type=Path, required=True, help="新規ZIP保存先。Repository／OneDrive外")
    args = parser.parse_args()
    try:
        result = build_export_portable_zip(args.source_dir, args.output)
    except (OSError, ValueError, json.JSONDecodeError, zipfile.BadZipFile) as error:
        print(f"D4 Pocket portable ZIP作成を停止した: {error}", file=sys.stderr)
        return 1
    print(json.dumps(result, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
