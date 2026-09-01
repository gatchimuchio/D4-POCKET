# GUI-Shell 正式実装仕様 v1.0

Status: 実装に伴って更新する仕様（living implementation specification）
Scope: desktop-first Runtime Operation Shell / LLM が読む実装制約
Release claim: この仕様は、完成製品の release readiness を主張しない。

この文書は GUI-Shell v1.0 の規範的な実装仕様である。Phase 0 Lock 文書は技術選定と設計姿勢を固定し、この文書は Codex、外部 LLM、第三者の実装者が GUI-Shell を拡張するときに維持すべき実装制約を固定する。

## 1. 目的

GUI-Shell は、統一された GUI から AI Runtime、Agent、Tool、Local Service を操作する desktop-first Runtime Operation Shell である。

GUI-Shell は BLUE-TANUKI 専用 GUI ではない（`not a BLUE-TANUKI-specific GUI`）。

BLUE-TANUKI は最初の Reference Runtime であり、Shell Core ではない（`BLUE-TANUKI is the first reference runtime`）。

## 2. 範囲

GUI-Shell は、次を提供する。

- Runtime の探索（Runtime discovery）
- Runtime の launch / stop / restart
- Runtime 状態の監視（Runtime status monitoring）
- Permission の管理
- Approval Queue による承認待ち管理
- Audit log の閲覧
- Recovery 操作
- Setup Doctor による環境診断
- Installer による first-run experience
- Adapter による Runtime integration

GUI-Shell 自体は Runtime intelligence を実装しない。

## 3. 対象外

GUI-Shell は、次を行ってはならない。

- BLUE-TANUKI 固有の logic を Shell Core に組み込む。
- GUI input を権限として扱う。
- 明示的な境界なしに secret を保存する。
- 宣言された Visibility Policy を越えて Runtime content を露出する。
- Adapter Contract を迂回する。
- 中核 system semantics を Flutter に依存させる。

## 4. アーキテクチャ（Architecture）

GUI-Shell は、次から構成する。

- 画面描画層（Flutter UI layer）
- 中核処理層（Shell Core）
- Runtime 登録簿（Runtime Registry）
- Adapter 読込器（Adapter Loader）
- Permission 台帳（Permission Ledger）
- Approval 待ち行列（Approval Queue）
- Audit 保管庫（Audit Store）
- Recovery 管理画面（Recovery Center）
- native 安全境界（Rust Native Helper / Rust Security Broker）
- Runtime 接続層（Runtime Adapter）

中核 asset は、framework から独立した状態を維持しなければならない。

## 5. Runtime モデル（Runtime Model）

Runtime は、Adapter を介して制御する外部 executable、Service、Agent、または local process である。

各 Runtime は、次を露出しなければならない。

- `id`
- `name`
- `version`
- `status`
- `health`
- `capabilities`
- `permissions`
- 承認要件（Approval requirement）
- 監査事象（AuditEvent）
- 診断情報（diagnostics）
- 修復手順（RecoveryAction）

## 6. Adapter の契約（Adapter Contract）

すべての Runtime integration は Adapter を経由しなければならない。

Adapter の責任:

- health の検査
- ready 状態の検査
- Runtime の snapshot
- Capability の宣言
- Permission の宣言
- Approval request の発行
- AuditEvent の発行
- 診断情報の export
- RecoveryAction の実行

Shell Core は Runtime 内部を直接呼び出してはならない。

## 7. Capability のモデル

Capability は、Runtime または Adapter が実行できることを定義する。

例:

- `process_control`
- `filesystem_read`
- `filesystem_write`
- `network_access`
- `browser_control`
- `external_api_call`
- `local_model_execution`
- `secret_access`
- `approval_required_action`

Capability は宣言的である。

Capability を持つことは、Permission を持つことを意味しない。

## 8. Permission のモデル

Permission は GUI-Shell Policy によって付与する。

規則:

- default deny とする。
- 明示的な grant を必須とする。
- Permission の変更を Audit に記録する。
- dangerous Permission には Approval を必須とする。
- Permission scope をユーザーに可視化する。
- Adapter は Permission を自己昇格できない。

## 9. Approval のモデル

次に影響する action には Approval が必要である。

- external service の利用
- filesystem の変更
- process の実行
- network の呼出し
- secret の利用
- 破壊的な operation
- authority を伴う decision

Approval record は、次を含まなければならない。

- request の id
- Runtime の id
- Adapter の id
- action の type
- 要求された Capability
- 表示可能な summary
- Content Visibility の level
- user の decision
- timestamp
- Audit の hash

## 10. 内容露出の境界（Content Exposure Boundary）

GUI-Shell は Content Visibility を遵守しなければならない。

許可する Visibility level:

- `none`
- `hash_only`
- `summary`
- `redacted`
- `full`

規則:

- `none`: 未加工の content を表示してはならない。
- `hash_only`: hash だけを表示する。
- `summary`: Adapter が承認した summary だけを表示する。
- `redacted`: redacted content だけを表示する。
- `full`: content の全文を表示できる。

## 11. 権限の除去（Authority Strip）

GUI-Shell は、外部の authority claim をそのまま受け入れてはならない。

規則:

