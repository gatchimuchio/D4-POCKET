# コマンドパレット操作面の責任正本

## 目的

コマンドパレットは全体検索と同じBroker現在状態indexを使うkeyboard操作面である。固定機能commandを列挙するcommand registryではない。検索・選択は画面移動だけを行い、Authority、Permission、Approval、Credential、Runtime作用を所有しない。

## 現在検索する内容

対象・取得元・露出上限は[Search操作面の責任正本](global-search-surface.md)に従う。対象は現在のHost、Workspace、および現在有効なWorkspace読取だけである。Brokerの現Host／Workspace一覧を取得できない場合は、Paletteを表示せず、古いSnapshotや固定commandへfallbackしない。

任意Host操作Moduleが除外されたbuildでは、Host操作面へ遷移するHost結果を候補から除く。Workspace結果と有効なWorkspace読取結果も、buildに含まれる目的画面だけへ遷移できる。

## 操作と境界

- `Ctrl+K`と`Ctrl+P`は同じPaletteを開く。
- Paletteと`Ctrl+Shift+F`の全体検索は、開くたびに同じ現在Broker indexを再取得する。
- 検索語は128文字、indexは最大96件、結果は最大30件にboundedする。
- 選択は現在の結果recordやAuthorityを渡さず、許可された画面へのnavigationだけを行う。対象画面で始める作用は既存のBroker経路で再評価する。
- 固定ページ遷移、通知・問題・復旧のSnapshot検索、Clipboard操作、停止要求などの作用commandはPaletteへ登録しない。
- UI close、検索結果選択、通知表示をRuntime stopとして扱わない。

## 検証

Broker一覧取得の正常・拒否・失効・transport failure、stale結果の不使用、Host／Workspace／Grantのbounded projection、画面遷移だけでBroker actionを起こさないことをDesktop testとConformanceで確認する。これらのtestはinstalled productやLIVE_RUNTIMEの証拠を代替しない。
