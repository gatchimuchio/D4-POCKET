# 運用プロファイル

状態: C10 実装済み（現行単位）

運用プロファイルは、Runtime運用構成を再利用するための要求設定である。Profileは次の設定だけを持つ。

- `Runtime`: 接続する実行系の識別子
- `Adapter`: 実行系へ接続する境界の識別子
- `要求Capability`: 必要とする能力の要求一覧
- `Content Exposure`: 内容公開範囲の要求
- `network exposure`: ネットワーク公開範囲の要求
- `resource limit`: 資源上限の要求
- `UI preference`: 画面表示設定の要求

ProfileはAuthority sourceではない。ProfileにCapabilityやfilesystem.writeの要求を記載できても、それは要求設定に留まり、Permission、Approval、Audit上の承認、Credential、Authorityを生成しない。`プロファイル適用要求`は現在のProfile hashを照合して適用意図を監査へ記録するだけで、Runtime操作やPermission ledgerの変更を行わない。

## 本番経路

通常認証済みBroker IPCから、作成、複製、適用要求、削除、export、import、一覧をRust Brokerへ送る。Profile状態は永続Brokerでは`profiles.json`へatomic writeし、起動時に版、件数、重複、Schema形状を再検証する。不正状態はBroker起動を失敗させ、権限作用へ昇格しない。

Flutter設定画面はProfile操作の入力と結果表示だけを担当する。FlutterはProfileを権限として判定せず、filesystem、process、network、Credentialを直接操作しない。

## Content Exposureと秘密情報

Profile Schemaは追加fieldを拒否し、Permission、Approval、Authority、Credential実値、audit identityを含めない。Content Exposureは許可された5値だけを受け付ける。exportはProfile設定のみを返し、秘密値・権限・監査本文を含めない。

## 検証対象

- ProfileのSchema、正常例、authority field混入負例
- 作成、適用要求、再起動後の再読込
- malformed state、重複、stale hash、未知fieldの拒否
- ProfileからPermissionを生成しないこと
