# D4 Pocket Codex施工指示書 v3.0

文書ID：D4-QE-CODEX-20261109／作成日：2026-11-09／対象：gatchimuchio/D4-POCKET・main／基準観測：319833cec73d1536310d00071153823e2fd0dbed。
上位：D4-QE-SPEC-20261109→D4-QE-ROADMAP-20261109→本書。AGENTS.md／規定／認可・秘密・Audit・Recovery・Git／既採用の旧全要求は維持。
状態：Codexへ引き渡すための全面改訂指示。作成本体はまだRepositoryへ未反映。施工の実着手は指示者の最新版として受領後に行う。本書自体をRelease・公開・別Repository変更の承認へ昇格しない。

## 0. 最上位目的と実行主体の識別［変更不可］

- 製品使用者＝人間のユーザー。AIへ目的・範囲・制約を指示する。
- 仕事の実行主体＝AI／LLM／Agent。必要な実Capabilityを自ら要求・使用し、実結果に基づいて次Task・完了・停止を選ぶ。
- D4 Pocket＝AIの身体・能力接続／実行基盤。製品のUI・Agent管理画面自体を主体・目的に置換しない。
- Rust Security Broker＝権限を評価・保持する唯一の実行境界。AIの判断・Tool結果・Flutter状態・開発フルアクセスは権限ではない。
- 開発施工者＝本指示を実行するCodex。製品内で被試験のCodex CLI Runtimeと同名でも役割・Session・権限は異なる。
- 五役＝設計者／指示者／施工者／使用者／責任者。ある人が兼務しても、設計採否・施工操作・製品利用者認可・正式出荷判断の対象を混線させない。
- 施工目的＝N01〜N11と旧P／C／Phase／BUF／全OSの全能力を同一設計に基づいて**一巡で全実装**し、AI→実能力→実結果→AI後続判断を一般製品で成立させる。Wの個別CLOSEDは実装着手の条件ではない。

施工品質：安全＞堅牢＞明瞭・Audit＞Contract／Runtime完全性＞機能＞利便性。既存能力継承、root cause是正、全旧要求追跡、Windows-first・Linux native独立必須・macOS・Android・iOS、8GB級環境も含む実測に応じた資源効率。性能目標の未測定値をPASSと書かない。日本語意味正本、外部wire語彙は局所原形。

## 1. 開発確認／製品認可／責任者決裁の完全分離

### 1.1 開発施工者の質問要否：A/B/C

| 分類 | 判断対象と発生条件 | Codexの行為 | 確認 |
|---|---|---|---|
| A［委任済み］ | 明示施工契約内の設計細化、全機能のコード／Contract／Schema／test変更、通常GUI／CLI／API操作、検証環境調整、ログ／障害調査、安全な回復、Git保全・通常push、既許可の手動Actions | 既存仕様・現状・危険境界を内部監査して実行。作業・Batch・Tool操作・pushごとに指示者へ再承認要求をしない | **不要** |
| B［外部規則により必須］ | 実際に使用するComputer Use／OS／App／Toolが**特定のその場操作**に本人の事前意思表示を強制することを現在の規則・UIで確認した場合 | 該当1操作・対象・scope・期限だけ所定の本人確認。拒否・失効を尊重。別手段で回避しない。独立作業は続行 | 該当操作の必要最小限だけ |
| C［新たな判断権］ | 未委任の目的・範囲・安全条件変更、不可逆ユーザー資産・履歴の損失、第三者本番データ送信、契約外課金・公表、本人署名鍵／正式Publisher／製品Release GO | 技術的準備・根拠・影響・代替案をまとめて停止対象のみ明示。新判断後だけ該当作用実行 | 新しい設計者・指示者・責任者の意思だけ |

判定式＝現在の一操作の主体・対象・実権限・副作用・可逆性を観測→Aの範囲なら自走→外部強制が実在すればBの一操作だけ質問→契約外の判断が不可欠ならC→それ以外の技術的失敗は施工者自身が修正。

