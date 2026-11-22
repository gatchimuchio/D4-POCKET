from __future__ import annotations

import hashlib
import json
from pathlib import Path
import struct
import tempfile
import unittest

from tooling.tests.linux_we03_export_runtime import (
    APP_ID_PATTERN,
    AUDIT_ID_PATTERN,
    SOURCE_APP_ID,
    SOURCE_AUDIT_ID,
    TEST_UPDATE_PUBLIC_KEY_DER_HEX,
    TEST_UPDATE_SOURCE_URL,
    _copy_ui_manifest,
    _last_smoke_result,
    validate_developer_update_trust,
    validate_builder_output,
    validate_manifest_bytes,
)


class LinuxWe03ExportRuntimeTests(unittest.TestCase):
    def setUp(self) -> None:
        self.app_id = "d4-pocket-app-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        self.audit_id = "audit-store-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        self.document = {
            "version": 1,
            "product": "D4 Pocket",
            "manifest": {
                "app_identity": {
                    "app_id": self.app_id,
                    "target_platform": "linux",
                },
                "audit_store": {"store_id": self.audit_id, "inherited": False},
                "inheritance_policy": {
                    "credential": "none",
                    "permission": "none",
                    "approval": "none",
                    "audit": "none",
                },
                "distribution_metadata": {"signed": False},
            },
        }

    def manifest_bytes(self) -> bytes:
        return json.dumps(self.document, separators=(",", ":")).encode("utf-8")

    def test_manifest_accepts_new_independent_identity(self) -> None:
        identity = validate_manifest_bytes(self.manifest_bytes())
        self.assertEqual(identity["app_id"], self.app_id)
        self.assertEqual(identity["audit_store_id"], self.audit_id)
        self.assertEqual(
            identity["manifest_sha256"], hashlib.sha256(self.manifest_bytes()).hexdigest()
        )

    def test_manifest_rejects_source_identity_reuse(self) -> None:
        self.document["manifest"]["app_identity"]["app_id"] = SOURCE_APP_ID
        with self.assertRaises(ValueError):
            validate_manifest_bytes(self.manifest_bytes())
        self.document["manifest"]["app_identity"]["app_id"] = self.app_id
        self.document["manifest"]["audit_store"]["store_id"] = SOURCE_AUDIT_ID
        with self.assertRaises(ValueError):
            validate_manifest_bytes(self.manifest_bytes())

    def test_manifest_rejects_authority_inheritance(self) -> None:
        self.document["manifest"]["audit_store"]["inherited"] = True
        with self.assertRaises(ValueError):
            validate_manifest_bytes(self.manifest_bytes())
        self.document["manifest"]["audit_store"]["inherited"] = False
        self.document["manifest"]["inheritance_policy"]["credential"] = "copy"
        with self.assertRaises(ValueError):
            validate_manifest_bytes(self.manifest_bytes())

    def test_manifest_rejects_signed_or_malformed_identity(self) -> None:
        self.document["manifest"]["distribution_metadata"]["signed"] = True
        with self.assertRaises(ValueError):
            validate_manifest_bytes(self.manifest_bytes())
        self.document["manifest"]["distribution_metadata"]["signed"] = False
        self.document["manifest"]["app_identity"]["app_id"] = "d4-pocket-app-ABC"
        with self.assertRaises(ValueError):
            validate_manifest_bytes(self.manifest_bytes())

    def test_developer_update_trust_is_bound_to_the_ui_manifest(self) -> None:
        public_key = bytes.fromhex(TEST_UPDATE_PUBLIC_KEY_DER_HEX)[12:]
        fingerprint = f"sha256:{hashlib.sha256(public_key).hexdigest()}"
        self.document["update_trust"] = {
            "版": 2,
            "algorithm": "Ed25519",
            "public_key_der_hex": TEST_UPDATE_PUBLIC_KEY_DER_HEX,
            "public_key_fingerprint": fingerprint,
            "package_sources": [
                {"channel": "stable", "base_url": TEST_UPDATE_SOURCE_URL}
            ],
        }
        self.assertEqual(
            validate_developer_update_trust(self.manifest_bytes()), fingerprint
        )

    def test_developer_update_trust_rejects_missing_or_changed_test_identity(self) -> None:
        with self.assertRaises(ValueError):
            validate_developer_update_trust(self.manifest_bytes())
        self.document["update_trust"] = {
            "版": 2,
            "algorithm": "Ed25519",
            "public_key_der_hex": TEST_UPDATE_PUBLIC_KEY_DER_HEX,
            "public_key_fingerprint": "sha256:" + "0" * 64,
            "package_sources": [
                {"channel": "stable", "base_url": TEST_UPDATE_SOURCE_URL}
            ],
        }
        with self.assertRaises(ValueError):
            validate_developer_update_trust(self.manifest_bytes())

    def test_developer_update_trust_requires_the_japanese_schema_version_field(self) -> None:
        public_key = bytes.fromhex(TEST_UPDATE_PUBLIC_KEY_DER_HEX)[12:]
        self.document["update_trust"] = {
            "version": 2,
            "algorithm": "Ed25519",
            "public_key_der_hex": TEST_UPDATE_PUBLIC_KEY_DER_HEX,
            "public_key_fingerprint": f"sha256:{hashlib.sha256(public_key).hexdigest()}",
            "package_sources": [
                {"channel": "stable", "base_url": TEST_UPDATE_SOURCE_URL}
            ],
        }
        with self.assertRaises(ValueError):
            validate_developer_update_trust(self.manifest_bytes())

    def test_identity_patterns_are_strict(self) -> None:
        self.assertIsNotNone(APP_ID_PATTERN.fullmatch(self.app_id))
        self.assertIsNotNone(AUDIT_ID_PATTERN.fullmatch(self.audit_id))
        self.assertIsNone(APP_ID_PATTERN.fullmatch(self.app_id.upper()))
        self.assertIsNone(AUDIT_ID_PATTERN.fullmatch(self.audit_id + "x"))

    def test_builder_output_binds_manifest_and_package(self) -> None:
        manifest_identity = validate_manifest_bytes(self.manifest_bytes())
        package_manifest = {
            "version": 1,
            "product_version": "0.1.0",
            "app_id": self.app_id,
            "audit_store_id": self.audit_id,
            "target_platform": "linux",
            "files": [],
        }
        package_manifest_bytes = json.dumps(package_manifest).encode("utf-8")
        package = b"D4PKG01\n" + struct.pack("<I", len(package_manifest_bytes)) + package_manifest_bytes
        with tempfile.TemporaryDirectory() as directory:
            output_root = Path(directory)
            package_path = output_root / "candidate.d4pkg"
            package_path.write_bytes(package)
            package_evidence = {
                "file_name": package_path.name,
                "format": "D4PKG01",
                "byte_length": len(package),
                "sha256": hashlib.sha256(package).hexdigest(),
                "signed": False,
                "installed_product_verified": False,
            }
            evidence = {
                "evidence_type": "d4_pocket_linux_export_build",
                "evidence_source": "INTERNAL_STATE",
                "target_platform": "linux",
                "build_mode": "release",
                "selection_input_source": "manifest_file_untrusted",
                "source_manifest_sha256": manifest_identity["manifest_sha256"],
                "source_receipt_sha256": None,
                "manifest_copied_byte_for_byte": True,
                "authority_boundary": {
                    "source_authority_verified": False,
                    "owner_authorization_verified": False,
                    "authority_strip": True,
                    "credential_values_explicitly_supplied": False,
                    "credential_inherited": False,
                    "permission_inherited": False,
                    "approval_inherited": False,
                    "audit_chain_inherited": False,
                },
                "compile_time_identity": {
                    "source": "schema_validated_manifest",
                    "app_id": self.app_id,
                    "audit_store_id": self.audit_id,
                    "manifest_sha256": manifest_identity["manifest_sha256"],
                },
                "package": package_evidence,
            }
            result = validate_builder_output(
                evidence,
                output_root=output_root,
                manifest_identity=manifest_identity,
            )
            self.assertEqual(result["package_sha256"], package_evidence["sha256"])
            evidence["source_receipt_sha256"] = "sha256:" + "1" * 64
            with self.assertRaises(RuntimeError):
                validate_builder_output(
                    evidence,
                    output_root=output_root,
                    manifest_identity=manifest_identity,
                )

    def test_builder_output_rejects_package_identity_mismatch(self) -> None:
        manifest_identity = validate_manifest_bytes(self.manifest_bytes())
        package_manifest = {
            "app_id": SOURCE_APP_ID,
            "audit_store_id": self.audit_id,
            "target_platform": "linux",
            "files": [],
        }
        package_manifest_bytes = json.dumps(package_manifest).encode("utf-8")
        package = b"D4PKG01\n" + struct.pack("<I", len(package_manifest_bytes)) + package_manifest_bytes
        with tempfile.TemporaryDirectory() as directory:
            output_root = Path(directory)
            path = output_root / "candidate.d4pkg"
            path.write_bytes(package)
            evidence = {
                "evidence_type": "d4_pocket_linux_export_build",
                "evidence_source": "INTERNAL_STATE",
                "target_platform": "linux",
                "build_mode": "release",
                "selection_input_source": "manifest_file_untrusted",
                "source_manifest_sha256": manifest_identity["manifest_sha256"],
                "source_receipt_sha256": None,
                "manifest_copied_byte_for_byte": True,
                "authority_boundary": {
                    "source_authority_verified": False,
                    "owner_authorization_verified": False,
                    "authority_strip": True,
                    "credential_values_explicitly_supplied": False,
                    "credential_inherited": False,
                    "permission_inherited": False,
                    "approval_inherited": False,
                    "audit_chain_inherited": False,
                },
                "compile_time_identity": {
                    "source": "schema_validated_manifest",
                    "app_id": self.app_id,
                    "audit_store_id": self.audit_id,
                    "manifest_sha256": manifest_identity["manifest_sha256"],
                },
                "package": {
                    "file_name": path.name,
                    "byte_length": len(package),
                    "sha256": hashlib.sha256(package).hexdigest(),
                    "signed": False,
                    "installed_product_verified": False,
                },
            }
            with self.assertRaises(RuntimeError):
                validate_builder_output(
                    evidence,
                    output_root=output_root,
                    manifest_identity=manifest_identity,
                )

    def test_smoke_result_selects_expected_scope(self) -> None:
        text = '{"unrelated":true}\n{"acceptance_scope":"agent_task_normal_release"}\n'
        self.assertEqual(
            _last_smoke_result(text, "agent_task_normal_release")["acceptance_scope"],
            "agent_task_normal_release",
        )
        with self.assertRaises(RuntimeError):
            _last_smoke_result(text, "update_lifecycle")

    def test_ui_manifest_capture_preserves_exact_bytes(self) -> None:
        raw = self.manifest_bytes()
        filename = f"{self.app_id}.json"
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            export_root = root / "export"
            capture_root = root / "capture"
            export_root.mkdir()
            capture_root.mkdir()
            source = export_root / filename
            source.write_bytes(raw)
            source.chmod(0o600)
            expected = {
                "file": filename,
                "sha256": hashlib.sha256(raw).hexdigest(),
            }
            captured = _copy_ui_manifest(export_root, capture_root, expected)
            destination = capture_root / filename
            self.assertEqual(destination.read_bytes(), raw)
            self.assertEqual(captured["manifest_sha256"], expected["sha256"])


if __name__ == "__main__":
    unittest.main()
