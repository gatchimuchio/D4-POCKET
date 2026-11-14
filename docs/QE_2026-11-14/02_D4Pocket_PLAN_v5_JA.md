# D4Pocket 統合開発工程表 v5.0 — 製品先行・全系一巡

文書ID=`D4-QE-PLAN-20261114-V5`／対象=`gatchimuchio/D4-POCKET`／作成=`2026-11-14 JST`／観測基準=`4f1eab7e9520e7a6ee68ceba266433ec5ef175d9`。
状態=`v5工程正本として採用・製品施工未着手`。仕様=`01_D4Pocket_SPEC_v5_JA.md`、施工命令=`03_D4Pocket_CODEX_v5_JA.md`。先行v4は履歴、安全契約・既存証拠の状態は不変。採用前main=`eccdf02f5a8a1b44475a470e59e08747f285dee5`。

## R00 工程の一文定義

`全要求の母集団と境界を一度で把握 → A〜J全躯体を一巡で実施工 → 全Producer/Consumerを結合 → 欠落の全数照合 → 統合実物QC → 標準OS仮想QA/Windows最終実機確認 → Owner独立GO`。

R00-01 施工対象はD4Pocket**完成製品**である。現3点セットの採用、21列台帳、CI緑化、コミット増加、W02単独fixtureは工程目的でない。
R00-02 指示者=ご主人様／設計者=チャッピー／施工者=Codex／製品使用者=エンドユーザー／実行Authority=Rust Broker／出荷責任者=ご主人様。各権限は非合算。
R00-03 全母集団=`N01..11, K01..20, P0..13, C0..34, Phase0..45, BUF-R01..09, BUF-T01..24, B0..6, D4-QE-R01..13, D4-LNX-T01..08, D4-AUD-F01..07, FQ全件, W00..08, Q0..07, 18Blocker, Windows/macOS/Linux/Android/iOS`。旧IDは施工順でなく履歴・要求・受入座標。
R00-04 工程の進行は実依存・安全・単一writer衝突のみで制約し、ID順・文書順・旧Gate番号順で直列化しない。
R00-05 通常の安全確認（Authority/Approval/Permission/Credential/Audit/Recovery/Host/IPC/Path/非冪等性）はBUILD中も即検査。**最終QC/OS QAの全套を局所編集ごとに繰返すこととは区別**。

## R01 工程状態遷移と解除条件

