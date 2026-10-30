from __future__ import annotations

import hashlib
import hmac
import json
import os
import signal
import stat
import subprocess
import sys
import tempfile
import types
import unittest
from unittest.mock import call, patch
from pathlib import Path

from tooling.tests.linux_installed_product_smoke import (
    _LinuxAgentTaskResponsesServer,
    _activate_accessible,
    _activate_named_control_by_keyboard,
    _actionable_node,
    _accessible_nodes,
    _accessible_snapshot,
    _atspi_desktop_failure_summary,
    _accessible_scope_with_named_descendant,
    _cleanup_linux_agent_task_process_groups,
    _answer_owner_dialog,
    _bounded_owner_text_diagnostics,
    _assert_secret_absent_from_files,
    _catalog_failure_diagnostics,
    _click_accessible_by_coordinates,
    _component_center,
    _capture_gtk_dialog_screenshot,
    _gtk_folder_chooser_diagnostics,
    _gtk_folder_row_candidates,
    _gtk_accessible_or_parent_selected,
    _gtk_location_entry_value,
    _credential_scan_file_role,
    _desktop_entry_targets_fixed_launcher,
    _enter_single_installed_mcp_tool_arguments,
    _expand_single_installed_mcp_tool_list,
    _focused_named_accessible,
    _find_operable_accessible,
    _find_workspace_inspector_control,
    _helper_exit_diagnostics,
    _open_mcp_center_from_installed_ui,
    _resolve_regular_package,
    _read_task_scratch_recovery_state,
    _stop_linux_agent_task_process_group,
    _review_single_installed_mcp_tool_arguments,
    _run_rust_test,
    _safe_log_tail,
    _select_folder_in_gtk_chooser,
    _set_public_text_by_keyboard,
    _secret_service_item_labels,
    _verify_export_manifest,
    _verify_credential_audit,
    _verify_live_audit,
    _verify_workspace_projection,
    _wait_for_workspace_projection,
    _scroll_product_page_down,
    _select_workspace_inspector_visibility,
    _wait_for_workspace_root_value,
    _workspace_root_field_value,
    _workspace_inspector_scope,
    _editable_text_ancestor,
    _open_agent_center_from_installed_ui,
    _open_installed_codex_registration,
    _verify_uninstalled_state,
    _trigger_update_action_from_installed_ui,
    _wait_for_catalog_fetch_status,
    _wait_for_gtk_dialog_closed,
    _write_linux_workspace_acceptance_fixture,
    _write_linux_agent_task_fixture,
    _wayland_launch_environment,
    MCP_OWNER_ARGUMENT_HASH_MARKER,
    MCP_OWNER_NO_CREDENTIAL_MARKER,
    MCP_OWNER_WORKSPACE_LABEL,
    _owner_dialog_timeout_diagnostics,
    _wait_for_owner_dialog_window,
    WINDOW_TITLE,
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

    def test_coordinate_activation_clicks_the_observed_accessible_center(self) -> None:
        from unittest.mock import patch

        environment = {"DISPLAY": ":99"}
        with (
            patch(
                "tooling.tests.linux_installed_product_smoke._component_center",
                return_value=(25, 40),
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke.subprocess.run",
                return_value=subprocess.CompletedProcess(args=[], returncode=0),
            ) as run,
        ):
            _click_accessible_by_coordinates(object(), "visible row", environment)

        run.assert_called_once_with(
            ["xdotool", "mousemove", "--sync", "25", "40", "click", "1"],
            env=environment,
            capture_output=True,
            check=False,
            timeout=5,
        )

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

    def test_accessible_tree_traversal_stops_at_deadline(self) -> None:
        class Root:
            name = "root"
            childCount = 1

            def getChildAtIndex(self, index: int):
                raise AssertionError("期限後の子要素を取得してはいけない")

        with patch(
            "tooling.tests.linux_installed_product_smoke.time.monotonic",
            side_effect=(0.0, 1.0),
        ):
            observed = list(_accessible_nodes(Root(), deadline=1.0))

        self.assertEqual(len(observed), 1)
        self.assertEqual(observed[0][1], "root")

    def test_accessible_tree_traversal_can_limit_desktop_search_depth(self) -> None:
        class Node:
            def __init__(self, name: str, children=()) -> None:
                self.name = name
                self.children = list(children)
                self.childCount = len(self.children)

            def getChildAtIndex(self, index: int):
                return self.children[index]

        target = Node("target")
        nested_app = Node("application", (Node("window", (target,)),))
        desktop = Node("desktop", (nested_app,))

        observed = list(_accessible_nodes(desktop, max_depth=1))

        self.assertEqual([name for _, name in observed], ["desktop", "application"])

    def test_exact_operable_lookup_skips_dialog_title_substring(self) -> None:
        from types import SimpleNamespace
        from unittest.mock import patch

        accept = self.Node("選択", ("click",))

        class Root:
            name = "D4 Pocket 作業領域の選択"
            childCount = 1

            def getChildAtIndex(self, index: int):
                return accept

        fake_pyatspi = SimpleNamespace(Registry=SimpleNamespace())
        with patch.dict("sys.modules", {"pyatspi": fake_pyatspi}):
            self.assertIs(
                _find_operable_accessible("選択", root=Root(), exact=True), accept
            )

    def test_workspace_visibility_opens_exact_current_dropdown_before_overlay_option(self) -> None:
        inspector, product_window = object(), object()
        current_value, full_option = object(), object()
        environment = {"DISPLAY": ":99"}
        with (
            patch(
                "tooling.tests.linux_installed_product_smoke._workspace_inspector_scope",
                return_value=(None, inspector),
            ) as find_inspector,
            patch(
                "tooling.tests.linux_installed_product_smoke._find_operable_accessible",
                side_effect=(current_value, full_option),
            ) as find_option,
            patch(
                "tooling.tests.linux_installed_product_smoke._find_accessible",
                return_value=product_window,
            ) as find_window,
            patch(
                "tooling.tests.linux_installed_product_smoke._activate_accessible"
            ) as activate,
        ):
            _select_workspace_inspector_visibility(
                "workspace-a", "full", environment
            )

        find_inspector.assert_called_once_with("workspace-a", timeout=10)
        self.assertEqual(
            find_option.call_args_list,
            [
                call("hash_only", root=inspector, timeout=10, exact=True),
                call("full", root=product_window, timeout=10, exact=True),
            ],
        )
        find_window.assert_called_once_with(WINDOW_TITLE, timeout=10)
        self.assertEqual(
            activate.call_args_list,
            [
                call(current_value, "hash_only", environment),
                call(full_option, "full", environment),
            ],
        )

    def test_workspace_visibility_failure_captures_inspector_window(self) -> None:
        inspector, product_window, current_value = object(), object(), object()
        environment = {"DISPLAY": ":99"}
        destination = Path("/tmp/workspace-inspector-failure.png")
        with (
            patch(
                "tooling.tests.linux_installed_product_smoke._workspace_inspector_scope",
                return_value=(None, inspector),
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke._find_operable_accessible",
                side_effect=(current_value, RuntimeError("full option missing")),
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke._find_accessible",
                return_value=product_window,
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke._activate_accessible"
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke._capture_gtk_dialog_screenshot",
                return_value=True,
            ) as capture,
        ):
            with self.assertRaisesRegex(RuntimeError, "full option missing"):
                _select_workspace_inspector_visibility(
                    "workspace-a",
                    "full",
                    environment,
                    diagnostic_screenshot_path=destination,
                )

        capture.assert_called_once_with(environment, WINDOW_TITLE, destination)

    def test_workspace_scope_includes_rows_sibling_to_the_heading(self) -> None:
        class TreeNode:
            def __init__(self, name: str, children=()) -> None:
                self.name = name
                self.parent = None
                self.children = list(children)
                self.childCount = len(self.children)
                for child in self.children:
                    child.parent = self

            def get_parent(self):
                return self.parent

            def getChildAtIndex(self, index: int):
                return self.children[index]

        heading = TreeNode("作業領域インスペクタ")
        registration_row = TreeNode("workspace-linux-installed-acceptance")
        inspector_column = TreeNode("", (heading, registration_row))
        page = TreeNode("Agent Center", (inspector_column,))

        self.assertIs(
            _accessible_scope_with_named_descendant(
                heading, "workspace-linux-installed-acceptance"
            ),
            inspector_column,
        )

    def test_agent_task_scope_includes_task_input_sibling_to_dialog_title(self) -> None:
        class TreeNode:
            def __init__(self, name: str, children=()) -> None:
                self.name = name
                self.parent = None
                self.children = list(children)
                self.childCount = len(self.children)
                for child in self.children:
                    child.parent = self

            def get_parent(self):
                return self.parent

            def getChildAtIndex(self, index: int):
                return self.children[index]

        title = TreeNode("Agent Task能力の事前検査")
        task_field = TreeNode("Task指示")
        dialog_content = TreeNode("", (title, task_field))
        product_window = TreeNode("D4 Pocket", (dialog_content,))

        self.assertIs(
            _accessible_scope_with_named_descendant(title, "Task指示"),
            dialog_content,
        )

    def test_mcp_palette_submits_ascii_query_and_refreshes_window_tree(self) -> None:
        window = object()
        fresh_window = object()
        with (
            patch(
                "tooling.tests.linux_installed_product_smoke._find_accessible",
                side_effect=(
                    window,
                    window,
                    object(),
                    object(),
                    fresh_window,
                    fresh_window,
                ),
            ) as find_accessible,
            patch(
                "tooling.tests.linux_installed_product_smoke.subprocess.run"
            ) as run,
        ):
            self.assertIs(
                _open_mcp_center_from_installed_ui({}, "0x42"), fresh_window
            )

        self.assertEqual(
            find_accessible.call_args_list,
            [
                call("D4 Pocket powered by GUI Shell", timeout=10),
                call("コマンドパレット", root=window, timeout=10),
                call(
                    "MCP接続一覧とOwner確認付き切断を開く",
                    root=window,
                    timeout=10,
                ),
                call("MCP接続センター", timeout=10),
                call("D4 Pocket powered by GUI Shell", timeout=10),
                call("MCP接続センター", root=fresh_window, timeout=10),
            ],
        )
        self.assertEqual(
            [entry.args[0] for entry in run.call_args_list],
            [
                ["xdotool", "windowactivate", "--sync", "0x42"],
                ["xdotool", "key", "--clearmodifiers", "ctrl+k"],
                [
                    "xdotool",
                    "type",
                    "--clearmodifiers",
                    "--delay",
                    "30",
                    "mcp",
                ],
                ["xdotool", "key", "--clearmodifiers", "Return"],
            ],
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

    def test_installed_mcp_owner_markers_match_the_broker_summary(self) -> None:
        root = Path(__file__).resolve().parents[2]
        broker = (root / "native/rust_helper/src/broker/mcp_center.rs").read_text(
            encoding="utf-8"
        )
        acceptance = (root / "tooling/tests/linux_installed_product_smoke.py").read_text(
            encoding="utf-8"
        )
        self.assertIn(f"{MCP_OWNER_WORKSPACE_LABEL}{{}}", broker)
        self.assertIn(MCP_OWNER_ARGUMENT_HASH_MARKER, broker)
        self.assertIn(MCP_OWNER_NO_CREDENTIAL_MARKER, broker)
        self.assertIn('f"{MCP_OWNER_WORKSPACE_LABEL}{workspace}"', acceptance)
        self.assertIn("MCP_OWNER_ARGUMENT_HASH_MARKER,", acceptance)
        self.assertIn("MCP_OWNER_NO_CREDENTIAL_MARKER,", acceptance)

    def test_installed_mcp_owner_summary_is_accepted_before_decision(self) -> None:
        server_id = "linux-installed-mcp-fixture"
        executable = "/usr/bin/python3"
        workspace = "/tmp/d4-pocket-mcp-workspace"
        summary = "\n".join((
            "このstdio Serverを起動して接続します。",
            f"Server ID: {server_id}",
            f"実行file: {executable}",
            f"{MCP_OWNER_WORKSPACE_LABEL}{workspace}",
            "引数: 2項目",
            MCP_OWNER_ARGUMENT_HASH_MARKER + " sha256:" + "a" * 64,
            MCP_OWNER_NO_CREDENTIAL_MARKER,
            "LinuxではDesktop sandboxを付与しません。",
            "要求hash: sha256:" + "b" * 64,
        ))
        dialog = self.Node("D4 Pocket Owner確認", [self.Node("", text=summary)])
        environment = {"DISPLAY": ":99"}
        button = object()
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
                return_value=button,
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke._activate_accessible"
            ) as activate,
        ):
            _answer_owner_dialog(
                environment,
                "拒否",
                expected_summary_markers=(
                    f"Server ID: {server_id}",
                    f"実行file: {executable}",
                    f"{MCP_OWNER_WORKSPACE_LABEL}{workspace}",
                    MCP_OWNER_NO_CREDENTIAL_MARKER,
                ),
            )
        activate.assert_called_once_with(button, "拒否", environment)

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


