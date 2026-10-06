# D4 Pocket / GUI-Shell rev5 Product-First 進捗

更新日: 2026-10-06
工程正本: ユーザー提示「D4 Pocket / GUI-Shell 統合実装仕様書 rev5」「統合開発工程表 rev5」「Codex実装指示書 rev5」
現行phase: `P11 Windows Productization` (`OPEN`)。`P10 Module Selection / Pruning`は2026-10-05にrev5 Product Build受入れを閉鎖。`P9 Standalone Export`、`P8 GUI-Shell Compose`、`P7 A2A / Host / Adapter`、`P6 Credential / MCP`、`P5 Workspace / History / Evaluation`も同日に閉鎖済み。
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

### P2 複数Agent比較 — CLOSED（Product Build）

対象は同一Codex CLI／同一providerを使う2つのAgent instanceである。Agent A/Bは別のSession、Workspace、Task、実行scratchを持ち、同一Taskを独立に実行する。Vendor／Modelの種類が揃うことを待たない。

| P2受入れ条件 | 状態 | 現在確認できた実装 |
| --- | --- | --- |
| Agent A／Bを独立instanceとして用意 | PASS | Agent CenterがBrokerの別Runtime／Workspaceに結合したactive SessionをA/Bとして選択する。同じCodex CLI／providerを別Runtime IDで登録できる。Broker契約上Runtime登録はWorkspaceへ結合される |
| 同一Taskを独立Workspace／Session／Taskへ投入 | 合格（証拠区分: `FIXTURE`＝試験用模擬） | 同じ入力hashを別要求として事前検査し、Permission／Approval／Task状態をSessionごとに分離するWidget試験 |
| 2つのTaskを同時実行 | 合格（証拠区分: `FIXTURE`＝試験用模擬） | Compare調停serviceが両Taskを並行dispatch。2件の実行要求と別Task IDをWidget試験で確認 |
| result／diff／tests／duration／failure／resource／Auditの比較表示 | PASS（Product Build範囲） | Task状態・result hash・Audit参照をA/B別表示。結果本文は個別Content Exposure後に表示、変更file／diffは独立Workspace Inspector、Agentのtest報告は未検証の結果本文として扱い、独立test recordとresource実測値は`unknown`、時間はUI観測値と明示 |
| 比較結果の選択 | 合格（証拠区分: `FIXTURE`＝試験用模擬） | completed候補だけ選択でき、選択は権限を生成しない |
| 選択した結果だけを適用 | 合格（証拠区分: `FIXTURE`＝試験用模擬） | 選択元のfull結果を未信頼入力としてA/Bと別Workspaceのtargetへ新しいAgent Task要求で渡す。target Permission／Approvalを新規取得し、直接file writeは行わない |
| 片側failure／cancelの相互隔離 | 合格（証拠区分: `FIXTURE`＝試験用模擬） | start outcomeをAgentごとに保持し、片側failureを他方へ伝播せず、cancelは各Task IDに個別送信 |

P2は2026-10-05、Agent CenterのWidget正常経路試験で、同一Taskの独立事前検査、別個のPermission／Approval、二重Taskの並行開始、完了結果とAudit参照、結果選択、別Workspaceへの新規Task準備を一度成立させたため閉鎖する。rev5は製品構築工程で試験専用Owner識別子、試験用模擬経路、内部開発経路を認め、通常Releaseの`task_execution=supported`や導入済み製品上の実Codex横断証明をP2のgateにしていない。選択はAuthorityを生成せず、適用先は新しいWorkspace Permission／Task Approval／Auditを通る。比較間でAuthorityは共有・移送しない。

2026-10-05のCompare実装更新: Agent Centerからactiveな別SessionをA/Bに選択し、同一Taskを別Sessionへ事前検査し、各Permission／Approvalを独立取得した後に2 Taskを並行起動できる。Task状態・result hash・終端Auditを比較表示し、completed候補だけを選択する。選択結果の適用はfull Content Exposureを別途承認後、A/Bから独立したtarget Sessionへ新Taskとして渡す。target側は通常のBroker Workspace Permission／Owner Approvalを別途取得する。選択済みsource結果は未信頼データとしてTask本文へ格納し、Authorityを移送しない。UI／Flutterはfileを書き込まない。

差分はA/Bそれぞれの登録Runtime／Workspaceに絞ったWorkspace Inspectorから別Approval・基準点で確認する。時間値は開始要求からterminal状態観測までのUI時計値のみで、Broker実行時間ではない。Agent test報告は未検証の結果本文、独立test recordとresource実測値は`unknown`。Widget／service試験は`FIXTURE`であり、実Codex CLI・通常Release capabilityを通した`LIVE_RUNTIME` Compareや製品releaseの証拠ではない。これらの統合・installed確認は`docs/FINAL_QA_QUEUE.md`へ送り、P2を再開しない。

検証済み: Desktop `flutter analyze --no-pub`、Compare調停service試験5件、Compare／選択／適用Widget試験1件、Desktop全test 162件、Windows Desktop debug build、Mobile `flutter analyze --no-pub`、Schema検査153 schema／153 example／197 negative fixture、Conformance 232 checks。日本語基底監査は終了コード0。今回追加分に関する指摘は表示語・comment・test文言を日本語化した後の再監査で0件。残る2件は旧rev3／rev4履歴文書だけの過去記録であり、rev5の現行Acceptanceへ転記・書換えしない。Flutter検証はASCII一時cloneで実行し、OneDrive上のFlutter Analyzer LSP障害と区別した。Windows buildは成功し、一時directory出力に関するMSB8029警告だけを記録した。Rust sourceは変更していない。

### P3 エージェント引継ぎ — CLOSED（Product Build）

| P3受入れ条件 | 状態 | 現在成立した実装 |
| --- | --- | --- |
| sender／receiver Agentを別Session・Runtime・Workspaceへ結合 | PASS | Agent Centerは完了Taskを持つ送信Sessionと、未使用で独立した受信Sessionだけを候補にする |
| Task contextと承認済みresultを転送 | PASS | 元Taskへ送った明示instructionと、Broker Content Exposureでnative Ownerが`full`を許可した同一result/hashをbounded packageへ含める |
| artifact／changed files／diff／testsを転送 | PASS | 承認済みresult本文内の厳密JSON形式を検証して抽出し、artifact内容・相対path・diff・test申告をpackageへ含める |
| Permission／Approval／Credential／Authority／Trust／hidden stateを非継承 | PASS | package SchemaはAuthority fieldを禁止し、非継承flagを固定。受信側は別Task要求としてBroker事前検査・新規grant経路へ進む |
| Handoff receiptとAudit参照 | PASS | 受信Taskのstart responseからHandoff receiptを生成。Audit IDは受信Task開始Auditへの参照であり、独立Handoff Broker operation／Auditではない |

2026-10-05、Agent Center Widget試験で送信元TaskをBroker fixture経路から完了し、resultのfull Content Exposureを取得、Handoff公開内容をpreviewした後、異なるRuntime／Session／Workspaceへ新TaskとしてBroker事前検査した。受信側のWorkspace PermissionとTask Owner Approvalを別途取得し、Taskを開始、開始Auditを参照するreceiptを表示した。要求payloadに旧grant ID／credential／authorityを含まないことも確認した。service試験は正常package生成、未完了・full未承認・hash不一致・同一Runtime・Authority混入を拒否した。Schema／fixtureとConformanceも接続した。証拠は`FIXTURE`であり、実Codex間・通常Release・installed productのHandoff証拠ではない。

Handoff受入れは有限条件を満たしたため閉鎖する。厳密なAgent result JSON形式でない出力、32,768-runeを超える受信Task package、agent-reported artifact/diff/testの独立検証、別個のHandoff Broker Audit operationは本Acceptance外であり、既存scopeを拡張しない。Agent申告は受信側へ未信頼として渡され、実行済みtestの証拠へ昇格しない。CompareのCLOSED条件は再試験・変更していない。

検証結果: `python tooling/schema_check/check_schemas.py` PASS（schema 154件／正常例154件／負例198件）、`python tooling/conformance_tests/run_conformance_skeleton.py` PASS（適合確認232件）、Desktop `flutter analyze --no-pub` PASS、Desktop全165試験 `flutter test --no-pub` PASS（Task context寿命制御を加える前に一度実行）、引継ぎ処理の単体試験2件PASS（未完了／full未承認／hash不一致／同一Runtime／不正相対path／上限超過を拒否）、引継ぎ正常系の画面試験1件PASS（最終表示語変更・Task context寿命制御後に再実行）、寿命制御変更後のDesktop `flutter analyze --no-pub`もPASS、携帯版 `flutter analyze --no-pub` PASS、Windows `flutter build windows --debug --no-pub` PASS、`python tooling/manifest.py --check` PASS（対象file 1142件）、`git diff --check` PASS。Flutter試験・buildは英数字のみの一時作業複製で実行し、OneDrive日本語pathで既知のFlutter静的解析問題と分離した。Windows buildはMSB8029（一時directory中間fileに関する警告）を出したが実行fileを生成して終了コード0。

`python -X utf8 tooling/日本語基底監査.py --strict`は終了コード1。今回の変更fileに指摘はなく、残る2 findingは既存履歴`docs/REV3_PROGRESS.md`と`docs/REV4_ACCEPTANCE_LEDGER.md`各1件のみ。過去記録を改変せず保持する。従ってstrict監査全体はPASS扱いにしない。

### P4 提供元・模型管理（Provider / Model Center）— CLOSED（Product Build）

