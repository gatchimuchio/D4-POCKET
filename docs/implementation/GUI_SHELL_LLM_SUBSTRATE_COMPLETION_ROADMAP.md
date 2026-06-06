# GUI Shell LLM Substrate Completion Roadmap

Status: C0 definition and evidence closure
Scope: Windows-first OSS v1.0 product completion, demonstrated LLM-readable extension substrate capability, initial public release, and post-public product QC
Current baseline: `main` after LLM-readable substrate definition lock

## 1. Product Definition

GUI Shell is a generic Runtime Operation Shell and LLM-readable application responsibility substrate.

It is designed for two first-class roles.

Human operator / owner:

- observes product and runtime state;
- grants or denies approval;
- authorizes recovery;
- accepts release claims;
- remains the final responsibility holder;
- remains the only source of final product GO.

LLM development / integration agent:

- reads GUI Shell architecture, standards, schemas, conformance rules, and operating procedures;
- implements bounded modules, adapters, tools, or runtime integrations;
- connects extensions through declared GUI Shell contracts;
- runs validation and reports evidence;
- must not create authority, approve its own sensitive operations, widen permissions silently, bypass conformance, or convert generated output, memory, tool output, metadata, UI state, or generated configuration into trusted authority.

Critical invariant:

```text
LLMs are first-class implementation and integration consumers of GUI Shell contracts, but are never authority sources.
```

GUI Shell remains generic and is not BLUE-TANUKI-specific. BLUE-TANUKI remains the first reference consumer/runtime through adapter boundaries only. Shell Core must remain runtime-neutral. Flutter remains the operator-facing UI layer and must not own authority. Rust Security Broker remains the authority-sensitive production boundary. Python may remain for tooling, validation, CI, migration oracle, parity comparison, and evidence validation, but must not remain required for the installed active authority runtime.

## 2. Current Verified Starting Point

Completed / already established:

- Phase A: personal Windows trial operation is complete.
- Phase B: owner-use operational hardening is complete.
- GUI Shell is documented as a generic Runtime Operation Shell, control plane, and LLM-readable application responsibility substrate.
- Human final authority remains explicit.
- BLUE-TANUKI remains adapter-only reference consumer/runtime.
- Flutter product path has begun broker-mediated integration.
- Rust Security Broker process, authenticated loopback IPC, durable audit/replay/session storage, authority parity operations, fail-closed handling, and release runtime assertions exist for the current scope.

Current C0 validation evidence:

- `python tooling/schema_check/check_schemas.py || python3 tooling/schema_check/check_schemas.py`: passed through `python3` fallback with `schema check passed: 26 schemas, 26 examples, 28 negative fixtures`.
- `python tooling/conformance_tests/run_conformance_skeleton.py || python3 tooling/conformance_tests/run_conformance_skeleton.py`: passed through `python3` fallback with `conformance skeleton passed: 132 checks`.
- `python3 tooling/manifest.py --check`: passed with `manifest check passed`.
- `python3 tooling/release_gate_check.py`: passed with `release gate check passed`.
- `python3 tooling/evidence_bundle.py --check`: passed with `evidence bundle check passed: 3 release blockers preserved, release_ready=False, classification=development_evidence`.

Historical validation entries that record `96 checks` remain preserved as earlier evidence. They must not be rewritten as current evidence.

Still incomplete / blockers that remain visible:

- Rust Broker production authority cutover remains incomplete: `authority_cutover_status=not_active`, real external command dispatch remains suspended, and process / credential / update gated execution remains incomplete.
- Installed product proof remains incomplete: installed no-Python-runtime evidence, Windows installed-path broker proof, installed first-run proof, installed Setup Doctor proof, and strict Windows release validation.
- Owner GO is not recorded.
- LLM-readable substrate is definition-locked, bounded reference extension conformance exists for the current contract/conformance layer, and one bounded cross-agent reproduction report exists for a controlled non-authoritative extension task. Public external claim evidence remains outside the current evidence scope.

## 3. Completion Target Definitions

### Target T0: Definition and Evidence Closure

Product definition is internally consistent. LLM-readable substrate wording is aligned across governing documents. Current validation evidence is internally consistent. No unsupported product or ecosystem claim exists.

