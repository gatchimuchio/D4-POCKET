from __future__ import annotations

import argparse
import json
import re
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
DESKTOP_FLUTTER = ROOT / "apps" / "desktop_flutter"


@dataclass(frozen=True)
class RuntimeAssertion:
    name: str
    status: str
    evidence_scope: str
    classification: str
    blocks_release: str
    reason: str
    required_action: str


def _pass(name: str, evidence_scope: str, reason: str, required_action: str) -> RuntimeAssertion:
    return RuntimeAssertion(name, "passed", evidence_scope, "none", "no", reason, required_action)


def _fail(name: str, evidence_scope: str, reason: str, required_action: str) -> RuntimeAssertion:
    return RuntimeAssertion(name, "failed", evidence_scope, "release_blocker", "yes", reason, required_action)


def _read(relative: str) -> str:
    return (ROOT / relative).read_text(encoding="utf-8")


def _product_body() -> str:
    text = _read("apps/desktop_flutter/lib/services/shell_core_client.dart")
    start = text.find("static Future<ShellCoreClient> product")
    end = text.find("factory ShellCoreClient.local", start)
    if start == -1 or end == -1:
        return ""
    return text[start:end]


def _broker_snapshot_body() -> str:
    text = _read("apps/desktop_flutter/lib/services/shell_core_client.dart")
    start = text.find("ShellSnapshot _brokerSnapshot")
    end = text.find("ShellSnapshot _brokerUnavailableSnapshot", start)
    if start == -1 or end == -1:
        return ""
    return text[start:end]


def _function_body(function_name: str) -> str:
    text = _read("apps/desktop_flutter/lib/services/shell_core_client.dart")
    start = text.find(f"Map<String, Object?> {function_name}")
    if start == -1:
        return ""
    end = text.find("\n}\n", start)
    if end == -1:
        return text[start:]
    return text[start : end + 3]


def _scan_files(paths: list[Path], tokens: list[str]) -> list[str]:
    findings: list[str] = []
    for path in sorted(paths):
        if not path.exists() or not path.is_file():
            continue
        text = path.read_text(encoding="utf-8")
        for token in tokens:
            if token in text:
                findings.append(f"{path.relative_to(ROOT).as_posix()} が{token}を含む")
    return findings


def _desktop_lib_files() -> list[Path]:
    return list((DESKTOP_FLUTTER / "lib").rglob("*.dart"))


def assert_flutter_product_entry_uses_broker() -> RuntimeAssertion:
    main = _read("apps/desktop_flutter/lib/main.dart")
    if "await ShellCoreClient.product()" not in main:
        return _fail(
            "flutter_product_entry_uses_broker",
            "CONFIG",
            "apps/desktop_flutter/lib/main.dartが製品entry pathでShellCoreClient.product()を呼び出していない。",
            "release validationの前に製品起動をShellCoreClient.product()経由にする。",
        )
    if "ShellCoreClient.local(" in main:
        return _fail(
            "flutter_product_entry_uses_broker",
            "CONFIG",
            "apps/desktop_flutter/lib/main.dartの製品起動codeがまだShellCoreClient.local()を参照している。",
            "製品起動からlocal snapshotのauthority利用を除去する。",
        )
    return _pass(
        "flutter_product_entry_uses_broker",
        "CONFIG",
        "製品のmain.dartがShellCoreClient.product()を初期化し、ShellCoreClient.local()を参照していない。",
        "製品起動はbroker-mediated client経由を維持する。",
    )


