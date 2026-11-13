# v4 E0 要求・Consumer照合台帳（作業中）

状態：`OPEN`。本書はv4全要求を現在の実装・Consumerへ結び直す初期差分台帳であり、E0完了・BUILD_COVERED・製品合格を示さない。

## 基準と証拠の扱い

| 項目 | 観測 |
|---|---|
| Repository main | `gatchimuchio/D4-POCKET` |
| v4採用後main | `300d3a58637441b129d5f096fd445dd1e53d12e9`。v4正本・Registry・Validator同期を含む |
| 製品コード基準 | `d503a4ef6daac8942abd1cb011442ce28c78ec55`。v4採用commitは文書・統治変更であり、製品コードの新規実装を含まない |
| Release Registry | 18件、16件unresolved／2件resolved、`release_ready=false` |
| 既存W00 | [2026-11-05 W00](../QE_2026-11-05/W00_要件consumer照合_20261105.md)のproducer/consumer調査を再利用する。`SCOPED_PASS`はv3範囲の履歴で、v4全数E0の合格ではない |
| 直近状態 | [v3進捗](../QE_2026-11-09/進捗.md)のW00〜W08／N01〜N11履歴を引き継ぐ。過去statusはその時点のまま保持し、v4状態へ自動転記しない |
| C/Phase対応 | [D4_POCKET_PHASE_MAPPING](../D4_POCKET_PHASE_MAPPING.md)を範囲対応の原典として使用する。範囲対応はruntime実装・PASSの移転を意味しない |
| F07 | [D4-AUD-F07](../QE_2026-10-22/F07_rev5原本要求照合_20261030.md)は当時の原本hash照合・要求差分の履歴証拠として使う。現時点の原本再取得やv4実装合格とは扱わない |
| ID個数確認 | v4 S04からN=11、phase mapからC=35/Phase=46、RegistryからBlocker=18（unresolved=16/resolved=2）をローカル抽出で確認。P/K/Buffer/Q/FQ/QE/監査の全ID個数・内容照合は未完 |

LedgerのE0-04列は、現ID、旧ID、原文参照、対象、目的、Producer、実Consumer、Contract/Data、正常作用、否定作用、停止/Recovery、標準OS候補、配布物、既存実装、BUILD_GAP、SourceSHA、TestClass、Evidence ID、Blocker、状態、残余である。下表は現物で既知の初期差分だけを記録する。未照合の詳細は空欄にせず`UNKNOWN`として後続のE0作業で埋める。

## N01〜N11 初期差分

| ID | v4要求（[仕様S04](01_D4Pocket_統合製品実装仕様書_v4.md)） | 既存Producer→Consumerの観測 | v4状態／残り |
|---|---|---|---|
| N01 | 一般利用入口からのAI実Task、結果、停止・Recovery | Agent Center→`AgentTaskClient`→Broker `AgentTask実行`→Dialogue→登録Adapter→Workspace／Task state/result→Coordinator／Agent Centerの候補経路をsourceで追跡。Windows W01/G1の限定LIVE_PRODUCT履歴あり。 | `OPEN`。Windows W01の履歴範囲を超える統合Acceptanceは未成立。Mobile画面にTask開始Consumerがなく、Device Link許可操作にも`AgentTask実行`がないためMobile Task経路は`BUILD_GAP`。他OS標準環境・配布・現結果Consumerの全数接続を照合する |
| N02 | AIが要求する実MCP Toolの結果を同Agentの後続Taskが使う | Codex向け一回提案Tool→Agent Centerの承認済みsource result／現Catalog照合→Broker MCP実作用→Broker内一回context→fresh Permission／Approval付き後続Task、というDesktop source経路を追跡。 | `OPEN`。Desktop実Codex→実MCP→結果依存後続Taskの同一installed candidate証拠は未成立。MobileはMCP状態画面がTool作用を非対応と表示し、Device Link allowlistにも`MCP Tool実行`がなく`BUILD_GAP`。五OS標準環境・配布・結果/Audit consumerを照合する |

## N01/N02 現行Source→Consumer追跡（E0差分）

照合対象の製品sourceは基準commit `d503a4ef6daac8942abd1cb011442ce28c78ec55`。v4採用・E0文書commitは下記source file群を変更していない（この2点の確認はsource差分なし）。下表の`WIRED`は現在のsource consumer接続を示し、実installed製品の受入れを意味しない。

| ID | Producer→実Consumer候補／データ | 正常作用とsource状態 | 否定・停止・Recovery境界 | OS・配布・証拠・不足 |
|---|---|---|---|---|
| N01 | `apps/desktop_flutter/lib/screens/agent_center.dart`: `_inspectTask`→`_startAgentTask`→`AgentTaskClient.inspect/start`。`packages/agent_orchestration/lib/agent_task_client.dart`のRuntime／Session／Workspace／instruction requestは`BrokerTransport`へ送られる。Rust `broker/protocol.rs::対話要求処理`→`mcp_center::enrich_agent_task_payload`→`broker/dialogue.rs::AgentTask操作`が登録Adapterへ渡し、`adapters/codex_cli.rs`がCodex CLIを実行する。Task state/resultは`AgentTaskExecutionCoordinator`とAgent Centerへ戻る。 | Desktop source上は`WIRED`。BrokerはWorkspace binding、current Adapter capability、Permission、本文hash／条件hashへ束縛した独立Owner Approvalを検査し、一回消費する。歴史証拠W01/G1はWindows normal Release/Codex CLIの限定Candidate UIによるcompleted Task／Workspace write／hash／Auditで`CLOSED`。 | Permission／Approval不足はBroker拒否。未対応Runtimeは拒否。Task開始時にscratchを予約し、cancel/deadline/crashでBroker停止・回復へ接続するsourceがある。Broker再起動時のinterrupted Taskはsuspendedとして復旧し、旧Permission／Approvalを復元しない。W01の試験・Task条件をv4全OS保証へ拡張しない。 | Windows W01の限定`LIVE_PRODUCT`履歴のみ。LinuxにはUbuntu 22.04 hosted Xvfb/X11のnormal Release positive Task（run #286、local no-auth Responses fixture）という別限定履歴がある。macOSの標準installed Taskは`UNKNOWN`。Mobile `agent_status.dart`はAdapter metadata表示のみでTaskを実行しない。`device_link.rs::許可操作`はTask操作を含まず、Mobile Taskはsource上`BUILD_GAP`。全OS標準version/arch/package/一般Providerの受入れは未成立。Windows W01原記録: [W01/G1](../QE_2026-11-05/W01_G1_通常ReleaseAgentTask証拠_20261107.md)。 |
| N02 | Codex AdapterはTask内stdio `d4p_task_request`／`d4_submit_mcp_tool_request`を一回だけ公開する。`adapters/codex_cli.rs::run_agent_task`はTool eventをproposalとして回収する。このstdio Toolは未権限のproposalをキューへ送りackを返すだけで、外部MCP Toolを呼ばない。Coordinatorはcompleted Task後に`AgentMcpTaskController.continueAfter`を呼び、Owner承認済みsource Task結果を照合後、current MCP catalogを再読込する。`McpConnectionClient.callTool`→Broker `MCP Tool実行`→`mcp_center::call_tool`が実server Toolを呼ぶ。成功時のbounded textはTask/runtime/session/workspace・source task/hashに束縛した5分・最大8件のin-memory one-shot contextとなり、後続Task requestはID/hashだけを渡す。Brokerがresultを未信頼データとして指示へ追加し、成功したTaskでcontextを消費する。 | Desktop source上は`WIRED`。Agent Centerはsource Task本文のnative結果表示承認後にproposalを解釈し、Toolはcurrent catalog／tool identity／schema／credentialを照合し、native Owner確認・一回Permission・Audit後にのみ呼ぶ。後続Taskは同じRuntime／Session／Workspaceだが別Taskとして事前検査し、新しいWorkspace Permissionと別Owner Approvalを要求する。Tool receipt／Auditへ本文を出さずhash中心にする。 | source task/result hash／current Tool／Credential不一致、Catalog stale、期限切れ、replay、MCP errorは拒否。結果不明時はMCP接続を隔離・停止し再送せずOwnerに副作用確認を要求。Audit失敗でも停止・隔離。Broker再起動でvolatile contextは失われ、後続Taskはcontext unavailableとしてfail-closedする（context復旧は実装されていない）。Mobile画面はTool作用を非対応と表示し、Device Link allowlistにも`MCP Tool実行`なし。 | Desktop sourceはWindows/Linux/macOS用Rust cfgとnative Owner経路を持つ。既存Flutter controller/widgetおよびRust Broker/stdio testsは`FIXTURE`。2026-11-07 W02記録ではinstalled UI停止観測後の実Codex Tool／結果依存follow-upを`NOT_RUN`とし、W02/G2を`OPEN`維持。2026-11-12 v3進捗も同じG2未成立を引継ぐ。Mobileは明示的`BUILD_GAP`。各Desktop OSの標準installed candidate・実Provider・同一candidateのTool→後続Task→Audit証拠、全OS配布経路は未成立。W02原記録: [W02作業記録](../QE_2026-11-05/W02_作業記録_20261107.md)。 |

### Source SHA・該当ファイル

以下のblob SHAはSourceSHA `d503a4ef6daac8942abd1cb011442ce28c78ec55`から取得した。照合時HEAD `4e3265d41319eacceffd2208ad3afecfa27b2739`時点で当該file差分は0。

| source file | blob SHA |
|---|---|
| `apps/desktop_flutter/lib/screens/agent_center.dart` | `2472faeb31a64d624423c30a0a30d0f153d2e2a3` |
| `apps/mobile_flutter/lib/screens/agent_status.dart` | `52e4907897c2bb3674bb8523204a787d3a6cb002` |
| `apps/mobile_flutter/lib/screens/mcp_status.dart` | `40af06807f8bdf1280d860b3d9ffa32ee2e22cc0` |
| `packages/agent_orchestration/lib/agent_task_execution_coordinator.dart` | `feb9aacb7ed77e6f4e76e9d9371290decca9cf7b` |
| `packages/agent_orchestration/lib/agent_mcp_task_controller.dart` | `fa46ff071cfb306d3e463a9ad2c79b9149dab6f5` |
| `packages/agent_orchestration/lib/agent_task_client.dart` | `a31fbb9f980a0039f809fd763c91f055fe9d02d0` |
| `packages/agent_orchestration/lib/mcp_connection_client.dart` | `71b17973216160bdd2bc447243ba0b04fd0fec63` |
| `native/rust_helper/src/mcp_task_tool_stdio.rs` | `6107890e27be2b9f6bef93a9d46828a488aa1bf1` |
| `native/rust_helper/src/adapters/codex_cli.rs` | `e5c9d1581ae9a1c0a3ae7fcfa626fdca6fea6d28` |
| `native/rust_helper/src/broker/mcp_center.rs` | `8f9ab897bdc625e448241ebd9b46f995cd1f22f6` |
| `native/rust_helper/src/broker/protocol.rs` | `9b2076d5f31ca6d77f1f87973d1b394aaf751961` |
| `native/rust_helper/src/broker/dialogue.rs` | `c3c3fddb630dd06b7e6cc1c13b39f3d4f6f6a999` |
| `native/rust_helper/src/broker/device_link.rs` | `c2e1ff88ff34a5764b2b4fb1d3f7abd8727e5c51` |
### N03〜N11 初期差分

| ID | v4要求 | 現行Producer→Consumerの観測 | v4状態／残り |
|---|---|---|---|
| N03 | 独立した異種Runtimeを共通Task/Result契約で使用 | Codex CLIはBrokerのAgent Task Adapter。MINIDORAは共通対話AdapterだがAgent Task methodを実装せず、既定unsupported。Desktop登録UIはCodex CLIのみ。 | `OPEN`。二つ目の独立したTask-capable Runtime、能力選択・拒否・結果・権限の製品経路と実Runtime証拠が不足（`BUILD_GAP`）。 |
| N04 | Compare／Handoff／A2Aの実作用と権限非継承 | Compare/Handoff/A2A source consumerはある。Compare実行時隔離、外部Agent連携、Artifact転送、Host越境は現installed候補の一貫した実証なし。Hostはmetadata registry。 | `OPEN`。source接続と実作用証拠を分離し、Compare/Handoff/A2A/Hostの残件を照合する。 |
| N05 | Capability CatalogからComposeを経て元Toolを実行 | 現行BrokerのRuntime/Agent metadata投影と接続中MCP CatalogをCompose前後で再照合。Settings UIは受理済みManifestを`MCP Tool実行`へ渡し、Brokerが同じManifestを再検査して元Toolを呼ぶ。旧W00のsource gapは現source上解消。 | `OPEN`。source `WIRED`。CapabilityはAdapterの未信頼宣言metadataで、専用の正規Capability Registryとの接続は未確認。Windows fixture test sourceは存在するが未実行。実installed product evidenceは未成立。 |
| N06 | 独立App Exportと選択Moduleの実除去 | Brokerが新規App/Audit identity・Manifest・Module依存planを作る。Windows/Linux Developer ExporterとLauncherはManifest hash/identityを扱う。Flutter UI Moduleのcompile-time define/AOT比較も存在する。 | `OPEN`。runtime evidenceは`runtime_manifest_consumed=false`、`standalone_app_verified=false`、`binary_pruning_verified=false`。Rust/第三者Module除去、構成による実機能変化、cold-start/resource測定、独立Installer/installed evidenceに`BUILD_GAP`。 |
| N07 | AI Edit proposalを別主体で審査しcontrolled apply/rollback | owner proposalのAudit、提案由来を束縛した別Workspace Taskの事前検査、fresh Permission/Approvalの通常経路はsource接続あり。 | `OPEN`。独立審査結果をapply前に確定するTask／Broker状態と、proposal由来の変更を照合するcontrolled apply/rollbackが`BUILD_GAP`。 |
| N08 | Bufferが元Taskの意味を保ってOS/依存差を調停 | Codex CLI Task専用Workspace scratch・TEMP/TMP束縛・journal Recoveryと、Task終端の非権限Buffer記録はsource接続あり。 | `OPEN`。実装は限定的なCodex TEMP/scratch調停。全OS/依存差、独立した前後意味・副作用比較、B5自己適用・B6全Consumerは未成立／`BUILD_GAP`。 |
| N09 | UIから実Task、結果、観測、停止・Recoveryを操作 | Desktop Agent CenterはBrokerのTask start/state/cancel/resultをTask IDへ束縛。Workspace Inspector、History、Eval、Resource、Observation、Notificationにも個別のBroker consumerがある。Trayは停止要求のみ。旧対応はP0/P5/P12、C1..6/C12..14/C20..23、K03/K04/K05/K10/K13。 | 部分`WIRED`。同一TaskへのDiff/Eval/Resource/Trace/Audit/Recovery/Searchの結合は未成立。Audit/Problems/Recovery/Searchは起動時snapshot由来、History replay/branchはDialogue新規要求でAgentTask再生ではない。AgentTask pause/resume/restart未実装。MobileにAgentTask操作なし。W02-B01 UI応答停止の原因は`UNKNOWN`。N09/W00は`OPEN`／`BUILD_GAP`。 |
| N10 | Windows/macOS/Linux/Android/iOSで通常導入・利用・更新・撤去 | Windows/LinuxにはExport・portable/bootstrap/updateの部分経路、macOS/Android/iOSにはnative/UI・debug/Simulator build経路がある。一般利用者のOS正規配布からinstalled実Task・更新/復旧/撤去まで結ぶ現Release候補は未成立。 | `OPEN`。OS別source・package役割・既存test/evidence・不足を下記へ記録。標準保証のversion/arch/package/最低資源と各OSの一貫したinstalled lifecycleは未確定。 |
| N11 | 全要求・Source・Consumer・証拠・Blocker・Release判断の結合 | Registry、Release Gate、Manifest、Windows evidence validator、development evidence bundle、Conformance、Final QA Queueは存在する。各々の責任と制約を下記に追跡。release_evidenceには`.gitkeep`以外の現候補evidenceがない。 | `OPEN`。E0全ID・実Consumer・否定/Recovery/OS/Evidence結合が未完。18 blockerは16 unresolved/2 resolved、`release_ready=false`。Q0〜Q7は`DEFERRED`。 |

## N03/N04 現行Source→Consumer追跡（E0差分）

照合Sourceは `d503a4ef6daac8942abd1cb011442ce28c78ec55`。調査開始HEAD `4e3265d41319eacceffd2208ad3afecfa27b2739`時点で、下表の製品source差分は0。`WIRED`はコードの接続状態、`BUILD_GAP`は要求Consumerの不存在または未接続、`FIXTURE`は試験区分であり、installed product受入れを意味しない。

| ID | Producer→Consumer／契約 | 正常作用・source状態 | 否定／停止／Recovery・Authority | OS／試験区分／Evidence／残余 |
|---|---|---|---|---|
| N03 | `main.rs`の`--codex-runtime`／`--minidora-runtime`→`ipc_server.rs`が各実装をBrokerへ登録→`dialogue.rs`の共通`実行系Adapter`→`protocol.rs`の`AgentTask実行`→選択Adapter。CodexはCLI Taskを実装。MINIDORAは`/health`、`/api/capabilities`、`/api/chat`、`/api/trace/{id}`を用いる対話Adapter。 | Codex CLI→Broker Agent Taskはsource上`WIRED`。MINIDORAは同じ対話契約へ接続するが、Agent Taskは実装していない。traitの既定は`AgentTask実行対応=false`かつ`AgentTask非対応`を返し、MINIDORAのimplにTask overrideはない。ゆえに二つ目のTask Runtimeは`BUILD_GAP`で、2系統のCODE_PRESENTを2種の実Taskと数えない。 | BrokerはAgent Task要求時に現在AdapterのTask capabilityを検査し、未対応を拒否する。MINIDORAの未実装能力がmetadataで昇格しない。MINIDORA対話はloopback限定、期限・取消・HTTP status/header/body長・JSON/trace結合不正を拒否する。Permission/ApprovalはBrokerが所有。 | Rust fixtureはMINIDORAの対話境界とCodex Adapterを対象にするが、本作業で未実行。W01/G1はWindows normal ReleaseのCodex CLI限定証拠で、MINIDORAのTask証拠ではない。N03/W03/G3は`OPEN`。`--minidora-runtime`はBroker起動引数で、Desktop Runtime Centerは一覧/資源/lifecycle表示、Agent Centerの登録入口はCodex CLIのみ。MINIDORAのGUI登録・Task-capable導入経路、二つ目のRuntimeの実Task/失敗/Authority evidence、Provider/Capability差の製品表示、五OSの標準packageは未成立。Mobile Runtime Task consumerも未確認。 |
| N04-Compare | Agent Centerのactive Session選択→独立Workspace検査→同じTask本文で2件の`AgentTaskClient.inspect`→各SessionのPermission/Approval確認→`startAgentComparisonTasks`が2つのBroker Task要求を並列開始。状態/結果はTask IDごとに別々に照会。完了結果を選択し、比較A/Bと別の第三Session/Workspaceへ未信頼結果を封入した新規Taskを事前検査する。 | UI/Coordinator/Broker要求はsource上`WIRED`。Workspace/Session IDの独立性検査、各Taskの別認可、片側開始例外の他側への非伝播、selected resultのhash/Task bindingがある。選択結果のApplyは直接Workspaceへ書かず、独立した適用先Agent Taskとして再評価する。 | 片側の開始・結果照会失敗は他側へ伝播させず、曖昧な開始結果は再送しない。Session別cancel gateを使用。選択・Apply時はsource Task/result/target stateの変化を再照合。結果本文は未信頼データ。画面もTask test主張を未検証、Task固有資源値をunknownと表示する。ただし投影上のID/Workspace相違は実filesystem隔離のLIVE証拠ではない。 | `agent_coordination_test.dart`、Agent Center widget、Coordinatorの試験は`FIXTURE`／Widget。過去の同一Runtime Compare source試験履歴はinstalled隔離証拠にしない。v4 W03/G3は`OPEN`。installed同一候補での同時実行、cross-agent contamination、失敗/取消分離、selected result適用/Auditの一気通貫Evidenceは未確認。 |
| N04-Handoff | 完了済みsource Taskの承認済みfull result→`AgentHandoffPackage`（task context/result/artifact/changed files/diff/test resultをbounded化）→別Runtime/Session/Workspace向け新規AgentTask request→Broker preflight→通常Permission/Approval後のTask consumer。外部A2A選択済みArtifactも別のhandoff packageとして新Task preflightへ送る。 | Internal HandoffとA2A Artifactのpackage/preflight sourceは`WIRED`。source/targetのSession・Workspace分離、approved result/hash束縛、変更検知、Artifact本文の未信頼明示、Permission/Approval/Credential/Trust/hidden stateを含めない契約がある。preflight自体はTask実行を行わず、受信Taskで再認可する。 | source/target/result変化、未完了結果、hash/size/path/schema不正を拒否。HandoffはAuthorityを移送しない。A2A Artifactはcompleted Taskから明示選択されたtext-only bounded Artifactに限定。A2A側の未確定外部作用はunknown/suspendedとし、再送しない。 | `agent_handoff_test.dart`、`a2a_artifact_handoff_test.dart`、Agent Center Widgetは`FIXTURE`。v3線Bではfake Broker/widgetとlocal fixture範囲を記録、実外部Agent・installed Handoff/A2A Artifact消費は未成立。N04/W03/G3=`OPEN`（[v3線B履歴](../QE_2026-11-09/進捗.md)）。 |
| N04-A2A/Host | Desktop A2A UI→`A2aConnectionClient`→Broker operation→`a2a_center.rs`→`a2a.rs`のAgent Card/SendMessage/SendStreamingMessage/GetTask・選択Artifact network call。Host CenterはHost登録/一覧/表示切替のmetadata registry。 | Rust BrokerにA2A network consumerがある。Agent Card登録はnative Owner確認とAuditを通る。SendMessage/Stream/GetTaskは各要求のnative Owner確認・現在endpoint/payload結合one-shot Permission・Audit後に作用する。A2A metadataはTrust/Authorityを生成しない。Host code自身はremote接続、Permission、Approval、Credential、Task権限を生成しない。 | Message/Stream/GetTaskの曖昧な非冪等結果はunknownとして停止し再送しない。外部本文/Artifactは未信頼入力。macOSは現AGENTS規則によりIPv4 loopback Agent Card/metadata限定で、Message/Task/Artifact/Stream作用を有効化しない。 | Windows/LinuxのRust network sourceは存在するが、実外部Agentが同一installed candidateでCard→作用→Artifact→受信Task→Auditまで通る証拠は未成立。macOSはmetadata-only、Mobile consumerは未確認。既存A2A Rust loopback／Flutter fake Broker/widgetは`FIXTURE`。Host越境TaskはConsumerがなく`BUILD_GAP`。W03/G3=`OPEN`。本単位ではRuntime testを実行していない。 |

### N03/N04 Source SHA・該当ファイル

以下のblob SHAは全て同じSourceSHA `d503a4ef6daac8942abd1cb011442ce28c78ec55`から取得。照合時HEAD `4e3265d41319eacceffd2208ad3afecfa27b2739`時点で当該file差分は0。既存test pathはfixture/Widget区分の確認用であり、本作業では実行していない。

| source file | blob SHA |
|---|---|
| `native/rust_helper/src/main.rs` | `b0a5df285db357172e7970074d7a67189f3384a3` |
| `native/rust_helper/src/adapters/mod.rs` | `06c540a59437b903b3f045b5aafb3790c21103bd` |
| `native/rust_helper/src/adapters/codex_cli.rs` | `e5c9d1581ae9a1c0a3ae7fcfa626fdca6fea6d28` |
| `native/rust_helper/src/adapters/minidora.rs` | `31c756ac7c9a264d73ed0e9654dbae599be83921` |
| `native/rust_helper/src/broker/ipc_server.rs` | `58a404b920ef5905e174668ec2de0e04a9e8b044` |
| `native/rust_helper/src/broker/dialogue.rs` | `c3c3fddb630dd06b7e6cc1c13b39f3d4f6f6a999` |
| `native/rust_helper/src/broker/protocol.rs` | `9b2076d5f31ca6d77f1f87973d1b394aaf751961` |
| `native/rust_helper/src/a2a.rs` | `9d7c6d9ab938ad5178d6ca46b3933dc3e61c6061` |
| `native/rust_helper/src/broker/a2a_center.rs` | `73ad601abd64a33ccefde4b73ab5f9dc4f728504` |
| `native/rust_helper/src/broker/host_center.rs` | `c27707faa61e5270b1c6d79e3ee3c5fe6ccd0bf8` |
| `apps/desktop_flutter/lib/screens/runtime_center.dart` | `d2c7af172073d2b10c76a6416b90f011f10e95db` |
| `apps/desktop_flutter/lib/screens/agent_center.dart` | `2472faeb31a64d624423c30a0a30d0f153d2e2a3` |
| `apps/desktop_flutter/lib/services/agent_coordination.dart` | `bdc13464f0e42b740a2bca3b7e8828a5019056d0` |
| `packages/agent_orchestration/lib/agent_comparison_coordinator.dart` | `6b5deaacb500a23e6bfdc37649d61ebc0d885127` |
| `apps/desktop_flutter/lib/services/agent_handoff.dart` | `a85110d96cbade732ae0e1bfea2529f03f37523c` |
| `apps/desktop_flutter/lib/services/a2a_connection_client.dart` | `19335054afe35bc06597629d92ca2e635ce50069` |
| `apps/desktop_flutter/lib/services/a2a_artifact_handoff.dart` | `e7e2f3454e2843f1d4f4e7b5aedfa55014cde05c` |

関連test source: `native/rust_helper/src/adapters/minidora.rs`、`native/rust_helper/src/broker/dialogue.rs`、`native/rust_helper/src/broker/a2a_center.rs`、`native/rust_helper/src/broker/protocol.rs`のRust test、`apps/desktop_flutter/test/agent_coordination_test.dart`、`agent_handoff_test.dart`、`a2a_connection_center_test.dart`、`a2a_artifact_handoff_test.dart`。これらはE0 source mappingで確認した既存対象で、今回のRuntime/installed acceptance結果ではない。

## N05/N06 現行Source→Consumer追跡（E0差分）

照合HEADは`06db10a8411c7c9f7e9913c01e8b2ff82b7c75e8`、製品コード比較基準は`d503a4ef6daac8942abd1cb011442ce28c78ec55`。列挙したN05/N06製品source・schema・test fileは基準からの差分0。Source traceだけを行い、Rust/Flutter/Exporter/product testはこの作業単位で実行していない。`WIRED`はsource call graph、`FIXTURE`はtest fixture、どちらもinstalled product acceptanceではない。

### N05 Capability/Compose

| 項目 | 現行観測 |
|---|---|
| Producer→Consumer | `settings.dart::_refreshComposeRuntimeAgentCatalog`はBrokerから現在登録Runtime/Agentを取得し、`_refreshComposeMcpCatalog`は接続済MCPとToolを取得する。`_runCompose`は選択内容を`compose_client.dart`経由でBroker `GUI Shell構成`へ送る。受理Manifestとdraftが一致するときだけ選択Tool操作を有効化し、`_runComposedTool`が`McpConnectionClient.callTool(..., composeManifest: manifest)`を呼ぶ。 |
| Catalog/source | `Broker::compose_runtime_agent_catalog`→`dialogue.rs::compose_catalog_snapshot`は現Broker登録AdapterのRuntime IDと`agent_adapter_projection`から作る。metadata形状とAdapter/Agent bindingを検査する。Compose capabilityはこのAgent Adapter metadata内の宣言値であり、CapabilityのAuthorityではない。MCP Tool候補は現在の接続Catalogから取得する。汎用`specs/capability.schema.json`はPython `shell_core/runtime_state.py`から参照されるが、Rust Compose Catalogのruntime consumerではない。 |
| 互換・危険条件 | `compose_center::validate_catalog_selection`はRuntime ID、選択Agentと同一Runtimeのready/degraded状態、Capability宣言、Tool選択時の同一Agent instance上の`task_execution/tool_support/mcp_support`、生存かつ非隔離のMCP接続、現在Catalog内のTool IDを確認する。欠落・不整合はreject。Composeは`manifest_only`を返し、App identity/Permissionを作らずAuthority類を継承しない。 |
| 実作用・Authority | `MCP Tool実行`は同じManifestを受け、現在Runtime/Agent/MCP/Toolを再照合する。Rust native Owner確認と永続Audit、Tool schema/identity検査、一回Permission/Auditの後に現在接続の元MCP Toolを呼び、result hashとbounded metadataを返す。Flutterは本文を表示しない。Agent metadata/CatalogからTrust・Permission・Approvalは生成しない。 |
| 否定・停止 | Composeはunknown Runtime/Agent/Capability、unsupported capability、選択instanceをまたぐ互換性寄せ集め、unknown/stale MCP/Toolを拒否する。MCP実行時もManifest選択が古ければTool送信前に拒否し、Owner/Audit/Permission不成立時は作用しない。 |
| Test class / evidence | `compose_center.rs`には現在Catalog/互換性/同一instance/manifest-only/権限非継承のRust testがある。Windows限定`mcp_center.rs::mcp_catalog_compose_rechecks_current_connection_and_rejects_stale_selection`はBrokerから子`cmd.exe` fixture MCPへ接続し、stale選択拒否とselected Tool一回呼出しを検査するsourceがある。Flutterの`compose_client_test.dart`と`settings_compose_test.dart`はclient/widget fixture。全て本単位`NOT_RUN`。子process fixtureの`LIVE_RUNTIME`ラベルはfixtureを製品runtime証拠へ変換しない。 |
| 契約差分・残余 | 現行施工正本v4 S04/N05は現在Catalog→互換/権限→実Tool作用を要求し、これが優先する。補助的な`docs/specs/gui-shell-compose.md`には参照ID入力・候補発見/実在確認なしという旧記述が残るため、E1文書同期で更新する。Sourceはv4要求の一部consumerまで`WIRED`だが、Adapter capability宣言が実能力と一致するか、AI自身の実要求、同一installed candidateの実MCP副作用/結果/Auditは`UNKNOWN`。この文書差分は現N05 blockerへ追加しない。N05/W04/G4は`OPEN`。 |

### N06 Standalone Export/Module

| 項目 | 現行観測 |
|---|---|
| Producer→Consumer | Settings Export UIは選択Compose Manifest、platform、optional Module IDsを`export_client.dart`から既存Brokerへ渡す。`export_center.rs`は固定Export directoryへManifest file/Receiptを作る。Windows/Linux Developer ExporterはReceipt+Manifestのhash/schema/targetを検証してportable bundleを組み立て、Desktop Launcherはbundle内`product_manifest.json`とcompile-time identityを照合する。 |
| 新規identity / 安全境界 | Broker Exportは新規App IDとAudit store IDを作り、Authority/Permission/Approval/Credential/Audit chainを継承しない。任意path/overwriteを認めず、native Owner確認とAuditを使う。LauncherもApp/Audit identity・platform・非継承条件を検査する。これは独立identityのsource接続であり、独立起動製品の実証ではない。 |
| Module plan | `resolve_module_plan`は固定Catalogから必須/安全保持Module、重複/未知/必須Module選択、依存閉包を検査し、選択/除外IDをManifest/Receiptへ記録する。`binary_pruning_status`は`not_applied`。Launcherも現Manifestでこの値を要求する。 |
| Build/runtime接続 | Windows/Linux ExporterはManifestをhash-boundでbundleへbyte-for-byte格納し、App/Audit identity等をbuildへ渡すが、build evidenceは`runtime_manifest_consumed=false`を明記する。LauncherのManifest consumerは固定hash・identity・非継承条件検査であり、ComposeのRuntime/Agent/Tool/Capability/optional Module構成に応じてruntime機能を構成するconsumerではない。生成物証拠はWindows `standalone_app_verified=false`、Linux `formal_distribution_claimed=false`、両者`binary_pruning_verified=false`。 |
| Pruning / 計測 | `build_module_pruned_windows.py`のcompile-time defineはFlutter UI surfaceに限定され、安全Core/Rust/third-party moduleをpruneしない。Windows AOT comparisonはsurface reportを作るが`runtime_unverified`、cold start/resource comparisonは未測定、`binary_pruning_verified=false`。Linux ExporterもFlutter AOT report/surface一致とartifact hashまでで、意味上の除外・runtime消費・resource減は保証しない。 |
| Test class / evidence | `export_center.rs`にはnew identity/non-inheritance、固定保存、Module dependency closure、unknown/duplicate/required module拒否等のRust unit testがある。`tooling/tests/test_export_linux_product.py`と`test_linux_portable_archive.py`はExporter/build/archive test（build pathはsubprocess mock/fixtureを含む）。`run_conformance_skeleton.py`にはcontract/static evidence validatorがある。いずれも本単位`NOT_RUN`であり、現在のinstalled独立製品の起動・構成消費・pruning証拠ではない。 |
| 残余 / 状態 | Compose/runtime manifestを実消費して機能を構成する独立App consumer、正規Installer/installed run、選択外機能のbinary/resourceからの除去と必要機能保持、実測比較は`BUILD_GAP`。署名はS04の別条件。N06/W05/G5は`OPEN`。release blocker `rev2_export_owner_ui_authority_path`、`rev2_module_pruning_binary_and_measurement`を維持する。 |

### N05/N06 Source SHA・該当ファイル

下表のblob SHAは全て製品基準`d503a4ef6daac8942abd1cb011442ce28c78ec55`から取得した。source群は照合HEAD `06db10a8411c7c9f7e9913c01e8b2ff82b7c75e8`まで差分なし。

| source file | blob SHA |
|---|---|
| `docs/specs/gui-shell-compose.md` | `6b43bb80f73d4268e734ac0afbc58b33ba07339f` |
| `docs/specs/gui-shell-export.md` | `3d7a1156e4bf095e4f6a64792b6e1683c14f9b17` |
| `docs/specs/gui-shell-module-pruning.md` | `5568c677be30402d709cc365b79eceb0e96ce831` |
| `specs/agent_adapter.schema.json` | `e0792589a2bae168ba221af4f4f9abb505815c12` |
| `specs/gui_shell_compose.schema.json` | `89b3d8e0e09c802d555fd6fee233daebc0fec045` |
| `specs/gui_shell_compose_catalog.schema.json` | `f9c76f5fa87bfb0866f33745dcd5a8a4e74864d5` |
| `specs/gui_shell_export.schema.json` | `8b96d344ac0f6a54011237be8b2d85964be48c4b` |
| `specs/gui_shell_export_manifest.schema.json` | `bf136b1a1daa258d33af3635ee414a2d4312bec5` |
| `specs/gui_shell_module_catalog.json` | `339cb8894b25ae963956e582c42b9e8fcc0d3931` |
| `specs/gui_shell_module_catalog.schema.json` | `bd54dd7903a1734ea4027d3f780b7a2b49e10c8e` |
| `apps/desktop_flutter/lib/screens/settings.dart` | `66374ec4157d573939e7f9cc5c36aecf5d65e251` |
| `apps/desktop_flutter/lib/services/compose_client.dart` | `8f9657d866b007a4d2212abefca4df9add81a702` |
| `apps/desktop_flutter/lib/services/export_client.dart` | `70d77edc6beb9e209e6b362219d1b22d69375489` |
| `native/rust_helper/src/broker/compose_center.rs` | `d80658fd9a342140acc35560b2e4c4e4ae97d888` |
| `native/rust_helper/src/broker/dialogue.rs` | `c3c3fddb630dd06b7e6cc1c13b39f3d4f6f6a999` |
| `native/rust_helper/src/broker/mcp_center.rs` | `8f9ab897bdc625e448241ebd9b46f995cd1f22f6` |
| `native/rust_helper/src/broker/export_center.rs` | `477e44d86ebf18d2b004a7294c9cdb14e35aa0f0` |
| `native/rust_helper/src/desktop_launcher.rs` | `6a9f335de07fa474ca10043aea63ad6e988b572d` |
| `native/rust_helper/src/linux_desktop_launcher.rs` | `f58fb226f21093c2d1f062259d394b4f8ec022f6` |
| `tooling/export_windows_product.py` | `31856179881a5a5aae1ba4451732e6de6cd7f338` |
| `tooling/export_linux_product.py` | `47fe9a3032a5b2680e3f7d39b7cd64120ac83843` |
| `tooling/build_module_pruned_windows.py` | `490aae7209ea5988f8e2cdcb05fe092b3d29504a` |
| `tooling/compare_module_builds_windows.py` | `09d926b9c6b611cd59e524cc947ce2dce1c81247` |
| `tooling/tests/test_export_linux_product.py` | `079a187de49cf8b2fb6a7ddadb4175d6916163d1` |
| `tooling/tests/test_linux_portable_archive.py` | `c04ac3c5d6d58396f650e9530b6731c6bb2037d6` |
| `tooling/conformance_tests/run_conformance_skeleton.py` | `a4b69353e43f21be0a12cc46cd565fee69a9691e` |
| `apps/desktop_flutter/test/compose_client_test.dart` | `c3fe30faa668482e5a4e42b2f5f5e5b218a01dc8` |
| `apps/desktop_flutter/test/settings_compose_test.dart` | `a78353f93617cc7b17391549ea256f007a7acdb1` |

## N07/N08 現行Source→Consumer追跡（E0差分）

