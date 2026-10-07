# macOS Desktop通常Broker接続

## 目的と有限単位

P13で未接続だったmacOS画面の`gui_shell/broker`を同梱Rust helperへ結合し、実Brokerの初回設定・health等の通常要求と正常終了を成立させる。これはmacOS移植の接続単位であり、Windows製品機能の全移植・Owner確認・正式配布の完成ではない。有限Acceptanceは手動Actions run 37598578513でCLOSED。正確な証拠範囲は`docs/REV5_PRODUCT_PROGRESS.md`を参照する。

## 責任

Flutter → 固定MethodChannel → native Runner → 起動時に作る匿名pipe → 同梱Rust helper → 既存認証loopback Broker → 既存handler。

Runnerは固定bundle内のhelper一個を引数なしで起動し、要求JSONと応答JSONだけを運ぶ。資格、endpoint、Audit、Permission、Approvalを読み取らず決定しない。環境はOSが与えたsandbox HOMEと一時保存先だけ。Rustは固定HOME配下の専用0700保存先、create-only起動lock、normal資格を所有する。入力session_idは既存Windows中継と同じ拒否用fieldを付加し、既存Brokerのvalidation／replay／Auditへ通す。通常接続のCLOSED単位ではOwner資格とOwner操作receiverを作らなかった。現在の追加receiverは本書後段のOwner追加契約だけに限定し、Owner資格fileは引き続き作らない。通常要求の可否はBrokerが判定し、Runnerがread-only能力やAuthorityを自己生成しない。

Rustは自身の実行pathから`.app/Contents/MacOS`、固定名app／helper、Info.plistの通常file・bundle内配置を確認してから、既存Brokerの初回UI設定初期化を有効にする。これは配置検査だけで、正式署名・install identity・Owner承認を証明しない。native試験の固定bootstrap操作観測はDebugだけへcompileし、要求本文・応答本文・秘密を記録しない。

要求は64KiB、応答は4MiB、一行JSON、画面内待機は最大4件。native応答期限超過・不正frame・helper停止時は通信路を閉じ、未確定要求を自動再送しない。画面終了はpipe EOFでBrokerの既存停止・Audit確定を待つ。期限内に終了しなければhelperを停止し、正常終了とは報告しない。Audit storeは保持し、同じ生成内容のsession fileと自分の起動lockだけを回収する。異常終了後の残留lockの自動除去はしない（最終Recovery保証は別単位）。

## App Sandboxとbuild

