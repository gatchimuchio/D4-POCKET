"""現行v11五文書の構造・業務分離と採用同一性。製品QAは行わない。"""
from __future__ import annotations

import copy
import json
import shutil
import tempfile
import unittest
from pathlib import Path

from tooling.current_governance_check import (
    ROOT, INDEX_PATH, DIRECTORY, EXPECTED_RECORDS, Invalid,
    governance_errors, read_nlc, validate_population,
)


class CurrentV11StructureTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.root = ROOT
        cls.index = json.loads((ROOT / INDEX_PATH).read_text(encoding="utf-8"))
        cls.entries = cls.index["現行施工採用"]["五文書"]
        cls.docs = {entry["役割"]: read_nlc((ROOT / entry["path"]).read_bytes())
                    for entry in cls.entries}

    def test_real_adoption_is_valid(self):
        self.assertEqual([], governance_errors(self.root))

    def test_document_counts(self):
        self.assertEqual(list(EXPECTED_RECORDS), [len(doc) for doc in self.docs.values()])
        validate_population(self.docs)

    def test_implementation_and_quality_are_disjoint(self):
        instruction = self.docs["実装開発実行指示"]
        build = {key.removeprefix("要求施工/") for key in instruction
                 if key.startswith("要求施工/")}
        qa = {key.removeprefix("品質所管分離/") for key in instruction
              if key.startswith("品質所管分離/")}
        self.assertEqual(152, len(build))
        self.assertEqual(36, len(qa))
        self.assertFalse(build & qa)
        self.assertFalse(any(k.startswith("品質検査/") for k in instruction))

    def test_reject_bom(self):
        raw = (self.root / self.entries[0]["path"]).read_bytes()
        with self.assertRaises(Invalid):
            read_nlc(b"\xef\xbb\xbf" + raw)

    def test_reject_crlf(self):
        raw = (self.root / self.entries[0]["path"]).read_bytes()
        with self.assertRaises(Invalid):
            read_nlc(raw.replace(b"\n", b"\r\n"))

    def test_reject_missing_final_lf(self):
        raw = (self.root / self.entries[0]["path"]).read_bytes()
        with self.assertRaises(Invalid):
            read_nlc(raw[:-1])

    def test_reject_duplicate_record(self):
        raw = (self.root / self.entries[0]["path"]).read_bytes()
        with self.assertRaises(Invalid):
            read_nlc(raw + raw.splitlines()[0] + b"\n")

    def test_reject_duplicate_key(self):
        with self.assertRaises(Invalid):
            read_nlc(b'{"識別":"x","識別":"y","種類":"要求"}\n')

    def test_reject_broken_internal_reference(self):
        with self.assertRaises(Invalid):
            read_nlc(b'{"識別":"x","種類":"要求","意味参照":["missing"]}\n')


class CurrentV11BindingTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.original = json.loads((ROOT / INDEX_PATH).read_text(encoding="utf-8"))
        cls.paths = ["README.md", "AGENTS.md", INDEX_PATH,
                     cls.original["現行施工採用"]["採用記録"],
                     *(entry["path"] for entry in cls.original["現行施工採用"]["五文書"])]
        cls.bytes = {p: (ROOT / p).read_bytes() for p in cls.paths}

    def setUp(self):
        tmp = tempfile.TemporaryDirectory(prefix="d4p-v11-governance-")
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        for relative, raw in self.bytes.items():
            path = self.root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(raw)
        self.assertEqual([], governance_errors(self.root))

    def write_index(self, data):
        (self.root / INDEX_PATH).write_text(
            json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

    def test_hash_change_fails(self):
        index = copy.deepcopy(self.original)
        index["現行施工採用"]["五文書"][0]["SHA256"] = "0" * 64
        self.write_index(index)
        self.assertTrue(governance_errors(self.root))

    def test_missing_document_fails(self):
        target = self.root / self.original["現行施工採用"]["五文書"][0]["path"]
        target.unlink()
        self.assertTrue(governance_errors(self.root))

    def test_missing_adoption_fails(self):
        index = copy.deepcopy(self.original)
        index["現行施工採用"]["採用記録"] = f"{DIRECTORY}/none.md"
        self.write_index(index)
        self.assertTrue(governance_errors(self.root))

    def test_missing_agents_reference_fails(self):
        p = self.root / "AGENTS.md"
        original = p.read_text(encoding="utf-8")
        target = self.original["現行施工採用"]["五文書"][4]["path"]
        p.write_text(original.replace(target, "removed"), encoding="utf-8")
        self.assertTrue(governance_errors(self.root))

    def test_missing_quality_separation_fails(self):
        index = copy.deepcopy(self.original)
        index["現行品質採用"]["工程表"] = "missing"
        self.write_index(index)
        self.assertTrue(governance_errors(self.root))

    def test_quality_cases_are_only_quality(self):
        source = self.root / self.original["現行施工採用"]["五文書"][3]["path"]
        data = source.read_bytes()
        last = data.splitlines()[-1]
        source.write_bytes(data + last + b"\n")
        self.assertTrue(governance_errors(self.root))


if __name__ == "__main__":
    unittest.main()
