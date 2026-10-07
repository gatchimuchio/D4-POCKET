# macOS 作業領域のOS選択

## 意味と有限Acceptance

状態: IMPLEMENTING（P13 Product Build）。「作業領域OS選択」はOS chooserで選んだfolderの公開pathを表示するcontrol操作であり、Workspace登録・Permission・Approval・Credential・Agent trustではない。有限条件は取消時の入力保持、OS選択pathの画面反映、その選択folderを既存Owner確認／Broker登録へ渡すhappy pathと、UI由来のpath・bookmark・Authority注入の拒否だけ。CLIはsandbox内の既存実行系を使い、CLOSEDの登録機能は新しい選択機能の直接依存としてのみ使用する。

## 接続と境界

既存`gui_shell/broker` → 固定Runner → 匿名pipe → 同梱Rust helperを再利用する。公開入力は`macos_workspace_selection.schema.json`の`version=1`だけ。Rustがenvelope、現在時刻、hash、client、session注入禁止を検査してからmain threadのNSOpenPanelを開く。選択は単一folder、alias解決・folder生成なし。bookmark・秘密・任意初期pathを受け取らない。parent appにOS chooser用`com.apple.security.files.user-selected.read-write`を追加するが、sandboxを無効化せずhelperのinherit-only規則も変更しない。

OSが選んだURLはRustだけが最大8件保持する。再送は禁止、選択nonceは最大64件、transportは305秒を超えれば既存fail-closed終了になる。OS選択後、同一要求に選択結果と元の要求hashを結合し、内部payloadを再hashして既存process内receiverへ渡す。元入力は公開version-only要求として固定され、元hashをAudit対象の内部payloadへ結合する。この投影変換は明示的なnative normalizationであり、FlutterによるAuthority編集ではない。

Brokerは当該操作だけを`MacOSWorkspaceSelection`という別の内部sourceで処理する。これはOwner承認sourceではなく、Task Permission／Approvalに使えない。通常IPC・Owner資格・UI metadataからこのsourceを作れず、受信結果は厳密な形状、scope期限、pathの表示上限を検査する。選択結果のAudit確定後だけURLを保持し、失敗・拒否時はscopeを終了する。終了時はBrokerを先に停止し、保持URLを解放する。再起動にbookmarkやOSアクセスを継承しない。

応答はBroker経由の`INTERNAL_STATE`投影であり、path・selected/cancelled・元hash・起動中期限・`permission_generated=false`・`approval_generated=false`・`registration_generated=false`だけを返す。Auditには選択path本文を保存せず、正本化した内部payload hashを結合する。OS chooserによる実アクセスはMac製品試験が観測し、投影の構造確認だけから保証しない。選択はまだ登録でないため、保護領域・APFS物理identity・secret除外・CLI probeは既存登録の別個のOwner確認後にBrokerが再評価する。

## 外部APIと未成立範囲

失敗分類はnativeの固定enumだけをBrokerの既存拒否codeへ結合する。main thread、表示開始、URL、path、その他native失敗を区別し、raw error・path・秘密をcodeやmessageへ追加しない。未登録・入力保持・自動再送なしは全分類で同じ。通常IPCやOwner資格からこの分類sourceを生成できず、失敗codeはAuthorityではない。

作用対応: CapabilityはmacOSのuser-selected folder選択、OS Permissionはその場で選択したURL、OSの明示選択はD4 Task Approvalとは別、AuditはBrokerによる選択投影hashの記録、Recoveryは取消・不正・配送／監査失敗で非登録のままscopeを終了すること。画面は非秘密pathだけを投影し、登録後のfilesystem作用は既存Workspace Permission／Task Approvalへ従う。

[Appleのsandboxファイルアクセス規約](https://developer.apple.com/documentation/security/accessing-files-from-the-macos-app-sandbox)は、open panelのURLにOSがscopeを開始し、終了時に同じURLの`stopAccessingSecurityScopedResource`を要求する。Rust native境界に固定`objc2 0.6.5`、`objc2-app-kit 0.3.2`、`objc2-foundation 0.3.2`を追加し、AppKit main-thread markerを必要とする。Coreのunsafe禁止は維持する。OS URL解放のFFIだけが独立native crate内の局所unsafeであり、pointerやscope実値はDartへ渡さない。

Appleはuser-selected entitlementだけでsandbox外programを実行できるとはしていない。本単位は外部CLI起動・CLI配布を実装しない。選択WorkspaceへのTask実行、restart access、Credential、正式署名・配布、最終QAは既存`release_blocker`とP13後続へ保持する。通常Release `task_execution=unsupported`と`release_ready=false`を変更しない。
