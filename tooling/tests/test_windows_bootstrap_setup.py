from __future__ import annotations

import hashlib
import json
import os
import struct
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from tooling import build_windows_installer as setup_builder


APP_ID = "d4-pocket-app-11111111111111111111111111111111"
AUDIT_STORE_ID = "audit-store-22222222222222222222222222222222"
SOURCE_COMMIT = "a" * 40


def _has_iscc() -> bool:
    if sys.platform != "win32":
        return False
    try:
        setup_builder._find_iscc(None)
        return True
    except ValueError:
        return False


class WindowsBootstrapSetupTests(unittest.TestCase):
    @staticmethod
    def pe_bytes(machine: int) -> bytes:
        content = bytearray(0x80 + 6)
        content[:2] = b"MZ"
        struct.pack_into("<I", content, 0x3C, 0x80)
        content[0x80:0x84] = b"PE\0\0"
        struct.pack_into("<H", content, 0x84, machine)
        return bytes(content)

    def create_bundle(self, root: Path) -> Path:
        bundle = root / "export bundle with spaces"
        bundle.mkdir()
        (bundle / setup_builder.RECEIPT_NAME).write_text(
            json.dumps(
                {
                    "source_commit": SOURCE_COMMIT,
                    "app_id": APP_ID,
                    "audit_store_id": AUDIT_STORE_ID,
                }
            ),
            encoding="utf-8",
        )
        (bundle / "product_manifest.json").write_text(
            json.dumps(
                {
                    "version": 1,
                    "product": "D4 Pocket",
                    "export_id": "export-test-01",
                    "manifest": {
                        "app_identity": {"app_id": APP_ID},
                        "audit_store": {"store_id": AUDIT_STORE_ID},
                    },
                }
            ),
            encoding="utf-8",
        )
        (bundle / "gui_shell_desktop_launcher.exe").write_bytes(
            APP_ID.encode("ascii") + b"\0" + AUDIT_STORE_ID.encode("ascii")
        )
        return bundle

    def validate_bundle(self, bundle: Path, source_commit: str = SOURCE_COMMIT):
        with (
            patch.object(setup_builder, "validate_build_evidence"),
            patch.object(setup_builder, "_require_clean_source", return_value=source_commit),
            patch.object(setup_builder, "validate_instance", return_value=[]),
            patch.object(setup_builder, "_schema", return_value={}),
            patch.object(setup_builder, "product_version_from_source", return_value="1.2.3+4"),
        ):
            return setup_builder.validate_export_bundle(bundle)

    def test_export_identity_and_clean_main_are_bound_before_setup_build(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-setup-test-") as temporary:
            bundle = self.create_bundle(Path(temporary))
            result = self.validate_bundle(bundle)

        self.assertEqual(result, (SOURCE_COMMIT, APP_ID, AUDIT_STORE_ID, "1.2.3+4"))

    def test_bundle_with_manifest_identity_mismatch_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-setup-test-") as temporary:
            bundle = self.create_bundle(Path(temporary))
            manifest = json.loads((bundle / "product_manifest.json").read_text(encoding="utf-8"))
            manifest["manifest"]["app_identity"]["app_id"] = "d4-pocket-app-" + "3" * 32
            (bundle / "product_manifest.json").write_text(json.dumps(manifest), encoding="utf-8")

            with self.assertRaisesRegex(ValueError, "identityがExport Receiptと一致しない"):
                self.validate_bundle(bundle)

    def test_bundle_from_another_commit_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-setup-test-") as temporary:
            bundle = self.create_bundle(Path(temporary))

            with self.assertRaisesRegex(ValueError, "source commitが現在のGitHub mainと一致しない"):
                self.validate_bundle(bundle, "b" * 40)

    def test_launcher_compile_time_identity_mismatch_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-setup-test-") as temporary:
            bundle = self.create_bundle(Path(temporary))
            (bundle / "gui_shell_desktop_launcher.exe").write_bytes(b"wrong identity")

            with self.assertRaisesRegex(ValueError, "compile-time identityがManifestと一致しない"):
                self.validate_bundle(bundle)

    def test_repository_path_cannot_be_used_as_export_input_or_output(self) -> None:
        with self.assertRaisesRegex(ValueError, "Repository／OneDriveの外"):
            setup_builder._outside_source_roots(setup_builder.ROOT, "Export bundle")

    def test_ispp_define_rejects_quote_and_newline_injection(self) -> None:
        with self.assertRaisesRegex(ValueError, "不正な文字"):
            setup_builder._ispp_string('C:\\bundle"\n#define Malicious')

    def test_pe_architecture_is_read_from_machine_field(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-pe-header-") as temporary:
            executable = Path(temporary) / "payload.exe"
            executable.write_bytes(self.pe_bytes(0x8664))

            self.assertEqual(setup_builder._pe_architecture(executable), "x64")

            executable.write_bytes(self.pe_bytes(0xAA64))
            self.assertEqual(setup_builder._pe_architecture(executable), "arm64")

    def test_malformed_or_unknown_pe_architecture_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-pe-header-") as temporary:
            executable = Path(temporary) / "payload.exe"
            executable.write_bytes(b"X" * 64)
            with self.assertRaisesRegex(ValueError, "DOS header"):
                setup_builder._pe_architecture(executable)

            executable.write_bytes(self.pe_bytes(0x01C4))
            with self.assertRaisesRegex(ValueError, "未対応"):
                setup_builder._pe_architecture(executable)

    def test_receipt_bound_payload_components_capture_arch_hash_and_signature_state(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-installer-components-") as temporary:
            bundle = Path(temporary)
            records = []
            payload = self.pe_bytes(0x8664)
            for _role, relative in setup_builder.WINDOWS_PAYLOAD_COMPONENTS:
                target = bundle.joinpath(*relative.split("/"))
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(payload)
                records.append(
                    {
                        "path": relative,
                        "size_bytes": len(payload),
                        "sha256": hashlib.sha256(payload).hexdigest(),
                    }
                )

            with patch.object(
                setup_builder,
                "_authenticode_info",
                return_value={"status": "NotSigned", "signer_thumbprint": None},
            ):
                components = setup_builder._inspect_payload_components(bundle, records)

        self.assertEqual([component["role"] for component in components], [
            "flutter_application", "rust_broker", "rust_launcher"
        ])
        self.assertTrue(all(component["architecture"] == "x64" for component in components))
        self.assertTrue(all(component["authenticode_status"] == "NotSigned" for component in components))
        self.assertTrue(all(component["signer_thumbprint"] is None for component in components))

    def test_payload_component_architecture_and_receipt_hash_must_match(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-installer-components-") as temporary:
            bundle = Path(temporary)
            records = []
            for index, (_role, relative) in enumerate(setup_builder.WINDOWS_PAYLOAD_COMPONENTS):
                payload = self.pe_bytes(0xAA64 if index == 0 else 0x8664)
                target = bundle.joinpath(*relative.split("/"))
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(payload)
                records.append(
                    {
                        "path": relative,
                        "size_bytes": len(payload),
                        "sha256": hashlib.sha256(payload).hexdigest(),
                    }
                )

            with patch.object(
                setup_builder,
                "_authenticode_info",
                return_value={"status": "NotSigned", "signer_thumbprint": None},
            ):
                with self.assertRaisesRegex(ValueError, "architectureはx64必須"):
                    setup_builder._inspect_payload_components(bundle, records)

                records[0]["sha256"] = "0" * 64
                with self.assertRaisesRegex(ValueError, "hash／sizeと一致しない"):
                    setup_builder._inspect_payload_components(bundle, records)

    def test_payload_change_during_authenticode_check_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-installer-components-") as temporary:
            bundle = Path(temporary)
            records = []
            payload = self.pe_bytes(0x8664)
            for _role, relative in setup_builder.WINDOWS_PAYLOAD_COMPONENTS:
                target = bundle.joinpath(*relative.split("/"))
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(payload)
                records.append(
                    {
                        "path": relative,
                        "size_bytes": len(payload),
                        "sha256": hashlib.sha256(payload).hexdigest(),
                    }
                )
            first_component = bundle.joinpath(
                *setup_builder.WINDOWS_PAYLOAD_COMPONENTS[0][1].split("/")
            )

            def mutate_during_signature_check(_path: Path) -> dict[str, str | None]:
                first_component.write_bytes(payload[:-1] + b"X")
                return {"status": "NotSigned", "signer_thumbprint": None}

            with patch.object(
                setup_builder,
                "_authenticode_info",
                side_effect=mutate_during_signature_check,
            ):
                with self.assertRaisesRegex(ValueError, "Authenticode確認中にWindows componentが変化"):
                    setup_builder._inspect_payload_components(bundle, records)

    def test_invalid_authenticode_state_fails_closed_but_unsigned_is_observable(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-authenticode-") as temporary:
            executable = Path(temporary) / "payload.exe"
            executable.write_bytes(self.pe_bytes(0x8664))
            not_signed = subprocess.CompletedProcess(
                args=[],
                returncode=0,
                stdout='{"status":"NotSigned","signer_thumbprint":null}',
                stderr="",
            )
            with (
                patch.object(
                    setup_builder,
                    "_windows_powershell_path",
                    return_value=Path(
                        r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe"
                    ),
                ),
                patch.object(
                    setup_builder.subprocess, "run", return_value=not_signed
                ) as run_authenticode,
            ):
                self.assertEqual(
                    setup_builder._authenticode_info(executable),
                    {"status": "NotSigned", "signer_thumbprint": None},
                )
            self.assertEqual(
                run_authenticode.call_args.kwargs["env"]["PSModulePath"],
                str(
                    Path(os.environ.get("SystemRoot", r"C:\Windows"))
                    / "System32"
                    / "WindowsPowerShell"
                    / "v1.0"
                    / "Modules"
                ),
            )
            self.assertEqual(
                run_authenticode.call_args.args[0][0],
                r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe",
            )

            valid_thumbprint = "a" * 40
            valid = subprocess.CompletedProcess(
                args=[],
                returncode=0,
                stdout=json.dumps(
                    {"status": "Valid", "signer_thumbprint": valid_thumbprint}
                ),
                stderr="",
            )
            with (
                patch.object(
                    setup_builder,
                    "_windows_powershell_path",
                    return_value=Path(
                        r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe"
                    ),
                ),
                patch.object(setup_builder.subprocess, "run", return_value=valid),
            ):
                self.assertEqual(
                    setup_builder._authenticode_info(executable),
                    {"status": "Valid", "signer_thumbprint": valid_thumbprint.upper()},
                )

            invalid = subprocess.CompletedProcess(
                args=[],
                returncode=0,
                stdout='{"status":"HashMismatch","signer_thumbprint":"' + "A" * 40 + '"}',
                stderr="",
            )
            with (
                patch.object(
                    setup_builder,
                    "_windows_powershell_path",
                    return_value=Path(
                        r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe"
                    ),
                ),
                patch.object(setup_builder.subprocess, "run", return_value=invalid),
            ):
                with self.assertRaisesRegex(ValueError, "署名が無効"):
                    setup_builder._authenticode_info(executable)

    def test_setup_snapshot_copies_only_receipt_bound_artifacts(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-setup-snapshot-") as temporary:
            root = Path(temporary)
            source = root / "source"
            snapshot = root / "snapshot"
            source.mkdir()
            payload = b"receipt-bound payload"
            (source / "payload.bin").write_bytes(payload)
            (source / "unlisted.tmp").write_bytes(b"must not be bundled")
            receipt = b'{"version":1}'

            setup_builder._snapshot_export_bundle(
                source,
                snapshot,
                [
                    {
                        "path": "payload.bin",
                        "size_bytes": len(payload),
                        "sha256": hashlib.sha256(payload).hexdigest(),
                    }
                ],
                receipt,
            )

            self.assertEqual((snapshot / "payload.bin").read_bytes(), payload)
            self.assertEqual((snapshot / setup_builder.RECEIPT_NAME).read_bytes(), receipt)
            self.assertFalse((snapshot / "unlisted.tmp").exists())

    def test_setup_snapshot_rejects_artifact_hash_change(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-setup-snapshot-") as temporary:
            root = Path(temporary)
            source = root / "source"
            source.mkdir()
            (source / "payload.bin").write_bytes(b"changed")

            with self.assertRaisesRegex(ValueError, "Receipt hash／sizeから変化した"):
                setup_builder._snapshot_export_bundle(
                    source,
                    root / "snapshot",
                    [
                        {
                            "path": "payload.bin",
                            "size_bytes": len(b"changed"),
                            "sha256": hashlib.sha256(b"bound").hexdigest(),
                        }
                    ],
                    b'{"version":1}',
                )

    def test_recipe_delegates_product_install_and_disables_its_own_uninstaller(self) -> None:
        script = setup_builder.ISS_SCRIPT.read_text(encoding="utf-8")
        self.assertIn("PrivilegesRequired=lowest", script)
        self.assertIn("Uninstallable=no", script)
        self.assertIn("CreateUninstallRegKey=no", script)
        self.assertNotIn("AppPublisher=", script)
        self.assertIn("DefaultDirName={tmp}\\D4PocketBootstrap-", script)
        self.assertIn("Flags: waituntilterminated runasoriginaluser", script)
        self.assertNotIn("{localappdata}", script.lower())
        self.assertNotIn("[Registry]", script)

    @unittest.skipUnless(_has_iscc(), "Inno Setup 6 ISCC is required")
    def test_inno_setup_compiles_temp_bootstrap_bundle(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-setup-compile-") as temporary:
            root = Path(temporary)
            bundle = root / "bundle path with spaces"
            output = root / "output"
            bundle.mkdir()
            output.mkdir()
            files = (
                "app/gui_shell_desktop.exe",
                "app/flutter_windows.dll",
                "app/data/app.so",
                "app/data/icudtl.dat",
                "app/data/flutter_assets/AssetManifest.bin",
                "broker/gui_shell_rust_helper.exe",
                "gui_shell_desktop_launcher.exe",
                "product_manifest.json",
            )
            for relative in files:
                target = bundle / relative
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(b"fixture payload")
            wrapper = root / "D4Pocket-Setup.iss"
            wrapper.write_text(
                "\n".join(
                    (
                        f"#define BundleDir {setup_builder._ispp_string(str(bundle))}",
                        f"#define ProductAppId {setup_builder._ispp_string(APP_ID)}",
                        f"#define ProductVersion {setup_builder._ispp_string('1.2.3+4')}",
                        f'#include "{setup_builder.ISS_SCRIPT.as_posix()}"',
                        "",
                    )
                ),
                encoding="utf-8",
            )
            result = subprocess.run(
                [str(setup_builder._find_iscc(None)), f"-O{output}", str(wrapper)],
                check=False,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                text=True,
                encoding="utf-8",
                errors="replace",
            )

            self.assertEqual(result.returncode, 0, result.stdout)
            setup_exe = output / "D4Pocket-Setup-1.2.3+4.exe"
            self.assertTrue(setup_exe.is_file())
            self.assertGreater(setup_exe.stat().st_size, 1024)


if __name__ == "__main__":
    unittest.main()
