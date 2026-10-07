# macOS 作業領域のOS選択

## 意味と有限Acceptance

状態: IMPLEMENTING（P13 Product Build）。OS chooserで選んだfolderの公開pathを表示するcontrol操作であり、Workspace登録・Permission・Approval・Credential・Agent trustではない。取消時の入力保持、実OS選択pathの画面反映、別個のOwner確認後の既存Broker登録、UI由来のpath／bookmark／Authority注入拒否、正常終了時のscope回収を有限条件とする。CLIはsandbox内の既存実行系を使う。CLOSED登録機能は新しい選択機能の直接依存としてだけ使う。

## 観測した失敗と責任配置の修正

手動Actions run 37696395012で、main thread／AppKit起動完了後のNSOpenPanel factoryがNULLを返し、nonnullの固定bindingがpanicしてhelperを101で終了した。OS entitlement拒否そのものは未観測であり、終了値だけから断定しない。[Apple DTSの同種事例](https://developer.apple.com/forums/thread/735493)はNSTask childでGUI chooserを使う配置を推奨していない。[Appleのsandbox規約](https://developer.apple.com/library/archive/documentation/Miscellaneous/Reference/EntitlementKeyReference/Chapters/EnablingAppSandbox.html)はinherit helperへの追加entitlementを認めず、後から選択した動的アクセスはstatic inheritanceで渡らないと明示する。

従って子helperへentitlementを足す回避をせず、通常GUI lifecycleとuser-selected entitlementを持つ親process内の固定Rust UI部品へchooserだけを移す。NULLはnative境界でOptionとして拒否し、成功扱いしない。独立Rust Broker、Owner確認、資格・署名・Approval・Auditの責任は変更しない。これは実際のfactory失敗に対する恒久的なOS責任配置であり、失敗を隠すalternate Brokerではない。一時のAppKit起動補助、診断feature、stderr pipe／log分類は撤去する。

## 公開入力とnative接続

Flutterは既存`gui_shell/broker`へ`macos_workspace_selection.schema.json`のversion-only要求を送る。固定RunnerはFlutter入力に`native_workspace_selection`があれば拒否する。OS選択時だけ、公開要求byte列と同じhelperへの既存匿名pipe fdを固定C ABIへ渡す。ABIはRust所有chooserとOS scopeの配送・解放だけであり、Broker資格、署名検証、Approval token、command dispatch、Audit確定を持たない。Swiftへ返す整数は非同期選択を開始できたかだけで、配送成功・承認bool・権限ではない。

親Rust部品はmain thread、公開要求の厳密形状、version-only payload、固定client、byte／ID長、pipe種別、scope上限を検査する。chooserは単一folder、alias解決・folder生成なしで、UIから初期pathを受け取らない。通常GUI windowの公開sheet completion APIで開始し、同期modal／入れ子run loopを使わない。callbackは公開要求の所有copyと複製pipeだけを保持し、借用pointerを保存せず、一回だけ配送する。OS選択で得たURLのimplicit bookmarkを生成し、元の公開要求文字列とselected／cancelledをprivate frameへ結合し、既存pipeだけへ直接書く。bookmark本文をSwift／Dartへ返さず、永続保存・log・trace・Audit・snapshot・test artifactへ出さない。親は最大8件のURLだけを保持し、拒否時には当該要求が追加したscopeだけを解放する。取消は以前のscopeを削除しない。helper停止後に全scopeを終了し、停止前に開始した遅延callbackの配送・scope追加も世代照合で拒否する。

子Rust helperはprivate frameを厳密にparseし、元の公開要求の重複field、hash、時刻、client、session注入、nonceを既存検査で再評価する。scopeは最大8件、nonceは最大64件、frameは64KiB未満。OS bookmarkは24KiB以内、native URL resolverはUI禁止・stale拒否・絶対UTF-8 path／1024byte／control文字拒否で解決する。[Appleのprocess間アクセス規約](https://developer.apple.com/documentation/security/accessing-files-from-the-macos-app-sandbox?language=objc)に従い、implicit bookmarkを起動中だけ使い、同じURLのscopeを終了する。scope実値をBrokerへ渡さず、元要求hashと公開pathの内部投影だけをprocess内receiverへ渡す。private frameとbookmark文字列は処理後に消去する。再送・restartアクセス復元はない。

## AuthorityとContent Exposure

Brokerの`MacOSWorkspaceSelection` sourceはOwner sourceではない。通常IPC、Owner資格、UI metadataからこのsourceを作れず、Task Permission／Approval／登録を生成しない。Brokerは厳密4fieldの内部投影と元hashを検査し、Audit確定後にだけ8fieldの非秘密投影を返す。親／子のOS scope保持はこの起動中だけで、拒否時は解放する。終了はBroker停止→子scope解放→helper終了→親scope解放の順。

応答は`INTERNAL_STATE`で、path、selected／cancelled、元hash、`scope_lifetime=broker_process`、Permission／Approval／registrationが未生成であることだけを表す。Auditにはpathやbookmark本文を保存せず、正本化した内部投影hashを結合する。CONFIG／構造試験だけからOS実アクセスを保証しない。保護領域、APFS identity、secret除外、CLI probe、登録可否は、その後の別個Owner確認と既存Brokerが再評価する。

作用対応: CapabilityはOS folder選択、OS Permissionは明示選択URL、D4 Task Approvalは別、AuditはBrokerの選択投影hash、Recoveryは取消・不正・配送／監査失敗で非登録のままscope終了。raw native errorは露出せず、既存の固定拒否codeを使う。UI／Adapter／runtime metadataから権限を生成しない。

## 検証と残存範囲

native公開入力／private形状の正常・負例、既存Brokerの否定・replay・非権限投影、Dart取消／投影、通常Mac build、対象製品XCUITestを使う。実OS panelは親製品processの公開XCTest APIで取消・folder入力・選択を操作し、OS結果やBroker返答を注入しない。既存Debug表示flagは試験の表示補助だけで、通常Release証拠とは分離する。

`release_blocker`: 選択WorkspaceへのTask実行、restart access、Credential、正式署名・配布、最終QAはP13後続／既存release gateへ保持する。sandbox外CLI実行・CLI配布を本単位へ追加しない。通常Release `task_execution=unsupported`、`release_ready=false`は維持する。
