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

Flutter 3.44.0のmacOS Runnerは同梱Rust helperを匿名pipe経由で使用する。Flutter／RunnerはBroker資格を読み取らず、Rustが既存認証loopback Brokerへ中継する。App Sandboxを保持し、helperへsandboxを継承する。network.clientとBrokerのloopback listen用network.serverを明示し、JITはDebug/Profileだけ。広域file access、任意helper・command指定、Owner資格読取権限は追加しない。有限接続単位と現行結果は`docs/specs/macos-desktop-channel.md`と`docs/REV5_PRODUCT_PROGRESS.md`を参照する。

開発buildは先に`cargo +1.95.0 build --locked --manifest-path native/rust_helper/Cargo.toml --bin gui_shell_macos_broker`を実行し、その後`flutter build macos --debug`で同梱する。ReleaseはRust側にも`--release`を指定する。Xcodeは同じprofileの固定helperを同梱し、欠損時はbuildを失敗させる。end userがRustやterminalを用意する経路ではない。

- item: macOSの残る製品機能・復旧・正式配布
  classification: release_blocker
  reason: 通常Broker接続・初回設定・正常終了は手動Actions run 37598578513で成立した。Owner専用操作・Credentials・Task・Windows固有製品機能のmacOS移植、異常終了後の起動lock復旧、正式署名・配布は未成立。bundle identifierは開発用である。
  required_action: P13の製品接続から順に不足を実装する。通常接続の成功をmacOS全機能完成へ昇格せず、最終QAと正式identity／配布を別途成立させる。
  blocks_release: yes
