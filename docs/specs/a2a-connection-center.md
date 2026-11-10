# A2A接続センター

## 対象

C16では、owner controlからA2A Agent Cardを取得し、接続receiptへboundedな`metadata_only`として射影する経路に加え、接続済み・無認証JSON-RPC v1.0 interfaceへの一回Message送信を扱う。外部Agentの宣言を自動信頼せず、接続成功とTrust、Permission、Approval、Authorityを分離する。

現行TransportはRust helperが所有するloopback IPv4 HTTPとpublic HTTPSである。HTTPはloopback IPv4だけを許可し、HTTPSはOS既定の証明書信頼storeでhost／certificateを検証する。HTTPSのDNS結果は全件をpublic IP検査し、解決した固定addressへ接続する。private addressが一件でも含まれる場合は拒否する。HTTPSではproxy、redirect、自動retry、HTTP/2を使わず、HTTP/1、`Accept: application/json`、`Accept-Encoding: identity`で取得する。userinfo、query、fragment、空白または非ASCIIを含むhost／path、Transfer-Encoding、Content-Encoding、重複Content-Length、不正なstatus／Content-Type、16 KiB超または64 field超のheader、1 MiB超のbody、credential実値を拒否する。bodyは8秒期限内に読み切る。OSの名前解決呼出しが期限内に戻らない場合もBroker要求は期限で終了し、追加のresolver workerを起動せずfail-closedにする。HTTPSのinterface URLはhashだけを保存する。

