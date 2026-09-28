# D4 Pocket 全体検索操作面の責任正本

## 目的

C22は、D4 Pocketの既存操作面を横断して、現在取得できている表示用metadataを読み取り専用で検索する。全体検索は操作面であり、Authority、Permission、Approval、Credential、Runtime実行、外部接続を所有しない。

## 検索対象

検索対象は次の表示用projectionである。

- 実行系（Runtime）
- エージェント（Agent）
- 対話セッション（Session）
- 権限（Permission）
- 承認（Approval）
- 監査（Audit）
- 復旧（Recovery）
- 問題（Problem）
- 証拠（Evidence）
- MCP接続
- A2A接続
- 接続先（Host）
- アダプター（Adapter）
- プロファイル（Profile）
- 評価（Evaluation）
- 通知（Notification）

Runtime、Agent、Session、Permission、Approval、Audit、Recovery、Problem、Evidence、Host、Adapter、Profileは現在の`ShellSnapshot`から取得できたbounded metadataを対象とする。MCP、A2A、Evaluation、Notificationは対象操作面を見つけるための固定surface entryも持つ。固定entryの存在は外部接続、実行、取得成功を意味しない。

## 操作とbounded境界

- `Ctrl+Shift+F`または画面上の`全体検索`から開く。
- 検索語は128文字までとする。
- indexは512件まで、結果は30件までとする。
- 実装上の検索語・結果上限は`GlobalSearchIndex.maxQueryLength`と`GlobalSearchIndex.maxResults`で固定する。
- 選択時は対象画面へ移動するだけで、検索結果から操作要求を発行しない。
- 現在のsnapshotがBroker由来と確認できるときだけ証拠範囲を`INTERNAL_STATE`と表示する。それ以外は`不明`と表示し、成功を推測しない。
- JSON内の`snapshot_source`は自己申告値として扱い、証拠源の判定に使わない。`ShellSnapshot.fromJson`で読み込んだ値は`unverified`とし、製品Broker取得経路が必要な応答を受理した後にだけ`broker`を設定する。この出所表示もAuthorityではない。

## 内容露出と権限境界

検索indexへ登録するのは識別子、状態、分類、画面遷移先、boundedな表示用summaryだけである。対話本文、Approvalのpayload、Auditのraw payload、Credential実値、秘密値、外部fileの内容、未許可のfull content、任意のpathは登録しない。

Flutterの全体検索はBroker IPC、filesystem、process、network、credential、Clipboardへ直接到達しない。検索結果、snapshot、MCP metadata、A2A Agent Card、Profile、History、TelemetryはAuthority、Permission、Approvalを生成または再利用しない。

## 未成立範囲

- `release_blocker`: 実Runtime・Agent・MCP・A2A・Hostの全surfaceから取得したinstalled productでの横断検索実機evidence、clean installed artifactとの結合、owner GO。
- `known_limitation`: 現行C22は現在のShellSnapshotと固定surface entryを対象にする読み取り専用indexであり、外部surfaceのlive再取得、全文検索、本文検索、検索結果からの操作実行を提供しない。
