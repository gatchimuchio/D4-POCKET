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
  reason: Windows Flutter analyze passed on a native Windows host.
  required_action: Keep `cd apps/desktop_flutter && flutter analyze` passing on Windows release candidates.
  blocks_release: no

- item: Windows Flutter test
  classification: required_for_v1
  reason: Windows Flutter test passed on a native Windows host.
  required_action: Keep `cd apps/desktop_flutter && flutter test` passing on Windows release candidates.
  blocks_release: no

- item: Windows build smoke
  classification: required_for_v1
  reason: `cd apps/desktop_flutter && flutter build windows` passed on a native Windows host.
  required_action: Keep Windows build smoke passing on release candidates.
  blocks_release: no

## Launch Smoke Evidence Requirement

- item: Windows launch smoke
  classification: required_for_v1
  reason: `.\build\windows\x64\runner\Release\gui_shell_desktop.exe` launched successfully on native Windows; Dashboard, NavigationRail, Runtime Status, and Invariant Status were visible in the first window.
  required_action: Keep Windows launch smoke passing on release candidates.
  blocks_release: no

## Installer And First-Run Requirement

- item: implementation first-run and Setup Doctor smoke
  classification: required_for_v1
  reason: cross-platform implementation smoke creates first-run config/audit paths, verifies audit writability, runs structured Setup Doctor diagnostics, and confirms installer/setup state grants no authority and silently approves no permissions.
  required_action: Keep `python3 tooling/release_smoke.py` passing while completing native Windows installed-path validation.
  blocks_release: no

- item: Windows installer and first-run smoke
  classification: release_blocker
  reason: native Windows installed-path installer and first-run evidence is missing from `release_evidence/windows_installed_smoke.json`.
  required_action: Install through the Windows release path, launch the installed Flutter `.exe` through the installed Rust broker, run `installer\windows\collect_broker_smoke.ps1`, `installer\windows\collect_setup_doctor.ps1`, and `installer\windows\collect_installed_smoke.ps1` with `-BrokerHelperExe`, `-NoPythonRuntime`, measured visible-surface, config, audit, and Setup Doctor inputs, and pass `python tooling\windows_release_evidence.py`.
  blocks_release: yes

- item: Windows Setup Doctor smoke
  classification: release_blocker
  reason: native Windows Setup Doctor smoke has not passed from the installed Windows app path because machine-readable evidence is missing.
  required_action: Run Setup Doctor from the installed Windows app path, record non-synthetic required diagnostics in `release_evidence/windows_installed_smoke.json`, and pass `python tooling\windows_release_evidence.py`.
  blocks_release: yes

- item: Windows installed evidence validator
  classification: required_for_v1
  reason: `tooling/windows_release_evidence.py` gates Windows installer/first-run and Setup Doctor release evidence on installed Flutter `.exe` launch, broker-mediated first-run endpoint evidence, No-Python launch evidence, artifact hash, non-zero window handle, visible-surface evidence source, config JSON parsing, audit write/read/delete probe, broker restricted loopback bind, broker authenticated IPC/restart/crash evidence, and non-synthetic non-authoritative diagnostics.
  required_action: Keep the validator strict enough to reject copied, edited, synthetic, manually confirmed, or non-Windows evidence before owner GO.
  blocks_release: no

## Windows-Specific Failure Modes

- item: PATH resolution
  classification: release_blocker
  reason: Flutter, Git, runtime, or helper commands may resolve differently across PowerShell, CMD, installer environment, and user shell.
  required_action: Validate PATH from the installed app path and Setup Doctor.
  blocks_release: yes

- item: PowerShell policy
  classification: release_blocker
  reason: execution policy can block scripts or helper launch paths.
  required_action: Detect and report policy issues without silently broadening authority.
  blocks_release: yes

- item: Visual Studio Build Tools
  classification: release_blocker
  reason: missing C++ workload or Windows SDK blocks `flutter build windows`.
  required_action: Detect missing build tools and provide operator-visible recovery guidance.
  blocks_release: yes

- item: Windows Defender
  classification: release_blocker
  reason: quarantine or controlled-folder access can block helper, installer, cache, or runtime files.
  required_action: Detect likely Defender interference and classify recovery steps.
  blocks_release: yes

- item: WSL boundary confusion
  classification: release_blocker
  reason: WSL paths and Windows paths can cross authority and filesystem expectations.
  required_action: Keep Windows release validation on native Windows app paths and classify WSL use separately.
  blocks_release: yes

- item: filesystem permission
  classification: release_blocker
  reason: Program Files, user profile, temp, and workspace permissions can differ.
  required_action: Validate filesystem diagnostics through Shell Core permission, approval, audit, and recovery mapping.
  blocks_release: yes

- item: Git credential / SSH credential confusion
  classification: release_blocker
  reason: Windows Credential Manager, SSH agent, Git config, and WSL credentials can diverge.
  required_action: Detect credential-surface ambiguity without exposing secrets or treating credentials as authority.
  blocks_release: yes
