# Windowsリリース証拠

Windows優先のリリース検証では、分離され、機械可読なインストール先証拠を使用する。

## R2信頼リセット

~~~yaml
- item: historical Windows staged-install PASS
  classification: release_blocker
  historical: true
  superseded_by: windows_evidence_provenance_isolation
  reason: 履歴上のWindows PASSは、所有者試行の履歴としてだけ保持する。旧runはaggregate native surface exposureへ依存し、現在必要な正確なsource commit、isolated install root、およびevidence bundle hashへ結び付いていなかったため、現行strict R2 proofには無効である。
  required_action: completed product releaseを主張する前に、現行のisolated evidence contractを用いてnative Windows evidenceを再収集する。
  blocks_release: yes

- item: native Windows Setup Doctor product evidence unresolved
  classification: release_blocker
  registry_id: windows_setup_doctor_smoke
  reason: 直近の実行済みv15 DiagnosticOnly runは必須surface・通常終了・reportとAudit hashの一致を観測したが、stage ownerと同じprofileでstrict evidenceではない。現行sourceのv16 UIAutomation text／topmost-window proofはまだ実UIで実行しておらず、pixel／contrast・screen readerも測っていない。
  required_action: collector v16をclean-source Windows Releaseとstage時と異なるprofileで実行し、Frontend MainWindowへ結合したSetup Doctor text／visible sample、config／report hash、accepted AuditEvent、通常終了、cleanup errorなしを同一bundleへ収集してstrict validatorへ通す。証拠が示さないpixel／contrast・screen reader範囲を合格と主張しない。
  blocks_release: yes
~~~

## 必須証拠ファイル

~~~text
release_evidence/windows_installed_smoke.json
~~~

このファイルは、native Windowsホスト上の1回のisolated staged runから生成しなければならない。そのrunには次を含める。

- `run_id`
- `source_commit`
- `source_worktree_clean=true`
- `source_status_porcelain=""`
- `app_artifact_sha256`
- `broker_artifact_sha256`
- `build_command`
- `build_timestamp`
- `isolated_install_root`
- `isolated_runtime_dir`
- `isolated_store_dir`
- `isolated_localappdata`（起動器runtimeと初回product configを内包する実行user profileの分離領域）
- `isolated_audit_dir`
- `evidence_bundle_sha256`
- ファイルごとのevidence bundle hash
- formal evidence groupごとの`field_provenance`

`%LOCALAPPDATA%\GUI-Shell\installed`は従来の共有pathであり、正式R2証拠には無効である。`-InstallRoot`を指定せずに`stage_installed_app.ps1`を使用し、固有の`%LOCALAPPDATA%\GUI-Shell\installed-runs\<run_id>` rootを作成する。

## 証拠分類

formal evidence groupでは、そのevidence sourceを次のように分類しなければならない。

- `artifact`: `directly_measured`、`EXTERNAL_EVIDENCE`
- `first_run.process`: `directly_measured`、`LIVE_RUNTIME`
- `first_run.visible_surfaces`: `directly_measured`、`LIVE_RUNTIME`
- `first_run.broker_health_request`: `directly_measured`、`LIVE_RUNTIME`（通常資格認証後にBrokerがhealth要求を受理し、Auditへ永続記録したこと。clientの応答受信と呼出し元PIDは証明しない）
- `first_run.config_audit`: `directly_measured`、`LIVE_RUNTIME`
- `first_run.installer_authority_boundary`: `static_assertion`、`CONFIG`
- formal gateの要求: `product_export`、`LIVE_RUNTIME`（現行の外部probeは`external_probe`、`EXTERNAL_EVIDENCE`であり不適合）
- `broker.ipc_restart_crash`: `directly_measured`、`LIVE_RUNTIME`
- `release_runtime_assertions`: `static_assertion`、`CONFIG` / `FIXTURE`

未対応の主張、および未分類のcollector declarationはリリースブロッカーである。

## Broker生成Setup Doctor reportと初回設定（実装状態 2026-09-26）

