# D4Pocket 統合開発工程表 v4.0

文書ID=D4-QE-ROADMAP-20261113-V4
対象=gatchimuchio/D4-POCKET／main
作成=2026-11-13 JST
基準観測=d503a4ef6daac8942abd1cb011442ce28c78ec55
状態=設計改訂成果／未取込／未施工／全体合格を主張しない
上位=01_D4Pocket_統合製品実装仕様書_v4.md
同版施工=03_D4Pocket_Codex施工指示書_v4.md
工程定義=全製品一巡製造→接続→全体構成確認→局所/結合QC→対象標準環境QA→出荷責任者判定

## R00 主語・目的・工程の最上位規則

R00-01 製造指示者=ご主人様。製品設計者=チャッピー。施工者=Codex。実仕事主体=AI/LLM/Agent。製品使用者=エンドユーザー。製品実権限=Rust Broker。出荷責任者=ご主人様。役割兼任は権限共有を意味しない。
R00-02 目的=一般的な標準OS環境の使用者がOSに適した配布物をダウンロード→インストール→起動→サービス利用→AI実Task→Tool/外部Runtime作用→実結果再消費→成果/停止/回復を、自分で開発用セットアップせず成立させること。
R00-03 対象=全N01..11・旧P0..13・C0..34・Phase0..45・K01..20・BUF-R01..09/T01..24・B0..6・D4-QE-R01..13・D4-LNX-T01..08・D4-AUD-F01..07・全FQ・W00..08・Q0..07・全18 Blocker・Windows/macOS/Linux/Android/iOS。過去の実装・証拠は保持し、旧番号は要求ID/履歴/受入座標であって製造順でない。
R00-04 工程禁止=機能1件→全試験→検収→次機能、W00→W01→W02→…→W08のCLOSED待機列、OS1個完成→別OSソース着手、B5後回し、UI/Schema/Fixture/Manifestだけの完成宣言。
R00-05 工程成立=全要求の実Producer→実Consumerが同一構成に存在するBUILD_COVEREDを製造完了座標とし、その後同一候補を局所検査・結合検査・QC/QAする。全体試験をBUILD_BATCH単位で反復しない。
R00-06 即時安全例外=認可/Permission/Approval/秘密/Content Exposure/Audit/IPC/Path/Host隔離/非冪等replay/自動適用/kill/scratch/配布署名に影響する差分はBUILD中でも負例と停止をその場で確認。FAILなら対象作用は停止・隔離、無関係施工は継続。試験そのものが製造開始Gateにならない。
R00-07 主工程を技術段階と保証段階に分離。Source有、実配線有、installed実動有、QC合格、OS条件QA合格、Release GO、公開済みは相互に独立した状態。
R00-08 現在の工程進行度をcommit数・テスト件数・局所CLOSEDから算出しない。完成率算出の母数/重みの新規定義なく%を捏造しない。欠落件数/必須consumer接続件数/OS別実証件数を出す。

## R01 統合工程台帳と停止規則

| 工程座標 | 一巡施工/検査内容 | 終了条件 | 次工程への意味 |
|---|---|---|---|
| E0 QE_DESIGN | 原文/現行repo/歴史的ID/仕様・権限/OS標準母集団/Producer・Consumer/異常時/証拠の全数照合。既存W00を差分利用 | 全要求に実装先・実Consumer・独立証拠/制約が定義され、UNKNOWNが列挙 | E1の構成一括着手。資料の整備で仕事が終わった扱いにしない |
| E1 QE_BUILD_WHOLE | 全A〜Jに初期着手。共通型・Broker・AI・各Adapter/UI/Export/全OS native・installer/Update/Buffer/監査を既存実装の上へ一巡で組上げ | 全要求に接続可能な実製品構成が揃う。仮製品やmockを完成扱いにしない | E2接続へ。E1中も順次実Consumerを繋いでよい |
| E2 QE_CONNECT | 各Producer→各実Consumer、認可/状態/結果/Recovery/各OSパッケージを横断接続。代表実仕事の通電相当の観測 | SYSTEM_SHAPEDを観測し、さらに全要求の未配線を追跡 | SYSTEM_SHAPEDはE3の早期検査座標、他線の起工Gateではない |
| E3 BUILD_COVERED | 全N/K/P/C/Phase/BUF/B/FQ/OS配布・使用入口と18 Blocker条件を実コード・実consumerで網羅照合 | 必須BUILD_GAP=0、Source/Contract/Consumer/停止/Recoveryの全数表が埋まる | 製造完了。ただし動作品の品質/OS保証は未成立 |
| E4 QE_QC | 入口でQ0として統合候補SHAを固定。静的/動的/異常/負例/依存差/安全/資源/性能/長期/導入/Update/撤去/副作用の系統検査 | 同一候補の実測・不適合是正と回帰・残余が確定 | 根因に応じE0/E1/E2へ戻るが無関係な完成状態を無効化しない |
| E5 QE_QA | Q0固定候補のQ1〜Q6、対象OSのVM/Emulator/Simulator/Hosted、Windows実機確認、再現証拠集約 | 宣言した保証範囲ごとにQA_SCOPED/QA_NOT_ESTABLISHED/OUT_OF_SCOPEを客観判定 | 物理Mac/Android/iPhone調達は要求しない |
| E6 QE_RELEASE_REVIEW | Q7出荷判断、18件のRelease Registry、公式配布条件、署名・Publisher・鍵・本人GO、安全・監査を独立審査 | 公式Registryと現候補が整合し責任者が正式GO | 未GOはrelease_ready=false。技術完成や配布候補生成と別 |

