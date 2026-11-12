# D4Pocket 統合製品実装仕様書 v4.0

文書ID=D4-QE-SPEC-20261113-V4
対象=gatchimuchio/D4-POCKET
作成=2026-11-13 JST
観測基準=main d503a4ef6daac8942abd1cb011442ce28c78ec55
文書状態=設計改訂成果／リポジトリ未採用／製品実装・QA・出荷の成立を主張しない
上位入力=指示者の2026-11-13までの原文要求・安全/認可/証拠不変契約
同版参照=02_D4Pocket_統合開発工程表_v4.md／03_D4Pocket_Codex施工指示書_v4.md
適用=旧資料の要求意味・検証履歴を保全した全製品仕様。施工・正本切替は指示者の別途受領時に行う。

## S00 対象・主語・目的・成果条件

S00-01 対象=D4Pocket。複数のAI/Agent/Runtime/Tool/Hostを発見・接続・実行・監督・観測・復旧・合成・独立製品化する、LLMが第一級実行・拡張主体の汎用Runtime Operation Shell。単なるチャットUI、特定Runtime専用GUI、BLUE-TANUKIの内蔵版ではない。
S00-02 使用者=人間。目的・範囲・条件・製品Permission/Approval・取消を与える。
S00-03 実仕事主体=AI/LLM/Agent。使用者の目的→能力選択→Broker経由の実作用→実結果消費→追加Task/完了/停止を決める。
S00-04 製品=D4Pocket。AIの実行身体、接続・調停・観測・構成・責任境界を供給する。D4自身は利用者意思・AI判断・認可権限を生成しない。
S00-05 設計者=チャッピー。原文要求の製品への射影・仕様/境界/成立条件/工程の設計責任を持ち、施工や出荷をしたと偽らない。
S00-06 指示者=ご主人様。要求/施工範囲/優先順位/安全条件の決定主体。既委任施工の毎手順承認者ではない。
S00-07 施工者=Codex。実装・結合・局所安全検査・全体QC・対象別QA用証拠・Git保全を遂行する。製品内の被試験Codex CLIとは別主体。
S00-08 責任者=ご主人様。要求採否・資産保全・品質・正式公開・鍵/署名・出荷GOを対象別に決裁する。同一人物が役割を兼ねても許可境界を合算しない。
S00-09 Authority=Rust Security Broker。Credential/Capability/Permission/Approval/Content Exposure/Audit/Recoveryの唯一の製品実行権限境界。UI/AI/Adapter/Tool結果/開発用フルアクセスは権限源でない。
S00-10 目的成立=使用者が対象OSの一般的・標準的環境で配布物を入手→OSの通常手順でインストール→アプリを起動→利用可能なD4サービスへ到達→AIが実能力による一般の仕事を一巡し、結果に基づいて次行為/成果/停止/復旧へ至る。CLI・SDK・ソース取得・依存手動導入・開発者設定は利用者に要求しない。
S00-11 製品の完成と検査の完成を分ける。仕様完全性→全体躯体のBUILD_COVERED→結合した実物のQC→定義した標準環境に対するQA→責任者の出荷判断。コード量、commit数、局所テスト件数、個別WゲートのCLOSEDは製品完成を意味しない。

## S01 製品利用体験契約［今回の上位確定変更］

