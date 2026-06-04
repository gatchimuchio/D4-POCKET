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
        ValidationStep("schema_check", python_step("tooling/schema_check/check_schemas.py"), ROOT),
        ValidationStep(
            "conformance_skeleton",
            python_step("tooling/conformance_tests/run_conformance_skeleton.py"),
            ROOT,
        ),
        ValidationStep("manifest_check", python_step("tooling/manifest.py", "--check"), ROOT),
        ValidationStep("release_gate_check", python_step("tooling/release_gate_check.py"), ROOT),
        ValidationStep("release_smoke", python_step("tooling/release_smoke.py"), ROOT),
        ValidationStep("evidence_bundle", python_step("tooling/evidence_bundle.py", "--check"), ROOT),
        ValidationStep(
            "release_runtime_assertions",
            python_step("tooling/release_runtime_assertions.py", "--check"),
            ROOT,
        ),
        ValidationStep(
            "broker_authority_parity",
            python_step("tooling/broker_parity/run_authority_parity.py"),
            ROOT,
        ),
    ]
    if python_only:
        return steps
    steps.extend([
        ValidationStep("rust_helper_cargo_test", ["cargo", "test"], ROOT / "native" / "rust_helper", "cargo"),
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
            else "mobile full release is outside v1.0 desktop scope unless owner explicitly includes mobile",
        )
    )
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
                    "Linux desktop build smoke passed on 2026-05-25.",
                    "Keep Linux build smoke passing as the development/verification slice.",
                ),
                EvidenceCheck(
                    "linux_desktop_launch_smoke",
                    "passed",
                    "none",
                    "no",
                    "Linux desktop launch smoke passed under WSLg with first-window evidence recorded; this is useful proof but not final Windows-first product proof by itself.",
                    "Keep Linux launch smoke passing while completing Windows-first release evidence.",
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
                    "`apps/desktop_flutter/windows` exists." if windows_project_exists else "`apps/desktop_flutter/windows` is not present, so Windows desktop project support is not generated.",
                    "Keep Windows Flutter desktop project files under version control." if windows_project_exists else "Generate Windows desktop project support and commit the bounded Flutter desktop project files.",
                ),
                EvidenceCheck(
                    "windows_development_toolchain_historical_smoke",
                    "historical_invalid_for_current_r2",
                    "none",
                    "no",
                    "Historical native Windows analyze/test/build/launch smoke is preserved as owner-trial history only. It is not current R2 formal evidence because it is not tied to the exact implementation commit and installed-path evidence bundle.",
                    "Collect fresh native Windows release-candidate evidence through the isolated installed-path evidence flow.",
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
                    "No macOS validation environment is currently available; GUI-Shell v1.0 does not claim verified macOS support.",
                    "Validate on a macOS host before claiming macOS support.",
                ),
                EvidenceCheck(
                    "macos_flutter_toolchain",
                    "unverified_planned",
                    "known_limitation",
                    "no",
                    "No macOS validation environment is currently available.",
                    "Validate macOS Flutter toolchain on macOS before claiming support.",
                ),
                EvidenceCheck(
                    "macos_desktop_build_smoke",
                    "unverified_planned",
                    "known_limitation",
                    "no",
                    "macOS build smoke has not run because no macOS validation environment is currently available.",
                    "Pass `flutter build macos` on a macOS host before claiming support.",
                ),
                EvidenceCheck(
                    "macos_desktop_launch_smoke",
                    "unverified_planned",
                    "known_limitation",
                    "no",
                    "macOS launch smoke evidence is not recorded because no macOS validation environment is currently available.",
                    "Launch macOS artifact and record evidence before claiming support.",
                ),
                EvidenceCheck(
                    "macos_packaging_notarization_plan",
                    "unverified_planned",
                    "known_limitation",
                    "no",
                    "macOS packaging/notarization remains planned portability validation.",
                    "Document and validate on macOS before claiming support.",
                ),
                EvidenceCheck(
                    "macos_installer_first_run_smoke",
                    "unverified_planned",
                    "known_limitation",
                    "no",
                    "macOS installer/first-run smoke is not in the Windows-first v1.0 release gate.",
                    "Validate on macOS host before claiming support.",
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
                "Explicit owner GO has not been recorded.",
                "Record explicit owner GO only after measured installed-path release evidence and strict validation pass.",
            )
        )
    return checks


def classify_not_run(step: ValidationStep, strict_release: bool) -> tuple[str, str, str, str]:
    if step.in_release_scope and strict_release:
        return ("release_blocker", "yes", f"{step.required_tool} not found on PATH", f"Install {step.required_tool} and rerun release validation.")
    if not step.in_release_scope:
        return ("post_v1_scope", "no", step.post_v1_reason or "outside v1.0 scope", "No v1.0 action required.")
    return ("release_blocker", "yes", f"{step.required_tool} not found on PATH", f"Install {step.required_tool} before release validation.")


def run_step(step: ValidationStep, strict_release: bool, desktop_platform: str) -> dict:
    step_command = list(step.command)
    if strict_release and desktop_platform == "all" and step.name == "release_gate_check":
        step_command.append("--strict-release")
    command = " ".join(step_command)
    if step.required_tool and shutil.which(step.required_tool) is None:
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
            "reason": f"validation command could not be started: {exc.__class__.__name__}: {exc}",
            "required_action": "Fix the validation host toolchain or command resolution and rerun; validator failures must remain structured release blockers.",
            "stdout": "",
            "stderr": traceback.format_exc().rstrip(),
            "exit": "",
        }
    passed = completed.returncode == 0
    if not step.in_release_scope:
        classification = "post_v1_scope"
        blocks_release = "no"
        reason = step.post_v1_reason or "outside v1.0 scope"
        required_action = "No v1.0 action required unless owner includes this scope."
    else:
        classification = "none"
        blocks_release = "no"
        reason = ""
        required_action = ""
    if not passed and step.in_release_scope:
        classification = "release_blocker"
        blocks_release = "yes"
        if strict_release and desktop_platform == "all" and step.name == "release_gate_check":
            reason = "all-desktop strict release gate found documented release_blocker classifications"
            required_action = "Resolve every classified Windows-first release_blocker, then rerun all-desktop strict validation. macOS remains an unverified known limitation unless owner changes scope."
        else:
            reason = "validation command failed"
            required_action = "Fix the failing validation command and rerun."
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
        help="Treat mobile Flutter validation as in-scope for the release gate.",
    )
    parser.add_argument(
        "--desktop-platform",
        choices=["current", "windows", "linux", "macos", "all"],
        default="current",
        help="Validate current host, a named desktop target, or the full Windows/macOS/Linux desktop scope.",
    )
    parser.add_argument(
        "--python-only",
        action="store_true",
        help="Run only Python/core validation steps for CI jobs that split Rust and Flutter into separate jobs.",
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
