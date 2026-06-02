# GUI Shell Roadmap

Status: Phase B owner-use complete; v1.0 desktop release roadmap  
Project: GUI Shell / Runtime Operation Shell  
Reference consumer/runtime: BLUE-TANUKI via adapter only  
Primary implementation path: Flutter UI + Rust Security Broker for authority-sensitive production convergence; Rust helper remains bounded native diagnostics/operations where outside authority.

## Completion Audit Priority

- item: Ghost Invariants
  classification: required_for_v1
  status: implemented_for_current_scope
  reason: `packages/shell_core/state_snapshot.py` now reports measured `InvariantEvaluator` results instead of static invariant flags.
  required_action: Keep intentional violation tests in conformance and extend them as new invariant surfaces are added.
  blocks_release: no

- item: Normalization Firewall
  classification: required_for_v1
  status: implemented_for_current_scope
  reason: Shell Core now preserves raw inbound payloads, normalizes keys, strips authority aliases, detects authority-like values, quarantines ambiguous payloads, and records normalization audit metadata.
  required_action: Keep Unicode/case/zero-width/camelCase/envelope/value-only escalation tests passing.
  blocks_release: no

- item: Language policy runtime convergence
  classification: release_blocker
  status: partially_started_not_passed
  reason: Rust broker skeleton and JSON envelope rejection tests exist under `native/rust_helper`, including NFKC / case / zero-width / camelCase / separator / alias / value-only metadata hardening and unit-scope `issued_at` freshness rejection, but production IPC transport, Flutter broker integration, persistent audit/replay/session state, authority-sensitive responsibility migration, no-Python-runtime product evidence, and no-FFI-authority release assertion are not complete.
  required_action: Select restricted IPC transport, add persistent audit/replay/session, integrate freshness checks into IPC tests, integrate Flutter through the broker for authority operations, migrate authority-sensitive active runtime responsibilities from Python to Rust with parity evidence, and prove Python is dev/test/migration oracle only before completed product release.
  blocks_release: yes

- item: Windows installer, first-run, and real Setup Doctor
  classification: release_blocker
  status: not_passed
  reason: installed app path Setup Doctor and Windows installer/first-run smoke are still the primary Windows-first product blockers.
  required_action: Implement installed app path diagnostics, Windows installer/first-run flow, artifact/hash evidence, and strict Windows validation.
  blocks_release: yes

## 0. Product Definition

GUI Shell is a PC-first AI Runtime / Agent Operation Shell for local runtimes, agents, tools, and services.

It is not a BLUE-TANUKI-specific GUI.

BLUE-TANUKI is a reference consumer/runtime and must connect through an adapter boundary. GUI Shell Core must not contain BLUE-TANUKI-specific logic. BLUE-TANUKI live integration is not a GUI-Shell v1.0 release dependency.

## v1.0 Desktop Release Scope

GUI-Shell v1.0 is Windows-first.

Platform priority:

- Primary: Windows
- Planned portability target: macOS
- Development/verification slice: Linux

Linux build and launch smoke are passed and useful, but Linux is not final product proof by itself. Windows is the main product gate.

GUI-Shell v1.0 does not claim verified macOS support. macOS support must not be advertised as supported, ready, or complete until it is validated on a macOS host.

- item: Linux desktop build smoke
  classification: required_for_v1
  reason: Linux development/verification build smoke passed on 2026-05-25.
  required_action: Keep `cd apps/desktop_flutter && flutter build linux` passing as a development verification slice.
  blocks_release: no

- item: Linux desktop launch smoke
  classification: required_for_v1
  reason: Linux launch smoke passed under WSLg on 2026-05-25 with Dashboard, NavigationRail, Runtime Status, and Invariant Status visible; this does not replace Windows-first release evidence.
  required_action: Keep Linux launch smoke evidence current while completing Windows product gates.
  blocks_release: no

- item: Windows desktop release gates
  classification: release_blocker
  reason: Windows is the primary product target. Windows project support, Flutter analyze, Flutter test, Windows build, and native launch smoke have passed as development evidence, but installed-path first-run evidence, installed-path Setup Doctor evidence, broker-mediated runtime evidence, language policy runtime convergence, strict Windows release validation, and owner GO have not passed.
  required_action: Keep Windows development smoke current, then pass installed-path first-run, installed-path Setup Doctor, broker-mediated runtime evidence, strict Windows release validation, and owner GO.
  blocks_release: yes