T0 is documentation/governance closure only. It does not prove runtime completion or LLM extension functionality.

### Target T1: LLM-Readable Extension Substrate Demonstrated

GUI Shell has explicit extension/integration contract coverage adequate for bounded LLM-built extensions. A bounded reference module or adapter can be added through declared contracts. Conformance proves the extension cannot create or escalate authority, bypass approval, bypass content exposure rules, bypass audit, bypass recovery, or bypass runtime neutrality. At least two independent LLM development agents can execute the same bounded extension task in isolated workspaces and preserve the required boundaries.

Permitted claim only after T1:

```text
GUI Shell demonstrates a contract-governed LLM-readable extension substrate for bounded reference integrations.
```

Do not claim public standard status or broad ecosystem adoption.

### Target T2: Windows-First OSS v1.0 Completed Product Release

Rust Security Broker authority-sensitive production path is complete for v1.0 scope. Installed product does not require Python for active authority runtime. No Flutter/Rust FFI authority bypass exists. Broker failure paths fail closed. Windows installed-path first-run, Setup Doctor, broker, UIAutomation, artifact hash, no-Python-runtime, audit probe, and recovery evidence pass. Strict Windows release validation passes with no v1.0 release blockers. Owner explicitly grants GO.

### Target T3: Initial Public Release with Central LLM-Substrate Positioning

Default release strategy: public OSS v1.0 should not be positioned around the new LLM-readable substrate identity until both T1 and T2 have passed.

If the owner explicitly chooses to release T2 before T1, external language must state only that the architecture is designed for LLM-readable extension and that demonstrated cross-agent extension evidence is not yet complete.

### Target T4: Post-Public Commercial / Product QC Readiness

T4 covers product support boundary, official distribution policy, paid/free scope separation, dependency/license/legal review, long-run stability, rollback/update servicing, user-facing failure recovery, commercial responsibility boundary, and owner-selected enterprise or third-party integration scope.

T4 is planned after initial public release unless the owner explicitly changes release strategy.

## 4. Two-Track Execution Architecture

Track R: Runtime / Product Responsibility Completion.

Purpose: complete the actual Windows-first product responsibility path.

Track R owns Rust Security Broker production convergence, authority cutover, command/process/credential/update gating, fail-closed runtime behavior, installed-path Windows evidence, installer / Setup Doctor / first-run proof, strict release validation, and owner GO.

Track L: LLM-Readable Extension Substrate Demonstration.

Purpose: prove that GUI Shell is usable as a safe contract reference and extension substrate for LLM development agents.

Track L owns contract sufficiency audit, extension/module onboarding model, bounded extension conformance, negative tests, agent-facing task packet, cross-agent independent reproduction, and claim promotion evidence.

Merge rule:

- Track L may develop using development/fixture/conformance evidence before Track R completes.
- Track L evidence must not be described as installed-product proof.
- Track R product completion must not be described as LLM-extension proof.
- Central public claim of GUI Shell as a demonstrated LLM-readable substrate requires both Track L and Track R completion unless the owner explicitly authorizes narrower wording.

## 5. Block-by-Block Roadmap

Each repository-state-modifying block must preserve the pushed pre-change state using the two-generation backup convention, implement only the scoped change, validate, update manifest if tracked files changed, commit, push, verify remote `main`, verify both backup-generation tags, verify clean/aligned working tree, and report rollback point.

### Block C0: Validation Evidence and Roadmap Closure

Objective: close the current evidence inconsistency and establish this canonical completion roadmap.

Allowed surface: `docs/implementation/GUI_SHELL_LLM_SUBSTRATE_COMPLETION_ROADMAP.md`, `ROADMAP.md`, `docs/PHASE_STRATEGY.md`, `RELEASE_CHECKLIST.md`, `CLAIM.md`, `README.md`, `VALIDATION.txt`, `MANIFEST.sha256.json`.

Expected deliverables:

- canonical roadmap document exists;
- current validation evidence records 99 conformance checks;
- historical 96-check evidence remains preserved as history;
- release blockers remain open;
- manifest passes.

