from __future__ import annotations

import hashlib
import json
import unittest

from tooling.tests.create_linux_update_catalog_fixture import (
    APP_ID,
    AUDIT_ID,
    CURRENT_VERSION,
    FIXTURE_DIR,
    OFFERED_VERSION,
    SUMMARY,
    UPDATE_ID,
    build_package,
)


class LinuxUpdateCatalogFixtureTests(unittest.TestCase):
    def test_catalog_binds_the_exact_small_linux_package(self) -> None:
        package = (FIXTURE_DIR / f"{UPDATE_ID}.pkg").read_bytes()
        catalog = json.loads((FIXTURE_DIR / "updates.json").read_text(encoding="utf-8"))
        self.assertEqual(package, build_package())
        self.assertEqual(catalog["版"], 1)
        self.assertEqual(len(catalog["候補"]), 1)
        candidate = catalog["候補"][0]
        signed = {
            key: candidate[key]
            for key in (
                "版",
                "更新ID",
                "現行版",
                "提供版",
                "channel",
                "内容概要",
                "package_sha256",
                "package_size_bytes",
                "rollback可能",
            )
        }
        self.assertEqual(candidate["更新ID"], UPDATE_ID)
        self.assertEqual(candidate["現行版"], CURRENT_VERSION)
        self.assertEqual(candidate["提供版"], OFFERED_VERSION)
        self.assertEqual(candidate["内容概要"], SUMMARY)
        self.assertEqual(candidate["package_sha256"], hashlib.sha256(package).hexdigest())
        self.assertEqual(candidate["package_size_bytes"], len(package))
        self.assertEqual(
            bytes.fromhex(candidate["署名対象"]),
            json.dumps(signed, ensure_ascii=False, separators=(",", ":")).encode(
                "utf-8"
            ),
        )
        self.assertEqual(len(bytes.fromhex(candidate["署名"])), 64)

    def test_package_is_linux_shaped_and_has_no_real_runtime_payload(self) -> None:
        package = build_package()
        self.assertEqual(package[:8], b"D4PKG01\n")
        manifest_length = int.from_bytes(package[8:12], "little")
        manifest = json.loads(package[12 : 12 + manifest_length])
        self.assertEqual(manifest["product_version"], OFFERED_VERSION)
        self.assertEqual(manifest["app_id"], APP_ID)
        self.assertEqual(manifest["audit_store_id"], AUDIT_ID)
        self.assertEqual(manifest["target_platform"], "linux")
        self.assertEqual(
            [entry["path"] for entry in manifest["files"]],
            [
                "data/icudtl.dat",
                "gui_shell_desktop",
                "gui_shell_desktop_launcher",
                "lib/libflutter_linux_gtk.so",
                "libexec/gui_shell_linux_broker",
                "product_manifest.json",
            ],
        )
        self.assertNotIn(b"\x7fELF", package[12 + manifest_length :])


if __name__ == "__main__":
    unittest.main()
