from __future__ import annotations

import hashlib
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from tooling import build_windows_setup_from_export as pipeline


APP_ID = "d4-pocket-app-11111111111111111111111111111111"
AUDIT_ID = "audit-store-22222222222222222222222222222222"
SOURCE_COMMIT = "a" * 40
SETUP_BYTES = b"synthetic unsigned setup candidate"


def _build_evidence(bundle: Path) -> dict[str, object]:
    bundle.mkdir()
    (bundle / "bundle.marker").write_bytes(b"temporary bundle")
    return {
        "source_commit": SOURCE_COMMIT,
        "source_receipt_sha256": "b" * 64,
        "source_manifest_sha256": "c" * 64,
        "export_id": "export-01",
        "target_platform": "windows",
        "portable_bundle_assembled": True,
        "signed": False,
        "formal_distribution_claimed": False,
        "app_id": APP_ID,
        "audit_store_id": AUDIT_ID,
        "module_plan": {
            "catalog_version": 1,
            "selection_mode": "explicit_optional",
            "requested_optional_module_ids": ["shell.history"],
            "unprunable_core_ids": ["core.security_broker"],
            "included_module_ids": [
                "core.security_broker",
                "shell.dashboard",
                "shell.history",
            ],
            "excluded_optional_module_ids": ["shell.trace_inspector"],
        },
        "artifact_tree_sha256": "d" * 64,
        "artifact_files": [
            {"path": relative, "size_bytes": 128, "sha256": "0" * 64}
            for _role, relative in pipeline.build_windows_installer.WINDOWS_PAYLOAD_COMPONENTS
        ],
    }