IPv6の`2001::/23` special-purpose親範囲は、IANA registryでより具体的なallocationが`Globally Reachable=true`とされる場合だけ許可する。対象は`2001:1::1/128`〜`2001:1::3/128`、`2001:3::/32`、`2001:4:112::/48`、`2001:20::/28`、`2001:30::/28`である。それ以外の親範囲内アドレス、benchmarkingの`2001:2::/48`、deprecatedな`2001:10::/28`は拒否する。[IANA IPv6 Special-Purpose Address Space](https://www.iana.org/assignments/iana-ipv6-special-registry)。この判定関数は更新取得経路でも共有される。

Agent Card内の各`supportedInterfaces`では`url`と`protocolVersion`を検証する。interface URL実値は保存・表示せず、`interface_hash`、protocol binding／version、transportだけをmetadataへ射影する。A2A v1.0 rootの`securityRequirements`と各skillの`securityRequirements`を調べ、その要件または`securitySchemes`があるinterfaceは`unsupported`にする。該当fieldが不正形の場合も要件ありとして扱う。receiptにはraw要件を保存せず、`security_requirements_present`だけを投影する。要件がなく、他の条件も満たすinterfaceには一回`SendMessage` consumerを接続する。Task応答への手動一回`GetTask`照会は下記C16補完に限り、Task lifecycle polling／完了判定、Artifact本文、Stream consumerは未接続のままである。Agent Cardの宣言自体を実行可能性の証拠へ昇格しない。

従来の保存receiptは起動時に読み取り専用で保守的に整形する。旧版にはinterface URLが保存されていないため`interface_hash=null`、protocol version／status=`unknown`へ降格し、`restored_pending_review`／`owner_reapproval_required`を保持する。旧履歴から接続・Trust・Permission・Approvalを再生成しない。

## 実行経路

```text
Desktop A2A接続センター
  → Rust起動器のnative Owner確認
  → 認証済みBroker IPC
  → A2A接続
  → Rust A2A helper
  → loopback HTTPまたは証明書検証済みpublic HTTPS Agent Card
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
  → loopback HTTPまたは証明書検証済みpublic HTTPS Agent Card
  → Agent Card検証
  → LIVE_RUNTIME metadata-only receipt

通常認証済みIPC
  → A2A接続一覧
  → Broker内部接続状態
  → INTERNAL_STATE metadata-only一覧

Desktop A2A接続センター
  → Rust起動器の本文全体native Owner確認
  → 認証済みBroker IPC
  → 一回Permissionの発行・消費
  → Rust A2A helper
  → 現在receipt hashに一致する無認証JSON-RPC v1.0 SendMessageを一回
  → bounded Task／Message metadata projectionまたは結果不明
  → 永続Audit
```

接続要求の受信・Agent Card取得・受理・通常一覧返却は永続Auditを要求する。有効な`connected` receiptと同じAgentIDの重複接続は拒否する。例外は、復元時にBroker自身が設定した`restored_pending_review`／`owner_reapproval_required`／`INTERNAL_STATE`の三条件が一致するreceiptだけである。この場合も、同じAgentIDと新しいAgent Card URIを明示した別要求についてfresh native Owner確認を行い、その要求に対してAgent Cardを再取得・再検証する。過去のOwner承認・Permission・Approvalは再利用しない。Agent Card取得、Auditまたは永続化の失敗時は既存のpending receiptを保持する。一覧は64件、応答bodyは1MiB、headerは16KiB、接続・読取期限はboundedとする。接続先URIの実値はreceipt、Audit reason、CLI表示へ投影しない。

接続receiptの一覧はBroker session memoryへ展開し、`a2a_connections.json`へbounded metadataとして永続化する。永続Auditと接続stateは再起動時にstrict検証する。復元後はlive connectionではなく、再承認が必要な内部状態として扱う。

## C16補完: 再起動後の接続metadata復元

Brokerの永続storeへ`a2a_connections.json`を追加し、接続receiptのbounded metadataだけをatomic writeする。起動時は版、件数、receiptの固定field、Agent Cardのidentity、endpoint hash、credential ref、秘密値非保持を再検証し、malformedまたは未知fieldはBroker起動をfail-closedにする。

復元は外部Agentへの自動再接続やTask実行を意味しない。復元されたreceiptは`接続状態=restored_pending_review`、`証拠種別=INTERNAL_STATE`、`承認状態=owner_reapproval_required`へ降格する。過去のowner承認、履歴、metadataから現在のApproval、Permission、Authorityを生成しない。URI実値、credential実値、raw contentはstate fileへ保存しない。

pending receiptの復帰操作は、画面から同じAgentIDと現在確認したAgent Card URIを入力し、各要求に束縛されたnative Owner確認後に実行する。Brokerは対象receiptの三条件を再確認してから新しいAgent Cardを取得し、永続Auditと接続stateを更新する。新規・復帰のいずれもOwner確認を省略しない。取得・監査・永続化が失敗した場合、既存pending receiptはメモリ上・永続store上とも維持し、旧承認を有効化しない。このmetadata receipt更新は、A2A transport sessionの再確立や外部Task／Message／Artifact／Stream作用を意味しない。

通常の復元は接続registryの表示状態を復元する単位である。一般のTLS再接続、quarantine、disconnect、Task送信は実行せず、別のrelease_blockerとして扱う。

## C16補完: 一回A2A Message送信

この追加経路は、現在の`connected`／`LIVE_RUNTIME` receipt、未信頼`pending_review` Trust、hashが完全一致する`JSONRPC`／protocol version `1.0` interfaceだけへ、本文1件を`SendMessage`として送る。Credential schemeまたはsecurity requirementを宣言するAgentは送信対象にしない。HTTPSは既存のpublic-IP固定・証明書検証経路、HTTPはIPv4 loopbackに限定する。A2A v1.0の`A2A-Version: 1.0`とJSON-RPC 2.0を使い、Task polling、streaming、retry、redirect、credential、file/data partは扱わない。

各送信には新しい要求hashへ束縛したRust native Owner確認を要求し、対象Agent、送信先、本文全体（改行・復帰・tabを可視表示）、本文hash、外部作用、再送禁止を表示する。Brokerは一回Permissionを現在のrequest ID／Agent／interface hash／本文hashに束縛して発行・即時消費し、永続Auditの`started`確定後にだけ送信する。同じ許可・履歴・Trustから後続Permissionを作らない。Trustは`pending_review`、権限生成なしを維持する。

成功応答はJSON-RPC request IDを照合し、TaskまたはMessageの一方だけを受理する。応答からID、state、part／artifact／history件数、response hashなどbounded metadataだけを投影し、raw応答、Message本文、Artifact本文は保存・表示しない。Taskの`accepted`／`submitted`は完了を意味しない。外部作用後にtimeout、通信切断、JSON不正、request ID不一致またはAudit確定失敗が起きた場合は`unknown`／`suspended`として再送可能をfalseにし、自動再送しない。Broker再起動時に`started`のままterminal Auditがない要求もunknownへ回復し、再実行しない。

この操作はowner control経路のみで受け付け、通常IPC・Owner credential・Flutter直接networkからは実行できない。Flutterは要求をBrokerへ運ぶだけであり、Permission、Approval、Authorityを決定しない。Windows／macOS／Linux Desktopのnative確認面は同一要求を確認し、送信要求を既存Broker receiverへ一回だけ渡す。

## Authorityと内容露出

- Agent CardはTrustを生成しない
- A2A metadataはPermissionではない
- A2A TaskはApprovalではない
- Credential refはCredential valueではない
- Capability diffはAuthorityではない
- authority_strip=true
- 権限生成=なし
- 公開範囲=metadata_only

Trustはpending_review、Capability diffはnot_evaluatedとしてoperator reviewを要求する。Agent Cardの能力、skill、認証scheme、streaming宣言は説明情報であり、Task実行資格、Permission、Approval、Credential実値を生成しない。送信本文はnative確認と一回作用のためだけに扱い、Audit、receipt、結果projectionへ複製しない。外部応答のTask／Message／Artifact raw本文は保存・表示しない。

A2A接続一覧は通常IPCだけが参照できる。Owner操作channelでの一覧、Flutterからの直接network接続、Adapter metadataからの接続、履歴やProfileからの再利用は許可しない。Desktop画面の新規接続およびpending receipt再検証要求は、現在のAgentID・URIを示すRust起動器のnative Owner確認後に既存Brokerへ渡す。接続一覧と表示はBroker metadata projectionだけを使う。

Desktopの`A2A接続センター`は、必須`shell.agent_operation`に含まれ、Agent IDとAgent Card URIの入力、Broker接続一覧、接続metadata表示、一回Message送信を提供する。接続URI実値はnative Owner確認の対象表示後、Flutterの入力欄から消去する。Message送信は接続済みreceiptに対応するAgentとinterfaceを選び、各要求で送信本文全体をnative確認した上でBrokerへ送る。本文は要求完了後に入力欄から消去する。画面は接続をTrustへ昇格させず、Task polling／完了判定、Artifact本文取得、Stream、retryを提供しない。Agent Card由来の表示文字列は未信頼metadataと明示し、行制御・方向制御文字を含むprojectionは表示前に拒否する。

接続要求のCredential refは`credential_id`、`purpose`、`target`、`required`、`status`の固定5 fieldだけを受け付ける。IDは小文字hex 32桁、purposeは`A2A接続`、targetは同一要求のAgent ID、requiredは`false`、statusは`missing`に限る。未知fieldおよび秘密値fieldは拒否する。この参照からCredential保管庫を検索せず、credential実値を解決・注入しない。

## 現行未接続範囲

- 導入済み製品を使った実外部HTTPS endpointへの接続・Message送信証拠、OAuth／API keyなどの認証実行
- Task lifecycle polling／完了判定、Artifact本文取得、Stream購読、cancel、A2A transport sessionの再接続、quarantine
- 外部Agent比較、複数Agent同一Workspace隔離、Handoff
- 実A2A Test Harness、Windows installed productでの接続証拠、長時間運用・障害注入

残る外部transport／Agentでの送信実証、Task lifecycle／完了判定／cancel等、Trust審査、正式配布の制約はrelease blockerまたは後続P7／P12／Final QAの各現行gateで分類する。本書の一回Message送信と手動一回状態照会はProduct Build範囲であり、外部Agentでの成功、Task完了、installed productでの実A2A運用やD4 Pocket全体の完成を主張しない。

## C16補完: 手動一回Task状態照会

Windows／Linux Desktopでは、Message送信結果がTaskである場合に限り、送信で返ったTask IDをvolatile UI stateとして保持し、画面上の明示操作で一回の`GetTask`照会を開始できる。Task IDは接続先・Agent IDとともに現在の操作へ束縛し、Agent切替、新規Message送信、receipt失効／復元、対象interface hash変更で破棄する。起動後に永続復元しない。

照会は通常Broker経路からnative Owner確認を要求する。各回で永続Auditと一回消費Permissionを新規発行し、現在のconnection receiptおよびinterface hashを照合する。要求はA2A v1.0 `GetTask`、`historyLength=0`に限定し、Task ID一致と空履歴を検査する。Task state、context ID、Artifact件数、履歴件数0、response hash以外を投影しない。raw status message、history、Artifact、Credential、Authority fieldは拒否または破棄し、AuditにはTask ID実値・URI・本文を保存せずTask ID hashだけを記録する。

timeout、Broker crash、応答不正、Audit結果確定失敗は成功にせずunknown／SUSPENDへ閉じ、自動retry・pollingをしない。Ownerが画面上で明示的に再照会した場合も、新しいOwner確認とPermissionを要求する。macOS native A2A contractにはこの操作を追加せず、macOS UIは未対応と表示する。本補完は外部Task完了判定、Artifact本文取得、Stream、cancel、installed／external Agent QAを成立させない。