- inbound authority key を除去する。
- external authority escalation を拒否する。
- Runtime が許可しない限り `authority_context` を生成しない。
- GUI input は権限ではない。
- Adapter の `metadata` は default では権限ではない。

## 12. Audit のモデル

意味のあるすべての state transition は AuditEvent を生成しなければならない。

Audit 対象の例:

- Runtime の start
- Runtime の stop
- Permission の grant
- Permission の revoke
- Approval の approve
- Approval の reject
- Recovery の実行
- Adapter の error
- Setup Doctor の result
- Installer の検証
- config の生成
- Content Visibility の decision

AuditEvent は append-only でなければならない。

## 13. Recovery のモデル

RecoveryAction は、安全な修復手順を定義する。

例:

- Runtime を restart する。
- config を再生成する。
- Permission file を修復する。
- 無効な cache を消去する。
- Setup Doctor を再実行する。
- diagnostic bundle を export する。
- Runtime component を再 install する。

RecoveryAction は、Adapter または Shell Policy が宣言しなければならない。

## 14. Rust helper の境界

Rust helper は、native risk を伴う operation を所有する。

責任:

- process の制御
- filesystem の診断
- port の検査
- hash／signature 用 utility
- update の検証
- secure IPC
- platform 固有の diagnostics

Flutter は、危険な native operation を直接実行してはならない。

## 15. UI Layer の責任

Flutter UI は、次を所有する。

- screen の描画
- navigation
- user input の収集
- theme
- localization
- accessibility
- visual state の保持

Flutter UI は、次を所有してはならない。

- Permission の semantics
- Approval の semantics
- Audit の format
- Runtime の Contract
- Recovery の Contract
- Adapter の Conformance
- Authority の rule

## 16. Setup Doctor による診断

Setup Doctor は、次を検証する。

- install の path
- Runtime executable の存在
- config の生成
- 書き込み可能な Audit directory
- Adapter の availability
- port の availability
- Native Helper の availability
- Runtime の health
- UI の launchability

Windows acceptance は、次を含まなければならない。

- Installer が完了する。
- install path から app が起動する。
- `MainWindowHandle` が 0 ではない。
- UIA-visible な window が存在する。
- config file が生成される。
- Audit write が成功する。

## 17. Installer の要件

目標とする product experience:

```text
Installer 完了 -> app 起動 -> Setup Doctor -> Runtime ready -> 利用可能な GUI
```

低水準の setup を通常のユーザーに負わせてはならない。

## 18. BLUE-TANUKI 用 Adapter

BLUE-TANUKI は最初の Reference Runtime である。

規則:

- GUI-Shell のために BLUE-TANUKI Core を書き換えない。
- 接続は Adapter を介して行う。
- BLUE-TANUKI 固有の concept を Shell Core に漏出させない。
- Shell Core は BLUE-TANUKI を複数ある Runtime の一つとして扱う。

## 19. 適合性（Conformance）

有効な GUI-Shell 実装は、次を通過しなければならない。

- schema の validation
- Adapter Conformance の test
- Authority Strip の test
- Content Exposure の test
- Permission Model の test
- Approval flow の test
- Audit append の test
- RecoveryAction の test
- Windows installed smoke tests による導入経路の検証

`Windows installed smoke tests` は release evidence であり、この文書が提供する証拠ではない。

## 20. リポジトリ構成

推奨構造:

```text
gui-shell/
  docs/
    specs/
      gui-shell-spec-v1.md
      adapter-conformance.md
      content-exposure-policy.md
      approval-visibility-boundary.md
      authority-strip-conformance.md
      runtime-catalog.md
  specs/
    runtime.schema.json
    adapter.schema.json
    capability.schema.json
    permission.schema.json
    approval.schema.json
    audit.schema.json
    recovery.schema.json
    diagnostic.schema.json
  apps/
    desktop_flutter/
  packages/
    shell_core/
    shell_contracts/
    shell_ui/
    blue_tanuki_adapter/
  native/
    rust_helper/
  installer/
    windows/
    linux/
    macos/
  tooling/
    schema_check/
    conformance_tests/
    ui_snapshot_tests/
```

## 21. 実装順序

必須の順序:

1. GUI-Shell specification を記述する。
2. schema を定義する。
3. Adapter Conformance を定義する。
4. Audit format を定義する。
5. Permission / Approval Model を定義する。
6. Shell Core skeleton を実装する。
7. Rust helper skeleton を実装する。
8. Flutter UI shell を実装する。
9. BLUE-TANUKI Adapter を実装する。
10. Windows Installer を実装する。
11. installed acceptance test を実行する。

UI を Contract より先に実装してはならない。

## 22. 受入条件（Acceptance criteria）

GUI-Shell v1 は、次を満たすとき acceptance target に適合する。

- Windows Installer が機能する。
- installed app が起動する。
- Setup Doctor が動作する。
- BLUE-TANUKI Adapter が接続する。
- Runtime status が可視である。
- Approval Queue が機能する。
- Permission Center が機能する。
- Audit Viewer が機能する。
- Recovery Center が機能する。
- risk を伴う operation に Native Helper を使用する。
- Conformance test が通過する。
- Shell Core に BLUE-TANUKI 固有の logic が存在しない。

これらは acceptance target である。完成製品の release には、すべての active `release_blocker` の解消と `explicit owner GO` も必要である。
