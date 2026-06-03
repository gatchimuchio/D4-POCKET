# Windows Staged-Install Acceptance Report

## 1. Conclusion
- Result: FAIL
- Tested commit: `deba494cba98328e90b7d981b2eaed8ca5b0459b`
- Branch: `main`
- Date/time: `2026-06-03T13:24:03.1335057+09:00`
- Execution environment: native Windows PowerShell 7.6.1 on Microsoft Windows 10.0.26200
- Scope: Windows staged-install acceptance only
- Validator exit code: `1`

## 2. What This Test Proves
- Native Windows Rust release build completed for `native\rust_helper\target\release\gui_shell_rust_helper.exe`.
- Native Windows Flutter release build completed for `apps\desktop_flutter\build\windows\x64\runner\Release\gui_shell_desktop.exe`.
- `stage_installed_app.ps1` created a staged-install layout at `%LOCALAPPDATA%\GUI-Shell\installed`.
- Broker smoke collection passed with authenticated loopback TCP, durable store readiness, replay rejection after restart, fresh health after restart, and fail-closed crash evidence.
- Setup Doctor installed-path evidence passed and reported no installer authority grant and no silent permission approval.
- Release runtime assertions reported `9 passed, 0 failed` with evidence scope `CONFIG,FIXTURE,LIVE_RUNTIME`.

## 3. What This Test Does Not Prove
- This does not prove general-user installation through a distributable installer.
- This does not prove automatic update behavior.
- This does not prove uninstall, repair, or rollback behavior.
- This does not prove behavior of any already distributed product.
- This does not prove completed product release readiness because installed smoke did not produce `release_evidence\windows_installed_smoke.json` and the Windows release evidence validator failed.

## 4. Execution Summary
| Step | Command / Target | Result | Notes |
|---|---|---|---|
| Pre-check | `git status --short --branch` | PASS | `## main...origin/main`; no tracked or untracked files before execution. |
| Pre-check | `git branch --show-current` | PASS | `main`. |
| Pre-check | `git rev-parse HEAD` | PASS | `deba494cba98328e90b7d981b2eaed8ca5b0459b`. |
| Pre-check | `git rev-parse origin/main` | PASS | `deba494cba98328e90b7d981b2eaed8ca5b0459b`. |
| Pre-check | `git remote -v` | PASS | `origin` points to `https://github.com/gatchimuchio/GUI-Shell.git`. |
| Transcript | `Start-Transcript -Path release_evidence\windows_acceptance_transcript.txt -Force` | PASS | Transcript retained locally. |
| Rust release build | `cargo build --release` in `native\rust_helper` | PASS | Finished release profile in 44.00s. |
| Flutter Windows release build | `flutter build windows --release` in `apps\desktop_flutter` | PASS | Built `build\windows\x64\runner\Release\gui_shell_desktop.exe` in 42.6s. |
| Stage install | `installer\windows\stage_installed_app.ps1` | PASS | Staged app at `C:\Users\mzcum\AppData\Local\GUI-Shell\installed`. |
| Runtime assertions | `python tooling\release_runtime_assertions.py --check` | PASS | `release runtime assertions: 9 passed, 0 failed, evidence_scope=CONFIG,FIXTURE,LIVE_RUNTIME`. |
| Broker smoke | `installer\windows\collect_broker_smoke.ps1` | PASS | Wrote `release_evidence\windows_broker_smoke.json`; status `passed`. |
| Setup Doctor smoke | `installer\windows\collect_setup_doctor.ps1` | PASS | Wrote `release_evidence\setup_doctor_installed.json`; status `pass`. |
| Installed smoke | `installer\windows\collect_installed_smoke.ps1` | FAIL | Script halted at trap rethrow; `release_evidence\windows_installed_smoke.json` was not generated. |
| Validator | `python tooling\windows_release_evidence.py` | FAIL | Exit code `1`; failed three release evidence checks because `windows_installed_smoke.json` is missing. |

## 5. Acceptance Criteria Results
| Criterion | Result | Evidence |
|---|---|---|
| `python tooling\windows_release_evidence.py` exit code is `0` | FAIL | Validator exit code was `1`. |
| `windows_installer_first_run_smoke` is `passed` | FAIL | Validator reported `release_evidence\windows_installed_smoke.json missing`. |
| `windows_setup_doctor_smoke` is `passed` | FAIL | Validator reported `release_evidence\windows_installed_smoke.json missing`; standalone Setup Doctor evidence status was `pass`. |
| `windows_broker_installed_smoke` is `passed` | FAIL | Validator reported `release_evidence\windows_installed_smoke.json missing`; standalone broker evidence status was `passed`. |
| `first_run.status = passed` in `windows_installed_smoke.json` | FAIL | `windows_installed_smoke.json` was not generated. |
| `main_window_handle` is non-zero | NOT VERIFIED | `windows_installed_smoke.json` was not generated. |
| `broker_mediated_launch = true` | NOT VERIFIED | `windows_installed_smoke.json` was not generated. |
| `broker_transport = authenticated_loopback_tcp` | NOT VERIFIED | Broker evidence passed authenticated loopback, but installed smoke JSON was not generated. |
| `no_python_runtime_requested = true` | NOT VERIFIED | `windows_installed_smoke.json` was not generated. |
| `python_commands_visible_after_scrub = []` | NOT VERIFIED | `windows_installed_smoke.json` was not generated. |
| `visible_surfaces_complete = true` | FAIL | `visible_surfaces.json` was generated, but `visible_surfaces` was empty. |
| `Dashboard` recorded | FAIL | `visible_surfaces.json` listed only `FLUTTERVIEW` and `ControlType.Pane`. |
| `NavigationRail` recorded | FAIL | `visible_surfaces.json` listed only `FLUTTERVIEW` and `ControlType.Pane`. |
| `Runtime Status` recorded | FAIL | `visible_surfaces.json` listed only `FLUTTERVIEW` and `ControlType.Pane`. |
| `Invariant Status` recorded | FAIL | `visible_surfaces.json` listed only `FLUTTERVIEW` and `ControlType.Pane`. |
| `installer_grants_authority = false` | PASS | `setup_doctor_installed.json` reported `installer_grants_authority: false`. |
| `installer_silently_approves_permissions = false` | PASS | `setup_doctor_installed.json` reported `installer_silently_approves_permissions: false`. |

