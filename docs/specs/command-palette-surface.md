# コマンドパレット操作面の責任正本

## 目的

C21は、既存のコマンドパレットへD4 Pocketの新機能を明示的な検索語で登録し、keyboard-firstの画面移動を成立させる。パレットは操作面であり、Authority、Permission、Approval、Credential、Runtime実行を所有しない。

## 登録コマンド

| コマンド | 遷移先 | 成立する挙動 |
| --- | --- | --- |
| Runtimeを開く | 実行系センター | Runtimeの表示面を開く |
| Agentを開く | エージェントセンター | Agentの表示面を開く |
| 履歴検索 | 実行履歴 | 履歴の検索面を開く |
| 評価実行 | 評価ラボ | 評価画面を開く。実行は既存のBroker・owner承認経路だけを使う |
| MCP接続 | 設定 | MCP接続の設定面を開く。接続要求を直接実行しない |
| 通知表示 | 通知センター | 通知の表示面を開く |
| 資源監視 | 実行系センター | Broker観測の資源面を開く |
| 資格情報 | 設定 | 資格情報の管理面を開く。秘密値を表示しない |
| 更新確認 | 設定 | 更新センターを開く。download・installを実行しない |
| Host切替 | Host操作面 | Host操作面を開く。切替はBrokerの表示コンテキスト操作で行う |
| 全Runtime停止要求を確認 | 実行系センター | Brokerの承認境界を確認する。直接停止は行わない |

## 境界

- `Ctrl+K` と `Ctrl+P` は同じパレットを開く。
- 検索結果は最大30件にboundedし、表示文言・Broker由来のmetadata・UI状態を検索するだけである。
- パレットからBrokerの通常IPC、owner control、filesystem、process、network、credentialへ直接到達しない。
- 検索結果の選択は画面遷移だけであり、検索結果からPermission、Approval、Authorityを生成しない。
- `評価実行`、`MCP接続`、`Host切替`、`全Runtime停止要求を確認`は画面を開く名称であり、処理の承認や実行を意味しない。

## 未成立範囲

- `release_blocker`: 全機能を実際のRuntime・Agent・MCP・Hostへ接続した製品操作と、Windows installed productでのkeyboard／palette実機evidence。
- `known_limitation`: パレットは現在、各操作面への遷移までを提供し、横断検索のindex作成はC22の対象とする。
