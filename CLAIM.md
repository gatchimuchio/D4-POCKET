# GUI Shell Claim Boundary

## Current Status

GUI-Shell is not yet a completed product release.

Current claim: PC-first AI Runtime / Agent Operation Shell with Phase B owner-use completion.

Repository definition update: GUI-Shell is now also documented as an LLM-readable application responsibility substrate. This means LLM development / integration agents are intended to read GUI Shell contracts and use them as first-class implementation and integration surfaces. LLMs remain non-authoritative; human operators retain final approval, recovery, responsibility, and release-claim authority.

The canonical completion roadmap for aligning Windows-first product responsibility and LLM-readable substrate demonstration is `docs/implementation/GUI_SHELL_LLM_SUBSTRATE_COMPLETION_ROADMAP.md`.

Phase A, personal Windows trial operation, is complete: the Windows desktop build and native launch smoke passed for owner-trial history. That historical PASS is invalid for current strict R2 formal evidence because the old path predated the isolated provenance/evidence-bundle contract and aggregate native surface shortcut ban. Phase B owner-use completion is complete: the owner can use the desktop shell for daily local operation with visible status, problems, evidence, recovery, trust, runtime, and authority surfaces. External claim hygiene, measured Windows release evidence, OSS release candidate claims, and paid/product QC remain later phases.

GUI-Shell v1.0 is Windows-first. Current-host Linux validation can pass as a development/verification slice, but it is not final product proof by itself. macOS is an unverified planned portability target, and BLUE-TANUKI remains a consumer/reference runtime rather than a GUI-Shell release dependency.

GUI-Shell v1.0 does not claim verified macOS support. macOS support must not be advertised as supported, ready, or complete without validation evidence from a macOS host.

The LLM-readable substrate definition, bounded reference extension conformance, and one bounded cross-agent reproduction report demonstrate controlled LLM-readable extension behavior for a non-authoritative task. They do not prove public standard adoption, broad third-party interoperability, installed-product behavior, or ecosystem readiness. They do not close any current Windows-first product release blocker.

## Current Completed Areas

- item: schema and conformance skeleton
  classification: required_for_v1
  status: current development validation passes with 138 conformance checks; historical check-count entries remain preserved in `VALIDATION.txt`; conformance tautology fix resolved by testing production authority stripping and ApprovalQueue behavior; ghost invariants are measured by production InvariantEvaluator; normalization firewall conformance now covers PolicyEvaluator and adapter metadata ingress; broker IPC contracts, static no-FFI/no-Python-spawn assertions, structured release blocker registry, release-facing blocker/doc synchronization, packaging portability, and bounded LLM-readable extension contract/conformance checks are covered.

- item: bounded cross-agent LLM-readable extension reproduction
  classification: required_for_v1
  status: `docs/evidence/LLM_CROSS_AGENT_REPRODUCTION_REPORT.md` records two independent agent executions from baseline `48082469089e9a63ef939b51f864dfc26e4ae2c9` producing the same bounded `model_output` non-authority-source diff with validation passing; this is limited to the controlled task and does not prove public standard adoption, broad interoperability, or installed-product behavior.

- item: personal Windows trial operation
  classification: required_for_v1
  status: Windows build and native launch smoke passed for owner trial use; this does not satisfy completed product release readiness.

- item: Flutter local Shell Core client
  classification: required_for_v1
  status: `ShellCoreClient.local()` reads structured local snapshot JSON and is no longer a direct mock alias; mock mode remains available for tests/demo.

- item: GUI operation surfaces
  classification: required_for_v1
  status: Trust Center, Authority Map, Audit Timeline, Recovery Playbook, Adapter Catalog, Permission Diff, Problems Panel, Evidence Center, Settings UX, Command Palette, and Status Bar vocabulary are present as Shell Core-bound operator surfaces.

- item: Shell snapshot and evidence bundle
  classification: required_for_v1
  status: `tooling/shell_snapshot.py` provides structured local GUI state for owner-use migration / development evidence and `tooling/evidence_bundle.py --check` validates a development evidence bundle while preserving Windows installed-path and audit anchor external tamper-evidence blockers with `release_ready=false`; the snapshot generator must not remain an installed product runtime dependency.

