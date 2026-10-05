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
→ 起動ごとのPID-bound pipeを準備して固定配置のFlutter executableを起動
→ Flutter RunnerのMethodChannel要求を起動器owned pipeから既存Brokerへrelay
→ Flutter終了後、起動器がprocess内停止通知をBrokerへ送り、endpointを照合削除
```

### 固定installed rootからのBootstrapper起動

製品別起動器binaryがKnown Folder由来の固定root直下にある場合だけ、起動器は`active_version.json`をBootstrapper記録として読み、同一App ID／Audit Store ID、製品版から導出した固定`versions/<version>-<package hash>`、version-local launcherとProduct Manifestのhash、通常package layoutを確認する。path入力やcommand引数は受け付けず、記録・directory・fileをno-followで確認し、hash不一致、reparse、未知field、重複JSON、必要file欠損はfail-closedとする。portable bundleとversion-local起動器は従来どおりsibling配置を使う。

Bootstrapperは環境をOS実行allowlistに絞り、Known Folder APIで得た`LOCALAPPDATA`を明示して、固定version-local起動器だけを起動する。recordは実行権限や配布元trustを生成しない。version-local起動器が既存Rust Brokerを起動し、通常のBroker lifecycle／Audit経路へ進む。有効版recordの生成・切替はstage操作と別のBroker／native Owner操作であり、stage操作のApprovalを流用しない。Broker fixtureはrecord writer、未配置時のroot Bootstrapper配置、atomic record replaceまで接続した。installed productでの次回起動、通常Installer経路、Start Menuおよび`LIVE_RUNTIME`は未成立である。

Flutterは`gui_shell/broker` MethodChannelで要求JSONだけをWindows Runnerへ渡し、Rust起動器が当該Flutter child PIDを照合したPID照合済み名前付きpipe経由で既存の認証付きloopback Brokerへrelayする。Dartからのendpoint file、secret、Socket、networkへの直接アクセスやfallbackはない。別bridge、FFI、privileged IPC、任意command、任意executable、Flutterからのprocess管理を追加しない。Brokerは権限を判断し、起動器はprocess lifecycleと境界付きrelayを管理する。

## 3. 接続情報と権限境界

- 起動器はBrokerのendpoint fileを読み、JSON構造、`127.0.0.1`、非zero port、`normal` role、`authenticated_loopback_tcp`、session識別子、64桁小文字hex資格、最大request長を検証する。
- Owner資格を作成・読み込み・Flutterへ渡さない。起動器はprivileged PermissionやApprovalを生成しない。通常画面の起動にOwner承認は必要なく、起動要求と起動器管理Brokerの終了を既存の永続Audit chainへ`LIVE_RUNTIME`として記録する。Capability／Permissionの範囲は固定Desktop process lifecycleだけで、UI task等のprivileged action承認にはならない。
- 起動lifecycle自体はOwner権限を付与しない。別contractで明示された操作に限り、PID-bound Named Pipe要求をRust起動器がoperation別にparseし、Windows native default-No確認を表示した後、capacity-1 process内channel経由でBroker所有threadへ渡す。Codex Runtime／Workspaceの起動中登録、Agent Task Permission／Approval、GUI Shell書出し、MCP接続・切断・Tool実行、回帰Case登録・削除・削除回復、資格情報失効、更新downloadなどの固定allowlistを使い、任意Owner operationへ拡張しない。Codex登録確認はCLI path・Workspace root・秘密path除外を個別表示し、No／未確認はBrokerへ登録せず拒否する。
- Native確認はWindows session上の明示操作記録であって、Windows account再認証や本人性証明ではない。表示は各operation contractから検証済みfieldだけを射影し、Brokerはsession、時刻、nonce、payload hashおよびoperation固有条件を再検証する。秘密値・Owner資格・privileged IPCをFlutterへ渡さない。
- 起動AuditEventのappendが失敗した場合、Brokerはendpoint準備完了を通知せずFlutterを起動しない。終了AuditEventまたはBroker停止が失敗した場合も正常終了として扱わず、起動器管理Brokerだけを止め、byte列が一致するendpointだけを整理し、durable storeを保持する。
- Flutter childの環境は起動器で消去し、`APPDATA`、`LOCALAPPDATA`、`PROGRAMDATA`、`SYSTEMDRIVE`、`SYSTEMROOT`、`TEMP`、`TMP`、`USERPROFILE`、`WINDIR`だけを引き継ぐ。これに起動ごとのpipe接続先札`GUI_SHELL_BROKER_CHANNEL_PIPE`を追加する。endpoint path、Broker runtime directory、資格情報、任意の親環境変数は渡さず、親processの残りの環境変数を継承しない。
- 開発用snapshot・Setup Doctor export・Semantics exportのenvironment overrideを含め、上記許可list外の環境変数は通常のFlutter起動processから取り除く。
- 受信した引数は受け付けず、起動先は配置root内の固定Flutter executableに限定する。

## 4. 保存・同時起動・終了

- Durable Broker状態とAudit chainは、generic GUI Shellでは`%LOCALAPPDATA%\GUI-Shell\broker\desktop\store`へ、製品identityを埋め込んだD4 Pocket Exportでは`%LOCALAPPDATA%\D4Pocket\apps\<App ID>\stores\<Audit store ID>\store`へ保存し、通常終了時も削除しない。
- C6回帰Case用ProtectedStoreはruntime root直下の`protected`へ接続する。generic GUI Shellでは`%LOCALAPPDATA%\GUI-Shell\broker\desktop\protected`、D4 Pocket Exportでは`%LOCALAPPDATA%\D4Pocket\apps\<App ID>\stores\<Audit store ID>\protected`となる。Broker起動時に通常directoryとして準備し、Windows NTFS、reparse point拒否、Broker store・session fileとの非重複を検証する。検証に失敗すればfail-closedで起動しない。この製品内経路はOwner資格や一般Permissionを作らず、GUI-Shellの任意ProtectedStore設定と併用しない。既存の別path保管物を移動・削除・上書きしない。
- `%LOCALAPPDATA%`配下でruntime identityに対応する階層、Broker `store`、ProtectedStore `protected`を各directory componentごとに確認し、symbolic link／junction等のreparse pointを拒否する。Store類を再帰作成せず、runtime root内の通常directoryであることを確認する。endpointと単一起動lockは同じidentity固有runtime directory内に置き、lockはexclusive file lockで保持して別起動器の同時Store利用を拒否する。
- Rust起動器は、Schema検証済みManifestの`app_identity.app_id`と`audit_store.store_id`を`GUI_SHELL_PRODUCT_APP_ID`／`GUI_SHELL_PRODUCT_AUDIT_STORE_ID`のcompile-time入力として受け取る。両方が未設定なら既存の固定runtime pathを使い、両方が正しい生成形式なら`%LOCALAPPDATA%\D4Pocket\apps\<App ID>\stores\<Audit store ID>`をrootとする。一方だけ、または不正形式ならdirectory作成・Broker起動・Flutter起動より前にfail-closedにする。任意pathは受け取らず、各componentでreparse pointとroot containmentを再検証する。Developer専用の`tooling/export_windows_product.py`はSchema検証済みManifestから両IDをRust Release buildのcompile-time入力へ渡し、Manifest由来IDの組ごとにCargo target directoryを分離する。このbuild補助はruntime Manifest消費やOwner承認済みproduction Export pathではない。
- Windows stagingはD4 Pocket Exportの`product_manifest.json`を受け取り、生成IDを起動器binary内のcompile-time IDと照合してから、製品runtime pathを`CONFIG`としてinstalled manifestへ記録する。Manifestそのものは起動器のruntime inputではない。installed smoke collectorはこのidentity形態に対応する新しいprofile内rootを`LIVE_RUNTIME`で観測し、ManifestのID／hashもevidence bundleへ結合する。旧形式のgeneric-only installed manifestで埋込製品identityを検証したことにしない。
- 埋め込みIDは製品ごとの保存領域partitionだけを選び、信頼、Permission、Approval、Capability、Credentialを生成しない。旧runtimeからのcopy／migration／継承は行わない。現行Developer専用Export build toolは識別子ごとに分離したCargo target directoryを使い、別Exportのcompile-time値を再利用しない。起動器のOwner確認には実際の製品別Export directoryを表示する。
- compile-time identityはFlutter childへ渡さない。通常Flutter起動はこれまでどおりenvironmentをclearし、OS allowlistと起動ごとのpipe札だけを渡す。Manifestをruntimeで読み直してpathやAuthorityを選ぶ経路は設けない。
- 起動前に、この起動器専用の古いendpoint/temp fileだけを型・reparse point・size検査後に整理する。保存Storeや親directoryを再帰削除しない。
- Broker readinessは15秒で上限を設ける。未準備・不正endpoint・UI起動失敗時はfail-closedで起動を中止する。
- UI終了時の停止は同一Rust process内の管理通知であり、Flutterからのshutdown IPCやOwner資格を使わない。Brokerは既存の短いIPC timeoutと最長10msのidle loop周期で停止を確認する。
- endpointは停止後、起動時に読んだbyte列と現在fileが一致するときだけ削除する。差し替わったfileは削除せず、失敗として操作者へ示す。起動器終了時のcleanup失敗を成功へ読み替えない。

## 5. 失敗と復旧

起動不能時はWindowsのGUI error dialogで固定error codeと日本語復旧案内を示す。session secret、endpoint JSON、credential、audit raw payload、filesystem pathをdialogやlogへ含めない。Broker停止中はFlutterをそのまま操作させず、起動器が自身のFlutter childだけを終了する。

## 6. 証拠と未成立範囲

Rust単体試験は固定配置検査、endpoint拒否条件、同一起動lock、Store／ProtectedStore junction拒否、endpoint差し替え時の削除拒否、process内部停止、起動／終了AuditEventを検査する。Windowsでの実起動試験は`LIVE_RUNTIME` evidenceとして対象commit、launcher・Flutter・Broker artifact hash、起動・終了・endpoint cleanup、Audit chainへの両lifecycle記録を別途記録する。Schema／静的走査／unit testだけでinstalled product成立を主張しない。

次は本局所正本の範囲外で、別工程・別証拠が必要:

- item: 正式配布物の導入・削除、取得から導入・起動まで、署名付き更新、以前の版への復旧
  classification: release_blocker
  reason: 本変更はstaged配置からのGUI起動とBroker lifecycle管理だけを扱う。
  required_action: 正式配布identity、署名条件、更新信頼、失敗復旧、導入先実測を含むPhase 34の各contractと実製品検証を完成する。
  blocks_release: yes
- item: Windows installed productの由来・可視画面・Setup Doctor・監査anchor証拠
  classification: release_blocker
  reason: 開発host上のlauncher smokeは分離installed productの正式証拠ではない。
  required_action: 現行release evidence collectorとOwner指定条件を満たす分離Windows検証を実施する。
  blocks_release: yes
