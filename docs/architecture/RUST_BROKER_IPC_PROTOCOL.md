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

Health responses must not claim active Rust authority ownership before production cutover. During the skeleton stage the health object reports:

- `boundary_role=rust_security_broker_candidate`
- `authority_cutover_status=not_active`
- `command_dispatch_enabled=false`
- `audit_persistence=in_memory_skeleton`
- `replay_persistence=in_memory_session_only`
- `session_persistence=in_memory_session_only`
- `persistence_required=false` during default skeleton mode
- `persistence_ready=false`

`authority_cutover_status=active`, persistent audit storage, persistent replay/session storage, and command dispatch require a future contract revision after the governed production path has corresponding capability, permission, approval, AuditEvent, RecoveryAction, IPC, and Windows installed-path evidence.

When the broker is configured to require persistent audit/replay/session state but no persistent store is connected, health returns `status=suspend`, `audit_persistence=in_memory_skeleton`, `replay_persistence=in_memory_session_only`, `session_persistence=in_memory_session_only`, `persistence_required=true`, `persistence_ready=false`, and `broker_persistence_unavailable`. Non-health operations fail closed with the same error.

## 5. Broker Error

Broker errors are structured and fail closed.

Allowed error codes:

- `broker_request_malformed`
- `broker_payload_hash_invalid`
- `broker_issued_at_invalid`
- `broker_persistence_unavailable`
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
- JSON request parsing with unknown-field rejection;
- JSON response serialization aligned with `ipc_response.schema.json`;
- typed request envelope validation;
- `issued_at` RFC3339 parsing and freshness rejection within a 300-second broker window;
- persistence-required fail-closed behavior when persistent state is required but unavailable;
- health response;
- shutdown response for test lifecycle;
- stale session rejection;
- nonce replay rejection;
- authority metadata rejection with NFKC / case / zero-width / camelCase / separator / alias / value-only hardening;
- command envelope suspension without dispatch.

It does not yet provide:

- production Windows named pipe / local socket / loopback transport;
- Flutter client integration;
- approval finalization;
- audit store persistence;
- replay/session persistence across broker restart;
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

## 8. Session, Replay, And Freshness Policy

Current skeleton behavior:

- `session_id` is an in-process string checked for non-health operations.
- `nonce` replay state is an in-memory `HashSet`.
- `issued_at` is parsed as RFC3339 and rejected when outside a 300-second broker freshness window.
- audit events are chained in memory only.

Production cutover requirements:

- authenticate installed broker sessions through the selected restricted IPC transport;
- persist or otherwise cryptographically bind replay protection across broker restart, crash recovery, and session reconnect;
- keep the `issued_at` freshness window documented and covered by integration tests;
- emit durable audit events before any approval, recovery, credential, update, process, or runtime command finalization;
- fail closed with SUSPEND / rejected state when freshness, replay, session, or audit persistence cannot be verified.

## 9. Validation

Current validation path:

```bash
python3 tooling/schema_check/check_schemas.py
python3 tooling/conformance_tests/run_conformance_skeleton.py
cd native/rust_helper && cargo test
```

This validation proves CONFIG / FIXTURE / INTERNAL_STATE scope. It does not prove Windows installed-path LIVE_RUNTIME broker ownership.
