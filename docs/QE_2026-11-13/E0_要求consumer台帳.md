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
| N05 | Capability CatalogからComposeを経て元Toolを実行 | Compose Contract/Manifest/UI経路がある。旧W00は実Catalog照合・元Tool Consumerを`BUILD_GAP`と記録。 | `OPEN`。現HEADでCatalog→検査→実作用まで再追跡する |
| N06 | 独立App Exportと選択Moduleの実除去 | Export Manifest、bundle、Module選択処理の履歴がある。独立製品の構成消費・全対象binary/resourceの除去状態は未確定。 | `OPEN`。identity/Broker/Runtime/Installerと実生成物の結合を照合する |
| N07 | AI Edit proposalを別主体で審査しcontrolled apply/rollback | AI Edit proposal-only経路が存在。Proposal以後の独立検査・認可Apply・Rollbackの統合状態は未確定。 | `OPEN`。自己承認禁止とBroker-controlled applyを追跡する |
| N08 | Bufferが元Taskの意味を保ってOS/依存差を調停 | B0〜B4の限定範囲は履歴CLOSED、B5はOPEN、B6は未成立。 | `OPEN`。B5自己適用、B6全Consumer、副作用・fail-closed経路を対応する |
| N09 | UIから実Task、結果、観測、停止・Recoveryを操作 | Agent Center等の一部Broker投影は存在。結果/Diff/Eval/Resource/Recoveryの全Consumer接続は未確定。 | `OPEN`。通常製品入口から実Broker stateと結果の往復を追跡する |
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

## 実装ファイルの初期目録（CODE_PRESENTのみ）

下記は`d503a4ef6daac8942abd1cb011442ce28c78ec55`時点のパス存在・責務候補を読み取った初期目録である。名前・コードの存在だけではProducer→Consumer接続、installed/runtime実作用、Acceptanceを証明しない。各行の`WIRED`、実作用、正常/否定/Recovery、Evidence、OS別状態は未照合なら`UNKNOWN`。

| 線／要求 | Producer・UI候補 | Broker／native候補 | 実Consumer・残余 |
|---|---|---|---|
| A／N01・N02 | `apps/desktop_flutter/lib/screens/agent_center.dart`、`services/agent_task_execution_coordinator.dart`、`agent_task_client.dart`、`agent_mcp_task_controller.dart` | `native/rust_helper/src/broker/ipc_server.rs`、`broker/agent_task_scratch.rs`、`adapters/codex_cli.rs`、`mcp_task_tool_stdio.rs`、`adapters/mcp_stdio.rs`、`broker/mcp_center.rs` | Codex CLIとBrokerの履歴はある。現行installed候補の完全なTask→Tool→同Agent follow-up→結果/Audit/Recovery call graphとv4 acceptanceは`UNKNOWN` |
| B／N03・N04 | `runtime_center.dart`、`agent_coordination.dart`、`agent_handoff.dart`、`a2a_connection_client.dart`、`a2a_artifact_handoff.dart` | `broker/runtime_registry.rs`、`adapters/codex_cli.rs`、`adapters/minidora.rs`、`broker/a2a_center.rs` | Adapter/Compare/A2A関連コードは存在する。二つ目の独立実Runtimeの実Task、外部A2A作用、host越境拒否、fresh authorityの現consumerは`UNKNOWN` |
| D／N05 | `compose_client.dart` とCompose UI | `broker/compose_center.rs` | Manifest/Compose層は存在する。Runtime/Agent/Tool実Catalogから元Toolの製品作用までの接続は`UNKNOWN` |
| E／N06 | `export_client.dart` とExport UI | `broker/export_center.rs`、`product_package.rs`、`linux_product_bundle.rs` | Export/Package codeは存在する。独立製品がManifestを消費すること、未選択Moduleのbinary/resource除去・測定は`UNKNOWN` |
| D／N07 | `ai_edit_client.dart` とAI Edit UI | `broker/ai_edit_center.rs` | Rust fileは`proposal_only`・自動applyなしの申告を記録する。別主体の検査からcontrolled apply/rollbackへ至る実製品経路は`UNKNOWN` |
| F／N08 | Repository内の`native/`、`packages/`、`tooling/`で`buffer`／`mediat`／`reconcil`／`arbitrat`を含むfile名は見つからなかった | 上記検索は名前照合に限定 | Buffer作用が別名の実装へあるかは未確認。不存在・Gapとは判定しない。B5/B6とのconsumer対応は`UNKNOWN` |
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
