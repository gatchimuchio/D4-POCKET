# Installer 境界

Installer はファイル、依存関係、launcher、初回起動状態を準備できる。

Installer は次を行ってはならない。

- permission を与える
- action を承認する
- 依存関係の失敗を隠す
- Shell Core policy 評価を迂回する
- audit を迂回する
- recovery 対応を迂回する

権限は Shell Core に留める。runtime 固有の準備状態は adapter 契約の背後に閉じる。