Prohibited scope expansion: no Rust, Flutter, installer, schema, conformance feature implementation, extension/module loader, agent integration, or release claim promotion.

Validation:

```bash
python tooling/schema_check/check_schemas.py || python3 tooling/schema_check/check_schemas.py
python tooling/conformance_tests/run_conformance_skeleton.py || python3 tooling/conformance_tests/run_conformance_skeleton.py
python3 tooling/manifest.py --check
python3 tooling/release_gate_check.py
python3 tooling/evidence_bundle.py --check
```

Exit criteria: current validation evidence is appended and internally consistent; historical evidence is not falsified; existing release blockers remain open; commit/push/remote verification/backup verification complete.

Release/claim impact: T0 progress only. No runtime or LLM-substrate demonstration claim is promoted.

Rollback point: pre-change `refs/tags/codex/backup-main`.

### Block L1: Existing Contract Sufficiency Audit for LLM Extensions

Objective: determine whether GUI Shell already has sufficient contract families for bounded LLM-built extensions, or whether a minimal explicit extension/integration contract is required.

Allowed surface: documentation and contract analysis only.

Expected deliverable: `docs/implementation/LLM_EXTENSION_CONTRACT_GAP_ANALYSIS.md`.

Required analysis surface: runtime, adapter, capability, permission, approval, audit, recovery, content exposure, update/install contracts, Agent Runtime Contract, Runtime Catalog, existing adapter examples, and the LLM-readable standard.

Exit criteria: contract gap decision is documented, no speculative schema is added without necessity, and the exact next conformance block is defined.

Release/claim impact: blocks demonstrated LLM-substrate claim until resolved; does not automatically block a narrowly described Windows desktop product release.

### Block L2: Minimal Extension Contract Closure

Execute only if L1 proves a contract gap.

Objective: introduce the smallest machine-readable contract required to represent bounded LLM-built extension onboarding safely.

Possible contracts only if justified: `specs/extension_manifest.schema.json` or `specs/extension_submission.schema.json`. Do not create both unless the distinction is proven necessary.

Required negative coverage: authority escalation metadata, self-approved sensitive behavior, undeclared capability use, undeclared permission use, audit omission, recovery omission, content exposure bypass, runtime-specific logic injected into Shell Core, and generated configuration attempting to create authority.

Exit criteria: schema/fixture/negative fixture coverage exists only where required, conformance checks use the actual governing contract path, and no new runtime execution path is activated.

### Block L3: Bounded Reference Extension Conformance Harness

Objective: prove the LLM-readable substrate claim at the contract/conformance layer.

Reference task: add a bounded reference adapter or non-authoritative diagnostic module with declared capability only, no privileged execution, audit evidence, failure mapping to RecoveryAction or SUSPEND, and runtime neutrality.

Required tests: the bounded extension can be registered only through declared contracts and cannot create authority, grant permission, approve itself, expose full content without policy, omit audit evidence, omit recovery mapping, use metadata/memory/generated config/tool response/UI state as authority, pull runtime-specific logic into Shell Core, or pass when malformed or unauthorized.

Evidence classification: contract/conformance demonstration; not installed-product evidence, not cross-agent reproduction evidence, and not public standard adoption evidence.

### Block L4: LLM Agent Task Packet and Repository Reading Surface

Objective: make the repository consumable by independent LLM development agents without requiring human oral explanation.

Expected deliverables: `docs/implementation/LLM_EXTENSION_TASK_PACKET.md` and `docs/implementation/LLM_EXTENSION_EVIDENCE_TEMPLATE.md`, or an equivalent bounded documentation set.

Constraints: not an SDK, marketplace, plugin registry, or live Codex/Claude/Copilot integration.

Exit criteria: an independent LLM agent can be given the repository, task packet, and ordinary repository instructions and can attempt the bounded extension without additional owner explanation.

### Block L5: Cross-Agent Reproduction Evidence

Objective: test whether more than one independent LLM development agent can read GUI Shell contracts and implement the same bounded extension while preserving responsibility boundaries.

Method: at least two independent development-agent executions in isolated branches, clones, or reproducible workspaces from the same baseline commit.

