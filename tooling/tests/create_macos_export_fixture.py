"""手動macOS Export build用の権限を持たない合成Receipt／Manifestを作る。"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
APP_ID = "d4-pocket-app-33333333333333333333333333333333"
AUDIT_STORE_ID = "audit-store-44444444444444444444444444444444"


def macos_export_pair() -> tuple[bytes, bytes]:
    receipt: dict[str, Any] = json.loads(
        (ROOT / "examples/contracts/gui_shell_export_receipt.valid.json").read_text(
            encoding="utf-8"
        )
    )
    manifest_file: dict[str, Any] = json.loads(
        (ROOT / "examples/contracts/gui_shell_export_manifest.valid.json").read_text(
            encoding="utf-8"
        )
    )
    export_id = "export-macos-build-fixture"
    manifest = manifest_file["manifest"]
    for document in (receipt["export_manifest"], manifest):
        document["app_identity"]["app_id"] = APP_ID
        document["app_identity"]["target_platform"] = "macos"
        document["audit_store"]["store_id"] = AUDIT_STORE_ID
        document["distribution_metadata"]["target_platform"] = "macos"
    receipt["export_id"] = export_id
    receipt["export_manifest"] = manifest
    manifest_file["export_id"] = export_id
    manifest_raw = json.dumps(
        manifest_file, ensure_ascii=False, separators=(",", ":")
    ).encode("utf-8")
    receipt["manifest_file"].update(
        {
            "file_name": f"{APP_ID}.json",
            "path": f"/tmp/{APP_ID}.json",
            "byte_length": len(manifest_raw),
            "sha256": "sha256:" + hashlib.sha256(manifest_raw).hexdigest(),
        }
    )
    receipt_raw = json.dumps(
        receipt, ensure_ascii=False, separators=(",", ":")
    ).encode("utf-8")
    return receipt_raw, manifest_raw


def write_fixture(output_dir: Path) -> tuple[Path, Path]:
    output_dir.mkdir(mode=0o700, parents=False, exist_ok=False)
    receipt_raw, manifest_raw = macos_export_pair()
    receipt_path = output_dir / "d4_pocket_export_receipt.json"
    manifest_path = output_dir / f"{APP_ID}.json"
    with receipt_path.open("xb") as target:
        target.write(receipt_raw)
    with manifest_path.open("xb") as target:
        target.write(manifest_raw)
    return receipt_path, manifest_path


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Owner authorityなしのmacOS Export検証fixtureを生成する"
    )
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()
    receipt_path, manifest_path = write_fixture(args.output_dir)
    print(
        json.dumps(
            {
                "fixture_only": True,
                "owner_authorization_verified": False,
                "receipt": str(receipt_path),
                "manifest": str(manifest_path),
            },
            separators=(",", ":"),
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
