# Phase Strategy

GUI-Shell uses phase-based readiness language so owner-use progress is not confused with completed product release readiness.

## Current Phase

- phase: A
  name: personal Windows trial operation
  status: complete
  evidence: Windows desktop build and native launch smoke passed; Dashboard, NavigationRail, Runtime Status, and Invariant Status were visible.

- phase: B
  name: owner-use operational hardening
  status: complete
  goal: make GUI-Shell useful for daily personal operation while preserving authority, audit, approval, recovery, and evidence boundaries.
  current_surfaces: Dashboard phase status, persistent status bar, Problems Panel, Evidence Center, Recovery Playbook, Trust Center, Runtime Center, and Authority Map are available as display-only owner-operation surfaces.

## Phase B Roadmap

- item: B-1 owner operation console
  classification: required_for_v1
  status: complete
  evidence: Dashboard phase status, persistent status bar, Problems Panel, Evidence Center, Recovery Playbook, and release-not-claimed UI are implemented.
  blocks_release: no

- item: B-2 local snapshot / local runtime wiring
  classification: required_for_v1
  status: complete
  evidence: `ShellCoreClient.local()` loads `GUI_SHELL_SNAPSHOT_JSON`, `%LOCALAPPDATA%\GUI-Shell\shell_snapshot.json`, or `.gui_shell/shell_snapshot.json`; parse/missing failures fall back safely without release claim.
  blocks_release: no

- item: B-3 owner launch flow
  classification: required_for_v1
  status: complete
  evidence: `scripts/launch_owner_desktop.sh` and `scripts/launch_owner_desktop.ps1` generate `.gui_shell/shell_snapshot.json` and launch Flutter desktop without strict release validation or release evidence generation.
  blocks_release: no

- item: B-4 Problems to Recovery loop
  classification: required_for_v1
  status: complete
  evidence: Problems rows carry `recovery_id`, `safe_to_ignore_for_phase_b`, `blocks_owner_use`, and `blocks_completed_product_release`; the Problems Panel displays the matching recovery command/path without executing privileged auto-fix.
  blocks_release: no

- item: B-5 Trust / Authority / Runtime Map
  classification: required_for_v1
  status: complete
  evidence: Trust Center and Authority Map are restored in desktop navigation; Runtime Center displays Runtime -> Capability -> Permission -> Approval -> Audit -> Recovery while preserving Flutter display-only authority boundaries.
  blocks_release: no

- item: B-6 owner-use completion gate
  classification: required_for_v1
  status: complete
  evidence: Owner launch helpers generate local snapshots and open the desktop shell; status, problems, evidence, recovery, trust, runtime, and authority surfaces are visible; local snapshot/fallback preserves `release_state: not claimed`.
  blocks_release: no

## Later Phases

- phase: C
  name: OSS claim hygiene
  status: next
  goal: keep README, CLAIM, release checklist, audit, installer, security, strategy docs, and language-policy convergence blockers aligned so external readers cannot mistake Phase B for release readiness.

- phase: D
  name: measured Windows release evidence
  status: later
  goal: collect native Windows installed-path evidence and pass the hardened Windows evidence validator.

- phase: E
  name: OSS v1.0 release candidate
  status: later
  goal: pass strict Windows release validation, preserve known limitations, and wait for owner GO.

- phase: F
  name: paid/product QC
  status: later
  goal: complete support, rollback, long-run, legal, dependency, installer, and third-party-user quality gates.

## Release Rule

Do not claim completed product release until language policy runtime convergence is proven, strict Windows release validation passes, and owner GO is explicit.

Phase B may improve owner usability without weakening strict release gates.

## Language Policy Gate

- item: Rust Security Broker migration
  classification: release_blocker
  status: partially_started_not_passed
  reason: Rust broker skeleton and JSON envelope rejection tests exist, but current owner-use and validation Shell Core behavior is still implemented in Python, and the broker is not yet the production authority path.
  required_action: Complete production IPC transport, parity migration, Flutter broker-mediated authority path, no-Python-runtime assertion, and no-FFI-authority assertion before Phase D evidence can support a completed product release claim.
  blocks_release: yes
