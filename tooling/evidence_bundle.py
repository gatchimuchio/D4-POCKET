from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from installer.setup_doctor import setup_doctor_report
from tooling.release_runtime_assertions import build_report as build_release_runtime_assertions
from tooling.release_smoke import run_release_smokes
from tooling.shell_snapshot import build_shell_snapshot
from tooling.windows_release_evidence import validate_windows_release_evidence


DEFAULT_BUNDLE_PATH = ROOT / "release_evidence" / "evidence_bundle.json"


def build_evidence_bundle() -> dict:
    windows_evidence = validate_windows_release_evidence()
    release_runtime_assertions = build_release_runtime_assertions()
    release_smoke = run_release_smokes()
    shell_snapshot = build_shell_snapshot()
    setup_doctor = setup_doctor_report()
    blockers = [
        {
            "name": result.name,
            "status": result.status,
            "classification": result.classification,
            "blocks_release": result.blocks_release == "yes",
            "reason": result.reason,
            "required_action": result.required_action,
        }
        for result in windows_evidence
        if result.classification == "release_blocker"
    ]
    return {
        "bundle_version": 1,
        "product": "GUI-Shell",
        "release_ready": False,
        "release_ready_reason": "Windows installed-path evidence and audit anchor external tamper-evidence proof are required before completed product release.",
        "classification": "development_evidence",
        "windows_release_evidence": [result.__dict__ for result in windows_evidence],
        "release_runtime_assertions": release_runtime_assertions,
        "release_smoke": release_smoke,
        "setup_doctor": setup_doctor,
        "shell_snapshot": shell_snapshot,
        "blockers": blockers,
        "authority_boundary": {
            "flutter_owns_authority": False,
            "flutter_authority_surface_broker_mediated": release_runtime_assertions["ok"],
            "flutter_spawns_python_for_authority": False,
            "flutter_rust_ffi_authority_bridge": False,
            "installer_grants_authority": setup_doctor["installer_grants_authority"],
            "installer_silently_approves_permissions": setup_doctor[
                "installer_silently_approves_permissions"
            ],
            "shell_core_authority_required": True,
        },
    }


def validate_evidence_bundle(bundle: dict) -> list[str]:
    errors: list[str] = []
    if bundle.get("release_ready") is not False:
        errors.append("development evidence bundle must not claim release_ready")
    if bundle.get("authority_boundary", {}).get("flutter_owns_authority") is not False:
        errors.append("evidence bundle says Flutter owns authority")
    if bundle.get("authority_boundary", {}).get("flutter_authority_surface_broker_mediated") is not True:
        errors.append("evidence bundle missing broker-mediated Flutter authority assertion")
    if bundle.get("authority_boundary", {}).get("flutter_spawns_python_for_authority") is not False:
        errors.append("evidence bundle says Flutter spawns Python for authority")
    if bundle.get("authority_boundary", {}).get("flutter_rust_ffi_authority_bridge") is not False:
        errors.append("evidence bundle says Flutter uses FFI authority bridge")
    if bundle.get("authority_boundary", {}).get("installer_grants_authority") is not False:
        errors.append("evidence bundle says installer grants authority")
    if not bundle.get("shell_snapshot", {}).get("trust_records"):
        errors.append("evidence bundle missing trust records")
    if not bundle.get("shell_snapshot", {}).get("authority_map"):
        errors.append("evidence bundle missing authority map")
    if not bundle.get("setup_doctor", {}).get("checks"):
        errors.append("evidence bundle missing Setup Doctor checks")
    if not bundle.get("release_smoke", {}).get("ok"):
        errors.append("release smoke failed inside evidence bundle")
    if not bundle.get("release_runtime_assertions", {}).get("ok"):
        errors.append("release runtime assertions failed inside evidence bundle")
    for index, blocker in enumerate(bundle.get("blockers", [])):
        if not isinstance(blocker, dict):
            errors.append(f"evidence bundle blocker {index} is not an object")
            continue
        for key in ["name", "status", "classification", "blocks_release", "reason", "required_action"]:
            if key not in blocker:
                errors.append(f"evidence bundle blocker {index} missing {key}")
        if blocker.get("classification") != "release_blocker":
            errors.append(f"evidence bundle blocker {index} is not release_blocker")
        if blocker.get("blocks_release") is not True:
            errors.append(f"evidence bundle blocker {index} must block release")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    bundle = build_evidence_bundle()
    errors = validate_evidence_bundle(bundle)
    if errors:
        print("evidence bundle validation failed:")
        for error in errors:
            print(f"  - {error}")
        return 1
    if args.check:
        print(
            "evidence bundle check passed: "
            f"{len(bundle['blockers'])} release blockers preserved, "
            f"release_ready={bundle['release_ready']}, "
            f"classification={bundle['classification']}"
        )
        return 0
    encoded = json.dumps(bundle, indent=2, sort_keys=True)
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(encoded + "\n", encoding="utf-8")
    else:
        print(encoded)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
