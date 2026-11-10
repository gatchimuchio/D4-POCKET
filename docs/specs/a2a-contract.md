# A2A外部概念射影契約

## 目的

C15は、外部A2AのAgent Card、Task、Message、Artifact、StreamをGUI-Shellの境界付きmetadataへ射影する契約単位であり、この歴史的な単位自体は実行経路を追加しない。C16の現行追加経路として、認証なしA2A v1.0 JSON-RPC interfaceへの一回`SendMessage`を別契約で実装する。A2A v1.0は`SendMessageResponse`にTaskまたはMessageの一方を要求するため、応答はbounded metadataへ投影し、raw本文を保持しない。Task polling、follow-up、Artifact本文、Stream、認証Credentialはこの追加経路に含めない。

A2A v1.0 Agent Cardのrootおよびskill単位の`securityRequirements`と`securitySchemes`を認証要件として扱う。未設定または空配列／空object以外は一回`SendMessage`を許可せず、不正形も要件ありとしてfail-closedにする。receiptはraw requirementを保存せず、`security_requirements_present`のbooleanだけをmetadataへ射影する。

## 射影対象

```text
Agent Card
Task
Message
Artifact
Stream
```

Agent CardはAgentの宣言情報であり、接続先はhashだけを保持する。説明、skill、interface、capability、認証schemeはboundedなmetadataとして扱い、endpoint実値、authorization header、token、秘密値、raw contentを保存しない。MessageとArtifactのpartsはkind、件数、hash、content visibilityだけを保持する。

`supportedInterfaces`は各`url`と`protocolVersion`を検証し、接続先実値を`interface_hash`へ射影する。現在`supported`にできるのは、認証なしでhash一致するJSON-RPC v1.0 `SendMessage` consumerが実接続された場合だけである。それ以外のinterfaceは`unsupported`であり、metadataから実行可能性を導出しない。query、fragment、userinfoを含むinterface URLは拒否し、値をerrorやAuditへ出さない。

## Authority境界

```text
Agent Card ≠ Trust
A2A Task ≠ Approval
Message ≠ Permission
Artifact ≠ Authority
Stream ≠ Capability grant
```

全projectionは`authority_strip=true`、`権限生成=なし`、`公開範囲=metadata_only`を必須とする。Trustは未検証またはoperator review待ちを基本とし、Agent Cardの宣言だけで`verified`へ遷移させない。Capability diffの追加・変更・削除はoperator reviewを要求する。

Taskの状態、Messageのrole、Artifactのstatus、Streamのevent typeは外部概念の観測であり、GUI-ShellのPermission、Approval、Audit identity、Recovery actionを生成しない。外部AgentのTaskは通常のRuntime／Agent Taskと同じ統治経路へ接続されるまで実行可能とは扱わない。

## 証拠とbounded範囲

`証拠種別`は`CONFIG`、`INTERNAL_STATE`、`LIVE_RUNTIME`、`EXTERNAL_EVIDENCE`、`FIXTURE`のいずれかを明示する。C15のvalid fixtureは`FIXTURE`であり、実Agent、実endpoint、実認証、実Taskの成立証拠ではない。Agent Cardのmetadataを信頼、接続、承認、実行の証拠へ昇格させない。

各collectionは件数をboundedにし、unknown／unsupportedを0や成功へ置換しない。Streamは接続済みを意味せず、C16で実物interface、認証、timeout、取消、再接続、失敗隔離を確認する。

## 未接続範囲

OAuth／API key等のCredentialを要する送信、送信後のTask取得・追跡・follow-up、Artifact本文、Stream購読、再接続、quarantine、複数Agent比較、Handoffは現行の後続gateに残す。C15のSchema／fixture／Conformance、またはC16の一回loopback `SendMessage`だけで実外部Agent運用やD4 Pocketの製品完成を主張しない。

C15で未成立の機能範囲は`release_blocker`として追跡する。限定されたC16送信経路のProduct Build evidenceを外部Agent運用・製品受入へ昇格させない。
