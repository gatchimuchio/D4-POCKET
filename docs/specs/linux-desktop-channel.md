# Linux Desktop通常Broker接続

## 位置付け

Linux native Desktopは完全対応が必須の製品対象である。本書はFlutter要求を既存Rust Brokerへ接続するLinux固有transport契約を定める。WSL／WSLgのbuildや起動は開発証拠であり、Linux native製品全体の受入れではない。

## 要求経路

```text
Flutter MethodChannel(gui_shell/broker)
  → GTK Runner（要求JSON文字列だけ）
  → 同梱 `libexec/gui_shell_linux_broker`（固定実行file、stdin/stdout pipe）
  → Rustが起動した既存authenticated loopback Broker
```

- Runnerは`/proc/self/exe`から固定bundle相対helper pathを解決する。任意path、environment override、shell command、endpoint探索を受け付けない。
- Runnerがhelperへ渡すenvironmentはHOME、XDG state/runtime、表示session、localeに限定し、credential-likeな親environmentを継承しない。
- transportは一要求一応答のnewline frameとし、要求64 KiB、応答4 MiBを上限とする。Runnerは要求をboundedに直列化し、payload／秘密／Broker資格／endpointをlogへ出さない。
- helperは同一bundleのRunnerと自身の固定配置を検査し、XDG private runtime／state下で既存Brokerだけを起動する。Brokerのsession secretはhelper memory内に限り、Credential／Permission／ApprovalをFlutterへ返さない。
- helperはprivate runtime内の固定lock fileをPOSIX exclusive flockで保持する。helper crash後はOSがlockを解放するため、次回起動で同じlockを取得後に限り、所有者・mode・通常file・link数・sizeを検査して前回のsession／一時fileを回収する。不正なfileや同時起動はfail-closedとし、lock file自体はunlinkしない。
- helperは受信frameの`session_id`をRust所有値へ束縛する。呼出側がsessionを提示した場合は拒否markerをBrokerへ渡し、normal資格はFlutterやRunnerから生成しない。
- EOFはhelperからBrokerへの停止要求となる。正常終了時はAudit付き停止後、同一bytesであるsession fileだけを削除する。要求失敗・frame不整合・helper停止時はtransportをpoisonし、自動再送／別経路fallbackを行わない。次回起動のstale file回収は処理結果やAuthorityを再利用しない。
- Permission、Approval、Owner確認、Credential保管、Audit確定は既存Brokerの責任である。GTK Runnerは承認判断をしない。

## 未接続のLinux製品面

通常Broker transportの接続だけでLinux完全対応を主張しない。Linux native Owner確認、秘密をDart／platform channelへ渡さないCredential入力・保管、WorkspaceのOS選択、Agent process監督とcrash／deadline Recovery、Installer／更新／Rollback／Uninstaller、X11／Wayland別のinstalled run／配布検証は別の未成立範囲として残す。未実装のOwner operationはBrokerのfail-closed拒否を維持する。

通常起動では`task_execution=unsupported`を維持する。Linuxでのprocess descendant停止、scratch cleanup、Recoveryおよびdurable Auditが既存有限受入条件を満たすまで、WindowsのTask能力証拠をLinuxへ転用しない。
