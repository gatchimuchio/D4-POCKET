# Windows installed path

GUI-Shell v1.0 is Windows-first. The installed path must launch the Flutter UI through a broker-mediated runtime path:

```text
GUI-Shell.brokered.cmd
  -> GUI-Shell.brokered.ps1
  -> gui_shell_rust_helper.exe broker-server
  -> gui_shell_desktop.exe with GUI_SHELL_BROKER_ENDPOINT_JSON
```

Use `stage_installed_app.ps1` to create a run-unique staged installed directory from an already-built Flutter Windows release directory and a Windows `gui_shell_rust_helper.exe`. The default install root is `%LOCALAPPDATA%\GUI-Shell\installed-runs\<run_id>` and the generated manifest records source commit, clean worktree state, artifact hashes, and isolated runtime/config/audit/store paths.

Use `collect_broker_smoke.ps1` before final installed evidence collection. It validates authenticated broker IPC, restricted `127.0.0.1` bind, durable store readiness, replay rejection after broker restart, and crash fail-closed connection behavior. No-Python/no-FFI values are recorded only as non-formal static declarations, not as broker runtime proof.

Use `collect_setup_doctor.ps1` only as an external installer/config/broker probe; it is not formal product evidence. `collect_installed_smoke.ps1` starts the installed Rust broker, launches the installed Flutter `.exe` with `GUI_SHELL_BROKER_ENDPOINT_JSON`, requests an installed-app Setup Doctor product export with `GUI_SHELL_SETUP_DOCTOR_EXPORT_JSON`, applies `-NoPythonRuntime` PATH scrubbing for launch evidence, captures UIAutomation visible-surface evidence and diagnostic tree projection when `-VisibleSurfacesJson` is not supplied, and combines app first-run evidence, app-generated Setup Doctor evidence, visible-surface evidence, broker evidence, provenance, and field provenance into `release_evidence/windows_installed_smoke.json`.

Normal users must not be required to manually use terminal, WSL, npm, Git, port setup, or runtime root discovery. The staged `.cmd` launcher is a bounded packaging step until a signed installer/MSIX wrapper is added.
