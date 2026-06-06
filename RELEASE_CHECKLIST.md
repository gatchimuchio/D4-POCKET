# Release Checklist

In this repository, "release" means completed product release. Skeleton, preview, alpha, beta, scaffold, and contract-preview states are not release states.

No completed product release may be claimed if any Windows-first v1.0 `release_blocker` remains. GUI-Shell v1.0 is Windows-first: Windows is primary, Linux is the validated development/verification slice, and macOS is an unverified planned portability target.

GUI-Shell v1.0 does not claim verified macOS support. macOS support must not be advertised as supported, ready, or complete until validated on a macOS host.

Phase A personal Windows trial operation is complete. Phase B owner-use operational hardening is complete. This checklist remains the completed product release gate and must not be weakened for Phase B.

The canonical completion roadmap for the combined Windows-first product path and LLM-readable substrate demonstration path is `docs/implementation/GUI_SHELL_LLM_SUBSTRATE_COMPLETION_ROADMAP.md`.

## Release Blockers

- item: language policy runtime convergence gate
  classification: release_blocker
  aggregate_of: windows_evidence_provenance_isolation, windows_installer_first_run_smoke, windows_broker_installed_smoke
  reason: Rust Security Broker production IPC via authenticated loopback IPC, durable audit/replay/session store, Rust authority parity operations, Flutter product broker client code, and release runtime static assertions exist. Product `main.dart` now uses `ShellCoreClient.product()` instead of `ShellCoreClient.local()`, `tooling/release_runtime_assertions.py --check` proves the current product authority surface uses broker IPC without Python process startup and satisfies the no-ffi-authority direct-bridge assertion, and Windows Flutter analyze/test passed through `flutter.bat`. However command dispatch remains suspended, broker health still reports `authority_cutover_status=not_active`, WSL direct `flutter` still fails because the external Flutter SDK shell scripts have CRLF line endings, installed no-Python-runtime product evidence, and Windows installed-path broker evidence are not complete.
  required_action: Complete the migration plan in `docs/implementation/RUST_SECURITY_BROKER_MIGRATION_PLAN.md`, prove Python is dev/test/migration oracle only in installed product runtime, collect broker-mediated Windows installed-path evidence, and rerun strict release validation.
  blocks_release: yes

- item: cargo test gate for in-scope Rust helper
  classification: required_for_v1
  reason: Rust helper and Rust Security Broker skeleton validation are required for completed desktop-first v1.0 release. Current run on 2026-06-01 passed with broker JSON envelope and rejection tests.
  required_action: Pass `cd native/rust_helper && cargo test` on the release candidate.
  blocks_release: no

- item: desktop flutter analyze gate
  classification: required_for_v1
  reason: Desktop Flutter analyze is required for completed desktop-first v1.0 release. Current run on 2026-05-25 passed after `unzip` became available.
  required_action: Pass `cd apps/desktop_flutter && flutter analyze` on the release candidate.
  blocks_release: no

- item: Linux desktop build dependencies gate
  classification: required_for_v1
  reason: Rust/Cargo, Flutter, `unzip`, and Linux desktop build dependencies are resolved for the development/verification slice. `flutter doctor -v` reports clang 21.1.8, cmake 4.2.3, ninja 1.13.2, and pkg-config 2.5.1.
  required_action: Keep Linux desktop build dependencies installed for development validation; do not treat Linux as final Windows-first product proof.
  blocks_release: no

- item: Linux desktop project configuration gate
  classification: required_for_v1
  reason: Linux desktop project support is configured and `cd apps/desktop_flutter && flutter build linux` passed on 2026-05-25, producing `build/linux/x64/release/bundle/gui_shell_desktop`.
  required_action: Keep Linux build smoke passing as a development/verification slice.
  blocks_release: no

- item: Linux desktop launch smoke gate
  classification: required_for_v1
  reason: `./build/linux/x64/release/bundle/gui_shell_desktop` launched successfully under WSLg on 2026-05-25; the first window opened with Dashboard, NavigationRail, Runtime Status, and Invariant Status visible.
  required_action: Keep Linux desktop launch smoke passing, but complete Windows launch smoke before product release.
  blocks_release: no

