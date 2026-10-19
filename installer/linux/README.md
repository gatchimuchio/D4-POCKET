# Linux native Desktop installer

Linux native DesktopはD4 Pocketの完全対応が必須の製品対象である。Linux installerをpost-v1、任意platform、release対象外として扱わない。

現時点ではLinux packaged installer、installed product、更新・rollback・uninstallの一貫した製品経路は未完成であり、`release_blockers.registry.json`の`linux_native_product_support`がreleaseを遮断する。WSL／WSLg／headless上のbuildや試験は開発証拠であり、Linux native installed productの受入れへ代用しない。

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
