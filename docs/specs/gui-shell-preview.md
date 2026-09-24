# GUI Shell構成Previewの意味正本

## 目的

`GUI Shell構成Preview`は、構成をbuildまたはExportする前に、現在構成と候補構成の差分、機能要件、対象platform、版とrollbackの表示範囲を確認する読み取り専用経路である。

## Broker経路

通常認証済みBroker IPCへ現在Manifest（未設定可）と候補Manifestを渡し、Brokerが両方の構造、識別子、表示範囲、継承禁止を再検証する。Brokerは差分を決定論的に計算し、`INTERNAL_STATE`のAuditEvent付きPreview Receiptを返す。FlutterはPreview結果を表示するだけで、filesystem、process、network、credential、Permission、Approval、rollback操作を直接扱わない。

## 機能要件と権限境界

候補ManifestのCapability requirementは、選択された機能の説明として`not_generated`／`not_requested`へ射影する。PreviewはPermission、Approval、Authority、Credential、Audit chainを生成または継承しない。`authority_strip=true`と継承値`none`をReceiptへ固定する。

## 版とrollback

`preview_mode=version_rollback`を受理するが、現行単位ではrollbackを実行しない。`rollback_available=false`、`rollback_status=not_executable`、`build_status=not_started`、`export_status=not_started`を返し、Preview成功をbuild、Export、rollback成功へ昇格させない。
