# D4 Pocket / GUI-Shell rev4 受入れ台帳

更新日: 2026-10-05
工程正本: ユーザー提示「D4 Pocket / GUI-Shell rev4 工程収束制御」
状態正本: 本台帳
補助索引: `release_blockers.registry.json` の `comprehensive_extension_rev1_completion.r2_bounded_exit_gate`

## 1. 目的と品質段階

本台帳は、D4 Pocket全体の完成目標を縮小せず、開発工程の受入れと最終品質保証を分離する。

- **R2開発通過gate:** R3へ安全に進むための機能成立、基本安全境界、致命的failureの不在を、条件ごとに必要十分な一回の受入れで確認する。
- **R13 Formal Evidence:** 最終候補の証拠bundle、provenance、署名等、正式証拠の成立を確認する。
- **R14最終QA:** 全機能統合後の全数regression、長時間稼働、fault injection、performanceおよびR13 Formal Evidenceを含む最終品質判断を行う。

R2のPASS/CLOSEDはrelease ready、formal Windows acceptance、製品全体の安全保証を意味しない。後工程のQAを先取りしてR2を延命しない。逆に工程収束を理由にAuthority、Permission、Approval、Audit、Recovery、secret／Workspace境界、Content Exposure契約を弱めない。

Acceptance外で見つけた課題は、既存Acceptanceを直接破壊しない限り現在工程のblockerへ追加せず、R13、R14、release gate、後続phase、hardening backlogまたはexternal limitationへ分類する。

## 2. 状態規則

工程状態は `OPEN` → `IMPLEMENTING` → `VALIDATING` → `ACCEPTANCE` → `CLOSED` とする。失敗が出た条件だけを局所修正する。PASSした条件は同一blockでCLOSEDとし、再訪は具体的regression evidenceが成立条件を破壊した場合の `REGRESSION_REOPENED` に限る。

次の条件は番号順に一つずつ処理する。active condition以外の条件を並行開始しない。CLOSED条件に対する別fixture名、証拠強化だけの再実行、追加platform variantは行わない。既存rev3文書は履歴として保持し、本台帳の状態を過去記録から推定・上書きしない。

R14へ送る作業には、追加long-run、網羅的fault matrix、全数regression、性能測定、正式証拠bundle全体の再検証を含む。R2ではR2条件の直接受入れに必要な場合だけ限定して実行する。

## 3. 受入れ台帳

| 識別子 | 条件 | 状態 | 証拠 | 再開条件 |
| --- | --- | --- | --- | --- |
| R2-C0 | 配置済み製品からの正常Task完了 | `CLOSED` | `docs/REV3_PROGRESS.md`の既存配置済みTask完了記録、台帳内R2正常Task記録 | 直接的な回帰証拠のみ |
| R2-C1 | Permission／Approvalの一回消費 | `CLOSED` | 同上の既存Permission／Approval監査証拠 | 直接的な回帰証拠のみ |
| R2-C2 | 永続AuditとBroker再起動後の読戻し | `CLOSED` | `docs/REV3_PROGRESS.md`の既存Audit再起動読戻し記録 | 直接的な回帰証拠のみ |
| R2-C3 | native trayからの正常終了 | `CLOSED` | `docs/REV3_PROGRESS.md`のrun40正常終了記録 | 直接的な回帰証拠のみ |
| R2-C4 | 局所cross-Workspace隔離LIVE_RUNTIME | `CLOSED` | 既存の局所隔離LIVE_RUNTIME記録 | 直接的な回帰証拠のみ |
| R2-A | active Task cancel／deadline、子孫停止、Recovery、stale authority不再利用 | `CLOSED` | 本書「R2-A受入れ結果」、`docs/REV3_PROGRESS.md`の2026-10-05追補、`release_blockers.registry.json`の`r2_bounded_exit_gate.closed_conditions.R2-A`。従来の失敗履歴は保持 | 直接的な回帰証拠のみ |
| R2-B | Codex／Broker／Launcher crash、子孫停止、Recovery、stale authority不再利用 | `OPEN` | registry `installed_task_crash_recovery_run50`／`installed_task_crash_recovery_run51`は部分・失敗範囲を保持し、受入れPASSへ昇格しない | 直接的な回帰証拠のみ |
| R2-C | Task間scratch／Agent状態の非汚染 | `OPEN` | 未着手 | 直接的な回帰証拠のみ |
| R2-D | Agent Centerのresult／diff／changed files／test result表示とContent Exposure境界 | `OPEN` | 実装・local test記録は`docs/REV3_PROGRESS.md`に保持。配置済み製品の受入れは未成立 | 直接的な回帰証拠のみ |
| R2-E | 配置済み経路で通常NTFS／OneDrive Cloud Files双方のsecret・Workspace境界 | `OPEN` | 未着手 | 直接的な回帰証拠のみ |
| R2-F | D4-owned WorkspaceTaskScratchのTask結合、通常／cancel／deadline／crash回収、failure時fail-closed | `OPEN` | cancel／deadline／crashのScratch証拠はR2-A/Bから参照し、同じ意味の試験を重複しない。MxC AppContainer TEMP物理削除は対象外 | 直接的な回帰証拠のみ |
| R2-G | R2-A〜FのPASS後、normal Release `task_execution=supported`へ昇格 | `OPEN` | R2-A〜FのPASS後に実施 | 具体的な回帰証拠のみ |
| R2-H | source commit、app／launcher／Broker hash、runtime identity、installed root、Audit store identityをRelease evidenceへ結合 | `OPEN` | 正式証拠bundleの最終検証はR13/R14 | 具体的な回帰証拠のみ |

