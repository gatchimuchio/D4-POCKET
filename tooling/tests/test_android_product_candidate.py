from __future__ import annotations

import json
import shutil
import subprocess
import tempfile
import uuid
import unittest
import zipfile
from pathlib import Path
from unittest.mock import patch

from tooling import package_android_product_candidate as candidate
from tooling.schema_check.check_schemas import validate_instance


ROOT = Path(__file__).resolve().parents[2]
COMMIT = "a" * 40


class AndroidProductCandidateTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory(prefix="d4p-android-candidate-test-")
        self.root = Path(self.temp.name)
        self.output = self.root / "published"
        self.build_root = (
            ROOT / "apps/mobile_flutter/build/app/outputs/flutter-apk"
        )
        self.created_directories = [
            self.build_root,
            self.build_root.parent,
            self.build_root.parent.parent,
            self.build_root.parent.parent.parent,
        ]
        self.existing_directories = {path for path in self.created_directories if path.exists()}
        self.fixture_dir = self.build_root / f"candidate-test-{uuid.uuid4().hex}"
        self.fixture_dir.mkdir(parents=True)
        self.debug_apk = self.fixture_dir / "app-debug.apk"
        self.release_apk = self.fixture_dir / "app-release.apk"
        self._write_apk(self.debug_apk)
        self._write_apk(self.release_apk, marker=b"release-build")
        self.app_id, self.version_name, self.version_code = candidate._application_identity()
        self.debug_dn_prefix = "V2 Signer: certificate DN: "
        self.release_unsigned_diagnostic = "ERROR: No signatures found in APK\n"
        sdk_tools = self.root / "sdk" / "build-tools" / "37.0.0"
        sdk_tools.mkdir(parents=True)
        self.aapt = sdk_tools / "aapt"
        self.apksigner = sdk_tools / "apksigner"

    def tearDown(self) -> None:
        self.temp.cleanup()
        shutil.rmtree(self.fixture_dir, ignore_errors=True)
        for directory in self.created_directories:
            if directory in self.existing_directories:
                continue
            try:
                directory.rmdir()
            except OSError:
                pass

    @staticmethod
    def _write_apk(path: Path, *, marker: bytes = b"debug-build", signed: bool = False) -> None:
        with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_DEFLATED) as archive:
            archive.writestr("AndroidManifest.xml", b"compiled-manifest")
            archive.writestr("classes.dex", marker)
            if signed:
                archive.writestr("META-INF/CERT.RSA", b"signature-marker")

    def _run_tool(self, arguments: list[str], **_kwargs: object) -> subprocess.CompletedProcess[str]:
        if arguments[0] == str(self.aapt):
            output = (
                f"package: name='{self.app_id}' versionCode='{self.version_code}' "
                f"versionName='{self.version_name}'\n"
            )
            return subprocess.CompletedProcess(arguments, 0, stdout=output, stderr="")
        if arguments[0] == str(self.apksigner) and "emulator-debug" in Path(arguments[-1]).name:
            return subprocess.CompletedProcess(
                arguments,
                0,
                stdout=f"{self.debug_dn_prefix}C=US, O=Android, CN=Android Debug\n",
                stderr="",
            )
        return subprocess.CompletedProcess(
            arguments,
            1,
            stdout="DOES NOT VERIFY\n",
            stderr=self.release_unsigned_diagnostic,
        )

    def _package(self) -> dict[str, object]:
        with (
            patch.object(candidate, "_require_clean_source", return_value=COMMIT),
            patch.object(
                candidate,
                "_find_android_tool",
                side_effect=lambda name, _explicit: self.aapt if name == "aapt" else self.apksigner,
            ),
            patch.object(candidate, "_run_tool", side_effect=self._run_tool),
        ):
            return candidate.package_android_product_candidate(
                self.debug_apk,
                self.release_apk,
                self.output,
                source_commit=COMMIT,
                flutter_version="3.44.0",
                java_version="17.0.20",
                android_build_tools_version="37.0.0",
            )

    def test_packages_debug_and_unsigned_release_without_distribution_claim(self) -> None:
        evidence = self._package()
        self.assertEqual(evidence["distribution_ready"], False)
        self.assertEqual(evidence["formal_distribution_claimed"], False)
        self.assertEqual(evidence["release_signature_state"], "unsigned")
        self.assertEqual(
            {path.name for path in self.output.iterdir()},
            {
                "d4-pocket-android-emulator-debug.apk",
                "d4-pocket-android-release-unsigned.apk",
                "android-product-candidate.json",
            },
        )
        record = json.loads((self.output / "android-product-candidate.json").read_text())
        schema = json.loads(candidate.SCHEMA_PATH.read_text(encoding="utf-8"))
        self.assertEqual(validate_instance(record, schema), [])
        self.assertEqual(record["application_identity_confirmed"], False)
        self.assertEqual(record["credential_included"], False)
        self.assertEqual(record["installed_product_verified"], False)

    def test_accepts_legacy_apksigner_debug_signer_line(self) -> None:
        self.debug_dn_prefix = "Signer #1 certificate DN: "
        evidence = self._package()
        self.assertEqual(evidence["release_signature_state"], "unsigned")

    def test_accepts_build_tools_37_unsigned_apksigner_diagnostic(self) -> None:
        self.release_unsigned_diagnostic = "ERROR: Missing META-INF/MANIFEST.MF\n"
        evidence = self._package()
        self.assertEqual(evidence["release_signature_state"], "unsigned")

    def test_schema_fixtures_separate_candidate_from_distribution_readiness(self) -> None:
        schema = json.loads(candidate.SCHEMA_PATH.read_text(encoding="utf-8"))
        valid = json.loads(
            (ROOT / "examples/contracts/gui_shell_android_product_candidate.valid.json")
            .read_text(encoding="utf-8")
        )
        invalid = json.loads(
            (
                ROOT
                / "examples/contracts/invalid/gui_shell_android_product_candidate_distribution_ready.invalid.json"
            ).read_text(encoding="utf-8")
        )
        self.assertEqual(validate_instance(valid, schema), [])
        self.assertTrue(validate_instance(invalid, schema))

    def test_rejects_unclassified_apksigner_failure(self) -> None:
        original = self._run_tool

        def unknown_failure(arguments: list[str], **kwargs: object) -> subprocess.CompletedProcess[str]:
            if (
                arguments[0] == str(self.apksigner)
                and Path(arguments[-1]).name.endswith("release-unsigned.apk")
            ):
                return subprocess.CompletedProcess(arguments, 1, stdout="", stderr="tool failure")
            return original(arguments, **kwargs)

        with patch.object(candidate, "_require_clean_source", return_value=COMMIT), patch.object(
            candidate,
            "_find_android_tool",
            side_effect=lambda name, _explicit: self.aapt if name == "aapt" else self.apksigner,
        ), patch.object(candidate, "_run_tool", side_effect=unknown_failure):
            with self.assertRaisesRegex(ValueError, "署名状態を安全に分類"):
                candidate.package_android_product_candidate(
                    self.debug_apk,
                    self.release_apk,
                    self.output,
                    source_commit=COMMIT,
                    flutter_version="3.44.0",
                    java_version="17.0.20",
                    android_build_tools_version="37.0.0",
                )
        self.assertFalse(self.output.exists())

    def test_rejects_signed_release_candidate(self) -> None:
        self._write_apk(self.release_apk, marker=b"signed-release", signed=True)
        with patch.object(candidate, "_require_clean_source", return_value=COMMIT), patch.object(
            candidate,
            "_find_android_tool",
            side_effect=lambda name, _explicit: self.aapt if name == "aapt" else self.apksigner,
        ), patch.object(candidate, "_run_tool", side_effect=self._run_tool):
            with self.assertRaisesRegex(ValueError, "署名なし"):
                candidate.package_android_product_candidate(
                    self.debug_apk,
                    self.release_apk,
                    self.output,
                    source_commit=COMMIT,
                    flutter_version="3.44.0",
                    java_version="17.0.20",
                    android_build_tools_version="37.0.0",
                )
        self.assertFalse(self.output.exists())

    def test_rejects_build_tools_version_mismatch(self) -> None:
        with patch.object(candidate, "_require_clean_source", return_value=COMMIT), patch.object(
            candidate,
            "_find_android_tool",
            side_effect=lambda name, _explicit: self.aapt if name == "aapt" else self.apksigner,
        ), patch.object(candidate, "_run_tool", side_effect=self._run_tool):
            with self.assertRaisesRegex(ValueError, "Build Tools version"):
                candidate.package_android_product_candidate(
                    self.debug_apk,
                    self.release_apk,
                    self.output,
                    source_commit=COMMIT,
                    flutter_version="3.44.0",
                    java_version="17.0.20",
                    android_build_tools_version="36.0.0",
                )
        self.assertFalse(self.output.exists())

    def test_rejects_apk_identity_mismatch_without_publishing(self) -> None:
        def wrong_identity(arguments: list[str], **kwargs: object) -> subprocess.CompletedProcess[str]:
            if arguments[0] == str(self.aapt):
                return subprocess.CompletedProcess(
                    arguments,
                    0,
                    stdout="package: name='com.other.app' versionCode='1' versionName='0.1.0'\n",
                    stderr="",
                )
            return self._run_tool(arguments, **kwargs)

        with patch.object(candidate, "_require_clean_source", return_value=COMMIT), patch.object(
            candidate,
            "_find_android_tool",
            side_effect=lambda name, _explicit: self.aapt if name == "aapt" else self.apksigner,
        ), patch.object(candidate, "_run_tool", side_effect=wrong_identity):
            with self.assertRaisesRegex(ValueError, "identity/versionがAndroid sourceと一致"):
                candidate.package_android_product_candidate(
                    self.debug_apk,
                    self.release_apk,
                    self.output,
                    source_commit=COMMIT,
                    flutter_version="3.44.0",
                    java_version="17.0.20",
                    android_build_tools_version="37.0.0",
                )
        self.assertFalse(self.output.exists())

    def test_refuses_repository_output_path(self) -> None:
        with self.assertRaisesRegex(ValueError, "Repository外"):
            candidate._output_target(ROOT / "d4-pocket-android-candidate-test-output")


if __name__ == "__main__":
    unittest.main()
