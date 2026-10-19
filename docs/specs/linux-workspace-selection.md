# Linux 作業領域のOS選択

## 位置付け

Linux native Desktopの作業領域入力を、Flutterが任意pathをOSへ渡す経路ではなく、GTK Runnerが表示するfolder chooserへ接続する。選択は入力補助だけであり、Workspace登録、filesystem access、Owner承認、Permission、Approval、Agent trustを生成しない。LinuxではmacOS security-scoped accessに相当する保持権限は作らない。

## 要求経路

```text
Agent Center（version-only要求）
  → GTK Runner / Rust helperのprivate stdin/stdout
  → challenge-bound GTK folder chooser
  → private selection reply
  → Rustが生成するLinuxWorkspaceSelection source
  → 既存Brokerの限定Owner-operation receiver
  → pathを含むINTERNAL_STATE投影
```

- Flutter要求は`specs/linux_workspace_selection.schema.json`に従い、`{"version":1}`だけを含む。path、bookmark、Permission、Approval、Credential、authority metadataは受け付けない。
- Rust helperは現在要求のoperation、client、issued-at、payload hash、nonceを検査し、nonceを一回だけ消費する。その要求へ結合した予測不能challengeをGTK Runnerへ送る。
- GTK Runnerは同じD4 Pocket windowを親とするGTK folder chooserを表示し、local directory一つだけを選ぶ。Dart MethodChannelからchooser応答やpathを受け取るAPIはない。
- 選択結果はUTF-8の絶対pathを最大1024 byteに制限し、制御文字を拒否した上でprivate pipe内hex frameとして返す。challenge不一致・不正frameは処理を失敗させ、自動再送やfallbackをしない。
- Rust helperは元要求hashと選択結果を結合した内部要求を作り、BrokerがOS選択専用`LinuxWorkspaceSelection` source、要求hash、現在session、path形状を再評価する。通常IPC、Owner credential、Flutter state、MCP metadataからこのsourceは作れない。
- Brokerは選択または取消をAuditへ記録し、pathを含むINTERNAL_STATE投影を画面へ返す。Audit本文へroot pathを含めない。投影は`scope_lifetime="none"`であり、登録・Permission・Approval生成flagは全てfalse。
- Workspace登録は既存の別操作でpath・filesystem・root identity・secret boundaryを再検査する。選択結果自体は登録、filesystem capability、読み書き可否、Task隔離の証拠ではない。
- 失敗／取消時は入力値を保持する。失敗を選択成功へ変換せず、自動再送もしない。

## 責任範囲

この経路が保証するのはGTK chooserから選ばれたpathのboundedな入力投影までである。対象filesystem適格性、Workspace登録、secret除外、Task permission、Task execution、cancel／deadline／crash Recoveryは各既存Broker contractに従い、選択経路が代行しない。WSL／WSLgでのbuild・fixture・GUI観測はLinux native installed productの受入れではない。Linux nativeのinstalled X11／Wayland実操作と配布・lifecycle確認は`linux_native_product_support` release blockerに残る。
