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

- item: native Windows Setup Doctor product export not recollected
  classification: release_blocker
  registry_id: windows_setup_doctor_smoke
  reason: installer/windows/collect_setup_doctor.ps1はexternal_installer_config_broker_probeに分類されたままであり、formal Setup Doctor product evidenceを満たしてはならない。installed Flutter appはGUI_SHELL_SETUP_DOCTOR_EXPORT_JSONに対応したが、isolated staged runからのnative Windows evidenceは再収集・検証されていない。
  required_action: native Windows上でcollect_installed_smoke.ps1を実行し、installed appにSetup Doctor product exportを書き出させた後、python tooling\windows_release_evidence.pyを通す。
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
- `isolated_config_dir`
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
- `first_run.config_audit`: `directly_measured`、`LIVE_RUNTIME`
- `first_run.installer_authority_boundary`: `static_assertion`、`CONFIG`
- `setup_doctor`: `product_export`、`LIVE_RUNTIME`
- `broker.ipc_restart_crash`: `directly_measured`、`LIVE_RUNTIME`
- `release_runtime_assertions`: `static_assertion`、`CONFIG` / `FIXTURE`

未対応の主張、および未分類のcollector declarationはリリースブロッカーである。

## 収集フロー

~~~powershell
powershell -ExecutionPolicy Bypass -File installer\windows\stage_installed_app.ps1 `
  -FlutterReleaseDir .\apps\desktop_flutter\build\windows\x64\runner\Release `
  -BrokerHelperExe .\native\rust_helper\target\release\gui_shell_rust_helper.exe

$Manifest = Get-Content -Raw "$env:LOCALAPPDATA\GUI-Shell\installed-runs\<run_id>\installed_manifest.json" | ConvertFrom-Json

powershell -ExecutionPolicy Bypass -File installer\windows\collect_broker_smoke.ps1 `
  -BrokerHelperExe $Manifest.broker_exe `
  -StoreDir $Manifest.store_dir `
  -SessionFile $Manifest.broker_session_file `
  -OutputPath (Join-Path $Manifest.evidence_dir "windows_broker_smoke.json")

python tooling\release_runtime_assertions.py --json > release_evidence\release_runtime_assertions.json

powershell -ExecutionPolicy Bypass -File installer\windows\collect_installed_smoke.ps1 `
  -InstalledExe $Manifest.app_exe `
  -InstalledManifestJson (Join-Path $Manifest.install_root "installed_manifest.json") `
  -SetupDoctorJson (Join-Path $Manifest.evidence_dir "setup_doctor_product_export.json") `
  -ConfigPath $Manifest.config_path `
  -AuditDir $Manifest.audit_dir `
  -VisibleSurfacesOutputPath (Join-Path $Manifest.evidence_dir "visible_surfaces.json") `
  -BrokerEvidenceJson (Join-Path $Manifest.evidence_dir "windows_broker_smoke.json") `
  -BrokerHelperExe $Manifest.broker_exe `
  -BrokerStoreDir $Manifest.store_dir `
  -BrokerSessionFile $Manifest.broker_session_file `
  -NoPythonRuntime `
  -RuntimeAssertionsJson release_evidence\release_runtime_assertions.json `
  -OutputPath release_evidence\windows_installed_smoke.json
~~~

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

## 厳格な境界

- 必須surface labelは、Windows UIAutomationを通して観測した実際のFlutter/Dart semanticsまたはaccessibilityから得なければならない。
- `Dashboard`、`NavigationRail`、`Runtime Status`、`Invariant Status`を集約するnative window title、native container name、およびroot accessible nameは禁止する。
- screenshotは補助資料に限る。
- broker smokeが証明するのはIPC/restart/crash behaviorだけである。top-level unmeasured booleanをno-Python/no-FFI proofとして用いることを禁止する。
- `tooling/release_runtime_assertions.py`は`CONFIG/FIXTURE` evidenceである。installed product runtime proofへの昇格を禁止する。

証拠が欠けている状態は引き続き`release_blocker`である。