工程期間は依存関係・施工量に応じた可変時間。日付ありきで局所完了を捏造しない。E1/E2は実務上重複可、E3/E4/E5の正式合格順は混同不可。構築全線への着手をQ/OSの証拠取得まで待たせない。重大安全破綻は該当作用のみ即停止。

## R02 E0：観測と構築基線を一度で固定

E0-01 最新のユーザー原文、本v4の仕様→本工程→指示、AGENTS.md、規定/00_日本語基底規定.md、規定/正本索引.json、現行採用記録、main HEAD/dirty/他writer/二世代backup tag/Release Registryを取得。資料正本・施工正本・履歴証拠を分離。
E0-02 原典索引=docs/総合機能拡張_rev1/、docs/D4_POCKET_PHASE_MAPPING.md、docs/PRODUCT_COMPLETION_PLAN.md、docs/REV2_PROGRESS.md/REV3_PROGRESS.md/REV4_ACCEPTANCE_LEDGER.md/REV5_PRODUCT_PROGRESS.md、docs/QE_2026-10-22/、docs/QE_2026-11-05/、docs/QE_2026-11-08/、docs/QE_2026-11-09/、docs/緩衝基盤_rev5補遺_QC/、docs/FINAL_QA_QUEUE.md、docs/specs/、specs/、packages/、native/、apps/、installer/、tooling/、.github/workflows/。
E0-03 F07旧rev5原本照合記録（2026-10-30、当時の原本SHAと差分0）を履歴として使用。原本実ファイルが無い場合、現v4作者が全文取得したと主張せず、欠測と現行要求との差分照合を分離。旧PASSを新SourceのPASSに遡及しない。
E0-04 要求ledger列={現ID,旧ID,元原文/正本参照,対象,目的,Producer,実Consumer,契約/データ,通常作用,否定作用,停止/復旧,標準OS候補,配布物,既存実装,BUILD_GAP,SourceSHA,TestClass,証拠ID,Blocker,状態,残余}。全数表に空欄があればUNKNOWNと明記。IDを丸めた「主要機能」として欠落を隠さない。
E0-05 既存のコード/ロジック/Fixture/QA/Bitrotを再利用可能性で分類＝REUSE_EXACT/REWIRE/REIMPLEMENT/REMOVE_CONFIRMED_NONREQUIREMENT/UNKNOWN。原要求に含まれる機能を削る権限はない。不要な書き直しで開発を遅くしない。
E0-06 標準OS保証母集団はOS名だけでなく、対象version・arch・OS通常security・GUI/session・Runtime・最低resource・配布方法・更新/撤去方式・仮想検査媒体まで記録。8GB Windows代表値は候補に含め、他OSの閾値は未測定のまま確定値にしない。
E0-07 共通契約の型/認可境界を決めてから並行BUILDへ展開。設計からの変更は原因/影響範囲/新Contract/全Consumer修正/既存合格の扱いを同時記録し、局所的な迂回実装を避ける。
E0-08 E0完了で旧W00を何度も全件再実行しない。要求追加・新証拠の差分のみ取り込む。資料の正本切替は採用状態を別記録し、古い3点・監査報告は削除しない。