禁止：
1. GUIに「許可」「接続」「続行」と書いてあるという理由で、無条件に指示者へB確認を出すこと。
2. 「安全のため」「念のため」「モデルが不確実だから」を根拠に、既委任施工の一手ごとの再承認を求めること。
3. 外部規則の存在を推測して、製品使用者の本番Permissionや開発権限を普遍的なB確認へ変換すること。
4. B・Cの一操作が止まったため、独立して安全に進められる全実装・検査・ドキュメント・Git保全を一括停止すること。
5. 拒否されたBの別UI／API／OS操作への切替えによる権限制約回避、他Session・他Workspaceへの旧回答流用、同一未変更scopeでの反復照会。
6. 本施工委任を、製品AIがBroker Permission／Approvalを自己付与する根拠として転用すること。

### 1.2 製品使用者の認可は開発A/B/Cではない

製品系統：ユーザー目的・許可設定→AI実行要求→Brokerの現在Capability／Workspace／Host／Task／Approval／Credential検査→許可された実作用→Audit／Recovery。

- 製品内のPermission・Approvalを発行／拒否するのはD4の通常製品認可経路。ユーザーが事前に許可した範囲は内部Stepごとの再確認を要求しないが、新scope・期限・明示確認条件は満たす。
- Codexの開発用フルアクセス＝製品Runtimeの別ユーザーPermissionではない。開発用合成許可fixtureはFIXTUREとして記録し、通常製品の正規本人認可のLIVE_PRODUCT証拠を捏造しない。
- 製品RuntimeのTool権限待ち＝PRODUCT_AUTH_PENDING。外部Tool上の本人確認＝AWAITING_EXTERNAL_INTERACTIVE_GATE。開発指示者の新判断＝RESPONSIBLE_DECISION_REQUIRED。技術失敗＝TECHNICAL_FAILURE。**それぞれ独立の状態機械**。
- Test用端末／製品ユーザー本人確認が現環境で成立しなければ、該当製品試験だけ未観測にし、能力の実装・fixture負例・他OSコード・Packageを継続する。ただし本人不在を理由に製品Brokerを迂回して実作用させない。
- 2026-11-06過去報告の「Workspace Permission発行は1回5分以内の確認が必要」は**その時のComputer Use操作経路に関する歴史的主張**。現在の具体的Toolに適用される強制規則が実在するか独立確認し、一般の全GUI操作や全製品Permissionへ拡大しない。強制された操作は正常系でその場本人確認を実施し、無理なら未実施を記録する。

### 1.3 分類の具体化

| 操作 | 処理 |
|---|---|
| リポ調査、アプリ起動／画面遷移／通常ボタン操作、テスト入力、ログ採取、Rust／Flutter／Python編集・検査 | A。指示者への質問なし |
| 仕様内の既存実装に従う修正、Contract変更・安全負例、Git tag保全・commit・direct-main push・remote照合、許可済みworkflow_dispatch | A。ただしAGENTSの品質・競合・保存規則を満たす |
| 現在使用するToolの本人確認義務が実際に発生するWorkspace権限発行 | B。その発行操作だけ。全BUILD・他操作は継続 |
| 新しい製品使用者のMCP Tool許可、Task Approval、Credential追加・失効 | **製品系統**。製品Brokerとnative確認契約に従い試験。開発指示者への新施工確認ではない |
| UIハング、MCP応答不明、Rust test FAIL、Flutter Timer残留、Package不一致、Test環境制約 | 技術失敗。原因を追跡して施工・再検査。承認待ちへ変えない |
| 公開配布、production署名・Audit鍵、本人だけの契約締結、未委任の不可逆削除 | C。技術準備まで進め、実行前の新判断を取得 |

## 2. 施工開始時・復帰時に読み込む全数情報

内部再同期は指示者への再承認ではない。

