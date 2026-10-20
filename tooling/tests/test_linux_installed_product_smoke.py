from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from tooling.tests.linux_installed_product_smoke import (
    _desktop_entry_targets_fixed_launcher,
    _resolve_regular_package,
    _safe_log_tail,
)


class LinuxInstalledDesktopEntryTests(unittest.TestCase):
    def test_log_tail_is_bounded_and_masks_isolated_roots(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            log = root / "launcher.log"
            log.write_text("x" * 2000 + str(root) + "x" * 3000, encoding="utf-8")
            tail = _safe_log_tail(log, (root,))
            self.assertLessEqual(len(tail), 4000)
            self.assertNotIn(str(root), tail)
            self.assertIn("<isolated-root>", tail)

    def test_accepts_registered_fixed_launcher(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            launcher = root / "product" / "gui_shell_desktop_launcher"
            launcher.parent.mkdir()
            launcher.write_bytes(b"launcher")
            entry = root / "D4Pocket-test.desktop"
            escaped = str(launcher).replace("\\", "\\\\").replace('"', '\\"')
            escaped = escaped.replace("`", "\\`").replace("$", "\\$").replace("%", "%%")
            entry.write_text(f'[Desktop Entry]\nType=Application\nExec="{escaped}"\n', encoding="utf-8")
            self.assertTrue(_desktop_entry_targets_fixed_launcher(entry, launcher))

    def test_rejects_other_or_ambiguous_exec_targets(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            launcher = root / "product" / "gui_shell_desktop_launcher"
            launcher.parent.mkdir()
            launcher.write_bytes(b"launcher")
            entry = root / "D4Pocket-test.desktop"
            entry.write_text(
                '[Desktop Entry]\nType=Application\nExec="/tmp/other-launcher"\n',
                encoding="utf-8",
            )
            self.assertFalse(_desktop_entry_targets_fixed_launcher(entry, launcher))
            entry.write_text(
                f'[Desktop Entry]\nExec="{launcher}"\nExec="{launcher}"\n',
                encoding="utf-8",
            )
            self.assertFalse(_desktop_entry_targets_fixed_launcher(entry, launcher))

    def test_rejects_symlink_desktop_entry(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            launcher = root / "product" / "gui_shell_desktop_launcher"
            launcher.parent.mkdir()
            launcher.write_bytes(b"launcher")
            actual = root / "actual.desktop"
            actual.write_text(f'[Desktop Entry]\nExec="{launcher}"\n', encoding="utf-8")
            link = root / "D4Pocket-test.desktop"
            try:
                link.symlink_to(actual)
            except (OSError, NotImplementedError):
                self.skipTest("symlink creation unavailable")
            self.assertFalse(_desktop_entry_targets_fixed_launcher(link, launcher))

    def test_resolves_regular_package_and_rejects_symlink_before_resolution(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            package = root / "release.d4pkg"
            package.write_bytes(b"D4PKG01\n")
            self.assertEqual(_resolve_regular_package(package), package.resolve())

            link = root / "release-link.d4pkg"
            try:
                link.symlink_to(package)
            except (OSError, NotImplementedError):
                self.skipTest("symlink creation unavailable")
            with self.assertRaisesRegex(RuntimeError, "通常file"):
                _resolve_regular_package(link)


if __name__ == "__main__":
    unittest.main()