照合HEADは`03f7b92d4dc67a544e8fa1e9bbaac5d3d912723e`、製品source比較基準は`d503a4ef6daac8942abd1cb011442ce28c78ec55`。本節の製品sourceとFlutter/Rust test fileは基準HEADから差分なし。`tooling/conformance_tests/run_conformance_skeleton.py`だけはsource基準後に更新されているため現blob SHAを別記した。ここはE0 source traceで、列挙testは全て本作業単位`NOT_RUN`。`WIRED`／`FIXTURE`はinstalled product acceptanceを意味しない。

### N07 AI Edit

| 項目 | 現行観測 |
|---|---|
| Producer→Consumer | `settings.dart`のAI Edit提案は`AiEditClient.propose`から`GUI Shell編集提案`をBrokerへ送る。受理後は`main.dart`からAgent Centerへ渡り、`AiEditProposalReview.toImplementationTaskInstruction`を含むAgent Taskを事前検査する。Task完了結果の適用準備は`AgentAiEditResultApplySnapshot`が提案ID/hash/Audit、元Taskと完了result hash、現在のsource/target Sessionを束縛し、異なるWorkspaceの未使用Sessionで再照合する。適用側は`AgentTaskClient.inspect`まででTaskを開始せず、通常Agent TaskのPermissionと一回Approvalを別途取得した後に開始する。 |
| Broker境界 | `ai_edit_center.rs::propose`はOwner経路限定、unknown field／非許可path／自己承認／非`proposal_only`を拒否する。Proposal本文はAuditへ保存せずpayload hashを記録し、file／Permission／Approvalは変更しない。結果・提案は未信頼contextで、source provenanceはApprovalにならない。Apply対象のstale source/result/sessionは事前検査前後のsnapshot比較で停止する。 |
| 要求との差・Gap | 最初のproposal Taskは通常Agent Taskとして実装作業へ進み得る。後段の別Workspace Taskには適用先fileの独立調査・test指示が含まれるが、審査結果をapply前に独立成果物として確定するBroker状態／Consumerではなく、同じTaskに調査・適用を委ねる指示である。適用後の実changed file/diffをproposal由来とBrokerが照合する専用operation、AI Edit専用Rollback/Recovery Consumerも見つからない。proposal受理とTask事前検査は`WIRED`、S04の独立審査→承認→controlled apply/rollback全体は`BUILD_GAP`。N07/W05/G5は`OPEN`。 |
| 否定／Authority | Brokerは通常資格での提案、自己承認、秘密pathを拒否する。Apply contextはAuthority/Permission/Approval/Credential/Trust非継承。異なるtarget Session/Workspace、fresh Task Permission/Approval、Owner apply instructionを使い、apply preparation中はTaskを開始しない。 |
| TestClass / Evidence | Rust `ai_edit_center.rs` owner-only/proposal-only/secret-path negative unit tests、Flutter `ai_edit_client_test.dart` receipt/hash/Audit/untrusted-data tests、`agent_coordination_test.dart` snapshot-binding test、`agent_task_client_test.dart` unused-session condition test、`widget_test.dart` fake Brokerでのproposal/preflight/別Workspace準備testが存在。`UNIT`／`FIXTURE`。実Agentによる独立review、専用Broker apply/rollback、installed product evidenceはこのsource群で確認できない。 |
| OS／状態 | 調査したsourceはFlutter Desktop設定・Agent Centerと共通Rust Broker。MobileのAI Edit consumer、各OS installed path/identityは`UNKNOWN`。Source traceのためRust/Flutter product testsは`NOT_RUN`。N07は部分`WIRED`、受入`OPEN`。 |

### N08 Buffer／調停

| 項目 | 現行観測 |
|---|---|
| Producer→Consumer | `codex_cli.rs`のAgent Task workerはWorkspace直下へTask-bound `WorkspaceTaskScratch`を作り、Broker journalへ予約／directory identityを保存する。`TaskTemporaryEnvironment`はHost `TEMP/TMP`、Linuxでは`TMPDIR`もscratchへ設定し、Codex sandbox環境overrideを生成する。Task終了時にcleanupし、observationをBrokerへ返す。`dialogue.rs::AgentTask進捗反映`はTask/runtime/Workspace/recovery binding/root identity/cleanupを突合後、version 2 Buffer記録とAudit、terminal Task historyを作り、Task stateへstatus/hash/Audit IDだけ投影する。Agent Centerはhash-only projectionを表示する。 |
| Contract／Recovery | `buffer_intervention_record.schema.json` v2は非権限記録と`作用後未照合`／独立照合条件を定義し、valid/authority-injection/unproven-absorption fixtureをstatic conformanceが検査する。`agent_task_scratch.rs`は最大件数付きjournal、Task/runtime/Workspace/recovery bindingとroot/scratch identityの一致を確認して再起動後に回収する。`runtime_content_reconciliation.schema.json`と`runtime_content_discard_reconciliation.schema.json`は削除中断時のOwner recovery照合契約であり、N08の意味調停consumerには数えない。 |
| 観測範囲・誠実性 | 現行Rustは成功結果にscratch observationがあれば`作用後未照合`／`同等性未証明`のBuffer記録を作る。調停成功へは昇格しない。`TaskTemporaryEnvironment`のafter hashは予定値から構成され、別OS実体の後状態を独立readbackした値ではない。Adapter自身のcleanup結果は記録するが、記録だけでは元Taskとの意味同等性を証明しない。Rust生成記録を実行時にJSON Schemaへ通すconsumerもsource上確認できない。汎用Broker成功経路ではobservationが`None`ならBuffer記録を作らず、全Adapterへの必須強制ではない。 |
| 否定／Failure | Scratch作成／cleanup失敗はTaskを失敗させ、journalを残してRecoveryへつなぐ。Brokerがobservation binding不一致、Tool結果不整合、Buffer記録/Audit不能を検出した場合はTaskを隔離し、結果本文を投影しない。journal recoveryはruntime/Workspace/binding/root/directory identity不一致やunsafe pathを削除せず保留する。 |
| 既存Runtime証拠 | `docs/REV3_PROGRESS.md`の過去installed候補ではMxC `mxc_appcontainer` TEMP write失敗が複数記録され、別runではchild中write PASSでもTaskは別のcontinuation timeoutでfailed、probe cleanupは`not_attempted`だった。別記録のD4-owned `.d4p-tmp-*` journalが空でもMxC TEMP物理削除は未証明。これらは限定`LIVE_RUNTIME`＋`FIXTURE`履歴であり、N08全体PASSへ昇格しない。MxC AppContainer TEMPとD4-owned WorkspaceTaskScratchは別責任境界として保持する。 |
| TestClass / Gap / 状態 | `codex_cli.rs`にenvironment binding、cleanup failure/Recovery、並行Task isolation tests、`agent_task_scratch.rs`にrestart recovery/identity mismatch tests、`dialogue.rs`にobservation mismatch/Audit failure/terminal projection tests、`protocol.rs`にBroker crash/restart scratch recovery tests、Flutter Agent Task state/UI hash-only projection tests、static schema fixturesが存在。Rust unit/fixture/conformanceは本単位`NOT_RUN`。既存実装はCodex Task scratch/TEMPの限定調停であり、全OS/API/ABI/toolchain/cache/process/network/OneDrive原因母集団、調停前後の同条件効果・副作用比較、B5自己適用、B6全Consumer/QCは未成立。B0〜B4の限定CLOSED、B5 `OPEN`、B6未成立を維持し、N08/W08/G7は`OPEN`。 |

### N07/N08 Source SHA・該当ファイル

blob SHAは照合HEAD `03f7b92d4dc67a544e8fa1e9bbaac5d3d912723e`の値。各fileとroleの詳細は上記traceを参照する。

| source/test/contract file | blob SHA |
|---|---|
| `apps/desktop_flutter/lib/services/ai_edit_client.dart` | `ba5e62311a02749d2f9fe0e4e72e2b0d4e0a497d` |
| `apps/desktop_flutter/lib/services/agent_coordination.dart` | `bdc13464f0e42b740a2bca3b7e8828a5019056d0` |
| `apps/desktop_flutter/lib/services/agent_task_client.dart` | `f54cefac29eba3dcf383bba19b4f64ee682e6217` |
| `apps/desktop_flutter/lib/screens/settings.dart` | `66374ec4157d573939e7f9cc5c36aecf5d65e251` |
| `apps/desktop_flutter/lib/screens/agent_center.dart` | `2472faeb31a64d624423c30a0a30d0f153d2e2a3` |
| `apps/desktop_flutter/lib/main.dart` | `6109c821ab5dc809224bec5234c4c59db1d5f988` |
| `native/rust_helper/src/broker/ai_edit_center.rs` | `7a1c2418dc6f71e14fc971385b6f4f2454d55b1c` |
| `native/rust_helper/src/broker/dialogue.rs` | `c3c3fddb630dd06b7e6cc1c13b39f3d4f6f6a999` |
| `native/rust_helper/src/adapters/codex_cli.rs` | `e5c9d1581ae9a1c0a3ae7fcfa626fdca6fea6d28` |
| `native/rust_helper/src/broker/agent_task_scratch.rs` | `de743cd819fb4533c9c3070c4e7bcae82e42d938` |
| `native/rust_helper/src/broker/execution_history.rs` | `8eeb1c26d06a9e4b01806413a4bfa8c2a73357d6` |
| `native/rust_helper/src/broker/protocol.rs` | `9b2076d5f31ca6d77f1f87973d1b394aaf751961` |
| `apps/desktop_flutter/test/ai_edit_client_test.dart` | `c5ecd0353fdc811ec7c4d52a9c2b7d10e1d01723` |
| `apps/desktop_flutter/test/agent_coordination_test.dart` | `6783e87a492d8436a855a7088b7cece38d122bd5` |
| `apps/desktop_flutter/test/agent_task_client_test.dart` | `0477a0bbbb321082494b6b55711815d34419fdd9` |
| `apps/desktop_flutter/test/widget_test.dart` | `f4e9c7b93f832ba2a2cefee779fd12426f240b04` |
| `specs/buffer_intervention_record.schema.json` | `26a1fa8ee9d010f18c157f5d3b66b9b34bdf94d5` |
| `examples/contracts/buffer_intervention_record.valid.json` | `3ebaf07e8a6241f9e44b8c78fefa45fa23523abf` |
| `examples/contracts/invalid/buffer_intervention_record_authority.invalid.json` | `981178ad70376fa633a76a686ed6e7d12287095d` |
| `examples/contracts/invalid/buffer_intervention_record_unproven_absorption.invalid.json` | `073dbfd11943588e045360dcce2009ae7245ebbf` |
| `tooling/conformance_tests/run_conformance_skeleton.py` | `c6f31d8cb561b48513c6784e54c85969c9cb953e` |

## N09 製品操作・観測・復旧 Source→Consumer追跡（E0差分）

照合HEADは`3d7213f1a36ad5f938f9602040570ac4777d1af0`。下記のSourceSHAは同HEAD上のblob値で、N09のSource追跡中に製品コードは変更していない。`WIRED`はコード上のProducer→Consumer接続のみを表し、installed product Acceptanceではない。既存fixture/widget/unit testは本source mapping単位では未実行。N09はW00に属するv4要求として`OPEN`を維持する。

| Surface | 現行Producer→Consumerと作用 | 境界・N09との差分 |
|---|---|---|
| Agent Center／実Task | Desktopの通常製品navigationからAgent Center→`AgentTaskClient.inspect/start/state/cancel`→Rust Broker `AgentTask実行/状態/取消`→Task ID付き状態・result hashへ接続。完了本文は同じTask ID/result hashの一回Content Exposure Approval後に取得し、表示投影を5分で消去する。Cancel UIは要求を送り、Brokerの終端状態を再確認する。 | 実Task開始・状態・取消・結果の限定経路は`WIRED`。Task単位のpause/resume/restart operationはない。Agent Centerの状態接続はTask IDに束縛されるが、他の観測画面すべてを同一Taskへ統合してはいない。 |
| Workspace／Diff／Recovery preview | Agent Center内のWorkspace Inspector→`WorkspaceClient`→Brokerの登録Workspace一覧、承認範囲付きtree/read/compare/change-list/diff/recovery-preview/whole-baseline operation。応答はWorkspace ID・登録hash・Approval・表示範囲へ結合し、可視性に応じて`LIVE_RUNTIME`または`INTERNAL_STATE`を要求する。 | Workspace実体の限定読取・比較は`WIRED`。Diffと変更一覧はWorkspace／baseline基準で、完了Task IDへ結び付くTask差分Artifactではない。Recovery previewは実復旧実行ではない。 |
| History／再実行／分岐 | History画面→`HistoryClient`→Brokerの現在History grant、Dialogue履歴、AgentTask履歴metadata。Task履歴はTask ID・result hash等の履歴投影で、本文・現在権限・稼働状態を表示しない。Dialogue replay/branchは参照Audit ID/hashと新入力から新しい要求を作る。 | Ownerの現在承認・Runtime binding・期限を再照合し、Task履歴読取とDialogue内容表示を区別する。Replay/branchは新しい承認待ちDialogueであり、AgentTaskの再生・pause/restartでも旧Authorityの再利用でもない。 |
| Evaluation／Resource | Evaluation Lab→`EvaluationClient`→Broker `evaluation_control`のDataset/Experiment/status/compare。実行時はDatasetのCaseごとに既存Dialogue sessionを作成し、決定論的Evaluatorで比較する。Resource client→Broker `実行系資源観測`はRuntime IDを指定し、実測またはunknownを返す。 | 個別consumerは`WIRED`だが、Evaluation ExperimentおよびRuntime resource observationにAgentTask IDの束縛はなく、同一Taskのtest結果／資源とは扱えない。評価はPermission/Audit/release判断を生成しない。 |
| Regression Case（旧C6） | 完了Dialogue paneが送信時Broker発行の要求ID/hashを保持し、ownerがredacted入力・条件等を明示した登録要求を作る。Rust起動器のoperation別native確認→既存Broker `回帰Case登録`→現在の対話result receipt／終了Audit／hashを再照合→Windows ProtectedStoreへprivate保存。通常IPCの`回帰Case一覧`はmetadata projectionのみ。削除／中断Recoveryも別のowner operation。 | source contractとproducer→consumerは`WIRED`。CaseはC5 Evaluation Datasetへ自動importされず、private定義を通常一覧・Audit・native確認へ返さない。回帰Case登録sourceの存在・fixture/conformanceはinstalled Owner操作やN09の同一AgentTask結合を証明しない。詳細はC6 crosswalkとSource SHA表。 |
| Observation／Trace／Notification | `ObservationCenter`はBroker Audit finalization後にbounded spanを作り、Observation/Trace画面は一覧を取得する。各spanはBroker operationと関連Audit IDを持つ。Notification CenterはAudit eventからsummaryだけを作り、既読／破棄はID/hashへ束縛する。 | Observationは最大1024 span保持・一応答最大256、現在は一span一traceで親なし。Trace IDはBroker内で採番され、Task IDではない。Notificationはsummary/navigation onlyで権限を作らず、raw payloadを出さない。Observation/Auditは責任が分離される。 |
| Audit／Problems／Recovery／Search | `ShellCoreClient.product`は起動時Broker応答から一つの`ShellSnapshot`を作る。Audit Viewer、Problems、Recovery、Dashboard等は`getSnapshot()`を参照する。Global Searchは同snapshotのbounded in-memory indexを検索する。 | Audit Viewerの一覧は起動時health/host/catalog/session/probe応答から作った投影であり、永続Audit chainのTask単位検索ではない。Problems/Recoveryも同snapshotで、Task crash後のlive Recovery catalogではない。Global SearchはBroker検索・最新Task/History検索ではない。 |
| Tray／Runtime lifecycle | Windows Trayはsnapshotを定期投影し、open/navigationとBroker `全Runtime停止要求`を持つ。要求応答は`停止実行済み=false`、Owner再承認待ち。別のRuntime CenterはBroker Runtime lifecycle status/approval/executeでstart/stop/restart/pause/resume/quarantineを操作する。 | Tray stopは実停止でなくOwner承認要求である。Runtime lifecycleのpause/restartはRuntime単位で、同一AgentTaskのpause/restartとは異なる。TrayはWindows固有、定期snapshotはTask IDに結び付かない。 |
| Mobile／UI応答 | Mobile Device Linkは限定operation allowlistでRuntime/Agent/Workspace/Dialogue metadata、Runtime resource/lifecycle state、通知summary、History、全Runtime stop request等を扱う。stopは停止を実行せずOwner再承認待ち。過去W02-B01にはWindows UI processの`Responding=False`とCPU増加が記録され、fake Broker widgetでは再現せず、原因は不明。 | Device Link allowlistに`AgentTask実行/状態/取消/結果`はなく、Mobileは同一AgentTaskの実行・回復consumerではない。UI応答停止は実観測履歴があるが根因修正・同条件installed再検証は未成立。 |

### N09 状態・TestClass・Evidence

Agent Center TaskとWorkspace/History/Evaluation/Resource/Observation/Notification/Regression Caseの個別source consumerは`WIRED`。要件N09が要求する同一Task単位の操作・結果・diff/eval/resource/trace/audit/recovery/search結合、AgentTask pause/resume/restart/replay、全体の停止・Recovery画面、Windows UI応答停止根因是正には`BUILD_GAP`または`UNKNOWN`を保持する。Runtime lifecycle機能やDialogue replayをAgentTask操作へ読み替えない。Regression Caseは完了Runtime Dialogueからのowner管理登録であり、同一AgentTaskの結果接続・C5 Dataset import・installed owner操作の証拠を別途要する。旧対応IDはP0/P5/P12、C1..6/C12..14/C20..23、K03/K04/K05/K10/K13（v4仕様S05・工程表R03線C）。

該当test sourceは`apps/desktop_flutter/test/agent_task_client_test.dart`、`history_screen_test.dart`、`workspace_inspector_test.dart`、`evaluation_lab_test.dart`、`runtime_resource_test.dart`、`runtime_lifecycle_test.dart`、`observation_client_test.dart`、`notification_client_test.dart`、`global_search_index_test.dart`、`widget_test.dart`、`apps/mobile_flutter/test/mobile_projection_client_test.dart`／`resource_overview_test.dart`、`packages/gui_shell_ui/test/evaluation_client_test.dart`／`runtime_resource_client_test.dart`／`runtime_lifecycle_client_test.dart`、およびRust Broker source内のunit tests。TestClassは`UNIT`／`FIXTURE`／`WIDGET`。本mappingでは全testを`NOT_RUN`。過去W02-B01のlimited Windows UI runtime observationは`docs/QE_2026-11-05/W02_作業記録_20261107.md`に残す。旧widget成功をinstalled UI root causeの修正証拠にしない。

### N09 Source SHA・該当ファイル

blob SHAは照合HEAD `3d7213f1a36ad5f938f9602040570ac4777d1af0`上の値。

| source file | blob SHA |
|---|---|
| `apps/desktop_flutter/lib/main.dart` | `6109c821ab5dc809224bec5234c4c59db1d5f988` |
| `apps/desktop_flutter/lib/screens/agent_center.dart` | `2472faeb31a64d624423c30a0a30d0f153d2e2a3` |
| `apps/desktop_flutter/lib/services/agent_task_client.dart` | `f54cefac29eba3dcf383bba19b4f64ee682e6217` |
| `apps/desktop_flutter/lib/services/agent_task_execution_coordinator.dart` | `5d8f48d0557bdcb1af838239abf9651893bce7f1` |
| `apps/desktop_flutter/lib/screens/workspace_inspector.dart` | `a02c95ec5062583f9c37523fbaf604a8d2da107e` |
| `apps/desktop_flutter/lib/services/workspace_client.dart` | `870b48e8f4bbd648bcfebc5886addca3784bcfaf` |
| `apps/desktop_flutter/lib/screens/history_screen.dart` | `dacc4c0ca15325c7f597cd1827a6455fb8ab5560` |
| `packages/gui_shell_ui/lib/src/history_client.dart` | `7f14ec2caf8b1e383bbf93483ae052100f921ae7` |
| `native/rust_helper/src/broker/history_access.rs` | `c004056b45c2f26859bf7f70a9ff2b632e358523` |
| `native/rust_helper/src/broker/execution_history.rs` | `8eeb1c26d06a9e4b01806413a4bfa8c2a73357d6` |
| `apps/desktop_flutter/lib/screens/evaluation_lab.dart` | `b74b3e96e7cd746874d1e19e05175548a5c66192` |
| `packages/gui_shell_ui/lib/src/evaluation_client.dart` | `3fe93e4a19bcba7c825f2a6e887f96f00016aa75` |
| `native/rust_helper/src/broker/evaluation_control.rs` | `570485772c3fce43d170d031f1769b7617214d40` |
| `native/rust_helper/src/broker/evaluation_lab.rs` | `3645202da2597e5f63cf9ef96a891e9ae801e2b2` |
| `apps/desktop_flutter/lib/screens/runtime_center.dart` | `d2c7af172073d2b10c76a6416b90f011f10e95db` |
| `packages/gui_shell_ui/lib/src/runtime_resource_client.dart` | `3a8e611b94136e27f6d132e550a49bcbaf586eaf` |
| `packages/gui_shell_ui/lib/src/runtime_lifecycle_client.dart` | `d34d773ec25f21bb682577faa7688d350921f059` |
| `apps/desktop_flutter/lib/screens/observability_center.dart` | `497fe25368b9b108d62bee921d454145b712b657` |
| `apps/desktop_flutter/lib/screens/trace_inspector.dart` | `c196c8e20786d151edc4b709b71dd3b8fa88812b` |
| `apps/desktop_flutter/lib/services/observation_client.dart` | `97a28fb5aadd82c77028d74928bb7fa86a0220cc` |
| `native/rust_helper/src/broker/observation_center.rs` | `62896e497a0d78267020db6bae2ecadf704fa1fb` |
| `apps/desktop_flutter/lib/screens/notifications.dart` | `850ac48a96ccff94b64b154a4e38d608aabb5d3c` |
| `apps/desktop_flutter/lib/services/notification_client.dart` | `746c74805ef5b80febf020306daeb360ff43b4e6` |
| `native/rust_helper/src/broker/notification_center.rs` | `ca6828a33baf94abd7101781d9730b22daca4a78` |
| `apps/desktop_flutter/lib/screens/audit_viewer.dart` | `21119c9b12cf6929a09a917195158c02a76a07f5` |
| `apps/desktop_flutter/lib/screens/problems_panel.dart` | `be2b95ba68356df91d770be634e55b93dd399199` |
| `apps/desktop_flutter/lib/screens/recovery_center.dart` | `4eeda112b05af3dc3289e0f962fabda85b157bdf` |
| `apps/desktop_flutter/lib/services/shell_core_client.dart` | `f08db061bd15649ff7c2fc3fc9e37184433cbf18` |
| `apps/desktop_flutter/lib/services/global_search_index.dart` | `f71d2f6c4563b011fa3621ea3aee596bffb099f0` |
| `apps/desktop_flutter/lib/services/windows_tray_client.dart` | `cf8214ceb95fa1b24c7eed4f14b312bda80e0b15` |
| `apps/mobile_flutter/lib/services/device_link_client.dart` | `e8c49337127cb7f62fad248988b99707c5e49219` |
| `apps/mobile_flutter/lib/services/mobile_projection_client.dart` | `ba82e4ddeaa5692793237a1bc3258ce53b7aa3ed` |
| `apps/mobile_flutter/lib/screens/history.dart` | `afa685cac135d4e113b9ea3e66aa4d2c934d8674` |
| `apps/mobile_flutter/lib/screens/resource_overview.dart` | `322af6bc2935c571017dab3f03b9c7a0cea7c616` |
| `apps/mobile_flutter/lib/screens/notifications.dart` | `243c59c321cb1cbff39cc6cc993bd01d589c760f` |
| `apps/mobile_flutter/lib/screens/emergency_stop.dart` | `4d861a1ad64bcb5a73e024e07cf25b58aa3bcfc2` |
| `native/rust_helper/src/broker/device_link.rs` | `c2e1ff88ff34a5764b2b4fb1d3f7abd8727e5c51` |
| `native/rust_helper/src/broker/protocol.rs` | `9b2076d5f31ca6d77f1f87973d1b394aaf751961` |
| `native/rust_helper/src/broker/dialogue.rs` | `c3c3fddb630dd06b7e6cc1c13b39f3d4f6f6a999` |

## N10 標準OS導入・利用・更新・撤去 Source→Consumer追跡（E0差分）

照合HEADは`2c220a4bdb06fff26471393422b063c9735811cc`。製品source比較基準は`d503a4ef6daac8942abd1cb011442ce28c78ec55`で、v4文書採用・台帳作業は以下の製品sourceを変更していない。`WIRED`は確認できたコード上のProducer→Consumer接続だけを意味し、一般利用者向け配布・installed lifecycle・対象OSの合格を意味しない。本単位はE0 source mappingであり、N10 acceptance test、現在候補の実OS QA、出荷適格性は`NOT_RUN`／未成立のまま扱う。

| OS | Producer→Consumer／現物 | 配布・導入・初回利用・更新/復旧/撤去の観測 | TestClass／Evidence／N10差分 |
|---|---|---|---|
| Windows | Desktop Flutter→Windows Runner→Rust `desktop_launcher`／認証Broker。`product_install.rs`／`update_center.rs`にはper-user install/update操作がある。Export builder、portable packager、D4PKG01 product packagerも存在する。 | `stage_installed_app.ps1`はDeveloper／release-evidence用staging。Export ZIPはportable配置物、D4PKG01はupdate packageで、一般利用者向け`.exe` installerとは異なる。manual workflowはsynthetic Receipt/Manifestから未署名bundleを作るだけで、workflow内にinstalled-product起動・Owner認可・Update/Rollback/Uninstallはない。W01/G1のWindows normal Release実Taskは限定履歴であり、通常installerの一貫した初回利用証拠ではない。 | workflowは`workflow_dispatch`のbuild fixture。N10実導入は`NOT_RUN`。`windows_installer_first_run_smoke`、`windows_setup_doctor_smoke`、`windows_broker_installed_smoke`、`windows_evidence_provenance_isolation`、`windows_distribution_identity`等が未解決。 |
| Linux | Linux export builder→固定bundle／Rust `linux_product_bundle`・launcher→Broker `product_install`／`update_center`。D4PKG01 writerはLinux host上のbundle梱包を行い、APT setup package builderも別に存在する。 | 現portable `tar.gz`候補はUbuntu 24.04 LTS／x86_64のunsigned開発archive。`d4-pocket-setup` `.deb`はbootstrap UI配置用で製品本体packageではない。D4PKG01は更新payloadで、OS package manager向け製品installerと同じものではない。manual Linux workflowにはinstalled GTK/X11・Wayland等のscopeがあるが、この台帳作業では未起動。過去run #77等のlimited Xvfb/X11、run #286のUbuntu 22.04 hosted evidenceは旧commit・限定scopeで、現候補の一般配布／実desktop／Wayland／全lifecycleへ昇格しない。 | package/smoke unit sourceとmanual workflowは存在。現行N10 acceptanceは`NOT_RUN`。`linux_native_product_support` unresolved。WSL/WSLg/Xvfb/headlessはnative standard desktop acceptanceへ読み替えない。 |
| macOS | macOS Flutter Runner→Rust `macos_desktop_worker`／`macos_desktop_owner`。App bundle設定とOwner UI test sourceがある。共有ProductPackagePlatform/update pathはWindows/Linuxのみ。 | `installer/macos/README.md`はproduct installerをv1後の計画と記載。bundle IDは`com.example.guiShellDesktop`のplaceholder。manual Apple workflowは`macos-15`上のdebug build/XCTest/UI scopeで、正式`.app`配布、署名/notarization、installed Update/Rollback/Uninstallは示さない。 | UI/XCTest sourceは存在するがN10 standard distribution lifecycleは`NOT_RUN`。release OS range/arch/package/identity/最低resourceも未固定。 |
| Android | Flutter Mobile→Kotlin `MainActivity`／native Device Link service→Rust Broker transport。Gradle・native product-flow instrumentation testがある。 | manual workflowはAPI 35 Google APIs x86_64 Emulatorを起動し、debug AABをbuildしdebug APKをinstallする。`applicationId`はplaceholder、`minSdk`/`targetSdk`はFlutter既定値参照。正式署名配布、store/一般配布、更新・復旧・撤去を結ぶ候補は未成立。N01/N02でMobile AgentTask/MCP作用consumerも`BUILD_GAP`。 | Emulator/debug build・native test sourceはあるが、今回workflowは実行していない。標準API/ABI・distribution identity・最低resource・N10 lifecycleは`UNKNOWN`／`NOT_RUN`。 |
| iOS | Flutter Mobile→Swift `DeviceLinkNativeService`／Keychain／Rust Broker transport。Xcode projectとDevice Link UI test sourceがある。 | `IPHONEOS_DEPLOYMENT_TARGET=13.0`はsource設定であり製品の保証範囲確定ではない。manual Apple workflowはiPhone 16 Pro／iOS 18.5 Simulator向けdebug UI scopeを持つが、device配布IPA、store経路、署名/entitlementを含むUpdate/Repair/Uninstall実証はない。Simulator artifactとdevice配布artifactは別対象。Bundle IDはplaceholder。 | Simulator UI test sourceはあるが今回workflowは実行していない。配布用identity/署名、保証OS範囲、最低resource、installed lifecycleは`UNKNOWN`／`NOT_RUN`。N01/N02のMobile Task/MCP consumerも`BUILD_GAP`。 |

#### OS標準候補と実証範囲の更新

| OS | 現source・workflowから確認できる候補値 | 未確定値／現時点で言えないこと |
|---|---|---|
| Windows | workflow runner `windows-2025`。Portable ZIPとD4PKG01 update payload。設計代表下限8GB RAMはv4の候補条件。 | 対象Windows edition/build・arch・一般installer/package・Publisher identity・実測最低資源・現候補のinstalled lifecycle。 |
| Linux | portable archiveのUbuntu 24.04 LTS／x86_64候補。APT bootstrap packageはUbuntu 24.04／amd64。manual workflowは通常scopeでUbuntu 24.04、bwrap制約のTask scopeだけUbuntu 22.04。 | 保証distro/version/arch集合、native X11/Wayland範囲、正式product package形式、最低資源、一般配布条件。hosted/Xvfb/WSL履歴はnative GUI保証ではない。 |
| macOS | manual build runner `macos-15`、source deployment target 10.15、placeholder bundle ID。 | 発売対象OS範囲/arch、通常配布形式、署名/notarization identity、minimum resource、適法な標準QA媒体。deployment target単独でsupported範囲を固定しない。 |
| Android | Emulator image API 35／Google APIs／x86_64、source `compileSdk=37`。 | 保証API範囲、Flutterから継承されるmin/target SDKの実値、配布ABI/identity、resource下限。compileSdk/Emulator imageはsupport範囲ではない。 |
| iOS | UI test workflowのiPhone 16 Pro／iOS 18.5 Simulator、source deployment target 13.0。 | 保証iOS範囲/Simulator arch、device配布形式・署名/entitlement、minimum resource。Simulator成功をdevice artifact実行へ拡張しない。 |

#### N10に関係するSource SHA

全blob SHAは照合HEAD `2c220a4bdb06fff26471393422b063c9735811cc`上の値。

| OS/範囲 | source file | blob SHA |
|---|---|---|
| 共通 | `native/rust_helper/src/product_package.rs` | `a2deb2b6e77bc00ae8d01cd5ca306ee63baa20e5` |
| 共通 | `native/rust_helper/src/product_bootstrapper.rs` | `3b0495dfc9a1937ba64f3f99c95ae4cc739ddc69` |
| Windows | `installer/windows/README.md` | `46287d463b38b1093a29e9818c0f0bfd99465393` |
| Windows | `installer/windows/stage_installed_app.ps1` | `c6d0797299624d69b71042416400b2f454787cc7` |
| Windows | `tooling/export_windows_product.py` | `31856179881a5a5aae1ba4451732e6de6cd7f338` |
| Windows | `tooling/package_windows_portable.py` | `d6cfba6941da10fbae35228e7137d7f87f8d3889` |
| Windows | `tooling/package_windows_product.py` | `c6f9155f2457ef0d89ecb711fa94424fbb1501ee` |
| Windows | `native/rust_helper/src/broker/product_install.rs` | `c91115bdf88dd28e5f3445e7ed66127902909677` |
| Windows | `native/rust_helper/src/broker/update_center.rs` | `f6fc98155fcd476765b60d382a680c7f7c6d675e` |
| Windows | `.github/workflows/windows-manual-export-build.yml` | `f8255269ad70a63f329b6abb78f6da3d62cde960` |
| Linux | `installer/linux/README.md` | `25e80411d6ca8f96e6d70718d59c19e7dc6a58e3` |
| Linux | `tooling/export_linux_product.py` | `47fe9a3032a5b2680e3f7d39b7cd64120ac83843` |
| Linux | `tooling/package_linux_product.py` | `0d32d04ec5b43e1815b3664a323c17db79bc928a` |
| Linux | `tooling/package_linux_deb_setup.py` | `e7a057b2f3db2fca8bbe91401e25d71fb88ceca3` |
| Linux | `native/rust_helper/src/linux_product_bundle.rs` | `f71ed312c629e81914c61180baeb9723b315a69a` |
| Linux | `native/rust_helper/src/linux_desktop_launcher.rs` | `f58fb226f21093c2d1f062259d394b4f8ec022f6` |
| Linux | `tooling/tests/linux_installed_product_smoke.py` | `8020ed8d1d68d59cff6acf70c55856fdeb2125e3` |
| Linux | `tooling/tests/test_linux_installed_product_smoke.py` | `dc94f9b782d3699810bbe50230a5e9510a5739f7` |
| Linux | `tooling/tests/test_linux_product_package.py` | `3bb3b520d3c4d15b1f4d43993c8cddb0f0c08ee1` |
| Linux | `.github/workflows/linux-manual-export-build.yml` | `8d59dd1e9e54a9a4ebb16992f59a402c1ab77c67` |
| macOS | `installer/macos/README.md` | `b3f3326fa37e9977c47cb0efc7702294a6e1d33a` |
| macOS | `apps/desktop_flutter/macos/Runner/Configs/AppInfo.xcconfig` | `b278481bc9b75d1851c11fa67313a2c6cd64c1e3` |
| macOS | `native/rust_helper/src/macos_desktop_worker.rs` | `09fa26563fc671e07f23c73f7dd2a7f2cf3fed7e` |
| macOS | `native/rust_helper/src/macos_desktop_owner.rs` | `31e13a57fcde422878fae63083bd323e07f482ec` |
| macOS | `apps/desktop_flutter/macos/RunnerUITests/AdapterOwnerUITests.swift` | `eeab11b1123165a8f82da55ebbb014eab8803862` |
| macOS/iOS | `.github/workflows/apple-manual-build.yml` | `ed60235da7b35cdfaac23a9fd2d07023e1d29843` |
| Android | `apps/mobile_flutter/android/app/build.gradle.kts` | `fe96edfa0f714c2d2be509ce3bc21bec6253a714` |
| Android | `apps/mobile_flutter/android/app/src/main/kotlin/com/example/gui_shell_mobile/MainActivity.kt` | `5c0e28f0ad371f80671d26072bd1c1a134a881d9` |
| Android | `apps/mobile_flutter/android/app/src/androidTest/kotlin/com/example/gui_shell_mobile/NativeDeviceLinkProductFlowTest.kt` | `4f4a79e0bd80b1f5eb280857dec7a7d5df12258c` |
| Android | `.github/workflows/android-manual-emulator.yml` | `f0c1ecb7185c91375adfbbdbfaf49e55a2672b57` |
| iOS | `apps/mobile_flutter/ios/Runner.xcodeproj/project.pbxproj` | `73876a3b79b5ee5bd50dea0428764f1632dfcb19` |
| iOS | `apps/mobile_flutter/ios/Runner/DeviceLinkNativeService.swift` | `9d185729c439934dce0553b490f2fcd1f7b28a0d` |
| iOS | `apps/mobile_flutter/ios/RunnerUITests/DeviceLinkProductUITests.swift` | `b74c1f4a846c07eed9b6ae9244ac18b4f1030eec` |

対象test sourceの分類は、Linux package/smoke `UNIT`/hosted `LIVE_PRODUCT`、macOS/iOS `XCTest/UI`、Android instrumentation/Emulator、各manual workflow `workflow_dispatch`。本mappingでは列挙testもworkflowも未実行。過去のWindows W01/G1、Linux #77/#286等は元commit・実行媒体・scopeを保持した限定履歴であり、current N10 Release candidateへ転記しない。Linux以外の物理端末が不要であることは、標準VM/Emulator/Simulator上の実installed QAを省く意味ではない。

N10は`OPEN`。OS別の実consumer sourceと一部のpackage/lifecycle部品は確認できたが、通常利用者向け配布から初回設定・基本サービス・実AI仕事・Update/Repair/Rollback/Uninstallまでを同一のcurrent candidateで通した証拠が全5 OSに揃っていない。未知の標準version/arch/最低資源を創作せず、個々のcandidate値と保証対象を分けて保持する。N10 source mappingはE0全数照合、E1全体BUILD、BUILD_COVERED、QE_QC/QA、Releaseを閉じない。

## N11 Evidence/QC/QA/Release Source→Consumer追跡（E0差分）

