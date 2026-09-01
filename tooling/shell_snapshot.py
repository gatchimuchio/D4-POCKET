from __future__ import annotations

import argparse
import json
import sys
from datetime import datetime, timezone
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from installer.setup_doctor import setup_doctor_report
from packages.shell_core.invariant_evaluator import InvariantEvaluator
from packages.shell_core.release_smoke import build_reference_state


DEFAULT_SNAPSHOT_PATH = ROOT / ".gui_shell" / "shell_snapshot.json"


def build_shell_snapshot() -> dict:
    state = build_reference_state()
    setup = setup_doctor_report()
    invariant_flags = InvariantEvaluator().evaluate()
    generated_at = datetime.now(timezone.utc).isoformat()
    runtimes = [
        {
            "runtime_id": runtime_id,
            "name": runtime.get("name", runtime_id),
            "status": runtime.get("status", "unknown"),
            "adapter_id": _adapter_for_runtime(state.adapters, runtime_id),
            "diagnostic_summary": "参照実行系の契約を利用可能",
        }
        for runtime_id, runtime in sorted(state.runtimes.items())
    ]
    permissions = [
        {
            "permission_id": permission_id,
            "capability_id": permission.get("capability_id", ""),
            "decision": permission.get("decision", "unknown"),
            "source": permission.get("source", "unknown"),
            "expires_at": permission.get("expires_at"),
        }
        for permission_id, permission in sorted(state.permissions.items())
    ]
    approvals = [
        {
            "approval_id": approval_id,
            "operation": approval.get("operation", ""),
            "status": approval.get("status", "unknown"),
            "content_visibility": approval.get("content_visibility", "redacted"),
            "projected_content": _projected_content(approval),
            "editable_fields": approval.get("editable_fields", []),
            "protected_fields": sorted(
                set(approval.get("authority_fields", []))
                | set(approval.get("sealed_fields", []))
                | set(approval.get("hidden_fields", []))
                | set(approval.get("sacred_fields", []))
                | {"payload_hash"}
            ),
        }
        for approval_id, approval in sorted(state.approvals.items())
    ]
    audit_events = [
        {
            "event_id": "audit-1",
            "action": "approval.requested",
            "result": "success",
            "payload_hash": "sha256:" + "2" * 64,
            "previous_event_hash": None,
        }
    ]
    recovery_actions = [
        {
            "recovery_id": recovery_id,
            "severity": recovery.get("severity", "warning"),
            "message": recovery.get("user_visible_message", ""),
            "safe_to_retry": recovery.get("safe_to_retry", False),
        }
        for recovery_id, recovery in sorted(state.recovery_actions.items())
    ]
    problems = _problems()
    release_blocker_count = sum(
        1 for problem in problems if problem.get("classification") == "release_blocker"
    )
    return {
        "snapshot_source": "generated",
        "snapshot_path": str(DEFAULT_SNAPSHOT_PATH.relative_to(ROOT)),
        "snapshot_generated_at": generated_at,
        "snapshot_freshness": generated_at,
        "phase_status": _phase_status(),
        "operation_status": _operation_status(runtimes, approvals, invariant_flags, problems),
        "runtimes": runtimes,
        "agent_sessions": [
            {
                "session_id": "agent-session-reference",
                "workspace": "/workspace/project",
                "task": "GUI-Shellのリリース証拠を検証する",
                "changed_files": ["VALIDATION.txt", "docs/GUI_OPERATION_SURFACES.md"],
                "tool_calls": ["schema_check", "conformance", "release_smoke"],
                "shell_commands": ["python3 tooling/validate_all.py"],
                "test_status": "開発検証に合格",
                "diff_summary": "操作画面と証拠経路を利用可能",
                "pending_approval_count": len(approvals),
                "rollback_candidate": "recover-1",
                "audit_event_id": "audit-1",
            }
        ],
        "permissions": permissions,
        "pending_approvals": approvals,
        "audit_events": audit_events,
        "recovery_actions": recovery_actions,
        "invariant_flags": invariant_flags,
        "setup_doctor_checks": setup["checks"],
        "setup_doctor_status": setup["status"],
        "installer_grants_authority": setup["installer_grants_authority"],
        "installer_silently_approves_permissions": setup["installer_silently_approves_permissions"],
        "trust_records": _trust_records(),
        "authority_map": _authority_map(state, approvals, audit_events, recovery_actions),
        "adapter_catalog": _adapter_catalog(state),
        "permission_diffs": _permission_diffs(),
        "problems": problems,
        "evidence": _evidence_records(),
        "settings": _settings_records(),
        "audit_chain_status": "verified",
        "network_exposure": "localhost only",
        "release_blocker_count": release_blocker_count,
        "evidence_summary": _evidence_summary(),
        "recovery_playbook": _recovery_playbook(),
    }


