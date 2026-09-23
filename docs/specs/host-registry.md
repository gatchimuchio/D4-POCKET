# C17 複数Host registry

## 対象

C17は、D4 Pocketが複数のHostを識別し、HostごとのRuntime／Agent構成を将来の操作面へ渡せるようにするmetadata registryの単位である。現行単位ではHostの登録と一覧だけを実装する。Host切替、Runtime／Agent列挙、Host間操作はC18以降の責任であり、ここへ混在させない。

## 実行経路

```text
owner control
  → Host登録
  → Rust Security Broker
  → Host receiptのbounded metadata化
  → hosts.jsonへのatomic永続化

通常認証済みIPC
  → Host一覧
  → Broker内部registry
  → INTERNAL_STATE metadata-only一覧
```

Host登録はowner controlだけを受け付け、通常IPCからの登録を拒否する。Host一覧は通常IPCだけを受け付け、owner controlからの一覧を拒否する。登録と一覧の受信・受理結果は永続Auditへ記録する。registryは64件、state fileは8MiBにboundedし、state fileのJSON重複field、未知field、重複Host ID、上限超過はBroker起動時にfail-closedで拒否する。

## Host情報と証拠

最低限の登録情報はHost ID、表示名、Platform、接続状態、Trust、証明書／identity、Runtime summary、最終接続である。証明書やidentityは`sha256:` hashだけを保持し、証明書・秘密鍵・接続先実値は受け付けない。Runtime／Agent countはownerが登録したbounded metadataであり、現在のRuntimeやAgentのlive列挙を意味しないため、`INTERNAL_STATE`として表示する。接続状態とTrustは登録時に必ず`pending_review`とし、owner metadataだけでconnected／verifiedへ昇格させない。

`LIVE_RUNTIME`の観測値を必要とする接続確認は別単位で実装する。現在のreceiptは`INTERNAL_STATE`、`公開範囲=metadata_only`、`authority_strip=true`、`権限生成=なし`に固定する。

## Authority境界

- Host ID、identity hash、Platform、Runtime／Agent countはPermissionではない
- Host登録のowner controlは登録操作の資格であり、Host自体へPermissionを付与しない
- Host AのApproval、Permission、Credential、AuthorityをHost Bへ共有・転用しない
- Host metadata、Device Link metadata、Agent metadata、Profile、HistoryからAuthorityを生成しない
- 過去stateの再読込でApproval、Permission、Trust verified、接続実行資格を再利用しない
- `permission_id`、`approval_id`、`authority`、Credential実値などのfieldは登録payloadと永続receiptへ入れない

receiptには契約上の`能力ID`と`権限ID`を記載するが、これは監査・Recoveryの対応識別子であり、実Permissionの発行を意味しない。`権限生成=なし`を同時に固定し、Host registryがAuthority sourceにならないことをConformanceとRust negative testで検査する。

## 現行未接続範囲

- Host切替、HostごとのRuntime一覧、Agent一覧、接続状態のlive再確認、Problem表示
- Device Linkとの実Host認証・証明書再検証・失効・quarantine
- Host間Workspace分離、複数Agent比較、Handoff、Resource／Usageのlive観測
- DesktopのHost操作画面

上記は`release_blocker`として保持する。C17はbounded Host metadata registryとowner／通常IPCの責任分離を成立させる単位であり、これだけで複数Hostの実接続、Host切替、D4 Pocket製品完成、正式releaseを主張しない。