| 段 | 意味／出力 | Exit／次段への許可条件 | 禁止された代用 |
|---|---|---|---|
| E0 着工同期 | 現HEAD/採用版/保護作業、既存全要求ID母集団、共通Contract・責任境界、A〜J施工先、明白な安全障害を**一回で識別** | `E0_EXIT={現行正本+最新Source位置確認,全ID範囲固定,A〜Jの実装先・契約境界定義,重大安全衝突の局所隔離}`。不足SourceSHA・実Consumer・個別試験は`BUILD_GAP/UNKNOWN`でE1へ引継ぐ | 21項目×全ID台帳を完成してからE1、E0単独のK別コミット、同じ旧W00の再実行 |
| E1 `WHOLE_BODY_BUILD` | A〜Jの全線について現存実コードを継承し、欠落機構・製品利用入口・各OS Native/Installer・Export・Buffer・Evidenceの**本番Sourceを一巡で施工** | 全A〜Jに必要な実製品Sourceと次Consumerへの実接続先/契約が存在。欠落はE2対象として列挙 | 1機能をQA完了して次へ、Stub/Manifestだけの製造完了、Docs-only実績 |
| E2 `WHOLE_CONNECT` | AI実Task→Broker→実Runtime/MCP/A2A→実結果→同AI次Task/UI/Audit/Recovery、Compose/Edit/Exportと五OS配布を全線接続 | 代表縦断`SYSTEM_SHAPED`観測＋全接続対象の欠落一覧。安全未成立作用は隔離 | W02合格までB〜J待機、UI描画/Fixtureを実Consumerと称する |
| E3 `BUILD_COVERED` | 全原ID・N/K/BUF/QE/FQ/OS/18Blockerへ実Producer/Consumerと拒否/Recovery、Package経路を逐件突合 | **必須未実装Consumer/配布生成経路の`BUILD_GAP=0`**。詳細ledger21項目・Source/Contract/Consumer/否定・回復・根拠をここで閉じる | 台帳行が埋まっただけで実コードなし、CODE_PRESENT≠WIRED |
| E4 `SYSTEM_QC` | 全体候補をQ0で固定し、通常/異常/境界/安全/資源/持久/副作用/各OS導入経路を集中的に検査・根因是正 | 不具合・影響consumer・同条件回帰・未確認・残余が候補版に帰属 | 小修正ごとにQ0..Q7全周回、試験削除で緑化 |
| E5 `SCOPED_QA` | Q1..Q6の全体/故障/安全/性能/OS/証拠判定。全五OS仮想installed product、Windows所有実機確認 | 宣言対象OS・版/arch・Artifactごとに`QA_SCOPED/QA_NOT_ESTABLISHED/OUT_OF_SCOPE`を判定 | 他OS実機必須、単なるcross compile、Simulator結果を実端末動作として主張 |
| E6 `RELEASE_REVIEW` | Q7、18Blocker、正式配布/署名/Publisher/Store/監査/Owner GO | Ownerが明示的に対象Release trackへGO。無ければ`release_ready=false` | `QA_SCOPED`だけで公開、署名資格をCodexが自作 |

R01-01 `E1/E2`は物理的に重複してよい。`E0→E1`移行で必要なのは**着工できる意味と安全境界**であり、未実装のConsumerを先に実証することではない。
R01-02 `E3`の詳細照合はBUILD後の**全数・厳格監査**であって省略不可。旧ID原定義と実体の差があればE1/E2へ原因戻しし、全数監査の合格下限を落とさない。
R01-03 `E4/Q0`からの正式全体候補検査はE3後に一度開始する。ビルド中の限定compile/契約検査・安全否定試験を「正式Qの開始」と記録しない。

## R02 E0 着工同期：完了させる物は4件だけ

`E0-OUT-A 現物座標`：現User原文・採用v4・本v5の採否・AGENTS・日本語正本・HEAD/remote/dirty/他writer/二世代復旧ref・既存進捗/Blockerの**現在座標**を1回読み固定。現HEADが4f1eab7と違えば差分だけ再観測。旧日付数値を現在値へ転記禁止。
`E0-OUT-B 全母集団ID索引`：R00-03の**全範囲・個数・原出典・未解決の存在**を既存`docs/QE_2026-11-13/E0_要求consumer台帳.md`等から抽出。K01..K14など**既存の85コミット成果は既読出典として継承**。K15..20、C/Phase、OS/Artifactなどの全詳細をここで新規21列完成させない。UNKNOWNを消さずE3対象に固定。
`E0-OUT-C 共通構造`：Shell Core/Agent Runtime/Rust Broker/Adapter/MCP/A2A/Flutter/Native/Buffer/Compose/Export/Installer/証拠の責任・入口/出口・Safety invariants・A〜Jの初回実装ファイル候補を列挙。技術的依存を確認するが製造前の実Consumer SHAまで固定しない。
`E0-OUT-D 安全・writer・適用矛盾`：何が危険なのか・どの作用だけ停止するのか・独立線が進められるか・過去AGENTS/v4の規則衝突を判別。**安全上の修復を含めてE1へ入る**。書類の全規則を統治一括修復してから製造するとする旧v4の指示を撤回。

