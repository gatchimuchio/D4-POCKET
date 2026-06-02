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

Current broker skeleton modules:

- `src/main.rs`: independent process lifecycle skeleton for health / shutdown smoke.
- `src/broker/protocol.rs`: JSON request parsing, typed envelope validation, `issued_at` RFC3339 freshness rejection, stale-session rejection, nonce replay rejection, NFKC/case/zero-width/camelCase/separator/alias/value-only authority-like metadata rejection, JSON response serialization, health cutover status, and command-envelope suspension.
- `src/broker/audit.rs`: broker-local in-memory append-only audit hash chain for accepted, rejected, and suspended requests.

Rust helper must remain callable through explicit IPC or FFI boundaries.

Authority-sensitive runtime ownership is not delegated to Flutter, Python, or FFI. The broker skeleton is the Rust Security Broker migration start point, but it is not a completed production cutover:

- real external command dispatch is disabled;
- production IPC transport is not selected;
- Flutter is not yet connected to the broker;
- Python Shell Core remains a migration oracle until parity and cutover evidence exist.
- health reports `boundary_role=rust_security_broker_candidate` and `authority_cutover_status=not_active`;
- audit and replay state are in-memory skeleton state only.

These incomplete items are `release_blocker` for completed product release, not release-ready evidence.
