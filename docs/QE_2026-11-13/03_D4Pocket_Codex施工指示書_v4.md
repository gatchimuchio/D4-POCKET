# D4Pocket Codex統合施工指示書 v4.0

文書ID=D4-QE-CODEX-20261113-V4
施工対象=gatchimuchio/D4-POCKET／main
作成=2026-11-13 JST
基準観測=d503a4ef6daac8942abd1cb011442ce28c78ec55
文書状態=新施工指示案／現Repo未採用。実行時は最新User指示と採用手順を確認。作成しただけで施工許可・正本切替・Git push・Release GOを仮定しない。
正本候補優先=指示者の原文→01_D4Pocket_統合製品実装仕様書_v4.md→02_D4Pocket_統合開発工程表_v4.md→本書。既存AGENTS.md・旧3点・規定・Contractsの安全不変条件を維持し、旧OS物理試験条件は採用済み設計変更として整合的に移行する。
言語=日本語基底、機械Token/正式API/既存ID/OS用語は原義保持。

## I00 指示対象と責任境界

I00-01 あなた=施工者Codex。目的=汎用AI RuntimeであるD4Pocketを、全体一括実装→全実Consumer接続→全体QC→標準OS仮想QA→出荷責任者決裁まで施工・証拠化すること。個別修正/テスト件数/コード量/ドキュメントを成果へすり替えない。
I00-02 指示者=ご主人様。設計者=チャッピー。実仕事主体=製品内AI/LLM/Agent。製品使用者=エンドユーザー。実行Authority=Rust Broker。製品出荷責任者=ご主人様。人間使用者の製品Permission、Codexの開発操作許可、Ownerの正式出荷GOは別の許可で相互流用不可。
I00-03 D4Pocketは特定AI製品に付属するGUIでもチャット画面でもない。複数Runtime/Agent/Tool/Hostを接続・調停・監査・復旧するAIの実行身体。AIが実Toolを要求・作用・結果再消費し仕事を進める。UIは実結果/許可/履歴/停止/回復の使用面であり、AIの意思決定とBroker認可を乗っ取らない。
I00-04 インストール体験=使用者はWindows/macOS/Linux/Android/iOSの宣言標準構成で、通常の配布物取得→OS正規インストール→アプリ起動→Setup/資格案内→利用可能サービス→実仕事。Windowsは.exe等一般Installer。他OSは各OS正規形式。同じ.exeを全OSへ使う意味ではない。Cargo/Flutter/SDK/CLI/Source Build/手作業依存導入を一般使用者に要求しない。
I00-05 全OS実機試験要求を施工中に復活させるな。指示者のWindows PCだけ完成品を実機確認する。他OSは規格・対象標準スペックを定めVM/Host/Android Emulator/iOS Simulator等で実インストール済み製品を検証する。VM不可・OS固有条件非再現はQA_NOT_ESTABLISHEDとし、未検査をSUPPORTEDと偽らない。物理機器なしをソース実装停止理由にしない。
I00-06 ここでの受領=本三点を正式施工投入された場合。ユーザーが文書を作れと要求した段階では、現Gitリポジトリへの無断採用・commit・pushは別行為。施工担当は指示書を実際に受領した時点で作業許可と既存権限を確認し、既存規定に従う。

## I01 開始前の必読・観測［一回の全体系同期、以後差分］