## 6. Evidence Files Retained Locally
| Evidence file | SHA-256 | Generated / Missing | Notes |
|---|---|---|---|
| `release_evidence\windows_acceptance_transcript.txt` | `5F4951581DD47AA81F3697124B93C0FD418419E565D8F7058E6328F44C000709` | Generated | Transcript retained locally. |
| `release_evidence\release_runtime_assertions.json` | `4C41180886F0F5FD91511EF169B5D28BD4AFC18024F7F126606F76F3667F8D63` | Generated | File contains text output from `release_runtime_assertions.py --check`, not JSON. |
| `release_evidence\windows_broker_smoke.json` | `FCADCDC4F32FC2B08AFD30B3E7C2221122AEC3B083E9DB56124A0725034F944B` | Generated | Broker smoke status `passed`. |
| `release_evidence\setup_doctor_installed.json` | `A0DD8CCE8FE0DE6B144C2CB4171C4816B7D0636636C041002B04E718C05DA470` | Generated | Setup Doctor status `pass`. |
| `release_evidence\visible_surfaces.json` | `5F23DCE29A83E6317F75D3D2D5339B66C996C9526EBFC850131CC1BCAE12C63C` | Generated | Window was found, but expected surface labels were not detected. |
| `release_evidence\windows_installed_smoke.json` | `not generated due to earlier failure` | Missing | `collect_installed_smoke.ps1` halted before writing the installed smoke evidence. |

Raw evidence files are retained locally and intentionally not committed to the repository.

The repository update contains this report only.

## 7. Failure Analysis
- Failed command: `powershell -ExecutionPolicy Bypass -File installer\windows\collect_installed_smoke.ps1 -InstalledExe $InstalledExe -SetupDoctorJson release_evidence\setup_doctor_installed.json -ConfigPath $ConfigPath -AuditDir $AuditDir -VisibleSurfacesOutputPath release_evidence\visible_surfaces.json -BrokerEvidenceJson release_evidence\windows_broker_smoke.json -BrokerHelperExe $BrokerExe -NoPythonRuntime -RuntimeAssertionsJson release_evidence\release_runtime_assertions.json -OutputPath release_evidence\windows_installed_smoke.json`
- Relevant stdout/stderr:
  - `ScriptHalted`
  - `At C:\Users\mzcum\repos\GUI-Shell\installer\windows\collect_installed_smoke.ps1:217 char:3`
  - `throw`
  - Validator output: `release_evidence\windows_installed_smoke.json missing`
- Observed evidence:
  - `visible_surfaces.json` reported `window_found: true` and `window_title: gui_shell_desktop`.
  - `visible_surfaces.json` did not detect `Dashboard`, `NavigationRail`, `Runtime Status`, or `Invariant Status`.
  - `release_runtime_assertions.json` contains plain text output, while `collect_installed_smoke.ps1` reads `-RuntimeAssertionsJson` through `ConvertFrom-Json`.
- Failure classification: installed smoke failure
- Related failure class: GUI session / window detection failure
- Required next action: inspect the installed smoke collector path without changing product behavior, especially runtime assertions evidence format and UIAutomation surface extraction, then rerun the staged-install evidence collection.

## 8. Repository Change Boundary
- Modified tracked files before execution: none
- Modified tracked files after execution: `docs/reports/WINDOWS_STAGED_INSTALL_ACCEPTANCE_REPORT.md`
- File committed: `docs/reports/WINDOWS_STAGED_INSTALL_ACCEPTANCE_REPORT.md`
- Local evidence artifacts retained, not committed: `release_evidence\windows_acceptance_transcript.txt`, `release_evidence\release_runtime_assertions.json`, `release_evidence\windows_broker_smoke.json`, `release_evidence\setup_doctor_installed.json`, `release_evidence\visible_surfaces.json`
- Missing evidence artifact: `release_evidence\windows_installed_smoke.json`
- Explicit confirmation: no source code, configuration file, generated evidence file, build artifact, staged install artifact, or dependency file is intended to be committed.

## 9. Final Determination
- The Windows staged-install acceptance condition is not complete.
- Remaining item: the installed smoke collector must produce `release_evidence\windows_installed_smoke.json`, and `python tooling\windows_release_evidence.py` must return exit code `0` with `windows_installer_first_run_smoke`, `windows_setup_doctor_smoke`, and `windows_broker_installed_smoke` all passed before this acceptance gate can be considered complete.
