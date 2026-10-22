from __future__ import annotations

import hashlib
import json
import os
import sys
import tempfile
import unittest
from types import SimpleNamespace
from pathlib import Path
from typing import Any
from unittest.mock import patch

from tooling.export_linux_product import (
    _linux_build_environment,
    _require_clean_source,
    _toolchain,
    build_linux_product,
)
from tooling.export_windows_product import validate_export_inputs
from tooling.package_linux_product import EXECUTABLE_PATHS
from tooling.schema_check.check_schemas import validate_instance
from tooling.tests.create_linux_export_fixture import linux_export_pair

ROOT = Path(__file__).resolve().parents[2]
APP_ID = "d4-pocket-app-11111111111111111111111111111111"
AUDIT_ID = "audit-store-22222222222222222222222222222222"
PAYLOADS = {
    "data/icudtl.dat": b"icu",
    "data/flutter_assets/AssetManifest.bin": b"assets",
    "gui_shell_desktop": b"runner",
    "gui_shell_desktop_launcher": b"launcher",
    "lib/libflutter_linux_gtk.so": b"flutter",
    "libexec/gui_shell_linux_broker": b"broker",
}


class LinuxExportInputTests(unittest.TestCase):
    def test_broker_shaped_receipt_and_manifest_are_hash_bound_for_linux(self) -> None:
        receipt_raw, manifest_raw = linux_export_pair()
        receipt, manifest, plan, receipt_hash, manifest_hash = validate_export_inputs(
            receipt_raw, manifest_raw, target_platform="linux"
        )
        self.assertEqual(receipt["manifest_file"]["byte_length"], len(manifest_raw))
        self.assertEqual(receipt_hash, hashlib.sha256(receipt_raw).hexdigest())
        self.assertEqual(manifest_hash, hashlib.sha256(manifest_raw).hexdigest())
        self.assertEqual(manifest["manifest"]["app_identity"]["target_platform"], "linux")
        self.assertIn("core.security_broker", plan.included_module_ids)
        self.assertFalse(receipt["credential_inherited"])
        self.assertFalse(receipt["permission_inherited"])
        self.assertFalse(receipt["approval_inherited"])
        self.assertFalse(receipt["audit_chain_inherited"])

    def test_linux_fixture_can_embed_bounded_ephemeral_update_trust(self) -> None:
        public_der = bytes.fromhex("302a300506032b6570032100") + bytes(range(32))
        receipt_raw, manifest_raw = linux_export_pair(public_der)
        _, manifest, _, _, _ = validate_export_inputs(
            receipt_raw, manifest_raw, target_platform="linux"
        )
        trust = manifest["update_trust"]
        self.assertEqual(trust["public_key_der_hex"], public_der.hex())
        self.assertEqual(
            trust["public_key_fingerprint"],
            "sha256:" + hashlib.sha256(public_der[-32:]).hexdigest(),
        )
        self.assertEqual(trust["package_sources"][0]["base_url"], "https://updates.example.invalid/d4/stable")

    def test_linux_fixture_can_pin_update_catalog_to_an_immutable_public_source(self) -> None:
        public_der = bytes.fromhex("302a300506032b6570032100") + bytes(range(32))
        source = (
            "https://raw.githubusercontent.com/gatchimuchio/D4-POCKET/"
            "0123456789abcdef0123456789abcdef01234567/"
            "tooling/tests/fixtures/linux-update-catalog"
        )
        receipt_raw, manifest_raw = linux_export_pair(public_der, source)
        _, manifest, _, _, _ = validate_export_inputs(
            receipt_raw, manifest_raw, target_platform="linux"
        )
        self.assertEqual(
            manifest["update_trust"]["package_sources"][0]["base_url"], source
        )

    def test_wrong_target_and_tampered_manifest_are_rejected(self) -> None:
        receipt_raw, manifest_raw = linux_export_pair()
        receipt = json.loads(receipt_raw)
        receipt["export_manifest"]["app_identity"]["target_platform"] = "windows"
        bad_receipt = json.dumps(receipt, ensure_ascii=False).encode("utf-8")
        with self.assertRaises(ValueError):
            validate_export_inputs(bad_receipt, manifest_raw, target_platform="linux")

        with self.assertRaisesRegex(ValueError, "byte長|SHA-256"):
            validate_export_inputs(receipt_raw, manifest_raw + b" ", target_platform="linux")

    def test_child_environment_drops_credentials_and_user_build_caches(self) -> None:
        inherited = {
            "PATH": "/usr/bin",
            "LANG": "C.UTF-8",
            "GITHUB_TOKEN": "secret-marker",
            "AWS_SECRET_ACCESS_KEY": "secret-marker",
            "CARGO_HOME": "/home/user/.cargo",
            "GUI_SHELL_PRODUCT_APP_ID": "inherited-app-id",
            "GUI_SHELL_UPDATE_TRUST_JSON": "inherited-trust",
        }
        result = _linux_build_environment(
            {
                "HOME": "/tmp/build-home",
                "CARGO_HOME": "/tmp/build-cargo",
                "GUI_SHELL_PRODUCT_APP_ID": APP_ID,
            },
            inherited=inherited,
        )
        self.assertEqual(result["PATH"], "/usr/bin")
        self.assertEqual(result["HOME"], "/tmp/build-home")
        self.assertEqual(result["CARGO_HOME"], "/tmp/build-cargo")
        self.assertEqual(result["GUI_SHELL_PRODUCT_APP_ID"], APP_ID)
        self.assertNotIn("GITHUB_TOKEN", result)
        self.assertNotIn("AWS_SECRET_ACCESS_KEY", result)
        self.assertNotIn("GUI_SHELL_UPDATE_TRUST_JSON", result)

    def test_toolchain_probes_do_not_inherit_credentials(self) -> None:
        inherited = {
            "PATH": "/usr/bin",
            "HOME": "/home/user",
            "GITHUB_TOKEN": "secret-marker",
            "AWS_SECRET_ACCESS_KEY": "secret-marker",
            "RUSTUP_HOME": "/tmp/rustup",
        }
        environment = _linux_build_environment(
            {"HOME": "/tmp/isolated-home", "RUSTUP_HOME": "/tmp/rustup"},
            inherited=inherited,
        )
        paths = {
            "flutter": "/usr/bin/flutter",
            "cargo": "/usr/bin/cargo",
            "rustc": "/usr/bin/rustc",
            "cmake": "/usr/bin/cmake",
            "ninja": "/usr/bin/ninja",
            "pkg-config": "/usr/bin/pkg-config",
            "clang": "/usr/bin/clang",
            "clang++": "/usr/bin/clang++",
        }
        calls: list[dict[str, Any]] = []

        def run(command: list[str], **kwargs: Any) -> SimpleNamespace:
            calls.append(kwargs)
            if "--machine" in command:
                stdout = json.dumps(
                    {
                        "frameworkVersion": "3.44.0",
                        "dartSdkVersion": "3.12.0",
                        "frameworkRevision": "e" * 40,
                    }
                )
            else:
                stdout = "tool version 1.0"
            return SimpleNamespace(stdout=stdout)

        with patch("tooling.export_linux_product.subprocess.run", side_effect=run):
            versions = _toolchain(paths, environment)
        self.assertEqual(versions["framework_revision"], "e" * 40)
        self.assertEqual(len(calls), 8)
        self.assertEqual(calls[0]["env"]["CI"], "true")
        self.assertTrue(all("CI" not in call["env"] for call in calls[1:]))
        self.assertTrue(all("GITHUB_TOKEN" not in call["env"] for call in calls))
        self.assertTrue(all("AWS_SECRET_ACCESS_KEY" not in call["env"] for call in calls))

    def test_source_must_be_clean_and_match_remote_main_or_explicit_codex_ref(self) -> None:
        commit = "a" * 40
        state = {
            ("branch", "--show-current"): "main",
            ("status", "--porcelain", "--untracked-files=all"): "",
            ("rev-parse", "HEAD"): commit,
            ("ls-remote", "origin", "refs/heads/main"): f"{commit}\trefs/heads/main",
        }
        with patch("tooling.export_linux_product._git_value", side_effect=lambda *args: state[args]):
            self.assertEqual(_require_clean_source(), ("main", commit))

        state[("ls-remote", "origin", "refs/heads/main")] = f"{'b' * 40}\trefs/heads/main"
        with patch("tooling.export_linux_product._git_value", side_effect=lambda *args: state[args]):
            with self.assertRaisesRegex(ValueError, "remote branch HEAD"):
                _require_clean_source()

        state[("branch", "--show-current")] = "codex/linux-export-test"
        state[("status", "--porcelain", "--untracked-files=all")] = " M dirty.txt"
        with patch("tooling.export_linux_product._git_value", side_effect=lambda *args: state[args]):
            with self.assertRaisesRegex(ValueError, "clean"):
                _require_clean_source(allow_verification_branch=True)

        state[("status", "--porcelain", "--untracked-files=all")] = ""
        state[("ls-remote", "origin", "refs/heads/codex/linux-export-test")] = (
            f"{commit}\trefs/heads/codex/linux-export-test"
        )
        with patch("tooling.export_linux_product._git_value", side_effect=lambda *args: state[args]):
            self.assertEqual(
                _require_clean_source(allow_verification_branch=True),
                ("codex/linux-export-test", commit),
            )


