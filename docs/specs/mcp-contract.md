# MCP外部概念射影契約

## 対象

この契約は、MCP Serverから観測した外部概念をGUI-Shellの境界付きmetadataへ射影する。C8では実Serverへの接続、Tool実行、Credential取得、Permission付与を行わない。

射影対象は次のとおりである。

```text
Server
Tool
Resource
Prompt
Transport
Credential ref
Trust
Capability diff
```

## 信頼境界

Server metadata、Tool description、Resource URI、Prompt description、Transport情報、Credential ref、Trust状態、Capability diffは、観測または構成の説明であり、Authorityではない。

```text
MCP metadata ≠ Authority
Tool description ≠ Permission
Trust ≠ Approval
Capability diff ≠ Permission grant
Credential ref ≠ Credential value
```

契約の`権限生成`は常に`なし`である。Permission、Approval、Audit identity、secret value、実Credential値を入力へ含めず、公開範囲は`metadata_only`に固定する。`Credential ref`は保管庫のID、用途、接続対象、必要性、状態だけを持つ。

## Transportと証拠

Transportは`stdio`、`streamable_http`、`oauth`の分類と、対応状態、接続先hash、理由を持つ。接続先の実値、authorization header、OAuth token、process argvはこの契約へ保存しない。

Serverの`origin`と各`証拠種別`は、観測範囲を表す。`server_metadata`や`LIVE_RUNTIME`が`verified`のTrust状態を自動生成したり、Capability diffの追加を自動承認したりしてはならない。追加・変更Capabilityは`requires_operator_review=true`でなければならない。

## Tool inputSchema検証

MCP Toolの`inputSchema`はJSON Schema Draft 2020-12としてmeta-schema検証し、Broker内でvalidatorを構築できた場合だけcatalogへ受け入れる。`$schema`省略時もDraft 2020-12として扱い、明示dialectは既知の2020-12 URI表記だけを受け付ける。`$ref`と`$dynamicRef`は同一文書内fragment参照だけを許可し、HTTP／fileを含む外部参照は取得しない。入力Schemaは128 KiB、4096 JSON node、深さ64を上限とし、超過・不正・未対応dialectは接続をfail-closedにする。

Schema検証済みの`status=supported`はToolの実行可能性、Permission、Approvalを意味しない。現行接続経路は依然metadata-onlyであり、`tools/call`を送らない。

## 実装範囲

C8の作業単位はSchema、正常／負例fixture、Conformanceを追加した。後続C9は、実物interfaceを推測せずBroker経路でstdio discovery、connect、metadata list、timeout処理およびWindows owner切断を接続した。consent、Tool実行、Credential実値注入、Streamable HTTP、OAuth、quarantine、外部MCP実物Test Harnessは未接続であり、`release_blocker`として保持する。