I01-01 最新指示者原文、現HEAD、git remote/current branch/status/dirty/stash/worktree/他writer、主にmainとorigin/mainの同一性を確認。HEADが基準観測SHAから進んでいれば現在の実装・証拠・残件を再取得。旧観測値を現在値へ無確認転記するな。
I01-02 AGENTS.md全条、規定/00_日本語基底規定.md、規定/正本索引.json、現行採用記録/正本順序、本v4仕様・工程・指示を読む。v4が未採用なら既存2026-11-09版が現正本である旨を明示し、ユーザーから新3点の施工投入を受けた場合はI12の変更管理で正本切替。同一時点に複数矛盾正本を黙って併存させるな。
I01-03 旧要求=docs/総合機能拡張_rev1/実装仕様書.md・工程表.md・実装指示書.md・進捗.md、docs/D4_POCKET_PHASE_MAPPING.md、docs/PRODUCT_COMPLETION_PLAN.md、docs/REV2_PROGRESS.md、docs/REV3_PROGRESS.md、docs/REV4_ACCEPTANCE_LEDGER.md、docs/REV5_PRODUCT_PROGRESS.md を逐条照合。C0..C34/Phase0..45、P0..13、K01..20の実consumerを抽出。
I01-04 旧QE=docs/QE_2026-10-22/採用三点・採用記録・F01..07・D4-QE-R01..13・D4-LNX-T01..08、docs/QE_2026-11-05/、docs/QE_2026-11-08/、docs/QE_2026-11-09/採用三点/採用記録/進捗/W00..W08証拠を照合。F07_rev5原本要求照合_20261030.mdの当時SHA/差分0記録は歴史として保存。rev5原本生データが取得不能なら取得済みと偽らない。
I01-05 Buffer=docs/緩衝基盤_rev5補遺_QC/の全三点とB0..B5証拠、BUF-R01..R09/BUF-T01..T24/B0..B6を実Consumerへ対応。
I01-06 現行契約・コード=docs/specs/、specs/Schema全件、packages/shell_core/、native/rust_helper/、apps/desktop_flutter/、apps/mobile_flutter/、installer/、tooling/、.github/workflows/、docs/FINAL_QA_QUEUE.md、release_blockers.registry.json、README/QUICKSTART/CLAIM/COMPATIBILITY_MATRIX/MOBILE_STATUS/INSTALLER_STATUS/RELEASE_CHECKLIST/SECURITYの実在する最新版を読む。存在しないパスは推定せずUNKNOWNとして代替実証点を探す。
I01-07 一行ごとに読み直すのでなく、要求の意味・旧証拠・新差分を ledger 化する。ledgerキー={ID,原文/出典,目的,主語,Producer,Consumer,契約,実装位置,既存状態,不足,OS母集団,安全負例,実測,証拠class,Blocker}。旧W00を毎Batchで一から作り直さない。
I01-08 読んだあと、現在の原理=AI実仕事/汎用Shell/唯一Broker権限/全体施工/通常installed UX/標準OS仮想QA、目的/対象/安全/停止/責任の一致を確認。未確認を推論で補完しない。旧資料とv4変更の衝突を記録・制御して、旧物理条件の存在をもって新設計を無断否認しない。

## I02 全要求の初期母集団［何かを削ってMVP化しない］

I02-01 新要求=N01,N02,N03,N04,N05,N06,N07,N08,N09,N10,N11。各Nの実Producer→Consumerはv4仕様S04を逐条使用。
I02-02 旧要求=P0..P13=14、C0..C34=35、Phase0..45=46、K01..K20=20。C/Phaseの全行はdocs/D4_POCKET_PHASE_MAPPING.mdから厳密に展開、旧Pを旧局所実装/現在不足と照合。要求IDを無断再採番しない。
I02-03 Buffer=BUF-R01..R09=9、BUF-T01..T24=24、B0..B6=7。B0..B4限定CLOSED、B5 OPEN、B6未成立を現HEADで再評価。B5をE1初期から開発環境自身へ適用し、他線の検証と一体化。
I02-04 検証・監査=D4-QE-R01..R13、D4-LNX-T01..T08、D4-AUD-F01..F07、FQ-R2-A..H、FQ-R2-TEMP、FQ-MODULE-PRUNING、FQ-TEST-LOOPBACK、FQ-P11-UNINSTALL-FINALIZER-CLEANUP、FQ-INTEGRATED、W00..W08、Q0..Q7。
I02-05 Release Registry=18件/現観測unresolved16/resolved2/release_ready=false。名称/旧status/旧証拠/旧バージョンを保持し、現実の新候補に対し一件ずつ条件/実証/残余を再評価。Owner GO/配布資格/署名の技術外条件を勝手にPASSしない。
I02-06 OS=Windows/macOS/Linux native/Android/iOS全件。共通基本機能・安全・監査・配布/Update/撤去を欠落させず、OS固有機能は標準環境の能力/Permission条件を明示。一般ユーザーが直接使えないBuild artifactを配布済み製品と呼ばない。
I02-07 除外=BLUE-TANUKI/MINIDORA等の非公開固有Brain本体、外部Provider固有の資格を無権限取得する行為、全機種物理実機試験、未宣言OS/改造構成、未委任の公開/本番署名。これらを理由に既存D4必須Adapter/Tool/Runtime接続を省略しない。

## I03 最重要施工順：全体一括BUILD→全接続→QC→QA

