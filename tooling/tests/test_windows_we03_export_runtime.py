from __future__ import annotations

import hashlib
import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from tooling.tests.windows_we03_export_runtime import (
    _write_actions_summary,
    create_manifest_only_input,
    validate_task_evidence,
)


class WindowsWe03ExportRuntimeTests(unittest.TestCase):
    def test_manifest_only_input_uses_fresh_windows_identity_and_exact_bytes(self) -> None:
        app_id = "d4-pocket-app-0123456789abcdef0123456789abcdef"
        audit_id = "audit-store-fedcba9876543210fedcba9876543210"
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / f"{app_id}.json"
            raw, digest = create_manifest_only_input(
                path,
                app_id=app_id,
                audit_store_id=audit_id,
            )
            self.assertEqual(path.read_bytes(), raw)
            self.assertEqual(hashlib.sha256(raw).hexdigest(), digest)
            self.assertEqual(path.name, f"{app_id}.json")

    def test_manifest_only_input_rejects_source_identity(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "d4-pocket-app-11111111111111111111111111111111.json"
            with self.assertRaisesRegex(ValueError, "source identity"):
                create_manifest_only_input(
                    path,
                    app_id="d4-pocket-app-11111111111111111111111111111111",
                    audit_store_id="audit-store-fedcba9876543210fedcba9876543210",
                )

    def test_task_evidence_requires_runtime_scope_and_real_owner_boundary(self) -> None:
        evidence = {
            "evidence_type": "d4_pocket_w_e03_windows_export_runtime_developer",
            "evidence_class": "LIVE_RUNTIME",
            "source_commit": "a" * 40,
            "manifest_sha256": "b" * 64,
            "app_id": "d4-pocket-app-0123456789abcdef0123456789abcdef",
            "audit_store_id": "audit-store-fedcba9876543210fedcba9876543210",
            "export_runtime_broker_relative_path": ".developer-verification\\broker\\gui_shell_rust_helper.exe",
            "export_runtime_broker_sha256": "c" * 64,
            "real_owner_authority": False,
            "synthetic_owner_fixture": True,
            "external_provider_requests": 0,
            "task_status": "completed",
            "task_result_hash": "sha256:" + "d" * 64,
            "outside_workspace_write": "denied",
            "secret_value_exposed": False,
            "approval_replay": "rejected",
            "scratch_cleanup": "passed",
            "codex_process_tree_cleanup": "passed",
            "durable_audit_chain": "passed",
            "durable_audit_event_count": 14,
            "durable_audit_sha256": "e" * 64,
            "task_body_in_audit": False,
            "workspace_marker_sha256": "f" * 64,
        }
        verified = validate_task_evidence(
            evidence,
            source_commit="a" * 40,
            manifest_sha256="b" * 64,
            app_id=evidence["app_id"],
            audit_store_id=evidence["audit_store_id"],
            expected_helper_relative_path=evidence["export_runtime_broker_relative_path"],
        )
        self.assertIs(verified, evidence)

        evidence["real_owner_authority"] = True
        with self.assertRaisesRegex(ValueError, "bounded contract"):
            validate_task_evidence(
                evidence,
                source_commit="a" * 40,
                manifest_sha256="b" * 64,
                app_id=evidence["app_id"],
                audit_store_id=evidence["audit_store_id"],
                expected_helper_relative_path=evidence["export_runtime_broker_relative_path"],
            )

    def test_actions_summary_uses_validated_fields_and_cleanup_result(self) -> None:
        record = {
            "source_commit": "a" * 40,
            "codex_cli_version": "codex-cli 0.166.0",
            "export": {
                "source_manifest_sha256": "b" * 64,
                "artifact_tree_sha256": "c" * 64,
                "artifact_total_bytes": 12,
                "app_id": "d4-pocket-app-0123456789abcdef0123456789abcdef",
                "audit_store_id": "audit-store-fedcba9876543210fedcba9876543210",
            },
            "independent_runtime": {
                "test_broker_sha256": "d" * 64,
                "task_status": "completed",
                "task_result_hash": "sha256:" + "e" * 64,
                "outside_workspace_write": "denied",
                "approval_replay": "rejected",
                "codex_process_tree_cleanup": "passed",
                "scratch_cleanup": "passed",
                "run_root_cleanup": "passed",
                "audit_chain": "passed",
            },
        }
        with tempfile.TemporaryDirectory() as directory:
            summary_path = Path(directory) / "summary.md"
            with patch.dict(os.environ, {"GITHUB_STEP_SUMMARY": str(summary_path)}):
                _write_actions_summary(record)
            summary = summary_path.read_text(encoding="utf-8")
            self.assertIn("`" + "b" * 64 + "`", summary)
            self.assertIn("`denied` / `rejected`", summary)
            self.assertIn("`passed`.", summary)


if __name__ == "__main__":
    unittest.main()
