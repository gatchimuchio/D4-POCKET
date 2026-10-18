from __future__ import annotations

import argparse
import hashlib
import platform
import shutil
import stat
import subprocess
import sys
import traceback
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")

from tooling.windows_release_evidence import validate_windows_release_evidence
from tooling.flutter_workspace import FlutterWorkspaceError, IsolatedFlutterWorkspace


@dataclass(frozen=True)
class ValidationStep:
    name: str
    command: list[str]
    cwd: Path
    required_tool: str | None = None
    in_release_scope: bool = True
    post_v1_reason: str | None = None


@dataclass(frozen=True)
class EvidenceCheck:
    name: str
    status: str
    classification: str
    blocks_release: str
    reason: str
    required_action: str


FLUTTER_STEP_CHOICES = (
    "shared_flutter_analyze",
    "shared_flutter_test",
    "desktop_flutter_analyze",
    "desktop_flutter_test",
    "desktop_flutter_build_linux",
    "mobile_flutter_analyze",
    "mobile_flutter_test",
    "mobile_android_debug_build",
)


def python_step(script: str, *args: str) -> list[str]:
    return [sys.executable, script, *args]


def find_tool(tool: str) -> str | None:
    resolved = shutil.which(tool)
    if resolved is not None:
        return resolved
    if platform.system() == "Windows" and Path(tool).suffix == "":
        for suffix in (".exe", ".bat", ".cmd"):
            resolved = shutil.which(f"{tool}{suffix}")
            if resolved is not None:
                return resolved
    return None


def resolve_step_command(command: list[str]) -> list[str]:
    resolved = list(command)
    if platform.system() != "Windows" or not resolved:
        return resolved
    executable = resolved[0]
    if Path(executable).suffix != "" or "\\" in executable or "/" in executable:
        return resolved
    for suffix in (".exe", ".bat", ".cmd"):
        candidate = shutil.which(f"{executable}{suffix}")
        if candidate is not None:
            resolved[0] = candidate
            break
    return resolved


def is_flutter_command(command: list[str]) -> bool:
    return bool(command) and Path(command[0]).name.casefold() == "flutter"


def requires_isolated_flutter_workspace(step: ValidationStep) -> bool:
    return is_flutter_command(step.command) or step.name == "desktop_flutter_test_rust_helper_build"


def current_desktop_platform() -> str:
    system = platform.system().lower()
    if system == "linux":
        return "linux"
    if system == "windows":
        return "windows"
    if system == "darwin":
        return "macos"
    return "unknown"


