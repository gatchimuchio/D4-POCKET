from __future__ import annotations

import copy
import unittest
from pathlib import Path
from typing import Any
from unittest.mock import patch

from tooling.tests.windows_installed_setup_lifecycle import (
    _EXPECTED_OWNER_DECISIONS,
    _REQUIRED_LIFECYCLE_FLAGS,
    _assert_lifecycle_evidence,
    _cargo_test,
)


SOURCE_COMMIT = "a" * 40
APP_ID = "d4-pocket-app-" + "1" * 32
AUDIT_STORE_ID = "audit-store-" + "2" * 32
INITIAL_PACKAGE_SHA256 = "3" * 64
UPDATE_PACKAGE_SHA256 = "4" * 64
SETUP_LIFECYCLE_SCRIPT = Path(__file__).with_name(
    "windows_installed_setup_lifecycle.ps1"
).read_text(encoding="utf-8")


def valid_lifecycle_evidence() -> dict[str, Any]:
    before = {
        "bytes": 4096,
        "events": 18,
        "sha256": "5" * 64,
        "contains_test_candidate_ids": True,
        "private_test_key_absent": True,
    }
    after = {
        "bytes": 4600,
        "events": 20,
        "sha256": "6" * 64,
        "contains_test_candidate_ids": True,
        "private_test_key_absent": True,
    }
    return {
        "result": "passed",
        "failure_stage": None,
        "source_commit": SOURCE_COMMIT,
        "app_id": APP_ID,
        "audit_store_id": AUDIT_STORE_ID,
        "real_owner_authority": False,
        "evidence_class": "LIVE_RUNTIME",
        "initial_version": "1.2.3",
        "update_version": "1.2.4",
        "initial_package_sha256": INITIAL_PACKAGE_SHA256,
        "update_package_sha256": UPDATE_PACKAGE_SHA256,
        **{flag: True for flag in _REQUIRED_LIFECYCLE_FLAGS},
        "owner_decisions": [
            {
                "operation": operation,
                "decision": decision,
                "test_identity": True,
            }
            for operation, decision in _EXPECTED_OWNER_DECISIONS
        ],
        "audit_before_uninstall": before,
        "audit_after_uninstall": after,
        "audit_event_count_after_uninstall": after["events"],
        "audit_bytes_after_uninstall": after["bytes"],
        "audit_sha256_after_uninstall": after["sha256"],
        "sentinel_sha256": "7" * 64,
        "cleanup_errors": [],
    }


class WindowsInstalledSetupEvidenceTests(unittest.TestCase):
    def assert_valid(self, evidence: dict[str, Any]) -> None:
        _assert_lifecycle_evidence(
            evidence,
            source_commit=SOURCE_COMMIT,
            app_id=APP_ID,
            audit_store_id=AUDIT_STORE_ID,
            initial_version="1.2.3",
            update_version="1.2.4",
            initial_package_sha256=INITIAL_PACKAGE_SHA256,
            update_package_sha256=UPDATE_PACKAGE_SHA256,
        )

    def test_accepts_complete_isolated_lifecycle_and_durable_audit(self) -> None:
        self.assert_valid(valid_lifecycle_evidence())

    def test_rejects_real_owner_claim_from_developer_automation(self) -> None:
        evidence = valid_lifecycle_evidence()
        evidence["real_owner_authority"] = True
        with self.assertRaisesRegex(ValueError, "real Owner authority"):
            self.assert_valid(evidence)

    def test_rejects_missing_lifecycle_step(self) -> None:
        evidence = valid_lifecycle_evidence()
        evidence["rollback"] = False
        with self.assertRaisesRegex(ValueError, "rollback"):
            self.assert_valid(evidence)

    def test_rejects_unexpected_owner_decision_sequence(self) -> None:
        evidence = valid_lifecycle_evidence()
        evidence["owner_decisions"].pop()
        with self.assertRaisesRegex(ValueError, "decision sequence"):
            self.assert_valid(evidence)

    def test_rejects_audit_without_uninstall_event(self) -> None:
        evidence = valid_lifecycle_evidence()
        evidence["audit_after_uninstall"] = copy.deepcopy(evidence["audit_before_uninstall"])
        evidence["audit_event_count_after_uninstall"] = evidence["audit_after_uninstall"]["events"]
        evidence["audit_bytes_after_uninstall"] = evidence["audit_after_uninstall"]["bytes"]
        evidence["audit_sha256_after_uninstall"] = evidence["audit_after_uninstall"]["sha256"]
        with self.assertRaisesRegex(ValueError, "did not append"):
            self.assert_valid(evidence)

    def test_rejects_cleanup_errors(self) -> None:
        evidence = valid_lifecycle_evidence()
        evidence["cleanup_errors"] = ["owned_process_cleanup_error"]
        with self.assertRaisesRegex(ValueError, "cleanup errors"):
            self.assert_valid(evidence)

    def test_cargo_test_exposes_success_output_only_when_requested(self) -> None:
        cargo_output = "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;\nW-E04_WINDOWS_SEED"
        with patch(
            "tooling.tests.windows_installed_setup_lifecycle._run",
            return_value=cargo_output,
        ) as run:
            result = _cargo_test("seed", "seed_test", {})
        self.assertEqual(result, cargo_output)
        default_command = run.call_args.args[0]
        self.assertNotIn("--nocapture", default_command)

        with patch(
            "tooling.tests.windows_installed_setup_lifecycle._run",
            return_value=cargo_output,
        ) as run:
            _cargo_test("seed", "seed_test", {}, show_test_output=True)
        visible_command = run.call_args.args[0]
        self.assertEqual(visible_command[-1], "--nocapture")

    def test_inno_setup_accepts_japanese_mnemonic_button_names(self) -> None:
        for caption in ("次へ(&N) >", "インストール(&I)", "完了(&F)"):
            with self.subTest(caption=caption):
                self.assertIn(caption, SETUP_LIFECYCLE_SCRIPT)

    def test_setup_wizard_checks_all_process_windows_and_reports_bounded_controls(self) -> None:
        setup_window_probe = SETUP_LIFECYCLE_SCRIPT.split(
            "function Get-SetupWindowCandidates {", maxsplit=1
        )[1].split("\nfunction Get-AuditSummary", maxsplit=1)[0]
        self.assertIn("RootElement.FindAll", setup_window_probe)
        self.assertIn("MainWindowHandle", setup_window_probe)
        self.assertNotIn("RootElement.FindFirst", setup_window_probe)
        self.assertIn("setup_ui_diagnostic", setup_window_probe)
        self.assertIn("buttons=", setup_window_probe)
        self.assertIn("Select-Object -First 16", setup_window_probe)


if __name__ == "__main__":
    unittest.main()
