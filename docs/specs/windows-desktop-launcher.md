# Windows Desktop起動管理

状態: Phase 34の起動基盤に対する現行局所正本

## 1. 目的と範囲

通常利用者がterminal、PowerShell、Python、Rust、Flutter、Gitを起動せず、配置済みのDesktopアプリを開始・終了できるWindows起動経路を定める。本書が対象とするのはアプリ起動中のBroker管理であり、Installer、正式配布、更新、rollbackの完成を意味しない。

製品名は`D4 Pocket`、既存Repositoryと実装基盤は`GUI-Shell`のままとする。正式なpackage identity、signing identity、Publisher、配布storeはOwnerが指定するまで設定しない。

## 2. 配置と通常経路

staged Windows配置の固定形:

```text
gui_shell_desktop_launcher.exe
app/gui_shell_desktop.exe
app/flutter_windows.dll
app/data/app.so
broker/gui_shell_rust_helper.exe
```

通常経路:

```text
利用者がDesktop起動器を開く
→ Rust起動器がユーザー保存領域と単一起動lockを確認
→ 既存Rust Brokerを起動器内threadで起動
→ loopback bind・通常資格endpointの生成完了を待つ
→ 固定配置のFlutter executableを通常資格endpoint path付きで起動
→ Flutter終了後、起動器がprocess内停止通知をBrokerへ送り、endpointを照合削除
```

Flutterは既存`BrokerClient`の認証付き`127.0.0.1`接続を使う。新bridge、FFI、privileged IPC、任意command、任意executable、Flutterからのprocess管理を追加しない。Brokerは権限を判断し、起動器はprocess lifecycleだけを管理する。

## 3. 接続情報と権限境界

- 起動器はBrokerのendpoint fileを読み、JSON構造、`127.0.0.1`、非zero port、`normal` role、`authenticated_loopback_tcp`、session識別子、64桁小文字hex資格、最大request長を検証する。
- Owner資格を作成・読み込み・Flutterへ渡さない。起動器はprivileged PermissionやApprovalを生成しない。通常画面の起動にOwner承認は必要なく、起動要求と起動器管理Brokerの終了を既存の永続Audit chainへ`LIVE_RUNTIME`として記録する。Capability／Permissionの範囲は固定Desktop process lifecycleだけで、UI task等のprivileged action承認にはならない。
- 起動AuditEventのappendが失敗した場合、Brokerはendpoint準備完了を通知せずFlutterを起動しない。終了AuditEventまたはBroker停止が失敗した場合も正常終了として扱わず、起動器管理Brokerだけを止め、byte列が一致するendpointだけを整理し、durable storeを保持する。
- Flutterへ渡す環境変数はendpoint file pathとBroker runtime directoryに限る。秘密値そのものをenvironment、command line、UI、snapshot、log、traceへ複製しない。
- 開発用snapshot・Setup Doctor export・Semantics exportのenvironment overrideは、通常のFlutter起動processから取り除く。
- 受信した引数は受け付けず、起動先は配置root内の固定Flutter executableに限定する。

## 4. 保存・同時起動・終了

- Durable Broker状態とAudit chainは`%LOCALAPPDATA%\GUI-Shell\broker\desktop\store`へ保存し、通常終了時も削除しない。
- `%LOCALAPPDATA%`配下の`GUI-Shell`、`broker`、`desktop`、Broker `store`を階層ごとに確認し、symbolic link／junction等のreparse pointを拒否する。Storeを再帰作成せず、runtime root内の通常directoryであることを確認する。endpointと単一起動lockは同じ`desktop` runtime directory内に置き、lockはexclusive file lockで保持して別起動器の同時Store利用を拒否する。
- 起動前に、この起動器専用の古いendpoint/temp fileだけを型・reparse point・size検査後に整理する。保存Storeや親directoryを再帰削除しない。
- Broker readinessは15秒で上限を設ける。未準備・不正endpoint・UI起動失敗時はfail-closedで起動を中止する。
- UI終了時の停止は同一Rust process内の管理通知であり、Flutterからのshutdown IPCやOwner資格を使わない。Brokerは既存の短いIPC timeoutと最長10msのidle loop周期で停止を確認する。
- endpointは停止後、起動時に読んだbyte列と現在fileが一致するときだけ削除する。差し替わったfileは削除せず、失敗として操作者へ示す。起動器終了時のcleanup失敗を成功へ読み替えない。

## 5. 失敗と復旧

起動不能時はWindowsのGUI error dialogで固定error codeと日本語復旧案内を示す。session secret、endpoint JSON、credential、audit raw payload、filesystem pathをdialogやlogへ含めない。Broker停止中はFlutterをそのまま操作させず、起動器が自身のFlutter childだけを終了する。

## 6. 証拠と未成立範囲

Rust単体試験は固定配置検査、endpoint拒否条件、同一起動lock、Store junction拒否、endpoint差し替え時の削除拒否、process内部停止、起動／終了AuditEventを検査する。Windowsでの実起動試験は`LIVE_RUNTIME` evidenceとして対象commit、launcher・Flutter・Broker artifact hash、起動・終了・endpoint cleanup、Audit chainへの両lifecycle記録を別途記録する。Schema／静的走査／unit testだけでinstalled product成立を主張しない。

次は本局所正本の範囲外で、別工程・別証拠が必要:

- item: Installer、Uninstall、Download→Install→Launch、Signed Update、Rollback
  classification: release_blocker
  reason: 本変更はstaged配置からのGUI起動とBroker lifecycle管理だけを扱う。
  required_action: 正式配布identity、署名条件、更新信頼、失敗復旧、導入先実測を含むPhase 34の各contractと実製品検証を完成する。
  blocks_release: yes
- item: Windows installed productの由来・可視画面・Setup Doctor・監査anchor証拠
  classification: release_blocker
  reason: 開発host上のlauncher smokeは分離installed productの正式証拠ではない。
  required_action: 現行release evidence collectorとOwner指定条件を満たす分離Windows検証を実施する。
  blocks_release: yes
