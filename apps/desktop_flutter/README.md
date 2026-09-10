# デスクトップ Flutter app

予約されたデスクトップ Flutter shell 境界。

classification: required_for_v1
reason: デスクトップ Flutter は交換可能な操作者用 UI 表層であり、UI の責任だけに留める。
blocks_release: no

この app は UI の責任だけに留める。

- 概要画面を描画する
- runtime 状態を表示する
- permission、approval、audit、recovery の各センターを表示する
- 生成済み contract client を呼び出す
- 明示的な境界を通じて adapter/runtime API を呼び出す

権限判定を所有してはならない。


## macOS開発構成

Flutter 3.44.0の標準macOS projectを追加した。App Sandboxを保持し、既存の認証付きloopback brokerへの接続にnetwork.clientを指定する。Debug/Profileのnetwork.serverとJITはFlutter標準の開発用設定であり、Releaseには追加しない。広域file accessやowner資格の読取権限は追加しない。通常資格fileはapp container内に明示配置する必要があり、外部pathを設定するだけではSandbox外の読取を許可しない。

- item: macOS実機のbroker資格配置・起動・配布
  classification: release_blocker
  reason: 標準projectと補助buildは実機のcontainer内資格配置・対話・署名を証明しない。bundle identifierは開発用である。
  required_action: Mac実機で資格配置と実対話を確認し、正式識別子・署名・配布手順を確定する。
  blocks_release: yes
