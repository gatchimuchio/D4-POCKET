from __future__ import annotations

import hashlib
import hmac
import json
import subprocess
import tempfile
import unittest
from pathlib import Path

from tooling.tests.linux_installed_product_smoke import (
    _activate_accessible,
    _activate_named_control_by_keyboard,
    _actionable_node,
    _component_center,
    _desktop_entry_targets_fixed_launcher,
    _focused_named_accessible,
    _find_operable_accessible,
    _resolve_regular_package,
    _safe_log_tail,
    _verify_export_manifest,
    _verify_live_audit,
    _wayland_launch_environment,
    _wait_for_owner_dialog_window,
)


class AccessibleActionResolutionTests(unittest.TestCase):
    class Actions:
        def __init__(self, names: tuple[str, ...]) -> None:
            self.names = names
            self.nActions = len(names)
            self.activated: int | None = None

        def getName(self, index: int) -> str:
            return self.names[index]

        def doAction(self, index: int) -> bool:
            self.activated = index
            return True

    class Node:
        def __init__(self, name: str, actions: tuple[str, ...] = ()) -> None:
            self.name = name
            self.parent = None
            self.actions = actions
            self.action_interface = AccessibleActionResolutionTests.Actions(actions)

        def queryAction(self):
            return self.action_interface

        def get_parent(self):
            return self.parent

        def queryComponent(self):
            return self

        def getExtents(self, coordinate_type: int):
            self.coordinate_type = coordinate_type
            return type("Bounds", (), {"x": 10, "y": 20, "width": 30, "height": 40})()

    def test_resolves_actionable_button_ancestor_of_visible_label(self) -> None:
        button = self.Node("", ("click",))
        label = self.Node("Owner確認してManifest fileを生成")
        label.parent = button
        self.assertIs(_actionable_node(label), button)

    def test_does_not_treat_non_actionable_label_as_a_button(self) -> None:
        label = self.Node("Owner確認してManifest fileを生成")
        self.assertIsNone(_actionable_node(label))

    def test_activates_the_exposed_at_spi_tap_action(self) -> None:
        button = self.Node("独立Appを書き出す", ("Tap", "Focus"))
        _activate_accessible(button, button.name, {"DISPLAY": ":99"})
        self.assertEqual(button.action_interface.activated, 0)

    def test_uses_live_accessible_bounds_for_coordinate_fallback(self) -> None:
        from types import SimpleNamespace
        from unittest.mock import patch

        label = self.Node("Owner確認してManifest fileを生成")
        with patch.dict("sys.modules", {"pyatspi": SimpleNamespace(XY_SCREEN=0)}):
            self.assertEqual(_component_center(label), (25, 40))
        self.assertEqual(label.coordinate_type, 0)


    def test_focused_lookup_requires_exact_name_and_visible_state(self) -> None:
        from types import SimpleNamespace
        from unittest.mock import patch

        class State:
            def __init__(self, values: set[int]) -> None:
                self.values = values

            def contains(self, value: int) -> bool:
                return value in self.values

        class Node:
            name = "Owner確認してManifest fileを生成"
            childCount = 0

            def __init__(self, values: set[int]) -> None:
                self.values = values

            def getState(self):
                return State(self.values)

        class Root:
            childCount = 2

            def __init__(self, nodes) -> None:
                self.nodes = nodes

            def getChildAtIndex(self, index: int):
                return self.nodes[index]

        visible_focus = Node({1, 2})
        hidden_focus = Node({1})
        fake_pyatspi = SimpleNamespace(
            STATE_FOCUSED=1,
            STATE_SHOWING=2,
            Registry=SimpleNamespace(
                getDesktop=lambda _: Root([hidden_focus, visible_focus])
            ),
        )
        with patch.dict("sys.modules", {"pyatspi": fake_pyatspi}):
            self.assertIs(
                _focused_named_accessible(visible_focus.name), visible_focus
            )
            self.assertIs(_focused_named_accessible("Owner確認"), visible_focus)
            self.assertIsNone(_focused_named_accessible("different label"))

    def test_operable_lookup_matches_a_composite_accessible_name(self) -> None:
        from types import SimpleNamespace
        from unittest.mock import patch

        button = self.Node(
            "独立Appを書き出す GUI Shell独立App書出し面を開く", ("Tap",)
        )

        class Root:
            childCount = 1

            def getChildAtIndex(self, index: int):
                return button

        fake_pyatspi = SimpleNamespace(
            Registry=SimpleNamespace(getDesktop=lambda _: Root())
        )
        with patch.dict("sys.modules", {"pyatspi": fake_pyatspi}):
            self.assertIs(
                _find_operable_accessible("独立Appを書き出す", root=Root()), button
            )

    def test_keyboard_activation_sends_enter_only_after_target_focus(self) -> None:
        from unittest.mock import patch

        focused_node = object()
        with (
            patch(
                "tooling.tests.linux_installed_product_smoke._find_accessible",
                return_value=object(),
            ) as find_accessible,
            patch(
                "tooling.tests.linux_installed_product_smoke._focused_named_accessible",
                side_effect=(None, focused_node),
            ) as focused_lookup,
            patch(
                "tooling.tests.linux_installed_product_smoke.subprocess.run"
            ) as run,
            patch("tooling.tests.linux_installed_product_smoke.time.sleep"),
        ):
            _activate_named_control_by_keyboard("target", {"DISPLAY": ":99"}, "0x42")

        self.assertEqual(
            [call.args[0] for call in run.call_args_list],
            [
                ["xdotool", "windowactivate", "--sync", "0x42"],
                ["xdotool", "key", "--clearmodifiers", "Tab"],
                ["xdotool", "key", "--clearmodifiers", "Return"],
            ],
        )
        self.assertEqual(
            find_accessible.call_args.args[0], "D4 Pocket powered by GUI Shell"
        )
        self.assertEqual(
            focused_lookup.call_args.kwargs["root"], find_accessible.return_value
        )

