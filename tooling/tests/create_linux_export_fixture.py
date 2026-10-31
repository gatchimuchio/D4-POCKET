"""手動Linux build検証用の非権限Broker-shaped Export fixtureを生成する。"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
APP_ID = "d4-pocket-app-11111111111111111111111111111111"
AUDIT_STORE_ID = "audit-store-22222222222222222222222222222222"


def linux_export_pair(
    update_trust_public_key_der: bytes | None = None,
    update_trust_base_url: str = "https://updates.example.invalid/d4/stable",
    *,
    include_host_operations: bool = False,
) -> tuple[bytes, bytes]:
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
    export_id = "export-linux-build-fixture"
    for document in (receipt["export_manifest"], manifest_file["manifest"]):
        document["app_identity"]["app_id"] = APP_ID
        document["app_identity"]["target_platform"] = "linux"
        document["audit_store"]["store_id"] = AUDIT_STORE_ID
        document["distribution_metadata"]["target_platform"] = "linux"
    if include_host_operations:
        module_plan = manifest_file["manifest"].get("module_plan")
        if not isinstance(module_plan, dict):
            raise ValueError("Linux Export fixtureのModulePlanがありません")
        module_id = "shell.host_operations"
        for field in ("requested_optional_module_ids", "included_module_ids"):
            values = module_plan.get(field)
            if not isinstance(values, list) or any(
                not isinstance(value, str) for value in values
            ):
                raise ValueError(f"Linux Export fixtureの{field}が不正です")
            if module_id not in values:
                values.append(module_id)
        excluded = module_plan.get("excluded_optional_module_ids")
        if not isinstance(excluded, list) or any(
            not isinstance(value, str) for value in excluded
        ):
            raise ValueError(
                "Linux Export fixtureのexcluded_optional_module_idsが不正です"
            )
        module_plan["excluded_optional_module_ids"] = [
            value for value in excluded if value != module_id
        ]
    receipt["export_id"] = export_id
    receipt["export_manifest"] = manifest_file["manifest"]
    manifest_file["export_id"] = export_id
    if update_trust_public_key_der is not None:
        ed25519_spki_prefix = bytes.fromhex("302a300506032b6570032100")
        if (
            len(update_trust_public_key_der) != len(ed25519_spki_prefix) + 32
            or not update_trust_public_key_der.startswith(ed25519_spki_prefix)
        ):
            raise ValueError("更新信頼用公開鍵はEd25519形式のSPKI-DERである必要があります")
        manifest_file["update_trust"] = {
            "版": 2,
            "algorithm": "Ed25519",
            "public_key_der_hex": update_trust_public_key_der.hex(),
            "public_key_fingerprint": "sha256:"
            + hashlib.sha256(update_trust_public_key_der[-32:]).hexdigest(),
            "package_sources": [
                {
                    "channel": "stable",
                    "base_url": update_trust_base_url,
                }
            ],
        }
    manifest_raw = json.dumps(
        manifest_file, ensure_ascii=False, separators=(",", ":")
    ).encode("utf-8")
    receipt["manifest_file"]["file_name"] = f"{APP_ID}.json"
    receipt["manifest_file"]["path"] = f"/tmp/{APP_ID}.json"
    receipt["manifest_file"]["byte_length"] = len(manifest_raw)
    receipt["manifest_file"]["sha256"] = "sha256:" + hashlib.sha256(manifest_raw).hexdigest()
    receipt_raw = json.dumps(
        receipt, ensure_ascii=False, separators=(",", ":")
    ).encode("utf-8")
    return receipt_raw, manifest_raw


def write_fixture(
    output_dir: Path,
    update_trust_public_key_der: bytes | None = None,
    update_trust_base_url: str = "https://updates.example.invalid/d4/stable",
    *,
    include_host_operations: bool = False,
) -> tuple[Path, Path]:
    output_dir.mkdir(mode=0o700, parents=False, exist_ok=False)
    receipt_raw, manifest_raw = linux_export_pair(
        update_trust_public_key_der,
        update_trust_base_url,
        include_host_operations=include_host_operations,
    )
    receipt = output_dir / "d4_pocket_export_receipt.json"
    manifest = output_dir / f"{APP_ID}.json"
    with receipt.open("xb") as target:
        target.write(receipt_raw)
    with manifest.open("xb") as target:
        target.write(manifest_raw)
    return receipt, manifest


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Owner authorityを含まない合成Linux Export入力fixtureを作る"
    )
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--update-trust-public-key-der", type=Path)
    parser.add_argument("--update-trust-base-url")
    parser.add_argument("--include-host-operations", action="store_true")
    args = parser.parse_args()
    update_trust_public_key_der = (
        args.update_trust_public_key_der.read_bytes()
        if args.update_trust_public_key_der is not None
        else None
    )
    receipt, manifest = write_fixture(
        args.output_dir,
        update_trust_public_key_der,
        args.update_trust_base_url or "https://updates.example.invalid/d4/stable",
        include_host_operations=args.include_host_operations,
    )
    print(
        json.dumps(
            {
                "fixture_only": True,
                "owner_authorization_verified": False,
                "receipt": str(receipt),
                "manifest": str(manifest),
            },
            separators=(",", ":"),
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
