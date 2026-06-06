# Windows Release Plan

Status date: 2026-05-26

GUI-Shell v1.0 is Windows-first. Linux build and launch smoke are useful development verification, but they are not final product proof by themselves. BLUE-TANUKI remains a consumer/reference runtime and is not a GUI-Shell release dependency.

macOS is an unverified planned portability target. GUI-Shell v1.0 does not claim verified macOS support.

Mobile remains `post_v1_scope` unless the owner explicitly changes v1.0 scope.

## Toolchain Requirements

- item: Flutter Windows desktop SDK
  classification: required_for_v1
  reason: Native Windows Flutter analyze, test, build, and launch smoke passed.
  required_action: Keep Windows Flutter desktop toolchain validation current on release candidates.
  blocks_release: no

- item: Visual Studio Build Tools
  classification: required_for_v1
  reason: Native Windows `flutter build windows` passed and produced `build\windows\x64\runner\Release\gui_shell_desktop.exe`.
  required_action: Keep Visual Studio Build Tools available for Windows desktop release-candidate builds.
  blocks_release: no

- item: Windows desktop project support
  classification: required_for_v1
  reason: `flutter create --platforms=windows .` generated `apps/desktop_flutter/windows` without overwriting existing `lib/` app code.
  required_action: Keep Windows Flutter desktop project files under version control.
  blocks_release: no

## Validation Commands

- item: Windows Flutter analyze
  classification: required_for_v1
  reason: Windows Flutter analyze passed historically on a native Windows host; strict R2 release promotion requires current release-candidate provenance.
  required_action: Keep `cd apps/desktop_flutter && flutter analyze` passing on Windows release candidates and bind current validation to the exact source commit.
  blocks_release: no

- item: Windows Flutter test
  classification: required_for_v1
  reason: Windows Flutter test passed historically on a native Windows host; strict R2 release promotion requires current release-candidate provenance.
  required_action: Keep `cd apps/desktop_flutter && flutter test` passing on Windows release candidates and bind current validation to the exact source commit.
  blocks_release: no

- item: Windows build smoke
  classification: required_for_v1
  reason: `cd apps/desktop_flutter && flutter build windows` passed historically on a native Windows host; strict R2 requires current app artifact hash linkage.
  required_action: Keep Windows build smoke passing on release candidates and record the app artifact hash in the isolated staged manifest.
  blocks_release: no

## Launch Smoke Evidence Requirement

- item: Windows launch smoke
  classification: required_for_v1
  reason: historical Windows launch smoke passed for owner-trial use. Strict R2 formal proof requires per-surface UIAutomation/accessibility evidence from the isolated installed run, not aggregate native surface exposure.
  required_action: Keep Windows launch smoke passing on release candidates and recollect strict visible-surface evidence from the isolated installed path.
  blocks_release: no

## Installer And First-Run Requirement

- item: implementation first-run and Setup Doctor smoke
  classification: required_for_v1
  reason: cross-platform implementation smoke creates first-run config/audit paths, verifies audit writability, runs structured Setup Doctor diagnostics, and confirms installer/setup state grants no authority and silently approves no permissions.
  required_action: Keep `python3 tooling/release_smoke.py` passing while completing native Windows installed-path validation.
  blocks_release: no

- item: Windows installer and first-run smoke
  classification: release_blocker
  registry_id: windows_installer_first_run_smoke
  reason: native Windows isolated installed-path installer and first-run evidence is missing from `release_evidence/windows_installed_smoke.json`.
  required_action: Install through the unique staged Windows path, launch the installed Flutter `.exe` through the installed Rust broker, run `installer\windows\collect_broker_smoke.ps1`, run `installer\windows\collect_installed_smoke.ps1` with `-BrokerHelperExe`, `-NoPythonRuntime`, installed manifest, measured UIAutomation diagnostic tree, config, audit, and broker field-provenance inputs, and pass `python tooling\windows_release_evidence.py`.
  blocks_release: yes

- item: Windows Setup Doctor smoke
  classification: release_blocker
  registry_id: windows_setup_doctor_smoke
  reason: installed-app generated Setup Doctor product export support exists, but native Windows isolated-run evidence is missing. The PowerShell Setup Doctor collector is external probe evidence and is rejected as product proof.
  required_action: Run `collect_installed_smoke.ps1` so the installed app writes machine-readable Setup Doctor product export evidence and pass `python tooling\windows_release_evidence.py`.
  blocks_release: yes

- item: Windows installed evidence validator
  classification: required_for_v1
  reason: `tooling/windows_release_evidence.py` gates Windows installer/first-run and Setup Doctor release evidence on exact source commit provenance, clean worktree state, isolated run paths, app/broker artifact hash linkage, evidence bundle hashes, field provenance, installed Flutter `.exe` launch, broker-mediated first-run endpoint evidence, No-Python launch evidence, non-zero window handle, visible-surface source plus diagnostic tree, config JSON parsing, audit write/read/delete probe, broker restricted loopback bind, broker authenticated IPC/restart/crash evidence, and installed-app generated Setup Doctor product export.
  required_action: Keep the validator strict enough to reject copied, edited, synthetic, manually confirmed, aggregate-surface, external-probe-as-product, unmeasured-declaration, or non-Windows evidence before owner GO.
  blocks_release: no

## Windows-Specific Failure Modes

- item: PATH resolution
  classification: release_blocker
  aggregate_of: windows_evidence_provenance_isolation, windows_installer_first_run_smoke, windows_setup_doctor_smoke
  reason: Flutter, Git, runtime, or helper commands may resolve differently across PowerShell, CMD, installer environment, and user shell.
  required_action: Validate PATH from the installed app path and Setup Doctor.
  blocks_release: yes

- item: PowerShell policy
  classification: release_blocker
  aggregate_of: windows_installer_first_run_smoke, windows_setup_doctor_smoke
  reason: execution policy can block scripts or helper launch paths.
  required_action: Detect and report policy issues without silently broadening authority.
  blocks_release: yes

- item: Visual Studio Build Tools
  classification: release_blocker
  aggregate_of: windows_evidence_provenance_isolation
  reason: missing C++ workload or Windows SDK blocks `flutter build windows`.
  required_action: Detect missing build tools and provide operator-visible recovery guidance.
  blocks_release: yes

- item: Windows Defender
  classification: release_blocker
  aggregate_of: windows_installer_first_run_smoke, windows_broker_installed_smoke
  reason: quarantine or controlled-folder access can block helper, installer, cache, or runtime files.
  required_action: Detect likely Defender interference and classify recovery steps.
  blocks_release: yes

- item: WSL boundary confusion
  classification: release_blocker
  aggregate_of: windows_evidence_provenance_isolation
  reason: WSL paths and Windows paths can cross authority and filesystem expectations.
  required_action: Keep Windows release validation on native Windows app paths and classify WSL use separately.
  blocks_release: yes

- item: filesystem permission
  classification: release_blocker
  aggregate_of: windows_installer_first_run_smoke, windows_broker_installed_smoke, audit_anchor_external_tamper_evidence_proof
  reason: Program Files, user profile, temp, and workspace permissions can differ.
  required_action: Validate filesystem diagnostics through Shell Core permission, approval, audit, and recovery mapping.
  blocks_release: yes

- item: Git credential / SSH credential confusion
  classification: release_blocker
  aggregate_of: windows_setup_doctor_smoke
  reason: Windows Credential Manager, SSH agent, Git config, and WSL credentials can diverge.
  required_action: Detect credential-surface ambiguity without exposing secrets or treating credentials as authority.
  blocks_release: yes
