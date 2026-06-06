# Rust Broker IPC Protocol

Status: Phase 3 production broker process and Flutter broker client started
Date: 2026-06-04
Scope: Rust Security Broker envelope, authenticated loopback IPC, session, health, replay, durable audit store, and suspended command execution gate reporting

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
- `authority_evaluate`
- `authority_fixture_evaluate`
- `approval_edit`
- `content_projection`
- `audit_verify`
- `normalize_payload`

`authority_evaluate` is the production authority decision operation. It accepts an action request only and rejects caller-supplied `payload.state`; runtime, capability, permission, approval, audit, and recovery state must be broker-owned. `authority_fixture_evaluate` is development/parity-only and may consume fixture `state` for Python oracle comparison.

`command_envelope` is intentionally suspended for dispatch. It returns broker-evaluated eligibility from broker-owned state and an `execution_gate` body with process / credential / update gate status, but it must not dispatch real external commands until product cutover and installed-product execution evidence exist.

`payload_hash` is the tagged SHA-256 hash of the canonical JSON request payload. Requests without a payload bind to the canonical JSON value `null`. The broker rejects a syntactically valid but mismatched hash with `broker_payload_hash_mismatch`, and audit events include the request `payload_hash`.

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

Health responses must not claim active Rust authority ownership before production cutover. In explicit `dev-stdin-smoke` mode the health object reports:

- `boundary_role=rust_security_broker_candidate`
- `authority_cutover_status=not_active`
- `command_dispatch_enabled=false`
- `audit_persistence=in_memory_skeleton`
- `replay_persistence=in_memory_session_only`
- `session_persistence=in_memory_session_only`
- `persistence_required=false` during default skeleton mode
- `persistence_ready=false`

In production broker-server mode, a connected durable file store reports:

- `audit_persistence=durable_file_store`
- `replay_persistence=durable_file_store`
- `session_persistence=durable_file_store`
- `persistence_required=true`
- `persistence_ready=true`

`authority_cutover_status=active` and command dispatch still require a future contract revision after the governed production path has corresponding capability, permission, approval, AuditEvent, RecoveryAction, runnable Flutter broker validation, and Windows installed-path evidence.

When the broker is configured to require persistent audit/replay/session state but no persistent store is connected, health returns `status=suspend`, `audit_persistence=in_memory_skeleton`, `replay_persistence=in_memory_session_only`, `session_persistence=in_memory_session_only`, `persistence_required=true`, `persistence_ready=false`, and `broker_persistence_unavailable`. Non-health operations fail closed with the same error.

## 5. Broker Error

Broker errors are structured and fail closed.

Allowed error codes:

- `broker_request_malformed`
- `broker_payload_hash_invalid`
- `broker_payload_hash_mismatch`
- `broker_issued_at_invalid`
- `broker_persistence_unavailable`
- `broker_authentication_failed`
- `broker_ipc_malformed`
- `broker_request_oversized`
- `broker_audit_append_failed`
- `broker_stale_session`
- `broker_replay_detected`
- `broker_authority_metadata_rejected`
- `broker_authority_state_rejected`
- `broker_command_dispatch_disabled`

`audit_event_required=true` and `fail_closed=true` are required by schema.

## 6. Current Rust Skeleton

The current Rust code provides:

