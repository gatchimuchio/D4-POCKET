"""C30 全数回帰の開発用検証ハーネス。

既存の検証経路を、C30が要求する機能群へ明示的に対応付ける。
このfileは権限、Approval、Audit、Recovery、Runtime、Agentを実行時に生成せず、
既存のBroker／Flutter／contract検証を起動して結果と証拠範囲だけをboundedに記録する。
外部実機、未導入Agent、未接続Runtimeを成功へ昇格しない。
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import shutil
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
RUST_ROOT = ROOT / "native" / "rust_helper"
SHARED_UI_ROOT = ROOT / "packages" / "gui_shell_ui"
MOBILE_ROOT = ROOT / "apps" / "mobile_flutter"


@dataclass(frozen=True)
class ValidationCommand:
    display: str
    command: tuple[str, ...]
    cwd: Path
    required_tool: str | None
    timeout_seconds: float


@dataclass(frozen=True)
class RegressionArea:
    area: str
    group: str
    evidence: tuple[str, ...]
    claim_scope: str
    commands: tuple[ValidationCommand, ...]


def _python(script: str, *args: str, cwd: Path = ROOT, timeout: float = 120.0) -> ValidationCommand:
    return ValidationCommand(
        display=f"python {script}{''.join(f' {arg}' for arg in args)}",
        command=(sys.executable, script, *args),
        cwd=cwd,
        required_tool=None,
        timeout_seconds=timeout,
    )


def _cargo(*args: str, timeout: float = 300.0) -> ValidationCommand:
    return ValidationCommand(
        display=f"cargo {' '.join(args)}",
        command=("cargo", *args),
        cwd=RUST_ROOT,
        required_tool="cargo",
        timeout_seconds=timeout,
    )


def _flutter(cwd: Path, args: tuple[str, ...], timeout: float = 240.0) -> ValidationCommand:
    return ValidationCommand(
        display=f"flutter {' '.join(args)}",
        command=("flutter", *args),
        cwd=cwd,
        required_tool="flutter",
        timeout_seconds=timeout,
    )


def _commands() -> tuple[RegressionArea, ...]:
    authority = _cargo("test", "--locked", "--", "--test-threads=1")
    return (
        RegressionArea(
            area="Trust / Authority / Permission / Approval / Audit / Recovery",
            group="authority_integrity",
            evidence=("LIVE_RUNTIME", "INTERNAL_STATE", "FIXTURE"),
            claim_scope="Rust helperの既存unit・integration・Broker IPC試験とnegative境界。installed製品のrelease証拠ではない",
            commands=(authority,),
        ),
        RegressionArea(
            area="Evidence",
            group="evidence_integrity",
            evidence=("CONFIG", "INTERNAL_STATE", "FIXTURE"),
            claim_scope="Evidence bundleとruntime assertionの整合。外部監査anchorの継続性や署名を生成しない",
            commands=(
                _python("tooling/evidence_bundle.py", "--check"),
                _python("tooling/release_runtime_assertions.py", "--check"),
            ),
        ),
        RegressionArea(
            area="Runtime / Dialogue",
            group="runtime_dialogue",
            evidence=("LIVE_RUNTIME", "FIXTURE", "INTERNAL_STATE"),
            claim_scope="実Broker IPCとlocalhost MINIDORA API fixtureの対話・再起動・接続断。外部Runtimeまたはinstalled productの長時間証拠ではない",
            commands=(
                _python(
                    "tooling/long_run_validation.py",
                    "--duration-seconds",
                    "30",
                    "--interval-seconds",
                    "1",
                    timeout=100.0,
                ),
            ),
        ),
        RegressionArea(
            area="Agent",
            group="agent_interface",
            evidence=("LIVE_RUNTIME", "CONFIG"),
            claim_scope="PATH上のCodex CLI version/help interfaceだけ。実task、credential、workspace書込、Agent比較、handoffを証明しない",
            commands=(_python("tooling/agent_adapter_probe.py", "--check", timeout=20.0),),
        ),
        RegressionArea(
            area="Compare",
            group="compare_projection",
            evidence=("FIXTURE",),
            claim_scope="共有Flutterの評価比較projectionとnegative parsing。実Runtime比較、実Agent比較、release判断を生成しない",
            commands=(_flutter(SHARED_UI_ROOT, ("test", "test/evaluation_client_test.dart")),),
        ),
        RegressionArea(
            area="Device Link",
            group="device_link_projection",
            evidence=("FIXTURE",),
            claim_scope="Mobile Device Link clientのcontract・失効・背景遷移fixture。実端末、TLS実接続、native secure storageを証明しない",
            commands=(_flutter(MOBILE_ROOT, ("test", "test/device_link_test.dart")),),
        ),
    )


def _resolve_command(command: ValidationCommand) -> list[str]:
    resolved = list(command.command)
    if platform.system() != "Windows" or not resolved or command.required_tool is None:
        return resolved
    tool = resolved[0]
    if Path(tool).suffix:
        return resolved
    for suffix in (".exe", ".bat", ".cmd"):
        candidate = shutil.which(f"{tool}{suffix}")
        if candidate:
            resolved[0] = candidate
            return resolved
    return resolved


def _tool_available(required_tool: str | None) -> bool:
    if required_tool is None:
        return True
    if shutil.which(required_tool):
        return True
    if platform.system() == "Windows":
        return any(shutil.which(f"{required_tool}{suffix}") for suffix in (".exe", ".bat", ".cmd"))
    return False


def _run(command: ValidationCommand) -> dict[str, Any]:
    started = time.perf_counter()
    if not _tool_available(command.required_tool):
        return {
            "command": command.display,
            "status": "unavailable",
            "exit": None,
            "elapsedMillis": round((time.perf_counter() - started) * 1000, 2),
            "reason": f"必須toolが利用できない: {command.required_tool}",
        }
    try:
        completed = subprocess.run(
            _resolve_command(command),
            cwd=command.cwd,
            env=os.environ.copy(),
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=command.timeout_seconds,
            check=False,
        )
    except subprocess.TimeoutExpired:
        return {
            "command": command.display,
            "status": "timeout",
            "exit": None,
            "elapsedMillis": round((time.perf_counter() - started) * 1000, 2),
            "reason": "検証commandがbounded timeoutを超過した",
        }
    except OSError as error:
        return {
            "command": command.display,
            "status": "unavailable",
            "exit": None,
            "elapsedMillis": round((time.perf_counter() - started) * 1000, 2),
            "reason": f"commandを起動できない: {type(error).__name__}",
        }
    result = {
        "command": command.display,
        "status": "passed" if completed.returncode == 0 else "failed",
        "exit": completed.returncode,
        "elapsedMillis": round((time.perf_counter() - started) * 1000, 2),
        "reason": "command完了。raw outputは意図的に保持しない",
    }
    if "tooling/agent_adapter_probe.py" in command.display and completed.returncode == 0:
        try:
            probe = json.loads(completed.stdout.decode("utf-8", errors="replace"))
            if isinstance(probe, dict):
                result["observed"] = {
                    "adapter_id": probe.get("adapter_id", "unknown"),
                    "version": probe.get("version", "unknown"),
                    "status": probe.get("status", "unknown"),
                    "evidence_source": probe.get("evidence_source", "unknown"),
                }
        except (UnicodeDecodeError, json.JSONDecodeError):
            result["observed"] = {"status": "unparseable"}
    return result


def _run_report() -> dict[str, Any]:
    areas = _commands()
    groups: dict[str, list[dict[str, Any]]] = {}
    for area in areas:
        results = groups.setdefault(area.group, [])
        for command in area.commands:
            results.append(_run(command))

    matrix = []
    for area in areas:
        results = groups[area.group]
        statuses = {result["status"] for result in results}
        if statuses == {"passed"}:
            status = "passed"
        elif "failed" in statuses or "timeout" in statuses:
            status = "failed"
        else:
            status = "unavailable"
        matrix.append(
            {
                "area": area.area,
                "group": area.group,
                "status": status,
                "evidence": list(area.evidence),
                "claim_scope": area.claim_scope,
                "commands": results,
            }
        )

    statuses = {item["status"] for item in matrix}
    if statuses == {"passed"}:
        state = "passed"
    elif "failed" in statuses:
        state = "failed"
    else:
        state = "partial"
    return {
        "版": 1,
        "状態": state,
        "証拠範囲": sorted({source for item in matrix for source in item["evidence"]}),
        "回帰対象": [
            "Trust",
            "Authority",
            "Permission",
            "Approval",
            "Audit",
            "Recovery",
            "Evidence",
            "Runtime",
            "Agent",
            "Dialogue",
            "Compare",
            "Device Link",
        ],
        "matrix": matrix,
        "raw outputを保存しない": True,
        "installed製品の証明ではない": True,
        "外部Runtime・外部Agent・実端末の証明ではない": True,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="C30全数回帰の開発用検証")
    parser.parse_args()
    report = _run_report()
    print(json.dumps(report, ensure_ascii=False, indent=2))
    return 0 if report["状態"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