## R03 E1/E2：A〜J全施工線を同じBUILD期間に組上げる

| 線 | 主語/Producer→実Consumer | BUILD時に必ず作る実経路 | 必須旧要求 |
|---|---|---|---|
| A AI実仕事 | 人間目的→Agent/Task/Session→Rust Broker→CLI/MCP→結果→同AI次Task | Runtime-neutralな自律Tool使用・結果依存追加Task・停止/監査/復旧。FlutterにAIの内部Controllerを閉じ込めない。既存Windows Codex経路を継承 | N01/N02、P0/P1、W01/W02、K01/K08 |
| B Runtime/A2A | Runtime Catalog/Adapter/Agent Card→2種以上実Runtime・A2A/Host→成果Diff/Handoff/Message | Provider・MCP/A2A実サービス・Agent間Artifact限定受渡し、権限Host跨ぎ拒否/失敗復帰 | N03/N04、P2/P3/P4/P6/P7、C15..19、K02/K06/K11/K12 |
| C 製品操作 | Flutter/Native UI→Broker実state/Task/Workspace/History/Eval/Process→使用者 | Agent Center、Workspace Inspector、Diff/Trace/Metric/Problems/Evaluation/Recovery/Tray/Palette/Search/Notification。UI freeze・Timer終了を設計根因で回収 | N04/N09、P2/P3/P5/P12、C1..6/C12..14/C20..23、K03/K04/K05/K10/K13 |
| D 能力/編集 | 実Catalog→Capability依存/互換→Compose→実Tool。AI proposal→独立検査/許可→Apply | 実在するCapabilityと実Tool作用の一体化、AI Edit自己承認禁止、Rollback/Review | N05/N07、P8、W04/W05、K14 |
| E 独立製品 | Compose→Module選択/実除外/独立identity→Installer/Export App→使用者実Task | Manifest実消費、安全Core維持、選択moduleだけbinary/resource含有、実サイズ/起動/権限を測るための計測点 | N06、P9/P10、W05、FQ-MODULE-PRUNING、K15 |
| F 安全/Buffer | OS/依存/Toolchain/TLS/OneDrive/Cache/process差→Buffer→元Task/副作用 | B0..B4再利用、B5開発へ自己適用、B6全Consumer結合。Broker default-denyと監査、停止/復旧/診断・根因帰還 | N08、BUF-R01..09、BUF-T01..24、B0..6、K07/K20 |
| G Windows製品 | Windows native/Flutter/Rust→Installer/Startup/SetupDoctor→AI仕事→Update/Repair/Uninstall | 一般Windows使用者の.exe install/登録/別profile初回/所有実機確認可能な本番導線。portable/dev状態を除外 | N10、P11/P12、W06、FQ-R2、K16 |
| H macOS/Linux | Native Keychain/secret/IPC/session→Flutter/Core/Broker→.app等・Linux package→実GUI | Linux native X11/Wayland/標準distro、macOS sandbox/Swift/Keychain/署名準備。どちらもOS正規導入/更新/撤去 | N10、P13、W07、LNX-T01..08、K17 |
| I Android/iOS | Android Keystore/iOS Keychain・Native Device Link→Broker/Flutter/Agent→OS通常配布/利用 | APK/AAB及びiOS許可された配布/初回設定・Lifecycle・Permission/秘密/Task継続/更新・撤去。実OS emulator/simulatorの評価導線 | N10、P13、W07、C24..26、K17 |
| J 構成/証拠 | Source/Contract/Test/実作用/OS/Artifact→Requirement Ledger/Release Gate→責任者 | Schema/Conformance/監査/形式整合/OS matrix/18 Registry/全Q/FQ/バージョン付き証拠。旧資料改竄なし | N11、W00..08、Q0..07、C27..34、K19 |

