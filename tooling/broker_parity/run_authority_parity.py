from __future__ import annotations

import json
import hashlib
import os
import shutil
import socket
import subprocess
import sys
import tempfile
import time
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

DEFAULT_BROKER_START_TIMEOUT_SECONDS = 120.0

from packages.shell_core.approval_queue import ApprovalQueue, canonical_hash
from packages.shell_core.audit_chain import chain_event, verify_audit_chain
from packages.shell_core.content_exposure import project_approval_content
from packages.shell_core.normalization import normalize_inbound_payload
from packages.shell_core.permission_ledger import NON_AUTHORITY_SOURCES
from packages.shell_core.policy_evaluator import PolicyEvaluator
from packages.shell_core.runtime_state import RuntimeState


def ipc_payload_hash(payload) -> str:
    encoded = json.dumps(payload, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    return "sha256:" + hashlib.sha256(encoded).hexdigest()


def broker_start_timeout_seconds() -> float:
    raw = os.environ.get("GUI_SHELL_BROKER_START_TIMEOUT_SECONDS")
    if raw is None:
        return DEFAULT_BROKER_START_TIMEOUT_SECONDS
    try:
        value = float(raw)
    except ValueError:
        return DEFAULT_BROKER_START_TIMEOUT_SECONDS
    return max(1.0, value)


def wait_for_process_exit(process: subprocess.Popen, timeout: float = 5.0) -> None:
    try:
        process.wait(timeout=timeout)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait(timeout=timeout)


def cleanup_temp_directory(path: Path, attempts: int = 25, delay_seconds: float = 0.2) -> None:
    last_error: OSError | None = None
    for _ in range(attempts):
        try:
            shutil.rmtree(path)
            return
        except FileNotFoundError:
            return
        except OSError as exc:
            last_error = exc
            time.sleep(delay_seconds)
    raise AssertionError(f"temporary broker parity directory cleanup failed: {path}: {last_error}")


def build_state() -> tuple[RuntimeState, dict]:
    state = RuntimeState()
    state.register_runtime(
        {
            "runtime_id": "runtime-1",
            "name": "Runtime 1",
            "kind": "local_service",
            "status": "ready",
            "adapter_id": "broker-parity-adapter",
        }
    )
    state.register_capability(
        {
            "capability_id": "filesystem.write",
            "runtime_id": "runtime-1",
            "name": "Filesystem write",
            "risk_level": "high",
            "default_permission": "ask",
            "requires_approval": True,
            "operations": ["filesystem.write"],
        }
    )
    state.record_permission(
        {
            "permission_id": "permission-allow",
            "runtime_id": "runtime-1",
            "capability_id": "filesystem.write",
            "operation": "filesystem.write",
            "target_scope": "workspace",
            "scope": "target",
            "decision": "allow",
            "source": "policy",
        }
    )
    state.record_permission(
        {
            "permission_id": "permission-deny",
            "runtime_id": "runtime-1",
            "capability_id": "filesystem.write",
            "operation": "filesystem.write",
            "target_scope": "workspace",
            "scope": "target",
            "decision": "deny",
            "source": "policy",
        }
    )
    approved = {
        "approval_id": "approval-approved",
        "runtime_id": "runtime-1",
        "status": "approved",
        "operation": "filesystem.write",
        "target_scope": "workspace",
        "content_visibility": "redacted",
        "payload_hash": canonical_hash({"path": "notes/today.md", "content": "hello"}),
        "full_payload": {"path": "notes/today.md", "content": "hello"},
        "redacted_payload": {"path": "notes/today.md", "content": "[redacted]"},
        "editable_fields": ["path"],
        "authority_fields": ["permission_id"],
    }
    pending = {**approved, "approval_id": "approval-pending", "status": "pending"}
    state.enqueue_approval(approved)
    state.enqueue_approval(pending)
    state.append_audit_event(
        {
            "event_id": "audit-1",
            "timestamp": "2026-06-05T00:00:00Z",
            "actor": "shell",
            "action": "filesystem.write",
            "target": "runtime-1",
            "result": "success",
            "payload_hash": canonical_hash({"path": "notes/today.md", "content": "hello"}),
        }
    )
    state.register_recovery_action(
        {
            "recovery_id": "recover-1",
            "runtime_id": "runtime-1",
            "operation": "filesystem.write",
            "class": "permission_denied",
            "severity": "warning",
            "user_visible_message": "Permission required.",
            "safe_to_retry": True,
        }
    )
    state_json = {
        "runtimes": list(state.runtimes.values()),
        "capabilities": list(state.capabilities.values()),
        "permissions": list(state.permissions.values()),
        "approvals": list(state.approvals.values()),
        "audit_events": list(state.audit_events.values()),
        "recovery_actions": list(state.recovery_actions.values()),
    }
    return state, state_json


def base_action() -> dict:
    return {
        "operation": "filesystem.write",
        "runtime_id": "runtime-1",
        "capability_id": "filesystem.write",
        "permission_id": "permission-allow",
        "approval_id": "approval-approved",
        "target_scope": "workspace",
        "payload": {"path": "notes/today.md", "content": "hello"},
        "audit_event": {
            "event_id": "audit-1",
            "payload_hash": canonical_hash({"path": "notes/today.md", "content": "hello"}),
        },
        "recovery_action": {"recovery_id": "recover-1"},
        "adapter_metadata": {"label": "safe"},
    }


class BrokerClient:
    def __init__(self, process: subprocess.Popen, endpoint: dict):
        self.process = process
        self.endpoint = endpoint
        self.counter = 0

    def call(self, operation: str, payload) -> dict:
        response = self.request(operation, payload)
        if response["status"] != "accepted":
            raise AssertionError(f"broker rejected {operation}: {response}")
        return response["body"]

    def request(self, operation: str, payload=None) -> dict:
        self.counter += 1
        request = {
            "request_id": f"parity-{self.counter}",
            "session_id": self.endpoint["session_id"],
            "operation": operation,
            "payload_hash": ipc_payload_hash(payload),
            "nonce": f"parity-nonce-{self.counter}",
            "issued_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
            "metadata": {"client": "broker_parity"},
        }
        if payload is not None:
            request["payload"] = payload
        with socket.create_connection((self.endpoint["host"], self.endpoint["port"]), timeout=5) as sock:
            sock.sendall(self.endpoint["session_secret"].encode("utf-8") + b"\n")
            sock.sendall(json.dumps(request, separators=(",", ":")).encode("utf-8") + b"\n")
            sock.shutdown(socket.SHUT_WR)
            raw = sock.makefile("r", encoding="utf-8").readline()
        return json.loads(raw)

    def shutdown(self) -> None:
        self.counter += 1
        request = {
            "request_id": f"parity-shutdown-{self.counter}",
            "session_id": self.endpoint["session_id"],
            "operation": "shutdown",
            "payload_hash": ipc_payload_hash(None),
            "nonce": f"parity-shutdown-nonce-{self.counter}",
            "issued_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
            "metadata": {"client": "broker_parity"},
        }
        with socket.create_connection((self.endpoint["host"], self.endpoint["port"]), timeout=5) as sock:
            sock.sendall(self.endpoint["session_secret"].encode("utf-8") + b"\n")
            sock.sendall(json.dumps(request, separators=(",", ":")).encode("utf-8") + b"\n")


def start_broker(workspace: Path) -> BrokerClient:
    store_dir = workspace / "store"
    session_file = workspace / "broker_session.json"
    stderr_path = workspace / "broker.stderr"
    stderr = stderr_path.open("wb")
    process = subprocess.Popen(
        [
            "cargo",
            "run",
            "--quiet",
            "--",
            "broker-server",
            "--store-dir",
            str(store_dir),
            "--session-file",
            str(session_file),
        ],
        cwd=ROOT / "native" / "rust_helper",
        stdout=subprocess.DEVNULL,
        stderr=stderr,
    )
    deadline = time.monotonic() + broker_start_timeout_seconds()
    while time.monotonic() < deadline:
        if session_file.exists():
            stderr.close()
            endpoint = json.loads(session_file.read_text(encoding="utf-8"))
            return BrokerClient(process, endpoint)
        if process.poll() is not None:
            stderr.close()
            detail = stderr_path.read_text(encoding="utf-8", errors="replace")
            raise AssertionError(f"broker process exited early: {process.returncode}: {detail}")
        time.sleep(0.05)
    stderr.close()
    wait_for_process_exit(process)
    detail = stderr_path.read_text(encoding="utf-8", errors="replace")
    raise AssertionError(
        "broker session file was not created within "
        f"{broker_start_timeout_seconds():.1f}s: {detail}"
    )


def error_codes(result: dict) -> list[str]:
    return [error["code"] for error in result["errors"]]


def compare_policy(state: RuntimeState, state_json: dict, broker: BrokerClient) -> list[str]:
    errors: list[str] = []
    cases = {
        "accepted": base_action(),
        "permission_denied": {**base_action(), "permission_id": "permission-deny"},
        "approval_pending": {**base_action(), "approval_id": "approval-pending"},
        "metadata_escalation": {**base_action(), "adapter_metadata": {"trustLevel": "root"}},
    }
    for name, action in cases.items():
        python_result = PolicyEvaluator(state).evaluate(action)
        rust_result = broker.call("authority_fixture_evaluate", {"state": state_json, "action": action})
        if python_result["allowed"] != rust_result["allowed"]:
            errors.append(f"{name}: allowed mismatch")
        if error_codes(python_result) != error_codes(rust_result):
            errors.append(f"{name}: error code mismatch {error_codes(python_result)} != {error_codes(rust_result)}")
    for source in sorted(NON_AUTHORITY_SOURCES):
        action = {**base_action(), "authority_source": source}
        python_result = PolicyEvaluator(state).evaluate(action)
        rust_result = broker.call("authority_fixture_evaluate", {"state": state_json, "action": action})
        if python_result["allowed"] != rust_result["allowed"]:
            errors.append(f"non_authority_source {source}: allowed mismatch")
        if error_codes(python_result) != error_codes(rust_result):
            errors.append(
                f"non_authority_source {source}: error code mismatch {error_codes(python_result)} != {error_codes(rust_result)}"
            )
        if "non_authority_source_attempt" not in error_codes(rust_result):
            errors.append(f"non_authority_source {source}: rust did not reject source")
    command_response = broker.request("command_envelope", {"state": state_json, "action": cases["accepted"]})
    if command_response["status"] != "suspended":
        errors.append("command_envelope: dispatch was not suspended")
    command_eligibility = command_response["body"]["eligibility"]
    if command_eligibility["allowed"] is not False:
        errors.append("command_envelope: caller fixture state authorized command eligibility")
    if "caller_state_rejected" not in error_codes(command_eligibility):
        errors.append("command_envelope: caller fixture state was not rejected by production eligibility")
    if command_response["body"]["dispatch_enabled"] is not False:
        errors.append("command_envelope: dispatch_enabled was not false")
    gate = command_response["body"]["execution_gate"]
    if gate["dispatch"] != "suspended" or gate["status"] != "suspended":
        errors.append("command_envelope: dispatch gate was not suspended")
    gated_operations = {
        "process": "process.spawn",
        "credential": "credential.read",
        "update": "update.apply",
    }
    for target, operation in gated_operations.items():
        gated_action = {**base_action(), "operation": operation, "capability_id": operation}
        response = broker.request("command_envelope", {"state": state_json, "action": gated_action})
        if response["status"] != "suspended":
            errors.append(f"command_envelope {target}: response was not suspended")
        gated = response["body"]["execution_gate"]
        if gated["target_kind"] != target:
            errors.append(f"command_envelope {target}: target_kind mismatch {gated['target_kind']} != {target}")
        if gated[target] != "suspended":
            errors.append(f"command_envelope {target}: target gate was not suspended")
        if response["body"]["dispatch_enabled"] is not False:
            errors.append(f"command_envelope {target}: dispatch_enabled was not false")
    return errors


def compare_normalization(broker: BrokerClient) -> list[str]:
    errors: list[str] = []
    cases = [
        {"safeLabel": "operator", "nested": {"path": "notes/today.md"}},
        {"trust\u200bLevel": "root", "nested": {"permissionGrant": "all"}},
        {"safeLabel": "first", "safe_label": "second"},
    ]
    for index, payload in enumerate(cases):
        python_result = normalize_inbound_payload(payload)
        rust_result = broker.call("normalize_payload", payload)
        for key in ["quarantined", "stripped_payload"]:
            if python_result[key] != rust_result[key]:
                errors.append(f"normalization {index}: {key} mismatch")
        if len(python_result["authority_key_findings"]) != len(rust_result["authority_key_findings"]):
            errors.append(f"normalization {index}: authority key count mismatch")
        if len(python_result["authority_value_findings"]) != len(rust_result["authority_value_findings"]):
            errors.append(f"normalization {index}: authority value count mismatch")
        if len(python_result["normalization_collision_findings"]) != len(rust_result["normalization_collision_findings"]):
            errors.append(f"normalization {index}: collision count mismatch")
    return errors


def compare_approval_edit_and_projection(broker: BrokerClient) -> list[str]:
    errors: list[str] = []
    approval = {
        "approval_id": "approval-1",
        "runtime_id": "runtime-1",
        "permission_id": "permission-1",
        "status": "pending",
        "content_visibility": "redacted",
        "payload_hash": canonical_hash({"path": "notes/today.md", "content": "hello"}),
        "summary": "Write a note",
        "redacted_payload": {"path": "notes/today.md", "content": "[redacted]"},
        "full_payload": {"path": "notes/today.md", "content": "hello"},
        "editable_fields": ["path", "payload_hash"],
        "authority_fields": ["permission_id"],
    }
    queue = ApprovalQueue()
    queue.enqueue(approval)
    python_edit = queue.edit("approval-1", "path", "notes/tomorrow.md")
    rust_edit = broker.call("approval_edit", {"approval": approval, "field": "path", "value": "notes/tomorrow.md"})
    if rust_edit.get("ok") is not True or rust_edit["approval"]["payload_hash"] != python_edit["payload_hash"]:
        errors.append("approval_edit: allowed edit hash mismatch")
    protected = broker.call("approval_edit", {"approval": approval, "field": "payload_hash", "value": "sha256:" + "0" * 64})
    if protected.get("ok") is not False:
        errors.append("approval_edit: protected payload_hash edit was not rejected")

    for visibility in ["none", "hash_only", "summary", "redacted", "full"]:
        projected_approval = {**approval, "content_visibility": visibility}
        python_projection = project_approval_content(projected_approval)
        rust_projection = broker.call("content_projection", projected_approval)
        if python_projection != rust_projection:
            errors.append(f"content_projection {visibility}: mismatch")
    return errors


def compare_audit_chain(broker: BrokerClient) -> list[str]:
    event = {
        "event_id": "audit-1",
        "timestamp": "2026-06-03T00:00:00Z",
        "actor": "shell",
        "action": "approval.requested",
        "target": "approval-1",
        "result": "success",
        "payload_hash": canonical_hash({"approval_id": "approval-1"}),
    }
    first = chain_event(event, None)
    second = chain_event({**event, "event_id": "audit-2", "result": "denied"}, first["event_hash"])
    events = [first, second]
    python_result = verify_audit_chain(events)
    rust_result = broker.call("audit_verify", events)
    errors = []
    for key in ["ok", "event_count", "latest_event_hash", "errors"]:
        if python_result[key] != rust_result[key]:
            errors.append(f"audit_verify: {key} mismatch")
    tampered = [{**first, "result": "failed"}, second]
    if broker.call("audit_verify", tampered)["ok"] is not False:
        errors.append("audit_verify: tampered event was not rejected")
    duplicate = chain_event({**event, "target": "runtime"}, first["event_hash"])
    duplicate_events = [first, duplicate]
    python_duplicate = verify_audit_chain(duplicate_events)
    rust_duplicate = broker.call("audit_verify", duplicate_events)
    if python_duplicate != rust_duplicate:
        errors.append("audit_verify: duplicate event_id mismatch")
    return errors


def main() -> int:
    state, state_json = build_state()
    directory = Path(tempfile.mkdtemp(prefix="gui-shell-broker-parity-"))
    try:
        broker = start_broker(Path(directory))
        try:
            errors = []
            errors.extend(compare_normalization(broker))
            errors.extend(compare_policy(state, state_json, broker))
            errors.extend(compare_approval_edit_and_projection(broker))
            errors.extend(compare_audit_chain(broker))
        finally:
            try:
                broker.shutdown()
            finally:
                wait_for_process_exit(broker.process)
        if errors:
            print("broker authority parity failed:")
            for error in errors:
                print(f"  - {error}")
            return 1
    finally:
        cleanup_temp_directory(directory)
    print("broker authority parity passed: accepted parity, rejected parity, rust-specific broker IPC path")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