S01-01 共通体験=入手→インストール→初回起動→正常な初回設定/Setup Doctor→既定の利用可能サービス→通常の仕事→履歴/停止/復旧→更新/撤去。Windows・macOS・Linux・Android・iOS間で意味と安全契約を統一し、OSに合った導入UI/許可UI/配布形式を使用。
S01-02 Windows=対象CPU/OS用の一般的インストーラー。実行可能配布物（例: .exe形式のインストーラー）から依存物を含めて導入し、通常のスタートメニュー等から利用可能。Windows Developer portable版を正式インストール完了と混同しない。
S01-03 macOS=一般的なmacOS配布形式と通常起動・更新・撤去。Linux=指定対象distro/arch向け一般的なpackage/installer。Android/iOS=OSが許す通常の配布/導入経路。全OSで同一.exeを動かす要求ではない。
S01-04 導入後の実利用=ユーザーが開発用command、Cargo/Flutter/Python/SDK、環境変数、manual dependency、Source Buildを実行しなくても、共通基本サービスが利用可能でなければ不適合。必要なRuntime/Bridge/Backend/資産の不足は製品側の配布・依存解決・導入・案内・失敗停止に帰属する。
S01-05 外部Providerの契約/本人資格/API鍵等は、その外部サービス固有の使用条件として製品内の案内・認可・安全なCredential登録に閉じ込める。製品の基本起動・内部機能・Setup Doctorを外部認証で全面阻止しない。外部資格が必要な作用を「インストールのみで無条件に利用可能」と虚偽表示しない。一般AI実仕事に必要な導入可能Runtime経路を製品として用意し、外部資格/追加導入が必要なら同一GUI内で完結可能な導線を構築する。
S01-06 追加のOS固有Permissionは、そのOSの正規UIとBroker認可で要求・拒否・取消を処理する。初回起動時の無制限許可要求は禁止。既許可内のAI内部手順への無意味な逐次確認は禁止。
S01-07 利用者へ表示する状態=READY／SETUP_REQUIRED／AUTH_REQUIRED／UNSUPPORTED／DEGRADED／SUSPENDED／RECOVERABLE_ERROR。無言の機能欠落・成功偽装・画面の無限待機は禁止。
S01-08 同一の製品核=LLM-firstの契約/Task/権限/監査/復旧/利用面。OS特有のTray、Accessibility、Keychain、Keystore、Device Link、Package等の差異はAdapter/Bridgeで吸収。OS固有任意機能の非対応は条件と理由を明示し、必須の共通サービス欠落を「仕様差」の名で隠さない。

## S02 OS保証と検査媒体の分離［今回の上位確定変更］

S02-01 保証対象=責任者が発売対象として宣言した一般的・標準的な各OS、CPU arch、OS major/minor範囲、配布形式、基本権限/GUI/秘密保護構成。全メーカー・全機種・全ビルド・独自改造・特殊周辺機器・未宣言構成の網羅保証はしない。
S02-02 保証の手続=規格/OS公式契約・実API/ABI・定義済み環境の適合条件を固定→各OS相当のVM/エミュレータ/シミュレータ/hosted検証環境でインストール可能な実製品相当ビルドを導入→起動/仕事/更新/撤去まで実行→配布用候補の生成・署名検証とSource由来を別途照合→証拠化。単なるcross compileやWindows上での他OS用fixture成功は対象OS上での実行検証ではない。
S02-03 実機方針=OSごとの物理端末実機検証は品質・完成・Releaseの必須条件としない。Windowsは指示者所有のWindows PCで完成品の確認を実施する。他OSは定義した標準環境の仮想検証で保証を成立させる。Windows物理確認を非Windowsの適合証拠へ流用しない。
S02-04 Windows物理確認は、設計仕様・仮想上の適合証拠を前提とする最終確認であり、実機だけの都度調整で品質を成立させる方法を採用しない。不適合観測時は原理/仕様/依存/共通コードへ戻って是正し、仮想基線も再検査する。
S02-05 検証環境例: Windows=Windows VM+所有実機、Linux=実ディストリビューションのフルVM/hosted desktop（X11/Wayland条件を宣言）、macOS=適法なmacOS VM/hosted runnerのGUI・native API、Android=Android Emulator、iOS=iOS Simulator。Apple等のプラットフォーム利用条件を守る。実行可能な相当環境が得られなければその対象はQA_NOT_ESTABLISHEDとし、実機を要求して代替しない。
S02-06 Xvfb/headless/WSL/WSLgは対象OSの標準desktop GUI・installer条件との意味同一性が成立する部分にのみ証拠利用可。Linux native完成を単独のWSL成功により断定不可。Wayland/X11は宣言する範囲を検査。
S02-07 OSマトリクスは設計時に候補を一覧化し、Q0で具体的なrelease時点の標準版/arch/パッケージ/依存/最低HWを固定する。未確認の版数や未実行環境を推測でSUPPORTEDと書かない。8GB級Windows端末を設計の代表下限に含め、各OSの最小資源値は構成ごとに実測で確定。
S02-08 OS不変テスト=使用者目的→AI Task→実Tool/Agent作用→AI結果消費→監査→停止/復旧＋初回起動/Setup Doctor/資格情報/Update/Rollback/Repair/Uninstall。OS固有秘密・認可・プロセス・ファイル権限・ABI・GUIの実条件を追加。仮想でもfixtureを実Tool成功と偽らない。
S02-09 証拠class=SOURCE/CONTRACT/FIXTURE/EMULATED_PRODUCT/VM_INSTALLED_PRODUCT/HOSTED_OS_PRODUCT/WINDOWS_PHYSICAL_CONFIRM/EXTERNAL_SIGNING/NOT_RUN。Q5・Q6が要求→ソースSHA→成果物SHA→OS環境→操作→実結果→監査を連結し、同じRelease候補へ帰属させる。
S02-10 旧「macOS実Mac必須」「Android/iOS実物理端末必須」「Linuxの実物理desktop必須」を完成条件として再採用しない。これらの旧記録は履歴・当時の保証境界として保存する。変更は製品の安全要求の削除でなく、指示者が保証対象と検証手段を再定義した設計変更。物理専用blockerを履歴破壊やsilent resolvedで消さず、同じ目的の標準環境仮想適合証拠へ置換する。正式Registry/AGENTS/Validator/Release Checklistは採用時に同期・差分監査が必要。
S02-11 本番Publisher、ストア申請、署名資格、本人鍵・対外契約・最終出荷GOは物理端末検証と別の権限/配布条件。Codexは技術的準備を先行できるが、本人固有意思/資格の発行を偽造不可。技術適合・正式配布・公開済みを別状態で報告。
S02-12 仮想機と配布対象のABI/署名/成果物が異なる場合、同一Binary SHAを要求・偽造しない。特にiOS Simulator実行Appと実機配布IPAは別ターゲットの成果物。同一SourceSHA/Contract/Release設定からSimulator実行ArtifactSHAと配布ArtifactSHAを別々に生成・対応付け、ビルド差・署名/Entitlement/Runtime制約を静的/仮想で監査する。Simulatorで観測していない実端末固有性は保証しない。Appleの実機Release試験推奨を満たしたとは主張しないが、それを本計画の物理端末必須Gateへ変換もしない。