1. 最新の原文指示、本v3仕様→v3工程→本指示、AGENTS.md全章、規定/00_日本語基底規定.md、規定/正本索引.json、現行の採用記録／正本優先順位。
2. git remote／main／HEAD／status／dirty差分／他writer／worktree／tags、過去2世代復旧点。現在のSource・Schema・Contract・Test・Frontend・Broker・OS・Release Registryを実物で取得。user変更・別writerを破壊しない。
3. rev5原本のSHA照合を含む docs/QE_2026-10-22/F07_rev5原本要求照合_20261030.md、旧C0〜C34↔Phase0〜45照合 docs/D4_POCKET_PHASE_MAPPING.md、docs/総合機能拡張_rev1/の3点と進捗、docs/REV3_PROGRESS.md、docs/REV4_ACCEPTANCE_LEDGER.md、docs/REV5_PRODUCT_PROGRESS.md。
4. docs/緩衝基盤_rev5補遺_QC/の採用3点、B0全件台帳・B1／B2／B3／B4／B5記録、BUF-R01〜R09・T01〜T24、B0〜B6、docs/QE_2026-10-22/採用QE3点とD4-QE-R01〜R13／D4-LNX-T01〜T08／D4-AUD-F01〜F07。
5. docs/QE_2026-11-05/採用3点・W00表・W01証拠・W02履歴、2026-11-06改訂v1.1、docs/QE_2026-11-08/採用3点・進捗・W02-B01実測、docs/FINAL_QA_QUEUE.md、release_blockers.registry.json、README.md／QUICKSTART.md／ROADMAP.md／RELEASE_CHECKLIST.md／SECURITY.md。
6. docs/specs/とspecs/の当該契約全数、packages/shell_core/、native/rust_helper/、apps/desktop_flutter/、apps/mobile_flutter/、installer/、tooling/、手動workflowの実producer／consumer／負例／実証コード。実装済み・接続済み・欠落・失敗・資格待ちを区別。
7. 各切替の固定文：主語＝AI／目的＝一般仕事の実作用による達成／対象＝D4全製品／現在のproducer→consumer／保持するPermission／実証できる範囲／未成立と次の横断消費先。原文の意味を要約へ置換しない。

過去rev5原本の生ファイルが現在環境にない場合、F07の全文SHA照合・差分0記録の射程を用い、原本を新規作成したと偽らない。追加の現行要求差分は同一性の確認を優先し、欠落はUNKNOWNで保持する。

## 3. 一巡全体BUILDの具体命令［Wゲート直列化禁止］

開始前の内部作業：既存W00の要求対応表を差分更新→全N01〜N11、旧P0〜P13、C0〜C34／Phase0〜45、BUF/B、FQ/Q、Windows／Linux／macOS／Android／iOSの全consumerと既存実装を10本の施工線A〜Jへ割当→共有Contract／安全と必須の真正依存だけ固定。

**ただちにA〜Jの全線へ施工を投入する。W02をCLOSEDにしてからW03へ進む手順、W03の後にCompose、Compose後にExport、最後に非WindowsとB5、という待機列は採用しない。**

