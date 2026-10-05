# Adapter管理操作の責任正本

## 目的

C19は、Runtime CenterのAdapter catalog（接続部品台帳）を表示専用のmetadata（メタデータ）から、Brokerが管理するbounded（上限付き）な操作面へ拡張する。これは外部artifact（成果物）のdownload（取得）、filesystem（ファイル領域）への導入、process（プロセス）の起動を完了したことを意味しない。

## 実行経路（Production path）

```text
Desktop Runtime Center
  → 通常認証済みBroker IPC
  → Rust Adapter Center
  → adapters.json（bounded・atomic state）
  → metadata-only receipt
```

`アダプター一覧`は通常IPCで参照できる。`アダプター導入`、`アダプター検証`、`アダプター有効化`、`アダプター無効化`、`アダプター隔離`、`アダプター更新`、`アダプター削除`はowner controlだけが状態を変更できる。通常IPCからの変更要求は`owner_reapproval_required`の`Suspended`として監査され、状態を変更しない。

### Desktop既存record操作のnative Owner経路

Desktop Runtime Centerからの`検証`、`有効化`、`無効化`、`隔離`、`削除`は、通常IPCだけでは状態変更しない。Windows Flutter RunnerのPID結合named pipeからRust Desktop起動器へ要求を送り、起動器が操作、Adapter ID、現在hash、payload hashをnative Owner dialogに表示する。承認後だけ、同じ要求をBrokerのOwner操作queueへ渡す。Brokerは処理直前に現在のrecord、hash、状態条件を再照合し、結果をAuditへ記録する。Windows Runnerの確認応答待ち時間はこの5操作に限って延長する。

native dialogの拒否は状態を変更せず、通常Broker経路の`Suspended` receiptとAuditで終わる。承認はOwner明示操作の記録であり、Permission、Approval、Credential、Trustを生成しない。5操作が変更するのはBroker内catalogだけで、外部file、process、既存process、Runtime実行状態には作用しない。Manifestの`導入`と`更新`は、次節の別のDesktop接続単位で扱う。

### Desktop Manifest導入・更新のnative Owner経路

Runtime Centerは、利用者が提示したManifest JSONを上限48 KiBで既存のBroker transportへ渡す。FlutterはJSON objectと要求の大きさを確認するだけで、署名検証、Trust、Permission、Approvalを決定しない。`アダプター導入`と`アダプター更新`はPID結合named pipeを経てRust Desktop起動器のnative Owner dialogへ進み、確認後だけBroker Owner操作queueへ渡る。通常IPCから直接送られた同操作は`owner_reapproval_required`で停止する。

native確認画面は、Adapter／Runtime ID、申告発行者、source、version、transport、Content Exposure、互換性、署名者fingerprint、全要求Capability、全許可差分、全既知リスク、署名対象byte長とhash、signature hash、要求全体hashを表示する。更新では更新対象の現在Adapter hashも表示する。署名対象byteとsignatureの実値は表示しない。申告値は未信頼として示し、Owner確認が署名検証、発行者Trust、Permission、Approval、Authorityを成立・付与しないこと、署名検証が別Broker操作であることを明示する。

導入・更新が変更するのはBroker catalog metadataとAuditだけであり、外部download、filesystem書込・削除、process起動は行わない。更新ManifestのAdapter IDは指定対象と一致しなければならない。Brokerは処理時に現在record hashを再照合し、古いhash、有効化中、隔離済みのrecordを拒否する。署名状態が未検証・未設定の登録は可能だが、Broker署名検証に成功するまで有効化できない。拒否は状態を変更せず、Auditを伴う。

## 検証と状態

導入・更新は、native Owner確認後にownerが提示したManifestをBroker内部catalogへ登録するmetadata操作である。署名、発行者、source、version、hash、要求Capability、許可差分、既知の危険、互換性、Content Exposureを保存する。署名の真偽はManifestやUIの申告ではなく、Broker所有のEd25519 trustと正本byteで別途検証する。検証済みでないAdapterは有効化できない。

隔離済みAdapterのRuntime IDは、実行系登録、lifecycle、資源観測、対話操作から再利用しない。削除はcatalog recordの除去だけであり、外部artifactのfilesystem削除を行わない。したがって外部artifactの実導入・実削除・process管理は本単位の未接続範囲である。

## Authority・内容露出境界

Adapter metadata、署名済みManifest、過去状態、hash、一覧、resource観測からPermission、Approval、Authority、Credentialを生成しない。receiptは`metadata_only`、`INTERNAL_STATE`、`権限生成=なし`、`authority_strip=true`に固定する。秘密値、raw payload、credential実値、filesystem path、process引数はcatalogまたはFlutterへ投影しない。

## 残存分類

- item: 外部artifactの実download・filesystem導入・process起動・実削除
  classification: release_blocker
  reason: C19ではcatalogの登録・検証・状態制御までを接続し、外部作用を未接続のまま維持する。
  required_action: Capability、Permission、Approval、AuditEvent、RecoveryActionに対応するBroker経路とWindows実証を追加する。
  blocks_release: yes

- item: Windows installed productでのAdapter管理実証
  classification: release_blocker
  reason: 現行のWindows installed evidence関門が未成立である。
  required_action: owner操作を含む実機証拠を保存し、release gateへ接続する。
  blocks_release: yes
