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
                intermediate = Path(env["CARGO_BUILD_BUILD_DIR"])
                temporary_io = Path(env["TMPDIR"])
                observed.update(
                    command=command,
                    cwd=cwd,
                    env=env,
                    check=check,
                    target=target,
                    temporary_io=temporary_io,
                )
                self.assertEqual(intermediate, target)
                self.assertEqual(temporary_io, target / "io-temp")
                (intermediate / "generated-artifact").write_bytes(b"test")
                (temporary_io / "compiler-temp").write_bytes(b"test")
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
                        "CARGO_BUILD_BUILD_DIR": str(repository / "intermediate"),
                        "CARGO_HOME": str(repository / ".cargo"),
                        **dict.fromkeys(("TEMP", "TMP", "TMPDIR"), str(repository)),
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
            self.assertEqual(
                observed["env"]["CARGO_HOME"], str(observed["target"] / "cargo-home")
            )
            self.assertEqual(observed["env"]["CARGO_TARGET_DIR"], str(observed["target"]))
            self.assertEqual(
                observed["env"]["CARGO_BUILD_TARGET_DIR"], str(observed["target"])
            )
            for name in ("TEMP", "TMP", "TMPDIR"):
                self.assertEqual(observed["env"][name], str(observed["temporary_io"]))
            self.assertFalse(observed["target"].exists())
            self.assertFalse((repository / ".cargo").exists())
            self.assertFalse((repository / "intermediate").exists())

    def test_isolates_cargo_home_inside_onedrive(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-cargo-test-") as temporary:
            base = Path(temporary)
            repository = base / "repository"
            temp_parent = base / "temp"
            onedrive = base / "OneDrive"
            repository.mkdir()
            temp_parent.mkdir()
            onedrive.mkdir()
            configured_cargo_home = onedrive / "Cargo"
            observed: dict[str, object] = {}

            def fake_run(_command, *, cwd, env, check):
                observed.update(cwd=cwd, env=env, check=check)
                cache = Path(env["CARGO_HOME"])
                self.assertTrue(cache.is_relative_to(Path(env["CARGO_TARGET_DIR"])))
                (cache / "registry-marker").write_text("isolated", encoding="utf-8")
                return SimpleNamespace(returncode=0)

            with (
                patch(
                    "tooling.run_cargo_isolated.safe_flutter_temp_parent",
                    return_value=temp_parent,
                ),
                patch.dict(
                    os.environ,
                    {"CARGO_HOME": str(configured_cargo_home), "OneDrive": str(onedrive)},
                ),
                contextlib.redirect_stderr(io.StringIO()),
            ):
                self.assertEqual(run_cargo(["check"], root=repository, runner=fake_run), 0)

            self.assertEqual(observed["cwd"], repository.resolve())
            self.assertFalse(observed["check"])
            self.assertFalse(configured_cargo_home.exists())
            self.assertFalse(Path(observed["env"]["CARGO_TARGET_DIR"]).exists())

    def test_preserves_shared_cargo_home_outside_source_and_onedrive(self) -> None:
        with tempfile.TemporaryDirectory(prefix="d4p-cargo-test-") as temporary:
            base = Path(temporary)
            repository = base / "repository"
            temp_parent = base / "temp"
            shared_cargo_home = base / "shared-cargo-home"
            repository.mkdir()
            temp_parent.mkdir()
            shared_cargo_home.mkdir()
            marker = shared_cargo_home / "existing-cache-marker"
            marker.write_text("keep", encoding="utf-8")
            observed: dict[str, object] = {}

            def fake_run(_command, *, cwd, env, check):
                observed.update(cwd=cwd, env=env, check=check)
                (Path(env["CARGO_HOME"]) / "new-cache-marker").write_text(
                    "shared", encoding="utf-8"
                )
                return SimpleNamespace(returncode=0)

            with (
                patch(
                    "tooling.run_cargo_isolated.safe_flutter_temp_parent",
                    return_value=temp_parent,
                ),
                patch.dict(os.environ, {"CARGO_HOME": str(shared_cargo_home), "OneDrive": ""}),
                contextlib.redirect_stderr(io.StringIO()),
            ):
                self.assertEqual(run_cargo(["check"], root=repository, runner=fake_run), 0)

            self.assertEqual(Path(observed["env"]["CARGO_HOME"]), shared_cargo_home)
            self.assertEqual(marker.read_text(encoding="utf-8"), "keep")
            self.assertEqual(
                (shared_cargo_home / "new-cache-marker").read_text(encoding="utf-8"),
                "shared",
            )
            self.assertFalse(Path(observed["env"]["CARGO_TARGET_DIR"]).exists())

    def test_rejects_target_directory_override_without_running_cargo(self) -> None:
        commands = [
            ["test", "--target-dir", "outside"],
            ["test", "--target-dir=outside"],
            ["--config", "build.target-dir=outside", "test"],
            ["test", "--config=build.target-dir=outside"],
            ["test", "--config=build.build-dir=outside"],
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