- item: Shell Core hardening skeleton
  classification: required_for_v1
  status: contract-level implementation present

- item: Runtime Catalog skeleton
  classification: required_for_v1
  status: schema, fixtures, package, and conformance present

- item: Agent Runtime skeleton
  classification: required_for_v1
  status: schema, fixtures, package, and conformance present

- item: Rust helper boundary skeleton
  classification: required_for_v1
  status: helper implementation and Rust Security Broker skeleton present; `cd native/rust_helper && cargo test` passed with broker JSON envelope/rejection tests on 2026-06-01

- item: Desktop Flutter skeleton
  classification: required_for_v1
  status: implementation present; `cd apps/desktop_flutter && flutter analyze`, `flutter test`, `flutter build linux`, and Linux launch smoke passed on 2026-05-25

- item: Setup Doctor skeleton
  classification: required_for_v1
  status: implementation present

- item: release-hardening documents
  classification: required_for_v1
  status: implementation present

## Current Release Blockers

- item: language policy runtime convergence not passed
  classification: release_blocker
  aggregate_of: windows_evidence_provenance_isolation, windows_installer_first_run_smoke, windows_broker_installed_smoke
  reason: Rust Security Broker skeleton and JSON envelope tests exist, but current Shell Core authority-sensitive behavior is still implemented in Python under `packages/shell_core/*.py` and used by owner-use snapshot generation / validation. Production IPC transport, Flutter broker-mediated authority path, no-Python-runtime product evidence, and no-FFI-authority release assertion are not complete.
  required_action: Migrate authority-sensitive active runtime responsibilities to the Rust Security Broker, keep Python only as dev/test/migration oracle, and prove Flutter authority operations are broker-mediated before completed product release.
  blocks_release: yes

- item: Linux desktop build and launch smoke
  classification: required_for_v1
  reason: Linux desktop build smoke and launch smoke passed on 2026-05-25 as development/verification proof.
  required_action: Keep Linux build and launch smoke passing, but do not treat them as Windows-first product proof.
  blocks_release: no

- item: Windows installer, first-run, and Setup Doctor release validation not passed
  classification: release_blocker
  aggregate_of: windows_evidence_provenance_isolation, windows_installer_first_run_smoke, windows_setup_doctor_smoke, windows_broker_installed_smoke
  reason: Windows project support and historical owner-trial launch smoke are preserved, but current strict R2 evidence requires a fresh native Windows isolated installed run with source commit, clean worktree state, app/broker artifact hashes, evidence bundle hashes, UIAutomation diagnostic tree, broker measured field provenance, and installed-app generated Setup Doctor product export. The product export path exists, but `release_evidence/windows_installed_smoke.json` is missing.
  required_action: Run native Windows installed smoke collection from an isolated staged run, collect measured window, visible-surface diagnostic tree, config JSON, audit write/read/delete, broker IPC/restart/crash field provenance, and installed-app generated Setup Doctor product evidence; pass `python tooling\windows_release_evidence.py`.
  blocks_release: yes

- item: macOS planned portability target unverified
  classification: known_limitation
  reason: no macOS validation environment is currently available, so GUI-Shell v1.0 does not claim verified macOS support.
  required_action: Validate on a macOS host before claiming macOS support.
  blocks_release: no

- item: Windows Setup Doctor diagnostics not passed
  classification: release_blocker
  registry_id: windows_setup_doctor_smoke
  reason: The installed app supports machine-readable Setup Doctor product export, but that evidence has not been collected on native Windows and passed through the strict validator. The PowerShell Setup Doctor collector remains external probe evidence only.
  required_action: Collect app-generated Setup Doctor product export through isolated Windows installed smoke and pass `python tooling\windows_release_evidence.py`.
  blocks_release: yes