R03-01 全A〜JはE1初期母集団へ**同時投入**。単一writerに抵触しない工程内局所順序/同じファイルの逐次編集は許す。安全Contractや実コンパイル依存があるからといって別の独立線を何日も待機させない。
R03-02 最初の製造一巡でSource/Interface/IPC/Native Bridge/Flutter UI/CLI/Tool/Agent/Export/Installer/Update/証拠取得点まで全体系に設ける。必須の本番経路が仮stub/模擬入力で代替されればBUILD_GAP（独立した検査Fixtureは保持可）。既存実装を引き継ぎ、空の雛形を「全体躯体完了」と呼ばない。
R03-03 接続は製品から見た全体の実作用を基準とする。Agentの実Tool要求→認可→作用→結果→同Agentの追加行為→Audit/Recovery、異種Agent・A2A、Capability/Compose、AI Edit、独立Exportを同じRust Brokerの権限境界へ接続。UIの描画だけ・manifestの連鎖だけは禁止。
R03-04 OSの差はBridge/Adapterへ閉じ込め、Core/AI意味/認可/責任/監査/Release identityを分岐ごとに失わない。言語一本化を前提にせずRust/Dart/Swift/Kotlin/必要Adapterを契約で分離。
R03-05 Windows-first=使用者所有Windowsで製品最終確認できる技術/検査面の優先。macOS/Linux/Android/iOSの**ソース施工・配布実装をWindows完成後まで延期する指示ではない**。
R03-06 A/B/C/F/G共有境界のW02 UIハング/結果依存問題は製品全体の接続不良として根因へ戻す。W02の単独fixtureを反復してB/D/E/H/Iを停止させない。AのSYSTEM_SHAPEDは観測点であり、既存W01限定CLOSEDの再拡大ではない。

## R04 BUILD_BATCHの製造単位・構築中の検査・Git

R04-01 BUILD_BATCH=共通Contract変更＋全影響Producer/Consumer＋安全即時負例＋同じ意味のOS/Installer/Docs/Evidence接続を一組にした横断施工単位。例: Capability/Permission/Result契約改訂ならShell/Rust/MCP/A2A/Flutter/Export/OS Bridgeを一括差分に含む。1画面/1テストの合格はBUILD_BATCHの終了条件にならない。
R04-02 バッチ作業前=main/remote HEAD/dirty/owner差分/単一writerを実確認。保全対象2世代tag refs/tags/codex/backup-main と refs/tags/codex/backup-main-prev を現AGENTSの手順で意味ある変更前に保全・検証。二つを同じcommitへ潰さない。既存tag/履歴/ユーザーデータの無断消去禁止。
R04-03 正常系確認=影響Scopeのcompile/Schema/Contract/Consumer実在、必要な少数の実作用トレース。安全境界変更=対象負例を差分時に即検査。標準OS導入候補の完成前は各OSの全QAを繰返さない。
R04-04 非クリティカル試験FAILは実装修正計画へ入れ、同じ原因が別Consumerで再現するかを検査。権限/秘密/Audit破綻なら危険作用だけ停止し、無関係BUILD継続。新規外部権限や公開・署名はC判定へ隔離。
R04-05 バッチ完了=変更差分の整合・実Consumerへの接続・必要な即時安全負例・source版/証拠・回復点・git diff --check/remote SHA照合・次の横断未接続を記録。各数行の編集ごとに全Suite/Q0..Q7/全2世代tag回転はしない。
R04-06 Direct-main/single-writer/PRなし、push-trigger CIなし、既存Actionsは手動workflow_dispatchのみ。リポ施工時のpush権限は施工指示と現AGENTSの許可範囲に限り、外部再許可が必要な操作を回避しない。E0で既存未コミット成果を保全。
R04-07 OneDrive同期領域をbuild/cache/tmpに使わず隔離。Cargo/Flutter SDK/cache/Windows SAC・OneDriveロック・loopback/TLS等はBuffer B5を同時適用し根因と残余を残す。retry/異HostのPASSだけで修復としない。
R04-08 関連する全Sourceがまだビルド不能な途中状態は工程上TRANSIENTと記録可。長期に無関係線を永久停止させず、E3候補へ向けて全体系を収束させる。境界未定義と権限欠落を無理にmockで隠さない。

## R05 BUILD_COVERED全数条件

