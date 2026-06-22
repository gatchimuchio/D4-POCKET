# GUI-Shell Specification v1.0

Status: living implementation specification
Scope: desktop-first Runtime Operation Shell / LLM-readable implementation constraint
Release claim: this specification does not claim completed product release readiness.

This document is the normative implementation specification for GUI-Shell v1.0. Phase 0 Lock documents freeze technology selection and design posture; this document fixes the implementation constraints that Codex, external LLMs, and third-party implementers must preserve when extending GUI-Shell.

## 1. Purpose

GUI-Shell is a desktop-first Runtime Operation Shell for operating AI runtimes, agents, tools, and local services through a unified GUI.

It is not a BLUE-TANUKI-specific GUI.

BLUE-TANUKI is the first reference runtime, not the shell core.

## 2. Scope

GUI-Shell provides:

- Runtime discovery
- Runtime launch / stop / restart
- Runtime status monitoring
- Permission management
- Approval queue
- Audit log viewing
- Recovery operations
- Setup Doctor
- Installer-based first-run experience
- Adapter-based runtime integration

GUI-Shell does not implement runtime intelligence itself.

## 3. Non-goals

GUI-Shell must not:

- Embed BLUE-TANUKI-specific logic into Shell Core
- Treat GUI input as authority
- Store secrets without explicit boundary
- Expose runtime content beyond declared visibility policy
- Bypass adapter contracts
- Depend on Flutter for core system semantics

## 4. Architecture

GUI-Shell consists of:

- Flutter UI layer
- Shell Core
- Runtime Registry
- Adapter Loader
- Permission Ledger
- Approval Queue
- Audit Store
- Recovery Center
- Rust Native Helper / Rust Security Broker
- Runtime Adapters

Core assets must remain framework-independent.

## 5. Runtime Model

A Runtime is an external executable, service, agent, or local process controlled through an Adapter.

Each Runtime must expose:

- id
- name
- version
- status
- health
- capabilities
- permissions
- approval requirements
- audit events
- diagnostics
- recovery actions

## 6. Adapter Contract

All Runtime integration must pass through an Adapter.

Adapter responsibilities:

- health check
- ready check
- runtime snapshot
- capability declaration
- permission declaration
- approval request emission
- audit event emission
- diagnostic export
- recovery action execution

Shell Core must not call runtime internals directly.

## 7. Capability Model

Capabilities define what a Runtime or Adapter can do.

Examples:

- process_control
- filesystem_read
- filesystem_write
- network_access
- browser_control
- external_api_call
- local_model_execution
- secret_access
- approval_required_action

Capabilities are declarative.

Possessing a capability does not imply permission.

## 8. Permission Model

Permissions are granted by GUI-Shell policy.

Rules:

- default deny
- explicit grant required
- permission changes must be audited
- dangerous permissions require approval
- permission scope must be visible to user
- adapter cannot self-escalate permissions

## 9. Approval Model

Approval is required for actions that affect:

- external services
- filesystem mutation
- process execution
- network calls
- secret usage
- destructive operations
- authority-bearing decisions

Approval records must include:

- request id
- runtime id
- adapter id
- action type
- requested capability
- visible summary
- content visibility level
- user decision
- timestamp
- audit hash

## 10. Content Exposure Boundary

GUI-Shell must respect content visibility.

Allowed visibility levels:

- none
- hash_only
- summary
- redacted
- full

Rules:

- none: raw content must not be shown
- hash_only: only hash is shown
- summary: only adapter-approved summary is shown
- redacted: only redacted content is shown
- full: full content may be shown

## 11. Authority Strip

GUI-Shell must not accept external authority claims as-is.

Rules:

- strip inbound authority keys
- reject external authority escalation
- do not generate authority_context unless runtime permits it
- GUI input is not authority
- adapter metadata is not authority by default

## 12. Audit Model

Every meaningful state transition must produce an AuditEvent.

Audit target examples:

