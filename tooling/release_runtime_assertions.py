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
                findings.append(f"{path.relative_to(ROOT).as_posix()} contains {token}")
    return findings


def _desktop_lib_files() -> list[Path]:
    return list((DESKTOP_FLUTTER / "lib").rglob("*.dart"))


def assert_flutter_product_entry_uses_broker() -> RuntimeAssertion:
    main = _read("apps/desktop_flutter/lib/main.dart")
    if "await ShellCoreClient.product()" not in main:
        return _fail(
            "flutter_product_entry_uses_broker",
            "CONFIG",
            "apps/desktop_flutter/lib/main.dart does not call ShellCoreClient.product() in the product entry path.",
            "Route product startup through ShellCoreClient.product() before release validation.",
        )
    if "ShellCoreClient.local(" in main:
        return _fail(
            "flutter_product_entry_uses_broker",
            "CONFIG",
            "apps/desktop_flutter/lib/main.dart still references ShellCoreClient.local() in product startup code.",
            "Remove local snapshot authority use from product startup.",
        )
    return _pass(
        "flutter_product_entry_uses_broker",
        "CONFIG",
        "Product main.dart initializes ShellCoreClient.product() and does not reference ShellCoreClient.local().",
        "Keep product startup on the broker-mediated client.",
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
            "ShellCoreClient.product() is missing broker-mediated authority tokens: " + ", ".join(missing),
            "Keep authority-sensitive product snapshot data sourced from broker responses.",
        )
    forbidden = ["ShellCoreClient.local(", "tooling/shell_snapshot.py", "python3 tooling/shell_snapshot.py"]
    findings = [token for token in forbidden if token in body]
    if findings:
        return _fail(
            "flutter_authority_operations_broker_mediated",
            "CONFIG",
            "ShellCoreClient.product() still references local/Python authority fallback tokens: " + ", ".join(findings),
            "Remove local/Python fallback from the product authority path.",
        )
    return _pass(
        "flutter_authority_operations_broker_mediated",
        "CONFIG",
        "ShellCoreClient.product() requests health, normalization, content projection, protected-field edit rejection, and command-envelope state from the broker and fail-closes through a broker_unavailable snapshot.",
        "Keep new authority surfaces broker-mediated and fail-closed.",
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
        f"_brokerCommandProbePayload contains {token}"
        for token in forbidden_probe_tokens
        if token in command_probe
    )
    if findings:
        return _fail(
            "flutter_product_snapshot_does_not_promote_probe_state",
            "CONFIG",
            "ShellCoreClient.product() promotes diagnostic probe state into product records: "
            + "; ".join(findings),
            "Keep broker probes as LIVE_RUNTIME evidence/setup checks only; do not emit synthetic permissions, approvals, authority maps, caller state, or caller audit mappings from the product snapshot.",
        )
    return _pass(
        "flutter_product_snapshot_does_not_promote_probe_state",
        "CONFIG",
        "ShellCoreClient.product() keeps broker projection/command probes out of product permissions, approvals, authority_map, caller state, and caller audit mappings.",
        "Keep future product records sourced from broker-owned state exports, not Flutter-created probes.",
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
        findings.append(f"nonzero conformance_check_count default: {conformance_default.group(0)}")
    if findings:
        return _fail(
            "flutter_models_do_not_default_evidence_to_passed",
            "CONFIG",
            "Dart evidence summary defaults can synthesize PASS values: "
            + "; ".join(findings),
            "Default missing evidence summary fields to not reported/0 and keep release PASS values sourced from explicit evidence.",
        )
    return _pass(
        "flutter_models_do_not_default_evidence_to_passed",
        "CONFIG",
        "Dart evidence summary defaults do not synthesize PASS or fixed check counts when source JSON omits evidence fields.",
        "Keep missing evidence visibly missing until an explicit validator/export supplies it.",
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
            "Product Flutter or owner launch path contains Python/process authority tokens: " + "; ".join(findings),
            "Remove Python process launch and snapshot generation from product authority startup.",
        )
    return _pass(
        "flutter_product_path_does_not_spawn_python",
        "CONFIG",
        "Desktop Flutter lib and owner launch scripts contain no Python snapshot generator invocation or Dart process-spawn API.",
        "Keep Python limited to tooling, CI, schema validation, and migration parity.",
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
                errors.append(f"{path.relative_to(ROOT).as_posix()} missing {token}")
        for token in ["shell_snapshot.py", "python tooling/", "python3 tooling/"]:
            if token in text:
                errors.append(f"{path.relative_to(ROOT).as_posix()} contains {token}")
    if errors:
        return _fail(
            "owner_launch_path_starts_broker_without_python_snapshot",
            "CONFIG",
            "; ".join(errors),
            "Start the Rust broker and pass its endpoint file to Flutter without generating product authority snapshots.",
        )
    return _pass(
        "owner_launch_path_starts_broker_without_python_snapshot",
        "CONFIG",
        "Owner desktop launch scripts start gui_shell_rust_helper broker-server and pass GUI_SHELL_BROKER_ENDPOINT_JSON without running Python snapshot generation.",
        "Keep broker launch in launcher/supervisor code and out of Flutter authority logic.",
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
            "Authority-sensitive Flutter/Rust direct bridge token found: " + "; ".join(findings),
            "Use restricted broker IPC for authority, approval, audit, recovery, credential, and command-dispatch paths.",
        )
    return _pass(
        "no_ffi_authority_path",
        "CONFIG",
        "No Dart FFI, flutter_rust_bridge, MethodChannel, or Rust FFI export token was found in the Flutter/Rust authority surface scan.",
        "Keep authority-sensitive Flutter-Rust communication on independent-process IPC.",
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
            pieces.append("missing " + ", ".join(missing))
        if findings:
            pieces.append("forbidden " + ", ".join(findings))
        return _fail(
            "broker_client_uses_authenticated_loopback_ipc",
            "CONFIG",
            "; ".join(pieces),
            "Keep BrokerClient on authenticated loopback IPC with an endpoint/session file.",
        )
    return _pass(
        "broker_client_uses_authenticated_loopback_ipc",
        "CONFIG",
        "BrokerClient uses Socket.connect with broker endpoint/session fields and has no direct bridge or process-start token.",
        "Keep request authentication and endpoint discovery covered by broker tests.",
    )


def assert_broker_client_binds_payload_hash_to_payload() -> RuntimeAssertion:
    text = _read("apps/desktop_flutter/lib/services/broker_client.dart")
    required = ["_payloadHash(payload)", "_canonicalizeJsonValue", "_sha256Tagged"]
    missing = [token for token in required if token not in text]
    fixed_hash = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    if missing or fixed_hash in text:
        pieces = []
        if missing:
            pieces.append("missing " + ", ".join(missing))
        if fixed_hash in text:
            pieces.append("fixed dummy payload_hash remains")
        return _fail(
            "broker_client_binds_payload_hash_to_payload",
            "CONFIG",
            "; ".join(pieces),
            "Compute request payload_hash from canonical JSON payload and let Rust broker reject mismatches.",
        )
    return _pass(
        "broker_client_binds_payload_hash_to_payload",
        "CONFIG",
        "BrokerClient computes payload_hash from canonical JSON payload instead of sending a fixed dummy hash.",
        "Keep Rust broker payload hash mismatch tests passing.",
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
            "Broker session secret token appears outside BrokerClient: " + "; ".join(findings),
            "Keep broker session secrets out of UI models, text, audit payloads, and snapshots.",
        )
    return _pass(
        "broker_secret_not_projected_to_ui",
        "CONFIG",
        "Broker session secret tokens are confined to BrokerClient and are not projected through UI/snapshot files.",
        "Keep secrets out of display and audit payloads.",
    )


def assert_flutter_fail_closed_tests_exist() -> RuntimeAssertion:
    test_text = _read("apps/desktop_flutter/test/widget_test.dart")
    required = [
        "product client fails closed when broker is unavailable",
        "product client fails closed on authentication rejection",
        "product client fails closed on stale broker session",
        "product client fails closed on malformed broker response",
        "broker.fail_closed",
        "broker_unavailable",
    ]
    missing = [token for token in required if token not in test_text]
    if missing:
        return _fail(
            "flutter_broker_fail_closed_test_coverage",
            "FIXTURE",
            "Flutter fail-closed test tokens missing: " + ", ".join(missing),
            "Add Flutter tests for broker unavailable, auth failure, stale session, malformed response, and blocked authority actions.",
        )
    return _pass(
        "flutter_broker_fail_closed_test_coverage",
        "FIXTURE",
        "Flutter tests cover broker unavailable, authentication rejection, stale session, malformed response, and broker.fail_closed setup diagnostics.",
        "Keep fail-closed UI tests passing on release candidates.",
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
            "Broker runtime persistence/shutdown test tokens missing: " + ", ".join(missing),
            "Cover broker launch/connect/shutdown, restart replay rejection, audit chain restart verification, and persisted-state tamper rejection.",
        )
    return _pass(
        "broker_runtime_restart_persistence_and_shutdown_coverage",
        "FIXTURE",
        "Rust broker test fixtures contain local broker process launch/connect/shutdown, unavailable-after-shutdown behavior, replay rejection after restart, audit chain verification after restart, and tampered/malformed persisted-state rejection coverage. This is not installed product runtime proof.",
        "Keep Rust broker IPC and persistence tests passing; Windows installed-path evidence remains a separate release gate.",
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
            "reason": "Windows installed-path no-Python-runtime and broker-mediated LIVE_RUNTIME evidence is still required before completed product release.",
            "required_action": "Run Windows installed-path broker launch/connect/restart/crash/no-Python-runtime smoke and pass strict Windows release validation.",
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
            "release runtime assertions: "
            f"{len(report['assertions']) - report['failure_count']} passed, "
            f"{report['failure_count']} failed, "
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