| 線 | 既存を再利用し追加する実生産コード・consumer | 完了までの実装操作 |
|---|---|---|
| A AI主導実仕事 | Task／Session／Workspace／Broker／MCP／結果から次Taskの共通Controller | AI自身のTool要求をGUI外のruntime-neutralな経路でも動く契約へ結合。実Tool結果で同AIの後続行為が変化。旧Approvalの再利用なし |
| B 異種Runtime・連携 | Codex CLI＋第2の独立Runtime、Provider・A2A・Host・Adapter、Compare／Handoff | 共通Task／Result・Artifact／diff・testの消費。実外部Task／Message／Artifact、Agent・Host間の権限分離 |
| C 観測と操作 | Desktop Flutter Agent Center／Workspace Inspector／History／Compare／Evaluations／Metrics／Trace／Problems／Recovery／tray／palette／search／notification | 実Broker stateをUIが消費。Timer／isolate寿命、Agent Task応答停止を除去。UIは認可・観測面でありAI自律実行の常駐制御元ではない |
| D Compose・AI拡張 | Catalog／Compose／AI Edit／Preview・差分審査 | 実在能力の依存・互換を検査し元Tool作用へ接続。AI編集案の独立施工・承認付きcontrolled apply |
| E 独立App | Export・Module Selection／Pruning・独立Runtime／Installer | 構成と能力が別artifactで一致。選択外機能除外、安全Coreとidentity・Audit維持、実サイズ・資源は検査で測定 |
| F 緩衝・安全 | BUFコア／Broker認可／process／scratch／cache／OS差の全Consumer | B0〜B4の既存限定成果を継承し、B5自己適用とB6結合条件を**今から**他線の変更へ適用。Authority・Audit・元目的意味の非改変 |
| G Windows | Launcher／IPC／Flutter／Broker／native利用者確認／Installer／Setup Doctor／Update／Rollback／Repair／Uninstall | 現行Windows実物とコードを全体系の実行検証面とし、一般ユーザーinstalled productまで到達する機構を同時整備 |
| H Linux／macOS | Linux native IPC／secret／process／X11・Wayland／Package、macOS Swift・Keychain・native確認／sandbox／配布 | Windows実装完了待ちにせずSource・Contract・実consumerを独立施工。非実機検査と実機受入の証拠は別 |
| I Android／iOS | Flutter Mobile／native保護・Device Link／Broker境界／LifeCycle／配布 | 物理端末・署名資格がなくても可能な完全ソースと試験接続・OS Packageを先に実装。使用者の認可はNative保護経路 |
| J 証拠・Gate | requirements ledger／Schema／Conformance／Strict Release Gate／OS証拠・FQ/Q | W00〜W08を検収条件にし、BUILD起動順として使わない。18 Release Blocker、原典・旧CLOSED／FAILと新候補証拠を全数照合 |

実行順序は「共有契約の実際の型・安全前提→全producer・consumer→横断接続・修正→全体系構成完成」であり「局所GATEを閉鎖→次の能力着手」ではない。並行施工は単一writerの下で安全にできる範囲で行い、排他・競合の禁止を犠牲にしない。各線の実装は部分コードの量でなく実作用・実consumerをもって前進と判定。

### 3.1 現W02問題は全体系の一部として修正

固定観測：
- W01/G1はWindows normal Release／Codex CLI限定CLOSED。現HEAD全機能や別OSのPASSではない。
- W02-B02の apps/desktop_flutter/lib/services/agent_mcp_task_controller.dart と agent_task_execution_coordinator.dart はFlutter Appレベルの配線として存在。線Aのruntime-neutralなAI連続実行契約・製品実物での自律成立を自動証明しない。
- MCP/ToolのRust Broker、現在Catalog、one-shot Permission、native使用者確認、bounded結果context、後続Task新規認可を活用。Agent Centerの画面操作をTool作用の唯一のControllerにしない。
- W02-B03は否定fixtureの限定PASS。実Codex→実MCP Tool→AI結果依存後続TaskはNOT_RUN。B04をCLOSEDへ上げない。
- Installed候補でMCP登録accepted後UI Responding=Falseを実観測。WERが別runのprocessであれば直接因果とは断定しない。現Source・artifact・Dart isolate／main thread／IPC／Broker queue／MCP processの時刻を対応させ、再現・否定・根因・修正・回帰まで責任を持つ。

施工：既存のA/B/C/F/Gの共有境界を同時修正→TaskからAI Tool要求・Broker許可・結果context・結果依存後続Taskまで頭尾を接続→GUIハングの根因へ帰還→安全負例→同一Windows通常製品のLIVE_PRODUCT実証。既存W02単位のCLOSEDが得られなくてもB/D/E/H/Iを施工し続ける。

### 3.2 既存成果は移植し直さない

P2/P3 Compare・Handoff、P4〜P13の旧局所Acceptance、B0〜B4、D4-AUD-F07を現在の実物条件とともに継承。最新契約で新消費先が必要な部分だけ差分実装。既存の正常機能を無理由に破棄した大規模再設計、独立LLMや非公開HDSの内蔵、BLUE-TANUKI／MINIDORA側の無委任改修は行わない。

## 4. BUILD_BATCH境界・安全の即時検査

