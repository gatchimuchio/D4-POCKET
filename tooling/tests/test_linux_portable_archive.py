from __future__ import annotations

import gzip
import hashlib
import io
import os
import sys
import tarfile
import tempfile
import unittest
from pathlib import Path

from tooling.export_linux_product import (
    PORTABLE_ARCHIVE_ROOT,
    PORTABLE_ARCHIVE_TARGET,
    PORTABLE_README_MEMBER,
    _portable_archive_plan,
    _validate_portable_archive,
    _write_portable_archive,
)
from tooling.export_windows_product import _artifact_inventory
from tooling.package_linux_product import EXECUTABLE_PATHS

PAYLOADS = {
    "data/icudtl.dat": b"icu",
    "data/flutter_assets/AssetManifest.bin": b"assets",
    "gui_shell_desktop": b"runner",
    "gui_shell_desktop_launcher": b"launcher",
    "lib/libflutter_linux_gtk.so": b"flutter",
    "libexec/gui_shell_linux_broker": b"broker",
}
README = (Path(__file__).resolve().parents[2] / "installer/linux/PORTABLE_README_ja.txt").read_bytes()


@unittest.skipUnless(sys.platform.startswith("linux"), "Linux archive mode semantics are required")
class LinuxPortableArchiveTests(unittest.TestCase):
    def create_bundle(self, root: Path) -> Path:
        bundle = root / "bundle"
        bundle.mkdir(mode=0o755)
        os.chmod(bundle, 0o755)
        for relative, payload in PAYLOADS.items():
            target = bundle / relative
            target.parent.mkdir(parents=True, exist_ok=True, mode=0o755)
            os.chmod(target.parent, 0o755)
            target.write_bytes(payload)
            os.chmod(target, 0o755 if relative in EXECUTABLE_PATHS else 0o644)
        return bundle

    def test_archive_is_reproducible_and_content_hash_bound(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-linux-portable-archive-") as temporary:
            root = Path(temporary)
            bundle = self.create_bundle(root)
            records, _size, _tree_hash = _artifact_inventory(bundle)
            first = root / "first.tar.gz"
            second = root / "second.tar.gz"

            first_record = _write_portable_archive(bundle, first, README, records)
            second_record = _write_portable_archive(bundle, second, README, records)
            self.assertEqual(first.read_bytes(), second.read_bytes())
            self.assertEqual(first_record["sha256"], second_record["sha256"])
            self.assertEqual(first_record["target"], PORTABLE_ARCHIVE_TARGET)
            self.assertEqual(first_record["readme_sha256"], hashlib.sha256(README).hexdigest())
            _validate_portable_archive(bundle, first, README, records, first_record)

            with tarfile.open(first, mode="r:gz") as archive:
                members = archive.getmembers()
                self.assertEqual(members[0].name, PORTABLE_ARCHIVE_ROOT)
                self.assertEqual(len(members), first_record["entry_count"])
                by_name = {member.name: member for member in members}
                self.assertIn(f"{PORTABLE_ARCHIVE_ROOT}/{PORTABLE_README_MEMBER}", by_name)
                self.assertEqual(by_name[f"{PORTABLE_ARCHIVE_ROOT}/gui_shell_desktop_launcher"].mode, 0o755)
                self.assertTrue(all(member.uid == 0 and member.gid == 0 for member in members))
                self.assertTrue(all(member.mtime == 0 for member in members))

    def test_archive_rejects_symlink_payload(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-linux-portable-symlink-") as temporary:
            root = Path(temporary)
            bundle = self.create_bundle(root)
            records, _size, _tree_hash = _artifact_inventory(bundle)
            outside = root / "outside.txt"
            outside.write_text("outside", encoding="utf-8")
            link = bundle / "data" / "symlink"
            link.symlink_to(outside)
            with self.assertRaises(ValueError):
                _write_portable_archive(bundle, root / "rejected.tar.gz", README, records)
            self.assertFalse((root / "rejected.tar.gz").exists())

    def test_archive_rejects_unsafe_source_mode(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-linux-portable-mode-") as temporary:
            root = Path(temporary)
            bundle = self.create_bundle(root)
            unsafe_file = bundle / "data" / "icudtl.dat"
            os.chmod(unsafe_file, 0o666)
            records, _size, _tree_hash = _artifact_inventory(bundle)
            with self.assertRaisesRegex(ValueError, "固定package mode"):
                _write_portable_archive(bundle, root / "rejected.tar.gz", README, records)
            self.assertFalse((root / "rejected.tar.gz").exists())

    def test_archive_rejects_reserved_readme_name_collision(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-linux-portable-readme-") as temporary:
            root = Path(temporary)
            bundle = self.create_bundle(root)
            (bundle / PORTABLE_README_MEMBER).write_bytes(b"payload collision")
            os.chmod(bundle / PORTABLE_README_MEMBER, 0o644)
            records, _size, _tree_hash = _artifact_inventory(bundle)
            with self.assertRaisesRegex(ValueError, "予約済みREADME名"):
                _write_portable_archive(bundle, root / "rejected.tar.gz", README, records)
            self.assertFalse((root / "rejected.tar.gz").exists())

    def test_archive_validator_rejects_path_traversal_member(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-linux-portable-traversal-") as temporary:
            root = Path(temporary)
            bundle = self.create_bundle(root)
            records, _size, _tree_hash = _artifact_inventory(bundle)
            plan = _portable_archive_plan(bundle, README, records)
            archive_path = root / "traversal.tar.gz"
            with archive_path.open("wb") as raw:
                with gzip.GzipFile(fileobj=raw, mode="wb", filename="", mtime=0) as compressed:
                    with tarfile.open(fileobj=compressed, mode="w|", format=tarfile.PAX_FORMAT) as archive:
                        for index, item in enumerate(plan):
                            name = item["name"]
                            if index == len(plan) - 1:
                                name = f"{PORTABLE_ARCHIVE_ROOT}/../escape"
                            info = tarfile.TarInfo(name)
                            info.uid = 0
                            info.gid = 0
                            info.uname = ""
                            info.gname = ""
                            info.mtime = 0
                            info.mode = item["mode"]
                            if item["kind"] == "directory":
                                info.type = tarfile.DIRTYPE
                                info.size = 0
                                archive.addfile(info)
                            else:
                                info.type = tarfile.REGTYPE
                                info.size = item["size"]
                                payload = item["payload"] if item["path"] is None else item["path"].read_bytes()
                                archive.addfile(info, io.BytesIO(payload))
            digest = hashlib.sha256(archive_path.read_bytes()).hexdigest()
            record = {
                "file_name": archive_path.name,
                "format": "tar.gz",
                "target": PORTABLE_ARCHIVE_TARGET,
                "byte_length": archive_path.stat().st_size,
                "sha256": digest,
                "entry_count": len(plan),
                "readme_sha256": hashlib.sha256(README).hexdigest(),
                "signed": False,
            }
            with self.assertRaisesRegex(ValueError, "構成要素の位置と並び順"):
                _validate_portable_archive(bundle, archive_path, README, records, record)
            self.assertFalse((root / "escape").exists())


if __name__ == "__main__":
    unittest.main()
