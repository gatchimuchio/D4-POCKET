from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

from tooling import release_gate_check


def owner_go_blocker() -> dict:
    return {
        "name": "owner_go",
        "status": "unresolved",
        "status_source": "manual",
        "active": True,
        "classification": "release_blocker",
        "cause_category": "owner_decision",
        "release_tracks": ["windows_v1", "mobile"],
        "blocks_release": True,
        "blocks_windows_technical_complete": False,
        "reason": "所有者の明示GOが記録されていません。",
        "required_action": "所有者の明示GOを記録してください。",
    }


class OwnerGoReleaseGateTests(unittest.TestCase):
    def registry_errors_for(self, blocker: dict) -> list[str]:
        with tempfile.TemporaryDirectory() as directory:
            registry_path = Path(directory) / "release_blockers.registry.json"
            registry_path.write_text(json.dumps({"blockers": [blocker]}), encoding="utf-8")
            with patch.object(release_gate_check, "RELEASE_BLOCKERS_REGISTRY", registry_path):
                return release_gate_check.registry_errors()

    def test_manual_owner_go_without_evidence_result_is_valid(self) -> None:
        self.assertEqual([], self.registry_errors_for(owner_go_blocker()))

    def test_unresolved_owner_go_is_reported_as_blocked_authority(self) -> None:
        effective = release_gate_check.effective_release_blocker(owner_go_blocker(), {})
        self.assertEqual("BLOCKED_AUTHORITY", effective["readiness_status"])

    def test_source_runtime_assertion_gap_reaches_release_gate(self) -> None:
        from tooling import release_runtime_assertions

        assertion = release_runtime_assertions._fail(
            "source_gap", "CONFIG", "source contract is absent", "implement the source"
        )
        with patch.object(release_runtime_assertions, "run_release_runtime_assertions", return_value=[assertion]):
            errors = release_gate_check.release_runtime_assertion_errors()
        self.assertEqual(["release runtime assertion [BUILD_GAP] source_gap: source contract is absent"], errors)

    def test_unknown_runtime_assertion_status_is_not_ignored(self) -> None:
        from tooling import release_runtime_assertions

        assertion = release_runtime_assertions.RuntimeAssertion(
            "unknown", "not_available", "CONFIG", "none", "no", "unknown state", "retry"
        )
        with patch.object(release_runtime_assertions, "run_release_runtime_assertions", return_value=[assertion]):
            errors = release_gate_check.release_runtime_assertion_errors()
        self.assertEqual(["release runtime assertion [NOT_RUN] unknown: unknown state"], errors)

    def test_windows_evidence_cannot_resolve_owner_go(self) -> None:
        blocker = owner_go_blocker()
        blocker["status_source"] = "windows_release_evidence"
        blocker["evidence_result"] = "owner_go"

        errors = self.registry_errors_for(blocker)
        self.assertTrue(any("owner_go" in error and "manual" in error for error in errors), errors)

        passed_windows_result = SimpleNamespace(
            status="passed",
            classification="platform_evidence",
            reason="synthetic passing evidence",
            required_action="",
        )
        effective = release_gate_check.effective_release_blocker(
            blocker, {"owner_go": passed_windows_result}
        )
        self.assertEqual("unresolved", effective["status"])
        self.assertEqual("manual", effective["effective_status_source"])
        self.assertIn("Owner本人", effective["reason"])

    def test_manual_owner_go_rejects_automatic_evidence_reference(self) -> None:
        blocker = owner_go_blocker()
        blocker["evidence_result"] = "windows_installed_smoke"

        errors = self.registry_errors_for(blocker)
        self.assertTrue(any("owner_go" in error and "evidence_result" in error for error in errors), errors)


if __name__ == "__main__":
    unittest.main()
