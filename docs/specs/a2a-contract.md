# A2A外部概念射影契約

## 目的

C15は、外部A2AのAgent Card、Task、Message、Artifact、StreamをGUI-Shellの境界付きmetadataへ射影する契約単位である。A2Aの実接続、Agent Card取得、Task送信、Message送信、Artifact取得、Stream購読はC16以降で扱い、この単位では実行経路を追加しない。

## 射影対象

```text
Agent Card
Task
Message
Artifact
Stream
```

Agent CardはAgentの宣言情報であり、接続先はhashだけを保持する。説明、skill、interface、capability、認証schemeはboundedなmetadataとして扱い、endpoint実値、authorization header、token、秘密値、raw contentを保存しない。MessageとArtifactのpartsはkind、件数、hash、content visibilityだけを保持する。

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

Agent Card discovery、外部Agent登録、A2A binding、credential注入、Task送信、Message送信、Artifact本文、Stream購読、timeout、取消、再接続、quarantine、複数Agent比較、Handoffは`release_blocker`として残す。C15のSchema／fixture／ConformanceだけでA2A接続やD4 Pocketの製品完成を主張しない。
