# D4 Pocket / GUI-Shell rev5 Product-First 進捗

更新日: 2026-10-05
工程正本: ユーザー提示「D4 Pocket / GUI-Shell 統合実装仕様書 rev5」「統合開発工程表 rev5」「Codex実装指示書 rev5」
現行phase: `P4 Provider / Model Center` (`OPEN`)。`P3 Agent Handoff`は2026-10-05にProduct Build受入れを閉鎖。
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

### P4 提供元・模型管理（Provider / Model Center）— 開始

rev5工程表の次phaseへ移行。Provider registry、Model registry、BYOK、capability、選択、health、fallback表示を現行実装とAcceptance Contractに照合し、一つ以上の実経路またはtest providerで成立させる。全Vendor網羅を開始条件にせず、P3の深いfailure matrixはFinal QAへ維持する。

## 3. 後続製品工程

P2の受入れ後はrev5工程表の順にP3 Handoff、P4 Provider / Model Center、P5 Workspace / History / Evaluation、P6 Credential / MCP、P7 A2A / Host / Adapter、P8 GUI-Shell Compose、P9 Standalone Export、P10 Module Selection / Pruning、P11 Windows Productization、P12 Product Integrationを進める。P12の統合経路成立をWindows Feature Completeとし、その後にQ0〜Q7 Final QAへ移る。P13 Mobile / Non-WindowsはWindows Feature Completeを止めず別trackとして扱う。Owner Finalizationではproduction Publisher／signing identity、production Audit key、不可逆な事業判断、Final GOだけをOwnerへ戻す。

## 4. 関連正本

- `ROADMAP.md`: rev5工程への入口と過去作業履歴
- `docs/FINAL_QA_QUEUE.md`: Feature Complete後へ移送した検査項目
- `docs/REV4_ACCEPTANCE_LEDGER.md`: rev4受入れ履歴。rev5現行状態の正本ではない
- `docs/REV3_PROGRESS.md`: rev2/rev3の作業・失敗・実行証拠の履歴
- `release_blockers.registry.json`: release gateの証拠・分類。開発phaseの進行順と同一視しない
