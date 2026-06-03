# Windows Release Evidence

Windows-first release validation uses machine-readable installed-path evidence.

Required evidence file:

```text
release_evidence/windows_installed_smoke.json
```

Generate it on a native Windows host after installing or staging the Windows release artifact:

```powershell
powershell -ExecutionPolicy Bypass -File installer\windows\collect_broker_smoke.ps1 `
  -BrokerHelperExe "$env:LOCALAPPDATA\GUI-Shell\installed\broker\gui_shell_rust_helper.exe" `
  -OutputPath release_evidence\windows_broker_smoke.json

powershell -ExecutionPolicy Bypass -File installer\windows\collect_setup_doctor.ps1 `
  -InstalledExe "$env:LOCALAPPDATA\GUI-Shell\installed\app\gui_shell_desktop.exe" `
  -ConfigPath "$env:ProgramData\GUI-Shell\config\gui_shell.json" `
  -AuditDir "$env:ProgramData\GUI-Shell\audit" `
  -BrokerEvidenceJson .\release_evidence\windows_broker_smoke.json `
  -OutputPath release_evidence\setup_doctor_installed.json

$RuntimeAssertionsJson = python tooling\release_runtime_assertions.py --json
$Utf8NoBom = New-Object System.Text.UTF8Encoding $false
[System.IO.File]::WriteAllText(
  (Resolve-Path release_evidence).Path + "\release_runtime_assertions.json",
  ($RuntimeAssertionsJson + [Environment]::NewLine),
  $Utf8NoBom
)

python tooling\release_runtime_assertions.py --check
if ($LASTEXITCODE -ne 0) {
  exit $LASTEXITCODE
}

powershell -ExecutionPolicy Bypass -File installer\windows\collect_installed_smoke.ps1 `
  -InstalledExe "$env:LOCALAPPDATA\GUI-Shell\installed\app\gui_shell_desktop.exe" `
  -SetupDoctorJson .\release_evidence\setup_doctor_installed.json `
  -ConfigPath "$env:ProgramData\GUI-Shell\config\gui_shell.json" `
  -AuditDir "$env:ProgramData\GUI-Shell\audit" `
  -VisibleSurfacesOutputPath .\release_evidence\visible_surfaces.json `
  -BrokerEvidenceJson .\release_evidence\windows_broker_smoke.json `
  -BrokerHelperExe "$env:LOCALAPPDATA\GUI-Shell\installed\broker\gui_shell_rust_helper.exe" `
  -NoPythonRuntime `
  -RuntimeAssertionsJson .\release_evidence\release_runtime_assertions.json `
  -OutputPath release_evidence\windows_installed_smoke.json
```

Then validate:

```powershell
python tooling\windows_release_evidence.py
python tooling\validate_all.py --strict-release --desktop-platform=windows
```

The evidence must prove:

- installed executable exists and has a tagged sha256 hash
- first run launches from the installed app path, remains running, and exposes a non-zero `MainWindowHandle`
- first run launches the installed Flutter `.exe` with `GUI_SHELL_BROKER_ENDPOINT_JSON` supplied by the installed Rust broker
- first run records `-NoPythonRuntime` launch evidence with Python PATH entries scrubbed and no `python`, `python3`, or `py` command visible to the launch process
- Dashboard, NavigationRail, Runtime Status, and Invariant Status are visible with recorded UIAutomation, screenshot, or accessibility-tree evidence
- first-run config exists at the recorded path and parses as JSON
- audit directory passes a write/read/delete probe
- installer/setup state grants no authority
- installer/setup state silently approves no permissions
- Setup Doctor runs from the installed app path
- Setup Doctor checks are operator-readable, non-authoritative, non-synthetic, and include installed path, artifact hash, config, audit, runtime connection, authority boundary, network public bind, recovery instruction, and audit storage checks
- the authority surface uses broker-mediated IPC without Python authority process startup or Flutter/Rust FFI/direct bridge tokens according to `tooling\release_runtime_assertions.py --check`
- broker installed-path smoke passes authenticated IPC, restricted `127.0.0.1` bind, durable store readiness, replay rejection after broker restart, crash fail-closed connection behavior, no authority Python runtime requirement, and no Flutter/Rust FFI authority bridge

The collectors must not synthesize Setup Doctor, visible-surface, or broker evidence. Missing `-SetupDoctorJson`, missing UIAutomation visible-surface evidence, missing `-BrokerEvidenceJson`, unmeasured config/audit probes, or manual confirmation evidence must remain release blockers.

Do not claim completed product release from copied, edited, or non-Windows evidence. Missing evidence remains a `release_blocker`.
