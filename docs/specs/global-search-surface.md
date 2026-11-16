# D4 Pocket Search／Command Paletteの責任正本

## 目的

SearchとCommand Paletteは、Rust Brokerから開くたびに取得した現在のHost／Workspaceと、現在有効なWorkspace読取許可だけを検索する。検索結果は画面遷移専用であり、Authority、Permission、Approval、Credential、Runtime実行、外部接続を所有しない。

## 現在検索する内容

- Host：`HostRegistrationClient.refresh()`が`Host一覧`から受け取るmetadata-only projection。表示名、Host ID、Platform、接続状態、Trust状態だけを使う。identity hash、証明書、Runtime／Agent summary、登録Audit IDは検索indexへ入れない。
- Workspace：`WorkspaceClient.list()`が`作業領域一覧`から受け取るWorkspace ID、Runtime ID、表示範囲、承認状態。
- 許可済み操作：Workspace一覧で`approved`かつ取得時点で期限内、表示範囲が`none`以外のWorkspace読取だけを「Workspace読取」として表示する。承認ID、登録hash、root pathは検索indexへ入れない。Task用の一回Permission／ApprovalやCapabilityは一覧化・許可判定に使用しない。

BrokerのHostまたはWorkspace取得が失敗した場合は、Search／Paletteを表示せず、古いSnapshotやlocal fixtureへfallbackしない。Broker取得後に許可が失効しても、結果選択は画面遷移だけで作用を起こさない。結果recordやAuthorityを移動先へ渡さず、移動後に操作者が開始する作用は移動先の既存Broker要求で再評価する。

## 操作とbounded境界

- Searchは`Ctrl+Shift+F`または画面上の`全体検索`、Command Paletteは`Ctrl+K`／`Ctrl+P`または画面上の`コマンドパレット`から開く。
- それぞれを開くたびに、認証済Brokerの`Host一覧`と`作業領域一覧`を読み取る。
- 検索語は128文字まで、indexは最大96件（Host 64、Workspace 16、Workspace読取許可16）、結果は30件まで。
- 結果選択はHost操作面またはAgent Centerへの画面遷移だけを行う。検索語、結果、Palette項目からClipboard、Broker操作、権限要求を実行しない。
- 応答は各Clientが既存のoperation、status、`INTERNAL_STATE`、Audit ID、bounded schemaで検証する。出所未検証の`ShellSnapshot`、固定surface entry、設定・Problem・Recovery・MCP・A2A・Agent・Capability一覧を検索へ混在させない。

## 内容露出と権限境界

登録識別子と状態だけをbounded projectionする。Workspace path、registration hash、approval ID、identity hash、証明書、Credential、Task本文、Approval payload、Audit raw payload、自由記述error、外部file内容を登録しない。

Flutterは表示と画面遷移だけを担い、Broker以外のfilesystem、process、network、credential、Clipboardへ到達しない。Host／Workspace metadata、検索結果、期限内Workspace読取の表示はPermission／Approvalを生成・再利用しない。検索結果はWorkspace readの実行ではない。

## 既知の未成立範囲

- Search／Paletteの全Host・Workspace・Task Grant経路を対象にしたinstalled product横断検索証拠は未成立。
- 現Brokerに汎用のPermission／Grant一覧operationはないため、一覧表示できるGranted Actionは既存Workspace読取許可に限定される。Task実行の一回Permission／ApprovalはSearch／Paletteから検索・実行しない。
- Command Paletteの直接作用、本文検索、検索結果からの操作実行は提供しない。
