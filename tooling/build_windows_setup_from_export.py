"""Broker Export Receiptから未署名Windows Setup候補までを一括生成する開発用tool。"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import stat
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from tooling import build_windows_installer, export_windows_product  # noqa: E402

OUTPUT_EVIDENCE_NAME = "d4_pocket_windows_setup_export.json"
SETUP_EVIDENCE_TYPE = "d4_pocket_windows_setup_export"
SETUP_EVIDENCE_VERSION = 2


def _validate_installer_artifact_evidence(
    setup_evidence: dict[str, Any], export_artifact_files: Any
) -> None:
    setup_architecture = setup_evidence.get("setup_architecture")
    if not isinstance(setup_architecture, str) or setup_architecture not in {
        "x86",
        "x64",
        "arm64",
    }:
        raise ValueError("Setup executableのarchitecture evidenceが不正である")
    status = setup_evidence.get("setup_authenticode_status")
    thumbprint = setup_evidence.get("setup_signer_thumbprint")
    if status != "NotSigned" or thumbprint is not None:
        raise ValueError("署名しないSetup recipeと実Authenticode状態が一致しない")

    expected = dict(build_windows_installer.WINDOWS_PAYLOAD_COMPONENTS)
    if not isinstance(export_artifact_files, list):
        raise ValueError("Export build evidenceにartifact inventoryがない")
    export_records: dict[str, dict[str, Any]] = {}
    for record in export_artifact_files:
        if not isinstance(record, dict):
            raise ValueError("Export artifact inventory recordが不正である")
        path = record.get("path")
        if not isinstance(path, str) or path in export_records:
            raise ValueError("Export artifact inventory pathが不正または重複している")
        export_records[path] = record
    components = setup_evidence.get("payload_components")
    if not isinstance(components, list) or len(components) != len(expected):
        raise ValueError("Windows payload component evidenceの件数が不正である")
    seen: set[str] = set()
    for component in components:
        if not isinstance(component, dict):
            raise ValueError("Windows payload component evidenceがobjectではない")
        role = component.get("role")
        if (
            not isinstance(role, str)
            or role not in expected
            or role in seen
            or component.get("path") != expected[role]
            or component.get("architecture") != "x64"
        ):
            raise ValueError("Windows payload component role／path／architectureが一致しない")
        seen.add(role)
        if type(component.get("size_bytes")) is not int or component["size_bytes"] < 1:
            raise ValueError("Windows payload component size evidenceが不正である")
        if not isinstance(component.get("sha256"), str) or not re.fullmatch(
            r"[0-9a-f]{64}", component["sha256"]
        ):
            raise ValueError("Windows payload component hash evidenceが不正である")
        source_record = export_records.get(expected[role])
        if (
            source_record is None
            or type(source_record.get("size_bytes")) is not int
            or source_record["size_bytes"] < 1
            or source_record.get("size_bytes") != component["size_bytes"]
            or not isinstance(source_record.get("sha256"), str)
            or not re.fullmatch(r"[0-9a-f]{64}", source_record["sha256"])
            or source_record.get("sha256") != component["sha256"]
        ):
            raise ValueError("Windows payload componentがExport artifact inventoryと一致しない")
        component_status = component.get("authenticode_status")
        component_thumbprint = component.get("signer_thumbprint")
        if not isinstance(component_status, str) or component_status not in {
            "Valid",
            "NotSigned",
        }:
            raise ValueError("Windows payload component Authenticode statusが不正である")
        if component_status == "Valid":
            if not isinstance(component_thumbprint, str) or not re.fullmatch(
                r"[0-9A-F]{40,64}", component_thumbprint
            ):
                raise ValueError("Windows payload component signer thumbprintが不正である")
        elif component_thumbprint is not None:
            raise ValueError("未署名payload componentに署名者情報がある")
    if seen != set(expected):
        raise ValueError("Windows payload component evidenceが不足している")

    if setup_evidence.get("authenticode_evidence_source") != {
        "evidence_class": "EXTERNAL_EVIDENCE",
        "tool": "PowerShell Get-AuthenticodeSignature",
    }:
        raise ValueError("Authenticode evidence sourceが明示されていない")


def _output_target(path: Path) -> Path:
    candidate = path.expanduser()
    if not candidate.is_absolute():
        candidate = ROOT / candidate
    if candidate.name in ("", ".", ".."):
        raise ValueError("Setup output directory名が不正である")

    parent_metadata = candidate.parent.lstat()
    if (
        not stat.S_ISDIR(parent_metadata.st_mode)
        or candidate.parent.is_symlink()
        or export_windows_product._is_reparse_point(parent_metadata)
    ):
        raise ValueError("Setup outputの親は通常directoryでなければならない")
    parent = candidate.parent.resolve(strict=True)
    resolved = parent / candidate.name
    if resolved.exists() or resolved.is_symlink():
        raise ValueError("Setup output directoryが既に存在する。上書きしない")
    if resolved.is_relative_to(ROOT.resolve()) or export_windows_product._is_one_drive_path(resolved):
        raise ValueError("Setup outputはRepository／OneDriveの外でなければならない")
    return resolved


def _hash_and_size(path: Path) -> tuple[str, int]:
    metadata = path.lstat()
    if (
        not stat.S_ISREG(metadata.st_mode)
        or path.is_symlink()
        or export_windows_product._is_reparse_point(metadata)
        or metadata.st_size < 1
    ):
        raise ValueError("Setup.exeが通常fileではない")
    digest = hashlib.sha256()
    size = 0
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
            size += len(chunk)
    if size != metadata.st_size:
        raise ValueError("Setup.exeの読取sizeがfile metadataと一致しない")
    return digest.hexdigest(), size


def _publish_setup(source: Path, destination: Path, expected_hash: str) -> tuple[str, int]:
    digest = hashlib.sha256()
    size = 0
    with source.open("rb") as input_file, destination.open("xb") as output_file:
        for chunk in iter(lambda: input_file.read(1024 * 1024), b""):
            output_file.write(chunk)
            digest.update(chunk)
            size += len(chunk)
    published_hash = digest.hexdigest()
    if published_hash != expected_hash or size < 1:
        raise ValueError("公開candidateのSetup.exeが生成元hashと一致しない")
    return published_hash, size


def build_setup_from_export(
    receipt_path: Path,
    manifest_path: Path,
    output_dir: Path,
    *,
    iscc_path: Path | None = None,
) -> dict[str, Any]:
    """Broker Exportを検証し、portable bundleからSetup候補を一時生成して公開する。"""
    if sys.platform != "win32":
        raise ValueError("Windows Setup候補の生成はWindows host上で実行する")
    output = _output_target(output_dir)

    with tempfile.TemporaryDirectory(
        prefix="d4p-setup-export-", dir=output.parent
    ) as temporary:
        temporary_root = Path(temporary)
        bundle = temporary_root / "portable-bundle"
        build_evidence = export_windows_product.build_portable_bundle(
            receipt_path, manifest_path, bundle
        )
        if (
            build_evidence.get("target_platform") != "windows"
            or build_evidence.get("portable_bundle_assembled") is not True
            or build_evidence.get("signed") is not False
            or build_evidence.get("formal_distribution_claimed") is not False
        ):
            raise ValueError("Windows Export build evidenceの対象または証拠範囲が不正である")

        setup_stage = temporary_root / "setup-stage"
        setup_stage.mkdir()
        setup_evidence = build_windows_installer.build_windows_installer(
            bundle, setup_stage, iscc_path=iscc_path
        )
        if (
            setup_evidence.get("source_commit") != build_evidence.get("source_commit")
            or setup_evidence.get("app_id") != build_evidence.get("app_id")
            or setup_evidence.get("audit_store_id") != build_evidence.get("audit_store_id")
            or setup_evidence.get("authenticode") != "not_signed_by_this_build_recipe"
            or setup_evidence.get("product_installed_runtime") != "not_exercised"
        ):
            raise ValueError("Setup evidenceがExport build identityまたは証拠範囲と一致しない")
        _validate_installer_artifact_evidence(
            setup_evidence, build_evidence.get("artifact_files")
        )

        source_setup = Path(setup_evidence["setup_path"])
        product_version = setup_evidence.get("product_version")
        if (
            not isinstance(product_version, str)
            or source_setup.name != f"D4Pocket-Setup-{product_version}.exe"
        ):
            raise ValueError("Setup file名とProduct versionが一致しない")
        source_setup_metadata = source_setup.lstat()
        if (
            not stat.S_ISREG(source_setup_metadata.st_mode)
            or source_setup.is_symlink()
            or export_windows_product._is_reparse_point(source_setup_metadata)
        ):
            raise ValueError("Setup Builder outputが通常fileではない")
        resolved_setup_stage = setup_stage.resolve(strict=True)
        resolved_source_setup = source_setup.resolve(strict=True)
        if (
            not resolved_source_setup.is_relative_to(resolved_setup_stage)
            or resolved_source_setup.parent != resolved_setup_stage
        ):
            raise ValueError("Setup Builder出力が一時Setup directory直下にない")
        source_hash, source_size = _hash_and_size(resolved_source_setup)
        if (
            source_hash != setup_evidence.get("setup_sha256")
            or source_size != setup_evidence.get("setup_size_bytes")
        ):
            raise ValueError("Setup builder evidenceのhash／sizeが出力fileと一致しない")

        published = temporary_root / "published"
        published.mkdir()
        setup_name = source_setup.name
        published_setup = published / setup_name
        final_hash, final_size = _publish_setup(
            resolved_source_setup, published_setup, source_hash
        )
        record = {
            "version": SETUP_EVIDENCE_VERSION,
            "evidence_type": SETUP_EVIDENCE_TYPE,
            "evidence_source": "INTERNAL_STATE",
            "source_commit": build_evidence["source_commit"],
            "source_receipt_sha256": build_evidence["source_receipt_sha256"],
            "source_manifest_sha256": build_evidence["source_manifest_sha256"],
            "export_id": build_evidence["export_id"],
            "app_id": build_evidence["app_id"],
            "audit_store_id": build_evidence["audit_store_id"],
            "target_platform": "windows",
            "product_version": product_version,
            "module_plan": build_evidence["module_plan"],
            "portable_bundle_tree_sha256": build_evidence["artifact_tree_sha256"],
            "setup_file": {
                "name": setup_name,
                "size_bytes": final_size,
                "sha256": final_hash,
                "authenticode": "not_signed_by_this_build_recipe",
                "architecture": setup_evidence["setup_architecture"],
                "authenticode_status": setup_evidence["setup_authenticode_status"],
                "signer_thumbprint": setup_evidence["setup_signer_thumbprint"],
            },
            "payload_components": setup_evidence["payload_components"],
            "authenticode_evidence_source": setup_evidence["authenticode_evidence_source"],
            "signed": False,
            "formal_distribution_claimed": False,
            "product_installed_runtime": "not_exercised",
            "portable_bundle_preserved": False,
        }
        schema_errors = export_windows_product.validate_instance(
            record,
            export_windows_product._schema(
                "gui_shell_windows_setup_export.schema.json"
            ),
        )
        if schema_errors:
            raise ValueError("Windows Setup Export evidenceが現行Schemaに適合しない")
        evidence_path = published / OUTPUT_EVIDENCE_NAME
        with evidence_path.open("xb") as evidence_file:
            evidence_file.write(
                (json.dumps(record, ensure_ascii=False, indent=2) + "\n").encode("utf-8")
            )
            evidence_file.flush()

        # Same-volume rename publishes both files atomically and refuses an existing target.
        os.rename(published, output)

    return {
        "source_commit": record["source_commit"],
        "app_id": record["app_id"],
        "audit_store_id": record["audit_store_id"],
        "product_version": record["product_version"],
        "setup_path": str(output / setup_name),
        "setup_size_bytes": final_size,
        "setup_sha256": final_hash,
        "evidence_path": str(output / OUTPUT_EVIDENCE_NAME),
        "signed": False,
        "formal_distribution_claimed": False,
        "product_installed_runtime": "not_exercised",
    }


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Broker Export Receiptから未署名Windows Setup候補まで一括生成する開発用tool。"
    )
    parser.add_argument("--receipt", type=Path, required=True, help="Broker Export Receipt JSON")
    parser.add_argument("--manifest-file", type=Path, required=True, help="Receiptに束縛されたManifest file")
    parser.add_argument(
        "--output-dir",
        type=Path,
        required=True,
        help="Repository／OneDrive外の未作成output directory",
    )
    parser.add_argument("--iscc", type=Path, help="Inno Setup 6 ISCC.exe")
    args = parser.parse_args()
    try:
        result = build_setup_from_export(
            args.receipt, args.manifest_file, args.output_dir, iscc_path=args.iscc
        )
    except (
        OSError,
        ValueError,
        RuntimeError,
        subprocess.CalledProcessError,
        tarfile.TarError,
        json.JSONDecodeError,
    ) as error:
        print(f"Windows Setup候補の生成を停止した: {error}", file=sys.stderr)
        return 1
    print(json.dumps(result, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
