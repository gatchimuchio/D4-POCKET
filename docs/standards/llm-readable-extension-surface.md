# LLM が読む拡張面の標準

状態: definition lock workstream
適用範囲: GUI Shell / Runtime Operation Shell / `LLM-readable application responsibility substrate`（LLM が読むアプリケーション責任基盤）
参照 Runtime: BLUE-TANUKI（Adapter 経由のみ）

## 1. 目的

GUI Shell は、汎用 Runtime Operation Shell であり、LLM が読む「アプリケーション責任基盤」である。

LLM 開発/統合エージェントが次の要素を再発明または迂回することなく application、Runtime、Tool、Service、Adapter を実装・接続できるように、安定した machine-readable な責任構造を提供する。

- Capability の境界
- Permission の境界
- Approval の境界
- 権限の除去（Authority Strip）
- Content Exposure の制御
- Audit の evidence
- Recovery の挙動
- update / install の責任
- Runtime の trust boundary
- validation / conformance の要件

この文書は、新しい Runtime authority を有効化せず、module loader や SDK を追加せず、外部標準への採用を主張しない。

## 2. 役割モデル

人間の operator／owner:

- Runtime と Shell の state を観測する。
- Approval を grant または deny する。
- Recovery を承認または実行する。
- release claim を受理または拒否する。
- 最終責任者であり続ける。

LLM 開発/統合エージェント:

- architecture、standard、schema、conformance rule、operating document を読む。
- 範囲を限定した code / documentation の変更を提案または作成する。
- 宣言済み contract を介して新しい Runtime、Tool、Service、Adapter を接続する。
- validation を実行し、evidence を報告する。
- 権限を作成せず、自らの sensitive operation を自己承認せず、Permission を暗黙に拡大せず、conformance を迂回せず、生成した output を信頼済みの事実へ変換しない。

Runtime / Tool / Service の対象:

- 宣言済み Runtime Contract または Adapter Contract だけを介して挙動を露出する。
- `metadata`、diagnostics、または self-reporting によって権限を作成してはならない。

接続層（Adapter）:

- 対象の state を GUI Shell の schema へ正規化する。
- Authority Strip を適用する。
- Content Exposure Boundary を適用する。
- Permission を付与せずに diagnostics と failure を対応付ける。

権限境界（Rust Security Broker）:

- IPC、Approval eligibility、Audit、Recovery classification、command-envelope gating、および機密性の高い native operation に関する、意図された authority-sensitive production boundary であり続ける。

画面層（UI layer）:

- operator surface を描画し、operator input を収集する。
- Authority、Approval semantics、Audit semantics、Recovery classification、Content Visibility Rule、Runtime Trust Rule を所有してはならない。

## 3. Authority のモデル

LLM は GUI Shell Contract の第一級の実装・統合コンシューマだが、決して権限源ではない。

LLM エージェントは、実装、提案、説明、検証を行ってよい。Permission の付与、sensitive action の承認、人間の責任の代替、または生成した state の trusted runtime truth への変換を行ってはならない。

Human operator は、最終的な Approval、Recovery、責任、および release claim の権限を保持する。

Broker、Contract、Conformance の各経路が sensitive execution を統制する。すべての sensitive action は、Capability、Permission、Approval state、AuditEvent、および失敗時の RecoveryAction への mapping を維持しなければならない。

## 4. 接続モデル

新しい機能は、既存 contract を介して接続する。既存 contract で表現できない場合は、schema-first / conformance-first の順序で review する contract を明示的に導入する。

権限に関係する新しい挙動を、文書化されていない shortcut、generated configuration の副作用、UI だけの判断、Adapter metadata の claim、memory / cache からの推論、diagnostic observation、または tool-response assertion として導入してはならない。

Runtime 固有の挙動は Adapter 境界の内側に維持する。BLUE-TANUKI は最初の Reference Runtime であり続け、GUI Shell の都合で Shell Core へ取り込んではならない。

提案された integration を既存 contract で表現できない場合、正しい帰結は contract-design task であり、即興の production path ではない。

## 5. 必須の Contract family

LLM が構築する module / integration は、完了を主張する前に次の Contract family を扱わなければならない。

- 能力宣言（Capability）
- 許可（Permission）
- 承認（Approval）
- 監査（Audit）
- 修復（Recovery）
- 内容露出（Content Exposure）
- 接続層（Adapter）
- 実行対象（Runtime）
- 更新／導入（update／install）

明示的な LLM extension submission または module integration の contract は、既存 schema では範囲を限定した module onboarding を安全に表現できないと contract design が証明した場合に限り、後で導入してよい。

## 6. Conformance の証明対象モデル

将来の証明対象:

```text
第三者の LLM 開発エージェントが GUI Shell リポジトリの contract を読み、
Authority、Approval、Audit、Recovery、Content Exposure、Runtime neutrality の制約を
破らずに、範囲を限定した Reference Module または Adapter を追加できる。
```

証明対象には、成功する fixture example だけでなく negative case が必要である。有効な harness は、extension が Authority を昇格できず、Approval を迂回できず、許可されていない content を露出できないこと、必要な Audit evidence を発行すること、および必要な場合に failure を RecoveryAction または SUSPEND へ対応付けることを証明しなければならない。

## 7. 非主張事項と延期した evidence

この definition update は、cross-agent implementation の成功を証明しない。

外部標準への採用を証明しない。

新しい Runtime authority を有効化しない。

現在の Windows-first release blocker を解消しない。

Rust Security Broker の production convergence を完了しない。

plugin registry、module loader、SDK、marketplace、または広範な Runtime implementation を承認しない。

Cross-agent reproduction と ecosystem に関する claim は、measured evidence が存在し、owner が claim promotion を承認するまで延期する。