def assert_flutter_authority_operations_are_broker_mediated() -> RuntimeAssertion:
    body = _product_body()
    required = [
        "await BrokerClient.connect()",
        "broker.request('health'",
        "broker.request(\n        'normalize_payload'",
        "broker.request(\n        'content_projection'",
        "broker.request(\n        'approval_edit'",
        "broker.request(\n        'command_envelope'",
        "_brokerUnavailableSnapshot",
    ]
    missing = [token for token in required if token not in body]
    if missing:
        return _fail(
            "flutter_authority_operations_broker_mediated",
            "CONFIG",
            "ShellCoreClient.product()にbroker-mediated authority tokenがない: " + ", ".join(missing),
            "authority-sensitiveな製品snapshot dataはbroker response由来に保つ。",
        )
    forbidden = ["ShellCoreClient.local(", "tooling/shell_snapshot.py", "python3 tooling/shell_snapshot.py"]
    findings = [token for token in forbidden if token in body]
    if findings:
        return _fail(
            "flutter_authority_operations_broker_mediated",
            "CONFIG",
            "ShellCoreClient.product()がlocal/Python authority fallback tokenをまだ参照している: " + ", ".join(findings),
            "製品authority pathからlocal/Python fallbackを除去する。",
        )
    return _pass(
        "flutter_authority_operations_broker_mediated",
        "CONFIG",
        "ShellCoreClient.product()はhealth、normalization、content projection、protected-field編集拒否、command-envelope stateをbrokerへ要求し、broker_unavailable snapshot経由でfail-closeする。",
        "新しいauthority surfaceはbroker-mediatedかつfail-closedに保つ。",
    )


def assert_flutter_product_snapshot_does_not_promote_probe_state() -> RuntimeAssertion:
    broker_snapshot = _broker_snapshot_body()
    command_probe = _function_body("_brokerCommandProbePayload")
    forbidden_snapshot_tokens = [
        "'pending_approvals_count': 1",
        "'permission_id': 'permission.broker.command_envelope'",
        "'approval_id': 'broker-projected-approval'",
        "'authority_map': [\n      {",
    ]
    findings = [token for token in forbidden_snapshot_tokens if token in broker_snapshot]
    forbidden_probe_tokens = ["'state':", "'audit_event':"]
    findings.extend(
        f"_brokerCommandProbePayloadが{token}を含む"
        for token in forbidden_probe_tokens
        if token in command_probe
    )
    if findings:
        return _fail(
            "flutter_product_snapshot_does_not_promote_probe_state",
            "CONFIG",
            "ShellCoreClient.product()がdiagnostic probe stateを製品recordへ昇格させている: "
            + "; ".join(findings),
            "broker probeはLIVE_RUNTIME evidence/setup checkのみとし、製品snapshotからsynthetic permission、approval、authority map、caller state、caller audit mappingを出力しない。",
        )
    return _pass(
        "flutter_product_snapshot_does_not_promote_probe_state",
        "CONFIG",
        "ShellCoreClient.product()はbroker projection/command probeを製品のpermission、approval、authority_map、caller state、caller audit mappingに入れない。",
        "将来の製品recordはFlutter生成probeではなく、broker所有のstate export由来に保つ。",
    )


def assert_flutter_models_do_not_default_evidence_to_passed() -> RuntimeAssertion:
    text = _read("apps/desktop_flutter/lib/models/generated_contracts.dart")
    forbidden = [
        "json['schema_check'] as String? ?? 'passed'",
        "json['release_smoke'] as String? ?? 'passed'",
        "json['release_gate_check'] as String? ?? 'passed'",
        "json['evidence_bundle'] as String? ?? 'passed'",
        "json['validate_all'] as String? ?? 'passed'",
        "json['strict_windows_release'] as String? ?? 'expected fail'",
    ]
    findings = [token for token in forbidden if token in text]
    conformance_default = re.search(
        r"json\[['\"]conformance_check_count['\"]\]\s+as\s+int\?\s+\?\?\s+(?!0\b)\d+",
        text,
    )
    if conformance_default:
        findings.append(f"0以外のconformance_check_count default: {conformance_default.group(0)}")
    if findings:
        return _fail(
            "flutter_models_do_not_default_evidence_to_passed",
            "CONFIG",
            "Dart evidence summaryのdefaultがPASS valueを合成できる: "
            + "; ".join(findings),
            "欠落したevidence summary fieldのdefaultはnot reported/0とし、release PASS valueは明示的evidence由来に保つ。",
        )
    return _pass(
        "flutter_models_do_not_default_evidence_to_passed",
        "CONFIG",
        "source JSONがevidence fieldを省略したとき、Dart evidence summaryのdefaultはPASSや固定check countを合成しない。",
        "明示的なvalidator/exportが供給するまで、欠落したevidenceは欠落として表示する。",
    )