I03-01 最初の一巡でA〜JをすべてBUILD母集団に投入せよ。「Aを作って検査して合格→B」ではなく、共有契約と全製品躯体を設け、全プロデューサと全コンシューマを同じシステムに配線せよ。
I03-02 W02の局所テスト/GUI再起動/fixture作り直しを、B〜J着工の先行条件にするな。W01限定CLOSEDを再利用しつつ、W02の真の結果依存AI次Task/installed UI hangをA/C/F/Gの共有因果で修正する。他線B/D/E/H/IのSource/Contract/配布経路を同時に進めよ。
I03-03 A〜Jの作業配分は所有するファイル/Contractと単一writer競合だけ局所直列可。全体工程は並行進行。すべての機能の存在と実接続が形になった後、各部の局所不適合を系統検査する。全体が未完成なのに製品全体の最終QAや出荷判定を周回するな。
I03-04 E1で各OSの正規Installer/Package、SetupDoctor/Update/Repair/Rollback/Uninstall、Flutter→Native→Broker→実Runtime→AI再消費の共通経路を確実に含める。OSごとにBuild方法だけ書いて出力物を後回しにしない。
I03-05 E2で全入力→実Broker→実Tool/Agent/Host/OS→実結果→後続AI/使用者UI→Audit/Recoveryの接続を確定せよ。MCP宣言だけ・A2A Cardだけ・Compose Manifestだけ・Export雛形だけ・AI Edit Proposalだけなら未施工。
I03-06 SYSTEM_SHAPED=代表一仕事が通常製品で通電する。これは全製品完成ではなく途中の接続観測。他線の施工を止めるGateではない。必須Consumerが一つでも未接続ならBUILD_GAPを保持。
I03-07 E3 BUILD_COVERED=全N/P/C/Phase/K/BUF/B/FQ/OS/18 Blockerのうち実装・実接続が必要な全件にSourceと実Consumerがあり、未実装必須数0。BUILD_COVEREDとQC_PASSを混同するな。
I03-08 E4入口でQ0候補を固定し同一統合候補の局所/接続/全体QC、E5でQ1..Q6/標準OS環境QA、E6でQ7正式Release Review。機能追加発見時はE1/E2へ差分帰還し、関連証拠のみ再取得。やり直しの際に無影響部分を無条件全破棄するな。

## I04 A〜Jの実装命令［各線を分割製品として閉じない］

I04-A AI実行線：AI/AgentのTask/Session/Workspace/Capability選択/CLI/MCP実Tool要求と、認証Broker Permission/Approval→実作用→結果の未信頼処理→同AIの結果依存後続Task→Audit/Stop/RecoveryをRuntime-neutralに結線。FlutterボタンやテストFixtureを唯一のControllerにしない。後続Taskには新しいidentity/認可を検査。
I04-B Runtime連携線：Codex CLIと別の独立した実Runtime最低2種を統一Adapterで接続。外部A2A Agent Card/Task/Message/Stream/Artifact、Host/Agent/Session間のHandoff/Compare/実Result/失敗/拒否を結合。別Hostの権限自動継承禁止。
I04-C 操作/観測線：Agent Center/Workspace Inspector/History/Diff/Artifacts/Eval/Resource/Trace/Metric/Problems/Notification/Tray/Search/Palette/Recovery/StopをBrokerの実状態へ接続。Flutter main isolate/UI Timerのハング/短命性を根因から修正。描画できる≠操作実体がある。
I04-D 能力線：実Capability Catalog→依存/互換/権限検査→Compose→元Toolの実仕事。AI Edit=proposal/diff→独立検査/別施工→製品使用者の許可→Broker制御apply/rollback。AI自身のapproval生成禁止。
I04-E Export線：構成を実消費する独立App、別identity/Broker/Runtime/必要Adapter/Module/Package、選択module以外をbinary/resourceから本当に削除する設計/計測点を持たせる。安全基幹は省略しない。実測size/起動/最小資源はE4へ送る。
I04-F 安全/調停線：Rust Broker唯一権限でCredential(native secret)、認証IPC、Content Exposure、Permission/Approval期限/単回、Path realpath/hardlink/TOCTOU、process tree、scratch、Audit append-only/externally anchored integrity、Recoveryの正常・拒否・異常。B0..B4再利用、B5自己適用/B6結合、Buffer変換の効果と副作用を元目的と突合。
I04-G Windows線：一般利用者用Windows installer(.exe等)、Launcher/Setup Doctor/Provider GUI設定/必要依存同梱または管理/実Broker/Tool/Agent/Update/Rollback/Repair/Uninstall。clean user profileでOS通常導入した候補の証拠を取得。Portable/dev toolやWindows自体の配布identity未確立を正常Installer完了へ偽装しない。
I04-H 非Windows Desktop線：Linux native（GUI Session、Wayland/X11宣言、Secret storage、IPC/process、対象distro/arch Package）とmacOS（Swift/Keychain/sandbox/native permission/IPC/Bundle/Update）。Windowsの成功やWSL/Xvfbを他OS native完成へ転用しない。ソース/配布Buildを今つくり、QAでは標準OS VM/hostedで実導入。
I04-I Mobile線：Android Keystore/Flutter/Device Link/OS permission/lifecycle、iOS Keychain/native bridge/sandbox/Device Link、認証Broker境界/Agent Task/停止/復旧/OS正規Distribution。Android Emulator・iOS Simulatorでinstalled候補を試す設計を最初から含める。物理Android/iPhone検証を要求しない。
I04-J 監査/出荷線：要求ledger/Schema/Conformance/Manifest/Source/Artifact/OS/実Tool/AI再消費/権限/Audit/Recovery/Negative/候補identity/18 Blocker/Strict Gate/更新前後/出荷GOを繋ぐ。複数世代の証拠を混線させない。