@unittest.skipUnless(sys.platform.startswith("linux"), "Linux CMake bundle semantics are required")
class LinuxExportBuildTests(unittest.TestCase):
    def _create_fake_bundle(self, bundle: Path) -> None:
        bundle.mkdir(parents=True)
        os.chmod(bundle, 0o755)
        for relative, payload in PAYLOADS.items():
            target = bundle / relative
            target.parent.mkdir(parents=True, exist_ok=True, mode=0o755)
            os.chmod(target.parent, 0o755)
            target.write_bytes(payload)
            os.chmod(target, 0o755 if relative in EXECUTABLE_PATHS else 0o644)

    def _input_files(self, root: Path) -> tuple[Path, Path]:
        receipt_raw, manifest_raw = linux_export_pair()
        receipt = root / "receipt.json"
        manifest = root / f"{APP_ID}.json"
        receipt.write_bytes(receipt_raw)
        manifest.write_bytes(manifest_raw)
        return receipt, manifest

    def test_release_bundle_package_and_provenance_are_published_together(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-linux-export-test-") as temporary:
            root = Path(temporary)
            receipt, manifest = self._input_files(root)
            output = root / "result"
            source_commit = "a" * 40
            tool_paths = {
                "flutter": "/usr/bin/flutter",
                "cargo": "/usr/bin/cargo",
                "rustc": "/usr/bin/rustc",
                "cmake": "/usr/bin/cmake",
                "ninja": "/usr/bin/ninja",
                "pkg-config": "/usr/bin/pkg-config",
                "clang": "/usr/bin/clang",
                "clang++": "/usr/bin/clang++",
            }
            toolchain = {
                "flutter": "3.44.0",
                "dart": "3.12.0",
                "framework_revision": "e" * 40,
                "cargo": "cargo 1.95.0",
                "rustc": "rustc 1.95.0",
                "cmake": "cmake 3.28",
                "ninja": "1.11",
                "pkg_config": "1.8",
                "clang": "clang 18",
                "clangxx": "clang++ 18",
                "distribution_id": "ubuntu",
                "distribution_version": "24.04",
                "architecture": "x86_64",
            }

            def extract_source(_archive: Path, destination: Path) -> None:
                project = destination / "apps" / "desktop_flutter"
                project.mkdir(parents=True)
                (project / "pubspec.yaml").write_text(
                    "name: d4p_fixture\nversion: 1.2.3+4\n", encoding="utf-8"
                )

            configure_args: list[str] = []
            commands: list[list[str]] = []
            command_environments: list[dict[str, str]] = []

            def run(command: list[str], **kwargs: Any) -> None:
                commands.append(command)
                command_environments.append(kwargs["env"])
                if "--build" in command:
                    bundle_arg = next(
                        value.split("=", 1)[1]
                        for value in configure_args
                        if value.startswith("-DCMAKE_INSTALL_PREFIX=")
                    )
                    self._create_fake_bundle(Path(bundle_arg))
                elif "-DCMAKE_INSTALL_PREFIX=" in " ".join(command):
                    configure_args.extend(command)

            def write_archive(_commit: str, archive_path: Path) -> str:
                archive_path.write_bytes(b"synthetic-git-archive")
                return hashlib.sha256(b"synthetic-git-archive").hexdigest()

            with (
                patch("tooling.export_linux_product._require_clean_source", return_value=("main", source_commit)),
                patch("tooling.export_linux_product._tool_paths", return_value=tool_paths),
                patch("tooling.export_linux_product._toolchain", return_value=toolchain),
                patch("tooling.export_linux_product._rustup_home", return_value=str(root)),
                patch("tooling.export_linux_product._write_source_archive", side_effect=write_archive),
                patch("tooling.export_linux_product._safe_extract_source", side_effect=extract_source),
                patch("tooling.export_linux_product.subprocess.run", side_effect=run),
            ):
                evidence = build_linux_product(receipt, manifest, output)

            self.assertTrue((output / "bundle" / "gui_shell_desktop").is_file())
            self.assertTrue((output / "D4Pocket-linux.d4pkg").is_file())
            self.assertTrue((output / "d4_pocket_linux_export_build.json").is_file())
            self.assertEqual(evidence["source_commit"], source_commit)
            self.assertEqual(evidence["compile_time_identity"]["app_id"], APP_ID)
            self.assertEqual(evidence["compile_time_identity"]["audit_store_id"], AUDIT_ID)
            self.assertFalse(evidence["authority_boundary"]["owner_authorization_verified"])
            self.assertFalse(evidence["signed"])
            self.assertFalse(evidence["formal_distribution_claimed"])
            self.assertGreaterEqual(len(commands), 4)
            flutter_commands = [
                (command, environment)
                for command, environment in zip(commands, command_environments, strict=True)
                if command[0] == tool_paths["flutter"]
            ]
            self.assertEqual(len(flutter_commands), 2)
            self.assertTrue(
                all(environment["CI"] == "true" for _, environment in flutter_commands)
            )
            cmake_environments = [
                environment
                for command, environment in zip(commands, command_environments, strict=True)
                if command[0] == tool_paths["cmake"]
            ]
            self.assertTrue(cmake_environments)
            self.assertTrue(
                all("CI" not in environment for environment in cmake_environments)
            )
            cmake_configure = next(command for command in commands if any(
                item.startswith("-DD4_PRODUCT_APP_ID=") for item in command
            ))
            self.assertIn(f"-DD4_PRODUCT_APP_ID={APP_ID}", cmake_configure)
            self.assertIn(f"-DD4_PRODUCT_AUDIT_STORE_ID={AUDIT_ID}", cmake_configure)
            schema = json.loads(
                (ROOT / "specs/gui_shell_linux_export_build.schema.json").read_text(
                    encoding="utf-8"
                )
            )
            self.assertEqual(validate_instance(evidence, schema), [])

    def test_failed_native_build_never_publishes_partial_output(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-linux-export-fail-") as temporary:
            root = Path(temporary)
            receipt, manifest = self._input_files(root)
            output = root / "failed-result"
            with (
                patch("tooling.export_linux_product._require_clean_source", return_value=("main", "a" * 40)),
                patch("tooling.export_linux_product._tool_paths", return_value={
                    name: f"/usr/bin/{name}" for name in (
                        "flutter", "cargo", "rustc", "cmake", "ninja", "pkg-config", "clang", "clang++"
                    )
                }),
                patch("tooling.export_linux_product._toolchain", return_value={
                    "flutter": "3.44.0", "dart": "3.12.0", "framework_revision": "e" * 40,
                    "cargo": "cargo 1.95.0", "rustc": "rustc 1.95.0", "cmake": "cmake 3.28",
                    "ninja": "1.11", "pkg_config": "1.8", "clang": "clang 18",
                    "clangxx": "clang++ 18", "distribution_id": "ubuntu",
                    "distribution_version": "24.04", "architecture": "x86_64",
                }),
                patch("tooling.export_linux_product._rustup_home", return_value=str(root)),
                patch("tooling.export_linux_product._write_source_archive", side_effect=lambda _commit, path: (path.write_bytes(b"archive"), "f" * 64)[1]),
                patch("tooling.export_linux_product._safe_extract_source", side_effect=lambda _archive, target: (
                    (target / "apps/desktop_flutter").mkdir(parents=True),
                    (target / "apps/desktop_flutter/pubspec.yaml").write_text("name: x\nversion: 1.0.0\n", encoding="utf-8"),
                )),
                patch("tooling.export_linux_product.subprocess.run", side_effect=RuntimeError("synthetic build failure")),
            ):
                with self.assertRaisesRegex(RuntimeError, "synthetic build failure"):
                    build_linux_product(receipt, manifest, output)
            self.assertFalse(output.exists())


if __name__ == "__main__":
    unittest.main()
