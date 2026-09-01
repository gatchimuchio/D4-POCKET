# Conformance 報告

## 現行 conformance

- item: schema 検査
  classification: required_for_v1
  status: 通過
  evidence: `schema checkが合格: schema 26件、example 26件、negative fixture 28件`

- item: conformance 検査
  classification: required_for_v1
  status: 通過
  evidence: `conformance skeletonが合格: 141 件のcheck`

- item: conformance の同義反復検査を修正
  classification: required_for_v1
  status: 解決済み
  evidence: 権限除去と approval 編集 guard check は、製品 Shell Core 実装を import して実行する。`docs/MUTATION_VERIFICATION.md` を参照。

- item: 製品権限除去の mutation coverage
  classification: required_for_v1
  status: 通過
  evidence: `adapter_loader.strip_authority_keys` が入力を未変更で返す mutation により conformance は失敗した。mutation を戻し、最終 conformance は通過した。

- item: 製品 approval guard の mutation coverage
  classification: required_for_v1
  status: 通過
  evidence: `ApprovalQueue.can_edit` が常に `True` を返す mutation と、`ApprovalQueue.edit` が保護 field guard を迂回する mutation はどちらも conformance を失敗させた。mutation を戻し、最終 conformance は通過した。

- item: 重複した権限 key 定義
  classification: required_for_v1
  status: 解決済み
  evidence: `packages/shell_core/authority_keys.py` を `AUTHORITY_KEYS` の唯一の製品 source とする。残る重複定義は `release_blocker` と分類する。

- item: ghost invariant 測定
  classification: required_for_v1
  status: 解決済み
  evidence: `packages/shell_core/state_snapshot.py` は静的 invariant flag ではなく `InvariantEvaluator().evaluate()` を使い、conformance が意図的な invariant 違反を検査する。

- item: 正規化 firewall
  classification: required_for_v1
  status: 実装済み
  evidence: conformance は `Trust_Level`、全角 `ｔｒｕｓｔ＿ｌｅｖｅｌ`、zero-width `trust\u200b_level`、`permissionGrant`、`admin_context`、入れ子 frame metadata 権限、値のみの権限試行、PolicyEvaluator のadapter metadata正規化、adapter metadataの値のみの試行拒否を対象とする。

- item: Flutter 局所 Shell Core client
  classification: required_for_v1
  status: 実装済み
  evidence: `ShellCoreClient.local()` は `mock()` へ alias せず構造化した局所 snapshot JSON を読む。Flutter test は局所 mode、環境診断の局所診断描画、墨消し内容射影、snapshot 由来 invariant flag を検証する。

- item: GUI 操作表層
  classification: required_for_v1
  status: 実装済み
  evidence: conformance は、信頼センター、権限 map、Adapter catalog、Permission diff、問題 panel、証拠センター、Command palette、Audit timeline action、Recovery playbook 語彙、Status bar が desktop Flutter 表層にあることを検証する。

- item: Shell snapshot 生成器
  classification: required_for_v1
  status: 実装済み
  evidence: `tooling/shell_snapshot.py` は Flutter 局所 mode 用の構造化局所 snapshot JSON を生成する。これには trust record、権限 map、adapter catalog、permission diff、問題、証拠、設定、環境診断 check、非権限的 installer flag を含む。

- item: Evidence bundle 出力
  classification: required_for_v1
  status: 実装済み
  evidence: `tooling/evidence_bundle.py --check` は、release blocker metadata を保存し、`release_ready=false` を保ち、Flutter/installer 非権限境界を記録する開発 evidence bundle を検証する。

- item: 構造化 release blocker registry
  classification: required_for_v1
  status: 実装済み
  evidence: strict release mode は policy または履歴 log の生の `release_blocker` 文字列ではなく `release_blockers.registry.json` を読み、未解決の有効 blocker で失敗する。

- item: artifact portability 検査
  classification: required_for_v1
  status: 実装済み
  evidence: conformance は `tooling/packaging_portability_check.py` の存在、portability のない追跡 path の拒否、POSIX locale で `unzip` 展開後の manifest、conformance、release gate 実行を検証する。

- item: 統合 Shell Core release smoke
  classification: required_for_v1
  status: 通過
  evidence: 製品 smoke は snapshot 保存/読込み、追記専用 audit chain 検証、HMAC audit anchor 検証、改変検知、approval 編集の再 hash/再検証、recovery_id policy 検証を対象とする。

- item: 初回起動と環境診断の実装 smoke
  classification: required_for_v1
  status: 通過
  evidence: `tooling/release_smoke.py` は初回起動の config/audit path を作成し、audit 書込み可否を検証し、環境診断を実行し、installer/setup state が権限を与えず permission を暗黙承認しないことを確認する。

- item: Windows インストール先証拠検証器
  classification: required_for_v1
  status: 実装済み
  evidence: conformance は厳格 R2 の `release_evidence/windows_installed_smoke.json` 形式だけを受理する。由来/分離の欠落、外部環境診断確認を製品証拠とすること、インストール済み実行ファイル確認の欠落、installer による権限付与、権限を与える環境診断 check、未実測/手動 GUI 可視性証拠、UIAutomation 診断 tree の欠落、config/audit 確認の欠落、合成環境診断証拠、単一 check だけの浅い環境診断 payload、集約 native 表層証拠、broker 最上位の未実測権限宣言を拒否する。

## Release に不十分な項目

- item: cargo test 用 gate
  classification: required_for_v1
  reason: Rust helper はv1.0範囲にあり、現行検証は通過する。
  required_action: `cd native/rust_helper && cargo test` を通過する状態に保つ。
  blocks_release: no

- item: desktop Flutter analyze 用 gate
  classification: required_for_v1
  reason: desktop app はv1.0範囲にあり、現行検証は通過する。
  required_action: `cd apps/desktop_flutter && flutter analyze` を通過する状態に保つ。
  blocks_release: no

- item: 厳格 release 検証は未通過
  classification: release_blocker
  reason: 完成製品 release には厳格 release 検証が必要である。
  required_action: `python3 tooling/validate_all.py --strict-release` を通過する。
  blocks_release: yes

Conformance 報告は製品 readiness を示唆してはならない。