| P4受入れ条件 | 状態 | 成立範囲 |
| --- | --- | --- |
| 提供元registry／選択 | PASS | 実装済みの1経路 `openai_codex_cli`（OpenAI via Codex CLI）をAgent Centerから選択・登録する。全Vendor選択は要求しない |
| 模型registry／選択 | PASS | 登録単位で利用者指定の模型識別子を保持し、1〜128文字のASCII許可文字集合へ制限する。Codex CLIの`--model`へ同じ値を渡す。live catalogや利用可否は偽装しない |
| BYOK認証 | PASS（既存CLI設定への委譲） | 利用者がCodex CLIへ設定した認証を使う。D4 Pocketは資格値を入力・読取・保持せず、metadataも`secret_value_present=false`を維持する |
| capability表示 | PASS | `provider_selection`／`model_selection`は対応済み、通常Releaseの`task_execution`は既存gateどおり`unsupported`のまま |
| health／fallback表示 | PASS | Provider接続・模型利用可否は証拠がないため`unknown`。自動fallbackは無効。Owner確認とAgent Center双方へ投影する |
| 提供元・模型選択の試験経路 | PASS（試験用提供元） | 試験用の偽Codex CLIを実Rust実行系Adapterから起動し、指定模型識別子を受領して作業完了結果へ返す |

P4は一つのProvider経路を選択可能にし、選択模型をBroker登録要求、native Owner確認、Codex CLI実行引数へ一貫して結合したため閉鎖する。未知Provider、権限類似field、自動fallback有効値、不正模型識別子は拒否する。複数Runtime登録はそれぞれの模型選択を保持できる。

実WindowsのCodex CLI `0.160.0`で`--version`と`exec --help`を読み、Adapterが必要とする`--cd`、`--json`、`--ephemeral`、`--ignore-user-config`、`--skip-git-repo-check`、`--sandbox`、`workspace-write`、`--model`を確認した。これはCLI interfaceの存在だけを示し、認証状態、Provider接続、模型の実在性・利用権限、実Provider Taskを証明しない。実Provider呼出しは行っていない。実行時状態は引き続き`unknown`として扱う。

検証: `python tooling/schema_check/check_schemas.py` PASS（155 schema／155正常例／199負例）、`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py` PASS（233 checks）、Agent Center対象Widget 50件PASS、Desktop全test 166件PASS、Desktop／Mobile `flutter analyze --no-pub` PASS、Windows `flutter build windows --debug --no-pub` PASS（MSB8029の一時directory警告あり）、Debug／Release Rust build PASS、P4登録native test 4件PASS、偽Codex CLI模型引渡しtest PASS。Flutter検証はASCII一時複製で実行しOneDrive同期対象へ深いbuild outputを作らなかった。

Rust全件`cargo test -- --test-threads=1`は423 passed／2 failed／6 ignored。失敗したA2AとCodex loopback fixtureは両方とも個別再実行で各1件PASSし、P4変更file・経路には該当しない。再現性のある単独失敗とは観測されなかったためP4を止めず、集約Rust suiteのWindows loopback安定性を`docs/FINAL_QA_QUEUE.md`へ送った。strict日本語監査の現行変更fileは0 finding。全体終了コードは1で、残る2件は旧rev3／rev4履歴文書内のみ。履歴は書き換えない。

### P5 作業領域／履歴／評価（Workspace / History / Evaluation）— CLOSED（Product Build）

| 受入れ条件 | 状態 | 成立範囲・証拠 |
| --- | --- | --- |
| 作業領域検査（`Workspace Inspector`） | 合格 | 実行系／作業領域に束縛した変更file・diffの投影。既存Desktop対象試験22件合格 |
| Agent Task履歴 | 合格 | Brokerの構造化Auditから状態を再構成。現在の実行系履歴閲覧承認を再照合し、本文・Authorityを返さない。Rust対象4件、Desktop履歴画面16件、Desktop／共通UI解析合格 |
| 対話履歴／再実行／分岐 | 合格 | 現行の履歴承認に束縛した一覧と新規要求準備。履歴画面対象14件、Rust P5対象51件合格 |
| 結果／差分／変更file／試験結果 | 合格（`Product Build`範囲） | 作業領域検査／Agent管理画面の上限付き表示。Agentが申告した試験結果は独立検証済みへ昇格しない |
| Regression Case（回帰Case） | 合格 | Owner登録・一覧・操作面の既存対象試験合格（共通UIクライアント6件、2件） |
| Evaluation Case／評価環境 | 合格 | Case登録と評価画面の既存対象試験合格（画面5件、共通UIクライアント10件） |

Task履歴のAudit記録は本文を含めず、Task開始・終端の照合に必要な識別子の要約値、状態、時刻、Audit参照だけを保持する。開始Auditが重複・欠落・不一致、状態と失敗分類が矛盾、実行系が異なる場合は表示・回復投影を拒否する。Broker再起動時の未終端Taskは`suspended`へ隔離する。閲覧は既存の期限付き実行系別履歴承認を再利用し、Flutterは権限・本文を復元しない。構造化記録導入前のTask Auditは安全な実行系別再構成ができないため、既存Auditを保持したまま新一覧の対象外とする。画面でこの範囲を明示する。

今回の追加検証: `cargo test --locked --lib agent_task_history -- --test-threads=1` PASS（4件）、`cargo build --locked` PASS、Desktop履歴画面 `flutter test --no-pub test/history_screen_test.dart` PASS（16件）、Desktopと共通UIの`flutter analyze --no-pub` PASS、Schema検査PASS（158 schema／155正常例／201負例）、Conformance PASS（234 checks）。Flutter試験はOneDrive上の既知のbuild cleanup問題を避けたASCII一時cloneで実行し、検査対象の3 Dart fileだけを同期した。`python -X utf8 tooling/日本語基底監査.py --strict`はFAILのまま。今回変更fileは0 findingであり、残る2 findingは旧rev3／rev4履歴文書の既存記録で、現行意味正本へ昇格・書換えしない。既存P5 focused tests: Workspace Inspector 22、履歴／内容表示14+4、Evaluation画面5、対話履歴27、Regression Case Client 6+2、Evaluation Client 10、Rust P5対象51（合計141件）PASS。これはProduct Build受入れであり、通常Release capability、installed product、広域regression、最終出荷保証を意味しない。大規模evaluationとFeature Complete後の横断品質保証はFinal QAへ送る。

### P6 資格情報／MCP — CLOSED（Product Build）

| 受入れ条件 | 状態 | 成立範囲・証拠 |
| --- | --- | --- |
| 資格情報のOwner登録・一覧・失効 | 合格（Broker経路） | 登録はOwner control、一覧はmetadata-only、失効はRust Desktop native確認と永続Auditを要求。資格情報Broker試験6件PASS |
| 提供元への資格情報結合 | 合格（Product Build fixture） | `openai_codex_cli`の登録済みCredential IDをBrokerが現在のTask Permission／Approvalと照合し、DPAPIから解決した合成秘密値をCodex CLI親process環境へだけ渡す。偽Codex CLI fixtureは引数への非露出とshell環境除外設定を検査 |
| MCP Credential結合 | 合格（Windows製品経路） | 登録済みCredential ID・用途・対象Server・状態をDPAPI保管と照合し、対象stdio childだけへ短命値を渡す。別Server再利用を拒否 |
| MCP接続・Tool一覧 | 合格 | BrokerがWindows stdio childからdiscovery／Tool Schemaを取得し、通常IPCへmetadata-onlyで投影。Trustや権限は生成しない |
| MCP Tool実行・監査 | 合格（Product Build範囲） | Rust Broker・DPAPI・実`cmd.exe` fixture processを通すWindows統合試験で一回限りPermission、Owner-confirmation要求、永続Auditへの記録、hash-only結果、切断を確認 |

開始時点の記録: P6のMCP基本経路は2026-10-05に先行実装済みだったが、Provider credential bindingは未接続だった。MCP統合試験は合成Credentialだけを使い、実値とTool本文markerがBroker応答・Auditに現れないことを確認した。Tool応答の`LIVE_RUNTIME`はfixture child processとの実通信を示すが、Windows確認dialogを表示するDesktop製品操作の証拠ではない。

2026-10-05のP6完了単位で、Broker資格情報解決を現在要求のWorkspace Permission／Task Approval後に限定し、永続使用Audit確定後だけDPAPI秘密値を短命bufferでCodex Adapterへ渡す経路を追加した。Codex CLI資格情報modeは既存CLI管理認証modeと分離し、自動fallbackしない。Provider credential binding、Owner登録／一覧／失効、MCP接続／Tool一覧／Tool実行のProduct Build基本経路が揃ったためP6をCLOSEDとする。

Provider結合の`FIXTURE`試験では、合成CredentialがCodex CLI親process環境へ届き、CLI引数・結果・Auditへ出ず、tool shellから除外する設定が渡ることを偽Codex CLIが確認した。Broker試験ではPermission／Approval欠落と本文差替時にResolverが呼ばれず、有効grant後に一度だけCredentialがAdapterへ届く。これは実Codex CLI／実Provider APIの成功証拠ではない。実Codex CLI 0.160.0を合成loopback APIへ接続する`LIVE_RUNTIME`試験は、API要求前にMxCが`CreateProcessSecurityEnvironment`／HRESULT `0x80070003`で停止した。Provider接続・模型利用可否は`unknown`のままとし、自動fallbackを無効、通常Releaseの`task_execution=unsupported`を維持する。実CLI／MxCの統合証拠と別profile installed経路はFinal QA／release evidenceで確認する。

検証結果: JSON Schema 158件、正常例155件、負例fixture 203件、適合確認234項目に合格。資格情報保管庫のRust対象試験7件、Broker資格情報解決の許可境界試験1件、偽Codex CLIへの合成資格情報引渡し試験1件、MCP Broker試験2件、MCP stdio試験5件、DesktopのMCP metadata試験12件、資格情報IDだけを登録する画面試験1件もそれぞれ合格。`cargo build --locked`も合格。