1. 全系設計：N／旧IDの成立条件→Producer→Consumer→実OS→正常・負例・回復→Evidence→Gateを一表へ固定。旧W00全件棚卸しを無駄に繰り返さない。
2. 対応実装：必要ならRust・Dart・Swift・Kotlin・TS／Adapter・Schema等、責任に適する言語を併用。単一言語固執・Flutterに認可権限集約・Python参照Shell Coreを製品本番依存化・TypeScriptを権限Core化しない。
3. バッチ単位：共通Contractと全consumerを同時変更する。小単位の切れ目では目的・現HEAD・実作用・次consumerを内部再同期するが、全suite・Git・正式QAを閉じない。
4. 安全対象：Capability、Permission、Approval、Audit、Credential、Recovery、Content Exposure、Task／Session／Host／Workspace実体、秘密、署名、非冪等重複・Tool／Agent間result、process kill／scratch／cache・OS保護に影響したときは**その場**で拒否・期限・replay・不正入力・障害時停止を試験。FAILなら危険な対象経路をdisable／quarantine／SUSPEND。他線は可能な限り継続。
5. 正常対象：Source→Rust Broker→実Adapter／Tool→Result→AI消費→Audit→使用者UI／CLI→Recoveryまで、単体生成物だけでなくproduction consumerを確認。
6. 変更検証：現Repository上の実用コマンドと引数を読み、差分のcompile・Schema・Conformance・Flutter・Rust・負例を限定実施。意味あるBUILD_BATCH末に全影響域を統合検証。Test順が原因で製品コードを歪めない。
7. B5自己適用：OneDrive／Cargo／Flutter artifact隔離、TLS／loopback、SAC、Toolchain／ABI、macOS／Mobile差、process・cache・Cleanupを、介入前後の同一条件で比較し副作用を記録。簡単な再試験PASSを未知根因の解消にしない。
8. BUILD_COVERED：全必須N/P/C/Phase/BUF／各OSの実consumer配線まで到達したか監査。全体にBUILD_GAPがある限り達成しない。物理環境・本人資格不足はBLOCKED_AUTHORITYまたはNOT_RUNを明記。
9. その後に同一候補のQE_QC・正式Q0〜Q7・対象別QE_QA・Release Review。BUILD中の局所安全品質を最終QC／QAと混同しない。

### 4.1 代表検査入口［実コマンドの事前確認］

- Python Schema：python tooling/schema_check/check_schemas.py
- Conformance：python tooling/conformance_tests/run_conformance_skeleton.py
- Manifest：python tooling/manifest.py --check
- Python統合：python tooling/validate_all.py --python-only
- Rust隔離：python -X utf8 tooling/run_cargo_isolated.py test --locked --offline --manifest-path native/rust_helper/Cargo.toml
- Windows Flutter：python tooling/validate_all.py --flutter-step desktop_flutter_test --desktop-platform windows
- Flutter解析：python tooling/validate_all.py --flutter-step desktop_flutter_analyze --desktop-platform windows
- Strict Release：python tooling/release_gate_check.py --strict-release
- Git差分：git diff --check

利用OS・Toolchain・現行CLIの正しい引数を優先し、存在しないcommandを捏造しない。pythonが存在しなければpython3等を適正環境で使用。Rust／Flutterの全対象はBUILD_COVERED後と重要統合Batchで実行し、小単位ごとの全実行はしない。Strict Release FAILは現行未出荷条件の正常な検出であり、Gateを緩めてPASSにしない。日本語基底strictも検出対象を勝手に減らさない。

## 5. 既存検証・証拠の精度と再検証

証拠分類＝CONFIG／INTERNAL_STATE／FIXTURE／LIVE_RUNTIME／LIVE_PRODUCT／EXTERNAL_EVIDENCE／NOT_RUN。Source・run・artifact・OS・実行者を識別。

