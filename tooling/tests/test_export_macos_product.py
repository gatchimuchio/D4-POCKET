from __future__ import annotations

import tempfile
import hashlib
import json
import shutil
import subprocess
import unittest
from pathlib import Path
from unittest.mock import patch

from tooling.export_macos_product import (
    _macos_artifact_inventory,
    _macos_environment,
    _darwin_build_architecture,
    _patch_app_identity,
    _patch_macos_export_build_settings,
    _require_macos_binary_architecture,
    _require_macos_code_without_publisher_identity,
    _scan_macos_artifacts,
    _validate_build_evidence,
    build_macos_product,
    bundle_identifier_for_app,
)
from tooling.export_windows_product import validate_export_inputs
from tooling.schema_check.check_schemas import validate_instance
from tooling.tests.create_macos_export_fixture import (
    APP_ID,
    AUDIT_STORE_ID,
    macos_export_pair,
)


class MacOSProductExportTests(unittest.TestCase):
    def test_isolated_macos_environment_preserves_explicit_rustup_home(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-macos-env-") as temp:
            root = Path(temp)
            flutter_root = root / "flutter"
            rustup_home = root / "runner-rustup"
            flutter_root.mkdir()
            rustup_home.mkdir()
            with patch.dict(
                "os.environ",
                {
                    "PATH": "/fixed/bin",
                    "RUSTUP_HOME": str(rustup_home),
                    "AWS_SECRET_ACCESS_KEY": "must-not-propagate",
                    "FLUTTER_XCODE_CODE_SIGNING_ALLOWED": "YES",
                    "FLUTTER_XCODE_CODE_SIGNING_REQUIRED": "YES",
                    "FLUTTER_XCODE_CODE_SIGN_IDENTITY": "unexpected-parent-identity",
                },
                clear=True,
            ):
                environment = _macos_environment(root, flutter_root=flutter_root)

            self.assertEqual(environment["PATH"], "/fixed/bin")
            self.assertEqual(environment["RUSTUP_HOME"], str(rustup_home))
            self.assertEqual(environment["HOME"], str(root / "home"))
            self.assertEqual(environment["CARGO_HOME"], str(root / "cargo"))
            self.assertEqual(environment["FLUTTER_ROOT"], str(flutter_root))
            self.assertEqual(environment["FLUTTER_XCODE_CODE_SIGNING_ALLOWED"], "NO")
            self.assertEqual(environment["FLUTTER_XCODE_CODE_SIGNING_REQUIRED"], "NO")
            self.assertEqual(environment["FLUTTER_XCODE_CODE_SIGN_IDENTITY"], "")
            self.assertNotIn("AWS_SECRET_ACCESS_KEY", environment)

    def test_isolated_macos_environment_rejects_missing_rustup_home(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-macos-env-missing-") as temp:
            root = Path(temp)
            flutter_root = root / "flutter"
            flutter_root.mkdir()
            with patch.dict("os.environ", {"PATH": "/fixed/bin"}, clear=True):
                with self.assertRaisesRegex(ValueError, "toolchain環境が不足"):
                    _macos_environment(root, flutter_root=flutter_root)

    def test_macos_receipt_and_manifest_are_valid_for_macos_module_plan(self) -> None:
        receipt_raw, manifest_raw = macos_export_pair()
        receipt, manifest, plan, receipt_hash, manifest_hash = validate_export_inputs(
            receipt_raw, manifest_raw, target_platform="macos"
        )
        self.assertEqual(receipt["export_manifest"]["app_identity"]["target_platform"], "macos")
        self.assertEqual(manifest["manifest"]["audit_store"]["store_id"], AUDIT_STORE_ID)
        self.assertEqual(plan.selection_mode, "explicit_optional")
        self.assertEqual(len(receipt_hash), 64)
        self.assertEqual(len(manifest_hash), 64)
        receipt["export_manifest"]["app_identity"]["target_platform"] = "linux"
        with self.assertRaises(ValueError):
            validate_export_inputs(
                json.dumps(receipt, ensure_ascii=False).encode("utf-8"),
                manifest_raw,
                target_platform="macos",
            )

    def test_bundle_identifier_is_derived_from_valid_app_id(self) -> None:
        self.assertEqual(
            bundle_identifier_for_app(APP_ID),
            "com.d4p.export.33333333333333333333333333333333",
        )
        with self.assertRaises(ValueError):
            bundle_identifier_for_app("d4-pocket-app-../outside")

    def test_darwin_architecture_is_canonical_and_unknown_hosts_fail_closed(self) -> None:
        with patch("tooling.export_macos_product.platform.machine", return_value="arm64"):
            self.assertEqual(_darwin_build_architecture(), "arm64")
        with patch("tooling.export_macos_product.platform.machine", return_value="AMD64"):
            self.assertEqual(_darwin_build_architecture(), "x86_64")
        with patch("tooling.export_macos_product.platform.machine", return_value="riscv64"):
            with self.assertRaisesRegex(ValueError, "host architectureが未対応"):
                _darwin_build_architecture()

    def test_source_snapshot_changes_only_the_export_bundle_identifier(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-macos-config-") as temp:
            config = Path(temp) / "apps/desktop_flutter/macos/Runner/Configs/AppInfo.xcconfig"
            config.parent.mkdir(parents=True)
            config.write_text(
                "PRODUCT_NAME = gui_shell_desktop\n"
                "PRODUCT_BUNDLE_IDENTIFIER = com.example.guiShellDesktop\n",
                encoding="utf-8",
            )
            _patch_app_identity(Path(temp), bundle_identifier_for_app(APP_ID))
            updated = config.read_text(encoding="utf-8")
            self.assertIn("PRODUCT_NAME = gui_shell_desktop", updated)
            self.assertIn(bundle_identifier_for_app(APP_ID), updated)
            self.assertNotIn("com.example.guiShellDesktop", updated)

    def test_export_snapshot_disables_signing_and_pins_host_architecture(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-macos-unsigned-") as temp:
            source_project = (
                Path(__file__).resolve().parents[2]
                / "apps/desktop_flutter/macos/Runner.xcodeproj/project.pbxproj"
            )
            project = Path(temp) / "apps/desktop_flutter/macos/Runner.xcodeproj/project.pbxproj"
            project.parent.mkdir(parents=True)
            shutil.copyfile(source_project, project)

            _patch_macos_export_build_settings(Path(temp), architecture="arm64")

            updated = project.read_text(encoding="utf-8")
            self.assertEqual(updated.count("CODE_SIGNING_ALLOWED = NO;"), 15)
            self.assertEqual(updated.count("CODE_SIGNING_REQUIRED = NO;"), 15)
            self.assertEqual(
                updated.count('CODE_SIGN_IDENTITY = "";'),
                15 + source_project.read_text(encoding="utf-8").count('CODE_SIGN_IDENTITY = "-";'),
            )
            self.assertNotIn('CODE_SIGN_IDENTITY = "-";', updated)
            self.assertEqual(updated.count("ARCHS = arm64;"), 15)
            self.assertIn("/usr/bin/install -m 755", updated)
            self.assertNotIn("/usr/bin/codesign --force --sign", updated)
            self.assertIn("/usr/bin/codesign --force --sign", source_project.read_text(encoding="utf-8"))

    def test_binary_architecture_scan_requires_exact_host_architecture(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-macos-arch-scan-") as temp:
            binaries = [Path(temp) / "app", Path(temp) / "broker"]
            for binary in binaries:
                binary.write_bytes(b"Mach-O fixture")
            matching = subprocess.CompletedProcess(args=[], returncode=0, stdout="arm64\n")
            with patch("tooling.export_macos_product.subprocess.run", return_value=matching):
                result = _require_macos_binary_architecture(
                    binaries, lipo="/usr/bin/lipo", architecture="arm64", environment={}
                )
            self.assertEqual(
                result,
                {
                    "status": "passed_host_architecture",
                    "architecture": "arm64",
                    "binary_files_checked": 2,
                },
            )
            for output in ("x86_64\n", "arm64 x86_64\n"):
                mismatch = subprocess.CompletedProcess(args=[], returncode=0, stdout=output)
                with patch("tooling.export_macos_product.subprocess.run", return_value=mismatch):
                    with self.assertRaisesRegex(ValueError, "architectureがbuild hostと一致しない"):
                        _require_macos_binary_architecture(
                            binaries, lipo="/usr/bin/lipo", architecture="arm64", environment={}
                        )

    def test_app_scan_accepts_unsigned_and_unsealed_adhoc_executable_signatures(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-macos-codesign-scan-") as temp:
            app = Path(temp) / "D4Pocket.app"
            executable = app / "Contents/MacOS/gui_shell_desktop"
            executable.parent.mkdir(parents=True)
            executable.write_bytes(b"\xcf\xfa\xed\xfeunsigned fixture")
            framework = app / "Contents/Frameworks/Flutter.framework"
            framework_binary = framework / "Versions/A/Flutter"
            framework_binary.parent.mkdir(parents=True)
            framework_binary.write_bytes(b"\xfe\xed\xfa\xcfunsigned fixture")
            adhoc = subprocess.CompletedProcess(
                args=[],
                returncode=0,
                stdout=(
                    "Format=app bundle with Mach-O thin (arm64)\n"
                    "CodeDirectory v=20500 size=250 flags=0x2(adhoc) hashes=3+0 location=embedded\n"
                    "Signature=adhoc\n"
                    "TeamIdentifier=not set\n"
                ),
            )
            unsigned = subprocess.CompletedProcess(
                args=[], returncode=1, stdout="code object is not signed at all\n"
            )
            responses = [adhoc, adhoc, unsigned, adhoc]
            with patch("tooling.export_macos_product.subprocess.run", side_effect=responses) as run:
                result = _require_macos_code_without_publisher_identity(
                    app, codesign="/usr/bin/codesign", environment={}
                )

            self.assertEqual(
                result,
                {
                    "status": "passed_no_publisher_identity",
                    "code_objects_checked": 4,
                    "unsigned_code_objects": 1,
                    "adhoc_executable_signatures": 3,
                    "publisher_identity_signatures": 0,
                    "resource_sealed_bundle_signatures": 0,
                },
            )
            self.assertEqual(run.call_count, 4)
            self.assertTrue(all(call.kwargs["env"]["LC_ALL"] == "C" for call in run.call_args_list))
            self.assertTrue(all("--verbose=4" in call.args[0] for call in run.call_args_list))

    def test_app_scan_rejects_publisher_or_resource_sealed_signatures(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-macos-codesign-negative-") as temp:
            app = Path(temp) / "D4Pocket.app"
            executable = app / "Contents/MacOS/gui_shell_desktop"
            executable.parent.mkdir(parents=True)
            executable.write_bytes(b"\xcf\xfa\xed\xfefixture")
            disallowed_signatures = [
                subprocess.CompletedProcess(
                    args=[],
                    returncode=0,
                    stdout=(
                        "Authority=Developer ID Application: Example (TEAM123)\n"
                        "TeamIdentifier=TEAM123\n"
                        "Signature=Developer ID Application\n"
                    ),
                ),
                subprocess.CompletedProcess(
                    args=[],
                    returncode=0,
                    stdout=(
                        "Format=app bundle with Mach-O thin (arm64)\n"
                        "CodeDirectory v=20500 size=250 flags=0x2(adhoc) hashes=3+0 location=embedded\n"
                        "Signature=adhoc\n"
                        "Sealed Resources version=2 rules=13 files=4\n"
                    ),
                ),
                subprocess.CompletedProcess(
                    args=[],
                    returncode=0,
                    stdout=(
                        "CodeDirectory v=20500 size=250 flags=0x2(adhoc) hashes=3+0 location=embedded\n"
                        "CodeDirectory v=20500 size=250 flags=0x10000(runtime) hashes=3+0 location=embedded\n"
                        "Signature=adhoc\n"
                    ),
                ),
            ]
            for signed in disallowed_signatures:
                with patch("tooling.export_macos_product.subprocess.run", return_value=signed):
                    with self.assertRaisesRegex(ValueError, "Publisher identity付きまたは許容外署名"):
                        _require_macos_code_without_publisher_identity(
                            app, codesign="/usr/bin/codesign", environment={}
                        )
            ambiguous = subprocess.CompletedProcess(args=[], returncode=1, stdout="verification failed\n")
            with patch("tooling.export_macos_product.subprocess.run", return_value=ambiguous):
                with self.assertRaisesRegex(ValueError, "code signature状態を分類できない"):
                    _require_macos_code_without_publisher_identity(
                        app, codesign="/usr/bin/codesign", environment={}
                    )

    def test_build_evidence_schema_rejects_publisher_signed_candidate_claims(self) -> None:
        schema = json.loads(
            (Path(__file__).resolve().parents[2] / "specs/gui_shell_macos_export_build.schema.json")
            .read_text(encoding="utf-8")
        )
        example_path = (
            Path(__file__).resolve().parents[2]
            / "examples/contracts/gui_shell_macos_export_build.valid.json"
        )
        evidence = json.loads(example_path.read_text(encoding="utf-8"))
        self.assertNotIn("--no-codesign", evidence["flutter_build_arguments"])
        self.assertEqual(
            evidence["app_code_signature_scan"]["status"], "passed_no_publisher_identity"
        )
        self.assertEqual(evidence["build_mode"], "release_candidate_no_publisher_identity")
        self.assertEqual(evidence["target_architecture"], evidence["toolchain"]["architecture"])
        self.assertEqual(
            evidence["macos_binary_architecture_scan"]["status"], "passed_host_architecture"
        )
        self.assertEqual(validate_instance(evidence, schema), [])
        evidence["package_signed"] = True
        self.assertTrue(validate_instance(evidence, schema))
        evidence["package_signed"] = False
        evidence["app_publisher_identity_signed"] = True
        self.assertTrue(validate_instance(evidence, schema))

    def test_build_evidence_is_bound_to_manifest_and_artifact_tree(self) -> None:
        root = Path(__file__).resolve().parents[2]
        evidence = json.loads(
            (root / "examples/contracts/gui_shell_macos_export_build.valid.json")
            .read_text(encoding="utf-8")
        )
        receipt_raw, manifest_raw = macos_export_pair()
        del receipt_raw
        manifest_hash = hashlib.sha256(manifest_raw).hexdigest()
        with tempfile.TemporaryDirectory(prefix="d4p-macos-evidence-") as temp:
            output = Path(temp)
            app = output / "D4Pocket-333333333333.app"
            resources = app / "Contents/Resources"
            resources.mkdir(parents=True)
            (resources / "product_manifest.json").write_bytes(manifest_raw)
            package_path = output / "D4Pocket-333333333333.pkg"
            package_path.write_bytes(b"unsigned package fixture")
            archive_path = output / "D4Pocket-333333333333.app.zip"
            archive_path.write_bytes(b"app archive fixture")

            records, regular_files, total_bytes, tree_hash = _macos_artifact_inventory(output, app)
            evidence.update(
                {
                    "app_id": APP_ID,
                    "audit_store_id": AUDIT_STORE_ID,
                    "bundle_identifier": bundle_identifier_for_app(APP_ID),
                    "installer_identifier": bundle_identifier_for_app(APP_ID) + ".installer",
                    "bundle_name": app.name,
                    "source_manifest_sha256": manifest_hash,
                    "target_architecture": "arm64",
                    "macos_binary_architecture_scan": {
                        "status": "passed_host_architecture",
                        "architecture": "arm64",
                        "binary_files_checked": 2,
                    },
                    "artifact_files": records,
                    "artifact_total_bytes": total_bytes,
                    "artifact_tree_sha256": tree_hash,
                    "app_code_signature_scan": {
                        "status": "passed_no_publisher_identity",
                        "code_objects_checked": 2,
                        "unsigned_code_objects": 1,
                        "adhoc_executable_signatures": 1,
                        "publisher_identity_signatures": 0,
                        "resource_sealed_bundle_signatures": 0,
                    },
                    "package": {
                        "file_name": package_path.name,
                        "format": "macOS flat pkg",
                        "byte_length": package_path.stat().st_size,
                        "sha256": hashlib.sha256(package_path.read_bytes()).hexdigest(),
                        "signed": False,
                        "installed_product_verified": False,
                    },
                    "app_archive": {
                        "file_name": archive_path.name,
                        "byte_length": archive_path.stat().st_size,
                        "sha256": hashlib.sha256(archive_path.read_bytes()).hexdigest(),
                    },
                }
            )
            evidence["compile_time_identity"].update(
                {
                    "app_id": APP_ID,
                    "audit_store_id": AUDIT_STORE_ID,
                    "manifest_sha256": manifest_hash,
                }
            )
            evidence["credential_artifact_scan"].update(
                {
                    "files_scanned": len(regular_files),
                    "bytes_scanned": total_bytes,
                    "artifact_tree_sha256": tree_hash,
                }
            )
            _validate_build_evidence(evidence, output, app, manifest_raw)
            evidence["target_architecture"] = "x86_64"
            with self.assertRaises(ValueError):
                _validate_build_evidence(evidence, output, app, manifest_raw)
            evidence["target_architecture"] = "arm64"
            evidence["macos_binary_architecture_scan"]["architecture"] = "x86_64"
            with self.assertRaises(ValueError):
                _validate_build_evidence(evidence, output, app, manifest_raw)
            evidence["macos_binary_architecture_scan"]["architecture"] = "arm64"
            evidence["compile_time_identity"]["manifest_sha256"] = "0" * 64
            with self.assertRaises(ValueError):
                _validate_build_evidence(evidence, output, app, manifest_raw)

    def test_inventory_binds_app_files_package_and_archive(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-macos-inventory-") as temp:
            root = Path(temp)
            app = root / "D4Pocket.app"
            (app / "Contents/Resources").mkdir(parents=True)
            (app / "Contents/Resources/product_manifest.json").write_bytes(b"manifest")
            (root / "candidate.pkg").write_bytes(b"package")
            (root / "candidate.app.zip").write_bytes(b"archive")
            records, files, total, tree_hash = _macos_artifact_inventory(root, app)
            self.assertEqual(len(records), 3)
            self.assertEqual(len(files), 3)
            self.assertEqual(total, len(b"manifestpackagearchive"))
            self.assertEqual(len(tree_hash), 64)
            scan = _scan_macos_artifacts(root, app, records, files, total, tree_hash)
            self.assertEqual(scan["status"], "passed_known_patterns")
            self.assertEqual(scan["findings"], 0)
            self.assertEqual(scan["artifact_tree_sha256"], tree_hash)

    def test_credential_pattern_in_app_is_rejected_without_echoing_value(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-macos-credential-") as temp:
            root = Path(temp)
            app = root / "D4Pocket.app"
            (app / "Contents/MacOS").mkdir(parents=True)
            canary = b"ghp_" + b"A" * 24
            (app / "Contents/MacOS/payload").write_bytes(canary)
            records, files, total, tree_hash = _macos_artifact_inventory(root, app)
            with self.assertRaisesRegex(ValueError, "Credential pattern"):
                _scan_macos_artifacts(root, app, records, files, total, tree_hash)

    def test_non_macos_host_cannot_start_product_export(self) -> None:
        with patch("tooling.export_macos_product.sys.platform", "win32"):
            with self.assertRaisesRegex(ValueError, "macOS native host"):
                build_macos_product(Path("receipt"), Path("manifest"), Path("output"))


if __name__ == "__main__":
    unittest.main()
