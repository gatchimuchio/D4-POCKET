from __future__ import annotations

import argparse
import platform
import shutil
import subprocess
import sys
import traceback
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from tooling.windows_release_evidence import validate_windows_release_evidence


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


def current_desktop_platform() -> str:
    system = platform.system().lower()
    if system == "linux":
        return "linux"
    if system == "windows":
        return "windows"
    if system == "darwin":
        return "macos"
    return "unknown"


def build_steps(include_mobile_release: bool, desktop_platform: str, python_only: bool = False) -> list[ValidationStep]:
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
    ]
    if python_only:
        return steps
    steps.extend([
        ValidationStep(
            "broker_authority_parity",
            python_step("tooling/broker_parity/run_authority_parity.py"),
            ROOT,
            "cargo",
        ),
        ValidationStep("rust_helper_cargo_test", ["cargo", "test"], ROOT / "native" / "rust_helper", "cargo"),
        ValidationStep("shared_flutter_analyze", ["flutter", "analyze"], ROOT / "packages" / "gui_shell_ui", "flutter"),
        ValidationStep("shared_flutter_test", ["flutter", "test"], ROOT / "packages" / "gui_shell_ui", "flutter"),
        ValidationStep("desktop_flutter_analyze", ["flutter", "analyze"], ROOT / "apps" / "desktop_flutter", "flutter"),
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
    return steps


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


def run_step(step: ValidationStep, strict_release: bool, desktop_platform: str) -> dict:
    step_command = resolve_step_command(step.command)
    if strict_release and step.name == "release_gate_check":
        step_command.append("--strict-release")
    command = " ".join(step_command)
    if step.required_tool and find_tool(step.required_tool) is None:
        classification, blocks_release, reason, required_action = classify_not_run(step, strict_release)
        return {
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
        }

    try:
        completed = subprocess.run(
            step_command,
            cwd=step.cwd,
            check=False,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
    except OSError as exc:
        classification, blocks_release, _, _ = classify_not_run(step, strict_release)
        return {
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
        }
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
    return {
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
    }


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
    parser.add_argument(
        "--python-only",
        action="store_true",
        help="Rust と Flutter を別途検証する場合に、Python/core validation step だけを実行する。",
    )
    args = parser.parse_args()

    mode = "strict_release" if args.strict_release else "development"
    results = [
        run_step(step, args.strict_release, args.desktop_platform)
        for step in build_steps(args.include_mobile_release, args.desktop_platform, args.python_only)
    ]
    evidence = platform_evidence_checks(args.desktop_platform, args.strict_release)
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
    return 1 if blockers else 0


if __name__ == "__main__":
    raise SystemExit(main())