照合HEADは`d32d053c702f834201784428ad597f321977580c`、製品source比較基準は`d503a4ef6daac8942abd1cb011442ce28c78ec55`。N10/N11台帳更新はrelease validator・runtime・schemaを変更していない。ここではvalidatorの実Consumerと証拠範囲を記録する。静的条件・fixture/conformance passを実Release candidateのLIVE_RUNTIME/EXTERNAL_EVIDENCEへ読み替えない。

| 面 | Producer→Consumer／現物 | 現在成立していること | N11差分／証拠限界 |
|---|---|---|---|
| 要求受入ledger | `docs/QE_2026-11-13/進捗.md`のN01〜N11/W00〜W08行→`tooling/release_gate_check.py::workplan_acceptance_errors`。 | strict releaseはN01〜N11を全て`CLOSED`、W01〜W08を`CLOSED`、W00は`SCOPED_PASS`/`CLOSED`とし、N行はW phase dependencyおよび既存local evidence参照を要求する。 | 現行N01〜N11/W02〜W08はOPEN。現在のN11や旧ID crosswalk全数接続、実candidate evidenceは未成立。Strict gateの条件定義は製品完成の証拠ではない。 |
| Release Registry | `release_blockers.registry.json`→`release_gate_check.py::registry_errors`／`unresolved_active_blockers`／`effective_release_blocker`→strict release/Windows Technical Complete判定。 | 18 IDの一意性、必須field、classification/cause/scope、manualまたは`windows_release_evidence`のstatus source、active状態を検査。動的Windows resultは正式evidence validatorから再評価する。現状態は16 unresolved/2 resolved、`release_ready=false`。 | 旧resolved 2件は履歴statusとして保持。他16件は未解決。Registry自体のvalidityは個々の要求の完成証拠ではなく、Owner GOや正式配布資格を生成しない。 |
| Windows installed evidence | `release_evidence/windows_installed_smoke.json`→`tooling/windows_release_evidence.py`→4件のWindows evidence-sourced blockers。Validatorはsource commit、artifact hash、profile/runtime path、process/UI、Setup Doctor、Broker/Audit、bundle provenance等を確認する。 | Windows向けinstalled evidenceをfield単位で分類し、`LIVE_RUNTIME`/`EXTERNAL_EVIDENCE`とstatic `CONFIG`を区別するconsumerはある。 | 現release_evidenceには`.gitkeep`だけでinstalled evidence fileなし。4 Windows blockerは未解決。Validatorのunit/conformance coverageは実installed acceptanceではない。 |
| Evidence Bundle | `tooling/evidence_bundle.py::build_evidence_bundle`→Windows validator／Release Runtime Assertions／Release Smoke／Shell Snapshot／Setup Doctor。 | 自動生成projectionは`classification=development_evidence`、`release_ready=false`を固定し、authority境界とblockerを含める。 | 5OSのartifact/evidenceを統合したRelease dossierではなく、all-N/P/C/Phase/BUF/Q/FQを結合するN11 acceptance validatorでもない。現candidate evidenceの代用にしない。 |
| Source Manifest | Git追跡file一覧→`tooling/manifest.py` SHA-256 manifest→`MANIFEST.sha256.json`。 | tracked source fileの存在・hash・重複・EOL/coverageを機械検査。現在1415 filesを記録。 | 内容が現Sourceと同一である証拠に限る。producer→consumer接続、runtime provenance、意味契約、OS実行を証明しない。 |
| Conformance | `tooling/conformance_tests/run_conformance_skeleton.py`内のregistry strict-gate、N/W evidence、release-doc sync、evidence bundle non-authoritative、Windows validator、manifest等のnamed checks。 | Registry/validatorとrelease-facing文書の形・fixture応答を検査する実行入口がある。 | test functionが存在することと、現在のRelease evidenceがあることは別。本N11 source mappingでは個別機能試験は`NOT_RUN`。全skeleton実行結果は検査時の対象commitとfixture範囲に限定する。 |
| Final QA / C32 Audit | `docs/FINAL_QA_QUEUE.md`のQ0〜Q7・FQ履歴、`tooling/final_development_audit.py`→`docs/specs/final-development-audit.json`／rev1 receipt/progress。 | Final QA Queueは`DEFERRED`。Q0/Q1の記録は旧candidate `ab746d4b...`の履歴。C32 audit toolは旧C0〜C31対応表の必須欄・参照path/classificationを静的検査する。 | 旧証拠をv4 current candidateへ転記しない。C32 static tableはC0〜C34全体、N/P/Phase/K/BUF/Q/FQ/OSの全consumerを結ぶものではない。正式Q0〜Q7はE0〜E3/BUILD_COVERED後。 |

N11関連source testは`test_release_blocker_registry_controls_strict_release`、`test_strict_release_requires_current_n_w_consumer_evidence`、`test_release_facing_docs_sync_release_blockers_to_registry`、`test_evidence_bundle_is_development_classified_and_non_authoritative`、`test_manifest_integrity_tooling_exists`、Windows Release Evidence validator tests等。区分は主に`CONFIG`/`FIXTURE`/`INTERNAL_STATE`で、対象のreal installed flowを実行する`LIVE_RUNTIME`ではない。`release_gate_check.py --strict-release`、Windows evidence generation、Q0〜Q7、全OS candidate QAは本N11 mappingでは`NOT_RUN`。

### N11 Source SHA

blob SHAは照合HEAD `d32d053c702f834201784428ad597f321977580c`上の値。

| source file | blob SHA |
|---|---|
| `release_blockers.registry.json` | `244adc04a01e7b406c4ebcdeb829f7077d4aa35f` |
| `tooling/release_gate_check.py` | `22a8d7bfe9adb259f44ce8501fb215652e1d58c6` |
| `tooling/manifest.py` | `aa4525ff20293cb8641e5598a2439561a3bfbd3b` |
| `tooling/evidence_bundle.py` | `68dead851236b0f1b69899151c37371e593ff3f9` |
| `tooling/windows_release_evidence.py` | `6f7957c6193c86c7bc15165e743f11651464f29c` |
| `tooling/release_runtime_assertions.py` | `0a0a351fe17151ad86517bd0a965358f40c61490` |
| `tooling/release_smoke.py` | `e9f38cea998a31bbf9272880dbf4965de5eb0113` |
| `tooling/final_development_audit.py` | `913c81bc22f71bc5bb3447fa4f4bd266326932ec` |
| `tooling/conformance_tests/run_conformance_skeleton.py` | `c6f31d8cb561b48513c6784e54c85969c9cb953e` |
| `docs/FINAL_QA_QUEUE.md` | `7b332cccab70bb68aa1ff4f8e23acc0e816d8282` |
| `docs/RELEASE_VALIDATION.md` | `b4b0462e628ad8176264473879dedb1f6201da7c` |
| `docs/specs/final-development-audit.md` | `cc3734d7647dbbbbd27bad49153f775bfc980f3f` |
| `docs/specs/final-development-audit.json` | `8126944c55339a3be272af5ba183a9bc2e3517dc` |

N11は`OPEN`。ValidatorとRelease Gateは一部のRegistry/Windows evidence/manifest/conformanceを実際に消費するが、全製品要求・旧ID・OS Artifact・結果/停止/Recovery・証拠class・所有者判断を同一candidateで束ねる統合consumerは未成立。18件のstatus、Q0〜Q7の履歴、`release_ready=false`は変更しない。N11 mappingはE0完了、BUILD_COVERED、QC/QA、Releaseを意味しない。

## P0〜P13 旧製品要求→v4 Consumer crosswalk（E0差分）

照合HEADは`9c6dfadc5d8a63b8e96f79ddd340de05c2963fef`、製品source比較基準は`d503a4ef6daac8942abd1cb011442ce28c78ec55`。旧P statusは[REV5製品進捗](../REV5_PRODUCT_PROGRESS.md)の当時のProduct Build境界で保持し、v4のN/K/Workstream statusへ昇格しない。v4仕様S05のP別直接mappingと、旧Phase/C対応は別の軸として記録する。Consumer sourceとblob SHAは、表が指すN01〜N10追跡節および該当Source SHA表を参照する。今回の作業は文書source mappingのみで、製品testは`NOT_RUN`。

| 旧ID | 旧要求／履歴status | v4 scope対応 | 現行Producer→Consumer／source参照 | 旧証拠classとv4残余 |
|---|---|---|---|---|
| P0 | Foundation。Rust Broker、Authority、Permission、Approval、Audit、Runtime/Workspace登録、Desktop shell/Launcher/IPCを既存基盤として再利用。単独の新機能CLOSED statusではない。 | v4仕様S05がN01/N09/N11、K01へ対応付ける。A〜Jに共通するが、独立Acceptance IDを新設しない。 | 登録・Broker実行・Task/resultの接続はN01、操作/観測/回復はN09、証拠/Gate接続はN11の現行Source→Consumer節とSHA表に記録。 | 旧基盤の存在をv4統合合格へ移さない。v4のN01/N09/N11と各OSの必要Consumerは未完部分を各IDに保持。 |
| P1 | Agent Task。rev5進捗は「製品開発上は完了」と記録。成立済み正常Task経路を再利用し、追加crash matrixを次機能の開始条件にしない。 | v4仕様S05がN01/N02、K01へ対応付ける。N02は別契約のままP1 mappingにも接続し、両IDのstatusを混ぜない。 | Agent Center→AgentTaskClient→認証Broker→登録Adapter/Codex CLI→Task state/result consumer。N01/N02 Source→Consumer節にsource hashとBroker拒否/停止/Recovery境界を記録。 | Windows W01/G1のCodex CLI限定`LIVE_PRODUCT`履歴は限定のまま。v4 N01/N02は`OPEN`、Mobile Task/MCPは`BUILD_GAP`、五OS標準installed保証は未成立。 |
| P2 | Multi-Agent Compare。旧rev5でProduct Build `CLOSED`。同一Codex/providerのAgent A/Bを独立Session/Workspace/Taskで比較する機能範囲。 | N04、K02、A/C/F。Phase 7がC1/C30の範囲へ接続する（[Phase mapping](../D4_POCKET_PHASE_MAPPING.md)）。 | Agent CenterのA/B選択・compare coordinator→Brokerの独立AgentTask要求→Task/result/hash/Audit投影。source詳細はN03/N04節とSHA表。 | 旧受入にWidget/service `FIXTURE`条件を含む。v4 N04/W03は`OPEN`。同一installed candidateの同時実行、独立Workspace/result、selected apply、片側failure/cancel isolationの結合証拠は未成立。 |
| P3 | Agent Handoff。旧rev5でProduct Build `CLOSED`。Task context、承認済result/artifact/diff/test申告を別Agentへ渡し、Authorityを継承しない。 | N04、K02、A/C/F。Phase 8がC30の範囲へ接続する（[Phase mapping](../D4_POCKET_PHASE_MAPPING.md)）。 | Handoff package producer→別Runtime/Session/Workspace向け新AgentTask request→Broker preflight→fresh Permission/Approval。N03/N04節にschema/source・負例・制限を記録。 | 旧service/Widget試験は主に`FIXTURE`。v4 N04/W03は`OPEN`。installed同一候補のArtifact受渡し、target再認可、結果/Auditまでの受入は未成立。 |
| P4 | Provider / Model Center。旧rev5でProduct Build `CLOSED`。Codex CLI一Provider選択、model identifier引渡し、既存CLI credential委譲、capability/health unknown表示。 | v4仕様S05の直接対応はN03/K06。Phase 15は旧phase map上のD4追加範囲で、rev1 C工程との同一要求を主張しない。N05は隣接するCapability/Compose要求でありP4の直接mappingではない。 | Agent/provider登録とmodel selection→native Owner確認→Broker/Adapter→Codex CLI引数。関連するCapability/ComposeのsourceはN05節に別途記録。 | 旧試験は実Provider呼出しを含まず、fake Codex CLI等の`FIXTURE`/source evidence。v4 N03/W03は`OPEN`。二つ目の独立実Runtime Task、実Provider到達、実能力と宣言の一致、installed OS経路は未成立。 |
| P5 | Workspace / History / Evaluation。旧rev5でProduct Build `CLOSED`。Workspace Inspector、Task/Dialogue履歴、再実行/分岐、結果/diff、Regression/Evaluation Caseが範囲。 | v4仕様S05の直接対応はN09/N11。関連能力はK03/K05。N09全体はK04/K10/K13も共有し、P5だけに帰属させない。Phase mapping上の関連C工程はPhase 6、9〜14（C1〜C6範囲）。施工線C。 | Workspace/History/Evaluation/Resource/Observation/Notification UI→各Broker consumer。詳細なTask ID binding、表示、Audit/Recoveryの現source・hashはN09節、Evidence/Gate consumerはN11節。 | 旧P5 focused testsはProduct Build証拠。現sourceではWorkspace差分やHistory/Evaluation/Resource/Traceが同一AgentTaskへ統合されず、Audit/Problems/Recovery/Searchはsnapshot由来。v4 N09/N11は`OPEN`、E0 mappingは製品再受入ではない。 |
| P6 | Credential / MCP。旧rev5でProduct Build `CLOSED`。Owner credential管理、Provider/MCP用途束縛、stdio discovery/Tool作用、秘密非露出。 | v4仕様S05の直接対応はN02/N05。関連能力はK07/K08。Phase 16〜19（C7〜C9範囲）。施工線A/D/F。 | Agent TaskのMCP提案→current Catalog/一回Permission/native Owner/Audit→実Tool→result-bound後続TaskはN02節。Credential metadata/secret resolutionとCompose/MCP consumerはN05節。 | 旧MCP `cmd.exe`/偽Codex CLI・合成Credential試験を実Provider成功としない。実Codex/MxC loopbackはAPI要求前failure履歴を持ち、Provider状態unknown。v4 N02/N05とW02/W04は`OPEN`、五OS installed evidenceは未成立。 |
| P7 | A2A / Multi-Host / Adapter管理。旧rev5でProduct Build `CLOSED`。A2A、Host metadata/capability、接続性、degraded mode、local/remote Runtime区別。 | v4仕様S05の直接対応はN03/N04。関連能力はK11/K12。Phase 24〜27（C15〜C19範囲）。施工線B/F。 | Runtime/AdapterとAgent Center/A2A UI→Broker Runtime/Host/A2A consumer。実Runtime capability・A2A action・Host境界のsourceとhashはN03/N04節。 | 旧P7の決定的fixture/限定接続をinstalled external-agent保証へ昇格しない。現v4 N03/N04・W03は`OPEN`。二つ目の実Runtime Task、外部Agent action、Host間Workspace/Authority隔離、Mac/Mobileの同等作用は未成立またはunsupported。 |
| P8 | GUI-Shell Compose。旧rev5で製品構築受入れ`CLOSED`。Capability/Dependency選択、Manifest/preview、AI edit proposalと制御適用を区別。 | v4仕様S05の直接対応はN05/N07。関連能力はK14。Phase 29〜31はD4追加範囲で、同等のrev1 C工程を主張しない。施工線D。 | Catalog/Compose UI→Broker compatibility check/Manifest→MCP Tool consumerはN05節。AI proposal→別Workspaceの審査Task→controlled apply/rollback候補はN07節。 | 現ConsumerはManifestの独立Appによる完全消費を示さず、AI Editはproposal/Task準備が中心でBroker-controlled apply/rollbackに`BUILD_GAP`。旧Product Build statusのまま、v4 N05/N07・W04/W05は`OPEN`。 |
| P9 | Standalone Export。旧rev5で製品構築受入れ`CLOSED`。構成から新identity/Manifest/選択moduleを持つ独立Appを出力する。 | v4仕様S05の直接対応はN06。関連能力はK15。Phase 32はD4追加範囲。Phase 33のModule pruning (P10)と分離する。施工線E。 | Export UI→Broker receipt/Manifest→Windows/Linux Developer Exporter→Launcher。現source/hashとManifest consumption/pruning limitはN06節。 | 旧Exporter/Product Build証拠は独立installed製品の実動作ではない。runtime manifest consumption、normal installer/installed runは未成立、`standalone_app_verified=false`。v4 N06/W05は`OPEN`。P10のbinary/resource除去も本IDへ統合しない。 |
| P10 | Module Selection / Pruning。旧rev5で機能上の除外範囲を`CLOSED`。選択dependency closure、compile-time UI除外、必須画面保持。 | v4仕様S05の直接対応はN06、関連能力K15。Phase 33のD4追加範囲で、同等のrev1 C工程なし。施工線E/F。 | Export Module plan→build define/receipt→Flutter UIの除外経路。Exporter/Launcherへの現source接続とhashはN06節。 | 旧plan/Widgetは`CONFIG`/`FIXTURE`。AOT/binary実除去、Rust/third-party dependency pruning、size/resourceの意味保証は未成立。`binary_pruning_verified=false`、FQ-MODULE-PRUNING／`rev2_module_pruning_binary_and_measurement`を保持。v4 N06/W05は`OPEN`。 |
| P11 | Windows Productization。旧rev5で製品機能受入れ`CLOSED`。P9 portable ZIPをbootstrapに使うBroker-owned Install/Update/Repair/Rollback/Uninstall導線。 | v4仕様S05の直接対応はN10、関連能力K16。Phase 34はC33/C34の配布範囲と関連。施工線G/F。 | Windows package/portable→bootstrap/Update Center→Broker product install/update→Launcher/Setup Doctor。現候補source・package役割・workflow・hashはN10節。 | 旧acceptanceはMSI/Setup.exeやsigned release artifact作成を主張しない。v4 N10は`OPEN`で、一般配布・同一candidateのinstalled lifecycle・署名/Publisher identityは未成立。`rev2_desktop_product_distribution`と`windows_distribution_identity`を保持。 |
| P12 | 製品全体統合。REV5進捗ではWindows側`FEATURE COMPLETE`。正式release/Owner GOと分離された製品機能統合の履歴。 | v4仕様S05の直接対応はN01/N09/N10、関連能力K19。Phase 39〜45はC27〜C34のQC/回帰/文書/出荷範囲に関連。単一施工線へ閉じない。 | Windows product entry→Task/result/operation UI→OS product lifecycle。実TaskはN01、operation/observation/recoveryはN09、installed OS distribution/lifecycleはN10の現ConsumerとSHA節へ分割記録。 | `FEATURE COMPLETE`を正式Release/QAへ昇格しない。v4 N01/N09/N10は`OPEN`、Q0〜Q7は`DEFERRED`、`release_ready=false`。統合後の全候補・全OS acceptanceは未成立。 |
| P13 | Mobile / Non-Windows Product Build。REV5履歴は2026-10-09に有限Product Build acceptanceを閉鎖したと要約する。旧Acceptanceの詳細evidence identity/classは本照合資料から`UNKNOWN`。 | v4仕様S05の直接対応はN10、関連能力K17。Phase 35/36はC24〜C26範囲に関連。施工線H/I。Linux native必須性は別途現行AGENTS/v4正本に従う。 | Linux/macOS Desktop、Android/iOS Mobile runner/native transport→Rust Broker／OS capability→installed product lifecycle。OS別source、package、workflow候補とSHAはN10節。 | 旧P13閉鎖を現行五OS標準installed QAへ移さない。Linux nativeは完全対応必須で、WSL/Xvfb/Headlessだけでは成立しない。Android/iOS実機を前提にせずEmulator/Simulator等を用いる。N10は`OPEN`、`linux_native_product_support`、`rev2_mobile_distribution`、`rev2_mobile_device_evidence`を保持。 |

P0〜P13全14 IDの旧要求範囲・限定statusをv4 N/K/Consumerへ接続した。これは旧statusの書換え、v4 acceptance、N01〜N11のCLOSED、E0完了のいずれも意味しない。C0〜C34、Phase0〜45、K01〜K20のSource→Consumer/scope mappingを記録した。P0〜P7は本節後段で全E0-04 fieldを個別補完し、P8〜P13とN11 Evidence/Blocker/OS Artifact行結合は継続する。P0/P1/P4の直接対応はv4仕様S05の明示mappingへ合わせて訂正し、旧statusは変更していない。P13の詳細evidence identity/classは`UNKNOWN`として保持する。

### P0〜P3 E0-04項目の個別補完（2026-11-13、ソースと利用先の照合）

以下は現行v4 S05の直接mappingに対するE0照合である。旧rev5原本3点は`D4-AUD-F07`で受領SHA-256と全文一致が確認された資料を原文参照とし、原本要求と現行S05/N Consumerを分けて記す。コード・SchemaのGit blob SHAは本表の列挙値を現HEAD `4d795f00858496bc038def2eb974a59eb325259d`で照合した。source/testの存在や過去statusは今回の製品Acceptanceではない。

#### P0 — 既存基盤の再利用

| E0-04項目 | 記録値 |
|---|---|
| 現行ID | 旧製品要求`P0`。v4仕様S05の直接対応先は`N01/N09/N11`。 |
| 旧ID | `P0`。単独の新機能Acceptance IDではなく、Agent Taskと製品操作の共有基盤要求。 |
| 原文参照 | rev5仕様書 §§0–3（SHA-256 `c0b78cfba1fb9c5c5de1947d01b47d0898f01b24de1aa8e1b0a0bd89e35c0877`）、`D4-AUD-F07` §要求差分。原文同一性は同記録の`VERIFIED`履歴。現行対応はv4仕様S05。 |
| 対象 | Rust `Broker`、`Authority`、`Permission`/`Approval`/`Audit`、`Runtime`/`Workspace`登録、Desktop shell/launcher/IPCという既存基盤。 |
| 目的 | 基盤を再実装せず、製品機能が現行`Broker`統治経路を共有し、UI/Adapterに`Authority`を持たせない。 |
| 生成側 | Desktop Flutter起動・Agent Center・Workspace/History/Runtime/Observation画面、native LauncherからのBroker要求。 |
| 実際の受け取り先 | native Launcher→認証Broker IPC→Runtime/Workspace registry・Broker operation→Task/Workspace/Audit consumer。画面はBroker応答を表示する。N01/N09/N11追跡節を参照。 |
| 契約とデータ | P0全体を表す単一schemaはない。Runtime/Workspace登録、Broker operation、AgentTask、Permission/Approval、Audit、Workspace検査/履歴の個別契約をN01/N09/N11で追跡する。 |
| 正常時の作用 | 登録済みRuntime/WorkspaceをBrokerが管理し、許可された要求を既存IPCで処理してTask/operation state・Auditを既存consumerへ返す。 |
| 否定時の作用 | 不正・未登録・unsupported要求、Authority/Permission/Approval不足、stale/replay要求はBrokerが拒否する。Flutterからfilesystem/process/credential/networkへ直接作用させない。 |
| 停止と回復 | Brokerの拒否・失敗をUI成功へ読み替えずAudit/Task stateへ返す。AgentTask scratch/restart recoveryはN01の範囲。基盤全体を束ねる独立P0回復契約は`UNKNOWN`。 |
| 標準対象OSの候補 | v4対象のWindows/macOS/Linux native/Android/iOS。Linux nativeは必須。候補はN10表、保証version/arch/GUI範囲は`UNKNOWN`。MobileのTask実行consumerは現sourceで`BUILD_GAP`。 |
| 配布 | N10記載のOS別候補を参照。一般配布artifact・Publisher identity・installed lifecycleは未確定。portable/debug/hosted候補を通常製品配布へ昇格しない。 |
| 既存実装 | Launcher/Broker/Runtime/Workspace/Audit sourceとDesktop UI consumerは存在し接続候補がある。N01/N09 source traceは限定`WIRED`。W01はWindows限定の過去履歴で、本P0統合の受入れではない。 |
| 実装不足 | 現行candidateで共有Broker基盤から全製品機能・五OS標準installed artifactまでを結ぶ受入れは未成立。Mobile AgentTask consumer欠落、同一candidateのOS別起動/権限/配布結合、N11全要求証拠結合が残る。 |
| ソース識別SHA | 現HEADのblob SHA: `apps/desktop_flutter/lib/main.dart`=`6109c821ab5dc809224bec5234c4c59db1d5f988`; `apps/desktop_flutter/lib/screens/agent_center.dart`=`2472faeb31a64d624423c30a0a30d0f153d2e2a3`; `apps/desktop_flutter/lib/screens/workspace_inspector.dart`=`a02c95ec5062583f9c37523fbaf604a8d2da107e`; `native/rust_helper/src/desktop_launcher.rs`=`6a9f335de07fa474ca10043aea63ad6e988b572d`; `native/rust_helper/src/broker/runtime_registry.rs`=`9ff28610862586a599848d75f9e891a6c6cadf2f`; `native/rust_helper/src/broker/protocol.rs`=`9b2076d5f31ca6d77f1f87973d1b394aaf751961`; `native/rust_helper/src/broker/audit.rs`=`eb1dcf5018d95c7fdf6171aadd4bca9554d1996f`; `tooling/release_gate_check.py`=`22a8d7bfe9adb259f44ce8501fb215652e1d58c6`; `tooling/conformance_tests/run_conformance_skeleton.py`=`c6f31d8cb561b48513c6784e54c85969c9cb953e`; `release_blockers.registry.json`=`244adc04a01e7b406c4ebcdeb829f7077d4aa35f`. |
| 試験区分 | 既存sourceに`UNIT`/`FIXTURE`/`WIDGET`/`CONFIG`と過去の限定`LIVE_PRODUCT`がある。本source/consumer照合では製品試験を実行していない（`NOT_RUN`）。 |
| 証拠ID | `NONE` — 新しいP0受入れ証拠IDは発行していない。過去のW01/N11参照は元の範囲を維持する。 |
| 阻止要件 | P0専用のRegistry IDはない。関連する未解決の横断項目は`comprehensive_extension_rev1_completion`、`windows_broker_installed_smoke`、`rev2_flutter_broker_channel_boundary`、`linux_native_product_support`、`rev2_mobile_flutter_native_device_link_boundary`。Registryの状態を維持し、P0への直接割当ては`UNKNOWN`。 |
| 状態 | 旧P0は共有基盤の再利用要求であり、独立した`CLOSED`条件ではない。現行v4 `N01/N09/N11=OPEN`、`E0=OPEN`。 |
| 残件 | P0の各要件と現行N11証拠/阻止要件、OS artifactの対応付けが残る。本記録はConsumer照合でありE3/E4/E5の受入れではない。 |

#### P1 — エージェントの課題実行

| E0-04項目 | 記録値 |
|---|---|
| 現行ID | 旧製品要求`P1`。v4仕様S05の直接対応先は`N01/N02`、製品能力は`K01`。 |
| 旧ID | `P1`。旧進捗はAgent Taskの製品開発成立を記録し、正常経路を後続機能の基盤とする。 |
| 原文参照 | rev5仕様書 §§10–11（SHA-256 `c0b78cfba1fb9c5c5de1947d01b47d0898f01b24de1aa8e1b0a0bd89e35c0877`）、`D4-AUD-F07` §要求差分。現行対応はv4仕様S05とN01/N02。 |
| 対象 | Agent Task要求、独立認可、Runtime Adapter実行、Workspace作用、Task state/result/Audit。 |
| 目的 | Ownerが開始したTaskをBroker境界内で実行し、結果・失敗・停止状態を同じTask identityへ返す。 |
| 生成側 | Desktop Agent CenterのTask inspect/start/result/cancel要求と`AgentTaskClient`/execution coordinator。 |
| 実際の受け取り先 | `BrokerTransport`→Rust Broker `AgentTask実行`→登録Adapter/Codex CLI→Workspace/Task state/result→Coordinator/Agent Center。N02は同Task内のMCP proposalからBroker Tool作用、結果を使う新Taskへ接続する別契約。 |
| 契約とデータ | `agent_task_request`, `agent_task`, `agent_task_workspace_permission`, `agent_task_owner_approval`, `agent_task_content_exposure`; Runtime/Session/Workspace/Task ID・本文/条件hash・Result/Audit参照。 |
| 正常時の作用 | 現在のRuntime/Workspace/capabilityを事前照合し、Workspace Permissionと別Owner Approvalを一回消費してCodex CLI Taskを実行、結果hash/state/Auditを表示する。N02 Tool follow-upは現在Catalog/native Owner・一回Permission/Audit・新TaskのPermission/Approvalを経る。 |
| 否定時の作用 | 未登録/unsupported Runtime、Workspace不一致、Permission/Approval不足/stale/replay、条件hash不一致、MCP catalog/tool/credential mismatchをBrokerが拒否する。Mobile Task/MCP operationはunsupported/BUILD_GAPとして扱う。 |
| 停止と回復 | cancel/deadline/crash時にTask process/scratchを停止し、interrupted Taskを`suspended`で回復して旧Permission/Approvalを再利用しないsourceがある。MCP結果不明時は隔離して再送しない。Broker再起動後にvolatile result contextが失われた場合はfail-closedする。 |
| 標準対象OSの候補 | Windows/macOS/Linux native/Android/iOSの五対象。Desktop Task sourceはWindows/Linux/macOS系Rust、Mobile Task受取先はない。標準version/arch/GUIはN10参照・`UNKNOWN`。 |
| 配布 | OS別配布候補はN10表。Windows/Linux候補履歴はportable/bootstrap/hostedに限定。macOS通常配布・Android/iOS store/device package/installed lifecycleは未成立。 |
| 既存実装 | Desktop AgentTask/MCP sourceは`WIRED`。W01/G1のWindows normal Release Codex CLI completed TaskとLinux #286 hosted Xvfb/X11/local no-auth fixtureは履歴範囲限定。MCP結果依存follow-up W02/G2は`OPEN`。 |
| 実装不足 | 同一v4 candidateの一般Provider Task→結果/Audit/recovery、MCP Tool→同Agent follow-up、五OS installed pathは未成立。Mobile AgentTask/MCP consumerは`BUILD_GAP`。 |
| ソース識別SHA | 現HEADのGit blob識別値: `apps/desktop_flutter/lib/screens/agent_center.dart`=`2472faeb31a64d624423c30a0a30d0f153d2e2a3`; `apps/desktop_flutter/lib/services/agent_task_client.dart`=`f54cefac29eba3dcf383bba19b4f64ee682e6217`; `apps/desktop_flutter/lib/services/agent_task_execution_coordinator.dart`=`5d8f48d0557bdcb1af838239abf9651893bce7f1`; `packages/agent_orchestration/lib/agent_task_client.dart`=`a31fbb9f980a0039f809fd763c91f055fe9d02d0`; `packages/agent_orchestration/lib/agent_task_execution_coordinator.dart`=`feb9aacb7ed77e6f4e76e9d9371290decca9cf7b`; `native/rust_helper/src/broker/protocol.rs`=`9b2076d5f31ca6d77f1f87973d1b394aaf751961`; `native/rust_helper/src/broker/dialogue.rs`=`c3c3fddb630dd06b7e6cc1c13b39f3d4f6f6a999`; `native/rust_helper/src/adapters/codex_cli.rs`=`e5c9d1581ae9a1c0a3ae7fcfa626fdca6fea6d28`; `native/rust_helper/src/broker/mcp_center.rs`=`8f9ab897bdc625e448241ebd9b46f995cd1f22f6`; `specs/agent_task_request.schema.json`=`cb25bb3684656bfd670ea5832bbb7fc5988845b6`; `specs/agent_task.schema.json`=`a552389072088d6cd42b3588fefda332be4ae6b6`; `specs/agent_task_workspace_permission.schema.json`=`01be05df95431e38ca5dc3ec1b5f04416b506fb2`; `specs/agent_task_owner_approval.schema.json`=`d190c9a8cc3d715eb199459c37c14e5de0ad6ad8`; `specs/agent_task_content_exposure.schema.json`=`a85b99c0213d18daaab3d5d863adcd3661f379a0`. |
| 試験区分 | 既存試験sourceは`UNIT`/`FIXTURE`/`WIDGET`、過去の限定`LIVE_PRODUCT`（W01/G1）。N02 W02/G2は`OPEN`。本v4照合での試験/製品受入れは`NOT_RUN`。 |
| 証拠ID | `NONE`。過去のW01/G1とLinux run #286は元の候補/範囲を保ち、五OS証拠とは扱わない。 |
| 阻止要件 | P1専用のRegistry IDはない。関連する未解決項目は`comprehensive_extension_rev1_completion`、`windows_broker_installed_smoke`、`rev2_flutter_broker_channel_boundary`、`linux_native_product_support`、`rev2_mobile_flutter_native_device_link_boundary`。P1への直接割当ては`UNKNOWN`で、Registry状態は変更しない。 |
| 状態 | 旧進捗の製品開発完了記録と限定正常経路を保持する。現行`N01/N02=OPEN`、Mobile Task/MCP=`BUILD_GAP`、`E0=OPEN`。 |
| 残件 | 五OSの通常配布受入れ、現候補のW02結果依存MCP完了証拠、Mobile Task受取先がない。 |

#### P2 — 複数Agent比較

| E0-04項目 | 記録値 |
|---|---|
| 現行ID | 旧製品要求`P2`。v4 S05の直接対応先は`N04`、製品能力は`K02`。 |
| 旧ID | `P2`。旧rev5の`Product Build`受入れは`CLOSED`。 |
| 原文参照 | rev5仕様書 §12、工程表 §§5–6、指示書 §§7–10（原本仕様SHA-256 `c0b78cfba1fb9c5c5de1947d01b47d0898f01b24de1aa8e1b0a0bd89e35c0877`）、`D4-AUD-F07` §要求差分。現行対応はv4 S05/N04。 |
| 対象 | 同じTaskを独立Agent A/Bへ渡す比較、結果差分、選択と選択結果の適用。 |
| 目的 | AgentごとにSession/Workspace/Task/resultを隔離し、比較と選択結果の適用をBroker認可・Auditの下で行う。 |
| 生成側 | Agent CenterのCompare操作→Session/Workspace事前検査→`agent_comparison` coordinator。 |
| 実際の受け取り先 | coordinatorは独立Session/Workspaceごとに二つのBroker Task要求を開始し、Task ID別にstate/resultを読む。選択結果は直接書かず、新しい第三のtarget Taskへ渡す。 |
| 契約とデータ | `agent_comparison.schema.json`、AgentTask request/result/hash、Session/Workspace ID、各TaskのPermission/Approval、selected result binding。Task test/resourceは実測値とAgent申告を区別する。 |
| 正常時の作用 | A/Bを別々に認可して同じTaskを開始し、結果/diff/test申告/duration/failure/resource/Auditを分けて表示する。Owner/Developerが選んだ一方だけを、新しいtarget Taskとして再評価・適用する。 |
| 否定時の作用 | Session/Workspace重複、source/result/hash/target変更、不正authority metadataを拒否する。片側の開始/照会失敗は他方へ伝播せず、曖昧なstartは再送しない。Test結果を独立実行証拠と誤認しない。 |
| 停止と回復 | cancelはSession単位。各TaskはBrokerの通常cancel/failure処理に従う。選択/apply時はsource/result/targetを再照合し、不一致なら停止する。Compare全体の導入済み製品でのcrash/recoveryは未確認。 |
| 標準対象OSの候補 | 五OS対象の製品機能母集団を適用する。現行Compare UI/Coordinatorの受け取り先はDesktop Agent Center。Mobile Compare actionの受け取り先は`UNKNOWN`。Linux nativeは必須。保証version/archはN10表の`UNKNOWN`。 |
| 配布 | N10のOS別候補とArtifact範囲を参照する。Compareを実行する同一installed product artifact/identityは未確定。 |
| 既存実装 | Compare UI/coordinator/Broker Task要求/schema sourceは`WIRED`。旧`Product Build`受入れstatusは`CLOSED`だが、過去testは主に`FIXTURE`/`WIDGET`で導入済み製品のCompareを証明しない。 |
| 実装不足 | 同一installed candidateでの同時A/B実行、実Workspace/scratchの混入防止、result/diff/Audit、選択結果の適用、片側failure/cancelの分離を連続して示す証拠がない。 |
| ソース識別SHA | 現HEADのGit blob識別値: `apps/desktop_flutter/lib/screens/agent_center.dart`=`2472faeb31a64d624423c30a0a30d0f153d2e2a3`; `apps/desktop_flutter/lib/services/agent_coordination.dart`=`bdc13464f0e42b740a2bca3b7e8828a5019056d0`; `packages/agent_orchestration/lib/agent_comparison_coordinator.dart`=`6b5deaacb500a23e6bfdc37649d61ebc0d885127`; `specs/agent_comparison.schema.json`=`d2993ce1145674214fb2d80d696fc2c161cfbffe`; `apps/desktop_flutter/test/agent_coordination_test.dart`=`6783e87a492d8436a855a7088b7cece38d122bd5`. |
| 試験区分 | 既存Rust試験は`UNIT`、Flutter試験は`FIXTURE`/`WIDGET`。`agent_coordination_test.dart`は`LIVE_PRODUCT`ではない。現candidateの製品受入れは`NOT_RUN`。 |
| 証拠ID | `NONE`。旧P2 `Product Build`の`CLOSED`と証拠区分は旧条件のまま保持し、v4 E4証拠は発行していない。 |
| 阻止要件 | P2専用のRegistry IDはない。広範な未解決項目`comprehensive_extension_rev1_completion`が関連し、`rev2_flutter_broker_channel_boundary`と`linux_native_product_support`は横断的な基盤条件。P2への直接帰属は`UNKNOWN`。Registry状態を維持する。 |
| 状態 | 旧P2の`Product Build=CLOSED`を維持。現行`N04/W03=OPEN`。このE0照合は旧受入れを再開せず、v4受入れも閉じない。 |
| 残件 | 導入済み製品でのA/B同時実行、Workspace/scratch/result分離、独立したtest/result証拠、選択結果の適用とAuditを、後続の製品BUILD/QCで現candidateに結び付ける。 |