R02-01 E0の終了条件は**4出力の有無**であり、個別ID台帳の埋まり率ではない。E0出力を数百行の再整理・台帳行増殖へ拡大することを禁止する。
R02-02 E0の後に`docs-only / ledger-only`を独立施工サイクルとして重ねない。正本採用の最小記録を除き、以後の意味あるBUILD_BATCHには必ず**実製品Source/実Consumerの有効変更**を含める。無用な空編集・仮stub・テストのみは要件違反。
R02-03 コード施工前に必要なContractの核（Task/Result/Capability/Permission/Approval/Identity/Audit/Recovery/OS interface）の意味を固定。既存Contractを再利用し、影響consumerはBUILDと同時に改修。安全が不明な当該作用以外の線を止めない。
R02-04 v5の正式採用・AGENTS等の最小同期は**一回の制御された初期統治変更で閉じ、次の製品施工委任に追加の上流改訂を要求しない**。本件のように指示者が統治文書整合のみを委任した場合、製品Source施工は別件かつ進捗0として記録する。最初の実Source BUILD_BATCHでは既採用v5を再作成せず実物着工へ直行する。保護された外部権限を回避しない。

## R03 E1：初回全躯体BUILDの実施工母集団

| 線 | 本番Source構築の指示（既存実装は継承） | 連接先・最低要素 |
|---|---|---|
| A | AI Task/Session/Runtime-neutral Tool Call/Result-fed next Task/Cancel/Deadline | B/F/C/G/I、N01/N02、旧W01/W02 |
| B | Runtime Catalog/2種目実Runtime/A2A/Host/Compare/Handoff | A/C/D、N03/N04、K02/K06/K11/K12 |
| C | Agent Center/Workspace/History/Diff/Eval/Process/Trace/Notification/Recoveryの実状態Consumer | A/B/F/G/H/I、N09、K03/K04/K05/K10/K13 |
| D | Capability実Catalog→依存/Permission→Compose→実Tool、AI Edit独立検査→Broker制御適用 | A/E/F、N05/N07、K14 |
| E | Standalone Export/独立identity/Broker/必要module/実pruning/OS package生成 | D/F/G/H/I、N06、K15、FQ-MODULE-PRUNING |
| F | Rust Broker・Credential/IPC/Stop/Audit/Recovery＋Buffer B5自己適用・B6結合 | A〜J全線、N08、K07/K20、BUF/B/FQ |
| G | Windows正規Installer/Launcher/SetupDoctor/Update/Repair/Rollback/Uninstall | A/C/E/F/J、N10、K16、Windows Blockers |
| H | Linux native package+X11/Wayland宣言、macOS Bundle/Keychain/Native IPC/UI/Update | A/C/E/F/J、N10、K17、LNX-T |
| I | Android/iOS Native Device Link/Keystore/Keychain/permission/lifecycle/OS配布 | A/C/E/F/J、N10、K17、Mobile Blockers |
| J | Requirement/Artifact/Release identity/evidence collector/OS matrix/Blocker/Validator接続 | A〜I、N11、Q/FQ/18Blocker |

R03-01 A〜Jの順序は表記順ではない。技術依存上A/F先行の局所施工が要る場合も、B/C/D/E/G/H/I/JのSource着工を特定W/OS/Qの完成まで延期しない。必要なら複数の独立source群を一つの横断BUILD_BATCHへ束ねる。
R03-02 **初回一巡の終了条件**=`各A〜Jの製品側Sourceと実利用側Consumerに本番作用へ繋がる有効な実装差分があり、欠落/未接続は具体的E2の対象として見える`。この時点で全機能の最終実動合格を要求せず、Stub/Mockしかなければ満たさない。
R03-03 OS標準配布経路はE1に**同時実装**し、Installerだけ後回し、Mobileだけ後回し、Exportだけ後回し、Bufferだけ後回しを禁止。単一ソースで共通意味、OS Bridgeで差異を吸収。
R03-04 既存W01 Windows normal Release/Codex CLIの成立範囲を保持。W02のinstalled MCP→AI次Task/UI hangをA/C/F/Gの共有不適合として修正し、W02小試験だけを無限反復しない。K14のProposal、N05のManifest、N06のExporter shell、N03のmetadata-onlyも本番Consumer欠損を埋める。
R03-05 既存成果の再実装は要求のない限り禁止。`REUSE_EXACT/REWIRE/REIMPLEMENT/UNKNOWN`に基づき**必要な実差分**だけ作る。実装されている線をダミー修正して「全線に触れた」と捏造しない。既済線は既存Source/実Consumerの証拠で充足可能、欠落線は実コードで埋める。