def build_steps(
    include_mobile_release: bool,
    desktop_platform: str,
    python_only: bool = False,
    android_debug_build: bool = False,
) -> list[ValidationStep]:
    steps = [
        ValidationStep(
            "japanese_base_audit",
            python_step("tooling/日本語基底監査.py", "--strict"),
            ROOT,
        ),
        ValidationStep("schema_check", python_step("tooling/schema_check/check_schemas.py"), ROOT),
        ValidationStep(
            "conformance_skeleton",
            python_step("tooling/conformance_tests/run_conformance_skeleton.py"),
            ROOT,
        ),
        ValidationStep("manifest_check", python_step("tooling/manifest.py", "--check"), ROOT),
        ValidationStep("release_gate_check", python_step("tooling/release_gate_check.py"), ROOT),
        ValidationStep(
            "packaging_portability_check",
            python_step("tooling/packaging_portability_check.py"),
            ROOT,
        ),
        ValidationStep("release_smoke", python_step("tooling/release_smoke.py"), ROOT),
        ValidationStep("evidence_bundle", python_step("tooling/evidence_bundle.py", "--check"), ROOT),
        ValidationStep(
            "release_runtime_assertions",
            python_step("tooling/release_runtime_assertions.py", "--check"),
            ROOT,
        ),
        ValidationStep(
            "c32_final_development_audit",
            python_step("tooling/final_development_audit.py"),
            ROOT,
            in_release_scope=False,
            post_v1_reason="C32の対応表構造監査。全機能完成、installed製品、正式releaseの証拠ではない",
        ),
    ]
    if python_only:
        return steps
    steps.append(
        ValidationStep(
            "c27_performance_validation",
            python_step("tooling/performance_validation.py"),
            ROOT,
            "flutter",
            in_release_scope=False,
            post_v1_reason="C27の開発用性能・ハング監視。実installed製品の性能証拠は別のWindows実機検証で扱う",
        )
    )
    steps.append(
        ValidationStep(
            "c28_long_run_smoke",
            python_step(
                "tooling/long_run_validation.py",
                "--duration-seconds",
                "30",
                "--interval-seconds",
                "1",
            ),
            ROOT,
            "cargo",
            in_release_scope=False,
            post_v1_reason="C28の短時間開発smoke。8時間の実測とinstalled製品証拠は別に収集する",
        )
    )
    steps.append(
        ValidationStep(
            "c29_failure_injection_smoke",
            python_step("tooling/failure_injection_validation.py"),
            ROOT,
            "cargo",
            in_release_scope=False,
            post_v1_reason="C29の開発用障害注入。installed製品・外部サービスの障害証拠は別に収集する",
        )
    )
    steps.append(
        ValidationStep(
            "c30_full_regression_validation",
            python_step("tooling/full_regression_validation.py"),
            ROOT,
            "cargo",
            in_release_scope=False,
            post_v1_reason="C30のlocal回帰マトリクス。installed製品・外部Agent・実端末の証拠は別に収集する",
        )
    )
    steps.extend([
        ValidationStep(
            "broker_authority_parity",
            python_step("tooling/broker_parity/run_authority_parity.py"),
            ROOT,
            "cargo",
        ),
        ValidationStep(
            "rust_helper_cargo_test",
            ["cargo", "test", "--locked", "--", "--test-threads=1"],
            ROOT / "native" / "rust_helper",
            "cargo",
        ),
        ValidationStep("shared_flutter_analyze", ["flutter", "analyze"], ROOT / "packages" / "gui_shell_ui", "flutter"),
        ValidationStep("shared_flutter_test", ["flutter", "test"], ROOT / "packages" / "gui_shell_ui", "flutter"),
        ValidationStep("desktop_flutter_analyze", ["flutter", "analyze"], ROOT / "apps" / "desktop_flutter", "flutter"),
        ValidationStep(
            "desktop_flutter_test_rust_helper_build",
            [
                "cargo",
                "build",
                "--locked",
                "--offline",
                "--manifest-path",
                "native/rust_helper/Cargo.toml",
                "--bin",
                "gui_shell_rust_helper",
            ],
            ROOT,
            "cargo",
        ),
        ValidationStep("desktop_flutter_test", ["flutter", "test"], ROOT / "apps" / "desktop_flutter", "flutter"),
    ])
    current = current_desktop_platform()
    target_linux = desktop_platform == "linux" or desktop_platform == "all" or (
        desktop_platform == "current" and current == "linux"
    )
    if target_linux and current == "linux":
        steps.append(
            ValidationStep(
                "desktop_flutter_build_linux",
                ["flutter", "build", "linux"],
                ROOT / "apps" / "desktop_flutter",
                "flutter",
            )
        )
    steps.append(
        ValidationStep(
            "mobile_flutter_analyze",
            ["flutter", "analyze"],
            ROOT / "apps" / "mobile_flutter",
            "flutter",
            in_release_scope=include_mobile_release,
            post_v1_reason=None
            if include_mobile_release
            else "owner が mobile を明示的に含めない限り、mobile 完全 release は v1.0 desktop scope 外である",
        )
    )
    if not python_only:
        steps.append(ValidationStep("mobile_flutter_test", ["flutter", "test"],
            ROOT / "apps" / "mobile_flutter", "flutter"))
    if android_debug_build:
        steps.append(
            ValidationStep(
                "mobile_android_debug_build",
                ["flutter", "build", "apk", "--debug"],
                ROOT / "apps" / "mobile_flutter",
                "flutter",
                in_release_scope=False,
                post_v1_reason="開発用Android debug APK build。実機動作、正式配布、release readinessの証拠ではない",
            )
        )
    return steps


