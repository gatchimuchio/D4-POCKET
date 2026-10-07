# macOS Desktop通常Broker接続

## 目的と有限単位

P13で未接続だったmacOS画面の`gui_shell/broker`を同梱Rust helperへ結合し、実Brokerの初回設定・health等の通常要求と正常終了を成立させる。これはmacOS移植の接続単位であり、Windows製品機能の全移植・Owner確認・正式配布の完成ではない。有限Acceptanceは手動Actions run 37598578513でCLOSED。正確な証拠範囲は`docs/REV5_PRODUCT_PROGRESS.md`を参照する。

## 責任

Flutter → 固定MethodChannel → native Runner → 起動時に作る匿名pipe → 同梱Rust helper → 既存認証loopback Broker → 既存handler。

Runnerは固定bundle内のhelper一個を引数なしで起動し、要求JSONと応答JSONだけを運ぶ。資格、endpoint、Audit、Permission、Approvalを読み取らず決定しない。環境はOSが与えたsandbox HOMEと一時保存先だけ。Rustは固定HOME配下の専用0700保存先、create-only起動lock、normal資格を所有する。入力session_idは既存Windows中継と同じ拒否用fieldを付加し、既存Brokerのvalidation／replay／Auditへ通す。Owner資格とOwner操作receiverは作らない。通常要求の可否はBrokerが判定し、Runnerがread-only能力やAuthorityを自己生成しない。

Rustは自身の実行pathから`.app/Contents/MacOS`、固定名app／helper、Info.plistの通常file・bundle内配置を確認してから、既存Brokerの初回UI設定初期化を有効にする。これは配置検査だけで、正式署名・install identity・Owner承認を証明しない。native試験の固定bootstrap操作観測はDebugだけへcompileし、要求本文・応答本文・秘密を記録しない。

要求は64KiB、応答は4MiB、一行JSON、画面内待機は最大4件。native応答期限超過・不正frame・helper停止時は通信路を閉じ、未確定要求を自動再送しない。画面終了はpipe EOFでBrokerの既存停止・Audit確定を待つ。期限内に終了しなければhelperを停止し、正常終了とは報告しない。Audit storeは保持し、同じ生成内容のsession fileと自分の起動lockだけを回収する。異常終了後の残留lockの自動除去はしない（最終Recovery保証は別単位）。

## App Sandboxとbuild

[Appleのhelper同梱規約](https://developer.apple.com/documentation/xcode/embedding-a-helper-tool-in-a-sandboxed-app)に従い、helperは`com.apple.security.app-sandbox`と`com.apple.security.inherit`のみを持つ。親appのsandboxは維持する。Brokerの認証loopback listenに必要なnetwork.serverをReleaseにも明示し、Rustは127.0.0.1以外でlistenしない。汎用network操作権やPermissionを付与する意味ではない。

開発時はRust helperを先にbuildし、Xcodeが固定場所からbundleへ同梱・署名する。通常利用者へRust／Flutter／Python／terminal操作を要求しない。未同梱buildは失敗させる。手動Actionsの`macos_product`だけで当該接続を検証し、CLOSEDのiOS／Androidを実行しない。

## Acceptanceと未成立範囲

対象Rust試験は実Brokerの通常応答、session注入・replay・Owner操作拒否、正常EOF終了、起動lock競合を確認する。native XCTestは製品窓のchannelから実helperへ要求し、応答とhelper正常終了を確認する。fixture／component証拠と製品画面投影を区別する。

Owner確認・Credentials・Task実行・Windows固有Install／Updateは本接続からsupportedへ昇格しない。macOS製品全体と正式署名・配布は`release_blocker`のまま。crash timing／長時間／網羅的alias等はFinal QAへ送り、基本接続Acceptanceを拡張しない。