#### P3 — エージェント間の引継ぎ

| E0-04項目 | 記録値 |
|---|---|
| 現行ID | 旧製品要求`P3`。v4 S05の直接対応先は`N04`、能力は`K02`。 |
| 旧ID | `P3`。旧rev5の`Product Build`受入れは`CLOSED`。 |
| 原文参照 | rev5仕様書 §13、工程表 §§5–6、指示書 §§7–10（原本仕様SHA-256 `c0b78cfba1fb9c5c5de1947d01b47d0898f01b24de1aa8e1b0a0bd89e35c0877`）、`D4-AUD-F07` §要求差分。現行対応はv4 S05/N04。 |
| 対象 | 完了Taskのcontext/result/artifact/diff/test declarationを別Agentへ渡すHandoff。 |
| 目的 | 受信側が成果を利用できる一方、送信元のPermission/Approval/Credential/Trust/hidden stateを継承せず、新しい条件で認可する。 |
| 生成側 | 承認済みの完了source Task result→`AgentHandoffPackage`／選択済みA2A Artifact package。 |
| 実際の受け取り先 | Agent CenterのHandoff事前検査→別Runtime/Session/Workspace向け新AgentTask request→Broker事前検査→受信側AgentTask consumer。事前検査だけではTaskを実行しない。 |
| 契約とデータ | `agent_handoff_package.schema.json`、`agent_handoff.schema.json`、AgentTask request。context/result/artifact/changed files/diff/test declarationは上限付きでsource Task/result hashに結合する。Authority/Credential/Trust/hidden stateを含めない。 |
| 正常時の作用 | 完了・承認済みsource resultを固定し、target identity/Workspaceを検査後、Handoff packageを新規Task要求へ渡す。Brokerは新しいPermissionとOwner Approvalを別々に要求し、受信結果/Auditを新Taskへ結び付ける。 |
| 否定時の作用 | 未完了/未承認result、source/target identity/hash変更、size/path/schema違反、不正Authority field、source/target同一性違反を拒否する。Artifactは上限付きtext-only/untrusted inputとして扱い、metadataからTrust/Permissionを与えない。 |
| 停止と回復 | 事前検査失敗ではTaskを開始しない。再認可不成立時はfail-closed。結果不明の外部作用を自動再送しない。導入済み製品の連続crash/recoveryは未確認。 |
| 標準対象OSの候補 | 五OS対象の製品機能母集団。現ConsumerはDesktop Agent Center/Internal HandoffとA2A Artifact UI。Mobile Handoff actionは`UNKNOWN`。Linux nativeは必須、保証version/archはN10参照・`UNKNOWN`。 |
| 配布 | N10のOS別候補を参照。Handoff実行用の同一installed artifactと通常配布identityは未確定。 |
| 既存実装 | Internal Handoff/package/事前検査とA2A Artifact事前検査sourceは`WIRED`。schema/否定fixtureあり。旧`Product Build=CLOSED`の履歴試験は主に`FIXTURE`/`WIDGET`。 |
| 実装不足 | 現候補で実Agent間の導入済みartifact受渡し→新しい受信側認可→Task実行→result/Auditを連続して示す`LIVE_PRODUCT`証拠がない。A2A外部作用の全範囲はN04の別残件。 |
| ソース識別SHA | 現HEADのGit blob識別値: `apps/desktop_flutter/lib/screens/agent_center.dart`=`2472faeb31a64d624423c30a0a30d0f153d2e2a3`; `apps/desktop_flutter/lib/services/agent_handoff.dart`=`a85110d96cbade732ae0e1bfea2529f03f37523c`; `apps/desktop_flutter/lib/services/a2a_artifact_handoff.dart`=`e7e2f3454e2843f1d4f4e7b5aedfa55014cde05c`; `specs/agent_handoff.schema.json`=`e37a276f8c3e19816bc4f80824efce546a116d00`; `specs/agent_handoff_package.schema.json`=`a8ae1e2a3be6c0ed5666849fc6553c98083a1cc3`; `apps/desktop_flutter/test/agent_handoff_test.dart`=`c6bc9bef8cc8d153ef59bf421efbd989185f8e0c`; `apps/desktop_flutter/test/a2a_artifact_handoff_test.dart`=`e47005320917f59dfa55ae1e04c4b99d452676de`. |
| 試験区分 | 既存schema否定fixture、Rust `UNIT`、Flutter `FIXTURE`/`WIDGET`。現行導入済み製品Handoff受入れは`NOT_RUN`。 |
| 証拠ID | `NONE`。旧P3 `Product Build=CLOSED`と試験範囲は変更せず、v4受入れ証拠は発行していない。 |
| 阻止要件 | P3専用Registry IDはない。広範な未解決項目`comprehensive_extension_rev1_completion`が関連し、`rev2_flutter_broker_channel_boundary`と`linux_native_product_support`は横断条件。P3への直接帰属は`UNKNOWN`。 |
| 状態 | 旧P3 `Product Build=CLOSED`を維持。現行`N04/W03=OPEN`。この照合はv4受入れではない。 |
| 残件 | 導入済み製品の送信元から受信先への受渡し、result/diff/test declaration表示、新Permission/Approval、受信側実行/Audit、現Artifact provenanceの受入れが残る。 |

P0〜P3の阻止要件欄はRegistryとの関連範囲を記録し、Registryに存在しない旧P専用Blockerを作っていない。五OSの標準候補・保証値はN10表のまま`UNKNOWN`を保持する。P0〜P3の全E0-04欄が埋まったことは、この4 IDの製品受入れ、N01/N02/N04/N09/N11の`CLOSED`、E0全体完了を意味しない。

### P4 E0-04項目の個別補完（2026-11-13、ソースと利用先の照合）

以下は旧Provider／Model製品要求を現行v4へ結ぶE0照合である。rev5原本仕様書§14（原本SHA-256 `c0b78cfba1fb9c5c5de1947d01b47d0898f01b24de1aa8e1b0a0bd89e35c0877`）は、`D4-AUD-F07`で全文・要求差分照合済みの歴史的原本参照を用いる。原本全文は現Repositoryにないため今回あらためて取得・読了したとは主張しない。Product Buildの旧受入れ範囲は`docs/REV5_PRODUCT_PROGRESS.md`のP4節を照合した。現行直接対応はv4 S05のN03/K06であり、隣接N05へ直接mappingを広げない。Source blob SHAは現HEAD `06831aad622186dc59e83335030b720bd5288755`で確認した。

| E0-04項目 | 記録値 |
|---|---|
| 現行ID | 旧製品要求`P4`。v4仕様S05の直接対応先は`N03`、能力対応は`K06`。`Phase 15`はD4追加範囲で、rev1のC工程と同一視しない。 |
| 旧ID | `P4 Provider / Model Center`。rev5 Product Buildは`CLOSED`。この履歴を現行N03の完了へ移さない。 |
| 原文参照 | rev5統合仕様書§14、原本SHA-256 `c0b78cfba1fb9c5c5de1947d01b47d0898f01b24de1aa8e1b0a0bd89e35c0877`。原本同一性・要求差分は`D4-AUD-F07`（VERIFIED）の履歴に依拠する。旧P4受入れ条件と実施範囲は`docs/REV5_PRODUCT_PROGRESS.md`「P4 提供元・模型管理」。現行契約はv4仕様S04/N03・S05/K06。 |
| 対象 | Agent Center内のCodex CLI Provider選択、登録ごとのModel ID、Runtime Adapter metadata、登録後のModel引渡しとProvider/Model状態表示。 |
| 目的 | 利用者が選んだProvider経路とModel IDをBroker登録・Owner確認・Codex CLI実行へ一貫して結び、未確認のProvider接続・Model利用可否や自動代替を成功扱いしない。 |
| 生成側 | Flutter Agent CenterのRuntime/Workspace/Agent CLI登録画面。Providerは現在`openai_codex_cli`固定、Model IDは利用者入力。 |
| 実際の受け取り先 | Agent Center登録要求→native Launcherの同一要求hashに束縛したOwner確認→認証Broker IPC/`broker/protocol.rs`→`ProviderModelSelection`検査とCodex Adapter生成→Codex CLIの`--model`。実Provider接続はこの選択・登録経路のConsumerでは未確認。 |
| 契約とデータ | `provider_model_selection.schema.json`はversion 1、provider ID固定、1〜128文字の許可Model ID、`codex_cli_managed`またはP6に属する`broker_credential_vault`参照、`automatic_fallback=false`を定義。登録契約は`agent_cli_runtime_workspace_registration.schema.json`、Adapter投影は`agent_adapter.schema.json`。Credential実値はpayload/summary/metadataへ含めず、P4旧要求の認証は既存Codex CLI設定委譲としてP6のVault経路と分離する。 |
| 正常時の作用 | 現実装の一経路`OpenAI via Codex CLI`と利用者指定Model IDを登録要求・native Owner確認・Broker Adapterへ保持し、Task実行時に同じModel IDをCLI `--model`へ渡す。Provider接続/Model実在性は`unknown`、自動fallbackは無効。Capability宣言は実Task capabilityを増やさない。 |
| 否定時の作用 | 未対応Provider、未知field/authority類似field、許可外・長さ違反Model ID、自動fallback有効値、認証source/credential参照不整合を拒否する。未確認のProvider/Model healthを推測せず、失敗時に別Providerへ自動迂回しない。 |
| 停止と回復 | native Owner確認またはBroker検査が拒否/不成立なら登録しない。Codex CLI interface検査・起動・実行失敗は該当操作の失敗として返し、P4経路でprovider fallbackを行わない。Provider healthとModel利用可否はunknownのまま。結果不明のTask再送・権限再利用は通常AgentTask規則へ従い、本mappingで新しい回復保証を追加しない。 |
| 標準対象OSの候補 | v4標準対象Windows/macOS/Linux native/Android/iOS。現在のP4画面とCodex CLI登録はDesktop Consumer。Linux nativeは必須対象だが、P4の五OS installed parityやMobile AgentTask Consumerは未確認／一部`BUILD_GAP`。保証version/arch/GUI条件はN10の`UNKNOWN`を保持する。 |
| 配布 | P4の旧試験はCodex CLI interface／偽CLI利用を含み、Provider資格・一般利用者へのCLI導入方法を確定しない。N10標準配布Artifact、Runtime同梱/導入、installed identity/lifecycleは未確定。CLI/terminalを一般利用者へ要求できるという意味へ拡張しない。 |
| 既存実装 | Agent Center登録画面とModel validator、登録schema、Rustの`ProviderModelSelection`/Codex Adapter、native Owner確認summary、Codex CLI `--model`引渡し、Provider/model capability表示が存在。Provider選択は一実装経路のみ。旧rev5 P4 Product Buildは限定範囲で`CLOSED`。 |
| 実装不足 | v4 N03が要求する二種類以上の独立Task-capable Runtimeの実Task/失敗/Authority証拠は未成立。MINIDORAは対話AdapterでAgent Taskを実装せず、第二Runtime要件を満たさない。実Provider接続・Model利用権限・Capability宣言との実測一致、現candidateの五OS installed経路も未確認。 |
| ソース識別SHA | 現HEADのGit blob識別値: `apps/desktop_flutter/lib/screens/agent_center.dart`=`2472faeb31a64d624423c30a0a30d0f153d2e2a3`; `apps/desktop_flutter/test/widget_test.dart`=`f4e9c7b93f832ba2a2cefee779fd12426f240b04`; `native/rust_helper/src/adapters/mod.rs`=`06c540a59437b903b3f045b5aafb3790c21103bd`; `native/rust_helper/src/adapters/codex_cli.rs`=`e5c9d1581ae9a1c0a3ae7fcfa626fdca6fea6d28`; `native/rust_helper/src/broker/protocol.rs`=`9b2076d5f31ca6d77f1f87973d1b394aaf751961`; `native/rust_helper/src/desktop_launcher.rs`=`6a9f335de07fa474ca10043aea63ad6e988b572d`; `native/rust_helper/tests/fixtures/fake_codex_cli.rs`=`f74302045176eeabd9ed36e85205fed3adc13767`; `specs/provider_model_selection.schema.json`=`348ef6104b9d73a5529a0ad81f9a268ae59e4a64`; `specs/agent_adapter.schema.json`=`e0792589a2bae168ba221af4f4f9abb505815c12`; `specs/agent_cli_runtime_workspace_registration.schema.json`=`32a09f2ce33a5a215d64350ee22e24e93f9037c7`. |
| 試験区分 | 旧P4 Product BuildではSchema 155/155正常例/199負例、Conformance 233、Agent Center Widget 50、Desktop test 166、Desktop/Mobile analyze、Windows debug build、Rust debug/release build、登録native 4件、fake Codex CLIのModel引渡し試験がPASSと記録されている。実Windows Codex CLI 0.160.0のhelp確認はinterface存在のみ。Provider API呼出し・実Model利用は未実施。本E0照合単位で再試験していない（`NOT_RUN`）。aggregate Rust 423 pass/2 fail/6 ignoredの二失敗は個別再実行でPASS、P4経路外として`FQ-TEST-LOOPBACK`へ履歴送致済み。 |
| 証拠ID | `NONE` — v4のP4/N03/W03受入れ証拠は発行していない。旧rev5 Product Build記録は旧候補・旧条件の履歴。 |
| 阻止要件 | P4専用Registry IDは存在しない。`comprehensive_extension_rev1_completion`はN03を含む広範な未解決項目、`linux_native_product_support`等はOS横断条件として関連し得るが、P4への直接blocker帰属は`UNKNOWN`。Registry状態・18件分類を変更しない。 |
| 状態 | 旧`P4 Product Build=CLOSED`を維持。現行`N03/W03=OPEN`、E0は`OPEN`。K06は能力対応先であり、本台帳の完了だけでは受入れ完了としない。 |
| 残件 | 第二の独立Task-capable Runtimeと実作用/失敗/Authority証拠、Provider/Model実接続の範囲、OS標準配布・installed consumer、N11 Evidence/Blocker/Artifact全数結合が残る。P4の旧条件を再開せず、残余はN03/K06/N10/N11へ帰属する。 |

P4の阻止要件欄はRegistryに存在しない旧P専用IDを作成せず、関連と直接帰属不明を区別した。旧Product Build CLOSEDは当時の限定機能範囲に固定し、現行N03/W03/E0の状態を変更しない。

### P5 E0-04項目の個別補完（2026-11-13、ソースと利用先の照合）

以下は旧Workspace／History／Evaluation製品要求を現行v4へ結ぶE0照合である。rev5原本仕様書§15（原本SHA-256 `c0b78cfba1fb9c5c5de1947d01b47d0898f01b24de1aa8e1b0a0bd89e35c0877`）は、`D4-AUD-F07`で全文・要求差分照合済みの歴史的原本参照に依拠する。原本全文を今回取得したとは主張しない。旧Product Buildの成立範囲は`docs/REV5_PRODUCT_PROGRESS.md`のP5節に照合した。v4仕様S05の直接対応はN09/N11、能力対応はK03/K05。Phase 6、9〜14（C1〜C6関連）は別軸の旧Phase対応であり、N09全体の共有能力K04/K10/K13をP5専用要件へ拡張しない。現行Source blob SHAは照合HEAD `ab98230a169a99a75262eecf3560b1cc28f80edc`で確認した。

| E0-04項目 | 記録値 |
|---|---|
| 現行ID | 旧製品要求`P5`。v4仕様S05の直接対応先は`N09/N11`、能力対応は`K03/K05`。Phase 6、9〜14は歴史的関連範囲で、直接N mappingを置換しない。 |
| 旧ID | `P5 Workspace / History / Evaluation`。rev5 Product Buildは`CLOSED`。この履歴を現行N09/N11の完了へ移さない。 |
| 原文参照 | rev5統合仕様書§15、原本SHA-256 `c0b78cfba1fb9c5c5de1947d01b47d0898f01b24de1aa8e1b0a0bd89e35c0877`。同一性・要求差分は`D4-AUD-F07`の`VERIFIED`履歴に依拠する。旧P5受入れ条件・試験範囲は`docs/REV5_PRODUCT_PROGRESS.md`のP5節。現行契約はv4仕様S04/N09・N11、S05/K03/K05。 |
| 対象 | Workspace Inspectorの登録Workspace内tree/read/compare/change-list/diff/recovery-preview、AgentTask/Dialogue履歴、Dialogue replay/branch、上限付きresult/diff/changed-file/test申告表示、Regression Case owner登録、Evaluation Case/Dataset/Experiment/比較。Resource/Observation/Trace/NotificationなどはN09の共有Consumerとして記録し、P5固有機能や同一Task機能と一括りにしない。 |
| 目的 | 利用者が登録Workspace、Dialogue/AgentTask履歴、結果投影、Regression/Evaluation Caseを閲覧・管理できるようにする。旧履歴からAuthorityを再利用せず、未検証のAgent申告を独立確認済みへ昇格させない。v4では実操作・観測・Audit/Recoveryを同一実Taskへ追跡できる不足を明示する。 |
| 生成側 | Desktop Workspace Inspector、History画面／履歴内容表示、Evaluation Lab、Runtime DialogueのRegression Case導線と各Flutter/UI client。UI state自体はAuthorityではない。 |
| 実際の受け取り先 | Workspace Inspector→`WorkspaceClient`→認証BrokerのWorkspace tree/read/compare/change-list/diff/recovery-preview/whole-baseline consumer。History→`HistoryClient`→Broker History grant、Dialogue履歴、AgentTask履歴metadata consumer。Evaluation Lab→`EvaluationClient`→Broker `evaluation_control`/`evaluation_lab`のDataset/Experiment/status/compare consumer。Regression Caseは送信要求ID/hashと完了Dialogue receipt→Ownerの明示redaction/native確認→Broker `回帰Case登録`→Windows ProtectedStore。通常一覧はmetadata-only。Evidence/Gateは本台帳・Release Registry/`release_gate_check.py`・Evidence Bundle/ManifestのN11 consumerへ別接続する。 |
| 契約とデータ | 機能ごとに別契約・stateを持ち、P5全体を表す単一Schemaや同一AgentTask横断契約はない。WorkspaceはWorkspace ID・登録hash・承認scope・bounded path/content、Historyは現在の期限付きRuntime別grant・Dialogue/Audit参照・Task ID/result hash等のmetadata、replay/branchは参照Audit/hashと新入力から作る新要求、EvaluationはDataset/Case/Experiment/result、Regression CaseはOwnerが明示redactionした定義と現在のDialogue receipt/Audit/hashを束縛する。 |
| 正常時の作用 | 登録Workspaceを範囲内で一覧・比較・表示し、History grantに従うDialogue/Task履歴を表示する。Dialogue replay/branchは新識別子・新しい承認待ち要求を準備する。EvaluationはDataset Caseごとの既存Dialogue経路と決定論的Evaluatorで比較する。Regression CaseはOwner登録経路で保存し、一覧はmetadataのみを返す。result/diff/changed-file/test表示は旧Product Buildの上限付きprojectionに限る。 |
| 否定時の作用 | 現在のHistory grant/Runtime binding/期限を満たさない要求、重複・欠落・不一致のTask開始/終端Audit、状態とfailure分類の矛盾、異なるRuntimeの履歴、許可外Workspace範囲を拒否する。構造化導入前で安全に再構成できないTask Auditは新一覧対象外。replayへ旧Authority/Permission/Approvalを継承しない。未検証のAgent申告testを独立検証済みと表示せず、秘密やbounded範囲外本文を出さない。 |
| 停止と回復 | Broker再起動時の未終端Task履歴は`suspended`へ隔離し、Audit照合不能なら履歴/Recovery投影を拒否する。Workspace Recovery previewは候補表示で、復旧実行ではない。Dialogue replay/branchはAgentTask再実行ではなくfresh request。現行AgentTaskにpause/resume/restart/replay operationはなく、Runtime lifecycleのpause/restartとは別。Audit/Problems/Recovery/Searchの起動snapshot由来表示をlive Task Recoveryとして扱わない。 |
| 標準対象OSの候補 | v4標準対象Windows/macOS/Linux native/Android/iOS。P5主要Workspace Inspector/History/Evaluation/Regression UIはDesktop consumer。Device Link Mobile History等は限定projectionでありMobile AgentTask操作はない。Linux nativeは必須製品対象。N10の保証distro/version/arch/GUI条件は`UNKNOWN`で、五OS installed parityは未成立。 |
| 配布 | P5旧Product Build試験は通常Release配布・同一candidate installed lifecycleの証拠ではない。五OSの通常配布Artifact、署名/identity、導入済み画面からWorkspace/History/Evaluationの一連consumerへ至る保証はN10に従い未確定。Linux native標準対応を省略せず、Windows debug/staged、Linux bootstrap package、Mobile debug/Simulator buildを一般配布と同一視しない。 |
| 既存実装 | Workspace Inspector/ClientとBroker tree/read/compare/diff/recovery-preview、History UI/clientとBroker履歴/Audit再構成、Evaluation Lab/clientとBroker Dataset/Experiment、Regression Case client/Broker/native confirmation、N11のRegistry/Manifest/Evidence Bundle/Release Gate consumerがソース上存在する。N09個別surfaceのProducer→Consumer接続は`WIRED`として記録済みだが、P5全体の一体Acceptanceではない。 |
| 実装不足 | Workspace差分、History/Evaluation/Resource/Trace/Test result、Audit/Recovery/Search/Problemsを同一AgentTask ID・同一candidateへ統合するConsumerは未成立。AgentTask pause/resume/restart/replay、Taskに束縛されたEvaluation/test結果・Resource/Trace、live Recovery catalog/検索も未成立または`UNKNOWN`。Mobile AgentTask Consumerなし。Regression CaseのC5 Dataset自動importはなく、旧履歴登録はTask完了結果の同一性を証明しない。N09/N11全要求・全OS artifact・Evidence IDまでのGate統合も未成立。 |
| ソース識別SHA | 現HEADのGit blob識別値: `apps/desktop_flutter/lib/screens/workspace_inspector.dart`=`a02c95ec5062583f9c37523fbaf604a8d2da107e`; `apps/desktop_flutter/lib/services/workspace_client.dart`=`870b48e8f4bbd648bcfebc5886addca3784bcfaf`; `apps/desktop_flutter/lib/screens/history_screen.dart`=`dacc4c0ca15325c7f597cd1827a6455fb8ab5560`; `packages/gui_shell_ui/lib/src/history_client.dart`=`7f14ec2caf8b1e383bbf93483ae052100f921ae7`; `native/rust_helper/src/broker/history_access.rs`=`c004056b45c2f26859bf7f70a9ff2b632e358523`; `native/rust_helper/src/broker/execution_history.rs`=`8eeb1c26d06a9e4b01806413a4bfa8c2a73357d6`; `apps/desktop_flutter/lib/screens/evaluation_lab.dart`=`b74b3e96e7cd746874d1e19e05175548a5c66192`; `packages/gui_shell_ui/lib/src/evaluation_client.dart`=`3fe93e4a19bcba7c825f2a6e887f96f00016aa75`; `native/rust_helper/src/broker/evaluation_control.rs`=`570485772c3fce43d170d031f1769b7617214d40`; `native/rust_helper/src/broker/evaluation_lab.rs`=`3645202da2597e5f63cf9ef96a891e9ae801e2b2`; `apps/desktop_flutter/lib/services/regression_case_client.dart`=`c29054e8b6256b15f097bd5e7de57b9cf2117e3a`; `packages/gui_shell_ui/lib/src/regression_case_client.dart`=`f14bd4aeda1d6f3262cdb1be4b924884d7128e5b`; `packages/gui_shell_ui/lib/src/regression_case_owner_client.dart`=`1f1d10cecca70dff2075bed1e5f331267e0ea296`; `native/rust_helper/src/broker/regression_case.rs`=`cf229213a54f0eaa512781fb417692506bf06775`; `native/rust_helper/src/desktop_launcher.rs`=`6a9f335de07fa474ca10043aea63ad6e988b572d`; N11 Gate処理source: `tooling/release_gate_check.py`=`22a8d7bfe9adb259f44ce8501fb215652e1d58c6`, `tooling/evidence_bundle.py`=`68dead851236b0f1b69899151c37371e593ff3f9`, `tooling/manifest.py`=`aa4525ff20293cb8641e5598a2439561a3bfbd3b`, `tooling/conformance_tests/run_conformance_skeleton.py`=`c6f31d8cb561b48513c6784e54c85969c9cb953e`. 履歴試験fileのGit blob識別値: `apps/desktop_flutter/test/workspace_inspector_test.dart`=`2f6e53c2a38d5e23c88f2ad91e079bf8be4808a5`; `apps/desktop_flutter/test/history_screen_test.dart`=`7828dc17fbf50cd0a2dca3255d90c3760030e58f`; `apps/desktop_flutter/test/evaluation_lab_test.dart`=`9b068a34d9d22743bee74ba409c8d95bee938638`; `packages/gui_shell_ui/test/evaluation_client_test.dart`=`044d71b4b18b7efa8891a7df735f28e6ed674ac1`. 詳細なN09/N11のソース一覧は各節の識別値表を参照。 |
| 試験区分 | rev5 P5 Product Build履歴はWorkspace Inspector 22、履歴/内容表示14+4、Evaluation画面5、Dialogue history 27、Regression Case client 6+2、Evaluation client 10、Rust P5対象51の合計141 test PASSと記録。UNIT/FIXTURE/WIDGETを含み、installed Release受入れではない。現N09 canonical source crosswalkの列挙testは本mappingでは`NOT_RUN`。この単位では旧P5 testを再実行せず、文書/hash/field整合のみ検査する。 |
| 証拠ID | `NONE`。旧P5 focused-test記録は旧Product Build候補・条件の履歴であり、v4 N09/N11/E0 evidence IDは発行していない。 |
| 阻止要件 | P5専用Registry IDはない。`comprehensive_extension_rev1_completion`はN09/N11を含む広範な未解決blockerとして関連する。`windows_installer_first_run_smoke`、`windows_broker_installed_smoke`、`linux_native_product_support`、`rev2_mobile_flutter_native_device_link_boundary`はOS/installed境界の横断blocker候補だが、P5への直接帰属は`UNKNOWN`。Registry上はいずれも`unresolved`。status/classificationを変更せず新blockerも作らない。 |
| 状態 | 旧`P5 Product Build=CLOSED`と当時の範囲・141 test履歴を維持。現行`N09/N11=OPEN`、`E0=OPEN`。旧statusはv4の五OS製品受入れ・N09/N11のCLOSEDを意味しない。 |
| 残件 | 同一AgentTask IDへのWorkspace diff/result/evaluation/test/resource/trace/Audit/Recovery/search結合、AgentTask操作の停止・再開・復旧、live evidence consumer、Regression CaseからEvaluation Datasetへの独立Owner操作を含む契約、Mobile/五OS installed consumer、N11 Evidence ID/Artifact/Blocker結合が残る。これらを本P5 mappingで実装要求へ昇格せず、N09/N11および後続QE工程の既存要件として保持する。 |

P5の旧CLOSED履歴は限定Product Build機能と当時testだけに適用する。N09/N11のOPEN、Registry blocker状態、Evidence ID `NONE`を維持し、P5 E0行の完成を製品AcceptanceやE0全体完了へ拡張しない。

### P6 E0-04項目の個別補完（2026-11-13、ソースと利用先の照合）

以下は旧資格情報／MCP製品要求を現行v4へ結ぶE0照合である。rev5原本仕様書§16（原本SHA-256 `c0b78cfba1fb9c5c5de1947d01b47d0898f01b24de1aa8e1b0a0bd89e35c0877`）は`D4-AUD-F07`の全文・要求差分確認済み履歴に依拠し、原本全文を今回取得したとは主張しない。旧Product Build範囲は`docs/REV5_PRODUCT_PROGRESS.md`のP6節に照合した。v4仕様S05の直接対応はN02/N05、能力対応はK07/K08。Phase 16〜19・C7〜C9は歴史的関連範囲、施工線A/D/Fは工程対応であり、P6の旧状態を現在のN02/N05へ移さない。照合HEADは`2f6bdcb425b88bf75850b0e91d99f11213eaaf09`。

| E0-04項目 | 記録値 |
|---|---|
| 現行ID | 旧製品要求`P6`。v4仕様S05の直接対応先は`N02/N05`、能力対応は`K07/K08`。Phase 16〜19／C7〜C9／施工線A・D・Fは別軸の対応である。 |
| 旧ID | `P6 Credential / MCP`。rev5 Product Buildは`CLOSED`。当時の限定credential/MCP機能状態として保持し、現行N02/N05の完了へ移さない。 |
| 原文参照 | rev5統合仕様書§16、原本SHA-256 `c0b78cfba1fb9c5c5de1947d01b47d0898f01b24de1aa8e1b0a0bd89e35c0877`。同一性・要求差分は`D4-AUD-F07`の`VERIFIED`履歴に依拠する。Product Build範囲・試験は`docs/REV5_PRODUCT_PROGRESS.md` P6節。現行契約はv4仕様S04/N02・N05、S05/K07・K08。 |
| 対象 | OwnerによるCredential登録・metadata一覧・失効、Provider/Tool/Server/purposeへ束縛するCredential ID、Providerへの秘密値解決、MCP stdio接続/discovery/Tool実行・監査、およびTool結果を同一Agentへfresh認可付き後続Taskで渡す限定経路。Mobile Device Link用pairing Credentialは別契約で、P6 Provider/MCP Credentialには含めない。 |
| 目的 | 秘密値をOS保護保管へ結び、現在の対象・用途・Permission/ApprovalをBrokerが再確認した後に限り指定Provider processまたは指定MCP childへ渡す。MCP Toolの実作用と未信頼結果をBrokerで監査・束縛し、権限を継承しない新AgentTaskが利用できるようにする。Credential参照・Capability metadataからAuthorityを作らない。 |
| 生成側 | Native Owner Credential入力／登録・失効要求、Desktop MCP Connection CenterのServer/Tool/Credential ID metadata要求、Agent CenterのCodex Task内一回MCP提案、AI orchestration coordinatorの結果依存後続Task要求。Flutterはsecretを取得・保持・返却するproducerではない。 |
| 実際の受け取り先 | Native OS入力→Rust Broker `credential_vault`→OS別保管先／Credential Audit。Provider要求→認証Broker→現在Task Permission/Approval後のCredential resolver→Codex Adapter→Codex CLI親process。MCP Center→`McpConnectionClient`→認証Broker `mcp_center`→現在のCatalog/Schema/Owner確認/一回Permission/Audit→`mcp_stdio`の対象stdio child。N02のTool結果はtask/runtime/session/workspace/source Task/hashに束縛したvolatile one-shot contextを経て`AgentMcpTaskController`の新Taskへ渡る。N05 Compose callは同じManifest/現在CatalogをBrokerが再確認して元Toolを呼ぶ。 |
| 契約とデータ | Credential登録/list/receipt/revocation schema、MCP connection/tool call/receipt schema、Agent MCP提案schemaを個別に使用する。公開要求はCredential ID・用途・対象Server/Provider・環境変数名等の参照metadata。secretの保存hash/状態/用途/対象をBrokerが照合し、secret実値はFlutter・通常IPCのprojection・Audit content/log/error/trace/test artifactへ返さない。Device Link用`device_link_credential`は用途・主体が異なる。P6全体を一つに統合するSchemaはない。 |
| 正常時の作用 | rev5履歴のProvider経路は有効Task grantsと使用Audit確定後に合成CredentialをCodex CLI親processへ渡し、CLI引数/result/Auditから除外し、tool child環境からも除外する。MCP経路は対象Credentialを接続stdio childだけへ渡し、metadata-only discoveryを返す。Tool callは現在Catalog・identity・Schemaを再確認しOwner確認／一回Permission／Audit後に一度実行し、本文を露出せずhash/bounded metadataを返す。後続Taskはsource Task/result hashを束縛し、新Permissionと新Owner Approvalを受ける。 |
| 否定時の作用 | 不正/未知/revoked/期限切れCredential、用途・対象Provider/Server・env var名の不一致、保管hash/state不一致、Permission/Approval欠落、Tool/Catalog/Schema stale、Authority類似metadata、replay、不正なsecret encoding、Audit失敗は拒否し、未認可時にresolver/child起動/外部Tool callを行わない。MCP Catalog/Agent metadataからTrust・Permission・Approvalを生成しない。Mobile Device Link CredentialをProvider/MCP secretへ転用しない。 |
| 停止と回復 | MCP Credential使用Auditを確定できない場合はTool/stdio childを停止し隔離する。Tool結果不明時は再送せず副作用確認を求める。Broker再起動でN02のvolatile result contextは失われ、後続Taskをfail-closedにする。Provider failureで自動fallbackしない。旧実Codex CLI／合成loopback試験はAPI要求前のMxC `CreateProcessSecurityEnvironment` failureで停止し、Provider/model状態を`unknown`のままにした。Credential revokeは既存のOwner操作・Broker規則で処理し、本mappingから物理削除・復旧保証を追加しない。 |
| 標準対象OSの候補 | Windows/macOS/Linux native/Android/iOSが現行標準対象。Credential storage sourceにはWindows protected storage、Linux Secret Service、macOS Keychain経路がある。Provider/MCPの主要UI・Tool実行はDesktop sourceでWindows/Linux/macOS cfgを持つ。MobileのDevice Link pairing Credentialは別用途で、Mobile MCP Tool/Agent follow-up consumerはない。Linux nativeはWindows/macOSと同列の必須対象であり、WSLだけへ限定しない。各OSの保証version/arch/GUI条件はN10の`UNKNOWN`を維持する。 |
| 配布 | 旧Windows Product BuildのDPAPI/fake CLI/`cmd.exe` fixture試験は一般Release・一般配布・別profile installed Credential操作の証拠ではない。Linux Secret Service/macOS Keychain sourceの存在も通常配布からCredential登録→installed MCP/Provider Taskまでの証明ではない。五OSの通常配布Artifact・署名identity・installed lifecycleはN10へ接続し未成立。Linux native package/GUI/secret service/IPCを必須製品経路として残す。 |
| 既存実装 | Rust Broker `credential_vault`・`credential_storage`、Windows/Linux/macOS native credential input、Codex Adapter credential binding、MCP Center/Catalog/Tool call、stdio transport、AgentMcpTaskControllerと一回結果context、credential/MCP/agent request schemas、Desktop MCP Center/clientが現HEAD source上存在する。Linux Secret ServiceとmacOS Keychain向けstore実装も存在する。N02 Desktop Tool→fresh follow-up、N05 Compose→current MCP Tool sourceは`WIRED`だが、実installed all-OS acceptanceではない。 |
| 実装不足 | N02/W02の実Codex→実MCP server→同じAgent結果依存Task→Auditのcurrent installed candidate証拠は未成立。N05/W04のCapability宣言と実能力の一致、同一candidate installedの元Tool作用/Auditも未成立。五OSごとのNative Owner input・OS vault・provider/stdio process・secret境界を同じ通常配布候補で通した証拠がない。MobileにProvider/MCP Credential経路・Tool action・AgentTask follow-upがない。Credential更新・対象変更・物理消去/recoveryなどの範囲を現行Sourceから完了扱いしない。 |
| ソース識別SHA | 現HEAD blob SHA 30件を直下のP6 source表へ記録した。N02/N05の全Source→Consumer/test候補は本台帳N01/N02・N05 canonical source表、Evidence/GateのSourceSHAはN11表を併用する。各SHAは照合HEAD `2f6bdcb425b88bf75850b0e91d99f11213eaaf09`で検証する。 |
| 試験区分 | rev5 P6履歴はSchema 158、正常例155、負例203、Conformance 234、Credential Vault Rust 7、Broker resolver境界1、偽Codex CLI合成Credential1、MCP Broker2、MCP stdio5、Desktop MCP metadata12、Credential ID登録画面1、および`cargo build --locked` PASSを記録する。Credential/MCPに関する子process試験は合成secret／`cmd.exe` fixtureで、外部Provider API成功ではない。実Codex CLI 0.160.0のloopback runはAPI要求前のMxC process failure。aggregate Rust 432 pass/2 fail/7 ignored、および別run 431 pass/0 fail/7 ignored/3 filteredのloopback/TLS履歴と`FQ-TEST-LOOPBACK`記録をその条件のまま保持する。本E0 mappingではこれら旧試験を再実行せず、現N02/N05 test sourceも`NOT_RUN`。 |
| 証拠ID | `NONE`。旧Windows Product Build・fixture・Action run履歴は旧candidate/条件の記録であり、v4 P6/N02/N05/W02/W04向けEvidence IDは発行していない。Registry narrative内の個別run参照も同candidateのP6受入証拠に昇格しない。 |
| 阻止要件 | P6専用Registry IDは確認できない。`comprehensive_extension_rev1_completion`はN02/N05を含む全体要件のblockerとして関連する。`linux_native_product_support`、`windows_installer_first_run_smoke`、`windows_broker_installed_smoke`、`rev2_flutter_broker_channel_boundary`、`rev2_mobile_flutter_native_device_link_boundary`はOS/IPC横断の関連候補だが、P6への直接帰属は`UNKNOWN`。Registry上の既存status/classificationを維持し、新blockerは作らない。 |
| 状態 | 旧`P6 Product Build=CLOSED`と当時の限定範囲・試験/FAIL履歴を維持。現行`N02/N05/W02/W04=OPEN`、E0=`OPEN`、Evidence ID=`NONE`。P6 mapping完了は通常Release capability、外部Provider接続、五OS製品受入れ、N02/N05 closureを意味しない。 |
| 残件 | 同一current candidateのCredential Owner入力→OS保管→Permission/Approval→実ProviderまたはMCP child→Tool結果→同Agent fresh Task→Audit/停止/Recoveryを各標準OSへ接続し、実secretを露出せず確認すること。Linux native標準package/GUI/Secret Service/IPCは必須。Mobileの未実装Provider/MCP操作とN02 follow-upはBUILD_GAPとして残す。N11 Evidence ID/Artifact/Blocker全数結合は別E0残件。 |

