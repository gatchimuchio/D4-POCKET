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
| N01 | 一般利用入口からのAI実Task、結果、停止・Recovery | Codex CLI Adapter→Rust Broker Task worker→Workspace／Agent Center／Audit。W01/G1はWindows normal Release/Codex CLIに限定してCLOSED。 | `OPEN`。W02、期限・crash、五OS標準導入経路とv4結果Consumerを全数照合する |
| N02 | AIが要求する実MCP Toolの結果を同Agentの後続Taskが使う | MCP Center／Broker Tool経路→Agent Task継続の実装・試行記録がある。W02/G2は未合格。 | `OPEN`。同一Task由来・結果依存・新規認可・Audit・UI応答を候補別に結ぶ |
| N03 | 独立した異種Runtimeを共通Task/Result契約で使用 | Codex Runtime経路は存在。2種類以上の独立実Runtimeが同一契約を通る範囲は未確定。 | `OPEN`。Runtime、版、実行Consumer、失敗・unsupported範囲を確認する |
| N04 | Compare／Handoff／A2Aの実作用と権限非継承 | Compare/Handoff/A2Aのコード・試験・履歴がある。要求全体に対する接続・LIVE_PRODUCT範囲は未確定。 | `OPEN`。分離Workspace/Session、成果選択、Artifact限定転送、再認可を照合する |
| N05 | Capability CatalogからComposeを経て元Toolを実行 | Compose Contract/Manifest/UI経路がある。旧W00は実Catalog照合・元Tool Consumerを`BUILD_GAP`と記録。 | `OPEN`。現HEADでCatalog→検査→実作用まで再追跡する |
| N06 | 独立App Exportと選択Moduleの実除去 | Export Manifest、bundle、Module選択処理の履歴がある。独立製品の構成消費・全対象binary/resourceの除去状態は未確定。 | `OPEN`。identity/Broker/Runtime/Installerと実生成物の結合を照合する |
| N07 | AI Edit proposalを別主体で審査しcontrolled apply/rollback | AI Edit proposal-only経路が存在。Proposal以後の独立検査・認可Apply・Rollbackの統合状態は未確定。 | `OPEN`。自己承認禁止とBroker-controlled applyを追跡する |
| N08 | Bufferが元Taskの意味を保ってOS/依存差を調停 | B0〜B4の限定範囲は履歴CLOSED、B5はOPEN、B6は未成立。 | `OPEN`。B5自己適用、B6全Consumer、副作用・fail-closed経路を対応する |
| N09 | UIから実Task、結果、観測、停止・Recoveryを操作 | Agent Center等の一部Broker投影は存在。結果/Diff/Eval/Resource/Recoveryの全Consumer接続は未確定。 | `OPEN`。通常製品入口から実Broker stateと結果の往復を追跡する |
| N10 | Windows/macOS/Linux/Android/iOSで通常導入・利用・更新・撤去 | Windows/Linuxのscope限定installed証拠と他OSの個別実装・試験履歴がある。v4標準OS全件のRelease候補証拠は未成立。 | `OPEN`。OS/arch/version/package/実行環境を固定し、未実装package経路を特定する |
| N11 | 全要求・Source・Consumer・証拠・Blocker・Release判断の結合 | Registry、Conformance、進捗、既存証拠は存在。v4全要求consumer ledgerは本書を含め未完成。 | `OPEN`。全IDをSource/Consumer/否定/Recovery/OS/Evidenceへ結び、18 blockerと同期する |

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