## S03 技術所有境界と不変集合

| ID | 生産/責任主体 | 実consumer／維持契約 | 禁止越境 |
|---|---|---|---|
| ARC-A | AI Runtime/Agent | Task→Capability選択→Tool要求→結果依存後続Task | 自己Permission/Approval生成、Tool結果からAuthority獲得 |
| ARC-B | Shell Core | Runtime/Task/Session/Workspace/Host/Catalog/State/Config共通意味→全Adapter/Broker/UI | 特定AI/Flutter固有関数をCoreの普遍要求へ昇格 |
| ARC-C | Adapter/Provider | Codex CLI・異種Runtime・MINIDORA・BLUE-TANUKI等の外部実体→共通Task/Result | Provider固有metadataをTrust・秘密・権限へ昇格 |
| ARC-D | MCP/A2A/Host/Capability | Tool/Agent Card/Task/Message/Artifact/Stream→Broker→実作用 | Manifest宣言だけで実能力成立、別Host/Agent権限横流し |
| ARC-E | Rust Security Broker | 認証IPC→Capability/Permission/Approval/Credential→作用→Audit/Recovery | UI/AIによる許可、Audit失敗時success、秘密実値露出 |
| ARC-F | Flutter Desktop/Mobile | 製品使用者の目的・許可・履歴・状態・通知・差分・停止/復旧表示 | UIをAIの内部意思/権限/無期限Controllerへする |
| ARC-G | Native OS Bridge | Launcher/IPC/Keychain/Keystore/process/permission/installer→Broker/Flutter | OS保護の回避、仮想と実製品証拠の混線 |
| ARC-H | Buffer/QE | 差異の観測→意味保持調停→元Tool作用/副作用比較→根因帰還 | 失敗隠蔽、単純retryを根因解消と称する、別権限系 |
| ARC-I | Compose/Export | 既存実能力/依存構成→独立identity/選択module/installer→利用者 | Manifestのみ・Pruning見せかけ・開発buildを製品配布扱い |
| ARC-J | Evidence/Release | 実装・実作用・OS・配布候補・監査→QC/QA/責任者GO | 旧PASSの現候補転記、test件数でRelease自動認可 |