当時の固定値：
- 現行履歴はW00＝SCOPED_PASS、W01＝Windows/Codex CLI限定CLOSED、W02＝OPEN、W03〜W08＝PLANNED、B5 OPEN、B6未成立、Q0〜Q7 DEFERRED。
- Release Registryは18件中16 unresolved・2 resolved、release_ready=false。
- W02-B02 Flutter fixture consumer WIRED、W02-B03負例fixture PASS。Windows実Codex→実Tool→AI後続TaskはNOT_RUN、installed UIハングの根因はUNKNOWN。
- 基準HEAD日本語strict＝41 files／281 findings FAIL。対象集合と技術Tokenの意味分類が残る。packaging portabilityはPASS。Schema 172／168正常例／223負例、Conformance 270 PASS。C0〜C31対応構造監査PASS、C32〜C34未監査。
- 過去Rust library suite＝541 passed／8 failed／13 ignored。偽CLI引数の5件は別途focused PASS。3件の未再検証名＝process_launch_rejection_is_fingerprinted_without_raw_command_content、agent_cli_owner_summary_does_not_claim_another_platforms_boundary、package_platform_mismatch_and_cross_platform_payload_are_rejected。修正後の全Rust suiteは未再実行。
- 過去Q0/Q1（2026-10-07）の凍結commit ab746d4b33b003761f5ddcef60a88afa97132a40は現候補QCに転記不可。FQ-TEST-LOOPBACK等の根因未解明を旧PASSで閉じない。

現在HEADで上記が変化したら、変更源・scope・対象候補を照合して**新状態を追記**。旧文書・失敗・ハッシュ・旧版の判断を遡及書換えしない。READMEやClaimでの一律SUPPORTED／UNSUPPORTEDはRuntime・OS・mode・Sourceを付して矛盾解消。W01のWindows限定supportedを別OSへ転記しない。

## 6. 要求全数・最終受入の照合命令

- N01〜N11＝全件。W00〜W08＝全Gate。旧P0〜P13、C0〜C34／Phase0〜45、BUF-R01〜R09・T01〜T24、B0〜B6、D4-QE-R01〜R13、D4-LNX-T01〜T08、D4-AUD-F01〜F07＝照合表へ全数残す。
- 旧FQ-R2-A〜H・FQ-R2-TEMP・FQ-MODULE-PRUNING・FQ-TEST-LOOPBACK・FQ-P11-UNINSTALL-FINALIZER-CLEANUP・FQ-INTEGRATEDを消さない。Q0〜Q7の必須試験対象を減らさない。
- Release Registryの18IDを全件照合。歴史的owner_goは出荷責任者GO、rev2_export_owner_ui_authority_pathは製品使用者の認可経路と解釈。識別子を勝手に改名・削除しない。
- 全線で「実装あり／実配線あり／実物動作／実物失敗／未実施／権限待ち／正式QA可否」を独立記録。Schema・fixture／unit PASSを通常製品のLIVE_PRODUCTへ昇格しない。
- FQ-R2、Runtime相互運用、Compose・Export独立アプリ、Module除外・実測、Windows別profile実製品、Linux native、macOS、Android・iOS実端末の必要証拠を得る。実機不可なら対象QA_NOT_ESTABLISHEDを維持し、ソースと実装を先に完成。
- Windows-firstの独立技術完成と、全OS必須製品完成・正式配布を混同しない。最終Strict Gateと責任者GOが揃うまでrelease_ready=false。

## 7. Git・履歴・正本施工

1. 本Repo単一writer／direct-main。着手前にremoteとlocal HEAD、dirty差分、保全済み作業・一時branch・active writerを実観測する。既存変更・他者ブランチを無断削除／上書きしない。
2. 編集を伴う**意味のあるBUILD_BATCHの直前**、AGENTSの規則でremote refs/tags/codex/backup-main と refs/tags/codex/backup-main-prev の二世代異なる復旧点をrotate／verify。tag保全はremote branch追加と異なる。main／tagの既存履歴を破壊せず、各小単位で無益に回転しない。
3. 対象批判箇所だけ修正し、局所compile・必要な正負・安全負例→BUILD_BATCHの実consumer結合／Git diff check／Manifest→必要な検証→commit→direct-main push→git ls-remote origin refs/heads/main の一致確認→tree整合を記録。未確認なら完了と報告しない。
4. GitHub Actionsは外部OS・重い検証の補助に限る手動workflow_dispatchのみ。必要な検証branchの有効期間、単一writer／成功commit fast-forward／後始末を守る。push-trigger CI／PR作成／CI greenによるRelease宣言は禁止。
5. OneDrive同期sourceへ生成cache・大規模中間物を置かない。Source snapshotとartifact・evidenceのhash／時点／実Hostを同一性として保持。既存ユーザーデータ・秘密・復旧点を破壊しない。
6. v3を実施工としてRepositoryへ取り込む際、旧2026-10-22、11-05、11-08の3点・採用記録・当時のSHAを**削除せず**残す。新v3の版・受領hash・採用判断・現在適用範囲を別の履歴行として記録し、AGENTSの「現行3点」参照・正本索引・進捗・READMEを整合更新する。旧原本と採用差分を新文書の生成によって遡及改変しない。
7. 他Repoのコード・公開Issue/PR・外部送信・本人のproduction鍵／署名は今回の施工権限外。安全・権限・法令・OS保護の拒否を別経路で回避しない。

