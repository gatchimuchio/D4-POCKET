from __future__ import annotations

import json
import os
import shutil
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from tooling import launch_owner_desktop_isolated as launcher


class FakeWorkspace:
    def __init__(self, source_root: Path):
        self.source_root = source_root
        self.temporary_root = Path(tempfile.mkdtemp(prefix="d4p-launch-test-"))
        self.root = self.temporary_root / "source"
        (self.root / "apps" / "desktop_flutter").mkdir(parents=True)
        (self.root / "native" / "rust_helper").mkdir(parents=True)
        self.environment = {
            **os.environ,
            "APPDATA": str(self.temporary_root / "temporary-appdata"),
            "XDG_CONFIG_HOME": str(self.temporary_root / "temporary-xdg"),
            "CARGO_TARGET_DIR": str(self.root / "native" / "rust_helper" / "target"),
            "GUI_SHELL_BROKER_ENDPOINT_JSON": "stale-endpoint",
        }
        self.exited = False
        self.broker_process: FakeProcess | None = None
        self.session_file: Path | None = None

    def __enter__(self) -> FakeWorkspace:
        return self

    def __exit__(self, _exc_type, _exc, _traceback) -> bool:
        self.exited = True
        if self.broker_process is not None and not self.broker_process.terminated:
            raise AssertionError("作業領域を回収する前にBroker processが停止されていない")
        if self.broker_process is not None and self.session_file is not None:
            if not self.session_file.is_file():
                raise AssertionError("作業領域終了前にBroker session fileが削除された")
        shutil.rmtree(self.temporary_root)
        return False


class FakeProcess:
    def __init__(self, returncode: int | None):
        self.returncode = returncode
        self.pid = 12345
        self.terminated = False
        self.waited = False

    def poll(self) -> int | None:
        return self.returncode

    def wait(self, timeout: float | None = None) -> int:
        del timeout
        self.waited = True
        if self.returncode is None:
            self.returncode = -15
        return self.returncode

    def terminate(self) -> None:
        self.terminated = True
        self.returncode = -15

    def kill(self) -> None:
        self.returncode = -9


class OwnerDesktopLauncherTests(unittest.TestCase):
    def test_run_uses_snapshot_targets_short_session_and_stops_broker_before_cleanup(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-launch-test-root-") as temporary:
            source_root = Path(temporary) / "repository"
            source_root.mkdir()
            workspace = FakeWorkspace(source_root)
            processes: list[FakeProcess] = []
            flutter_observation: dict[str, object] = {}

            def fake_popen(command, *, cwd, env, **_kwargs):
                process = FakeProcess(0 if len(processes) != 1 else None)
                processes.append(process)
                command = list(command)
                if command[0] == "cargo":
                    target = Path(env["CARGO_TARGET_DIR"])
                    self.assertTrue(target.is_relative_to(workspace.root))
                    helper_name = "gui_shell_rust_helper.exe" if os.name == "nt" else "gui_shell_rust_helper"
                    helper = target / "debug" / helper_name
                    helper.parent.mkdir(parents=True)
                    helper.write_bytes(b"test-helper")
                elif "broker-server" in command:
                    workspace.broker_process = process
                    session_file = Path(command[command.index("--session-file") + 1])
                    workspace.session_file = session_file
                    session_file.write_text(
                        json.dumps(
                            {
                                "credential_role": "normal",
                                "host": "127.0.0.1",
                                "transport": "authenticated_loopback_tcp",
                                "secret": "never-print-this",
                            }
                        ),
                        encoding="utf-8",
                    )
                    process.returncode = None
                else:
                    flutter_observation.update(command=command, cwd=cwd, env=env)
                    process.returncode = 23
                return process

            with (
                patch.object(launcher, "IsolatedFlutterWorkspace", return_value=workspace),
                patch.object(launcher, "_desktop_device", return_value="windows"),
                patch.object(launcher, "_find_tool", side_effect=lambda name: name),
                patch.object(launcher.subprocess, "Popen", side_effect=fake_popen),
                patch.object(launcher, "_process_options", return_value={}),
                patch.dict(os.environ, {"APPDATA": "user-appdata", "GUI_SHELL_BROKER_ENDPOINT_JSON": "inherited-stale"}),
            ):
                result = launcher.run_owner_desktop(source_root)

            self.assertEqual(result, 23)
            self.assertTrue(workspace.exited)
            self.assertTrue(processes[1].terminated)
            self.assertTrue(processes[1].waited)
            self.assertEqual(flutter_observation["command"], ["flutter", "run", "-d", "windows"])
            self.assertEqual(flutter_observation["cwd"], workspace.root / "apps" / "desktop_flutter")
            environment = flutter_observation["env"]
            self.assertEqual(environment["APPDATA"], "user-appdata")
            self.assertNotEqual(environment["GUI_SHELL_BROKER_ENDPOINT_JSON"], "inherited-stale")
            self.assertTrue(Path(environment["GUI_SHELL_BROKER_ENDPOINT_JSON"]).is_relative_to(workspace.temporary_root))
            self.assertTrue(Path(environment["CARGO_TARGET_DIR"]).is_relative_to(workspace.root))
            self.assertTrue((source_root / ".gui_shell" / "broker" / "store").is_dir())
            self.assertFalse(workspace.temporary_root.exists())

    def test_session_validation_rejects_owner_role_without_disclosing_secret(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-launch-session-test-") as temporary:
            path = Path(temporary) / "session.json"
            path.write_text(
                json.dumps(
                    {
                        "credential_role": "owner",
                        "host": "127.0.0.1",
                        "transport": "authenticated_loopback_tcp",
                        "secret": "never-print-this",
                    }
                ),
                encoding="utf-8",
            )
            with self.assertRaises(RuntimeError) as raised:
                launcher._validate_session_file(path)
            self.assertNotIn("never-print-this", str(raised.exception))

    def test_wait_for_session_rejects_stale_file_after_broker_exit(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-launch-exited-broker-test-") as temporary:
            session_file = Path(temporary) / "session.json"
            session_file.write_text(
                json.dumps(
                    {
                        "credential_role": "normal",
                        "host": "127.0.0.1",
                        "transport": "authenticated_loopback_tcp",
                    }
                ),
                encoding="utf-8",
            )
            with self.assertRaisesRegex(RuntimeError, "session準備後に終了しました: 17"):
                launcher._wait_for_session(FakeProcess(17), session_file)

    def test_runtime_environment_preserves_user_app_data_but_removes_stale_endpoint(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-launch-env-test-") as temporary:
            workspace = FakeWorkspace(Path(temporary))
            with patch.dict(os.environ, {"APPDATA": "real-appdata", "GUI_SHELL_BROKER_ENDPOINT_JSON": "stale"}, clear=True):
                environment = launcher._runtime_environment(workspace)
            self.assertEqual(environment["APPDATA"], "real-appdata")
            self.assertNotIn("GUI_SHELL_BROKER_ENDPOINT_JSON", environment)
            self.assertEqual(environment["CARGO_TARGET_DIR"], workspace.environment["CARGO_TARGET_DIR"])
            shutil.rmtree(workspace.temporary_root)


if __name__ == "__main__":
    unittest.main()
