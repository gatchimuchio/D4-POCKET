# IPC Threat Model

Status: Phase 3 production broker process and Flutter broker product path started
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

この文書は broker production path の threat model である。Flutter product entry は broker-mediated path に切り替わったが、まだ command dispatch は suspended、`authority_cutover_status=not_active`、Windows installed-path `LIVE_RUNTIME` proof は未取得である。

## 2. Threats And Required Handling

| threat | required handling | current skeleton scope | release classification |
| --- | --- | --- | --- |
| spoofed UI request | session id、nonce、payload hash、operation、IPC auth secret を検証し、失敗時は rejected / audited | authenticated loopback IPC + typed envelope validation; Flutter product client uses broker session file | release_blocker aggregate_of=windows_broker_installed_smoke until Windows installed path proves it |
| replayed approval request | nonce replay を拒否し、broker audit に記録する | persisted nonce store rejects replay after broker restart | release_blocker aggregate_of=windows_broker_installed_smoke until Windows installed path proves it |
| forged runtime metadata | authority-like key/value を検出し、adapter metadata を authority として扱わない | implemented for broker metadata scanner and Python-oracle parity path | release_blocker aggregate_of=windows_broker_installed_smoke until Windows installed path proves it |
| malformed envelope | request_id / operation / payload_hash / nonce を必須にし、fail closed | implemented in Rust broker and exercised by Rust IPC tests | release_blocker aggregate_of=windows_broker_installed_smoke until installed proof |
| stale or malformed `issued_at` | RFC3339 と freshness window を検証し、失敗時は rejected / audited | 300-second freshness window covered in Rust IPC integration tests | release_blocker aggregate_of=windows_broker_installed_smoke until installed proof |
| persistent state unavailable | audit/replay/session persistence required 時に store がなければ health suspend / operation reject | durable file store implemented; unavailable mode remains fail-closed | release_blocker aggregate_of=windows_broker_installed_smoke until installed proof |
| audit anchor key / log co-residency | HMAC anchor を local corruption / partial tamper detection として扱い、同一権限で key / anchor / log を同時更新できる相手への外部完全性証明として扱わない | `audit_anchor.json` + `audit_anchor.key` implemented; external anchor / Windows key-protection proof is not complete | release_blocker registry_id=audit_anchor_external_tamper_evidence_proof until product release proof defines Windows ACL / DPAPI / external-anchor handling |
| authority-like key/value alias | case / zero-width / camelCase / separator / alias を拒否する | implemented for broker metadata scanner and parity harness | release_blocker aggregate_of=windows_broker_installed_smoke until installed proof |
| Unicode / case / zero-width normalization bypass | Unicode/case/zero-width を negative tests に含める | NFKC / zero-width / case / camelCase covered in Rust unit scope and Python oracle parity | release_blocker aggregate_of=windows_broker_installed_smoke until installed proof |
| stale session | session mismatch を rejected / audited にする | production broker-server generates session per process; Flutter product tests cover fail-closed stale session handling | release_blocker aggregate_of=windows_broker_installed_smoke until installed proof |
| broker unavailable | Flutter must not infer authority; UI must enter fail-closed / SUSPEND state | `ShellCoreClient.product()` returns `broker_unavailable` SUSPEND snapshot and does not read local JSON authority; Flutter product tests pass through Windows `flutter.bat`; `tooling/release_runtime_assertions.py --check` verifies fail-closed test coverage is present | release_blocker aggregate_of=windows_broker_installed_smoke until installed proof |
| broken pipe / crash during approval | approval finalization must not complete; RecoveryAction required | documented only | release_blocker aggregate_of=windows_broker_installed_smoke |
| audit append failure | broker must block finalization if audit append fails | documented only | release_blocker aggregate_of=windows_broker_installed_smoke |
| keychain unavailable | credential-gated operation must fail closed | documented only | post_v1_scope until credential-gated operation is included in v1.0 |

## 3. Evidence Source Rules

