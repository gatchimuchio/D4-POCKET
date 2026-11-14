# D4Pocket 統合製品実装仕様書 v5.0 — 全躯体一巡施工契約

文書ID=`D4-QE-SPEC-20261114-V5`／言語=`日本語基底`／対象=`gatchimuchio/D4-POCKET`／作成=`2026-11-14 JST`。
観測基準=`main 4f1eab7e9520e7a6ee68ceba266433ec5ef175d9`（2026-11-14観測、後続状態を保証しない）。
状態=`v5施工正本として採用・製品施工未着手`。本書の採用は製品実装完了・QA・Release GOの許可ではない。
対応文書=`02_D4Pocket_PLAN_v5_JA.md`、`03_D4Pocket_CODEX_v5_JA.md`。先行採用版=`docs/QE_2026-11-13/` v4（履歴）。採用前main=`eccdf02f5a8a1b44475a470e59e08747f285dee5`。
原要求の意味、既存技術契約・安全境界・既存能力・歴史的証拠は保全。旧文書と本書の差異は設計変更として明示し、歴史の遡及修正を禁ずる。

## S00 主語・目的・手段・責任

| 主語 | 目的／責任 | 非権限 |
|---|---|---|
| 製品 D4Pocket | 複数AI/Agent/Runtime/Tool/Hostの発見・実行・調停・観測・復旧・合成・独立製品化を提供する汎用AI Runtime Operation Shell | 使用者の意思、Agent判断、認可の創出 |
| 製品使用者 | 標準OSで通常導入してサービスを使用し、目的・許可・停止を指定する | 開発者権限の自動取得 |
| 製品内AI/LLM/Agent | 意図→Task→Capability選択→Tool/Runtime作用→結果再消費→次Task／完了／停止 | 自己Permission/Approval/秘密取得 |
| Rust Security Broker | 認証IPC、Capability、Permission、Approval、Credential、Content Exposure、Audit、Recoveryの唯一の作用Authorityを保持する | UI/AI/Adapter/外部資料へのAuthority移譲 |
| 指示者＝ご主人様 | 要求・施工範囲・優先順位・採否を指示する | 個々の既承認手順に対する恒常的再承認義務 |
| 設計者＝チャッピー | 仕様・構造・工程・検査射程の設計責任 | 施工済み・品質成立・採用済みの自己宣言 |
| 施工者＝Codex | 既承認範囲内の実装・結合・検査・是正・証拠・Git保全 | 要求の独断縮小、本人許可、秘密・署名・公開の越権 |
| 責任者＝ご主人様 | 品質・資産保全・正式公開・署名資格・最終GOに関する対象別決裁 | 施工報告への責任移転 |

S00-01 目的=`一般ユーザーが対象OSに適した配布物をダウンロード→OS通常手順でインストール→起動→Setup Doctor/必要な資格登録→基本サービスを使用→AIが実能力により仕事を処理→結果/停止/復旧/更新/撤去へ到達すること`。
S00-02 D4Pocket≠単なるChat UI、特定LLM専用GUI、BLUE-TANUKI/MINIDORAの固有Brain組込。外部Runtimeを独立Adapterとして扱い、交換しても共通意味契約を保持する。
S00-03 手段=`Shell Core + Rust Broker + Native OS Bridge + Flutter Desktop/Mobile + Runtime/MCP/A2A Adapter + Buffer + Compose/Export + Installer + Evidence`。実装言語は責任境界ごとに選択可能、言語の統一は目的でない。
S00-04 工業施工上の製品成立と評価成立を分離する。`CODE_PRESENT`≠`WIRED`≠`LIVE_ACTION_OBSERVED`≠`BUILD_COVERED`≠`QC_PASS`≠`QA_SCOPED`≠`RELEASE_GO`≠`PUBLISHED`。
S00-05 合格の主語は**製品全体の使用可能性**。コミット数・台帳行数・テスト件数・Schema個数・局所CLOSED・ドキュメント整備は代替指標にしない。

## S01 製品使用体験・OS横断契約

