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
| R2-A | active Task cancel／deadline、子孫停止、Recovery、stale authority不再利用 | `IMPLEMENTING` | 既存失敗履歴: registry `installed_task_deadline_attempt_run52`、`installed_task_cancellation_probe_2026_10_05`、`installed_task_cancellation_probe_120s_registration_hang_2026_10_05`。いずれもPASS証拠にはしない | 直接的な回帰証拠のみ |
| R2-B | Codex／Broker／Launcher crash、子孫停止、Recovery、stale authority不再利用 | `OPEN` | registry `installed_task_crash_recovery_run50`／`installed_task_crash_recovery_run51`は部分・失敗範囲を保持し、受入れPASSへ昇格しない | 直接的な回帰証拠のみ |
| R2-C | Task間scratch／Agent状態の非汚染 | `OPEN` | 未着手 | 直接的な回帰証拠のみ |
| R2-D | Agent Centerのresult／diff／changed files／test result表示とContent Exposure境界 | `OPEN` | 実装・local test記録は`docs/REV3_PROGRESS.md`に保持。配置済み製品の受入れは未成立 | 直接的な回帰証拠のみ |
| R2-E | 配置済み経路で通常NTFS／OneDrive Cloud Files双方のsecret・Workspace境界 | `OPEN` | 未着手 | 直接的な回帰証拠のみ |
| R2-F | D4-owned WorkspaceTaskScratchのTask結合、通常／cancel／deadline／crash回収、failure時fail-closed | `OPEN` | cancel／deadline／crashのScratch証拠はR2-A/Bから参照し、同じ意味の試験を重複しない。MxC AppContainer TEMP物理削除は対象外 | 直接的な回帰証拠のみ |
| R2-G | R2-A〜FのPASS後、normal Release `task_execution=supported`へ昇格 | `OPEN` | R2-A〜FのPASS後に実施 | 具体的な回帰証拠のみ |
| R2-H | source commit、app／launcher／Broker hash、runtime identity、installed root、Audit store identityをRelease evidenceへ結合 | `OPEN` | 正式証拠bundleの最終検証はR13/R14 | 具体的な回帰証拠のみ |

現在の工程状態: `IMPLEMENTING`

現在のactive condition: `R2-A`

次条件: `R2-A`がPASS/CLOSEDになった後に限り`R2-B`を開始

### R2-A 開発通過Acceptance Contract

R2-Aは次の一回ずつの限定受入れでPASSとする。短縮期限は試験専用`r2-e2e`経路に閉じ、normal Releaseの900秒上限・Permission・Approval・process supervisionを変更しない。

1. staged installed D4 Pocket UIから、native Owner確認、production Broker、実Codex CLI／MxC childを通して合成Taskをactiveにする。
2. Owner cancelを一回行い、cancelled終端、Codex／child process停止、D4-owned WorkspaceTaskScratch回収、Recovery／Auditを確認する。
3. 短縮deadlineを一回発生させ、deadline由来のfailed終端、Codex／child process停止、Scratch回収、Recovery／Auditを確認する。cancelとdeadlineの終端理由を混同しない。
4. 各終端後、消費済みPermission／Approvalを再利用できず、次Taskは現行条件で新規Permission／Approvalを要求することを同一受入れ内で確認する。
5. loopback偽APIのみを使用し、real model、production credential、課金、外部要求を発生させない。

受入れはこの範囲で終了する。複数回race、全種process fault matrix、通常900秒deadline待ち、長時間反復、performance、別OS／追加platform variantはR14等の後工程へ送る。MxC AppContainer TEMPの物理削除はD4 PocketのR2保証対象外。

## 4. R2後の必須遷移

R2-A〜HがすべてPASS/CLOSEDとなったblock内で、R2をCLOSED、`task_execution=supported`を有効化、R3をOPENとし、Codex A/Bの独立Taskによる最初のCompare Taskを開始する。release readinessは別gateとして未成立のまま維持できる。R3はR2安全試験を再実行せず、R3固有の比較orchestrationを検証する。R3完了後はR4 Handoffへ直ちに進む。
