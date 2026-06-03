# GUI Shell

GUI Shell is a desktop-first AI Runtime / Agent Operation Shell and LLM-readable application responsibility substrate for local runtimes, agents, tools, and services.

It is a generic Runtime Operation Shell control plane whose contracts, safety boundaries, adapter model, approval model, audit model, recovery model, and extension rules are intended to be read by both human operators and LLM development/integration agents.

It is **not** a BLUE-TANUKI-specific GUI. BLUE-TANUKI is the first reference consumer/runtime and must connect through an adapter boundary; it is not a v1.0 release gate.

## TL;DR

1. **GUI Shell is a control plane and LLM-readable responsibility substrate.** Flutter renders operator surfaces; it does not own authority.
2. **Schemas and conformance own the contract.** Runtime, adapter, permission, approval, audit, recovery, and content exposure semantics are JSON Schema-first.
3. **LLMs may implement and integrate; they are not authority.** Human operators remain final approval, recovery, responsibility, and release-claim holders.
4. **Safety first, robustness second, product UI third.** Product screens never outrank authority strip, content exposure, approval, audit, and recovery boundaries.

## Phase 0 locked surface

- Generic Runtime Operation Shell direction
- BLUE-TANUKI is frozen as the Phase 0 reference runtime contract target through adapter only
- Flutter + Rust helper as primary implementation candidate
- Compose Multiplatform watchlist
- Tauri desktop-heavy fallback
- FrameworkRiskProfile for UI framework governance risk
- Adapter Conformance requirements
- Content Exposure Boundary
- Authority Strip Conformance
- Schema-first / conformance-first work order

## Explicit boundaries

- Shell Core must not contain BLUE-TANUKI-specific logic.
- Flutter-specific code must not define core contracts or authority decisions.
- Adapter metadata is untrusted and must never grant permissions.
- Memory, local cache, and previous state must never grant authority by themselves.
- Full content may be displayed only when `content_visibility=full`.
- Approval payload fields marked authority, sealed, hidden, or sacred are not editable.
- Sensitive actions must map to capability, permission, approval state, audit event, and recovery action.
- Network, filesystem, process, credential, and IPC access must not be silently introduced or broadened.

## Architecture

```text
Runtime / Agent / Tool / Local Service
  -> Adapter
      -> authority strip
      -> content exposure policy
      -> schema validation
      -> capability declaration
  -> Shell Core
      -> runtime registry
      -> permission ledger
      -> approval queue
      -> audit events
      -> recovery actions
  -> Rust Security Broker
      -> authority-sensitive IPC acceptance/rejection
      -> broker-local audit for broker decisions
      -> command-envelope validation before future dispatch
  -> UI Layer
      -> Flutter rendering
      -> operator input
      -> navigation and local UI state
  -> Rust Helper
      -> bounded native diagnostics and operations
```

The UI can display and request actions. It cannot create authority, bypass adapter conformance, or reinterpret runtime trust.

## Human Operator and LLM Extension-Agent Model

GUI Shell has two first-class interaction roles:

- Human operator / owner: observes state, grants or denies approval, authorizes recovery, accepts release claims, and remains the final responsibility holder.
- LLM development / integration agent: reads architecture and contract documents, implements bounded modules or adapters, connects runtimes/tools/services through declared contracts, runs validation, and reports evidence.

LLMs are first-class implementation and integration consumers of GUI Shell contracts, but are never authority sources.

LLM output, generated configuration, memory, external metadata, GUI state, adapter metadata, previous state, and tool responses must not grant authority, approve sensitive operations, widen permissions, bypass conformance, or turn generated content into trusted truth. Sensitive work remains governed by capability, permission, approval, audit, recovery, content exposure, adapter, runtime, update/install, and broker boundaries.

## Why LLM-Readable Contracts Matter

GUI Shell is intended to make application/runtime extension work inspectable before implementation begins. A development agent should be able to read repository contracts and understand:

- which contract surface a new module or adapter must consume;
- which authority, approval, audit, recovery, and content exposure boundaries apply;
- which failure case must reject, block, audit, or suspend behavior;
- which validation path proves conformance;
- when a proposed integration requires a new contract instead of an improvised shortcut.

This definition is not an external ecosystem claim. Cross-agent reproduction, public standard status, and third-party interoperability remain unproven until measured evidence exists.

## Quickstart

Validation checks the repository contracts and conformance skeleton. This skeleton does not assume Flutter or Rust is already installed.

```bash
python tooling/schema_check/check_schemas.py
python tooling/conformance_tests/run_conformance_skeleton.py
```

If the host only exposes Python as `python3`, use:

