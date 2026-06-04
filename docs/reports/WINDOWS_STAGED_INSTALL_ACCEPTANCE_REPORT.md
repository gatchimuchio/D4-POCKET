# Windows Staged-Install Acceptance Report

## 0. Current R2 Evidence Classification

- item: this historical report
  classification: release_blocker
  reason: This report is preserved as historical owner-trial evidence only. It is invalid for current strict R2 formal proof because the old run used aggregate native surface exposure, lacked isolated run provenance, lacked exact source/artifact/evidence bundle linkage, and treated external Setup Doctor probe output as formal evidence.
  required_action: Recollect Windows evidence through the strict R2 isolated installed-path flow before any completed product release claim.
  blocks_release: yes

## 1. 結論
- Result: historical PASS, invalid for current strict R2 formal proof
- Tested implementation commit: `3e7d6079eaeb3af0dd1cd0fc38c84ba095dad9f7`
- Branch: `main`
- Date/time: `2026-06-03T14:24:01.8473316+09:00`
- Execution environment: native Windows PowerShell 7.6.1 on Microsoft Windows 10.0.26200
- Scope: Windows staged-install acceptance only
- Validator command: `python tooling\windows_release_evidence.py`
- Validator exit code: `0`

This proves only that the historical staged Windows install evidence gate passed under the older validator for the tested commit. It is no longer valid R2 formal release evidence.

## 2. 修正した原因分類
| Cause | Classification | Fix |
|---|---|---|
| Runtime assertions evidence mismatch | evidence-format defect | `tooling\release_runtime_assertions.py --json` output is now the evidence file passed to the installed smoke collector; `--check` remains a separate gate. |
| PowerShell JSON BOM in evidence files | collector serialization defect | Windows collectors now write machine evidence JSON with .NET `UTF8Encoding(false)` so Python `encoding="utf-8"` validation can parse it. |
| Flutter UIAutomation surface visibility gap | Windows accessibility exposure defect | Flutter semantic labels were added/enabled, and the Windows runner exposes the first-run visible surface labels through the native FlutterView accessible name so UIAutomation can record them. |

Validator conditions, authority boundaries, and broker design were not weakened.

## 3. Execution Summary
| Step | Command / Target | Result | Evidence |
|---|---|---|---|
| Rust release build | `cargo build --release` in `native\rust_helper` | PASS | Finished release profile. |
| Flutter Windows release build | `flutter build windows --release` in `apps\desktop_flutter` | PASS | Built `build\windows\x64\runner\Release\gui_shell_desktop.exe`. |
| Stage install | `installer\windows\stage_installed_app.ps1` | PASS | Staged app at `%LOCALAPPDATA%\GUI-Shell\installed`. |
| Runtime assertions | `python tooling\release_runtime_assertions.py --check` | PASS | `9 passed, 0 failed`, evidence scope `CONFIG,FIXTURE,LIVE_RUNTIME`. |
| Broker smoke | `installer\windows\collect_broker_smoke.ps1` | PASS | `release_evidence\windows_broker_smoke.json`, broker status `passed`. |
| Setup Doctor smoke | `installer\windows\collect_setup_doctor.ps1` | PASS | `release_evidence\setup_doctor_installed.json`, setup status `pass`. |
| Installed smoke | `installer\windows\collect_installed_smoke.ps1` | PASS | `release_evidence\windows_installed_smoke.json`, first-run status `passed`. |
| Validator | `python tooling\windows_release_evidence.py` | PASS | Exit code `0`; all 3 release evidence checks passed. |

## 4. Release Gate Results
| Gate | Result | Validator classification |
|---|---|---|
| `windows_installer_first_run_smoke` | PASS | `none`, `blocks_release=no` |
| `windows_setup_doctor_smoke` | PASS | `none`, `blocks_release=no` |
| `windows_broker_installed_smoke` | PASS | `none`, `blocks_release=no` |

