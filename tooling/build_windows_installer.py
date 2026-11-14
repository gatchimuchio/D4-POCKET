"""Export Receiptに固定されたWindows D4 Pocket bootstrap Setupを生成する。"""

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
    digest = hashlib.sha256()
    with setup_path.open("rb") as setup:
        for chunk in iter(lambda: setup.read(1024 * 1024), b""):
            digest.update(chunk)
    return {
        "source_commit": source_commit,
        "app_id": app_id,
        "audit_store_id": audit_store_id,
        "product_version": product_version,
        "setup_path": str(setup_path),
        "setup_size_bytes": metadata.st_size,
        "setup_sha256": digest.hexdigest(),
        "authenticode": "not_signed_by_this_build_recipe",
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