- item: audit anchor external tamper-evidence proof missing
  classification: release_blocker
  registry_id: audit_anchor_external_tamper_evidence_proof
  reason: Local HMAC audit anchors detect local corruption and partial tamper, but same-user or administrator/root rewrite resistance is not proven without Windows ACL/DPAPI, an external anchor, or signed evidence.
  required_action: Collect Windows installed-path audit anchor key-protection or external-anchor proof and pass strict Windows release validation.
  blocks_release: yes

- item: implementation first-run smoke
  classification: required_for_v1
  reason: implementation first-run smoke creates config/audit paths and verifies installer/setup state is non-authoritative.
  required_action: Keep implementation first-run smoke passing; native Windows installed-path first-run remains a release blocker.
  blocks_release: no

- item: Shell Core persistence smoke
  classification: required_for_v1
  reason: integrated Shell Core release smoke saves and loads state snapshots.
  required_action: Keep persistence smoke passing on release candidates.
  blocks_release: no

- item: Audit chain verification smoke
  classification: required_for_v1
  reason: integrated Shell Core release smoke verifies audit chain linkage, HMAC audit anchor verification, and tamper detection.
  required_action: Keep audit chain and local anchor smoke passing on release candidates.
  blocks_release: no

- item: Runtime Catalog live/use smoke
  classification: required_for_v1
  reason: release smoke registers runtime and adapter manifests through RuntimeCatalog and confirms catalog authority remains false.
  required_action: Keep Runtime Catalog smoke passing.
  blocks_release: no

- item: Agent Runtime mock/reference smoke
  classification: required_for_v1
  reason: release smoke validates workspace boundary, secret path denial, shell command permission mapping, and auditable diff behavior.
  required_action: Keep Agent Runtime reference smoke passing.
  blocks_release: no

- item: Strict release validation not passed
  classification: release_blocker
  aggregate_of: windows_evidence_provenance_isolation, windows_installer_first_run_smoke, windows_setup_doctor_smoke, windows_broker_installed_smoke, audit_anchor_external_tamper_evidence_proof, owner_go
  reason: completed Windows-first product release requires Windows strict validation.
  required_action: Pass `python3 tooling/validate_all.py --strict-release --desktop-platform=windows`; `--desktop-platform=all` may still fail because macOS is unverified, but that does not block Windows-first v1.0.
  blocks_release: yes

- item: Owner GO missing
  classification: release_blocker
  registry_id: owner_go
  reason: release claim promotion requires owner approval.
  required_action: Obtain explicit owner GO.
  blocks_release: yes

## Post-v1 Scope

- item: Mobile full release
  classification: post_v1_scope
  reason: v1.0 scope is Windows-first PC desktop unless owner explicitly includes mobile.
  required_action: Complete after v1.0 or update scope by owner instruction.
  blocks_release: no

- item: Multi-user mode
  classification: post_v1_scope
  reason: v1.0 is single-user.
  required_action: Defer until post-v1 planning.
  blocks_release: no

- item: Cloud service
  classification: post_v1_scope
  reason: v1.0 is local-first.
  required_action: Defer until post-v1 planning.
  blocks_release: no

- item: Marketplace
  classification: post_v1_scope
  reason: v1.0 does not include runtime marketplace distribution.
  required_action: Defer until post-v1 planning.
  blocks_release: no

- item: Enterprise admin
  classification: post_v1_scope
  reason: v1.0 is single-user desktop.
  required_action: Defer until post-v1 planning.
  blocks_release: no

- item: Full live third-party agent integrations
  classification: post_v1_scope
  reason: v1.0 requires generic Agent Runtime contract and mock/reference agent, not all live adapters.
  required_action: Add after v1.0 as adapter work.
  blocks_release: no

- item: BLUE-TANUKI product completion
  classification: post_v1_scope
  reason: BLUE-TANUKI is a consumer/reference runtime, not a GUI-Shell release gate.
  required_action: Complete as consumer integration after GUI-Shell v1.0 gate.
  blocks_release: no

## Known Limitations

- item: local single-user only
  classification: known_limitation
  reason: v1.0 product scope is Windows-first PC desktop, single-user, local-first.
  required_action: Keep README, CLAIM, and RELEASE_CHECKLIST aligned.
  blocks_release: no