S01-01 共通体験=`入手→導入→初回起動→診断→実仕事→状態表示→停止/復旧→更新/修復→アンインストール`。OS固有の導入パッケージ・permission UI・秘密保護の相違はNative Bridge/Adapterへ閉じる。
S01-02 必須OS=`Windows/macOS/Linux native Desktop + Android/iOS Mobile`。Windows-firstは**確認優先順位**であって、非Windows製品ソースの施工開始を待機させる条件でない。
S01-03 Windows配布物は一般ユーザー向け`.exe`等の正規インストーラーを含む。macOSは適法な`.app`/installer、Linuxは指定distro/archの通常package、Androidは正規APK/AAB等、iOSはAppleの正規配布要件を満たす構成。**全OSで同一`.exe`を動かすことは要求しない**。
S01-04 利用者はSource取得、Cargo/Flutter/Python/SDK/CLI導入、開発者Terminal操作、手動依存解決を行わずに基本サービスを利用できる。Portable/debug/test harness/WSL試験用bootstrapを完成installerと呼ばない。
S01-05 最低限のローカルShell起動・診断・管理・既定の利用可能機能は外部Provider資格未登録でも使用可能。外部AI/API/Store等の契約・本人秘密が必要な機能は、製品UIから正規設定・拒否・取消・復旧できる。外部資格が無い状態でも実AI作用が無条件に可能と偽らない。
S01-06 使用状態=`READY/SETUP_REQUIRED/AUTH_REQUIRED/UNSUPPORTED/DEGRADED/SUSPENDED/RECOVERABLE_ERROR`。無言の失敗、成功偽装、無限UI待機、初回一括無制限Permission要求を禁止する。
S01-07 端末/OS別任意機能の非対応は根拠と代替経路を明示。D4の必須共通サービス欠落を「OS差異」に転嫁しない。
S01-08 正規Installer・Store公開・署名・Publisher資格と、実装完了/QA完了は独立条件。未承認の外部公開や資格行使を製品の仮想合格で代用しない。

## S02 安全・権限・異常時不変契約

S02-01 `default-deny`、必要最小権限、認証済IPC、主語/対象/Host/Session/Workspace/Task/作用/期限/一回性に束縛したPermission/Approval。Agent/Tool/Manifest/Flutter/Context/開発用フルアクセスは認可源でない。
S02-02 Content Exposure=`none/hash_only/summary/redacted/full`を個別作用で守る。CredentialはOS native保護領域へ保存し、実値をUI・ログ・Agent・Tool結果へ無条件露出させない。外部Tool応答は未信頼の資料であり命令・承認へ昇格しない。
S02-03 Path/Workspace/Host境界=`正規化+実体束縛+symlink/hardlink/TOCTOU否定`。所有外ファイル、scratch/Temp、Session間状態、Host/Agent間権限、旧Approvalの新Taskへの自動継承を禁止。
S02-04 非冪等replay防止、deadline/cancel/crash/kill時のprocess tree回収、停止/復旧/再開の可逆性、部分失敗時の状態説明、監査先行・追記専用Audit・external tamper evidence・署名identity/provenance・fail-closedを維持する。
S02-05 AI Edit=`提案/差分→独立検査・別実行主体→利用者認可→Broker-controlled apply→実結果監査/rollback`。提案だけでは実適用でない。AI自己承認禁止。
S02-06 Buffer変換は元Taskの目的と効果/副作用/失敗条件の同一性を保持。単純retry・Host変更・検査器の緑化を根因解消と呼ばない。
S02-07 セキュリティ重大不成立は**該当する危険作用のみ**即停止/隔離。無関係な承認済み製品施工は継続。事故・境界不明を隠したまま有効化は禁止。

## S03 製品全線 A〜J：実Producer→実Consumer