I04-01 共有契約を編集するときはShell Core/Rust/Dart/Native/Schema/Test/Installer/OS Branch/Exportの全影響Consumerを同じBUILD_BATCHへ含めよ。部分patchで局所成功させて別画面・別OSに同型故障を移すな。
I04-02 実装済みコードは条件/改訂差分を照合し、再利用できるものを保存。旧有限CLOSEDを全部作り直して進捗を失うな。製品能力を「性能劣化しない程度に」など曖昧に削るな。設計上の完全性と安全性を守れ。
I04-03 追加ライブラリ/言語を選ぶ場合は依存・ABI/API/OS/保証/保全の責任を記録。1言語だけで全OSを無理に完結させない。将来のAICompiler移行を理由に現時点の実効性を損ねない。

## I05 BUILD_BATCH中の検査・危険停止

I05-01 原則=製造中の各小行為では目的/主体/実consumer/変更差分だけ再同期。関係する複数局所編集を一つのBUILD_BATCHへ束ね、バッチで最小限のcompile/契約照合/必要な正負/差分監査/Gitを閉じる。毎局所一巡の全Suite、Q0..Q7、OS全QA、backup tag回転を禁止。
I05-02 安全差分ではその場で検査=default-deny/認証IPC/Permission拒否・期限・single-use/Approval本人経路/Credential非露出/Content Exposure全5段階/Audit不能時停止/外部改竄痕跡/Task間/Host間/Workspace間隔離/Path symlink hardlink TOCTOU/Tool結果偽指示/非冪等Replay/Cancel Deadline crash後process tree/Tempと所有Scratch境界。危険時は該当実作用をfail-closedへ隔離し、独立線を進めよ。
I05-03 安全性や規格を理由に通常インストール体験をCLI依存へ降格しない。無署名などで導入が阻害される場合は正規署名・資格/受入範囲/配布状態を分離し、セキュリティ回避策を利用者へ指示しない。
I05-04 BuilderのTEST環境のネットワーク/Toolchain/OS不具合はその原因を分類。失敗=TECHNICAL_FAILURE/ENVIRONMENT_UNAVAILABLE/AUTHORITY_PENDING/UNKNOWN。再試験の成功のみを原因修正とせず、改修前後の同一条件で再観測し副作用を比較。OneDrive/cache/TLS/loopback/Windows SACのB5は実施工へ組込。
I05-05 検査例は現repoの実ファイル/CLI引数を読むこと。Python Schema: python tooling/schema_check/check_schemas.py；Conformance: python tooling/conformance_tests/run_conformance_skeleton.py；Manifest: python tooling/manifest.py --check；Python統合: python tooling/validate_all.py --python-only；Rust隔離: python -X utf8 tooling/run_cargo_isolated.py test --locked --offline --manifest-path native/rust_helper/Cargo.toml；Flutter例: python tooling/validate_all.py --flutter-step desktop_flutter_test --desktop-platform windows；analyze: 同 desktop_flutter_analyze；Strict Release: python tooling/release_gate_check.py --strict-release；差分: git diff --check。
I05-06 上の例は現リポジトリの引数/環境確認が前提で、コマンドの偽造や、Python3/Windows Powershellへの未検証転記は禁止。検査Resultは実exec/exit/SourceSHA/実Hostを記録。Strict Gate失敗は現未出荷の可能な正常検出、テストを弱める改修理由ではない。
I05-07 E3後は全Rust/Flutter/Schema/Conformance/OS installed/外部A2A/Export/安全/長期運転/資源の**正式QC**を同一候補で集中実行。Qの合格境界へfixture testだけを流用するな。

