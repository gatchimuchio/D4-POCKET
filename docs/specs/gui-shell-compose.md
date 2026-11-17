# GUI Shell構成の意味正本

## 目的

GUI Shell構成は、D4 Pocketで選択したRuntime、Agent、Tool、MCP、Theme、Capability、Settingsを、独立Appのbuild前にManifestへまとめる。現行単位の出力はManifestだけであり、構成要求をbuild成功、独立App identity、製品配布、権限付与へ昇格させない。

## Broker経路

通常認証済みBroker IPCの`GUI Shell構成Catalog`から現在のRuntime／Agent／Tool／Host候補を取得する。Flutterは候補の形式、重複ID、AgentとRuntimeの対応、Authority非生成条件を検査して表示する。Catalogにはboundedな`snapshot_hash`が含まれ、Compose要求でBrokerが現在Catalogを再計算する。Runtime／Tool／Host状態、認証失効、Host切替等で内容が変わっていれば古いsnapshotを拒否する。

Composeは通常認証済みBroker IPCの`GUI Shell構成`へ既存ManifestとCatalog snapshot hashを渡す。BrokerがManifestの構造、重複、識別子、表示範囲、継承禁止、現在Catalogとの整合を再検証する。受理時は`INTERNAL_STATE`のAuditEventを作成し、`gui_shell_compose_receipt`を返す。FlutterはBrokerを呼び出すだけで、filesystem、process、network、credential、Permission、Approvalを直接扱わない。

Catalogは候補の観測を表し、実行可能性・Trust・Permission・Approvalを保証しない。現在producerに依存関係の識別子を供給するcontractがない候補は`dependencies.status=unknown`として投影し、空の一覧を「依存なし」と解釈しない。依存関係の不存在・循環拒否はI04-D01／D02の未成立条件として残り、dependency producerとresolverの契約が接続するまで成立扱いにしない。

## 継承禁止

構成ManifestはAuthority、Permission、Approval、Credential、Audit chainを継承しない。`inheritance_policy`の各値は`none`に固定する。Capability requirementは選択要求の説明であり、PermissionまたはAuthorityではない。

## Catalog候補の選択

Desktop設定画面はBrokerの現在Catalogに存在するRuntime／Agent／Tool／Host候補から選択する。Runtime IDと候補recordの集合は一致し、Agent型を複数Runtime instanceから選ぶ場合もAdapter identityの競合を拒否する。MCP Toolは登録済み接続、Tool schema hash、Server／Protocol version、認証状態と対応付ける。Host候補にはlocal実行状態と登録済みmetadataを区別して表示する。

未取得の版・ABI・OS・Permission・依存・Scope情報は`unknown`のまま表示し、推測で補わない。`connected`、`ready`、`configured`、Host metadataのいずれも、それだけでusable／trusted／authorizedを意味しない。必須Credentialのmissing／revoked、stale snapshot、重複identity、未登録Runtime／Agent／ToolをBrokerが拒否する。候補の選択自体はMCP Tool実行やPermission生成を行わない。

## UI構成の選択

Desktop設定画面はTheme mode（system／light／dark）、表示密度（compact／comfortable）、内容表示要求（none／hash_only／summary／redacted／full）、Capability requirement IDをManifestへ記録できる。Theme IDは現行`d4-pocket`、Localeは`ja-JP`に固定する。Capability requirementと内容表示要求は構成上の要望にすぎず、Permission、Approval、Authorityまたは実際のContent Exposureを設定しない。実行時の可視性は別途BrokerのContent Exposure Boundaryで判定する。

任意Moduleの選択欄は同じDesktop設定画面の書出し領域に置くが、Compose Manifestとは分離された`GUI Shell書出し`要求へ渡し、Broker ReceiptのModule planに記録する。Compose Manifest自体にModule selectionは含めない。Module selectionは計画であり、buildやbinary pruningの実施を意味しない。

構成Previewは、この設定画面で直近にBrokerが受理したManifestを比較元として使う。まだ受理済みManifestがない場合は比較元を`null`にする。この基準は画面内の一時的な差分入力に限り、保存済み構成、履歴、Approval、復旧元、権限の証拠ではない。

## 現行の非対応範囲

`build_status=not_started`、`app_identity_status=not_generated`、`output_mode=manifest_only`を必須とする。Windows Exportは別の`GUI Shell書出し`経路で新規identityと監査storeのManifestを生成するが、実artifact、Installer、署名、Module Pruning、Distributionは開始しない。構成の差分と版rollbackの可否は、別の`GUI Shell構成Preview`経路で表示する。