def _phase_status() -> dict:
    return {
        "phase_a_status": "complete",
        "phase_b_status": "complete",
        "phase_c_status": "next",
        "phase_d_status": "later",
        "phase_e_status": "later",
        "phase_f_status": "later",
        "completed_product_release_claimed": False,
    }


def _operation_status(
    runtimes: list[dict],
    approvals: list[dict],
    invariant_flags: dict,
    problems: list[dict],
) -> dict:
    return {
        "runtime_status": runtimes[0]["status"] if runtimes else "unknown",
        "invariant_status": "blocked" if any(invariant_flags.values()) else "ok",
        "trust_status": "restricted",
        "pending_approvals_count": len(approvals),
        "audit_chain_status": "verified",
        "problems_count": len(problems),
        "release_state": "not claimed",
    }


def _adapter_for_runtime(adapters: dict, runtime_id: str) -> str:
    for adapter_id, adapter in sorted(adapters.items()):
        if adapter.get("runtime_id") == runtime_id:
            return adapter_id
    return ""


def _projected_content(approval: dict) -> dict:
    if approval.get("content_visibility") == "full":
        return dict(approval.get("full_payload", {}))
    payload = approval.get("full_payload", {})
    if isinstance(payload, dict) and "path" in payload:
        return {"path": payload["path"], "content": "[redacted]"}
    return {"summary": "[redacted]"}


def _trust_records() -> list[dict]:
    return [
        {
            "scope": "workspace_trust",
            "state": "restricted",
            "source": "ローカル方針",
            "expires_at": None,
            "blocked_operations": ["process.spawn", "network.public_bind"],
        },
        {
            "scope": "runtime_trust",
            "state": "trusted",
            "source": "署名済みマニフェスト",
            "expires_at": None,
            "blocked_operations": [],
        },
        {
            "scope": "adapter_trust",
            "state": "inherited",
            "source": "runtime_trust",
            "expires_at": None,
            "blocked_operations": [],
        },
        {
            "scope": "installer_trust",
            "state": "unknown",
            "source": "インストール先証拠なし",
            "expires_at": None,
            "blocked_operations": ["release_ready_claim"],
        },
    ]


def _authority_map(state, approvals: list[dict], audit_events: list[dict], recovery_actions: list[dict]) -> list[dict]:
    rows = []
    approval_id = approvals[0]["approval_id"] if approvals else ""
    audit_id = audit_events[0]["event_id"] if audit_events else ""
    recovery_id = recovery_actions[0]["recovery_id"] if recovery_actions else ""
    for permission_id, permission in sorted(state.permissions.items()):
        capability_id = permission.get("capability_id", "")
        capability = state.capabilities.get(capability_id, {})
        rows.append(
            {
                "runtime_id": capability.get("runtime_id", ""),
                "capability_id": capability_id,
                "permission_id": permission_id,
                "approval_id": approval_id,
                "audit_event_id": audit_id,
                "recovery_id": recovery_id,
                "dangerous": capability_id in {"process.spawn", "network.public_bind"},
                "warning": "" if permission.get("decision") in {"allow", "approved"} else "許可が付与されていない",
            }
        )
    return rows