ANDROID_DEBUG_APK_RELATIVE_PATH = Path(
    "apps/mobile_flutter/build/app/outputs/flutter-apk/app-debug.apk"
)


def capture_mobile_android_debug_artifact(result: dict, execution_root: Path) -> dict:
    if result.get("status") != "passed":
        return result

    artifact = execution_root / ANDROID_DEBUG_APK_RELATIVE_PATH
    try:
        resolved_root = execution_root.resolve(strict=True)
        resolved_artifact = artifact.resolve(strict=True)
        resolved_artifact.relative_to(resolved_root)
        metadata = artifact.lstat()
        reparse_point = getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0x400)
        if not stat.S_ISREG(metadata.st_mode) or getattr(metadata, "st_file_attributes", 0) & reparse_point:
            raise OSError("APK成果物が通常のファイルではない、または隔離作業領域の所有下にない")

        digest = hashlib.sha256()
        with artifact.open("rb") as stream:
            for block in iter(lambda: stream.read(1024 * 1024), b""):
                digest.update(block)
    except (OSError, ValueError) as exc:
        result.update(
            {
                "status": "failed",
                "reason": f"開発用APK成果物の検証に失敗した: {exc.__class__.__name__}",
                "required_action": "隔離作業領域内にAPK成果物が生成され、許可された格納先にあることを確認する。",
                "artifact_evidence": {
                    "status": "failed",
                    "path": ANDROID_DEBUG_APK_RELATIVE_PATH.as_posix(),
                },
            }
        )
        return result

    result["artifact_evidence"] = {
        "status": "captured",
        "path": ANDROID_DEBUG_APK_RELATIVE_PATH.as_posix(),
        "size_bytes": metadata.st_size,
        "sha256": digest.hexdigest(),
    }
    return result


def validation_exit_code(
    results: list[dict],
    blockers: list[dict],
    required_step_names: set[str] | None = None,
) -> int:
    required = required_step_names or set()
    if blockers:
        return 1
    for result in results:
        status = result.get("status")
        if status == "failed" or (result.get("name") in required and status != "passed"):
            return 1
    return 0


def select_flutter_validation_steps(
    steps: list[ValidationStep],
    android_debug_build_only: bool = False,
    selected_step_name: str | None = None,
) -> list[ValidationStep]:
    if android_debug_build_only:
        return [step for step in steps if step.name == "mobile_android_debug_build"]
    if selected_step_name is not None:
        selected_names = {selected_step_name}
        if selected_step_name == "desktop_flutter_test":
            selected_names.add("desktop_flutter_test_rust_helper_build")
        selected = [
            step
            for step in steps
            if step.name in selected_names and requires_isolated_flutter_workspace(step)
        ]
        if not any(step.name == selected_step_name for step in selected):
            return []
        return selected
    return [step for step in steps if requires_isolated_flutter_workspace(step)]


def mark_workspace_cleanup_failure(result: dict, error: Exception) -> None:
    result["workspace_cleanup"] = "failed"
    if result.get("status") == "passed":
        result["status"] = "failed"
        if result.get("classification") == "post_v1_scope":
            result["blocks_release"] = "no"
        else:
            result["classification"] = "release_blocker"
            result["blocks_release"] = "yes"
        result["reason"] = "隔離Flutter作業領域の後片付けまたは元source同一性確認に失敗した"
        result["required_action"] = "一時作業領域を保全し、後片付けまたは元source同一性の原因を解消する。"
    result["stderr"] = (result.get("stderr", "") + f"\n{error.__class__.__name__}: {error}").strip()