| 母集団 | 件数/対象 | 必須Consumer条件とE3判定 |
|---|---|---|
| 新目的 | N01..N11=11 | 全要求に production Input→Broker/Adapter/Action→Result→AI/UI/Release consumer、否定/回復接続が存在 |
| 原始要求 | P0..P13=14 | 旧W/証拠を保持し、新Nへの既存能力と欠落Consumerがすべて結合 |
| 総合拡張 | C0..C34=35／Phase0..45=46 | docs/D4_POCKET_PHASE_MAPPING.mdの逐行を新構造へ対応。C32..34等を未観測のままCLOSED化しない |
| 既存製品群 | K01..K20=20 | 実Product FeatureとBroker/GUI/Tool/Export/OSに consumer を持つ |
| 緩衝 | BUF-R01..09=9／BUF-T01..24=24／B0..6=7 | B5自己適用を実構成へ施工、B6全系検査の配線と測定器。旧B0..B4限定CLOSEDは保存 |
| QE/監査 | D4-QE-R01..13=13／D4-LNX-T01..08=8／D4-AUD-F01..07=7 | 旧監査根因・制約・Linux native・二世代tag・正本索引を実Validator/Traceへ接続 |
| Q/FQ | Q0..Q7=8／FQ-R2-A..H, TEMP, MODULE-PRUNING, TEST-LOOPBACK, P11-UNINSTALL-FINALIZER-CLEANUP, INTEGRATED | 自動/手動の試験手段が実作用、証拠class・結果に到達。Q合格自体はE5で判定 |
| OS配布 | Windows/macOS/Linux/Android/iOS | OS標準環境の実配布候補→install/first-run/service/AI実仕事/Update/撤去までの実コード・Build recipe・同一性取得点 |
| Release | 現Registry18件 | 全IDに現契約・対象OS・検証方法・証拠class/Owner決裁が接続。旧Statusと歴史を維持 |

E3式=BUILD_GAP_COUNT（必須実Producer/Consumer/拒否/回復/Package経路の不在）=0。実装の存在だけをもって実行保証・品質合格としない。署名/Store/実行環境不可/本人決裁等はBUILD_GAPではなくAUTHORITY/QA環境別の残件として正しく区別するが、OSパッケージ生成経路の未実装はBUILD_GAP。

## R06 W00〜W08：受入座標へ再定義

| W | 受入で見る仕事 | 基準観測/施工線 | 独立工程制約 |
|---|---|---|---|
| W00/G0 | 全原文・旧実装・要求台帳・consumer/安全/OSの全数 | SCOPED_PASS、E0/Jで差分維持 | 毎回全数棚卸しを繰返さない |
| W01/G1 | Windows normal Release・Codex CLIの一仕事+Permission拒否/Cancel/Deadline/Audit | 限定CLOSED、A/G | 新Runtime/新候補/全OSへの転記禁止 |
| W02/G2 | AI→実MCP Tool→結果依存AI次Task、Broker/使用者許可/Audit/UI応答 | OPEN、A/C/F/G | E1〜E3で他全線と同時製造。Fixture PASS≠通常installed |
| W03/G3 | 異種Runtime実Task2種+外部A2A/Handoff/Compare | 未合格、B | W02のCLOSED待ちをやめる |
| W04/G4 | 現実在Capability→依存/互換→Compose→実作用 | 未合格、D | Catalog宣言だけをPASSにしない |
| W05/G5 | AI Edit非自己承認・別実行者のApply、独立Export/Pruning | 未合格、D/E | 旧Proposal/Manifestだけでは不成立 |
| W06/G6-WIN | Windows installed一候補・初回/Doctor/Tool/Update/Repair/Rollback/Uninstall | 未合格、G | W02待ちでInstallerを作らない状態を禁止 |
| W07/G6-OS | Linux native/macOS/Android/iOSの通常配布→実利用 | 未合格、H/I | 標準OS仮想環境で評価。物理非Windows端末要求しない |
| W08/G7 | Buffer B5/B6と全QC/QA/全証拠/18 Blocker/出荷責任者GO | 未合格、F/J | ソース施工は初期から。正式検収は後段 |

W00..W08各CLOSED条件はSource/Release Candidate/Artifact/Runtime/OS/許可/実Tool/AI消費/Auditの射程を伴う。旧W02-B03 fixture PASS・W02-B04 LIVE_PRODUCT NOT_RUN等の歴史を保存。全WのCLOSED達成をE1着手順序へ戻さない。

## R07 OS標準環境検証計画：実機試験の要求を再配置

R07-00 旧v3の「macOS実Mac」「Android物理端末」「iPhone実端末」「Linux別物理端末」を全OS共通のQA必須条件へ残さない。新主語は「宣言した一般・標準OS構成で製品を使う使用者」。テスト機材はそれを再現する手段であり目的でない。定義・仮想検証・証拠だけで保証境界を判定。Apple OSのVM/Simulatorはプラットフォーム条件遵守が前提。