def _adapter_catalog(state) -> list[dict]:
    return [
        {
            "adapter_id": adapter_id,
            "runtime_id": adapter.get("runtime_id", ""),
            "publisher": "GUI-Shell参照実装",
            "version": adapter.get("contract_version", "unknown"),
            "signature": "development",
            "hash": "sha256:pending",
            "requested_capabilities": list(adapter.get("declared_capabilities", [])),
            "granted_capabilities": [],
            "denied_capabilities": ["network.public_bind"],
            "trust_status": "inherited",
            "last_verified": "release_smoke",
            "update_available": False,
            "known_risks": ["参照アダプター専用"],
        }
        for adapter_id, adapter in sorted(state.adapters.items())
    ]


def _permission_diffs() -> list[dict]:
    return [
        {
            "subject": "blue_tanuki_reference",
            "added": ["filesystem.write"],
            "removed": [],
            "changed": ["content_visibility: full → redacted（完全表示から墨消し表示）"],
            "dangerous": [],
        }
    ]


def _problems() -> list[dict]:
    return [
        {
            "problem_id": "windows-installed-evidence-missing",
            "severity": "blocked",
            "category": "missing_evidence",
            "message": "Windowsインストール先の証拠がありません。",
            "target": "release_evidence/windows_installed_smoke.json",
            "recovery_id": "recover-windows-evidence",
            "item": "Windowsインストール先の初回起動実測証拠なし",
            "classification": "release_blocker",
            "reason": "インストール先の初回起動を実測した証拠が記録されていません。",
            "required_action": "ネイティブWindowsで強化済みインストール簡易検査を実行してください。",
            "blocks_release": True,
            "safe_to_ignore_for_phase_b": True,
            "blocks_owner_use": False,
            "blocks_completed_product_release": True,
        },
        {
            "problem_id": "setup-doctor-installed-evidence-missing",
            "severity": "blocked",
            "category": "missing_evidence",
            "message": "インストール先で生成した非合成の環境診断証拠がありません。",
            "target": "release_evidence/windows_installed_smoke.json",
            "recovery_id": "recover-setup-doctor-evidence",
            "item": "インストール先の非合成環境診断証拠なし",
            "classification": "release_blocker",
            "reason": "インストール済みアプリのパスから環境診断を実行した証明がありません。",
            "required_action": "インストール済みWindowsアプリのパスから環境診断を実行し、必須検査を記録してください。",
            "blocks_release": True,
            "safe_to_ignore_for_phase_b": True,
            "blocks_owner_use": False,
            "blocks_completed_product_release": True,
        },
        {
            "problem_id": "owner-go-missing",
            "severity": "blocked",
            "category": "release_gate",
            "message": "所有者GOがありません。",
            "target": "リリース確認表",
            "recovery_id": "recover-owner-go",
            "item": "所有者GOなし",
            "classification": "release_blocker",
            "reason": "完成製品のリリースには所有者の明示承認が必要です。",
            "required_action": "リリース遮断要因を解消した後に所有者GOを記録してください。",
            "blocks_release": True,
            "safe_to_ignore_for_phase_b": True,
            "blocks_owner_use": False,
            "blocks_completed_product_release": True,
        },
        {
            "problem_id": "audit-anchor-external-tamper-evidence-missing",
            "severity": "blocked",
            "category": "missing_evidence",
            "message": "監査アンカーの外部改ざん耐性証明がありません。",
            "target": "release_evidence/windows_installed_smoke.json",
            "recovery_id": "recover-audit-anchor-external-proof",
            "item": "監査アンカーの外部改ざん耐性証明なし",
            "classification": "release_blocker",
            "reason": "ローカルHMACアンカーだけでは、Windows ACL／DPAPI、外部アンカー、署名済み証拠がない限り、同一ユーザーまたは管理者／rootによる書換えへの耐性を証明できません。",
            "required_action": "Windowsインストール先で監査アンカー鍵の保護証拠または外部アンカー証拠を収集してください。",
            "blocks_release": True,
            "safe_to_ignore_for_phase_b": True,
            "blocks_owner_use": False,
            "blocks_completed_product_release": True,
        },
        {
            "problem_id": "macos-unverified",
            "severity": "info",
            "category": "scope",
            "message": "macOSは未検証です。",
            "target": "デスクトップ対応表",
            "recovery_id": "recover-macos-validation",
            "item": "macOS未検証",
            "classification": "known_limitation",
            "reason": "macOS検証環境を利用できません。",
            "required_action": "macOS対応を主張する前にmacOS上で検証してください。",
            "blocks_release": False,
            "safe_to_ignore_for_phase_b": True,
            "blocks_owner_use": False,
            "blocks_completed_product_release": False,
        },
        {
            "problem_id": "mobile-post-v1",
            "severity": "info",
            "category": "scope",
            "message": "モバイル完全リリースはv1後の範囲です。",
            "target": "モバイル状態",
            "recovery_id": "recover-mobile-scope",
            "item": "モバイルはv1後の範囲",
            "classification": "post_v1_scope",
            "reason": "所有者が範囲を変更しない限り、v1.0はWindows優先のデスクトップ版です。",
            "required_action": "モバイルのリリース作業を保留してください。",
            "blocks_release": False,
            "safe_to_ignore_for_phase_b": True,
            "blocks_owner_use": False,
            "blocks_completed_product_release": False,
        },
        {
            "problem_id": "paid-qc-later",
            "severity": "info",
            "category": "scope",
            "message": "有償製品の品質管理は後続段階です。",
            "target": "段階戦略",
            "recovery_id": "recover-paid-qc",
            "item": "有償製品の品質管理は後続",
            "classification": "post_v1_scope",
            "reason": "段階Bは所有者利用の堅牢化であり、有償製品の品質管理ではありません。",
            "required_action": "有償製品の品質管理は段階Fまで保留してください。",
            "blocks_release": False,
            "safe_to_ignore_for_phase_b": True,
            "blocks_owner_use": False,
            "blocks_completed_product_release": False,
        }
    ]


