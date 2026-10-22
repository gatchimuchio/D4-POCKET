from __future__ import annotations

import contextlib
import io
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from tooling import final_development_audit as audit


class 開発監査参照試験(unittest.TestCase):
    def setUp(self) -> None:
        temporary = tempfile.TemporaryDirectory(prefix="d4p-audit-reference-")
        self.addCleanup(temporary.cleanup)
        self.base = Path(temporary.name)
        self.root = self.base / "repository"
        self.reference = self.root / "docs" / "参照.md"
        self.reference.parent.mkdir(parents=True)
        self.reference.write_text("試験用参照\n", encoding="utf-8")

    def test_相対参照の両区切りを現在OSの実体へ解決する(self) -> None:
        with patch.object(audit, "ROOT", self.root):
            for reference in ("docs/参照.md", "docs\\参照.md"):
                with self.subTest(reference=reference):
                    self.assertEqual(
                        audit._relative_path(reference), self.reference.resolve()
                    )
                    errors: list[str] = []
                    audit._check_reference_paths([reference], errors, "試験")
                    self.assertEqual(errors, [])

    def test_不正参照と欠落を報告して後続参照を検査する(self) -> None:
        invalid = [
            "",
            "   ",
            ".",
            "..",
            "../outside.md",
            "docs/../outside.md",
            "docs\\..\\outside.md",
            "/etc/passwd",
            "\\Windows\\system.ini",
            "C:\\outside.md",
            "C:outside.md",
            "docs/C:outside.md",
            "\\\\server\\share\\outside.md",
            "docs/invalid\0.md",
        ]
        with patch.object(audit, "ROOT", self.root):
            for reference in invalid:
                with self.subTest(reference=reference):
                    with self.assertRaises(ValueError):
                        audit._relative_path(reference)
            errors: list[str] = []
            audit._check_reference_paths(
                [None, *invalid, "docs/missing.md", "docs/参照.md"],
                errors,
                "試験",
            )
        self.assertEqual(len(errors), len(invalid) + 2)
        self.assertIn("参照pathが存在しない: docs/missing.md", errors[-1])

    def test_root外symlinkを拒否し内部symlinkは実体を検査する(self) -> None:
        outside = self.base / "outside.md"
        outside.write_text("root外の試験用参照\n", encoding="utf-8")
        (self.root / "outside-link.md").symlink_to(outside)
        (self.root / "inside-link.md").symlink_to(self.reference)
        with patch.object(audit, "ROOT", self.root):
            with self.assertRaises(ValueError):
                audit._relative_path("outside-link.md")
            errors: list[str] = []
            audit._check_reference_paths(
                ["outside-link.md", "inside-link.md"], errors, "試験"
            )
        self.assertEqual(len(errors), 1)
        self.assertIn("outside-link.md", errors[0])

    def test_解決不能な参照も監査エラーとして返す(self) -> None:
        errors: list[str] = []
        with patch.object(
            audit, "_relative_path", side_effect=OSError("参照を解決できない")
        ):
            audit._check_reference_paths(["docs/参照.md"], errors, "試験")
        self.assertEqual(len(errors), 1)
        self.assertIn("docs/参照.md", errors[0])

    def test_現行対応表を検査しても出荷可能へ昇格しない(self) -> None:
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            exit_code = audit.main()
        report = json.loads(output.getvalue())
        self.assertEqual(exit_code, 0)
        self.assertEqual(report["状態"], "passed")
        self.assertEqual(report["工程数"], 32)
        self.assertEqual(report["未監査工程"], ["C32", "C33", "C34"])
        self.assertEqual(report["エラー"], [])
        self.assertIs(report["release_ready"], False)
        self.assertIs(report["release_blocker"], True)

    def test_不正参照の行は異常終了せず失敗JSONと終了値1を返す(self) -> None:
        data = audit._load_json(audit.AUDIT_PATH)
        receipt = audit._load_json(audit.RECEIPT_PATH)
        data["工程"][0]["参照"].extend(
            ["../outside.md", "docs/f01-missing-reference.md", None]
        )
        output = io.StringIO()
        with (
            patch.object(audit, "_load_json", side_effect=[data, receipt]),
            contextlib.redirect_stdout(output),
        ):
            exit_code = audit.main()
        report = json.loads(output.getvalue())
        self.assertEqual(exit_code, 1)
        self.assertEqual(report["状態"], "failed")
        self.assertEqual(report["工程数"], 32)
        self.assertEqual(len(report["エラー"]), 3)
        self.assertTrue(all(error.startswith("C0の") for error in report["エラー"]))
        self.assertIs(report["release_ready"], False)


if __name__ == "__main__":
    unittest.main()
