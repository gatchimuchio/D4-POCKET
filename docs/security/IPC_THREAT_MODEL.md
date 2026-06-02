# IPC Threat Model

Status: Phase 2 initial boundary freeze
Date: 2026-06-01
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
| spoofed UI request | session id、nonce、payload hash、operation を検証し、失敗時は rejected / audited | typed envelope validation | release_blocker until transport auth exists |
| replayed approval request | nonce replay を拒否し、broker-local audit に記録する | implemented in Rust skeleton | release_blocker until integration proof |
| forged runtime metadata | authority-like key/value を検出し、adapter metadata を authority として扱わない | implemented for broker metadata scanner | release_blocker until parity migration |
| malformed envelope | request_id / operation / payload_hash / nonce を必須にし、fail closed | implemented in Rust skeleton | none for skeleton; release_blocker for product cutover |
| stale or malformed `issued_at` | RFC3339 と freshness window を検証し、失敗時は rejected / audited | 300-second freshness window implemented in Rust unit scope | release_blocker until IPC integration proof |
| persistent state unavailable | audit/replay/session persistence required 時に store がなければ health suspend / operation reject | implemented as fail-closed skeleton mode | release_blocker until persistent store exists |
| authority-like key/value alias | case / zero-width / camelCase / separator / alias を拒否する | implemented for broker metadata scanner | release_blocker until active-path parity proof |
| Unicode / case / zero-width normalization bypass | Unicode/case/zero-width を negative tests に含める | NFKC / zero-width / case / camelCase covered in Rust unit scope | release_blocker until parity harness and cutover proof |
| stale session | session mismatch を rejected / audited にする | implemented in Rust skeleton | release_blocker until real session lifecycle |
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

- item: production IPC transport auth absent
  classification: release_blocker
  reason: Windows named pipe / local socket / loopback authenticated session selection is not complete.
  required_action: select and implement Windows-first IPC transport with authenticated session and reconnection behavior.
  blocks_release: yes

- item: full Python normalization parity absent
  classification: release_blocker
  reason: Rust skeleton now rejects NFKC / case / zero-width / camelCase / separator / alias / value-only authority metadata in unit scope, but the active product path has not been connected to a Python fixture parity harness or cut over.
  required_action: add parity harness for Python `normalize_inbound_payload` and prove active path cutover before release-ready claim.
  blocks_release: yes

- item: Flutter broker unavailable behavior absent
  classification: release_blocker
  reason: Flutter still reads local snapshot JSON and does not yet use broker IPC.
  required_action: add broker unavailable / crash / stale-session UI fail-closed tests.
  blocks_release: yes
