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

## Windows画面要素の自動認識

Windowsの起動処理では `flutter::AccessibilityMode::IAccessibleEx` を明示選択する。Flutter Windows Embedderの[公開API](https://api.flutter.dev/windows-embedder/dart__project_8h_source.html)はこの方式を実験段階として説明している。検証したFlutter 3.44.0の既定設定では、自動認識できた要素はwindow枠と `FLUTTERVIEW` だけだったが、明示選択後は `Dashboard`、`NavigationRail`、`Runtime Status`、`Invariant Status` の4画面要素を個別に認識し、通知領域のメニューから通常終了できた。

Dartの画面最上位で [`SemanticsBinding.instance.ensureSemantics()`](https://api.flutter.dev/flutter/semantics/SemanticsBinding/ensureSemantics.html) を常時保持しない。今回のWindows実測では、この呼出しを残した試行で画面要素は現れず、OSからの要求に任せる構成では個別要素が公開された。これは現行のFlutter実行時における実動観測であり、原因を他のFlutter版にも一般化しない。

実験段階のAPIを使うため、Flutter 3.44.0以外の実行環境と実際の画面読み上げソフトとの互換性は未検証である。別Windows利用者での正式なSetup Doctor証拠と厳格なrelease証拠検査も必要であり、この設定だけでは製品公開の条件を満たさない。


## macOS開発構成

Flutter 3.44.0の標準macOS projectを追加した。App Sandboxを保持し、既存の認証付きloopback brokerへの接続にnetwork.clientを指定する。Debug/Profileのnetwork.serverとJITはFlutter標準の開発用設定であり、Releaseには追加しない。広域file accessやowner資格の読取権限は追加しない。通常資格fileはapp container内に明示配置する必要があり、外部pathを設定するだけではSandbox外の読取を許可しない。

- item: macOS実機のbroker資格配置・起動・配布
  classification: release_blocker
  reason: 標準projectと補助buildは実機のcontainer内資格配置・対話・署名を証明しない。bundle identifierは開発用である。
  required_action: Mac実機で資格配置と実対話を確認し、正式識別子・署名・配布手順を確定する。
  blocks_release: yes