#### P6 ソース識別値・該当ファイル

下表は現HEAD `2f6bdcb425b88bf75850b0e91d99f11213eaaf09`上のGit管理ファイル識別値。試験用ソースの存在は本単位の実行・受入れを意味しない。

| ファイル | 識別値 |
|---|---|
| `docs/REV5_PRODUCT_PROGRESS.md` | `9c978021d3982690e9aaa87896436bad7125b433` |
| `native/rust_helper/src/broker/credential_vault.rs` | `029dccda7671d51c2bbd3debd4fe5d7e92221162` |
| `native/rust_helper/src/broker/credential_storage.rs` | `86cf0459915bb748f2d3d5848b20913d9c6080f2` |
| `native/rust_helper/src/broker/mcp_center.rs` | `8f9ab897bdc625e448241ebd9b46f995cd1f22f6` |
| `native/rust_helper/src/adapters/codex_cli.rs` | `e5c9d1581ae9a1c0a3ae7fcfa626fdca6fea6d28` |
| `native/rust_helper/src/adapters/mcp_stdio.rs` | `b7e08327541ad75bca53bb1563e0e20d576d9b66` |
| `native/rust_helper/src/mcp_task_tool_stdio.rs` | `6107890e27be2b9f6bef93a9d46828a488aa1bf1` |
| `native/rust_helper/src/windows_credential_input.rs` | `3f22358ce15f69d34a93ab5f46186374caba13ec` |
| `native/rust_helper/src/linux_credential_input.rs` | `8594403a89e4ed1f8eab38b0bb73c62bbceeb79b` |
| `native/rust_helper/src/macos_credential_input.rs` | `f4a44a0dd9d374243e38dab8726bb44c51970422` |
| `apps/desktop_flutter/lib/main.dart` | `6109c821ab5dc809224bec5234c4c59db1d5f988` |
| `apps/desktop_flutter/lib/screens/mcp_connection_center.dart` | `4c2dd1122e2162a4c3ab30b5c2536e267fee8519` |
| `apps/desktop_flutter/lib/services/mcp_connection_client.dart` | `12a173a3956368c53615255c56ef06a2f5939c9a` |
| `apps/desktop_flutter/lib/services/agent_mcp_task_controller.dart` | `fba6642904d98890339197bef269164970031830` |
| `packages/agent_orchestration/lib/mcp_connection_client.dart` | `71b17973216160bdd2bc447243ba0b04fd0fec63` |
| `packages/agent_orchestration/lib/agent_mcp_task_controller.dart` | `fa46ff071cfb306d3e463a9ad2c79b9149dab6f5` |
| `specs/credential_registration.schema.json` | `65c8bc535e36c040e8574277f06ab1ecb2589d19` |
| `specs/credential_list.schema.json` | `61f706b09533181df57ac781d7eed5adb1199254` |
| `specs/credential_receipt.schema.json` | `657bfc7917123d212cff462ff7572886effc3457` |
| `specs/credential_revocation_request.schema.json` | `43aed5aa2d4f8dc56d9f91386b69b73bed7a3427` |
| `specs/mcp_connection.schema.json` | `a8d613bd911c9e88f949b984b1a2032eb19db332` |
| `specs/mcp_tool_call.schema.json` | `54c044cdfb53588ea5bab4bd056404372dc39acd` |
| `specs/mcp_tool_call_receipt.schema.json` | `0049a393104b035bbf047e653e962754bc4009b3` |
| `specs/agent_mcp_tool_request.schema.json` | `f3aa71661ac1dadf453954141802a7663755da9d` |
| `apps/desktop_flutter/test/mcp_connection_center_test.dart` | `c81a37ca78b15e57c838f48519aa3e4abb6d6665` |
| `apps/desktop_flutter/test/agent_mcp_task_controller_test.dart` | `5c165d5362eea4906b6e739407b0d0e75c34485c` |
| `apps/desktop_flutter/test/linux_credential_vault_test.dart` | `64c250a967fcfbc3b5da941810afbf558178dfe7` |
| `apps/desktop_flutter/test/macos_credential_vault_test.dart` | `5d2ae1900cf31f7ff06ee6236ede132b0e933ea3` |
| `native/rust_helper/tests/unit/codex_cli_fixture.rs` | `40e5750bf267ccf9e583971e0d2ba4e6125d9a5d` |
| `native/rust_helper/tests/fixtures/fake_codex_cli.rs` | `f74302045176eeabd9ed36e85205fed3adc13767` |

P6の旧CLOSEDは上記Product Build限定機能と当時条件に適用する。N02/N05/W02/W04/E0のOPEN、Release Registry、`release_ready=false`を変更せず、P6のSource mappingを現行製品受入れへ拡張しない。

### P7 E0-04項目の個別補完（2026-11-13、A2A／Host／Adapter）

以下は旧A2A／複数Host／Adapter管理要求を現行v4へ結ぶE0照合である。旧範囲とProduct Build判定は`docs/REV5_PRODUCT_PROGRESS.md`のP7節、Phase 24〜27／C15〜C19の対応は`docs/D4_POCKET_PHASE_MAPPING.md`に照合した。v4仕様S05の直接対応はN03/N04、関連能力はK11/K12である。N04のCompare／内部Handoff受入れ範囲は旧P2/P3 mappingの責任に残し、P7のA2A／Host経路へ重複計上しない。Source照合HEADは`4e8233f0519f71ce136415b691431557249f612c`。

| E0-04項目 | 記録値 |
|---|---|
| 現行ID | 旧製品要求`P7`。v4仕様S05の直接対応先は`N03/N04`、関連能力は`K11/K12`。Phase 24〜27／C15〜C19と施工線B/Fは工程上の対応であり、旧P7の状態を現行受入れへ移さない。 |
| 旧ID | `P7 対A2A連携／複数Host／Adapter管理`。rev5 Product Buildは`CLOSED`。当時の限定受入れと証拠classを履歴として保持し、外部Runtime・Agentの全能力保証へ拡張しない。 |
| 原文参照 | `docs/REV5_PRODUCT_PROGRESS.md`のP7全節、`docs/D4_POCKET_PHASE_MAPPING.md`のPhase 24〜27／C15〜C19、v4仕様S04 N03/N04・S05 P7/K11/K12、工程表R03線B/F。旧P7の実接続／fixtureと未成立範囲は同進捗記録の記載を維持する。 |
| 対象 | Codex CLIとMINIDORAのRuntime能力差、Desktop A2AのAgent Card／Message／Stream／Task状態取得／選択Artifact、Host登録・一覧・表示・Broker snapshot由来capability/degraded状態、Adapter既存record操作とManifest metadata登録／更新。Host／Agent／Adapterの外部実接続、Runtime Task、filesystem導入、process起動は別々のconsumerとして扱う。 |
| 目的 | 異種Runtime・外部Agent・Host・AdapterをBrokerの既存統治経路へ接続し、宣言metadataと実能力、ローカルBroker状態とremote Host状態、metadata管理と実artifact導入を区別する。外部AgentやAdapterの宣言からTrust／Permission／Approval／Credential／Authorityを生成しない。 |
| 生成側 | Agent CenterのRuntime／Agent選択、Desktop A2A接続画面、Host登録・操作・能力画面、Runtime CenterのAdapter管理、Brokerが保持するRuntime／Host／A2A／Adapter状態。Agent Card／Host／Adapter由来dataは未信頼入力として受け取る。 |
| 実際の受け取り先 | Runtime起動引数→Broker登録→共通`実行系Adapter`→Agent Task consumer。Desktop A2A画面→`A2aConnectionClient`→認証Broker operation→`a2a_center`→`a2a.rs`。Host画面→`host_registration_client`／Broker snapshot→`host_center`・`host_capability`。Runtime Center→`shell_core_client`→Broker `protocol`→`adapter_center`。現行source上、Host registryはmetadata管理でremote Host実行経路ではない。 |
| 契約とデータ | Adapter／Runtime、A2A Contract・Connection・Message／Stream／Task request、Host registration／list／capability／switch、Adapter Manifest／management request・receiptの個別Schemaを使う。A2A作用は現在endpoint・payload hash・native確認／一回Permission／Auditに結合する。Host／Agent Card／Capability／Adapter metadataはAuthorityではない。 |
| 正常時の作用 | Source上、Codex CLIはBroker Agent Taskを実行できる。MINIDORAはloopback対話APIへ接続するがAgent Task methodは持たず、既定unsupportedを返す。Windows/Linux A2A sourceはCard取得・Message／Stream／GetTaskをBroker経由で扱い、Host登録・snapshot表示とAdapter catalog metadata操作も接続済み。Adapter Manifest操作は登録／更新metadataまでで、外部artifact取得・filesystem変更・Adapter process起動を行わない。 |
| 否定時の作用 | 未対応Runtime Task、虚偽のCapability、未知／不正Schema、endpoint・payload・要求hashの不一致、古いAdapter record hash、署名未検証Adapterの有効化、非Broker／fallback snapshotからの実測表示は拒否または未観測として返す。Agent Card／Host／Adapter metadataはTrust・Permission・Approvalを昇格しない。 |
| 停止と回復 | A2Aの非冪等作用後に応答・Auditが不確定なら`unknown/suspended`で停止し自動再送しない。Audit確定に失敗した作用を成功にしない。macOS A2AはAgent Card／metadata限定でMessage／Task／Artifact／Stream作用を有効化せず、Mobile consumerは未確認。Host metadataからremote capabilityやWorkspace権限を推定せず、Adapter metadata登録をprocess／filesystem導入成功へ読み替えない。 |
| 標準対象OSの候補 | Windows／macOS／Linux native／Android／iOSの五OS。現行A2A network sourceはWindows/Linux、macOSはloopback Card／metadata限定、MobileのA2A／Host／Adapter作用は未確認。Codex CLI TaskのW01証拠は限定Windows候補、MINIDORA Taskや二つ目のTask-capable Runtimeは未成立。五OS標準installed product parityはN10へ接続し、WSL／fixtureをLinux native受入れへ転用しない。 |
| 配布 | 通常配布・installed productにおけるRuntime選択→外部A2A作用→Host越境拒否→Adapter取得・導入・更新・停止は五OSとも未成立。既存Adapter Manifest機能はBroker catalog metadata操作に限られ、署名／artifact取得／filesystem／processを含む導入済み製品能力ではない。通常配布物・Publisher identityはN10／既存Release Registryの別条件。 |
| 既存実装 | `a2a_center`／`a2a.rs`、Host registry／capability投影、Adapter catalog／Manifest metadata操作、Desktop Runtime／Host／A2A画面、Codex CLI／MINIDORA Adapter、各request／receipt Schemaが現HEADに存在する。N03のCodex TaskとN04のA2A／UI接続はsource上`WIRED`。Host remote action、第二Task Runtime、A2A installed external-agent chain、Adapter filesystem/process導入は`BUILD_GAP`または未確認。 |
| 実装不足 | N03: Agent Task可能な二つ目の独立Runtime、MINIDORAのTask-capable接続・GUI登録・能力差表示・実Task証拠がない。N04: 同一installed候補で外部Agent Card→Message／Artifact→受信Agent Task→Auditを完走した証拠、Host間の実Workspace隔離／remote capability、全OS作用 parityがない。Adapterの外部artifact取得・検証・導入／削除・実process管理と製品利用者向け実証も未成立。W03/G3、N03/N04、E0は`OPEN`。 |
| ソース識別値 | P7に直接関係するRust／Dart／Schema／Test source 40件のGit blob SHAを現HEADで下表に記録した。N03/N04のより広いProducer→Consumer表と追加sourceは本台帳「N03/N04 現行Source→Consumer追跡」を併用する。v4採用基準HEADから製品source／Schema／Testの差分は0で、ここで列挙した識別値は上記照合HEADの値である。 |
| 試験区分 | rev5 P7履歴はA2A Rust境界9、native Owner確認候補1、Broker gate 1、Flutter A2A 9、Navigation 2、Host画面54、Adapter Center Rust 7、Owner候補／中継2、Adapter Manifest Rust 9、Win32 Owner画面自動操作、Desktop画面57、Schema 158／正常例155／否定例205、Conformance 234、Windows debug build等の限定PASSを記録する。Rust全体443 pass／2 fail／9 ignoredのloopback／TLS fixture失敗も履歴のまま保持する。本E0 mappingで旧試験を再実行せず、現Rust／Flutter／installed product試験は`NOT_RUN`。旧Win32確認画面自動操作は製品installed受入れへ昇格しない。 |
| 証拠ID | `NONE`。旧P7のRust／Flutter fixture、開発build、Owner画面自動操作は当時のProduct Build範囲に限り、v4 N03/N04/W03/E0向けEvidence IDを発行していない。外部Agent／通常配布／五OS installed chainの証拠ではない。 |
| 阻止要件 | P7専用Release Registry IDは確認できない。`comprehensive_extension_rev1_completion`、`rev2_flutter_broker_channel_boundary`、`linux_native_product_support`、`rev2_desktop_product_distribution`等は広域の関連候補だが、P7への個別帰属・新規blocker化は`UNKNOWN`。Registryの18 ID／status／分類を変更しない。W03/G3の未成立は現行工程台帳で保持する。 |
| 状態 | 旧`P7 Product Build=CLOSED`と当時の限定受入れ／FAIL履歴を維持。現行`N03/N04/W03/E0=OPEN`。A2A／Adapterの一部sourceは`WIRED`、MINIDORA Agent Task・Host remote action等は`BUILD_GAP`または未確認、v4 Evidence IDは`NONE`。本mappingは旧P7を再開せず、現行要求の受入れ完了も意味しない。 |
| 残件 | N03の二つ目のTask-capable Runtimeと製品経路、N04の外部A2A実作用・Artifactのfresh認可付きAI消費・Host分離、Adapter artifactの実導入／process管理をBroker経路へ結び、Windows／macOS／Linux／Android／iOSの標準対象範囲を個別に記録する。Source接続とinstalled／VM product証拠、比較・Handoffとの境界、全OSのunsupportedを明示する。 |

#### P7 ソース識別値・該当ファイル

下表のGit blob SHAは現HEAD `4e8233f0519f71ce136415b691431557249f612c`から取得した。既存source／testの存在は、本単位での実行や製品受入れを意味しない。

| ファイル | 識別値 |
|---|---|
| `docs/REV5_PRODUCT_PROGRESS.md` | `9c978021d3982690e9aaa87896436bad7125b433` |
| `native/rust_helper/src/adapters/mod.rs` | `06c540a59437b903b3f045b5aafb3790c21103bd` |
| `native/rust_helper/src/adapters/codex_cli.rs` | `e5c9d1581ae9a1c0a3ae7fcfa626fdca6fea6d28` |
| `native/rust_helper/src/adapters/minidora.rs` | `31c756ac7c9a264d73ed0e9654dbae599be83921` |
| `native/rust_helper/src/broker/ipc_server.rs` | `58a404b920ef5905e174668ec2de0e04a9e8b044` |
| `native/rust_helper/src/broker/dialogue.rs` | `c3c3fddb630dd06b7e6cc1c13b39f3d4f6f6a999` |
| `native/rust_helper/src/broker/protocol.rs` | `9b2076d5f31ca6d77f1f87973d1b394aaf751961` |
| `native/rust_helper/src/a2a.rs` | `9d7c6d9ab938ad5178d6ca46b3933dc3e61c6061` |
| `native/rust_helper/src/broker/a2a_center.rs` | `73ad601abd64a33ccefde4b73ab5f9dc4f728504` |
| `native/rust_helper/src/broker/host_center.rs` | `c27707faa61e5270b1c6d79e3ee3c5fe6ccd0bf8` |
| `native/rust_helper/src/broker/host_capability.rs` | `08a82c7e52947439229447dc82b69097af67ba1f` |
| `native/rust_helper/src/broker/adapter_center.rs` | `66575bb77e4a375dca4f8488f97df442960bb5a9` |
| `native/rust_helper/src/desktop_launcher.rs` | `6a9f335de07fa474ca10043aea63ad6e988b572d` |
| `apps/desktop_flutter/lib/screens/runtime_center.dart` | `d2c7af172073d2b10c76a6416b90f011f10e95db` |
| `apps/desktop_flutter/lib/screens/host_operation_center.dart` | `e16605e6bb39bbfc12806e6154e08d6fbab1b93b` |
| `apps/desktop_flutter/lib/screens/host_capability_center.dart` | `e8fa774a435e713a3b75b0d33f00bd500b0a2d7f` |
| `apps/desktop_flutter/lib/screens/a2a_connection_center.dart` | `2716d1c2fa22a184044e6f4e2eb401cb33c6e011` |
| `apps/desktop_flutter/lib/services/a2a_connection_client.dart` | `19335054afe35bc06597629d92ca2e635ce50069` |
| `apps/desktop_flutter/lib/services/host_registration_client.dart` | `3bc3b45bddddf4e1131e051acb3cdee599ca4d3d` |
| `apps/desktop_flutter/lib/services/shell_core_client.dart` | `f08db061bd15649ff7c2fc3fc9e37184433cbf18` |
| `specs/agent_adapter.schema.json` | `e0792589a2bae168ba221af4f4f9abb505815c12` |
| `specs/adapter_manifest.schema.json` | `116f5830983d195f501cc344a25990d58bf323c0` |
| `specs/adapter_management_request.schema.json` | `2c9c98d0c60603adfaa246f9efa4753bfc0e7597` |
| `specs/adapter_management_receipt.schema.json` | `7b43a2d2b74c5bd0a931ead66f32f55663919b04` |
| `specs/adapter_management_manifest.schema.json` | `1f1b75c7d4299c96fcf5039b652d718f7805cd2d` |
| `specs/adapter_management_list.schema.json` | `530bc79a6a5117bb29d0de9e0932e3c40d8d785d` |
| `specs/a2a_contract.schema.json` | `8c7f50bb7fb12dde41293727e5dc9109445d1bc3` |
| `specs/a2a_connection.schema.json` | `1dbbe44e54b302a1196d7e4e55f1cae35cda569b` |
| `specs/a2a_connection_receipt.schema.json` | `f63fd0d61b6200cdd8cc4b23d5cf7968a82fa5c5` |
| `specs/a2a_message_send_request.schema.json` | `0a5f87250f17a88b6be55e7838d32b8e9114bbed` |
| `specs/a2a_stream_send_request.schema.json` | `e0a0841f2fd9f7e92e0fb8a036ebaa6852e198d8` |
| `specs/a2a_task_get_request.schema.json` | `df11a5fd3291de0ae379694c6c1b49de51f2f157` |
| `specs/host_registration.schema.json` | `8bed26fb2fe8f8e296c1d3404978f3215d571240` |
| `specs/host_list.schema.json` | `93880c43b04a5408c033bf351ee695073e54664a` |
| `specs/host_capability.schema.json` | `6b721b5ab7842edc7df6562bc56fdd5b73d5de79` |
| `specs/host_switch.schema.json` | `01e5fc7322e8d4d2c9142375e7641fc4255148d5` |
| `specs/host_switch_receipt.schema.json` | `cb60df0b174a38c871e0e8835c65537a17d022fb` |
| `apps/desktop_flutter/test/a2a_connection_center_test.dart` | `5ebdbb4cee62e581062370ed6c305c36df89b7e2` |
| `apps/desktop_flutter/test/host_registration_test.dart` | `74ab39e078bf72214a3bbe84f69ebc77fceb2493` |
| `apps/desktop_flutter/test/widget_test.dart` | `f4e9c7b93f832ba2a2cefee779fd12426f240b04` |

## 旧C0〜C6のv4 Source→Consumer crosswalk（E0差分）

旧rev1の仕様・工程・進捗は当時の要求範囲と履歴statusを確認する資料として読み、現在statusはv4 S04/S05、工程表R03、現行Source→Consumerへ接続した。C0〜C6の旧statusは変更しない。v4 target statusはN11/C0のbaseline記録を含め、製品受入は未成立のままである。Phase対応は`docs/D4_POCKET_PHASE_MAPPING.md`の既存逆引きを参照した。

| 旧ID／Phase | 旧要求とrev1時点のstatus・証拠 | v4対応 | 現在のProducer→Consumer／確認できる状態 | v4で残る差分・証拠限界 |
|---|---|---|---|---|
| C0／Phase 0 | clean `main` baseline、backup、Schema/Conformance、Rust/Flutter、release blockerを記録。進捗表は「基準検証保存・先行検証整理中」。保存記録のsource commitは`c5aa1ec06c2074b93ca2475abdf9938557699d2d`、`validate_all.py --desktop-platform windows --include-mobile-release` exit 0／開発17項目PASS。Schema 36、Conformance 150、Rust 63+IPC5+checkpoint8、Flutter検査を含む。外部log pathとSHAは旧進捗に記録済み。 | N11／工程表R03の線J。これは製品機能Consumerでなく、baseline・証拠・Gateの管理対象。 | 旧baseline記録→本E0台帳／root `MANIFEST.sha256.json`／`tooling/manifest.py`／`tooling/release_gate_check.py`。旧source SHA・旧検査scopeは履歴として保持する。 | 旧baselineは現v4候補の検査ではない。再実行・v4 PASSへの転記をしない。N11/E0は`OPEN`で、現在のcandidate証拠・全OS QA・Release判定は別途未成立。 |
| C1／Phase 5・6 | Workspace Inspector：File Tree、Diff、Tool/Shell履歴、Test結果、Approval/Audit、rollback候補。旧進捗は読取・範囲基準点・diff UI接続、全Workspace探索・巨大file比較・実履歴・rollback preview未完。 | N09、K03/K13。 | Workspace Inspector→`WorkspaceClient`→Broker tree/read/compare/change-list/diff/recovery-preview。N09のWorkspace行とSource SHA表を参照。source上の個別consumerは`WIRED`。 | 現DiffはWorkspace/baseline基準で完了TaskのDiff Artifactではない。Recovery previewは復旧作用ではない。旧全探索・巨大file等の残差とN09のTask binding gapを保持。 |
| C2／Phase 9・10 | History、再実行、分岐：新要求・新識別子・新Audit、古いApproval/PermissionやUI stateの継承拒否。旧進捗は実履歴・新承認による再実行/分岐・保存内容閲覧を接続、保存/破棄Recoveryと全体判定未完。 | N09、K03。 | History UI→`HistoryClient`→Broker History/Dialogue/AgentTask metadata。Dialogue replay/branchは参照Auditと新入力から新要求を作る。 | Dialogue replay/branchはAgentTaskの再実行ではない。旧承認・Permissionの継承に使わない。Task history/結果表示と同一AgentTaskのreplay・Recovery統合は`BUILD_GAP`/`UNKNOWN`。 |
| C3／Phase 11 | Runtime別process/CPU/RAM/Disk/Network/uptime、取得可能時のみGPU、履歴chart。旧進捗はWindows開発経路・Broker観測・Flutter表示・隔離実MINIDORA接続まで記録し、installed/releaseと非Windows物理実機を未取得とした。 | N09、K04/K10。installed OSの受入媒体はN10。 | Runtime Center Resource→`RuntimeResourceClient`→Broker `実行系資源観測`／`RuntimeResourceRegistry`。実測不能値はunknownとし、Source/Evidence境界はN09に記録。 | ResourceはRuntime ID単位でAgentTask IDへ束縛されず、Task test/result指標ではない。旧物理実機待ちはv4のN10標準VM/Emulator/Simulator installed QAに置換し、Linux nativeを含むOS対象自体は保持。N09/N10は`OPEN`。 |
| C4／Phase 12 | Capability-first lifecycle start/stop/restart/pause/resume/quarantineをBroker Authority経路で操作。旧進捗はBroker統治、専用IPC、debug画面、development fixtureでの限定実接続を記録。 | N09、K04/K13。installed OSの受入媒体はN10。 | Runtime Center lifecycle UI→Broker Runtime lifecycle status/approval/execute→登録Adapter。旧進捗にあるdebug fixtureの履歴はそのscopeに保持。 | Runtime lifecycleはRuntime単位で、同一AgentTaskのpause/restartとは別機能。production Adapter・installed OSの実作用は未成立。旧物理実機待ちは現v4 OS標準QA待ちへ移行し、物理非Windows deviceを必須条件にしない。 |
| C5／Phase 13 | 評価Dataset/Case/Evaluator/Experiment/Result/Comparison。旧進捗はWindows Broker、owner専用Dataset、決定論評価器、UIとSchema/Conformance/Rust/Flutterの当時検査を記録。実稼働Runtimeの受入とreleaseは未成立。 | N09、K05。 | Evaluation Lab→`EvaluationClient`→Broker `evaluation_control`、Dataset/Experiment/status/compare、既存Dialogue承認経路。詳細はN09 Evaluation/Resource行。 | Evaluationは同一AgentTask IDのtest結果を評価していない。旧検査数・debug画面は現v4 installed QAではない。Runtime横断比較と同一candidate acceptanceはOPEN。 |
| C6／Phase 14 | 完了Dialogueから明示redaction済みRegression Caseをowner登録。旧進捗はBroker証跡照合、Windows ProtectedStore、hash-only receipt、metadata一覧、Owner確認付きdelete/recoveryのsource/component接続を記録。installed Owner操作未証明、C5 Dataset revisionへのimport未完。 | N09、K05（Regression Case/Evaluation）。Dialogueの実Task/result producerはN01との境界確認が必要。 | Runtime Dialogue pane→送信receiptの要求ID/hash→ownerの明示redacted Case→Rust launcher native confirmation→Broker `回帰Case登録`→結果receipt/終了Audit再照合→Windows ProtectedStore。通常一覧はmetadata only。Current specs/source SHAは次表。 | 旧C6はRuntime Dialogueからの登録で、N09同一AgentTaskのTask bindingを証明しない。C5 Datasetへの自動importは禁止・未実装（独立owner操作/revisionが必要）。Installed product上のOwner登録/delete/recovery実作用は未証明。native confirmationはWindows本人性証明ではない。 |

C6のsource hashは現HEAD `b2e20c2a16e2f324f3848ecc06203103755ceabf`上のGit blob SHA。ここではpath/consumerの同一性を記録し、実行・Acceptanceを意味しない。

| Source | Blob SHA |
|---|---|
| `docs/specs/regression-case.md` | `ce58a891facfca9f1e8e498e13bf983011350ba3` |
| `specs/runtime_dialogue_submission_receipt.schema.json` | `204b7871ab08539c16da2431541a5764b023f1fe` |
| `specs/regression_case_registration.schema.json` | `04165813e2791196c38fe515c84b4e1adeefd000` |
| `specs/regression_case_receipt.schema.json` | `3defc85eab0e8ed4ebac4ba85a1a5b83de6016a4` |
| `specs/regression_case_list_request.schema.json` | `72ded3ae0e37214b442811ba973bdf4a490aa854` |
| `specs/regression_case_list.schema.json` | `a7f99f9784b287b7bb21678bc12ab531debf5af0` |
| `specs/regression_case_delete_request.schema.json` | `7ed67c02cb4796da94adc6dd0b1674f4b8a91ca4` |
| `specs/regression_case_delete_receipt.schema.json` | `d051f4e795f8c05051356b0f1d1dc89947db3633` |
| `specs/regression_case_delete_recovery_request.schema.json` | `c69d71ef2f07a2a7090ba3d0d648448c60ca1c4e` |
| `specs/regression_case_delete_recovery_receipt.schema.json` | `b9b1de18f811bc03f38d5dfaa497f673e335e496` |
| `native/rust_helper/src/broker/regression_case.rs` | `cf229213a54f0eaa512781fb417692506bf06775` |
| `native/rust_helper/src/broker/protocol.rs` | `9b2076d5f31ca6d77f1f87973d1b394aaf751961` |
| `native/rust_helper/src/broker/ipc_server.rs` | `58a404b920ef5905e174668ec2de0e04a9e8b044` |
| `native/rust_helper/src/desktop_launcher.rs` | `6a9f335de07fa474ca10043aea63ad6e988b572d` |
| `packages/gui_shell_ui/lib/src/runtime_dialogue_client.dart` | `44f810aa2509a8a28ddf2933a5491528f3b07147` |
| `packages/gui_shell_ui/lib/src/runtime_dialogue.dart` | `f78083b376f80d06f284387bb8b7aa2d769141bf` |
| `packages/gui_shell_ui/lib/src/regression_case_owner_client.dart` | `1f1d10cecca70dff2075bed1e5f331267e0ea296` |
| `packages/gui_shell_ui/lib/src/regression_case_client.dart` | `f14bd4aeda1d6f3262cdb1be4b924884d7128e5b` |
| `apps/desktop_flutter/lib/screens/runtime_dialogue.dart` | `803edd0a687acdfd00e19898fbb5ba6b85ce1449` |
| `apps/desktop_flutter/lib/services/runtime_dialogue_client.dart` | `a0853ee7327cafa60ee89b0c5ee49312b73c46c4` |
| `apps/desktop_flutter/lib/services/regression_case_client.dart` | `c29054e8b6256b15f097bd5e7de57b9cf2117e3a` |
| `packages/gui_shell_ui/test/regression_case_owner_client_test.dart` | `48a895c5a2f139eb2823df67f4aad3e1b8639748` |
| `packages/gui_shell_ui/test/regression_case_client_test.dart` | `a30a002abf021730f0ba09e66126ebbb2a39e12a` |
| `apps/desktop_flutter/test/runtime_dialogue_test.dart` | `899912f92ee3751f8616632d434e04f95ee5ef77` |
| `tooling/conformance_tests/run_conformance_skeleton.py` | `c6f31d8cb561b48513c6784e54c85969c9cb953e` |

Related test source: `packages/gui_shell_ui/test/regression_case_client_test.dart`、`regression_case_owner_client_test.dart`、`apps/desktop_flutter/test/runtime_dialogue_test.dart`、`tooling/conformance_tests/run_conformance_skeleton.py`内のC6 contract検査。TestClassはDart unit/widget・schema/conformance sourceで、今回のSource mappingでは未実行。Broker receipt/listは旧contract上`INTERNAL_STATE`、削除後ProtectedStore観測は`LIVE_RUNTIME`と明示される。実installed Owner操作・Windows artifact provenanceへ証拠を昇格しない。

C0〜C6の7 IDについて、旧要求・Phase・旧statusを変更せずv4の直接N/Kとconsumer/gapへ接続した。これはC0〜C6 acceptanceの再判定、C1〜C6の製品完了、N09/N11のCLOSED、E0全数完了ではない。C7〜C14、C15〜C23、C24〜C34の後続crosswalkも記録し、C0〜C34全35 IDの旧ID対応付けを完了した。全E0-04列の充足、Phase/K/Buffer/Q/FQ/W照合、N11 Evidence結合とE0完了は別途継続する。

## 旧C7〜C14のv4 Source→Consumer crosswalk（E0差分）

旧仕様・工程・進捗のstatusはrev1当時の履歴として保持し、変更しない。以下の現行source観測基準はmain f8a6b5ed521495fe7d0a4b5c79292fa3a7c169a0。N/K対応はv4 S04/S05およびR03の実consumerに限り、現行のN受入れやPhase完了へ旧C statusを昇格しない。旧Phase対応はdocs/D4_POCKET_PHASE_MAPPING.mdの逆引きである。Evidence IDのない現候補受入れはUNKNOWN、列挙するtest sourceは本source-mapping単位でNOT_RUN。

| 旧ID／Phase | 主語・目的／rev1時点のstatus | v4対応 | 現行Producer→Consumer・契約・source状態 | 否定・停止・Authority・Recovery | OS・配布・TestClass・Evidence・残差 |
|---|---|---|---|---|---|
| C7／Phase 16 | Credential VaultがRuntime／Toolの秘密値を保護し、GUIへ露出させない。履歴ではOwner登録、Windows DPAPI、metadata一覧、秘密値非投影、暗号文欠落／改変のfail-closedを接続。取得／注入、更新／失効／削除、接続先変更、Recovery、GUIは未完了。 | N01／N02／N05、K07／K08。Codex Provider API keyはN01、MCP接続CredentialはN02／N05のconsumer。OS別vaultとinstalled経路はN10で別判定。 | Rust Owner入力からBroker credential registration／Audit／OS storeへ書込み、通常IPCは検証済metadataだけを一覧投影。MCP CenterはCredential ID・purpose・server binding・環境変数名のみBrokerへ渡し、Brokerが現在Audit／保管hash／対象を再検査してstdio childへ限定注入する。Codex Provider用API keyもopenai_codex_cliの承認済Taskに限定する。sourceはWIREDの部分範囲。 | Credential自体はPermission／Approval／Trustを作らない。Flutter、Audit本文、snapshot、error、log、traceへ秘密値を出さない。通常IPCの登録・Owner作用を拒否し、失効後の利用や不整合をfail-closed。Credential更新、対象変更、暗号文の物理削除／Recoveryは未接続。 | Windows DPAPI、Linux Secret Service、macOS Keychainには限定consumerがあるが、保証OSとinstalled配布の全経路は未成立。Linuxのprocess group外やBroker crash後cleanupは保証しない。Rust unit／macOS・Linux Flutter bridge test sourceあり、本単位NOT_RUN。Windows installed登録・注入・失効はUNKNOWN。 |
| C8／Phase 17 | MCP Server／Tool／Resource／Prompt／Transport／Credential ref／Trust／Capability差分の射影契約とConformance。履歴ではmetadata-only contract、fixture、Conformance完了、実接続はC9へ分離。 | N02／N05、K08。 | specs/mcp_contract.schema.jsonとdocs/specs/mcp-contract.mdがmetadata contractを定義し、Broker MCP Catalog／Tool実行が現Catalog・schemaを再検査する。Tool実作用はN02/N05に接続されsource上WIRED。Resource／Prompt等の全概念が実処理へ接続済みとは扱わない。 | MCP metadata、Tool description、Credential ref、Trust、Capability差分はAuthorityではない。未知field、外部schema参照、Authority／secret混入、stale Catalogを拒否。Tool作用には別個のnative Owner確認、一回Permission、durable Auditが必要。 | Desktop MCP sourceはWindows／macOS／Linux経路に分岐。Mobile Tool consumerは未接続。Schema／Conformance／Rust test sourceあり、全て本単位NOT_RUN。現在candidateのinstalled Conformance／実接続EvidenceはUNKNOWN。 |
| C9／Phase 18・19 | MCP Centerがstdio discovery／connect／listを行い、Tool作用をBrokerへ通す。履歴ではdiscovery/catalog／receipt／一覧、timeout・secret・Authority拒否が接続済み、Tool実行等は未完了。 | N02／N05、K08。OS製品化EvidenceはN10の範囲。 | Flutter MCP Center→通常Broker client→Rust mcp_center→stdio child。Brokerがdiscovery・一覧・current Catalogを扱い、操作者のTool callはschema／ID／引数を再検査後、native確認・一回Permission・Auditを経て一度だけ送信する。N02ではCodex proposal→Tool→bounded result context→同Agentの後続Task source経路がWIREDだが、実Codex installed positive W02/G2はOPEN。 | timeout／malformed response／結果Audit失敗等は結果不明としてconnectionを隔離し自動再送しない。Tool result本文は通常receipt／UI／Auditへ出さずhash中心。W02 handoffは別Taskで新Workspace PermissionとOwner Approvalを要求し、結果は未信頼入力。Mac process groupとLinux groupの保証をWindows Job Objectへ読み替えない。 | Windows／macOS／Linuxのstdio範囲。Linux UI/Secret Service・process groupはsandboxでなく、group外child／Broker crash cleanupを保証しない。Mobile Tool作用は明示的BUILD_GAP。Rust unit/fixture、Flutter widget/controller、real_broker_mcp_probe sourceはあり、本単位NOT_RUN。外部実MCP installed productと現候補W02/G2 Evidenceは未成立。 |
| C10／Phase 20 | 運用Profileを作成／複製／適用要求／削除／export／importし、Profile由来Permissionを禁止。履歴ではProfile contract／永続化／IPC／Desktop設定／Permission非生成を接続、Runtime反映と専用file pickerは未完了。 | K09。現行ProfileからAgent TaskやRuntimeを構成する実consumerは特定できず、N対応を無理に割り当てない。 | Desktop Settings→profile_client→Broker profile_center→profiles.json。create／copy／apply-intent／delete／export／import／listを扱う。現行applyはProfile hashを照合して意図をAuditへ記録するのみで、Runtime operationやPermission ledgerを変更しない。 | Schema未知field・Authority／Credential混入・malformed／duplicate／stale stateを拒否。ProfileはPermission／Approval／Credential／Authorityを生成せず、通常IPCは実際のRuntime適用権限を得ない。Broker persistent stateの不正時は起動をfail-closed。 | Desktop設定consumerのsourceはあるが、Profile値を実Runtime／Taskへ反映する経路、専用file picker、OS別installed acceptanceは未成立。Rust unit／Flutter client test sourceあり、本単位NOT_RUN。現在candidate acceptance EvidenceはUNKNOWN。 |
| C11／Phase 21 | Update Centerで候補一覧／署名検査／適用要求／延期／rollback。履歴では署名検査・永続候補一覧・延期・suspended request・Settings UIまで接続、download／install／process／rollback実行とtrust本番provisioningは未完了。 | N10、K09／K16。 | Settings/update_client→Broker update_center／update_download。Broker所有固定HTTPS sourceから候補取得・署名検証し、download、未起動version stage、別Owner確認後のactive record／Start Menu切替、直前版rollback、限定repair／uninstall操作を持つ。Windows起動器／Bootstrapperが固定rootから次回起動するsource contractがある。 | Flutterはnetwork／filesystem／processを直接扱わない。Broker trust・package digest・fixed root・product identity・Owner要求を再検査し、bad signature／stale trust／unknown pathを拒否。stage済みやUI stateはAuthorityではない。rollbackはrecordを切替えるがprocessをその場で起動せず、完全なinstalled recoveryを意味しない。 | N10のOS候補/package表に従う。現在の実装・Rust/Flutter/update fixture test sourceはあるが本単位NOT_RUN。source/specはdownload・stage・activation・rollbackを含む一方、同一candidateの一般installed lifecycle、配布元／署名trustの本番provisioning、各OS QAは未成立。歴史的C11未完statusは維持。 |
| C12／Phase 22 | Desktop Notification store/surfaceでRuntime／Approval／Audit／Recovery／Device／MCP／A2A／Update／Evaluationを要約。履歴ではAudit由来summary、hash-bound state、IPC、UI、既読／破棄／navigationを接続。Windows native toast・全Runtime source・installed通知証拠は未完了。 | N09、K10／K13。 | Broker確定Audit eventの固定operation mapping→通知summary projection／hash-bound notifications.json state→通常Broker IPC→Flutter通知画面。caller任意notice登録不可。既読／破棄はcurrent notification hashに束縛し、openは既存画面へのnavigationのみ。N09の観測・操作surfaceの一部。 | Audit reason／payload／metadata／Credentialは表示しない。通知からPermission／Approval／Authorityを生成せず、sourceは固定対応のみ。scan 4096件、return 256件にbounded、malformed／stale stateはfail-closed。 | Desktop consumerのsourceあり。Windows native toastはBroker host capability未接続。Mobile summary projectionは独立範囲でありC12 Desktop通知完成を意味しない。Rust unit／Flutter client test sourceあり、本単位NOT_RUN。全Runtime／installed EvidenceはUNKNOWN／未成立。 |
| C13／Phase 23 | Auditと独立したSpan／Trace／Metric観測modelを導入し、初期は内部実装、後続でOpenTelemetry adapterを想定。履歴ではBroker内部span／bounded保持／IPC／Desktop画面を接続、Runtime全体の観測・export・collector・installedは未完了。 | N09、K10／K13。 | Broker Audit finalizationに結び付くbounded ObservationCenter span→Trace／Metric projection→通常認証済み観測一覧IPC→Desktop Observability/Trace screens。Source classはBroker internal stateであり正式Auditと別契約。 | Trace／MetricはAudit proof・Permission・Approval・Authority・Credentialではない。保存・返却はbounded。Runtime／Adapter／Tool／外部通信のspanをBroker spanから推測／複製しない。 | Desktop source・Rust unit／Flutter client testあり、本単位NOT_RUN。Runtime全体の実測、長期保持、OpenTelemetry export／外部collector、installed runtime Evidenceは未成立。 |
| C14／Phase 23 | Read-only Trace InspectorでTraceID filter、waterfall、duration、status、parent、error分類を表示。履歴ではC13一覧再利用とBroker-only境界を接続、Runtime／Adapter／Tool／外部trace・export・installedは未完了。 | N09、K10／K13。 | Observation list→Broker observation_center→Flutter observation client→Trace Inspector。現在はBroker内部spanのTrace filter／waterfall／関連Audit IDを表示する。Runtime／Adapter／Tool／external communicationの実測sourceはない。 | read-only projection。Trace表示はAuditを置換せず、Permission／Approval／Authority／Credentialや過去承認を復元しない。raw payload／secretは非表示。 | Desktop sourceあり。A2A traceはC15以降の実接続に従う。Rust ObservationCenter unit／Flutter observation client test sourceあり、本単位NOT_RUN。外部collector・長期trace・Runtime実span・installed EvidenceはUNKNOWN／未成立。 |

