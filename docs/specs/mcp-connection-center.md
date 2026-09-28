# MCP接続センター

## 対象

C9は、owner controlから明示されたMCP stdio ServerをRust Brokerのprocess境界へ接続し、MCPのdiscoveryとTool／Resource／Prompt一覧を検証済みmetadataへ射影する。MCPの外部metadataはC8と同じくAuthority、Permission、Approval、Trustを生成しない。

現行の接続対象は`stdio`だけである。現行MCP lifecycleの`server/discover`を先に試し、旧`initialize`／`notifications/initialized`はlegacy protocolとしてだけfallbackする。外部Serverの実interfaceを推測せず、JSON-RPC wire、response id、capability、list結果、重複、pagination、secret／authority fieldをBroker側で検証する。receiptの公開範囲は`metadata_only`に固定する。

## 実行経路

```text
owner control
  → MCP接続
  → Rust Broker
  → MCP stdio child process
  → server/discover または initialize
  → tools/list・resources/list・prompts/list
  → metadata-only receipt
```

`MCP接続一覧`は通常IPCの読み取り専用経路であり、owner channelからは拒否する。接続開始、受信、受理、一覧返却には永続Auditを要求する。child processは環境変数をallowlistへ制限し、stderrを取り込まず、response timeout、line上限、終了状態をfail-closedで扱う。

ownerは`MCP切断`で対象Server IDを明示できる。要求はowner controlだけで受け、未知field、未知Server、通常IPCからの切断は拒否する。Windows Job Objectのprocess群停止確認後に`LIVE_RUNTIME`の結果Auditを永続化し、その後に限ってBrokerの接続記録を消す。停止または結果Auditを確定できない場合は成功を返さず記録を保持する。停止済みだがAuditに失敗した場合は、ownerの明示再試行で停止を再確認して監査確定する。receipt／Auditに実行path、引数、workspace、Credential実値を含めない。切断はPermissionを生成しない。

WindowsではMCP stdio Serverも既存のRust process群監督経路から起動する。childの初期threadを再開する前に専用Job Objectへ割り当て、Broker異常終了時のhandle closeでrootと子孫を停止する。root終了後に子孫が残っている場合もBrokerは接続一覧を返さず、終了を確認できない停止要求を成功扱いしない。この保証はWindows process群の範囲であり、MCP ServerのTrust、filesystem sandbox、credential安全性は証明しない。

## 境界

- `MCP metadata ≠ Authority`
- `Tool description ≠ Permission`
- `Trust ≠ Approval`
- `Capability diff ≠ Permission grant`
- `Credential ref ≠ Credential value`
- Agent metadata、MCP metadata、履歴、Profile、Tool schemaから権限を生成しない。
- Credential実値の注入、Tool実行、Resource／Prompt実取得、Streamable HTTP、OAuth、consent、quarantineはこの単位では接続しない。
- 非Windowsではprocess群停止を保証する既存監督がないため、`mcp_process_tree_supervision_unsupported`として`MCP切断`をfail-closedで拒否する。WindowsのJob Object試験を他OSのprocess群停止証拠へ流用しない。
- Windowsのfake MCP stdio child／descendant process fixtureはJob Objectによる起動・停止だけを検証する。外部MCP Serverの適合やinstalled product経路の証拠ではない。Windows以外のprocess群監督は別途検証を要する。
- `server/discover`またはlegacy `initialize`の応答は、Serverが信頼済みまたは承認済みであることを証明しない。

## 失敗と残存範囲

未知Server、応答id不一致、malformed JSON-RPC、authority／secret field、未処理pagination、応答timeout、child終了、必須Credentialの不足は接続または一覧を拒否する。`mcp_server_unavailable`と`mcp_timeout`を成功へ変換しない。

Tool実行、Credential実値注入、外部MCP実物Test Harness、非Windows process群監督が未成立であることは`release_blocker`である。C9のstdio discovery／catalog／Windows owner切断だけで、D4 Pocket全体または正式releaseの完成を主張しない。
