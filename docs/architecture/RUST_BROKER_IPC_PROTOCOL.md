# Rust Broker IPC Protocol

Status: Phase 2 / Phase 3 initial protocol skeleton
Date: 2026-06-01
Scope: Rust Security Broker envelope, response, session, health, and audit skeleton

## 1. Protocol Owner

Rust Security Broker owns authority-sensitive IPC acceptance and rejection. Flutter may render request state and collect operator input, but must not decide authority, approval validity, audit finality, recovery eligibility, credential access, or external command dispatch.

Authority-sensitive Flutter-Rust接続は独立 broker process + restricted IPC を原則とし、この protocol は FFI/direct bridge による authority path を定義しない。

## 2. Contract Files

Initial broker contracts:

- `specs/ipc_request.schema.json`
- `specs/ipc_response.schema.json`
- `specs/broker_error.schema.json`
- `specs/broker_session.schema.json`
- `specs/broker_health.schema.json`
- `specs/broker_command_envelope.schema.json`

Examples and negative fixtures live under `examples/contracts/`.

## 3. Request Envelope

Required fields:

- `request_id`
- `operation`
- `payload_hash`
- `nonce`
- `issued_at`
- `metadata`

`session_id` is optional for `health` during early startup. All authority-sensitive operations require a current broker session once session lifecycle is active.

Allowed operations:

- `health`
- `shutdown`
- `command_envelope`

`command_envelope` is intentionally suspended in the skeleton. It must not dispatch real external commands until authority migration tests exist.

## 4. Response Envelope

Required fields:

- `request_id`
- `operation`
- `status`
- `evidence_source`
- `audit_event_id`
- `error`

Allowed status values:

- `accepted`
- `rejected`
- `suspended`

Every rejected or suspended response requires an audit event. Accepted health and shutdown responses are also audited to preserve broker-local append-only evidence.

## 5. Broker Error

Broker errors are structured and fail closed.

Allowed error codes:

- `broker_request_malformed`
- `broker_payload_hash_invalid`
- `broker_stale_session`
- `broker_replay_detected`
- `broker_authority_metadata_rejected`
- `broker_command_dispatch_disabled`

`audit_event_required=true` and `fail_closed=true` are required by schema.

## 6. Current Rust Skeleton

The current Rust code provides:

- `native/rust_helper/src/main.rs` process lifecycle skeleton;
- `native/rust_helper/src/broker/protocol.rs` request/response decision skeleton;
- `native/rust_helper/src/broker/audit.rs` broker-local audit hash chain;
- typed request envelope validation;
- health response;
- shutdown response for test lifecycle;
- stale session rejection;
- nonce replay rejection;
- authority metadata rejection;
- command envelope suspension without dispatch.

It does not yet provide:

- production Windows named pipe / local socket / loopback transport;
- Flutter client integration;
- approval finalization;
- audit store persistence;
- credential/keychain access;
- process or update gated execution.

Unimplemented broker transport and migration items remain `release_blocker` for completed product release.

## 7. IPC Transport Decision

Transport decision remains open:

- Windows named pipe;
- localhost loopback socket with authenticated session;
- cross-platform local socket abstraction.

Selection criteria:

- Windows installed-path stability;
- authority isolation from Flutter UI;
- session authentication and replay rejection;
- crash/reconnect behavior;
- auditable failure modes;
- no hidden filesystem/process/network/credential expansion.

## 8. Validation

Current validation path:

```bash
python3 tooling/schema_check/check_schemas.py
python3 tooling/conformance_tests/run_conformance_skeleton.py
cd native/rust_helper && cargo test
```

This validation proves CONFIG / FIXTURE / INTERNAL_STATE scope. It does not prove Windows installed-path LIVE_RUNTIME broker ownership.
