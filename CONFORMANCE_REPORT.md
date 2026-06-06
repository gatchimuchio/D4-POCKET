# Conformance Report

## Current Conformance

- item: schema check
  classification: required_for_v1
  status: passed
  evidence: `schema check passed: 26 schemas, 26 examples, 28 negative fixtures`

- item: conformance checks
  classification: required_for_v1
  status: passed
  evidence: `conformance skeleton passed: 138 checks`

- item: conformance tautology fix
  classification: required_for_v1
  status: resolved
  evidence: authority stripping and approval edit guard checks now import and exercise production Shell Core implementations; see `docs/MUTATION_VERIFICATION.md`.

- item: production authority strip mutation coverage
  classification: required_for_v1
  status: passed
  evidence: mutating `adapter_loader.strip_authority_keys` to return input unchanged caused conformance failure; mutation was reverted and final conformance passed.

- item: production approval guard mutation coverage
  classification: required_for_v1
  status: passed
  evidence: mutating `ApprovalQueue.can_edit` to always return `True` and mutating `ApprovalQueue.edit` to bypass protected-field guards both caused conformance failure; mutations were reverted and final conformance passed.

- item: duplicate authority key definitions
  classification: required_for_v1
  status: resolved
  evidence: `packages/shell_core/authority_keys.py` is the single production source for `AUTHORITY_KEYS`; remaining duplicate definitions are classified as `release_blocker`.

- item: ghost invariant measurement
  classification: required_for_v1
  status: resolved
  evidence: `packages/shell_core/state_snapshot.py` uses `InvariantEvaluator().evaluate()` instead of static invariant flags, and conformance checks intentional invariant violations.

- item: normalization firewall
  classification: required_for_v1
  status: implemented
  evidence: conformance covers `Trust_Level`, fullwidth `ｔｒｕｓｔ＿ｌｅｖｅｌ`, zero-width `trust\u200b_level`, `permissionGrant`, `admin_context`, nested frame metadata authority, value-only authority attempts, PolicyEvaluator adapter metadata normalization, and adapter metadata value-only rejection.

- item: Flutter local Shell Core client
  classification: required_for_v1
  status: implemented
  evidence: `ShellCoreClient.local()` reads structured local snapshot JSON instead of aliasing `mock()`, and Flutter tests verify local mode, Setup Doctor local diagnostics rendering, redacted content projection, and snapshot-sourced invariant flags.

- item: GUI operation surfaces
  classification: required_for_v1
  status: implemented
  evidence: conformance verifies Trust Center, Authority Map, Adapter Catalog, Permission Diff, Problems Panel, Evidence Center, Command Palette, Audit Timeline actions, Recovery Playbook vocabulary, and Status Bar are present in the desktop Flutter surface.

- item: Shell snapshot generator
  classification: required_for_v1
  status: implemented
  evidence: `tooling/shell_snapshot.py` generates structured local snapshot JSON for Flutter local mode, including trust records, authority map, adapter catalog, permission diffs, problems, evidence, settings, Setup Doctor checks, and non-authoritative installer flags.

- item: Evidence bundle export
  classification: required_for_v1
  status: implemented
  evidence: `tooling/evidence_bundle.py --check` validates a development evidence bundle that preserves release blocker metadata, keeps `release_ready=false`, and records Flutter/installer non-authority boundaries.

- item: structured release blocker registry
  classification: required_for_v1
  status: implemented
  evidence: strict release mode reads `release_blockers.registry.json` and fails on unresolved active blockers instead of raw `release_blocker` text in policy or historical logs.

- item: artifact portability check
  classification: required_for_v1
  status: implemented
  evidence: conformance verifies `tooling/packaging_portability_check.py` exists, rejects non-portable tracked paths, and runs manifest, conformance, and release gate after POSIX-locale `unzip` extraction.

- item: integrated Shell Core release smoke
  classification: required_for_v1
  status: passed
  evidence: production smoke covers save/load snapshot, append-only audit chain verification, HMAC audit anchor verification, tamper detection, approval edit rehash/revalidation, and recovery_id policy verification.

- item: first-run and Setup Doctor implementation smoke
  classification: required_for_v1
  status: passed
  evidence: `tooling/release_smoke.py` creates first-run config/audit paths, verifies audit writability, runs Setup Doctor, and confirms installer/setup state grants no authority and silently approves no permissions.

- item: Windows installed-path evidence validator
  classification: required_for_v1
  status: implemented
  evidence: conformance accepts only the strict R2 `release_evidence/windows_installed_smoke.json` shape and rejects missing provenance/isolation, external Setup Doctor probe as product evidence, missing installed executable confirmation, installer authority grants, authority-granting Setup Doctor checks, unmeasured/manual GUI visibility evidence, missing UIAutomation diagnostic tree, missing config/audit probes, synthetic Setup Doctor evidence, shallow one-check Setup Doctor payloads, aggregate native surface evidence, and broker top-level unmeasured authority declarations.

## Not Sufficient For Release

- item: cargo test gate
  classification: required_for_v1
  reason: Rust helper is in v1.0 scope and current validation passes.
  required_action: Keep `cd native/rust_helper && cargo test` passing.
  blocks_release: no

- item: desktop flutter analyze gate
  classification: required_for_v1
  reason: desktop app is in v1.0 scope and current validation passes.
  required_action: Keep `cd apps/desktop_flutter && flutter analyze` passing.
  blocks_release: no

- item: strict release validation not pass
  classification: release_blocker
  reason: completed product release requires strict release validation.
  required_action: Pass `python3 tooling/validate_all.py --strict-release`.
  blocks_release: yes

Conformance report must not imply production readiness.