- item: macOS planned portability target
  classification: known_limitation
  reason: no macOS validation environment is currently available, so GUI-Shell v1.0 does not claim verified macOS support.
  required_action: Validate on a macOS host before claiming macOS support as supported, ready, or complete.
  blocks_release: no

- item: Windows Setup Doctor diagnostics
  classification: release_blocker
  reason: Windows-specific Setup Doctor diagnostics are part of the primary product gate.
  required_action: Pass Windows Setup Doctor diagnostics smoke from the app path.
  blocks_release: yes

- item: Windows installer and first-run plan
  classification: release_blocker
  reason: Windows installer and first-run behavior are part of the primary product gate.
  required_action: Complete and validate Windows installer/first-run flow.
  blocks_release: yes

## 1. Non-Negotiable Priorities

1. Safety
2. Robustness
3. Operator clarity / UX
4. Product features
5. Convenience

Feature completion must never outrank safety, authority boundaries, auditability, recovery, or operator visibility.

## 2. Core Principle

GUI Shell is a control plane, not a visual wrapper.

The UI may display runtime state and collect operator input.
The UI must not create authority, grant permission, reinterpret trust, bypass adapter conformance, or hide sensitive actions.

Schemas and conformance own the contract.

## 3. Locked Phase 0 Decisions

The following decisions are locked for the current execution path:

- Generic Runtime Operation Shell direction
- BLUE-TANUKI as reference runtime only
- BLUE-TANUKI connected through adapter boundary
- Flutter UI + Rust Security Broker as the authority-sensitive implementation path
- Rust helper as bounded non-authority native diagnostics/operations
- Compose Multiplatform as watchlist candidate
- Tauri as desktop-heavy fallback
- JSON Schema-first contract model
- Conformance-first implementation order
- FrameworkRiskProfile for UI framework governance risk
- Authority Strip Conformance
- Content Exposure Boundary
- Approval visibility and edit boundaries

## 4. Architecture Target

```text
Runtime / Agent / Tool / Local Service
  -> Adapter
      -> schema validation
      -> authority strip
      -> content exposure policy
      -> capability declaration
      -> diagnostic normalization
  -> Rust Security Broker
      -> restricted IPC endpoint
      -> schema-validated broker envelope
      -> runtime registry authority path
      -> capability / permission eligibility
      -> approval validation and protected-field enforcement
      -> audit append / verification authority
      -> recovery classification
      -> command-envelope eligibility
      -> process / credential / update gated execution
  -> Shell Core Contracts
      -> runtime-neutral JSON Schema / protocol semantics
      -> migration oracle parity fixtures until cutover
  -> Flutter UI Layer
      -> Flutter rendering
      -> operator input
      -> navigation
      -> local UI state
  -> Rust Helper
      -> bounded non-authority native diagnostics
      -> bounded non-authority native operations
```

## 5. Phase Plan

The canonical phase definitions live in [docs/PHASE_STRATEGY.md](./docs/PHASE_STRATEGY.md). This file keeps the execution roadmap only:

- Phase A: complete
- Phase B: owner-use complete
- Phase C: OSS claim hygiene next
- Phase D: measured Windows installed-path release evidence later
- Phase E: OSS v1.0 RC later
- Phase F: paid/product QC later

Completed product release is not claimed. Language policy runtime convergence, Phase D measured Windows installed-path evidence, and explicit owner GO remain `release_blocker` items before any release-ready claim.

### Phase 0: Standard / Selection Freeze

Goal: Freeze the conceptual and technical selection boundary.

Deliverables:

- `docs/standards/gui-shell-extended-standard.md`
- `docs/research/flutter-governance-risk.md`
- `docs/research/compose-mp-watchlist.md`
- `docs/research/tauri-fallback.md`
- `CLAIM.md`
- `CONFIG.md`
- `AUDIT.md`
- `SECURITY.md`
- `TROUBLESHOOTING.md`

Exit criteria:

