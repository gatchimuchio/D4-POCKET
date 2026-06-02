# Windows Release Evidence

Windows-first release validation uses machine-readable installed-path evidence.

Required evidence file:

```text
release_evidence/windows_installed_smoke.json
```

Generate it on a native Windows host after installing or staging the Windows release artifact:

```powershell
powershell -ExecutionPolicy Bypass -File installer\windows\collect_installed_smoke.ps1 `
  -InstalledExe .\build\windows\x64\runner\Release\gui_shell_desktop.exe `
  -SetupDoctorJson .\release_evidence\setup_doctor_installed.json `
  -ConfigPath "$env:ProgramData\GUI-Shell\config\gui_shell.json" `
  -AuditDir "$env:ProgramData\GUI-Shell\audit" `
  -VisibleSurfacesJson .\release_evidence\visible_surfaces.json `
  -OutputPath release_evidence\windows_installed_smoke.json
```

Then validate:

```powershell
python tooling\release_runtime_assertions.py --check
python tooling\windows_release_evidence.py
python tooling\validate_all.py --strict-release --desktop-platform=windows
```

The evidence must prove:

- installed executable exists and has a tagged sha256 hash
- first run launches from the installed app path, remains running, and exposes a non-zero `MainWindowHandle`
- Dashboard, NavigationRail, Runtime Status, and Invariant Status are visible with recorded UIAutomation, screenshot, or accessibility-tree evidence
- first-run config exists at the recorded path and parses as JSON
- audit directory passes a write/read/delete probe
- installer/setup state grants no authority
- installer/setup state silently approves no permissions
- Setup Doctor runs from the installed app path
- Setup Doctor checks are operator-readable, non-authoritative, non-synthetic, and include installed path, artifact hash, config, audit, runtime connection, authority boundary, network public bind, recovery instruction, and audit storage checks
- the authority surface uses broker-mediated IPC without Python authority process startup or Flutter/Rust FFI/direct bridge tokens according to `tooling\release_runtime_assertions.py --check`

The collector must not synthesize Setup Doctor evidence. Missing `-SetupDoctorJson`, missing visible-surface evidence, unmeasured config/audit probes, or manual confirmation evidence must remain release blockers.

Do not claim completed product release from copied, edited, or non-Windows evidence. Missing evidence remains a `release_blocker`.
