# C24 Mobile投影の意味正本

状態: C24実装単位。MobileはD4 Pocketの補助操作面であり、Authorityの所有者ではない。

## 目的

Desktopの既存Broker契約から、外出先で必要な状態確認と復旧導線だけをMobileへ投影する。Mobileへowner資格、Approval発行権、Credential実値、任意command、MCP Tool実行権を渡さない。

## 実行経路

Mobile Flutter → Device Link TLS → Desktop Rust Broker → 既存の読み取り専用Broker handler → メタデータ / 資源観測 / ライフサイクル投影 → Mobile表示

端末経路で追加した操作は次に限る。

- 実行系ライフサイクル状態
- 実行系資源観測
- 通知一覧
- 全Runtime停止要求（停止受付記録の照会だけで、実停止を行わない）
- 対話履歴閲覧状態
- 対話履歴閲覧（Desktop ownerが発行した現在の履歴閲覧承認の範囲だけ）

各操作は既存Rust Broker handlerへ分岐し、Mobile専用の権限判定、別bridge、別audit storeを作らない。

## 画面対応

| モバイル画面 | 成立範囲 | 境界 |
| --- | --- | --- |
| 通知 | Broker監査イベント由来の要約一覧 | 本文・理由・メタデータを表示せず、既読・破棄を行わない |
| Approval | owner承認が必要であることの表示 | MobileはApprovalを発行・編集・延長・失効しない |
| Runtime状態 | 実行系ライフサイクル状態のbounded projection | 表示成功を実行成功・権限・trustへ昇格しない |
| 資源概要 | 実行系資源観測の実測とunknown値 | 取得不能値を0へ変換せず、観測値は権限を生成しない |
| 履歴 | 現在のowner履歴承認に結合したメタデータ | 本文閲覧、再実行、分岐、承認発行を行わない |
| 接続先 | 保存資格からのHost／HostID／証明書hash表示 | 接続先メタデータからTrustやPermissionを生成しない |
| MCP | Desktop owner管理面のみを示す未観測表示 | Mobileから接続、Tool実行、Credential参照を行わない |
| 緊急停止要求 | Brokerのowner再承認待ち停止受付記録表示 | 実停止、kill、owner承認生成を行わない |
| Recovery（復旧） | 資格再確認、失効確認、再結合の既存導線 | 自動再送、秘密値復元、過去Approval再利用を行わない |

## セキュリティ・内容境界

- Runtime Capability ≠ Permission、Agent request ≠ Approval、MCP metadata ≠ Authorityを維持する。
- Mobileの画面状態、履歴、通知、resource observation、Host metadata、MCP metadataはAuthorityの入力にしない。
- Credential実値、対話本文、Approval payload、Audit raw reason、秘密値をMobile表示・event・error・test artifactへ投影しない。
- Mobileが未接続、Broker拒否、承認期限切れ、観測不能の場合は未接続、未観測、不明として停止する。
- 通信がbackgroundへ移った場合は既存Device Link controllerが通信を停止し、復帰時は資格確認だけを再実行する。

## 現在の未成立範囲

- MCPのlive一覧はowner専用Desktop操作面のためMobileでは未観測とする。
- Mobile実機、Android/iOSの安全保管、TLS実接続、長時間運用、障害注入はこのWindows hostでは未検証である。
- Windows installed productでのMobile連携実証、owner GO、C0-C34全数完成、正式releaseはrelease_blockerである。
- MINIDORAの内部実装はMobileまたはShell Coreへ輸入しない。
