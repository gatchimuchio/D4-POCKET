# Rust Broker IPC 規約

状態: Phase 3 の製品 broker process と Flutter broker client を開始済み
日付: 2026-06-04
範囲: Rust Security Broker envelope、認証済み loopback IPC、session、health、replay、永続 audit store、および suspended command の実行関門報告

## 1. 規約の責任主体

Rust Security Broker が権限依存 IPC の受理と拒否に責任を持つ。Flutter は要求状態の表示と operator input の収集を行ってよいが、authority、approval validity、audit finality、recovery eligibility、credential access、または external command dispatch を判定してはならない。

Authority-sensitive Flutter-Rust接続は独立 broker process + restricted IPC を原則とし、この protocol は FFI/direct bridge による authority path を定義しない。

## 2. Contract ファイル

初期 broker contract:

- `specs/ipc_request.schema.json`
- `specs/ipc_response.schema.json`
- `specs/broker_error.schema.json`
- `specs/broker_session.schema.json`
- `specs/broker_health.schema.json`
- `specs/broker_command_envelope.schema.json`

example と negative fixture は `examples/contracts/` 配下に置く。

## 3. 要求 envelope

必須 field:

- `request_id`
- `operation`
- `payload_hash`
- `nonce`
- `issued_at`
- `metadata`

初期起動中の `health` では `session_id` は任意である。session lifecycle が active になった後は、すべての権限依存 operation が現在の broker session を必要とする。

許可する operation:

- `health`
- `shutdown`
- `command_envelope`
- `authority_evaluate`
- `authority_fixture_evaluate`
- `approval_edit`
- `content_projection`
- `audit_verify`
- `normalize_payload`

`authority_evaluate` は製品権限判定 operation である。action 要求だけを受理し、呼出し側が提供する `payload.state` を拒否する。runtime、capability、permission、approval、audit、recovery の各 state は broker が所有しなければならない。`authority_fixture_evaluate` は development/parity 専用であり、Python oracle 比較のために fixture `state` を利用してよい。

`command_envelope` の dispatch は意図的に suspended とする。broker 所有状態から broker が評価した eligibility と、process / credential / update gate の状態を伴う `execution_gate` body を返す。ただし製品 cutover とインストール済み製品の実行証拠が揃うまでは、実際の外部 command を dispatch してはならない。

`payload_hash` は正本 JSON request payload に対する tag 付き SHA-256 hash である。payload のない要求は正本 JSON value `null` に結び付ける。broker は構文上有効でも一致しない hash を `broker_payload_hash_mismatch` で拒否し、audit event は要求の `payload_hash` を含む。

## 4. 応答 envelope

必須 field:

- `request_id`
- `operation`
- `status`
- `evidence_source`
- `audit_event_id`
- `error`

許可する status value:

- `accepted`
- `rejected`
- `suspended`

すべての rejected または suspended response は audit event を必要とする。受理した health と shutdown の response も監査し、broker 内部の append-only evidence を保持する。

製品 cutover 前の health response は、active な Rust 権限責任主体を表明してはならない。明示的な `dev-stdin-smoke` mode では health object が次を報告する。

- `boundary_role=rust_security_broker_candidate`
- `authority_cutover_status=not_active`
- `command_dispatch_enabled=false`
- `audit_persistence=in_memory_skeleton`
- `replay_persistence=in_memory_session_only`
- `session_persistence=in_memory_session_only`
- default skeleton mode 中は `persistence_required=false`
- `persistence_ready=false`

製品 broker-server mode では、接続された durable file store が次を報告する。

- `audit_persistence=durable_file_store`
- `replay_persistence=durable_file_store`
- `session_persistence=durable_file_store`
- `persistence_required=true`
- `persistence_ready=true`

`authority_cutover_status=active` と command dispatch には、統制された製品経路に対応する capability、permission、approval、AuditEvent、RecoveryAction、実行可能な Flutter broker validation、および Windows installed-path evidence が揃った後の将来 contract 改訂がなお必要である。

broker が永続 audit/replay/session state を必須とする設定なのに persistent store が接続されていない場合、health は `status=suspend`、`audit_persistence=in_memory_skeleton`、`replay_persistence=in_memory_session_only`、`session_persistence=in_memory_session_only`、`persistence_required=true`、`persistence_ready=false`、`broker_persistence_unavailable` を返す。health 以外の operation は同じ error で fail closed する。

## 5. Broker error の構造

Broker error は構造化し、fail closed する。

許可する error code:

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

schema は `audit_event_required=true` と `fail_closed=true` を必須とする。

## 6. 現行 Rust skeleton

現行 Rust code は次を提供する。

