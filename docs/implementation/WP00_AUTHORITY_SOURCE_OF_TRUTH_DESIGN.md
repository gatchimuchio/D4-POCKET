# WP00 Authority Source-of-Truth Design Report

Date: 2026-06-05
Scope: D-P0-00 / D-P0-01 and prerequisite slices of D-P0-07 / D-P0-08.

This report is the required pre-implementation design record for Work Package 00.

## Affected Symbols

Rust production path:

- `native/rust_helper/src/broker/protocol.rs`
  - `BrokerOperation`
  - `Broker::handle`
  - `Broker::suspend_command`
  - `Broker::accept_body`
  - `json_command_eligibility`
  - `BrokerHealth.authority_cutover_status`
  - `BrokerHealth.command_dispatch_enabled`
- `native/rust_helper/src/broker/authority.rs`
  - current fixture-style `evaluate_authority`
  - authority source rejection logic
  - audit mapping requirement logic
- `native/rust_helper/tests/broker_ipc.rs`
  - IPC negative tests for production authority requests.

Python fixture / parity path:

- `tooling/broker_parity/run_authority_parity.py`
  - current calls to `authority_evaluate` with `{"state": ..., "action": ...}`
  - command-envelope expectations.
- `packages/shell_core/policy_evaluator.py`
  - retained as Python fixture/oracle evaluator for conformance and parity.
- `packages/shell_core/runtime_state.py`
  - retained as fixture/runtime-state object, not production broker authority state.
- `packages/shell_core/runtime_registry.py`
- `packages/shell_core/permission_ledger.py`
- `packages/shell_core/approval_queue.py`
- `packages/shell_core/audit_store.py`
- `packages/shell_core/recovery_catalog.py`
- `packages/shell_core/update_policy_store.py`

Dart product path:

- `apps/desktop_flutter/lib/services/broker_client.dart`
  - no contract change required for generic broker requests.
- `apps/desktop_flutter/lib/services/shell_core_client.dart`
  - `_brokerCommandProbePayload` currently sends caller fixture `state` and caller `audit_event`.
- `apps/desktop_flutter/test/widget_test.dart`
  - fake command response may need eligibility expectation updates only.

Schemas and protocol documents:

- `specs/ipc_request.schema.json`
  - add fixture-only `authority_fixture_evaluate`.
- `specs/broker_error.schema.json`
  - add production authority contract rejection code.
- `docs/architecture/RUST_BROKER_IPC_PROTOCOL.md`
  - distinguish production authority evaluation from fixture parity evaluation.
- `GUI_Shell_Product_Quality_Integrated_Correction_Ledger_v4_2026-06-05.md`
  - canonical ledger already added for this work package.

## Reused Components

The existing Python components are retained as development/fixture components:

- `RuntimeState` continues to support conformance fixtures and Python oracle tests.
- `PolicyEvaluator(RuntimeState)` remains the fixture evaluator used by `authority_fixture_evaluate`.
- `RuntimeRegistry`, `PermissionLedger`, `ApprovalQueue`, `AuditStore`, `RecoveryCatalog`, and `UpdatePolicyStore` are small broker-owned-state candidates, but they are Python components and are not directly imported into the Rust broker production process.

For WP00, the Rust broker gets a minimal internal authority registry model in the Rust authority module. It is intentionally conservative:

- it is broker-owned, not caller supplied;
- it is closed over a small broker command-dispatch record set;
- it keeps permission denied while command dispatch is suspended;
- it emits decision results from broker-owned state only.

Full persisted registry migration, richer relation closure, canonical ActionEnvelope, and cross-language canonical payload hash are deferred to WP01.

## Production Authority Input Contract

Production `authority_evaluate` accepts an action request only:

```json
{
  "action": {
    "operation": "command_envelope.dispatch",
    "runtime_id": "gui_shell_rust_broker",
    "capability_id": "command_envelope.dispatch",
    "permission_id": "permission.broker.command_envelope",
    "approval_id": "broker-projected-approval",
    "recovery_action": {"recovery_id": "recover-command-dispatch"},
    "adapter_metadata": {"client": "desktop_flutter"}
  }
}
```

Production `authority_evaluate` rejects payloads containing `state`. It also rejects caller-declared authority provenance fields and caller-submitted `audit_event` mappings. Broker audit emission is the only production audit source for the decision.

`command_envelope` remains suspended. Its eligibility section is derived from the same broker-owned authority state and no longer treats caller fixture state as proof.

## Fixture Isolation

The existing state/action parity evaluator is moved behind a separate operation:

```text
authority_fixture_evaluate
```

This operation is for Python-oracle parity and conformance only. It keeps the existing `{"state": ..., "action": ...}` payload shape so existing fixture coverage remains useful, but production UI and command-envelope code must not use it.

## Internal Provenance / Issuer Rules

Production authority provenance is internal to the broker:

- issuer: `gui-shell-rust-broker`
- source: `rust_security_broker`
- evidence source: `INTERNAL_STATE` for denied/suspended authority decisions until cutover
- caller-provided `authority_source` is not an authority source, even if it names the broker.
- unknown, forged, or denylisted caller authority source values deny the decision.

## Broker-Emitted Audit Sequencing

For production authority evaluation:

1. IPC envelope validation, freshness, session, replay, metadata authority stripping, and persistence checks run first.
2. Replay nonce is recorded before operation handling.
3. Broker-owned authority state evaluates the action request.
4. The broker appends one audit event for the decision.
5. If audit append fails, the broker returns suspended `broker_audit_append_failed`.
6. The response body includes the authority decision; top-level transport status may be `accepted` when the request was processed, but the broker audit event decision is `authorized`, `denied`, or `suspended`.

Caller-supplied audit event mappings are never accepted as authority prerequisites in the production path.

## Migration Impact And WP01 Deferrals

WP00 intentionally does not activate command dispatch and does not make authority cutover active.

Deferred to WP01:

- canonical `ActionEnvelope` schema;
- full runtime/capability/operation/permission/approval/recovery/target-scope relation closure;
- required `permission.runtime_id` migration;
- persisted broker-owned registries;
- canonical payload hash binding across Dart/Rust/Python;
- normalization key collision rejection;
- adapter ingress single-route closure;
- `SensitiveActionRouter` production fail-closed redesign.