- GUI Shell is clearly defined as a generic shell.
- BLUE-TANUKI-specific logic is prohibited from Shell Core.
- Flutter risk and migration conditions are documented.
- Phase 0 claim boundary is explicit.

### Phase 1: Schema / Contract Closure

Goal: Define all core contracts before UI implementation.

Deliverables:

- `specs/runtime.schema.json`
- `specs/adapter.schema.json`
- `specs/capability.schema.json`
- `specs/permission.schema.json`
- `specs/approval.schema.json`
- `specs/audit.schema.json`
- `specs/recovery.schema.json`
- `specs/diagnostic.schema.json`
- `specs/update.schema.json`
- `specs/content_exposure.schema.json`
- `specs/framework_risk_profile.schema.json`
- `tooling/schema_check/check_schemas.py`

Required invariants:

- All schemas must contain `$schema`, `$id`, `title`, and `type`.
- Adapter metadata must be untrusted.
- `authority_strip=true` must be required for adapters.
- Content visibility must support `none`, `hash_only`, `summary`, `redacted`, and `full`.
- Approval payloads must use tagged SHA-256 hashes.
- Framework risk must be explicitly represented.

Exit criteria:

```bash
python3 tooling/schema_check/check_schemas.py
```

passes.

### Phase 2: Conformance Closure

Goal: Prevent product UI from outrunning safety contracts.

Deliverables:

- `tooling/conformance_tests/run_conformance_skeleton.py`
- `docs/specs/adapter-conformance.md`
- `docs/specs/content-exposure-policy.md`
- `docs/specs/approval-visibility-boundary.md`
- `docs/specs/authority-strip-conformance.md`

Required conformance checks:

- Inbound authority keys are stripped.
- External metadata cannot escalate authority.
- GUI input cannot create runtime-disallowed authority context.
- Memory, cache, and previous state cannot grant authority by themselves.
- Full content cannot be displayed unless `content_visibility=full`.
- `authority_fields`, `sealed_fields`, `hidden_fields`, and `sacred_fields` cannot be edited.
- Edited approval payloads are rehashed and revalidated.
- Sensitive actions must map to capability, permission, approval state, AuditEvent, and RecoveryAction on failure.

Exit criteria:

```bash
python3 tooling/conformance_tests/run_conformance_skeleton.py
```

passes with meaningful failure-case coverage.

### Phase 3: Shell Core Skeleton

Goal: Build framework-independent Shell Core.

Deliverables:

- `packages/shell_core/`
- `packages/shell_contracts/`
- `packages/shell_ui/` only for framework-neutral UI state abstractions
- Runtime Registry
- Adapter Loader
- Permission Ledger
- Approval Queue
- Audit Store
- Recovery Catalog
- Update Policy Store

Required rules:

- Shell Core must not import Flutter.
- Shell Core must not import BLUE-TANUKI internals.
- Shell Core must not trust adapter metadata.
- Shell Core must not treat memory/cache as authority.
- Shell Core must expose deterministic state snapshots for inspection.

Exit criteria:

- Core tests pass.
- Sensitive action routing is testable without Flutter.
- BLUE-TANUKI adapter can be developed without changing Shell Core.

### Phase 4: Rust Helper Boundary

Goal: Add bounded native capabilities without creating hidden authority.

Deliverables:

- `native/rust_helper/`
- process diagnostics
- filesystem diagnostics
- network diagnostics
- update verification
- audit hashing
- secure IPC
- structured helper responses

Required rules:

- Rust helper must not become an independent authority path.
- Every helper action must be capability-scoped.
- Every sensitive helper action must require permission and approval linkage.
- Helper outputs must be schema-validated.
- Helper failures must map to RecoveryAction.

Exit criteria:

```bash
cd native/rust_helper && cargo test
```

passes when Rust is installed.

### Phase 5: BLUE-TANUKI Reference Adapter

Goal: Connect BLUE-TANUKI as the first runtime through adapter only.

Deliverables:

- `packages/blue_tanuki_adapter/`
- health adapter
- ready adapter
- runtime snapshot adapter
- authority trace adapter
- notification adapter
- approval adapter
- audit export adapter
- diagnostics adapter
- recovery adapter

Required rules:

