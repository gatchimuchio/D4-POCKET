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
| N10 | Windows/macOS/Linux/Android/iOSで通常導入・利用・更新・撤去 | Windows/Linuxのscope限定installed証拠と他OSの個別実装・試験履歴がある。v4標準OS全件のRelease候補証拠は未成立。 | `OPEN`。OS/arch/version/package/実行環境を固定し、未実装package経路を特定する |
| N11 | 全要求・Source・Consumer・証拠・Blocker・Release判断の結合 | Registry、Conformance、進捗、既存証拠は存在。v4全要求consumer ledgerは本書を含め未完成。 | `OPEN`。全IDをSource/Consumer/否定/Recovery/OS/Evidenceへ結び、18 blockerと同期する |

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
| Observation／Trace／Notification | `ObservationCenter`はBroker Audit finalization後にbounded spanを作り、Observation/Trace画面は一覧を取得する。各spanはBroker operationと関連Audit IDを持つ。Notification CenterはAudit eventからsummaryだけを作り、既読／破棄はID/hashへ束縛する。 | Observationは最大1024 span保持・一応答最大256、現在は一span一traceで親なし。Trace IDはBroker内で採番され、Task IDではない。Notificationはsummary/navigation onlyで権限を作らず、raw payloadを出さない。Observation/Auditは責任が分離される。 |
| Audit／Problems／Recovery／Search | `ShellCoreClient.product`は起動時Broker応答から一つの`ShellSnapshot`を作る。Audit Viewer、Problems、Recovery、Dashboard等は`getSnapshot()`を参照する。Global Searchは同snapshotのbounded in-memory indexを検索する。 | Audit Viewerの一覧は起動時health/host/catalog/session/probe応答から作った投影であり、永続Audit chainのTask単位検索ではない。Problems/Recoveryも同snapshotで、Task crash後のlive Recovery catalogではない。Global SearchはBroker検索・最新Task/History検索ではない。 |
| Tray／Runtime lifecycle | Windows Trayはsnapshotを定期投影し、open/navigationとBroker `全Runtime停止要求`を持つ。要求応答は`停止実行済み=false`、Owner再承認待ち。別のRuntime CenterはBroker Runtime lifecycle status/approval/executeでstart/stop/restart/pause/resume/quarantineを操作する。 | Tray stopは実停止でなくOwner承認要求である。Runtime lifecycleのpause/restartはRuntime単位で、同一AgentTaskのpause/restartとは異なる。TrayはWindows固有、定期snapshotはTask IDに結び付かない。 |
| Mobile／UI応答 | Mobile Device Linkは限定operation allowlistでRuntime/Agent/Workspace/Dialogue metadata、Runtime resource/lifecycle state、通知summary、History、全Runtime stop request等を扱う。stopは停止を実行せずOwner再承認待ち。過去W02-B01にはWindows UI processの`Responding=False`とCPU増加が記録され、fake Broker widgetでは再現せず、原因は不明。 | Device Link allowlistに`AgentTask実行/状態/取消/結果`はなく、Mobileは同一AgentTaskの実行・回復consumerではない。UI応答停止は実観測履歴があるが根因修正・同条件installed再検証は未成立。 |

### N09 状態・TestClass・Evidence

Agent Center TaskとWorkspace/History/Evaluation/Resource/Observation/Notificationの個別source consumerは`WIRED`。要件N09が要求する同一Task単位の操作・結果・diff/eval/resource/trace/audit/recovery/search結合、AgentTask pause/resume/restart/replay、全体の停止・Recovery画面、Windows UI応答停止根因是正には`BUILD_GAP`または`UNKNOWN`を保持する。Runtime lifecycle機能やDialogue replayをAgentTask操作へ読み替えない。旧対応IDはP0/P5/P12、C1..6/C12..14/C20..23、K03/K04/K05/K10/K13（v4仕様S05・工程表R03線C）。

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
| 製品要求 P0〜P13 | 14 | `docs/REV5_PRODUCT_PROGRESS.md`、v4仕様S05/S10 | 旧CLOSED/FAILの条件付き履歴とv4各N/Consumerの対応が未完 |
| 総合機能 C0〜C34 | 35 | `docs/総合機能拡張_rev1/` | 旧仕様・工程・進捗の全項目から現Consumerへの照合が未完 |
| D4 Phase 0〜45 | 46 | `docs/D4_POCKET_PHASE_MAPPING.md` | C対応は範囲関係のみ。現在のProducer/Consumer・状態照合が未完 |
| 製品能力 K01〜K20 | 20 | v4仕様S05 | 必須Consumer、OS対象、実装位置、GapのID別照合が未完 |
| Buffer BUF-R01〜R09／BUF-T01〜T24 | 9／24 | 緩衝基盤採用記録・v4仕様S06 | 元要求と現在の各実Consumer・副作用証拠の対応が未完 |
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
| Windows | 通常installer→installed Desktop→実Task→Update/撤去。所有PCで完成品確認 | Windows VM/clean user profile。8GB級PCを設計代表下限へ含める | Windows edition/build、CPU arch、installer identity、最低資源の実測値 |
| Linux | Linux native package/installer→GUI session→Broker/Task→Update/Uninstall | native distro full VM/hosted desktop。X11/Waylandは宣言別。WSL/WSLg/Xvfb単独をnative完成へ使わない | distro/version、arch、Wayland/X11保証範囲、package形式、最低資源 |
| macOS | 通常配布→installed app→Keychain/sandbox/Task→Update/撤去 | 適法なmacOS VM/hosted GUIと実native API | OS version、arch、配布形式、最低資源、仮想実行可能な環境 |
| Android | 通常配布→Emulator installed app→Keystore/Device Link/Broker Task/更新 | Android Emulator | API level、ABI、Emulator image、配布候補identity、最低資源 |
| iOS | Apple許容配布→Simulator installed app→Keychain/Device Link/Task | iOS Simulator。Simulator artifactと配布artifactを別識別 | OS version、simulator arch、配布形式、署名/Entitlement差分、最低資源 |

## 次のE0照合作業

1. `docs/総合機能拡張_rev1/`のC0〜C34原文・工程・進捗をID単位で照合し、既存実装・Consumer・履歴statusを埋める。
2. `docs/D4_POCKET_PHASE_MAPPING.md`のPhase0〜45とP0〜P13・K01〜K20をv4 N/A〜Jへ一意または多対多で接続し、範囲対応と実作用を分ける。
3. BUF-R/T、B0〜B6、D4-QE/LNX/AUD、Q/FQ/W、18 blockerを現Registry・Contract・Validator・実Consumerへ接続する。
4. `docs/specs/`、`specs/`、Shell Core、Rust Broker、Flutter、Native、Installer、Toolingのproducer/consumer inventoryを取り、各行のSourceSHA・TestClass・Evidence ID・BUILD_GAPを埋める。
5. 五OSの宣言標準環境候補を固定可能な公式仕様・既存対応から抽出する。未宣言のversion/arch/resource値を推測せず`UNKNOWN`にする。

上記未完了のため、E0は`OPEN`。E1全体BUILDの網羅、BUILD_COVERED、QC/QA、Releaseの成立は主張しない。