```bash
python3 tooling/schema_check/check_schemas.py
python3 tooling/conformance_tests/run_conformance_skeleton.py
```

Expected successful output:

```text
schema check passed: 25 schemas, 25 examples, 27 negative fixtures
conformance skeleton passed: 102 checks
```

See [QUICKSTART.md](./QUICKSTART.md).

## Validation

Required before reporting implementation work:

```bash
python tooling/schema_check/check_schemas.py
python tooling/conformance_tests/run_conformance_skeleton.py
```

Conditional toolchain checks:

```bash
cd native/rust_helper && cargo test
cd apps/desktop_flutter && flutter analyze
cd apps/mobile_flutter && flutter analyze
```

Or run the aggregate reporter:

```bash
python3 tooling/validate_all.py
```

See [VALIDATION.txt](./VALIDATION.txt) for the last recorded validation output.

## Repository layout

```text
docs/
  standards/
  research/
  specs/

specs/
  *.schema.json

apps/
  desktop_flutter/
  mobile_flutter/

packages/
  shell_core/
  shell_ui/
  shell_contracts/
  blue_tanuki_adapter/

native/
  rust_helper/

installer/
  windows/
  macos/
  linux/

tooling/
  codegen/
  schema_check/
  conformance_tests/
  ui_snapshot_tests/
```

## Current status

GUI-Shell has completed Phase A: personal Windows trial operation and Phase B: owner-use operational hardening.

Phase B owner-use complete means the owner can launch and use the desktop shell for daily local operation with visible status, problems, evidence, recovery, trust, runtime, and authority surfaces. It does not mean OSS v1.0 RC or paid/product completion.

The canonical phase source is [docs/PHASE_STRATEGY.md](./docs/PHASE_STRATEGY.md):

- Phase A: complete
- Phase B: owner-use complete
- Phase C: OSS claim hygiene next
- Phase D: measured Windows installed-path release evidence later
- Phase E: OSS v1.0 RC later
- Phase F: paid/product QC later

The canonical completion roadmap for the combined Windows-first product path and LLM-readable substrate demonstration path is [docs/implementation/GUI_SHELL_LLM_SUBSTRATE_COMPLETION_ROADMAP.md](./docs/implementation/GUI_SHELL_LLM_SUBSTRATE_COMPLETION_ROADMAP.md).

This repository does not claim completed product release readiness. Strict Windows installed-path evidence, measured Setup Doctor evidence, and explicit owner GO are classified as `release_blocker` before any release-ready claim.

## Release-Gate Classification

- item: personal Windows trial operation
  classification: required_for_v1
  reason: Windows build and native launch smoke passed for owner trial use.
  required_action: Keep the owner-use operation path usable while Phase C/D/E remain separate.
  blocks_release: no

- item: completed product release
  classification: release_blocker
  reason: strict Windows installed-path release validation and owner GO have not passed.
  required_action: Complete measured Windows installed-path evidence and all v1.0 release blockers in `RELEASE_CHECKLIST.md`.
  blocks_release: yes

- item: local single-user desktop scope
  classification: known_limitation
  reason: deliberate v1.0 product scope.
  required_action: Keep README.md, CLAIM.md, and RELEASE_CHECKLIST.md aligned.
  blocks_release: no

- item: mobile full release
  classification: post_v1_scope
  reason: v1.0 is desktop-first unless owner changes scope.
  required_action: Complete in mobile release phase.
  blocks_release: no

It intentionally prioritizes:

1. standards
2. specs
3. conformance boundaries
4. runtime adapter contracts
5. helper boundaries

before product UI.

## Top-level references

- [AGENTS.md](./AGENTS.md): repository agent rules
- [ROADMAP.md](./ROADMAP.md): phase roadmap and execution order
- [docs/implementation/GUI_SHELL_LLM_SUBSTRATE_COMPLETION_ROADMAP.md](./docs/implementation/GUI_SHELL_LLM_SUBSTRATE_COMPLETION_ROADMAP.md): canonical completion roadmap for product responsibility and LLM-readable substrate proof
- [docs/PHASE_STRATEGY.md](./docs/PHASE_STRATEGY.md): Phase A/B/C/D/E/F readiness boundaries
- [docs/OPERATING_MODEL.md](./docs/OPERATING_MODEL.md): repository flow, backup model, and validation gates
- [CLAIM.md](./CLAIM.md): current claim boundary
- [CONFIG.md](./CONFIG.md): configuration reference
- [AUDIT.md](./AUDIT.md): audit and invariant expectations
- [SECURITY.md](./SECURITY.md): security posture and reporting
- [TROUBLESHOOTING.md](./TROUBLESHOOTING.md): validation and setup failure guide