## R04 E2：全横断配線と全体作用

R04-01 代表通電経路=`製品User目的→実AI Task→Capability→Broker authenticated IPC→Permission/Approval→実MCP Tool or CLI→Result/Audit→同Agent次Task→UI/History/Recovery`。Fixture/Mock/局所Unit成功はこの観測の代替でない。
R04-02 異種Runtime2種、外部A2A/Host、Agent間Compare/Handoff、Compose→元Tool、AI Edit→独立承認Apply、Standalone Export→独立実仕事、Mobile Device Link、全OS Launcher/Installerを同じ契約境界の**独立縦断消費者**として接続。
R04-03 契約の更新では全影響Consumerを漏らさず、局所的UI hardcode/権限の多重経路/別OSへ欠陥転嫁を禁止。未知/未実装は`BUILD_GAP`と明示してE1/E2で是正。
R04-04 `SYSTEM_SHAPED`は縦断**一仕事**観測、全完成を意味しない。成立した瞬間にB〜Jの工事を終えることも、成立待ちで他線を止めることも禁止。

## R05 E3：全数照合はここで完成させる

R05-01 母集団=仕様S04〜S07および過去ID/18 Blocker全件。`docs/D4_POCKET_PHASE_MAPPING.md`のC35/Phase46行と、既存K台帳20群、旧P14行を逐件参照し、実Source・実Consumer・契約/拒否/Recovery・OS Packageへ照合。
R05-02 詳細ledger項目=`{現ID,旧ID/出典,主語,目的,Producer,実Consumer,契約,正常作用,異常/否定作用,停止/復旧,OS/arch,配布物,SourceSHA,ArtifactSHA,TestClass,既存能力,BUILD_GAP,Blocker,履歴証拠,現状態,残余}`。**E0で完成要求しない**。E1/E2で得た実体から記録し、対応実体がない場合は台帳値の工夫で埋めず製造へ戻す。
R05-03 `BUILD_GAP_COUNT`=未実装の必須Producer/実Consumer/認可/拒否/回復/配布生成経路数。母集団・合格下限を縮めない。`BUILD_GAP=0`でE3閉鎖。外部秘密/署名/Store審査/Owner GOの権限不足と実コード不足は別分類。
R05-04 旧P/B/C/Phase/BUF/QE/FQ/W/Qの既済結果は**元の環境・SHA・証拠classに限定**して再利用。新OS/新Runtime/別候補の保証へ遡及流用しない。

## R06 E4：統合実物QC、E5：対象標準OS QA

| 検査座標 | 主語/確認対象 | E4/E5の作業 |
|---|---|---|
| Q0 候補固定 | 全体Source/Contract/OS/Artifact/18 Blocker | 同一版/ビルド設定/パッケージの証拠マニフェスト固定。E4で一回開始 |
| Q1 全体系 | AI/Tool/A2A/Compose/Edit/Export/GUI | 正常・拒否・結果依存後続・実Producer/Consumer縦断 |
| Q2 故障/復旧 | Broker/Buffer/Process/Network/OneDrive | Crash/Cancel/Deadline/TEMP/TLS/cache/Host/scratch・副作用・根因への帰還。FQ-R2/TEMP/LOOPBACK |
| Q3 安全 | Authority/Permission/Approval/Credential/Audit/Path/Content Exposure | 認証IPC、一回性/期限/Host束縛、秘密/改竄/非冪等Replay/Fail-closed |
| Q4 資源/耐久 | Shell/Flutter/Native/Export/Buffer | 8GB級Windows代表、各OS条件、process leak、性能、binary pruning・実サイズ比較 |
| Q5 OS installed | 通常OS使用者 | Windows VM＋所有実機、Linux native VM/hosted、macOS適法VM/hosted、Android Emulator、iOS Simulator。実Installer→first-run→Tool→Update/Repair/Rollback/Uninstall |
| Q6 証拠集約 | Release Validator/Requirement Ledger | 全ID・Candidate/OS/Artifact・実結果・18 Blocker・旧履歴の非混線・残余照合 |
| Q7 最終判断 | 責任者 | 正式配布資格/署名/監査/Owner GO、Registry/公開状態の独立決裁 |