非選別の`cargo test --locked -- --test-threads=1`はlib試験432件成功／2件失敗／7件ignoredで終了した。失敗は既存Windows HTTP／TLS接続fixtureの接続切断（OS error 10054／`ConnectionReset`）で、Codex loopback fixtureは単独再実行で成功、Update DownloadのTLS修復fixtureも単独再実行で成功した一方、既存package置換fixtureは単独でも接続切断を再現した。現行差分の他targetまで結果を得るため、この3試験だけを`--skip failed_tool_result_is_not_replayed_as_another_exec_command --skip failed_replacement_keeps_the_existing_corrupt_package_unchanged --skip local_tls_server_repairs_only_after_verified_package_bytes`で明示除外した全target実行を行い、lib 431件成功／0失敗／7 ignored／3 filteredとなり、他の全実行targetも失敗なしで完了した。3試験は削除・変更せず、集約loopback安定性を既存`FQ-TEST-LOOPBACK`へ記録する。これはP6資格情報変更経路の失敗ではなく、P6 Acceptanceを阻止しない。通常Release capabilityや正式配布可否へは昇格しない。

日本語基底strict監査は終了code 1で、旧rev3／rev4履歴文書の2件と、試験内の外部Authorization protocol値`Bearer`に対する静的heuristic finding 1件が残る。今回のテスト説明文は日本語化済み。過去履歴は改変せず、protocol値も監査回避のために変形しない。この監査結果は日本語文書負債として記録し、P6のCredential authority境界・Provider結合Acceptanceとは分離する。

### P7 対A2A連携／複数Host／Adapter管理 — CLOSED（Product Build）

rev5の範囲はA2A、Multi Host、Adapter Manager、Host capability、接続性、degraded mode、local／remote Runtimeの区別である。実接続または決定的fixtureで主要経路を成立させる受入れ条件を満たしたため、P7 Product Buildを閉鎖する。各機能の経路・証拠は次の完結単位に記録する。

#### P7内の完結単位: A2A loopback接続面 — CLOSED（Product Build）

- 必須`shell.agent_operation`へDesktop専用A2A接続センターを追加。専用画面、全体検索、コマンドパレット、NavigationRailから到達し、optional Moduleを全て除いたbuildでも表示する。
- 接続要求はFlutterから既存Broker IPCへ送り、Rust起動器のnative Owner確認を経る。BrokerもA2A接続をnative確認専用とし、通常IPC／Owner credential要求を拒否する。Owner対象表示とBroker helper双方で、loopback IPv4 HTTP、query／fragment／userinfoなし、未対応Credential ref、Task等を送らない境界を検査する。Windows Runnerのnative確認応答timeout allowlistにもA2A操作を追加。
- Brokerが既に持つAgent Card取得・永続Audit・接続state・metadata-only一覧をDesktopへ接続。画面は接続先URIを成功後に消去し、endpoint hashとbounded metadataを表示する。Trustは`pending_review`を維持し、Permission／Approval／AuthorityやA2A Task実行へ昇格させない。
- Agent Card由来の表示文字列は未信頼として表示し、改行／方向制御文字を含む値をUI clientで拒否する。認証実値は要求へ含めない。
- 個別検証結果: RustのA2A境界試験9件、native Owner確認候補1件、Brokerのnative確認専用gate 1件、Flutter接続service／画面／検索試験9件、任意Moduleをすべて無効にしたNavigationRail試験2件、`Conformance` 234件、Windows版Desktopのdebug build成功（MSB8029の一時作業フォルダー警告あり）。Flutter試験とWindows buildはOneDrive外の`ASCII`一時複製で実行。
- Rust全targetは444件中435成功／2失敗／7 ignored。今回のA2A関連11件は全件成功。失敗のうち`failed_tool_result_is_not_replayed_as_another_exec_command`は単独再試験で成功し、`local_tls_server_repairs_only_after_verified_package_bytes`は単独でもHTTP/TLS fixtureの応答不整合とConnectionResetで失敗した。いずれも今回変更したA2A経路外で、既存`FQ-TEST-LOOPBACK`へ追加記録し、このA2A受入れを拡張しない。
- Strict日本語基底監査は終了コード1で、現行変更fileは0 finding。既存findingは3件（旧rev3文書1、旧rev4文書1、未変更Codex CLI診断文字列1）。過去文書・無関係Adapter診断は今回のA2A Acceptance外として保持し、監査全体をPASS扱いしない。
- rev5 Product Buildの基準（実装、build、基本正常経路、次工程からの利用、Authority非破壊）を満たしたため、このloopback UI接続単位をCLOSEDとする。C16の既存Desktop画面未成立記述を現行状態へ訂正した。外部A2A transport／Task実作用とinstalled product証拠は別gateであり、この受入れを拡張しない。P7全体の受入れ判定は本章末尾に記録する。

#### P7内の完結単位: Host所在・能力・degraded表示 — CLOSED（Product Build）

- Host operation surfaceは、`snapshot_source=broker`として受理された製品snapshot内で、Host capabilityのHost IDとHost registry IDが一致した場合だけ、そのHostを現在Brokerの実行場所（ローカル）と表示し、Broker由来Runtime／Agent一覧を接続する。
- `mock`、fallback、in-memory diagnostic、未検証snapshotはlocal／remoteまたは実測能力の証拠にしない。別Hostは登録metadataのruntime／agent件数だけを表示し、remote接続と個別一覧は未観測に保つ。Host registryの接続／Trust状態と現在Brokerの実測を別表示する。
- Host capability画面では、受理済みBroker snapshotの`degraded`と機能別証拠を表示し、権限生成へ昇格させない。非Broker snapshotでは能力値を未観測の表示用状態として明示する。
- 検証: 画面試験54件は全て合格（模擬／診断snapshotの昇格拒否、受理済みBroker snapshotによるローカルHost判定、別Host要約の分離、縮退表示を含む）。変更対象のDart 3 fileの静的解析とWindows向けデバッグbuildも合格した。`MSB8029`は`ASCII`一時検証先へのbuild出力警告。試験とbuildは`OneDrive`外の`ASCII`一時複製で実行した。
- この完結単位はHost状態の製品表示・snapshot分類を閉じる。実Host間接続、Device Link認証、remote capability discovery、Host間Workspace隔離を証明するものではない。これらの未成立範囲はrelease gateで保持し、P7 Product Buildのfixture受入れとは分離する。試験中に検出したNavigationRail件数3箇所の旧値（20）は新しい必須A2A画面を含む現行値（21）へ同期した。

#### P7内の完結単位: Adapter既存recordのDesktop操作 — CLOSED（Product Build）

- Desktop Runtime Centerの`検証`、`有効化`、`無効化`、`隔離`、`削除`を、既存のPID結合named pipe、Rust Desktop起動器のnative Owner確認、Broker Owner操作queueへ接続した。操作名、Adapter ID、現在hash、payload hashを確認対象に固定し、未知field、操作不一致、形式不正、古いpayload hashは確認候補から拒否する。
- native Owner確認で拒否すると状態変更なしの`Suspended` receiptとAuditで終わる。承認後はBrokerが現行catalog recordのhashと状態条件を再照合する。変更範囲はBroker内catalogとAuditに限り、Permission／Approval／Credential／Trust、外部file、process、既存processへ作用しない。通常IPCの直接変更要求は引き続き`owner_reapproval_required`で停止する。
- Install／UpdateのManifest経路、外部artifactのdownload／filesystem導入・削除、process起動、Windows installed product証拠はこの完結単位に含まれない。C19の既存release blockerを解消したとは扱わない。
- 個別検証: Adapter Center Rust試験7件、Desktop Owner候補／relay試験2件、実Win32 Owner dialog自動化（拒否・承認）1件、既存Flutter通常IPC拒否widget試験1件が成功。ASCII一時複製で`flutter build windows --debug --no-pub`も成功し、Windows Runnerの変更を含むcompileを確認した。MSB8029一時出力先警告あり。試験自動化がdialogを操作し、Ownerの画面操作は不要だった。
- この既存record操作のProduct Build受入れを閉じた。旧rev3／rev4の進捗記録は変更せず、現行状態の判定には本rev5進捗と現行codeを使う。

#### P7内の完結単位: Adapter Manifest導入・更新のDesktop metadata操作 — CLOSED（Product Build）