- `native/rust_helper/src/main.rs` process lifecycle skeleton;
- `native/rust_helper/src/broker/ipc_server.rs` authenticated `127.0.0.1` loopback server for independent-process IPC;
- `native/rust_helper/src/broker/store.rs` durable file store for audit hash-chain, HMAC audit anchor, replay nonces, and session state;
- `native/rust_helper/src/broker/authority.rs` Rust ownership for normalization, policy eligibility, approval edit / rehash, content projection, audit verification, recovery mapping, and command-envelope eligibility;
- `native/rust_helper/src/broker/protocol.rs` request/response decision path;
- `native/rust_helper/src/broker/audit.rs` broker audit hash chain;
- `tooling/broker_parity/run_authority_parity.py` Python oracle to Rust broker IPC parity harness;
- `apps/desktop_flutter/lib/services/broker_client.dart` Flutter-side authenticated loopback IPC client and broker endpoint discovery path. Flutter does not spawn processes on the authority path; broker process launch belongs to installer / launcher / supervisor code;
- `apps/desktop_flutter/lib/services/shell_core_client.dart` product-mode broker snapshot projection and fail-closed broker unavailable/auth/stale/malformed handling;
- JSON request parsing with unknown-field rejection;
- JSON response serialization aligned with `ipc_response.schema.json`;
- typed request envelope validation;
- `issued_at` RFC3339 parsing and freshness rejection within a 300-second broker window;
- payload hash binding between `payload_hash` and canonical request `payload`;
- persistence-required fail-closed behavior when persistent state is required but unavailable;
- durable audit append, HMAC anchor, and restart verification;
- durable replay nonce rejection after broker restart;
- replay nonce timestamping and compaction;
- malformed or tampered persisted state rejection;
- health response;
- shutdown response for test lifecycle;
- stale session rejection;
- nonce replay rejection;
- authority metadata rejection with NFKC / case / zero-width / camelCase / separator / alias / value-only hardening;
- authority operation response bodies for production `authority_evaluate`, fixture-only `authority_fixture_evaluate`, `approval_edit`, `content_projection`, `audit_verify`, and `normalize_payload`;
- production `authority_evaluate` rejects caller-supplied state and caller audit mappings, and records broker-owned authority decisions as authorized / denied / suspended audit decisions while command dispatch remains disabled;
- command envelope eligibility with dispatch suspension and structured process / credential / update gate reporting.

Release-blocked items:

- WSL direct Flutter validation is `release_blocker` because the external Flutter shell scripts currently fail with CRLF line endings;
- credential/keychain access is `release_blocker` for credential-gated product behavior;
- process execution and update execution remain `release_blocker` until explicit gate activation, installed-product evidence, and owner GO exist.

No-Python-runtime product proof, execution gate activation, and Windows installed-path evidence remain `release_blocker` for completed product release. Flutter analyze/test evidence exists through Windows `flutter.bat`; WSL direct `flutter` remains environment-blocked.

## 7. IPC Transport Decision

The initial production transport is localhost loopback socket with authenticated session:

- bind address is restricted to `127.0.0.1`;
- each broker process generates a cryptographically random session secret via `getrandom`;
- the secret is written only to the broker session file for client discovery and is not logged, exposed through UI fields, or added to audit payloads;
- each connection sends the secret on an auth line before the JSON envelope;
- request size is bounded by the broker-server `--max-request-bytes` limit;
- malformed IPC, authentication failure, stale `issued_at`, replayed nonce, and oversized request fail closed.

Selection criteria:

- Windows installed-path stability;
- authority isolation from Flutter UI;
- session authentication and replay rejection;
- crash/reconnect behavior;
- auditable failure modes;
- no hidden filesystem/process/network/credential expansion.

## 8. Session, Replay, And Freshness Policy

Current production broker-server behavior:

- `session_id` is generated per broker process and checked for non-health operations.
- `nonce` replay state is persisted in `replay_nonces.jsonl` with `recorded_at_epoch_seconds` and bounded compaction.
- `issued_at` is parsed as RFC3339 and rejected when outside a 300-second broker freshness window.
- audit events are chained, include `payload_hash`, and are persisted in `audit.jsonl`.
- audit chain head/count are HMAC-bound in `audit_anchor.json` using the broker-local `audit_anchor.key`; this is local corruption / partial tamper evidence and does not replace external release notarization, Windows key-protection evidence, or same-user key+anchor+log rewrite resistance.
- `session.json` records active durable session state.

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
python3 tooling/release_runtime_assertions.py --check
cd native/rust_helper && cargo test
```

This validation proves CONFIG / FIXTURE / INTERNAL_STATE scope. It does not prove Windows installed-path LIVE_RUNTIME broker ownership.
Rust integration tests additionally exercise a local independent broker process on Linux. That is LIVE_RUNTIME for the local broker process, but not Windows installed-path product proof.
`tooling/release_runtime_assertions.py --check` also proves CONFIG scope for product Flutter broker entry, no Python authority process startup, and no Flutter/Rust FFI/direct bridge tokens; its Rust broker restart/crash coverage is local test evidence only and still does not replace Windows installed-path proof.