## I06 OS規格/製品配布/仮想実物QAの施工指示

I06-01 OS保証対象をE0で宣言。Windows/macOS/Linux/Android/iOSそれぞれに標準version範囲/arch/ABI/API/権限/GUI・secret・Resource min/配布形式/Simulator・VM・Hosted媒体/OS固有非対応を記録。Q0で現候補用の確定行へ固定。版数・メモリ量・性能値を観測前に創作しない。
I06-02 OSごとに適切なInstaller/Packageをビルド・同一性記録・インストールするための手順を実装。Windows.exe等、macOS一般Bundle/package、Linux適用Package、Android正規App形式、iOS正規配布App。使用者にSDK/Flutter/Cargo/Console手作業を要求しない。
I06-03 OS検査=宣言標準OSのVM/hosted native desktop（Linux X11/Waylandの対象別）、適法macOS GUI VM/hosted、Android Emulator、iOS Simulatorで**対象環境へ実導入できる製品相当ビルドを実行して**初回起動→基本サービス→実仕事→実Broker/認可/秘密/Audit/Recovery→Update/Repair/Rollback/Uninstallを試験。同一SourceSHAと各Target固有ArtifactSHAを紐付ける。配布用device版とSimulator用のbinary差は別保存し、実行したのはどちらか明記。
I06-04 Windows=Windows VMで環境定義・自動再現、指示者所有のWindows PCで完成品の物理確認を最終実施。Windows実機を開発不具合の都度手合わせ調整に使わない。物理確認で異常なら仕様/実装/VM再現条件へ戻る。
I06-05 非Windows物理実機は**不要**。Mac・Android・iPhoneの実端末がないため全体BUILDやQAを凍結する旧規則は採用しない。ただし仮想環境が不在、必要なnative API/実際の標準条件を再現不能、UI/Installerを実動かせない場合は該当OSがQA_NOT_ESTABLISHED。Windows buildやcross-compile/fixtureで穴埋めしない。
I06-06 ストア資格・署名・notarization・publishing・本人Developer資格・外部規約・Owner GOを実機試験とは別に管理。技術的配布物準備/仮想OS適合と公開済み/ストア承認を別に表示。正規インストールが成立しない候補を「即使用可能」と表示しない。
I06-07 rev2_mobile_device_evidenceを機械IDごと残し、保証命題の主語を「標準Android/iOSにおけるinstalled App/Native Permission/Task」と再定義して試験参照・Registry/Validator/Docsの全Consumersを改修。rev2_mobile_flutter_native_device_link_boundaryは安全機能のまま保持。linux_native_product_supportをLinux native VM/hostedで判断し、WSLのみを合格根拠にしない。
I06-08 Android Emulator/iOS Simulatorが近似しない個別Hardware/OS能力について、保証母集団の対応条件と未実証を明記。無根拠に「Simulator試験=全物理実機・全メーカー保証」と拡大しない。全機種物理試験要求をその代償として復活させない。
I06-09 同一体験の意味=各OSの使用者が普通にインストールし基本サービスを利用可能、必要な外部Provider資格は製品のGUIで案内/登録、製品固有の初期設定負担を排除。外部契約を製品が無条件供給するという虚偽保証は禁止。
I06-10 iOS Simulatorビルド≠署名済iOS配布IPA。Simulator実行とdevice配布候補は同一SourceSHA/Contract/Build構成の追跡で結び、異なるABI/ArtifactSHA/Entitlementを別検証。Appleの端末でのRelease検証推奨を履行したと偽らず、本計画の物理iPhone検証は要求しない。公開/署名/Store審査は独立の未決裁/未成立状態として扱い、使用者が端末で動かす現実の配布条件を明示する。

## I07 既存W/FQ/Q/Blocker受入と停止

