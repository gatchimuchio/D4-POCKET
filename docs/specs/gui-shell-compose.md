# GUI Shell構成の意味正本

## 目的

GUI Shell構成は、D4 Pocketで選択したRuntime、Agent、Tool、MCP、Theme、Capability、Settingsを、独立Appのbuild前にManifestへまとめる。現行単位の出力はManifestだけであり、構成要求をbuild成功、独立App identity、製品配布、権限付与へ昇格させない。

## Broker経路

通常認証済みBroker IPCの`GUI Shell構成`へManifestを渡し、BrokerがSchema相当の構造、重複、識別子、表示範囲、継承禁止を再検証する。受理時は`INTERNAL_STATE`のAuditEventを作成し、`gui_shell_compose_receipt`を返す。FlutterはBrokerを呼び出すだけで、filesystem、process、network、credential、Permission、Approvalを直接扱わない。

## 継承禁止

構成ManifestはAuthority、Permission、Approval、Credential、Audit chainを継承しない。`inheritance_policy`の各値は`none`に固定する。Capability requirementは選択要求の説明であり、PermissionまたはAuthorityではない。

## 参照ID入力

Desktop設定画面のRuntime、Agent、Tool、MCP接続欄は、1行ごとの参照IDをManifestへ記録する入力面である。前後空白と空行は除外する。候補一覧の発見・実在確認・接続確認・Agent trust判定を行わない。IDの存在やmetadataはAuthority、Permission、Approvalを生成しない。重複IDを含む契約検証はBrokerが行う。未対応のRuntime／Agentを実行可能と表示してはならない。

## UI構成の選択

Desktop設定画面はTheme mode（system／light／dark）、表示密度（compact／comfortable）、内容表示要求（none／hash_only／summary／redacted／full）、Capability requirement IDをManifestへ記録できる。Theme IDは現行`d4-pocket`、Localeは`ja-JP`に固定する。Capability requirementと内容表示要求は構成上の要望にすぎず、Permission、Approval、Authorityまたは実際のContent Exposureを設定しない。実行時の可視性は別途BrokerのContent Exposure Boundaryで判定する。

任意Moduleの選択欄は同じDesktop設定画面の書出し領域に置くが、Compose Manifestとは分離された`GUI Shell書出し`要求へ渡し、Broker ReceiptのModule planに記録する。Compose Manifest自体にModule selectionは含めない。Module selectionは計画であり、buildやbinary pruningの実施を意味しない。

構成Previewは、この設定画面で直近にBrokerが受理したManifestを比較元として使う。まだ受理済みManifestがない場合は比較元を`null`にする。この基準は画面内の一時的な差分入力に限り、保存済み構成、履歴、Approval、復旧元、権限の証拠ではない。

## 現行の非対応範囲

`build_status=not_started`、`app_identity_status=not_generated`、`output_mode=manifest_only`を必須とする。Windows Exportは別の`GUI Shell書出し`経路で新規identityと監査storeのManifestを生成するが、実artifact、Installer、署名、Module Pruning、Distributionは開始しない。構成の差分と版rollbackの可否は、別の`GUI Shell構成Preview`経路で表示する。
