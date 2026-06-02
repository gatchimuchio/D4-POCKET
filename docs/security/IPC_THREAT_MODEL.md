# IPC Threat Model

Status: Phase 3 production broker process started
Date: 2026-06-03
Scope: Flutter UI -> Rust Security Broker -> Adapter / Runtime IPC

## 1. Boundary

GUI-Shell の authority-sensitive 経路は、Flutter UI process ではなく Rust Security Broker process を境界にする。
Authority-sensitive Flutter-Rust 接続は restricted IPC を使い、FFI/direct bridge は approval-token、authority decision、external command dispatch、audit finalization、credential、recovery authorization に使わない。

```text
Flutter GUI Process
  -> restricted IPC request
Rust Security Broker Process
  -> broker-validated envelope
Adapter / External Runtime
```

この文書は broker skeleton の初期 threat model である。まだ production cutover は完了していないため、現時点の証拠は `INTERNAL_STATE` と Rust unit tests が中心であり、Windows installed-path `LIVE_RUNTIME` proof ではない。

## 2. Threats And Required Handling

| threat | required handling | current skeleton scope | release classification |
| --- | --- | --- | --- |
| spoofed UI request | session id、nonce、payload hash、operation、IPC auth secret を検証し、失敗時は rejected / audited | authenticated loopback IPC + typed envelope validation | release_blocker until Flutter product path uses broker |
| replayed approval request | nonce replay を拒否し、broker audit に記録する | persisted nonce store rejects replay after broker restart | release_blocker until Flutter product path uses broker |
| forged runtime metadata | authority-like key/value を検出し、adapter metadata を authority として扱わない | implemented for broker metadata scanner and Python-oracle parity path | release_blocker until Flutter product path uses broker |
| malformed envelope | request_id / operation / payload_hash / nonce を必須にし、fail closed | implemented in Rust skeleton | none for skeleton; release_blocker for product cutover |
| stale or malformed `issued_at` | RFC3339 と freshness window を検証し、失敗時は rejected / audited | 300-second freshness window covered in Rust IPC integration tests | release_blocker until Flutter product path uses broker |
| persistent state unavailable | audit/replay/session persistence required 時に store がなければ health suspend / operation reject | durable file store implemented; unavailable mode remains fail-closed | release_blocker until Flutter product path uses broker |
| authority-like key/value alias | case / zero-width / camelCase / separator / alias を拒否する | implemented for broker metadata scanner and parity harness | release_blocker until Flutter product path uses broker |
| Unicode / case / zero-width normalization bypass | Unicode/case/zero-width を negative tests に含める | NFKC / zero-width / case / camelCase covered in Rust unit scope and Python oracle parity | release_blocker until Flutter product path uses broker |
| stale session | session mismatch を rejected / audited にする | production broker-server generates session per process | release_blocker until Flutter product path uses broker |
| broker unavailable | Flutter must not infer authority; UI must enter fail-closed / SUSPEND state | documented only | release_blocker |
| broken pipe / crash during approval | approval finalization must not complete; RecoveryAction required | documented only | release_blocker |
| audit append failure | broker must block finalization if audit append fails | documented only | release_blocker |
| keychain unavailable | credential-gated operation must fail closed | documented only | release_blocker |

## 3. Evidence Source Rules

- CONFIG: JSON Schema files under `specs/`.
- INTERNAL_STATE: Rust broker unit tests and typed envelope validation.
- LIVE_RUNTIME: future broker process integration and Windows installed-path evidence.
- EXTERNAL_EVIDENCE: future signed artifact / installed path evidence.
- FIXTURE: examples and negative fixtures under `examples/contracts/`.

CONFIG、INTERNAL_STATE、FIXTURE の結果は、LIVE_RUNTIME broker proof には昇格しない。

## 4. Fail-Closed Requirements

- malformed request: reject and audit;
- malformed or stale `issued_at`: reject and audit;
- stale session: reject and audit;
- replayed nonce: reject and audit;
- authority metadata: reject and audit;
- command envelope dispatch before migration: suspend and audit;
- broker audit append failure: block finalization;
- persistent state required but unavailable: suspend health and reject operations;
- broker unavailable: Flutter shows unavailable state and performs no authority decision.

## 5. Current Limitations

- item: Flutter broker unavailable behavior absent
  classification: release_blocker
  reason: Rust broker now has authenticated loopback IPC and durable store tests, but Flutter still reads local snapshot JSON and does not yet use broker IPC.
  required_action: add broker launch/discovery, broker unavailable / crash / stale-session UI fail-closed tests, and authority surface cutover.
  blocks_release: yes

- item: Flutter broker authority surface cutover absent
  classification: release_blocker
  reason: Rust broker authority parity exists for Python oracle fixtures, but Flutter still reads local snapshot JSON and does not yet use broker authority operations.
  required_action: connect Permission Center, Approval Center, Audit Viewer, Recovery Center, and authority status to broker IPC and prove fail-closed states.
  blocks_release: yes

- item: Windows installed-path broker proof absent
  classification: release_blocker
  reason: authenticated loopback IPC and durable store are proven by local Rust integration tests, not installed Windows app evidence.
  required_action: run Windows installed-path broker launch, authenticated IPC, restart persistence, and crash fail-closed evidence collection.
  blocks_release: yes
