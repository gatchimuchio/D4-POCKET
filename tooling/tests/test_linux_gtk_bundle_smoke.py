from __future__ import annotations

import subprocess
import unittest
from pathlib import Path
from unittest.mock import patch

from tooling.tests.linux_gtk_bundle_smoke import (
    _owned_product_groups,
    _request_x11_window_manager_close,
    _wayland_socket_connected,
)


class WaylandSocketSmokeTests(unittest.TestCase):
    def test_accepts_successful_connection_to_exact_isolated_socket(self) -> None:
        output = (
            'connect(5, {sa_family=AF_UNIX, sun_path="/tmp/d4p/r/w"}, 17) = 0\n'
        )
        self.assertTrue(_wayland_socket_connected(output, Path("/tmp/d4p/r/w")))

    def test_rejects_connection_to_a_different_socket(self) -> None:
        output = (
            'connect(5, {sa_family=AF_UNIX, sun_path="/tmp/other/wayland-0"}, 24) = 0\n'
        )
        self.assertFalse(_wayland_socket_connected(output, Path("/tmp/d4p/r/w")))

    def test_rejects_failed_connection_to_exact_socket(self) -> None:
        output = (
            'connect(5, {sa_family=AF_UNIX, sun_path="/tmp/d4p/r/w"}, 17) = -1 ECONNREFUSED\n'
        )
        self.assertFalse(_wayland_socket_connected(output, Path("/tmp/d4p/r/w")))

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


class X11WindowManagerCloseTests(unittest.TestCase):
    def test_uses_window_manager_close_key_for_the_detected_window(self) -> None:
        with patch(
            "tooling.tests.linux_gtk_bundle_smoke.subprocess.run",
            side_effect=(
                subprocess.CompletedProcess([], 0),
                subprocess.CompletedProcess([], 0),
            ),
        ) as run:
            _request_x11_window_manager_close("0x42", {"DISPLAY": ":99"})

        self.assertEqual(
            run.call_args_list[0].args[0],
            ["xdotool", "windowactivate", "--sync", "0x42"],
        )
        self.assertEqual(
            run.call_args_list[1].args[0],
            ["xdotool", "key", "--clearmodifiers", "Alt+F4"],
        )

    def test_does_not_send_close_key_when_window_activation_fails(self) -> None:
        with patch(
            "tooling.tests.linux_gtk_bundle_smoke.subprocess.run",
            return_value=subprocess.CompletedProcess([], 1),
        ) as run:
            with self.assertRaisesRegex(RuntimeError, "activate"):
                _request_x11_window_manager_close("0x42", {"DISPLAY": ":99"})

        run.assert_called_once()


if __name__ == "__main__":
    unittest.main()