不変契約=default-deny、最小権限、Permission/Approvalの対象・期限・一回性、Content Exposure{none,hash_only,summary,redacted,full}、Credentialのnative保護/非投影、認証IPC、Path正規化・実体束縛・symlink/hardlink/TOCTOU、Task/Session/Host/Workspace隔離、非冪等replay防止、タイムアウト/取消/kill時process tree回収、scratchと所有外資産境界、Audit append-only/finality/外部改竄痕跡、Recovery/停止/再開、署名/identity/provenance、fail-closed。
旧Approval/Permissionの新Session/別Host/後続Toolへの無条件継承禁止。Tool応答は未信頼資料。実Taskと結果に新識別子/新認可/監査が必要。AI Editはproposal→独立施工→検査→制御適用、自己承認禁止。Runtime内の非公開HDS/MINIDORA/BLUE固有Brain・RAG/Memory/検索/InterpreterをD4 Coreへ無断移植不可。

## S04 新要求全件：D4-N01〜N11［施工順ではない］

| ID | producer→実consumer | 成立条件 |
|---|---|---|
| N01 実仕事 | 使用者目的→AI Task→Broker→Runtime/Workspace→結果/Audit/停止/回復 | 一般利用入口の実Task、無権限拒否・Cancel/Deadline/Crash。旧W01 Windows Codex CLI限定CLOSEDは保存、新候補全体合格へ昇格不可 |
| N02 AI→Tool→AI | AI自発MCP Tool要求→現在Catalog/認可→実MCP作用→結果→同AIの結果依存後続Task | FlutterボタンがAI連続実行を肩代わりしない。source Task/hash束縛、結果の未信頼、各後続要求の新規認可、Audit/停止 |
| N03 異種Runtime | Runtime Catalog/Provider/Adapter→統一Task/Result/Capability→独立実Runtime | 少なくとも2種類の実Runtimeの実Task・失敗・権限証拠。unsupported明示。D4内蔵特定LLM不要 |
| N04 Compare/Handoff/A2A | Agent/Session/Workspace独立実行→diff/test/result選択；A2A Card/Task/Message/Artifact/Stream→転送/消費 | 実外部Agent作用、比較隔離、明示Artifactの限定転送、権限非継承、外部metadata未信頼 |
| N05 Capability/Compose | 現実在Catalog→依存/互換/権限検査→Compose→元Tool実作用 | 不在/衝突/危険条件拒否。manifest_only≠wired≠実使用 |
| N06 Standalone Export/Module | Compose構成→独立identity+Broker+Runtime+manifest+選択module→独立App/installer | 生成Appが構成実消費・必要機能動作・不要機能除外。binary/resource減は実測、署名は別条件 |
| N07 AI Edit | AI提案/差分→別実行主体の審査/検査Task→承認→controlled apply/rollback | 提案≠適用。別Workspace/Session/権限。入力申告未信頼、適用結果はBroker照合 |
| N08 緩衝/調停 | OS/API/ABI/toolchain/cache/process/network/OneDrive差異→意味保持調停→元Task成立 | B0〜B4継承、B5自己適用・B6結合。副作用/負例/根因追跡、権限非緩和 |
| N09 利用/観測/復旧 | Agent Center/Workspace/History/Diff/Eval/Resource/Trace/Problems/Audit/Recovery→同一実Task | 実Broker状態消費、pause/stop/restart/replay/分岐/通知/検索/Tray、UI無応答是正 |
| N10 全OS製品化 | 五OS適合Layer→配布物→install→first-run→仕事→update/repair/rollback/uninstall | S01・S02の体験/標準環境仮想検証。Windows所有実機確認、他OS物理端末不要。OS別独立技術証拠 |
| N11 Evidence/QC/QA/Release | 全N/P/C/Phase/BUF/B/FQ/OS→Source/Contract/実consumer/検証→Gate/Blocker/決裁 | 全数照合、候補同一性、結果区分、18 Blocker、Q0〜Q7、正式GOとrelease_ready分離 |

## S05 既存全能力保持：K01〜K20