### C7〜C14 Source SHA・test source

下記blob SHAはsource照合基準main f8a6b5ed521495fe7d0a4b5c79292fa3a7c169a0の値。production source内Rust unit testsを含み、別OSのinstalled/runtime証拠ではない。C7〜C11の主要SourceとC12〜C14のSource/Testを列挙する。既存N02/N05/N09節はそれぞれの広域consumer／追加SHAを保持する。

| source file | blob SHA |
|---|---|
| docs/specs/credential-vault.md | f27d54012cf5932356c186d4ebef295dbd535178 |
| specs/credential_registration.schema.json | 65c8bc535e36c040e8574277f06ab1ecb2589d19 |
| specs/credential_receipt.schema.json | 657bfc7917123d212cff462ff7572886effc3457 |
| specs/credential_list.schema.json | 61f706b09533181df57ac781d7eed5adb1199254 |
| specs/credential_revocation_request.schema.json | 43aed5aa2d4f8dc56d9f91386b69b73bed7a3427 |
| native/rust_helper/src/broker/credential_vault.rs | 029dccda7671d51c2bbd3debd4fe5d7e92221162 |
| native/rust_helper/src/broker/credential_storage.rs | 86cf0459915bb748f2d3d5848b20913d9c6080f2 |
| native/rust_helper/src/windows_credential_input.rs | 3f22358ce15f69d34a93ab5f46186374caba13ec |
| native/rust_helper/src/macos_credential_input.rs | f4a44a0dd9d374243e38dab8726bb44c51970422 |
| native/rust_helper/src/linux_credential_input.rs | 8594403a89e4ed1f8eab38b0bb73c62bbceeb79b |
| apps/desktop_flutter/test/macos_credential_vault_test.dart | 5d2ae1900cf31f7ff06ee6236ede132b0e933ea3 |
| apps/desktop_flutter/test/linux_credential_vault_test.dart | 64c250a967fcfbc3b5da941810afbf558178dfe7 |
| docs/specs/mcp-contract.md | 62fffe6af42253af5656ca06f630868487f277bc |
| docs/specs/mcp-connection-center.md | 64d46042f19fccb0f7a947a95711e9aae87a1564 |
| specs/mcp_contract.schema.json | 733f1ecf2ad968e98c7d0c3557e5ec897a4cac23 |
| specs/mcp_connection.schema.json | a8d613bd911c9e88f949b984b1a2032eb19db332 |
| specs/mcp_tool_call.schema.json | 54c044cdfb53588ea5bab4bd056404372dc39acd |
| specs/mcp_tool_call_receipt.schema.json | 0049a393104b035bbf047e653e962754bc4009b3 |
| native/rust_helper/src/broker/mcp_center.rs | 8f9ab897bdc625e448241ebd9b46f995cd1f22f6 |
| native/rust_helper/src/adapters/mcp_stdio.rs | b7e08327541ad75bca53bb1563e0e20d576d9b66 |
| native/rust_helper/src/broker/protocol.rs | 9b2076d5f31ca6d77f1f87973d1b394aaf751961 |
| apps/desktop_flutter/lib/screens/mcp_connection_center.dart | 4c2dd1122e2162a4c3ab30b5c2536e267fee8519 |
| apps/desktop_flutter/lib/services/mcp_connection_client.dart | 12a173a3956368c53615255c56ef06a2f5939c9a |
| packages/agent_orchestration/lib/mcp_connection_client.dart | 71b17973216160bdd2bc447243ba0b04fd0fec63 |
| packages/agent_orchestration/lib/agent_mcp_task_controller.dart | fa46ff071cfb306d3e463a9ad2c79b9149dab6f5 |
| apps/desktop_flutter/test/mcp_connection_center_test.dart | c81a37ca78b15e57c838f48519aa3e4abb6d6665 |
| apps/desktop_flutter/test/agent_mcp_task_controller_test.dart | 5c165d5362eea4906b6e739407b0d0e75c34485c |
| apps/desktop_flutter/test/support/real_broker_mcp_probe.dart | 0cc9aeb2b1ae6f0e14f1573d4e0cbd7aef7fa137 |
| docs/specs/operation-profile.md | c53720b7eb4ad6983d3d84bb6cb3de563936c5e5 |
| specs/profile.schema.json | 8619ff6bb553d2d75a6400fd4a4528ca2b83c733 |
| specs/profile_list.schema.json | df38cbef669704b4489a4111d579e975d91c544f |
| specs/profile_receipt.schema.json | 7b9298f9fc5423051a33bfccc7bd28c4d8e875a6 |
| native/rust_helper/src/broker/profile_center.rs | d56264a07ee9f51ffe25317e5a304aa7b97ae45a |
| apps/desktop_flutter/lib/services/profile_client.dart | ab967d9fb3f84c48b7bbc9a1193fbf7563d99105 |
| apps/desktop_flutter/lib/screens/settings.dart | 66374ec4157d573939e7f9cc5c36aecf5d65e251 |
| apps/desktop_flutter/test/profile_client_test.dart | ba2bba18bf3b7bc299940984d9240cd4eeffd29a |
| docs/specs/update-center.md | be37e1f661cb3e4cb05e7c1242051263665a92b2 |
| specs/update.schema.json | 351a76c29f0279cc2cef92970f082ba7c2f7aaf1 |
| specs/update_candidate.schema.json | 1d803d8a2aaf22576346c77f4c7113ad834d213c |
| specs/update_list.schema.json | b4ff3283dfd7e781bf6af9a0c208caf3aa7ed7e1 |
| specs/update_trust.schema.json | 5caa430cf621c09e6e3d96bd6c377758095b9491 |
| native/rust_helper/src/broker/update_center.rs | f6fc98155fcd476765b60d382a680c7f7c6d675e |
| native/rust_helper/src/broker/update_download.rs | 9f9f9b23113e724a206427bea082a11dd4764115 |
| apps/desktop_flutter/lib/services/update_client.dart | 26d2e946b5b15ea3328867e606e9f75bf4bfc2d1 |
| apps/desktop_flutter/test/update_client_test.dart | f70bb4a6dbc2340b5d26d867496952629dad1218 |
| docs/specs/notification-center.md | 81b4ed30b2defd538a56c9323fecb566a1fad8dd |
| specs/notification.schema.json | 3028750ccf7b647867314664cf5280ea7ec26812 |
| specs/notification_list.schema.json | 0fa037e50d58c10c4b5a7364c0a7472e708996b6 |
| specs/notification_action.schema.json | d7ab880c65799ac8e69a02c1ae1da594989e8f17 |
| native/rust_helper/src/broker/notification_center.rs | ca6828a33baf94abd7101781d9730b22daca4a78 |
| apps/desktop_flutter/lib/services/notification_client.dart | 746c74805ef5b80febf020306daeb360ff43b4e6 |
| apps/desktop_flutter/lib/screens/notifications.dart | 850ac48a96ccff94b64b154a4e38d608aabb5d3c |
| apps/desktop_flutter/test/notification_client_test.dart | e92e994e4a6107abc83de34cfed6032f260dd550 |
| docs/specs/trace-inspector.md | d7ca9524d7e3bf26c6fe03e55c6175ca6abfc61a |
| specs/observation_list.schema.json | 1c4eba48d5b8a3318253e7ba6890305bd5e89e7c |
| specs/observation_span.schema.json | 8cc3b8e66f242e8009db546145ae8f3fa1582786 |
| specs/observation_trace.schema.json | 8365f277eab716618c02e7635eaf5896e6a84820 |
| specs/observation_metric.schema.json | eaa8757dd6b43611728c358a7277ac84efa5f242 |
| native/rust_helper/src/broker/observation_center.rs | 62896e497a0d78267020db6bae2ecadf704fa1fb |
| apps/desktop_flutter/lib/services/observation_client.dart | 97a28fb5aadd82c77028d74928bb7fa86a0220cc |
| apps/desktop_flutter/lib/screens/trace_inspector.dart | c196c8e20786d151edc4b709b71dd3b8fa88812b |
| apps/desktop_flutter/lib/screens/observability_center.dart | 497fe25368b9b108d62bee921d454145b712b657 |
| apps/desktop_flutter/test/observation_client_test.dart | 6d6ac31671e16d771bbc1a39fa7edcd9a3929dfb |

C7〜C14のSource/Schema/test blob SHAは現行source baselineへ固定した。上記test sourceはUNIT／FIXTURE／WIDGET／CONFORMANCEのコード存在であり、この作業単位ではNOT_RUN。C7〜C14のinstalled／実OS acceptance Evidenceは新規発行していない。C7〜C14の旧statusはrev1履歴のまま、v4 N01/N02/N05/N09/N10、K07〜K10/K13/K16はそれぞれの現行ledgerでOPENを維持する。

## 旧C15〜C23のv4 Source→Consumer crosswalk（E0差分）

旧C15〜C23の要求文・Phase対応・rev1進捗statusは履歴として保持し、書き換えない。以下のsource観測baselineはmain `3c188ae54de6ea353490a60d91c1a75757d06e74`。v4対応はS04/S05およびR03上の関係を示すもので、旧statusをN/K受入れへ昇格しない。旧Phase対応は`docs/D4_POCKET_PHASE_MAPPING.md`。Evidence IDのない製品受入れは`UNKNOWN`、記載test sourceはコードの存在であり本mapping単位では`NOT_RUN`。

| 旧ID／Phase | 主語・目的／rev1履歴 | v4対応 | 現行Producer→Consumer・契約・source状態 | 否定・停止・Authority・Recovery | OS・TestClass・Evidence・残差 |
|---|---|---|---|---|---|
| C15／Phase 24 | A2A Contract。Agent Card／Task／Message／Artifact／Streamのmetadata-only Schema、fixture、Authority Strip、Trust非生成を接続した履歴。実interface・作用は当時未完。 | N04、K11。選択済みtext ArtifactのhandoffはK02とも関係する。 | A2A契約SchemaとRust `a2a`／Broker `a2a_center` consumerがある。現sourceはbounded Agent Card・SendMessage・SendStreamingMessage・GetTask経路へ拡張されている。Contract／Schemaの存在だけを外部Agent相互運用の受入れとしない。 | Agent Card／metadata／CapabilityはTrust・Permission・Approval・Authorityを生成しない。本文やArtifactは未信頼入力。未知・不正な構造と曖昧な外部作用は拒否または`unknown/suspended`で停止し、自動再送しない。 | Desktop sourceはWindows/Linuxでnetwork作用を持つ。macOSは現行規則によりloopback Agent Card／metadata-only、Message等の作用なし。Mobile consumer未確認。Schema／Rust／Flutter fixture test sourceあり、本単位`NOT_RUN`。外部Agent installed evidenceなし。 |
| C16／Phase 25 | A2A接続センター。旧進捗はOwner Agent Card取得、loopback HTTP、receipt／一覧／永続metadata、秘密・endpoint・Authority拒否を記録し、HTTPS／作用／Artifact／Stream／quarantine／Desktop画面を未完としていた。 | N04、K11、K02。 | Flutter A2A Center→`A2aConnectionClient`→Rust Broker `a2a_center`→`a2a.rs`。現sourceには各要求のnative確認・one-shot Permission・Auditを伴うSendMessage／Stream／GetTaskもある。選択済みtext Artifactは別のHandoff package／Task preflightへ渡すsourceがあるが、外部Agent installed end-to-endではない。 | 要求hash・現在endpoint・payloadを束縛し、外部作用後のtimeout／不正応答／Audit不確定は再送不能で停止する。外部内容をAuthorityとして扱わず、Handoff先でfresh Permission／Approvalを要求する。Credential実値注入は未対応。 | Windows/Linuxのbounded HTTP(S) source、macOS metadata-only、Mobile unknown。Rust loopback／Flutter fake Broker・widget／Handoff fixture sourceあり、本単位`NOT_RUN`。同一installed candidateでの外部接続→作用→Artifact→受信Task→Audit証拠なし。 |
| C17／Phase 2・26 | 複数Host registry。Owner登録、通常一覧、hash-only identity、bounded永続化、再起動fail-closedを実装した履歴。C18 UIへ接続。 | N04、K11。 | Owner controlのHost登録→Rust `host_center`→bounded `hosts.json`／Audit、通常IPCの一覧→metadata projection。Host identity／Runtime・Agent数は登録metadataであり、remote live consumerはない。 | Host／identity hash／登録時Trust／countはTrustやPermissionを生成しない。登録は`pending_review`・metadata-only。壊れたstate／unknown field等はBroker起動時fail-closed。Host間Authorityは共有しない。 | Desktop Broker registry source。Rust unit／Flutter fixture sourceあり、本単位`NOT_RUN`。live remote discovery／接続／失効・quarantine、installed Host証拠なし。 |
| C18／Phase 26・28 | Host操作面。Host一覧／状態／Trust表示、Host切替、Runtime／Agent観測境界、Host間非混線を接続した履歴。live再接続・Trust検証・remote discoveryは未完。 | N04、N09、K11、K13。 | Flutter `HostOperationCenter`／`HostCapabilityCenter`→Host registration client→Broker Host list／switch／capability consumer。Host切替は表示contextであり、個別remote Runtime／Agent列挙やTask routeではない。Broker現在Hostと一致しないHostは未観測として表示する。 | 別HostへPermission／Approval／Credential／Authorityを持ち越さない。登録metadataからTrust verifiedや接続を推定しない。未知Host・不一致receipt・native Owner拒否は切替成功へ昇格しない。 | Desktop UI source。Linux/macOS公開metadata登録経路のsourceあり。Widget／client fixture sourceあり、本単位`NOT_RUN`。remote Host再接続・Host越境Task consumerなし（`BUILD_GAP`）、installed evidenceなし。 |
| C19／Phase 27 | Adapter管理。Catalog、Broker Owner操作、署名検査／隔離、Desktop Runtime Centerを接続した履歴。外部artifact作用・installed evidenceは未完。 | N03、N10、K06、K12。 | Runtime Center→通常Broker IPC／限定Owner経路→Rust `adapter_center`→bounded catalog／Audit。署名検査・enable等はcatalog recordを更新する。現契約では外部artifact download／filesystem install・delete／process startを行わない。 | Adapter／Manifest metadata・署名宣言・catalog状態はPermission／Approval／Credential／Trustを生成しない。未検証Adapterはenable拒否。隔離Runtime IDはTask/lifecycle再利用しない。 | Desktop UI／Broker source。`widget_test.dart`を含むtest sourceは本単位`NOT_RUN`。外部artifactの実作用と各OS installed lifecycleは未成立。 |
| C20／Phase 28 | Windows native tray。表示／前面化／status・件数projection／stop request receipt／exit入口を接続した履歴。installed product・実Runtime停止は未完。 | N09、N10、K13。 | Windows tray controller→Flutter `windows_tray_client`／Broker-derived projection。停止操作はowner再確認待ちの要求receiptを返し、trayからprocess killやRuntime停止を実行しない。終了はDesktop UI exitであり、Runtime停止完了を意味しない。 | 取得不能値を0にしない。stop receiptは`executed=false`を保ち、UI要求を停止成功・Permission・Approvalとして扱わない。 | Windows native sourceのみ。Flutter client test sourceあり、本単位`NOT_RUN`。Windows installed lifecycle evidenceなし。非Windows tray未提供は既存scopeとして保持し、Linux Desktop対応全体の可否へ一般化しない。 |
| C21／Phase 28 | Command Palette。rev1当時のscopeを実装し、全機能実行とWindows installed実証を未完とした履歴。 | N09、K13。 | `main.dart`のCtrl+K／Ctrl+P→最大30件の画面候補→既存surface navigation。MCP／Credential／Update／Host等の項目は各画面を開く入口であり、処理を直接実行しない。 | 選択は画面遷移だけ。通常IPC、Owner control、filesystem、process、network、Credentialへ直接到達せず、Permission／Approval／Authorityを生成しない。 | Desktop Flutter widget source/testあり、本単位`NOT_RUN`。実Runtime操作やinstalled keyboard evidenceへ昇格しない。 |
| C22／Phase 28 | Global Search。boundedな表示用snapshot／surface entryを検索する旧scopeを実装し、live横断取得・installed実証を未完とした履歴。 | N09、K13。 | `ShellSnapshot`→`GlobalSearchIndex`→全体検索UI。query 128文字、index 512件、結果30件にbounded。Broker由来と確認できないsnapshotは不明と表示し、固定surface entryは操作面の存在だけを示す。 | read-only navigation。対話本文、Approval/Audit raw payload、秘密値、任意pathをindexしない。検索結果はBroker live query・権限・承認・作用を起動しない。 | Desktop Flutter unit/widget sourceあり、本単位`NOT_RUN`。live cross-Broker searchとinstalled evidenceなし。 |
| C23／Phase 28 | Desktop UX統合。既存画面を論理グループへ整理し、画面到達性・UI境界を維持する旧scopeを実装した履歴。Windows installed実証は未完。 | N09、N10、K13。 | `main.dart`のNavigationRail group selection→Flutter内表示状態→既存screen navigation。現specは21画面を全体表示で保持し、運用／安全／開発／設定へ絞り込む。palette／search等からの遷移も維持する。 | グループ選択はUI stateのみ。画面、権限、Broker経路を削除・変更せず、filesystem／process／network／Credentialへ到達しない。 | Desktop Flutter widget sourceあり、本単位`NOT_RUN`。Windows installed UI evidenceおよび他OSの標準installed UX evidenceは未成立。Mobile navは別範囲。 |

### C15〜C23 source blob SHA・test source

下記Git blob SHAは照合基準main `3c188ae54de6ea353490a60d91c1a75757d06e74`の値。これらは仕様・schema・source・test fileの同一性を固定するもので、実行やinstalled acceptanceを証明しない。重複する`main.dart`／`widget_test.dart`は一度だけ記載する。

| 旧C範囲 | Source file | Blob SHA |
|---|---|---|
| C15–16 | `docs/specs/a2a-contract.md` | `abe32b25bf7c186faf6e1beb3ae3f5d754de61c5` |
| C15–16 | `docs/specs/a2a-connection-center.md` | `a2f815d61aed80b4e1f619c0a2f947960d7bd317` |
| C15–16 | `docs/specs/a2a-streaming.md` | `2b1cd55f29cd9c7f4af177d2703b677071e9e96a` |
| C15–16 | `docs/specs/macos-a2a-center.md` | `6f5bc7444f2e406aa5c7d21d1aea1f7599c743ad` |
| C15 | `specs/a2a_contract.schema.json` | `8c7f50bb7fb12dde41293727e5dc9109445d1bc3` |
| C16 | `specs/a2a_connection.schema.json` | `1dbbe44e54b302a1196d7e4e55f1cae35cda569b` |
| C16 | `specs/a2a_connection_list.schema.json` | `ee18b204349b025464ca3af8adaa355add34e650` |
| C16 | `specs/a2a_connection_receipt.schema.json` | `f63fd0d61b6200cdd8cc4b23d5cf7968a82fa5c5` |
| C16 | `specs/a2a_message_send_request.schema.json` | `0a5f87250f17a88b6be55e7838d32b8e9114bbed` |
| C16 | `specs/a2a_message_send_response.schema.json` | `23ee690e0ce350ac94c64d86f6920afb63c199f6` |
| C16 | `specs/a2a_stream_send_request.schema.json` | `e0a0841f2fd9f7e92e0fb8a036ebaa6852e198d8` |
| C16 | `specs/a2a_stream_send_response.schema.json` | `a526fed89fb5b3766a03c52444a338060caf16cc` |
| C16 | `specs/a2a_task_get_request.schema.json` | `df11a5fd3291de0ae379694c6c1b49de51f2f157` |
| C16 | `specs/a2a_task_get_response.schema.json` | `56a55f348d6b0ba7cb0092ee7ea2b4ec809e5360` |
| C15–16 | `native/rust_helper/src/a2a.rs` | `9d7c6d9ab938ad5178d6ca46b3933dc3e61c6061` |
| C16 | `native/rust_helper/src/broker/a2a_center.rs` | `73ad601abd64a33ccefde4b73ab5f9dc4f728504` |
| C16 | `apps/desktop_flutter/lib/screens/a2a_connection_center.dart` | `2716d1c2fa22a184044e6f4e2eb401cb33c6e011` |
| C16 | `apps/desktop_flutter/lib/services/a2a_connection_client.dart` | `19335054afe35bc06597629d92ca2e635ce50069` |
| C16 | `apps/desktop_flutter/lib/services/a2a_artifact_handoff.dart` | `e7e2f3454e2843f1d4f4e7b5aedfa55014cde05c` |
| C15–16 | `apps/desktop_flutter/test/a2a_connection_center_test.dart` | `5ebdbb4cee62e581062370ed6c305c36df89b7e2` |
| C16 | `apps/desktop_flutter/test/a2a_artifact_handoff_test.dart` | `e47005320917f59dfa55ae1e04c4b99d452676de` |
| C17–18 | `docs/specs/host-registry.md` | `a8a94c5c656ca74e016294bd708525bc1909aa34` |
| C18 | `docs/specs/host-operation-surface.md` | `149ee962e8d464615ec375d9baa81c1d1b0e7679` |
| C18 | `docs/specs/macos-host-registration.md` | `c7b379163a4aec66ef8cfe178034439afae1fcd8` |
| C17 | `specs/host_registration.schema.json` | `8bed26fb2fe8f8e296c1d3404978f3215d571240` |
| C17 | `specs/host_receipt.schema.json` | `237a84717cf27d000357d0dac53d33dd6f535766` |
| C17 | `specs/host_list.schema.json` | `93880c43b04a5408c033bf351ee695073e54664a` |
| C18 | `specs/host_switch.schema.json` | `01e5fc7322e8d4d2c9142375e7641fc4255148d5` |
| C18 | `specs/host_switch_receipt.schema.json` | `cb60df0b174a38c871e0e8835c65537a17d022fb` |
| C18 | `specs/host_capability.schema.json` | `6b721b5ab7842edc7df6562bc56fdd5b73d5de79` |
| C17 | `native/rust_helper/src/broker/host_center.rs` | `c27707faa61e5270b1c6d79e3ee3c5fe6ccd0bf8` |
| C18 | `native/rust_helper/src/broker/host_capability.rs` | `08a82c7e52947439229447dc82b69097af67ba1f` |
| C18 | `apps/desktop_flutter/lib/screens/host_operation_center.dart` | `e16605e6bb39bbfc12806e6154e08d6fbab1b93b` |
| C18 | `apps/desktop_flutter/lib/screens/host_capability_center.dart` | `e8fa774a435e713a3b75b0d33f00bd500b0a2d7f` |
| C17–18 | `apps/desktop_flutter/lib/services/host_registration_client.dart` | `3bc3b45bddddf4e1131e051acb3cdee599ca4d3d` |
| C17–18 | `apps/desktop_flutter/test/host_registration_test.dart` | `74ab39e078bf72214a3bbe84f69ebc77fceb2493` |
| C19 | `docs/specs/adapter-management-surface.md` | `d13859c61b7042d7d5492feea1cdbc1662eee398` |
| C19 | `docs/specs/adapter-conformance.md` | `e095d9f3bf9d4e6931dac193a304f43d75355c3d` |
| C19 | `specs/adapter.schema.json` | `0fee764bc04ad524665e4be2b02828dcdb488f73` |
| C19 | `specs/adapter_manifest.schema.json` | `116f5830983d195f501cc344a25990d58bf323c0` |
| C19 | `specs/adapter_management_manifest.schema.json` | `1f1b75c7d4299c96fcf5039b652d718f7805cd2d` |
| C19 | `specs/adapter_management_list.schema.json` | `530bc79a6a5117bb29d0de9e0932e3c40d8d785d` |
| C19 | `specs/adapter_management_request.schema.json` | `2c9c98d0c60603adfaa246f9efa4753bfc0e7597` |
| C19 | `specs/adapter_management_receipt.schema.json` | `7b43a2d2b74c5bd0a931ead66f32f55663919b04` |
| C19 | `native/rust_helper/src/broker/adapter_center.rs` | `66575bb77e4a375dca4f8488f97df442960bb5a9` |
| C19 | `apps/desktop_flutter/lib/screens/runtime_center.dart` | `d2c7af172073d2b10c76a6416b90f011f10e95db` |
| C19 | `apps/desktop_flutter/test/widget_test.dart` | `f4e9c7b93f832ba2a2cefee779fd12426f240b04` |
| C20 | `docs/specs/windows-tray-surface.md` | `c7f9317e26b16098301989a8f4b5eaa11540e714` |
| C20 | `specs/windows_tray_projection.schema.json` | `5c3975e03b34281a0f202374cf91127357f2a1e0` |
| C20 | `specs/tray_stop_request.schema.json` | `579755262940027079b37ef859e6d2f32a071cc0` |
| C20 | `specs/tray_stop_response.schema.json` | `d6cce063d5deaf7104b97ef67f4efb14f780cc72` |
| C20 | `apps/desktop_flutter/lib/services/windows_tray_client.dart` | `cf8214ceb95fa1b24c7eed4f14b312bda80e0b15` |
| C20 | `apps/desktop_flutter/windows/runner/tray_controller.h` | `67ec337a335b948115559394ad5b55bffadb7502` |
| C20 | `apps/desktop_flutter/windows/runner/tray_controller.cpp` | `2676afcc76e5c6e53fff2dd00d0fbb3ae531ccc1` |
| C20 | `apps/desktop_flutter/test/windows_tray_client_test.dart` | `0b24e07221087e43355ac465ca4ace73a48af31e` |
| C21 | `docs/specs/command-palette-surface.md` | `05867c94592a66b3a0d788d6f29456bb9ba6bc15` |
| C21–23 | `apps/desktop_flutter/lib/main.dart` | `6109c821ab5dc809224bec5234c4c59db1d5f988` |
| C21 | `apps/desktop_flutter/test/module_pruning_test.dart` | `20470e523c2640b681e54c51f9c6749ee35f5e05` |
| C22 | `docs/specs/global-search-surface.md` | `1a74b337e239dc7c23cf70d2f6dbc6f55ac5234e` |
| C22 | `apps/desktop_flutter/lib/services/global_search_index.dart` | `f71d2f6c4563b011fa3621ea3aee596bffb099f0` |
| C22 | `apps/desktop_flutter/test/global_search_index_test.dart` | `2b96129f13ab11bd38afb0aba5e9a5b07d8eb3e3` |
| C23 | `docs/specs/desktop-ux-integration.md` | `b73ec534c2b4df94ca7eb52be5d7929cf0a20aa1` |
| C21–23 | `apps/desktop_flutter/test/widget_test.dart` | `f4e9c7b93f832ba2a2cefee779fd12426f240b04` |

C15〜C23の旧statusはrev1履歴として保持した。source/schema/test blob SHAとSource→Consumer対応は現行候補の同一性を示すだけで、test実行や製品Acceptance evidenceではない。A2A／Host／Adapter／Desktop surfaceの製品受入れは未判定、N03/N04/N09/N10および関連KのOPEN状態を維持する。C0〜C34の35 IDはcrosswalk済みだが、全E0-04列、Phase/K等の全数照合とE0完了は未成立である。

## 旧C24〜C34のv4 Source→Consumer crosswalk（E0差分）

旧C24〜C34の要求、Phase、進捗statusはrev1履歴として保持し、再判定・書換えをしない。以下のsource観測baselineはmain `d45fa6109fa6300ed42449ea03a93bfe25f7ae95`。v4対応はS04/S05・R03上の対応関係であり、旧statusはN/K受入れへ昇格しない。Test sourceは存在と種類だけを示し、このmapping単位では`NOT_RUN`。Phase対応は`docs/D4_POCKET_PHASE_MAPPING.md`。

