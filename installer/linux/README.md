# Linux nativeデスクトップ用インストーラー

Linux native DesktopはD4 Pocketの完全対応が必須の製品対象である。Linux installerをpost-v1、任意platform、release対象外として扱わない。

最初のportable archive検証targetをUbuntu 24.04 LTS／x86_64の`tar.gz`とする。ファイルマネージャーで展開し、同梱launcherから初回画面を開く方式で、通常利用者にPython、Rust、Flutter、Git、clone、terminalを要求しない。GTK 3、GLib 2、ログイン中のD-Bus sessionが必要で、Credential機能にはSecret Service互換の資格情報保管サービスが必要となる。起動後の製品導入・更新・rollback・uninstallは既存Brokerのnative Owner確認を通す。

portable `tar.gz`は初回起動用のunsigned archiveであり、D4PKG01更新payloadや通常のsystem package manager installerとは異なる。署名・Publisher identity・正式な一般配布が成立したことを示さない。Linux packaged installer、導入済み製品、更新・rollback・uninstallの一貫した製品受入れは未完了であり、`release_blockers.registry.json`の`linux_native_product_support`がreleaseを遮断する。WSL／WSLg／headless上のbuildや試験は開発証拠であり、Linux native導入済み製品の受入れへ代用しない。

Ubuntu 24.04 LTS／amd64向けAPT `d4-pocket-setup` bootstrap packageは、同じ初回起動画面を`/opt/d4-pocket/setup/`と固定`.desktop` entryへ配置する。手動Actions run [#258](https://github.com/gatchimuchio/D4-POCKET/actions/runs/41345798027)で、packageのinstall／v2 upgrade／不正candidate拒否／再導入回復／downgrade／再upgrade／purge、Xvfb／X11でのlauncher正常起動・終了、隔離XDG user-data sentinel保持を確認した。packageにmaintainer scriptはなく、APT操作はpackage所有fileに限定される。これはbootstrapのLIVE_RUNTIME受入れに限る。初回画面からBrokerへ要求する製品本体のper-user Install／Update／Rollback／Uninstallは、このrunでは実行していない。実desktop、正式署名／配布、Linux native製品全体の完成も示さない。詳細とhashは[進捗記録](../../docs/REV5_PRODUCT_PROGRESS.md)および[`linux-product-package.md`](../../docs/specs/linux-product-package.md)を参照する。

Linux版Rust launcherは実装済みであり、Flutter Linux runtime bundleとともにbuild／installされる。起動時に固定bundle、compile-time App／Audit identity、active-version record、版別起動器の実行元を照合し、Frontend終了後に所有process groupを停止する。XDG_DATA_HOME（未設定時HOME/.local/share）由来のversioned install rootとBroker identity配線、Linux Product ManifestをD4PKG01にするpackage writer、source／toolchain／artifact hash provenance付きExport builder、Linux Brokerの署名済みpackage stage／有効版切替／固定XDG デスクトップ項目登録／直前の署名済み版へのrollbackが実装され、WSL Linux-target fixture試験で確認されている。Ubuntu 24.04.5手動Actionsでは合成Broker-shaped Release bundle／unsigned package buildと一時root cleanupに加え、run [#39326770256](https://github.com/gatchimuchio/D4-POCKET/actions/runs/39326770256)でrollbackを含むRust tests・all-target compile・XDG／Cargo cleanupを確認した。一方、実際のLinux native 導入済み製品上でのInstaller lifecycle、desktop sessionからの起動、Update／Rollback／Uninstaller、X11／Wayland全体の受入れは未成立である。従ってLinux対応完成やnative 導入済み製品 acceptanceを意味しない。

製品identity付きportable bundleは、固定XDG rootに有効版recordがない初回状態（XDG data root自体がまだ存在しない場合を含む）に限り、rootを作らずUIを起動する。Brokerで有効版recordを作成した後は現画面終了後に導入版launcherへ引き継ぎ、同じlaunch-bound one-shot Uninstall ticket最終化経路を使う。破損record、別rootに残ったversioned launcher、recordと不一致のbundleはfail-closedとする。この経路はLinux native installed desktop sessionでのInstaller／Owner確認／起動一体受入れをまだ示さず、下記のrelease blockerを解除しない。

## 製品要件

- 対象Linux distribution、architecture、desktop session、package形式と依存関係を明示し、その対象で導入・起動・更新・復旧・削除を検証する。
- 通常利用者にPython、Rust、Flutter、Git、clone、terminal、手動port設定、runtime root探索を要求しない。
- installerは明示されたpackage／製品fileだけを配置し、Permission、Approval、Credential、Agent trustを作らない。
- 更新・rollback・uninstallは既存BrokerのOwner確認、Audit、Recovery境界へ接続する。OS package managerを使う場合も、失敗・中断後の状態と復旧経路を検証する。
- `XDG_DATA_HOME`／`XDG_CONFIG_HOME`／`XDG_STATE_HOME`とデスクトップ項目の扱い、権限・所有者・symlink境界、version／artifact provenanceを固定する。
- X11とWaylandのnative installed起動・Owner確認・正常終了を分けて受け入れる。

## 状態

分類: `release_blocker`

Linux nativeの導入、更新、rollback、uninstall、installed runtime、X11／Waylandの製品受入れは未完了。責任正本は`docs/specs/linux-desktop-channel.md`と`release_blockers.registry.json`の`linux_native_product_support`。この文書は作業要求を弱めず、検証済みでないdistribution/package方式を既成事実化しない。
