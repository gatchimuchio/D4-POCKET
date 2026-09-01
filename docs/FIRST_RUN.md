# 初回起動

GUI-Shell の初回起動は terminal 優先経路ではなく、環境診断から始める。

環境診断は次の構造化状態を報告する。

- Python の利用可否
- native helper test に必要な Rust の利用可否
- desktop UI 分析または起動に必要な Flutter の利用可否
- runtime 接続の準備状態
- local permission の前提条件
- update policy 契約
- audit storage 契約
- recovery catalog 契約
- adapter の準備状態

Installer と環境診断の state は権限を与えず、permission を承認しない。
