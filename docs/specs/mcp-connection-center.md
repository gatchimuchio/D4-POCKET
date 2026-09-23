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

## 境界

- `MCP metadata ≠ Authority`
- `Tool description ≠ Permission`
- `Trust ≠ Approval`
- `Capability diff ≠ Permission grant`
- `Credential ref ≠ Credential value`
- Agent metadata、MCP metadata、履歴、Profile、Tool schemaから権限を生成しない。
- Credential実値の注入、Tool実行、Resource／Prompt実取得、Streamable HTTP、OAuth、consent、disconnect、quarantineはこの単位では接続しない。
- `server/discover`またはlegacy `initialize`の応答は、Serverが信頼済みまたは承認済みであることを証明しない。

## 失敗と残存範囲

未知Server、応答id不一致、malformed JSON-RPC、authority／secret field、未処理pagination、応答timeout、child終了、必須Credentialの不足は接続または一覧を拒否する。`mcp_server_unavailable`と`mcp_timeout`を成功へ変換しない。

Tool実行と外部MCP実物Test Harnessが未成立であることは`release_blocker`である。C9のstdio discovery／catalog接続だけで、D4 Pocket全体または正式releaseの完成を主張しない。