| 旧ID／Phase | 主語・目的／rev1履歴status | v4対応 | 現行Producer→Consumer・契約・source状態 | 否定・停止・Authority・Recovery | OS・TestClass・Evidence・残差 |
|---|---|---|---|---|---|
| C24／Phase 35 | Mobile補助操作面。旧進捗は9画面、通知summary、Runtime／資源／履歴metadata、MCP未観測、停止receipt、Recovery導線、Device Link/Broker read-only接続、Conformance・Mobile/Rust試験を記録。実機・TLS／Windows installed連携は未取得。 | N09/N10、K03/K04/K13/K17。 | Mobile画面→`MobileProjectionClient`／`DeviceLinkClient`→OS-native Device Link TLS service→既存Rust Broker read-only handler。Agent statusもmetadata projection。新Authority経路を増やさない。 | Approval画面はowner操作の表示に限定。MCPは未観測。停止は`executed=false`・owner reapproval receiptで実停止なし。履歴本文・Credential実値・Workspace pathを返さず、資源unknownを0にしない。Agent metadataはTask capabilityやTrustではない。 | Android Kotlin／iOS Swift native source、Flutter client/widget tests、Android Emulator/iOS Simulator workflow sourceあり。本単位`NOT_RUN`。Hostのlive inventory、Mobile AgentTask開始、各標準OS installed product／Device Link統合は未成立。物理端末はv4必須条件ではなく、標準Emulator／Simulator evidenceも本単位では未実行。 |
| C25／Phase 36 | Apple補助検証。rev1履歴は`workflow_dispatch`のMac build補助を保持し、Windows hostからの起動実証・現行commitのmacOS/iOS Simulator evidenceは未取得。 | N10/N11、K17/K19。 | `.github/workflows/apple-manual-build.yml`にmacOS/iOS target別manual build・test入口がある。workflow定義は実行結果ではなく、target OS consumerへの補助経路。 | Dispatch入力があることだけでOS build成功・製品実行・署名・配布を主張しない。失敗時はworkflow結果を対象SHA／runner／targetごとに限定し、release readinessへ昇格させない。 | macOS runner／iOS Simulator候補のmanual source。workflowは本単位`NOT_RUN`。現行candidateのmacOS／iOS標準installed evidence、正式署名／notarization／配布は未成立。物理iPhone要件へ戻さない。 |
| C26／Phase 35・36 | Android build回帰。rev1履歴ではWindows Flutter analyze、32試験、debug APK/AAB buildをPASS。Android install／launch、native安全保管、正式署名は未取得。 | N10/N11、K17。 | Android Gradle設定と`android-manual-emulator.yml`はAPI 35 Google APIs x86_64 Emulator、Flutter analyze/test、debug APK/AAB build、Emulator install/launch、native Device Link product flowを記述する。これは実行可能なmanual validation consumerで、当該workflowの現candidate結果ではない。 | Debug APK/AAB・source testは署名済通常配布やKeystore保証ではない。Native Device Linkの拒否／秘密境界はBroker/OS経路で維持し、MobileからAgentTask／MCP Tool作用を生成しない。 | Windows過去debug buildは`SOURCE`／`FIXTURE`履歴。Emulator workflow sourceあり、本単位`NOT_RUN`。現行candidate Emulator installed path、更新／撤去、標準配布identityは未成立。物理Androidは凍結条件と独立してEmulator検証を利用できるが、ここでは実行していない。 |
| C27／Phase 41 | 開発用全数性能・UI停止監視。旧履歴はsnapshot、Flutter projection、4096行差分等のbounded測定をPASSし、実installed起動・GPU frame・8時間・実Runtime負荷を未測定と記録。 | N09/N11、K19。 | `performance_validation.py`→generated snapshot／`c27_performance_validation_test.dart`のFlutter projection／表示処理をwatchdog測定する。現consumerは内部snapshotとfixtureであり、実installed product telemetryではない。 | watchdog超過を失敗として返し、resource unknownを0にしない。fixture／内部計測の成功をLIVE_RUNTIME、性能SLA、GPU frame保証へ昇格しない。 | TestClassは`INTERNAL_STATE`＋`FIXTURE`の開発測定。test sourceは本単位`NOT_RUN`。実installed cold/warm startup、実画面frame、GPU、実Agent/MCP/A2A負荷は未成立。 |
| C28／Phase 39 | Windows長時間運転。旧履歴は実Broker IPCの短時間smoke／120秒stressをPASS、clean commitの8時間試行が開始22.766秒で失敗したと保持。 | N01/N08/N11、K19/K20。 | `long_run_validation.py`は8時間実行option、Broker restart／network disconnect等の監督、結果JSON／failure classificationを持つ。Runtime HTTP応答はlocalhost fixture。短時間smokeと8h candidate runは別consumer/evidence。 | timeout、client受信未確認、Audit/state失敗を成功へ読み替えない。fixture serverのflush記録はclient受信証明ではない。資源不明を0にせず、期限後の資格file cleanupは固定回数だけ再試行し、未成功ならfailed。 | Brokerは`LIVE_RUNTIME`を限定観測し、Runtimeは`FIXTURE`、resourceは`INTERNAL_STATE`。8時間の旧FAILは保持し、PASSへ変更しない。current installed productの8h／実Runtime負荷は未成立。本mapping単位では長時間試験を再実行しない。 |
| C29／Phase 40 | 開発用障害注入。旧進捗はRuntime/Broker/MCP/A2A/Credential/store/Audit/stateの8件をPASS、installed／外部サービス証拠なしと記録。 | N01/N02/N04/N05/N08/N11、K19/K20。 | `failure_injection_validation.py`が実Broker IPCとlocal MCP stdio／A2A loopback fixtureへ限定障害を注入し、失敗を成功・権限・Audit・Recoveryへ昇格しないことを検査する。 | deny/fail-closed、曖昧結果停止、再送禁止、credential非露出を期待する。fixture・開発Validatorの成功を実外部Runtime／MCP／A2Aやinstalled製品の異常系へ転用しない。 | TestClassはlocal Broker／`FIXTURE`中心。spec／script sourceあり、本単位`NOT_RUN`。実installed fault injection、外部service、全OS標準packageのfailure/recoveryは未成立。 |
| C30／Phase 7・8・42 | Local全数回帰matrix。旧履歴はTrust／Authority／Permission／Approval／Audit／Recovery／Evidence／Runtime／Agent／Dialogue／Compare／Device Link群の回帰実装、実Runtime・実Agent・実端末・installed product未成立を記録。 | N11/K19。個々のregression行は対象N/Kのconsumerへ結合する。 | `full_regression_validation.py`は固定された開発回帰matrixを実行し、`full-regression-validation.md`とConformanceがentry／分類を検査する。これは全要求の実物QAや全OS installed回帰とは別。 | unknown／不正入力・権限昇格・Audit/Recovery境界違反を拒否するが、matrixの列挙やexit 0だけで未実行consumerをPASSにしない。TestClass、対象SHA、OS、NOT_RUN範囲を分離。 | Local source/regression／Conformance。現candidateの全製品testは本単位`NOT_RUN`。外部Runtime／Agent、標準VM/Emulator/Simulator installed path、Windows owner physical evidence、FQ/Q正式QAは未成立。 |
| C31／Phase 1・43 | 製品文書同期。旧履歴はREADME／ROADMAP／GUI操作面／SECURITY／CONFIG／MOBILE_STATUS／COMPATIBILITY_MATRIX／REV2_PROGRESSをC24〜C30のscope・残差へ同期した記録。 | N11/K19。文書は仕様／工程／進捗／Release判断の説明consumer。 | 旧対象文書は現repoに存在する。現状の施工正本は`docs/QE_2026-11-13/`三点・採用記録・進捗、製品各文書はREADME、ROADMAP、GUI_OPERATION_SURFACES、SECURITY、CONFIG、MOBILE_STATUS、COMPATIBILITY_MATRIX、REV2_PROGRESS。文書更新は実consumer動作の代替ではない。 | 旧文書やC31記載がv4正本・現HEADと競合した場合、現v4を優先し、過去status／failureは履歴として残す。未成立を文言変更でPASS化しない。 | Markdown／Registry/Manifest consumer。過去の文書更新statusはその時点の履歴であり、現在の五OS製品QAやrelease成立を示さない。本文の現行整合はこのcrosswalk以外の全ファイル監査を完了した意味ではない。 |
| C32／Phase 42・44 | rev1 C0〜C31対応表の最終開発監査。旧履歴はchecker／Conformance PASS、C32〜C34・installed product・external/physical evidence・release未成立。 | N11/K19。 | `final-development-audit.json`の監査母集団はC0〜C31。`final_development_audit.py`は必須field、参照path、C番号、残余risk分類を照合し、C32〜C34を未監査として明示する。 | report `passed`は対応表構造／参照整合のみで、C0〜C31製品挙動・実Acceptance・release_readyを証明しない。owner GOを生成しない。 | TestClassはstructure/Conformance。監査tool／unittest sourceは本単位`NOT_RUN`。v4 E0全ID、P/Phase/K/BUF/Q/FQ/W/OS/Artifactの一体監査は未成立。 |
| C33／Phase 34・44 | Windows最大到達点。旧履歴はRelease build／Broker smoke／collector／ConformanceをPASSした一方、Windows installed product総合evidenceはUI surface、Setup Doctor path、外部Audit anchorによりfailed。 | N10/N11、K16/K19。 | `stage_installed_app.ps1`→installed app/Broker→`collect_installed_smoke.ps1`／`windows_release_evidence.py`／`release_smoke.py`が候補経路。現registry consumerはcurrent installed evidence fileを要求する。tracked proof-pack内の旧Windows evidence copyはsource commit `28a700f…`・2026-06-07由来で、現main candidateではない。`release_evidence/`には`.gitkeep`以外の現candidate bundleがない。 | Collectorはsource／artifact／profile／runtime/config/audit pathのprovenanceを検査する。過去のsubgate PASSを総合PASSへ合成せず、古い証拠や別commitのartifactを現候補へ再結合しない。Windows Owner physical confirmationとAudit anchorは別々に保持。 | Windows release source・旧`EXTERNAL_EVIDENCE` copyあり。ただし旧C33 total status failed、現candidate installed product evidenceなし。本単位`NOT_RUN`。通常installer→first-run→Task→update/repair/rollback/uninstall、clean source identity、Windows所有者PC確認は未成立。 |
| C34／Phase 45 | 正式release前作業。rev1進捗は未着手。原要求はPublisher identity、正式署名、Audit offline signature、実機／標準OS evidence、strict release、Owner GOを含む。 | N10/N11、K16/K17/K19。 | `release_blockers.registry.json`→`release_gate_check.py`／Strict Release／`FINAL_QA_QUEUE.md`／`RELEASE_CHECKLIST.md`がrelease decision candidateを検査する。Gateは証拠・blockerを評価するがPublisher identity、production key、offline signature、Owner GO自体は作らない。 | 18 blockerの16 unresolved／2 resolvedと`release_ready=false`を維持。正式identity／署名鍵／Audit production key／公開／Owner GOは本人決裁と必要資格なしに実行しない。技術的release prepは独立作業として継続可能。 | Registry／Validator sourceはあるがC34実施・strict release・配布・Owner GOは未成立。responsible decision／production credential項目は未取得のまま。ここではstatus変更もrelease actionも行わない。 |

### C24〜C34 source blob SHA・test source

下記blob SHAは照合基準main `d45fa6109fa6300ed42449ea03a93bfe25f7ae95`の値。source／validator／過去evidence fileの同一性を固定し、動作や現行candidate受入れを証明しない。旧C24〜C34のstatus・FAILは`docs/総合機能拡張_rev1/`の履歴値のまま維持する。

| 旧C範囲 | Source file | Blob SHA |
|---|---|---|
| C24 | `docs/specs/mobile-surface.md` | `071ee69485ede6f792d303dd7fea615d1d912607` |
| C24 | `apps/mobile_flutter/lib/services/mobile_projection_client.dart` | `ba82e4ddeaa5692793237a1bc3258ce53b7aa3ed` |
| C24 | `apps/mobile_flutter/lib/services/device_link_client.dart` | `e8c49337127cb7f62fad248988b99707c5e49219` |
| C24 | `apps/mobile_flutter/lib/screens/device_connection.dart` | `4adb15803bf6b208c13aca930bb5496f03b4d6b8` |
| C24 | `apps/mobile_flutter/lib/screens/agent_status.dart` | `52e4907897c2bb3674bb8523204a787d3a6cb002` |
| C24 | `apps/mobile_flutter/test/mobile_projection_client_test.dart` | `74661d2f2e58046a83e99f711435e4c1b2a74b08` |
| C24 | `apps/mobile_flutter/test/device_link_test.dart` | `509b4febc424d1c0fd330b71529b00457b5cf429` |
| C24 | `apps/mobile_flutter/android/app/src/main/kotlin/com/example/gui_shell_mobile/DeviceLinkNativeService.kt` | `0fd3c6f5bd09e7a18df9fefe97e2e7bb9e9da706` |
| C24 | `apps/mobile_flutter/ios/Runner/DeviceLinkNativeService.swift` | `9d185729c439934dce0553b490f2fcd1f7b28a0d` |
| C24 | `specs/mobile_agent_list.schema.json` | `61558ea30f269c886d40f414cfe31bda09fa5d11` |
| C24 | `specs/mobile_workspace_selection.schema.json` | `6963dbea6db4e876340ea40bbd13a394fae92e23` |
| C25 | `.github/workflows/apple-manual-build.yml` | `ed60235da7b35cdfaac23a9fd2d07023e1d29843` |
| C26 | `.github/workflows/android-manual-emulator.yml` | `f0c1ecb7185c91375adfbbdbfaf49e55a2672b57` |
| C26 | `apps/mobile_flutter/android/app/build.gradle.kts` | `fe96edfa0f714c2d2be509ce3bc21bec6253a714` |
| C26 | `apps/mobile_flutter/android/app/src/androidTest/kotlin/com/example/gui_shell_mobile/NativeDeviceLinkProductFlowTest.kt` | `4f4a79e0bd80b1f5eb280857dec7a7d5df12258c` |
| C27 | `docs/specs/performance-validation.md` | `6746542d94a1f417d443f1a3992779e6480e74ed` |
| C27 | `tooling/performance_validation.py` | `d0744916a8be65e10b61a453864bb8b80318241e` |
| C27 | `apps/desktop_flutter/test/c27_performance_validation_test.dart` | `e833fbd760a8d2e6c4c90e4c6bee6a8c3cb7cff1` |
| C28 | `docs/specs/long-run-validation.md` | `ebba5dc1cc97fa69525502d12dfc059e89023380` |
| C28 | `tooling/long_run_validation.py` | `00d1ec4fa0a7b7fea4ef678a94a56f20367a243c` |
| C28 | `tooling/conformance_tests/test_long_run_validation.py` | `3976e57497d86aa8a3ca612758227c091db617cb` |
| C29 | `docs/specs/failure-injection-validation.md` | `14d2a946f424b0e800a4a271c51da8b5f797dfac` |
| C29 | `tooling/failure_injection_validation.py` | `cace434de47d60f7f848656f229336d8293908ff` |
| C29–30 | `tooling/conformance_tests/run_conformance_skeleton.py` | `c6f31d8cb561b48513c6784e54c85969c9cb953e` |
| C30 | `docs/specs/full-regression-validation.md` | `dd5953d3bea595c2f25474ed0a9c42cf01ff51e3` |
| C30 | `tooling/full_regression_validation.py` | `8a33a9fab9ecbc18119302ae312201d41618012e` |
| C31 | `README.md` | `38664991fe614acfc516eb6743dde6dce754191c` |
| C31 | `ROADMAP.md` | `6a922a38cca617690bc08297274e907f0425b722` |
| C31 | `docs/GUI_OPERATION_SURFACES.md` | `4b48d55a69866dd0e1b3180e2192fc9efcc90598` |
| C31 | `SECURITY.md` | `6784fb68043aaceccc6d063b253c5d7d50210065` |
| C31 | `CONFIG.md` | `7349ca355614fa265a80c104f2be3b3503a92590` |
| C31 | `MOBILE_STATUS.md` | `476b565a3f583be52ef58966f67c51abbb774aa8` |
| C31 | `COMPATIBILITY_MATRIX.md` | `c4885d5b67202712c0bcdecc6da634cb778cd6d3` |
| C31 | `docs/REV2_PROGRESS.md` | `56f0f168b22d1e64303db2bc6f2c66668393dba2` |
| C31 | `docs/QE_2026-11-13/進捗.md` | `d75e96eb788a6f980465575337a1afcca3dc135c` |
| C32 | `docs/specs/final-development-audit.json` | `8126944c55339a3be272af5ba183a9bc2e3517dc` |
| C32 | `docs/specs/final-development-audit.md` | `cc3734d7647dbbbbd27bad49153f775bfc980f3f` |
| C32 | `tooling/final_development_audit.py` | `913c81bc22f71bc5bb3447fa4f4bd266326932ec` |
| C32 | `tooling/conformance_tests/test_final_development_audit.py` | `628c77a50adf867e87b4e4a893d5e691df8111d1` |
| C32 | `docs/総合機能拡張_rev1/受領記録.json` | `e8433d4b4769200c00f1aa0c6e1b4c7671229352` |
| C32 | `docs/総合機能拡張_rev1/進捗.md` | `1392588f0cd5273013578e84d8e33c5c9dc9a01a` |
| C33 | `docs/WINDOWS_RELEASE_PLAN.md` | `5d4c35f8c9599c9f38b7c02ae475710e8c5a64e8` |
| C33 | `docs/WINDOWS_RELEASE_EVIDENCE.md` | `9c836b028c352da8d37f9feae53dcb27b537199d` |
| C33 | `tooling/windows_release_evidence.py` | `6f7957c6193c86c7bc15165e743f11651464f29c` |
| C33 | `tooling/release_smoke.py` | `e9f38cea998a31bbf9272880dbf4965de5eb0113` |
| C33 | `installer/windows/stage_installed_app.ps1` | `c6d0797299624d69b71042416400b2f454787cc7` |
| C33 | `installer/windows/collect_installed_smoke.ps1` | `92c4720e82eb0af91002659c742f8e03b15aa2de` |
| C33 | `installer/windows/collect_broker_smoke.ps1` | `f5f8abfad981d64c30a6cf1f4e9eafde1b43c357` |
| C33 | `openai_submission_assets/windows_proof_pack/evidence_copies/windows_installed_smoke.json` | `7f7f1742460e31c89ecc353fb4b9411c1118f5a5` |
| C34 | `tooling/release_gate_check.py` | `22a8d7bfe9adb259f44ce8501fb215652e1d58c6` |
| C34 | `release_blockers.registry.json` | `244adc04a01e7b406c4ebcdeb829f7077d4aa35f` |
| C34 | `docs/FINAL_QA_QUEUE.md` | `7b332cccab70bb68aa1ff4f8e23acc0e816d8282` |
| C34 | `docs/RELEASE_VALIDATION.md` | `b4b0462e628ad8176264473879dedb1f6201da7c` |
| C34 | `RELEASE_CHECKLIST.md` | `35f31043a9311d49c94acdc9631e30b61949c65d` |
| C34 | `tooling/evidence_bundle.py` | `68dead851236b0f1b69899151c37371e593ff3f9` |
| C34 | `tooling/release_runtime_assertions.py` | `0a0a351fe17151ad86517bd0a965358f40c61490` |

C24〜C34のSource／Schema／validator／旧evidence blob SHAは照合baseline上で一致させた。旧C24〜C34のsource/test resultは履歴を保持し、本単位で製品試験は実行していない。とくにC28の8時間FAIL、C33の総合installed FAIL、C34未着手をPASSへ読み替えない。C0〜C34の35 IDとPhase0〜45/K01〜K20のv4 scope mappingは記録済み。E0-04全項目の充填、Buffer/Q/FQ/W全数Consumer接続、v4標準OS QA、N11 evidence結合は未完で、E0およびN10/N11は`OPEN`を維持する。

## v4対応：Phase 0〜45とK01〜K20のSource→Consumer crosswalk

本節は、`docs/D4_POCKET_PHASE_MAPPING.md`の旧Phase 0〜45→C対応を履歴として保持したまま、現行v4の要求N、施工線A〜J、製品能力K、既存Consumerへ接続する。Phaseは旧工程範囲、Cは旧rev1との範囲関係であり、どちらもv4 Acceptanceや状態を移転しない。P0〜P13は本台帳の先行P crosswalkがv4仕様S05の直接mappingを記録済みであり、根拠のないPhase→P対応は追加しない。

各行のProducer/Consumer、契約、Source SHA、TestClass、証拠範囲は参照先N01〜N11の現行追跡・SHA表を正本とする。参照行にないPhase固有のSource/Test/Evidenceは`UNKNOWN`、本crosswalkで新規発行したPhase/K Acceptance Evidence IDは`NONE`。これはE0-04全列が埋まった意味ではない。`Nxx`の状態・`BUILD_GAP`・`OPEN`は参照する現行N行の値であり、Phase/KをPASS/CLOSEDへ昇格しない。A〜Jは工程表R03の同時施工線で、Phase順の逐次施工を意味しない。Source/Fixtureの存在やmapping検査は製品実作用・E3 BUILD_COVERED・OS QAを証明しない。

### Phase 0〜45

| 旧Phase／主題／旧C | v4関連N／施工線A〜J／関連K | 現行Producer→Consumer参照 | 現状・残余境界 |
|---|---|---|---|
| Phase 0 基準状態固定／C0 | N11／J／K19 | current main・Registry・Source→E0台帳／Manifest／Release Gate（N11） | 現HEADを作業単位ごとに再観測する。旧C0値は履歴。manifest整合はruntime・製品Acceptanceではない。 |
| Phase 1 製品境界／C31 | N11／J／K19 | v4 S00/S10→採用記録・進捗・正本索引・製品文書（N11） | 要求境界・文書は現在の正本で管理。文書整合だけでは製品runtime consumerは成立しない。 |
| Phase 2 接続先能力の観測／C17 | N03/N04／B・C／K01/K06/K11 | Runtime/Host Catalog→Rust Broker registry→Runtime/Host Center（N03/N04） | Host表示はmetadata registry。Trust、Permission、remote Taskを生成しない。 |
| Phase 3 Agent接続契約／D4追加 | N01/N03／A・B／K01/K06 | Runtime/Agent要求→Adapter Contract・Broker capability検査（N01/N03） | Codex CLIの限定Task履歴を別Runtimeへ拡張しない。二つ目の独立Task RuntimeはBUILD_GAP。 |
| Phase 4 Agent起動器／D4追加 | N01/N03／A・B／K01/K06 | Agent登録・実interface probe→Broker登録Adapter→Task runner（N01/N03） | Codex CLI接続sourceと限定証拠はある。未対応Runtimeは拒否し、起動器sourceを一般Provider acceptanceにしない。 |
| Phase 5 Agent操作表示盤／C1 | N01/N09／A・C／K01/K13 | Agent Center→AgentTaskClient/Broker→Task state/result projection（N01/N09） | Desktop sourceはWIRED、W01はWindows/Codex CLI限定LIVE_PRODUCT履歴。現行全OS・candidate AcceptanceはOPEN。 |
| Phase 6 作業領域検査／C1 | N09／C／K03 | Workspace Inspector→Workspace client/Broker tree/read/change/diff/history（N09） | Workspace/baseline diffと完了Taskのdiff/resultは別consumer。全Workspace・大容量・Task result投影の残差を保持。 |
| Phase 7 Compare/AgentTask/Session/Workspace結合／C1・C30 | N01/N02/N04／A・B・C／K01/K02/K03/K08 | Agent Center/Coordinator→独立Session/WorkspaceごとのTask preflight・Broker認可・Tool/Result（N01/N02/N04） | Desktop orchestration sourceはWIRED。別Workspaceの実隔離・同時installed Task・cross-agent contamination・結果適用は未Acceptance、W03 OPEN。 |
| Phase 8 Agent引継ぎ／C30 | N04／B／K02/K11 | 承認済result/artifact→Handoff package→別Session/Workspaceの新Broker Task preflight（N04-Handoff） | context/artifact/diff/testはbounded/untrusted。Permission/Approval/Credential/Trust/hidden stateを移さず、受信側で再評価。installed Handoffは未確認。 |
| Phase 9 実行履歴／C2 | N09／C／K03/K10 | Broker execution state→History client/UI（N09） | bounded metadata projection。HistoryはApprovalではなく、履歴本文の全面表示も保証しない。 |
| Phase 10 再実行・分岐／C2 | N09／C／K03/K05 | History selection→新Task/Evaluation request（N09） | 新規Taskとして現在条件で検査。過去履歴からPermission/Approvalを再利用しない。全OS実作用は未成立。 |
| Phase 11 実行系資源監視／C3 | N09／C／K04/K10 | Process/resource source→Broker snapshot→Resource UI（N09） | 未取得値はunknown。Fixture/internal snapshotを実installed CPU/RAM/I/O/GPU性能保証へ昇格しない。 |
| Phase 12 実行系ライフサイクル／C4 | N01/N03/N09／A・B・C／K01/K04/K06 | Runtime/Task lifecycle request→Broker/Adapter process control→state/UI/Audit（N01/N03/N09） | Source action・status projectionと各OS installed process supervisionは別。非対応・停止結果不明は成功扱いしない。 |
| Phase 13 評価ラボ／C5 | N09/N11／C・J／K05/K19 | Dataset/Case→Broker Evaluation Lab→bounded result/history（N09） | Evaluation/evaluator sourceはある。評価値やLLM Judge単独を実Runtime比較・製品品質保証へ昇格しない。 |
| Phase 14 回帰事例登録／C6 | N09/N11／C・J／K05/K19 | approved dialogue/result receipt→Broker case registration→protected store/metadata list（C6・N09） | case登録sourceの存在とfixture/conformance履歴に限定。private caseを通常一覧/Auditへ露出せず、C6の局所履歴を現v4 Acceptanceにしない。 |
| Phase 15 Provider/Model設定／D4追加 | N03／B／K06 | Provider/Model選択→Runtime Catalog/Adapter binding（N03） | 表示・登録sourceだけでは実Runtime呼出しでない。第二RuntimeのTask/失敗/権限証拠はBUILD_GAP。 |
| Phase 16 資格情報保管庫／C7 | N02/N05／A・B・F／K07/K08 | Owner/native credential operation→Rust Broker/OS vault→対象MCP/Provider利用（N02/N05） | 秘密値をFlutter/log/Auditへ返さない。Source/TestClassはN02/N05参照。全OSのinstalled vault証拠は未成立。 |
| Phase 17 MCP接続契約／C8 | N02/N05／A・D／K08 | MCP Server/Tool contract→current Catalog/Schema→Broker Tool call（N02/N05） | Metadata≠Authority。Tool作用は現Catalog/権限/Auditを再検査。contract/sourceはinstalled external MCP acceptanceではない。 |
| Phase 18 MCP試験基盤／C8・C9 | N02/N05/N08/N11／A・D・F・J／K08/K19/K20 | Fake MCP server/negative fixture→Broker Tool/Conformance validators（N02/N05/N08/N11） | Fake serverはfixture。manual workflowやfixture testの存在をexternal MCP実作用にしない。本単位で再実行していない。 |
| Phase 19 MCP接続管理／C9 | N02/N05／A・D／K08 | MCP Center→Broker connect/catalog/call/disconnect→untrusted result projection（N02/N05） | source capabilityはあるが現candidateの実server→同Agent後続Task→Auditは未成立。失敗/曖昧結果は停止・隔離。 |
| Phase 20 運用Profile／C10 | N09/N11／C・J／K09 | Profile設定/apply request→Audit intent/current configuration review（P crosswalk・N11） | ProfileはPermissionではない。現sourceから実RuntimeへProfileを適用するconsumerは特定できずBUILD_GAP/UNKNOWNとして維持。 |
| Phase 21 更新管理／C11 | N10/N11／G・H・I・J／K09/K16/K17 | signed update/package→OS product installer/update/rollback/repair lifecycle（N10） | Windows/Linux source候補あり、他OSにもsource/workflow候補あり。一般配布・同一candidate lifecycle・署名identityは未成立。 |
| Phase 22 通知管理／C12 | N09／C／K10/K13 | Broker notification event→notification client/desktop/mobile projection（N09） | NotificationはApproval/Auditではない。Windows toast欠落等、OS/consumer差はN09記録に従う。 |
| Phase 23 稼働観測／C13・C14 | N09/N11／C・J／K04/K10 | Runtime/Broker observation and Trace→UI; formal Audit remains separate（N09/N11） | Resource/Trace/MetricはAuditではない。内部span欠落やresource unknownを保持し、表示sourceから全runtime計測を推論しない。 |
| Phase 24 A2A接続契約／C15 | N04／B／K11 | A2A Card/Task/Message/Artifact/Stream contract→Broker operation validators（N04-A2A） | External metadata/Card≠Trust。作用ごとのOwner/Permission/AuditおよびOS範囲をN04の現行境界に従う。 |
| Phase 25 A2A接続管理／C16 | N04／B／K11 | A2A Center→Broker card/message/stream/get-task operations（N04-A2A） | Windows/Linux sourceと限定operationがあるが、同一installed candidateでの外部Agent一気通貫は未成立。macOSはmetadata-only。 |
| Phase 26 複数Host管理／C17・C18 | N03/N04／B・C／K01/K11 | Host registration/list/display→Broker metadata registry/Host Center（N04-A2A/Host） | Host registryはremote connection/Trust/Permission/Taskを作らない。Host越境Task consumerはBUILD_GAP。 |
| Phase 27 Adapter管理／C19 | N03/N05／B・D／K06/K12 | Adapter catalog/manage request→Broker catalog/compatibility projection（C19・N03/N05） | 現C19 crosswalkはmetadata管理まで。外部artifact/filesystem/process作用や完全lifecycleをsource mappingだけで認定しない。 |
| Phase 28 Windows操作面／C18・C20〜C23 | N09/N10／C・G／K13/K16 | Flutter/Runner operation surfaces→Broker tray/notification/search/recovery/process consumers（N09） | Desktop surface sourceと表示はある。installed product、実Taskとの同一ID結合、未接続operationはN09/N10 residualに従う。 |
| Phase 29 GUI-Shell構成作成／D4追加 | N05／D／K14 | selected Runtime/Agent/Tool Catalog→Compose request→Broker manifest validation（N05） | manifest-only≠実作用。Capability宣言は未信頼metadataで、Runtime権限を生成しない。 |
| Phase 30 構成差分確認／D4追加 | N05/N06／D・E／K14/K15 | Compose manifest/config→bounded diff/preview→export consumer（N05/N06） | Preview/diffは構成意図の投影。runtime構成消費・binary/module効果は別で、未確認をBUILD_GAP/UNKNOWNに残す。 |
| Phase 31 AI編集提案／D4追加 | N07／D／K14 | AI proposal/result→independent review/test Task→Broker controlled apply/rollback candidate（N07） | Proposal≠apply。独立審査結果とBroker-controlled apply/rollbackはBUILD_GAP。AuthorityをAIへ渡さない。 |
| Phase 32 Windows独立App書出し／D4追加 | N06/N10／E・G／K15/K16 | Compose config→Exporter/identity/manifest/launcher→standalone product/install candidate（N06/N10） | Builder/sourceの存在だけ。独立Appの構成消費・正規installer・実Taskは未成立。 |
| Phase 33 機能単位選択・除去／D4追加 | N06／E／K15 | module selection/manifest→exporter/build graph→binary/resource comparison | Module pruningの受入れは未成立。実binary/resource除去はfalse/未測定のまま維持し、安全Coreを省かない。 |
| Phase 34 製品配布／C33・C34 | N10/N11／G・H・I・J／K16/K17 | OS package/artifact→installer→first-run→Broker/Task→update/repair/rollback/uninstall; evidence→Gate（N10/N11） | C33の旧Windows installed total FAILを保持。現candidate dossier不在、標準OS配布経路・Owner identity/署名・GOは未成立。 |
| Phase 35 Mobile統合／C24・C26 | N01/N02/N10／I／K17 | Flutter projection→native Device Link service→Rust Broker read-only allowlist (N10/C24) | Mobile Task/MCP start consumerはBUILD_GAP。Emulator/Simulator sourceはあるが本単位未実行。Physical deviceを必須条件に戻さない。 |
| Phase 36 Apple/Linux補助検証／C25・C26 | N10/N11／H・I・J／K17 | workflow_dispatch Apple/Android source＋Linux native package/GUI consumer（N10） | workflow定義と実行結果を分離し、本単位未実行。Linux nativeは必須製品対象。WSL/Xvfb/headlessだけでnative GUI/installをPASSにしない。 |
| Phase 37 MINIDORA接続口／D4追加 | N03／B／K18 | external Runtime Adapter/HTTP boundary→Broker Runtime catalog and Task capability check（N03） | MINIDORAは接続Runtime。Codex以外のTask capabilityは未成立、MINIDORA内部実装はShell Coreへ持ち込まずproduction依存にしない。 |
| Phase 38 MINIDORA段階統合／D4追加 | N03/N08／B・F／K18/K20 | optional Runtime module→reference/parity/failure-isolation/fallback gate | 将来のmodule単位置換候補。Parity等の証拠前に切替せず、現在のproduction pathへ依存を追加しない。 |
| Phase 39 長時間運用／C28 | N11／F・J／K19/K20 | long-run runner→Broker/fixture/runtime observation→bounded report and failure classification（C28） | 旧8h試行22.766秒FAILを維持。current installed long-runはNOT_RUN。本crosswalkでは再試験しない。 |
| Phase 40 障害注入／C29 | N11／F・J／K19/K20 | failure injector→Broker/MCP/A2A/Credential/Audit fixture responses（C29） | 旧8件は開発用fixture範囲。external service/installed product全OSの耐障害PASSへ拡張しない。 |
| Phase 41 性能評価／C27 | N09/N11／C・J／K04/K19 | generated snapshot/projection/diff watchdog→local perf validator/widget test（C27） | INTERNAL_STATE/FIXTURE範囲。installed startup/frame/GPU/long-run/real Runtime SLAは未測定。 |
| Phase 42 全数回帰／C30・C32 | N11／J／K19 | local regression matrix/old static audit→validator/conformance entry（C30/C32） | Matrix/audit sourceの存在や局所PASSは全OS実物QC/QAではない。現行候補の実Runtime/installed試験は別状態。 |
| Phase 43 文書・ブランド統合／C31 | N11／J／K19 | current v4 documents/README/release docs→Manifest/Conformance/Release Gate (N11) | C31の旧文書同期statusは履歴。現文書のhash整合と製品runtime/release成立を区別する。 |
| Phase 44 Windows最大到達／C32・C33 | N10/N11／G・J／K16/K19 | installed app/Broker/Setup Doctor→Windows evidence collector/Registry (N10/N11) | 旧C33 total FAIL保持。現candidate installed evidence未収集。旧copy/旧commit evidenceを再結合しない。 |
| Phase 45 正式公開／C34 | N10/N11／G・H・I・J／K16/K17/K19 | five-OS candidate/package/evidence→strict gate/Release Registry→responsible owner decision（N10/N11） | 16 unresolved／2 resolved、release_ready=false。Publisher/signing/Audit production key/Final GOは未成立のまま。 |

### 製品能力K01〜K20

| K／S05能力 | v4関連N／施工線A〜J | 現行Producer→Consumer参照 | Source状態・残余・証拠境界 |
|---|---|---|---|
| K01 Runtime Catalog/Agent/Task/Session/Workspace/state/Launcher/IPC/Broker/Capability | N01/N03/N04／A・B・C | Runtime/Agent registry→Agent Center/Runtime Center→Rust Broker/Dialogue/Adapter→Task result/state（N01/N03） | Desktop Task source WIRED、Windows W01限定履歴あり。第二Task Runtime/Mobile TaskはBUILD_GAP、現五OS全体はOPEN。 |
| K02 独立Compare/Handoff/Workspace分離/同時Task/選択適用/Authority非継承 | N04／B・C | Compare coordinator/Handoff package→per-Agent Task preflight→selected result/new target Task（N04） | UI/source wiringあり。installed cross-agent contamination、cancellation/failure isolation、filesystem isolation/Auditの実証未成立。 |
| K03 Workspace Inspector/tree/diff/history/rerun/branch/recovery | N01/N09／C | Workspace UI→Broker tree/read/change-list/diff/history/recovery projection（N09） | Source候補あり。Task-result diffとbaseline diffの差を保持し、全Workspace/履歴/Recoveryの実consumerはOPEN。 |
| K04 PID/CPU/RAM/I/O/Latency/error/optional GPU/lifecycle | N09／C | Broker process/resource/lifecycle source→bounded UI snapshot（N09） | 未取得値はunknown。internal snapshotを実測値/Performance Acceptanceへ昇格せず、installed process metrics・GPU等未成立。 |
| K05 Eval/Dataset/Case/regression/deterministic evaluator/Runtime compare | N09/N11／C・J | Evaluation Lab/Dataset/Case→Broker evaluation→bounded result/history（N09） | Source/wiring区分はN09参照。LLM Judge単独を保証にせず、実Runtime横断比較と全要求評価は未成立。 |
| K06 Provider/Model/Runtime Adapter管理・選択・失敗・unsupported | N03／B | Provider/Runtime Catalog→Adapter capability check→Broker Task dispatch/reject（N03） | Codex CLIは限定実装・証拠。MINIDORA Task非対応は明示reject、第二の実Task RuntimeはBUILD_GAP。 |
| K07 Credential Vault/secret boundary/OS protection/lifecycle | N02/N05／A・B・F | native Owner operation→Broker credential metadata/OS vault→対象Tool/provider request（N02/N05） | 秘密のprojection/log/Audit非露出を維持。全OSのsecure storage/installed taskとの一体証拠は未成立。 |
| K08 MCP server/transport/tool/resource/prompt/Center/isolation/AI consumption | N02/N05／A・D | MCP Center/Catalog→Broker connect/call/credential binding→Tool result→同Agent follow-up context（N02/N05） | Desktop source WIRED、fixture testsあり。installed same-candidate real MCP→result-dependent Task→Auditは未成立、MobileはBUILD_GAP。 |
| K09 Profile/Channel/signed update/rollback/repair | N09/N10/N11／C・G・H・I・J | Profile config/apply intent→Audit/config; signed package/update→OS installer/update center（N10/N11） | Profileの実Runtime consumerは特定されずBUILD_GAP/UNKNOWN。update sourceと製品lifecycle/正式署名は別で、配布証拠未成立。 |
| K10 Notification/Trace/MetricとAuditの分離 | N09/N11／C・J | Broker observation/notification→Flutter UI; Audit remains separate Broker store/finalizer（N09/N11） | notification/trace/metricはAuditに昇格しない。OS toast・complete spans・全画面同一Task projectionに残差。 |
| K11 A2A Card/Task/Message/Stream/Artifact/Host/identity/Trust | N04／B | A2A/Host UI→Broker operation/Host metadata registry (N04) | 外部metadataからTrustを生成しない。OS別作用を分離し、macOS metadata-only、Host越境Task/外部installed E2E未成立。 |
| K12 Adapter Publisher/version/hash/signature/risk/compat/quarantine/update/stop | N03/N05／B・D | Adapter catalog/manage operation→Broker registry/compatibility and risk projection（C19/N03/N05） | C19 sourceはmetadata管理範囲。署名・隔離・更新・停止の全lifecycle実consumerとinstalled proofは全数未確認/OPEN。 |
| K13 Desktop Tray/quick stop/Palette/Search/Agent Center/Problems/Evidence/Recovery/Settings | N09／C | Flutter/Native surface→Broker Task/history/notification/recovery/search/tray handlers（N09） | Surface sourceと個別projectionはある。同一実Task/installed UI→stop/recovery/EvidenceのE2Eは未成立。 |
| K14 Compose/Preview/AI Edit/diff/independent review/controlled apply | N05/N07／D | Capability Catalog→Compose manifest/MCP Tool; AI proposal→independent review/apply candidate（N05/N07） | Composeはmanifest/source経路、AI Editはproposal/audit。independent review resultとBroker controlled apply/rollbackはBUILD_GAP。 |
| K15 Export/module selection/pruning/identity/Broker/Installer/config consumption | N06/N10／E・G・H・I | Compose config→Exporter→identity/manifest/launcher/package/runtime candidate（N06/N10） | identity/manifest sourceあり。runtime config consumption、independent installed app、binary/resource pruningは未成立。 |
| K16 Windows install/first-run/Setup Doctor/update/rollback/repair/uninstall | N10/N11／G・J | Windows artifact/staging→installed app/Broker→collector/Release Gate（N10/N11） | Build/staging/collector sourceあり。一般利用installer-first-run lifecycleとcurrent candidate evidence不足、Registry blocker unresolved。 |
| K17 Linux native/macOS/Android/iOS IPC/protection/Device Link/GUI/install | N10およびN01/N02 Mobile／G・H・I | OS native bridge/package→Flutter/Core/Broker/Device Link→standard installed consumer (N10) | Linux nativeは必須。workflow/source候補はOS別だが標準package/installed lifecycle未成立。Mobile Task/MCPはBUILD_GAP、物理端末は不要。 |
| K18 MINIDORA等の外部Runtime/contract接続 | N03/N08／B・F | external Runtime Adapter→Broker Catalog/Task capability/timeout/failure path（N03） | Adapterは外部接続境界。MINIDORA内部実装をCoreへ輸入せず、現production dependencyやTask supportを偽装しない。 |
| K19 long-run/performance/fault/full regression/docs/release | N09/N11／C・J | local validator/evidence/Manifest/Registry→Conformance/Release Gate (N11) | local validator/source coverageあり。旧C27〜C34限定PASS/FAILは保持。installed product QA/8h/strict releaseは未成立。 |
| K20 Buffer/root causes/B0〜B6/self-application/side effects/equivalence | N08／F | OS/dependency/cache/process/network cause→Buffer mediation→original Task/Audit/Recovery（N08） | N08 source is Codex CLI TEMP/scratch限定。B0〜B4の限定closedを保持、B5 OPEN/B6未成立、全OS意味・副作用同等性は未検証。 |

#### Phase/K crosswalkの判定

旧Phase 0〜45の46 ID、K01〜K20の20 IDを、現v4 N01〜N11・R03 A〜J・正本Consumerへ対応付けた。旧C番号との対応は履歴範囲として保持し、P0〜P13は別のv4 S05 direct mappingに従う。Source SHA/TestClassは参照N節、Phase/K固有Evidence IDはNONE、個別の完全E0-04行は未充足のものをUNKNOWNとして残す。ここでのmappingは範囲・Consumer参照の整備だけであり、N/K/Phase Acceptance、E3 BUILD_COVERED、全OS QC/QAをCLOSEDにしない。N11 Evidence ID・Blocker・全OS artifact/standard guaranteesとの行単位結合、BUF-R/Tの個別E0-04欄、B0〜B6、Q/FQ/W/QE/AUD全数照合は次工程に残る。