- CONFIG: JSON Schema files under `specs/`.
- INTERNAL_STATE: Rust broker unit tests and typed envelope validation.
- LIVE_RUNTIME: Rust broker process integration and future Windows installed-path evidence.
- EXTERNAL_EVIDENCE: future signed artifact / installed path evidence.
- FIXTURE: examples and negative fixtures under `examples/contracts/`.
- CONFIG / FIXTURE / LIVE_RUNTIME mixed assertion: `tooling/release_runtime_assertions.py --check` verifies current product entry, no Python authority process startup, no FFI/direct bridge token, broker fail-closed test coverage, and local broker restart/crash persistence test presence.

CONFIG、INTERNAL_STATE、FIXTURE の結果は、LIVE_RUNTIME broker proof には昇格しない。

## 4. Fail-Closed Requirements

- malformed request: reject and audit;
- malformed or stale `issued_at`: reject and audit;
- stale session: reject and audit;
- replayed nonce: reject and audit;
- authority metadata: reject and audit;
- release classification: `release_blocker` until Windows installed proof exists for unavailable persistence / broker cases;
- command envelope dispatch before migration: suspend and audit;
- broker audit append failure: block finalization;
- persistent state required but unavailable (`release_blocker` until installed proof): suspend health and reject operations;
- broker unavailable (`release_blocker` until installed proof): Flutter shows unavailable state and performs no authority decision.

## 5. Audit Anchor Threat Model

`audit_anchor.json` は audit event count と head event hash を `audit_anchor.key` で HMAC 化する。これは broker store 内の audit log truncate、partial rewrite、anchor mismatch、local corruption を検出するための INTERNAL_STATE / LIVE_RUNTIME local evidence である。

この anchor は external notarization、signed release artifact、remote transparency log、または OS key-protection proof の代替ではない。`audit_anchor.key`、`audit_anchor.json`、`audit.jsonl` を同一権限で同時に書き換えられる same-user attacker は、整合した HMAC anchor を再生成できる。administrator / root adversary に対する改ざん耐性も、この local file-store anchor だけでは主張しない。

Windows product release claim では、installed app path 上で key / anchor / audit log の配置、ACL、DPAPI などの OS key protection、または external anchor / signed evidence のどれを採用するかを明示し、その evidence source を EXTERNAL_EVIDENCE または Windows installed-path LIVE_RUNTIME として検証する必要がある。

## 6. Current Limitations

- item: Flutter broker unavailable installed proof absent
  classification: release_blocker
  registry_id: windows_broker_installed_smoke
  reason: Flutter product code now fail-closes broker unavailable / auth / stale / malformed response paths, release runtime assertions verify the fail-closed coverage tokens, and Windows Flutter analyze/test passed. Windows installed-path proof is still absent.
  required_action: run installed Windows broker-down / crash / stale-session UI fail-closed tests.
  blocks_release: yes

- item: Flutter broker authority surface active proof incomplete
  classification: release_blocker
  aggregate_of: windows_broker_installed_smoke
  reason: Product `main.dart` now uses broker IPC and renders broker-derived authority status/projection. This is not an independent registry blocker; it is represented by installed-path broker/runtime evidence until v1.0 explicitly changes command dispatch scope.
  required_action: collect Windows installed-path broker evidence for the v1.0 authority surface; keep command dispatch suspended unless owner changes v1.0 scope.
  blocks_release: yes

- item: Windows installed-path broker proof absent
  classification: release_blocker
  registry_id: windows_broker_installed_smoke
  reason: authenticated loopback IPC and durable store are proven by local Rust integration tests, not installed Windows app evidence.
  required_action: run Windows installed-path broker launch, authenticated IPC, restart persistence, and crash fail-closed evidence collection.
  blocks_release: yes

- item: Audit anchor external tamper-evidence proof absent
  classification: release_blocker
  registry_id: audit_anchor_external_tamper_evidence_proof
  reason: broker-local HMAC anchor detects local corruption and partial tamper, but the key, anchor, and audit log currently live in the same product store trust region. Same-user or administrator/root rewrite resistance requires Windows ACL/DPAPI evidence, external anchoring, signed evidence, or an explicitly accepted narrower threat model.
  required_action: define and validate the Windows installed-path key-protection or external-anchor model before product release claim.
  blocks_release: yes