| 線 | 生産主体→消費主体／製品能力 | 実製品成立の最低境界 |
|---|---|---|
| A | 使用者目的→AI Runtime/Task/Session→Broker→Tool→結果→同AI次Task | AI発のMCP実Tool要求、fresh認可、結果依存後続Task、Audit/停止/Recovery。Flutterのボタンによる代替やfixture循環では不可 |
| B | Catalog/Adapter/Agent/Host→異種実RuntimeとMCP/A2A→Task/Message/Artifact/Handoff/Compare | 2種類以上の実Runtimeが独立Taskを実行し、分離/拒否/成果転送に到達 |
| C | Flutter/Native→Broker実状態→使用者Agent Center/Workspace/Process/Eval | Inspector/History/Diff/Trace/Metric/Problems/通知/Tray/Search/Command Palette/Stop/Replay/Recoveryが実状態を消費 |
| D | 現実在Capability→依存/互換/許可→Compose→実Tool；AI提案→独立検査→適用 | 実Tool作用とControlled AI Edit/rollback。Manifestのみ・Previewのみは未施工 |
| E | Compose→Module選択/実削除→独立identity/Broker/Runtime→Export App/Installer | 選択Module実消費、不要Moduleのbinary/resource実除外、独立起動/認可/結果。単なるコピー/雛形は不可 |
| F | OS/API/ABI/依存/OneDrive/cache/TLS/process差→Buffer/Broker→元Task | B0..B4能力維持、B5自己適用、B6全結合、監査・安全・意味・副作用検査点 |
| G | Windows Flutter/Native/Broker→installer/Launcher/SetupDoctor/Update→通常使用者 | clean profile installed初回→実仕事→Repair/Rollback/Uninstall。dev portableを代入不可 |
| H | macOS/Linux Native Bridge→Broker/Flutter/Installer→通常使用者 | Keychain/secret/IPC/process、Linux native distro+X11/Wayland宣言、正規Desktop起動/更新/撤去 |
| I | Android/iOS native/Flutter/Device Link→Broker/Agent Task→通常使用者 | Keystore/Keychain、lifecycle/permission、OS正規導入/仕事/停止/復旧/更新。Emulator/Simulator検査対応 |
| J | 全Source/契約/実作用/Artifact/OS→Evidence/Validator/Release Registry→責任者 | 18 Blocker、全要求ID、Q/FQ、同一候補の根拠追跡とGO境界。文書の成立を製品成立に偽装不可 |

S03-01 共通契約の変更は全影響Producer/Consumer/Native/Flutter/Installer/Export/Schema/Testsを同じ意味構造として補正する。局所PASSのための別経路・重複権限系を作らない。
S03-02 全A〜Jは初回製造**同一一巡の施工母集団**。順番はID番号によらず実依存・ファイル競合・安全隔離だけで決める。特定線の完成/QA合格を他線の着工Gateにしない。
S03-03 既存の正当な能力・Code・test・証拠・型を再利用し、改修で性能・機能を黙って落とさない。新しいstub/mockは検査器限定とし、production欠損を隠さない。
S03-04 Mobile Agent Task申請はDevice Linkで既存Brokerへ送る期限付き・件数上限付きの**依頼下書き**であり、受信だけではTask・Permission・Approvalを生成しない。実装上の保持上限は端末ごと16件、全端末合計64件、期限は15分とする。Desktop Agent Centerが依頼本文を表示し、Ownerが現在のRuntime/Workspace/本文を確認した後、Workspace PermissionとTask固有の一回Owner Approvalを既存Broker/native確認経路で別々に発行する。Owner Approvalの実行条件hashは申請IDへ束縛する。Mobileの開始候補表示は状態の投影に限り、開始要求時にBrokerが申請の同一性・期限・現在Permission/Approvalを再評価する。Mobileは対になった自端末の申請だけを状態確認・一回開始・取消要求できる。Mobile向け申請/Task本文投影にAudit識別子・Credential・Permission/Approval metadata・Authorityを含めず、Task本文も返さない。Device Link共通応答envelopeの監査receiptは既存契約どおり返し、Task clientは検証後に画面へ露出しない。結果本文の取得とContent ExposureはDesktopの別個の既存Owner確認経路に限る。

## S04 新製品要求 N01〜N11（全11件、先行施工順ではない）

