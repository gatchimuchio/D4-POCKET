# A2A接続センター

## 対象

C16の最初の完結単位は、owner controlからA2A Agent Cardを取得し、接続receiptへboundedな`metadata_only`として射影する経路である。外部Agentの宣言を自動信頼せず、接続成功とTrust、Permission、Approval、Authorityを分離する。

現行Transportは、Rust helperのloopback HTTP取得だけである。HTTPはloopback IPへ限定し、HTTPS、外部公開HTTP、redirect、Transfer-Encoding、Content-Encoding、重複Content-Length、巨大header、巨大body、credential実値を受け付けない。Agent Cardのendpointはhashだけを保存する。

## 実行経路

```text
owner control
  → A2A接続
  → Rust Security Broker
  → Rust A2A helper
  → loopback HTTP Agent Card
  → Agent Card検証
  → LIVE_RUNTIME metadata-only receipt

通常認証済みIPC
  → A2A接続一覧
  → Broker内部接続状態
  → INTERNAL_STATE metadata-only一覧
```

接続要求の受信・Agent Card取得・受理・通常一覧返却は永続Auditを要求する。同じAgentIDの再接続は拒否し、一覧は64件、応答bodyは1MiB、headerは16KiB、接続・読取期限はboundedとする。接続先URIの実値はreceipt、Audit reason、CLI表示へ投影しない。

接続receiptの一覧は現在のBroker session memoryに保持する。永続Auditは再起動後も検証対象になるが、A2A接続registry自体の再起動後復元は未接続であり、release_blockerとして扱う。

## Authorityと内容露出

- Agent CardはTrustを生成しない
- A2A metadataはPermissionではない
- A2A TaskはApprovalではない
- Credential refはCredential valueではない
- Capability diffはAuthorityではない
- authority_strip=true
- 権限生成=なし
- 公開範囲=metadata_only

Trustはpending_review、Capability diffはnot_evaluatedとしてoperator reviewを要求する。Agent Cardの能力、skill、認証scheme、streaming宣言は説明情報であり、Task実行資格、Permission、Approval、Credential実値を生成しない。Task、Message、Artifactのraw本文は取得・保存・表示しない。

A2A接続一覧は通常IPCだけが参照できる。owner操作からの一覧、UIからの直接接続、Adapter metadataからの接続、履歴やProfileからの再利用は許可しない。

## 現行未接続範囲

- HTTPSのTLS接続、公開endpoint discovery、OAuth／API keyなどの認証実行
- Task送信、Message送信、Artifact本文取得、Stream購読、cancel、再接続、quarantine
- 外部Agent比較、複数Agent同一Workspace隔離、Handoff
- D4 Pocket Desktopの専用接続画面
- 実A2A Test Harness、Windows installed productでの接続証拠、長時間運用・障害注入

上記はrelease_blockerである。C16の現行単位は、loopback HTTP Agent Cardの実取得、Broker統治、metadata-only receipt、通常一覧、拒否試験までを成立させる。A2A接続が実装されたことだけでD4 Pocketまたは正式releaseの完成を主張しない。
