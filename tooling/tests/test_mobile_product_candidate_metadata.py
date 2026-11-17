from __future__ import annotations

import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from tooling.mobile_product_candidate_metadata import (
    DEVICE_LINK_SCHEMA_FILES,
    device_link_schema_evidence,
)


class MobileProductCandidateMetadataTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory(prefix="d4p-mobile-schema-evidence-")
        self.root = Path(self.temp.name)
        for relative in DEVICE_LINK_SCHEMA_FILES:
            path = self.root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text('{"title":"fixture"}\n', encoding="utf-8")

    def tearDown(self) -> None:
        self.temp.cleanup()

    def test_hashes_all_device_link_schemas_and_canonical_manifest(self) -> None:
        evidence = device_link_schema_evidence(self.root)
        expected_files = {
            Path(relative).name: hashlib.sha256(
                (self.root / relative).read_bytes()
            ).hexdigest()
            for relative in DEVICE_LINK_SCHEMA_FILES
        }
        canonical = json.dumps(
            expected_files, sort_keys=True, separators=(",", ":")
        ).encode("ascii")
        self.assertEqual(evidence["files"], expected_files)
        self.assertEqual(evidence["sha256"], hashlib.sha256(canonical).hexdigest())

    def test_fails_closed_when_a_required_device_link_schema_is_missing(self) -> None:
        (self.root / DEVICE_LINK_SCHEMA_FILES[0]).unlink()
        with self.assertRaisesRegex(ValueError, "確認できない"):
            device_link_schema_evidence(self.root)


if __name__ == "__main__":
    unittest.main()