def assert_flutter_does_not_spawn_python_product_path() -> RuntimeAssertion:
    paths = _desktop_lib_files() + [
        ROOT / "scripts" / "launch_owner_desktop.sh",
        ROOT / "scripts" / "launch_owner_desktop.ps1",
    ]
    forbidden = [
        "Process.run",
        "Process.start",
        "Process.killPid",
        "tooling/shell_snapshot.py",
        "python tooling/shell_snapshot.py",
        "python3 tooling/shell_snapshot.py",
    ]
    findings = _scan_files(paths, forbidden)
    if findings:
        return _fail(
            "flutter_product_path_does_not_spawn_python",
            "CONFIG",
            "製品Flutterまたはowner launch pathがPython/process authority tokenを含む: " + "; ".join(findings),
            "製品authority起動からPython process launchとsnapshot生成を除去する。",
        )
    return _pass(
        "flutter_product_path_does_not_spawn_python",
        "CONFIG",
        "Desktop Flutter libとowner launch scriptはPython snapshot generatorの呼出しやDart process-spawn APIを含まない。",
        "Pythonはtooling、local validation、schema validation、migration parityに限定する。",
    )


def assert_launch_scripts_start_broker_without_python_snapshot() -> RuntimeAssertion:
    scripts = [
        ROOT / "scripts" / "launch_owner_desktop.sh",
        ROOT / "scripts" / "launch_owner_desktop.ps1",
    ]
    errors: list[str] = []
    for path in scripts:
        text = path.read_text(encoding="utf-8")
        for token in ["broker-server", "GUI_SHELL_BROKER_ENDPOINT_JSON"]:
            if token not in text:
                errors.append(f"{path.relative_to(ROOT).as_posix()} に{token}がない")
        for token in ["shell_snapshot.py", "python tooling/", "python3 tooling/"]:
            if token in text:
                errors.append(f"{path.relative_to(ROOT).as_posix()} が{token}を含む")
    if errors:
        return _fail(
            "owner_launch_path_starts_broker_without_python_snapshot",
            "CONFIG",
            "; ".join(errors),
            "製品authority snapshotを生成せずにRust brokerを起動し、endpoint fileをFlutterへ渡す。",
        )
    return _pass(
        "owner_launch_path_starts_broker_without_python_snapshot",
        "CONFIG",
        "owner desktop launch scriptはPython snapshot生成を実行せず、gui_shell_rust_helper broker-serverを起動してGUI_SHELL_BROKER_ENDPOINT_JSONを渡す。",
        "broker launchはlauncher/supervisor codeに保ち、Flutter authority logicから分離する。",
    )


def assert_no_ffi_or_direct_bridge_authority_path() -> RuntimeAssertion:
    dart_forbidden = ["dart:ffi", "flutter_rust_bridge", "MethodChannel("]
    rust_forbidden = ['extern "C"', "#[no_mangle]", "cxx::bridge"]
    findings = _scan_files(_desktop_lib_files(), dart_forbidden)
    findings.extend(_scan_files(list((ROOT / "native" / "rust_helper" / "src").rglob("*.rs")), rust_forbidden))
    if findings:
        return _fail(
            "no_ffi_authority_path",
            "CONFIG",
            "authority-sensitiveなFlutter/Rust direct bridge tokenを検出した: " + "; ".join(findings),
            "authority、approval、audit、recovery、credential、command-dispatch pathには制限されたbroker IPCを使う。",
        )
    return _pass(
        "no_ffi_authority_path",
        "CONFIG",
        "Flutter/Rust authority surfaceのscanでDart FFI、flutter_rust_bridge、MethodChannel、Rust FFI export tokenは検出されなかった。",
        "authority-sensitiveなFlutter-Rust通信はindependent-process IPC上に保つ。",
    )


