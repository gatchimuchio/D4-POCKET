# Audit Evidence

## Current Evidence

- item: AuditEvent schema requires core audit fields
  classification: required_for_v1
  status: schema validation present

- item: PolicyEvaluator requires audit_event.event_id
  classification: required_for_v1
  status: conformance present

- item: PolicyEvaluator requires audit_event.payload_hash when payload exists
  classification: required_for_v1
  status: conformance present

- item: AuditStore maintains previous hash linkage in memory
  classification: required_for_v1
  status: contract-level implementation present

- item: audit chain tamper detection
  classification: required_for_v1
  status: conformance and release smoke present, including local HMAC audit anchor verification

- item: Shell Core persistence and audit smoke
  classification: required_for_v1
  status: `tooling/release_smoke.py` passes snapshot save/load, append-only audit chain verification, HMAC audit anchor verification, and tamper detection for the current implementation path.

- item: MANIFEST integrity coverage
  classification: required_for_v1
  status: `python3 tooling/manifest.py --check` passes and is included in `tooling/release_gate_check.py`.
  reason: MANIFEST covers Shell Core, tooling, schemas, desktop Flutter, Rust helper, root governance/release docs, and docs. MANIFEST does not claim completed product release readiness.
  required_action: Keep `MANIFEST.sha256.json` current with `python3 tooling/manifest.py --write`; it excludes itself from its own file list.
  blocks_release: no

## Phase B Internal Operation

- item: operator-facing audit chain validation
  classification: required_for_v1
  reason: Phase B owner operation needs audit status visible from the desktop shell.
  required_action: Keep Audit Timeline and Evidence Center surfaces connected to Shell Core snapshot/evidence data.
  blocks_release: no

- item: audit export verification tooling
  classification: required_for_v1
  reason: evidence bundle export exists for development evidence and must remain non-authoritative until measured Windows installed-path evidence passes.
  required_action: Keep `tooling/evidence_bundle.py --check` passing.
  blocks_release: no

## Remaining Release Blockers

- item: measured Windows installed-path audit evidence
  classification: release_blocker
  aggregate_of: windows_evidence_provenance_isolation, windows_installer_first_run_smoke
  reason: completed product release still requires native Windows installed-path evidence to prove config/audit initialization from the installed app path.
  required_action: Generate measured `release_evidence/windows_installed_smoke.json` on native Windows and pass `python tooling/windows_release_evidence.py`.
  blocks_release: yes

- item: installed-app Setup Doctor product evidence
  classification: release_blocker
  registry_id: windows_setup_doctor_smoke
  reason: Native Windows launch smoke is development evidence. Strict R2 requires installed-app generated Setup Doctor product evidence from an isolated native Windows run; the current PowerShell Setup Doctor collector is external probe evidence only.
  required_action: Record installed-app generated Setup Doctor export evidence through isolated Windows installed smoke. Strict release must still fail until measured `windows_installed_smoke.json`, Setup Doctor product evidence, and owner GO exist.
  blocks_release: yes

- item: audit anchor external tamper-evidence proof
  classification: release_blocker
  registry_id: audit_anchor_external_tamper_evidence_proof
  reason: Local HMAC audit anchor verification does not prove same-user or administrator/root rewrite resistance by itself.
  required_action: Record Windows ACL/DPAPI, external anchor, or signed-evidence proof for audit anchor files and pass strict Windows release validation.
  blocks_release: yes