- Runtime CenterからManifestを上限48 KiBで既存Broker transportへ送り、PID結合pipe、Rust Desktop起動器のnative Owner確認、Broker Owner操作queueを通じて、Broker catalogのmetadataだけを登録・更新する。通常IPCからの直接変更は停止する。
- Owner確認は申告Capability／許可差分／危険・互換性・署名metadataと署名対象／signature hashを示すが、署名実値は表示しない。Owner確認はTrust、署名検証、Permission、Approval、Credential、Authorityを生成しない。未検証Adapterは有効化できない。
- 更新は対象Adapter IDと現在hashを要求に束縛し、Brokerが処理時にhash・disabled・非隔離状態を再確認する。古いhash、有効化中、隔離済み、対象ID不一致は変更前に拒否する。拒否・受理はBroker Audit receiptへ結合する。
- 受入れ範囲はmetadata登録・更新であり、外部download、filesystem install／remove、process起動・管理、Windows installed product evidenceを含まない。これらC19のrelease blockerは解消していない。P7 Product Buildの総合判定は本章末尾へ分離し、この完結単位とC19全要件を同一視しない。
- 検証: Rust Adapter Center対象試験9件合格、Manifest Owner候補／中継試験合格、実Win32 Owner確認画面の自動操作（拒否・承認）1件合格、Desktop全画面試験57件合格（日本語表示ラベル修正後に対象Widgetを再実行）、変更したDart 3 fileの静的解析合格、Schema 158件／正常例155件／負例205件合格、適合確認234件合格、Windows Desktop debug build合格。全Desktop静的解析は未変更の`agent_center.dart`に廃止予定APIの情報指摘5件があり終了コード1、今回変更したDart fileに問題はない。BuildはASCII一時複製で行い、MSB8029一時出力先警告を記録。厳格日本語監査は今回変更fileの指摘0件、全体では既存の3指摘（rev3履歴、rev4履歴、未変更Codex CLI診断文字列）により終了コード1。旧記録・既存指摘は保持し、本完結単位へ転記・修正しない。
- Rust全体試験`cargo test --locked -- --test-threads=1`は443件合格／2件失敗／9件ignored。失敗は既存Codex loopbackとUpdate Downloadのlocal TLS fixtureでConnectionResetを観測したもの。2件を個別再実行すると各1件合格し、Adapter差分外だったため、既存`FQ-TEST-LOOPBACK`へ追記した。Adapter対象focused試験と本単位の受入れは合格であり、P7を阻止しない。
- 本単位をCLOSEDとする。Host実接続／remote capabilityと外部artifactのfilesystem・process作用は、C17〜C19のrelease blockerとして別管理する。rev5 Product Build受入れとrelease gateを混同せず、PASS済み単位は再開しない。

#### P7 rev5 Product Build受入れ — CLOSED

| 受入れ領域 | 結果 | 現行証拠・境界 |
| --- | --- | --- |
| A2A／接続性 | PASS | Rust Brokerのloopback接続とDesktop接続面を実装・試験済み。外部A2A transport／Task実作用は未成立。 |
| Multi Host／local・remote区別 | PASS | Host registryとHost操作面を接続。localは受理済みBroker Host ID一致時だけ、他Hostは登録metadataとして表示し個別状態を未観測に保つ。 |
| ホスト能力と縮退表示 | 合格 | Broker snapshotの証拠種別と制限状態を表示し、試験用fixture／診断値を実測値へ読み替えない。 |
| Adapter管理 | 合格 | 既存記録の検証・状態制御・削除と、native Owner確認経路によるManifest metadata導入／更新を実装。署名未検証では有効化を拒否する。 |
| Authority非生成 | PASS | Host／Adapter metadata、Owner確認、接続表示からPermission／Approval／Trust／Credential／Authorityを生成しない。 |

rev5工程表の「実接続または決定的fixtureで主要経路成立」に対し、A2A loopbackの実接続とHost／degraded／remote未観測の決定的fixture、Broker Owner経路のAdapter管理を確認した。これによりP7のProduct Build機能範囲を満たし、P7をCLOSEDとする。

実Host間の認証付き通信・remote capability discovery・Host間Workspace隔離、外部Adapter artifactのdownload／filesystem導入・削除・process管理、Windows installed product evidenceは未成立のままrelease blockerとして保持する。これらを成立済み、C17〜C19の全要件完了、またはrelease-readyへ読み替えない。Feature Complete後の横断保証はFinal QA queueに従う。

### P8 GUI-Shell構成 — CLOSED（製品構築受入れ完了）

| 受入れ項目 | 結果 | 現行証拠・境界 |
| --- | --- | --- |
| Runtime／Agent／Tool／MCP選択 | 合格 | Desktop設定画面で参照IDを入力しManifestへ正規化してBrokerへ送る。実在性・接続・Agent trustの発見や判定は行わない。 |
| UI構成 | 合格 | Theme mode、表示密度、内容表示要求、Capability requirementを画面から選択しManifestへ反映。Theme IDとLocaleは現行契約により固定。これらの要求はPermission／Authorityを生成しない。 |
| Module選択 | 合格 | 任意Module checkboxをCompose Manifestと同じ書出し操作へ渡し、BrokerがExport ReceiptのModule planを生成する。Compose Manifest本体とは別fieldであり、実binary pruningを意味しない。 |
| 事前表示（Preview） | 合格 | 直近受理済みManifestと候補をBrokerへ送り差分を計算。rollback、build、Exportを実行しない。 |
| AI編集提案（AI Edit） | 合格 | 明示指示を審査待ちproposalとしてBrokerへ送り、自己承認とfile writeをしない。 |
| 一構成を作成 | 合格 | Desktop設定画面からManifest作成・Preview・Module選択・proposal要求へ接続。Compose結果はManifest-onlyであり、独立App／executableは生成しない。 |

検証結果: Compose関連Desktop client／Widget試験12件合格（800px幅のSettings画面統合fixtureを含む）。Rust Broker focused試験はCompose／Preview 7件、Export／Module plan 13件、AI Edit 2件が合格。Schema検査158 schema／155正常例／205負例、Conformance 234 checksが合格。変更Dart 4 fileの`flutter analyze --no-pub`は問題なし、Mobile全体の`flutter analyze --no-pub`は問題なし。Desktop全体解析は終了値1で、未変更`agent_center.dart`の既存deprecated API情報5件だけ。Windows Desktop debug buildはASCII一時複製で成功（MSB8029一時directory警告あり）。OneDrive checkoutでのFlutter試験起動は`build\unit_test_assets`削除失敗により不可だったため、Flutter試験／buildはASCII一時複製で実行した。これはfixtureとlocal buildのProduct Build証拠で、Windows installed／standalone product実起動証拠ではない。

これをrev5工程表のP8範囲に限ったProduct Build受入れとしてCLOSEDにする。実executable、portable package、installed product、実Module binary除去はP9以降またはrelease gateで扱い、P8へ逆流させない。

### P9 Standalone Export — CLOSED（製品構築受入れ完了）

| 受入れ項目 | 結果 | 現行証拠・境界 |
| --- | --- | --- |
| 独立App ID／Audit Store IDとManifest | 合格 | clean `main` commit `77821953a70ee3d1ff1059835dad56fc658a35b3`のchecked-in test Receipt／Manifestから、App ID `d4-pocket-app-11111111111111111111111111111111`、Audit Store ID `audit-store-22222222222222222222222222222222`を含む製品をbuild。入力は合成fixtureであり、Owner承認済みproduction Exportではない。 |
| executable／Rust Broker／選択Module／portable package | 合格 | Windows Release bundleにRust起動器、Flutter executable／assets、Rust Broker、byte-for-byte Manifest copyを含む14 filesを生成。要求された`shell.trace_inspector`をincludeし、optional module planをFlutter compile defineへ渡した。 |
| 生成物一覧／秘密情報の既知パターン検査 | 合格 | 62,856,402 bytes、tree SHA-256 `d5624b93eb73e3f1f9b9a1102e43ea32394fb76404be67f8e581ea4d1a73360e`。既知パターン検査は14 files／0件。未知形式を含むCredential不存在やbinary pruningは証明しない。 |
| 元D4 Pocketと分離した製品起動 | 合格 | 生成bundleのRust起動器から同bundle内Flutter childが起動し、Windows process parent／image path、応答中の`D4 Pocket` main window、製品ID別runtime pathの新規生成を`LIVE_RUNTIME`で観測。runtime rootは起動前に不在で、起動後は`%LOCALAPPDATA%\D4Pocket\apps\<App ID>\stores\<Audit Store ID>`に専用Store／新規Audit anchorができ、generic GUI-Shell Broker pathと異なる。 |
| Credential／Permission／Approval／Audit chain／Authority非継承 | 合格 | fixture Manifest／Receiptのauthority stripと全`*_inherited=false`、build evidenceの同境界、起動前に不在だった専用runtime root、新規Audit anchorを確認。Credential実値は入力していない。これはfixture起動範囲であり、production Owner operation、署名、一般的なsecret不存在の証拠ではない。 |

実行したbuild commandは`python -X utf8 tooling/export_windows_product.py`（checked-in test Receiptと対応Manifestを使い、一意なTemp input／output directoryを指定）。Flutter 3.44.0／Dart 3.12.0、Rust/Cargo 1.95.0でRelease build・Schema／Receipt／Manifest照合・Module plan解決・inventory再計算・known-pattern scanまで成功した。Manifestはpackage rootの`product_manifest.json`にbyte-for-byteで含まれる。Rust起動器のStore選択は現行launcher contractどおりManifest再読込ではなくbuild時のSchema検証済みIDを使用する。

Windows Computer Useはこの2個目の同名Flutter windowをtargetable windowとして返さなかったため、画面screenshotや視覚的UI確認は主張しない。Windows process metadataではchildが`Responding=true`でmain windowを生成したことを観測した。`WM_CLOSE`は現行Windows Runner仕様に従い通知領域へ隠す動作だった。normal tray exitはP9の受入れ項目ではなく、この試験では未確認。試験cleanupでは対象bundleのfrontendだけを停止し、起動器が終了Error dialogを出した後、対象launcherも停止した。これはnormal exit evidenceに数えず、P12統合Productのnormal exit確認へ送る。Broker session fileは消え、専用Audit Storeは独立確認用のtest stateとして保持した。

これをrev5工程表P9の有限なProduct Build条件の完了としてCLOSEDにする。実Owner確認によるExport、別user profile／installed artifact同一性、署名済み配布、正式Installer、runtime Manifestのproduction trust、binary pruning、normal exit、release readinessは成立扱いにせず、既存release blockerまたは後続P12／Final QAの範囲で保持する。現在phaseは直ちにP10 Module Selection / Pruningとする。

### P10 Module Selection / Pruning — CLOSED（機能上の除外）