class InstalledMcpToolListTests(unittest.TestCase):
    def test_expands_tool_list_before_looking_for_echo(self) -> None:
        events: list[tuple[object, ...]] = []
        environment = {"DISPLAY": ":99"}
        window = object()
        tool_list = object()

        def find_operable(name: str, *, root: object, timeout: float):
            events.append(("find-control", name, root, timeout))
            return tool_list

        def activate(control: object, name: str, observed_environment: dict[str, str]):
            events.append(("activate", control, name, observed_environment))

        def find_accessible(name: str, *, root: object, timeout: float):
            events.append(("find-tool", name, root, timeout))
            return object()

        with (
            patch(
                "tooling.tests.linux_installed_product_smoke._find_operable_accessible",
                side_effect=find_operable,
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke._activate_accessible",
                side_effect=activate,
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke._find_accessible",
                side_effect=find_accessible,
            ),
        ):
            _expand_single_installed_mcp_tool_list(environment, window)

        self.assertEqual(
            events,
            [
                ("find-control", "Tool一覧：1件", window, 15),
                ("activate", tool_list, "Tool一覧：1件", environment),
                ("find-tool", "echo", window, 15),
            ],
        )

    def test_argument_field_and_button_are_resolved_from_window_root(self) -> None:
        events: list[tuple[object, ...]] = []
        environment = {"DISPLAY": ":99"}
        window = object()
        confirm_button = object()

        def find_accessible(name: str, *, root: object, timeout: float):
            events.append(("find-title", name, root, timeout))
            return object()

        def set_public_text(
            root: object,
            label: str,
            value: str,
            observed_environment: dict[str, str],
            window_id: str,
        ):
            events.append(("set-text", root, label, value, observed_environment, window_id))

        def find_operable(name: str, *, root: object, timeout: float):
            events.append(("find-button", name, root, timeout))
            return confirm_button

        def activate(control: object, name: str, observed_environment: dict[str, str]):
            events.append(("activate", control, name, observed_environment))

        with (
            patch(
                "tooling.tests.linux_installed_product_smoke._find_accessible",
                side_effect=find_accessible,
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke._set_public_text_by_keyboard",
                side_effect=set_public_text,
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke._find_operable_accessible",
                side_effect=find_operable,
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke._activate_accessible",
                side_effect=activate,
            ),
        ):
            _enter_single_installed_mcp_tool_arguments(environment, window, "0x42")

        self.assertEqual(
            events,
            [
                ("find-title", "echo のarguments", window, 10),
                (
                    "set-text",
                    window,
                    "JSON形式のobject",
                    '{"text": "installed-mcp-public-input"}',
                    environment,
                    "0x42",
                ),
                ("find-button", "入力内容を確認", window, 10),
                ("activate", confirm_button, "入力内容を確認", environment),
            ],
        )

    def test_argument_preview_uses_accessible_text_snapshot_before_continue(self) -> None:
        events: list[tuple[object, ...]] = []
        environment = {"DISPLAY": ":99"}
        window = object()
        proceed_button = object()

        def find_accessible(name: str, *, root: object, timeout: float):
            events.append(("find", name, root, timeout))
            return object()

        def accessible_snapshot(root: object):
            events.append(("snapshot", root))
            return (
                "送信する引数を確認\n"
                '{"text": "installed-mcp-public-input"}\n'
                "Tool結果本文は返却されず"
            )

        def find_operable(name: str, *, root: object, timeout: float):
            events.append(("find-button", name, root, timeout))
            return proceed_button

        def activate(control: object, name: str, observed_environment: dict[str, str]):
            events.append(("activate", control, name, observed_environment))

        with (
            patch(
                "tooling.tests.linux_installed_product_smoke._find_accessible",
                side_effect=find_accessible,
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke._accessible_snapshot",
                side_effect=accessible_snapshot,
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke._find_operable_accessible",
                side_effect=find_operable,
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke._activate_accessible",
                side_effect=activate,
            ),
        ):
            _review_single_installed_mcp_tool_arguments(environment, window)

        self.assertEqual(
            events,
            [
                ("find", "送信する引数を確認", window, 10),
                ("snapshot", window),
                ("find-button", "Linux確認へ進む", window, 10),
                ("activate", proceed_button, "Linux確認へ進む", environment),
            ],
        )

    def test_argument_preview_missing_public_input_fails_before_native_owner_step(
        self,
    ) -> None:
        window = object()
        with (
            patch(
                "tooling.tests.linux_installed_product_smoke._find_accessible"
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke._accessible_snapshot",
                return_value="送信する引数を確認\nTool結果本文は返却されず",
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke._find_operable_accessible"
            ) as find_button,
        ):
            with self.assertRaisesRegex(RuntimeError, "公開引数"):
                _review_single_installed_mcp_tool_arguments({}, window)

        find_button.assert_not_called()


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
    def test_rust_test_uses_runner_tool_cache_without_changing_isolated_home(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            runner_home = Path(directory) / "runner-home"
            runner_home.mkdir()
            acceptance_home = str(Path(directory) / "acceptance-home")
            with (
                patch.dict(os.environ, {"HOME": str(runner_home)}, clear=True),
                patch(
                    "tooling.tests.linux_installed_product_smoke.subprocess.run",
                    return_value=subprocess.CompletedProcess([], 0, "", ""),
                ) as run,
            ):
                _run_rust_test(
                    "safe-test",
                    {"HOME": acceptance_home},
                    Path(directory) / "private-test-key.der",
                )

        test_environment = run.call_args.kwargs["env"]
        self.assertEqual(test_environment["HOME"], acceptance_home)
        self.assertEqual(test_environment["CARGO_HOME"], str(runner_home / ".cargo"))
        self.assertEqual(test_environment["RUSTUP_HOME"], str(runner_home / ".rustup"))

    def test_credential_scan_roles_keep_xdg_user_data_distinct(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            user_data = root / "home/.local/share/app-data/state.bin"
            home_other = root / "home/.ssh/config.bin"
            user_data.parent.mkdir(parents=True)
            home_other.parent.mkdir(parents=True)
            self.assertEqual(_credential_scan_file_role(user_data, root), "user_data")
            self.assertEqual(_credential_scan_file_role(home_other, root), "home_other")

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

    def test_public_text_preserves_bounded_multiline_argument_fields(self) -> None:
        class TextField:
            value = ""

            def queryEditableText(self):
                return self

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
                field.value += command[-1]
            elif command[1] == "key" and command[-1] == "ctrl+a":
                field.value = ""
            elif command[1] == "key" and command[-1] == "Return":
                field.value += "\n"
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
                object(),
                "起動引数（1行に1項目、最大32項目）",
                "/path/to/mcp-fixture.py\nseed",
                {},
                "0x42",
            )

        self.assertEqual(field.value, "/path/to/mcp-fixture.py\nseed")
        self.assertEqual(
            [command[-1] for command in commands if command[1] == "type"],
            ["/path/to/mcp-fixture.py", "seed"],
        )
        self.assertEqual(
            sum(command[1] == "key" and command[-1] == "Return" for command in commands),
            1,
        )

    def test_public_text_rejects_carriage_return_and_nonprintable_controls(self) -> None:
        with self.assertRaisesRegex(RuntimeError, "許容形式"):
            _set_public_text_by_keyboard(
                object(), "field", "first\r\nsecond", {}, "0x42"
            )
        with self.assertRaisesRegex(RuntimeError, "許容形式"):
            _set_public_text_by_keyboard(
                object(), "field", "first\x00second", {}, "0x42"
            )

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
                raise AssertionError("想定外のSecret Serviceプロパティ要求です")

            def GetSecret(self, *_args):
                raise AssertionError("Secret Serviceからの秘密取得は禁止です")

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
                raise dbus_exception("秘密を含み得る保管処理の詳細")

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
            self.assertRaisesRegex(RuntimeError, "メタデータ検索に失敗"),
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
                    "reason": "メタデータのみ",
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
            with self.assertRaisesRegex(RuntimeError, "永続監査に漏えい"):
                _verify_credential_audit(store, "secret-not-in-audit")

    def test_acceptance_file_scan_reports_leak_without_echoing_value(self) -> None:
        marker = "test-secret-that-must-not-escape"
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "test.log").write_text(marker, encoding="utf-8")
            with self.assertRaisesRegex(RuntimeError, "受入れファイルまたはログへ漏えい") as error:
                _assert_secret_absent_from_files(root, marker)
        self.assertNotIn(marker, str(error.exception))

    def test_acceptance_file_scan_detects_secret_across_read_chunks(self) -> None:
        marker = "test-secret-that-must-not-escape"
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "test.log").write_text("prefix-" + marker + "-suffix", encoding="utf-8")
            with (
                patch(
                    "tooling.tests.linux_installed_product_smoke._CREDENTIAL_SCAN_CHUNK_BYTES",
                    7,
                ),
                self.assertRaisesRegex(RuntimeError, "受入れファイルまたはログへ漏えい"),
            ):
                _assert_secret_absent_from_files(root, marker)

    def test_acceptance_file_scan_fails_closed_at_total_byte_budget(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "large.log").write_bytes(b"123456789")
            with (
                patch(
                    "tooling.tests.linux_installed_product_smoke._CREDENTIAL_SCAN_MAX_BYTES",
                    8,
                ),
                self.assertRaisesRegex(RuntimeError, "上限バイト数を超えました") as error,
            ):
                _assert_secret_absent_from_files(root, "test-secret")
        self.assertIn("files_scanned=0", str(error.exception))
        self.assertIn("next_file_bytes=9", str(error.exception))
        self.assertIn("next_file_role=diagnostic_or_text", str(error.exception))
        self.assertIn("scanned_role_totals=none", str(error.exception))
        self.assertNotIn("large.log", str(error.exception))

    def test_acceptance_file_scan_budget_diagnostic_has_role_totals_only(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "first.log").write_bytes(b"1234")
            (root / "second.json").write_bytes(b"5678")
            (root / "third.txt").write_bytes(b"9012")
            with (
                patch(
                    "tooling.tests.linux_installed_product_smoke._CREDENTIAL_SCAN_MAX_BYTES",
                    8,
                ),
                self.assertRaisesRegex(RuntimeError, "上限バイト数を超えました") as error,
            ):
                _assert_secret_absent_from_files(root, "test-secret")
        diagnostic = str(error.exception)
        self.assertIn("scanned_role_totals=diagnostic_or_text(files=2,bytes=8)", diagnostic)
        self.assertNotIn("first.log", diagnostic)
        self.assertNotIn("second.json", diagnostic)
        self.assertNotIn("third.txt", diagnostic)


