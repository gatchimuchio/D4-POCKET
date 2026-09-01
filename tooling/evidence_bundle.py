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
        "release_ready_reason": "完成製品releaseの前にWindows installed-path evidenceとaudit anchorの外部tamper-evidence proofが必要である。",
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
        errors.append("development evidence bundleはrelease_readyを主張してはならない")
    if bundle.get("authority_boundary", {}).get("flutter_owns_authority") is not False:
        errors.append("evidence bundleがFlutterによるauthority所有を示している")
    if bundle.get("authority_boundary", {}).get("flutter_authority_surface_broker_mediated") is not True:
        errors.append("evidence bundleにFlutter authority surfaceがbroker-mediatedであるというassertionがない")
    if bundle.get("authority_boundary", {}).get("flutter_spawns_python_for_authority") is not False:
        errors.append("evidence bundleがFlutterによるauthority用Python spawnを示している")
    if bundle.get("authority_boundary", {}).get("flutter_rust_ffi_authority_bridge") is not False:
        errors.append("evidence bundleがFlutterによるFFI authority bridge使用を示している")
    if bundle.get("authority_boundary", {}).get("installer_grants_authority") is not False:
        errors.append("evidence bundleがinstallerによるauthority付与を示している")
    if not bundle.get("shell_snapshot", {}).get("trust_records"):
        errors.append("evidence bundleにtrust recordがない")
    if not bundle.get("shell_snapshot", {}).get("authority_map"):
        errors.append("evidence bundleにauthority mapがない")
    if not bundle.get("setup_doctor", {}).get("checks"):
        errors.append("evidence bundleにSetup Doctor checkがない")
    if not bundle.get("release_smoke", {}).get("ok"):
        errors.append("evidence bundle内のrelease smokeが失敗")
    if not bundle.get("release_runtime_assertions", {}).get("ok"):
        errors.append("evidence bundle内のrelease runtime assertionが失敗")
    for index, blocker in enumerate(bundle.get("blockers", [])):
        if not isinstance(blocker, dict):
            errors.append(f"evidence bundle blocker {index}がobjectではない")
            continue
        for key in ["name", "status", "classification", "blocks_release", "reason", "required_action"]:
            if key not in blocker:
                errors.append(f"evidence bundle blocker {index}に{key}がない")
        if blocker.get("classification") != "release_blocker":
            errors.append(f"evidence bundle blocker {index}がrelease_blockerではない")
        if blocker.get("blocks_release") is not True:
            errors.append(f"evidence bundle blocker {index}はreleaseを阻止しなければならない")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    bundle = build_evidence_bundle()
    errors = validate_evidence_bundle(bundle)
    if errors:
        print("evidence bundle validationが失敗:")
        for error in errors:
            print(f"  - {error}")
        return 1
    if args.check:
        print(
            "evidence bundle checkが合格: "
            f"release blockerを{len(bundle['blockers'])} 件保持、"
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