def platform_evidence_checks(desktop_platform: str, strict_release: bool = False) -> list[EvidenceCheck]:
    current = current_desktop_platform()
    checks: list[EvidenceCheck] = []

    include_linux = desktop_platform in ("linux", "all") or (desktop_platform == "current" and current == "linux")
    include_windows = desktop_platform in ("windows", "all") or (desktop_platform == "current" and current == "windows")
    include_macos = desktop_platform in ("macos", "all") or (desktop_platform == "current" and current == "macos")

    if include_linux:
        checks.extend(
            [
                EvidenceCheck(
                    "linux_desktop_build_smoke",
                    "passed",
                    "none",
                    "no",
                    "Linux desktop build smoke は 2026-05-25 に合格した。",
                    "開発・検証範囲の証拠として Linux build smoke の合格を維持する。",
                ),
                EvidenceCheck(
                    "linux_desktop_launch_smoke",
                    "passed",
                    "none",
                    "no",
                    "Linux desktop launch smoke は WSLg 上で合格し、最初の window 証拠を記録済みである。これは有用な証拠だが、単独で最終的な Windows-first 製品証拠にはならない。",
                    "Windows-first release 証拠を完成させる間も Linux launch smoke の合格を維持する。",
                ),
            ]
        )
    if include_windows:
        windows_project_exists = (ROOT / "apps" / "desktop_flutter" / "windows").is_dir()
        checks.extend(
            [
                EvidenceCheck(
                    "windows_desktop_project_support_exists",
                    "passed" if windows_project_exists else "failed",
                    "none" if windows_project_exists else "release_blocker",
                    "no" if windows_project_exists else "yes",
                    "`apps/desktop_flutter/windows` が存在する。" if windows_project_exists else "`apps/desktop_flutter/windows` が存在しないため、Windows desktop project support は未生成である。",
                    "Windows Flutter desktop project files を version control 下に維持する。" if windows_project_exists else "Windows desktop project support を生成し、限定した Flutter desktop project files を commit する。",
                ),
                EvidenceCheck(
                    "windows_development_toolchain_historical_smoke",
                    "historical_invalid_for_current_r2",
                    "none",
                    "no",
                    "過去の native Windows analyze/test/build/launch smoke は owner 試用履歴としてのみ保存している。正確な実装 commit と installed-path evidence bundle に紐付いていないため、現行 R2 の正式証拠ではない。",
                    "隔離した installed-path evidence flow で新しい native Windows release-candidate 証拠を収集する。",
                ),
            ]
        )
        checks.extend(
            EvidenceCheck(
                result.name,
                result.status,
                result.classification,
                result.blocks_release,
                result.reason,
                result.required_action,
            )
            for result in validate_windows_release_evidence()
        )
    if include_macos:
        checks.extend(
            [
                EvidenceCheck(
                    "macos_desktop_project_support_exists",
                    "unverified_planned",
                    "known_limitation",
                    "no",
                    "現在利用できる macOS 検証環境はなく、GUI-Shell v1.0 は検証済み macOS support を主張しない。",
                    "macOS support を主張する前に macOS host で検証する。",
                ),
                EvidenceCheck(
                    "macos_flutter_toolchain",
                    "unverified_planned",
                    "known_limitation",
                    "no",
                    "現在利用できる macOS 検証環境はない。",
                    "support を主張する前に macOS 上で macOS Flutter toolchain を検証する。",
                ),
                EvidenceCheck(
                    "macos_desktop_build_smoke",
                    "unverified_planned",
                    "known_limitation",
                    "no",
                    "macOS 検証環境を利用できないため、macOS build smoke は未実行である。",
                    "support を主張する前に macOS host で `flutter build macos` を合格させる。",
                ),
                EvidenceCheck(
                    "macos_desktop_launch_smoke",
                    "unverified_planned",
                    "known_limitation",
                    "no",
                    "macOS 検証環境を利用できないため、macOS launch smoke 証拠は未記録である。",
                    "support を主張する前に macOS artifact を起動して証拠を記録する。",
                ),
                EvidenceCheck(
                    "macos_packaging_notarization_plan",
                    "unverified_planned",
                    "known_limitation",
                    "no",
                    "macOS packaging/notarization は計画段階の portability 検証のままである。",
                    "support を主張する前に macOS 上で文書化し、検証する。",
                ),
                EvidenceCheck(
                    "macos_installer_first_run_smoke",
                    "unverified_planned",
                    "known_limitation",
                    "no",
                    "macOS installer/first-run smoke は Windows-first v1.0 release gate の対象外である。",
                    "support を主張する前に macOS host で検証する。",
                ),
            ]
        )
    if strict_release:
        checks.append(
            EvidenceCheck(
                "owner_go",
                "failed",
                "release_blocker",
                "yes",
                "明示的な owner GO は記録されていない。",
                "計測済み installed-path release 証拠と strict validation 合格の後に限り、明示的な owner GO を記録する。",
            )
        )
    return checks