Expected report: `docs/evidence/LLM_CROSS_AGENT_REPRODUCTION_REPORT.md`.

Required evidence: baseline commit, task packet, resulting diff, validation commands and outputs, boundary failures, manual repair status, and unauthorized scope expansion attempts.

Claim rule: only if reproduction passes may public documentation claim demonstrated bounded cross-agent LLM-readable extension behavior. Do not claim industry standard status or general ecosystem compatibility.

### Block R1: Rust Security Broker Responsibility Cutover Closure

Objective: complete the authority-sensitive product runtime path required for Windows-first v1.0.

Migration order:

1. authority normalization / strip / quarantine;
2. capability and permission eligibility;
3. approval validation / protected field enforcement / rehash;
4. content visibility enforcement;
5. audit append / hash-chain verification / tamper rejection;
6. recovery classification;
7. command-envelope eligibility;
8. process / credential / update gated execution.

Mandatory rule for every responsibility: Python oracle behavior is enumerated where applicable, Rust implementation exists, parity evidence passes for accepted cases, Rust rejects at least equivalent negative cases, Rust-specific IPC/session/replay failures are audited, active product invocation no longer depends on Python for that responsibility, and rollback point is recorded.

Prohibited shortcuts: no FFI authority bridge, hidden Python runtime authority path, UI-owned authority, metadata-created authority, or command dispatch activation before eligibility, audit, recovery, and rollback evidence exist.

Exit criteria: `authority_cutover_status` may become active only when justified by measured evidence.

### Block R2: Windows Installed-Path Product Evidence

Objective: prove the actual installed Windows application path, not merely local development behavior.

Required measured evidence: installed application artifact hash, installed executable launch, broker-mediated product launch, no-Python-runtime active authority evidence, non-zero main window handle, real UIAutomation per-surface evidence, no aggregate/native fake surface shortcut, first-run config creation and JSON parsing, audit directory write/read/delete probe, Setup Doctor from installed app path, broker authenticated IPC, restricted loopback or approved transport, durable store readiness, restart replay rejection, crash fail-closed behavior, no Flutter/Rust FFI authority bridge, and recovery evidence.

Execution boundary: Ubuntu-side Codex prepares source and scripts. Native Windows executes evidence collection. Native Windows must not introduce ad hoc code changes to make proof pass.

Exit criteria: Windows evidence validator passes with non-synthetic evidence tied to exact implementation commit.

### Block R3: Strict Windows Release Candidate Gate

Objective: produce an OSS v1.0 release candidate with all Windows-first product blockers closed.

Required validation includes schema check, conformance skeleton, manifest check, release gate check, evidence bundle check, release runtime assertions, Windows release evidence, `validate_all.py --strict-release --desktop-platform=windows`, Rust `cargo fmt --check`, Rust `cargo test`, and required native Windows Flutter build/test/launch validations.

Exit criteria: no Windows-first `release_blocker` remains except owner GO; claim documentation matches evidence exactly; known limitations remain visible; macOS remains unclaimed unless separately validated; BLUE-TANUKI remains non-blocking reference consumer/runtime.

### Block P1: Public Claim Hygiene and Release Packaging

Objective: prepare public OSS-facing material without overclaim.

Required public surfaces: README, CLAIM, RELEASE_CHECKLIST, ROADMAP, PHASE_STRATEGY, AUDIT, SECURITY, installation instructions, evidence reports, release notes, manifest, and rollback instructions.

Required claim tiers:

1. architecture-defined: designed as an LLM-readable responsibility substrate;
2. conformance-demonstrated: bounded extension behavior is contract/conformance-tested;
3. cross-agent-demonstrated: multiple independent LLM agents reproduced bounded extension behavior;
4. completed Windows-first product release: installed product evidence and strict release gate passed.

Never combine these levels without evidence.

### Block P2: Owner GO and Initial Public OSS Release

Objective: perform the initial public release only after required evidence is closed.

Default owner-GO prerequisites: T1 passed, T2 passed, public claims aligned, manifest and rollback point preserved, final owner review and explicit GO.

Owner-controlled exception: the owner may explicitly approve a public release before cross-agent reproduction only if public wording is reduced to design intent and states that independent cross-agent reproduction evidence is not yet complete.

