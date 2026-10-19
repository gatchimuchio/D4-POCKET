from __future__ import annotations

import hashlib
import json
import os
import sys
import tempfile
import unittest
from pathlib import Path

from tooling.package_linux_product import MAGIC, write_product_package
from tooling.schema_check.check_schemas import validate_instance

ROOT = Path(__file__).resolve().parents[2]
APP_ID = "d4-pocket-app-11111111111111111111111111111111"
AUDIT_ID = "audit-store-22222222222222222222222222222222"
PAYLOADS = {
    "data/icudtl.dat": b"icu",
    "data/flutter_assets/AssetManifest.bin": b"assets",
    "gui_shell_desktop": b"runner",
    "gui_shell_desktop_launcher": b"launcher",
    "lib/libflutter_linux_gtk.so": b"flutter",
    "libexec/gui_shell_linux_broker": b"broker",
}


@unittest.skipUnless(sys.platform.startswith("linux"), "Linux POSIX mode semantics are required")
class LinuxProductPackageTests(unittest.TestCase):
    def create_bundle(self, root: Path) -> Path:
        root.mkdir(parents=True, exist_ok=True)
        bundle = root / "bundle"
        bundle.mkdir(mode=0o755)
        os.chmod(bundle, 0o755)
        for relative, payload in PAYLOADS.items():
            target = bundle / relative
            target.parent.mkdir(parents=True, exist_ok=True, mode=0o755)
            os.chmod(target.parent, 0o755)
            target.write_bytes(payload)
            os.chmod(target, 0o755 if relative in {
                "gui_shell_desktop",
                "gui_shell_desktop_launcher",
                "libexec/gui_shell_linux_broker",
            } else 0o644)
        product_manifest = {
            "version": 1,
            "product": "D4 Pocket",
            "export_id": "export-linux-test",
            "manifest": {
                "app_identity": {"app_id": APP_ID, "target_platform": "linux"},
                "audit_store": {
                    "store_id": AUDIT_ID,
                    "chain_status": "new",
                    "inherited": False,
                },
                "distribution_metadata": {"target_platform": "linux"},
                "inheritance_policy": {
                    "authority": "none",
                    "permission": "none",
                    "approval": "none",
                    "credential": "none",
                    "audit_chain": "none",
                },
            },
        }
        manifest_path = bundle / "product_manifest.json"
        manifest_path.write_text(
            json.dumps(product_manifest, separators=(",", ":")), encoding="utf-8"
        )
        os.chmod(manifest_path, 0o644)
        return bundle

    def test_package_bytes_match_linux_schema_hashes_and_fixed_modes(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-linux-package-test-") as temporary:
            root = Path(temporary)
            bundle = self.create_bundle(root)
            output = root / "D4Pocket-linux.d4pkg"

            result = write_product_package(
                bundle, output, product_version="1.2.3"
            )

            package = output.read_bytes()
            self.assertEqual(package[:8], MAGIC)
            manifest_length = int.from_bytes(package[8:12], "little")
            start = 12
            end = start + manifest_length
            manifest = json.loads(package[start:end].decode("utf-8"))
            self.assertEqual(
                validate_instance(
                    manifest,
                    json.loads(
                        (ROOT / "specs/d4_pocket_linux_product_package_manifest.schema.json")
                        .read_text(encoding="utf-8")
                    ),
                ),
                [],
            )
            self.assertEqual(manifest["target_platform"], "linux")
            self.assertEqual(manifest["app_id"], APP_ID)
            self.assertEqual(manifest["audit_store_id"], AUDIT_ID)
            self.assertEqual(
                [item["path"] for item in manifest["files"]],
                sorted(PAYLOADS.keys() | {"product_manifest.json"}, key=str.casefold),
            )

            cursor = end
            for item in manifest["files"]:
                length = item["byte_length"]
                payload = package[cursor : cursor + length]
                cursor += length
                self.assertEqual(len(payload), length)
                self.assertEqual(hashlib.sha256(payload).hexdigest(), item["sha256"])
                relative = item["path"]
                expected_mode = 0o755 if relative in {
                    "gui_shell_desktop",
                    "gui_shell_desktop_launcher",
                    "libexec/gui_shell_linux_broker",
                } else 0o644
                self.assertEqual(item["mode"], expected_mode)
                expected_payload = (
                    (bundle / relative).read_bytes()
                )
                self.assertEqual(payload, expected_payload)
            self.assertEqual(cursor, len(package))
            self.assertEqual(result["package_sha256"], hashlib.sha256(package).hexdigest())
            self.assertEqual(result["signed"], False)
            self.assertEqual(result["installed_product_verified"], False)
            self.assertEqual(
                result["credential_artifact_scan"]["coverage"], "known_patterns_only"
            )

    def test_symlink_and_hardlink_payloads_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-linux-package-link-") as temporary:
            root = Path(temporary)
            bundle = self.create_bundle(root)
            outside = root / "outside"
            outside.write_bytes(b"not a bundle file")
            (bundle / "data/leak").symlink_to(outside)
            with self.assertRaisesRegex(ValueError, "symlink"):
                write_product_package(bundle, root / "symlink.d4pkg", product_version="1.2.3")
            (bundle / "data/leak").unlink()

            original = bundle / "data/icudtl.dat"
            linked = bundle / "data/second.dat"
            os.link(original, linked)
            with self.assertRaisesRegex(ValueError, "単独file"):
                write_product_package(bundle, root / "hardlink.d4pkg", product_version="1.2.3")

    def test_unsafe_modes_missing_files_and_wrong_target_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-linux-package-negative-") as temporary:
            root = Path(temporary)
            bundle = self.create_bundle(root)
            ordinary = bundle / "data/icudtl.dat"
            os.chmod(ordinary, 0o662)
            with self.assertRaisesRegex(ValueError, "permission"):
                write_product_package(bundle, root / "unsafe-mode.d4pkg", product_version="1.2.3")

            os.chmod(ordinary, 0o644)
            (bundle / "libexec/gui_shell_linux_broker").unlink()
            with self.assertRaisesRegex(ValueError, "必須"):
                write_product_package(bundle, root / "missing.d4pkg", product_version="1.2.3")

            bundle = self.create_bundle(root / "wrong-target")
            manifest_path = bundle / "product_manifest.json"
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            manifest["manifest"]["distribution_metadata"]["target_platform"] = "windows"
            manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "Linux"):
                write_product_package(bundle, root / "wrong-target.d4pkg", product_version="1.2.3")

    def test_existing_or_in_bundle_output_is_never_overwritten(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-linux-package-output-") as temporary:
            root = Path(temporary)
            bundle = self.create_bundle(root)
            inside = bundle / "inside.d4pkg"
            with self.assertRaisesRegex(ValueError, "新規"):
                write_product_package(bundle, inside, product_version="1.2.3")

            existing = root / "existing.d4pkg"
            existing.write_bytes(b"preserve")
            with self.assertRaisesRegex(ValueError, "新規"):
                write_product_package(bundle, existing, product_version="1.2.3")
            self.assertEqual(existing.read_bytes(), b"preserve")

    def test_known_credential_patterns_are_rejected_without_echoing_value(self) -> None:
        canary = "AKIA0000000000000000"
        with tempfile.TemporaryDirectory(prefix="d4p-linux-package-secret-") as temporary:
            root = Path(temporary)
            bundle = self.create_bundle(root)
            artifact = bundle / "data/flutter_assets/AssetManifest.bin"
            artifact.write_bytes(canary.encode("ascii"))
            os.chmod(artifact, 0o644)

            with self.assertRaisesRegex(ValueError, "aws_access_key_id") as raised:
                write_product_package(
                    bundle, root / "must-not-exist.d4pkg", product_version="1.2.3"
                )
            self.assertNotIn(canary, str(raised.exception))
            self.assertFalse((root / "must-not-exist.d4pkg").exists())


if __name__ == "__main__":
    unittest.main()
