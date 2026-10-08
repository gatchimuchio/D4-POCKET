# macOS Agent CLI実行fileのOS選択

## 意味と有限Acceptance

状態: IMPLEMENTING（P13 Product Build）。sandbox外の既存CLI実行fileをOS chooserで一個選び、その公開pathをAgent登録フォームへ反映する。download・導入・更新ではない。選択だけではprocessを起動せず、Runtime登録・Permission・Approval・Credential・Trustを生成しない。その後の別個native Owner確認と既存BrokerのCLI probeが登録可否を決める。

有限条件は取消時のCLI入力保持、通常fileの実OS選択と画面反映、別個Owner確認後の実Codex CLI probe／登録、公開入力へのpath・bookmark・Authority注入拒否、正常終了時のscope回収。既存Workspace選択／Agent登録のCLOSED条件は再検査せず、既存登録を直接依存として使う。Task、模型要求、認証、正式配布、全CLI互換性は対象外。

## 接続と責任

公開要求は`AgentCLI実行fileOS選択`、version-onlyの`macos_agent_cli_selection.schema.json`。親GUIに同梱した既存Rust UI ABIはこの固定操作だけを追加し、[NSOpenPanelのfile選択](https://developer.apple.com/documentation/appkit/nsopenpanel/canchoosefiles)を単一file・directory選択不可・alias解決なしで構成する。UI由来の初期pathやscopeは受け取らない。操作は公開要求に結合し、private frameへの別のmode指定を受け付けない。

OSが返すimplicit bookmarkは既存匿名pipeを通して子Rust helperだけへ渡す。[Appleのprocess間アクセス規約](https://developer.apple.com/documentation/security/accessing-files-from-the-macos-app-sandbox)に従い、既存Workspace用のprivate輸送・世代照合・最大8 scope・nonce上限・ゼロ化・終了回収を再利用する。ABI名／既存Workspace契約は変更しない。Swift／Flutter／Audit／artifactへbookmarkを出さず、sandboxとhelperのinherit-onlyを維持する。URLの解決後に子Rustは通常file・symlinkではないことを検査し、OS実行許可とCLI interfaceは別個登録の既存probeで確認する。

Brokerの`MacOSAgentCLISelection` sourceはOwner sourceではない。通常IPC、Owner資格、UI metadataはこれを生成できない。厳密な内部投影を受け、Audit確定後だけ`macos_agent_cli_selection_receipt.schema.json`のpath／取消投影を返す。pathとbookmarkはAudit本文へ入れず投影hashだけを結合する。OSアクセスをTask Authorityへ昇格せず、既存登録とTask非対応判定を保持する。

作用対応: Capability=単一CLI fileのOS選択、OS Permission=明示選択URLの起動中access、D4 Approval=別個登録のnative確認、Audit=非秘密投影hash、Recovery=失敗時入力保持・非登録・自動再送なし。応答は`INTERNAL_STATE`で、CLIの無害性やTask成功を証明しない。

## 検証と残存範囲

新CLI操作のnative公開入力／Broker境界／Dart投影を局所試験し、手動Actions `macos_agent_cli_selection`で通常Mac buildと専用製品XCUITestを実行する。公式固定Codex CLIをsandbox外の合成fileに配置し、選択後は実`--version`／`exec --help`だけを使う。既存Debug表示補助はUI試験限定。

`release_blocker`: CLI配布・restart access・Agent Task・Credential保管・正式署名配布とFinal QAは既存gateへ保持する。通常Release `task_execution=unsupported`、`release_ready=false`を変更しない。
