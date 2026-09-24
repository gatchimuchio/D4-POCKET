# GUI Shell構成の意味正本

## 目的

GUI Shell構成は、D4 Pocketで選択したRuntime、Agent、Tool、MCP、Theme、Capability、Settingsを、独立Appのbuild前にManifestへまとめる。現行単位の出力はManifestだけであり、構成要求をbuild成功、独立App identity、製品配布、権限付与へ昇格させない。

## Broker経路

通常認証済みBroker IPCの`GUI Shell構成`へManifestを渡し、BrokerがSchema相当の構造、重複、識別子、表示範囲、継承禁止を再検証する。受理時は`INTERNAL_STATE`のAuditEventを作成し、`gui_shell_compose_receipt`を返す。FlutterはBrokerを呼び出すだけで、filesystem、process、network、credential、Permission、Approvalを直接扱わない。

## 継承禁止

構成ManifestはAuthority、Permission、Approval、Credential、Audit chainを継承しない。`inheritance_policy`の各値は`none`に固定する。Capability requirementは選択要求の説明であり、PermissionまたはAuthorityではない。

## 現行の非対応範囲

`build_status=not_started`、`app_identity_status=not_generated`、`output_mode=manifest_only`を必須とする。Windows Exportは別の`GUI Shell書出し`経路で新規identityと監査storeのManifestを生成するが、実artifact、Installer、署名、Module Pruning、Distributionは開始しない。構成の差分と版rollbackの可否は、別の`GUI Shell構成Preview`経路で表示する。