| OS対象 | 配布物と通常UX | 標準環境の検証媒体 | E5の必須観測 | 実機方針 |
|---|---|---|---|---|
| Windows | .exe等正式installer→Start Menu/起動→基本サービス | 実Windows VM、clean別profile、指示者Windows PC | installerの真のSource/Artifact identity、初回/Doctor/Broker/AI仕事、認可/secret/Update/撤去 | 所有Windows PCで完成品確認。実機を設計調整の第一原因としない |
| Linux | 対象distro/archのnative package/installer→GUI起動 | 実distro full VM/hosted native session、宣言X11/Wayland | native IPC/secret/file/process、install/AI/Update/Uninstall、D4-LNX-T01..08 | 他の物理PC不要。WSL単独やXvfbだけは本物証拠の代替不可 |
| macOS | 通常macOS .app/インストーラー→起動→利用 | 適法なmacOS VM/hosted macOS GUI（必要な実API/Keychain） | native consent/IPC/Keychain/sandbox、install/Task/Repair/Update/撤去、署名状態を別明示 | 物理Mac取得不要。実OS仮想実行できなければQA_NOT_ESTABLISHED |
| Android | 対応配布導線→インストール→アプリ利用 | Android Emulator、対象ABI/API/Keystore/Permission構成 | install/first-run/Device Link/Broker/Task/秘密/復旧/更新/撤去 | 物理Android端末不要 |
| iOS | Apple許容の通常配布→インストール→利用 | iOS Simulator、対象OS/API/Keychain/Permission構成 | build/インストール/first-run/Device Link/Task/秘密/復旧、Simulator非対応能力を明示 | 物理iPhone不要。Simulatorで未再現の固有条件は保証未成立と記録 |

R07-01 E0では各OS/version/arch/ABI/最低メモリ/基礎GUI/セキュリティ条件/Package形式を候補定義し、E5/Q0でRelease候補の宣言保証範囲として固定。特定版やminimum値は実観測前に捏造しない。全メーカー・カスタムROM・改造kernel・特異周辺機器まで保証しない。
R07-02 仮想でもOS-native API/ABI/Permission/Credential/Process/Installer/GUI/更新/撤去を実装した**インストール済み製品候補**として実行する。Cross-compile・Source test・WSL・Xvfb・Mock・単一Fixtureは一部の契約証拠であり、「そのOSで使える」の全根拠にはならない。
R07-03 検証媒体の不能/クラウド資格不足はQA_NOT_ESTABLISHEDまたはBLOCKED_AUTHORITYで対象だけ保留。ユーザーに別OS実機調達を新たに要求しない。製品のOS Source/Installer構築は継続。
R07-04 各OSの標準環境でのインストール/初回起動/Setup/実仕事/監査/拒否/取消/再起動/Update/Repair/Rollback/Uninstallを候補とOSを結び記録。同一UXの意味を保持し、OS固有permission UIを一般体験の例外として扱う。
R07-05 macOS/iOSの署名/Notarization/App Store審査等の「配布資格」は物理実機とは別の独立Release条件。未取得を虚偽に保証せず、利用者に別途開発者Buildを要求しない設計を維持。
R07-06 第三者ユーザーがどのOSでも通常導入できることを目標とし、Windowsで動くexeが全OSで動くと誤解しない。OSに合った一般的パッケージを渡し、配布条件を公開文言と一致させる。
R07-07 iOS Simulatorなどで配布用device binaryを直接導入できない場合は、同一SourceSHAからSimulator用実行Artifactとdevice配布用Artifactを分け、両ArtifactSHA・ABI・build configuration・Entitlement/署名条件の関係を検証。Simulatorの正常実行と配布物の静的妥当性を併記し、device版の実動作そのものを観測したと偽らない。Androidでも対象ABIとEmulator ABIの差を保持。AppleはRelease buildを物理端末で試すことを推奨するが、本工程は使用者指示により物理iPhone必須Gateを設けないため、端末固有の未証明を保証へ混入しない。

## R08 Q0〜Q7／QC・QAを全体施工後に実施

