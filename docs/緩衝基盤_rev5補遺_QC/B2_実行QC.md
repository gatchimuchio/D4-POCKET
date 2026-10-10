# B2 実行QC：Task一時環境の媒介

状態: `CLOSED`（B2の有限Acceptanceを充足。別moduleのcrate全体試験failureはB6回帰候補として記録）\
基準HEAD: `e27fcae90025c48e3b8a661ee98a651eb24c0a5a`\
対象: Broker承認済みAgent Task内のCodex CLI一時領域束縛\
適用正本: 現行`AGENTS.md`、`ROADMAP.md`、補遺`実装仕様書.md`§1–2、`工程表.md` B2、`実装指示書.md`§5

## 1. 有限な実装範囲

B2で最初に実作用を通す対象を、Task scratchとCodex実行環境の境界に限定する。既存経路ではRustがTaskごとのWorkspace scratchを作り、Codex CLI processの`TEMP`／`TMP`とCodex sandboxの`TEMP`／`TMP`設定を別々に組み立てていた。両者がずれると一時出力がTask隔離から外れるため、同じ検査済みscratchから両設定を作る型付き媒介を追加した。

媒介は既存のWorkspace／Permission／一回Approval判定を変更せず、新しいcommand、path、Credential、Approvalを受け取らない。scratchがWorkspace内でない、またはWorkspace rootそのものならCLIを起動しない。Codex CLIの本来のTask呼出しと要求本文は変更しない。

## 2. 実装

`native/rust_helper/src/adapters/codex_cli.rs`の`TaskTemporaryEnvironment`が、1つのscratch pathからHost側process環境とCodex sandbox設定を生成する。生成したCodex設定は`exec`のcommand argumentsより前へ追加し、Host側`TEMP`／`TMP`にも同じpathを渡す。既存の`WorkspaceTaskScratch`がTaskごとに隔離領域を作成し、Codex CLI終了後に削除する。

媒介処理自身はAuthorityを持たない。実行可否は引き続き既存BrokerのWorkspace束縛、Permission、別個Task Approval、Approval一回消費に従う。

## 3. 実行証拠

### 正常縦断

実行したcommand:

```text
cargo test --lib Broker承認経路から実CodexCLIをloopback偽APIで実行し隔離とcleanupを確認する_LIVE_RUNTIME -- --ignored --nocapture
```

結果: **PASS**（1 passed、0 failed、14.41秒）。実Codex CLI `0.162.0-alpha.2`、Rust BrokerのTask制御、Codex CLI/MxC、localhost限定Responses API試験器を通した。試験用Adapterは能力metadataだけをfixtureとしてsupportedへ上書きし、Permission／Owner Approval／Task処理は現行Brokerコードを通る。Owner確認は試験fixtureであり、native Owner UIまたはinstalled productの証拠ではない。

観測した結果:

- Permission前とTask Approval前の要求は拒否され、CLI/API接続は開始しない。
- Permissionと一回Approvalの後、実Codex CLIがfixture APIから受けたTool呼出しを実行し、Taskは`completed`、result hashが生成された。
- Toolは合成Workspace marker `synthetic-task-write`を書いた。Workspace外write markerは作られず、登録secretの内容は不変。
- Task scratchをCLI実行中に確認し、Task終端後に残らない。
- MxC TEMP markerは実行中に親から読め、CLI終了後に消えた。取消TaskのTEMP markerもchild終了後に消えた。
- `loopback_proxy_blocked_external_requests=0`。外部API／ユーザーのCodex資格情報は使用していない。
- Task projectionに指示本文、secret本文、fixture応答本文を出さない。

### 関連Rust試験

実行したcommand:

```text
cargo test --lib codex_cli::tests:: -- --quiet
```

結果: **25 passed / 0 failed / 2 ignored**。Task一時環境束縛、scope外拒否、Codex設定とHost環境値の一致、scratch cleanup失敗からRecovery、hardlink secret拒否、fixture CLI実行を含む。2 ignoredは別途明示実行されるLive試験である。

対象file整形確認:

```text
rustfmt --edition 2021 --check src/adapters/codex_cli.rs
```

結果: PASS。repo全体の`cargo fmt -- --check`は、この差分外にある既存未整形fileを列挙してFAILしたため、全体整形は実施していない。

Rust crate全体の必須確認:

```text
cargo test
```

初回実行（Windows故障注入test追加前）の結果: **FAIL**（517 passed / 3 failed / 12 ignored）。失敗は`broker::update_download::tests`のlocalhost HTTPS fixture 3件。接続resetにより2件が要求送信失敗、1件が`Network`となり期待した`DigestMismatch`に到達しなかった。

```text
cargo test -- --test-threads=1 --quiet
```

