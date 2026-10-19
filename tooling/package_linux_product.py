"""既存Linux bundleを未署名D4PKG01へ固定形式で梱包する開発用tool。"""

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

from tooling.export_windows_product import scan_credential_artifacts
from tooling.schema_check.check_schemas import validate_instance

MAGIC = b"D4PKG01\n"
MAX_MANIFEST_BYTES = 1024 * 1024
MAX_PRODUCT_MANIFEST_BYTES = 64 * 1024
MAX_PACKAGE_BYTES = 4 * 1024 * 1024 * 1024
MAX_FILES = 20_000
COPY_BUFFER_BYTES = 1024 * 1024
PACKAGE_SCHEMA = ROOT / "specs" / "d4_pocket_linux_product_package_manifest.schema.json"
APP_ID_PATTERN = re.compile(r"d4-pocket-app-[a-f0-9]{32}\Z")
AUDIT_ID_PATTERN = re.compile(r"audit-store-[a-f0-9]{32}\Z")
VERSION_PATTERN = re.compile(r"[A-Za-z0-9][A-Za-z0-9.+_-]{0,63}\Z")
EXECUTABLE_PATHS = {
    "gui_shell_desktop",
    "gui_shell_desktop_launcher",
    "libexec/gui_shell_linux_broker",
}
REQUIRED_PATHS = {
    "data/icudtl.dat",
    "gui_shell_desktop",
    "gui_shell_desktop_launcher",
    "lib/libflutter_linux_gtk.so",
    "libexec/gui_shell_linux_broker",
    "product_manifest.json",
}
ROOT_FILES = {
    "gui_shell_desktop",
    "gui_shell_desktop_launcher",
    "product_manifest.json",
}
DIRECTORY_FLAGS = (
    os.O_RDONLY
    | getattr(os, "O_CLOEXEC", 0)
    | getattr(os, "O_DIRECTORY", 0)
    | getattr(os, "O_NOFOLLOW", 0)
)
FILE_FLAGS = (
    os.O_RDONLY
    | getattr(os, "O_CLOEXEC", 0)
    | getattr(os, "O_NOFOLLOW", 0)
    | getattr(os, "O_NONBLOCK", 0)
)


def _require_linux() -> None:
    if not sys.platform.startswith("linux") or not hasattr(os, "O_NOFOLLOW"):
        raise ValueError("Linux host上でのみLinux product packageを生成できる")


