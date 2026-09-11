"""Windowsの実collectorが無関係な証拠を保護成立へ昇格しない回帰試験。"""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


@unittest.skipUnless(os.name == "nt", "WindowsのACLとDPAPIを使う実collector試験")
class WindowsAnchorCollectorTests(unittest.TestCase):
    def test_unprotected_store_and_unrelated_files_fail_closed(self):
        shell = shutil.which("pwsh") or shutil.which("powershell")
        self.assertIsNotNone(shell)
        with tempfile.TemporaryDirectory(prefix="gui-shell-anchor-test-") as directory:
            root = Path(directory)
            store = root / "store"
            store.mkdir()
            files = [store / name for name in ("audit_anchor.key", "audit_anchor.json", "audit.jsonl")]
            for path in files:
                path.write_bytes(b"unprotected-test-content")
                with path.open("r+b"):
                    pass
            unrelated = root / "unrelated.txt"
            unrelated.write_text("not an audit anchor", encoding="utf-8")
            cases = [[], ["-ExternalAnchorPath", str(unrelated)], ["-SignedEvidencePath", shell]]
            for index, extra in enumerate(cases):
                with self.subTest(extra=extra):
                    output = root / f"result-{index}.json"
                    result = subprocess.run(
                        [shell, "-NoProfile", "-File", str(ROOT / "installer/windows/collect_audit_anchor_proof.ps1"),
                         "-InstalledRoot", str(root), "-AuditDir", str(store), "-OutputPath", str(output), *extra],
                        capture_output=True, timeout=45,
                    )
                    self.assertEqual(result.returncode, 0, result.stderr.decode(errors="replace"))
                    evidence = json.loads(output.read_text(encoding="utf-8-sig"))
                    self.assertEqual(evidence["status"], "failed")
                    for flag in ("key_anchor_log_same_user_rewrite_mitigated", "dpapi_verified",
                                 "external_anchor_verified", "signed_evidence_verified"):
                        self.assertIs(evidence[flag], False, flag)
                    self.assertIsInstance(evidence["dpapi_available"], bool)
                    self.assertTrue(evidence["errors"])
                    for path in files:
                        self.assertEqual(path.read_bytes(), b"unprotected-test-content")


if __name__ == "__main__":
    unittest.main()
