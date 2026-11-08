from __future__ import annotations

import hashlib
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from tooling.tests.prepare_linux_same_source_update import (
    _build_evidence_validation_inputs,
    _safe_exception_summary,
)


ROOT = Path(__file__).resolve().parents[2]
EXPORT_HELPER = ROOT / "tooling/linux_export_workflow.py"
DEBIAN_HELPER = ROOT / "tooling/linux_debian_setup_workflow.py"
SUMMARY_HELPER = ROOT / "tooling/linux_installed_acceptance_summary.py"


def write_json(path: Path, value: object) -> None:
    path.write_text(json.dumps(value), encoding="utf-8")


def run_helper(script: Path, *arguments: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, "-X", "utf8", str(script), *arguments],
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


class LinuxWorkflowHelperTests(unittest.TestCase):
    def test_same_source_candidate_diagnostic_redacts_absolute_paths(self) -> None:
        summary = _safe_exception_summary(
            ValueError("invalid evidence at /home/runner/work/private/output and C:\\Users\\runner\\private")
        )
        self.assertIn("invalid evidence at", summary)
        self.assertIn("<path>", summary)
        self.assertNotIn("/home/runner", summary)
        self.assertNotIn("C:\\Users\\runner", summary)

    def test_same_source_candidate_revalidates_exported_portable_archive(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            output = root / "output"
            output.mkdir()
            package = output / "D4Pocket-linux.d4pkg"
            package.write_bytes(b"D4PKG01\n")
            archive = output / "D4Pocket-0.1.1-linux-x64-portable.tar.gz"
            archive.write_bytes(b"portable archive")

            package_path, archive_path, readme_raw = _build_evidence_validation_inputs(
                {
                    "package": {"file_name": package.name},
                    "portable_archive": {"file_name": archive.name},
                },
                root,
            )

            self.assertEqual(package_path, package.resolve())
            self.assertEqual(archive_path, archive.resolve())
            self.assertEqual(
                readme_raw,
                (ROOT / "installer" / "linux" / "PORTABLE_README_ja.txt").read_bytes(),
            )

    def test_same_source_candidate_rejects_portable_archive_path_escape(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            output = root / "output"
            output.mkdir()
            package = output / "D4Pocket-linux.d4pkg"
            package.write_bytes(b"D4PKG01\n")
            with self.assertRaisesRegex(ValueError, "portable archive file名"):
                _build_evidence_validation_inputs(
                    {
                        "package": {"file_name": package.name},
                        "portable_archive": {"file_name": "../outside.tar.gz"},
                    },
                    root,
                )

    def test_verify_build_result_and_rejects_wrong_source(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            package = root / "D4Pocket-linux.d4pkg"
            package.write_bytes(b"package")
            write_json(root / "d4_pocket_linux_export_build.json", {
                "target_platform": "linux",
                "source_commit": "source-1",
                "artifact_tree_sha256": "a" * 64,
                "signed": False,
                "formal_distribution_claimed": False,
                "package": {"installed_product_verified": False, "sha256": digest(package)},
                "cmake_configure_arguments": ["-DD4_LINUX_R2_E2E_TEST_BUILD=ON"],
            })
            valid = run_helper(EXPORT_HELPER, "verify-build-result", str(root), "source-1", "agent_task_supervision")
            self.assertEqual(valid.returncode, 0, valid.stderr)
            self.assertIn('"r2_e2e_test_build":true', valid.stdout)
            invalid = run_helper(EXPORT_HELPER, "verify-build-result", str(root), "source-2", "agent_task_supervision")
            self.assertNotEqual(invalid.returncode, 0)

    def test_debian_lifecycle_is_a_verification_build_only_for_that_scope(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            package = root / "D4Pocket-linux.d4pkg"
            package.write_bytes(b"package")
            for scope, expected in (("debian_setup_lifecycle", True), ("product_build", False)):
                write_json(root / "d4_pocket_linux_export_build.json", {
                    "target_platform": "linux",
                    "source_commit": "source-1",
                    "artifact_tree_sha256": "a" * 64,
                    "signed": False,
                    "formal_distribution_claimed": False,
                    "package": {"installed_product_verified": False, "sha256": digest(package)},
                    "cmake_configure_arguments": [
                        "-DD4_LINUX_R2_E2E_TEST_BUILD=" + ("ON" if expected else "OFF")
                    ],
                })
                result = run_helper(EXPORT_HELPER, "verify-build-result", str(root), "source-1", scope)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn(f'"r2_e2e_test_build":{str(expected).lower()}', result.stdout)

    def test_verify_portable_archive_and_rejects_bad_digest(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            archive = root / "D4Pocket-linux.tar.gz"
            archive.write_bytes(b"archive")
            record = {
                "target": "ubuntu-24.04-x86_64", "format": "tar.gz",
                "file_name": archive.name, "byte_length": archive.stat().st_size,
                "sha256": digest(archive),
            }
            write_json(root / "d4_pocket_linux_export_build.json", {"portable_archive": record})
            valid = run_helper(EXPORT_HELPER, "verify-portable-archive", str(root), archive.name)
            self.assertEqual(valid.returncode, 0, valid.stderr)
            record["sha256"] = "0" * 64
            write_json(root / "d4_pocket_linux_export_build.json", {"portable_archive": record})
            invalid = run_helper(EXPORT_HELPER, "verify-portable-archive", str(root), archive.name)
            self.assertNotEqual(invalid.returncode, 0)

    def test_verify_package_pair_and_rejects_tree_mismatch(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = root / "product_manifest.json"
            write_json(manifest, {"manifest": {"app_identity": {"app_id": "app"}, "audit_store": {"store_id": "audit"}}})
            records = []
            for revision in (1, 2):
                package = root / f"package-{revision}.deb"
                package.write_bytes(f"package-{revision}".encode())
                records.append({
                    "evidence_type": "d4_pocket_linux_debian_setup_package",
                    "target": "ubuntu-24.04-amd64", "package_version": f"0.1.1-{revision}",
                    "source_commit": "source-1", "artifact_tree_sha256": "a" * 64,
                    "scope": "first_run_setup_bootstrap_only", "signed": False,
                    "owner_authorization_created": False, "permission_created": False,
                    "approval_created": False, "credential_values_included": False,
                    "user_xdg_data_modified_by_package": False, "package_manager_scripts": [],
                    "output_path": str(package), "package_size_bytes": package.stat().st_size,
                    "package_sha256": digest(package),
                })
            files = []
            for index, record in enumerate(records, start=1):
                path = root / f"record-{index}.json"
                write_json(path, record)
                files.append(path)
            valid = run_helper(DEBIAN_HELPER, "verify-package-pair", *(str(p) for p in files), "source-1", str(manifest))
            self.assertEqual(valid.returncode, 0, valid.stderr)
            records[1]["artifact_tree_sha256"] = "b" * 64
            write_json(files[1], records[1])
            invalid = run_helper(DEBIAN_HELPER, "verify-package-pair", *(str(p) for p in files), "source-1", str(manifest))
            self.assertNotEqual(invalid.returncode, 0)

    def test_verify_first_install_package_and_rejects_wrong_digest(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            package = root / "setup.deb"
            package.write_bytes(b"setup package")
            record = {
                "evidence_type": "d4_pocket_linux_debian_setup_package",
                "target": "ubuntu-24.04-amd64", "source_commit": "source-1",
                "scope": "first_run_setup_bootstrap_only", "signed": False,
                "owner_authorization_created": False, "permission_created": False,
                "approval_created": False, "credential_values_included": False,
                "package_manager_scripts": [], "output_path": str(package),
                "package_size_bytes": package.stat().st_size, "package_sha256": digest(package),
                "artifact_tree_sha256": "a" * 64,
            }
            record_path = root / "record.json"
            manifest = root / "manifest.json"
            write_json(record_path, record)
            write_json(manifest, {"manifest": {"app_identity": {"app_id": "app"}, "audit_store": {"store_id": "audit"}}})
            valid = run_helper(DEBIAN_HELPER, "verify-first-install-package", str(record_path), "source-1", str(manifest))
            self.assertEqual(valid.returncode, 0, valid.stderr)
            record["package_sha256"] = "0" * 64
            write_json(record_path, record)
            invalid = run_helper(DEBIAN_HELPER, "verify-first-install-package", str(record_path), "source-1", str(manifest))
            self.assertNotEqual(invalid.returncode, 0)

    def test_validate_installed_acceptance_selects_scope_and_checks_versions(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            log = root / "acceptance.log"
            result = {
                "acceptance_scope": "debian_setup_lifecycle",
                "first_install": {"product_version": "0.1.1", "package_sha256": "a" * 64},
                "update_lifecycle": {"active_record": {"product_version": "0.1.2"}},
                "active_version": "0.1.1", "uninstall": {"completed": True}, "live_audit": {"valid": True},
            }
            log.write_text("noise\n" + json.dumps({"acceptance_scope": "other"}) + "\n" + json.dumps(result) + "\n", encoding="utf-8")
            valid = run_helper(DEBIAN_HELPER, "validate-installed-acceptance", str(log), "debian_setup_lifecycle", "a" * 64)
            self.assertEqual(valid.returncode, 0, valid.stderr)
            result["first_install"]["product_version"] = "0.1.0"
            log.write_text(json.dumps(result) + "\n", encoding="utf-8")
            invalid = run_helper(DEBIAN_HELPER, "validate-installed-acceptance", str(log), "debian_setup_lifecycle", "a" * 64)
            self.assertNotEqual(invalid.returncode, 0)

    def test_summary_helpers_accept_matching_records_and_reject_missing_scope(self) -> None:
        cases = (
            ("runtime_provenance", {
                "source_commit": "source-1", "package_sha256": "a" * 64, "package_byte_length": 10,
                "artifact_tree_sha256": "b" * 64, "installed_artifact_tree_sha256": "c" * 64,
                "artifact_count": 3, "installed_root": "/installed", "active_stage": "active",
                "runtime_identity": {"app_id": "app", "audit_store_id": "audit"},
                "component_sha256": {"runner": "d", "launcher": "e", "broker": "f"},
                "live_processes": {"runner": {"sha256": "d", "process_count": 1}, "broker": {"sha256": "f", "process_count": 1}},
                "audit_store_observation": {"path": "/audit", "audit.jsonl": {"sha256": "g"}, "audit_anchor.json": {"sha256": "h"}, "anchor_key_private_file": False},
            }, "summarize-runtime-provenance"),
            ("wayland_rendering", {
                "source_commit": "source-1", "installed_package_sha256": "a" * 64,
                "display_server": "wayland", "visible_window": True,
                "compositor_output": {"width": 1024, "height": 640, "changed_pixels": 10,
                    "required_changed_pixels": 5, "baseline_png_sha256": "b", "rendered_png_sha256": "c", "image_files_retained": False},
                "live_installed_runner": {"runner": {"sha256": "d", "process_count": 1}},
                "owned_process_cleanup": "pass",
            }, "summarize-wayland-rendering"),
            ("wayland_workspace_selection", {
                "source_commit": "source-1", "installed_package_sha256": "a" * 64, "product_version": "0.1.0",
                "display_server": "wayland", "visible_window": True, "keyboard_input": "fixture",
                "workspace_selection": {"workspace_selection": "accepted", "registration": "recorded",
                    "cli_interface_probe_calls": 1, "task_execution": "unsupported", "workspace_read_permission": False,
                    "accepted_registration_audit": True, "audit_secret_and_root_exposure": False},
                "live_installed_runner": {"runner": {"sha256": "d", "process_count": 1}},
                "owned_process_cleanup": "pass",
            }, "summarize-wayland-workspace-selection"),
        )
        with tempfile.TemporaryDirectory() as directory:
            log = Path(directory) / "summary.jsonl"
            for scope, fields, operation in cases:
                with self.subTest(scope=scope):
                    log.write_text(json.dumps({"acceptance_scope": scope, **fields}) + "\n", encoding="utf-8")
                    valid = run_helper(SUMMARY_HELPER, operation, str(log))
                    self.assertEqual(valid.returncode, 0, valid.stderr)
                    log.write_text(json.dumps({"acceptance_scope": "other", **fields}) + "\n", encoding="utf-8")
                    invalid = run_helper(SUMMARY_HELPER, operation, str(log))
                    self.assertNotEqual(invalid.returncode, 0)

    def test_unknown_operation_fails_closed(self) -> None:
        result = run_helper(EXPORT_HELPER, "not-an-operation")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("使い方:", result.stderr)


if __name__ == "__main__":
    unittest.main()