| 受入れ項目 | 結果 | 現行証拠・境界 |
| --- | --- | --- |
| 選択Moduleと依存閉包 | 合格 | `python -X utf8 tooling/build_module_pruned_windows.py --receipt examples/contracts/gui_shell_export_receipt.valid.json --plan-only`が追跡画面と依存先の観測画面だけを含め、任意画面6件を除外した。固定安全Coreと必須画面はすべて保持し、Authority検証false、binary pruning未実施を出力した。 |
| 書出し製品への選択反映 | 合格 | P9 Windows Release bundleのbuild receiptは、選択された追跡画面と依存先にtrue、除外6画面にfalseのcompile-time defineを記録する。これはbuild設定の証拠であり、AOT／実行可能fileからのbinary除去証拠ではない。 |
| 製品画面の機能上の除外 | 合格 | 全8任意画面を無効にしたFlutter Widget試験2件が、任意画面のナビゲーション非表示、必須画面13件の保持、除外後の操作で例外・未搭載画面が出ないこと、除外されたHost操作Commandが出ないことを確認した。証拠源は`FIXTURE`。 |

実行した試験は、Desktop projectで`flutter test --no-pub --no-test-assets --dart-define=GUI_SHELL_MODULE_SETUP_DOCTOR=false --dart-define=GUI_SHELL_MODULE_HISTORY=false --dart-define=GUI_SHELL_MODULE_EVALUATION_LAB=false --dart-define=GUI_SHELL_MODULE_HOST_CAPABILITIES=false --dart-define=GUI_SHELL_MODULE_NOTIFICATIONS=false --dart-define=GUI_SHELL_MODULE_OBSERVABILITY=false --dart-define=GUI_SHELL_MODULE_TRACE_INSPECTOR=false --dart-define=GUI_SHELL_MODULE_HOST_OPERATIONS=false test/module_pruning_test.dart`（2件合格）、`python -X utf8 tooling/build_module_pruned_windows.py --receipt examples/contracts/gui_shell_export_receipt.valid.json --plan-only`、Schema検査（158 Schema／155正常例／205負例）、Conformance（234 checks）。最初のasset付きFlutter試験は、OneDrive上の`build/unit_test_assets`をFlutterが削除できず、試験本体開始前に失敗した。assetを要しない対象試験を`--no-test-assets`で再実行して合格した。

P9 portable bundleのWindows再起動時、Computer Use helperは同名製品windowを対象可能な窓として返さなかった。process path／parentは対象bundleを識別できたが、視覚的な画面確認は主張しない。P10の機能受入れはcompile-time define、製品build記録、Widget試験の範囲で閉じる。AOT／binaryの意味上の除去、Rust／第三者依存の除去、サイズ、cold startup、実行時resource、最終統合製品上の再確認は`docs/FINAL_QA_QUEUE.md`と既存release blockerへ送る。`binary_pruning_verified=false`は維持する。現行phaseは直ちにP11 Windows Productizationとする。

### P11 Windows製品化 — OPEN

現行P11では、署名package staging、有効版record／Start Menu登録、およびBrokerが記録した直前版へのRollbackを独立native Owner確認付きでBroker fixture経路へ接続した。Rollbackは両候補を現在trustで再検証し、active recordだけをatomic toggleする。version-local launcherのSetup Doctor向けinstalled-path照合、初回Install／導入済みUpdateを区別するUI、同一download候補確認後だけ行えるstage操作に加え、Broker固定trust配布元から署名検証済み更新候補catalogを取得する通常UI操作を接続した。UninstallerのSettings要求からBroker native Owner確認・durable Audit・一回ticket・終了後固定root削除helperまで実装し、Windows fixtureで固定製品root／一致するshortcutだけの除去と利用者data保持を確認した。ただし導入済み製品での連続実行は未実証。正式Installer、installed product全体経路、Repair、crash／電源断Recoveryは未成立でありP11はOPEN。

#### 2026-10-06 P11 Uninstaller接続 — 実装済み（installed end-to-end未実証）

- SettingsのUninstaller要求は既存Rust Desktop native Owner確認→既存Broker operationへ接続した。Brokerはinstalled-path evidence、埋込みApp／Audit Store identity、Windows Known Folderから再導出した固定root／shortcut先、native確認時のpayload hashを照合し、durable削除意図Audit確定後だけ一回ticketを返す。LauncherはticketをFlutter応答から除き、正常終了後に起動した同一launcher hashのhelperへstdinで引き渡し、親Launcher終了後に保持されるBroker Audit Storeで未使用ticketを再検証してから固定App ID rootとそのrootを指すshortcutだけを削除する。Credential／設定／Workspace／Audit Storeは削除対象外。完了／失敗をAuditへ記録し、開始後の失敗・crash後に同ticketを再使用せず、新しいOwner確認を要求する。
- 正常fixtureは製品root・一致shortcutの削除とAppData Audit fileの保持を確認し、shortcut競合fixtureは製品rootとshortcutを保持して停止する。Broker fixtureはOwner確認なし・不正／未知ticket・開始前完了・失敗ticket再利用・完了後再利用を拒否し、失敗後の新Owner ticketとBroker再起動を挟んだticket非再利用を確認する。Launcher試験はOwner確認表示に固定root・保持dataを示し、raw ticketをFlutter応答へ出さないことを確認する。
- 検証: `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib uninstall_ -- --test-threads=1`は初回のAudit識別marker不一致を修正後、5件合格／0件失敗。`cargo build --locked --release --manifest-path native/rust_helper/Cargo.toml` PASS（未変更`minidora.rs`に`dead_code`警告2件）。Schema 161／正常例157／負例208、Conformance 236 checks PASS。Flutter更新画面試験8件、変更対象Dart 3 fileの静的解析、Mobile全体静的解析、Windows版Desktop Debug buildもPASS（MSB8028中間directory共有warning）。Dart形式確認は変更対象3 file・変更なし。Rust全体試験は489件合格／1件失敗／12件除外。唯一の失敗は差分外の既知A2A loopback fixtureで、単独再実行1件PASSし、既存`FQ-TEST-LOOPBACK`に追加記録する。Desktop全体の`flutter analyze --no-pub`は終了値1で、未変更`agent_center.dart`内の廃止予定API案内5件のみ。変更対象Dartの解析はPASS。厳格日本語監査は今回変更fileにfindingなし、現行差分外の履歴／診断3件を検出して全体終了値1。manifest 1178 file check、Release Gate check、`git diff --check`はいずれもPASS。
- これはProduct Buildの実装・局所Windows fixture・compile証拠であり、正式Installerからの導入、実installed product上のnative確認からuninstall完了までの`LIVE_RUNTIME`、外部プロセスcrash／電源断復旧を示さない。正式Installer、導入から次回起動・Uninstallerを含む統合経路、Repair、実installed Recoveryは引き続きP11残件。現finalizerはTemp内へ検証済み一時copyを作るが、処理後の自身の一時directory cleanupはまだ行わないため、`FQ-P11-UNINSTALL-FINALIZER-CLEANUP`へ送り、製品統合後に処理する。

#### Package format／reader基盤 — Broker fixtureへ接続済み（installed product未実証）

