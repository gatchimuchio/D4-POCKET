# Rust Helper / Rust Security Broker Skeleton

Native helper boundary for operations that should not live in UI code.

Current helper modules:

- process
- filesystem
- network
- diagnostics
- update_verification
- audit_hash
- ipc

Current broker modules:

- `src/main.rs`: independent process lifecycle for explicit `dev-stdin-smoke` diagnostics and `broker-server`.
- `src/broker/protocol.rs`: JSON request parsing, typed envelope validation, canonical payload-hash binding, `issued_at` RFC3339 freshness rejection, audit/replay/session store readiness reporting, persistent-state-required unavailable fail-closed behavior, stale-session rejection, nonce replay rejection, NFKC/case/zero-width/camelCase/separator/alias/value-only authority-like metadata rejection, JSON response serialization, health cutover status, authority operation routing, and command-envelope suspension with process / credential / update gate reporting.
- `src/broker/audit.rs`: broker-local append-only audit hash chain for accepted, rejected, and suspended requests, with request `payload_hash` included in each event hash.
- `src/broker/store.rs`: durable file store for audit hash-chain, HMAC audit anchor, compacted replay nonce state, and session state in `broker-server` mode.
- `src/broker/authority.rs`: Rust authority evaluation, normalization/quarantine, approval edit, content projection, audit-chain verification, and command eligibility evaluation.

Rust helper must remain callable through explicit IPC or FFI boundaries.

Authority-sensitive runtime ownership is not delegated to Flutter, Python, or FFI. The broker is the Rust Security Broker migration path, but it is not a completed production cutover:

- real external command dispatch is disabled;
- Flutter product path uses broker IPC, but installed Windows product evidence is still a separate release blocker;
- Python Shell Core remains a migration oracle, tooling path, and parity comparison source until cutover evidence exists.
- health reports `boundary_role=rust_security_broker_candidate` and `authority_cutover_status=not_active`;
- explicit `dev-stdin-smoke` mode uses in-memory state; `broker-server` mode uses durable audit/replay/session state when `--store-dir` is available.
- persistent-state-required mode suspends/rejects when no persistent store is connected.

These incomplete items are `release_blocker` for completed product release, not release-ready evidence.
