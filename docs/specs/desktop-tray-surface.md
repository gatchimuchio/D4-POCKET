# DesktopネイティブTrayの共通契約

## 目的

Desktop Trayは、Windows／macOS／Linuxのネイティブ操作面からD4 Pocketを開き、Brokerが返す限定状態を表示する。TrayとFlutterはAuthority、Permission、Approval、Credential、Runtime processを所有しない。

## 共通操作

| 操作 | 経路 | 成立する挙動 |
| --- | --- | --- |
| D4 Pocketを開く | OS native menu → Flutter | ウィンドウを前面化し、概要面を開く |
| 状態・件数 | Broker snapshot／通常Broker IPC → bounded tray projection | 取得不能・型不一致・上限超過は`不明`。0へ読み替えない |
| 全Runtime停止要求 | OS native menu → Flutter → 通常Broker IPC | Brokerから要求受付／Owner再承認待ちreceiptを取得。実停止ではない |
| 終了 | OS native menu → 通常アプリ終了経路 | Brokerの既存終了処理を通す。Runtime停止権限を生成しない |

`stop_request`が示すのはBroker要求の入口だけである。`停止実行済み=false`、`承認状態=owner_reapproval_required`、`権限生成=なし`を維持し、Native側からprocessを停止しない。

FlutterはBroker receiptの固定Schema、全Runtime targetのID／状態／再承認条件を検証し、ProblemsへRuntime別内訳を表示する。receiptの共有は同一Desktop process内の揮発状態で、Broker transportとgenerationに束縛する。Broker切替、新規要求、応答不明では破棄する。要求対象はBroker上の全Runtimeであり、個別Host／Taskへ結合しない。

## OS接続

- WindowsはWin32 `Shell_NotifyIconW`と既存`gui_shell/tray` channelを使う。詳細は[Windows Tray責任正本](windows-tray-surface.md)。
- macOSはAppKit `NSStatusItem`のMenuから同じ`gui_shell/tray` channelへ固定Action／bounded projectionだけを接続する。ウィンドウの閉じるボタンはTrayへ隠し、メニューの「終了」は既存AppDelegateのBroker終了処理を呼ぶ。
- LinuxはD-Bus StatusNotifierItem／DBusMenuを使う。StatusNotifierWatcher／Hostの実在時だけTrayを利用可能とし、後着・切断をFlutterへ通知する。Tray Hostが未対応ならアプリは通常ウィンドウとして動作し、Tray用Broker pollingをしない。
- MobileにDesktop Trayを追加しない。

## Authority・露出・寿命

- `gui_shell/tray`は固定の`initialize`、`isWindowVisible`、`publish`とNativeからの`open`／`stop_request`だけを扱う。MethodChannel／D-Bus metadataはAuthorityではない。
- Brokerの状態・通知からはboundedなstatus、件数、severityだけを表示する。Task本文、diff、監査payload、raw error、秘密値、CredentialをNative menuへ渡さない。
- Stop要求の応答／例外に含まれるtechnical detailをUIへそのまま出さず、固定の再確認メッセージを表示する。
- ウィンドウが隠れている間は30秒pollを止め、投影を`不明`へ戻す。再表示後に現在値を再取得してから投影する。Native visibility照会ができない場合もpollしない。
- Linux Hostの切断時はAvailabilityをfalseに戻し、隠れていたウィンドウを復帰する。後着HostでAvailabilityをtrueへ戻し、現在の可視状態を再同期する。
- 表示値はNative境界でも文字長／文字種／数値範囲を検査する。無効値は`不明`とし、stop menuは正しいBooleanがtrueの時だけ有効にする。

## 検証範囲

- Flutter Client試験はAvailability後着・切断、visibility通知、Broker投影を確認する。
- Linuxのnative C++試験はfake D-Bus watcherを使い、Menu、固定Action、Host後着／切断／復帰を確認する。fixture試験はDesktop installed product実証を意味しない。
- Windows／macOS／Linuxの各Native build・installed runはOSごとに別の証拠として扱う。別OSやfixtureのPASSを実機PASSへ昇格しない。
- Windows installed productと各Desktop OSの配布／署名は、それぞれの正式なrelease gateで判定する。