| ID | 必須実挙動／受入対象 |
|---|---|
| N01 | 一般使用者の目的→AI Task→Broker→実Runtime/Workspace→成果・取消・期限・Crash/Recovery/Audit |
| N02 | 同一AIの実MCP Tool要求→現在Catalog・fresh Permission/Approval→実Tool作用→結果をAI再消費→結果依存後続Task |
| N03 | 共通Adapterから少なくとも2種の実Runtimeへ独立実Task/失敗/拒否/Unsupported対応 |
| N04 | 独立Agent Compare/Handoff/Workspace隔離、A2A Agent Card/Task/Message/Artifact/Stream、Host境界 |
| N05 | 現実在するCapability→依存/互換/許可評価→Compose→実Tool、衝突・不存在・危険条件拒否 |
| N06 | 独立identity/Broker/選択Module付きStandalone Export/installer、実module pruning・計測 |
| N07 | AI Edit proposal/diff→独立審査・認可→controlled apply/rollback、自己承認/権限継承禁止 |
| N08 | OS・依存・Toolchain・OneDrive・Cache・Transport差異の合法Buffer調停、目的同等性・副作用・B5自己適用/B6結合 |
| N09 | Agent Center/Workspace Inspector/History/Diff/Eval/Resource/Trace/Problems/通知/Tray/Stop/Recoveryの実Broker消費 |
| N10 | 五OS標準環境の通常配布物→install→first-run→基本サービス/実仕事→update/repair/rollback/uninstall |
| N11 | 全ID→Source/Contract/実Consumer/実結果/OS/Artifact/安全/Recovery→QC/QA/18 Blocker→Owner決裁 |

S04-01 N01の既存W01 Windows Codex CLI限定CLOSEDを全Runtime・全OSの成立へ拡大しない。N02のW02はinstalled結果依存完走に未成立部分あり。N03二種目Runtime、N06独立Export/pruning、N07 controlled apply、N08 B5/B6、N10五OS installedは現観測で残件。上記は**施工対象**であり、未完成を前提に要求削除しない。

## S05 既存製品能力 K01〜K20（全20件、実体を継承）

K01=Runtime Catalog/Task/Agent/Session/Workspace/State/Launcher/IPC/Authority。
K02=独立Agent Compare/Handoff/同時Task/隔離/選択成果・権限非継承。
K03=Workspace Inspector/Tree/Diff/History/Redo/Branch/Recovery。
K04=PID/CPU/RAM/I/O/Latency/Error/GPU optional/Stop/Restart/Process isolation。
K05=Evaluation Lab/Dataset/Case/Regression/決定論的評価/Runtime比較（LLM Judge単独不可）。
K06=Provider/Model/Runtime Adapter管理/実Runtime選択/失敗/非対応。
K07=Credential Vault/OS native secret/登録・失効・削除/対象束縛。
K08=MCP Server/Transport/Tool/Resource/Prompt/Catalog/実Tool/AI再消費。
K09=Profile/Channel/署名Update/Rollback/Repair（Profile≠Authority）。
K10=通知/Trace/Metric/観測/正式Auditとの区別。
K11=A2A Agent Card/Task/Message/Stream/Artifact/Host identity/Trust/越境拒否。
K12=Adapter Publisher/version/hash/signature/risk/互換/隔離/更新/停止。
K13=Desktop Tray/Quick Stop/Palette/Global Search/Agent Center/Problems/Evidence/Recovery/Settings。
K14=Compose/Preview/AI Edit/独立検査・Controlled Apply。
K15=Export/Module選択・実除外/独立identity/Broker/Installer/実構成消費。
K16=Windows install/first-launch/Setup Doctor/Update/Rollback/Repair/Uninstall/候補同一性。
K17=Linux native/macOS/Android/iOSのNative IPC/秘密/Device Link/GUI/配布。
K18=MINIDORA/BLUE-TANUKI等の外部Runtime契約接続。非公開Brain本体は非対象。
K19=長期運転/性能/故障注入/全数回帰/文書・ブランド/正式出荷。
K20=Buffer/根因母集団/B0〜B6/自己適用/意味保持/副作用。

## S06 歴史的要求・試験・原因母集団：削除・再採番禁止