- runtime start
- runtime stop
- permission grant
- permission revoke
- approval approve
- approval reject
- recovery execution
- adapter error
- setup doctor result
- installer verification
- config generation
- content visibility decision

Audit events must be append-only.

## 13. Recovery Model

RecoveryAction defines safe repair procedures.

Examples:

- restart runtime
- regenerate config
- repair permission file
- clear invalid cache
- re-run setup doctor
- export diagnostic bundle
- reinstall runtime component

Recovery actions must be declared by adapter or shell policy.

## 14. Rust Helper Boundary

Rust helper owns native-risk operations.

Responsibilities:

- process control
- filesystem diagnostics
- port checks
- hash/signature utilities
- update verification
- secure IPC
- platform-specific diagnostics

Flutter must not directly perform dangerous native operations.

## 15. UI Layer Responsibilities

Flutter UI owns:

- screen rendering
- navigation
- user input
- theme
- localization
- accessibility
- visual state

Flutter UI must not own:

- permission semantics
- approval semantics
- audit format
- runtime contract
- recovery contract
- adapter conformance
- authority rules

## 16. Setup Doctor

Setup Doctor validates:

- install path
- runtime executable presence
- config generation
- writable audit directory
- adapter availability
- port availability
- native helper availability
- runtime health
- UI launchability

Windows acceptance must include:

- installer completes
- app launches from installed path
- MainWindowHandle is non-zero
- UIA-visible window exists
- config file is generated
- audit write succeeds

## 17. Installer Requirement

The target product experience is:

```text
installer complete -> app launch -> setup doctor -> runtime ready -> usable GUI
```

Low-level setup must not be pushed to normal users.

## 18. BLUE-TANUKI Adapter

BLUE-TANUKI is the first reference runtime.

Rules:

- BLUE-TANUKI Core is not rewritten for GUI-Shell
- connection occurs through adapter
- BLUE-TANUKI-specific concepts must not leak into Shell Core
- Shell Core treats BLUE-TANUKI as one Runtime among many

## 19. Conformance

A valid GUI-Shell implementation must pass:

- schema validation
- adapter conformance tests
- authority strip tests
- content exposure tests
- permission model tests
- approval flow tests
- audit append tests
- recovery action tests
- Windows installed smoke tests

Windows installed smoke tests are release evidence, not proof supplied by this document.

## 20. Repository Layout

Recommended structure:

```text
gui-shell/
  docs/
    specs/
      gui-shell-spec-v1.md
      adapter-conformance.md
      content-exposure-policy.md
      approval-visibility-boundary.md
      authority-strip-conformance.md
      runtime-catalog.md
  specs/
    runtime.schema.json
    adapter.schema.json
    capability.schema.json
    permission.schema.json
    approval.schema.json
    audit.schema.json
    recovery.schema.json
    diagnostic.schema.json
  apps/
    desktop_flutter/
  packages/
    shell_core/
    shell_contracts/
    shell_ui/
    blue_tanuki_adapter/
  native/
    rust_helper/
  installer/
    windows/
    linux/
    macos/
  tooling/
    schema_check/
    conformance_tests/
    ui_snapshot_tests/
```

## 21. Implementation Order

Required order:

1. Write GUI-Shell specification
2. Define schemas
3. Define adapter conformance
4. Define audit format
5. Define permission / approval model
6. Implement shell core skeleton
7. Implement Rust helper skeleton
8. Implement Flutter UI shell
9. Implement BLUE-TANUKI adapter
10. Implement Windows installer
11. Run installed acceptance tests

UI must not precede contracts.

## 22. Acceptance Criteria

GUI-Shell v1 is acceptable when:

- Windows installer works
- installed app launches
- setup doctor runs
- BLUE-TANUKI adapter connects
- runtime status is visible
- approval queue works
- permission center works
- audit viewer works
- recovery center works
- native helper is used for risky operations
- conformance tests pass
- no BLUE-TANUKI-specific logic exists in Shell Core

These criteria are acceptance targets. Completed product release also requires all active `release_blocker` items to be closed and explicit owner GO.
