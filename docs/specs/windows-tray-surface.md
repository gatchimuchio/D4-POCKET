# Windows常駐トレイ操作面の責任正本

## 目的

C20は、D4 PocketのWindows日常操作をsystem tray（常駐トレイ）へ限定的に投影する。トレイは表示と操作入口であり、Authority、Permission、Approval、Credential、Runtime実装を所有しない。

## 操作

| 操作 | 経路 | 成立する挙動 |
| --- | --- | --- |
| D4 Pocketを開く | Windowsの常駐トレイ → Flutter | ウィンドウを前面化し、概要面を開く |
| 実行系状態 | Broker由来snapshot → tray projection | 取得不能時は`不明`と表示し、0へ変換しない |
| 保留承認件数 | Broker由来snapshot → tray projection | Broker由来でない値は`不明`と表示する |
| 重大通知件数 | 通常Broker IPCの`通知一覧`（未読） | `重大件数`だけを表示し、本文・監査reasonは表示しない |
| 全Runtime停止要求 | tray → Flutter → 通常Broker IPC | `全Runtime停止要求`のbounded receiptを返す。直接kill、owner承認、実停止は行わない |
| 終了 | Windowsの常駐トレイ → ウィンドウ終了 | 通常のアプリ終了だけを行う。Runtimeへ停止権限を生成しない |

## 境界

- Windows native trayはfilesystem、process、network、credential、Broker secretへアクセスしない。
- trayのMethodChannelは表示射影と固定actionだけを受け付け、受信値は上限付き型検査を行う。
- 全Runtime停止はBrokerが保持するlifecycle対象を列挙してowner再承認待ちreceiptを返すだけであり、成功した停止とは扱わない。
- `停止実行済み=false`、`承認状態=owner_reapproval_required`、`権限生成=なし`を変更しない。
- 重大通知の根拠は`INTERNAL_STATE`の通知summaryであり、通知本文・監査payload・秘密値をtrayへ渡さない。

## 未成立範囲

- `release_blocker`: Windows installed productでのtray icon、前面化、常駐、終了、Broker-mediated stop requestの実機evidence。
- `release_blocker`: 全Runtimeを実際に停止するowner承認・lifecycle実行の統合。C20は要求receiptまでを対象とする。
- `known_limitation`: Windows以外にはnative tray実装を提供しない。
