from __future__ import annotations

import hashlib
import struct
import subprocess
import tempfile
import unittest
import zlib
from pathlib import Path
from unittest.mock import patch

from tooling.tests.linux_gtk_bundle_smoke import (
    _capture_weston_frame,
    _decode_png_rgba,
    _different_pixel_count,
    _owned_product_groups,
    _request_x11_window_manager_close,
    _sway_arguments,
    _wayland_socket_connected,
    _weston_arguments,
    _weston_backend_is_unsupported,
    _weston_renderer_is_unsupported,
)


def _paeth(left: int, above: int, upper_left: int) -> int:
    estimate = left + above - upper_left
    distances = (
        abs(estimate - left),
        abs(estimate - above),
        abs(estimate - upper_left),
    )
    return (left, above, upper_left)[distances.index(min(distances))]


def _png_chunk(name: bytes, payload: bytes) -> bytes:
    return (
        struct.pack(">I", len(payload))
        + name
        + payload
        + struct.pack(">I", zlib.crc32(name + payload) & 0xFFFFFFFF)
    )


def _png_image(width: int, rows: list[bytes], *, channels: int = 4) -> bytes:
    color_type = 6 if channels == 4 else 2
    stride = width * channels
    previous = bytearray(stride)
    filtered_rows = bytearray()
    for row_number, raw in enumerate(rows):
        if len(raw) != stride:
            raise ValueError("test PNG row length does not match width")
        filter_type = row_number % 5
        filtered_rows.append(filter_type)
        for index, value in enumerate(raw):
            left = raw[index - channels] if index >= channels else 0
            above = previous[index]
            upper_left = previous[index - channels] if index >= channels else 0
            if filter_type == 1:
                predictor = left
            elif filter_type == 2:
                predictor = above
            elif filter_type == 3:
                predictor = (left + above) // 2
            elif filter_type == 4:
                predictor = _paeth(left, above, upper_left)
            else:
                predictor = 0
            filtered_rows.append((value - predictor) & 0xFF)
        previous = bytearray(raw)
    header = struct.pack(">IIBBBBB", width, len(rows), 8, color_type, 0, 0, 0)
    return (
        b"\x89PNG\r\n\x1a\n"
        + _png_chunk(b"IHDR", header)
        + _png_chunk(b"IDAT", zlib.compress(bytes(filtered_rows)))
        + _png_chunk(b"IEND", b"")
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


class WestonBackendCompatibilityTests(unittest.TestCase):
    def test_enables_screenshot_debug_protocol_only_for_explicit_capture_scope(self) -> None:
        ordinary = _weston_arguments(
            "--backend=headless",
            "--renderer=pixman",
            "w",
            enable_output_capture=False,
        )
        capture = _weston_arguments(
            "--backend=headless",
            "--renderer=pixman",
            "w",
            enable_output_capture=True,
        )
        self.assertNotIn("--debug", ordinary)
        self.assertEqual(capture.count("--debug"), 1)


    def test_retries_legacy_module_name_only_for_unknown_short_backend(self) -> None:
        self.assertTrue(
            _weston_backend_is_unsupported(
                "--backend=headless", 'Error: unknown backend "headless"\n'
            )
        )
        self.assertFalse(
            _weston_backend_is_unsupported(
                "--backend=headless", "fatal: failed to create compositor backend\n"
            )
        )
        self.assertFalse(
            _weston_backend_is_unsupported(
                "--backend=headless-backend.so",
                'Error: unknown backend "headless"\n',
            )
        )


    def test_retries_legacy_pixman_flag_only_for_unhandled_renderer_option(self) -> None:
        self.assertTrue(
            _weston_renderer_is_unsupported(
                "--renderer=pixman", "fatal: unhandled option: --renderer=pixman\n"
            )
        )
        self.assertFalse(
            _weston_renderer_is_unsupported(
                "--renderer=pixman", "fatal: failed to initialize pixman renderer\n"
            )
        )
        self.assertFalse(
            _weston_renderer_is_unsupported(
                "--use-pixman", "fatal: unhandled option: --renderer=pixman\n"
            )
        )


class SwayBackendCompatibilityTests(unittest.TestCase):
    def test_sway_test_compositor_uses_explicit_private_config(self) -> None:
        config = Path("/tmp/d4-sway.conf")
        self.assertEqual(
            _sway_arguments(config),
            ["sway", "--debug", "--config", str(config)],
        )


class WestonOutputCaptureTests(unittest.TestCase):
    def test_decodes_all_png_filters_and_rgb_alpha(self) -> None:
        rows = [
            bytes(
                (row * 31 + column * 17 + channel * 9) & 0xFF
                for column in range(3)
                for channel in range(4)
            )
            for row in range(5)
        ]
        image = _png_image(3, rows)
        self.assertEqual(_decode_png_rgba(image), (3, 5, b"".join(rows)))

        rgb_rows = [bytes((10, 20, 30, 40, 50, 60)), bytes((70, 80, 90, 100, 110, 120))]
        rgb_image = _png_image(2, rgb_rows, channels=3)
        expected_rgba = b"".join(
            rgb_rows[index][offset : offset + 3] + b"\xff"
            for index in range(len(rgb_rows))
            for offset in range(0, len(rgb_rows[index]), 3)
        )
        self.assertEqual(_decode_png_rgba(rgb_image), (2, 2, expected_rgba))

    def test_rejects_corrupt_png_crc(self) -> None:
        image = bytearray(_png_image(1, [b"\x01\x02\x03\x04"]))
        image[29] ^= 0x01
        with self.assertRaisesRegex(RuntimeError, "CRC"):
            _decode_png_rgba(bytes(image))

    def test_rejects_unknown_critical_chunk_and_noncontiguous_idat(self) -> None:
        image = _png_image(1, [b"\x01\x02\x03\x04"])
        unknown_critical = _png_chunk(b"ABCD", b"")
        with self.assertRaisesRegex(RuntimeError, "critical"):
            _decode_png_rgba(image[:33] + unknown_critical + image[33:])

        header_end = 8 + 25
        idat_length = struct.unpack_from(">I", image, header_end)[0]
        first_idat_end = header_end + 12 + idat_length
        idat = image[header_end:first_idat_end]
        chunk_length = struct.unpack_from(">I", idat, 0)[0]
        chunk_data = idat[8 : 8 + chunk_length]
        split = len(chunk_data) // 2
        first = _png_chunk(b"IDAT", chunk_data[:split])
        ancillary = _png_chunk(b"tEXt", b"separated")
        second = _png_chunk(b"IDAT", chunk_data[split:])
        noncontiguous = image[:header_end] + first + ancillary + second + image[first_idat_end:]
        with self.assertRaisesRegex(RuntimeError, "連続"):
            _decode_png_rgba(noncontiguous)

    def test_counts_changed_pixels_only_when_frame_sizes_match(self) -> None:
        first = (2, 1, b"\x00\x00\x00\xff\x01\x01\x01\xff")
        second = (2, 1, b"\x00\x00\x00\xff\x02\x01\x01\xff")
        self.assertEqual(_different_pixel_count(first, second), 1)
        with self.assertRaisesRegex(RuntimeError, "寸法"):
            _different_pixel_count(first, (1, 2, first[2]))

    def test_captures_one_private_weston_png_and_removes_it(self) -> None:
        png = _png_image(1, [b"\x01\x02\x03\x04"])
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            pictures = root / "capture"

            def create_capture(command, *, env, **kwargs):
                self.assertEqual(command, ["weston-screenshooter"])
                self.assertEqual(Path(env["XDG_PICTURES_DIR"]), pictures)
                (pictures / "wayland-screenshot-test.png").write_bytes(png)
                return subprocess.CompletedProcess(command, 0, b"", b"")

            with (
                patch(
                    "tooling.tests.linux_gtk_bundle_smoke.shutil.which",
                    return_value="/usr/bin/weston-screenshooter",
                ),
                patch(
                    "tooling.tests.linux_gtk_bundle_smoke.subprocess.run",
                    side_effect=create_capture,
                ),
            ):
                frame, digest = _capture_weston_frame({"PATH": "/usr/bin"}, pictures)

            self.assertEqual(frame, _decode_png_rgba(png))
            self.assertEqual(digest, hashlib.sha256(png).hexdigest())
            self.assertFalse(pictures.exists())


class X11WindowManagerCloseTests(unittest.TestCase):
    def test_sends_wm_delete_to_the_detected_window(self) -> None:
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
            ["xdotool", "windowclose", "0x42"],
        )

    def test_does_not_send_wm_delete_when_window_activation_fails(self) -> None:
        with patch(
            "tooling.tests.linux_gtk_bundle_smoke.subprocess.run",
            return_value=subprocess.CompletedProcess([], 1),
        ) as run:
            with self.assertRaisesRegex(RuntimeError, "activate"):
                _request_x11_window_manager_close("0x42", {"DISPLAY": ":99"})

        run.assert_called_once()


if __name__ == "__main__":
    unittest.main()
