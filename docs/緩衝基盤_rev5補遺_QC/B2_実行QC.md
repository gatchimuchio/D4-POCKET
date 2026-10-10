# B2 実行QC：Task一時環境の媒介

状態: `IMPLEMENTING`（この縦断部分の実作用は確認。B2全体は未受入）\
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
cargo test --lib codex_cli::tests:: -- --nocapture
```

結果: **24 passed / 0 failed / 2 ignored**。Task一時環境束縛、scope外拒否、Codex設定とHost環境値の一致、scratch cleanup、hardlink secret拒否、fixture CLI実行を含む。2 ignoredは別途明示実行されるLive試験である。

対象file整形確認:

```text
rustfmt --edition 2021 --check src/adapters/codex_cli.rs
```

結果: PASS。repo全体の`cargo fmt -- --check`は、この差分外にある既存未整形fileを列挙してFAILしたため、全体整形は実施していない。

Rust crate全体の必須確認:

```text
cargo test
```

結果: **FAIL**（517 passed / 3 failed / 12 ignored）。失敗は未変更の`broker::update_download::tests`内のlocalhost HTTPS fixture 3件で、接続resetにより2件が要求送信失敗、1件が`Network`となり期待した`DigestMismatch`に到達しなかった。

```text
cargo test -- --test-threads=1 --quiet
```

結果: **FAIL**（519 passed / 1 failed / 12 ignored）。同じlocalhost HTTPS fixtureの`bounded_catalog_fetch_accepts_small_https_document`で接続reset。

```text
cargo test --lib broker::update_download::tests:: -- --test-threads=1 --quiet
```

結果: **PASS**（17 passed / 0 failed）。実行shellにproxy環境変数は設定されていなかった。全crate実行時との相互作用の根因は確定していないため、全crate試験をPASSとは扱わない。失敗sourceは今回差分外であり、このB2縦断では変更していない。

### 是正履歴

初回Live試験はCodex設定引数を`exec`後へ付加する実装ミスにより、設定が適用されず偽APIへ到達しなかった。proxyは外部向け接続をすべて遮断した。根因を修正して同じRust test群を再実行し、前記24件およびLive縦断がPASSした。初回FAILは履歴として保持し、成功結果で消去しない。

## 4. B2全体の状態と未成立条件

本縦断は、実Codex CLIとMxCでのTask一時領域作用・Tool結果・正常／取消cleanupを示す。B2全体のCLOSED、installed D4 Pocket／native Owner UI、通常Release capability、既存R2 blocker解消を示さない。crate全体のRust試験も上記の未解決失敗によりPASSではない。

B2工程の明記条件は、既存Broker認可→緩衝作用→元Toolchain→独立した結果確認の実動作と、権限拒否・失敗伝播・局所是正の確認である。残るB2否定経路は、Host不一致、意味不一致、rollback失敗時の安全停止、log秘匿について、既存testが今回の縦断へ十分に結び付くかをAcceptance Ledgerで確認する。`buffer_intervention_record`を通常製品経路で消費しUIへ投影することはB4、同一条件の介入前後比較と開発工程への自己適用はB5に属し、それらをB2完了条件へ前倒ししない。実証していない結果を`吸収成立`へ昇格しない。

## 5. 局所判定

```yaml
Condition: B2 Task temporary environment mediation vertical slice
Result: PASS (local vertical slice only)
Evidence: actual Codex CLI + current Rust Broker control + MxC + synthetic localhost Responses API; Rust tests above
Regression: initial command-argument ordering defect was reproduced, fixed, and the same tests passed
Status: B2 remains IMPLEMENTING; no phase closure or release promotion
Next: close only B2's remaining negative-path Acceptance items; defer product consumer/UI integration to B4 and matched self-application comparison to B5
```
