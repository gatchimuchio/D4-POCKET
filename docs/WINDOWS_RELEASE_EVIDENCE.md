# Windows Release Evidence

Windows-first release validation uses isolated, machine-readable installed-path evidence.

## R2 Trust Reset

- item: historical Windows staged-install PASS
  classification: release_blocker
  historical: true
  superseded_by: windows_evidence_provenance_isolation
  reason: The historical Windows PASS is retained only as owner-trial history. It is invalid for current strict R2 proof because the old run depended on aggregate native surface exposure and was not bound to the exact source commit, isolated install root, and evidence bundle hash required now.
  required_action: Recollect native Windows evidence with the current isolated evidence contract before any completed product release claim.
  blocks_release: yes

- item: native Windows Setup Doctor product export not recollected
  classification: release_blocker
  registry_id: windows_setup_doctor_smoke
  reason: `installer/windows/collect_setup_doctor.ps1` remains classified as `external_installer_config_broker_probe` and must not satisfy formal Setup Doctor product evidence. The installed Flutter app now supports `GUI_SHELL_SETUP_DOCTOR_EXPORT_JSON`, but native Windows evidence has not been recollected and validated from an isolated staged run.
  required_action: Run `collect_installed_smoke.ps1` on native Windows so the installed app writes the Setup Doctor product export, then pass `python tooling\windows_release_evidence.py`.
  blocks_release: yes

## Required Evidence File

```text
release_evidence/windows_installed_smoke.json
```

The file must be generated on a native Windows host from one isolated staged run. The run must include:

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
- per-file evidence bundle hashes
- `field_provenance` for every formal evidence group

`%LOCALAPPDATA%\GUI-Shell\installed` is a legacy shared path and is invalid for formal R2 evidence. Use `stage_installed_app.ps1` without `-InstallRoot` to create a unique `%LOCALAPPDATA%\GUI-Shell\installed-runs\<run_id>` root.

## Evidence Classes

Formal evidence groups must classify their evidence source:

- `artifact`: `directly_measured`, `EXTERNAL_EVIDENCE`
- `first_run.process`: `directly_measured`, `LIVE_RUNTIME`
- `first_run.visible_surfaces`: `directly_measured`, `LIVE_RUNTIME`
- `first_run.config_audit`: `directly_measured`, `LIVE_RUNTIME`
- `first_run.installer_authority_boundary`: `static_assertion`, `CONFIG`
- `setup_doctor`: `product_export`, `LIVE_RUNTIME`
- `broker.ipc_restart_crash`: `directly_measured`, `LIVE_RUNTIME`
- `release_runtime_assertions`: `static_assertion`, `CONFIG` / `FIXTURE`

Unsupported claims and unclassified collector declarations are release blockers.

## Collection Flow

```powershell
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
```

Validate:

```powershell
python tooling\windows_release_evidence.py
python tooling\validate_all.py --strict-release --desktop-platform=windows
```

## Windows Diagnostic Mode

`collect_installed_smoke.ps1` always stores `visible_surfaces_evidence.diagnostic_tree` with UIAutomation element projection:

- `Name`
- `AutomationId`
- `ControlType`
- `ClassName`
- `FrameworkId`
- runtime id
- parent runtime id
- supported patterns
- parent/child edges

`-DiagnosticOnly` may be used for a cause-observation run. Diagnostic-only output is useful for failure analysis, but its `first_run.status=diagnostic_only` is a release blocker until a formal run passes.

## Strict Boundaries

- Required surface labels must come from actual Flutter/Dart semantics or accessibility observed through Windows UIAutomation.
- Native window titles, native container names, and root accessible names that aggregate `Dashboard`, `NavigationRail`, `Runtime Status`, and `Invariant Status` are forbidden.
- Screenshots are supporting material only.
- Broker smoke proves IPC/restart/crash behavior only. Top-level unmeasured booleans are forbidden as no-Python/no-FFI proof.
- `tooling/release_runtime_assertions.py` is CONFIG/FIXTURE evidence. Promotion to installed product runtime proof is forbidden.

Missing evidence remains a `release_blocker`.