## 8. 状態機械と停止単位

| 状態 | 判定と許可する次作用 |
|---|---|
| AUTHORIZED_CONSTRUCTION | Aに属する通常開発。再承認不要、継続 |
| AWAITING_EXTERNAL_INTERACTIVE_GATE | 実在する外部規則が要求する特定一操作のみ一時停止。本人確認が必要。非依存実装を継続 |
| PRODUCT_AUTH_PENDING | 製品内で使用者の新規Permission／Approvalが不足。その実Tool作用のみ拒否。開発確認と無関係 |
| RESPONSIBLE_DECISION_REQUIRED | 仕様外・不可逆・公開・正式署名・出荷等。該当決裁だけ待つ |
| TECHNICAL_FAILURE | 実装／接続／試験／UI／環境等の不適合。Codexが原因・変更・再試験を実行 |
| BUILD_GAP | 必須Producer→Consumer欠落。該当施工線の実装を進める |
| BLOCKED_AUTHORITY | 必須本人資格・OS保護・権限の未取得。迂回せず対象だけ未実証にし他線続行 |
| SCOPED_PASS／CLOSED | Source・Artifact・OS・Runtime・実作用・証拠範囲を必ず付記 |
| QA_NOT_ESTABLISHED | 対象機能・OS・正式候補に必要な実証が未成立。PASSではない |

完全停止すべき対象＝Authority漏出・秘密漏出・破壊的なユーザー資産消失・認証IPCの迂回・Audit不能時の実作用完了、未委任の不可逆外部変更、重大なGit保全破綻。停止対象をできるだけ局所化。一般の実装困難性と実行不能性を混同せず、他線・他OSのソース実装を止めない。

## 9. Codexの最終報告構文

第一文＝「AIが実際に何をできるようになったか／できないか、元Toolが作用したか、実結果をAIのどの次Taskが消費したか」。

各Batchの機械出力：
- 原目的／主語／指示者・使用者・実行主体・技術責任者
- 旧ID群→N01〜N11→施工線A〜J→Gate W00〜W08→Q段階
- Before／After source SHA、変更ファイル、実Producer→Consumer、実結果、権限・Audit・Recovery
- 使用者の製品Permission／Approvalと、開発A/B/C確認の**別記録**
- 検査命令・exit・PASS／FAIL／NOT_RUN、FIXTURE／LIVE_RUNTIME／LIVE_PRODUCT、実行Host／artifact hash
- 旧CLOSEDの保持条件・新FAIL／未知根因・Blocker・証拠の版別帰属
- git branch／commit／push／remote HEAD／二世代backup tag／rollback point／worktree
- 全要求のBUILD_COVERED達成／未達、対象別QE_QC・QE_QA・release_ready・正式出荷GOの独立状態
- 次の**横断施工線**と残る真の停止対象。単に「次Wへ進む」とは書かない

この施工の終了判定＝一括で全機能を実装し全必須consumerを結合できたときBUILD_COVERED、同一候補の実物QC・対象別QA・安全と証拠により製品技術品質、最終責任者決裁により正式出荷。この3条件を混同せず、Wの数字順・確認回数・文書量・テスト数を仕事の成果に置換しない。
