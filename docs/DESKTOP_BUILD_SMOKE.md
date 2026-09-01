# デスクトップビルド・スモーク検証

状態基準日: 2026-05-25

GUI-Shell v1.0はデスクトップ優先である。所有者がリリース範囲を明示的に変更しない限り、モバイルは`post_v1_scope`のままとする。

## スモーク検証の水準

### 静的解析スモーク

~~~yaml
- command: cd apps/desktop_flutter && flutter analyze
  purpose: デスクトップFlutter UI層に対するDart・Flutter静的解析
  current_status: passed
  classification: required_for_v1
  blocks_release: no
~~~

静的解析スモークは、ネイティブLinuxデスクトップバンドルを生成または起動できることの証明ではない。

### ビルド・スモーク

~~~yaml
- command: cd apps/desktop_flutter && flutter build linux
  purpose: ネイティブLinuxデスクトップバンドルの生成
  current_status: passed
  classification: required_for_v1
  blocks_release: no
  evidence: build/linux/x64/release/bundle/gui_shell_desktop
  required_action: Linuxデスクトッププロジェクトファイルを維持し、リリース候補でflutter build linuxを通す。
~~~

`flutter doctor -v`によれば、現在は次のLinuxデスクトップビルド依存関係が存在する。

- clang（C/C++コンパイラー）: `Ubuntu clang version 21.1.8`
- `cmake: 4.2.3`（ビルド構成）
- `ninja: 1.13.2`（ビルド実行器）
- `pkg-config: 2.5.1`（依存関係検出）

Linuxデスクトップのビルド・スモークは2026-05-25時点で解消済みである。

### 起動スモーク

~~~yaml
- command: cd apps/desktop_flutter && ./build/linux/x64/release/bundle/gui_shell_desktop
  purpose: ビルド済みデスクトップ成果物から最初のウィンドウが起動することの検証
  current_status: passed
  classification: required_for_v1
  blocks_release: no
  evidence: WSLg上で最初のウィンドウが開き、Dashboard、NavigationRail、Runtime Status、Invariant Statusを視認した。
  required_action: リリース候補でLinuxデスクトップ起動スモークを合格状態に保つ。
~~~

起動中、WSLgは端末へlibEGL/MESA警告を出力した。将来のスモーク実行で描画または安定性が失敗しない限り、この警告は`release_blocker`ではなく`known_limitation`である。

## リリースゲート

~~~yaml
- item: Linuxデスクトップのビルド・スモーク
  classification: required_for_v1
  reason: cd apps/desktop_flutter && flutter build linuxは2026-05-25に合格し、build/linux/x64/release/bundle/gui_shell_desktopを生成した。
  required_action: リリース候補でLinuxデスクトップのビルド・スモークを合格状態に保つ。
  blocks_release: no

- item: Linuxデスクトップの起動スモーク
  classification: required_for_v1
  reason: cd apps/desktop_flutter && ./build/linux/x64/release/bundle/gui_shell_desktopは2026-05-25にWSLg上で正常に起動し、最初のウィンドウの証拠を記録した。
  required_action: リリース候補でLinuxデスクトップの起動スモークを合格状態に保つ。
  blocks_release: no

- item: WSLgのlibEGL/MESAグラフィックス警告
  classification: known_limitation
  reason: WSLg起動中に端末へ警告が現れたが、gui_shell_desktopウィンドウの起動やDashboard、NavigationRail、Runtime Status、Invariant Statusの描画を妨げなかった。
  required_action: 描画または安定性が失敗した場合はrelease_blockerへ再分類する。
  blocks_release: no
~~~
