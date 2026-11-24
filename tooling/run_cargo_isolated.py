from __future__ import annotations

import os
import subprocess
import sys
import tempfile
from collections.abc import Callable, Sequence
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from tooling.flutter_workspace import (
    FlutterWorkspaceError,
    cargo_home_path,
    cargo_output_environment,
    safe_flutter_temp_parent,
)


def _cargo_target_override_error(arguments: Sequence[str]) -> str | None:
    cargo_arguments = list(arguments)
    if "--" in cargo_arguments:
        cargo_arguments = cargo_arguments[: cargo_arguments.index("--")]
    for argument in cargo_arguments:
        if argument == "--target-dir" or argument.startswith("--target-dir="):
            return "--target-dirは一時target隔離を迂回するため使用できません"
        if argument == "--config" or argument.startswith("--config="):
            return "--configは一時target隔離を迂回し得るため使用できません"
    return None


def run_cargo(
    arguments: Sequence[str],
    *,
    root: Path = ROOT,
    runner: Callable[..., subprocess.CompletedProcess[bytes]] | None = None,
) -> int:
    cargo_arguments = list(arguments)
    if not cargo_arguments:
        print(
            "使い方: python tooling/run_cargo_isolated.py <cargo commandと引数>",
            file=sys.stderr,
        )
        return 2

    override_error = _cargo_target_override_error(cargo_arguments)
    if override_error:
        print(override_error, file=sys.stderr)
        return 2

    repository = root.expanduser().resolve(strict=True)
    if not repository.is_dir():
        print("Repository rootがdirectoryではありません", file=sys.stderr)
        return 2

    try:
        temporary_parent = safe_flutter_temp_parent(repository)
    except FlutterWorkspaceError as error:
        print(f"安全なCargo一時領域を使用できません: {error}", file=sys.stderr)
        return 2

    invoke = runner or subprocess.run
    target_path: Path | None = None
    cargo_exit = 2
    try:
        with tempfile.TemporaryDirectory(
            prefix="d4p-cargo-target-", dir=temporary_parent
        ) as target_directory:
            target_path = Path(target_directory).resolve(strict=True)
            temporary_io = target_path / "io-temp"
            temporary_io.mkdir()
            cargo_home, cargo_home_mode = cargo_home_path(repository, target_path)
            environment = os.environ.copy()
            environment["CARGO_HOME"] = str(cargo_home)
            environment.update(cargo_output_environment(target_path))
            for variable in ("TEMP", "TMP", "TMPDIR"):
                environment[variable] = str(temporary_io)
            print(
                "Cargoの成果物・中間生成物・一時ファイルをRepository／OneDrive外へ隔離します。"
                f"CARGO_HOME={cargo_home_mode}。",
                file=sys.stderr,
            )
            result = invoke(
                ["cargo", *cargo_arguments],
                cwd=repository,
                env=environment,
                check=False,
            )
            cargo_exit = result.returncode
    except (OSError, subprocess.SubprocessError) as error:
        print(f"Cargo実行または一時target回収に失敗しました: {error}", file=sys.stderr)
        return 2

    if target_path is not None and target_path.exists():
        print("Cargo一時targetが残存しています", file=sys.stderr)
        return cargo_exit if cargo_exit != 0 else 2
    return cargo_exit


def main() -> int:
    return run_cargo(sys.argv[1:])


if __name__ == "__main__":
    raise SystemExit(main())