- Do not change BLUE-TANUKI Core for GUI Shell convenience.
- Do not import BLUE-TANUKI internals into Shell Core.
- BLUE-TANUKI adapter must normalize runtime state into generic GUI Shell schemas.
- Runtime-specific concepts must remain inside the adapter layer.

Exit criteria:

- Adapter conformance tests pass.
- BLUE-TANUKI state can be displayed generically.
- No BLUE-TANUKI-specific authority logic exists in Shell Core.

### Phase 6: Desktop Flutter Operator Shell

Goal: Build the first visible operator shell.

Deliverables:

- `apps/desktop_flutter/`
- Dashboard
- Setup Doctor
- Runtime Center
- Permission Center
- Approval Center
- Audit Viewer
- Recovery Center
- Settings
- Runtime Invariants surface

Required rules:

- Flutter owns rendering only.
- Flutter must not define permission semantics.
- Flutter must not define audit semantics.
- Flutter must not grant authority.
- Flutter must not display full content unless contract permits it.
- Flutter UI actions must go through Shell Core APIs.

Exit criteria:

```bash
cd apps/desktop_flutter && flutter analyze
```

passes when Flutter is installed.

### Phase 7: Installer / First-Run Path

Goal: Make the shell usable without exposing low-level complexity.

Deliverables:

- `installer/windows/`
- `installer/macos/`
- `installer/linux/`
- first-run wizard
- environment diagnostics
- dependency checks
- runtime connection checks
- recovery instructions

Required rules:

- Do not expose CLI/WSL/npm/Git complexity to normal users as the primary path.
- Setup Doctor must explain failures in operator language.
- Installation must not silently grant permissions.
- Installer state must not become authority.

Exit criteria:

- A non-expert user can install, launch, and see runtime state through the app path.
- Failures are classified and recoverable.

### Phase 8: Mobile Shell / Companion

Goal: Add mobile participation without bypassing desktop authority.

Deliverables:

- `apps/mobile_flutter/`
- device pairing
- notification view
- approval review
- runtime status
- emergency stop request
- recovery instruction view

Required rules:

- Mobile must not bypass Shell Core.
- Mobile approvals must preserve field visibility and edit constraints.
- Mobile device identity must be explicit.
- Device pairing must be auditable.

Exit criteria:

- Mobile can observe and approve within policy.
- Mobile cannot create hidden authority paths.

### Phase 9: Release Hardening

Goal: Prepare OSS release with explicit claim boundary.

Deliverables:

- release checklist
- security review
- license verification
- signed build plan
- update verification
- compatibility matrix
- conformance report
- audit evidence bundle

Exit criteria:

- Owner explicitly approves release claim.
- All applicable validations pass.
- Public README claim matches actual implementation state.

## 6. Current Claim Boundary

Until later promotion, GUI Shell only claims:

- desktop-first AI Runtime / Agent Operation Shell skeleton with v1.0 product-completion scaffolding
- schema-first contracts
- conformance-first work order
- Flutter + Rust helper as first implementation candidate
- BLUE-TANUKI as reference runtime through adapter only
- framework-independent core assets for permission, approval, audit, recovery, policy evaluation, deterministic state snapshots, and content exposure

It does not yet claim:

- production readiness
- signed installer readiness
- stable mobile readiness
- complete BLUE-TANUKI integration
- complete Rust helper implementation
- complete Flutter product UI
- security completeness

## 7. Required Validation

Minimum validation before every completed work report:

```bash
python tooling/schema_check/check_schemas.py
python tooling/conformance_tests/run_conformance_skeleton.py
```

If `python` is unavailable:

```bash
python3 tooling/schema_check/check_schemas.py
python3 tooling/conformance_tests/run_conformance_skeleton.py
```

If Rust is installed:

```bash
cd native/rust_helper && cargo test
```

If Flutter is installed:

```bash
cd apps/desktop_flutter && flutter analyze
cd apps/mobile_flutter && flutter analyze
```

Aggregate reporter:

```bash
python3 tooling/validate_all.py
```

## 8. Release Rule

Do not claim release readiness until:

- schema validation passes
- conformance validation passes
- Rust helper tests pass when applicable
- Flutter analysis passes when applicable
- sensitive action audit evidence exists
- installer behavior is verified
- owner explicitly approves release promotion