K01=Runtime Catalog/Agent/Task/Session/Workspace/永続state/Launcher/IPC/Broker/Capability。
K02=独立Agent Compare/Handoff、Workspace分離、同時Task、選択結果適用/Authority非継承。
K03=Workspace Inspector/file tree/変更・差分・履歴・再実行・分岐・復旧候補。
K04=PID/CPU/RAM/I/O/Latency/error/optional GPU/開始・停止・再起動・隔離。
K05=Evaluation Lab/Dataset/Case/回帰評価/決定論的評価器/Runtime横断比較。LLM Judge単独保証不可。
K06=Provider/Model/Runtime Adapter管理、実Runtime選択・失敗・unsupported。
K07=Credential Vault/秘密非露出/OS保護/登録・更新・失効・削除/対象束縛。
K08=MCP Server/Transport/Tool/Resource/Prompt/接続Center/隔離/実Tool/AI消費。
K09=Profile/Channel/署名Update/Rollback/Repair、Profileは権限源でない。
K10=通知/Trace/Metric/観測と正式Auditの非同一性。
K11=A2A Agent Card/Task/Message/Stream/Artifact/Host/identity/Trust/Host越境拒否。
K12=Adapter Publisher/version/hash/signature/risk/互換/隔離/更新/停止。
K13=Desktop Tray/quick stop/Command Palette/Global Search/Agent Center/Problems/Evidence/Recovery/Settings。
K14=Compose/Preview/AI Edit/差分/独立審査・controlled apply。
K15=Export/Module選択・除外/独立identity/Broker/Installer/構成消費。
K16=Windows install/first launch/Setup Doctor/update/rollback/repair/uninstall、候補同一性。
K17=Linux native/macOS/Android/iOS、Native IPC/保護/Device Link/GUI/OS固有導入。
K18=MINIDORA等を外部Runtime/契約で接続。非公開内部実装は非対象。
K19=長期運転/性能/故障注入/全数回帰/文書・ブランド/正式出荷。
K20=緩衝基盤/根因母集団/B0〜B6/自己適用/副作用/意味保持。

旧ID保持=rev5 P0..P13(14件)；総合機能拡張rev1 C0..C34(35件)；D4 Phase0..45(46件)；K01..K20(20群)。詳細の全46行/35行逆引きは docs/D4_POCKET_PHASE_MAPPING.md を逐条消費、原IDは削除/再採番禁止。旧局所CLOSEDを引き継ぎ、現在Nの未配線をCLOSEDにしない。

P別対応=P0:N01/N09/N11, P1:N01/N02, P2/P3:N04, P4:N03, P5:N09/N11, P6:N02/N05, P7:N03/N04, P8:N05/N07, P9/P10:N06, P11:N10, P12:N01/N09/N10, P13:N10。旧Pの実績と新Nの完成は別。

## S06 緩衝基盤・監査・試験IDの全数保持

BUF-R01=原因母集団/欠測全数；R02=緩衝の実作用；R03=製品・言語・OSに閉じない；R04=元目的/結果の意味同等；R05=Brokerの認可/秘密/Audit/回復保持；R06=障害伝播・無益反復抑制；R07=D4開発へ自己適用；R08=構想から配布まで品質責任；R09=生成原因・設計・評価器まで帰還。
BUF-T01=全事象照合；T02=ABI/依存矛盾；T03=API/Schema往復；T04=OneDrive/Flutter source同一性；T05=loopback/TLS断続；T06=OS保護/署名非回避；T07=Workspace/Host/path/TOCTOU；T08=認可/監査/秘密の拒否；T09=cancel/crash/deadline後回収；T10=Task/Host間非干渉；T11=cache/lock/partial/corrupt回復；T12=Tool異常と意味保持；T13=証拠class区分；T14=Export/Install/Update hash；T15=調停前後効果/副作用；T16=自己適用；T17=未知/非対応時停止；T18=Q連動；T19=目的→実結果追跡；T20=反例で因果更新；T21=是正・同条件再観測；T22=retry/Host変更を根因解消扱い禁止；T23=下流失敗移転；T24=旧証拠/現候補の版別帰属。
B0=原因台帳・欠測限定CLOSED、B1=設計限定CLOSED、B2=作用限定CLOSED、B3=153原因分類限定CLOSED、B4=限定接続CLOSED、B5=OPEN、B6=未成立。B5/B6も一括BUILD初期から実装し、正式結合QCの実測は後段。
QE方法論保持=D4-QE-R01..R13(13件)、監査所見D4-AUD-F01..F07(7件)、Linux旧試験D4-LNX-T01..T08(8件)。原定義は docs/QE_2026-10-22/ に固定、ただしOS物理条件はS02で設計変更。F01=Linux path、F02=2世代tag、F03=OS証拠表現、F04=日本語監査、F05=single writer、F06=正本索引、F07=rev5原本同一性。過去の是正済み記録を改竄しない。
FQ保持=FQ-R2-A/B/C/D/E/F/G/H、FQ-R2-TEMP、FQ-MODULE-PRUNING、FQ-TEST-LOOPBACK、FQ-P11-UNINSTALL-FINALIZER-CLEANUP、FQ-INTEGRATED。Q0..Q7正式試験IDは保持。旧Q0/Q1のPASSは旧候補のみ。