通常製品UIは認証済みRust Broker IPCで`Setup Doctor報告取得`（payloadは`version: 1`のみ）を要求し、受け取ったreportを表示する。Brokerは実起動時に検証したinstalled package配置、実際のIPv4 loopback bind、永続Audit状態から固定7 checkを生成し、`%LOCALAPPDATA%\GUI-Shell\broker\desktop\store\setup_doctor_report.json`へ最大64 KiB・最新一件だけをatomic replaceする。要求から出力pathを選べず、reportはCapability／Permission／Approvalを生成しない。保存byte列のSHA-256は同operationの`accepted` AuditEvent payload hashへ結合する。保存・Audit失敗時は成功reportを返さない。

Windows collector version 15は通常起動した製品が生成した固定store reportを読み、正確なbyte列、SHA-256、`Setup Doctor報告取得 / accepted / setup_doctor_report_exported / LIVE_RUNTIME` AuditEventを照合する。report bytesとAuditEventをrelease evidenceへ同梱する。初回UI設定についても起動前不在、固定既定値、固定Store fileのSHA-256、`初回設定取得 / accepted / LIVE_RUNTIME` AuditEventのpayload hashを照合し、config fileをbundleへ含める。可視surfaceとtray終了menuはprocessを限定したWindows UI Automation Control Viewから収集し、10,000 node上限・重複runtime ID・個別surfaceの親子／geometryを厳格検証する。上限や重複があるtreeは可視surface evidenceとして受理しない。外部probe、collector推定check、任意JSONを製品reportとしては受け入れない。strict validatorは正本Schema、hash、Audit payload hash、evidence bundle file、固定Store path、check一覧を再検証し、全check `pass`とoperator readabilityを要求する。

2026-09-29、staged Release buildを`-DiagnosticOnly -NoPythonRuntime`で再実行した。v15 collectorは122 nodeのControl Viewを上限未到達で収集し、4つの必須surfaceを実測してsurface validatorが合格した。製品trayの「終了」操作でFlutter画面とRust起動器が通常終了し、起動器exit code 0、endpoint cleanup、cleanup errorなしを確認した。Setup Doctor reportは`pass`、初回設定fileとaccepted AuditEvent hashも一致した。ただし出力は`diagnostic_only`であり、stage userと同じprofileだったため、formal first-run gateはprofile分離とstrict statusの理由で未合格である。Setup Doctor専用画面のoperator readabilityも未検証で、release blockerは保持する。Computer Useの画面／accessibility観測は、上記collectorの正式evidenceに使用していない。

初回UI設定の固定store生成・read-only IPC取得contractはSetup Doctor reportとは別に接続済みであり、report側の`config_created`は同一実行中にinstalled path・固定Store・Schema・Audit hashが一致した場合だけ`pass`へできる。現行ReleaseからSetup Doctor reportの機械可読product evidenceは取得できたが、Setup Doctor専用画面のoperator readabilityと別profileのstrict Windows evidenceは未成立である。従って`windows_setup_doctor_smoke`は`release_blocker`のままで、release-readyを意味しない。

## 収集フロー

> `collect_installed_smoke.ps1`はRust Desktop起動器経由へ変更済みである。正式実行はstageに使ったWindows userとは別のprofileから行い、`-UseCurrentWindowsProfile`を指定する。collectorはそのprofile内にrun固有のLOCALAPPDATAを作って起動器の実Broker runtimeを分離する。manifestはrun固有salt付きuser identity digestだけを保存し、raw SIDを含めない。stage manifestの`runtime/` pathは外部probe scratchであり、製品runtime／config／Auditの証拠として使用しない。Flutter本番起動が最初に要求するhealthについて、通常資格認証後のBroker `accepted / LIVE_RUNTIME` AuditEventをcollectorが数え、event IDを記録する。この記録はBrokerが認証済み要求を受理・記録した証拠であり、Flutterが応答を受け取ったことや呼出し元PIDは証明しない。Setup Doctor reportは同じ実runtime固定storeから読み、accepted Audit payload hashと照合する。別profileでの実収集、初回config生成、operator readabilityは未成立なのでrelease blockerを維持する。

