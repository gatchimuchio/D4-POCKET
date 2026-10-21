from __future__ import annotations

import hashlib
import hmac
import json
import subprocess
import tempfile
import unittest
from unittest.mock import patch
from pathlib import Path

from tooling.tests.linux_installed_product_smoke import (
    _activate_accessible,
    _activate_named_control_by_keyboard,
    _actionable_node,
    _answer_owner_dialog,
    _bounded_owner_text_diagnostics,
    _assert_secret_absent_from_files,
    _catalog_failure_diagnostics,
    _component_center,
    _desktop_entry_targets_fixed_launcher,
    _focused_named_accessible,
    _find_operable_accessible,
    _helper_exit_diagnostics,
    _open_mcp_center_from_installed_ui,
    _resolve_regular_package,
    _safe_log_tail,
    _set_public_text_by_keyboard,
    _secret_service_item_labels,
    _verify_export_manifest,
    _verify_credential_audit,
    _verify_live_audit,
    _verify_uninstalled_state,
    _trigger_update_action_from_installed_ui,
    _wait_for_catalog_fetch_status,
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

    def test_mcp_palette_uses_unique_subtitle_not_navigation_label(self) -> None:
        window = object()
        command = object()
        with (
            patch(
                "tooling.tests.linux_installed_product_smoke._find_accessible",
                return_value=window,
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke._find_operable_accessible",
                return_value=command,
            ) as find_operable,
            patch(
                "tooling.tests.linux_installed_product_smoke._activate_accessible"
            ) as activate,
            patch("tooling.tests.linux_installed_product_smoke.subprocess.run"),
        ):
            self.assertIs(
                _open_mcp_center_from_installed_ui({}, "0x42"), window
            )

        find_operable.assert_called_once_with(
            "MCP接続一覧とOwner確認付き切断を開く",
            root=window,
            timeout=10,
        )
        activate.assert_called_once_with(command, "MCP接続", {})

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
        self.assertIsNone(focused_lookup.call_args.kwargs["context_node"])

    def test_update_action_uses_visible_focus_when_flutter_exposes_no_action(self) -> None:
        with (
            patch(
                "tooling.tests.linux_installed_product_smoke._open_update_center_from_installed_ui",
                return_value=object(),
            ) as open_center,
            patch(
                "tooling.tests.linux_installed_product_smoke._find_accessible"
            ) as find_accessible,
            patch(
                "tooling.tests.linux_installed_product_smoke._activate_named_control_by_keyboard"
            ) as activate_by_focus,
        ):
            _trigger_update_action_from_installed_ui(
                {"DISPLAY": ":99"},
                "0x42",
                "download",
                row_marker="0.1.2 (linux-installed-catalog-display)",
            )

        self.assertEqual(open_center.call_args.args, ({"DISPLAY": ":99"}, "0x42"))
        self.assertEqual(find_accessible.call_args_list[0].args[0], "download")
        self.assertEqual(
            find_accessible.call_args_list[0].kwargs["root"],
            open_center.return_value,
        )
        activate_by_focus.assert_called_once_with(
            "download",
            {"DISPLAY": ":99"},
            "0x42",
            context_marker="0.1.2 (linux-installed-catalog-display)",
        )

    def test_focused_duplicate_action_must_share_the_candidate_row(self) -> None:
        class State:
            def __init__(self, focused: bool) -> None:
                self.focused = focused

            def contains(self, state: int) -> bool:
                return self.focused if state == 1 else True

        class Node:
            def __init__(self, name: str, children=(), *, focused: bool = False) -> None:
                self.name = name
                self.children = list(children)
                self.childCount = len(self.children)
                self.parent = None
                self.state = State(focused)
                for child in self.children:
                    child.parent = self

            def getChildAtIndex(self, index: int):
                return self.children[index]

            def get_parent(self):
                return self.parent

            def getState(self):
                return self.state

        old_action = Node("download", focused=True)
        old_row = Node("old candidate", [Node("0.1.0 (linux-installed-acceptance)"), old_action])
        selected_action = Node("download", focused=True)
        selected_row = Node(
            "selected candidate",
            [Node("0.1.2 (linux-installed-catalog-display)"), selected_action],
        )
        root = Node("window", [old_row, selected_row])
        fake_pyatspi = type(
            "FakePyatspi",
            (),
            {"STATE_FOCUSED": 1, "STATE_SHOWING": 2, "Registry": type("Registry", (), {"getDesktop": staticmethod(lambda _: root)})},
        )

        with patch.dict("sys.modules", {"pyatspi": fake_pyatspi}):
            focused = _focused_named_accessible(
                "download", root=root, context_node=selected_row
            )
            old_action.state.focused = True
            selected_action.state.focused = False
            wrong_row = _focused_named_accessible(
                "download", root=root, context_node=selected_row
            )

        self.assertIs(focused, selected_action)
        self.assertIsNone(wrong_row)


class InstalledCatalogFetchStatusTests(unittest.TestCase):
    class Node:
        def __init__(self, name: str, children=()) -> None:
            self.name = name
            self.children = list(children)
            self.childCount = len(self.children)

        def getChildAtIndex(self, index: int):
            return self.children[index]

    def test_requires_terminal_success_with_exactly_one_candidate(self) -> None:
        status = "Brokerが署名検証済み候補を取得しました（1件）。"
        root = self.Node("window", [self.Node(status)])
        self.assertEqual(_wait_for_catalog_fetch_status(root), status)

    def test_surfaces_broker_catalog_fetch_error(self) -> None:
        status = "更新候補の取得に失敗しました: 候補配布元の名前解決に失敗"
        root = self.Node("window", [self.Node(status)])
        with self.assertRaisesRegex(RuntimeError, "名前解決に失敗"):
            _wait_for_catalog_fetch_status(root)

    def test_rejects_unexpected_candidate_count(self) -> None:
        status = "Brokerが署名検証済み候補を取得しました（0件）。"
        root = self.Node("window", [self.Node(status)])
        with self.assertRaisesRegex(RuntimeError, "0件"):
            _wait_for_catalog_fetch_status(root)

    def test_failure_diagnostics_expose_process_and_audit_metadata_only(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            helper = root / "gui_shell_linux_broker"
            helper.write_bytes(b"fixture")
            audit_path = root / "audit.jsonl"
            audit_path.write_text(
                json.dumps(
                    {
                        "event_id": "broker-audit-2",
                        "operation": "更新候補取得",
                        "decision": "rejected",
                        "evidence_source": "LIVE_RUNTIME",
                        "payload": "secret-test-payload",
                        "reason": "secret-test-reason",
                    }
                )
                + "\n",
                encoding="utf-8",
            )
            with patch(
                "tooling.tests.linux_installed_product_smoke._processes_using",
                return_value=set(),
            ):
                diagnostics = _catalog_failure_diagnostics(helper, set(), audit_path)

        rendered = json.dumps(diagnostics, ensure_ascii=False)
        self.assertFalse(diagnostics["helper_alive"])
        self.assertEqual(diagnostics["helper_count"], 0)
        self.assertIn('"operation": "更新候補取得"', rendered)
        self.assertNotIn("secret-test", rendered)


class InstalledOwnerSummaryTests(unittest.TestCase):
    class Text:
        def __init__(self, value: str) -> None:
            self.value = value
            self.characterCount = len(value)

        def getText(self, start: int, end: int) -> str:
            return self.value[start:] if end == -1 else self.value[start:end]

    class Node:
        def __init__(self, name: str, children=(), text: str | None = None) -> None:
            self.name = name
            self.children = list(children)
            self.childCount = len(self.children)
            self.text = text

        def getChildAtIndex(self, index: int):
            return self.children[index]

        def queryText(self):
            if self.text is None:
                raise NotImplementedError
            return InstalledOwnerSummaryTests.Text(self.text)

    def test_accepts_required_owner_markers_split_across_accessible_text_nodes(self) -> None:
        package_hash = "c24c86c83f5fcacc15f03030ba2a6c10fabeeed405b4735ee0c1260dd7c21123"
        summary = "\n".join((
            "Linux製品の署名検査済み更新packageを取得しますか？",
            "更新ID: linux-installed-catalog-display",
            "提供版: 0.1.2",
            "channel: stable",
            "署名者fingerprint: " + "a" * 64,
            "取得host: raw.githubusercontent.com",
            f"package SHA-256: {package_hash}",
            "byte長: 1809",
            "要求hash: " + "b" * 64,
        ))
        dialog = self.Node("D4 Pocket Owner確認", [self.Node("", text=summary)])
        with (
            patch(
                "tooling.tests.linux_installed_product_smoke._wait_for_owner_dialog_window"
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke._find_accessible",
                return_value=dialog,
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke._find_operable_accessible",
                return_value=object(),
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke._activate_accessible"
            ) as activate,
        ):
            _answer_owner_dialog(
                {"DISPLAY": ":99"},
                "拒否",
                expected_summary_markers=(
                    "Linux製品の署名検査済み更新packageを取得しますか？",
                    "更新ID: linux-installed-catalog-display",
                    "提供版: 0.1.2",
                    "取得host: raw.githubusercontent.com",
                    f"package SHA-256: {package_hash}",
                    "1809",
                ),
            )
        activate.assert_called_once()

    def test_failure_diagnostics_redact_absolute_paths(self) -> None:
        observed = _bounded_owner_text_diagnostics(
            ["desktop entry: /home/runner/work/d4p/app.desktop", r"root: C:\Users\runner\private"]
        )
        self.assertNotIn("/home/runner", observed)
        self.assertNotIn(r"C:\Users\runner", observed)
        self.assertIn("<path>", observed)

    def test_linux_native_owner_summary_uses_bounded_accessible_line_labels(self) -> None:
        source = (
            Path(__file__).resolve().parents[2]
            / "apps/desktop_flutter/linux/runner/broker_process_channel.cc"
        ).read_text(encoding="utf-8")
        self.assertIn(
            "constexpr size_t kOwnerSummaryLineLimit = 256;",
            source,
        )
        self.assertIn("std::count(state->summary.begin(), state->summary.end()", source)
        self.assertIn("gtk_label_new(line.c_str())", source)
        self.assertIn("state->summary[line_end] != '\\n'", source)


class LinuxWaylandEnvironmentTests(unittest.TestCase):
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


class LinuxCredentialVaultAcceptanceHelperTests(unittest.TestCase):
    def test_public_text_uses_keyboard_and_is_read_back_before_owner_action(self) -> None:
        class TextField:
            def __init__(self) -> None:
                self.value = ""

            def queryEditableText(self):
                return self

            def setTextContents(self, value: str) -> bool:
                self.value = value
                return True

            def queryText(self):
                return self

            def getText(self, start: int, end: int) -> str:
                return self.value

            @property
            def characterCount(self) -> int:
                return len(self.value)

        field = TextField()
        commands = []

        def run(command, **kwargs):
            commands.append(command)
            if command[1] == "type":
                field.value = command[-1]
            return subprocess.CompletedProcess(command, 0)

        with (
            patch(
                "tooling.tests.linux_installed_product_smoke._find_accessible",
                return_value=field,
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke._focused_named_accessible",
                return_value=field,
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke.subprocess.run",
                side_effect=run,
            ),
            patch("tooling.tests.linux_installed_product_smoke.time.sleep"),
        ):
            _set_public_text_by_keyboard(
                object(), "サーバー識別子", "installed-server", {}, "0x42"
            )

        self.assertEqual(field.value, "installed-server")
        self.assertEqual(commands[1], ["xdotool", "key", "--clearmodifiers", "ctrl+a"])
        self.assertEqual(commands[2][-1], "installed-server")

    def test_secret_service_listing_reads_metadata_without_requesting_values(self) -> None:
        from types import SimpleNamespace

        labels_by_path = {
            "/org/freedesktop/secrets/collection/test/item1": (
                "D4 Pocket credential key"
            ),
            "/org/freedesktop/secrets/collection/test/item2": (
                "D4 Pocket credential ciphertext"
            ),
        }
        requested_properties = []

        class FakeItem:
            def __init__(self, path: str) -> None:
                self.path = path

            def Get(self, interface: str, property_name: str):
                requested_properties.append(property_name)
                if property_name == "Label":
                    return labels_by_path[self.path]
                raise AssertionError("unexpected Secret Service property request")

            def GetSecret(self, *_args):
                raise AssertionError("Secret Service secret retrieval is forbidden")

        class FakeService:
            def SearchItems(self, attributes):
                self.attributes = attributes
                paths = tuple(labels_by_path)
                return paths, ()

        class FakeBus:
            def __init__(self) -> None:
                self.service = FakeService()

            def get_object(self, _service: str, path: str):
                if path == "/org/freedesktop/secrets":
                    return self.service
                return FakeItem(path)

        bus = FakeBus()
        dbus_exception = type("DBusException", (Exception,), {})

        def dictionary(values, signature: str):
            bus.service.search_signature = signature
            return values

        fake_dbus = SimpleNamespace(
            SessionBus=lambda: bus,
            Interface=lambda proxy, _interface: proxy,
            Dictionary=dictionary,
            exceptions=SimpleNamespace(DBusException=dbus_exception),
        )
        with patch.dict("sys.modules", {"dbus": fake_dbus}):
            labels = _secret_service_item_labels()

        self.assertEqual(
            sorted(labels),
            ["D4 Pocket credential ciphertext", "D4 Pocket credential key"],
        )
        self.assertEqual(bus.service.attributes, {"application": "d4-pocket"})
        self.assertEqual(bus.service.search_signature, "ss")
        self.assertEqual(requested_properties.count("Label"), 2)
        self.assertNotIn("Secret", requested_properties)
        self.assertNotIn("Attributes", requested_properties)

    def test_secret_service_dbus_error_is_sanitized(self) -> None:
        from types import SimpleNamespace

        dbus_exception = type("DBusException", (Exception,), {})

        class FailingService:
            def SearchItems(self, _attributes):
                raise dbus_exception("sensitive backend detail")

        with (
            patch.dict(
                "sys.modules",
                {
                    "dbus": SimpleNamespace(
                        SessionBus=lambda: SimpleNamespace(
                            get_object=lambda *_args: FailingService()
                        ),
                        Interface=lambda proxy, _interface: proxy,
                        Dictionary=lambda values, signature: values,
                        exceptions=SimpleNamespace(DBusException=dbus_exception),
                    )
                },
            ),
            self.assertRaisesRegex(RuntimeError, "metadata search failed"),
        ):
            _secret_service_item_labels()

    def test_audit_chain_hmac_and_secret_non_exposure(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            store = Path(directory) / "store"
            store.mkdir()
            key = b"k" * 32
            previous_hash = None
            events = []
            for index, (operation, decision) in enumerate(
                (
                    ("資格情報登録", "rejected"),
                    ("資格情報登録", "accepted"),
                    ("資格情報失効", "rejected"),
                    ("資格情報失効", "accepted"),
                ),
                start=1,
            ):
                event = {
                    "event_id": f"broker-audit-{index}",
                    "request_id": f"credential-request-{index}",
                    "operation": operation,
                    "decision": decision,
                    "reason": "metadata only",
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
                    hash_input.encode("utf-8")
                ).hexdigest()
                event["event_hash"] = previous_hash
                events.append(event)
            (store / "audit.jsonl").write_text(
                "".join(json.dumps(event) + "\n" for event in events),
                encoding="utf-8",
            )
            anchor_input = (
                f"version=1|event_count={len(events)}|head_event_hash={previous_hash}"
            ).encode("utf-8")
            anchor_hmac = "sha256:" + hmac.new(
                key, anchor_input, hashlib.sha256
            ).hexdigest()
            (store / "audit_anchor.json").write_text(
                json.dumps(
                    {
                        "version": 1,
                        "event_count": len(events),
                        "head_event_hash": previous_hash,
                        "anchor_hmac": anchor_hmac,
                    }
                ),
                encoding="utf-8",
            )
            (store / "audit_anchor.key").write_text(key.hex(), encoding="ascii")
            self.assertEqual(
                _verify_credential_audit(store, "secret-not-in-audit"),
                {"events": 4, "registration_events": 2, "revocation_events": 2},
            )
            with (store / "audit.jsonl").open("ab") as audit:
                audit.write(b"secret-not-in-audit")
            with self.assertRaisesRegex(RuntimeError, "leaked into durable Audit"):
                _verify_credential_audit(store, "secret-not-in-audit")

    def test_acceptance_file_scan_reports_leak_without_echoing_value(self) -> None:
        marker = "test-secret-that-must-not-escape"
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "test.log").write_text(marker, encoding="utf-8")
            with self.assertRaisesRegex(RuntimeError, "leaked into an acceptance file") as error:
                _assert_secret_absent_from_files(root, marker)
        self.assertNotIn(marker, str(error.exception))


class LinuxInstalledDesktopEntryTests(unittest.TestCase):
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

    def test_helper_exit_diagnostics_extract_only_numeric_categories(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            log = Path(directory) / "installed-app.log"
            log.write_text(
                "token=must-not-leave-this-file\n"
                "Linux Broker helper exited with classified code 22\n"
                "Linux Broker helper terminated by signal 9\n",
                encoding="utf-8",
            )
            diagnostics = _helper_exit_diagnostics(log)
            self.assertEqual(diagnostics, {"exit_codes": [22], "signals": [9]})
            self.assertNotIn("token", json.dumps(diagnostics))

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
            audit_cases = (
                ("GUI Shell書出し", "rejected", "INTERNAL_STATE"),
                ("GUI Shell書出し", "accepted", "INTERNAL_STATE"),
                ("製品アンインストール要求", "rejected", "INTERNAL_STATE"),
                ("製品アンインストール要求", "accepted", "LIVE_RUNTIME"),
                ("製品アンインストール完了", "completed", "LIVE_RUNTIME"),
                ("更新rollback要求", "rejected", "INTERNAL_STATE"),
                ("更新rollback要求", "queued", "INTERNAL_STATE"),
                ("更新rollback要求", "completed", "INTERNAL_STATE"),
            )
            for index, (operation, decision, evidence_source) in enumerate(audit_cases, start=1):
                event = {
                    "event_id": f"broker-audit-{index}",
                    "request_id": f"request-{index}",
                    "operation": operation,
                    "decision": decision,
                    "reason": (
                        "desktop_native_owner_confirmation_required"
                        if operation in {"製品アンインストール要求", "更新rollback要求"}
                        and decision == "rejected"
                        else "fixture"
                    ),
                    "evidence_source": evidence_source,
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
                f"version=1|event_count={len(events)}|head_event_hash={previous_hash}"
            ).encode()
            anchor_hmac = "sha256:" + hmac.new(key, anchor_input, hashlib.sha256).hexdigest()
            (store / "audit_anchor.json").write_text(
                json.dumps(
                    {
                        "version": 1,
                        "event_count": len(events),
                        "head_event_hash": previous_hash,
                        "anchor_hmac": anchor_hmac,
                    }
                ),
                encoding="utf-8",
            )
            self.assertEqual(
                _verify_live_audit(
                    store, require_uninstall=True, require_rollback=True
                ),
                {
                    "events": 8,
                    "export_accepted": 1,
                    "export_rejected": 1,
                    "uninstall_accepted": 1,
                    "uninstall_rejected": 1,
                    "uninstall_completed": 1,
                    "rollback_rejected": 1,
                    "rollback_queued": 1,
                    "rollback_completed": 1,
                    "catalog_fetch_external_accepted": 0,
                    "update_download_owner_rejected": 0,
                    "update_download_live_completed": 0,
                    "update_stage_owner_rejected": 0,
                    "update_stage_completed": 0,
                    "update_activation_owner_rejected": 0,
                    "update_activation_completed": 0,
                },
            )

            events[0]["reason"] = "tampered"
            (store / "audit.jsonl").write_text(
                "".join(json.dumps(event) + "\n" for event in events),
                encoding="utf-8",
            )
            with self.assertRaisesRegex(RuntimeError, "event hash"):
                _verify_live_audit(store)

    def test_uninstall_keeps_audit_outside_removed_product_and_entry(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            product_root = root / "data/apps/d4-pocket/app-id"
            entry = root / "data/applications/d4-pocket.desktop"
            audit_store = root / "state/d4-pocket/linux-broker/store"
            user_data = root / "config/owner-data-sentinel/uninstall-preserved.marker"
            product_root.mkdir(parents=True)
            entry.parent.mkdir(parents=True)
            entry.write_text("[Desktop Entry]\n", encoding="utf-8")
            audit_store.mkdir(parents=True)
            (audit_store / "audit.jsonl").write_text("{}\n", encoding="utf-8")
            (audit_store / "audit_anchor.json").write_text("{}", encoding="utf-8")
            user_data.parent.mkdir(parents=True)
            user_data.write_bytes(b"D4P uninstall scope sentinel\n")

            with self.assertRaisesRegex(RuntimeError, "製品rootが残っています"):
                _verify_uninstalled_state(product_root, entry, audit_store, user_data)

            product_root.rmdir()
            entry.unlink()
            self.assertEqual(
                _verify_uninstalled_state(product_root, entry, audit_store, user_data),
                {
                    "product_root": "removed",
                    "desktop_entry": "removed",
                    "audit_store": "retained_outside_product_root",
                    "user_data": "retained_outside_product_root",
                },
            )


if __name__ == "__main__":
    unittest.main()
