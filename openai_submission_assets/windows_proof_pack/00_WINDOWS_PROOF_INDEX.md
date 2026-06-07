# Windows Proof Pack Index

This index is factual raw material only. It is not an OpenAI application draft, pitch text, essay, marketing story, or final submission answer.

## Repository State
- repo: C:\Users\mzcum\Documents\GUI-Shell
- branch: main
- current_git_commit: 4e92339259c584460671c5adce33b7f86a724167
- origin_main: 4e92339259c584460671c5adce33b7f86a724167
- worktree_start_state_log: notes/repo_state.txt
- windows_evidence_source_commit: 28a700fbe4b6b3aa095994825ca7d3d0aa4f2ecb
- windows_evidence_run_id: run-audit-anchor-28a700f-20260607

## Windows Host Summary
- Host Name:                     LAPTOP-89BTKF4N
- OS Name:                       Microsoft Windows 11 Home
- OS Version:                    10.0.26200 N/A Build 26200
- System Type:                   x64-based PC
- full_environment_log: logs/windows_environment.log

## Toolchain Summary
- python --version => Python 3.11.9
- py --version => Python 3.13.7
- git --version => git version 2.51.0.windows.1
- rustc --version => rustc 1.95.0 (59807616e 2026-04-14)
- cargo --version => cargo 1.95.0 (f2d3ce0bd 2026-03-21)
- flutter --version => Flutter 3.44.0 • channel stable • https://github.com/flutter/flutter.git
- dart --version => Dart SDK version: 3.12.0 (stable) (Fri May 8 01:51:14 2026 -0700) on "windows_x64"
- full_toolchain_log: logs/windows_environment.log

## Validation Commands And Results
| Command | Exit |
| --- | ---: |
| `python tooling\schema_check\check_schemas.py` | 0 |
| `python tooling\conformance_tests\run_conformance_skeleton.py` | 0 |
| `python tooling\manifest.py --check` | 0 |
| `python tooling\release_gate_check.py` | 0 |
| `python tooling\windows_release_evidence.py` | 0 |
| `python tooling\evidence_bundle.py --check` | 0 |
| `python tooling\validate_all.py --python-only` | 0 |
| `python tooling\release_gate_check.py --strict-release` | 1 |
- validation_log: logs/validation.log

## Build Commands And Results
| Command | Exit |
| --- | ---: |
| `cargo fmt --check` | 0 |
| `cargo test` | 0 |
| `cargo build --release` | 0 |
| `flutter analyze` | 0 |
| `flutter test` | 0 |
| `dart format --output=none --set-exit-if-changed .` | 0 |
| `flutter build windows` | 0 |
- build_validation_log: logs/build_validation.log

## Build Artifacts And Hashes
```text
# Artifact hashes
captured_at=2026-06-07T01:24:38.6164171Z
git_head=4e92339259c584460671c5adce33b7f86a724167

path=apps\desktop_flutter\build\windows\x64\runner\Release\gui_shell_desktop.exe
sha256=785B464BC04091169657C041F51314DA8C2961BFBD1E022369BA0F2D59B67468
size_bytes=91136
last_write_time=2026-06-07T02:03:07.0297695+09:00
full_path=C:\Users\mzcum\Documents\GUI-Shell\apps\desktop_flutter\build\windows\x64\runner\Release\gui_shell_desktop.exe

path=native\rust_helper\target\release\gui_shell_rust_helper.exe
sha256=515B2D09839BB629670B76A94CE6DC522CDDE626424D1BB7675FB035D7606C59
size_bytes=869888
last_write_time=2026-06-07T02:04:29.6364610+09:00
full_path=C:\Users\mzcum\Documents\GUI-Shell\native\rust_helper\target\release\gui_shell_rust_helper.exe
```

