# D4 Pocket / GUI-Shell rev5 Product-First 進捗

更新日: 2026-10-05
工程正本: ユーザー提示「D4 Pocket / GUI-Shell 統合実装仕様書 rev5」「統合開発工程表 rev5」「Codex実装指示書 rev5」
現行phase: `P7 A2A / Host / Adapter` (`OPEN`)。`P6 Credential / MCP`は2026-10-05にProduct Build受入れを閉鎖。`P5 Workspace / History / Evaluation`も同日に閉鎖済み。
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

### P7 対A2A連携／複数Host／Adapter管理 — OPEN

現行rev5の範囲はA2A、Multi Host、Adapter Manager、Host capability、接続性、degraded mode、local／remote Runtimeの区別である。P6を再訪せず、現行rev5 Acceptanceに対して実装済み経路と未成立条件を確認して順に進める。過去版の状態を現行受入れへ継承しない。

#### P7内の完結単位: A2A loopback接続面 — CLOSED（Product Build）

- 必須`shell.agent_operation`へDesktop専用A2A接続センターを追加。専用画面、全体検索、コマンドパレット、NavigationRailから到達し、optional Moduleを全て除いたbuildでも表示する。
- 接続要求はFlutterから既存Broker IPCへ送り、Rust起動器のnative Owner確認を経る。BrokerもA2A接続をnative確認専用とし、通常IPC／Owner credential要求を拒否する。Owner対象表示とBroker helper双方で、loopback IPv4 HTTP、query／fragment／userinfoなし、未対応Credential ref、Task等を送らない境界を検査する。Windows Runnerのnative確認応答timeout allowlistにもA2A操作を追加。
- Brokerが既に持つAgent Card取得・永続Audit・接続state・metadata-only一覧をDesktopへ接続。画面は接続先URIを成功後に消去し、endpoint hashとbounded metadataを表示する。Trustは`pending_review`を維持し、Permission／Approval／AuthorityやA2A Task実行へ昇格させない。
- Agent Card由来の表示文字列は未信頼として表示し、改行／方向制御文字を含む値をUI clientで拒否する。認証実値は要求へ含めない。
- 個別検証結果: RustのA2A境界試験9件、native Owner確認候補1件、Brokerのnative確認専用gate 1件、Flutter接続service／画面／検索試験9件、任意Moduleをすべて無効にしたNavigationRail試験2件、`Conformance` 234件、Windows版Desktopのdebug build成功（MSB8029の一時作業フォルダー警告あり）。Flutter試験とWindows buildはOneDrive外の`ASCII`一時複製で実行。
- Rust全targetは444件中435成功／2失敗／7 ignored。今回のA2A関連11件は全件成功。失敗のうち`failed_tool_result_is_not_replayed_as_another_exec_command`は単独再試験で成功し、`local_tls_server_repairs_only_after_verified_package_bytes`は単独でもHTTP/TLS fixtureの応答不整合とConnectionResetで失敗した。いずれも今回変更したA2A経路外で、既存`FQ-TEST-LOOPBACK`へ追加記録し、このA2A受入れを拡張しない。
- Strict日本語基底監査は終了コード1で、現行変更fileは0 finding。既存findingは3件（旧rev3文書1、旧rev4文書1、未変更Codex CLI診断文字列1）。過去文書・無関係Adapter診断は今回のA2A Acceptance外として保持し、監査全体をPASS扱いしない。
- rev5 Product Buildの基準（実装、build、基本正常経路、次工程からの利用、Authority非破壊）を満たしたため、このloopback UI接続単位をCLOSEDとする。C16の既存Desktop画面未成立記述を現行状態へ訂正した。P7全体はOPENで、次は現行rev5のMulti Host／Host capability／Adapter Managerの未成立範囲へ進む。外部A2A transport／実作用とinstalled product証拠は別gateであり、この受入れを拡張しない。
- この単位だけでP7全体は閉じない。Multi Host、Adapter Manager、Host capability／接続状態／degraded mode／local・remote区別のrev5受入れは引き続きOPEN。Windows installed product全経路証拠とA2A外部transport／Task等の別gateも未主張。

## 3. 後続製品工程

P2 Multi-Agent Compare、P3 Handoff、P4 Provider / Model Center、P5 Workspace / History / Evaluation、P6 Credential / MCPはProduct Build受入れを閉鎖した。現行はP7 A2A / Host / Adapter。続いてP8 GUI-Shell Compose、P9 Standalone Export、P10 Module Selection / Pruning、P11 Windows Productization、P12 Product Integrationを進める。P12の統合経路成立をWindows Feature Completeとし、その後にQ0〜Q7 Final QAへ移る。P13 Mobile / Non-WindowsはWindows Feature Completeを止めず別trackとして扱う。Owner Finalizationではproduction Publisher／signing identity、production Audit key、不可逆な事業判断、Final GOだけをOwnerへ戻す。

## 4. 関連正本

- `ROADMAP.md`: rev5工程への入口と過去作業履歴
- `docs/FINAL_QA_QUEUE.md`: Feature Complete後へ移送した検査項目
- `docs/REV4_ACCEPTANCE_LEDGER.md`: rev4受入れ履歴。rev5現行状態の正本ではない
- `docs/REV3_PROGRESS.md`: rev2/rev3の作業・失敗・実行証拠の履歴
- `release_blockers.registry.json`: release gateの証拠・分類。開発phaseの進行順と同一視しない