- `native/rust_helper/src/main.rs` の process lifecycle skeleton。
- `native/rust_helper/src/broker/ipc_server.rs` の independent-process IPC 用認証済み `127.0.0.1` loopback server。
- `native/rust_helper/src/broker/store.rs` の audit hash-chain、HMAC audit anchor、replay nonce、session state 用 durable file store。
- `native/rust_helper/src/broker/authority.rs` による normalization、policy eligibility、approval edit / rehash、content projection、audit verification、recovery mapping、command-envelope eligibility の Rust 責任実装。
- `native/rust_helper/src/broker/protocol.rs` の request/response 判定経路。
- `native/rust_helper/src/broker/audit.rs` の broker 監査 hash chain。
- `tooling/broker_parity/run_authority_parity.py` の Python oracle 対 Rust broker IPC parity harness。
- `apps/desktop_flutter/lib/services/broker_client.dart` の Flutter 側認証済み loopback IPC client と broker endpoint discovery 経路。Flutter は権限経路で process を起動しない。broker process の起動は installer / launcher / supervisor code の責任である。
- `apps/desktop_flutter/lib/services/shell_core_client.dart` の製品 mode における broker snapshot projection と、broker unavailable/auth/stale/malformed 時の fail-closed 処理。
- 未知 field を拒否する JSON request parsing。
- `ipc_response.schema.json` に整合する JSON response serialization。
- 型付けされた request envelope validation。
- `issued_at` の RFC3339 parsing と、broker の300秒 window 内に収まらない freshness の拒否。
- `payload_hash` と正本 request `payload` の payload hash binding。
- persistent state が必須だが利用不能な場合の persistence-required fail-closed behavior。
- durable audit の追記、HMAC anchor、restart verification。
- broker restart 後の durable replay nonce の拒否。
- replay nonce の timestamping と compaction。
- malformed または改ざんされた persisted state の拒否。
- health の応答。
- test lifecycle 用 shutdown response。
- stale session の拒否。
- nonce replay の拒否。
- NFKC / case / zero-width / camelCase / separator / alias / value-only hardening を伴う authority metadata の拒否。
- 製品用 `authority_evaluate`、fixture 専用 `authority_fixture_evaluate`、`approval_edit`、`content_projection`、`audit_verify`、`normalize_payload` に対する権限 operation の応答 body。
- 製品用 `authority_evaluate` は呼出し側提供 state と呼出し側 audit mapping を拒否し、command dispatch を disabled のまま、broker 所有の authority decision を authorized / denied / suspended の audit decision として記録する。
- dispatch suspension と、構造化した process / credential / update gate reporting を伴う command envelope eligibility。

リリースを阻止する項目:

- WSL から直接行う Flutter validation は、外部 Flutter shell script が現在 CRLF line ending により失敗するため `release_blocker` である。
- credential/keychain access は、credential-gated product behavior に対する `release_blocker` である。
- process execution と update execution は、明示的な gate activation、installed-product evidence、owner GO が揃うまで `release_blocker` のままである。

Python runtime 非依存の製品証拠、execution gate activation、Windows installed-path evidence は、製品完成のリリースに対する `release_blocker` のままである。Windows `flutter.bat` 経由の Flutter analyze/test evidence は存在するが、WSL から直接実行する `flutter` は環境要因で blocked のままである。

## 7. IPC transport の判断

初期製品 transport は、認証済み session を伴う localhost loopback socket とする。

- bind address は `127.0.0.1` に制限する。
- 各 broker process は `getrandom` により暗号学的にランダムな session secret を生成する。
- secret は client discovery 用 broker session file だけに書き込み、log、UI field への露出、audit payload への追加を行わない。
- 各 connection は JSON envelope より前の auth line で secret を送信する。
- request size は broker-server の `--max-request-bytes` limit で制限する。
- malformed IPC、authentication failure、古い `issued_at`、replayed nonce、oversized request は fail closed する。

選定基準:

- Windows installed-path の安定性。
- Flutter UI からの authority isolation。
- session authentication と replay rejection。
- crash/reconnect の挙動。
- 監査可能な failure mode。
- 隠れた filesystem/process/network/credential expansion がないこと。

## 8. Session、replay、freshness の方針

現在の製品 broker-server behavior:

- `session_id` は broker process ごとに生成し、health 以外の operation で検査する。
- `nonce` replay state は `recorded_at_epoch_seconds` と限定 compaction を伴って `replay_nonces.jsonl` に永続化する。
- `issued_at` は RFC3339 として parse し、broker の300秒 freshness window 外なら拒否する。
- audit event は chain 化し、`payload_hash` を含め、`audit.jsonl` に永続化する。
- audit chain の head/count は broker-local `audit_anchor.key` を使って `audit_anchor.json` 内で HMAC に結び付ける。これは local corruption / partial tamper evidence であり、external release notarization、Windows key-protection evidence、または同一 user による key+anchor+log rewrite への耐性を代替しない。
- `session.json` は active な durable session state を記録する。

製品 cutover の要件:

- 選定した restricted IPC transport を介して installed broker session を認証する。
- broker restart、crash recovery、session reconnect をまたぐ replay protection を永続化するか、別の方法で暗号学的に結び付ける。
- `issued_at` freshness window を文書化し、integration test で網羅し続ける。
- approval、recovery、credential、update、process、または runtime command を確定する前に durable audit event を出力する。
- freshness、replay、session、または audit persistence を検証できない場合は SUSPEND / rejected state で fail closed する。

## 9. 検証

現在の検証経路:

```bash
python3 tooling/schema_check/check_schemas.py
python3 tooling/conformance_tests/run_conformance_skeleton.py
python3 tooling/release_runtime_assertions.py --check
cd native/rust_helper && cargo test
```

この検証が証明するのは CONFIG / FIXTURE / INTERNAL_STATE の範囲である。Windows installed-path の LIVE_RUNTIME broker 責任主体は証明しない。
Rust integration test は Linux 上の local 独立 broker process も実行する。これは local broker process に対する LIVE_RUNTIME だが、Windows installed-path の製品証拠ではない。
`tooling/release_runtime_assertions.py --check` は、製品 Flutter broker entry、Python authority process startup がないこと、Flutter/Rust FFI/direct bridge token がないことについて CONFIG の範囲も証明する。その Rust broker restart/crash coverage は local test evidence にすぎず、Windows installed-path proof をなお代替しない。