def classify_not_run(step: ValidationStep, strict_release: bool) -> tuple[str, str, str, str]:
    if step.in_release_scope and strict_release:
        return ("release_blocker", "yes", f"{step.required_tool} が PATH 上に存在しない", f"{step.required_tool} を導入して release validation を再実行する。")
    if not step.in_release_scope:
        return ("post_v1_scope", "no", step.post_v1_reason or "v1.0 scope 外", "v1.0 で必要な対応はない。")
    return ("release_blocker", "yes", f"{step.required_tool} が PATH 上に存在しない", f"release validation の前に {step.required_tool} を導入する。")


def run_step(
    step: ValidationStep,
    strict_release: bool,
    desktop_platform: str,
    *,
    execution_root: Path | None = None,
    execution_environment: dict[str, str] | None = None,
    source_commit: str | None = None,
    source_sha256: str | None = None,
    pub_cache_mode: str | None = None,
    cargo_home_mode: str | None = None,
) -> dict:
    step_command = resolve_step_command(step.command)
    if strict_release and step.name == "release_gate_check":
        step_command.append("--strict-release")
        if desktop_platform == "windows" or (
            desktop_platform == "current" and current_desktop_platform() == "windows"
        ):
            step_command.extend(["--release-track", "windows_v1"])
    command = " ".join(step_command)
    execution_cwd = step.cwd
    if execution_root is not None:
        try:
            relative_cwd = step.cwd.relative_to(ROOT)
        except ValueError:
            relative_cwd = None
        if relative_cwd is None:
            return {
                "name": step.name,
                "command": command,
                "status": "not_run",
                "classification": "release_blocker",
                "blocks_release": "yes",
                "reason": "Flutter validation working directory がsource root外にある",
                "required_action": "source root内の検証対象だけをtemporary snapshotで実行する。",
                "stdout": "",
                "stderr": "",
                "exit": "",
            }
        execution_cwd = execution_root / relative_cwd
        if not execution_cwd.is_dir():
            return {
                "name": step.name,
                "command": command,
                "status": "not_run",
                "classification": "release_blocker",
                "blocks_release": "yes",
                "reason": "isolated source snapshot内にFlutter project directoryがない",
                "required_action": "snapshot作成対象とFlutter step working directoryを照合する。",
                "stdout": "",
                "stderr": "",
                "exit": "",
            }

    def include_workspace_metadata(result: dict) -> dict:
        if execution_root is not None:
            result.update(
                {
                    "execution_workspace": "isolated_temporary_source",
                    "source_commit": source_commit or "unknown",
                    "source_sha256": source_sha256 or "unknown",
                    "pub_cache_mode": pub_cache_mode or "unknown",
                    "cargo_home_mode": cargo_home_mode or "unknown",
                    "workspace_cleanup": "pending",
                }
            )
        return result

    if step.required_tool and find_tool(step.required_tool) is None:
        classification, blocks_release, reason, required_action = classify_not_run(step, strict_release)
        return include_workspace_metadata({
            "name": step.name,
            "command": command,
            "status": "not_run",
            "classification": classification,
            "blocks_release": blocks_release,
            "reason": reason,
            "required_action": required_action,
            "stdout": "",
            "stderr": "",
            "exit": "",
        })

    try:
        completed = subprocess.run(
            step_command,
            cwd=execution_cwd,
            check=False,
            text=True,
            encoding="utf-8",
            errors="replace",
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=execution_environment,
        )
    except OSError as exc:
        classification, blocks_release, _, _ = classify_not_run(step, strict_release)
        return include_workspace_metadata({
            "name": step.name,
            "command": command,
            "command_array": step_command,
            "status": "not_run",
            "classification": classification,
            "blocks_release": blocks_release,
            "reason": f"validation command を起動できない: {exc.__class__.__name__}: {exc}",
            "required_action": "validation host toolchain または command 解決を修正して再実行する。validator failure は構造化された release blocker のまま保持する。",
            "stdout": "",
            "stderr": traceback.format_exc().rstrip(),
            "exit": "",
        })
    passed = completed.returncode == 0
    if not step.in_release_scope:
        classification = "post_v1_scope"
        blocks_release = "no"
        reason = step.post_v1_reason or "v1.0 scope 外"
        required_action = "owner がこの scope を含めない限り、v1.0 で必要な対応はない。"
    else:
        classification = "none"
        blocks_release = "no"
        reason = ""
        required_action = ""
    if not passed and step.in_release_scope:
        classification = "release_blocker"
        blocks_release = "yes"
        if strict_release and step.name == "release_gate_check":
            reason = "strict release gate が未解決の有効な structured release blocker を検出した"
            required_action = "有効または証拠上有効な Windows-first release_blocker をすべて解消し、strict validation を再実行する。owner が scope を変更しない限り、macOS は未検証の既知制限のままである。"
        else:
            reason = "validation command が失敗した"
            required_action = "失敗した validation command を修正して再実行する。"
    return include_workspace_metadata({
        "name": step.name,
        "command": command,
        "status": "passed" if passed else "failed",
        "classification": classification,
        "blocks_release": blocks_release,
        "reason": reason,
        "required_action": required_action,
        "stdout": completed.stdout.rstrip(),
        "stderr": completed.stderr.rstrip(),
        "exit": str(completed.returncode),
    })


