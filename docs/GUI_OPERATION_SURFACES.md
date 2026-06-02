# GUI Operation Surfaces

Status date: 2026-05-26

GUI-Shell GUI hardening imports proven operation patterns without moving authority into Flutter. Flutter renders state and operator intent surfaces; Shell Core remains the authority boundary.

## Implemented Surfaces

- item: Trust Center
  classification: required_for_v1
  status: implemented
  evidence: desktop app exposes workspace/runtime/adapter/installer trust records with trusted/restricted/inherited/unknown states and blocked operations.
  authority_boundary: display-only unless a future Shell Core trust mutation operation grants capability, permission, approval, audit, and recovery mapping.

- item: Authority Map
  classification: required_for_v1
  status: implemented
  evidence: desktop app exposes Runtime -> Capability -> Permission -> Approval -> AuditEvent -> RecoveryAction mapping with warning/danger fields.
  authority_boundary: visual map only; it does not grant permissions.

- item: Audit Timeline
  classification: required_for_v1
  status: implemented
  evidence: Audit Viewer includes filters for runtime, adapter, approval, permission, setup_doctor, normalization, installer, and error/warning/blocked plus copy/export/verify/jump action vocabulary.
  authority_boundary: verify/export operations must be backed by Shell Core audit verification.

- item: Recovery Playbook
  classification: required_for_v1
  status: implemented
  evidence: Recovery Center includes severity, retry state, pre_check, action_steps, post_check, rollback, and audit/recovery mapping vocabulary.
  authority_boundary: no recovery action executes without Shell Core authorization.

- item: Adapter Catalog and Permission Diff
  classification: required_for_v1
  status: implemented
  evidence: Runtime Center renders adapter publisher/source/version/signature/hash, requested/granted/denied capabilities, trust status, risks, and permission diffs.
  authority_boundary: install/disable/quarantine/remove remain Shell Core operations.

- item: Settings UX
  classification: required_for_v1
  status: implemented
  evidence: Settings screen includes search filters, source/default/current/effective values, modified/dangerous/authority flags, reset/export vocabulary, and command palette vocabulary.
  authority_boundary: setting mutation is represented as Shell Core controlled operation.

- item: Problems Panel and Evidence Center
  classification: required_for_v1
  status: implemented
  evidence: Dashboard renders release blockers, problems, and evidence status; Setup Doctor renders installed-path evidence.
  authority_boundary: evidence display does not satisfy release readiness without machine-validated Windows installed-path evidence.

- item: Status Bar
  classification: required_for_v1
  status: implemented
  evidence: persistent status bar renders runtime status, trust status, pending approvals, audit chain status, network exposure, and release blocker count.
  authority_boundary: status bar is read-only.

- item: Shell snapshot generator migration oracle
  classification: required_for_v1
  status: implemented
  evidence: `python3 tooling/shell_snapshot.py --write .gui_shell/shell_snapshot.json` creates local diagnostic JSON consumed only by `ShellCoreClient.local()` in development / inspection mode. Product `main.dart` uses `ShellCoreClient.product()` and broker IPC.
  authority_boundary: snapshot generation records Shell Core and Setup Doctor state for owner-use migration / development evidence; it does not grant authority and must not remain an installed product runtime dependency.

- item: Evidence bundle export
  classification: required_for_v1
  status: implemented
  evidence: `python3 tooling/evidence_bundle.py --check` verifies the bundle preserves Windows installed-path blockers, embeds `tooling/release_runtime_assertions.py --check`, and does not claim release readiness.
  authority_boundary: evidence export is read-only and non-authoritative.

- item: Release runtime assertions
  classification: required_for_v1
  status: implemented
  evidence: `python3 tooling/release_runtime_assertions.py --check` verifies product Flutter entry uses broker IPC, product path does not start Python or invoke Python snapshot generation, no Flutter/Rust FFI or direct bridge token exists in the authority surface scan, broker secret tokens are not projected to UI snapshots, and fail-closed / restart persistence test coverage is present.
  authority_boundary: assertion output is validation evidence only; Windows installed-path runtime proof remains required before completed product release.

## Remaining Release Blocker

- item: Windows installed-path evidence
  classification: release_blocker
  reason: `release_evidence/windows_installed_smoke.json` is still missing in this environment.
  required_action: collect native Windows installed-path evidence and pass `python tooling/windows_release_evidence.py`.
  blocks_release: yes
