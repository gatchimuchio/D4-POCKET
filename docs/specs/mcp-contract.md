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

## 実装範囲

この作業単位はSchema、正常／負例fixture、Conformanceだけを追加する。MCP discovery、connect、list、consent、Tool実行、timeout、disconnect、quarantineはC9のBroker経路で実装するまで未接続であり、`release_blocker`として保持する。外部MCPサービスの実物interfaceを推測して実装しない。