## Evidence Files And Hashes
```text
# Evidence copy hashes
captured_at=2026-06-07T01:36:00.2609104Z

source=release_evidence\windows_installed_smoke.json
copied_to=C:\Users\mzcum\Documents\GUI-Shell\openai_submission_assets\windows_proof_pack\evidence_copies\windows_installed_smoke.json
sha256=6CE96CA2461662E21F374CC55EB80EA4DA43F8E108BA19CC18D12189BBFEA817
size_bytes=36226

source=release_evidence\windows_broker_smoke.json
copied_to=C:\Users\mzcum\Documents\GUI-Shell\openai_submission_assets\windows_proof_pack\evidence_copies\windows_broker_smoke.json
sha256=CDBB1FD6BC1B382802067C31CFE09819E839C13D8EA4A226E5EB8CE0C99AE63C
size_bytes=2858

source=release_evidence\audit_anchor_external_tamper_evidence.json
copied_to=C:\Users\mzcum\Documents\GUI-Shell\openai_submission_assets\windows_proof_pack\evidence_copies\audit_anchor_external_tamper_evidence.json
sha256=40D29B8A2CB81D27F7ED55A44728493472668CE6C8B4055AB8B42F00600B6E27
size_bytes=4593

source=release_evidence\release_runtime_assertions.json
copied_to=C:\Users\mzcum\Documents\GUI-Shell\openai_submission_assets\windows_proof_pack\evidence_copies\release_runtime_assertions.json
sha256=78CF9AA06F8F79FBF41BDEF42693E45CCE0F7359DB4D2623ACCC476E26A218A7
size_bytes=6193

source=release_evidence\setup_doctor_installed.json
copied_to=C:\Users\mzcum\Documents\GUI-Shell\openai_submission_assets\windows_proof_pack\evidence_copies\setup_doctor_installed.json
sha256=2B610FF480E54A2FDB93D348D1B40ABE7B57B64973CFE54300DF7FDE95EC2B74
size_bytes=4370

source=release_evidence\setup_doctor_context.json
copied_to=C:\Users\mzcum\Documents\GUI-Shell\openai_submission_assets\windows_proof_pack\evidence_copies\setup_doctor_context.json
sha256=0E68629EE683CD1024C9003BFBD0CC74378AD1224E275A0FD345F7C8789125D3
size_bytes=886

source=release_evidence\visible_surfaces_collected.json
copied_to=C:\Users\mzcum\Documents\GUI-Shell\openai_submission_assets\windows_proof_pack\evidence_copies\visible_surfaces_collected.json
sha256=B46CA54B714EBCCB8A1D5661E4CA5625EB89DCBB47F6B62ADF6AAAB630BD1DD7
size_bytes=10100

source=release_evidence\surface_semantics_export.json
copied_to=C:\Users\mzcum\Documents\GUI-Shell\openai_submission_assets\windows_proof_pack\evidence_copies\surface_semantics_export.json
sha256=1E63388F2EB5D991E7127717874E60BE7317D0D696A87E473D8070D34D52C8C3
size_bytes=10091
```

## Screenshots
- screenshots/01_main_window.png (171672 bytes)
- screenshots/02_setup_doctor_or_status.png (171672 bytes)
- screenshots/03_validation_pass.png (306784 bytes)
- screenshots/04_strict_release_owner_go_only.png (164446 bytes)
- screenshots/05_windows_build_artifact.png (248288 bytes)
- screenshots/06_release_evidence_files.png (257360 bytes)
- screenshots/07_flutter_build_success.png (215996 bytes)
- screenshots/08_cargo_test_success.png (285673 bytes)

## Audit Anchor Proof Summary
- status: passed
- windows_acl_verified: True
- external_anchor_verified: False
- signed_evidence_verified: False
- synthetic: False
- evidence_class: LIVE_RUNTIME

## Remaining Blocker Status
- release_ready_in_registry: False
- strict_release_exit: 1
```text
release gate check failed:
  - strict release active blocker unresolved: owner_go - Explicit owner GO has not been recorded.
```

## Explicit Non-Claims
- release_ready was not set.
- owner GO was not recorded.
- product release was not claimed.
- no WSL/Git Bash/MSYS/Cygwin evidence was used for this pack.
- this pack is raw material for Ubuntu-side submission editing.
