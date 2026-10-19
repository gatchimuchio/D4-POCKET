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

## Linux Brokerへの製品identity

Linux Brokerは`GUI_SHELL_PRODUCT_APP_ID`と`GUI_SHELL_PRODUCT_AUDIT_STORE_ID`のcompile-time値だけを製品identityとして受け取る。両方未設定の開発buildはidentityなしで起動できるが、片方だけ、または`d4-pocket-app-`／`audit-store-`と32桁小文字16進suffixに一致しない値はBroker起動前にfail-closed拒否する。Flutter要求、environment、保存済みstateからidentityを選ばない。これはBrokerへのidentity配線であり、identityを生成・束縛するLinux製品exporter、installed provenance、Audit storeの配布／更新保証が成立した証拠ではない。

Linux MCP接続／切断も同じGTK native Owner往復を使う。接続確認はServer ID、実行file、Workspace、引数hash、Credential ID／対象／環境変数名を示し、CredentialがあればServer processと子孫が読取・外部送信できることを警告する。LinuxはDesktop sandboxを付与しない。BrokerはLinux Secret Serviceの現在record・用途・対象・失効状態・暗号文hashとdurable Auditを照合してから、最小化したenvironmentでstdio childを起動する。childは独立process groupへ置き、明示切断時はそのgroup停止を確認する。group外へ離脱したprocess、Broker crash後のorphan cleanup、外部副作用は保証しない。これらの限界はnative確認に表示し、MCP接続からTrust／Permission／Approvalを生成しない。MCP Tool実行は現在Catalog／Schema、一回Permission、AuditをBrokerが再評価する。

## 作業領域folder選択

Agent Centerの`作業領域OS選択`はversion-only要求だけを受け、GTK Runnerが同じapp windowを親とするlocal-folder chooserを表示する。Rust helperがchallengeを生成し、private pipeで選択応答を照合してから、OS選択専用のLinux Broker sourceへ一回配送する。Flutterから選択pathを注入できず、pathはAudit本文ではなくbounded `INTERNAL_STATE`応答へだけ投影する。取消・失敗は入力を保持し、自動再送しない。選択結果はfilesystem scopeではなく、Workspace登録やPermission／Approvalも作らない。責任正本は[Linux Workspace Selection](linux-workspace-selection.md)、要求／receipt schemaは`specs/linux_workspace_selection*.schema.json`。

実装時の対象検査は、Task Owner候補のscope/hash結合・本文非表示・拒否・一回内部配送を確認するRust test 3件と、challenge一致・応答形状・summary/frame上限を確認するLinux worker test 2件。Ubuntu-24.04 on WSLで`cargo build --locked --offline --release --bin gui_shell_linux_broker --features linux-desktop-broker`とFlutter 3.44.0 Linux Release bundle buildは成功し、同梱Runner／helperのELF依存解決も成功した。これはLinux native GUI上の実操作・installed product・X11／Wayland個別受入れを示さない。

2026-10-19の追加施工ではLinux MCP接続／切断をGTK native Owner allowlistとBroker再評価へ接続し、Credentialを設定した接続は一致するSecret Service recordをBrokerが復号してstdio childの指定environmentだけへ渡す。Linux childは独立process groupで起動し、明示切断でgroup停止を確認する。Ubuntu-24.04 on WSLの隔離source snapshotで`adapters::process_tree::unix_tests::unix_owned_group_stops_root_and_inherited_child`（1件）、`desktop_mcp`（native Owner境界／表示summary 2件）、Secret Service暗号包絡（1件）、隔離DBus／unlocked synthetic Secret Serviceを使う`linux_secret_service_actual_store_is_create_only_namespaced_and_cleaned`（1件）がPASSした。実Credential値は合成のみ。Linux Credential画面のFlutter test 2件もPASS。これはnative Owner UI操作やLinux installed product経路の受入れではない。Linux MCPにDesktop sandboxはなく、process group外への離脱とBroker crash後のcleanupは保証しない。

## 未接続のLinux製品面

通常Broker transport、GTK native Owner確認、Linux Secret Service資格情報登録／MCP結合、native Workspace chooserのコード接続だけでLinux完全対応を主張しない。RustにはXDG_DATA_HOMEからversioned install rootを固定導出する基盤があるが、Broker package staging／activation、desktop entry、更新／Rollback／Uninstallerへ未接続である。Linux native installed chooserのX11／Wayland実操作、Agent Taskのprocess監督とscratch／descendant cleanup・cancel／deadline／crash Recovery、Installer／更新／Rollback／Uninstaller、配布検証は未成立範囲として残す。資格情報入力・保管のfallbackはなく、Secret Serviceが利用不能・lockedなら失敗を維持する。未接続のOwner operationはBrokerのfail-closed拒否を維持する。

通常起動では`task_execution=unsupported`を維持する。Linuxでのprocess descendant停止、scratch cleanup、Recoveryおよびdurable Auditが既存有限受入条件を満たすまで、WindowsのTask能力証拠をLinuxへ転用しない。
