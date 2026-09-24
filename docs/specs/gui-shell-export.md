# GUI Shell Windows書出しの意味正本

## 目的

GUI Shell Windows書出しは、D4 Pocketの構成Manifestから独立Appの初期Manifestを生成する。現行単位は、書出し先の新規App identity、新規監査store、設定、Runtime／Adapter構成、Capability requirement、配布metadataをBrokerで生成するManifest-only経路である。

## Broker経路

`GUI Shell書出し`要求をRust Brokerで再検証し、`INTERNAL_STATE`のAuditEventと書出しReceiptを返す。対象platformはWindows、出力modeは`manifest_only`に固定する。任意画面の選択はGUI入力にすぎず、Brokerが機械可読なModule一覧を照合し、必須Moduleと依存閉包を再計算する。

現行Desktopの`BrokerClient`は通常資格だけを利用する一方、Brokerの書出し操作はOwner制御資格を要求する。このため現在の設定画面からの要求はBrokerで拒否され、Owner資格をFlutterへ渡す代替経路は作らない。Ownerが明示操作できるBroker統治経路は別途必要であり、現行画面から実Exportできるとは扱わない。

## 継承禁止

書出し元のAuthority、Permission、Approval、Credential、Audit chainは継承しない。書出し先は新規App identityと新規監査storeを持ち、`authority_strip=true`、各`*_inherited=false`、`inheritance_policy`の各値`none`を返す。Capability requirementは必要機能の説明であり、Permissionではない。

## 未成立範囲

`build_status=not_started`、`artifact_status=not_built`、installer未開始、署名なしを固定する。Module計画の`binary_pruning_status=not_applied`も固定し、Manifest上の除外をExport artifactからの削除へ読み替えない。Developer専用Flutter UIのbaseline／選択build比較とAOT report上のsurface library node有無は確認済みだが、独立Export artifactの生成・pruning、安全Core保持、製品起動・資源比較、Installer、署名、配布、rollback、実際のfilesystem書込みは未成立の`release_blocker`として保持する。Manifest ReceiptやDeveloper UI buildだけで独立App完成や製品releaseを主張しない。
