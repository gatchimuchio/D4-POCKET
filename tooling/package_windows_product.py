"""未署名Windows portable buildをBroker更新用のD4PKG01へ梱包する開発用tool。"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import stat
import struct
import sys
import uuid
from pathlib import Path, PurePosixPath
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from tooling.export_windows_product import (
    RECEIPT_NAME,
    _is_one_drive_path,
    _is_reparse_point,
    _read_regular_file,
    _require_clean_source,
    _schema,
    _sha256_file,
    product_version_from_source,
    validate_build_evidence,
)
from tooling.schema_check.check_schemas import validate_instance

MAGIC = b"D4PKG01\n"
MAX_MANIFEST_BYTES = 1024 * 1024
MAX_PACKAGE_BYTES = 4 * 1024 * 1024 * 1024
PACKAGE_SCHEMA = ROOT / "specs" / "d4_pocket_product_package_manifest.schema.json"
REQUIRED_PATHS = {
    "app/gui_shell_desktop.exe",
    "app/flutter_windows.dll",
    "app/data/app.so",
    "app/data/icudtl.dat",
    "broker/gui_shell_rust_helper.exe",
    "gui_shell_desktop_launcher.exe",
    "product_manifest.json",
}
PAYLOAD_ROOTS = {"app", "broker"}
ROOT_FILES = {"gui_shell_desktop_launcher.exe", "product_manifest.json"}
MAX_PRODUCT_MANIFEST_BYTES = 64 * 1024


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("Product Manifestに重複JSON keyがある")
        result[key] = value
    return result


def _validate_product_manifest(source_root: Path, app_id: str, audit_store_id: str) -> None:
    path = source_root / "product_manifest.json"
    metadata = path.lstat()
    if not stat.S_ISREG(metadata.st_mode) or stat.S_ISLNK(metadata.st_mode) or _is_reparse_point(metadata):
        raise ValueError("Product Manifestが通常fileではない")
    if metadata.st_size > MAX_PRODUCT_MANIFEST_BYTES:
        raise ValueError("Product Manifestが64 KiB上限を超える")
    with path.open("rb") as source:
        raw = source.read(MAX_PRODUCT_MANIFEST_BYTES + 1)
    if len(raw) > MAX_PRODUCT_MANIFEST_BYTES:
        raise ValueError("Product Manifestが64 KiB上限を超える")
    try:
        value = json.loads(raw.decode("utf-8"), object_pairs_hook=_unique_object)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ValueError("Product ManifestがUTF-8 JSONではない") from error
    if not isinstance(value, dict):
        raise ValueError("Product ManifestがJSON objectではない")
    product = value.get("manifest") if isinstance(value, dict) else None
    app_identity = product.get("app_identity") if isinstance(product, dict) else None
    audit = product.get("audit_store") if isinstance(product, dict) else None
    inheritance = product.get("inheritance_policy") if isinstance(product, dict) else None
    if (
        type(value.get("version")) is not int
        or value.get("version") != 1
        or value.get("product") != "D4 Pocket"
        or not isinstance(value.get("export_id"), str)
        or not re.fullmatch(r"[A-Za-z0-9._-]{1,64}", value["export_id"])
        or not isinstance(app_identity, dict)
        or app_identity.get("app_id") != app_id
        or not isinstance(audit, dict)
        or audit.get("store_id") != audit_store_id
        or audit.get("chain_status") != "new"
        or audit.get("inherited") is not False
        or not isinstance(inheritance, dict)
        or any(inheritance.get(field) != "none" for field in ("authority", "permission", "approval", "credential", "audit_chain"))
    ):
        raise ValueError("Product Manifestの新規identity／Authority非継承条件が一致しない")


def _validate_relative_path(value: str) -> PurePosixPath:
    path = PurePosixPath(value)
    if (
        not value
        or not value.isascii()
        or "\\" in value
        or ":" in value
        or path.is_absolute()
        or any(part in ("", ".", "..") for part in path.parts)
        or any(part.endswith((".", " ")) for part in path.parts)
        or any(any(ord(character) < 0x20 or character in '<>:"|?*' for character in part) for part in path.parts)
    ):
        raise ValueError("product package pathがWindows通常file pathではない")
    for part in path.parts:
        stem = part.split(".", 1)[0].upper()
        if stem in {"CON", "PRN", "AUX", "NUL"} or re.fullmatch(r"(?:COM|LPT)[1-9]", stem):
            raise ValueError("product package pathにWindows予約名がある")
    if path.parts[0] not in PAYLOAD_ROOTS and not (len(path.parts) == 1 and path.parts[0] in ROOT_FILES):
        raise ValueError("product package pathがpayload root外を指している")
    return path


def _inventory_payload(source_root: Path) -> list[dict[str, Any]]:
    root_info = source_root.lstat()
    if not stat.S_ISDIR(root_info.st_mode) or stat.S_ISLNK(root_info.st_mode) or _is_reparse_point(root_info):
        raise ValueError("product bundle rootが通常directoryではない")
    resolved_root = source_root.resolve(strict=True)
    records: list[dict[str, Any]] = []
    seen: set[str] = set()
    for path in sorted(source_root.rglob("*"), key=lambda item: item.relative_to(source_root).as_posix().casefold()):
        metadata = path.lstat()
        if stat.S_ISDIR(metadata.st_mode):
            if stat.S_ISLNK(metadata.st_mode) or _is_reparse_point(metadata):
                raise ValueError("product bundleにreparse directoryがある")
            continue
        if not stat.S_ISREG(metadata.st_mode) or stat.S_ISLNK(metadata.st_mode) or _is_reparse_point(metadata):
            raise ValueError("product bundleに通常file以外がある")
        relative = path.relative_to(source_root).as_posix()
        if relative == RECEIPT_NAME or not (relative.split("/", 1)[0] in PAYLOAD_ROOTS or relative in ROOT_FILES):
            continue
        _validate_relative_path(relative)
        folded = relative.casefold()
        if folded in seen:
            raise ValueError("product package pathがWindows上で重複する")
        seen.add(folded)
        if not path.resolve(strict=True).is_relative_to(resolved_root):
            raise ValueError("product package fileがroot外を指している")
        records.append({
            "path": relative,
            "byte_length": metadata.st_size,
            "sha256": _sha256_file(path),
        })
    records.sort(key=lambda item: item["path"].casefold())
    if not REQUIRED_PATHS.issubset({item["path"] for item in records}):
        raise ValueError("product packageに必須app／Broker／Manifest fileが揃っていない")
    total = sum(item["byte_length"] for item in records)
    if total + MAX_MANIFEST_BYTES + 12 > MAX_PACKAGE_BYTES:
        raise ValueError("product packageが4 GiB上限を超える")
    return records


def write_product_package(
    source_root: Path,
    output_path: Path,
    *,
    product_version: str,
    app_id: str,
    audit_store_id: str,
) -> dict[str, Any]:
    source_root = source_root.resolve(strict=True)
    output_candidate = output_path.expanduser()
    if not output_candidate.is_absolute():
        output_candidate = Path.cwd() / output_candidate
    if output_candidate.name in ("", ".", ".."):
        raise ValueError("package output filenameが不正である")
    parent = output_candidate.parent.resolve(strict=True)
    output = parent / output_candidate.name
    if output.exists() or output.is_symlink() or output.is_relative_to(source_root) or _is_one_drive_path(output):
        raise ValueError("package outputは新規かつbundle／Repository／OneDrive外でなければならない")

    _validate_product_manifest(source_root, app_id, audit_store_id)

    manifest = {
        "version": 1,
        "product": "D4 Pocket",
        "product_version": product_version,
        "app_id": app_id,
        "audit_store_id": audit_store_id,
        "files": _inventory_payload(source_root),
    }
    errors = validate_instance(manifest, _schema(PACKAGE_SCHEMA.name))
    if errors:
        raise ValueError("product package manifestがSchemaに適合しない: " + "; ".join(errors))
    manifest_bytes = json.dumps(manifest, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
    if not manifest_bytes or len(manifest_bytes) > MAX_MANIFEST_BYTES:
        raise ValueError("product package manifest byte長が上限を超える")

    temporary = parent / f".{output.name}.{uuid.uuid4().hex}.part"
    digest = hashlib.sha256()
    byte_count = 0
    try:
        with temporary.open("xb") as destination:
            for piece in (MAGIC, struct.pack("<I", len(manifest_bytes)), manifest_bytes):
                destination.write(piece)
                digest.update(piece)
                byte_count += len(piece)
            for record in manifest["files"]:
                source = source_root / PurePosixPath(record["path"])
                file_digest = hashlib.sha256()
                copied = 0
                with source.open("rb") as input_file:
                    while True:
                        chunk = input_file.read(1024 * 1024)
                        if not chunk:
                            break
                        destination.write(chunk)
                        digest.update(chunk)
                        file_digest.update(chunk)
                        copied += len(chunk)
                        byte_count += len(chunk)
                if copied != record["byte_length"] or file_digest.hexdigest() != record["sha256"]:
                    raise ValueError("package作成中に入力artifactが変化した")
                if byte_count > MAX_PACKAGE_BYTES:
                    raise ValueError("product packageが4 GiB上限を超える")
            destination.flush()
            os.fsync(destination.fileno())
        if output.exists() or output.is_symlink():
            raise ValueError("package作成中にoutput pathが作成された")
        os.rename(temporary, output)
    except Exception:
        try:
            temporary.unlink(missing_ok=True)
        except OSError:
            pass
        raise
    return {
        "product_version": product_version,
        "app_id": app_id,
        "audit_store_id": audit_store_id,
        "file_count": len(manifest["files"]),
        "package_size_bytes": byte_count,
        "package_sha256": digest.hexdigest(),
        "output_path": str(output),
    }


def build_export_package(source_root: Path, output_path: Path) -> dict[str, Any]:
    receipt_path = source_root / RECEIPT_NAME
    receipt_raw = _read_regular_file(receipt_path, 4 * 1024 * 1024, "Windows Export build receipt")
    evidence = json.loads(receipt_raw.decode("utf-8"))
    validate_build_evidence(evidence, source_root)
    if _require_clean_source() != evidence["source_commit"]:
        raise ValueError("product package作成sourceがExport build receiptのcommitと一致しない")
    product_version = product_version_from_source(ROOT)
    manifest = json.loads((source_root / "product_manifest.json").read_text(encoding="utf-8"))
    identity = manifest.get("manifest", {})
    app_id = identity.get("app_identity", {}).get("app_id")
    audit_store_id = identity.get("audit_store", {}).get("store_id")
    if app_id != evidence.get("app_id") or audit_store_id != evidence.get("audit_store_id"):
        raise ValueError("portable Product Manifestとbuild receiptのidentityが一致しない")
    return write_product_package(
        source_root,
        output_path,
        product_version=product_version,
        app_id=app_id,
        audit_store_id=audit_store_id,
    )


def main() -> int:
    parser = argparse.ArgumentParser(description="検証済みWindows書出し後の配置一式をD4PKG01へ梱包する開発用tool。")
    parser.add_argument("--source-dir", type=Path, required=True, help="Windows書出し後の配置一式")
    parser.add_argument("--output", type=Path, required=True, help="新規package保存先。Repository／OneDrive外")
    args = parser.parse_args()
    try:
        result = build_export_package(args.source_dir, args.output)
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"D4 Pocket package作成を停止した: {error}", file=sys.stderr)
        return 1
    print(json.dumps(result, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
