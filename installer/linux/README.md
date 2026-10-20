# Linux native Desktop installer

Linux native DesktopはD4 Pocketの完全対応が必須の製品対象である。Linux installerをpost-v1、任意platform、release対象外として扱わない。

現時点ではLinux packaged installer、installed product、更新・rollback・uninstallの一貫した製品経路は未完成であり、`release_blockers.registry.json`の`linux_native_product_support`がreleaseを遮断する。WSL／WSLg／headless上のbuildや試験は開発証拠であり、Linux native installed productの受入れへ代用しない。

Linux版Rust launcherは実装済みであり、Flutter Linux runtime bundleとともにbuild／installされる。起動時に固定bundle、compile-time App／Audit identity、active-version record、版別起動器の実行元を照合し、Frontend終了後に所有process groupを停止する。XDG_DATA_HOME（未設定時HOME/.local/share）由来のversioned install rootとBroker identity配線、Linux Product ManifestをD4PKG01にするpackage writer、source／toolchain／artifact hash provenance付きExport builder、Linux Brokerの署名済みpackage stage／有効版切替／固定XDG desktop entry登録／直前の署名済み版へのrollbackが実装され、WSL Linux-target fixture試験で確認されている。Ubuntu 24.04.5手動Actionsでは合成Broker-shaped Release bundle／unsigned package buildと一時root cleanupに加え、run [#39326770256](https://github.com/gatchimuchio/D4-POCKET/actions/runs/39326770256)でrollbackを含むRust tests・all-target compile・XDG／Cargo cleanupを確認した。一方、実際のLinux native installed product上でのInstaller lifecycle、desktop sessionからの起動、Update／Rollback／Uninstaller、X11／Wayland全体の受入れは未成立である。従ってLinux対応完成やnative installed product acceptanceを意味しない。

製品identity付きportable bundleは、固定XDG rootに有効版recordがない初回状態（XDG data root自体がまだ存在しない場合を含む）に限り、rootを作らずUIを起動する。Brokerで有効版recordを作成した後は現画面終了後に導入版launcherへ引き継ぎ、同じlaunch-bound one-shot Uninstall ticket最終化経路を使う。破損record、別rootに残ったversioned launcher、recordと不一致のbundleはfail-closedとする。この経路はLinux native installed desktop sessionでのInstaller／Owner確認／起動一体受入れをまだ示さず、下記のrelease blockerを解除しない。

## 製品要件

- 対象Linux distribution、architecture、desktop session、package形式と依存関係を明示し、その対象で導入・起動・更新・復旧・削除を検証する。
- 通常利用者にPython、Rust、Flutter、Git、clone、terminal、手動port設定、runtime root探索を要求しない。
- installerは明示されたpackage／製品fileだけを配置し、Permission、Approval、Credential、Agent trustを作らない。
- 更新・rollback・uninstallは既存BrokerのOwner確認、Audit、Recovery境界へ接続する。OS package managerを使う場合も、失敗・中断後の状態と復旧経路を検証する。
- `XDG_DATA_HOME`／`XDG_CONFIG_HOME`／`XDG_STATE_HOME`とdesktop entryの扱い、権限・所有者・symlink境界、version／artifact provenanceを固定する。
- X11とWaylandのnative installed起動・Owner確認・正常終了を分けて受け入れる。

## 状態

分類: `release_blocker`

Linux nativeの導入、更新、rollback、uninstall、installed runtime、X11／Waylandの製品受入れは未完了。責任正本は`docs/specs/linux-desktop-channel.md`と`release_blockers.registry.json`の`linux_native_product_support`。この文書は作業要求を弱めず、検証済みでないdistribution/package方式を既成事実化しない。
