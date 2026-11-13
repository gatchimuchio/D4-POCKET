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
| N03 | 独立した異種Runtimeを共通Task/Result契約で使用 | Codex CLIはBrokerのAgent Task Adapter。MINIDORAは共通対話AdapterだがAgent Task methodを実装せず、既定unsupported。Desktop登録UIはCodex CLIのみ。N03個別E0-04へ現HEADのsource/hash・契約・試験源を記録済み。 | `OPEN`。二つ目の独立したTask-capable Runtime、能力選択・拒否・結果・権限の製品経路と実Runtime証拠が不足（`BUILD_GAP`）。 |
| N04 | Compare／Handoff／A2Aの実作用と権限非継承 | Compare/Handoff/A2A source consumerはある。N04個別E0-04へ現HEADのsource/hash・契約・試験源を記録済み。Compare実行時のfilesystem隔離、外部Agent連携、Artifact転送、Host越境は現installed候補の一貫した実証なし。Hostはmetadata registry。 | `OPEN`。source接続と実作用証拠を分離し、Compare/Handoff/A2A/Hostの残件を照合する。 |
| N05 | 現実在Capability CatalogからComposeを経て元Toolを実行 | Brokerの現在Runtime/Agent metadata・接続MCP Tool CatalogをCompose時と実行時に再照合するsource経路は`WIRED`。N05 E0-04個別mappingを現HEADへ記録済み。 | `OPEN`。CapabilityはAgent Adapterの未信頼宣言metadataで、実Capability Catalog／依存解決のconsumerは未確認。重複Tool IDのBroker拒否も未成立。既存Windows fixture testは未実行、installed evidenceなし。 |
| N06 | 独立App Exportと選択Moduleの実除去 | N06 E0-04へ現HEADのProducer→Consumer・Contract・実装状態を個別記録済み。Brokerの新規App/Audit identity・Manifest・依存plan、Windows/Linux developer bundle、Launcherのhash/identity検査はsource上接続。 | `OPEN`。Windows/Linux evidenceは`runtime_manifest_consumed=false`、`binary_pruning_verified=false`。Module planは依存閉包までで実除去なし。macOS/Mobile Export consumerも未接続。正規配布・独立Appの実機能・実除去/資源比較は未成立。 |
| N07 | AI Edit proposalを別主体で審査しcontrolled apply/rollback | N07 E0-04全21項目と現HEAD `cd06f58b`の45 SourceSHAを個別結合済み。Owner-only proposal/Audit、提案・結果の未信頼束縛、別Workspace Task事前検査、fresh Permission/Approvalはsource接続あり。 | `OPEN`。独立審査結果をapply前に確定するTask／Broker状態と、proposal由来の変更を照合するcontrolled apply/rollbackが`BUILD_GAP`。 |
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

## N05/N06 初期Source→Consumer追跡（履歴参照）

この旧い粗粒度mappingは照合HEAD `06db10a8411c7c9f7e9913c01e8b2ff82b7c75e8`時点の履歴であり、下記の現行N05/N06 E0-04個別mappingが各要求の状態を更新する。製品コード比較基準は`d503a4ef6daac8942abd1cb011442ce28c78ec55`。当時列挙したsource/schema/testの差分0という記録、旧範囲、未実行状態は履歴として保持する。`WIRED`はsource接続、`FIXTURE`はtest fixtureであり、どちらもinstalled acceptanceではない。

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

#### N07 E0-04 21項目の個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | N07（AI提案／差分→別実行主体の審査・検査Task→使用者承認→controlled apply／rollback） |
| 旧ID | v4 S05で`P8→N05/N07`、`Phase 31` AI編集提案、`K14`。工程座標は施工線D、`W05/G5`。旧P8のProduct Build状態・旧Phase履歴は維持し、現N07 Acceptanceへ昇格しない。 |
| 原文参照 | v4仕様S04 N07／S05 K14、工程表R03線D・W05/G5、施工指示I04-D。提案と適用を分け、別Workspace／Session／実行主体で独立検査し、使用者の許可後にBroker管理の適用・rollbackへ接続する。提案入力とAgent申告は未信頼情報で、Authorityを継承しない。 |
| 対象 | SettingsのAI Edit proposal→Broker `GUI Shell編集提案`→Audit付きreceipt→Owner／Developerの明示確認→Agent Centerの通常Task事前検査→異なるWorkspace／未使用Sessionへの結果受け渡し。現Consumerは最終的に通常Agent Taskへ戻り、専用のBroker apply／rollback operationは持たない。 |
| 目的 | Owner／Developerが開始した開発TaskでAI提案を扱い、提案者の主張だけで変更を承認・適用しない。独立した審査結果、fresh Permission／Approval、proposal由来の実変更照合と回復をBroker境界に接続する。 |
| 生成元（`Producer`） | Settings UIで指示・対象path・予定変更を明示し、`AiEditClient.propose`が要求を生成する。別Workspaceへの適用準備では、Agent Centerが元proposal ID／payload hash／Audit ID、元Task／完了result hash、target Session／Workspaceをsnapshotへ束縛し、Owner／Developerの適用指示を含む新規`AgentTaskRequest`を作る。 |
| 実Consumer | Rust `ai_edit_center::propose`はOwner経路でproposalを検査し、`INTERNAL_STATE` Audit receiptを返す。Flutterは確認済みproposalを未信頼contextとしてAgent Centerへ渡す。適用準備側はsnapshotを再照合し、異なる未使用Sessionへ`require_unused_task_session=true`付き要求を送り、`AgentTaskClient.inspect`まで実施する。Task開始後の書込は、対象Workspaceの通常PermissionとTask単位Approvalを新規取得するAgent Task経路の作用であり、AI Edit専用apply consumerではない。 |
| 契約／Data | `gui_shell_edit_proposal.schema.json`／receipt schema、`ipc_request`／`ipc_response`、`agent_task_request`／`agent_task`、Workspace Permission request／record、Owner Approval、Content Exposure、`approval.schema.json`。意味正本は`docs/specs/gui-shell-ai-edit.md`、Task履歴は`docs/specs/agent-task-history.md`。Proposal hashとAudit ID、source Task/result hash、target Session／Workspace、unused-session条件を束縛する。本文・Result・権限は相互継承しない。 |
| 正常時作用 | Brokerは`owner_required`、schema、ID／範囲／target path、repository rule acknowledgement、`self_approval=false`、`execution_mode=proposal_only`を検証し、本文をAuditへ保存せずpayload hashを記録する。受理receiptは`apply_status=not_started`、`files_written=false`、`permission_generated=false`を固定する。明示確認後のTask準備と実行は通常Brokerの事前検査・fresh Permission／Approvalへ戻る。 |
| 否定時作用 | 非Owner、unknown field、禁止path／secret／`.git`等、自己承認、`proposal_only`以外、不正ID／指示／変更数をrejectする。result・source Task・Session／Workspace／proposal hashの不一致、Content Exposure不足、使用済みtarget Sessionまたは履歴照合不能は事前検査を拒否する。拒否をaccepted／appliedへ昇格しない。 |
| 停止／Recovery | proposal Audit append失敗は受理を返さない。snapshotが入力待機中またはBroker応答中に変化した場合、full resultが許可されない場合、targetが使用済みの場合、Broker／fresh Permission／Approvalが得られない場合はTask開始・書込前に停止する。開始後は通常TaskのRecoveryへ接続するが、AI Edit専用のrollback transaction／result-to-diff recovery consumerは見つからない。 |
| 標準OS候補 | 共有Flutter Desktop UIとRust Brokerのsource経路を確認。Windows／macOS／Linux nativeのinstalled AI Edit候補は現mappingで未実証、Android／iOSにはAI Edit UI／consumerを確認できない。Linuxは必須製品対象のまま維持し、他OSの未確認をPASSとしない。 |
| 配布物 | N07専用の配布物はない。通常製品のOS package／install／updateはN10へ接続する。N07のsource・schema・fixtureは独立Productの配布・installed動作を示さない。 |
| 既存実装 | `settings.dart`／`ai_edit_client.dart`のproposal UI・request、`ai_edit_center.rs`のOwner-only proposal受付とAudit hash、`main.dart`／`agent_center.dart`の明示受渡し、`AgentAiEditResultApplySnapshot`のprovenance束縛、`agent_task_client.dart`のpreflight、`dialogue.rs`／`execution_history.rs`のTask・未使用Session照合を確認。秘密値やApprovalを提案から移送しない。 |
| BUILD_GAP | 独立審査の結果をapply前に固定する専用Task／Broker状態がない。現行apply準備指示は、同一の通常Taskに調査・検査とWorkspace変更を依頼するため、審査完了と適用許可を独立した段階として確定しない。Brokerが実changed files／diffをproposalと照合するapply operation、AI Edit専用rollback／recovery、変更後の実Consumer確認も未接続。 |
| `SourceSHA` | 照合HEAD=`cd06f58b6541131bc5d4d39bb5434b31eed6b3cb`。下記45 Git blobを個別に記録し、全path／blobを照合した。既存N07/N08先行trace（HEAD `03f7b92d…`）とP8 crosswalkは履歴として残す。 |
| 試験区分（`TestClass`） | Rust `ai_edit_center.rs`等のUNIT、Flutter `ai_edit_client_test.dart`／`agent_coordination_test.dart`／`agent_task_client_test.dart`／widget testsのFake Broker・snapshot binding、Schema valid／negative fixture、Conformance static check。これらはSOURCE／UNIT／FIXTUREであり、独立Agent review、専用apply／rollback、installed productのEvidenceではない。本mapping作業では製品Rust／Flutter／installed testを`NOT_RUN`とする。 |
| 証拠ID（`Evidence ID`） | `NONE`。本作業は現HEADへのSourceSHA mapping。既存N07/N08 source trace、P8局所試験、schema／fixtureを現N07 installed acceptanceへ転記しない。 |
| 阻害要因（`Blocker`） | N07／施工線D／W05-G5は`OPEN`。Registry 18件（16 unresolved／2 resolved）、`release_ready=false`を保持し、新規Blocker／Acceptanceは追加しない。 |
| 状態 | N07 E0-04 mappingは21 field・45 SourceSHAを現HEADへ結合済み。N07製品Acceptance、W05/G5、E0全体は`OPEN`。 |
| 残余 | 審査結果を別実行主体の独立成果として確定するBroker consumer、Ownerの審査後許可を束縛したcontrolled apply、実diff照合、専用rollback／Recovery、各OS installed候補の実作用が残る。 |

##### N07 現行ソース識別値（`SourceSHA`）

| `Source file` | `Git blob SHA` |
|---|---|
| `apps/desktop_flutter/lib/screens/settings.dart` | `66374ec4157d573939e7f9cc5c36aecf5d65e251` |
| `apps/desktop_flutter/lib/main.dart` | `6109c821ab5dc809224bec5234c4c59db1d5f988` |
| `apps/desktop_flutter/lib/screens/agent_center.dart` | `2472faeb31a64d624423c30a0a30d0f153d2e2a3` |
| `apps/desktop_flutter/lib/services/ai_edit_client.dart` | `ba5e62311a02749d2f9fe0e4e72e2b0d4e0a497d` |
| `apps/desktop_flutter/lib/services/agent_coordination.dart` | `bdc13464f0e42b740a2bca3b7e8828a5019056d0` |
| `apps/desktop_flutter/lib/services/agent_task_client.dart` | `f54cefac29eba3dcf383bba19b4f64ee682e6217` |
| `apps/desktop_flutter/test/ai_edit_client_test.dart` | `c5ecd0353fdc811ec7c4d52a9c2b7d10e1d01723` |
| `apps/desktop_flutter/test/agent_coordination_test.dart` | `6783e87a492d8436a855a7088b7cece38d122bd5` |
| `apps/desktop_flutter/test/agent_task_client_test.dart` | `0477a0bbbb321082494b6b55711815d34419fdd9` |
| `apps/desktop_flutter/test/settings_compose_test.dart` | `a78353f93617cc7b17391549ea256f007a7acdb1` |
| `apps/desktop_flutter/test/widget_test.dart` | `f4e9c7b93f832ba2a2cefee779fd12426f240b04` |
| `native/rust_helper/src/broker/ai_edit_center.rs` | `7a1c2418dc6f71e14fc971385b6f4f2454d55b1c` |
| `native/rust_helper/src/broker/protocol.rs` | `9b2076d5f31ca6d77f1f87973d1b394aaf751961` |
| `native/rust_helper/src/broker/dialogue.rs` | `c3c3fddb630dd06b7e6cc1c13b39f3d4f6f6a999` |
| `native/rust_helper/src/broker/execution_history.rs` | `8eeb1c26d06a9e4b01806413a4bfa8c2a73357d6` |
| `native/rust_helper/src/broker/agent_task_scratch.rs` | `de743cd819fb4533c9c3070c4e7bcae82e42d938` |
| `native/rust_helper/src/adapters/codex_cli.rs` | `e5c9d1581ae9a1c0a3ae7fcfa626fdca6fea6d28` |
| `native/rust_helper/tests/agent_task_production_e2e.rs` | `fdc7565aa58fa7466e6e24e5492321e8015567eb` |
| `native/rust_helper/tests/unit/agent_task_fixture_fs.rs` | `d19ac79583b98bec9b828d6e064fe832af95bfa5` |
| `specs/gui_shell_edit_proposal.schema.json` | `b1a0b41570d5e1e1eadbf2ded3e8984d9fe21130` |
| `specs/gui_shell_edit_proposal_receipt.schema.json` | `d9f426e20254c8cf009b90e7be4cbccf453db286` |
| `specs/ipc_request.schema.json` | `f4cb3a2d9a5eeba88638eff084f25ef615fc5813` |
| `specs/ipc_response.schema.json` | `52b2dd30bb291bae4e4fdfa328cbc8c27ab0908b` |
| `specs/agent_task_request.schema.json` | `cb25bb3684656bfd670ea5832bbb7fc5988845b6` |
| `specs/agent_task.schema.json` | `a552389072088d6cd42b3588fefda332be4ae6b6` |
| `specs/agent_task_workspace_permission_request.schema.json` | `05f7a7f97c7bac884ce706cc7f1533792c35e486` |
| `specs/agent_task_workspace_permission.schema.json` | `01be05df95431e38ca5dc3ec1b5f04416b506fb2` |
| `specs/agent_task_owner_approval.schema.json` | `d190c9a8cc3d715eb199459c37c14e5de0ad6ad8` |
| `specs/agent_task_content_exposure.schema.json` | `a85b99c0213d18daaab3d5d863adcd3661f379a0` |
| `specs/approval.schema.json` | `11d60398c8f8a753f804624d0420922a4bace54d` |
| `docs/specs/gui-shell-ai-edit.md` | `33d2df562d2231d0d894afcbf6097c512a8efd8b` |
| `docs/specs/agent-task-history.md` | `8fd4cc57560662fc5f5dd5398ade8badeb98f32f` |
| `docs/specs/approval-visibility-boundary.md` | `c0c0ba87c96872344ce3968a16fc3a0ebf54a1bc` |
| `docs/specs/content-exposure-policy.md` | `9c07d3ce6bd393a3043ba507e11c0e8abd50f44a` |
| `examples/contracts/gui_shell_edit_proposal.valid.json` | `055f5b5b2d98cc755a725640122489d2408ee7f3` |
| `examples/contracts/gui_shell_edit_proposal_receipt.valid.json` | `3515521b9bc85bf666d98ff78e922967c60fd3e7` |
| `examples/contracts/invalid/gui_shell_edit_proposal_self_approval.invalid.json` | `958989e4be92acc4b482824dd16bb5628f7b0dda` |
| `examples/contracts/invalid/gui_shell_edit_proposal_receipt_apply.invalid.json` | `6a679ba144c7dae8023c1718c984dbf545bcf5e1` |
| `examples/contracts/invalid/gui_shell_edit_proposal_receipt_missing_count.invalid.json` | `061c93a5dc590f79a8ce7191afa3a6f4feb3ccc3` |
| `examples/contracts/agent_task_request.valid.json` | `29fbec0225e7c77cb1af694f16b8a6c51bca4ff5` |
| `examples/contracts/agent_task.valid.json` | `fa69e50ac55ce1e6e34ae038db0c5ddc9755d3c3` |
| `examples/contracts/agent_task_workspace_permission_request.valid.json` | `66a26513dd6756331819fed5ba6df185fae6ad6a` |
| `examples/contracts/agent_task_owner_approval.valid.json` | `678f59ae0826099c70454a6fe9d3356d7dfbd7e5` |
| `examples/contracts/agent_task_content_exposure.valid.json` | `91fc6f2c0fac829610749918e67cb7f3c8a71880` |
| `tooling/conformance_tests/run_conformance_skeleton.py` | `c6f31d8cb561b48513c6784e54c85969c9cb953e` |

### N08 Buffer／調停

| 項目 | 現行観測 |
|---|---|
| Producer→Consumer | `codex_cli.rs`のAgent Task workerはWorkspace直下へTask-bound `WorkspaceTaskScratch`を作り、Broker journalへ予約／directory identityを保存する。`TaskTemporaryEnvironment`はHost `TEMP/TMP`、Linuxでは`TMPDIR`もscratchへ設定し、Codex sandbox環境overrideを生成する。Task終了時にcleanupし、observationをBrokerへ返す。`dialogue.rs::AgentTask進捗反映`はTask/runtime/Workspace/recovery binding/root identity/cleanupを突合後、version 2 Buffer記録とAudit、terminal Task historyを作り、Task stateへstatus/hash/Audit IDだけ投影する。Agent Centerはhash-only projectionを表示する。 |
| Contract／Recovery | `buffer_intervention_record.schema.json` v2は非権限記録と`作用後未照合`／独立照合条件を定義し、valid/authority-injection/unproven-absorption fixtureをstatic conformanceが検査する。`agent_task_scratch.rs`は最大件数付きjournal、Task/runtime/Workspace/recovery bindingとroot/scratch identityの一致を確認して再起動後に回収する。`runtime_content_reconciliation.schema.json`と`runtime_content_discard_reconciliation.schema.json`は削除中断時のOwner recovery照合契約であり、N08の意味調停consumerには数えない。 |
| 観測範囲・誠実性 | 現行Rustは成功結果にscratch observationがあれば`作用後未照合`／`同等性未証明`のBuffer記録を作る。調停成功へは昇格しない。`TaskTemporaryEnvironment`のafter hashは予定値から構成され、別OS実体の後状態を独立readbackした値ではない。Adapter自身のcleanup結果は記録するが、記録だけでは元Taskとの意味同等性を証明しない。Rust生成記録を実行時にJSON Schemaへ通すconsumerもsource上確認できない。汎用Broker成功経路ではobservationが`None`ならBuffer記録を作らず、全Adapterへの必須強制ではない。 |
| 否定／Failure | Scratch作成／cleanup失敗はTaskを失敗させ、journalを残してRecoveryへつなぐ。Brokerがobservation binding不一致、Tool結果不整合、Buffer記録/Audit不能を検出した場合はTaskを隔離し、結果本文を投影しない。journal recoveryはruntime/Workspace/binding/root/directory identity不一致やunsafe pathを削除せず保留する。 |
| 既存Runtime証拠 | `docs/REV3_PROGRESS.md`の過去installed候補ではMxC `mxc_appcontainer` TEMP write失敗が複数記録され、別runではchild中write PASSでもTaskは別のcontinuation timeoutでfailed、probe cleanupは`not_attempted`だった。別記録のD4-owned `.d4p-tmp-*` journalが空でもMxC TEMP物理削除は未証明。これらは限定`LIVE_RUNTIME`＋`FIXTURE`履歴であり、N08全体PASSへ昇格しない。MxC AppContainer TEMPとD4-owned WorkspaceTaskScratchは別責任境界として保持する。 |
| TestClass / Gap / 状態 | `codex_cli.rs`にenvironment binding、cleanup failure/Recovery、並行Task isolation tests、`agent_task_scratch.rs`にrestart recovery/identity mismatch tests、`dialogue.rs`にobservation mismatch/Audit failure/terminal projection tests、`protocol.rs`にBroker crash/restart scratch recovery tests、Flutter Agent Task state/UI hash-only projection tests、static schema fixturesが存在。Rust unit/fixture/conformanceは本単位`NOT_RUN`。既存実装はCodex Task scratch/TEMPの限定調停であり、全OS/API/ABI/toolchain/cache/process/network/OneDrive原因母集団、調停前後の同条件効果・副作用比較、B5自己適用、B6全Consumer/QCは未成立。B0〜B4の限定CLOSED、B5 `OPEN`、B6未成立を維持し、N08/W08/G7は`OPEN`。 |

#### N08 E0-04 21項目の個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | N08（OS／API／ABI／toolchain／cache／process／network／OneDrive差異→意味保持調停→元Task成立） |
| 旧ID | v4 S06のBUF-R01〜R09、BUF-T01〜T24、B0〜B6。既存製品能力K07／K20、施工線F、W08/G7へ接続する。既存B0〜B4の限定CLOSEDとB5／B6の現状態は維持する。 |
| 原文参照 | v4仕様S04 N08・S06、工程表R03-F／R06／R09、指示書I04-F。採用済み緩衝補遺の要求意味・証拠履歴・BUF／B工程の詳細は補助参照とし、v4の現状態を上書きしない。 |
| 対象 | 元Taskの要求・成功条件・権限・Host／Workspaceを保ったまま、環境差・依存・一時状態・失敗を緩衝処理し、元Toolの作用・結果・副作用へ接続する責任境界。現行実consumerはCodex Agent TaskのWorkspaceTaskScratch／TEMP bindingとBroker記録へ限定される。 |
| 目的 | 下位の互換変換・隔離・状態調停・実行再構成が有効な場合にBufferが実施し、元のTask意味を保った成功または明示失敗へ戻す。未知・未証明を成功として隠さず、Authorityを緩和しない。 |
| 生成元（`Producer`） | Rust BrokerのAgentTask request／Runtime／Workspace／Host binding、Codex CLI実行環境、Task scratch作成・cleanup観測、元Toolの終了値とresult hash。開発時はCargo／Flutterの隔離runnerが環境・source差分を扱う。 |
| 実Consumer | Codex AdapterはBrokerから渡されたTask／Workspace／Recovery contextで`.d4p-tmp-*`を作り、`TEMP`／`TMP`（Linuxでは`TMPDIR`も）とCodex sandbox overrideを束縛する。Broker Protocolは永続scratch journalを開き、Workspace登録時にRecoveryを行う。DialogueはTask bindingを照合し、成功結果＋observationがある場合にv2記録／Audit／terminal historyを作る。AgentTask stateはhash-onlyを返し、Flutter Agent Centerがstatus／hash／Audit IDを表示する。`run_cargo_isolated.py`／`flutter_workspace.py`は開発用Consumerであり、製品Bufferではない。 |
| 契約／Data | `buffer_intervention_record.schema.json` v2はTask／Broker session由来Host／Workspace／入力・Toolchain指紋・介入・実作用・元Tool結果・証拠・未解決理由を含む非権限記録。`agent_task_scratch_recovery.schema.json`は最大256件のbounded journalを定義する。Task terminal historyは記録hash／状態／Audit IDだけを保持し、UI projectionは`hash_only`。Schemaの存在・fixture合格はruntimeでのSchema実行や意味同等性を証明しない。 |
| 正常時作用 | 登録Workspace root identityを固定してTask専用scratchをjournalへ予約・作成し、OS別TEMP環境を束縛して元Codex Taskを実行する。成功結果、終了値、Task binding一致、cleanup observationがそろうとBrokerは調停記録を`作用後未照合`としてAuditし、result／record hashをTask履歴とAgent Centerへ投影する。これはscratch運用の限定接続で、調停成功や意味同等性の判定ではない。 |
| 否定時作用 | scratch生成／環境binding／cleanupの失敗でTaskを失敗させ、journalを残してRecoveryへ渡す。Workspace／Task／Runtime／Recovery bindingまたはroot／scratch identity不一致、unsafe path、journal HMAC不一致は削除せず拒否・保留する。Tool結果不整合、Broker binding不一致、Audit／record生成失敗はTaskを隔離し、結果／Buffer projectionを返さない。Schema fixtureはAuthority注入と未証明吸収を拒否する。 |
| 停止／Recovery | bounded・永続journalをBroker起動後に再読し、Workspace ID・Task／Runtime／Recovery binding・root／scratch directory identityが一致した所有scratchだけを回収する。不一致・改竄・安全でないpathはfail-closedで残して保留する。これはTask scratchの限定Recoveryであり、任意のcache／process／network状態や一般Buffer介入を復旧するconsumerではない。 |
| 標準OS候補 | Windows／macOS／Linux native／Android／iOS。Rust sourceはLinuxで`TMPDIR`を追加束縛する分岐を持つ。現行Agent Task／Buffer UIはDesktop Rust／Flutter経路に限られ、Mobile Flutterの実AgentTask consumerは見つからない。5 OSのinstalled N08作用・保証は本mappingでは未確認。Linux nativeは必須対象で、WSL／headless証拠へ置換しない。 |
| 配布物 | N08専用の配布物はない。開発用isolated runner、Schema／fixture、source codeは製品配布証拠ではない。各OSの通常Installer／Package／更新・撤去はN10へ接続し、N08 installed AcceptanceはOPEN。 |
| 既存実装 | `codex_cli.rs`のTask scratch／環境binding、`agent_task_scratch.rs`のbounded journal／回収、`protocol.rs`のBroker起動Recovery、`dialogue.rs`／`execution_history.rs`の記録・Audit・Task state、Flutter `agent_task_client.dart`／Agent Centerのhash-only投影、Cargo／Flutter隔離runner、Buffer／scratch Schemaとstatic Conformance。いずれも限定Consumerとして保持する。 |
| BUILD_GAP | 汎用Buffer dispatcher／原因選択と、全OS・API／ABI・依存／toolchain・cache・process・network・OneDrive原因のproduction Consumerは見つからない。Task成功にobservationが必須ではなく、observationが`None`ならBuffer記録を作らず成功できる経路がある。`after_state_hash`は予定環境値から構成され、独立readbackではない。Rust生成recordのruntime Schema validation、元Task結果との独立readback・意味比較、調停前後の副作用／因果比較、B5自己適用の全条件、B6の全Consumer結合は未成立。 |
| `SourceSHA` | 照合HEAD=`885e22bdd2e6beb863abd7c44e975bbc059591f1`。下記30 Git blobを個別照合し、全path／hashが一致した。旧N07/N08 SourceSHA表（HEAD `03f7b92d…`）と既存BUF-R/T全21-field mappingは履歴／別要求として維持する。 |
| 試験区分（`TestClass`） | Rust UNIT／integration・crash-recovery test、Flutter fake-Broker／widget test、Schema valid／negative fixture、static Conformanceがsourceに存在する。補遺の過去B4／B5／MxC記録は限定scopeの`LIVE_RUNTIME`／`FIXTURE`履歴。これらのRust／Flutter／installed product testは本mapping単位で`NOT_RUN`。 |
| 証拠ID（`Evidence ID`） | `NONE`。SourceSHA mappingのみで、限定履歴を現行N08 installed Acceptanceへ昇格しない。 |
| 阻害要因（`Blocker`） | N08／W08/G7、B5は`OPEN`、B6は未成立。既存Registry 18件（16 unresolved／2 resolved）、`release_ready=false`を維持し、新規Blockerは追加しない。 |
| 状態 | N08 E0-04 mappingは21 field・30 SourceSHAを現HEADへ結合済み。N08製品Acceptance、B5／B6、E0全体は`OPEN`／未成立。 |
| 残余 | OS／Toolchain差異の完全な原因母集団と実Consumer、元要求・実結果の独立同等性測定、観測欠落時の必須停止、原因に対応した介入・副作用比較、標準5 OSのinstalled経路、B5自己適用とB6結合検査、N11 Evidence／Blocker／Artifactへの全件接続が残る。 |

##### N08 現行ソース識別値（`SourceSHA`）

| `Source file` | `Git blob SHA` |
|---|---|
| `native/rust_helper/src/adapters/codex_cli.rs` | `e5c9d1581ae9a1c0a3ae7fcfa626fdca6fea6d28` |
| `native/rust_helper/src/broker/agent_task_scratch.rs` | `de743cd819fb4533c9c3070c4e7bcae82e42d938` |
| `native/rust_helper/src/broker/dialogue.rs` | `c3c3fddb630dd06b7e6cc1c13b39f3d4f6f6a999` |
| `native/rust_helper/src/broker/execution_history.rs` | `8eeb1c26d06a9e4b01806413a4bfa8c2a73357d6` |
| `native/rust_helper/src/broker/history_access.rs` | `c004056b45c2f26859bf7f70a9ff2b632e358523` |
| `native/rust_helper/src/broker/protocol.rs` | `9b2076d5f31ca6d77f1f87973d1b394aaf751961` |
| `native/rust_helper/src/broker/workspace_root.rs` | `15f208e0b855b8cee53e878d2b27aaf457ea2937` |
| `native/rust_helper/src/main.rs` | `b0a5df285db357172e7970074d7a67189f3384a3` |
| `native/rust_helper/tests/agent_task_crash_recovery_e2e.rs` | `23517fcfe8ef94b2357a94e04879d423c3360501` |
| `native/rust_helper/tests/agent_task_production_e2e.rs` | `fdc7565aa58fa7466e6e24e5492321e8015567eb` |
| `native/rust_helper/tests/unit/agent_task_fixture_fs.rs` | `d19ac79583b98bec9b828d6e064fe832af95bfa5` |
| `packages/agent_orchestration/lib/agent_task_client.dart` | `a31fbb9f980a0039f809fd763c91f055fe9d02d0` |
| `apps/desktop_flutter/lib/screens/agent_center.dart` | `2472faeb31a64d624423c30a0a30d0f153d2e2a3` |
| `apps/desktop_flutter/test/agent_task_client_test.dart` | `0477a0bbbb321082494b6b55711815d34419fdd9` |
| `apps/desktop_flutter/test/widget_test.dart` | `f4e9c7b93f832ba2a2cefee779fd12426f240b04` |
| `specs/buffer_intervention_record.schema.json` | `26a1fa8ee9d010f18c157f5d3b66b9b34bdf94d5` |
| `specs/agent_task_scratch_recovery.schema.json` | `40f1eac71cd9645396ae7230d49cad25f3b77237` |
| `examples/contracts/buffer_intervention_record.valid.json` | `3ebaf07e8a6241f9e44b8c78fefa45fa23523abf` |
| `examples/contracts/invalid/buffer_intervention_record_authority.invalid.json` | `981178ad70376fa633a76a686ed6e7d12287095d` |
| `examples/contracts/invalid/buffer_intervention_record_unproven_absorption.invalid.json` | `073dbfd11943588e045360dcce2009ae7245ebbf` |
| `examples/contracts/agent_task_scratch_recovery.valid.json` | `0c5e671a7482da04715fb3cb4ec9cef68429766f` |
| `examples/contracts/invalid/agent_task_scratch_recovery_absolute_path.invalid.json` | `951d41849671a76f20fbd178b479eeed5983ced7` |
| `tooling/conformance_tests/run_conformance_skeleton.py` | `c6f31d8cb561b48513c6784e54c85969c9cb953e` |
| `tooling/run_cargo_isolated.py` | `6890c89b384db2169b92a7326f04dbd2ea6341e1` |
| `tooling/flutter_workspace.py` | `dab1c73e2dad774fd7c8c385991a4d66a969ca19` |
| `tooling/conformance_tests/test_run_cargo_isolated.py` | `b5a8c42b6c9fee9f9886b6dd3fb7e39f65530a99` |
| `docs/specs/agent-task-history.md` | `8fd4cc57560662fc5f5dd5398ade8badeb98f32f` |
| `docs/specs/content-exposure-policy.md` | `9c07d3ce6bd393a3043ba507e11c0e8abd50f44a` |
| `docs/specs/process-supervision.md` | `5b07fc9ddacb75eb9e66c547e49201183ae06290` |
| `docs/specs/failure-injection-validation.md` | `14d2a946f424b0e800a4a271c51da8b5f797dfac` |

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

### N09 E0-04 21項目の個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | N09 利用・観測・復旧。Agent Center／Workspace／History／Diff／Eval／Resource／Trace／Problems／Audit／Recoveryを同一の実Taskへ結ぶ要求。 |
| 旧ID | rev5 P0／P5／P12、総合機能拡張rev1 C1〜C6／C12〜C14／C20〜C23。製品能力K03／K04／K05／K10／K13。現行施工座標は仕様S04・S05、工程R03-C・R06 W00、指示I04-C。旧status・旧証拠は各履歴を保持し、N09へ転記しない。 |
| 原文参照 | v4仕様S04 N09・S01・S03・S08・S09、S05 K03／K04／K05／K10／K13。工程表R03-C、R06 W00、R08。施工指示I04-C、I05、I10。旧P/C/Phase対応は本台帳の既存crosswalkおよび正本の原文を参照する。 |
| 対象 | Agent Centerから開始したAgentTaskと、Workspace／History／Diff／Evaluation／Resource／Observation／Notification／Audit／Problems／Recovery／Search／Trayの製品面。各表示・操作を実Broker状態、同一Task ID、Workspace／Session／Runtime、Audit／Recoveryへ正しく束縛する範囲。 |
| 目的 | 利用者が実行中・完了・失敗した同一Taskを操作・観測・結果確認・検索・復旧できるようにし、古いsnapshot、別Runtime操作、Dialogue replay、fixtureをTask実体として表示しない。停止と復旧の結果をBroker状態へ帰還させる。 |
| 生成元（Producer） | 利用者のTask要求、Agent CenterのAgentTask preflight／start／state／cancel、BrokerのTask・Workspace・Runtime・Audit・Recovery状態、各観測operationの実測値またはunknown、通知対象Audit event。UI／Agent／MCP metadataはAuthorityの生成元ではない。 |
| 実Consumer | Desktop Agent Center→AgentTaskClient→Rust Brokerのinspect／start／state／cancel。Workspace Inspector→WorkspaceClient→Brokerの限定tree／read／compare／change-list／diff／Recovery preview。HistoryClientはTask履歴metadataとDialogue履歴・replay／branchを別経路で扱う。EvaluationClient、RuntimeResourceClient、Observation／Notification clientは各Broker operationへ接続する。Audit／Problems／Recovery／Global SearchはShellCoreClientが起動時に取得したShellSnapshotを参照する。Windows Trayはsnapshot投影とOwner再確認待ちの全Runtime停止要求を持つ。Runtime Centerのpause／resume／restartはRuntime lifecycleでありAgentTask操作ではない。Mobile Device Linkは許可されたmetadata／resource／lifecycle／notification／History／stop要求を投影するがAgentTaskのstart／state／cancel／result consumerを持たない。詳細source pathは既存N09 traceおよび下記現行SourceSHA参照。 |
| 契約／Data | `AgentTask request`／`ID`／`record`／`history page`／`content exposure`、Workspace登録／`inspection`／`diff`、`History grant`／`execution history`、`Evaluation Dataset`／`Experiment`／`result`、`Runtime resource query`／`observation`、`Observation span`／`trace`／`metric`、`Notification`、`Audit`／`Recovery`、`Windows tray stop`、`Mobile Device Link request`／`control`の現行`Schema`と`Broker protocol`。ID・hash・期限・対象・一回承認は各Broker境界で再検査し、Trace／Metric／History／Profile／通知／Adapter metadataからPermissionやApprovalを生成しない。 |
| 正常時作用 | Agent CenterでTaskをinspectし、独立したPermission／Approvalを経てstartし、同Task IDでstate／cancel／resultを取得する経路はsource上WIRED。Workspace／History／Evaluation／Resource／Observation／Notificationの各限定operationも個別にBrokerへ接続される。Audit／Recovery／Searchは現状snapshot由来のbounded表示であり、同じTaskのlive履歴・Recoveryとしては扱わない。結果本文はBrokerのContent Exposure経路に従う。 |
| 否定時作用 | 不正／未知ID、期限切れgrant、Workspace／Runtime binding不一致、余分なfield、未許可Device Link operation、欠落またはunknownの測定値は該当Client／Broker／Schemaで拒否・unknownとして扱う。Tray／Mobile全Runtime stop要求は実停止済みとして表示せず、Owner再確認待ちを返す。過去のWidget／Fixture成功をinstalled動作の証拠にしない。 |
| 停止／Recovery | AgentTask cancel要求は同じTask IDへ送られ、Broker終端状態を再読する。Runtime停止は別のRuntime lifecycle／Owner承認経路。Workspace Recovery previewは候補表示で、実Recoveryではない。Audit／Problems／Recovery画面は起動時snapshotでありTask crash後のlive Recovery catalogではない。AgentTask pause／resume／restart／replayの完全な製品操作、全Taskに結合したprocess cleanup／Recovery表示は未接続。 |
| 標準OS候補 | Windows／macOS／Linux native／Android／iOSの五OSを製品対象とする。現sourceはDesktop Flutter／Rust Broker、Windows固有Tray、および限定Mobile Device Link projectionを含む。Linux nativeは必須製品対象。OSごとのTask連続操作・観測・Recoveryのinstalled一致は本source mappingから推定せず、未確認。 |
| 配布物 | N09固有の配布物なし。通常配布・install／update／repair／rollback／uninstallはN10のOS別package／installerへ接続する。Debug／portable／test fixtureはN09 installed acceptanceの代替ではない。 |
| 既存実装 | AgentTask inspect/start/state/cancelとhash／Audit ID表示、Workspace限定読取／diff／Recovery preview、History Task metadataとDialogue replay／branch、独立Evaluation／Runtime Resource／Observation／Notification、snapshot型Audit／Problems／Recovery／Search、Windows Tray、Runtime lifecycle、Mobile projectionがある。Trace上限・応答上限、Credential／Audit／内容露出の既存境界は維持する。 |
| BUILD_GAP | Diff／changed files／test result／Evaluation／Resource／Trace／Audit／Recovery／Searchの統一投影が同一AgentTask IDへ結合されていない。EvaluationとResourceはTask IDでなくDataset／Runtimeを受け取る。Workspace diffはWorkspace／baseline基準でTask成果Artifactではない。Dialogue replay／branchは新規要求でありAgentTask replayではない。Task pause／resume／restartはなく、Runtime lifecycleと混同できない。永続Audit／RecoveryのTask別検索と通知、Trayの実停止、Mobile Task operationは未成立。過去W02-B01のWindows UI Responding=False／CPU増加は観測済みだが、fake widgetでは再現せず原因・修正・同条件installed再検証はUNKNOWN。 |
| `SourceSHA` | 基準HEAD=dd29c100c06ea5f029603932ab2f50259e84a794。直前のN09 source traceに記録した39 production blobは全39件が現在HEADでも同一SHA。追加の18 test sourceおよび46 Schema blobを下記現行表へ記録し、総計103 Git blobを個別照合した。ハッシュはGit blob識別値であり、test実行や製品Acceptanceの証拠ではない。 |
| 試験区分（TestClass） | UNIT／FIXTURE／WIDGETのtest sourceとSchema／正常・否定exampleが存在する。今回Source mapping単位では製品Rust／Flutter／installed testを実行していない（NOT_RUN）。テストsourceの存在・hash照合はruntime動作の証拠ではない。 |
| 証拠ID（Evidence ID） | NONE。W02-B01等の旧限定観測は履歴として保持し、N09の現候補Evidenceへ転用しない。 |
| 阻害要因（Blocker） | N09製品AcceptanceはOPEN、W00はSCOPED_PASS。関連するW02はOPEN。Release Registry 18件中16 unresolved／2 resolved、release_ready=falseを維持する。N09固有Blockerの新規追加・既存Blockerの状態変更は行わず、既存Blockerとの直接帰属はUNKNOWNのままとする。 |
| 状態 | N09 E0-04の21項目と現行source／test／Schema mappingは完了。N09製品AcceptanceおよびE0全体はOPEN。 |
| 残余 | 同一Task IDに結び付いた結果・差分・試験・評価・資源・観測・監査・復旧・検索の表示と、Task停止／再開／再起動／再実行／分岐／通知の実操作接続。Task異常終了後の復旧一覧、Windows UI応答停止の根因是正と同条件確認、Mobile Task境界、五OSの通常配布済み製品経路・OS別証拠、N11の証拠／阻害要因／成果物結合が残る。 |

#### N09 E0-04 現行SourceSHA補足

下記は照合HEAD dd29c100c06ea5f029603932ab2f50259e84a794におけるtest source 18件と、N09関連Schema 46件のGit blob SHA。39 production sourceの個別path／SHAは上記N09履歴trace表にあり、全39件の現HEAD一致を確認済み。

| 区分 | 出典ファイル | Git識別値 |
|---|---|---|
| 試験 | apps/desktop_flutter/test/agent_task_client_test.dart | 0477a0bbbb321082494b6b55711815d34419fdd9 |
| 試験 | apps/desktop_flutter/test/history_screen_test.dart | 7828dc17fbf50cd0a2dca3255d90c3760030e58f |
| 試験 | apps/desktop_flutter/test/history_content_test.dart | af34c1e4e61415916fe651a3f5096a95f946022b |
| 試験 | apps/desktop_flutter/test/workspace_inspector_test.dart | 2f6e53c2a38d5e23c88f2ad91e079bf8be4808a5 |
| 試験 | apps/desktop_flutter/test/evaluation_lab_test.dart | 9b068a34d9d22743bee74ba409c8d95bee938638 |
| 試験 | apps/desktop_flutter/test/runtime_resource_test.dart | c44d66bb3797bf523be0f48f24d626068b8e4894 |
| 試験 | apps/desktop_flutter/test/runtime_lifecycle_test.dart | 57cb17c453c3a50169d3f9e39de150c691e8e160 |
| 試験 | apps/desktop_flutter/test/observation_client_test.dart | 6d6ac31671e16d771bbc1a39fa7edcd9a3929dfb |
| 試験 | apps/desktop_flutter/test/notification_client_test.dart | e92e994e4a6107abc83de34cfed6032f260dd550 |
| 試験 | apps/desktop_flutter/test/global_search_index_test.dart | 2b96129f13ab11bd38afb0aba5e9a5b07d8eb3e3 |
| 試験 | apps/desktop_flutter/test/windows_tray_client_test.dart | 0b24e07221087e43355ac465ca4ace73a48af31e |
| 試験 | apps/desktop_flutter/test/widget_test.dart | f4e9c7b93f832ba2a2cefee779fd12426f240b04 |
| 試験 | apps/mobile_flutter/test/mobile_projection_client_test.dart | 74661d2f2e58046a83e99f711435e4c1b2a74b08 |
| 試験 | apps/mobile_flutter/test/resource_overview_test.dart | 7695155627699274b8c0768c0de302344caa9eec |
| 試験 | apps/mobile_flutter/test/device_link_test.dart | 509b4febc424d1c0fd330b71529b00457b5cf429 |
| 試験 | packages/gui_shell_ui/test/evaluation_client_test.dart | 044d71b4b18b7efa8891a7df735f28e6ed674ac1 |
| 試験 | packages/gui_shell_ui/test/runtime_resource_client_test.dart | 46c52c83eab712e5b84599c7e29eed660af0ed12 |
| 試験 | packages/gui_shell_ui/test/runtime_lifecycle_client_test.dart | 1b1f274cdcd8d4cf92225db173eafcc09c0c9b8f |
| Schema定義 | specs/agent_task_content_exposure.schema.json | a85b99c0213d18daaab3d5d863adcd3661f379a0 |
| Schema定義 | specs/agent_task_history_page.schema.json | 4658365b2d4784becc96a1c84f004826d8cecd8e |
| Schema定義 | specs/agent_task_id_request.schema.json | 4c6cb2096acbbf55e8a43f63e7d933e13ba42c22 |
| Schema定義 | specs/agent_task_owner_approval.schema.json | d190c9a8cc3d715eb199459c37c14e5de0ad6ad8 |
| Schema定義 | specs/agent_task_request.schema.json | cb25bb3684656bfd670ea5832bbb7fc5988845b6 |
| Schema定義 | specs/agent_task_scratch_recovery.schema.json | 40f1eac71cd9645396ae7230d49cad25f3b77237 |
| Schema定義 | specs/agent_task_workspace_permission_request.schema.json | 05f7a7f97c7bac884ce706cc7f1533792c35e486 |
| Schema定義 | specs/agent_task_workspace_permission.schema.json | 01be05df95431e38ca5dc3ec1b5f04416b506fb2 |
| Schema定義 | specs/agent_task.schema.json | a552389072088d6cd42b3588fefda332be4ae6b6 |
| Schema定義 | specs/agent_workspace.schema.json | 47058678398b5aac1b0b1467cac7e78daef72b32 |
| Schema定義 | specs/audit_checkpoint.schema.json | 9d18b2d8129f47c147e4bda79c5fdc7f84086023 |
| Schema定義 | specs/audit.schema.json | 01da4c598b52b2ae80ebf46f9e9050d7bec16ca3 |
| Schema定義 | specs/device_link_control.schema.json | a850f7b44eb4197ff7addfb0830fd339bdd5d592 |
| Schema定義 | specs/device_link_request.schema.json | f0539522c5900148753c3e6eb500c935f7b11ac2 |
| Schema定義 | specs/evaluation_case.schema.json | 5fed01706bb374697294ccb19eec1ae309d53f6a |
| Schema定義 | specs/evaluation_comparison.schema.json | 124af9c4e4def4c048751e911809b6f2ed3b577a |
| Schema定義 | specs/evaluation_dataset_registration.schema.json | a79c103e32229dc47d263e62c05914624085610b |
| Schema定義 | specs/evaluation_dataset.schema.json | 8a90b7248e16c04d7b51ab9bca0a9a5f6c459ed1 |
| Schema定義 | specs/evaluation_evaluator.schema.json | d2d5298afc927dfb82c73f136db7dce77bdccf13 |
| Schema定義 | specs/evaluation_experiment.schema.json | e35d7a242a0a9d318900673df9fb900e841b93a2 |
| Schema定義 | specs/evaluation_public_result.schema.json | 77a285a53a0473fc4185d18b37b863f90be71ada |
| Schema定義 | specs/evaluation_result.schema.json | f1b994e19dfaf1473a615cbf1347de22f72ccbed |
| Schema定義 | specs/mobile_local_recovery_audit.schema.json | aa30b130979b5f869b5a08cc21315f3918679747 |
| Schema定義 | specs/notification_action.schema.json | d7ab880c65799ac8e69a02c1ae1da594989e8f17 |
| Schema定義 | specs/notification_list.schema.json | 0fa037e50d58c10c4b5a7364c0a7472e708996b6 |
| Schema定義 | specs/notification.schema.json | 3028750ccf7b647867314664cf5280ea7ec26812 |
| Schema定義 | specs/observation_list.schema.json | 1c4eba48d5b8a3318253e7ba6890305bd5e89e7c |
| Schema定義 | specs/observation_metric.schema.json | eaa8757dd6b43611728c358a7277ac84efa5f242 |
| Schema定義 | specs/observation_span.schema.json | 8cc3b8e66f242e8009db546145ae8f3fa1582786 |
| Schema定義 | specs/observation_trace.schema.json | 8365f277eab716618c02e7635eaf5896e6a84820 |
| Schema定義 | specs/recovery.schema.json | 9a147fdb00678d20eab3668b60e65f357c06c771 |
| Schema定義 | specs/regression_case_delete_recovery_receipt.schema.json | b9b1de18f811bc03f38d5dfaa497f673e335e496 |
| Schema定義 | specs/regression_case_delete_recovery_request.schema.json | c69d71ef2f07a2a7090ba3d0d648448c60ca1c4e |
| Schema定義 | specs/runtime_agent_task_history_access.schema.json | baddfeb38762fe0a04513cb1edcfcd258bd804ed |
| Schema定義 | specs/runtime_agent_task_history_request.schema.json | 0bff21625a0f339367ae8352910b643b1f76b361 |
| Schema定義 | specs/runtime_execution_history_page.schema.json | e6dab615707d30c9266df7dc67fc90df24d0ed9b |
| Schema定義 | specs/runtime_execution_history.schema.json | 5b260e9b6026758dc0a730d3381b4f7d72ba2e98 |
| Schema定義 | specs/runtime_history_access.schema.json | 8187ddce917c3e80f40c8ce1c4b89059277f1cfd |
| Schema定義 | specs/runtime_resource_observation.schema.json | 62f587b8c88d0c5c0bd316eaa3d0a15c9c1bd2d1 |
| Schema定義 | specs/runtime_resource_query.schema.json | c7617ba9c2283255fb04b6cfa529ff7b04f6137f |
| Schema定義 | specs/tray_stop_request.schema.json | 579755262940027079b37ef859e6d2f32a071cc0 |
| Schema定義 | specs/tray_stop_response.schema.json | d6cce063d5deaf7104b97ef67f4efb14f780cc72 |
| Schema定義 | specs/windows_tray_projection.schema.json | 5c3975e03b34281a0f202374cf91127357f2a1e0 |
| Schema定義 | specs/workspace_diff.schema.json | 1389ad85c48e263e688ab6dfa5276f96eb26fef0 |
| Schema定義 | specs/workspace_inspection_request.schema.json | a8210c300b2909c040188d0766633f1e30884e7a |
| Schema定義 | specs/workspace_inspection_response.schema.json | 6cc9f80f4dd9b8bd9f71441bee845017428fa38a |

## N10 標準OS導入・利用・更新・撤去 Source→Consumer追跡（E0差分）

照合HEADは`ca9d3cfd080ca9f5bd6dad3850e5ebe79a24f125`。`d503a4e`はv4採用時の製品source比較基準として履歴保持する。既存N10 Source SHA表の33件は現HEADのblob値と全一致した。`WIRED`は確認できたコード上の生成元→実Consumer接続だけを示し、一般利用者向け配布・導入済み製品の全工程・対象OSの合格を意味しない。本単位はE0 source mappingであり、現候補のN10全工程受入・OS QA・出荷適格性は未判定のままとする。

| OS | Producer→Consumer／現物 | 配布・導入・初回利用・更新/復旧/撤去の観測 | TestClass／Evidence／N10差分 |
|---|---|---|---|
| Windows | Desktop Flutter→Windows Runner→Rust `desktop_launcher`／認証Broker。`product_install.rs`／`update_center.rs`にはper-user install/update操作がある。Export builder、portable packager、D4PKG01 product packagerも存在する。 | `stage_installed_app.ps1`はDeveloper／release-evidence用staging。Export ZIPはportable配置物、D4PKG01はupdate packageで、一般利用者向け`.exe` installerとは異なる。manual workflowはsynthetic Receipt/Manifestから未署名bundleを作るだけで、workflow内にinstalled-product起動・Owner認可・Update/Rollback/Uninstallはない。W01/G1のWindows normal Release実Taskは限定履歴であり、通常installerの一貫した初回利用証拠ではない。 | workflowは`workflow_dispatch`のbuild fixture。N10実導入は`NOT_RUN`。`windows_installer_first_run_smoke`、`windows_setup_doctor_smoke`、`windows_broker_installed_smoke`、`windows_evidence_provenance_isolation`、`windows_distribution_identity`等が未解決。 |
| Linux | Linux export builder→固定bundle／Rust `linux_product_bundle`・launcher→Broker `product_install`／`update_center`。D4PKG01 writerはLinux host上のbundle梱包を行い、APT setup package builderも別に存在する。 | 現portable `tar.gz`候補はUbuntu 24.04 LTS／x86_64のunsigned開発archive。`d4-pocket-setup` `.deb`はbootstrap UI配置用で製品本体packageではない。D4PKG01は更新payloadで、OS package manager向け製品installerと同じものではない。manual Linux workflowにはinstalled GTK/X11・Wayland等のscopeがあるが、この台帳作業では未起動。過去run #77等のlimited Xvfb/X11、run #286のUbuntu 22.04 hosted evidenceは旧commit・限定scopeで、現候補の一般配布／実desktop／Wayland／全lifecycleへ昇格しない。 | package/smoke unit sourceとmanual workflowは存在。現行N10 acceptanceは`NOT_RUN`。`linux_native_product_support` unresolved。WSL/WSLg/Xvfb/headlessはnative standard desktop acceptanceへ読み替えない。 |
| macOS | macOS Flutter Runner→Rust `macos_desktop_worker`／`macos_desktop_owner`。App bundle設定とOwner UI test sourceがある。共有ProductPackagePlatform/update pathはWindows/Linuxのみ。 | `installer/macos/README.md`はproduct installerをv1後の計画と記載。bundle IDは`com.example.guiShellDesktop`のplaceholder。manual Apple workflowは`macos-15`上のdebug build/XCTest/UI scopeで、正式`.app`配布、署名/notarization、installed Update/Rollback/Uninstallは示さない。 | UI/XCTest sourceは存在するがN10 standard distribution lifecycleは`NOT_RUN`。release OS range/arch/package/identity/最低resourceも未固定。 |
| Android | Flutter Mobile→Kotlin `MainActivity`／native Device Link service→Rust Broker transport。Gradle・native product-flow instrumentation testがある。 | manual workflowはAPI 35 Google APIs x86_64 Emulatorを起動し、debug AABをbuildしdebug APKをinstallする。`applicationId`はplaceholder、`minSdk`/`targetSdk`はFlutter既定値参照。正式署名配布、store/一般配布、更新・復旧・撤去を結ぶ候補は未成立。N01/N02でMobile AgentTask/MCP作用consumerも`BUILD_GAP`。 | Emulator/debug build・native test sourceはあるが、今回workflowは実行していない。標準API/ABI・distribution identity・最低resource・N10 lifecycleは`UNKNOWN`／`NOT_RUN`。 |
| iOS | Flutter Mobile→Swift `DeviceLinkNativeService`／Keychain／Rust Broker transport。Xcode projectとDevice Link UI test sourceがある。 | `IPHONEOS_DEPLOYMENT_TARGET=13.0`はsource設定であり製品の保証範囲確定ではない。manual Apple workflowはiPhone 16 Pro／iOS 18.5 Simulator向けdebug UI scopeを持つが、device配布IPA、store経路、署名/entitlementを含むUpdate/Repair/Uninstall実証はない。Simulator artifactとdevice配布artifactは別対象。Bundle IDはplaceholder。 | Simulator UI test sourceはあるが今回workflowは実行していない。配布用identity/署名、保証OS範囲、最低resource、installed lifecycleは`UNKNOWN`／`NOT_RUN`。N01/N02のMobile Task/MCP consumerも`BUILD_GAP`。 |

#### OS標準候補と実証範囲の更新

| OS | 現sourceから確認できる候補（版・arch・安全・GUI・Runtime） | 配布／更新／撤去の実装候補 | 検査媒体／資源 | 保証対象として未確定の値・作用 |
|---|---|---|---|---|
| Windows | manual runner `windows-2025`。通常Windows user profile／`%LOCALAPPDATA%` scope、Rust launcher＋Flutter Desktop＋認証Broker。正式対応Windows edition/build/archは未固定。 | portable ZIPと`D4PKG01`更新payload、Broker側per-user install/update sourceあり。`stage_installed_app.ps1`は証拠用stagingで、一般配布`.exe` installerではない。正式Publisher／署名・rollback／uninstall一体経路は未受入れ。 | manual workflowは合成Receipt／Manifestから未署名bundleを生成しrunner内で破棄。Windows installed collector sourceあり。8GB RAMは候補下限で実測でない。所有Windows確認は後工程。 | 保証edition/build/arch、通常配布installer、実測最低資源、別profileの現候補初回利用・AI仕事・update／repair／rollback／uninstall。 |
| Linux | archive候補Ubuntu 24.04 LTS／x86_64。GTK 3、GLib 2、ログイン中D-Bus、XDG data root。資格機能はSecret Service。標準OS保護と通常userで起動。 | portable `tar.gz`、bootstrap用`d4-pocket-setup` `.deb`、Brokerの製品`D4PKG01` install／update／rollback sourceあり。bootstrap `.deb`は製品本体packageではない。manual lifecycle scopeはsource上あり。 | `ubuntu-24.04` hosted環境にXvfb／X11、isolated Weston headless/Pixman Wayland、`debian_setup_lifecycle`がある。今回未実行。最低resource未測定。過去Actions参照は各runの限定scopeのみ。 | 保証distro/version/arch、実desktop X11とWaylandの適用範囲、正式package／署名、最低資源、一般利用者向け完全lifecycle。WSL／Xvfb成功はnative desktop保証でない。 |
| macOS | manual runner `macos-15`。source deployment target 10.15はbuild設定で保証範囲ではない。Rust helper／Swift native owner／App Sandboxの経路あり。 | `.app` installerはREADMEで将来計画。`product_package`の対象platformはWindows／Linux。標準macOS product installer／in-app update／rollback／uninstallは未成立。 | Apple manual workflowはdebug build／XCTest／native UIの限定検査。今回未実行。製品最低資源・保証arch未測定。 | 保証macOS version/arch、配布bundle／installer、正式署名/notarization、更新・撤去とinstalled first-run全経路。 |
| Android | 手動検査の`API 35`／`Google APIs`／`x86_64 Emulator`、sourceの`compileSdk=37`／`Java 17`。`minSdk`／`targetSdk`は固定`Flutter toolchain`から継承。Android標準の`app sandbox`／native `Device Link store`・`TLS service`。 | workflowはdebug APK／debug AABをbuildし、debug APKをEmulatorへinstall／launch。`applicationId=com.example.gui_shell_mobile`は開発用placeholder。store／通常署名配布・製品update／rollback／uninstallは未成立。 | Android Emulator source/API 35。manual workflow・instrumentation test sourceあり、今回未実行。最低resource未測定。 | 保証API／ABI、正式application identity／署名・配布、初回からDevice Link serviceまでの通常利用者lifecycle、更新・撤去、最低資源。 |
| iOS | manual Apple runner `macos-15`、iPhone 16 Pro／iOS 18.5 Simulator候補。source deployment target 13.0は保証範囲ではない。Swift Device Link／Keychain／Rust Broker transport sourceあり。 | workflowはSimulator向けdebug build／UI test。配布用署名済みIPA／Store identity／更新・rollback／uninstall経路は未成立。Bundle IDはplaceholder。 | iOS Simulator sourceあり、今回未実行。device配布物とSimulator buildは別Artifact。最低resource未測定。 | 保証iOS／Simulator arch、正式identity／signing／entitlement、通常配布・更新・撤去、最低資源。 |

#### N10に関係するSource SHA

既存33件のSource SHAは照合HEAD `ca9d3cfd080ca9f5bd6dad3850e5ebe79a24f125`でも全一致。

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

#### N10 現行Source／Test／Schemaの追加識別値

下記28件を既存33件のSource SHA表へ追加した。合計61 distinct Git blobは照合HEADに結合する。これはsource同一性と試験sourceの存在を示し、試験の実行結果やN10 Acceptanceを示さない。

| 種別 | OS／役割 | ソースファイル | Git blobのSHA |
|---|---|---|---|
| 実装 | 共通 | `native/rust_helper/src/installed_artifact.rs` | `7328ccce0378c92743b13e14f26925e6749826dc` |
| 実装 | 共通 | `native/rust_helper/src/update_verification.rs` | `6347d90043cc7196feece6d708fafe96d27f3f19` |
| 実装 | `Mobile Broker` | `native/rust_helper/src/broker/device_link.rs` | `c2e1ff88ff34a5764b2b4fb1d3f7abd8727e5c51` |
| 実装 | Linux 導入検査 | `tooling/linux_debian_setup_workflow.py` | `297d1dedc0a633277aa98f40a7162c9ef3627ba9` |
| 実装 | Windows 更新画面用client | `apps/desktop_flutter/lib/services/update_client.dart` | `26d2e946b5b15ea3328867e606e9f75bf4bfc2d1` |
| 実装 | Windows 導入状態表示 | `apps/desktop_flutter/lib/screens/setup_doctor.dart` | `41fe7f3e904fcc2179ccc8c8373e53f0b646d1fe` |
| 実装 | Android native保管 | `apps/mobile_flutter/android/app/src/main/kotlin/com/example/gui_shell_mobile/DeviceLinkNativeStore.kt` | `55484d80a94abaad907074defebf116fdcf7aeb7` |
| 実装 | iOS native保管 | `apps/mobile_flutter/ios/Runner/DeviceLinkNativeStore.swift` | `04d8e5fcea97b3d05268f593da5a40e212d1e518` |
| 試験 | Linux package導入 | `tooling/tests/test_linux_deb_setup_package.py` | `2467e29b9eb13bd65065fea5a80872a3068cdfcd` |
| 試験 | `Linux update catalog` | `tooling/tests/test_linux_update_catalog_fixture.py` | `a841f259911eefe99f4e474e18c57aae91cde994` |
| 試験 | `Linux workflow helper` | `tooling/tests/test_linux_workflow_helpers.py` | `15c71a2ba5ac09ddab2c6ea23b36db256aeab85a` |
| 試験 | Linux export | `tooling/tests/test_export_linux_product.py` | `079a187de49cf8b2fb6a7ddadb4175d6916163d1` |
| LIVE_PRODUCT試験補助 | `Linux installed update lifecycle` | `tooling/tests/run_linux_same_source_update_acceptance.py` | `6891a0a2db378577d30aec9c9466af594864b8ec` |
| 試験補助 | Linux lifecycle準備 | `tooling/tests/prepare_linux_same_source_update.py` | `d88746ee1c6c9307b0d5189c1759f55135958a25` |
| 試験 | Windows install画面 | `apps/desktop_flutter/test/settings_install_flow_test.dart` | `73fc7b9706a7f4246972db46d5398d8b73decb09` |
| 試験 | `Windows update client` | `apps/desktop_flutter/test/update_client_test.dart` | `f70bb4a6dbc2340b5d26d867496952629dad1218` |
| 試験 | `Android native policy` | `apps/mobile_flutter/android/app/src/test/kotlin/com/example/gui_shell_mobile/DeviceLinkPolicyTest.kt` | `cca69277be00014e8681f59db3a04a4adbb9fb23` |
| 試験 | `Mobile Device Link UI` | `apps/mobile_flutter/test/device_link_test.dart` | `509b4febc424d1c0fd330b71529b00457b5cf429` |
| Schema定義 | Windows／共通package | `specs/d4_pocket_product_package_manifest.schema.json` | `3151ba81a869b87e6c0fa8f8633a0e424fc43ae7` |
| Schema定義 | Linux package | `specs/d4_pocket_linux_product_package_manifest.schema.json` | `34087f190e7189641b34ae3163b568a038c9f1c2` |
| Schema定義 | `Export Manifest` | `specs/gui_shell_export_manifest.schema.json` | `bf136b1a1daa258d33af3635ee414a2d4312bec5` |
| Schema定義 | `Update request` | `specs/update.schema.json` | `351a76c29f0279cc2cef92970f082ba7c2f7aaf1` |
| Schema定義 | `Update catalog` | `specs/update_catalog.schema.json` | `faa9297d56029f326cfe0c0962a5c89f55c97470` |
| Schema定義 | `Update candidate` | `specs/update_candidate.schema.json` | `1d803d8a2aaf22576346c77f4c7113ad834d213c` |
| Schema定義 | `Download job` | `specs/update_download_job.schema.json` | `42f457bf1fb1275c17f55a90f671cc91e8a51e2e` |
| Schema定義 | `Update list` | `specs/update_list.schema.json` | `b4ff3283dfd7e781bf6af9a0c208caf3aa7ed7e1` |
| Schema定義 | `Update receipt` | `specs/update_receipt.schema.json` | `86471c54dd55fa55bc1cdc7ebe32050d9f30ed36` |
| Schema定義 | `Update trust` | `specs/update_trust.schema.json` | `5caa430cf621c09e6e3d96bd6c377758095b9491` |

対象test sourceの分類は、Linux package/smoke `UNIT`/hosted `LIVE_PRODUCT`、macOS/iOS `XCTest/UI`、Android instrumentation/Emulator、各manual workflow `workflow_dispatch`。本mappingでは列挙testもworkflowも未実行。過去のWindows W01/G1、Linux #77/#286等は元commit・実行媒体・scopeを保持した限定履歴であり、current N10 Release candidateへ転記しない。Linux以外の物理端末が不要であることは、標準VM/Emulator/Simulator上の実installed QAを省く意味ではない。

N10は`OPEN`。OS別の実consumer sourceと一部のpackage/lifecycle部品は確認できたが、通常利用者向け配布から初回設定・基本サービス・実AI仕事・Update/Repair/Rollback/Uninstallまでを同一のcurrent candidateで通した証拠が全5 OSに揃っていない。未知の標準version/arch/最低資源を創作せず、個々のcandidate値と保証対象を分けて保持する。N10 source mappingはE0全数照合、E1全体BUILD、BUILD_COVERED、QE_QC/QA、Releaseを閉じない。

### N10 E0-04 21項目の個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | N10 全OS製品化。Windows／macOS／Linux native Desktop／Android／iOSで、配布物→導入→初回起動→基本サービス→実Task→更新／修復／復旧／撤去を一般利用者が利用できる状態にする要求。 |
| 旧ID | P11→N10、P12→N01／N09／N10、P13→N10。K16／K17。旧C24〜C26およびPhase対応は既存crosswalkを参照し、旧status・限定証拠を履歴として保持する。 |
| 原文参照 | 仕様S04 N10、S01／S02／S09、S05 K16／K17。工程R00-02、R02 E0-04〜06、R03 G〜I、R07-00。指示I04-G/H/I、I06-03、I10-01/02。旧P/Phase/C対応は本台帳の既存表を参照する。 |
| 対象 | 五OS向け通常配布・OS標準保護・native GUI/session・Runtime／Broker・導入済み製品のfirst-run、AI仕事、更新・修復・rollback・uninstall。Build artifact、portable、debug、Simulator、証拠collectorだけの経路は対象製品として扱わない。 |
| 目的 | 通常利用者がPython／Rust／Flutter／Git／clone／terminal等の開発環境を導入せず、OSに合った配布物から安全に起動し、実AI Taskの作用・結果・停止／復旧と製品更新・撤去を完了できるようにする。 |
| 生成元（Producer） | Versioned source／toolchainとExport builder、OS package／bundle builder、Product Manifest／package、normal app launch、Broker install／update／recovery operation、OS session／security、runtime Task、version／artifact測定点。 |
| 実Consumer | Windowsの接続は`Flutter → Windows Runner → Rust desktop_launcher → Broker`。導入は`stage`／`per-user Store`。Linuxは`portable／APT bootstrap launcher → Rust linux_desktop_launcher → Broker Product Install／Update Center → XDG entry`。macOSは`Flutter Runner → Rust helper → Broker`の経路があるが、製品installerはない。Androidは`Flutter → Kotlin Device Link service → Rust Broker transport`。iOSは`Flutter → Swift Device Link service／Keychain → Rust Broker transport`。 |
| 契約／Data | `d4_pocket_product_package_manifest.schema.json`、`d4_pocket_linux_product_package_manifest.schema.json`、`gui_shell_export_manifest.schema.json`、`update*.schema.json`、`ProductPackagePlatform`、`D4PKG01`、`App`／`Audit identity`、`source commit`、`bundle/package/installed-artifact hash`、`target OS/arch`、`update trust/receipt`、`OS path/entry`、Brokerの`Approval`／`Audit`／`Recovery`。 |
| 正常時作用 | 目標は同一SourceSHAからOS別Artifactを生成し、同一identity／package hash／Broker runtimeへ導入、初回Setup Doctorと標準GUIを起動し、同じinstalled candidateでTask・結果・Auditを確認後、Owner認可付きupdate／repair／rollback／uninstallを完了する。現sourceはWindows／LinuxのBroker package/update部品、Linux manual lifecycle harness、Android/iOS Device Link、各OS manual build workflowを持つ。 |
| 否定時作用 | Manifest／packageのSchema違反、unknown field、OS／App／Audit identity不一致、hash／署名／version／root／path不一致は該当Builder／Broker検証で拒否するsourceがある。Windows／Androidのmanual exportはsynthetic／debug scopeであり署名・installer・利用をPASSにしない。Linux lifecycleはmanual test identityを使うscopeで、本番署名を主張しない。macOS/iOSの未対応配布物を利用可能と表示しない。 |
| 停止／Recovery | Windows／Linux Broker sourceはversion stage／activate／rollback／限定uninstallと状態検査を持つ。Linux harnessはOwner拒否後の無変更、承認後のversion切替／撤去、Audit／process cleanupを試験する設計。ただしこの作業では起動していない。macOSの正式Installer/update recovery、およびAndroid/iOSのsigned product update／rollback／撤去は未成立。OS／Broker失敗で危険な更新を成功扱いしない。 |
| 標準OS候補 | 5 OS候補、版／arch／安全／GUI/session／Runtime／資源／配布／更新／検査媒体は上記「OS標準候補」表に逐OS記録。Linux nativeは必須対象で、対象をWSLに限定しない。最低resourceはWindows 8GBが設計候補のみ、他OSを含め実測値はUNKNOWN。 |
| 配布物 | Windows: portable ZIP／`D4PKG01`候補、正式一般`.exe` installerなし。Linux: unsigned Ubuntu 24.04／x86_64 `tar.gz`とbootstrap用APT `.deb`、`D4PKG01`は製品更新payload。macOS: debug `.app` build sourceのみ、正式installerなし。Android: debug APK／AAB。iOS: Simulator debug Artifact。正式Publisher／Store identity／production signingは別の責任条件で、現候補に未設定。 |
| 既存実装 | 共通の`package verifier`／`installed artifact identity`、Windows／Linux Brokerの`product_install`／`update_center`／`version activation`、Linux `launcher`／`portable`／APT `setup builder`、`Setup Doctor`／`first-run projection`、Android／iOSのDevice Link／`secure store`、Windows／Linux／macOS／Androidの手動検査定義が存在する。製品導入への対応は`ProductPackagePlatform::{Windows,Linux}`に限られる。 |
| BUILD_GAP | 五OS共通の通常配布物からfirst-run→実Task→同候補update／repair／rollback／uninstallへ至る現候補の接続・証拠が不足。Windows formal installer／別profile acceptance、Linux native package／実desktop X11・Wayland受入、macOS installer／signed bundle／lifecycle、Android/iOS production identity・配布・update／rollback、各OSの最低資源と保証matrixが未完。Windows／Linux sourceの存在は全OS完成を意味しない。 |
| ソース識別値 | 照合HEAD=`ca9d3cfd080ca9f5bd6dad3850e5ebe79a24f125`。既存33＋追加28＝61 unique Source／test／Schema Git blobを表内のSHAと照合。古いSource baselineは履歴として保持し、製品コード変更なし。 |
| 試験区分（TestClass） | Rust／Python `UNIT`・`FIXTURE`、Flutter `WIDGET`、Android instrumentation／Emulator、macOS/iOS `XCTest/UI`、Linux manual `LIVE_PRODUCT`／`workflow_dispatch`のsourceあり。N10 Source mapping単位では製品試験・各OS workflow・installed runtimeを実行していない（`NOT_RUN`）。 |
| 証拠ID（Evidence ID） | `NONE`。W01/G1の限定Windows証拠、Linux READMEに記録された過去Actions runのbootstrap／限定lifecycle scopeは元source／Artifact／OS／scopeの履歴であり、current N10 Evidence IDへ昇格しない。 |
| 阻害要因（Blocker） | N10／W06／W07は`OPEN`。Registry 18件中16 unresolved／2 resolved、`release_ready=false`を維持。関係する既存名は`windows_evidence_provenance_isolation`、`windows_installer_first_run_smoke`、`windows_setup_doctor_smoke`、`windows_broker_installed_smoke`、`windows_distribution_identity`、`linux_native_product_support`、`rev2_mobile_evidence`、`rev2_mobile_distribution`、`rev2_mobile_flutter_native_device_link_boundary`。新規Blocker・既存status変更なし。 |
| 状態 | N10 E0-04 21項目、既存五OS source mapping、61 blob識別を完了。N10製品Acceptance、W06／W07、E0全体は`OPEN`。Evidence IDは`NONE`。 |
| 残余 | 同じSourceSHA／各OS固有ArtifactSHAを結合した通常配布→installed AI Task→更新／修復／rollback／uninstall、Windows別profile／所有実機確認、Linux native X11／Wayland／package、macOS product installer、Android/iOS通常配布・lifecycle、署名／Publisher identity、最低resource、N11 Evidence／Blocker／Artifact結合。これらはE1/E2／後続OS QAへ渡し、N10のsource mapping PASSへすり替えない。 |

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
| P10 | Module Selection / Pruning。旧rev5で機能上の除外範囲を`CLOSED`。選択dependency closure、compile-time UI除外、必須画面保持。 | v4仕様S05の直接対応はN06、関連能力K15。Phase 33のD4追加範囲で、同等のrev1 C工程なし。施工線E/F。 | Export Module plan→build define/receipt→Flutter UIの除外経路。Exporter/Launcherへの現source接続とhashはN06節。 | 旧plan/Widgetは`CONFIG`/`FIXTURE`。Windows AOT source面の一致とLinux WSL2 fixtureのsize差は限定観測で、binary実除去・Rust/第三者依存pruning・実行時resource保証ではない。`binary_pruning_verified=false`、FQ-MODULE-PRUNING／`rev2_module_pruning_binary_and_measurement`を保持。v4 N06/W05は`OPEN`。 |
| P11 | Windows Productization。旧rev5で製品機能受入れ`CLOSED`。P9 portable ZIPをbootstrapに使うBroker-owned Install/Update/Repair/Rollback/Uninstall導線。 | v4仕様S05の直接対応はN10、関連能力K16。Phase 34はC33/C34の配布範囲と関連。施工線G/F。 | Windows package/portable→bootstrap/Update Center→Broker product install/update→Launcher/Setup Doctor。現候補source・package役割・workflow・hashはN10節。 | 旧acceptanceはMSI/Setup.exeやsigned release artifact作成を主張しない。v4 N10は`OPEN`で、一般配布・同一candidateのinstalled lifecycle・署名/Publisher identityは未成立。`rev2_desktop_product_distribution`と`windows_distribution_identity`を保持。 |
| P12 | 製品全体統合。REV5進捗ではWindows側`FEATURE COMPLETE`。正式release/Owner GOと分離された製品機能統合の履歴。 | v4仕様S05の直接対応はN01/N09/N10、関連能力K19。Phase 39〜45はC27〜C34の全体QC/回帰/文書/出荷範囲に関連する後続gateで、旧P12 Acceptanceの再実施範囲ではない。単一施工線へ閉じない。 | Windows product entry→Task/result/operation UI→OS product lifecycle。実TaskはN01、operation/observation/recoveryはN09、installed OS distribution/lifecycleはN10の現ConsumerとSHA節へ分割記録。P12固有の代表UI/tray source hashは個別E0-04節へ記録済み。 | `FEATURE COMPLETE`を正式Release/QAへ昇格しない。v4 N01/N09/N10は`OPEN`、Q0〜Q7は`DEFERRED`、`release_ready=false`。統合後の全候補・全OS acceptanceは未成立。 |
| P13 | Mobile / Non-Windows Product Build。REV5履歴は2026-10-09に有限Product Build acceptanceを閉鎖したと要約する。旧Acceptanceの詳細evidence identity/classは本照合資料から`UNKNOWN`。個別のmacOS、Android、iOS作業記録はREV5 P13節に残る。 | v4仕様S05の直接対応はN10、関連能力K17。Phase 35/36はC24〜C26範囲に関連。施工線H/I。Linux nativeは完全対応必須。 | Linux/macOS Desktop、Android/iOS Mobile runner/native transport→Rust Broker／OS capability→installed product lifecycle。OS別source、package、workflow候補とSHAはN10節および以下のP13個別補完。 | 旧P13閉鎖を現行五OS標準installed QAへ移さない。Linux nativeは完全対応必須で、WSL/Xvfb/Headlessだけでは成立しない。Android/iOS実機を前提にせずEmulator/Simulator等を用いる。N10は`OPEN`、`linux_native_product_support`、`rev2_mobile_distribution`、`rev2_mobile_device_evidence`、`rev2_mobile_flutter_native_device_link_boundary`を保持。 |

P0〜P13全14 IDの旧要求範囲・限定statusをv4 N/K/Consumerへ接続した。これは旧statusの書換え、v4 acceptance、N01〜N11のCLOSED、E0完了のいずれも意味しない。C0〜C34、Phase0〜45、K01〜K20のSource→Consumer/scope mappingを記録した。P0〜P13は全E0-04 fieldを個別補完済み。N11 Evidence/Blocker/OS Artifactの要求単位結合、残る要求行の詳細台帳化、E0完了判定は継続する。P0/P1/P4の直接対応はv4仕様S05の明示mappingへ合わせて訂正し、旧statusは変更していない。P13旧Acceptance全体の詳細evidence identity/classは`UNKNOWN`として保持する。

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

### P8 E0-04項目の個別補完（2026-11-13、Compose／AI Edit）

以下は旧P8「GUI-Shell Compose」をv4へ結ぶE0照合である。旧Product Build範囲・status・限定証拠はdocs/REV5_PRODUCT_PROGRESS.mdのP8節、Phase 29〜31のD4追加範囲はdocs/D4_POCKET_PHASE_MAPPING.mdに照合した。v4仕様S04の直接対応はN05/N07、能力対応はK14、施工線はD。Compose Manifestを作ることと元Toolを実行すること、AI proposalを受理することと変更を制御適用することを分ける。P8 UIにあったModule選択は、Manifest・Export Module planの境界に限り記録し、独立Export／実binary pruningのN06・P9/P10受入れをP8へ重複計上しない。照合HEADは9eb663789b64adf15d0abb14b64d23400fa3c978。

| E0-04項目 | 記録値 |
|---|---|
| 現行ID | 旧製品要求P8。v4仕様S05の直接対応先はN05/N07、関連能力はK14。Phase 29〜31／施工線Dは工程上の対応であり、旧P8のstatusを現在受入れへ移さない。 |
| 旧ID | P8 GUI-Shell Compose。rev5 Product BuildはCLOSED。Compose UI、Manifest/Preview、module選択の受渡し、AI Edit proposalの当時受入れ範囲として保持する。 |
| 原文参照 | docs/REV5_PRODUCT_PROGRESS.mdのP8節、docs/D4_POCKET_PHASE_MAPPING.mdのPhase 29〜31、v4仕様S04 N05/N07・S05 K14、工程表R03線D。旧P8試験の範囲・制約はrev5進捗の記録を維持する。 |
| 対象 | Runtime/Agent/MCP Tool候補のCatalog表示、能力・依存のCompose、Manifest作成とPreview、Theme/表示密度/Content Exposure要求等のUI設定、Compose画面からのModule選択表示、およびOwner/Developerが明示開始するAI Edit proposalと別Workspace Task準備。 |
| 目的 | 現在のRuntime/Agent/MCP CatalogをBroker境界で照合して構成Manifestを作り、選択結果から元Tool作用へのBroker経路へ接続する。AIの編集案は未信頼proposalとして受け取り、独立レビュー・fresh認可・制御適用が成立するまでfile変更とAuthority生成を行わない。 |
| 生成側 | Desktop SettingsのCompose/Preview/AI Edit操作、BrokerのRuntime/Agent/MCP Catalog、Owner/Developerが入力する変更指示と対象範囲。Catalog metadata、proposal本文、Agent結果は未信頼入力として扱う。 |
| 実際の受け取り先 | Composeはcompose_client.dartからRust BrokerのCatalog/Compose/Preview operation、compose_center.rsへ届く。受理Manifestを使うTool操作はSettingsからMCP client/BrokerのMCP Tool実行へ進み、mcp_center.rsが現在Runtime/Agent/接続/Toolを再確認してから既存のnative Owner・一回Permission・Audit境界を通し元MCP Toolを呼ぶ。AI Editはai_edit_client.dart→BrokerのGUI Shell編集提案→ai_edit_center.rsでproposal receipt/Auditとなり、main.dartからAgent Centerへ渡される。Agent Centerは別Workspace/Sessionの通常Agent Taskを事前検査するが、直接applyしない。 |
| 契約とデータ | gui_shell_compose.schema.json、gui_shell_compose_catalog.schema.json、gui_shell_compose_receipt.schema.json、agent_adapter.schema.json、MCP接続/Tool CatalogとCompose Manifest。AI Editはgui_shell_edit_proposal.schema.json、gui_shell_edit_proposal_receipt.schema.json、agent_task_request.schema.json、Workspace Permission/Owner Approval契約を使う。Capability宣言はmetadataでありAuthorityではない。 |
| 正常時の作用 | Composeは同一Runtime/Agent instanceの現在能力とMCP接続・Tool選択を検査しmanifest_onlyのManifestを返す。Previewは受理済みManifestと候補の差分を返し、build/export/rollbackを行わない。選択Tool実行時にBrokerが同じManifestを再評価し、認可・Audit後に元Toolを呼ぶ。AI EditはOwner/Developer明示proposalをhash/Audit付き審査待ちで記録し、本文をAuditへ保存せず、fileを書かず、Agent Centerで別Task準備へ渡す。 |
| 否定時の作用 | unknown/unsupported Runtime・Agent・Capability、異なるinstance間の互換性寄せ集め、stale/unknown MCP connection/Tool、現在Manifestとの不一致をBrokerが拒否する。Owner確認・Permission・Audit不成立時はToolを呼ばない。AI Editは通常資格、自己承認、proposal_only以外、許可外/secret path、不正fieldやstaleなproposal/result/sessionを拒否し、提案受理やTask事前検査を変更完了へ昇格しない。 |
| 停止と回復 | Compose/Tool要求のBroker/Audit失敗は成功表示せず作用前に停止する。Tool結果/Auditが不確定な場合は通常MCPのunknown/suspended境界を使い自動再送しない。proposal拒否・事前検査失敗はfileを変更せず、Task下書きは未消費のまま保持または破棄可能。別Workspace Taskの実行には現在のWorkspace Permissionと独立一回Approvalを新規に通す。Proposal専用controlled apply/rollbackとそのRecovery consumerは未成立。 |
| 標準対象OSの候補 | v4標準候補はWindows/macOS/Linux native/Android/iOSの五対象。確認したP8のSettings/Compose/AI Edit consumerはDesktop Flutterと共通Rust Broker。MobileのCompose/AI Edit consumerは未確認または未接続。各OSの標準version/arch/GUI条件とinstalled parityはN10でUNKNOWN/未成立のまま。 |
| 配布 | Manifest作成・Preview・proposal・Task事前検査のsource/UIは独立Appの通常配布、導入済み製品による一連作用の証拠ではない。P8のModule選択表示はP9/P10の独立Export・Manifest実消費・binary pruningを成立させず、通常Installer/installed identityはN06/N10へ分離する。 |
| 既存実装 | N05はCatalog/Compose/Preview/MCP Tool再検査と実行までDesktop/Rust sourceで部分WIRED。Composeのmanifest_only、metadata非権限性、stale選択拒否のtest sourceがある。N07はOwner-only proposal、receipt/Audit、Agent Centerへの未信頼受渡し、別Workspace/unused Sessionの通常Agent Task事前検査がsource接続済み。 |
| 実装不足 | N05はCapability metadataと実Capabilityの一致を保証する専用正規Registry consumerが未確認。AI自発要求と同一installed candidateでの実MCP作用・結果・Auditまでは本source mappingで実証していない。N07は別実行主体による独立review/test結果をapply前に確定するBroker状態、提案由来と実changed files/diffを照合するcontrolled apply、AI Edit専用rollback/recoveryがBUILD_GAP。Mobile consumerと五OS installed経路も未成立。 |
| ソース識別SHA | 次のP8表に現HEAD 9eb663789b64adf15d0abb14b64d23400fa3c978上のsource/schema/test/validator 40件を記録した。40件中39件はv4採用基準d503a4efから差分なし。Conformance validator 1件は採用基準後に更新されており、現HEAD blobを記録した。 |
| 試験区分 | rev5 P8履歴はCompose client/Widget 12件、Rust Compose/Preview 7件、Export/Module plan 13件、AI Edit 2件、Schema 158/155/205、Conformance 234 checksのPASSを記録する。変更Dart 4 file analyzeとMobile全体analyzeはPASS、Desktop全体analyzeは未変更agent_center.dartの既存deprecated情報5件でexit 1、Windows debug buildはASCII一時複製でPASS（MSB8029警告）、OneDrive上のFlutter試験はbuild/unit_test_assets削除失敗のためASCII一時複製を使った。いずれも当時の限定履歴で、本source mapping単位の製品testはNOT_RUN。 |
| 証拠ID | NONE。旧P8のfixture・Schema・local debug buildは当時のProduct Build範囲であり、v4 N05/N07/W04/W05のEvidence IDやinstalled/runtime Acceptanceを発行していない。 |
| 阻止要件 | P8専用Release Registry IDは確認できない。N05/W04/G4、N07/W05/G5は現行工程の受入れ座標としてOPEN。広域のcomprehensive_extension_rev1_completion等のRegistry statusは変更せず、P8へ個別帰属できないblockerはUNKNOWNのまま保持し、新規blockerを追加しない。 |
| 状態 | 旧P8 Product Build=CLOSEDと当時の試験結果/制約を維持。現行N05/N07/W04/W05/E0=OPEN。N05は部分WIRED、N07はproposal/Task事前検査までWIRED、独立review結果とcontrolled apply/rollbackはBUILD_GAP。 |
| 残件 | N05の実Capability Catalog/互換性から通常installed MCP Tool作用までの実Consumer・結果・Audit、N07の独立review/test Task結果・fresh target approval・Broker controlled apply/rollback/Recovery、Mobileおよび五OS標準installed範囲を後続BUILD/QC/QAで接続・確認する。P8のSource mapping完了をN05/N07 AcceptanceまたはE0全体CLOSEDへ拡張しない。 |

#### P8 ソース識別値・該当ファイル

下表のGit blob SHAは照合HEAD 9eb663789b64adf15d0abb14b64d23400fa3c978から取得した。source/testの存在と過去PASSは本単位での実行・製品受入れを意味しない。

| ファイル | 識別値 |
|---|---|
| docs/REV5_PRODUCT_PROGRESS.md | 9c978021d3982690e9aaa87896436bad7125b433 |
| docs/D4_POCKET_PHASE_MAPPING.md | 064ea334f86b137d94fba2370d2a2fd732f6e972 |
| docs/specs/gui-shell-compose.md | 6b43bb80f73d4268e734ac0afbc58b33ba07339f |
| docs/specs/gui-shell-ai-edit.md | 33d2df562d2231d0d894afcbf6097c512a8efd8b |
| specs/agent_adapter.schema.json | e0792589a2bae168ba221af4f4f9abb505815c12 |
| specs/capability.schema.json | a243a84b22cdd704875f35afc56e3450f9c9e1ca |
| specs/gui_shell_compose.schema.json | 89b3d8e0e09c802d555fd6fee233daebc0fec045 |
| specs/gui_shell_compose_catalog.schema.json | f9c76f5fa87bfb0866f33745dcd5a8a4e74864d5 |
| specs/gui_shell_compose_receipt.schema.json | 7c4ac9d0f28f38f79a0074ed7d6c84d9124aadaa |
| specs/gui_shell_edit_proposal.schema.json | b1a0b41570d5e1e1eadbf2ded3e8984d9fe21130 |
| specs/gui_shell_edit_proposal_receipt.schema.json | d9f426e20254c8cf009b90e7be4cbccf453db286 |
| specs/agent_task_request.schema.json | cb25bb3684656bfd670ea5832bbb7fc5988845b6 |
| specs/agent_task.schema.json | a552389072088d6cd42b3588fefda332be4ae6b6 |
| specs/agent_task_workspace_permission.schema.json | 01be05df95431e38ca5dc3ec1b5f04416b506fb2 |
| specs/agent_task_owner_approval.schema.json | d190c9a8cc3d715eb199459c37c14e5de0ad6ad8 |
| specs/agent_task_content_exposure.schema.json | a85b99c0213d18daaab3d5d863adcd3661f379a0 |
| apps/desktop_flutter/lib/screens/settings.dart | 66374ec4157d573939e7f9cc5c36aecf5d65e251 |
| apps/desktop_flutter/lib/main.dart | 6109c821ab5dc809224bec5234c4c59db1d5f988 |
| apps/desktop_flutter/lib/screens/agent_center.dart | 2472faeb31a64d624423c30a0a30d0f153d2e2a3 |
| apps/desktop_flutter/lib/services/compose_client.dart | 8f9657d866b007a4d2212abefca4df9add81a702 |
| apps/desktop_flutter/lib/services/ai_edit_client.dart | ba5e62311a02749d2f9fe0e4e72e2b0d4e0a497d |
| apps/desktop_flutter/lib/services/agent_coordination.dart | bdc13464f0e42b740a2bca3b7e8828a5019056d0 |
| apps/desktop_flutter/lib/services/agent_task_client.dart | f54cefac29eba3dcf383bba19b4f64ee682e6217 |
| apps/desktop_flutter/lib/services/agent_task_execution_coordinator.dart | 5d8f48d0557bdcb1af838239abf9651893bce7f1 |
| apps/desktop_flutter/lib/services/mcp_connection_client.dart | 12a173a3956368c53615255c56ef06a2f5939c9a |
| native/rust_helper/src/broker/compose_center.rs | d80658fd9a342140acc35560b2e4c4e4ae97d888 |
| native/rust_helper/src/broker/ai_edit_center.rs | 7a1c2418dc6f71e14fc971385b6f4f2454d55b1c |
| native/rust_helper/src/broker/dialogue.rs | c3c3fddb630dd06b7e6cc1c13b39f3d4f6f6a999 |
| native/rust_helper/src/broker/mcp_center.rs | 8f9ab897bdc625e448241ebd9b46f995cd1f22f6 |
| native/rust_helper/src/broker/protocol.rs | 9b2076d5f31ca6d77f1f87973d1b394aaf751961 |
| native/rust_helper/src/broker/ipc_server.rs | 58a404b920ef5905e174668ec2de0e04a9e8b044 |
| native/rust_helper/src/adapters/codex_cli.rs | e5c9d1581ae9a1c0a3ae7fcfa626fdca6fea6d28 |
| apps/desktop_flutter/test/compose_client_test.dart | c3fe30faa668482e5a4e42b2f5f5e5b218a01dc8 |
| apps/desktop_flutter/test/settings_compose_test.dart | a78353f93617cc7b17391549ea256f007a7acdb1 |
| apps/desktop_flutter/test/ai_edit_client_test.dart | c5ecd0353fdc811ec7c4d52a9c2b7d10e1d01723 |
| apps/desktop_flutter/test/agent_coordination_test.dart | 6783e87a492d8436a855a7088b7cece38d122bd5 |
| apps/desktop_flutter/test/agent_task_client_test.dart | 0477a0bbbb321082494b6b55711815d34419fdd9 |
| apps/desktop_flutter/test/mcp_connection_center_test.dart | c81a37ca78b15e57c838f48519aa3e4abb6d6665 |
| apps/desktop_flutter/test/widget_test.dart | f4e9c7b93f832ba2a2cefee779fd12426f240b04 |
| tooling/conformance_tests/run_conformance_skeleton.py | c6f31d8cb561b48513c6784e54c85969c9cb953e |

### P9 E0-04項目の個別補完（2026-11-13、Standalone Export）

以下は旧P9 Standalone Exportを現行v4へ結ぶE0照合である。旧Product Build条件・status・限定証拠はdocs/REV5_PRODUCT_PROGRESS.mdのP9節、Phase 32のD4追加範囲はdocs/D4_POCKET_PHASE_MAPPING.mdに照合した。v4仕様S04のN06、S05のP9/P10→N06とK15、工程表R03線Eが現行の対応先である。Phase 32がN10にも接続する配布・導入要件と、Phase 33／旧P10のbinary pruning要件は、P9の旧CLOSED範囲へ混ぜない。照合HEADはdadfc7154771bb6ab3bb445075779d66efc0775e。

| E0-04項目 | 記録値 |
|---|---|
| 現行ID | 旧製品要求P9。v4仕様S05の直接対応はN06、関連能力はK15。Phase 32／施工線Eは関連工程で、Phase 32のN10配布consumerはN10側で別に追跡する。 |
| 旧ID | P9 Standalone Export。rev5 Product BuildはCLOSED。固定fixture入力を使ったWindows portable bundle生成・限定起動の当時範囲を保持し、Owner承認済みの実構成による一般製品受入れへ拡張しない。 |
| 原文参照 | docs/REV5_PRODUCT_PROGRESS.mdのP9節、docs/D4_POCKET_PHASE_MAPPING.mdのPhase 32／C33・C34対応、v4仕様S04 N06・S05 P9/P10/K15、工程表R03線E。旧P9の試験・失敗・cleanup・証拠境界はREV5進捗の記録を保持する。 |
| 対象 | Compose Manifestから新App/Audit Store identity、設定・Runtime manifest、選択Module planを持つ独立Appを作る経路。Settings Export UI、Rust Broker、Windows/Linux Developer ExporterとLauncher、Manifest/runtime構成消費、利用者向けInstaller/Export Appを別consumerとして扱う。 |
| 目的 | 選択構成を独立identity・Broker・Runtimeへ束縛したAppとして生成し、実行時に構成を消費して要求機能を動かす。Authorityを継承せず安全Coreを保ち、構成の実使用・独立導入・Moduleの選択結果を証拠と結ぶ。 |
| 生成側 | Desktop SettingsのCompose/Export操作、compose_client.dartで得るManifest、export_client.dartのplatform/Module選択、Rust BrokerのOwner確認済み要求がproducer。Manifest、Receipt、build evidence、bundleは信頼権限ではなく構成・証拠データである。 |
| 実際の受け取り先 | export_client.dart→既存Brokerのexport operation→broker/export_center.rsが固定Export領域へManifest/Receiptを作成。tooling/export_windows_product.py／export_linux_product.pyがReceiptとManifestを検査しportable bundleを生成し、desktop_launcher.rs／linux_desktop_launcher.rsがbundle内Manifest hash・identity・非継承条件を確認する。RuntimeはComposeのRuntime/Agent/Tool/Capability/Module構成を実消費しておらず、そのproducer→consumer接続はBUILD_GAP。 |
| 契約とデータ | gui_shell_export.schema.json、gui_shell_export_manifest.schema.json、gui_shell_export_receipt.schema.json、gui_shell_module_catalog.json/schema、runtime_manifest.schema.json、Windows/Linux build evidenceとProduct Package Manifest Schemaを使う。App ID・Audit Store ID・platform・Manifest hash・Module dependency closureを束縛する。Credential/Permission/Approval/Audit chainの実値や権限はManifestへ継承しない。 |
| 正常時の作用 | Source上、Brokerは新規App ID/Audit Store IDを生成し、権限非継承を記録したManifest/Receiptを固定先へ出力する。Module planはrequired/safety Moduleを保持しoptional依存閉包と選択/除外IDを記録する。Windows/Linux Exporterは検査済Manifestをbyte-for-byte bundleへ入れ、build evidenceとcompile-time identityを作る。Launcherはhash/identity/platform/非継承条件を照合する。runtime_manifest_consumed=falseのため、Compose構成から実Runtime機能を設定する作用は成立していない。 |
| 否定時の作用 | 不正Schema/Receipt/Manifest、hash・target・identity不一致、unknown/duplicate/required Module選択、reparse point等の不正入力、既存出力の上書き要求、Owner確認/Audit不成立は拒否または失敗として扱う。Receipt/metadataからAuthorityを作らず、Credential実値をbundleへ含めないという限定契約を維持する。既知pattern scanは未知形式の秘密不存在を証明しない。 |
| 停止と回復 | BrokerのOwner/Audit・出力作成前検査が不成立ならExporterへ進めない。Exporterは検証失敗時にbundle成功を報告せず、Brokerのtemporary manifest cleanup情報と既存Recovery扱いへ留める。Runtime構成消費・Installer・rollback成功を推定しない。過去P9起動確認のcleanupはnormal exitではなく、異常終了dialogを伴った限定cleanupとして履歴保持する。 |
| 標準対象OSの候補 | v4対象はWindows/macOS/Linux native/Android/iOSの五OS。現行Export sourceはWindows/Linux Developer Exporterと各Launcherまで確認。Windowsには合成fixture bundleの限定build/起動履歴、LinuxにはExporter/archive fixture sourceがある。macOS・Android・iOSの独立Export consumer、全OS標準installed parityはUNKNOWN/BUILD_GAP。WSL/XvfbはLinux native製品の受入れではない。 |
| 配布 | 旧P9はWindows portable ZIP相当のDeveloper bundleであり、通常Installer、別profileへの独立導入、Publisher/署名、一般利用者への正規配布ではない。Linux sourceのbundle/archive fixtureもnative installed distributionではない。Installer・update/rollback/uninstallと五OS標準packageはN10/Phase 32側の別Consumerとして保持する。 |
| 既存実装 | Broker新規identity/Manifest/Receipt/Module dependency plan、Windows/Linux Developer Exporter、LauncherのManifest hash/identity検査はsource上WIRED。Windowsの旧P9 bundleはRust LauncherからFlutter childを起動し専用runtime root/Audit anchorを作る限定LIVE_RUNTIME履歴がある。入力はsynthetic checked-in fixtureで、Owner承認済みproduction Exportではない。 |
| 実装不足 | v4 N06ではManifestのRuntime構成実消費、生成Appでの要求機能・Agent/Tool実作用、正規Installer/installed product経路、選択Moduleだけの実binary/resource効果と測定がBUILD_GAP。runtime_manifest_consumed=false、Windows standalone_app_verified=false、両Exporterのbinary_pruning_verified=falseを維持する。Module pruning詳細は旧P10/Phase 33、InstallerはN10へ分離する。 |
| ソース識別値 | P9に関係する仕様履歴、Schema/example/negative、Flutter、Rust Broker/Launcher、Windows/Linux Exporter、test、Conformanceを現HEAD dadfc7154771bb6ab3bb445075779d66efc0775eのGit blob SHAで下表に記録する。SHAは現物同一性であり、test実行・製品Acceptanceではない。 |
| 試験区分 | 旧P9記録はFlutter 3.44.0/Dart 3.12.0、Rust/Cargo 1.95.0のWindows Release build、14 files/62,856,402 bytes、Manifest byte copy、known-pattern 0件、synthetic fixture bundleのprocess/window/runtime root/Audit anchor観測を記録する。Owner production Export、正常終了、一般Installer、runtime Manifest消費は未確認。現SourceのRust unit、Linux Python fixture、Conformance testは存在するが、本mapping単位では全てNOT_RUN。 |
| 証拠ID | NONE。旧P9の限定bundle launch evidenceはrev5 Product Build履歴だけに属する。v4 N06/W05向けEvidence ID、通常配布・構成消費・全OS受入れの証拠は発行しない。 |
| 阻止要件 | 既存rev2_export_owner_ui_authority_pathとrev2_module_pruning_binary_and_measurementを関連する既存Release blockerとして参照し、各々の既存分類/statusを変更しない。通常Installer/installed lifecycleはrev2_desktop_product_distribution等のN10範囲で扱う。新規blockerは追加しない。 |
| 状態 | 旧P9 Product Build=CLOSEDと限定証拠/cleanup制約を維持。現行N06/W05/G5/E0=OPEN。Source上のManifest/identity/bundle接続はWIREDだが、Manifest実消費・standalone installed acceptance・binary pruningは未成立。 |
| 残件 | N06の構成実消費と独立Appの正常/否定実Task、Authority非継承、Module機能保持/実除外・size/resource計測をE線で接続する。N10の正規Installer/OS別installed lifecycleとは証拠を分ける。旧P9を再試験せず、P10 pruning、P11〜P13、Evidence/Blocker/全OS結合へ旧状態を混在させない。 |

#### P9 ソース識別値・該当ファイル

下表は照合HEAD dadfc7154771bb6ab3bb445075779d66efc0775eの`Git blob SHA`である。過去fixture・source/test fileの存在は本単位の実行や製品受入れを意味しない。

| ファイル | 対象ファイル識別値 |
|---|---|
| `docs/REV5_PRODUCT_PROGRESS.md` | `9c978021d3982690e9aaa87896436bad7125b433` |
| `docs/D4_POCKET_PHASE_MAPPING.md` | `064ea334f86b137d94fba2370d2a2fd732f6e972` |
| `docs/specs/gui-shell-export.md` | `3d7a1156e4bf095e4f6a64792b6e1683c14f9b17` |
| `docs/specs/gui-shell-module-pruning.md` | `5568c677be30402d709cc365b79eceb0e96ce831` |
| `specs/agent_adapter.schema.json` | `e0792589a2bae168ba221af4f4f9abb505815c12` |
| `specs/gui_shell_compose.schema.json` | `89b3d8e0e09c802d555fd6fee233daebc0fec045` |
| `specs/gui_shell_export.schema.json` | `8b96d344ac0f6a54011237be8b2d85964be48c4b` |
| `specs/gui_shell_export_manifest.schema.json` | `bf136b1a1daa258d33af3635ee414a2d4312bec5` |
| `specs/gui_shell_export_receipt.schema.json` | `0448ec5f631509fa6fbe25cb4957bd6cf51e1562` |
| `specs/gui_shell_module_catalog.json` | `339cb8894b25ae963956e582c42b9e8fcc0d3931` |
| `specs/gui_shell_module_catalog.schema.json` | `bd54dd7903a1734ea4027d3f780b7a2b49e10c8e` |
| `specs/gui_shell_windows_export_build.schema.json` | `5c7234692a8f178882edfc42c1a762038e7414fa` |
| `specs/gui_shell_linux_export_build.schema.json` | `6f1af3d3da2277807dabac5d753897f5ddee453c` |
| `specs/gui_shell_module_build_evidence.schema.json` | `1a37d4b831f998ce334fc9f0126c5dd2ee09a01b` |
| `specs/runtime_manifest.schema.json` | `27c5566d1243274a06bf68aaa55c4adc3c47db06` |
| `specs/d4_pocket_product_package_manifest.schema.json` | `3151ba81a869b87e6c0fa8f8633a0e424fc43ae7` |
| `specs/d4_pocket_linux_product_package_manifest.schema.json` | `34087f190e7189641b34ae3163b568a038c9f1c2` |
| `examples/contracts/gui_shell_export.valid.json` | `ea10286920a82f019ce9ee8b1e8cc4cf710adde4` |
| `examples/contracts/gui_shell_export_manifest.valid.json` | `8e845373fdfcbfd77f555d3030ef3d35b3dd4ed4` |
| `examples/contracts/gui_shell_export_receipt.valid.json` | `fae30c21b719d2ae9bf9bcff99228bf6eb6b0df1` |
| `examples/contracts/gui_shell_windows_export_build.valid.json` | `31915727f6f0899bf4358996f709a23a76e9554d` |
| `examples/contracts/gui_shell_linux_export_build.valid.json` | `fd710b0597b140d41f610085b7f3fcd6722b97bb` |
| `examples/contracts/d4_pocket_product_package_manifest.valid.json` | `da9fc2eeec548deae980f9dc90898fe08f2d7629` |
| `examples/contracts/d4_pocket_linux_product_package_manifest.valid.json` | `05d4fc3c08efd5338943d44c8f105af1672c0812` |
| `examples/contracts/invalid/gui_shell_export_credential.invalid.json` | `8c03e7f4ae329271a1b78a6869c8ae4050d7d164` |
| `examples/contracts/invalid/gui_shell_export_receipt_inherited.invalid.json` | `52faca660ce45470ef5ce5136a9946181ebbdbb6` |
| `examples/contracts/invalid/gui_shell_export_receipt_pruning_claim.invalid.json` | `6766cfe13dd417fe762cb30dddeb2644a14fb354` |
| `examples/contracts/invalid/gui_shell_export_unknown_module.invalid.json` | `ae5a354654aaddf15821eb64a90f29c00a2fcc0e` |
| `apps/desktop_flutter/lib/screens/settings.dart` | `66374ec4157d573939e7f9cc5c36aecf5d65e251` |
| `apps/desktop_flutter/lib/services/compose_client.dart` | `8f9657d866b007a4d2212abefca4df9add81a702` |
| `apps/desktop_flutter/lib/services/export_client.dart` | `70d77edc6beb9e209e6b362219d1b22d69375489` |
| `apps/desktop_flutter/test/export_client_test.dart` | `b4ec8686b91370b1eaff06dd88b301ee6ef0370c` |
| `apps/desktop_flutter/test/settings_compose_test.dart` | `a78353f93617cc7b17391549ea256f007a7acdb1` |
| `native/rust_helper/src/broker/export_center.rs` | `477e44d86ebf18d2b004a7294c9cdb14e35aa0f0` |
| `native/rust_helper/src/broker/ipc_server.rs` | `58a404b920ef5905e174668ec2de0e04a9e8b044` |
| `native/rust_helper/src/desktop_launcher.rs` | `6a9f335de07fa474ca10043aea63ad6e988b572d` |
| `native/rust_helper/src/linux_desktop_launcher.rs` | `f58fb226f21093c2d1f062259d394b4f8ec022f6` |
| `tooling/export_windows_product.py` | `31856179881a5a5aae1ba4451732e6de6cd7f338` |
| `tooling/export_linux_product.py` | `47fe9a3032a5b2680e3f7d39b7cd64120ac83843` |
| `tooling/build_module_pruned_windows.py` | `490aae7209ea5988f8e2cdcb05fe092b3d29504a` |
| `tooling/tests/test_export_linux_product.py` | `079a187de49cf8b2fb6a7ddadb4175d6916163d1` |
| `tooling/tests/test_linux_portable_archive.py` | `c04ac3c5d6d58396f650e9530b6731c6bb2037d6` |
| `tooling/conformance_tests/run_conformance_skeleton.py` | `c6f31d8cb561b48513c6784e54c85969c9cb953e` |

### P10 E0-04項目の個別補完（2026-11-13、Module Selection / Pruning）

以下は旧P10 Module Selection / Pruningを現行v4へ結ぶE0照合である。旧Product Buildの機能上の除外CLOSEDと限定証拠は`docs/REV5_PRODUCT_PROGRESS.md`のP10節および2026-11-11追補に照合した。v4仕様S05の直接対応はN06、能力対応はK15、工程表R03の線E/F、Phase 33のD4追加範囲である。機能上のUI除外、AOT source surface観測、artifact size差、意味上のbinary pruningを別の主張として記録し、旧P10のCLOSEDを実binary除去へ拡張しない。照合HEADは`f418f5e879dc2c3ae64302284c0b315bf1f5be74`。

| 照合項目 | 内容 |
|---|---|
| 現行ID | 旧P10。v4仕様S05の直接ConsumerはN06、関連能力はK15。工程表R03線E（独立製品）と線F（安全Core維持・差分緩衝）が関係する。Phase 33は同等のrev1 C工程がないD4追加範囲であり、安全Core保持と実binary／比較証拠を別々に追跡する。 |
| 旧ID | P10 Module Selection / Pruning。rev5 Product Buildは`CLOSED（機能上の除外）`。成立範囲は依存閉包計画、選択結果のcompile-time UI define、Flutter画面上の機能除外と必須画面維持。AOT／binaryやRust／第三者依存の意味上の除去は当時から対象外であり、`binary_pruning_verified=false`を保持する。 |
| 原文参照 | `docs/REV5_PRODUCT_PROGRESS.md`のP10節・2026-11-11 Linux WSL2 fixture追補、`docs/specs/gui-shell-module-pruning.md`、`docs/D4_POCKET_PHASE_MAPPING.md`のPhase 33、v4仕様S04 N06・S05 P9/P10/K15、工程表R03線E/F。P10の過去試験と失敗・限定範囲は履歴として維持する。 |
| 対象 | ExportのModule Catalog／選択→BrokerのModule plan・dependency closure→build define／receipt→Flutter UIの任意画面・操作除外。安全Core・required moduleの保持は独立条件。Runtime構成消費、実binary pruning、installer／installed productは別Consumerとして扱う。 |
| 目的 | 任意UI機能を構成選択に応じて除外し、required／安全Coreを保持する。選択情報は構成データであり、Authority・Permission・Approval・Credential・Auditを生成・継承しない。 |
| 生成側 | SettingsのExport UI／`export_client.dart`がModule選択要求を生成し、既存Brokerの`export_center.rs`が固定Catalogに基づくplanを作る。Windows/Linux exporterは検証済みReceiptからbuild defineを構成する。選択内容とbuild evidenceは非権威データである。 |
| 実際の受け取り先 | Broker `resolve_module_plan`がunknown／duplicate／required module不正選択とdependency closureを処理し、Receiptへ含める。Windows Flutter buildはcompile-time define、Flutter `main.dart`／Settings screenは任意UIの可視性・操作に消費する。Widget試験はfake Broker／fixtureでUI consumerを確認する。Launcher／RuntimeはManifestの機能構成を実消費しておらず、製品機能を構成するconsumerは`BUILD_GAP`。 |
| 契約とデータ | `specs/gui_shell_module_catalog.json`、Catalog／Receipt／Module build・comparison／Windows・Linux向けExport build証拠の各Schema、正常・負例JSONを対象とする。任意機能ID・依存関係・包含／除外ID・ビルド定義・生成物識別値を扱う。ビルド証拠Schemaは`binary_pruning_verified=false`を固定し、compile-time define適用後の比較待ちを記録する。Authorityや製品完成の主張を受け付けない。 |
| 正常時の作用 | 旧Windows plan-only試験はTrace Inspectorと依存Observabilityを選択し、optional 8件中6件を除外、required画面と固定安全Coreを保持する計画を生成した。旧Flutter Widget試験2件は8 optional画面無効時に任意navが隠れ、required 13画面が残り、除外画面選択時に例外／未搭載画面表示がなく、除外Host操作Commandが送信されないことを`FIXTURE`で確認した。2026-11-12 Windows portable buildのAOT reportは選択2画面／除外6画面のsource surface一致を`INTERNAL_STATE`として記録するが、意味上の除去ではない。2026-11-11 Ubuntu 24.04.5 WSL2 same-source Linux Release fixture比較では、選択2／除外6のvariant間で`libapp.so`が8,078,224→7,783,312 bytes（294,912 bytes、3.65%減）、bundle inventoryが53,766,771→53,473,069 bytes（293,702 bytes、0.55%減）。両variantのXvfb GTK window-close smokeはPASSしたが、native Linux installed productの証拠ではない。 |
| 否定時の作用 | 現Catalog／Schemaと負例はmissing safety Core、unknown dependency、unknown／duplicate／required選択、Authority claim、product／binary pruning claimを拒否する契約を表す。旧P10で明示実行した試験としてはplanとWidget 2件、Schema 158／正常例155／負例205、Conformance 234 checksの範囲を保持する。これら全ての負例を本mapping単位で再実行したとは主張しない。Manifest／Receipt／AOT reportからAuthorityやbinary pruningを昇格させず、`binary_pruning_verified=false`を維持する。 |
| 停止と回復 | 旧Flutter試験は初回asset付き起動がOneDrive配下の`build/unit_test_assets`削除拒否でtest本体前に失敗し、asset不要の同対象を`--no-test-assets`で再実行して2件PASSした。旧Windows portable UIは別名windowをcomputer-use targetとして取得できず、視覚確認なしとした。Linux WSL2 fixtureは両variantでwindow-close smoke PASS。これらは限定環境の履歴であり、製品crash／Recovery・installed process停止を証明しない。build／Schema検証失敗時は成功扱いしない。 |
| 標準対象OSの候補 | v4の標準対象はWindows、macOS、Linux native、Android、iOSの5 OS。Module UI／Windows build defineのsourceは存在し、Linux exporter／WSL2 fixture evidenceもある。macOS／Android／iOSの独立Module選択→installed product consumerは未確認。WSL2／Xvfbの成功をLinux native GUI・導入・権限境界・配布の受入へ転用しない。 |
| 配布 | 旧P10はModule plan／Flutter UI除外を閉じたもので、独立製品の一般配布ではない。Windows portable bundleはP9のDeveloper fixture、Linux WSL2 archiveは比較用fixtureである。正規Installer、Publisher／署名、OS別installed Export、runtime Manifest消費はN06/N10の別条件であり未成立。 |
| 既存実装 | Catalog／dependency plan、Windows Flutter compile-time define、任意UIのWidget test source、Windows/Linux exporter、build／comparison evidence Schema、否定fixtureはsource上`WIRED`または`CODE_PRESENT`。旧P10のplan・Widgetは限定Product Build条件としてCLOSED。AOT source surfaceとWSL2 fixture size差は後続の限定観測として記録されている。 |
| 実装不足 | RuntimeがManifestから選択構成を消費する経路、選択moduleに応じた実機能／元Toolの起動・除外、意味上のAOT／binary除去、Rust／third-party依存pruning、安全Coreの生成artifact内実測、cold-start／runtime resource比較、正規Installer／installed acceptanceは`BUILD_GAP`。`runtime_manifest_consumed=false`、`standalone_app_verified=false`、Windows/Linux `binary_pruning_verified=false`を保持する。サイズ差とAOT source surface一致だけではこれらを成立扱いしない。 |
| ソース識別値 | P10関連の履歴、契約・正常／負例、Flutter producer/consumer・Widget、Rust Broker、Windows/Linux build・comparison source、test、Conformanceの42 fileを、照合HEAD `f418f5e879dc2c3ae64302284c0b315bf1f5be74`のGit object識別値で下表に結合する。識別値はsource同一性であり、試験実行・製品Acceptanceではない。 |
| 試験区分 | 旧P10記録：Flutter 3.44.0／Dart 3.12.0、対象Widget 2件PASS、Windows plan-only、Schema 158／正常例155／負例205、Conformance 234 checks PASS。2026-11-11 WSL2 Linux Release fixture比較はsize差と両variantのXvfb GTK window-close smokeを記録。2026-11-12 Windows portable AOT reportは選択面一致を`INTERNAL_STATE`で記録。これらの履歴試験は再実行していない。本mappingのRust／Flutter／exporter／installed product testは`NOT_RUN`。文書・Manifest・Conformance・通常Release Gateの検査結果は進捗へ別記し、製品受入へ昇格しない。 |
| 証拠ID | `NONE`。旧P10の限定CLOSED範囲、AOT／size fixture記録を保持する。v4 N06/K15、native Linux、installed product、意味上のbinary pruningのEvidence IDは発行しない。 |
| 阻止要件 | 既存`FQ-MODULE-PRUNING`とRegistryの`rev2_module_pruning_binary_and_measurement`を参照し、現行status・release分類を変更しない。N06/W05の既存未成立範囲を維持する。新規blockerは追加しない。 |
| 状態 | 旧rev5 P10 Product Buildの機能上除外は`CLOSED`。E0-04 P10のsource→consumer mappingは本単位で記録完了。v4 N06/W05/G5/E0は`OPEN`。`binary_pruning_verified=false`、`release_ready=false`および既存blocker状態を維持する。 |
| 残件 | 線E/Phase 33でRuntime Manifest実消費、実binary／依存除去、安全Core保持、選択結果から生じる機能差、測定可能なsize／startup／resourceとOS別consumerを統合する。Linux nativeは必須対象のまま後段で実装・検証する。N10の通常配布・Installer/installed lifecycleとは証拠を分離し、FQ-MODULE-PRUNINGの条件で処理する。 |

#### P10 ソース識別値・該当ファイル

下表は照合HEAD `f418f5e879dc2c3ae64302284c0b315bf1f5be74`のGit object識別値である。Source、Schema、fixture、testの存在は本単位での実行や製品受入れを意味しない。

| ファイル | 対象ファイル識別値 |
|---|---|
| `docs/REV5_PRODUCT_PROGRESS.md` | `9c978021d3982690e9aaa87896436bad7125b433` |
| `docs/D4_POCKET_PHASE_MAPPING.md` | `064ea334f86b137d94fba2370d2a2fd732f6e972` |
| `docs/QE_2026-11-09/進捗.md` | `dbfe0e52ff510d8248a90154681ab4c144e23ce2` |
| `docs/specs/gui-shell-module-pruning.md` | `5568c677be30402d709cc365b79eceb0e96ce831` |
| `docs/FINAL_QA_QUEUE.md` | `7b332cccab70bb68aa1ff4f8e23acc0e816d8282` |
| `release_blockers.registry.json` | `244adc04a01e7b406c4ebcdeb829f7077d4aa35f` |
| `specs/gui_shell_export_receipt.schema.json` | `0448ec5f631509fa6fbe25cb4957bd6cf51e1562` |
| `specs/gui_shell_module_catalog.json` | `339cb8894b25ae963956e582c42b9e8fcc0d3931` |
| `specs/gui_shell_module_catalog.schema.json` | `bd54dd7903a1734ea4027d3f780b7a2b49e10c8e` |
| `specs/gui_shell_module_build_evidence.schema.json` | `1a37d4b831f998ce334fc9f0126c5dd2ee09a01b` |
| `specs/gui_shell_module_comparison_evidence.schema.json` | `801ba33bfc0656a4048aa5ca1920b96b7787c304` |
| `specs/gui_shell_windows_export_build.schema.json` | `5c7234692a8f178882edfc42c1a762038e7414fa` |
| `specs/gui_shell_linux_export_build.schema.json` | `6f1af3d3da2277807dabac5d753897f5ddee453c` |
| `examples/contracts/gui_shell_export_receipt.valid.json` | `fae30c21b719d2ae9bf9bcff99228bf6eb6b0df1` |
| `examples/contracts/invalid/gui_shell_export_receipt_pruning_claim.invalid.json` | `6766cfe13dd417fe762cb30dddeb2644a14fb354` |
| `examples/contracts/gui_shell_module_catalog.valid.json` | `6d5902928fb3edac491e6c56a982f5826c9c4b93` |
| `examples/contracts/invalid/gui_shell_module_catalog_missing_core.invalid.json` | `6a72157b94b65f785249f99f6e06f773cbdb9319` |
| `examples/contracts/invalid/gui_shell_module_catalog_unknown_dependency.invalid.json` | `5264ee627f7e01f0ed0a910aec44d686d2534d7b` |
| `examples/contracts/gui_shell_module_build_evidence.valid.json` | `55aaf255258e67258a247c46acabcea451101ed7` |
| `examples/contracts/invalid/gui_shell_module_build_evidence_authority_claim.invalid.json` | `d8ea3736c96c61b0280450cbb9710532333ae59e` |
| `examples/contracts/invalid/gui_shell_module_build_evidence_product_claim.invalid.json` | `d2c8d2271196322c5f60a279625d004d326a570d` |
| `examples/contracts/gui_shell_module_comparison_evidence.valid.json` | `d635ad7d042e80d452424da180b6635e89bc0349` |
| `examples/contracts/invalid/gui_shell_module_comparison_evidence_pruning_claim.invalid.json` | `cafa6b9ae9f5fc1f00ad885c222ef9d6916370f3` |
| `examples/contracts/gui_shell_windows_export_build.valid.json` | `31915727f6f0899bf4358996f709a23a76e9554d` |
| `examples/contracts/invalid/gui_shell_windows_export_build_authority.invalid.json` | `b29a306a0fe8e4e2061730ade3b69071fe41f80c` |
| `examples/contracts/gui_shell_linux_export_build.valid.json` | `fd710b0597b140d41f610085b7f3fcd6722b97bb` |
| `examples/contracts/invalid/gui_shell_linux_export_build_authority.invalid.json` | `bc3708f85596ae327ee9b9d50afc9f8e8c80f23c` |
| `examples/contracts/invalid/gui_shell_linux_export_build_wrong_target.invalid.json` | `1c6aa19d32236efba543a8f6685e73290ff41920` |
| `apps/desktop_flutter/lib/main.dart` | `6109c821ab5dc809224bec5234c4c59db1d5f988` |
| `apps/desktop_flutter/lib/screens/settings.dart` | `66374ec4157d573939e7f9cc5c36aecf5d65e251` |
| `apps/desktop_flutter/lib/services/export_client.dart` | `70d77edc6beb9e209e6b362219d1b22d69375489` |
| `apps/desktop_flutter/test/module_pruning_test.dart` | `20470e523c2640b681e54c51f9c6749ee35f5e05` |
| `native/rust_helper/src/broker/export_center.rs` | `477e44d86ebf18d2b004a7294c9cdb14e35aa0f0` |
| `tooling/build_module_pruned_windows.py` | `490aae7209ea5988f8e2cdcb05fe092b3d29504a` |
| `tooling/compare_module_builds_windows.py` | `09d926b9c6b611cd59e524cc947ce2dce1c81247` |
| `tooling/export_windows_product.py` | `31856179881a5a5aae1ba4451732e6de6cd7f338` |
| `tooling/export_linux_product.py` | `47fe9a3032a5b2680e3f7d39b7cd64120ac83843` |
| `tooling/linux_export_workflow.py` | `64af2da3282e1cc83deb8daa105939fab4858614` |
| `tooling/linux_manual_export_build.sh` | `29c05203cd513e4322d0a323778053a7ccfd87c5` |
| `tooling/tests/test_linux_portable_archive.py` | `c04ac3c5d6d58396f650e9530b6731c6bb2037d6` |
| `tooling/tests/test_linux_workflow_helpers.py` | `15c71a2ba5ac09ddab2c6ea23b36db256aeab85a` |
| `tooling/conformance_tests/run_conformance_skeleton.py` | `c6f31d8cb561b48513c6784e54c85969c9cb953e` |

### P11 E0-04項目の個別補完（2026-11-13、Windows製品化）

以下は旧P11 Windows Productizationを現行v4へ結ぶE0照合である。旧Product Buildの限定受入れ・statusは`docs/REV5_PRODUCT_PROGRESS.md`のP11節を保持した。v4仕様S04の直接ConsumerはN10、S05の能力対応はK16、工程表R03の施工線G/F、Phase 34とC33/C34の配布範囲が関係する。portable bootstrap上の機能試験と、一般利用者向けInstaller・同一候補のinstalled lifecycle・正式配布を区別する。照合HEADは`70673ad56c69d8a03d087cc8e182497f0a5262ce`。

| 照合項目 | 内容 |
|---|---|
| 現行ID | 旧P11。v4仕様S05の直接対応はN10、能力対応はK16。Phase 34はC33/C34と関連するD4製品配布工程であり、施工線G（Windows製品）とF（安全・回復境界）が関係する。現行Windows受入座標はW06/G6-WIN。 |
| 旧ID | P11 Windows Productization。rev5 Product Buildは`CLOSED`。閉鎖範囲はP9 portable ZIPをbootstrapとして利用するInstall／Update Center／Broker操作、first-run／Setup Doctor、Update、Rollback、固定root／payload欠損時の限定Repair、Uninstall、version表示の機能実装と限定試験。別個のMSI／Setup.exeや署名済み一般配布物を作成したとは主張しない。 |
| 原文参照 | `docs/REV5_PRODUCT_PROGRESS.md`のP11節、`docs/D4_POCKET_PHASE_MAPPING.md`のPhase 34およびC33/C34逆引き、v4仕様S04 N10・S05 P11/K16、工程表R03線G/F・W06、N10 OS別Consumer表。旧P11の試験範囲・限定statusは履歴として保持する。 |
| 対象 | Windows portable ZIP bootstrap→Update Center→Rust Brokerの製品Install／Update／Repair／Rollback／Uninstall、first-run／Setup Doctor、active version／Start Menu登録までの候補経路。一般向けInstaller、正式配布、Publisher identityは別条件として追跡する。 |
| 目的 | Windows利用者が導入・初回設定・更新・復旧・撤去できる製品経路を成立させ、file／process／署名検査／AuditをRust Broker・native helperへ置く。Flutterは表示・要求に限り、権限判定やOS作用を所有しない。 |
| 生成側 | 利用者がportable bootstrapまたは製品内SettingsのInstall／Update Centerから明示操作を行う。package builder／Update catalogは候補・package metadataを供給するが、それ自体は信頼・Permission・Approvalではない。 |
| 実際の受け取り先 | Flutter Settings／`update_client.dart`→Windows Runner／PID束縛IPC→既存Rust Broker `update_center.rs`／`product_install.rs`。Rust `product_bootstrapper.rs`／`product_package.rs`とWindows package toolingが候補を検査し、固定rootへのstage／version切替／rollback／uninstall作用を受け持つ。Setup Doctorは専用画面／collectorへ投影する。現行N10のsourceは接続候補を示すが、一般Installerからinstalled製品までの一貫した実consumerは未成立。 |
| 契約とデータ | Product Package Manifest／D4PKG01、Update Trust／Catalog／Candidate／Receipt／Download Job、Setup Doctor reportと各Schema／正常・負例JSONを照合した。package hash／署名・target・固定root・version・Audit IDを扱う。Credential実値、Approval、Permissionをpackage／UI metadataから継承しない。 |
| 正常時の作用 | 旧P11の試験記録はFlutter `settings_install_flow_test.dart`／`settings_compose_test.dart`／`update_client_test.dart`計17件PASS、Rustのdownload／native Owner確認・stage／version switch／portable activationに関するfocused test各1件PASSを含む。既存progress記録ではBrokerの固定product root、durable Audit、active version、Start Menu、Update／Rollback／限定Repair／Uninstallへのsource・fixture接続を確認した。これはinstalled正式製品の一連実作用ではない。 |
| 否定時の作用 | Update Trust未設定、不正なpackage／Catalog／署名・hash・target、path境界違反、危険policy、Setup DoctorのAuthority claim等をSchema・source・fixtureで拒否する設計を確認した。旧P11のPASS済みunit／Widget／Conformance範囲を保持し、すべての否定例がinstalled製品経路で実行されたとはしない。信頼metadataや画面状態から権限・署名を生成しない。 |
| 停止と回復 | 旧P11はstage／version切替／Rollback fixture、Download境界、固定root／payload限定Repairの局所条件を記録した。crash／電源断・別profile・完全Repair・正式Installer導入からの連続回復は未成立。Uninstaller finalizer自身の一時directory cleanupと失敗時の回復証拠は`FQ-P11-UNINSTALL-FINALIZER-CLEANUP`として後続Final QAへ保留し、旧P11を再開しない。 |
| 標準対象OSの候補 | P11の対象はWindows Desktop。現行N10に`windows-2025`のmanual build workflow候補とportable／D4PKG01 package sourceはあるが、workflow build fixtureはinstalled acceptanceではない。v4は標準Windows環境でのinstalled QAに加えてOwner所有Windows実機の確認結果を記録する。対象edition／build／arch／最低資源と一般配布package条件は未確定。 |
| 配布 | 旧P11の入口はP9 portable ZIPをbootstrapにするDeveloper／限定Product Build経路である。別個のMSI／Setup.exe、署名済みrelease artifact、Publisher identity、一般利用者向け正規配布は成立していない。P9 portable ZIP、D4PKG01 update payload、stage scriptを製品Installerと同一視しない。 |
| 既存実装 | Flutter Install／Update／Setup Doctor UI、Windows Runner／Broker IPC、Rust package／install／update、Windows Export／portable／product package tooling、Setup Doctor・evidence collector、manual build workflow、Schema／正常・負例fixtureはsource上`CODE_PRESENT`／`WIRED`。旧P11の限定Product Build acceptanceは`CLOSED`のまま。 |
| 実装不足 | 同一候補の一般利用者向けInstaller→first-run→native Owner／Broker→実Task→Update／Repair／Rollback／Uninstallを結ぶN10 installed lifecycle、別profile、公式配布package・署名／Publisher identity・trust provisioning、現候補provenance、Windows所有実機のQA evidenceは未成立。`windows_installer_first_run_smoke`等のRegistry条件とN10/W06をOPENのまま保持する。 |
| ソース識別値 | P11に関係する旧進捗・Phase／Final QA・package/Update/Setup Doctor契約と正常・負例、Flutter UI/test、Windows Runner、Rust Broker、Installer/tooling/workflow/Conformanceの56 fileを、照合HEAD `70673ad56c69d8a03d087cc8e182497f0a5262ce`のGit object識別値で下表に結合する。識別値はsource同一性で、test実行・installed acceptanceを意味しない。 |
| 試験区分 | 旧P11記録：Flutter 17件、Rust focused test 3件、Schema 161／正常例157／負例208、Conformance 236 checks、Manifest／通常Release Gate PASS。試験はunit／fixture／build evidenceの範囲であり、正式Installer、一般配布、別profile installed run、同一候補の全lifecycle、crash／電源断、Windows所有実機の完成品QAを証明しない。本mappingでは旧試験・製品Rust／Flutter・workflow・installed productを再実行せず`NOT_RUN`。 |
| 証拠ID | `NONE`。旧rev5 P11の限定CLOSED記録は保持する。v4 N10/W06向けinstalled product／QA Evidence IDは発行しない。 |
| 阻止要件 | 既存Registryの`windows_installer_first_run_smoke`、`windows_setup_doctor_smoke`、`windows_broker_installed_smoke`、`windows_evidence_provenance_isolation`、`windows_distribution_identity`、`rev2_desktop_product_distribution`を参照する。`FQ-P11-UNINSTALL-FINALIZER-CLEANUP`は後続Final QA項目として区別する。各status・分類を変更せず、新規blockerを追加しない。 |
| 状態 | 旧rev5 P11 Product Build機能受入れは`CLOSED`。E0-04 P11 source→consumer mappingは本単位で記録完了。v4 N10/W06/G6-WIN/E0は`OPEN`、Windows一般配布・installed lifecycle・正式署名は未成立。`release_ready=false`と既存Registry statusを維持する。 |
| 残件 | N10/G線で一つのcandidate identityに固定したWindows標準環境installed lifecycle、native Owner／Broker・Audit・Setup Doctor・実Task・Update／Repair／Rollback／Uninstall、provenance、Owner所有実機QAを接続する。package／署名／Publisherと一般配布資格は別のRelease条件として保持する。finalizer cleanupはFQ/Q5の後続Final QAで扱い、本P11 acceptanceへ混ぜない。 |

#### P11 ソース識別値・該当ファイル

下表は照合HEAD `70673ad56c69d8a03d087cc8e182497f0a5262ce`のGit object識別値である。Source、Schema、fixture、test、workflowの存在は本単位での実行やinstalled製品受入れを意味しない。

| ファイル | 対象ファイル識別値 |
|---|---|
| `docs/REV5_PRODUCT_PROGRESS.md` | `9c978021d3982690e9aaa87896436bad7125b433` |
| `docs/D4_POCKET_PHASE_MAPPING.md` | `064ea334f86b137d94fba2370d2a2fd732f6e972` |
| `docs/FINAL_QA_QUEUE.md` | `7b332cccab70bb68aa1ff4f8e23acc0e816d8282` |
| `docs/specs/windows-product-package.md` | `19c5c45dc8cbdbd4560b7b2cb8fe5d8e9f82a2c3` |
| `docs/QE_2026-11-09/進捗.md` | `dbfe0e52ff510d8248a90154681ab4c144e23ce2` |
| `release_blockers.registry.json` | `244adc04a01e7b406c4ebcdeb829f7077d4aa35f` |
| `specs/d4_pocket_product_package_manifest.schema.json` | `3151ba81a869b87e6c0fa8f8633a0e424fc43ae7` |
| `specs/update_trust.schema.json` | `5caa430cf621c09e6e3d96bd6c377758095b9491` |
| `specs/update_catalog.schema.json` | `faa9297d56029f326cfe0c0962a5c89f55c97470` |
| `specs/update_candidate.schema.json` | `1d803d8a2aaf22576346c77f4c7113ad834d213c` |
| `specs/update.schema.json` | `351a76c29f0279cc2cef92970f082ba7c2f7aaf1` |
| `specs/update_receipt.schema.json` | `86471c54dd55fa55bc1cdc7ebe32050d9f30ed36` |
| `specs/update_download_job.schema.json` | `42f457bf1fb1275c17f55a90f671cc91e8a51e2e` |
| `specs/setup_doctor_report.schema.json` | `9c083d42cdbf176989b105d8e3dfd4c44116a72f` |
| `examples/contracts/d4_pocket_product_package_manifest.valid.json` | `da9fc2eeec548deae980f9dc90898fe08f2d7629` |
| `examples/contracts/invalid/d4_pocket_product_package_manifest_parent_path.invalid.json` | `e5c3a55e87e7f25bb7ceebd485b4be2a9ba626d9` |
| `examples/contracts/update_trust.valid.json` | `17f01e0d53312d919888863c1afbaf46884b6557` |
| `examples/contracts/update_catalog.valid.json` | `8fe1df4c289e04322b331259b4a6b75f2b0d7e7b` |
| `examples/contracts/update_candidate.valid.json` | `09955e5e9ed97e9435ec6cc96f53a05b2d1a755b` |
| `examples/contracts/update.valid.json` | `04e8649af6e603266903f0818f1f907e9d3bf808` |
| `examples/contracts/update_receipt.valid.json` | `a6c54ca5bbc1bbacb3433bf31055a077d4a9a1da` |
| `examples/contracts/update_download_job.valid.json` | `f2b45d607352c84fa178fc41503063198ff3d8b4` |
| `examples/contracts/setup_doctor_report.valid.json` | `5eef23d2af04c44d41fc42bfa1187d01a59d7a9d` |
| `examples/contracts/invalid/update_trust.invalid.json` | `b5b9c73800f30e37663c079aca341b4e8cced2f1` |
| `examples/contracts/invalid/update_unsafe_policy.invalid.json` | `7a1ad2e938de711e9c315c090b66ff59c3a0c402` |
| `examples/contracts/invalid/update_download_job_path_leak.invalid.json` | `66e6ef09467e915988c0cc1ed88835f37fff480b` |
| `examples/contracts/invalid/setup_doctor_report_authority_escalation.invalid.json` | `b8d782b3f2b9e6af2b1f4eed9cc4c057aae10e3d` |
| `native/rust_helper/src/product_package.rs` | `a2deb2b6e77bc00ae8d01cd5ca306ee63baa20e5` |
| `native/rust_helper/src/product_bootstrapper.rs` | `3b0495dfc9a1937ba64f3f99c95ae4cc739ddc69` |
| `native/rust_helper/src/broker/product_install.rs` | `c91115bdf88dd28e5f3445e7ed66127902909677` |
| `native/rust_helper/src/broker/update_center.rs` | `f6fc98155fcd476765b60d382a680c7f7c6d675e` |
| `native/rust_helper/src/desktop_launcher.rs` | `6a9f335de07fa474ca10043aea63ad6e988b572d` |
| `native/rust_helper/src/broker/ipc_server.rs` | `58a404b920ef5905e174668ec2de0e04a9e8b044` |
| `native/rust_helper/src/main.rs` | `b0a5df285db357172e7970074d7a67189f3384a3` |
| `apps/desktop_flutter/windows/runner/main.cpp` | `be0084505135d0d33311915948fd660e59ea66ab` |
| `apps/desktop_flutter/windows/runner/broker_pipe_controller.cpp` | `e645d2d8cc25a1e61d9cb702013b5c3ee2e31eef` |
| `apps/desktop_flutter/windows/runner/broker_pipe_controller.h` | `f178545f83a1d2980b29c197f3d4d82ba1c4dbab` |
| `apps/desktop_flutter/windows/runner/CMakeLists.txt` | `84f310a9a3887b627ec88dbe8601753db48be30d` |
| `apps/desktop_flutter/lib/screens/settings.dart` | `66374ec4157d573939e7f9cc5c36aecf5d65e251` |
| `apps/desktop_flutter/lib/services/update_client.dart` | `26d2e946b5b15ea3328867e606e9f75bf4bfc2d1` |
| `apps/desktop_flutter/lib/screens/setup_doctor.dart` | `41fe7f3e904fcc2179ccc8c8373e53f0b646d1fe` |
| `apps/desktop_flutter/test/settings_install_flow_test.dart` | `73fc7b9706a7f4246972db46d5398d8b73decb09` |
| `apps/desktop_flutter/test/settings_compose_test.dart` | `a78353f93617cc7b17391549ea256f007a7acdb1` |
| `apps/desktop_flutter/test/update_client_test.dart` | `f70bb4a6dbc2340b5d26d867496952629dad1218` |
| `installer/windows/README.md` | `46287d463b38b1093a29e9818c0f0bfd99465393` |
| `installer/windows/stage_installed_app.ps1` | `c6d0797299624d69b71042416400b2f454787cc7` |
| `installer/windows/collect_setup_doctor.ps1` | `3695f3402dd175f8eafead3baa2c82bcc45e6364` |
| `installer/windows/collect_installed_smoke.ps1` | `92c4720e82eb0af91002659c742f8e03b15aa2de` |
| `installer/windows/collect_broker_smoke.ps1` | `f5f8abfad981d64c30a6cf1f4e9eafde1b43c357` |
| `installer/setup_doctor.py` | `23b43a8a00814e4033addec71efa11126d3b916a` |
| `tooling/export_windows_product.py` | `31856179881a5a5aae1ba4451732e6de6cd7f338` |
| `tooling/package_windows_portable.py` | `d6cfba6941da10fbae35228e7137d7f87f8d3889` |
| `tooling/package_windows_product.py` | `c6f9155f2457ef0d89ecb711fa94424fbb1501ee` |
| `tooling/windows_release_evidence.py` | `6f7957c6193c86c7bc15165e743f11651464f29c` |
| `.github/workflows/windows-manual-export-build.yml` | `f8255269ad70a63f329b6abb78f6da3d62cde960` |
| `tooling/conformance_tests/run_conformance_skeleton.py` | `c6f31d8cb561b48513c6784e54c85969c9cb953e` |

### P12 E0-04項目の個別補完（2026-11-13、製品全体統合）

以下は旧P12のWindows Product Build統合記録を現行v4のConsumerへ結ぶE0照合である。旧rev5のP12 `CLOSED（Windows 機能実装完了）`と`FEATURE COMPLETE`はその時点の機能統合範囲で保持し、正式Release、現候補のinstalled end-to-end、五OSの製品受入れへ昇格しない。v4仕様S05の直接対応はN01/N09/N10、関連能力はK19。Phase 39〜45／C27〜C34は全体QC・回帰・文書・出荷の後続対応であり、P12の旧Acceptanceや個別CLOSEDを置き換えない。照合HEADは`dfe4db6231838709022cf5517fe857dde2b8fcbc`。製品ソース比較基準`d503a4ef6daac8942abd1cb011442ce28c78ec55`からP12の代表Flutter／Windows tray sourceに差分がないことを確認した。詳細なTask／操作／OS lifecycle consumerとsource SHAはN01/N09/N10の正本節を参照する。

| 照合項目 | 内容 |
|---|---|
| 現行ID | 旧P12。v4仕様S05の直接対応はN01（実Task）、N09（製品操作・観測・復旧）、N10（五OS製品化）、関連能力はK19。製品操作を含む施工線C、Windows製品線G、証拠・最終品質の線Jと接点を持つ。Phase 39〜45／C27〜C34は後続QC・回帰・文書・出荷範囲として関連し、P12のAcceptanceへ合算しない。現行受入座標はN01/N09/N10、W06/G6-WIN、W07/G6-OS、N11。 |
| 旧ID | P12「製品全体統合」。rev5ではWindows側`FEATURE COMPLETE`、Product Build `CLOSED`。15段階の正常利用scenarioを製品経路へ接続した記録があり、接続単位1〜10は主に個別Widget/Fixture、単位11はWindows Debug Runnerのnative tray正常終了。15段階を一度のinstalled-product `LIVE_RUNTIME`で通した記録ではない。 |
| 原文参照 | `docs/REV5_PRODUCT_PROGRESS.md`のP12接続単位1〜11・受入判定、`docs/D4_POCKET_PHASE_MAPPING.md`のPhase 34および39〜45/C27〜C34、v4仕様S04 N01/N09/N10・S05 P12/K19、工程表R03の施工線C/G/J・R06〜R08、現行N01/N09/N10/N11節。旧P12のstatusと証拠classは当時の履歴として保持する。 |
| 対象 | Dashboard／first-runからAgent Center、Provider/Model設定、Runtime/Workspace登録、Agent Session/Task、Compare事前検査、Handoff、History/Diff導線、MCP Center/Tool request、Compose/Export、Update Center、Windows native tray終了へつなぐWindows製品UI経路。 |
| 目的 | 既存の製品機能を通常の利用導線へ接続し、画面・検索・コマンドから利用者が目的の機能へ到達できるようにする。Task、権限、OS操作は既存Broker/native責任境界を維持し、画面接続の完了を製品配布・Release・他OS完成と混同しない。 |
| 生成側 | Flutter DesktopのDashboard／first-run、Agent Center、Settings、全体検索・コマンドパレット、およびWindows Runnerのnative tray。TaskやMCP操作は操作者の明示要求から既存clientがBroker要求を生成する。 |
| 実際の受け取り先 | Flutter `ShellCoreClient.brokerTransport`から既存`AgentTaskClient`／Task coordinator、Compare/Handoff、History、MCP、Compose/Export、Update clientへ接続する。Rust Brokerと各operationの責務はN01/N09/N10の現行Source→Consumer表を参照。trayの終了要求はWindows `TrayController`／`FlutterWindow`へ渡り、対象Debug Runnerのprocess終了を観測した。Fake Broker経由のWidget応答を実Brokerの受入れとみなさない。 |
| 契約とデータ | 新しい権限契約は追加していない。既存Broker request/response、Task/Session/Workspace binding、result hash／bounded projection、MCP metadata/receipt、Compose Manifest、Update候補一覧、native tray終了を利用する。Permission／Approval／CredentialをUI stateやfixtureから生成・継承しない。契約とsource identityの詳細はN01/N09/N10節へ委譲する。 |
| 正常時の作用 | 旧P12単位1〜10では、first-run導線、Provider/ModelとRuntime/Workspace登録、Agent TaskのUI接続、Compare preflight、Handoff、History/Diff導線、MCP接続一覧、Compose/Export、Update一覧、MCP Tool receipt投影を個別scenarioで接続した。単位11では記録されたWindows Debug Runner artifactをtray「終了」から終了し、process exit code 0を観測した。Task/MCP等のFixture応答は実Provider／MCP副作用を示さない。 |
| 否定時の作用 | 過去Widget記録はBroker未接続時に登録・更新等を実行しない、起動時にMCP/Update作用を自動要求しない、明示操作前にTool callを送らない、permission/approvalを登録payloadへ含めない、Task結果画面と独立Workspace観測を同一視しない等の限定条件を確認した。P12全体を通す実Broker権限拒否・secret boundary・cross-OS negative matrixは本履歴で成立していない。既存安全条件と負例はN01/N09/N10/N11の現行gateに残す。 |
| 停止と回復 | P12単位11でDebug Runnerのnative tray終了がexit code 0となった記録がある。これはBroker finalization／終端Audit、installed製品の停止、crash・deadline・process群回復、Update/Rollback/Uninstall全体の連続動作を証明しない。これらはN01/N09/N10、FQ、Q2/Q5および既存blockerの責任範囲に残し、P12を再開しない。 |
| 標準対象OSの候補 | P12の機能統合記録はWindows Desktopに限定される。tray証拠はWindows Debug Runner、source commit `eaea63b52a5a4c35c44117b50b7979dbc2c57024`、生成時刻`2026-10-06T15:50:05Z`、artifact SHA-256 `68ad367d0d6053ea44e179d5dce9dc53b5ee5795889879178f0858aa658550a0`。他OSの一般利用・installed受入れはP12の範囲外であり、Linux nativeを含む五OS条件はN10/W07で継続する。 |
| 配布 | P12自体は一般配布Installerを生成・検証していない。前工程P11のportable ZIP bootstrap上の限定機能受入れと、P12のDebug Runner tray記録を区別する。MSI/Setup.exe、正式署名、Publisher identity、同一候補の通常Installer→first-run→実Task→更新／復旧／撤去の連続証拠は未成立。 |
| 既存実装 | Desktop Flutterの導線・各client接続とWindows tray sourceは現HEADに存在する。N01/N09/N10に記録したSource→Consumerはコード上の`WIRED`範囲を示す。P12の旧Product Build受入れは歴史status `CLOSED`のまま。UI sourceの存在・widget fixtureだけからinstalled製品動作を主張しない。 |
| 実装不足 | 同一Release candidateのinstalled Windows productで、一般導入からnative Owner/Broker、実Provider Task、結果・diff・History、MCP実作用、Update/Repair/Rollback/Uninstall、durable AuditとRecoveryまでを一体検証した証拠がない。macOS/Linux native/Android/iOSの標準配布・installed機能もP12では未受入れ。N01/N09/N10のcurrent BUILD_GAP／UNKNOWNは各正本節に残す。 |
| ソース識別値 | P12の代表Flutter導線・Widget integration test、Windows tray／window、旧P12履歴とFinal QA参照の12 fileを照合HEAD `dfe4db6231838709022cf5517fe857dde2b8fcbc`のGit blob識別値へ結合した。これは現物同一性であり実行証拠ではない。Broker／OS別の追加sourceはN01/N09/N10 SHA表を正本とする。 |
| 試験区分 | 旧P12単位1はfocused Widget test 2件、単位2〜10は各1件のfocused Widget testを記録し、主な証拠classは`FIXTURE`。単位11は別途`LIVE_RUNTIME`に分類されたWindows Debug Runner tray試験だが、範囲はnative tray／window lifecycleだけ。旧Schema／Conformance／Manifest／Gate結果は各当時の条件に限定する。本E0 mappingでは旧試験・製品Rust/Flutter・workflow・installed product・OS QAを再実行せず`NOT_RUN`。 |
| 証拠ID | `NONE`。旧P12の個別Fixture記録とDebug Runner tray hashは履歴参照であり、現行v4 N01/N09/N10/N11向けEvidence IDやcurrent Release candidate evidenceではない。 |
| 阻止要件 | 現行N01/N09/N10、W06/G6-WIN、W07/G6-OSをOPENのまま保持する。既存Registryの`windows_evidence_provenance_isolation`、`windows_installer_first_run_smoke`、`windows_setup_doctor_smoke`、`windows_broker_installed_smoke`、`windows_distribution_identity`、`rev2_desktop_product_distribution`、`rev2_flutter_broker_channel_boundary`、`rev2_export_owner_ui_authority_path`は現状`unresolved`として参照し、statusを変更しない。正式Q0〜Q7は`DEFERRED`、`release_ready=false`。新規blockerは作成しない。 |
| 状態 | 旧rev5 P12 Windows Product Build `CLOSED`／`FEATURE COMPLETE`を当時の機能統合・限定証拠の範囲で維持する。P12のE0-04 source→consumer mappingは本単位で記録完了。現行N01/N09/N10、W06/W07、N11およびE0は`OPEN`で、正式Release／QA、全OS installed acceptanceは未成立。 |
| 残件 | N01の実AI Task・結果／停止／Recovery、N09の同一Taskへ結合した操作・観測・Diff/Eval/Resource/Audit/Recovery、N10の五OS標準配布と同一候補installed lifecycle、N11の全要求・証拠・Artifact・Blocker統合を各現行IDの後続工程で閉じる。P12は再開せず、P13のE0-04 mappingへ進む。Linux nativeは必須製品対象のまま保持する。 |

#### P12 ソース識別値・該当ファイル

下表は照合HEAD `dfe4db6231838709022cf5517fe857dde2b8fcbc`上のGit blob識別値。製品sourceは比較基準`d503a4ef6daac8942abd1cb011442ce28c78ec55`から変更されていない。履歴文書の一部はE0後続更新を含む。N01/N09/N10/N11の他source・Contract・OS artifact hashは各Canonical SHA表を参照し、重複転記しない。

| ファイル | 対象ファイル識別値 |
|---|---|
| `docs/REV5_PRODUCT_PROGRESS.md` | `9c978021d3982690e9aaa87896436bad7125b433` |
| `docs/D4_POCKET_PHASE_MAPPING.md` | `064ea334f86b137d94fba2370d2a2fd732f6e972` |
| `docs/FINAL_QA_QUEUE.md` | `7b332cccab70bb68aa1ff4f8e23acc0e816d8282` |
| `apps/desktop_flutter/lib/main.dart` | `6109c821ab5dc809224bec5234c4c59db1d5f988` |
| `apps/desktop_flutter/lib/screens/dashboard.dart` | `3a1002c7eaa5c546f72865c71fc3226ac1ab920e` |
| `apps/desktop_flutter/lib/screens/agent_center.dart` | `2472faeb31a64d624423c30a0a30d0f153d2e2a3` |
| `apps/desktop_flutter/lib/screens/settings.dart` | `66374ec4157d573939e7f9cc5c36aecf5d65e251` |
| `apps/desktop_flutter/test/widget_test.dart` | `f4e9c7b93f832ba2a2cefee779fd12426f240b04` |
| `apps/desktop_flutter/windows/runner/tray_controller.cpp` | `2676afcc76e5c6e53fff2dd00d0fbb3ae531ccc1` |
| `apps/desktop_flutter/windows/runner/tray_controller.h` | `67ec337a335b948115559394ad5b55bffadb7502` |
| `apps/desktop_flutter/windows/runner/flutter_window.cpp` | `67849099e5141b696b237ec7390729793bc0e843` |
| `apps/desktop_flutter/windows/runner/flutter_window.h` | `bc0bb661a9ef7c2b639856a6d5e46b68b98ca4b0` |

P12の旧CLOSED status、test／artifact scope、v4のN01/N09/N10 OPEN状態は分離して保持する。この個別mapping完了はP12再受入れ、既存CLOSEDの再試験、N01/N09/N10/N11のclosure、E0全体完了を意味しない。

### P13 E0-04項目の個別補完（2026-11-13、非Windows／モバイル製品構築）

以下は旧P13の有限Product Build履歴を、現行v4のN10/K17と標準OS Consumerへ対応付けるE0照合である。旧P13の受入れ完了日はREV5記録の2026-10-09を維持する。これは現行五OSの通常配布・installed lifecycle・実利用QAの合格ではない。v4の直接対応はN10、関連能力はK17、工程表の施工線はH/I、受入座標はW07/G6-OS。Phase 35/36とC24〜C26は関連履歴として扱い、P13の旧statusを置き換えない。照合HEADは`10c0a969d917a0b8dc84a31b91f67a818d7d9899`。旧P13全体のevidence identity/classは採用正本・履歴索引から一意に特定できず`UNKNOWN`とする。個別のrun記録がある場合も、そのscopeを越えて集約しない。

| E0-04項目 | 記録値 |
|---|---|
| 現行ID | 旧P13。v4仕様S05の直接対応はN10、能力対応はK17。工程表R03のH（macOS/Linux）・I（Android/iOS）、R06のW07/G6-OSへ接続する。Linux nativeは必須製品対象。Phase 35/36、C24〜C26は関連範囲で、直接N mappingを変更しない。 |
| 旧ID | P13「`Mobile / Non-Windows Product Build`」。`docs/REV5_PRODUCT_PROGRESS.md`は2026-10-09に有限`Product Build`受入れを`CLOSED`と記録する。閉鎖対象の個別記録には`macOS Broker/Owner/Workspace/CLI/Host/A2A/MCP`、`iOS native Device Link`、`Android native Device Link`等がある。旧`CLOSED`の履歴statusは保持する。 |
| 原文参照 | `docs/REV5_PRODUCT_PROGRESS.md`の「P13モバイル／非Windows製品構築 — 受入れ完了」節（`macOS/iOS/Android`個別単位）、`docs/D4_POCKET_PHASE_MAPPING.md`の`Phase 35/36`・`C24〜C26`、v4仕様`S02/S04 N10/S05 P13・K17`、工程表`R03`線H/I・`R06 W07`・`R07`、現行N10表とOS別ソース識別値表。各参照は旧履歴と現行要求を分離して使う。 |
| 対象 | Linux/macOS Desktop、Android/iOS MobileにおけるFlutter UIとOS-native adapter/transport、Rust Broker接続、OS固有資格情報・session・配布候補、および一般使用者向け導入から実利用・更新・修復・撤去までの現行製品要求。旧P13の具体的に記録された個別作業範囲と、v4で要求する五OS lifecycleを別対象として扱う。 |
| 目的 | 非Windows各OSでD4 Pocketが通常の製品として導入・起動・設定され、Native IPC/OS保護を保ったままBroker統治下の仕事を利用できる構成へ接続する。旧P13の有限受入れ履歴を保持しつつ、現行N10が要求するOSごとの標準配布・installed product・仕事・更新／復旧／撤去の成立状況を独立に可視化する。 |
| 生成側 | `Linux/macOS Flutter Desktop Runner`、`Android Kotlin/iOS Swift native Device Link`、`Rust helper/Broker`、OS別`package/export builder`、手動`workflow`およびnative `UI/API`がソース・候補・要求を生成する。`metadata`、`workflow`、`package builder`自体を`Authority`や配布承認の源としない。 |
| 実際の受け取り先 | N10正本表の実Consumer。Linuxはexport/package tool→Linux product bundle/launcher→Rust Broker product install/update。macOSはFlutter Runner→`macos_desktop_worker`/`macos_desktop_owner`→Broker。AndroidはFlutter→Kotlin `MainActivity`/native Device Link→既存Rust Broker transport。iOSはFlutter→Swift `DeviceLinkNativeService`/Keychain→既存Rust Broker transport。現行N10表に示すOS別導入・利用・Update/Repair/Rollback/Uninstallの一貫Consumerは未成立。 |
| 契約とデータ | 現行OS別`package/build`設定、`Rust Broker IPC`、native `Device Link request`、`Keychain/Keystore`、OSの`permission/session`、`Task/Workspace/Audit`に接続する既存`contract`と、各OSの`package/workflow/test source`を参照する。`Flutter`へ秘密実値やOwner/Authority判断を渡さず、native `transport`からPermission/Approval/Trustを生成しない。N10の既存source/schema/test正本を消費し、新しいcontractは本mappingで追加しない。 |
| 正常時の作用 | REV5 P13節は有限Product Build単位の閉鎖を記録し、macOSのOwner/Broker/Workspace/CLI/Host/A2A/MCP各操作、iOSの製品UI/native service/Device LinkとSimulator局所検証、Android native Device Linkとbuild/test等を個別記録する。P13全体を同一Release candidate・五OS・installed lifecycleで一度に通した証拠とは扱わない。v4 N10の一般導入→初回設定→実仕事→更新/復旧/撤去は本mappingで未実行。 |
| 否定時の作用 | 個別の旧P13負例・拒否・取消はREV5各単位の正確なscopeに限る。旧P13全体のnegative TestClass/evidence identityは`UNKNOWN`で、横断したPermission/secret/Device Link/OS-install negative matrixは本照合資料から確認できない。現行contractはnative/Broker境界で拒否し、UI/Adapter metadataから権限を生成しない。現行OS installed経路での否定動作はN10 acceptance未実行。 |
| 停止と復旧 | 旧P13個別記録にある操作取消・正常終了・局所cleanupはそれぞれ記録された範囲に限る。旧P13全体のcrash/recovery/更新復旧保証は`UNKNOWN`または現行N10未成立として扱う。各OSの同一candidateにおける再起動、Broker/Native process回復、Update/Rollback/Repair/Uninstallの連続検証は未実行で、N10/W07後続工程に残す。 |
| 標準対象OSの候補 | Linux: Ubuntu 24.04 LTS/x86_64 portable `tar.gz`候補と24.04/amd64 bootstrap `.deb`候補。macOS: `macos-15` manual debug/UI、deployment target 10.15 source候補。Android: API 35 Google APIs x86_64 Emulator候補、compileSdk 37 source値。iOS: iPhone 16 Pro/iOS 18.5 Simulator候補、deployment target 13.0 source値。これらはN10で観測したcandidate値であり、v4の保証範囲・minimum・supported OS/ABIとして固定しない。Linux nativeは標準Linux GUI/sessionの必須対象であり、WSL/WSLg/Xvfb/headless単独では合格しない。 |
| 配布 | 旧P13のCLOSEDは一般利用者向けの署名済みinstaller/store配布の証拠ではない。Linux `tar.gz`はunsigned開発archive候補、`d4-pocket-setup` `.deb`はbootstrap UI用で製品本体installerではない。macOSは通常配布/署名/notarizationが未成立。Android workflowのdebug APK/AABとiOS Simulator artifactは通常配布・device artifactではない。通常配布形式、署名・Publisher/App identity、Store/Notarization資格は保証確定前で`UNKNOWN`。 |
| 既存実装 | N10 E0 source inventoryは共通package/bootstrapper 2件、Linux 10件、macOS 5件、macOS/iOS workflow 1件、Android 4件、iOS 3件を列挙する。UI/native adapter、package builder、manual workflow、focused test sourceの一部は`CODE_PRESENT`/`WIRED`。sourceの存在・接続と一般利用者向け配布・現候補のinstalled acceptanceを分離する。現行source SHAは下記25件で個別固定し、N10のWindows分を含む全5OS正本表を参照する。 |
| BUILD_GAP | 五OSそれぞれで通常配布→installed product→初回起動/設定→Broker統治された実仕事→Audit/拒否/停止→更新/Repair/Rollback/Uninstallを同一候補で結ぶ受入れは未成立。Linux nativeの通常GUI session、OS package、X11/Wayland範囲、標準distro・arch、macOS一般配布とidentity、Mobileのsupported API/ABI・署名配布・lifecycle、および最低resourceの保証値も未確定。N10は`OPEN`。 |
| ソース識別値 | 下表25件は照合HEAD `10c0a969d917a0b8dc84a31b91f67a818d7d9899`のGit blob SHA。N10の当初source表（照合HEAD `2c220a4bdb06fff26471393422b063c9735811cc`）にある該当blobと一致することを現HEADで確認した。これはsource同一性であり、test実行・製品動作・旧P13 evidence identityの証明ではない。 |
| 試験区分 | 旧P13全体を代表する集約classは`UNKNOWN`。REV5の個別記録はmacOS XCTest/UI・Rust/Flutter、Android build/instrumentation、iOS Simulator/XCTest等の個別範囲を記載するが、全P13証拠を同一candidateへ集約しない。現N10 source表のtest候補はLinux `UNIT`/hosted `LIVE_PRODUCT`、macOS/iOS `XCTest/UI`、Android instrumentation/Emulator。本mappingは文書・Git object照合のみで、製品test/workflowは`NOT_RUN`。 |
| 証拠ID | `NONE`（現行v4 N10/N11向け）。旧P13の有限CLOSED記録は履歴参照であり、個々のrun/artifactをP13全体の単一Evidence IDへ統合しない。現行N11へ取り込むOS/Artifact/SourceSHA/実結果の行単位結合は後続作業。 |
| 阻止要件 | Registryの既存`unresolved`を保持し、新規追加・status変更なし。P13/N10に直接関連する`linux_native_product_support`、`rev2_mobile_device_evidence`、`rev2_mobile_distribution`、`rev2_mobile_flutter_native_device_link_boundary`、`rev2_desktop_product_distribution`は未解決のまま。OS別N10のblocker詳細とRelease GateはRegistryを正本とし、本mappingを解消根拠にしない。 |
| 状態 | 旧REV5 P13有限Product Build acceptanceは`CLOSED`（2026-10-09）の履歴statusを維持。P13のv4 E0-04 field mappingは本単位で記録完了。現行N10、W07/G6-OS、N11、E0は`OPEN`。Linux native product supportと全OS installed QA、正式Releaseは未成立。`release_ready=false`を維持する。 |
| 残件 | 五OSの保証母集団・配布形式・OS/arch/resource値をE0-06で確定し、N10/W07で同一candidateの標準OS installed lifecycleを実装・検証する。Linux native GUI/package/permission/Update/撤去を必須完了する。macOS署名/notarization、Android/iOSの通常配布と署名資格を分離して記録する。N11にOS別source/artifact/evidence/blocker/statusを一意結合する。旧P13は再試験せず、現行N10作業を継続する。 |

#### P13 現行Source SHA・該当ファイル

下表はP13対象OSと共通package経路の25 source/test/workflowについて、現在HEAD `10c0a969d917a0b8dc84a31b91f67a818d7d9899`で照合したGit blob SHAである。Windows固有sourceはN10の全5OS表に残し、本P13表へ重複転記しない。OS別test/workflowは候補sourceであり、今回実行していない。

| OS/範囲 | ソースファイル | Git blob識別値 |
|---|---|---|
| 共通 | `native/rust_helper/src/product_package.rs` | `a2deb2b6e77bc00ae8d01cd5ca306ee63baa20e5` |
| 共通 | `native/rust_helper/src/product_bootstrapper.rs` | `3b0495dfc9a1937ba64f3f99c95ae4cc739ddc69` |
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
| `Android` | `apps/mobile_flutter/android/app/build.gradle.kts` | `fe96edfa0f714c2d2be509ce3bc21bec6253a714` |
| `Android` | `apps/mobile_flutter/android/app/src/main/kotlin/com/example/gui_shell_mobile/MainActivity.kt` | `5c0e28f0ad371f80671d26072bd1c1a134a881d9` |
| `Android` | `apps/mobile_flutter/android/app/src/androidTest/kotlin/com/example/gui_shell_mobile/NativeDeviceLinkProductFlowTest.kt` | `4f4a79e0bd80b1f5eb280857dec7a7d5df12258c` |
| `Android` | `.github/workflows/android-manual-emulator.yml` | `f0c1ecb7185c91375adfbbdbfaf49e55a2672b57` |
| `iOS` | `apps/mobile_flutter/ios/Runner.xcodeproj/project.pbxproj` | `73876a3b79b5ee5bd50dea0428764f1632dfcb19` |
| `iOS` | `apps/mobile_flutter/ios/Runner/DeviceLinkNativeService.swift` | `9d185729c439934dce0553b490f2fcd1f7b28a0d` |
| `iOS` | `apps/mobile_flutter/ios/RunnerUITests/DeviceLinkProductUITests.swift` | `b74c1f4a846c07eed9b6ae9244ac18b4f1030eec` |

このP13個別補完は旧P13の再受入れ、N10/W07/N11のCLOSED、E0完了、正式配布またはRelease承認を意味しない。旧P13・旧証拠のstatusと現行N10の未成立条件を分離して保持する。

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

C6 source hash欄は旧crosswalk記録時の照合HEAD `b2e20c2a16e2f324f3848ecc06203103755ceabf`上のGit blob SHA。履歴を保持し、以下の現行E0-04表では現在HEADを別途照合する。いずれもpath/consumerの同一性を記録するもので、実行・Acceptanceを意味しない。

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

旧仕様・工程・進捗のstatusはrev1当時の履歴として保持し、変更しない。この要約と旧SourceSHA表はmain f8a6b5ed521495fe7d0a4b5c79292fa3a7c169a0を基準に作成した履歴値であり、現行v4の製品受入れではない。現行N/K対応はv4 S04/S05およびR03の実consumerに限り、旧C statusを現行N受入れやPhase完了へ昇格しない。旧Phase対応はdocs/D4_POCKET_PHASE_MAPPING.mdの逆引きである。現候補Evidence IDのない受入れはUNKNOWN、列挙するtest sourceは本source-mapping単位でNOT_RUN。

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

下記blob SHAは当初のsource照合基準main f8a6b5ed521495fe7d0a4b5c79292fa3a7c169a0で採取した履歴値である。今回の現行HEAD照合では63 pathすべて同一blob SHAであることを確認した。production source内Rust unit testsを含み、別OSのinstalled/runtime証拠ではない。C7〜C11の主要SourceとC12〜C14のSource/Testを列挙する。既存N02/N05/N09節はそれぞれの広域consumer／追加SHAを保持する。

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

C7〜C14の旧Source/Schema/test blob SHA一覧は履歴として保持した。現行main 78a067eb4ddb6150b4ac0465569eb97ab36abffbとの63 path一致は今回の照合結果である。上記test sourceはUNIT／FIXTURE／WIDGET／CONFORMANCEのコード存在であり、この作業単位ではNOT_RUN。C7〜C14のinstalled／実OS acceptance Evidenceは新規発行していない。C7〜C14の旧statusはrev1履歴のまま、v4 N01/N02/N05/N09/N10、K07〜K10/K13/K16はそれぞれの現行ledgerでOPENを維持する。

### C7〜C14 E0-04全21項目の現行Source→Consumer照合（2026-11-13）

現行main HEAD 78a067eb4ddb6150b4ac0465569eb97ab36abffbで旧SourceSHA表63 pathを再照合し、63/63一致。旧表に含まれない更新実装の追加sourceは下段に個別記録する。以下は要求・実Consumer・責任境界のmappingであり、Cの履歴status変更、v4製品Acceptance、E3 BUILD_COVERED、OS別QAを意味しない。Source mapping作業の製品試験とEvidence発行はNOT_RUN／NONE。

#### C7 資格情報保管庫

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 旧C7 Credential Vault。秘密値のBroker保管、metadata投影、限定consumerへの受渡しと失効。 |
| 旧ID | 総合機能拡張rev1 C7／Phase 16。過去のOwner登録、DPAPI、metadata一覧、秘密非投影と限定fail-closedを履歴保持。 |
| 原文参照 | v4仕様S04 N01/N02/N05/N10、S05 K07/K08/K17、工程R03/R06、credential-vault.mdとOS別binding契約。 |
| 対象 | Ownerによる登録・metadata一覧・論理失効、承認済みCodex Provider TaskおよびMCP stdio childへの用途限定利用。 |
| 目的 | 秘密をUI／IPC／Audit／error／log／traceへ漏らさず、所持をPermission／Approval／Trustへ変換しない。 |
| 生成元（Producer） | Rust native Owner入力が登録要求をBrokerへ送り、BrokerがAuditとOS保管先へ記録。Provider TaskまたはMCP接続はCredential ID等を参照する。 |
| 実Consumer | Rust credential_vault／credential_storage、Windows／macOS／Linux native入力、固定openai_codex_cli経路、MCP Center→mcp_center／mcp_stdio→対象stdio child。通常画面はmetadata-only一覧を受け取る。 |
| 契約／Data | credential_registration／receipt／list／revocation schema。ID、用途、対象、種類、保管方式、状態、暗号文hash、Audit ID。秘密実値は保管先と短命Rust処理内に限定。 |
| 正常時作用 | Owner確認後にBrokerがAuditを確定しOS保管先へ登録。通常一覧はmetadataのみ。利用時はAudit／hash／用途／対象／失効を再確認し、対象CLIまたはstdio childへ限定する。 |
| 否定時作用 | 重複ID、未知field、Audit不在、store欠落・改変、対象／用途不一致、失効、未対応保管先を拒否。秘密値を公開面へ出さない。 |
| 停止／Recovery | 検証・Audit・store不成立時は利用を開始しない。論理失効後の再利用は禁止し、新IDで再登録。物理削除とRecoveryは未接続。Linux group外子孫・Broker crash後cleanupは保証しない。 |
| 標準OS候補 | 製品対象はWindows／macOS／Linux native／Android／iOS。現sourceの限定consumerはDesktop三OS中心で、Android Keystore・iOS Keychainを含む五OS保証は未成立。Linux nativeは必須対象。 |
| 配布物 | 独自配布物なし。N10のOS別通常package／導入経路が対象。source・fixtureはinstalled製品の証拠ではない。 |
| 既存実装 | Windows DPAPI、Linux Secret Service、macOS固有bindingの一部、metadata一覧、Owner確認、論理失効、Codex Provider／MCP限定注入のsourceを確認。 |
| BUILD_GAP | 五OS installedでの登録・再起動・利用・失効・復旧が一体受入れされていない。Android／iOS保管consumer、物理削除／Recovery、任意Runtime／A2Aへの一般注入は未接続。 |
| ソース識別値（`SourceSHA`） | 旧SourceSHA節のC7 source 11 pathを現HEAD 78a067eへ照合し全件一致。Schema、Rust vault/storage、三OS入力、Mac/Linux bridge testのhashは同節。 |
| 試験区分（TestClass） | Rust UNIT、Flutter bridge test、Schema／negative fixture sourceあり。本mappingでの再実行はNOT_RUN。五OS installed testではない。 |
| 証拠ID（Evidence ID） | NONE。過去の局所証拠は元OS・build・scope内の履歴として保持し、現行五OS Acceptanceへ転記しない。 |
| 阻害要因（Blocker） | Registry 18件（16 unresolved／2 resolved）の現状態を維持。C7への個別帰属が不明なものはUNKNOWN。新規登録・状態変更なし。 |
| 状態 | C7 E0-04 mapping記載済み。旧status不変。現行N01/N02/N05/N10およびE0製品AcceptanceはOPEN。 |
| 残余 | Android／iOS保管、五OS installed一体試験、Provider／MCPの失敗後cleanup境界、物理削除とRecovery、N11のArtifact／Evidence／Blocker接続。 |

#### C8 MCP契約

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 旧C8 MCP Server／Tool／Resource／Prompt等のmetadata contractと検査境界。 |
| 旧ID | 総合機能拡張rev1 C8／Phase 17。metadata-only contractと当時のfixture／conformance状態を履歴保持。 |
| 原文参照 | v4仕様S04 N02/N05/N10、S05 K08/K17、工程R03、mcp-contract.mdとMCP schema群。 |
| 対象 | 外部MCP由来metadata、Capability差分、Tool inputSchemaをBroker catalogへ取り込む契約と実行前検査。 |
| 目的 | description／metadata／Trust／Credential referenceをAuthorityにせず、Tool call前に現在のBroker状態と引数契約を検査する。 |
| 生成元（Producer） | MCP discovery応答からBrokerが未信頼metadataを正規化し、接続catalogとTool／Resource／Prompt情報を生成する。 |
| 実Consumer | Rust mcp_center／mcp_stdioが接続catalogとTool identity／inputSchemaを検査。Flutter MCP CenterはBroker projectionを表示し、Compose／Agent orchestrationは限定metadataを参照する。 |
| 契約／Data | mcp_contract／connection／tool_call／tool_call_receipt schemaとmcp-contract.md。Resource／Promptの全概念はschema上の存在だけで実処理済みとしない。 |
| 正常時作用 | 対応範囲内で検証したTool catalogを表示。実callは現在のServer／Tool ID・名前・schemaを送信直前に再照合し、別途native Owner確認・一回Permission・Auditを通す。 |
| 否定時作用 | 未知field、外部schema参照、unsupported dialect、上限超過、Authority／Credential混入、stale catalog、identity不一致、schema不適合を拒否する。 |
| 停止／Recovery | discovery／Schema失敗を成功扱いしない。metadataだけで実行・retry・Recoveryを起動せず、Tool作用はC9の明示操作と現行権限へ戻す。 |
| 標準OS候補 | Desktop Windows／macOS／Linux nativeにsource経路。Android／iOSにTool作用consumerなし。Linux nativeは必須でありsourceをinstalled acceptanceへ昇格しない。 |
| 配布物 | 独自配布物なし。MCP Serverは接続先、D4 Pocket packageはN10対象。 |
| 既存実装 | MCP contract schema、Broker catalog／Tool validator、Rust stdio adapter、Flutter connection UIのsourceあり。C8契約とC9接続・作用は別判定。 |
| BUILD_GAP | Resource／Promptの完全consumer、五OS installed conformance、Mobile Tool作用、現candidateの実MCP証拠は未成立。 |
| ソース識別値（`SourceSHA`） | 旧SourceSHA節のC8 source／schema／test pathを現HEAD 78a067eへ照合し全件一致。mcp-contract、mcp_center、mcp_stdio、protocol等のhashは同節。 |
| 試験区分（TestClass） | Rust UNIT、Schema／conformance、Flutter client／widget、fixture probe sourceあり。本mappingの試験はNOT_RUN。 |
| 証拠ID（Evidence ID） | NONE。Schema／fixture／conformance sourceを実MCP installed acceptanceとして扱わない。 |
| 阻害要因（Blocker） | Registry 18件の状態を維持。C8直接Blockerが明示できない場合はUNKNOWN。新規Blockerなし。 |
| 状態 | C8 E0-04 mapping記載済み。旧status不変。現行N02/N05/N10およびE0はOPEN。 |
| 残余 | Resource／Prompt実作用、五OS installed接続、Mobile Tool consumer、実Server・現candidate Acceptance evidence。 |

#### C9 MCP接続センター

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 旧C9 MCP Centerのdiscovery／接続／Tool実行とAgent結果利用。 |
| 旧ID | 総合機能拡張rev1 C9／Phase 18・19。catalog・receipt・拒否試験の旧statusを履歴保持。 |
| 原文参照 | v4仕様S04 N02/N05/N10、S05 K08/K17、工程R03、mcp-connection-center.md、credential-vault.md、N02／W02記録。 |
| 対象 | Flutter MCP CenterからBroker管理stdio Server、metadata一覧、明示Tool call、Task result bindingを経てAgentへ結果を渡す範囲。 |
| 目的 | Tool作用を既存BrokerのAuthority／Permission／Approval／Audit境界内で実行し、外部結果を未信頼入力として扱う。 |
| 生成元（Producer） | OperatorのServer登録・接続・一覧・Tool call要求、およびCodex Agent Taskが生成する未信頼Tool proposal。 |
| 実Consumer | Flutter MCP Center→通常Broker client→Rust mcp_center／mcp_stdio→stdio child。Task連携はagent_mcp_task_controllerとBroker結果contextから同Agentのfresh Permission／Approval付き後続Taskへsource接続する。 |
| 契約／Data | MCP connection／tool call／receipt、Credential ID参照、catalog hash、Tool ID／argument hash、Audit ID、Task／Session／Workspace binding付き結果context。 |
| 正常時作用 | 接続・catalog更新後、現在catalog／schema／credential binding・Owner確認・一回Permission・Auditを再検査して一度dispatch。許可されたN02経路は未信頼Tool結果を同Agentの新規Task contextへ渡す。 |
| 否定時作用 | timeout、malformed応答、stale catalog、失効Credential、未知Tool、Permission／Owner／Audit不成立、Task bindingずれを拒否または隔離し、自動再送しない。 |
| 停止／Recovery | 外部作用の成否不明を成功へ昇格せず、既存停止・隔離・Auditへ接続。後続Taskはfresh Permission／Approvalを要求する。OSごとのprocess containment差を維持する。 |
| 標準OS候補 | Desktop Windows／macOS／Linux nativeにsource。Android／iOSのTool作用はBUILD_GAP。Linux nativeは必須で、group外子孫／Broker crash cleanupを保証しない。 |
| 配布物 | MCP Center独自配布物なし。接続Serverと通常OS別D4 Pocket packageはN10対象。 |
| 既存実装 | Broker catalog／Tool execution、Credential ID限定注入、Flutter画面、Agent MCP coordinator、result-context bindingとtest／probe sourceを確認。 |
| BUILD_GAP | 実Codexが実MCPを呼び結果依存follow-upまで同一installed candidateで通るW02/G2はOPEN。五OS一体証拠、全OS cleanup保証、Mobile Tool作用も未成立。 |
| ソース識別値（`SourceSHA`） | 旧SourceSHA節のC9 UI／client／orchestration／Rust／schema／test pathを現HEAD 78a067eへ照合し全件一致。N02追加source traceも参照。 |
| 試験区分（TestClass） | Rust UNIT／FIXTURE、Flutter client／controller、real_broker_mcp_probe sourceあり。本mappingではNOT_RUN。sourceはinstalled LIVE_PRODUCT結果ではない。 |
| 証拠ID（Evidence ID） | NONE。過去の限定実接続・W02履歴は元candidateと条件内に保持する。 |
| 阻害要因（Blocker） | W02/G2はOPEN。Registry 18件（16 unresolved／2 resolved）を維持し、C9直接帰属不明はUNKNOWN。新規追加なし。 |
| 状態 | C9 E0-04 mapping記載済み。旧status不変。N02/N05/W02/N10/E0製品AcceptanceはOPEN。 |
| 残余 | W02 installed実Codex→Tool→follow-up、OS別installed経路、process cleanup限界、Mobile Tool consumer、N11証拠結合。 |

#### C10 運用プロファイル

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 旧C10 運用Profileの作成・複製・適用要求・削除・export・import。 |
| 旧ID | 総合機能拡張rev1 C10／Phase 20。Profile永続化とPermission非生成の過去状態を維持。 |
| 原文参照 | v4仕様S05 K09、工程R03/R06、operation-profile.mdとprofile schema群。直接のN対応を無理に割り当てない。 |
| 対象 | Desktop Settingsと認証済Broker IPCを通るProfile文書のCRUD／複製／適用意図／入出力。 |
| 目的 | Runtime運用要求を再利用しつつ、ProfileからPermission・Approval・Credential・Authorityを発行しない。 |
| 生成元（Producer） | OperatorがSettingsでProfileの要求設定を入力し、profile_clientがBroker要求を生成する。 |
| 実Consumer | profile_client→Rust profile_center→Broker永続profiles.json。SettingsはBroker receipt／list projectionを表示する。 |
| 契約／Data | profile／profile_list／profile_receipt schema。Runtime／Adapter ID、Capability要求、Content Exposure、network/resource limit、UI preference。 |
| 正常時作用 | Brokerが作成・複製・削除・export／import・一覧を処理。適用要求は現Profile hashを照合し、意図をAuditへ記録するだけ。 |
| 否定時作用 | 未知field、Authority／Credential情報、malformed state、重複ID、stale hash、上限超過を拒否。不正永続状態は起動fail-closed。 |
| 停止／Recovery | atomic writeと起動時再検査を使う。実Runtimeへの適用consumerや適用後rollbackは未接続で、apply-intentを実作用扱いしない。 |
| 標準OS候補 | Desktop Settings／Broker sourceあり。五OS製品対象を維持し、各OS installed Profile操作は本mappingで未確認。Linux native必須。 |
| 配布物 | 独自配布物なし。通常D4 PocketのN10 packageに含む候補で、独立配布・installed acceptanceは未確認。 |
| 既存実装 | Profile schema、Rust永続Broker処理、Flutter profile clientとSettings接続のsourceあり。仕様はC10の限定範囲を定義する。 |
| BUILD_GAP | ProfileからRuntime／Agent Taskへの実適用、専用file picker、適用変更のRecovery、現candidate installed受入れ未成立。 |
| ソース識別値（`SourceSHA`） | 旧SourceSHA節のC10 spec／schema／Broker／client／Settings／testの7 pathを現HEAD 78a067eへ照合し全件一致。 |
| 試験区分（TestClass） | Rust UNIT、Flutter client test、Schema／negative fixture sourceあり。本mappingでの実行はNOT_RUN。 |
| 証拠ID（Evidence ID） | NONE。過去の局所実装状態を五OS Product acceptanceへ転記しない。 |
| 阻害要因（Blocker） | Registry 18件の状態不変。C10直接帰属が不明なblockerはUNKNOWN。新規登録・状態変更なし。 |
| 状態 | C10 E0-04 mapping記載済み。旧status不変。K09関連とE0の製品AcceptanceはOPEN。 |
| 残余 | Profileの実consumer適用、OS別UI／保存境界、適用時Audit／RecoveryとN11結合。 |

#### C11 更新センター

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 旧C11 Update Centerの候補検証・取得・適用・延期・rollback・限定修復。 |
| 旧ID | 総合機能拡張rev1 C11／Phase 21。候補一覧・署名・UI等の歴史statusを変更しない。 |
| 原文参照 | v4仕様S04 N10、S05 K09/K16/K17、工程R03/R06、update-center.mdとwindows-desktop-launcher.md。 |
| 対象 | Broker-owned trust／fixed sourceからの署名候補取得、package download／stage、active version切替、rollback、限定Repair／Uninstall。 |
| 目的 | UpdateをFlutterの任意network／filesystem／process動作から分離し、trust・package identity・Owner確認・Auditで統治する。 |
| 生成元（Producer） | Operatorの明示Update操作、固定trust／source、署名候補metadata、Broker download job。 |
| 実Consumer | Flutter update_client→Rust update_center／update_download／update_verification。Windows product_install／product_bootstrapperとLauncherがfixed root／active recordを消費する。 |
| 契約／Data | update／candidate／list／trust schema、package SHA-256・byte length、channel・candidate hash、Owner confirmation、Audit ID、active version record。秘密署名鍵はbundleしない。 |
| 正常時作用 | Brokerがfixed HTTPS sourceから署名候補を検査し、別個native Owner確認後にdownload／stage／active record／Start Menu切替／直前版toggleを行う。process起動は固定Launcher契約。 |
| 否定時作用 | 不正署名／trust、source不一致、redirect／proxy／非global DNS、byte長・hash・inventory不一致、unknown path／identity、stale requestを拒否し既存有効版を保持。 |
| 停止／Recovery | Broker jobは再起動で消え、明示再要求時にpackage／stageを再検証する。途中stage不整合を自動適用しない。fixtureはinstalled crash／電源断Recoveryではない。 |
| 標準OS候補 | 製品対象五OS。詳細な更新・Launcher sourceはWindows中心。Linux native／macOS／Android／iOSの標準更新・rollback・uninstall lifecycleは未成立としてN10に残す。 |
| 配布物 | Windows bootstrap／Broker-managed package sourceあり。一般向けPublisher署名配布や五OS packageはN10別条件であり、組立sourceは製品受入れでない。 |
| 既存実装 | Rust署名検証、fixed HTTPS候補取得、download／stage／rollback、Windows active record／Start Menu／限定repair、Flutter UIとRust／TLS fixture test sourceあり。 |
| BUILD_GAP | 実配布元・本番trust provision、同一installed candidateの導入→更新→起動→rollback→uninstall、crash recovery、五OS lifecycle／Artifact結合は未成立。 |
| ソース識別値（`SourceSHA`） | 旧SourceSHA節のC11 docs／schema／Broker／client／test pathは現HEAD 78a067eで一致。追加SourceSHAは下段に記録する。 |
| 試験区分（TestClass） | Rust UNIT／FIXTURE、TLS fixture、native dialog／Flutter client test sourceあり。本mappingではNOT_RUN。 |
| 証拠ID（Evidence ID） | NONE。旧Product Build／fixtureを現行N10 installed lifecycle Acceptanceへ昇格しない。 |
| 阻害要因（Blocker） | Registry 18件（16 unresolved／2 resolved）とrelease_ready=falseを維持。OS別direct blockerの特定不能はUNKNOWN。新規追加なし。 |
| 状態 | C11 E0-04 mapping記載済み。旧status不変。N10/K09/K16/E0製品AcceptanceはOPEN。 |
| 残余 | 本番署名identity、五OS標準package／Lifecycle、installed update recovery、Artifact provenance、N11同候補結合。 |

#### C12 通知センター

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 旧C12 Notification CenterのAudit event由来summary・一覧・状態操作。 |
| 旧ID | 総合機能拡張rev1 C12／Phase 22。旧summary／hash-bound state／UI接続を履歴保持。 |
| 原文参照 | v4仕様S04 N09/N10、S05 K10/K13/K17、工程R03/R06、notification-center.mdとschema群。 |
| 対象 | Broker確定Auditから作る固定summary、永続notification state、Desktop一覧・既読／破棄／navigation。 |
| 目的 | Boundedな通知を返しAudit本文・秘密・通知経由の権限作用を公開しない。 |
| 生成元（Producer） | Brokerの固定operation mappingが確定Audit eventをsummaryへ射影。caller任意通知登録なし。 |
| 実Consumer | Rust notification_centerがAuditとnotifications.jsonを読み、通常認証Broker IPCへ返す。Flutter notification_client／notifications screenが表示し、openは既存画面へ遷移。 |
| 契約／Data | notification／notification_list／notification_action schema。通知ID・hash・summary・固定種別・read/discard state。Audit reason／payload／Credentialは含めない。 |
| 正常時作用 | 直近4096 Audit eventをscanし最大256件を返す。既読／破棄は現在hashに束縛しBrokerへ記録する。 |
| 否定時作用 | caller任意source、未知operation、malformed／duplicate state、hash不一致、stale notificationを拒否。通知からPermission／Approval／Authorityを生成しない。 |
| 停止／Recovery | 永続状態不正・Audit確定失敗はactionを成功扱いしない。navigationからsensitive actionを直実行しない。 |
| 標準OS候補 | Desktop UI／Broker sourceあり。Windows native toast host capability未接続。Mobile summaryは別範囲。五OS installed parity未確認。Linux native必須。 |
| 配布物 | 独自配布物なし。OS別通知機能・Desktop package統合はN10で判定する。 |
| 既存実装 | Rust固定Audit projection、bounded scan／return、hash-bound persistent state、Flutter通知screen／client／test sourceあり。 |
| BUILD_GAP | Windows native toast、全Runtime event source、Mobile parity、同一installed candidateでの全OS表示・Recovery証拠は未成立。 |
| ソース識別値（`SourceSHA`） | 旧SourceSHA節のC12 docs／schema／Broker／client／screen／test pathを現HEAD 78a067eへ照合し全件一致。 |
| 試験区分（TestClass） | Rust UNIT、Flutter client／navigation、Schema／negative fixture sourceあり。本mappingでの実行はNOT_RUN。 |
| 証拠ID（Evidence ID） | NONE。局所UI／Broker sourceをinstalled notification evidenceにしない。 |
| 阻害要因（Blocker） | Registry 18件の状態不変。C12直接帰属の不明分はUNKNOWN。新規Blockerなし。 |
| 状態 | C12 E0-04 mapping記載済み。旧status不変。N09/N10/E0製品AcceptanceはOPEN。 |
| 残余 | Windows toast host、OS別通知経路、全必要event producer、Task／Release evidenceとの同一candidate結合。 |

#### C13 内部観測センター

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 旧C13 Auditから独立したSpan／Trace／Metric観測面。 |
| 旧ID | 総合機能拡張rev1 C13／Phase 23。Broker内bounded観測・IPC・Desktop画面の旧状態を維持。 |
| 原文参照 | v4仕様S04 N09/N11、S05 K10/K13/K19、工程R03/R08、observability-center.mdとobservation schema群。 |
| 対象 | Broker Audit finalization処理由来の内部Span／Trace／Metric、認証済一覧IPCとDesktop表示。 |
| 目的 | 性能・挙動・運用状態をboundedに観測し、Audit責任、Permission、Approval、Authorityを置き換えない。 |
| 生成元（Producer） | Audit確定後にRust ObservationCenterが内部処理時間等を記録。callerのSpan登録なし。 |
| 実Consumer | Rust observation_centerが保持中データを集約し、Broker IPCのobservation_listをFlutter observation_client／observability_centerが表示する。 |
| 契約／Data | observation_list／span／trace／metric schema。内部Audit確定の範囲・関連Audit IDのみで、raw payload／reason／Credentialを投影しない。 |
| 正常時作用 | 最大1024 Spanをsession memoryに保持。応答は最大256 Span／Trace・16 Metric。測定不能値は0でなくunknown。 |
| 否定時作用 | 不正limit／TraceID／未知field、Audit append失敗を拒否。MetricからPermission／Approval／Capabilityを導かない。 |
| 停止／Recovery | Audit未確定の処理は観測済みと記録しない。上限超過では古い観測を落とす。再起動後の永続復元や長時間運転を主張しない。 |
| 標準OS候補 | Broker／Desktop sourceあり。実測対象はBroker内部Audit finalizationのみ。五OS installed resource／runtime観測は未成立。Linux native必須。 |
| 配布物 | 独自配布物なし。通常packageへ含む候補だが、製品resource／外部collector未接続。 |
| 既存実装 | Rust bounded ObservationCenter、通常Broker IPC、schema、Flutter Observability Centerとclient/test sourceあり。 |
| BUILD_GAP | Runtime／Adapter／Tool／外部通信span、OpenTelemetry export／collector、長期保持、OS resource観測、installed／long-run evidence未接続。 |
| ソース識別値（`SourceSHA`） | 旧SourceSHA節のC13 schema／Broker／client／screen／test pathを現HEAD 78a067eへ照合し全件一致。 |
| 試験区分（TestClass） | Rust UNIT、Flutter client／widget、Schema／conformance sourceあり。本mappingでの実行はNOT_RUN。 |
| 証拠ID（Evidence ID） | NONE。内部観測をLIVE_RUNTIMEまたは正式Audit証拠としない。 |
| 阻害要因（Blocker） | Registry 18件の状態不変。C13への直接Blocker対応が不明なものはUNKNOWN。 |
| 状態 | C13 E0-04 mapping記載済み。旧status不変。N09/N11/E0製品AcceptanceはOPEN。 |
| 残余 | 実Runtime／Adapter／Tool測定、unknown投影、保持・負荷評価、五OS installed接続とN11 evidence結合。 |

#### C14 追跡情報閲覧

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 旧C14 read-only Trace Inspector。 |
| 旧ID | 総合機能拡張rev1 C14／Phase 23。C13一覧再利用・Broker-only表示の旧statusを維持。 |
| 原文参照 | v4仕様S04 N09/N11、S05 K10/K13/K19、工程R03/R08、trace-inspector.md／observability-center.md。 |
| 対象 | DesktopがBroker observation listをTraceIDで絞り、Span時間軸・status・duration・parent・error分類・関連Audit IDを表示する範囲。 |
| 目的 | 内部観測を限定表示し、Audit証拠・Authority・実行状態を再構成したように見せない。 |
| 生成元（Producer） | C13 ObservationCenterがBroker内Audit finalizationからSpanを生成する。Trace Inspector自身は観測を生成しない。 |
| 実Consumer | Flutter Trace Inspector→observation_client→通常認証Broker IPC→Rust observation_centerのTraceID filter／bounded一覧。 |
| 契約／Data | observation_list／span／trace schema。TraceID、SpanID、parent、開始／終了、状態、時間、限定error分類、関連Audit ID、証拠種別。 |
| 正常時作用 | 読み取り専用でbounded waterfallを表示し、観測source範囲がBroker内であることと証拠種別を示す。 |
| 否定時作用 | 不正TraceID／limitをBrokerが拒否。Trace／parent／Audit IDからPermission／Approval／Authorityや未観測spanを補完しない。 |
| 停止／Recovery | 一覧を取得できない時は未取得と表示し、古いsnapshotをlive statusへ昇格しない。TraceからTask cancel／Recoveryを起動しない。 |
| 標準OS候補 | Desktop Flutter sourceあり。Mobile consumer未確認。五OS installed Trace parity未成立。Linux native必須。 |
| 配布物 | 独自配布物なし。Desktop製品UIに含む候補で、N10 package／installed証拠へ接続する。 |
| 既存実装 | TraceID filter、manual refresh、waterfall、duration／status／parent／error／Audit ID表示のFlutter sourceとBroker list sourceあり。 |
| BUILD_GAP | Runtime／Adapter／Tool／外部通信span、A2A実接続、長期trace、export／collector、Mobile／全OS installed evidence未成立。 |
| ソース識別値（`SourceSHA`） | 旧SourceSHA節のC14 docs／schema／Broker／client／screen／test pathを現HEAD 78a067eへ照合し全件一致。 |
| 試験区分（TestClass） | Rust UNIT、Flutter client／widget、Schema／conformance sourceあり。本mappingでの実行はNOT_RUN。 |
| 証拠ID（Evidence ID） | NONE。画面sourceや内部観測を実Runtime span／正式QA evidenceへしない。 |
| 阻害要因（Blocker） | Registry 18件の状態不変。C14直接帰属が不明な項目はUNKNOWN。新規登録なし。 |
| 状態 | C14 E0-04 mapping記載済み。旧status不変。N09/N11/E0製品AcceptanceはOPEN。 |
| 残余 | 実Runtime段階のSpan producer、同一Task／Trace結合、OS別installed表示、長期運用とN11証拠統合。 |

##### C11追加SourceSHA（現行main 78a067eb4ddb6150b4ac0465569eb97ab36abffb）

| 出典ファイル | `Git blob SHA` |
|---|---|
| native/rust_helper/src/update_verification.rs | 6347d90043cc7196feece6d708fafe96d27f3f19 |
| native/rust_helper/src/broker/product_install.rs | c91115bdf88dd28e5f3445e7ed66127902909677 |
| native/rust_helper/src/product_bootstrapper.rs | 3b0495dfc9a1937ba64f3f99c95ae4cc739ddc69 |
| docs/specs/windows-desktop-launcher.md | 609a0c724d5bb77ca15c7721e760267ac3c4d018 |

以上8 IDのE0-04個別mappingは文書上完了。旧C7〜C14 status、N02/N05/N09/N10/N11、W02/G2、Release Registryは変更しない。ここでのsource照合はProducer／Consumerの同定とGit blob一致のみで、runtime再試験・製品Acceptance・五OS QAではない。

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

### C15〜C23 E0-04全21項目の現行Source→Consumer照合（2026-11-13）

変更前main f66234f46a22366dc1f63ef46ea933e583e7884cで既存C15〜C23 source表63 pathを照合し、63/63が同一Git blob SHA。照合baseline 3c188ae54de6ea353490a60d91c1a75757d06e74以降、対象source pathの変更は0。以下は旧要求ごとの全field mappingであり、旧status、新N受入れ、E3 BUILD_COVERED、五OS QAを変更しない。本mappingでの製品Rust／Flutter／installed試験はNOT_RUN、Evidence IDはNONE。

#### C15 A2A外部概念Contract

| E0-04項目 | 個別値 |
|---|---|
| 現ID | C15 A2A外部Agent Card／Task／Message／Artifact／Streamの境界付き契約射影。 |
| 旧ID | 総合機能拡張rev1 C15／Phase 24。metadata-only Schema、Authority Strip、Trust非生成と当時の未接続範囲を履歴保持。 |
| 原文参照 | rev1実装仕様・工程・進捗C15／Phase 24、docs/D4_POCKET_PHASE_MAPPING.md、v4仕様S03 ARC-D・S04 N04・S05 K11、docs/specs/a2a-contract.md。 |
| 対象 | 外部A2A v1.0のCard、Task、Message、Artifact、Stream概念と認証要件を内部の未信頼metadataへ射影する契約。 |
| 目的 | 外部状態・能力・本文をTrust、Permission、Approval、Audit identity、Authorityへ変換せず、対応形状のみを限定的に受け取る。 |
| 生成元（Producer） | 外部Agent CardとA2A response／eventが未信頼入力を生成する。C15契約自体は外部作用を起動しない。 |
| 実Consumer | specs/a2a_contract.schema.json、Rust a2a.rsのparse／validation、Broker a2a_centerとC16接続処理。 |
| 契約／Data | Card／Task／Message／Artifact／Streamの限定field、hash、状態、part種別、securityRequirements／securitySchemesの有無。raw本文・秘密・Authority fieldは投影しない。 |
| 正常時作用 | 対応形状をbounded metadataへ正規化する。security要件が未設定または空の場合だけ、C16の限定一回Message経路へ候補を渡す。 |
| 否定時作用 | malformed／unknown構造、未知variant、外部Authority／Credential field、非空または不正なsecurity要件を拒否する。 |
| 停止／Recovery | parse／Schema／identity検査が不成立ならdispatchしない。外部作用後の応答不明はunknown／SUSPENDで止め、自動再送しない。 |
| 標準OS候補 | Schemaは共通。Desktop作用sourceはWindows／Linux。macOSはCard／metadata-only、Message等は未対応。Mobile consumer未確認。Linux nativeは必須だがinstalled証拠なし。 |
| 配布物 | 独立配布物なし。D4 Pocket Desktop本体の契約。外部A2A Agentは接続先。 |
| 既存実装 | Contract文書・Schema、Rust parser／Broker consumer、Authority境界と否定条件、A2A接続fixture sourceあり。外部相互運用・Trust lifecycleは未成立。 |
| BUILD_GAP | 外部Agentとのinstalled相互運用、外部Agent identity／Trust lifecycle、Mobile consumer、全OSの実consumerは未成立。 |
| ソース識別値（SourceSHA） | C15〜C23 SourceSHA表のa2a-contract.md、a2a_contract.schema.json、a2a.rs、a2a_connection_center_test.dart等を現HEADへ照合し63/63一致。 |
| 試験区分（TestClass） | Schema／Rust UNIT／FIXTURE sourceあり。本mappingではNOT_RUN。外部Agent installed試験ではない。 |
| 証拠ID（Evidence ID） | NONE。Contract／Schema／fixtureを外部Agent実動Acceptanceへ昇格しない。 |
| 阻害要因（Blocker） | Registry 18件は不変。comprehensive_extension_rev1_completionはunresolved。C15個別の正式Registry対応はUNKNOWN。 |
| 状態 | C15 E0-04 mapping記載済み。旧status不変。N04／W03／E0製品AcceptanceはOPEN。 |
| 残余 | 外部AgentでのContract相互運用、Trust評価との境界、標準installed consumer、N11 Evidence／Artifact結合。 |

#### C16 A2A接続センター

| E0-04項目 | 個別値 |
|---|---|
| 現ID | C16 外部A2A Agentの接続・一回作用・限定Task／Artifact projectionを行うDesktop Center。 |
| 旧ID | 総合機能拡張rev1 C16／Phase 25。Owner Card取得、loopback HTTP、receipt／一覧と当時の未完範囲を履歴保持。 |
| 原文参照 | rev1実装仕様・工程・進捗C16／Phase 25、docs/D4_POCKET_PHASE_MAPPING.md、v4仕様S04 N04・S05 K02/K11、a2a-connection-center.md、a2a-streaming.md、macos-a2a-center.md。 |
| 対象 | Flutter CenterからBrokerへ接続し、Windows／Linux Desktopで一回SendMessage、明示一回GetTask／Stream、選択済みtext Artifact表示とTask preflightを行う範囲。 |
| 目的 | 外部作用をendpoint／Agent／interfaceへ束縛したnative確認・一回Permission・Auditへ通し、応答をAuthorityにせず失敗をunknownで止める。 |
| 生成元（Producer） | Operatorの明示接続／送信／状態照会／Artifact選択要求と、接続先AgentのCard／Message／Task／Stream応答。 |
| 実Consumer | Flutter a2a_connection_center／A2aConnectionClient→通常Broker a2a_center→Rust a2a.rs HTTP(S)。ArtifactはFlutter handoff preflight／Agent Task要求へ接続する。 |
| 契約／Data | A2A connection／receipt／message／stream／task schemas、Agent ID、interface／request／response hash、Audit ID、限定Artifact ID／hash／text parts。 |
| 正常時作用 | native Owner確認・永続Audit・一回Permission後に限定要求を一度送る。Artifactは内容表示確認後に未信頼Task draftへ渡す。受信Task開始には新規Permission／Approvalが必要。 |
| 否定時作用 | endpoint／interface stale、malformed応答、未知event、Credential必要、Task／Context不一致、Artifact hash／size／part不適合、Owner／Permission／Audit不成立を拒否する。 |
| 停止／Recovery | timeout、切断、Audit不確定、Broker再起動はunknown／SUSPEND。retry／reconnect／pollingなし。本文・draftは揮発保持し失敗／切替で消去する。 |
| 標準OS候補 | Windows／Linux Desktopに限定作用source。macOSはmetadata-only、送信未対応。Android／iOS作用未確認。native installed QA未成立。 |
| 配布物 | 独立配布物なし。D4 Pocket Desktopへ統合。A2A Agentは外部service。 |
| 既存実装 | Flutter画面／接続部品、Rust Broker／通信経路、Schema、選択した文章Artifactの引継ぎ、macOSのメタデータ限定境界と試験ひな型あり。外部Agentを導入した一連の実証なし。 |
| BUILD_GAP | installed候補で外部接続→Agent作用→Artifact→受信Task→Audit／Recoveryの一気通貫、remote Host、macOS作用、Mobile consumerは未成立。 |
| ソース識別値（SourceSHA） | SourceSHA表のA2A connection／stream／macOS docs、schemas、a2a_center、client、handoff、関連testを現HEADへ照合し63/63一致。 |
| 試験区分（TestClass） | Rust UNIT／FIXTURE、Flutter fake Broker／Widget／handoff fixture sourceあり。本mappingではNOT_RUN。 |
| 証拠ID（Evidence ID） | NONE。過去の限定loopback／fixtureを現候補の外部Agent Acceptanceにしない。 |
| 阻害要因（Blocker） | comprehensive_extension_rev1_completion等の既存unresolved状態を保持。C16個別の正式Registry linkはUNKNOWN。 |
| 状態 | C16 E0-04 mapping記載済み。旧status不変。N04／N10／W03／E0はOPEN。 |
| 残余 | 現候補の外部Agent chain、結果依存後続AI実仕事、OS別native確認／配布／Recovery証拠。 |

#### C17 複数Host registry

| E0-04項目 | 個別値 |
|---|---|
| 現ID | C17 Host登録・metadata registry・bounded persistence。 |
| 旧ID | 総合機能拡張rev1 C17／Phase 2・26。Owner登録、通常一覧、hash-only identity、再起動時fail-closedの履歴。 |
| 原文参照 | rev1実装仕様・工程・進捗C17、docs/D4_POCKET_PHASE_MAPPING.md、v4仕様S03 ARC-D・S04 N04・S05 K11、host-registry.md。 |
| 対象 | Ownerが登録したHost metadataをBrokerで限定永続化し、認証IPC経由で一覧表示する範囲。 |
| 目的 | Host登録を接続・Trust・Permission・Approval・Credential・Task権限と混同せず、別HostのAuthorityを分離する。 |
| 生成元（Producer） | Owner確認済みHost登録要求。遠隔Hostの稼働時探索／接続結果は対象外。 |
| 実Consumer | Flutter host_registration_client→Rust host_center→bounded hosts.json／Audit。Host listは通常認証IPCで投影する。 |
| 契約／Data | host registration／receipt／list schemas、Host ID、表示metadata、identity hash、状態、bounded登録記録、Audit reference。秘密・Task authorityなし。 |
| 正常時作用 | Owner確認後に登録metadataをAudit付きで保存し一覧を返す。pending_review等はmetadata状態であり、接続済み・Trust verifiedではない。 |
| 否定時作用 | malformed／unknown field、ID／hash不整合、上限外・破損stateを拒否し、登録metadataから外部identity保証を推論しない。 |
| 停止／Recovery | 保存／Audit失敗は成功receiptにしない。破損stateやunknown fieldはBroker起動時fail-closed。失効／quarantine／再接続は未接続。 |
| 標準OS候補 | Desktop Windows／macOS／Linuxに登録・一覧source。Mobile consumer未確認。Linux native installed acceptanceなし。 |
| 配布物 | 独立配布物なし。D4 Pocket DesktopのBroker state。 |
| 既存実装 | Host registry contract／Schema、Rust host_center、Flutter client、一覧／永続化／Auditとfixture sourceあり。 |
| BUILD_GAP | remote discovery／connection、失効／quarantine、Host配下Runtime／Agent route、越境Task実installed拒否は未成立。 |
| ソース識別値（SourceSHA） | SourceSHA表のhost-registry.md、host registration/list schemas、host_center.rs、client、host_registration_test.dartを照合し63/63一致。 |
| 試験区分（TestClass） | Rust UNIT／Flutter client fixture／Schema sourceあり。本mappingではNOT_RUN。 |
| 証拠ID（Evidence ID） | NONE。metadata registry／fixtureをlive Host接続証拠にしない。 |
| 阻害要因（Blocker） | Host registry契約はlive discovery／接続等を出荷阻害条件として記載。Registry 18件の状態は不変で、C17個別のRegistry識別子対応はUNKNOWN。 |
| 状態 | C17 E0-04 mapping記載済み。旧status不変。N04／W03／E0はOPEN。 |
| 残余 | remote Host接続／Trust lifecycle、越境拒否の実結果、Host別Task／Audit／Recovery installed evidence。 |

#### C18 Host操作面

| E0-04項目 | 個別値 |
|---|---|
| 現ID | C18 Host一覧・状態表示・切替context・Capability projection。 |
| 旧ID | 総合機能拡張rev1 C18／Phase 26・28。Host表示・切替と旧境界を履歴保持。 |
| 原文参照 | rev1実装仕様・工程・進捗C18、docs/D4_POCKET_PHASE_MAPPING.md、v4仕様S04 N04/N09・S05 K11/K13、host-operation-surface.md。 |
| 対象 | Flutter HostOperationCenter／HostCapabilityCenterがBroker Host list／switch／Capability metadataを表示する操作面。 |
| 目的 | Host選択を表示contextに限定し、非current HostのCapabilityをlive能力／Trustへ見せかけない。 |
| 生成元（Producer） | OperatorのHost切替要求、registry metadata、現在Brokerの限定Capability projection。 |
| 実Consumer | Flutter host_operation_center／host_capability_center→registration client→Broker IPCのHost list／switch／capability consumer。 |
| 契約／Data | host_switch／receipt／host_capability schemas、Host ID、current Host、表示context、Capability metadataと観測範囲。 |
| 正常時作用 | 登録Host metadataを一覧・切替表示する。product初期化で受理したBroker snapshot sourceとHost IDが一致する場合だけ現在BrokerのRuntime／Agent観測を表示し、それ以外は未観測とする。 |
| 否定時作用 | unknown Host、不一致receipt、stale登録、Owner拒否を接続成功へ昇格せず、metadata-only値をverified Trustとしない。 |
| 停止／Recovery | Broker取得失敗・不一致時は成功表示しない。stale projectionをlive stateとして使わず、Host切替からTask停止／Recoveryを起動しない。 |
| 標準OS候補 | Desktop Flutter／Broker source。一部macOS／Linux metadata操作あり。Mobile consumer未確認。Linux installed UX未検証。 |
| 配布物 | 独立配布物なし。Desktop製品のHost操作面。 |
| 既存実装 | Host操作／Capability画面、switch schemas、Broker consumer、registration clientとfixture sourceあり。remote Runtime／Agent列挙なし。 |
| BUILD_GAP | remote Host再接続、配下Runtime／Agent列挙、Host越境Task route／Authority拒否の実作用、全OS installed surfaceは未成立。 |
| ソース識別値（SourceSHA） | SourceSHA表のhost-operation／macOS docs、switch／capability schemas、host_capability.rs、screens、client／testを照合し63/63一致。 |
| 試験区分（TestClass） | Rust／Flutter client／widget fixture sourceあり。本mappingではNOT_RUN。 |
| 証拠ID（Evidence ID） | NONE。UI／fixtureをremote Host操作・Trust evidenceにしない。 |
| 阻害要因（Blocker） | Host操作契約はlive再接続／Trust検証／remote Runtime列挙を出荷阻害条件として記載。Registry状態は不変で、C18個別の識別子対応はUNKNOWN。 |
| 状態 | C18 E0-04 mapping記載済み。旧status不変。N04／N09／N10／E0はOPEN。 |
| 残余 | remote Host／Runtime実状態consumer、越境実作用境界、OS別installed UI／Recovery evidence。 |

#### C19 Adapter管理

| E0-04項目 | 個別値 |
|---|---|
| 現ID | C19 Adapter catalog管理・署名状態確認・有効化／隔離のmetadata操作面。 |
| 旧ID | 総合機能拡張rev1 C19／Phase 27。Catalog、Owner Broker操作、署名検査／隔離、Desktop Runtime Centerの履歴。 |
| 原文参照 | rev1実装仕様・工程・進捗C19、docs/D4_POCKET_PHASE_MAPPING.md、v4仕様S03 ARC-C/ARC-G・S04 N03/N10・S05 K06/K12、adapter-management-surface.md、adapter-conformance.md。 |
| 対象 | Flutter Runtime CenterからBrokerへAdapter manifest／catalogの限定操作を送り、Brokerがcatalog recordとAuditを管理する範囲。 |
| 目的 | Adapter宣言・署名metadata・有効状態をTrust／Permission／Approval／Credentialへ変換せず、Adapter適合境界を保つ。 |
| 生成元（Producer） | Operatorの明示管理要求と、Brokerに登録されたmanifest／catalog metadata。 |
| 実Consumer | Runtime Center／Broker client→Rust adapter_center→bounded Adapter catalog／Audit。外部artifact installer／process launcherではない。 |
| 契約／Data | Adapter管理の契約定義・一覧・要求・受領票、識別子、版、照合値、発行者／署名情報、危険度／互換性、状態、監査ID。 |
| 正常時作用 | Owner controlで宣言書登録／検証・有効／無効化・隔離等のcatalog metadataを管理しAuditへ記録する。外部成果物取得・filesystem書込／削除・Adapter起動は行わない。 |
| 否定時作用 | malformed manifest、未知field、signature／hash不一致、unsupported／非適合Adapterを有効化せず、metadataから実能力／Trustを推論しない。 |
| 停止／Recovery | Broker／Audit不成立時は成功receiptにしない。隔離Runtime IDをTask／lifecycleへ再利用しない。artifact展開後Recoveryは未接続。 |
| 標準OS候補 | Desktop Rust／Flutter source。全OSのinstalled artifact lifecycleは未成立。Mobile consumer未確認。 |
| 配布物 | 独立配布物なし。Adapter artifactは別配布物になり得るが、本管理面は導入／削除installerではない。 |
| 既存実装 | Adapter contract／Schema、Rust adapter_center、Runtime Center／client、signature／catalog metadataとWidget test sourceあり。 |
| BUILD_GAP | artifact取得・署名鎖・隔離展開・install／update／rollback／uninstall、Adapter process起動、全OS lifecycleは未成立。 |
| ソース識別値（SourceSHA） | SourceSHA表のadapter-management／conformance docs、adapter schemas、adapter_center.rs、runtime_center.dart、widget_test.dartを照合し63/63一致。 |
| 試験区分（TestClass） | Rust UNIT／Flutter widget／Schema fixture sourceあり。本mappingではNOT_RUN。 |
| 証拠ID（Evidence ID） | NONE。catalog／signature fixtureを実artifact導入／Trust証拠にしない。 |
| 阻害要因（Blocker） | Adapter管理契約は外部成果物の実導入／削除／process起動とWindows導入済み実証を出荷阻害条件として記載。Registry状態は不変で、C19とのID対応はUNKNOWN。 |
| 状態 | C19 E0-04 mapping記載済み。旧status不変。N03／N10／W03／E0はOPEN。 |
| 残余 | Adapter artifact取得／install／隔離実体／process起動・停止、OS別更新／撤去、候補provenance。 |

#### C20 Windows常駐トレイ

| E0-04項目 | 個別値 |
|---|---|
| 現ID | C20 Windows system trayの限定表示・操作入口。 |
| 旧ID | 総合機能拡張rev1 C20／Phase 28。表示、前面化、status／件数projection、stop receipt、exit入口の履歴。 |
| 原文参照 | rev1実装仕様・工程・進捗C20、docs/D4_POCKET_PHASE_MAPPING.md、v4仕様S04 N09/N10・S05 K13/K16、windows-tray-surface.md。 |
| 対象 | Windows native tray controllerからFlutter／Broker由来statusを限定表示し、停止要求receiptまたはDesktop UI終了へ接続する範囲。 |
| 目的 | Trayを表示・要求入口に限定し、Runtime Authority、Permission、Approval、Runtime停止完了の主体にしない。 |
| 生成元（Producer） | Windows tray event、Desktop status projection、Operatorのstop request／exit操作。 |
| 実Consumer | Windows tray_controller.cpp／.h→Flutter windows_tray_client→対象surface／Broker projection。ExitはDesktop UIへ渡る。 |
| 契約／Data | windows_tray_projection、tray_stop_request／response schemas。status、bounded counts、request receipt、executed=false等。 |
| 正常時作用 | Trayから画面表示・前面化・状態確認を行い、stop操作は実行待ちreceiptを返す。UI exitはDesktop appを閉じる。 |
| 否定時作用 | unavailable／unknown値を0にせず、receiptをRuntime停止済み・Permission・Approvalと扱わない。任意process操作なし。 |
| 停止／Recovery | Broker／Flutter応答なし・拒否時はunknown／未実行。Tray exit後のRuntime停止やinstalled起動復旧は保証しない。 |
| 標準OS候補 | Windows native trayのみ。この契約はmacOS／Linux／Mobileへ拡張しない。Linux Desktop全体の可否へ一般化しない。 |
| 配布物 | Windows D4 Pocket Desktop packageに含む候補。独立配布物なし。installed lifecycle evidenceなし。 |
| 既存実装 | Native controller、Flutter client、projection／stop schemasとclient test sourceあり。Runtime停止実体は未接続。 |
| BUILD_GAP | Owner確認後のRuntime stop、process lifecycle結果receipt、Windows installed tray／Recovery evidenceは未成立。 |
| ソース識別値（SourceSHA） | SourceSHA表のtray docs／schemas、windows_tray_client.dart、tray_controller.h／cpp、client testを照合し63/63一致。 |
| 試験区分（TestClass） | Windows native／Flutter client test sourceあり。本mappingではNOT_RUN。installed product試験ではない。 |
| 証拠ID（Evidence ID） | NONE。tray／receipt fixtureをRuntime stopやinstalled Acceptanceとしない。 |
| 阻害要因（Blocker） | Tray契約はWindows導入済みtray evidenceと実Runtime停止統合を出荷阻害条件として記載。Registry状態は不変で、C20とのID対応はUNKNOWN。 |
| 状態 | C20 E0-04 mapping記載済み。旧status不変。N09／N10／E0はOPEN。 |
| 残余 | Broker owner stopとprocess lifecycle、停止Audit／Recovery、現候補Windows installed evidence。 |

#### C21 コマンドパレット

| E0-04項目 | 個別値 |
|---|---|
| 現ID | C21 Command Paletteによるbounded command検索と画面遷移。 |
| 旧ID | 総合機能拡張rev1 C21／Phase 28。既存Palette拡張と全機能実行／installed evidence不足を履歴保持。 |
| 原文参照 | rev1実装仕様・工程・進捗C21、docs/D4_POCKET_PHASE_MAPPING.md、v4仕様S04 N09・S05 K13、command-palette-surface.md。 |
| 対象 | Desktop main.dartのCtrl+K／Ctrl+P Paletteから固定候補を検索し既存screenへ遷移する範囲。 |
| 目的 | Keyboard-first navigationを提供し、Palette選択を実作用・Owner許可・Authorityにしない。 |
| 生成元（Producer） | Operatorのshortcut／query／選択。固定command catalogはUI navigation entry。 |
| 実Consumer | Flutter main.dartが最大30件を表示し、選択後に既存navigation／screenを開く。候補名は処理APIではない。 |
| 契約／Data | command-palette-surface.mdとFlutter内command entry／query state。Broker permission／外部command契約なし。 |
| 正常時作用 | 候補を表示して画面遷移する。MCP接続・Host切替・評価・停止要求も入口画面を開くのみ。 |
| 否定時作用 | unknown command／queryは実行せず、Permission／Approval／Credential／filesystem／process／network authorityを作らない。 |
| 停止／Recovery | Palette終了・候補なし・遷移失敗を処理済み／Runtime作用済みと表示しない。 |
| 標準OS候補 | Desktop Flutter候補。Windows／macOS／Linux installed keyboard／navigation未検証。Mobile navigationは別範囲。 |
| 配布物 | 独立配布物なし。D4 Pocket Desktop UIの一部。 |
| 既存実装 | Palette surface文書、main.dart shortcut／bounded候補、module_pruning_test sourceあり。実処理へのdirect linkなし。 |
| BUILD_GAP | 実Broker／Runtime operationへのcommand接続、各Desktop OS installed keyboard／accessibility evidenceは未成立。 |
| ソース識別値（SourceSHA） | SourceSHA表のcommand-palette-surface.md、main.dart、module_pruning_test.dartを照合し63/63一致。 |
| 試験区分（TestClass） | Flutter widget／navigation sourceあり。本mappingではNOT_RUN。 |
| 証拠ID（Evidence ID） | NONE。画面遷移を実操作／installed keyboard acceptanceにしない。 |
| 阻害要因（Blocker） | Palette契約は全機能の実作用とWindows導入済みkeyboard evidenceを出荷阻害条件として記載。Registry状態は不変で、C21とのID対応はUNKNOWN。 |
| 状態 | C21 E0-04 mapping記載済み。旧status不変。N09／N10／E0はOPEN。 |
| 残余 | installed keyboard／accessibility、各操作面の実Consumer、製品一連操作への接続。 |

#### C22 全体検索

| E0-04項目 | 個別値 |
|---|---|
| 現ID | C22 bounded Global Searchによるsnapshot／surface entry検索。 |
| 旧ID | 総合機能拡張rev1 C22／Phase 28。bounded snapshot検索とlive横断検索不足を履歴保持。 |
| 原文参照 | rev1実装仕様・工程・進捗C22、docs/D4_POCKET_PHASE_MAPPING.md、v4仕様S04 N09・S05 K13、global-search-surface.md。 |
| 対象 | Flutter ShellSnapshotをlocal indexへ取り込み、固定上限のquery結果／surface entryを表示するDesktop UI。 |
| 目的 | 表示情報を検索し、秘密・会話本文・Audit raw payloadを複製せず、検索からAuthorityを生成しない。 |
| 生成元（Producer） | Broker projectionとして渡されたShellSnapshotと固定surface entry。任意path・生会話・秘密は対象外。 |
| 実Consumer | Flutter global_search_index.dartがquery 128文字、index 512件、result 30件以内で処理しSearch UIへ返す。 |
| 契約／Data | global-search-surface.md、ShellSnapshotのbounded text／metadata、surface navigation entry。live Broker query契約ではない。 |
| 正常時作用 | 受信snapshotの候補を検索・表示する。source未確認snapshotは不明とし、surface entryは入口の存在だけを示す。 |
| 否定時作用 | 上限を超える入力を制限し、会話本文、Approval／Audit raw payload、秘密、任意filesystem pathをindexしない。 |
| 停止／Recovery | snapshot未取得／stale／source不明はlive状態と見なさない。更新失敗を未取得と表示し旧snapshotから作用しない。 |
| 標準OS候補 | Desktop Flutter UI。Windows／macOS／Linux installed Search未確認。Mobileは別consumer。Linux native必須だが本source mappingは保証でない。 |
| 配布物 | 独立配布物なし。D4 Pocket Desktop UIに含む候補。 |
| 既存実装 | Search surface contract、bounded index、navigationとunit/widget test sourceあり。全Broker横断live searchなし。 |
| BUILD_GAP | Brokerの現在状態を問い合わせる実経路、全画面を横断する受取先、導入済み製品でのOS別検索証拠は未成立。 |
| ソース識別値（SourceSHA） | SourceSHA表のglobal-search-surface.md、global_search_index.dart、関連test、main.dart／widget_test.dartを照合し63/63一致。 |
| 試験区分（TestClass） | Flutter UNIT／WIDGET／snapshot fixture sourceあり。本mappingではNOT_RUN。 |
| 証拠ID（Evidence ID） | NONE。snapshot／fixture検索をlive cross-Broker実測やinstalled Acceptanceへしない。 |
| 阻害要因（Blocker） | 全体検索契約は実Runtime／Agent等を含む導入済み横断検索とartifact結合を出荷阻害条件として記載。Registry状態は不変で、C22とのID対応はUNKNOWN。 |
| 状態 | C22 E0-04 mapping記載済み。旧status不変。N09／N10／E0はOPEN。 |
| 残余 | snapshot鮮度／source-bound live query、検索対象の完全性、installed UI、N11証拠結合。 |

#### C23 Desktop UX統合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | C23 Desktop navigation groupと全screen到達性を統合するUX surface。 |
| 旧ID | 総合機能拡張rev1 C23／Phase 28。既存画面grouping／到達性とWindows installed未検証を履歴保持。 |
| 原文参照 | rev1実装仕様・工程・進捗C23、docs/D4_POCKET_PHASE_MAPPING.md、v4仕様S04 N09/N10・S05 K13、desktop-ux-integration.md。 |
| 対象 | Desktop main.dart NavigationRail／group stateから既存surfaceへ到達し、palette／search導線を保つ画面構成。 |
| 目的 | 画面選択と操作導線を一貫させ、UI整理で機能・Permission・Broker境界を削除／変形しない。 |
| 生成元（Producer） | OperatorのNavigation group／screen selectionと各既存画面のnavigation entry。 |
| 実Consumer | Flutter main.dartのnavigation stateが21画面を運用／安全／開発／設定groupへ表示し既存screenへ切り替える。 |
| 契約／Data | desktop-ux-integration.md、Flutter内group／selected screen state、既存route。共通権限／外部作用Schemaなし。 |
| 正常時作用 | Group選択で画面表示を切り替え、既存surfaceとpalette／search導線への到達性を保つ。 |
| 否定時作用 | group切替でsurface／Authority／Permission／filesystem／process／network／Credential capabilityを暗黙に削除・追加しない。 |
| 停止／Recovery | navigation失敗・画面破棄はUI状態に留まり、Broker状態変更・Task cancel・Recovery成功を偽装しない。 |
| 標準OS候補 | Desktop Flutter UX。Windows／macOS／Linux installed navigation／accessibility未成立。Mobile navigationは別実装範囲。 |
| 配布物 | 独立配布物なし。D4 Pocket Desktop main UI。 |
| 既存実装 | Desktop UX contract、main.dart group/navigation、関連widget test sourceあり。screen sourceはinstalled操作受入れでない。 |
| BUILD_GAP | OS標準installed候補での画面到達性、全surface実Consumer、製品UX／accessibility受入れは未成立。 |
| ソース識別値（SourceSHA） | SourceSHA表のdesktop-ux-integration.md、main.dart、widget_test.dart、共有palette／search sourceを照合し63/63一致。 |
| 試験区分（TestClass） | Flutter WIDGET／navigation fixture sourceあり。本mappingではNOT_RUN。 |
| 証拠ID（Evidence ID） | NONE。Navigation fixtureを製品UX／OS installed QAとしない。 |
| 阻害要因（Blocker） | Desktop UX契約はWindows導入済み画面証拠、全C0〜C34完成、Owner最終判断を出荷阻害条件として記載。Registry状態は不変で、C23とのID対応はUNKNOWN。 |
| 状態 | C23 E0-04 mapping記載済み。旧status不変。N09／N10／E0はOPEN。 |
| 残余 | 各Desktop OS向け導入済み製品での画面操作、21画面の到達性と実作用、支援技術対応、N11証拠と配布物の結合。 |

C15〜C23のE0-04個別mappingは文書上完了。旧C15〜C23 status、N03/N04/N09/N10、W03、Release Registryを変更しない。63件のSourceSHA一致はソース同一性のみを示し、外部Agent、Host、Adapter、Tray、installed Desktopの実行・Acceptance・五OS QAを意味しない。

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

C24〜C34の歴史的SourceSHA表はbaseline main d45fa6109fa6300ed42449ea03a93bfe25f7ae95を固定する。編集前main 9c3838e8f06c5d2233830fab8b6ea23982a28860で56/56 pathを再確認し、55/56 blobはbaselineと一致、C31の進捗文書だけ後続更新を確認した。現行Source→ConsumerとE0-04全21 fieldは次節に記録する。旧C28の8時間FAIL、C33の総合installed FAIL、C34未着手をPASSへ読み替えず、旧test結果は履歴に保つ。製品testはこのmapping単位でNOT_RUN。C0〜C34の35 IDとPhase0〜45/K01〜K20のscope mappingは記録済みだが、Buffer/Q/FQ/W全field、標準OS QA、N11 evidence結合が残り、E0・N10・N11はOPEN。

### C24〜C34 E0-04全21項目の現行Source→Consumer照合（2026-11-13）

照合基準は編集前のcurrent main 9c3838e8f06c5d2233830fab8b6ea23982a28860。旧SourceSHA tableの56 pathを現HEADへ再照合し、全path存在、55/56 baseline blob一致を確認した。唯一の後続差分はC31のdocs/QE_2026-11-13/進捗.mdで、旧値d75e96eb788a6f980465575337a1afcca3dc135cから編集前HEADのd67d0199e48458510defd765499ff3d1054d0442へ更新されている。各C rowのSourceSHAは直前のpath／baseline blob tableとこの差分注記を組み合わせて固定する。旧C status／失敗・限定PASS、N/K／Registry状態は変更せず、現行製品受入Evidence IDは発行しない。

#### C24 Mobile補助操作面／Device Link

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 旧C24 Mobile操作・Device Link投影。v4 N09/N10、K03/K04/K13/K17。 |
| 旧ID | 総合機能拡張rev1 C24／Phase 35。9画面、通知summary、Runtime／資源／履歴metadata、MCP未観測、停止receipt、Recovery導線の履歴。 |
| 原文参照 | rev1実装仕様・工程表・進捗C24、docs/D4_POCKET_PHASE_MAPPING.md Phase 35、v4仕様S01/S02/S04 N09・N10/S05 K03・K04・K13・K17、docs/specs/mobile-surface.md。 |
| 対象 | `Mobile` Flutter投影とAndroid `Kotlin`／`iOS` `Swift` `Device` `Link` clientからDesktop `Rust` Brokerのread-`only` allowlistへ接続する範囲。 |
| 目的 | Task実行AuthorityをMobileへ移さず、登録済みDesktopの許可された状態・識別子だけを利用者へ表示する。 |
| 生成元（Producer） | 利用者のMobile画面操作、登録済みDevice Link endpoint、Desktop Brokerの限定projection。 |
| 実Consumer | `MobileProjectionClient`／`DeviceLinkClient`→`Android`／`iOS` `native` `DeviceLink` `service`→既存TLS `transport`→`Rust` `Broker` `read-only` `handler`→`bounded` Flutter表示。 |
| 契約／Data | mobile-surface契約、mobile_agent_list.schema.json、mobile_workspace_selection.schema.json。Runtime／Workspace識別子等のallowlist投影。 |
| 正常時作用 | 認証済み接続で許可されたmetadataだけを取得し表示する。Agent statusはmetadataでありTrustやTask capabilityではない。 |
| 否定時作用 | 未知／malformed識別子、TLS・登録・scope不一致、未許可fieldを拒否。秘密実値、履歴本文、Workspace pathを出さず、取得不能resourceはunknownとする。 |
| 停止／Recovery | 失効・transport／Broker異常・応答不一致時はprojectionを停止しunknown／errorを表示する。停止receipt executed=falseはTask実停止ではなく、自動再送もしない。 |
| 標準OS候補 | MobileはAndroid EmulatorとiOS Simulator、接続先Desktopは宣言対象のWindows／macOS／Linux native。Emulator／Simulator実測は本mappingで未実施。 |
| 配布物 | 独立配布物なし。将来のD4 Pocket Mobile AppのAndroid／iOS標準配布artifact。正式identity／署名は別Release条件。 |
| 既存実装 | `Flutter` `projection`/`client`、`Kotlin`／`Swift` `Device` `Link` `service`、`read-only` `Rust` `Broker` `handlers`、`Schemas`、`Flutter`／`Android`／`iOS` `workflow` sourceがある。 |
| BUILD_GAP | MobileからAgentTask／MCP実作用を開始する経路、live host inventory、現候補の標準Emulator／Simulator installed lifecycleとDesktop Device Link結合が未成立。 |
| ソース識別値（SourceSHA） | 旧C24 SourceSHA tableの11 pathは変更前main 9c3838e8f06c5d2233830fab8b6ea23982a28860で11/11 blob一致。対象pathは同表に固定。 |
| 試験区分（TestClass） | Schema／Flutter UNIT・WIDGET／FIXTURE、Android／iOS workflow source。旧試験結果は履歴。本mappingの製品・workflow実行はNOT_RUN。 |
| 証拠ID（Evidence ID） | NONE。Source、fixture、旧限定試験を現候補Acceptanceへ昇格しない。 |
| 阻害要因（Blocker） | 既存Registryのrev2_mobile_device_evidence、rev2_mobile_flutter_native_device_link_boundary、rev2_mobile_distributionはunresolved。comprehensive_extension_rev1_completionもunresolved。statusは変更しない。 |
| 状態 | C24のE0-04 21 fieldを記録。旧C24履歴を維持。N09／N10／N11／E0はOPEN。 |
| 残余 | 標準Emulator／Simulator installed product、Desktop接続との一体動作、OS native秘密・Lifecycle、Broker権限外操作拒否、N11 Artifact結合。 |

#### C25 Apple補助workflow

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 旧C25 Apple補助検証。v4 N10/N11、K17/K19。 |
| 旧ID | 総合機能拡張rev1 C25／Phase 36。apple-manual-build.ymlのworkflow_dispatch補助経路を保持し、当時のWindows host起動・現行target証拠なし。 |
| 原文参照 | rev1工程表・進捗C25、docs/D4_POCKET_PHASE_MAPPING.md Phase 36、v4仕様S02/S04 N10・N11/S05 K17・K19、.github/workflows/apple-manual-build.yml。 |
| 対象 | 手動起動のmacOS／iOS target build・test補助経路と、その結果をcandidate・OS別に記録する範囲。 |
| 目的 | Windows上にないApple OS環境の検査を手動runnerで補い、Source／target／artifact／実行結果の同一性を保つ。 |
| 生成元（Producer） | 明示的workflow_dispatchと固定されたsource commit／target選択。 |
| 実Consumer | apple-manual-build.ymlのmacOS runner jobsとiOS Simulator向けbuild／test job。workflow definitionがconsumerであり、dispatch記録は別の実行結果。 |
| 契約／Data | workflow_dispatch inputs、checked-out commit、target OS／simulator、build／test result、artifact metadata。Secretsや署名資格を新設・表示しない。 |
| 正常時作用 | 指定commit・targetでjobが完了すれば当該build／testだけの補助証拠を保存する。installed product／署名／配布成功へ昇格しない。 |
| 否定時作用 | 未知target、checkout hash不一致、build／test失敗、artifact欠落を失敗として扱い、未実行jobをPASSにしない。 |
| 停止／Recovery | runner不足・job failure・証拠不整合では対象の補助検証を停止し、正確なrun・commit・targetを残す。retryだけで根因解消としない。 |
| 標準OS候補 | macOS native VM／適法hosted GUI、iOS Simulator。workflow sourceは実行済み保証ではなく、物理iPhoneを必須条件に戻さない。 |
| 配布物 | 検証用build／test artifact。macOS正式bundle、署名済iOS distribution artifact、Store配布とは別。 |
| 既存実装 | apple-manual-build.ymlにmanual target別入口が存在する。 |
| BUILD_GAP | 現candidateのmanual run結果、GUI installed execution、Source／Artifact identity結合、署名／notarization／distributionは未成立。 |
| ソース識別値（SourceSHA） | 旧C25 SourceSHA tableの1 pathは変更前main 9c3838e8f06c5d2233830fab8b6ea23982a28860で1/1 blob一致。 |
| 試験区分（TestClass） | WORKFLOW_SOURCE／SOURCE。現candidate workflow実行・Apple OS product試験はNOT_RUN。 |
| 証拠ID（Evidence ID） | NONE。workflow sourceをOS実行証拠にしない。 |
| 阻害要因（Blocker） | C25専用Registry IDなし。comprehensive_extension_rev1_completionはunresolved。iOS配布面ではrev2_mobile_device_evidence／rev2_mobile_distributionの履歴statusもunresolved。 |
| 状態 | C25 E0-04 mapping記録済み。過去workflow statusは維持。N10／N11／E0 OPEN。 |
| 残余 | 現候補macOS／iOS Simulator build・test、installed GUI／lifecycle、配布artifact identity・署名条件とN11結合。 |

#### C26 Android build／Emulator回帰

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 旧C26 Android build回帰。v4 N10/N11、K17。 |
| 旧ID | 総合機能拡張rev1 C26／Phase 35・36。Flutter analyze・32試験・debug APK／AAB buildの過去PASSを保持。Android install／launch・正式署名は未取得。 |
| 原文参照 | rev1工程表・進捗C26、docs/D4_POCKET_PHASE_MAPPING.md Phase 35／36、v4仕様S02/S04 N10/S05 K17、android-manual-emulator.yml、NativeDeviceLinkProductFlowTest.kt。 |
| 対象 | `Android` `manual` workflowのFlutter検査、`debug` `package` `build`、`API` 35 `Google` `APIs` `x86_64` `Emulator` `install`／`launch`／`native` `Device` `Link` `test`。 |
| 目的 | 開発用Android経路のbuild・Emulator実行を再現し、結果の対象commit／API／ABI／artifact範囲を限定して記録する。 |
| 生成元（Producer） | android-manual-emulator.ymlのworkflow_dispatch、Gradle／Flutter build、Emulator test invocation。 |
| 実Consumer | `GitHub` `Actions` `Android` `runner`／`Emulator`、`Flutter` `analyzer`/`tests`、`Gradle` `debug` `APK`／`AAB`、`NativeDeviceLinkProductFlowTest`。 |
| 契約／Data | `workflow` `inputs`、`API` 35／`x86_64` `system` `image`、`debug` `artifact` `SHA`、`install`／`launch`／`JUnit` `results`、`Device` `Link` `boundary` `contract`。 |
| 正常時作用 | 同一run内のbuildとEmulator testが成功した範囲だけを保存する。debug packageは正式署名・配布物と別に扱う。 |
| 否定時作用 | build／install／launch／test errorを失敗として保持。credential／TLS／native identity不一致を成功化せず、Flutter metadataからAuthorityを作らない。 |
| 停止／Recovery | Emulator起動・install・native serviceが失敗したら当該jobを終了し、target・log・artifactを区別。正式配布・production keyへ進まない。 |
| 標準OS候補 | Android Emulator API 35 Google APIs x86_64はworkflow候補。Android標準release OS／ABI matrixの適合は未検証。 |
| 配布物 | 過去debug APK／AAB、およびworkflowのdebug APK／AAB artifact。正式Android package identity／署名とは異なる。 |
| 既存実装 | Gradle設定、`manual` `Emulator` `workflow`、`native` `Device` `Link` `instrumented` `test` sourceがある。 |
| BUILD_GAP | 現candidate workflow実行、標準Emulator installed lifecycle、KeyStore／Permission／update／uninstall、正式配布identityは未成立。 |
| ソース識別値（SourceSHA） | 旧C26 SourceSHA tableの3 pathは変更前main 9c3838e8f06c5d2233830fab8b6ea23982a28860で3/3 blob一致。 |
| 試験区分（TestClass） | WORKFLOW_SOURCE／Flutter UNIT／FIXTURE／debug buildの旧SOURCE履歴。今回のworkflow・製品試験はNOT_RUN。 |
| 証拠ID（Evidence ID） | NONE。過去debug buildやtest sourceをcurrent installed Acceptanceにしない。 |
| 阻害要因（Blocker） | rev2_mobile_device_evidence、rev2_mobile_distribution、rev2_mobile_flutter_native_device_link_boundaryがunresolved。全能力blocker comprehensive_extension_rev1_completionもunresolved。 |
| 状態 | C26 E0-04 mapping記録済み。過去debug build範囲のみ履歴保持。N10／N11／E0 OPEN。 |
| 残余 | 現候補Emulator install／launch、OS native secret／Device Link／lifecycle、正式artifact identity、更新・撤去、N11結合。 |

#### C27 性能／UI停止watchdog

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 旧C27 開発用性能・UI停止監視。v4 N09/N11、K19。 |
| 旧ID | 総合機能拡張rev1 C27／Phase 41。snapshot、Flutter projection、4096行diffのbounded測定PASS履歴を保持。 |
| 原文参照 | rev1工程表・進捗C27、docs/D4_POCKET_PHASE_MAPPING.md Phase 41、v4仕様S04 N09／S05 K19、docs/specs/performance-validation.md。 |
| 対象 | performance_validation.pyから生成する内部snapshotとc27_performance_validation_test.dartのFlutter projection／diff watchdog測定。 |
| 目的 | 開発中の限定処理で計測値とwatchdog失敗を検出する。 |
| 生成元（Producer） | 開発validatorが生成するsnapshot／bounded diff入力とFlutter test操作。 |
| 実Consumer | `tooling`/`performance_validation.py`→`generated` `snapshot`／`c27_performance_validation_test.dart`→`Flutter` `projection`・`watchdog` `result`。 |
| 契約／Data | 性能検査仕様のcase・測定結果・threshold・duration・resource state。欠測resourceはunknownで保持。 |
| 正常時作用 | fixture／内部stateの処理時間とdiff範囲を測り、watchdog内に終わった事実だけを報告する。 |
| 否定時作用 | threshold超過・stall・invalid snapshotをfailureにする。unknown resourceを0へ置換しない。 |
| 停止／Recovery | watchdog超過で該当caseを停止し失敗を記録。内部測定からruntime healthやrecovery成功を推定しない。 |
| 標準OS候補 | 旧測定は開発環境／Flutter test。v4対象五OSの標準installed performance matrixはE4/E5まで未固定・未測定。 |
| 配布物 | 独立配布物なし。開発validator／Flutter test。 |
| 既存実装 | performance_validation.py、Flutter test、性能検査仕様がある。 |
| BUILD_GAP | installed cold/warm startup、実画面frame／GPU、実Agent／MCP／A2A負荷、OS別Resource SLAと同一candidate証拠は未成立。 |
| ソース識別値（SourceSHA） | 旧C27 SourceSHA tableの3 pathは変更前main 9c3838e8f06c5d2233830fab8b6ea23982a28860で3/3 blob一致。 |
| 試験区分（TestClass） | INTERNAL_STATE／FIXTUREの限定開発測定。過去sourceの実行結果は履歴。本mappingでのtestはNOT_RUN。 |
| 証拠ID（Evidence ID） | NONE。限定watchdog PASSを製品性能QA／SLAにしない。 |
| 阻害要因（Blocker） | 専用C27 Registry IDはなし。comprehensive_extension_rev1_completionはunresolved。全体性能の正式証拠はQ4／R14後段に属する。 |
| 状態 | C27の旧限定PASS範囲を保持。C27 E0-04 mapping記録済み、N09／N11／E0 OPEN。 |
| 残余 | 統合候補での実測、資源／frame／startup条件固定、標準OS別結果とN11 artifact／evidence結合。 |

#### C28 長時間運転／通信監視

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 旧C28 長時間運用検証基盤。v4 N01/N08/N11、K19/K20。 |
| 旧ID | 総合機能拡張rev1 C28／Phase 39。短時間Broker IPC smoke／120秒stressは限定PASS。clean-source 8時間試行は22.766秒でFAILした履歴を保持。 |
| 原文参照 | rev1工程表・進捗C28、docs/D4_POCKET_PHASE_MAPPING.md Phase 39、v4仕様S04 N01/N08/N11・S05 K19/K20、docs/specs/long-run-validation.md、FQ-INTEGRATED。 |
| 対象 | long_run_validation.pyの8時間run option、Broker restart／network monitor、localhost Runtime fixture、結果JSON／failure classification。 |
| 目的 | 候補の長時間挙動と失敗を計測し、clientが確認していない応答・Audit／state不整合を成功にしない。 |
| 生成元（Producer） | 長時間runner、`Broker` `IPC`、`localhost` `HTTP` `Runtime` `fixture`、監督対象のrestart／`network` `events`。 |
| 実Consumer | long_run_validation.pyがBroker IPCとfixture応答を監督し、result JSON／failure classifierを作る。 |
| 契約／Data | long-run-validation.mdのduration、stage、heartbeat、client response、Audit／state、resource sample、cleanup status。 |
| 正常時作用 | 各観測済みstageとclient受信、永続state、cleanupを区別して報告する。120秒stress PASSは8時間結果へ適用しない。 |
| 否定時作用 | timeout、未確認client receive、fixture／Audit/state異常、resource不明、cleanup未成立をFAIL／UNKNOWNとして保持する。 |
| 停止／Recovery | 再起動後のidentity／Audit不整合やcleanup失敗は中断し、bounded cleanupの最終状態を記録。自動retryだけで復旧PASSにしない。 |
| 標準OS候補 | 旧8h runはWindows development environment、Broker LIVE_RUNTIMEとRuntime FIXTUREが混在。v4五OS installed candidateの長時間試験は未実施。 |
| 配布物 | 独立配布物なし。開発長時間runnerとfixture。 |
| 既存実装 | long_run_validation.py、仕様、Conformance test source、Broker IPCとlocalhost fixtureの限定経路がある。 |
| BUILD_GAP | 8h成功、現候補installed product、実外部Runtime負荷、全OS資源・永続state・process recoveryの一体測定が未成立。旧FAILの原因解消もこのmappingでは再試験しない。 |
| ソース識別値（SourceSHA） | 旧C28 SourceSHA tableの3 pathは変更前main 9c3838e8f06c5d2233830fab8b6ea23982a28860で3/3 blob一致。 |
| 試験区分（TestClass） | 過去Broker IPCは限定LIVE_RUNTIME、RuntimeはFIXTURE、resourceはINTERNAL_STATE。8時間FAILを含む旧履歴のみ。本mappingのrunはNOT_RUN。 |
| 証拠ID（Evidence ID） | NONE。旧8時間FAILを消去・再分類せず、current candidate evidenceも発行しない。 |
| 阻害要因（Blocker） | 専用C28 Registry IDはなし。comprehensive_extension_rev1_completionはunresolved。正式長期／耐久試験はQ4等の全体QC工程。 |
| 状態 | C28の8h historical FAILと短時間限定PASSを保持。E0-04 mapping記録済み、N01／N08／N11／E0 OPEN。 |
| 残余 | 8時間同一candidate実行、client受信とAudit／state／resourceの連続観測、失敗根因とcleanup、N11連結。 |

#### C29 障害注入validator

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 旧C29 開発用障害注入。v4 N01/N02/N04/N05/N08/N11、K19/K20。 |
| 旧ID | 総合機能拡張rev1 C29／Phase 40。Runtime／Broker／MCP／A2A／Credential／store／Audit／stateの8分類試験PASSを履歴保持。 |
| 原文参照 | rev1工程表・進捗C29、docs/D4_POCKET_PHASE_MAPPING.md Phase 40、v4仕様S04 N01/N02/N04/N05/N08/N11、docs/specs/failure-injection-validation.md。 |
| 対象 | failure_injection_validation.pyの開発用Broker IPC、local MCP stdio／A2A loopback fixtureに限定した故障投入と判定。 |
| 目的 | 障害・応答不確定・Credential／Audit失敗を成功、Authority、再送へ昇格させないことを局所検査する。 |
| 生成元（Producer） | failure injectorが作るBroker／fixture fault caseと失敗応答。 |
| 実Consumer | failure_injection_validation.py→Rust Broker IPCおよびlocal MCP／A2A fixture→case result／failure classification。 |
| 契約／Data | failure-injection-validation.mdのfault class、expected deny／suspend、Audit／Recovery outcome、credential-safe output。 |
| 正常時作用 | 故障caseが期待する拒否・停止・bounded reportを返した場合だけ、その開発caseの期待値をPASSにする。 |
| 否定時作用 | 曖昧結果をsuccess、Permission／Approval、Audit、Recoveryへ進めない。credential非露出、非再送を維持。 |
| 停止／Recovery | Audit不可、応答不明、state破損などでは対象case／作用を停止。fixtureの応答を外部serviceの再現と扱わない。 |
| 標準OS候補 | Local development hostとBroker test harness。五OS標準installed productのfault injectionは未実施。 |
| 配布物 | 独立配布物なし。開発validator／local fixtures。 |
| 既存実装 | failure_injection_validation.py、failure-injection-validation.md、Broker IPC／MCP／A2A fixture sourceがある。 |
| BUILD_GAP | 外部Runtime／MCP／A2Aやinstalled candidateのfailure、OS process／network／storage障害と製品Recoveryの一体検査は未成立。 |
| ソース識別値（SourceSHA） | 旧C29専用2 pathとC29–30共有Conformance pathは変更前main 9c3838e8f06c5d2233830fab8b6ea23982a28860で3 reference全てblob一致。 |
| 試験区分（TestClass） | LOCAL_BROKER／UNIT／FIXTURE中心。8旧caseの限定PASS履歴を保持。本mappingのvalidator実行はNOT_RUN。 |
| 証拠ID（Evidence ID） | NONE。旧fixture PASSをinstalled／external failure evidenceへ流用しない。 |
| 阻害要因（Blocker） | 専用C29 Registry IDはなし。comprehensive_extension_rev1_completionはunresolved。各機能別既存Blockerのstatusは不変。 |
| 状態 | 旧C29の8開発用case PASS履歴を保持。E0-04 mapping記録済み、現行関連NとE0 OPEN。 |
| 残余 | production／installed failure behavior、external service cases、process descendant cleanup、Audit／Recovery、五OS証拠結合。 |

#### C30 全数回帰matrix

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 旧C30 Local全数回帰matrix。v4 N11/K19。各回帰caseは該当N/K consumerにも接続する。 |
| 旧ID | 総合機能拡張rev1 C30／Phase 7・8・42。Trust／Authority／Permission／Approval／Audit／Recovery／Evidence／Runtime／Agent／Dialogue／Compare／Device Linkの開発matrix履歴。 |
| 原文参照 | rev1工程表・進捗C30、docs/D4_POCKET_PHASE_MAPPING.md Phase 7／8／42、v4仕様S03 ARC-J／S04 N11／S05 K19、docs/specs/full-regression-validation.md。 |
| 対象 | full_regression_validation.pyと固定local regression entries、Conformanceによるentry／classification形状確認。 |
| 目的 | 既知の安全・状態・Runtime・Agent等のlocal回帰を分類して、狭い開発検査を再現できるようにする。 |
| 生成元（Producer） | 手動実行のlocal regression validatorと各entryの既存unit／fixture/assertion結果。 |
| 実Consumer | `full_regression_validation.py`→`full-regression-validation.md` `matrix`→`Conformance` `entry`/`classification` `check`。 |
| 契約／Data | 固定matrix entry、requirement／test reference、scope／OS／evidence class／result。全OS installed resultは含まれない。 |
| 正常時作用 | 対象entryを列挙し、その実行済み検査結果だけを集約する。未実行consumerは未確認のまま残す。 |
| 否定時作用 | unknown input、missing path／entry、classification欠落、権限昇格・Audit／Recovery違反をrejectまたはfailedとして示す。 |
| 停止／Recovery | matrix／validator failureを成功へ隠さず、影響testに限定して修正する。実製品のprocess／OS recoveryを模擬済みにしない。 |
| 標準OS候補 | Local source／Conformance hostが対象。v4標準五OS installed regression suiteは全体E4/E5に未到達。 |
| 配布物 | 独立配布物なし。開発validator、matrix文書、test references。 |
| 既存実装 | full_regression_validation.py、matrix仕様、Conformance validatorがある。 |
| BUILD_GAP | 全必須要求・実consumer・OS packageを同一Release candidateへ結合した横断regressionは未成立。外部Runtime／Agentとinstalled productは対象外。 |
| ソース識別値（SourceSHA） | 旧C30専用2 pathとC29–30共有Conformance pathは変更前main 9c3838e8f06c5d2233830fab8b6ea23982a28860で3 reference全てblob一致。 |
| 試験区分（TestClass） | LOCAL_REGRESSION／CONFORMANCE source。旧履歴の実装範囲を保持。本mappingのvalidator・製品regressionはNOT_RUN。 |
| 証拠ID（Evidence ID） | NONE。matrix構造やexit codeだけで全要求・全OS QAを証明しない。 |
| 阻害要因（Blocker） | 専用C30 Registry IDはなし。comprehensive_extension_rev1_completionはunresolved。既存個別Blockerはその条件ごとに維持。 |
| 状態 | C30 E0-04 mapping記録済み。旧matrix履歴を維持。N11／E0 OPEN。 |
| 残余 | matrix全要求の実consumer結合、current candidate result、標準OS installed／external runtime証拠、N11 Evidence／Artifact統合。 |

#### C31 文書・製品claim同期

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 旧C31 製品文書更新。v4 N11/K19。 |
| 旧ID | 総合機能拡張rev1 C31／Phase 1・43。README／ROADMAP／GUI操作面／SECURITY／CONFIG／MOBILE_STATUS／COMPATIBILITY_MATRIX／REV2_PROGRESSを当時のC24〜C30 scopeへ同期した履歴。 |
| 原文参照 | rev1工程表・進捗C31、docs/D4_POCKET_PHASE_MAPPING.md Phase 1／43、v4仕様S00/S10/N11/K19、docs/QE_2026-11-13/採用記録.md。 |
| 対象 | 製品文書と要求・source・実結果・OS保証・Release gate間の説明整合。 |
| 目的 | 使い方・対応OS・機能・証拠の公開説明を現行正本と実在consumerに一致させ、古いstatus／failureを歴史として保持する。 |
| 生成元（Producer） | 現行v4正本、実装／検証結果、Registry／Manifest状態、過去進捗。 |
| 実Consumer | README／ROADMAP／GUI_OPERATION_SURFACES／SECURITY／CONFIG／MOBILE_STATUS／COMPATIBILITY_MATRIX／REV2_PROGRESS／現v4進捗→Manifest／Conformance／Release Gateと利用者説明。 |
| 契約／Data | 版、採用hash、source／artifact identity、evidence class、OS scope、OPEN／FAIL／NOT_RUN、Release blocker。文言はruntime能力を生成しない。 |
| 正常時作用 | 現行の確認済状態に限定して文書へ反映し、過去記録は追記・参照で保全する。 |
| 否定時作用 | 旧文書・claimと正本／evidenceが矛盾したら現行正本を優先し、unsupported／UNKNOWNを成功表示せず差分を残す。 |
| 停止／Recovery | source／artifactの根拠がない主張は公開状態へ進めず、文書修正だけで製品failure／AcceptanceをPASS化しない。 |
| 標準OS候補 | 五OSの利用・配布claimを扱う文書consumer。各OSのinstalled適合は別々に未成立／UNKNOWNとして管理する。 |
| 配布物 | README等の製品案内とRelease説明。実Installer／packageではない。 |
| 既存実装 | 列挙した製品文書・current v4 set・Manifest/Conformance/Release Gateが存在する。全現行文書の横断整合監査を完了した意味ではない。 |
| BUILD_GAP | 全文書と現Release candidate artifact／installed UXを同一候補に結ぶ監査は未成立。C31旧文書更新statusのみでは現行整合を証明しない。 |
| ソース識別値（SourceSHA） | 旧C31の9 refs中8件は変更前main 9c3838e8f06c5d2233830fab8b6ea23982a28860でbaseline blob一致。docs/QE_2026-11-13/進捗.mdはbaseline SHA d75e96eb788a6f980465575337a1afcca3dc135cから現行pre-edit SHA d67d0199e48458510defd765499ff3d1054d0442へ更新済み。 |
| 試験区分（TestClass） | DOC／MANIFEST／CONFORMANCE／RELEASE_GATEのconsumer。ここで実行する文書構造検査は製品Document consumerの検査であり、製品runtime testはNOT_RUN。 |
| 証拠ID（Evidence ID） | NONE。文書hash／Manifest合格を製品QAや公開承認へしない。 |
| 阻害要因（Blocker） | 専用C31 Registry IDはなし。comprehensive_extension_rev1_completionはunresolved。具体的なOS／配布blockerはそれぞれのRegistry条件を保持。 |
| 状態 | C31の過去文書更新statusとFAIL履歴を維持。今回のC31 E0-04 mapping記録済み。N11／E0 OPEN。 |
| 残余 | 公開文書全数のcandidate同一性監査、五OS installed evidence・Registry同期、N11結合。 |

#### C32 最終開発監査validator

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 旧C32 最終開発監査／対応表検査。v4 N11/K19。 |
| 旧ID | 総合機能拡張rev1 C32／Phase 42・44。C0〜C31のmapping structure／path／ID／残余分類を検査した旧checker／Conformance PASS履歴。C32〜C34は未監査。 |
| 原文参照 | rev1工程表・進捗C32、docs/D4_POCKET_PHASE_MAPPING.md Phase 42／44、v4仕様S04 N11/S05 K19、final-development-audit.json/.md、tooling/final_development_audit.py。 |
| 対象 | 旧C0〜C31静的対応表と、C32−C34を未監査として明示するfinal-development-audit構造検査。 |
| 目的 | 要求対応資料のfield／参照path／ID／risk分類の欠落を検出し、開発監査の記録整合を確認する。 |
| 生成元（Producer） | final-development-audit.jsonのscopeとC0〜C31 mapping／参照source。 |
| 実Consumer | tooling/final_development_audit.py→report result、tooling/conformance_tests/test_final_development_audit.py→構造・entry検査。 |
| 契約／Data | `audit` `report` `schema`、`required` `fields`、`path` `existence`、`C` `ID` `range`、`risk` `classification`、`not-audited` `range`。 |
| 正常時作用 | 構造と参照整合だけを判定し、未監査IDを明示する。 |
| 否定時作用 | field/path/ID/classification欠落・不正値をfailureにし、report passedを実runtime／Acceptance passとして扱わない。 |
| 停止／Recovery | 参照欠損や未知ID時はreportを未完了に留め、Owner GO／Registry／Product statusを変更しない。 |
| 標準OS候補 | Repository-local Python validator／Conformance。対象OSのruntimeやinstalled appを起動しない。 |
| 配布物 | 独立配布物なし。静的監査器と報告data。 |
| 既存実装 | final-development-audit.json／.md、Python checker、unittest／Conformance sourcesがある。 |
| BUILD_GAP | C32〜C34、全N/P/Phase/K/BUF/Q/FQ/W/OS/artifactの現行E0／N11一体監査は未成立。 |
| ソース識別値（SourceSHA） | 旧C32 SourceSHA tableの6 pathは変更前main 9c3838e8f06c5d2233830fab8b6ea23982a28860で6/6 blob一致。 |
| 試験区分（TestClass） | STRUCTURE／UNIT／CONFORMANCEの旧source履歴。旧checker限定PASSを保持。本mappingでのchecker／test実行はNOT_RUN。 |
| 証拠ID（Evidence ID） | NONE。旧C32のC0〜C31 static audit結果は現行N11 Evidenceにならない。 |
| 阻害要因（Blocker） | comprehensive_extension_rev1_completion unresolved。owner_goもunresolvedだが、これは最終Release判断であり構造検査の実行権ではない。 |
| 状態 | 旧C32 checkerの限定PASS・C32〜C34未監査を保持。E0-04 mapping記録済み、N11／E0 OPEN。 |
| 残余 | 現行全要求台帳、artifact／OS identity、Evidence／Blocker一体validation、未監査範囲の実Consumer対応。 |

#### C33 Windows導入済み製品の証拠経路

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 旧C33 Windows最大到達点。v4 N10/N11、K16/K19。 |
| 旧ID | 総合機能拡張rev1 C33／Phase 34・44。Release build／Broker smoke／collector／Conformanceの限定PASSと、Windows installed total evidenceのFAIL履歴を保持。 |
| 原文参照 | rev1工程表・進捗C33、docs/D4_POCKET_PHASE_MAPPING.md Phase 34／44、v4仕様S02/S04 N10・N11/S05 K16・K19、docs/WINDOWS_RELEASE_PLAN.md／WINDOWS_RELEASE_EVIDENCE.md。 |
| 対象 | Windows staged／installed appとBrokerからcollector／release evidence validatorへ候補起動・first-run・証拠provenanceを収集する範囲。 |
| 目的 | current installed candidateのsource／artifact／profile／runtime／config／Audit identityを束ね、現行Windows製品証拠として検査できるようにする。 |
| 生成元（Producer） | 同一sourceから構築したWindows app／Launcher／Broker、導入・first-run実行、stage_installed_app.ps1／collect_installed_smoke.ps1。 |
| 実Consumer | `installed` `app`／`Broker`→`collect_installed_smoke.ps1`／`collect_broker_smoke.ps1`→`windows_release_evidence.py`／`release_smoke.py`→`release` `registry`。 |
| 契約／Data | `Source` `commit`、`launcher`／`Broker`／`app` `hash`、`isolated` `user` `profile`、`runtime`/`config`/`Audit` `store` `identity`、`installed` `evidence` `schema`／`artifact` `path`。 |
| 正常時作用 | current candidateから生成された全一致証拠だけを同一Release runとして受け入れる。各subgateと総合gateを分離する。 |
| 否定時作用 | stale source／artifact、同一profile、path漏れ、Setup Doctor未通過、Audit anchor欠落、旧proof copyをrejectまたは未成立とする。 |
| 停止／Recovery | provenance mismatch・欠測・collector failureでevidence採用を停止し、旧bundleを現候補へ再結合しない。 |
| 標準OS候補 | Windows standard VM／isolated profileに加え所有Windows PCの最終確認。ここではどちらも実行していない。全OS releaseは別N10 row。 |
| 配布物 | 最終Windows general installer／.exe candidateを想定。現在のstage/debug/evidence copyは通常配布物でない。 |
| 既存実装 | stage／collector／Windows evidence validator／release smoke sourceあり。旧proof packにはsource commit 28a700f由来の2026-06-07 evidence copyがある。release_evidenceは.gitkeepのみ。 |
| BUILD_GAP | current candidateのclean installed bundle、正規installer→first-run→Task→Update／Repair／Rollback／Uninstall、owner PC confirmation、external Audit anchorが未成立。 |
| ソース識別値（SourceSHA） | 旧C33 SourceSHA tableの8 pathは変更前main 9c3838e8f06c5d2233830fab8b6ea23982a28860で8/8 blob一致。 |
| 試験区分（TestClass） | 旧subgate SOURCE／LIVE_RUNTIME／EXTERNAL_EVIDENCE copyを履歴分離。旧C33総合installed statusはFAIL。現candidate collector／installed QAはNOT_RUN。 |
| 証拠ID（Evidence ID） | NONE。古いEXTERNAL_EVIDENCE copyおよび旧subgate PASSをcurrent candidate Acceptanceへ持ち込まない。 |
| 阻害要因（Blocker） | 現Registryのwindows_evidence_provenance_isolation、windows_installer_first_run_smoke、windows_setup_doctor_smoke、windows_broker_installed_smoke、audit_anchor_external_tamper_evidence_proof、rev2_desktop_product_distribution、comprehensive_extension_rev1_completionはunresolved。 |
| 状態 | 旧C33総合FAILを維持。C33 E0-04 mapping記録済み、N10／N11／E0 OPEN、release_ready=false。 |
| 残余 | source/artifact identity付きfresh installed candidate、clean別profile、Setup Doctor／Broker／Audit、Windows owner physical confirmation、five-OS evidence join。 |

#### C34 正式公開準備／責任者決裁

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 旧C34 正式Release前準備。v4 N10/N11、K16/K17/K19。 |
| 旧ID | 総合機能拡張rev1 C34／Phase 45。旧進捗は未着手。Publisher identity、formal signing、offline Audit signature、標準OS／実機証拠、strict release、Owner GO要求を保持。 |
| 原文参照 | rev1工程表・進捗C34、docs/D4_POCKET_PHASE_MAPPING.md Phase 45、v4仕様S02/S04 N10・N11/S07/S08/S09、release_blockers.registry.json、FINAL_QA_QUEUE.md、RELEASE_CHECKLIST.md。 |
| 対象 | candidate source／contract／artifact／OS evidence／18 blockerを集約し、strict release gateとOwnerの独立判断へ渡す最終準備。 |
| 目的 | 技術適合、配布資格、production signing／Audit key、release_ready、Owner GOを別々に判定し、未成立を公開扱いにしない。 |
| 生成元（Producer） | 現行要求・Registry、evidence collectors／bundle、OS package identities、strict gate result、責任者の独立判断。 |
| 実Consumer | `tooling`/`evidence_bundle.py`／`release_runtime_assertions.py`→`release_gate_check.py`／`Strict` `Release`→`Registry`／`RELEASE_CHECKLIST`／`FINAL_QA_QUEUE`→`Owner` `decision`。 |
| 契約／Data | 18 `named` `blocker` `statuses`、`candidate`/`source`/`artifact` `hash`、`OS`/`arch`/`install`/`runtime`/`audit` `evidence`, `signing`/`publisher` `identity`, `production` `Audit` `checkpoint`, `release_ready`, `GO` `record`。 |
| 正常時作用 | 完全な同一candidate evidenceがgate条件を満たした結果をOwner判断材料にする。gate passだけでは公開やOwner GOを生成しない。 |
| 否定時作用 | missing/invalid evidence、unresolved blocker、provenance mismatch、strict gate failureはrelease_ready=false／rejectを維持する。 |
| 停止／Recovery | 署名鍵・Publisher・production Audit key・Owner GO不足ではformal release actionを止める。現段階で公開・署名を実行しない。 |
| 標準OS候補 | Windows/macOS/Linux native/Android/iOSの各宣言標準環境とOS固有package。候補version／archはrelease時点で固定し、現時点で全受入れ未成立。 |
| 配布物 | Windows installer、macOS bundle/package、Linux native package、Android package、iOS正規配布artifact。署名／Store／Publisher条件は個別。 |
| 既存実装 | Registry 18 IDs、evidence bundle／runtime assertion／release gate／checklist／queue sourcesあり。release_evidence current bundleは不存在。 |
| BUILD_GAP | E0/N11全数結合、candidate一致の五OS artifact・installed evidence、formal signing／Publisher／offline Audit key、全gate／Owner decisionは未成立。 |
| ソース識別値（SourceSHA） | 旧C34 SourceSHA tableの7 pathは変更前main 9c3838e8f06c5d2233830fab8b6ea23982a28860で7/7 blob一致。 |
| 試験区分（TestClass） | GATE_SOURCE／REGISTRY／DOC。旧進捗C34は未着手。本mappingのstrict release／署名／配布／Owner decisionはNOT_RUN。 |
| 証拠ID（Evidence ID） | NONE。E0 mappingはRelease EvidenceまたはGO記録ではない。 |
| 阻害要因（Blocker） | Registry 18件中16 unresolved／2 resolved、release_ready=false。該当するwindows／linux／mobile distribution・evidence、audit_anchor_external_tamper_evidence_proof、comprehensive_extension_rev1_completion、owner_go等の既存IDを維持。 |
| 状態 | 旧C34未着手statusとRegistryを変更せず、E0-04 mappingのみ記録。N10／N11／E0 OPEN。 |
| 残余 | Release candidate固定、全OS evidence・全blocker判定、Publisher／signing／production Audit key、Ownerの正式GO。 |

C24〜C34の11 ID×21 fieldを文書上展開した。旧C28の8時間FAIL、C33のinstalled総合FAIL、C34未着手は履歴として保持。今回のmappingは製品test・OS installed acceptanceではなく、N10／N11／E0のOPENやRelease Registryを閉じない。次のE0全数作業はPhase／K等の未完全fieldとN11 Evidence／Blocker／OS Artifact結合。

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

#### Phase 0〜5 E0-04個別記録（2026-11-13）

旧Phase/Cの関係を保持し、現行v4 N consumerへ全21項目で接続する。SourceSHAは編集前main `ec68d1c00f7546cf4efbbc3ebe17c81543225564` のGit blob値であり製品受入れを証明しない。

##### Phase 0 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 基準状態固定。 |
| 旧ID | `C0`旧baselineと検査結果を履歴保持。 |
| 原文参照 | `docs/D4_POCKET_PHASE_MAPPING.md` `Phase 0`/`C0`、`v4` `R02` `E0-01〜08` `S10` `I01/I09`。 |
| 対象 | GitHub/local main、dirty/worktree、復旧ref、Registry、Manifest、source/test台帳。 |
| 目的 | 編集基準と保護差分を観測し旧PASSを現候補へ転記しない。 |
| 生成元（Producer） | origin/main、local Git、Registry、Manifest、検査tooling。 |
| 実Consumer | `manifest.py`／`release_gate_check.py`／`run_conformance_skeleton.py`→E0台帳/Gate。 |
| 契約／Data | Commit/blob SHA、Manifest hash、Registry status、worktree/dirty、E0-04参照。 |
| 正常時作用 | HEAD・復旧ref・Manifest/Gate整合を調べ記録する。 |
| 否定時作用 | hash不一致・欠測・dirtyはUNKNOWN/FAILで残し旧C0を現行PASSにしない。 |
| 停止／Recovery | 既存差分/稼働中worktreeをreset/cleanせず、衝突時は対象だけ保護し復旧refを再照合。 |
| 標準OS候補 | 開発基準Windows。製品保証五OSはN10対象で、この行はOS製品受入れでない。 |
| 配布物 | Git管理source/規定/検査記録。製品artifactは生成しない。 |
| 既存実装 | Git backup、Manifest、Release Gate、Conformance skeleton、旧C0記録。 |
| BUILD_GAP | 旧baseline整合だけではE0/N11全数・OS/artifact結合を満たさない。 |
| ソース識別値（SourceSHA） | `HEAD=ec68d1c00f7546cf4efbbc3ebe17c81543225564`。`release_blockers.registry.json=244adc04a01e7b406c4ebcdeb829f7077d4aa35f`；`tooling/manifest.py=aa4525ff20293cb8641e5598a2439561a3bfbd3b`；`tooling/release_gate_check.py=22a8d7bfe9adb259f44ce8501fb215652e1d58c6`；`tooling/conformance_tests/run_conformance_skeleton.py=c6f31d8cb561b48513c6784e54c85969c9cb953e` |
| 試験区分（TestClass） | MANIFEST/CONFORMANCE/RELEASE_GATE既存source。製品test `NOT_RUN`。 |
| 証拠ID（Evidence ID） | `NONE`。旧C0 log/hashは履歴。 |
| 阻害要因（Blocker） | Registry 18件（16 unresolved/2 resolved）、release_ready=false維持。新規blockerなし。 |
| 状態 | Phase 0 mapping記録。C0 status不変、E0 `OPEN`。 |
| 残余 | 全要求・consumer・候補同一性・N11証拠結合が残る。 |

##### Phase 1 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 製品境界。 |
| 旧ID | `C31`文書整備との歴史的関係を維持。製品境界はD4追加範囲。 |
| 原文参照 | `docs/D4_POCKET_PHASE_MAPPING.md` Phase 1/C31、v4 S00/S10、R02/R03、I01/I03/I12、採用record。 |
| 対象 | D4 Pocket製品、GUI-Shell技術基盤、Runtime/Agent/Tool制御面、現行説明文書。 |
| 目的 | 製品と技術の責任を分け、ブランド変更を技術契約renameに波及させない。 |
| 生成元（Producer） | owner採用v4三点、AGENTS、README/ROADMAP、製品文書。 |
| 実Consumer | `AGENTS.md`／`README.md`／`ROADMAP.md`／v4三点→開発者/利用者説明。 |
| 契約／Data | v4 S00/S10、採用hash、Repository identity、製品名/基盤名の役割。 |
| 正常時作用 | D4 Pocketを製品、GUI-Shellを基盤として説明し要求と未成立を維持。 |
| 否定時作用 | Repository/public schema/API/save pathをrenameしない。旧版や会話を現正本にしない。 |
| 停止／Recovery | 正本衝突時は採用record/責任正本へ戻り、C31履歴を遡及変更しない。 |
| 標準OS候補 | 五OS共通境界。version/archはN10母集団を参照し文書をOS合格にしない。 |
| 配布物 | 三点・採用record・Git文書。installed packageは別Acceptance。 |
| 既存実装 | v4三点/hash、AGENTS、README、ROADMAPが存在。 |
| BUILD_GAP | 文書定義だけではTask/配布/権限/Audit/Recoveryの実consumerを成立させない。 |
| ソース識別値（SourceSHA） | `HEAD=ec68d1c00f7546cf4efbbc3ebe17c81543225564`。`docs/QE_2026-11-13/01_D4Pocket_統合製品実装仕様書_v4.md=352c53180cbad07888beb956aad0fb2a58074c58`；`docs/QE_2026-11-13/02_D4Pocket_統合開発工程表_v4.md=d32dd251e5f7cbb97d92f6e7540d6b6ed0ecec05`；`docs/QE_2026-11-13/03_D4Pocket_Codex施工指示書_v4.md=49447973e88c50a09b2f4978adfed84fd464c6d0`；`docs/QE_2026-11-13/採用記録.md=f159c1aa6ea5bc6e80e0877d8068f7e44da4c4d3`；`README.md=38664991fe614acfc516eb6743dde6dce754191c`；`ROADMAP.md=6a922a38cca617690bc08297274e907f0425b722`；`AGENTS.md=1eafcadd129fd01f1f2f683594c381605ed7d48d` |
| 試験区分（TestClass） | DOCUMENTATION/SOURCEのみ。Rust/Flutter/installed test `NOT_RUN`。 |
| 証拠ID（Evidence ID） | `NONE`。旧C31 statusは履歴。 |
| 阻害要因（Blocker） | 既存18 blocker維持。専用blocker追加なし。 |
| 状態 | Phase 1 mapping記録。製品acceptance未成立、E0 `OPEN`。 |
| 残余 | OS別実配線・package・権限/Audit/Recovery検査は後続へ。 |

##### Phase 2 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 接続先能力の観測。 |
| 旧ID | `C17`複数Host登録履歴。metadataをTrust/権限へ拡張しない。 |
| 原文参照 | `docs/D4_POCKET_PHASE_MAPPING.md` Phase 2/C17、S04 N03/N04・S05 K01/K06/K11、R03線B/C。 |
| 対象 | Runtime Catalog、Host metadata登録、Runtime/Host Centerの限定表示。 |
| 目的 | 接続先識別と宣言能力を観測しmetadataからTrust/実行許可を作らない。 |
| 生成元（Producer） | Agent/Runtime/Host登録要求とBroker registry。 |
| 実Consumer | `runtime_center.dart`／`host_operation_center.dart`→`host_center.rs`／Runtime registry→bounded表示。 |
| 契約／Data | `host_registration.schema.json`／`host_list.schema.json`、識別子/公開metadata。 |
| 正常時作用 | 妥当なmetadataを登録・一覧・表示し能力を未信頼宣言として扱う。 |
| 否定時作用 | malformed/unknown/duplicate/scope違反を拒否。metadataからTrust/Permission/Approval/remote Taskを生成しない。 |
| 停止／Recovery | Broker/Audit/応答異常は停止しunknown表示。古い登録からTaskを実行しない。 |
| 標準OS候補 | Desktop候補Windows/macOS/Linux。Mobileはread-only projection。版/archはN10正本。 |
| 配布物 | Desktop package候補。UI/sourceはinstalled Host接続証拠でない。 |
| 既存実装 | `runtime_center.dart`、`host_operation_center.dart`、`host_center.rs`、`dialogue.rs`、`Host`用`Schema`群。 |
| BUILD_GAP | Host registryはmetadataのみ。remote Task/Trustと二つ目のTask Runtimeは未成立。 |
| ソース識別値（SourceSHA） | `HEAD=ec68d1c00f7546cf4efbbc3ebe17c81543225564`。`apps/desktop_flutter/lib/screens/runtime_center.dart=d2c7af172073d2b10c76a6416b90f011f10e95db`；`apps/desktop_flutter/lib/screens/host_operation_center.dart=e16605e6bb39bbfc12806e6154e08d6fbab1b93b`；`native/rust_helper/src/broker/host_center.rs=c27707faa61e5270b1c6d79e3ee3c5fe6ccd0bf8`；`native/rust_helper/src/broker/dialogue.rs=c3c3fddb630dd06b7e6cc1c13b39f3d4f6f6a999`；`specs/host_registration.schema.json=8bed26fb2fe8f8e296c1d3404978f3215d571240`；`specs/host_list.schema.json=93880c43b04a5408c033bf351ee695073e54664a` |
| 試験区分（TestClass） | N03/N04 UNIT/FIXTURE source。mapping中product test `NOT_RUN`。 |
| 証拠ID（Evidence ID） | `NONE`。C17/fake記録は限定履歴。 |
| 阻害要因（Blocker） | N03/N04/W03・Registry既存status維持。新blockerなし。 |
| 状態 | Phase 2記録。N03/N04 product acceptance `OPEN`。 |
| 残余 | 実Host接続、Trust、第二Runtime、OS installed証拠。 |

##### Phase 3 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | Agent接続契約。D4追加、旧C同等IDなし。 |
| 旧ID | 旧C直接対応なし。N03/N01 statusへ接続。 |
| 原文参照 | `docs/D4_POCKET_PHASE_MAPPING.md` Phase 3、S04 N01/N03・S05 K01/K06、R03線A/B、I04-A/B。 |
| 対象 | `Agent Runtime`識別子、`Task`能力契約、`Adapter`／`Broker`境界。 |
| 目的 | 実interface/対応能力/unsupportedを固定しmetadataを権限に変えず共通Taskへ接続。 |
| 生成元（Producer） | 利用者選択Runtime binding、interface probe、Task要求。 |
| 実Consumer | `agent_runtime.schema.json`→`adapters/mod.rs`→Broker `dialogue.rs` capability検査→Adapter。 |
| 契約／Data | `agent_runtime.schema.json`／`agent_task_request.schema.json`／`agent_task.schema.json`。 |
| 正常時作用 | 対応Runtime Task要求をBroker境界へ渡しunsupportedを拒否。 |
| 否定時作用 | unknown Runtime、malformed contract、metadata escalation、history/UI stateからApproval再利用を拒否。 |
| 停止／Recovery | Broker capability/Permission/Approvalが失敗/不明なら開始せず権限をRuntime間で移さない。 |
| 標準OS候補 | Desktop候補。N10 Windows/macOS/Linux参照。Mobile Task consumerはBUILD_GAP。 |
| 配布物 | Desktop/Runtime候補。Codex CLI sourceはあるが一般Provider配布保証なし。 |
| 既存実装 | `adapters/mod.rs`、`codex_cli.rs`、`dialogue.rs`、`Task` Schema群、`E2E`ソース。 |
| BUILD_GAP | Task AdapterはCodex CLI。第二Runtime/一般Provider実作用/五OS証拠なし。MINIDORAをTask対応扱いしない。 |
| ソース識別値（SourceSHA） | `HEAD=ec68d1c00f7546cf4efbbc3ebe17c81543225564`。`specs/agent_runtime.schema.json=b8e031e53f582048211f0e1a6deeb2e5cb87e4fb`；`native/rust_helper/src/adapters/mod.rs=06c540a59437b903b3f045b5aafb3790c21103bd`；`native/rust_helper/src/adapters/codex_cli.rs=e5c9d1581ae9a1c0a3ae7fcfa626fdca6fea6d28`；`native/rust_helper/src/broker/dialogue.rs=c3c3fddb630dd06b7e6cc1c13b39f3d4f6f6a999`；`native/rust_helper/tests/agent_task_production_e2e.rs=fdc7565aa58fa7466e6e24e5492321e8015567eb` |
| 試験区分（TestClass） | UNIT/FIXTUREとproduction E2E source。第二Runtime/installed test `NOT_RUN`。 |
| 証拠ID（Evidence ID） | `NONE`。W01は限定履歴。 |
| 阻害要因（Blocker） | N03/W03/G3・Registry既存OPEN維持。blocker追加なし。 |
| 状態 | `Phase 3`記録。`N01`/`N03`の受入れと`E0`は`OPEN`。 |
| 残余 | 独立第二Runtime、Task/Result/拒否、OS installed証拠。 |

##### Phase 4 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | Agent起動器。D4追加、旧C同等IDなし。 |
| 旧ID | 旧C直接対応なし。Phase 3契約を使う起動範囲。 |
| 原文参照 | `docs/D4_POCKET_PHASE_MAPPING.md` Phase 4、S04 N01/N03・S05 K01/K06、R03線A/B、I04-A/B/I05。 |
| 対象 | Broker認可TaskからAdapter/CLI childを起動しprocess/result/scratchを回収。 |
| 目的 | 確認済interfaceのみ起動しBroker Task/result/stop/Recoveryを保つ。 |
| 生成元（Producer） | Broker認可Task、Runtime登録、対応CLI path/config。 |
| 実Consumer | `dialogue.rs`→`codex_cli.rs`→`Codex`子プロセス／作業用領域の監督→`Broker`状態／結果。 |
| 契約／Data | Runtime/Task schema、Adapter起動設定、process/scratch identity。CredentialをUI/Auditへ返さない。 |
| 正常時作用 | Codex CLI AdapterにTask起動・結果返却sourceがある。 |
| 否定時作用 | unsupported Runtime/version、path/config不一致、stale Taskを起動しない。output/metadataから権限を作らない。 |
| 停止／Recovery | cancel/deadline/crashはBroker stop/scratch Recoveryへ。結果不明をcompleted/retryにしない。 |
| 標準OS候補 | Desktop Windows/macOS/Linux候補をN10で管理。Mobile launcherでない。 |
| 配布物 | Desktop/Runtime候補。通常Installerと依存配布はN10未成立。 |
| 既存実装 | `codex_cli.rs`、`agent_task_scratch.rs`、`dialogue.rs`、`Task`／`crash`用`E2E`ソース。 |
| BUILD_GAP | CLI sourceと限定Windows履歴あり。第二Runtime、全OS supervision、同一配布は未成立。 |
| ソース識別値（SourceSHA） | `HEAD=ec68d1c00f7546cf4efbbc3ebe17c81543225564`。`native/rust_helper/src/adapters/codex_cli.rs=e5c9d1581ae9a1c0a3ae7fcfa626fdca6fea6d28`；`native/rust_helper/src/broker/agent_task_scratch.rs=de743cd819fb4533c9c3070c4e7bcae82e42d938`；`native/rust_helper/src/broker/dialogue.rs=c3c3fddb630dd06b7e6cc1c13b39f3d4f6f6a999`；`native/rust_helper/tests/agent_task_production_e2e.rs=fdc7565aa58fa7466e6e24e5492321e8015567eb`；`native/rust_helper/tests/agent_task_crash_recovery_e2e.rs=23517fcfe8ef94b2357a94e04879d423c3360501` |
| 試験区分（TestClass） | UNIT/FIXTURE/E2E source。mappingでprocess/installed test `NOT_RUN`。 |
| 証拠ID（Evidence ID） | `NONE`。W01/N01履歴限定。 |
| 阻害要因（Blocker） | N01/N03/Registry status維持。promotion/blocker追加なし。 |
| 状態 | `Phase 4`対応記録。`Runtime`受入れは`OPEN`。 |
| 残余 | 異種Runtime/OS installed launch、failure/Recovery、package identity。 |

##### Phase 5 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | Agent操作表示盤。旧C1範囲との関係。 |
| 旧ID | `C1` Agent Center/Workspace表示履歴。W01はWindows normal Release/Codex CLI限定。 |
| 原文参照 | `docs/D4_POCKET_PHASE_MAPPING.md` Phase 5/C1、S01/S04 N01/N09・S05 K01/K13、R03線A/C、I04-A/C。 |
| 対象 | Desktop Agent CenterのAgent/Task操作・状態/result表示。Workspace Inspectorとは別。 |
| 目的 | Broker由来の許可状態を表示しUIにAuthorityを持たせない。 |
| 生成元（Producer） | Task操作、Broker receipt/state/result、Runtime/Session/Workspace。 |
| 実Consumer | `agent_center.dart`→`agent_task_client.dart`→`Broker`の`dialogue.rs`→`Task`表示。 |
| 契約／Data | `agent_task_request.schema.json`、`Task`／`runtime`／`session`／`workspace`の束縛、公開内容の上限。 |
| 正常時作用 | Task requestをBrokerへ渡しTask IDに束縛された許可状態を投影。 |
| 否定時作用 | stale/unknown ID、scope不一致、malformed result、secret/未許可内容を拒否。履歴をApprovalにしない。 |
| 停止／Recovery | cancel receiptと実停止を区別。Audit/result不明はunknown、再起動で旧Authorityを復活しない。 |
| 標準OS候補 | Desktop N10候補。W01はWindows/Codex限定LIVE_PRODUCT履歴。Mobileはread-onlyでTask開始なし。 |
| 配布物 | Desktop candidate。各OS packageはN10別Acceptance。 |
| 既存実装 | `agent_center.dart`、`agent_task_client.dart`、`dialogue.rs`、`Task` Schema、`Flutter`試験。 |
| BUILD_GAP | 全OS installedのTask/result/diff/stop/recovery/Audit一体証拠なし。Mobile TaskはBUILD_GAP。 |
| ソース識別値（SourceSHA） | `HEAD=ec68d1c00f7546cf4efbbc3ebe17c81543225564`。`apps/desktop_flutter/lib/screens/agent_center.dart=2472faeb31a64d624423c30a0a30d0f153d2e2a3`；`packages/agent_orchestration/lib/agent_task_client.dart=a31fbb9f980a0039f809fd763c91f055fe9d02d0`；`native/rust_helper/src/broker/dialogue.rs=c3c3fddb630dd06b7e6cc1c13b39f3d4f6f6a999`；`specs/agent_task_request.schema.json=cb25bb3684656bfd670ea5832bbb7fc5988845b6`；`apps/desktop_flutter/test/agent_task_client_test.dart=0477a0bbbb321082494b6b55711815d34419fdd9` |
| 試験区分（TestClass） | Flutter UNIT/WIDGET、Rust UNIT/FIXTURE source。W01は過去限定。product test `NOT_RUN`。 |
| 証拠ID（Evidence ID） | `NONE`。W01を現候補へ転記しない。 |
| 阻害要因（Blocker） | N01/N09/W01/Registry status維持。再開・blocker追加なし。 |
| 状態 | `Phase 5`記録。`N01`/`N09`の受入れは`OPEN`。 |
| 残余 | 他OS installed Task/UI、same Task result/diff/Audit/Recovery、Mobile境界。 |

Phase 0〜14の15行はE0-04全項目を記録した。旧Phase/C/P status、N01/N02/N03/N04/N09/N11のOPEN、W01限定CLOSED、W03/G3、Release Blockerは変更していない。Phase 15〜45、K01〜K20の個別fieldとN11 Evidence/Blocker/OS Artifact行結合は未完了。E0はOPEN。
#### Phase 6〜8 E0-04個別記録（2026-11-13）

旧Phase/Cの履歴状態を保持し、現行v4要求consumerへE0-04全21項目で接続する。SourceSHAは編集前main 3bd056d65cb5e1d8450c4995fd240964dfb567edのGit blob値で、製品Acceptanceの証拠ではない。本mapping単位で製品testは再実行しない。

##### Phase 6 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | Phase 6 作業領域検査。現行consumerはN09、施工線C、能力K03。 |
| 旧ID | Phase 6／C1 Workspace Inspector。旧statusと過去の差分・履歴は当時の記録として維持し、再判定しない。 |
| 原文参照 | docs/D4_POCKET_PHASE_MAPPING.md Phase 6/C1、v4仕様S04 N09・S05 K03、工程表R03線C、施工指示I04-C。 |
| 対象 | Desktop Workspace Inspectorのtree、bounded read、change list、diff、history、Recovery preview。 |
| 目的 | Broker境界内でWorkspaceを確認し、UI表示を権限・復旧作用と混同させない。 |
| 生成元（Producer） | Broker管理のWorkspace登録/root、許可されたread/change/history、比較基準、Recovery候補。 |
| 実Consumer | `workspace_inspector.dart→workspace_client.dart→Broker workspace operations→workspace_root.rs／workspace_reader.rs／workspace_diff.rs。結果はInspector/Diff panelへ戻る。` |
| 契約／Data | workspace_inspection_request.schema.json、workspace_inspection_response.schema.json、workspace_diff.schema.json。Workspace ID・登録root・要求範囲・bounded resultを束縛。 |
| 正常時作用 | 現在の登録・許可を検査しtree/read/change list/history/diff/Recovery previewを返す。baseline diffとTask result diffは別data。 |
| 否定時作用 | 未登録/stale Workspace、root外path、secret、malformed request、上限超過を拒否し、不明な結果を成功表示しない。 |
| 停止／Recovery | 登録・権限・Audit不整合は拒否またはsuspend。Recovery previewは候補表示のみでrollback実行ではない。 |
| 標準OS候補 | Desktop N10候補のWindows/Linux/macOS。Mobile Inspector consumerは未接続。各OSのinstalled acceptanceは未成立。 |
| 配布物 | Desktop候補packageはN10管理。source/debug buildを通常配布・installed受入れへ昇格しない。 |
| 既存実装 | `apps/desktop_flutter/lib/screens/workspace_inspector.dart、workspace_diff_panel.dart、services/workspace_client.dart、native/rust_helper/src/broker/workspace.rs、workspace_root.rs、workspace_reader.rs、workspace_diff.rs。` |
| BUILD_GAP | Task result/diff/testをTask単位で示すconsumer、全Workspace・大容量file、Mobile parity、五OS installed一体証拠は未成立。 |
| ソース識別値（SourceSHA） | `HEAD=3bd056d65cb5e1d8450c4995fd240964dfb567ed。docs/D4_POCKET_PHASE_MAPPING.md=064ea334f86b137d94fba2370d2a2fd732f6e972; apps/desktop_flutter/lib/screens/workspace_inspector.dart=a02c95ec5062583f9c37523fbaf604a8d2da107e; apps/desktop_flutter/lib/screens/workspace_diff_panel.dart=5e2154b870921a707b3da9a79c9f55055ce3cb1f; apps/desktop_flutter/lib/services/workspace_client.dart=870b48e8f4bbd648bcfebc5886addca3784bcfaf; native/rust_helper/src/broker/workspace.rs=68075d680a3c7d57785083e64d3d21523d79374c; native/rust_helper/src/broker/workspace_root.rs=15f208e0b855b8cee53e878d2b27aaf457ea2937; native/rust_helper/src/workspace_reader.rs=3d635b50f85cffb6db382ed3c061ff6068238099; native/rust_helper/src/workspace_diff.rs=990041d9f2a67e2d372371e74fafb225e4e78e79; specs/workspace_inspection_request.schema.json=a8210c300b2909c040188d0766633f1e30884e7a; specs/workspace_inspection_response.schema.json=6cc9f80f4dd9b8bd9f71441bee845017428fa38a; specs/workspace_diff.schema.json=1389ad85c48e263e688ab6dfa5276f96eb26fef0; apps/desktop_flutter/test/workspace_inspector_test.dart=2f6e53c2a38d5e23c88f2ad91e079bf8be4808a5; native/rust_helper/tests/workspace_reader.rs=4723cbb121a0fe50d2e73ad6592ca9eac6c42a39; native/rust_helper/tests/workspace_diff.rs=a467a45c233112c6f81ab8ffe504781e53aa937c; native/rust_helper/tests/workspace_startup.rs=e497c2747d58fba4d5d31c9fe9d7fc6a4152510a.` |
| 試験区分（TestClass） | Flutter UNIT/WIDGET、Rust UNIT/FIXTURE sourceあり。source/installed product testは本mapping単位でNOT_RUN。 |
| 証拠ID（Evidence ID） | NONE。Source mappingはN09 installed acceptance、Task result projection、OS別証拠を証明しない。 |
| 阻害要因（Blocker） | N09/W00等とRegistry 18件（16 unresolved／2 resolved）、release_ready=falseを維持。新規Blockerなし。直接帰属はN11照合までUNKNOWN。 |
| 状態 | Phase 6のE0-04 mapping 21項目を記録。旧Phase/C status不変、N09製品AcceptanceとE0はOPEN。 |
| 残余 | Task result diffとbaseline diff、Task test表示、bounded Workspace/大容量処理、各OS installed受入れ。 |

##### Phase 7 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | Phase 7 Compare／AgentTask／Session／Workspace結合。現行N01/N02/N04、施工線A/B/C、能力K01/K02/K03/K08。 |
| 旧ID | Phase 7／C1・C30。旧P2対応と旧C/P statusは履歴として保持し、現N04 acceptanceへ移さない。 |
| 原文参照 | docs/D4_POCKET_PHASE_MAPPING.md Phase 7/C1/C30、v4仕様S04 N01/N02/N04・S05 K01/K02/K03/K08、工程表R03線A/B/C、施工指示I04-A/B/C。 |
| 対象 | 同一Taskを独立Session/Workspaceへ束縛したAgent A/Bの実行、結果比較・選択、failure/cancel isolation。 |
| 目的 | 同一Workspace書込みや共有Authorityを避け、AgentごとのTask/result/認可を分離。 |
| 生成元（Producer） | Agent Center Task要求、comparison coordinator、Session/Workspace、Broker Task receipt/result、変更/test情報。 |
| 実Consumer | `agent_center.dart／agent_coordination.dart→agent_comparison_coordinator.dart→agent_task_client.dart／agent_task_execution_coordinator.dart→Broker dialogue/workspace/scratch→Codex CLI adapter。` |
| 契約／Data | agent_comparison.schema.json、agent_task_request.schema.json、agent_task.schema.json、agent_workspace.schema.json、Workspace Permission/Owner Approval/scratch/content exposure contracts。ID・Session・Workspace・result hashを独立束縛。 |
| 正常時作用 | 独立preflightからBroker Taskを別々に開始・照会しbounded result/diff/testを比較。Windows/Linux normal ReleaseのCodex task_execution=supportedは限定Agent Task能力で、Compare acceptanceではない。 |
| 否定時作用 | 同一Workspace、不正Session、stale/unknown Task、権限欠落、Agent metadata escalation、cross-agent contaminationを許可しない。選択結果は未信頼入力。 |
| 停止／Recovery | 片側failure/cancelを他方へ伝播させず、曖昧な開始・停止結果は再送しない。旧Approvalは再利用しない。installed descendant isolationは未実証。 |
| 標準OS候補 | Desktop N10候補のWindows/Linux/macOS。MobileにCompare/Task実作用consumerなし。 |
| 配布物 | Desktop候補packageはN10 scope。Linux nativeを含む各OSのinstalled Compareは未成立。 |
| 既存実装 | `packages/agent_orchestration/lib/agent_comparison_coordinator.dart、apps/desktop_flutter/lib/services/agent_coordination.dart、screens/agent_center.dart、services/agent_task_execution_coordinator.dart、agent_task_client.dart、Rust broker/dialogue.rs、workspace.rs、agent_task_scratch.rs、adapters/codex_cli.rs。` |
| BUILD_GAP | installed同時Codex A/B、実filesystem隔離/cross-agent contamination、selected result projection/apply、failure/cancel isolation、resource/time/result Audit acceptanceは未成立。 |
| ソース識別値（SourceSHA） | `HEAD=3bd056d65cb5e1d8450c4995fd240964dfb567ed。packages/agent_orchestration/lib/agent_comparison_coordinator.dart=6b5deaacb500a23e6bfdc37649d61ebc0d885127; apps/desktop_flutter/lib/services/agent_coordination.dart=bdc13464f0e42b740a2bca3b7e8828a5019056d0; apps/desktop_flutter/lib/screens/agent_center.dart=2472faeb31a64d624423c30a0a30d0f153d2e2a3; packages/agent_orchestration/lib/agent_task_client.dart=a31fbb9f980a0039f809fd763c91f055fe9d02d0; apps/desktop_flutter/lib/services/agent_task_execution_coordinator.dart=5d8f48d0557bdcb1af838239abf9651893bce7f1; native/rust_helper/src/broker/dialogue.rs=c3c3fddb630dd06b7e6cc1c13b39f3d4f6f6a999; native/rust_helper/src/broker/workspace.rs=68075d680a3c7d57785083e64d3d21523d79374c; native/rust_helper/src/broker/agent_task_scratch.rs=de743cd819fb4533c9c3070c4e7bcae82e42d938; native/rust_helper/src/adapters/codex_cli.rs=e5c9d1581ae9a1c0a3ae7fcfa626fdca6fea6d28; specs/agent_comparison.schema.json=d2993ce1145674214fb2d80d696fc2c161cfbffe; specs/agent_task_request.schema.json=cb25bb3684656bfd670ea5832bbb7fc5988845b6; specs/agent_task.schema.json=a552389072088d6cd42b3588fefda332be4ae6b6; specs/agent_workspace.schema.json=47058678398b5aac1b0b1467cac7e78daef72b32; specs/agent_task_workspace_permission_request.schema.json=05f7a7f97c7bac884ce706cc7f1533792c35e486; specs/agent_task_workspace_permission.schema.json=01be05df95431e38ca5dc3ec1b5f04416b506fb2; specs/agent_task_owner_approval.schema.json=d190c9a8cc3d715eb199459c37c14e5de0ad6ad8; specs/agent_task_scratch_recovery.schema.json=40f1eac71fb4533c9c3070c4e7bcae82e42d938; specs/agent_task_content_exposure.schema.json=a85b99c0213d18daaab3d5d863adcd3661f379a0; apps/desktop_flutter/test/agent_coordination_test.dart=6783e87a492d8436a855a7088b7cece38d122bd5; apps/desktop_flutter/test/agent_task_client_test.dart=0477a0bbbb321082494b6b55711815d34419fdd9; native/rust_helper/tests/agent_task_production_e2e.rs=fdc7565aa58fa7466e6e24e5492321e8015567eb; native/rust_helper/tests/agent_task_crash_recovery_e2e.rs=23517fcfe8ef94b2357a94e04879d423c3360501.` |
| 試験区分（TestClass） | Flutter UNIT/WIDGET、Rust UNIT/FIXTURE sourceあり。installed Compare、filesystem隔離、failure/cancel、selected result apply/AuditはNOT_RUN。 |
| 証拠ID（Evidence ID） | NONE。fixture/local runtimeや別条件のTask証拠をN04 Compare acceptanceへ転記しない。 |
| 阻害要因（Blocker） | N01/N02/N04、W03/G3、Registry 18件（16 unresolved／2 resolved）、release_ready=falseを維持。新規Blockerなし。直接帰属はN11までUNKNOWN。 |
| 状態 | Phase 7のE0-04 mapping 21項目を記録。旧Phase/C/P status不変、N01/N02/N04・W03/G3・E0 acceptanceはOPEN。 |
| 残余 | installed同時比較、実Workspace/Task隔離、cross-agent contamination、selected result apply、failure/cancel isolation、Audit、各OS consumer。 |

##### Phase 8 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | Phase 8 Agent引継ぎ。現行N04-Handoff、施工線B、能力K02/K11。 |
| 旧ID | Phase 8／C30、旧P3との関係は履歴crosswalkのみ。過去C/P statusを現N04へ移さない。 |
| 原文参照 | docs/D4_POCKET_PHASE_MAPPING.md Phase 8/C30、v4仕様S04 N04・S05 K02/K11、工程表R03線B、施工指示I04-B。 |
| 対象 | 完了Taskの公開result/context/Artifact/diff/test resultを別Agentへbounded handoffし新規Taskとして扱う内部HandoffとA2A Artifact path。 |
| 目的 | 成果物を渡しながらPermission/Approval/Credential/Trust/Authority/hidden stateを継承せず、受信側で現在条件を再評価。 |
| 生成元（Producer） | source AgentTask result/Artifact/diff/test projectionと明示handoff context。A2A Artifactは別のuntrusted source。 |
| 実Consumer | `agent_handoff.dart／a2a_artifact_handoff.dart→Agent Center→agent_task_client.dart fresh preflight→Broker dialogue.rs。A2Aはa2a_center.rs／a2a.rs。` |
| 契約／Data | agent_handoff.schema.json、agent_handoff_package.schema.json、agent_task_request.schema.json、agent_task.schema.json、a2a_contract.schema.jsonおよび限定A2A Message/Task schemas。target Session/Workspace・result/artifact hashを束縛。 |
| 正常時作用 | source resultをbounded package化して別targetのfresh AgentTask preflightへ渡す。Brokerが新規Workspace Permissionと別Approvalを判定し、handoff自体は権限を持たない。 |
| 否定時作用 | 未完了/未承認source、stale hash、target混同、oversized/malformed Artifact、secret、Authority/Approval/Credential/Trust/hidden state継承を拒否。metadata/CardはTrustを生成しない。 |
| 停止／Recovery | target preflight失敗・結果不明は停止し、曖昧なA2A非冪等作用を再送しない。internal receipt projectionをdurable Auditと扱わない。 |
| 標準OS候補 | Desktop N10候補のWindows/Linux/macOS。ただしmacOS A2Aはloopback Card/metadata-onlyで作用なし。Mobile Handoff consumer未接続。 |
| 配布物 | 各OS Desktop候補packageはN10 scope。通常配布・installed Handoff/A2A一体受入れは未成立。 |
| 既存実装 | `apps/desktop_flutter/lib/services/agent_handoff.dart、a2a_artifact_handoff.dart、screens/agent_center.dart、packages/agent_orchestration/lib/agent_task_client.dart、native/rust_helper/src/broker/dialogue.rs、broker/a2a_center.rs、a2a.rs。` |
| BUILD_GAP | installed source→selected public result/Artifact→new target Task→Permission/Approval→result/Audit、Mobile parity、macOS metadataを越えるA2A作用は未成立。 |
| ソース識別値（SourceSHA） | `HEAD=3bd056d65cb5e1d8450c4995fd240964dfb567ed。docs/D4_POCKET_PHASE_MAPPING.md=064ea334f86b137d94fba2370d2a2fd732f6e972; apps/desktop_flutter/lib/services/agent_handoff.dart=a85110d96cbade732ae0e1bfea2529f03f37523c; apps/desktop_flutter/lib/services/a2a_artifact_handoff.dart=e7e2f3454e2843f1d4f4e7b5aedfa55014cde05c; apps/desktop_flutter/lib/screens/agent_center.dart=2472faeb31a64d624423c30a0a30d0f153d2e2a3; packages/agent_orchestration/lib/agent_task_client.dart=a31fbb9f980a0039f809fd763c91f055fe9d02d0; native/rust_helper/src/broker/dialogue.rs=c3c3fddb630dd06b7e6cc1c13b39f3d4f6f6a999; native/rust_helper/src/broker/a2a_center.rs=73ad601abd64a33ccefde4b73ab5f9dc4f728504; native/rust_helper/src/a2a.rs=9d7c6d9ab938ad5178d6ca46b3933dc3e61c6061; specs/agent_handoff.schema.json=e37a276f8c3e19816bc4f80824efce546a116d00; specs/agent_handoff_package.schema.json=a8ae1e2a3be6c0ed5666849fc6553c98083a1cc3; specs/agent_task_request.schema.json=cb25bb3684656bfd670ea5832bbb7fc5988845b6; specs/agent_task.schema.json=a552389072088d6cd42b3588fefda332be4ae6b6; specs/a2a_contract.schema.json=8c7f50bb7fb12dde41293727e5dc9109445d1bc3; specs/a2a_task_get_request.schema.json=df11a5fd3291de0ae379694c6c1b49de51f2f157; specs/a2a_task_get_response.schema.json=56a55f348d6b0ba7cb0092ee7ea2b4ec809e5360; specs/a2a_message_send_request.schema.json=0a5f87250f17a88b6be55e7838d32b8e9114bbed; apps/desktop_flutter/test/agent_handoff_test.dart=c6bc9bef8cc8d153ef59bf421efbd989185f8e0c; apps/desktop_flutter/test/a2a_artifact_handoff_test.dart=e47005320917f59dfa55ae1e04c4b99d452676de.` |
| 試験区分（TestClass） | Flutter UNIT/WIDGET、Rust UNIT/FIXTURE sourceあり。installed target reauthorization、A2A external action、durable AuditはNOT_RUN。 |
| 証拠ID（Evidence ID） | NONE。fixture/loopback Card metadata evidenceはinstalled external Handoff acceptanceを証明しない。 |
| 阻害要因（Blocker） | N04/W03/G3、Registry 18件（16 unresolved／2 resolved）、release_ready=falseを維持。新規Blockerなし。 |
| 状態 | Phase 8のE0-04 mapping 21項目を記録。旧Phase/C/P status不変、N04・W03/G3・E0 acceptanceはOPEN。 |
| 残余 | installed target reauthorization→result/Audit、selected Artifact boundary、Mobile consumer、macOS A2A作用範囲。 |

Phase 6〜8の3行は各E0-04全21項目を記録。旧Phase/C/P status、N01/N02/N04/N09/N11、W03/G3、Release Blockerは変更せず、Evidence IDは全てNONE。続くPhase 9〜14の記録後も、Phase 15〜45、K01〜K20個別field、N11 Evidence/Blocker/OS Artifact行結合は未完了で、E0はOPEN。

#### Phase 9〜11 E0-04個別記録（2026-11-13）

旧Phase/Cの履歴状態を維持し、現行v4の要求consumerへE0-04全21項目で対応付ける。SourceSHAは編集前main 5ce80af0ce23b87269d91d817aa4ec4ff71c43a6のGit blob値であり、製品Acceptanceの証拠ではない。製品testの再実行は本mapping単位の範囲外でNOT_RUNとする。

##### Phase 9 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | Phase 9 実行履歴。現行consumerはN09、施工線C、能力K03/K10。 |
| 旧ID | Phase 9/C2 History。過去の局所status・証拠は当時の条件に保持し、再判定しない。 |
| 原文参照 | docs/D4_POCKET_PHASE_MAPPING.md Phase 9/C2、v4仕様書S04 N09・S05 K03/K10、工程表R03線C、施工指示書I04-C。 |
| 対象 | AgentTask履歴metadataとRuntime Dialogue履歴・bounded contentの参照、履歴画面の一覧・詳細投影。 |
| 目的 | 現在許可された範囲の履歴を表示し、記録や履歴本文をApproval・Permission・現在の実行状態として扱わない。 |
| 生成元（Producer） | BrokerのAgentTask execution record、Dialogue record、現在のHistory閲覧grant。 |
| 実Consumer | Desktop History screen/content dialog→共有HistoryClient→Broker history_access/execution_history。AgentTask履歴metadataとDialogue履歴・内容閲覧は異なるoperationとして扱う。 |
| 契約／Data | runtime_history_access、runtime_execution_history/page、runtime_agent_task_history_request/access。record identity、Runtime、grant、期限、ページ上限をBroker側で検査する。 |
| 正常時作用 | 有効grantの範囲でbounded pageを表示する。AgentTaskはmetadata projection、Dialogue本文は別の現在grantへ一致した場合にだけ期限付きで表示する。 |
| 否定時作用 | 不正・期限切れgrant、異なるRuntime/session、改変・不整合record、範囲外要求を拒否する。過去の履歴・UI stateは現在のAuthorityを生成しない。 |
| 停止／Recovery | grant失効・期限超過・identity/Audit不一致では内容取得と表示を停止する。AgentTaskのcrash recovery、再実行、復旧操作が履歴画面に統合済みとは扱わない。 |
| 標準OS候補 | Windows/macOS/Linux native Desktop。Mobileは限定metadata projectionで、Desktopの履歴内容閲覧との同等性は未確認。Linux nativeは必須対象、installed受入れはN10に従う。 |
| 配布物 | Phase 9固有artifactなし。各OS通常installed packageはN10の候補・受入れ範囲に従う。 |
| 既存実装 | apps/desktop_flutter/lib/screens/history_screen.dart、history_content_dialog.dart、packages/gui_shell_ui/lib/src/history_client.dart、Rust Broker history_access.rs/execution_history.rs/protocol.rs、History/ExecutionHistory/AgentTaskHistory schema群。 |
| BUILD_GAP | AgentTask履歴のmetadata表示はあるが、Task単位のdurable Audit検索・Recovery・result/diff/test projectionとの一体consumerは未成立。Dialogue本文の閲覧範囲は現在grantに限定され、Task result本文の表示と混同しない。 |
| ソース識別値（SourceSHA） | 照合HEAD=5ce80af0ce23b87269d91d817aa4ec4ff71c43a6。apps/desktop_flutter/lib/screens/history_screen.dart=dacc4c0ca15325c7f597cd1827a6455fb8ab5560; apps/desktop_flutter/lib/screens/history_content_dialog.dart=4602a29edc7a0c7c2b90e5b3b8a55155a28cfdb6; packages/gui_shell_ui/lib/src/history_client.dart=7f14ec2caf8b1e383bbf93483ae052100f921ae7; native/rust_helper/src/broker/history_access.rs=c004056b45c2f26859bf7f70a9ff2b632e358523; native/rust_helper/src/broker/execution_history.rs=8eeb1c26d06a9e4b01806413a4bfa8c2a73357d6; native/rust_helper/src/broker/protocol.rs=9b2076d5f31ca6d77f1f87973d1b394aaf751961; specs/runtime_history_access.schema.json=8187ddce917c3e80f40c8ce1c4b89059277f1cfd; specs/runtime_execution_history.schema.json=5b260e9b6026758dc0a730d3381b4f7d72ba2e98; specs/runtime_execution_history_page.schema.json=e6dab615707d30c9266df7dc67fc90df24d0ed9b; specs/runtime_agent_task_history_request.schema.json=0bff21625a0f339367ae8352910b643b1f76b361; specs/runtime_agent_task_history_access.schema.json=baddfeb38762fe0a04513cb1edcfcd258bd804ed; apps/desktop_flutter/test/history_screen_test.dart=7828dc17fbf50cd0a2dca3255d90c3760030e58f; apps/desktop_flutter/test/history_content_test.dart=af34c1e4e61415916fe651a3f5096a95f946022b. |
| 試験区分（TestClass） | Flutter UNIT/WIDGET、Rust UNIT/FIXTUREのsourceあり。本mapping単位ではこれらのtestを実行していない（NOT_RUN）。 |
| 証拠ID（Evidence ID） | NONE。Source mappingと既存test sourceはN09のinstalled AcceptanceやTask単位のdurable履歴証拠ではない。 |
| 阻害要因（Blocker） | N09製品Acceptance OPEN。既存Registry 18件（16 unresolved/2 resolved）、release_ready=falseを維持し、新規Blockerを追加しない。直接帰属はN11照合までUNKNOWN。 |
| 状態 | Phase 9のE0-04 mapping 21項目を記録。旧Phase/C status不変、N09製品AcceptanceとE0はOPEN。 |
| 残余 | Task単位のdurable履歴・Audit・Recovery検索と、履歴由来のTask操作を現在の認可へ接続するconsumer、五OS installed証拠。 |

##### Phase 10 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | Phase 10 再実行・分岐。現行consumerはN09、施工線C、能力K03/K05。 |
| 旧ID | Phase 10/C2 replay・branch。過去Dialogueの再実行／分岐とAgentTaskの再実行を同一能力へ読み替えない。 |
| 原文参照 | docs/D4_POCKET_PHASE_MAPPING.md Phase 10/C2、v4仕様書S04 N09・S05 K03/K05、工程表R03線C、施工指示書I04-C。 |
| 対象 | History selectionからのRuntime Dialogue replay/branch要求と、AgentTask履歴から将来新しいTaskを作成する境界。 |
| 目的 | 再実行・分岐を新規要求として識別・監査し、履歴、過去Permission/Approval、Session stateからAuthorityを再利用しない。 |
| 生成元（Producer） | 利用者の明示的なDialogue履歴選択、新規入力・条件、Brokerの履歴recordと現在のSession/Runtime binding。 |
| 実Consumer | History screen→HistoryClient→Broker dialogue operationでDialogue replay/branchを要求する。AgentTask履歴metadataは別operationであり、履歴からAgentTaskを再実行する一体consumerは未成立。 |
| 契約／Data | runtime_dialogue_operation/request/response/comparisonとruntime_agent_task_history_request/access。参照Audit ID/hashを保持し、新しい要求・Session・Runtime条件をBrokerで検査する。 |
| 正常時作用 | Dialogue replay/branchは履歴を参照して新しい入力・requestとして処理する。AgentTaskの履歴選択を現在条件でpreflightし新Permission/Approvalを得る製品経路はBUILD_GAP。 |
| 否定時作用 | 改変・stale history、異なるSession/Runtime、期限切れgrant、旧Permission/Approvalの再利用を拒否する。履歴選択だけで実行・評価・許可を開始しない。 |
| 停止／Recovery | reference identity/Audit不一致または不明な要求結果では停止し、旧承認を使った再送をしない。Dialogue replay/branchはAgentTaskのrestart/recoveryではない。 |
| 標準OS候補 | Windows/macOS/Linux native Desktop。Mobile履歴projectionは限定metadataであり、Desktop Dialogue replay/branchまたはAgentTask再実行の製品同等性は未確認。 |
| 配布物 | Phase 10固有artifactなし。通常installed Desktop packageとOS別QAはN10の範囲に従う。 |
| 既存実装 | history_screen.dart、共有history_client.dart、Broker history_access.rs/execution_history.rs/dialogue.rs/protocol.rs、Dialogue operation/request/response/comparisonとAgentTask History schema群。 |
| BUILD_GAP | AgentTask履歴から新Session/WorkspaceのTaskを作り、現行Workspace Permissionと独立Owner Approvalを再評価したうえで結果/Audit/Recoveryへ結ぶconsumerは未成立。Dialogue replay/branchは独立した会話要求である。 |
| ソース識別値（SourceSHA） | 照合HEAD=5ce80af0ce23b87269d91d817aa4ec4ff71c43a6。apps/desktop_flutter/lib/screens/history_screen.dart=dacc4c0ca15325c7f597cd1827a6455fb8ab5560; packages/gui_shell_ui/lib/src/history_client.dart=7f14ec2caf8b1e383bbf93483ae052100f921ae7; native/rust_helper/src/broker/history_access.rs=c004056b45c2f26859bf7f70a9ff2b632e358523; native/rust_helper/src/broker/execution_history.rs=8eeb1c26d06a9e4b01806413a4bfa8c2a73357d6; native/rust_helper/src/broker/dialogue.rs=c3c3fddb630dd06b7e6cc1c13b39f3d4f6f6a999; native/rust_helper/src/broker/protocol.rs=9b2076d5f31ca6d77f1f87973d1b394aaf751961; specs/runtime_dialogue_operation.schema.json=8793ab0f59ea2320b7102afec7256cfe62264fc5; specs/runtime_dialogue_request.schema.json=fe0bd153eaf3a0a2354b25cffc3ed9af934fbae1; specs/runtime_dialogue_response.schema.json=5e7220e8ce5954057ee270cd77cb4a68e1ef830f; specs/runtime_dialogue_comparison.schema.json=874dd1158a6aa911ff30d85e141b4bd4927c91e3; specs/runtime_agent_task_history_request.schema.json=0bff21625a0f339367ae8352910b643b1f76b361; specs/runtime_agent_task_history_access.schema.json=baddfeb38762fe0a04513cb1edcfcd258bd804ed; apps/desktop_flutter/test/history_screen_test.dart=7828dc17fbf50cd0a2dca3255d90c3760030e58f; apps/desktop_flutter/test/runtime_dialogue_test.dart=899912f92ee3751f8616632d434e04f95ee5ef77; packages/gui_shell_ui/test/runtime_dialogue_test.dart=f997ac25c0d8227e2898dc6d1351d324911324e4. |
| 試験区分（TestClass） | Flutter UNIT/WIDGET、Rust UNIT/FIXTURE sourceあり。本mapping単位では履歴外部作用・Task再実行testを実行していない（NOT_RUN）。 |
| 証拠ID（Evidence ID） | NONE。Dialogue fixture/testやsource mappingをAgentTask再実行のinstalled Acceptanceへ転用しない。 |
| 阻害要因（Blocker） | N09製品Acceptance OPEN。既存Registryの状態を維持し、新規Blockerを追加しない。直接帰属はN11照合までUNKNOWN。 |
| 状態 | Phase 10のE0-04 mapping 21項目を記録。旧Phase/C status不変、N09製品AcceptanceとE0はOPEN。 |
| 残余 | AgentTask履歴→現在条件でのfresh Task preflight/Permission/Approval→結果/Audit/Recovery、各OS installed経路。 |

##### Phase 11 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | Phase 11 実行系資源監視。現行consumerはN09、施工線C、能力K04/K10。 |
| 旧ID | Phase 11/C3 Resource monitoring。旧Windows開発接続・Broker observation・Flutter表示と未実施scopeは履歴として保持する。 |
| 原文参照 | docs/D4_POCKET_PHASE_MAPPING.md Phase 11/C3、v4仕様書S04 N09・S05 K04/K10、工程表R03線C、施工指示書I04-C。 |
| 対象 | 登録Runtimeに対するBroker資源観測とDesktop Runtime Center／限定Mobile resource projection。 |
| 目的 | 実測可能な値だけを表示し、取得不能・未対応・失敗をunknown/unavailableとして維持する。Resource telemetryをTask result・Audit・Authorityと混同しない。 |
| 生成元（Producer） | Broker/OS observation source、Runtime registry、測定時刻付きmetricまたは明示的unknown。GPU/VRAM等は取得できる場合だけ値を生成する。 |
| 実Consumer | Runtime Center→Desktop RuntimeResourceClient→共有RuntimeResourceClient→Broker resource query/RuntimeResourceRegistry。Mobile resource overviewはDevice Linkの限定projectionを読む。 |
| 契約／Data | runtime_resource_query、runtime_resource_observation、正規化metric vectorとcanonical decimal vector。Runtime identity・時刻・source/evidence class・値可用性を保持し、metadataからPermission/Approvalを生成しない。 |
| 正常時作用 | 有効なRuntime queryに測定済みmetricとunknown/unavailable metricを分けて返す。Mobileは許可された限定projectionだけを表示する。 |
| 否定時作用 | 未取得値を0で代用しない。invalid Runtime/metric/timestamp、stale source、証拠class不整合、不正Mobile requestは拒否またはunknownへ閉じる。 |
| 停止／Recovery | Source停止・観測不能時は前回値をfreshとして表示せずstale/unknown化する。観測失敗はRuntime processの起動・停止や権限変更を起こさない。 |
| 標準OS候補 | Desktop Windows/macOS/Linux native、限定Mobile Android/iOS。Linux nativeは必須製品対象。各OSのinstalled実測・負荷・長時間QAは未成立。 |
| 配布物 | Phase 11固有artifactなし。OS別通常packageとinstalled計測はN10候補および製品QAに従う。 |
| 既存実装 | Desktop画面の`apps/desktop_flutter/lib/screens/runtime_center.dart`と`RuntimeResourceClient`、Mobile画面の`apps/mobile_flutter/lib/screens/resource_overview.dart`、Rust Brokerの`runtime_registry.rs`／`observation_center.rs`／`device_link.rs`、`runtime_resource_query`／`runtime_resource_observation` schema、正規化vector。 |
| BUILD_GAP | Resource observationはRuntime ID単位でAgentTask ID・Task duration/result/test/failureへ結合されない。五OS installed計測範囲、unknown projection、long-run/負荷性能の製品受入れは未成立。 |
| ソース識別値（SourceSHA） | 照合HEAD=5ce80af0ce23b87269d91d817aa4ec4ff71c43a6。apps/desktop_flutter/lib/screens/runtime_center.dart=d2c7af172073d2b10c76a6416b90f011f10e95db; apps/desktop_flutter/lib/services/runtime_resource_client.dart=d8da9ebee9afdd64b8d1d3afa0a82f3a4cb53cfd; packages/gui_shell_ui/lib/src/runtime_resource_client.dart=3a8e611b94136e27f6d132e550a49bcbaf586eaf; apps/mobile_flutter/lib/screens/resource_overview.dart=322af6bc2935c571017dab3f03b9c7a0cea7c616; native/rust_helper/src/broker/runtime_registry.rs=9ff28610862586a599848d75f9e891a6c6cadf2f; native/rust_helper/src/broker/observation_center.rs=62896e497a0d78267020db6bae2ecadf704fa1fb; native/rust_helper/src/broker/device_link.rs=c2e1ff88ff34a5764b2b4fb1d3f7abd8727e5c51; specs/runtime_resource_query.schema.json=c7617ba9c2283255fb04b6cfa529ff7b04f6137f; specs/runtime_resource_observation.schema.json=62f587b8c88d0c5c0bd316eaa3d0a15c9c1bd2d1; tooling/canonicalization_vectors/runtime_resource_observation_decimal.json=ce00c0ea9571409a707c4e1993069c61761cb6a0; apps/desktop_flutter/test/runtime_resource_test.dart=c44d66bb3797bf523be0f48f24d626068b8e4894; packages/gui_shell_ui/test/runtime_resource_client_test.dart=46c52c83eab712e5b84599c7e29eed660af0ed12; apps/mobile_flutter/test/resource_overview_test.dart=7695155627699274b8c0768c0de302344caa9eec. |
| 試験区分（TestClass） | Desktop/shared/Mobile Flutter UNIT/WIDGET、Rust UNIT/FIXTURE sourceあり。本mapping単位で製品資源test・OS実測は実行していない（NOT_RUN）。 |
| 証拠ID（Evidence ID） | NONE。Schema/vector/test sourceはinstalled OS resource measurementや性能Acceptanceの証拠ではない。 |
| 阻害要因（Blocker） | N09製品Acceptance OPEN。Registry状態を維持し、新規Blockerを追加しない。直接帰属はN11照合までUNKNOWN。 |
| 状態 | Phase 11のE0-04 mapping 21項目を記録。旧Phase/C status不変、N09製品AcceptanceとE0はOPEN。 |
| 残余 | Task ID単位のResource/duration比較、五OS installed実測・unknown表示・負荷／長時間QA。 |

Phase 9〜11の3行は各E0-04全21項目を記録し、計41件のSourceSHA参照（33 unique path）を編集前mainへ結合した。旧C2/C3 status、N09、Release Registry、Evidence IDは変更せず、製品Acceptanceを発行していない。後続Phase 12〜14記録後もPhase 15〜45、K01〜K20個別fieldおよびN11 Evidence/Blocker/OS Artifact結合は未完了、E0はOPEN。

#### Phase 12〜14 E0-04個別記録（2026-11-13）

旧Phase/Cの履歴状態を保持し、現行v4要求consumerへE0-04全21項目で対応付ける。SourceSHAは編集前main 1d15ceec1b85bea1d7b857a48fd7c8dbd92f5d37のGit blob値であり、製品Acceptanceの証拠ではない。本mapping単位で製品testは再実行せずNOT_RUNとする。

##### Phase 12 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | Phase 12 Runtime lifecycle。現行consumerはN09、施工線C、能力K04/K13。 |
| 旧ID | Phase 12/C4。Runtime start/stop/restart/pause/resume/quarantineのcapability-first契約を保持し、旧debug/fixture statusは当時の限定範囲にとどめる。 |
| 原文参照 | docs/D4_POCKET_PHASE_MAPPING.md Phase 12/C4、v4仕様書S04 N09・S05 K04/K13、工程表R03線C、施工指示書I04-C。 |
| 対象 | Brokerが管理するRuntimeの状態照会と、登録Adapterへ要求するRuntime lifecycle操作。AgentTask lifecycleとは別対象。 |
| 目的 | 現在登録されたRuntime capability・Permission・独立ApprovalをBrokerで再検査し、操作結果・Audit・Recoveryを同じ要求へ結合する。 |
| 生成元（Producer） | 利用者のRuntime lifecycle request、BrokerのRuntime登録・capability・Permission・Approval状態。Adapter metadataはAuthorityを生成しない。 |
| 実Consumer | Runtime Center→Desktop RuntimeLifecycleClient→共有RuntimeLifecycleClient→Rust Brokerのstatus/approval/execute→登録Adapter。通常ReleaseのRuntimeRegistryは現在unsupportedを返す。 |
| 契約／Data | runtime_lifecycle_request/result、Runtime identity、operation capability、要求hash、Permission、Approval、RecoveryAction、Audit binding。 |
| 正常時作用 | 信頼済みAdapter・対応capability・有効なPermission・一回Approval・RecoveryActionが揃う場合だけ操作し、Brokerが結果とAuditを確定する。現在の通常Releaseにはtrusted lifecycle adapterがなく、この作用は未成立。 |
| 否定時作用 | unsupported/unknown Runtime・capability、期限切れ/不一致承認、invalid transition、Audit/Recovery欠落を副作用前に拒否する。未対応を成功へ変換しない。 |
| 停止／Recovery | Adapter失敗・応答不一致・Audit/Recovery失敗では成功表示を止め、必要な隔離を行う。Runtime停止結果とAgentTask process群の停止結果は別状態として扱う。 |
| 標準OS候補 | Windows/macOS/Linux native Desktop。Mobile操作面の存在はOS process authorityの証拠ではない。Linux nativeは必須、installed環境はN10へ接続する。 |
| 配布物 | Phase 12固有artifactなし。Adapterを含むOS別通常package・installed lifecycleはN10と各OS契約に従う。 |
| 既存実装 | Runtime Center、Desktop/shared RuntimeLifecycleClient、Rust runtime_lifecycle.rs/runtime_registry.rs/protocol.rs、request/result schemasとFlutter/Rust test source。 |
| BUILD_GAP | 通常Releaseにtrusted lifecycle adapterがなく、production Runtimeへの作用は未接続。旧debug/fixtureとclient operationはinstalled作用の証拠でなく、AgentTask単位のpause/resume/restartも未接続。 |
| ソース識別値（SourceSHA） | 照合HEAD=1d15ceec1b85bea1d7b857a48fd7c8dbd92f5d37。apps/desktop_flutter/lib/screens/runtime_center.dart=d2c7af172073d2b10c76a6416b90f011f10e95db; apps/desktop_flutter/lib/services/runtime_lifecycle_client.dart=332e438a3d67fc01118ca6cc48745a372c44e01e; packages/gui_shell_ui/lib/src/runtime_lifecycle_client.dart=d34d773ec25f21bb682577faa7688d350921f059; native/rust_helper/src/broker/runtime_lifecycle.rs=2746ec301e9a47a5ececf1485ca7ad8a17304388; native/rust_helper/src/broker/runtime_registry.rs=9ff28610862586a599848d75f9e891a6c6cadf2f; native/rust_helper/src/broker/protocol.rs=9b2076d5f31ca6d77f1f87973d1b394aaf751961; specs/runtime_lifecycle_request.schema.json=7c1ae69a0088cd32593481c7c803c4219baa6444; specs/runtime_lifecycle_result.schema.json=1695ec8c3cc2d2e492ff0ac326f8990ec00fe77e; apps/desktop_flutter/test/runtime_lifecycle_test.dart=57cb17c453c3a50169d3f9e39de150c691e8e160; packages/gui_shell_ui/test/runtime_lifecycle_client_test.dart=1b1f274cdcd8d4cf92225db173eafcc09c0c9b8f. |
| 試験区分（TestClass） | Flutter UNIT/WIDGET、Rust UNIT/FIXTUREのsourceあり。本mapping単位ではRuntime操作testを実行していない（NOT_RUN）。 |
| 証拠ID（Evidence ID） | NONE。debug/fixture sourceやCapability表示はproduction Runtime作用のinstalled Acceptanceではない。 |
| 阻害要因（Blocker） | N09製品Acceptance OPEN。Release Registryの既存状態を維持し、新規Blockerを追加しない。直接帰属はN11照合までUNKNOWN。 |
| 状態 | Phase 12のE0-04 mapping 21項目を記録。旧C4 statusは履歴のまま、N09製品AcceptanceとE0はOPEN。 |
| 残余 | trusted production Adapter、Windows/macOS/Linux installed Runtime操作、failure/Recoveryの実作用、AgentTask lifecycleとの正しい結合。 |

##### Phase 13 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | Phase 13 Evaluation Lab。現行consumerはN09、施工線C、能力K05。 |
| 旧ID | Phase 13/C5。Dataset/Case/Evaluator/Experiment/Result/Comparisonと旧Windows局所statusは履歴のまま保持する。 |
| 原文参照 | docs/D4_POCKET_PHASE_MAPPING.md Phase 13/C5、v4仕様書S04 N09・S05 K05、工程表R03線C、施工指示書I04-C。LLM Judge単独を保証にしない。 |
| 対象 | Owner管理Evaluation Dataset/Case/Evaluator、明示Experiment、bounded Result/Comparison。Regression Case管理は別consumerとして区別する。 |
| 目的 | 評価入力・対象・結果のidentityを保ち、決定論的評価を実施する。評価metadataやscoreをPermission/Approval/release authorityへ変換しない。 |
| 生成元（Producer） | Ownerが管理するDataset/Case/Evaluatorと明示Experiment request、Brokerの評価Result/Comparison。 |
| 実Consumer | Evaluation Lab→Desktop EvaluationClient→共有EvaluationClient→Broker evaluation_control/evaluation_lab。Regression Caseの一覧/delete/recoveryだけは別clientへ接続し、登録・private内容表示・自動importはしない。 |
| 契約／Data | evaluation_case/dataset/registration/evaluator/experiment/result/public_result/comparison schemas、Runtime identity、bounded result、公開・非公開範囲。 |
| 正常時作用 | BrokerがDataset/Runtime/要求を検査し、許可された決定論EvaluatorのExperiment結果・Comparisonを返す。評価実行自体は製品AcceptanceやTask resultではない。 |
| 否定時作用 | malformed/unknown Dataset・Runtime、無効Evaluator/Experiment、stale request、Authority metadata混入、private result露出を拒否・制限する。LLM Judge単独結果は保証として扱わない。 |
| 停止／Recovery | 失敗・部分Result・Audit/hash不一致をPASSへ昇格せず、表示・比較を保留する。Evaluation resultでTask失敗・Runtime停止を置換しない。 |
| 標準OS候補 | Windows/macOS/Linux native Desktop。Mobile metadata投影をEvaluation実行/比較の同等性と扱わない。Linux native必須、installed候補はN10参照。 |
| 配布物 | Phase 13固有artifactなし。OS別通常packageとinstalled Evaluation QAはN10の範囲へ接続する。 |
| 既存実装 | Evaluation画面の`apps/desktop_flutter/lib/screens/evaluation_lab.dart`、Desktop/sharedの`EvaluationClient`、Rust Brokerの`evaluation_control.rs`／`evaluation_lab.rs`／`authority.rs`／`protocol.rs`、8件の`schema`、Flutter `UNIT`／`WIDGET` test source。 |
| BUILD_GAP | Evaluation Resultは同一AgentTask IDのtest result/Diff/Auditへ未結合。Runtime横断比較・同一candidate acceptanceのinstalled evidence、旧Dataset import/version残差はOPEN。 |
| ソース識別値（SourceSHA） | 照合HEAD=1d15ceec1b85bea1d7b857a48fd7c8dbd92f5d37。apps/desktop_flutter/lib/screens/evaluation_lab.dart=b74b3e96e7cd746874d1e19e05175548a5c66192; apps/desktop_flutter/lib/services/evaluation_client.dart=d617dbbf10334754652a131e8612d3ca99aa61e6; packages/gui_shell_ui/lib/src/evaluation_client.dart=3fe93e4a19bcba7c825f2a6e887f96f00016aa75; native/rust_helper/src/broker/evaluation_control.rs=570485772c3fce43d170d031f1769b7617214d40; native/rust_helper/src/broker/evaluation_lab.rs=3645202da2597e5f63cf9ef96a891e9ae801e2b2; native/rust_helper/src/broker/authority.rs=067a36442138897aac396bdb588c2e01f90a5318; native/rust_helper/src/broker/protocol.rs=9b2076d5f31ca6d77f1f87973d1b394aaf751961; specs/evaluation_case.schema.json=5fed01706bb374697294ccb19eec1ae309d53f6a; specs/evaluation_dataset.schema.json=8a90b7248e16c04d7b51ab9bca0a9a5f6c459ed1; specs/evaluation_dataset_registration.schema.json=a79c103e32229dc47d263e62c05914624085610b; specs/evaluation_evaluator.schema.json=d2d5298afc927dfb82c73f136db7dce77bdccf13; specs/evaluation_experiment.schema.json=e35d7a242a0a9d318900673df9fb900e841b93a2; specs/evaluation_result.schema.json=f1b994e19dfaf1473a615cbf1347de22f72ccbed; specs/evaluation_public_result.schema.json=77a285a53a0473fc4185d18b37b863f90be71ada; specs/evaluation_comparison.schema.json=124af9c4e4def4c048751e911809b6f2ed3b577a; apps/desktop_flutter/test/evaluation_lab_test.dart=9b068a34d9d22743bee74ba409c8d95bee938638; packages/gui_shell_ui/test/evaluation_client_test.dart=044d71b4b18b7efa8891a7df735f28e6ed674ac1. |
| 試験区分（TestClass） | Flutter UNIT/WIDGET、Rust UNIT/FIXTURE sourceあり。本mapping単位で評価testを実行していない（NOT_RUN）。 |
| 証拠ID（Evidence ID） | NONE。旧C5局所test/結果を同一Task評価またはN09 installed Acceptanceへ転用しない。 |
| 阻害要因（Blocker） | N09製品Acceptance OPEN。Registry 18件（16 unresolved/2 resolved）とrelease_ready=falseを維持し、新規Blockerなし。 |
| 状態 | Phase 13のE0-04 mapping 21項目を記録。旧C5 status不変、N09製品AcceptanceとE0はOPEN。 |
| 残余 | 同一Task IDのtest/Result/Diff/Audit接続、Runtime横断実比較、Dataset import/version、五OS installed QA。 |

##### Phase 14 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | Phase 14 DialogueからRegression CaseをOwner登録。現行consumerはN09、施工線C、能力K05。実Task生成境界はN01/N09。 |
| 旧ID | Phase 14/C6。完了DialogueからOwnerが明示redactionしたRegression Caseを登録する旧Windows ProtectedStore/metadata receipt/delete-recovery scopeを保持。 |
| 原文参照 | docs/D4_POCKET_PHASE_MAPPING.md Phase 14/C6、docs/specs/regression-case.md、v4仕様書S04 N01/N09・S05 K05、工程表R03線C、施工指示書I04-C。 |
| 対象 | Broker-bound Dialogue submission receiptとOwner明示redaction済みRegression Caseの登録、metadata list、Owner delete/recovery。 |
| 目的 | 内容露出境界内で意図した回帰入力を保存し、Dialogue resultからEvaluation Dataset・Permission・Approvalを暗黙生成しない。 |
| 生成元（Producer） | Dialogue submission receipt/終了Auditと、利用者が明示redactionしたCase payloadおよび操作別native confirmation要求。 |
| 実Consumer | Dialogue UI→RuntimeDialogueClient/RegressionCaseOwnerClient→Windows Rust launcher native confirmation→Broker RegressionCase operation→ProtectedStore/hash-only receipt。通常一覧はmetadata-only RegressionCaseClient。 |
| 契約／Data | runtime_dialogue_submission_receipt、regression_case_registration/receipt/list/delete/delete-recovery schemas。Request hash、source identity、redaction、Owner confirmation、Broker Audit bindingを検査する。 |
| 正常時作用 | Brokerがreceipt/Audit/hash/redaction/current policyを再照合して許可されたCaseを保存し、hash-only receipt/metadataを返す。登録・private contentの画面表示・C5への自動importとは別。 |
| 否定時作用 | raw/secret content、tampered receipt、stale request/approval、hash mismatch、不許可delete/recoveryを拒否する。Windows native confirmationはWindows本人性・OS account identity proofではない。 |
| 停止／Recovery | receipt/Audit/stored object不一致では成功表示せず、Recovery状態を保持する。deleteと中断Recoveryは別Owner-bound requestであり、保存済Caseは過去Authorityを与えない。 |
| 標準OS候補 | Windows Desktop source consumerのみ確認。macOS/Linux nativeの同等ProtectedStore/native confirmation/installed作用は未確立。Mobileは本Consumer外。Linux native自体は必須対象。 |
| 配布物 | Phase 14固有artifactなし。各OSの正式artifact・Owner UI・store・installed lifecycleはN10/OS契約で確認する。 |
| 既存実装 | docs/specs/regression-case.md、Desktop Dialogue/RegressionCase UI・client、Rust regression_case.rs/protocol.rs/ipc_server.rs/desktop_launcher.rs、登録/list/delete/recovery schemas、UNIT/FIXTURE/Conformance test source。 |
| BUILD_GAP | 現candidate installed Owner register/delete/recovery、Windows以外の保護store/confirmation、AgentTask result artifactへの完全bindingは未成立。C5 Dataset importには独立Owner操作/versionが必要で自動importなし。 |
| ソース識別値（SourceSHA） | 照合HEAD=1d15ceec1b85bea1d7b857a48fd7c8dbd92f5d37。docs/specs/regression-case.md=ce58a891facfca9f1e8e498e13bf983011350ba3; specs/runtime_dialogue_submission_receipt.schema.json=204b7871ab08539c16da2431541a5764b023f1fe; specs/regression_case_registration.schema.json=04165813e2791196c38fe515c84b4e1adeefd000; specs/regression_case_receipt.schema.json=3defc85eab0e8ed4ebac4ba85a1a5b83de6016a4; specs/regression_case_list_request.schema.json=72ded3ae0e37214b442811ba973bdf4a490aa854; specs/regression_case_list.schema.json=a7f99f9784b287b7bb21678bc12ab531debf5af0; specs/regression_case_delete_request.schema.json=7ed67c02cb4796da94adc6dd0b1674f4b8a91ca4; specs/regression_case_delete_receipt.schema.json=d051f4e795f8c05051356b0f1d1dc89947db3633; specs/regression_case_delete_recovery_request.schema.json=c69d71ef2f07a2a7090ba3d0d648448c60ca1c4e; specs/regression_case_delete_recovery_receipt.schema.json=b9b1de18f811bc03f38d5dfaa497f673e335e496; native/rust_helper/src/broker/regression_case.rs=cf229213a54f0eaa512781fb417692506bf06775; native/rust_helper/src/broker/protocol.rs=9b2076d5f31ca6d77f1f87973d1b394aaf751961; native/rust_helper/src/broker/ipc_server.rs=58a404b920ef5905e174668ec2de0e04a9e8b044; native/rust_helper/src/desktop_launcher.rs=6a9f335de07fa474ca10043aea63ad6e988b572d; packages/gui_shell_ui/lib/src/runtime_dialogue_client.dart=44f810aa2509a8a28ddf2933a5491528f3b07147; packages/gui_shell_ui/lib/src/runtime_dialogue.dart=f78083b376f80d06f284387bb8b7aa2d769141bf; packages/gui_shell_ui/lib/src/regression_case_owner_client.dart=1f1d10cecca70dff2075bed1e5f331267e0ea296; packages/gui_shell_ui/lib/src/regression_case_client.dart=f14bd4aeda1d6f3262cdb1be4b924884d7128e5b; apps/desktop_flutter/lib/screens/runtime_dialogue.dart=803edd0a687acdfd00e19898fbb5ba6b85ce1449; apps/desktop_flutter/lib/services/runtime_dialogue_client.dart=a0853ee7327cafa60ee89b0c5ee49312b73c46c4; apps/desktop_flutter/lib/services/regression_case_client.dart=c29054e8b6256b15f097bd5e7de57b9cf2117e3a; packages/gui_shell_ui/test/regression_case_owner_client_test.dart=48a895c5a2f139eb2823df67f4aad3e1b8639748; packages/gui_shell_ui/test/regression_case_client_test.dart=a30a002abf021730f0ba09e66126ebbb2a39e12a; tooling/conformance_tests/run_conformance_skeleton.py=c6f31d8cb561b48513c6784e54c85969c9cb953e; apps/desktop_flutter/test/runtime_dialogue_test.dart=899912f92ee3751f8616632d434e04f95ee5ef77. |
| 試験区分（TestClass） | Flutter UNIT/WIDGET、Rust UNIT/FIXTURE、STATIC_CONFORMANCE sourceあり。本mapping単位ではinstalled Owner actionを実行していない（NOT_RUN）。 |
| 証拠ID（Evidence ID） | NONE。過去Broker/ProtectedStore限定証拠とsource mappingは現候補のinstalled Owner Acceptanceではない。 |
| 阻害要因（Blocker） | N09製品Acceptance OPEN。C6専用Blockerの直接帰属はUNKNOWN、Registryへの追加・状態変更なし。 |
| 状態 | Phase 14のE0-04 mapping 21項目を記録。旧C6 statusは履歴のまま、N09製品AcceptanceとE0はOPEN。 |
| 残余 | 導入済み製品での`Owner`登録・削除・復旧、OS別保護保存先・確認、同一`Task`結果/成果物との束縛、`C5 Dataset`への明示取込・版管理。 |

Phase 12〜14の3行は各E0-04全21項目を記録し、計52件のSourceSHA参照（50 unique path）を編集前mainへ結合した。旧C4/C5/C6 status、N09、Registry、Evidence IDは変更せず、製品Acceptanceを発行していない。Phase 15〜45、K01〜K20個別field、N11 Evidence/Blocker/OS Artifact結合は未完了、E0はOPEN。

#### Phase 15〜17 E0-04個別記録（2026-11-13）

旧Phase/Cとの関係と過去statusを保持し、現行v4の要求consumerへ接続する。下記SourceSHAは全て照合基準main `3a573cbe76bea863318c1cb37fec49f21dec223e`時点のGit blobであり、製品動作・Acceptanceを証明しない。

##### Phase 15 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | D4追加Phase 15「接続先・Model設定」。現行要求consumerはv4仕様S04 N03、能力対応はS05 K06、施工線B。 |
| 旧ID | rev1に独立した同一C工程なし。旧製品P4 Provider/Model Centerは範囲の一部が重なる履歴であり、Phase 15/N03全体と同一視しない。 |
| 原文参照 | `docs/D4_POCKET_PHASE_MAPPING.md` Phase 15、v4仕様S04 N03／S05 P4・K06、工程表R03線B、施工指示書I04-B。旧P4範囲は本台帳P4照合を参照。 |
| 対象 | Provider／Model選択、Runtime/Adapter登録・能力確認、選択Runtimeへの共通AgentTask dispatch・Result・failure・Authority境界。 |
| 目的 | Provider/Modelの選択を登録・Broker認可・実Runtime作用へ一貫して結び、未対応・未確認の能力や接続を成功と誤表示しない。少なくとも二種類の独立したTask-capable Runtimeを比較可能な共通契約へ接続する。 |
| 生成元（Producer） | Agent CenterのProvider/Model/Runtime選択とOwner-bound登録要求、Broker Runtime/Agent Catalog、各Adapterの実能力とTask要求。選択値・宣言metadata自体はAuthorityではない。 |
| 実Consumer | Agent Center登録画面→native Launcher確認→Rust Broker IPC/protocol→`ProviderModelSelection`／Runtime registry→Adapter選択→共通AgentTask dispatch→Codex CLIまたは明示非対応の拒否。Runtime Centerは限定Catalog/lifecycle投影を表示する。 |
| 契約／Data | `provider_model_selection.schema.json`、`agent_cli_runtime_workspace_registration.schema.json`、`agent_adapter.schema.json`、Adapter管理契約。現sourceのProvider IDはCodex CLI向け固定、Model ID検査、自動fallback無効。実Credential値は選択projectionへ含めない。 |
| 正常時作用 | 現在確認できるCodex経路ではModel IDが登録要求からBroker Adapterを経てCLI `--model`へ渡る。Provider接続・Model実在性・実能力は観測した範囲でのみ表示し、未確認はunknownとする。 |
| 否定時作用 | 未知/未対応Provider、malformed/stale登録、許可外Model ID、能力不一致、Task非対応Runtimeを拒否する。Adapter/Agent metadataから実能力・Trust・Permissionを昇格せず、自動Provider fallbackをしない。 |
| 停止／Recovery | Owner/Broker検査、interface probe、Runtime dispatchの失敗をその操作の失敗として返す。結果不明のTaskを再送・過去Approvalで再開せず、通常AgentTaskのAudit/Recovery規則へ接続する。 |
| 標準OS候補 | Windows／macOS／Linux native／Android／iOSが対象。現Provider/AgentTaskのsource経路は主にDesktop。Linux nativeは必須対象だが、五OSの同等installed経路やMobile AgentTask consumerは未成立。保証version/arch/sessionはN10のUNKNOWNを保持する。 |
| 配布物 | Phase 15固有artifactなし。Runtime/Adapterの同梱・導入・更新と一般利用者経路はN10およびOS契約に属する。既存Codex CLI試験を利用者へのterminal/CLI導入要求へ読み替えない。 |
| 既存実装 | Codex CLI一経路のProvider/Model selection、Model validator、native確認、Broker登録、共通Task dispatchとCodex Adapterがsource上接続。MINIDORAは対話AdapterでTask未対応として拒否する。P4の過去fixture/build履歴は限定履歴のまま。 |
| BUILD_GAP | 二つ目の独立Task-capable Runtimeの実Task・失敗・権限証拠、実Provider/Model到達、宣言Capabilityと実動作の照合、同一候補の五OS installed経路が未成立。MINIDORAを第二Task Runtimeとして数えない。 |
| ソース識別値（SourceSHA） | 編集前main `3a573cbe76bea863318c1cb37fec49f21dec223e`のGit blob: `docs/specs/adapter-conformance.md`=`e095d9f3bf9d4e6931dac193a304f43d75355c3d`; `specs/provider_model_selection.schema.json`=`348ef6104b9d73a5529a0ad81f9a268ae59e4a64`; `specs/agent_cli_runtime_workspace_registration.schema.json`=`32a09f2ce33a5a215d64350ee22e24e93f9037c7`; `specs/agent_adapter.schema.json`=`e0792589a2bae168ba221af4f4f9abb505815c12`; `specs/adapter_management_request.schema.json`=`2c9c98d0c60603adfaa246f9efa4753bfc0e7597`; `specs/adapter_management_receipt.schema.json`=`7b43a2d2b74c5bd0a931ead66f32f55663919b04`; `specs/adapter_management_manifest.schema.json`=`1f1b75c7d4299c96fcf5039b652d718f7805cd2d`; `native/rust_helper/src/main.rs`=`b0a5df285db357172e7970074d7a67189f3384a3`; `native/rust_helper/src/adapters/mod.rs`=`06c540a59437b903b3f045b5aafb3790c21103bd`; `native/rust_helper/src/adapters/codex_cli.rs`=`e5c9d1581ae9a1c0a3ae7fcfa626fdca6fea6d28`; `native/rust_helper/src/adapters/minidora.rs`=`31c756ac7c9a264d73ed0e9654dbae599be83921`; `native/rust_helper/src/broker/ipc_server.rs`=`58a404b920ef5905e174668ec2de0e04a9e8b044`; `native/rust_helper/src/broker/dialogue.rs`=`c3c3fddb630dd06b7e6cc1c13b39f3d4f6f6a999`; `native/rust_helper/src/broker/protocol.rs`=`9b2076d5f31ca6d77f1f87973d1b394aaf751961`; `native/rust_helper/src/broker/runtime_registry.rs`=`9ff28610862586a599848d75f9e891a6c6cadf2f`; `native/rust_helper/src/broker/adapter_center.rs`=`66575bb77e4a375dca4f8488f97df442960bb5a9`; `native/rust_helper/src/desktop_launcher.rs`=`6a9f335de07fa474ca10043aea63ad6e988b572d`; `apps/desktop_flutter/lib/screens/runtime_center.dart`=`d2c7af172073d2b10c76a6416b90f011f10e95db`; `apps/desktop_flutter/lib/screens/agent_center.dart`=`2472faeb31a64d624423c30a0a30d0f153d2e2a3`; `apps/desktop_flutter/test/widget_test.dart`=`f4e9c7b93f832ba2a2cefee779fd12426f240b04`; `native/rust_helper/tests/agent_task_production_e2e.rs`=`fdc7565aa58fa7466e6e24e5492321e8015567eb`; `native/rust_helper/tests/fixtures/fake_codex_cli.rs`=`f74302045176eeabd9ed36e85205fed3adc13767`. |
| 試験区分（TestClass） | Rust UNIT/FIXTURE、Flutter WIDGET、Schema/Conformance test sourceあり。本Source mapping単位の製品testはNOT_RUN。fake CLI、source test、過去P4結果は実Provider接続や二Runtime Acceptanceではない。 |
| 証拠ID（Evidence ID） | NONE。P4の過去限定証拠は元候補・範囲の履歴として保持し、N03/W03へ転記しない。 |
| 阻害要因（Blocker） | Registry 18件（16 unresolved／2 resolved）と`release_ready=false`を維持。N03との直接帰属が確定しない項目はUNKNOWN。新規Blocker登録・既存status変更なし。 |
| 状態 | Phase 15のE0-04全21項目を記録。旧P4の限定CLOSED履歴は維持し、N03/W03/G3およびE0製品AcceptanceはOPEN。 |
| 残余 | 第二RuntimeのTask-capable実装と正常/失敗/権限証拠、実Provider/Model確認、能力差表示、五OS標準配布・installed consumer、N11 Evidence/Blocker/Artifact結合。 |

##### Phase 16 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 旧C7「資格情報保管庫」。現行consumerはN01/N02/N05/N10、施工線A/B/F、能力K07/K08/K17。 |
| 旧ID | 総合機能拡張rev1 C7／Phase 16。Owner登録、DPAPI等の限定store、metadata一覧、秘密非投影、当時の拒否・fixture履歴はその範囲の履歴として保持する。 |
| 原文参照 | `docs/D4_POCKET_PHASE_MAPPING.md` Phase 16、v4仕様S04 N01/N02/N05/N10・S05 K07/K08/K17、工程表R03/R06、施工指示I04-A/B/F、`docs/specs/credential-vault.md`とOS別binding契約、本台帳C7照合。 |
| 対象 | OwnerによるCredential登録・metadata参照・更新／失効／削除要求と、承認済みProvider/MCP用途での対象限定Credential利用。Device Link credentialは別用途・別主体として区別する。 |
| 目的 | 秘密の保存・限定利用をBroker/OS保護境界へ閉じ、実値のFlutter/IPC projection/Audit/error/log/trace/test artifactへの露出と、Credential所持からPermission/Approval/Trustへの昇格を防ぐ。 |
| 生成元（Producer） | native OS入力面のOwner要求、Rust Broker credential registration/revocation、OS vault metadata、Brokerが検査するProvider/MCP用途・対象binding。 |
| 実Consumer | Windows/macOS/Linux native credential input→Rust Broker `credential_vault`/`credential_storage`→OS保管先。Providerは許可済みCodex Task→Broker resolver→Codex CLI parent process、MCPはMCP Center→Broker `mcp_center`→対象stdio child。Flutterには検証済metadataのみ投影する。 |
| 契約／Data | Credential registration/receipt/list/revocation schemas、MCP connection/tool schemas、Credential VaultとOS別binding文書。公開要求はID/purpose/target/secret-free metadataに限る。secret実値はOS storeおよび短命な限定Rust処理へ閉じる。 |
| 正常時作用 | native Owner操作後BrokerがAudit/store状態・用途・対象を照合して登録する。通常listはmetadata-only。利用直前に登録hash・失効・対象binding・Task/MCP認可を再検査し、現在の許可されたCLIまたはstdio childへ限定する。 |
| 否定時作用 | malformed/unknown fields、重複ID、Audit/store欠落・改変、失効、用途/対象不一致、非対応保管先を拒否する。secret値は画面・通常IPC・Audit本文・log/error/traceへ返さない。 |
| 停止／Recovery | 登録/Audit/storeの検査不成立なら利用を開始しない。失効後の再利用は禁止し、新規Owner要求で再登録する。物理削除/Recoveryおよび各子孫process cleanupの完全保証は未接続・OS依存としてunknownのまま。 |
| 標準OS候補 | Windows/macOS/Linux native/Android/iOS全て標準製品対象。現Credential provider/MCP sourceは主にDesktop三OS、Android Keystore/iOS Keychainを含む同一Task経路は未成立。Device Link資格は一般Provider/MCP Credential consumerの証拠ではない。Linux nativeは必須。 |
| 配布物 | Phase 16独自artifactなし。OS別vault、native Owner input、provider/MCP process binding、通常配布とinstalled lifecycleはN10対象。source・bridge testはinstalled受入れではない。 |
| 既存実装 | Rust Broker vault/storage、Windows DPAPI、Linux Secret Service、macOS Keychain向け限定source、native入力、Codex/MCP credential binding、metadata-only client projectionがある。旧C7/C9/P6の試験履歴は元scopeで保持。 |
| BUILD_GAP | 五OS installedでOwner登録→保管→Task利用→失効/更新/削除/復旧を結ぶ証拠なし。Android/iOS一般Provider/MCP vault consumer、物理削除/Recovery、全OS process isolation/cleanupの一体保証は未成立。 |
| ソース識別値（SourceSHA） | 編集前main `3a573cbe76bea863318c1cb37fec49f21dec223e`のGit blob: `docs/specs/credential-vault.md`=`f27d54012cf5932356c186d4ebef295dbd535178`; `docs/specs/macos-credential-vault.md`=`73af335c3c33a6788d3ac0070156535b022c6608`; `docs/specs/macos-provider-credential-binding.md`=`39a1a5db2a132a025f146681817dc4fadc0cb983`; `docs/specs/macos-mcp-credential-binding.md`=`a86e2a10bafd63caeb392159d31f4fd7d5c1fa9a`; `specs/credential_registration.schema.json`=`65c8bc535e36c040e8574277f06ab1ecb2589d19`; `specs/credential_receipt.schema.json`=`657bfc7917123d212cff462ff7572886effc3457`; `specs/credential_list.schema.json`=`61f706b09533181df57ac781d7eed5adb1199254`; `specs/credential_revocation_request.schema.json`=`43aed5aa2d4f8dc56d9f91386b69b73bed7a3427`; `specs/mcp_connection.schema.json`=`a8d613bd911c9e88f949b984b1a2032eb19db332`; `specs/mcp_tool_call.schema.json`=`54c044cdfb53588ea5bab4bd056404372dc39acd`; `specs/mcp_tool_call_receipt.schema.json`=`0049a393104b035bbf047e653e962754bc4009b3`; `specs/agent_mcp_tool_request.schema.json`=`f3aa71661ac1dadf453954141802a7663755da9d`; `native/rust_helper/src/broker/credential_vault.rs`=`029dccda7671d51c2bbd3debd4fe5d7e92221162`; `native/rust_helper/src/broker/credential_storage.rs`=`86cf0459915bb748f2d3d5848b20913d9c6080f2`; `native/rust_helper/src/broker/mcp_center.rs`=`8f9ab897bdc625e448241ebd9b46f995cd1f22f6`; `native/rust_helper/src/adapters/codex_cli.rs`=`e5c9d1581ae9a1c0a3ae7fcfa626fdca6fea6d28`; `native/rust_helper/src/adapters/mcp_stdio.rs`=`b7e08327541ad75bca53bb1563e0e20d576d9b66`; `native/rust_helper/src/mcp_task_tool_stdio.rs`=`6107890e27be2b9f6bef93a9d46828a488aa1bf1`; `native/rust_helper/src/windows_credential_input.rs`=`3f22358ce15f69d34a93ab5f46186374caba13ec`; `native/rust_helper/src/macos_credential_input.rs`=`f4a44a0dd9d374243e38dab8726bb44c51970422`; `native/rust_helper/src/linux_credential_input.rs`=`8594403a89e4ed1f8eab38b0bb73c62bbceeb79b`; `native/rust_helper/src/broker/protocol.rs`=`9b2076d5f31ca6d77f1f87973d1b394aaf751961`; `apps/desktop_flutter/lib/main.dart`=`6109c821ab5dc809224bec5234c4c59db1d5f988`; `apps/desktop_flutter/lib/screens/mcp_connection_center.dart`=`4c2dd1122e2162a4c3ab30b5c2536e267fee8519`; `apps/desktop_flutter/lib/services/mcp_connection_client.dart`=`12a173a3956368c53615255c56ef06a2f5939c9a`; `apps/desktop_flutter/lib/services/agent_mcp_task_controller.dart`=`fba6642904d98890339197bef269164970031830`; `packages/agent_orchestration/lib/mcp_connection_client.dart`=`71b17973216160bdd2bc447243ba0b04fd0fec63`; `packages/agent_orchestration/lib/agent_mcp_task_controller.dart`=`fa46ff071cfb306d3e463a9ad2c79b9149dab6f5`; `apps/desktop_flutter/test/mcp_connection_center_test.dart`=`c81a37ca78b15e57c838f48519aa3e4abb6d6665`; `apps/desktop_flutter/test/agent_mcp_task_controller_test.dart`=`5c165d5362eea4906b6e739407b0d0e75c34485c`; `apps/desktop_flutter/test/linux_credential_vault_test.dart`=`64c250a967fcfbc3b5da941810afbf558178dfe7`; `apps/desktop_flutter/test/macos_credential_vault_test.dart`=`5d2ae1900cf31f7ff06ee6236ede132b0e933ea3`; `native/rust_helper/tests/unit/codex_cli_fixture.rs`=`40e5750bf267ccf9e583971e0d2ba4e6125d9a5d`; `native/rust_helper/tests/fixtures/fake_codex_cli.rs`=`f74302045176eeabd9ed36e85205fed3adc13767`; `apps/desktop_flutter/test/support/real_broker_mcp_probe.dart`=`0cc9aeb2b1ae6f0e14f1573d4e0cbd7aef7fa137`. |
| 試験区分（TestClass） | Rust UNIT、Flutter bridge/client/widget、Schema/negative fixture sourceあり。旧C7/P6の集計は履歴。本Source mapping単位のtestはNOT_RUN、五OS installed testではない。 |
| 証拠ID（Evidence ID） | NONE。過去の限定OS/store/fixture記録を現candidateの五OS Credential Acceptanceへ転記しない。 |
| 阻害要因（Blocker） | Registry 18件（16 unresolved／2 resolved）と`release_ready=false`を維持。C7直接Blockerの帰属が不明なものはUNKNOWN。新規追加・状態変更なし。 |
| 状態 | Phase 16のE0-04全21項目を記録。旧C7 status/証拠範囲は履歴のまま、N01/N02/N05/N10とE0製品AcceptanceはOPEN。 |
| 残余 | Android/iOS一般Credential consumer、五OS installed一体経路、更新/物理削除/Recovery、Provider/MCP子processの失敗後cleanup境界、N11 artifact/evidence/blocker結合。 |

##### Phase 17 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 旧C8 MCP接続契約。現行consumerはN02/N05/N10、施工線A/D、能力K08/K17。 |
| 旧ID | 総合機能拡張rev1 C8／Phase 17。metadata-only契約、当時のfixture/conformance結果と実接続をPhase 19/C9へ分けた履歴を保持する。 |
| 原文参照 | `docs/D4_POCKET_PHASE_MAPPING.md` Phase 17、v4仕様S04 N02/N05/N10・S05 K08/K17、工程表R03、施工指示I04-A/D、`docs/specs/mcp-contract.md`、本台帳C8/C9照合。 |
| 対象 | MCP Server/Transport/Connection、Tool/Resource/Prompt metadata、Tool inputSchema、Credential reference、Capability差分とBrokerでの検査契約。実接続・外部作用はPhase 19/C9の別consumer。 |
| 目的 | 外部MCP metadata/schemaを未信頼入力として検査し、Tool identity/引数契約と現在Broker Catalogの一致を保つ。Metadata/description/Trust/Credential reference/CapabilityからAuthorityを作らない。 |
| 生成元（Producer） | MCP discovery/connection応答の外部metadata、Tool/Resource/Prompt description・schema、OperatorのServer/Tool選択。Brokerが外部値を検査・正規化したcatalogを生成する。 |
| 実Consumer | Rust `mcp_center`/`mcp_stdio`はconnection/catalog/Tool identity/inputSchemaを検査。Flutter MCP CenterはBroker projectionを表示。ComposeとAgent orchestrationは限定Catalog参照、Tool dispatchは別途現行認可済みC9/N02/N05 consumerへ渡す。 |
| 契約／Data | `mcp_contract`、connection/list/receipt、disconnect、tool_call/receipt、Agent MCP request schemasと`mcp-contract.md`。Resource/Prompt schema/metadataの存在は全操作consumerの成立を意味しない。 |
| 正常時作用 | 対応範囲のServer/Tool Catalogを表示し、Tool作用直前に現在Server/Tool ID・name・schema・argumentを再照合する。実作用にはnative Owner確認、一回Permission、Audit、適切なCredential bindingが別途必要。 |
| 否定時作用 | malformed/unknown field、外部schema参照、unsupported dialect、サイズ上限超過、Authority/secret混入、stale catalog、identity/schema不一致を拒否する。外部metadataはTrust/Permission/Approvalを生成しない。 |
| 停止／Recovery | Discovery/schema/transport失敗を成功扱いしない。契約metadataから実行・自動retry/Recoveryを起動せず、作用結果不明なら既存C9停止/隔離/Audit規則へ戻す。 |
| 標準OS候補 | Windows/macOS/Linux native Desktopにsource経路、Android/iOSのMCP Tool action consumerは未接続。Linux native必須。source/conformanceの存在はOS別installed acceptanceではない。 |
| 配布物 | 独自配布物なし。MCP Serverは接続先、D4 Pocketの通常package/installed process boundaryはN10対象。 |
| 既存実装 | MCP contract/connection/call schemas、Broker Catalog/Tool validator、Rust stdio adapter、Flutter MCP Center、Compose/Agent限定参照sourceがある。旧C8のmetadata contract/conformance履歴と実接続C9の区分を維持。 |
| BUILD_GAP | Resource/Prompt完全処理、五OS installed conformance、Mobile Tool作用、現candidateの外部MCPから同Agent follow-up/Auditまでの一体証拠は未成立。C8 contract sourceをC9実接続成功とみなさない。 |
| ソース識別値（SourceSHA） | 編集前main `3a573cbe76bea863318c1cb37fec49f21dec223e`のGit blob: `docs/specs/mcp-contract.md`=`62fffe6af42253af5656ca06f630868487f277bc`; `docs/specs/mcp-connection-center.md`=`64d46042f19fccb0f7a947a95711e9aae87a1564`; `specs/mcp_contract.schema.json`=`733f1ecf2ad968e98c7d0c3557e5ec897a4cac23`; `specs/mcp_connection.schema.json`=`a8d613bd911c9e88f949b984b1a2032eb19db332`; `specs/mcp_connection_receipt.schema.json`=`ef8909932c68391ce680bbf53b8d80db2254c5f2`; `specs/mcp_connection_list.schema.json`=`45f5613aa46be7f936bd87995da0b6b6b38d047f`; `specs/mcp_tool_call.schema.json`=`54c044cdfb53588ea5bab4bd056404372dc39acd`; `specs/mcp_tool_call_receipt.schema.json`=`0049a393104b035bbf047e653e962754bc4009b3`; `specs/mcp_disconnect.schema.json`=`0808dab9afcafa3e2cd04743a081d33bb30e227d`; `specs/mcp_disconnect_receipt.schema.json`=`7c635dd550354263df062b68136b4a19f224aad7`; `specs/agent_mcp_tool_request.schema.json`=`f3aa71661ac1dadf453954141802a7663755da9d`; `native/rust_helper/src/mcp.rs`=`0ba59eb42c2b2328c05044dc4d459f6b0e288703`; `native/rust_helper/src/adapters/mcp_stdio.rs`=`b7e08327541ad75bca53bb1563e0e20d576d9b66`; `native/rust_helper/src/broker/mcp_center.rs`=`8f9ab897bdc625e448241ebd9b46f995cd1f22f6`; `native/rust_helper/src/broker/protocol.rs`=`9b2076d5f31ca6d77f1f87973d1b394aaf751961`; `native/rust_helper/src/broker/credential_vault.rs`=`029dccda7671d51c2bbd3debd4fe5d7e92221162`; `native/rust_helper/src/broker/compose_center.rs`=`d80658fd9a342140acc35560b2e4c4e4ae97d888`; `apps/desktop_flutter/lib/screens/mcp_connection_center.dart`=`4c2dd1122e2162a4c3ab30b5c2536e267fee8519`; `apps/desktop_flutter/lib/services/mcp_connection_client.dart`=`12a173a3956368c53615255c56ef06a2f5939c9a`; `apps/desktop_flutter/lib/services/agent_mcp_task_controller.dart`=`fba6642904d98890339197bef269164970031830`; `apps/desktop_flutter/lib/screens/settings.dart`=`66374ec4157d573939e7f9cc5c36aecf5d65e251`; `packages/agent_orchestration/lib/mcp_connection_client.dart`=`71b17973216160bdd2bc447243ba0b04fd0fec63`; `packages/agent_orchestration/lib/agent_mcp_task_controller.dart`=`fa46ff071cfb306d3e463a9ad2c79b9149dab6f5`; `apps/desktop_flutter/test/mcp_connection_center_test.dart`=`c81a37ca78b15e57c838f48519aa3e4abb6d6665`; `apps/desktop_flutter/test/agent_mcp_task_controller_test.dart`=`5c165d5362eea4906b6e739407b0d0e75c34485c`; `apps/desktop_flutter/test/support/real_broker_mcp_probe.dart`=`0cc9aeb2b1ae6f0e14f1573d4e0cbd7aef7fa137`; `native/rust_helper/tests/unit/mcp_disconnect.rs`=`98b5ac79fc023f0188690fd787ca685ae88a40b2`; `native/rust_helper/tests/agent_task_production_e2e.rs`=`fdc7565aa58fa7466e6e24e5492321e8015567eb`. |
| 試験区分（TestClass） | Rust UNIT/FIXTURE、Schema/Conformance、Flutter client/widget、real-broker probe sourceあり。本mappingの実MCP/installed試験はNOT_RUN。Fake serverやSchema PASSは外部MCP作用の証拠でない。 |
| 証拠ID（Evidence ID） | NONE。旧C8 metadata/conformance履歴およびC9限定接続履歴は各条件内で保持し、現候補installed Acceptanceへ転用しない。 |
| 阻害要因（Blocker） | Registry 18件（16 unresolved／2 resolved）、`release_ready=false`を維持。C8直接Blockerの帰属が不明なものはUNKNOWN。新規追加・既存状態変更なし。 |
| 状態 | Phase 17のE0-04全21項目を記録。旧C8 status/履歴不変。N02/N05/N10およびE0製品AcceptanceはOPEN。 |
| 残余 | MCP Resource/Prompt実consumer、C9の外部MCP実作用と同Agent follow-up、五OS installed接続、Mobile Tool action、N11 Evidence/Blocker/Artifact結合。 |

Phase 15〜17の3行は各E0-04全21項目を記録し、85件（65 unique path）のSourceSHA参照を編集前main `3a573cbe76bea863318c1cb37fec49f21dec223e`へ結合した。P4、C7、C8の過去status/証拠範囲、N02/N03/N05/N10、Registry、Evidence IDは変更せず、現行製品Acceptanceを発行していない。Phase 18〜45、K01〜K20個別field、N11 Evidence/Blocker/OS Artifact結合は未完、E0はOPEN。Rust/Flutter product testとinstalled/五OS QAはNOT_RUN。

#### Phase 18〜20 E0-04個別記録（2026-11-13）

現行v4要求と実ConsumerをPhase別に照合する。SourceSHAは編集前main 1a3fffdd9cf140e80658723edaea4c07c85f987bのGit blob値であり、source mappingやfixtureは製品Acceptanceを証明しない。

##### Phase 18 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 旧Phase 18 MCP試験基盤。v4ではMCP契約／接続の否定・適合試験を支える限定test infrastructure。 |
| 旧ID | Phase 18／C8・C9。fake server、fixture、conformanceの当時範囲と旧statusを履歴として保持する。 |
| 原文参照 | v4仕様S04 N02/N05/N08/N11、S05 K08/K19/K20、工程表R03、docs/D4_POCKET_PHASE_MAPPING.md Phase 18、docs/specs/mcp-contract.md、docs/specs/mcp-connection-center.md。 |
| 対象 | MCP discovery/catalog/Tool requestと応答のSchema・validator・Broker／Flutter consumerをfixtureで検査する開発用基盤。 |
| 目的 | malformed、未知field、stale identity、Tool schema不適合、Authority／Credential混入などを検出し、試験用応答を実MCP接続やTrustへ昇格させない。 |
| 生成元（Producer） | conformance vectors、Schema正常／否定例、Rust/Flutter test fixture、および開発専用synthetic stdio Server。 |
| 実Consumer | tooling/conformance_tests/run_conformance_skeleton.py、Rust mcp parser／Broker validator、Flutter MCP client/controller testがfixture dataを消費する。macOS synthetic serverは開発用exampleで通常製品へ同梱しない。 |
| 契約／Data | mcp_contract、connection/list/receipt、disconnect、tool_call/receipt、agent_mcp_tool_request Schemaとstdio JSON-RPC discovery／Tool応答。 |
| 正常時作用 | 合成discovery／catalog／Tool応答と正常Schema例をvalidator・Broker consumerへ与え、期待するprojection／receipt形を検査できる。fixtureの応答は外部作用ではない。 |
| 否定時作用 | malformed／unknown／oversize／unsupported schema、metadataによるAuthority主張、Tool identity／引数不一致等を拒否する試験sourceがある。 |
| 停止／Recovery | fixtureまたはconformance不成立は失敗として扱い、実MCP操作・自動retry・Permission／Approvalを発行しない。開発fixtureのcleanup挙動を全OS production cleanupへ一般化しない。 |
| 標準OS候補 | Rust／Flutterのtest sourceはある。synthetic MCP exampleはmacOS向け開発専用。Windows／macOS／Linux nativeのinstalled testを本mappingで実行・確認しておらず、Android／iOSのTool作用consumerは未接続。Linux nativeは必須製品対象。 |
| 配布物 | test/conformance sourceと開発専用example。通常のD4 Pocket配布物へfixtureを含める契約はない。製品package／導入／OS別実行はN10で別途扱う。 |
| 既存実装 | MCP Schema群、Rust parser／Broker code、Flutter test／controller source、conformance validator、およびmacOS専用synthetic stdio exampleを確認。実行結果は本単位では未取得。 |
| BUILD_GAP | 現候補での実MCP接続・同Agent結果依存Task・Auditをfixture基盤から証明できない。OS別testの実行・installed product Acceptanceも本mappingの範囲外で未成立。 |
| ソース識別値（SourceSHA） | 編集前mainのGit blob: docs/specs/mcp-contract.md=62fffe6af42253af5656ca06f630868487f277bc; specs/mcp_contract.schema.json=733f1ecf2ad968e98c7d0c3557e5ec897a4cac23; specs/mcp_connection.schema.json=a8d613bd911c9e88f949b984b1a2032eb19db332; specs/mcp_connection_receipt.schema.json=ef8909932c68391ce680bbf53b8d80db2254c5f2; specs/mcp_connection_list.schema.json=45f5613aa46be7f936bd87995da0b6b6b38d047f; specs/mcp_disconnect.schema.json=0808dab9afcafa3e2cd04743a081d33bb30e227d; specs/mcp_disconnect_receipt.schema.json=7c635dd550354263df062b68136b4a19f224aad7; specs/mcp_tool_call.schema.json=54c044cdfb53588ea5bab4bd056404372dc39acd; specs/mcp_tool_call_receipt.schema.json=0049a393104b035bbf047e653e962754bc4009b3; specs/agent_mcp_tool_request.schema.json=f3aa71661ac1dadf453954141802a7663755da9d; native/rust_helper/src/mcp.rs=0ba59eb42c2b2328c05044dc4d459f6b0e288703; native/rust_helper/src/broker/mcp_center.rs=8f9ab897bdc625e448241ebd9b46f995cd1f22f6; native/rust_helper/src/adapters/mcp_stdio.rs=b7e08327541ad75bca53bb1563e0e20d576d9b66; native/rust_helper/tests/unit/mcp_disconnect.rs=98b5ac79fc023f0188690fd787ca685ae88a40b2; native/rust_helper/examples/macos_mcp_fixture.rs=b7b805544a2445b10075dc920c8bd2d0d51f10ae; apps/desktop_flutter/test/mcp_connection_center_test.dart=c81a37ca78b15e57c838f48519aa3e4abb6d6665; apps/desktop_flutter/test/agent_mcp_task_controller_test.dart=5c165d5362eea4906b6e739407b0d0e75c34485c; apps/desktop_flutter/test/support/real_broker_mcp_probe.dart=0cc9aeb2b1ae6f0e14f1573d4e0cbd7aef7fa137; tooling/conformance_tests/run_conformance_skeleton.py=c6f31d8cb561b48513c6784e54c85969c9cb953e. |
| 試験区分（TestClass） | Rust UNIT／FIXTURE、Flutter client／controller／widget、Schema／conformance、synthetic stdio source。各testは本Source mapping単位でNOT_RUN。 |
| 証拠ID（Evidence ID） | NONE。本mappingでは新規Evidenceを発行せず、fixture／Schema／test sourceをLIVE_PRODUCT evidenceとしない。 |
| 阻害要因（Blocker） | Registry 18件（16 unresolved／2 resolved）とrelease_ready=falseを維持。Phase 18固有の直接Blockerは未確定としてUNKNOWN。新規追加・既存status変更なし。 |
| 状態 | Phase 18のE0-04全21項目を記録。旧C8/C9 status・試験履歴は元scopeに保持し、N02/N05/N08/N11の製品AcceptanceはOPEN。 |
| 残余 | fixture／validatorの本単位実行、実外部MCPの現候補接続、五OS installed試験、N11 Evidence／Blocker／OS Artifact結合。 |

##### Phase 19 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 旧Phase 19 MCP接続管理。v4ではN02/N05とK08に属する実MCP discovery／connect／catalog／Tool作用／結果投影consumer。 |
| 旧ID | Phase 19／C9。catalog・receipt・拒否試験の履歴statusを保持し、Phase 18 test infrastructureと実接続を混同しない。 |
| 原文参照 | v4仕様S04 N02/N05/N10、S05 K08/K17、工程表R03、施工線A/D、docs/D4_POCKET_PHASE_MAPPING.md Phase 19、docs/specs/mcp-connection-center.md、docs/specs/mcp-contract.md、OS別MCP契約。 |
| 対象 | Flutter MCP Centerから認証済Brokerを経由するstdio ServerのOwner確認付き接続・一覧・切断、明示Tool call、およびAgent MCP結果context接続。 |
| 目的 | 実Tool作用を現在のBroker Catalog／Schema／一回Permission／Owner確認／Auditで統治し、Agent／Server metadataやMCP結果からTrust・Authorityを生成しない。 |
| 生成元（Producer） | OperatorのServer設定・connect/list/disconnect/Tool要求、およびAgent Task本文から抽出する未信頼MCP Tool proposal。 |
| 実Consumer | Flutter MCP Center→Broker client→Rust protocol／mcp_center→mcp_stdio child。Agent連携はmcp_connection_client／agent_mcp_task_controllerからBroker Tool result contextとfresh follow-up Taskへ接続する。Credential実値は対象stdio childに限定。 |
| 契約／Data | MCP接続／一覧／受領記録、切断要求／受領記録、Tool実行要求／受領記録、資格情報ID参照、登録済み能力一覧の識別値、Tool識別子／引数識別値、監査ID、元Task結果識別値、Agent／Session／Workspaceへの結合情報。 |
| 正常時作用 | Brokerが接続要求をnative Owner確認と既存Permission／Audit境界で処理し、Catalogを検査。明示Tool call時にcurrent Catalog／Schema／Credential bindingを再評価して一度dispatchし、許可された結果依存経路は同Agentの新しいTaskへ未信頼contextを渡す。 |
| 否定時作用 | 未知Server／Tool、malformed／stale catalog、schema不一致、失効Credential、Owner／Permission／Audit不成立、Task／Session／Workspace結合ずれを拒否・隔離する。外部Toolの成否不明を成功にせず自動再送しない。 |
| 停止／Recovery | transport／Tool結果が不明または監査不成立なら停止・隔離し、結果依存follow-upへ進めない。後続Taskは現在条件でPermission／Approvalを新規評価する。OS固有process containmentを維持し、全OS一様な子孫cleanupを推論しない。 |
| 標準OS候補 | Windows／macOS／Linux native Desktopにsource経路。Linux nativeは必須製品対象だが本source mappingはinstalled acceptanceでない。Android／iOSのMCP Tool作用consumerはBUILD_GAP。 |
| 配布物 | 独立MCP Center配布物なし。stdio Serverは利用者が接続する外部runtimeであり、D4 PocketのOS別通常package／installed境界はN10対象。 |
| 既存実装 | Rust MCP parser／Broker catalog・Tool処理・stdio adapter、Windows native launcher／macOS限定Owner経路、Flutter Center／client／Agent controller、Credential bindingと関連test／probe sourceを確認。 |
| BUILD_GAP | W02/G2の現候補installed実Codex→実MCP Tool→結果依存同Agent Task→AuditはOPEN。五OSのinstalled一体証拠、Mobile Tool作用、OS間process cleanup同等性も未成立。 |
| ソース識別値（SourceSHA） | 編集前mainのGit blob: docs/specs/mcp-connection-center.md=64d46042f19fccb0f7a947a95711e9aae87a1564; docs/specs/mcp-contract.md=62fffe6af42253af5656ca06f630868487f277bc; docs/specs/macos-mcp-center.md=62482591a7db1948e279cdb09b06de7b3ded2afb; docs/specs/macos-mcp-credential-binding.md=a86e2a10bafd63caeb392159d31f4fd7d5c1fa9a; specs/mcp_contract.schema.json=733f1ecf2ad968e98c7d0c3557e5ec897a4cac23; specs/mcp_connection.schema.json=a8d613bd911c9e88f949b984b1a2032eb19db332; specs/mcp_connection_receipt.schema.json=ef8909932c68391ce680bbf53b8d80db2254c5f2; specs/mcp_connection_list.schema.json=45f5613aa46be7f936bd87995da0b6b6b38d047f; specs/mcp_disconnect.schema.json=0808dab9afcafa3e2cd04743a081d33bb30e227d; specs/mcp_disconnect_receipt.schema.json=7c635dd550354263df062b68136b4a19f224aad7; specs/mcp_tool_call.schema.json=54c044cdfb53588ea5bab4bd056404372dc39acd; specs/mcp_tool_call_receipt.schema.json=0049a393104b035bbf047e653e962754bc4009b3; specs/agent_mcp_tool_request.schema.json=f3aa71661ac1dadf453954141802a7663755da9d; native/rust_helper/src/mcp.rs=0ba59eb42c2b2328c05044dc4d459f6b0e288703; native/rust_helper/src/broker/mcp_center.rs=8f9ab897bdc625e448241ebd9b46f995cd1f22f6; native/rust_helper/src/adapters/mcp_stdio.rs=b7e08327541ad75bca53bb1563e0e20d576d9b66; native/rust_helper/src/broker/protocol.rs=9b2076d5f31ca6d77f1f87973d1b394aaf751961; native/rust_helper/src/broker/credential_vault.rs=029dccda7671d51c2bbd3debd4fe5d7e92221162; native/rust_helper/src/broker/credential_storage.rs=86cf0459915bb748f2d3d5848b20913d9c6080f2; native/rust_helper/src/mcp_task_tool_stdio.rs=6107890e27be2b9f6bef93a9d46828a488aa1bf1; native/rust_helper/src/broker/compose_center.rs=d80658fd9a342140acc35560b2e4c4e4ae97d888; native/rust_helper/src/desktop_launcher.rs=6a9f335de07fa474ca10043aea63ad6e988b572d; apps/desktop_flutter/lib/screens/mcp_connection_center.dart=4c2dd1122e2162a4c3ab30b5c2536e267fee8519; apps/desktop_flutter/lib/services/mcp_connection_client.dart=12a173a3956368c53615255c56ef06a2f5939c9a; apps/desktop_flutter/lib/services/agent_mcp_task_controller.dart=fba6642904d98890339197bef269164970031830; packages/agent_orchestration/lib/mcp_connection_client.dart=71b17973216160bdd2bc447243ba0b04fd0fec63; packages/agent_orchestration/lib/agent_mcp_task_controller.dart=fa46ff071cfb306d3e463a9ad2c79b9149dab6f5; native/rust_helper/tests/unit/mcp_disconnect.rs=98b5ac79fc023f0188690fd787ca685ae88a40b2; native/rust_helper/tests/agent_task_production_e2e.rs=fdc7565aa58fa7466e6e24e5492321e8015567eb; apps/desktop_flutter/test/mcp_connection_center_test.dart=c81a37ca78b15e57c838f48519aa3e4abb6d6665; apps/desktop_flutter/test/agent_mcp_task_controller_test.dart=5c165d5362eea4906b6e739407b0d0e75c34485c; apps/desktop_flutter/test/support/real_broker_mcp_probe.dart=0cc9aeb2b1ae6f0e14f1573d4e0cbd7aef7fa137; apps/desktop_flutter/test/widget_test.dart=f4e9c7b93f832ba2a2cefee779fd12426f240b04. |
| 試験区分（TestClass） | Rust UNIT／FIXTURE／production E2E source、Flutter client／controller／widget、real Broker probe sourceあり。本mappingでは再実行しておらず、installed外部MCP Acceptanceではない。 |
| 証拠ID（Evidence ID） | NONE。既存W02/G2履歴は当時のcandidate／条件で保持し、本mappingから新しいLIVE_PRODUCT Evidenceを発行しない。 |
| 阻害要因（Blocker） | W02/G2はOPEN。Registry 18件（16 unresolved／2 resolved）・release_ready=falseを維持し、C9以外への直接帰属はUNKNOWN。新規Blockerなし。 |
| 状態 | Phase 19のE0-04全21項目を記録。旧C9 status・Evidenceは履歴のまま、N02/N05/N10およびE0製品AcceptanceはOPEN。 |
| 残余 | 現候補installed W02/G2、OS別実MCP接続・結果追跡、Mobile Tool consumer、N11 Evidence／Blocker／OS Artifact結合。 |

##### Phase 20 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 旧Phase 20 運用Profile。v4能力K09に直接対応し、N09/N11は運用状態確認・Audit／Evidence投影の境界で関係する。 |
| 旧ID | Phase 20／C10。Profile CRUD・永続化・Permission非生成の旧statusを保持し、実Runtime適用の未接続を維持する。 |
| 原文参照 | v4仕様S05 K09、S04 N09/N11の関連consumer、工程表R03/R06、docs/D4_POCKET_PHASE_MAPPING.md Phase 20、docs/specs/operation-profile.md。 |
| 対象 | Desktop SettingsからProfileを作成／複製／一覧／適用要求／削除／export／importし、Brokerが文書を永続化・検査する経路。 |
| 目的 | 再利用可能なRuntime／Adapter／Capability／Content Exposure／resource要求とUI preferenceを保持する一方、ProfileからPermission／Approval／Credential／Authorityを発行しない。 |
| 生成元（Producer） | OperatorのSettings入力と選択からFlutter profile_clientがBroker operation requestを生成する。 |
| 実Consumer | Settings→profile_client→認証済Broker IPC→Rust profile_center→Broker store内profiles.json。SettingsはBroker receipt／list projectionを表示する。Runtime／Agent TaskへProfileを適用するconsumerは現sourceから特定できない。 |
| 契約／Data | profile／profile_list／profile_receipt Schema。Profile文書、ProfileID/hash、Runtime／Adapter ID、要求Capability、Content Exposure、network/resource limit、UI preference。 |
| 正常時作用 | BrokerがCRUD／複製／入出力／一覧を処理する。適用要求は現在Profile hashを照合し、適用意図をAuditへ記録するだけでRuntime設定・Permission ledgerを変更しない。 |
| 否定時作用 | unknown field、Authority／Credential混入、malformed／duplicate ID、stale hash、上限超過を拒否し、不正な永続stateを起動時fail-closedとする。 |
| 停止／Recovery | atomic persistent writeと起動時再検査を用いる。Runtime適用consumer、適用後rollback、専用file pickerは未接続。Profile apply intentを実作用と扱わない。 |
| 標準OS候補 | Desktop Flutter／Rust Broker sourceを確認。製品対象はWindows／macOS／Linux native／Android／iOSのまま。各OS installed Profile操作は本mappingでは未確認で、Linux native必須を維持。 |
| 配布物 | 独立Profile packageなし。通常D4 Pocket productの一部候補であり、OS別package／installed lifecycleはN10に属する。 |
| 既存実装 | Profile schemas、Rust profile_center／store dispatch、Flutter profile_client／Settings CRUD・apply intentおよびclient test sourceあり。実Runtime適用は見つからない。 |
| BUILD_GAP | Profile valueをRuntime／Agent Taskの現在設定へ適用する実consumer、適用変更のRecovery、OS別installed Acceptanceは未成立。N09/N11との存在だけで適用完了としない。 |
| ソース識別値（SourceSHA） | 編集前mainのGit blob: docs/specs/operation-profile.md=c53720b7eb4ad6983d3d84bb6cb3de563936c5e5; specs/profile.schema.json=8619ff6bb553d2d75a6400fd4a4528ca2b83c733; specs/profile_list.schema.json=df38cbef669704b4489a4111d579e975d91c544f; specs/profile_receipt.schema.json=7b9298f9fc5423051a33bfccc7bd28c4d8e875a6; native/rust_helper/src/broker/profile_center.rs=d56264a07ee9f51ffe25317e5a304aa7b97ae45a; native/rust_helper/src/broker/protocol.rs=9b2076d5f31ca6d77f1f87973d1b394aaf751961; native/rust_helper/src/broker/store.rs=356b490e403b693db499befb777789784b57d40d; apps/desktop_flutter/lib/services/profile_client.dart=ab967d9fb3f84c48b7bbc9a1193fbf7563d99105; apps/desktop_flutter/lib/screens/settings.dart=66374ec4157d573939e7f9cc5c36aecf5d65e251; apps/desktop_flutter/test/profile_client_test.dart=ba2bba18bf3b7bc299940984d9240cd4eeffd29a. |
| 試験区分（TestClass） | Rust profile center source/unit coverage, Flutter profile client test, Schema/negative input source. 本Source mappingでtestはNOT_RUN。 |
| 証拠ID（Evidence ID） | NONE。既存局所証拠を五OS Product acceptanceへ転記せず、新規Evidenceを発行しない。 |
| 阻害要因（Blocker） | Registry 18件（16 unresolved／2 resolved）・release_ready=falseを維持。C10への直接Blocker帰属が不明なものはUNKNOWN。新規追加・状態変更なし。 |
| 状態 | Phase 20のE0-04全21項目を記録。旧C10 status・証拠範囲は履歴のまま、K09関連・E0製品AcceptanceはOPEN。 |
| 残余 | 実Runtime／Task適用、適用時のAudit／Recovery、専用file picker、OS別installed consumer、N11 Evidence／Blocker／OS Artifact結合。 |

Phase 18〜20の3行は各E0-04全21項目を記録し、62件（44 unique path）のSourceSHA参照を編集前main 1a3fffdd9cf140e80658723edaea4c07c85f987bへ結合した。C8/C9/C10の旧status、N02/N05/N08/N09/N10/N11、W02/G2、Registry 18件（16 unresolved／2 resolved）、release_ready=falseを変更せず、新規Evidence ID／Blockerなし。fixture／Schema／sourceを製品Acceptanceへ昇格していない。Phase 21〜45、K01〜K20個別field、N11 Evidence／Blocker／OS Artifact全数結合、E0は未完／OPEN。製品test・installed runtime・五OS QAはNOT_RUN。

#### Phase 21〜23 E0-04個別記録（2026-11-13）

現行v4要求・工程線と実ConsumerをPhase別に照合する。以下のSourceSHAは編集前main 32c02bd9de6ceca6fa90b1e82db28f114568e77dのGit blob値であり、source／fixtureはOS別製品Acceptanceを証明しない。

##### Phase 21 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 旧Phase 21 更新管理。現行v4ではN10/N11、施工線G/H/I/J、能力K09/K16/K17へ接続する更新・修復Consumer。 |
| 旧ID | Phase 21／C11。旧Update Centerのcatalog・署名・download・stage・rollback・repair記録を当時のstatusで保持する。 |
| 原文参照 | v4仕様S04 N10/N11、S05 K09/K16/K17、工程表R03/R04、docs/D4_POCKET_PHASE_MAPPING.md Phase 21、docs/specs/update-center.md。 |
| 対象 | Broker所有の固定trust/sourceを使う候補一覧・検証・取得・stage・有効版切替・rollback・限定repair／uninstallとOS別通常package境界。 |
| 目的 | 更新をFlutterの任意network／filesystem／process作用から分離し、package identity・hash・Owner確認・Audit・復旧可能なactive versionで統治する。 |
| 生成元（Producer） | Operatorの明示更新／repair要求、固定trustと配布元、署名済みcandidate metadata、Broker所有download job。 |
| 実Consumer | Flutter SettingsのUpdateClient→認証Broker IPC→Rust update_center／update_download／update_verification／product_install。Windows Launcher／bootstrapperとLinux package toolingがOS側候補を消費する。 |
| 契約／Data | update／candidate／list／receipt／trust／catalog／download-job Schema、package byte length／SHA-256、candidate hash／channel、Owner確認、Audit ID、active version record。署名秘密鍵をbundleしない。 |
| 正常時作用 | Brokerが固定配布元・trust・署名・package bytesを検査し、明示Owner確認後にstage／active version切替／rollback等を要求する経路がある。sourceの存在は一般配布の成立ではない。 |
| 否定時作用 | 不正署名／trust、配布元不一致、redirect／proxy／非global DNS、byte長／hash／inventory不一致、未知path／identity、stale requestを拒否し現有効版を保持する。 |
| 停止／Recovery | Broker jobは再起動で失われ、再要求時にpackage／stageを再検証する。途中stageを自動適用しない。fixture／source試験をinstalled crash・電源断Recoveryへ拡張しない。 |
| 標準OS候補 | WindowsのLauncher／install sourceとLinux package/update sourceがある。Linux nativeは必須対象。macOS／Android／iOSの標準package・署名・update／rollback／uninstall lifecycleは未成立。 |
| 配布物 | Windows bootstrap／Broker-managed packageおよびLinux package source／fixtureを確認。一般向け署名Installer、五OSの同一候補lifecycle、Publisher identityは未成立。 |
| 既存実装 | Rust trust／signature／HTTPS download／stage／rollback、Windows active record／Start Menu／限定repair、Flutter Settings client、Linux package test／acceptance harness sourceを確認。 |
| BUILD_GAP | 一般利用者向け配布からinstalled起動・更新・rollback・repair・uninstallまでの五OS同一candidate経路、実Publisher trust provision、installed crash recoveryとartifact provenanceが未成立。 |
| ソース識別値（SourceSHA） | 編集前mainのGit blob: docs/specs/update-center.md=be37e1f661cb3e4cb05e7c1242051263665a92b2; specs/update.schema.json=351a76c29f0279cc2cef92970f082ba7c2f7aaf1; specs/update_candidate.schema.json=1d803d8a2aaf22576346c77f4c7113ad834d213c; specs/update_list.schema.json=b4ff3283dfd7e781bf6af9a0c208caf3aa7ed7e1; specs/update_receipt.schema.json=86471c54dd55fa55bc1cdc7ebe32050d9f30ed36; specs/update_trust.schema.json=5caa430cf621c09e6e3d96bd6c377758095b9491; specs/update_catalog.schema.json=faa9297d56029f326cfe0c0962a5c89f55c97470; specs/update_download_job.schema.json=42f457bf1fb1275c17f55a90f671cc91e8a51e2e; native/rust_helper/src/broker/update_center.rs=f6fc98155fcd476765b60d382a680c7f7c6d675e; native/rust_helper/src/broker/update_download.rs=9f9f9b23113e724a206427bea082a11dd4764115; native/rust_helper/src/update_verification.rs=6347d90043cc7196feece6d708fafe96d27f3f19; native/rust_helper/src/broker/product_install.rs=c91115bdf88dd28e5f3445e7ed66127902909677; native/rust_helper/src/product_bootstrapper.rs=3b0495dfc9a1937ba64f3f99c95ae4cc739ddc69; native/rust_helper/src/desktop_launcher.rs=6a9f335de07fa474ca10043aea63ad6e988b572d; native/rust_helper/src/installed_artifact.rs=7328ccce0378c92743b13e14f26925e6749826dc; native/rust_helper/src/broker/protocol.rs=9b2076d5f31ca6d77f1f87973d1b394aaf751961; apps/desktop_flutter/lib/services/update_client.dart=26d2e946b5b15ea3328867e606e9f75bf4bfc2d1; apps/desktop_flutter/lib/screens/settings.dart=66374ec4157d573939e7f9cc5c36aecf5d65e251; apps/desktop_flutter/test/update_client_test.dart=f70bb4a6dbc2340b5d26d867496952629dad1218; apps/desktop_flutter/test/settings_install_flow_test.dart=73fc7b9706a7f4246972db46d5398d8b73decb09; tooling/tests/test_linux_update_catalog_fixture.py=a841f259911eefe99f4e474e18c57aae91cde994; tooling/tests/test_linux_installed_product_smoke.py=dc94f9b782d3699810bbe50230a5e9510a5739f7; tooling/tests/run_linux_same_source_update_acceptance.py=6891a0a2db378577d30aec9c9466af594864b8ec; tooling/tests/linux_update_https_server.py=a343f30ae615334a75371bd059891e89a0563907; tooling/package_linux_product.py=0d32d04ec5b43e1815b3664a323c17db79bc928a; specs/d4_pocket_product_package_manifest.schema.json=3151ba81a869b87e6c0fa8f8633a0e424fc43ae7. |
| 試験区分（TestClass） | Rust UNIT／FIXTURE／TLS source、Flutter client／install-flow test、Linux package／installed-update harness sourceあり。本mappingではNOT_RUN。 |
| 証拠ID（Evidence ID） | NONE。旧build／fixture／package sourceを現candidateの五OS installed lifecycle Acceptanceへ転記しない。 |
| 阻害要因（Blocker） | Registry 18件（16 unresolved／2 resolved）とrelease_ready=falseを維持。C11直接Blocker／OS帰属が不明なものはUNKNOWN。新規追加・既存変更なし。 |
| 状態 | Phase 21のE0-04全21項目を記録。旧C11 status・証拠範囲は履歴のまま、N10/N11/K09/K16/E0製品AcceptanceはOPEN。 |
| 残余 | Publisher／trust provisioning、五OS installed lifecycle、Linux native標準環境の更新／撤去、同一candidate provenance、crash／Recovery、N11 Artifact結合。 |

##### Phase 22 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 旧Phase 22 通知管理。v4ではN09、施工線C、能力K10/K13のBroker event由来通知・表示consumer。 |
| 旧ID | Phase 22／C12。固定Audit summary、hash-bound表示状態、Desktop／Mobile投影の過去statusを維持する。 |
| 原文参照 | v4仕様S04 N09/N10関連境界、S05 K10/K13/K17、工程表R03/R04、docs/D4_POCKET_PHASE_MAPPING.md Phase 22、docs/specs/notification-center.md。 |
| 対象 | 確定Audit eventから固定summaryを作るBroker Notification Center、永続read／discard state、Desktop通知画面／tray projectionとMobile read-only projection。 |
| 目的 | boundedな通知を提供し、Audit本文・reason・Credentialを露出せず、通知からPermission／Approval／Authorityを作らない。 |
| 生成元（Producer） | Brokerの固定operation mappingが確定済みAudit eventをsummaryへ射影する。caller任意通知登録はない。 |
| 実Consumer | Rust notification_center→通常Broker IPC→Desktop notification_client／notifications screen／windows_tray_client。MobileはMobileProjectionClient経由の通知summary表示。 |
| 契約／Data | notification／notification_list／notification_action Schema。通知ID／hash、固定種別／summary、read／discard状態。Audit reason／raw payload／Credentialは含めない。 |
| 正常時作用 | Brokerが直近4096 Audit eventを走査して最大256件を返し、read／discardを現在hashに結合する。DesktopとMobileの表示は既存投影を使う。 |
| 否定時作用 | caller任意source、未知operation、malformed／duplicate state、hash不一致、stale notificationを拒否する。通知を承認・監査確定・実行権限として扱わない。 |
| 停止／Recovery | 不正永続stateまたはAudit確定失敗時にactionを成功扱いしない。通知navigationからsensitive actionを直実行しない。native Windows toastの欠落を表示成功に読み替えない。 |
| 標準OS候補 | Desktop／Mobile projection sourceあり。Windows native toast host未接続、OS別installed parity未確認。Linux nativeを含む五OS対象を維持し、sourceだけで対応完了としない。 |
| 配布物 | 独立通知packageなし。通常D4 Pocket UI／BrokerとOS native通知統合の一部候補で、製品package／installed evidenceはN10に属する。 |
| 既存実装 | Rust fixed Audit projection／bounded scan／hash-bound state、Desktop notification screen／client／tray summary、Mobile summary client／screen、Schema／test sourceあり。 |
| BUILD_GAP | Windows toast host、全必須event producer、全OS native notification parity、installed表示／Recoveryの同一candidate evidenceが未成立。Mobile summary sourceはOS配布QAを証明しない。 |
| ソース識別値（SourceSHA） | 編集前mainのGit blob: docs/specs/notification-center.md=81b4ed30b2defd538a56c9323fecb566a1fad8dd; specs/notification.schema.json=3028750ccf7b647867314664cf5280ea7ec26812; specs/notification_list.schema.json=0fa037e50d58c10c4b5a7364c0a7472e708996b6; specs/notification_action.schema.json=d7ab880c65799ac8e69a02c1ae1da594989e8f17; native/rust_helper/src/broker/notification_center.rs=ca6828a33baf94abd7101781d9730b22daca4a78; native/rust_helper/src/broker/audit.rs=eb1dcf5018d95c7fdf6171aadd4bca9554d1996f; native/rust_helper/src/broker/store.rs=356b490e403b693db499befb777789784b57d40d; native/rust_helper/src/broker/protocol.rs=9b2076d5f31ca6d77f1f87973d1b394aaf751961; apps/desktop_flutter/lib/services/notification_client.dart=746c74805ef5b80febf020306daeb360ff43b4e6; apps/desktop_flutter/lib/screens/notifications.dart=850ac48a96ccff94b64b154a4e38d608aabb5d3c; apps/desktop_flutter/lib/services/windows_tray_client.dart=cf8214ceb95fa1b24c7eed4f14b312bda80e0b15; apps/mobile_flutter/lib/screens/notifications.dart=243c59c321cb1cbff39cc6cc993bd01d589c760f; apps/mobile_flutter/lib/services/mobile_projection_client.dart=ba82e4ddeaa5692793237a1bc3258ce53b7aa3ed; apps/desktop_flutter/test/notification_client_test.dart=e92e994e4a6107abc83de34cfed6032f260dd550; apps/desktop_flutter/test/windows_tray_client_test.dart=0b24e07221087e43355ac465ca4ace73a48af31e; apps/desktop_flutter/test/widget_test.dart=f4e9c7b93f832ba2a2cefee779fd12426f240b04; apps/mobile_flutter/test/mobile_projection_client_test.dart=74661d2f2e58046a83e99f711435e4c1b2a74b08. |
| 試験区分（TestClass） | Rust UNIT／FIXTURE、Flutter Desktop client／tray／widget、Mobile projection client、Schema／negative sourceあり。本mappingではNOT_RUN。 |
| 証拠ID（Evidence ID） | NONE。旧局所summary／UI記録を五OS installed notification Acceptanceへ転記しない。 |
| 阻害要因（Blocker） | Registry 18件・release_ready=falseを維持。C12直接Blockerの帰属が不明なものはUNKNOWN。新規Blockerなし。 |
| 状態 | Phase 22のE0-04全21項目を記録。旧C12 status・Evidenceは履歴のまま、N09/N10/E0製品AcceptanceはOPEN。 |
| 残余 | Windows toast host、全producer coverage、Mobile／Desktop／native投影のOS別installed検証、N11 Artifact／Evidence結合。 |

##### Phase 23 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 旧Phase 23 稼働観測。C13 Broker内部Span／Trace／MetricとC14 read-only Trace Inspector、関連するC3 Runtime resource observationを含む。 |
| 旧ID | Phase 23／C13・C14。bounded Broker observation、Trace表示、resourceのlimited sourceと旧statusを各元scopeで保持する。 |
| 原文参照 | v4仕様S04 N09/N11、S05 K04/K10/K13/K19、工程表R03/R04/R08、docs/D4_POCKET_PHASE_MAPPING.md Phase 23、docs/specs/observability-center.md、trace-inspector.md、runtime-resource-observation.md。 |
| 対象 | Audit確定後のBroker内部Span／Trace／Metric、Trace Inspectorの読取り投影、Broker登録Runtimeを対象にした別個のresource query契約。 |
| 目的 | boundedな挙動・性能・資源観測を示し、正式Audit、Runtime health、Permission／Approval／Authority、未観測処理の証明へすり替えない。 |
| 生成元（Producer） | ObservationCenterはAudit finalization後に内部Spanを生成。Runtime resource queryは登録RuntimeへのBroker-owned process bindingから単回sampleを要求する。Trace Inspectorは生成者でない。 |
| 実Consumer | Rust observation_center／runtime_registry→Broker protocolの観測一覧／実行系資源観測→Desktop Observation／Trace／Runtime Center。GUI Shell UI packageとMobile resource overviewのsourceも存在するが、各OSの実行能力は別判定。 |
| 契約／Data | observation_list／span／trace／metric、runtime_resource_query／runtime_resource_observation Schema。TraceID／SpanID、限定Audit ID、証拠種別、PID-bound resource sample、measured／unknown状態。 |
| 正常時作用 | Broker Observationは最大1024 Spanをsession memoryに保持し、一覧応答を最大256 Span／Trace・16 Metricに制限する。Runtime resource routeはread-onlyで、測定不能値をunknownとする。 |
| 否定時作用 | 不正limit／TraceID／unknown field、Audit未確定span、Runtime process binding不一致を拒否またはunknown化する。Metric／TraceからPermission／Approval／Capabilityや未観測spanを推論しない。 |
| 停止／Recovery | Audit失敗時にSpanを記録しない。保持上限では古い内部観測を落とし、再起動後の永続復元／長時間保証を主張しない。Runtime binding失効時はPID関連stateと前回値を破棄しunknownを返す。 |
| 標準OS候補 | Desktop Observation／Trace sourceあり。Runtime process resource contractは現在Windows実装を中心とし、未対応OSはunsupported。Android／iOSに同じRuntime trace consumerは未確認。Linux nativeは必須製品対象。 |
| 配布物 | 独立Observability packageなし。Desktop／Mobile画面と通常製品の一部候補。標準OS package／installed resource・trace parityはN10で別途確認する。 |
| 既存実装 | Rust bounded ObservationCenter、認証Broker IPC、Windows Runtime resource registry、Desktop Observation／Trace／Runtime Center、共有UI resource client、Mobile resource overview、Schema／canonicalization／test sourceを確認。 |
| BUILD_GAP | Runtime／Adapter／Tool／外部通信span、collector/export、長期保存、全OS resource sampling、Mobile Trace、同一Task／Trace結合、installed／long-run evidenceが未接続または未成立。Broker内部spanを他段階へ複製しない。 |
| ソース識別値（SourceSHA） | 編集前mainのGit blob: docs/specs/observability-center.md=f379cd60bdaeaffa1f6970a377999ccaaa393498; docs/specs/trace-inspector.md=d7ca9524d7e3bf26c6fe03e55c6175ca6abfc61a; docs/specs/runtime-resource-observation.md=af29714c0372592a5a86141c6d4ea4f491cbf851; specs/observation_list.schema.json=1c4eba48d5b8a3318253e7ba6890305bd5e89e7c; specs/observation_span.schema.json=8cc3b8e66f242e8009db546145ae8f3fa1582786; specs/observation_trace.schema.json=8365f277eab716618c02e7635eaf5896e6a84820; specs/observation_metric.schema.json=eaa8757dd6b43611728c358a7277ac84efa5f242; specs/runtime_resource_query.schema.json=c7617ba9c2283255fb04b6cfa529ff7b04f6137f; specs/runtime_resource_observation.schema.json=62f587b8c88d0c5c0bd316eaa3d0a15c9c1bd2d1; native/rust_helper/src/broker/observation_center.rs=62896e497a0d78267020db6bae2ecadf704fa1fb; native/rust_helper/src/broker/runtime_registry.rs=9ff28610862586a599848d75f9e891a6c6cadf2f; native/rust_helper/src/broker/protocol.rs=9b2076d5f31ca6d77f1f87973d1b394aaf751961; apps/desktop_flutter/lib/services/observation_client.dart=97a28fb5aadd82c77028d74928bb7fa86a0220cc; apps/desktop_flutter/lib/screens/observability_center.dart=497fe25368b9b108d62bee921d454145b712b657; apps/desktop_flutter/lib/screens/trace_inspector.dart=c196c8e20786d151edc4b709b71dd3b8fa88812b; apps/desktop_flutter/lib/services/runtime_resource_client.dart=d8da9ebee9afdd64b8d1d3afa0a82f3a4cb53cfd; apps/desktop_flutter/lib/screens/runtime_center.dart=d2c7af172073d2b10c76a6416b90f011f10e95db; packages/gui_shell_ui/lib/runtime_resource_client.dart=2df13807ceed7f3305e13ca752cd31f824583543; packages/gui_shell_ui/lib/src/runtime_resource_client.dart=3a8e611b94136e27f6d132e550a49bcbaf586eaf; apps/mobile_flutter/lib/screens/resource_overview.dart=322af6bc2935c571017dab3f03b9c7a0cea7c616; apps/desktop_flutter/test/observation_client_test.dart=6d6ac31671e16d771bbc1a39fa7edcd9a3929dfb; apps/desktop_flutter/test/runtime_resource_test.dart=c44d66bb3797bf523be0f48f24d626068b8e4894; packages/gui_shell_ui/test/runtime_resource_client_test.dart=46c52c83eab712e5b84599c7e29eed660af0ed12; apps/mobile_flutter/test/resource_overview_test.dart=7695155627699274b8c0768c0de302344caa9eec; tooling/conformance_tests/run_conformance_skeleton.py=c6f31d8cb561b48513c6784e54c85969c9cb953e; tooling/canonicalization_vectors/runtime_resource_observation_decimal.json=ce00c0ea9571409a707c4e1993069c61761cb6a0. |
| 試験区分（TestClass） | Rust UNIT／FIXTURE、Flutter Desktop／Mobile／共有UI tests、Schema／canonicalization／conformance sourceあり。本mappingではNOT_RUN。長時間・五OS installed testではない。 |
| 証拠ID（Evidence ID） | NONE。C3 limited resource evidenceと旧C13/C14履歴は元版・条件で保持し、本mappingから新LIVE_RUNTIME／OS QA Evidenceを発行しない。 |
| 阻害要因（Blocker） | Registry 18件・release_ready=falseを維持。C13/C14への直接Blocker帰属が不明なものはUNKNOWN。新規追加・既存変更なし。 |
| 状態 | Phase 23のE0-04全21項目を記録。旧C13/C14／C3 status・証拠範囲は履歴のまま、N09/N11/K04/K10/E0製品AcceptanceはOPEN。 |
| 残余 | Runtime／Adapter／Tool producer、OS別resource境界、長期保持・load、Trace／Task同一性、Mobile parity、N11 Artifact／Evidence結合。 |

Phase 21〜23の3行は各E0-04全21項目を記録し、69件（67 unique path）のSourceSHA参照を編集前main 32c02bd9de6ceca6fa90b1e82db28f114568e77dへ結合した。旧C11〜C14 status、N09/N10/N11、K04/K09/K10/K13/K16/K17、Registry 18件（16 unresolved／2 resolved）、release_ready=falseを変更せず、新規Evidence ID／Blockerなし。Phase 24〜45、K01〜K20個別field、N11 Evidence／Blocker／OS Artifact全数結合、E0は未完／OPEN。製品Rust／Flutter test、installed update／notification／observation、五OS QAはNOT_RUN。



#### Phase 24〜26 E0-04個別記録（2026-11-13）

現行v4 N04／K11等のConsumerと旧C15〜C18をPhase単位で照合する。以下のSourceSHAは編集前main 895a856e6ecf6904fdae153d52d027ce42acf1a6 のGit blobである。旧P7／C15〜C18の局所status・Evidenceは当時scopeに保持し、現行N04／K11の製品受入れへ転記しない。

##### Phase 24 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 現行v4 N04／K11、施工線B。Agent間接続契約のA2A概念・metadata射影。 |
| 旧ID | D4 Phase 24／総合機能拡張rev1 C15「エージェント間接続契約（A2A Contract）」。旧C15の契約・fixture・Conformance記録を当時の範囲で保持する。 |
| 原文参照 | v4仕様S04 N04、S05 K11、工程表R03 B、docs/D4_POCKET_PHASE_MAPPING.md Phase 24、docs/specs/a2a-contract.md、docs/specs/authority-strip-conformance.md。 |
| 対象 | A2A Agent Card／Task／Message／Artifact／StreamをGUI-Shellのbounded metadataへ射影するcontract、Authority Strip、外部宣言の未信頼境界。 |
| 目的 | 外部Agentの概念を共通表示・契約へ正規化し、Card／Task／Message／Artifact／Stream metadataからTrust、Permission、Approval、Authorityを作らない。 |
| 生成元（Producer） | 外部A2A Card／Task／Message／Artifact／Stream宣言とRust A2A正規化・Broker projection。 |
| 実Consumer | Rust a2a正規化とA2A Broker projection、a2a_contract Schema／Conformance、Desktop A2A接続client／画面がmetadataを消費する。contract単独では外部操作を発生させない。 |
| 契約／Data | a2a_contract Schema、metadata_only、authority_strip=true、権限生成なし、interface URL hash、protocol／version／boundedなID・状態・件数・hash・content visibility。 |
| 正常時作用 | Cardと外部概念を検証・bounded metadataへ射影する。A2A v1.0の安全境界を守る限定SendMessage／Stream等はPhase 25の別実作用として扱う。 |
| 否定時作用 | malformed／unknown／unsupported field、認証要件のあるinterface、raw endpoint／credential／contentや権限注入を拒否またはunsupportedへ落とし、Agent宣言を実行可能性・Trustとみなさない。 |
| 停止／Recovery | bounded失敗を返し、過去receiptはowner再確認待ちへ降格する。Card／Task状態からPermission・Approval・Audit identity・Recovery actionを復元せず、自動再送しない。 |
| 標準OS候補 | Schema／正規化は共通。Desktop A2A consumerはWindows／Linux経路を持つ。macOSはAgent Card metadata-onlyの履歴証拠に限定し、Mobile A2A consumerは未確認／未接続。Linux nativeは必須対象で、fixtureは製品配布証拠ではない。 |
| 配布物 | 独立packageなし。D4 Pocket Desktopの契約・Broker・UIに含む候補であり、OS別配布とinstalled acceptanceはN10／N04で別途判断。 |
| 既存実装 | a2a_contract Schema、RustのCard／概念正規化、Authority Strip／Conformance source、Desktop metadata client／画面とfixture testを確認。旧C15のsource・fixture・Conformanceは実外部接続を証明しない。 |
| BUILD_GAP | C15単体では実外部Card／Task／Message／Artifact／Streamの完全相互運用、Credentialを要するinterface、全Artifact型、Task lifecycle、Mobile consumerを成立させない。現N04の実外部Agent作用・OS別統合もOPEN。 |
| ソース識別値（SourceSHA） | 編集前mainのGit blob: docs/specs/a2a-contract.md=abe32b25bf7c186faf6e1beb3ae3f5d754de61c5; docs/specs/a2a-streaming.md=2b1cd55f29cd9c7f4af177d2703b677071e9e96a; docs/specs/authority-strip-conformance.md=117cfed2bd13af62b3fe6c464a98d183fa9fb657; specs/a2a_contract.schema.json=8c7f50bb7fb12dde41293727e5dc9109445d1bc3; native/rust_helper/src/a2a.rs=9d7c6d9ab938ad5178d6ca46b3933dc3e61c6061; native/rust_helper/src/broker/a2a_center.rs=73ad601abd64a33ccefde4b73ab5f9dc4f728504; native/rust_helper/src/broker/protocol.rs=9b2076d5f31ca6d77f1f87973d1b394aaf751961; apps/desktop_flutter/lib/services/a2a_connection_client.dart=19335054afe35bc06597629d92ca2e635ce50069; apps/desktop_flutter/lib/screens/a2a_connection_center.dart=2716d1c2fa22a184044e6f4e2eb401cb33c6e011; apps/desktop_flutter/test/a2a_connection_center_test.dart=5ebdbb4cee62e581062370ed6c305c36df89b7e2; tooling/conformance_tests/run_conformance_skeleton.py=c6f31d8cb561b48513c6784e54c85969c9cb953e. |
| 試験区分（TestClass） | Rust UNIT（a2a正規化・負例source）、Schema／Conformance、Flutter client／widgetのFake Broker test source。本mappingではNOT_RUN。fixture／Schemaは実外部Agentや製品QAのEvidenceではない。 |
| 証拠ID（Evidence ID） | NONE。旧C15のSchema／fixture／Conformance記録は履歴scopeのまま。現N04／K11 Acceptanceへ新Evidenceを転記しない。 |
| 阻害要因（Blocker） | 既存Release Registry 18件（16 unresolved／2 resolved）とrelease_ready=falseを維持。C15への直接Blocker／OS帰属はN11確認前のためUNKNOWN。新規Registry Blockerなし。 |
| 状態 | Phase 24のE0-04全21項目を記録。旧C15 status／証拠範囲を保持し、現N04／K11／E0製品AcceptanceはOPEN。 |
| 残余 | 実外部interface・認証／unsupported判定・全Artifact／Stream・Task lifecycleの相互運用、Mobile接続、五OS installed product接続とN11 Evidence／Blocker／OS Artifact結合。 |

##### Phase 25 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 現行v4 N04／K11、施工線B。外部A2A接続・限定操作のProducer→Broker→結果projection。 |
| 旧ID | D4 Phase 25／総合機能拡張rev1 C16「A2A接続センター」。旧C16のCard接続、限定操作、Mac Card-only受入れを元scope／条件で保持する。 |
| 原文参照 | v4仕様S04 N04、S05 K11、工程表R03 B、docs/D4_POCKET_PHASE_MAPPING.md Phase 25、docs/specs/a2a-connection-center.md、docs/specs/macos-a2a-center.md。 |
| 対象 | Owner明示のAgent Card取得・接続receipt、通常一覧、Windows／Linux Desktopの限定SendMessage／手動GetTask／選択text Artifact／Stream source、およびmacOSのA2A入口境界。 |
| 目的 | A2A接続を既存Rust Broker／native Owner／単回Permission／Auditへ束縛し、外部Agent metadataを信頼・権限へ昇格させず結果不明時の再送を防ぐ。 |
| 生成元（Producer） | 操作者の明示接続／送信要求、現在のA2A connection receipt・interface hash、外部Agent Card／JSON-RPC／SSE response。 |
| 実Consumer | Desktop A2A接続client／screen→Rust Broker a2a_center／a2a helper／固定HTTP(S) transport→bounded result／Audit。macOS RunnerのBrokerProcessChannelはA2A接続・A2Aメッセージ送信を分類し、Rust native Owner候補と既存Brokerへ接続するsourceを含む。 |
| 契約／Data | a2a_connection／receipt／list、SendMessage／GetTask／Stream request／response Schema、Agent ID・interface hash・request hash・単回Permission・Audit ID・bounded metadata。Credential実値は未使用。 |
| 正常時作用 | Windows／Linux DesktopでOwner確認後、公開Cardをmetadata-only接続し、契約で限定された一回作用をBroker経由で実行するsourceがある。macOS Card接続は過去の限定LIVE_RUNTIME記録。Mac message sourceは現AGENTS.md制限との不整合があるため許可済み／LIVE_RUNTIMEとして数えない。 |
| 否定時作用 | 認証要件、private／非global宛先、redirect／proxy、malformed／oversize response、stale receipt/interface hash、Task／Artifact不一致、unsafe／non-text contentを拒否。unknown outcomeを成功や再送へ昇格しない。 |
| 停止／Recovery | 接続receipt復元はpending_review／owner_reapproval_requiredへ降格。Broker／Audit／永続化失敗はfail-closed。送信・照会の結果不明、期限、再起動で自動retry／polling／reconnectしない。 |
| 標準OS候補 | Windows／Linux DesktopにA2A操作source。macOSはAGENTS.md上、Agent Card metadata-onlyの限定controlのみ。macOS product documents／Swift BrokerProcessChannelにMessage送信記述・入口分類があり規則矛盾。Mobile A2A consumerは未確認。Linux nativeは必須だがinstalled一般外部Agent QAは未成立。 |
| 配布物 | 独立packageなし。D4 Pocket Desktop候補の一機能。標準OS installer／署名・更新・撤去・installed同一candidate証拠はN10へ接続。 |
| 既存実装 | Rust bounded HTTP／HTTPS Card取得・Broker A2A center・SendMessage／GetTask／Artifact／Stream source、Desktop接続画面、Mac Card接続の限定旧受入れを確認。SwiftはA2A接続／メッセージ送信を分類する。 |
| BUILD_GAP | 実外部Agentの一般互換／Credential、全Task lifecycle・cancel・reconnect、file/data／全Artifact、Mobile consumer、五OS installed acceptanceは未成立。macOS A2A Message操作はAGENTS.mdのmetadata-only制限と製品文書／Swift sourceが不一致で、利用可能なconsumer範囲をUNKNOWN／SUSPENDとして扱う。 |
| ソース識別値（SourceSHA） | 編集前mainのGit blob: docs/specs/a2a-connection-center.md=a2f815d61aed80b4e1f619c0a2f947960d7bd317; docs/specs/a2a-contract.md=abe32b25bf7c186faf6e1beb3ae3f5d754de61c5; docs/specs/a2a-streaming.md=2b1cd55f29cd9c7f4af177d2703b677071e9e96a; docs/specs/macos-a2a-center.md=6f5bc7444f2e406aa5c7d21d1aea1f7599c743ad; docs/specs/macos-desktop-channel.md=0611524b89c09365d35c33d23367a5fb6f65a4d6; specs/a2a_connection.schema.json=1dbbe44e54b302a1196d7e4e55f1cae35cda569b; specs/a2a_connection_receipt.schema.json=f63fd0d61b6200cdd8cc4b23d5cf7968a82fa5c5; specs/a2a_connection_list.schema.json=ee18b204349b025464ca3af8adaa355add34e650; specs/a2a_message_send_request.schema.json=0a5f87250f17a88b6be55e7838d32b8e9114bbed; specs/a2a_message_send_response.schema.json=23ee690e0ce350ac94c64d86f6920afb63c199f6; specs/a2a_task_get_request.schema.json=df11a5fd3291de0ae379694c6c1b49de51f2f157; specs/a2a_task_get_response.schema.json=56a55f348d6b0ba7cb0092ee7ea2b4ec809e5360; specs/a2a_stream_send_request.schema.json=e0a0841f2fd9f7e92e0fb8a036ebaa6852e198d8; specs/a2a_stream_send_response.schema.json=a526fed89fb5b3766a03c52444a338060caf16cc; native/rust_helper/src/a2a.rs=9d7c6d9ab938ad5178d6ca46b3933dc3e61c6061; native/rust_helper/src/broker/a2a_center.rs=73ad601abd64a33ccefde4b73ab5f9dc4f728504; native/rust_helper/src/broker/update_download.rs=9f9f9b23113e724a206427bea082a11dd4764115; native/rust_helper/src/broker/protocol.rs=9b2076d5f31ca6d77f1f87973d1b394aaf751961; native/rust_helper/src/broker/audit.rs=eb1dcf5018d95c7fdf6171aadd4bca9554d1996f; native/rust_helper/src/broker/dialogue.rs=c3c3fddb630dd06b7e6cc1c13b39f3d4f6f6a999; native/rust_helper/src/macos_desktop_owner.rs=31e13a57fcde422878fae63083bd323e07f482ec; native/rust_helper/examples/macos_a2a_fixture.rs=20299e4cdd564e07abce8f97c5b128404f150282; apps/desktop_flutter/lib/services/a2a_connection_client.dart=19335054afe35bc06597629d92ca2e635ce50069; apps/desktop_flutter/lib/screens/a2a_connection_center.dart=2716d1c2fa22a184044e6f4e2eb401cb33c6e011; apps/desktop_flutter/lib/services/a2a_artifact_handoff.dart=e7e2f3454e2843f1d4f4e7b5aedfa55014cde05c; apps/desktop_flutter/test/a2a_connection_center_test.dart=5ebdbb4cee62e581062370ed6c305c36df89b7e2; apps/desktop_flutter/test/a2a_artifact_handoff_test.dart=e47005320917f59dfa55ae1e04c4b99d452676de; apps/desktop_flutter/macos/Runner/BrokerProcessChannel.swift=a1269e01608bc194b0f0913a6f96cd489b926155; apps/desktop_flutter/macos/RunnerUITests/AdapterOwnerUITests.swift=eeab11b1123165a8f82da55ebbb014eab8803862. |
| 試験区分（TestClass） | Rust UNIT／FIXTURE（embedded／loopback A2A・response negative）、Flutter Fake Broker test、歴史的macOS Card-only product LIVE_RUNTIME記録あり。これはMac Message／Task操作や実外部Agentの現候補QAではない。本mappingでRust／Flutter／installed testはNOT_RUN。 |
| 証拠ID（Evidence ID） | NONE。旧C16／MAC-A2Aの限定Card evidenceは履歴条件で保持し、現N04の一般外部Agent操作へ転記しない。 |
| 阻害要因（Blocker） | 既存Registry 18件（16 unresolved／2 resolved）、release_ready=falseを維持。A2A関連BlockerのPhase25／OSへの直接帰属はN11で確認し、ここでは新規追加しない。 |
| 状態 | Phase 25のE0-04 fieldは記録するが、macOS A2A Message範囲はAGENTS.mdと現行製品記述／sourceの不整合によりUNKNOWN／SUSPEND。現N04／K11／E0はOPEN。規則を拡張・緩和せず、既存コードも変更しない。 |
| 残余 | macOS A2A Messageの正本整合・許可範囲の解決が必要。その他、実外部Agent／Credential／全Task lifecycle・Artifact／Mobile／五OS installed evidenceとN11結合が残る。 |

##### Phase 26 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 現行v4 N04／K11・K13・K17、施工線B/C。Host registryと表示Context操作。 |
| 旧ID | D4 Phase 26／総合機能拡張rev1 C17「複数Host」＋C18「Host操作面」。C17はPhase 2とも、C18はPhase 28とも対応するため機能範囲を重複完了扱いしない。 |
| 原文参照 | v4仕様S04 N04／N10、S05 K11／K13／K17、工程表R03 B/C、docs/D4_POCKET_PHASE_MAPPING.md Phase 2／26／28、docs/specs/macos-host-registration.md。 |
| 対象 | 公開Host metadataのowner registration・bounded registry／一覧、Hostの表示context切替、Host別Runtime／Agent summaryの可視性境界。 |
| 目的 | Hostごとに識別・表示を分け、Host metadataや切替からremote trust、Task実行先、Permission、Approval、Credential、Authorityを生成・継承しない。 |
| 生成元（Producer） | 操作者の明示Host登録／一覧／切替要求と、Owner確認済み公開metadata、Broker registry／現在Host観測。 |
| 実Consumer | Rust host_center／host_capabilityとBroker protocol／Audit、Desktop host_registration_client、Host操作面／Host capability画面。macOSはnative Owner確認から同じ既存Broker登録へ接続するsourceと限定CLOSED product recordがある。 |
| 契約／Data | host_registration／host_list／host_receipt／host_capability／host_switch／host_switch_receipt Schema。Host ID・display/platform・identity hash・申告件数・metadata_only・Trust pending_review・authority_strip。 |
| 正常時作用 | Owner control登録、通常IPC metadata一覧、Audit付き表示context切替を行う。Host切替後のRuntime／Agentは現在Broker観測Hostが一致する範囲だけを表示し、切替自体はremote再接続ではない。 |
| 否定時作用 | 重複／未知Host、malformed／権限field注入、stale receipt、Host不一致を拒否または未観測へ制限。Host間Permission／Approval／Credential／Authority持越しをしない。 |
| 停止／Recovery | 登録は期限付きOwner確認・永続Audit失敗時に不受理。bounded persistent stateの破損／再起動はfail-closed。switchは表示のみでreconnect／remote作用を自動起動しない。 |
| 標準OS候補 | Desktop Rust／Flutter consumer。macOSの公開metadata登録・一覧・表示切替には履歴限定LIVE_RUNTIME。Windows／Linux product pathはsource上で確認するが、このmappingではOS別installed acceptanceを主張しない。Mobile consumer／native Host parityは未確認。Linux native必須対象は維持。 |
| 配布物 | 独立Host packageなし。D4 Pocket DesktopのBroker／UI内機能候補。OS別installer／installed product evidenceはN10に接続する。 |
| 既存実装 | bounded Rust Host registry・owner registration／regular list／switch receipt／Audit・hash-only identity、Desktop UI/client、macOS native Owner Host登録sourceと過去の有限製品受入れを確認。 |
| BUILD_GAP | Host registration／display selectionはlive remote Host connection、Trust attestation、remote Runtime／Agent discovery、Host間Workspace isolationを成立させない。Android／iOS consumerと五OS installed acceptanceは未接続／未証明。 |
| ソース識別値（SourceSHA） | 編集前mainのGit blob: docs/D4_POCKET_PHASE_MAPPING.md=064ea334f86b137d94fba2370d2a2fd732f6e972; docs/specs/macos-host-registration.md=c7b379163a4aec66ef8cfe178034439afae1fcd8; docs/specs/macos-desktop-channel.md=0611524b89c09365d35c33d23367a5fb6f65a4d6; specs/host_registration.schema.json=8bed26fb2fe8f8e296c1d3404978f3215d571240; specs/host_list.schema.json=93880c43b04a5408c033bf351ee695073e54664a; specs/host_receipt.schema.json=237a84717cf27d000357d0dac53d33dd6f535766; specs/host_capability.schema.json=6b721b5ab7842edc7df6562bc56fdd5b73d5de79; specs/host_switch.schema.json=01e5fc7322e8d4d2c9142375e7641fc4255148d5; specs/host_switch_receipt.schema.json=cb60df0b174a38c871e0e8835c65537a17d022fb; native/rust_helper/src/broker/host_center.rs=c27707faa61e5270b1c6d79e3ee3c5fe6ccd0bf8; native/rust_helper/src/broker/host_capability.rs=08a82c7e52947439229447dc82b69097af67ba1f; native/rust_helper/src/broker/protocol.rs=9b2076d5f31ca6d77f1f87973d1b394aaf751961; native/rust_helper/src/broker/audit.rs=eb1dcf5018d95c7fdf6171aadd4bca9554d1996f; native/rust_helper/src/macos_desktop_owner.rs=31e13a57fcde422878fae63083bd323e07f482ec; apps/desktop_flutter/lib/services/host_registration_client.dart=3bc3b45bddddf4e1131e051acb3cdee599ca4d3d; apps/desktop_flutter/lib/screens/host_operation_center.dart=e16605e6bb39bbfc12806e6154e08d6fbab1b93b; apps/desktop_flutter/lib/screens/host_capability_center.dart=e8fa774a435e713a3b75b0d33f00bd500b0a2d7f; apps/desktop_flutter/test/host_registration_test.dart=74ab39e078bf72214a3bbe84f69ebc77fceb2493; apps/desktop_flutter/test/widget_test.dart=f4e9c7b93f832ba2a2cefee779fd12426f240b04; apps/desktop_flutter/macos/Runner/BrokerProcessChannel.swift=a1269e01608bc194b0f0913a6f96cd489b926155; apps/desktop_flutter/macos/RunnerUITests/AdapterOwnerUITests.swift=eeab11b1123165a8f82da55ebbb014eab8803862. |
| 試験区分（TestClass） | Rust単体の否定試験（重複登録、権限注入、未知Host、Host間状態）、FlutterのHost画面試験、macOSの限定製品実行履歴を確認した。本単位ではRust／Flutter／導入済み製品試験を実行していない。Macの証拠はHost登録／表示だけ。 |
| 証拠ID（Evidence ID） | NONE。過去MAC-HOST/P13 limited evidenceは当時の公開metadata／表示範囲で保持し、N04のremote Host／trust／workspace isolationへ転記しない。 |
| 阻害要因（Blocker） | 既存Registry 18件（16 unresolved／2 resolved）とrelease_ready=falseを保持。C17／C18の個別Blocker帰属はN11で照合し、このmappingで新規追加しない。 |
| 状態 | Phase 26のE0-04全21項目を記録。旧C17／C18 statusと証拠範囲を保持し、Phase 2／28との重複を分離。現N04／K11／K13／K17／E0はOPEN。 |
| 残余 | 実Host接続／Trust検証／remote discovery／Workspace隔離、Mobile／五OS installed parity、N11 Evidence／Blocker／OS Artifact結合。 |

Phase 24〜26の3行は各E0-04全21項目を記録し、61件（47 unique path）のSourceSHA参照を編集前main 895a856e6ecf6904fdae153d52d027ce42acf1a6へ結合した。旧C15〜C18 statusと限定証拠、N04／K11／K13／K17、Registry 18件（16 unresolved／2 resolved）、release_ready=falseを変更せず、新Evidence ID／Registry Blockerなし。Phase 2／28とのcrosswalkを保持し、Phase 27〜45、K01〜K20個別field、N11 Evidence／Blocker／OS Artifact結合、E0はOPEN。macOS A2A MessageについてはAGENTS.mdのmetadata-only制限と現行製品文書／Swift sourceの不整合を記録し、許可済み・LIVE_RUNTIMEとは判定しない。製品Rust／Flutter／installed A2A／Host test、五OS QAはNOT_RUN。


#### Phase 27〜29 E0-04個別記録（2026-11-13）

現行v4のN03／N05／N09／N10と旧Phase／C crosswalkを、Phase単位で照合する。以下のSourceSHAは編集前main 87461aaf0e44388833cc4d2bd95ff034177a6c0dのGit blobである。旧C19〜C23の限定status／証拠は履歴scopeに保持し、現行N要件やOS別installed acceptanceへ転記しない。

##### Phase 27 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 現行v4 N03／N05、施工線B／D、K06／K12。Adapter管理とRuntime Centerの接続。 |
| 旧ID | D4 Phase 27／総合機能拡張rev1 C19「Adapter管理操作」。旧C19の局所履歴をそのまま保持する。 |
| 原文参照 | v4仕様S04 N03／N05、S05 K06／K12、工程表R03 B／D、施工指示I04-B／I04-D、docs/D4_POCKET_PHASE_MAPPING.md Phase 27、docs/specs/adapter-management-surface.md、docs/specs/adapter-conformance.md。 |
| 対象 | Runtime CenterのAdapter metadata一覧、登録、署名検証、状態管理、有効化／無効化、隔離、更新、catalog record削除。 |
| 目的 | Runtime Adapterの申告とBroker管理状態を表示・統制し、Adapter metadata、Capability申告、署名表示からPermission、Approval、Trust、Credential、Authorityを生成しない。 |
| 生成元（Producer） | 操作者が提示するAdapter manifest／管理要求、現在のBroker catalog record、Broker所有の署名trustと監査状態。 |
| 実Consumer | Runtime Center→ShellCoreClient→認証Broker IPC／Owner control→adapter_center、署名検証、永続catalog、Audit。macOSはBrokerProcessChannelからnative Owner確認と既存Broker receiverへ接続する限定sourceがある。 |
| 契約／Data | adapter／adapter_manifest／adapter_management_manifest／request／receipt／list Schema。Adapter ID・version・hash・source・publisher・signature metadata・要求Capability・permission diff・risk・compatibility・Content Exposure。 |
| 正常時作用 | 一覧はmetadata-only。導入／更新はBroker catalogへのmetadata登録、検証はBroker trustと正本byteの照合、状態操作はBroker内recordだけを変更する。隔離済Runtime IDは実行へ再利用しない。 |
| 否定時作用 | 未知operation、malformed／stale hash、対象record不一致、未検証署名の有効化、隔離recordの再利用、通常IPCからの無Owner変更、危険な権限昇格を拒否する。 |
| 停止／Recovery | Owner要求hash・対象ID／現hashの不一致、Audit／永続化失敗では変更を成立させない。再起動後はBroker catalogを読み戻す。外部fileやprocessの削除・停止を成功扱いしない。 |
| 標準OS候補 | Desktop Flutter／Brokerの共通source候補はWindows／Linux。macOSはnative Owner経路の限定sourceあり。Android／iOS consumerは未確認。Linux nativeは必須対象だがinstalled product受入れを意味しない。 |
| 配布物 | 独立Adapter packageではない。D4 Pocket Desktop内のRuntime Center／Broker機能候補。OS別installed artifact・標準Installerの受入れはN10へ接続する。 |
| 既存実装 | adapter_centerのmetadata操作・署名検証境界・永続record・否定試験、Runtime CenterとShellCoreClient、通常IPC／Owner control、macOS native Owner経路のsourceを確認。 |
| BUILD_GAP | 外部artifactの実download、filesystem導入／更新／削除、実process起動・停止、一般的なRuntime Adapter実仕事、五OS installed lifecycleは本C19範囲で未成立。 |
| ソース識別値（SourceSHA） | 編集前mainのGit blob: docs/D4_POCKET_PHASE_MAPPING.md=064ea334f86b137d94fba2370d2a2fd732f6e972; docs/specs/adapter-management-surface.md=d13859c61b7042d7d5492feea1cdbc1662eee398; docs/specs/adapter-conformance.md=e095d9f3bf9d4e6931dac193a304f43d75355c3d; docs/specs/macos-desktop-channel.md=0611524b89c09365d35c33d23367a5fb6f65a4d6; specs/adapter.schema.json=0fee764bc04ad524665e4be2b02828dcdb488f73; specs/adapter_manifest.schema.json=116f5830983d195f501cc344a25990d58bf323c0; specs/adapter_management_manifest.schema.json=1f1b75c7d4299c96fcf5039b652d718f7805cd2d; specs/adapter_management_request.schema.json=2c9c98d0c60603adfaa246f9efa4753bfc0e7597; specs/adapter_management_receipt.schema.json=7b43a2d2b74c5bd0a931ead66f32f55663919b04; specs/adapter_management_list.schema.json=530bc79a6a5117bb29d0de9e0932e3c40d8d785d; specs/capability.schema.json=a243a84b22cdd704875f35afc56e3450f9c9e1ca; native/rust_helper/src/broker/adapter_center.rs=66575bb77e4a375dca4f8488f97df442960bb5a9; native/rust_helper/src/broker/protocol.rs=9b2076d5f31ca6d77f1f87973d1b394aaf751961; native/rust_helper/src/broker/audit.rs=eb1dcf5018d95c7fdf6171aadd4bca9554d1996f; native/rust_helper/src/macos_desktop_owner.rs=31e13a57fcde422878fae63083bd323e07f482ec; apps/desktop_flutter/lib/services/shell_core_client.dart=f08db061bd15649ff7c2fc3fc9e37184433cbf18; apps/desktop_flutter/lib/screens/runtime_center.dart=d2c7af172073d2b10c76a6416b90f011f10e95db; apps/desktop_flutter/lib/models/generated_contracts.dart=64e7a80a308c2734eb18c9d4b26eb50c8354f67b; apps/desktop_flutter/macos/Runner/BrokerProcessChannel.swift=a1269e01608bc194b0f0913a6f96cd489b926155; apps/desktop_flutter/macos/RunnerUITests/AdapterOwnerUITests.swift=eeab11b1123165a8f82da55ebbb014eab8803862; apps/desktop_flutter/test/widget_test.dart=f4e9c7b93f832ba2a2cefee779fd12426f240b04; apps/desktop_flutter/test/authenticated_loopback_broker_transport_test.dart=c511e97f5dfbf3ab55e6c193d939d82517123ff5; apps/desktop_flutter/test/broker_client_payload_hash_test.dart=7e978416326730f639976d0abe4313d6a8e4e2e6; tooling/conformance_tests/run_conformance_skeleton.py=c6f31d8cb561b48513c6784e54c85969c9cb953e。 |
| 試験区分（TestClass） | Rust UNIT／Broker protocol、Flutter widget／loopback transport、Schema・Conformanceのsourceと旧C19記録を確認。本mappingでは製品test未実行。実artifact／installed動作の証拠ではない。 |
| 証拠ID（Evidence ID） | NONE。旧C19の限定Schema／Broker／UI記録を履歴scopeのまま保持し、N03／N05製品Evidenceへ昇格しない。 |
| 阻害要因（Blocker） | 既存Release Registry 18件（16 unresolved／2 resolved）とrelease_ready=falseを維持。旧BlockerのN11個別帰属は別工程で照合し、新規Blockerなし。 |
| 状態 | Phase 27のE0-04全21項目を記録。旧C19の限定範囲を保持し、現N03／N05、K06／K12、E0はOPEN。 |
| 残余 | 外部artifactの取得・実適用・実process lifecycle、独立Runtime TaskとAdapterの接続、五OS installed evidence、N11 Evidence／Blocker／OS Artifact結合。 |

##### Phase 28 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 現行v4 N09／N10、施工線C／G、K13／K16。Desktop operation surfaceの到達性と既存Broker consumer。 |
| 旧ID | D4 Phase 28／C18 Host操作面のDesktop表面、C20 Windows常駐トレイ、C21 Command Palette、C22 Global Search、C23 Desktop UX統合。Phase 26のHost registry・表示contextは重複完了扱いしない。 |
| 原文参照 | v4仕様S04 N09／N10、S05 K13／K16、工程表R03 C／G、施工指示I04-C／I04-G、docs/D4_POCKET_PHASE_MAPPING.md Phase 28、C18／C20〜C23の各責任正本。 |
| 対象 | Windows Tray、Desktop Command Palette、読み取り専用Global Search、4つのNavigation groupと全体表示、Host操作画面への到達。 |
| 目的 | Desktop上の表示・画面遷移とBroker操作を明確に分け、表示状態や検索結果からPermission、Approval、Credential、Authority、Runtime実作用を生成しない。 |
| 生成元（Producer） | 操作者による表示・検索・画面移動要求、Windowsの常駐トレイ通知、Broker由来の状態情報・通知概要・上限付き停止要求記録。 |
| 実Consumer | Win32 tray_controller→Flutterの固定tray channel→windows_tray_client／ShellCoreClient→Broker通常IPC。main.dartはPalette／Search／Navigationを現在画面とShellSnapshotへ結合し、GlobalSearchIndexはsnapshotと固定surface entryを読む。 |
| 契約／Data | windows_tray_projection／tray_stop_request／response Schema、通知summary、Palette command ID、GlobalSearchIndexのbounded metadata、Navigation groupと現在21画面。 |
| 正常時作用 | Trayは前面化・status／保留件数・重大通知件数・終了、Broker経由の停止要求receiptを表示。Paletteは画面遷移、Searchはsnapshot metadataと固定surface entryの絞込み、Navigation groupはRail表示だけを変更する。 |
| 否定時作用 | 取得不能値を0にしない。未検証snapshot出所を信頼しない。検索上限・結果上限・secret／本文除外を維持し、Palette／Search／Navigationから直接Broker、filesystem、process、network、credentialへ到達しない。 |
| 停止／Recovery | TrayのBroker pollはwindow visible中だけに制限し、非表示時に停止・不明化する。Tray停止要求はowner再承認待ちreceiptまでで、kill／実停止ではない。画面遷移や検索から再実行・再認可を起こさない。 |
| 標準OS候補 | 共通Desktop Flutter操作面はWindows／macOS／Linux source候補。native TrayはWindows限定。Mobile NavigationはC24別scope。各OS installed UI evidenceは本source mappingでは未実施。 |
| 配布物 | D4 Pocket DesktopのUI／Windows Runner。独立packageなし。OS別正規Installer、tray常駐、実Task統合、installed同一候補証拠はN10へ接続する。 |
| 既存実装 | Win32 tray controllerとFlutter channel/client、Broker通知／stop receipt、Palette／Search／NavigationのFlutter実装、各surface正本、widget／Conformance sourceを確認。 |
| BUILD_GAP | installed製品での各画面・Tray実操作、同一Task IDに結合した検索／結果／Recovery、実Runtime横断のlive index、全Runtime実停止lifecycle、macOS／Linux native trayは未成立または非対応。C18 registry自体はPhase 26の範囲。 |
| ソース識別値（SourceSHA） | 編集前mainのGit blob: docs/D4_POCKET_PHASE_MAPPING.md=064ea334f86b137d94fba2370d2a2fd732f6e972; docs/specs/windows-tray-surface.md=c7f9317e26b16098301989a8f4b5eaa11540e714; docs/specs/command-palette-surface.md=05867c94592a66b3a0d788d6f29456bb9ba6bc15; docs/specs/global-search-surface.md=1a74b337e239dc7c23cf70d2f6dbc6f55ac5234e; docs/specs/desktop-ux-integration.md=b73ec534c2b4df94ca7eb52be5d7929cf0a20aa1; specs/windows_tray_projection.schema.json=5c3975e03b34281a0f202374cf91127357f2a1e0; specs/tray_stop_request.schema.json=579755262940027079b37ef859e6d2f32a071cc0; specs/tray_stop_response.schema.json=d6cce063d5deaf7104b97ef67f4efb14f780cc72; apps/desktop_flutter/lib/main.dart=6109c821ab5dc809224bec5234c4c59db1d5f988; apps/desktop_flutter/lib/services/global_search_index.dart=f71d2f6c4563b011fa3621ea3aee596bffb099f0; apps/desktop_flutter/lib/services/windows_tray_client.dart=cf8214ceb95fa1b24c7eed4f14b312bda80e0b15; apps/desktop_flutter/lib/services/shell_core_client.dart=f08db061bd15649ff7c2fc3fc9e37184433cbf18; apps/desktop_flutter/windows/runner/tray_controller.cpp=2676afcc76e5c6e53fff2dd00d0fbb3ae531ccc1; apps/desktop_flutter/windows/runner/tray_controller.h=67ec337a335b948115559394ad5b55bffadb7502; apps/desktop_flutter/windows/runner/flutter_window.cpp=67849099e5141b696b237ec7390729793bc0e843; native/rust_helper/src/broker/protocol.rs=9b2076d5f31ca6d77f1f87973d1b394aaf751961; native/rust_helper/src/broker/notification_center.rs=ca6828a33baf94abd7101781d9730b22daca4a78; apps/desktop_flutter/test/windows_tray_client_test.dart=0b24e07221087e43355ac465ca4ace73a48af31e; apps/desktop_flutter/test/global_search_index_test.dart=2b96129f13ab11bd38afb0aba5e9a5b07d8eb3e3; apps/desktop_flutter/test/widget_test.dart=f4e9c7b93f832ba2a2cefee779fd12426f240b04; tooling/conformance_tests/run_conformance_skeleton.py=c6f31d8cb561b48513c6784e54c85969c9cb953e。 |
| 試験区分（TestClass） | 旧C20〜C23のSchema／Conformance／Flutter試験・Debug build履歴を保持。現sourceにはTray／Search／Widget testがある。本mappingでの製品test、installed OS実行はNOT_RUN。 |
| 証拠ID（Evidence ID） | NONE。旧局所証拠はsurface／fixture／debug buildの履歴scopeに保持し、N09／N10 installed acceptanceへ転記しない。 |
| 阻害要因（Blocker） | 既存Registry 18件（16 unresolved／2 resolved）、release_ready=falseを維持。N11でC20〜C23とOS別Blockerを照合し、本mappingでは追加しない。 |
| 状態 | Phase 28のE0-04全21項目を記録。C18 Host surfaceのPhase 26との重複を分離し、現N09／N10、K13／K16、E0はOPEN。 |
| 残余 | Windows installed tray／UI操作、各OSの製品UI・実Task統合、同一Task result／Audit／Recoveryの横断consumer、N11 Evidence／Blocker／OS Artifact結合。 |

##### Phase 29 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 現行v4 N05、施工線D、K14。GUI-Shell構成のManifest生成と既存Catalog／Tool consumerへの接続。 |
| 旧ID | D4 Phase 29「GUI-Shell構成作成」。総合機能拡張rev1に同一の独立C工程はないD4追加範囲。 |
| 原文参照 | v4仕様S04 N05、S05 K14、工程表R03 D、施工指示I04-D、docs/D4_POCKET_PHASE_MAPPING.md Phase 29、docs/specs/gui-shell-compose.md。 |
| 対象 | Desktop SettingsからRuntime／Agent／Tool／MCP参照ID、Theme、表示設定、Capability requirementをCompose Manifestへまとめ、Brokerで検証する。 |
| 目的 | 構成意図を機械可読Manifestへまとめ、Authority／Permission／Approval／Credential／Audit chainを継承せず、未確認Catalog宣言を実行権限へ昇格しない。 |
| 生成元（Producer） | 使用者のDesktop設定入力とBroker Compose Catalogの現在Runtime／Agent／互換情報。Tool／MCP参照IDやCapability requirementは入力された申告値。 |
| 実Consumer | Settings→ComposeClient→認証Broker IPC→compose_centerのCatalog／Manifest検証／receipt。mcp_centerはTool要求に添付されたManifestを選択照合し、export_centerは別の書出し要求でManifestを解析する。Compose UIから実Tool作用・独立App生成まで一体acceptanceは未成立。 |
| 契約／Data | gui_shell_compose／catalog／receipt、capability、mcp_tool_call、gui_shell_export Schema。output_mode=manifest_only、継承policyはすべてnone。Module selectionは別Export要求。 |
| 正常時作用 | Brokerが現在CatalogとのRuntime／Agent対応、重複、識別子、互換性、Capability要件、表示範囲、継承禁止を再検査し、INTERNAL_STATEの監査付きCompose receiptとManifestを返す。 |
| 否定時作用 | malformed／unknown field、重複選択、Runtime／Agent対応不一致、互換性・Capability不整合、Authority等の継承値を拒否する。入力IDの記載だけで実在・接続・Trustを成立させない。 |
| 停止／Recovery | Catalog mismatch、Broker／Audit失敗、構造不正ではManifest acceptanceを返さない。Previewは差分表示だけで、構成適用、Build、Rollback、Export、Tool再実行を自動しない。 |
| 標準OS候補 | Desktop Flutter／Rust Brokerの共通source候補はWindows／macOS／Linux。ComposeのMobile consumerは未確認。Linux native必須性は維持するが、source存在をnative installed受入れにしない。 |
| 配布物 | D4 Pocket DesktopのSettingsとBroker。現在の出力はManifestのみで、Standalone App、Installer、Binary、独立identityを生成しない。 |
| 既存実装 | Broker Compose Catalog／Manifest検証／Preview source、Settings／ComposeClient、MCP Tool要求時のManifest照合、Export要求側Manifest parser、Schema／fixture／Flutter testを確認。 |
| BUILD_GAP | 入力画面はCatalog候補を発見・実在確認せず参照IDを受け取る。実Capability dependency resolutionから構成消費、実Tool作用、Task結果／Auditまでの統合経路と独立Appへの構成反映は未成立。 |
| ソース識別値（SourceSHA） | 編集前mainのGit blob: docs/D4_POCKET_PHASE_MAPPING.md=064ea334f86b137d94fba2370d2a2fd732f6e972; docs/specs/gui-shell-compose.md=6b43bb80f73d4268e734ac0afbc58b33ba07339f; specs/gui_shell_compose.schema.json=89b3d8e0e09c802d555fd6fee233daebc0fec045; specs/gui_shell_compose_catalog.schema.json=f9c76f5fa87bfb0866f33745dcd5a8a4e74864d5; specs/gui_shell_compose_receipt.schema.json=7c4ac9d0f28f38f79a0074ed7d6c84d9124aadaa; specs/capability.schema.json=a243a84b22cdd704875f35afc56e3450f9c9e1ca; specs/mcp_tool_call.schema.json=54c044cdfb53588ea5bab4bd056404372dc39acd; specs/gui_shell_export.schema.json=8b96d344ac0f6a54011237be8b2d85964be48c4b; native/rust_helper/src/broker/compose_center.rs=d80658fd9a342140acc35560b2e4c4e4ae97d888; native/rust_helper/src/broker/mcp_center.rs=8f9ab897bdc625e448241ebd9b46f995cd1f22f6; native/rust_helper/src/broker/export_center.rs=477e44d86ebf18d2b004a7294c9cdb14e35aa0f0; native/rust_helper/src/broker/protocol.rs=9b2076d5f31ca6d77f1f87973d1b394aaf751961; apps/desktop_flutter/lib/services/compose_client.dart=8f9657d866b007a4d2212abefca4df9add81a702; apps/desktop_flutter/lib/services/shell_core_client.dart=f08db061bd15649ff7c2fc3fc9e37184433cbf18; apps/desktop_flutter/lib/screens/settings.dart=66374ec4157d573939e7f9cc5c36aecf5d65e251; apps/desktop_flutter/lib/services/export_client.dart=70d77edc6beb9e209e6b362219d1b22d69375489; apps/desktop_flutter/test/compose_client_test.dart=c3fe30faa668482e5a4e42b2f5f5e5b218a01dc8; apps/desktop_flutter/test/settings_compose_test.dart=a78353f93617cc7b17391549ea256f007a7acdb1; apps/desktop_flutter/test/widget_test.dart=f4e9c7b93f832ba2a2cefee779fd12426f240b04; examples/contracts/gui_shell_compose.valid.json=d72bf3e84f73bf906106ce97d8df7749475fea74; examples/contracts/invalid/gui_shell_compose_authority.invalid.json=3dc3b2fa9b1330739b8b7aa4468dede5f83da30c; examples/contracts/invalid/mcp_tool_call_compose_authority.invalid.json=44bf1b10e691283f88324858a762a7b03386b9fb; tooling/conformance_tests/run_conformance_skeleton.py=c6f31d8cb561b48513c6784e54c85969c9cb953e。 |
| 試験区分（TestClass） | Rust UNIT（Compose／MCP／Export contract）、Flutter Fake Broker／widget、Schema／fixture／Conformance sourceを確認。本mappingでは製品Rust／Flutter／Tool runtime試験を実行していない。 |
| 証拠ID（Evidence ID） | NONE。Manifest／fixture／unit履歴をN05実使用Acceptanceへ転記しない。 |
| 阻害要因（Blocker） | 既存Release Registry 18件（16 unresolved／2 resolved）とrelease_ready=falseを保持。N11の現要求／OS別帰属確認前のBlocker追加なし。 |
| 状態 | Phase 29のE0-04全21項目を記録。D4追加Compose scopeを旧C工程へ混同せず、現N05／K14／E0はOPEN。 |
| 残余 | 実Catalogの網羅、Capability dependency／permission requirement照合、Composeから元Toolへの実作用、Manifest構成の独立Product消費、五OS installed QA、N11結合。 |

Phase 27〜29の3行は各E0-04全21項目を記録し、68件（57 unique path）のSourceSHA参照を編集前main blobへ結合した。旧C19〜C23 status／証拠、現N03／N05／N09／N10、K06／K12／K13／K14／K16、Registry 18件（16 unresolved／2 resolved）、release_ready=falseを変更せず、新Evidence／Blockerなし。Phase 28のC18 Host surfaceはPhase 26のregistry／context範囲と分離した。製品Rust／Flutter／installed Adapter／Tray／Compose Tool実作用、五OS QAは本mapping単位でNOT_RUN。

##### Phase 30 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 現行v4 N05、施工線D、K14。構成Manifestの一時的な差分／rollback previewに限定する。 |
| 旧ID | D4 Phase 30「構成差分確認」。総合機能拡張rev1に同一の独立C工程はないD4追加範囲。Phase 29のManifest作成と区別する。 |
| 原文参照 | v4仕様S04 N05／S05 K14、工程表R03 D、施工指示I04-D、docs/D4_POCKET_PHASE_MAPPING.md Phase 30、docs/specs/gui-shell-compose.md。 |
| 対象 | Settingsで直近にBrokerが受理したManifest（未受理ならnull）と候補Manifestを比較し、BrokerがPreview結果を返すDesktop経路。 |
| 目的 | 構成差分と要求機能の表示に限り、保存済み構成・承認・復旧元・実Build・Rollbackを成立したように扱わない。 |
| 生成元（Producer） | Desktop Settingsの候補入力と画面が保持する直近受理Manifest。Preview要求はcurrent／candidate Manifest、target platform、version_rollback modeを含む。 |
| 実Consumer | Settings→ComposeClient→認証Broker IPC→compose_center Preview parser／差分計算→receipt→Settings表示。現行経路に永続構成履歴・版保存consumerはない。 |
| 契約／Data | gui_shell_preview／gui_shell_preview_receipt Schema、Compose Manifest。current_manifest／candidate_manifest、build_target、preview_mode。これは一時Preview要求である。 |
| 正常時作用 | Brokerが両Manifestを検査し、決定論的な変更・機能要件のPreviewを返す。build／exportは未開始、version rollbackは利用不能の状態を返す。 |
| 否定時作用 | 不正構造・未知field・Authority継承・不正なPreview要求を拒否し、差分を受理・適用したと表示しない。 |
| 停止／Recovery | Broker拒否・失敗時はPreview表示を止め、構成を変更しない。PreviewからBuild、Export、Rollback、Tool再実行は起動しない。永続履歴がないため復元処理は提供しない。 |
| 標準OS候補 | Desktop Flutter／Rust Brokerのsource候補はWindows／macOS／Linux。Compose Mobile consumerは確認できない。source候補を各OS installed Previewの証拠へ昇格しない。 |
| 配布物 | 通常のD4 Pocket Desktop SettingsとBroker。独立App、Installer、binary、独立identityは生成しない。 |
| 既存実装 | ComposeClient Preview呼出し、Settingsの候補表示、Rust Brokerの差分計算、Preview Schema／valid・negative examples、Rust／Flutter test sourceを確認。 |
| BUILD_GAP | Preview基準は画面内の直近受理Manifestで永続revisionではない。実構成の保存・復旧、実Capability dependency解決、Composeから元Toolの実作用までの一体経路は別途未成立。 |
| ソース識別値（SourceSHA） | 編集前main `b7bf4c6e41bd9b5a919a11a0bc9ab189a0f14418`のGit blob: docs/D4_POCKET_PHASE_MAPPING.md=064ea334f86b137d94fba2370d2a2fd732f6e972; docs/specs/gui-shell-compose.md=6b43bb80f73d4268e734ac0afbc58b33ba07339f; specs/gui_shell_preview.schema.json=c8aa1c4b54f3395e603fbaae143ac9d8005c4659; specs/gui_shell_preview_receipt.schema.json=dc5832d7a2fc4f74a642e032a53b4ddab53052bb; specs/gui_shell_compose.schema.json=89b3d8e0e09c802d555fd6fee233daebc0fec045; examples/contracts/gui_shell_preview.valid.json=6d827b9af16e3ae416b0ae6aafcd4b1ef80c1ea7; examples/contracts/gui_shell_preview_receipt.valid.json=15a1e66fb02f014a7cf2f182823f91d244028b51; examples/contracts/invalid/gui_shell_preview_authority.invalid.json=1a098bd1a61e64e525f11b08dba6d5f9f5ad77c8; examples/contracts/invalid/gui_shell_preview_receipt_authority.invalid.json=dd093860c1aadbe56118acc49d2266054f1caa9c; native/rust_helper/src/broker/compose_center.rs=d80658fd9a342140acc35560b2e4c4e4ae97d888; native/rust_helper/src/broker/protocol.rs=9b2076d5f31ca6d77f1f87973d1b394aaf751961; apps/desktop_flutter/lib/services/broker_client.dart=122de14350dfe1f160c7f5f911dd1e1d56da8233; packages/broker_ipc_client/lib/src/authenticated_loopback_broker_transport.dart=9ac3b6e0c1bed45e6c373e683c875867ac55ab56; apps/desktop_flutter/test/authenticated_loopback_broker_transport_test.dart=c511e97f5dfbf3ab55e6c193d939d82517123ff5; apps/desktop_flutter/test/broker_client_payload_hash_test.dart=7e978416326730f639976d0abe4313d6a8e4e2e6; apps/desktop_flutter/lib/services/compose_client.dart=8f9657d866b007a4d2212abefca4df9add81a702; apps/desktop_flutter/lib/screens/settings.dart=66374ec4157d573939e7f9cc5c36aecf5d65e251; apps/desktop_flutter/test/compose_client_test.dart=c3fe30faa668482e5a4e42b2f5f5e5b218a01dc8; apps/desktop_flutter/test/settings_compose_test.dart=a78353f93617cc7b17391549ea256f007a7acdb1; apps/desktop_flutter/test/widget_test.dart=f4e9c7b93f832ba2a2cefee779fd12426f240b04; tooling/conformance_tests/run_conformance_skeleton.py=c6f31d8cb561b48513c6784e54c85969c9cb953e。 |
| 試験区分（TestClass） | Rust `compose_center.rs`の決定論的差分計算・復元不可判定・Authority継承と未知項目拒否を扱う単体（`UNIT`）、Flutter ComposeClient／Settingsの模擬Broker（`FIXTURE`／`WIDGET`）、Schema正常・否定例（`FIXTURE`）、適合確認実装（`SOURCE`）を確認。今回のmappingでは再実行していない。 |
| 証拠ID（Evidence ID） | `NONE`。Preview Schema・fixture・UNIT／Widget sourceは現在のN05 installed Tool acceptanceへ転記しない。 |
| 阻害要因（Blocker） | Registry 18件（16 unresolved／2 resolved）、`release_ready=false`を保持する。既存IDの現行scope帰属を変えず、新規Blockerなし。 |
| 状態 | Phase 30のE0-04全21項目を記録。N05／K14／E0と製品Acceptanceは`OPEN`。 |
| 残余 | 永続構成版・実Rollback、全Capability依存の実解決、元Toolへの接続、各OS installed動作とN11証拠結合。 |

##### Phase 31 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 現行v4 N07、施工線D、K14。AI Edit提案を別Taskの検査・controlled applyへ接続する要求。 |
| 旧ID | D4 Phase 31「AI編集提案」。D4追加範囲で、旧P8の製品構築CLOSEDを現N07の成立へ移さない。 |
| 原文参照 | v4仕様S04 N07／S05 K14、工程表R03 D・W05/G5、施工指示I04-D、docs/D4_POCKET_PHASE_MAPPING.md Phase 31、docs/specs/gui-shell-ai-edit.md。 |
| 対象 | Owner／Developerが明示開始するSettings proposal、Broker receipt／Audit、確認後にAgent Centerの通常Task事前検査へ未信頼contextを渡す経路。 |
| 目的 | 提案と適用を分離し、別実行主体の検査・新規Permission／Approval・Broker管理の変更照合を経ない自己承認や自己変更を許さない。 |
| 生成元（Producer） | 明示開始された編集指示、scope、target paths、expected changesとproposal hash。Agent／proposal／result metadataは未信頼入力で、権限源にしない。 |
| 実Consumer | Settings→AiEditClient→Owner-controlled Rust Broker `ai_edit_center`がproposal receipt／Auditを作成。Owner確認後、`agent_coordination`／`agent_task_client`が異なるWorkspace／未使用Sessionの通常Task事前検査へ送る。専用apply consumerはない。 |
| 契約／Data | gui_shell_edit_proposal／receipt Schema、Agent Task要求／provenance。`proposal_only`、`review_required=true`、`self_approval=false`、files_written=false固定。 |
| 正常時作用 | Brokerはpath／scope／規約／ID／hashを検査し、本文をreceiptへ返さずAudit付き提案を受理する。使用者確認後の別Task準備はfresh Permission／Approvalを継承せず通常Broker経路へ返す。 |
| 否定時作用 | Owner要件不成立、禁止path／secret、unknown field、self approval、proposal以外のmode、hash／provenance不一致、使用済みSessionや履歴照合失敗を拒否する。proposalを適用済みにしない。 |
| 停止／Recovery | Audit失敗、古い／不一致snapshot、Content Exposure不足、Task事前検査／認可不成立ではTask開始・書込前に停止。開始後は通常Task Recoveryを使うが、専用apply transaction／rollbackはない。 |
| 標準OS候補 | Windows／macOS／Linux Desktopの共有UI／Rust Broker source候補。Mobile UI／consumerは確認できず、各OS installed経路は未実証。 |
| 配布物 | D4 Pocket Desktop内のDeveloper向けproposal／Task準備面。独立AppやRuntime自己編集機能は生成しない。 |
| 既存実装 | Owner-only Rust proposal／Audit、Flutter receipt検査と確認Dialog、Agent Center通常Taskの未信頼context／別Workspace事前検査、Schema／negative examples／UNIT／Widget sourceを確認。 |
| BUILD_GAP | 独立レビュー結果を適用前に確定・Brokerが照合するconsumer、proposal由来diffを限定適用するBroker operation、結果の実ファイル照合と専用rollback transactionが不足。 |
| ソース識別値（SourceSHA） | 編集前main `b7bf4c6e41bd9b5a919a11a0bc9ab189a0f14418`のGit blob: docs/D4_POCKET_PHASE_MAPPING.md=064ea334f86b137d94fba2370d2a2fd732f6e972; docs/specs/gui-shell-ai-edit.md=33d2df562d2231d0d894afcbf6097c512a8efd8b; specs/gui_shell_edit_proposal.schema.json=b1a0b41570d5e1e1eadbf2ded3e8984d9fe21130; specs/gui_shell_edit_proposal_receipt.schema.json=d9f426e20254c8cf009b90e7be4cbccf453db286; examples/contracts/gui_shell_edit_proposal.valid.json=055f5b5b2d98cc755a725640122489d2408ee7f3; examples/contracts/gui_shell_edit_proposal_receipt.valid.json=3515521b9bc85bf666d98ff78e922967c60fd3e7; examples/contracts/invalid/gui_shell_edit_proposal_receipt_apply.invalid.json=6a679ba144c7dae8023c1718c984dbf545bcf5e1; examples/contracts/invalid/gui_shell_edit_proposal_receipt_missing_count.invalid.json=061c93a5dc590f79a8ce7191afa3a6f4feb3ccc3; examples/contracts/invalid/gui_shell_edit_proposal_self_approval.invalid.json=958989e4be92acc4b482824dd16bb5628f7b0dda; native/rust_helper/src/broker/ai_edit_center.rs=7a1c2418dc6f71e14fc971385b6f4f2454d55b1c; native/rust_helper/src/broker/protocol.rs=9b2076d5f31ca6d77f1f87973d1b394aaf751961; apps/desktop_flutter/lib/services/broker_client.dart=122de14350dfe1f160c7f5f911dd1e1d56da8233; packages/broker_ipc_client/lib/src/authenticated_loopback_broker_transport.dart=9ac3b6e0c1bed45e6c373e683c875867ac55ab56; apps/desktop_flutter/test/authenticated_loopback_broker_transport_test.dart=c511e97f5dfbf3ab55e6c193d939d82517123ff5; apps/desktop_flutter/test/broker_client_payload_hash_test.dart=7e978416326730f639976d0abe4313d6a8e4e2e6; native/rust_helper/src/desktop_launcher.rs=6a9f335de07fa474ca10043aea63ad6e988b572d; apps/desktop_flutter/lib/services/ai_edit_client.dart=ba5e62311a02749d2f9fe0e4e72e2b0d4e0a497d; apps/desktop_flutter/lib/services/agent_coordination.dart=bdc13464f0e42b740a2bca3b7e8828a5019056d0; apps/desktop_flutter/lib/services/agent_task_client.dart=f54cefac29eba3dcf383bba19b4f64ee682e6217; apps/desktop_flutter/lib/screens/settings.dart=66374ec4157d573939e7f9cc5c36aecf5d65e251; apps/desktop_flutter/test/ai_edit_client_test.dart=c5ecd0353fdc811ec7c4d52a9c2b7d10e1d01723; apps/desktop_flutter/test/agent_coordination_test.dart=6783e87a492d8436a855a7088b7cece38d122bd5; apps/desktop_flutter/test/agent_task_client_test.dart=0477a0bbbb321082494b6b55711815d34419fdd9; apps/desktop_flutter/test/widget_test.dart=f4e9c7b93f832ba2a2cefee779fd12426f240b04; tooling/conformance_tests/run_conformance_skeleton.py=c6f31d8cb561b48513c6784e54c85969c9cb953e。 |
| 試験区分（TestClass） | Rust `ai_edit_center.rs`のOwner限定・提案限定・対象path拒否を扱う単体（`UNIT`）、Flutter AI Edit／Agent Task／連携処理の模擬Broker・状態固定試験（`FIXTURE`／`WIDGET`）、Schema正常・否定例（`FIXTURE`）、適合確認実装（`SOURCE`）を確認。本mappingでは未実行。独立Agent審査／適用／復元の導入済み製品試験は確認できない。 |
| 証拠ID（Evidence ID） | `NONE`。既存proposal／Task-preflight sourceやfixtureを独立review・controlled apply acceptanceへ転記しない。 |
| 阻害要因（Blocker） | Registry 18件（16 unresolved／2 resolved）、`release_ready=false`を保持する。既存Blockerを維持し、新規追加なし。 |
| 状態 | Phase 31のE0-04全21項目を記録。N07／K14／W05/G5／E0と製品Acceptanceは`OPEN`。 |
| 残余 | 別実行主体による審査結果束縛、Broker-controlled apply・rollback、実diff照合、各OS installed経路とN11証拠結合。 |

##### Phase 32 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 現行v4 N06、施工線E、K15、W05/G5。Windows独立ExportをN10の通常Installer／配布工程およびPhase 33のPruningと分離する。 |
| 旧ID | D4 Phase 32「Windows独立App書出し」。旧P9 Standalone Exportと関連するが、P9のProduct Build CLOSEDは現N06 Acceptanceへ移さない。 |
| 原文参照 | v4仕様S04 N06／S05 K15、工程表R03 E・W05/G5、施工指示I04-E、docs/D4_POCKET_PHASE_MAPPING.md Phase 32、docs/specs/gui-shell-export.md。 |
| 対象 | Desktop Export要求→Brokerの新規App／Audit-store identity付きManifest出力→Windows Developer Exporter／portable ZIP→launcher候補。 |
| 目的 | 独立構成の機械可読Manifestとidentityを安全に生成し、Credential／Authorityを継承せず、Manifest生成・開発bundleをinstalled Productや配布へ誤認しない。 |
| 生成元（Producer） | Compose Manifest、明示されたWindows target／optional Module選択、Rust native Owner確認。Receipt／ManifestはBrokerが再検証・生成する。 |
| 実Consumer | Settings→ExportClient→native Owner確認→認証Broker IPC→`export_center`が固定Export directoryへManifest fileを生成しAuditへ結合。Developer-only `export_windows_product.py`／`package_windows_portable.py`がreceipt・Manifestを検証し未署名bundle／ZIPを組む。Launcher sourceは起動前Manifest hash／identity検査を持つ。 |
| 契約／Data | gui_shell_export／export_manifest／export_receipt／module_catalog／windows_export_build Schema。`manifest_file`、Windows target、non-inheritance、`binary_pruning_status=not_applied`。Build evidenceは`runtime_manifest_consumed=false`、`standalone_app_verified=false`。 |
| 正常時作用 | Brokerが新規App ID／将来用Audit-store IDと設定・Runtime／Adapter・Module planを含むManifestを固定領域へ保存しReceipt／Auditへhashを結ぶ。開発Exporterは未署名portable artifactを生成・検証するsource経路を持つ。 |
| 否定時作用 | target／Module／依存不整合、unknown field、Authority継承、Credential、Manifest／artifact hash不一致、symlink／不正inventoryを拒否する。module planを実除去と報告しない。 |
| 停止／Recovery | Broker Audit失敗は成功Receiptを返さずSuspended、未確定fileの照合を要求する。temporary cleanup失敗はpendingとして記録。Exporter／ZIP検査失敗時は完成・署名・配布と表示せず、既存成果物を上書きしない。 |
| 標準OS候補 | 本PhaseはWindows host／Desktop target。Linux／macOSのExporter候補は別scope。Windows source／manual workflow存在は通常installed Windows製品の証拠ではない。 |
| 配布物 | 現sourceが生成可能なのは開発用・未署名portable bundle／ZIP。一般利用installer、正式Publisher署名・配布、更新・rollbackは未成立。 |
| 既存実装 | Rust Broker Export／fixed-directory保存、Flutter Export UI、Windows Developer Exporter／ZIP packager、launcher Manifest hash／identity validation、Schema／negative fixtures／Broker・Flutter testsを確認。 |
| BUILD_GAP | installed独立AppがManifestを実消費して実Task／Auditを行う候補、通常Installerとlifecycle、独立Product acceptanceは未成立。実binary／resource pruning・性能測定はPhase 33の別工程。 |
| ソース識別値（SourceSHA） | 編集前main `b7bf4c6e41bd9b5a919a11a0bc9ab189a0f14418`のGit blob: docs/D4_POCKET_PHASE_MAPPING.md=064ea334f86b137d94fba2370d2a2fd732f6e972; docs/specs/gui-shell-export.md=3d7a1156e4bf095e4f6a64792b6e1683c14f9b17; docs/specs/gui-shell-module-pruning.md=5568c677be30402d709cc365b79eceb0e96ce831; specs/gui_shell_export.schema.json=8b96d344ac0f6a54011237be8b2d85964be48c4b; specs/gui_shell_export_manifest.schema.json=bf136b1a1daa258d33af3635ee414a2d4312bec5; specs/gui_shell_export_receipt.schema.json=0448ec5f631509fa6fbe25cb4957bd6cf51e1562; specs/gui_shell_module_catalog.schema.json=bd54dd7903a1734ea4027d3f780b7a2b49e10c8e; specs/gui_shell_windows_export_build.schema.json=5c7234692a8f178882edfc42c1a762038e7414fa; examples/contracts/gui_shell_export.valid.json=ea10286920a82f019ce9ee8b1e8cc4cf710adde4; examples/contracts/gui_shell_export_manifest.valid.json=8e845373fdfcbfd77f555d3030ef3d35b3dd4ed4; examples/contracts/gui_shell_export_receipt.valid.json=fae30c21b719d2ae9bf9bcff99228bf6eb6b0df1; examples/contracts/gui_shell_windows_export_build.valid.json=31915727f6f0899bf4358996f709a23a76e9554d; examples/contracts/invalid/gui_shell_export_unknown_module.invalid.json=ae5a354654aaddf15821eb64a90f29c00a2fcc0e; examples/contracts/invalid/gui_shell_export_manifest_shape.invalid.json=1eb6811c357a161b357a192cbc1bf430b5b42901; examples/contracts/invalid/gui_shell_export_credential.invalid.json=8c03e7f4ae329271a1b78a6869c8ae4050d7d164; examples/contracts/invalid/gui_shell_export_receipt_inherited.invalid.json=52faca660ce45470ef5ce5136a9946181ebbdbb6; examples/contracts/invalid/gui_shell_export_receipt_pruning_claim.invalid.json=6766cfe13dd417fe762cb30dddeb2644a14fb354; examples/contracts/invalid/gui_shell_windows_export_build_authority.invalid.json=b29a306a0fe8e4e2061730ade3b69071fe41f80c; native/rust_helper/src/broker/export_center.rs=477e44d86ebf18d2b004a7294c9cdb14e35aa0f0; native/rust_helper/src/broker/compose_center.rs=d80658fd9a342140acc35560b2e4c4e4ae97d888; native/rust_helper/src/broker/protocol.rs=9b2076d5f31ca6d77f1f87973d1b394aaf751961; apps/desktop_flutter/lib/services/broker_client.dart=122de14350dfe1f160c7f5f911dd1e1d56da8233; packages/broker_ipc_client/lib/src/authenticated_loopback_broker_transport.dart=9ac3b6e0c1bed45e6c373e683c875867ac55ab56; apps/desktop_flutter/test/authenticated_loopback_broker_transport_test.dart=c511e97f5dfbf3ab55e6c193d939d82517123ff5; apps/desktop_flutter/test/broker_client_payload_hash_test.dart=7e978416326730f639976d0abe4313d6a8e4e2e6; native/rust_helper/src/desktop_launcher.rs=6a9f335de07fa474ca10043aea63ad6e988b572d; native/rust_helper/src/bin/gui_shell_desktop_launcher.rs=809efba7a03eacddf9e172011149d9a8d5e3fda7; apps/desktop_flutter/lib/services/export_client.dart=70d77edc6beb9e209e6b362219d1b22d69375489; apps/desktop_flutter/lib/screens/settings.dart=66374ec4157d573939e7f9cc5c36aecf5d65e251; apps/desktop_flutter/test/export_client_test.dart=b4ec8686b91370b1eaff06dd88b301ee6ef0370c; apps/desktop_flutter/test/settings_compose_test.dart=a78353f93617cc7b17391549ea256f007a7acdb1; tooling/export_windows_product.py=31856179881a5a5aae1ba4451732e6de6cd7f338; tooling/package_windows_portable.py=d6cfba6941da10fbae35228e7137d7f87f8d3889; tooling/conformance_tests/run_conformance_skeleton.py=c6f31d8cb561b48513c6784e54c85969c9cb953e。 |
| 試験区分（TestClass） | Rust Export Broker単体（`UNIT`）、Flutter ExportClientの模擬Broker試験（`FIXTURE`）、Schema正常・否定例（`FIXTURE`）、適合確認実装とWindows Export組立・ZIP梱包tool（`SOURCE`）を確認。追跡したsource file群では専用Windows Exporter試験／導入済み製品の端から端までの試験を確認できない。本mappingでは全試験未実行。 |
| 証拠ID（Evidence ID） | `NONE`。過去P9の限定portable buildやsource evidenceを現N06 installed／通常配布Acceptanceへ転記しない。 |
| 阻害要因（Blocker） | Registry 18件（16 unresolved／2 resolved）と`release_ready=false`を維持する。関連するExport Owner UI／Desktop distribution／Module pruning blockerは既存IDのまま保持し、新規Blockerなし。 |
| 状態 | Phase 32のE0-04全21項目を記録。旧P9の履歴を維持し、現N06／N10／K15／W05/G5／E0と製品Acceptanceは`OPEN`。 |
| 残余 | Windowsの通常installed独立Productと実Task、Installer／更新／撤去、Manifest runtime consumption、正式配布identity／署名、別工程Phase 33の実pruning、N11証拠結合。 |

Phase 30〜32の3行は各E0-04全21項目を記録し、80件（60 unique path）のSourceSHA参照を編集前main `b7bf4c6e41bd9b5a919a11a0bc9ab189a0f14418`のGit blobへ結合した。Previewの差分表示、proposal-onlyから通常Task事前検査への受渡し、Broker Manifest書出しとDeveloper bundle生成を区別し、実Rollback／controlled apply／installed独立製品へ昇格しない。旧P8/P9状態、N05/N06/N07/N10、W04/W05、Registry 18件（16 unresolved／2 resolved）、`release_ready=false`を保持し、新Evidence／Blockerなし。Phase 33 Module pruningとWindows通常配布のPhase 34を分離した。製品Rust／Flutter／Exporter／installed runtime試験、五OS QAは本mapping単位でNOT_RUN。

##### Phase 33 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 現行v4 N06、施工線E、K15、W05/G5、FQ-MODULE-PRUNING。Phase 32のManifest／bundle生成、Phase 34の通常配布・Installerと区別する。 |
| 旧ID | D4 Phase 33「機能単位選択・除去」。旧P10 Module Selection／Pruningの機能上除外CLOSED履歴と関連するが、D4 Phase 33はrev1に同一の独立C工程がない追加範囲。 |
| 原文参照 | v4仕様S04 N06／S05 P9・P10対応、工程表R03 E・FQ-MODULE-PRUNING、施工指示I04-E・I02-04、docs/D4_POCKET_PHASE_MAPPING.md Phase 33、docs/specs/gui-shell-module-pruning.md。 |
| 対象 | Desktop Export設定の任意Module選択→Broker ModulePlan→Developer専用Windows／Linux Export build→Flutter画面surface／AOT比較。最終独立Productの除去・稼働保証とは別。 |
| 目的 | 選択Moduleと依存閉包をManifestへ正確に記録し、必須画面／安全Coreを保持する。compile-time選択やAOT reportを実binary／resource pruning、実行挙動またはRelease合格と誤認しない。 |
| 生成元（Producer） | Settingsの8 optional Module checkbox（初期状態は全選択）とExportClientの`module_selection.optional_module_ids`。Broker Catalogが10 required UI Module、8 unprunable Core、8 optional Moduleと依存を評価し、要求・包含・除外計画を生成する。 |
| 実Consumer | Flutter Settings→ExportClient→Owner-only Broker `export_center`→Receipt `module_plan`。Developer toolは検証済みの未信頼Receipt／Manifest計画をWindows／Linux Flutter `--dart-define`へ射影し、`main.dart`のcompile-time boolがNavigation／screen consumerを条件表示する。Windows同一commit比較toolはall-enabled／selected AOT reportとartifact hashを検査する。 |
| 契約／Data | `gui_shell_module_catalog`、Export／Receipt／Manifest、Windows/Linux Export build、Module build／comparison evidence Schema。planは`selection_mode`・requested／included／excluded IDs・dependency closureを記録し、Receiptは`binary_pruning_status=not_applied`、build／comparison evidenceは`binary_pruning_verified=false`、runtime未検証を固定する。 |
| 正常時作用 | 不指定時は後方互換で全optionalを選択。明示選択時はBrokerが依存を再帰包含し、必須10 UI／unprunable Core 8を維持したplanをReceiptへ結合する。Windows／Linux Developer buildは選択に対応するFlutter surface defineとAOT report／artifact hashを記録する。 |
| 否定時作用 | unknown／duplicate／required Module、依存先欠落・循環・不整合、固定必須/Core集合の欠落、壊れたManifest／Receipt、選択とAOT surfaceの不一致を拒否する。入力metadata／receiptからAuthority・Permission・Approval・Credentialを生成しない。 |
| 停止／Recovery | Brokerのcatalog／plan／Audit不成立はExport受理を停止する。build・Manifest hash・AOT解析の不一致時は有効なevidence／packageを作成せずfail-closed。Developer build失敗は独立製品やpruning成功を示さない。実Product rollback／起動回復はPhase 34以降の別consumerであり本sourceに接続しない。 |
| 標準OS候補 | Desktop Windows／macOS／Linuxの共有Settings／Flutter surface source候補。専用選択build／比較はWindows、独立Developer Export／AOT surface buildはWindows／Linux。macOSの同等Export consumerとMobile Module Exportは確認できず、各OS installed受入れではない。 |
| 配布物 | Broker Receipt／Manifest計画、および未署名のDeveloper Flutter／portable Export artifacts。通常Installer、正式署名／Publisher配布、実pruned independent productは本Phaseの成果物として確認できない。 |
| 既存実装 | Rust Broker catalog・dependency closure・required/Core固定検査、Flutter checkbox／payload／compile-time surface gates、Windows Developer build／same-commit comparison、Windows／Linux Export AOT report、schema／negative fixtures／Rust・Widget・Linux tooling・Conformance test sourceを確認。過去Windows AOT比較とLinux Setup導入履歴は限定scopeを保持する。 |
| BUILD_GAP | Manifest選択から最終独立製品binary／resourceへの完全な除去、Rust／第三者依存のpruning、安全Coreの最終artifact保持と必要機能の実動作を一体確認するproduction consumerが未成立。Cold startup／実行時resource比較も未測定。 |
| ソース識別値（SourceSHA） | 編集前main `5c7e13d453cba99b3907ac3b06b0388657b850de`のGit blob: docs/D4_POCKET_PHASE_MAPPING.md=064ea334f86b137d94fba2370d2a2fd732f6e972; docs/specs/gui-shell-module-pruning.md=5568c677be30402d709cc365b79eceb0e96ce831; docs/specs/gui-shell-export.md=3d7a1156e4bf095e4f6a64792b6e1683c14f9b17; docs/REV2_PROGRESS.md=56f0f168b22d1e64303db2bc6f2c66668393dba2; docs/REV5_PRODUCT_PROGRESS.md=9c978021d3982690e9aaa87896436bad7125b433; docs/FINAL_QA_QUEUE.md=7b332cccab70bb68aa1ff4f8e23acc0e816d8282; release_blockers.registry.json=244adc04a01e7b406c4ebcdeb829f7077d4aa35f; specs/gui_shell_module_catalog.json=339cb8894b25ae963956e582c42b9e8fcc0d3931; specs/gui_shell_module_catalog.schema.json=bd54dd7903a1734ea4027d3f780b7a2b49e10c8e; specs/gui_shell_export.schema.json=8b96d344ac0f6a54011237be8b2d85964be48c4b; specs/gui_shell_export_receipt.schema.json=0448ec5f631509fa6fbe25cb4957bd6cf51e1562; specs/gui_shell_export_manifest.schema.json=bf136b1a1daa258d33af3635ee414a2d4312bec5; specs/gui_shell_module_build_evidence.schema.json=1a37d4b831f998ce334fc9f0126c5dd2ee09a01b; specs/gui_shell_module_comparison_evidence.schema.json=801ba33bfc0656a4048aa5ca1920b96b7787c304; specs/gui_shell_windows_export_build.schema.json=5c7234692a8f178882edfc42c1a762038e7414fa; specs/gui_shell_linux_export_build.schema.json=6f1af3d3da2277807dabac5d753897f5ddee453c; examples/contracts/gui_shell_module_catalog.valid.json=6d5902928fb3edac491e6c56a982f5826c9c4b93; examples/contracts/gui_shell_module_build_evidence.valid.json=55aaf255258e67258a247c46acabcea451101ed7; examples/contracts/gui_shell_module_comparison_evidence.valid.json=d635ad7d042e80d452424da180b6635e89bc0349; examples/contracts/gui_shell_export_receipt.valid.json=fae30c21b719d2ae9bf9bcff99228bf6eb6b0df1; examples/contracts/gui_shell_export_manifest.valid.json=8e845373fdfcbfd77f555d3030ef3d35b3dd4ed4; examples/contracts/gui_shell_windows_export_build.valid.json=31915727f6f0899bf4358996f709a23a76e9554d; examples/contracts/gui_shell_linux_export_build.valid.json=fd710b0597b140d41f610085b7f3fcd6722b97bb; examples/contracts/invalid/gui_shell_module_catalog_missing_core.invalid.json=6a72157b94b65f785249f99f6e06f773cbdb9319; examples/contracts/invalid/gui_shell_module_catalog_unknown_dependency.invalid.json=5264ee627f7e01f0ed0a910aec44d686d2534d7b; examples/contracts/invalid/gui_shell_module_build_evidence_authority_claim.invalid.json=d8ea3736c96c61b0280450cbb9710532333ae59e; examples/contracts/invalid/gui_shell_module_build_evidence_product_claim.invalid.json=d2c8d2271196322c5f60a279625d004d326a570d; examples/contracts/invalid/gui_shell_module_comparison_evidence_pruning_claim.invalid.json=cafa6b9ae9f5fc1f00ad885c222ef9d6916370f3; examples/contracts/invalid/gui_shell_export_receipt_pruning_claim.invalid.json=6766cfe13dd417fe762cb30dddeb2644a14fb354; examples/contracts/invalid/gui_shell_export_unknown_module.invalid.json=ae5a354654aaddf15821eb64a90f29c00a2fcc0e; examples/contracts/invalid/gui_shell_export_manifest_shape.invalid.json=1eb6811c357a161b357a192cbc1bf430b5b42901; native/rust_helper/src/broker/export_center.rs=477e44d86ebf18d2b004a7294c9cdb14e35aa0f0; apps/desktop_flutter/lib/services/export_client.dart=70d77edc6beb9e209e6b362219d1b22d69375489; apps/desktop_flutter/lib/screens/settings.dart=66374ec4157d573939e7f9cc5c36aecf5d65e251; apps/desktop_flutter/lib/main.dart=6109c821ab5dc809224bec5234c4c59db1d5f988; apps/desktop_flutter/test/export_client_test.dart=b4ec8686b91370b1eaff06dd88b301ee6ef0370c; apps/desktop_flutter/test/settings_compose_test.dart=a78353f93617cc7b17391549ea256f007a7acdb1; apps/desktop_flutter/test/module_pruning_test.dart=20470e523c2640b681e54c51f9c6749ee35f5e05; tooling/build_module_pruned_windows.py=490aae7209ea5988f8e2cdcb05fe092b3d29504a; tooling/compare_module_builds_windows.py=09d926b9c6b611cd59e524cc947ce2dce1c81247; tooling/export_windows_product.py=31856179881a5a5aae1ba4451732e6de6cd7f338; tooling/export_linux_product.py=47fe9a3032a5b2680e3f7d39b7cd64120ac83843; tooling/package_windows_portable.py=d6cfba6941da10fbae35228e7137d7f87f8d3889; tooling/package_linux_product.py=0d32d04ec5b43e1815b3664a323c17db79bc928a; tooling/tests/test_export_linux_product.py=079a187de49cf8b2fb6a7ddadb4175d6916163d1; tooling/conformance_tests/run_conformance_skeleton.py=c6f31d8cb561b48513c6784e54c85969c9cb953e. |
| 試験区分（TestClass） | Rust Export Broker unit（`UNIT`）、Flutter Module surface／選択payloadのWidget・client（`WIDGET`／`FIXTURE`）、Schema正常・否定例（`FIXTURE`）、Windows build／比較・Linux Export checker・Conformanceの実装（`SOURCE`）を確認。旧Windows同一commit AOT比較はcompiler report上の限定`INTERNAL_STATE`、Linux selected Exportの初回Setup履歴はModule pruningを示さない`LIVE_RUNTIME`。本mappingでの製品試験再実行は`NOT_RUN`。 |
| 証拠ID（Evidence ID） | `NONE`。旧P10機能除外CLOSEDとPhase33の限定AOT report／source recordは履歴として保持するが、現v4 N06／FQ-MODULE-PRUNINGの製品Acceptanceへ転記しない。 |
| 阻害要因（Blocker） | 既存`rev2_module_pruning_binary_and_measurement`／`FQ-MODULE-PRUNING`を`release_blocker`として維持。Registry 18件（16 unresolved／2 resolved）、`release_ready=false`、N06／W05/G5はOPEN。新規Blockerなし。 |
| 状態 | Phase 33のE0-04全21項目をSource→Consumer／gapへ記録。P10の機能上除外CLOSEDは歴史的範囲のまま。Phase33の実binary pruningおよびv4 N06製品AcceptanceはOPEN。 |
| 残余 | hash固定の最終独立Productで選択外画面／共有symbolの意味上除去、安全Core保持、Rust／第三者依存除去、選択機能の実動作、cold startup／実行時resource比較をE4/Q4/Q5/FQ-MODULE-PRUNINGで実証する。通常Export Owner UI、Installer、配布はN06/N10の別Consumerとして追跡する。 |

Phase 33のE0-04個別欄を21項目で記録し、46件（46 unique path）のSourceSHAを編集前main `5c7e13d453cba99b3907ac3b06b0388657b850de`のGit blobへ結合する。Broker ModulePlan、Flutter compile-time surface、Windows／Linux Developer AOT証拠を実binary除去・実runtime・安全Core保持へ昇格しない。旧P10の機能除外CLOSED、FQ blocker、N06/N10 status、Registry 18件、`release_ready=false`を保持し、新Evidence／Blockerなし。Phase34の製品配布・Installerとは別consumerに保つ。本mapping単位の製品試験・OS QAはNOT_RUN。

##### Phase 34 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 現行v4 N10、施工線G/H/I、W06、K16/K17。旧Phase34はC33/C34と関連する。現行N10の五OS Consumer行と結合し、Windows工程だけで製品完成を判定しない。 |
| 旧ID | D4 Phase 34「製品配布」。`docs/D4_POCKET_PHASE_MAPPING.md`はC33 Windows最大到達点・C34正式releaseと関連付ける。rev1 C33のWindows限定実装・evidenceとC34未着手を保持し、rev5 P11/P12の局所Product Build受入れを正式配布へ移さない。 |
| 原文参照 | v4仕様S01/S02/S04 N10/S05 K16・K17、工程R00-02/R03 G〜I/R07/R10、施工指示I04-G/H/I・I10、docs/D4_POCKET_PHASE_MAPPING.md Phase 34/C33/C34、docs/specs/windows-product-package.md、docs/WINDOWS_RELEASE_EVIDENCE.md。 |
| 対象 | OS標準配布物→導入→初回設定・Setup Doctor→同一installed候補のGUI／実Task／結果→Owner認可付き更新・限定Repair／Rollback／Uninstallと、Windows Flutter Runner→Rust Desktop Launcher→既存Broker transport／Auditの製品経路。 |
| 目的 | 一般利用者が開発環境なしに各OSの通常配布物から製品を利用し、installed artifact identity・Authority・Auditを保持したまま初回利用・実Task・更新・回復・撤去まで完了する。旧Phase34/ C33/C34のWindows限定歴史を現N10全OS成立へ拡張しない。 |
| 生成元（Producer） | 固定source／toolchain、OS別Exporter／package builder、製品ManifestとApp／Audit identity、候補package／署名・hash、OSごとのInstaller／bootstrap、実installed起動・Task・Update lifecycle、release collector／evidence validator。 |
| 実Consumer | Windows Flutter `BrokerClient`→Runner MethodChannel／named-pipe controller→Rust Desktop LauncherのPID-bound relay→既存authenticated Broker。製品package reader／Bootstrapper→Broker `product_install`／`update_center`→Update Center UI／Tray／Setup Doctor、Windows evidence collector／validatorが出力を消費する。Linux portable／APT bootstrap→Linux launcher→Broker install/update consumer、macOS Runner、Android Kotlin／iOS Swift Device LinkもN10行に個別記録済み。 |
| 契約／Data | `Windows`／`Linux`の`Product Package Manifest Schema`、`Desktop Broker channel request Schema`、`D4PKG01`・`Product Manifest`・`source commit`・`App`／`Audit` identity・`package`／`installed artifact SHA`・`target OS`／`arch`・`update trust`／`receipt`・`fixed install root`、`runtime`／`Audit`／`Setup Doctor`／`first-run evidence`。 |
| 正常時作用 | 目標は同一sourceからOS別candidateを構成し、OS標準導入・初回起動・通常UI・実AI Task・durable Audit・更新／Repair／Rollback／Uninstallを同一candidate identityへ結合すること。現sourceはWindows pipe relayとBroker更新・package component、Linux manual package／lifecycle harness、macOS runner・Mobile Device Linkの一部を持つが、全体配布経路は未成立。 |
| 否定時作用 | package／manifest Schema、未知field、platform／identity／hash／signature／version／root不一致、stale候補、Owner拒否、reparse／unsafe pathの拒否sourceを確認。portable ZIP／debug artifact／test identity／手動Actions成功を通常配布・Publisher trust・installed product PASSへ昇格しない。 |
| 停止／Recovery | Windows／Linux Broker sourceはpackage検査、Audit先行、version stage／activate／直前版rollback／限定uninstallのfail-closed consumerを持つ。Linux manual harnessにはOwner拒否後無変更・Audit／process cleanupの検査源がある。clean same-candidateでの別profile installed lifecycle、正式配布元、macOS Installer/recovery、Mobile production update／rollbackは未成立であり、このmappingでは実行していない。 |
| 標準OS候補 | 現行v4はWindows／macOS／Linux native Desktop、Android Emulator、iOS Simulatorの五OS。Windows実機確認は別条件、macOS/Linuxは正規GUI／native条件を宣言する。Linuxは必須製品対象であり、Phase36の「補助検証」やWSL／WSLgだけへ縮小しない。Android／iOSの物理端末は要求せず標準Emulator／Simulatorを使う。 |
| 配布物 | 現行候補はWindows portable ZIP／D4PKG01（一般利用者向け正式`.exe` installerなし）、Linux unsigned Ubuntu 24.04／x86_64 tar.gz・bootstrap APT `.deb`、macOS debug `.app` source、Android debug APK/AAB、iOS Simulator artifact。正式Publisher／Store identity・production signingと各OS通常配布物は未成立。 |
| 既存実装 | Windows Rust launcher／named pipe relay／Broker package・update・tray・Setup Doctor、Developer staging／smoke／evidence collector、Windows/Linux package／exporter、Linux bootstrap/lifecycle source、manual build workflowとFlutter/Rust/Python/Schema test sourceを確認。Windows stage scriptはDeveloper staging tool、Windows/Linux manual Actionsは検査・artifact補助であり、通常利用Installerや公開pipelineではない。 |
| BUILD_GAP | Windows正式Installer・別profile strict first-run・同一candidate installed Task／Update/Repair/Rollback/Uninstall evidence、Linux native標準X11/Wayland・正式package lifecycle、macOS signed bundle／Installer／recovery、Android/iOS production identity／通常配布／更新／撤去が一つのRelease candidateへ未結合。C34正式release consumer／Owner GOも未成立。 |
| ソース識別値（SourceSHA） | 編集前main `46eae8e11d62db9d018bb262e0a9c9ef786f55d1`のGit blob: AGENTS.md=1eafcadd129fd01f1f2f683594c381605ed7d48d; docs/D4_POCKET_PHASE_MAPPING.md=064ea334f86b137d94fba2370d2a2fd732f6e972; docs/REV2_PROGRESS.md=56f0f168b22d1e64303db2bc6f2c66668393dba2; docs/REV5_PRODUCT_PROGRESS.md=9c978021d3982690e9aaa87896436bad7125b433; docs/総合機能拡張_rev1/進捗.md=1392588f0cd5273013578e84d8e33c5c9dc9a01a; docs/FINAL_QA_QUEUE.md=7b332cccab70bb68aa1ff4f8e23acc0e816d8282; docs/specs/windows-product-package.md=19c5c45dc8cbdbd4560b7b2cb8fe5d8e9f82a2c3; docs/specs/windows-desktop-launcher.md=609a0c724d5bb77ca15c7721e760267ac3c4d018; docs/specs/update-center.md=be37e1f661cb3e4cb05e7c1242051263665a92b2; docs/specs/desktop-broker-channel.md=f9178f356061e60d170f29e57a3cec0f7f228867; docs/specs/linux-product-package.md=faad8b2a9487afc9a818ad3938e0ff92c77cc268; docs/WINDOWS_RELEASE_EVIDENCE.md=9c836b028c352da8d37f9feae53dcb27b537199d; docs/INSTALLER_BOUNDARY.md=7703408c2546ec1cb3b23f27d8ae879b503d55d5; docs/SETUP_DOCTOR.md=f482c934c8bae002a4517fa8a3a7848fb5067fff; installer/windows/README.md=46287d463b38b1093a29e9818c0f0bfd99465393; installer/linux/README.md=25e80411d6ca8f96e6d70718d59c19e7dc6a58e3; installer/macos/README.md=b3f3326fa37e9977c47cb0efc7702294a6e1d33a; installer/windows/stage_installed_app.ps1=c6d0797299624d69b71042416400b2f454787cc7; installer/windows/collect_installed_smoke.ps1=92c4720e82eb0af91002659c742f8e03b15aa2de; installer/windows/collect_broker_smoke.ps1=f5f8abfad981d64c30a6cf1f4e9eafde1b43c357; installer/windows/collect_setup_doctor.ps1=3695f3402dd175f8eafead3baa2c82bcc45e6364; installer/windows/collect_audit_anchor_proof.ps1=309149d2c069bf6f4e2cf3638a932b84a6b60fe0; tooling/windows_release_evidence.py=6f7957c6193c86c7bc15165e743f11651464f29c; tooling/release_smoke.py=e9f38cea998a31bbf9272880dbf4965de5eb0113; tooling/release_runtime_assertions.py=0a0a351fe17151ad86517bd0a965358f40c61490; tooling/release_gate_check.py=22a8d7bfe9adb259f44ce8501fb215652e1d58c6; tooling/package_windows_product.py=c6f9155f2457ef0d89ecb711fa94424fbb1501ee; tooling/package_windows_portable.py=d6cfba6941da10fbae35228e7137d7f87f8d3889; tooling/package_linux_product.py=0d32d04ec5b43e1815b3664a323c17db79bc928a; tooling/package_linux_deb_setup.py=e7a057b2f3db2fca8bbe91401e25d71fb88ceca3; tooling/export_windows_product.py=31856179881a5a5aae1ba4451732e6de6cd7f338; tooling/export_linux_product.py=47fe9a3032a5b2680e3f7d39b7cd64120ac83843; installer/setup_doctor.py=23b43a8a00814e4033addec71efa11126d3b916a; installer/first_run.py=204c49e8ec5226bad1de2a1767afc2ace20de26d; native/rust_helper/src/product_package.rs=a2deb2b6e77bc00ae8d01cd5ca306ee63baa20e5; native/rust_helper/src/product_bootstrapper.rs=3b0495dfc9a1937ba64f3f99c95ae4cc739ddc69; native/rust_helper/src/desktop_launcher.rs=6a9f335de07fa474ca10043aea63ad6e988b572d; native/rust_helper/src/broker/product_install.rs=c91115bdf88dd28e5f3445e7ed66127902909677; native/rust_helper/src/broker/update_center.rs=f6fc98155fcd476765b60d382a680c7f7c6d675e; native/windows_broker_channel/src/lib.rs=94c7e5fc41473cc662ac1947b0b773f0adf67f66; apps/desktop_flutter/windows/runner/broker_pipe_controller.cpp=e645d2d8cc25a1e61d9cb702013b5c3ee2e31eef; apps/desktop_flutter/windows/runner/main.cpp=be0084505135d0d33311915948fd660e59ea66ab; apps/desktop_flutter/lib/services/broker_client.dart=122de14350dfe1f160c7f5f911dd1e1d56da8233; apps/desktop_flutter/lib/services/windows_tray_client.dart=cf8214ceb95fa1b24c7eed4f14b312bda80e0b15; apps/desktop_flutter/lib/services/update_client.dart=26d2e946b5b15ea3328867e606e9f75bf4bfc2d1; apps/desktop_flutter/lib/screens/setup_doctor.dart=41fe7f3e904fcc2179ccc8c8373e53f0b646d1fe; apps/desktop_flutter/lib/screens/settings.dart=66374ec4157d573939e7f9cc5c36aecf5d65e251; apps/desktop_flutter/test/settings_install_flow_test.dart=73fc7b9706a7f4246972db46d5398d8b73decb09; apps/desktop_flutter/test/update_client_test.dart=f70bb4a6dbc2340b5d26d867496952629dad1218; apps/desktop_flutter/test/windows_tray_client_test.dart=0b24e07221087e43355ac465ca4ace73a48af31e; specs/d4_pocket_product_package_manifest.schema.json=3151ba81a869b87e6c0fa8f8633a0e424fc43ae7; specs/d4_pocket_linux_product_package_manifest.schema.json=34087f190e7189641b34ae3163b568a038c9f1c2; specs/desktop_broker_channel_request.schema.json=c2376d6aa1b1a5a4bf5e1998f6e5510b3dee3c26; release_blockers.registry.json=244adc04a01e7b406c4ebcdeb829f7077d4aa35f; .github/workflows/windows-manual-rust-validation.yml=3bce337fcb5988fb3fb6c292eb964d889fb4ffb8; .github/workflows/windows-manual-export-build.yml=f8255269ad70a63f329b6abb78f6da3d62cde960; .github/workflows/windows-manual-desktop-flutter-validation.yml=c709faa3005f0893915c5d6044f780847ac66190; .github/workflows/linux-manual-rust-validation.yml=8c03bdd11e6bbfe3d9acd006b2200e7bc46f7788; .github/workflows/linux-manual-export-build.yml=8d59dd1e9e54a9a4ebb16992f59a402c1ab77c67; .github/workflows/apple-manual-build.yml=ed60235da7b35cdfaac23a9fd2d07023e1d29843; .github/workflows/android-manual-emulator.yml=f0c1ecb7185c91375adfbbdbfaf49e55a2672b57. |
| 試験区分（TestClass） | Rust package／Broker／pipe単体・Flutter Widget・Python packaging／evidence fixture（`UNIT`／`WIDGET`／`FIXTURE`）、手動Windows／Linux／Apple／Android build workflow（`SOURCE`／限定build）、過去C33 Windows staged／collector (`LIVE_RUNTIME`の限定run)、Linux hosted/Xvfb lifecycle (`LIVE_PRODUCT`の限定run)が履歴にある。これらは同一候補の五OS通常配布Acceptanceではない。本mappingで製品test・workflow・installed runtime再実行は`NOT_RUN`。 |
| 証拠ID（Evidence ID） | `NONE`。過去C33の`installed smoke subgate`、Phase34の`Developer staged Broker health`、`Linux hosted lifecycle run`は元`commit`／`OS`／`identity`／`scope`の履歴として保持し、現行v4の`N10`／`W06`正式製品`Acceptance`へ転記しない。 |
| 阻害要因（Blocker） | Registry 18件（16 unresolved／2 resolved）、`release_ready=false`を維持。既存`rev2_desktop_product_distribution`、`windows_distribution_identity`、`rev2_flutter_broker_channel_boundary`、`windows_installer_first_run_smoke`、`windows_setup_doctor_smoke`、`windows_evidence_provenance_isolation`、`linux_native_product_support`、`rev2_mobile_distribution`、`comprehensive_extension_rev1_completion`、`owner_go`はunresolved。新規Blockerなし。 |
| 状態 | Phase34のE0-04全21項目をProducer→Consumer／OS／evidence gapとして記録。C33 Windows最大到達履歴とC34未着手を保持。現N10／W06、OS別製品Acceptance、E0はOPEN、正式Release／Owner GOは未成立。 |
| 残余 | 同じclean source candidateの通常配布物、OS標準installed first-runと実Task、更新／Repair／Rollback／Uninstall、各OSのArtifact provenance／identity／Auditを結合する。Windows別profile strict evidence、Linux native GUI／installer、macOS正式導入、Android/iOS production配布・lifecycleはN10行で追跡し、Q0〜Q7／N11へ証拠を結合する。 |

Phase34のE0-04個別欄を21項目で記録し、61件（61 unique path）のSourceSHAを編集前main `46eae8e11d62db9d018bb262e0a9c9ef786f55d1`のGit blobへ結合する。旧Phase34 Windows channel／staged evidence、C33の限定Windows最大到達とC34未着手を履歴範囲に留め、現行N10全OS要件へ完了を転記しない。Linux native必須性、Registry 18件、関連release blocker、`release_ready=false`を保持し新Evidence／Blockerなし。C33/C34に関連する正式distribution／installed lifecycleは未成立。本mapping単位の製品test・OS QAはNOT_RUN。

##### Phase 35 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 現行v4 `N09`／`N10`、施工線I、`W07/G6-OS`、能力`K17`。旧Phase35の`C24`／`C26`を現行製品Acceptanceと分離する。 |
| 旧ID | D4 Phase 35「Mobile統合」。`C24`はMobile補助操作・Agent状態metadata、`C26`はAndroid build回帰。旧`C24`／`C26`の局所PASS履歴を保持する。`C25`／Phase36のApple補助検証、旧`P13`の非Windows Product Buildとは独立のcrosswalkとして扱う。 |
| 原文参照 | v4仕様`S01/S02/S04 N09・N10/S05 K17`、工程`R00-02/R03線I/R06 W07/R07/R10`、施工指示`I00-04/05/I04-I/I06-01〜10/I07-05〜07`、`docs/D4_POCKET_PHASE_MAPPING.md` Phase 35・C24/C26、rev1 C24/C26、rev2 Phase35、`docs/specs/mobile-surface.md`、`docs/specs/device-link.md`。 |
| 対象 | `Android`／`iOS`の`Flutter`補助操作面→OS-native `Device Link`資格保管・`TLS` transport→既存Rust `Broker`読み取り／対話handler→bounded projection、および各OS通常配布・installed lifecycle候補。 |
| 目的 | Mobileから現在のAgent／Runtime／通知／資源等を限定表示し、既存対話要求を同じ`Broker`へ送る。`Permission`／`Approval`／`Credential`／`Trust`をMobileで生成せず、OS-native transportと通常利用可能な製品候補を維持する。 |
| 生成元（Producer） | `Flutter`画面・projection client・Device Link request、Android `Kotlin`／iOS `Swift` native service、OS保護資格・`TLS` client、Rust `Broker`の既存handler、Schema／build workflow。 |
| 実Consumer | Agent画面選択→`MobileProjectionClient.agents`→`Agent一覧`読み取りhandler→Schema検査→メタデータ投影。接続・操作はFlutter channel→Android `MainActivity`／iOS `AppDelegate`のnative service→OS保護store／`TLS`→Rust `device_transport`／`device_link`→既存Broker handler。`対話開始`／`対話送信`は既存Owner承認待ち対話要求へ流れる。MobileのMCP画面は未観測状態だけを示す。 |
| 契約／Data | `device_link_request`、`mobile_device_link_channel_request`、`mobile_agent_list`、`agent_adapter`、`mobile_workspace_selection`、`mobile_local_recovery_audit`。Flutter channelには固定版・operation・bounded非秘密payload／projectionだけを通し、招待・資格実値はnative保管・transport内に留める。 |
| 正常時作用 | C24の通知／Runtime／resource／履歴metadata／停止receipt／Recovery導線、読み取り専用Agent一覧、Owner Approval待ちの対話要求にsource consumerがある。`ready`やAgent metadataは実Task capabilityではない。一般Agent Task orchestration・結果依存次Task・Mobile MCP Tool実行consumerはない。 |
| 否定時作用 | 不明operation／field、重複JSON key、秘密・owner権限field・過大payload、資格不一致／失効／期限切れ、background中の要求、未知Agent metadataを拒否／未観測とする。Flutter／snapshot／log／error／test artifactへ資格実値を出さず、MCP metadataから作用権限を作らない。 |
| 停止／Recovery | Android Activity／iOS sceneのnative lifecycleを通信可否の正本とし、backgroundで通信を止める。KeyStore／Keychain失敗はfail-closed、復帰時に資格を再確認し自動再送しない。端末内recovery recordはBroker Auditとは別で、停止receiptは実Runtime停止ではない。一般Task cancel／crash recoveryのMobile一体証拠は未成立。 |
| 標準OS候補 | Android `API 35 Google APIs x86_64 Emulator`、iOS `iPhone 16 Pro / iOS 18.5 Simulator`を現台帳候補として保持。どちらもRelease QA確定・installed製品合格ではない。物理Android／iPhoneは要求しない。 |
| 配布物 | Android debug `APK/AAB`、iOS Simulator app／manual build artifactのsource候補。標準OS通常配布identity、production署名／資格、一般利用者向けinstalled product・更新／撤去は未成立。 |
| 既存実装 | `mobile-surface`／`device-link`意味契約、Flutter 9画面・Agent metadata projection、Android `Kotlin`／iOS `Swift` store・strict JSON・native `TLS`、既存Rust `Broker` allowlist、mobile／native test sourceとAndroid／Apple manual workflowを確認。存在・試験sourceは現候補QAではない。 |
| BUILD_GAP | Mobile `Agent Task`の専用開始・結果依存連鎖、`MCP Tool`実行、installed同一候補からBroker実仕事までのAndroid Emulator／iOS Simulator実測、通常配布・署名identity・update／repair／rollback／uninstallが未成立。Mobile Native Device Linkの現候補E2E・秘密／Permission拒否・lifecycle evidenceも未結合。 |
| ソース識別値（SourceSHA） | 編集前main `a24b7005437a8b6ba5d381b27d31d3471e2a6c2c`のGit blob: AGENTS.md=1eafcadd129fd01f1f2f683594c381605ed7d48d; docs/D4_POCKET_PHASE_MAPPING.md=064ea334f86b137d94fba2370d2a2fd732f6e972; docs/REV2_PROGRESS.md=56f0f168b22d1e64303db2bc6f2c66668393dba2; docs/REV5_PRODUCT_PROGRESS.md=9c978021d3982690e9aaa87896436bad7125b433; docs/総合機能拡張_rev1/進捗.md=1392588f0cd5273013578e84d8e33c5c9dc9a01a; docs/QE_2026-11-13/01_D4Pocket_統合製品実装仕様書_v4.md=352c53180cbad07888beb956aad0fb2a58074c58; docs/QE_2026-11-13/02_D4Pocket_統合開発工程表_v4.md=d32dd251e5f7cbb97d92f6e7540d6b6ed0ecec05; docs/QE_2026-11-13/03_D4Pocket_Codex施工指示書_v4.md=49447973e88c50a09b2f4978adfed84fd464c6d0; docs/QE_2026-11-13/採用記録.md=f159c1aa6ea5bc6e80e0877d8068f7e44da4c4d3; docs/QE_2026-11-13/E0_要求consumer台帳.md=d134be4877614aab99f8df1ddae602e56f283170; docs/QE_2026-11-13/進捗.md=6830cf815bddedf9276457439fa8ec33a1b21e16; docs/specs/mobile-surface.md=071ee69485ede6f792d303dd7fea615d1d912607; docs/specs/device-link.md=0e2af0081c2d82531ff1eb28bec7ccc810b63951; apps/mobile_flutter/README.md=4b0cb8e02e387c05efb779bd04b2756b6b86919f; apps/mobile_flutter/pubspec.yaml=cfc6f166f19db1729d1d903bdfb3f90d58772d76; apps/mobile_flutter/lib/main.dart=abf16b9774d135c2f960633ad922be608b5fd0c5; apps/mobile_flutter/lib/services/device_link_client.dart=e8c49337127cb7f62fad248988b99707c5e49219; apps/mobile_flutter/lib/services/device_link_controller.dart=d8240d454e76ca7917cb1d62bcb41f40e34a976f; apps/mobile_flutter/lib/services/mobile_projection_client.dart=ba82e4ddeaa5692793237a1bc3258ce53b7aa3ed; apps/mobile_flutter/lib/screens/mobile_dashboard.dart=57839004013b3b198ba2bc05c87cbae872bbf7dd; apps/mobile_flutter/lib/screens/agent_status.dart=52e4907897c2bb3674bb8523204a787d3a6cb002; apps/mobile_flutter/lib/screens/mcp_status.dart=40af06807f8bdf1280d860b3d9ffa32ee2e22cc0; apps/mobile_flutter/lib/screens/device_connection.dart=4adb15803bf6b208c13aca930bb5496f03b4d6b8; apps/mobile_flutter/lib/screens/approval_review.dart=636fd71b0883c433cc2c602eee5a4a41acec0f49; apps/mobile_flutter/lib/screens/emergency_stop.dart=4d861a1ad64bcb5a73e024e07cf25b58aa3bcfc2; apps/mobile_flutter/lib/screens/history.dart=afa685cac135d4e113b9ea3e66aa4d2c934d8674; apps/mobile_flutter/lib/screens/notifications.dart=243c59c321cb1cbff39cc6cc993bd01d589c760f; apps/mobile_flutter/lib/screens/runtime_status.dart=461f63b8dfcb0ee1b6acb662b20cea5501d114d2; apps/mobile_flutter/lib/screens/resource_overview.dart=322af6bc2935c571017dab3f03b9c7a0cea7c616; apps/mobile_flutter/lib/screens/recovery_instruction.dart=f4ff9aad37adb542a012d09c3827f4dad6e4d9f6; apps/mobile_flutter/lib/screens/shared.dart=0e703d1ccebc2b0beeefdeec696665f66f1e4c6f; apps/mobile_flutter/test/device_link_test.dart=509b4febc424d1c0fd330b71529b00457b5cf429; apps/mobile_flutter/test/mobile_projection_client_test.dart=74661d2f2e58046a83e99f711435e4c1b2a74b08; apps/mobile_flutter/test/resource_overview_test.dart=7695155627699274b8c0768c0de302344caa9eec; apps/mobile_flutter/android/app/build.gradle.kts=fe96edfa0f714c2d2be509ce3bc21bec6253a714; apps/mobile_flutter/android/app/src/main/AndroidManifest.xml=dd33cc927554248e402c699561c15863477c0ff1; apps/mobile_flutter/android/app/src/main/kotlin/com/example/gui_shell_mobile/MainActivity.kt=5c0e28f0ad371f80671d26072bd1c1a134a881d9; apps/mobile_flutter/android/app/src/main/kotlin/com/example/gui_shell_mobile/DeviceLinkNativeService.kt=0fd3c6f5bd09e7a18df9fefe97e2e7bb9e9da706; apps/mobile_flutter/android/app/src/main/kotlin/com/example/gui_shell_mobile/DeviceLinkTlsClient.kt=c47456791740765ae5c8a311a6dc0777de907027; apps/mobile_flutter/android/app/src/main/kotlin/com/example/gui_shell_mobile/DeviceLinkNativeStore.kt=55484d80a94abaad907074defebf116fdcf7aeb7; apps/mobile_flutter/android/app/src/main/kotlin/com/example/gui_shell_mobile/DeviceLinkModels.kt=1a14056a1cadb5b9a2d23f4b295b9ed5ad793839; apps/mobile_flutter/android/app/src/main/kotlin/com/example/gui_shell_mobile/StrictJson.kt=8e323e147bfc12b03a2773ce739621113006f43c; apps/mobile_flutter/android/app/src/test/kotlin/com/example/gui_shell_mobile/DeviceLinkPolicyTest.kt=cca69277be00014e8681f59db3a04a4adbb9fb23; apps/mobile_flutter/android/app/src/test/kotlin/com/example/gui_shell_mobile/StrictJsonTest.kt=dc77d56888639709e829e46969f2455599b7d509; apps/mobile_flutter/android/app/src/androidTest/kotlin/com/example/gui_shell_mobile/NativeDeviceLinkProductFlowTest.kt=4f4a79e0bd80b1f5eb280857dec7a7d5df12258c; apps/mobile_flutter/ios/Runner/AppDelegate.swift=c5fe56698d097e6e03587de9c1be10107e28e855; apps/mobile_flutter/ios/Runner/SceneDelegate.swift=b9ce8ea2b2adb8d8061af09c75d2480999773ac3; apps/mobile_flutter/ios/Runner/DeviceLinkNativeService.swift=9d185729c439934dce0553b490f2fcd1f7b28a0d; apps/mobile_flutter/ios/Runner/DeviceLinkTLSClient.swift=35163651ca550f198d04dd70db53be82f2e7c737; apps/mobile_flutter/ios/Runner/DeviceLinkNativeStore.swift=04d8e5fcea97b3d05268f593da5a40e212d1e518; apps/mobile_flutter/ios/Runner/DeviceLinkModels.swift=296eda00b60cd8c2c5203cc2e09183f265a2a884; apps/mobile_flutter/ios/Runner/DeviceLinkStrictJSON.swift=636eddbfb38578341f46f59e03fa9f5c2fad0f08; apps/mobile_flutter/ios/Runner/DeviceLinkProductTestInput.swift=5c7d8674d271d9e12108e0fb1a8c83d48e86a38c; apps/mobile_flutter/ios/Runner/Runner.entitlements=18a645490361df74f9acc3b0f61b2f7f99c6a3b0; apps/mobile_flutter/ios/Runner/Info.plist=a45d01d480e73c6f992273b9c8d2d57260293fc7; apps/mobile_flutter/ios/Runner.xcodeproj/project.pbxproj=73876a3b79b5ee5bd50dea0428764f1632dfcb19; apps/mobile_flutter/ios/RunnerTests/RunnerTests.swift=86841901b49b5631879d5c1dbdcae99730446b9e; apps/mobile_flutter/ios/RunnerUITests/DeviceLinkProductUITests.swift=b74c1f4a846c07eed9b6ae9244ac18b4f1030eec; native/rust_helper/src/broker/device_link.rs=c2e1ff88ff34a5764b2b4fb1d3f7abd8727e5c51; native/rust_helper/src/broker/device_transport.rs=54af4e22d794821c6c2cc8105a85588d15ff36d2; native/rust_helper/src/broker/ipc_server.rs=58a404b920ef5905e174668ec2de0e04a9e8b044; native/rust_helper/src/broker/dialogue.rs=c3c3fddb630dd06b7e6cc1c13b39f3d4f6f6a999; native/rust_helper/tests/broker_ipc.rs=e57f4509e9957923b88baf8b998b6a50e5f3a123; specs/device_link_request.schema.json=f0539522c5900148753c3e6eb500c935f7b11ac2; specs/mobile_device_link_channel_request.schema.json=5de3cbe5f3df5c60df77228e00f6a138ae319578; specs/mobile_agent_list.schema.json=61558ea30f269c886d40f414cfe31bda09fa5d11; specs/agent_adapter.schema.json=e0792589a2bae168ba221af4f4f9abb505815c12; specs/mobile_workspace_selection.schema.json=6963dbea6db4e876340ea40bbd13a394fae92e23; specs/mobile_local_recovery_audit.schema.json=aa30b130979b5f869b5a08cc21315f3918679747; examples/contracts/mobile_agent_list.valid.json=835891bc94c675b58b6668d2415404b77b12f10f; examples/contracts/mobile_workspace_selection.valid.json=74db571dbf7823c831e97eaecd7e4a4f29fcced9; examples/contracts/invalid/mobile_agent_list_authority.invalid.json=14c7e797349de1c24cd85586b4421d929adff1d8; examples/contracts/invalid/mobile_device_link_channel_request_secret.invalid.json=d9408ea1a1c3602f9c37e1b844f57417bc5e6cb8; .github/workflows/android-manual-emulator.yml=f0c1ecb7185c91375adfbbdbfaf49e55a2672b57; .github/workflows/apple-manual-build.yml=ed60235da7b35cdfaac23a9fd2d07023e1d29843; tooling/schema_check/check_schemas.py=7dca27717cbe5cccb45e1013b3436918a2322e75; tooling/conformance_tests/run_conformance_skeleton.py=c6f31d8cb561b48513c6784e54c85969c9cb953e; release_blockers.registry.json=244adc04a01e7b406c4ebcdeb829f7077d4aa35f。 |
| 試験区分（TestClass） | 旧C24のFlutter／Rust限定試験、旧C26のFlutter analyze・32試験・Android debug APK/AAB build、現在の`UNIT`／`WIDGET`／`FIXTURE`／native instrumented／manual `workflow_dispatch`のsourceを区別する。Phase35 mapping時点のMobile test・Android／Apple workflow・Emulator／Simulator installed QAは`NOT_RUN`。 |
| 証拠ID（Evidence ID） | `NONE`。C24/C26とDevice Linkの旧実行記録は各日付・commit・scopeの履歴のみ。現v4 `N09/N10/W07`製品Acceptanceへ転記しない。 |
| 阻害要因（Blocker） | Registry 18件（16 unresolved／2 resolved）、`release_ready=false`を維持。既存`rev2_mobile_device_evidence`、`rev2_mobile_distribution`、`rev2_mobile_flutter_native_device_link_boundary`、`comprehensive_extension_rev1_completion`、`owner_go`はunresolved。Linux native blockerはPhase36／N10の別scopeで保持。新規Blockerなし。 |
| 状態 | Phase35のE0-04全21項目を現`N09/N10`・線I・`K17`へmapping済み。旧C24/C26 statusと限定試験履歴は保持。現行Mobile installed Acceptance、`N09/N10`、`W07`、E0はOPEN。製品test／OS QAは未実行。 |
| 残余 | 専用Mobile Agent Task／MCP作用が要求されるconsumerと既存read-only／対話要求の境界を保ち、各OSの通常導入候補、native秘密保管、Broker連携、実Task、拒否・停止・復帰、Audit、update／撤去を同一artifact identityへ結合する。標準OS候補の最終値とQAはN10／W07／N11の責任範囲。 |

Phase35のE0-04欄を全21項目で記録し、78件（78 unique path）のSourceSHAを編集前main `a24b7005437a8b6ba5d381b27d31d3471e2a6c2c`へ結合する。旧C24/C26の限定status・test historyを保持し、Mobileの表示／対話requestをAgent Task・MCP作用・N10通常配布へ昇格しない。既存Registry 18件・Mobile blocker・`release_ready=false`は不変、新Evidence／Blockerなし。追加のPhase35製品試験・workflow・Emulator／Simulator QAはNOT_RUN。

##### Phase 36 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 現行v4 N10、N11、施工線H/I、R07、W07/G6-OS、能力K17/K19、およびD4-LNX-T01〜T08へ対応する。旧Phase36の「補助検証」という名称でLinux native必須要件を縮小しない。 |
| 旧ID | D4 Phase 36「Apple／Linux補助検証」。C25 Apple補助workflowと、Phase35にも対応するC26 Android build回帰の交差範囲。C26のAndroid側履歴はPhase35の記録を参照し、ここでは再検査しない。 |
| 原文参照 | 現行v4仕様S01/S02/S04 N10/S05 K17・K19、工程R00-02/R03線H・I/R07/R10、施工指示I00-05/I04-H・I/I06-01〜10/I07-05〜07、docs/D4_POCKET_PHASE_MAPPING.md Phase36・C25/C26、rev1進捗C25/C26、rev2 Phase35/36、docs/QE_2026-10-22のD4-QE-R13・D4-LNX-T01〜T08。旧資料は履歴と試験要求の来歴であり、現行受入れはv4に従う。 |
| 対象 | macOS Desktop／iOS Simulator向けApple manual workflowとnative consumer、Linux native DesktopのFlutter Runner→Rust Launcher／Broker→製品Package／導入／更新経路。C26 Android build履歴はPhase35との対応確認だけを対象とする。 |
| 目的 | 旧Apple／Linux補助工程のSource→Consumerを現行OS別製品化要求へ接続し、workflow・debug build・hosted fixtureの証拠範囲を明示する。Linux nativeを完全対応必須対象として保持し、Apple／Linux補助作業を製品Acceptanceや五OS完成へ読み替えない。 |
| 生成元（Producer） | apple-manual-build.ymlのworkflow_dispatch target、macOS/iOS用Flutter・Rust・Xcode build／test、linux-manual-export-build.ymlのLinux Export／Package／installed acceptance scope、Linux product builder／setup package builder、OS別LauncherとBroker要求。 |
| 実Consumer | Apple側はmacOS Runnerの固定`Broker channel`→同梱`Rust helper`→既存Broker、およびiOS Flutter→Swift `Device Link service`／`Keychain`・`TLS`→既存Rust Broker transport。Linux側は製品初回起動用archive／APT初期導入からRust Linux Launcher、既存Brokerの`Package Install`／`Update Center`／`XDG entry`へ接続するsourceがある。手動`workflow`は指定`commit`とOS実行環境で限定build／test／受入れprobeを実行する。 |
| 契約／Data | OSごとの`source commit`、`Product Manifest`／Package `inventory`・`hash`、App／Audit identity、`D4PKG01`、Linux package manifest、macOS sandbox／匿名pipe、iOS native `Device Link`／`Keychain`、Android C26のdebug build履歴を区別する。Workflow `input`・Adapter metadata・Simulator stateはPermission／Approval／Trustを生成しない。 |
| 正常時作用 | macOS／iOS runnerで指定targetのbuild・native test・Simulator試験を行い、Linuxでは対象distro向けbundle／packageと限定installed操作を観測するsource経路がある。製品成立には宣言OSへの通常導入、native GUI/session、Broker認可、実Task、Audit、更新／修復／復旧／撤去を同じcandidate identityで結ぶ必要がある。 |
| 否定時作用 | OS／ABI／manifest／identity／hash不一致、未知target、破損package、権限・秘密境界違反を拒否または失敗として保持する。manual workflowの成功、debug artifact、Xvfb、WSL、Simulator buildを別OSのinstalled productや正式配布のPASSへ拡張しない。 |
| 停止／Recovery | build／test／install／Audit／cleanupの失敗は対象commit・runner・scopeのFAILまたはNOT_RUNに留め、正常化・再起動・cleanupが未確認なら成功にしない。Linuxのpackage導入・更新・rollback・uninstallは既存Broker／Owner／Audit境界を使う。macOS/iOS配布資格と製品lifecycleは別の未成立条件として保持する。 |
| 標準OS候補 | LinuxはUbuntu 24.04 LTS/x86_64 portable・APT候補、Task supervisionを含む一部manual scopeはUbuntu 22.04 hosted。実distro範囲はUNKNOWN。native X11とWaylandを別々に扱い、Xvfbだけを実desktop証拠としない。macOS workflow runnerはmacos-15、source deployment targetは10.15だが保証OS範囲はUNKNOWN。iOS候補は既存表のiPhone 16 Pro／iOS 18.5 Simulator。Android API 35 Google APIs x86_64 EmulatorはPhase35 cross-referenceの候補で再試験しない。非Windows物理端末は要求しない。 |
| 配布物 | Linux unsigned portable tar.gz、bootstrap用APT .deb、D4PKG01更新payloadのsourceがある。正式署名・一般配布identityは未成立。macOSはdebug .app／限定runner artifactで一般向けinstaller・notarization未成立。iOS Simulator buildはdevice IPAではない。Android debug APK/AABは旧C26履歴で、Release配布物ではない。 |
| 既存実装 | `Linux Runner`／`Broker`接続面、Rust `Linux Launcher`・`Product Bundle`、Linux書出し／package／APT初期導入tool、macOS `Runner`／Rust helper接続、iOS Swift native `service`／`store`／`TLS`、Apple／Linux `workflow`と対応`test source`を確認した。`Run #258`のAPT初期導入、`#277`の限定同一`source` update lifecycle、`#286`のUbuntu 22.04 Xvfb／X11 normal Release positive Taskは各`run`・`source`・identity・scopeを伴う過去証拠である。 |
| BUILD_GAP | Linux native全体の標準distro／GUI session、X11／Wayland別の実導入・実利用、必須consumer全数、正式配布・署名が未成立で、linux_native_product_supportはunresolved。Run #286はhosted Xvfb／X11・synthetic identity・local no-auth fixtureの限定LIVE_PRODUCTで、実desktop／Wayland・外部Provider・全Task recoveryを証明しない。macOS正式installer／配布・installed lifecycle、iOS device配布Artifact／署名とproduction lifecycleも未成立。Android Agent Task／MCPや通常配布の残差はPhase35／N10へ残す。 |
| ソース識別値（SourceSHA） | 編集前main c84c86b3b9c185b9b6f1ff34d69e15b21e21033dのGit blobを84件・84 unique path照合: AGENTS.md=1eafcadd129fd01f1f2f683594c381605ed7d48d; docs/QE_2026-11-13/01_D4Pocket_統合製品実装仕様書_v4.md=352c53180cbad07888beb956aad0fb2a58074c58; docs/QE_2026-11-13/02_D4Pocket_統合開発工程表_v4.md=d32dd251e5f7cbb97d92f6e7540d6b6ed0ecec05; docs/QE_2026-11-13/03_D4Pocket_Codex施工指示書_v4.md=49447973e88c50a09b2f4978adfed84fd464c6d0; docs/QE_2026-11-13/採用記録.md=f159c1aa6ea5bc6e80e0877d8068f7e44da4c4d3; docs/QE_2026-11-13/E0_要求consumer台帳.md=d39518ead9fa06e664e2cb6b047760ba3866bdc6; docs/QE_2026-11-13/進捗.md=4eecae8bf4f6df26f8c3b4ed933618029f2345d3; docs/D4_POCKET_PHASE_MAPPING.md=064ea334f86b137d94fba2370d2a2fd732f6e972; docs/REV2_PROGRESS.md=56f0f168b22d1e64303db2bc6f2c66668393dba2; docs/REV5_PRODUCT_PROGRESS.md=9c978021d3982690e9aaa87896436bad7125b433; docs/総合機能拡張_rev1/進捗.md=1392588f0cd5273013578e84d8e33c5c9dc9a01a; docs/QE_2026-10-22/採用記録.md=1f9d6332132759ed68f3af8b4587368b162096c2; docs/QE_2026-10-22/D4Pocket_01_QE統合実装仕様書_20261022.md=400cbfbd524fa76ca34e3f062641be629227246d; docs/QE_2026-10-22/D4Pocket_02_QE統合工程表_20261022.md=331daeecc141ca95a49e4a758aa9f1dbe643cf07; docs/QE_2026-10-22/D4Pocket_03_Codex_QE施工指示書_20261022.md=8f1cb26aa9673ba29d5c1cc50eaeff7ec0b55993; docs/specs/linux-product-package.md=faad8b2a9487afc9a818ad3938e0ff92c77cc268; docs/specs/linux-desktop-channel.md=c1cc8c1624c5b3ecc053bae32131b335456e47d5; docs/specs/macos-desktop-channel.md=0611524b89c09365d35c33d23367a5fb6f65a4d6; docs/specs/device-link.md=0e2af0081c2d82531ff1eb28bec7ccc810b63951; docs/specs/mobile-surface.md=071ee69485ede6f792d303dd7fea615d1d912607; docs/specs/update-center.md=be37e1f661cb3e4cb05e7c1242051263665a92b2; installer/linux/README.md=25e80411d6ca8f96e6d70718d59c19e7dc6a58e3; installer/linux/PORTABLE_README_ja.txt=703f86b2d113c165f7930f026b842df762f2b205; installer/macos/README.md=b3f3326fa37e9977c47cb0efc7702294a6e1d33a; .github/workflows/apple-manual-build.yml=ed60235da7b35cdfaac23a9fd2d07023e1d29843; .github/workflows/linux-manual-export-build.yml=8d59dd1e9e54a9a4ebb16992f59a402c1ab77c67; .github/workflows/android-manual-emulator.yml=f0c1ecb7185c91375adfbbdbfaf49e55a2672b57; apps/desktop_flutter/linux/runner/CMakeLists.txt=18ee1a19f4ae322522e05b9954ef2b362fcdfe7f; apps/desktop_flutter/linux/runner/main.cc=aed7d44034536c80dd748eac9515be35716d790d; apps/desktop_flutter/linux/runner/my_application.cc=c5c0943f4ce4058290e1cb56ff64286aa7b54f54; apps/desktop_flutter/linux/runner/my_application.h=db16367a77d061e7c4b63e46b8aea134ade776d4; apps/desktop_flutter/linux/runner/broker_process_channel.cc=98d6f7e73baae7228c25d72d29343a2564291d97; apps/desktop_flutter/linux/runner/broker_process_channel.h=0ee8f4587c6464d98e08a943949bbc369ebb9646; apps/desktop_flutter/macos/Runner/AppDelegate.swift=b32d02ff0255eae0226b96b5f0a3aa2fe5cfc41f; apps/desktop_flutter/macos/Runner/BrokerProcessChannel.swift=a1269e01608bc194b0f0913a6f96cd489b926155; apps/desktop_flutter/macos/Runner/MainFlutterWindow.swift=94ab8e30b7adfa10ea9100ceb3570b7dd058937a; apps/desktop_flutter/macos/Runner/Release.entitlements=06098d53759f154c3c337db05bc658e1575c329d; apps/desktop_flutter/macos/Runner/DebugProfile.entitlements=bbe062f049eeb43d0dae96d6a8d1548897379b60; apps/desktop_flutter/macos/Runner/Info.plist=4789daa6a443eefa7f3f729164a18351e06e9800; apps/desktop_flutter/macos/Runner.xcodeproj/project.pbxproj=4e99d89acfe7a19a7c4b8c5d38590a763c68c7ea; apps/desktop_flutter/macos/RunnerTests/RunnerTests.swift=1328e4b5774434404645e10a774297496b3175f7; apps/desktop_flutter/macos/RunnerUITests/AdapterOwnerUITests.swift=eeab11b1123165a8f82da55ebbb014eab8803862; apps/mobile_flutter/ios/Runner/AppDelegate.swift=c5fe56698d097e6e03587de9c1be10107e28e855; apps/mobile_flutter/ios/Runner/SceneDelegate.swift=b9ce8ea2b2adb8d8061af09c75d2480999773ac3; apps/mobile_flutter/ios/Runner/DeviceLinkNativeService.swift=9d185729c439934dce0553b490f2fcd1f7b28a0d; apps/mobile_flutter/ios/Runner/DeviceLinkNativeStore.swift=04d8e5fcea97b3d05268f593da5a40e212d1e518; apps/mobile_flutter/ios/Runner/DeviceLinkTLSClient.swift=35163651ca550f198d04dd70db53be82f2e7c737; apps/mobile_flutter/ios/Runner/DeviceLinkStrictJSON.swift=636eddbfb38578341f46f59e03fa9f5c2fad0f08; apps/mobile_flutter/ios/Runner/Runner.entitlements=18a645490361df74f9acc3b0f61b2f7f99c6a3b0; apps/mobile_flutter/ios/Runner.xcodeproj/project.pbxproj=73876a3b79b5ee5bd50dea0428764f1632dfcb19; apps/mobile_flutter/ios/RunnerTests/RunnerTests.swift=86841901b49b5631879d5c1dbdcae99730446b9e; apps/mobile_flutter/ios/RunnerUITests/DeviceLinkProductUITests.swift=b74c1f4a846c07eed9b6ae9244ac18b4f1030eec; native/rust_helper/src/bin/gui_shell_desktop_launcher.rs=809efba7a03eacddf9e172011149d9a8d5e3fda7; native/rust_helper/src/bin/gui_shell_macos_broker.rs=6c830d9de6e3e43bb4d829dae39412799a8704f9; native/rust_helper/src/linux_desktop_launcher.rs=f58fb226f21093c2d1f062259d394b4f8ec022f6; native/rust_helper/src/linux_product_bundle.rs=f71ed312c629e81914c61180baeb9723b315a69a; native/rust_helper/src/macos_desktop_worker.rs=09fa26563fc671e07f23c73f7dd2a7f2cf3fed7e; native/rust_helper/src/macos_desktop_owner.rs=31e13a57fcde422878fae63083bd323e07f482ec; native/rust_helper/src/broker/product_install.rs=c91115bdf88dd28e5f3445e7ed66127902909677; native/rust_helper/src/broker/update_center.rs=f6fc98155fcd476765b60d382a680c7f7c6d675e; native/rust_helper/src/broker/device_link.rs=c2e1ff88ff34a5764b2b4fb1d3f7abd8727e5c51; native/rust_helper/src/broker/device_transport.rs=54af4e22d794821c6c2cc8105a85588d15ff36d2; native/rust_helper/src/broker/ipc_server.rs=58a404b920ef5905e174668ec2de0e04a9e8b044; tooling/export_linux_product.py=47fe9a3032a5b2680e3f7d39b7cd64120ac83843; tooling/package_linux_product.py=0d32d04ec5b43e1815b3664a323c17db79bc928a; tooling/package_linux_deb_setup.py=e7a057b2f3db2fca8bbe91401e25d71fb88ceca3; tooling/linux_debian_setup_workflow.py=297d1dedc0a633277aa98f40a7162c9ef3627ba9; tooling/linux_installed_acceptance_summary.py=8a774d4e1d90c350ca79d6a3bfac722c4ec3fd55; tooling/linux_manual_export_build.sh=29c05203cd513e4322d0a323778053a7ccfd87c5; tooling/tests/linux_installed_product_smoke.py=8020ed8d1d68d59cff6acf70c55856fdeb2125e3; tooling/tests/test_linux_product_package.py=3bb3b520d3c4d15b1f4d43993c8cddb0f0c08ee1; tooling/tests/test_linux_deb_setup_package.py=2467e29b9eb13bd65065fea5a80872a3068cdfcd; tooling/tests/test_linux_installed_product_smoke.py=dc94f9b782d3699810bbe50230a5e9510a5739f7; tooling/tests/test_export_linux_product.py=079a187de49cf8b2fb6a7ddadb4175d6916163d1; tooling/tests/test_linux_gtk_bundle_smoke.py=2f962348665fc80c29cd040c9c399734f084eb6b; tooling/tests/run_linux_same_source_update_acceptance.py=6891a0a2db378577d30aec9c9466af594864b8ec; tooling/schema_check/check_schemas.py=7dca27717cbe5cccb45e1013b3436918a2322e75; tooling/conformance_tests/run_conformance_skeleton.py=c6f31d8cb561b48513c6784e54c85969c9cb953e; tooling/release_gate_check.py=22a8d7bfe9adb259f44ce8501fb215652e1d58c6; specs/d4_pocket_product_package_manifest.schema.json=3151ba81a869b87e6c0fa8f8633a0e424fc43ae7; specs/d4_pocket_linux_product_package_manifest.schema.json=34087f190e7189641b34ae3163b568a038c9f1c2; specs/device_link_request.schema.json=f0539522c5900148753c3e6eb500c935f7b11ac2; specs/mobile_device_link_channel_request.schema.json=5de3cbe5f3df5c60df77228e00f6a138ae319578; release_blockers.registry.json=244adc04a01e7b406c4ebcdeb829f7077d4aa35f。 |
| 試験区分（TestClass） | Apple manual workflow_dispatch、macOS XCTest／UI test、iOS Simulator XCTest、Linux UNIT／FIXTURE／manual LIVE_PRODUCT／workflow_dispatchのsourceを区別。#258/#277/#286は限定過去証拠、C25/C26履歴は元scopeを維持。本Phase36 mapping単位の製品test・manual workflow・Emulator／Simulator・installed QAはNOT_RUN。 |
| 証拠ID（Evidence ID） | NONE。#258/#277/#286およびP13/C25/C26の履歴はそれぞれのsource／OS／Artifact identity／scopeに固定し、Phase36またはN10の現行Acceptanceへ転記しない。 |
| 阻害要因（Blocker） | 既存Registry 18件（16 unresolved／2 resolved）、release_ready=falseを維持。linux_native_product_support、rev2_mobile_device_evidence、rev2_mobile_distribution、rev2_mobile_flutter_native_device_link_boundary等の既存状態を変更しない。新規Blocker／Evidenceなし。 |
| 状態 | Phase36のE0-04全21項目を現行N10/N11・線H/I・R07・W07・K17/K19・D4-LNX-T01〜T08へmapping済み。旧C25/C26 statusとLinuxの過去FAIL／限定PASSは履歴として保持。N10／N11／W07、Linux/macOS/iOS製品Acceptance、E0全体はOPEN。 |
| 残余 | Linux nativeの宣言対象distro・native X11／Wayland・通常導入・全必須Consumer・Task／Audit／Recovery・正式配布を後続BUILD/QC/QAへ結ぶ。macOS通常配布と同一candidate lifecycle、iOS device用Artifact／署名条件、AndroidのPhase35残差、全OS Artifact／Evidence／BlockerのN11行単位結合を継続する。 |

Phase36のE0-04欄を全21項目で記録する。84件（84 unique path）のSourceSHAは編集前main c84c86b3b9c185b9b6f1ff34d69e15b21e21033dのGit blobへ結合する。旧C25/C26の履歴statusを保持し、Android範囲はPhase35と重複試験しない。Linux native必須性とRegistry blocker 18件、release_ready=falseを維持し、新Evidence／Blockerなし。Run #258/#277/#286の限定Linux証拠を通常配布・実desktop／Wayland・Linux全体Acceptanceへ昇格しない。本mapping単位の製品試験・OS QAはNOT_RUN。

##### Phase 37 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 現行v4 N03「異種Runtime」、S05 K18、および工程R03線Bへ接続するD4追加範囲。 |
| 旧ID | `docs/D4_POCKET_PHASE_MAPPING.md` Phase 37「MINIDORA接続口」。rev1に同名の独立C工程はない。将来段階統合を扱うPhase 38とは分ける。 |
| 原文参照 | 現行v4仕様S03 ARC-C／S04 N03／S05 K18、工程R01 E0・R03線B、施工指示I02-07／I04-B／I05、旧Phase37行。仕様原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`。K18はMINIDORA等を外部Runtime／契約で接続し、非公開内部実装を対象外とする。 |
| 対象 | D4 Rust Broker起動時の`--minidora-runtime`登録→IPv4 loopback HTTP `MinidoraAdapter`→共通対話要求／応答→Runtime Center。Agent Task入口は同じRuntime対話と別のBroker capability判定を通る。 |
| 目的 | MINIDORAを外部Runtimeとして契約境界経由で接続し、Runtime固有の内部意味処理をD4 Shell Coreへ持ち込まず、製品production経路を未完成Runtimeへ依存させない。 |
| 生成元（`Producer`） | Broker起動引数の`--minidora-runtime <ID=127.0.0.1:port>`、外部Runtimeの公開HTTP応答、およびRuntime Centerからの対話要求。 |
| 実Consumer | `main.rs`／`ipc_server.rs`が登録値をBrokerへ渡し、`adapters/minidora.rs`が`/health`・`/api/capabilities`・`/api/chat`・`/api/trace/{trace_id}`を取得する。`runtime_dialogue_client.dart`がRuntime Center要求を既存Broker経路へ送る。Agent Task要求は`dialogue.rs`のmetadata検査とAdapter実装能力の両方を満たさない限り拒否される。 |
| 契約／Data | 公開API版`MINIDORA-PRODUCT-API-v1`、明示IPv4 loopback／port、対話Session ID、要求本文、応答・trace識別値とbounded sources。能力metadataは説明情報でありPermission／Approval／Trust／Agent Task実装を生成しない。非公開Brain・RAG・Memory等はD4へ複製しない。 |
| 正常時作用 | 有効な公開API応答はBroker所有のRuntime対話経路で応答投影される。これはSource上の接続範囲であり、本mappingでは実MINIDORA process／installed製品の起動・対話成功を確認していない。MINIDORA Agent Taskは未対応のまま表示・拒否する。 |
| 否定時作用 | 非loopback／port不正、HTTP／JSON／API版不正、Session／trace不一致、timeout／cancelはAdapter失敗へする。Agent TaskはTrait既定`AgentTask実行対応=false`または実Adapter method未実装ならBrokerが`AgentTask実行非対応`で拒否し、metadata宣言だけでは許可しない。 |
| 停止／Recovery | 対話の期限・取消・不正応答は失敗として終了し、Agent Task非対応は実行開始前に停止する。Brokerの認可・Audit境界を迂回しない。今回の対応はSource／既存test分類の照合であり、実Runtime障害後のinstalled Recovery／Audit耐久を再検証していない。 |
| 標準OS候補 | v4五OSの全てが製品対象。Windows、Linux native（Ubuntu 24.04 LTS／x86_64は現候補）、macOS、Android Emulator、iOS Simulatorの各installed条件はN03／N10と結合する。MINIDORA接続のOS別標準範囲・配布版・実Runtime証拠はUNKNOWN。物理非Windows端末は要求しない。 |
| 配布物 | D4製品へのMINIDORA内部実装bundleや専用Runtime配布物はない。現sourceのBroker起動引数は接続先登録面であり、利用者向け一般導入・外部Runtimeの同梱・全OS installed接続成立を示さない。 |
| 既存実装 | Rust `MinidoraAdapter`のloopback HTTP対話処理と境界test、Broker起動引数／Runtime登録、Agent Task能力の既定拒否とmetadata構造検査、Flutter Runtime Center／対話client、Runtime／Agent Task Schema・Python宣言コードがある。MINIDORA向けAgent Task実装はない。 |
| BUILD_GAP | N03が要求する独立した第二Task-capable Runtime、その実Task・失敗・権限・結果・Auditの製品経路は未成立。MINIDORAは対話Adapterのみ。実MINIDORAとのinstalled接続、五OS parity、通常配布・Recovery証拠も未取得。これは既存N03/W03/G3のOPEN範囲であり新Acceptance／Blockerを作らない。 |
| ソース識別値（`SourceSHA`） | 照合HEAD=`1cd0bc2828f519682fb2688b4b339323b6832829`。以下24 pathのGit blobを個別に記録・照合: `apps/desktop_flutter/lib/screens/agent_center.dart`=`2472faeb31a64d624423c30a0a30d0f153d2e2a3`; `apps/desktop_flutter/lib/screens/runtime_center.dart`=`d2c7af172073d2b10c76a6416b90f011f10e95db`; `apps/desktop_flutter/lib/services/runtime_dialogue_client.dart`=`a0853ee7327cafa60ee89b0c5ee49312b73c46c4`; `apps/desktop_flutter/lib/services/agent_task_client.dart`=`f54cefac29eba3dcf383bba19b4f64ee682e6217`; `apps/desktop_flutter/test/runtime_dialogue_test.dart`=`899912f92ee3751f8616632d434e04f95ee5ef77`; `apps/desktop_flutter/test/agent_task_client_test.dart`=`0477a0bbbb321082494b6b55711815d34419fdd9`; `native/rust_helper/src/main.rs`=`b0a5df285db357172e7970074d7a67189f3384a3`; `native/rust_helper/src/adapters/codex_cli.rs`=`e5c9d1581ae9a1c0a3ae7fcfa626fdca6fea6d28`; `native/rust_helper/src/adapters/minidora.rs`=`31c756ac7c9a264d73ed0e9654dbae599be83921`; `native/rust_helper/src/broker/adapter_center.rs`=`66575bb77e4a375dca4f8488f97df442960bb5a9`; `native/rust_helper/src/broker/dialogue.rs`=`c3c3fddb630dd06b7e6cc1c13b39f3d4f6f6a999`; `native/rust_helper/src/broker/ipc_server.rs`=`58a404b920ef5905e174668ec2de0e04a9e8b044`; `native/rust_helper/src/broker/protocol.rs`=`9b2076d5f31ca6d77f1f87973d1b394aaf751961`; `native/rust_helper/src/broker/runtime_registry.rs`=`9ff28610862586a599848d75f9e891a6c6cadf2f`; `native/rust_helper/tests/agent_task_production_e2e.rs`=`fdc7565aa58fa7466e6e24e5492321e8015567eb`; `native/rust_helper/tests/agent_task_crash_recovery_e2e.rs`=`23517fcfe8ef94b2357a94e04879d423c3360501`; `packages/agent_runtime/contract.py`=`9e05ca1dd63883ad21d1871c62ddfdfef307d82f`; `packages/runtime_catalog/catalog.py`=`003d3c533cbb48bb5f6520cb9b7a3566f2493748`; `specs/runtime.schema.json`=`bdef33f26fe06b334eb6f94124930fd08594bbc9`; `specs/agent_runtime.schema.json`=`b8e031e53f582048211f0e1a6deeb2e5cb87e4fb`; `specs/agent_adapter.schema.json`=`e0792589a2bae168ba221af4f4f9abb505815c12`; `specs/agent_cli_runtime_workspace_registration.schema.json`=`32a09f2ce33a5a215d64350ee22e24e93f9037c7`; `specs/agent_task_request.schema.json`=`cb25bb3684656bfd670ea5832bbb7fc5988845b6`; `specs/agent_task.schema.json`=`a552389072088d6cd42b3588fefda332be4ae6b6`。全24 pathは同一HEADのblob hashと一致。 |
| 試験区分（`TestClass`） | `SOURCE`／既存`UNIT`／`FIXTURE`の実装・test sourceを確認。Adapter HTTP境界、BrokerのTask capability否定、Flutter表示の既存caseがある。MINIDORA実接続・第二Runtime Agent Taskを証明するtestは確認できず、本mapping単位では既存Rust／Flutter testも実行していない。 |
| 証拠ID（`Evidence ID`） | `NONE`。Source mappingのみ。MINIDORAの実Runtime対話／Agent Task／installed productの証拠へ昇格しない。 |
| 阻害要因（`Blocker`） | N03／W03／G3はOPEN。Registry 18件（16 unresolved／2 resolved）と`release_ready=false`を維持する。K18への直接Blocker帰属はN11で全数照合する。新規Blocker／Evidenceなし。 |
| 状態 | Phase37のE0-04全21項目と24 SourceSHAを現mainへ結合済み。これは要求consumer mappingの完了であり、N03／K18の製品Acceptance、E0全体はOPEN。 |
| 残余 | 独立した第二Agent Task Runtimeが必要。MINIDORAを後でTask対応させる場合も、公開契約・Broker既存認可／Audit／Recovery consumerへ別途接続し、Task実行・拒否・失敗の実証を要する。内部実装の輸入とproduction依存はしない。五OSのruntime／package／installed evidenceはN03／N10へ残す。 |

Phase37のE0-04欄を全21項目で記録し、24件（24 unique path）のSourceSHAを編集前main `1cd0bc2828f519682fb2688b4b339323b6832829`のGit blobへ結合する。MINIDORAの対話AdapterとAgent Task非対応を分け、内実輸入・production dependencyを追加しない。N03／W03／G3、Registry blocker18件、`release_ready=false`は維持し、新Evidence／Blockerなし。MINIDORA実接続・Agent Task・Rust／Flutter product tests・五OS QAはNOT_RUN。


##### Phase 38 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 現行v4 N03／N08、S05 K18／K20、工程R03-B／Fに接続するD4追加範囲。旧Phase38の照合単位であり、新しい製品Acceptance IDではない。 |
| 旧ID | `docs/D4_POCKET_PHASE_MAPPING.md` Phase 38「MINIDORA段階統合」。rev1に同一の独立C工程はない。Phase37の外部Runtime接続口と、将来の段階統合候補を区別する。 |
| 原文参照 | 現行v4仕様S04 N03／S05 K18・K20／S06 N08、工程R03線B・F、施工指示I02-07／I04-B／I04-F／I05、および現行Phase対応表。v4採用仕様SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`。旧Phase38行の「referenceとの一致試験等」は将来範囲の記述。現行v4 3点セットは置換対象・受入値を有限に定義しておらず、本mappingで条件を追加しない。 |
| 対象 | MINIDORAの外部Runtime接続と、将来検討する意味処理moduleの段階置換境界。現行は公開HTTP対話Adapterであり、Shell Core内のMINIDORA内部処理module、切替機構、module parity consumerは確認できない。 |
| 目的 | MINIDORA非公開実装をShell Coreへ複製せず、Runtime／Bufferの既存契約とBroker責任境界を保ったまま、旧Phase38の将来範囲と現Consumerの差を識別する。現production pathへ依存を追加しない。 |
| 生成元（`Producer`） | 現行の固定reference sourceは`tooling/minidora_live_check.py`がpinするMINIDORA commit `3400a3bb68b37efa1dc14ee8aaa28fda779bf1f8`。開発用serverが公開HTTP APIを提供し、BrokerのRuntime対話要求を受ける。置換候補moduleの生成元は現sourceに存在しない。 |
| 実Consumer | `main.rs`／`ipc_server.rs`から登録される`MinidoraAdapter`は`/health`・`/api/capabilities`・`/api/chat`等を呼ぶ外部対話境界。`runtime_dialogue_client.dart`はRuntime Center要求をBrokerへ送る。`dialogue.rs`はAgent TaskでRuntime metadataと実Adapter methodをともに検査する。`minidora_live_check.py`は固定referenceを2 instance起動してBroker対話を行うが、module置換比較consumerではない。`run_authority_parity.py`はAuthority parity用で、MINIDORA意味出力のparityではない。 |
| 契約／Data | 現行公開API版`MINIDORA-PRODUCT-API-v1`、Runtime／Agent Adapter／Agent TaskのJSON Schema、Broker対話要求・応答、N08の非権限`buffer_intervention_record` v2。reference commitは検査source上で固定される。module input/output vector、意味同等性schema、module切替receiptは現行consumerとして未確認。Capability metadataはAuthorityを生成しない。 |
| 正常時作用 | 開発用`minidora_live_check.py`には固定referenceをRust Brokerへ接続して実対話する`LIVE_RUNTIME`検査経路がある。現行production Adapterは外部Runtimeとの対話だけを行う。代替moduleの参照出力との比較・段階切替・結果切戻しconsumerはない。本mappingではlive checkを実行していない。 |
| 否定時作用 | 現在のAdapterはloopback／API version／HTTP・JSON・session／trace／期限・cancelを検査する。Agent Taskは既定unsupportedまたは実Adapter methodなしならBrokerが拒否する。candidate moduleの不一致・malformed output・failure-isolation・fallbackを比較して切替可否を決める実consumerは未確認。 |
| 停止／Recovery | 現行対話失敗とTask非対応はBroker経路で失敗／拒否し、Authorityを引き継がない。N08のjournal／scratch RecoveryはCodex CLI Task限定で、MINIDORA moduleのfailure isolationやfallbackを回復するものではない。module交換後のRecovery consumerはない。 |
| 標準OS候補 | v4対象のWindows／Linux native／macOS／Android／iOSを維持。`minidora_live_check.py`の参照server＋Rust Brokerは開発用経路で、5 OS installed module integrationやLinux native配布・GUI保証を示さない。OS別module候補・標準保証値はUNKNOWN。 |
| 配布物 | D4製品にMINIDORA内部moduleの同梱・置換bundleはない。`--minidora-runtime`は外部loopback Runtime登録用で、通常配布・installed module switch・rollbackを提供しない。 |
| 既存実装 | Rust `MinidoraAdapter`と固定referenceを利用する開発用live harness、Broker共通対話／Agent Task能力検査、Codex CLI Task Adapterと限定WorkspaceTaskScratch/Buffer consumerがある。`tooling/broker_parity/run_authority_parity.py`はauthority検査。MINIDORA内部の意味module／module parity／切替consumerは確認できない。 |
| BUILD_GAP | 旧Phase38が示す将来reference比較をmodule置換へ結ぶconsumerは現行sourceにない。固定reference対話harnessは存在するが、代替moduleの比較・切替を行わない。現行v4のmodule置換Acceptanceは未定義としてUNKNOWNに保ち、既存N03/N08 OPEN範囲を維持して新Blockerを追加しない。 |
| ソース識別値（`SourceSHA`） | 照合HEAD=`b76552b44f90e958e0e98797dd278a68be0e471f`。以下38 pathのGit blobを編集前HEADとworking treeで個別一致確認：`AGENTS.md`=`1eafcadd129fd01f1f2f683594c381605ed7d48d`<br>`docs/QE_2026-11-13/01_D4Pocket_統合製品実装仕様書_v4.md`=`352c53180cbad07888beb956aad0fb2a58074c58`<br>`docs/QE_2026-11-13/02_D4Pocket_統合開発工程表_v4.md`=`d32dd251e5f7cbb97d92f6e7540d6b6ed0ecec05`<br>`docs/QE_2026-11-13/03_D4Pocket_Codex施工指示書_v4.md`=`49447973e88c50a09b2f4978adfed84fd464c6d0`<br>`docs/QE_2026-11-13/採用記録.md`=`f159c1aa6ea5bc6e80e0877d8068f7e44da4c4d3`<br>`docs/QE_2026-11-13/E0_要求consumer台帳.md`=`87472a85ce41fb6434c5a1204bbb9fff5beeb026`<br>`docs/QE_2026-11-13/進捗.md`=`f549c6cbd7b92e11a2276778f57b4fa39d987869`<br>`docs/D4_POCKET_PHASE_MAPPING.md`=`064ea334f86b137d94fba2370d2a2fd732f6e972`<br>`docs/緩衝基盤_rev5補遺_QC/実装仕様書.md`=`2d0e98fb0b58ceec65244fde6a8fbf8e27c49b7e`<br>`docs/緩衝基盤_rev5補遺_QC/工程表.md`=`a5092b7b77361f147ce1f188bee71081a1aa8490`<br>`docs/緩衝基盤_rev5補遺_QC/B0_原因対応台帳.md`=`299c68b3a5266ab4b1946f4b06caa9e4069274a9`<br>`docs/緩衝基盤_rev5補遺_QC/B3_全原因機構対応.md`=`8a25f3ddacada889ba95de1d4761af77a074d26b`<br>`docs/緩衝基盤_rev5補遺_QC/B4_製品接続設計.md`=`7d74b5fbfa2f31a774513c8bc9214caa7fa775a1`<br>`docs/緩衝基盤_rev5補遺_QC/B5_自己適用.md`=`16081537b6699c176144e32ca97d6bdd20d98866`<br>`apps/desktop_flutter/lib/screens/agent_center.dart`=`2472faeb31a64d624423c30a0a30d0f153d2e2a3`<br>`apps/desktop_flutter/lib/screens/runtime_center.dart`=`d2c7af172073d2b10c76a6416b90f011f10e95db`<br>`apps/desktop_flutter/lib/services/agent_task_client.dart`=`f54cefac29eba3dcf383bba19b4f64ee682e6217`<br>`apps/desktop_flutter/lib/services/runtime_dialogue_client.dart`=`a0853ee7327cafa60ee89b0c5ee49312b73c46c4`<br>`native/rust_helper/src/main.rs`=`b0a5df285db357172e7970074d7a67189f3384a3`<br>`native/rust_helper/src/adapters/mod.rs`=`06c540a59437b903b3f045b5aafb3790c21103bd`<br>`native/rust_helper/src/adapters/minidora.rs`=`31c756ac7c9a264d73ed0e9654dbae599be83921`<br>`native/rust_helper/src/adapters/codex_cli.rs`=`e5c9d1581ae9a1c0a3ae7fcfa626fdca6fea6d28`<br>`native/rust_helper/src/broker/adapter_center.rs`=`66575bb77e4a375dca4f8488f97df442960bb5a9`<br>`native/rust_helper/src/broker/agent_task_scratch.rs`=`de743cd819fb4533c9c3070c4e7bcae82e42d938`<br>`native/rust_helper/src/broker/dialogue.rs`=`c3c3fddb630dd06b7e6cc1c13b39f3d4f6f6a999`<br>`native/rust_helper/src/broker/ipc_server.rs`=`58a404b920ef5905e174668ec2de0e04a9e8b044`<br>`native/rust_helper/src/broker/protocol.rs`=`9b2076d5f31ca6d77f1f87973d1b394aaf751961`<br>`native/rust_helper/src/broker/runtime_registry.rs`=`9ff28610862586a599848d75f9e891a6c6cadf2f`<br>`native/rust_helper/tests/agent_task_production_e2e.rs`=`fdc7565aa58fa7466e6e24e5492321e8015567eb`<br>`native/rust_helper/tests/agent_task_crash_recovery_e2e.rs`=`23517fcfe8ef94b2357a94e04879d423c3360501`<br>`tooling/minidora_live_check.py`=`400bf04a05480ffaf77e4960dbf5abdd0904427f`<br>`tooling/broker_parity/run_authority_parity.py`=`4f0fd65cbb1fb9bdcc4223d05d0915a50e8b0f78`<br>`specs/runtime.schema.json`=`bdef33f26fe06b334eb6f94124930fd08594bbc9`<br>`specs/agent_runtime.schema.json`=`b8e031e53f582048211f0e1a6deeb2e5cb87e4fb`<br>`specs/agent_adapter.schema.json`=`e0792589a2bae168ba221af4f4f9abb505815c12`<br>`specs/agent_task.schema.json`=`a552389072088d6cd42b3588fefda332be4ae6b6`<br>`specs/agent_task_request.schema.json`=`cb25bb3684656bfd670ea5832bbb7fc5988845b6`<br>`specs/buffer_intervention_record.schema.json`=`26a1fa8ee9d010f18c157f5d3b66b9b34bdf94d5`。 |
| 試験区分（`TestClass`） | `SOURCE`／既存`UNIT`・`FIXTURE`と、pin済み外部referenceを起動する開発用`LIVE_RUNTIME` harness sourceを確認。HarnessはRuntime対話・Broker境界を対象にし、MINIDORA module parity／Agent Task実装を証明しない。本mapping単位でRust／Flutter／live harnessは未実行。 |
| 証拠ID（`Evidence ID`） | `NONE`。Source→Consumer mappingのみ。固定reference harness sourceの存在を実行済みLIVE_RUNTIME evidenceへ昇格しない。 |
| 阻害要因（`Blocker`） | N03／W03／G3とN08／W08／G7はOPEN。Release Registry 18件（16 unresolved／2 resolved）、`release_ready=false`を維持。K18／K20への直接Blocker・Evidence結合はN11の残作業。新規Blocker／Evidenceなし。 |
| 状態 | Phase38のE0-04全21項目と38 SourceSHAを現mainへ結合済み。Phase38のmodule置換Acceptance、N03／N08／K18／K20、E0全体はOPEN。 |
| 残余 | 現行v4ではmodule置換対象・比較値・切替条件が未定義のためUNKNOWNに残す。旧版の詳細gateを自動継承せず、reference live harnessの実対話を置換Acceptanceへ昇格しない。N03／N08の既存残件、N11 Evidence／Blocker／五OS Artifact結合を継続。 |


##### Phase 39 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 旧Phase39/C28を現行v4 N01／N08／N11、S05 K19／K20、QA Q4へ対応するE0照合。N11のEvidence結合対象であり、Q4や製品Acceptanceを閉じるIDではない。 |
| 旧ID | `docs/D4_POCKET_PHASE_MAPPING.md` Phase 39「長時間運用」／`C28`。短時間試験を長時間合格へ昇格しないという履歴条件を保持する。 |
| 原文参照 | 現行v4仕様S04 N01／N08／N11、S05 K19／K20、S08 QE_QA Q4、工程R03線F/J・R08 Q4、施工指示I07-04／I07-07、`docs/FINAL_QA_QUEUE.md` Q4。`docs/specs/long-run-validation.md`と`docs/REV2_PROGRESS.md`はC28の実装範囲／失敗履歴を示す補助資料。 |
| 対象 | 統合候補の長時間Task運用、Runtime/Broker再起動・接続断、bounded history/Audit、resource／storageの持続観測。現C28 runnerは開発用Rust Broker＋localhost Runtime fixtureで、v4 Q4は候補全体の反復Task／Compare／Handoffと資源・OSを対象とする。 |
| 目的 | 正常・予定内接続断・予期しない失敗・再起動後の状態を長時間条件で観測し、source・candidate・OS・証拠classを固定する。開発fixtureの成功をinstalled product、実Runtime、製品SLAへ拡張しない。 |
| 生成元（`Producer`） | `tooling/long_run_validation.py`の`--duration-seconds`／`--duration-hours`／`--interval-seconds`、反復Dialogue、localhost MINIDORA API fixture、Broker再起動・Runtime停止／復帰、Broker resource/store observation。 |
| 実Consumer | C28 runnerはdebug Rust helperをbuildし、normal IPCとowner IPCでBroker processを起動する。`LongRun.run`はDialogue、fixture restart、development lifecycle fixture、Broker restart、network disconnect/reconnect、bounded historyとresource samplesを繰り返し、report JSONを更新する。`full_regression_validation.py`はC28を30秒で呼ぶsourceであり、正式Q4 consumerではない。ConformanceはC28 unit suiteを起動する。 |
| 契約／Data | C28 CLIの既定30秒と明示hour option、atomic reportの`running/passed/failed`、最大512 resource samples／32 HTTP result／128 trace references、固定failure codesと診断allowlist。Reportはcredentialや対話本文を保存せず、runtime Report Schema／N11受入bindingの実consumerは確認できない。 |
| 正常時作用 | source上は実Broker IPC・永続store・履歴を通るDialogueの反復、fixture／Broker再起動、bounded metrics採取が実装される。証拠classはBroker `LIVE_RUNTIME`、Runtime `FIXTURE`、metrics `INTERNAL_STATE`。現candidateの8時間完遂は未確認で、sourceの存在は成功結果ではない。 |
| 否定時作用 | 停止中fixtureへのDialogueを失敗として扱い、復帰後に再接続する。予期しない結果・timeout・Broker拒否はfixed failure codeでfailedを返す。fixture serverのwrite/flush記録はclient受信・検証を証明しない。clean commitの歴史的8時間runは22.766秒でfailedし、根因は未確定のまま維持する。 |
| 停止／Recovery | runnerはRuntime fixtureとBrokerを再起動し同じ永続storeへ戻る。Broker shutdownが10秒で完了しない場合は子processをkillする。期限後の資格file削除は20回まで再試行し失敗時はrunをfailedにする。最終temp root削除は`ignore_errors=True`のbest-effortであり、不在確認を生成しない。 |
| 標準OS候補 | v4五OSを保証対象として維持。Phase39の旧C28履歴はWindows長時間運転が中心。runnerはWindows/Linuxのhelper binary名を分けるが、各OS installed product／GUI／native authority／配布でのC28/Q4証拠を意味しない。Q4の対象OS resource QAは未実施。 |
| 配布物 | C28は`native/rust_helper/target/debug`の開発helperと開発lifecycle fixtureを使う。標準installer、installed Release candidate、署名済配布物、外部Runtimeを使う経路ではない。 |
| 既存実装 | `long_run_validation.py`、C28固有unit suite、Conformanceへのunit起動登録、`full_regression_validation.py`の30秒smoke呼出しがある。C28 reportは範囲を自己記述する一方、最終reportはcurrent Q4／Release Registryへ自動受入結合されない。 |
| BUILD_GAP | 現行source baselineの8時間／24時間candidate完遂・正式QA evidenceはない。既存unit／Conformanceはharness境界だけ、C30側の30秒設定は長時間Acceptanceではない。C28 runnerはDialogue中心で、v4 Q4が列挙する同一候補の反復Task／Compare／Handoff、installed product、対象OS resource／Audit増加を一括証明しない。N11証拠結合とQ4はOPEN／DEFERRED。新Blockerは追加しない。 |
| ソース識別値（`SourceSHA`） | 照合HEAD=`f53035662078c5179819f4148413f3287c144f6d`。以下38 pathのGit blobを編集前HEADとworking treeで個別一致確認：`AGENTS.md`=`1eafcadd129fd01f1f2f683594c381605ed7d48d`<br>`docs/QE_2026-11-13/01_D4Pocket_統合製品実装仕様書_v4.md`=`352c53180cbad07888beb956aad0fb2a58074c58`<br>`docs/QE_2026-11-13/02_D4Pocket_統合開発工程表_v4.md`=`d32dd251e5f7cbb97d92f6e7540d6b6ed0ecec05`<br>`docs/QE_2026-11-13/03_D4Pocket_Codex施工指示書_v4.md`=`49447973e88c50a09b2f4978adfed84fd464c6d0`<br>`docs/QE_2026-11-13/採用記録.md`=`f159c1aa6ea5bc6e80e0877d8068f7e44da4c4d3`<br>`docs/D4_POCKET_PHASE_MAPPING.md`=`064ea334f86b137d94fba2370d2a2fd732f6e972`<br>`docs/QE_2026-11-13/E0_要求consumer台帳.md`=`f8044860367c9f2c05d0a110b521ae680f25a63d`<br>`docs/QE_2026-11-13/進捗.md`=`9afb8e85ad58a03c2492efc926d703e8125a7f3e`<br>`docs/FINAL_QA_QUEUE.md`=`7b332cccab70bb68aa1ff4f8e23acc0e816d8282`<br>`docs/specs/long-run-validation.md`=`ebba5dc1cc97fa69525502d12dfc059e89023380`<br>`docs/specs/full-regression-validation.md`=`dd5953d3bea595c2f25474ed0a9c42cf01ff51e3`<br>`docs/REV2_PROGRESS.md`=`56f0f168b22d1e64303db2bc6f2c66668393dba2`<br>`release_blockers.registry.json`=`244adc04a01e7b406c4ebcdeb829f7077d4aa35f`<br>`tooling/long_run_validation.py`=`00d1ec4fa0a7b7fea4ef678a94a56f20367a243c`<br>`tooling/conformance_tests/test_long_run_validation.py`=`3976e57497d86aa8a3ca612758227c091db617cb`<br>`tooling/full_regression_validation.py`=`8a33a9fab9ecbc18119302ae312201d41618012e`<br>`tooling/conformance_tests/run_conformance_skeleton.py`=`c6f31d8cb561b48513c6784e54c85969c9cb953e`<br>`tooling/manifest.py`=`aa4525ff20293cb8641e5598a2439561a3bfbd3b`<br>`tooling/release_gate_check.py`=`22a8d7bfe9adb259f44ce8501fb215652e1d58c6`<br>`tooling/evidence_bundle.py`=`68dead851236b0f1b69899151c37371e593ff3f9`<br>`tooling/release_runtime_assertions.py`=`0a0a351fe17151ad86517bd0a965358f40c61490`<br>`tooling/日本語基底監査.py`=`77474767dd93eb4d69382af5ea4a0ea384eeccbe`<br>`native/rust_helper/src/main.rs`=`b0a5df285db357172e7970074d7a67189f3384a3`<br>`native/rust_helper/src/adapters/minidora.rs`=`31c756ac7c9a264d73ed0e9654dbae599be83921`<br>`native/rust_helper/src/broker/adapter_center.rs`=`66575bb77e4a375dca4f8488f97df442960bb5a9`<br>`native/rust_helper/src/broker/ipc_server.rs`=`58a404b920ef5905e174668ec2de0e04a9e8b044`<br>`native/rust_helper/src/broker/dialogue.rs`=`c3c3fddb630dd06b7e6cc1c13b39f3d4f6f6a999`<br>`native/rust_helper/src/broker/runtime_registry.rs`=`9ff28610862586a599848d75f9e891a6c6cadf2f`<br>`native/rust_helper/src/broker/audit.rs`=`eb1dcf5018d95c7fdf6171aadd4bca9554d1996f`<br>`native/rust_helper/src/broker/store.rs`=`356b490e403b693db499befb777789784b57d40d`<br>`native/rust_helper/src/broker/execution_history.rs`=`8eeb1c26d06a9e4b01806413a4bfa8c2a73357d6`<br>`native/rust_helper/src/broker/agent_task_scratch.rs`=`de743cd819fb4533c9c3070c4e7bcae82e42d938`<br>`specs/runtime.schema.json`=`bdef33f26fe06b334eb6f94124930fd08594bbc9`<br>`specs/runtime_resource_query.schema.json`=`c7617ba9c2283255fb04b6cfa529ff7b04f6137f`<br>`specs/runtime_resource_observation.schema.json`=`62f587b8c88d0c5c0bd316eaa3d0a15c9c1bd2d1`<br>`specs/audit.schema.json`=`01da4c598b52b2ae80ebf46f9e9050d7bec16ca3`<br>`specs/runtime_execution_history.schema.json`=`5b260e9b6026758dc0a730d3381b4f7d72ba2e98`<br>`specs/buffer_intervention_record.schema.json`=`26a1fa8ee9d010f18c157f5d3b66b9b34bdf94d5`。 |
| 試験区分（`TestClass`） | `SOURCE`／`UNIT`／`LIVE_RUNTIME`／`FIXTURE`／`INTERNAL_STATE`を分離。`tooling/conformance_tests/run_conformance_skeleton.py`の272 checks内でC28 unit suiteが実行される。C28 unit suiteはpassしたが、8時間／24時間runner本体・installed product・Q4全OS QAは本E0 mappingでは未実行。 |
| 証拠ID（`Evidence ID`） | `NONE`。current v4 candidateのC28長時間Evidenceは未発行。過去の8時間FAILは`docs/REV2_PROGRESS.md`の当時のsource・条件・report hashへ限定した履歴であり、current PASSにはしない。 |
| 阻害要因（`Blocker`） | Release Registry 18件（16 unresolved／2 resolved）と`release_ready=false`を維持。N11はOPEN、Q0〜Q7は`DEFERRED`。C28 old failureを新規Blockerへ複製せず、K19/K20のEvidence／Blocker完全結合はN11の残作業。 |
| 状態 | Phase39のE0-04全21項目と38 SourceSHAを現mainへ結合済み。Phase39/C28の長時間Acceptance、v4 Q4、N11、E0全体は未成立／OPENのまま。 |
| 残余 | 8時間／24時間candidate試験はQ4開始条件・同一候補・対象OSと結合して後段で行う。歴史的22.766秒FAIL、120秒等の短時間限定結果は保持し、現candidateへ転記しない。runnerの最終temp-root cleanupはbest-effortで不在未確認だが、現v4のPhase39 E0 mappingでは新Acceptance／Blockerにせず、長時間検証実行時のartifact cleanup確認へ送る。 |


##### Phase 40 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 旧Phase/Cの障害注入を現行v4 S04 N01/N09/N11、S05 K19、S08 Q2へ証拠境界付きで対応。C29自体は現行v4の製品Acceptance IDではない。 |
| 旧ID | D4 Phase 40「障害注入」／rev1 C29。歴史的な9/9 PASSは当時sourceの限定証拠として保持し、現candidateのPASSへ転記しない。 |
| 原文参照 | 現行v4仕様S04 N01/N09/N11・S05 K19・S08 Q2、工程R03/R08、施工指示I07-02、`docs/D4_POCKET_PHASE_MAPPING.md` Phase40、`docs/specs/failure-injection-validation.md`、`docs/FINAL_QA_QUEUE.md` Q2。`docs/REV2_PROGRESS.md`とrev1 progressは当時の条件・履歴の補助資料。 |
| 対象 | 開発用`tooling/failure_injection_validation.py`と`LongRun` harnessが、debug Rust Broker IPC/process、localhost Runtime／MCP stdio／A2A loopback fixture、一時storeへ注入する障害。 |
| 目的 | 故障を成功・Authority・Permission/Approval・Audit・Recovery完了へ昇格させず、失敗分類と限定復帰を観測する。開発用caseの結果をinstalled製品・実外部サービスへ広げない。 |
| 生成元（`Producer`） | `case_runtime_crash`、`case_broker_crash`、`case_mcp_timeout`、`case_mcp_malformed_schema`、`case_a2a_timeout`、`case_credential_unavailable`、`case_disk_full_simulation`、`case_audit_failure`、`case_corrupt_state`の9 case。 |
| 実Consumer | `tooling/validate_all.py`の`c29_failure_injection_smoke`が`python tooling/failure_injection_validation.py`を直接起動し、scriptがdebug helperをbuild、実Broker IPC/processとfixtureを通してcase結果JSONをstdoutへ返す。C30 regressionは別step。Conformance 272 checksはC29全runnerを呼ばない。 |
| 契約／Data | C29結果は`passed/failed`、case別status・経過ms・限定観測・証拠classと「installed製品の証明ではない」flagを持つstdout JSONで、専用Schema/Evidence IDへ自動結合しない。Credentialはmissing refだけを使い、実値・応答本文を出さない。補助仕様はsourceと不一致だった8件記述を9件へ同期した。 |
| 正常時作用 | 9 case全てで期待する失敗／停止・限定回復が成立するとcase statusをpassedとする。Runtime停止後の再接続、Broker再起動時のactive state拒否、stdio Tool Schema dialect拒否を含む。scriptがC29経路を実行するsourceは存在するが本作業でrunnerは未実行。 |
| 否定時作用 | Runtime停止中Dialogueを失敗、Broker kill後IPCを失敗、MCP timeout・未対応inputSchema dialect・A2A timeoutを拒否、Credential unavailableをchild起動前に拒否、store書込不能をBroker起動拒否、Audit append不能をsuspended、malformed A2A stateをrestart拒否にする。disk full caseは実容量枯渇でない。 |
| 停止／Recovery | Runtime fixtureを再起動してDialogueを再確認、Broker crash後は明示起動、Audit pathを試験後に戻し、corrupt stateでは起動を拒否する。caseはfinallyで`LongRun.close`へ進むが、shared temp cleanupの独立した不在証拠はこのsourceにない。 |
| 標準OS候補 | v4五OS範囲を維持。C29 source mappingはOS別installed候補、標準OS版／arch／GUI／権限のQ2証拠を作らない。WSL/fixture/開発実行をLinux nativeまたは五OS製品QAへ転記しない。 |
| 配布物 | ローカル`native/rust_helper/target/debug`の開発helperと一時fixture/store。通常Release、正式Installer、署名済Artifact、実外部Runtime/MCP/A2Aではない。 |
| 既存実装 | 9 case sourceと`validate_all.py`上のC29専用smoke consumerがある。`docs/specs/failure-injection-validation.md`の件数不一致を本作業で9へ補正。歴史的9/9 runは現HEADとは別source。C29 reportはN11へ自動固定されない。 |
| BUILD_GAP | C29開発検証経路はsource上WIRED。現v4同一Release candidateの統合Q2、installed product、実外部接続、実容量枯渇、五OS結果は未成立で後段QAに残る。新しいPhase40 blockerは追加しない。 |
| ソース識別値（`SourceSHA`） | 照合HEAD=`84df37cbc095e1a3b72f50662057d371bbf63c15`。編集前baselineの47 unique tracked pathを個別照合し、全blob SHA一致。`AGENTS.md`=`1eafcadd129fd01f1f2f683594c381605ed7d48d`<br>`docs/QE_2026-11-13/01_D4Pocket_統合製品実装仕様書_v4.md`=`352c53180cbad07888beb956aad0fb2a58074c58`<br>`docs/QE_2026-11-13/02_D4Pocket_統合開発工程表_v4.md`=`d32dd251e5f7cbb97d92f6e7540d6b6ed0ecec05`<br>`docs/QE_2026-11-13/03_D4Pocket_Codex施工指示書_v4.md`=`49447973e88c50a09b2f4978adfed84fd464c6d0`<br>`docs/QE_2026-11-13/採用記録.md`=`f159c1aa6ea5bc6e80e0877d8068f7e44da4c4d3`<br>`docs/D4_POCKET_PHASE_MAPPING.md`=`064ea334f86b137d94fba2370d2a2fd732f6e972`<br>`docs/QE_2026-11-13/E0_要求consumer台帳.md`=`0f40651b704dbc7717d26cc5beba98db53567578`<br>`docs/QE_2026-11-13/進捗.md`=`bbc71e9322fcb28eabc0f11c008b475596cbd73a`<br>`docs/FINAL_QA_QUEUE.md`=`7b332cccab70bb68aa1ff4f8e23acc0e816d8282`<br>`docs/specs/failure-injection-validation.md`=`14d2a946f424b0e800a4a271c51da8b5f797dfac`<br>`docs/REV2_PROGRESS.md`=`56f0f168b22d1e64303db2bc6f2c66668393dba2`<br>`docs/総合機能拡張_rev1/進捗.md`=`1392588f0cd5273013578e84d8e33c5c9dc9a01a`<br>`release_blockers.registry.json`=`244adc04a01e7b406c4ebcdeb829f7077d4aa35f`<br>`tooling/failure_injection_validation.py`=`cace434de47d60f7f848656f229336d8293908ff`<br>`tooling/long_run_validation.py`=`00d1ec4fa0a7b7fea4ef678a94a56f20367a243c`<br>`tooling/validate_all.py`=`2744f9a10702872f6dc0aad705f922466fc198fc`<br>`tooling/full_regression_validation.py`=`8a33a9fab9ecbc18119302ae312201d41618012e`<br>`tooling/conformance_tests/run_conformance_skeleton.py`=`c6f31d8cb561b48513c6784e54c85969c9cb953e`<br>`tooling/manifest.py`=`aa4525ff20293cb8641e5598a2439561a3bfbd3b`<br>`tooling/schema_check/check_schemas.py`=`7dca27717cbe5cccb45e1013b3436918a2322e75`<br>`tooling/release_gate_check.py`=`22a8d7bfe9adb259f44ce8501fb215652e1d58c6`<br>`tooling/evidence_bundle.py`=`68dead851236b0f1b69899151c37371e593ff3f9`<br>`tooling/release_runtime_assertions.py`=`0a0a351fe17151ad86517bd0a965358f40c61490`<br>`tooling/日本語基底監査.py`=`77474767dd93eb4d69382af5ea4a0ea384eeccbe`<br>`native/rust_helper/Cargo.toml`=`6352fd7de3188b4c7335167f4f8875a4f8f4e28b`<br>`native/rust_helper/Cargo.lock`=`8647830893561de17e82063231b651322c8909bd`<br>`native/rust_helper/src/main.rs`=`b0a5df285db357172e7970074d7a67189f3384a3`<br>`native/rust_helper/src/adapters/minidora.rs`=`31c756ac7c9a264d73ed0e9654dbae599be83921`<br>`native/rust_helper/src/adapters/mcp_stdio.rs`=`b7e08327541ad75bca53bb1563e0e20d576d9b66`<br>`native/rust_helper/src/broker/protocol.rs`=`9b2076d5f31ca6d77f1f87973d1b394aaf751961`<br>`native/rust_helper/src/broker/ipc_server.rs`=`58a404b920ef5905e174668ec2de0e04a9e8b044`<br>`native/rust_helper/src/broker/dialogue.rs`=`c3c3fddb630dd06b7e6cc1c13b39f3d4f6f6a999`<br>`native/rust_helper/src/broker/runtime_registry.rs`=`9ff28610862586a599848d75f9e891a6c6cadf2f`<br>`native/rust_helper/src/broker/authority.rs`=`067a36442138897aac396bdb588c2e01f90a5318`<br>`native/rust_helper/src/broker/mcp_center.rs`=`8f9ab897bdc625e448241ebd9b46f995cd1f22f6`<br>`native/rust_helper/src/broker/a2a_center.rs`=`73ad601abd64a33ccefde4b73ab5f9dc4f728504`<br>`native/rust_helper/src/broker/credential_vault.rs`=`029dccda7671d51c2bbd3debd4fe5d7e92221162`<br>`native/rust_helper/src/broker/audit.rs`=`eb1dcf5018d95c7fdf6171aadd4bca9554d1996f`<br>`native/rust_helper/src/broker/store.rs`=`356b490e403b693db499befb777789784b57d40d`<br>`native/rust_helper/src/mcp.rs`=`0ba59eb42c2b2328c05044dc4d459f6b0e288703`<br>`native/rust_helper/src/a2a.rs`=`9d7c6d9ab938ad5178d6ca46b3933dc3e61c6061`<br>`specs/runtime.schema.json`=`bdef33f26fe06b334eb6f94124930fd08594bbc9`<br>`specs/mcp_connection.schema.json`=`a8d613bd911c9e88f949b984b1a2032eb19db332`<br>`specs/mcp_contract.schema.json`=`733f1ecf2ad968e98c7d0c3557e5ec897a4cac23`<br>`specs/a2a_connection.schema.json`=`1dbbe44e54b302a1196d7e4e55f1cae35cda569b`<br>`specs/a2a_contract.schema.json`=`8c7f50bb7fb12dde41293727e5dc9109445d1bc3`<br>`specs/audit.schema.json`=`01da4c598b52b2ae80ebf46f9e9050d7bec16ca3`。C29補助仕様の8→9件修正はこの作業内の文書訂正で、source実装は変更しない。 |
| 試験区分（`TestClass`） | `SOURCE`／`UNIT`／`LIVE_RUNTIME`／`FIXTURE`／`INTERNAL_STATE`を分離。過去C29の9/9限定PASSは履歴。今回C29 runner・C30・製品Rust/Flutterは`NOT_RUN`。Conformance 272・Schema等の文書バッチ検査は進捗に別記する。 |
| 証拠ID（`Evidence ID`） | `NONE`。現candidate Phase40/C29またはv4 Q2の新Evidenceを発行しない。 |
| 阻害要因（`Blocker`） | 現Release Registry 18件（16 unresolved／2 resolved）と`release_ready=false`を維持。Q2は延期／後段、N11/E0はOPEN。既存blockerをC29専用の新規IDへ複製しない。 |
| 状態 | Phase40のE0-04全21項目と47 SourceSHAを照合し、C29文書の8/9件差を同期した。これはC29実行、Q2、N11、E0、製品AcceptanceのCLOSEDではない。 |
| 残余 | 現candidate C29 runnerと統合Release candidateのQ2は未実行。外部Runtime/MCP/A2A・実ディスクfull・installed processの障害保証は未成立。過去9/9は旧sourceの履歴に限る。追加の障害fixtureや受入条件は現行Phaseへ導入しない。 |

##### Phase 41 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 旧Phase/Cの性能測定を現行v4 S04 N09/N11、S05 K04/K19、S08 Q4へ証拠境界付きで対応する。C27自体は現行v4の製品Acceptance IDではない。 |
| 旧ID | D4 Phase 41「性能評価」／rev1 C27。snapshot・Flutter projection・4096行diffの限定開発測定履歴を保持し、現candidateの性能合格へ転記しない。 |
| 原文参照 | 現行v4仕様S04 N09/N11、S05 K04/K19、S08 Q4、工程R03線C/J・R08 Q4、施工指示I07-04、`docs/FINAL_QA_QUEUE.md` Q4、`docs/D4_POCKET_PHASE_MAPPING.md` Phase41、`docs/specs/performance-validation.md`。`docs/REV2_PROGRESS.md`と`docs/総合機能拡張_rev1/進捗.md`は旧計測条件・結果の履歴資料。 |
| 対象 | `tooling/performance_validation.py`、`tooling/shell_snapshot.py`、`apps/desktop_flutter/test/c27_performance_validation_test.dart`と、その参照projection/client。installed product・実Runtime性能は対象外。 |
| 目的 | 開発中のbounded snapshot／Flutter表示projectionに明白な遅延や停止がないか検知する。C27のwatchdog／局所時間を性能SLA、実画面frame、実Runtime負荷、release証拠へ拡張しない。 |
| 生成元（`Producer`） | `build_shell_snapshot()`の生成状態、Flutter mock snapshot、512件Runtime projection、検索、200件notification fixture、12指標`unknown`のresource fixture、4096行diff widget。旧履歴には2026-09-24 snapshot 870ms／Flutter process 7,087ms、2026-09-25（旧source commit `8202ac796eba0909c9d5b83cf71e89d8fc97fc5a`）snapshot 1,016ms／Flutter process 20,245ms等が記録される。 |
| 実Consumer | `tooling/validate_all.py`はFlutter利用可能時の非`--python-only`構成で`c27_performance_validation`を追加する。stepは`in_release_scope=False`で製品release blockerにならない。Python runnerはgenerated snapshotを計測後、Flutter test processを同期起動しstdout JSONと`PERF_C27`行を出す。`full_regression_validation.py`とConformance skeletonはC27 runnerを起動しない。 |
| 契約／Data | Snapshotの値はreference stateから生成された`INTERNAL_STATE`、Flutter transport／snapshot／diffは`FIXTURE`。Python結果はelapsed／status／evidence classを標準出力へ出すが、永続Evidence Schema・N11への自動固定はない。Flutter内の各計測値はdebug出力。12 resource metricは値`null`、状態`unknown`を保持する。 |
| 正常時作用 | snapshot生成が5秒未満、Flutter testが正常終了し、テスト内watchdog／bounded projection assertionを満たす場合に限り開発smokeがpassedとなる。test sourceにはmodel初期化、snapshot読込、projection/search、fixture notification/resource、lazy diffの2試験がある。 |
| 否定時作用 | assertion失敗またはFlutter非zero exitを失敗として返す。async操作・widget pumpの一部には5秒Future timeoutがあるが、同期計算は完了後の計測判定であり中断されない。外側のFlutter `subprocess.run`および`validate_all.py`のstep実行にprocess timeoutがなく、停止したchildを確実に終了する契約はない。 |
| 停止／Recovery | C27 runnerに独立したcancel／deadline／child process kill／Recovery／cleanup保証はない。resource値は取得不能時に`unknown`を維持する。process停止時の堅牢な期限・回収は将来C27を実行する必要がある場合のharness課題であり、このPhase41 mappingの新blockerとはしない。 |
| 標準OS候補 | v4のWindows／macOS／Linux native／Android／iOS母集団を保持。今回のsource mappingはWindows上のinstalled productや他OS標準環境の性能試験を行わず、OS別Q4結果を生成しない。 |
| 配布物 | 開発用Python validator、Flutter test process、mock／generated dataのみ。一般installer、installed Release candidate、署名済artifact、実Agent／Runtime／MCP／A2A loadは使用しない。 |
| 既存実装 | C27 validator／spec／Flutter testと`validate_all.py`接続がある。ConformanceおよびC30 regressionとは別step。旧C27の過去計測は`docs/総合機能拡張_rev1/進捗.md`／`docs/REV2_PROGRESS.md`に条件付きで記録され、現mainでの再測定ではない。 |
| BUILD_GAP | C27開発smokeのsource接続を確認。v4全体候補に対する実installed性能／資源／長時間Q4 consumerとQA証拠は未成立だが、Q4は全体候補後の後段QAであり、本E0 mappingへ新しい製品Acceptance／Blockerを追加しない。 |
| ソース識別値（`SourceSHA`） | 照合HEAD=`157ea8dfd1dbec7f55fedee6b4dd216886419025`。編集前baselineの32 unique tracked pathを個別照合し、全blob SHA一致。`AGENTS.md`=`1eafcadd129fd01f1f2f683594c381605ed7d48d`<br>`docs/QE_2026-11-13/01_D4Pocket_統合製品実装仕様書_v4.md`=`352c53180cbad07888beb956aad0fb2a58074c58`<br>`docs/QE_2026-11-13/02_D4Pocket_統合開発工程表_v4.md`=`d32dd251e5f7cbb97d92f6e7540d6b6ed0ecec05`<br>`docs/QE_2026-11-13/03_D4Pocket_Codex施工指示書_v4.md`=`49447973e88c50a09b2f4978adfed84fd464c6d0`<br>`docs/QE_2026-11-13/採用記録.md`=`f159c1aa6ea5bc6e80e0877d8068f7e44da4c4d3`<br>`docs/D4_POCKET_PHASE_MAPPING.md`=`064ea334f86b137d94fba2370d2a2fd732f6e972`<br>`docs/QE_2026-11-13/E0_要求consumer台帳.md`=`a97446c84456ab089b5b28ac772adbb0af989921`<br>`docs/QE_2026-11-13/進捗.md`=`d3f7b141a1ed33f393271b9fdc93728467c6f153`<br>`docs/FINAL_QA_QUEUE.md`=`7b332cccab70bb68aa1ff4f8e23acc0e816d8282`<br>`docs/specs/performance-validation.md`=`6746542d94a1f417d443f1a3992779e6480e74ed`<br>`docs/REV2_PROGRESS.md`=`56f0f168b22d1e64303db2bc6f2c66668393dba2`<br>`docs/総合機能拡張_rev1/進捗.md`=`1392588f0cd5273013578e84d8e33c5c9dc9a01a`<br>`release_blockers.registry.json`=`244adc04a01e7b406c4ebcdeb829f7077d4aa35f`<br>`tooling/performance_validation.py`=`d0744916a8be65e10b61a453864bb8b80318241e`<br>`tooling/shell_snapshot.py`=`34823638523ae93fa750e52aeb51a078169932c1`<br>`installer/setup_doctor.py`=`23b43a8a00814e4033addec71efa11126d3b916a`<br>`packages/shell_core/invariant_evaluator.py`=`960c5ee99109796512dd720ec27e132f87dcfaaf`<br>`packages/shell_core/release_smoke.py`=`f5adf1eba0401d48d70bfa2acfc29774e4e139f6`<br>`tooling/validate_all.py`=`2744f9a10702872f6dc0aad705f922466fc198fc`<br>`tooling/full_regression_validation.py`=`8a33a9fab9ecbc18119302ae312201d41618012e`<br>`tooling/conformance_tests/run_conformance_skeleton.py`=`c6f31d8cb561b48513c6784e54c85969c9cb953e`<br>`apps/desktop_flutter/test/c27_performance_validation_test.dart`=`e833fbd760a8d2e6c4c90e4c6bee6a8c3cb7cff1`<br>`apps/desktop_flutter/lib/models/generated_contracts.dart`=`64e7a80a308c2734eb18c9d4b26eb50c8354f67b`<br>`apps/desktop_flutter/lib/screens/workspace_diff_panel.dart`=`5e2154b870921a707b3da9a79c9f55055ce3cb1f`<br>`apps/desktop_flutter/lib/services/global_search_index.dart`=`f71d2f6c4563b011fa3621ea3aee596bffb099f0`<br>`apps/desktop_flutter/lib/services/notification_client.dart`=`746c74805ef5b80febf020306daeb360ff43b4e6`<br>`apps/desktop_flutter/lib/services/shell_core_client.dart`=`f08db061bd15649ff7c2fc3fc9e37184433cbf18`<br>`packages/gui_shell_ui/lib/runtime_resource_client.dart`=`2df13807ceed7f3305e13ca752cd31f824583543`<br>`tooling/manifest.py`=`aa4525ff20293cb8641e5598a2439561a3bfbd3b`<br>`tooling/release_gate_check.py`=`22a8d7bfe9adb259f44ce8501fb215652e1d58c6`<br>`tooling/schema_check/check_schemas.py`=`7dca27717cbe5cccb45e1013b3436918a2322e75`<br>`tooling/日本語基底監査.py`=`77474767dd93eb4d69382af5ea4a0ea384eeccbe`。 |
| 試験区分（`TestClass`） | SOURCE／`INTERNAL_STATE`／`FIXTURE`。本単位はE0のsource mappingと文書整合のみ。C27 runner、Flutter/Rust product test、installed product、Q4は`NOT_RUN`。旧C27 PASSは旧source／旧環境の履歴。Conformance等の文書batch validationは進捗に別記する。 |
| 証拠ID（`Evidence ID`） | `NONE`。current v4 C27/Q4の性能Evidenceは発行しない。 |
| 阻害要因（`Blocker`） | Release Registry 18件（16 unresolved／2 resolved）、`release_ready=false`、N11/E0 OPEN、Q0〜Q7 `DEFERRED`を維持。harnessのprocess timeout欠如を新Blockerへ複製しない。 |
| 状態 | Phase41のE0-04全21項目と32 SourceSHAを現mainへ結合。C27の限定開発consumer／timeout範囲を明確化した。これは現行Q4、N09/N11、E0、製品AcceptanceのCLOSEDではない。 |
| 残余 | Q4で同一候補の8GB級Windows代表・対象OS別resource・起動・CPU/memory/process leak・module pruning・反復Task/Compare/Handoffを後段で測定し、N11 Evidenceへ結合する。C27を将来実行する場合は、実際に使うharnessのprocess-level deadline／停止・結果保存の保証を確認する。性能SLAは実測前に定義・主張しない。 |

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

#### Phase 42 E0-04個別記録（2026-11-14）

##### Phase 42 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 旧D4 Phase 42「全数回帰」。現行v4ではN11／K19へ接続するE0 source→consumer照合IDであり、C30/C32や製品Acceptanceを新規にCLOSEDへ移すIDではない。 |
| 旧ID | `docs/D4_POCKET_PHASE_MAPPING.md`のPhase 42はrev1 C30「全数回帰」とC32「最終開発監査」に対応する。C30/C32各記録の旧status・旧証拠・失敗履歴はそのまま保持する。 |
| 原文参照 | 現行v4採用記録・仕様S04 N11/S05 K19・工程E0-04/R08・施工指示I03/I05、Phase mapping Phase 42、`docs/specs/full-regression-validation.md`、`docs/specs/final-development-audit.md`／`.json`。旧rev1のC30/C32と受領記録・進捗は当時statusの参照に限定する。 |
| 対象 | C30 `tooling/full_regression_validation.py`の6 group／7 command、呼出登録、command別evidence/scope、C32 `tooling/final_development_audit.py`の静的table validator、入力3資料、`validate_all.py`登録、およびC32 unit-test sourceの配線有無。 |
| 目的 | 現HEADのlocal validation producer→consumer、実行範囲、報告の永続性、制約をN11/K19のE0全数台帳へ正確に結ぶ。source mappingを実行結果・installed製品QA・全OS合格へ読み替えない。 |
| 生成元（`Producer`） | C30は既存Rust Broker test、Evidence/runtime assertion、localhost Runtime fixture、Codex CLI interface probe、共有Flutter／Mobile Flutter fixture testを一覧化し、6 groupのstatus・evidence class・scopeを集約する。C32は機械可読のC0〜C31対応表、旧受領記録、旧進捗の行番号集合を読む。両reportはJSONを標準出力へ出し、runner自身は結果artifactを永続保存しない。 |
| 実Consumer | C30の明示的な実行consumerは単体CLIと`tooling/validate_all.py`の非`--python-only` full構成にある`c30_full_regression_validation` step。`in_release_scope=False`であり、Conformance skeletonからC30 runnerへの呼出しはない。C32 checkerは`validate_all.py`共通stepとしてPython-only/full両構成に登録され、`in_release_scope=False`。C32 unittest sourceはdocsに直接実行例があるが、`validate_all.py`／Conformance skeletonの実行登録は見つからない。 |
| 契約／Data | C30の7本は、Rust全テスト（制限時間300秒）、証拠一式と実行時確認（各120秒）、30秒指定のlocalhost Runtime試験用固定データとBrokerプロセス間通信（上限100秒）、Codex CLIの版情報・ヘルプ確認（20秒）、Flutter共有部品の比較表示試験（240秒）、Mobile Device Link表示の試験用固定データ（240秒）。子プロセスの標準出力・標準エラーは報告に保存せず、CLI確認が成功した場合も定めた識別情報だけ返す。C32は必須10項目、Repository内相対参照、C0〜C31の過不足・重複、risk分類、受領記録・進捗表C0〜C34、未監査C32〜C34を検査し、`release_ready=false`を出力する。 |
| 正常時作用 | C30は全commandが実行され全6 groupが`passed`のときだけlocal matrix `passed`／exit 0。tool不在を`unavailable`とし、failed/timeoutをPASSに丸めない。C32はtable/reference構造が成立する場合に限り構造監査`passed`／exit 0を出す。C32の終了値0は対応表の構造・参照成立だけを意味する。 |
| 否定時作用 | C30のcommand失敗／timeoutはmatrix failed、未利用toolを含み失敗がない場合はpartialとなり、いずれもexit 1。失敗reportはstdoutに表示されるがファイル保存されない。C32は入力JSON不正、field欠落、path不正/Repository外/symlink越境/参照不在、工程ID不一致/重複、risk分類不正、受領/進捗行欠落、未監査list差異をerror JSON／exit 1とする。 |
| 停止／Recovery | C30は各`subprocess.run(timeout=...)`でcommandをboundedに待つが、子孫process群の停止確認・task cancellation・Recovery再開・output artifact永続化のcontractはない。timeout後のprocess-tree cleanup保証をsourceから主張しない。C32はread-only入力検査で、Runtime/Agent/Permission/Approvalを起動しない。中断後の再実行は手動CLI動作であり、Recovery証拠ではない。 |
| 標準OS候補 | v4要求のWindows／macOS／Linux native／Android／iOSを母集団として保持する。C30のRust/UI/Mobile実行はローカルhost上のtest consumerであり、OS別installed candidate、外部Agent/Runtime、実端末、TLS/native secure storeを網羅しない。C32のPython静的検査も各OS製品の受入証拠ではない。Linux nativeは必須製品対象のまま。 |
| 配布物 | 開発checkout上のPython validator、Rust test対象、shared Flutter/Mobile Flutter test対象。Installer、installed app、release artifact、OS別配布物、外部署名artifactを生成・起動しない。C30/C32のstdout JSONは保存Evidence fileではない。 |
| 既存実装 | C30は6 group/7 commandとdocumented local entryを持ち、`validate_all.py` full構成へ接続済み。C32 checkerは`validate_all.py`共通stepへ接続済み。C32 unit test sourceとmanual commandは存在するが共通検証経路に未登録。Conformance skeletonは両runnerを直接呼ばない。旧C30の「開発範囲で成立」等のstatusは旧tableの履歴値として不変。 |
| BUILD_GAP | 今回は構造・呼出しsourceのE0照合のみ。C30 runner、C32 checker/unit test、Rust/Flutter suitesはいずれも`NOT_RUN`。C30 matrix reportのdurable Evidence/N11結合、installed製品・全OS・外部Agent/Runtime・実端末の作用証拠はこのsource mappingから成立しない。C32のC0〜C31静的表はC32自身を監査対象にせず、C32〜C34を明示的に未監査とする。後段の同一候補QC/QAとN11 Evidence結合へ残し、今回の新規acceptance/blockerにはしない。 |
| ソース識別値（`SourceSHA`） | 照合HEAD=`4aea4bb47b9bc9e283cb60e39566003d0ce5ea2f`。編集前baseline上の29 unique tracked path/blobを個別固定した。E0台帳・進捗の値は編集前blobであり、この記録を作成する前のConsumer source状態を指す。<br>`AGENTS.md`=`1eafcadd129fd01f1f2f683594c381605ed7d48d`<br>`docs/QE_2026-11-13/01_D4Pocket_統合製品実装仕様書_v4.md`=`352c53180cbad07888beb956aad0fb2a58074c58`<br>`docs/QE_2026-11-13/02_D4Pocket_統合開発工程表_v4.md`=`d32dd251e5f7cbb97d92f6e7540d6b6ed0ecec05`<br>`docs/QE_2026-11-13/03_D4Pocket_Codex施工指示書_v4.md`=`49447973e88c50a09b2f4978adfed84fd464c6d0`<br>`docs/QE_2026-11-13/採用記録.md`=`f159c1aa6ea5bc6e80e0877d8068f7e44da4c4d3`<br>`docs/D4_POCKET_PHASE_MAPPING.md`=`064ea334f86b137d94fba2370d2a2fd732f6e972`<br>`docs/QE_2026-11-13/E0_要求consumer台帳.md`=`b520146d6fb53c0bf7b5e1d763463bb93ce9ae77`<br>`docs/QE_2026-11-13/進捗.md`=`0c3637d127fa289635884975bd68c42964dcde90`<br>`docs/FINAL_QA_QUEUE.md`=`7b332cccab70bb68aa1ff4f8e23acc0e816d8282`<br>`release_blockers.registry.json`=`244adc04a01e7b406c4ebcdeb829f7077d4aa35f`<br>`docs/specs/full-regression-validation.md`=`dd5953d3bea595c2f25474ed0a9c42cf01ff51e3`<br>`tooling/full_regression_validation.py`=`8a33a9fab9ecbc18119302ae312201d41618012e`<br>`tooling/validate_all.py`=`2744f9a10702872f6dc0aad705f922466fc198fc`<br>`tooling/conformance_tests/run_conformance_skeleton.py`=`c6f31d8cb561b48513c6784e54c85969c9cb953e`<br>`tooling/evidence_bundle.py`=`68dead851236b0f1b69899151c37371e593ff3f9`<br>`tooling/release_runtime_assertions.py`=`0a0a351fe17151ad86517bd0a965358f40c61490`<br>`tooling/long_run_validation.py`=`00d1ec4fa0a7b7fea4ef678a94a56f20367a243c`<br>`tooling/agent_adapter_probe.py`=`8f3ca24e1f4a61139c808914390d85f6631da5a6`<br>`packages/gui_shell_ui/test/evaluation_client_test.dart`=`044d71b4b18b7efa8891a7df735f28e6ed674ac1`<br>`apps/mobile_flutter/test/device_link_test.dart`=`509b4febc424d1c0fd330b71529b00457b5cf429`<br>`native/rust_helper/Cargo.toml`=`6352fd7de3188b4c7335167f4f8875a4f8f4e28b`<br>`native/rust_helper/Cargo.lock`=`8647830893561de17e82063231b651322c8909bd`<br>`docs/specs/final-development-audit.md`=`cc3734d7647dbbbbd27bad49153f775bfc980f3f`<br>`docs/specs/final-development-audit.json`=`8126944c55339a3be272af5ba183a9bc2e3517dc`<br>`tooling/final_development_audit.py`=`913c81bc22f71bc5bb3447fa4f4bd266326932ec`<br>`tooling/conformance_tests/test_final_development_audit.py`=`628c77a50adf867e87b4e4a893d5e691df8111d1`<br>`docs/総合機能拡張_rev1/受領記録.json`=`e8433d4b4769200c00f1aa0c6e1b4c7671229352`<br>`docs/総合機能拡張_rev1/進捗.md`=`1392588f0cd5273013578e84d8e33c5c9dc9a01a`<br>`tooling/release_gate_check.py`=`22a8d7bfe9adb259f44ce8501fb215652e1d58c6`<br>|
| 試験区分（`TestClass`） | 今回の判定は`SOURCE`／`CONFIG`の読み取り分類。C30はsource上`LIVE_RUNTIME`／`INTERNAL_STATE`／`FIXTURE`を含むlocal test matrix、C32は`CONFIG`の静的table audit。これらは実行結果ではない。C30/C32 runner・C32 unittest・製品Rust/Flutter・installed product・OS QAはいずれも`NOT_RUN`。 |
| 証拠ID（`Evidence ID`） | `NONE`。今回のE0 source mappingからC30/C32の実行Evidenceを発行せず、旧C30/C32 status・旧測定値を現在のEvidenceへ転記しない。 |
| 阻害要因（`Blocker`） | 既存Release Registry 18件（16 unresolved／2 resolved）、`release_ready=false`、N11/E0 OPEN、Q0〜Q7 `DEFERRED`を維持する。今回新しいrelease blocker／acceptanceを追加しない。C30のprocess-tree cleanup未規定を今回のE0 blockerへ昇格しない。 |
| 状態 | Phase 42のE0-04全21項目を編集前main `4aea4bb47b9bc9e283cb60e39566003d0ce5ea2f`に対するSourceSHAと結合し、C30/C32の実Consumerと証拠上限を記録した。これはC30回帰実行、C32監査実行、C32〜C34、N11、E0、製品AcceptanceのCLOSEDではない。 |
| 残余 | E0ではPhase43〜45等の未記録fieldを継続する。C30/C32結果の同一candidate・N11 evidence chain結合、C33 Windows installed evidence、C34 release前条件、全OS標準環境QAはそれぞれ現行後段工程に残る。新しい試験要求は追加しない。 |

#### Phase 43 E0-04個別記録（2026-11-14）

##### Phase 43 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 旧D4 Phase 43「文書・ブランド統合」。現行v4ではN11／K19へ接続するE0 source→consumer照合IDであり、C31の旧statusや製品Acceptanceを変更するIDではない。 |
| 旧ID | `docs/D4_POCKET_PHASE_MAPPING.md`ではPhase 43はC31に対応し、C31はPhase 1にも対応する。既存C31の状態、限定証拠、失敗履歴をその時点の記録として保持する。 |
| 原文参照 | v4採用記録、仕様S00/S10/N11/K19、工程E0-04、施工指示のE0・文書同期規則、Phase mapping Phase 43、rev1 C31記録。旧版は当時の履歴参照に限る。 |
| 対象 | 現行製品文書の版・保証・状態のconsumer、および文書内容／識別値を検査するManifest、Conformance、Release Gate、日本語基底監査の接続範囲。 |
| 目的 | 製品説明を現行v4要求・確認済みsource・証拠・Release状態へ結び、履歴文書を現行実証と誤認させない。文書存在や検査成功を製品runtime／QA成立へ昇格しない。 |
| 生成元（`Producer`） | 採用hash付きv4三点、現在の進捗・要求台帳、実装source、Release Registry、過去のC31および製品検証記録。 |
| 実Consumer | 利用者向けREADME／QUICKSTART／ROADMAP／CLAIM／COMPATIBILITY_MATRIX／RELEASE_CHECKLIST／SECURITY、操作面・設定・Mobile状態文書、FINAL_QA_QUEUE、Manifest／日本語監査／Schema／Conformance／通常Release Gate。`docs/REV2_PROGRESS.md`とrev1進捗は履歴consumerとして区別する。 |
| 契約／Data | 文書版・採用hash、機能／OS範囲、Source／Artifact identity、evidence class、OPEN／FAIL／NOT_RUN、Release blocker、claim境界。文言・metadata・履歴はAuthorityやPermissionを生成しない。 |
| 正常時作用 | 現行文書のv4導入部は五OSを必須対象とし、候補installed QA未成立、正式Release未宣言、`release_ready=false`等の状態を表示する。履歴部分は日付・対象・限定範囲を保持する。 |
| 否定時作用 | 古いclaimを現行状態として示す、根拠のないOS保証を表示する、または文書検査結果をruntime保証へ読み替える場合は不整合として扱い、成功・Release readinessを主張しない。 |
| 停止／Recovery | 現行正本と文書記述の意味上の不一致は該当claimをUNKNOWNとして保留し、現行正本側で原因を解消する。履歴の改変や文書修正だけによるAcceptance／Blockerの昇格は行わない。 |
| 標準OS候補 | Windows／macOS／Linux Desktop、Android／iOS Mobileの五OSを文書母集団に含める。標準環境へのinstalled製品適合はOS別の後続証拠で判定する。物理非Windows端末の不在を施工条件にしない。 |
| 配布物 | README、Quickstart、Release説明などの文書。実Installer、署名済artifact、installed製品をこのmappingで生成・検証しない。 |
| 既存実装 | README／ROADMAP／MOBILE_STATUS／COMPATIBILITY_MATRIX／CLAIM／RELEASE_CHECKLISTには現行v4状態の節があり、QUICKSTARTは現行進捗を参照する。GUI_OPERATION_SURFACES／SECURITY／CONFIGには2026-09-24基準のC31記述が残る。REV2_PROGRESSは履歴資料である。 |
| BUILD_GAP | Manifestはhash整合、Conformance／Schema／Release Gateは各々の構造・契約・登録条件、日本語監査は散文の静的ヒューリスティックを検査するが、これらは全公開文書の意味的一貫性、候補artifact／installed UXとの同一性、五OS製品挙動を証明しない。全claimと同一Release候補を結ぶ監査およびN11のEvidence／Blocker／OS Artifact全数結合は未完。 |
| ソース識別値（`SourceSHA`） | 照合HEAD=`b66c74966ec338f1ad681bf3b5bdeb6cb3d7ae36`。編集前mainの32 unique tracked path/blobを個別固定した。<br>`AGENTS.md`=`1eafcadd129fd01f1f2f683594c381605ed7d48d`<br>`規定/00_日本語基底規定.md`=`f3a050806228d2f50b22c95bcc30259d7bc0733f`<br>`規定/正本索引.json`=`ba250de80a3ec94109171c09adf54cbe227add10`<br>`docs/QE_2026-11-13/01_D4Pocket_統合製品実装仕様書_v4.md`=`352c53180cbad07888beb956aad0fb2a58074c58`<br>`docs/QE_2026-11-13/02_D4Pocket_統合開発工程表_v4.md`=`d32dd251e5f7cbb97d92f6e7540d6b6ed0ecec05`<br>`docs/QE_2026-11-13/03_D4Pocket_Codex施工指示書_v4.md`=`49447973e88c50a09b2f4978adfed84fd464c6d0`<br>`docs/QE_2026-11-13/採用記録.md`=`f159c1aa6ea5bc6e80e0877d8068f7e44da4c4d3`<br>`docs/D4_POCKET_PHASE_MAPPING.md`=`064ea334f86b137d94fba2370d2a2fd732f6e972`<br>`README.md`=`38664991fe614acfc516eb6743dde6dce754191c`<br>`ROADMAP.md`=`6a922a38cca617690bc08297274e907f0425b722`<br>`docs/GUI_OPERATION_SURFACES.md`=`4b48d55a69866dd0e1b3180e2192fc9efcc90598`<br>`SECURITY.md`=`6784fb68043aaceccc6d063b253c5d7d50210065`<br>`CONFIG.md`=`7349ca355614fa265a80c104f2be3b3503a92590`<br>`MOBILE_STATUS.md`=`476b565a3f583be52ef58966f67c51abbb774aa8`<br>`COMPATIBILITY_MATRIX.md`=`c4885d5b67202712c0bcdecc6da634cb778cd6d3`<br>`CLAIM.md`=`0bfbf988469e0f34c9e7f96f805ce26faf3b1044`<br>`RELEASE_CHECKLIST.md`=`35f31043a9311d49c94acdc9631e30b61949c65d`<br>`QUICKSTART.md`=`9653c4276f04392eead13736ba6b36ee2031bdb5`<br>`docs/REV2_PROGRESS.md`=`56f0f168b22d1e64303db2bc6f2c66668393dba2`<br>`docs/FINAL_QA_QUEUE.md`=`7b332cccab70bb68aa1ff4f8e23acc0e816d8282`<br>`docs/総合機能拡張_rev1/進捗.md`=`1392588f0cd5273013578e84d8e33c5c9dc9a01a`<br>`docs/specs/final-development-audit.json`=`8126944c55339a3be272af5ba183a9bc2e3517dc`<br>`docs/QE_2026-11-13/進捗.md`=`5bace45844d87c34a4be584221a50917a1da0c58`<br>`docs/QE_2026-11-13/E0_要求consumer台帳.md`=`a00c65554ee8b0e4018319b089e92cc10911f745`<br>`release_blockers.registry.json`=`244adc04a01e7b406c4ebcdeb829f7077d4aa35f`<br>`MANIFEST.sha256.json`=`db5b8ef4ca4ccc774ac07b064268bb61d7cd01b5`<br>`tooling/日本語基底監査.py`=`77474767dd93eb4d69382af5ea4a0ea384eeccbe`<br>`tooling/schema_check/check_schemas.py`=`7dca27717cbe5cccb45e1013b3436918a2322e75`<br>`tooling/conformance_tests/run_conformance_skeleton.py`=`c6f31d8cb561b48513c6784e54c85969c9cb953e`<br>`tooling/manifest.py`=`aa4525ff20293cb8641e5598a2439561a3bfbd3b`<br>`tooling/validate_all.py`=`2744f9a10702872f6dc0aad705f922466fc198fc`<br>`tooling/release_gate_check.py`=`22a8d7bfe9adb259f44ce8501fb215652e1d58c6` |
| 試験区分（`TestClass`） | `SOURCE`／`DOC`／`CONFIG`のread-only照合。後続の文書構造検査は文書・Manifest・Schema・Conformance・通常Release Gateの範囲に限定する。Strict Release、C31旧試験、製品Rust／Flutter、installed product、五OS QAは本mappingで実行しない。 |
| 証拠ID（`Evidence ID`） | `NONE`。SourceSHAと既存文書／validator接続の照合のみで、製品QA／公開承認のEvidenceを発行しない。 |
| 阻害要因（`Blocker`） | 既存Registry 18件（16 unresolved／2 resolved）、`release_ready=false`、正式Q0〜Q7 `DEFERRED`、N11／E0 `OPEN`を維持する。新規Release blocker／Acceptanceは追加しない。 |
| 状態 | Phase 43のE0-04全21項目を編集前main `b66c74966ec338f1ad681bf3b5bdeb6cb3d7ae36`に対するSourceSHAへ結合し、文書・validator consumerと保証限界を記録した。C31の過去status／FAIL、五OS製品Acceptance、N11、E0は変更していない。 |
| 残余 | Phase44〜45の個別field、N11 Evidence／Blocker／OS Artifactの全数結合、Release候補と文書claim・installed製品の同一性確認を後続へ残す。歴史的C31文書の全面書換えや製品試験は本mappingの対象外。 |

#### Phase 44 E0-04個別記録（2026-11-14）

##### Phase 44 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 旧D4 Phase 44「Windows最大到達」。現行v4ではN10/N11、施工線G/J、K16/K19に接続するsource→consumer照合ID。Phase単位の記録であり、Windows製品AcceptanceやN10全体のCLOSEDを意味しない。 |
| 旧ID | `docs/D4_POCKET_PHASE_MAPPING.md`のPhase 44は旧C32最終開発監査・C33 Windows最大到達へ対応する。C32の静的対応表consumerはPhase42で既に記録済み。C33/Phase34の旧installed証拠と総合FAILを履歴として保持する。 |
| 原文参照 | 採用済みv4仕様S02/S04 N10・N11/S05 K16・K19、工程R03 G/J・R05、施工指示I04-G/I04-J・I07-02・I07-07、Phase mapping Phase34/42/44、旧rev1 C32/C33進捗、現行Windows release evidence文書。Phase42のC32対応付けやPhase34の配布consumerを再実行しない。 |
| 対象 | Windows staged／installed candidateのsource・artifact・profile・runtime・config・Audit identityを収集し、installed collectorからWindows evidence validator、Release Gate／Registryへ渡す証拠経路。通常配布InstallerやN10全OS受入れの全体実装ではない。 |
| 目的 | 開発build／portable・過去runの部分PASSを現行installed candidateの総合証拠と混同せず、同一Windows candidateから取得した証拠だけを対応するRegistry gateへ接続する。 |
| 生成元（`Producer`） | source commitに結び付くWindows app／Launcher／Broker artifact、`stage_installed_app.ps1`、別profileから実起動する`collect_installed_smoke.ps1`、`collect_broker_smoke.ps1`、製品Store／Auditと監査アンカーcollector。 |
| 実Consumer | staged app／Launcher／Broker→`collect_installed_smoke.ps1`／Broker・Audit collector→`tooling/windows_release_evidence.py`の個別evidence result→`tooling/release_gate_check.py`のRegistry同期。`release_smoke.py`と`evidence_bundle.py`は開発／fixture側の別consumerであり、installed product evidenceの代替ではない。 |
| 契約／Data | installed evidence Schema、source commit・clean worktree、app／Launcher／Broker SHA-256、isolated install/run rootとprofile digest、runtime／config／Audit identity、各evidence file hash、`field_provenance`・evidence class、五つのWindows blocker result。raw SID／Credentialを公開せず、現candidateの識別に必要な値だけを結合する。 |
| 正常時作用 | 全必須fileとprovenanceが同一candidateに一致し、installed app・Broker・Setup Doctor・Audit anchorの各validator条件が成立したときに限り個別resultを`passed`へできる。部分subgateの成功を総合製品PASSへ合成しない。 |
| 否定時作用 | Evidence欠落、古いsource／artifact、dirty source、profile/root不一致、hash・schema・field provenance不一致、Setup Doctor／Broker／Audit証拠不成立を個別失敗として返し、対応blockerを有効のままにする。旧source commitのproof copyは現candidateへ再結合しない。 |
| 停止／Recovery | collector／validatorで欠測・不一致・証拠改変を検出したら該当evidence採用とRelease promotionを停止し、`release_blocker`を維持する。collector failureをPASSへ正規化せず、既存evidenceの上書き・再利用でRecovery扱いしない。 |
| 標準OS候補 | Windows native標準環境／隔離profileと、仕様で定める指示者所有Windows PCの最終物理確認。Phase44 mappingではどちらも実行しない。Windows結果を他OS証拠へ流用しない。 |
| 配布物 | 現sourceにstaged／portable ZIP／D4PKG01等の候補はあるが、正式な一般利用者向けWindows `.exe` Installer・Publisher identity・署名済配布物は未成立。Developer staging artifactを配布完了へ昇格しない。 |
| 既存実装 | Rust Desktop Launcher／Broker、Windows named-pipe relay、stager、installed／Broker／Setup Doctor／Audit anchor collector、Windows evidence validator、通常Release Gateのsourceが存在する。現`release_evidence/`には`.gitkeep`だけで、current candidateのinstalled evidence bundleはない。collector v17のstrict別profile/UIA経路はsourceにあるがfresh installed runでは未実証。 |
| BUILD_GAP | 正確なcurrent sourceからのclean installed bundleと別profile実行、通常Installer→first run→実Task→Update／Repair／Rollback／Uninstall同一candidate経路、Setup Doctor UI／reportのfresh evidence、installed Broker evidence、外部Audit anchor proof、仕様範囲のWindows PC確認が未成立。N10全OS配布・導入も別範囲でOPEN。 |
| ソース識別値（`SourceSHA`） | 照合HEAD=`0ea5fb8c829deb0436118965f2a5f10060cfad71`。以下42 unique tracked pathのGit blobを編集前mainで個別固定：<br>`AGENTS.md`=`1eafcadd129fd01f1f2f683594c381605ed7d48d`<br>`規定/00_日本語基底規定.md`=`f3a050806228d2f50b22c95bcc30259d7bc0733f`<br>`規定/正本索引.json`=`ba250de80a3ec94109171c09adf54cbe227add10`<br>`docs/QE_2026-11-13/01_D4Pocket_統合製品実装仕様書_v4.md`=`352c53180cbad07888beb956aad0fb2a58074c58`<br>`docs/QE_2026-11-13/02_D4Pocket_統合開発工程表_v4.md`=`d32dd251e5f7cbb97d92f6e7540d6b6ed0ecec05`<br>`docs/QE_2026-11-13/03_D4Pocket_Codex施工指示書_v4.md`=`49447973e88c50a09b2f4978adfed84fd464c6d0`<br>`docs/QE_2026-11-13/採用記録.md`=`f159c1aa6ea5bc6e80e0877d8068f7e44da4c4d3`<br>`docs/QE_2026-11-13/進捗.md`=`7ae2f175ce3f5e72250bc0aa827341a43246dc54`<br>`docs/QE_2026-11-13/E0_要求consumer台帳.md`=`5d71ce4f75c2dd18412d0378792117ac7c32d9f3`<br>`docs/D4_POCKET_PHASE_MAPPING.md`=`064ea334f86b137d94fba2370d2a2fd732f6e972`<br>`docs/REV2_PROGRESS.md`=`56f0f168b22d1e64303db2bc6f2c66668393dba2`<br>`docs/REV5_PRODUCT_PROGRESS.md`=`9c978021d3982690e9aaa87896436bad7125b433`<br>`docs/総合機能拡張_rev1/進捗.md`=`1392588f0cd5273013578e84d8e33c5c9dc9a01a`<br>`docs/specs/final-development-audit.md`=`cc3734d7647dbbbbd27bad49153f775bfc980f3f`<br>`docs/specs/final-development-audit.json`=`8126944c55339a3be272af5ba183a9bc2e3517dc`<br>`docs/WINDOWS_RELEASE_PLAN.md`=`5d4c35f8c9599c9f38b7c02ae475710e8c5a64e8`<br>`docs/WINDOWS_RELEASE_EVIDENCE.md`=`9c836b028c352da8d37f9feae53dcb27b537199d`<br>`docs/FINAL_QA_QUEUE.md`=`7b332cccab70bb68aa1ff4f8e23acc0e816d8282`<br>`docs/specs/windows-product-package.md`=`19c5c45dc8cbdbd4560b7b2cb8fe5d8e9f82a2c3`<br>`docs/specs/windows-desktop-launcher.md`=`609a0c724d5bb77ca15c7721e760267ac3c4d018`<br>`docs/specs/desktop-broker-channel.md`=`f9178f356061e60d170f29e57a3cec0f7f228867`<br>`docs/specs/update-center.md`=`be37e1f661cb3e4cb05e7c1242051263665a92b2`<br>`docs/SETUP_DOCTOR.md`=`f482c934c8bae002a4517fa8a3a7848fb5067fff`<br>`installer/windows/README.md`=`46287d463b38b1093a29e9818c0f0bfd99465393`<br>`installer/windows/stage_installed_app.ps1`=`c6d0797299624d69b71042416400b2f454787cc7`<br>`installer/windows/collect_installed_smoke.ps1`=`92c4720e82eb0af91002659c742f8e03b15aa2de`<br>`installer/windows/collect_broker_smoke.ps1`=`f5f8abfad981d64c30a6cf1f4e9eafde1b43c357`<br>`installer/windows/collect_setup_doctor.ps1`=`3695f3402dd175f8eafead3baa2c82bcc45e6364`<br>`installer/windows/collect_audit_anchor_proof.ps1`=`309149d2c069bf6f4e2cf3638a932b84a6b60fe0`<br>`tooling/windows_release_evidence.py`=`6f7957c6193c86c7bc15165e743f11651464f29c`<br>`tooling/release_smoke.py`=`e9f38cea998a31bbf9272880dbf4965de5eb0113`<br>`tooling/evidence_bundle.py`=`68dead851236b0f1b69899151c37371e593ff3f9`<br>`tooling/audit_checkpoint_verifier.py`=`71d6ada188163bb56f73da5f5be85566bc973bf5`<br>`tooling/release_gate_check.py`=`22a8d7bfe9adb259f44ce8501fb215652e1d58c6`<br>`tooling/release_runtime_assertions.py`=`0a0a351fe17151ad86517bd0a965358f40c61490`<br>`tooling/final_development_audit.py`=`913c81bc22f71bc5bb3447fa4f4bd266326932ec`<br>`tooling/conformance_tests/test_final_development_audit.py`=`628c77a50adf867e87b4e4a893d5e691df8111d1`<br>`release_blockers.registry.json`=`244adc04a01e7b406c4ebcdeb829f7077d4aa35f`<br>`CLAIM.md`=`0bfbf988469e0f34c9e7f96f805ce26faf3b1044`<br>`RELEASE_CHECKLIST.md`=`35f31043a9311d49c94acdc9631e30b61949c65d`<br>`COMPATIBILITY_MATRIX.md`=`c4885d5b67202712c0bcdecc6da634cb778cd6d3`<br>`README.md`=`38664991fe614acfc516eb6743dde6dce754191c` |
| 試験区分（`TestClass`） | 今回は`SOURCE`／`CONFIG`のE0 mapping。現在のinstalled collector、Windows evidence validator、C32 checker／testは実行していない。C33の旧`LIVE_RUNTIME` subgate・旧総合FAILは履歴のまま保持し、旧source証拠をcurrent candidateへ転用しない。 |
| 証拠ID（`Evidence ID`） | NONE。現candidateのinstalled evidence file／bundleは不在。E0 mapping自体を製品EvidenceやN10 Acceptanceへ登録しない。 |
| 阻害要因（`Blocker`） | 既存Registry 18件（16 unresolved／2 resolved）を維持。Phase44関連の既存unresolvedは`windows_evidence_provenance_isolation`、`windows_installer_first_run_smoke`、`windows_setup_doctor_smoke`、`windows_broker_installed_smoke`、`audit_anchor_external_tamper_evidence_proof`、`comprehensive_extension_rev1_completion`。新規Blockerなし。 |
| 状態 | Phase44のE0-04全21項目を編集前main `0ea5fb8c829deb0436118965f2a5f10060cfad71`の42 SourceSHAへ結合。Phase42のC32静的対応付けとPhase34/C33の旧Windows証拠を参照し、再試験していない。N10/N11、W06、E0はOPEN、`release_ready=false`。 |
| 残余 | 現行ソースと同一候補に結び付いたWindows導入済み証拠、別利用者profileでの新規collector実行、通常InstallerからTask実行・更新／回復／撤去までの一連の利用経路、独立した監査アンカー、Windows物理確認を各正本工程で処理する。N10五OS受入れ／N11全IDの証拠結合／正式Releaseは後続工程。 |

Phase44のE0-04個別欄を21項目で記録し、42件（42 unique path）のSourceSHAを編集前main `0ea5fb8c829deb0436118965f2a5f10060cfad71`のGit blobへ結合した。C32のPhase42構造監査対応付け、C33/Phase34の旧installed evidence範囲・総合FAILを保持し、checker・collector・installed productを再実行していない。current candidate evidence bundle不在、Windows blocker群、N10/N11/W06/E0 OPEN、Registry 18件、`release_ready=false`を維持し、新Evidence／Blockerなし。

#### Phase/K crosswalkの判定

#### Phase 45 E0-04個別記録（2026-11-14）

##### Phase 45 個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | 旧D4 Phase 45「正式公開」。現行v4ではN10/N11、施工線G/H/I/J、K16/K17/K19に接続する最終Release準備・責任者判断のsource→consumer照合ID。E0 mapping自体はRelease Acceptanceではない。 |
| 旧ID | `docs/D4_POCKET_PHASE_MAPPING.md` Phase 45／総合機能拡張rev1 C34「正式release前作業」。C34の旧進捗`未着手`を保持する。C34要求にあるPublisher identity、正式署名、監査オフライン署名、実機／標準OS証拠、strict release、Owner GOを現行v4の責任区分へ接続する。 |
| 原文参照 | 採用済みv4仕様S02/S04 N10・N11/S05 K16・K17・K19/S07〜S10、工程R01〜R03 G〜J/R05/R07/R08、施工指示I02/I06/I07/I08/I10/I11/I12、`docs/D4_POCKET_PHASE_MAPPING.md` Phase 45、総合機能拡張rev1工程表・進捗C34、`release_blockers.registry.json`、`docs/FINAL_QA_QUEUE.md`、`RELEASE_CHECKLIST.md`。 |
| 対象 | 同一Release候補のsource／contract／OS別package・artifact／installed evidence／candidate identity／18 blocker／strict gate結果を集約し、正式な配布資格・署名・Audit keyとOwnerの独立判断へ渡す責任境界。公開や鍵投入そのものは本mappingの作用に含めない。 |
| 目的 | 技術的な構築・検査・候補生成、OS別配布資格、production signing／Audit key、`release_ready`、Owner GOを別々に評価し、未成立の候補を正式公開済みとして扱わない。 |
| 生成元（`Producer`） | 採用済みv4要求と同一source commit、各OSのpackage／build pipeline・package Manifest、install/runtime/evidence collector、Source／Artifact hash、Release Registry、QA記録、署名・Publisher・Audit資格、および責任者の独立判断。後三者の本人決裁情報は技術toolが生成しない。 |
| 実Consumer | 各OS artifact／candidate data→package Schema／OS固有検査。Windows installed evidenceのみ`tooling/windows_release_evidence.py`が4 evidence-sourced blockerを再評価する。Registry→`tooling/release_gate_check.py`の通常／`--strict-release`判定→`RELEASE_CHECKLIST.md`／`docs/FINAL_QA_QUEUE.md`。`tooling/evidence_bundle.py`は`development_evidence`専用。全5 OSのartifact／installed evidenceを一つのcandidate identityで統合するconsumer、および最終Owner decision consumerは未成立／未特定。 |
| 契約／Data | candidate source commit／SourceSHA、contract／Manifest version、OS／arch／build環境、OS別ArtifactSHA、install/runtime/config/Audit identity、evidence class／provenance、18 blocker status、Q0〜Q7結果、Publisher／signing identity、production Audit checkpoint、`release_ready`、Owner GO record。Simulator／VM／hosted／physical evidenceと各ArtifactSHAを区別する。 |
| 正常時作用 | 必須要求・OS別candidate evidence・署名／配布条件・Audit条件・全Registry条件が同一候補へ結合され、strict gateが条件を満たす場合に、その範囲をRelease review材料として提示する。Gate PASS単独で公開・署名・`release_ready=true`・Owner GOを作らない。 |
| 否定時作用 | 必須証拠の欠落／不正、source・artifact・OS・identityの不一致、未解決blocker、Q0〜Q7未完了、strict gate failureは未成立／拒否として残す。旧候補・別OS・別Artifactの証拠を合成せず、resolved 2件の履歴statusも改変しない。 |
| 停止／Recovery | provenance不一致、validator／gate失敗、署名・Publisher・production Audit key・配布資格・Owner GOの欠落時はRelease promotion／公開を停止し`release_ready=false`を保持する。修正後は影響候補の正しいSource／Artifactから証拠を再生成する。鍵・本人判断を模擬生成せず、旧evidenceをcurrent candidateへ再利用しない。 |
| 標準OS候補 | N10のWindows／macOS／Linux native／Android／iOS各標準環境・version／arch候補を参照する。Linux nativeは必須対象。Windows所有PCの最終確認は別証拠として保持する。各OSの保証範囲・仮想実行条件・未確定値はN10のOS候補表に従い、本mappingで物理／VM受入れを実施しない。 |
| 配布物 | Windows一般Installer、macOS通常bundle／package、Linux native package、Android通常配布Artifact、iOS許容配布Artifactを個別候補とする。現N10記録上、Windows formal installer、macOS installer／署名bundle、Linux正式package／保証範囲、Android/iOS production identity・通常配布／lifecycleは未成立。Debug／portable／bootstrap／Simulator Artifactを正式配布と扱わない。 |
| 既存実装 | 18項目Registry、通常／strict Release Gate、Manifest／package Schema、Evidence Bundle、Windows installed evidence collector／validator、Release Runtime Assertions、Release Checklist、Final QA Queue、N11 source mapがある。現`release_evidence/`は`.gitkeep`のみ。Bundleはdevelopment-only、Windows validatorは限定4項目であり、5OS統合Release dossier／全ID candidate gateではない。 |
| BUILD_GAP | E0全要求・OS artifact・同一candidate evidence・Consumer・blocker条件の行単位結合、全OSの通常配布／installed evidence、N10 lifecycle、Q0〜Q7、production signing／Publisher／Audit key、最終Release review／Owner decision consumerが未成立。5OS共通Release artifact/evidence consumerは現行sourceから特定できない。 |
| ソース識別値（`SourceSHA`） | 照合HEAD=`8c22bcdb7aa8c6597011f0bf7491cd143f6f1f45`。以下28 unique tracked pathのGit blobを編集前mainで個別照合した：<br>`AGENTS.md`=`1eafcadd129fd01f1f2f683594c381605ed7d48d`<br>`docs/QE_2026-11-13/採用記録.md`=`f159c1aa6ea5bc6e80e0877d8068f7e44da4c4d3`<br>`docs/QE_2026-11-13/01_D4Pocket_統合製品実装仕様書_v4.md`=`352c53180cbad07888beb956aad0fb2a58074c58`<br>`docs/QE_2026-11-13/02_D4Pocket_統合開発工程表_v4.md`=`d32dd251e5f7cbb97d92f6e7540d6b6ed0ecec05`<br>`docs/QE_2026-11-13/03_D4Pocket_Codex施工指示書_v4.md`=`49447973e88c50a09b2f4978adfed84fd464c6d0`<br>`docs/QE_2026-11-13/進捗.md`=`af070daa070bad66a0b54b345c0d9c055e2f5373`<br>`docs/QE_2026-11-13/E0_要求consumer台帳.md`=`bd48b7693affb65b90debad6f5a51735843e768c`<br>`docs/D4_POCKET_PHASE_MAPPING.md`=`064ea334f86b137d94fba2370d2a2fd732f6e972`<br>`docs/総合機能拡張_rev1/工程表.md`=`0d03c001efeabfda53d3c0b44fc5c9b6e0f1e14e`<br>`docs/総合機能拡張_rev1/進捗.md`=`1392588f0cd5273013578e84d8e33c5c9dc9a01a`<br>`release_blockers.registry.json`=`244adc04a01e7b406c4ebcdeb829f7077d4aa35f`<br>`tooling/release_gate_check.py`=`22a8d7bfe9adb259f44ce8501fb215652e1d58c6`<br>`tooling/evidence_bundle.py`=`68dead851236b0f1b69899151c37371e593ff3f9`<br>`tooling/release_runtime_assertions.py`=`0a0a351fe17151ad86517bd0a965358f40c61490`<br>`tooling/windows_release_evidence.py`=`6f7957c6193c86c7bc15165e743f11651464f29c`<br>`tooling/manifest.py`=`aa4525ff20293cb8641e5598a2439561a3bfbd3b`<br>`tooling/conformance_tests/run_conformance_skeleton.py`=`c6f31d8cb561b48513c6784e54c85969c9cb953e`<br>`tooling/final_development_audit.py`=`913c81bc22f71bc5bb3447fa4f4bd266326932ec`<br>`docs/FINAL_QA_QUEUE.md`=`7b332cccab70bb68aa1ff4f8e23acc0e816d8282`<br>`docs/RELEASE_VALIDATION.md`=`b4b0462e628ad8176264473879dedb1f6201da7c`<br>`RELEASE_CHECKLIST.md`=`35f31043a9311d49c94acdc9631e30b61949c65d`<br>`CLAIM.md`=`0bfbf988469e0f34c9e7f96f805ce26faf3b1044`<br>`specs/d4_pocket_product_package_manifest.schema.json`=`3151ba81a869b87e6c0fa8f8633a0e424fc43ae7`<br>`specs/d4_pocket_linux_product_package_manifest.schema.json`=`34087f190e7189641b34ae3163b568a038c9f1c2`<br>`docs/specs/windows-product-package.md`=`19c5c45dc8cbdbd4560b7b2cb8fe5d8e9f82a2c3`<br>`docs/specs/linux-product-package.md`=`faad8b2a9487afc9a818ad3938e0ff92c77cc268`<br>`docs/WINDOWS_RELEASE_PLAN.md`=`5d4c35f8c9599c9f38b7c02ae475710e8c5a64e8`<br>`docs/WINDOWS_RELEASE_EVIDENCE.md`=`9c836b028c352da8d37f9feae53dcb27b537199d`。 |
| 試験区分（`TestClass`） | 本作業は`SOURCE`／`CONFIG`のE0 mapping。18 Registry IDのS07行と現Registry nameは18/18一致・重複なし。既存strict-gate／Registry／Bundle／Windows-validator testは主に`FIXTURE`／`INTERNAL_STATE`。今回のstrict release、5OS build／installed QA、署名、Owner decisionは`NOT_RUN`。 |
| 証拠ID（`Evidence ID`） | `NONE`。SourceSHAはsource mappingの識別値。現candidateのOS artifact／統合installed evidenceはなく、E0 row・development bundle・過去runをRelease EvidenceやOwner GOへ登録しない。 |
| 阻害要因（`Blocker`） | Registry全18件を既存statusで保持（16 unresolved／2 resolved）：`windows_evidence_provenance_isolation`、`windows_installer_first_run_smoke`、`windows_setup_doctor_smoke`、`windows_broker_installed_smoke`、`audit_anchor_external_tamper_evidence_proof`、`owner_go`、`linux_native_product_support`、`rev2_mobile_device_evidence`、`rev2_mobile_distribution`、`rev2_desktop_launch_regression`（resolved）、`comprehensive_extension_rev1_completion`、`rev2_desktop_product_distribution`、`windows_distribution_identity`、`rev2_flutter_broker_channel_boundary`、`rev2_export_owner_ui_authority_path`、`rev2_module_pruning_binary_and_measurement`、`windows_rust_integration_test_execution_policy`（resolved）、`rev2_mobile_flutter_native_device_link_boundary`。既存条件だけを参照し、新規Blocker／Acceptanceは追加しない。 |
| 状態 | Phase45のE0-04全21項目を編集前main `8c22bcdb7aa8c6597011f0bf7491cd143f6f1f45`の28 SourceSHAへ結合。C34旧`未着手`、N10/N11/E0 `OPEN`、Q0〜Q7 `DEFERRED`、Registry 18件（16 unresolved／2 resolved）、`release_ready=false`を維持。正式Release／Owner GOは実行せず、Phase45 AcceptanceはOPEN。 |
| 残余 | 同一candidateの5OS配布Artifact／installed evidence／保証matrix、N11全IDと18 blockerの個別証拠結合、正式署名・Publisher identity・production Audit key、strict release、Windows所有PC確認、Q0〜Q7、Ownerの独立GOが未成立。これはE0 source mapping完了と別Acceptanceである。 |

Phase45 E0-04 mappingでは、v4要求・C34履歴・Registry／Gate／Evidence consumersの責任境界を記録し、編集前mainの28 source blobを照合した。S07 blocker IDはRegistryと18/18一致し、重複・欠落なし。C34旧`未着手`、N10/N11/E0 OPEN、Q0〜Q7 DEFERRED、Registry 16 unresolved／2 resolved、`release_ready=false`は変更せず、新Evidence／Blockerなし。strict release、配布・署名・Owner decision、5OS installed product試験は`NOT_RUN`。

#### Phase/K crosswalkの判定

旧Phase 0〜45の46 ID、K01〜K20の20 IDを、現v4 N01〜N11・R03 A〜J・正本Consumerへ対応付けた。旧C番号との対応は履歴範囲として保持し、P0〜P13は別のv4 S05 direct mappingに従う。Phase固有SourceSHA/TestClassは各Phase行、K03〜K20は参照N節へ接続し、Phase/K固有Evidence IDはNONEを維持する。K01とK02は下記E0-04全21 fieldを個別記録したがK03〜K20の個別fieldは未完。ここでのmappingは範囲・Consumer参照の整備だけであり、N/K/Phase Acceptance、E3 BUILD_COVERED、全OS QC/QAをCLOSEDにしない。N11 Evidence ID・Blocker・全OS artifact/standard guaranteesとの行単位結合、B0〜B6、Q/FQ/W/QE/AUD全数照合は継続。BUF-R01〜R09とBUF-T01〜T24は個別21 fieldを記録済みだが、AcceptanceはOPEN。

### K01 E0-04 21項目の個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | K01 Runtime Catalog／Agent／Task／Session／Workspace／永続state／Launcher／IPC／Broker／Capability。v4 S05能力IDであり、Acceptance状態は関連するN01／N03／N04の現行statusで判定する。 |
| 旧ID | P0／P1、Phase 2〜8およびPhase mapping上の旧C対応。P0の既存基盤・P1の限定Agent Task実績は履歴範囲として保持し、K01全能力・全OSの完成へ昇格しない。 |
| 原文参照 | 採用済みv4仕様S02/S04 N01・N03・N04/S05 K01/S08/S09、工程R02 E0-04〜08/R03 A〜C/R05/R06 W01・W03/R07、施工指示I04-A/B/C/I07/I10/I11、`docs/D4_POCKET_PHASE_MAPPING.md` Phase 2〜8、既存N01/N03/N04 E0行。 |
| 対象 | Runtime／Agent Catalogと登録、Task・Session・Workspaceの束縛、永続state／履歴、Launcher／IPC、Rust Broker、AdapterとRuntime Capabilityを経由する一般Agent実行経路。Compare／Handoff固有作用はN04/K02に残し、K01へAcceptanceを重複追加しない。 |
| 目的 | 使用者が選んだRuntime／Agentと現在のSession・WorkspaceをBroker統治下のTask作用へ結び、実状態・結果を返す。Runtime Capability・Agent metadata・UI stateはPermission／Approval／Trust／Authorityを生成せず、全OS・複数Runtimeでの適用範囲を正しく示す。 |
| 生成元（`Producer`） | Runtime／Agent登録・Adapter probe、Runtime Centerの選択、Agent CenterのTask要求、Session／Workspace登録、Flutter desktop transport／native Launcher、Rust Broker registry・protocol・Adapter。これらの各producerを通じてもCredentialや権限をUIへ渡さない。 |
| 実Consumer | DesktopのRuntime／Agent Center→`AgentTaskClient`／`AgentTaskExecutionCoordinator`または`RuntimeDialogueClient`→desktop Broker transport／Launcher／IPC→Rust `runtime_registry`／`adapter_center`／`protocol`／`dialogue`／`workspace`→登録Codex CLI Adapter→Task／Session／Workspace state・result consumer。既存source上Desktop Codex CLI Taskは`WIRED`。Mobile `Agent Status`はmetadata表示に限りTask開始consumerではない。 |
| 契約／Data | `runtime.schema.json`、`agent_runtime.schema.json`、`agent_adapter.schema.json`、`agent_cli_runtime_workspace_registration.schema.json`、`agent_workspace.schema.json`、`agent_task_request.schema.json`、`agent_task.schema.json`、`agent_task_scratch_recovery.schema.json`、`agent_task_owner_approval.schema.json`、Runtime Dialogue／Workspace schemas。Runtime／Adapter／Session／Workspace ID、Task request、Workspace binding、Capability、Permission／Approval receipt、state／result hash／Audit correlationを扱う。Capability≠Permission、metadata≠Authorityを維持する。 |
| 正常時作用 | 現在登録済みRuntime／Agent／Workspaceを選択し、BrokerがRuntime・Adapter対応能力・現在のWorkspace binding・個別Permission／Approvalを再評価してTaskを実行する。Task状態と結果をIDに束縛して読戻し、許可範囲のprojectionへ渡す。Codex CLI以外のTask-capable Runtimeは現実に接続されていないためunsupportedのままにする。 |
| 否定時作用 | unknown／unsupported Runtime、欠落・malformed Schema、staleなRuntime／Session／Workspace ID、Capability不一致、Workspace scope不一致、Permission／Approval欠落・再利用、UI／Adapter metadataによる権限昇格を拒否する。MINIDORAの対話接続をTask能力へ読み替えず、未対応Taskをfail-closedにする。 |
| 停止／Recovery | Agent Taskのcancel／deadline／crash／Broker再起動時に成功を偽装せず、実装済みJournal／Recovery経路へ渡し、消費済みPermission／Approvalを復元しない。Codex CLIの限定recovery履歴はその条件に限る。MINIDORA unsupportedと未知Runtimeは起動を拒否し、結果・停止状態が不明なら再送／completed化しない。K01 mappingではRecovery試験を再実行していない。 |
| 標準OS候補 | Windows installed Desktop、Linux native（Ubuntu 24.04 LTS／x86_64候補）、macOS installed（保証条件UNKNOWN）、Android API 35 Google APIs／x86_64 Emulator候補、iOS iPhone 16 Pro／iOS 18.5 Simulator候補。Linuxは必須製品対象。これらは候補値で、現K01の五OS Acceptanceを示さない。 |
| 配布物 | Windows portable ZIP／D4PKG01候補、Linux Ubuntu 24.04／amd64 portable tar.gz・bootstrap deb・D4PKG01候補、macOS macos-15 manual debug/UI workflow、Android debug APK/AAB、iOS Simulator app。各OSの通常配布・同一candidate installed Runtime／Task pathはN10でOPEN。 |
| 既存実装 | Desktop Agent／Runtime Center・Task client/coordinator・Runtime Dialogue/Workspace client、agent_orchestration package、Rust desktop Launcher／Broker protocol・runtime registry・adapter center・dialogue・workspace・Codex CLI Adapter・Task scratch recovery、関連Schema／UNIT／FIXTURE／E2E sourceがある。N01のWindows W01およびLinux #286は限定履歴。MINIDORAは対話AdapterのみでAgent Task capabilityはfalse。 |
| BUILD_GAP | `Codex CLI`以外の第2実行基盤がなく、`Runtime`登録から`Broker`を介した`Task`の実行・結果・失敗・権限・承認・監査までの経路は未成立。`MINIDORA`は対話のみ、`Mobile`に`Task`開始機能なし。五OSの標準配布・導入済み候補での統合状態も未成立または未確認。`Catalog`／`Schema`／`fixture`の存在だけでは不足を埋めない。 |
| ソース識別値（`SourceSHA`） | 照合HEAD=`70989d1813602c13e6edf0d91526c97975aae5a9`。以下47 unique tracked pathのGit blobを編集前mainで個別照合：<br>`AGENTS.md`=`1eafcadd129fd01f1f2f683594c381605ed7d48d`<br>`docs/QE_2026-11-13/採用記録.md`=`f159c1aa6ea5bc6e80e0877d8068f7e44da4c4d3`<br>`docs/QE_2026-11-13/01_D4Pocket_統合製品実装仕様書_v4.md`=`352c53180cbad07888beb956aad0fb2a58074c58`<br>`docs/QE_2026-11-13/02_D4Pocket_統合開発工程表_v4.md`=`d32dd251e5f7cbb97d92f6e7540d6b6ed0ecec05`<br>`docs/QE_2026-11-13/03_D4Pocket_Codex施工指示書_v4.md`=`49447973e88c50a09b2f4978adfed84fd464c6d0`<br>`docs/QE_2026-11-13/進捗.md`=`f84c5e41d3b5cb59096817d1f365ab580fa7b8ad`<br>`docs/QE_2026-11-13/E0_要求consumer台帳.md`=`85796ca89429e1aba7638775c71a43523ea44f79`<br>`docs/D4_POCKET_PHASE_MAPPING.md`=`064ea334f86b137d94fba2370d2a2fd732f6e972`<br>`apps/desktop_flutter/lib/screens/agent_center.dart`=`2472faeb31a64d624423c30a0a30d0f153d2e2a3`<br>`apps/desktop_flutter/lib/screens/runtime_center.dart`=`d2c7af172073d2b10c76a6416b90f011f10e95db`<br>`apps/desktop_flutter/lib/services/agent_task_client.dart`=`f54cefac29eba3dcf383bba19b4f64ee682e6217`<br>`apps/desktop_flutter/lib/services/agent_task_execution_coordinator.dart`=`5d8f48d0557bdcb1af838239abf9651893bce7f1`<br>`apps/desktop_flutter/lib/services/runtime_dialogue_client.dart`=`a0853ee7327cafa60ee89b0c5ee49312b73c46c4`<br>`apps/desktop_flutter/lib/services/workspace_client.dart`=`870b48e8f4bbd648bcfebc5886addca3784bcfaf`<br>`packages/agent_orchestration/lib/agent_task_client.dart`=`a31fbb9f980a0039f809fd763c91f055fe9d02d0`<br>`packages/agent_orchestration/lib/agent_task_execution_coordinator.dart`=`feb9aacb7ed77e6f4e76e9d9371290decca9cf7b`<br>`native/rust_helper/src/desktop_launcher.rs`=`6a9f335de07fa474ca10043aea63ad6e988b572d`<br>`native/rust_helper/src/broker/protocol.rs`=`9b2076d5f31ca6d77f1f87973d1b394aaf751961`<br>`native/rust_helper/src/broker/dialogue.rs`=`c3c3fddb630dd06b7e6cc1c13b39f3d4f6f6a999`<br>`native/rust_helper/src/broker/runtime_registry.rs`=`9ff28610862586a599848d75f9e891a6c6cadf2f`<br>`native/rust_helper/src/broker/adapter_center.rs`=`66575bb77e4a375dca4f8488f97df442960bb5a9`<br>`native/rust_helper/src/broker/workspace.rs`=`68075d680a3c7d57785083e64d3d21523d79374c`<br>`native/rust_helper/src/broker/workspace_root.rs`=`15f208e0b855b8cee53e878d2b27aaf457ea2937`<br>`native/rust_helper/src/broker/runtime_lifecycle.rs`=`2746ec301e9a47a5ececf1485ca7ad8a17304388`<br>`native/rust_helper/src/broker/agent_task_scratch.rs`=`de743cd819fb4533c9c3070c4e7bcae82e42d938`<br>`native/rust_helper/src/adapters/codex_cli.rs`=`e5c9d1581ae9a1c0a3ae7fcfa626fdca6fea6d28`<br>`specs/runtime.schema.json`=`bdef33f26fe06b334eb6f94124930fd08594bbc9`<br>`specs/agent_runtime.schema.json`=`b8e031e53f582048211f0e1a6deeb2e5cb87e4fb`<br>`specs/agent_adapter.schema.json`=`e0792589a2bae168ba221af4f4f9abb505815c12`<br>`specs/agent_cli_runtime_workspace_registration.schema.json`=`32a09f2ce33a5a215d64350ee22e24e93f9037c7`<br>`specs/agent_workspace.schema.json`=`47058678398b5aac1b0b1467cac7e78daef72b32`<br>`specs/agent_task_request.schema.json`=`cb25bb3684656bfd670ea5832bbb7fc5988845b6`<br>`specs/agent_task.schema.json`=`a552389072088d6cd42b3588fefda332be4ae6b6`<br>`specs/agent_task_scratch_recovery.schema.json`=`40f1eac71cd9645396ae7230d49cad25f3b77237`<br>`specs/agent_task_owner_approval.schema.json`=`d190c9a8cc3d715eb199459c37c14e5de0ad6ad8`<br>`specs/runtime_dialogue_request.schema.json`=`fe0bd153eaf3a0a2354b25cffc3ed9af934fbae1`<br>`specs/runtime_dialogue_session.schema.json`=`5bfc5b8c86ce40626d76b1e37ebdf7989afdef90`<br>`specs/workspace_startup.schema.json`=`cafe1667c338011a207418c654f20c87a2bdf2a3`<br>`apps/desktop_flutter/test/agent_task_client_test.dart`=`0477a0bbbb321082494b6b55711815d34419fdd9`<br>`apps/desktop_flutter/test/runtime_dialogue_test.dart`=`899912f92ee3751f8616632d434e04f95ee5ef77`<br>`native/rust_helper/tests/agent_task_production_e2e.rs`=`fdc7565aa58fa7466e6e24e5492321e8015567eb`<br>`native/rust_helper/tests/agent_task_crash_recovery_e2e.rs`=`23517fcfe8ef94b2357a94e04879d423c3360501`<br>`native/rust_helper/tests/unit/codex_cli_fixture.rs`=`40e5750bf267ccf9e583971e0d2ba4e6125d9a5d`<br>`native/rust_helper/tests/unit/agent_task_fixture_fs.rs`=`d19ac79583b98bec9b828d6e064fe832af95bfa5`<br>`native/rust_helper/tests/unit/workspace_registry.rs`=`0a743ac2aba371e86dba3e06b7d3fe5fff93db44`<br>`native/rust_helper/tests/unit/workspace_protocol.rs`=`589a168fc63df02f8f27bc2d9c7cb158ee91408a`<br>`native/rust_helper/tests/workspace_startup.rs`=`e497c2747d58fba4d5d31c9fe9d7fc6a4152510a`。 |
| 試験区分（`TestClass`） | 現sourceにはFlutter UNIT／WIDGET、Rust UNIT／FIXTURE、ignored／gated `LIVE_RUNTIME`／`LIVE_PRODUCT`候補がある。今回のK01作業はSource mappingであり、これらのtestは`NOT_RUN`。W01/G1およびLinux #286はN01記録内の元OS／Artifact／scopeに限定した履歴で、K01全体や第二RuntimeのEvidenceではない。 |
| 証拠ID（`Evidence ID`） | `NONE`。既存W01/G1 Windows Codex CLI限定証拠とLinux #286 Ubuntu hosted fixture証拠はN01の履歴参照に限る。本K01 mappingを新Acceptance Evidenceへ登録しない。 |
| 阻害要因（`Blocker`） | N01／N03／N04とW03/G3はOPEN。関連し得る既存Registry IDは`windows_broker_installed_smoke`、`windows_evidence_provenance_isolation`、`linux_native_product_support`、`comprehensive_extension_rev1_completion`、`rev2_mobile_device_evidence`、`rev2_mobile_flutter_native_device_link_boundary`。Registry 18件（16 unresolved／2 resolved）・`release_ready=false`を維持し、N11で直接帰属が確定するまでは既存条件以上へ拡張しない。新規Blockerなし。 |
| 状態 | K01のE0-04全21項目を編集前main `70989d1813602c13e6edf0d91526c97975aae5a9`の47 SourceSHAへ結合。K01 product acceptanceをCLOSEDにせず、N01／N03／N04、W03/G3、E0をOPEN、Evidence IDをNONEのまま保持する。 |
| 残余 | 第二Task-capable Runtimeとその実Adapter／Capability／Task/Result/Permission/Approval/Audit consumer、MINIDORAのTask接続、Mobile Task入口、全OS標準配布・installed candidate、同一候補のRuntime／Session／Workspace state・停止／Recovery証拠を後続N01/N03/N10/N11へ接続する。K02 Compare/Handoff acceptanceは別IDに残す。 |


### K02 E0-04 21項目の個別照合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | K02 独立Agent Compare／Handoff、Workspace分離、同時Task、選択結果適用、Authority非継承。v4 S05能力IDであり、製品AcceptanceはN04／W03／G3の現行statusで判定する。 |
| 旧ID | P2／P3、Phase 7／8／24〜27、C15〜C19、K05／K11／K12、W03／G3。旧Compare／HandoffのProduct Build CLOSED・fixture・限定evidenceは当時の範囲で保持し、K02の現Acceptanceへ昇格しない。A2AとHostの外部接続条件はN04内の別Consumerとして保持する。 |
| 原文参照 | 採用済みv4仕様S04 N04・S05 K02、工程R02 E0-04〜08／R03線B・C／R06 W03・G3、施工指示I04-B/C・I07、`docs/D4_POCKET_PHASE_MAPPING.md` Phase 7／8、N04 E0行。仕様原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`。 |
| 対象 | Desktop Agent Centerの同一Task比較、Agentごとに独立したSession／Workspaceでの並列Task、Task IDに束縛したresult／diff／test申告の比較、明示選択結果の別target Taskへの適用、および内部Agent Handoff package。外部A2A作用・Host registryはK02の内部Compare/Handoff acceptanceへ混在させず、N04内の別scopeに残す。 |
| 目的 | Agent A/BのTask・Session・Workspace・結果を分離したまま同じ目的を比較し、操作者が選択したboundedな公開結果だけを新しいtarget Taskへ渡す。sourceのPermission／Approval／Credential／Trust／Authority／hidden stateは継承せず、受信側で現在のWorkspace・Permission・Approvalを再評価する。 |
| 生成元（`Producer`） | Agent CenterのCompare入力が共通Task本文と異なるAgent／Session／Workspaceを選び、二つの個別preflight／開始要求を生成する。result取得は各Task IDへ束縛し、result選択とApply先は独立操作とする。Handoff生成は完了source Task、hash一致したfull result、明示公開承認、選択Artifactとtarget sessionを入力とする。 |
| 実Consumer | Agent Center → `agent_coordination.dart` → Desktop `AgentTaskClient`／`AgentTaskExecutionCoordinator` →各Session用Broker inspect／preflight／start／status／result。Compare結果適用は直接filesystemを書かず、選択済みsource resultを固定したうえでtarget Session／Workspace向けの新規Task preflight／通常AgentTask consumerへ渡す。Handoffは`agent_handoff.dart`→同じ通常AgentTask経路へ接続する。source上の配線はinstalled受入を意味しない。 |
| 契約／Data | `agent_handoff.schema.json`、`agent_handoff_package.schema.json`、`agent_task_request.schema.json`、`agent_task.schema.json`、`agent_task_scratch_recovery.schema.json`、`agent_task_owner_approval.schema.json`、`agent_workspace.schema.json`。Agent／Session／Workspace／Task ID、Task本文hash、結果hash・visibility・bounded result／diff／test申告、handoff lineage、target binding、Permission／Approval／Audit参照を扱う。handoff packageへCredential・Permission・Approval・Trust・Authority・hidden stateを含めない。 |
| 正常時作用 | 同一Task本文を別Session／Workspaceで個別にpreflightし、task startを独立Task IDとして並列化する。状態・resultを各IDに束縛して比較し、test resultはAgent申告を未検証の値として表示する。操作者が選んだ結果だけを別target Taskの新規requestへ入れ、target側の現在条件で再preflightした後、通常のPermission／Approval／Task／Audit経路を通す。Handoff packageはapproved full resultと選択Artifactを上限内で投影する。 |
| 否定時作用 | 同一Session／Workspace、共通Task本文の不一致、Workspace参照欠落、staleなTask／result／target、結果hash不一致、未完了または未承認結果、hidden／非full result、malformed／上限超過packageをdispatch／Apply前に拒否する。片側start／query failureを他側の成功へ読み替えず、開始成否不明時に二重送信しない。受信AgentやmetadataからAuthorityを生成せず、fresh Permission／Approvalのないtarget Taskを拒否する。 |
| 停止／Recovery | 各Sessionの取消gate・Task ID・statusを独立させ、片方のcancel／failureが他方の状態を書き換えない。dispatch結果が曖昧ならunknownとして停止し、自動再送しない。Apply／Handoff preflightはTask実行・権限移送を行わず、targetの現Workspace・Permission・Owner Approvalを別途評価する。既存Journal／scratch recoveryは通常Task側の責任であり、本mappingではCompare/Handoff installed recoveryを再実行していない。 |
| 標準OS候補 | Windows installed Desktop候補。Linux nativeは必須製品対象で、Ubuntu 24.04 LTS／x86_64を標準候補とする。macOS installed候補、Android API 35 Google APIs／x86_64 Emulator、iOS iPhone 16 Pro／iOS 18.5 SimulatorはN10候補として照合する。現sourceはDesktop Compare/Handoff中心で、五OS installed parityは未成立。WSL／Xvfb／fixtureをLinux native受入に転用しない。 |
| 配布物 | Windows portable ZIP／D4PKG01、Linux Ubuntu 24.04／amd64 portable tar.gz・bootstrap deb／D4PKG01、macOS macos-15 debug/UI workflow、Android debug APK/AAB、iOS Simulator appをN10の候補として記録する。通常利用者向け正式packageと同一installed candidateでのCompare／Handoff lifecycleは未成立であり、候補を配布Acceptanceにしない。 |
| 既存実装 | `apps/desktop_flutter/lib/screens/agent_center.dart`、Desktop `agent_coordination.dart`／`agent_handoff.dart`／Task client・coordinator、`packages/agent_orchestration/lib/agent_task_client.dart`／`agent_task_execution_coordinator.dart`、Rust Broker `protocol.rs`／`dialogue.rs`／`audit.rs`／`workspace.rs`／`workspace_root.rs`／`agent_task_scratch.rs`、Compare／Handoff／Task／Workspace SchemaとFlutter UNIT／WIDGET・Rust FIXTURE／gated E2E sourceがある。 |
| BUILD_GAP | Compare/Handoffのsource consumerとFake Broker／unit testは存在するが、同一installed candidate上で独立Workspaceの実体隔離、同時実Task、Task／Agent scratch非干渉、選択結果の新規target Task適用とAudit、failure／cancel isolationを一体で成立させた証拠はない。Workspace IDの相違だけではfilesystem isolationを証明しない。K02製品AcceptanceはOPENのまま。 |
| ソース識別値（`SourceSHA`） | 照合HEAD=`b523cf7928b568e701f10986e1088efd48a07638`。以下34 unique tracked pathのGit blobを編集前mainで個別照合：<br>`AGENTS.md`=`1eafcadd129fd01f1f2f683594c381605ed7d48d`<br>`docs/QE_2026-11-13/採用記録.md`=`f159c1aa6ea5bc6e80e0877d8068f7e44da4c4d3`<br>`docs/QE_2026-11-13/01_D4Pocket_統合製品実装仕様書_v4.md`=`352c53180cbad07888beb956aad0fb2a58074c58`<br>`docs/QE_2026-11-13/02_D4Pocket_統合開発工程表_v4.md`=`d32dd251e5f7cbb97d92f6e7540d6b6ed0ecec05`<br>`docs/QE_2026-11-13/03_D4Pocket_Codex施工指示書_v4.md`=`49447973e88c50a09b2f4978adfed84fd464c6d0`<br>`docs/QE_2026-11-13/進捗.md`=`95227ea83ce23f394dc92b3f0db240209994479a`<br>`docs/QE_2026-11-13/E0_要求consumer台帳.md`=`a4eef769e86d8d14b7af94728c18533e1afe4c30`<br>`docs/D4_POCKET_PHASE_MAPPING.md`=`064ea334f86b137d94fba2370d2a2fd732f6e972`<br>`release_blockers.registry.json`=`244adc04a01e7b406c4ebcdeb829f7077d4aa35f`<br>`apps/desktop_flutter/lib/screens/agent_center.dart`=`2472faeb31a64d624423c30a0a30d0f153d2e2a3`<br>`apps/desktop_flutter/lib/services/agent_coordination.dart`=`bdc13464f0e42b740a2bca3b7e8828a5019056d0`<br>`apps/desktop_flutter/lib/services/agent_handoff.dart`=`a85110d96cbade732ae0e1bfea2529f03f37523c`<br>`apps/desktop_flutter/lib/services/agent_task_client.dart`=`f54cefac29eba3dcf383bba19b4f64ee682e6217`<br>`apps/desktop_flutter/lib/services/agent_task_execution_coordinator.dart`=`5d8f48d0557bdcb1af838239abf9651893bce7f1`<br>`packages/agent_orchestration/lib/agent_task_client.dart`=`a31fbb9f980a0039f809fd763c91f055fe9d02d0`<br>`packages/agent_orchestration/lib/agent_task_execution_coordinator.dart`=`feb9aacb7ed77e6f4e76e9d9371290decca9cf7b`<br>`native/rust_helper/src/broker/protocol.rs`=`9b2076d5f31ca6d77f1f87973d1b394aaf751961`<br>`native/rust_helper/src/broker/dialogue.rs`=`c3c3fddb630dd06b7e6cc1c13b39f3d4f6f6a999`<br>`native/rust_helper/src/broker/audit.rs`=`eb1dcf5018d95c7fdf6171aadd4bca9554d1996f`<br>`native/rust_helper/src/broker/workspace.rs`=`68075d680a3c7d57785083e64d3d21523d79374c`<br>`native/rust_helper/src/broker/workspace_root.rs`=`15f208e0b855b8cee53e878d2b27aaf457ea2937`<br>`native/rust_helper/src/broker/agent_task_scratch.rs`=`de743cd819fb4533c9c3070c4e7bcae82e42d938`<br>`specs/agent_handoff.schema.json`=`e37a276f8c3e19816bc4f80824efce546a116d00`<br>`specs/agent_handoff_package.schema.json`=`a8ae1e2a3be6c0ed5666849fc6553c98083a1cc3`<br>`specs/agent_task_request.schema.json`=`cb25bb3684656bfd670ea5832bbb7fc5988845b6`<br>`specs/agent_task.schema.json`=`a552389072088d6cd42b3588fefda332be4ae6b6`<br>`specs/agent_task_scratch_recovery.schema.json`=`40f1eac71cd9645396ae7230d49cad25f3b77237`<br>`specs/agent_task_owner_approval.schema.json`=`d190c9a8cc3d715eb199459c37c14e5de0ad6ad8`<br>`specs/agent_workspace.schema.json`=`47058678398b5aac1b0b1467cac7e78daef72b32`<br>`apps/desktop_flutter/test/agent_coordination_test.dart`=`6783e87a492d8436a855a7088b7cece38d122bd5`<br>`apps/desktop_flutter/test/agent_handoff_test.dart`=`c6bc9bef8cc8d153ef59bf421efbd989185f8e0c`<br>`apps/desktop_flutter/test/agent_task_client_test.dart`=`0477a0bbbb321082494b6b55711815d34419fdd9`<br>`native/rust_helper/tests/agent_task_production_e2e.rs`=`fdc7565aa58fa7466e6e24e5492321e8015567eb`<br>`native/rust_helper/tests/agent_task_crash_recovery_e2e.rs`=`23517fcfe8ef94b2357a94e04879d423c3360501`。 |
| 試験区分（`TestClass`） | Compare projection／同一Task本文・Workspace分離・並列開始・片側失敗／取消、handoff hash・完了・full visibility・target bindingを確認するFlutter UNIT／WIDGET sourceとFake Broker testがある。RustのTask production/crash E2Eは別の限定Task経路またはgated候補でありCompare/Handoffのinstalled証拠ではない。今回のE0 Source mappingで製品testは`NOT_RUN`。 |
| 証拠ID（`Evidence ID`） | `NONE`。旧P2/P3の限定Product Build、Compare／Handoff fixture、N04のA2A/MAC-A2A metadata-only履歴をK02のinstalled Compare/Handoff acceptance evidenceへ転記しない。 |
| 阻害要因（`Blocker`） | N04／W03／G3はOPEN。Registry 18件（16 unresolved／2 resolved）と`release_ready=false`を保持し、新規Blockerは追加しない。既存blockerのN04/K02への直接帰属はN11の行単位照合までUNKNOWNとする。 |
| 状態 | K02 E0-04全21項目を編集前main `b523cf7928b568e701f10986e1088efd48a07638`の34 SourceSHAへ結合。K02製品AcceptanceをCLOSEDにせず、N04／W03／G3／E0をOPEN、Evidence IDをNONEのまま保持する。 |
| 残余 | 独立Workspaceのfilesystem隔離、同時実Agent Task、片側failure／cancel isolation、selected resultのtarget新規Task・fresh Permission／Approval・Auditまでのinstalled一体証拠が残る。外部A2A／HostはN04内の別scope、OS配布／installed parityはN10へ接続する。 |

## v4対応：BUF-R01〜R09／BUF-T01〜T24 Source→Consumer crosswalk

現行要求の正本はv4仕様S06（採用原本SHA-256 8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF）。緩衝基盤実装仕様書§2.1/§8.3（HEAD 5f1bb53のblob 2d0e98fb0b58ceec65244fde6a8fbf8e27c49b7e、SHA-256 A354E8493EE3A86DD480FCEF256FD9BDB569631B5735B3155FAC41EA77E38473）は採用済み要求の意味・来歴を参照する補助資料とし、現行status・CLOSED範囲・受入を上書きしない。以下は要求ID全33件の固有目的・現Consumer・状態境界を明示する。実装fileごとの現行blob SHAはN08/N10/N11各SHA表、過去自己適用evidenceはdocs/緩衝基盤_rev5補遺_QC/B5_自己適用.mdと採用記録を参照する。

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

### BUF-R01〜R09（E0-04全21 fieldの個別記録）

この小節は各R要求をE0-04の21 fieldで個別化したSource→Consumer記録である。要求の現行意味はv4仕様S06を優先し、採用済み緩衝補遺は意味の来歴と限定証拠の参照に使う。補遺の旧QC工程や旧CLOSEDは、現行製品Acceptanceへ昇格させない。各SourceSHAは照合基準HEAD 5f1bb53c827cf52f8a2cb012f632931374b6989eのGit blob。UNIT／CONFORMANCE／DOCの存在は、この文書単位で試験を実行したことや製品Acceptanceを示さない。

#### BUF-R01 全ボトルネックと欠測を取得可能範囲付きで全件追跡

| E0-04項目 | 個別値 |
|---|---|
| 現ID | BUF-R01 |
| 旧ID | BUF-R01（識別子を維持） |
| 原文参照 | v4仕様S06、仕様原本SHA-256 8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF。採用済み補遺の実装仕様書§1.2・§3は来歴資料。 |
| 対象 | D4Pocket開発で観測可能な障害・失敗・未解明事象と、その取得可能なsource／run／job／log／artifactの母集団。 |
| 目的 | 期間・証拠源・実行対象を明示し、既知・未解決・取得不能を全件追跡する。未知を既知原因や解決済みへ変換しない。 |
| 生成元（`Producer`） | Repository履歴、手動Actions履歴、source／workflow差分、実行log／artifactの可用性、開発作業で観測したfailure。 |
| 実Consumer | B0原因対応台帳・不足証拠一覧が事象と欠測を記録し、E0要求台帳とN11 Evidence／Gateが現候補への関係を受け取る。既存B0の限定CLOSEDはこの全母集団の完了を意味しない。 |
| 契約／Data | 既存B0台帳とE0-04の出典、対象、Producer、Consumer、SourceSHA、証拠区分、Blocker、残余。根拠の種類と直接観測／推定／UNKNOWNを分離する。新しいruntime schemaは追加しない。 |
| 正常時作用 | 取得可能な各記録を出典とscopeへ結び、欠測・重複・未解明の状態を残したまま要求と現Consumerへ対応付ける。 |
| 否定時作用 | HTTP 410／404、artifactなし、source不一致、同型failureのみの場合に個別原因・因果・解決を推定しない。 |
| 停止／Recovery | 根拠不足はUNKNOWN／欠測として保持し、取得元や再取得可能性を記録する。過去記録を黙って再分類せず、追加取得時は該当行を追補する。 |
| 標準OS候補 | Windows、macOS、Linux native、Android、iOS。OS別のfailure母集団や候補version／archは未確定値をUNKNOWNとする。Linux nativeは必須製品対象。 |
| 配布物 | 5 OSの配布候補はN10の現行対応表に従う。Installer／package／version／archは未確定ならUNKNOWN。WSL／Xvfbの記録をLinux native製品証拠にしない。 |
| 既存実装 | B0_原因対応台帳.md（限定された履歴・原因対応）、B0_不足証拠一覧.md（欠測と未解明）、E0 ledger（要求対応）。 |
| BUILD_GAP | 全開発期間・全取得可能sourceの一意な母集団化と、現在の要求・現Consumer・Evidence Registryまでの全件結合が未完。 |
| 参照元ハッシュ（`SourceSHA`） | v4仕様S06原本のSHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`。照合時Git commit=`5f1bb53`のGit blob: `docs/緩衝基盤_rev5補遺_QC/B0_原因対応台帳.md`=`299c68b3a5266ab4b1946f4b06caa9e4069274a9`、`docs/緩衝基盤_rev5補遺_QC/B0_不足証拠一覧.md`=`ff35b791d6a022f1879d3f0002ed0d219e89578b`、docs/QE_2026-11-13/E0_要求consumer台帳.md d58c09023e9b34568f13e032ad147633b06591e。 |
| 試験区分（`TestClass`） | DOC／HISTORY／LEDGER。全母集団の再走査・製品実行は本mappingでNOT_RUN。 |
| 証拠ID（`Evidence ID`） | `NONE`（製品Evidence IDを発行しない）。 |
| 阻害要因（`Blocker`） | E0 mappingはOPEN。Release blocker registryは変更せず、新規Blockerを作成しない。 |
| 状態 | `OPEN`（E0-04 row mappingは記載、R01の全件Acceptanceは未成立）。 |
| 残余 | 欠測run／jobと原因UNKNOWNを含む全source provenance、全原因の現行要求結合が残る。 |

#### BUF-R02 緩衝主体が成立可能な下位操作を実行

| E0-04項目 | 個別値 |
|---|---|
| 現ID | BUF-R02 |
| 旧ID | BUF-R02（識別子を維持） |
| 原文参照 | v4仕様S06、仕様原本SHA-256 8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF。補遺実装仕様書§1.2・§4は来歴資料。 |
| 対象 | failure報告で終わらず、既存の許可範囲にある互換変換・隔離・状態調停・実行接続再構成等を実行し、元のTool作用と結果へ戻す経路。 |
| 目的 | 成立可能な下位操作を緩衝主体が実行し、上位のTaskを本来の意味のまま完了または明示失敗へ導く。 |
| 生成元（`Producer`） | Runtime／OS／Toolchainからの実行要求、環境観測、原因候補、現在のBroker Task／Workspace束縛。 |
| 実Consumer | 現在はCodex CLI TaskのWorkspaceTaskScratch／TEMP・TMP bindingとBroker journal／Audit、開発用isolated Cargo／Flutter runner。汎用Bufferから許可Toolへ渡す本番Consumerは未接続。 |
| 契約／Data | 非権限のbuffer_intervention_record v2とsource／Task／Host／Workspace／input／Toolchain指紋・作用参照・結果証拠。recordは権限や成功を発行しない。 |
| 正常時作用 | 既存Brokerで適格性と許可を再確認した範囲だけ介入し、指定された元Toolを実行して作用後の結果を観測する。 |
| 否定時作用 | 未知・非対応・意味変更・未証明の提案は実行しない。記録作成やfailure reportだけで吸収成立と扱わない。 |
| 停止／Recovery | Brokerの既存deny／Task failure／suspendと、既存Task journal／Recovery境界を使う。隔離runner cleanupは製品Task Recoveryと混同しない。 |
| 標準OS候補 | Windows、macOS、Linux native、Android、iOS。現行作用のOS保証範囲は個別N08／N10／N11記録に限る。Linux nativeは必須製品対象。 |
| 配布物 | 5 OSのproduction packageはN10対応表へ委譲し、具体package／version／archはUNKNOWNなら確定しない。開発runnerは配布物ではない。 |
| 既存実装 | codex_cli.rsのTask scratchと環境binding、agent_task_scratch.rsのjournal／recovery、dialogue.rsのBroker照合・Audit・結果projection、toolingのisolated runners。 |
| BUILD_GAP | 一般Buffer介入の実作用、元Tool実行、前後の独立結果測定、原因別の安全な選択がproduction全体へ接続されていない。 |
| 参照元ハッシュ（`SourceSHA`） | v4仕様S06原本のSHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`。照合時Git commit=`5f1bb53`のGit blob: `native/rust_helper/src/adapters/codex_cli.rs`=`e5c9d1581ae9a1c0a3ae7fcfa626fdca6fea6d28`、`native/rust_helper/src/broker/agent_task_scratch.rs`=`de743cd819fb4533c9c3070c4e7bcae82e42d938`、`native/rust_helper/src/broker/dialogue.rs`=`c3c3fddb630dd06b7e6cc1c13b39f3d4f6f6a999`。 |
| 試験区分（`TestClass`） | CODE_PRESENT／UNIT／限定LIVE_RUNTIMEの過去証拠。今回の製品runtime testはNOT_RUN。 |
| 証拠ID（`Evidence ID`） | `NONE`。過去の限定Task証拠をR02の汎用Acceptanceへ昇格しない。 |
| 阻害要因（`Blocker`） | E0 mapping OPEN。既存FQ／Registryはそれぞれのscopeのまま維持し、新規Blockerを作成しない。 |
| 状態 | `OPEN`。 |
| 残余 | Generic Buffer→既存Broker許可Tool→元Tool結果までの実作用と、因果・副作用の観測が未成立。 |

#### BUF-R03 新しい言語・OS・依存・外部Toolへ汎用責任モデルを適用

| E0-04項目 | 個別値 |
|---|---|
| 現ID | BUF-R03 |
| 旧ID | BUF-R03（識別子を維持） |
| 原文参照 | v4仕様S06、仕様原本SHA-256 8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF。補遺実装仕様書§1.2・§5は来歴資料。 |
| 対象 | 新しい依存、ABI/API、言語、OS、Runtime、外部Toolが加わったときの共通責任とAdapter／Broker接続。 |
| 目的 | 既知エラーの個別ハードコードを避け、未知入力を含む新しい実行対象にも同じauthority・意味・証拠境界を適用する。 |
| 生成元（`Producer`） | OS／Runtime Adapter、dependency・Toolchain情報、元Taskの固定済み入力と現在の実行Host。 |
| 実Consumer | 限定Codex CLI adapter、tooling/run_cargo_isolated.py、tooling/flutter_workspace.pyが個別Consumer。全OS／言語／Tool共通のBuffer Consumerは確認できない。 |
| 契約／Data | Runtime中立のBroker／Adapter責任境界とbuffer_intervention_record v2。Vendor／Tool metadataからauthorityを生成しない。 |
| 正常時作用 | 対応済みConsumerは各自の既存Tool契約で処理し、共通記録には検証可能なscope・source・結果だけを結ぶ。 |
| 否定時作用 | 未知Runtime／未対応OS／不明ABIやTool契約を既知環境へ読み替えず、fallbackで成功扱いしない。 |
| 停止／Recovery | Adapterがない、証拠が不足、契約が不一致なら現在Brokerでdeny／suspendし、対象環境の未成立を記録する。 |
| 標準OS候補 | Windows、macOS、Linux native、Android、iOS。候補version／arch／ABIは実測前に固定しない。Linux nativeは完全対応必須。 |
| 配布物 | 5 OSごとの製品Installer／Package／更新／削除はN10に対応付ける。具体形式・保証値が確定していない箇所はUNKNOWN。WSL等はnative Linuxを代替しない。 |
| 既存実装 | codex_cli.rs、run_cargo_isolated.py、flutter_workspace.py、既存Broker contract/conformance。各々が限定scopeであり汎用cross-platform接続ではない。 |
| BUILD_GAP | 同じ責任モデルを適用する製品Adapter／Tool consumer、標準OSごとの導入・実行・配布経路と拡張conformanceが未完成。 |
| 参照元ハッシュ（`SourceSHA`） | v4仕様S06原本のSHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`。照合時Git commit=`5f1bb53`のGit blob: `native/rust_helper/src/adapters/codex_cli.rs`=`e5c9d1581ae9a1c0a3ae7fcfa626fdca6fea6d28`、`tooling/run_cargo_isolated.py`=`6890c89b384db2169b92a7326f04dbd2ea6341e1`、`tooling/flutter_workspace.py`=`dab1c73e2dad774fd7c8c385991a4d66a969ca19`。 |
| 試験区分（`TestClass`） | UNIT／CONFORMANCE／開発runnerの限定試験。5 OS製品適合・新Tool一般化は本mappingでNOT_RUN。 |
| 証拠ID（`Evidence ID`） | `NONE`。 |
| 阻害要因（`Blocker`） | E0 mapping OPEN。既存OS／release blockerを維持し、R03専用の新規登録は行わない。 |
| 状態 | `OPEN`。 |
| 残余 | 新OS・言語・依存・Toolに対する同一責任のproduction consumerと対象別配布証拠が不足。 |

#### BUF-R04 元要求・結果・権限・成功条件の意味を保持

| E0-04項目 | 個別値 |
|---|---|
| 現ID | BUF-R04 |
| 旧ID | BUF-R04（識別子を維持） |
| 原文参照 | v4仕様S06、仕様原本SHA-256 8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF。補遺実装仕様書§1.2・§6は来歴資料。 |
| 対象 | 調停前後の要求、権限範囲、元Tool結果、成功判定、状態差と副作用。 |
| 目的 | 同等性が証明できる作用だけを許し、意味を変える／証明できない変換を適用しない。 |
| 生成元（`Producer`） | 元Task要求・成功条件・source／input／Host／Workspace指紋、調停記録、作用後Tool結果。 |
| 実Consumer | dialogue.rsのBroker observation照合とTask終端投影、buffer_intervention_record schema／Conformance、将来の独立結果検査。独立readback consumerは未確認。 |
| 契約／Data | version 2の非権限recordと固定されたTool成功条件、作用参照、前後の実測結果・hash。Schemaがvalidでもruntimeの意味同等性を証明しない。 |
| 正常時作用 | 同一要求のまま作用を実行し、元Toolの実結果を独立に読み戻して要求・成功条件と照合する。 |
| 否定時作用 | planned hash、record内自己申告、Schema適合だけでは作用や成功を認めない。欠落・不一致時はTaskを成功にしない。 |
| 停止／Recovery | 意味・hash・Host／Workspace束縛の不一致では確定を拒否し、既存Broker failure／Audit／Recoveryへ接続する。 |
| 標準OS候補 | Windows、macOS、Linux native、Android、iOS。各候補の独立readback／成功条件は未測定範囲をUNKNOWNとする。 |
| 配布物 | 5 OSの実行候補と配布形態はN10対応表。形式・version／archが未確定ならUNKNOWN。 |
| 既存実装 | buffer_intervention_record.schema.json、conformance skeletonの正常／authority注入／未証明吸収negative、dialogue.rsの記録とprojection。 |
| BUILD_GAP | Rust生成recordのruntime Schema validation、予定値と独立実測の分離、元Tool結果のindependent readbackと意味比較が未成立。 |
| 参照元ハッシュ（`SourceSHA`） | v4仕様S06原本のSHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`。照合時Git commit=`5f1bb53`のGit blob: `specs/buffer_intervention_record.schema.json`=`26a1fa8ee9d010f18c157f5d3b66b9b34bdf94d5`、`native/rust_helper/src/broker/dialogue.rs`=`c3c3fddb630dd06b7e6cc1c13b39f3d4f6f6a999`、`tooling/conformance_tests/run_conformance_skeleton.py`=`c6f31d8cb561b48513c6784e54c85969c9cb953e`。 |
| 試験区分（`TestClass`） | CONFORMANCE／FIXTURE／UNITが存在。今回これらはNOT_RUN、独立実作用比較も未成立。 |
| 証拠ID（`Evidence ID`） | `NONE`。 |
| 阻害要因（`Blocker`） | E0 mapping OPEN。既存Blocker registryは無変更。 |
| 状態 | `OPEN`。 |
| 残余 | 実際のpre/post結果、意味同等性、状態差、副作用の独立比較が未確認。 |

#### BUF-R05 BrokerのAuthority・Permission・Approval・Audit・Recovery等を維持

| E0-04項目 | 個別値 |
|---|---|
| 現ID | BUF-R05 |
| 旧ID | BUF-R05（識別子を維持） |
| 原文参照 | v4仕様S06、仕様原本SHA-256 8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF。補遺実装仕様書§1.2・§7は来歴資料。 |
| 対象 | 既存Rust Brokerが所有するAuthority、Permission、Approval、Credential／secret、Audit、Recovery、Content Exposure、Host／Workspace境界。 |
| 目的 | Bufferは実行主体の権限源にならず、既存Brokerの許可・scope・結果・監査責任を維持する。 |
| 生成元（`Producer`） | UI／Agent／Adapter／Toolからの既存Task要求と非権限のBuffer record。 |
| 実Consumer | Rust Broker protocol／dialogue／Task scratch pathが限定的に再評価する。Flutterから新しいAuthority経路は作らない。全Buffer介入のBroker consumerは未接続。 |
| 契約／Data | 現BrokerのTask／Runtime／Host／Workspace束縛、Permission／一回Approval、Audit／Recovery、許可された内容投影。buffer recordはmetadata／evidenceのみで権限を持たない。 |
| 正常時作用 | 現在条件でBrokerが既存権限を再評価し、許可範囲の下位作用だけを実行し、Audit／Recovery／表示境界を保持する。 |
| 否定時作用 | UI／Adapter／Agent／MCP／A2A metadata、履歴、過去Approval、Buffer判定から権限を生成・再利用・昇格しない。 |
| 停止／Recovery | Permission／Approval不成立、scope不一致、Audit不能、secret境界違反はBrokerで拒否し、危険作用を停止する。既存Recoveryへ戻し、代替Authority経路を作らない。 |
| 標準OS候補 | Windows、macOS、Linux native、Android、iOS。共通Coreの存在を各OSのruntime保証へ昇格しない。 |
| 配布物 | 5 OSのnative Bridge／Broker packagingはN10およびOS別契約に従う。未確認のpackage／version／署名状態はUNKNOWN。 |
| 既存実装 | protocol.rs、dialogue.rs、agent_task_scratch.rs、buffer record schema。現行Codex Task pathの限定record・Audit・hash-only projection。 |
| BUILD_GAP | 全てのBuffer作用のBroker統治・permission／approval消費・secret／Content Exposure・Host／Workspace scope・failure Auditを全対象へ接続した実証がない。 |
| 参照元ハッシュ（`SourceSHA`） | v4仕様S06原本のSHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`。照合時Git commit=`5f1bb53`のGit blob: `native/rust_helper/src/broker/protocol.rs`=`9b2076d5f31ca6d77f1f87973d1b394aaf751961`、`native/rust_helper/src/broker/dialogue.rs`=`c3c3fddb630dd06b7e6cc1c13b39f3d4f6f6a999`、`native/rust_helper/src/broker/agent_task_scratch.rs`=`de743cd819fb4533c9c3070c4e7bcae82e42d938`、`specs/buffer_intervention_record.schema.json`=`26a1fa8ee9d010f18c157f5d3b66b9b34bdf94d5`。 |
| 試験区分（`TestClass`） | UNIT／CONFORMANCE／限定LIVE_RUNTIME履歴。全Buffer作用の実Broker negative・全OS確認は本mappingでNOT_RUN。 |
| 証拠ID（`Evidence ID`） | `NONE`。 |
| 阻害要因（`Blocker`） | E0 mapping OPEN。現行authority／OS／release blockerを保持し、新規Blockerを加えない。 |
| 状態 | `OPEN`。 |
| 残余 | 汎用Bufferの全作用を既存Brokerの権限・監査・内容露出・回復境界へ通すproduction経路は未成立。 |

#### BUF-R06 同原因の反復調査と人手介入を減らし障害を隠さない

| E0-04項目 | 個別値 |
|---|---|
| 現ID | BUF-R06 |
| 旧ID | BUF-R06（識別子を維持） |
| 原文参照 | v4仕様S06、仕様原本SHA-256 8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF。補遺実装仕様書§1.2・§8は来歴資料。 |
| 対象 | 同原因の調査反復、人手介入、場当たり的再試行と、それらのfailure／副作用の可視性。 |
| 目的 | 再利用可能な修正で反復負担を減らし、failure・根因不明・新しい副作用を隠さない。 |
| 生成元（`Producer`） | B0 cause row、同一条件のfailure観測、scope fingerprint、再実行・介入の時間／回数／人手記録。 |
| 実Consumer | B0/B3資料は原因整理の記録consumer。run_cargo_isolated.py等のrunnerは限定隔離consumer。一般のcause-reuse production consumerや工数計測consumerは未確認。 |
| 契約／Data | Cause identityと根拠class／scopeを結び、retry・recovery・result・副作用・所要時間を区別する。比較基準の計測契約は未実装。 |
| 正常時作用 | 同じ根拠とscopeで再利用できる修正だけを適用し、成功条件・副作用・人手介入の実測値を元Tool結果と共に残す。 |
| 否定時作用 | error類似だけ、retry成功だけ、assertion緩和だけで根因解消・負担削減を主張しない。 |
| 停止／Recovery | 原因のscope／証拠が違うか効果が確認できなければ再利用を止め、failure／UNKNOWNを保持し、既存復旧手順に戻す。 |
| 標準OS候補 | Windows、macOS、Linux native、Android、iOS。現行測定値がないOSの時間・人手効果はUNKNOWN。 |
| 配布物 | 5 OSのproduction candidateはN10表を参照。開発用runnerの再利用効果を製品配布物の保証へ転用しない。 |
| 既存実装 | B0原因台帳・不足証拠一覧、B3原因機構対応、isolated Cargo runnerとその限定test。効果比較データは未確認。 |
| BUILD_GAP | 同原因の再利用consumer、介入前後の調査回数／人手時間／成功・副作用の同条件比較が未成立。 |
| 参照元ハッシュ（`SourceSHA`） | v4仕様S06原本のSHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`。照合時Git commit=`5f1bb53`のGit blob: `docs/緩衝基盤_rev5補遺_QC/B0_原因対応台帳.md`=`299c68b3a5266ab4b1946f4b06caa9e4069274a9`、`docs/緩衝基盤_rev5補遺_QC/B3_全原因機構対応.md`=`8a25f3ddacada889ba95de1d4761af77a074d26b`、`tooling/run_cargo_isolated.py`=`6890c89b384db2169b92a7326f04dbd2ea6341e1`、`tooling/conformance_tests/test_run_cargo_isolated.py`=`b5a8c42b6c9fee9f9886b6dd3fb7e39f65530a99`。 |
| 試験区分（`TestClass`） | DOC／HISTORY／UNIT。介入効果・反復削減・人手削減の比較testはNOT_RUN／未確認。 |
| 証拠ID（`Evidence ID`） | `NONE`。 |
| 阻害要因（`Blocker`） | E0 mapping OPEN。新しいrelease blockerは作らず、既存Registryは無変更。 |
| 状態 | `OPEN`。 |
| 残余 | 因果効果や負担削減の定量値なし。ログ・失敗の可視性と副作用を含む一般consumerも未成立。 |

#### BUF-R07 D4Pocket自身の実開発・検査へ適用

| E0-04項目 | 個別値 |
|---|---|
| 現ID | BUF-R07 |
| 旧ID | BUF-R07（識別子を維持） |
| 原文参照 | v4仕様S06、仕様原本SHA-256 8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF。補遺実装仕様書§1.2・§9、B5自己適用記録は限定履歴。 |
| 対象 | D4Pocketのsource、build、test、artifact、cleanupを含む実際の開発・検査経路。 |
| 目的 | 同じsourceと条件で緩衝基盤を自己適用し、過去failureとの因果差・副作用・境界移転を検査する。 |
| 生成元（`Producer`） | D4 source commit、実際の開発／test request、OS・Host・Workspace・Toolchain入力。 |
| 実Consumer | tooling/run_cargo_isolated.py、tooling/flutter_workspace.py、launch_owner_desktop_isolated.py、限定されたB5記録とrunner test。 |
| 契約／Data | source commit／snapshot hash、固定入力・Host／Toolchain、実行command、元Tool result、生成物hash、state diff、cleanup状態を結合する。既存B5の各記録は同じ粒度・条件ではない。 |
| 正常時作用 | 対象sourceのisolated runを開始し、同条件で元Tool結果とcleanup／差分を観測できる。現在の限定runnerが持つ範囲だけを再利用する。 |
| 否定時作用 | source／入力／Host／Toolchain条件が一致しないrunを因果比較へ使わない。別OSのPASSを対象OS／製品の代替にしない。 |
| 停止／Recovery | snapshot／hash不一致またはcleanup失敗を成功扱いせず、隔離runを終了・回収できない範囲は未成立として保持する。共有cacheは別所有物として勝手に削除しない。 |
| 標準OS候補 | Windows、macOS、Linux native、Android、iOS。候補ごとの開発host実測と製品OS証拠を分ける。Linux nativeは必須。 |
| 配布物 | 5 OS product packageはN10に従う。isolated Cargo／Flutter runnerは開発用で配布物ではない。具体候補が未確定ならUNKNOWN。 |
| 既存実装 | run_cargo_isolated.py、flutter_workspace.pyのIsolatedFlutterWorkspace、isolated launcher、B5自己適用のWindows限定記録。 |
| BUILD_GAP | 全D4開発経路の接続、同一source・同条件の因果比較、全OSへの転移確認、Artifact／副作用／cleanupの統合記録が未成立。 |
| 参照元ハッシュ（`SourceSHA`） | v4仕様S06原本のSHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`。照合時Git commit=`5f1bb53`のGit blob: `tooling/run_cargo_isolated.py`=`6890c89b384db2169b92a7326f04dbd2ea6341e1`、`tooling/flutter_workspace.py`=`dab1c73e2dad774fd7c8c385991a4d66a969ca19`、`tooling/launch_owner_desktop_isolated.py`=`208f0fae05cceb40c52930d6c782acf179a60794`、`tooling/conformance_tests/test_run_cargo_isolated.py`=`b5a8c42b6c9fee9f9886b6dd3fb7e39f65530a99`、`docs/緩衝基盤_rev5補遺_QC/B5_自己適用.md`=`16081537b6699c176144e32ca97d6bdd20d98866`。 |
| 試験区分（`TestClass`） | UNIT／CONFORMANCE／限定Windows LIVE_RUNTIMEの過去記録。今回再実行せず、全B5 acceptanceは未成立。 |
| 証拠ID（`Evidence ID`） | `NONE`。B5の局所履歴は製品Evidence IDではない。 |
| 阻害要因（`Blocker`） | E0 mapping OPEN。既存B5／OS blocker状態を変更しない。 |
| 状態 | `OPEN`。 |
| 残余 | 本体の全開発・検査へ自己適用した同条件効果と、別OS／製品境界の独立検証が未完。 |

#### BUF-R08 構想から配布まで品質責任を連続させる

| E0-04項目 | 個別値 |
|---|---|
| 現ID | BUF-R08 |
| 旧ID | BUF-R08（識別子を維持） |
| 原文参照 | v4仕様S06、仕様原本SHA-256 8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF。補遺実装仕様書§1.2・§10は来歴資料。 |
| 対象 | 要求・設計・実装・試験・是正・配布を通じた継続的な品質責任と実Consumer結合。 |
| 目的 | QCを後段だけの活動にせず、要求から実作用・失敗・回復・OS packageまでの責任と証拠をつなぐ。 |
| 生成元（`Producer`） | 現行仕様／工程／指示、変更されたsource／contract、build／test／runtime結果、是正記録。 |
| 実Consumer | AGENTS.md、v4三点セット、E0 ledger／進捗が開発processを統治。manifest、conformance、release gate validatorは機械的な文書／契約consumerであり製品QC／QAではない。 |
| 契約／Data | QE_DESIGN→QE_BUILD→SYSTEM_SHAPED→BUILD_COVERED→QE_QC→QE_QA→QE_RELEASE_REVIEW、現行AcceptanceとSource→Consumer→Evidence追跡。 |
| 正常時作用 | 各工程で責任・受入条件・consumer・必要な検査・残余を更新し、BUILD完了とQC／QA／Release判断を別状態で扱う。 |
| 否定時作用 | 文書・schema・mock・validatorのPASSを実runtime／製品QA／配布成功へ読み替えない。未接続必須consumerをCLOSEDにしない。 |
| 停止／Recovery | 必須consumer・責任・要求の欠落をBUILD_GAPとして保持する。安全重要境界は作用時に必要なnegative／停止確認を行い、工程外のrelease gateはそのまま残す。 |
| 標準OS候補 | Windows、macOS、Linux native、Android、iOSを対象に、開発／QC／QAの証拠classを分離する。Linux nativeは必須。 |
| 配布物 | 全5 OSの正規package／installer／update／rollback／uninstallはN10に対応付ける。詳細仕様や候補が未確定ならUNKNOWNであり、文書上の宣言は完成証拠ではない。 |
| 既存実装 | AGENTS、v4仕様／工程／施工指示、E0 ledger／progress、manifest.py、run_conformance_skeleton.py、release_gate_check.py。 |
| BUILD_GAP | 文書上のQC過程を全製品Producer／ConsumerとOS別実作用・package・QAへ通したsystem evidenceは未成立。 |
| 参照元ハッシュ（`SourceSHA`） | v4仕様S06原本のSHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`。照合時Git commit=`5f1bb53`のGit blob: `AGENTS.md`=`1eafcadd129fd01f1f2f683594c381605ed7d48d`、`docs/QE_2026-11-13/01_D4Pocket_統合製品実装仕様書_v4.md`=`352c53180cbad07888beb956aad0fb2a58074c58`、`docs/QE_2026-11-13/02_D4Pocket_統合開発工程表_v4.md`=`d32dd251e5f7cbb97d92f6e7540d6b6ed0ecec05`、`docs/QE_2026-11-13/03_D4Pocket_Codex施工指示書_v4.md`=`49447973e88c50a09b2f4978adfed84fd464c6d0`、docs/QE_2026-11-13/E0_要求consumer台帳.md d58c09023e9b34568f13e032ad147633b06591e、`docs/QE_2026-11-13/進捗.md`=`0601b9c96a09eebd33a5b1513b8b483e9d1ba47e`、`tooling/manifest.py`=`aa4525ff20293cb8641e5598a2439561a3bfbd3b`、`tooling/conformance_tests/run_conformance_skeleton.py`=`c6f31d8cb561b48513c6784e54c85969c9cb953e`、`tooling/release_gate_check.py`=`22a8d7bfe9adb259f44ce8501fb215652e1d58c6`。 |
| 試験区分（`TestClass`） | DOC／MANIFEST／CONFORMANCE／RELEASE_GATE。これらのprocess checksは本mapping後に実行、Product runtime／五OS QAはNOT_RUN。 |
| 証拠ID（`Evidence ID`） | `NONE`。 |
| 阻害要因（`Blocker`） | E0 mapping OPEN。現行18件のRelease blockerとQ0〜Q7 DEFERREDを保持し、新規登録なし。 |
| 状態 | `OPEN`。 |
| 残余 | 全機能統合後のsystem QC、標準OS QA、正式Release Reviewは後続工程であり、現在の文書mappingで成立しない。 |

#### BUF-R09 不適合を上流の目的・設計・境界・検査方法へ帰還

| E0-04項目 | 個別値 |
|---|---|
| 現ID | BUF-R09 |
| 旧ID | BUF-R09（識別子を維持） |
| 原文参照 | v4仕様S06、仕様原本SHA-256 8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF。補遺実装仕様書§1.2・§11、B0／B1／B5限定記録は来歴。 |
| 対象 | 不適合のcause、上流目的／モデル／責任境界／設計／検査方法と是正後の影響。 |
| 目的 | 機械的retryや事後報告で閉じず、根因に応じ上流へ戻り、是正の副作用・別境界への移転も再観測する。 |
| 生成元（`Producer`） | Runtime／test failure、Audit・source／result evidence、欠測・反証、比較条件と過去cause row。 |
| 実Consumer | B0 causes／deficiency → B1設計QC → B2/B3/B4機構・接続記録 → B5自己適用と matched observation。製品共通の自動feedback consumerは未確認。 |
| 契約／Data | 直接観測・強い推定・UNKNOWNの区別、causeのsource／scope、要求と責任、仮説・反証、修正、同条件結果、副作用・移転状態。 |
| 正常時作用 | 根因に対応する上流条件・設計・実装／test責任へ戻し、同条件の再観測と別consumerへの影響を記録する。 |
| 否定時作用 | 別Host／source／成功条件でのretry PASS、ログの後付け、症状だけの変更でroot cause closureを主張しない。 |
| 停止／Recovery | 根因や転移が確定しない場合はUNKNOWN／OPENのまま維持し、作用が安全でない範囲を停止する。既存Broker recoveryと原因記録を分離しない。 |
| 標準OS候補 | Windows、macOS、Linux native、Android、iOS。根因・影響が観測されていないOSへ結果を転用しない。Linux nativeは必須。 |
| 配布物 | 全5 OSの候補とrelease evidenceはN10／N11へ結ぶ。具体形式／version／archが未確定ならUNKNOWN。 |
| 既存実装 | B0_原因対応台帳.md、B0_不足証拠一覧.md、B1_設計QC.md、B3_全原因機構対応.md、B5_自己適用.md。これらは範囲限定の記録であり汎用runtime feedback loopではない。 |
| BUILD_GAP | 原因recordから要求／design／implementation／test ownerへ製品全体で結び、matched post-fix observation・副作用・downstream transferを一体追跡するconsumerが未成立。 |
| 参照元ハッシュ（`SourceSHA`） | v4仕様S06原本のSHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`。照合時Git commit=`5f1bb53`のGit blob: `docs/緩衝基盤_rev5補遺_QC/B0_原因対応台帳.md`=`299c68b3a5266ab4b1946f4b06caa9e4069274a9`、`docs/緩衝基盤_rev5補遺_QC/B0_不足証拠一覧.md`=`ff35b791d6a022f1879d3f0002ed0d219e89578b`、`docs/緩衝基盤_rev5補遺_QC/B1_設計QC.md`=`aa113006eb7a108848211f4dfb3475dc0cfbc9d5`、`docs/緩衝基盤_rev5補遺_QC/B3_全原因機構対応.md`=`8a25f3ddacada889ba95de1d4761af77a074d26b`、`docs/緩衝基盤_rev5補遺_QC/B5_自己適用.md`=`16081537b6699c176144e32ca97d6bdd20d98866`。 |
| 試験区分（`TestClass`） | DOC／HISTORY／UNIT／限定LIVE_RUNTIME。cause update・matched effect・全consumer transferの統合testはNOT_RUN／未成立。 |
| 証拠ID（`Evidence ID`） | `NONE`。 |
| 阻害要因（`Blocker`） | E0 mapping OPEN。既存failure／release blocker statusを変えず、新規Blockerを作成しない。 |
| 状態 | `OPEN`。 |
| 残余 | 全原因の上流帰還と修正後の同条件因果観測、下流への障害移転監視が未完。 |

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

上段の33行はID別scope／Consumer／既存証拠境界の要約であり、BUF-R/T Acceptance、B0〜B6判定、E0完了を意味しない。BUF-R01〜R09とBUF-T01〜T24の個別E0-04 21 fieldは下記に記録した。限定source、開発runner、fixture、文書／Registry consumerを製品Bufferの一般保証と数えず、旧CLOSED／FAILとB0〜B6状態は変更しない。N08/N10/N11の全OS標準環境・Artifact・Evidence/Blocker結合は継続する。

### BUF-T01〜T24（E0-04全21 fieldの個別記録）

上段の24 ID summaryを、v4工程表R02のE0-04全21項目へ展開した。SourceSHAは変更開始前のmain `80a16a3a03a91da63fd6b2b7fa1f972369ae9cc6`で固定し、本台帳の今回編集後blobを自己参照しない。旧B5観測は元の条件内の履歴として保持。

#### BUF-T01 観測母集団・欠測・重複原因

| E0-04項目 | 個別値 |
|---|---|
| 現ID | BUF-T01 |
| 旧ID | BUF-T01（識別子を維持） |
| 原文参照 | v4仕様S06、原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`。旧補遺実装仕様§8.3は来歴資料。 |
| 対象 | 取得可能な故障／検査／介入履歴と欠測範囲。 |
| 目的 | 記録の出所・欠測を示し、事象／原因／対処を重複なく結ぶ。 |
| 生成元（`Producer`） | Actions run、B0履歴、source commit、test結果。 |
| 実Consumer | B0原因台帳／不足証拠一覧は限定的。全記録収集consumerなし。 |
| 契約／Data | run/cause/source参照、commit、取得可否。完全性schemaなし。 |
| 正常時作用 | 取得済み／不能を分け、重複runをcauseへ対応付ける。 |
| 否定時作用 | 欠測をPASS扱いせず、同runを独立causeとして水増ししない。 |
| 停止／Recovery | 参照欠落・不一致をUNKNOWN／OPENとして保持し完全判定を停止。 |
| 標準OS候補 | Windows／macOS／Linux native／Android／iOS。各OSの標準環境を別々に評価し、Linux nativeを必須対象として保持。今回OS実行なし。 |
| 配布物 | 各OSのN10 package／installer／update候補。形式・version・archが未固定のものはUNKNOWN。本source mappingは配布Acceptanceではない。 |
| 既存実装 | B0原因対応台帳・不足証拠一覧・参照原本一覧。 |
| BUILD_GAP | 全source列挙、重複cause validator、母集団閉包consumerが未成立。 |
| 参照元ハッシュ（`SourceSHA`） | 照合HEAD=`80a16a3a03a91da63fd6b2b7fa1f972369ae9cc6`。v4仕様原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`、v4仕様Git blob=`352c53180cbad07888beb956aad0fb2a58074c58`、v4採用記録Git blob=`f159c1aa6ea5bc6e80e0877d8068f7e44da4c4d3`、旧補遺実装仕様Git blob=`2d0e98fb0b58ceec65244fde6a8fbf8e27c49b7e`。`docs/緩衝基盤_rev5補遺_QC/B0_原因対応台帳.md`=`299c68b3a5266ab4b1946f4b06caa9e4069274a9`、`docs/緩衝基盤_rev5補遺_QC/B0_不足証拠一覧.md`=`ff35b791d6a022f1879d3f0002ed0d219e89578b`。Git blobはpathと対にした照合HEAD時点の識別値。 |
| 試験区分（`TestClass`） | 文書/historyのみ。本mapping testは`NOT_RUN`。 |
| 証拠ID（`Evidence ID`） | `NONE`。本E0 mappingから製品Acceptance evidenceを発行しない。 |
| 阻害要因（`Blocker`） | E0 mappingはOPEN。既存Registry／Release blockerを変更せず、新規Blockerを作らない。 |
| 状態 | `OPEN`。E0照合状態。旧statusとB工程状態は変更しない。 |
| 残余 | 旧B0限定CLOSED・取得数値・欠測を維持。全事象網羅はOPEN。 |

#### BUF-T02 版・ABI・dependency graphとlockfile

| E0-04項目 | 個別値 |
|---|---|
| 現ID | BUF-T02 |
| 旧ID | BUF-T02（識別子を維持） |
| 原文参照 | v4仕様S06、原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`。旧補遺実装仕様§8.3は来歴資料。 |
| 対象 | Cargo／Flutter toolchain・ABI・依存graph・lockfile。 |
| 目的 | 矛盾を隔離／拒否し、意図しない依存解決・lock変更を防ぐ。 |
| 生成元（`Producer`） | manifest/lock、toolchain、target/ABI、runner要求。 |
| 実Consumer | Cargo targetとFlutter source/cacheを隔離するrunner。fixtureは引数・exit・cleanupを検査。 |
| 契約／Data | Cargo/Flutter lockとtemporary-root規則。全commandへの`--locked`強制はない。 |
| 正常時作用 | 分離下で実行し元Tool exitを保持して一時物を回収。 |
| 否定時作用 | override/unsafe rootを拒否し、依存矛盾・lock差分を隠さない。 |
| 停止／Recovery | 隔離／cleanup失敗はerrorで停止しsource/lockを修復しない。 |
| 標準OS候補 | Windows／macOS／Linux native／Android／iOS。各OSの標準環境を別々に評価し、Linux nativeを必須対象として保持。今回OS実行なし。 |
| 配布物 | 各OSのN10 package／installer／update候補。形式・version・archが未固定のものはUNKNOWN。本source mappingは配布Acceptanceではない。 |
| 既存実装 | `run_cargo_isolated.py`、`flutter_workspace.py`、runnerの試験用`fixture`。 |
| BUILD_GAP | 全入口のgraph/ABI検出、lock不変性と5 OS package経路が未結合。 |
| 参照元ハッシュ（`SourceSHA`） | 照合HEAD=`80a16a3a03a91da63fd6b2b7fa1f972369ae9cc6`。v4仕様原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`、v4仕様Git blob=`352c53180cbad07888beb956aad0fb2a58074c58`、v4採用記録Git blob=`f159c1aa6ea5bc6e80e0877d8068f7e44da4c4d3`、旧補遺実装仕様Git blob=`2d0e98fb0b58ceec65244fde6a8fbf8e27c49b7e`。`tooling/run_cargo_isolated.py`=`6890c89b384db2169b92a7326f04dbd2ea6341e1`、`tooling/conformance_tests/test_run_cargo_isolated.py`=`b5a8c42b6c9fee9f9886b6dd3fb7e39f65530a99`、`tooling/flutter_workspace.py`=`dab1c73e2dad774fd7c8c385991a4d66a969ca19`。Git blobはpathと対にした照合HEAD時点の識別値。 |
| 試験区分（`TestClass`） | 既存source／fixtureあり。本mappingでの実行は`NOT_RUN`。 |
| 証拠ID（`Evidence ID`） | `NONE`。本E0 mappingから製品Acceptance evidenceを発行しない。 |
| 阻害要因（`Blocker`） | E0 mappingはOPEN。既存Registry／Release blockerを変更せず、新規Blockerを作らない。 |
| 状態 | `OPEN`。E0照合状態。旧statusとB工程状態は変更しない。 |
| 残余 | 局所runner機能を製品全体の依存隔離保証とみなさない。 |

#### BUF-T03 API／Schema往復と不一致時非適用

| E0-04項目 | 個別値 |
|---|---|
| 現ID | BUF-T03 |
| 旧ID | BUF-T03（識別子を維持） |
| 原文参照 | v4仕様S06、原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`。旧補遺実装仕様§8.3は来歴資料。 |
| 対象 | Buffer recordのSchema/API変換とruntime作用入力。 |
| 目的 | 往復で意味を保ち、不一致時に実作用を適用しない。 |
| 生成元（`Producer`） | 記録生成部、`JSON Schema`、`Broker`対話要求入力。 |
| 実Consumer | Schema checker／fixtureとRust dialogueは存在。runtime validation/readbackは未接続。 |
| 契約／Data | buffer record schemaとRust projection。Schema適合だけでは作用証拠でない。 |
| 正常時作用 | 独立readbackと意味/hash一致後に作用確定。 |
| 否定時作用 | 欠落・余剰・未知・hash差異では適用せず成功にしない。 |
| 停止／Recovery | runtime検証不能は未確定としBroker failure/Audit/Recoveryへ送る。 |
| 標準OS候補 | Windows／macOS／Linux native／Android／iOS。各OSの標準環境を別々に評価し、Linux nativeを必須対象として保持。今回OS実行なし。 |
| 配布物 | 各OSのN10 package／installer／update候補。形式・version・archが未固定のものはUNKNOWN。本source mappingは配布Acceptanceではない。 |
| 既存実装 | 記録Schema、`dialogue.rs`、`Conformance`のSchema用`fixture`。 |
| BUILD_GAP | 生成record runtime validation、独立readback、mismatch no-opが未成立。 |
| 参照元ハッシュ（`SourceSHA`） | 照合HEAD=`80a16a3a03a91da63fd6b2b7fa1f972369ae9cc6`。v4仕様原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`、v4仕様Git blob=`352c53180cbad07888beb956aad0fb2a58074c58`、v4採用記録Git blob=`f159c1aa6ea5bc6e80e0877d8068f7e44da4c4d3`、旧補遺実装仕様Git blob=`2d0e98fb0b58ceec65244fde6a8fbf8e27c49b7e`。`specs/buffer_intervention_record.schema.json`=`26a1fa8ee9d010f18c157f5d3b66b9b34bdf94d5`、`native/rust_helper/src/broker/dialogue.rs`=`c3c3fddb630dd06b7e6cc1c13b39f3d4f6f6a999`、`tooling/conformance_tests/run_conformance_skeleton.py`=`c6f31d8cb561b48513c6784e54c85969c9cb953e`。Git blobはpathと対にした照合HEAD時点の識別値。 |
| 試験区分（`TestClass`） | `CONTRACT`／`SOURCE`／`FIXTURE` sourceあり。runtime testは`NOT_RUN`。 |
| 証拠ID（`Evidence ID`） | `NONE`。本E0 mappingから製品Acceptance evidenceを発行しない。 |
| 阻害要因（`Blocker`） | E0 mappingはOPEN。既存Registry／Release blockerを変更せず、新規Blockerを作らない。 |
| 状態 | `OPEN`。E0照合状態。旧statusとB工程状態は変更しない。 |
| 残余 | static fixtureをruntime parityへ拡張しない。 |

#### BUF-T04 OneDrive／Flutter再現・隔離・source identity

| E0-04項目 | 個別値 |
|---|---|
| 現ID | BUF-T04 |
| 旧ID | BUF-T04（識別子を維持） |
| 原文参照 | v4仕様S06、原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`。旧補遺実装仕様§8.3は来歴資料。 |
| 対象 | OneDrive同期対象起因のFlutter失敗とisolated snapshot/cache。 |
| 目的 | 同一source/command/toolchainで再現し隔離効果とsourceを結ぶ。 |
| 生成元（`Producer`） | Flutter tree/commit、command、toolchain、OneDrive path/sync結果。 |
| 実Consumer | Flutter runnerはsnapshot/cache隔離。B5に限定history。SyncEngine終端consumerなし。 |
| 契約／Data | source SHA、runner fingerprint、cache/root、cleanup結果。 |
| 正常時作用 | source/commandを固定し隔離前後結果とcleanupを対応。 |
| 否定時作用 | retry/path移動だけで根因解消とせずsource違いを比較から除く。 |
| 停止／Recovery | sync/cleanup不明はUNKNOWN。元sourceを変更せず停止。 |
| 標準OS候補 | Windows／macOS／Linux native／Android／iOS。各OSの標準環境を別々に評価し、Linux nativeを必須対象として保持。今回OS実行なし。 |
| 配布物 | 各OSのN10 package／installer／update候補。形式・version・archが未固定のものはUNKNOWN。本source mappingは配布Acceptanceではない。 |
| 既存実装 | `flutter_workspace.py`、isolated runner、B5記録。 |
| BUILD_GAP | current candidate同条件比較とSyncEngine source/result結合が未成立。 |
| 参照元ハッシュ（`SourceSHA`） | 照合HEAD=`80a16a3a03a91da63fd6b2b7fa1f972369ae9cc6`。v4仕様原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`、v4仕様Git blob=`352c53180cbad07888beb956aad0fb2a58074c58`、v4採用記録Git blob=`f159c1aa6ea5bc6e80e0877d8068f7e44da4c4d3`、旧補遺実装仕様Git blob=`2d0e98fb0b58ceec65244fde6a8fbf8e27c49b7e`。`docs/緩衝基盤_rev5補遺_QC/B5_自己適用.md`=`16081537b6699c176144e32ca97d6bdd20d98866`、`tooling/flutter_workspace.py`=`dab1c73e2dad774fd7c8c385991a4d66a969ca19`、`tooling/run_cargo_isolated.py`=`6890c89b384db2169b92a7326f04dbd2ea6341e1`。Git blobはpathと対にした照合HEAD時点の識別値。 |
| 試験区分（`TestClass`） | 旧B5に限定Windows観測。本candidate実行は`NOT_RUN`。 |
| 証拠ID（`Evidence ID`） | `NONE`。本E0 mappingから製品Acceptance evidenceを発行しない。 |
| 阻害要因（`Blocker`） | E0 mappingはOPEN。既存Registry／Release blockerを変更せず、新規Blockerを作らない。 |
| 状態 | `OPEN`。E0照合状態。旧statusとB工程状態は変更しない。 |
| 残余 | OneDrive原因・転移・他OS効果はUNKNOWN。 |

#### BUF-T05 loopback／TLS間欠失敗

| E0-04項目 | 個別値 |
|---|---|
| 現ID | BUF-T05 |
| 旧ID | BUF-T05（識別子を維持） |
| 原文参照 | v4仕様S06、原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`。旧補遺実装仕様§8.3は来歴資料。 |
| 対象 | localhost TCP/TLS fixture failure、RST capture/parserと初期条件。 |
| 目的 | 通信方向・条件を区別しretry PASSを根因閉鎖にしない。 |
| 生成元（`Producer`） | `Rust`用`fixture`、`Pktmon`捕捉／解析、workflow要約、B5記録。 |
| 実Consumer | probe→parser/summary→B5。原因process特定consumerなし。 |
| 契約／Data | パケット事象／tuple、run／source識別値、失敗記録。 |
| 正常時作用 | 識別済みtuple/方向/process情報だけcauseへ結ぶ。 |
| 否定時作用 | 空parse・event数・同probe retryをcause解決としない。 |
| 停止／Recovery | capture/parser/permission失敗はUNKNOWN。network policyを迂回せず停止。 |
| 標準OS候補 | Windows／macOS／Linux native／Android／iOS。各OSの標準環境を別々に評価し、Linux nativeを必須対象として保持。今回OS実行なし。 |
| 配布物 | 各OSのN10 package／installer／update候補。形式・version・archが未固定のものはUNKNOWN。本source mappingは配布Acceptanceではない。 |
| 既存実装 | Pktmon probe/parser/test、B5 #46–#55履歴。FQ保持。 |
| BUILD_GAP | RST方向・発生主体・既往failureとの因果join未成立。 |
| 参照元ハッシュ（`SourceSHA`） | 照合HEAD=`80a16a3a03a91da63fd6b2b7fa1f972369ae9cc6`。v4仕様原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`、v4仕様Git blob=`352c53180cbad07888beb956aad0fb2a58074c58`、v4採用記録Git blob=`f159c1aa6ea5bc6e80e0877d8068f7e44da4c4d3`、旧補遺実装仕様Git blob=`2d0e98fb0b58ceec65244fde6a8fbf8e27c49b7e`。`docs/緩衝基盤_rev5補遺_QC/B5_自己適用.md`=`16081537b6699c176144e32ca97d6bdd20d98866`、`tooling/probe_loopback_rst.ps1`=`490ea8bd71469a8c5de735d92f2ad6127d38df9a`、`tooling/pktmon_packet_summary.ps1`=`e0f982ddd20ac44b768a3c4455bc2fc6b6de1682`、`tooling/test_pktmon_packet_summary.ps1`=`a5b0dc6ae7d45845012be47e221df8d2b727f217`。Git blobはpathと対にした照合HEAD時点の識別値。 |
| 試験区分（`TestClass`） | parser source／fixtureあり。旧限定Windows観測のみ。現candidate実行は`NOT_RUN`。 |
| 証拠ID（`Evidence ID`） | `NONE`。本E0 mappingから製品Acceptance evidenceを発行しない。 |
| 阻害要因（`Blocker`） | E0 mappingはOPEN。既存`FQ-TEST-LOOPBACK`／B5状態を保持し、新規Blockerを作らない。 |
| 状態 | `OPEN`。E0照合状態。旧statusとB工程状態は変更しない。 |
| 残余 | 方向／原因UNKNOWN。`FQ-TEST-LOOPBACK`とB5 OPEN。 |

#### BUF-T06 Smart App Control／OS署名拒否

| E0-04項目 | 個別値 |
|---|---|
| 現ID | BUF-T06 |
| 旧ID | BUF-T06（識別子を維持） |
| 原文参照 | v4仕様S06、原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`。旧補遺実装仕様§8.3は来歴資料。 |
| 対象 | Windows SAC/Code Integrityと各OS署名/sandbox拒否。 |
| 目的 | OS保護を迂回せず拒否を観測し正規経路のみ使う。 |
| 生成元（`Producer`） | OS policy/event、生成binary、署名/package identity、通常build。 |
| 実Consumer | 通常runnerとB5限定Windows記録。全policy inventory・他OS証拠なし。 |
| 契約／Data | OS固有拒否recordとartifact/source identity。例外allowlistなし。 |
| 正常時作用 | policyとartifactを結び正規署名経路だけ実行。 |
| 否定時作用 | 保護無効化・署名偽装・個別許可・権限変更をしない。 |
| 停止／Recovery | 拒否原因不明・inventory不可はUNKNOWNとしartifact実行停止。 |
| 標準OS候補 | Windows／macOS／Linux native／Android／iOS。各OSの標準環境を別々に評価し、Linux nativeを必須対象として保持。今回OS実行なし。 |
| 配布物 | 各OSのN10 package／installer／update候補。形式・version・archが未固定のものはUNKNOWN。本source mappingは配布Acceptanceではない。 |
| 既存実装 | isolated Cargo runnerとB5非管理者check／CiTool拒否履歴。 |
| BUILD_GAP | policy/event cause joinと5 OS署名/sandbox統合検査が未成立。 |
| 参照元ハッシュ（`SourceSHA`） | 照合HEAD=`80a16a3a03a91da63fd6b2b7fa1f972369ae9cc6`。v4仕様原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`、v4仕様Git blob=`352c53180cbad07888beb956aad0fb2a58074c58`、v4採用記録Git blob=`f159c1aa6ea5bc6e80e0877d8068f7e44da4c4d3`、旧補遺実装仕様Git blob=`2d0e98fb0b58ceec65244fde6a8fbf8e27c49b7e`。`docs/緩衝基盤_rev5補遺_QC/B5_自己適用.md`=`16081537b6699c176144e32ca97d6bdd20d98866`、`tooling/run_cargo_isolated.py`=`6890c89b384db2169b92a7326f04dbd2ea6341e1`、`tooling/conformance_tests/test_run_cargo_isolated.py`=`b5a8c42b6c9fee9f9886b6dd3fb7e39f65530a99`。Git blobはpathと対にした照合HEAD時点の識別値。 |
| 試験区分（`TestClass`） | 旧Windows限定`LIVE_RUNTIME`相当履歴あり。現candidate／他OSは`NOT_RUN`。 |
| 証拠ID（`Evidence ID`） | `NONE`。本E0 mappingから製品Acceptance evidenceを発行しない。 |
| 阻害要因（`Blocker`） | E0 mappingはOPEN。既存Registry／Release blockerを変更せず、新規Blockerを作らない。 |
| 状態 | `OPEN`。E0照合状態。旧statusとB工程状態は変更しない。 |
| 残余 | 各OS条件を転用せずOS保護を弱めない。 |

#### BUF-T07 作業領域／`Host` rootの`secret`・`alias`・`hardlink`・`TOCTOU`逸脱拒否

| E0-04項目 | 個別値 |
|---|---|
| 現ID | BUF-T07 |
| 旧ID | BUF-T07（識別子を維持） |
| 原文参照 | v4仕様S06、原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`。旧補遺実装仕様§8.3は来歴資料。 |
| 対象 | 登録rootとAgent Taskのfilesystem path解決・書込み。 |
| 目的 | root外・secret・alias・hardlink・競合置換による逸脱をBrokerが拒否。 |
| 生成元（`Producer`） | Workspace/Host登録、relative path、filesystem identity。 |
| 実Consumer | `workspace_root.rs`、Task scratch、Workspace protocolのBroker経路。 |
| 契約／Data | root binding、path identity、task scratch、Permission/Audit。test scopeは限定。 |
| 正常時作用 | 現在root/Permission内のみ作用しTaskへ束縛。 |
| 否定時作用 | outside/secret/parent/alias/hardlink/reparse/TOCTOU不一致を拒否。 |
| 停止／Recovery | identity/ancestry再検査失敗はfail-closedし書込停止・Audit。 |
| 標準OS候補 | Windows／macOS／Linux native／Android／iOS。各OSの標準環境を別々に評価し、Linux nativeを必須対象として保持。今回OS実行なし。 |
| 配布物 | 各OSのN10 package／installer／update候補。形式・version・archが未固定のものはUNKNOWN。本source mappingは配布Acceptanceではない。 |
| 既存実装 | workspace root/scratch RustとWorkspace protocol/fixture tests。 |
| BUILD_GAP | 全攻撃条件installed mediationと5 OS evidenceが未成立。 |
| 参照元ハッシュ（`SourceSHA`） | 照合HEAD=`80a16a3a03a91da63fd6b2b7fa1f972369ae9cc6`。v4仕様原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`、v4仕様Git blob=`352c53180cbad07888beb956aad0fb2a58074c58`、v4採用記録Git blob=`f159c1aa6ea5bc6e80e0877d8068f7e44da4c4d3`、旧補遺実装仕様Git blob=`2d0e98fb0b58ceec65244fde6a8fbf8e27c49b7e`。`native/rust_helper/src/broker/workspace_root.rs`=`15f208e0b855b8cee53e878d2b27aaf457ea2937`、`native/rust_helper/src/broker/agent_task_scratch.rs`=`de743cd819fb4533c9c3070c4e7bcae82e42d938`、`native/rust_helper/tests/unit/workspace_protocol.rs`=`589a168fc63df02f8f27bc2d9c7cb158ee91408a`、`native/rust_helper/tests/unit/agent_task_fixture_fs.rs`=`d19ac79583b98bec9b828d6e064fe832af95bfa5`。Git blobはpathと対にした照合HEAD時点の識別値。 |
| 試験区分（`TestClass`） | 既存source／fixtureあり。本mapping testは`NOT_RUN`。 |
| 証拠ID（`Evidence ID`） | `NONE`。本E0 mappingから製品Acceptance evidenceを発行しない。 |
| 阻害要因（`Blocker`） | E0 mappingはOPEN。既存Registry／Release blockerを変更せず、新規Blockerを作らない。 |
| 状態 | `OPEN`。E0照合状態。旧statusとB工程状態は変更しない。 |
| 残余 | OS別path保証を他OSのsource testから推論しない。 |

#### BUF-T08 Permission／Approval／Audit／Credential不足時deny

| E0-04項目 | 個別値 |
|---|---|
| 現ID | BUF-T08 |
| 旧ID | BUF-T08（識別子を維持） |
| 原文参照 | v4仕様S06、原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`。旧補遺実装仕様§8.3は来歴資料。 |
| 対象 | Task/Tool要求と現在のPermission・一回Approval・Audit・Credential。 |
| 目的 | 不足・失効・不一致時作用前拒否しBuffer metadataからAuthorityを作らない。 |
| 生成元（`Producer`） | Owner要求、Permission/Approval record、Audit状態、Credential reference。 |
| 実Consumer | Rust Broker authority/dialogue/protocolとAgent Task operation。 |
| 契約／Data | Task/Workspace/Host束縛のPermission/Approval/Audit。secret非表示。 |
| 正常時作用 | 現在条件成立時のみ一回作用し結果/Auditを返す。 |
| 否定時作用 | 欠落/期限切れ/scope違いはdeny。履歴/Profile/Agent metadataで再利用しない。 |
| 停止／Recovery | Audit不能/secret境界違反はfail-closed、process停止・Recovery。 |
| 標準OS候補 | Windows／macOS／Linux native／Android／iOS。各OSの標準環境を別々に評価し、Linux nativeを必須対象として保持。今回OS実行なし。 |
| 配布物 | 各OSのN10 package／installer／update候補。形式・version・archが未固定のものはUNKNOWN。本source mappingは配布Acceptanceではない。 |
| 既存実装 | `authority.rs`、`dialogue.rs`、`Agent Task`製品経路試験source。 |
| BUILD_GAP | 全欠落状態のinstalled製品・5 OS一体検証なし。 |
| 参照元ハッシュ（`SourceSHA`） | 照合HEAD=`80a16a3a03a91da63fd6b2b7fa1f972369ae9cc6`。v4仕様原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`、v4仕様Git blob=`352c53180cbad07888beb956aad0fb2a58074c58`、v4採用記録Git blob=`f159c1aa6ea5bc6e80e0877d8068f7e44da4c4d3`、旧補遺実装仕様Git blob=`2d0e98fb0b58ceec65244fde6a8fbf8e27c49b7e`。`native/rust_helper/src/broker/authority.rs`=`067a36442138897aac396bdb588c2e01f90a5318`、`native/rust_helper/src/broker/dialogue.rs`=`c3c3fddb630dd06b7e6cc1c13b39f3d4f6f6a999`、`native/rust_helper/tests/agent_task_production_e2e.rs`=`fdc7565aa58fa7466e6e24e5492321e8015567eb`。Git blobはpathと対にした照合HEAD時点の識別値。 |
| 試験区分（`TestClass`） | 既存source／fixtureあり。現candidate実行は`NOT_RUN`。 |
| 証拠ID（`Evidence ID`） | `NONE`。本E0 mappingから製品Acceptance evidenceを発行しない。 |
| 阻害要因（`Blocker`） | E0 mappingはOPEN。既存Registry／Release blockerを変更せず、新規Blockerを作らない。 |
| 状態 | `OPEN`。E0照合状態。旧statusとB工程状態は変更しない。 |
| 残余 | broker境界を保持。全状態runtime evidenceはOPEN。 |

#### BUF-T09 crash／cancel／deadline後の子孫・scratch回収

| E0-04項目 | 個別値 |
|---|---|
| 現ID | BUF-T09 |
| 旧ID | BUF-T09（識別子を維持） |
| 原文参照 | v4仕様S06、原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`。旧補遺実装仕様§8.3は来歴資料。 |
| 対象 | 稼働中`Task`のプロセス系統、`Broker`回復記録、`WorkspaceTaskScratch`。 |
| 目的 | 中断後子孫停止・scratch回収・Recovery・Approval非再利用を一体確認。 |
| 生成元（`Producer`） | `Task`状態、期限切れ／取消／異常終了事象、PID系統、scratch回復記録。 |
| 実Consumer | `Broker`のプロセス監督・停止機構、Task scratchの回復処理。 |
| 契約／Data | Task/process identity束縛のscratch schema、terminal state/Audit。OS保証別。 |
| 正常時作用 | terminal state、停止確認、scratch cleanup、Recovery/Auditを束ねる。 |
| 否定時作用 | 子孫残存・cleanup失敗・stale Approvalをsuccess扱いしない。 |
| 停止／Recovery | 再起動journalから隔離回収。不明processは再開せず停止。 |
| 標準OS候補 | Windows／macOS／Linux native／Android／iOS。各OSの標準環境を別々に評価し、Linux nativeを必須対象として保持。今回OS実行なし。 |
| 配布物 | 各OSのN10 package／installer／update候補。形式・version・archが未固定のものはUNKNOWN。本source mappingは配布Acceptanceではない。 |
| 既存実装 | scratch実装、crash recovery e2e source（Windows実Codex一部ignore）、process supervision。 |
| BUILD_GAP | 5 OS installed containmentと同一candidate cleanup evidenceなし。 |
| 参照元ハッシュ（`SourceSHA`） | 照合HEAD=`80a16a3a03a91da63fd6b2b7fa1f972369ae9cc6`。v4仕様原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`、v4仕様Git blob=`352c53180cbad07888beb956aad0fb2a58074c58`、v4採用記録Git blob=`f159c1aa6ea5bc6e80e0877d8068f7e44da4c4d3`、旧補遺実装仕様Git blob=`2d0e98fb0b58ceec65244fde6a8fbf8e27c49b7e`。`native/rust_helper/src/broker/agent_task_scratch.rs`=`de743cd819fb4533c9c3070c4e7bcae82e42d938`、`native/rust_helper/tests/agent_task_crash_recovery_e2e.rs`=`23517fcfe8ef94b2357a94e04879d423c3360501`、`native/process_supervision/src/lib.rs`=`4887f42eba04bc0e067c76638ceaf647293ec452`、`specs/agent_task_scratch_recovery.schema.json`=`40f1eac71cd9645396ae7230d49cad25f3b77237`。Git blobはpathと対にした照合HEAD時点の識別値。 |
| 試験区分（`TestClass`） | `SOURCE`／`CONTRACT`／`FIXTURE`あり。実行・installed runtimeは`NOT_RUN`。 |
| 証拠ID（`Evidence ID`） | `NONE`。本E0 mappingから製品Acceptance evidenceを発行しない。 |
| 阻害要因（`Blocker`） | E0 mappingはOPEN。既存Registry／Release blockerを変更せず、新規Blockerを作らない。 |
| 状態 | `OPEN`。E0照合状態。旧statusとB工程状態は変更しない。 |
| 残余 | source testをinstalled cleanupへ昇格しない。 |

#### BUF-T10 Task／並行Task／Host間の非汚染

| E0-04項目 | 個別値 |
|---|---|
| 現ID | BUF-T10 |
| 旧ID | BUF-T10（識別子を維持） |
| 原文参照 | v4仕様S06、原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`。旧補遺実装仕様§8.3は来歴資料。 |
| 対象 | 異なるTask/Agent/Hostのscratch、state/cache、Credential/Permission/result。 |
| 目的 | 各境界を保ち別Taskへstate・成果・Authorityを漏らさない。 |
| 生成元（`Producer`） | `Task`／`Agent`／`Host`識別子、`Workspace`束縛、Task別scratch、Tool実行環境。 |
| 実Consumer | Codex adapter／Broker scratchとCompare/result consumers。 |
| 契約／Data | 独立Task/session/Workspace/Permission/Approval/scratch。shared cacheは明示scope。 |
| 正常時作用 | 成果を隔離先へ保存し、選択された成果だけ明示適用。 |
| 否定時作用 | 並行実行やmetadata/cacheから別Task内容・Authorityを継承しない。 |
| 停止／Recovery | cross-write時該当Task停止、相手state保持、Auditし再利用拒否。 |
| 標準OS候補 | Windows／macOS／Linux native／Android／iOS。各OSの標準環境を別々に評価し、Linux nativeを必須対象として保持。今回OS実行なし。 |
| 配布物 | 各OSのN10 package／installer／update候補。形式・version・archが未固定のものはUNKNOWN。本source mappingは配布Acceptanceではない。 |
| 既存実装 | `Codex` adapter、Task別scratch、`Agent Task`製品経路試験source。 |
| BUILD_GAP | 導入済み製品で複数Hostを並行利用した場合の内容混入・失敗分離を示す証拠がない。 |
| 参照元ハッシュ（`SourceSHA`） | 照合HEAD=`80a16a3a03a91da63fd6b2b7fa1f972369ae9cc6`。v4仕様原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`、v4仕様Git blob=`352c53180cbad07888beb956aad0fb2a58074c58`、v4採用記録Git blob=`f159c1aa6ea5bc6e80e0877d8068f7e44da4c4d3`、旧補遺実装仕様Git blob=`2d0e98fb0b58ceec65244fde6a8fbf8e27c49b7e`。`native/rust_helper/src/adapters/codex_cli.rs`=`e5c9d1581ae9a1c0a3ae7fcfa626fdca6fea6d28`、`native/rust_helper/src/broker/agent_task_scratch.rs`=`de743cd819fb4533c9c3070c4e7bcae82e42d938`、`native/rust_helper/tests/agent_task_production_e2e.rs`=`fdc7565aa58fa7466e6e24e5492321e8015567eb`。Git blobはpathと対にした照合HEAD時点の識別値。 |
| 試験区分（`TestClass`） | 既存source／fixtureあり。installed parallel testは`NOT_RUN`。 |
| 証拠ID（`Evidence ID`） | `NONE`。本E0 mappingから製品Acceptance evidenceを発行しない。 |
| 阻害要因（`Blocker`） | E0 mappingはOPEN。既存Registry／Release blockerを変更せず、新規Blockerを作らない。 |
| 状態 | `OPEN`。E0照合状態。旧statusとB工程状態は変更しない。 |
| 残余 | 局所task-bound設計と製品全体非汚染を区別。 |

#### BUF-T11 cache／lock／partial／corrupt state回復

| E0-04項目 | 個別値 |
|---|---|
| 現ID | BUF-T11 |
| 旧ID | BUF-T11（識別子を維持） |
| 原文参照 | v4仕様S06、原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`。旧補遺実装仕様§8.3は来歴資料。 |
| 対象 | 開発cache/lock、partial/corrupt update artifactとinstall state。 |
| 目的 | 共有cacheとTask出力を分け、破損から安全に復旧し成功へ誤昇格しない。 |
| 生成元（`Producer`） | `Cargo`／`Flutter`の`cache`、更新物の`payload`／`hash`／`state`、中断／失敗事象。 |
| 実Consumer | isolated dev runnerとLinux update verifierに局所consumer。汎用recoveryなし。 |
| 契約／Data | `cache`範囲、package hash、更新state遷移、部分生成物の回収。 |
| 正常時作用 | 検証artifactのみstage/applyしcache scopeと結果を再確認。 |
| 否定時作用 | 破損/部分/lock conflictを適用せず他Task/projectへ伝播させない。 |
| 停止／Recovery | stage隔離、稼働版/lock/source保持。不能時はerror。 |
| 標準OS候補 | Windows／macOS／Linux native／Android／iOS。各OSの標準環境を別々に評価し、Linux nativeを必須対象として保持。今回OS実行なし。 |
| 配布物 | 各OSのN10 package／installer／update候補。形式・version・archが未固定のものはUNKNOWN。本source mappingは配布Acceptanceではない。 |
| 既存実装 | `Flutter`／`Cargo`隔離runner、`Broker`更新module、Linux更新用`fixture`／specification。 |
| BUILD_GAP | 全OS cache分類・corrupt rollback・cross-project isolation・installed recovery未結合。 |
| 参照元ハッシュ（`SourceSHA`） | 照合HEAD=`80a16a3a03a91da63fd6b2b7fa1f972369ae9cc6`。v4仕様原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`、v4仕様Git blob=`352c53180cbad07888beb956aad0fb2a58074c58`、v4採用記録Git blob=`f159c1aa6ea5bc6e80e0877d8068f7e44da4c4d3`、旧補遺実装仕様Git blob=`2d0e98fb0b58ceec65244fde6a8fbf8e27c49b7e`。`tooling/flutter_workspace.py`=`dab1c73e2dad774fd7c8c385991a4d66a969ca19`、`native/rust_helper/src/broker/update_center.rs`=`f6fc98155fcd476765b60d382a680c7f7c6d675e`、`docs/specs/update-center.md`=`be37e1f661cb3e4cb05e7c1242051263665a92b2`、`tooling/tests/test_linux_update_catalog_fixture.py`=`a841f259911eefe99f4e474e18c57aae91cde994`。Git blobはpathと対にした照合HEAD時点の識別値。 |
| 試験区分（`TestClass`） | source／contract／fixtureあり。統合corrupt recoveryは`NOT_RUN`。 |
| 証拠ID（`Evidence ID`） | `NONE`。本E0 mappingから製品Acceptance evidenceを発行しない。 |
| 阻害要因（`Blocker`） | E0 mappingはOPEN。既存Registry／Release blockerを変更せず、新規Blockerを作らない。 |
| 状態 | `OPEN`。E0照合状態。旧statusとB工程状態は変更しない。 |
| 残余 | 開発cache隔離をproduct update rollbackと同一視しない。 |

#### BUF-T12 Tool異常exit・log保持・意味変更拒否

| E0-04項目 | 個別値 |
|---|---|
| 現ID | BUF-T12 |
| 旧ID | BUF-T12（識別子を維持） |
| 原文参照 | v4仕様S06、原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`。旧補遺実装仕様§8.3は来歴資料。 |
| 対象 | 実`Tool`の終了値／標準出力／標準エラーと、Agent Task結果の投影。 |
| 目的 | 異常を隠さず元Toolの意味・exit・failureを後続へ伝える。 |
| 生成元（`Producer`） | `Codex CLI` process、Adapter応答解析、`Broker`対話処理。 |
| 実Consumer | Codex adapter→dialogue/Task resultとfixture source。 |
| 契約／Data | 終了値／状態、上限付き出力／error、終端結果／Audit、secret除外。 |
| 正常時作用 | failure code/detailを保持し後続AI/UIへbounded投影。 |
| 否定時作用 | failureをsuccess化、logを隠蔽、secret露出で完了を演出しない。 |
| 停止／Recovery | 不正応答/crash/parse failureはfailed/unknown。再実行は新Approval。 |
| 標準OS候補 | Windows／macOS／Linux native／Android／iOS。各OSの標準環境を別々に評価し、Linux nativeを必須対象として保持。今回OS実行なし。 |
| 配布物 | 各OSのN10 package／installer／update候補。形式・version・archが未固定のものはUNKNOWN。本source mappingは配布Acceptanceではない。 |
| 既存実装 | `codex_cli.rs`、`dialogue.rs`、Codex用`fixture`／fake CLI。 |
| BUILD_GAP | 実Codex failureからUI/後続AIへの意味・secret/Audit統合証拠なし。 |
| 参照元ハッシュ（`SourceSHA`） | 照合HEAD=`80a16a3a03a91da63fd6b2b7fa1f972369ae9cc6`。v4仕様原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`、v4仕様Git blob=`352c53180cbad07888beb956aad0fb2a58074c58`、v4採用記録Git blob=`f159c1aa6ea5bc6e80e0877d8068f7e44da4c4d3`、旧補遺実装仕様Git blob=`2d0e98fb0b58ceec65244fde6a8fbf8e27c49b7e`。`native/rust_helper/src/adapters/codex_cli.rs`=`e5c9d1581ae9a1c0a3ae7fcfa626fdca6fea6d28`、`native/rust_helper/src/broker/dialogue.rs`=`c3c3fddb630dd06b7e6cc1c13b39f3d4f6f6a999`、`native/rust_helper/tests/unit/codex_cli_fixture.rs`=`40e5750bf267ccf9e583971e0d2ba4e6125d9a5d`。Git blobはpathと対にした照合HEAD時点の識別値。 |
| 試験区分（`TestClass`） | 既存source／fixtureあり。実Tool failure chainは`NOT_RUN`。 |
| 証拠ID（`Evidence ID`） | `NONE`。本E0 mappingから製品Acceptance evidenceを発行しない。 |
| 阻害要因（`Blocker`） | E0 mappingはOPEN。既存Registry／Release blockerを変更せず、新規Blockerを作らない。 |
| 状態 | `OPEN`。E0照合状態。旧statusとB工程状態は変更しない。 |
| 残余 | fixture結果を実Codex end-to-end保証へ拡張しない。 |

#### BUF-T13 build/testとinstalled製品・OS evidence class分離

| E0-04項目 | 個別値 |
|---|---|
| 現ID | BUF-T13 |
| 旧ID | BUF-T13（識別子を維持） |
| 原文参照 | v4仕様S06、原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`。旧補遺実装仕様§8.3は来歴資料。 |
| 対象 | build/test、artifact、installed product、OS別evidence。 |
| 目的 | 証明scopeを分類しbuild PASSをinstalled successへ読み替えない。 |
| 生成元（`Producer`） | `Manifest`、証拠bundle、OS検査器、candidate／artifact識別値。 |
| 実Consumer | manifest/evidence/release validatorsとN11 registryが文書/hash/gateを部分確認。 |
| 契約／Data | `SourceSHA`／`Contract`／`ArtifactSHA`／`OS-version-arch`／`environment`／`TestClass`／`command`／`exit`／`Evidence ID`。 |
| 正常時作用 | evidenceをcandidateへ結びvalidatorは確認scopeだけ通す。 |
| 否定時作用 | fixture/static checkをinstalled product/署名/release readinessへ昇格しない。 |
| 停止／Recovery | identity/hash/evidence欠落はgateでdenyし再収集へ。 |
| 標準OS候補 | Windows／macOS／Linux native／Android／iOS。各OSの標準環境を別々に評価し、Linux nativeを必須対象として保持。今回OS実行なし。 |
| 配布物 | 各OSのN10 package／installer／update候補。形式・version・archが未固定のものはUNKNOWN。本source mappingは配布Acceptanceではない。 |
| 既存実装 | `manifest.py`、`evidence_bundle.py`、`release_gate_check.py`、結果schema。 |
| BUILD_GAP | 要求・OS artifact・installed action・後続resultの一体bundle未成立。 |
| 参照元ハッシュ（`SourceSHA`） | 照合HEAD=`80a16a3a03a91da63fd6b2b7fa1f972369ae9cc6`。v4仕様原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`、v4仕様Git blob=`352c53180cbad07888beb956aad0fb2a58074c58`、v4採用記録Git blob=`f159c1aa6ea5bc6e80e0877d8068f7e44da4c4d3`、旧補遺実装仕様Git blob=`2d0e98fb0b58ceec65244fde6a8fbf8e27c49b7e`。`tooling/manifest.py`=`aa4525ff20293cb8641e5598a2439561a3bfbd3b`、`tooling/evidence_bundle.py`=`68dead851236b0f1b69899151c37371e593ff3f9`、`tooling/release_gate_check.py`=`22a8d7bfe9adb259f44ce8501fb215652e1d58c6`、`specs/runtime_result_evidence.schema.json`=`0844d38d864c9e556631a4a41cc75de13a99c40e`。Git blobはpathと対にした照合HEAD時点の識別値。 |
| 試験区分（`TestClass`） | source／contract／fixtureあり。OS product class実行は`NOT_RUN`。 |
| 証拠ID（`Evidence ID`） | `NONE`。本E0 mappingから製品Acceptance evidenceを発行しない。 |
| 阻害要因（`Blocker`） | E0 mappingはOPEN。既存Registry／Release blockerを変更せず、新規Blockerを作らない。 |
| 状態 | `OPEN`。E0照合状態。旧statusとB工程状態は変更しない。 |
| 残余 | 既存validator PASSはそのscopeのみ。Q0〜Q7 DEFERRED。 |

#### BUF-T14 `Export`／`Install`／`Update`／`Rollback`成果物の識別子結合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | BUF-T14 |
| 旧ID | BUF-T14（識別子を維持） |
| 原文参照 | v4仕様S06、原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`。旧補遺実装仕様§8.3は来歴資料。 |
| 対象 | 同一candidateのExport、package/install、update/rollback artifact。 |
| 目的 | source/contract/hash/installed identity/rollback対象を束縛。 |
| 生成元（`Producer`） | Export生成器、package構築器、更新catalog、導入済み製品識別値。 |
| 実Consumer | Broker export/install/update modules、OS tooling、N10/N11 validatorsが部分消費。 |
| 契約／Data | package manifest、成果物SHA、app／audit／runtime識別値、version遷移。 |
| 正常時作用 | 各OS artifactとinstalled/update/rollback identityを同一source candidateへ対応。 |
| 否定時作用 | 別OS/ABI/dev fixture/hash違いを同一artifact/candidateとしない。 |
| 停止／Recovery | hash/signature/identity違いはapply拒否、稼働版維持・Audit。 |
| 標準OS候補 | Windows／macOS／Linux native／Android／iOS。各OSの標準環境を別々に評価し、Linux nativeを必須対象として保持。今回OS実行なし。 |
| 配布物 | 各OSのN10 package／installer／update候補。形式・version・archが未固定のものはUNKNOWN。本source mappingは配布Acceptanceではない。 |
| 既存実装 | Export／install／updateのRust実装、Linux／Windows package／証拠tooling。 |
| BUILD_GAP | 5 OSのExport→正式Install→Update→Rollback lifecycle evidenceなし。 |
| 参照元ハッシュ（`SourceSHA`） | 照合HEAD=`80a16a3a03a91da63fd6b2b7fa1f972369ae9cc6`。v4仕様原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`、v4仕様Git blob=`352c53180cbad07888beb956aad0fb2a58074c58`、v4採用記録Git blob=`f159c1aa6ea5bc6e80e0877d8068f7e44da4c4d3`、旧補遺実装仕様Git blob=`2d0e98fb0b58ceec65244fde6a8fbf8e27c49b7e`。`native/rust_helper/src/broker/export_center.rs`=`477e44d86ebf18d2b004a7294c9cdb14e35aa0f0`、`native/rust_helper/src/broker/product_install.rs`=`c91115bdf88dd28e5f3445e7ed66127902909677`、`native/rust_helper/src/broker/update_center.rs`=`f6fc98155fcd476765b60d382a680c7f7c6d675e`、`tooling/package_linux_product.py`=`0d32d04ec5b43e1815b3664a323c17db79bc928a`、`tooling/windows_release_evidence.py`=`6f7957c6193c86c7bc15165e743f11651464f29c`、`specs/d4_pocket_product_package_manifest.schema.json`=`3151ba81a869b87e6c0fa8f8633a0e424fc43ae7`。Git blobはpathと対にした照合HEAD時点の識別値。 |
| 試験区分（`TestClass`） | `SOURCE`／`CONTRACT`を識別。installed lifecycleは`NOT_RUN`。 |
| 証拠ID（`Evidence ID`） | `NONE`。本E0 mappingから製品Acceptance evidenceを発行しない。 |
| 阻害要因（`Blocker`） | E0 mappingはOPEN。既存Registry／Release blockerを変更せず、新規Blockerを作らない。 |
| 状態 | `OPEN`。E0照合状態。旧statusとB工程状態は変更しない。 |
| 残余 | 個別package fixtureを一候補lifecycle PASSへ合成しない。 |

#### BUF-T15 介入前後matched比較・副作用・performance

| E0-04項目 | 個別値 |
|---|---|
| 現ID | BUF-T15 |
| 旧ID | BUF-T15（識別子を維持） |
| 原文参照 | v4仕様S06、原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`。旧補遺実装仕様§8.3は来歴資料。 |
| 対象 | 緩衝有無で揃えたsource/input/Host/toolchainとresult/state/resource。 |
| 目的 | 実効果を比較しfailure/resource/負担の下流転移を確認。 |
| 生成元（`Producer`） | 条件一致した実行metadata、状態／結果diff、resource／回収／失敗log。 |
| 実Consumer | B5限定比較、observation center/resource contractに測定点。汎用comparatorなし。 |
| 契約／Data | 条件fingerprint、介入、実結果、resource/time/cleanup、下流測定。 |
| 正常時作用 | 同条件前後を比較し改善・副作用・測定不能を分離。 |
| 否定時作用 | retry/path/assertion差を因果とせず取得不能resourceを0にしない。 |
| 停止／Recovery | 条件不一致/欠測はUNKNOWN。危険作用は停止。 |
| 標準OS候補 | Windows／macOS／Linux native／Android／iOS。各OSの標準環境を別々に評価し、Linux nativeを必須対象として保持。今回OS実行なし。 |
| 配布物 | 各OSのN10 package／installer／update候補。形式・version・archが未固定のものはUNKNOWN。本source mappingは配布Acceptanceではない。 |
| 既存実装 | B5の限定比較、`observation_center.rs`、resource観測specification。 |
| BUILD_GAP | downstream failure/Authority/resource/operator burdenのmatched acceptanceなし。 |
| 参照元ハッシュ（`SourceSHA`） | 照合HEAD=`80a16a3a03a91da63fd6b2b7fa1f972369ae9cc6`。v4仕様原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`、v4仕様Git blob=`352c53180cbad07888beb956aad0fb2a58074c58`、v4採用記録Git blob=`f159c1aa6ea5bc6e80e0877d8068f7e44da4c4d3`、旧補遺実装仕様Git blob=`2d0e98fb0b58ceec65244fde6a8fbf8e27c49b7e`。`docs/緩衝基盤_rev5補遺_QC/B5_自己適用.md`=`16081537b6699c176144e32ca97d6bdd20d98866`、`native/rust_helper/src/broker/observation_center.rs`=`62896e497a0d78267020db6bae2ecadf704fa1fb`、`docs/specs/runtime-resource-observation.md`=`af29714c0372592a5a86141c6d4ea4f491cbf851`。Git blobはpathと対にした照合HEAD時点の識別値。 |
| 試験区分（`TestClass`） | B5旧限定比較あり。現candidate comparisonは`NOT_RUN`。 |
| 証拠ID（`Evidence ID`） | `NONE`。本E0 mappingから製品Acceptance evidenceを発行しない。 |
| 阻害要因（`Blocker`） | E0 mappingはOPEN。既存Registry／Release blockerを変更せず、新規Blockerを作らない。 |
| 状態 | `OPEN`。E0照合状態。旧statusとB工程状態は変更しない。 |
| 残余 | Q4/resource long-runはdeferred。本mappingで先行実行しない。 |

#### BUF-T16 D4 source／Toolchain pinと自己適用

| E0-04項目 | 個別値 |
|---|---|
| 現ID | BUF-T16 |
| 旧ID | BUF-T16（識別子を維持） |
| 原文参照 | v4仕様S06、原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`。旧補遺実装仕様§8.3は来歴資料。 |
| 対象 | D4 source、依存lock、toolchain fingerprint、開発／test runner。 |
| 目的 | 結果を再現可能なsource/dependency候補へ結ぶ。 |
| 生成元（`Producer`） | `main`のcommit/tree、`Cargo`／`Flutter` lock、toolchain、runner結果。 |
| 実Consumer | 隔離runner、B5自己適用history、`Manifest`／証拠validator。 |
| 契約／Data | source/blob、lock SHA、toolchain、target／command／exit／artifact識別値。 |
| 正常時作用 | 固定source/toolchainだけ実行し結果を同candidateへ帰属。 |
| 否定時作用 | 別worktree/unlocked resolution/過去runを現candidate PASSにしない。 |
| 停止／Recovery | pin変化/cleanup failureはscope mismatchとして隔離。 |
| 標準OS候補 | Windows／macOS／Linux native／Android／iOS。各OSの標準環境を別々に評価し、Linux nativeを必須対象として保持。今回OS実行なし。 |
| 配布物 | 各OSのN10 package／installer／update候補。形式・version・archが未固定のものはUNKNOWN。本source mappingは配布Acceptanceではない。 |
| 既存実装 | `Cargo`／`Flutter` runner、Cargo manifest／lock、B5記録。 |
| BUILD_GAP | 全入口・全OSのsource/dependency/artifact provenanceとself-application acceptanceなし。 |
| 参照元ハッシュ（`SourceSHA`） | 照合HEAD=`80a16a3a03a91da63fd6b2b7fa1f972369ae9cc6`。v4仕様原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`、v4仕様Git blob=`352c53180cbad07888beb956aad0fb2a58074c58`、v4採用記録Git blob=`f159c1aa6ea5bc6e80e0877d8068f7e44da4c4d3`、旧補遺実装仕様Git blob=`2d0e98fb0b58ceec65244fde6a8fbf8e27c49b7e`。`docs/緩衝基盤_rev5補遺_QC/B5_自己適用.md`=`16081537b6699c176144e32ca97d6bdd20d98866`、`tooling/run_cargo_isolated.py`=`6890c89b384db2169b92a7326f04dbd2ea6341e1`、`native/rust_helper/Cargo.toml`=`6352fd7de3188b4c7335167f4f8875a4f8f4e28b`、`native/rust_helper/Cargo.lock`=`8647830893561de17e82063231b651322c8909bd`。Git blobはpathと対にした照合HEAD時点の識別値。 |
| 試験区分（`TestClass`） | B5は対象commit/fingerprint付き履歴。今回self-application testは`NOT_RUN`。 |
| 証拠ID（`Evidence ID`） | `NONE`。本E0 mappingから製品Acceptance evidenceを発行しない。 |
| 阻害要因（`Blocker`） | E0 mappingはOPEN。既存Registry／Release blockerを変更せず、新規Blockerを作らない。 |
| 状態 | `OPEN`。E0照合状態。旧statusとB工程状態は変更しない。 |
| 残余 | B5全体やOS製品受入へ昇格しない。 |

#### BUF-T17 Adapter不在・未知原因・OS制約時の停止

| E0-04項目 | 個別値 |
|---|---|
| 現ID | BUF-T17 |
| 旧ID | BUF-T17（識別子を維持） |
| 原文参照 | v4仕様S06、原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`。旧補遺実装仕様§8.3は来歴資料。 |
| 対象 | 未対応Adapter/Runtime、unknown error/cause、OS拒否、capability不足。 |
| 目的 | 未対応を明示しfallbackや権限拡張を防ぐ。 |
| 生成元（`Producer`） | Adapter／runtime registry、`Broker`操作、capability／error結果。 |
| 実Consumer | Adapter registry・Runtime registry・Broker protocolのunsupported/deny path。 |
| 契約／Data | known IDs、capability state、bounded error、独立Broker authority check。 |
| 正常時作用 | 対応capabilityだけ実行し未対応理由/回復可能性を返す。 |
| 否定時作用 | unknown metadataから権限を作らずsilent fallbackしない。 |
| 停止／Recovery | unknown/OS拒否/consumer欠落はunsupported/UNKNOWNで停止しAudit保持。 |
| 標準OS候補 | Windows／macOS／Linux native／Android／iOS。各OSの標準環境を別々に評価し、Linux nativeを必須対象として保持。今回OS実行なし。 |
| 配布物 | 各OSのN10 package／installer／update候補。形式・version・archが未固定のものはUNKNOWN。本source mappingは配布Acceptanceではない。 |
| 既存実装 | `adapters/mod.rs`、`runtime_registry.rs`、`protocol.rs`、`Conformance`検査source。 |
| BUILD_GAP | 各OSのunknown failureからGUI/Audit/Recoveryまでのproduct evidenceなし。 |
| 参照元ハッシュ（`SourceSHA`） | 照合HEAD=`80a16a3a03a91da63fd6b2b7fa1f972369ae9cc6`。v4仕様原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`、v4仕様Git blob=`352c53180cbad07888beb956aad0fb2a58074c58`、v4採用記録Git blob=`f159c1aa6ea5bc6e80e0877d8068f7e44da4c4d3`、旧補遺実装仕様Git blob=`2d0e98fb0b58ceec65244fde6a8fbf8e27c49b7e`。`native/rust_helper/src/adapters/mod.rs`=`06c540a59437b903b3f045b5aafb3790c21103bd`、`native/rust_helper/src/broker/runtime_registry.rs`=`9ff28610862586a599848d75f9e891a6c6cadf2f`、`native/rust_helper/src/broker/protocol.rs`=`9b2076d5f31ca6d77f1f87973d1b394aaf751961`、`tooling/conformance_tests/run_conformance_skeleton.py`=`c6f31d8cb561b48513c6784e54c85969c9cb953e`。Git blobはpathと対にした照合HEAD時点の識別値。 |
| 試験区分（`TestClass`） | source／fixtureあり。標準OS拒否testは`NOT_RUN`。 |
| 証拠ID（`Evidence ID`） | `NONE`。本E0 mappingから製品Acceptance evidenceを発行しない。 |
| 阻害要因（`Blocker`） | E0 mappingはOPEN。既存Registry／Release blockerを変更せず、新規Blockerを作らない。 |
| 状態 | `OPEN`。E0照合状態。旧statusとB工程状態は変更しない。 |
| 残余 | unknown causeとfallback不在の全runtime保証は未確立。 |

#### BUF-T18 Q1〜Q6統合とQ0 candidate freeze

| E0-04項目 | 個別値 |
|---|---|
| 現ID | BUF-T18 |
| 旧ID | BUF-T18（識別子を維持） |
| 原文参照 | v4仕様S06、原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`。旧補遺実装仕様§8.3は来歴資料。 |
| 対象 | 統合candidate、Q0〜Q7/FQ queue、Release Gate。 |
| 目的 | candidate freezeと正式最終QC入口を守り旧証拠を転用しない。 |
| 生成元（`Producer`） | candidateのcommit／source／artifact／evidence識別値、E0〜E3状態。 |
| 実Consumer | FINAL_QA_QUEUE、release gate、N11 registryが部分消費。 |
| 契約／Data | Q ID、現行進捗、candidate identity、stage入口条件。 |
| 正常時作用 | 現工程条件成立後、同一candidateのQ段階を実施・記録。 |
| 否定時作用 | E0 mappingからQ0を開始せず旧PASS/FAILを付替えない。 |
| 停止／Recovery | 入口未成立/候補変更は保留。history保持し新証拠を分離。 |
| 標準OS候補 | Windows／macOS／Linux native／Android／iOS。各OSの標準環境を別々に評価し、Linux nativeを必須対象として保持。今回OS実行なし。 |
| 配布物 | 各OSのN10 package／installer／update候補。形式・version・archが未固定のものはUNKNOWN。本source mappingは配布Acceptanceではない。 |
| 既存実装 | FINAL_QA_QUEUE、release gate、Conformance source、v4工程表。 |
| BUILD_GAP | Q0 freezeとQ1〜Q6 integrated evidenceはDEFERRED/未実行。 |
| 参照元ハッシュ（`SourceSHA`） | 照合HEAD=`80a16a3a03a91da63fd6b2b7fa1f972369ae9cc6`。v4仕様原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`、v4仕様Git blob=`352c53180cbad07888beb956aad0fb2a58074c58`、v4採用記録Git blob=`f159c1aa6ea5bc6e80e0877d8068f7e44da4c4d3`、旧補遺実装仕様Git blob=`2d0e98fb0b58ceec65244fde6a8fbf8e27c49b7e`。`docs/FINAL_QA_QUEUE.md`=`7b332cccab70bb68aa1ff4f8e23acc0e816d8282`、`tooling/release_gate_check.py`=`22a8d7bfe9adb259f44ce8501fb215652e1d58c6`、`tooling/conformance_tests/run_conformance_skeleton.py`=`c6f31d8cb561b48513c6784e54c85969c9cb953e`、`docs/QE_2026-11-13/02_D4Pocket_統合開発工程表_v4.md`=`d32dd251e5f7cbb97d92f6e7540d6b6ed0ecec05`。Git blobはpathと対にした照合HEAD時点の識別値。 |
| 試験区分（`TestClass`） | source／contractのみ。formal Q0〜Q7は`NOT_RUN`／DEFERRED。 |
| 証拠ID（`Evidence ID`） | `NONE`。本E0 mappingから製品Acceptance evidenceを発行しない。 |
| 阻害要因（`Blocker`） | E0 mappingはOPEN。既存Registry／Release blockerを変更せず、新規Blockerを作らない。 |
| 状態 | `OPEN`。E0照合状態。旧statusとB工程状態は変更しない。 |
| 残余 | 現行工程順を保持。Q0〜Q7を開始・再定義しない。 |

#### BUF-T19 要求原文→実作用→実測結果traceability

| E0-04項目 | 個別値 |
|---|---|
| 現ID | BUF-T19 |
| 旧ID | BUF-T19（識別子を維持） |
| 原文参照 | v4仕様S06、原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`。旧補遺実装仕様§8.3は来歴資料。 |
| 対象 | 目的・design・source/contract・product action・result・consumer・Audit。 |
| 目的 | 要求と実作用のtraceを作り不一致時の上流前提を特定。 |
| 生成元（`Producer`） | v4要求／S06、source commit、artifact／runtime作用／結果。 |
| 実Consumer | E0 ledger、runtime result schema、Evidence bundle/Registryが部分消費。 |
| 契約／Data | 要求→`SourceSHA`→`Contract`→artifact／OS／runtime→Tool作用／結果／Audit／Recovery。 |
| 正常時作用 | 実測値・作用をcandidate/consumerへ結び要求条件と比較。 |
| 否定時作用 | Manifest/hash/schema適合だけで製品作用を証明しない。 |
| 停止／Recovery | link欠落・不一致はEvidence IDなし、OPEN/UNKNOWNへ。 |
| 標準OS候補 | Windows／macOS／Linux native／Android／iOS。各OSの標準環境を別々に評価し、Linux nativeを必須対象として保持。今回OS実行なし。 |
| 配布物 | 各OSのN10 package／installer／update候補。形式・version・archが未固定のものはUNKNOWN。本source mappingは配布Acceptanceではない。 |
| 既存実装 | v4仕様、runtime result schema、証拠bundle／release gate。 |
| BUILD_GAP | 実Tool・後続AI/UI・Auditまでの一体record consumerが未成立。 |
| 参照元ハッシュ（`SourceSHA`） | 照合HEAD=`80a16a3a03a91da63fd6b2b7fa1f972369ae9cc6`。v4仕様原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`、v4仕様Git blob=`352c53180cbad07888beb956aad0fb2a58074c58`、v4採用記録Git blob=`f159c1aa6ea5bc6e80e0877d8068f7e44da4c4d3`、旧補遺実装仕様Git blob=`2d0e98fb0b58ceec65244fde6a8fbf8e27c49b7e`。`docs/QE_2026-11-13/01_D4Pocket_統合製品実装仕様書_v4.md`=`352c53180cbad07888beb956aad0fb2a58074c58`、`specs/runtime_result_evidence.schema.json`=`0844d38d864c9e556631a4a41cc75de13a99c40e`、`tooling/evidence_bundle.py`=`68dead851236b0f1b69899151c37371e593ff3f9`、`tooling/release_gate_check.py`=`22a8d7bfe9adb259f44ce8501fb215652e1d58c6`。Git blobはpathと対にした照合HEAD時点の識別値。 |
| 試験区分（`TestClass`） | `CONTRACT`／`SOURCE`あり。製品作用 evidenceは`NOT_RUN`。 |
| 証拠ID（`Evidence ID`） | `NONE`。本E0 mappingから製品Acceptance evidenceを発行しない。 |
| 阻害要因（`Blocker`） | E0 mappingはOPEN。既存Registry／Release blockerを変更せず、新規Blockerを作らない。 |
| 状態 | `OPEN`。E0照合状態。旧statusとB工程状態は変更しない。 |
| 残余 | E0 mappingはtraceability一部で製品Acceptanceでない。 |

#### BUF-T20 反例／unknownによるcause model再評価

| E0-04項目 | 個別値 |
|---|---|
| 現ID | BUF-T20 |
| 旧ID | BUF-T20（識別子を維持） |
| 原文参照 | v4仕様S06、原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`。旧補遺実装仕様§8.3は来歴資料。 |
| 対象 | 仮説と反する観測、unknown failure、介入後反例・上流条件。 |
| 目的 | statusだけでなく原因モデル・設計責任・介入方法を更新。 |
| 生成元（`Producer`） | 矛盾する観測、B0原因参照、条件一致した修正後結果。 |
| 実Consumer | B0/B1 docsに選択履歴。汎用runtime feedback consumerなし。 |
| 契約／Data | observation ID、hypothesis/version、反例、design/test owner。 |
| 正常時作用 | 反例を仮説へ結び支持/棄却/unknownと上流責任を記録。 |
| 否定時作用 | 成功だけ採択せず失敗を消す／causeを後付けで一意化しない。 |
| 停止／Recovery | 原因未確定はUNKNOWN/OPEN、危険作用を止めdesign/test ownerへ戻す。 |
| 標準OS候補 | Windows／macOS／Linux native／Android／iOS。各OSの標準環境を別々に評価し、Linux nativeを必須対象として保持。今回OS実行なし。 |
| 配布物 | 各OSのN10 package／installer／update候補。形式・version・archが未固定のものはUNKNOWN。本source mappingは配布Acceptanceではない。 |
| 既存実装 | B0 cause ledger／不足証拠／B1 design QC histories。 |
| BUILD_GAP | 観測→要求/design/test owner→matched correction/effect consumerが未接続。 |
| 参照元ハッシュ（`SourceSHA`） | 照合HEAD=`80a16a3a03a91da63fd6b2b7fa1f972369ae9cc6`。v4仕様原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`、v4仕様Git blob=`352c53180cbad07888beb956aad0fb2a58074c58`、v4採用記録Git blob=`f159c1aa6ea5bc6e80e0877d8068f7e44da4c4d3`、旧補遺実装仕様Git blob=`2d0e98fb0b58ceec65244fde6a8fbf8e27c49b7e`。`docs/緩衝基盤_rev5補遺_QC/B0_原因対応台帳.md`=`299c68b3a5266ab4b1946f4b06caa9e4069274a9`、`docs/緩衝基盤_rev5補遺_QC/B0_不足証拠一覧.md`=`ff35b791d6a022f1879d3f0002ed0d219e89578b`、`docs/緩衝基盤_rev5補遺_QC/B1_設計QC.md`=`aa113006eb7a108848211f4dfb3475dc0cfbc9d5`。Git blobはpathと対にした照合HEAD時点の識別値。 |
| 試験区分（`TestClass`） | 文書/history sourceのみ。今回cause update/testは`NOT_RUN`。 |
| 証拠ID（`Evidence ID`） | `NONE`。本E0 mappingから製品Acceptance evidenceを発行しない。 |
| 阻害要因（`Blocker`） | E0 mappingはOPEN。既存Registry／Release blockerを変更せず、新規Blockerを作らない。 |
| 状態 | `OPEN`。E0照合状態。旧statusとB工程状態は変更しない。 |
| 残余 | 選択historyを全母集団のfeedbackへ一般化しない。 |

#### BUF-T21 変更前観測→是正→変更後測定→局所受入

| E0-04項目 | 個別値 |
|---|---|
| 現ID | BUF-T21 |
| 旧ID | BUF-T21（識別子を維持） |
| 原文参照 | v4仕様S06、原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`。旧補遺実装仕様§8.3は来歴資料。 |
| 対象 | 各開発単位のbaseline、change、post observation、Acceptance。 |
| 目的 | 実装存在だけでQC完了とせず実物作用を目的へ照らす。 |
| 生成元（`Producer`） | BUILD_BATCH前後source/test/runtime evidence、command/exit/result。 |
| 実Consumer | AGENTS/v4施工指示、progress/E0 ledger、各機能consumer。 |
| 契約／Data | requirement ID、source baseline/final、条件、change、test/outcome/residual。 |
| 正常時作用 | 事前観測→是正→必要test→局所判定を記録。 |
| 否定時作用 | docs/compile/fixtureのみでproduct QC/QA受入としない。 |
| 停止／Recovery | 条件/source変化で比較不能ならscopeを保ちAcceptanceを限定保留。 |
| 標準OS候補 | Windows／macOS／Linux native／Android／iOS。各OSの標準環境を別々に評価し、Linux nativeを必須対象として保持。今回OS実行なし。 |
| 配布物 | 各OSのN10 package／installer／update候補。形式・version・archが未固定のものはUNKNOWN。本source mappingは配布Acceptanceではない。 |
| 既存実装 | root AGENTS、v4 Codex指示、Manifest/Conformance/Release validators。 |
| BUILD_GAP | 自動pre/post測定とAcceptance recordからproduct actionへの接続なし。 |
| 参照元ハッシュ（`SourceSHA`） | 照合HEAD=`80a16a3a03a91da63fd6b2b7fa1f972369ae9cc6`。v4仕様原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`、v4仕様Git blob=`352c53180cbad07888beb956aad0fb2a58074c58`、v4採用記録Git blob=`f159c1aa6ea5bc6e80e0877d8068f7e44da4c4d3`、旧補遺実装仕様Git blob=`2d0e98fb0b58ceec65244fde6a8fbf8e27c49b7e`。`AGENTS.md`=`1eafcadd129fd01f1f2f683594c381605ed7d48d`、`docs/QE_2026-11-13/03_D4Pocket_Codex施工指示書_v4.md`=`49447973e88c50a09b2f4978adfed84fd464c6d0`、`tooling/manifest.py`=`aa4525ff20293cb8641e5598a2439561a3bfbd3b`、`tooling/release_gate_check.py`=`22a8d7bfe9adb259f44ce8501fb215652e1d58c6`。Git blobはpathと対にした照合HEAD時点の識別値。 |
| 試験区分（`TestClass`） | 本件は文書mapping。製品挙動classは`NOT_RUN`。 |
| 証拠ID（`Evidence ID`） | `NONE`。本E0 mappingから製品Acceptance evidenceを発行しない。 |
| 阻害要因（`Blocker`） | E0 mappingはOPEN。既存Registry／Release blockerを変更せず、新規Blockerを作らない。 |
| 状態 | `OPEN`。E0照合状態。旧statusとB工程状態は変更しない。 |
| 残余 | process規則はproduct result証拠ではない。 |

#### BUF-T22 matched条件と因果閉鎖の区別

| E0-04項目 | 個別値 |
|---|---|
| 現ID | BUF-T22 |
| 旧ID | BUF-T22（識別子を維持） |
| 原文参照 | v4仕様S06、原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`。旧補遺実装仕様§8.3は来歴資料。 |
| 対象 | 介入前後source/input/Host/toolchain/command/assertion条件。 |
| 目的 | 一致比較で効果を判断しretry/path/assertion変更だけを根因解消としない。 |
| 生成元（`Producer`） | B5比較記録、source commit、toolchain／Host fingerprint、command／結果。 |
| 実Consumer | B5に部分matched recordsと未解決case。専用comparatorなし。 |
| 契約／Data | 条件fingerprint、変更要因、control/intervention result、measurement scope。 |
| 正常時作用 | 差分を明示し同条件対照と比較して仮説へ帰属。 |
| 否定時作用 | retry PASS、OneDrive外移動、assertion変更を因果修正としない。 |
| 停止／Recovery | 条件不一致は履歴保持し比較無効。再設計は別record。 |
| 標準OS候補 | Windows／macOS／Linux native／Android／iOS。各OSの標準環境を別々に評価し、Linux nativeを必須対象として保持。今回OS実行なし。 |
| 配布物 | 各OSのN10 package／installer／update候補。形式・version・archが未固定のものはUNKNOWN。本source mappingは配布Acceptanceではない。 |
| 既存実装 | B5履歴、Pktmon解析fixture、Cargo隔離fixture。 |
| BUILD_GAP | 統一fingerprint/comparatorと誤causal closure reject testなし。 |
| 参照元ハッシュ（`SourceSHA`） | 照合HEAD=`80a16a3a03a91da63fd6b2b7fa1f972369ae9cc6`。v4仕様原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`、v4仕様Git blob=`352c53180cbad07888beb956aad0fb2a58074c58`、v4採用記録Git blob=`f159c1aa6ea5bc6e80e0877d8068f7e44da4c4d3`、旧補遺実装仕様Git blob=`2d0e98fb0b58ceec65244fde6a8fbf8e27c49b7e`。`docs/緩衝基盤_rev5補遺_QC/B5_自己適用.md`=`16081537b6699c176144e32ca97d6bdd20d98866`、`tooling/test_pktmon_packet_summary.ps1`=`a5b0dc6ae7d45845012be47e221df8d2b727f217`、`tooling/conformance_tests/test_run_cargo_isolated.py`=`b5a8c42b6c9fee9f9886b6dd3fb7e39f65530a99`。Git blobはpathと対にした照合HEAD時点の識別値。 |
| 試験区分（`TestClass`） | 旧部分記録あり。current matched causal testは`NOT_RUN`。 |
| 証拠ID（`Evidence ID`） | `NONE`。本E0 mappingから製品Acceptance evidenceを発行しない。 |
| 阻害要因（`Blocker`） | E0 mappingはOPEN。既存Registry／Release blockerを変更せず、新規Blockerを作らない。 |
| 状態 | `OPEN`。E0照合状態。旧statusとB工程状態は変更しない。 |
| 残余 | 過去FAIL/UNKNOWNを保持しretry/path/assertionのみで閉じない。 |

#### BUF-T23 failure転移・resource・Authority・運用負荷

| E0-04項目 | 個別値 |
|---|---|
| 現ID | BUF-T23 |
| 旧ID | BUF-T23（識別子を維持） |
| 原文参照 | v4仕様S06、原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`。旧補遺実装仕様§8.3は来歴資料。 |
| 対象 | Buffer変更と隣接/downstream consumer、cleanup、resource、Authority、operator work。 |
| 目的 | 局所改善が他境界へfailure/負荷を移していないか測る。 |
| 生成元（`Producer`） | Buffer出力、下流結果／失敗、resource、回収、Permission／Audit。 |
| 実Consumer | B5にsnapshot/shared Pub cacheの限定local PASS。observation centerに一部resource consumer。 |
| 契約／Data | before/after downstream identity、測定値/unknown、cleanup/failure/authority effect。 |
| 正常時作用 | 隣接consumerとresource/cleanupを測りunknownはunknownと記録。 |
| 否定時作用 | 未取得resourceを0にせず局所PASSを長期/全OSへ拡張しない。 |
| 停止／Recovery | 下流failure/Authority drift時に介入停止、元境界を保護し上流へ。 |
| 標準OS候補 | Windows／macOS／Linux native／Android／iOS。各OSの標準環境を別々に評価し、Linux nativeを必須対象として保持。今回OS実行なし。 |
| 配布物 | 各OSのN10 package／installer／update候補。形式・version・archが未固定のものはUNKNOWN。本source mappingは配布Acceptanceではない。 |
| 既存実装 | B5の限定T23記録、Flutter作業領域、resource観測。 |
| BUILD_GAP | 長期/並行/multiproject/全OS転移・負荷評価consumerなし。 |
| 参照元ハッシュ（`SourceSHA`） | 照合HEAD=`80a16a3a03a91da63fd6b2b7fa1f972369ae9cc6`。v4仕様原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`、v4仕様Git blob=`352c53180cbad07888beb956aad0fb2a58074c58`、v4採用記録Git blob=`f159c1aa6ea5bc6e80e0877d8068f7e44da4c4d3`、旧補遺実装仕様Git blob=`2d0e98fb0b58ceec65244fde6a8fbf8e27c49b7e`。`docs/緩衝基盤_rev5補遺_QC/B5_自己適用.md`=`16081537b6699c176144e32ca97d6bdd20d98866`、`tooling/flutter_workspace.py`=`dab1c73e2dad774fd7c8c385991a4d66a969ca19`、`native/rust_helper/src/broker/observation_center.rs`=`62896e497a0d78267020db6bae2ecadf704fa1fb`、`docs/specs/runtime-resource-observation.md`=`af29714c0372592a5a86141c6d4ea4f491cbf851`。Git blobはpathと対にした照合HEAD時点の識別値。 |
| 試験区分（`TestClass`） | 旧B5のbounded local resultのみ。広域candidate testは`NOT_RUN`。 |
| 証拠ID（`Evidence ID`） | `NONE`。本E0 mappingから製品Acceptance evidenceを発行しない。 |
| 阻害要因（`Blocker`） | E0 mappingはOPEN。既存Registry／Release blockerを変更せず、新規Blockerを作らない。 |
| 状態 | `OPEN`。E0照合状態。旧statusとB工程状態は変更しない。 |
| 残余 | 旧T23 local PASSとQ4 deferredを分離。一般非転移/性能はOPEN。 |

#### BUF-T24 旧QC／Q0／Q1と新candidateの版別結合

| E0-04項目 | 個別値 |
|---|---|
| 現ID | BUF-T24 |
| 旧ID | BUF-T24（識別子を維持） |
| 原文参照 | v4仕様S06、原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`。旧補遺実装仕様§8.3は来歴資料。 |
| 対象 | 旧QC・2026-10-07 Q0/Q1、緩衝後candidateと統合QC evidence。 |
| 目的 | 証拠をsource/条件へ束縛し旧PASS/FAILを現candidateへ付替えない。 |
| 生成元（`Producer`） | 旧history ref、新candidate commit/artifact/OS/class、新evidence record。 |
| 実Consumer | FINAL_QA_QUEUE、v4 adoption/progress、Evidence bundle/Manifest/Registryが部分保持。 |
| 契約／Data | candidate／source／artifact／OS／条件／class／Evidence ID／Q工程履歴。 |
| 正常時作用 | 旧新candidateを別版で保存し再利用条件を明記。 |
| 否定時作用 | old PASS/FAILやManifestだけをnew product acceptanceにしない。 |
| 停止／Recovery | identity/class不明はUNKNOWN、Evidence ID NONE。旧record不変。 |
| 標準OS候補 | Windows／macOS／Linux native／Android／iOS。各OSの標準環境を別々に評価し、Linux nativeを必須対象として保持。今回OS実行なし。 |
| 配布物 | 各OSのN10 package／installer／update候補。形式・version・archが未固定のものはUNKNOWN。本source mappingは配布Acceptanceではない。 |
| 既存実装 | `FINAL_QA_QUEUE`、v4／2026-10-22採用記録、証拠bundle。 |
| BUILD_GAP | old→current ancestry/OS artifact validatorとQ0 freeze consumerなし。 |
| 参照元ハッシュ（`SourceSHA`） | 照合HEAD=`80a16a3a03a91da63fd6b2b7fa1f972369ae9cc6`。v4仕様原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`、v4仕様Git blob=`352c53180cbad07888beb956aad0fb2a58074c58`、v4採用記録Git blob=`f159c1aa6ea5bc6e80e0877d8068f7e44da4c4d3`、旧補遺実装仕様Git blob=`2d0e98fb0b58ceec65244fde6a8fbf8e27c49b7e`。`docs/FINAL_QA_QUEUE.md`=`7b332cccab70bb68aa1ff4f8e23acc0e816d8282`、`docs/QE_2026-11-13/採用記録.md`=`f159c1aa6ea5bc6e80e0877d8068f7e44da4c4d3`、`docs/QE_2026-10-22/採用記録.md`=`1f9d6332132759ed68f3af8b4587368b162096c2`、`tooling/evidence_bundle.py`=`68dead851236b0f1b69899151c37371e593ff3f9`。Git blobはpathと対にした照合HEAD時点の識別値。 |
| 試験区分（`TestClass`） | history/document sourceのみ。formal new-candidate Q0は`NOT_RUN`／DEFERRED。 |
| 証拠ID（`Evidence ID`） | `NONE`。本E0 mappingから製品Acceptance evidenceを発行しない。 |
| 阻害要因（`Blocker`） | E0 mappingはOPEN。既存Registry／Release blockerを変更せず、新規Blockerを作らない。 |
| 状態 | `OPEN`。E0照合状態。旧statusとB工程状態は変更しない。 |
| 残余 | 旧証拠を保持。現candidate Evidence ID/Acceptance未発行・OPEN。 |

24 IDすべてに21 fieldを記録した。これはE0 mappingであり、BUF-T Acceptance、B工程判定、N08/N10/N11受入、E0完了を意味しない。今回の製品runtime／Rust／Flutter／installed OS testは`NOT_RUN`。過去のCLOSED／FAIL／部分PASSは元source・OS・条件の履歴として保持し、再試験・status変更・新規Evidence/Blocker発行を行っていない。


## 旧C0〜C6 E0-04個別照合（現行v4）

以下は旧要求IDごとの21-field Source→Consumer mapping。旧状態・旧証拠は上の履歴crosswalkのまま保持し、v4のE0照合状態と製品Acceptanceを分離する。照合HEADはmain `d2355704dd108987ae3469eaa17d62fb38915254`。SourceSHA表はそのHEADにおけるGit blob IDで、source同一性を示す。今回、C0〜C6の製品Rust／Flutter／installed／OS testは再実行していない。CLOSED履歴の再試験、新規Evidence ID・Blockerの発行はない。

### C0 — Baseline再固定

| E0-04項目 | 個別値 |
|---|---|
| 現ID | C0 Baseline再固定（現行v4ではN11／E0の基準・証拠統治へ対応）。 |
| 旧ID | C0／Phase 0。旧rev1の「基準検証保存・先行検証整理中」、旧main・検査数・外部logをその時点の履歴として保持。 |
| 原文参照 | 旧rev1工程表C0。v4仕様S04 N11、S06、採用記録付き三点セット（仕様／工程表／指示書）のmanifest SHAは採用記録に固定。 |
| 対象 | 現行main／manifest／Schema・example・conformance／検査器／Release Blocker Registry／E0 source台帳の基準同一性。製品機能の実行経路ではない。 |
| 目的 | 現候補を再現可能なcommit・資産・検査範囲へ束縛し、過去baselineや部分検査を現候補の製品合格へ誤転記しない。 |
| 生成元（Producer） | 確認済みremote mainとlocal HEAD、追跡ファイル、manifest、正本採用記録、検査器出力、現Registry。 |
| 実Consumer | manifest.pyがtracked file hashを照合し、release_gate_check.pyが現Registry等を検査する。E0台帳／進捗が要求・source・試験範囲・残余を記録する。三点セット採用hashは採用記録が保持する。 |
| 契約／Data | Git commit／blob、三点セットSHA-256・Git blob、MANIFEST.sha256.json、release_blockers.registry.json、Schema／Conformance inventory、validation commandとその結果。 |
| 正常時作用 | 現HEAD・三点セット・追跡manifest・Registry・実行した検査を同じ候補へ結び、旧C0の検査数は旧source／条件に固定する。 |
| 否定時作用 | remote/local不一致、未保護差分、manifest/hash不一致、正本pin不一致、invalid schema／fixture、Release Gate失敗をPASSへ昇格せず、差分・条件を特定する。 |
| 停止／Recovery | 既存2世代復旧refを保持し、履歴証拠を上書きしない。不一致時はmanifestまたはRelease判定を成功扱いせず、root cause確認までbaselineを未確定とする。 |
| 標準OS候補 | Windows／macOS／Linux native／Android／iOS。各OS候補はN10の標準環境表に従う。ここでのsource照合は各OS実行を意味しない。Linux nativeは必須製品対象。 |
| 配布物 | C0固有の配布物なし。製品artifact／installer／更新・撤去はN10のOS別契約。 |
| 既存実装 | tooling/manifest.py、tooling/release_gate_check.py、tooling/conformance_tests/run_conformance_skeleton.py、tooling/validate_all.py、Registry、採用記録。 |
| BUILD_GAP | C0の旧検査結果はv4 candidateのQAではない。全OS installed evidence・Release証拠・E0の全要求結合は未成立。manifest checkだけで全runtimeを保証しない。 |
| 参照元ハッシュ（SourceSHA） | 現HEAD d2355704dd108987ae3469eaa17d62fb38915254。C0 SourceSHA表の9 pathを参照。自己参照するE0台帳／進捗／生成後MANIFEST blobは循環を避けて含めず、manifest検証結果は独立checkで確認する。 |
| 試験区分（TestClass） | STATIC_CONFORMANCE／Schema／manifest／Release Gate。旧C0のvalidate_all.py `--desktop-platform windows --include-mobile-release`成功は歴史範囲のみ。本mappingは製品試験NOT_RUN。 |
| 証拠ID（Evidence ID） | NONE。C0の旧外部log／旧commitは当時の証拠に限定し、v4製品Evidenceへ転用しない。 |
| 阻害要因（Blocker） | comprehensive_extension_rev1_completion等の既存Registry状態を維持。追加／状態変更なし。Registryは18件（16 unresolved／2 resolved）、release_ready=false。 |
| 状態 | OPEN（E0 mapping対象）。旧C0の履歴statusを変更せず、N11／E0と製品Release判定もOPEN。 |
| 残余 | 現候補の全要求・source・test・証拠・OS／artifactのN11結合と、最終QA／Release Review。 |

### C1 — 作業領域インスペクタ

| E0-04項目 | 個別値 |
|---|---|
| 現ID | C1 Workspace Inspector／現行v4 N09、K03。 |
| 旧ID | C1／Phase 5・6。File Tree、Diff、Tool／Shell履歴、Test結果、Approval／Audit、rollback候補。旧部分実装と未完範囲は履歴crosswalkを維持。 |
| 原文参照 | 旧rev1工程表C1。v4仕様S04 N09、S05 K03、工程R03-C／R06、施工指示I04-C。 |
| 対象 | 選択Workspaceのtree／bounded read／比較／差分表示／change list／Recovery previewと、Workspace permission・秘密・root境界。 |
| 目的 | 利用者が現在のBroker許可範囲内のWorkspace状態と差分を確認し、外部／secret内容を露出させず復旧候補を理解できるようにする。 |
| 生成元（Producer） | UIのWorkspace選択・許可済みread request、Brokerの登録root・permission・baseline・inspection result。 |
| 実Consumer | Agent Center／Workspace Inspector→WorkspaceClient→Flutter Broker transport→Rust Brokerのtree／read／compare／change-list／diff／recovery-preview。表示はBroker responseを消費する。 |
| 契約／Data | workspace_inspection_request／response、workspace_diff、Workspace登録・Task Workspace Permission request、Broker operation／result。結果／diffはWorkspace／baseline基準でTask成果物と同一とは限らない。 |
| 正常時作用 | Brokerが登録Workspaceと現在Permissionを検査し、bounded tree／read／diff等を返し、UIが現行応答を投影する。Recovery previewは候補の表示にとどまる。 |
| 否定時作用 | secret／Workspace外／未知root／期限切れPermission／stale request／malformed diffをBroker境界で拒否し、binary／oversize内容はcontract上の制限に従い露出を抑える。 |
| 停止／Recovery | permission・登録hash・Audit・response bindingの不一致は停止し、実Recovery成功として表示しない。Preview以後のrestore／rollback実作用は別の統治済み経路が必要。 |
| 標準OS候補 | Desktop Windows／macOS／Linux native。Mobileの限定投影は別Consumerで、Inspector全機能の同等性を示さない。各OSの標準候補はN10参照、Linux native必須。 |
| 配布物 | C1固有なし。Desktop／Mobileの実installed artifactはN10のOS別候補。形式が未確定の範囲はUNKNOWN。 |
| 既存実装 | workspace_inspector.dart、workspace_diff_panel.dart、WorkspaceClient、Rust workspace／workspace_root／workspace_reader／workspace_diff／protocol、関連Schemaとtests。 |
| BUILD_GAP | Task由来のresult／diff artifact・changed files・test resultへの結合、Tool／Shell履歴の実Task表示、実restore／rollback、旧巨大file等の残差は未成立。N09の同一AgentTask投影gapを保持。 |
| 参照元ハッシュ（SourceSHA） | 照合HEAD d2355704dd108987ae3469eaa17d62fb38915254。C1 SourceSHA表の15 pathを参照。 |
| 試験区分（TestClass） | UNIT／FIXTURE／WIDGET sourceあり（Flutter workspace_inspector_test、Rust workspace_reader／workspace_diff）。このmappingでは試験NOT_RUN。 |
| 証拠ID（Evidence ID） | NONE。旧C1局所証拠は当時の範囲に限定。 |
| 阻害要因（Blocker） | N09製品Acceptance OPEN、W00 SCOPED_PASS。専用Blockerの直接帰属はUNKNOWN。Registry変更なし。 |
| 状態 | OPEN（C1の現行Consumer mapping）。旧C1の部分成立／未完statusを保持し、N09 acceptanceへ昇格しない。 |
| 残余 | 同一Task result／diff／test投影、Task binding、実復旧作用、installed製品の境界証拠。 |

### C2 — 実行履歴・再実行・分岐

| E0-04項目 | 個別値 |
|---|---|
| 現ID | C2 History／replay／branch。現行v4 N09、K03。 |
| 旧ID | C2／Phase 9・10。過去履歴の表示、再実行／分岐における新規要求・識別子・Audit、旧権限の非継承。 |
| 原文参照 | 旧rev1工程表C2。v4仕様S04 N09、S05 K03、工程R03-C／R06、施工指示I04-C。 |
| 対象 | AgentTask履歴metadata、Runtime Dialogue履歴・bounded content、Dialogue replay／branchとその新規要求生成。 |
| 目的 | 記録を参照し再実行を新しい要求として作成し、過去Approval／Permission／Session stateをauthorityとして再利用させない。 |
| 生成元（Producer） | Brokerの実行履歴・Dialogue recordと、利用者が明示する新しいreplay／branch要求。 |
| 実Consumer | History screen／content dialog→共有HistoryClient→Broker history access／execution history／dialogue operation。AgentTask history metadataとDialogue replay／branchは異なるoperationとして扱う。 |
| 契約／Data | runtime_history_access、runtime_execution_history／page、runtime_dialogue_operation／request／response／comparison、runtime_agent_task_history request／access。各recordはidentity・範囲・grantをBrokerで検査する。 |
| 正常時作用 | 許可されたbounded履歴を表示する。Dialogue replay／branchは元内容を参照して新しい入力／requestを作る。 |
| 否定時作用 | 履歴改変、invalid／expired grant、closed／別Session・Runtime、旧Approval／Permissionの転用を拒否する。過去履歴やUI stateはApprovalではない。 |
| 停止／Recovery | identity／grant／Audit不一致は表示・再実行を停止する。AgentTask自体のreplay／restore／crash recoveryの一体consumerは未成立であり、Dialogue replayをそれと称さない。 |
| 標準OS候補 | Desktop Windows／macOS／Linux native。Mobile history projectionは限定metadataで、Desktop replay／branchの製品同等性を示さない。Linux native必須、OS別候補はN10参照。 |
| 配布物 | C2固有なし。OS別の通常installed artifactはN10候補に従う。 |
| 既存実装 | history_screen.dart、history_content_dialog.dart、共有HistoryClient、Broker history_access／execution_history／dialogue／protocol、History／Dialogue SchemaとFlutter tests。 |
| BUILD_GAP | 表示されたAgentTask historyから同じAgentTaskを新権限・新承認で再実行するconsumer、Task result／Audit／Recoveryを束ねる統合、旧C2のRecovery残差は未成立。Dialogue branchはAgentTask branchではない。 |
| 参照元ハッシュ（SourceSHA） | 照合HEAD d2355704dd108987ae3469eaa17d62fb38915254。C2 SourceSHA表の20 pathを参照。 |
| 試験区分（TestClass） | UNIT／FIXTURE／WIDGET sourceあり（history screen/content、runtime dialogue）。本mappingは実行履歴や外部作用を再試験せずNOT_RUN。 |
| 証拠ID（Evidence ID） | NONE。旧C2の限定UI／Broker証拠はその時点のstatusを保持。 |
| 阻害要因（Blocker） | N09製品Acceptance OPEN。既存Registryの18件状態を維持し、新規帰属／Blockerを作らない。 |
| 状態 | OPEN（現行source mapping）。 |
| 残余 | AgentTaskの履歴→新規認可付き再実行、結果／差分／Audit／RecoveryのTask単位結合とinstalled evidence。 |

### C3 — 資源監視

| E0-04項目 | 個別値 |
|---|---|
| 現ID | C3 Runtime resource observation。現行v4 N09、K04／K10。 |
| 旧ID | C3／Phase 11。process／CPU／RAM／Disk／Network／uptime、取得可能時のみGPU、Runtime Center表示。旧Windows開発用接続と未実施scopeは履歴crosswalkに保持。 |
| 原文参照 | 旧rev1工程表C3。v4仕様S04 N09、S05 K04／K10、工程R03-C／R06、施工指示I04-C。 |
| 対象 | Brokerが取得可能なRuntime資源観測とDesktop／限定Mobile投影。 |
| 目的 | 計測可能な実値だけを観測として表示し、未取得・非対応・失敗をunknownとして保つ。 |
| 生成元（Producer） | Rust Broker／OS observation producer。GPU等の値は取得できる場合だけ生成。 |
| 実Consumer | Runtime Center→RuntimeResourceClient→Broker resource query／RuntimeResourceRegistry。Mobile resource overview／Device Linkは許可された限定projection。 |
| 契約／Data | runtime_resource_query、runtime_resource_observation、正規化vector、Runtime identity／測定時刻／可用性。権限・Approvalはresource metadataから生成しない。 |
| 正常時作用 | 現在のRuntimeに対応する測定値を返し、取れないmetricはunknown／unavailableとして投影する。 |
| 否定時作用 | 未取得を0として捏造しない。invalid Runtime／metric／timestamp、stale observation、unauthorized Mobile requestを拒否またはunknownにする。 |
| 停止／Recovery | Sourceが停止・観測不能なら古い値を新値として表示せずstale／unknown化する。観測失敗でRuntime processを起動・停止しない。 |
| 標準OS候補 | Windows／macOS／Linux native Desktop、Android／iOSの限定Mobile observation。全OSの標準環境候補はN10参照。Linux native必須。各OSの実測・installed QAは未確立。 |
| 配布物 | C3固有なし。各OS配布候補はN10。 |
| 既存実装 | `runtime_center.dart`、デスクトップ／共通の`RuntimeResourceClient`、`resource_overview.dart`、`Broker`の`observation_center`／`runtime_registry`／`device_link`、資源Schema・正規化規約・試験源。 |
| BUILD_GAP | 資源観測は`Runtime` ID単位であり、`AgentTask` IDと結合されず、`Task`結果・実行時間・失敗比較でもない。全OSのインストール済み製品における資源測定・長時間／負荷の受入れは未成立。 |
| 参照元ハッシュ（SourceSHA） | 照合HEAD d2355704dd108987ae3469eaa17d62fb38915254。C3 SourceSHA表の13 pathを参照。 |
| 試験区分（TestClass） | UNIT／FIXTURE／WIDGET sourceあり（Desktop／shared／Mobile）。今回試験NOT_RUN。 |
| 証拠ID（Evidence ID） | NONE。このmappingは現行source接続の所在だけを記録する。 |
| 阻害要因（Blocker） | N09製品Acceptance OPEN。追加Registry項目なし。 |
| 状態 | OPEN（現行resource consumer mapping）。 |
| 残余 | Task単位のResource／duration比較、OS installed環境での取得範囲・unknownの表示・計測負荷確認。 |

### C4 — Runtimeライフサイクル

| E0-04項目 | 個別値 |
|---|---|
| 現ID | C4 Runtime lifecycle。現行v4 N09、K04／K13のRuntime操作境界。 |
| 旧ID | C4／Phase 12。start／stop／restart／pause／resume／quarantineのcapability-first要求と旧限定debug／fixture実装を履歴として保持。 |
| 原文参照 | 旧rev1工程表C4。v4仕様S04 N09、S05 K04／K13、工程R03-C／R06、施工指示I04-C。 |
| 対象 | Broker管理Runtimeの状態照会、capability・Permission・Approval・Audit・Recoveryを介したRuntime lifecycle要求。AgentTask lifecycleとは別対象。 |
| 目的 | 登録Runtimeに宣言された操作だけを既存Broker Authority経路で実行し、承認・記録・Recoveryを同一要求へ結ぶ。 |
| 生成元（Producer） | 利用者のRuntime操作要求、Brokerのregistered Runtime／capability／Permission／Approval state。 |
| 実Consumer | Runtime Center→RuntimeLifecycleClient→Broker lifecycle status／approval／execute→登録Adapter。現Release buildにtrusted lifecycle adapterを同梱しない旨を実装commentが明記。 |
| 契約／Data | `runtime_lifecycle_request`／`runtime_lifecycle_result`、実行系登録状態・操作能力・要求`hash`、`Permission`／`Approval`／`RecoveryAction`／`Audit`への束縛。 |
| 正常時作用 | 登録Runtime・対応capability・有効な権限・一回Approval・RecoveryActionが揃った時だけAdapterへ操作し、状態とAudit resultを返す。 |
| 否定時作用 | unknown Runtime／capability、permission denied、approval missing／expired／stale、invalid transition、Audit失敗、Recovery missingは副作用前に拒否・停止する。 |
| 停止／Recovery | Adapter失敗・state mismatch・Audit／Recovery不成立を成功へ昇格しない。quarantine／Recovery recordはBrokerの現状態を再確認し、AgentTask停止と混同しない。 |
| 標準OS候補 | Windows／macOS／Linux native Desktop。Mobile操作が存在してもOSごとのRuntime process authorityを証明しない。Linux native必須。標準環境・実測はN10参照。 |
| 配布物 | C4固有なし。Runtime adapterの通常配布・有効化もN10および各OS contractへ従う。 |
| 既存実装 | Runtime Center、Desktop／shared RuntimeLifecycleClient、runtime_lifecycle.rs、runtime_registry.rs、protocol、request/result SchemaとUNIT tests。 |
| BUILD_GAP | trusted lifecycle adapterが通常Releaseにないためproduction Runtimeへの実作用は未成立。既存fixture/debug接続は製品installed作用ではない。AgentTask単位のpause／resume／restartも別途未接続。 |
| 参照元ハッシュ（SourceSHA） | 照合HEAD d2355704dd108987ae3469eaa17d62fb38915254。C4 SourceSHA表の10 pathを参照。 |
| 試験区分（TestClass） | UNIT／FIXTURE／WIDGET sourceあり。旧debug／fixture試験は当時の限定証拠。本mappingのRuntime操作試験NOT_RUN。 |
| 証拠ID（Evidence ID） | `NONE`。 |
| 阻害要因（Blocker） | N09製品Acceptance OPEN。existing release blocker registry不変。 |
| 状態 | `OPEN`（ライフサイクル実装とConsumerの対応付け）。 |
| 残余 | Trusted production adapter／installed Runtime実作用、state transition・failure・Recoveryの製品経路。 |

### C5 — 評価ラボ

| E0-04項目 | 個別値 |
|---|---|
| 現ID | C5 Evaluation Lab／現行v4 N09、K05。 |
| 旧ID | C5／Phase 13。Dataset／Case／Evaluator／Experiment／Result／Comparison、旧Windows Broker／Owner-only dataset・決定論評価器・UIの局所statusを保持。 |
| 原文参照 | 旧rev1工程表C5。v4仕様S04 N09、S05 K05（LLM Judge単独保証不可）、工程R03-C／R06、施工指示I04-C。 |
| 対象 | Evaluation Dataset登録・Case／Evaluator定義・Experiment実行・Result閲覧・Runtime横断ComparisonとRegression Caseの別個の明示操作。 |
| 目的 | 決定論的Evaluatorを基礎に評価入力・対象・結果・比較のidentityを保持し、評価metadataをPermission／Approval／Release Authorityにしない。 |
| 生成元（Producer） | Ownerが作成・登録したDataset／Case／Evaluatorと明示Experiment要求、Brokerが返すResult／Comparison。 |
| 実Consumer | Evaluation Lab→EvaluationClient→Broker evaluation_control／evaluation_lab。Regression Case一覧・削除・Recoveryは別RegressionCaseClient／OwnerClientとして接続。 |
| 契約／Data | evaluation_case／dataset／registration／evaluator／experiment／result／public_result／comparison schemas、runtime IDs、bounded result。LLM judge単独は保証としない。 |
| 正常時作用 | Brokerが登録済みdataset／runtimeと要求を検査し、決定論評価を実行してExperiment result／ComparisonをUIへ返す。 |
| 否定時作用 | malformed／unknown Dataset・Runtime、無効Evaluator／Experiment、stale request、Authority metadata混入、private result露出を拒否・制限する。 |
| 停止／Recovery | Experiment失敗や不完全resultをPASS扱いしない。Audit／result binding不一致は表示・比較を保留。Task失敗やRuntime停止を評価resultで置き換えない。 |
| 標準OS候補 | Windows／macOS／Linux native Desktop。MobileからのmetadataがあってもEvaluation実行／比較同等性ではない。Linux native必須。N10標準環境参照。 |
| 配布物 | C5固有なし。各OSの候補artifactはN10へ。 |
| 既存実装 | `evaluation_lab.dart`、デスクトップ／共通の`EvaluationClient`、`evaluation_control.rs`／`evaluation_lab.rs`／`authority.rs`／`protocol.rs`、8件のスキーマ定義、Flutter画面の試験源。 |
| BUILD_GAP | Evaluation resultは同一AgentTask IDのTask test result・Diff・Auditへ結合されていない。Runtime横断比較／同一candidate acceptanceのinstalled evidenceはOPEN。旧C5 Dataset import gapは維持。 |
| 参照元ハッシュ（SourceSHA） | 照合HEAD d2355704dd108987ae3469eaa17d62fb38915254。C5 SourceSHA表の17 pathを参照。 |
| 試験区分（TestClass） | UNIT／FIXTURE／WIDGET sourceあり（Evaluation Lab／shared client）。今回試験NOT_RUN。旧検査数は旧candidateの履歴。 |
| 証拠ID（Evidence ID） | NONE。旧C5局所証拠をN09 acceptanceへ転用しない。 |
| 阻害要因（Blocker） | N09製品Acceptance OPEN。Registry 18／16／2とrelease_ready=falseを維持。新規Blockerなし。 |
| 状態 | `OPEN`（評価機能の`Consumer`対応付け）。 |
| 残余 | 同一Taskのtest／result／diff／Audit接続、Runtime横断の実結果比較、dataset import／versionとinstalled QA。 |

### C6 — 対話→回帰Case登録

| E0-04項目 | 個別値 |
|---|---|
| 現ID | C6 `Regression Case`の`Owner`登録。現行v4 N09／K05。`Dialogue`実`Task`はN01／N09の生成元境界。 |
| 旧ID | C6／Phase 14。完了対話から明示redaction済みRegression CaseをOwner登録。旧Windows ProtectedStore／metadata receipt／Owner delete-recoveryの限定履歴を保持。 |
| 原文参照 | 旧rev1工程表C6とdocs/specs/regression-case.md。v4仕様S04 N01／N09、S05 K05、工程R03-C／R06、施工指示I04-C。 |
| 対象 | Runtime Dialogue submission receiptとOwnerが明示redactionしたRegression Caseの登録、metadata listing、Owner delete／recovery。 |
| 目的 | 内容露出境界を守り明示された回帰入力だけを保存し、Dialogue結果からEvaluation Datasetや権限を暗黙生成しない。 |
| 生成元（Producer） | Runtime DialogueのBroker-bound submission receipt／Auditと、Ownerが確認したredacted Case payload。 |
| 実Consumer | Dialogue UI→RuntimeDialogueClient／RegressionCaseOwnerClient→Rust launcher native confirmation→Broker 回帰Case登録→Windows ProtectedStore／receipt。Case listはmetadata-only RegressionCaseClientから読む。 |
| 契約／Data | runtime_dialogue_submission_receipt、regression_case_registration／receipt／list／delete／recovery schemas。Request hash・source identity・redaction・Owner confirmation・Broker Audit bindingを維持。 |
| 正常時作用 | Native confirmation後、Brokerがreceipt／Audit／hash／redaction・current policyを再照合し、許可されたCaseを保存してhash-only receipt／metadataを返す。 |
| 否定時作用 | raw／secret content、tampered receipt、stale request／approval、hash mismatch、unauthorized delete／recoveryを拒否。Native confirmationはWindows本人性・OS account identity proofではない。 |
| 停止／Recovery | receipt／Audit／stored object mismatch時は成功表示せず、Brokerのfailure状態を維持。Case delete／recoveryは別のOwner-bound request。保存済みCaseが過去Approval／Permissionを与えない。 |
| 標準OS候補 | Windows Desktop source経路あり。macOS／Linux nativeの同等protected store／native confirmation／installed作用は別OS候補として未確立。Android／iOSは本C6のConsumerではない。Linux native必須製品対象、環境候補はN10参照。 |
| 配布物 | C6固有なし。ProtectedStore／Owner UIを含む正式OS artifact・installed lifecycleはN10と各OS contractへ接続し、現時点UNKNOWN／未成立。 |
| 既存実装 | `docs/specs/regression-case.md`、`Dialogue`／`RegressionCase` UI・client、Rust `regression_case`／`protocol`／`ipc_server`／`desktop_launcher`、`Schema`、`UNIT`／`FIXTURE`／`Conformance`の試験源。 |
| BUILD_GAP | installed製品上のOwner register／delete／recovery実作用と別OS保管・確認経路は未証明。保存CaseからC5 Datasetへの自動importなし（独立Owner操作／revisionが必要）。同一AgentTask ID／result artifactとの完全結合も未証明。 |
| 参照元ハッシュ（SourceSHA） | 照合HEAD d2355704dd108987ae3469eaa17d62fb38915254。C6 SourceSHA表の25 pathを参照。 |
| 試験区分（TestClass） | UNIT／FIXTURE／WIDGET／STATIC_CONFORMANCE sourceあり。過去Broker／ProtectedStore evidenceは当時の限定scope。今回installed Owner action NOT_RUN。 |
| 証拠ID（Evidence ID） | NONE。旧証拠を現v4 installed acceptanceへ昇格しない。 |
| 阻害要因（Blocker） | N09製品Acceptance OPEN。C6専用Blockerの直接帰属はUNKNOWN。Registryに追加／状態変更なし。 |
| 状態 | OPEN（source mapping）。旧C6の局所statusは履歴のまま。 |
| 残余 | 現candidate installed Owner action、OS別ProtectedStore／confirmation、Task ID／redacted result binding、Evaluation Datasetへの明示import path。 |

### C0〜C6 SourceSHA（現HEAD d2355704）

各hashは`git rev-parse HEAD:path`で得たGit blob ID。pathの存在・同一性だけを証明し、test実行・runtime接続・OS installed Acceptanceを証明しない。C0の三点セットは採用記録にpinされた原本SHA-256と対応する現行Git blobを示す。

| 旧ID | `Source path` | `Git blob SHA` |
|---|---|---|
| C0 | docs/QE_2026-11-13/01_D4Pocket_統合製品実装仕様書_v4.md | 352c53180cbad07888beb956aad0fb2a58074c58 |
| C0 | docs/QE_2026-11-13/02_D4Pocket_統合開発工程表_v4.md | d32dd251e5f7cbb97d92f6e7540d6b6ed0ecec05 |
| C0 | docs/QE_2026-11-13/03_D4Pocket_Codex施工指示書_v4.md | 49447973e88c50a09b2f4978adfed84fd464c6d0 |
| C0 | docs/QE_2026-11-13/採用記録.md | f159c1aa6ea5bc6e80e0877d8068f7e44da4c4d3 |
| C0 | release_blockers.registry.json | 244adc04a01e7b406c4ebcdeb829f7077d4aa35f |
| C0 | tooling/manifest.py | aa4525ff20293cb8641e5598a2439561a3bfbd3b |
| C0 | tooling/release_gate_check.py | 22a8d7bfe9adb259f44ce8501fb215652e1d58c6 |
| C0 | tooling/conformance_tests/run_conformance_skeleton.py | c6f31d8cb561b48513c6784e54c85969c9cb953e |
| C0 | tooling/validate_all.py | 2744f9a10702872f6dc0aad705f922466fc198fc |
| C1 | apps/desktop_flutter/lib/screens/workspace_inspector.dart | a02c95ec5062583f9c37523fbaf604a8d2da107e |
| C1 | apps/desktop_flutter/lib/screens/workspace_diff_panel.dart | 5e2154b870921a707b3da9a79c9f55055ce3cb1f |
| C1 | apps/desktop_flutter/lib/services/workspace_client.dart | 870b48e8f4bbd648bcfebc5886addca3784bcfaf |
| C1 | native/rust_helper/src/broker/workspace.rs | 68075d680a3c7d57785083e64d3d21523d79374c |
| C1 | native/rust_helper/src/broker/workspace_root.rs | 15f208e0b855b8cee53e878d2b27aaf457ea2937 |
| C1 | native/rust_helper/src/workspace_reader.rs | 3d635b50f85cffb6db382ed3c061ff6068238099 |
| C1 | native/rust_helper/src/workspace_diff.rs | 990041d9f2a67e2d372371e74fafb225e4e78e79 |
| C1 | native/rust_helper/src/broker/protocol.rs | 9b2076d5f31ca6d77f1f87973d1b394aaf751961 |
| C1 | specs/workspace_inspection_request.schema.json | a8210c300b2909c040188d0766633f1e30884e7a |
| C1 | specs/workspace_inspection_response.schema.json | 6cc9f80f4dd9b8bd9f71441bee845017428fa38a |
| C1 | specs/workspace_diff.schema.json | 1389ad85c48e263e688ab6dfa5276f96eb26fef0 |
| C1 | specs/agent_task_workspace_permission_request.schema.json | 05f7a7f97c7bac884ce706cc7f1533792c35e486 |
| C1 | apps/desktop_flutter/test/workspace_inspector_test.dart | 2f6e53c2a38d5e23c88f2ad91e079bf8be4808a5 |
| C1 | native/rust_helper/tests/workspace_reader.rs | 4723cbb121a0fe50d2e73ad6592ca9eac6c42a39 |
| C1 | native/rust_helper/tests/workspace_diff.rs | a467a45c233112c6f81ab8ffe504781e53aa937c |
| C2 | apps/desktop_flutter/lib/screens/history_screen.dart | dacc4c0ca15325c7f597cd1827a6455fb8ab5560 |
| C2 | apps/desktop_flutter/lib/screens/history_content_dialog.dart | 4602a29edc7a0c7c2b90e5b3b8a55155a28cfdb6 |
| C2 | packages/gui_shell_ui/lib/src/history_client.dart | 7f14ec2caf8b1e383bbf93483ae052100f921ae7 |
| C2 | native/rust_helper/src/broker/history_access.rs | c004056b45c2f26859bf7f70a9ff2b632e358523 |
| C2 | native/rust_helper/src/broker/execution_history.rs | 8eeb1c26d06a9e4b01806413a4bfa8c2a73357d6 |
| C2 | native/rust_helper/src/broker/dialogue.rs | c3c3fddb630dd06b7e6cc1c13b39f3d4f6f6a999 |
| C2 | native/rust_helper/src/broker/protocol.rs | 9b2076d5f31ca6d77f1f87973d1b394aaf751961 |
| C2 | specs/runtime_history_access.schema.json | 8187ddce917c3e80f40c8ce1c4b89059277f1cfd |
| C2 | specs/runtime_execution_history.schema.json | 5b260e9b6026758dc0a730d3381b4f7d72ba2e98 |
| C2 | specs/runtime_execution_history_page.schema.json | e6dab615707d30c9266df7dc67fc90df24d0ed9b |
| C2 | specs/runtime_dialogue_operation.schema.json | 8793ab0f59ea2320b7102afec7256cfe62264fc5 |
| C2 | specs/runtime_dialogue_request.schema.json | fe0bd153eaf3a0a2354b25cffc3ed9af934fbae1 |
| C2 | specs/runtime_dialogue_response.schema.json | 5e7220e8ce5954057ee270cd77cb4a68e1ef830f |
| C2 | specs/runtime_dialogue_comparison.schema.json | 874dd1158a6aa911ff30d85e141b4bd4927c91e3 |
| C2 | specs/runtime_agent_task_history_request.schema.json | 0bff21625a0f339367ae8352910b643b1f76b361 |
| C2 | specs/runtime_agent_task_history_access.schema.json | baddfeb38762fe0a04513cb1edcfcd258bd804ed |
| C2 | apps/desktop_flutter/test/history_screen_test.dart | 7828dc17fbf50cd0a2dca3255d90c3760030e58f |
| C2 | apps/desktop_flutter/test/history_content_test.dart | af34c1e4e61415916fe651a3f5096a95f946022b |
| C2/C6 | apps/desktop_flutter/test/runtime_dialogue_test.dart | 899912f92ee3751f8616632d434e04f95ee5ef77 |
| C2 | packages/gui_shell_ui/test/runtime_dialogue_test.dart | f997ac25c0d8227e2898dc6d1351d324911324e4 |
| C3 | apps/desktop_flutter/lib/screens/runtime_center.dart | d2c7af172073d2b10c76a6416b90f011f10e95db |
| C3 | apps/desktop_flutter/lib/services/runtime_resource_client.dart | d8da9ebee9afdd64b8d1d3afa0a82f3a4cb53cfd |
| C3 | packages/gui_shell_ui/lib/src/runtime_resource_client.dart | 3a8e611b94136e27f6d132e550a49bcbaf586eaf |
| C3 | apps/mobile_flutter/lib/screens/resource_overview.dart | 322af6bc2935c571017dab3f03b9c7a0cea7c616 |
| C3/C4 | native/rust_helper/src/broker/runtime_registry.rs | 9ff28610862586a599848d75f9e891a6c6cadf2f |
| C3 | native/rust_helper/src/broker/observation_center.rs | 62896e497a0d78267020db6bae2ecadf704fa1fb |
| C3 | native/rust_helper/src/broker/device_link.rs | c2e1ff88ff34a5764b2b4fb1d3f7abd8727e5c51 |
| C3 | specs/runtime_resource_query.schema.json | c7617ba9c2283255fb04b6cfa529ff7b04f6137f |
| C3 | specs/runtime_resource_observation.schema.json | 62f587b8c88d0c5c0bd316eaa3d0a15c9c1bd2d1 |
| C3 | tooling/canonicalization_vectors/runtime_resource_observation_decimal.json | ce00c0ea9571409a707c4e1993069c61761cb6a0 |
| C3 | apps/desktop_flutter/test/runtime_resource_test.dart | c44d66bb3797bf523be0f48f24d626068b8e4894 |
| C3 | packages/gui_shell_ui/test/runtime_resource_client_test.dart | 46c52c83eab712e5b84599c7e29eed660af0ed12 |
| C3 | apps/mobile_flutter/test/resource_overview_test.dart | 7695155627699274b8c0768c0de302344caa9eec |
| C4 | apps/desktop_flutter/lib/screens/runtime_center.dart | d2c7af172073d2b10c76a6416b90f011f10e95db |
| C4 | apps/desktop_flutter/lib/services/runtime_lifecycle_client.dart | 332e438a3d67fc01118ca6cc48745a372c44e01e |
| C4 | packages/gui_shell_ui/lib/src/runtime_lifecycle_client.dart | d34d773ec25f21bb682577faa7688d350921f059 |
| C4 | native/rust_helper/src/broker/runtime_lifecycle.rs | 2746ec301e9a47a5ececf1485ca7ad8a17304388 |
| C4 | native/rust_helper/src/broker/protocol.rs | 9b2076d5f31ca6d77f1f87973d1b394aaf751961 |
| C4 | specs/runtime_lifecycle_request.schema.json | 7c1ae69a0088cd32593481c7c803c4219baa6444 |
| C4 | specs/runtime_lifecycle_result.schema.json | 1695ec8c3cc2d2e492ff0ac326f8990ec00fe77e |
| C4 | apps/desktop_flutter/test/runtime_lifecycle_test.dart | 57cb17c453c3a50169d3f9e39de150c691e8e160 |
| C4 | packages/gui_shell_ui/test/runtime_lifecycle_client_test.dart | 1b1f274cdcd8d4cf92225db173eafcc09c0c9b8f |
| C5 | apps/desktop_flutter/lib/screens/evaluation_lab.dart | b74b3e96e7cd746874d1e19e05175548a5c66192 |
| C5 | apps/desktop_flutter/lib/services/evaluation_client.dart | d617dbbf10334754652a131e8612d3ca99aa61e6 |
| C5 | packages/gui_shell_ui/lib/src/evaluation_client.dart | 3fe93e4a19bcba7c825f2a6e887f96f00016aa75 |
| C5 | native/rust_helper/src/broker/evaluation_control.rs | 570485772c3fce43d170d031f1769b7617214d40 |
| C5 | native/rust_helper/src/broker/evaluation_lab.rs | 3645202da2597e5f63cf9ef96a891e9ae801e2b2 |
| C5 | native/rust_helper/src/broker/authority.rs | 067a36442138897aac396bdb588c2e01f90a5318 |
| C5 | native/rust_helper/src/broker/protocol.rs | 9b2076d5f31ca6d77f1f87973d1b394aaf751961 |
| C5 | specs/evaluation_case.schema.json | 5fed01706bb374697294ccb19eec1ae309d53f6a |
| C5 | specs/evaluation_dataset.schema.json | 8a90b7248e16c04d7b51ab9bca0a9a5f6c459ed1 |
| C5 | specs/evaluation_dataset_registration.schema.json | a79c103e32229dc47d263e62c05914624085610b |
| C5 | specs/evaluation_evaluator.schema.json | d2d5298afc927dfb82c73f136db7dce77bdccf13 |
| C5 | specs/evaluation_experiment.schema.json | e35d7a242a0a9d318900673df9fb900e841b93a2 |
| C5 | specs/evaluation_result.schema.json | f1b994e19dfaf1473a615cbf1347de22f72ccbed |
| C5 | specs/evaluation_public_result.schema.json | 77a285a53a0473fc4185d18b37b863f90be71ada |
| C5 | specs/evaluation_comparison.schema.json | 124af9c4e4def4c048751e911809b6f2ed3b577a |
| C5 | apps/desktop_flutter/test/evaluation_lab_test.dart | 9b068a34d9d22743bee74ba409c8d95bee938638 |
| C5 | packages/gui_shell_ui/test/evaluation_client_test.dart | 044d71b4b18b7efa8891a7df735f28e6ed674ac1 |
| C6 | docs/specs/regression-case.md | ce58a891facfca9f1e8e498e13bf983011350ba3 |
| C6 | specs/runtime_dialogue_submission_receipt.schema.json | 204b7871ab08539c16da2431541a5764b023f1fe |
| C6 | specs/regression_case_registration.schema.json | 04165813e2791196c38fe515c84b4e1adeefd000 |
| C6 | specs/regression_case_receipt.schema.json | 3defc85eab0e8ed4ebac4ba85a1a5b83de6016a4 |
| C6 | specs/regression_case_list_request.schema.json | 72ded3ae0e37214b442811ba973bdf4a490aa854 |
| C6 | specs/regression_case_list.schema.json | a7f99f9784b287b7bb21678bc12ab531debf5af0 |
| C6 | specs/regression_case_delete_request.schema.json | 7ed67c02cb4796da94adc6dd0b1674f4b8a91ca4 |
| C6 | specs/regression_case_delete_receipt.schema.json | d051f4e795f8c05051356b0f1d1dc89947db3633 |
| C6 | specs/regression_case_delete_recovery_request.schema.json | c69d71ef2f07a2a7090ba3d0d648448c60ca1c4e |
| C6 | specs/regression_case_delete_recovery_receipt.schema.json | b9b1de18f811bc03f38d5dfaa497f673e335e496 |
| C6 | native/rust_helper/src/broker/regression_case.rs | cf229213a54f0eaa512781fb417692506bf06775 |
| C6 | native/rust_helper/src/broker/protocol.rs | 9b2076d5f31ca6d77f1f87973d1b394aaf751961 |
| C6 | native/rust_helper/src/broker/ipc_server.rs | 58a404b920ef5905e174668ec2de0e04a9e8b044 |
| C6 | native/rust_helper/src/desktop_launcher.rs | 6a9f335de07fa474ca10043aea63ad6e988b572d |
| C6 | packages/gui_shell_ui/lib/src/runtime_dialogue_client.dart | 44f810aa2509a8a28ddf2933a5491528f3b07147 |
| C6 | packages/gui_shell_ui/lib/src/runtime_dialogue.dart | f78083b376f80d06f284387bb8b7aa2d769141bf |
| C6 | packages/gui_shell_ui/lib/src/regression_case_owner_client.dart | 1f1d10cecca70dff2075bed1e5f331267e0ea296 |
| C6 | packages/gui_shell_ui/lib/src/regression_case_client.dart | f14bd4aeda1d6f3262cdb1be4b924884d7128e5b |
| C6 | apps/desktop_flutter/lib/screens/runtime_dialogue.dart | 803edd0a687acdfd00e19898fbb5ba6b85ce1449 |
| C6 | apps/desktop_flutter/lib/services/runtime_dialogue_client.dart | a0853ee7327cafa60ee89b0c5ee49312b73c46c4 |
| C6 | apps/desktop_flutter/lib/services/regression_case_client.dart | c29054e8b6256b15f097bd5e7de57b9cf2117e3a |
| C6 | packages/gui_shell_ui/test/regression_case_owner_client_test.dart | 48a895c5a2f139eb2823df67f4aad3e1b8639748 |
| C6 | packages/gui_shell_ui/test/regression_case_client_test.dart | a30a002abf021730f0ba09e66126ebbb2a39e12a |
| C6 | tooling/conformance_tests/run_conformance_skeleton.py | c6f31d8cb561b48513c6784e54c85969c9cb953e |

上記C0〜C6の各行は現HEADのpath/blob対を照合したsource mappingである。E0-04は21 fieldを満たすが、C1〜C6の製品Acceptance・N09・N11・全体E0はOPENのまま。各Cの詳細なProducer→Consumer境界、差分、履歴statusを本節と上記crosswalkで保持する。

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

このcrosswalkは工程の現Consumerと証拠の所在・限界を特定するもの。既存CLOSEDの再試験、新たなEvidence ID発行、B5/B6の受入判定は行っていない。BUF-R01〜R09とBUF-T01〜T24の個別E0-04全21 fieldおよびSourceSHA／Consumerは記録済み。N11 Evidence ID・Blocker・OS／Artifactとの全ID結合は継続し、R/T要求の製品AcceptanceはOPENのまま。

## 実装ファイルの初期目録（CODE_PRESENTのみ）

下記は`d503a4ef6daac8942abd1cb011442ce28c78ec55`時点のパス存在・責務候補を読み取った初期目録である。名前・コードの存在だけではProducer→Consumer接続、installed/runtime実作用、Acceptanceを証明しない。各行の`WIRED`、実作用、正常/否定/Recovery、Evidence、OS別状態は未照合なら`UNKNOWN`。

| 線／要求 | Producer・UI候補 | Broker／native候補 | 実Consumer・残余 |
|---|---|---|---|
| A／N01・N02 | `apps/desktop_flutter/lib/screens/agent_center.dart`、Desktop／packageの`agent_task_execution_coordinator.dart`、`agent_task_client.dart`、`agent_mcp_task_controller.dart` | `native/rust_helper/src/adapters/codex_cli.rs`、`mcp_task_tool_stdio.rs`、`adapters/mcp_stdio.rs`、`broker/mcp_center.rs`、`broker/protocol.rs`、`broker/dialogue.rs` | Desktop sourceのAI提案→Broker実Tool→新規認可の同Agent後続Taskは`WIRED`。W02/G2 installed受入れは`OPEN`。Mobileは`BUILD_GAP`、macOS installed scopeは`UNKNOWN` |
| B／N03・N04 | `runtime_center.dart`、`agent_coordination.dart`、`agent_handoff.dart`、`a2a_connection_client.dart`、`a2a_artifact_handoff.dart` | `broker/runtime_registry.rs`、`adapters/codex_cli.rs`、`adapters/minidora.rs`、`broker/a2a_center.rs`、`a2a.rs`、`broker/host_center.rs` | N03: Codex CLIのAgent TaskはBrokerへsource接続（`WIRED`）、MINIDORAは対話のみでTask capabilityなし、第二Runtimeは`BUILD_GAP`。N04: Compare/Handoff/A2Aのsource consumerは`WIRED`だが、installed外部Agent作用・Workspace実体隔離・Artifact受渡しから新Task/Auditまでの一貫証拠は未成立。Hostはmetadata registryでremote Task consumerなし |
| D／N05 | `settings.dart`のRuntime/Agent/MCP Catalog更新→Compose受理→選択Tool操作、`compose_client.dart`、`mcp_connection_client.dart` | `broker/dialogue.rs`の現登録Adapter snapshot、`broker/compose_center.rs`のcompatibility check、`broker/mcp_center.rs::call_tool` | 現在Catalog→Manifest→同じBrokerのMCP実作用はsource `WIRED`。Capability metadataは未信頼宣言で、実Provider能力を独立測定するものではない。fixtureとinstalled evidenceは別区分 |
| E／N06 | Export UI→`export_client.dart`→Broker Manifest/Receipt→Windows/Linux Developer Exporter→product launcher | `broker/export_center.rs`、`desktop_launcher.rs`、`linux_desktop_launcher.rs`、`export_windows_product.py`、`export_linux_product.py` | 新規identityとManifest/Module planはsource `WIRED`。runtimeは構成を実消費せず、証拠がruntime consumption/pruningをfalseと明記。実独立製品・Installer・binary/resource除去は`BUILD_GAP` |
| D／N07 | `settings.dart`／`ai_edit_client.dart`からproposalをBroker記録し、`main.dart`経由でAgent CenterのTaskへ受け渡す。適用側は`AgentAiEditResultApplySnapshot`が元Task/result/proposalと別Session/Workspaceを再照合する。 | `broker/ai_edit_center.rs`と通常`dialogue.rs` Agent Task経路 | proposal-only/Audit、結果由来・別Workspace束縛、fresh Task preflightはsource `WIRED`。独立審査結果、Broker-controlled diff apply/rollbackは`BUILD_GAP`。 |
| F／N08 | `adapters/codex_cli.rs`のWorkspaceTaskScratch/TEMP mediation | `broker/agent_task_scratch.rs`、`dialogue.rs` Buffer record/Audit、`execution_history.rs` | Codex CLI限定のTEMP/scratch作用・回復記録はsource `WIRED`。広域原因Consumer、意味/副作用同等性、B5/B6統合は`BUILD_GAP`。初期のfilename-only検索によるUNKNOWNは本差分で精査済み。 |
| C／N09 | Agent Center、Workspace Inspector、History、Evaluation、Observability、Problems、Recovery、Audit、TraceのFlutter screen/service files | 対応候補として`broker/history_access.rs`、`execution_history.rs`、`evaluation_lab.rs`、`observation_center.rs`、`notification_center.rs`、`content_recovery.rs`、`audit.rs` | 個別画面・Broker moduleは存在する。実Task IDを共有したend-to-end表示、failure/restart/notification/stop consumerは`UNKNOWN` |
| G/H/I／N10 | Windows/Linux/macOS Desktop runners、`apps/mobile_flutter` Dart/Android Kotlin/iOS Swift source | `native/rust_helper/src/desktop_launcher.rs`、Linux/macOS launcher・worker、Mobile Device Link native services | Windows installerに複数PowerShell収集/配置scriptがある。`installer/linux/README.md`・`installer/macos/README.md`はあるが、当該installer root内にpackage builderは見当たらない。別locationを含む正規package経路は`UNKNOWN` |
| J／N11 | `docs/QE_2026-11-13/`、README/Checklist/Progress | `tooling/manifest.py`、`tooling/release_gate_check.py`、`tooling/conformance_tests/run_conformance_skeleton.py`、`release_blockers.registry.json` | v4正本/Registry/validatorの参照は同期済み。E0全ID/Consumer/Evidence validator・完全なrelease candidate結合は`UNKNOWN` |

## 新規要求の個別E0-04照合

### N01 実仕事

| E0-04項目 | 個別値 |
|---|---|
| 現ID | N01 実仕事 |
| 旧ID | P0／P1、K01／K08、W01／G1。C0〜C34とPhase 0〜45の個別ID接続は既存crosswalkに残し、各IDのE0-04完了へは昇格しない。 |
| 原文参照 | v4仕様S04「N01 実仕事」。仕様原本SHA-256=8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF。受入条件は一般利用入口の実Task、無権限拒否、Cancel／Deadline／Crash。 |
| 対象 | 使用者が選択したAgent／Runtime／Session／WorkspaceとTask指示から始まる一般利用製品経路、および実Task状態・結果・Audit・停止・復旧を消費するDesktop／Mobile面。 |
| 目的 | 使用者の目的をBroker統治下の実Runtime作用へ渡し、許可済みWorkspace内で結果を得て、現在Taskの状態・内容露出・停止・復旧を製品へ戻す。旧W01の限定成立を全OS・全配布・全Runtimeへ拡張しない。 |
| `Producer` | Desktop Flutter Agent CenterのTask要求作成・事前検査・開始・取消操作。入力はruntime／session／workspace ID、Task instruction、現在のPermission／Approval state。製品使用者の操作はBroker権限を直接生成しない。 |
| 実Consumer | DesktopはAgent Center→AgentTaskClient／AgentTaskExecutionCoordinator→BrokerTransport→Rust Broker protocol／dialogue→登録Adapter（Codex CLI）→Workspace／Task state・result／Audit→Coordinator／Agent Centerの候補経路がWIRED。Mobile Agent StatusはAdapter metadataの読取表示のみで、Task開始consumerではない。 |
| 契約／Data | specs/agent_task.schema.json、agent_task_request.schema.json、agent_task_scratch_recovery.schema.json。Runtime／Session／Workspace identity、instruction hash、独立Workspace Permission／Owner Approval receipt、一回Task request、status・result hash・bounded result projection、監査相関を使用。UI／Adapter metadataはAuthorityではない。 |
| 正常時作用 | Brokerが現在Runtime／Workspace binding、Adapter capability、Permission、本文・条件hashに束縛された別個Approvalを検査し、一回消費してAdapterを実行する。状態・結果はTask IDを基点に読戻し、結果本文は別の内容露出制御後に表示する。 |
| 否定作用 | 不正・staleな要求、未知／非対応Runtime、Permission／Approval欠落、Workspace不一致はBroker側で拒否する。cancel／deadline／crashはTaskを成功へ偽装せず停止・Recoveryへ渡す。Mobile Device Link allowlistにTask実行操作がないことを確認した。 |
| 停止／Recovery | Desktop sourceにcancel要求、期限付き実行、Broker再起動後のinterrupted Taskをsuspendedとして記録するRecovery journalがある。消費済みPermission／Approvalを復元しない。W01ではWindows限定条件のcancel／deadline／crash、子process停止、scratch回収を検収済み。全OS・一般配布候補の停止保証は未成立。 |
| 標準OS候補 | Windows：一般installed Desktop。Linux native：Ubuntu 24.04 LTS／x86_64を候補、保証範囲・Wayland等はUNKNOWN。macOS：標準installed条件UNKNOWN。Android：API 35 Google APIs／x86_64 Emulator候補。iOS：iPhone 16 Pro／iOS 18.5 Simulator候補。これらは候補であり、現行QA合格を意味しない。 |
| 配布物 | Windows portable ZIP／D4PKG01 update payload候補。Linux Ubuntu 24.04／amd64 portable tar.gz・bootstrap deb・D4PKG01候補。macOSはmacos-15 manual debug/UI workflow候補。Androidはdebug APK／AAB、iOSはSimulator app候補。通常利用者向けの全OS installer／配布／installed lifecycleは未成立。 |
| 既存実装 | Desktop: agent_center.dart、desktop services／agent_task_client.dart、agent_task_execution_coordinator.dart、agent_orchestration package、Rust broker/protocol.rs・dialogue.rs、adapters/codex_cli.rs、broker/agent_task_scratch.rs。Mobile: agent_status.dartはmetadata投影、device_link.rsは限定操作allowlist。SchemaとDesktop／Rust／Android／iOSの関連test sourceが存在する。 |
| BUILD_GAP | Desktop source配線とWindows W01の限定LIVE_PRODUCTを確認。現行統合候補の一般installed／配布・他OS一巡は未成立。Linux #286はUbuntu 22.04 hosted Xvfb／X11・normal Release・local no-auth fixtureの限定positive Task。macOS installed TaskはUNKNOWN。Android／iOSにTask開始経路がなくBUILD_GAP。 |
| `SourceSHA` | 照合HEAD=57d905c5bb89070a156e8f9ff83b60ecfa8acdb9。Git blob: apps/desktop_flutter/lib/screens/agent_center.dart=2472faeb31a64d624423c30a0a30d0f153d2e2a3; apps/desktop_flutter/lib/services/agent_task_client.dart=f54cefac29eba3dcf383bba19b4f64ee682e6217; apps/desktop_flutter/lib/services/agent_task_execution_coordinator.dart=5d8f48d0557bdcb1af838239abf9651893bce7f1; packages/agent_orchestration/lib/agent_task_client.dart=a31fbb9f980a0039f809fd763c91f055fe9d02d0; packages/agent_orchestration/lib/agent_task_execution_coordinator.dart=feb9aacb7ed77e6f4e76e9d9371290decca9cf7b; native/rust_helper/src/broker/protocol.rs=9b2076d5f31ca6d77f1f87973d1b394aaf751961; native/rust_helper/src/broker/dialogue.rs=c3c3fddb630dd06b7e6cc1c13b39f3d4f6f6a999; native/rust_helper/src/adapters/codex_cli.rs=e5c9d1581ae9a1c0a3ae7fcfa626fdca6fea6d28; native/rust_helper/src/broker/agent_task_scratch.rs=de743cd819fb4533c9c3070c4e7bcae82e42d938; apps/mobile_flutter/lib/screens/agent_status.dart=52e4907897c2bb3674bb8523204a787d3a6cb002; native/rust_helper/src/broker/device_link.rs=c2e1ff88ff34a5764b2b4fb1d3f7abd8727e5c51; apps/desktop_flutter/test/agent_task_client_test.dart=0477a0bbbb321082494b6b55711815d34419fdd9; native/rust_helper/tests/agent_task_production_e2e.rs=fdc7565aa58fa7466e6e24e5492321e8015567eb; native/rust_helper/tests/agent_task_crash_recovery_e2e.rs=23517fcfe8ef94b2357a94e04879d423c3360501; apps/mobile_flutter/android/app/src/androidTest/kotlin/com/example/gui_shell_mobile/NativeDeviceLinkProductFlowTest.kt=4f4a79e0bd80b1f5eb280857dec7a7d5df12258c; apps/mobile_flutter/ios/RunnerUITests/DeviceLinkProductUITests.swift=b74c1f4a846c07eed9b6ae9244ac18b4f1030eec; specs/agent_task.schema.json=a552389072088d6cd42b3588fefda332be4ae6b6; specs/agent_task_request.schema.json=cb25bb3684656bfd670ea5832bbb7fc5988845b6; specs/agent_task_scratch_recovery.schema.json=40f1eac71cd9645396ae7230d49cad25f3b77237。 |
| `TestClass` | 現source／契約／fixture分類の存在確認のみ。今回のruntime testはNOT_RUN。既存W01/G1およびLinux run #286は原記録上の限定LIVE_PRODUCT履歴で、現行候補のOS別QAへ転用しない。v4標準TestClassへの対応付けはN11側で行う。 |
| `Evidence ID` | Windows W01/G1: ../QE_2026-11-05/W01_G1_通常ReleaseAgentTask証拠_20261107.md。Linux run #286: ../QE_2026-11-09/進捗.md（export evidence SHA-256=8fb4d23f8f7f54b8e894f007d0e9e39747acdee6da8719d3971d4b62980f0f93、package SHA-256=1282ec26c9aa64002475dfea0451978515a000a77e34ec75ee66e71e293ac030。artifactは保持されていない）。 |
| `Blocker` | 既存Registryとの関係: windows_broker_installed_smoke、windows_evidence_provenance_isolation、linux_native_product_support、comprehensive_extension_rev1_completion、rev2_mobile_device_evidence、rev2_mobile_flutter_native_device_link_boundary。現在の状態は各Registry記録を維持し、新規Blockerは追加しない。 |
| 状態 | OPEN（N01のE0-04 mappingは本記録で全21項目を充足。N01製品AcceptanceはOPEN）。 |
| 残余 | 現行Release候補のWindows installed provenance／一般installer、Linux native標準GUI・配布、macOS installed Task、Android／iOS Task consumer、全OS標準保証・artifact identity、同一候補でのTask／停止／Recovery／結果露出／Auditの結合証拠は未成立またはUNKNOWN。README.mdの2026-11-10記載（Linuxの限定normal Release履歴）は日付・証拠範囲のまま扱う。現行v4 CLAIM.md／COMPATIBILITY_MATRIX.mdのinstalled QA未成立と混同せず、保証表示の整合はN11で照合する。 |

### N02 `AI→Tool→AI`

| E0-04項目 | 個別値 |
|---|---|
| 現ID | N02 AI→Tool→AI結果依存Task |
| 旧ID | P1／P6、K07／K08、C8／C9、Phase 18／19／35、W02／G2。既存crosswalkの他ID接続は履歴参照にとどめ、個別Acceptanceへ昇格しない。 |
| 原文参照 | v4仕様S04「N02 AI→Tool→AI」。仕様原本SHA-256=8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF。AI自発MCP Tool要求、現在Catalog／認可、実作用、結果、同AIの結果依存後続Taskを要求する。 |
| 対象 | DesktopのCodex CLI Agent TaskからMCP提案、Rust Brokerの現在Catalog／権限／Audit検査、実MCP Server作用、Broker内結果context、同一Runtime／Session／Workspaceの後続Taskへ至る経路と、結果／状態を投影するAgent Center。5 OSのTask／MCP接続候補も照合対象とする。 |
| 目的 | AIが必要と判断したMCP ToolをBroker統治下で一度実行し、その未信頼結果を元Agentの別Taskへ渡す。後続Taskは常に新規Workspace Permission／Owner Approvalを要求し、Flutter操作・MCP metadata・Tool応答からAuthorityを継承しない。 |
| `Producer` | Codex CLI Task内の`d4_submit_mcp_tool_request` stdio ToolがServerID／ToolID／名前／argumentsの提案を発行する。現在Task完了後、`AgentTaskExecutionCoordinator`が`AgentMcpTaskController`を起動する。後続はFlutterの連続実行ボタンではなく、Broker結果を受けたこの統治済みCoordinatorが開始する。 |
| 実Consumer | Desktop sourceはCodex CLI→`mcp_task_tool_stdio.rs`の提案queue→完了Taskのhash／identity照合→Agent Center／orchestration coordinator→現在MCP Catalog→Rust `MCP Tool実行`→`mcp_stdio`の監督子process→hash-only receipt／一回結果context→新規Permission／Approval付き同Agent Task→Agent Center状態投影までWIRED。Mobile Agent Statusはmetadata表示で、Device Link allowlistにTask開始／MCP Tool実行Consumerがない。 |
| 契約／Data | `specs/agent_mcp_tool_request.schema.json`、`agent_task_request.schema.json`、`mcp_contract.schema.json`、`mcp_tool_call.schema.json`、`mcp_tool_call_receipt.schema.json`、`agent_task_owner_approval.schema.json`、`agent_task_workspace_permission_request.schema.json`、`agent_task_content_exposure.schema.json`。提案は未信頼。Broker結果contextはsource Task ID／result hash、Runtime／Session／Workspace、Server／Tool、Tool result hash、context hashへ結合し、5分期限・最大8件のone-shot状態を持つ。本文は通常receiptではhash-only、後続Task時にBrokerが未信頼データとして注入する。 |
| 正常時作用 | Coordinatorは完了・現行のsource Taskと結果projectionを照合し、現在Catalogの接続Server／Tool ID／名前を再解決する。native Owner確認後、Brokerが現行record／失効／Tool schema／一回Permission／Auditを再評価してMCP子processへ一回dispatchする。実結果をsource binding付きcontextとして保持し、別の新規PermissionとOwner Approvalを取得して同じAgentの後続Taskを開始する。成功した後続Taskでcontextを消費する。 |
| 否定作用 | malformed／重複・不在・切替済みCatalog、source Task未完了／hash不一致、Runtime／Session／Workspace不一致、期限切れ／消費済みcontext、既存Permission／Approvalの残存はTool作用または後続Taskを拒否／停止する。Tool error／context欠落は後続Taskを開始しない。Broker側のnative Owner経路以外からの実行とDevice Link経由の昇格を許さない。外部Tool結果・資格値をAuthorityやAudit本文へ昇格しない。 |
| 停止／Recovery | 各境界で`isCurrent`を再確認する。Session失効時は後続Taskを開始せず、開始競合時はBrokerへcancel要求し、停止未確認／cancel失敗を成功扱いしない。Brokerは開始Audit確定失敗時に外部dispatchしない。結果contextは揮発性で、期限／容量超過／成功後消費で失効し、Broker再起動後の再利用はできない。MCP外部作用の曖昧な成否は自動再送せず、既存Brokerの停止・隔離・Auditを使う。物理的な外部副作用の巻戻しは保証しない。 |
| 標準OS候補 | Windows：一般installed Desktop候補、W02/G2は未成立。Linux native：Ubuntu 24.04 LTS／x86_64候補、#286はUbuntu 22.04 hosted Xvfb／X11・local no-auth fixtureの履歴に限定。macOS：macos-15開発workflow候補、installed同一経路はUNKNOWN。Android：API 35 Google APIs／x86_64 Emulator候補、Task／MCPはBUILD_GAP。iOS：iPhone 16 Pro／iOS 18.5 Simulator候補、Task／MCPはBUILD_GAP。どれも本行の製品Acceptance合格を意味しない。 |
| 配布物 | Windows portable ZIP／D4PKG01候補、Linux Ubuntu 24.04／amd64 portable tar.gz・bootstrap deb・D4PKG01候補、macOS macos-15 manual debug／UI workflow候補、Android debug APK／AAB候補、iOS Simulator app候補。通常利用者向け正式配布・署名・installed lifecycleはN10で未成立として管理する。 |
| 既存実装 | Flutter: `apps/desktop_flutter/lib/screens/agent_center.dart`、desktop servicesの`agent_task_client.dart`／`agent_task_execution_coordinator.dart`／`agent_mcp_task_controller.dart`。共有package: `packages/agent_orchestration/lib/agent_task_client.dart`／`agent_task_execution_coordinator.dart`／`agent_mcp_task_controller.dart`／`mcp_connection_client.dart`。Rust: `adapters/codex_cli.rs`、`mcp_task_tool_stdio.rs`、`adapters/mcp_stdio.rs`、`broker/mcp_center.rs`、`broker/protocol.rs`、`broker/dialogue.rs`。Mobile比較: `apps/mobile_flutter/lib/screens/agent_status.dart`、`broker/device_link.rs`。 |
| BUILD_GAP | Desktopはソース配線WIREDだが、現v4同一installed候補で実Codexが自発Tool要求し、実MCP Server作用→結果依存follow-up→永続Audit／Recoveryまで成立するW02/G2受入れはOPEN。現指示書のW02-B02 WIRED／B03 fixture PASS／B04 installed実仕事NOT_RUNと、UI/message-dispatch停止原因UNKNOWNを保持する。過去のWindows fixture／synthetic Workspace記録をLIVE_PRODUCTにしない。Linux #286も外部MCPと結果依存Taskを証明しない。macOS installedはUNKNOWN、Android／iOSはTask／MCP実行ConsumerのBUILD_GAP。N02通常Release能力や全OS保証は未成立。 |
| `SourceSHA` | 照合HEAD=798536884154527c4080ba3b33961442ef045f0c。Git blob: `apps/desktop_flutter/lib/screens/agent_center.dart`=2472faeb31a64d624423c30a0a30d0f153d2e2a3; `apps/desktop_flutter/lib/services/agent_task_client.dart`=f54cefac29eba3dcf383bba19b4f64ee682e6217; `apps/desktop_flutter/lib/services/agent_task_execution_coordinator.dart`=5d8f48d0557bdcb1af838239abf9651893bce7f1; `apps/desktop_flutter/lib/services/agent_mcp_task_controller.dart`=fba6642904d98890339197bef269164970031830; `packages/agent_orchestration/lib/agent_task_client.dart`=a31fbb9f980a0039f809fd763c91f055fe9d02d0; `packages/agent_orchestration/lib/agent_task_execution_coordinator.dart`=feb9aacb7ed77e6f4e76e9d9371290decca9cf7b; `packages/agent_orchestration/lib/agent_mcp_task_controller.dart`=fa46ff071cfb306d3e463a9ad2c79b9149dab6f5; `packages/agent_orchestration/lib/mcp_connection_client.dart`=71b17973216160bdd2bc447243ba0b04fd0fec63; `native/rust_helper/src/adapters/codex_cli.rs`=e5c9d1581ae9a1c0a3ae7fcfa626fdca6fea6d28; `native/rust_helper/src/mcp_task_tool_stdio.rs`=6107890e27be2b9f6bef93a9d46828a488aa1bf1; `native/rust_helper/src/adapters/mcp_stdio.rs`=b7e08327541ad75bca53bb1563e0e20d576d9b66; `native/rust_helper/src/broker/mcp_center.rs`=8f9ab897bdc625e448241ebd9b46f995cd1f22f6; `native/rust_helper/src/broker/protocol.rs`=9b2076d5f31ca6d77f1f87973d1b394aaf751961; `native/rust_helper/src/broker/dialogue.rs`=c3c3fddb630dd06b7e6cc1c13b39f3d4f6f6a999; `native/rust_helper/src/broker/device_link.rs`=c2e1ff88ff34a5764b2b4fb1d3f7abd8727e5c51; `apps/mobile_flutter/lib/screens/agent_status.dart`=52e4907897c2bb3674bb8523204a787d3a6cb002; `apps/desktop_flutter/test/agent_mcp_task_controller_test.dart`=5c165d5362eea4906b6e739407b0d0e75c34485c; `apps/desktop_flutter/test/mcp_connection_center_test.dart`=c81a37ca78b15e57c838f48519aa3e4abb6d6665; `apps/desktop_flutter/test/agent_task_client_test.dart`=0477a0bbbb321082494b6b55711815d34419fdd9; `native/rust_helper/tests/agent_task_production_e2e.rs`=fdc7565aa58fa7466e6e24e5492321e8015567eb; `native/rust_helper/tests/agent_task_crash_recovery_e2e.rs`=23517fcfe8ef94b2357a94e04879d423c3360501; `specs/agent_mcp_tool_request.schema.json`=f3aa71661ac1dadf453954141802a7663755da9d; `specs/agent_task_request.schema.json`=cb25bb3684656bfd670ea5832bbb7fc5988845b6; `specs/agent_task.schema.json`=a552389072088d6cd42b3588fefda332be4ae6b6; `specs/agent_task_owner_approval.schema.json`=d190c9a8cc3d715eb199459c37c14e5de0ad6ad8; `specs/agent_task_workspace_permission_request.schema.json`=05f7a7f97c7bac884ce706cc7f1533792c35e486; `specs/agent_task_content_exposure.schema.json`=a85b99c0213d18daaab3d5d863adcd3661f379a0; `specs/mcp_contract.schema.json`=733f1ecf2ad968e98c7d0c3557e5ec897a4cac23; `specs/mcp_tool_call.schema.json`=54c044cdfb53588ea5bab4bd056404372dc39acd; `specs/mcp_tool_call_receipt.schema.json`=0049a393104b035bbf047e653e962754bc4009b3。全30 Git blob pathを照合済み。 |
| `TestClass` | `UNIT`／`FIXTURE`: `agent_mcp_task_controller_test.dart`はAI proposal、Catalog差替え、Tool error、fresh Permission／Approval、stale Session、cancel競合、通常Taskの非介入をmock Broker transportで検査。`mcp_center.rs`／`mcp_stdio.rs`／`protocol.rs`内の既存testはresult binding／untrusted注入／期限・one-shot replay／Audit／fixture Serverを検査。`mcp_connection_center_test.dart`と`agent_task_client_test.dart`もFixture／Client境界。`agent_task_production_e2e.rs`／`agent_task_crash_recovery_e2e.rs`は共通Task・Recoveryの試験源でありMCP installed acceptanceの代替ではない。本E0 mappingでは製品試験を再実行していない。 |
| `Evidence ID` | NONE（このmapping作業で製品Evidence IDを発行しない）。`W02/G2`、W02-B02／B03／B04と過去のregistry evidenceは、それぞれ記録済みscopeの履歴参照に限定する。 |
| `Blocker` | W02/G2はOPEN。`broker_codex_task_current_cli_followup`等の旧registry記録はsynthetic Owner／Workspace・in-process Broker等の限定証拠として保持し、現installed N02受入れへ昇格しない。現Release Registry 18件中16 unresolved／2 resolved、`release_ready=false`を維持する。N02専用blocker IDとの対応はN11照合待ちでUNKNOWN。新規Release blockerは追加しない。 |
| 状態 | OPEN（N02 E0-04 mappingは本記録で全21項目を充足。N02製品AcceptanceとE0全体はOPEN）。 |
| 残余 | 現行候補の同一installed source／artifactにおける実CodexのAI自発Tool request、実MCP Server副作用、native Owner／新規権限、結果依存follow-up、durable Audit／Recoveryの一体受入れ。Windows UI hang／message-dispatchのroot cause、macOS installed scope、Mobile Task/MCP実作用、5 OS配布・通常利用者経路は未成立。 |

### N03 異種Runtime

| E0-04項目 | 個別値 |
|---|---|
| 現ID | N03 異種Runtime |
| 旧ID | P4→N03、P7→N03/N04、C15〜C19、Phase 2〜4／12／15／26、K02／K06／K11／K12／K18、W03／G3。旧P4/P7 Product BuildのCLOSED・部分試験は当時の範囲に保持し、N03受入れへ移さない。N04 Compare/Handoff/A2Aの完了条件は別行。 |
| 原文参照 | v4仕様S04「N03 異種Runtime」・S05のP4/P7・K02/K06/K11/K12/K18、工程表R03線B・W03/G3、施工指示I04-B。仕様原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`。要求は少なくとも2種類の独立した実Runtimeで実Task・失敗・権限を示すこと。特定LLMの内蔵は要求しない。 |
| 対象 | Runtime/Provider/Adapterの登録・能力選択から、共通Agent Task要求、実Runtime実行、結果／失敗、Broker権限・Audit、製品表示へ至る現行経路。Desktop Codex CLIとMINIDORA対話Adapter、Runtime Center、Agent Center、Task client、Broker登録・dispatchを確認し、他の標準OS候補は同一要件の別targetとして照合する。 |
| 目的 | Runtime固有の違いを境界付きAdapterで統一Task/Result/Capabilityへ接続し、2種類以上の独立Runtimeが実Taskを行える製品能力を成立させる。Provider/Model変更だけを独立Runtimeとして数えず、metadata・Schema・fixtureからTask能力やAuthorityを作らない。 |
| 生成元（`Producer`） | Desktop Agent CenterのCLI Runtime／Workspace登録UIは現在Codex CLI経路のみを提示する。Runtime CenterはRuntime状態・対話等の操作面で、MINIDORA登録経路はRust起動時の`--minidora-runtime`引数によるもの。Agent Task要求はAgent Center／`AgentTaskClient`から生成される。 |
| 実Consumer | Codex CLI: Agent Center→`AgentTaskClient`→Flutter/native Broker transport→Rust `protocol.rs`／`dialogue.rs`→登録Codex Adapter→Task/Workspace/resultへsource接続（`WIRED`）。MINIDORA: `minidora.rs`は`実行系Adapter`の対話接続を実装するが、Agent Task overrideなし。Trait既定の`AgentTask実行対応=false`／`AgentTask非対応`によりTask要求を拒否する。Python runtime contract/catalogとJSON Schemaは宣言・参照コードであり、Rust production Agent Taskの第二Runtime consumerではない。 |
| 契約／Data | `specs/runtime.schema.json`、`agent_runtime.schema.json`、`agent_adapter.schema.json`、`agent_cli_runtime_workspace_registration.schema.json`、`agent_task_request.schema.json`、`agent_task.schema.json`、Rust `実行系Adapter`／Broker runtime registry・protocol。Runtime/Adapter ID、Task要求、Workspace binding、Capability/Resultはデータ契約で、Provider metadata・Capability申告はAuthorityではない。 |
| 正常時作用 | 現行sourceではCodex CLI normal Release TaskがWindows/Linuxで、登録済みWorkspace-write interface条件付きでBroker経由実行可能。MINIDORAの通常対話は別経路で応答するが、Agent Taskとしては未対応を明示して開始しない。実行可能な独立第二Runtimeは現状存在せず、N03正常Acceptanceは未成立。 |
| 否定時作用 | Adapter trait既定がTask未対応を返し、Brokerは`AgentTask実行非対応`として拒否する。MINIDORA metadata／Runtime catalogやSchemaの能力宣言はTask実装を追加しない。Codex CLI source capabilityはunit/debug／fixture条件で分離され、macOS等の未対応条件をunsupportedとする。未対応Runtime、能力不足、登録／Workspace不一致は成功や権限へ昇格しない。 |
| 停止／Recovery | Codex CLI既存Task経路にはBrokerの期限・取消・process監督・Task Recoveryが接続される。MINIDORAへのAgent Taskは未開始のままfail-closed。どちらもこのE0 source mappingで再実行しておらず、現在のinstalled候補・第二Runtimeでの停止／Recoveryは証明しない。失敗・結果不明をcompletedへ昇格せず、fresh Permission／Approvalを既存Brokerが評価する。 |
| 標準OS候補 | Windows Desktop（installed製品のversion/build/archはN10でUNKNOWN）。Linux nativeは必須製品対象、Ubuntu 24.04 LTS／x86_64を現在の標準候補とし、WSL/hosted証拠をnative受入れへ流用しない。macOS installed標準条件UNKNOWN。Android API 35 Google APIs／x86_64 Emulator、iOS iPhone 16 Pro／iOS 18.5 Simulatorは候補。5 OSの第二Runtime Task/失敗/権限parityはいずれも未受入れ。 |
| 配布物 | Windows portable ZIP／D4PKG01、Linux Ubuntu 24.04／amd64 portable tar.gz・bootstrap deb／D4PKG01、macOS macos-15 debug/UI workflow、Android debug APK/AAB、iOS Simulator appは候補としてN10へ接続する。正式な利用者向け配布物・OSごとのRuntime同梱／導入・installed lifecycleは未成立で、形式や版が未固定の値はUNKNOWN。 |
| 既存実装 | Desktop: `agent_center.dart`、`runtime_center.dart`、`runtime_dialogue_client.dart`、`agent_task_client.dart`。Rust: `adapters/codex_cli.rs`、`adapters/minidora.rs`、`broker/dialogue.rs`、`broker/protocol.rs`、`broker/runtime_registry.rs`、`broker/adapter_center.rs`。Reference/declaration: `packages/agent_runtime/contract.py`、`packages/runtime_catalog/catalog.py`。Mobile metadata: `agent_status.dart`、`broker/device_link.rs`。Schema/Test sourceはSourceSHA欄参照。 |
| BUILD_GAP | 第二の独立したTask-capable実Runtimeとその登録／選択／Broker dispatch／統一結果・失敗／Permission・Approval・Auditの製品経路がない。MINIDORAは対話のみで、Task非対応は正しいfail-closed状態。複数Modelや同一Codex CLI設定を別Runtimeに数えない。5 OSの通常配布・installed Runtime接続・実作用受入れも未成立。新しいAcceptanceやRelease blockerは追加せず、N03/W03/G3のOPENを維持する。 |
| `SourceSHA` | 照合HEAD=`de147064e5d8d3424c8a0c1fa227862a3a2365b2`。Git blob: `apps/desktop_flutter/lib/screens/agent_center.dart`=`2472faeb31a64d624423c30a0a30d0f153d2e2a3`; `apps/desktop_flutter/lib/screens/runtime_center.dart`=`d2c7af172073d2b10c76a6416b90f011f10e95db`; `apps/desktop_flutter/lib/services/runtime_dialogue_client.dart`=`a0853ee7327cafa60ee89b0c5ee49312b73c46c4`; `apps/desktop_flutter/lib/services/agent_task_client.dart`=`f54cefac29eba3dcf383bba19b4f64ee682e6217`; `native/rust_helper/src/adapters/codex_cli.rs`=`e5c9d1581ae9a1c0a3ae7fcfa626fdca6fea6d28`; `native/rust_helper/src/adapters/minidora.rs`=`31c756ac7c9a264d73ed0e9654dbae599be83921`; `native/rust_helper/src/broker/dialogue.rs`=`c3c3fddb630dd06b7e6cc1c13b39f3d4f6f6a999`; `native/rust_helper/src/broker/protocol.rs`=`9b2076d5f31ca6d77f1f87973d1b394aaf751961`; `native/rust_helper/src/broker/runtime_registry.rs`=`9ff28610862586a599848d75f9e891a6c6cadf2f`; `native/rust_helper/src/broker/adapter_center.rs`=`66575bb77e4a375dca4f8488f97df442960bb5a9`; `packages/agent_runtime/contract.py`=`9e05ca1dd63883ad21d1871c62ddfdfef307d82f`; `packages/runtime_catalog/catalog.py`=`003d3c533cbb48bb5f6520cb9b7a3566f2493748`; `apps/desktop_flutter/test/widget_test.dart`=`f4e9c7b93f832ba2a2cefee779fd12426f240b04`; `apps/desktop_flutter/test/agent_task_client_test.dart`=`0477a0bbbb321082494b6b55711815d34419fdd9`; `native/rust_helper/tests/agent_task_production_e2e.rs`=`fdc7565aa58fa7466e6e24e5492321e8015567eb`; `native/rust_helper/tests/agent_task_crash_recovery_e2e.rs`=`23517fcfe8ef94b2357a94e04879d423c3360501`; `apps/mobile_flutter/lib/screens/agent_status.dart`=`52e4907897c2bb3674bb8523204a787d3a6cb002`; `native/rust_helper/src/broker/device_link.rs`=`c2e1ff88ff34a5764b2b4fb1d3f7abd8727e5c51`; `specs/runtime.schema.json`=`bdef33f26fe06b334eb6f94124930fd08594bbc9`; `specs/agent_runtime.schema.json`=`b8e031e53f582048211f0e1a6deeb2e5cb87e4fb`; `specs/agent_adapter.schema.json`=`e0792589a2bae168ba221af4f4f9abb505815c12`; `specs/agent_cli_runtime_workspace_registration.schema.json`=`32a09f2ce33a5a215d64350ee22e24e93f9037c7`; `specs/agent_task_request.schema.json`=`cb25bb3684656bfd670ea5832bbb7fc5988845b6`; `specs/agent_task.schema.json`=`a552389072088d6cd42b3588fefda332be4ae6b6`。上記24件すべてpath/blobを照合した。 |
| 試験区分（`TestClass`） | source内UNIT／FIXTURE: Codex capability・command試験、Broker `dialogue.rs`のTask capability/unsupported拒否・metadata authority否定、MINIDORAのlocal dialogue fixture、Flutter provider UIおよびunsupported client tests。`agent_task_production_e2e.rs`／`agent_task_crash_recovery_e2e.rs`は既存の単一Codex Task試験源。二つ目のRuntime実Task試験はない。本mapping単位ではいずれも再実行していない。 |
| 証拠ID（`Evidence ID`） | `NONE`。W01/G1はWindows normal Release Codex CLI単独の限定履歴であり、二つ目のRuntime証拠ではない。旧P4/P7のfixture・部分接続履歴もv4 N03 Acceptanceへ転記しない。 |
| 阻害要因（`Blocker`） | W03/G3はOPEN。Registryは18件（16 unresolved／2 resolved）、`release_ready=false`を維持する。`linux_native_product_support`等はOS横断条件として関係し得るが、N03への直接帰属はN11照合までUNKNOWN。新規Blockerは作成しない。 |
| 状態 | OPEN（N03のE0-04 mappingは21項目を充足。N03製品Acceptance、W03/G3、E0全体はOPEN）。 |
| 残余 | 独立した第二Task-capable Runtimeを選択・登録し、通常Broker認可下で実Task／失敗／結果／権限／Auditを成立させること。MINIDORAを使う場合は正式なAgent Task Adapterとその実consumerが必要。各OS標準配布・installed Runtime条件はN10と結合する。N04 Compare/Handoff/A2Aは別要求として残る。 |

### N04 複数Agent比較・成果受渡し・`A2A`

| E0-04項目 | 個別値 |
|---|---|
| 現ID | N04（複数Agentによる比較、成果受渡し、外部接続） |
| 旧ID | P2／P3→N04、P7→N03/N04、C15〜C19、Phase 7／8／24〜27、K02／K05／K11／K12、W03／G3。旧P2/P3/P7のProduct Build CLOSED・部分証拠は当時の条件へ固定し、現行N04へ移さない。 |
| 原文参照 | v4仕様S04「N04 Compare/Handoff/A2A」・S05 P2/P3/P7・K02/K05/K11/K12、工程表R03線B/C・W03/G3、施工指示I04-B/C。仕様原本SHA-256=`8CEEDF21F881A783E35EE8642673FAAEEF949465343DF65E63DC21B04EE5A3EF`。要求は独立Session/Workspace実行、diff/test/result選択、実外部A2A作用、明示Artifactの限定転送、権限非継承、外部metadata未信頼。 |
| 対象 | Desktop Agent Centerの並列Compare・結果選択・選択結果の新規Task適用、内部Agent Handoff package、A2A Agent Card/Message/Stream/Task状態/Artifact取得・選択、選択Artifactを受信Agentの新規Taskへ渡す経路、およびHost Centerの境界。Windows/macOS/Linux/Android/iOSの標準候補を照合し、A2AやHostのmetadata表示と実外部作用を分ける。 |
| 目的 | Agent A/BのTask・Workspace・結果を独立に比較し、明示選択した公開成果だけを新しい受信Taskへ渡す。A2A外部作用もBroker統治下に置き、外部metadataをTrustにせず、sourceからPermission／Approval／Credential／Authority／hidden stateを継承しない。 |
| 生成元（`Producer`） | Agent CenterのCompare操作が二つの別Session／Workspaceと同一Task本文を選び、個別preflight後に並列開始する。結果選択とApply先は別操作。Internal/A2A Handoffは完了source Task・full resultの明示公開承認と、選択Artifactからbounded packageを作る。A2A CenterはAgent ID／Card URIとMessage／Stream／Task照会を要求し、Host Centerは公開Host metadataを登録・選択する。 |
| 実Consumer | CompareはAgent Center→`agent_coordination.dart`→Task client→別々のBroker AgentTask inspect/start・状態取得へsource接続（`WIRED`）。片側失敗を他側へ伝播せず、選択結果Applyは直接filesystemを書かず第三の対象Session/Workspace向け新規Taskへ送る。Handoffは`agent_handoff.dart`／`a2a_artifact_handoff.dart`→Broker preflight→通常AgentTask consumerへ接続（`WIRED`）。A2AはFlutter client→既存Broker `a2a_center.rs`→`a2a.rs` HTTP actionへ接続（`WIRED`）。Host Center→`host_center.rs`はmetadata registryのみでremote Task/Workspace consumerを持たない。Source配線はinstalled external Agent受入れを意味しない。 |
| 契約／Data | `specs/agent_handoff.schema.json`、`agent_handoff_package.schema.json`、`agent_task_request.schema.json`、`agent_task.schema.json`、`a2a_contract.schema.json`、`a2a_connection.schema.json`、`a2a_connection_receipt.schema.json`、`a2a_connection_list.schema.json`、Message／Stream／GetTaskの各要求・応答Schema、Host登録／一覧／能力／切替Schema。送信元／受信先Task・Session・Workspace、結果／接続先／送信内容の`hash`、選択済み完了成果物参照、native `Owner` receipt、一回`Permission`、`Audit`参照を結合する。成果受渡しpackageへ`Authority`／`Credential`／`Approval`を含めない。 |
| 正常時作用 | Compareは別Session／Workspaceを事前検査し同一Taskを並列Broker Taskとして開始、結果を各Task IDから投影する。test結果はAgent申告を未検証として表示し、ユーザーが選んだ結果のみ別の受信Taskへ渡す。Handoffはsource結果・Artifactをbounded化し、target側の現在状態を事前検査して新規AgentTask requestへ渡す。A2Aは現行Brokerのnative Owner確認・現在要求束縛・一回Permission・Audit後にCard取得／Message／Stream／Task照会を行い、選択済みcompleted text Artifactのみ新しいtarget Taskへ送る。Host登録・表示切替はmetadata操作に留まる。 |
| 否定時作用 | Compareは同一Session／Workspace、Task本文不一致、参照不足、staleなTask/result/target stateをdispatchまたはApply前に拒否する。片側start/query failureは他方を成功扱いせず、曖昧な開始を再送しない。A2A/Handoffはmalformed・改変・未完了・未承認・hash/size/path/schema/endpoint不一致、Owner拒否、古いrequestを拒否する。外部Agent Card／Capability／Host metadataはTrust/Authorityを生成しない。受信Taskは必ず新しいPermission／Approvalを得る。remote作用の成否不明は自動再送しない。 |
| 停止／Recovery | CompareはSessionごとの取消gateを使い、A/Bの取消・失敗・結果照会を独立させる。開始結果不明は二重dispatchせずunknownとして停止する。Handoff preflightはTask実行や権限移送を行わず、target Task側の現在Workspace／Permission／Owner Approvalを改めて評価する。A2Aの非冪等作用がunknown、native wait/Auditが失敗、接続やTask状態が不明なら成功にせず隔離／停止し再送しない。これらはsourceの責任配置であり、本mappingではinstalled recoveryを実行していない。 |
| 標準OS候補 | Windows Desktop installed候補。Linux nativeは必須対象、Ubuntu 24.04 LTS／x86_64を標準候補とし、WSL/Xvfb/loopback fixtureをnative installed外部Agent証拠へ転用しない。macOSはnative A2A契約がIPv4 loopback Agent Card／metadata-onlyであり、現AGENTSとmacOS仕様によりMessage/Task/Artifact/Stream作用は範囲外。Android API 35 Google APIs／x86_64 EmulatorとiOS iPhone 16 Pro／iOS 18.5 Simulatorは候補だが、現行Mobile A2A consumerは確認できない。五OSのCompare／Handoff installed parityは未成立。 |
| 配布物 | Windows portable ZIP／D4PKG01、Linux Ubuntu 24.04／amd64 portable tar.gz・bootstrap deb／D4PKG01、macOS macos-15 debug/UI workflow、Android debug APK/AAB、iOS Simulator appはN10候補。正式な利用者向けpackage、同一候補のA2A外部接続／Compare／Handoff lifecycleは未成立。候補形式を配布Acceptanceへ昇格しない。 |
| 既存実装 | 画面: `apps/desktop_flutter/lib/screens/agent_center.dart`、`a2a_connection_center.dart`、`host_operation_center.dart`、`host_capability_center.dart`。呼出しservice: `agent_coordination.dart`、`agent_handoff.dart`、`a2a_connection_client.dart`、`a2a_artifact_handoff.dart`、`host_registration_client.dart`。共通Task処理: `packages/agent_orchestration/lib/agent_task_client.dart`／`agent_task_execution_coordinator.dart`。Rust Broker: `native/rust_helper/src/broker/a2a_center.rs`、`a2a.rs`、`broker/host_center.rs`／`host_capability.rs`、`broker/protocol.rs`、`broker/dialogue.rs`、`broker/audit.rs`。macOSの`A2A`接続契約は`docs/specs/macos-a2a-center.md`。試験とSchemaの識別値は`SourceSHA`欄。 |
| BUILD_GAP | Compare/Handoff/A2Aはsource上`WIRED`だが、同一installed candidateで別Workspaceの実体隔離、同時実Task、failure/cancel isolation、selected-result Apply/Auditまでを成立させたv4 evidenceはない。実外部AgentのCard→Message/Stream/Task→明示Artifact→target新Task→fresh Permission/Approval/Auditという一貫した実作用証拠もない。macOSはloopback metadata-onlyの契約範囲、Mobileにはconsumerなし。Host registryはremote Task/Workspace authority境界consumerではない。ここで列挙した不足を別工程の受入条件まで拡張せず、N04/W03/G3の有限要求として保持する。 |
| `SourceSHA` | 照合HEAD=`0f732acca6f236db2c59867730f176114f2bc90e`。Git blob: `apps/desktop_flutter/lib/screens/agent_center.dart`=`2472faeb31a64d624423c30a0a30d0f153d2e2a3`; `apps/desktop_flutter/lib/screens/a2a_connection_center.dart`=`2716d1c2fa22a184044e6f4e2eb401cb33c6e011`; `apps/desktop_flutter/lib/screens/host_operation_center.dart`=`e16605e6bb39bbfc12806e6154e08d6fbab1b93b`; `apps/desktop_flutter/lib/screens/host_capability_center.dart`=`e8fa774a435e713a3b75b0d33f00bd500b0a2d7f`; `apps/desktop_flutter/lib/services/agent_coordination.dart`=`bdc13464f0e42b740a2bca3b7e8828a5019056d0`; `apps/desktop_flutter/lib/services/agent_handoff.dart`=`a85110d96cbade732ae0e1bfea2529f03f37523c`; `apps/desktop_flutter/lib/services/a2a_connection_client.dart`=`19335054afe35bc06597629d92ca2e635ce50069`; `apps/desktop_flutter/lib/services/a2a_artifact_handoff.dart`=`e7e2f3454e2843f1d4f4e7b5aedfa55014cde05c`; `apps/desktop_flutter/lib/services/host_registration_client.dart`=`3bc3b45bddddf4e1131e051acb3cdee599ca4d3d`; `packages/agent_orchestration/lib/agent_task_client.dart`=`a31fbb9f980a0039f809fd763c91f055fe9d02d0`; `packages/agent_orchestration/lib/agent_task_execution_coordinator.dart`=`feb9aacb7ed77e6f4e76e9d9371290decca9cf7b`; `native/rust_helper/src/broker/a2a_center.rs`=`73ad601abd64a33ccefde4b73ab5f9dc4f728504`; `native/rust_helper/src/a2a.rs`=`9d7c6d9ab938ad5178d6ca46b3933dc3e61c6061`; `native/rust_helper/src/broker/host_center.rs`=`c27707faa61e5270b1c6d79e3ee3c5fe6ccd0bf8`; `native/rust_helper/src/broker/host_capability.rs`=`08a82c7e52947439229447dc82b69097af67ba1f`; `native/rust_helper/src/broker/protocol.rs`=`9b2076d5f31ca6d77f1f87973d1b394aaf751961`; `native/rust_helper/src/broker/dialogue.rs`=`c3c3fddb630dd06b7e6cc1c13b39f3d4f6f6a999`; `native/rust_helper/src/broker/audit.rs`=`eb1dcf5018d95c7fdf6171aadd4bca9554d1996f`; `apps/desktop_flutter/test/agent_coordination_test.dart`=`6783e87a492d8436a855a7088b7cece38d122bd5`; `apps/desktop_flutter/test/agent_handoff_test.dart`=`c6bc9bef8cc8d153ef59bf421efbd989185f8e0c`; `apps/desktop_flutter/test/a2a_connection_center_test.dart`=`5ebdbb4cee62e581062370ed6c305c36df89b7e2`; `apps/desktop_flutter/test/a2a_artifact_handoff_test.dart`=`e47005320917f59dfa55ae1e04c4b99d452676de`; `apps/desktop_flutter/test/host_registration_test.dart`=`74ab39e078bf72214a3bbe84f69ebc77fceb2493`; `apps/desktop_flutter/test/widget_test.dart`=`f4e9c7b93f832ba2a2cefee779fd12426f240b04`; `native/rust_helper/tests/agent_task_production_e2e.rs`=`fdc7565aa58fa7466e6e24e5492321e8015567eb`; `specs/agent_handoff.schema.json`=`e37a276f8c3e19816bc4f80824efce546a116d00`; `specs/agent_handoff_package.schema.json`=`a8ae1e2a3be6c0ed5666849fc6553c98083a1cc3`; `specs/agent_task_request.schema.json`=`cb25bb3684656bfd670ea5832bbb7fc5988845b6`; `specs/agent_task.schema.json`=`a552389072088d6cd42b3588fefda332be4ae6b6`; `specs/a2a_contract.schema.json`=`8c7f50bb7fb12dde41293727e5dc9109445d1bc3`; `specs/a2a_connection.schema.json`=`1dbbe44e54b302a1196d7e4e55f1cae35cda569b`; `specs/a2a_connection_receipt.schema.json`=`f63fd0d61b6200cdd8cc4b23d5cf7968a82fa5c5`; `specs/a2a_connection_list.schema.json`=`ee18b204349b025464ca3af8adaa355add34e650`; `specs/a2a_message_send_request.schema.json`=`0a5f87250f17a88b6be55e7838d32b8e9114bbed`; `specs/a2a_message_send_response.schema.json`=`23ee690e0ce350ac94c64d86f6920afb63c199f6`; `specs/a2a_stream_send_request.schema.json`=`e0a0841f2fd9f7e92e0fb8a036ebaa6852e198d8`; `specs/a2a_stream_send_response.schema.json`=`a526fed89fb5b3766a03c52444a338060caf16cc`; `specs/a2a_task_get_request.schema.json`=`df11a5fd3291de0ae379694c6c1b49de51f2f157`; `specs/a2a_task_get_response.schema.json`=`56a55f348d6b0ba7cb0092ee7ea2b4ec809e5360`; `specs/host_registration.schema.json`=`8bed26fb2fe8f8e296c1d3404978f3215d571240`; `specs/host_list.schema.json`=`93880c43b04a5408c033bf351ee695073e54664a`; `specs/host_capability.schema.json`=`6b721b5ab7842edc7df6562bc56fdd5b73d5de79`; `specs/host_switch.schema.json`=`01e5fc7322e8d4d2c9142375e7641fc4255148d5`; `docs/specs/macos-a2a-center.md`=`6f5bc7444f2e406aa5c7d21d1aea1f7599c743ad`。上記44件すべてpath/blobを照合した。 |
| 試験区分（`TestClass`） | source内UNIT／FIXTURE: `agent_coordination_test.dart`はWorkspace/Session非共有、同一本文、並列開始、片側失敗・cancel、同Runtime比較を確認。`agent_handoff_test.dart`はcompleted/full・hash束縛・異なる境界、`a2a_artifact_handoff_test.dart`は明示Artifactとtarget preflight、`a2a_connection_center_test.dart`はmetadata sanitization・Owner拒否・unknown作用の非再送、`host_registration_test.dart`とWidgetはmetadata/UIをFake Brokerで確認。Rust `a2a.rs`／`a2a_center.rs`にはloopback・permission・Auditのunit/fixtureがある。macOS P13のAgent Card接続はsynthetic loopback metadata-onlyの過去`LIVE_RUNTIME`。実外部Agent installed E2Eではない。本mappingで製品testは再実行していない。 |
| 証拠ID（`Evidence ID`） | `NONE`。旧P2/P3/P7 Product Build記録、旧Compare fixture、macOS P13 MAC-A2A限定LIVE_RUNTIME（固定loopback Card・metadata-only）は履歴参照だけで、N04の実外部Agent作用／Compare隔離／Handoff target Task受入れ証拠へ転記しない。 |
| 阻害要因（`Blocker`） | W03/G3はOPEN。Registryは18件（16 unresolved／2 resolved）、`release_ready=false`を維持する。`comprehensive_extension_rev1_completion`等は広域関連し得るがN04への直接帰属はN11照合までUNKNOWN。新規Release blockerは作成しない。 |
| 状態 | OPEN（N04のE0-04 mappingは21項目を充足。N04製品Acceptance、W03/G3、E0全体はOPEN）。 |
| 残余 | installed候補で独立Workspaceを使う実Compare、実外部Agentとの作用、選択Artifactからfresh Permission／Approval付き受信Task・Auditまでの一体証拠が残る。Host registryからremote TaskへのConsumerは存在しない。各OSの標準package／installed経路はN10と結合する。 |

### N05 Capability Catalog→Compose→元Tool実作用

| E0-04項目 | 個別値 |
|---|---|
| 現ID | N05（Capability Catalog、依存／互換／権限検査、Compose、元Tool実作用） |
| 旧ID | v4 S05対応は`P6→N02/N05`、`P8→N05/N07`、`K14`、W04/G4。旧Phase 29/30はCompose/Preview、Phase 31はAI EditのためN07へ分離する。旧PのCLOSEDは当時の範囲のまま保持し、N05へ移さない。 |
| 原文参照 | v4仕様S04 N05／S05 K14、工程表R03線D・W04/G4、施工指示I04-D。現行3点セットの採用記録・hashは冒頭基準表のとおり。要求は現実在Capability→依存／互換／権限検査→Compose→元Tool実作用であり、不在／衝突／危険条件を拒否する。 |
| 対象 | Desktop SettingsのCompose UI、Broker Runtime/AgentとMCP Catalog、Compose Manifest/Preview、MCP Tool call、Windows/Linux/macOS native Owner境界。Android/iOSのMCP状態画面は接続・Tool作用を非対応として示す。 |
| 目的 | Manifest作成だけで完了扱いせず、現在実在するCapabilityとToolを確認し、危険・不在・衝突を拒否したうえで、承認された構成から元MCP Toolを実行し、bounded結果とAuditへ結ぶ。 |
| 生成元（`Producer`） | `settings.dart`がBroker Runtime/Agent Catalogと接続MCP/Tool Catalogを取得し、OperatorがRuntime、Agent、接続、Tool、Capability requirementを選ぶ。`ComposeClient`がManifest/Preview要求をBrokerへ渡す。受理Manifestとdraftが一致した状態での明示Tool操作が既存`McpConnectionClient.callTool`要求を生成する。AI自発要求や新しい権限はCompose操作から生成しない。 |
| 実Consumer | `protocol.rs`が既存Broker operationを`compose_center`／`mcp_center`へ配送する。Composeは現在Runtime/Agent/MCP/Tool Catalogを再照合して`manifest_only`を返す。Tool callは同Manifestを再検査後、native Owner確認、永続Audit、Tool identity/schema、Broker内一回Permissionを通し、登録済み接続の元Toolを呼ぶ。Receiptはhashとbounded metadataで、Flutterは本文を表示しない。 |
| 契約／Data | `gui_shell_compose.schema.json`、`gui_shell_compose_catalog.schema.json`、`gui_shell_compose_receipt.schema.json`、`agent_adapter.schema.json`、`capability.schema.json`、`mcp_connection_list.schema.json`、`mcp_tool_call.schema.json`、`mcp_tool_call_receipt.schema.json`。Composeは`manifest_only`、Authority/Permission/Approval/Credential/Audit chain継承なし。MCP Tool実作用のPermission/Approvalは別のBroker operationで一回・対象束縛する。 |
| 正常時作用 | 現在のRuntime/Agent/MCP/Tool選択からCompose Manifestを生成し、Brokerが受理する。Tool call時に現在Manifestと接続を再検査し、native Owner確認、Tool/引数検査、Audit、`server_id/tool_id/arguments_hash`束縛の一回Permission後に元Toolを一度呼ぶ。成功receiptは`result_hash`、content type/count、状態、Audit ID等に限定する。 |
| 否定時作用 | unknown/stale Runtime、Agent、Capability、unsupported宣言、Runtime instanceをまたぐCapability寄せ集め、unknown/stale接続／Tool、invalid schema、Owner拒否、Audit不成立、失効Credential、隔離済み接続を拒否または送信前停止する。異なるMCP Serverに同じTool IDがある場合、Flutterは実行対象を一意に決められずボタンを無効化する。ただしBroker側Composeは現在Tool IDを接続横断の集合で照合し、重複を明示rejectしていない（具体的なBUILD_GAP）。 |
| 停止／Recovery | 送信前の不一致・Permission・Approval・Audit失敗はfail-closedでToolを呼ばない。送信後に結果不明なら自動再送せず接続を隔離・子process群停止を試み、結果不明／停止失敗を別errorとRecoveryActionで返し、外部副作用をOwnerが確認してから再接続する。Compose/Preview自体はbuild・Export・rollbackを実行しない。 |
| 標準OS候補 | Windows、Linux native、macOS DesktopではBroker/MCP Tool callのsource経路とOS固有Owner確認入口を確認。これは各OSのinstalled候補・実MCP接続のAcceptanceを意味しない。Android/iOSにはCompose/Tool実作用consumerがなく、mobile状態画面はTool操作非対応を明示するためBUILD_GAP。Linuxは必須製品対象。 |
| 配布物 | OS別のinstalled候補・正規PackageはN10/W07で候補同一性と共に扱う。現mappingはWindows/Linux/macOSの同一installed候補でCapability→Compose→実Toolを通した配布／実使用証拠を含まない。Android/iOSのCompose/Tool対応配布経路も未成立。 |
| 既存実装 | `dialogue.rs::compose_catalog_snapshot`は登録済Runtime IDと`agent_adapter_projection` metadataを投影する。`compose_center.rs::validate_catalog_selection`は宣言Capability supportを互換条件として読むが、Compose専用のBroker-owned実Capability Catalog／dependency graph consumerは見つからない。一般`capability.schema.json`やPython conformance stateは、このRust Compose runtime consumerの証拠ではない。MCP実行側の権限判定は`authority.rs`および`mcp_center.rs`にあり、Agent metadataからAuthorityを生成しない。 |
| BUILD_GAP | ①実Capability CatalogをComposeが消費して、依存・実能力・permission requirementを現在のBroker状態と照合するproducer→consumerが未接続／未確認。現行はAgent Adapterの未信頼宣言値を照合する。②異なるMCP Server間のTool ID衝突をBroker自身が拒否しない。③Android/iOSのCompose→Tool consumerがない。④実Capability→Compose→元Tool→結果/Auditの同一installed候補・実サービス証拠がない。 |
| `SourceSHA` | 照合HEAD=`1f072c632009199e84ffb519017bb345fbbf1664`。以下28 Git blobを個別に記録し、本HEADの`HEAD:path`と照合した。 |
| 試験区分（`TestClass`） | Rust UNIT: `compose_center.rs`のCatalog/compatibility/manifest-only/非継承 test。Rust FIXTURE: Windows限定`mcp_center.rs::mcp_catalog_compose_rechecks_current_connection_and_rejects_stale_selection`は子`cmd.exe` MCP fixtureでstale拒否と一回作用を扱う。Flutter client/widget testはFake Broker。いずれもsource上のtest classであり、本mapping作業ではRust/Flutter/product testを実行していない（NOT_RUN）。 |
| 証拠ID（`Evidence ID`） | `NONE`。SourceSHAによる現行source mappingのみ。fixture、schema、旧W00/P8証拠を現N05のinstalled product acceptanceへ転記しない。 |
| 阻害要因（`Blocker`） | W04/G4とN05はOPEN。既存Release Registry 18件（16 unresolved／2 resolved）、`release_ready=false`を維持する。N11で直接対応が確定するまでは既存blockerのN05帰属は`UNKNOWN`。新規Release blockerは追加しない。 |
| 状態 | E0-04 N05 mappingは21 fieldと28 SourceSHAを現HEADへ結合済み。N05製品Acceptance、W04/G4、E0全体はOPEN。 |
| 残余 | 実Capability Catalog／依存解決、Broker側Tool ID collision rejection、Mobile consumer、同一installed候補の実Tool/Audit証拠が残る。AI自発Tool要求やAI EditはN02/N07へ分離し、Compose manifestの存在を実使用と見なさない。 |

#### `N05 SourceSHA`

| `Source file` | `Git blob SHA` |
|---|---|
| `apps/desktop_flutter/lib/screens/settings.dart` | `66374ec4157d573939e7f9cc5c36aecf5d65e251` |
| `apps/desktop_flutter/lib/services/compose_client.dart` | `8f9657d866b007a4d2212abefca4df9add81a702` |
| `apps/desktop_flutter/lib/services/mcp_connection_client.dart` | `12a173a3956368c53615255c56ef06a2f5939c9a` |
| `packages/agent_orchestration/lib/mcp_connection_client.dart` | `71b17973216160bdd2bc447243ba0b04fd0fec63` |
| `apps/mobile_flutter/lib/screens/mcp_status.dart` | `40af06807f8bdf1280d860b3d9ffa32ee2e22cc0` |
| `native/rust_helper/src/broker/compose_center.rs` | `d80658fd9a342140acc35560b2e4c4e4ae97d888` |
| `native/rust_helper/src/broker/dialogue.rs` | `c3c3fddb630dd06b7e6cc1c13b39f3d4f6f6a999` |
| `native/rust_helper/src/broker/mcp_center.rs` | `8f9ab897bdc625e448241ebd9b46f995cd1f22f6` |
| `native/rust_helper/src/broker/protocol.rs` | `9b2076d5f31ca6d77f1f87973d1b394aaf751961` |
| `native/rust_helper/src/broker/authority.rs` | `067a36442138897aac396bdb588c2e01f90a5318` |
| `native/rust_helper/src/desktop_launcher.rs` | `6a9f335de07fa474ca10043aea63ad6e988b572d` |
| `native/rust_helper/src/linux_desktop_launcher.rs` | `f58fb226f21093c2d1f062259d394b4f8ec022f6` |
| `native/rust_helper/src/macos_desktop_owner.rs` | `31e13a57fcde422878fae63083bd323e07f482ec` |
| `native/rust_helper/src/broker/device_link.rs` | `c2e1ff88ff34a5764b2b4fb1d3f7abd8727e5c51` |
| `specs/gui_shell_compose.schema.json` | `89b3d8e0e09c802d555fd6fee233daebc0fec045` |
| `specs/gui_shell_compose_catalog.schema.json` | `f9c76f5fa87bfb0866f33745dcd5a8a4e74864d5` |
| `specs/gui_shell_compose_receipt.schema.json` | `7c4ac9d0f28f38f79a0074ed7d6c84d9124aadaa` |
| `specs/agent_adapter.schema.json` | `e0792589a2bae168ba221af4f4f9abb505815c12` |
| `specs/capability.schema.json` | `a243a84b22cdd704875f35afc56e3450f9c9e1ca` |
| `specs/mcp_connection_list.schema.json` | `45f5613aa46be7f936bd87995da0b6b6b38d047f` |
| `specs/mcp_tool_call.schema.json` | `54c044cdfb53588ea5bab4bd056404372dc39acd` |
| `specs/mcp_tool_call_receipt.schema.json` | `0049a393104b035bbf047e653e962754bc4009b3` |
| `apps/desktop_flutter/test/compose_client_test.dart` | `c3fe30faa668482e5a4e42b2f5f5e5b218a01dc8` |
| `apps/desktop_flutter/test/settings_compose_test.dart` | `a78353f93617cc7b17391549ea256f007a7acdb1` |
| `apps/desktop_flutter/test/mcp_connection_center_test.dart` | `c81a37ca78b15e57c838f48519aa3e4abb6d6665` |
| `docs/specs/agent-runtime.md` | `195e5e2e28e14fa65e2212db9f2adba81cbdb5c0` |
| `docs/specs/mcp-connection-center.md` | `64d46042f19fccb0f7a947a95711e9aae87a1564` |
| `docs/specs/gui-shell-compose.md` | `6b43bb80f73d4268e734ac0afbc58b33ba07339f` |

### N06 Compose→独立App／Installer／Module実除去

| E0-04項目 | 個別値 |
|---|---|
| 現ID | N06（Compose構成から独立identity・Broker・Runtime・Manifest・選択Moduleを持つ独立App／Installerを生成） |
| 旧ID | v4 S05対応は`P9/P10→N06`、`K15`、`W05/G5`のExport／Pruning範囲。旧Phase 32はStandalone Export、Phase 33はModule選択／除外。P9/P10の旧rev5局所CLOSED・fixture／build証拠は当時の条件へ固定し、現N06へ移さない。W05のAI Edit portionはN07。 |
| 原文参照 | v4仕様S04 N06／S05 K15、工程表R03線E・W05/G5、施工指示I04-E。構成を実消費する独立App、別identity/Broker/Runtime/Package、選択外Moduleの実除去と必要機能保持を要求する。実測サイズ／起動／最小資源の詳細は後続E4。署名は別条件。 |
| 対象 | `settings.dart`の書出し画面→Broker `GUI Shell書出し`→固定`Manifest`／`Receipt`→Windows/Linuxの`export_windows_product.py`／`export_linux_product.py`・`package builder`→Rust `Launcher`／Broker。`macOS`向けBrokerの配布対象情報、Android／iOS対象の拒否も照合する。OS別の配布物／導入／更新／撤去の詳細は現行N10のOS表を参照し、N06の独立製品への設定消費と混同しない。 |
| 目的 | 新しいApp identityとAudit store identityを作り、Authority等を継承せず、選択したCompose／Runtime／Adapter／Module構成を独立Appが実際に読み込み、必要機能を動作させ、除外した機能をbinary/resourceから除く。Manifestやportable bundleを独立Product完成と見なさない。 |
| 生成元（`Producer`） | SettingsのExport UIがaccepted Compose Manifest、target platform、optional Module selection、distribution channel、任意Update trust metadataを`ExportClient`経由で既存Brokerへ渡す。BrokerはCompose/current catalogと固定Module Catalogを検査し、App ID・Audit store IDを新規生成、non-inheritance policyとmodule dependency planを含むExport Manifest/Receiptを固定Export directoryへ作る。 |
| 実Consumer | `tooling/export_windows_product.py`／`export_linux_product.py`がBroker Receipt・Manifestのschema/hash/target/module planを検査し、Windows/Linux portable bundleへManifestをbyte-for-byte格納する。Windows/Linux Launcherは製品Manifestのhash、target、App/Audit identity、非継承条件と`binary_pruning_status=not_applied`を検査する。現Exporter evidence自身が`runtime_manifest_consumed=false`、`binary_pruning_verified=false`を明記し、Launcherは選択Runtime／Adapter設定を動作構成へ反映するconsumerではない。 |
| 契約／Data | `gui_shell_export.schema.json`、`gui_shell_export_manifest.schema.json`、`gui_shell_export_receipt.schema.json`、`gui_shell_module_catalog.json/schema`、`gui_shell_module_build_evidence.schema.json`、`gui_shell_module_comparison_evidence.schema.json`、Windows/Linux Export build schema、Windows/Linux product-package manifest schema。Manifestは新App/Audit identity、settings、runtime ID、Adapter ID、Capability requirement、target/channel、Module planを持ち、Authority／Permission／Approval／Credential／Audit chainは`none`。Credential実値をBundleしない。 |
| 正常時作用 | Brokerは新規App ID/Audit store ID、Manifest hash、module plan、fixed-path Manifest file/Receiptを生成する。Module planは固定Catalogのrequired 10、unprunable security core 8、optional 8を分け、unknown/duplicate/required module指定を拒否し、依存閉包を解決する。Windows/Linux developer toolはvalidな入力からportable bundle/packageを組み、Launcherはhash-bound identity／非継承Manifestを検査する。 |
| 否定時作用 | malformed/stale Compose catalog、unknown/duplicate/required Module、未知または循環依存、Manifest/Receipt hash・target・identity不一致、binary pruningを`applied`と偽るManifest、symlink/reparseや不正Package inventoryは拒否する。Broker Exportは任意保存path/overwriteを受けない。Android/iOS targetはBrokerで拒否する。 |
| 停止／Recovery | Brokerの受理Audit／Manifest作成／完了Auditを順に記録し、Audit失敗やManifest write/cleanup不整合は成功receiptへ昇格せずRecovery stateを返す。Exporter/package buildの不一致はbundle/packageを受理せず停止する。LauncherはManifest検証失敗時にProduct processを起動しない。選択Moduleによる実機能変更・旧版Rollback・インストール復旧の成立は本mappingでは確認されず、N10/E4へ接続する。 |
| 標準OS候補 | Export Broker targetはWindows/Linux/macOSを受理するが、現Export build consumerはWindows/Linuxのみ。macOSは現行OS表上、通常Product Export/Installer未接続。Android/iOSはExport targetを拒否しMobile Export consumerなし。Linux nativeは必須製品対象。各OSの保証版／arch／正規導入候補はN10で管理し、portable/VM/fixtureを通常配布合格へ転用しない。 |
| 配布物 | Windows portable ZIPとD4PKG update payload、Linux portable `tar.gz`／D4PKG update payloadおよびbootstrap-only setup `.deb`のsourceがある。現在のExport evidenceは`installer_status=not_started`、`signed=false`、`formal_distribution_claimed=false`（Windowsは`standalone_app_verified=false`）を明示。`.deb` bootstrap／D4PKG updateを製品Installerと呼ばず、詳細はN10 OS/package表に従う。 |
| 既存実装 | `export_center.rs`はnew identity・Manifest・module dependency planとOwner/Audit境界を生成。Windows/Linux ExporterとPackage writerはManifest hashとidentityを保存・検査。Desktop Launcher/Linux Launcherは固定hashのManifestを検証。`resolve_module_plan`は安全Coreをunprunableに保ち、dependency closure・unknown/duplicate/required拒否を実装。Windows `build_module_pruned_windows.py`はDeveloper build用compile-time Dart defineをUI surfaceに適用し、AOT comparisonを生成する。 |
| BUILD_GAP | ①Compose ManifestのRuntime／Agent／Tool／Capability設定を独立Appがruntime configurationとして消費する経路がない。②Module Catalogの`source_surfaces`はoptional Flutter UI surface中心で、必要機能の完全なsource／dependency closureを示さない。③Export planは`binary_pruning_status=not_applied`。Windows define/AOT比較はUI surface限定で`runtime_unverified`、resource comparison未測定、Rust/third-party Moduleは除去されない。Linuxの実pruning、macOS/Mobile exporter/consumerも未接続。④Windows/Linux package builderはあるが現sourceで通常利用者向けInstaller／正式配布や同一候補のinstalled独立Product動作は成立していない。⑤実除去後の必要機能・権限・Broker・Audit・Runtime同一候補検証がない。実測サイズ／起動／資源はE4へ送る。 |
| `SourceSHA` | 照合HEAD=`c1ee25071ea5b5f2e73af6d4b3c2dd5a7043b6d2`。以下44 Git blobを個別に記録し、本HEADの`HEAD:path`と照合した。 |
| 試験区分（`TestClass`） | Rust UNITは`export_center.rs`のidentity/non-inheritance、fixed-path write、dependency closure、unknown/duplicate/required Module拒否。Launcher UNITはManifest hash/identity/non-inheritance拒否。Flutter client/widgetはFake Broker。Linux Exporter/packageにはPython unit/integration source、Windows pruningにはConformance／AOT evidence pathがある。旧P9/P10限定build/fixture証拠は当時のscopeであり、今回のN06 acceptanceではない。本mappingでRust／Flutter／Exporter/package／installed product testは`NOT_RUN`。 |
| 証拠ID（`Evidence ID`） | `NONE`。SourceSHAと既存source/test classの対応付けのみ。旧P9/P10 Product Build、portable bundle、Module UI除外/AOT comparisonの限定evidenceは履歴を保持し、現N06の独立App／Manifest consumption／binary pruning acceptanceへ転記しない。 |
| 阻害要因（`Blocker`） | N06/W05/G5はOPEN。既存Registryの`rev2_export_owner_ui_authority_path`と`rev2_module_pruning_binary_and_measurement`を保持。Registry 18件（16 unresolved／2 resolved）、`release_ready=false`を変えず、新しいBlocker／Acceptanceは追加しない。 |
| 状態 | E0-04 N06 mappingは21 field・44 SourceSHAを現HEADへ結合済み。N06製品Acceptance、W05/G5、E0全体はOPEN。 |
| 残余 | Manifestを使って構成を実行する独立Product consumer、Windows/Linux正式Installerとinstalled受入れ、macOS/Mobile Export経路、binary/resourceからのModule実除去と必要機能保持の実証が残る。E4の実測範囲、N10のOS lifecycle、署名／Publisher条件をそれぞれの責任工程へ送る。 |

#### `N06 SourceSHA`

| `Source file` | `Git blob SHA` |
|---|---|
| `apps/desktop_flutter/lib/screens/settings.dart` | `66374ec4157d573939e7f9cc5c36aecf5d65e251` |
| `apps/desktop_flutter/lib/services/compose_client.dart` | `8f9657d866b007a4d2212abefca4df9add81a702` |
| `apps/desktop_flutter/lib/services/export_client.dart` | `70d77edc6beb9e209e6b362219d1b22d69375489` |
| `apps/desktop_flutter/test/settings_compose_test.dart` | `a78353f93617cc7b17391549ea256f007a7acdb1` |
| `apps/desktop_flutter/test/export_client_test.dart` | `b4ec8686b91370b1eaff06dd88b301ee6ef0370c` |
| `apps/desktop_flutter/test/module_pruning_test.dart` | `20470e523c2640b681e54c51f9c6749ee35f5e05` |
| `native/rust_helper/src/broker/export_center.rs` | `477e44d86ebf18d2b004a7294c9cdb14e35aa0f0` |
| `native/rust_helper/src/broker/protocol.rs` | `9b2076d5f31ca6d77f1f87973d1b394aaf751961` |
| `native/rust_helper/src/broker/product_install.rs` | `c91115bdf88dd28e5f3445e7ed66127902909677` |
| `native/rust_helper/src/desktop_launcher.rs` | `6a9f335de07fa474ca10043aea63ad6e988b572d` |
| `native/rust_helper/src/linux_desktop_launcher.rs` | `f58fb226f21093c2d1f062259d394b4f8ec022f6` |
| `native/rust_helper/src/linux_product_bundle.rs` | `f71ed312c629e81914c61180baeb9723b315a69a` |
| `native/rust_helper/src/product_package.rs` | `a2deb2b6e77bc00ae8d01cd5ca306ee63baa20e5` |
| `native/rust_helper/src/bin/gui_shell_desktop_launcher.rs` | `809efba7a03eacddf9e172011149d9a8d5e3fda7` |
| `tooling/export_windows_product.py` | `31856179881a5a5aae1ba4451732e6de6cd7f338` |
| `tooling/export_linux_product.py` | `47fe9a3032a5b2680e3f7d39b7cd64120ac83843` |
| `tooling/package_windows_product.py` | `c6f9155f2457ef0d89ecb711fa94424fbb1501ee` |
| `tooling/package_windows_portable.py` | `d6cfba6941da10fbae35228e7137d7f87f8d3889` |
| `tooling/package_linux_product.py` | `0d32d04ec5b43e1815b3664a323c17db79bc928a` |
| `tooling/package_linux_deb_setup.py` | `e7a057b2f3db2fca8bbe91401e25d71fb88ceca3` |
| `tooling/build_module_pruned_windows.py` | `490aae7209ea5988f8e2cdcb05fe092b3d29504a` |
| `tooling/compare_module_builds_windows.py` | `09d926b9c6b611cd59e524cc947ce2dce1c81247` |
| `tooling/tests/test_export_linux_product.py` | `079a187de49cf8b2fb6a7ddadb4175d6916163d1` |
| `tooling/tests/test_linux_product_package.py` | `3bb3b520d3c4d15b1f4d43993c8cddb0f0c08ee1` |
| `tooling/tests/test_linux_deb_setup_package.py` | `2467e29b9eb13bd65065fea5a80872a3068cdfcd` |
| `tooling/tests/test_linux_installed_product_smoke.py` | `dc94f9b782d3699810bbe50230a5e9510a5739f7` |
| `tooling/tests/test_linux_portable_archive.py` | `c04ac3c5d6d58396f650e9530b6731c6bb2037d6` |
| `tooling/conformance_tests/run_conformance_skeleton.py` | `c6f31d8cb561b48513c6784e54c85969c9cb953e` |
| `specs/gui_shell_export.schema.json` | `8b96d344ac0f6a54011237be8b2d85964be48c4b` |
| `specs/gui_shell_export_manifest.schema.json` | `bf136b1a1daa258d33af3635ee414a2d4312bec5` |
| `specs/gui_shell_export_receipt.schema.json` | `0448ec5f631509fa6fbe25cb4957bd6cf51e1562` |
| `specs/gui_shell_module_catalog.json` | `339cb8894b25ae963956e582c42b9e8fcc0d3931` |
| `specs/gui_shell_module_catalog.schema.json` | `bd54dd7903a1734ea4027d3f780b7a2b49e10c8e` |
| `specs/gui_shell_module_build_evidence.schema.json` | `1a37d4b831f998ce334fc9f0126c5dd2ee09a01b` |
| `specs/gui_shell_module_comparison_evidence.schema.json` | `801ba33bfc0656a4048aa5ca1920b96b7787c304` |
| `specs/gui_shell_windows_export_build.schema.json` | `5c7234692a8f178882edfc42c1a762038e7414fa` |
| `specs/gui_shell_linux_export_build.schema.json` | `6f1af3d3da2277807dabac5d753897f5ddee453c` |
| `specs/d4_pocket_product_package_manifest.schema.json` | `3151ba81a869b87e6c0fa8f8633a0e424fc43ae7` |
| `specs/d4_pocket_linux_product_package_manifest.schema.json` | `34087f190e7189641b34ae3163b568a038c9f1c2` |
| `docs/specs/gui-shell-export.md` | `3d7a1156e4bf095e4f6a64792b6e1683c14f9b17` |
| `docs/specs/gui-shell-module-pruning.md` | `5568c677be30402d709cc365b79eceb0e96ce831` |
| `docs/specs/windows-product-package.md` | `19c5c45dc8cbdbd4560b7b2cb8fe5d8e9f82a2c3` |
| `docs/specs/linux-product-package.md` | `faad8b2a9487afc9a818ad3938e0ff92c77cc268` |
| `release_blockers.registry.json` | `244adc04a01e7b406c4ebcdeb829f7077d4aa35f` |

## 要求母集団・照合状態

| 母集団 | 件数 | 正本・原典 | v4 E0状態 |
|---|---:|---|---|
| 新規要求 N01〜N11 | 11 | v4仕様S04 | N01〜N10はE0-04全21項目を現行SourceSHAへ個別結合済み。製品AcceptanceはすべてOPEN、N11の個別全数照合は未完 |
| 全体施工線 A〜J | 10 | v4工程表R03・Codex指示I04 | 全線同時BUILDのproducer/consumerと現行実装配置を全数照合中 |
| 製品要求 P0〜P13 | 14 | `docs/REV5_PRODUCT_PROGRESS.md`、v4仕様S05/S10 | P0〜P13は全E0-04 fieldを個別記録。P13旧aggregate evidence identity/classは`UNKNOWN`。N11 Evidence/Blocker/OS Artifact全数結合は未完 |
| 総合機能 C0〜C34 | 35 | `docs/総合機能拡張_rev1/` | C0〜C34全35 IDのE0-04 21 fieldを個別記録。旧status維持、N11 Evidence/Blocker/OS Artifact結合と製品Acceptanceは継続 |
| D4 Phase 0〜45 | 46 | `docs/D4_POCKET_PHASE_MAPPING.md` | 46 IDのscope crosswalk済み。Phase 0〜45はE0-04全21 fieldを個別記録。Evidence/Blocker/OS Artifact全数結合・Acceptance未完 |
| 製品能力 K01〜K20 | 20 | v4仕様S05 | 20 IDをv4 N01〜N11/A〜JとCanonical N consumerへcrosswalk済み。K01/K02はE0-04全21 fieldとSourceSHAを個別記録済み。K03〜K20の個別fieldとN11 Evidence/Blocker/OS Artifact結合は継続 |
| Buffer BUF-R01〜R09／BUF-T01〜T24 | 9／24 | 緩衝基盤採用記録・v4仕様S06 | 33 IDのscope/Consumer/現証拠境界と各21 fieldを記録。BUF-R/T Acceptance、cause/effectの全件結合、N11 evidence、OS artifactとの結合は未完 |
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

1. N01〜N10のE0-04 21項目は現行SourceSHAへ個別照合済み。各製品AcceptanceはOPENのまま維持し、N10の五OS実導入・利用・更新・Recovery QAは後続の製品Acceptanceに残す。
2. C0〜C34およびPhase 0〜45の個別field記録を維持し、K01/K02のSource mappingを再試験せず、K03〜K20の個別fieldとN11 Evidence／Blocker／OS Artifact行結合を続ける。旧CLOSED／FAILは履歴として保持し、再試験しない。
3. BUF-R/TのSourceSHAとConsumer scopeは記録済み。AcceptanceはOPEN。B0〜B6の状態を保持し、原因→effect、N08/N10/N11、OS/Artifact/evidence結合を続ける。
4. D4-QE-R01〜R13、D4-LNX-T01〜T08、D4-AUD-F01〜F07、Q0〜Q7、FQ群、W00〜W08、18 blockerを現行Consumer/証拠/gateへ一意に接続する。
5. specs/Schema、Shell Core、Rust Broker、Flutter、Native、Installer、Toolingのproducer/consumer inventoryを現在HEADへ再照合し、SourceSHA・TestClass・Evidence ID・BUILD_GAPを埋める。
6. N10の各OS標準候補・配布・実装・workflow/test・限定履歴を記録する。保証version/arch/resource/package条件は公式条件と実測から固定し、未知をUNKNOWNにする。
7. N11 Evidence/Registry/validator/QA queue consumer mappingに全ID・全OS artifactを接続する。Acceptance/E3 BUILD_COVERED/E4 QC/E5 QA/E6 Release判定は正本工程の順序と区分を維持する。

上記未完了のため、E0は`OPEN`。E1全体BUILDの網羅、BUILD_COVERED、QC/QA、Releaseの成立は主張しない。