- `D4PKG01`固定無圧縮format、JSON Schema、Developer専用portable Export packager、Rust package readerを追加した。
- Rust readerは上限付きpackage／manifest、case-insensitive path順序・一意性、Windows path traversal／予約名／reparse point拒否、required payload、各file byte length／SHA-256、trailing byte、同梱Product ManifestのApp ID／Audit Store ID／新規Audit／Authority非継承を検証し、未存在stage directoryだけへ展開する。
- Python packagerも同梱Product Manifestのidentity／Authority条件を照合する。Builder／readerとも配布元署名を検証せず、trust・Permission・Approvalを生成しない。
- 個別証拠: `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib product_package -- --test-threads=1` 4件合格。`python -X utf8 tooling/schema_check/check_schemas.py`はSchema 159件／正常例156件／負例206件で合格。`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`は235 checksで合格。
- portable ExportのSettingsへsource `pubspec.yaml`由来の製品版を表示し、package metadataとFlutter compile defineに同じsource値を渡す局所経路を実装した。`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`（235 checks）、ASCII一時複製での`flutter test --no-pub --dart-define=GUI_SHELL_PRODUCT_VERSION=1.2.3 test/settings_compose_test.dart`（1件）、`flutter analyze --no-pub`（Mobileは問題なし、Desktopは未変更`agent_center.dart`の既存deprecated API情報5件で終了値1）、および変更Dart fileのformatter確認が得られた。OneDrive直下のFlutter testは`build/unit_test_assets`削除前に停止した。Desktop全体解析はchanged fileにfindingを出しておらず、対象Widgetはcompile・表示を通過した。日本語厳格監査は既存3 findings（旧rev3／rev4履歴および未変更CLI診断文字列）のため終了値1。version表示はbuild identityの投影だけで、installed product、署名・trustの証拠ではない。
- Installer／Uninstaller／正式導入first-run／installed product上のStart Menu／Rollback／Repairおよびinstalled product全体経路は未成立。署名済みpackageのstaging、active record／Start Menu切替、1世代Rollbackは既存BrokerのCapability／Permission／独立native Owner Approval／durable Audit経路へBroker fixtureとして接続済み。追加fixtureはstage prefix再開、完全stage再照合、相違prefix／余分なentry拒否を確認した。実installed productでの強制crash／電源断LIVE_RUNTIME復旧証拠は未成立。standalone SetupがBrokerを迂回してfilesystem／HKCUへ作用する案は廃止し、fixture単位の成立だけでP11受入れ、Windows Feature Complete、正式配布可を主張しない。
- 起動時のpackage layout検証とinstalled root検証を別状態へ分離した。portable bundleは固定package配置を満たせばBroker登録／固定storeの初回UI設定を利用できるが、installed pathのSetup Doctor項目は独立したinstalled-root evidenceがない限り`unknown`である。installed evidenceの昇格、Installer、実installed Recovery／Rollbackは未成立。active version切替は下記のBroker fixture経路へ接続したが、installed起動証拠ではない。
- 2026-10-06 P11導入先基礎: Windows Known Folder由来のLocalAppData、App ID、製品版、package SHA-256からper-user固定install pathを決定するtest-only計画modelと拒否試験を追加した。通常Releaseでは未接続であり、package展開、native Owner確認、Broker Permission／Audit、Start Menu、実installed起動の成立を意味しない。P11 OPENを維持する。
- 2026-10-06 P11 package consumer基礎: Broker consumer向けRust readerにcapabilityで開いた同一file handleの全package SHA-256検証とstage展開を追加し、digest不一致時のstage除去を試験した。これ自体は配布元署名、Broker Owner確認、install root公開、Update applyを実装したことを意味しない。
- 2026-10-06 P11 reader書込境界: 出力parent `Dir` capabilityと単一componentのstage名を受ける展開APIを追加した。stage directory identityを開いたhandleと照合し、package内directory／fileをno-followの相対操作だけで作成する。既存stage衝突を保持し、digest不一致およびfile/directory衝突ではidentity照合済みの自身のstageだけをhandle経由でcleanupする。stage名のtraversal／Windows予約名も拒否する。focused reader test 7件PASS、対象fileの`rustfmt --check` PASS。これはInstall／Update consumerへの接続、Broker Owner確認、Permission／Audit、installed productの証拠ではない。
- 2026-10-06 P11 capability-reader回帰: 非選別の逐次Rust suiteは465件中454 passed／2 failed／9 ignored。既存MINIDORA loopback fixtureは単独再実行でPASS、既存Update Download TLS fixtureは単独でもOS error 10054／`ConnectionReset`でFAIL。双方とも本差分外として既存`FQ-TEST-LOOPBACK`へ記録し、P11受入れのblockerにはしない。Schemaは159 schema／156 example／206 negative fixture、Conformanceは235 checksでPASS。厳格日本語監査は終了値1の既知3 findingsのみで、今回変更した現行文書・readerにfindingなし。
- 2026-10-06 package consumer reader検証: `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib product_package::tests -- --test-threads=1`は5件合格。非選別のRust全試験は453件合格／1件失敗／9件ignoredで、失敗は既存`broker::update_download::tests::local_tls_server_repairs_only_after_verified_package_bytes`のOS error 10054／`ConnectionReset`。この同じ試験を単独実行すると1件合格した。fixture競合の原因は未確定で、現行P11のreader差分外かつ既存`FQ-TEST-LOOPBACK`に属するため、P11 blockerへ追加しない。Schema（159件／正常156件／負例206件）、Conformance（235 checks）、変更fileの日本語監査、`git diff --check`は合格。
- 2026-10-06検証: portable判定Rust試験1件、Schema（159件／正常156件／負例206件）、Conformance（235 checks）はPASS。Rust全試験の逐次実行は449 passed／1 failed／9 ignoredで、失敗は既存P7 A2A loopback fixtureの応答期限超過。同test単独再実行はPASSし、原因は未確定。並列全試験のUpdate Download TLS fixture接続resetも逐次再実行では再現しなかった。現phase blockerにはせず`docs/FINAL_QA_QUEUE.md`へ記録。厳格日本語監査は既知3 findingsのみで、変更fileにfindingなし。`git diff --check`はPASS。Rust formatter確認は既存コード領域の未整形差分で不合格となり、広域整形は行っていない。

2026-10-06 P11更新配布物の未起動版保存処理: `更新適用要求`をRust Desktop起動器のOwner確認からRust Brokerへ接続した。Brokerは現在の信頼設定と署名済み候補、build時に埋め込んだApp ID／Audit Store ID、Windows既知フォルダーAPIから導出する導入先を再照合する。展開前後の永続Auditが確定した後だけ、利用者単位の固定`versions` Capability内へ、同一ファイルハンドルから配布物を展開する。成功状態は`version_staged`、有効化状態は`suspended`。既存版は上書きせず、Start Menu、有効版切替、プロセス起動、rollbackは行わない。Flutter表示も未起動版の展開完了と、切替／起動／rollbackの保留を示す。

対象Rustは`cargo check --locked --manifest-path native/rust_helper/Cargo.toml`と`cargo build --locked --release --manifest-path native/rust_helper/Cargo.toml`がPASS。Release buildは未変更`minidora.rs`のdead_code warning 2件を出したが生成は完了。署名済み配布物のBroker staging境界test 1件、固定versions capability 4件、Owner確認文test 1件PASS。実Win32 Owner dialogのNo／Yes自動操作testも明示実行で1件PASS。Flutter `update_client_test.dart` 4件PASS、変更Dart fileのformatter PASS。Desktop analyzeはrepository-root ASCII junction経由で全体実行し、今回変更file finding 0、未変更`agent_center.dart`にdeprecated API info 5件で終了値1。Mobile analyzeは同経路で問題なし。

Rust全target逐次実行はlib 469件中456 passed／3 failed／10 ignored。A2A loopbackと`failed_replacement_keeps_the_existing_corrupt_package_unchanged`は各単独再実行PASS、`local_tls_server_repairs_only_after_verified_package_bytes`は単独でもOS error 10054／`ConnectionReset`で失敗。原因未確定の既存localhost fixture問題として`FQ-TEST-LOOPBACK`へ記録し、今回のstaging変更による回帰とは観測されず、P11受入れblockerへ加えない。

Schemaは159件／正常156例／負例206件、Conformanceは235 checksでPASS。strict日本語監査は既存履歴／診断文の3 findingsで全体終了値1、今回変更fileのfindingsは0。6つの変更Rust source fileは子moduleを展開しない`rustfmt --check`でPASS。既存書式を保った`workspace_protocol.rs`は呼出し引数1行だけを追随修正し、`git diff --check`もPASS。正式installed product、実配布元download、installed process crash／電源断後のLIVE_RUNTIME Recovery、有効版／Start Menu切替、Installer／Uninstaller、起動／Rollbackの証拠は未成立で、既存Windows配布`release_blocker`を維持する。

2026-10-06 P11部分展開Recovery: Broker consumerは、native Owner確認後の再適用時に固定destination内のstage directoryをno-follow capabilityで開き、packageの各file byte列と既存fileの全prefixを逐次比較する。一致する途中fileだけを残りbyte追記で完了し、完全stageは同じhash／manifest検査を冪等に再実行する。stage全体を走査してpackage外file／directoryやreparse pointも拒否する。相違prefix、異常file、余分なentryは上書き・削除せず保持し、Audit付き失敗でfail-closed。`resume=true`は応答とcompletion Auditへ記録する。権限境界は従来どおり毎回のnative Owner確認、現在trust再照合、Permission、durable Auditを維持する。

部分stage試験: `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib product_package::tests -- --test-threads=1`（8 passed）、`cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib native_owner_apply_stages_exact_signed_package_without_activation_or_overwrite -- --test-threads=1`（1 passed）。Rust test fixtureで途中file prefix、完全stage再要求、相違prefixを保持して拒否、余分entryを保持して拒否、およびBroker consumer経由の既存stage再開を確認。これは実process強制終了・電源断を起こすinstalled product `LIVE_RUNTIME`証拠ではない。

再開処理の統合検証: `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`は全targetで506 passed／0 failed／11 ignored。既存loopback試験を含め今回の実行は全成功。`cargo build --locked --release --manifest-path native/rust_helper/Cargo.toml`はPASSし、未変更`minidora.rs`のdead_code warning 2件を記録。Schema（159件／正常156例／負例206件）、Conformance（235 checks）、Manifest（1171 file）、変更Rust 2 fileの`rustfmt --check`、`git diff --check`もPASS。strict日本語監査は既存3 findings（rev3履歴、rev4履歴、未変更CLI診断文）のみで、今回の変更fileは0 finding。部分stage fixture合格は実installed process crash／電源断の証拠へ昇格しない。

2026-10-06 P11固定root Bootstrapper読取経路: `d4_pocket_active_version.schema.json`と有効版recordの正常／任意executable path負例を追加し、Windows起動器がKnown Folder由来の固定製品rootでのみrecordをno-follow capabilityから読み、version／package hashで導出したdirectory内のversion-local launcherとProduct Manifest hashを照合して起動する分岐を接続した。portable配置とversion-local起動は既存sibling layoutのまま。root／stageのnon-reparse・file identity・同一volume、recordの重複／未知JSON field、identity、version、hash mismatch、required package layoutをfail-closedで検査する。`open_existing_product_root`は起動時にdirectoryを新規作成しない。active recordはAuthorityではない。

局所試験: `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib product_bootstrapper::tests -- --test-threads=1`（5 passed）、`cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib broker::product_install::tests -- --test-threads=1`（5 passed）、`cargo test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`（512 passed／0 failed／11 ignored）、`cargo build --locked --release --manifest-path native/rust_helper/Cargo.toml`、Schema（160 schema／157正常例／207負例）、Conformance（236 checks）、Manifest（1171 file）、`git diff --check`がPASS。初回全targetでは入口識別fixtureに固定起動器fileがなく1件失敗したため、fixtureへ実fileを追加し、focused 5件と全targetを再実行してPASSした。Release buildは未変更`minidora.rs`のdead_code warning 2件を出した。全crate `cargo fmt --check`は今回差分外の多数の既存未整形箇所で終了値1となった一方、変更した`product_bootstrapper.rs`と`product_install.rs`の局所`rustfmt --check`はPASS。strict日本語監査は全体終了値1で既存3件（rev3履歴、rev4履歴、未変更CLI診断）を報告し、今回の変更fileは0件。ここはBroker有効版record writer、native Owner切替操作、初回Bootstrapper配置を接続する前の履歴であり、現行状態を示す記述ではない。