S06-01 `P0..P13=14`、`C0..C34=35`、`Phase0..45=46`は全件必須。元定義=`docs/総合機能拡張_rev1/`・`docs/D4_POCKET_PHASE_MAPPING.md`・`docs/REV2_PROGRESS.md`・`docs/REV3_PROGRESS.md`・`docs/REV4_ACCEPTANCE_LEDGER.md`・`docs/REV5_PRODUCT_PROGRESS.md`。C/Phaseは全行の原ID・元目的を保持し、現A〜J・N・Kへ対応する。ここで全35/46行を転記しないことは除外を意味しない。
S06-02 `P0:N01/N09/N11;P1:N01/N02;P2/P3:N04;P4:N03;P5:N09/N11;P6:N02/N05;P7:N03/N04;P8:N05/N07;P9/P10:N06;P11:N10;P12:N01/N09/N10;P13:N10`は初期対応。別の原文要件をこの対応で無断消去しない。
S06-03 Buffer原理 `BUF-R01..R09=9`: R01全原因/欠測、R02実調停、R03OS/言語横断、R04目的/効果同等、R05認可/監査/秘密/Recovery、R06伝播・無益反復抑制、R07開発自己適用、R08構想から製品までの品質責任、R09生成原因・設計・評価器への帰還。
S06-04 Buffer試験 `BUF-T01..T24=24`: T01全事象、T02 ABI/依存、T03 API/Schema、T04 OneDrive/Flutter Source、T05 loopback/TLS、T06 OS保護/署名、T07 Path/Host/TOCTOU、T08 Authority/秘密/Audit拒否、T09 Cancel/Crash/Deadline、T10非干渉、T11 cache/lock/corrupt、T12 Tool異常、T13証拠class、T14 Export/Installer hash、T15変換効果/副作用、T16自己適用、T17未知時停止、T18 Q連動、T19目的→実結果、T20反例帰還、T21同条件是正、T22 Retry/Host変更非免責、T23下流失敗転嫁拒否、T24旧証拠/現候補区別。
S06-05 `B0..B6=7`: B0〜B4の限定CLOSED履歴保持、B5 OPEN、B6未成立。開発への実Buffer自己適用はFと全線の初期BUILDに含む。
S06-06 `D4-QE-R01..R13=13`、`D4-LNX-T01..T08=8`、`D4-AUD-F01..F07=7`、`W00..W08=9`、`Q0..Q7=8`、`FQ-R2-A..H`、`FQ-R2-TEMP`、`FQ-MODULE-PRUNING`、`FQ-TEST-LOOPBACK`、`FQ-P11-UNINSTALL-FINALIZER-CLEANUP`、`FQ-INTEGRATED`の原定義・旧証拠・反例を保全。
S06-07 F01=Linux path、F02=2世代Git tag、F03=OS証拠範囲、F04=日本語基底strict、F05=single-writer、F06=正本索引、F07=rev5原本照合履歴。F07当時のSHA照合記録を今の全原本再取得と偽らない。
S06-08 全IDの**存在と要求意味**を施工初期から母集団へ保持する。ただしIDごとのSourceSHA/ArtifactSHA/実Consumer/20超項目の詳細台帳を**E1着工の必要条件にはしない**。実装・全線結合後のE3/E4/E5で全件証拠を完成。意味を確定できない危険な作用のみ局所停止し、独立ラインを続行。

## S07 既存Release Blocker 18件：同ID継承と保証対象変更

| 機械ID | 合格責任／現行設計への扱い |
|---|---|
| `windows_evidence_provenance_isolation` | Windows source/artifact/installed evidence分離 |
| `windows_installer_first_run_smoke` | 真のInstaller/別profile初回起動 |
| `windows_setup_doctor_smoke` | Windows Setup Doctorから利用可能なサービス |
| `windows_broker_installed_smoke` | installed Broker/認可/Audit実動 |
| `audit_anchor_external_tamper_evidence_proof` | Auditアンカー/外部改竄痕跡 |
| `owner_go` | 最終決裁、施工者が自動解放不可 |
| `linux_native_product_support` | native Linux full VM/hosted標準GUI/Package/実仕事。WSLだけでは不可 |
| `rev2_mobile_device_evidence` | 旧物理試験からAndroid Emulator/iOS Simulatorの標準環境installed製品証拠へ移行。旧ID・履歴保全 |
| `rev2_mobile_distribution` | Mobile正規配布/署名/Store条件。物理端末試験と別 |
| `rev2_desktop_launch_regression` | 旧resolved、当時の証拠限定。新候補の自動PASSでない |
| `comprehensive_extension_rev1_completion` | C0..34/Phase0..45の実Consumer全数 |
| `rev2_desktop_product_distribution` | Desktopの正規installed UX/更新/撤去 |
| `windows_distribution_identity` | Publisher/署名/配布identity、資格別判定 |
| `rev2_flutter_broker_channel_boundary` | Flutter→Broker認証IPC |
| `rev2_export_owner_ui_authority_path` | 製品使用者の許可/Export Authority |
| `rev2_module_pruning_binary_and_measurement` | binary/resource実除外/測定 |
| `windows_rust_integration_test_execution_policy` | 旧resolved、当時の証拠限定 |
| `rev2_mobile_flutter_native_device_link_boundary` | Mobile native秘密/IPC/Device Link/認可境界 |