def assert_broker_client_uses_authenticated_loopback_ipc() -> RuntimeAssertion:
    text = _read("apps/desktop_flutter/lib/services/broker_client.dart")
    required = [
        "Socket.connect",
        "sessionSecret",
        "session_secret",
        "GUI_SHELL_BROKER_ENDPOINT_JSON",
        "127.0.0.1",
    ]
    missing = [token for token in required if token not in text]
    forbidden = ["Process.run", "Process.start", "MethodChannel(", "flutter_rust_bridge", "dart:ffi"]
    findings = [token for token in forbidden if token in text]
    if missing or findings:
        pieces = []
        if missing:
            pieces.append("欠落: " + ", ".join(missing))
        if findings:
            pieces.append("禁止token: " + ", ".join(findings))
        return _fail(
            "broker_client_uses_authenticated_loopback_ipc",
            "CONFIG",
            "; ".join(pieces),
            "BrokerClientはendpoint/session fileを用いた認証付きloopback IPC上に保つ。",
        )
    return _pass(
        "broker_client_uses_authenticated_loopback_ipc",
        "CONFIG",
        "BrokerClientはbrokerのendpoint/session fieldとSocket.connectを使い、direct bridgeやprocess-start tokenを持たない。",
        "request authenticationとendpoint discoveryはbroker testの対象に保つ。",
    )


def assert_broker_client_binds_payload_hash_to_payload() -> RuntimeAssertion:
    text = _read("apps/desktop_flutter/lib/services/broker_client.dart")
    required = ["_payloadHash(payload)", "_canonicalizeJsonValue", "_sha256Tagged"]
    missing = [token for token in required if token not in text]
    fixed_hash = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    if missing or fixed_hash in text:
        pieces = []
        if missing:
            pieces.append("欠落: " + ", ".join(missing))
        if fixed_hash in text:
            pieces.append("固定dummy payload_hashが残っている")
        return _fail(
            "broker_client_binds_payload_hash_to_payload",
            "CONFIG",
            "; ".join(pieces),
            "canonical JSON payloadからrequest payload_hashを計算し、不一致はRust brokerに拒否させる。",
        )
    return _pass(
        "broker_client_binds_payload_hash_to_payload",
        "CONFIG",
        "BrokerClientは固定dummy hashを送らず、canonical JSON payloadからpayload_hashを計算する。",
        "Rust brokerのpayload hash mismatch testを合格状態に保つ。",
    )


def assert_broker_secret_not_projected_to_ui() -> RuntimeAssertion:
    files = [
        path
        for path in _desktop_lib_files()
        if path.name != "broker_client.dart"
    ]
    findings = _scan_files(files, ["session_secret", "sessionSecret"])
    if findings:
        return _fail(
            "broker_secret_not_projected_to_ui",
            "CONFIG",
            "Broker session secret tokenがBrokerClientの外に現れる: " + "; ".join(findings),
            "broker session secretをUI model、text、audit payload、snapshotに入れない。",
        )
    return _pass(
        "broker_secret_not_projected_to_ui",
        "CONFIG",
        "Broker session secret tokenはBrokerClientに限定され、UI/snapshot file経由で射影されない。",
        "secretをdisplayとaudit payloadから分離する。",
    )


def assert_flutter_fail_closed_tests_exist() -> RuntimeAssertion:
    test_text = _read("apps/desktop_flutter/test/widget_test.dart")
    required = [
        "ブローカー利用不可時に製品クライアントが閉鎖側へ失敗する",
        "認証拒否時に製品クライアントが閉鎖側へ失敗する",
        "期限切れブローカーセッションで製品クライアントが閉鎖側へ失敗する",
        "不正なブローカー応答で製品クライアントが閉鎖側へ失敗する",
        "broker.fail_closed",
        "broker_unavailable",
    ]
    missing = [token for token in required if token not in test_text]
    if missing:
        return _fail(
            "flutter_broker_fail_closed_test_coverage",
            "FIXTURE",
            "Flutter fail-closed test tokenがない: " + ", ".join(missing),
            "broker利用不可、認証失敗、stale session、malformed response、拒否されたauthority actionのFlutter testを追加する。",
        )
    return _pass(
        "flutter_broker_fail_closed_test_coverage",
        "FIXTURE",
        "Flutter testはbroker利用不可、認証拒否、stale session、malformed response、broker.fail_closed setup diagnosticを対象にする。",
        "release candidateでfail-closed UI testを合格状態に保つ。",
    )


