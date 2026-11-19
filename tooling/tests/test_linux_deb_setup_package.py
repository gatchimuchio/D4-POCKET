from __future__ import annotations

import hashlib
import json
import os
import re
import shutil
import stat
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from tooling.package_linux_deb_setup import (
    DESKTOP_CONTENT,
    PACKAGE_NAME,
    _control_text,
    _parse_dependencies,
    main,
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
                "app_identity": {
                    "app_id": APP_ID,
                    "display_name": "D4 Pocket",
                    "target_platform": "linux",
                },
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


class ControlFieldTests(unittest.TestCase):
    def test_dependency_folding_preserves_group_separators(self) -> None:
        dependencies = ", ".join(f"libexample-{index} (>= 1.0)" for index in range(6))
        control = _control_text("1.2.3-1", dependencies, 1, package_name=PACKAGE_NAME)
        lines = control.splitlines()
        start = next(index for index, line in enumerate(lines) if line.startswith("Depends: "))
        field_lines = [lines[start]]
        for line in lines[start + 1 :]:
            if not line.startswith((" ", "\t")):
                break
            field_lines.append(line)

        self.assertGreater(len(field_lines), 1)
        self.assertEqual(re.sub(r"\n[ \t]+", " ", "\n".join(field_lines)), f"Depends: {dependencies}")

    def test_cli_selects_independent_export_only_when_requested(self) -> None:
        required = [
            "package_linux_deb_setup.py",
            "--source-dir", "bundle",
            "--build-evidence", "evidence.json",
            "--output-dir", "packages",
            "--package-revision", "1",
        ]
        for options, independent in (([], False), (["--independent-export"], True)):
            with (
                self.subTest(independent=independent),
                patch.object(sys, "argv", required + options),
                patch("tooling.package_linux_deb_setup.write_debian_setup_package", return_value={}) as write,
                patch("builtins.print"),
            ):
                self.assertEqual(main(), 0)
                write.assert_called_once_with(
                    Path("bundle"), Path("evidence.json"), Path("packages"),
                    package_revision=1, independent_export=independent,
                )


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

    def create_export(
        self,
        root: Path,
        *,
        app_id: str = APP_ID,
        audit_store_id: str = AUDIT_ID,
        display_name: str = "D4 Pocket",
    ) -> tuple[Path, Path]:
        output = root / "export"
        bundle = output / "bundle"
        bundle.mkdir(parents=True)
        for relative, payload in PAYLOADS.items():
            if relative == "product_manifest.json":
                manifest = json.loads(payload)
                manifest["manifest"]["app_identity"].update(app_id=app_id, display_name=display_name)
                manifest["manifest"]["audit_store"]["store_id"] = audit_store_id
                payload = json.dumps(manifest).encode("utf-8")
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

    def build_package(
        self,
        root: Path,
        revision: int,
        output_name: str | None = None,
        source_modes: dict[str, int] | None = None,
        *,
        app_id: str = APP_ID,
        audit_store_id: str = AUDIT_ID,
        display_name: str = "D4 Pocket",
        independent_export: bool = False,
    ) -> tuple[Path, dict[str, object]]:
        bundle, evidence = self.create_export(
            root, app_id=app_id, audit_store_id=audit_store_id, display_name=display_name
        )
        for relative, mode in (source_modes or {}).items():
            (bundle / relative).chmod(mode)
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
                independent_export=independent_export,
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

    def test_export_setup_packages_have_nonoverlapping_install_paths(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-linux-deb-independent-") as temporary:
            root = Path(temporary)
            extracted = root / "installed-tree"
            files = set()
            for suffix, audit_suffix, display_name in (("1", "2", "青い製品"), ("3", "4", "緑の製品\\作業用")):
                app_id = f"d4-pocket-app-{suffix * 32}"
                audit_store_id = f"audit-store-{audit_suffix * 32}"
                package, result = self.build_package(
                    root / suffix,
                    1,
                    app_id=app_id,
                    audit_store_id=audit_store_id,
                    display_name=display_name,
                    independent_export=True,
                )
                package_name = f"d4-pocket-setup-{suffix * 32}"
                self.assertEqual(self.dpkg_field(package, "Package").strip(), package_name)
                self.assertEqual(result["package_name"], package_name)
                install_root = f"/opt/d4-pocket/exports/{app_id}/setup"
                desktop_entry = f"/usr/share/applications/{package_name}.desktop"
                self.assertEqual(result["installed_root"], install_root)
                self.assertEqual(result["desktop_entry"], desktop_entry)
                subprocess.run(["dpkg-deb", "--extract", package, extracted], check=True)
                next_files = {path for path in extracted.rglob("*") if path.is_file()}
                self.assertLess(files, next_files)
                self.assertEqual(len(next_files - files), len(PAYLOADS) + 1)
                files = next_files
                launcher = extracted / install_root.lstrip("/") / "gui_shell_desktop_launcher"
                self.assertTrue(os.access(launcher, os.X_OK))
                entry = (extracted / desktop_entry.lstrip("/")).read_text(encoding="utf-8").splitlines()
                self.assertIn(f"Exec={install_root}/gui_shell_desktop_launcher", entry)
                escaped_name = display_name.replace("\\", "\\\\")
                self.assertIn(f"Name={escaped_name} Setup", entry)
                manifest = json.loads(launcher.with_name("product_manifest.json").read_text())
                self.assertEqual(manifest["manifest"]["app_identity"]["app_id"], app_id)
                self.assertEqual(manifest["manifest"]["audit_store"]["store_id"], audit_store_id)
            self.assertFalse((extracted / "opt/d4-pocket/setup").exists())

    def test_export_setup_rejects_unsafe_desktop_identity(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-linux-deb-invalid-identity-") as temporary:
            root = Path(temporary)
            with self.assertRaisesRegex(ValueError, "App IDが不正"):
                self.build_package(root / "id", 1, app_id="../other", independent_export=True)
            with self.assertRaisesRegex(ValueError, "表示名"):
                self.build_package(
                    root / "name", 1, display_name="製品\nExec=/tmp/other", independent_export=True
                )
            self.assertFalse(list(root.rglob("*.deb")))

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

    def test_safe_source_modes_are_normalized_to_fixed_package_modes(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-linux-deb-mode-normalization-") as temporary:
            root = Path(temporary)
            package, _result = self.build_package(
                root,
                1,
                source_modes={
                    "data/icudtl.dat": 0o600,
                    "gui_shell_desktop_launcher": 0o700,
                },
            )
            extracted = root / "extracted"
            extracted.mkdir()
            result = subprocess.run(
                ["dpkg-deb", "--extract", str(package), str(extracted)],
                check=False,
                capture_output=True,
                text=True,
                encoding="utf-8",
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            installed_root = extracted / "opt/d4-pocket/setup"
            self.assertEqual(stat.S_IMODE((installed_root / "data/icudtl.dat").stat().st_mode), 0o644)
            self.assertEqual(stat.S_IMODE((installed_root / "gui_shell_desktop_launcher").stat().st_mode), 0o755)

    def test_unsafe_source_mode_is_rejected_before_package_output(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-linux-deb-unsafe-mode-") as temporary:
            root = Path(temporary)
            with self.assertRaisesRegex(ValueError, "Linux bundle input permissionが安全でない"):
                self.build_package(
                    root,
                    1,
                    source_modes={"data/icudtl.dat": 0o664},
                )
            self.assertFalse(list(root.rglob("*.deb")))

    def test_package_root_does_not_collide_with_shlibdeps_metadata(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-linux-deb-stage-roots-") as temporary:
            root = Path(temporary)
            bundle, evidence = self.create_export(root)

            def collect_dependencies(stage: Path, package_root: Path, bundle_stage: Path) -> tuple[str, str]:
                control_root = stage / "debian"
                control_root.mkdir(mode=0o700)
                (control_root / "control").write_text("shlibdeps input\n", encoding="utf-8")
                self.assertFalse((control_root / PACKAGE_NAME).exists())
                self.assertTrue(bundle_stage.is_relative_to(package_root))
                return DEPENDENCIES, ""

            with (
                patch("tooling.package_linux_deb_setup._require_supported_target"),
                patch("tooling.package_linux_deb_setup._require_source_checkout"),
                patch("tooling.package_linux_deb_setup.product_version_from_source", return_value="1.2.3"),
                patch("tooling.package_linux_deb_setup._collect_dependencies", side_effect=collect_dependencies),
                patch("tooling.package_linux_deb_setup.validate_build_evidence"),
            ):
                result = write_debian_setup_package(
                    bundle,
                    evidence,
                    root / "deb",
                    package_revision=1,
                )

            package = Path(str(result["output_path"]))
            self.assertTrue(package.is_file())
            self.assertTrue(package.name.endswith("_1.2.3-1_amd64.deb"))

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

            package, _ = self.build_package(
                root / "existing", 1, "deb-existing", independent_export=True
            )
            bundle, evidence = self.create_export(root / "second")
            with (
                patch("tooling.package_linux_deb_setup._require_supported_target"),
                patch("tooling.package_linux_deb_setup._require_source_checkout"),
                patch("tooling.package_linux_deb_setup.product_version_from_source", return_value="1.2.3+4"),
                patch("tooling.package_linux_deb_setup._collect_dependencies", return_value=(DEPENDENCIES, "")),
                patch("tooling.package_linux_deb_setup.validate_build_evidence"),
            ):
                with self.assertRaisesRegex(ValueError, "already exists"):
                    write_debian_setup_package(
                        bundle, evidence, package.parent, package_revision=1, independent_export=True
                    )

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