2026-10-06 P11独立有効版record切替: `更新有効版切替要求`をstage operationとは別のBroker operation・native Owner確認で接続した。Brokerは現在の署名候補／trust、package digest／byte長、App／Audit identity、Known Folder由来の固定destination、永続Auditを再照合し、同じ署名packageと既存stageの全byte／inventoryをread-onlyで比較する。不一致stageは修復・削除しない。一致時のみ固定root Bootstrapperを未配置なら配置し、`active_version.json`をtemporary file同期後に同一root renameで公開する。応答はrecord公開までで、process起動、Start Menu変更、旧版削除、Rollbackは行わない。

検証: Windows native Owner dialogのNo／Yes自動操作1件PASS、Bootstrapper初回配置／既存配置保持とactive record公開・置換2件PASS、Broker stage-only／activation分離fixture 1件PASS、read-only stage検証1件PASS、Flutter `test/update_client_test.dart` 4件PASS、変更Dart 4 file format確認PASS。最初の全Rust実行では新規Windows test fixtureがdirectory capabilityを開いたまま掃除して2件失敗し、handleを閉じる修正後にfocused 2件PASS。既存download TLS testはWindowsでbody受信前にserver threadをjoinするとWSA 10054となったため、test fixtureの順序だけを修正しfocused testはPASS。最初の統合再試行で変更外MINIDORA localhost testが一度失敗したが、focused再実行と全target再試行ではPASS。最新`cargo test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`は517 passed／0 failed／12 ignored。通常installed productのnext-launch、Installer／Uninstaller、Start Menu、crash／電源断Recovery、Rollbackおよびrelease readinessの証拠ではなく、P11はOPENを維持する。

追加検証: `python -X utf8 tooling/schema_check/check_schemas.py`は160 schema／157正常例／207負例、`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`は236 checks、`python -X utf8 tooling/manifest.py --write`後の`--check`は1175 fileでPASS。変更Dart 3 fileの`flutter analyze --no-pub`は問題なし、Windows `flutter build windows --debug --no-pub`は成功しRunner C++を含むdebug executableを生成した。既存build cacheが過去の`Z:/apps/desktop_flutter`絶対pathを保持していたため、削除せず一時外部build directoryを使い、Flutter設定を元の未設定状態へ復元した。Flutter全体analyzeは変更外`agent_center.dart`の既存deprecated API info 5件で終了値1。Windows debug buildはinstalled product、署名済み配布物、next-launch、Installer、LIVE_RUNTIMEの証拠ではない。`git diff --check`もPASS。

2026-10-06 P11有効版切替のStart Menu接続: native Owner確認に固定Start Menu shortcut登録先を追加し、Brokerが実行時に現在利用者のKnown Folder、App ID、固定rootと照合する。完全検証済みstageから固定root Bootstrapperを配置し、有効版recordを公開した後、そのBootstrapperを指すshortcutをBroker capability経由でcreate-only登録する。同targetへの再実行は冪等、既存の別target shortcutは上書きせず拒否する。登録失敗時は失敗Auditを残し、stage／active record／既存shortcutを自動削除・復元しない。COMへ渡すpathだけextended Windows path prefixをShell互換形式へ正規化し、path比較・Broker境界の検証値は変えない。

検証: `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib broker::product_install::tests -- --test-threads=1`は8 passed（shortcut作成、target照合、冪等再試行、競合非上書きを含む）。`cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib broker::update_center::tests::native_owner_apply_stages_exact_signed_package_without_activation_or_overwrite -- --test-threads=1`は1 passed（stale shortcut先拒否、有効版record／shortcut登録、再試行）。native Owner確認の実Win32 dialog自動操作も1 passed（No／Yes）。`cargo test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`は517 passed／0 failed／12 ignored。最初のshortcut作成試験ではWindows canonical pathの`\\?\` prefixをShell Link `SetPath`が拒否したため、COMへ渡すpathのみ正規化してfocused試験と全targetを再実行した。

Desktopの`update_client_test.dart`は4 passed。通常OneDrive worktreeからのFlutter testはCloud Files reparse placeholder `build/unit_test_assets`をFlutterが削除できず終了したため、このdirectoryを変更せず、同一source／testとlocal packageだけを一時scratchへ複製して再実行しPASSした。変更対象Dartの`dart format --output=none --set-exit-if-changed`と限定`flutter analyze`は変更なし／問題なし。Desktop全体analyzeは未変更`agent_center.dart`の既存deprecated情報5件で終了値1、Mobile全体analyzeは問題なし。Windows `flutter build windows --debug --no-pub`はPASSし、外部scratchから生成したdebug executableでDesktop変更をcompile確認した。Flutter build-dir設定は元の未設定へ復元した。Schema（160／157／207）、Conformance（236）、Manifest（1175）もPASS。厳格日本語監査は既存履歴／診断文3 findingsのみで、今回変更fileに新規findingなし。これらはBroker fixture／debug buildの証拠であり、installed product上のStart Menu・次回起動、Installer／Uninstaller、Rollbackの証拠ではない。P11はOPENを維持する。

2026-10-06 P11 Broker記録の直前版への復帰: Windows版Rust起動器で独立したnative所有者確認を行い、Brokerが現行版・直前版候補の署名と内容識別値、製品／監査識別子、固定導入先、両版の展開内容を再検証する。永続監査に意図を記録した後、`active_version.json`だけを原子的に切り替える。統合試験では1.1.0から1.0.0への復帰、古い対象と通常IPCの拒否、展開済み版の保持、Start Menu不変更、完了後の正確な内容識別配布物だけの除去を確認した。native確認画面の拒否／承認試験1件、更新制御15件、起動器8件、配布物除去1件、Flutter更新画面4件が合格。最終`cargo test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`はRust library 476 passed／0 failed／12 ignored、全target合計522 passed／0 failed／13 ignored。中間集約試験ではCodex loopbackがHTTP応答見出しの到着前に`ConnectionReset`となったが、単独再試験1件と後続全targetは合格した。別の中間回では`broker_ipc`が9 passed／1 failedだったが、後続全targetでは10件すべて合格した。P11変更との因果は観測されず、loopback事象は既存`FQ-TEST-LOOPBACK`へ記録する。Rust Release buildは合格（未変更`minidora.rs`の未使用code warning 2件）。Conformanceは当初、inline `#[cfg(test)]`で実装sourceを誤って切り落としていたため検査器をtest module境界で分割し直し、236 checks合格。Schema 160／157／207も合格。Desktop変更対象の限定analyzeとDart形式確認は合格。全crate Rust形式確認は既存の多数の未整形差分により終了値1で、全体整形は行っていない。厳格日本語監査は既知3 findingsのみ。これはBroker試験とnative画面境界の証拠であり、installed productでの復帰／次回起動、Installer／Uninstaller、P11完了を意味しない。P11はOPEN、配布`release_blocker`を維持する。

2026-10-06 P11 version-local起動器の導入path照合: Rust起動器の通常起動経路で、自身の実行fileをKnown Folder由来の固定product rootにあるactive recordの選択launcherと照合し、App ID／Audit Store ID、版directory、launcher／Product Manifest hashを再検証する。一致時だけBrokerのSetup Doctor向けinstalled-path観測を`LIVE_RUNTIME`として構成する。portable起動、別identity、root Bootstrapper本体、無関係launcherは未確認のままにする。この観測はSetup Doctor表示専用で、Authorityや配布元trustを作らない。focused test 1件PASS、Schema 160／157／207 PASS、Conformance 236 checks PASS、Manifest 1175 file PASS、Release build PASS（未変更`minidora.rs` dead_code warning 2件）。Rust helper全testは475 passed／1 failed／12 ignoredで、失敗した既存Codex loopback testは単独再実行で1 passedし、既存`FQ-TEST-LOOPBACK`と同分類。strict日本語監査は今回変更fileにfindingなし、既存履歴・診断3 findingsで終了値1。これはinstalled productの一連の起動LIVE_RUNTIME証拠ではない。正式Installer／Uninstaller、Start Menuからの製品起動、Repair、P11受入れは引き続き未成立。

2026-10-06 P11初回Install／Update導線: BrokerはRollback先のないactive versionも現行candidate／trust照合後に`現在版`へ投影する。Flutterは`unavailable + 現在版=null`を初回Install、active versionありをUpdateとして区別し、同じdownload jobの更新ID・候補hash・package SHA-256が一致しないstage要求を無効化する。Brokerが`version_staged`を返した場合だけ画面内のstage印を置き、別Owner確認による有効版切替を有効化する。このUI stateは表示制御だけでAuthorityではなく、Brokerは各操作で再検証する。Flutter update client 7件、Broker導入fixture 1件、変更file analyze、Dart形式、Schema 160／157／207、Conformance 236 checks、Release build、Manifest 1175 fileがPASS。Mobile全体analyzeはPASS。Desktop全体analyzeは未変更`agent_center.dart`のdeprecated info 5件のみで終了値1、変更対象`settings.dart`／`update_client.dart` analyzeはPASS。Rust全testは474 passed／2 failed／12 ignored。失敗した既存A2A／MINIDORA loopback testは個別再実行で各1 passedし、`FQ-TEST-LOOPBACK`へ送る。strict日本語監査は今回変更fileにfindingなし、旧履歴・既存診断の3 findingsで終了値1。初回導入からinstalled product起動までのLIVE_RUNTIME、正式Installer／Uninstaller、Repairは未成立で、P11 OPENを維持する。