- item: WSLg libEGL/MESA graphics warnings
  classification: known_limitation
  reason: WSLg emitted libEGL/MESA warnings during Linux desktop launch, but rendering and first-window stability did not fail.
  required_action: Keep documented in release-facing docs and reclassify as `release_blocker` if rendering or stability fails.
  blocks_release: no

- item: Windows desktop project support generated
  classification: required_for_v1
  reason: `flutter create --platforms=windows .` generated `apps/desktop_flutter/windows` without overwriting existing `lib/` app code.
  required_action: Keep Windows Flutter desktop project files under version control.
  blocks_release: no

- item: conformance tautology fix
  classification: required_for_v1
  reason: authority stripping, approval edit guard, approval status, and recovery ID conformance checks now call production Shell Core code and pass; mutation verification confirmed production authority strip and approval guard weakenings fail conformance.
  required_action: Keep conformance tests importing production implementations; do not reintroduce test-local authority stripping or approval edit guard copies; keep `docs/MUTATION_VERIFICATION.md` updated when this surface changes.
  blocks_release: no

- item: ghost invariant measurement
  classification: required_for_v1
  reason: state snapshot invariant flags now come from measured production `InvariantEvaluator` checks instead of static false values.
  required_action: Keep invariant flags measured and mutation-test intentional violations when invariant surfaces change.
  blocks_release: no

- item: normalization firewall
  classification: required_for_v1
  reason: Shell Core now normalizes inbound authority-bearing payloads before authority strip; PolicyEvaluator, AdapterLoader, RuntimeCatalog, and BLUE-TANUKI authority trace use shared normalization scanners; conformance covers Unicode, case, zero-width, alias, envelope, and value-only escalation attempts.
  required_action: Keep raw payload preservation, normalized projection, quarantine decision, normalization audit metadata, and metadata value-only rejection in authority-bearing ingress paths.
  blocks_release: no

- item: Flutter local Shell Core client
  classification: required_for_v1
  reason: `ShellCoreClient.local()` reads structured local snapshot JSON and is no longer a direct mock alias; mock mode remains separate for tests and demo data.
  required_action: Keep local snapshot loading covered by Flutter tests and replace fallback diagnostics with installed app data on release candidates.
  blocks_release: no

- item: GUI operation surfaces
  classification: required_for_v1
  reason: desktop Flutter now exposes Trust Center, Authority Map, Audit Timeline, Recovery Playbook, Adapter Catalog, Permission Diff, Problems Panel, Evidence Center, Settings UX, Command Palette, and Status Bar operation vocabulary without moving authority into Flutter.
  required_action: Keep GUI surfaces read-only or Shell Core-authorized and expand them only with corresponding conformance/evidence coverage.
  blocks_release: no

- item: Shell snapshot generator migration oracle
  classification: required_for_v1
  reason: `tooling/shell_snapshot.py` generates the structured local snapshot consumed by `ShellCoreClient.local()` for owner-use migration and development evidence, including trust, authority, evidence, settings, Setup Doctor, audit, recovery, and non-authoritative installer status. It must not remain an installed product runtime dependency.
  required_action: Keep snapshot generation aligned with Flutter model fields and Shell Core authority boundaries during migration, then replace product runtime dependency with broker-mediated state before completed product release.
  blocks_release: no

- item: evidence bundle export
  classification: required_for_v1
  reason: `tooling/evidence_bundle.py --check` validates a development evidence bundle that preserves Windows installed-path blockers, keeps `release_ready=false`, and embeds the release runtime assertions for broker-mediated Flutter authority, no Python authority process startup, and no FFI authority bridge.
  required_action: Keep evidence bundle export non-authoritative until Windows installed-path evidence and owner GO pass.
  blocks_release: no

- item: no-Python runtime / no-FFI authority assertion
  classification: required_for_v1
  reason: `tooling/release_runtime_assertions.py --check` is part of `tooling/validate_all.py` and verifies that product `main.dart` enters `ShellCoreClient.product()`, Flutter authority operations are broker-mediated, owner launch scripts start `broker-server` without Python snapshot generation, Flutter lib does not use Dart process-spawn APIs for authority, broker secrets are not projected to UI snapshots, and no Flutter/Rust FFI or direct bridge token appears in the authority surface scan.
  required_action: Keep release runtime assertions passing and extend them whenever a new authority-sensitive product surface is added.
  blocks_release: no