def assert_broker_runtime_restart_and_crash_tests_exist() -> RuntimeAssertion:
    ipc = _read("native/rust_helper/tests/broker_ipc.rs")
    protocol = _read("native/rust_helper/src/broker/protocol.rs")
    required = [
        "broker_process_launch_connect_and_shutdown",
        "try_send_raw",
        "broker_ipc_rejects_replay_after_process_restart",
        "persistent_store_rejects_replayed_nonce_after_restart",
        "persistent_store_verifies_audit_chain_after_restart",
        "persistent_store_rejects_tampered_audit_chain",
        "persistent_store_rejects_malformed_replay_state",
    ]
    combined = ipc + "\n" + protocol
    missing = [token for token in required if token not in combined]
    if missing:
        return _fail(
            "broker_runtime_restart_persistence_and_shutdown_coverage",
            "FIXTURE",
            "Broker runtimeのpersistence/shutdown test tokenがない: " + ", ".join(missing),
            "broker launch/connect/shutdown、再起動後replay拒否、audit chainの再起動時検証、永続state改ざん拒否を対象にする。",
        )
    return _pass(
        "broker_runtime_restart_persistence_and_shutdown_coverage",
        "FIXTURE",
        "Rust broker test fixtureはlocal broker processのlaunch/connect/shutdown、shutdown後の利用不可挙動、再起動後のreplay拒否とaudit chain検証、改ざん/不正形式の永続state拒否を対象にする。これはinstalled product runtime proofではない。",
        "Rust broker IPCとpersistence testを合格状態に保つ。Windows installed-path evidenceは別のrelease gateのままである。",
    )


def run_release_runtime_assertions() -> list[RuntimeAssertion]:
    return [
        assert_flutter_product_entry_uses_broker(),
        assert_flutter_authority_operations_are_broker_mediated(),
        assert_flutter_product_snapshot_does_not_promote_probe_state(),
        assert_flutter_models_do_not_default_evidence_to_passed(),
        assert_flutter_does_not_spawn_python_product_path(),
        assert_launch_scripts_start_broker_without_python_snapshot(),
        assert_no_ffi_or_direct_bridge_authority_path(),
        assert_broker_client_uses_authenticated_loopback_ipc(),
        assert_broker_client_binds_payload_hash_to_payload(),
        assert_broker_secret_not_projected_to_ui(),
        assert_flutter_fail_closed_tests_exist(),
        assert_broker_runtime_restart_and_crash_tests_exist(),
    ]


def build_report() -> dict:
    assertions = run_release_runtime_assertions()
    failures = [item for item in assertions if item.classification == "release_blocker"]
    return {
        "ok": not failures,
        "assertions": [item.__dict__ for item in assertions],
        "failure_count": len(failures),
        "evidence_scope": sorted({item.evidence_scope for item in assertions}),
        "release_gate_note": {
            "classification": "release_blocker",
            "blocks_release": "yes",
            "reason": "完成製品releaseの前にWindows installed-pathのno-Python-runtime evidenceとbroker-mediated LIVE_RUNTIME evidenceが引き続き必要である。",
            "required_action": "Windows installed-pathでbrokerのlaunch/connect/restart/crash/no-Python-runtime smokeを実行し、strict Windows release validationに合格させる。",
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    report = build_report()
    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
    elif args.check:
        print(
            "release runtime assertion結果: "
            f"合格 {len(report['assertions']) - report['failure_count']} 件、"
            f"失敗 {report['failure_count']} 件、"
            f"evidence_scope={','.join(report['evidence_scope'])}"
        )
        for item in report["assertions"]:
            if item["classification"] == "release_blocker":
                print(f"  - {item['name']}: {item['reason']}")
    else:
        print(json.dumps(report, indent=2, sort_keys=True))
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