## v4対応：BUF-R01〜R09／BUF-T01〜T24 Source→Consumer crosswalk

要求原文はv4仕様S06（採用原本SHA-256 8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF）および緩衝基盤実装仕様書§2.1/§8.3（current git blob 2d0e98fb0b58ceec65244fde6a8fbf8e27c49b7e、SHA-256 A354E8493EE3A86DD480FCEF256FD9BDB569631B5735B3155FAC41EA77E38473）に固定する。以下は要求ID全33件の固有目的・現Consumer・状態境界を明示する。実装fileごとの現行blob SHAはN08/N10/N11各SHA表、過去自己適用evidenceはdocs/緩衝基盤_rev5補遺_QC/B5_自己適用.mdと採用記録を参照する。

### E0-04共通field値と適用範囲

| E0-04 field | このcrosswalkで共通に使う値 |
|---|---|
| 対象／目的 | 独立したOS・Toolchain・API・Filesystem・process・証拠境界間の干渉。元の要求・成功条件を保った実行可能な調停、または安全に不成立を確定する。 |
| Producer→Consumer | 失敗を生む上位要求／実行環境／Tool→D4 Buffer観測・判定→既存Rust Broker/Adapter/隔離runner→元Toolの作用・結果→Task/Audit/RecoveryまたはN11 Evidence/Gate。項目ごとの具体Consumerは下表。 |
| 契約／正常・否定作用 | 適用可能なTask-bound非権限 buffer_intervention_record v2／source、Runtime、Workspace、結果・証拠hash。適格なら記録済み調停後に元Toolを実行し、結果を独立照合。不明・unsupported・Authority不足・意味不一致・証拠不一致時は変換/成功判定を拒否し、現Brokerのdeny/fail-closedを維持。Rust生成recordのruntime Schema validation・独立readbackは未確認。 |
| 停止／Recovery | Broker Task failure/suspend、既存Audit・Task scratch journal/recoveryへ接続する範囲はN08を参照。開発用isolated runner cleanupは製品Task recoveryと別。Buffer record単体はAuthority/Permission/Approvalを生成せず、Recovery成功の証拠にもならない。項目別OS保証はUNKNOWNを許す。 |
| 標準OS候補／配布物 | Windows/macOS/Linux native/Android/iOSをv4対象とする。候補・未確定version/arch/GUI/security/resource/package/update/removalはN10表を参照。一般配布物の受入れは未成立で値はUNKNOWN。WSL/Xvfb/fixtureをLinux native受入れに使わない。 |
| 既存実装／E0-05 | REWIREとして記録できるのはCodex CLI TaskのWorkspaceTaskScratch、TEMP/TMP binding、Broker journal/Audit、限定隔離Cargo/Flutter runner。9要求・24試験全体の自動吸収は存在しない。個別根因ごとのREUSE_EXACT/REWIRE/REIMPLEMENT/REMOVE_CONFIRMED_NONREQUIREMENT/UNKNOWN確定はB0/B3全数対応待ちでUNKNOWN。 |
| SourceSHA／TestClass／Evidence ID | 原要求SHAは本節冒頭。現source SHAとUNIT/FIXTURE/CONFORMANCE/限定LIVE_RUNTIMEの分類は各N source tableおよびB5履歴に限定。今回のcrosswalkでは製品Evidence IDを発行しない（NONE）。未列挙field/file/hashはUNKNOWN。今回の製品Runtime試験はNOT_RUN。 |
| Blocker／状態／残余 | 現在のE0/N08/N10/N11をOPENのまま保持。該当する既存FQ-TEST-LOOPBACK等のIDだけを参照し、新規Release blockerは作成しない。各行の残余は下表を適用。 |

### BUF-R01〜R09（要求意味→現Consumer）

| ID／原要求 | 現Consumer・作用主体 | 現在の実装／証拠区分 | v4 status／BUILD_GAP・残余 |
|---|---|---|---|
| BUF-R01 全ボトルネックと欠測を取得可能範囲付きで全件追跡 | 履歴/Actions/source→B0原因台帳・不足証拠一覧→E0/N11要求・Evidence registry | B0台帳に履歴と欠測範囲はある。取得可能な全失敗・構想・原因の横断統合は未完。DOC/HISTORY。 | OPEN。B0の限定CLOSEDを全母集団完了へ広げない。全cause inventoryとsource provenanceが残る。 |
| BUF-R02 報告ではなく成立可能な下位操作を緩衝層が実際に行う | OS/Toolchain差→Buffer→Broker許可Tool→元Tool結果 | N08のCodex Task scratch/TEMP bindingは限定作用。診断/Buffer recordだけでは実調停でなく、汎用実作用はBUILD_GAP。 | OPEN。原因別の有効変換・隔離・再構成、元Tool実行・結果実測が未成立。 |
| BUF-R03 新しい言語・OS・依存・外部Toolにも責任モデルを適用 | Runtime/OS別Adapter→同一Broker/Buffer contract→該当Tool | N08はCodex CLI、dev runnersはCargo/Flutterの限定経路。全OS/言語/Toolの汎用実Consumerは未接続。 | OPEN。error-code一覧化を汎用性と見なさず、OS別実装/配布/standard environmentが未完。 |
| BUF-R04 要求・結果・権限・成功条件の意味と同等性を保持 | intervention pre/post observation→元Tool result/独立照合→Task finalization | buffer_intervention_record v2とfixtureはある。after hashは予定値生成の場合があり、Schema runtime validation・独立readback/意味比較は未確認。 | OPEN。同等性が証明されない変換を適用しない。元目的・実結果を結ぶ実作用比較が未成立。 |
| BUF-R05 Broker Authority/Permission/Approval/Audit/Recovery/Content Exposure/Host/Workspaceを維持 | Buffer request→既存Rust Brokerの既存Task認可・scope・Audit・Recovery | Codex scratch observationはBroker照合/Audit/hash-only projectionに接続。全介入pathのBroker強制・全OS統合は未確認。 | OPEN。Buffer自身はauthorityを持たず、現在の許可なし・Audit不能は拒否。 |
| BUF-R06 同原因の調査反復・人手介入を減らすが障害を隠さない | error/cause registry→再利用可能な介入選択→Tool result・failure report | B0記録と限定isolated runnersあり。反復/人手工数の前後測定や、一般的cause-reuse consumerは見つからない。 | OPEN。失敗隠蔽・false PASS・retryだけのroot-cause closureを禁止。削減効果未測定。 |
| BUF-R07 D4Pocket自身の実開発・検査へ自己適用 | D4 source/build/test task→isolated Cargo/Flutter runner→same-source artifact/result/cleanup | run_cargo_isolated.py、IsolatedFlutterWorkspace、B5のWindows局所記録あり。別条件・複数環境・全開発consumerは未接続。 | OPEN。B5自己適用の局所結果を保持、全体の同条件因果比較・副作用/別境界転移は未完。 |
| BUF-R08 要求から設計・実装・検査・是正・配布まで品質責任を連続させる | v4 E0〜E6 / A〜J BUILD→QE_QC→QE_QA→Release Review | AGENTS/v4工程/進捗/台帳がprocess consumer。文書整合は製品QC/QAではない。 | OPEN。局所施工QCと全体統合QC/標準OS QA/Owner Release decisionを分離し、Q0〜Q7を勝手に開始しない。 |
| BUF-R09 品質不適合を要求・設計・境界・検査方法まで根因へ戻す | 失敗/evidence→B0 cause row→B1 design→implementation/test correction→matched observation | B0/B1/B5 cause/evidence docs and local test harnesses. Cause update is record-driven; no generic product feedback consumer. | OPEN。根因未確定をUNKNOWNのまま保持し、下流移転・反証・同条件再観測を行う。 |

### BUF-T01〜T24（試験要求→現試験Consumer）

| ID／要求する検査 | test producer→actual/intended Consumer | 現証拠・TestClass | v4 status／残余・欠損 |
|---|---|---|---|
| BUF-T01 観測母集団全数・未取得履歴・重複原因 | history/action/source inventory→B0 cause ledger/E0 ID registry | B0 source records; DOC/HISTORY, current full population test NOT_RUN | OPEN。取得可能範囲/欠測/duplicate-cause linkageの網羅未判定。 |
| BUF-T02 version/ABI/dependency graph conflict・lockfile非改変 | Cargo/Flutter/API dependency input→isolated build/test & lock validation | Cargo isolated runner and tooling tests exist; UNIT/FIXTURE, this unit NOT_RUN | OPEN。矛盾時の実隔離/拒否・元Tool resultは全OSで未証明。 |
| BUF-T03 API/Schema roundtrip and mismatch rejection | schema/record producer→schema checker/Conformance plus runtime consumer | Schema valid/invalid fixtures and static Conformance exist; runtime record validation absent in N08 trace | OPEN。runtime roundtrip/equivalence and no-op on mismatch not demonstrated. |
| BUF-T04 OneDrive/Flutter failure reproduction, safe isolation, source identity | Flutter source→validate_all.py/IsolatedFlutterWorkspace→build/test output | B5 historical same-source/path observations; UNIT/FIXTURE and limited LIVE_RUNTIME. OneDrive SyncEngine final result/source-cause unknown. | OPEN/partial history. No current candidate same-command causal comparison or SyncEngine proof. |
| BUF-T05 loopback/TLS intermittent failure by initial condition; retry PASS is not cause closure | Rust fixture client/server + probe→Pktmon parser/workflow summary and failure ledger | B5 captured RST locally and hosted run #55 parser failed to decode expected tuple; test/log records are limited LIVE_RUNTIME/FIXTURE. | OPEN; existing FQ-TEST-LOOPBACK preserved. Origin/direction/common cause remain UNKNOWN; same probe/source is not repeated absent discriminating conditions. |
| BUF-T06 Smart App Control/OS signature rejection without bypass | OS policy/Cargo generated binary→normal isolated runner/Code Integrity observation | B5 non-admin Windows cargo check --all-targets local PASS; CiTool inventory denied; Apple signing/sandbox not inspected. LIVE_RUNTIME limited. | OPEN/partial. Do not relax OS protection. Other OS/package/signature path unverified. |
| BUF-T07 Workspace/Host root, secret, alias, hardlink, TOCTOU escape rejection | root/path input→Broker Workspace/Host binding and scratch capability checks | N08 Rust tests and installed Linux test source include security/recovery cases; UNIT/FIXTURE/SOURCE only here. | OPEN. No current all-OS installed mediation boundary test; new product test NOT_RUN. |
| BUF-T08 Permission/Approval/Audit/Credential absent must deny | authorization/credential state→Rust Broker request/Task/tool operation | Existing Broker denial/secret tests in N08 source list; UNIT/FIXTURE, no run in this mapping. | OPEN. Buffer cannot grant authority; current installed full path and all OS still unverified. |
| BUF-T09 crash/cancel/deadline/descendants/scratch cleanup | Task/process event→Broker process supervisor, WorkspaceTaskScratch journal and Recovery | Rust Codex/agent scratch/process tests and Linux installed smoke source exist; source/tests not executed here. | OPEN. Keep OS-specific guarantee scoped; no cross-OS product completion claimed. |
| BUF-T10 Task/parallel Task/Host state, authority, cache isolation | Task/Host identity→task-bound scratch/environment/cache and Broker | Codex CLI task-isolation Rust test source and N08 consumer; UNIT/FIXTURE source only. | OPEN. Installed parallel multi-Host noncontamination remains unproved. |
| BUF-T11 cache/lock/partial/corrupt state isolate and recover | cache/lock/artifact state→Cargo/Flutter isolation or package-update recovery consumer | Cargo/Flutter runner implementation; specific universal Buffer test/evidence identity UNKNOWN. | OPEN. Separate legitimate shared cache from per-task output; corrupt/partial rollback and cross-project effects unverified. |
| BUF-T12 abnormal Tool exit/log not hidden/meaning-changing repair rejected | original Tool exit/output→Codex adapter→Broker Dialogue/terminal result | Codex adapter and Dialogue tests, UNIT/FIXTURE source; B5 historical test runs are version-scoped. | OPEN. End-to-end meaning retention after real failure not tested in this work unit. |
| BUF-T13 build/test versus installed product and OS evidence class separation | test/build artifacts→N11 Manifest/Evidence Bundle/Release Gate | Existing manifest/evidence validators; B5 limited Windows/Android historical evidence. | OPEN. Current crosswalk emits no product evidence; candidate/OS identity joining incomplete. |
| BUF-T14 export/install/update/rollback artifact hash and identity binding | artifact producer→Exporter/Installer/Updater/rollback and N10/N11 evidence consumers | N06/N10/N11 sources in canonical SHA tables; no complete one-candidate lifecycle in this mapping. | OPEN. Artifact/source/runtime identity not yet joined across each distribution path. |
| BUF-T15 before/after intervention comparison, transfer, regression/performance cost | matched source/input/Host/Toolchain→before/after action/result/state/resource comparison | B5 includes limited path cleanup comparison; resource/long-run Q4 conditions remain deferred/not run. | OPEN/partial history. No general matched acceptance and downstream/overhead measure. |
| BUF-T16 D4 source/Toolchain pin and self-application | main source/locked toolchain→isolated D4 dev/test runner and result | B5 has several target-commit/fingerprint/toolchain-scoped entries; not all current paths. | OPEN. This unit ran documentation checks only; source self-application is not complete. |
| BUF-T17 missing Adapter/unknown cause/OS denial must stop safely | capability/environment/error input→Broker capability check/deny and observable failure | Broker unsupported/deny tests and N08 code source, UNIT/FIXTURE references only. | OPEN. Unknown cause remains unresolved; no silent fallback or success inference. |
| BUF-T18 Q1–Q6 integration and Q0 candidate freeze | versioned result→FINAL_QA_QUEUE/Q0–Q7/Release Gate | Q0–Q7 are DEFERRED; strict final candidate evidence not run. | OPEN/deferred by current stage. Do not begin formal Q stages from this crosswalk. |
| BUF-T19 requirement→design→source→product action→measured result traceability | source requirement→E0 ledger/N11 evidence identity/gate | This table adds only scope/consumer links; Manifest/Conformance check document integrity, not product effect. | OPEN. End-to-end evidence ID/source/artifact/runtime/result link remains incomplete. |
| BUF-T20 contrary/unknown observations update causal model and intervention | failed observation→B0/B1 cause hypothesis→design/test correction | B0 cause ledger and B1 design QC docs; causal updates exist for selected historical events, not full population. | OPEN. Root cause and selected interventions remain unknown/unvalidated for many rows. |
| BUF-T21 pre-change observation→correction→post-change measurement→local acceptance | BUILD_BATCH process→source/tests/progress ledger and return path | AGENTS/v4 progress/process rules; current unit is document mapping only. | OPEN. Does not establish product behavior or close a software implementation task. |
| BUF-T22 matched conditions; retry/location/assertion changes not causal proof | before/after condition fingerprints→B5 observation comparison | B5 partial matched records and explicit unclosed cases; UNIT/LIVE_RUNTIME historical scope. | OPEN/partial. Avoid claim from retry, path change, or relaxed oracle alone. |
| BUF-T23 no transfer to other boundary, resource, authority, operation burden | Buffer output→next process/product consumer + resource/cleanup/failure observations | B5 T23 has one bounded local PASS about Flutter snapshots/shared Pub cache; long-run/multiproject/Q4 remains outstanding. | Current v4 OPEN; preserve old B5 local PASS only in its exact scope. General transfer and resource effects unverified. |
| BUF-T24 bind historical QC/Q0/Q1 and new candidate evidence to version | old records+commit/hash→N11 versioned evidence/Manifest/Registry | Old evidence hash identities retained in source histories; this mapping issued no evidence ID and Q0 remains deferred. | OPEN. No old PASS/FAIL may be attached to a new candidate. |

この33行はID別scope/Consumer/既存証拠境界の照合であり、BUF-R/T Acceptance、B0〜B6判定、E0完了を意味しない。N08限定source、開発runner、fixture、文書/Registry consumerを製品Bufferの一般保証と数えない。BUF-R/T各IDの残る個別E0-04欄（対象/目的/Producer/Consumer/契約/作用/否定/Recovery/OS/配布/実装分類/SourceSHA/TestClass/Evidence/Blocker/状態/残余）の確定はN08/N10/N11のCanonical source rowsへ結合し、未知はUNKNOWNのままとする。今回のB0〜B6進捗状態は変更しない。

## v4対応：Buffer工程B0〜B6のAcceptance→Consumer／証拠crosswalk

本表は`docs/緩衝基盤_rev5補遺_QC/工程表.md`の工程条件、同補遺のB0〜B5受入記録、およびv4仕様S06・工程表R03/R09・施工指示I02-03/I04-Fを現行Consumerへ対応づける。旧工程の限定CLOSEDは履歴のまま保持する。表のConsumer対応はE0照合であり、工程Acceptanceの再判定、BUF-R/Tの全数成立、N08/W08/G7のCLOSEDを意味しない。

| 工程 | 原受入条件・過去の判定範囲 | 現v4 Producer→Consumer／責任面 | 証拠・証拠classと限界 | v4状態・残余 |
|---|---|---|---|---|
| B0 全数調査 | 取得可能な履歴範囲、欠測、事象→原因→既存対処／候補の追跡。2026-10-10時点の限定CLOSED。 | Actions・進捗・source履歴→`B0_原因対応台帳`／不足証拠・重複境界・参照原本一覧→本E0台帳／N11 Evidence・Blocker台帳。製品Runtime consumerではない。 | `B0_参照原本一覧.md`の限定受入は356 run、203 failure、直接参照147件、原因対応190/203、13 run-level UNKNOWN、取得不能log等を明記。DOC/HISTORY。CLOSEDは取得範囲内の索引成立で、全世界の実行履歴や全根因の解決を証明しない。 | 旧B0限定CLOSEDを維持。v4では全要求・証拠の行単位結合がOPEN。欠測・UNKNOWNを既知原因へ補完しない。 |
| B1 設計QC | 緩衝の意味、非権限責任、代替案、反例、契約形状の設計受入。実作用は範囲外。限定CLOSED。 | 日本語意味条件→`specs/buffer_intervention_record.schema.json`→正常／否定例→`tooling/conformance_tests/run_conformance_skeleton.py`。現在確認できるconsumerは設計契約の静的検査で、runtimeの証拠真偽・fingerprint一致を評価するconsumerではない。 | `B1_設計QC.md` §§1,5–8およびSchema/conformanceの履歴。STATIC_CONFORMANCE／FIXTURE。設計・契約形状とAuthority field拒否のみで、製品作用や意味同等性の実証ではない。 | 旧B1限定CLOSEDを維持。v4の全OS・全Tool・実Broker接続はOPEN／BUILD_GAP。 |
| B2 最小縦断 | 既存Broker認可→限定媒介→元Toolchain実行→独立結果確認、拒否・失敗伝播。Codex CLI scratch/TEMP限定でCLOSED。 | Agent Task→既存Rust BrokerのPermission／一回Approval→`adapters/codex_cli.rs::TaskTemporaryEnvironment`／`WorkspaceTaskScratch`→実Codex CLI→Broker結果・Audit。Consumer sourceはN08 canonical rows参照。 | `B2_実行QC.md` §§3–5の`LIVE_RUNTIME`記録は、local Rust Broker、実Codex CLI/MxC、synthetic localhost Responses API、Task完了・Workspace marker・負例を対象とする。Owner UI／installed product／通常Release capabilityの証拠ではない。 | 旧B2限定CLOSEDを維持。現v4ではCodex Taskの限定経路。OS/API/ABI/toolchain/cache/network一般化、同条件効果・副作用の独立照合はOPEN。 |
| B3 原因群対応 | B0原因対応行を既存機構／吸収実証／実証した吸収不能／未成立・未知へ分類し、保護境界と残余を明示。153/153行分類の限定CLOSED。 | B0原因行→`B3-M01〜M75`の分類・是正記録→N08/N10のsource/test consumerとE0残余。分類記録は各製品機構のruntime consumerそのものではない。 | `B3_全原因機構対応.md` §79。局所受入記録75件で153原因対応行を分類。DOC/HISTORYおよび各記録に明記されたsource/test/run証拠。未知行を保持し、分類完了を故障解消へ読み替えない。 | 旧B3限定CLOSEDを維持。現行sourceへのcause単位接続、未解決原因の解消、全OS機構はOPEN／BUILD_GAP。 |
| B4 製品接続 | Agent/Runtime/Workspace/Host、Broker、Audit、Agent Centerを既存製品入口で接続。Windows Desktopのstaged `r2-e2e` buildに限るCLOSED。 | Codex Adapterのscratch observation→Broker `dialogue.rs`／`agent_task_scratch.rs`照合→Buffer record／Audit→Agent Center hash-only projection。現source pathとSHAはN08 canonical rows参照。 | `B4_製品接続設計.md`の2026-10-17記録はTask completed、result/record hash、Audit chain、Workspace境界、localhost synthetic APIを含む`LIVE_RUNTIME`。Evidenceはlocal staged `r2-e2e`候補限定で、通常Release・全OS installed productの証拠ではない。 | 旧B4限定CLOSEDを維持。通常Release `task_execution=unsupported`、`release_ready=false`を保持。現在候補の全Consumer接続と他OSはOPEN。 |
| B5 開発への自己適用 | D4 Pocket自身のtoolchain／OS／開発検査へ同条件で介入前後を適用し、原因効果・副作用・問題移送を確認。全体OPEN。 | 開発検査要求→`tooling/validate_all.py`／`tooling/flutter_workspace.py`／`tooling/run_cargo_isolated.py`等→元Flutter/Cargo検査とsnapshot/cache/process観測。実行入口ごとの適用・結果・cleanupを結合する。 | `B5_自己適用.md`にはWindows開発用`LIVE_RUNTIME`、hosted/manual補助、UNIT/FIXTUREを含む局所結果がある。BUF-T23の限定PASS、T04/T15/T22の部分観測、T05/T06のOPEN、T16の部分範囲等を各記録の条件内で保持する。source／command／OneDrive実状態が一致しない過去記録は因果証明にしない。 | B5 OPEN。OneDrive同一条件の因果、loopback/TLS根因、保護状態の全条件、全OSへの適用、波及・長期資源を未成立のまま保持。 |
| B6 有限受入・回帰固定 | 24試験の必要範囲、局所肯定／否定／副作用／回帰、QC上流帰還の実効性を統合して判定。未解決はOPENのまま分離。 | BUF-T01〜T24→該当するN08/N10/N11の実Consumer・通常検査・安全回帰・証拠Registry。既存Conformance／Release Gateは個々の静的・構造条件だけを検査し、B6全体受入consumerとしては未接続。 | B6の有限受入条件は工程表に定義済みだが、B6をCLOSEDとする受入記録は確認できない。既存272 Conformance checksや文書crosswalkは`CONFIG`/`FIXTURE`/静的整合の範囲であり、B6の全数実作用・回帰・QC帰還を証明しない。正式Q0〜Q7もDEFERRED。 | 未成立。全BUF-Tの現candidate適用範囲、該当runtime証拠、副作用・回帰、未解決残余の統合判定が必要。B5やB6をCLOSEDへ変更しない。 |

このcrosswalkは工程の現Consumerと証拠の所在・限界を特定するもの。既存CLOSEDの再試験、新たなEvidence ID発行、B5/B6の受入判定は行っていない。BUF-R/T各行のE0-04詳細field、個別cause/effect、N11 Evidence ID・Blocker・OS/Artifactとの完全結合は未完である。

## 実装ファイルの初期目録（CODE_PRESENTのみ）

下記は`d503a4ef6daac8942abd1cb011442ce28c78ec55`時点のパス存在・責務候補を読み取った初期目録である。名前・コードの存在だけではProducer→Consumer接続、installed/runtime実作用、Acceptanceを証明しない。各行の`WIRED`、実作用、正常/否定/Recovery、Evidence、OS別状態は未照合なら`UNKNOWN`。

| 線／要求 | Producer・UI候補 | Broker／native候補 | 実Consumer・残余 |
|---|---|---|---|
| A／N01・N02 | `apps/desktop_flutter/lib/screens/agent_center.dart`、`services/agent_task_execution_coordinator.dart`、`agent_task_client.dart`、`agent_mcp_task_controller.dart` | `native/rust_helper/src/broker/ipc_server.rs`、`broker/agent_task_scratch.rs`、`adapters/codex_cli.rs`、`mcp_task_tool_stdio.rs`、`adapters/mcp_stdio.rs`、`broker/mcp_center.rs` | Codex CLIとBrokerの履歴はある。現行installed候補の完全なTask→Tool→同Agent follow-up→結果/Audit/Recovery call graphとv4 acceptanceは`UNKNOWN` |
| B／N03・N04 | `runtime_center.dart`、`agent_coordination.dart`、`agent_handoff.dart`、`a2a_connection_client.dart`、`a2a_artifact_handoff.dart` | `broker/runtime_registry.rs`、`adapters/codex_cli.rs`、`adapters/minidora.rs`、`broker/a2a_center.rs` | Adapter/Compare/A2A関連コードは存在する。二つ目の独立実Runtimeの実Task、外部A2A作用、host越境拒否、fresh authorityの現consumerは`UNKNOWN` |
| D／N05 | `settings.dart`のRuntime/Agent/MCP Catalog更新→Compose受理→選択Tool操作、`compose_client.dart`、`mcp_connection_client.dart` | `broker/dialogue.rs`の現登録Adapter snapshot、`broker/compose_center.rs`のcompatibility check、`broker/mcp_center.rs::call_tool` | 現在Catalog→Manifest→同じBrokerのMCP実作用はsource `WIRED`。Capability metadataは未信頼宣言で、実Provider能力を独立測定するものではない。fixtureとinstalled evidenceは別区分 |
| E／N06 | Export UI→`export_client.dart`→Broker Manifest/Receipt→Windows/Linux Developer Exporter→product launcher | `broker/export_center.rs`、`desktop_launcher.rs`、`linux_desktop_launcher.rs`、`export_windows_product.py`、`export_linux_product.py` | 新規identityとManifest/Module planはsource `WIRED`。runtimeは構成を実消費せず、証拠がruntime consumption/pruningをfalseと明記。実独立製品・Installer・binary/resource除去は`BUILD_GAP` |
| D／N07 | `settings.dart`／`ai_edit_client.dart`からproposalをBroker記録し、`main.dart`経由でAgent CenterのTaskへ受け渡す。適用側は`AgentAiEditResultApplySnapshot`が元Task/result/proposalと別Session/Workspaceを再照合する。 | `broker/ai_edit_center.rs`と通常`dialogue.rs` Agent Task経路 | proposal-only/Audit、結果由来・別Workspace束縛、fresh Task preflightはsource `WIRED`。独立審査結果、Broker-controlled diff apply/rollbackは`BUILD_GAP`。 |
| F／N08 | `adapters/codex_cli.rs`のWorkspaceTaskScratch/TEMP mediation | `broker/agent_task_scratch.rs`、`dialogue.rs` Buffer record/Audit、`execution_history.rs` | Codex CLI限定のTEMP/scratch作用・回復記録はsource `WIRED`。広域原因Consumer、意味/副作用同等性、B5/B6統合は`BUILD_GAP`。初期のfilename-only検索によるUNKNOWNは本差分で精査済み。 |
| C／N09 | Agent Center、Workspace Inspector、History、Evaluation、Observability、Problems、Recovery、Audit、TraceのFlutter screen/service files | 対応候補として`broker/history_access.rs`、`execution_history.rs`、`evaluation_lab.rs`、`observation_center.rs`、`notification_center.rs`、`content_recovery.rs`、`audit.rs` | 個別画面・Broker moduleは存在する。実Task IDを共有したend-to-end表示、failure/restart/notification/stop consumerは`UNKNOWN` |
| G/H/I／N10 | Windows/Linux/macOS Desktop runners、`apps/mobile_flutter` Dart/Android Kotlin/iOS Swift source | `native/rust_helper/src/desktop_launcher.rs`、Linux/macOS launcher・worker、Mobile Device Link native services | Windows installerに複数PowerShell収集/配置scriptがある。`installer/linux/README.md`・`installer/macos/README.md`はあるが、当該installer root内にpackage builderは見当たらない。別locationを含む正規package経路は`UNKNOWN` |
| J／N11 | `docs/QE_2026-11-13/`、README/Checklist/Progress | `tooling/manifest.py`、`tooling/release_gate_check.py`、`tooling/conformance_tests/run_conformance_skeleton.py`、`release_blockers.registry.json` | v4正本/Registry/validatorの参照は同期済み。E0全ID/Consumer/Evidence validator・完全なrelease candidate結合は`UNKNOWN` |

## 要求母集団・照合状態

| 母集団 | 件数 | 正本・原典 | v4 E0状態 |
|---|---:|---|---|
| 新規要求 N01〜N11 | 11 | v4仕様S04 | 初期Producer/Consumer差分を記録。契約・否定・OS・証拠の全項目は未完 |
| 全体施工線 A〜J | 10 | v4工程表R03・Codex指示I04 | 全線同時BUILDのproducer/consumerと現行実装配置を全数照合中 |
| 製品要求 P0〜P13 | 14 | `docs/REV5_PRODUCT_PROGRESS.md`、v4仕様S05/S10 | P0〜P6は全E0-04 fieldを個別記録。P7〜P13の個別field、P13の旧evidence identity/class、N11 Evidence/Blocker/OS Artifact全数結合は未完／UNKNOWN |
| 総合機能 C0〜C34 | 35 | `docs/総合機能拡張_rev1/` | C0〜C34全35 IDのSource→Consumer crosswalkを記録。個々の全E0-04列とv4 Acceptance／Evidence/N11結合は継続 |
| D4 Phase 0〜45 | 46 | `docs/D4_POCKET_PHASE_MAPPING.md` | 46 IDをv4 N01〜N11/A〜J/Kへcrosswalk済み。Phase固有E0-04全field、Evidence ID/Blocker、Acceptanceは未完 |
| 製品能力 K01〜K20 | 20 | v4仕様S05 | 20 IDをv4 N01〜N11/A〜JとCanonical N consumerへcrosswalk済み。K別の完全E0-04 field、OS/Evidence/Blocker確認は継続 |
| Buffer BUF-R01〜R09／BUF-T01〜T24 | 9／24 | 緩衝基盤採用記録・v4仕様S06 | 33 IDのscope/Consumer/現証拠境界をcrosswalk。個別完全E0-04欄・全根因/effect/evidenceは未完 |
| Buffer工程 B0〜B6 | 7 | `docs/緩衝基盤_rev5補遺_QC/`・v4仕様S06 | B0〜B4の限定CLOSEDを保持、B5 OPEN、B6未成立。詳細Consumer照合が未完 |
| QE/監査 D4-QE-R01〜R13／D4-LNX-T01〜T08／D4-AUD-F01〜F07 | 13／8／7 | `docs/QE_2026-10-22/`・各採用記録 | 旧証拠と現在の実装/Validatorへの接続が未完 |
| Q0〜Q7、FQ群、W00〜W08 | 8／13／9 | v4工程表R05〜R08、`docs/FINAL_QA_QUEUE.md`、進捗履歴 | Q0〜Q7はDEFERRED。FQ/W各IDの現在Consumer・証拠・残余の照合が未完 |
| Release Blocker | 18 | `release_blockers.registry.json` | 18件中16件unresolved／2件resolvedを保持。18件すべてのv4条件移行を個別に照合中 |

## Release Blocker初期一覧

| ID | 初期状態 | v4 E0処理 |
|---|---|---|
| windows_evidence_provenance_isolation | unresolved | Source／Artifact／Runtime identityとinstalled evidenceの現consumerを確認 |
| windows_installer_first_run_smoke | unresolved | 一般installer、初回起動、Setup Doctor、実Brokerの接続を確認 |
| windows_setup_doctor_smoke | unresolved | Broker統治された通常製品Setup Doctor出力を確認 |
| windows_broker_installed_smoke | unresolved | installed productからBroker実作用までの候補identityを確認 |
| audit_anchor_external_tamper_evidence_proof | unresolved | Audit耐久・外部改竄検出・証拠主体を確認 |
| owner_go | unresolved | 技術作業と責任者の最終決裁を分離して維持 |
| linux_native_product_support | unresolved | Linux native GUI/session・Package・Broker・標準環境の必須性を維持 |
| rev2_mobile_device_evidence | unresolved | 旧物理端末原因を標準Emulator／Simulator installed QAへ移行。署名等は別保持 |
| rev2_mobile_distribution | unresolved | OS正規配布・資格・署名条件を確認 |
| rev2_desktop_launch_regression | resolved | 旧resolved状態・証拠を履歴として維持 |
| comprehensive_extension_rev1_completion | unresolved | C0〜C34を含む要求Consumer未接続分を特定 |
| rev2_desktop_product_distribution | unresolved | Windows製品配布・Update/Repair/Rollback/Uninstall Consumerを確認 |
| windows_distribution_identity | unresolved | Publisher/app identityは技術外のOwner条件として分離 |
| rev2_flutter_broker_channel_boundary | unresolved | Flutter/native/Rust Broker唯一経路と標準環境Consumerを確認 |
| rev2_export_owner_ui_authority_path | unresolved | Export/Owner UI/Authority boundaryを実Consumerまで照合 |
| rev2_module_pruning_binary_and_measurement | unresolved | 選択Moduleの実binary/resourceへの効果と測定点を確認 |
| windows_rust_integration_test_execution_policy | resolved | 旧resolved状態・証拠を履歴として維持 |
| rev2_mobile_flutter_native_device_link_boundary | unresolved | Native boundary・Broker Task・標準Simulator経路を照合。物理端末要件は追加しない |

## 標準OS候補と未確定値

| OS | v4で必要な実行対象 | 初期候補・既知条件 | 未確定（`UNKNOWN`） |
|---|---|---|---|
| Windows | 一般installer→installed Desktop→実Task→Update/Repair/Rollback/Uninstall。所有PCで完成品確認 | 現source候補: portable ZIP、D4PKG01 update payload、`windows-2025` manual bundle build。8GB級PCは設計代表下限候補。 | edition/build、arch、一般installer形式、Publisher identity、最低資源実測、installed lifecycle evidence |
| Linux | 対象distro package/installer→native GUI session→Broker/Task→Update/Repair/Rollback/Uninstall | Ubuntu 24.04 LTS/x86_64 portable `tar.gz`、Ubuntu 24.04/amd64 bootstrap `.deb`、D4PKG01 update payload。Manual workflow通常scopeは24.04、bwrap制約scopeは22.04。 | 保証distro/versions/arch、native X11/Wayland範囲、product package形式、最低資源、current installed QA |
| macOS | 通常配布→installed app→Keychain/sandbox/Task→Update/Repair/Rollback/Uninstall | `macos-15` manual debug/UI workflow、source deployment target 10.15。 | 保証OS範囲/arch、通常配布形式、署名/notarization identity、最低資源、適法なQA媒体 |
| Android | 通常配布→Emulator installed app→Keystore/Device Link/Broker Task/更新・撤去 | API 35 Google APIs x86_64 Emulator、source compileSdk 37、debug APK/AAB build/install。 | supported API/ABI、min/target SDK実値、配布identity/署名、最低資源、installed lifecycle |
| iOS | Apple許容配布→Simulator installed app→Keychain/Device Link/Broker Task/更新・撤去 | iPhone 16 Pro／iOS 18.5 Simulator UI scope、source deployment target 13.0。 | supported OS/Simulator arch、device配布形式・署名/Entitlement、最低資源、installed lifecycle |

## 次のE0照合作業

1. P7〜P13をID順に全E0-04 field、N11 Evidence ID、Blocker、OS対象/Artifactへ結合する。P0〜P6は完了済みmappingとして保持。続けてN/P/C/Phase/K残行へ展開し、既存CLOSED/FAILは履歴保持して再試験しない。
2. BUF-R/T crosswalkをCanonical N08/N10/N11 source SHA・各IDの完全E0-04 field・B5 evidenceへ結合する。B0〜B6の工程・状態・受入Consumer・証拠のcrosswalkは記録済み。個別cause/effect、OS/Artifact、N11 Evidence/Blockerの結合は継続。
3. D4-QE-R01〜R13、D4-LNX-T01〜T08、D4-AUD-F01〜F07、Q0〜Q7、FQ群、W00〜W08、18 blockerを現行Consumer/証拠/gateへ一意に接続する。
4. specs/Schema、Shell Core、Rust Broker、Flutter、Native、Installer、Toolingのproducer/consumer inventoryを現在HEADへ再照合し、SourceSHA・TestClass・Evidence ID・BUILD_GAPを埋める。
5. N10の各OS標準候補・配布・実装・workflow/test・限定履歴を記録する。保証version/arch/resource/package条件は公式条件と実測から固定し、未知をUNKNOWNにする。
6. N11 Evidence/Registry/validator/QA queue consumer mappingに全ID・全OS artifactを接続する。Acceptance/E3 BUILD_COVERED/E4 QC/E5 QA/E6 Release判定は正本工程の順序と区分を維持する。

上記未完了のため、E0は`OPEN`。E1全体BUILDの網羅、BUILD_COVERED、QC/QA、Releaseの成立は主張しない。