- item: duplicate authority key definitions
  classification: required_for_v1
  reason: `packages/shell_core/authority_keys.py` is the single production source of `AUTHORITY_KEYS`; any remaining duplicate authority key definition is a `release_blocker`.
  required_action: Keep production modules importing `packages.shell_core.authority_keys.AUTHORITY_KEYS`.
  blocks_release: no

- item: Windows Flutter analyze gate
  classification: required_for_v1
  reason: Historical Windows Flutter analyze passed on a native Windows host, but strict R2 release evidence requires current release-candidate validation tied to the exact implementation commit.
  required_action: Keep `cd apps/desktop_flutter && flutter analyze` passing on Windows release candidates and record current-run provenance before release promotion.
  blocks_release: no

- item: Windows Flutter test gate
  classification: required_for_v1
  reason: Historical Windows Flutter test passed on a native Windows host, but strict R2 release evidence requires current release-candidate validation tied to the exact implementation commit.
  required_action: Keep `cd apps/desktop_flutter && flutter test` passing on Windows release candidates and record current-run provenance before release promotion.
  blocks_release: no

- item: Windows Flutter toolchain verified
  classification: required_for_v1
  reason: Native Windows Flutter analyze, test, build, and launch smoke passed historically for owner-trial use. This is invalid for current strict R2 formal evidence until a fresh exact-commit Windows run is recorded.
  required_action: Keep Windows Flutter toolchain validation current on release candidates and bind it to the isolated evidence run.
  blocks_release: no

- item: Windows desktop build smoke
  classification: required_for_v1
  reason: `flutter build windows` passed on a native Windows host and produced `build\windows\x64\runner\Release\gui_shell_desktop.exe`.
  required_action: Keep Windows desktop build smoke passing on release candidates.
  blocks_release: no

- item: Windows desktop launch smoke
  classification: required_for_v1
  reason: `.\build\windows\x64\runner\Release\gui_shell_desktop.exe` launched successfully on native Windows as historical owner-trial evidence. The old launch smoke is invalid for current strict R2 proof because aggregate native surface exposure and missing exact-run provenance are forbidden.
  required_action: Keep Windows desktop launch smoke passing on release candidates, then recollect per-surface UIAutomation/accessibility evidence from the isolated installed run.
  blocks_release: no

- item: R2 Windows formal evidence path reset
  classification: release_blocker
  aggregate_of: windows_evidence_provenance_isolation, windows_installer_first_run_smoke, windows_setup_doctor_smoke, windows_broker_installed_smoke
  reason: Current strict Windows evidence now requires isolated run provenance, source commit, clean worktree state, app/broker artifact hashes, evidence bundle hashes, field provenance, full UIAutomation diagnostic tree, measured broker IPC/restart/crash fields, and installed-app generated Setup Doctor product export. Historical PASS and external probe reports are invalid for this gate.
  required_action: Complete the redesigned Windows evidence collection path and run strict Windows validation on native Windows.
  blocks_release: yes

- item: Windows installer first-run smoke not passed
  classification: release_blocker
  registry_id: windows_installer_first_run_smoke
  reason: Windows installed-path first-run evidence has not been recorded in `release_evidence/windows_installed_smoke.json` with the strict R2 provenance/isolation contract.
  required_action: Stage the Windows installed app into a unique run root, run `installer\windows\collect_broker_smoke.ps1`, run `installer\windows\collect_installed_smoke.ps1` on native Windows with `-BrokerHelperExe`, `-NoPythonRuntime`, UIAutomation diagnostic tree evidence, broker evidence, config path, audit dir probe inputs, and installed manifest; pass `python tooling\windows_release_evidence.py`.
  blocks_release: yes

