from __future__ import annotations

import unittest
from pathlib import Path
from unittest.mock import patch

from tooling.tests.linux_gtk_bundle_smoke import (
    PRODUCT_WINDOW_TITLE,
    _owned_product_groups,
    _wayland_toplevel_ready,
)


class WaylandToplevelSmokeTests(unittest.TestCase):
    def test_accepts_exact_toplevel_title_and_surface_commit(self) -> None:
        output = (
            f'[1.0] -> xdg_toplevel@7.set_title("{PRODUCT_WINDOW_TITLE}")\n'
            "[1.1] -> wl_surface@5.commit()\n"
        )
        self.assertTrue(_wayland_toplevel_ready(output))

    def test_rejects_unrelated_toplevel_title(self) -> None:
        output = (
            '[1.0] -> xdg_toplevel@7.set_title("Other application")\n'
            "[1.1] -> wl_surface@5.commit()\n"
        )
        self.assertFalse(_wayland_toplevel_ready(output))

    def test_rejects_title_without_surface_commit(self) -> None:
        output = f'[1.0] -> xdg_toplevel@7.set_title("{PRODUCT_WINDOW_TITLE}")\n'
        self.assertFalse(_wayland_toplevel_ready(output))

    def test_tracks_only_groups_for_bundle_processes_started_by_this_smoke(self) -> None:
        class Launcher:
            pid = 99

            @staticmethod
            def poll() -> None:
                return None

        with (
            patch(
                "tooling.tests.linux_gtk_bundle_smoke._processes_using",
                side_effect=({10, 11}, {20, 21}),
            ),
            patch(
                "tooling.tests.linux_gtk_bundle_smoke.os.getpgid",
                side_effect={11: 110, 21: 210}.get,
                create=True,
            ),
        ):
            groups = _owned_product_groups(
                Launcher(),  # type: ignore[arg-type]
                Path("runner"),
                Path("helper"),
                {10},
                {20},
            )
        self.assertEqual(groups, {99, 110, 210})


if __name__ == "__main__":
    unittest.main()
