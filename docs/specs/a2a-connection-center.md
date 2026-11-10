# A2A接続センター

## 対象

C16の最初の完結単位は、owner controlからA2A Agent Cardを取得し、接続receiptへboundedな`metadata_only`として射影する経路である。外部Agentの宣言を自動信頼せず、接続成功とTrust、Permission、Approval、Authorityを分離する。

現行Transportは、Rust helperのloopback IPv4 HTTP取得だけである。userinfo、query、fragment、空白または非ASCIIを含むpathを拒否する。HTTPS、外部公開HTTP、redirect、Transfer-Encoding、Content-Encoding、重複Content-Length、巨大header、巨大body、credential実値を受け付けない。Agent Cardのendpointはhashだけを保存する。

Agent Card内の各`supportedInterfaces`では`url`と`protocolVersion`を検証する。interface URL実値は保存・表示せず、`interface_hash`、protocol binding／version、transportだけをmetadataへ射影する。Task／Message consumerは未接続のため、宣言されたinterfaceとstreamingは`unsupported`を返す。metadataを実行可能性へ昇格しない。

従来の保存receiptは起動時に読み取り専用で保守的に整形する。旧版にはinterface URLが保存されていないため`interface_hash=null`、protocol version／status=`unknown`へ降格し、`restored_pending_review`／`owner_reapproval_required`を保持する。旧履歴から接続・Trust・Permission・Approvalを再生成しない。

## 実行経路

```text
Desktop A2A接続センター
  → Rust起動器のnative Owner確認
  → 認証済みBroker IPC
  → A2A接続
  → Rust A2A helper
  → loopback HTTP Agent Card
  → Agent Card検証
  → LIVE_RUNTIME metadata-only receipt

Desktop A2A接続センター
  → 認証済みBroker IPC（通常要求）
  → A2A接続一覧
  → INTERNAL_STATE metadata-only一覧

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

接続要求の受信・Agent Card取得・受理・通常一覧返却は永続Auditを要求する。有効な`connected` receiptと同じAgentIDの重複接続は拒否する。例外は、復元時にBroker自身が設定した`restored_pending_review`／`owner_reapproval_required`／`INTERNAL_STATE`の三条件が一致するreceiptだけである。この場合も、同じAgentIDと新しいAgent Card URIを明示した別要求についてfresh native Owner確認を行い、その要求に対してAgent Cardを再取得・再検証する。過去のOwner承認・Permission・Approvalは再利用しない。Agent Card取得、Auditまたは永続化の失敗時は既存のpending receiptを保持する。一覧は64件、応答bodyは1MiB、headerは16KiB、接続・読取期限はboundedとする。接続先URIの実値はreceipt、Audit reason、CLI表示へ投影しない。

接続receiptの一覧はBroker session memoryへ展開し、`a2a_connections.json`へbounded metadataとして永続化する。永続Auditと接続stateは再起動時にstrict検証する。復元後はlive connectionではなく、再承認が必要な内部状態として扱う。

## C16補完: 再起動後の接続metadata復元

Brokerの永続storeへ`a2a_connections.json`を追加し、接続receiptのbounded metadataだけをatomic writeする。起動時は版、件数、receiptの固定field、Agent Cardのidentity、endpoint hash、credential ref、秘密値非保持を再検証し、malformedまたは未知fieldはBroker起動をfail-closedにする。

復元は外部Agentへの自動再接続やTask実行を意味しない。復元されたreceiptは`接続状態=restored_pending_review`、`証拠種別=INTERNAL_STATE`、`承認状態=owner_reapproval_required`へ降格する。過去のowner承認、履歴、metadataから現在のApproval、Permission、Authorityを生成しない。URI実値、credential実値、raw contentはstate fileへ保存しない。

pending receiptの復帰操作は、画面から同じAgentIDと現在確認したAgent Card URIを入力し、各要求に束縛されたnative Owner確認後に実行する。Brokerは対象receiptの三条件を再確認してから新しいAgent Cardを取得し、永続Auditと接続stateを更新する。新規・復帰のいずれもOwner確認を省略しない。取得・監査・永続化が失敗した場合、既存pending receiptはメモリ上・永続store上とも維持し、旧承認を有効化しない。このmetadata receipt更新は、A2A transport sessionの再確立や外部Task／Message／Artifact／Stream作用を意味しない。

通常の復元は接続registryの表示状態を復元する単位である。一般のTLS再接続、quarantine、disconnect、Task送信は実行せず、別のrelease_blockerとして扱う。

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

A2A接続一覧は通常IPCだけが参照できる。Owner操作channelでの一覧、Flutterからの直接network接続、Adapter metadataからの接続、履歴やProfileからの再利用は許可しない。Desktop画面の新規接続およびpending receipt再検証要求は、現在のAgentID・URIを示すRust起動器のnative Owner確認後に既存Brokerへ渡す。接続一覧と表示はBroker metadata projectionだけを使う。

Desktopの`A2A接続センター`は、必須`shell.agent_operation`に含まれ、Agent IDとAgent Card URIの入力、Broker接続一覧、接続metadata表示を提供する。接続URI実値はnative Owner確認の対象表示後、Flutterの入力欄から消去する。画面は接続をTrustへ昇格させず、Task／Message／Artifact／Streamの操作を提供しない。Agent Card由来の表示文字列は未信頼metadataと明示し、行制御・方向制御文字を含むprojectionは表示前に拒否する。

接続要求のCredential refは`credential_id`、`purpose`、`target`、`required`、`status`の固定5 fieldだけを受け付ける。IDは小文字hex 32桁、purposeは`A2A接続`、targetは同一要求のAgent ID、requiredは`false`、statusは`missing`に限る。未知fieldおよび秘密値fieldは拒否する。この参照からCredential保管庫を検索せず、credential実値を解決・注入しない。

## 現行未接続範囲

- HTTPSのTLS接続、公開endpoint discovery、OAuth／API keyなどの認証実行
- Task送信、Message送信、Artifact本文取得、Stream購読、cancel、A2A transport sessionの再接続、quarantine
- 外部Agent比較、複数Agent同一Workspace隔離、Handoff
- 実A2A Test Harness、Windows installed productでの接続証拠、長時間運用・障害注入

残るtransport、Task等の実作用、Trust審査、外部接続や正式配布の制約はrelease blockerまたは後続P7／P12／Final QAの各現行gateで分類する。本書のloopback接続単位は、Desktop入力面からnative Owner確認・Broker・metadata-only projectionまでを接続するProduct Build範囲であり、installed productでの実A2A運用やD4 Pocket全体の完成を主張しない。