def _evidence_records() -> list[dict]:
    return [
        {
            "evidence_id": "windows-installed-smoke",
            "kind": "installed-path",
            "status": "missing",
            "path": "release_evidence/windows_installed_smoke.json",
            "hash": "",
            "exportable": False,
        },
        {
            "evidence_id": "development-validation",
            "kind": "validation",
            "status": "passed",
            "path": "tooling/validate_all.py",
            "hash": "",
            "exportable": True,
        },
    ]


def _settings_records() -> list[dict]:
    return [
        {
            "key": "content_visibility.default",
            "group": "authority",
            "default": "redacted",
            "current": "redacted",
            "effective": "redacted",
            "source": "Shell Core方針",
            "modified": False,
            "dangerous": False,
            "authority_related": True,
        },
        {
            "key": "network.public_bind",
            "group": "runtime",
            "default": "blocked",
            "current": "blocked",
            "effective": "blocked",
            "source": "許可台帳",
            "modified": False,
            "dangerous": True,
            "authority_related": True,
        },
        {
            "key": "phase.owner_use",
            "group": "phase",
            "default": "complete",
            "current": "complete",
            "effective": "complete",
            "source": "docs/PHASE_STRATEGY.md",
            "modified": False,
            "dangerous": False,
            "authority_related": False,
        },
        {
            "key": "release.state",
            "group": "release",
            "default": "not claimed",
            "current": "not claimed",
            "effective": "not claimed",
            "source": "厳格リリース関門",
            "modified": False,
            "dangerous": True,
            "authority_related": True,
        },
    ]


def _evidence_summary() -> dict:
    return {
        "schema_check": "passed",
        "conformance_check_count": 138,
        "release_smoke": "passed",
        "release_gate_check": "passed",
        "evidence_bundle": "passed",
        "validate_all": "passed",
        "strict_windows_release": "expected fail",
        "missing_measured_windows_evidence": True,
        "missing_setup_doctor_evidence": True,
        "missing_audit_anchor_external_tamper_evidence": True,
        "owner_go": "missing",
    }


