from __future__ import annotations

import hashlib
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from tooling.export_windows_product import validate_export_manifest_only_input
from tooling.tests.create_linux_export_fixture import (
    APP_ID,
    AUDIT_STORE_ID,
    linux_export_pair,
    write_fixture,
)


class LinuxExportFixtureIdentityTests(unittest.TestCase):
    def test_default_fixture_preserves_existing_identity_and_receipt(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            receipt_path, manifest_path = write_fixture(Path(temporary) / "fixture")
            self.assertIsNotNone(receipt_path)
            assert receipt_path is not None
            manifest_file = json.loads(manifest_path.read_text(encoding="utf-8"))
            receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
            self.assertEqual(
                manifest_file["manifest"]["app_identity"]["app_id"], APP_ID
            )
            self.assertEqual(
                manifest_file["manifest"]["audit_store"]["store_id"],
                AUDIT_STORE_ID,
            )
            self.assertEqual(receipt["manifest_file"]["file_name"], manifest_path.name)

    def test_manifest_only_fixture_has_new_identity_and_no_receipt(self) -> None:
        app_id = "d4-pocket-app-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        audit_store_id = "audit-store-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        with tempfile.TemporaryDirectory() as temporary:
            receipt_path, manifest_path = write_fixture(
                Path(temporary) / "fixture",
                app_id=app_id,
                audit_store_id=audit_store_id,
                manifest_only=True,
            )
            self.assertIsNone(receipt_path)
            self.assertEqual(
                sorted(path.name for path in manifest_path.parent.iterdir()),
                [manifest_path.name],
            )
            manifest_file = json.loads(manifest_path.read_text(encoding="utf-8"))
            self.assertEqual(
                manifest_file["manifest"]["app_identity"]["app_id"], app_id
            )
            self.assertEqual(
                manifest_file["manifest"]["audit_store"]["store_id"],
                audit_store_id,
            )
            self.assertFalse(manifest_file.get("authority_verified", False))
            accepted_manifest, _, digest = validate_export_manifest_only_input(
                manifest_path.read_bytes(),
                manifest_path.name,
                target_platform="linux",
            )
            self.assertEqual(
                accepted_manifest["manifest"]["app_identity"]["app_id"], app_id
            )
            self.assertEqual(digest, hashlib.sha256(manifest_path.read_bytes()).hexdigest())

    def test_manifest_only_fixture_rejects_default_d4_identity_before_output(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            output_dir = Path(temporary) / "fixture"
            with self.assertRaisesRegex(ValueError, "別のApp/Audit Store identity"):
                write_fixture(output_dir, manifest_only=True)
            self.assertFalse(output_dir.exists())

    def test_cli_emits_manifest_only_for_the_requested_identity(self) -> None:
        app_id = "d4-pocket-app-eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
        audit_store_id = "audit-store-ffffffffffffffffffffffffffffffff"
        with tempfile.TemporaryDirectory() as temporary:
            output_dir = Path(temporary) / "fixture"
            completed = subprocess.run(
                [
                    sys.executable,
                    "-B",
                    str(Path(__file__).with_name("create_linux_export_fixture.py")),
                    "--output-dir",
                    str(output_dir),
                    "--app-id",
                    app_id,
                    "--audit-store-id",
                    audit_store_id,
                    "--manifest-only",
                ],
                check=True,
                capture_output=True,
                text=True,
                encoding="utf-8",
            )
            report = json.loads(completed.stdout)
            self.assertTrue(report["fixture_only"])
            self.assertFalse(report["owner_authorization_verified"])
            self.assertTrue(report["manifest_only"])
            self.assertIsNone(report["receipt"])
            self.assertEqual(
                sorted(path.name for path in output_dir.iterdir()),
                [f"{app_id}.json"],
            )

    def test_manifest_only_input_pair_is_still_broker_shaped_but_non_authority(self) -> None:
        app_id = "d4-pocket-app-cccccccccccccccccccccccccccccccc"
        audit_store_id = "audit-store-dddddddddddddddddddddddddddddddd"
        receipt_raw, manifest_raw = linux_export_pair(
            app_id=app_id, audit_store_id=audit_store_id
        )
        manifest_file = json.loads(manifest_raw)
        receipt = json.loads(receipt_raw)
        self.assertEqual(receipt["manifest_file"]["file_name"], f"{app_id}.json")
        self.assertFalse(receipt.get("authority_verified", False))
        self.assertEqual(
            manifest_file["manifest"]["audit_store"]["inherited"], False
        )
        self.assertEqual(
            manifest_file["manifest"]["inheritance_policy"],
            {
                "authority": "none",
                "permission": "none",
                "approval": "none",
                "credential": "none",
                "audit_chain": "none",
            },
        )

    def test_identity_fields_reject_path_and_case_variants(self) -> None:
        for app_id, audit_store_id in (
            ("../d4-pocket-app-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", AUDIT_STORE_ID),
            ("d4-pocket-app-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", AUDIT_STORE_ID),
            (APP_ID, "../audit-store-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
        ):
            with self.subTest(app_id=app_id, audit_store_id=audit_store_id):
                with self.assertRaises(ValueError):
                    linux_export_pair(
                        app_id=app_id, audit_store_id=audit_store_id
                    )


if __name__ == "__main__":
    unittest.main()