R06-01 Q1〜Q5の各実測内容をE4実QCとして扱い、その候補と結果に対するE5の**保証判定**を別に行う。対象候補が変われば影響範囲のQC/QAのみ再検査し、影響無しの根拠を無条件破棄しない。
R06-02 OS版/arch/パッケージ/一般スペック/ABI/API/Native Permission/GUI/秘密保護/OS更新/最低資源を**Release候補時の保証母集団**として確定する。測定・公開規格から未確認数値を捏造しない。
R06-03 非Windows物理デバイス試験は不要。仮想環境が存在しない/固有APIを再現不能な対象だけ`QA_NOT_ESTABLISHED`。Windows VM/所有実機の結果を他OSへ流用不可。配布用iOS device版とSimulator実行版のBinaryは異なるArtifactとして関係を監査し、Simulator結果をdevice実動証拠にしない。
R06-04 本番署名・Store資格・Publisher identity・外部本人認証は**QA実環境の代替条件でもBUILD停止条件でもない**。技術実装・仮想証拠の取得を進め、実際の確定操作はOwnerへ切り戻す。

## R07 施工中の限定品質検査と安全即時検査

R07-01 小編集：対象Sourceの意味・実Consumer・依存・安全作用を確認。**毎小編集で全Schema/Conformance/Rust/Flutter/OS/QAを走らせない**。
R07-02 BUILD_BATCH末：影響したType/Compile/Contract/Integration Smoke/必要なnegative・差分検査だけ実行。Contractが変わったなら日本語意味→Schema→必要な正負試験→全実Consumerを同期。
R07-03 Authority/Approval/Permission/Credential/Content Exposure/IPC/Path/Host isolation/Audit/Recovery/process tree/秘密/OS保護が変われば、対象の必要な否定・停止・復旧テストを**その場で**実施。不適合なら危険作用をfail-closedへ隔離し独立線を進める。テスト削除やSkipで品質を演出しない。
R07-04 影響外の全suite・OS installed・正式Q0..Q7の反復はE4/E5まで不要。検証不能は`NOT_RUN`、ソース不具合は`TECHNICAL_FAILURE`、権限不足は`BLOCKED_AUTHORITY`、対象環境不足は`QA_NOT_ESTABLISHED`へ分離。
R07-05 OneDrive同期領域のsourceとCargo/Flutter build/cache/tempは分離。ループバック/TLS/他Host成功・retryを原因修正としない。外部保護やOS権限を迂回してPASSを捏造しない。

## R08 Gitと文書変更の作業閉包（施工許可時のみ）