S07-01 観測基準のRegistry=`18件、unresolved16/resolved2、release_ready=false`。再施工時は現HEADから再集計。旧status/証拠/版を改竄・削除しない。
S07-02 新OS検証設計へ移行しても、安全・本人許可・正式配布資格・署名・監査・Owner GOの条件を無効化しない。Registry+Validator+文書Consumerの改訂は採用時の差分として管理し、台帳更新を全躯体BUILDの無期限先行作業にしない。

## S08 OS保証：対象母集団、検証媒体、主張範囲

S08-01 保証の主語=`宣言した通常/一般的OS構成で通常製品を使う利用者`。版/arch/ABI/API、GUI/session、secret/permission、通常security、パッケージ形式、最低資源、更新/撤去経路をRelease候補ごとに宣言。全機種/全メーカー/改造OS/未宣言環境の普遍保証は対象外。
S08-02 検証方式=`公式規格/API/ABI/パッケージ契約との適合→定義済みVM/Emulator/Simulator/hosted環境へ実製品相当Artifactを導入→起動→AI実仕事→認可/Audit/失敗/Recovery→Update/Uninstall→CandidateとEvidenceを対応`。cross compileやfixtureを対象OS実利用の証拠へすり替えない。
S08-03 Windows=`Windows VM + 指示者所有のWindows PCで完成版最終確認`。実機は完成後の確認であり、実機への都度手合わせ調整で仕様を構築するものではない。
S08-04 Linux=`対象distro/archのfull VM/hosted native desktop、X11/Wayland宣言、実secret/IPC/GUI/Package`。WSL/WSLg/Xvfb/headlessは成立範囲を区別し、Linux nativeの全保証に流用不可。
S08-05 macOS=`適法なmacOS VM/hosted native GUI、Keychain/IPC/sandbox/通常Bundle`。物理Mac取得を要求しない。
S08-06 Android=`Android Emulatorで対象API/ABI/Keystore/Permission/lifecycle/installed実仕事`。物理Android取得を要求しない。
S08-07 iOS=`iOS Simulatorで実行可能対象・Keychain/native permission/Task/installed UX`。物理iPhone取得を要求しない。Simulator実行Binaryと配布用device Binaryは**異なるArtifactSHA/ABI**を保持し、同一Source/Contract/Build設定からの関係・署名/Entitlementを監査する。Simulatorで再現不能なdevice固有挙動は保証成立と宣言しない。
S08-08 物理端末を全OS分揃えることはBUILD/QC/QA/Releaseの必須Gateでない。必要な仮想環境/資格/OS固有APIを取得できない場合、そのOSだけ`QA_NOT_ESTABLISHED`または`BLOCKED_AUTHORITY`とし、実装済みでない事実や他OSの結果を偽装しない。
S08-09 8GB級Windows機を代表的な設計環境に含める。OS別最小スペックの未測定数値は確定値にしない。特定versionの架空サポート表を作らず、Release候補時に証拠で固定。
S08-10 QA証拠class=`SOURCE/CONTRACT/FIXTURE/EMULATED_PRODUCT/VM_INSTALLED_PRODUCT/HOSTED_OS_PRODUCT/WINDOWS_PHYSICAL_CONFIRM/EXTERNAL_SIGNING/NOT_RUN`。各classの射程を守り、OS・SourceSHA・BuildConfig・ArtifactSHA・実Task・実結果・監査/失敗・Recoveryを結び付ける。

## S09 完成・品質・公開の独立した成立条件