## 5. Acceptance Criteria Results
| Criterion | Result | Evidence |
|---|---|---|
| `python tooling\windows_release_evidence.py` exit code is `0` | PASS | Validator exit code `0`. |
| `first_run.status = passed` | PASS | `windows_installed_smoke.json`. |
| `main_window_handle` is non-zero | PASS | `6358766`. |
| `process_running_after_launch = true` | PASS | Recorded in `first_run`. |
| `first_window_visible = true` | PASS | Recorded in `first_run`. |
| `broker_mediated_launch = true` | PASS | Recorded in `first_run`. |
| `broker_transport = authenticated_loopback_tcp` | PASS | Recorded in `first_run`. |
| `no_python_runtime_requested = true` | PASS | Recorded in `first_run`. |
| `python_path_entries_remaining_count = 0` | PASS | Recorded in `first_run`. |
| `python_commands_visible_after_scrub = []` | PASS | Recorded in `first_run`. |
| `visible_surfaces_complete = true` | PASS | UIAutomation evidence recorded all required labels. |
| `Dashboard` recorded | PASS | `visible_surfaces.json`. |
| `NavigationRail` recorded | PASS | `visible_surfaces.json`. |
| `Runtime Status` recorded | PASS | `visible_surfaces.json`. |
| `Invariant Status` recorded | PASS | `visible_surfaces.json`. |
| `installer_grants_authority = false` | PASS | Recorded in first-run and Setup Doctor evidence. |
| `installer_silently_approves_permissions = false` | PASS | Recorded in first-run and Setup Doctor evidence. |

## 6. Evidence Files Retained Locally
| Evidence file | SHA-256 | Status | Notes |
|---|---|---|---|
| `release_evidence\windows_acceptance_transcript.txt` | `945BB8237D17062A407CB1C53CAE413F61A038CC206BA0800777F9AA954ADCBE` | Generated | Transcript retained locally. |
| `release_evidence\release_runtime_assertions.json` | `EA6B36A77BFD13DDA8431742D6EC703A372181DA8FCEC20C09DE67189822A25A` | Generated | JSON evidence from `--json`; `--check` also passed. |
| `release_evidence\windows_broker_smoke.json` | `A0E99FC143C2EBF69CC19E3B18CFFF0D58BE391415BCEB40D7772CF1B145041A` | Generated | Broker smoke status `passed`. |
| `release_evidence\setup_doctor_installed.json` | `885E7BE86996D076E1F4E2A70FCEB07760DC22CCB127B94A8A1FA2AA00580DD3` | Generated | Setup Doctor status `pass`. |
| `release_evidence\visible_surfaces.json` | `8C8D5F3B09418D73F3C0B62FF97562E6B89707B98A0DBCB81C768AB2064C28B6` | Generated | UIAutomation source; all required surface labels recorded. |
| `release_evidence\windows_installed_smoke.json` | `B7B48A72AF6AE88776E4B5F24A2624819C04C6CCCB9B55C0060CF9DAB071466B` | Generated | Final installed smoke evidence; validator input. |

Raw evidence files are retained locally and intentionally not committed.

## 7. Safety / Authority Notes
- Installer and Setup Doctor evidence report `installer_grants_authority=false`.
- Installer and Setup Doctor evidence report `installer_silently_approves_permissions=false`.
- Broker evidence reports restricted `127.0.0.1` loopback bind, authenticated IPC, durable store readiness, restart replay rejection, crash fail-closed behavior, no Python authority runtime requirement, and no Flutter/Rust FFI authority bridge.
- Runtime assertions still pass: product Flutter entry uses `ShellCoreClient.product()`, broker-mediated authority operations remain fail-closed, no Python authority process startup is detected, and no FFI/direct bridge authority path is detected.

## 8. Remaining Risks
- item: distributable installer install/uninstall/repair/update evidence
  classification: release_blocker
  reason: This report validates staged-install acceptance only, not a packaged installer lifecycle.
  required_action: Run and validate the distributable installer lifecycle before completed product release.
  blocks_release: yes

## 9. Repository Change Boundary
- Source changes were committed separately from this report.
- Report update target: `docs\reports\WINDOWS_STAGED_INSTALL_ACCEPTANCE_REPORT.md`.
- Raw evidence target: `release_evidence\*`, local only, not committed.
- Build/stage artifacts are local only and not committed.

## 10. Final Determination
- Windows staged-install acceptance for commit `3e7d6079eaeb3af0dd1cd0fc38c84ba095dad9f7`: PASS.
- Completed product release readiness: not claimed.
