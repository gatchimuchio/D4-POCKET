from __future__ import annotations

import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from tooling.package_linux_deb_setup import (
    DESKTOP_CONTENT,
    PACKAGE_NAME,
    _parse_dependencies,
    write_debian_setup_package,
)

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
    "product_manifest.json": json.dumps(
        {
            "version": 1,
            "product": "D4 Pocket",
            "export_id": "export-linux-test",
            "manifest": {
                "app_identity": {"app_id": APP_ID, "target_platform": "linux"},
                "audit_store": {
                    "store_id": AUDIT_ID,
                    "chain_status": "new",
                    "inherited": False,
                },
                "distribution_metadata": {"target_platform": "linux"},
                "inheritance_policy": {
                    "authority": "none",
                    "permission": "none",
                    "approval": "none",
                    "credential": "none",
                    "audit_chain": "none",
                },
            },
        },
        separators=(",", ":"),
    ).encode("utf-8"),
}
DEPENDENCIES = "libc6 (>= 2.38), libgtk-3-0t64 (>= 3.24.41)"


@unittest.skipUnless(
    sys.platform.startswith("linux") and shutil.which("dpkg-deb"),
    "Linux dpkg-deb is required for Debian package validation",
)
class LinuxDebSetupPackageTests(unittest.TestCase):
    def test_cli_requires_explicit_build_inputs(self) -> None:
        result = subprocess.run(
            [sys.executable, str(ROOT / "tooling/package_linux_deb_setup.py")],
            check=False,
            capture_output=True,
            text=True,
            encoding="utf-8",
        )
        self.assertEqual(result.returncode, 2)
        self.assertIn("--source-dir", result.stderr)

    def create_export(self, root: Path) -> tuple[Path, Path]:
        output = root / "export"
        bundle = output / "bundle"
        bundle.mkdir(parents=True)
        for relative, payload in PAYLOADS.items():
            target = bundle / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(payload)
            target.chmod(
                0o755
                if relative
                in {
                    "gui_shell_desktop",
                    "gui_shell_desktop_launcher",
                    "libexec/gui_shell_linux_broker",
                }
                else 0o644
            )
        product_package = output / "D4Pocket-linux.d4pkg"
        product_package.write_bytes(b"D4PKG01\nsynthetic")
        evidence = {
            "evidence_type": "d4_pocket_linux_export_build",
            "source_commit": "a" * 40,
            "artifact_tree_sha256": hashlib.sha256(b"synthetic").hexdigest(),
            "package": {
                "file_name": product_package.name,
                "format": "D4PKG01",
                "byte_length": product_package.stat().st_size,
                "sha256": hashlib.sha256(product_package.read_bytes()).hexdigest(),
            },
            "portable_archive": None,
        }
        evidence_path = output / "d4_pocket_linux_export_build.json"
        evidence_path.write_text(json.dumps(evidence), encoding="utf-8")
        return bundle, evidence_path

    def build_package(self, root: Path, revision: int, output_name: str | None = None) -> tuple[Path, dict[str, object]]:
        bundle, evidence = self.create_export(root)
        output_dir = root / (output_name or f"deb-{revision}")
        with (
            patch("tooling.package_linux_deb_setup._require_supported_target"),
            patch("tooling.package_linux_deb_setup._require_source_checkout"),
            patch("tooling.package_linux_deb_setup.product_version_from_source", return_value="1.2.3+4"),
            patch("tooling.package_linux_deb_setup._collect_dependencies", return_value=(DEPENDENCIES, "")),
            patch("tooling.package_linux_deb_setup.validate_build_evidence"),
        ):
            result = write_debian_setup_package(
                bundle,
                evidence,
                output_dir,
                package_revision=revision,
            )
        return Path(str(result["output_path"])), result

    def dpkg_field(self, package: Path, field: str) -> str:
        result = subprocess.run(
            ["dpkg-deb", "--field", str(package), field],
            check=False,
            capture_output=True,
            text=True,
            encoding="utf-8",
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        return result.stdout

    def test_package_is_reproducible_scoped_and_has_no_maintainer_scripts(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-linux-deb-package-") as temporary:
            root = Path(temporary)
            package_one, result = self.build_package(root / "first", 1, "deb-one")
            package_two, _ = self.build_package(root / "second", 1, "deb-two")

            self.assertEqual(package_one.read_bytes(), package_two.read_bytes())
            self.assertEqual(result["package_sha256"], hashlib.sha256(package_one.read_bytes()).hexdigest())
            self.assertEqual(result["scope"], "first_run_setup_bootstrap_only")
            self.assertFalse(result["signed"])
            self.assertEqual(self.dpkg_field(package_one, "Package").strip(), PACKAGE_NAME)
            self.assertEqual(self.dpkg_field(package_one, "Version").strip(), "1.2.3+4-1")
            self.assertEqual(self.dpkg_field(package_one, "Architecture").strip(), "amd64")
            self.assertEqual(
                _parse_dependencies("shlibs:Depends=" + self.dpkg_field(package_one, "Depends").replace("\n ", " ").strip()),
                DEPENDENCIES,
            )

            listing = subprocess.run(
                ["dpkg-deb", "--contents", str(package_one)],
                check=False,
                capture_output=True,
                text=True,
                encoding="utf-8",
            )
            self.assertEqual(listing.returncode, 0, listing.stderr)
            self.assertTrue(listing.stdout.strip())
            self.assertTrue(all("root/root" in line for line in listing.stdout.splitlines() if line.strip()))
            self.assertIn("./opt/d4-pocket/setup/gui_shell_desktop_launcher", listing.stdout)
            self.assertIn("./usr/share/applications/d4-pocket-setup.desktop", listing.stdout)
            self.assertNotIn("/home/", listing.stdout)

            with tempfile.TemporaryDirectory(prefix="d4p-linux-deb-control-") as control_tmp:
                control_dir = Path(control_tmp) / "control"
                control_dir.mkdir()
                control = subprocess.run(
                    ["dpkg-deb", "--control", str(package_one), str(control_dir)],
                    check=False,
                    capture_output=True,
                    text=True,
                    encoding="utf-8",
                )
                self.assertEqual(control.returncode, 0, control.stderr)
                self.assertLessEqual({path.name for path in control_dir.iterdir()}, {"control", "md5sums"})
                self.assertFalse(
                    {"preinst", "postinst", "prerm", "postrm", "config"}
                    & {path.name for path in control_dir.iterdir()}
                )

    def test_export_evidence_failure_and_unsafe_dependencies_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-linux-deb-negative-") as temporary:
            root = Path(temporary)
            bundle, evidence = self.create_export(root)
            output_dir = root / "deb"
            with (
                patch("tooling.package_linux_deb_setup._require_supported_target"),
                patch("tooling.package_linux_deb_setup._require_source_checkout"),
                patch("tooling.package_linux_deb_setup.product_version_from_source", return_value="1.2.3"),
                patch("tooling.package_linux_deb_setup.validate_build_evidence", side_effect=ValueError("artifact mismatch")),
            ):
                with self.assertRaisesRegex(ValueError, "artifact mismatch"):
                    write_debian_setup_package(bundle, evidence, output_dir, package_revision=1)
            self.assertFalse(output_dir.exists())

            with (
                patch("tooling.package_linux_deb_setup._require_supported_target"),
                patch("tooling.package_linux_deb_setup._require_source_checkout"),
                patch("tooling.package_linux_deb_setup.product_version_from_source", return_value="1.2.3"),
                patch(
                    "tooling.package_linux_deb_setup._collect_dependencies",
                    side_effect=lambda *_args: (
                        _parse_dependencies("shlibs:Depends=libc6; touch /tmp/bad"),
                        "",
                    ),
                ),
                patch("tooling.package_linux_deb_setup.validate_build_evidence"),
            ):
                with self.assertRaisesRegex(ValueError, "不正なpackage expression"):
                    write_debian_setup_package(bundle, evidence, output_dir, package_revision=1)
            self.assertFalse(list(output_dir.glob("*.deb")))

    def test_symlink_and_existing_output_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-linux-deb-links-") as temporary:
            root = Path(temporary)
            bundle, evidence = self.create_export(root)
            external = root / "outside"
            external.write_bytes(b"outside")
            (bundle / "data/leak").symlink_to(external)
            with (
                patch("tooling.package_linux_deb_setup._require_supported_target"),
                patch("tooling.package_linux_deb_setup._require_source_checkout"),
                patch("tooling.package_linux_deb_setup.validate_build_evidence"),
            ):
                with self.assertRaisesRegex(ValueError, "symlink"):
                    write_debian_setup_package(bundle, evidence, root / "deb", package_revision=1)
            (bundle / "data/leak").unlink()

            package, _ = self.build_package(root / "existing", 1, "deb-existing")
            bundle, evidence = self.create_export(root / "second")
            with (
                patch("tooling.package_linux_deb_setup._require_supported_target"),
                patch("tooling.package_linux_deb_setup._require_source_checkout"),
                patch("tooling.package_linux_deb_setup.product_version_from_source", return_value="1.2.3+4"),
                patch("tooling.package_linux_deb_setup._collect_dependencies", return_value=(DEPENDENCIES, "")),
                patch("tooling.package_linux_deb_setup.validate_build_evidence"),
            ):
                with self.assertRaisesRegex(ValueError, "already exists"):
                    write_debian_setup_package(bundle, evidence, package.parent, package_revision=1)

    def test_dependency_parser_rejects_control_injection(self) -> None:
        for value in (
            "shlibs:Depends=libc6\nDepends: attacker",
            "shlibs:Depends=libc6; touch /tmp/x",
            "shlibs:Depends=",
        ):
            with self.subTest(value=value), self.assertRaises(ValueError):
                _parse_dependencies(value)


if __name__ == "__main__":
    unittest.main()