Flutter child processの同定は、起動器PIDを親PIDとして持つ`Win32_Process`観測、起動器と同じWindows session、起動後のprocess作成時刻、実行imageのSHA-256一致を組み合わせる。Windows package virtualizationがWMIの`ExecutablePath`を別のLocalCache表記へ写す場合があるため、path文字列一致だけをimage identityの根拠にしない。終了時cleanupも同じ親PID・時刻・session・hash条件で対象を再確認する。Desktopの`WM_CLOSE`は通常trayへ隠す動作であり終了ではないため、collectorは実tray callbackからnative『終了』menuを開き、UIAutomationで該当processのmenu itemを実行する。強制終了またはcleanup errorがあれば正式smokeを拒否する。evidenceの`first_run.process_identity`はprocess identityのLIVE_RUNTIME観測を保持し、BrokerがFlutter caller PIDを識別したという意味ではない。

収集は二つのWindows user contextで行う。stage ownerはartifactをstageし、Broker単体smokeとruntime assertionsを共有evidence directoryへ出力する。その後、stage時とは別のWindows account/profileでDesktop collectorを起動する。別profileにはrepository collector script・stage済みapp／manifestへのread accessと、共有evidence directoryへのwrite accessが必要である。ACLはownerが限定範囲で準備し、collectorはACLを変更しない。同一userの別PowerShell processではprofile分離条件を満たさない。

~~~powershell
powershell -ExecutionPolicy Bypass -File installer\windows\stage_installed_app.ps1 `
  -FlutterReleaseDir .\apps\desktop_flutter\build\windows\x64\runner\Release `
  -BrokerHelperExe .\native\rust_helper\target\release\gui_shell_rust_helper.exe `
  -DesktopLauncherExe .\native\rust_helper\target\release\gui_shell_desktop_launcher.exe

$Manifest = Get-Content -Raw "<stage-installed-root>\installed_manifest.json" | ConvertFrom-Json
$EvidenceRoot = "<ownerがtest userに必要最小限のACLを設定した共有evidence directory>"
New-Item -ItemType Directory -Path $EvidenceRoot -ErrorAction Stop | Out-Null

powershell -ExecutionPolicy Bypass -File installer\windows\collect_broker_smoke.ps1 `
  -BrokerHelperExe $Manifest.broker_exe `
  -StoreDir $Manifest.store_dir `
  -SessionFile $Manifest.broker_session_file `
  -OutputPath (Join-Path $EvidenceRoot "windows_broker_smoke.json")

python tooling\release_runtime_assertions.py --json | Set-Content -LiteralPath (Join-Path $EvidenceRoot "release_runtime_assertions.json") -Encoding UTF8

# ここでstage時と異なるWindows account/profileに切り替え、別shellで以下を実行する。
$Manifest = Get-Content -Raw "<stage-installed-root>\installed_manifest.json" | ConvertFrom-Json
$EvidenceRoot = "<ownerがtest userに必要最小限のACLを設定した共有evidence directory>"

powershell -ExecutionPolicy Bypass -File installer\windows\collect_installed_smoke.ps1 `
  -InstalledExe $Manifest.app_exe `
  -DesktopLauncherExe $Manifest.launcher_exe `
  -InstalledManifestJson (Join-Path $Manifest.install_root "installed_manifest.json") `
  -BrokerEvidenceJson (Join-Path $EvidenceRoot "windows_broker_smoke.json") `
  -VisibleSurfacesOutputPath (Join-Path $EvidenceRoot "visible_surfaces.json") `
  -UseCurrentWindowsProfile `
  -NoPythonRuntime `
  -RuntimeAssertionsJson (Join-Path $EvidenceRoot "release_runtime_assertions.json") `
  -OutputPath (Join-Path $EvidenceRoot "windows_installed_smoke.json")
~~~

collectorはstage manifestのconfig／audit scratchを読み書きせず、外部Setup Doctor JSONも取り込まない。独立した`collect_setup_doctor.ps1`のprobe結果はproduct evidenceとは別扱いにする。起動器lifecycle Audit、health受理Audit、Setup Doctor accepted Auditは新規profile内の実Storeから収集する。Setup Doctor evidenceには保存済みreportのbase64 byte列とhashを保持し、report hashに一致するaccepted AuditEventだけを関連付ける。endpointのnormal role metadata単独はBroker接続証拠にならず、health受理eventもclient応答受信・PID帰属へ昇格しない。壊れたAudit JSON行は読み飛ばさず収集失敗にする。test userがstage時userとは異なるSIDでない場合、collectorは正式実行を拒否する。出力先とinstalled rootへのACL設定は外部のowner管理作業であり、collectorはACLを変更しない。