class LinuxWorkspaceProjectionTests(unittest.TestCase):
    def test_waits_for_public_body_from_accessible_text_interface(self) -> None:
        marker = "D4P_WORKSPACE_PUBLIC_PROJECTION_OK"
        root = InstalledOwnerSummaryTests.Node(
            "Workspace Inspector",
            [
                InstalledOwnerSummaryTests.Node("public.txt"),
                InstalledOwnerSummaryTests.Node("", text=marker),
            ],
        )
        snapshot, evidence = _wait_for_workspace_projection(
            lambda: _accessible_snapshot(root),
            public_markers=("public.txt", marker),
            forbidden_markers=(".env", "D4P_WORKSPACE_SECRET_CANARY"),
            timeout=1,
        )
        self.assertIn(marker, snapshot)
        self.assertEqual(evidence["public_markers_visible"], 2)

    def test_workspace_projection_fails_closed_without_echoing_exposed_secret(self) -> None:
        marker = "D4P_WORKSPACE_SECRET_CANARY_DO_NOT_PRINT"
        root = InstalledOwnerSummaryTests.Node(
            "Workspace Inspector",
            [InstalledOwnerSummaryTests.Node("", text=marker)],
        )
        with self.assertRaisesRegex(
            RuntimeError, "exposed secret or external content"
        ) as error:
            _wait_for_workspace_projection(
                lambda: _accessible_snapshot(root),
                public_markers=("public.txt", "public-content"),
                forbidden_markers=(marker,),
                timeout=1,
            )
        self.assertNotIn(marker, str(error.exception))

    def test_accepts_public_content_only_when_secret_and_external_markers_are_absent(self) -> None:
        evidence = _verify_workspace_projection(
            "Workspace Inspector\npublic.txt\nD4P_WORKSPACE_PUBLIC_PROJECTION_OK",
            public_markers=("public.txt", "D4P_WORKSPACE_PUBLIC_PROJECTION_OK"),
            forbidden_markers=(
                ".env",
                "private/token.txt",
                "external-alias.txt",
                "D4P_WORKSPACE_SECRET_CANARY",
            ),
        )
        self.assertEqual(
            evidence,
            {
                "public_markers_visible": 2,
                "secret_and_external_markers_absent": 4,
            },
        )

    def test_rejects_any_secret_or_external_content_without_echoing_it(self) -> None:
        marker = "D4P_WORKSPACE_SECRET_CANARY_DO_NOT_PRINT"
        with self.assertRaisesRegex(
            RuntimeError, "exposed secret or external content"
        ) as error:
            _verify_workspace_projection(
                f"public.txt\n{marker}",
                public_markers=("public.txt",),
                forbidden_markers=(marker,),
            )
        self.assertNotIn(marker, str(error.exception))

    def test_fails_closed_when_allowed_public_projection_is_missing(self) -> None:
        with self.assertRaisesRegex(RuntimeError, "public Workspace content"):
            _verify_workspace_projection(
                "Workspace Inspector",
                public_markers=("public.txt",),
                forbidden_markers=(".env",),
            )


class LinuxEditableTextAncestorTests(unittest.TestCase):
    class Node:
        def __init__(self, parent=None, editable=False):
            self.parent = parent
            self.editable = editable

        def get_parent(self):
            return self.parent

        def queryEditableText(self):
            if not self.editable:
                raise RuntimeError("not editable")
            return object()

    def test_finds_editable_control_within_bounded_nested_form_tree(self) -> None:
        editable = self.Node(editable=True)
        current = editable
        for _ in range(10):
            current = self.Node(parent=current)
        self.assertIs(_editable_text_ancestor(current), editable)

    def test_fails_closed_when_editable_control_exceeds_depth_bound(self) -> None:
        editable = self.Node(editable=True)
        current = editable
        for _ in range(18):
            current = self.Node(parent=current)
        self.assertIsNone(_editable_text_ancestor(current))


