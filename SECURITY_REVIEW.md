# Security Review

Current phase: Phase B owner-use complete. This file records security posture for internal operation and claim hygiene; it is not a paid/product QC sign-off.

## Established Boundaries

- item: Shell Core owns policy evaluation
  classification: required_for_v1
  status: contract-level checks present

- item: adapter metadata is untrusted
  classification: required_for_v1
  status: conformance checks present

- item: memory, cache, previous state, and local UI state cannot grant authority
  classification: required_for_v1
  status: conformance checks present

- item: Rust helper is diagnostic/framing/hash/signature-boundary only
  classification: required_for_v1
  status: source boundary checks present

- item: Flutter apps are operator surfaces only
  classification: required_for_v1
  status: source boundary checks present

## Release Blockers

- item: Windows installed-path evidence
  classification: release_blocker
  aggregate_of: windows_evidence_provenance_isolation, windows_installer_first_run_smoke, windows_setup_doctor_smoke, windows_broker_installed_smoke
  reason: completed Windows-first product release requires measured installed-path evidence for launch, config, audit, visible surfaces, isolated provenance, artifact hash linkage, broker measured field provenance, and installed-app generated Setup Doctor product diagnostics.
  required_action: Generate and validate `release_evidence/windows_installed_smoke.json` on native Windows.
  blocks_release: yes

- item: owner GO
  classification: release_blocker
  registry_id: owner_go
  reason: release claim promotion requires explicit owner approval after blockers are cleared.
  required_action: Obtain explicit owner GO before claiming completed product release.
  blocks_release: yes

## Current Required-for-v1 Evidence

- item: persistent audit storage smoke
  classification: required_for_v1
  reason: `tooling/release_smoke.py` passes snapshot save/load and append-only audit chain checks for the current implementation path.
  required_action: Keep release smoke passing and prove the same path from installed Windows evidence before release claim.
  blocks_release: no

- item: audit chain verification smoke
  classification: required_for_v1
  reason: `tooling/release_smoke.py` verifies audit chain linkage and tamper detection.
  required_action: Keep audit chain smoke passing.
  blocks_release: no

## Later Security QC

- item: signed update verification if update mechanism ships
  classification: post_v1_scope
  reason: update distribution is outside Phase B owner-use operation unless owner explicitly ships an update mechanism.
  required_action: Either exclude update mechanism from v1.0 or pass signed update verification tests.
  blocks_release: no

- item: installer behavior review
  classification: required_for_v1
  reason: Windows installed-path evidence validator now rejects synthetic, manual, shallow, or unmeasured evidence.
  required_action: Pass the hardened Windows evidence collector and validator before release claim.
  blocks_release: no

- item: dependency/license review
  classification: post_v1_scope
  reason: paid/product QC and broad third-party distribution require fuller dependency/license review than Phase B owner-use operation.
  required_action: Add dependency/license review before OSS release candidate or paid/product release.
  blocks_release: no

## Post-v1 Scope

- item: mobile cryptographic pairing
  classification: post_v1_scope
  reason: mobile full release is outside v1.0 unless owner changes scope.
  blocks_release: no

- item: enterprise admin security
  classification: post_v1_scope
  reason: enterprise admin is outside v1.0 scope.
  blocks_release: no

- item: cloud security
  classification: post_v1_scope
  reason: cloud service is outside v1.0 scope.
  blocks_release: no

## Known Limitations

- item: local single-user only
  classification: known_limitation
  reason: accepted v1.0 product scope.
  required_action: Keep README.md and CLAIM.md aligned.
  blocks_release: no