def _recovery_playbook() -> list[dict]:
    return [
        {
            "recovery_id": "recover-windows-evidence",
            "item": "Windowsインストール先の実測証拠なし",
            "severity": "release",
            "classification": "release_blocker",
            "safe_to_ignore_for_phase_b": True,
            "blocks_owner_use": False,
            "required_action": "ネイティブWindowsで強化済みインストール簡易検査を実行してください。",
            "blocks_completed_product_release": True,
            "command": "python tooling/windows_release_evidence.py",
            "path": "release_evidence/windows_installed_smoke.json",
        },
        {
            "recovery_id": "recover-setup-doctor-evidence",
            "item": "インストール先の非合成環境診断証拠なし",
            "severity": "release",
            "classification": "release_blocker",
            "safe_to_ignore_for_phase_b": True,
            "blocks_owner_use": False,
            "required_action": "インストール先から環境診断を実行し、非合成の検査結果を記録してください。",
            "blocks_completed_product_release": True,
            "command": "python tooling/windows_release_evidence.py",
            "path": "release_evidence/windows_installed_smoke.json",
        },
        {
            "recovery_id": "recover-owner-go",
            "item": "所有者GOなし",
            "severity": "release",
            "classification": "release_blocker",
            "safe_to_ignore_for_phase_b": True,
            "blocks_owner_use": False,
            "required_action": "リリース遮断要因の検査合格後に所有者GOを記録してください。",
            "blocks_completed_product_release": True,
            "command": "",
            "path": "RELEASE_CHECKLIST.md",
        },
        {
            "recovery_id": "recover-audit-anchor-external-proof",
            "item": "監査アンカーの外部改ざん耐性証明なし",
            "severity": "release",
            "classification": "release_blocker",
            "safe_to_ignore_for_phase_b": True,
            "blocks_owner_use": False,
            "required_action": "監査アンカーファイルについて、Windowsインストール先のACL／DPAPI、外部アンカー、または署名済み証拠を収集してください。",
            "blocks_completed_product_release": True,
            "command": "python tooling/windows_release_evidence.py",
            "path": "release_evidence/windows_installed_smoke.json",
        },
        {
            "recovery_id": "recover-macos-validation",
            "item": "macOS未検証",
            "severity": "scope",
            "classification": "known_limitation",
            "safe_to_ignore_for_phase_b": True,
            "blocks_owner_use": False,
            "required_action": "macOS対応を主張する前にmacOS上で検証してください。",
            "blocks_completed_product_release": False,
            "command": "",
            "path": "docs/PHASE_STRATEGY.md",
        },
        {
            "recovery_id": "recover-mobile-scope",
            "item": "モバイル完全リリース",
            "severity": "scope",
            "classification": "post_v1_scope",
            "safe_to_ignore_for_phase_b": True,
            "blocks_owner_use": False,
            "required_action": "モバイル完全リリースはv1後まで保留してください。",
            "blocks_completed_product_release": False,
            "command": "",
            "path": "docs/PHASE_STRATEGY.md",
        },
        {
            "recovery_id": "recover-owner-use-surfaces",
            "item": "段階Bの所有者利用上の問題",
            "severity": "owner-use",
            "classification": "required_for_v1",
            "safe_to_ignore_for_phase_b": False,
            "blocks_owner_use": True,
            "required_action": "概要、状態、問題、証拠、復旧の各画面を利用可能に保ってください。",
            "blocks_completed_product_release": False,
            "command": "bash scripts/launch_owner_desktop.sh",
            "path": ".gui_shell/shell_snapshot.json",
        },
    ]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path)
    parser.add_argument("--write", type=Path)
    args = parser.parse_args()
    snapshot = build_shell_snapshot()
    output_path = args.write or args.output
    if output_path:
        snapshot["snapshot_path"] = str(output_path)
    encoded = json.dumps(snapshot, indent=2, sort_keys=True)
    if output_path:
        output_path.parent.mkdir(parents=True, exist_ok=True)
        output_path.write_text(encoded + "\n", encoding="utf-8")
    else:
        print(encoded)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