2026-10-06 P11 配布元catalog取得導線: Desktop Update Centerの明示操作から新しいBroker operation `更新候補取得`を呼び出し、Broker所有trustに事前登録されたchannel別HTTPS配布元の`updates.json`だけを取得する経路を追加した。Flutterからの直接network access、候補指定URL、redirect、system proxy、自動retry、圧縮応答を使わず、HTTPS証明書／hostname検証、public-address DNS pinning、各catalog 64 KiB／一回の全配布元取得30秒の上限を適用する。catalogはSchema版1・重複field拒否で解析し、全sourceの取得に成功した後、各候補のchannel／更新ID一意性とBroker Ed25519署名を検証したものだけを一括永続化する。いずれかのcatalogが不正なら既存候補を変更しない。利用者明示のread-only metadata取得を`更新候補取得` capability、Broker所有`package_sources`／HTTPS／public-address境界をpermissionとして記録する。package download／Install Approvalは不要なまま独立native Owner確認を維持する。Auditは配布元件数・catalog hash・候補件数だけを記録し、本文や資格情報を保存しない。失敗時は既存候補を保持し、trust／配布元確認後の明示再試行をRecoveryActionとする。未構成時は通信せず`unconfigured`を返す。

検証: Rustの`broker::update_center::tests`19件と`broker::update_download::tests`17件がPASS。Schema 161件／正常例157件／負例208件、Conformance 236 checks、Manifest 1175 fileもPASS。Rust Release buildはPASS（未変更`minidora.rs`にdead_code警告2件）。Desktopの`settings_compose_test.dart`と`update_client_test.dart`は製品版識別値を明示した一時作業複製で計9件PASS。変更対象Dart 2 fileの`flutter analyze --no-pub`とWindows Desktop Debug buildもPASS（中間出力先に関するMSB8029警告）。初回のFlutter実行では既存Compose試験が期待する製品版識別値を渡さず1件失敗したため、値を明示して再実行した。HTTPS上限の異常系試験はCargo試験並行中に一度Windows socket error 10054で接続切断したが、単独および逐次再実行でPASS。厳格日本語基底監査は全体終了値1で、既存の旧履歴2件と既存Codex CLI診断1件のみが残り、今回変更fileの指摘は0件。導入済み製品の実配布元接続、正式Installer／Uninstaller、導入済み起動・Rollback、Repair、正式配布判定を証明するものではなく、P11はOPENを維持する。

2026-10-06 P11 Windows payloadの現行main再build: clean `main`／`origin/main`一致commit `3a578519a2d4364a2f6fc0478b49474612fce2e3`から、checked-in合成Export Receipt／Manifest fixtureを用いて`tooling/export_windows_product.py`を実行した。Receipt指定のManifest file名と実file名が異なる初回試行は入力検査で停止し、内容を変えずに一時directory内で期待basenameへ複製して再実行した。Flutter Windows Releaseはbuild成功（MSB8029一時directory警告）、Rust Release Broker／起動器もcompile成功（未変更`minidora.rs`のdead_code警告2件）。生成bundleは14 file、63,571,666 bytes、artifact tree SHA-256 `8326087ea2191dec71421e1af99da0c7b4cd7964b4efe83dab1535c19aae9fb2`。続けて`tooling/package_windows_product.py`がD4PKG01を生成し、14 file／63,573,871 bytes、package SHA-256 `8ecee8358325d5ac0ac167eb1efa92e5c90301c2dc27901574fd927a94a6126a`を報告した。成果物はTemp内の隔離開発artifactで、synthetic Receipt／Manifest由来の試験identityを持ち、Owner承認済みExport、package署名、実インストール、起動、Update／Rollback、正式配布の証拠ではない。P11はOPENを維持する。

2026-10-06 P11 portable起点Install後の導入版起動遷移: Brokerが`更新有効版切替要求`を`Accepted`し、`有効化=active_version_recorded`を返した場合に限り、portable起動元のRust起動器が応答へ`起動=after_current_exit`を付加する。製品identityがないbuild、導入済み版からの更新、rollback、拒否応答では付加しない。Flutterはこの明示応答後に案内を表示して画面を正常終了し、Rust起動器はBroker停止と現在processの正常終了を待つ。続いて固定rootからactive version launcher／package layoutを再照合し、instance lock解放後に限定環境で起動する。起動先Brokerは検証済みinstalled entrypointの起動を`LIVE_RUNTIME` Auditへ記録する。初回Install後の起動遷移とSetup Doctor provenance表示をつなぐ実装であり、実installed productを通した一連のLIVE_RUNTIME成功を示すものではない。

検証: Rust focused `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib activation -- --test-threads=1`は7 passed／0 failed／1 ignored。`cargo build --locked --release --manifest-path native/rust_helper/Cargo.toml`、Flutter `flutter test --no-pub --dart-define=GUI_SHELL_PRODUCT_VERSION=1.2.3 test/update_client_test.dart`（8件）、変更Dart 2 fileの`flutter analyze --no-pub`、変更3 fileのDart format確認、`flutter build windows --debug --no-pub --dart-define=GUI_SHELL_PRODUCT_VERSION=1.2.3`、Schema（161／157／208）、Conformance（236 checks）、Manifest（1178 file）がPASS。Debug buildはTemp作業複製から成功し、MSB8029（一時directory build warning）を出した。Rust `--all-targets`逐次実行のlibrary suiteは484 passed／1 failed／12 ignoredとなり、Cargoは当該library targetの失敗後に非zero終了した。失敗は今回の変更file外の既知`broker::update_download::tests::local_tls_server_repairs_only_after_verified_package_bytes`（Windows loopback TLSの`ConnectionReset`／`InvalidContentType`）であり、現行`FQ-TEST-LOOPBACK` Final QA分類に留める。日本語基底strict監査は今回変更fileにfindingなし、既存rev3／rev4履歴とCLI診断文の3 findingsにより全体終了値1。installed product実起動、正式Installer／Uninstaller、Repair、実配布署名は未成立で、P11 OPENとWindows distribution `release_blocker`を維持する。

起動許可はBroker応答status・active record公開・元要求とのrequest ID／operation一致へ結合し、staleまたは別operation応答からは生成しない。

2026-10-06 P11 初回Install引継ぎの重複防止と終了失敗表示: Update Centerの状態変更を一件に直列化し、要求中はcatalog取得、download、stage、active version切替、延期、rollback、Uninstallを無効化する。Brokerが初回Installのactive version record／Start Menu登録を受理し`起動=after_current_exit`を返した場合、Flutterは必須終了を一度だけ要求する。終了が拒否／失敗した場合は導入操作そのものを失敗と誤認せず、画面内操作を再開してStart Menu起動を案内する。`_loadUpdates`の`setState` callbackからFutureが返る例外も修正した。

検証: `flutter test --no-pub --no-test-assets --dart-define=GUI_SHELL_PRODUCT_VERSION=1.2.3 test/settings_install_flow_test.dart test/settings_compose_test.dart test/update_client_test.dart`は12件PASS。testはBroker transport fixtureと注入した終了応答を使い、初回stage→切替→必須終了要求、二重tapの単一要求化、Uninstall中の状態変更直列化、終了拒否後の未削除表示と画面操作復帰を検証する。実Broker、Rust起動器、installed product起動のLIVE_RUNTIME証拠ではない。Schema 161件／正常例157件／負例208件、Conformance 236 checks PASS。Mobile全体`flutter analyze --no-pub` PASS。Desktop全体解析は変更外`agent_center.dart`の既存deprecated API info 5件で終了値1、変更ファイルにfindingなし。OneDrive上のFlutter C++ wrapperがCloud Files placeholderとなり通常checkoutのWindows buildは生成source file欠落で失敗した（source repoとOneDrive設定は変更せず）。tracked Flutter appとpath dependencyだけを生成cache・ephemeral directoryを除外してASCII Tempへ複製し、`flutter pub get`と`flutter build windows --debug --no-pub --dart-define=GUI_SHELL_PRODUCT_VERSION=1.2.3`を実行、Windows Debug executable生成までPASSした。MSB8029一時build directory警告あり。これはhost-local compile証拠でありinstalled product起動やLIVE_RUNTIME証拠ではない。Strict日本語監査は既存履歴／診断3 findingsのみで全体終了値1、今回変更fileのfindingsは0。これはOneDriveの同期／生成物配置制約で、製品回帰やrelease blockerには分類しない。

## 3. 後続製品工程

P2 Multi-Agent Compare、P3 Handoff、P4 Provider / Model Center、P5 Workspace / History / Evaluation、P6 Credential / MCP、P7 A2A / Host / Adapter、P8 GUI-Shell Compose、P9 Standalone Export、P10 Module Selection / PruningはProduct Build受入れを閉鎖した。現行はP11 Windows Productization。続いてP12 Product Integrationを進める。P12の統合経路成立をWindows Feature Completeとし、その後にQ0〜Q7 Final QAへ移る。P13 Mobile / Non-WindowsはWindows Feature Completeを止めず別trackとして扱う。Owner Finalizationではproduction Publisher／signing identity、production Audit key、不可逆な事業判断、Final GOだけをOwnerへ戻す。

## 4. 関連正本

- `ROADMAP.md`: rev5工程への入口と過去作業履歴
- `docs/FINAL_QA_QUEUE.md`: Feature Complete後へ移送した検査項目
- `docs/REV4_ACCEPTANCE_LEDGER.md`: rev4受入れ履歴。rev5現行状態の正本ではない
- `docs/REV3_PROGRESS.md`: rev2/rev3の作業・失敗・実行証拠の履歴
- `release_blockers.registry.json`: release gateの証拠・分類。開発phaseの進行順と同一視しない