I07-01 W00=旧SCOPED_PASS、W01=Windows/Codex CLI限定CLOSED、W02=OPEN、W03..W08未完了という過去の時点を記録。W01→W02→W03の順で施工開始を待たず、W00..W08は一巡完成後の受入座標とする。W02-B02 WIRED/B03 Fixture PASS/B04 installed実仕事NOT_RUNとUI非応答原因UNKNOWNを混同しない。
I07-02 C0..C34/Phase0..45の全行はdocs/D4_POCKET_PHASE_MAPPING.mdを読み、Requirement LedgerにSource→Consumer→実物証拠を割付。旧CLOSEDは同版・同条件のみ保持。P2/P3等既済能力を別OS・別Runtimeへ自動拡張しない。
I07-03 FQ-R2-A..H/TEMP、MODULE-PRUNING、TEST-LOOPBACK、P11-UNINSTALL-FINALIZER-CLEANUP、INTEGRATEDを全数試験へ入れる。BUF-R/Tの旧定義・Q0..Q7・D4-QE-R/旧Linux-T/旧Audit-Fの根因を削除しない。
I07-04 Q0候補固定→Q1横断実仕事→Q2故障/復旧→Q3権限・秘密→Q4耐久・資源/Pruning→Q5対象標準OSのVM/Emulator/Simulator installedおよびWindows所有実機確認→Q6証拠/Blocker集約→Q7責任者判断。Qの番号は保証の判定順であり、E1中の製造順序ではない。
I07-05 現Registry18IDを保持：windows_evidence_provenance_isolation、windows_installer_first_run_smoke、windows_setup_doctor_smoke、windows_broker_installed_smoke、audit_anchor_external_tamper_evidence_proof、owner_go、linux_native_product_support、rev2_mobile_device_evidence、rev2_mobile_distribution、rev2_desktop_launch_regression、comprehensive_extension_rev1_completion、rev2_desktop_product_distribution、windows_distribution_identity、rev2_flutter_broker_channel_boundary、rev2_export_owner_ui_authority_path、rev2_module_pruning_binary_and_measurement、windows_rust_integration_test_execution_policy、rev2_mobile_flutter_native_device_link_boundary。
I07-06 上記のうちrev2_desktop_launch_regression/windows_rust_integration_test_execution_policyは旧resolved2件として歴史保持。全18を「現候補は実OS物理試験不要」だけでresolvedへするな。安全/OS標準実証/配布資格/Owner GOを個別再監査。release_ready=falseをGO前に変更するな。
I07-07 Q証拠は要求ID→SourceSHA→Contract→ArtifactSHA→OS/version/Arch/VM/Simulator/Host→インストール手順→実Tool/AI後続→Audit/拒否→復旧→実command/exit→残余の連鎖。Test classはSOURCE/CONTRACT/FIXTURE/EMULATED_PRODUCT/VM_INSTALLED_PRODUCT/HOSTED_OS_PRODUCT/WINDOWS_PHYSICAL_CONFIRM/EXTERNAL_SIGNING/NOT_RUN。旧LIVE_PRODUCTと新VM_installedの対応は実事実で記録。

## I08 開発操作・承認の三段階［製品使用者の許可と別］

| 類別 | 主語/許可 | 施工者の実行 | 追加確認の扱い |
|---|---|---|---|
| A 通常委任施工 | 指示者が3点セットで委任したCodex | 設計に沿ったコード、Local/VM検査、Build、必要な通常のUI操作、既定Git直main、技術修復 | 内容を理解した再承認は不要。必要な内部再同期を指示者へ転嫁しない |
| B 外部必須Interactive Gate | 他サービス/OS/Computer Useの強制的な本人認証・権限クリック | 操作前に必要な画面・最小範囲・対象・制約・次に自動継続する操作を説明し、その操作のみ停止 | 開発の通常毎手順承認に拡大しない。保護/認証ルール回避禁止 |
| C 新たな責任者判断 | 本番署名/鍵投入、公開/Store登録、未委任の外部送信、破壊的な既存資産削除、要求範囲変更、Owner GO | 技術準備・影響調査は実施し、確定行為の直前に止める | 別途指示・資格/権限が得られるまでC行為は実施しない |

I08-01 D4製品内のPermission/Approvalは**利用者からRust Brokerへの明示認可**であり、CodexのA委任によって付与できない。Credential/秘密とAuthorityは絶対にTool結果/Flutter/AI/Adapterに委譲しない。
I08-02 既承認施工の再確認を工程の摩擦へしない。一方、外部サービスの実際の承認UIを模倣・迂回して確認を省略することはしない。Bが止めるのは該当クリック/資格だけで、独立施工を継続。
I08-03 GitHub正本への新3点の反映そのものは、作成時には実施済みでない。Codexへの施工投入を受けた段階でAとして定義された範囲で作業し、原既存AGENTS/安全/単一writer/保護ルールと矛盾した場合は採用管理へ差し戻す。

## I09 Git・OneDrive・履歴保証［採用時のリポ施工］

