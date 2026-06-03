# Windows installed path

GUI-Shell v1.0 is Windows-first. The installed path must launch the Flutter UI through a broker-mediated runtime path:

```text
GUI-Shell.brokered.cmd
  -> GUI-Shell.brokered.ps1
  -> gui_shell_rust_helper.exe broker-server
  -> gui_shell_desktop.exe with GUI_SHELL_BROKER_ENDPOINT_JSON
```

Use `stage_installed_app.ps1` to create a staged installed directory from an already-built Flutter Windows release directory and a Windows `gui_shell_rust_helper.exe`.

Use `collect_broker_smoke.ps1` before final installed evidence collection. It validates authenticated broker IPC, restricted `127.0.0.1` bind, durable store readiness, replay rejection after broker restart, crash fail-closed connection behavior, and records that Python and FFI are not authority runtime requirements.

Use `collect_setup_doctor.ps1` to collect installed-path Setup Doctor JSON without Python. `collect_installed_smoke.ps1` then starts the installed Rust broker, launches the installed Flutter `.exe` with `GUI_SHELL_BROKER_ENDPOINT_JSON`, applies `-NoPythonRuntime` PATH scrubbing for launch evidence, captures UIAutomation visible-surface evidence when `-VisibleSurfacesJson` is not supplied, and combines app first-run evidence, Setup Doctor evidence, visible-surface evidence, and broker evidence into `release_evidence/windows_installed_smoke.json`.

Normal users must not be required to manually use terminal, WSL, npm, Git, port setup, or runtime root discovery. The staged `.cmd` launcher is a bounded packaging step until a signed installer/MSIX wrapper is added.
