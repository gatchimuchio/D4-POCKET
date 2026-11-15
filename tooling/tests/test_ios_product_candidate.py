from __future__ import annotations

import plistlib
import subprocess
import tempfile
import unittest
import zipfile
from pathlib import Path
from unittest.mock import patch

from tooling import package_ios_product_candidate as candidate


class IosProductCandidateTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory(prefix="d4p-ios-candidate-test-")
        self.root = Path(self.temp.name)
        self.output_parent = self.root / "output-parent"
        self.output_parent.mkdir()
        self.simulator = self._app("simulator", b"simulator executable")
        self.device = self._app("device", b"device executable")
        self.output = self.output_parent / "candidate"
        self._codesign_result = subprocess.CompletedProcess(
            ["/usr/bin/codesign"],
            1,
            stdout="",
            stderr="code object is not signed at all",
        )
        self._repeat_platform = False
        self._platform = patch.object(candidate.platform, "system", return_value="Darwin")
        self._which = patch.object(candidate.shutil, "which", side_effect=self._which_tool)
        self._run = patch.object(candidate.subprocess, "run", side_effect=self._run_tool)
        self._platform.start()
        self._which.start()
        self._run.start()
        self.addCleanup(self._platform.stop)
        self.addCleanup(self._which.stop)
        self.addCleanup(self._run.stop)

    def tearDown(self) -> None:
        self.temp.cleanup()

    def _app(self, name: str, executable: bytes, bundle_id: str = "com.example.guiShellMobile") -> Path:
        bundle = self.root / name / "Runner.app"
        bundle.mkdir(parents=True)
        (bundle / "Runner").write_bytes(executable)
        with (bundle / "Info.plist").open("wb") as handle:
            plistlib.dump(
                {"CFBundleIdentifier": bundle_id, "CFBundleExecutable": "Runner"},
                handle,
            )
        return bundle

    @staticmethod
    def _which_tool(name: str) -> str | None:
        return {
            "codesign": "/usr/bin/codesign",
            "xcrun": "/usr/bin/xcrun",
            "ditto": "/usr/bin/ditto",
        }.get(name)

    def _run_tool(self, arguments: list[str], **kwargs: object) -> subprocess.CompletedProcess[str]:
        if arguments[0] == "/usr/bin/codesign":
            return self._codesign_result
        if arguments[1:3] == ["lipo", "-archs"]:
            return subprocess.CompletedProcess(arguments, 0, stdout="arm64\n", stderr="")
        if arguments[1:3] == ["vtool", "-show-build"]:
            platform = "IOSSIMULATOR" if "simulator" in str(arguments[-1]).lower() else "IOS"
            count = 2 if self._repeat_platform else 1
            output = "".join(f"platform {platform}\n" for _ in range(count))
            return subprocess.CompletedProcess(arguments, 0, stdout=output, stderr="")
        if arguments[0] == "/usr/bin/ditto":
            source = Path(arguments[-2])
            destination = Path(arguments[-1])
            with zipfile.ZipFile(destination, "w", compression=zipfile.ZIP_DEFLATED) as archive:
                for path in sorted(source.rglob("*")):
                    if path.is_file() and not path.is_symlink():
                        archive.write(path, f"{source.name}/{path.relative_to(source).as_posix()}")
            return subprocess.CompletedProcess(arguments, 0, stdout="", stderr="")
        raise AssertionError(f"unexpected command: {arguments}")

    def _build(self) -> dict[str, object]:
        return candidate.build_ios_product_candidate(
            self.simulator,
            self.device,
            self.output,
            source_commit="a" * 40,
            flutter_version="3.44.0",
            xcode_version="16.4",
        )

    def test_creates_distinct_simulator_and_device_candidate_archives(self) -> None:
        evidence = self._build()
        self.assertTrue(self.output.is_dir())
        self.assertEqual(evidence["distribution_ready"], False)
        self.assertEqual(evidence["formal_distribution_claimed"], False)
        self.assertEqual(evidence["simulator_artifact"]["build_configuration"], "debug")
        self.assertEqual(evidence["device_artifact"]["build_configuration"], "release")
        self.assertEqual(
            evidence["simulator_artifact"]["app_bundle_signature_state"],
            "unsigned",
        )
        self.assertEqual(
            evidence["device_artifact"]["app_bundle_signature_state"],
            "unsigned",
        )
        self.assertNotEqual(
            evidence["simulator_artifact"]["sha256"],
            evidence["device_artifact"]["sha256"],
        )
        self.assertEqual(
            {path.name for path in self.output.iterdir()},
            {
                "d4-pocket-ios-simulator-app.zip",
                "d4-pocket-ios-device-app.zip",
                "ios-product-candidate.json",
            },
        )
        self.assertFalse(any(path.suffix == ".ipa" for path in self.output.iterdir()))

    def test_rejects_bundle_identifier_mismatch_without_publishing(self) -> None:
        self.device = self._app("device-other-id", b"device executable", "com.other.product")
        with self.assertRaisesRegex(ValueError, "Bundle Identifier"):
            self._build()
        self.assertFalse(self.output.exists())

    def test_rejects_developer_identity_and_provisioning_profile(self) -> None:
        (self.device / "_CodeSignature").mkdir()
        self._codesign_result = subprocess.CompletedProcess(
            ["/usr/bin/codesign"],
            0,
            stdout="Authority=Apple Development: private identity\nTeamIdentifier=ABCDE12345\n",
            stderr="",
        )
        with self.assertRaisesRegex(ValueError, "identity"):
            self._build()
        (self.device / "_CodeSignature").rmdir()
        self._codesign_result = subprocess.CompletedProcess(
            ["/usr/bin/codesign"],
            1,
            stdout="",
            stderr="code object is not signed at all",
        )
        (self.device / "embedded.mobileprovision").write_bytes(b"fixture")
        with self.assertRaisesRegex(ValueError, "provisioning"):
            self._build()
        self.assertFalse(self.output.exists())

    def test_records_linker_adhoc_signature_without_claiming_distribution_identity(self) -> None:
        self._codesign_result = subprocess.CompletedProcess(
            ["/usr/bin/codesign"],
            0,
            stdout=(
                "CodeDirectory v=20400 flags=0x20002(adhoc,linker-signed)\n"
                "Signature=adhoc\nTeamIdentifier=not set\n"
            ),
            stderr="",
        )
        evidence = self._build()
        self.assertEqual(
            evidence["simulator_artifact"]["app_bundle_signature_state"],
            "adhoc_no_identity",
        )
        self.assertEqual(
            evidence["device_artifact"]["app_bundle_signature_state"],
            "adhoc_no_identity",
        )
        self.assertEqual(evidence["bundle_identity"]["distribution_identity"], False)
        self.assertEqual(evidence["distribution_ready"], False)

    def test_accepts_repeated_platform_commands_when_all_match_target(self) -> None:
        self._repeat_platform = True
        evidence = self._build()
        self.assertEqual(evidence["simulator_artifact"]["platform"], "IOSSIMULATOR")
        self.assertEqual(evidence["device_artifact"]["platform"], "IOS")

    def test_accepts_code_resources_directory_when_codesign_reports_unsigned(self) -> None:
        signature_metadata = self.device / "_CodeSignature"
        signature_metadata.mkdir()
        (signature_metadata / "CodeResources").write_text("unsigned fixture", encoding="utf-8")
        evidence = self._build()
        self.assertEqual(
            evidence["device_artifact"]["app_bundle_signature_state"],
            "unsigned",
        )

    def test_fails_closed_when_codesign_cannot_classify_bundle(self) -> None:
        self._codesign_result = subprocess.CompletedProcess(
            ["/usr/bin/codesign"], 1, stdout="", stderr="codesign tool failure"
        )
        with self.assertRaisesRegex(ValueError, "signature状態を分類できない"):
            self._build()
        self.assertFalse(self.output.exists())

    def test_rejects_missing_arm64_slice(self) -> None:
        def no_arm64(arguments: list[str], **kwargs: object) -> subprocess.CompletedProcess[str]:
            if arguments[0] == "/usr/bin/codesign":
                return self._codesign_result
            if arguments[1:3] == ["lipo", "-archs"]:
                return subprocess.CompletedProcess(arguments, 0, stdout="x86_64\n", stderr="")
            raise AssertionError(f"unexpected command: {arguments}")

        self._run.stop()
        self._run = patch.object(
            candidate.subprocess,
            "run",
            side_effect=no_arm64,
        )
        self._run.start()
        self.addCleanup(self._run.stop)
        with self.assertRaisesRegex(ValueError, "arm64"):
            self._build()
        self.assertFalse(self.output.exists())

    def test_rejects_wrong_apple_platform(self) -> None:
        def wrong_platform(arguments: list[str], **kwargs: object) -> subprocess.CompletedProcess[str]:
            if arguments[0] == "/usr/bin/codesign":
                return self._codesign_result
            if arguments[1:3] == ["lipo", "-archs"]:
                return subprocess.CompletedProcess(arguments, 0, stdout="arm64\n", stderr="")
            if arguments[1:3] == ["vtool", "-show-build"]:
                return subprocess.CompletedProcess(arguments, 0, stdout="platform IOS\n", stderr="")
            raise AssertionError(f"unexpected command: {arguments}")

        with patch.object(candidate.subprocess, "run", side_effect=wrong_platform):
            with self.assertRaisesRegex(ValueError, "platformが不一致"):
                self._build()
        self.assertFalse(self.output.exists())

    def test_rejects_mixed_platform_commands(self) -> None:
        def mixed_platform(arguments: list[str], **kwargs: object) -> subprocess.CompletedProcess[str]:
            if arguments[0] == "/usr/bin/codesign":
                return self._codesign_result
            if arguments[1:3] == ["lipo", "-archs"]:
                return subprocess.CompletedProcess(arguments, 0, stdout="arm64\n", stderr="")
            if arguments[1:3] == ["vtool", "-show-build"]:
                return subprocess.CompletedProcess(
                    arguments,
                    0,
                    stdout="platform IOSSIMULATOR\nplatform IOS\n",
                    stderr="",
                )
            raise AssertionError(f"unexpected command: {arguments}")

        with patch.object(candidate.subprocess, "run", side_effect=mixed_platform):
            with self.assertRaisesRegex(ValueError, r"observed=IOS,IOSSIMULATOR"):
                self._build()
        self.assertFalse(self.output.exists())

    def test_refuses_reused_output_directory(self) -> None:
        self.output.mkdir()
        with self.assertRaisesRegex(ValueError, "既に存在"):
            self._build()
        self.assertEqual(list(self.output.iterdir()), [])

    def test_refuses_non_macos_build_host(self) -> None:
        with patch.object(candidate.platform, "system", return_value="Windows"):
            with self.assertRaisesRegex(ValueError, "macOS native"):
                self._build()
        self.assertFalse(self.output.exists())

    def test_refuses_output_inside_repository(self) -> None:
        inside = candidate.ROOT / "generated-ios-candidate"
        with patch.object(candidate, "ROOT", self.root):
            inside = self.root / "generated-ios-candidate"
            with self.assertRaisesRegex(ValueError, "Repository外"):
                candidate.build_ios_product_candidate(
                    self.simulator,
                    self.device,
                    inside,
                    source_commit="a" * 40,
                    flutter_version="3.44.0",
                    xcode_version="16.4",
                )
        self.assertFalse(inside.exists())


if __name__ == "__main__":
    unittest.main()