I09-01 Git正本=GitHub main。PRは使用しない、単一writer。初期にremote HEAD/local HEAD/dirty/他writer/復旧点を確認。作成物、ユーザー未commit差分、別開発者資産を上書き・破壊しない。
I09-02 意味あるBUILD_BATCHまたは統治改訂ブロックの編集前に2世代backup。local recovery refsを現AGENTSでrotateし、remoteのrefs/tags/codex/backup-mainとrefs/tags/codex/backup-main-prevへ保存・source commitを検証。branchではなくtag。小単位ごとに何度も回転せず、両世代を同一SHAに誤上書きしない。
I09-03 実装/局所安全・差分検査→対象commit→direct-main push→git ls-remote origin refs/heads/mainでHASH一致確認→remote backup refs検証→clean worktreeを記録。未push/非同一なら完了報告禁止。既存2世代tagの保全・rotationは権限/状態に合う現手順を守り、force-push mainや無断reset/無関係branch削除禁止。
I09-04 手動GitHub ActionsはOS利用不能時/重い検査用のworkflow_dispatchのみ。push-trigger CI/PR/必須status checkを開発の新しいGateにしない。必要な検証用一時branchはsingle-writer/valid SHA/fast-forward/安全回収を条件とし、検証用GitHub Actionsの成功を実OS installedや出荷保証へ昇格しない。
I09-05 OneDrive=使用者のbackup、作業場のsource snapshot、build/cache/tempの隔離。Flutter/Cargo/App signing secretなどを同期領域へ無管理で生成しない。データ削除は対象/影響/回復策/権限を確かめ、無断不可逆削除禁止。
I09-06 現行v3/旧補遺/F07/当時進捗を履歴として保存し、新v4の正本切替は採用日/受領SHA/設計変更表/旧資料優先順位/主語・目的・影響/Validator/Registry更新を別のRecordに記載。従来の数値・時点を上書きしない。

## I10 結果判定・品質・不正な完了報告禁止

| 状態 | 意味/許される判断 |
|---|---|
| CODE_PRESENT | ソース/型だけ存在。結合・実作用・品質は未証明 |
| WIRED | 本番ConsumerがSource/Contractを使用する。動作/OS保証は未証明 |
| SYSTEM_SHAPED | 代表実仕事の通電観測。全機能の完成を意味しない |
| BUILD_GAP | 必須実Consumer/Package/停止・回復接続なし。製造続行 |
| BUILD_COVERED | 全必須実配線あり。QCとQAの合格は別 |
| TECHNICAL_FAILURE | Source/環境/設計の観測不整合。根因追跡/安全対象停止 |
| BLOCKED_AUTHORITY | 本人資格/外部OS条件/権限不足。迂回せず対象だけ保留 |
| QA_NOT_ESTABLISHED | QA実証が欠落/失敗/非再現。保証/GO不可 |
| QA_SCOPED | Source/Artifact/OS/対象条件で実証合格。全OSへの無条件拡張禁止 |
| RESPONSIBLE_DECISION_REQUIRED | 署名/公開/Owner GO等、本人決裁のみ待つ |
| RELEASE_GO | 必須品質/配布/18 Blocker/本人決裁が一致。公開済みとはなお別 |

I10-01 目的が「サービスを使えること」であるのに、Registry修正/Documentation/テスト緑化を目的化しない。App installerの存在もインストール後AI実仕事が起きなければN10不成立。
I10-02 実OS物理検証を不要とすることは、他OS実動作を検証せず合格にする意味ではない。VM/Simulator/Hosted標準環境で実製品を動かし、未再現APIは保証対象外/未成立へ区別する。USB特異Device・全メーカー例外を全体障害へ拡大しない。
I10-03 仕様/Tool/Repo/実体/契約・母数の矛盾があれば、低レベルへ誤りを移転させず上流の生成条件へ帰還。旧判定は履歴保存、変更に必要な原因/影響・反例・同条件再検査を新記録に残す。
I10-04 製品の安全破綻/秘密漏出/ユーザーデータ不可逆消失/証拠改竄/本人非委任外部作用/Registry不正解放/単一writer衝突は該当行為を停止。作業環境不足はそれ以外の独立製造を止めない。

## I11 進捗・最終報告を機械的に出力