現在の工程状態: `IMPLEMENTING`

現在のactive condition: `R2-B`

次条件: `R2-B`を開始

### R2-A 開発通過Acceptance Contract

R2-Aは次の一回ずつの限定受入れでPASSとする。短縮期限は試験専用`r2-e2e`経路に閉じ、normal Releaseの900秒上限・Permission・Approval・process supervisionを変更しない。

1. staged installed D4 Pocket UIから、native Owner確認、production Broker、実Codex CLI／MxC childを通して合成Taskをactiveにする。
2. Owner cancelを一回行い、cancelled終端、Codex／child process停止、D4-owned WorkspaceTaskScratch回収、Recovery／Auditを確認する。
3. 短縮deadlineを一回発生させ、deadline由来のfailed終端、Codex／child process停止、Scratch回収、Recovery／Auditを確認する。cancelとdeadlineの終端理由を混同しない。
4. Task開始Auditで当該Permission／Approvalの一回消費を確認する。消費済みGrantの再利用拒否は既存CLOSEDのR2-C1証拠を再利用し、R2-Aで同じ試験を再実行しない。終端後に旧Grantが復元・再発行されないことを監査記録で確認する。
5. loopback偽APIのみを使用し、real model、production credential、課金、外部要求を発生させない。

受入れはこの範囲で終了する。複数回race、全種process fault matrix、通常900秒deadline待ち、長時間反復、performance、別OS／追加platform variantはR14等の後工程へ送る。MxC AppContainer TEMPの物理削除はD4 PocketのR2保証対象外。

### R2-A 受入れ結果（2026-10-05）

- `Owner cancel`: installed run `d4p-r2-A-cd4f2ab-run2`、実Codex CLI 0.160.0／MxC child。Task `e3a093e56923d91b8f2053336df094cd`は新規Permission（`broker-audit-139/140`）と独立Owner Approval（`broker-audit-142/143`）を使い、開始Audit `broker-audit-146`に一回消費を記録した。取消要求`broker-audit-148/149`後、installed Agent Centerは`cancelled`と終端Audit `broker-audit-151`を表示した。終端Audit reasonには`RecoveryAction=Workspace差分を確認`が記録された。loopback偽APIは1 request／tool call送信／tool resultなし／外部要求0。終端後にCodex／MxC子process、Task scratch journal entry、`.d4p-tmp-*`、完了markerは存在しなかった。
- `短縮deadline`: 同じstaged installed経路の新Session `253b9133001ba0189eac12c210debc4b`、実Codex CLI 0.160.0／MxC childでTask `38285d80bf8d4aa0d088fb93b81211c5`を開始。新規Permission `broker-audit-167/168`、Owner Approval `broker-audit-169/170`、一回消費を記録した開始Audit `broker-audit-173`、installed UIの`failed`終端とAudit `broker-audit-182`を確認した。終端Audit reasonには`RecoveryAction=Workspace差分を確認`が記録された。試験専用`r2-e2e`短縮期限（25,000ms）で、長時間待機markerを持つ合成toolとloopback偽Responses APIを使用。偽APIは1 request／tool call送信／result未受信／外部要求0を報告。Owner cancel要求はなく、期限到達後にCodex／MxC子processとscratch journal entryはなく、Workspace完了markerもなかった。
- deadlineのAudit reasonは汎用のTask失敗／取消表現であり、終端理由単独では期限を識別できない。期限への帰属は、試験専用短縮設定、期限を越えて待機する制御probe、cancel要求の不在、Taskの`failed`終端、子孫停止を組み合わせた`CONFIG`＋`FIXTURE`＋`LIVE_RUNTIME`証拠による。コード上、期限超過はprocess群停止後に`期限超過`として返り、Task状態を`failed`へ写像する。
- 両TaskのOwner確認・Permission・Approvalは各run内で新規発行され、開始時に一回消費された。終端後の自動Grant復元・再発行はAuditにない。Grant再利用拒否は既存CLOSED R2-C1の証拠に委ね、再試験していない。
- 実モデル、production credential、課金、外部要求、OS保護設定変更なし。MxC AppContainer TEMPの書込み失敗はD4-owned WorkspaceTaskScratchの保証範囲外であり、R2-Aの判定には含めない。
- 判定: R2-Aの有限受入れ条件はPASS。過去の失敗・不成立試行は履歴として維持し、再分類しない。追加race、長時間、fault matrix、別platform検査はR14へ送る。

## 4. R2後の必須遷移

R2-A〜HがすべてPASS/CLOSEDとなったblock内で、R2をCLOSED、`task_execution=supported`を有効化、R3をOPENとし、Codex A/Bの独立Taskによる最初のCompare Taskを開始する。release readinessは別gateとして未成立のまま維持できる。R3はR2安全試験を再実行せず、R3固有の比較orchestrationを検証する。R3完了後はR4 Handoffへ直ちに進む。
