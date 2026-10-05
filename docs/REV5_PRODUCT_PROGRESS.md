# D4 Pocket / GUI-Shell rev5 Product-First 進捗

更新日: 2026-10-05
工程正本: ユーザー提示「D4 Pocket / GUI-Shell 統合実装仕様書 rev5」「統合開発工程表 rev5」「Codex実装指示書 rev5」
現行phase: `P2 Multi-Agent Compare` (`OPEN`; rev5実装指示書では`R3`)
基準Repository状態: rev5文書同期commit `39dd3f4bc7aafe350ca94fce9392095f1064d2bc`。その後の実装・検証状態は本書末尾の更新履歴を参照。

## 正本の選び方

常にユーザーが現在提示した最新版の仕様書・工程表・実装指示書と、そこへ同期したリポジトリ内の現行進捗を正本とする。旧版文書は、明示的に現行正本へ採用されない限り、履歴・補助証拠としてのみ使う。旧版の状態や要求を現行状態へ推定転記しない。

## 1. 工程方針

rev5は、完成前の製品機能開発と、Feature Complete後の最終品質保証を分離する。旧R2 Agent TaskはPRODUCT BUILD上`FUNCTIONALLY ESTABLISHED`、P1は`DONE FOR PRODUCT BUILD`とする。R2で得た実Task・Permission／Approval・Audit・Workspace境界等の証拠は保持する。R2-A〜Hの追加探索・出荷級の再確認はP2以降を止めず、必要なものを`docs/FINAL_QA_QUEUE.md`へ送る。

これはR2-A〜Hの全項目がPASSした、通常Release capabilityが有効になった、またはrelease-readyであるという意味ではない。通常Releaseの`task_execution`および明示release gateは現状のfail-closed状態を維持する。P2の検証は、実資格・実課金・外部副作用を使わない隔離試験経路で行う。通常Owner操作を待ち条件にせず、テスト専用identityまたはfixtureを使うが、実際のnative Owner確認経路を自動承認する回避策は作らない。

`docs/REV4_ACCEPTANCE_LEDGER.md`はrev4時点の履歴として保持する。そこに記録されたPASS／CLOSED、OPEN、FAILをrev5の受入れ結果へ転記・改変しない。

rev5工程表は段階を`P0`〜`P13`、同rev5実装指示書は次のCompareを`R3`と表記する。両者のCompare機能範囲は一致するため、本進捗では工程表の`P2`を現行phase名として使い、`R3`を同一範囲の別表記として記録する。

## 2. 現在の製品工程

### P0 基盤 — 既成立部分を再利用

Rust Broker、Authority、Permission、Approval、Audit、Runtime／Workspace登録、Desktop shell、Launcher、IPCを再構築しない。

### P1 Agent Task — 製品開発上は完了

成立済み正常Task経路を利用する。追加crash matrix等の深掘りをP2の開始条件にしない。rev4 ledgerの未完了・失敗履歴は保持し、統合後の検査対象をFinal QA queueへ送る。

### P2 複数Agent比較 — OPEN

対象は同一Codex CLI／同一providerを使う2つのAgent instanceである。Agent A/Bは別のSession、Workspace、Task、実行scratchを持ち、同一Taskを独立に実行する。Vendor／Modelの種類が揃うことを待たない。

| P2受入れ条件 | 状態 | 現在確認できた実装 |
| --- | --- | --- |
| Agent A／Bを独立instanceとして用意 | OPEN | Agent Centerに複数Runtime／Workspace登録を保持し、登録ごとのSession開始を追加。2 SessionのCompare run結合は未接続 |
| 同一Taskを独立Workspace／Session／Taskへ投入 | OPEN | 単一Session用Agent Task APIが存在。Compare orchestrationなし |
| 2つのTaskを同時実行 | OPEN | Compare単位の並行起動・取消制御なし |
| result／diff／tests／duration／failure／resource／Auditの比較表示 | OPEN | `AgentComparisonProjection`は識別子重複等の事前projectionだけ。実Task結果の比較ではない |
| 比較結果の選択 | OPEN | 未実装 |
| 選択した結果だけを適用 | OPEN | 未実装 |
| 片側failure／cancelの相互隔離 | OPEN | Compare単位の失敗・取消隔離なし |

P2は一回の正常CompareがD4 Pocket UIから成立し、基本的なAgent間取り違えがないことを確認した時点で閉じ、直ちにP3へ進む。選択・適用でもOwnerのAuthority、Workspace Permission、Task Approval、Auditを省略しない。比較時にAuthorityを共有・移送しない。

2026-10-05の実装更新: Agent CenterはBroker起動中に複数のRuntime／Workspace登録を保持し、それぞれの登録から別Sessionを開始できる。Session一覧に実在し一意に照合できるactive entryだけをTask操作対象とし、mock/local snapshotは比較へ入れない。登録だけではPermission／Approvalを生成しない。検証はDesktop Flutter Analyzerと全test suiteでPASS。これは独立2-SessionのCompare Task実行・差分投影・結果選択・適用の成立を意味しない。

次は現行BrokerのTask／Workspace契約を使い、2 Sessionへ同一Taskを個別事前検査し、各々の独立Permission／Approvalを維持したまま同時起動するCompare runを接続する。既存の`AgentComparisonProjection`を実比較完了へ読み替えない。

## 3. 後続製品工程

P2の受入れ後はrev5工程表の順にP3 Handoff、P4 Provider / Model Center、P5 Workspace / History / Evaluation、P6 Credential / MCP、P7 A2A / Host / Adapter、P8 GUI-Shell Compose、P9 Standalone Export、P10 Module Selection / Pruning、P11 Windows Productization、P12 Product Integrationを進める。P12の統合経路成立をWindows Feature Completeとし、その後にQ0〜Q7 Final QAへ移る。P13 Mobile / Non-WindowsはWindows Feature Completeを止めず別trackとして扱う。Owner Finalizationではproduction Publisher／signing identity、production Audit key、不可逆な事業判断、Final GOだけをOwnerへ戻す。

## 4. 関連正本

- `ROADMAP.md`: rev5工程への入口と過去作業履歴
- `docs/FINAL_QA_QUEUE.md`: Feature Complete後へ移送した検査項目
- `docs/REV4_ACCEPTANCE_LEDGER.md`: rev4受入れ履歴。rev5現行状態の正本ではない
- `docs/REV3_PROGRESS.md`: rev2/rev3の作業・失敗・実行証拠の履歴
- `release_blockers.registry.json`: release gateの証拠・分類。開発phaseの進行順と同一視しない
