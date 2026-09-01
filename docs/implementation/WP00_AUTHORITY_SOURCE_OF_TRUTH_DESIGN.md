# WP00 権限正本の設計報告

日付: 2026-06-05
範囲: D-P0-00 / D-P0-01、および D-P0-07 / D-P0-08 の前提部分。

本報告は Work Package 00 に必須の実装前設計記録である。

## 影響を受けるシンボル

Rust 製品経路:

- `native/rust_helper/src/broker/protocol.rs`
  - `BrokerOperation`
  - `Broker::handle`
  - `Broker::suspend_command`
  - `Broker::accept_body`
  - `json_command_eligibility`
  - `BrokerHealth.authority_cutover_status`
  - `BrokerHealth.command_dispatch_enabled`
- `native/rust_helper/src/broker/authority.rs`
  - 現行の fixture 形式 `evaluate_authority`
  - 権限源の拒否ロジック
  - 監査対応付けの要求ロジック
- `native/rust_helper/tests/broker_ipc.rs`
  - 製品権限要求に対する IPC negative test。

Python fixture / parity 経路:

- `tooling/broker_parity/run_authority_parity.py`
  - `{"state": ..., "action": ...}` を伴う現行の `authority_evaluate` 呼出し
  - command-envelope の期待値。
- `packages/shell_core/policy_evaluator.py`
  - conformance と parity の Python fixture/oracle evaluator として維持。
- `packages/shell_core/runtime_state.py`
  - 製品 broker の権限状態ではなく、fixture/runtime-state object として維持。
- `packages/shell_core/runtime_registry.py`
- `packages/shell_core/permission_ledger.py`
- `packages/shell_core/approval_queue.py`
- `packages/shell_core/audit_store.py`
- `packages/shell_core/recovery_catalog.py`
- `packages/shell_core/update_policy_store.py`

Dart 製品経路:

- `apps/desktop_flutter/lib/services/broker_client.dart`
  - 汎用 broker 要求について contract の変更は不要。
- `apps/desktop_flutter/lib/services/shell_core_client.dart`
  - `_brokerCommandProbePayload` は現在、呼出し側 fixture の `state` と呼出し側の `audit_event` を送信する。
- `apps/desktop_flutter/test/widget_test.dart`
  - fake command response では eligibility の期待値更新だけが必要となる可能性がある。

schema と protocol 文書:

- `specs/ipc_request.schema.json`
  - fixture 専用の `authority_fixture_evaluate` を追加。
- `specs/broker_error.schema.json`
  - 製品権限 contract の拒否コードを追加。
- `docs/architecture/RUST_BROKER_IPC_PROTOCOL.md`
  - 製品権限評価と fixture parity 評価を区別。
- `GUI_Shell_Product_Quality_Integrated_Correction_Ledger_v4_2026-06-05.md`
  - この Work Package の正本 ledger は追加済み。

## 再利用する構成要素

既存の Python 構成要素は development/fixture 構成要素として維持する。

- `RuntimeState` は conformance fixture と Python oracle test を引き続き支える。
- `PolicyEvaluator(RuntimeState)` は `authority_fixture_evaluate` が使う fixture evaluator として維持する。
- `RuntimeRegistry`、`PermissionLedger`、`ApprovalQueue`、`AuditStore`、`RecoveryCatalog`、`UpdatePolicyStore` は小規模な broker 所有状態の候補だが、Python 構成要素であり Rust broker 製品 process へ直接 import しない。

WP00 では、Rust broker の Rust authority module に最小限の内部権限 registry model を置く。これは意図的に保守的な設計である。

- 呼出し側が供給するのではなく、broker が所有する。
- 小規模な broker command-dispatch record 集合に閉じる。
- command dispatch が suspended の間は permission denied を維持する。
- broker 所有状態だけから判定結果を出力する。

永続化 registry の完全移行、より豊かな関係閉包、正本 `ActionEnvelope`、言語間の正本 payload hash は WP01 に延期する。

## 製品権限入力 contract

製品用 `authority_evaluate` は action 要求のみを受け付ける。

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

製品用 `authority_evaluate` は `state` を含む payload を拒否する。呼出し側が宣言した authority provenance field と、呼出し側が提出した `audit_event` mapping も拒否する。判定に対する製品監査源は broker による audit emission だけである。

`command_envelope` は suspended のままとする。その eligibility section は同じ broker 所有権限状態から導出し、呼出し側の fixture state を証拠として扱わない。

## Fixture の分離

既存の state/action parity evaluator は、独立した次の operation の背後へ移す。

```text
authority_fixture_evaluate
```

この operation は Python-oracle parity と conformance 専用である。既存の fixture coverage を活用できるよう `{"state": ..., "action": ...}` payload 形式を維持するが、製品 UI と command-envelope code はこれを使ってはならない。

## 内部 provenance / issuer 規則

製品権限の provenance は broker 内部に置く。

- issuer: `gui-shell-rust-broker`
- source: `rust_security_broker`
- 証拠源: cutover までの denied/suspended 権限判定には `INTERNAL_STATE`
- 呼出し側が提供する `authority_source` は、broker 名を指定していても権限源ではない。
- 未知、偽造、または denylist 対象の呼出し側権限源値は判定を拒否させる。

## Broker が出力する監査の順序

製品権限評価では、次の順序に従う。

1. IPC envelope validation、freshness、session、replay、metadata authority stripping、persistence check を先に実行する。
2. operation 処理前に replay nonce を記録する。
3. broker 所有権限状態が action 要求を評価する。
4. broker が判定について一つの audit event を追記する。
5. audit append に失敗した場合、broker は suspended `broker_audit_append_failed` を返す。
6. response body は権限判定を含む。要求を処理した場合、最上位 transport status は `accepted` となり得るが、broker audit event の decision は `authorized`、`denied`、または `suspended` である。

製品経路では、呼出し側が提供した audit event mapping を権限の前提として決して受理しない。

## 移行への影響と WP01 への延期事項

WP00 では意図的に command dispatch を有効化せず、authority cutover も active にしない。

WP01 へ延期する事項:

- 正本 `ActionEnvelope` schema。
- runtime/capability/operation/permission/approval/recovery/target-scope 関係閉包の完全化。
- 必須の `permission.runtime_id` 移行。
- 永続化された broker 所有 registry。
- Dart/Rust/Python 間で正本 payload hash を結び付けること。
- normalization key collision の拒否。
- adapter ingress の単一路閉包。
- `SensitiveActionRouter` の製品用 fail-closed 再設計。