def print_report(mode: str, desktop_platform: str, results: list[dict], evidence: list[EvidenceCheck], blockers: list[dict]) -> None:
    print(f"validation_mode: {mode}")
    print(f"desktop_platform: {desktop_platform}")
    print("")
    print("checks:")
    for result in results:
        print(f"  - name: {result['name']}")
        print(f"    command: {result['command']}")
        print(f"    status: {result['status']}")
        print(f"    classification: {result['classification']}")
        print(f"    blocks_release: {result['blocks_release']}")
        print(f"    reason: {result['reason']}")
        if result["exit"]:
            print(f"    exit: {result['exit']}")
        for key in (
            "execution_workspace",
            "source_commit",
            "source_sha256",
            "pub_cache_mode",
            "cargo_home_mode",
            "workspace_cleanup",
            "workspace_cleanup_retries",
        ):
            if result.get(key):
                print(f"    {key}: {result[key]}")
        for key, value in result.get("artifact_evidence", {}).items():
            print(f"    artifact_{key}: {value}")
        if result["stdout"]:
            print("    stdout: |")
            for line in result["stdout"].splitlines():
                print(f"      {line}")
        if result["stderr"]:
            print("    stderr: |")
            for line in result["stderr"].splitlines():
                print(f"      {line}")
    if evidence:
        print("")
        print("desktop_platform_evidence:")
        for check in evidence:
            print(f"  - name: {check.name}")
            print(f"    status: {check.status}")
            print(f"    classification: {check.classification}")
            print(f"    blocks_release: {check.blocks_release}")
            print(f"    reason: {check.reason}")
            print(f"    required_action: {check.required_action}")
    print("")
    if mode == "flutter_step":
        print("release_gate: 未評価")
        return
    print("release_gate:")
    print(f"  status: {'fail' if blockers else 'pass'}")
    print("  blockers:")
    for blocker in blockers:
        print(f"    - item: {blocker['name']}")
        print(f"      classification: {blocker['classification']}")
        print(f"      blocks_release: {blocker['blocks_release']}")
        print(f"      reason: {blocker['reason']}")
        print(f"      required_action: {blocker['required_action']}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--strict-release", action="store_true")
    parser.add_argument(
        "--include-mobile-release",
        action="store_true",
        help="mobile Flutter validation を release gate の scope 内として扱う。",
    )
    parser.add_argument(
        "--desktop-platform",
        choices=["current", "windows", "linux", "macos", "all"],
        default="current",
        help="現在の host、指定した desktop target、または Windows/macOS/Linux desktop 全 scope を検証する。",
    )
    validation_scope = parser.add_mutually_exclusive_group()
    validation_scope.add_argument(
        "--python-only",
        action="store_true",
        help="Rust と Flutter を別途検証する場合に、Python/core validation step だけを実行する。",
    )
    validation_scope.add_argument(
        "--flutter-only",
        action="store_true",
        help="Flutter analyze/testと選択したplatform build、必要なDesktop Rust helper buildを一時workspaceで実行する。",
    )
    validation_scope.add_argument(
        "--flutter-step",
        choices=FLUTTER_STEP_CHOICES,
        help="指定したFlutter検証stepだけを一時source snapshot上で実行する。Desktop testはRust helper buildも含む。",
    )
    parser.add_argument(
        "--mobile-android-debug-build",
        action="store_true",
        help="flutter-onlyでAndroid debug APK buildだけを実行し、artifact hashを記録する。",
    )
    args = parser.parse_args()

    if args.flutter_step and args.strict_release:
        parser.error("--flutter-stepは開発用の対象限定検査であり、--strict-releaseと併用できない")
    if args.flutter_step and args.mobile_android_debug_build:
        parser.error("--flutter-stepと--mobile-android-debug-buildは併用できない")
    if args.mobile_android_debug_build and not args.flutter_only:
        parser.error("--mobile-android-debug-buildには--flutter-onlyが必要")
    if args.mobile_android_debug_build and args.strict_release:
        parser.error("開発用Android debug buildをstrict release検査へ混在できない")

    mode = "strict_release" if args.strict_release else (
        "flutter_step" if args.flutter_step else ("flutter_only" if args.flutter_only else "development")
    )
    steps = build_steps(
        args.include_mobile_release,
        args.desktop_platform,
        args.python_only,
        android_debug_build=args.mobile_android_debug_build or args.flutter_step == "mobile_android_debug_build",
    )
    if args.flutter_only or args.flutter_step:
        steps = select_flutter_validation_steps(
            steps,
            android_debug_build_only=args.mobile_android_debug_build,
            selected_step_name=args.flutter_step,
        )
        if not steps:
            parser.error("指定したFlutter stepが現在のplatformで利用できない")

    has_flutter = any(requires_isolated_flutter_workspace(step) for step in steps)
    workspace: IsolatedFlutterWorkspace | None = None
    workspace_error: Exception | None = None
    if has_flutter:
        candidate = IsolatedFlutterWorkspace(ROOT)
        try:
            workspace = candidate.__enter__()
        except Exception as exc:
            workspace_error = exc

    results: list[dict] = []
    isolated_results: list[dict] = []
    desktop_helper_build: dict | None = None
    for step in steps:
        if requires_isolated_flutter_workspace(step):
            if workspace is None:
                classification = "release_blocker" if step.in_release_scope else "post_v1_scope"
                blocks_release = "yes" if step.in_release_scope else "no"
                result = {
                    "name": step.name,
                    "command": " ".join(resolve_step_command(step.command)),
                    "status": "not_run",
                    "classification": classification,
                    "blocks_release": blocks_release,
                    "reason": "isolated Flutter source workspaceを準備できず、OneDrive上での直接実行を拒否した",
                    "required_action": "source/archive、temporary path、permissionの原因を修正して再実行する。",
                    "stdout": "",
                    "stderr": f"{workspace_error.__class__.__name__}: {workspace_error}" if workspace_error else "workspace unavailable",
                    "exit": "",
                    "execution_workspace": "not_available",
                    "workspace_cleanup": "not_created",
                }
            elif step.name == "desktop_flutter_test" and desktop_helper_build is not None and desktop_helper_build.get("status") != "passed":
                result = {
                    "name": step.name,
                    "command": " ".join(resolve_step_command(step.command)),
                    "status": "not_run",
                    "classification": "release_blocker" if step.in_release_scope else "post_v1_scope",
                    "blocks_release": "yes" if step.in_release_scope else "no",
                    "reason": "Desktop Flutter testのRust helper事前buildが成功していない",
                    "required_action": "先行するdesktop_flutter_test_rust_helper_buildの失敗を解消してから再実行する。",
                    "stdout": "",
                    "stderr": "",
                    "exit": "",
                    "execution_workspace": "isolated_temporary_source",
                    "source_commit": workspace.source_commit or "unknown",
                    "source_sha256": workspace.source_sha256 or "unknown",
                    "pub_cache_mode": workspace.pub_cache_mode or "unknown",
                    "cargo_home_mode": workspace.cargo_home_mode or "unknown",
                    "workspace_cleanup": "pending",
                }
                isolated_results.append(result)
            else:
                assert workspace.root is not None
                result = run_step(
                    step,
                    args.strict_release,
                    args.desktop_platform,
                    execution_root=workspace.root,
                    execution_environment=workspace.environment,
                    source_commit=workspace.source_commit,
                    source_sha256=workspace.source_sha256,
                    pub_cache_mode=workspace.pub_cache_mode,
                    cargo_home_mode=workspace.cargo_home_mode,
                )
                if step.name == "mobile_android_debug_build" and workspace.root is not None:
                    result = capture_mobile_android_debug_artifact(result, workspace.root)
                isolated_results.append(result)
                if step.name == "desktop_flutter_test_rust_helper_build":
                    desktop_helper_build = result
            results.append(result)
        else:
            results.append(run_step(step, args.strict_release, args.desktop_platform))

    if workspace is not None:
        try:
            workspace.__exit__(None, None, None)
            for result in isolated_results:
                result["workspace_cleanup"] = "passed"
                result["workspace_cleanup_retries"] = workspace.cleanup_retry_count
        except Exception as exc:
            for result in isolated_results:
                result["workspace_cleanup_retries"] = workspace.cleanup_retry_count
                mark_workspace_cleanup_failure(result, exc)

    evidence = (
        []
        if args.flutter_step
        else platform_evidence_checks(args.desktop_platform, args.strict_release)
    )
    blockers = [
        result
        for result in results
        if result["classification"] == "release_blocker"
    ]
    if args.strict_release:
        blockers.extend(
            {
                "name": check.name,
                "classification": check.classification,
                "blocks_release": check.blocks_release,
                "reason": check.reason,
                "required_action": check.required_action,
            }
            for check in evidence
            if check.classification == "release_blocker"
        )
    print_report(mode, args.desktop_platform, results, evidence, blockers)
    required_step_names = (
        {step.name for step in steps}
        if args.flutter_step
        else ({"mobile_android_debug_build"} if args.mobile_android_debug_build else set())
    )
    return validation_exit_code(results, blockers, required_step_names)


if __name__ == "__main__":
    raise SystemExit(main())
