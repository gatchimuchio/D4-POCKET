# rev2 実装進捗と証拠境界

各節は作業時点の履歴である。現在状態は次の現況節と対象commitに結合した実証拠で確認し、過去の未実装記述を現在の状態へ読み替えない。

## D4 Pocket C9 MCP modern要求metadataとlegacy fallbackの境界修正（2026-09-28）

公式MCP 2026-07-28仕様との照合で、新protocol要求に必須の`clientCapabilities`欠落と、discoveryの失敗全般で旧protocolへ再起動する問題を修正した。全要求に固定client識別と、追加機能を宣言しない空の能力objectを付す。応答`resultType`は`complete`以外を拒否し、旧応答との互換のため欠落だけを許容する。旧protocolへの切替はJSON-RPCのmethod not found（`-32601`）または対応版なしの場合に限り、通信期限切れ・parameter不正・応答形式不正などでは再試行しない。公式版と適用範囲は`規定/正本索引.json`へ固定した。

- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib mcp -- --test-threads=1`：9件成功。Windows fake-serverでmethod not found時のlegacy fallbackと、invalid params時に起動回数1回のまま拒否することを確認。
- `cargo check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`：成功。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`：exit 1。Library 329件、CLI 10件、Broker IPC統合試験9件は成功。統合試験1件は`tests/broker_ipc.rs:370`のBroker子process起動がWindows Application Control（OS error 4551）で拒否され失敗した。今回のMCP 9件は全件成功しているが、全target試験成功へ昇格しない。`windows_rust_integration_test_execution_policy` release blockerを保持する。
- その後、診断文だけを日本語化した最終差分で同じfocused試験を再実行したが、test executableが起動前にWindows Application Control（OS error 4551）で拒否され、現行fileの再実行結果は未取得。test binaryの移動や実行制御の変更はしていない。
- `python -X utf8 tooling/schema_check/check_schemas.py`：Schema 147件、正常例147件、負例187件で成功。
- `python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`：225件で成功。
- `python -X utf8 tooling/日本語基底監査.py --strict`：負債0件・指摘0件で成功。
- `python -X utf8 tooling/release_gate_check.py`と`python -X utf8 tooling/evidence_bundle.py --check`：成功。evidence bundleはrelease blocker 5件を保持し`release_ready=false`。
- `rustfmt --check --edition 2021 --config skip_children=true native/rust_helper/src/mcp.rs native/rust_helper/src/adapters/mcp_stdio.rs`：変更したRust 2 fileの形式検査に成功。
- `cargo fmt --manifest-path native/rust_helper/Cargo.toml -- --check`：変更範囲外を含むcrate全体に既存の整形差分があり不合格。無関係なfileを一括変更せず、今回変更したRust fileだけ整形・再検査する。
- MCP要求metadataとfallback以外の機能、Credential注入、Tool実行、Resource／Prompt本文取得はこの単位で追加していない。active unresolved blocker 15件、`release_ready=false`を維持する。

## D4 Pocket C9 Desktop MCP Resource／Prompt metadata閲覧（2026-09-28）

DesktopのMCP接続一覧からResource／Prompt欄を展開し、Resource名・ID・URI hash、Prompt名・ID・引数Schema hashを読めるようにした。URI実値、Resource本文、Prompt説明・引数・本文は取得・表示しない。JSON SchemaはResource／Prompt各projectionを未知field拒否のnested contractへ具体化し、URI実値fieldと非空Prompt descriptionを拒否するnegative fixtureを追加した。Flutter clientはID／hash、status、表示名の境界を再検証し、`INTERNAL_STATE` metadataのみをUIへ渡す。Resource read／Prompt getのMCP要求、Permission、Approval、本文表示は追加していない。

- `python -X utf8 tooling/schema_check/check_schemas.py`：Schema 147件、正常example 147件、negative fixture 187件で合格。
- `python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`：225 checksで合格。
- `python -X utf8 tooling/日本語基底監査.py --strict`：負債file 0件、finding 0件で合格。
- `python -X utf8 tooling/manifest.py --check`、`git diff --check`：合格。
- Flutter MCP client／widget focused test 4件、`flutter analyze --no-pub`は最新index sourceの隔離Temp copyで合格。UI testはResource URI実値とPrompt説明markerが表示されないことを含む。installed productや外部MCP Serverの運用証拠ではない。
- Rust補助機能は本作業で変更していないためRust試験は未実施。全対象試験は既存の阻害状態を引き継ぎ、ライブラリ試験326件の後、Windowsのアプリケーション実行制御が起動用試験実行ファイルを拒否（OS error 4551）。
- Resource／Prompt実取得、Tool実行、Credential実値注入、外部MCP Harness、installed product、正式releaseは`release_blocker`。active unresolved blocker 15件、`release_ready=false`を維持する。

## D4 Pocket C9 Desktop MCP Toolカタログのmetadata閲覧（2026-09-28）

MCP接続ごとにTool一覧を展開表示し、Rust Brokerの既存projectionからTool名、Tool ID、入力Schema hash、危険度`unknown`だけを確認できるようにした。Tool descriptionと入力Schema本文は画面へ出さず、接続一覧の`INTERNAL_STATE`境界を維持する。Tool receiptのJSON Schemaをnested strict objectへ具体化し、未知field、非空description summary、既知以外のrisk/status形状を拒否するnegative fixtureを追加した。Flutter clientもID/hash形式、Tool名のcontrol/bidi文字、未知fieldを再検証してからUIへ渡す。新しい権限・Approval・Tool実行経路は追加していない。

- `python -X utf8 tooling/schema_check/check_schemas.py`：Schema 147件、正常example 147件、negative fixture 185件で合格。
- `python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`：225 checksで合格。
- `python -X utf8 tooling/日本語基底監査.py --strict`：負債file 0件、finding 0件で合格。
- `python -X utf8 tooling/manifest.py --check`、`git diff --check`：合格。
- FlutterのMCP client／widget focused test 4件、`flutter analyze --no-pub`は、最新index sourceの隔離Temp copyで合格。WidgetはTool名・ID・Schema hashの表示、説明marker非露出、一覧操作がTool実行を起こさないことを確認する。インストール済み製品や外部MCP Serverの証拠ではない。
- Rust helperはこの単位で変更していないためRust testを再実行していない。直前の同一Rust sourceでは全target compileが成功し、全target testはlibrary 326件成功後に`gui_shell_desktop_launcher` test executableがWindows Application Control（OS error 4551）で起動拒否され未完了。Rust全target gateはactive `release_blocker`のまま。
- Tool execution、Credential実値注入、Resource／Prompt実取得、外部MCP Harness、installed product、正式releaseは引き続き`release_blocker`。active unresolved blocker 15件、`release_ready=false`を維持する。

## D4 Pocket C9 Desktop MCP stdio接続作成・Owner確認（2026-09-28）

既存のMCP metadata一覧／切断画面へstdio Serverの新規接続設定を追加し、Desktop Flutterから既存Broker channel、Windows Rust起動器のdefault No native Owner確認、Rust Broker、既存Job Object監督下のMCP child起動／discoveryへ接続した。入力はServer ID、Windows絶対実行path、Workspace path、1行1項目の起動引数で、Flutterはfilesystem・process・Credentialへ直接アクセスしない。Broker要求とnative確認候補は全階層を型検証し、Credential refは対象Serverに一致する固定missing値だけを受理する。Owner確認にはServer ID、実行path、Workspace、引数件数・hash、payload hashを表示し、引数本文は秘密を含む可能性があるため表示しない。Credential実値注入、Tool実行、Resource／Prompt取得、Trust付与は行わない。

- `cargo check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`：成功。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib MCP接続 -- --test-threads=1 --nocapture`：2件成功。未知Credential ref field／対象不一致のprocess起動前拒否と、native Owner確認候補が起動範囲を示し引数本文を出さないことを確認した。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`：exit 1。Rust library 326件は成功したが、次の`gui_shell_desktop_launcher` test executableはWindows Application ControlのOS error 4551で起動前に拒否され、後続targetを含む全target試験は未完了。拒否fileの移動・再配置、test除外、policy変更はしていない。
- Flutterの新規接続client／widget focused test 4件と`flutter analyze --no-pub`は、index化した最新sourceの隔離Temp copyで成功した。Temp copyで`flutter pub get --offline`も成功。OneDrive workspace内実行やinstalled product上のOwner操作ではない。
- `python -X utf8 tooling/schema_check/check_schemas.py`：Schema 147件、正常example 147件、negative fixture 184件で成功。`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`：225 checksで成功。`python -X utf8 tooling/日本語基底監査.py --strict`：負債file 0件、finding 0件で成功。`python -X utf8 tooling/manifest.py --check`：成功。
- これによりDesktop接続設定面の実装は存在するが、focused widget／Broker試験は外部Serverとのinstalled product運用・実環境のOwner操作を証明しない。Tool実行、Credential実値注入、Resource／Prompt実取得、Streamable HTTP、OAuth、consent、quarantine、外部MCP Harness、非Windows process群監督およびinstalled product証拠は`release_blocker`のまま。Registryのactive unresolved blocker 15件、`release_ready=false`を維持する。

## D4 Pocket C9 Desktop MCP接続センターとOwner切断（2026-09-28）

先行するRust Brokerのstdio discovery／一覧／Windows Owner切断へ、Desktop設定画面のMCP接続センターを接続した。一覧は利用者の明示操作で通常Broker IPCから取得し、Broker保持projectionを`INTERNAL_STATE`として扱う。表示はServer ID、表示名、stdio種別、Tool／Resource／Prompt件数に限定し、Tool説明本文、Credential ref、接続設定、秘密値は画面へ出さない。切断要求は既存Windows Broker channelから厳密な固定payloadで送り、Rust起動器のdefault No native Owner確認を通す。Owner確認後もBrokerが要求を再検証し、Job Objectのprocess群停止、永続`LIVE_RUNTIME` Audit確定、接続記録解消の順を維持する。FlutterはOwner資格、session file、Approvalを保持せず、新しいauthority経路は追加していない。

- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib MCP切断 -- --test-threads=1 --nocapture`：2件成功。Brokerの切断統治とnative Owner確認候補の固定範囲・余分field拒否を確認した。確認候補testはインストール済製品上のOwner操作証拠ではない。
- `cargo check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`：成功。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`：exit 1。library 324件、main 10件、`broker_ipc` 10件、`canonical_decimal_hash` 1件、`checkpoint` 8件、`protected_data` 2件は成功した。次の`protected_startup` test executableはWindows Application ControlのOS error 4551で起動前に拒否され、後続targetを含む全target試験は未完了。拒否fileの移動・再配置、test除外、policy変更はしていない。
- Flutter focused MCP client／widget testはTemp source copyで2件成功し、`flutter analyze --no-pub`も同copyで指摘なし。OneDrive workspaceでのFlutter testは`build\\unit_test_assets` cleanup拒否によりtest前に失敗した。full Desktop Flutter suiteはTemp copyからRust helperを実行した段階で同じOS error 4551により完遂できず、全suite成功とは扱わない。
- `python -X utf8 tooling/schema_check/check_schemas.py`：Schema 147件、正常example 147件、negative fixture 183件で成功。`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`：225 checksで成功。`python -X utf8 tooling/日本語基底監査.py --strict`：負債file 0件、finding 0件で成功。`python -X utf8 tooling/manifest.py --check`：成功。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`：exit 0。開発用10検査が成功し、evidence bundleはWindows release blocker 5件を保持して`release_ready=false`。Rust全target／Flutter全suite、installed product、実外部MCP Server、正式releaseの証拠へ昇格しない。
- Registryのactive unresolved release blocker 15件と`release_ready=false`を維持する。画面widget／fake transport／native確認候補はinstalled productのOwner操作証拠ではない。新規MCP接続設定画面、Tool実行、Credential実値注入、Resource／Prompt実取得、Streamable HTTP、OAuth、consent、quarantine、外部MCP Harness、非Windows process群監督は未成立の`release_blocker`として維持する。

## D4 Pocket C9 MCP stdio Owner切断のBroker実装（2026-09-28）

既存Rust Brokerのowner-control経路に`MCP切断`を追加した。Owner CLIは既存のBroker認証経路へ厳密な切断要求を送り、Brokerは登録済みServerだけを対象にする。Windowsでは既存Job Object監督下のprocess群停止を確認し、`LIVE_RUNTIME`として分類した永続accepted Auditの確定後に限って接続記録を削除する。停止確認またはAudit確定が失敗した場合は記録を残して再確認を要求する。非Windowsではprocess群停止保証がないためfail-closedで拒否する。通常IPC、未知field、未知Serverを拒否し、receiptとAuditへ実行path、argv、workspace、credential実値を含めない。Permission・Approvalを生成しない。

- Owner CLI→既存Rust Broker→MCP connection center→Windows Job Object停止→永続Audit→接続記録解消が、この局所実装の実行経路である。確認済みの実Windows動作は、Windows上のBroker library試験内でfake stdio Serverを用いた範囲に限る。正式installed Broker／製品UI／実外部MCP Serverの証拠ではない。Desktop／Mobileの切断操作面は未接続であり、`release_blocker`。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib "MCP切断はowner専用でprocess群停止後に永続Auditと記録解消を確定する" -- --test-threads=1 --nocapture`：1件成功。`cargo check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`：成功。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`：exit 1。library 323件とCLI 10件は成功したが、`tests/broker_ipc.exe`はWindows Application ControlのOS error 4551で起動前に拒否された。以後の全targetは未実行。testの除外、拒否fileの移動、Application Control変更はしていない。`windows_rust_integration_test_execution_policy`をunresolved／activeへ戻した。
- `python -X utf8 tooling/schema_check/check_schemas.py`：Schema 147件、正常例147件、negative fixture 182件で成功。`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`：225 checksで成功。`python -X utf8 tooling/日本語基底監査.py --strict`：負債file 0件、finding 0件で成功。
- 未完了のMCP機能はTool実行、Credential実値注入、Resource／Prompt取得、Streamable HTTP、OAuth、consent、quarantine、外部Harness適合および非Windows process群監督で、すべて`release_blocker`。この局所実装はC9または製品releaseの完成を意味しない。

## D4 Pocket C9 MCP stdio process群監督のWindows実証（2026-09-28）

MCP stdio childを`native/rust_helper/src/adapters/process_tree.rs`の既存Job Object経路から起動するよう変更した。Windowsではchild初期threadを停止状態で生成しJob割当後に再開する。MCP stdio停止はJob内processが0件になったことを確認し、停止失敗・discovery後のServer終了を接続成功へ昇格しない。legacy fallbackも先行process群の終了確認後だけ起動する。`SupervisedChild`のDebug表示はprocess IDだけに限定した。

- Windows fake-server process fixtureは固定MCP discovery応答を返し、rootの子として`ping.exe`を起動する。Focused試験でMCP接続、孫process起動marker、root存続、Job内全process終了、root回収を確認した。fixtureはProtocol形状と実Windows process群監督の証拠であり、外部MCP Server適合、installed Broker、Broker異常終了時のMCP専用挙動は証明しない。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib "MCP_stdio_Serverのprocess群をBroker監督下で起動し終了する" -- --test-threads=1 --nocapture`: 1件成功。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`: lib 322件、main 9件成功後、`broker_ipc` test executableがWindows Application ControlのOS error 4551により起動前に拒否され、commandはexit 1。全target成功とは扱わない。拒否回避・test除外は行わない。
- `cargo check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`: 成功。
- `python -X utf8 tooling/schema_check/check_schemas.py`: Schema 145件、正常例145件、negative fixture179件で成功。`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`: 225件で成功。`python -X utf8 tooling/日本語基底監査.py --strict`: 負債file 0件・finding 0件で成功。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`: exit 0。日本語厳格監査、Schema、Conformance、Manifest、release gate、packaging portability、release smoke、evidence bundle、release runtime assertion、C32対応表監査の開発用10検査が成功。検査結果は`release_ready=false`で、registryのactive unresolved blocker 14件を維持する。この開発validatorは正式release gate合格、installed product、実外部MCP接続を証明しない。
- 対象fileは`native/rust_helper/src/adapters/mcp_stdio.rs`、`native/rust_helper/src/adapters/process_tree.rs`、`docs/specs/mcp-connection-center.md`、`docs/specs/process-supervision.md`、Conformance検査、進捗・ROADMAPである。Tool実行、credential実値注入、Resource／Prompt取得、Streamable HTTP、OAuth、consent、disconnect、quarantine、実外部MCP Server検証は引き続き`release_blocker`。registry上のactive blockerと`release_ready=false`を維持する。

## D4 Pocket Phase 11 Mobile資源観測scope変更時の直列化追補（2026-09-28）

先行変更後の追試で、観測中にRuntime scopeが変わると、古い応答を破棄できても新scopeの要求を旧要求完了前に始め得ることを確認した。Mobile資源画面に画面instance内の要求直列化を追加し、旧要求の完了後に限って現scopeの要求を送る。Permission、Approval、Broker統治経路、他画面の要求は変更しない。

- 変更前の最新push済みcommit `866a254243cc00382e03c64a6f020e33fad19492`から作成した非OneDrive managed worktreeへ同じMobile source/test差分を適用し、両Dart fileのSHA-256一致を確認した。
- `apps/mobile_flutter`で`flutter test test/resource_overview_test.dart`: 3件成功。Runtime scopeを遅延要求中に置換し、旧要求の完了までは新要求が送信されないこと、観測中要求の最大数が1であることを確認した。
- `apps/mobile_flutter`で`flutter test`: 21件すべて成功。これはFlutter testであり、Android実機、installed product、TLS/Device LinkのLIVE_RUNTIME結合証拠ではない。
- DesktopとMobileの`flutter analyze`は両方exit 1。analysis serverがLSP初期化JSONの途中切れで`FormatException: Unexpected end of input`を出して異常終了したため、analyze成功とは扱わない。Dart formatterと`git diff --check`は成功。
- Registry上のactive release blocker 14件、Android実機凍結、`release_ready=false`を維持する。

## D4 Pocket Phase 11 Mobile資源観測を表示中に限定（2026-09-28）

MobileHomeは全画面を`IndexedStack`で保持するため、資源画面のbuild時自動観測が非表示でも走っていた。選択中かつ接続中に限り一度観測し、Runtimeごと逐次・最大16件・同時要求1件に制限する。画面離脱または接続scope変更で表示を破棄し、in-flight結果を採用せず、後続Runtimeへの要求を止める。再選択時は新しい観測を行う。Permission、Approval、Broker統治経路は変更しない。

- 最新`origin/main`の変更前commit `e78c12230872cd834b9ce0478db70a7e9917584f`から作成した非OneDrive managed worktreeへ同じMobile source/test差分を適用した。`flutter test test/resource_overview_test.dart`: 2件成功。画面選択・離脱・再入場と遅延応答後の後続Runtime要求抑止を検査する。
- 同worktreeの`apps/mobile_flutter`で`flutter test`: 20件すべて成功。これはFlutter widget/client testであり、Android実機・installed product・TLS/Device Link LIVE_RUNTIME結合の証拠ではない。
- `apps/mobile_flutter`と`apps/desktop_flutter`の`flutter analyze`は、分析serverから途中で切れたLSP初期化JSONを受けた`FormatException`でexit 1。成功・警告なしとは扱わない。Dart formatterは変更3 fileに成功した。
- Release blocker 14件、Android実機凍結、`release_ready=false`を維持する。Flutter画面試験はDevice Link native境界または正式製品の実機証拠を代替しない。

## D4 Pocket Phase 9 履歴承認の期限監視を期限駆動へ変更（2026-09-28）

実行履歴画面の100ms周期timerを廃止し、履歴承認の残り時間について壁時計と単調時計の早い方で一度限りの期限timerを設定する。承認の再確認後にtimerを更新し、従来の2秒ごとの承認・履歴再取得、期限切れ時の表示消去、背景化・画面離脱時の破棄は維持する。承認の意味、Broker権限、履歴取得頻度は変更しない。

- `python -X utf8 tooling/schema_check/check_schemas.py`: Schema 145件、正常例145件、negative fixture179件で合格。
- `python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`: 225 checksで合格。
- `python -X utf8 tooling/日本語基底監査.py --strict`: 厳格監査合格、負債file 0、finding 0。
- `python -X utf8 tooling/manifest.py --write`／`--check`、`python -X utf8 tooling/release_gate_check.py`: 合格。
- 最新`origin/main`の変更前commit `686a2a4e3e043d79e07b6d2bca248bcfccdfd18a`から作成した非OneDrive managed worktreeへ同じDart差分を適用し、`flutter test test/history_screen_test.dart test/history_content_test.dart`: 18件すべて成功。期限timer発火後の表示消去を含む。これは対象画面と共有clientのtestであり、Desktop製品全体の試験ではない。
- OneDrive内の元checkoutで同じFlutter testを起動した場合は、Flutterが`apps/desktop_flutter/macos/Flutter/ephemeral/Packages/.packages`を削除できず終了した。ACLや生成物は変更・削除していない。
- Desktop／Mobileの`flutter analyze`と両appの`dart analyze`は、分析サーバー異常終了でexit 1。`dart analyze`は`%LOCALAPPDATA%\\Dart\\perf\\<pid>`の削除でOS error 1920となり、対象は`ReparsePoint`でACL照会・reparse照会も同じOS errorになった。Flutter analyzeはLSP初期化JSONの途中終了を報告した。類似するDart SDKの公開報告（[dart-lang/sdk #63343](https://github.com/dart-lang/sdk/issues/63343)）があるが、本環境の原因が同一とは断定しない。OS policy・ACLの回避は行っていない。
- この局所変更でPhase 9完了やrelease readinessを主張しない。登録済みrelease blockerと`release_ready=false`を維持する。

## D4 Pocket Phase 7 Task用Windows sandbox方式の明示（2026-09-28）

現行Codex CLI `0.158.0-alpha.2.1`とTask command構成を照合した。Task commandは`--ignore-user-config`を指定する一方、Windows sandbox方式をCLI overrideで固定していなかった。現環境のCodex user configには`[windows] sandbox = "elevated"`があるが、このTask commandはその設定fileを読まないため、製品Taskが同じ強い方式を選ぶことは保証されていなかった。

Task限定のconfig overrideへ`windows.sandbox="elevated"`を追加し、Task起動時に`-c`で明示する。read-only Dialogueは引き続き既存経路を使い、このoverrideを付けない。権限profile本体・Authority経路・Adapter metadataは変更しない。

- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib Dialogueはread_onlyのままTaskだけ専用permission_profileを使う -- --test-threads=1`：追加後の再実行で1件成功。
- `python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`：Task経路にelevated方式の固定値があることを含む全225件が合格。
- `codex --version`：`codex-cli 0.158.0-alpha.2.1`。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`：lib 321件、main 9件、Broker IPC 10件、他integration 15件（計355件）が成功した後、`workspace_diff` test executableがWindows Application Control（OS error 4551）で起動前にblockされ、command全体は終了code 1。`cargo check --all-targets`とfocused Codex Adapter testは別途成功。前回の全target 366件成功記録は履歴として保持し、今回の実行をPASSへ昇格しない。

この変更はsandbox方式の選択を明示するだけで、deny-read ACLが実効する証拠ではない。permission profile付きhelperでは合成`.env`の読取成功と外部path deny適用失敗を既に観測しており、`codex exec` Task実動作も未検証である。実環境の否定・許可対照試験を通過するまで`task_execution=unsupported`とrelease blockerを維持する。

## D4 Pocket Phase 7 Broker強制終了後scratch回復の別プロセス試験（2026-09-28）

直前の同一test process内Broker再生成試験を強化し、Rust test harnessの子process内で永続Brokerを起動し、Runtime／Workspace登録とscratch journalの予約・有効化後に準備完了PIDを通知させ、親processから子Broker processを強制終了する試験へ変更した。親は同じ永続storeを使う新Brokerで`作業領域起動登録`を実行し、実scratch directory削除、journal記録解消、回復AuditEventを確認する。試験終了時にはfixture storeとWorkspaceも削除する。

- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib broker強制終了後の別process起動登録で永続scratchを監査付き回収する -- --nocapture --test-threads=1`：1件成功。
- `cargo check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`：成功。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`：exit 0。library 321、CLI 9、Broker IPC 10、integration 26の計366件が成功。Desktop launcher test targetは0件で正常起動。
- `python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`：225件合格。Conformanceはprocess-boundary試験の存在・子process起動・明示killを静的に検査する。

この試験は別OS processの強制終了と永続storeを介した実Broker library／filesystem回復を検証する`FIXTURE`であり、production `broker-server`／IPC listenerを起動しての回復、実Agent Task、電源断、製品導入後のcleanupを証明しない。AuditEvent内の`LIVE_RUNTIME`分類値も、この試験全体の証拠分類を変更しない。`task_execution=unsupported`、Agent Taskの`release_blocker`、`release_ready=false`を維持する。

## D4 Pocket Phase 7 Broker再起動後のscratch回収接続試験（2026-09-28）

永続journal単体試験に加え、最初の`Broker`でRuntime／Workspaceを登録してAgent Task scratchをjournalへ予約・有効化し、同一永続storeから新しい`Broker` instanceを生成して、実際の`作業領域起動登録`経路で回収するRust試験を追加した。試験はscratch directoryの削除、journal記録の解消、回復AuditEventを確認する。Conformanceにも、起動時回復呼出しがIPC listener bindより前にあることの接続順検査を加えた。

- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib -- --test-threads=1`: 321件合格、失敗0件。
- `cargo check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`: 成功。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`: exit 0。library 321、CLI 9、Broker IPC 10、integration 26の計366件すべて成功。desktop launcher targetは0 testsで正常起動した。

これは同一試験プロセス内で`Broker`を再生成し、永続store・実filesystem・Brokerの起動登録経路を通した試験である。Windows上でBrokerプロセスを強制終了した場合や電源断後の回復を示す`LIVE_RUNTIME`証拠ではなく、Agent Taskの製品環境での一時領域後始末保証は未成立。過去の`Windows Application Control`／`OS error 4551`による起動拒否は履歴として保持するが、最新コードのRust全対象試験関門は解消済み。`task_execution=unsupported`、Agent Taskの`release_blocker`、`release_ready=false`を維持する。

## D4 Pocket Phase 7 Broker crash後scratch回復の実装（2026-09-28）

Codex Agent Taskのscratch directoryをBroker再起動後に限定回収するため、HMAC認証・bounded永続journal、`reserved`／`active`状態、stable `recovery_binding_hash`、Workspace登録後かつIPC listener開始前のreaperを追加した。権限用の揮発registration hashは再起動ごとに変化するためjournalへ流用せず、recovery bindingをRuntime／Workspace ID、secret除外指定、rootと祖先のdevice/file identityから再計算する。削除条件は現在登録とのbinding一致、root identity一致、scratch直接子名の固定形式、nofollow open、active recordの実directory identity一致である。予約だけでidentity未確認、別登録、reparse、identity不一致、破損journalは削除せず保持し、Workspaceに未解決記録がある間はAgent Taskを拒否する。本文、absolute path、Credential、Permission、Approval、出力はjournalへ保存しない。

- `python -X utf8 tooling/schema_check/check_schemas.py`: 成功。Schema 145、正常例145、negative fixture179。
- `python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`: 成功。225 checks。
- `python -X utf8 tooling/日本語基底監査.py --strict`: 成功。負債file 0、finding 0。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`: exit 0。登録済み10検査が成功。これは開発検証であり、release_ready=false、release blocker 31件を維持する。
- `cargo check --all-targets`: 成功。
- `cargo test --lib broker::agent_task_scratch::tests -- --test-threads=1`: 7件成功。永続journal再読込・一致identity回収、binding／root identity不一致の保護、reserved path保護、scratch名再利用後のidentity不一致保護、HMAC改竄拒否、消失path記録の照合解消を検査。
- 最新`cargo test --all-targets -- --test-threads=1`: library 320件は全件成功。その後`gui_shell_desktop_launcher` test executableがWindows Application ControlのOS error 4551で起動前に拒否され、全targetは未完了。これより前の実行も別targetの起動拒否で停止しており、成功へ読み替えない。

7件のunit testと225件のConformanceは局所実装／Contractの証拠で、Windows上のBroker強制終了／電源断を伴う`LIVE_RUNTIME` cleanup証拠ではない。Codex metadata `task_execution=unsupported`、`release_ready=false`、release blockerを維持し、有料資格による実Agent実行はしていない。

## D4 Pocket Phase 7 Windows permission profile deny-read実測失敗（2026-09-28）

前節で組み立てた`d4p-agent-task`をCodex sandbox helperへ明示指定し、秘密値を含まない合成marker `D4P_DENY_PROBE=NON_SECRET_SYNTHETIC_MARKER`を`tooling/.codex-profile-probe/probe.env`へ置いて読取拒否を検証した。OneDrive配下の製品checkoutと`C:\Users\ohira\.codex\worktrees\windows-release-clean\D4ポケット`の非OneDrive managed worktreeの両方で、`cmd.exe /d /c type ...probe.env`がmarker本文を出力しexit 0となった。検証後、両markerは削除した。

- 非OneDrive側のfile attributesは`Archive`のみでCloud Files reparse pointではなかった。`icacls`は`CodexSandboxUsers`の継承`Modify`／`Read & execute`許可の後ろに継承`DENY(Read)`を示した。helper childはsandbox user `intelitin\codexsandboxoffline`だったが実際の読取は成功した。ACE順序が読取成功に関与するという説明は推定であり、OS access checkの因果は確定していない。
- `C:\Windows\win.ini`をexact denyにした同helper probeは、読取前に`windows sandbox failed: helper_unknown_error: apply deny-read ACLs`で終了した。過去のprofile付きhelperから同fileを読めた結果と合わせ、外部path拒否は成立していない。
- これはCodex sandbox helperの実行証拠であり、`codex exec`のproduction Agent TaskやBroker経路の証明ではない。有料資格・model実行は使用していない。profile denyの効果が現環境で成立しなかったため、Codex Adapter `task_execution=unsupported`を維持し、このprofileをTaskの安全境界として扱わない。
- 次の検証はdeny ACL生成・適用順序の原因を特定し、OneDrive Cloud Filesと通常NTFS双方で`.env` marker拒否、外部read拒否、workspace内の許可操作をLIVE_RUNTIMEで対照確認すること。Broker統治Task、cleanup、失敗Recovery、実Agent差分表示も未成立で、各々`release_blocker`のまま。

## D4 Pocket Phase 7 Codex Task専用permission profile（2026-09-28）

Codexの新permission profile仕様（[公式Permissions文書](https://learn.chatgpt.com/docs/permissions)）を使い、既存のread-only Dialogueへ影響させずTask commandだけへ固定`d4p-agent-task` profileを与えるcommand構成を追加した。profileは`:workspace`を継承し、glob走査深度8までのWorkspace内`**/*.env`、`**/.ssh/**`、`**/secrets/**`をdeny、networkを無効にする。深度8超や別名のsecret path全体を包括的に拒否するものではない。従来のTask TEMP/TMP scratchとJob Object process群監督は維持する。

- `codex --help`／`codex exec --help`：global `-c key=value` config overrideとTaskに必要なJSONL、ephemeral、user-config無視optionを確認した。
- `codex sandbox --permission-profile d4p-task ... cmd.exe /c type C:\Windows\win.ini`：同等の`:workspace`継承・`.env` deny glob設定を与えたsandbox helperがprofileを受理し、外部system fileの内容を出力した。これはhelperのprofile読み込みと外部read可を示すだけで、`codex exec` production Task経路やworkspace deny glob実効性の証拠ではない。
- Rust command構成testとConformanceに、Taskだけへprofileを与えること、Dialogueのread-only維持、secret deny pattern、network無効、sandbox bypassの不在を検査する条件を追加した。実Codex `exec`、secret marker読取拒否、workspace書込／外部read境界は未実行。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib Dialogueはread_onlyのままTaskだけ専用permission_profileを使う -- --test-threads=1`: 成功。続く`--all-targets`はlibrary 313件、CLI 9件、Broker IPC 10件が成功した後、次のintegration test executable `canonical_decimal_hash-091b16c7c8c9c9d6.exe`がWindows Application Controlに起動前拒否され、exit 1。policy変更・実行file移動等の回避はしていない。
- 実モデル呼出し・有料資格は使用していない。Codex Adapter metadataは`task_execution=unsupported`のまま。Workspace外read、deny glob実効性、Broker crash後scratch recovery、実Agent failure／cancellation／deadline、結果・diff Content Exposureは`release_blocker`として維持する。

## D4 Pocket Phase 7 Codex Agent Taskの固定workspace-write Adapter（2026-09-28）

実インストール済みCodex CLI `0.158.0-alpha.2.1`で`codex exec --help`を確認し、`--json`、`--ephemeral`、`--ignore-user-config`、`--sandbox workspace-write`、`--cd`とstdin promptの実物interfaceを固定した。Adapter登録は従来read-only Dialogueに必要なexec optionだけを検査し、`workspace-write`の有無はTask対応probeとして別に記録する。古いCLIがTask optionを持たなくてもread-only Dialogueの登録を妨げない。既存Dialogueは引き続き`read-only`で起動し、Task専用経路だけを`sandbox=workspace-write`にする。`--add-dir`、worktree切替、dangerous bypass、任意command／pathは追加していない。

Task呼出し前に登録済みWorkspaceの実体を再照合し、OS乱数名のTEMP/TMP scratch directoryを同Workspace直下に作る。rootとscratchをnofollowで開きWorkspace identityを確認してからchild環境のTEMP/TMPだけをscratchへ上書きし、Windows Job Object監督下でCodex CLIを実行する。JSONLは上限付きで読み、完了eventとfinal agent messageが揃わなければ失敗とする。正常終了・通常error・取消・期限超過後はprocess群停止を要求し、open scratch directoryを削除する。cleanup失敗は成功へ昇格しない。

`workspace-write`はWorkspace内の`.env`等の秘密fileをAgentから読めなくする契約ではない。秘密fileの読取除外・拒否を実測する機構は未成立であり、実Taskは有効化しない。またscratch削除はBrokerが稼働している通常の終了経路だけで、Broker crash／強制終了／電源断後に残るscratchを回収するreaperや再起動時Recoveryはない。これらは明示的な`release_blocker`であり、OS Job Objectによるprocess停止をfilesystem cleanupの証拠へ昇格しない。

Rust Adapter実装はWindowsに限りTask起動関数を持つが、`task_execution` capability metadataは`unsupported`を維持するためBroker consumerは起動しない。model／課金資格は使わず、実Codex Agent Task・Broker経由のworkspace書込み・外部path拒否を実行していない。Broker crashまたは電源断後のscratch cleanup、実Agent sandboxの全失敗経路、差分／結果表示とContent Exposure、cross-agent contaminationは未成立の`release_blocker`として残す。Codex direct sandbox helperでの過去probeは本Task実装や`codex exec` Broker pathの証拠へ転用しない。

- `codex --version`：`codex-cli 0.158.0-alpha.2.1`。`codex exec --help`のversion／option観測はinterface範囲のみ。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`：358件合格（library 313、CLI 9、Broker IPC 10、その他のintegration 26）。Windows scratch明示cleanup／Drop cleanup、fixed sandbox option、外部TEMP拒否、help option gateを含む。
- `python -X utf8 tooling/schema_check/check_schemas.py`：Schema 144件、正常example 144件、negative fixture 178件で合格。`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`：224 checksで合格。
- `python -X utf8 tooling/日本語基底監査.py --strict`：初回はCodex helpを模したRust test fixtureの英語だけの固定option列1件を検出。fixtureへ日本語の用途表示を加え、再実行で負債file 0、finding 0。追加修正時、UTF-8日本語をbyte stringへ直接記述して`rustfmt --check`が一度失敗したため、UTF-8文字列から`.as_bytes()`を渡す形へ修正。修正後は`rustfmt --edition 2021 --check native/rust_helper/src/adapters/codex_cli.rs`、`cargo check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`、対象Codex interface test 1件が合格。
- 実Model呼出しと外部pathを含むAgentの試験は未実行。能力宣言は`unsupported`のまま維持し、release blockerを閉じない。

## D4 Pocket Phase 7 Windows Codex process群の終了管理基盤（2026-09-28）

Codex Adapterの既存read-only対話processを、Windowsでは初期thread停止中に専用Job Objectへ割り当ててから再開する。Jobは`KILL_ON_JOB_CLOSE`を設定し、通常の取消・期限超過・異常時はprocess群の停止と終了確認を行う。Root process終了時も残存processを停止してからpipe readerを回収し、Broker異常終了ではOSによるJob handle closeを最終停止境界とする。Win32 unsafe呼出しは独立したWindows専用`native/process_supervision` crateへ閉じ、Broker crateの`#![forbid(unsafe_code)]`を保持する。

- 明示unsafe例外レビューと適用範囲は`docs/specs/process-supervision.md`および`docs/specs/agent-runtime.md`へ固定し、Conformanceでunsafe呼出しを`native/process_supervision/src/windows_job.rs`一fileへ限定する。
- `cargo test --locked --manifest-path native/process_supervision/Cargo.toml --target-dir native/rust_helper/target/process_supervision -- --nocapture`：Windows実processを使う2件が合格。Broker相当ownerを強制終了した後にJob内childが終了することを確認した。証拠はWindows OS process管理の`LIVE_RUNTIME`であり、実Codex CLIやTask実行の証拠ではない。
- `cargo test --locked --all-targets cancellation_terminates_the_supervised_child`：Adapter側process監督接続のWindows実process試験1件が合格。
- `cargo check --locked --all-targets`：成功。
- `cargo test --locked --all-targets`：355件合格（library 310、CLI 9、Broker IPC 10、その他integration 26）。
- `python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`：224 checksで合格。process supervision unsafe例外の範囲、Broker禁止、SAFETY根拠数と契約文書を検査する。
- `python -X utf8 tooling/schema_check/check_schemas.py`：Schema 144件、正常example 144件、negative fixture 178件で合格。`python -X utf8 tooling/日本語基底監査.py --strict`：負債0／finding 0で合格。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`：exit 0。`manifest`、release gate、packaging portability、release smoke等のdevelopment検査を通過。製品状態は`release_ready=false`、既存release blocker 31件を維持する。
- これはread-only対話Adapterのprocess管理基盤であり、Codex Adapterの`task_execution=unsupported`は維持する。書込Task consumer、Workspace scratch隔離・cleanup、実Agent起動、隔離書込、Task向けLIVE_RUNTIME failure injectionは未接続で、既存`release_blocker`を維持する。Job Objectはprocess群管理であり、filesystem/network sandboxではない。

## D4 Pocket統合 Phase 7 作業TaskのBroker実行経路（2026-09-28）

Rust Brokerに独立した`AgentTask実行`／`AgentTask状態`／`AgentTask取消`経路を追加した。実行開始では、構造検査済みAdapter metadataとRust Adapter実装の双方がTask対応を示すこと、現行Session、Workspace登録hash、Adapter固定root identity、Task本文hash、Owner Approvalの実行条件hash、およびWorkspace Permission／Owner Approvalの壁時計・単調時計期限を同一Broker排他区間で再照合する。開始Auditを確定した後、揮発PermissionとApprovalを不可分に取り除いてからworkerを起動し、同じgrantの再利用を拒否する。Owner Approvalの有効期間は発行後5分、Taskは開始後15分を上限とする固定実行policyをnative確認文とhash結合条件へ明示した。

Task状態は版2の固定label・Runtime／Session／Workspace・Broker計算指示hash・Audit参照・任意result hashだけを投影する。worker出力本文はBroker側でhash化後に破棄し、監査本文・状態・errorへ保存しない。取消要求後または期限後に返った成功応答もBrokerで拒否する。Taskは同一Session／Workspace物理rootにつき同時1件、Broker全体4件、Task状態record合計128件を上限とし、取消要求を受けてもAdapterの停止応答前にterminal状態へしない。Session終端・資格隔離・Broker Drop時はCancellationを要求するが、flagだけではOS process群の停止証明にならない。現在のCodex AdapterはこのTask経路を実装せずmetadataも`unsupported`のままなので、production実行されない。

Schema／IPC／ConformanceにTask ID要求と実行・状態・取消操作を登録した。Workspace差分・結果本文UI、Codexの`workspace-write`実行、Workspace内TEMP／TMP専用scratchと全終端cleanup、OS process-tree supervision、実Agent実行、別Agent間隔離、LIVE_RUNTIME失敗注入は引き続き`release_blocker`であり、Consumerの`FIXTURE` testやcompileをsandbox・製品実行の証拠へ昇格しない。

- `cargo check --locked --manifest-path native/rust_helper/Cargo.toml --target-dir C:\D4Pocket-agent-task-test-target --all-targets`：成功、warningなし。testを含む全Rust targetのcompile確認。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --target-dir C:\D4Pocket-agent-task-test-target --lib AgentTask -- --test-threads=1`：9件合格。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --target-dir C:\D4Pocket-agent-task-test-target --all-targets -- --test-threads=1`：初回は308件中、取消fixtureがworker開始前に取消した競合で1件失敗。fixtureをworker開始後に取消するよう同期し、focused 9件と全target計353件（library 308、CLI 9、Broker IPC 10、その他integration 26）が再実行ですべて合格した。過去の別実行でWindows Application Controlに拒否された履歴は各時点の記録として保持し、今回成功で遡及上書きしない。
- `python -X utf8 tooling/schema_check/check_schemas.py`：Schema／正常example 144件、negative fixture 178件で合格。
- `python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`：223 checksで合格。
- `python -X utf8 tooling/日本語基底監査.py --strict`：負債file 0、finding 0で合格。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`：初回はManifest生成時に新規fileがGit indexへ未登録だったためManifest checkと依存検査3件が失敗。staged source 1065件でManifestを再生成した後の再実行はexit 0で登録済み10検査すべて合格。これはdevelopment検査であり、`release_ready=false`とCONFIG／FIXTURE証拠境界を維持する。
- Codex CLI `--version`／`exec --help`で現行の`workspace-write` optionを確認した。モデルは起動せず、課金・外部Agent起動・Workspace書込試験は行っていない。このinterface確認は実Task隔離の証拠ではない。
- release blockerは維持し、`release_ready=false`。残作業・失敗・検証範囲は最新正本と次回検証時に再照合する。

## D4 Pocket Phase 7 Windows Codex sandboxの作業領域境界probe（2026-09-28）

installed `codex-cli 0.158.0-alpha.2.1` の `codex sandbox --permission-profile :workspace` を使い、モデルを起動せず、固有名を付けたscratch fileの作成可否をWindowsで実測した。通常processとsandbox childは異なるWindows userで実行され、現在のRepository root内のfile作成は成功し、`LOCALAPPDATA`下の別scratch rootへのfile作成は`UnauthorizedAccessException`で拒否された。これはCodex CLI sandbox helperの`LIVE_RUNTIME`証拠であり、D4 Broker経由のAgent Taskや`codex exec` production pathの証拠ではない。

標準`:workspace` permission profileはOS Temp directoryへの書込みも許す。TEMP/TMPをprocess-scopedでWorkspace内の専用scratch directoryへ向けた再試験では、そのdirectory内の作成が成功し、元のTemp siblingへの作成は拒否された。したがって将来のTask launchは、Owner Approval後のTask専用scratchをWorkspace内に作り、child processのTEMP/TMPをそこへ限定し、終了・失敗・取消時に監査可能なcleanupを行う必要がある。標準profileだけを指定してsandbox境界がWorkspaceへ完全限定されたとは扱わない。

- `codex --version`: `codex-cli 0.158.0-alpha.2.1`。
- `codex sandbox --help`／`codex exec --help`: 現行CLIが必要とするpermission profileと `workspace-write` sandbox optionを確認。modelは起動していない。
- `codex sandbox --permission-profile :workspace --cd <Repository> powershell.exe ...`: Repository root内marker作成成功、`LOCALAPPDATA`下の別rootへの作成拒否、sandbox child userの相違を確認。
- 同sandbox helperをprocess-scoped TEMP/TMPで再実行: Workspace内Task scratchへの作成成功、元Temp siblingへの作成拒否。
- machine config、ACL、Windows policyは変更していない。試験用Repository markerは除去済み。試験専用Temp scratchの削除commandはCodex実行環境のcommand policyに拒否され、`%TEMP%\d4pocket-sandbox-probe-20260928` と空の `%LOCALAPPDATA%\D4PocketSandboxProbe-20260928` が残存する。
- `release_blocker`: `codex exec` AdapterのTask実行、TEMP/TMP分離とcleanupのproduction接続、Permission／Approvalの実行直前原子的再検証・一回消費、実行前後Audit／Recovery、Agent Task結果/diff保存および実Agentの隔離試験は未成立。`release_ready=false`を維持。

## D4 Pocket Phase 7 AgentTask実体Workspace照合（2026-09-28）

AgentTaskの要求検査、Workspace Permission発行、Owner Approval発行で、Broker登録Workspaceの物理root識別子とAdapterが固定したroot識別子を照合する。識別子の欠落・不一致は拒否し、同じWorkspace IDの宣言だけでは代替できない。Codex Adapterは起動時に捕捉したroot識別子を返すが、Task実行能力は引き続き`unsupported`である。この照合は`INTERNAL_STATE`の束縛確認であり、OS sandbox、実書込隔離、Task実行の証拠ではない。

- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --target-dir C:\\D4Pocket --lib -- --test-threads=1`: 最終実行で305件成功。実root不一致時の要求・Permission・Approval拒否と一致時のPermission発行を含む。先行suite実行ではMINIDORA通信testが一度`通信失敗`となったが、単独再実行と次の全library suiteで成功した。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --target-dir C:\\D4Pocket --all-targets -- --test-threads=1`: Rust library 305件、CLI unit 9件、Broker IPC integration 10件は成功。その後`canonical_decimal_hash`のtest executable起動をWindows Application Controlが`OS error 4551`で拒否し、全体は未完了。OneDrive外の別target pathではlibrary test executable自体も同errorで起動拒否された。OS policyは変更していない。
- `cargo check --locked --manifest-path native/rust_helper/Cargo.toml --target-dir C:\\D4Pocket --all-targets`: 成功。
- `python -X utf8 tooling/schema_check/check_schemas.py`: Schema／正常example各143件、negative fixture 177件で合格。`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`: 222 checksで合格。`python -X utf8 tooling/日本語基底監査.py --strict`: 合格。
- `python -X utf8 tooling/manifest.py --write`／`--check`、`python -X utf8 tooling/release_gate_check.py`、`git diff --check`: 合格。
- `cargo fmt --manifest-path native/rust_helper/Cargo.toml --all --check`: 失敗。既存ファイルを含むworkspace全体に未整形差分が多数あり、一括整形による無関係変更は行わない。変更箇所の新規整形差分は個別確認して修正。
- `release_blocker`: AgentTask実行Consumer、実証済みsandbox／隔離Workspace、実行直前の原子的再検証と一回消費、結果・diff保存、実行前後Audit／Recoveryは未成立。`release_ready=false`を維持。

## D4 Pocket Phase 7 AgentTask版2状態Contract（2026-09-28）

既存の未versioned `agent_task.schema.json`を壊さず、`record_version=2`の状態projectionを追加した。版2はBroker計算の指示hashとAgent Runtime／Session／Workspaceを必須結合し、監査参照・Task状態・任意のresult hashだけを持つ。旧shapeは履歴互換として受理するがRuntime／Workspace結合や実行の証拠には使わない。permission／approval ID、instruction本文、Agent出力、command、filesystem path、Credential実値はSchemaで拒否し、descriptionは非機密の固定labelに限定する意味を日本語正本へ追加した。ただしSchemaは自由文字列の機密性を判定しないため、将来Consumerが固定label以外を生成しない保証は未実装。

- `python -X utf8 tooling/schema_check/check_schemas.py`: Schema・example各143件、negative fixture 177件で合格。
- `python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`: 222件のcheckが合格。版2の必須結合field、旧shape履歴互換、禁止情報、状態enum、hash形式、label上限を検査。
- `python -X utf8 tooling/日本語基底監査.py --strict`: 負債file 0、finding 0で合格。
- `python -X utf8 tooling/manifest.py --write`／`--check`、`python -X utf8 tooling/release_gate_check.py`、`git diff --check`: 合格。manifest更新前のrelease gateは変更file hash不一致で失敗し、再生成・再検査後に合格した。
- Schema／fixture／Conformanceは`FIXTURE`範囲。Rust Broker runtime consumer、実Task起動、one-shot Permission／Approval消費、隔離、実行前後Audit／Recovery、出力・diff保存は今回未実装・未検証。
- `release_blocker`: Agent Task実行Consumerと実証済み隔離を含むPhase 7の残作業。`release_ready=false`を維持。

## D4 Pocket Phase 7 Windows Flutter検証のOneDrive ACL分離（2026-09-28）

cleanな`main`／`origin/main` commit `917e075d8b151b48ee465e6a6fa16755cd370d43`のtracked sourceを`git archive`でOneDrive外の短縮scratchへ展開し、Flutter Desktop／Mobileの解析とtestを実行した。元OneDrive checkoutでは両appの`flutter analyze`が`Flutter/ephemeral/Packages/.packages`の削除拒否で失敗した。該当fileの継承ACLに`CodexSandboxUsers`のread／delete denyおよび`Everyone`の子directory削除denyを観測した。ACLは変更していない。

- Clean source archive: `C:\D4Pocket-agent-task-test-target\validation\flutter-917e075.zip`、SHA-256 `F62F04478AA3D9B6D987999F845F55E02B39E00D42BE3F0FFD901345E1FDA429`。source commitは上記`917e075`。OneDrive外のscratchで`flutter analyze`はDesktop 14.7秒、Mobile 7.8秒、双方`No issues found`。
- Mobile: `flutter test`は18件すべて成功。
- Desktop: clean archiveだけで行った初回`flutter test`は、`native/rust_helper/target/debug/gui_shell_rust_helper.exe`未buildを前提とする2 integration testが失敗し、+107件時点で停止した。scratch内で`cargo build --locked --manifest-path native/rust_helper/Cargo.toml --bin gui_shell_rust_helper`を成功させた後、`flutter test`を再実行し113件すべて成功。実行されたRust helperは当該clean source commitから生成。
- `git status --short --branch`は作業開始・終了時ともclean。OneDrive内Flutter解析のACL失敗は再現済み`known_limitation`として扱い、scratch検証PASSを元pathのACL修復やinstalled productの証拠へ昇格しない。`rev2_flutter_broker_channel_boundary`、Windows installed smokeおよびrelease gateは未解除。

## D4 Pocket Phase 7 Windows Broker smokeのLIVE_RUNTIME再確認（2026-09-28）

cleanで`origin/main`と一致するsource commit `25978d20b57ef0ac34cf4fe3c10aed3104891511`からRust Broker Release helperをビルドし、`installer/windows/collect_broker_smoke.ps1`を分離scratch store／session fileへ実行した。Buildは成功し、`native/rust_helper/src/adapters/minidora.rs`の未使用`C28_DIAGNOSTIC_LIMIT`／`c28_route_label` warning 2件が出た。collectorはexit 0、status `passed`であり、authenticated loopback IPC、durable store、再起動後の同一nonce拒否（`broker_replay_detected`）、新規health受理、強制終了後の接続拒否、一時session資格fileの作成・削除を実Broker processで観測した。

- Build: `cargo build --locked --manifest-path native/rust_helper/Cargo.toml --target-dir C:\\D4Pocket-agent-task-test-target --release --bin gui_shell_rust_helper`。出力helper SHA-256は`F10DD9BD0124169DC9EC176E09732B8BAD2817AE37BCAF619CB93E9DD27D2164`。
- Collector: `powershell.exe -NoProfile -ExecutionPolicy Bypass -File installer\\windows\\collect_broker_smoke.ps1 -BrokerHelperExe C:\\D4Pocket-agent-task-test-target\\release\\gui_shell_rust_helper.exe -OutputPath C:\\D4Pocket-broker-smoke-25978d2\\windows_broker_smoke.json -StoreDir C:\\D4Pocket-broker-smoke-25978d2\\store -SessionFile C:\\D4Pocket-broker-smoke-25978d2\\broker_session.json`。結果JSON SHA-256は`2D95894E2E73EFE5977A6DAA3F8F749E815EBF91F66E54F1A08591C440434071`。session fileはcollector終了後に不在。
- Evidence: Broker単体processの`LIVE_RUNTIME`とsession file削除の`EXTERNAL_EVIDENCE`。この結果はRust起動器、Flutter child、installed product、別Windows user profile、正式配布の証拠ではなく、`windows_broker_installed_smoke`／`rev2_flutter_broker_channel_boundary`のrelease blockerを解除しない。結果と生成storeはOneDrive外の`C:\\D4Pocket-broker-smoke-25978d2`へ隔離して保持した。

## D4 Pocket Phase 7 Task ApprovalのWorkspace登録hash差替負例（2026-09-28）

Rust Broker対話制御のFIXTURE testへ、同じRuntime／Session／Workspace IDでも登録hashが変わった場合、既発行Workspace PermissionとOwner Approvalをpreflightで再利用できない負例を追加した。発行時のhashなら両状態が有効、異なる登録hashを渡すとSession不一致で拒否される。production挙動は変更していない。この試験は内部状態の境界検査であり、実Registry差替、原子的Task消費、Task実行、LIVE_RUNTIME隔離の証拠ではない。

- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --target-dir C:\\D4Pocket-agent-task-test-target --lib AgentTaskOwnerApprovalはWorkspace登録hash差替後に再利用できない -- --test-threads=1`：初回は試験callback closureのlifetime推論でcompile失敗。callbackを各呼出しの局所closureへ変更後、再実行で1件成功。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --target-dir C:\\D4Pocket-agent-task-test-target --all-targets -- --test-threads=1`：exit 0。Rust library 304件、CLI unit 9件、Broker IPC integration 10件、その他integration 26件、計349件成功。試験生成物はOneDrive外の固定短縮pathへ出し、sourceはこのcheckoutから読んだ。
- `python -X utf8 tooling/schema_check/check_schemas.py`：Schema／正常example各143件、negative fixture 177件で合格。`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`：221 checksで合格。
- `python -X utf8 tooling/日本語基底監査.py --strict`：負債file 0、finding 0で合格。`python -X utf8 tooling/manifest.py --write`／`--check`、`python -X utf8 tooling/release_gate_check.py`、`git diff --check`も、この単位の最終差分で合格。
- `release_blocker`: Task実行Consumer、Workspaceの実書込隔離、実行直前の原子的Permission／Approval再検証・一回消費は未成立。

## D4 Pocket Phase 7 Rust全target Windows検証の再確認（2026-09-28）

cleanな`main` commit `2da5fd370382f2fe5acc032b8f26ac25880048ee`と一致する`C:\D4Pocket` checkoutでRust全targetを再実行し、全件成功した。試験開始時のbranchは`main`、working treeはcleanで、`origin/main`も同commitだった。

- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`（`C:\D4Pocket`）：exit 0。Rust library 303件、CLI unit 9件、Broker IPC integration 10件、その他integration 26件、計348件が成功。
- Broker IPC process起動試験とWorkspace startup 7件を含む全integration targetが実行された。過去にOS error 4551で起動拒否された試験targetも今回成功した。過去の失敗記録は削除せず、履歴として保持する。
- 証拠源は当該checkout上のRust試験（ローカル一時状態を用いる`FIXTURE`を含む）。この結果は同checkoutのRust test executable実行を確認するもので、OneDrive checkoutのpath長／同期挙動、Desktop Flutter全suite、installed product、実Agent／Task、他OS、release readinessを証明しない。
- `windows_rust_integration_test_execution_policy`は、Rust全target試験実行阻害について解消とする。OneDrive固有のpath／同期制約および製品release blockerは別範囲として維持する。

## D4 Pocket Phase 7 Agent Task未対応Runtimeへの権限発行拒否（2026-09-28）

Rust BrokerのTask要求検査、Task Workspace Permission発行、Task Owner Approval発行に、構造検査済みAdapter metadataの`task_execution=supported`を必須とするfail-closed gateを追加した。capabilityが欠落、`unknown`、`unsupported`の場合は`AgentTask実行非対応`で拒否する。Adapter metadataは拒否条件としてだけ参照し、Permission／Approval／Trustを付与しない。Codex Adapterの現行宣言は`unsupported`のため、既知のread-only経路へTask用Owner権限を発行しない。対応を宣言する試験AdapterはBroker権限経路試験専用であり、実Task実行・隔離の証拠ではない。

- capability gate導入直後の最初のAgent絞込testは、Task Permission発行用試験Adapterが`supported`を宣言していなかったため1件失敗した。試験Adapterの責任範囲を明示する宣言を追加し、`cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib Agent -- --test-threads=1`を再実行して14件成功した。製品Adapterの能力宣言は変更していない。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`: 成功。Rust library 303件、CLI unit 9件、Broker IPC 10件、他のintegration test 26件（計348件）が成功した。前節に残す過去のWindows Application Control拒否記録は履歴として保持し、今回の短い`C:\D4Pocket` checkoutにおける全target成功と混同しない。
- `python tooling/schema_check/check_schemas.py`: Schema／正常example各143件、negative fixture 177件で成功。`python tooling/conformance_tests/run_conformance_skeleton.py`: 221件成功。
- `python -X utf8 tooling/日本語基底監査.py --strict`: 負債file 0、finding 0で成功。`python -X utf8 tooling/manifest.py --write`（1062 file）／`--check`、`python -X utf8 tooling/release_gate_check.py`、`git diff --check`も成功。Flutter／Mobile／実Agent・Task実行は今回変更対象外で未実行。
- Git: `main`の実装commitは`124f23aa1dff9cd462f656868c090b12d9b673c4`、push成功。push直後のremote `main`は同hash、作業ツリーはcleanだった。remote backup tagは`codex/backup-main=124f23aa1dff9cd462f656868c090b12d9b673c4`、`codex/backup-main-prev=a523ebbf36ef3c4dc4e899f519614761253f3d1f`で、両hashをremote照合した。rollback pointは変更前の`a523ebbf36ef3c4dc4e899f519614761253f3d1f`。
- `release_blocker`: Task実行Consumer、一回消費と実行直前の原子的再検証、実証済みsandbox／隔離Workspace、実行前後Audit／Recovery、結果/diff、比較／Handoff、Windows実Broker子process等は未成立。能力gateの試験は`FIXTURE`であり、実Task／release readinessの証拠ではない。

## D4 Pocket Phase 7 Rust全target Windows検証の追補（2026-09-28）

`b12aeb8e2ecf852bd2f5f8341f16355e6e711e0a`で全targetを実行した。Rust library 302件とCLI unit 9件は成功した。一方、`broker_ipc` integration 10件のうち1件は成功、9件は失敗した。失敗testは共通してBroker子processの`Command::spawn()`でWindows Application Control `OS error 4551`を受け、対象IPC挙動へ到達していない。integration test sourceの該当spawn箇所と例外を照合した。

- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`: exit 1。302 library + 9 main unit合格、`broker_ipc`は1合格／9失敗。9失敗はtest executableからBroker processを起動できない同一host制約。
- `Microsoft-Windows-CodeIntegrity/Operational`のevent 3077／3033を確認。`broker_ipc-…exe`が生成した`gui_shell_rust_helper.exe`はPolicy ID `{0283ac0f-fff1-49ae-ada1-8a933130cad6}`のEnterprise signing levelを満たさず拒否された。これは試験hostの実行制約の特定であり、Broker IPC挙動の実測ではない。
- 実行fileの移動、Application Control弱体化、別実行pathによる回避は行っていない。失敗を製品回帰の証拠にはしない一方、process境界のLIVE_RUNTIME検証を合格へ昇格させない。
- Cargoは`broker_ipc`失敗後に終了したため、後続の全integration targetが実行済みとは扱わない。`--lib` 302件の成功はBroker子processの起動・IPC統合証拠を代替しない。
- `release_blocker`: Windows実Broker process、認証IPC、再起動・replay等のintegration証拠は未成立。Enterprise signing levelを満たすOwner承認済みの開発署名経路または管理者のpolicy decisionが必要な場合は、その承認待ちとする。policy・署名鍵を変更せず、processを起動しないunit／Schema／Conformance検証と実装は続行する。

## D4 Pocket Phase 7 WindowsローカルRust検証の再確認（2026-09-28）

現在のcleanな`main`（`25524638df0eccdb87bf14e00b4ef4b59199ed7b`）で、以前はWindows Application Controlにより起動を拒否されたRust test executableが、短い`C:\D4Pocket` checkout上で実行できることを確認した。これは本checkout・現在のtest構成に対するローカル検証結果であり、OneDrive checkoutのACL、installed product、全targetまたは他OSの証拠へ一般化しない。

- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib -- --test-threads=1`: 302件成功。Agent作業要求のBroker検査testを含む。
- `python tooling/schema_check/check_schemas.py`: Schema 143件、正常example 143件、negative fixture 177件で合格。
- `python tooling/conformance_tests/run_conformance_skeleton.py`: 221件のcheckが合格。
- `python -X utf8 tooling/日本語基底監査.py --strict`: 負債file 0、finding 0で合格。`python -X utf8 tooling/manifest.py --write`／`--check`、`python -X utf8 tooling/release_gate_check.py`、`git diff --check`も合格。
- `cargo test --all-targets`、Flutter/Desktop/Mobile、実Agent／実Task、installed Windows product、実端末、他OSの検証は今回実行していない。
- `codex exec --help`は成功し、導入済みCLIがJSON event、sandbox選択肢、worktree等のoptionを列挙することだけを確認した。optionの組合せ、隔離、task結果の回収、process-tree停止、Credential／費用挙動は実証していない。Codex AdapterからモデルTaskを起動しておらず、CLI helpはTask実行機能の証拠ではない。
- 証拠境界: Schema／fixtureは`FIXTURE`、Broker unit testは試験adapter・一時状態での検査。`LIVE_RUNTIME`や製品実行の証拠ではない。
- `release_blocker`: Agent Task実行consumer、一回消費、実行直前の再検証、実証済み隔離、実行前後Audit／Recovery、結果／diff、比較／Handoffは未成立。全target Rust、Flutter、installed product、実Agentおよび実端末の未実行範囲もrelease evidenceとして未確認のまま維持する。

## D4 Pocket Phase 7 隔離SessionのTask Permission即時失効（2026-09-27）

Task用Workspace Permissionの揮発記録を、対話Sessionが中止・期限超過・監査失敗・worker障害で`中止後隔離`へ遷移する全経路から直ちに除去する。Sessionが非利用状態のためTask要求検査自体も拒否するが、状態照合だけに頼らずAuthority記録を消去する。対話のread-only実行経路とTask実行未接続状態は変更しない。

- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib 中止後にworker応答を回収しても評価遅延を残さない -- --test-threads=1`: 成功。owner発行済みTask Permissionが中止によるSession隔離時に即時消去されることを確認。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml -- --test-threads=1`: library 300件成功、Desktop launcher harness 0件成功。その後`gui_shell_rust_helper` test executableはWindows Application Control `os error 4551`で起動前に拒否されたため、all-targetは不成立。
- `python -X utf8 tooling/schema_check/check_schemas.py`: Schema／example 142件、negative fixture 176件で合格。`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`: 219件で合格。
- `python -X utf8 tooling/日本語基底監査.py --strict`: 負債file 0、finding 0で合格。`git diff --check`も合格。
- `python -X utf8 tooling/manifest.py --write`（tracked source 1059件）／`--check`と`python -X utf8 tooling/release_gate_check.py`は合格。release blockerは残り、`release_ready=false`。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`: exit 0。Python側統合validationは合格、Windows installed-product evidenceを含むrelease blockerは保持。
- `release_blocker`: Task実行consumerとPermission消費、Task本文・実行条件へ結合する別Owner Approval、実行前後AuditEvent、実行可能なRecoveryAction、隔離書込実行、結果/diff保存、比較／Handoffは未接続。今回の変更はTask実行やrelease readinessを証明しない。Windows Application Controlによる全target試験の起動拒否も未解決。

## D4 Pocket Phase 7 Agent作業要求のBroker照合経路（2026-09-27）

既存`agent_task_request.schema.json`をRust Brokerの`Agent作業要求検査`へ接続した。要求ごとにAgent metadata、利用中Session、Session作成時のWorkspace登録hashを現在のBroker状態と照合する。指示本文は応答・Auditへ出さずBroker計算hashだけを返し、成功応答にも未実行・Permission未付与・Approval未取得を明示する。これは登録関係の`INTERNAL_STATE`検査のみで、実Agent稼働、Workspace隔離、Task保存・実行は証明しない。既存対話ApprovalをTask権限へ流用せず、Codex Adapterのread-only境界を維持する。

- `cargo test --all-targets -- --test-threads=1`: 344件成功（library 299、main 9、IPC 10、他integration 26）。
- `python -X utf8 tooling/schema_check/check_schemas.py`: Schema 140件、正常example 140件、negative fixture 174件で合格。`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`: 218件で合格。
- `python -X utf8 tooling/日本語基底監査.py --strict`: 負債file 0、finding 0で合格。`git diff --check`も合格。
- `rustfmt --check --edition 2021 native/rust_helper/src/broker/dialogue.rs native/rust_helper/src/broker/protocol.rs native/rust_helper/src/broker/workspace.rs`: 既存未整形箇所を含む差分が検出され不合格。一括整形は行わず、既存差分を保持。
- `python -X utf8 tooling/manifest.py --write`（tracked source 1053件）／`--check`、`python -X utf8 tooling/release_gate_check.py`：合格。release gateは残存release blockerを維持し、release readinessを示さない。
- commit／push／remote HEAD、backup tagとrollback pointは変更単位を閉じた後に最終報告する。
- 残る`release_blocker`: Task専用Workspace Permission、別個の一回限りOwner Approval、実行前後AuditEvent、失敗時RecoveryAction、隔離Workspace上のTask起動、結果/diff保存、比較／Handoff。

## D4 Pocket Phase 7 Agent作業要求Contractと権限非内包境界（2026-09-27）

Agent作業要求の機械契約として`specs/agent_task_request.schema.json`を追加し、登録Agent Runtime ID・Session ID・Workspace ID・最大32,768文字の指示本文だけを受け付ける。Permission／Approval／Audit ID、authority、sandbox、実path、executable、command、credential fieldを拒否するpositive／negative contract exampleとConformanceを追加した。意味正本では、対話送信のApprovalと書込みTaskのPermission／Owner Approvalを別物と定義し、要求本文のhashはBroker側が計算する責任を定めた。

- `python -X utf8 tooling/schema_check/check_schemas.py`: Schema 140件、正常example 140件、negative fixture 174件で合格。
- `python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`: 218件のcheckが合格。
- `python -X utf8 tooling/日本語基底監査.py --strict`: 負債file 0、finding 0で合格。
- 証拠境界: Schema／fixtureの`FIXTURE`と日本語意味契約のみ。Rust Broker consumer、task専用Permission／Approval経路、監査・Recovery、実Agent起動、書込み隔離、比較・Handoffは未接続・未実証であり、既存Codex Adapterのread-only境界と`release_blocker`は維持する。

## D4 Pocket Phase 7 Windows loopback fixture待ち時間の追補（2026-09-27）

試験用loopback serverが suite 負荷時に早く閉じる可能性へ対処し、A2A／Broker fixtureのread timeoutとMINIDORA fixtureのread timeout・当該unit test期限を2秒から10秒へ延ばした。製品側HTTP timeout、retry、Authority、process経路は変更していない。検査を重ねた初回にはMINIDORA unit testが`通信失敗`となったが、単独試験と検査を重ねずに実行した全targetでは再現せず成功したため、host負荷が原因との因果は未確定であり、loopback resetの`known_limitation`を解消扱いしない。

- `cargo test --all-targets -- --test-threads=1`: library 298件、main 9件、integration 36件、launcher test binary 0件、合計343件成功。
- Schema 140件／example 140件／negative fixture 174件、Conformance 218件、strict日本語監査（負債0／finding 0）も成功。
- crate全体の`cargo fmt --check`は変更対象外を含む既存Rust fileの差分を検出して不合格。一括整形はせず、今回変更した3 fileへの`rustfmt --check`だけは成功。
- 証拠境界: fixture安定性の`FIXTURE`とWindows local Rust testのみ。実Agent接続、production transportの性能、loopback reset原因、release readinessは証明しない。AgentTaskのBroker実行・task専用Permission／Owner Approval・Audit／Recovery・書込み隔離は引き続き`release_blocker`。

## Windows開発checkoutをOneDrive外へ確立（2026-09-27）

OneDrive配下の日本語・長pathでFlutter Analysis ServerのLSP JSONが壊れ、test準備時には`build/unit_test_assets`の削除もACL拒否される既存host制約に対し、remote `main`のcommit `1f3049abef858c0bd4a6220c8de0af15f7226bec`から`C:\D4Pocket`へ独立したASCII短path checkoutを作った。既存OneDrive checkout、ACL、同期設定は変更・削除していない。以後のWindows開発・検証ではこのcheckoutを使用できる。

- 新checkoutの`HEAD`と`origin/main`は上記commitで一致。backup branchはremote tag `codex/backup-main`と`codex/backup-main-prev`から初期化し、作業treeはclean。
- `flutter pub get`：成功。`flutter analyze --no-pub`（`apps/desktop_flutter`）：`No issues found!`。
- `flutter test --no-pub --reporter expanded test/widget_test.dart`（`apps/desktop_flutter`）：42件成功。OneDrive側の削除deny ACLを変更せず、同じFlutter試験を非同期対象pathで実行できる。
- このcheckoutはOneDrive同期対象外の別cloneであり、Git remoteを共有する。各checkoutの未commit変更は共有されないため、編集前に使用checkoutとremote HEADを確認する。このsource-level Flutter検証はWindows installed product、Rust child-process integration、release readinessを証明しない。

## D4 Pocket Phase 7 Agent Centerのfixture内容非表示（2026-09-27）

現行Agent CenterはBrokerの`対話セッション一覧`が返すmetadataだけを実値として表示し、実AgentのTask結果・diff・Tool・command内容は取得しない。一方、画面はlocal／mock snapshotに含まれるTask例を描画でき、識別markerを一部regexで置換しただけのHandoff概要も生成していた。Client modeが実Brokerの`broker`である場合だけSession metadataを表示し、`local`／`mock`のSession例は隠してfixtureである旨を示す。Broker対話Sessionでは従来の実行情報欄を「未取得」と明示し、承認情報が未取得でもApprovalなしとは解釈しない。実Handoff経路がない間は固定の未接続状態を表示し、regex redactionから概要を合成しない。Broker sessionの重複ID／未知field拒否、Workspace対応表示、比較metadata検査は維持し、実Agent比較・Handoff完了とは扱わない。

- `dart format --output=none --set-exit-if-changed`：変更したDesktop画面・比較service・testで成功（0 file changed）。
- ASCII検証copy `C:\d4nativecheck3`：`flutter analyze --no-pub`成功。`flutter test --no-pub --reporter expanded test/widget_test.dart`は最新sourceで42件成功。Broker metadata表示とmock fixture非表示の専用widget testも成功し、`test/agent_coordination_test.dart`は3件成功。
- Desktop全suiteのscratch実行は、修正前に111成功・2失敗だった。Workspace metadata widget assertionはClient modeを`product`と誤認した本変更の不具合で、実際のmode`broker`へ修正した。fixture表示文言のtest期待値も画面表示へ揃え、最新widget suiteは42件成功。Rust全suiteではRust lifecycle Broker process testがWindows Application Controlにより子process起動前に拒否（OS error 4551）されており、全suite合格とは扱わない。policyは変更せず、拒否後の再配置・再試行はしていない。
- `python -X utf8 tooling/schema_check/check_schemas.py`：Schema 139件、example 139件、negative fixture 173件で成功。`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`：217件成功。`python -X utf8 tooling/release_gate_check.py`：成功、ただしこれはrelease blockerが残る状態を正しく維持するgate検査であり、release readinessの証明ではない。strict日本語監査・Manifest検査・`git diff --check`も成功。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`：exit 0。strict日本語、Schema、Conformance、Manifest、release gate、packaging portability、release smoke、evidence bundle、runtime assertion、C32 final development auditの全10 checkが成功した。これはdevelopment validationであり、Windows installed productの証拠は5項目未成立、`release_ready=false`のまま。
- 実OneDrive workspaceではDesktop Flutter analyze時に日本語pathを含むLSP JSON応答が`FormatException: Unterminated string`で異常終了した。Desktop Flutter試験準備も`build\\unit_test_assets`の削除拒否で起動前に失敗していた。ACLは変更せず、ASCII copyで解析・試験した。これはWindows開発hostの`known_limitation`である。
- Task結果、独立Workspace上のAgent実行、比較、実Handoff、再評価・取消・失敗隔離・Recoveryは依然`release_blocker`。本単位は誤認表示を抑止するUI境界であり、これらの機能を実装・実証していない。

## D4 Pocket Phase 7 Mobile登録Workspace選択のID投影（2026-09-27）

Mobile対話画面から登録済みWorkspaceを選択できるよう、既存Device Linkの許可操作へ空payload限定の一覧要求を追加した。Rust Brokerは既存のowner登録Workspace要求・監査経路を使い、返答をWorkspace IDとRuntime IDだけへ限定してから端末向け監査・応答へ渡す。Android Kotlin、iOS Swift、Flutterは同じ固定projectionを検証し、内部登録hash、Approval metadata、path等の追加fieldを拒否する。Mobileは選択したIDを通常の対話開始要求へ渡すだけであり、選択・Agent metadata・端末表示はPermissionやApprovalを生成しない。

- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib -- --test-threads=1`：Rust library 298件成功（当該Rust変更を含む実行）。
- `flutter test --no-pub --no-test-assets --concurrency 1 --reporter expanded test`（`packages/gui_shell_ui`）：56件成功。`flutter test --no-pub --no-test-assets --concurrency 1 --reporter expanded test`（`apps/mobile_flutter`）：17件成功。
- `python -X utf8 tooling/schema_check/check_schemas.py`：139 Schema／139 example／173 negative fixtureで成功。`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`：217 check成功。
- ASCII検証copy `C:\d4nativecheck3` で`flutter pub get`と`apps/mobile_flutter/android/gradlew.bat testDebugUnitTest --no-daemon`が成功し、Android Kotlin実装・JUnitをcompile／実行した。これはsource-level Windows検証であり、端末TLS接続・実機動作の証拠ではない。
- Flutter analyzeはOneDrive配下の日本語pathで3 projectともAnalysis ServerのLSP JSON `FormatException: Unterminated string`により終了した。同じ最新sourceのASCII検証copyでは共有UI・Mobile・Desktopすべて`flutter analyze --no-pub`が`No issues found!`。path依存のhost制約とsource診断結果を分けて記録する。
- iOS native XCTest、Desktop Rust Brokerへの実端末TLS、端末foreground/background・失効を含むLIVE_RUNTIME証拠は未実施。Workspace一覧・ID投影は`INTERNAL_STATE`であり、実Agent隔離／実行の証拠ではない。これらのrelease blockerと`release_ready=false`を維持する。

## D4 Pocket Phase 7 Codex task spawn時Workspace path guard（2026-09-27）

Codex Adapter登録時に、nofollowで開いたWorkspaceのdevice ID／file IDを記録し、version／help probe中はdirectory handleを保持する。各task process spawnの直前に同じ固定pathを開き直し、登録identityと違うWorkspaceを通信失敗として拒否する。Windowsではvolume rootからWorkspaceまでの各handleを`Command::spawn`完了まで保持する。既存cap-std Windows directory handleは`FILE_SHARE_DELETE`を許可しないため、通常NTFS pathで対象Workspaceと祖先directoryのrename／deleteを拒否する。guardはchild process生成後に解放し、Broker稼働中ずっとWorkspaceをロックしない。

この限定対策は、別名path／管理者mount等を網羅しない。Unixではhandle保持だけでrenameを防げず、identity再確認からchild processのcwd解決までのraceが残る。実Codex CLI task、Agent専用Session、実Workspace分離、cross-agent contamination、比較・Handoffの証拠ではなく、これらのrelease blockerと`release_ready=false`を維持する。

- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib adapters::codex_cli -- --nocapture`：7件成功。登録後に同じpathへ別directoryを置く負例を拒否し、WindowsではWorkspaceと祖先directoryのrenameがguard保持中に拒否されることを確認した。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib -- --test-threads=1`：Rust library 298件成功。
- `cargo test --no-run --locked --manifest-path native/rust_helper/Cargo.toml`：Rust全12 test executableをcompileできた。`cargo test --locked --manifest-path native/rust_helper/Cargo.toml --test workspace_startup -- --test-threads=1`：process-level Workspace startup 7件成功。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml -- --test-threads=1`：初回は既存A2A loopback fixture 1件が応答読取失敗となった。対象testの単独再実行は成功し、次の全target再実行でもRust library 298件すべて成功したが、次の`gui_shell_desktop_launcher` test executableはWindows Application ControlのOS error 4551で起動前に拒否された。launcherおよび後続integration executableはその全target実行では未実行であり、test failureへ読み替えない。policy変更・test fileの移動や再配置はしていない。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`：登録済み10検査すべて合格。`schema_check`は138 Schema／138 example／170 negative fixture、Conformanceは217 check。release gate整合性checkはpassだが、release evidence blocker 5件と`release_ready=false`を維持する。
- `python -X utf8 tooling/manifest.py --write`および`--check`、`rustfmt --edition 2021 --check native/rust_helper/src/adapters/codex_cli.rs`、`git diff --check`：すべて合格。日本語基底監査の初回はtest診断文1件を検出したため日本語へ修正し、上記最終検証ではfinding 0となった。
- 検証証拠はWindows上のRust test processと一時NTFS directoryによる`FIXTURE`。Codex CLI実起動／実task、Unix path race、mount aliasは未検証。既存Windows Application Controlのtarget別4551制約を回避していない。

## D4 Pocket Phase 7 Agent Sessionと登録WorkspaceのBroker結合（2026-09-27）

Agent Adapterの対話開始にWorkspace IDを明示させ、既存Rust BrokerのWorkspace registryが同一Runtime IDで現在保持する登録だけを受理する。未指定・未登録・別RuntimeのWorkspaceはSessionを作らず拒否する。通常RuntimeのWorkspaceなしSessionは従来どおり許可し、登録Workspaceの任意指定もmetadataとして結合するだけでPermissionにはしない。Desktop共有対話画面は既存Broker一覧からRuntimeごとのWorkspace IDを選び、選択値を通常要求へ渡す。Session作成Auditの既存hash射影は維持し、Session ID・Runtime ID・Workspace ID・登録hashの関係を別AuditEventへ記録する。Agent Session一覧は作成Audit IDと結合Audit IDを別々に返し、DesktopはBroker登録上のmetadata対応として表示する。

この結合はAgent専用実行Session、Runtimeが実際に使うdirectory、書込み隔離、Task実行、独立Agent比較・Handoffの証拠ではない。Mobile Device LinkにWorkspace選択面はなく、Agent対話開始はfail-closedで拒否する。PermissionやApprovalは生成しない。Release blockerと`release_ready=false`を維持する。

- `cargo test --no-run --locked --manifest-path native/rust_helper/Cargo.toml`：Rust全12 test executableをcompileできた。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml -- --test-threads=1`：Rust library 296件、`main.rs` 9件、`broker_ipc` 10件、`canonical_decimal_hash` 1件、`checkpoint` 8件、`protected_data` 2件、`protected_startup` 1件が成功。その後`protected_store` test executable自体がWindows Application ControlのOS error 4551で起動拒否され、全target commandは停止した。同targetの3件は未実行でassertion failureではない。`workspace_diff` 2件と`workspace_reader` 2件は個別target実行で成功した。`workspace_startup`は7件すべてexecutable起動前に同じ4551で拒否された。実行できたRust testは合計331件であり、全target成功とは扱わない。policy変更・test fileの移動や再配置・test除外は行っていない。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib 通常要求経路で監査済み内部状態だけを返す -- --test-threads=1`：1件成功。通常IPCでWorkspace未指定・未登録・他Runtime所属を拒否し、同一Runtimeの登録IDだけで開始する境界を検査した。異Runtime fixtureは既存Workspace範囲検査が要求する完全なdirectory ancestry付きで登録した。
- `flutter test --no-pub --no-test-assets --concurrency 1 --reporter expanded test`（`packages/gui_shell_ui`）：55件成功。Workspace登録一覧のRuntime別厳格parse、未承認の読取grantを権限化せずWorkspace IDを選択する経路、選択IDだけをSession開始へ送るClient／widget試験、Mobile向け未対応面ではWorkspace一覧要求を送らない試験を含む。`flutter analyze --no-pub`（共有UI package、Desktop、Mobile）はすべて指摘なし。
- 同じDesktop全test commandは初回109/111件で、実Broker childを起動する2件が一時4551で拒否された。対象2 fileの個別再実行は3件と16件が成功し、その後のDesktop全test再実行は113件すべて成功した。
- 最新変更後のDesktop全testは同じ直列commandで113件成功し、Mobile全testは16件成功した。Mobile画面が未対応のWorkspace一覧操作を発行しないことは共有UIのwidget試験でも確認した。
- `python -X utf8 tooling/schema_check/check_schemas.py`：Schema 138件、正常example 138件、negative fixture 170件で合格。`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`：217 checksで合格。`python -X utf8 tooling/日本語基底監査.py --strict`：負債0／finding 0で合格。
- `python -X utf8 tooling/manifest.py --write`（tracked source 1045件）と`--check`は合格。`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`はexit 0で登録済み10検査すべて成功し、`python -X utf8 tooling/packaging_portability_check.py`を含む。release gate検査は整合passだが、release blocker 5件と`release_ready=false`を維持する。Release smoke／evidence bundle／runtime assertion／C32監査は開発・fixture範囲であり、製品実証へ読み替えない。
- 証拠境界：Broker結合試験はBroker-owned registryと監査を通る`INTERNAL_STATE`／試験Broker証拠。実Agentのdirectory使用、同時書込み隔離、実Task、Windows installed product、`protected_store`と`workspace_startup`の実行挙動は未検証。Windows Application Controlの間欠・target別起動拒否を既存`windows_rust_integration_test_execution_policy` release blockerとして保持する。

## D4 Pocket Phase 7 Codex Workspace root設定の物理identity照合（2026-09-27）

Codex Adapterの固定作業pathと、owner起動設定で同じruntime IDへ登録するWorkspace rootが独立指定であり、食い違いを検知する起動preflightがなかった。Workspace設定をCodex CLIのversion／help probe前に一度読み、同じruntime IDの全Workspace rootとAdapter作業pathを既存のnofollow・filesystem・保護path検査で開いてdevice ID／file IDを比較する。不一致・未観測・保護領域重複は`CONFIG`拒否Auditを残し、IPC endpointを作る前に起動を止める。読み取った同じ設定objectを後続のWorkspace登録へ渡す。

この単位はownerが宣言した固定path同士の起動時整合だけを検査する。Workspace設定のないSessionへbindingを推定せず、対話SessionへWorkspace IDを結合しない。照合後のpath差替え防止、実Agent間の書込み隔離、Agent比較・Handoffは未成立のまま保持する。

- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib codex_workspace -- --nocapture`：一致root／不一致root／複数指定中の不一致／異なるruntimeの非binding、および実Broker起動での拒否監査・endpoint非生成を含む2件が成功。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --test workspace_startup -- --test-threads=1`：7件すべて、Broker子プロセス起動時にWindows Application ControlからOSエラー4551が返り失敗。Workspace検査へ到達する前であり、制御設定の変更や実行ファイルの移動・再配置はしていない。`windows_rust_integration_test_execution_policy`を未解決へ戻す。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib -- --test-threads=1`：機能実装後のRust単体試験292件すべて成功。全targetの1回目は292件中291件成功し、未変更のA2A Broker模擬HTTP接続試験が`a2a_connection_failed`となって終了値1。再実行した全target commandは試験実行file起動前にWindows Application ControlからOSエラー4551で拒否された。全対象試験の成功とは扱わず、両方の失敗を保持する。
- `python -X utf8 tooling/schema_check/check_schemas.py`：Schema 138、正常example 138、negative fixture 170で合格。`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`：217件で合格。`python -X utf8 tooling/日本語基底監査.py --strict`：負債0／finding 0で合格。
- `python -X utf8 tooling/manifest.py --write`：tracked source 1045件を更新し、続く`--check`が合格。書込み前のrelease gate失敗はこの5 fileのManifest hash不一致であり、後続に再実行する。
- `rustfmt --edition 2021 --check native/rust_helper/src/broker/ipc_server.rs native/rust_helper/src/broker/workspace_root.rs`：不合格。変更対象外の既存コードを多数含むfile全体にformat driftがある。追加範囲は局所的に整形し、無関係な一括再formatは行わない。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`：exit 0、登録済みPython／開発検証10件すべて成功。release gate整合性はpassだが、既存release blockerにより`release_ready=false`を維持する。
- 証拠境界: 物理identity検査はOS directory handleを使うが、この試験の証拠は`FIXTURE`である。導入済みCodex processが同じWorkspaceを使ったこと、起動後も結合が維持されたことは証明しない。Git状態の閉包は最終確認待ち。

## D4 Pocket Phase 7追補: 比較可否表示と操作面記述の同期（2026-09-27）

比較projectionのAgent runtime ID一意性検査に合わせ、Agent Centerの状態文言とGUI操作面の責任記述を更新した。表示は宣言上のruntime／Workspace参照が重複しない場合に限る条件付き表示であり、実Agent identityや実行時隔離の確認済み表示にはしない。

- `dart format --output=none --set-exit-if-changed apps/desktop_flutter/lib/services/agent_coordination.dart apps/desktop_flutter/lib/screens/agent_center.dart apps/desktop_flutter/test/agent_coordination_test.dart`：合格。
- `flutter test --no-pub --no-test-assets --concurrency 1 --reporter expanded test/agent_coordination_test.dart`：4件成功。`flutter analyze --no-pub`：Desktop／Mobileとも指摘なし。変更前後のDesktop全体111件試験も成功。
- 証拠境界: UI文言・projectionの整合であり、実Agentの比較起動、Workspace隔離、Agent間汚染防止は実証しない。これらの`release_blocker`は維持する。

## D4 Pocket Phase 7補足: Agent runtime識別のUI projection同期（2026-09-27）

Desktop Agent比較projectionがSession／Workspace参照しか保持せず、同一Agent runtimeを異なるAgentとして比較可能にする契約不整合を修正した。snapshot由来のAgent runtime IDを保持し、欠落・不正形式・entry間の重複で比較を停止する。Schema／Conformanceの一意性要件とFlutterの入力検査を同期した。

この検査はINTERNAL_STATEの宣言識別子の照合であり、実Agent identity、別Workspace実体、path alias不在、実行時のcross-agent contamination防止を証明しない。実Agentの隔離実行経路は`release_blocker`のまま維持する。

## D4 Pocket Phase 32補足: portable Export Credential scan（2026-09-27）

Windows portable Export buildへ、artifact公開前の既知Credential pattern scanを追加した。署名鍵marker、AWS／GitHub／Google／Slackの既知token形式、Credential名付き設定値、Bearer authorization、Credentialを示す固定file名をbundle内のruntime artifact全fileから検出し、該当時は公開を停止する。AWS markerを1 MiB chunk境界にまたがらせた負例を含め、全対象fileをchunk読取し、scan前後のartifact inventoryとtree hashが一致することを検査する。build evidenceはscanのfile数、byte数、0 finding、対象tree hashを持つ。

scan範囲は有限の既知pattern集合であり、`known_patterns_only`として記録する。未知形式、暗号化、分割、変換された秘密の不存在は証明しない。自己参照のためbuild evidence自身はruntime artifact inventory／scan対象から除く。合成fixture上のConformance成功は実bundleのscan結果ではない。

- Validation: `python -m py_compile tooling/export_windows_product.py tooling/conformance_tests/run_conformance_skeleton.py`、`python -X utf8 tooling/schema_check/check_schemas.py`（Schema 137／正常例137／負例169）、`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`（216件）、`python -X utf8 tooling/日本語基底監査.py --strict`（負債0／finding0）はPASS。
- 最初の集約実行は更新前Manifestのhash不一致で失敗した。Manifest更新後の実行では追加した進捗行が厳格日本語監査に1件検出されたため、その記述を直して再検証した。最終 `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows` はexit 0で設定済み10検査すべて成功し、既存release blocker 5件と`release_ready=false`を維持した。集約に含むsmoke／fixture成功を実bundleやinstalled製品の証拠へ昇格しない。
- 実物検証: この変更ではExport buildを再実行していない。直近の試行では`Cargo`の`build script`が`Windows Application Control`に`OS error 4551`で拒否され、生成物は作られなかった。このため製品bundleの`Credential scan`、導入後の起動、実行時`Manifest`読込、Module除去は未確認であり、成立を主張しない。`rev2_export_owner_ui_authority_path`と`rev2_module_pruning_binary_and_measurement`は`release_blocker`のまま、`release_ready=false`を維持する。

## D4 Pocket Phase 32追補: Export runtime保存領域の分離対応（2026-09-26）

Rust Desktop起動器に、コンパイル時に埋め込まれた生成済みApp IDとAudit store IDから製品別runtime rootを選ぶ経路を追加した。両IDが未設定の既存GUI-Shell起動は従来の`%LOCALAPPDATA%\GUI-Shell\broker\desktop`を維持する。両IDが所定形式なら`%LOCALAPPDATA%\D4Pocket\apps\<App ID>\stores\<Audit store ID>`を使い、片側欠落・不正形式では保存directoryの作成、Broker起動、Flutter起動より前にfail-closedとなる。reparse point拒否とLocal AppData rootへのcontainment検査を各directory componentへ適用し、製品別Owner確認に保存先を表示する。

IDはデータ分離用の識別子であり、Permission、Approval、Capability、Credential、Agent trustを生成しない。旧runtimeからの移行・継承は行わず、IDはFlutter childのenvironment allowlistへ渡さない。runtime pathとBroker起動の既存Rust経路を変更し、個別製品IDごとの異なる保存先を利用できるところまで成立した。

- Evidence class: `CONFIG`（compile-time identity選択規則）、`FIXTURE`（Windows一時directory・不正ID・junction拒否のRust component testとConformance）。Installed Export Appの`LIVE_RUNTIME`証拠ではない。
- 未接続範囲: Schema検証済みManifestからcompile-time値を供給するExport build tool、製品IDごとに分離したCargo target directoryを使うbuild orchestration、実行可能bundle、installed product上の独立Runtime／物理Audit storeは未成立。Manifest-only Exportやこの起動器対応を独立App完成へ昇格させない。
- 検証: `python -X utf8 tooling/schema_check/check_schemas.py`（Schema／正常example 136、negative fixture 168）、`python tooling/conformance_tests/run_conformance_skeleton.py`（215 checks）、`python -X utf8 tooling/日本語基底監査.py --strict`（負債0 files／0 findings）、`python -X utf8 tooling/manifest.py --check`、`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`（development 10 checks）はPASS。`cargo test --locked --manifest-path native/rust_helper/Cargo.toml -- --test-threads=1`はRust全target 330件PASS（library 285、CLI 9、integration 36）。製品ID compile-time環境を指定した別targetの`runtime_directory_uses_only_embedded_identity_values`も1件PASS。`git diff --check`はPASS。
- Format: `rustfmt --edition 2021 --check native/rust_helper/src/desktop_launcher.rs`はimport順、既存関数、既存testを含むfile全体の既存format driftでFAIL。追加した箇所はrustfmtの該当差分を確認・整形した。無関係な全file再formatは行っていない。
- 未成立分類: `rev2_export_owner_ui_authority_path`と`rev2_module_pruning_binary_and_measurement`を`release_blocker`として維持する。Windows installed productでのOwner操作・保存先・完了Auditの`LIVE_RUNTIME`証拠、最終製品binary pruning・安全Core保持・起動／資源比較は未確認。

## D4 Pocket rev2 C6 Desktop対話pane owner登録接続（2026-09-25）

前段のC6回帰Case保管・一覧・Owner削除／中断Recoveryに対し、Desktop対話paneから完了結果をownerが明示的に回帰Case登録する経路を追加した。送信受付時にBrokerが返す要求ID／要求hashを対話clientが厳密に検証して保持する。要求hashは結果との相関値に限定し、Approval、Permission、Owner資格、実行許可を生成しない。既存Broker応答にSchemaがなかったため、`runtime_dialogue_submission_receipt.schema.json`、正常／否定fixture、Schema checker、Conformanceを追加し、既存応答のfieldと権限境界を固定した。

Desktop画面は`full`表示かつ成功／保留結果で、終了監査ID付き実行記録を確認できる場合だけ登録操作を示す。元の対話入力・応答は自動転記せず、ownerが公開名、秘密を除いた再現入力、条件、参照、期待経路を明示記入する。Flutterは既存の通常Broker transportから登録要求を送り、owner資格、秘密、privileged IPCを持たない。既存Rust起動器のoperation別native確認・capacity-1 process内Owner経路を再利用し、登録用の固定summaryには公開名、要求ID／hash、結果状態、件数、入力文字数、payload hashだけを含める。ownerが確認してもBrokerは現在の要求hash、full表示、結果証跡、終了監査、ProtectedStore条件を再検証する。登録定義の本文・条件・参照・期待経路はnative summary、Audit、receiptへ出さない。既知secret marker拒否は秘密が存在しないことの証明ではない。native確認もWindows account再認証や本人性証明ではない。

Production pathは`Desktop対話pane → 既存Rust起動器のnamed-pipe relay → operation別native確認 → process内Owner操作channel → Broker再検証・永続Audit・Regression ProtectedStore → hash-only receipt`。新bridge、C5自動import、private定義の通常閲覧・実行、Authority継承は追加していない。C5 Datasetへの明示importとOwner確認を伴うinstalled Windows product上の登録／削除／中断Recoveryは別途未成立の`release_blocker`であり、全体`release_ready=false`を維持する。

検証結果:

- `python tooling/schema_check/check_schemas.py`: Schema 132件／正常例132件／negative fixture 163件でPASS。
- `python tooling/conformance_tests/run_conformance_skeleton.py`: 201 checksでPASS。
- `python -X utf8 tooling/日本語基底監査.py --strict`: 最初の実行では対話画面の新規英語表示1行を検出してFAILした。表示語を日本語化して再実行し、負債0 files／0 findingsでPASS。例外登録はしていない。
- `python tooling/final_development_audit.py`: C0〜C31の32工程対応を確認してPASS。これは対応表構造の検査であり全機能完成の証明ではない。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml -- --test-threads=1`: library 265件、helper CLI 9件、Broker IPC 9件、他integration 26件、合計309件でPASS。先行実行ではA2A loopback試験2件に読み取り失敗が出たが、単独再試験と直列全target再試験はPASSした。失敗原因は特定しておらず、直列成功だけを恒久的な安定性保証へ昇格しない。
- `flutter test --no-pub`（共有UI）: 51件でPASS。表示語修正後も`flutter test --no-pub test/runtime_dialogue_test.dart`は全件PASS。
- `flutter test --no-pub`（Desktop Flutter）: 106件でPASS。共有UI／Desktop双方の`flutter analyze --no-pub`は指摘なし。Dart formatは対象8 file、変更0件。
- `flutter build windows --debug --no-pub`をOneDrive workspaceの`R:` aliasから試行したが、`windows/flutter/ephemeral/cpp_client_wrapper`の再解析点配下に必要な4 C++ sourceが実体化しておらず、MSBuild C1083で失敗した。再解析点とsource欠落を確認し、製品sourceの不具合とは区別した。
- 同じstaged差分をdetached Temp worktreeへ適用し、`flutter pub get --offline`、`flutter build windows --debug --no-pub`、`cargo build --locked --bins --manifest-path native/rust_helper/Cargo.toml`を実行。Debug DesktopとRust全binaryのbuildは成功した。MSBuildはTemp内build出力に対するMSB8029 warningを出したがerrorではなく、OneDrive workspaceまたは正式installed productの実行証拠とはしない。
- `python tooling/validate_all.py --python-only --desktop-platform windows`: 編集途中の日本語監査／古いmanifest、新規Schema／fixtureが未stageだった配布互換性検査という先行FAILを修正し、sourceをGit indexへ登録、manifest追補後に最終再実行した。開発modeの10 checksすべてPASS。evidence bundleは既存release blocker 5件と`release_ready=false`を保持し、CONFIG／FIXTUREのsmoke・assertionをWindows installed product証拠へ昇格しない。
- Rust format checkは変更Rust file内の既存広範差分を報告してFAIL。無関係な行を大量変更する整形は行わず、`git diff --check`で差分空白を別途確認する。

この節はContract、Desktop source、component試験の成立を記録する。Windows native MessageBoxでOwnerのYes／Noを操作した実証、clean commitからのinstalled product、外部収集証拠、C5 importは含まない。以前のC6進捗節は当時の状態を示す履歴としてそのまま保持する。

## D4 Pocket C6 Desktop Owner削除・中断Recovery接続（2026-09-25）

Desktop評価ラボにOwner確認付き削除と中断状態照合を接続した。Flutterは既存BrokerTransportへ通常要求を送り、秘密・Owner資格を保持しない。Rust Desktop起動器が固定allowlist（`GUI Shell書出し`、`回帰Case削除`、`回帰Case削除中断確認`）とoperation別summaryを検証してWindows native default-No確認を表示し、Yesの要求だけをcapacity-1 process内channelからBroker所有threadへ渡す。確認は本人認証ではない。Recovery要求はCase IDだけで、過去の削除承認監査IDを受け付けない。

production接続に必要な既存条件も修正した。Desktop Brokerは従来ProtectedStoreを登録しておらず、C6の一覧・削除・Recoveryがfail-closedになる経路だった。Rust起動器は固定`%LOCALAPPDATA%\\GUI-Shell\\broker\\desktop\\protected`をruntime隣接領域として準備し、Broker設定はこのpath以外を拒否する。Windows NTFS、reparse point拒否、Broker store・session fileとの非重複を起動時に検査し、失敗時はendpointを出さずBroker起動を止める。Owner資格や一般Permissionは生成せず、既存の別path保管物を移動・上書き・削除しない。

Schemaとruntimeの不一致も修正した。中断RecoveryのSchema／fixtureは古い削除承認IDを要求していたが、Brokerは現在の永続Auditから未確定削除を導出するため、入力仕様から承認IDを除き、余分な過去IDをnegative fixtureとConformanceで拒否する。

Validation: `cargo test --no-run --locked --manifest-path native/rust_helper/Cargo.toml`成功。`cargo test --locked --manifest-path native/rust_helper/Cargo.toml -- --test-threads=1`はRust全target 309件成功（library 265、helper CLI 9、Broker IPC 9、残りintegration 26）。Windows実filesystemとBroker経路の試験を含むが、installed productの実操作ではない。Schema 131件／正常例131件／negative fixture 162件、Conformance 201 checks、strict日本語監査は負債0 files／0 findingsで成功。FlutterはASCII別名`R:`経由で評価ラボ5件、shared Client 6件、両packageの`flutter analyze --no-pub`、Dart format check（Desktop 4 files／shared 3 files、変更0件）が成功。

追加検証: `python tooling/manifest.py --check`、`python tooling/packaging_portability_check.py`、`python tooling/final_development_audit.py`、`python tooling/validate_all.py --python-only --desktop-platform windows`が成功。集約のrelease-runtime assertion 12件は`CONFIG`／`FIXTURE`範囲であり、installed product証拠ではない。集約は5件のrelease blockerと`release_ready=false`を保持した。

native確認testはoperation候補、表示summary、allowlist、拒否・stale条件、Broker監査接続を検査するが、実際のWindows MessageBox操作やinstalled productでのOwner操作を証明しない。C6 Owner登録GUI、C5への明示import、installed Windows productでのOwner削除／中断Recovery実証は`release_blocker`として残す。全体`release_ready=false`を維持する。

## D4 Pocket C6 Owner削除と中断Recovery（2026-09-25）

前段で接続したmetadata-only一覧の上に、既存Owner CLI→認証済みRust Broker経路だけを使う一件削除と中断Recoveryを追加した。Owner CLIはCase ID・定義hash・暗号文hashを要求し、Brokerは登録receipt、既存Audit chain、ProtectedStoreの排他削除準備を照合する。削除承認Auditを永続化した後に削除し、ProtectedStoreの削除後再観測で暗号文不在を確かめ、結果Auditが確定した場合にだけ`削除確定`を返す。結果はfile状態の`LIVE_RUNTIME`観測であり、媒体の物理消去を主張しない。

中断時は通常一覧から未確定Caseを除外し、自動再削除しない。Ownerの`回帰Case削除 --session-file … 中断照合 <CaseID>`は現在状態を再観測し、`暗号文不在・中断照合済み`または`暗号文残存・再試行可能`をLIVE_RUNTIMEでAuditへ記録する。残存が同一ciphertext hashの場合のみ通常一覧へ戻り、次の削除には新しいOwner要求と削除承認Auditが要る。秘密値はFlutterや通常IPCへ渡さず、新Bridgeは追加していない。

Production path: `Owner CLI → 既存認証Broker control → 登録receipt／Audit検証 → ProtectedStore prepare_delete → 削除承認Audit → 削除 → file不在再観測 → 結果Audit`。Recovery pathは`Owner CLI → 既存認証Broker control → 未確定削除Audit照合 → ProtectedStore現在状態観測 → 結果Audit`。Schema・正常／否定fixture・conformanceはこの経路のcontract境界を検査するが、Rust test binary実行とWindows実Broker owner操作の成功を代替しない。

Validation: `python tooling/schema_check/check_schemas.py`はSchema 131件／正常例131件／否定例161件、`python tooling/conformance_tests/run_conformance_skeleton.py`は201 checks、`python -X utf8 tooling/日本語基底監査.py --strict`は負債0 files／0 findings、`python tooling/final_development_audit.py`はC0〜C31の対応32件でPASS。`cargo test --no-run --locked --manifest-path native/rust_helper/Cargo.toml`も全target compileでPASSした。

`cargo test --locked --manifest-path native/rust_helper/Cargo.toml -- --test-threads=1`は全307件（lib 263、main 9、integration 35）PASS。標準並列実行ではMINIDORAとA2Aのloopback HTTP fixture試験が各1件timeout由来で失敗したが、2試験を個別実行するとPASSし、直列全数実行もPASSした。現在の証拠はWindows上のRust試験と実ProtectedStore操作を含むが、Owner CLIからinstalled Windows製品を通した実Broker操作証拠ではない。

`cargo fmt --all -- --check`はRust workspace内の複数moduleにformat差分を報告してexit 1となった。広範な既存format差分を一括変更するのは本作業範囲外のためformatterによる一括書換えは行っていない。これは形式検査の未成立であり、compile／test成功とは区別する。C6 Owner GUI登録・削除面、C5明示import、Windows installed product上のOwner実Broker実証は`release_blocker`のまま。全体`release_ready=false`を維持する。

## D4 Pocket C6回帰Case一覧（2026-09-25、metadata-only）

C6既存保存物を閲覧する読み取り専用経路を追加した。通常Broker IPCの`回帰Case一覧`は、既存Audit登録receiptから公開metadataだけを最大100件単位で返し、要求cursorはページ位置にのみ用いる。対象ページの保存ciphertext hashをreceiptと照合するが、private本文を復号せず、本文・credential・authority情報を応答へ含めない。C5 DatasetとC6回帰Caseは別operation・別Desktop tabのまま分離した。Desktop「評価ラボ」に手動更新と次ページだけのC6 tabを追加した。当時はOwner削除・Recoveryを未実装としていたが、次の2026-09-25追補で既存Owner CLI→Broker経路に接続した。

Production接続面は`標準BrokerClient → Rust Broker回帰Case一覧 → Audit receiptとProtectedStore metadata照合 → metadata-only response → Desktop C6 tab`。Schema、normal／negative fixture、Python schema checker、conformance、shared Dart client、Desktop表示testをこの経路へ接続した。これらのfixture・source testは構造およびcomponent範囲の証拠であり、Windows実BrokerによるC6 runtime一連動作を代替しない。

WindowsでRust test binaryの実行はApplication Control（OS error 4551）に拒否され、隔離target dirでの再試行も同じ制約を受けた。`cargo test --no-run`は全targetのtest executableをcompileするが、Rust testの実行成功を意味しない。Windows productでのC6実Broker実測も未実施である。これらはWindows開発環境／runtime evidenceの不足であり、既存release blockerを解消しない。

C6に残す機能範囲は、Owner権限による登録UI、削除とRecovery、明示的なC5 import、Windows Owner操作および実Broker evidenceである。read-only一覧の追加をC6完成やrelease readinessへ昇格せず、全体`release_ready=false`と既存release blocker 5件を維持する。

検証結果: `python tooling/schema_check/check_schemas.py`はSchema 127件・正常例127件・否定例155件でPASS、`python tooling/conformance_tests/run_conformance_skeleton.py`は201 checksでPASS、`python -X utf8 tooling/日本語基底監査.py --strict`は負債0 files／0 findingsでPASSした。共有Clientの`flutter test --no-pub test/regression_case_client_test.dart`は2/2、Desktopの`flutter test --no-pub test/evaluation_lab_test.dart`は3/3 PASSし、両packageの`flutter analyze --no-pub`も問題なし。`cargo test --no-run --locked --manifest-path native/rust_helper/Cargo.toml`は全12 test executableをcompileしたが実行はしていない。Windows Rust test実行はApplication Control（OS error 4551）で拒否され、runtime test結果は未確認である。

`python tooling/manifest.py --write`後の`python tooling/manifest.py --check`と`git diff --cached --check`はPASS。統合集約`python tooling/validate_all.py --python-only --desktop-platform windows`は開発modeの10 checkがすべてPASSし、release gate checkも既存blockerを分類できた。一方、installed Windows evidenceが未収集のためrelease blockerは5件、`release_ready=false`であり、集約内のrelease smoke／構造検査をinstalled product実証へ読み替えない。

## D4 Pocket Phase35追補: Schema date-time形式検査（2026-09-25）

端末内回復記録のUTC時刻に`format: date-time`と形状patternを定義していたが、共有Schema検査器がJSON Schemaの`format`を評価せず、`2026-02-30T12:00:00Z`を受理することを再現した。検査器へRFC3339 date-timeの書式・暦日・時刻・UTC offset検査を追加し、既存の各date-time Schemaも同じ意味検査を通す。

有効例（秒、長い小数、offset、小文字区切り）と無効例（実在しない日、zoneなし、範囲外offset、末尾改行）をConformanceへ追加し、端末内回復Schema用に実在しない暦日のnegative fixtureを追加した。`python tooling/schema_check/check_schemas.py`はSchema 125／正常例125／negative fixture 153、`python tooling/conformance_tests/run_conformance_skeleton.py`は201 checksでPASSした。`python -X utf8 tooling/日本語基底監査.py --strict`も負債0 files／0 findingsでPASSした。`python tooling/validate_all.py --python-only --desktop-platform windows`は10検査すべてPASSし、既存release blocker 5件と`release_ready=false`を維持した。これは`CONFIG`／`FIXTURE`範囲のSchema検証器の改善であり、live runtimeや実際の記録保存を証明しない。

## D4 Pocket Phase35追補: 履歴閲覧grant参照の限定例外（2026-09-25）

既存Mobile `HistoryClient`が、ownerによる事前承認後に`approval_id`と閉じたqueryを`対話履歴閲覧`で提示することを現行sourceから確認した。直前のnative Channel Schemaは権限関連keyを一律拒否していたため、正規のread-only履歴閲覧まで構造上利用不能だった。`approval_id`を`対話履歴閲覧`のみに限定した操作別payloadへ移し、他operationのauthority／approval／audit keyは引き続き拒否する。native adapterは値を権限として評価せず、現在有効なgrant、期限、実行系scopeの最終照合をRust Brokerだけが行う。

正例には現行HistoryClientの全query flagと実行系filterを用い、Agent一覧へのapproval_id混入、履歴queryのowner／authority field、未知query fieldを否定例として追加した。Schema 124／正常例124／negative fixture 150、Conformance 197 checks、strict日本語監査0 files／0 findingsがPASS。これは契約整合性であり、native transportや履歴製品経路のruntime証拠ではない。

## D4 Pocket 第35工程追記: モバイル端末連携のOS接続契約（2026-09-25）

現行Mobile production sourceを再確認し、招待JSON、端末資格、flutter_secure_storage呼出し、SecureSocket TLS／networkをDartが直接扱うrev2境界違反を確認した。nativeへの移譲前提を日本語意味正本へ明記し、Flutterからの固定channel requestをmobile_device_link_channel_request Schemaとして定義した。pair要求に引数を認めず、既存Device Link operation以外、Owner操作、authority／permission／approval／audit、招待・資格field、および再帰payload中のcredential／secret等のfield名を拒否する。channelはnative OS保管・TLS transportの境界であり、既存Desktop Rust Brokerを迂回する権限経路ではない。

Schema検査器がこれまで意味評価していなかったJSON Schema anyOf、not、propertyNames、containsを評価するよう修正し、それぞれの正常・否定をConformanceへ追加した。以前はcontainsを黙って無視していたため、Module比較Schema内のbaseline Dart define禁止条件が正しく評価されていなかった。Schema 124件／正常例124件／negative fixture 150件、Conformance 197 checks、厳格日本語監査（負債0 files／0 findings）がPASSした。git diff --checkもPASSした。

ここで得た証拠は設定値と試験用例に基づく構造確認だけである。現在はDart側に招待・端末資格・通信・安全保管の直接処理が残っており、AndroidとiOSのOS標準実装、OSの安全保管領域、Flutter境界で秘密を渡さない処理、実際のRustブローカー接続、OSによる背景移行・再起動時の実行証拠は成立していない。実装と実測を後続単位へ残し、Mobileの現行境界・実機証拠・正式配布に関するrelease blockerを維持する。

## D4 Pocket Phase35追補: 日本語監査とWindows Rust全target試験の再確認（2026-09-25）

直前のWindows環境では日本語基底監査が3 files／19 findingsで失敗し、Rust統合test executable 3件もOS policyに拒否された記録がある。これらの旧FAILは当時の履歴として保持する。現在の修正では、Rust試験用失敗説明18箇所とWindows launcher仕様の英語混在1箇所だけを日本語化し、公開識別子・Broker挙動・Protocol・authority判定は変えていない。`python -X utf8 tooling/日本語基底監査.py --strict`は負債0 files／0 findingsでPASSした。

Windowsで`cargo test --locked --manifest-path native/rust_helper/Cargo.toml -- --test-threads=1`を全targetに対して実行し、library 244件、helper binary 7件、integration 35件（`broker_ipc` 9件を含む）がすべて成功した。これにより、現環境でRust test executableが実行前に拒否されるという`windows_rust_integration_test_execution_policy`の開発上の阻害状態を解消し、registry上の項目をresolvedへ更新する。これはRust test suiteの実行証拠であり、Windows installed productやrelease readinessの証拠ではない。

補助の`rustfmt --check --edition 2021 native/rust_helper/src/broker/ipc_server.rs native/rust_helper/src/desktop_launcher.rs`は、対象file内の広範なformat差分を報告した。今回触れていない行を一括整形せず、書式差はこの機能・安全性変更と分離して扱う。対象Rust codeは全target testでcompile・実行できた。

## D4 Pocket Phase35追補: 必須source manifest漏れの修復（2026-09-25）

commit `c45a1bd8d39cd59018928fa5fa1b4572924d87f9` 後の現行mainを使った集約基準検証で、Phase35のAgent状態面に属する5つのGit追跡file（Mobile画面、Schema、正常例、拒否例2件）が`MANIFEST.sha256.json`へ未登録であることを確認した。このためmanifest検査と、それに依存するrelease gate／packaging portability検査が失敗していた。既存成果の意味や検査条件は変更せず、現行Git追跡fileとhashからmanifestを再生成した。

再検証では`python tooling/manifest.py --check`、`python tooling/packaging_portability_check.py`、`python tooling/schema_check/check_schemas.py`（Schema123・正常例123・拒否例149）、`python tooling/conformance_tests/run_conformance_skeleton.py`（195 checks）、`python tooling/release_gate_check.py`が成功した。集約検証時点の`python -X utf8 tooling/日本語基底監査.py --strict`は既存3 files／19 findings（Windows launcher仕様1、Rust IPC 12、Rust Desktop launcher 6）で失敗しており、本修復の対象外として`release_blocker`のまま保持する。Agent状態面・Device Linkの実接続やrelease readinessを、このmanifest修復から推論しない。

## D4 Pocket Phase 34追補: Flutter–Rust Broker channel契約（2026-09-24）

現行`broker_client.dart`がendpoint file・session secret・TCP socketを直接扱い、Flutter UI責務の境界を越えていることを再確認した。Windows Desktopの次期経路を、Rust起動器所有pipe → Flutter child PID照合 → Rust内relay → 既存authenticated loopback TCP Brokerへ限定する日本語契約とSchemaを追加した。relayはnormal資格だけを使い、既存Brokerのoperation判定・Audit・replay/stale検査を再実装しない。Owner経路、資格file、session secretはRunner/Dartへ渡さず、pipe不通時もfallbackしない。

この変更時点で成立したのは設計契約の`CONFIG`とnegative fixtureの`FIXTURE`範囲だけである。`python tooling/schema_check/check_schemas.py`はSchema122・正常example122・負例147、`python tooling/conformance_tests/run_conformance_skeleton.py`は192件でPASSし、`git diff --check`もPASSした。`python -X utf8 tooling/日本語基底監査.py --strict`は既存の`docs/specs/windows-desktop-launcher.md`、`native/rust_helper/src/broker/ipc_server.rs`、`native/rust_helper/src/desktop_launcher.rs`の3 files / 19 findingsでFAILし、新規channel文書・Schemaのfindingはなかった。Rust named-pipe server、Runner MethodChannel client、Flutter direct socket/file removal、pipeのPID/malformed/replay否定実行、clean Windows LIVE_RUNTIME実証は未成立であり、`rev2_flutter_broker_channel_boundary`を`release_blocker`のまま保持する。Flutter内Setup Doctor、local snapshot、export等の別filesystem/network/process使用はこのchannel契約の対象外で、別途監査を要する。

## D4 Pocket Phase 34追補: Flutter–Broker channel実装（2026-09-24、component証拠）

上記契約をproduction sourceへ接続した。Flutter BrokerClientからendpoint file読取、session secret保持、直接TCP Socketを除去し、`gui_shell/broker` MethodChannelへ要求JSONだけを渡す。Windows Runnerはrequest stringをbound・非同期でnamed pipeへ送り、response lineを返す。Rust起動器は起動ごとのpipe、remote拒否、Flutter child PID照合、normal資格relayを所有し、operation authority・Approval・Audit・Recoveryは既存Brokerに残す。Owner資格経路は追加していない。Dart payload hashと応答request ID/operation照合を維持した。

Negative / failure coverageとして、pipe componentで別PID接続拒否、relayがsession IDの偽装・malformed JSON・oversizeを既存Brokerへ送り、同じBrokerの拒否応答・Auditを確認する。Dart MethodChannel fixtureではsession/credential/role fieldを送らないことと応答binding不一致拒否を確認する。既存Flutterの実Broker TCP integration testはproduction clientから分離して`test/support`の明示的test-only transportへ移した。

検証: `cargo check --locked --manifest-path native/rust_helper/Cargo.toml --bins --target x86_64-pc-windows-msvc` PASS、`cargo build --release --locked --manifest-path native/rust_helper/Cargo.toml --bins --target x86_64-pc-windows-msvc` PASS、`cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib -- --test-threads=1`は244/244 PASS。別途並行実行した最初の全Rust試験は既存A2A loopback fixture 1件が接続失敗したが、同test単独は1/1 PASS、全体逐次再実行は244/244 PASS。named-pipe crate Windows testsは2/2 PASS。`flutter analyze --no-pub` PASS、Desktop Flutter全testは105件PASS、`flutter build windows --release` PASS。Schemaは122・正常example122・negative fixture147、conformanceは192件PASS、release runtime assertionsはfailure 0でPASS（証拠classはCONFIG/FIXTURE）。

追加のWindows実起動はDeveloper scratch限定で実施した。Run IDは`broker-pipe-1669148061d44b56b7de09f8c6fccf0a`、stage rootは`C:\Users\ohira\AppData\Local\Temp\D4PocketBrokerPipe-1669148061d44b56b7de09f8c6fccf0a\stage`。manifestのsource commitは`14387cc7cbeb3e29d15d72d1d29c8743477e4841`、`source_worktree_clean=false`、宣言上のlauncher runtimeは`per_user / isolated=false`。実際の起動ではProcess環境の`LOCALAPPDATA`を同runの`...\localappdata`へ一時overrideし、runtime store fileが同scratch内に生成されたことを別途確認した。app/broker/launcher SHA-256は`683676fee10cf51a98bbf9c22eabd20838f125f21fa49bec3646266d3a55afb3` / `30afbe8823da991944e30d3925ffeba165a16cb8136eeed4d463fe772ef98696` / `c39f67d703e0c3b7bb57e67dd37875acb6e76797e45724cdf66cca14e59b221a`。stage画面の実画面/UIAはtitle `D4 Pocket`、snapshot source `broker`を示し、runtime storeの永続`audit.jsonl` 16 records中にDesktop起動`recorded / LIVE_RUNTIME`と`health / accepted / LIVE_RUNTIME`を確認した。対象はこの変更を含むdirty worktreeからのmanual stageで、正式release collector、clean commitからのstage、通常tray終了操作は未実施。この実測はRunner→Rust pipe→既存Brokerの通常health往復・永続Auditという限定的`LIVE_RUNTIME`証拠だが、負例全体やrelease smokeを証明しない。

`python -X utf8 tooling/日本語基底監査.py --strict`は既存負債の`docs/specs/windows-desktop-launcher.md`、`native/rust_helper/src/broker/ipc_server.rs`、`native/rust_helper/src/desktop_launcher.rs`の3 files / 19 findingsでFAIL。今回の追加行によるfinding増加はない。対象の`desktop_launcher.rs`と`windows_broker_channel` crateは個別`rustfmt --check` PASS。`cargo fmt --manifest-path native/rust_helper/Cargo.toml -- --check`は他moduleを含む既存複数箇所のformat driftを列挙してFAILしたため、対象外fileの一括formatは行っていない。

以上により、normal health transportのWindows実行経路は限定実測したが、`rev2_flutter_broker_channel_boundary`は解除しない。Owner-only/credential/authority/session拒否、別process・別起動pipe、stale/replay、Broker停止・pipe障害時no-fallback、clean committed productのrelease smokeと正常終了は未証明である。別途Setup Doctor、snapshot、export等に残るFlutter filesystem/network/process経路も未解決である。

- item: Windows product Runner→pipe→Broker end-to-end実測
  classification: release_blocker
  reason: dirty worktreeのDeveloper scratchでRunner→pipe→Broker health往復とLIVE_RUNTIME永続Auditは確認したが、clean committed productの正式smoke、Owner-only/authority/credential/session拒否、別process・別起動pipe、stale/replay、Broker停止時no-fallback、通常tray終了は未確認。
  required_action: clean commitから分離配置したWindows productをformal collectorで起動し、health/Auditと列挙済みnegative・failure経路・正常終了を証拠bundleへ結合する。別Flutter filesystem/network/process surfaceは独立契約と負例試験へ接続する。
  blocks_release: yes

- item: Flutter内Broker以外のfilesystem/network/process surface
  classification: release_blocker
  reason: BrokerClientの直接アクセスは除去したが、Setup Doctor、snapshot、export等の独立surfaceは本作業で移譲していない。
  required_action: production Dart sourceの全surfaceを列挙し、権限依存作用をRust Brokerへ移すか、UI-only projectionとして契約・実行境界を検証する。
  blocks_release: yes

## D4 Pocket Phase 34追補: clean commitからのWindows staged実起動（2026-09-24）

commit `1674c3b05f58fb7c2e53ffb2ee67089c46ce4432`のclean sourceからWindows Releaseを再buildし、Developer staging root `C:\Users\ohira\AppData\Local\Temp\D4Pocket-PostCommit-1674c3b-20260924`の`gui_shell_desktop_launcher.exe`で起動した。manifestは`source_worktree_clean=true`、source commit一致を記録する。artifact SHA-256はFlutter app `fcbdc9aac97b64f680eef1709ba4d90f8475f23833a293f83d16fec2582daa45`、Broker `52126f7b53e8cdf708db43a31842ad060f101895de49c21fdf45872210475c3c`、起動器 `1c21080ac0bb8da61c60114700f50853d3eb179064682b1ffb8d02f632f7f508`。

- Windows画面観測: titleは`D4 Pocket`。Accessibility treeと実画面は`snapshot_source=broker`、`broker_session=restricted (authenticated_loopback_tcp)`、Audit `durable_file_store`、不変条件`ok`、Runtime `suspend`、release `not claimed`を表示した。実測Auditの起動event `broker-audit-66`は`LIVE_RUNTIME`。製品command dispatchは引き続き停止し、通常資格から権限を昇格していない。
- 限界: 起動器runtimeは`C:\Users\ohira\AppData\Local\GUI-Shell\broker\desktop`の既存per-user Storeであり、manifestも`isolated=false`、`evidence_class=CONFIG`、`formal_runtime_proof=false`。別Windows user profileではなく、official installed smoke collectorも通していない。従ってこれはclean-sourceのmanual staged `LIVE_RUNTIME`観測に限り、正式installed product／初回起動gateを満たさない。
- 終了: 通常tray `終了`経路は未検証。今回の検査processは実画面確認後も実行中で、終了eventは未観測。この状態を正常終了・lifecycle完了へ読み替えない。
- release_blocker: collector起動器経由化、分離Windows profileでの実runtimeと終了Audit、Flutter内Dart file／credential／socket除去、formal Installer／Uninstaller／identity／signing／update／rollback、その他registryの既存blockerを維持する。

## D4 Pocket Phase 34追補: Windows staged Desktop起動器（2026-09-24）

staged Windows配置の標準entry pointとして、Windows GUI subsystemのRust起動器を追加する。起動器は既存Brokerを同一process内の管理threadで起動し、固定配置を検証したFlutter executableへ既存normal Broker endpoint pathとruntime directoryだけを渡す。session secretをcommand line／environmentへ複製せず、FlutterにOwner資格を渡さず、子process終了後はBrokerへprocess内部停止を通知する。durable storeは保持し、endpointは起動時byteと停止後byteが一致する場合だけ消去する。

- 配布境界: `installer/windows/stage_installed_app.ps1`はDeveloper staging toolのまま必須の`-DesktopLauncherExe`を受け取りrootへcopyし、hashをmanifestへ記録する。旧CMD／PowerShell launcher fileをproduct rootへ生成しない。
- Authority／Audit: 起動要求・起動器管理Broker終了を既存永続Audit chainへ記録する。監査append失敗時はUIを起動せず、起動器はCapability／Permission／Approvalをprivileged operation用に生成しない。終了時も管理対象Brokerのみ停止し、durable storeを維持する。
- Negative: loopback以外、Owner role、未知transport、不正secret形式、過大request上限、endpoint差替えcleanup、重複起動、必須package file欠落を拒否する。
- 既存Gap: Windows installed smoke collectorはFlutter executableを直接起動する現状のままで、この起動器とlifecycle Auditを通す正式evidenceではない。Flutter `BrokerClient`のDart file read／session secret保持／loopback socketもD4指定のRust側責務とのGapとしてrelease blockerへ追加し、今回の変更では修正済みと主張しない。
- 未成立分類: 正式Installer／Uninstaller、Download→Install→Launch、Signed Update、Rollback、formal package identity／signing、installed product LIVE_RUNTIME証拠、GUI Shell Export／Module Pruning、owner GO、正式releaseは`release_blocker`。Phase 33も独立に未完了。

## D4 Pocket Phase 34追補: Windows staged起動実測と初期化契約修正（2026-09-24）

修正版Desktop Releaseをstaged配置へ置き、Rust Desktop起動器からWindows GUIを実起動した。Flutter起動直後の初期化要求に既存Broker契約との不一致があり、画面がBroker利用不可へfail-closedすることをAuditと表示の両方で確認した。`Agent一覧`はBroker側で空objectだけを許可していたが、Flutterは版metadata付きobjectを送り、最初の実行で拒否された。metadataを省略した再試行もBrokerで`null`となり拒否されたため、Flutter要求を明示的な空object `{}`へ修正した。再build後は製品画面の初期化がBroker snapshotまで完了した。

- Windows実測対象: 最終buildは`C:\Users\ohira\AppData\Local\Temp\D4Pocket-Launcher-Smoke-20260924-Retest-03`。manifestはsource commit `a3c8dc573afe609e8b0c5400584d2165a5b9a11a`、`source_worktree_clean=false`を記録する。Flutter artifact SHA-256 `fcbdc9aac97b64f680eef1709ba4d90f8475f23833a293f83d16fec2582daa45`、Broker `87f2a594d10662caef748f867e06c7a9c1945fd84e9f8490c49bc769d70439a6`、起動器 `fb6390082bf72fc8e791a64f549fbd4fc5a48861ce1a126e232e859b28dd97a6`。manifestはcollector scratch runtimeと起動器runtimeを分け、後者を`%LOCALAPPDATA%\GUI-Shell\broker\desktop`、`scope=per_user`、`isolated=false`、`evidence_class=CONFIG`、`formal_runtime_proof=false`として宣言した。dirty sourceによるDeveloper stageでありclean commit／formal installed evidenceではない。
- 実行経路と観測範囲: stage rootの`gui_shell_desktop_launcher.exe`→同一processのRust Broker→固定配置`app/gui_shell_desktop.exe`。Windows native window titleは`D4 Pocket`。画面とaccessibility treeは`snapshot_source=broker`、`broker_session=restricted (authenticated_loopback_tcp)`、Audit `durable_file_store`、不変条件`ok`、完成製品release未主張を表示した。health、Host能力／一覧、Adapter一覧、Agent一覧、normalize、redacted content projection、作業領域一覧、Profile／Update／Notification／Observation一覧が通常Broker要求として受理された。保護field編集probeは製品projection上で`rejected`、`command_envelope`は`suspended`で、いずれも権限付与へ昇格していない。
- Lifecycle: 最終runの起動・終了は永続Audit chainへ`LIVE_RUNTIME`で記録され（終了event `broker-audit-65`）、endpoint fileは終了後に不在、durable storeとAudit chainは保持された。検証後、正確なstage pathのFlutter childだけを停止したため起動器は`FRONTEND_EXIT_FAILED`を表示した。通常tray Exit経路は今回確認していないため、手動staging smokeを正式なinstaller初回起動証拠へ昇格させない。
- 追レビュー修正: staged実測artifactの作成後、Broker `store` directory自体をjunctionで外部へ向けられる検査漏れを発見した。起動器はStoreを再帰作成せず、通常directory・root内canonical pathを検査するよう修正し、junction拒否negative testを追加した。後述のlibrary testには含むが、上記staged artifact hashとGUI実測にはこの追レビュー修正が含まれないため、修正後のclean-source staging再実測が必要。
- 検証: 単独実行した`flutter test`は105件PASS、`flutter analyze`はDesktop／MobileともPASS、`flutter build windows --release` PASS、`cargo build --release --locked --manifest-path native/rust_helper/Cargo.toml --bin gui_shell_rust_helper --bin gui_shell_desktop_launcher` PASS。`python tooling/schema_check/check_schemas.py`はSchema 121／example 121／negative fixture 146、`python tooling/conformance_tests/run_conformance_skeleton.py`は191 checks PASS。単独実行したRust library testは242件PASS（junction拒否を含む）。
- 実行環境による未成立: full `cargo test --locked --manifest-path native/rust_helper/Cargo.toml -- --test-threads=1`はBroker子process起動がWindows App Control／Code IntegrityのOS error 4551で拒否され、`tests/broker_ipc.rs`の9件を実行できず全体exit 1。これはhost policyにより未実行であり、Product PASSへ読み替えない。該当release blockerを維持する。
- Staging manifest境界: `runtime_dir`／`store_dir`はcollector用scratch pathで、標準起動器は`launcher_runtime`に宣言するper-user `%LOCALAPPDATA%\GUI-Shell\broker\desktop`を使う。後者は`isolated=false`、`evidence_class=CONFIG`、`formal_runtime_proof=false`であり、実runtimeを分離user profileで測った証拠ではない。
- `release_blocker`: Flutter `BrokerClient`のDart内file read／session secret保持／loopback socketをRust境界へ移譲する経路、公式installed smoke collectorの起動器経由化と実runtimeを隔離したclean evidence、formal Installer／Uninstaller／署名／identity／update／rollback、Phase 33の製品binary pruningと実行時性能、外部Runtime／Agent／MCP／A2A、実端末、C28長時間実測、owner GO、正式release。

## D4 Pocket Phase 33追補: Windows同一commit AOT比較とsurface node実証（2026-09-24）

commit `aa3f2eac4f829d230a782fbd5f5cf7fc58d79c6c`をsourceとして、all-enabled baselineとReceipt選択buildを同じWindows Flutter toolchain／Release条件で実行した。比較tool自身がAOT reportからCatalogのsurface library一覧を読み取り、新Schemaへ記録した。保存evidence・両report・両artifact・snapshot・precompiler traceをread-backし、Schema／Receipt対応とsize・SHA-256・tree hashを照合した。

- 実行command: `python tooling/compare_module_builds_windows.py --receipt examples/contracts/gui_shell_export_receipt.valid.json --artifact-dir C:\Users\ohira\AppData\Local\GUI-Shell\developer-module-builds\comparison-aotnodes-aa3f2ea`。出力はRepository外のCodex LocalCacheへ転送され、evidenceは`C:\Users\ohira\AppData\Local\Packages\OpenAI.Codex_2p2nqsd0c76g0\LocalCache\Local\GUI-Shell\developer-module-builds\comparison-aotnodes-aa3f2ea\comparison_evidence.json`。
- 対象環境: Flutter `3.44.0`、Dart `3.12.0`、framework revision `559ffa3f75e7402d65a8def9c28389a9b2e6fe42`。Baselineは8任意画面全有効、選択buildはObservabilityとTrace Inspectorを選び、Setup Doctor、History、Evaluation Lab、Host Capabilities、Notifications、Host Operationsを無効化した。Receipt SHA-256 `066280fc3dc4a2edc5d03ddcd53ba266f0a74425cbb8a46da8138bdd3c78e24c`。
- Artifact: baseline `30,892,494 bytes`、tree SHA-256 `2fae275940157070320bd72fc782e06cb6ec2d59a331ed2b3113186d4458ce98`。選択build `30,663,118 bytes`、tree SHA-256 `8182ece8cfb5c989052642a977d1d5119a9124029d209dc4851645443828e1d2`。減少は`229,376 bytes`（224 KiB）で、差分artifactは`data/app.so`だけ。baseline `data/app.so`: `6,816,656 bytes`, SHA-256 `bb165b36298e462b6d0545354dfd2e0f45f30d44ab727b3e97fa53999dd2e747`; selected: `6,587,280 bytes`, SHA-256 `bb327951056422bfb15dcf1a2a17731aaf2c60ba1187db68fe8c7d28a213e6b8`。それ以外のartifact fileは同一hash・size。
- AOT report: baseline report SHA-256 `6e7bbb538145f67bb32d1d2894a2d3d4c80e6dee85929a4fd61d8a2a5fcd3bd4`; selected report SHA-256 `108d524db7560265428c2f4d8786d71b308569d3bc0a4ca12f785d52b784174e`。baseline reportにはCatalogの8 surface library全件が存在し、選択reportにはObservabilityとTrace Inspectorの2件だけが存在することを抽出・照合した。size-analysis snapshot／traceと全artifact fileもevidenceに記録されたsize・SHA-256へ一致した。
- Build経過時間はbaseline `52,143 ms`、選択`51,278 ms`。これはbuild所要であって製品起動速度ではない。`binary_pruning_verified=false`、standalone／product claim false、cold startupとresourceは`not_measured`のまま。
- 検証: tool exit 0。保存JSONのSchema・Receipt・ModulePlan一致、AOT reportからのsurface再抽出、report／snapshot／trace／artifactの実size・SHA-256・tree hashを独立read-backして一致確認。比較tool実装commitの集約validationもPASS（Schema 121、Conformance 190、strict日本語監査findings 0、manifest、packaging、release gate、smoke、runtime assertion、C32監査。development mode、`release_ready=false`）。
- 未成立分類: `release_blocker` `rev2_module_pruning_binary_and_measurement`は継続する。Flutter compiler reportのlibrary node不在は、このUI AOT reportにおけるnode境界の証拠であり、共有symbolや画面意味全体の不在、製品binary内の安全Core保持、runtime挙動、Rust／third-party pruningを証明しない。独立製品Export、hash結合された製品cold startup／resource計測も未成立。

## D4 Pocket Phase 33追補: AOT surface証拠の機械検査（2026-09-24）

Windows比較toolを拡張し、Flutter size-analysis reportのDart AOT treeからCatalog記載surface library nodeを抽出するようにした。baselineは全Catalog surface、選択buildは選択Moduleに対応するsurfaceのみを記録し、ConformanceがReceipt／Catalog／defineと一致しない残存・欠落を拒否する。選択外library残存と選択済library欠落のnegative testを追加した。`binary_pruning_verified=false`は維持し、AOT report nodeの一致を実行時・独立製品の証拠へ昇格させない。

- 既存の`comparison-d732047`（source commit `d7320470f9d75608c68f24fcbf21660dc3d346a8`）に含まれるbaseline／selected reportへ、新しい読取専用抽出関数を適用し、baseline 8 library、selected 2 library（Observability、Trace Inspector）の期待集合と一致することを確認した。保存済みevidence JSONやartifactは変更していない。
- 今回の検査: Python構文検査PASS、Schema 121件・正常example 121件・negative fixture 146件PASS、Conformance 190 checks PASS。実buildに新fieldを記録した正式比較evidenceは、この実装を含むclean commit後に再生成して別追補へ記録する。
- 未成立分類: release blocker `rev2_module_pruning_binary_and_measurement`は継続する。AOT reportのsurface node確認は限定的なFlutter compiler evidenceであり、最終製品意味・runtime挙動・Rust／third-party除去・安全Core保持、cold startup、実行時resource、独立Export起動の証拠ではない。

## D4 Pocket Phase 33追補: Windows Developer Release build実証（2026-09-24）

commit `f0e9a40bd279b25c36fcefd6a65a50a3c1a80c9c`から、Receiptの画面選択をcompile-time defineへ渡すWindows Release AOT buildを実行し、全artifact file hashを採取した。Developer専用のDesktop Flutter UI build経路とそのhash採取は成立したが、独立製品、実導入・起動、実binary pruningの証拠ではない。

- 実行command: `python tooling/build_module_pruned_windows.py --receipt examples/contracts/gui_shell_export_receipt.valid.json --artifact-dir C:\Users\ohira\AppData\Local\GUI-Shell\developer-module-builds\phase33-f0e9a40`。build補助は出力先をCodex Windows packageのLocalCacheへ仮想化して保存した。採取先は`C:\Users\ohira\AppData\Local\Packages\OpenAI.Codex_2p2nqsd0c76g0\LocalCache\Local\GUI-Shell\developer-module-builds\phase33-f0e9a40`で、Repository外である。
- Windows build実証: toolがclean worktreeとsource commitを確認し、Flutter `3.44.0`／Dart `3.12.0`／framework revision `559ffa3f75e7402d65a8def9c28389a9b2e6fe42`で`flutter --suppress-analytics build windows --release --no-pub`と8つのdefineを実行した。build所要`47971 ms`、artifact合計`30663118 bytes`、tree SHA-256 `8182ece8cfb5c989052642a977d1d5119a9124029d209dc4851645443828e1d2`。Receipt SHA-256は`066280fc3dc4a2edc5d03ddcd53ba266f0a74425cbb8a46da8138bdd3c78e24c`。 evidence JSONは出力先の`build_evidence.json`に保存し、Schema照合済み。
- 選択内容: 全必須画面と安全Coreを保持し、任意画面はObservabilityとTrace Inspectorを含めた。Setup Doctor、History、Evaluation Lab、Host Capabilities、Notifications、Host Operationsは選択外。Receiptは未信頼の画面選択JSONとしてだけ扱い、Owner権限を検証・獲得していない。
- failureと修正履歴: 最初の起動はPowerShellが解決する裸の`flutter`名をPython子processが直接起動できず、`WinError 2`でbuild前に停止した。PATH上の実物が`flutter.bat`であることを観測し、toolを`flutter.bat`／`flutter.cmd`／`flutter.exe`探索へ修正した。独自wrapperや環境回避策は追加せず、明示fileを直接呼ぶ再実行でRelease buildとevidence生成に成功した。
- 証拠の上限: evidenceは`INTERNAL_STATE`で、`product_artifact_claimed=false`、`standalone_app_claimed=false`、`authority_verified=false`、`binary_pruning_verified=false`、`signed=false`を固定する。`compile_time_defines_applied_binary_comparison_pending`であり、画面codeが最終binaryから除去されたかを比較・調査していない。artifactを起動・installせず、同一commit／toolchainでのall-enabled baselineも作成していない。
- このbuild実行後に確認した検査: Schema 120・正常example 120・negative fixture 145、Conformance 189 checks、strict日本語監査findings 0、manifest check、packaging portability check、Python構文検査、plan-only依存閉包検査はPASS。Flutter実物probeは上記3.44.0／3.12.0／revisionを返した。Desktop／Mobile `flutter analyze`とDesktop 105試験は前段の同一DartソースでPASSし、選択無効化focus test 2件もPASSした。集約`validate_all.py --python-only --desktop-platform windows`は直前commit `3e4403799eae4bc44c1e034f24a7ad16c8cdc1fc`でPASSしており、後続のFlutter command resolver修正commitには再実行していない。
- 未成立分類: baselineと選択artifactの同一条件・hash結合比較、binary内容の除去実証、size差、cold startup、resource比較は`release_blocker`（`rev2_module_pruning_binary_and_measurement`）。Owner操作資格付きGUI Export経路は`release_blocker`（`rev2_export_owner_ui_authority_path`）。artifact単体の起動・Installer・署名・installed product証拠も独立release gateに残る。

## D4 Pocket Phase 33追補: Flutter画面選択build経路（2026-09-24）

Phase 33の既存ModulePlanをFlutter Desktopの画面表示選択へ接続するDeveloper専用build補助を追加した。8つの任意画面をcompile-time defineで個別に選択でき、定義省略時は従来どおり全画面を有効にする。Trace Inspectorを選ぶ場合はObservabilityを依存として含める。必須画面の位置・表示と安全Coreの保持条件を変更しない。

- 実装面: `apps/desktop_flutter/lib/main.dart`の任意画面・NavigationRail・画面切替・関連Command Palette項目をcompile-time defineで制御する。対象画面を含まない古いnavigation要求は例外にせず、日本語の非搭載案内を表示する。`tooling/build_module_pruned_windows.py`はDeveloperが明示実行し、Export Receipt JSONを未検証の画面選択入力としてSchema照合・依存閉包検査後、Windows Flutter Release buildへdefineを渡す。製品RuntimeとExport UIからこのtoolを呼び出さない。
- 信頼・証拠境界: Receipt JSONの署名、出所、Owner操作の真正性は検証しない。選択入力はAuthorityではなく、`authority_verified=false`を記録する。出力証拠も`product_artifact_claimed=false`、`standalone_app_claimed=false`、`binary_pruning_verified=false`を固定する。これはDesktop Flutter UIのbuildに限られ、Rust／第三者Moduleの除去、独立App構成、Installer、実製品配布を証明しない。
- Contract・回帰: `specs/gui_shell_module_build_evidence.schema.json`と正常・負例fixtureを追加し、Schema検査・Conformanceへ接続した。正常例は権限検証・製品・独立App・binary除去を主張できず、権限または製品完成を偽って主張する2負例は拒否する。全Phase 0〜45と既存rev1 C0〜C34の要求範囲対応を`docs/D4_POCKET_PHASE_MAPPING.md`へ追加した。対応表は範囲の紐付けだけで、旧工程のPASS・失敗・証拠を移転または上書きしない。
- この追補作成前に実行した確認: `python tooling/schema_check/check_schemas.py`（Schema 120、正常例120、negative fixture 145）、`python tooling/conformance_tests/run_conformance_skeleton.py`（189 checks）、`python tooling/build_module_pruned_windows.py --receipt examples/contracts/gui_shell_export_receipt.valid.json --plan-only`（成功。Trace InspectorとObservability依存を含む計画を表示し、Authority検証false・binary pruning未検証を明示）、Desktop／Mobileの`flutter analyze`（問題なし）、Desktop `flutter test --no-pub`（105件成功）、8画面無効の`flutter test --no-pub --dart-define=GUI_SHELL_MODULE_SETUP_DOCTOR=false --dart-define=GUI_SHELL_MODULE_HISTORY=false --dart-define=GUI_SHELL_MODULE_EVALUATION_LAB=false --dart-define=GUI_SHELL_MODULE_HOST_CAPABILITIES=false --dart-define=GUI_SHELL_MODULE_NOTIFICATIONS=false --dart-define=GUI_SHELL_MODULE_OBSERVABILITY=false --dart-define=GUI_SHELL_MODULE_TRACE_INSPECTOR=false --dart-define=GUI_SHELL_MODULE_HOST_OPERATIONS=false test/module_pruning_test.dart`（2件成功）、`python tooling/日本語基底監査.py --strict`（findings 0）。工程表対応表の日本語修正後にConformance・manifest・梱包・集約検証を再実行し、この作業単位の最終結果を確定する。
- 未成立分類: 同一commit・toolchainに固定した全画面baselineと選択buildのartifact比較、binaryからの除去実証、差分size・cold startup・resource実測は`release_blocker`（`rev2_module_pruning_binary_and_measurement`）。Owner操作資格付きGUI Export経路は`release_blocker`（`rev2_export_owner_ui_authority_path`）。実Windows Release build結果は本追補作成時点で未実行であり、実行結果を別の追補へ記録する。Installer・署名・導入済み製品証拠も既存release gateに従って未成立のままとする。

## D4 Pocket Phase 33: Module Pruning選択計画（2026-09-24）

Desktop画面Module一覧、必須Module、任意選択、依存閉包をSchema・Rust Export Broker・設定画面に接続した。Authority、Approval、Audit、Recovery等の必須Moduleは選択入力で外せず、Trace Inspector選択時はObservabilityを依存として含める。旧要求で選択fieldがない場合は全任意Moduleを保持する。

- Contract／経路: `specs/gui_shell_module_catalog.json` → Export要求の`module_selection` → Rust Brokerによる一覧・依存再検査 → Export Receiptの`module_plan`。任意画面チェックはUI stateであり、AuthorityやPermissionを生成しない。
- 既知の経路欠陥: 既存Phase32記録の「Desktop SettingsからOwner制御資格付きBrokerへ到達する」というProduction path記述は正確でなかった。現行Flutter `BrokerClient`は通常資格endpointを使い、Rust BrokerはExportをOwner専用として拒否する。Owner秘密値をFlutterへ渡す回避は採らず、`rev2_export_owner_ui_authority_path`をrelease blockerとして登録した。
- 固定安全境界: Catalog欠落がSchemaとCatalogを同時に縮小すれば通る隙を監査で検出した。Rust BrokerがCore 8 IDと必須画面10 IDを固定値と照合し、Catalog／Receipt Schemaの件数を固定、Conformanceでも全IDを独立照合し、欠落Coreのnegative fixtureとRust負例を追加した。
- 証拠境界: Receiptは`binary_pruning_status=not_applied`を固定する。Module選択・依存閉包の検証は実施するが、artifactからのコード除去やサイズ、cold startup、resourceの測定はこの単位で成立しない。
- 未成立分類: Owner明示操作のBroker統治経路は`release_blocker`（`rev2_export_owner_ui_authority_path`）。実binary除去と、hashで比較対象を固定したサイズ・cold startup・resource測定は`release_blocker`（`rev2_module_pruning_binary_and_measurement`）。Windows Enterprise signing policyが3つのRust integration test executable（計17件）を起動拒否し、完全実行証拠がないことも`release_blocker`（`windows_rust_integration_test_execution_policy`）。Installer・署名・配布・installed product証拠も独立したrelease blockerのままである。
- 検証結果: `python tooling/schema_check/check_schemas.py`はSchema 119・正常example 119・negative fixture 143、`python tooling/conformance_tests/run_conformance_skeleton.py`は188 check、`python tooling/日本語基底監査.py --strict`はfindings 0、`python tooling/manifest.py --check`、`python tooling/packaging_portability_check.py`、`python tooling/validate_all.py --python-only --desktop-platform windows`はPASS。全体validationはdevelopment modeでありrelease_ready=false。Rustは`cargo test --no-run --tests`が全target compile PASS、`cargo test --lib --bins -- --test-threads=1`がlib 235・main 7の計242件PASS、Export対象unit testが6件PASS。integration targetは6 suite・計18件PASSした。一方、`broker_ipc` 9件、`protected_startup` 1件、`workspace_startup` 7件はWindows Code Integrity Event 3033／3077によりEnterprise signing level不適合として起動前に拒否された（OS error 4551）。拒否された17件は未実行でありassertion failureではない。OS保護policyは変更・回避していない。Flutter `analyze`は問題0、desktop testは104件PASS、Windows debug buildも成功。計画Receiptやfixtureだけを実製品のpruning／起動／資源証拠として報告しない。

## D4 Pocket 第8段階 Windows書出し（2026-09-24）

GUI Shell構成Manifestを、Windows向け独立Appの初期Manifestへ変換するBroker経路を追加した。現行単位は実artifactを作らず、書出し先のidentityと監査storeを新規生成し、設定、Runtime／Adapter構成、Capability requirement、配布metadataを返す。

- Contract: `specs/gui_shell_export.schema.json`と`specs/gui_shell_export_receipt.schema.json`を追加し、Windows、`manifest_only`、新規App identity、新規監査store、`authority_strip=true`、Credential／Permission／Approval／Audit chain非継承、`not_built`、installer未開始、未署名を固定した。
- Production path（後日監査で訂正）: Desktop Settings → `ExportClient`は通常資格endpointへ送信する。Rust Brokerの`GUI Shell書出し`はOwner制御資格が必要なため、現行Flutterからの要求は拒否される。Broker Owner処理自体はunit test経路に存在するが、これをDesktopから使える製品経路として扱わない。
- Negative boundary: 通常資格、未知field、構成Manifestの継承要求、書出し先の継承済みReceiptを拒否する。書出し元Capability requirementをPermissionへ昇格せず、既存のAuthority、Approval、Credential、Audit chainを再利用しない。
- Validation: Schema 118件、正常example 118件、negative fixture 139件、Conformance 188件、厳格日本語監査、Manifest検査、梱包可能性検査、`validate_all.py --python-only --desktop-platform windows`、Flutter `analyze`／全104試験、Windows debug build、Rust全試験（lib 231件、main 7件、統合9・1・8・2・2・7件）がPASSした。A2A loopbackのWindows試験fixtureは応答送信後の`Shutdown::Write`をやめ、通常の接続終了で安定化した。
- 未成立分類: 実artifact生成、Installer、署名、Module Pruning、Distribution、書出し先の実起動、Windows installed productの正式証拠は`release_blocker`。Manifest Receiptだけで独立App完成や正式releaseを主張しない。

## D4 Pocket 第7段階 編集提案（2026-09-24）

GUI Shell編集提案の開発経路を追加した。Owner／Developerが明示開始した構成・UI・Contract変更候補を、認証済みRust Brokerが対象path、規約確認、自己承認禁止、提案専用実行を再検証し、審査待ちReceiptへ射影する。製品Runtimeの自己変更や自動applyは行わない。

- Contract: `specs/gui_shell_edit_proposal.schema.json`と`specs/gui_shell_edit_proposal_receipt.schema.json`を追加し、対象path境界、`owner_directed_agent`、`proposal_only`、`review_required`、`files_written=false`、`permission_generated=false`を固定した。
- Production path: Desktop Settings → `AiEditClient` → owner資格付きRust Broker `GUI Shell編集提案` → 提案再検証 → `INTERNAL_STATE` AuditEvent付き審査待ちReceipt。指示本文は返さず、要求hashだけをReceiptへ結合する。
- Negative boundary: 通常資格、self approval、未知field、許可外path、secret／credential／private／signing path、自動applyを拒否する。提案metadataはAuthority、Permission、Approval、Credentialを生成しない。
- 未成立分類: 提案を実差分へapplyするOwner／Developer操作、独立App Export、Module Pruning、Distributionは`release_blocker`。提案Receiptだけで実装完了、自己変更、製品releaseを主張しない。

## D4 Pocket 第6段階 構成Preview（2026-09-24）

GUI Shell構成のPreview経路を追加した。現在Manifestと候補Manifestを認証済みRust Brokerへ渡し、実行基盤・エージェント・ツール・MCP接続・機能要件の差分、対象platform、版rollbackの可否を決定論的に返す。Desktop設定画面はPreview結果を表示する。

- Contract: `specs/gui_shell_preview.schema.json`と`specs/gui_shell_preview_receipt.schema.json`を追加し、`version_rollback`、差分配列、機能要件の`not_generated`／`not_requested`、`rollback_available=false`を固定した。
- Production path: Desktop Settings → `ComposeClient` → 認証済みRust Broker `GUI Shell構成Preview` → Manifest再検証・差分計算 → `INTERNAL_STATE` AuditEvent付きPreview Receipt。build、Export、rollbackは実行しない。
- Negative boundary: Permission継承、未知field、不正対象platform、候補Manifestの不正値を拒否する。Capability requirementは説明へ留まり、PermissionまたはApprovalを生成しない。
- 未成立分類: Windows Export、独立アプリ識別子／監査ストア／設定／実行基盤Manifestの生成、Module Pruning、Distributionは`release_blocker`。Preview表示だけでbuild、Export、rollback、独立アプリ完成を主張しない。

## D4 Pocket 第5段階 GUI Shell構成（2026-09-24）

GUI Shell構成のManifest-only経路を追加した。実行基盤、エージェント、ツール、MCP接続、表示テーマ、機能要件、設定を選択要求へまとめ、Rust Brokerが構造・重複・継承禁止を再検証して`GUI Shell構成`Receiptを返す。Desktop設定面は認証済みBrokerへ接続し、結果Manifestを表示する。

- Contract: `specs/gui_shell_compose.schema.json`と`specs/gui_shell_compose_receipt.schema.json`を追加し、`manifest_only`、`not_started`、`not_generated`、権限非生成、Authority Strip、継承値`none`を固定した。
- Production path: Desktop Settings → `ComposeClient` → 認証済みRust Broker `GUI Shell構成` → 構成Manifestの再検証 → `INTERNAL_STATE` AuditEvent付きReceipt。ビルド、独立アプリ識別子、資格情報、Permission、Approval、Audit chainは実行・継承しない。
- Negative boundary: 未知field、Permission継承、重複選択、不正locale、ReceiptのPermission生成を拒否する。Capability requirementは要求説明に留まり、権限を生成しない。
- 未成立分類: Windows Export、独立アプリ識別子／監査ストア／設定／実行基盤Manifestの生成、Preview／rollback、Module Pruning、Distributionは`release_blocker`。ComposeのManifest表示だけで独立アプリ完成を主張しない。

## D4 Pocket C33: Windows最大到達点（2026-09-24）

Windowsで実行可能な範囲を最大化するため、release build、Rust helper release build、Broker smoke、installed smokeの証拠収集経路を整備した。`installer/windows/collect_broker_smoke.ps1`はWindows Rust testのfull-duplex応答を維持して読み取り、`collect_installed_smoke.ps1`はUTF-8の製品JSONを明示的に読み取る。UI surface収集は親要素と同じRaw UI Automation treeを再帰走査する。いずれも、証拠collectorの責務であり、Flutterへ権限経路を追加しない。

- Validation: `flutter build windows --release`、`cargo build --release --locked`、`python tooling/conformance_tests/run_conformance_skeleton.py`（182 checks）、`python tooling/validate_all.py --python-only --desktop-platform windows`（exit 0）はPASS。`python tooling/windows_release_evidence.py --evidence C:\Users\ohira\AppData\Local\GUI-Shell\installed-runs\c33-2c8ad7d\runtime\evidence\windows_installed_smoke.json`は、正式Windows証拠が未成立のためexit 1となった。
- Clean isolated run: source commitは`2c8ad7ddd033ef97d90989341450b37f84a87d76`、source worktreeはclean。Broker smokeはcollector v4で`passed`となり、認証IPC、永続store、replay拒否、再起動後health、crash fail-closedを確認した。installed smoke自体はexit 0だが、`installed_status=failed`、`visible_surfaces=[]`、`visible_complete=false`、`setup_doctor_status=warning`、`setup_doctor_path=false`である。
- Evidence boundary: Windows UI Automationの正式collectorはroot／Flutter viewの2要素だけを観測し、個別surfaceを証明できない。Computer Useで個別semantic surfaceを補助観測できても、正式collectorのLIVE_RUNTIME／EXTERNAL_EVIDENCEを代替しない。Setup Doctorは製品プロセスのresolved executableがCodexホストのLocalCache配下となり、installed contextの厳密なpath一致を満たさない。path比較を緩めて成功扱いにはしない。
- `release_blocker`: `windows_evidence_provenance_isolation`、`windows_installer_first_run_smoke`、`windows_setup_doctor_smoke`、`audit_anchor_external_tamper_evidence_proof`、C28の8時間実測、外部Runtime／Agent／MCP／A2A、実端末、正式署名・配布、owner GO、正式release。
- `known_limitation`: Broker installed smokeの個別sub-gateはPASSしたが、総合installed evidenceの成立へは昇格させない。UI AutomationのFlutter surface取得限界とCodexホストの実行path virtualizationは、現環境で観測した制約である。

この単位はWindows最大到達点の開発候補であり、製品releaseやC34正式release前作業の完了を意味しない。

## D4 Pocket C31: 文書更新（2026-09-24）

README、ROADMAP、GUI操作面、SECURITY、CONFIG、MOBILE_STATUS、COMPATIBILITY_MATRIX、および本書を、C24〜C30の現行実装・検証範囲へ更新した。更新の目的は、製品の未検証範囲を隠さず、各面のproduction path、Authority境界、Content Exposure境界、証拠class、残存分類を同じ状態へそろえることである。過去の文書記録に含まれる旧Schema／Conformance件数、旧platform証拠、旧工程状態は履歴として保持し、現行commitのPASSへ読み替えない。

- 現行基準: Schema 108件、正常example 108件、negative fixture 129件、Conformance 182件、厳格日本語監査、manifest検査はPASS。
- 現行回帰: C27性能smoke、C28 30秒運用smoke、C29障害注入8件、C30回帰matrixはPASS。C28の8時間実測は途中で通常対話通信失敗となり、PASSへ昇格していない。
- 証拠境界: C30 Agent probeはCodex CLI version/help interfaceだけ、Compare／Device Linkはfixture、Runtime／Dialogueはlocalhost fixtureを含む開発経路であり、installed product、外部Runtime／Agent、実端末を証明しない。
- `release_blocker`: Windows installed productの総合証拠（provenance、first-run、Setup Doctor、Audit anchor外部改変）、8時間実測、外部Runtime／Agent／MCP／A2A、実端末、正式署名・配布、C32〜C34、owner GO、正式release。Broker installed smokeの個別sub-gateはC33でPASSしたが、総合evidenceの成立条件は満たしていない。
- `known_limitation`: MobileのMCP live一覧非提供、fixtureだけの比較／Device Link、外部接続未成立、実測不能値の`unknown`表示。

検証入口: `python tooling/full_regression_validation.py` および `python tooling/validate_all.py --desktop-platform windows --include-mobile-release`。いずれも開発検証であり、正式releaseの許可ではない。

## D4 Pocket C24: Mobile対応（2026-09-24）

Desktopの既存Broker契約から、Mobileへ必要な状態確認と復旧導線を選択的に投影した。MobileのNavigationは既存9画面を保持したまま、資源概要、履歴、MCP状態を追加した。端末TLS経路は、Runtime lifecycle状態、資源観測、通知summary、停止receipt、現在owner承認に結合した履歴metadataだけを既存Rust Broker handlerへ渡す。

- Production path: Mobile Flutter → Device Link TLS → Desktop Rust Broker → 既存読み取り専用handler → bounded projection。Mobile専用の権限判定、owner操作、別bridge、別監査storeは追加していない。
- Authority boundary: MobileはApprovalを発行・編集・延長・失効せず、MCP接続・Tool実行・Credential参照・実停止を行わない。停止画面はowner再承認待ちreceiptだけを表示し、資源のunknownを0へ変換しない。
- Content boundary: 通知はsummary、履歴は現在承認のmetadataだけで、対話本文・Approval payload・Audit raw reason・Credential実値を投影しない。Host表示は既存保存資格のmetadataに閉じる。
- Validation target: Mobile flutter analyze、flutter test、Rust端末統治試験、C24 Conformance、Schema、厳格日本語監査を実行する。
- 未成立分類: Mobile実機・Android/iOS安全保管・TLS実接続、Windows installed productでのMobile連携、長時間運用・障害注入、owner GO、C0-C34全数完成、正式releaseはrelease_blocker。MCP live一覧をMobileへ出さないことはowner専用管理面を守るknown_limitation。

## D4 Pocket C23: Desktop UX統合（2026-09-24）

既存20画面を削除せず、Desktop Navigationを`運用`、`安全`、`開発`、`設定`、`すべて`の論理グループへ整理した。グループ選択はFlutterの表示状態だけを変更し、別グループの画面へ移動した場合は対象グループへ切り替える。画面本体と既存のBroker、owner control、Approval、Audit、Recovery経路は置き換えていない。

- Production path: 操作者の操作グループ選択 → Flutter `NavigationRail`の表示対象絞り込み → 既存画面の選択。全体表示では既存20画面を現在の順序で表示する。
- Authority boundary: グループ名と選択状態はUI状態であり、Authority、Permission、Approval、Credential、Broker IPC、filesystem、process、networkを生成・実行しない。
- Validation: Desktop Flutter全93試験、`flutter analyze`、`flutter build windows --debug`、Conformance 178 checks、厳格日本語監査、Schema検査を実行しPASSした。Rustは変更していないため今回のRust全試験は未実行。
- 未成立分類: Windows installed productでの4グループ実画面操作evidence、C0-C34全数完成、正式release、owner GOは`release_blocker`。Mobile Navigationへの投影はC24の対象で`known_limitation`。

## D4 Pocket C22: 全体検索（2026-09-24）

既存の操作面を横断する読み取り専用の全体検索を追加した。`Ctrl+Shift+F`と画面上の全体検索ボタンから開き、現在の`ShellSnapshot`に含まれるbounded metadataと、MCP／A2A／評価／通知などのsurface entryを検索する。検索結果の選択は対象画面への移動だけであり、検索から権限作用へ到達しない。

- Production path: Desktopの全体検索 → `GlobalSearchIndex` → Snapshotの表示用metadata → 対象画面へのGUI navigation。indexは512件、検索語は128文字、結果は30件にboundedする。
- Authority boundary: Agent／Runtime／Session／Approval／Audit／MCP／A2A等のmetadata、History、Profile、TelemetryはAuthority、Permission、Approval、Credentialを生成・再利用しない。FlutterのindexはBroker IPC、filesystem、process、network、credential、Clipboardへ直接到達しない。
- Content boundary: 対話本文、Approval payload、Audit raw payload、Credential実値、秘密値、未許可のfull content、任意pathをindexへ登録しない。Broker由来と確認できないsnapshotは証拠範囲を`不明`と表示する。
- 未成立分類: 実Runtime・Agent・MCP・A2A・Hostの全surfaceから取得したinstalled productでの横断検索実機evidence、clean installed artifactとの結合、owner GOは`release_blocker`。外部surfaceのlive再取得、全文検索、本文検索、検索結果からの操作実行を提供しないことは`known_limitation`。

## D4 Pocket C20: Windows常駐トレイ（2026-09-24）

Windows native trayを、表示と操作入口に限定したWin32実装として追加した。トレイからD4 Pocketの前面化、実行系状態、保留承認件数、重大通知件数、全Runtime停止要求、終了を操作できる。表示射影はBroker由来値だけを採用し、取得不能値を0へ変換しない。

- Production path: Windows Win32 tray → Flutterの固定`gui_shell/tray`表示チャネル → Snapshot／通常Broker IPC。停止要求は`全Runtime停止要求`としてRust Brokerへ渡し、lifecycle registryの対象をboundedに列挙したreceiptを返す。
- Authority boundary: Win32 trayとFlutterのトレイチャネルはAuthority、Permission、Approval、Credential、Runtime操作を所有しない。停止receiptは`停止実行済み=false`、`承認状態=owner_reapproval_required`、`権限生成=なし`に固定し、直接killとowner承認生成を行わない。
- Validation: C20 Schema 3件、正常例3件、負例3件、Conformance 175 checks、厳格日本語監査、release runtime assertion、Rust全試験（lib 218、owner CLI 7、IPC 9、canonical hash 1、checkpoint 8、protected data 2、protected startup 1、protected store 3、workspace diff 2、workspace reader 2、workspace startup 7）、Desktop Flutter解析・86試験、Windows debug buildを実行しPASSした。
- 未成立分類: Windows installed productでのトレイアイコン・前面化・常駐・終了・Broker停止要求の実機evidence、owner再承認後の全Runtime実停止lifecycle統合、正式releaseは`release_blocker`。Windows以外のnative tray未提供は`known_limitation`。

## D4 Pocket C19: Adapter管理操作（2026-09-24）

Runtime CenterのAdapter catalogをRust Brokerのbounded管理経路へ接続した。`アダプター一覧`は通常IPCのmetadata-only projection、導入・検証・有効化・無効化・隔離・更新・削除はowner controlだけが状態を変更する。通常IPCの変更要求は`owner_reapproval_required`のsuspendedとして監査し、状態を変更しない。

- Production path: Desktop Runtime Center → `アダプター一覧`／Adapter管理要求 → Rust Adapter Center → `adapters.json` → metadata-only receipt。導入・更新はManifestをcatalogへ登録するだけで、外部download、filesystem、processは実行しない。
- Authority boundary: 署名はBroker所有Ed25519 trustとManifest正本byteで検証し、未検証Adapterは有効化できない。Adapter metadata、署名、hash、過去状態からPermission、Approval、Authority、Credentialを生成しない。
- Quarantine boundary: 隔離済みRuntime IDを実行系登録、lifecycle、資源観測、対話で再利用しない。削除はcatalog recordだけを除去し、外部artifactの実削除を主張しない。
- Validation target: Adapter管理Schema、正常／負例fixture、Rust state transition／signature path／restart state、Conformance、Desktop Runtime Centerを接続する。
- 未成立分類: 外部artifactの実download・filesystem導入・process起動・実削除、Windows installed productのAdapter管理実証は`release_blocker`。C19だけでAdapter製品機能全体または正式releaseを主張しない。

## D4 Pocket C18: Host操作面（2026-09-24）

C17のHost registryをDesktopのHost操作面へ接続した。`ShellCoreClient.product()`は通常認証済みBroker IPCの`Host一覧`を取得し、Host metadataをSnapshotへ投影する。DesktopはHost一覧、接続状態、Trust、Runtime／Agent summary、Host切替を表示する。

- Production path: `Host一覧` → Rust Brokerのbounded metadata-only receipt → Desktop Snapshot → Host操作面。`Host切替`は通常IPCでregistryのHost IDを再照合し、Audit確定後に表示コンテキストのreceiptを返す。
- Authority boundary: Host切替は`権限生成=なし`、`authority_strip=true`、`承認状態=not_reused`に固定する。Host AのPermission、Approval、AuthorityをHost Bへ再利用しない。Host registryのRuntime／Agent件数は`INTERNAL_STATE` summaryであり、個別live一覧の証拠ではない。
- Observation boundary: 選択Hostが現在のBroker観測Hostと一致するときだけ現行SnapshotのRuntime／Agentを表示する。それ以外はsummaryと`未観測`だけを表示し、remoteの個別状態を推測しない。
- Validation target: Host切替の未知Host、未知field、owner channel、Host間非混線をRust unit test、Schema、Conformance、Desktop widget surfaceへ接続する。
- 未成立分類: live Host再接続、Trust検証、Host別remote Runtime／Agent discovery、Host間Workspace隔離、Device Link実認証、Desktop installed evidenceは`release_blocker`。C18だけでHostの実接続や正式releaseを主張しない。

## D4 Pocket C17: 複数Host registry（2026-09-24）

C17のHost metadata registryをRust Security Brokerへ接続した。owner controlだけがHostを登録し、通常認証済みIPCだけがHost一覧を参照する。Host ID、表示名、Platform、接続状態、Trust、証明書／identity hash、Runtime summary、最終接続をboundedなmetadata-only receiptへ射影し、`hosts.json`へatomicに永続化する。登録時のTrustと接続状態は`pending_review`に固定し、Host metadataからPermission、Approval、Authority、Credentialを生成しない。

- Production path: owner control → `Host登録` → Rust Host registry → hash-only identity／bounded summary → `hosts.json`。通常認証済みIPCの`Host一覧`はBroker内部registryを`INTERNAL_STATE` metadata-onlyとして返す。
- Boundary: 通常IPCからの登録、owner controlからの一覧、未知field、重複Host ID、identity実値、`permission_id`／`approval_id`／`authority`、connected／verifiedの自己申告、state fileのmalformed／重複field／上限超過を拒否する。Host Aのmetadata・承認・PermissionをHost Bへ共有しない。
- Validation: Schema 99件／正常例99件／負例118件、Conformance 172 checks、Rust全試験251件（lib 209、owner CLI 7、IPC 9、canonical hash 1、checkpoint 8、protected data 2、protected startup 1、protected store 3、workspace diff 2、workspace reader 2、workspace startup 7）、厳格日本語監査、manifest、packaging portability、Windows開発集約検証をPASSした。集約検証はdevelopment modeであり、既存のWindows installed証拠5件を`release_blocker`として保持する。
- 未成立分類: Host切替、HostごとのRuntime／Agent一覧、live接続再確認、Device Linkの実認証・失効・quarantine、Host間Workspace隔離、複数Agent比較／Handoff、Desktop Host操作面、installed product証拠、長時間運用・障害注入は`release_blocker`。C17だけで複数Host製品機能全体または正式releaseを主張しない。

## D4 Pocket C16: A2A接続センター初期経路（2026-09-23）

C15のA2A外部概念契約を、owner controlから実Agent Cardを取得するRust Security Broker経路へ接続した。現行単位はloopback HTTPだけを許可し、Agent Cardのbounded検証と`LIVE_RUNTIME` metadata-only receipt、通常IPCの接続一覧を成立させる。外部Agentの宣言からTrust、Permission、Approval、Authorityを生成しない。

- Production path: owner CLI／control → `A2A接続` → Rust A2A接続センター → loopback HTTP Agent Card → Agent Card検証 → receipt。通常認証済みIPCの`A2A接続一覧`はBroker内部接続状態を`INTERNAL_STATE`として返す。
- Boundary: HTTPS、非loopback HTTP、redirect、Transfer-Encoding、Content-Encoding、重複Content-Length、巨大header／body、credential実値、Agent Card endpoint実値、Task／Message／Artifact raw contentは拒否または非投影。Trustは`pending_review`、Capability diffは`not_evaluated`、`authority_strip=true`、`権限生成=なし`、`公開範囲=metadata_only`に固定する。
- Bounded behavior: Agent Card body 1MiB、header 16KiB、URI 2048 bytes、接続一覧64件、接続・読取期限bounded。同一AgentIDの再接続、ownerからの一覧、通常IPCからの接続要求を拒否する。
- Validation: Schema 96件／正常例96件／負例115件、Conformance 171 checks、Rust全試験247件（lib 205、owner CLI 7、IPC 9、canonical hash 1、checkpoint 8、protected data 2、protected startup 1、protected store 3、workspace diff 2、workspace reader 2、workspace startup 7）、厳格日本語監査をこの単位で実行しPASSした。Desktop専用接続画面、HTTPS実接続、実A2A Test Harnessは未検証または未接続として別分類する。
- 未成立分類: HTTPS TLS、公開endpoint discovery、認証実行、Task／Message送信、Artifact本文、Stream購読、cancel、再接続、quarantine、複数Agent比較、Handoff、Desktop接続画面、Windows installed product証拠、長時間運用・障害注入は`release_blocker`。C16初期経路だけでA2A製品機能全体または正式releaseを主張しない。

## D4 Pocket C16補完: A2A接続metadataの再起動復元（2026-09-24）

C16のsession memoryだったA2A接続registryを、Broker永続storeの`a2a_connections.json`へbounded・atomicに保存し、再起動時にstrict再検証して復元する経路を追加した。URI実値、credential実値、raw content、未知field、重複AgentID、上限超過は保存または起動時復元しない。

- Production path: `A2A接続` → Agent Cardのmetadata-only receipt → Rust Brokerの永続state → Broker再起動 → strict state検証 → 通常IPCの`A2A接続一覧`。
- Safety boundary: 復元receiptは`接続状態=restored_pending_review`、`証拠種別=INTERNAL_STATE`、`承認状態=owner_reapproval_required`へ降格する。過去のowner承認を再利用せず、復元stateからPermission、Approval、Authority、credential実値を生成しない。
- Validation: 再起動を模したRust Broker再open試験で1件のreceipt復元、状態降格、state file内のendpoint実値非保持、malformed stateのfail-closedを確認した。最終Rust試験は248件（lib 206、owner CLI 7、IPC 9、canonical hash 1、checkpoint 8、protected data 2、protected startup 1、protected store 3、workspace diff 2、workspace reader 2、workspace startup 7）で全件PASSした。
- 未成立分類: TLS再接続、Agent Card再検証、disconnect、quarantine、Task／Message送信、Artifact本文、Stream購読、複数Agent比較、Desktop接続画面、実A2A Test Harnessは`release_blocker`。復元は表示registryの再構成であり、外部Agentとのlive connection復旧ではない。

## D4 Pocket C14: 追跡情報閲覧（2026-09-23）

C13の`観測一覧`を再利用する読み取り専用Trace InspectorをDesktopへ接続した。Broker内部で実測されたSpanを、開始・終了・所要時間・状態・親Span・関連Audit・エラー分類のbounded waterfallとして表示する。

- Production path: Desktop Trace Inspector → 通常認証済みBroker IPC → Rust Observation Center → `観測一覧`。TraceID filter、手動更新、Broker接続なしのfail-closed表示を接続した。
- Evidence boundary: 現在の実測対象はBrokerだけである。Runtime、Adapter、Tool、外部通信はproduction観測経路が接続されるまで実測済みと表示しない。表示は`INTERNAL_STATE`であり、Auditのreason、payload、metadata、秘密値を表示しない。
- Authority boundary: Trace表示からPermission、Approval、Authority、Capability、Credentialを生成しない。Traceは監視表示であり、承認・実行・権限判断の経路ではない。
- 検証: Desktop NavigationRail／追跡情報画面を含む全83 Flutter試験、Flutter analyze、Schema 92件／正常例92件／負例109件、Conformance 169 checks、厳格日本語監査、manifest検査を実行し、PASSした。`python tooling/validate_all.py --python-only --desktop-platform windows`も開発モードでPASSした。
- 未成立分類: Runtime／Adapter／Tool／外部通信のproduction trace、OpenTelemetry export、外部collector、long-term trace、8時間運用、Windows installed product証拠は`release_blocker`。C14はBroker内部観測の閲覧範囲だけを主張する。

## D4 Pocket C15: A2A外部概念射影契約（2026-09-23）

A2AのAgent Card、Task、Message、Artifact、Streamを、実接続なしのboundedな`metadata_only`契約へ射影した。C15はSchema、valid／negative fixture、Conformance、正本文書だけを追加し、外部Agentへの接続やTask実行経路は追加していない。

- Contract path: A2A外部概念 → `a2a_contract.schema.json` → Schema／fixture／Conformance。`protocol_version`、Agent Cardのinterface／capability／skill／authentication metadata、Task状態、Message／Artifactのpart kind・件数・hash、Stream状態を固定した。
- Authority boundary: Agent CardはTrustではなく、TaskはApprovalではなく、MessageはPermissionではなく、ArtifactはAuthorityではなく、StreamはCapability grantではない。全概念へ`authority_strip=true`を要求し、`権限生成=なし`、`公開範囲=metadata_only`、認証実値・endpoint実値・raw content非保持を検証する。
- Evidence boundary: valid fixtureは`FIXTURE`であり、実Agent、実endpoint、実認証、実Task、実Streamの証拠ではない。Trustはoperator review待ち、Capability diffの追加・変更・削除はoperator review必須とする。
- Validation: Schema 93件／正常例93件／負例112件、Conformance 170 checks、厳格日本語監査、manifest検査を実行する。Rust／Flutterは変更していないため、この単位では未実行とする。
- 未成立分類: Agent Card discovery、A2A binding、外部Agent登録、credential注入、Task／Message送信、Artifact本文、Stream購読、timeout、取消、再接続、quarantine、複数Agent比較、Handoffは`release_blocker`。C15契約だけでA2A接続や製品完成を主張しない。

## D4 Pocket C13: 観測センター（2026-09-23）

BrokerのAudit確定処理を、Auditとは別の内部観測としてboundedなSpan、Trace、Metricへ射影し、Desktop観測センターへ接続した。観測はsession内memoryに限定し、caller登録、外部export、権限生成を持たない。

- Production path: Desktop観測センター → 通常認証済みBroker IPC → Rust Observation Center → Brokerが確定したAudit eventの処理時間。`観測一覧`の上限とTraceID filterを接続した。
- Authority boundary: 応答は`INTERNAL_STATE`に固定し、Auditのreason、payload hash、metadata、credential、raw contentを返さない。観測からPermission、Approval、Authority、Capabilityを生成しない。
- Bounded behavior: Broker保持1024Span、IPC返却256Span／256Trace／16Metric。測定不能なdurationは0ではなく`unknown`として扱い、OpenTelemetry exportは`unsupported`である。
- Validation: 観測Schema 4件、正常／負例fixture、Rust内部観測・bounded・Metric・Broker経路試験、Desktop client／NavigationRail／全Flutter試験を接続した。
- 未成立分類: C14 Trace Inspectorのwaterfall、親子Span、OpenTelemetry export、外部collector、Runtime全体の実測、Windows installed productでの観測証拠、8時間運用は`release_blocker`。本単位はBroker Audit確定処理の内部観測だけを主張する。

## D4 Pocket C12: 通知センター（2026-09-23）

監査eventから通知summaryを限定射影するRust Notification Centerを追加し、既読・破棄状態をBroker所有の`notifications.json`へhash結合して保存する。通知をcallerが登録する操作はなく、通知操作自身も通知sourceへ射影しない。

- Production path: Desktop通知画面 → 通常認証済みBroker IPC → Rust Notification Center → 監査event／表示状態。`通知一覧`、`通知既読`、`通知破棄`、`通知全既読`を接続した。
- Authority boundary: 通知は`INTERNAL_STATE`のsummaryであり、Permission、Approval、Authority、Capability、Credentialを生成しない。理由、payload、metadata、秘密値を返さず、開く操作は画面navigationだけである。
- Bounded behavior: 監査走査は4096件、通知返却は256件、表示状態はhash付き4096件まで。malformed state、未知field、重複ID、stale hashはfail-closedとする。
- Validation: 通知Schema 3件、正常／負例fixture、Rust 4単体試験、Desktop通知画面／NavigationRail、Schema／Conformanceを接続する。
- 未成立分類: Windows native toast投影、Windows installed productでの通知実証、8時間運用、全通知sourceの実Runtime証拠は`release_blocker`。本単位はin-app通知センターの完成範囲だけを主張する。

## D4 Pocket C11: 更新センター（2026-09-23）

更新候補のSchema、Rust Broker署名検査、永続一覧、延期、download／適用／rollback要求、Desktop設定画面を接続した。候補metadataから署名対象byteを再構成し、Broker所有のEd25519公開鍵とfingerprintを使う。候補側の公開鍵、Profile、MCP metadata、履歴、UI stateは信頼源にならない。信頼設定未構成または署名不正の候補は保存しない。

- Production path: Desktop設定画面 → 通常Broker IPC → Rust Update Center → `updates.json`／Audit。`更新一覧`、`更新署名検査`、`更新確認`、`更新延期`、`更新download要求`、`更新適用要求`、`更新rollback要求`を接続した。
- Authority boundary: 更新署名の信頼はBroker所有設定だけから成立する。署名済み候補もPermission、Approval、Authority、Credential、外部process実行権限を生成しない。candidate hashのstale照合を行う。
- Execution boundary: download、install、process、rollbackの外部実行はsuspendedである。要求receiptを実行完了と報告しない。
- Validation: UpdateCandidate／Receipt／ListのSchema、negative fixture、Ed25519検証、信頼設定未構成、候補hash、永続化、実行要求suspended、Desktop UpdateClientを検証する。
- 未成立分類: 外部download、install、process、rollback適用、owner公開鍵の本番provisioning、Windows installed productの更新実証は`release_blocker`。C11のBroker要求受付と署名検査だけで更新製品機能または正式releaseを主張しない。

## D4 Pocket C10: 運用プロファイル（2026-09-23）

運用プロファイルをSchema-firstで定義し、Rust Brokerの永続状態と通常認証済みIPCへ接続した。ProfileはRuntime、Adapter、要求Capability、Content Exposure、network exposure、resource limit、UI preferenceだけを保持する。`プロファイル適用要求`はProfile hashを照合して適用意図をAuditへ記録するだけで、Permission、Approval、Authority registry、Runtime操作を変更しない。

- Production path: Desktop設定画面 → 通常Broker IPC → Rust Profile Center → `profiles.json`のatomic write／起動時再検証。作成、複製、適用要求、削除、export、import、一覧を実装した。
- Authority boundary: Profileは要求configurationであり、権限源ではない。Schemaの追加field拒否とBrokerの`deny_unknown_fields`でPermission、Approval、Authority、Credential実値、Audit identityの混入を拒否する。
- Validation: Profileの正常／負例、作成・適用要求・再起動後再読込・禁止field拒否をRust試験へ追加した。Schema／Conformance／日本語基底監査／Desktop Flutter解析を実行する。
- 未成立分類: 専用ファイル選択UI、ProfileからRuntimeへ実設定を反映する操作、ProfileのPermission・Credential・MCP・A2A接続は`release_blocker`。現在のC10は要求設定の保存・監査・表示までであり、Profile適用による権限作用を主張しない。

## D4 Pocket Phase 4: Codex AdapterのBroker登録とread-only Launcher基盤（2026-09-23）

Windowsで確認した実物Codex CLIを、Rust Brokerの明示起動設定から既存の実行系対話経路へ接続した。`--codex-runtime <ID=絶対executable path=絶対workspace path>` はowner起動時だけ受け付け、Adapterの登録時にexecutable／workspaceの絶対path、通常file／directory、secret pathでないこと、`codex --version`、`codex exec --help`を確認する。IPCから任意の実行path、argv、environment、workspaceを受け取る経路は追加していない。

- Production path: `実行系挙`で登録されたCodexを既存の `対話開始`、owner承認付きの`対話送信`、`対話取得`、`対話中止`、`対話終了`から利用する。汎用`command_envelope` dispatchは引き続き停止中で、Flutterはprocess／filesystem／credential／networkを直接実行しない。
- Runtime boundary: 実行は固定された `codex exec --json --ephemeral --ignore-user-config --sandbox read-only --color never --cd <workspace> -` に限定し、環境変数はallowlistだけを渡す。JSONLのthread、agent message、turn完了を検証し、失敗event、不正応答、出力上限超過、取消、期限超過を成功へ昇格しない。Broker起動と認証付き通常IPCによる`codex`実行系列挙をWindowsで実行確認した。
- Evidence boundary: Codex CLIのread-only JSONL応答`READY`は、独立したephemeral／read-only smokeで実物interfaceを確認したもの。Broker登録と実行系列挙はLIVE_RUNTIMEの実Broker証拠である。静的conformanceはsource boundaryを検査するが、write-capable Agent、MCP、複数Agent比較、Handoff、長時間運用、installed productを証明しない。
- 未成立分類: write-capable Agent execution、実taskのProduct UI完結、MCP／A2A、複数Agent比較／Handoff、Usage／Cost、Claude／Gemini接続は`release_blocker`。Codex Adapterのread-only限定、`--codex-runtime` executable path内の`=`未対応、未導入Vendorは`known_limitation`として保持する。

## D4 Pocket C7: 資格情報保管庫のowner登録と公開metadata（2026-09-23）

C7の最初の完結単位として、owner controlから新規資格情報をWindows ProtectedStoreの`Purpose::Credential`へDPAPI保管し、通常IPCへ検証済みmetadata一覧を返すContractとBroker経路を接続した。同じ資格情報IDの再登録、normal IPCからの追加、owner channelからの一覧、保管先未登録、暗号文欠落・改変はfail-closedで拒否する。

- Contract: `credential_registration.schema.json`、`credential_receipt.schema.json`、`credential_list.schema.json`、正常例、権限field混入・秘密値混入・件数負値の負例、`docs/specs/credential-vault.md`を追加した。秘密値は登録payloadからDPAPIへ渡すだけで、receipt、一覧、Audit reason、CLI出力へ投影しない。
- Production path: `資格情報登録`はowner資格経路だけ、`資格情報一覧`は通常資格経路だけを受け付ける。Flutter、Adapter metadata、Profile、History、MCP metadata、A2A Agent Cardは資格情報の権限源にも登録経路にもならない。資格情報のRuntime／Tool／MCP／A2A注入はまだ接続していない。
- Validation: `python tooling/schema_check/check_schemas.py` はSchema 74件／正常例74件／負例89件、`python tooling/conformance_tests/run_conformance_skeleton.py` はConformance 163 checks、`python tooling/日本語基底監査.py --strict`、`python tooling/validate_all.py --python-only --desktop-platform windows`、manifest、release gate、packaging portability、release smoke、evidence bundle、release runtime assertionsがPASSした。C7専用Rust試験は3件、Rust全試験はlib 176件、main 5件、統合35件、合計216件がPASSした。今回のC7専用試験ではowner/normal channel、秘密値非投影、暗号文欠落時の部分一覧拒否を確認した。なお、未stage状態ではpackaging対象が追跡file一覧に限定されるため、C7新規fileをstageした状態でpackaging portabilityを実行した。
- 未成立分類: 資格情報の取得・Runtime／Tool／MCP／A2A注入、更新、失効、削除、接続先変更、Recovery操作、GUI管理面、Windows実機owner登録証拠、非Windows安全保管は`release_blocker`。登録と一覧だけで資格情報保管庫全体または製品releaseを主張しない。

## D4 Pocket C8: MCP外部概念射影契約（2026-09-23）

MCP実接続に先行して、外部metadataをGUI-Shellの境界付き契約へ射影した。`Server`、`Tool`、`Resource`、`Prompt`、`Transport`、`Credential ref`、`Trust`、`Capability diff`を`metadata_only`として表し、MCP metadataからAuthority、Permission、Approvalを生成しない。C8では外部Serverの発見・接続・Tool実行を開始していない。

- Contract: `mcp_contract.schema.json`、正常fixture、ToolへのPermission混入、Credential refへの秘密値混入、Authority／Approval混入の負例、`docs/specs/mcp-contract.md`を追加した。Transportの接続先はhashのみ、Credential refはIDと用途・対象・必要性・状態だけを持つ。
- Conformance: 外部概念の必須射影、`権限生成=なし`、`公開範囲=metadata_only`、Trust未確定、Capability diffのoperator review要求、秘密値／権限fieldの拒否を検査する。
- Validation: `python tooling/schema_check/check_schemas.py` はSchema 75件／正常例75件／負例92件、`python tooling/conformance_tests/run_conformance_skeleton.py` はConformance 164 checks、`python tooling/日本語基底監査.py --strict`、`python tooling/validate_all.py --python-only --desktop-platform windows`、manifest、packaging portability、release gateがPASSした。Flutter／Rustの実装変更はないためFlutter解析とRust試験は未実行とした。
- 未成立分類: MCP discovery、connect、authentication、consent、Tool／Resource／Promptの実取得、Broker経由Tool実行、timeout、server unavailable、disconnect、quarantine、実MCP Test Harnessは`release_blocker`。契約射影だけでMCP接続または製品releaseを主張しない。

## D4 Pocket C9: MCP stdio接続センター（2026-09-23）

C8の契約をRust Brokerのowner control接続経路へ結合した。`MCP接続`はownerが指定した絶対executable、workspace、引数、stdio transport、Credential refだけを受け付け、Rust process boundaryから現行`server/discover`を試行する。旧`initialize`／`notifications/initialized`はlegacy protocolとして明示fallbackする。discovery後はcapabilityに従ってTool／Resource／Prompt一覧を取得し、Schema検証済みmetadata-only receiptへ射影する。

- Production path: owner control → `MCP接続` → Rust Broker → MCP stdio child → discovery／list →永続Audit付きreceipt。`MCP接続一覧`は通常IPC専用であり、接続processが終了・timeout・不整合となった場合は一覧を返さない。
- Authority boundary: MCP metadata、Tool description、Trust、Capability diff、Credential refはAuthority、Permission、Approval、Credential実値を生成しない。Credential実値注入とTool実行は未接続であり、必須Credentialを要求するServerは`mcp_credential_unavailable`で拒否する。
- Validation boundary: malformed JSON-RPC、response id mismatch、unknown tool、authority／secret field、重複metadata、未処理pagination、timeout、Server終了をRust unit test／Conformanceへ接続した。Rust processの実MCP Test Harness、Tool実行、Streamable HTTP、OAuth、consent、disconnect、quarantineは未成立として扱う。
- 未成立分類: Tool／Resource／Prompt実取得を用いた実Broker運用、Tool execution、Credential injection、Streamable HTTP、OAuth、consent、disconnect、quarantine、実MCP Test Harness、Windows installed product証拠は`release_blocker`。C9 stdio catalog接続だけでMCP全体またはD4 Pocket正式releaseを主張しない。

## D4 Pocket C6: 対話結果からの回帰Case owner登録（2026-09-23）

C5の評価Datasetと混ぜず、完了済み通常対話をownerが明示的に回帰Caseへ登録する独立Contractを追加した。Rust Brokerは要求ID/hash、`表示範囲=full`、永続結果証跡、終了監査ID、結果状態を現在の対話制御で再照合する。元の対話入力・応答本文は自動コピーせず、ownerのredacted定義を`ProtectedStore::Purpose::Regression`へ暗号化し、CLIにはhash-only receiptだけを返す。

- Contract: `regression_case_registration.schema.json`、`regression_case_receipt.schema.json`、正常例、権限field混入とraw receipt混入の負例、`docs/specs/regression-case.md`を追加した。C5の`Purpose::Evaluation`とは保管purposeを分離した。
- Production path: `回帰Case登録`はowner資格経路からBrokerへ入り、現行対話証跡と明示定義を結合してAudit確定する。通常IPC、履歴、Profile、MCP metadata、Agent metadataは登録資格または権限を生成しない。
- Validation: `python tooling/schema_check/check_schemas.py` はSchema 71件／正常例71件／負例86件、`python tooling/conformance_tests/run_conformance_skeleton.py` はConformance 162 checks、`python tooling/日本語基底監査.py --strict`、Rust全試験（lib 173件、main 4件、統合35件、合計212件）、`python tooling/validate_all.py --python-only --desktop-platform windows`、manifest、release gate、packaging portabilityをPASSした。Windows ProtectedStoreのRegression purpose分離試験もPASSした。Flutter解析・UI試験はFlutter変更がないため未実行であり、Windows installed productの実機証拠は別のrelease blockerとして保持する。
- 未成立分類: GUIのprivate登録画面、Case一覧、削除Recovery、C5 Dataset revisionへの明示import、Windows実機でのowner登録証拠は`release_blocker`。known marker拒否は秘密不存在の証明ではなく、owner redaction責任を置き換えないため`known_limitation`として保持する。

## D4 Pocket Phase 4前提: Codex CLI実物interface probe（2026-09-23）

Phase 4 Agent Launcherの前提確認として、Windowsで実際にPATHへ存在するCodex CLIのinterfaceをdevelopment-only probeから読み取り専用で確認した。`codex --version`は`codex-cli 0.155.0-alpha.16`、`codex exec --help`は`exec`、`resume`、`fork`、`review`等のsurfaceを返した。probeはversion/help以外を呼ばず、prompt、credential、workspace変更、task実行、process dispatchを行わない。

- Adapter evidence: version/helpの観測は`LIVE_RUNTIME`として記録し、Codex Adapterは`degraded`とする。Brokerのcommand dispatchが停止中のため、task executionは`unknown`、process spawnは`unsupported`である。help表示だけではsession操作の実動作を証明しないため、session control/session supportも`unknown`とする。
- Boundary: Claude、Gemini、その他のCLI/APIはこのWindows環境で未導入のため、存在・対応を推測しない。probeは秘密値を環境から子processへ渡さず、Adapter recordにも実値を保持しない。
- 未成立: Agent Launcherのproduction path、実task実行、Cancellation、Tool／MCP、Usage／Cost、Provider／Model接続、複数Agent比較、Handoffは未完成である。BrokerのApproval／Audit／Recovery統治を迂回する実行経路は追加していない。

この節はCodex CLIのinterface観測を証明するが、D4 PocketのAgent実行または完成製品releaseを証明しない。既存のrelease_blocker分類を保持する。

## D4 Pocket Phase 4 bounded projection: Agent比較・Handoff境界（2026-09-24）

実Agentの起動・書込・外部接続を推測せず、複数Agent比較とHandoffで公開できる情報のContract、conformance、Desktop投影を追加した。既存のCodex Adapter read-only経路、Rust Broker経路、rev1進捗を変更していない。

- Contract: `specs/agent_comparison.schema.json`は異なるWorkspaceの2〜8セッション、公開結果概要、bounded metric、`unknown`値を定義する。`specs/agent_handoff.schema.json`はTask／差分／試験／公開実行概要だけを渡し、Authority、Permission、Approval、Credential、hidden contextを固定拒否する。
- Conformance: 同一Workspace、同一セッション重複、Authority／Approval再利用、取得不能値の0補完、同一Agent Handoff、公開概要の秘密値をfail-closedで検査する。証拠種別は`INTERNAL_STATE`であり、実Agent比較・実Handoffの証拠ではない。
- Desktop path: Agent Centerへ、snapshotから比較可否と公開Handoff概要を読み取る表示を追加した。FlutterからAgent起動、process、filesystem、network、credential、権限付与へ到達する経路は追加していない。
- 未成立分類: 実Agentの独立Workspace起動、実結果の比較、target AgentへのHandoff・再評価・取消・失敗隔離・Recoveryは`release_blocker`。既存のwrite-capable Agent、MCP／A2A、Provider／Model、Claude／Gemini未接続の分類は変更しない。

## D4 Pocket Phase 3: Agent Adapter契約のSchema接続（2026-09-23）

Agent Adapter契約を追加した。`specs/agent_adapter.schema.json`はAgent identity、Provider、Version、Model、capability宣言、Workspace要件、Tool／MCP／Session／Cancellation／Usage／Cost対応、認証方式、Host要件を定義する。対応状態は`supported`、`unsupported`、`unknown`と理由を必須にし、認証は参照方式だけを許可してsecret実値を持たせない。

- Contract: valid exampleと`permission_id`混入のnegative fixtureを追加した。`additionalProperties=false`と認証の`secret_value_present=false`で、Agent Adapter宣言をPermission／Approval／trustへ昇格させない。
- Conformance: Agent Adapterが宣言専用であり、unsupported／unknownに理由があり、空理由が拒否されることを検査する。Schema 69件／valid example 69件／negative fixture 84件、Conformance 159 checksへ更新した。
- Production boundary: 今回は契約と開発時conformanceまでで、Vendor CLI／APIのLauncherは接続していない。実物interfaceを確認するまでunsupportedとし、認証値・process起動・network接続を推測で追加しない。

この節でAgent実行、複数Agent比較、Handoff、MCP接続、Provider／Model接続の完成を主張しない。これらはPhase 4以降の実物interface確認とBroker統治経路の検証対象であり、未成立範囲は既存のrelease_blocker分類を保持する。

## D4 Pocket Phase 2: Host Capabilityの契約接続（2026-09-23）

D4 Pocketのブランド表面を追加し、rev2 Phase 2のHost Capabilityを、Schema → Rust Broker → 認証付きIPC → Flutter読み取り専用操作面まで接続した。既存のGUI-Shell技術契約、Shell Coreの権限境界、rev1の進捗履歴は変更していない。

- Contract: `specs/host_capability.schema.json`、valid example、`Permission`混入のnegative fixtureを追加した。能力の状態と証拠種別（`CONFIG`、`INTERNAL_STATE`、`LIVE_RUNTIME`、`EXTERNAL_EVIDENCE`、`FIXTURE`）を分離し、Host Capability自身にPermission／Approval fieldを許可しない。
- Rust production path: Broker operation `ホスト能力`を既存の認証付きIPCへ追加した。payloadはnullだけを受け付け、非null payloadを拒否する。Brokerが観測できる範囲だけを返し、観測結果から権限を生成しない。
- Desktop product path: `ShellCoreClient.product()`がhealth後にHost Capabilityを取得し、`D4 Pocket ホスト能力`の読み取り専用画面へ投影する。UIはRust Brokerを呼び出すが、Permission、Approval、filesystem、process、network、credentialを直接扱わない。
- Evidence: Schema 68件／valid example 68件／negative fixture 83件、Conformance 158 checks、Host Capability unit test、Rust全test 204件（Broker IPC 9件を含む）、Desktop analyze／全76 tests、共有UI analyze／39 tests、Mobile analyze／29 testsがこの作業単位でPASSした。`Permission`混入と不正payloadの拒否も実行した。Python-only集約検証、manifest、release gate、packaging portability、release smoke、evidence bundle、release runtime assertions、日本語基底監査もPASSした。

この節でrev2全体の完成を主張しない。Agent Adapter／Launcher／Dashboard／Compare／Handoff、Provider／Model、MCP／A2A、Multi-host、Compose／Preview／Export／Pruning、長時間運用、Windows installed productの全数証拠は後続作業または既存release gateに残る。既存のrelease_blockerをpost_v1_scopeへ読み替えない。

## D4 Pocket 第4段階 エージェント接続表示盤（2026-09-24）

起動時に実物Codex CLIのversion／help interfaceを確認済みのAdapterだけを、Rust Brokerの認証済み`Agent一覧`からmetadata-onlyでDesktopへ投影する経路を追加した。既存の`実行系列挙`とAgent Adapterを混同せず、通常Runtime AdapterはAgent権限へ昇格させない。

- Production path: owner起動設定 → Rust `CodexCliAdapter::new` → version／help probe → `Agent一覧` → 認証済みDesktop IPC → Agent CenterのProvider／Model／状態／証拠種別／能力表示。
- Evidence boundary: interface確認は`LIVE_RUNTIME`だが、Codex Adapter状態は`degraded`で固定し、task execution、session control、Tool、MCP、Cancellation、Usage、Cost、process spawnはunknownまたはunsupportedとする。Credential実値、executable path、Workspace pathはAgent一覧receiptへ返さない。
- Validation: Rust全試験でAgent metadataのread-only、秘密値非保持、process spawn停止を確認し、Desktop Agent Centerは認証Broker応答が返すAgent Adapter一覧だけを描画する。
- 未成立分類: write-capable Agent execution、Agentの選択・実task・取消、Claude／Gemini接続、複数Agentの実比較、実Handoff、MCP／A2Aは`release_blocker`。Agent Adapter Dashboardのmetadata表示だけでAgent製品機能の完成を主張しない。

## 現況：Windows実機を基準とした開発継続（2026-09-13）

owner指示により、現在実機検証できるOSはWindowsだけとする。Android実機は凍結、その他の非Windows実機も未検証として保持する。実装・build・自動試験・利用可能な仮想環境の検証を先に進め、実機の不在だけを開発停止条件にしない。実運用監査署名は正式release直前まで延期する。具体的な運用はROADMAPの「現在の開発・検証条件」を参照する。

次の保存済み結果を今回読み直した。対象sourceは `7d4766d0969a335e57bcba997540851c2e25aa6c` であり、この文書更新後の新commitの実行証拠へ読み替えない。

- Windows: `%LOCALAPPDATA%/GUI-Shell/installed-runs/rev2-7d4766d-20260911/runtime/evidence/validation-result.json` では、provenance分離、installed初回起動、Setup Doctor、brokerの4関門がpassed。実運用署名の関門はfailed。末尾に残る「正式Windows再収集が必要」という履歴項目は、このsourceの通常collectorによる収集で解消した。
- Apple: Repository外の `GUI-Shell-apple-7d4766d/verified-result.json` では手動run `34560858168`、artifact `10184321688` がpassed。Mac上Rust試験64単体・5 IPC・7 checkpoint、macOS開発appとiOS Simulator appのbuildを確認した記録である。tarのSHA-256は `370851b078b74b266deadce4c4a08ffeba4ba0ef2381a1d84f06768111cb2da9`。実機起動・Keychain・正式配布を証明しない。

これらは保存済み検証結果の確認であり、今回の文書修正で製品を再実行した結果ではない。履歴を消して過去の判断を隠さず、対象sourceと証拠範囲を固定して更新する。

- item: 非Windows実機証拠・Mobile正式配布・実運用監査署名・owner GO
  classification: release_blocker
  reason: 実機はWindowsのみ利用可能で、Android実機は凍結中。運用署名は正式release直前まで延期、正式配布条件とowner GOも未成立。利用可能な環境での開発継続を妨げる条件ではない。
  required_action: 現在は実装と利用可能な検証を継続する。対象環境の提供またはownerの再開指示後に該当証拠を収集し、正式release時に署名と配布条件を確定してstrict検証後にowner GOを得る。
  blocks_release: yes

## Android仮想端末とApple補助結果の確定（2026-09-13）

source `9c4d89efcb5dedf2b5d229dcd9fd4ef0b2b47bc7` の手動Android run `34743675304` とApple run `34743676639` がsuccessで終了し、取得成果物のenvironmentとsourceを照合した。双方とも追跡差分は空、未追跡表示は当該jobのevidence directoryだけである。Androidはnative保管2件とMobile29件・解析がPASS。実TLS統合でnative再読取・controller再生成・OS背景停止と復帰再接続・二つの実MINIDORA応答・失効資格拒否がPASSした。従来のAVD容量不足とCRLF応答による検証未到達はこのsourceで解消した。

AppleはRust74単体・5 IPC・7 checkpoint・2差分・2取得器統合、共有18・Desktop33・Mobile29、native保管2件、実TLS統合とmacOS/iOS Simulator buildがPASS。今回追加したBroker読取制御より前のsourceであり、その新実装のApple証拠へ読み替えない。Apple tar SHA-256は `09d6fb2677ecf68fcbf4e43107522827cc360c03937a245195f818b6dada20f8`、Android成果物各fileのhash集合のSHA-256は `36879e4d1703196033383ae1ef7f41c94ec7a82553a04210a5570ba4af913d6e`。Repository外のGUI-Shell-apple-9c4d89eとGUI-Shell-android-emulator-9c4d89eへ原成果物とverified-result.jsonを保存した。physical_device_verified=falseを維持し、実機・正式署名配布・owner GOはrelease_blockerのままとする。

## Android補助hostの容量と終了待機を修正（2026-09-13）

9a60cdd/run `34742562494` は有効configでuserdata=2Gとなっていたが、emulatorの要求は7372.80MBのままで、空き6986.07MBに対して再び起動前FATALとなった。設定変更が実容量を減らしたとは言えず、2G指定を除去する。元のAPI35 imageに必要な領域を確保する。SDK記録にはNDK27.3・28.2・29.0が同居し、固定Flutter3.44.0のFlutterExtension.ktとapp設定の消費は28.2.13676358である。隔離された手動runnerだけでSDK managerのuninstallにより未使用の27.3.13750724と29.0.14206865を除き、28.2の存在を検査する。対象版以外を計算して削除しない。変更前後のSDK一覧と工具結果、AVD有効設定、空き容量を保存する。ローカルhost・ownerのfile・製品依存を削除せず、必要NDKが変わるときはこの固定照合を再検討する。

先行c86da93/run `34741135813` はcancelledで終了したが、artifactのemulator.txtにboot 61356ms、avd-name.txtに正しいAVD名とOKのCRLF応答が残り、Flutter試験logは存在しなかった。既存grep -Fxは末尾CRを含む値と一致せず、cleanupのwaitには期限がなかった。AVD名は外部consoleの行末CRだけを除去してから完全一致を要求し、ADBの観測にも10秒期限を付ける。cleanupはこのstepが起動したPIDだけへ終了を要求し、10秒後も残る場合はKILLして回収する。boot条件・仮想属性・名前・native試験を省略しない。未成立のAndroid統合はrelease_blockerとして維持する。

Schema37/正常37/負例39、conformance151件、厳格日本語監査、YAML読取とworkflow6 stepのbash構文検査はPASS。隔離fixtureでCRLF応答の正常一致・別名拒否、TERMを無視する自分の子processがcleanupで15秒以内に回収されることを実行確認した。SDK削除はローカルでは実行せず、修正後の手動runnerで結果と容量を確認する。これらの局所検証をAndroid native統合成功へ昇格しない。

## Android専用AVDのuserdata容量を明示（2026-09-13）

3431173/run `34741774416` はemulator起動前に失敗し、Flutter native試験には未到達だった。artifactのemulator.txtで、専用AVD directoryの空き6987.37MBに対しuserdata作成が7372.80MBを要求したFATALを確認した。KVM・AVD存在・system image検出は成立しており、前回のAVD未検出とは異なる容量不足である。Repository外のGUI-Shell-android-emulator-3431173へ原logを保存した。

専用AVDの標準設定disk.dataPartition.sizeを2Gに明示する。[Android公式の仮想端末設定例](https://android.googlesource.com/platform/external/adt-infra/+/refs/heads/emu-master-dev/emu-image/templates/avd/Pixel2.avd/config.ini)にもある通常設定で、製品runtimeの代替実装や容量検査の抑止ではない。この短時間のnative保管・実TLS試験用AVDに限る。作成時設定と有効設定、起動前の空き容量を証拠へ保存し、emulator側の容量検査、boot、仮想属性、実試験の判定は維持する。既存fileやSDKを削除せず、実機凍結にも変更はない。大量data試験を追加するときは容量と取得証拠の範囲を再検討する。2Gでの実起動と試験は修正後の手動runで確認し、未成立の間はAndroid仮想端末統合をrelease_blockerに保持する。

Schema37/正常37/負例39、conformance151件、厳格日本語監査、workflow6 stepのbash構文検査はPASS。設定更新部分は一時fixtureで正常更新・key欠落拒否・重複拒否を実行し、対象key以外が変わらないことと拒否時にfileを変更しないことを確認した。これらはAVD実起動の証拠ではない。

## Apple実TLS統合の証拠確定とAndroidへの接続（2026-09-13）

fd1b23e/run `34740828826` は手動Mac job全体が成功した。保存したartifactのtar SHA-256は `3f62ce784508b8dcfdd0ecb3bd3fba86b5e287570309ca6f7b6fea9f7f94e9b9` と一致し、追跡差分は空。共有18・Desktop33・Mobile29試験と解析、native保管2試験、macOS/iOS Simulator buildを確認した。実TLS統合結果はnative再読取、controller再生成、OS背景時停止と復帰再接続、二つの実MINIDORA応答、失効後拒否がPASS。Repository外のGUI-Shell-apple-fd1b23e/verified-result.jsonへ対象sourceと範囲を保存した。過去のMac起動期限超過はこのsourceで解消した。physical_device_verified=falseであり、正式配布・実機の保護特性・owner GOは証明しない。

同じdevelopment専用統合をAndroidへ接続する。Linux hostとemulator-5554を固定し、harnessとdriverの双方がro.kernel.qemu=1と専用AVD名gui_shell_native_testを確認してから進む。端末列挙や実機操作は行わない。OSのHome遷移と明示MainActivityの再前景化を使い、製品のlifecycle observerで停止・復帰を観測する。接続先には[公式のhost loopback alias](https://developer.android.com/studio/run/emulator-networking-address)である10.0.2.2を一時招待生成時に指定する。brokerは127.0.0.1へbindしたまま、証明書hash照合・認証・Approval・Auditを通す。製品network境界の変更、平文化、証明書検証の省略はない。接続情報は一時fileと認証付きdebug VMで渡し、成果物へ資格を保存しない。

変更後のMobile `flutter analyze --no-pub` と `flutter test --no-pub --reporter expanded` は29件PASS。Schema36/正常36/負例38、conformance150件、厳格日本語監査、手動workflowの6 stepのbash構文検査がPASS。既存Windows実API・実TLS・両Dart client・監査再読取は同じharnessの全client指定でPASSし、Repository外のGUI-Shell-android-native-harness-regression.txtに保存した。host不一致・実機serial・非仮想属性・異AVD名の拒否4件は呼出を差し替えたFIXTUREとして検証し、実機操作を行っていない。新Android経路の実行証拠とは区別する。

- item: Android仮想端末のnative保管・実TLS・OS復帰の実行結果
  classification: release_blocker
  reason: この追加経路はローカル解析だけでは仮想端末で成立したと判断できない。
  required_action: 対象commitを手動runnerで実行し、専用AVD属性・実結果・対象sourceを照合する。非Windows実機の延期は維持する。
  blocks_release: yes

## Android workflowの変数評価位置を修正（2026-09-13）

fd1b23eのAndroid手動dispatchはHTTP422で拒否された。jobのenvでrunner.tempを参照した位置ではrunner contextを使用できず、GitHubのworkflow検査を通らなかった。YAML・bashの構文検査はこのGitHub固有のcontext制約を検証していなかった。ANDROID_AVD_HOMEの値は、実行開始stepでRUNNER_TEMPを使ってGITHUB_ENVへ設定し、後続の作成・起動stepへ同じ値を渡す。起動条件、保存先の境界、AVD存在検査、仮想端末属性確認を変更しない。手動dispatchと実行結果は修正後commitで確認する。未実行のAndroid native保管は上記release_blockerとして保持する。Schema36/正常36/負例38、conformance150件、厳格日本語監査は修正後もPASS。

## 仮想端末検証hostの起動前提を修正（2026-09-13）

Macのcf5c771/run `34739605831` attempt2は、参照のimportが0.096秒以内に終わり、HTTP bind中の `socket.getfqdn` で10秒後も待機していた。stackはPython3.14のHTTPServer.server_bindからの逆引きを示した。attempt1のGitHub DNS取得失敗は別の環境失敗として保持する。Windowsの同じ経路は約0.47秒／0.38秒で起動している。

このharnessはliteralな127.0.0.1だけへbindし、参照のAPIHandler・製品チャットはserver_nameを権限や接続先として利用しない。通常constructorには逆引きを省略する指定がないため、stdlibのbind_and_activate=False、TCPServer.server_bind、server_activateを使用し、表示用server_nameも実bind先の127.0.0.1へ固定する。試験の期限、MINIDORA本体、API/trace、Rust broker、実送信経路は変えない。これは全host共通のdevelopment限定初期化であり、通常製品runtimeへのwrapperではない。DNS名による参照server提供を追加する場合はこの前提を再検討する。

Androidのd952991/run `34739883443` はKVM利用可能とSDK準備を確認したが、emulatorがAVD名を見つけられず終了し、ADB待機が期限切れとなった。標準のANDROID_AVD_HOMEをjob内の専用directoryへ固定し、avdmanagerの作成path、iniの存在、emulatorの列挙を照合してから起動する。起動成功を推定せず、端末属性・AVD名・native試験の既存判定も維持する。環境変数の定義は[Android公式資料](https://developer.android.com/tools/variables)を参照した。 初回の圧縮容量2.20GBはpreview channelのemulator 37.2.8を拾っていたため補正する。安定版37.1.11は441,926,448 bytes、API35 imageとの合計は2,180,742,351 bytes（約2.18GB）。観測時空き容量約2.19GBとほぼ同量で、別途必要な展開領域の余裕がないという判断は保持する。

逆引きを必ず例外にする故障注入で、harness内の実server起動コードを実行し、固定MINIDORAのcapabilities APIがHTTP200を返すことを確認した。証拠はRepository外のGUI-Shell-no-reverse-dns-proof.json。Windowsの実API・実TLS・両Dart client・監査chain再読取もPASSし、GUI-Shell-loopback-bind-regression.txtへ保存した。Schema36/正常36/負例38、conformance150件、日本語監査、Android workflowの5 step構文検査もPASS。

- item: 修正後のMac実TLS統合とAndroid native保管
  classification: release_blocker
  reason: 原因に対応した検証環境修正であり、修正後の手動runが成功した証拠はまだない。
  required_action: ローカル回帰後に両手動runを実行し、対象commitと実結果を確認する。
  blocks_release: yes

## Android仮想端末の手動補助経路（2026-09-13）

Windows SDK managerの一覧にemulator 37.1.11とAPI35 Google APIs x86_64 revision9を確認した。SDK catalogで確認した圧縮imageは1,738,815,903 bytes、Windows emulator候補は459,029,121 bytes。空き容量約2.19GBのhostでは展開余地がなく、ローカル導入を強行しない。中間生成物削除の自動承認拒否を迂回せず、許可された手動補助環境で仮想端末検証を進める。

Ubuntu 24.04 runner上で専用AVDを作成し、Mobile解析・29既存試験とnative安全保管2試験を実行するworkflowを追加する。triggerは手動限定、contentsはread、Flutter/actionは既存と同じ固定点、証拠保管3日。ADB対象を明示作成のemulatorに固定し、qemu属性とAVD名を照合する。KVMの利用者限定権限設定は隔離されたdevelopment hostのための設定であり、製品Permissionへ転用しない。SDK licenseの自動承認は追加せず、既存runnerで不足する場合は失敗として保持する。 ローカルではSchema36/正常36/負例38、conformance150件、厳格日本語監査、5つのrun stepの `bash -n` がPASS。これらはCONFIG・静的検査であり仮想端末の実行証拠ではない。

参考一次資料: [Android Emulatorの加速方式](https://developer.android.com/studio/run/emulator-acceleration)、[Ubuntu 24.04 runner image](https://github.com/actions/runner-images/blob/main/images/ubuntu/Ubuntu2404-Readme.md)。これらの記載を実行証拠にせず、使用toolchainと起動結果はrunごとに記録する。

- item: Android仮想端末のnative保管実行証拠
  classification: release_blocker
  reason: workflow追加だけでは仮想端末の起動・native API成功を証明しない。
  required_action: push後の手動runで実行し、対象commitと結果を照合する。Android実機凍結は維持する。
  blocks_release: yes

## Mac上の参照Runtime起動期限超過を診断（2026-09-13）

c5aa1ecの手動run `34739077800` はnative保管2試験がPASSした後、最初のMINIDORA参照APIの起動確認で20秒期限を超過した。Simulator実TLS試験は未到達でありPASSにしない。Windowsの同じ参照・harnessの実接続は成功しているが、Mac環境の根本原因を確定したものではない。

開発専用server起動コードへ、import、HTTP bind、製品初期化、readyの経過秒記録と10秒時点のstack採取を追加する。期限、参照製品、実API、権限経路を変更せず、待機箇所を観測する。失敗時は段階名・経過秒と起動前stackだけを出力し、資格や要求本文、localsを出力しない。通常製品runtimeには追加しない。Windowsの実API二実行系・両Dart client・実TLS・監査chain再読取は計測追加後もPASS。Schema36/正常36/負例38、conformance150件、厳格日本語監査もPASS。ログはRepository外のGUI-Shell-runtime-startup-diagnostics-regression.txt。

- item: Mac上の参照Runtime起動とSimulator実TLS統合
  classification: release_blocker
  reason: 起動期限超過の原因は未確定で、実TLS統合へ到達していない。
  required_action: 計測結果から原因を特定し、根拠のある修正後に同じ統合試験を再実行する。
  blocks_release: yes

## native保管・実TLS・実API・OS復帰の統合試験（2026-09-13）

先行d9f23cdの手動run `34738442843` は、Simulator上のnative安全保管2試験を含め成功した。これを実TLSやOS lifecycleの証拠へ読み替えず、次の統合試験を追加する。

既存minidora_live_checkへMac専用 `--mobile-simulator <UDID>` を追加した。固定3400a3bの実API二実行系と一時Rust brokerを使い、試験招待だけを認証付きFlutter debug VMからintegration_testへ渡す。native保管後のcontroller再生成、証明書不一致の拒否、製品MobileHomeへのOS背景・復帰通知、左右の実応答、保存資格消失時の停止、端末離脱とnative削除・失効後拒否を検査する。追加経路はdevelopment専用で、運用資格・実鍵・製品runtimeを変更しない。試験の失敗をAPI成功へ昇格せず、期限とprocess cleanupを既存harnessへ接続する。

Windows上の `flutter analyze --no-pub` とMobile29試験はPASS。変更後の `python tooling/minidora_live_check.py --reference C:/Users/mzcum/codex-work/MINIDORA-reference --binary C:/Users/mzcum/codex-work/GUI-Shell/native/rust_helper/target/release/gui_shell_rust_helper.exe --dart-client --mobile-client --dart-mobile-client` はPASSし、既存の実API・実TLS・両Dart client・監査chain再読取の回帰なしを確認した。ログはRepository外のGUI-Shell-native-integration-harness-regression.txt。新Simulator経路の実行証拠はpush後の手動runnerで別途確認する。

- item: Simulatorでのnative実TLS統合の実行証拠
  classification: release_blocker
  reason: Windows解析・試験はMac上の追加統合経路を実行していない。
  required_action: 対象commitの手動runで統合経路を実行し、実結果と保管済み成果物を確認する。実機限定事項は延期を維持する。
  blocks_release: yes

## Mobile native安全保管の仮想端末試験経路（2026-09-13）

SDK付属integration_testをdevelopment依存に追加し、実SecureDeviceStoreを使う2試験を作成した。書込・別instance読取・更新・削除、端末IDの再読取、資格なし・破損資格の通信停止を検査する。試験キーはprefixで分離し、既存製品資格を読まない。前景切替はcontroller入力でありOS lifecycleではない。手動Apple補助は利用可能なiPhone Simulatorを選択・起動し、試験後に終了する。契約・本番権限・平文fallbackは変更しない。

変更前b07dba3の手動run `34738095886` は共有18・Desktop33・Mobile29試験と解析、Rust64単体・5 IPC・7 checkpoint、macOS/iOS Simulator buildが成功した。ログはRepository外のGUI-Shell-apple-b07dba3-log.txt。今回追加したnative試験の結果とは区別する。成果物の最初の取得とローカルMobile試験はディスク容量不足で失敗した。処理終了後の空き容量回復を確認し、Mobile試験を再実行して29件PASSを確認した。追加試験を含む `flutter analyze --no-pub`、Schema36/正常36/負例38、conformance150件、厳格日本語監査もPASS。再生成物の削除は自動承認レビューで拒否され、迂回していない。

- item: native安全保管のSimulator実行と実接続・OS lifecycle
  classification: release_blocker
  reason: native試験の実行はpush後の手動runnerで確認する必要があり、controller入力だけではOS lifecycleやTLS再接続を証明できない。
  required_action: Simulator上の実行結果を収集し、次に実TLS接続とOS lifecycleの試験を接続する。非Windows実機は指定どおり延期する。
  blocks_release: yes

## Apple手動補助へFlutter解析・試験を追加（2026-09-13）

手動workflowはRust試験とapp buildのみで、共有UI・Desktop・MobileのFlutter試験を実行していなかった。各packageの解析と試験を追加し、依存解決・解析・試験のログを対象commitの補助成果物へ保存する。pipefailにより解析・試験の失敗をbuild成功で隠さない。固定toolchain・手動起動限定・read権限・保管期間・追跡差分拒否は保持する。これはdevelopment専用経路であり、製品の権限や外部送信経路を変更しない。

変更前46f384fで `python tooling/validate_all.py --desktop-platform windows --include-mobile-release` は正常終了し、共有18・Desktop33・Mobile29試験を確認した。ログはRepository外のGUI-Shell-extension-c0-baseline.txt。Mac上の実行結果はこの変更をpush後に手動実行して対象commitと結合して確認する。

- item: Mobile native保管・lifecycle・再接続の仮想環境統合検証
  classification: release_blocker
  reason: この追加はMac host上のFlutter試験であり、SimulatorやAndroidエミュレータのnative実行証拠ではない。
  required_action: 実機凍結を維持し、利用可能な仮想環境でnative統合試験を実装・実行する。
  blocks_release: yes

## 比較画面の応答照会を左右独立に実行（2026-09-13）

共有Widgetの応答照会が左右直列で、左の通信待機中は右の完了応答を表示できないことを再現した。照会中フラグを左右別に持ち、各側の周期照会を独立させる。同じ側への重複照会を防ぎ、旧要求の遅延応答・失敗を現在表示へ転用しない条件は維持する。変更はUIの表示取得経路で、Rustの権限・Approval・監査・外部送信は変更しない。

左右それぞれを待機させる製品Widget試験で、他側の先行表示、待機中の重複照会なし、待機解除後の両結果表示を確認した。`python tooling/validate_all.py --desktop-platform windows --include-mobile-release` は開発検証17項目PASS。共有18・Desktop33・Mobile29試験、3か所の解析、Schema36/正常36/負例38、conformance150件、Rust63単体・5 IPC・8 checkpoint、厳格日本語監査を含む。ログはRepository外のGUI-Shell-independent-poll-validation.txt。埋込installed証拠は変更前b84bbbdのものであり、この修正後の実機証拠ではない。証拠は通信を遅延させたFIXTUREであり、実機の通信遅延測定ではない。非Windows実機・正式配布・運用署名・owner GOの延期は冒頭のrelease_blockerに保持する。

## 実行系列挙の重複による選択欄の不整合を拒否（2026-09-13）

共有clientは実行系IDの型・形式・件数だけを検証しており、重複を受理していた。leftを2件返す応答でclientの誤受理とFlutter Dropdownのassertionを別々に再現した。実行系列挙の一意性をclientで検証し、重複応答は既存の接続エラー表示と送信停止へ接続する。重複を黙って除去せず、正常な返却順序と空一覧を保持する。Rustの登録・権限・列挙内容は変更しない。

共有packageの `flutter test --no-pub --reporter expanded` は16件PASS。隣接・非隣接の重複拒否、正常順序、空一覧、製品Widgetの接続エラーと送信無効を確認した。共有・Desktop・Mobileの `flutter analyze --no-pub` はPASS。証拠は不正応答を注入した製品client／WidgetのFIXTUREである。非Windows実機・正式配布・運用署名・owner GOは冒頭のrelease_blockerとして延期を保持する。

## 対話結果の参照・能力配列を検証後に固定（2026-09-13）

DialogueResultの外側のMapは変更不可だったが、参照・能力のListは受信元と共有していた。検証後に元の空配列へ要素を追加するとnoneの表示結果にも現れ、公開getterからも配列を変更できることを2試験で再現した。検証通過後に両配列をコピーして変更不可にし、文字列とMapだけでなく結果全体の検証済み内容を固定する。新しい公開内容には再検証を要求し、表示資格の判定を増やさない。

共有の `flutter test --no-pub --reporter expanded` は14件PASS。none/fullで元配列の変更が伝播しないこと、getterとfieldsの両経路からの変更が拒否されること、既存の公開参照・能力が保持されることを確認した。共有・Desktop・Mobileの `flutter analyze --no-pub`、Schema36/正常36/負例38、conformance150件もPASS。証拠は製品parserとWidgetを実行するFIXTUREである。broker・TLS・実機保管は変更していない。非Windows実機・正式配布・運用署名・owner GOは冒頭のrelease_blockerとして延期を保持する。

## 中止進捗と内包結果の状態を照合（2026-09-13）

共有対話clientで、進捗が中止なのに結果が成功で本文を持つ応答を受理することを再現した。Rustの中止処理は中止結果を返すため、clientにも進捗と結果の対応検証を追加した。中止進捗に成功・保留・失敗が混在する3負例を拒否し、正しい中止結果を受理する。Rustの採否・中止処理や表示資格を変更しない。

共有packageの `flutter test --no-pub --reporter expanded` は12件PASS。共有・Desktop・Mobileの `flutter analyze --no-pub`、Schema36/正常36/負例38、conformance150件、日本語基底監査はPASS。変更中の作業ツリーで `python tooling/minidora_live_check.py --reference C:/Users/mzcum/codex-work/MINIDORA-reference --binary C:/Users/mzcum/codex-work/GUI-Shell/native/rust_helper/target/release/gui_shell_rust_helper.exe --dart-client --mobile-client --dart-mobile-client` もPASSした。固定参照3400a3bの実API二実行系、owner CLI承認、通常資格拒否、表示分離、trace、保留、片側失敗、両失敗、監査chain再読取、両製品Dart clientとTLS経路を確認した。ログはRepository外のGUI-Shell-dialogue-cancel-consistency-live.txt。

不正状態の注入はFIXTURE、通常通信の回帰はLIVE_RUNTIMEである。Mobile実機安全保管・OS lifecycleの証拠ではない。非Windows実機・正式配布・運用署名・owner GOは冒頭のrelease_blockerとして延期を保持する。

## セッション切替の部分失敗と旧結果表示（2026-09-13）

旧セッションの終了成功後に新規開始が失敗すると、sessionだけが空になり旧要求・応答・完了状態が残る不具合を製品Widgetで再現した。旧セッションの終了成功時点で現在表示を未開始へ戻し、旧要求と応答を外す。終了自体が失敗した場合は旧sessionと結果を維持し、終了成功を推定しない。監査記録やbrokerのsession管理は変更しない。

開始失敗・終了失敗の2試験を追加した。変更前は開始失敗の旧応答消去がFAIL、終了失敗時の保持はPASS。変更後は両方PASSで、その後の手動再試行による新規開始も確認した。共有・Desktop・Mobileの `flutter test --no-pub --reporter expanded` は11・33・29件PASS、3か所の `flutter analyze --no-pub` もPASS。Schema36/正常36/負例38、conformance150件を確認した。

証拠範囲は製品Widgetを実行するFIXTUREである。新しい実機操作・ビルド証拠を今回作ったとは主張しない。非Windows実機・正式配布・運用署名・owner GOは冒頭のrelease_blockerとして延期を保持する。

## 共有対話画面の遅延失敗を元の要求へ限定（2026-09-13）

応答照会中に中止して新規セッションへ切り替えると、旧要求の遅延通信失敗が新しいセッションのerrorを上書きすることをWidget試験で再現した。成功応答は既に要求IDとpendingを検査していたが、catch側には同じ結合確認がなかった。例外の表示にも現在の要求ID一致と待機中の条件を要求した。経路はDesktop/Mobile共有UIの表示処理で、brokerの採否・取消・監査は変更しない。

初回の再現試験は遅延を設定する前に周期照会が完了し、取消状態の前提を満たせなかった。送信前に応答の待機を設定して順序を固定すると、修正前は遅延成功の除外がPASS、遅延失敗の除外がFAILとなった。修正後は両方PASSで、現在待機中の要求の失敗は表示する対照試験もPASSした。これは製品Widgetを実行するFIXTUREであり、実ネットワーク障害を起こした実機証拠ではない。

共有package・Desktop・Mobileで `flutter test --no-pub --reporter expanded` がそれぞれ9・33・29件PASS、3か所の `flutter analyze --no-pub` もPASS。`python tooling/schema_check/check_schemas.py`（Schema36/正常36/負例38）、`python tooling/conformance_tests/run_conformance_skeleton.py`（150件）、`python tooling/日本語基底監査.py --strict` はPASS。非Windows実機と実運用署名の延期は冒頭のrelease_blockerに保持する。

## Mobile復帰時の安全保管の再確認（2026-09-13）

仕様が求める復帰時の資格読取に対し、製品controllerはメモリ上の資格だけで端末確認を再開していた。保存資格の削除・破損・秘密変更・Host変更・期限変更・端末ID変更・読取障害の7負例すべてで、変更前に接続再開を再現した。

起動・復帰・手動再確認の共通経路で端末IDと保存資格を再読取し、構造・期限・現在の結合内容の一致を確認してから通信する。保存値から別資格を自動採用せず、不一致や読取障害では停止する。非同期の保管読取・端末確認・実行系列挙の間にbackgroundへ移った結果を後の復帰へ転用しないよう、前面状態の世代を照合する。読取中のbackground移行では通信せず、その後の復帰で再確認できる試験も追加した。

`flutter test --no-pub --reporter expanded` はMobile全29件PASS。Desktop/Mobileの `flutter analyze --no-pub`、`python tooling/schema_check/check_schemas.py`（Schema36/正常36/負例38）、`python tooling/conformance_tests/run_conformance_skeleton.py`（150件）、`python tooling/日本語基底監査.py --strict` はPASS。保管と通信の障害注入はFIXTUREで、製品controllerの復帰制御を検証した範囲である。非Windows実機の安全保管・OS lifecycleは冒頭のrelease_blockerとして延期を保持する。

同じ変更中の作業ツリーからWindows上で `flutter build apk --debug --no-pub` が成功した（Gradle 187.1秒）。生成APKは175742389 byte、SHA-256 `6fe9a83c2ab31a80e47729f0c02daab7a0ba1aa00ef5984b54b052ed8713cc66`。debug buildの成立範囲であり、実機install・起動・正式配布の証拠ではない。artifactはignoreされたbuild領域に保持する。

## Mobileの解除・保存資格削除の確認（2026-09-13）

Mobileのcontrol経路で、保存APIのdeleteが例外なく戻るだけで削除完了と表示していた。通常解除と端末内だけの削除の両方で、資格残存・削除後の読取障害を注入した4負例が変更前に失敗した。またclient不在でdisconnectを直接呼ぶと、端末離脱を送らずDesktop解除成功と表示する負例も再現した。既存UIはclientを作れない破損資格で通常解除を無効にするが、controller自身にも拒否を置いた。

通常解除には端末離脱の確認を要求し、保存資格はdelete後に同じDeviceStoreから再読取して不在を確認する。失敗時は資格の管理状態を保持して通信を止め、未確認と表示する。通常解除・端末内削除の正常2例と負例5例を追加し、製品controllerを呼ぶMobile試験は全21件PASS。Desktop/Mobileの `flutter analyze --no-pub`、Schema36/正常36/負例38、conformance150件、日本語基底監査もPASS。Mobile試験commandは `flutter test --no-pub --reporter expanded`。

試験の保管・通信は障害を注入するFIXTUREで、productionのDeviceLinkControllerの採否・表示・通信停止を検証した。Rustの失効・監査経路、TLS、保管pluginは変更していない。Android/iOSの実機安全保管の証拠へは昇格しない。実機未検証は冒頭のrelease_blockerに保持し、現在の開発を継続する。

## 統治変更（2026-09-10）

対象はローカル品質判定と手動補助 Actions の分離。製品 runtime の権限・実行経路は変更しない。自動 CI は禁止を維持し、手動起動条件を構造として検査する。証拠分類は CONFIG / FIXTURE であり、外部実行や branch protection の保証ではない。

`python tooling/schema_check/check_schemas.py`、`python tooling/conformance_tests/run_conformance_skeleton.py`、`python tooling/日本語基底監査.py --strict` は PASS。Conformance は142項目。手動起動の文字列・列挙・対応形式、不在、実ファイル読取りを確認し、自動起動の混在、重複鍵、不正 YAML、独自タグを拒否した。

`PYTHONUTF8=1` を設定した Windows の `python tooling/validate_all.py --desktop-platform=windows` は FAIL。既存の `packaging_portability_check` だけが失敗し、他の13検査は PASS。Rust は42単体＋4統合、Flutter は31試験、desktop/mobile analyze と broker parity も PASS。日本語4ファイルの展開名不整合は変更前から存在し、独立した最小 ZIP でも再現した。UTF-8 locale 指定でも改善しない。統治変更に起因する失敗ではない。

WSL/Linux の `python3 tooling/validate_all.py --python-only --desktop-platform=linux` は9検査すべて PASS。これは Python 側の検証であり、reporter が併記する過去の Linux build / launch 記録を今回の実行証拠とは扱わない。Windows の環境依存の失敗は以下へ分離して保持する。

- item: Windows の ZIP 展開名不整合
  classification: release_blocker
  reason: Git for Windows 同梱 unzip で日本語名が変化する環境依存の失敗。統治変更で検査を除外しない。
  required_action: 次の Baseline 単位で標準展開機構を確認し、同一の manifest / conformance / release gate 検査を維持して修正する。
  blocks_release: yes

- item: installed-path 証拠・実機検証・owner GO
  classification: release_blocker
  reason: 統治単位は製品の完成証拠を作成しない。strict release は未実行である。
  required_action: 製品単位で正式な実機証拠と明示承認を揃える。
  blocks_release: yes

後続の実行系対話、MINIDORA Adapter、比較、Mobile、端末連携、各 platform の実装と検証は未完了。今回の統治単位の完成をそれらの完成へ昇格しない。これらの owner 指示内の未実装は rev2 完成に対する release_blocker として次単位以降で扱う。GitHub Actions は未使用。

## Baseline の Windows 配布検証修正

Git for Windows 同梱 unzip 6.00 は最小 ZIP の日本語名も文字化けさせた。`LC_ALL=C.UTF-8` と `-UU` でも再現した。一方、Windows 標準 tar.exe（bsdtar 3.8.8）は `LC_ALL=C` のまま同じ ZIP の名前を保持した。

検証専用経路を Windows は System32/tar.exe、POSIX は従来の unzip に分けた。独自 wrapper、代替の製品 runtime、検査除外は追加しない。展開後の manifest、conformance、release gate 検査は維持する。適用範囲は dev / release validation の source ZIP 展開のみである。新しい日本語名・空白を含むパスの実展開、内容hash一致、破損ZIPの拒否、展開器不在を試験する。これは当該 OS の外部展開器に対する EXTERNAL_EVIDENCE であり installed 製品証拠ではない。

本修正は Windows 標準機構の恒久的な選択である。将来対応OSの標準展開器が変わる場合は、同じ名前・内容・失敗の試験と実展開後検査を成立させて選択を見直す。

検証結果: Windows の `python tooling/validate_all.py --desktop-platform=windows` は14検査すべて PASS（終了値0）。WSL/Linux の `python3 tooling/validate_all.py --python-only --desktop-platform=linux` は9検査すべて PASS。Conformance は143項目。上記の Windows ZIP 展開名不整合はこの修正で解消した。installed-path 証拠と owner GO は未解消の release_blocker のままであり、開発用集約結果の pass を製品 release の許可にしない。

## 日本語意味正本・契約・Conformance

`docs/specs/runtime-dialogue.md` で要求・応答・セッション・比較、取消の限界、表示境界、権限と監査の前提を定義した。MINIDORA API の実コードは commit `3400a3bb68b37efa1dc14ee8aaa28fda779bf1f8` へ固定した。参照系のコードは変更しない。

四つの Schema、正常・権限混入の否定fixture、開発専用の関係検査を接続した。要求上限、本文上限、参照数上限、空白入力、識別子末尾の改行、未知field、閉じたセッション、応答対応、非全文表示への漏洩、左右成功・片側失敗・両失敗・応答入替え・セッション共有・表示許可転用を検査する。これらは CONFIG / FIXTURE 証拠であり、MINIDORA の live 動作、実行系停止、権限発行の証拠ではない。

- item: 製品対話の実装接続
  classification: release_blocker
  reason: 現単位の消費経路は Schema catalog と開発用 conformance。Rust 外部送信、UI、端末連携は未接続。
  required_action: 次単位で統治済み Rust 経路と Adapter、UI を実装し、実物通信と失敗・権限否定・取消を検証する。
  blocks_release: yes

契約単位の検証: Schema 30件、正常例30件、否定例32件、Conformance 145項目は PASS。Windows 開発用一括検証14検査と WSL/Linux の Python 側9検査は PASS。比較単独での空白入力の見逃しを追加監査で検出し、応答・比較の関係検査も拒否するよう補強した。権限判定や許可発行をこの開発検証へ持ち込まない。

製品接続の前提確認: 現行 `native/rust_helper/src/broker/authority.rs` の production_default は decision=deny、approvals=[] であり、owner 承認の登録経路がない。継続指示に基づき専用の Rust ローカルowner制御操作を採用した。以下の単位で通常UI資格と分離する。既存汎用commandのdeny/suspendを解除する変更ではない。

## Rust対話Core・MINIDORA Adapter・owner制御

専用の対話要求を通常IPCで保留し、別資格のowner CLIによる要求hash一致・期限内・一回限りの承認後だけ固定loopback APIを呼ぶ。Shell Coreは汎用traitを使い、MINIDORA固有のHTTPと応答射影はAdapterへ分離した。Flutter、Python、metadataから権限を発生させない。既存汎用command dispatchはsuspendedを保持する。

通信期限、要求・受信上限、worker数上限、セッション隔離、取消後の採用防止、raw受信と表示射影の分離を実装した。監査失敗時は送信または結果公開を拒否する。成功Adapter応答もCoreでセッションと構造を再検査する。owner資格は同一OS利用者に対する強い隔離や人間本人性の証明ではない。Windows installed-path保護の未検証を隠さない。

検証: `cargo test --manifest-path native/rust_helper/Cargo.toml` は53単体・5統合がPASS。`python tooling/minidora_live_check.py --reference C:/Users/mzcum/codex-work/MINIDORA-reference` は固定commitの実MINIDORA二processを用いPASS。通常資格拒否、owner CLI承認、基本会話、trace整合、表示分離、保留、片側停止・両側停止、監査chainの再起動読取を実行した。実API証拠はLIVE_RUNTIMEだが、基礎Core・外部検索能力の保証へ拡張しない。Schema31件・正常例31件・否定例33件、Conformance146項目、日本語厳格監査もPASS。

HTTP固定ヘッダーとCRLFだけを `JBE-007` の局所固定表記へ登録した。監査器の検出は弱めず、説明と診断は日本語のまま残す。Brokerは非同期状態を所有するためClone/Eqの導出を廃止した。生存workerや承認待ち状態を複製する内部APIは提供しない。

- item: Desktop比較UI・Mobile・端末連携・platform別実機証拠
  classification: release_blocker
  reason: この単位で接続したのはRust対話経路と実API検証。UI実装、secure storage、端末資格、Android/iOSの実動作は後続単位に残る。
  required_action: Flutter共有化と対話表示、二実行系比較、端末連携、platform別buildと実機検証を順次実装する。
  blocks_release: yes

- item: owner資格のOS保護とinstalled-path検証
  classification: release_blocker
  reason: ローカル開発資格の分離は同一利用者の任意processや管理者への耐性を証明しない。
  required_action: 既存の資格保護・監査anchorのrelease gateに従いWindows installed-path証拠を収集する。
  blocks_release: yes

集約検証: Windowsの `python tooling/validate_all.py --desktop-platform=windows` は14検査すべてPASS（終了値0）。WSL/Linuxでも `cargo test --manifest-path native/rust_helper/Cargo.toml` の53単体・5統合がPASSし、Linux binaryとLinux側の同一commit参照cloneによる `tooling/minidora_live_check.py` がPASSした。Windows cloneをWSL Gitで確認した際の改行差分は参照コードを書き換えず別cloneで分離した。GitHub Actionsは未使用。これらは開発環境の検証であり、installed-pathやMobile実機、owner GOのrelease_blockerを解除しない。

## Desktop対話・二実行系比較

Desktopのナビゲーションとコマンドパレットに対話を追加した。新規セッション、実行系選択、入力、送信、中止、応答、参照、能力、経路、追跡、失敗・復旧を表示する。比較では異なる実行系とセッションへ同じ入力を独立送信する。通常clientの操作allowlistにowner承認を含めず、応答の要求・実行系・session・表示境界を検査する。画面遷移で対話を保持し、デモ表示では新しい操作を無効にする。下部バーは対話の状態を起動時snapshotから推定しない。

接続資格のhostは127.0.0.1、TCP port・乱数secretの構造を検証し、Flutterの受信上限を4MiBにした。実行系への直接通信は追加していない。

Windowsで `flutter analyze`、`flutter test`（37件）、`flutter build windows --debug` を実行した。画面試験は左右の同一入力、独立した失敗表示、中止、デモの送信禁止を含む。製品Dart clientによる `python tooling/minidora_live_check.py --reference C:/Users/mzcum/codex-work/MINIDORA-reference --dart-client` もPASS。

Windowsの開発binaryを実起動し、Computer Useでコマンドパレットから対話画面へ移動、比較を選び「こんにちは」を送信した。左右に異なるsession・要求が作られ、owner CLI承認後に両方の実MINIDORA応答と異なる追跡IDが画面表示されたことを観測した。これは開発binaryのLIVE_RUNTIME観測であり、installer配布・installed-pathのrelease証拠ではない。

Dartの自動修正toolは終了時にperf一時file削除のOS Error 1920で失敗したため、lint指摘をソースで修正し、その後のanalyze/testで検証した。製品コードにtool障害の回避層は入れていない。

次単位のbackup更新で短いref名がbranch/tagの同名と衝突した。`git push -f origin codex/backup-main-prev:refs/tags/codex/backup-main-prev codex/backup-main:refs/tags/codex/backup-main` は曖昧なrefとして失敗した。完全な `refs/heads/...` 指定で修復し、remote tagの新世代5499637、前世代9bd8838を確認してから編集を開始した。今後も完全なref名で区別する。

- item: Flutter共通化・Mobile・端末連携・platform別最終証拠
  classification: release_blocker
  reason: Desktop対話の実装と検証をMobileやinstalled-pathの成立へ昇格しない。
  required_action: 次の共通化、Mobile正式project、端末認証と安全保管、各platformの検証を実施する。
  blocks_release: yes

Desktop単位の最終検証: Windowsの一括14検査はすべてPASS（終了値0）。Linux側の同一差分でも `flutter analyze`、`flutter test`（37件）、`flutter build linux --debug` がPASS。Linux製品Dart client→Rust→実MINIDORAの試験もPASSした。WSLgの `GDK_BACKEND=x11` でLinux開発binaryを起動し、broker接続とX11のIsViewableウィンドウを観測した。既定WaylandのウィンドウをX11検査器で検出できなかったことは、製品regressionとは分類しない。X11指定は試験の環境設定だけであり製品に追加していない。実起動証拠の環境範囲はREADMEのknown_limitationへ記録した。

## Flutter共通表示とMobile platform構成

対話画面・安全な応答検査・通常transport契約を `packages/gui_shell_ui` へ移した。接続は注入し、Desktopの資格読取とTCPはDesktop側へ残した。Dartのみのclient入口とFlutter表示入口を分け、dev-only実API検証がFlutter engineを必要としない構成を保持した。共通5試験とDesktop32試験で元の37試験の責任を保持する。

既存Mobileのlibを保持したまま、Flutter標準生成元からAndroid/iOS projectを追加した。Mobile依存lockfileはGit追跡するが、既存規約どおりMANIFEST対象から除外する。releaseの開発鍵流用は除去した。開発識別子・配布署名・実機証拠はMobile READMEのrelease_blockerとして明記した。

実API回帰の初回はFlutter表示層の推移的importで起動期限を超過した。client入口の分離後、`python tooling/minidora_live_check.py --reference C:/Users/mzcum/codex-work/MINIDORA-reference --dart-client` はPASS。Windows bat親processの終了だけでは子Dartが試験logを保持したため、検証toolはFlutter同梱のDart executableを直接起動するようにした。製品起動経路への回避層追加ではない。

集約検証の初回は16項目中14項目がPASS。Mobile lockfileをMANIFESTへ含めた変更が既存conformanceと衝突し、conformanceとZIP展開後conformanceが失敗した。MANIFESTの既存除外規約を維持する形に戻した。検査の削除・弱体化は行っていない。

- item: Mobile端末連携・安全保管・Android/iOS実機証拠
  classification: release_blocker
  reason: この単位は共有表示とplatform構成。Mobile libは従来の試作画面であり、実状態ではない。
  required_action: 端末資格、Desktopへの暗号化接続、失効、設定、lifecycleを実装して実機検証する。
  blocks_release: yes

修正後の conformance（146件）と packaging portability はPASS。共通・Desktop・Mobile analyze、共通5件・Desktop32件test、Rust53単体・5統合は集約検証でPASSした。

共有化後の lutter build windows --debug もPASS。Android/iOSのnative buildはこの単位では未検証であり、上記platform別release_blockerに含める。

## 端末連携の契約

招待・結合資格・暗号化要求の三Schemaと日本語意味正本を追加した。通常操作のallowlist、Host証明書固定、端末所有関係、招待300秒・結合8時間、nonce再使用拒否、失効・lifecycle・安全保管を定義した。port上限を機械検証するためSchema検証器にmaximum検査を追加した。

`python tooling/schema_check/check_schemas.py` は34 Schema・34正常例・36否定例、`python tooling/conformance_tests/run_conformance_skeleton.py` は147項目でPASS。必須field欠落、未知権限field、不正型、port境界、禁止操作、操作と内容の不一致を含む。証拠classはFIXTUREであり、暗号化や端末認証の実動作を証明しない。

- item: 端末連携の製品消費経路
  classification: release_blocker
  reason: この単位は契約と構造検査。TLS・資格・所有関係・安全保管の製品経路は次単位で実装する。
  required_action: RustとMobileの正常・否定経路および実通信を検証する。
  blocks_release: yes

## Rustの端末TLS経路

明示bind時だけ公開するTLS経路を同じDesktop Rust brokerへ接続した。通常loopbackとowner資格の分離を保持し、端末操作は対話・確認・離脱だけを許可する。招待の消費、秘密hash照合、期限、nonce、Host・端末結合、対話所有関係をRustで強制する。入力は全階層の重複fieldを拒否し、受信上限と有限期限を持つ。owner招待秘密は新規fileへ保存して標準出力へ出さない。

既存Rust53単体・5統合は変更後にPASS。端末状態機械の6否定／正常試験と、永続監査障害・期限処理を実Coreへ通す2試験もPASS。実TLSのdev-only clientによる正常結合、再接続、再使用拒否、不正資格、Host不一致、権限昇格拒否、他端末の対話・要求アクセス拒否、実MINIDORA応答、失効後の保留承認拒否、招待取消、離脱を実行した。製品Desktop Dart clientとの併用もPASS。

新依存はTLS・証明書生成・秘密hash比較に限定し、Cargo.lockへ固定した。通常RuntimeのMINIDORA接続は引き続き既存Adapter経路だけである。TLSの実通信証拠をMobile製品画面やOS安全保管の成立へ昇格しない。

- item: Mobile製品clientと安全保管・lifecycle
  classification: release_blocker
  reason: Rustの端末経路は実動作したが、Mobile libへの接続とAndroid/iOS実機証拠は未完成。
  required_action: MobileでHost照合・安全保管・対話画面・復帰処理を接続し検証する。
  blocks_release: yes

端末失効の反復試験では、当初64回で対話枠が尽きる資源保持を再現した。失効済みsessionを解放し、送信中workerだけは遅延応答のhash監査まで保持する修正を行った。修正後の実TLSによる結合・対話開始・失効70回はすべてPASS。遅延応答の破棄監査と資源解放の回帰試験も追加した。最終Rust検証は63単体・5統合でPASS。

集約検証は14項目PASS、2項目（broker authority parity、cargo test）が起動中のWindows binaryと再buildの競合によるOS error 5で失敗した。実通信processの終了後に順番を分け、両項目を再実行してPASS。製品や検査に回避層は追加していない。修正後の実TLS＋実MINIDORA＋製品Dart clientも再実行してPASSした。

最終監査では、試験内のraw file読取がhelper境界の禁止patternに該当したため、既存の `BrokerPersistentStore` 再読取・chain検証経路で監査記録を確認する試験へ置換した。検出器は変更していない。また高負荷時に既存取消試験の固定150ms待機が不足したため、取消の即時結果と空本文のassertionを保持し、2秒以内の実受信完了を待ってraw保持を検査する形へ修正した。再実行は63単体・5統合でPASS。TLS session再開も無効にし、毎接続の端末資格検査を維持した。


## Mobile製品client・安全保管・lifecycle

MobileのTLS clientは招待のHost・端末ID・証明書hashを固定し、実peer照合と証明書有効期間の確認より前にapplication資格を送らない。重複field・未知field・不正型・期限・private IPv4境界を拒否する。通常操作allowlistと有限通信期限を持ち、owner資格やRuntime直結を導入しない。

Androidの安全保管とiOS Keychainを接続し、書込後の再読取を必須にした。保管失敗時の平文fallbackはない。Android backup・device transferを除外した。復帰時は資格を再確認し、background中の通信と保留入力の自動再送を停止する。失効・不正資格・通信失敗時は入力を止め、正常解除と通信不能時のlocal削除を区別する。

既存6画面を維持し、対話・接続先・設定を追加した。固定previewの準備完了・承認件数を除去した。共有対話画面はinactive時に接続・polling・送信を停止し、再開時に照会する。旧previewのdevice_id / pairing_idと現行端末ID / 結合IDの対応を復旧画面ソースへ記録し、既存conformanceの用語検査を保持した。

`python tooling/minidora_live_check.py --reference C:/Users/mzcum/codex-work/MINIDORA-reference --dart-mobile-client` はPASS。Mobile製品Dart client → 実TLS → Rust Core → 実MINIDORA二processで本文・追跡・owner承認を検証した。異なる証明書の拒否、停止・再確認、失効後の拒否もPASS。OS安全保管と実機lifecycleの証拠には昇格しない。

`flutter analyze` と `flutter test`（Mobile12件）はPASS。初回widget試験は概要とdrawerの同名表示を両方拾って失敗したため、検査対象を実際のNavigationDrawerに限定して再実行した。安全保管失敗、期限・構造・Host不一致、資格確認と復帰の競合、local削除、破損資格の上書き拒否を含むFIXTURE検証である。

Android初回buildはFlutter生成値のGradle heap 8GBでnative memory allocationに失敗した（環境要因）。heap 2GB、metaspace 768MB、worker 2に限定した。次のbuildではflutter_secure_storage 11がSDK 37を要求し、AGP 9.0.1が新しいandroid-37.0を解決できず失敗した（toolchain互換性）。標準のSDK packageが不存在の`platforms;android-37`を要求するsdkmanagerコマンドも失敗した。公式対応表に従いAGP 9.1.1 / Gradle 9.3.1へ更新した。SDK directoryの偽装・pluginソース改変・依存検査の抑止は行わない。

一次資料: [AGP 9.1.1の対応範囲](https://developer.android.com/build/releases/agp-9-1-0-release-notes)、[FlutterのAGP 9移行](https://docs.flutter.dev/release/breaking-changes/migrate-to-built-in-kotlin)。

- item: Android/iOSのOS安全保管・実機install・launch・lifecycle
  classification: release_blocker
  reason: host上のDart実通信とFlutter fixtureは実機での成立を証明しない。ADB接続済み実機はまだ検出されていない。
  required_action: buildを成立させ、実機で保存・再起動・結合・対話・失効・復帰を測定する。
  blocks_release: yes

集約検証は17項目中16項目がPASSし、共有UI analyzeの波括弧lintだけが失敗した。修正後の共有analyzeはPASS。共有6試験・Desktop32試験・Rust63単体と5統合・broker parity・packaging・Schema・conformanceは集約時にPASSした。MobileはHost照合前の送信禁止と保管障害からの復帰試験を追加し、最終analyze・14試験がPASSした。


AndroidのAGP修正後、`flutter build apk --debug` はPASS（初回1778.1秒）。IME学習・自動入力の無効指定を含む最終ソースで再buildし38.5秒、`flutter build appbundle --debug` は42.0秒でPASS。SDK 35とCMake 3.22.1もpluginの標準依存として導入された。Kotlinの生存markerは`.kotlin` cacheとしてignoreし、生成物をcommitしない。

開発APKは175738543 bytes、SHA-256 `e5c5f30108f812d92e444993087c417a812806c2f47f3a445454035743dffdb6`。開発AABは71596894 bytes、SHA-256 `0ce3fdb996e048c98e665c5c770bac8a188a5e39dc3b57b115a395da35764586`。APKはarm64-v8a、armeabi-v7a、x86_64を含む。`apksigner verify --verbose --print-certs` はPASS、Android DebugのRSA 2048 / v2署名であり公開配布署名ではない。`zipalign -c -P 16 4` はPASS。`apkanalyzer manifest print` でmin SDK 24、target SDK 36、debuggable=true、allowBackup=false、fullBackupContent=false、usesCleartextTraffic=false、dataExtractionRulesの実格納を確認した。これは生成物のCONFIG / EXTERNAL_EVIDENCEであり実機の動作証拠ではない。

JDK17の`jarsigner -verify`は終了値0だが、自己署名・timestampなし・POSIX属性・JarFileとJarInputStreamの検証差について警告した。警告を削除するための再梱包は行わない。公式bundletool 1.18.3（公開asset SHA-256 `a099cfa1543f55593bc2ed16a70a7c67fe54b1747bb7301f37fdfd6d91028e29` を照合）の`validate --bundle=...`と`build-apks --bundle=... --mode=universal --output=...`はいずれもPASS。変換したuniversal APKのapksigner検証もPASS。AABのAndroid工具による消費は確認できたが、正式配布・署名・汎用JAR stream検証差は次の配布前確認へ保持する。

## Apple platformの手動補助build

このローカルhostはWindows/WSLであり、ローカルMacはない。owner rev2とAGENTS 3.1に従い、workflow_dispatchだけの補助workflowを追加した。Windowsで検証したFlutter commit `559ffa3f75e7402d65a8def9c28389a9b2e6fe42` とRust 1.95.0を用い、macOS開発app・iOS Simulator app・Rust helperのbuildを対象とする。GitHubの品質必須statusや自動CIは追加しない。対象commitと環境情報、log、tar成果物、hash、buildによるソース差分を収集する。追跡ソースが自動変更された場合は成功とせず、差分を確認する。

- item: Apple補助実行と実機証拠
  classification: release_blocker
  reason: workflowの追加は外部実行の成功やMac/iOS実機の成立を証明しない。
  required_action: 手動実行の対象commit・結果・artifactを確認し、実機install・launch・安全保管・lifecycleは別途測定する。
  blocks_release: yes

workflow追加のlocal検証はSchema（35正常・37否定）、conformance（147項目）、日本語厳格監査がPASS。これは手動起動限定のCONFIG検証であり、外部build結果は手動実行後に記録する。


Apple初回補助実行（run 34445302630、対象52bcbd2cc82519f5f6ebc6c80c8c60f96a1e12af）はRust64単体・5統合およびarm64 Mach-O buildがPASS。macOS project未追加によりFlutter buildがFAILし、iOSは未実行となった。分類はproduct regressionではなく未実装構成の検出であり、この時点のApple buildはrelease_blockerである。固定Flutter標準生成元からmacOS projectを追加し、Sandboxを保持したまま既存broker用network.clientを指定する。正式配布・実機資格配置はDesktop READMEのrelease_blockerへ保持する。


## 複数OS間のmanifest修復

Linux最新checkoutの集約検証ではFlutter/Rust関連検査はPASSしたが、manifest・release gate・梱包の3検査が失敗した。Windows編集時のCRLFをraw hashへ記録し、Gitが既存.gitattributesに従ってLFへ保存したことが原因である。検証toolのhash照合は変更せず、作業fileを既存の改行規約へ戻してmanifestを再生成する。生成時にはGitのeol属性と作業byteの不一致を拒否し、無言の正規化やhash比較の緩和を行わない。Git実repositoryを使いLF、明示CRLF、binary、混在改行、修復後の正常化を検証する。


## 現時点の要求監査（2026-09-10）

rev2全体は未完了。実装済みの対話Core・MINIDORA Adapter・Desktop比較・Mobile端末連携と、実機で未確認の範囲を分離する。完成製品releaseとowner GOは主張しない。

|要求|確認した経路・結果|証拠の限界|
|---|---|---|
|統治・日本語意味正本・Schema|手動Actions限定、対話・比較・端末連携の正本、35 Schema、35正常・37否定fixture PASS|CONFIG/FIXTURE|
|CoreとMINIDORA Adapter|Rustの要求・承認・監査・取消・隔離試験、固定参照commit 3400a3bb68b37efa1dc14ee8aaa28fda779bf1f8の実API PASS|基本会話と保留の範囲。基礎Core能力の保証ではない|
|対話・比較|共有6件、Desktop32件、Mobile14件、左右成功・片側失敗・両失敗・session/権限非混線の試験 PASS|画面fixtureとhost上の実通信を区別|
|端末連携|製品Dart TLS client、招待・失効・replay・Host・権限否定、実MINIDORA PASS|OS安全保管は実機未確認|
|Android版|analyze/test、APK/AAB、署名・alignment・bundletool PASS|実機install以降はrelease_blocker|
|Apple版|手動実行34446194013、対象`27b8713fd1a9ecdb81abe1d4225b99b26bda84ba`でRust単体64・統合5試験、macOS開発app・iOS Simulator appのビルドに合格|実機launch・Keychain・対話はrelease_blocker|
|Windows版|共有化・端末連携後の集約検査でlint修正後PASS、最新debug build PASS|2026-09-11に画面回帰を再開し下記の範囲で確認。installed-path証拠はrelease_blocker|
|Linux版|最新対話実装のFlutter build、各analyze/test、Rust試験、Desktop/Mobile製品Dart clientと実MINIDORA PASS|最新release起動とPID一致の可視window・broker監査も確認。WSLg X11の範囲|
|manifest・梱包|改行修復後のWindows/Linux Python系9検査 PASS。conformance148件|開発検証のpassはstrict releaseのpassではない|

Apple成果物は外部artifact `10139803376`（86677901 bytes）を取得しZIP SHA-256 `3110e6493467eef9c38dc371746655dbbbb7efeb89696ba03421c4916752a9ac` を照合した。内部tar SHA-256 `59496c1c87f3b36f4bb3d7f592457df3315af3099155066f24b0200495041cca` も一致し、両app・Rust実行file・iOS安全保管plugin資産を確認した。環境はmacOS15.7.9 arm64、Xcode16.4、iOS Simulator SDK18.5。追跡差分patchは空。Flutterが生成したmacOS registrantは既存Windows/Linuxと同じく追跡対象へ追加する。生成内容を手編集せず、依存の生成元はpubspecである。

artifact取得の初回HTTP直取得はredirect先で401となり、認証headerを別hostへ引き継がない取得で回復した。ログ表示のcp932 UnicodeEncodeErrorはPYTHONUTF8=1で回復した。いずれも製品build失敗ではない。

Windowsの画面回帰は当初Escで中断したが、2026-09-11のowner再開指示後に下記の開発回帰を実測した。installed-pathの独立したrelease blockerは保持する。

- item: 実機・installed-path・正式配布・owner GO
  classification: release_blocker
  reason: Android実機検証はowner指示で凍結中。Mac/iOS実機なし。Windows installed-path全体の証拠と正式配布署名も未成立。
  required_action: Androidはownerの再開指示を待つ。それ以外の必要な実機証拠・配布指定とstrict validationは独立して扱う。
  blocks_release: yes


## Windows隔離配置・Linux最新起動の追加証拠

対象ソースはcleanな `47299ce839f51f0bcb39cd3d19d98f69f1510003`。Windowsの `cargo build --release --locked --manifest-path native/rust_helper/Cargo.toml` は95秒、`flutter build windows --release` は92.4秒でPASS。`installer/windows/stage_installed_app.ps1` で新規run `rev2-47299ce-20260910` を作成し、source_worktree_clean=trueと生成manifestの実artifact hash一致を確認した。Flutter exe SHA-256は `e671cd41eeb1c1d9c147178f5a4ae5a53052ac375807f421c9bc31e49769f7e6`、Rust exeは `d2015557a120f6552aceb518fc6e30fce36c744b4b384e19bd97161548dbbf07`。

この配置に対する `installer/windows/collect_broker_smoke.ps1` はstatus=passed、errors=[]。制限loopback、認証付きIPC、永続store、再起動後のnonce再使用拒否、新規要求の受理、強制終了後の接続拒否を実測した。証拠は `%LOCALAPPDATA%/GUI-Shell/installed-runs/rev2-47299ce-20260910/runtime/evidence/windows_broker_smoke.json`。画面・Setup Doctor・監査アンカー保護を測定していないため、Windows installed製品全体のgateは解除しない。

Linuxは同じソース系統のrelease binary（SHA-256 `6582fd0acc0b94f0a1c09239af5626c1ada4180150f1ff00a9d25a7bc47268c8`）を実MINIDORA二processとRust brokerに接続して起動した。`GDK_BACKEND=x11` のWSLg環境で、`xwininfo -root -tree`、対象の`xprop -id <観測ID> _NET_WM_PID`、`xwininfo -id <観測ID>` により起動PID411との一致とIsViewableを観測した。app終了前に生存も確認し、検証後に自身のprocessを終了した。

永続監査にはflutter-request-1から5が記録された。health、normalize_payload、content_projection、approval_editの受理と、command_envelopeのsuspendedを確認した。これは起動時の既存broker経路の証拠であり、画面からの対話入力・表示内容全体の証拠ではない。Desktop/Mobile製品Dart clientから実MINIDORAまでの経路は別の実通信検証でPASSしている。DRI3 deviceを取得できないlibEGL警告は出たが起動は成立した。描画性能・Wayland・物理Linux端末への同等性は主張しない。


## Windows画面回帰の再開・Android実機検証の凍結（2026-09-11）

ownerの明示指示でWindows画面検証を再開した。対象は `d5341f96f207b57085453af4797536abe125a5ed` の実装を持つWindows debug app、Rust broker、固定参照MINIDORA二process。Computer Useによる可視画面の操作・観測であり、検証入力は「こんにちは」だけとした。設定・権限の変更は行っていない。

概要、環境診断、信頼、実行系、権限、agent、承認、監査、復旧、問題、証拠、設定、対話の13画面へ移動し、表示を確認した。既存画面から対話へ戻った際も入力と左右のsession状態を保持した。監査画面の既存projectionを新しい対話監査の表示証拠へ読み替えていない。

二実行系比較で同一入力を送信し、左右の異なるsession・要求IDと承認待ちを観測した。検証用owner CLIの `対話承認操作 --session-file <検証用owner file> 承認 <要求ID> <要求hash> full` で各要求を承認した後、双方の完了、成功、full、基本会話の応答、別々のtrace/hashを画面で確認した。通常UIへowner資格は渡していない。

次の要求では左を中止しても右の承認待ちが維持され、右も個別に中止できた。左だけ新規sessionへ切り替え、右の中止済みsessionを残して再送すると、左は承認待ち、右は送信失敗と自動再送しない旨を表示した。最後に左も中止した。検証用brokerの永続監査でも対話送信・承認・取得・中止を確認し、検証後は自身が起動したprocessを終了した。これはLIVE_RUNTIMEの開発経路証拠であり、installed製品全体の証拠ではない。

操作上、下方へscrollした位置で座標指定の送信clickに反応を観測できない試行があった。入力欄からTab・Enterで送信でき、その後は上方に表示した送信buttonのmouse clickでも要求発行を確認した。座標操作と製品側のどちらが原因かは確定していない。全画面サイズ・全入力装置の成立は主張しない。初回window列挙のtimeoutは待機後の再取得で回復した。

Androidは実機検証だけを凍結し、ownerの再開指示まで端末接続要求、install、launch、結合、対話、安全保管、lifecycleの実機試験を行わない。APK/AABと既存build結果を保持する。iOSは凍結対象外。凍結を合格やrelease scopeの削除へ置き換えない。

- item: Windows全表示条件とinstalled製品証拠
  classification: release_blocker
  reason: 今回の開発画面観測は、全入力・表示条件やinstalled-path、Setup Doctor、外部監査アンカーの証拠を満たさない。scroll後の座標click不成立の原因も未確定。
  required_action: installed製品の検証時に入力位置と反応を再現確認し、既存の機械検証可能なrelease evidenceを収集する。
  blocks_release: yes

この文書更新の初回release gate検査は、解決済み項目のblocks_releaseをfalseへ変更した台帳記述を拒否した。既存contractでは分類属性をtrueのまま保持し、status=resolved / active=falseで解決を表すため、台帳だけを修正した。検査器は変更していない。


## インストール済みSetup Doctorの製品出力（2026-09-11）

既存隔離配置 `rev2-47299ce-20260910` のrelease Flutter appとRust brokerを、新規runtime領域 `runtime/setup-doctor-20260911` で実行した。app SHA-256は `e671cd41eeb1c1d9c147178f5a4ae5a53052ac375807f421c9bc31e49769f7e6`、brokerは `d2015557a120f6552aceb518fc6e30fce36c744b4b384e19bd97161548dbbf07` で、以前の隔離配置の値と一致した。

実行commandはrepository外の `python ../GUI-Shell-installed-export-check.py`。この補助は既存のbroker-serverを起動し、実測path・hashを製品出力contextへ渡し、既存の `GUI_SHELL_SETUP_DOCTOR_CONTEXT_JSON` / `GUI_SHELL_SETUP_DOCTOR_EXPORT_JSON` 経路を起動するだけで、diagnostic checkを生成・改変しない。collector自体はPythonを使用するが、起動したappのPATHはWindowsとSystem32に限定した。この環境指定だけをPython非依存の完全な証拠には扱わない。製品経路の置換やUI操作の代替ではなく、既存製品出力に限定した開発用測定である。統合collectorの正式証拠へ接続した時点で補助測定は不要になる。

製品が書き出した `product.json` はSHA-256 `903e46b4f574e1b5768eb5e32be922b780eb3a175ada777777adf445f727c501`。`tooling.windows_release_evidence.validate_setup_doctor` へそのまま渡し、windows_setup_doctor_smoke=passedを得た。製品内の10項目はすべてpassであり、設定生成・監査領域書込み・認証付きbrokerと永続化の接続を含む。出力時のapp PID4904の生存を確認し、検証終了時に自身のappとbrokerを終了した。証拠は `%LOCALAPPDATA%/GUI-Shell/installed-runs/rev2-47299ce-20260910/runtime/setup-doctor-20260911/result.json` と同directoryの製品出力である。

- item: Windows統合release evidence
  classification: release_blocker
  reason: Setup Doctor単独の製品出力は得たが、可視画面、全体provenance、監査アンカー保護を含むcanonicalなwindows_installed_smoke.jsonは未成立。製品context由来のCONFIGとbroker実観測を全体保証へ昇格しない。
  required_action: 同一隔離runに結び付く統合証拠を収集してWindows release evidence全項目を検証する。
  blocks_release: yes


## 監査アンカー収集器の誤った保護成立判定を修正（2026-09-11）

実installed storeに対し `collect_audit_anchor_proof.ps1` がpassed / key_anchor_log_same_user_rewrite_mitigated=trueを返した。しかし同じユーザーでaudit_anchor.key、audit_anchor.json、audit.jsonlのすべてをFileMode.Open / FileAccess.Writeで開けた。byteは書いていない。広範な主体へのwrite ACEがないことを、所有者自身による一括書換え防護へ誤って昇格する既存不具合だった。

収集器はrelease用診断経路に限定して修正した。DPAPI固定文字列の往復はdpapi_availableへ区別し、監査鍵保護を表すdpapi_verifiedを成立させない。外部fileの存在/hashと任意fileのAuthenticode検証も対象chain・独立保管・信頼済み署名者の結合を証明しないため、外部アンカーや署名済み監査証拠の合格へ昇格しない。現行実装に同一ユーザーの書換えを防ぐ独立境界の検証はないため、保護成立を主張せずfailedを返す。runtimeの鍵・ACL・権限は変更していない。

`python -m unittest tooling.conformance_tests.test_windows_anchor_collector` は実PowerShell collectorを3ケース（書込可能なstore、無関係な外部file、署名file）で実行しPASS。Windows conformanceの既存collector接続検査にも組み込んだ。初回はWindows PowerShell 5.1と継承module環境の不一致によりGet-FileHash等の読込みが失敗し、利用可能なPowerShell 7を優先する試験起動へ修正した。製品検査を弱めていない。

同一の実installed storeで修正後に再実行しstatus=failed、same_user_rewrite_mitigated=false、dpapi_verified=false、dpapi_available=trueを観測した。`runtime/setup-doctor-20260911/anchor-before.json` と `anchor-after.json` に比較証拠がある。旧passedはreleaseの根拠に使わない。

- item: 監査アンカーの同一ユーザー書換え防護
  classification: release_blocker
  reason: 誤判定は修正したが、独立した保管・信頼基点・chain結合の実装と証拠は未成立。
  required_action: 独立境界と巻戻し・置換・改変の検出経路を定義して実装・実測する。
  blocks_release: yes


## 旧アンカー合格記録のrelease受入れを拒否（2026-09-11）

前項の収集器修正だけでは、修正前の `anchor-before.json` をWindows release検証器へ渡すとwindows audit anchor gateがpassedとなった。実測recordを検証用provenanceへ組み込んだ検証器単位の再現であり、統合releaseの成功を示すものではない。

`validate_audit_anchor_external_tamper_evidence` は、現行形式のcollector自己申告だけでは対象chainと独立した信頼基点の結合を検証できないことを明示し、release_blockerを返すよう修正した。同じ実測recordを再投入するとfailedへ変わった。external_anchor / signed_evidenceへsource_kindを付け替えてverified=trueとする3種類のfixtureも拒否し、他のWindows gateの正常fixtureは合格を維持した。元の全項目合格fixtureはこの未検証の保証を誤って正常扱いしていたため、他の正常経路の成立とアンカーの拒否を別々にassertする試験へ修正した。

- item: 独立したアンカー証拠の受理経路
  classification: release_blocker
  reason: 現行形式に信頼済み署名者・対象chain・置換や巻戻しを検証する消費経路がない。過去のpassedも保護を証明しない。
  required_action: 独立した信頼基点と保管先の境界を決め、chainへ結合した証拠を実際に検証する経路と否定試験を実装する。
  blocks_release: yes


## owner確定方式のオフライン署名checkpoint（2026-09-11）

ownerが指定したEd25519オフライン署名方式を、Rustのrelease専用CLI・Schema・Collector・release再検証へ接続した。固定順序canonical checkpointは監査head、log/anchorのraw SHA-256、source commit、実artifact hash、時刻、sequence、前署名checkpoint hash、versionを署名対象にする。Repositoryの公開鍵fingerprintが未固定なら拒否する。通常brokerのIPC・runtimeから到達せず、秘密鍵の読取り・署名APIを追加していない。

継続性記録を証拠から自己採用しない。owner管理の最新sequence/hashを別に与え、直前署名の検証と連続性・同番号置換・後退を検査する。検証対象と継続性記録の両方を巻き戻す場合の限界、ownerの意図的再署名・物理侵害の対象外、administrator_root_resistance_claimed=falseを正本へ明記した。

Rust試験はメモリ上の使い捨て鍵を用い、正常署名、不正署名、別鍵、未固定鍵、checkpoint byte改変・非canonical・重複field、log/anchor/head/source/artifactの不一致、時刻、previous不一致、巻戻し・同sequence別署名を検証した。実fileの変更と、実Rust CLI → PowerShell Collector → Python release consumer → Rust再検証も検証し、収集後のartifact改変で拒否した。これらはFIXTUREの実実行でありowner署名証拠ではない。実秘密鍵は生成・保存・読取りしていない。

最初の編集commandでUTF-8 fileをcp932として読もうとして失敗した。書込み前の失敗で、明示UTF-8で再実行した。追加暗号依存は既存rustls依存でも使用しているring 0.17.14を直接参照しただけで、独自暗号方式を追加していない。

- item: 実ownerの公開鍵固定と署名済み証拠
  classification: release_blocker
  reason: 実装と試験用署名経路は成立したが、実運用fingerprintはnullであり、実owner署名・外部媒体の継続性記録は未取得。
  required_action: docs/OFFLINE_SIGNING_OWNER.mdのownerローカル操作を実施し、公開鍵・署名証拠だけを取得して実配置で検証する。
  blocks_release: yes

この単位の初回集約検証は配布パス検査だけがFAILした。新しい公開鍵固定fileの日本語pathが既存ASCII配布規約に合わなかったため、機械読取りpathをconfig/audit_signing_trust.jsonへ変更し、日本語意味正本と責任索引を保持した。検査allowlistは拡大していない。修正後の署名6試験（Rust CLI・Collector・release再検証を含む）はPASSし、ringで署名したcheckpointをOpenSSL標準検証でも受理することを確認した。

最終の `python tooling/validate_all.py --desktop-platform windows --include-mobile-release` は開発検証17項目すべてPASS。Rust63単体・5 IPC・6 checkpoint試験、共有6・Desktop32・Mobile14試験を含む。これは実owner署名とWindows統合release証拠の合格ではなく、owner手動操作の前で停止する。


## 実運用鍵を延期してWindows統合収集を検証（2026-09-11）

owner指示により、監査アンカーの実運用鍵・実署名は正式release直前まで外部条件待ちとする。Android実機検証の凍結も維持する。他の実装・検証は継続する。

source `7a4afd838aed44489c71d560dcd6a876b53ce9dd` をLinuxへfast-forwardし、`CARGO_TARGET_DIR=/home/mzcum/.cache/gui-shell-rev2-target python3 tooling/validate_all.py --desktop-platform linux --include-mobile-release` は18項目PASS（Rust64単体、5 IPC、5 checkpointを含む）。Windowsは `flutter build windows --release` と `cargo build --locked --release` から `rev2-7a4afd8-20260911` へ分離配置した。実broker検証とSetup Doctor製品出力10項目はPASS。Computer Useで配置先windowの概要・環境診断への遷移とscrollを観測した。

同配置で `collect_installed_smoke.ps1 -NoPythonRuntime` の `-VisibleSurfacesJson` に、その起動で製品が生成するsurface_semantics_export.jsonを指定した。UIAutomationは実行していない。旧検証器はWindows4関門をPASSとしたが、出力の生成元を確認すると `SurfaceSemanticsRegistry` はbuild時の名前を蓄積するだけで、現在の描画・可視性・破棄を観測していなかった。したがって、この初回起動PASSを完成証拠には採用しない。

収集器はこの既知のbuild registry形式をINTERNAL_STATEとして保存し、可視surface・初回起動を合格へ昇格しない。release検証器も旧collectorのpassedと、source名だけを変更した同形式を拒否する。既存の受入れ試験はこの誤った保証を正常扱いしていたため、拒否の回帰試験に置き換えた。初回の試験編集では別の不足fieldによる拒否を拾っていたため、元fixtureのfieldを保持して再実行し、修正前FAIL・修正後PASSを確認した。

実collector再実行は `runtime/registry-rejection/windows_installed_smoke.json` に保存した。first_run=failed、visible_surfacesのevidence_class=INTERNAL_STATE、formal_release_input=falseを観測。旧記録の再投入もfirst-run=failedとなり、両記録でSetup DoctorとbrokerはPASSを維持した。ログ・実配置・証拠・秘密をRepositoryへstageしていない。

- item: Windows初回起動の可視surface証拠
  classification: release_blocker
  reason: build registryによる誤受理は修正したが、現windowの個別surfaceを外部から確認する厳格な統合証拠は未成立。
  required_action: 実描画・可視性を観測する経路を初回起動と結合し、非表示・破棄・別起動の負例も検証する。
  blocks_release: yes


## 可視性を捏造しない製品診断出力（2026-09-11）

固定Flutter 3.44.0のWindows engineソース `flutter_window.cc::OnGetObject` を確認した。UIAutomation応答は `FLUTTER_ENGINE_USE_UIA` のコンパイル条件内で、MSAA応答は別経路にある。公式の背景説明は [Flutter issue 114547](https://github.com/flutter/flutter/issues/114547) にある。これは個別widgetを取得できない既観測と整合するが、配布済みDLLのコンパイル条件や外部tool側の挙動まで確認した証拠ではない。独自engineへの差替えは行っていない。

前単位で受入れ側を修正したbuild registry出力について、生成元にも残っていた架空の座標、is_offscreen=false、node ID、可視surfaceの合格を除去した。登録履歴はregistered_surfaces / registered_identifiersへ保存し、起動PID、INTERNAL_STATE、visibility_measured=false、formal_release_input=falseを明示する。旧source / diagnostic mode識別子を維持し、既存collectorとrelease検証器で診断資料として扱い、可視証拠への昇格を拒否する。通常UIの描画・Semantics識別子・権限経路は変更しない。

非表示Offstageのwidgetを実際にbuildした試験で、旧出力が必須4surfaceをすべてvisible_surfacesへ入れることを再現した。修正後は非表示時と破棄後の双方でvisible_surfaces・surface_matches・観測nodeが空となり、登録履歴だけを残す。Desktop全33試験とanalyze、Schema36/正常36/負例38、conformance148がPASS。

- item: 外部から観測したWindows個別surface
  classification: release_blocker
  reason: 診断出力の虚偽の可視性表現は除去したが、実画面の外部観測を厳格な初回起動証拠へ結合する経路は未成立。
  required_action: 固定toolchainの対応アクセシビリティ経路と利用可能な観測toolの接続を確認し、現在window・個別surface・非表示の負例を実測する。
  blocks_release: yes


Windows releaseビルド後、実broker経由でappを起動し、生成されたsurface.jsonのprocess_idが起動PID 14112と一致することを確認した。登録履歴は存在するが可視surface・match・観測nodeは空で、INTERNAL_STATE / visibility_measured=false / formal_release_input=falseを確認した。実測は `%TEMP%/gui-shell-build-registry-zaynljnf/result.json` に保存し、起動したappとbrokerは終了した。Mobile analyzeもPASS。これは製品診断出力の実行証拠であり、外部可視性の証明ではない。

このビルドでrunner exeのSHA-256は既存配置と同じ `e671cd41eeb1c1d9c147178f5a4ae5a53052ac375807f421c9bc31e49769f7e6` だった。一方、Dart AOTのdata/app.soは旧配置 `1c8776ff3a88b4af1bbaf6b9902a7231053be36da6d6c1f8eafe0f1fc0da513b` から `03590c9ea67be5fc603c1bcb32e1e7a7a5a4e3f1bcf5cf3440f786753f4af157` へ変化した。native/rust_helper/src/checkpoint.rsのmeasureはartifact引数の単一fileだけをhashし、現在のWindows collectorはapp exeを渡している。source commitの一致だけでは配置後のapp.so改変を検出しない。

- item: 監査checkpointの配置成果物hash範囲
  classification: release_blocker
  reason: 現行のexe単体hashではDart AOT、engine、plugin等の実行配布物の置換を検出できない。
  required_action: 配置app/brokerの実行配布物一式をcanonical manifest等へ結合し、欠落・追加・置換・path改変の負例を含めてcheckpointとrelease再検証へ接続する。
  blocks_release: yes


## checkpointをWindows配布物一式へ結合（2026-09-11）

checkpoint version 2へ更新し、installed_artifact_sha256をRustがinstalled rootから実測するcanonical配布物一覧のhashへ変更した。app/broker以下の全file・directoryと両launcherを含み、path・種別・size・内容hashを結合する。runtimeとinstalled_manifest.json以外のroot追加、必須成果物欠落、規約外path、symbolic link/junction/reparse point、特殊file、大小文字衝突を拒否する。上限付きで二度走査し、変化すれば失敗する。旧exe単体署名version 1は拒否する。

Schemaと正本を先に更新し、prepare / verifyの引数をinstalled rootへ変更、artifact-manifest CLIを追加した。CollectorとPython release再検証も同じrootを渡し、version 2の実検証結果のみ受理する。通常runtimeの経路や新しい依存は増やしていない。ownerの実運用鍵・署名は正式release直前まで外部条件待ちのまま。

Rustは63単体・5 IPC・8 checkpoint試験PASS。既存の署名・chain・sequence・source不一致に加え、AOT/engine/broker/launcherの改変・削除、追加file、rename、root追加、junction、規約外path、旧version拒否を確認した。実CLI → OpenSSL公開鍵検証 → PowerShell Collector → Python release consumer → Rust再検証で、runnerを変えずにAOTだけを改変すると拒否する。秘密の試験鍵はprocessメモリ内だけに生成し、実運用鍵を扱っていない。初回は試験内のJSON型代入がcompile errorとなり修正した。junction試験のmklink引数にforward slashが残っていたためInvalid switchとなり、Windows path componentの組立てを修正して全試験を再実行した。

実配置 `rev2-7a4afd8-20260911` の15 file / 21 entryをRustで測定し、Pythonで独立に列挙したpath・size・SHA-256と全件一致した。別の検証用コピーでapp.soだけを改変し、runner hashが変わらないまま成果物一覧hashが `027f460e382cf4913d701d9d605a108111db89cbef00dc1c038cdced2de72238` から `308feef71c18b8c4c3c52be4ebbebdfa2ca6b94eee9827e261cdb672a12457d7` へ変化した。実測は `%TEMP%/gui-shell-artifact-actual-tafn4o16/runtime/result.json` とbefore/after一覧へ保存した。これは未署名の実配布物測定であり、owner署名証拠ではない。

Schema36/正常36/負例38、conformance148もPASS。配布物の変更を止めた検証時点のsnapshotを扱い、原子的なfilesystem snapshotや検証後の実行時差替え防止は主張しない。正本にその境界を明記した。

- item: 実運用署名済み配布物checkpoint
  classification: release_blocker
  reason: 配布物一式を結合する実装・負例は検証したが、owner公開鍵固定と実署名は延期中。
  required_action: 正式release直前にclean commitから配置し、外部媒体上のowner鍵でversion 2を署名して現配置を再検証する。
  blocks_release: yes


## 現在commitの集約回帰とfilesystem境界試験（2026-09-11）

source `0cdbab7a13629cc6d96618160883286e7a718336` で `python tooling/validate_all.py --desktop-platform windows --include-mobile-release` の開発検証17項目、Linuxで同commandのplatform=linuxを指定した18項目が全PASSした。Windowsの正式統合証拠がRepository既定pathに存在しないことによる個別release blockerは残る。開発モードのrelease_gate: passを厳格releaseの合格と扱わない。ログはRepository外のGUI-Shell-0cdbab7-windows-validation.txt / GUI-Shell-0cdbab7-linux-validation.txtへ保存した。

配布物のpath試験にあった「UnixならCASE/caseを別entryとして作れる」という前提を除去した。実directoryを列挙して2 entryなら大小文字衝突を拒否、1 entryなら一つだけを測定し最後に書いたbyteのhashと一致することを検査する。OSによる試験除外も不要とした。production codeと拒否要件は変更しない。変更後のcheckpoint試験はWindows8件・Linux7件PASS、Schema36/正常36/負例38・conformance148もPASS。


## WindowsのSemantics起動順序を修正（2026-09-11）

DesktopのmainでOS要求より先に保持していたSemanticsHandleを除去し、Flutter標準のplatform lifecycleに有効化を任せた。独自engine、UIAutomation wrapper、依存追加はない。固定Flutter 3.44.0のbindingとWindows engineを調べると、手動handleがある場合は後続のOS要求でDart側の有効状態が変化しない。この順序が初回treeの欠落に関係するという仮説で、同一broker起動ハーネスとreleaseビルドを比較した。engine内部の通知順序をtraceしたわけではないため、詳細な因果経路は推論として保持する。

変更前のPID15840 / window3999218では外部アクセシビリティ照会にwindow枠だけが現れた。8行の起動処理を除去した比較版PID6884 / window1770286では、初回に108要素を観測し、概要、ナビゲーション、実行系状態、不変条件の個別groupと内容を取得できた。診断へのクリック後は環境診断、broker IPC、保護項目拒否を含むtreeへ切り替わり、概要へ戻ると実行系・不変条件の内容が再出現した。再描画で要素番号が失効したクリック2回は未成立として扱い、画面を再取得して座標操作後に遷移完了を再観測した。先行文書のUIAコンパイル条件は今回の原因を確定する証拠ではなく、この標準engineでも個別widgetを取得できることが新たな観測である。

観測はComputer Useの外部treeであり、製品build registryから合成していない。Repository外のGUI-Shell-semantics-external-observation.jsonに診断と概要復帰の実treeを保存した。ハーネスの診断出力は引き続きINTERNAL_STATE、可視surface空、formal_release_input=falseを確認した。署名鍵は使用していない。

`flutter build windows --release` とDesktopの `flutter test --reporter compact`（33件）、Desktop/Mobileの `flutter analyze`、Schema36/正常36/負例38、conformance148はPASS。厳格なinstalled collectorの再収集をこの開発用起動で代替しない。

- item: 現行配置からの厳格なWindows可視surface証拠
  classification: release_blocker
  reason: 外部tree取得と画面切替を実測したが、clean sourceと分離配置に結合した正式collectorの証拠は未成立。
  required_action: 現行commitを分離配置し、個別surfaceの実観測と非表示の負例を厳格な収集・検証経路で確認する。
  blocks_release: yes


## 可視surfaceの宣言と実観測の結合（2026-09-11）

clean source 53182cd4cab77e025c61913357cf527270f4dd11をrev2-53182cd-20260911へ分離配置した。Windows releaseビルドとRust releaseビルド、配置先broker smokeは成功し、正式consumerではSetup Doctorとbrokerの2関門がPASS。初回起動と可視証拠provenanceはFAIL、実運用署名は延期によるFAILを維持した。実証拠とvalidation-result.jsonは配置先runtime/evidenceに保存している。

標準MSAA APIの実測ではwidgetの名前・役割・座標を取得できたが、画面外要素も非表示bitなしで返った。既存collectorは名前だけで候補を選び、consumerはmatchを観測treeへ結合せず、診断keyも先頭20件しか調べていなかった。正常fixtureも4surface宣言に対しrootと1surfaceしか記録していなかった。今回の単位はbuild/release経路のこの誤受理を修正する。

正本を更新し、全観測要素の識別子・root・親子edge・件数を検証する。matchは同じelement_keyの実観測属性と一致し、root/containerでないことを要求する。要素からrootまで明示的なis_offscreen=falseと有限・正の矩形を要求し、共通領域が正面積の候補だけを可視とする。collectorも同じ幾何条件で候補を選び、状態取得失敗はnullとして保持する。親列挙をRawViewWalkerへ揃えた。

17負例は変更前すべて誤受理、変更後すべて拒否した。過大整数座標の例外化も拒否する試験を追加し、負例は計18件。画面外、親領域外、非表示、状態・座標欠落、ゼロ面積、非有限座標、親欠落・循環、識別子重複、match差替え、未観測match、件数・edge不一致、container流用を含む。実collectorの純粋判定関数も正常と11負例で試験した。Windows PowerShell 5.1ではUTF-8 BOMなしの日本語scriptを既定encodingで読むと構文エラーになるため、試験入口とAST読込みでUTF-8を明示した。production用wrapperや新依存は追加していない。Schema36/正常36/負例38、conformance149件、日本語基底監査はPASS。

これは収集済みtree上の表示領域交差の検証であり、別windowによる遮蔽やpixel内容の証明ではない。MSAA実測を正式collectorへ接続する作業は、この条件を省略せず続行する。

- item: MSAA実測と正式Windows収集経路の接続
  classification: release_blocker
  reason: UIAutomationの現経路ではwindow枠しか取得できず、MSAAは診断実測のみ。今回の可視性負例はFIXTUREであり、正式installed初回起動の合格ではない。
  required_action: MSAAの要素同一性・親子関係・実矩形と日本語surfaceを正式収集へ結合し、実画面と負例を再検証する。
  blocks_release: yes


## 日本語surfaceを既存UIAutomation経路で実測（2026-09-11）

前単位ではMSAA接続が必要と判断したが、新しい診断runでは日本語label対応を追加した既存UIAutomation経路だけで108要素を取得した。別起動とMSAA追加コード除去後も計3 runで必須4surfaceと観測tree・矩形交差の検証が通った。前のwindow枠2要素との時間・観測経路による差の詳細は未確定であり、日本語対応がOS側tree公開自体を変えたとは主張しない。MSAAの独自collectorは未使用で、必要性を実証できないため採用せず除去した。新言語・依存・実行経路は追加しない。

修正はcollectorとconsumerの意味対応に限る。概要、ナビゲーション、実行系状態、不変条件状態、および既存Dart安定identifierを固定し、日本語名は空白正規化後の完全一致または見出し二重連結だけを許可する。元の観測名、UIA element key、runtime ID、親子関係、座標を保持する。説明文やTab名、identifierの余分なsuffixは拒否する。

診断実測はrev2-53182cd-20260911/runtime内のmsaa-check、uia-japanese-repeat、uia-only-finalへ保存した。すべてsource=uiautomationであり、製品registryでもMSAAの合成出力でもない。診断専用のため正式初回起動の合格と扱わない。Schema36/正常36/負例38、conformance150件はPASS。MSAA接続そのものを残作業とした直前の判断は撤回し、現行commitのcleanな分離配置からの正式再収集を次の境界とする。

- item: 日本語surface対応後の正式Windows再収集
  classification: release_blocker
  reason: 現行collectorの個別surface実測は成功したが、上記は変更中の診断専用runであり、現行clean commitからの正式統合runは別に必要。
  required_action: commit後に分離配置して同collectorを通常モードで実行し、署名を除く各関門の実結果を確認する。
  blocks_release: yes


## Flutter診断snapshot経路から直接filesystem読込を除去（2026-09-25）

`ShellCoreClient.local()`が環境変数、`LOCALAPPDATA`、相対pathを探索してJSONを同期読込していた。現行製品entryは`ShellCoreClient.product()`だが、この診断経路もFlutterのfilesystem境界に反するため、path探索・file readを削除した。local clientは呼出側から明示注入された型付き`ShellSnapshot`だけを表示し、未注入時は固定fallbackを使う。snapshot source/pathをメモリ診断値へ固定し、鮮度をunknown、完了release claimをfalse、release stateを`not claimed`へ強制する。`SetupDoctor`の表示も実pathを前提としない「snapshot参照」へ変更した。製品状態は引き続きBroker経由であり、この変更はBroker権限・実測の証拠ではない。

local clientの正常注入、release claim偽装入力の抑止、未注入fallback、Setup Doctor表示をDesktop testで検証した。JSON Schemaは変更していない。Python snapshot generatorは開発・移行検証資料として残すが、Flutterからは読まない。過去のrev1／rev2進捗記述は履歴として書き換えず、現行説明とmappingだけを更新した。

検証結果:

- `dart format lib/models/generated_contracts.dart lib/screens/setup_doctor.dart lib/services/shell_core_client.dart test/widget_test.dart`：整形完了。
- `flutter analyze`（`apps/desktop_flutter`）：Desktopの静的解析完了。
- `flutter test --reporter compact`（`apps/desktop_flutter`）：104件すべて成功。
- `flutter analyze`（`apps/mobile_flutter`）：Mobileの静的解析完了。
- `flutter build windows --release`（`apps/desktop_flutter`）：Windows向け製品構成の生成完了。インストール済み製品の起動実証ではない。
- `python tooling/schema_check/check_schemas.py`：Schema 122件、正常例122件、拒否例147件すべて適合。
- `python tooling/conformance_tests/run_conformance_skeleton.py`：適合検査192件すべて成功。
- `python tooling/release_runtime_assertions.py --check`：検査12件すべて成功。証拠区分はCONFIG／FIXTUREのみ。
- `python -X utf8 tooling/日本語基底監査.py --strict`: FAIL。既存baselineの3 files／19 findings（`docs/specs/windows-desktop-launcher.md` 1、`native/rust_helper/src/broker/ipc_server.rs` 12、`native/rust_helper/src/desktop_launcher.rs` 6）であり、この変更の新規findingは確認されなかった。

今回確認したのは静的境界、model projection、test/buildの範囲であり、live runtime、installed Windows実行、正式release証拠ではない。既存の`release_blockers.registry.json`項目`rev2_flutter_broker_channel_boundary`は継続する。Setup Doctor exportには別のFlutter filesystem利用が残り、別単位で監査・移譲する。


## Flutter内Surface Semantics file出力を撤去（2026-09-25）

Flutter起動後にSemanticsのbuild登録表をJSON fileへ書き出す経路を削除した。この出力は`INTERNAL_STATE`で、可視surfaceの正式release入力ではなく、release validatorも受理しない。画面内の安定Semantics identifierと登録表のunit testは残し、可視性判定はWindows collectorの外部UIAutomation treeだけを使う。collectorは登録表fileへfallbackせず、外部観測が不足すれば不足のまま扱う。旧形式のregistryを明示入力された場合に拒否するvalidatorは維持した。

この変更ではOwner権限・新IPC・新Bridgeを追加しない。正式Setup Doctor product exportと初回設定／audit directoryのFlutter直接file accessは未解決のままで、既存`rev2_flutter_broker_channel_boundary`の`release_blocker`に残す。古い進捗記録は当時の状態として保持する。

検証結果:

- `flutter analyze`（`apps/desktop_flutter`）：成功、指摘0件。
- `flutter test --reporter compact`（`apps/desktop_flutter`）：成功、104件すべて通過。
- `flutter build windows --release`（`apps/desktop_flutter`）：成功、`build/windows/x64/runner/Release/gui_shell_desktop.exe`を生成。installed smokeの代替ではない。
- `python -X utf8 tooling/manifest.py --write`：成功、975件を書込（exit code 0）。`python -X utf8 tooling/manifest.py --check`：成功（exit code 0）。
- `python tooling/conformance_tests/run_conformance_skeleton.py`：成功、192件すべて通過。
- `python tooling/release_runtime_assertions.py --check`：成功、12件すべて通過。証拠範囲はCONFIG／FIXTURE。
- `python -X utf8 tooling/日本語基底監査.py --strict`：FAIL。既存baselineの3 files／19 findings（`docs/specs/windows-desktop-launcher.md` 1、`native/rust_helper/src/broker/ipc_server.rs` 12、`native/rust_helper/src/desktop_launcher.rs` 6）と一致し、この変更由来の追加findingはない。

この単位の試験は静的境界・widget表示・buildまでで、Windows installed productの起動、外部UIAutomation treeの正式収集、release readinessを新たに証明しない。


## 製品Setup Doctorから開発toolchainと生path表示を除去（2026-09-25）

通常利用者向けのSetup DoctorがFlutter／Dart、Python、Rust helperの開発検証手段を必要条件のように表示していたため、Broker snapshotの取得元・鮮度・network exposure・audit chainと既存診断checkを中心にした製品情報へ整理した。snapshot/config/auditの生pathは画面要約へ出さず、開発toolchainの確認は開発者向け検証文書とCLIへ分離する。AuthorityやBroker operationの意味は変更しない。

検証結果:

- Desktop `flutter analyze`：成功、指摘0件。
- Desktop `flutter test --reporter compact`：成功、104件すべて通過。
- `python tooling/conformance_tests/run_conformance_skeleton.py`：成功、193件すべて通過。
- Windows `flutter build windows --release`：成功、`build/windows/x64/runner/Release/gui_shell_desktop.exe`を生成。
- `python tooling/schema_check/check_schemas.py`：成功、Schema122件・正常例122件・拒否例147件。
- `python tooling/release_runtime_assertions.py --check`：成功、検査12件。証拠範囲はCONFIG／FIXTURE。
- `python -X utf8 tooling/日本語基底監査.py --strict`：FAIL。既存3 files／19 findingsと一致し、本単位の追加findingは解消済み。
- `MANIFEST.sha256.json`：975件を再生成し、検査成功（いずれも終了値0）。`git diff --check`成功。

本単位は製品版環境診断の表示面だけを変更する。製品証拠の書出しと初回設定・監査保存先の入出力責任に残る正式配布阻害条件は解消しない。


## Setup DoctorのFlutter直接filesystemと自己申告証拠を撤去（2026-09-25）

現行mainの再読で、Flutter起動時にcollector注入環境変数があれば、Dartが設定JSONを新規作成し、監査directoryへ固定名probeを書いて削除し、product-generatedと自己申告する診断JSONを保存していたことを確認した。通常起動ではこの環境変数がなく、初回設定生成も実行されなかった。したがって、従来のhelper testは実製品の通常動作や正式証拠を示していなかった。

FlutterのSetup Doctor file操作・export helper・mainからの呼出しを削除した。表示面は従来どおりBroker snapshotを読むだけとする。installed smokeはWindows外部collectorの観測結果を明確な`EXTERNAL_EVIDENCE`として記録し、製品生成・正式release入力へ偽装しない。正式Setup Doctor validatorは変更せず、この外部probeを引き続きrejectする。設定または監査directoryが存在しない場合にcollectorが作成して成功扱いする挙動を除き、probe名をGUID化し`CreateNew`で作ることで固定名fileを上書きしない。さらに、起動前から存在するconfigを初回起動生成と誤認しないよう、起動前状態を記録し、正式evidence validatorにその不存在を要求する。外部Setup Doctor collectorのconfig checkも、存在・parse成功だけでは生成合格にせずwarningに留める。非権限・recoveryの性質は維持する。

検証結果:

- Desktop `flutter analyze`：成功、指摘0件。
- Mobile `flutter analyze`：成功、指摘0件。
- Desktop全test：`flutter test --concurrency=1 --reporter compact`で102件すべて成功。既定並列実行では既存の実Broker統合testが終了時の10秒上限に一度到達したが、同testの単独再実行と直列全testは成功した。
- `flutter build windows --release --no-pub`：成功。これはinstalled Windows実行の証明ではない。
- `python tooling/schema_check/check_schemas.py`：Schema 122件、正常例122件、拒否例147件すべて成功。
- `python tooling/conformance_tests/run_conformance_skeleton.py`：195件すべて成功。起動前config既存の負例と外部probeの未観測生成判定を含む。
- Windows PowerShell parser：変更したcollector 2本の構文parse成功。collectorのisolated installed product実行は未実施。
- `python tooling/release_runtime_assertions.py --check`：12件すべて成功。証拠範囲はCONFIG／FIXTURE。
- `python -X utf8 tooling/日本語基底監査.py --strict`：FAIL。既存baselineの3 files／19 findings（Windows launcher仕様1、Rust IPC 12、Rust desktop launcher 6）と一致し、新規findingはない。
- `MANIFEST.sha256.json`：tracked削除をindexへ反映後、974件で再生成し、`--check`も成功。

この単位はFlutter内Setup Doctor filesystem境界と誤った証拠生成経路を閉じた。初回設定の正式生成、Broker統治された機械可読Setup Doctor、Rust起動器を通すclean installed Windows run、negative/failure経路は未成立であり、`rev2_flutter_broker_channel_boundary`、`windows_installer_first_run_smoke`、`windows_setup_doctor_smoke`を`release_blocker`のまま維持する。変更はその代替証明ではない。


## D4 Pocket rev2 Phase35: Mobile Agent状態metadata投影（2026-09-25）

現行main、Phase35仕様、C24 Mobile正本、Agent Adapter contract、既存Rust Broker／Device Link実装を再確認した。Brokerにはread-only `Agent一覧`操作が既にあり、Agent Adapter metadataを返していたが、Mobile側の許可操作・投影画面がなく、Phase35のAgent状態 surfaceへ接続されていなかった。

既存Broker handlerをそのまま使い、TLS Device Linkのread-only allowlistへ`Agent一覧`を追加した。MobileはAgent画面を選択したときだけ一度取得し、常駐pollingしない。新Schemaで一覧とAgent Adapter metadataを閉じ、共通Agent Adapter Schemaにも256文字・64要素・8 platformの上限を加えた。Mobile clientで証拠種別、件数、field集合、状態enum、Agent ID／Capability ID重複、Workspace・credential・Host metadataを検証する。画面modelにはAgent ID、provider、version、model、Broker報告状態、Capability IDと対応状態だけを残す。理由文字列、secret path、資格情報、任意fieldを表示へ持ち込まず、未知・不正・拒否応答は未観測または取得失敗として扱う。`ready`も実task実行可否、Trust、Permission、Approvalの証拠とはしない。Agent起動、編集、比較、権限操作、新bridgeは追加していない。

検証結果:

- `flutter analyze`（`apps/mobile_flutter`）：成功、指摘0件。
- `flutter test --concurrency=1 --reporter compact`（`apps/mobile_flutter`）：成功、36件すべて通過。Agent metadataの正常投影、権限field／秘密値、証拠種別、重複ID、65件超過の拒否、および画面選択前に要求しないことを含む。
- `flutter build apk --debug`（`apps/mobile_flutter`）：成功、`build/app/outputs/flutter-apk/app-debug.apk`を生成。Android SDKからSDK XML v4をv3までのtoolが読んだという互換性warningが出たが、debug buildは完了した。実機install／起動の証拠ではない。
- `dart format --output=none --set-exit-if-changed ...`：成功、対象6 fileに差分なし。`git diff --check`も成功。
- `python tooling/schema_check/check_schemas.py`：Schema123件、正常例123件、拒否例149件すべて成功。Agent一覧について65件と257文字の動的負例も拒否した。
- `python tooling/conformance_tests/run_conformance_skeleton.py`：195件すべて成功。
- `python tooling/release_runtime_assertions.py --check`：12件成功。証拠範囲はCONFIG／FIXTUREに限り、live product healthの証明ではない。
- `cargo test --locked --lib -- --test-threads=1`（`native/rust_helper`）：244件すべて成功。全targetの`cargo test --locked -- --test-threads=1`はlibrary 244件とhelper binary 7件が通過した後、`tests/broker_ipc.exe`をWindows Application ControlがOS error 4551で起動前拒否し、完遂できなかった。policy回避は行っていない。`windows_rust_integration_test_execution_policy`は既存`release_blocker`のまま。
- `python -X utf8 tooling/日本語基底監査.py --strict`：既存baselineの3 files／19 findingsと一致（Windows launcher仕様1、Rust IPC 12、Rust desktop launcher 6）。この監査時点で本単位由来の追加findingはなかった。
- Android実機接続・install・起動は実施していない。既存指示による凍結を維持し、`rev2_mobile_device_evidence`を`release_blocker`のまま保持する。

本単位はPhase35のAgent metadata表示面と既存read-only経路接続の部分成果であり、Phase35全体、Agent実task実行、Mobileの実機・配布、Windows installed product、rev1総合完成を成立させない。既存release blockerの状態は変更していない。


## D4 Pocket rev2 Mobile: Android Device Linkをnative境界へ移行（2026-09-25）

現行Mobile sourceを再確認し、Dartが招待JSON、DeviceCredential、Secure Storage、TLS／networkを直接扱う経路と、試験招待をdebug VM extensionからDartへ渡すintegration harnessが残っていたことを確認した。後者は「資格・secretをFlutterへ渡さない」というrev2境界と両立しないため、旧Dart client、試験driver、integration test、および`--dart-mobile-client`／Simulator用harness経路を廃止した。旧PASS記録は履歴として保持し、現行native経路の証拠へ読み替えない。

Flutter側は資格の有無を示すboolと閉じた状態projection、固定MethodChannel経由のBroker operationだけを扱う。招待JSON、端末ID、Host情報、資格値、TLS、暗号化保管はDartへ返さない。履歴閲覧時だけ既存Brokerが現在のowner grantを照合するための`approval_id`参照とbounded queryをchannelへ渡し、他operationでは同fieldを拒否する。request payloadをIPC前に64KiBへ制限し、応答の資格field・error自由文を拒否／除去する。

Android Kotlin経路にnative招待dialog、Host／HostID／証明書hashの操作者照合、Android Keystore由来AES-GCM暗号化保管、private IPv4範囲・期限・credential bindingの検証、証明書hash固定TLS、bounded single-frame JSON、有限timeout、background時のsocket cancellationを追加した。backupとdevice transferをManifest／data extraction ruleから除外する。Activityの`onResume`／`onPause`／`onStop`だけがnative foreground状態を更新し、Flutter channelから状態を偽装する`set_foreground`要求を削除した。native pairingは二重開始を排他し、結合中のread／Broker／解除要求を拒否する。

検証結果:

- `flutter analyze --no-pub`（`apps/mobile_flutter`）：成功、指摘0件。
- `flutter test --no-pub --reporter expanded`（`apps/mobile_flutter`）：13件成功。固定channel、資格field拒否、history grant参照の限定、自由文error除去、背景中要求拒否、native側foreground再確認、MissingPlugin fail-closedを含む。
- `gradlew.bat :app:testDebugUnitTest`（`apps/mobile_flutter/android`）：成功。Strict JSON、重複key／malformed input、payload allowlist、credential leak拒否、Host／端末bindingのKotlin unit test 7件。
- `flutter build apk --debug --no-pub`、`flutter build appbundle --debug --no-pub`：両方成功。これらはemulator／実機installやnative runtime動作を証明しない。
- `python tooling/schema_check/check_schemas.py`：Schema124件、正常例124件、拒否fixture150件すべて成功。
- `python tooling/conformance_tests/run_conformance_skeleton.py`：197件成功。Mobile channel Schema、Rust Device Link allowlist、Flutter禁止経路、native lifecycle正本、pairing排他を含む。
- `python -X utf8 tooling/日本語基底監査.py --strict`：成功、追加負債0件。
- `python tooling/manifest.py --check`、`python tooling/release_gate_check.py`、`python tooling/packaging_portability_check.py`：成功。
- `python -m py_compile tooling/minidora_live_check.py tooling/conformance_tests/run_conformance_skeleton.py`：成功。
- `flutter devices`：Windows／Chrome／Edgeのみ。Android device/emulatorは検出されない。Android実機検証凍結を維持しinstall／起動／結合／TLS接続は行っていない。

この単位でAndroid sourceのcompileとdebug package生成は成立したが、Android Keystoreの実OS動作、TLSから実Rust Brokerへのnative接続、結合・失効・OS背景遷移は未検証。iOS native handlerは未実装である。iOS compileはWindowsでは行えない。秘密をDart／debug VM／log／artifactへ渡さないplatform live-test harnessも未成立である。加えて現行`local_delete`はDesktop Rust BrokerのAuditEventを生成しないため、回復監査の閉包も未成立と明記した。`rev2_mobile_flutter_native_device_link_boundary`と`rev2_mobile_device_evidence`を`release_blocker`のまま保持し、Android実機凍結と正式配布識別子／署名blockerも変更しない。


## D4 Pocket rev2 Mobile: iOS Device Link native経路の追加（2026-09-25）

現行iOS Runner、Rust `device_transport`／`device_link`／Broker応答、Flutter MethodChannel契約を照合し、iOS側にnative実装がないGapを埋めた。Flutter implicit engineのapplication messengerへ固定channel handlerを登録し、招待入力と接続先照合をUIKit native画面に置いた。iOS native層で重複JSON key、招待期限・端末ID・Host・証明書hashを検査し、ThisDeviceOnly・非同期なしのKeychain itemへ保存後に再読取する。Network.framework TLS clientはRust側の一要求一改行frame、4MiB応答上限、5秒期限、証明書DER hash固定、TLS resumption/ticket無効化、background時cancelに合わせた。通常Broker要求は既存TLSからRust Brokerへ送り、応答nonce・operation・status・Audit ID・evidence class・資格漏えいを検査してからFlutterへ限定projectionする。新しい権限経路は追加していない。

Runner XCTestを追加し、重複／escape重複JSON key、招待binding・expiry・IP範囲、履歴grant referenceのoperation scope、Simulator KeychainのThisDeviceOnly属性・write/readback/deleteを検査する。Apple補助workflowは`workflow_dispatch`のみのまま、iPhone Simulator XCTestを実行しsummaryを保存するよう拡張した。Flutter／native sourceのtracked差分だけでなく未追跡source生成も検出する。これはApple補助runであり、CI gateではない。

検証結果:

- `python tooling/schema_check/check_schemas.py`：成功、Schema124件・正常例124件・拒否例150件。
- `python tooling/conformance_tests/run_conformance_skeleton.py`：成功、197件。
- `python -X utf8 tooling/日本語基底監査.py --strict`：成功、新規負債0件。
- `flutter analyze --no-pub`（Mobile）：成功、指摘0件。
- `flutter test --no-pub --reporter expanded`（Mobile）：成功、13件。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml -- --test-threads=1`：成功、library244・helper binary7・全integration35件を含む全286件。
- `python -X utf8 tooling/manifest.py --write`／`--check`、`python tooling/release_gate_check.py`、`python tooling/packaging_portability_check.py`：成功。
- Windows hostに`swift`／`xcodebuild`はなく、Xcode compileとSimulator XCTestは未実行。Apple workflowも現時点では未dispatch。Swift sourceのcompileを成功扱いしない。
- 実端末からnative TLSをDesktop Rust Brokerへ接続するplatform-native LIVE_RUNTIME harnessは未実装・未検証。Android実機検証凍結は維持する。

残存項目:

- item: iOS native compile／XCTestおよびiOS実機のKeychain・TLS・lifecycle証拠
  classification: release_blocker
  reason: WindowsではApple toolchainを実行できず、追加sourceは未compile。Simulator XCTestも未実行であり、physical deviceや実運用を証明しない。
  required_action: 手動Apple workflowを現行commitで実行し、compile／XCTest失敗を修正する。別途iOS実機のnative実通信を検証する。
  blocks_release: yes
- item: native Device Linkの実接続試験経路と`local_delete`回復監査
  classification: release_blocker
  reason: secretをFlutter／debug VM／log／artifactへ出さずに実Rust Brokerへ接続するplatform LIVE_RUNTIME harnessはなく、通信不能時のlocal_deleteにもBroker AuditEventがない。
  required_action: native境界を保持する実接続harnessを追加し、local_deleteを監査済みBroker操作または安全境界を維持する明示的な回復監査契約へ接続する。
  blocks_release: yes
- item: Android実機検証
  classification: release_blocker
  reason: 2026-09-11のowner凍結指示が継続中であり、未実施を合格へ読み替えない。
  required_action: 凍結解除のowner指示後にのみ実機検証を再開する。
  blocks_release: yes

この実装単位もD4 Pocket完成、Mobile実機安全性、正式配布、owner GOを成立させない。`rev2_mobile_flutter_native_device_link_boundary`と`rev2_mobile_device_evidence`を未解決release blockerとして維持する。


## Windows Broker smoke一時session資格fileの後始末（2026-09-25）

Windows実行で既存の`collect_broker_smoke.ps1`を確認したところ、Brokerを強制終了した後も、session secretを含むendpoint fileが検証用profileに残ることを観測した。資格値そのものは表示・記録していない。collectorをversion 5へ更新し、Broker停止処理の後始末でfile生成の観測値を固定してからendpoint fileを削除する。停止経路で例外があってもnested `finally`で削除を試み、削除失敗は成功として扱わない。証拠には`session_file_created`と`session_file_removed_after_collection`を別fieldで記録する。release validatorは両方の直接測定とfield provenanceを要求し、削除falseの負例を拒否する。`windows_broker_installed_smoke`のrequired actionもcleanup実測を含むよう更新した。

現行Rust source commit `5310c4c1c64f20a16dc02beae730734e1c3a5b1a`からWindows debug helperをbuildし、collector version 5をdirty worktreeから隔離profileで実行した。結果は`status=passed`、IPC認証・loopback bind・durable store・restart後replay拒否・新規要求・crash fail-closedがすべてtrue、errors 0。endpoint fileはcollector開始時に前回の検証残留を除去し、今回の終了後にも`session_file_removed_after_collection=true`、実file不存在を確認した。session資格は生成evidenceへ含まない。artifactは`%LOCALAPPDATA%\GUI-Shell\development-evidence\broker-smoke-5310c4c-2af62dbde1c44c8ead8669fffad490d0\windows_broker_smoke.json`、SHA-256 `7425ed3bbb4285c2a2145c30c1b0190148ea6f76fdeb485dceccdcd44ddd54ba`。helper SHA-256は`66bd3cf43dfeb7e4a797cd3f48b86f3244b58492e2196541bed97e1093d7d54c`。collector証拠を`validate_broker_smoke`へ直接渡し、broker evidence項目として機械検証に合格した。

検証結果:

- `cargo build --locked --manifest-path native/rust_helper/Cargo.toml`：成功（Windows debug profile）。
- `powershell -NoProfile -ExecutionPolicy Bypass -File installer/windows/collect_broker_smoke.ps1 -BrokerHelperExe "C:\Users\ohira\OneDrive\ドキュメント\ChatGPT\D4ポケット\native\rust_helper\target\debug\gui_shell_rust_helper.exe" -OutputPath "C:\Users\ohira\AppData\Local\GUI-Shell\development-evidence\broker-smoke-5310c4c-2af62dbde1c44c8ead8669fffad490d0\windows_broker_smoke.json" -StoreDir "C:\Users\ohira\AppData\Local\GUI-Shell\development-evidence\broker-smoke-5310c4c-2af62dbde1c44c8ead8669fffad490d0\store" -SessionFile "C:\Users\ohira\AppData\Local\GUI-Shell\development-evidence\broker-smoke-5310c4c-2af62dbde1c44c8ead8669fffad490d0\broker_session.json"`：成功。Brokerを実起動してrestart/replay/crash境界とcredential file cleanupを実測した。出力は`LIVE_RUNTIME`／`EXTERNAL_EVIDENCE`だが、helperはdevelopment build、collector sourceも未commit時点であり、clean-source／installed product証拠ではない。
- `python -c 'import json; from tooling.windows_release_evidence import validate_broker_smoke; p=r"C:\Users\ohira\AppData\Local\GUI-Shell\development-evidence\broker-smoke-5310c4c-2af62dbde1c44c8ead8669fffad490d0\windows_broker_smoke.json"; data=json.loads(open(p, encoding="utf-8").read()); print(validate_broker_smoke({"broker": data}))'`：成功。これはcollector objectの項目検査であり、installed product全体の合格ではない。
- `python -m py_compile tooling/windows_release_evidence.py tooling/conformance_tests/run_conformance_skeleton.py`：成功。
- `python tooling/schema_check/check_schemas.py`：Schema 124件、正常例124件、拒否fixture150件で成功。
- `python tooling/conformance_tests/run_conformance_skeleton.py`：197件で成功。削除falseのBroker証拠をrelease blockerとして拒否する。
- Windows PowerShell parser：`collect_broker_smoke.ps1`を構文解析し成功。
- `python tooling/validate_all.py --python-only --desktop-platform windows`：初回は今回の変更によるMANIFEST不一致でmanifest／release gate／packaging checkが失敗。`python tooling/manifest.py --write`後の再実行では集約開発検証が完了し、日本語基底監査・Schema・Conformance・manifest・release gate・packaging・release smoke・evidence bundle・runtime assertionsが成功した。Windows installed evidenceは未存在のため各製品release gateは`release_blocker`、`release_ready=false`のまま。

証拠の範囲は開発用Rust Broker processであり、正式配置、Rust起動器→Flutter Runner→named pipe→Brokerの結合、Setup Doctor、初回設定生成、署名配布を証明しない。`windows_broker_installed_smoke`、`windows_installer_first_run_smoke`、`windows_setup_doctor_smoke`、`rev2_flutter_broker_channel_boundary`は引き続き`release_blocker`とし、状態を合格へ変更しない。


## D4 Pocket rev2 BrokerのAgent Adapter宣言検証（2026-09-25）

既存の`Agent一覧`経路を再確認し、Flutter側にはAgent Adapter Schema検査がある一方、BrokerはAdapterの任意JSONを未検証のまま一覧へ投影していたGapを確認した。Brokerで既存`agent_adapter.schema.json`の必須field、型、列挙値、上限、nested unknown field、`secret_value_present=false`を検査する。Authority metadataと既知credential markerを含む文字列、重複Adapter／Agent IDも拒否し、失敗時は本文をerror／Auditへ複写せず`応答不正`とBroker Audit記録だけを返す。有効な宣言は既存read-only一覧経路で維持する。既知marker検出はそのmarker群だけを対象とし、任意形式の秘密値が存在しないことを証明しない。

Rust単体試験では、schema準拠fixture、有効Agent一覧のBroker受理、root／nested Authority・credential field、既知credential形式を説明文字列へ埋め込む試行、`secret_value_present=true`、null optional fieldを検証する。Broker拒否試験ではresponseとAuditの両方に試験用sentinelが現れないことを確認した。日本語意味正本へこの宣言境界を追加し、正本索引へ登録した。Agent実task接続、Agent間比較、Handoff、同一Workspace汚染の実Agent試験を成立させる変更ではない。

検証結果:

- `python tooling/schema_check/check_schemas.py`：Schema124件、正常例124件、拒否fixture150件で成功。
- `python tooling/conformance_tests/run_conformance_skeleton.py`：198件成功。
- `python -X utf8 tooling/日本語基底監査.py --strict`：成功、新規負債0件。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib -- --test-threads=1`：初回は246件中245件成功。既存A2A loopback試験1件が応答本文のsocket読取で失敗したため、同試験の単独再実行は成功し、その後の全体再実行では246件すべて成功した。失敗を隠さず記録する。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib agent_list_rejects_untrusted_adapter_metadata_with_audit_and_no_leak`：成功。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib Mobileの読み取り投影は既存Broker統治経路だけを通りowner作用を拒否する`：成功。有効metadataのBroker経由一覧投影を確認。
- `cargo build --locked --manifest-path native/rust_helper/Cargo.toml`：Windows debug profile build成功。
- `python tooling/manifest.py --write`／`python tooling/manifest.py --check`：成功。現行tracked fileをmanifestへ再固定した。
- `python tooling/validate_all.py --python-only --desktop-platform windows`：終了code 0。日本語監査、Schema、Conformance、manifest、release gate、packaging portability、release smoke、evidence bundle、runtime assertion、C32 development auditの全stepが成功。これはPython側の統合開発検証であり、Rust library試験・buildおよびWindows installed product evidenceとは別範囲。release evidence bundleは`release_ready=false`と既存blockerを維持した。
- `rustfmt --edition 2021 --check native/rust_helper/src/broker/dialogue.rs`：既存の同file内format差分を含むため失敗。新規追加部の差分は整形したが、task外の既存行は変更していない。
- `git diff --check`：成功。

既存release blocker registryは変更していない。Owner操作を伴うGUI-Shell Export経路、総合機能拡張rev1全体、Windows installed product、正式配布、実Agentの本番接続は本単位では成立しない。release_readyはfalseのまま維持する。


## D4 Pocket rev2 C27/C29再測定とC28長時間試験開始（2026-09-25）

Agent Adapter metadata検証を含むsource commit `8202ac796eba0909c9d5b83cf71e89d8fc97fc5a`で、C27性能測定とC29障害注入を再実行した。C27はsnapshot生成1016ms、Flutter projection test process全体20245ms。Flutter fixture内部は起動model 0ms、snapshot読込38ms、Runtime／Audit／History projection 4ms、検索2ms、通知5ms、resource polling 13ms、4096行差分UI 1041msだった。C29はRuntime停止・再接続、Broker crash、MCP timeout、A2A timeout、必須credential不在、store書込不能simulation、Audit書込失敗、malformed state起動拒否の8件すべてPASSした。

証拠はC27が`INTERNAL_STATE`／`FIXTURE`、C29が`LIVE_RUNTIME`／`FIXTURE`／`INTERNAL_STATE`。C27値は性能SLA、installed product、実GPU frame、実Runtime負荷の保証ではない。C29のstore障害は容量を実際に枯渇させた物理disk-full試験ではなく、外部MCP／A2A serviceの実証でもない。

同じsource commit `8202ac796eba0909c9d5b83cf71e89d8fc97fc5a`で8時間C28試験を開始した。証拠fileは`%LOCALAPPDATA%\GUI-Shell\development-evidence\c28-8h-8202ac7-20260925.json`。2026-09-25 01:26:35Zの観測では`running`、経過662.5秒、対話144成功、接続断で想定した失敗36、Runtime相当再起動36、Broker再起動36、再接続72だった。これは試験途中の進捗であり、8時間完遂・成功を示さない。実Broker processとlocalhost Runtime fixtureによる開発試験であり、installed productまたは外部Runtimeの長時間稼働証拠ではない。終了状態を後続記録で追記する。

- `release_blocker`: C28の8時間運用完遂、installed product／外部Runtimeでの運用、実製品性能測定、rev1全工程完成、Windows installed product、正式配布・署名、owner GOおよび正式releaseは未成立。Owner明示操作を伴うGUI-Shell Export経路も未成立。
- `known_limitation`: C27/C29測定は開発時のfixture・限定障害注入範囲であり、製品SLAまたは全外部障害条件の証拠へ昇格しない。

既存release blocker registryは変更していない。`release_ready=false`を維持する。


## D4 Pocket rev2 C28再測定失敗と一時資格file cleanup修正（2026-09-25）

前項の8時間試行は、source commit `8202ac796eba0909c9d5b83cf71e89d8fc97fc5a`で703.391秒後、`段階=Broker再起動`においてC28検証器自身が次回起動前に一時`broker.json`を削除できず終了した。証拠は`%LOCALAPPDATA%\GUI-Shell\development-evidence\c28-8h-8202ac7-20260925.json`。状態は`failed`、対話成功150、接続断で想定した失敗37、Broker再起動37。記録された例外はWindows `Permission denied`である。失敗後の後始末で一時directoryが削除され、Broker helper processが残っていないことを確認した。拒否が一時file占有かACL等のどれに由来したかは特定できていない。C28失敗をRuntime製品欠陥または成功へ読み替えない。

検証器のBroker再起動処理だけを変更し、normal／owner一時資格fileの`PermissionError`を最大20回・50ms間隔で再試行する。再試行回数を`Broker資格file削除再試行数`として証拠へ含め、上限後も拒否される場合は従来どおりfailedにする。削除対象は検証器が所有する固定session pathに限り、Broker操作、Audit、Runtime requestは再試行しない。製品のBroker、Owner資格、IPC、Authority、Permissionのproduction経路は変更していない。

- Unit test: `python -m unittest tooling.conformance_tests.test_long_run_validation`は4件成功。瞬間的PermissionErrorからの回復、継続拒否の失敗維持、既削除file、統計への再試行数計上を確認した。
- C28 30秒: status `passed`、対話成功10、想定disconnect失敗2、Runtime相当再起動3、Broker再起動2、資格file削除再試行0。実Broker／localhost fixtureの短時間smoke。
- C28 120秒: status `passed`、対話成功36、想定disconnect失敗9、Runtime相当再起動9、Broker再起動9、再接続18、資格file削除再試行0。実Broker／localhost fixtureの短時間stress。
- `python tooling/failure_injection_validation.py`: 修正後にC29障害注入8件すべて`passed`。Runtime crash、Broker crash、MCP／A2A timeout、credential unavailable、store書込不能simulation、Audit failure、malformed stateを含む。
- `python tooling/validate_all.py --python-only --desktop-platform windows`: exit 0。strict日本語監査、Schema、conformance 199件、manifest、release gate、packaging portability、release smoke、evidence bundle、runtime assertions、C32構造監査のconfigured stepが成功した。`--python-only`のためC27/C28/C29、Rust test、Flutter test/buildは含まず、C28/C29は上記の個別実行結果である。release blockerは残り`release_ready=false`。
- これらは未commit作業ツリーでの検証器回帰であり、8時間完遂、installed product、外部Runtime、実製品性能を証明しない。clean committed sourceからの8時間再試行は別途実施する。

- item: C28の8時間運用完遂
  classification: release_blocker
  reason: 旧試行は検証器の一時資格file削除拒否で703.391秒後にfailedとなり、修正後は30秒・120秒のfixture試験だけである。
  required_action: 修正をcommit・pushしたclean source commitで8時間試験を再実行し、失敗時は失敗内容を保持して調査する。
  blocks_release: yes
- item: Windows installed product／外部Runtime長時間運用
  classification: release_blocker
  reason: C28は開発用localhost fixtureとdevelopment Brokerを使い、installed productまたは外部Runtimeを検証しない。
  required_action: C28とは独立したWindows installed productおよび外部RuntimeのLIVE_RUNTIME証拠を成立させる。
  blocks_release: yes

release blocker registryを変更せず、`release_ready=false`を維持する。


## D4 Pocket rev2 C28 clean-source再試行の通信失敗（2026-09-25）

cleanup修正commit `adc9e1d2f5e533e83e3b0f065ea6280d3585b77b`から開始した8時間再試行は、122.265秒後、`段階=network切断`でfailedとなった。証拠fileは`%LOCALAPPDATA%\GUI-Shell\development-evidence\c28-8h-adc9e1d-20260925.json`。対話成功35、予定した接続断失敗9、Broker再起動9、再接続18、資格file削除retry 0。失敗後の復旧対話はBroker上で`通信失敗`となり、本文・応答routeなしで完了記録された。

同証拠のRuntime fixture観測では失敗した対話の`/api/chat`および`/api/trace` handlerまで到達し、status 200が記録されている。ただし現行fixtureはHTTP statusをresponse write前に記録するため、全response bytesのflushまたはRust Adapterによる受信を証明しない。この観測だけではBroker／Adapter側の通信失敗とfixture側の送信切断を切り分けられない。cleanup修正がこの失敗を解決したとは扱わない。

- item: C28の8時間運用とRuntime切断後の復旧対話
  classification: release_blocker
  reason: clean commitでの8時間再試行が122.265秒後に通信失敗となり、現fixture evidenceはHTTP response送信完了を示さない。
  required_action: fixtureのresponse flush結果とbounded transport error classを記録し、対話本文や資格を露出せず失敗層を特定する。原因修正後、clean committed sourceで長時間再試験を行う。
  blocks_release: yes
- item: C28 fixture HTTP statusの送信証拠範囲
  classification: known_limitation
  reason: statusがresponse write前に採取され、server handler到達は示すがclient受信を示さない。
  required_action: status採取をwrite/flush後へ移し、送信失敗時はerror classのみを記録する。証拠をserver-side flush範囲に限定して説明する。
  blocks_release: yes

既存release blocker registryは変更せず、`release_ready=false`を維持する。


追加のflush計測入り120秒再現も41.969秒後、`段階=Runtime相当再起動`で`通信失敗`となった。対話成功13、想定disconnect失敗3、Runtime相当再起動4、Broker再起動3、再接続7、資格file cleanup retry 0。直前のfixture `/api/chat`とtrace responseは全件`server_flush_succeeded`だったが、これはclient受信の証明ではない。そこでC28が起動するdebug Broker childに限り、Adapterのtransport失敗stageと標準error kindを、route分類付き・最大32行・payload非記録でstderrへ出す診断を追加した。検証器側は許可値だけをreportへ抽出し、生のstderrをreportへ複写しない。次の再現で得る診断は原因特定用であり、現時点で失敗原因やproduction動作を断定しない。

## D4 Pocket rev2 C28 flush計測・transport診断付き短時間再現（2026-09-25）

前回失敗の観測限界を閉じるため、Runtime fixtureはwrite／flush成功後にstatusを記録し、送信例外時は例外classだけを記録するよう変更した。debug Broker childにはopt-in診断を追加し、C28失敗reportは許可値に合う最大32件の固定transport分類だけを含む。例外messageとBroker stderr全文はreportへ出さず、例外型名のみを残す。これらの診断はfailure-onlyの補助情報で、通信成功やclient受信の保証ではない。

- `python tooling/long_run_validation.py --duration-seconds 120 --output %LOCALAPPDATA%\GUI-Shell\development-evidence\c28-120s-diagnostics-20260925.json`: status `passed`、経過122.671秒、対話成功44、予定disconnect失敗11、Runtime相当再起動11、Broker再起動11、再接続22、資格file cleanup retry 0。対話最大1112.73ms。Broker working setは11,948,032～14,577,664 bytes、storeは14,113～675,297 bytes。観測区間中に予期しないtransport failureはなく、Rust診断の実失敗経路はこのrunでは発火していない。
- `python -m unittest tooling.conformance_tests.test_long_run_validation`: 8 tests passed。資格file cleanup、flush前後の計測順、fixture write error class限定、diagnostic allowlist／32行上限、exception message非出力を検証。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib adapters::minidora`: 4 tests passed。対象fileの`rustfmt --edition 2021 --check`も成功。workspace全体の`cargo fmt --check`は変更外の既存Rust filesに大量のformat差分があるため失敗し、全体整形は行っていない。

これは未commit作業ツリーでの120秒development fixture測定であり、先行したclean sourceの8時間失敗を解消した証拠ではない。C28 release blockerは維持し、次はclean committed sourceで8時間検証を再実施する。既存release blocker registryは変更せず`release_ready=false`を維持する。


## D4 Pocket rev2 Owner確認付きGUI Shell Export統合（2026-09-25）

`rev2_export_owner_ui_authority_path`の開発経路を実装した。設定画面でExportを開始すると、Flutterは通常Broker要求だけを送る。Rust Desktop起動器は厳格parse、`desktop_flutter` metadata、Broker session未注入、freshness、payload hash、Manifest、Module計画を確認し、hashと検証済み表示値を含むWindowsネイティブOwner確認を表示する。Yesの場合だけ、容量1のprocess内channelでBroker所有threadへ固定の`GUI Shell書出し`要求を渡す。Owner秘密値、Owner role、privileged IPCはFlutterへ渡さず、Owner資格fileも生成しない。No／未確認は通常資格のBroker経路へ送り、Owner不足または期限切れとして監査する。Brokerはsession、時刻、nonce/replay、payload hash、Authority metadataを処理時に再検証し、内部Owner経路ではExport以外を拒否する。

受理時は既存Export contractの新規App identity／Audit store／Manifest Receiptを返し、Authority、Permission、Approval、Credential、Audit chainを継承しない。現段階はManifest-onlyの応答であり、永続Broker Audit以外のfile、binary、Installer、署名、buildは作らない。Owner確認はWindows session上の明示操作であって、Windows account再認証または本人性証明ではない。

- Rust `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib -- --test-threads=1`: 251件成功。通常資格拒否、テスト用Yes／No、永続Broker Audit、別operation拒否、hash改変、期限切れ、session偽装、Authority metadata、継承禁止を確認。先行parallel実行は既存localhost通信test 2件のtimeoutと追加testのAudit期待文字列不一致で失敗した。期待値を実際のAudit (`owner_required`) に修正し、serial全件を再実行してPASSした。
- `docs/specs/gui-shell-export.md`を更新し、実経路と未成立範囲を区別した。`release_blockers.registry.json`の該当blockerは実装・自動統合testだけで解消扱いにせず、clean commitからの対話的Windows Desktop確認・Yes／Noと実Audit証拠を待つ状態へ更新した。`release_ready=false`を維持する。
- 確認ダイアログはRust `MessageBox`のdefault-No。Dart待機は305秒、Windows named-pipe transportはExport要求だけ310秒、Broker要求鮮度は300秒であり、期限後のOwner操作はBrokerが拒否する。
- Rust `cargo test --locked --manifest-path native/rust_helper/Cargo.toml -- --test-threads=1 --quiet`は通常のrepository pathから全target完了。library 251件、main 7件、Broker IPC 9件、その他integration 26件、合計293件がPASSした。前段の隔離worktree実行ではA2A localhost試験が1度失敗したが、単独再試験とserialのlibrary再試験、およびこの最終全体試験では再現しなかった。
- `cargo build --locked --manifest-path native/rust_helper/Cargo.toml --bins`、Flutter `flutter analyze --no-pub`、`flutter test --no-pub`（103件）、`flutter build windows --debug --no-pub`、`flutter build windows --release --no-pub`はいずれも隔離worktreeでPASSした。Flutter Windows buildはOneDrive配下のCloud Files reparse placeholderでC++ wrapper sourceを読めず、OneDrive上のAnalyzerが不正URIで停止したため、現在のcommitと差分を一時detached worktreeへ複製して実施した。製品repositoryの恒久移動やOneDrive設定変更はしていない。
- Release buildを一時staging layoutへ配置し、`LOCALAPPDATA`も専用試験directoryへ分離して実Desktopを起動した。設定画面のnative Owner確認には「いいえ」を選び、隔離Brokerの永続Auditで`operation=GUI Shell書出し`、`decision=rejected`、`reason=owner_required`を確認した。実行内容はManifest-onlyであり、Export artifactの作成は確認されなかった。これは未commit差分のdevelopment LIVE_RUNTIME観測であり、clean commitに結び付くinstalled-path evidence bundleではない。Ownerの「はい」は承認操作なので実施していない。
- `python tooling/validate_all.py --python-only`は全10 check PASS（日本語基底strict、Schema 124、Conformance 199、manifest、release gate、packaging、release smoke、evidence bundle、runtime assertions、C32構造監査）。evidence bundle検査は5件の既存release blockerと`release_ready=false`を維持していることを確認した。C32のPASSは対応表構造の検査であり、全機能完成やreleaseの証明ではない。
- Windows product起動は実施したが、clean commitからのinstalled-product証拠、Owner許可側の実クリック、独立binary生成／Module pruning／Installer／署名／配布／rollbackは未成立の`release_blocker`であり、この作業単位では解消扱いにしない。

直前commit `59b9c921a0aa4e347d299b193b3ba914c136fee1`をsourceとするC28 8時間試験は、Owner Export開発を優先して628.875秒で意図的に中断した。`%LOCALAPPDATA%\GUI-Shell\development-evidence\c28-8h-59b9c92-20260925.json`は中断時の`状態=running`のままで、対話成功138、接続断で想定した失敗34、Runtime相当再起動35、Broker再起動34、再接続69、証拠file 255,661 bytesを記録する。停止後に対象Python／Broker helper processが残っていないことを確認した。これは途中記録であり、失敗または8時間完遂・成功の証拠ではない。C28 release blockerは維持する。


## D4 Pocket rev2 clean-commit Windows Export拒否確認（2026-09-25）

source commit `bde6d3c51fcf27820e952d52ad2f4abaac53b868`を共有cloneから隔離Tempへcheckoutし、そのtracked sourceを変更せずWindows Release productをbuild・起動した。Flutterはclean cloneに`.dart_tool`がないため最初の`flutter build windows --release --no-pub`がDart plugin生成で失敗した。`flutter pub get`でlockfile依存を復元後、同じbuild commandは91.5秒で成功した。Temp stagingのためMSBuild `MSB8029` warningが出たがbuildは完了した。`cargo build --locked --release --manifest-path native/rust_helper/Cargo.toml --bins`も成功した（release時に未使用となるC28診断関数2件のdead-code warning）。

配置した主要実行ファイルのSHA-256値:

- Flutter `gui_shell_desktop.exe`: `47D58E153F928914A97A8B3943757A3B24DED39B10EBF97B623E4C5FD4B89F19`
- Rust `gui_shell_desktop_launcher.exe`: `41CF83731C77179C9FFD65ED23A0B730BA9A0F5350C0C1BBA27AA5F83D3B0388`
- Rust helper実行ファイル `gui_shell_rust_helper.exe`: `40101206EDEB8A896F52333AA5C138AE187974BC82FD622229AD92BE41F68298`

製品を独立`LOCALAPPDATA`の隔離directoryで起動し、実設定画面からnative Owner確認を開いた。dialogはManifest-only、artifact／Installer／build／署名／user fileなし、Windows account再認証ではない旨を表示した。「いいえ」を選び、隔離Brokerの永続Auditに`operation=GUI Shell書出し`、`decision=rejected`、`reason=owner_required`、`evidence_source=INTERNAL_STATE`が記録された。製品を実際に起動したnegative動作確認だが、Auditの証拠源は`INTERNAL_STATE`であり、外部収集証拠やformal installerを経たinstalled-path evidenceではない。全package hashも未取得。Ownerの「はい」は承認操作なので実行していない。

従って`rev2_export_owner_ui_authority_path`はrelease blockerのまま維持する。Owner許可側Audit／Receipt非継承、formal installed-path collector、完全な独立Export artifact／Module pruning／Installer／署名／配布は未確認または未成立である。前回のdirty worktree実証をclean-source証拠へ書き換えず、この追補で新しい検証条件と範囲を記録する。


## D4 Pocket rev2 C28検証器 telemetry 上限修正（2026-09-25）

commit `0bfc1aa21966c6f75ddb7430cc1c2ab3cbbce22f`から開始した8時間C28試験は、約1,408.89秒後に手動中断した。出力`%LOCALAPPDATA%\GUI-Shell\development-evidence\c28-8h-0bfc1aa-20260925.json`は中断時点の`状態=running`を保持し、file sizeは1,521,395 bytesだった。C28 fixtureがtrace IDを含む可変HTTP pathをroute counterのkeyへ使い、trace詳細辞書も無制限に保持していた。これは検証器自身のメモリ・証拠量を増加させ、8時間の資源測定を汚染するため、このrunを完遂証拠として採用しない。出力は改変・削除せず、部分記録として保持する。専用Python／Broker helper processが終了したことを確認した。

修正ではHTTP pathを5種の固定route labelへ正規化し、trace fixture参照を最大128件、HTTP結果ring bufferを32件、資源sampleを512件に制限する。snapshotはlock内で複写する。任意trace IDはHTTP telemetryへ出さず、古いtraceは上限超過後にfixtureから参照できない。製品runtimeや保存契約は変更していない。

- `python -m unittest tooling.conformance_tests.test_long_run_validation`: 9件成功。fixture pathの正規化、HTTP結果32件上限、trace参照128件上限、先頭trace追い出し、任意path／trace IDのtelemetry非出力を確認。
- `python tooling/manifest.py --write`および`--check`: 成功。
- `python -X utf8 tooling/日本語基底監査.py --strict`: PASS、負債file 0／findings 0。
- `python tooling/validate_all.py --python-only --desktop-platform windows`: exit 0。Schema 124／124 example／150 negative、conformance 199 checks、manifest、release gate、packaging portability、release smoke、evidence bundle、runtime assertions、C32構造監査を含むconfigured checksがPASS。Windows installed evidence 5項目は`release_blocker`のまま。
- 修正後smoke `python tooling/long_run_validation.py --duration-seconds 120 --interval-seconds 1 --output %LOCALAPPDATA%\GUI-Shell\development-evidence\c28-120s-bounded-20260925.json`: `passed`、122.25秒、対話成功36、予定disconnect失敗9、Runtime再起動9、Broker再起動9、再接続18。resource sample 12、HTTP route 4種、HTTP結果32件、trace ID非出力、証拠JSON 57,505 bytes。

- `release_blocker`: C28の8時間運用完遂
  classification: release_blocker
  reason: 先行clean-source試行は検証器telemetryが非boundedと判明したため手動中断し、完遂していない。
  required_action: telemetry上限修正を含むclean commitから8時間試験を再実行し、完遂結果と資源sample上限を確認する。
  blocks_release: yes

既存release blocker registryは変更せず、`release_ready=false`を維持する。


## D4 Pocket rev2 Mobile 端末内回復記録の契約・表示追加（2026-09-25）

現行mainと端末連携正本、Android／iOS native source、Flutter channel、Schema・conformanceを読み直し、通信不能時の`local_delete`が端末資格を消すだけで回復記録を残さないGapを確認した。Desktopへ到達できない操作をDesktop Rust Broker AuditEventへ偽装せず、OS保護領域内の独立した`mobile_local_recovery`記録として定義した。

記録は資格削除と同じ暗号化状態を一度だけ更新し、readbackで削除と記録を確認する。Androidは既存AES-GCM／Keystore保護状態、iOSはThisDeviceOnly Keychain itemを用いる。旧状態version 1を読めるようにし、次回writeでversion 2へ移行する。記録は最大32件で古いものをrotationし、固定の公開操作hashと固定fieldだけを含める。資格、Host、端末ID、自由文は含まない。操作者identityは`unverified`、Desktop失効は`unconfirmed`、authority effectは`none`、証拠源は`INTERNAL_STATE`に固定した。Flutterにはversionだけを渡す読み取り専用native methodと、限定fieldの検証・表示を追加した。未記録・破損・過大・追加secret・失効確認への偽装は拒否する。これはDesktop Audit chain、外部収集証拠、失効、本人性、権限を証明しない。

検証結果:

- `python tooling/schema_check/check_schemas.py`：Schema 125件、正常例125件、negative fixture 151件で成功。固定操作hashの変更と資格secret混入を拒否した。
- `python tooling/conformance_tests/run_conformance_skeleton.py`：200件成功。Android／iOS native store・channel契約のsource境界、上限、通常disconnect分離、Flutterの厳密projectionを検査した。
- `python -X utf8 tooling/日本語基底監査.py --strict`：成功、新規負債0件。途中でROADMAP／registry記述が監査閾値に触れたため日本語表現へ修正し、再実行してPASSした。
- `flutter test --no-pub --reporter expanded`（一時`R:` path alias経由）：16件すべて成功。端末内削除からのBroker要求不発、回復記録読取、固定projection、secret field拒否、UTC日時形式と存在しない日付・改行・offset拒否を確認した。
- `flutter analyze --no-pub`（一時`R:` path alias経由）：成功、指摘0件。
- `dart format`：変更したFlutter client／controller／画面／testの4 fileを整形した。`git diff --check`成功。
- `gradlew.bat :app:testDebugUnitTest`を2回、`:app:compileDebugKotlin`を1回実行したが、すべて`:app:cleanMergeDebugAssets`で停止しKotlin compile前に失敗した。対象は追跡外の`apps/mobile_flutter/build/.../mergeDebugAssets`で、ACLに`Everyone Deny DeleteSubdirectoriesAndFiles`が設定されていることを観測した。ACL変更は所有者判断を待っており、この変更後sourceのAndroid unit test／compile／APK buildは未検証である。以前のsourceに対するAPK成功を流用しない。
- iOSの`xcodebuild`／XCTestはWindows hostにtoolchainがなく未実行。Apple補助workflowは`workflow_dispatch`限定だが、利用可能なGitHub操作にworkflow dispatchがなく、`gh` CLIも見つからないため起動していない。Simulator・実機試験の代替とはしない。
- Android実機検証凍結を維持し、端末を接続・起動していない。今回sourceに対するAPK生成、native TLSからRust Brokerへの`LIVE_RUNTIME`、iOS Keychain実動作は未確認。
- `python tooling/manifest.py --write`／`--check`：成功。manifestはSchema・正例・負例を含む993 fileを固定した。
- `python tooling/validate_all.py --python-only --desktop-platform windows`：exit 0。日本語strict、Schema 125／125／151、conformance 200、manifest、release gate、packaging portability、release smoke、evidence bundle、runtime assertions、C32構造監査が成功。これはPython側統合検証で、Android／iOS native build・Flutter test／analyzeを含まず、`release_ready=false`と既存release blocker 5件を維持する。

### Android Gradle ACL調査の追補

上記の初回失敗後、ACLを変更せず生成物directoryだけを削除対象から外す診断実行を追加した。`apps/mobile_flutter/build`以下のignored生成物には親から継承した`Everyone Deny DeleteSubdirectoriesAndFiles`があり、Gradle標準cleanupが拒否されることをACLで確認した。これは製品sourceの不具合とは区別するが、標準build検証を阻害する環境制約である。

- `gradlew.bat :app:testDebugUnitTest -x :app:cleanMergeDebugAssets`：`compileDebugKotlin`と`compileDebugUnitTestKotlin`は成功した。続くJUnit実行は`build/app/test-results/testDebugUnitTest/binary`の削除拒否で停止し、unit test assertionは実行されていない。
- `gradlew.bat :app:assembleDebug -x :app:cleanMergeDebugAssets`：incremental debug APK生成は成功した。APKは161,029,992 bytes、SHA-256 `DAD766390A5CF36555C867474F7B295E8DD7B246DD385EB6AC64D7F5730EF53E`。これはclean build、標準cleanup経路、実機動作またはrelease evidenceではない。
- `:app:cleanMergeDebugAssets`除外の責任は、ACLで削除不能な既存のFlutter生成asset directoryのcleanupをその一回だけ避け、残りのGradle taskを診断することに限る。生成物再利用の可能性が残るため、標準buildと同等のcleanlinessを主張しない。source変更や製品runtimeの回避策ではなく、一時的な検証補助であり、ACL解消後に除外指定なしのunit testとdebug APK buildを再実行する。
- JUnit結果directoryにも同種のdeny ACLを確認した。ACL変更の許可は得られていないためACLを変更していない。通常unit test、clean build、APKの標準経路はいずれも未成立としてrelease blockerを維持する。

ROADMAP、Mobile状態、端末連携正本、release blockerの説明を実装状態へ追従させた。`rev2_mobile_flutter_native_device_link_boundary`と`rev2_mobile_device_evidence`は`release_blocker`のまま維持し、`release_ready=false`を変更しない。この実装単位は変更後Android／iOS nativeの全platform build・実機動作、正式release、D4 Pocket完成を成立させない。

## D4 Pocket rev2 Android一時検証先によるclean・unit test追補（2026-09-25）

OneDrive上の生成物ACLを変更せず通常Android検証を完了できるかを確認した。Gradle初期化scriptでbuild出力先を後から一時場所へ移す試行は、Android Gradle Pluginが早期に固定した`:app:cleanMergeDebugAssets`出力先を移せず、同じ削除拒否で失敗した。元の作業場所で後片付けtaskだけを除外したAPK再組立も、別の除外対象`packageDebugResources\\merged.dir\\values`への`AccessDeniedException`で失敗した。従って元のAndroid build出力ACL制約は一つのdirectoryだけではなく、除外対象`apps/mobile_flutter/build`配下に及ぶことを確認した。ACL変更は行っていない。

そこで現行repositoryの追跡済み／未追跡sourceをWindowsの一時workspaceへ複製した。`apps/mobile_flutter`と`packages/gui_shell_ui`の計117ファイルについて、複製直後と`flutter pub get --offline`後にSHA-256を照合し、差分0件だった。除外された端末固有の`android/local.properties`、Gradle wrapper script／JARだけは同じ元workspaceから検証用複製へ追加した。Dart／Kotlin／Swift source、Schema、製品設定の回避変更は行っていない。

検証結果:

- `flutter pub get --offline`: 成功。
- `gradlew.bat clean :app:testDebugUnitTest :app:assembleDebug --no-daemon --offline --console=plain`（一時複製内の`apps/mobile_flutter/android`）: cleanup除外なしで成功。64 task、57 executed、7 up-to-date。clean出力先は新規複製内であり、OneDrive ACLには触れない。
- JUnit XML: `DeviceLinkPolicyTest` 5件、`StrictJsonTest` 3件、計8件。failure 0、error 0、skipped 0。
- debug APK: `%TEMP%\\D4PocketMobileValidate-90c66f63c8d246ac81eb3f62c717c3b6\\workspace\\apps\\mobile_flutter\\build\\app\\outputs\\flutter-apk\\app-debug.apk`、146,311,641 bytes、SHA-256 `2D40701A90A518261D5E9E7E5E96AADF036D1A78354B9181B0E001E9A6632129`。開発用debug成果物で、実機install／起動・release署名・配布の証拠ではない。
- これはこのWindows hostのACL制約を避ける一時検証用の複製であり、製品のbuild出力先変更ではない。OneDrive内で直接buildする必要がある場合に限り、継承ACLを管理者判断で修正後、元の作業場所上で標準commandを再実行する。ACL未解消でも同一sourceのAndroid clean／unit／package検証は成立したため、ACL制約を製品のrelease blockerへ昇格しない。
- Flutter `test --no-pub --reporter expanded`: 16件成功。Flutter `analyze --no-pub`: 指摘0件。
- `python tooling/validate_all.py --python-only --desktop-platform windows`: exit 0。日本語基底strict、Schema 125／正常例125／negative 152、conformance 200、manifest、release gate、配布互換性、release smoke、evidence bundle、runtime assertions、C32構造監査が成功した。既存release blocker 5件と`release_ready=false`を維持する。統合検証にはiOS／Android native compile、Flutter試験、実機検証は含まれない。
- Android実機検証の凍結、Apple開発環境によるiOS compile／XCTest、実際のTLS通信／Rust Broker結合、KeyStore／Keychainを用いる実機動作、lifecycleはこの検証で成立しない。既存のMobile統合・実機証拠に関する`release_blocker`と`release_ready=false`を維持する。

## D4 Pocket rev2 C28失敗分類・phase診断境界の改善（2026-09-25）

clean commit `6a7ccffe9bb3b433c35e9b97e2dc7bef7604a1a7`から開始した8時間試行は22.766秒後、Runtime相当再起動phaseでfailedとなった。証拠`%LOCALAPPDATA%\GUI-Shell\development-evidence\c28-8h-6a7ccff-20260925-142215.json`のSHA-256は`8B8876ADAF0F856DC7E9A1E95728AC03B480632AF6F4F8AB914F5B40CDDE0868`。対話成功9件、想定disconnect失敗2件、Runtime再起動3件、Broker再起動2件、再接続5件で終了した。reportは旧形式の`RuntimeError`とphaseをまたぐtransport診断しかなく、根本原因は特定不能だった。証拠ファイルは変更していない。

検証器へ固定allowlistの`失敗分類`を追加し、既知の対話非成功、Broker拒否、fixture probe失敗、timeoutを固定codeへ分類した。transport診断は各phase action開始offset以降の最大64 KiBだけから抽出し、既存のroute／stage／detail allowlistと32行上限を維持する。exception message、stderr全文、対話本文、資格値は出力しない。Retry、成功条件の緩和、production adapter timeout変更は行っていない。

- `python -m unittest tooling.conformance_tests.test_long_run_validation`: 12 tests passed。未知code拒否、secret markerを含むexception messageの非出力、古いphase diagnosticの除外を含む。
- 変更中worktreeでの`python tooling/long_run_validation.py --duration-seconds 120 --output %LOCALAPPDATA%\GUI-Shell\development-evidence\c28-120s-classified-6a7ccff-20260925.json`: passed、120.25秒、対話成功38、想定disconnect失敗9、Runtime再起動10、Broker再起動9、再接続19、Broker working set 12,005,376〜14,843,904 bytes、store 14,747〜573,165 bytes。artifact SHA-256: `383DF236F9CA63174E4A7FAA69E54824F5BEE0648B412ED1E393BD98CA5F3303`。
- 追加の30秒Broker regressionもpassed（対話成功12、想定disconnect失敗3）。単独Runtime fixture stop／start／probeの20反復もpassedだが、Brokerとの結合安定性を示さない。
- `python -X utf8 tooling/日本語基底監査.py --strict`: PASS、負債file 0／finding 0。`python tooling/validate_all.py --python-only --desktop-platform windows`: exit 0、設定された10検査がすべてPASS。Schema 132／正常例132／negative fixture 163、Conformance 201件、manifest、release gate、配布互換性、release smoke、evidence bundle、runtime assertions、C32構造監査を含む。統合検証のC28は30秒smokeであり、8時間試行のfailed記録を置き換えない。
- この短時間証拠は8時間成立へ昇格しない。診断改善を含むclean commitからC28の8時間試験を再実行する。`release_blocker`: 8時間運用完遂、installed product・外部Runtime evidence。`known_limitation`: fixture測定はinstalled product、外部Runtime、製品SLAを証明しない。`release_ready=false`を維持する。

## D4 Pocket rev2 Windows installed evidence collector境界更新（2026-09-26）

clean commit `b81fc607e65ecaf7fc8bc5de53376e39949e0f20`から行ったC28 8時間再試行は、開始約0.578秒で`大量対話`段階に失敗した。証拠 `%LOCALAPPDATA%\GUI-Shell\development-evidence\c28-8h-b81fc60-20260925-145127.json` のSHA-256は`AC69F9D977F9FAE6E10941F7FB3555CA5D06F2585C09FBFB119CCC7A1D3DC0B0`。固定分類は`dialogue_result_not_success`、診断は`C28_RUNTIME_DIAGNOSTIC|health|read_headers|ConnectionReset`。fixtureはhealth要求2件についてserver-side flush成功を記録しただけで、clientがresponse bytesを受信した証拠ではない。過去failed記録は保持し、8時間PASS・根本原因特定へ昇格しない。

Windows collectorでは、`collect_installed_smoke.ps1`のFlutter executable直接起動と独立Broker起動を除き、staged manifestでhash照合したRust Desktop起動器からFlutter childを同一実行内で起動する。正式collectorはstage時と異なるWindows user profileを要求し、run固有LOCALAPPDATA下の実runtime endpointとBroker起動／終了Auditを観測する。raw SIDやsession secretを証拠へ書かず、manifestにはrun固有salt付きSID digestだけを保存する。外部Setup Doctor JSON、stage scratch config／Auditを正式product evidenceへ混入しない。endpoint role metadataのみを製品接続証明にしない。

validatorにはruntime・config pathが実行userのisolated LOCALAPPDATA配下にあることを要求する否定試験を追加した。PowerShell構文解析、Schema 132／example 132／negative fixture 163、Conformance 205件、manifest 1,025 file、`python -X utf8 tooling/日本語基底監査.py --strict`、`python tooling/validate_all.py --python-only --desktop-platform windows`（全10検査）がPASSした。統合集約はrelease blocker 5件と`release_ready=false`を維持した。Rust起動器、Broker helper、Flutter Release executableはいずれもworkspaceに存在しないため、installed collectorの実行・別Windows profile実証は未実施である。現行Desktop productは通常Broker接続signal、初回config生成、Broker統治Setup Doctor exportをformal pathとして提供していないため、first-run／Setup Doctor blockerを維持する。

### 同日ソース追跡の補足

前項の「通常Broker接続signal未接続」はsource確認不足を含むため、ここで証拠境界を更新する。`apps/desktop_flutter/lib/services/shell_core_client.dart`の製品起動は最初に`health`を要求し、Rust `handle_stream`は通常endpoint secretの照合後にだけBroker処理へ進む。`Broker::accept_health`は受理時に`operation=health`、`decision=accepted`、`evidence_source=LIVE_RUNTIME`を永続Auditへ記録し、Audit書込み成功後にresponseを書き出す。collectorは新規隔離StoreのこのAuditを読み、厳密なhealth受理record数と最初のevent IDを記録する。endpoint metadataだけの証明ではない。

このAuditはBroker側の認証済み要求受理・永続記録を示す一方、clientがresponse bytesを受信したことや、記録の呼出し元PIDを識別するものではない。evidenceには両方を未観測と明記し、通常資格のhealth受理Auditを`LIVE_RUNTIME` provenanceとしてvalidatorが要求する。したがってsignalの収集実装は存在するが、実際のclean-source／別Windows profile runは未実施であり、初回config生成と正式Setup Doctor product exportも未接続である。前項の実行・release blocker判定は解除せず、説明だけをこの観測境界へ精密化する。

初回conformanceでは合成証拠の`unsupported_claims`を非空にしたため厳格validatorに拒否された。client応答受信・PID帰属は専用fieldで未観測と記録し、unsupported claim一覧を空にした後の再実行は205件PASSした。最終検証ではPowerShell parse、`git diff --check`、Schema 132／example 132／negative fixture 163、日本語厳格監査（負債file 0／finding 0）、manifest 1,025 fileがPASSした。文書・fixture修正後の`python tooling/validate_all.py --python-only --desktop-platform windows`もexit 0で設定済み10検査すべてPASSし、release blocker 5件と`release_ready=false`を維持した。Rust／Flutter toolchainは利用可能だが、Rust起動器、Broker helper、Flutter Release executableがworkspaceに存在しないため、実installed product・別profile収集は未実施であり製品実証へ昇格させない。

### 同日Windows installed app診断・collector process同定修正（2026-09-26）

上記source追跡に続けて、OneDrive外の短い一時checkoutをclean基準commit `b81fc607e65ecaf7fc8bc5de53376e39949e0f20`へ固定し、Rust Release起動器／Broker helperとFlutter Windows Release appをbuildし、run固有rootへstageした。clean product binaryに対する同一profile `-DiagnosticOnly` runではD4 Pocket画面が実際に起動し、Broker Auditに通常資格後の`health / accepted / LIVE_RUNTIME`と画面初期化の複数recordが残った。一方、collectorは起動器childを特定できず失敗し、失敗処理が画面processを残すことも確認した。この診断は別Windows profileではないため正式release evidenceではない。

原因はWindows package virtualizationにより、WMI `ExecutablePath`がstage pathではなくCodex packageの`LocalCache`表記を返すことだった。独立process probeで、報告されたimageのSHA-256はstage済みFlutter executableと一致し、observed parent PIDはRust起動器PID、Windows sessionも一致することを確認した。collectorは旧来のpath文字列一致を、直接親PID・同session・起動後start time・image SHA-256による照合へ置換する。失敗時cleanupも同じ検証済みchild選択を使う。strict evidence validatorとnegative conformanceは、親PID不一致、child関係欠落、image hash不一致／未検証を拒否する。

当該記録作成時点では、この修正後の再実行・cleanup確認、全ローカルvalidation、commit／push／remote HEAD確認は未完了であり、結果を先取りしていない。Windowsの別profile、初回config製品生成、Setup Doctor product exportを含む既存release blocker 5件と`release_ready=false`を維持する。

### 同日実installed app診断の再試行（2026-09-26）

tray exit実装の確認後、同一のclean `b81fc607e65ecaf7fc8bc5de53376e39949e0f20` product binaryを使い、更新済みcollector version 12を`-DiagnosticOnly -NoPythonRuntime`で再実行した。可視UIA surface 4件、直接child／parent PID／same session／起動後start time／image SHA-256照合、通常資格health受理Audit 1件、lifecycle shutdown Audit、endpoint削除を観測した。通知領域『終了』menuはUIAutomationで起動器childのprocess上から実行され、強制終了=false、cleanup error=false、起動器exit code 0で完了した。終了後に対象runのapp processとendpointが残存しないことを確認した。

実測evidence `%LOCALAPPDATA%\D4Pocket-Windows-Diagnostic-853a77b567c64d73b1f659aa0a928720\installed-smoke-diagnostic-final.json` のSHA-256は`ab575eaa3ff4d332279b7f74ae124276a9b9b520b602a1d99c11273c8bcff3fe`。statusは意図どおり`diagnostic_only`、stage userと同じprofile（`profileSeparate=false`）であるため、Windows正式release evidenceには昇格しない。別profile実証、初回config product生成、正式Setup Doctor export等のrelease blocker 5件と`release_ready=false`は継続する。

修正作業blockの最終検証は、`python tooling/schema_check/check_schemas.py`（Schema 132／正常例132／negative fixture 163）、`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`（208件）、`python -X utf8 tooling/日本語基底監査.py --strict`（負債0件）、`python tooling/manifest.py --check`、PowerShell AST parse、`git diff --check`がすべて成功した。`python tooling/validate_all.py --python-only --desktop-platform windows`もexit 0でconfigured 10 checksが成功し、既存release blocker 5件と`release_ready=false`を維持した。Rust／Flutter product sourceはこのblockでは変更していない。別profile正式runとrelease evidence欠落は依然として`release_blocker`である。

終了操作の自然終了raceを拒否するguardを加えた最終sourceでもcollector version 12を再実行した。`installed-smoke-diagnostic-final-v2.json`のSHA-256は`311d56c7bad2981cc6a0032bbecf7f001f652d4c4b5da4c8fdf648ab03336309`。statusは`diagnostic_only`、profile分離false、可視surface 4件、health受理1件、shutdown Auditあり、endpoint削除済み、終了menu実行済み、強制終了なし、cleanup errorなし、起動器exit code 0であり、対象app processも残っていない。正式release証拠ではない。

## D4 Pocket rev2 Broker統治Setup Doctor報告経路（2026-09-26）

通常製品UIから認証済みRust Broker IPCへ固定payload `{version: 1}`を送り、Brokerが生成した7項目のSetup Doctor報告を読む経路を追加した。報告には任意の保存先指定を許さず、固定Broker Store内の最新一件だけを64 KiB以内でatomic replaceする。保存byte列のSHA-256を同じ要求の`accepted / LIVE_RUNTIME` AuditEventへ結び、保存・監査が成立しない場合は成功応答を返さない。Flutterはfilesystemを触らず報告を表示し、Schemaどおりのfield集合・識別子・状態・証拠classを満たさない入力は`unknown`へ閉じる。Windows collectorは固定storeの生byte列と受理Auditを照合してbundleへ含め、strict validatorはSchema・hash・Audit結合・隔離store pathを再検証する。Setup Doctor自体はAuthorityを付与しない。

検証結果:

- `cargo test -- --test-threads=1`：最終再実行成功。Rust library 267件、CLI 9件、統合・checkpoint等36件を含む全targetがPASSし、Setup Doctorのloopback TCP IPC、認証拒否時の非生成、保存byte列と受理Audit hashの一致、任意path拒否を実行した。手戻し中の初回compileはreparse判定helper不足を検出したため実装し、最終全targetを再実行した。さらに、先行する一回はWindows Application Controlが生成test executableの起動をerror 4551で拒否したが、OS設定変更・別配置binaryによる迂回をせず、後続の最終source全target試験は成功した。
- `python -X utf8 tooling/schema_check/check_schemas.py`：Schema 133件、正常例133件、negative fixture 164件で成功。
- `python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`：210件成功。
- `python -X utf8 tooling/日本語基底監査.py --strict`：負債file 0件、finding 0件で成功。これは静的意味監査であり、実画面や自然さを証明しない。
- `flutter test --no-pub --reporter expanded`：107件すべて成功。`dart format lib/services/shell_core_client.dart test/widget_test.dart`を実施した。追加fieldに資格値を混入する否定試験が画面側で`unknown`へ閉じることを含む。
- `flutter analyze --no-pub`：OneDrive内では既存Flutter ephemeral reparse pathの削除拒否があるため、一時検証copyで実行。変更した両Dart fileのSHA-256を作業treeと照合後、指摘0件で成功した。これは作業tree外copy上のsource解析であり、Desktop実行証拠ではない。
- `python -m py_compile tooling/windows_release_evidence.py tooling/schema_check/check_schemas.py tooling/conformance_tests/run_conformance_skeleton.py`、PowerShell parserによるcollector AST parse、`git diff --check`：成功。
- `python tooling/validate_all.py --python-only --desktop-platform windows`：exit 0、configured 10検査すべてPASS。Schema／conformance／manifest／release gate／配布互換性／release smoke／evidence bundle／runtime assertion／C32構造監査を含む。集約はrelease blocker 5件と`release_ready=false`を維持する。Windows installed product証拠や製品完成の証明ではない。

今回の報告はsource-levelのproduction経路・認証済みloopback統合試験までを示し、変更後Windows installed productの別profile runやSetup Doctor画面の実読取を示さない。初回config生成contractは未接続で`setup_doctor.config_created=unknown`、`operator_readable=false`を維持し、`windows_setup_doctor_smoke`等のrelease blockerと`release_ready=false`は解除しない。前回までの失敗記録を成功へ書き換えず、現行実装の検証結果を別記する。

## D4 Pocket rev2 Broker統治 初回UI設定の生成・読取接続（2026-09-26）

既存UI既定値をBroker所有の固定初回設定へ接続した。インストール先検証済みDesktop起動器だけが起動前に固定Broker storeへ既定設定をcreate-onlyで生成し、通常Broker IPCは固定payloadのread-only取得だけを受け付ける。既存fileの改変・未知field・過大値・reparse pointは置換せずfail-closedにする。初期生成と取得はBroker Auditへ記録し、取得Auditのpayload hashは保存byte列のSHA-256と結び付ける。Flutterは設定を表示テーマ・locale・densityへ投影するだけで、file、process、資格情報、networkへ直接触れず、未知fieldや固定Schema外の値を受け取った場合はBroker unavailableへ閉じる。初回設定はUI preferenceだけであり、Capability、Permission、Approval、authorityを含まない。

`first_run_configuration`／`first_run_configuration_request` JSON Schemaと正常・negative fixtureを追加し、Schema checker、conformance、Broker persistent store、IPC起動経路、Flutter product client、Windows installed evidence collector、strict evidence validatorへ接続した。collectorは起動前のfile不在、固定pathでの生成、厳格UTF-8／field／値検査、accepted `初回設定取得` AuditEventと実file byte hashの一致を要求する。証拠bundleとvalidatorも同一path・hash・LIVE_RUNTIME eventを再照合する。失敗・事前作成・hash不一致は正式first-run PASSに昇格しない。

統合検証の初回実行では、未追跡の新規Schema／fixtureがGit-index基準の配布file一覧から欠落し、展開後conformanceがSchemaを見つけられないことを検出した。意図した新規contract fileをstage対象に含めてから検査を行う運用へ合わせ、Manifestを更新し、source archiveにcontract／fixtureが入ることを確認した。またWindowsでは配布検査とrelease gateの子Pythonがlocale依存出力を行い、親のUTF-8読取threadが`UnicodeDecodeError`を起こし得る境界を確認した。両checkerのPython子processにUTF-8 mode／stdout encodingを明示し、親もUTF-8で読取るよう固定した。失敗時の日本語diagnosticを保持するconformanceを加えた。この固定UTF-8接続は恒久的な検査契約であり、製品runtimeや権限境界は変更しない。

検証結果:

- `python -X utf8 tooling/schema_check/check_schemas.py`：Schema 135件、正常例135件、negative fixture 167件で成功。
- `python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`：214件成功。新しいSchemaの権限・path・preference拒否、Windows evidenceのfile hashとAudit不一致拒否、配布子processのUTF-8 diagnostic保持を含む。
- `python -X utf8 tooling/packaging_portability_check.py`：成功。1035件のGit追跡済み入力からsource archiveを作成・展開し、展開先でManifest、conformance、release gateを実行した。Schema／fixtureを含むことを確認した。
- `python -X utf8 tooling/日本語基底監査.py --strict`：成功。負債file 0、finding 0。静的監査であり、実画面の自然さやruntime表示を証明しない。
- `python -X utf8 tooling/manifest.py --write`：file 1035件を記録。文書追記後に再生成・照合する。
- `flutter test --no-pub --no-test-assets --concurrency 1 --reporter compact`：Desktop Flutter全108 test成功。OneDrive管理下のtest asset reparse pointを削除しないno-test-assets経路であり、実Desktop起動の証拠ではない。
- `flutter analyze --no-pub`：一時`R:` path aliasと分離`LOCALAPPDATA`から実行し、指摘0件。aliasは検証終了時に解除した。
- `cargo test --lib --target-dir C:\Users\ohira\AppData\Local\Temp\codex-d4pocket-rust-target-20260926 -- --test-threads=1`（`native/rust_helper`から）：Rust library 273件成功。初回configのcreate-only、改変／未知field／過大値を置換しない試験を含む。
- `cargo check --all-targets --target-dir C:\Users\ohira\AppData\Local\Temp\codex-d4pocket-rust-target-20260926`（`native/rust_helper`から）：成功。
- Python `py_compile`、Windows PowerShell parserによるcollector AST parse、`git diff --check`：成功。
- `cargo test`全targetはWindows Application Controlが生成済みtest executableをerror 4551で拒否した試行があり、全targetの実行結果は本記録では確定しない。library 273件とall-target `cargo check`は成功したが、統合test executableの実行証拠には代替しない。OS policyを無効化して回避しない。

残存項目:

- item: current sourceのWindows installed productを別user profileから正式collectorで再実行し、初回設定・Audit hash・Setup Doctor画面・Broker lifecycleを実測する。
  classification: release_blocker
  reason: 今回の証拠はSchema／conformance／unit test／配布展開検証であり、変更後installed productのLIVE_RUNTIME evidenceではない。
  required_action: clean sourceから生成した製品をstage時と異なるSIDのWindows profileで起動し、strict evidence validatorへ合格させる。
  blocks_release: yes
 - item: `windows_rust_integration_test_execution_policy`。変更後sourceの全target Rust integration testを実行できなかった。
  classification: release_blocker
  reason: Windows Application Controlのerror 4551でtest executableが起動拒否された。cargo checkとlibrary testは別の検証面である。
  required_action: OS policyを弱めず、許可済みWindows hostで変更後sourceの全target `cargo test --locked --manifest-path native/rust_helper/Cargo.toml -- --test-threads=1`を成功させる。
  blocks_release: yes

過去の「初回設定未接続」と2026-09-25のRust全target成功記録は履歴として保持する。現行sourceについて全target integration test実行が未成立の新証拠を受け、`windows_rust_integration_test_execution_policy`を`unresolved`へ戻し、Release Checklistとregistryのrequired actionを同期した。他のblockerを解消扱いせず`release_ready=false`を維持する。この実装blockはD4 Pocket完成、Windows正式証拠、製品releaseを成立させない。

### 同日現行source Windows installed app 診断追補（2026-09-26）

変更中sourceの追跡済み29 pathを一時workspaceへ複製しSHA-256一致を確認後、Flutter Windows Release executableとRust Desktop launcher／Brokerをbuildしてstaged packageを起動した。document／registry更新後のコピー差はmetadataのみで、実行binaryへ影響するsourceは同一。staged App SHA-256 `98f99fff10089650f28cf36685cf61635107a47b8d370db5dd8c5aeab266ca1c`、launcher `d4c57d037aa0e0638ebe59daf4130d6909545adf60034050423187cad47a935d`、Broker `af368cf9bfd5d2ea963eea1300bbca8b3080656d3597874d72b3e762df80edc1`。Windows collector v14を`-DiagnosticOnly -NoPythonRuntime`で実行し、collector exit 0、diagnostic output SHA-256 `3715b79bc041eb9bcb0a907936781b39d62bc1ed466db565aaa6f8a495c43e4e`。

実測では、起動前config不存在、固定Broker storeでのconfig生成、Schema field/value妥当性、config SHA-256 `c7617ab054b70ee2388d96c9d76ee33835800ca9b76298d01686e5320db9f817`、`初回設定取得` accepted／`LIVE_RUNTIME` AuditEvent 1件（`broker-audit-8`）とのpayload hash一致を確認した。通常資格のhealth要求、Setup Doctor report生成・accepted Audit hash結合、no-Python PATH scrubも動作した。Setup Doctor report statusはpassだが、製品画面上のoperator readabilityを意味しない。

同じ実行でWindows window handleと`D4 Pocket` titleは観測したが、UIAutomation treeはroot windowと`FLUTTERVIEW` containerの2 nodeのみで、必須surface `Dashboard`、`NavigationRail`、`Runtime Status`、`Invariant Status`はいずれも取得されず、`visible_surfaces_complete=false`。collectorの終了menu操作は成立せず、強制cleanup=true、cleanup error=true、起動器通常終了falseとなった。cleanup後に当該実行のprocess残存はなかった。実行profileのSIDはstage時と同一（`profile_identity_isolated_from_staging_user=false`）であり、これは`diagnostic_only`に限定される。別profile formal evidenceではない。

この診断によって「初回設定未接続」は現行sourceについて訂正できた一方、Windows Setup Doctor画面の操作者向け可読性／画面要素の意味情報、通常終了・後片付け、別利用者profileによる証拠、Rust全target統合試験は未解決である。`windows_installer_first_run_smoke`と`windows_setup_doctor_smoke`は`release_blocker`のまま維持し、registry／checklistの理由と必須対応をUIA／後片付けの観測に合わせた。`release_ready=false`を維持する。

### 同日Windows Flutter UIAutomation surface露出の修正・診断（2026-09-26）

現行sourceからWindows Flutter Releaseを再構築し、UIAutomation treeが2 nodeに留まった診断を基準に、Windows Embedderの`DartProject`で`AccessibilityMode::IAccessibleEx`を明示選択した。Dart rootで`SemanticsBinding.instance.ensureSemantics()`を常時保持する実験はsurface公開を改善せず、明示modeと併用した試行でもsemantic nodeを観測できなかったため、製品sourceへ残さなかった。今回の成功実行ではIAccessibleEx明示選択とOSのsemantics lifecycleに任せる構成を使い、Flutter 3.44.0 Windows Releaseの実測範囲だけを記録する。これはEmbedder mode選択との観測上の比較であり、他Flutter engine版に一般化する因果保証ではない。

一時検証workspaceは`C:\Users\ohira\AppData\Local\Temp\d4pocket-first-run-source-current-20260926`。Dart `main.dart`と`widget_test.dart`のSHA-256はそれぞれ`7EEB0E1C837655EB96FAB4C122E907AE623703BF6A0D6393FB4FDCC827AE119A`、`FECE3CC984D67FF9BCE41BB2EF64298CDA8F708DAD960C1CD02BA78283F7086B`で、変更前HEADと一致した。runner sourceの一時copyとの差はこの修正で加えた非機能commentだけで、IAccessibleEx選択行は同一。`flutter build windows --release --no-pub`はexit 0（MSBuildは一時pathについてMSB8029 warningを出力）。App SHA-256は`18d6481da5a0ab33273a88c997bf89e6edf059e3b83a9747e4057d36bed2d1dd`。生成物は同一SIDの一時stagingからcollector v14相当を`-DiagnosticOnly -NoPythonRuntime`で実行し、source commit `a0836bbffe3a3db527045e318784939d154b1889`、dirty対象がWindows runner設定のみであることを記録した。staged Rust launcher／Brokerのhashはそれぞれ`d4c57d037aa0e0638ebe59daf4130d6909545adf60034050423187cad47a935d`、`af368cf9bfd5d2ea963eea1300bbca8b3080656d3597874d72b3e762df80edc1`。

診断JSONのSHA-256は`C05464FF4BC4F64D0BD8BCC78C01EF8BE6FB49241ACDA0B7B3C195F1E199CA9A`、可視surface JSONは`3084289F0FCB48E1F49DCF6D11E2477866DC79C4DA648BCE2D85EA0BF94404E6`。UIAutomationは121 nodeを列挙し、`Dashboard`、`NavigationRail`、`Runtime Status`、`Invariant Status`の4 surfaceを個別に認識した。通知領域の通常終了が成立し、強制終了false、cleanup errorなし、起動器終了およびBroker endpoint除去を観測した。Setup Doctor report自体のstatusはpassだったが、画面上のoperator readabilityはfalseのままであり、画面操作・可読性の証拠ではない。stage userと実行profileのSIDは同一で、statusは`diagnostic_only`。正式collectorのstrict validatorにも通していない。

`apps/desktop_flutter/windows/runner/main.cpp`でFlutterView作成前にIAccessibleExを選ぶ回帰検査をConformanceへ追加し、Desktop Dart rootがSemantics treeを強制しないことも固定した。READMEはAPIのexperimental性と実証範囲を記し、Setup Doctor blockerの理由・対応を現在の観測に合わせた。従前の2-node／終了cleanup失敗記録は履歴として保持する。Flutter Windows EmbedderがIAccessibleExをexperimentalと説明しているため、`windows_flutter_iaccessible_experimental_mode`を非release-blockingな`known_limitation`として追加し、engine更新時のUIA・screen reader再検証を要求する。

検証結果:

- `python -X utf8 tooling/schema_check/check_schemas.py`：Schema 135件、正常例135件、negative fixture 167件で成功。
- `python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`：215件成功。Windows runnerのEmbedder mode選択とDart Semantics treeの常時強制禁止を含む。
- `python -X utf8 tooling/日本語基底監査.py --strict`：負債file 0件、finding 0件で成功。
- `python -X utf8 tooling/manifest.py --write`／`--check`：1,035件を記録し、検査成功。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`：exit 0、構成済み10検査すべてPASS。Windows formal evidence fileがないrelease gateは失敗／blockerとして正しく残り、集約は`release_ready=false`。
- `flutter analyze --no-pub`：一時検証workspaceと分離`LOCALAPPDATA`で指摘0件。これは静的Dart解析で、Windows runtime証拠ではない。
- `flutter test --no-pub --no-test-assets --concurrency 1 --reporter compact`：実リポジトリを一時`R:` drive aliasから実行し、Desktop全108件成功。aliasは終了時に解除。先行する一時copy上の試行は8件失敗し、そのうちRust lifecycle testはcopyに`native/rust_helper/target/debug`がなく明示的に失敗したため、copy実行を製品結果として扱わず、Rust helperを含む実リポジトリから全件を再実行した。
- `python -X utf8 -m py_compile tooling/conformance_tests/run_conformance_skeleton.py`、`git diff --check`：成功。
- Rust sourceはこの作業blockで変更していないためRust testは再実行していない。全target実行に関する既存blockerは引き続き別途保持する。

この結果は現Flutter 3.44.0 Release上のUIAutomation surface露出と通常tray終了を改善した診断である。別Windows profileのformal evidence、Setup Doctor画面の可読性、strict evidence validator、実screen reader、他Flutter engine版は未確認であり、`windows_setup_doctor_smoke`と`release_ready=false`を維持する。

## D4 Pocket rev2 Windows Rust全target test再確認（2026-09-26）

対象source commitは`fdbff3395a337b115bf1c33b19474ea6b3e567a5`。Rust sourceは直前block以降変更していないが、registryで`windows_rust_integration_test_execution_policy`が`unresolved`のため、Windows host上で要求された正規Cargo実行を再確認した。

- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml -- --test-threads=1`：library 273件中272件成功、A2A loopback Agent Card取得test 1件が応答読取時の`a2a_connection_failed`で失敗。したがってCargoは最初のlibrary target内で停止し、全target完遂の証拠にはならない。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml a2a::tests::loopback_HTTPからAgent_Cardを取得してmetadata_onlyへ射影する -- --exact --nocapture --test-threads=1`：library上の対象testは1件成功した。その後Cargoが次の`gui_shell_desktop_launcher` test executableを起動しようとしたが、Windows Application ControlにOS error 4551で拒否され、コマンド全体はexit 1。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib -- --test-threads=1`：library全273件成功、32.10秒。先行失敗したA2A loopback testも成功したため、一時失敗の原因はこの検証では特定できていない。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib a2a::tests::loopback_HTTPからAgent_Cardを取得してmetadata_onlyへ射影する -- --exact --quiet --test-threads=1`：PowerShellから同一testを20回連続実行し、20回すべて成功。再現性の低い一時失敗の原因特定や全target成功の証拠にはならない。

したがってRust libraryは現sourceで273件PASSだが、helper／launcher／integrationを含む全target testは未完了であり、Windows Application Controlの実行拒否も再確認された。OS policyを無効化したり別配置binaryへ移したりせず、許可済みWindows hostで全target試験を完遂する必要がある。`windows_rust_integration_test_execution_policy`は`release_blocker`のまま維持し、過去の全target PASS記録を現sourceへ転用しない。`release_ready=false`を維持する。

## D4 Pocket rev2 GUI Shell Export実file生成・終了経路修正（2026-09-26）

現行`main`を変更前に`origin/main`と一致するclean stateで確認し、2世代rollback refを更新してから着手した。旧GUI Shell ExportはReceiptを返すのみでfileを作成しなかった。現単位では製品完成・実行可能Appではなく、独立構成Manifest JSON fileの生成までを実体化した。

`export_mode=manifest_file`要求をRust Brokerで再検証し、Rust Desktop起動器が固定 `%LOCALAPPDATA%\GUI-Shell\broker\desktop\exports` pathを検査して開いたdirectory handleだけをBrokerへ渡す。Brokerは新規128-bit App ID名のManifestを最大64 KiBで作り、symlink追跡を無効にしたcreate-new一時fileからhard-linkで確定するため、既存fileを上書きしない。Receiptと完了Auditは実fileのpath、SHA-256、byte長を結び、credential／permission／approval／audit chainは継承しない。`cleanup_pending`時は一時file名とOwner確認・hash照合後のRecoveryActionをReceipt／Auditへ明示し、Flutterにも警告する。完了Auditに失敗した場合は成功Receiptを返さず、確定fileと一時fileの復旧情報を返して停止する。Exportを起動可能package、物理Audit store、独立Runtime、binary pruning、Installer、署名へ読み替えない。

併せて全target試験で、永続Auditにshutdown acceptedがあるのにloopback listenerが残り、`workspace_startup` integration testがprocess終了を待ち続ける実行を観測した。応答後socket drain errorを`unwrap_or(false)`が通常継続へ潰す経路を局所修正し、shutdown結果を切断後処理から独立させるRust unit testを追加した。再build後に同じintegration executableを起動する試行はWindows Application ControlのOS error 4551で拒否され、この実integrationでの修正確認は成立していない。

検証結果:

- `python -X utf8 tooling/schema_check/check_schemas.py`：Schema 136件、正常example 136件、negative fixture 168件で成功。
- `python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`：215 check成功。新Manifest Schema／receipt、一時file後処理状態、既存file非上書き、directory handle境界、権限非継承を含む。
- `python -X utf8 tooling/日本語基底監査.py --strict`：負債file 0件、finding 0件。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib -- --test-threads=1`：Rust library 278件成功。書出しfile、既存file非上書き、cleanup pending、固定保存先なし拒否、junction拒否、Owner／Audit境界とshutdown後処理unit testを含む。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --test workspace_startup startup_owner_approval_real_ipc_read_and_revocation_are_connected -- --exact --nocapture --test-threads=1`：再build後のintegration executable起動をWindows Application ControlがOS error 4551で拒否。修正後の実process統合testは未検証。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml -- --test-threads=1`：1回目はA2A／MINIDORA loopback testが一時失敗し、各単独再実行は成功。次の実行はlibrary 277件、CLI 9件、IPC 10件などを通過後、workspace startup shutdown待ちで完了しなかったため停止した。修正後の全target成功は未確認。
- `flutter analyze --no-pub`：Desktop、Mobileとも一時`R:` drive aliasから実行し指摘0件。直接の日本語OneDrive pathではAnalyzer JSON parse errorが発生したが、path alias利用で回避できた。
- `flutter test --no-pub --no-test-assets --concurrency 1 --reporter failures-only`：Desktop全108件成功。書出しClientの古い`manifest_only`期待値を`manifest_file`へ同期した後の結果。Owner書出しClientの個別testも3件成功。
- `python -X utf8 -m py_compile tooling/conformance_tests/run_conformance_skeleton.py tooling/schema_check/check_schemas.py`：正常に終了した。
- `rustfmt --edition 2021 --check`を変更Rust fileへ実行した結果はexit 1。未変更領域を含む既存の広範なRust書式差分があり、全fileの一括formatは実施していない。
- `python -X utf8 tooling/manifest.py --write`：1038 fileを記録し、その後の`--check`も合格した。最初のsource archive検査は新規Schema／fixtureがGit index未登録で失敗したが、stage後の`python -X utf8 tooling/packaging_portability_check.py`は合格した。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`：exit 0。登録済みPython／Windows検査は成功し、formal Windows evidenceがない既存release blockerと`release_ready=false`はそのまま残る。
- Windows installed product上のOwner No／Yes、実Manifest file、完了Auditに対するformal LIVE_RUNTIME evidenceは取得していない。OwnerのYes操作も実施していない。

`rev2_export_owner_ui_authority_path`はUI未接続ではなくinstalled productのformal LIVE_RUNTIME evidence未成立を理由とする`release_blocker`のまま維持する。`rev2_module_pruning_binary_and_measurement`も実package、binary pruning、安全Core保持、独立Runtime、性能測定が未成立のため維持する。`windows_rust_integration_test_execution_policy`はApplication Controlによる実integration test拒否と、修正後test未実証を理由に維持する。Release statusは`release_ready=false`。

## D4 Pocket rev2 Flutter子process環境分離とRust全target再確認（2026-09-26）

作業開始時の`main`はcleanで`origin/main`と一致する`c0c50c39a31da352cc2fbc3ce9fa4ef32b0c5691`だった。2世代rollback refは`codex/backup-main=c0c50c39a31da352cc2fbc3ce9fa4ef32b0c5691`、`codex/backup-main-prev=405de8a608e6ec48a2f82882b4502ccbb4db12fb`で、remote backup tagも同じhashを指す。Ownerはpushとcleanupを禁止しているため、この追補を含む変更はlocal作業状態に保持し、push、temp／build削除、branch cleanupは行わない。

再監査でRust起動器の`Command`が親process環境を継承していた点を確認した。起動器はFlutter child環境を`env_clear()`し、`APPDATA`、`LOCALAPPDATA`、`PROGRAMDATA`、`SYSTEMDRIVE`、`SYSTEMROOT`、`TEMP`、`TMP`、`USERPROFILE`、`WINDIR`だけを許可し、起動ごとの`GUI_SHELL_BROKER_CHANNEL_PIPE`だけを別途設定する。`PATH`、資格情報候補、Codex環境、Broker runtime／endpoint／session環境値は継承しない。Windows `cmd.exe`を実際に起動するRust unit testへ合成markerを置き、子環境にOS必須値とpipe札があり、credential・Codex・PATH・Broker path markerがないことを観測する。Dart、Runner、Authority、IPC protocolは変更していない。

MethodChannel／PID-bound pipeは現行実装済みのため再実装せず、`desktop-broker-channel.md`、`windows-desktop-launcher.md`、ROADMAP、GUI操作面台帳を同期した。ConformanceはRust実起動経路が環境をclearしてallowlist＋pipeだけを設定すること、negative marker試験があること、旧いDart直Socket/fileやendpoint環境引渡しの記述が再導入されないことを検査する。`rev2_flutter_broker_channel_boundary`の正式clean product smoke／failure coverage／first-run・Setup Doctor evidenceは未成立であり継続する。

検証結果:

- `python -X utf8 tooling/schema_check/check_schemas.py`：Schema 136件、正常example 136件、negative fixture 168件でPASS。
- `python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`：215項目が合格。Broker子プロセス環境の許可リストと、旧契約記述の再導入防止を含む。
- `python -X utf8 tooling/日本語基底監査.py --strict`：未解消の日本語負債0件、違反0件で合格。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml -- --test-threads=1`：現sourceで全target成功。Rust library 279件、helper CLI 9件、integration 36件、合計324件が成功し、失敗0件。起動器binaryとdoc testは各0件で正常終了した。子process環境分離testもこの全件実行に含む。
- 全対象一括試験の合格前に行ったライブラリ再試行では、A2A BrokerのTCP折返し通信試験が`a2a_connection_failed`／`A2A Agent Card bodyを読めない`で2回、MINIDORAのTCP折返し通信試験が`対話失敗::通信失敗`で1回失敗した。各対象を単独実行した結果は合格し、その後の全対象一括試験も合格した。失敗時の低層OS通信基盤のエラー種別を取得できず、根本原因を特定したとは主張しない。この実行揺らぎは`RELEASE_CHECKLIST.md`の既知制約へ記録し、再発時にOS側のエラー種別を採取する。Application Controlのエラー4551は今回の最終全対象実行では再現しなかったため、`windows_rust_integration_test_execution_policy`を解決済み／無効へ更新した。これはローカル試験実行gateの解消であり、release evidenceではない。
- `rustfmt --edition 2021 --check native/rust_helper/src/desktop_launcher.rs`：整形検査は不合格。ファイル全体に既存の広範な整形差分があるため、今回の変更箇所以外は一括整形しなかった。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`：exit 0。日本語厳格監査、Schema 136件・正常example 136件・negative fixture 168件、conformance 215件、manifest、release gate、packaging portability、release smoke、evidence bundle、runtime assertion 12件（失敗0件）、C32開発監査が合格した。Evidence bundleはrelease blocker 5件を保持し、`release_ready=false`。Windows installed evidence欠落による5件のrelease blockerは継続する。
- 最終進捗追記後の`python -X utf8 tooling/日本語基底監査.py --strict`：未解消負債0件。`python -X utf8 tooling/manifest.py --write`、`python -X utf8 tooling/manifest.py --check`、`git diff --check`もすべて成功。

Windows installed productのclean-source formal smoke、Dart経路の全negative/failure実測、正常終了とfirst-run／Setup Doctor evidenceは未確認で、`rev2_flutter_broker_channel_boundary`を`release_blocker`のまま維持する。正式distribution、Module Pruning最終artifact検証、Mobile実機／iOS、owner GO等の既存blockerも変わらず、`release_ready=false`である。

## D4 Pocket rev2 Windows Export開発bundle構築経路（2026-09-26）

GUI-Shell ExportのManifest file生成後に、Broker Export ReceiptとManifest fileを検証してWindows向けportable開発bundleを組み立てる開発専用経路を追加した。Receipt／Manifestのschema・ID・filename・byte length・SHA-256・module planを照合し、sourceはcleanでpush済みの`main`と`origin/main`一致を要求する。source snapshot、Flutter build、Cargo build、依存cacheはrepository／OneDrive外の一時領域へ分離する。FlutterとRust子processへ渡す環境変数を許可list化し、出力は同一volume上のstagingで検証してから新規pathへ確定する。Build receiptはartifact inventoryとtree hashを記録し、実行・runtime接続・binary pruning・credential scan・正式配布が未検証である状態を明示する。

これは実際のBrokerによるOwner承認済みReceiptではなく、repository内のsynthetic example fixtureを使う開発build経路である。最初のclean pushed commitからの実構築ではFlutter Windows Releaseが77.7秒で成功した一方、Cargo linkerの出力pathが260文字に達し、MSVC `link.exe`が中間実行fileを開けずLNK1104で停止した。TemporaryDirectoryの後片付けもWinError 145で失敗し、当該実行が作った一時Cargo build directoryが`%TEMP%`に残った。App／Audit identity pairのSHA-256を使う短いCargo target path、短いcache名、linker出力path 240 UTF-16文字の事前上限検査を加え、commit `72ad51b4282f04853852c9bc046fbfcbc27e697b`から再実行した。Flutter Windows Releaseは77.3秒で成功し、前回のLNK1104は再発しなかった。その後Cargoは`proc-macro2` build script実行時に`Windows Application Control`からOS error 4551で拒否され、Cargo全体がexit 101で停止した。新しい短いpath上でのhost実行許可が得られず、portable bundleは生成されていない。短いtarget pathは以前の260文字障害を避けたが、host実行policyを弱めたり、拒否された実行fileを別pathへ移動したりする回避は行わない。先行失敗の一時Cargo build directoryは削除操作が環境policyで拒否され、`%TEMP%`に残存している。2回目の一時build rootと未完成outputは残存していない。fixture／Flutterのみの成功はOwner authority、LIVE_RUNTIME、credential非混入、standalone app、binary pruning、release readinessの証拠にならない。Cargo構築、実際のBroker Receipt、Credential監査、正式identity、署名、installer、実runtime／独立Audit、module pruningは未成立で、対応する`release_blocker`と`release_ready=false`を維持する。

実行済みの局所検証（実build前）:

- `python -X utf8 tooling/schema_check/check_schemas.py`：Schema 137件、正常example 137件、negative fixture 169件でPASS。
- `python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`：216 checksでPASS。Receipt／Manifestの不一致、credential・authority・permission・approval・audit・signature・runtime・pruning・launch成功の虚偽主張、manifest／artifact改変、重複JSON key、危険なarchive path、OneDrive出力を拒否する。
- `python -X utf8 tooling/日本語基底監査.py --strict`：負債0件、finding 0件でPASS。
- `python -X utf8 tooling/manifest.py --check`：staged source 1,042件のhash manifest検査でPASS。
- `python -X utf8 tooling/packaging_portability_check.py`：source package portability検査でPASS。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`：exit 0。登録済みPython／Windows開発検査はPASS。Windows formal installed evidence等の既存blockerは保持し、`release_ready=false`。
- `python -X utf8 -m py_compile tooling/export_windows_product.py tooling/conformance_tests/run_conformance_skeleton.py`、`python -X utf8 tooling/export_windows_product.py --help`、`git diff --cached --check`：全項目合格。
- `Rust`／`Flutter`の実装は変更していない。変更を含まない`origin/main`を基点とするWindows向け出力物の構築、生成物の起動、認証情報実値が配布物へ混入していないかの調査は未実行であり、本記録時点では未検証。

## D4 Pocket rev2 Export identity形式の契約同期（2026-09-26）

現行BrokerはApp IDを`d4-pocket-app-`＋小文字hex 32桁、Audit store IDを`audit-store-`＋小文字hex 32桁として生成する。一方、Receipt／Manifest fileが共有するSchemaは両方を任意文字列として許し、valid exampleのAudit store IDもBroker生成形式と異なっていた。Schemaへ両形式を固定し、valid example、日本語のExport意味正本、ReceiptおよびManifest file双方のnegative Conformanceを同期した。大文字hex App IDと不定形式Audit store IDを拒否する。

この変更はJSON形状の検査だけであり、IDの出所、Owner操作、暗号学的真正性、IDの実行時利用を証明しない。権限・Runtime・Manifest file生成経路は変更していない。独立Export artifact、別Runtime／Audit store、Module pruning、製品性能は依然未成立であり、`rev2_export_owner_ui_authority_path`と`rev2_module_pruning_binary_and_measurement`は`release_blocker`、`release_ready=false`を維持する。

検証結果:

- `python -X utf8 tooling/schema_check/check_schemas.py`：Schema 136件、正常example 136件、negative fixture 168件でPASS。
- `python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`：215 checksでPASS。ReceiptとManifest fileのApp ID／Audit store ID不正形式を個別に拒否した。
- `python -X utf8 tooling/日本語基底監査.py --strict`：負債0件、違反0件でPASS。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml 書出しは新規identityと監査storeを生成するが権限を継承しない`：Brokerが実生成した両識別子の形式検査1件PASS。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml -- --test-threads=1`：library 279件、helper CLI 9件、integration 36件、全324件PASS。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`：exit 0。統合されたPython-only検証10項目PASS。Evidence bundleは既存release blocker 5件と`release_ready=false`を維持した。
- `rustfmt --edition 2021 --check native/rust_helper/src/broker/export_center.rs`：不合格。今回の追加箇所はrustfmt出力と一致させたが、対象fileの既存箇所に広範な整形差分が残るためfile全体の一括整形は行わなかった。

## D4 Pocket rev2 Compose UI参照入力とPreview比較元（2026-09-26）

ComposeのRuntime／Agent／Tool／MCP参照IDを設定画面の固定値から改行区切り入力へ変更する。これらはManifest内の未解決参照であり、候補発見、存在・接続・信頼の確認、権限付与を行わない。ComposeをBrokerが受理した後、同画面を開いている間だけそのManifestをPreview比較元として保持し、拒否された要求では比較元を更新しない。これは保存、履歴、Approval、復旧、Authorityの仕組みではない。

検証結果:

- `flutter test --no-pub --no-test-assets`（`apps/desktop_flutter`）：109 tests PASS。Compose参照ID draft、Compose送信配列、Preview比較元の転送を含む。
- `flutter analyze --no-pub`（`Z:\apps\desktop_flutter`）：No issues found。`flutter analyze --no-pub`（`Z:\apps\mobile_flutter`）：No issues found。日本語を含む元のOneDrive pathではAnalysis ServerがLSP JSON `FormatException`で終了したため、一時`subst Z:`経由で実行し、検証後に割当を解除した。workspace内容・ACLは変更していない。
- `python -X utf8 tooling/日本語基底監査.py --strict`：負債0 files／0 findingsでPASS。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`：exit 0。Schema 137、正常example 137、negative fixture 169、Conformance 216 checks、Manifest、release gate、packaging portability、release smoke、evidence bundle、runtime assertion、C32開発監査がPASS。
- `MANIFEST.sha256.json`を更新し、統合検証内の`manifest.py --check`がPASS。

通常のFlutter test commandでは生成用`build/unit_test_assets`の削除がWindows ACL／reparse pointで拒否された。test assetを無効にした同一Desktop test suiteは全件PASSした。製品のSchema、Broker、Rust権限経路は変更していない。既存のWindows installed evidence等の`release_blocker`と`release_ready=false`は維持する。本単位はManifest編集・Preview改善であり、Compose／Previewのproduction接続完成や独立App配布を意味しない。

## D4 Pocket Windows Export短縮scratch再試行（2026-09-27）

前回の長い一時pathでのMSBuild失敗を切り分けるため、Repository／OneDrive外の短い`%USERPROFILE%\D4PB`を専用scratchにし、当該PowerShell processだけ`TEMP`／`TMP`を`%USERPROFILE%\D4PB\t`へ向けて、clean pushed commit `d05eae4be26e73f0866cce220b6f43cfc52b10fc`からExport buildを再試行した。合成valid fixtureを用いたため、実Broker生成ReceiptやOwner操作の証拠ではない。

- `git fetch --prune origin`後、local `main`、`origin/main`、remote HEADはすべて`d05eae4be26e73f0866cce220b6f43cfc52b10fc`で一致し、working treeはcleanだった。
- `python -X utf8 tooling/export_windows_product.py --receipt <scratch receipt.json> --manifest-file <scratch d4-pocket-app-11111111111111111111111111111111.json> --output-dir <scratch output>`：Flutter `pub get`とWindows Release buildは成功し、実行時間は約80秒。MSB8029警告はあったがFlutter executableを生成した。その後Cargoは`io-lifetimes`と`proc-macro2`のbuild script executableを起動できず、Windows Application ControlのOS error 4551でexit 101。tool全体はexit 1で停止し、portable outputは生成されなかった。
- 短いprofile直下pathでFlutter buildが完了したため、今回のCargo停止は先行LNK1104のpath長障害とは別である。Cargo targetや拒否されたexecutableの移動、Application Control policy変更は行っていない。Cargoの一時source／targetはtool終了時に片付き、残存する専用scratchにはfixture入力と空のMSBuild管理directoryだけがある。
- 出力は未署名bundleの構築段階にも到達せず、Credential scan、Manifest runtime消費、binary pruning、起動、配布、formal LIVE_RUNTIMEを証明しない。`rev2_export_owner_ui_authority_path`と`rev2_module_pruning_binary_and_measurement`、`release_ready=false`を維持する。

## D4 Pocket Phase 7 Agent比較projectionのfail-closed同期（2026-09-27）

現行Flutterの比較表示判定を`agent_comparison.schema.json`と照合したところ、2件未満だけを拒否し、9件以上や空Workspace参照を比較可能としていた。またWorkspace文字列の相違だけで「Workspace隔離を確認済み」と表示していた。Desktop側projectionを2〜8件、比較ID形式、空でないWorkspace参照、Session ID／Workspace参照の重複検査へ同期し、異なる参照は実行時隔離の証明ではないと表示・仕様へ明記した。Agent実行、Workspace作成、Broker権限経路は追加していない。

- 正常／負例: 2件の異なる参照はprojection対象になるが、同一Session、同一Workspace参照、空のWorkspace参照、不正Session ID、9件は拒否する。Handoff表示のAuthority再評価要求と秘密markerの既存testも維持する。
- 初回の厳格日本語監査はID検査用正規表現リテラルをDart画面文言として1件検出した。監査例外を追加せず、同じASCII ID規則をcode unit判定で実装し直した。最終の`python -X utf8 tooling/日本語基底監査.py --strict`は負債0／finding 0でPASS。
- `flutter test --no-pub --no-test-assets`（Desktop、Z:一時alias）：110件PASS。`flutter analyze --no-pub`（Desktop／Mobile、同alias）：両方No issues found。`subst Z: /D`でalias解除を確認した。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`：exit 0。Schema 137、正常example 137、negative fixture 169、Conformance 216、Manifest、release gate、packaging portability、release smoke、evidence bundle、runtime assertions、C32監査を含む登録済みlocal検証がPASS。release blockerと`release_ready=false`は維持する。
- 証拠境界: これはFlutterのsnapshot projectionと表示の検査であり、Agent比較実行、path alias／junctionを含むWorkspace実隔離、実Handoffを証明しない。実Agentを隔離Workspaceで起動・比較するBroker経路とtarget側の再評価／失敗隔離／Recoveryは`release_blocker`として継続し、snapshot文字列だけで隔離を主張しない制限は`known_limitation`である。

## D4 Pocket Phase 7 比較ContractのAgent識別分離（2026-09-27）

比較projectionのsemantic Conformanceに、Session／Workspaceの一意性だけでなくAgent runtime IDの一意性を追加した。別Session・別Workspaceを用意しても同じAgent runtimeを複数Agentとして水増しする入力を拒否する。配列entry間の関係であるためSchema形状は変えず、`docs/specs/agent-coordination.md`にConformanceおよび将来Broker経路での検査責任を記録した。

- `python -X utf8 -m py_compile tooling/conformance_tests/run_conformance_skeleton.py`：合格。
- `python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`：Conformance 216件で合格。同一Agent実行系、Workspace、Sessionの重複、Authorityの再利用、取得不能値を0で補う入力を拒否する。
- `python -X utf8 tooling/schema_check/check_schemas.py`：Schema 137、正常example 137、negative fixture 169で合格。Conformanceのみの意味制約のためSchema件数・fixture形状は変更していない。
- 証拠境界: この検査は比較記録の識別値を照合するだけであり、Agent起動、別実Workspaceの作成、並列実行時に一方のAgentの状態が他方へ混入しないこと、実結果の比較を証明しない。実Agent比較と隔離Broker経路の`release_blocker`は維持する。

## D4 Pocket Phase 7 Agent対話セッションmetadata投影（2026-09-27）

現行BrokerとDesktopの接続を確認したところ、Rust BrokerはAgent metadata一覧と対話開始を扱うが、DesktopのAgent Centerに表示するBroker由来session一覧が未接続だった。通常認証IPCに空payloadの`対話セッション一覧`を追加し、現在Broker内で保持するsessionのうち、登録済みAdapter metadataが既存`agent_adapter` Schemaに適合するAgent runtimeに結び付くものだけを、最大64件で返す。Agent metadataは表示上の分類にだけ使用し、Trust、Permission、Approval、Authorityを生成しない。一般Runtimeや未登録RuntimeのsessionはAgent表示へ混ぜない。

一覧fieldはsession ID、runtime ID、状態、作成監査IDに限定する。Brokerの既存対話開始監査が成功した後に監査IDをsessionへ保持し、開始応答とその既存Audit hash射影は変更しない。Desktopは応答operation・`INTERNAL_STATE`証拠種別・Audit参照・版・field集合・識別子・状態・最大件数を検査してからAgent Centerへ投影する。Workspace bindingがないためTask、差分、Tool、command、承認数、比較、Handoffは表示・利用しない。通常Mobile Device Link allowlistには追加しない。

本経路はmetadata読取だけであり、Adapter invocation、Agent起動、Task実行、外部Runtime health、Workspace隔離を証明しない。監査IDはBroker内の開始監査への参照であり、client側の連鎖検証証拠ではない。証拠種別は`INTERNAL_STATE`。fixtureを用いた自動試験結果を`LIVE_RUNTIME`へ昇格しない。

検証結果:

- `python -X utf8 tooling/schema_check/check_schemas.py`：Schema 138件、正常example 138件、negative fixture 170件で合格。
- `python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`：217件のcheckが合格。Task混入、未知field、Authority、Workspace、状態、監査参照、重複ID、件数上限、Mobile経路への混入を拒否する。
- `python -X utf8 tooling/日本語基底監査.py --strict`：最初の実行は新規Rust試験の英語診断2行を検出して失敗。診断を日本語化し、最終実行は負債0件／finding 0件で合格。監査例外は追加していない。
- Desktop `flutter test --no-pub --no-test-assets --concurrency 1 --reporter failures-only`：113件合格。Broker応答解析、異常field・重複ID・上限超過・証拠種別不一致拒否、未結合WorkspaceでTask／比較／Handoffを表示しないことを含む。
- `flutter analyze --no-pub`：DesktopとMobileの両方で指摘なし。日本語OneDrive pathによるAnalysis ServerのJSON parse問題を避ける一時`Z:` aliasから実行し、検証後に割当を解除した。Mobile source／権限経路は変更していない。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib -- --test-threads=1`：288件合格。通常Broker経路、Agent metadata分類、一般／未知Runtime除外、64件上限、監査参照、既存開始Audit hash非変更、Mobile拒否を含む。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml -- --test-threads=1`：exit 101。library 288件、CLI 9件、Broker IPC 10件、canonical hash 1件、checkpoint 8件は合格したが、`protected_data` integration executableをWindows Application ControlがOS error 4551で起動前に拒否した。`protected_startup`、`workspace_startup`の個別起動も同じ拒否。`protected_store` 3件、`workspace_diff` 2件、`workspace_reader` 2件は個別実行で合格した。拒否されたexecutableの移動・再配置、Application Control変更はしていない。
- 新しい試験の初回全体実行では既存Mobile投影試験が一般Runtimeを指定していたため、Agent session一覧が正しく空になり、その試験の期待値が不一致だった。試験をschema適合Agent Adapterへ結び直し、通常Mobile側での拒否も含めて修正版Library 288件とBroker IPC 10件が合格した。
- `python -X utf8 tooling/manifest.py --write`でtracked source 1045件を反映し、`python -X utf8 tooling/manifest.py --check`は合格。追跡対象を含むportable source archiveでの`python -X utf8 tooling/packaging_portability_check.py`も合格した。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`：exit 0。10の登録済みPython／Windows開発検査が成功し、Evidence bundleは`release_ready=false`を維持した。`git diff --check`も合格。合成smoke／fixtureは製品実行証拠へ昇格しない。

新たなOS error 4551を受け、`windows_rust_integration_test_execution_policy`を`release_blocker`／unresolvedへ戻し、`RELEASE_CHECKLIST.md`とregistryを同期した。これは開発停止理由ではなく、全target検証の未成立を表す。Windows Application Controlを弱めず全Rust targetを実行できる承認済み環境での再検証が必要であり、他工程は継続する。ほかのrelease blockerも維持し、`release_ready=false`である。

## D4 Pocket Phase 7 Workspace rootのAgent間重複拒否（2026-09-27）

既存Workspace registryは、異なるWorkspace ID／Agent runtime IDへroot handleを登録するとき、各rootのdevice IDとfile IDを保持し、別runtimeですでに登録された同一物理directoryを拒否する。file IDが0で未観測の場合も登録しない。実装はRust Broker WorkspaceRegistryであり、Agent Adapter metadataやFlutter stateをauthorityに使わない。owner起動登録経路は拒否をBroker Auditへ記録し、登録成功しない。

Rust unit testは同一temporary rootを別handleから二つのAgent runtimeへ登録する負例が拒否されること、および別rootなら二つ目の登録が通ることを確認する。これはregistryの`FIXTURE`相当の境界試験であり、親子directory rootの重なり、別名となるmount、実Agentの同時書込み・比較・Handoff隔離を証明しない。Workspace comparisonとHandoffは実Agent経路が成立するまで未接続である。`comprehensive_extension_rev1_completion`は`release_blocker`のまま維持し、`release_ready=false`である。

検証結果:

- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib -- --test-threads=1`：最初と最後の全library実行は289件合格。間の一括実行と単独実行では既存A2A loopback Agent Card fixtureが`a2a_connection_failed`となったが、続く単独実行と全target実行は合格した。失敗時の低層OS通信errorは採取できなかったため、この揺らぎを`RELEASE_CHECKLIST.md`へ`known_limitation`として記録した。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --quiet -- --test-threads=1`：exit 0。ライブラリ／実行ファイル／結合試験の全対象334件が合格し、Windows Application ControlのOSエラー4551は再発しなかった。`windows_rust_integration_test_execution_policy`はローカルRust全対象試験の実行関門を解消状態へ戻した。これは製品runtimeやrelease証拠ではない。
- `python -X utf8 tooling/schema_check/check_schemas.py`：Schema 138件、valid example 138件、negative fixture 170件で合格。
- `python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`：217件のcheckが合格。
- `python -X utf8 tooling/日本語基底監査.py --strict`：負債0件／finding 0件で合格。初回はこの進捗記録とregistryの非日本語diagnostic語句を2件検出したため日本語化し、監査例外を追加せず再実行で解消した。
- `python -X utf8 tooling/manifest.py --write`：tracked source 1045件を更新し、`python -X utf8 tooling/manifest.py --check`が合格。
- `python -X utf8 tooling/packaging_portability_check.py`：portable source archiveを展開し、内包Manifest／Conformance／release gate検査を含めて合格。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`：exit 0。10の登録済みPython／Windows開発検査がすべて成功し、Evidence bundleは既存release blocker 5件と`release_ready=false`を維持した。release smoke等のsynthetic結果はWindows installed productや実Agent実行の証拠へ昇格しない。
- `git diff --check`および`git diff --cached --check`：commit前の最終差分確認で実行し、結果を記録する。

## D4 Pocket Phase 7 Agent Workspace親子root重複拒否（2026-09-27）

Workspace起動時に、nofollowで開いたvolume／rootから指定directoryまでの各(directory device ID, file ID)を取得し、同じpathを二度解決した際にfilesystemと識別列全体が一致することを確認するよう拡張した。WorkspaceRegistryは識別列をregistration hashへ含め、異なるruntime間では同一rootだけでなく通常pathで観測した親子rootも拒否する。親→子・子→親の双方を拒否し、独立rootは受理する。既存のhandle-only登録に完全な祖先列がない場合、他runtimeと併存する登録をfail-closedで拒否する。Owner起動登録の親子拒否はBroker Auditへ記録され、失敗後にWorkspace一覧へrootを残さない。

根拠はRust test process内のtemporary filesystemとBrokerを用いた`FIXTURE`検査であり、製品Runtimeの実Agent間書込み隔離を証明しない。通常path上のidentity列から外れるbind mount等の別名範囲は網羅しない。実Agentの比較・Handoff・cross-agent contamination試験も未成立のままであり、`comprehensive_extension_rev1_completion`は`release_blocker`、`release_ready=false`を維持する。

検証結果:

- `cargo check --locked --manifest-path native/rust_helper/Cargo.toml --tests`：成功。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib 別Agentの同一rootと親子rootを拒否し独立rootを許可する -- --test-threads=1`：1件成功。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib workspace_startup_rejects_parent_child_roots_and_records_rejection_audit -- --test-threads=1`：1件成功。
- A2A loopback Broker projectionの単独再実行：1件成功。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml`：exit 101。library 290件中288件成功、loopback HTTP試験2件が通信読取で失敗。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml -- --test-threads=1`：library 290件成功。その後のdesktop launcher test executableはWindows Application ControlにOS error 4551で起動前に拒否され、全targetは未完了。実行fileの移動・再配置やApplication Control変更は行っていない。`windows_rust_integration_test_execution_policy`を未解決へ戻し、release checklist／registryと同期した。
- 最終codeで`cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib -- --test-threads=1`を再実行し、290件すべて成功した。追加したroot identityだけの不完全祖先列拒否もこのlibrary試験に含む。
- 続けて最終sourceで`cargo test --locked --manifest-path native/rust_helper/Cargo.toml -- --test-threads=1`を再実行し、exit 0。library 290件、CLI 9件、Broker IPC 10件、canonical hash 1件、checkpoint 8件、protected data 2件、protected startup 1件、protected store 3件、workspace diff 2件、workspace reader 2件、workspace startup 7件の全335 testが成功した。desktop launcher test binaryは正常起動し0件、doc-testも0件で正常終了。先行runのOS error 4551は再発せず、実行fileの移動・再配置やApplication Control変更は行っていないため、`windows_rust_integration_test_execution_policy`をresolved／inactiveへ戻した。
- 既知のloopback揺らぎを`RELEASE_CHECKLIST.md`に更新記録した。並列失敗と直列library成功の因果は未確認であり、実Runtime接続証拠ではない。
- `python -X utf8 tooling/schema_check/check_schemas.py`：Schema 138件、正常example 138件、negative fixture 170件で合格。`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`：217件合格。
- `python -X utf8 tooling/日本語基底監査.py --strict`：負債file 0、finding 0で合格。`python -X utf8 tooling/manifest.py --write`はtracked source 1045件を書込み、`--check`も合格。
- `python -X utf8 tooling/packaging_portability_check.py`：合格。`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`：exit 0、登録済み10検査すべて合格。Evidence bundleは他の未解決release blockerにより`release_ready=false`を維持する。このPython-only集約自体はRust試験を含まず、別記した全target 335件の結果を置き換えない。
- `python -m json.tool release_blockers.registry.json`と`git diff --check`：合格。

## D4 Pocket Phase 7 loopback HTTP fixtureのreset調査（2026-09-27）

Windows上のRust loopback HTTP試験が間欠的に`ConnectionReset`となる現象を調査した。製品transport、read/write timeout、retry、Authority pathは変更していない。

test fixture側で確認した欠陥を修正した。MINIDORA fixtureは要求を1回だけreadして残りを無視していたため、boundedなread timeoutを設定し、HTTP header終端まで要求を読むようにした。A2A moduleとBrokerのfixtureでは、同じbounded request readに加え、header/bodyを一括送信し、応答側write半閉鎖後にclient closeを待つ。A2A module fixtureはclient結果が失敗してもserver workerの結果を先に回収する。

失敗観測では、fixture改善前のA2A body応答でheader 91 byte受信後にreset、header受信前のresetもあった。fixture改善後も単独A2A反復100回中1回が`a2a_connection_failed`となり、全target標準並列実行でA2A module fixture 1件が失敗した。Minidoraの診断有効実行では`read_headers / ConnectionReset`を採取した。A2A Broker fixtureの不足を修正した後は同Broker test単独と直列全targetが通過した。従ってtest fixtureの欠陥は一部解消したが、Windows loopback resetの残存発生源は未確定であり、`RELEASE_CHECKLIST.md`の`known_limitation`を維持する。失敗を直列化、retry、結果抑制で隠さず、fixture成功を実外部Agent接続の証拠へ昇格しない。

検証結果:

- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib 'broker::a2a_center::tests::owner接続をBrokerで受理し通常IPC一覧へbounded射影する' -- --exact`：1件成功。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --quiet -- --test-threads=1`：exit 0。全335件（library 290、CLI 9、Broker IPC 10、他integration 26）が成功。
- 修正後の標準並列 `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --quiet`：exit 101。library 290件中289件成功、A2A module loopback fixture 1件が`a2a_connection_failed`。並列全target成功とは扱わない。
- `cargo fmt --manifest-path native/rust_helper/Cargo.toml -- --check`：不合格。変更対象外を含むcrate内の多数の既存fileで整形差分を検出したため、一括整形は行わず、今回のfixture変更file以外へ波及させていない。
- `python -X utf8 tooling/schema_check/check_schemas.py`：Schema 138件、valid example 138件、negative fixture 170件で合格。`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`：217件合格。
- `python -X utf8 tooling/日本語基底監査.py --strict`：最初はfixture診断の英語2件を検出して不合格。両方を日本語化した後の最終runは負債0／finding0で合格。
- `python -X utf8 tooling/manifest.py --write`：tracked source 1045件を書込み、`--check`合格。`python -X utf8 tooling/packaging_portability_check.py`も合格。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`：exit 0、登録済み10検査が成功。Evidence bundleは`release_ready=false`を維持し、合成smokeはinstalled productの証拠ではない。`python -X utf8 tooling/release_gate_check.py`、registry JSON検証、`git diff --check`も合格。
- この作業単位ではproduction runtimeを変更せず、既存release blockerおよびWindows並列loopback試験の`known_limitation`を維持する。

## Windows短縮path開発checkoutの全体再検証（2026-09-27）

OneDrive配下の長い日本語pathで確認されたFlutter解析・test cleanup問題を切り分けるため、`C:\D4Pocket`の独立Git cloneで現行`main` commit `1fbfabca78d05b9f0be1e1a5b62509d86646b634`を再検証した。OneDrive checkoutやACLは変更していない。短縮pathではFlutter解析とUI／client test群が進行したが、Windows Application ControlのOS error 4551は引き続きRust executableの起動を拒否した。したがって短縮pathはOneDrive path固有の問題を避ける開発場所として有効だが、全環境問題を解消したとは扱わない。

- `python tooling/validate_all.py --python-only --desktop-platform windows`（作業dir `C:\D4Pocket`）：exit 0。厳格日本語監査、Schema 139件、正常example 139件、negative fixture 173件、Conformance 217件、Manifest、release gate、packaging portability、release smoke、evidence bundle、runtime assertion、C32監査を含む登録済み検査が合格。Windows installed-evidence上の5項目をrelease blockerとして検出し、`release_ready=false`。
- `flutter analyze --no-pub`（`C:\D4Pocket\apps\desktop_flutter`）：`No issues found`。
- `flutter test --no-pub`：113件中111件合格、2件失敗。`runtime_lifecycle_test.dart`と`workspace_inspector_test.dart`の失敗は実Broker連携に必要なRust helperがbuildできず起動できないためで、helper未生成を検出した。これはDesktop suite全体の合格ではない。
- `cargo test --locked`（`C:\D4Pocket\native\rust_helper`）：exit 1。`generic-array`および`io-extras` build script executableがWindows Application ControlにOS error 4551で起動拒否され、Rust全target試験へ到達しなかった。実行fileの移動・再配置、policy変更、試験除外はしていない。
- 証拠境界: Pythonの統合検査はCONFIG／FIXTUREを中心とする開発検査であり、Rust全target実行、installed product、実Agentの書込み隔離、release readinessを証明しない。Windows Rust全targetの`release_blocker`は未解決のまま維持する。短縮cloneはOneDrive checkoutのACL修復や移行ではない。

## Agent Task用Workspace PermissionのOwner発行経路（2026-09-27）

Agent Task用Workspace Permission要求／receipt Schemaと正常・権限昇格負例を追加し、正本索引、Schema検査、Conformanceへ登録した。Rust BrokerはTask専用PermissionをRuntime／現行Session／Workspace登録hashへ束縛し、Rust Desktopのnative Owner確認経路からのみ、固定operation `agent_task.execute`、Session・Workspace限定、5分、1回で発行する。通常IPCやrequest由来のpath／command／Permission scope／期限は拒否する。active grantの重複発行を拒否し、Session終了・隔離とBroker再起動で揮発Permissionを失効させる。発行Auditに失敗した場合はPermissionを取り消す。

この作業単位はPermission発行までであり、Task executorはまだ未接続である。Permission発行だけではTask本文を承認せず、保存、process起動、Workspace書込を行わない。実行時の本文hash・条件へ結合する独立Owner Approval、実行前後AuditEvent、RecoveryAction、diff保存、read-only Codex Adapterの書込化は未成立で、`comprehensive_extension_rev1_completion`を`release_blocker`のまま維持する。

検証証拠:

- `python -X utf8 tooling/schema_check/check_schemas.py`：Schema 142件、正常example 142件、negative fixture 176件で合格。
- `python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`：219件合格。
- `python -X utf8 tooling/日本語基底監査.py --strict`：負債file 0、finding 0で合格。
- `cargo check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`：全targetのcompile確認に成功。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib Agent作業要求検査は現行SessionとWorkspaceだけを照合し本文を露出せず未実行を明示する -- --test-threads=1`：1件合格。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib AgentTaskPermissionのnative確認は固定範囲を示しTask本文と追加権限を拒否する -- --test-threads=1`：1件合格。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml -- --test-threads=1`：library 300件は全件合格。その後の`gui_shell_desktop_launcher` test executableはWindows Application ControlのOS error 4551で起動前に拒否され、Cargo全targetは未完了。policy変更、試験除外、実行fileの移動・再配置はしていない。
- `python -X utf8 tooling/manifest.py --write`：追跡source 1059件を収録。`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`はexit 0で登録済み10検査を通過し、portable source、Manifest、release gateも合格。Evidence bundleは5件のrelease blockerを保持し、`release_ready=false`。

静的compile、fixture、focused test、library testはそれぞれの範囲だけを証明する。全Rust target、Windows installed product、実Agent Taskの書込隔離・失敗回復・cross-agent contamination、正式releaseは証明していない。

## Agent Task検査への現行Permission状態投影（2026-09-27）

Permission発行後もAgent Task検査が常に「未付与」を返す不整合を修正した。Brokerは検査ごとに、揮発Permissionの壁時計期限と単調時計期限、Runtime ID、Session ID、Workspace ID、現在のWorkspace登録hashを照合し、有効／未付与だけを返す。Permission IDは応答へ出さず、Task状態は未実行、Task Approvalは未取得のまま保つ。Broker内状態の検査であり、Task実行やWorkspace隔離の証拠ではない。

- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib Agent作業要求検査は現行SessionとWorkspaceだけを照合し本文を露出せず未実行を明示する -- --test-threads=1`：1件合格。Owner-native発行後はPermission状態が有効、期限境界では未付与となることを含む。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml -- --test-threads=1`：Rust library 300件は全件合格したが、その後`gui_shell_desktop_launcher` test executableがWindows Application ControlのOS error 4551で起動前に拒否され、全targetは未完了。回避操作や試験除外はしていない。
- `cargo check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`、Schema検査（142件）、Conformance（219件）は合格。

残るTask executor、独立Owner Approval、実行前後Audit、Recovery、diff保管、実Agent隔離は`release_blocker`。Windows全target試験もOS実行制御により未確認である。

## Agent Broker操作の共通IPC Schema登録（2026-09-27）

現行Rust Broker protocolには`Agent一覧`、`Agent作業要求検査`、`AgentTaskWorkspacePermissionGrant`が実装されている一方、共通IPC要求／応答Schemaのoperation enumに同じ3操作が欠けていた。両Schemaへ登録し、当該3操作がRust protocolと要求／応答Schemaの三面で宣言されていることをConformanceへ追加した。Runtime・権限・Permission発行挙動は変更していない。

- `python -X utf8 tooling/schema_check/check_schemas.py`：Schema 142件、正常example 142件、negative fixture 176件で合格。
- `python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`：220 checksで合格。
- 追加した`test_agent_broker_operations_are_declared_in_ipc_contracts`：合格。
- `python -X utf8 tooling/日本語基底監査.py --strict`：負債file 0、finding 0で合格。
- `python -X utf8 -m py_compile tooling/conformance_tests/run_conformance_skeleton.py`、`python -X utf8 tooling/manifest.py --check`、`git diff --check`：合格。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`：exit 0。登録済み開発検査は合格。Evidence bundleは対象範囲のWindows release evidence不足5件を報告し、`release_blockers.registry.json`のactive blocker 15件は未解消、`release_ready=false`を維持。

これはIPC Contractとの同期を示し、Agent稼働、Task実行、Workspace隔離、Owner Approval、release readinessを証明しない。これらの既存`release_blocker`と`release_ready=false`は維持する。

## Agent Task Owner Approvalのnative発行・preflight結合（2026-09-27）

`AgentTaskOwnerApprovalGrant`をBroker IPC operation、Rust Desktop native Owner確認allowlist、IPC要求／応答Schemaへ追加した。発行は既存Workspace Permissionが有効な現行Runtime／Session／Workspace結合に限定し、Brokerが指示本文hashと、登録hash・Permission内部識別子・固定実行条件policyを含む条件hashを計算して揮発状態に5分保持する。native確認文は本文を表示せず、文字数・本文hash・対象ID・policy・未実行境界を示す。Task preflightは本文hash・条件hash・wall／monotonic期限を再照合し、一致時だけApprovalを有効表示する。要求本文、Approval ID、Permission IDは応答／Auditへ出さない。

- Rust Protocol／Broker試験で通常IPC拒否、Permission未付与時のnative発行拒否、native発行、本文・Audit非露出、preflight一致／本文差替え不一致／期限切れ、未実行状態を検査する。
- Desktop native確認候補testで本文非表示、hash／固定policy表示、request側Approval ID注入拒否を検査する。
- 発行receipt専用Schema、正常／実行済みnegative fixture、正本索引、本文・権限・pathなど禁止fieldのConformanceを追加し、`AgentTaskOwnerApprovalGrant`をIPC Schema三面同期Conformanceへ含める。
- この単位はApprovalの発行と照合のみ。consumerによる一回消費、実行直前の原子的再検証、sandbox・独立書込隔離、実行前後Audit／Recovery、結果保存は未成立で`release_blocker`を維持する。固定policy IDはsandboxの実在証拠ではない。

## Agent Task失効・再送・Session終端の回帰試験補強（2026-09-27）

既存Rust Broker挙動の回帰試験へ、Owner Approval要求の再送拒否、失効Workspace Permissionのnative再発行、Permission置換時にApprovalを継承しないこと、Session終了後のTask preflight拒否を追加した。production挙動は変更していない。

- `cargo check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`：合格。追加試験を含む全targetのcompileを確認。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib Agent作業要求検査は現行SessionとWorkspaceだけを照合し本文を露出せず未実行を明示する -- --test-threads=1`：test executableの生成後、Windows Application ControlがOS error 4551で起動を拒否。追加assertionの実行結果は未確認。policy変更、実行fileの移動、alternate targetによる回避は行っていない。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`：exit 0。Schema 143件、example 143件、negative fixture 177件、Conformance 221 checks、manifest、release gate、packaging portability、release smoke等の登録検査は合格。release evidence不足を含み`release_ready=false`を維持。

この試験補強はTask consumer、Approvalの一回消費、sandbox・独立書込隔離、実行前後Audit／Recovery、実Agent実行を証明しない。Rust動的試験の今回追加分は未実行であり、Windows Application Controlによる検証制約を維持する。

## Phase 7対応表とCodex Adapter能力宣言の現状同期（2026-09-27）

`docs/D4_POCKET_PHASE_MAPPING.md`のPhase 7記述がTask要求をBroker未接続としており、現行`ROADMAP.md`および実装と矛盾していたため修正した。現状はTask要求照合、Workspace Permission、別Owner Approvalの発行・preflight照合まで接続済みであり、原子的な一回消費、実Task実行、sandbox・隔離書込、実行前後Audit／Recovery、結果保存、比較／Handoffは未成立と分けて記載した。

Codex Adapterの`task_execution`能力宣言も`unknown`から`unsupported`へ修正した。現行Adapterはread-only対話だけを実装し、Broker統治済みの書込Task経路を持たないためである。実Agent側の機能可否やTask実行のLIVE_RUNTIME証拠へ読み替えない。

- `cargo check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`：変更を含むcompileは合格。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`：exit 0。Schema、Conformance、strict日本語監査、Manifest、release gate等の登録検査は合格。
- Rustの該当unit testは新しい`unsupported` assertionを含め未実行。Windows Application ControlのOS error 4551によりtest executable起動が阻止される既知制約のため、再試行・回避は行っていない。

この訂正はTask実行経路やsandboxの成立を意味せず、release blockerと`release_ready=false`を維持する。

## Agent Task Permission／Owner Approval発行の監査失敗負例（2026-09-27）

対話制御の単体試験へ監査callback故障を注入する負例を追加した。試験は、Agent Task用Workspace Permissionの発行監査が失敗した場合に揮発Permissionを残さないこと、Permission発行後のOwner Approval監査失敗ではApprovalを保存しないことを検査する。さらにApproval監査理由へTask本文を含めず、失敗後preflightが`Permission=有効`、`Approval=未取得`、`実行=未実行`となることを検査対象とする。

- `cargo check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`：合格。追加したRust試験を含む全targetのcompileを確認。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib AgentTask発行監査失敗ではPermissionを残さずApprovalを発行しない -- --test-threads=1`：試験実行fileのcompile後、Windows Application ControlがOS error 4551で起動を拒否。追加試験のassertionは未実行。

この試験の証拠源は`FIXTURE`であり、永続Audit storeの故障注入やBrokerのLIVE_RUNTIME障害処理を実証しない。実Task consumer、一回限りの原子的消費、sandbox・独立書込隔離、実行前後Audit／Recovery、Agent Task実行のrelease blockerは維持する。

## D4 Pocket 製品表示名の統一（2026-09-27）

Mobileの画面・Android／iOS表示名とWindows実行fileの製品表示を`D4 Pocket`へ統一した。技術基盤名は日本語説明中の`GUI Shell`として併記し、Android namespace／applicationId、iOS bundle identifier／CFBundleName、Windows executable identityは変更していない。日本語基底の局所例外台帳には製品固有名`D4 Pocket`だけを追加し、利用者向け説明は日本語を維持した。

- `flutter analyze`（`apps/mobile_flutter`、`apps/desktop_flutter`）：合格。
- `flutter test --no-pub`（`apps/mobile_flutter`）：18 tests passed。表示名と drawer の技術基盤表記も検査。
- `python -X utf8 tooling/schema_check/check_schemas.py`：schema 143件、example 143件、negative fixture 177件で合格。
- `python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`：221 checksで合格。
- `python -X utf8 tooling/日本語基底監査.py --strict`：負債file 0、finding 0で合格。

この表示名変更はplatform build、Mobile native Device Linkの実機安全性、正式配布、製品完成、owner GOを証明しない。既存のrelease blockerと`release_ready=false`を維持する。

## 現行sourceのWindows Cargo build script起動拒否（2026-09-28）

現行`origin/main`と一致するsource commit `f4bf6b0e65e751da8b662595f2e9865970c6f460`で、OneDrive外の短いmanaged worktreeからRust全target試験を再確認した。通常のCargo target出力先では`io-lifetimes` build scriptがWindows Application ControlのOS error 4551で起動前に拒否された。別のCargo target出力先`C:\D4Pocket\codex-f4-target`を指定しても、`generic-array`と`io-extras`のbuild scriptが同じerrorで拒否された。依存build scriptの実行に至らずtest executableも完遂していないため、test結果は未取得である。短いsource pathと別target directoryだけでは回復しない。

- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`（非OneDrive managed worktree）：exit 1。build script起動がOS error 4551で拒否され、全target未完了。
- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --target-dir C:\D4Pocket\codex-f4-target --all-targets -- --test-threads=1`：exit 1。`generic-array`／`io-extras` build scriptがOS error 4551で起動拒否され、全target未完了。
- 2つの失敗したCargo生成物だけを、それぞれ`cargo clean`で削除した。既存の`C:\D4Pocket` checkout内容、Windows Application Control設定、拒否された実行fileは変更していない。

過去のsource commit `2da5fd370382f2fe5acc032b8f26ac25880048ee`に対する全target 348件成功記録は履歴として維持し、現行sourceへ転用しない。最新sourceの全target検証が完遂していないため、`windows_rust_integration_test_execution_policy`を`release_blocker`／unresolvedへ戻し、registryとRelease Checklistを同期する。これは開発停止を意味せず、Rust全target試験実行環境を必要とする検証上の阻害である。`task_execution=unsupported`と`release_ready=false`を維持する。