`PRODUCT_STRUCTURE_PRESENT`: 全A〜Jに実製品のSource・実Consumer・OS配布出力生成経路が存在。空の型/Mock/Schema/Manifestだけでは偽。
`SYSTEM_SHAPED`: 代表一仕事で使用者入力→AI/Runtime→Broker→実Tool→実結果→AI次作用/表示→Audit/Recoveryの縦断を観測。全要求完成ではない。
`BUILD_COVERED`: N11件+K20群+P14+C35+Phase46+BUF-R9/T24+B7+QE13/LNX8/AUD7+FQ全件+W9+Q8+5OS+18Blockerに、該当する必須**製品実Source/Producer/Consumer/拒否・復旧/Package経路**を全数対応し`required BUILD_GAP=0`。Q合格や資格発行はこの条件に含めない。
`QC_PASS`: 同一結合候補で正常・境界・異常・副作用・セキュリティ・資源・耐久・更新撤去を観測し、必要な根因是正と影響回帰を完了。
`QA_SCOPED`: 候補版・OS/arch・Artifact・標準検証環境・保証する条件を固定して適合確認。未観測/失敗は`QA_NOT_ESTABLISHED`。宣言外は`OUT_OF_SCOPE`。
`RELEASE_GO`: Release Registry18、対象OS別配布資格、署名/監査、Owner正式判断が満たされ、GOを明示。資格・公開・署名・物理試験は混同しない。
`PUBLISHED`: 正式提供先で配布が実在する別事実。

S09-01 完成判定を日付・コミット数・テスト総数・ドキュメント差分で代用しない。E1で全線Sourceを前進させることを、E3詳細台帳の前提とする。個別安全変更の即時否定検査は製造中も必須。
S09-02 記録形式=`{要求ID,元原文/出典,対象,目的,Producer,Consumer,契約,SourceSHA,OS/Arch,ArtifactSHA,実入力/作用/結果,同AI次作用,Permission,Approval,Audit,Recovery,検査条件/command/exit,class,未観測/残余,旧証拠Ref}`。**詳細記録の全数完成はE3〜E5の検査対象でありE0の着工Gateではない**。

## S10 出典・適用範囲・v4からの修正

S10-01 元要求と原典=`指示者原文`、`規定/00_日本語基底規定.md`、`規定/正本索引.json`、`AGENTS.md`、`docs/総合機能拡張_rev1/`、`docs/D4_POCKET_PHASE_MAPPING.md`、`docs/REV2_PROGRESS.md`、`docs/REV3_PROGRESS.md`、`docs/REV4_ACCEPTANCE_LEDGER.md`、`docs/REV5_PRODUCT_PROGRESS.md`、`docs/PRODUCT_COMPLETION_PLAN.md`、`docs/QE_2026-10-22/`、`docs/QE_2026-11-05/`、`docs/QE_2026-11-08/`、`docs/QE_2026-11-09/`、`docs/QE_2026-11-13/`、`docs/緩衝基盤_rev5補遺_QC/`、`docs/FINAL_QA_QUEUE.md`、`docs/specs/`、`specs/`、`packages/`、`native/`、`apps/`、`installer/`、`tooling/`、`.github/workflows/`、`release_blockers.registry.json`、`README/QUICKSTART/CLAIM/COMPATIBILITY_MATRIX/MOBILE_STATUS/INSTALLER_STATUS/RELEASE_CHECKLIST/SECURITY`。不存在・取得不能はそれぞれ記録し想像補完しない。
S10-02 v4設計欠陥=`E0の全要求21項目台帳・SourceSHA・実Consumer/独立証拠の事前確定をE1開始Gateにしたこと`。観測=`v4受領後85コミットで台帳中心、A〜J製品Source差分0`（基準SHA比較結果）。v5是正=`全要求の存在と意味・共通安全Contractのみ短く同期し、既存K台帳を再製造せず、直ちに全A〜J Source一巡→実接続→E3全数証拠→QC/QA`。
S10-03 v5は旧「全数監査」要求を削らない。**監査の対象・密度は保持、詳細監査の時点を製造後へ配置する**。危険を前倒し検査しないという意味ではない。
S10-04 AGENTSの§3/§4/§25にある小単位施工・統治変更・再読手順の解釈と、現行正本索引・現行検査ツールの参照先・製品公開案内はv5採用に伴い差分同期した。Release Registryの18件・status・判定条件は未変更。将来の実Source施工で出会う旧Consumerの残差はその時点で差分管理し、AGENTS/Validator/Registry全文整備を無期限資料作業へ展開して製品BUILDを凍結しない。
S10-05 本v5の正本採用と製品Source施工・実証・Release GOは別状態。施工者がv5を正式に製品施工指示として受領した後は**最新HEADで現物再観測し**、初期値・旧PASS・旧FAIL・他writer差分・Owner未決裁を別状態で保持する。正式指示なしの製品Source書込み・公開は行わない。