| Gate | 検証主語・目的 | 試験と証拠 | 不合格時 |
|---|---|---|---|
| Q0 候補固定 | J/Owner：対象範囲とSource、各OS/Artifactの同一性確定 | SourceSHA、contract/manifest、release artifact SHA、OS保証マトリクス/正本、18 Blocker | E4入口に実施。候補同一性未確定はQC/QAを合格認定しない |
| Q1 全体系正常/拒否 | AI/Broker/実Runtime：一仕事→結果依存連続、異種A2A、Compose/Edit/Export | 実Producer/Consumer/Tool/Task/Permission/Audit/使用者画面。無許可・未知Tool拒否 | 共通原因へ戻り影響Consumer全数修復 |
| Q2 異常/Recovery | Broker/Buffer：Crash/Cancel/Deadline/loopback/OneDrive/cache/kill/scratch/Host差 | FQ-R2、BUF-T全件、負例・副作用、Recovery再開、同条件復帰 | 元原因・設計に戻る、単純retry PASSで閉じない |
| Q3 認可/秘密 | Broker/Native：Permission/Approval/credential/IPC/Path/audit tamper/Content Exposure | 一回・期限・identity・Host・OS security、deny/改竄/ログ終端 | 影響する危険操作を即停止し境界再検査 |
| Q4 耐久/性能 | Shell/UI/Process/Export：通常と異常の持久・資源・サイズ | 8GB級Windows代表、対象OS resource、Module Pruning実測、メモリ/CPU/process leak/起動 | 実測に基づき改善、未測定の数値は仕様化しない |
| Q5 OS/Install | 各OS使用者：通常導入→AI仕事→更新/復旧→撤去 | R07全OS仮想実物＋Windows所有実機確認、Package/署名条件、native権限 | 対象OS QA_NOT_ESTABLISHED、物理機材要求へ転嫁しない |
| Q6 証拠統合 | J：原文/全ID/実Source/実結果/OS/Artifact/権限/Gateを照合 | QA_SCOPED等、Trace、Blocker 18、README/CLAIM/Registry検証 | Missing/UNKNOWNを残し、隠さず原因へ帰還 |
| Q7 出荷判定 | 責任者：保証/安全/公開/法的資格/運用判断 | strict release gate、Owner GO、署名/更新/rollback、公開先 | 資格・GOなしは出荷停止。構築・独立trackの結果は保存 |

R08-01 Q0..Q7は**同一Release候補の全体系受入**。E1〜E3の小さなBUILD_BATCHごとにQ0→Q7を起動しない。重大安全負例は前倒しで即検査してよいがQ3正式合格には統合候補の全証拠が必要。
R08-02 E4/QCxの細部試験は初回実配線の全体結合完了後に集中。局所不具合を解消したら影響Consumer/失敗/副作用の範囲に比例して再試験し、無関係範囲を無用に繰返さない。原理変更で影響範囲が拡大すれば必要な回帰は増やす。
R08-03 Release Registryやvalidatorの既存物理端末要求は新R07規則に合わせて目的・範囲・実証方法・履歴を全部改訂。安全Gateや配布資格を「実機不要」を口実に無効化しない。
R08-04 Q5でWindows実機を使用者が最終確認する際、設計者・Codexが実機結果だけに合わせて条件を後付け変更しない。異常が出れば同条件のVM/Contract/Native境界へ帰還する。
R08-05 本番資格/公開/GOのC区分不足を技術失敗として隠さず、技術品質の未成立をOwner決裁不足へすり替えない。

## R09 FQ・旧監査・Bufferを検証へ接続

FQ-R2-A..H=Cancel/Deadline/Crash/Scratch/Content Exposure/Windows通常Release・Broker/identityをA/F/G/J、Q2/Q3/Q5へ配置。
FQ-R2-TEMP=外部Tool TEMPとD4所有scratchの境界をA/F/Q2へ配置。
FQ-MODULE-PRUNING=E/G/J、Q4/Q5、binary実除外/体積/起動/必要module稼働の比較。
FQ-TEST-LOOPBACK=B/F/J、Q2、ConnectionReset等の発生原因・Host/Network/ABI/TLSの同一条件を比較。単純なHost乗換成功で解決としない。
FQ-P11-UNINSTALL-FINALIZER-CLEANUP=G/J、Q5、Windows installer一時finalizer・失敗後後始末の証拠。
FQ-INTEGRATED=J、Q0..Q7の統合候補で判定。

BUF-R01..09/BUF-T01..24の全件はS06原定義を維持してF/B5/B6へ接続。B0..B4は旧限定CLOSED、B5 OPEN、B6未成立を維持してBUILD初期からself-application→結合検査に転換。T01..03/T12/T17/T19..24は全数・意味/根因、T04..11はOS/障害/安全、T13..16/T18は証拠/Package/自己適用/全体QCへ配置。
D4-AUD-F01=Linux相対path否定維持、F02=復旧tag二世代、F03=OS証拠表示の同一性、F04=日本語基底strict、F05=single-writer/枝回収、F06=正本索引、F07=rev5原本照合歴史。過去の是正を撤回しない。
D4-QE-R01..R13とD4-LNX-T01..T08は原定義と試験コードを読み、現保証条件だけを正当な設計変更として更新。C0..34/Phase0..45を通し番号で作り直さず、対応表全行を要求ledgerへ引く。

