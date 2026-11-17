from __future__ import annotations

import os
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

from tooling import (
    audit_checkpoint_verifier,
    evidence_bundle,
    full_regression_validation,
    release_gate_check,
    windows_release_evidence,
)


class ReleaseReadinessStatusTests(unittest.TestCase):
    def test_evidence_bundle_requires_v8_readiness_status(self) -> None:
        bundle = {
            "release_ready": False,
            "authority_boundary": {
                "flutter_owns_authority": False,
                "flutter_authority_surface_broker_mediated": True,
                "flutter_spawns_python_for_authority": False,
                "flutter_rust_ffi_authority_bridge": False,
                "installer_grants_authority": False,
            },
            "shell_snapshot": {"trust_records": [{}], "authority_map": [{}]},
            "setup_doctor": {"checks": [{}]},
            "release_smoke": {"ok": True},
            "release_runtime_assertions": {"ok": True},
            "blockers": [
                {
                    "name": "missing_windows_evidence",
                    "status": "failed",
                    "classification": "release_blocker",
                    "blocks_release": True,
                    "readiness_status": "NOT_RUN",
                    "reason": "No evidence file",
                    "required_action": "Run the installed test",
                }
            ],
        }

        self.assertEqual([], evidence_bundle.validate_evidence_bundle(bundle))
        del bundle["blockers"][0]["readiness_status"]
        self.assertTrue(
            any("readiness_status" in error for error in evidence_bundle.validate_evidence_bundle(bundle))
        )
        bundle["blockers"][0]["readiness_status"] = "PASS"
        self.assertTrue(
            any("readiness_status" in error for error in evidence_bundle.validate_evidence_bundle(bundle))
        )

    def test_unavailable_regression_command_is_not_run(self) -> None:
        command = full_regression_validation.ValidationCommand(
            display="missing validation tool",
            command=("missing-validation-tool",),
            cwd=full_regression_validation.ROOT,
            required_tool="codex_missing_validation_tool_794fd3",
            timeout_seconds=1,
        )

        result = full_regression_validation._run(command)

        self.assertEqual("unavailable", result["status"])
        self.assertEqual("NOT_RUN", result["readiness_status"])

    def test_absent_windows_artifact_is_not_run(self) -> None:
        with tempfile.TemporaryDirectory(prefix=".release-readiness-", dir=windows_release_evidence.ROOT) as directory:
            path = Path(directory) / "missing.json"
            results = windows_release_evidence.validate_windows_release_evidence(path)

        self.assertEqual(5, len(results))
        self.assertTrue(all(result.status == "failed" for result in results))
        self.assertTrue(all(result.readiness_status == "NOT_RUN" for result in results))

    def test_invalid_windows_artifact_does_not_become_not_run(self) -> None:
        with tempfile.TemporaryDirectory(prefix=".release-readiness-", dir=windows_release_evidence.ROOT) as directory:
            path = Path(directory) / "invalid.json"
            path.write_text("{", encoding="utf-8")
            results = windows_release_evidence.validate_windows_release_evidence(path)

        self.assertTrue(all(result.status == "failed" for result in results))
        self.assertTrue(all(result.readiness_status == "QA_NOT_ESTABLISHED" for result in results))

    def test_missing_owner_trust_floor_is_blocked_authority(self) -> None:
        with patch.dict(os.environ, {"GUI_SHELL_AUDIT_TRUSTED_HEAD": ""}):
            with self.assertRaises(audit_checkpoint_verifier.CheckpointVerificationFailure) as caught:
                audit_checkpoint_verifier.verify_collected({"checkpoint_inputs": {}})

        self.assertEqual("BLOCKED_AUTHORITY", caught.exception.readiness_status)

    def test_missing_checkpoint_inputs_is_not_run(self) -> None:
        with patch.dict(os.environ, {"GUI_SHELL_AUDIT_TRUSTED_HEAD": "sha256:" + "0" * 64}):
            with self.assertRaises(audit_checkpoint_verifier.CheckpointVerificationFailure) as caught:
                audit_checkpoint_verifier.verify_collected({})

        self.assertEqual("NOT_RUN", caught.exception.readiness_status)

    def test_release_gate_preserves_not_run_for_missing_windows_evidence(self) -> None:
        blocker = {
            "name": "windows_installer_first_run_smoke",
            "status": "unresolved",
            "status_source": "windows_release_evidence",
            "evidence_result": "windows_installer_first_run_smoke",
        }
        result = SimpleNamespace(
            status="failed",
            classification="release_blocker",
            readiness_status="NOT_RUN",
            reason="evidence file missing",
            required_action="run installed smoke",
        )

        effective = release_gate_check.effective_release_blocker(
            blocker, {"windows_installer_first_run_smoke": result}
        )

        self.assertEqual("unresolved", effective["status"])
        self.assertEqual("NOT_RUN", effective["readiness_status"])

    def test_release_gate_classifies_missing_result_status(self) -> None:
        blocker = {
            "name": "windows_installer_first_run_smoke",
            "status": "unresolved",
            "status_source": "windows_release_evidence",
            "evidence_result": "windows_installer_first_run_smoke",
        }
        for result_status, expected in (("unavailable", "NOT_RUN"), ("failed", "QA_NOT_ESTABLISHED")):
            with self.subTest(result_status=result_status):
                result = SimpleNamespace(
                    status=result_status,
                    classification="release_blocker",
                    readiness_status=None,
                    reason="missing classification",
                    required_action="collect evidence",
                )
                effective = release_gate_check.effective_release_blocker(
                    blocker, {"windows_installer_first_run_smoke": result}
                )
                self.assertEqual(expected, effective["readiness_status"])


if __name__ == "__main__":
    unittest.main()