[Appleのhelper同梱規約](https://developer.apple.com/documentation/xcode/embedding-a-helper-tool-in-a-sandboxed-app)に従い、helperは`com.apple.security.app-sandbox`と`com.apple.security.inherit`のみを持つ。親appのsandboxは維持する。Brokerの認証loopback listenに必要なnetwork.serverをReleaseにも明示し、Rustは127.0.0.1以外でlistenしない。汎用network操作権やPermissionを付与する意味ではない。

開発時はRust helperを先にbuildし、Xcodeが固定場所からbundleへ同梱・署名する。通常利用者へRust／Flutter／Python／terminal操作を要求しない。未同梱buildは失敗させる。手動Actionsの`macos_product`だけで当該接続を検証し、CLOSEDのiOS／Androidを実行しない。

## Acceptanceと未成立範囲

対象Rust試験は実Brokerの通常応答、session注入・replay・Owner操作拒否、正常EOF終了、起動lock競合を確認する。native XCTestは製品窓のchannelから実helperへ要求し、応答とhelper正常終了を確認する。fixture／component証拠と製品画面投影を区別する。

Owner確認・Credentials・Task実行・Windows固有Install／Updateは本接続からsupportedへ昇格しない。macOS製品全体と正式署名・配布は`release_blocker`のまま。crash timing／長時間／網羅的alias等はFinal QAへ送り、基本接続Acceptanceを拡張しない。

## Owner追加契約 — Agent CLI／Workspace起動中登録

状態: IMPLEMENTING。既存`AgentCLI実行系作業領域登録`とSchemaを再利用し、Runtime／Workspace ID、CLI絶対path、Workspace root、秘密path除外、Provider／Modelを要求hashとともにRust所有OS確認へ表示する。入力構造の検査はBrokerと同じ実装を共有し、未知field、未知Adapter、相対root、不正秘密path、Broker資格情報保管庫の未対応参照は確認候補へ昇格しない。既存のWindows入口・Task能力判定は変更しない。

承認後だけ既存BrokerがAPFS rootのnofollow・物理identity・Broker内部領域との非重複を検査し、既存Adapterの`--version`と`exec --help`を起動する。OS App Sandboxは継承したまま、アクセス不能・CLI不正・重複登録・Audit失敗は既存拒否へ通す。CLIの固定引数検査であり、外部実行fileの無害性を保証しない。登録は最大8件のprocess内状態、Task／Provider通信は開始せず、Credential・Trust・Permission・Approvalを生成しない。Owner確認最大300秒と既存2回の5秒probeを区別し、Broker結果は15秒、transportは320秒まで待つ。期限超過時に自動再送しない。

有限Acceptanceは、実Brokerで非承認時の未登録、承認後の登録とTask非対応、重複・未知Adapter・余分なAuthority field・秘密path逸脱・保護root・CLI不在の拒否、Auditへのpath／秘密非露出を確認すること、およびMac製品Agent Centerから実OS拒否・承認・登録結果表示を通すこと。`macos_agent_registration`手動Actionsだけで実行し、CLOSEDのAdapter／Mobile試験を選択しない。

製品試験はsandbox内の専用合成Workspaceと、公式npm配布の固定Codex CLI 0.159.2 darwin-arm64を使用する。package SHA-512を固定・照合し、実binary hash／`--version`／`exec --help`を記録する。配布方法の根拠は[公式Codex CLI案内](https://learn.chatgpt.com/docs/codex/cli)と[版固定npm導入例](https://developers.openai.com/cookbook/examples/codex/build_iterative_repair_loops_with_codex)。試験は認証・模型要求・Agent Taskを行わず、CLIとWorkspaceを回収する。UIは既存Debug表示補助を使用する。通常利用者へ開発tool導入を要求する設計ではない。

この単位はsandbox外のWorkspace選択／security-scoped access、CLI配布／取得UI、macOS Credential保管、対話／Task実行、正式署名・製品全体の完成を主張しない。これらはP13後続機能と既存`release_blocker`へ保持する。sandboxを無効化して成功へ変えない。

## Owner追加契約 — 既存Adapterの状態管理

状態: CLOSED。commit `7c4219f5202402ad574e81b81e32d6eb02c82cd9`、手動Actions run `37629357493`で下記有限Acceptanceが成立した。P13の接続単位として、既存の検証・有効化・無効化・隔離・削除ボタンをRust所有OS確認から既存Brokerへ接続した。導入・更新のCLOSEDを再開しない。要求Schemaは既存`adapter_management_request.schema.json`のままで、公開Contractを増やさない。

Rustは操作名、Adapter ID、現在Adapter hash、要求hashを確認画面へ結合する。未知field、Manifest混入、操作不一致、不正ID／hashは確認候補にならない。承認後もBrokerが現行record、hash、署名と状態条件を再評価する。承認による署名Trust、Permission、Task Approvalの生成はない。作用はcatalogとAuditだけで、外部artifact／processには作用しない。

Flutterは画面で最後に取得・選択したrecordのIDとhashを要求へ渡す。起動時snapshotだけに固定すると、起動後に導入・更新したrecordを操作できない。選択metadataは要求対象を固定するだけで、現在性・Authority・署名を証明しない。Brokerの再検査で古いhashは拒否される。

SwiftとFlutterの待機期限は当該5操作も305秒とし、既存の上限300秒のOS確認を待てるようにする。期限を延ばしてもAuthorityは生成せず、取消・失敗・期限超過は非承認のまま。通常要求は従来どおり5秒である。

有限Acceptance: 実Broker経路で拒否時不変、署名不正・未検証有効化拒否、無効化・隔離・削除とAudit、古いrecord hash・不正payload拒否を確認する。macOS製品画面から5操作を実OS確認へ接続し、結果を表示する。署名検証・有効化の正の状態遷移は既存Broker署名試験の直接依存確認を使い、macOS製品でのTrust導入成功とは扱わない。通常macOS helperにはTrust設定の製品入口が未接続のため、Trust未設定／不正署名の拒否を保持する。この追加機能・正式署名配布は`release_blocker`として別単位へ残し、現在のOwner接続試験を拡大しない。

macOS実動作の検証環境は手動Actionsの`macos_adapter_lifecycle`。旧導入試験は選択せず、新しい状態操作の準備としてだけ合成Manifestを登録した。UI試験は下記と同じ明示Debug表示補助を使い、通常buildと区別する。ローカルRust／Flutter試験とMac画面の証拠範囲は現行進捗に保持する。

## Owner追加契約 — Adapter導入・更新

状態: 有限Acceptanceはcommit `0729f07437f308c71c1f4dc694e62016ea58ad3e`、手動Actions run `37623944221`でCLOSED。製品UIの拒否・承認・導入・通常終了、実Brokerの導入／更新・永続化・否定条件、実OS期限切れ非承認を確認した。証拠classと試験表示flagの範囲は`docs/REV5_PRODUCT_PROGRESS.md`へ記録する。他Owner操作や通常Release全機能の完了を意味しない。

UI試験の合成Manifestは、日本語意味を保持したJSON Unicode escapeのASCII表記で`typeText`へ渡す。Macの文字入力経路で`既`／`署`が互換漢字へ変化し、見た目とSwiftの辞書比較では同等でもRust契約のfield名と一致しない失敗を観測したためである。変換は試験入力だけであり、製品側で入力key・署名対象・hashを無言に正本化するものではない。通常のJSON parserと厳密なBroker検査を通す。型／本文の一時診断は原因特定後に撤去した。

通常接続のCLOSEDを維持し、次の製品差分としてAdapterの導入・更新を追加する。受信要求は権限ではない。Rust helperが現行envelopeのsession注入禁止、時刻、metadata、hash、既存Adapter Manifest検査を行い、表示用に検査されたsummaryだけをOS確認画面へ渡す。画面は既定「承認しない」、明示「今回の操作を承認」の二択、上限300秒。取消・期限超過・未知応答・OS失敗は非承認である。

承認後は確認した同一要求だけを既存Brokerのprocess内Owner receiverへ渡し、既存のfreshness／replay／hash／Authority／監査／永続化を再評価する。資格fileを生成・公開せず、Swift／Flutterの入力や過去の確認から承認を作らない。非承認・対象外・入力不正は通常Brokerの拒否・監査へ通す。要求の自動再送をしない。Adapter metadata登録は外部code起動、署名trust、Permission、Task Approvalではない。WorkspaceやCredentials等へ本allowlistを転用しない。

Swiftは当該二操作のtransport待機だけを305秒とし、判断は所有しない。Rust coreはunsafe禁止を維持する。OS FFIは独立`native/macos_owner`の同期`CFUserNotificationDisplayAlert`一呼出しへ限定する。既存lock内のcore-foundation 0.10.1／core-foundation-sys 0.8.7を明示固定し、新runtimeやscriptを追加しない。

FFI例外の局所レビュー: CFStringは呼出し終了まで保持し、optional URLはNULL、出力flagは非承認で初期化する。整数return code成功と明示alternate responseと単調時刻期限を全て必要とする。raw pointerを外へ返さず、コールバックや共有可変状態を作らない。外部参照は[Apple API](https://developer.apple.com/documentation/corefoundation/cfusernotificationdisplayalert(_:_:_:_:_:_:_:_:_:_:_:))と[固定binding](https://docs.rs/core-foundation-sys/0.8.7/core_foundation_sys/user_notification/index.html)。

有限Acceptanceは、実Brokerでの導入・更新と永続catalog、拒否時不変、replay／session注入／不正hash拒否、macOSのOS確認画面・期限の動作を対象とする。fixtureとnative操作の証拠を分離する。通常接続を再証明する追加試験や全Adapter機能の最終QAは行わない。

UI試験はdevelopment専用XCTest。初回XCTestではFlutter内部の要素がなく、外部AX接続要求もOSに拒否されたため、Debugかつ`D4_MACOS_OWNER_UI_TEST`の明示試験buildだけで表示用semanticsを有効にする。[固定FlutterEngine実装](https://github.com/flutter/flutter/blob/3.44.0/engine/src/flutter/shell/platform/darwin/macos/framework/Source/FlutterEngine.mm)が受けるアクセシビリティ通知だけを使い、Broker要求・Owner選択・資格・永続状態へ注入しない。通常buildには通知をcompileしない。OS保護設定の変更や秘密入力fallbackも行わない。標準XCTestだけでsemanticsが接続可能になった時点でこの表示補助を除去できる。通常製品buildは別途確認し、UI操作証拠が試験表示flag付きであることを明記する。