### Block F1: Post-Public Commercial / Product QC

Objective: transition from open/public initial release toward commercial product responsibility only after the owner chooses that path.

Scope candidates: official distribution channel, paid/free boundary, enterprise deployment boundary, support and incident responsibility, installer/update servicing policy, long-run operation testing, dependency and license audit, vulnerability response, telemetry/privacy policy if introduced, commercial integrations, and additional runtimes or agent products.

Rule: do not prematurely burden the initial open/public release with speculative enterprise features. Do not omit product responsibility work once commercial distribution is chosen.

## 6. Dependency and Merge-Gate Model

| Block | Depends On | May Run Before Product Release? | Required for Central LLM-Substrate Public Claim? | Required for Windows OSS v1.0 Product Release? |
| ----- | ---------- | ------------------------------: | -----------------------------------------------: | ----------------------------------------------: |
| C0 | Current `main` | Yes | Yes | Yes |
| L1 | C0 | Yes | Yes | No, unless claim used |
| L2 | L1 gap decision | Yes | If gap exists | No, unless claim used |
| L3 | L1/L2 | Yes | Yes | No, unless claim used |
| L4 | L3 | Yes | Yes | No |
| L5 | L4 | Yes | Yes | No unless owner adopts default combined release |
| R1 | C0 | Yes | Yes for installed/product-backed positioning | Yes |
| R2 | R1 | No final claim before pass | Yes for product-backed positioning | Yes |
| R3 | R2 | No | Yes for completed product claim | Yes |
| P1 | L/R evidence as applicable | Yes | Yes | Yes |
| P2 | P1 + owner GO | Release event | Yes under default strategy | Yes |
| F1 | Public release / owner decision | Post-release | No | No |

## 7. Evidence and Validation Model

Every future block must distinguish:

- CONFIG evidence;
- INTERNAL_STATE evidence;
- FIXTURE evidence;
- LIVE_RUNTIME evidence;
- EXTERNAL_EVIDENCE;
- cross-agent reproduction evidence.

Never promote documentation into runtime proof, fixture success into installed-product proof, one LLM's successful diff into cross-agent reproducibility, local Windows build into installed-path release proof, or architecture definition into public standard adoption.

Each block must record behavior implemented, contract path exercised, runtime/governed path exercised, negative case rejected, commands executed, exact outputs, unverified claims, remaining blockers, and rollback point.

## 8. Claim Promotion Rules

Existing product release blockers remain release blockers:

- Rust Security Broker production convergence;
- installed no-Python-runtime evidence;
- Windows installed-path first-run evidence;
- Setup Doctor evidence;
- strict Windows release validation;
- owner GO.

LLM-readable claim blockers:

- LLM extension contract sufficiency unresolved blocks demonstrated LLM-substrate claim but does not automatically block a narrowly described desktop product release.
- Bounded extension conformance not passed blocks claims that LLM-built integrations are demonstrated.
- Cross-agent reproduction not passed blocks claims that the substrate is demonstrated across independent LLM agents.

Default combined-public-release rule:

- Because LLM-readable substrate is now central product positioning, do not make it the central public product claim until both product runtime proof and LLM extension proof pass.
- Owner may override only explicitly and with narrower claims.

## 9. Rollback and Backup Requirements

Every repository-state-modifying block must follow the two-generation backup convention:

- `refs/tags/codex/backup-main`;
- `refs/tags/codex/backup-main-prev`.

Remote backup branches must not be retained unless the owner explicitly requests emergency handoff. If push, remote HEAD verification, or backup verification fails, the block is not complete.

Rollback point for each block is the pre-change `refs/tags/codex/backup-main` hash reported in the completion report.

## 10. Explicit Out-of-Scope / Post-Public Items

Out of scope for C0 through initial public release unless explicitly promoted by owner:

- plugin registry;
- marketplace;
- SDK;
- enterprise administration;
- commercial integrations;
- cloud service;
- mobile full release;
- broad third-party runtime catalog;
- public standard adoption claim;
- telemetry/privacy policy unless telemetry is introduced;
- BLUE-TANUKI product completion as a GUI Shell release dependency.
