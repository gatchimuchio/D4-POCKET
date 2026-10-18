from __future__ import annotations

import contextlib
import io
import os
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

from tooling.flutter_workspace import FlutterWorkspaceError
from tooling.run_cargo_isolated import run_cargo


class IsolatedCargoRunnerTests(unittest.TestCase):
    def test_uses_temporary_target_propagates_exit_and_cleans(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-cargo-test-") as temporary:
            base = Path(temporary)
            repository = base / "repository"
            temp_parent = base / "temp"
            repository.mkdir()
            temp_parent.mkdir()
            observed: dict[str, object] = {}

            def fake_run(command, *, cwd, env, check):
                target = Path(env["CARGO_TARGET_DIR"])
                observed.update(command=command, cwd=cwd, env=env, check=check, target=target)
                (target / "generated-artifact").write_bytes(b"test")
                return SimpleNamespace(returncode=23)

            arguments = [
                "test",
                "--locked",
                "--manifest-path",
                "native/rust_helper/Cargo.toml",
                "--",
                "--test-threads=1",
            ]
            with (
                patch(
                    "tooling.run_cargo_isolated.safe_flutter_temp_parent",
                    return_value=temp_parent,
                ),
                patch.dict(
                    os.environ,
                    {
                        "CARGO_TARGET_DIR": "unsafe-existing-target",
                        "CARGO_BUILD_TARGET_DIR": "unsafe-config-target",
                    },
                ),
                contextlib.redirect_stderr(io.StringIO()),
            ):
                result = run_cargo(arguments, root=repository, runner=fake_run)

            self.assertEqual(result, 23)
            self.assertEqual(
                observed["command"],
                ["cargo", *arguments],
            )
            self.assertEqual(observed["cwd"], repository.resolve())
            self.assertFalse(observed["check"])
            self.assertEqual(observed["env"]["CARGO_TARGET_DIR"], str(observed["target"]))
            self.assertEqual(
                observed["env"]["CARGO_BUILD_TARGET_DIR"], str(observed["target"])
            )
            self.assertFalse(observed["target"].exists())

    def test_rejects_target_directory_override_without_running_cargo(self) -> None:
        commands = [
            ["test", "--target-dir", "outside"],
            ["test", "--target-dir=outside"],
            ["--config", "build.target-dir=outside", "test"],
            ["test", "--config=build.target-dir=outside"],
        ]
        with tempfile.TemporaryDirectory(prefix="d4p-cargo-test-") as temporary:
            repository = Path(temporary)
            for command in commands:
                with self.subTest(command=command), contextlib.redirect_stderr(io.StringIO()):
                    self.assertEqual(
                        run_cargo(
                            command,
                            root=repository,
                            runner=lambda *_args, **_kwargs: self.fail(
                                "Cargo was invoked despite a target override"
                            ),
                        ),
                        2,
                    )

    def test_rejects_unsafe_temporary_parent(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-cargo-test-") as temporary:
            repository = Path(temporary)
            runner_called = False

            def fake_run(*_args, **_kwargs):
                nonlocal runner_called
                runner_called = True
                return SimpleNamespace(returncode=0)

            with (
                patch(
                    "tooling.run_cargo_isolated.safe_flutter_temp_parent",
                    side_effect=FlutterWorkspaceError("OneDrive配下"),
                ),
                contextlib.redirect_stderr(io.StringIO()),
            ):
                result = run_cargo(["test"], root=repository, runner=fake_run)

            self.assertEqual(result, 2)
            self.assertFalse(runner_called)


if __name__ == "__main__":
    unittest.main()