- item: Windows Setup Doctor smoke not passed
  classification: release_blocker
  registry_id: windows_setup_doctor_smoke
  reason: The installed app supports machine-readable Setup Doctor product export, but native Windows isolated-run evidence has not been collected and validated. The PowerShell Setup Doctor collector remains external probe evidence only.
  required_action: Run isolated Windows installed smoke so the installed app writes Setup Doctor product export evidence, then pass `python tooling\windows_release_evidence.py`.
  blocks_release: yes

- item: macOS planned portability target unverified
  classification: known_limitation
  reason: no macOS validation environment is currently available, so GUI-Shell v1.0 does not claim verified macOS support.
  required_action: Validate on a macOS host before claiming macOS support as supported, ready, or complete.
  blocks_release: no

- item: Windows installed-path evidence validator
  classification: required_for_v1
  reason: `tooling/windows_release_evidence.py` now validates installed executable hash, exact source commit provenance, clean worktree state, isolated run paths, app/broker artifact hash linkage, evidence bundle hashes, field provenance, installed Flutter `.exe` launch evidence, broker-mediated first-run endpoint evidence, No-Python launch evidence, non-zero window handle, visible-surface source plus diagnostic tree, first-run config JSON parsing, audit write/read/delete probe, installed-app generated Setup Doctor product evidence, and broker authenticated IPC/restart/crash measured field provenance.
  required_action: Keep evidence validation fail-closed and reject copied, edited, synthetic, manually confirmed, shallow, aggregate-surface, non-Windows, external-probe-as-product, or unmeasured-declaration evidence.
  blocks_release: no

- item: Windows Setup Doctor diagnostics evidence not passed
  classification: release_blocker
  registry_id: windows_setup_doctor_smoke
  reason: Installed-app generated Windows Setup Doctor product evidence has not passed for the Windows-first product target; external probe evidence is invalid for this gate.
  required_action: Pass Windows Setup Doctor product export evidence from `collect_installed_smoke.ps1`; macOS diagnostics remain planned portability validation.
  blocks_release: yes

- item: validate_all.py strict release mode not passed
  classification: release_blocker
  aggregate_of: windows_evidence_provenance_isolation, windows_installer_first_run_smoke, windows_setup_doctor_smoke, windows_broker_installed_smoke, audit_anchor_external_tamper_evidence_proof, owner_go
  reason: Current-host Linux validation may pass, but Windows-first strict release mode must not report release blockers before completed product release.
  required_action: Pass `python3 tooling/validate_all.py --strict-release --desktop-platform=windows`; `--desktop-platform=all` may still fail because macOS is unverified, but that does not block Windows-first v1.0.
  blocks_release: yes

- item: implementation first-run smoke
  classification: required_for_v1
  reason: `tooling/release_smoke.py` creates first-run config and audit paths, verifies audit directory writability, and confirms installer/setup state grants no authority and silently approves no permissions.
  required_action: Keep implementation first-run smoke passing; native Windows installed-path first-run smoke remains a separate release blocker.
  blocks_release: no

- item: implementation Setup Doctor diagnostics smoke
  classification: required_for_v1
  reason: `tooling/release_smoke.py` runs structured Setup Doctor diagnostics and verifies all checks remain non-authoritative.
  required_action: Keep implementation Setup Doctor smoke passing; native Windows installed-path Setup Doctor smoke remains a separate release blocker.
  blocks_release: no

- item: Shell Core persistence smoke
  classification: required_for_v1
  reason: integrated Shell Core release smoke saves and loads a deterministic state snapshot.
  required_action: Keep integrated persistence smoke passing.
  blocks_release: no

- item: audit chain and local anchor verification smoke
  classification: required_for_v1
  reason: integrated Shell Core release smoke appends JSONL audit events, verifies hash chain linkage, verifies the HMAC audit anchor, and detects tampering.
  required_action: Keep integrated audit chain and local anchor smoke passing.
  blocks_release: no

- item: audit anchor external tamper-evidence proof
  classification: release_blocker
  registry_id: audit_anchor_external_tamper_evidence_proof
  reason: Local HMAC audit anchors detect corruption and partial tamper, but completed product release requires measured Windows ACL/DPAPI, external anchor, or signed-evidence proof before claiming same-user tamper evidence beyond the local file authority boundary.
  required_action: Record installed-path audit anchor key-protection or external-anchor evidence and pass strict Windows release validation.
  blocks_release: yes