class LinuxInstalledDesktopEntryTests(unittest.TestCase):
    def test_workspace_fixture_is_a_home_child_with_sibling_outside_target(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            acceptance_root = Path(directory) / "acceptance"
            home = acceptance_root / "home"
            acceptance_root.mkdir()
            home.mkdir()

            fixture_root, workspace, decoy, cli_fixture, cli_marker = (
                _write_linux_workspace_acceptance_fixture(acceptance_root, home)
            )

            self.assertEqual(fixture_root, home / "workspace-picker")
            self.assertEqual(workspace, fixture_root / "selected-workspace")
            self.assertEqual(decoy, fixture_root / "decoy-workspace")
            self.assertEqual(workspace.parent.parent, home)
            alias = workspace / "external-alias.txt"
            self.assertTrue(alias.is_symlink())
            self.assertEqual(
                alias.resolve(), fixture_root / "outside-root" / "external.txt"
            )
            self.assertTrue(cli_fixture.is_file())
            self.assertEqual(cli_marker.parent, acceptance_root)

    def test_workspace_fixture_rejects_symlinked_isolated_home(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            acceptance_root = root / "acceptance"
            outside_home = root / "outside-home"
            acceptance_root.mkdir()
            outside_home.mkdir()
            home_link = acceptance_root / "home"
            home_link.symlink_to(outside_home, target_is_directory=True)

            with self.assertRaisesRegex(RuntimeError, "direct isolated child"):
                _write_linux_workspace_acceptance_fixture(
                    acceptance_root, home_link
                )

    def test_gtk_dialog_screenshot_is_private_and_uses_bounded_window_geometry(self) -> None:
        environment = {"DISPLAY": ":99"}
        with tempfile.TemporaryDirectory() as directory:
            destination = Path(directory) / "chooser.png"
            root = object()
            screen = types.SimpleNamespace(get_root_window=lambda: root)
            display = types.SimpleNamespace(get_default_screen=lambda: screen)
            pixbuf = types.SimpleNamespace(
                savev=lambda path, image_type, keys, values: Path(path).write_bytes(
                    b"png-test-double"
                )
            )
            gdk = types.SimpleNamespace(
                Display=types.SimpleNamespace(get_default=lambda: display),
                pixbuf_get_from_window=lambda window, x, y, width, height: (
                    pixbuf
                    if (window, x, y, width, height) == (root, 11, 22, 300, 180)
                    else None
                ),
            )
            gi = types.ModuleType("gi")
            gi.__path__ = []
            gi.require_version = unittest.mock.Mock()
            gi_repository = types.ModuleType("gi.repository")
            gi_repository.Gdk = gdk
            search = subprocess.CompletedProcess(
                args=[], returncode=0, stdout=b"123\n", stderr=b""
            )
            geometry = subprocess.CompletedProcess(
                args=[],
                returncode=0,
                stdout=b"X=11\nY=22\nWIDTH=300\nHEIGHT=180\n",
                stderr=b"",
            )
            with (
                patch.dict(sys.modules, {"gi": gi, "gi.repository": gi_repository}),
                patch(
                    "tooling.tests.linux_installed_product_smoke.subprocess.run",
                    side_effect=(search, geometry),
                ) as run,
            ):
                self.assertTrue(
                    _capture_gtk_dialog_screenshot(
                        environment, "D4 Pocket 作業領域の選択", destination
                    )
                )

            self.assertEqual(destination.read_bytes(), b"png-test-double")
            if os.name == "posix":
                self.assertEqual(stat.S_IMODE(destination.stat().st_mode), 0o600)
            self.assertEqual(run.call_count, 2)

    def test_agent_palette_activates_the_observed_command_item(self) -> None:
        window = object()
        destination_window = object()
        command = object()
        environment = {"DISPLAY": ":99"}
        with (
            patch(
                "tooling.tests.linux_installed_product_smoke._find_accessible",
                side_effect=(window, object(), object(), destination_window, object()),
            ) as find_accessible,
            patch(
                "tooling.tests.linux_installed_product_smoke._find_operable_accessible",
                return_value=command,
            ) as find_operable,
            patch(
                "tooling.tests.linux_installed_product_smoke._activate_accessible"
            ) as activate,
            patch(
                "tooling.tests.linux_installed_product_smoke.subprocess.run"
            ) as run,
        ):
            self.assertIs(
                _open_agent_center_from_installed_ui(environment, "0x42"),
                destination_window,
            )

        find_operable.assert_called_once_with(
            "エージェントセンターを開く", root=window, timeout=10
        )
        activate.assert_called_once_with(command, "エージェントセンターを開く", environment)
        self.assertEqual(
            [entry.args[0] for entry in run.call_args_list],
            [
                ["xdotool", "windowactivate", "--sync", "0x42"],
                ["xdotool", "key", "--clearmodifiers", "ctrl+k"],
            ],
        )
        self.assertEqual(find_accessible.call_count, 5)

    def test_missing_agent_registration_action_reports_broker_unavailable(self) -> None:
        environment = {"DISPLAY": ":99"}
        with (
            patch(
                "tooling.tests.linux_installed_product_smoke._visible_product_window",
                return_value="0x42",
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke._open_agent_center_from_installed_ui",
                return_value=object(),
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke._find_operable_accessible",
                side_effect=RuntimeError("control missing"),
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke._accessible_snapshot",
                return_value="スナップショット: broker_unavailable",
            ),
        ):
            with self.assertRaisesRegex(RuntimeError, "broker_unavailable"):
                _open_installed_codex_registration(environment)

    def test_agent_registration_reuses_supplied_window_when_lookup_is_transiently_empty(self) -> None:
        environment = {"DISPLAY": ":99"}
        window = object()
        button = object()
        with (
            patch(
                "tooling.tests.linux_installed_product_smoke._visible_product_window",
                return_value=None,
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke._open_agent_center_from_installed_ui",
                return_value=window,
            ) as open_agent_center,
            patch(
                "tooling.tests.linux_installed_product_smoke._find_operable_accessible",
                return_value=button,
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke._activate_accessible"
            ) as activate,
            patch(
                "tooling.tests.linux_installed_product_smoke._find_accessible"
            ),
        ):
            self.assertIs(
                _open_installed_codex_registration(environment, window_id="0x42"),
                window,
            )

        open_agent_center.assert_called_once_with(environment, "0x42")
        activate.assert_called_once_with(button, "登録を開始", environment)

    def test_gtk_chooser_close_timeout_names_the_response_action(self) -> None:
        for action in (
            "absolute folder path entry and accept action",
            "cancel control click",
        ):
            with self.subTest(action=action):
                with self.assertRaisesRegex(RuntimeError, action):
                    _wait_for_gtk_dialog_closed(
                        {"DISPLAY": ":99"},
                        "D4 Pocket 作業領域の選択",
                        action=action,
                        timeout=0,
                    )

    def test_workspace_root_value_reacquires_the_current_product_window(self) -> None:
        window = object()
        field = types.SimpleNamespace(queryEditableText=lambda: object())
        expected = "/home/test/workspace"
        with (
            patch(
                "tooling.tests.linux_installed_product_smoke._find_accessible",
                side_effect=(window, field),
            ) as find,
            patch(
                "tooling.tests.linux_installed_product_smoke._accessible_text",
                return_value=expected,
            ),
        ):
            actual = _workspace_root_field_value(timeout=2)

        self.assertEqual(actual, expected)
        self.assertEqual(
            find.call_args_list,
            [
                call(WINDOW_TITLE, timeout=2),
                call("Workspace rootの絶対path", root=window, timeout=2),
            ],
        )

    def test_workspace_root_value_waits_for_exact_update_and_fails_closed(self) -> None:
        expected = "/home/test/selected-workspace"
        with (
            patch(
                "tooling.tests.linux_installed_product_smoke._workspace_root_field_value",
                side_effect=("/home/test/decoy-workspace", expected),
            ) as read,
            patch("tooling.tests.linux_installed_product_smoke.time.sleep") as sleep,
        ):
            _wait_for_workspace_root_value(expected, timeout=1)
        self.assertEqual(read.call_count, 2)
        sleep.assert_called_once()

        with (
            patch(
                "tooling.tests.linux_installed_product_smoke.time.monotonic",
                side_effect=(0.0, 0.0, 0.02),
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke._workspace_root_field_value",
                return_value="/home/test/decoy-workspace",
            ),
            patch("tooling.tests.linux_installed_product_smoke.time.sleep"),
        ):
            with self.assertRaisesRegex(
                RuntimeError,
                "GTK chooser selection did not update the Workspace input.*"
                "expected_basename=selected-workspace.*observed_basename=decoy-workspace",
            ) as failure:
                _wait_for_workspace_root_value(expected, timeout=0.01)
        self.assertNotIn(expected, str(failure.exception))
        self.assertNotIn("/home/test/decoy-workspace", str(failure.exception))

    def test_workspace_inspector_control_retains_live_scope_while_scrolling(self) -> None:
        window_root = object()
        first_window, second_window = object(), object()
        first_scope, second_scope, control = object(), object(), object()
        with (
            patch(
                "tooling.tests.linux_installed_product_smoke._workspace_inspector_scope",
                side_effect=((first_window, first_scope), (second_window, second_scope)),
            ) as find_scope,
            patch(
                "tooling.tests.linux_installed_product_smoke._find_operable_accessible",
                side_effect=(RuntimeError("not visible"), control),
            ) as find_control,
            patch(
                "tooling.tests.linux_installed_product_smoke._visible_product_window",
                return_value="0x42",
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke._scroll_product_page_down"
            ) as scroll,
            patch("tooling.tests.linux_installed_product_smoke.time.sleep"),
        ):
            actual = _find_workspace_inspector_control(
                "workspace-a",
                "workspace-a",
                {"DISPLAY": ":99"},
                timeout=1,
                accessibility_root=window_root,
            )

        self.assertIs(actual, control)
        self.assertEqual(find_scope.call_count, 2)
        self.assertEqual(
            [entry.args[0] for entry in find_scope.call_args_list],
            ["workspace-a", "workspace-a"],
        )
        self.assertTrue(
            all(0 < entry.kwargs["timeout"] <= 1 for entry in find_scope.call_args_list)
        )
        self.assertEqual(
            [entry.kwargs["root"] for entry in find_scope.call_args_list],
            [window_root, window_root],
        )
        self.assertEqual(
            [entry.kwargs["root"] for entry in find_control.call_args_list],
            [first_scope, second_scope],
        )
        self.assertEqual(find_control.call_count, 2)
        scroll.assert_called_once_with({"DISPLAY": ":99"}, "0x42")

    def test_workspace_inspector_scope_uses_heading_before_window_title(self) -> None:
        heading, inspector = object(), object()
        with (
            patch(
                "tooling.tests.linux_installed_product_smoke.time.monotonic",
                return_value=1.0,
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke._find_accessible",
                return_value=heading,
            ) as find,
            patch(
                "tooling.tests.linux_installed_product_smoke._accessible_scope_with_named_descendant",
                return_value=inspector,
            ) as find_scope,
        ):
            window, actual = _workspace_inspector_scope(
                "workspace-a", timeout=5
            )

        self.assertIsNone(window)
        self.assertIs(actual, inspector)
        self.assertEqual(find.call_count, 1)
        self.assertEqual(find.call_args.args[0], "作業領域インスペクタ")
        self.assertIsNone(find.call_args.kwargs.get("root"))
        self.assertEqual(find.call_args.kwargs["max_depth"], 20)
        self.assertIs(find_scope.call_args.args[0], heading)
        self.assertEqual(find_scope.call_args.args[1], "workspace-a")

    def test_workspace_inspector_scope_anchors_to_registered_workspace_row(self) -> None:
        window, registration, inspector = object(), object(), object()
        with (
            patch(
                "tooling.tests.linux_installed_product_smoke.time.monotonic",
                return_value=1.0,
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke._find_accessible",
                return_value=registration,
            ) as find,
            patch(
                "tooling.tests.linux_installed_product_smoke._accessible_scope_with_named_descendant",
                return_value=inspector,
            ) as find_scope,
        ):
            actual_window, actual_scope = _workspace_inspector_scope(
                "workspace-a", timeout=5, root=window
            )

        self.assertIs(actual_window, window)
        self.assertIs(actual_scope, inspector)
        find.assert_called_once_with(
            "workspace-a",
            root=window,
            timeout=3.0,
            max_depth=20,
        )
        self.assertIs(find_scope.call_args.args[0], registration)
        self.assertEqual(find_scope.call_args.args[1], "登録状態を更新")

    def test_workspace_inspector_scope_keeps_window_scoped_fallback(self) -> None:
        window, heading, inspector = object(), object(), object()
        with (
            patch(
                "tooling.tests.linux_installed_product_smoke.time.monotonic",
                side_effect=(1.0, 1.1, 1.2),
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke._find_accessible",
                side_effect=(
                    RuntimeError("desktop heading missing"),
                    window,
                    heading,
                ),
            ) as find,
            patch(
                "tooling.tests.linux_installed_product_smoke._accessible_scope_with_named_descendant",
                return_value=inspector,
            ),
        ):
            actual_window, actual_scope = _workspace_inspector_scope(
                "workspace-a", timeout=5
            )
        self.assertIs(actual_window, window)
        self.assertIs(actual_scope, inspector)
        self.assertEqual(
            [call.args[0] for call in find.call_args_list],
            ["作業領域インスペクタ", WINDOW_TITLE, "作業領域インスペクタ"],
        )
        self.assertIs(find.call_args_list[2].kwargs["root"], window)

    def test_workspace_inspector_failure_captures_only_the_named_product_window(self) -> None:
        from pathlib import Path

        destination = Path("/tmp/workspace-inspector-failure.png")
        with (
            patch(
                "tooling.tests.linux_installed_product_smoke.time.monotonic",
                side_effect=(1.0, 1.0),
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke._visible_product_window",
                return_value="0x42",
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke._capture_gtk_dialog_screenshot",
                return_value=True,
            ) as capture,
            patch(
                "tooling.tests.linux_installed_product_smoke._atspi_desktop_failure_summary",
                return_value='{"status":"process_timeout"}',
            ),
            self.assertRaisesRegex(
                RuntimeError,
                "x11_window_visible=True; screenshot_captured=True",
            ),
        ):
            _find_workspace_inspector_control(
                "workspace-a",
                "workspace-a",
                {"DISPLAY": ":99"},
                timeout=0,
                diagnostic_screenshot_path=destination,
            )

        capture.assert_called_once_with(
            {"DISPLAY": ":99"}, WINDOW_TITLE, destination
        )

    def test_atspi_failure_summary_is_bounded_and_drops_accessible_names(self) -> None:
        payload = {
            "status": "ok",
            "elapsed_ms": 17,
            "node_count": 1,
            "targets": [True, False, True],
            "nodes": [
                {
                    "depth": 1,
                    "role": "frame",
                    "children": 2,
                    "name": "PRIVATE_PATH_SHOULD_NOT_ESCAPE",
                }
            ],
        }
        result = subprocess.CompletedProcess(
            [], 0, json.dumps(payload).encode("utf-8"), b""
        )
        with patch(
            "tooling.tests.linux_installed_product_smoke.subprocess.run",
            return_value=result,
        ) as run:
            summary = _atspi_desktop_failure_summary(
                {"DISPLAY": ":99"}, "workspace-a"
            )

        parsed = json.loads(summary)
        self.assertEqual(parsed["targets"], {
            "product_window": True,
            "inspector_heading": False,
            "workspace_id": True,
        })
        self.assertEqual(parsed["nodes"], [{"depth": 1, "role": "frame", "children": 2}])
        self.assertNotIn("PRIVATE_PATH_SHOULD_NOT_ESCAPE", summary)
        self.assertEqual(run.call_args.kwargs["timeout"], 3)
        compile(run.call_args.args[0][2], "<atspi-diagnostic>", "exec")

    def test_atspi_failure_summary_reports_timeout_without_partial_tree(self) -> None:
        with patch(
            "tooling.tests.linux_installed_product_smoke.subprocess.run",
            side_effect=subprocess.TimeoutExpired([sys.executable], 3),
        ) as run:
            summary = _atspi_desktop_failure_summary(
                {"DISPLAY": ":99"}, "workspace-a"
            )
        self.assertEqual(json.loads(summary), {"status": "process_timeout"})
        self.assertEqual(run.call_args.kwargs["timeout"], 3)

    def test_product_page_scroll_uses_bounded_window_relative_coordinates(self) -> None:
        completed = subprocess.CompletedProcess([], 0, b"", b"")
        geometry = subprocess.CompletedProcess(
            [], 0, b"X=1\nY=2\nWIDTH=1200\nHEIGHT=800\n", b""
        )
        environment = {"DISPLAY": ":99"}
        with patch(
            "tooling.tests.linux_installed_product_smoke.subprocess.run",
            side_effect=(completed, geometry, completed),
        ) as run:
            _scroll_product_page_down(environment, "0x42")

        self.assertEqual(
            [entry.args[0] for entry in run.call_args_list],
            [
                ["xdotool", "windowactivate", "--sync", "0x42"],
                ["xdotool", "getwindowgeometry", "--shell", "0x42"],
                [
                    "xdotool",
                    "mousemove",
                    "--window",
                    "0x42",
                    "900",
                    "600",
                    "click",
                    "5",
                ],
            ],
        )

        with (
            patch(
                "tooling.tests.linux_installed_product_smoke.subprocess.run"
            ) as run,
            self.assertRaisesRegex(RuntimeError, "window IDが不正"),
        ):
            _scroll_product_page_down(environment, "not-a-window")
        run.assert_not_called()

    def test_folder_chooser_navigates_from_home_by_exact_folder_rows_then_accepts(self) -> None:
        from unittest.mock import call

        with tempfile.TemporaryDirectory() as directory:
            home = Path(directory) / "home"
            picker_root = home / "workspace-picker"
            folder = picker_root / "selected-workspace"
            folder.mkdir(parents=True)
            environment = {"DISPLAY": ":99", "HOME": str(home)}
            chooser = object()
            home_control = object()
            picker_row = object()
            workspace_row = object()
            accept = object()
            visible = subprocess.CompletedProcess(
                args=[], returncode=0, stdout=b"0x42\n", stderr=b""
            )
            with (
                patch(
                    "tooling.tests.linux_installed_product_smoke.subprocess.run",
                    side_effect=(None, visible, visible),
                ) as run,
                patch(
                    "tooling.tests.linux_installed_product_smoke._find_accessible",
                    return_value=chooser,
                ),
                patch(
                    "tooling.tests.linux_installed_product_smoke._find_operable_accessible",
                    side_effect=(home_control, accept),
                ) as find_operable,
                patch(
                    "tooling.tests.linux_installed_product_smoke._activate_accessible"
                ) as activate,
                patch(
                    "tooling.tests.linux_installed_product_smoke._wait_for_gtk_folder_row",
                    side_effect=(picker_row, workspace_row),
                ) as find_row,
                patch(
                    "tooling.tests.linux_installed_product_smoke._double_click_accessible_by_coordinates"
                ) as double_click,
                patch(
                    "tooling.tests.linux_installed_product_smoke._click_accessible_by_coordinates"
                ) as click_row,
                patch(
                    "tooling.tests.linux_installed_product_smoke._wait_for_gtk_folder_row_selected"
                ) as wait_selected,
                patch(
                    "tooling.tests.linux_installed_product_smoke._wait_for_gtk_dialog_closed"
                ) as wait_closed,
            ):
                _select_folder_in_gtk_chooser(environment, "0x42", folder)

        self.assertEqual(
            [entry.args[0] for entry in run.call_args_list],
            [
                ["xdotool", "windowactivate", "--sync", "0x42"],
                [
                    "xdotool",
                    "search",
                    "--onlyvisible",
                    "--name",
                    "^D4 Pocket 作業領域の選択$",
                ],
                [
                    "xdotool",
                    "search",
                    "--onlyvisible",
                    "--name",
                    "^D4 Pocket 作業領域の選択$",
                ],
            ],
        )
        self.assertEqual(
            find_operable.call_args_list,
            [
                call("Home", root=chooser, timeout=10, exact=True),
                call("選択", root=chooser, timeout=10, exact=True),
            ],
        )
        self.assertEqual(
            activate.call_args_list,
            [
                call(home_control, "Home", environment),
                call(accept, "選択", environment),
            ],
        )
        self.assertEqual(
            find_row.call_args_list,
            [call(chooser, picker_root), call(chooser, folder)],
        )
        double_click.assert_called_once_with(
            picker_row, "Workspace picker root row", environment
        )
        click_row.assert_called_once_with(
            workspace_row, "Workspace folder row", environment
        )
        wait_selected.assert_called_once_with(chooser, folder)
        wait_closed.assert_called_once_with(
            environment,
            "D4 Pocket 作業領域の選択",
            action="AT-SPI exact accept action",
            timeout=2,
        )

    def test_folder_chooser_fails_closed_if_exact_workspace_row_is_missing(self) -> None:
        from unittest.mock import call

        with tempfile.TemporaryDirectory() as directory:
            home = Path(directory) / "home"
            picker_root = home / "workspace-picker"
            folder = picker_root / "selected-workspace"
            folder.mkdir(parents=True)
            environment = {"DISPLAY": ":99", "HOME": str(home)}
            chooser = object()
            home_control = object()
            picker_row = object()
            visible = subprocess.CompletedProcess(
                args=[], returncode=0, stdout=b"0x42\n", stderr=b""
            )
            with (
                patch(
                    "tooling.tests.linux_installed_product_smoke.subprocess.run",
                    side_effect=(None, visible, visible),
                ),
                patch(
                    "tooling.tests.linux_installed_product_smoke._find_accessible",
                    return_value=chooser,
                ),
                patch(
                    "tooling.tests.linux_installed_product_smoke._find_operable_accessible",
                    return_value=home_control,
                ) as find_operable,
                patch(
                    "tooling.tests.linux_installed_product_smoke._wait_for_gtk_folder_row",
                    side_effect=(picker_row, RuntimeError("exact row absent")),
                ),
                patch(
                    "tooling.tests.linux_installed_product_smoke._double_click_accessible_by_coordinates"
                ),
                patch(
                    "tooling.tests.linux_installed_product_smoke._gtk_folder_chooser_diagnostics",
                    return_value={"bounded": True},
                ),
                patch(
                    "tooling.tests.linux_installed_product_smoke._click_accessible_by_coordinates"
                ) as click_row,
            ):
                with self.assertRaisesRegex(RuntimeError, "exact row absent"):
                    _select_folder_in_gtk_chooser(environment, "0x42", folder)

        find_operable.assert_called_once_with(
            "Home", root=chooser, timeout=10, exact=True
        )
        click_row.assert_called_once_with(home_control, "Home", environment)

    def test_folder_chooser_rejects_workspace_outside_isolated_home(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            home = Path(directory) / "home"
            outside = Path(directory) / "outside" / "selected-workspace"
            home.mkdir()
            outside.mkdir(parents=True)
            with patch(
                "tooling.tests.linux_installed_product_smoke.subprocess.run"
            ) as run:
                with self.assertRaisesRegex(RuntimeError, "HOME直下"):
                    _select_folder_in_gtk_chooser(
                        {"DISPLAY": ":99", "HOME": str(home)}, "0x42", outside
                    )
        run.assert_not_called()

    def test_folder_chooser_uses_exact_control_coordinates_if_action_does_not_close(self) -> None:
        from unittest.mock import call

        with tempfile.TemporaryDirectory() as directory:
            home = Path(directory) / "home"
            picker_root = home / "workspace-picker"
            folder = picker_root / "selected-workspace"
            folder.mkdir(parents=True)
            environment = {"DISPLAY": ":99", "HOME": str(home)}
            chooser = object()
            home_control = object()
            picker_row = object()
            workspace_row = object()
            accept = object()
            visible = subprocess.CompletedProcess(
                args=[], returncode=0, stdout=b"0x42\n", stderr=b""
            )
            with (
                patch(
                    "tooling.tests.linux_installed_product_smoke.subprocess.run",
                    side_effect=(None, visible, visible),
                ),
                patch(
                    "tooling.tests.linux_installed_product_smoke._find_accessible",
                    return_value=chooser,
                ),
                patch(
                    "tooling.tests.linux_installed_product_smoke._find_operable_accessible",
                    side_effect=(home_control, accept),
                ),
                patch("tooling.tests.linux_installed_product_smoke._activate_accessible"),
                patch(
                    "tooling.tests.linux_installed_product_smoke._wait_for_gtk_folder_row",
                    side_effect=(picker_row, workspace_row),
                ),
                patch(
                    "tooling.tests.linux_installed_product_smoke._double_click_accessible_by_coordinates"
                ),
                patch(
                    "tooling.tests.linux_installed_product_smoke._wait_for_gtk_folder_row_selected"
                ),
                patch(
                    "tooling.tests.linux_installed_product_smoke._click_accessible_by_coordinates"
                ) as coordinate_click,
                patch(
                    "tooling.tests.linux_installed_product_smoke._wait_for_gtk_dialog_closed",
                    side_effect=(RuntimeError("still open"), None),
                ) as wait_closed,
            ):
                _select_folder_in_gtk_chooser(environment, "0x42", folder)

        self.assertEqual(
            coordinate_click.call_args_list,
            [
                call(workspace_row, "Workspace folder row", environment),
                call(accept, "選択", environment),
            ],
        )
        self.assertEqual(wait_closed.call_count, 2)
        self.assertEqual(
            wait_closed.call_args_list[0].kwargs["action"],
            "AT-SPI exact accept action",
        )
        self.assertEqual(
            wait_closed.call_args_list[1].kwargs["action"],
            "AT-SPI action and exact-control coordinate fallback",
        )

    def test_gtk_location_entry_value_reads_editable_absolute_path(self) -> None:
        class Node:
            def __init__(self, name: str, role: str, bounds, text: str = "") -> None:
                self.name = name
                self.role = role
                self.bounds = bounds
                self.text = text
                self.childCount = 0

            def getRoleName(self) -> str:
                return self.role

            def queryComponent(self):
                from types import SimpleNamespace

                x, y, width, height = self.bounds
                return SimpleNamespace(
                    getExtents=lambda _coordinate: SimpleNamespace(
                        x=x, y=y, width=width, height=height
                    )
                )

            def queryText(self):
                from types import SimpleNamespace

                return SimpleNamespace(getText=lambda _start, _end: self.text)

            def queryEditableText(self):
                return object()

        path = Node("", "text", (650, 0, 490, 34), "/tmp/selected-workspace")
        unrelated = Node("", "text", (650, 160, 400, 30), "/tmp/not-a-location-entry")
        nodes = [(path, ""), (unrelated, "")]
        with (
            patch(
                "tooling.tests.linux_installed_product_smoke._accessible_nodes",
                side_effect=lambda _root: iter(nodes),
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke._accessible_component_bounds",
                side_effect=lambda node: node.bounds,
            ),
        ):
            self.assertEqual(
                _gtk_location_entry_value(object()), "/tmp/selected-workspace"
            )

    def test_folder_chooser_diagnostics_emit_only_bounded_state(self) -> None:
        from types import SimpleNamespace

        class State:
            def contains(self, state: int) -> bool:
                return state == 1

        class Node:
            name = ""
            childCount = 0

            def getRoleName(self) -> str:
                return "table cell"

            def getState(self):
                return State()

        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory) / "selected-workspace"
            with (
                patch.dict("sys.modules", {"pyatspi": SimpleNamespace(STATE_SELECTED=1)}),
                patch(
                    "tooling.tests.linux_installed_product_smoke._accessible_snapshot",
                    return_value=str(folder.parent),
                ),
            ):
                diagnostics = _gtk_folder_chooser_diagnostics(Node(), folder)

        self.assertEqual(
            diagnostics,
            {
                "workspace_parent_path_visible": True,
                "location_entry_matches_workspace_parent": False,
                "location_entry_is_autocompleted_workspace_child": False,
                "workspace_folder_row_visible": False,
                "workspace_folder_row_selected": False,
                "selected_accessible_count": 1,
                "common_roles": [("table cell", 1)],
                "focused_roles": [],
                "visible_header_bounds": [],
                "visible_table_bounds": [],
                "visible_row_entries": [],
                "target_entries": [],
            },
        )
        self.assertNotIn(str(folder.parent), repr(diagnostics))

    def test_folder_row_candidate_is_exact_visible_table_entry(self) -> None:
        from types import SimpleNamespace

        class State:
            def contains(self, state: int) -> bool:
                return state == 1

        class Node:
            def __init__(self, role: str, name: str, bounds) -> None:
                self.role = role
                self.name = name
                self.bounds = bounds

            def getRoleName(self) -> str:
                return self.role

            def getState(self):
                return State()

        table = Node("table", "", (675, 76, 514, 729))
        header = Node("table column header", "Name", (675, 76, 514, 25))
        node = Node("table cell", "selected-workspace", (690, 103, 130, 18))
        sidebar = Node("list item", "selected-workspace", (93, 147, 581, 36))
        nodes = [
            (table, ""),
            (header, "Name"),
            (node, "selected-workspace"),
            (sidebar, "selected-workspace"),
        ]
        bounds = {item: item.bounds for item, _ in nodes}
        with (
            patch.dict(
                "sys.modules",
                {"pyatspi": SimpleNamespace(STATE_SHOWING=1, STATE_SELECTED=2)},
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke._accessible_nodes",
                side_effect=lambda _root: iter(nodes),
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke._accessible_component_bounds",
                side_effect=lambda item: bounds.get(item),
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke._accessible_text",
                return_value="",
            ),
        ):
            with tempfile.TemporaryDirectory() as directory:
                folder = Path(directory) / "selected-workspace"
                self.assertEqual(_gtk_folder_row_candidates(object(), folder), [node])
                self.assertFalse(_gtk_accessible_or_parent_selected(node))

    def test_folder_row_candidate_rejects_ambiguous_visible_matches(self) -> None:
        from types import SimpleNamespace

        class State:
            def contains(self, state: int) -> bool:
                return state == 1

        class Node:
            def __init__(self, role: str, bounds) -> None:
                self.role = role
                self.bounds = bounds

            def getRoleName(self) -> str:
                return self.role

            def getState(self):
                return State()

        table = Node("table", (675, 76, 514, 729))
        header = Node("table column header", (675, 76, 514, 25))
        first = Node("table cell", (700, 120, 130, 24))
        second = Node("table cell", (700, 160, 130, 24))
        nodes = [
            (table, ""),
            (header, "Name"),
            (first, "selected-workspace"),
            (second, "selected-workspace"),
        ]
        bounds = {item: item.bounds for item, _ in nodes}
        with (
            patch.dict(
                "sys.modules",
                {"pyatspi": SimpleNamespace(STATE_SHOWING=1, STATE_SELECTED=2)},
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke._accessible_nodes",
                side_effect=lambda _root: iter(nodes),
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke._accessible_component_bounds",
                side_effect=lambda item: bounds.get(item),
            ),
            patch(
                "tooling.tests.linux_installed_product_smoke._accessible_text",
                return_value="",
            ),
        ):
            with tempfile.TemporaryDirectory() as directory:
                folder = Path(directory) / "selected-workspace"
                self.assertEqual(_gtk_folder_row_candidates(object(), folder), [])

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

    def test_owner_timeout_diagnostics_classify_windows_and_allowlisted_status(self) -> None:
        class Node:
            def __init__(self, name: str, children=()) -> None:
                self.name = name
                self.children = list(children)
                self.childCount = len(self.children)

            def getChildAtIndex(self, index: int):
                return self.children[index]

        root = Node(
            "",
            [
                Node("MCP接続は確定していません"),
                Node("untrusted fixture content must not be logged"),
            ],
        )
        with tempfile.TemporaryDirectory() as directory:
            audit_path = Path(directory) / "audit.jsonl"
            audit_path.write_text(
                json.dumps(
                    {
                        "operation": "MCP接続",
                        "decision": "rejected",
                        "reason": "MCP接続にはowner controlが必要",
                        "untrusted": "fixture secret marker must not be logged",
                    },
                    ensure_ascii=False,
                ),
                encoding="utf-8",
            )
            with patch(
                "tooling.tests.linux_installed_product_smoke.subprocess.run",
                side_effect=(
                    subprocess.CompletedProcess(
                        args=[], returncode=0, stdout=b"0x1\n0x2\n", stderr=b""
                    ),
                    subprocess.CompletedProcess(
                        args=[],
                        returncode=0,
                        stdout=(WINDOW_TITLE + "\n").encode(),
                        stderr=b"",
                    ),
                    subprocess.CompletedProcess(
                        args=[],
                        returncode=0,
                        stdout="D4 Pocket Owner確認\n".encode(),
                        stderr=b"",
                    ),
                ),
            ):
                diagnostics = _owner_dialog_timeout_diagnostics(
                    {"DISPLAY": ":99"}, root, audit_path
                )

        self.assertIn("product:1", diagnostics)
        self.assertIn("owner:1", diagnostics)
        self.assertIn("MCP接続は確定していません", diagnostics)
        self.assertNotIn("untrusted fixture content", diagnostics)
        self.assertIn(
            "mcp_connect_audit=decision:rejected,category:owner_required",
            diagnostics,
        )
        self.assertNotIn("fixture secret marker", diagnostics)

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
                ("MCP接続", "rejected", "INTERNAL_STATE"),
                ("MCP接続", "accepted", "INTERNAL_STATE"),
                ("MCP Tool実行", "approved", "INTERNAL_STATE"),
                ("MCP Tool実行", "started", "INTERNAL_STATE"),
                ("MCP Tool実行", "accepted", "LIVE_RUNTIME"),
                ("MCP切断", "rejected", "INTERNAL_STATE"),
                ("MCP切断", "accepted", "LIVE_RUNTIME"),
                ("作業領域OS選択", "accepted", "INTERNAL_STATE"),
                ("作業領域OS選択", "accepted", "INTERNAL_STATE"),
                ("AgentCLI実行系作業領域登録", "rejected", "INTERNAL_STATE"),
                ("AgentCLI実行系作業領域登録", "accepted", "LIVE_RUNTIME"),
                ("作業領域承認", "recorded", "INTERNAL_STATE"),
                ("作業領域承認", "accepted", "INTERNAL_STATE"),
                ("作業領域ツリー", "accepted", "LIVE_RUNTIME"),
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
                        if operation in {
                            "製品アンインストール要求",
                            "更新rollback要求",
                            "MCP接続",
                            "MCP切断",
                            "AgentCLI実行系作業領域登録",
                        }
                        and decision == "rejected"
                        else (
                            "Capability=mcp.tool.call Permission=one consumed=true"
                            if operation == "MCP Tool実行" and decision == "started"
                            else (
                                "ツール結果のハッシュだけを含む受領票"
                                if operation == "MCP Tool実行" and decision == "accepted"
                                else (
                                    "ownerが現在登録の読取を承認"
                                    if operation == "作業領域承認" and decision == "recorded"
                                    else "fixture"
                                )
                            )
                        )
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
                    store,
                    require_export=True,
                    require_mcp=True,
                    require_workspace=True,
                    require_uninstall=True,
                    require_rollback=True,
                ),
                {
                    "events": 22,
                    "export_accepted": 1,
                    "export_rejected": 1,
                    "mcp_connect_owner_rejected": 1,
                    "mcp_connect_internal_accepted": 1,
                    "mcp_tool_approved": 1,
                    "mcp_tool_permission_consumed": 1,
                    "mcp_tool_live_hash_only_completed": 1,
                    "mcp_disconnect_owner_rejected": 1,
                    "mcp_disconnect_live_completed": 1,
                    "workspace_selection_accepted": 2,
                    "workspace_registration_owner_rejected": 1,
                    "workspace_registration_live_accepted": 1,
                    "workspace_read_approval_recorded": 1,
                    "workspace_read_approval_accepted": 1,
                    "workspace_tree_live_accepted": 1,
                    "agent_task_permission_or_approval_events": 0,
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
            with self.assertRaisesRegex(RuntimeError, "監査事象ハッシュ"):
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


class WorkspaceTaskScratchJournalEvidenceTests(unittest.TestCase):
    def test_empty_journal_hmac_and_rejects_tampering(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            store = Path(directory)
            key = b"k" * 32
            (store / "audit_anchor.key").write_text(key.hex(), encoding="ascii")
            state = {"version": 1, "entries": []}
            serialized_state = json.dumps(
                state, ensure_ascii=False, sort_keys=True, separators=(",", ":")
            )
            signed = {
                "version": 1,
                "state": state,
                "hmac": "sha256:" + hmac.new(
                    key,
                    f"version=1|state={serialized_state}".encode(),
                    hashlib.sha256,
                ).hexdigest(),
            }
            journal = store / "agent_task_scratch_recovery.json"
            journal.write_text(json.dumps(signed), encoding="utf-8")
            self.assertEqual(
                _read_task_scratch_recovery_state(store),
                {"entries": 0, "hmac_present": True},
            )
            signed["state"] = {"version": 1, "entries": [], "tampered": True}
            journal.write_text(json.dumps(signed), encoding="utf-8")
            with self.assertRaisesRegex(RuntimeError, "HMACが一致しません"):
                _read_task_scratch_recovery_state(store)
            signed["state"] = {"version": 1, "entries": [{"task_id": "stale"}]}
            journal.write_text(json.dumps(signed), encoding="utf-8")
            with self.assertRaisesRegex(RuntimeError, "残存entry"):
                _read_task_scratch_recovery_state(store)


class LinuxAgentTaskFailureCleanupTests(unittest.TestCase):
    @staticmethod
    def _write_marker(root: Path, *, pid: int, pgid: int, starttime: int) -> None:
        workspace = root / "agent-task-fixture" / "workspace"
        workspace.mkdir(parents=True)
        (workspace / "agent-task-cancel-active.json").write_text(
            json.dumps(
                {
                    "mode": "cancel",
                    "pid": pid,
                    "pgid": pgid,
                    "starttime": starttime,
                }
            ),
            encoding="utf-8",
        )

    def test_failure_cleanup_stops_only_verified_group_and_escalates(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self._write_marker(root, pid=234, pgid=345, starttime=678)
            members = [(234, 678, "S")]
            with (
                patch(
                    "tooling.tests.linux_installed_product_smoke.os.getpgrp",
                    return_value=999,
                    create=True,
                ),
                patch(
                    "tooling.tests.linux_installed_product_smoke._linux_process_group_members",
                    return_value=members,
                ),
                patch(
                    "tooling.tests.linux_installed_product_smoke._linux_proc_identity",
                    return_value=(345, 678, "S"),
                ),
                patch(
                    "tooling.tests.linux_installed_product_smoke._wait_for_linux_process_group_exit",
                    side_effect=[RuntimeError("still live"), None],
                ),
                patch(
                    "tooling.tests.linux_installed_product_smoke.os.killpg",
                    create=True,
                ) as killpg,
                patch(
                    "tooling.tests.linux_installed_product_smoke.signal.SIGKILL",
                    9,
                    create=True,
                ),
            ):
                _cleanup_linux_agent_task_process_groups(root)
            self.assertEqual(
                killpg.call_args_list,
                [call(345, signal.SIGTERM), call(345, getattr(signal, "SIGKILL", 9))],
            )

    def test_failure_cleanup_refuses_runner_group_and_reused_pid(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self._write_marker(root, pid=234, pgid=345, starttime=678)
            with (
                patch(
                    "tooling.tests.linux_installed_product_smoke.os.getpgrp",
                    return_value=345,
                    create=True,
                ),
                patch(
                    "tooling.tests.linux_installed_product_smoke.os.killpg",
                    create=True,
                ) as killpg,
            ):
                with self.assertRaisesRegex(RuntimeError, "cleanup") as caught:
                    _cleanup_linux_agent_task_process_groups(root)
                self.assertRegex(str(caught.exception.__cause__), "runner自身")
                killpg.assert_not_called()

            with (
                patch(
                    "tooling.tests.linux_installed_product_smoke.os.getpgrp",
                    return_value=999,
                    create=True,
                ),
                patch(
                    "tooling.tests.linux_installed_product_smoke._linux_process_group_members",
                    return_value=[(234, 678, "S")],
                ),
                patch(
                    "tooling.tests.linux_installed_product_smoke._linux_proc_identity",
                    return_value=(345, 679, "S"),
                ),
                patch(
                    "tooling.tests.linux_installed_product_smoke.os.killpg",
                    create=True,
                ) as killpg,
            ):
                with self.assertRaisesRegex(RuntimeError, "cleanup") as caught:
                    _cleanup_linux_agent_task_process_groups(root)
                self.assertRegex(str(caught.exception.__cause__), "identity")
                killpg.assert_not_called()

    def test_failure_cleanup_continues_to_other_run_owned_group(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self._write_marker(root, pid=234, pgid=345, starttime=678)
            workspace = root / "agent-task-fixture" / "workspace"
            (workspace / "agent-task-deadline-active.json").write_text(
                json.dumps(
                    {"mode": "deadline", "pid": 456, "pgid": 567, "starttime": 890}
                ),
                encoding="utf-8",
            )
            with patch(
                "tooling.tests.linux_installed_product_smoke._stop_linux_agent_task_process_group",
                side_effect=[RuntimeError("first group remains live"), None],
            ) as stop:
                with self.assertRaisesRegex(RuntimeError, "cleanup"):
                    _cleanup_linux_agent_task_process_groups(root)
            self.assertEqual(
                stop.call_args_list,
                [call(345, 234, 678), call(567, 456, 890)],
            )


class AgentTaskAuditAcceptanceTests(unittest.TestCase):
    def test_three_independent_terminal_tasks_and_scratch_cleanup_are_audited(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            store = Path(directory)
            key = b"a" * 32
            (store / "audit_anchor.key").write_text(key.hex(), encoding="ascii")
            rows: list[tuple[str, str, str]] = []
            task_ids = ("task-complete", "task-cancel", "task-deadline")
            statuses = ("completed", "cancelled", "failed")
            for index, task_id in enumerate(task_ids):
                rows.extend(
                    (
                        ("AgentTaskWorkspacePermissionGrant", "recorded", "permission"),
                        ("AgentTaskOwnerApprovalGrant", "recorded", "approval"),
                    )
                )
                common = {
                    "version": 1,
                    "task_id": task_id,
                    "runtime_id": "runtime-test",
                    "session_id": "session-test",
                    "workspace_id": "workspace-test",
                    "instruction_hash": f"sha256:{index + 1:064x}",
                    "request_hash": f"sha256:{index + 4:064x}",
                    "created_at": index + 1,
                    "updated_at": index + 1,
                    "result_hash": None,
                    "failure_class": None,
                    "start_audit_event_id": None,
                }
                start = {**common, "stage": "start", "status": "running"}
                terminal = {
                    **common,
                    "stage": "terminal",
                    "status": statuses[index],
                    "failure_class": "期限超過" if statuses[index] == "failed" else None,
                }
                rows.append(
                    (
                        "AgentTask実行",
                        "recorded",
                        "AgentTask履歴:" + json.dumps(start, ensure_ascii=False, separators=(",", ":")),
                    )
                )
                rows.append(
                    (
                        "AgentTask状態",
                        "recorded",
                        "AgentTask履歴:" + json.dumps(terminal, ensure_ascii=False, separators=(",", ":")),
                    )
                )
            rows.append(
                (
                    "AgentTask状態",
                    "recorded",
                    "AgentTask調停作用:" + json.dumps(
                        {"scratch_cleanup_completed": True}, separators=(",", ":")
                    ),
                )
            )
            events: list[dict[str, object]] = []
            previous_hash: str | None = None
            for index, (operation, decision, reason) in enumerate(rows, start=1):
                event = {
                    "event_id": f"broker-audit-{index}",
                    "request_id": f"request-{index}",
                    "operation": operation,
                    "decision": decision,
                    "reason": reason,
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
                previous_hash = "sha256:" + hashlib.sha256(hash_input.encode()).hexdigest()
                event["event_hash"] = previous_hash
                events.append(event)
            (store / "audit.jsonl").write_text(
                "".join(json.dumps(event, ensure_ascii=False) + "\n" for event in events),
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
            evidence = _verify_live_audit(
                store,
                require_export=False,
                require_agent_task=True,
            )
            self.assertEqual(evidence["agent_task_started"], 3)
            self.assertEqual(evidence["agent_task_completed"], 1)
            self.assertEqual(evidence["agent_task_cancelled"], 1)
            self.assertEqual(evidence["agent_task_deadline_failed"], 1)
            self.assertEqual(evidence["agent_task_permission_grants"], 3)
            self.assertEqual(evidence["agent_task_owner_approvals"], 3)
            self.assertEqual(evidence["agent_task_scratch_cleanup_confirmed"], 1)


class LinuxAgentTaskResponsesFixtureTests(unittest.TestCase):
    def test_fixture_builds_codex_function_call_and_final_response(self) -> None:
        server = _LinuxAgentTaskResponsesServer()
        self.addCleanup(server.close)
        server.configure("complete", "/usr/bin/python3 task_fixture.py complete", Path("/tmp/workspace"))
        tool_events = server._response_events(
            {"input": [], "tools": [{"name": "exec_command"}]}
        )
        call_event = next(
            event
            for event in tool_events
            if event["type"] == "response.output_item.done"
            and isinstance(event.get("item"), dict)
            and event["item"].get("type") == "function_call"
        )
        item = call_event["item"]
        call_id = item["call_id"]
        arguments = json.loads(item["arguments"])
        self.assertEqual(arguments["workdir"], str(Path("/tmp/workspace")))
        self.assertEqual(arguments["shell"], "/bin/bash")

        final_events = server._response_events(
            {
                "input": [
                    {
                        "type": "function_call_output",
                        "call_id": call_id,
                        "output": "safe fixture result",
                    }
                ],
                "tools": [{"name": "exec_command"}],
            }
        )
        self.assertIn("response.completed", [event["type"] for event in final_events])
        snapshot = server.snapshot()
        self.assertEqual(snapshot["rejections"], [])
        self.assertEqual(snapshot["scenarios"][0]["tool_outputs"], 1)
        self.assertEqual(snapshot["scenarios"][0]["posts"], 2)

    def test_fixture_rejects_secret_canary_in_tool_output(self) -> None:
        server = _LinuxAgentTaskResponsesServer()
        self.addCleanup(server.close)
        server.configure("cancel", "/usr/bin/sleep 300", Path("/tmp/workspace"))
        call_event = next(
            event
            for event in server._response_events(
                {"input": [], "tools": [{"name": "exec_command"}]}
            )
            if event["type"] == "response.output_item.done"
            and isinstance(event.get("item"), dict)
            and event["item"].get("type") == "function_call"
        )
        with self.assertRaisesRegex(RuntimeError, "secret canary exposed"):
            server._response_events(
                {
                    "input": [
                        {
                            "type": "function_call_output",
                            "call_id": call_event["item"]["call_id"],
                            "output": "D4P_SYNTHETIC_SECRET_CANARY=do-not-forward",
                        }
                    ],
                    "tools": [{"name": "exec_command"}],
                }
            )
        snapshot = server.snapshot()
        self.assertEqual(snapshot["rejections"], ["synthetic_secret_canary_in_tool_output"])
        self.assertNotIn("do-not-forward", json.dumps(snapshot))


@unittest.skipUnless(sys.platform.startswith("linux"), "Linux filesystem semantics are required")
class LinuxAgentTaskFixtureSourceTests(unittest.TestCase):
    def test_generated_task_fixture_compiles_and_uses_workspace_bound_scratch(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-linux-task-fixture-") as temporary:
            fixture_root, workspace, outside_file = _write_linux_agent_task_fixture(
                Path(temporary)
            )
            fixture_source = workspace / "task_fixture.py"
            source = fixture_source.read_text(encoding="utf-8")
            compile(source, str(fixture_source), "exec")
            self.assertEqual(
                outside_file.read_text(encoding="utf-8"),
                "D4P_SYNTHETIC_SECRET_CANARY=outside-workspace\n",
            )
            self.assertTrue((workspace / "outside-link.txt").is_symlink())
            self.assertEqual(
                (workspace / "outside-link.txt").resolve(strict=True),
                outside_file.resolve(strict=True),
            )
            self.assertIn("scratch_workspace_bound = scratch.parent == workspace", source)
            self.assertTrue(fixture_root.is_dir())


if __name__ == "__main__":
    unittest.main()
