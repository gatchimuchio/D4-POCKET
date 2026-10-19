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

## Owner確認の限定往復

GTK RunnerはRust helperから`D4OWNER1 <challenge> <summary-hex>`のbounded control frameを受けた場合だけ、同じDesktop windowを親とするGTK Yes／No確認を表示する。summaryはRust側のoperation-specific candidate生成器が作った最大16 KiBの表示専用文章であり、Task本文やCredential実値は含めない。RunnerはUTF-8、frame長、challenge形状、制御文字を検査し、確認画面の初期選択を拒否へ固定する。確認待ちには300秒の期限を設ける。

Runnerは同じprivate helper pipeへ、受信したchallengeに一致する一回限りの`D4OWNER1 <challenge> YES|NO`だけを返す。Flutter MethodChannelから確認bool、challenge応答、Owner資格を送るAPIはない。確認summaryと確認応答はDart、Broker request metadata、log、traceへ渡さない。拒否・閉じる・期限切れ・親window不在・不正frameは確認成立にせず、通常資格の既存Broker要求はBrokerの拒否・監査へ進む。壊れたpipeや不正control frameはtransportを停止し、自動再送・alternate authority経路を作らない。

Rust helperは、現在時刻・`desktop_flutter` client・payload hash・operation固有payloadを確認したallowlist要求だけを内部`DesktopOwnerOperationRequest`へ一回配送する。Brokerは現在状態と要求を再評価して最終判断し、拒否やAudit失敗を成功へ昇格しない。GTKでの肯定応答は操作者の一回のgestureに過ぎず、Permission／Approval／Trust／Credentialを単独生成せず、要求間で再利用できない。現在未対応のOwner operationはnormal Broker経路でfail-closed拒否される。

Agent TaskのOwner操作として、Workspace Permission発行、Task本文hash／実行条件hashへのApproval発行、完了結果表示Approvalを確認allowlistへ接続する。Permission確認にはRuntime／Session／Workspaceと限定scopeを、Task Approval確認にはRuntime／Session／Workspace、指示文字数と指示hashを、結果表示確認にはTask ID・結果hash・visibilityを表示する。Task本文・結果本文・Credential実値はRust確認文やRunner transportへ載せない。Brokerは操作時点のSession／Permission／Approval条件を再評価する。

実装時の対象検査は、Task Owner候補のscope/hash結合・本文非表示・拒否・一回内部配送を確認するRust test 3件と、challenge一致・応答形状・summary/frame上限を確認するLinux worker test 2件。Ubuntu-24.04 on WSLで`cargo build --locked --offline --release --bin gui_shell_linux_broker --features linux-desktop-broker`とFlutter 3.44.0 Linux Release bundle buildは成功し、同梱Runner／helperのELF依存解決も成功した。これはLinux native GUI上の実操作・installed product・X11／Wayland個別受入れを示さない。

## 未接続のLinux製品面

通常Broker transportとこの限定Owner確認経路だけでLinux完全対応を主張しない。秘密をDart／platform channelへ渡さないCredential入力・保管、WorkspaceのOS選択、Agent process監督とcrash／deadline Recovery、Installer／更新／Rollback／Uninstaller、X11／Wayland別のinstalled run／配布検証は別の未成立範囲として残す。未実装のOwner operationはBrokerのfail-closed拒否を維持する。

通常起動では`task_execution=unsupported`を維持する。Linuxでのprocess descendant停止、scratch cleanup、Recoveryおよびdurable Auditが既存有限受入条件を満たすまで、WindowsのTask能力証拠をLinuxへ転用しない。