- item: approval edit to rehash to revalidation smoke
  classification: required_for_v1
  reason: integrated Shell Core release smoke edits an allowed approval field, recalculates payload hash, and marks the approval `requires_validation`.
  required_action: Keep approval lifecycle smoke passing.
  blocks_release: no

- item: content_visibility UI enforcement smoke
  classification: required_for_v1
  reason: desktop Flutter widget smoke confirms redacted approval projection is visible and hidden full payload content is not rendered.
  required_action: Keep UI projection smoke passing.
  blocks_release: no

- item: Runtime Catalog validation and use smoke
  classification: required_for_v1
  reason: `tooling/release_smoke.py` registers runtime and adapter manifests through production RuntimeCatalog and confirms catalog does not grant authority.
  required_action: Keep Runtime Catalog smoke passing.
  blocks_release: no

- item: Agent Runtime Contract validation and reference smoke
  classification: required_for_v1
  reason: `tooling/release_smoke.py` checks workspace boundary, secret path denial, shell permission mapping, and auditable diff behavior through production AgentRuntimeContract.
  required_action: Keep Agent Runtime reference smoke passing.
  blocks_release: no

- item: owner GO missing
  classification: release_blocker
  registry_id: owner_go
  required_action: Obtain explicit owner GO.
  blocks_release: yes

## LLM-Readable Substrate Claim Gates

- item: LLM extension contract sufficiency unresolved
  classification: known_limitation
  reason: GUI Shell is defined as an LLM-readable substrate, but the existing contract families have not yet been audited for bounded LLM-built extension onboarding.
  required_action: Complete Block L1 in `docs/implementation/GUI_SHELL_LLM_SUBSTRATE_COMPLETION_ROADMAP.md`.
  blocks_release: no

- item: bounded extension conformance not passed
  classification: known_limitation
  reason: No bounded reference extension / adapter conformance harness has yet proven that LLM-built integrations cannot escalate authority, bypass approval, bypass content exposure, omit audit, omit recovery, or break runtime neutrality.
  required_action: Complete Blocks L2/L3 as required by the L1 gap decision.
  blocks_release: no

- item: cross-agent reproduction not passed
  classification: known_limitation
  reason: More than one independent LLM development agent has not yet reproduced the same bounded extension task from the repository contracts.
  required_action: Complete Blocks L4/L5 before making cross-agent LLM-readable substrate claims.
  blocks_release: no

These items block a demonstrated LLM-readable substrate public claim. They do not automatically block a narrowly described Windows-first desktop product release unless the owner chooses the default combined public positioning in the canonical roadmap.

## Post-v1 Scope Defaults

- item: mobile full release
  classification: post_v1_scope
  reason: v1.0 is Windows-first PC desktop unless owner explicitly includes mobile in release scope.
  blocks_release: no

- item: multi-user mode
  classification: post_v1_scope
  reason: v1.0 is single-user.
  blocks_release: no

- item: cloud sync
  classification: post_v1_scope
  reason: v1.0 is local-first.
  blocks_release: no

- item: marketplace
  classification: post_v1_scope
  reason: v1.0 excludes runtime marketplace.
  blocks_release: no

- item: enterprise admin
  classification: post_v1_scope
  reason: v1.0 is not enterprise admin scope.
  blocks_release: no

- item: full live Codex / Claude Code / Copilot / Cursor / Devin / OpenHands integrations
  classification: post_v1_scope
  reason: v1.0 requires generic Agent Runtime contract and mock/reference agent only.
  blocks_release: no

- item: BLUE-TANUKI product completion
  classification: post_v1_scope
  reason: BLUE-TANUKI is a consumer/reference runtime, not a GUI-Shell release gate.
  blocks_release: no

## Known Limitation Rule

Known limitations are allowed only if:

- classification: known_limitation
  reason: limitation does not violate v1.0 release criteria
  required_action: Document in README.md and CLAIM.md
  blocks_release: no

- classification: known_limitation
  reason: limitation does not hide safety, authority, audit, recovery, installer, or validation failures
  required_action: Keep release-facing documentation explicit
  blocks_release: no
