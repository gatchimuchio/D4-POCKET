from __future__ import annotations

import copy
import hashlib
import json
import unittest
from pathlib import Path

from tooling.export_windows_product import (
    BUILD_SCHEMA,
    validate_export_manifest_only_input,
)
from tooling.schema_check.check_schemas import validate_instance


ROOT = Path(__file__).resolve().parents[2]
MANIFEST_FIXTURE = ROOT / "examples/contracts/gui_shell_export_manifest.valid.json"
BUILD_EVIDENCE_FIXTURE = (
    ROOT
    / "docs/QE_2026-11-21/evidence/W-E02-Windows-portable-bundle-b737fe3.json"
)


class ExportManifestOnlyInputTests(unittest.TestCase):
    def setUp(self) -> None:
        self.manifest_raw = MANIFEST_FIXTURE.read_bytes()
        self.manifest = json.loads(self.manifest_raw.decode("utf-8"))
        self.file_name = (
            self.manifest["manifest"]["app_identity"]["app_id"] + ".json"
        )

    def test_broker_manifest_file_is_sufficient_to_resolve_untrusted_build_plan(self) -> None:
        manifest, plan, digest = validate_export_manifest_only_input(
            self.manifest_raw, self.file_name
        )

        self.assertEqual(manifest["export_id"], self.manifest["export_id"])
        self.assertEqual(
            plan.included_module_ids,
            tuple(self.manifest["manifest"]["module_plan"]["included_module_ids"]),
        )
        self.assertEqual(digest, hashlib.sha256(self.manifest_raw).hexdigest())
        self.assertTrue(plan.excluded_optional_ids)

    def test_file_name_target_and_manifest_integrity_are_bound(self) -> None:
        with self.assertRaisesRegex(ValueError, "file名がApp IDと一致"):
            validate_export_manifest_only_input(self.manifest_raw, "other.json")

        with self.assertRaisesRegex(ValueError, "指定Desktop platform"):
            validate_export_manifest_only_input(
                self.manifest_raw, self.file_name, target_platform="macos"
            )

        changed = copy.deepcopy(self.manifest)
        changed["manifest"]["module_plan"]["included_module_ids"].remove(
            "shell.observability"
        )
        with self.assertRaisesRegex(ValueError, "必須Moduleと依存閉包に一致しない"):
            validate_export_manifest_only_input(
                json.dumps(changed).encode("utf-8"), self.file_name
            )

    def test_manifest_only_build_evidence_cannot_claim_receipt_or_authority(self) -> None:
        evidence = json.loads(BUILD_EVIDENCE_FIXTURE.read_text(encoding="utf-8"))
        evidence["selection_input_source"] = "manifest_file_untrusted"
        evidence["source_receipt_sha256"] = None
        self.assertEqual(validate_instance(evidence, _build_schema()), [])

        evidence["source_receipt_sha256"] = "a" * 64
        self.assertTrue(validate_instance(evidence, _build_schema()))
        evidence["source_receipt_sha256"] = None
        evidence["authority_boundary"]["source_authority_verified"] = True
        self.assertTrue(validate_instance(evidence, _build_schema()))

    def test_receipt_and_manifest_evidence_keeps_existing_hash_requirement(self) -> None:
        evidence = json.loads(BUILD_EVIDENCE_FIXTURE.read_text(encoding="utf-8"))
        evidence["selection_input_source"] = "receipt_and_manifest"
        self.assertEqual(validate_instance(evidence, _build_schema()), [])
        evidence["source_receipt_sha256"] = None
        self.assertTrue(validate_instance(evidence, _build_schema()))


def _build_schema() -> dict[str, object]:
    return json.loads(BUILD_SCHEMA.read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()