R08-01 Git正本=`main`・単一writer・原則PRなし・承認範囲でdirect-main・2世代`refs/tags/codex/backup-main`/`refs/tags/codex/backup-main-prev`・remote SHA照合。2タグを同一SHAへ潰さない。未commit差分/他worktree/別writerの所有物を保持。
R08-02 Git閉包の単位=`実Producer→実Consumerが前進した意味ある横断BUILD_BATCH`または正式な一回の統治変更。K01/K02等の台帳行単位でbackup/manifest/Conformance/commit/pushを周回させない。
R08-03 始業時にremote+dirty+他writer+backup確認、変更前に二世代保全、Batch完成時に限定検証・commit・push・remote HEAD/Backup照合。git status/競合/Permissionの問題は破壊的reset/forceで回避しない。
R08-04 Actionsは既存の手動`workflow_dispatch`補助限定、通常push-trigger CI/PR workflow/CI green gateへ工程を変更しない。OS外部ビルドや重い検査の実証classを混同しない。
R08-05 文書正本の更新は採用版/旧版/新番/根拠/適用範囲/Validator/Registry対応を一回の差分で扱う。E0で旧AGENTS/Registryの全行を完全編集してからE1へ進む旧条件を廃止。安全上の衝突は対象を止め、施工継続可能な線を並行進行。正式採用が未指示ならGit変更不可。

## R09 進捗反復の停止条件・制御信号

R09-01 **停滞検出**：施工中に、意味ある連続した作業閉包が`docs/ledger/schema/manifest/test`のみで、production Source/実Consumerの有効前進が0なら`CONSTRUCTION_LIVENESS_FAIL`を発行。進捗を偽って次Kへ進むことは禁止。現在作業を中断し、実コード欠落の最上流からE1へ戻す。危険作用を有効化しない。
R09-02 `K01..K20`・`W00..W08`・`Q0..Q7`・`Phase0..45`の番号は**検査座標**。番号順で前項CLOSEDを必要とする施工順へ変換した時点で`PHASE_ORDER_DRIFT`。担当は製品全線の実Source施工へ戻る。
R09-03 E0同種台帳の反復再生成、証拠なくRegistry数値だけ更新、毎回の全Manifest生成、Doc-onlyコミットを`GOVERNANCE_LOOP`と判定。欠測を残し、全A〜J実装後にE3全数照合する。
R09-04 一線の安全STOP/本人資格/外部Plugin/非Windows QA環境不足は該当線の作用/保証判定のみ保留し、独立線のE1/E2を継続。全体停止が必要なら危険の伝播経路と停止範囲/再開条件を具体化。
R09-05 不具合/反例発見→生成原因（上位仕様/Contract/依存/設計/Consumer/検査器）を監査→必要な上流へ戻す→修正→同条件再観測/副作用検査。旧PASS/FAILを削除・改竄しない。

## R10 状態報告と判定式

`CONSTRUCTION_COMPLETE=(E3 BUILD_COVERED AND mandatory_build_gap=0)`。
`PRODUCT_QA_COMPLETE=(E4 QC必須合格 AND E5宣言OS毎に必要仮想installed実証・Windows完成実機確認成立)`。
`RELEASE_AUTHORIZED=(Owner GO AND 対象Release trackのRegistry18/安全/正式配布/署名・資格の全条件成立)`。

報告Record=`{dateJST,remoteSHA,activeSpec,ownerInstruction,actualSourceDiff,actualProducerConsumer,A..J:CODE_PRESENT/WIRED/GAP,N01..11,buildGapCount,systemShaped,buildCovered,criticalNegative,OS:version/arch/artifact/virtualEvidence,WindowsPhysical,q0..q7,blocker18,releaseReady,ownerGO,rootCause,unverified,backup2,gitCommitPush,nextCrossLineSource}`。

R10-01 正式工程進捗は「A〜Jの実体と消費者の接続、欠落0、OS別installed結果」で評価。Commit数・台帳行数・テスト件数を完成度の分母に使わない。新規%を出すときは全母数・重み・UNKNOWNを別定義しない限り算出しない。
R10-02 確定した製造内容/重要障害/本人判断必要事項のみ人間へ報告。通常の内部再同期/既承認施工の次手承認を繰返さない。
R10-03 本書は正式採用された工程正本であり、**製品施工を実施・完了した証拠ではない**。v4採用済みHEAD・既存作業・歴史的レポートは自動変更されない。別途実製品施工を投入された施工者は現在HEADから進捗/未解決項目を再観測し、初回一巡の実Source施工へ移行する。