class WindowsExportSetupPipelineTests(unittest.TestCase):
    def setup_builder(self, bundle: Path, output: Path, *, iscc_path: Path | None = None):
        self.assertTrue((bundle / "bundle.marker").is_file())
        setup = output / "D4Pocket-Setup-1.2.3+4.exe"
        setup.write_bytes(SETUP_BYTES)
        return {
            "source_commit": SOURCE_COMMIT,
            "app_id": APP_ID,
            "audit_store_id": AUDIT_ID,
            "product_version": "1.2.3+4",
            "setup_path": str(setup),
            "setup_size_bytes": len(SETUP_BYTES),
            "setup_sha256": hashlib.sha256(SETUP_BYTES).hexdigest(),
            "authenticode": "not_signed_by_this_build_recipe",
            "setup_architecture": "x86",
            "setup_authenticode_status": "NotSigned",
            "setup_signer_thumbprint": None,
            "payload_components": [
                {
                    "role": role,
                    "path": relative,
                    "size_bytes": 128,
                    "sha256": "0" * 64,
                    "architecture": "x64",
                    "authenticode_status": "NotSigned",
                    "signer_thumbprint": None,
                }
                for role, relative in pipeline.build_windows_installer.WINDOWS_PAYLOAD_COMPONENTS
            ],
            "authenticode_evidence_source": {
                "evidence_class": "EXTERNAL_EVIDENCE",
                "tool": "PowerShell Get-AuthenticodeSignature",
            },
            "product_installed_runtime": "not_exercised",
        }

    def build(self, root: Path):
        receipt = root / "broker-receipt.json"
        manifest = root / "export-manifest.json"
        receipt.write_text("fixture", encoding="utf-8")
        manifest.write_text("fixture", encoding="utf-8")
        output = root / "new-setup-output"
        with (
            patch.object(pipeline.sys, "platform", "win32"),
            patch.object(
                pipeline.export_windows_product,
                "build_portable_bundle",
                side_effect=lambda _receipt, _manifest, bundle: _build_evidence(bundle),
            ) as build_bundle,
            patch.object(
                pipeline.build_windows_installer,
                "build_windows_installer",
                side_effect=self.setup_builder,
            ) as build_setup,
        ):
            result = pipeline.build_setup_from_export(receipt, manifest, output)
        return result, output, build_bundle, build_setup

    def test_broker_export_is_built_and_atomically_published_as_unsigned_setup(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-setup-pipeline-") as temporary:
            result, output, build_bundle, build_setup = self.build(Path(temporary))

            self.assertTrue(output.is_dir())
            setup = output / "D4Pocket-Setup-1.2.3+4.exe"
            evidence_path = output / pipeline.OUTPUT_EVIDENCE_NAME
            self.assertEqual(setup.read_bytes(), SETUP_BYTES)
            evidence = json.loads(evidence_path.read_text(encoding="utf-8"))
            self.assertEqual(evidence["source_commit"], SOURCE_COMMIT)
            self.assertEqual(evidence["app_id"], APP_ID)
            self.assertEqual(evidence["audit_store_id"], AUDIT_ID)
            self.assertEqual(evidence["version"], 2)
            self.assertEqual(evidence["setup_file"]["architecture"], "x86")
            self.assertEqual(evidence["setup_file"]["authenticode_status"], "NotSigned")
            self.assertEqual(
                {component["architecture"] for component in evidence["payload_components"]},
                {"x64"},
            )
            self.assertEqual(
                evidence["authenticode_evidence_source"]["evidence_class"],
                "EXTERNAL_EVIDENCE",
            )
            self.assertEqual(
                evidence["setup_file"]["sha256"],
                hashlib.sha256(SETUP_BYTES).hexdigest(),
            )
            self.assertFalse(evidence["signed"])
            self.assertFalse(evidence["formal_distribution_claimed"])
            self.assertEqual(evidence["product_installed_runtime"], "not_exercised")
            self.assertFalse((output / "portable-bundle").exists())
            self.assertEqual(result["setup_sha256"], evidence["setup_file"]["sha256"])
            self.assertEqual(
                build_bundle.call_args.args[:2],
                (
                    Path(temporary) / "broker-receipt.json",
                    Path(temporary) / "export-manifest.json",
                ),
            )
            build_setup.assert_called_once()

    def test_source_identity_mismatch_keeps_final_output_absent(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-setup-pipeline-") as temporary:
            root = Path(temporary)
            receipt = root / "receipt.json"
            manifest = root / "manifest.json"
            receipt.write_text("fixture", encoding="utf-8")
            manifest.write_text("fixture", encoding="utf-8")
            output = root / "new-setup-output"

            def mismatched_setup(bundle: Path, stage: Path, *, iscc_path: Path | None = None):
                evidence = self.setup_builder(bundle, stage, iscc_path=iscc_path)
                evidence["source_commit"] = "e" * 40
                return evidence

            with (
                patch.object(pipeline.sys, "platform", "win32"),
                patch.object(
                    pipeline.export_windows_product,
                    "build_portable_bundle",
                    side_effect=lambda _receipt, _manifest, bundle: _build_evidence(bundle),
                ),
                patch.object(
                    pipeline.build_windows_installer,
                    "build_windows_installer",
                    side_effect=mismatched_setup,
                ),
            ):
                with self.assertRaisesRegex(ValueError, "identityまたは証拠範囲"):
                    pipeline.build_setup_from_export(receipt, manifest, output)

            self.assertFalse(output.exists())
            self.assertEqual({path.name for path in root.iterdir()}, {"receipt.json", "manifest.json"})

    def test_payload_architecture_mismatch_never_publishes_output(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-setup-pipeline-") as temporary:
            root = Path(temporary)
            receipt = root / "receipt.json"
            manifest = root / "manifest.json"
            receipt.write_text("fixture", encoding="utf-8")
            manifest.write_text("fixture", encoding="utf-8")
            output = root / "new-setup-output"

            def wrong_architecture(bundle: Path, stage: Path, *, iscc_path: Path | None = None):
                evidence = self.setup_builder(bundle, stage, iscc_path=iscc_path)
                evidence["payload_components"][0]["architecture"] = "arm64"
                return evidence

            with (
                patch.object(pipeline.sys, "platform", "win32"),
                patch.object(
                    pipeline.export_windows_product,
                    "build_portable_bundle",
                    side_effect=lambda _receipt, _manifest, bundle: _build_evidence(bundle),
                ),
                patch.object(
                    pipeline.build_windows_installer,
                    "build_windows_installer",
                    side_effect=wrong_architecture,
                ),
            ):
                with self.assertRaisesRegex(ValueError, "role／path／architecture"):
                    pipeline.build_setup_from_export(receipt, manifest, output)

            self.assertFalse(output.exists())
            self.assertEqual({path.name for path in root.iterdir()}, {"receipt.json", "manifest.json"})

    def test_signed_setup_is_not_claimed_as_unsigned_recipe_output(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-setup-pipeline-") as temporary:
            root = Path(temporary)
            receipt = root / "receipt.json"
            manifest = root / "manifest.json"
            receipt.write_text("fixture", encoding="utf-8")
            manifest.write_text("fixture", encoding="utf-8")
            output = root / "new-setup-output"

            def signed_setup(bundle: Path, stage: Path, *, iscc_path: Path | None = None):
                evidence = self.setup_builder(bundle, stage, iscc_path=iscc_path)
                evidence["setup_authenticode_status"] = "Valid"
                evidence["setup_signer_thumbprint"] = "A" * 40
                return evidence

            with (
                patch.object(pipeline.sys, "platform", "win32"),
                patch.object(
                    pipeline.export_windows_product,
                    "build_portable_bundle",
                    side_effect=lambda _receipt, _manifest, bundle: _build_evidence(bundle),
                ),
                patch.object(
                    pipeline.build_windows_installer,
                    "build_windows_installer",
                    side_effect=signed_setup,
                ),
            ):
                with self.assertRaisesRegex(ValueError, "署名しないSetup recipe"):
                    pipeline.build_setup_from_export(receipt, manifest, output)

            self.assertFalse(output.exists())
            self.assertEqual({path.name for path in root.iterdir()}, {"receipt.json", "manifest.json"})

    def test_payload_hash_mismatch_against_export_inventory_never_publishes_output(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-setup-pipeline-") as temporary:
            root = Path(temporary)
            receipt = root / "receipt.json"
            manifest = root / "manifest.json"
            receipt.write_text("fixture", encoding="utf-8")
            manifest.write_text("fixture", encoding="utf-8")
            output = root / "new-setup-output"

            def mismatched_payload_hash(
                bundle: Path, stage: Path, *, iscc_path: Path | None = None
            ):
                evidence = self.setup_builder(bundle, stage, iscc_path=iscc_path)
                evidence["payload_components"][0]["sha256"] = "f" * 64
                return evidence

            with (
                patch.object(pipeline.sys, "platform", "win32"),
                patch.object(
                    pipeline.export_windows_product,
                    "build_portable_bundle",
                    side_effect=lambda _receipt, _manifest, bundle: _build_evidence(bundle),
                ),
                patch.object(
                    pipeline.build_windows_installer,
                    "build_windows_installer",
                    side_effect=mismatched_payload_hash,
                ),
            ):
                with self.assertRaisesRegex(ValueError, "Export artifact inventoryと一致しない"):
                    pipeline.build_setup_from_export(receipt, manifest, output)

            self.assertFalse(output.exists())
            self.assertEqual({path.name for path in root.iterdir()}, {"receipt.json", "manifest.json"})

    def test_builder_hash_mismatch_never_publishes_output(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-setup-pipeline-") as temporary:
            root = Path(temporary)
            receipt = root / "receipt.json"
            manifest = root / "manifest.json"
            receipt.write_text("fixture", encoding="utf-8")
            manifest.write_text("fixture", encoding="utf-8")
            output = root / "new-setup-output"

            def mismatched_hash(bundle: Path, stage: Path, *, iscc_path: Path | None = None):
                evidence = self.setup_builder(bundle, stage, iscc_path=iscc_path)
                evidence["setup_sha256"] = "f" * 64
                return evidence

            with (
                patch.object(pipeline.sys, "platform", "win32"),
                patch.object(
                    pipeline.export_windows_product,
                    "build_portable_bundle",
                    side_effect=lambda _receipt, _manifest, bundle: _build_evidence(bundle),
                ),
                patch.object(
                    pipeline.build_windows_installer,
                    "build_windows_installer",
                    side_effect=mismatched_hash,
                ),
            ):
                with self.assertRaisesRegex(ValueError, "hash／size"):
                    pipeline.build_setup_from_export(receipt, manifest, output)

            self.assertFalse(output.exists())
            self.assertEqual({path.name for path in root.iterdir()}, {"receipt.json", "manifest.json"})

    def test_existing_output_and_non_windows_host_are_rejected_before_build(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-setup-pipeline-") as temporary:
            root = Path(temporary)
            existing = root / "existing"
            existing.mkdir()
            receipt = root / "receipt.json"
            manifest = root / "manifest.json"
            receipt.write_text("fixture", encoding="utf-8")
            manifest.write_text("fixture", encoding="utf-8")

            with patch.object(pipeline.sys, "platform", "win32"):
                with self.assertRaisesRegex(ValueError, "既に存在する"):
                    pipeline.build_setup_from_export(receipt, manifest, existing)

            with patch.object(pipeline.sys, "platform", "linux"):
                with self.assertRaisesRegex(ValueError, "Windows host"):
                    pipeline.build_setup_from_export(receipt, manifest, root / "unused")


if __name__ == "__main__":
    unittest.main()
