# A2A外部概念射影契約

## 目的

C15は、外部A2AのAgent Card、Task、Message、Artifact、StreamをGUI-Shellの境界付きmetadataへ射影する契約単位であり、この歴史的な単位自体は実行経路を追加しない。C16の追加経路として、認証なしA2A v1.0 JSON-RPC interfaceへの一回SendMessageを実装する。応答はbounded metadataへ投影し、raw本文を保持しない。Task状態取得は元のSendMessage経路に含めず、Task応答に対する手動一回GetTask照会だけをC16補完として別契約に定める。Artifact選択表示ではcompleted Taskから操作者が選択したtext-only Artifactに限り、fresh native Owner確認後の一回取得と揮発表示を追加する。Task lifecycle polling／完了判定、file／data Artifact、Stream、認証Credentialは対象外とする。

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

OAuth／API key等のCredentialを要する送信、送信後のTask lifecycle追跡・follow-up、未選択／file／data Artifact本文、Stream購読、再接続、quarantine、複数Agent比較、Handoffは現行の後続gateに残す。C15のSchema／fixture／Conformance、またはC16の一回loopback `SendMessage`だけで実外部Agent運用やD4 Pocketの製品完成を主張しない。

C15で未成立の機能範囲は`release_blocker`として追跡する。限定されたC16送信経路のProduct Build evidenceを外部Agent運用・製品受入へ昇格させない。

## C16補完: 明示一回A2A GetTask状態照会

C16 `SendMessage`がTask metadataを返した場合に限り、Windows／Linux Desktopの画面から、操作者が明示した時だけ、その同一Agent・同一interface・同一Task IDに対するA2A v1.0 JSON-RPC `GetTask`を一回実行できる。定期polling、自動retry、バックグラウンド照会、Task完了を前提とする後続作用は行わない。A2A v1.0 specificationの`historyLength: 0`を送信し、応答でもTask IDの完全一致および空履歴を要求する。

各照会は新しいnative Owner確認、現在のLIVE_RUNTIME接続receipt/interface hash再検証、永続Audit、および要求ID／Agent ID／interface hash／Task ID hashに束縛した一回限りPermissionを必要とする。照会結果不明またはBroker再起動で`started`のみ残った場合は`unknown`として閉じ、自動再照会しない。再照会は新しい画面操作・Owner確認・Permission・Auditを要する。

C16のmetadata-only照会は、Task ID、bounded context ID、状態enum、Artifact件数、history_count=0、response hashと、後続の明示選択で対象を決めるbounded Artifact manifestだけを返す。manifestはArtifact ID/hash、part件数、text-only判定、64 KiB以内の場合のUTF-8 byte countに限る。Task status message、history、未選択Artifact本文、外部metadata、Credential実値は保存・Audit・表示しない。Task IDは画面上だけのvolatile stateで保持し、credential・Permission・Approval・Trust・Authorityへ昇格させない。Trustはpending_review、authority_strip=true、権限生成=なしを維持する。

macOS native A2A Owner契約には本操作を追加せず、macOS画面では照会操作を提供しない。これはC16のMessage送信対応範囲を変更せず、外部A2Aの全Task lifecycle、Artifact、Stream、cancel、Task完了判定を完成扱いしない。実外部Agent／installed product／macOS／Linux製品運用の証拠は別途QA／release gateで判断する。

## A2A Artifact選択表示: 明示選択されたtext Artifactの取得・表示

A2A v1.0 GetTask responseに含まれるArtifact manifestは、最大32件、Artifactごとに最大32 partsとして検査する。通常のmetadata照会ではID、Artifact全体hash、part件数、text-only判定、64 KiB以内の場合のUTF-8 byte countだけを返し、part本文・name・description・metadata・extensionsは公開しない。A2A GetTaskのwire応答に全Artifact partsが含まれる場合はあるため、Brokerは64 KiBのresponse上限内で受信・解析し、選択されていない本文を結果projection・Audit・Workspaceへ渡さない。A2A v1.0のArtifact／Part構造に合わせるが、選択時もkind=text以外のpartを受け付けない。[A2A v1.0.0 specification](https://a2a-protocol.org/v1.0.0/specification/)

操作者が一覧から一件を明示選択した時は、同じTaskに対する別のGetTask requestを作り、ArtifactIDと前回manifestのArtifact hashを組で送る。選択要求全体のhashを表示した新しいRust native Owner確認を必須とし、Brokerは同じ要求hashに束縛した一回Permissionを発行・消費した後だけ外部照会する。PermissionやOwner確認は前回照会から引き継がない。Taskがcompletedで、現在の外部応答のArtifact全体hashが選択hashと一致し、全partsが安全なtext、合計64 KiB以下の場合だけ全文を返す。差替え、非text、選択Artifactのfile/data、危険な方向制御・制御文字、件数／size超過はfail-closedで拒否する。他Artifactやfile/dataがGetTask応答に含まれていても、その本文を画面・Audit・Workspaceへ公開しない。

全文は外部未信頼textとして画面に明示表示するvolatile UI stateだけに置く。Workspace、履歴、snapshot、log、trace、Audit本文へ複製せず、外部metadataからApproval／Permission／Trust／Authorityを生成しない。Auditは要求hash、Artifact ID hash、result／Artifact／content hash、byte countだけを記録する。再表示・再取得には毎回新しいnative Owner確認とPermissionが必要で、自動取得しない。Windows／Linux Desktopを対象とし、macOS native A2A contractは変更しない。

本実装はA2A全Artifact形式、file/data transfer、Task lifecycle、Stream、external／installed AgentのProduct QAを成立させない。N04、W02、A2A release blockerは既存gateのまま評価する。