次のコマンドで検証する。

~~~powershell
python tooling\windows_release_evidence.py
python tooling\validate_all.py --strict-release --desktop-platform=windows
~~~

## Windows診断モード

`collect_installed_smoke.ps1`は常に、UIAutomation element projectionを`visible_surfaces_evidence.diagnostic_tree`へ次の項目とともに格納する。

- `Name`
- `AutomationId`
- `ControlType`
- `ClassName`
- `FrameworkId`
- `runtime id`（ランタイム識別子）
- `parent runtime id`（親ランタイム識別子）
- `supported pattern`（対応パターン）
- `parent/child edge`（親子間の辺）

原因観測用のrunでは`-DiagnosticOnly`を使用してよい。診断専用出力は失敗分析に有用だが、正式runが合格するまで、その`first_run.status=diagnostic_only`はリリースブロッカーである。

2026-09-26の現行source Windows diagnostic runでは、初回configの生成・Schema妥当性・read-only取得accepted AuditEventと保存hash一致、およびSetup Doctor report取得を観測した。一方、UIAutomation treeは`D4 Pocket` windowと`FLUTTERVIEW` containerの2要素のみで、必須surface labelを一つも認識しなかった。通常終了も成立せず、collectorは強制cleanupとcleanup errorを記録した。これはUI semanticsと終了経路の未解決を示す診断であり、別profile／正式release evidenceへの代替ではない。

## 厳格な境界

- 必須surface labelは、Windows UIAutomationを通して観測した実際のFlutter/Dart semanticsまたはaccessibilityから得なければならない。
- `Dashboard`、`NavigationRail`、`Runtime Status`、`Invariant Status`を集約するnative window title、native container name、およびroot accessible nameは禁止する。
- screenshotは補助資料に限る。
- broker smokeが証明するのはIPC/restart/crash behaviorだけである。top-level unmeasured booleanをno-Python/no-FFI proofとして用いることを禁止する。
- `tooling/release_runtime_assertions.py`は`CONFIG/FIXTURE` evidenceである。installed product runtime proofへの昇格を禁止する。

証拠が欠けている状態は引き続き`release_blocker`である。

## 個別surfaceと観測treeの結合

surface matchは同じ収集結果のelement_keyに結合し、名前、AutomationId、ControlType、ClassName、FrameworkId、root/container分類が観測要素と一致しなければならない。別途生成したmatch宣言だけを受理しない。観測件数、要素識別子、親子edgeも全件検証し、重複、欠落、循環、rootへ到達しない要素を拒否する。

可視候補はrootでもnative containerでもなく、状態を取得できた要素に限る。要素からrootまでis_offscreen=falseと有限な正の矩形を要求し、その全矩形の共通領域が正の面積を持つ場合だけ候補とする。画面外、親の表示領域外、ゼロ面積、座標欠落、NaN/Infinity、状態取得失敗を可視へ昇格しない。UIAutomationの親はRawViewWalkerで取得し、列挙する全要素と同じtreeを用いる。

これはアクセシビリティtree上の表示領域との交差を検証する。別windowによる遮蔽やpixel内容を証明しない。MSAA等の新しい収集経路もこの境界を満たす必要があり、名前だけの登録履歴で代替してはならない。

## 日本語surfaceの対応

surfaceの意味対応は固定する。Dashboard=概要、NavigationRail=ナビゲーション、Runtime Status=実行系状態、Invariant Status=不変条件状態。日本語名は空白正規化後の完全一致（同じ見出しが二度連結された場合を含む）に限り、長い説明文やTab名の部分一致を使用しない。既存の英語labelとDartのgui_shell.surface.*識別子も対応する。元の観測名を英語へ書き換えない。