I11-01 各BUILD_BATCHの報告一文目=「製品AIが実際にどのTool/Runtimeで何を実行し、結果をどの後続AI/使用者に消費させたか。未成立ならどのProducer/Consumerが欠落か」。
I11-02 報告Record={date_jst,remote_main_sha,local_head_sha,source_baseline,requirements_total,old_ids_preserved,N01..N11_status,A..J_status,build_gap_count,system_shaped,build_covered,physical_test_scope,os_standard_matrix,os_virtual_evidence,windows_owner_physical_confirmation,qa_scope,q0..q7,release_registry_18,release_ready,owner_go,critical_negative_tests,provenance,unresolved_root_cause,git_commit_push,backup_two_tags,rollback_point,next_cross_line_work}。
I11-03 要求件数/実ConsumerのSource/実作用/Negative/回復/OS/Artifactを母集団とする。暫定%を出すときも分母/各重み/未確認を明記し、テスト件数で完成率を上げない。
I11-04 小単位の再同期は内部記録で充分。指示者への報告は意味あるBUILD_BATCH、実際の重大障害、C/B型の局所権限停止、製造完了、全体QC/QA結果に限定する。定期的な無意味な確認や新3点作成要求の再提出を避ける。
I11-05 連続施工の停止時は最終HEAD/未commit/未push/Backup/権限/危険経路の状態と安全に再開できる座標を出す。「全OS未実機なので全製品開発停止」は報告するな。本人GO不在のとき技術準備の結果は独立保存。

## I12 新三点の採用・旧条件からの移行命令

I12-01 現Repository AGENTS.mdと採用記録は2026-11-09版を指す。v4を正式受領した施工者は最初の統治改訂ブロックで、旧v3と新v4の要求差分・責任/権限/証拠の変化を全数表にし、新三点を現Repoの適正docs/QE_2026-11-13/へ保管する。受領・本人許可がない時点では採用済と装うな。
I12-02 変更の核=施工「逐次小Gate反復」から「全体一巡BUILD→配線→集中QC」、OS証拠「物理Mac/Android/iPhoneを必須」から「標準版/arch/OS正規配布/実installed仮想検査+Windows所有実機確認」。既存安全・全機能・出荷/署名条件の弱体化ではない。旧要件・旧FAIL・CLOSED・原本SHAを保存する。
I12-03 同じ統治改訂単位でAGENTS.md、規定/正本索引.json、docs/FINAL_QA_QUEUE.md、release_blockers.registry.json、README/QUICKSTART/CLAIM/COMPATIBILITY_MATRIX/MOBILE_STATUS/INSTALLER_STATUS/RELEASE_CHECKLIST、実Validator/OS Evidence Schema、必要なSource Test/Workflowの物理前提・用語を全数探し、一致する新OSテストとConsumer条件へ更新。存在しない文書を捏造しない。旧Registry statusを改竄せず変更履歴を追記。
I12-04 旧「全OS実機必須」と新方針の参照が一本でも残れば採用整合は未達。特にrev2_mobile_device_evidence/linux_native_product_support/rev2_mobile_flutter_native_device_link_boundaryについて機械ID/旧履歴保持→標準OS検証条件へ移行→Validatorで現Release証拠を再判定。安全系は元より厳格な境界を維持。
I12-05 変更管理=現BASE SHA→正本採用SHA→仕様・工程・指示各hash→改訂理由/原文→影響Consumer/テスト/Registry→旧履歴への非遡及→差分検査→direct-main保全/commit/push/remote確認。v4未施工段階で本書自身の採用/全製品完成をCLOSEDと記載しない。
I12-06 旧文書とAGENTSの合致を取り戻してからE1の全体BUILDへ進め。正本変更そのものを無限の事前審査・設計会議にして施工を止めるな。取り込み不能時は原因/該当権限/安全復旧点を示し、許された独立Build設計を続行する。

## I13 終了条件

CONSTRUCTION_COMPLETE=(E3 BUILD_COVERED=true AND 必須実Consumer未実装=0)。この時点で製品保証・正式配布を主張しない。
PRODUCT_QA_COMPLETE=(E4全体QCの対象合格 AND E5の宣言OS/版/arch/仮想installed保証証拠合格 AND Windows所有実機確認の結果記録 AND 安全/回復/資源の必要証拠合格)。対象OS/資格の欠測はQA_NOT_ESTABLISHEDとして残す。
RELEASE_AUTHORIZED=(18 Registry/Strict Gate/配布資格/署名/Publisher/監査/Owner正式GOの全条件成立)。本人GOなしにrelease_ready=trueへ変更しない。
すべての未達は未達として記録し、根因に従い必要な工程へ戻る。全体実装の目的を局所試験の達成へ置換せず、通常OS使用者の即利用可能性を最終対象として維持する。
