# GUI Shell Windows書出しの意味正本

## 目的

GUI Shell Windows書出しは、D4 Pocketの構成Manifestから独立Appの初期Manifestを生成する。現行単位は、書出し先の新規App identity、新規監査store、設定、Runtime／Adapter構成、Capability requirement、配布metadataをBrokerで生成するManifest-only経路である。

## Broker経路

Owner制御資格の`GUI Shell書出し`要求をRust Brokerで再検証し、`INTERNAL_STATE`のAuditEventと書出しReceiptを返す。FlutterはBrokerの応答を表示するだけで、filesystem、process、network、Credential、Permission、Approvalを直接扱わない。対象platformはWindows、出力modeは`manifest_only`に固定する。

## 継承禁止

書出し元のAuthority、Permission、Approval、Credential、Audit chainは継承しない。書出し先は新規App identityと新規監査storeを持ち、`authority_strip=true`、各`*_inherited=false`、`inheritance_policy`の各値`none`を返す。Capability requirementは必要機能の説明であり、Permissionではない。

## 未成立範囲

`build_status=not_started`、`artifact_status=not_built`、installer未開始、署名なしを固定する。実artifact生成、Installer、署名、Module Pruning、配布、rollback、実際のfilesystem書込みはこの単位で開始しない。これらは`release_blocker`として保持し、Manifest Receiptだけで独立App完成や製品releaseを主張しない。