def _schema() -> dict[str, Any]:
    value = json.loads(PACKAGE_SCHEMA.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError("Linux product package SchemaがJSON objectではない")
    return value


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("Product Manifestに重複JSON keyがある")
        result[key] = value
    return result


def _validate_relative_path(value: str) -> PurePosixPath:
    path = PurePosixPath(value)
    parts = path.parts
    if (
        not value
        or len(value) > 1024
        or not value.isascii()
        or "\\" in value
        or ":" in value
        or path.is_absolute()
        or any(part in ("", ".", "..") for part in parts)
        or any(part.endswith((".", " ")) for part in parts)
        or any(any(ord(character) < 0x20 or character in '<>:"|?*' for character in part) for part in parts)
    ):
        raise ValueError("Linux product package pathが固定file pathではない")
    for part in parts:
        stem = part.split(".", 1)[0].upper()
        if stem in {"CON", "PRN", "AUX", "NUL"} or re.fullmatch(r"(?:COM|LPT)[1-9]", stem):
            raise ValueError("Linux product package pathに予約名がある")
    if (
        (parts[0] in {"data", "lib"} and len(parts) >= 2)
        or value == "libexec/gui_shell_linux_broker"
        or value in ROOT_FILES
    ):
        return path
    raise ValueError("Linux product package pathが固定payload root外を指している")


def _validate_directory_path(value: str) -> None:
    parts = PurePosixPath(value).parts
    if (
        not value
        or len(value) > 1024
        or not value.isascii()
        or not parts
        or any(part in ("", ".", "..") for part in parts)
    ):
        raise ValueError("Linux product bundle directory pathが不正である")
    if parts[0] in {"data", "lib"} or value == "libexec":
        return
    raise ValueError("Linux product bundleに許可されないdirectoryがある")


def _same_file(left: os.stat_result, right: os.stat_result) -> bool:
    return (
        left.st_dev == right.st_dev
        and left.st_ino == right.st_ino
        and stat.S_IFMT(left.st_mode) == stat.S_IFMT(right.st_mode)
        and stat.S_IMODE(left.st_mode) == stat.S_IMODE(right.st_mode)
        and left.st_size == right.st_size
        and left.st_nlink == right.st_nlink
        and left.st_mtime_ns == right.st_mtime_ns
        and left.st_ctime_ns == right.st_ctime_ns
    )


def _open_directory(root_fd: int, relative: str) -> int:
    current = os.dup(root_fd)
    if not relative:
        return current
    try:
        for component in relative.split("/"):
            next_fd = os.open(component, DIRECTORY_FLAGS, dir_fd=current)
            opened = os.fstat(next_fd)
            if not stat.S_ISDIR(opened.st_mode):
                os.close(next_fd)
                raise ValueError("Linux product bundleに通常directory以外がある")
            os.close(current)
            current = next_fd
        return current
    except Exception:
        os.close(current)
        raise


def _open_file(root_fd: int, relative: str) -> int:
    components = relative.split("/")
    parent = _open_directory(root_fd, "/".join(components[:-1]))
    try:
        return os.open(components[-1], FILE_FLAGS, dir_fd=parent)
    finally:
        os.close(parent)


def _digest_fd(fd: int, *, capture_limit: int | None = None) -> tuple[int, str, bytes | None, os.stat_result]:
    before = os.fstat(fd)
    if not stat.S_ISREG(before.st_mode) or before.st_nlink != 1:
        raise ValueError("Linux product bundleに通常の単独file以外がある")
    permissions = stat.S_IMODE(before.st_mode)
    if permissions & 0o7022 or not permissions & 0o400:
        raise ValueError("Linux product bundle fileのpermissionが安全でない")
    digest = hashlib.sha256()
    captured = bytearray() if capture_limit is not None else None
    length = 0
    os.lseek(fd, 0, os.SEEK_SET)
    while True:
        piece = os.read(fd, COPY_BUFFER_BYTES)
        if not piece:
            break
        length += len(piece)
        if length > MAX_PACKAGE_BYTES:
            raise ValueError("Linux product packageが4 GiB上限を超える")
        digest.update(piece)
        if captured is not None:
            if length > capture_limit:
                raise ValueError("Product Manifestが64 KiB上限を超える")
            captured.extend(piece)
    after = os.fstat(fd)
    if length != before.st_size or not _same_file(before, after):
        raise ValueError("Linux product bundle fileが読取中に変化した")
    return length, digest.hexdigest(), bytes(captured) if captured is not None else None, before


def _validate_product_manifest(raw: bytes) -> tuple[str, str]:
    try:
        value = json.loads(raw.decode("utf-8"), object_pairs_hook=_unique_object)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ValueError("Product ManifestがUTF-8 JSONではない") from error
    if not isinstance(value, dict):
        raise ValueError("Product ManifestがJSON objectではない")
    product = value.get("manifest")
    app_identity = product.get("app_identity") if isinstance(product, dict) else None
    audit = product.get("audit_store") if isinstance(product, dict) else None
    inheritance = product.get("inheritance_policy") if isinstance(product, dict) else None
    app_id = app_identity.get("app_id") if isinstance(app_identity, dict) else None
    audit_store_id = audit.get("store_id") if isinstance(audit, dict) else None
    if (
        type(value.get("version")) is not int
        or value.get("version") != 1
        or value.get("product") != "D4 Pocket"
        or not isinstance(value.get("export_id"), str)
        or not re.fullmatch(r"[A-Za-z0-9._-]{1,64}", value["export_id"])
        or not isinstance(app_id, str)
        or not APP_ID_PATTERN.fullmatch(app_id)
        or not isinstance(audit_store_id, str)
        or not AUDIT_ID_PATTERN.fullmatch(audit_store_id)
        or app_identity.get("target_platform") != "linux"
        or not isinstance(audit, dict)
        or audit.get("chain_status") != "new"
        or audit.get("inherited") is not False
        or not isinstance(inheritance, dict)
        or any(
            inheritance.get(field) != "none"
            for field in ("authority", "permission", "approval", "credential", "audit_chain")
        )
    ):
        raise ValueError("Product ManifestのLinux identity／Authority非継承条件が一致しない")
    distribution = product.get("distribution_metadata")
    if not isinstance(distribution, dict) or distribution.get("target_platform") != "linux":
        raise ValueError("Product Manifestのdistribution targetがLinuxではない")
    return app_id, audit_store_id


def _inventory_payload(root_fd: int) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    seen: set[str] = set()
    total = 0
    pending = [""]
    while pending:
        relative_directory = pending.pop()
        directory_fd = _open_directory(root_fd, relative_directory)
        try:
            with os.scandir(directory_fd) as iterator:
                names = sorted((entry.name for entry in iterator), key=str.casefold)
            for name in names:
                relative = f"{relative_directory}/{name}" if relative_directory else name
                if not name.isascii() or "/" in name or "\\" in name:
                    raise ValueError("Linux product bundleに非ASCIIまたは不正なpathがある")
                metadata = os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
                if stat.S_ISLNK(metadata.st_mode):
                    raise ValueError("Linux product bundleにsymlinkがある")
                if stat.S_ISDIR(metadata.st_mode):
                    _validate_directory_path(relative)
                    child_fd = os.open(name, DIRECTORY_FLAGS, dir_fd=directory_fd)
                    try:
                        if not _same_file(metadata, os.fstat(child_fd)):
                            raise ValueError("Linux product bundle directoryが検査中に変化した")
                    finally:
                        os.close(child_fd)
                    pending.append(relative)
                    continue
                _validate_relative_path(relative)
                folded = relative.casefold()
                if folded in seen:
                    raise ValueError("Linux product packageにcase-insensitive duplicateがある")
                seen.add(folded)
                fd = _open_file(root_fd, relative)
                try:
                    if not _same_file(metadata, os.fstat(fd)):
                        raise ValueError("Linux product bundle fileが検査中に変化した")
                    length, digest, _, opened = _digest_fd(fd)
                finally:
                    os.close(fd)
                mode = 0o755 if relative in EXECUTABLE_PATHS else 0o644
                if relative in EXECUTABLE_PATHS and not (stat.S_IMODE(opened.st_mode) & 0o100):
                    raise ValueError("Linux product bundleのD4 executableにowner execute bitがない")
                total += length
                if total + MAX_MANIFEST_BYTES + len(MAGIC) + 4 > MAX_PACKAGE_BYTES:
                    raise ValueError("Linux product packageが4 GiB上限を超える")
                records.append({
                    "path": relative,
                    "byte_length": length,
                    "sha256": digest,
                    "mode": mode,
                    "_dev": opened.st_dev,
                    "_ino": opened.st_ino,
                    "_source_mode": stat.S_IMODE(opened.st_mode),
                    "_size": opened.st_size,
                    "_mtime_ns": opened.st_mtime_ns,
                    "_ctime_ns": opened.st_ctime_ns,
                })
                if len(records) > MAX_FILES:
                    raise ValueError("Linux product package file数が上限を超える")
        finally:
            os.close(directory_fd)
    records.sort(key=lambda item: item["path"].casefold())
    if not REQUIRED_PATHS.issubset({record["path"] for record in records}):
        raise ValueError("Linux product packageに必須Runner／Broker／Manifest fileが揃っていない")
    return records


def _verify_source_identity(record: dict[str, Any], opened: os.stat_result) -> None:
    if (
        not stat.S_ISREG(opened.st_mode)
        or opened.st_nlink != 1
        or opened.st_dev != record["_dev"]
        or opened.st_ino != record["_ino"]
        or opened.st_size != record["_size"]
        or stat.S_IMODE(opened.st_mode) != record["_source_mode"]
        or opened.st_mtime_ns != record["_mtime_ns"]
        or opened.st_ctime_ns != record["_ctime_ns"]
    ):
        raise ValueError("Linux product bundle file identityがpackage作成中に変化した")


def _write_all(fd: int, payload: bytes) -> None:
    view = memoryview(payload)
    while view:
        written = os.write(fd, view)
        if written <= 0:
            raise OSError("D4PKG01への書込みが進まない")
        view = view[written:]


def write_product_package(
    source_root: Path,
    output_path: Path,
    *,
    product_version: str,
) -> dict[str, Any]:
    """固定Linux bundleをD4PKG01にする。署名・trust・導入権限は生成しない。"""
    _require_linux()
    if not isinstance(product_version, str) or not VERSION_PATTERN.fullmatch(product_version):
        raise ValueError("Linux product versionが固定形式ではない")
    source_candidate = Path(source_root).expanduser().absolute()
    source_metadata = source_candidate.lstat()
    if not stat.S_ISDIR(source_metadata.st_mode) or stat.S_ISLNK(source_metadata.st_mode):
        raise ValueError("Linux product bundle rootが通常directoryではない")
    source = source_candidate.resolve(strict=True)
    source_fd = os.open(source, DIRECTORY_FLAGS)
    try:
        if not _same_file(source_metadata, os.fstat(source_fd)):
            raise ValueError("Linux product bundle rootが検査中に変化した")
        records = _inventory_payload(source_fd)
        manifest_record = next(
            record for record in records if record["path"] == "product_manifest.json"
        )
        manifest_fd = _open_file(source_fd, "product_manifest.json")
        try:
            manifest_length, manifest_digest, manifest_raw, manifest_stat = _digest_fd(
                manifest_fd, capture_limit=MAX_PRODUCT_MANIFEST_BYTES
            )
            _verify_source_identity(manifest_record, manifest_stat)
        finally:
            os.close(manifest_fd)
        if (
            manifest_raw is None
            or manifest_length != manifest_record["byte_length"]
            or manifest_digest != manifest_record["sha256"]
        ):
            raise ValueError("Product Manifestを読み取れない")
        app_id, audit_store_id = _validate_product_manifest(manifest_raw)

        package_manifest = {
            "version": 1,
            "product": "D4 Pocket",
            "product_version": product_version,
            "target_platform": "linux",
            "app_id": app_id,
            "audit_store_id": audit_store_id,
            "files": [
                {key: record[key] for key in ("path", "byte_length", "sha256", "mode")}
                for record in records
            ],
        }
        schema_errors = validate_instance(package_manifest, _schema())
        if schema_errors:
            raise ValueError("Linux product package目録が正本Schemaに適合しない: " + "; ".join(schema_errors))
        manifest_bytes = json.dumps(
            package_manifest, ensure_ascii=False, separators=(",", ":")
        ).encode("utf-8")
        if not manifest_bytes or len(manifest_bytes) > MAX_MANIFEST_BYTES:
            raise ValueError("Linux product package目録byte長が上限を超える")

        output_candidate = Path(output_path).expanduser()
        if not output_candidate.is_absolute():
            output_candidate = Path.cwd() / output_candidate
        if output_candidate.name in ("", ".", ".."):
            raise ValueError("Linux product package output filenameが不正である")
        parent = output_candidate.parent.resolve(strict=True)
        output = parent / output_candidate.name
        if (
            output.exists()
            or output.is_symlink()
            or output.is_relative_to(source)
            or output.is_relative_to(ROOT.resolve())
            or any(
                part.casefold() == "onedrive"
                or part.casefold().startswith("onedrive -")
                for part in output.parts
            )
        ):
            raise ValueError("package outputは新規かつbundle／Repository／OneDrive外でなければならない")

        credential_scan = scan_credential_artifacts(source)
        parent_fd = os.open(parent, DIRECTORY_FLAGS)
        temporary_name = f".{output.name}.{uuid.uuid4().hex}.part"
        published = False
        try:
            output_fd = os.open(
                temporary_name,
                os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_CLOEXEC", 0) | getattr(os, "O_NOFOLLOW", 0),
                0o600,
                dir_fd=parent_fd,
            )
            digest = hashlib.sha256()
            byte_count = 0
            try:
                os.fchmod(output_fd, 0o644)
                for piece in (MAGIC, struct.pack("<I", len(manifest_bytes)), manifest_bytes):
                    _write_all(output_fd, piece)
                    digest.update(piece)
                    byte_count += len(piece)
                for record in records:
                    fd = _open_file(source_fd, record["path"])
                    file_digest = hashlib.sha256()
                    copied = 0
                    try:
                        opened = os.fstat(fd)
                        _verify_source_identity(record, opened)
                        while True:
                            chunk = os.read(fd, COPY_BUFFER_BYTES)
                            if not chunk:
                                break
                            copied += len(chunk)
                            byte_count += len(chunk)
                            if byte_count > MAX_PACKAGE_BYTES:
                                raise ValueError("Linux product packageが4 GiB上限を超える")
                            file_digest.update(chunk)
                            digest.update(chunk)
                            view = memoryview(chunk)
                            while view:
                                written = os.write(output_fd, view)
                                if written <= 0:
                                    raise OSError("D4PKG01への書込みが進まない")
                                view = view[written:]
                        after = os.fstat(fd)
                        _verify_source_identity(record, after)
                    finally:
                        os.close(fd)
                    if copied != record["byte_length"] or file_digest.hexdigest() != record["sha256"]:
                        raise ValueError("Linux package作成中にsource artifactが変化した")
                os.fsync(output_fd)
            finally:
                os.close(output_fd)
            os.link(
                temporary_name,
                output.name,
                src_dir_fd=parent_fd,
                dst_dir_fd=parent_fd,
                follow_symlinks=False,
            )
            published = True
            os.unlink(temporary_name, dir_fd=parent_fd)
            os.fsync(parent_fd)
        except Exception:
            try:
                os.unlink(temporary_name, dir_fd=parent_fd)
            except OSError:
                pass
            if published:
                try:
                    os.unlink(output.name, dir_fd=parent_fd)
                    os.fsync(parent_fd)
                except OSError:
                    pass
            raise
        finally:
            os.close(parent_fd)
        return {
            "product_version": product_version,
            "target_platform": "linux",
            "app_id": app_id,
            "audit_store_id": audit_store_id,
            "file_count": len(records),
            "package_size_bytes": byte_count,
            "package_sha256": digest.hexdigest(),
            "credential_artifact_scan": credential_scan,
            "output_path": str(output),
            "signed": False,
            "installer_status": "not_started",
            "installed_product_verified": False,
        }
    finally:
        os.close(source_fd)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Linux Release bundleをBroker更新用の未署名D4PKG01へ梱包する開発専用tool。"
    )
    parser.add_argument("--source-dir", type=Path, required=True, help="Linux版のRelease生成物")
    parser.add_argument("--output", type=Path, required=True, help="新規package保存先。bundle／Repository／OneDrive外")
    parser.add_argument("--product-version", required=True, help="Product Manifestに対応する製品版")
    args = parser.parse_args()
    try:
        result = write_product_package(
            args.source_dir, args.output, product_version=args.product_version
        )
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"D4 Pocket Linux package作成を停止した: {error}", file=sys.stderr)
        return 1
    print(json.dumps(result, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
