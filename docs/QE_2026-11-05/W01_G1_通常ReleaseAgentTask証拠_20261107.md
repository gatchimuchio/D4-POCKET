# W01 / G1 通常Release Agent Task検収

- 判定：**CLOSED（Windows normal Release / Codex CLI経路）**
- 実施日：2026-11-07
- 範囲：通常ReleaseのBroker Agent Task成功・権限拒否・cancel・deadline、Audit相関、Task scratchと子process cleanup。

この判定はW01/G1に限る。正式Installer・配布・Setup DoctorはW06、統合QCと最終QAはW08、正式release readinessは別判定。`release_ready=false`と未解決release blockerは維持する。他OSの`task_execution`はこの証拠で昇格しない。

## 候補と由来

- Source HEAD：`4beabac26054ecc45fc452cdc6d8a00e65431a4b`。
- Build時worktree：dirty。`w01-run.json`に記録されたsource snapshot SHA-256：`e57f65f8fa71eb3246b4d609994d6f86a5f06debaabd63a17613e2deefa614f9`。dirty worktreeをclean source commitと誤認しない。
- Build mode：Flutter Windows Release AOTを再利用しRust Launcher／Brokerを`cargo build --release --locked`で再構築。`r2-e2e` featureなし。
- App ID：`d4-pocket-app-ce1e9f955869450eb567f0d20eac71c8`。Audit store ID：`audit-store-a5a8d33ba2c846149b96d2ff3eedf2d8`。
- App `gui_shell_desktop.exe` SHA-256：`f434912ad1715638fe30572d502af5b6044e3e2923ad4616b7a8d194d1f2b2d9`。
- Flutter AOT `app.so` SHA-256：`77feeef7c4b7df214b40a059c5c2b8ec524902349e1fe4218dbce32459669f7d`。Build record上、Release AOTとのhash一致を確認。
- Launcher SHA-256：`8da8449ee3d906b0dafe5280ede5548bb8d3d3326850f77401926fb168e0aaac`。
- Broker helper SHA-256：`2ff6fe16a2bf2d28b4a72001a0a43a5be0a1f8413abae6da6a858d3df2ab3622`。
- Product manifest SHA-256：`4be8d0cdfaf853bf27574e5caf3a839bedf4bc2ee8e898d95698743699eda290`。

## 成立したAcceptance

### 正常Task

Candidate UIから登録済みRuntime／Workspaceを使い、native Owner確認、Workspace Permission、Task本文hashに束縛した別個のOwner Approvalを通して実Codex CLI Taskを実行した。

- Runtime／Workspace：`w01-deadline-codex`／`w01-deadline-workspace`。
- Session：`56c2e130574b35ba445e5090c18ce7fd`。Session作成／Workspace結合Audit：`broker-audit-105`／`broker-audit-106`。
- Permission発行Audit：`broker-audit-114`。Task Owner Approval発行Audit：`broker-audit-116`。
- Task：`4e3dbaf68e4711630c4e2e05b0b3bd54`。開始Audit：`broker-audit-118`。terminal Audit：`broker-audit-126`、状態`completed`。
- 結果hash：`sha256:178bc4e59cd3f8a9129e974b845eabc005ce99883d34dbeb77e9fe45ed90293b`。登録Workspace内に`w01-positive.txt`（38 bytes）が生成された。scratch cleanupはBroker結果で`true`。
- Agent報告の正しさをBrokerが検証したとは扱わない。buffer intervention状態`作用後未照合`はそのまま記録し、結果hash／Task完了から独立照合済みと推論しない。

### 権限不足の拒否

Candidateと同一hashのnormal Release Broker helperを使う既存LIVE_RUNTIME integration testを実行した。認証IPCで実Agent登録・Session・Workspaceを結合し、Permission／ApprovalのないTask要求が`rejected`／`権限拒否`となり、本文を返さず、実CLIを開始しないことを確認した。native Owner confirmationを要するPermission／Approval要求も拒否される。Broker再起動後は旧Session要求を拒否し、終了後にpersisted Auditを再読込して各responseのAudit event参照を確認した。Workspace Task scratchの残留なし。

- Test：`normal_release_agent_task_requires_native_owner_grants_over_authenticated_ipc`。
- 結果：`1 passed; 0 failed`。
- この負例ではTask自体を起動しない。実Task成功は上記Candidate UI経路で別に確認している。

Candidate UIでは別Session `6b6477fd998a789b43b2670413ca8df8`を使い、Task事前検査だけを実行した。Session／Workspace結合Auditは`broker-audit-176`／`broker-audit-177`、事前検査要求Auditは`broker-audit-196`。画面とBroker応答はPermission `未付与`、Approval `未取得`、Task `未実行`を示した。このUI事前検査を実行拒否と取り違えず、上記認証IPC負例と合わせて扱う。

### Cancel

- Task：`15a25a6134b2e66dd572cf8c38f03511`。開始Audit `broker-audit-147`、cancel要求Audit `broker-audit-153`、terminal Audit `broker-audit-155`。
- terminal状態`cancelled`／`取消`。Codex CLI／shell descendantsは停止し、candidate Launcher／Flutterだけが残った。WorkspaceTaskScratch journalは空で、Workspace内にはこのTaskによる新規fileなし。

### Deadline

- Task：`8c45282559bc25ea70bd92471f08722c`。開始Audit `broker-audit-53`、terminal Audit `broker-audit-89`。
- terminal状態`failed`／`期限超過`。Audit時刻は開始`2026-11-06T16:08:08Z`、terminal`2026-11-06T16:23:57Z`（949秒）。process treeは開始から約934秒後までの観測で既に不在だった。正確なprocess exit時刻は取得していないため、900秒ちょうどで停止したとは主張しない。期限超過でのterminal化と後続process cleanupを確認した。

## Audit・検証

- Candidate file-backed Audit：214 events。event ID重複0、previous-link不一致0、anchor count 214、anchor headと最終event hash一致。
- Normal Release拒否integration testは、Broker終了後に永続Auditを開き直し、拒否Taskと関連要求のAudit event参照を照合した。
- 実行済み：Release helper build、`normal_release_agent_task_requires_native_owner_grants_over_authenticated_ipc`、実Codex CLI loopback偽APIを用いた`Broker承認経路から実CodexCLIをloopback偽APIで実行し隔離とcleanupを確認する_LIVE_RUNTIME`（1 passed）、既存のTask用permission profile focused test（1 passed）。Warningsは未使用import／variable等で、対象testはすべてpass。
- `approval_policy="never"`はD4 BrokerがTask単位のOwner Approvalを消費した後、当該Task commandにのみ追加する。既存permission profileは維持し、read-only Dialogueには追加しない。focused testでTask commandに一つだけ、read-only commandにゼロを確認。

## Capability状態と残る射程

`codex_cli` AdapterはWindowsのnormal Release build（test/debug/`r2-e2e`除外）に限り`task_execution=supported`を返す。Candidate UIでもこのRuntime capabilityを確認した。Permission／Approvalは別個にBrokerが再評価する。

このW01判定は全体の実行planeやreleaseを解禁しない。製品画面の全機能、正式installed lifecycle、他OS、長時間・performance・全数fault QA、正式署名／Publisher identity、Audit production key、Owner Final GOは未判定のまま保持する。
