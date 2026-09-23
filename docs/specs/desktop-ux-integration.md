# D4 Pocket Desktop UX統合の責任正本

## 目的

C23は、増えた操作面を論理グループへ整理し、操作者が責任範囲を理解したまま既存画面へ到達できるようにする。これはNavigationの表示整理であり、画面、Runtime、Authority、Permission、Approval、Credential、Audit、Recoveryの所有責任を変更しない。

## 論理グループ

Desktop Navigationは次の4グループと全体表示を提供する。

- `運用`: 概要、実行系、エージェント、対話、履歴、評価、ホスト能力、通知、観測、Host操作
- `安全`: 信頼、権限、承認、監査、復旧、問題、証拠
- `開発`: 環境診断、追跡情報
- `設定`: 設定
- `すべて`: 既存20画面を現在の順序で表示する初期状態

グループを選ぶとNavigationRailだけを対象範囲へ絞り込む。画面本体は削除せず、コマンドパレット、全体検索、通知、既存の画面遷移からも到達できる。別グループの画面へ遷移した場合は、その画面を含むグループへ表示を移す。

## 本番経路

操作者のグループ選択 → FlutterのNavigation表示状態 → 既存の選択済み画面、というUI経路だけを追加する。既存画面のBroker client、owner control、Approval、Audit、Recovery経路を置き換えない。

## 境界

- グループ名、選択状態、NavigationRailの絞り込みはUI状態であり、Authority、Permission、Approval、Credentialを生成しない。
- グループ選択からfilesystem、process、network、credential、Broker IPCへ直接到達しない。
- 全体表示は既存20画面を保持する。グループ表示は画面の削除、権限の縮小、実行経路の変更を意味しない。
- Accessibility上、グループ選択は`操作グループ選択`という意味ラベルを持ち、画面選択とは別の操作として識別できる。

## 未成立範囲

- `release_blocker`: Windows installed productでの4グループの実画面操作evidence、全C0-C34完成、owner GO。
- `known_limitation`: C23はDesktop Navigationの整理を対象とし、Mobile Navigationの投影はC24で扱う。