class LinuxInstalledDesktopEntryTests(unittest.TestCase):
    def test_wayland_launch_environment_cannot_fall_back_to_x11(self) -> None:
        source = {
            "DISPLAY": ":99",
            "XAUTHORITY": "/tmp/xauth",
            "GDK_BACKEND": "x11",
            "XDG_SESSION_TYPE": "x11",
            "XDG_RUNTIME_DIR": "/run/user/1000/d4p",
        }

        result = _wayland_launch_environment(source, "wayland-test")

        self.assertNotIn("DISPLAY", result)
        self.assertNotIn("XAUTHORITY", result)
        self.assertEqual(result["GDK_BACKEND"], "wayland")
        self.assertEqual(result["XDG_SESSION_TYPE"], "wayland")
        self.assertEqual(result["WAYLAND_DISPLAY"], "wayland-test")
        self.assertEqual(result["XDG_RUNTIME_DIR"], source["XDG_RUNTIME_DIR"])
        self.assertEqual(source["DISPLAY"], ":99")

    def test_waits_for_native_owner_dialog_to_become_visible(self) -> None:
        from unittest.mock import patch

        visible = subprocess.CompletedProcess(
            args=[], returncode=0, stdout=b"0x42\n", stderr=b""
        )
        with (
            patch(
                "tooling.tests.linux_installed_product_smoke.subprocess.run",
                side_effect=(
                    subprocess.CompletedProcess(
                        args=[], returncode=1, stdout=b"", stderr=b""
                    ),
                    visible,
                ),
            ) as run,
            patch("tooling.tests.linux_installed_product_smoke.time.sleep") as sleep,
        ):
            self.assertEqual(
                _wait_for_owner_dialog_window({"DISPLAY": ":99"}), "0x42"
            )

        self.assertEqual(run.call_count, 2)
        sleep.assert_called_once_with(0.1)

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


class LinuxInstalledOwnerEvidenceTests(unittest.TestCase):
    def test_export_manifest_requires_fresh_linux_identity_and_no_inheritance(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "d4-pocket-app-0123456789abcdef0123456789abcdef.json").write_text(
                json.dumps(
                    {
                        "version": 1,
                        "product": "D4 Pocket",
                        "manifest": {
                            "app_identity": {
                                "app_id": "d4-pocket-app-0123456789abcdef0123456789abcdef",
                                "target_platform": "linux",
                            },
                            "audit_store": {
                                "store_id": "audit-store-fedcba9876543210fedcba9876543210",
                                "inherited": False,
                            },
                            "inheritance_policy": {
                                "authority": "none",
                                "permission": "none",
                                "approval": "none",
                                "credential": "none",
                                "audit_chain": "none",
                            },
                            "distribution_metadata": {"signed": False},
                        },
                    }
                ),
                encoding="utf-8",
            )
            evidence = _verify_export_manifest(root)
            self.assertEqual(
                evidence["file"],
                "d4-pocket-app-0123456789abcdef0123456789abcdef.json",
            )
            self.assertRegex(evidence["sha256"], r"^[0-9a-f]{64}$")

    def test_live_audit_verifies_chain_and_anchor_hmac_for_reject_then_allow(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            store = Path(directory)
            key = bytes(range(32))
            (store / "audit_anchor.key").write_text(key.hex(), encoding="ascii")
            events = []
            previous_hash = None
            for index, decision in enumerate(("rejected", "accepted"), start=1):
                event = {
                    "event_id": f"broker-audit-{index}",
                    "request_id": f"request-{index}",
                    "operation": "GUI Shell書出し",
                    "decision": decision,
                    "reason": "fixture",
                    "evidence_source": "INTERNAL_STATE",
                    "payload_hash": f"sha256:{index:064x}",
                    "previous_event_hash": previous_hash,
                }
                hash_input = "|".join(
                    (
                        event["event_id"],
                        event["request_id"],
                        event["operation"],
                        event["decision"],
                        event["reason"],
                        event["evidence_source"],
                        event["payload_hash"],
                        previous_hash or "",
                    )
                )
                previous_hash = "sha256:" + hashlib.sha256(
                    hash_input.encode()
                ).hexdigest()
                event["event_hash"] = previous_hash
                events.append(event)
            (store / "audit.jsonl").write_text(
                "".join(json.dumps(event) + "\n" for event in events),
                encoding="utf-8",
            )
            anchor_input = (
                f"version=1|event_count=2|head_event_hash={previous_hash}"
            ).encode()
            anchor_hmac = "sha256:" + hmac.new(key, anchor_input, hashlib.sha256).hexdigest()
            (store / "audit_anchor.json").write_text(
                json.dumps(
                    {
                        "version": 1,
                        "event_count": 2,
                        "head_event_hash": previous_hash,
                        "anchor_hmac": anchor_hmac,
                    }
                ),
                encoding="utf-8",
            )
            self.assertEqual(
                _verify_live_audit(store),
                {"events": 2, "export_accepted": 1, "export_rejected": 1},
            )

            events[0]["reason"] = "tampered"
            (store / "audit.jsonl").write_text(
                "".join(json.dumps(event) + "\n" for event in events),
                encoding="utf-8",
            )
            with self.assertRaisesRegex(RuntimeError, "event hash"):
                _verify_live_audit(store)


if __name__ == "__main__":
    unittest.main()
