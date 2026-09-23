from __future__ import annotations

import argparse
import json
import subprocess
import sys
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from tooling.shell_snapshot import build_shell_snapshot


WATCHDOG_SECONDS = 5.0


def measure_snapshot_build() -> dict[str, int | str]:
    started = time.perf_counter()
    snapshot = build_shell_snapshot()
    elapsed = int((time.perf_counter() - started) * 1000)
    if not isinstance(snapshot, dict) or snapshot.get("snapshot_source") != "generated":
        raise RuntimeError("snapshot生成結果の証拠源を確認できない")
    if elapsed >= int(WATCHDOG_SECONDS * 1000):
        raise RuntimeError(f"snapshot生成がwatchdogを超過: {elapsed}ms")
    return {
        "name": "snapshot_generation",
        "elapsed_ms": elapsed,
        "evidence": "INTERNAL_STATE",
        "status": "passed",
    }


def run_flutter_projection_test() -> dict[str, int | str]:
    command = [
        "flutter",
        "test",
        "--no-pub",
        "test/c27_performance_validation_test.dart",
    ]
    started = time.perf_counter()
    executable = command[0]
    if sys.platform == "win32":
        resolved = next(
            (
                candidate
                for candidate in ("flutter.bat", "flutter.cmd", "flutter.exe")
                if _which(candidate)
            ),
            None,
        )
        if resolved is not None:
            executable = resolved
    completed = subprocess.run(
        [executable, *command[1:]],
        cwd=ROOT / "apps" / "desktop_flutter",
        text=True,
        encoding="utf-8",
        errors="replace",
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    elapsed = int((time.perf_counter() - started) * 1000)
    if completed.returncode != 0:
        print(completed.stdout, end="")
        raise RuntimeError(f"Flutter C27性能試験が失敗: exit={completed.returncode}")
    print(completed.stdout, end="")
    return {
        "name": "flutter_projection_test_process",
        "elapsed_ms": elapsed,
        "evidence": "FIXTURE",
        "status": "passed",
    }


def _which(name: str) -> str | None:
    import shutil

    return shutil.which(name)


def main() -> int:
    parser = argparse.ArgumentParser(description="C27の開発用性能・ハング監視")
    parser.add_argument("--skip-flutter", action="store_true")
    args = parser.parse_args()

    results = [measure_snapshot_build()]
    if not args.skip_flutter:
        results.append(run_flutter_projection_test())
    print("C27性能検証")
    print(json.dumps(results, ensure_ascii=False, indent=2))
    print(
        "C27境界: 開発用snapshot／Flutter projectionのbounded測定。"
        "実installed製品の起動、GPU frame、8時間運用はこの検査の証拠ではない。"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