結果: **FAIL**（519 passed / 1 failed / 12 ignored）。同じlocalhost HTTPS fixtureの`bounded_catalog_fetch_accepts_small_https_document`で接続reset。

最新実行（Windows故障注入test追加後）:

```text
cargo test --quiet
```

結果: **FAIL**（518 passed / 3 failed / 12 ignored）。失敗は`failed_tool_result_is_not_replayed_as_another_exec_command`、`bounded_catalog_fetch_accepts_small_https_document`、`local_tls_server_repairs_only_after_verified_package_bytes`。いずれもlocalhost loopback応答中のConnectionReset。原因は未確定。

```text
cargo test --lib broker::update_download::tests:: -- --test-threads=1 --quiet
```

結果: **PASS**（17 passed / 0 failed）。実行shellにproxy環境変数は設定されていなかった。全crate実行時との相互作用の根因は確定していないため、全crate試験をPASSとは扱わない。失敗sourceは今回差分外であり、このB2縦断では変更していない。

```text
cargo test --lib broker_codex_loopback_support::tests:: -- --test-threads=1 --quiet
```

結果: **PASS**（12 passed / 0 failed）。全crate実行で失敗したTool結果再送防止testを含む関連loopback support群を単独・直列で実行した結果。全crate実行における失敗原因は未確定のまま残す。

### 是正履歴

初回Live試験はCodex設定引数を`exec`後へ付加する実装ミスにより、設定が適用されず偽APIへ到達しなかった。proxyは外部向け接続をすべて遮断した。根因を修正して同じRust test群を再実行し、前記24件およびLive縦断がPASSした。初回FAILは履歴として保持し、成功結果で消去しない。

### 明示負例・失敗経路のAcceptance

| B2条件 | 判定 | 実行証拠 |
|---|---|---|
| Brokerの既存Permission／Approval不足を拒否 | PASS | 上記LIVE_RUNTIMEでPermission前・Approval前を拒否し、CLI/API開始なし |
| Workspace scope外のscratch拒否 | PASS | `Task一時環境緩衝はHostとCodex内のTEMP_TMPを同一scratchへ束縛する`、`task起動時に登録後のWorkspace差し替えを拒否する` |
| Host／Adapter root不一致拒否 | PASS | `AgentTask操作は同じWorkspaceIDでもAdapter実体rootが異なれば拒否する` |
| 不完全／失敗結果を成功へ昇格しない | PASS | `不完全または失敗eventを成功へ昇格しない` |
| scratch有効化・cleanup失敗を成功扱いせずRecoveryへ残す | PASS | `scratchのjournal有効化失敗では未起動directoryを回収する`、`scratch削除失敗はTask失敗と回復記録を保ち解放後にRecoveryで回収する`（Windows file-share故障注入、1 passed） |
| secret／Task本文／fixture応答のlog・projection漏えいを防ぐ | PASS | `TaskContextのDebugは登録secretのpath名を隠す`、上記LIVE_RUNTIMEのbounded projection確認 |
| 緩衝後も元Codex Tool実行・結果検証を維持 | PASS | 実Codex CLI／MxC Tool call、Workspace marker、Task `completed`、result hashを確認 |
| 修正前後の局所QC | PASS | 初回引数順序FAILを再現・修正し、Codex adapter 25 passedとLIVE_RUNTIMEを再実行 |

## 4. B2の閉鎖境界と後続工程

本縦断と上記負例は、B2表の「実験可能な非権限媒介コアと縦断経路」について、既存Broker認可→緩衝作用→元Toolchain→独立結果確認、権限拒否、失敗伝播、局所是正を満たす。B2をCLOSEDとする。これはinstalled D4 Pocket／native Owner UI、通常Release capability、既存R2 blocker解消を示さない。

B2明記条件外で観測した全crate Rust試験failureはB2完了へ取り込まず、B6回帰候補として保持する。Update／Codex loopback fixture群は個別suiteではPASSする一方、全crate実行でConnectionResetが複数回観測された。原因は未確定であり、全crate試験のPASSを主張しない。`buffer_intervention_record`を通常製品経路で消費しUIへ投影することはB4、同一条件の介入前後比較と開発工程への自己適用はB5に属し、それらをB2完了条件へ前倒ししない。実証していない結果を`吸収成立`へ昇格しない。

## 5. 局所判定

```yaml
Condition: B2 non-authority mediation core and actual vertical path
Result: PASS (B2 finite acceptance; not release evidence)
Evidence: actual Codex CLI + current Rust Broker control + MxC + synthetic localhost Responses API; B2 negative-path tests above
Regression: initial command-argument ordering defect was reproduced, fixed, and the same tests passed
Status: B2 CLOSED; `task_execution=unsupported` and `release_ready=false` remain unchanged
Next: B3 OPEN; B4 product consumer/UI integration and B5 matched self-application comparison remain later phases
```