## R10 Release Registry 18件と設計変更マイグレーション

R10-01 既存18IDを厳密維持。基準状態unresolved16/resolved2、release_ready=false。既存resolvedは rev2_desktop_launch_regression と windows_rust_integration_test_execution_policy の2件；新候補の自動PASSではない。
R10-02 新OS保証基準へ移行対象=rev2_mobile_device_evidence（旧物理Device証拠→標準Android Emulator/iOS Simulator実物証拠）、linux_native_product_support（Linux native VM/hosted実session/installed実仕事で判定）、rev2_mobile_flutter_native_device_link_boundary（OS native API/Permission/秘密/IPCの検査は残す）。新資格と証拠を要求し、legacy欄を残す。
R10-03 Windows固有必須=windows_evidence_provenance_isolation/windows_installer_first_run_smoke/windows_setup_doctor_smoke/windows_broker_installed_smoke/windows_distribution_identity。新Owner所有実機の確認を取り込むが、実機が唯一の品質根拠になることはない。
R10-04 安全必須=audit_anchor_external_tamper_evidence_proof/rev2_flutter_broker_channel_boundary/rev2_export_owner_ui_authority_path。能力完成=comprehensive_extension_rev1_completion/rev2_module_pruning_binary_and_measurement。配布=rev2_desktop_product_distribution/rev2_mobile_distribution。旧resolvedのrev2_desktop_launch_regression/windows_rust_integration_test_execution_policyは履歴保持。owner_go=責任者GO。
R10-05 全文と実Validatorの照合対象=release_blockers.registry.json、AGENTS.md、規定/正本索引.json、README.md、QUICKSTART.md、CLAIM/COMPATIBILITY_MATRIX/MOBILE_STATUS/INSTALLER_STATUS/RELEASE_CHECKLIST、docs/FINAL_QA_QUEUE.md、OS用Tool、CI/Actions。旧「物理必須」を単に削るのでなく、新仮想OS実実行と安全拒否・配布資格/署名を同じ意味で代入する。
R10-06 Registryをv4へ実採用するまではrelease_ready=false。署名鍵/Developer Program/Store審査/Owner判断が無ければ「技術適合あり、対外配布不可」と区別。ブロッカーを改名/抹消/偽resolvedにせず、採用前旧規則と新設計案の衝突を明示。

## R11 施工進行報告と不完了の表現

基準時点=2026-11-12観測main d503a4ef6daac8942abd1cb011442ce28c78ec55。A..J=OPEN、N01..11統合受入OPEN、W00=SCOPED_PASS、W01=限定CLOSED、W02=OPEN、W03..W08未完了、B5 OPEN、B6未完成、正式Q0..Q7 DEFERRED、Registry unresolved16/18、release_ready=false。後続新commitにこの状態を固定転記しない。
進捗単位={日時JST,最新HEAD,要求母集団版,線A..Jの実Consumer配線,BUILD_GAP個数,標準OS/Arch/配布物/仮想媒体,実装・検査・QAそれぞれの状態,解消/未解消Blocker,安全停止,再現根因/副作用,Git復旧点,次の横断施工}。
報告に許される確定語=CODE_PRESENT／WIRED／LIVE_ACTION_OBSERVED／BUILD_COVERED／QC_SCOPED_PASS／QA_SCOPED／RELEASE_GO。NOT_RUN/UNKNOWN/BLOCKED_AUTHORITY/TECHNICAL_FAILURE/QA_NOT_ESTABLISHEDを混同しない。
止める場合は該当対象と根拠/権限/再開条件を列挙。他線は可能な限り継続する。既存指示の再承認を無用に求めず、別途必須の外部Interactive Gateおよび正式署名・公開・非可逆な新作用だけ責任境界に従う。
最終合否は製造完了（E3）、製品品質（E4/E5）、出荷（E6）を独立報告する。全五OSでのQAが未成立でもWindowsの限定技術状態を嘘なく報告できるが、未保証OSを含む「全OS完成/配布」宣言は不可。