## S07 既存Release Blockerの全件維持と新保証規則への移行

現観測=release_blockers.registry.json 18件、unresolved16/resolved2、release_ready=false。以下は既存機械ID、改名・消去・合格自称は禁止。

| ID（既存名） | 移行分類 | 意味 |
|---|---|---|
| windows_evidence_provenance_isolation | Windows必須 | source/artifact/installed evidence identity |
| windows_installer_first_run_smoke | Windows必須 | 別profile、初回起動・導入実物 |
| windows_setup_doctor_smoke | Windows必須 | 設置診断と利用可能性 |
| windows_broker_installed_smoke | Windows必須 | Broker/認可/Audit実体 |
| audit_anchor_external_tamper_evidence_proof | 安全必須 | 監査アンカー/改竄痕跡 |
| owner_go | 責任者決裁 | 技術試験外の公開GO |
| linux_native_product_support | 対象標準Linux必須 | native VM/hosted仮想条件で導入/GUI/Tool実動。WSLだけで完了不可 |
| rev2_mobile_device_evidence | 保証基準変更の要移行 | 物理端末必須→Android Emulator/iOS Simulator等での標準条件適合＋identity/evidence。履歴を保持し判定実装も改修 |
| rev2_mobile_distribution | 配布条件 | ストア/署名等、物理端末試験とは独立 |
| rev2_desktop_launch_regression | 旧resolved | 別版証拠を保持 |
| comprehensive_extension_rev1_completion | 全能力必須 | C0..C34/Phase0..45の全consumer |
| rev2_desktop_product_distribution | 製品必須 | installed UX/更新/撤去 |
| windows_distribution_identity | 配布資格 | Publisher/署名identity |
| rev2_flutter_broker_channel_boundary | 安全必須 | Flutter→Broker認証IPC |
| rev2_export_owner_ui_authority_path | 安全必須 | 製品使用者の許可経路 |
| rev2_module_pruning_binary_and_measurement | 製品必須 | 実binary除外/実測 |
| windows_rust_integration_test_execution_policy | 旧resolved | 当時の検査成果を保持 |
| rev2_mobile_flutter_native_device_link_boundary | 安全必須 | Mobile Device Link/native秘密 |

移行操作=旧ID/旧status/旧根拠/旧対象→新S02保証対象/代替の仮想実行条件/試験方法/期待結果/残余→RegistryとValidator/Docs全consumerの差分→形式整合検査。移行未完了はOPEN/QA_NOT_ESTABLISHED。責任者GOなしにrelease_ready=true不可。物理試験要求が残る別箇所（規定/正本索引、AGENTS、COMPATIBILITY_MATRIX、MOBILE_STATUS、RELEASE_CHECKLIST、FINAL_QA_QUEUE、OS-specific validator、README/CLAIM）も同時是正し、単なるフラグfalse化で完了しない。

## S08 BUILD・QC・QA・Releaseの合否意味

QE_DESIGN=全要求→Producer→Consumer→Contract→OS/利用条件→安全/失敗→証拠型/試験を固定。
QE_BUILD=全構造を最初の1巡で施工。A〜Jは番号順で直列化しない。ソース/Schema/fixture/OS包装/Installer/Updateを分断せず全consumerを接続。
SYSTEM_SHAPED=代表一仕事の入口→実Broker→実作用→AI再消費→Audit/回復が通る。ただし後続線の着手Gateでない。
BUILD_COVERED=全N/旧P/C/Phase/K/BUF/B/QE/FQ/OS必須対象の実producer→consumer配線が存在し、BUILD_GAP/未実装必須consumer=0。安全負例はBUILD時に即行う。これはQC合格ではない。
QE_QC=全体構築後、Q0で同一Release候補を固定して正常/否定/失敗/故障/耐久/資源/OS VM/Install/Update/Recoveryを集中検査。根因に応じ設計まで帰還、同条件・副作用再検証。必要な影響範囲だけ再試験する。
QE_QA=Q0固定した候補に対しQ1横断→Q2故障/復旧→Q3権限/秘密→Q4耐久・性能→Q5 OS標準環境の導入/利用/撤去＋Windows所有実機確認→Q6証拠集約→Q7責任者判断。Q7は出荷審査工程で行い、Q0..Q7を小機能単位ごとに開始しない。候補変化時は影響あるQC/QA証拠を再取得。
QE_RELEASE_REVIEW=各Release trackの対象OS/機能/配布資格/合法性/署名/監査/未解決blocker/責任者GOで判定。Windows trackと全五OS trackの完成・出荷を混同しない。Release Registry原本改訂前はfalse維持。
QA評価=QA_SCOPED［対象/版/環境/条件が一致］／QA_NOT_ESTABLISHED［要証拠なし/環境なし/失敗］／OUT_OF_SCOPE［宣言保証母集団外］。Fixture/Build/VM/実機/Store配布を区別。虚偽の保証・資料からの推定完成禁止。

