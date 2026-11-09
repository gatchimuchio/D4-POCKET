# macOS Codex Provider資格情報binding

## 対象と責任境界

本契約は、macOS上のAgent CenterからCodex Provider API keyを登録し、既存Rust Brokerが承認済みCodex Taskの実行時だけKeychainから読み出してCodex CLI childへ渡すconsumerを定める。Keychainの登録・一覧・失効基盤は`macos-credential-vault.md`のCLOSED範囲を再利用する。MCPへの資格情報注入は`macos-mcp-credential-binding.md`の独立範囲であり、この契約へ含めない。

Agent Centerは固定の公開metadata（`provider_api_key`／`openai_codex_cli`／`api_key`）だけを既存Broker登録要求へ送る。秘密値は固定Rust native入力UIからprivate pipeを通り、別個の期限付きOwner確認後に既存Broker Owner receiverへ渡す。秘密入力とOwner確認は別の作用であり、Dart、Swift、通常IPCの公開要求・応答・Audit・error・log・traceへ秘密値を出さない。

Codex Task登録時とTask実行時にBrokerが現在のCredential ledgerを再照合する。Task実行側は既存Permission／Approval消費後に限り、用途・Provider target・有効状態・暗号文hashを結び付けてKeychainから短命zeroizing bufferへ読み出し、使用Auditを確定してから既存Codex CLI adapterへ渡す。Credential ID、Provider metadata、登録receiptは権限ではない。失効・対象不一致・未登録・改変・Audit不成立はfail-closedとし、Codex CLI管理認証へ自動fallbackしない。

## 有限なBUILD受入れ

- macOS Agent Centerから開始する登録要求の公開fieldは`版`・`資格情報ID`・`用途`・`接続対象`・`種類`だけで、固定用途・対象・種類および`macos_keychain` receiptを確認する。
- BrokerのProvider参照確認とTask時使用処理がmacOSで既存Keychain storeを使用し、現在record・対象Provider・状態・暗号文hashを再評価する。
- 合成秘密値だけでKeychainの登録→内部読取→使用Audit→論理失効→失効後拒否を検証する。秘密値は公開receipt・Audit・Flutter応答へ含めない。
- 他OSからmacOS登録methodを呼ぶ要求、別Provider、未知ID、失効済みIDを拒否する。
- 正常Releaseの`task_execution`、Provider health、実外部Provider通信、署名・配布、Final QAはこの受入れから昇格させない。

## 証拠境界

macOS hosted runnerのKeychain実API試験はそのrunner上の`LIVE_RUNTIME`証拠である。Flutter Widget／Client試験は`FIXTURE`であり、installed Mac製品・実使用者・正式identityの`LIVE_PRODUCT`証拠ではない。実Provider endpointや本番資格情報は使用しない。