## S09 証拠の必須形と基準時点

記録単位={要求ID,旧ID,主語,目的,入力,作用,出力,Producer,Consumer,条件,SourceSHA,ContractVersion,ArtifactSHA,OS/Arch/Build,VM/Simulator/Host,Runtime/Adapter,Permission/Approval,ToolRequest,実結果,AI次作用,Audit,Recovery,TestCommand,Exit,Class,残余,HistoryRef}。
本書作成時点の実状態=HEAD d503a4ef6daac8942abd1cb011442ce28c78ec55、A〜J全線OPEN、N01〜N11統合受入OPEN、W00=SCOPED_PASS、W01=Windows normal Release/Codex CLI限定CLOSED、W02=OPEN、W03..W08未完了、B5=OPEN、B6未成立、正式Q0..Q7=DEFERRED、18 Blocker中16未解決、release_ready=false。2026-11-12進捗ではAgent/MCP/A2A/Export/AI Edit等の局所FIxture/VM証拠は増えたが全体保証未成立。この固定値は後日のHEADを表さない。
W02の既知問題=Codex→MCP実Tool→AI結果依存後続Taskの通常installed完走未証明、native Owner表示/期待文言差、UI停止・sporadic transport failureの根因未確定。W02の再試験をB〜Jの全体系施工停止要因にしない。
過去の実績=W01限定CLOSED、P2/P3等の歴史的局所CLOSED、B0..B4限定CLOSED、F07原本SHA照合VERIFIED、各種Schema/Conformance/Flutter/Rust局所PASS。現在候補や別OSへの保証継承はしない。

## S10 正本・出典・非後退条件

規律=AGENTS.md、規定/00_日本語基底規定.md、規定/正本索引.json。
旧機能=docs/総合機能拡張_rev1/実装仕様書.md・工程表.md・実装指示書.md・進捗.md、docs/D4_POCKET_PHASE_MAPPING.md、docs/REV2_PROGRESS.md・REV3_PROGRESS.md・REV4_ACCEPTANCE_LEDGER.md・REV5_PRODUCT_PROGRESS.md、docs/PRODUCT_COMPLETION_PLAN.md。
rev5原本=原文三点は現Repo未収録。2026-10-30版 docs/QE_2026-10-22/F07_rev5原本要求照合_20261030.md は旧原文の3 SHA・当時の差分未照合0を記録。歴史的同一性を保持するが本v4作成者による原本再取得・全文同一性保証は主張しない。
QE方法論=docs/QE_2026-10-22/三点・採用記録・F01..F07。緩衝=docs/緩衝基盤_rev5補遺_QC/三点・採用記録・B0..B5台帳。製品新要求=docs/QE_2026-11-05/・docs/QE_2026-11-08/・docs/QE_2026-11-09/ の三点・採用記録・進捗・W履歴。各資料の時点/役割/証拠を分離。
技術=docs/specs/全契約、specs/Schema、packages/shell_core、native/rust_helper、apps/desktop_flutter、apps/mobile_flutter、installer、tooling、.github/workflows、FINAL_QA_QUEUE.md、release_blockers.registry.json、README/QUICKSTART/COMPATIBILITY_MATRIX/CLAIM/RELEASE_CHECKLIST/SECURITY。
原理順序=目的と保証対象の同一性→安全・権限・責任→全体機構→方法/試験→証拠/保証。古い「実機がないからQA不可能」という設計前提はS02で変更し、原資料の歴史を改竄しない。その他の旧要求を省略・新機能へ置換しない。欠測はUNKNOWNとして保持し、Codexは全数台帳を構築して文書差分で検査する。
