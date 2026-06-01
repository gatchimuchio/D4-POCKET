# GUI Shell Agent Instructions

This file defines repository-wide work discipline for AI agents working in GUI Shell.

## Part I. Common Base Discipline

### 1. Purpose and Rule Precedence

This repository is operated by AI implementation agents under explicit safety, auditability, and completion-evidence requirements.

The common base discipline in this section applies to all work in this repository.

Rule precedence:

1. Explicit owner instruction for the current task
2. Common Base Discipline in this `AGENTS.md`
3. Repository Extension rules in this `AGENTS.md`
4. Active implementation instruction / roadmap / phase document
5. Repository contracts, schemas, tests, and validation scripts
6. Existing implementation patterns

Repository Extension rules may narrow, strengthen, or specialize the Common Base Discipline for this repository. They must not silently weaken safety, authority boundaries, auditability, validation evidence, or recovery requirements.

When rules appear to conflict, preserve the stricter interpretation unless the owner explicitly instructs a controlled change to the governing rule.

### 2. Non-Negotiable Priorities

Apply this priority order unless a repository extension defines a stricter specialization:

1. Safety
2. Robustness
3. Operator clarity / auditability
4. Contract and runtime integrity
5. Product features
6. Convenience

Feature completion never outranks safety, authority boundaries, auditability, recovery, or validation evidence.

Convenience never justifies hidden authority, false completion claims, unverified runtime guarantees, unexplained workarounds, or weakened failure handling.

### 3. Bounded Implementation Discipline

Do not treat a complex specification as permission for broad generation.

For every task:

- inspect existing files, contracts, tests, validation commands, and relevant documentation before editing;
- implement the smallest maintainable change that satisfies the task;
- preserve repository-specific boundaries;
- do not perform opportunistic refactors;
- do not add speculative features;
- do not broaden permissions, authority, runtime reachability, dependencies, toolchains, or environment assumptions without explicit requirement;
- remove debris, stale TODOs, abandoned partial paths, and temporary implementation residue introduced by the task before reporting completion.

A large amount of generated structure is not evidence of completeness.

### 4. Completion Evidence Rule

A completion claim is not evidence.

Before reporting a work block as complete, identify:

- the behavior implemented;
- the production, runtime, contract, or validation path that exercises it;
- the exact validation commands actually run;
- the exact results;
- validation that was not run;
- remaining stubs, mocks, placeholders, TODOs, unconnected contracts, environment limitations, or known limitations.

Documentation, schema presence, mock success, fixture success, or unit-test success alone must not be reported as proof that a production path or product behavior is complete.

For security-critical, authority-critical, audit-critical, recovery-critical, or release-critical changes, state what evidence demonstrates that the real governed path is exercised.

### 5. Evidence Source / No Ghost Invariants Rule

Do not report runtime health, system integrity, security invariants, authority integrity, or release readiness based only on:

- configuration validation;
- schema validation;
- self-generated state objects;
- mocked runtime state;
- fixture-only results;
- static object or dictionary consistency checks.

When adding or modifying a health check, invariant check, conformance check, or integrity report, classify its evidence source as one or more of:

- CONFIG
- INTERNAL_STATE
- LIVE_RUNTIME
- EXTERNAL_EVIDENCE
- FIXTURE

Each evidence class proves only the scope it actually observes.

CONFIG, INTERNAL_STATE, or FIXTURE results must not be promoted into live-runtime or external-integrity guarantees without corresponding evidence.

If required evidence is unavailable, report the limitation or return SUSPEND where the repository contract requires fail-closed behavior.

### 6. Trust Boundary and Input Verification Rule

Do not assume inbound data is safe merely because it is structured, parsed, schema-shaped, or supplied by another component.

For any input that may affect authority, permission, execution, approval, audit identity, workspace scope, command scope, content visibility, recovery, or release behavior, the responsible boundary must explicitly account for the applicable parts of:

- raw input retention for audit;
- canonicalization / normalization;
- schema or structural validation;
- origin or source validation;
- integrity or tamper checks;
- replay protection;
- authority or execution-eligibility evaluation;
- audit emission;
- fail-closed or SUSPEND behavior.

External data, UI state, adapter metadata, channel metadata, previous state, memory, history, diagnostics, and tool output must not create, escalate, replace, or bypass authority unless the repository extension explicitly defines a bounded, validated authority path.

### 7. Active Production Path Minimization Rule

Keep the normal production or runtime execution path minimal and responsibility-bounded.

Do not silently mix ordinary runtime behavior with:

- diagnostic functionality;
- repair or recovery tooling;
- migrations;
- release-only verification;
- development-only fixtures;
- bootstrap-only tooling;
- administrative commands.

When adding privileged or operational functionality, classify it as one of:

- runtime path;
- control path;
- diagnostic path;
- repair / recovery path;
- build / release path;
- development-only path.

If non-runtime functionality must be reachable from an ordinary runtime path, document why, identify its authority and audit consequences, and validate that it does not expand hidden execution power.

### 8. Wrapper / Workaround Accountability Rule

Do not introduce wrapper scripts, shims, custom execution layers, alternate build paths, environment bypasses, or host-specific workaround logic merely to make a failing task appear complete.

If such a mechanism is required and not prohibited by the repository extension, document:

- the original failure;
- the root cause;
- why the native or existing repository mechanism is insufficient;
- the exact responsibility of the added mechanism;
- the environments in which it applies;
- validation performed;
- whether it is temporary or permanent;
- its removal condition or formalization condition.

A deliberate, tested, bounded, documented normalization mechanism may be acceptable unless prohibited by the repository extension.

An unexplained, unbounded, or symptom-hiding workaround is not acceptable.

### 9. Environment and Product-Proof Separation Rule

Keep development environment, CI environment, validation environment, release-proof environment, and target product environment conceptually separate.

Do not report success in one environment as proof of success in another environment unless the repository explicitly defines that equivalence and evidence supports it.

When validation is blocked or distorted by host environment limitations:

- identify the environment limitation;
- distinguish it from a product regression;
- do not modify product architecture merely to hide the host failure;
- report what remains unverified in the target environment.

Local development convenience must not silently become permanent product architecture or release evidence.

### 10. Contract-to-Runtime Connection Rule

A schema, interface, protocol object, adapter contract, audit contract, invariant contract, fixture, or success profile is not complete merely because it exists.

When adding or modifying a contract intended to affect real behavior, identify:

- its consuming production, runtime, validator, or governed execution path;
- the validation or conformance path that exercises it;
- the negative or failure case that must be rejected, blocked, audited, or suspended;
- any part that remains intentionally unconnected or deferred.

Do not claim behavioral completion for contracts that are defined but not exercised by the intended governed path.

### 11. Audit Outcome and Reporting Rule

Every completed work report must distinguish:

- observed implementation facts;
- validation actually executed;
- unverified claims;
- environment-limited checks;
- remaining risks;
- intentionally deferred scope;
- repository-specific release blockers.

Where the repository uses release-gate classifications, preserve and apply them.

Where a safety-critical, authority-critical, or execution-critical requirement cannot be verified, do not infer success. Report SUSPEND, blocker, or the repository-specific equivalent.

## Part II. Repository Extension

### 12. Repository Identity and Scope

Do not treat this repository as a normal app scaffold.

GUI Shell is a generic Runtime Operation Shell control plane.

Flutter, adapters, reference runtimes, local caches, memory, installers, native helpers, and product UI are downstream or bounded implementation surfaces. They do not own authority.

GUI Shell implements a generic GUI Shell / Runtime Operation Shell.

It is not a BLUE-TANUKI-specific GUI.

BLUE-TANUKI is the first reference runtime and must connect through an adapter boundary.

### 13. Architecture Constraints

- UI framework: Flutter
- Native helper: Rust
- Contracts: JSON Schema
- Language policy: `docs/LANGUAGE_POLICY.md`
- Reference runtime: BLUE-TANUKI via adapter only
- Shell Core must remain framework-independent
- Adapter contracts must remain runtime-neutral
- Flutter must remain a replaceable UI layer
- Flutter / Dart is the UI product layer and must not become the authority boundary
- Rust is the native safety boundary for authority-sensitive helper, broker, IPC, audit, signature, and runtime command-envelope work
- TypeScript / Node must not become GUI-Shell core runtime; keep it limited to external SDK, adapter sample, protocol client sample, or bridge example scope
- Python must not become GUI-Shell runtime dependency; keep it limited to dev-only tooling, schema generation, migration helper, CI support, or temporary validation script scope
- Authority-sensitive Flutter-Rust connection must prefer independent process IPC; FFI/direct bridge is allowed only outside authority, signature, approval-token, external command dispatch, and audit finalization boundaries
- BLUE-TANUKI implementation must not be modified for GUI Shell convenience unless the owner explicitly requests it

### 14. Boundary Semantics

#### Shell Core

Shell Core owns:

- runtime registry
- permission ledger
- approval queue
- audit store
- recovery catalog
- update policy
- content exposure enforcement
- adapter conformance enforcement

Shell Core must not:

- import Flutter
- contain BLUE-TANUKI-specific logic
- trust adapter metadata
- use memory/cache/previous state as authority by itself
- silently broaden permission

#### UI Layer

Flutter may own:

- rendering
- operator input
- navigation
- local UI state
- theme
- localization
- accessibility

Flutter must not own:

- authority decisions
- permission semantics
- approval semantics
- audit semantics
- recovery classification
- content visibility rules
- runtime trust rules

GUI display success is not proof of runtime-contract completion or authority safety.

#### Adapter Layer

Adapters may:

- normalize runtime state
- expose runtime health
- expose runtime diagnostics
- translate runtime events into GUI Shell schemas

Adapters must not:

- grant permission through metadata
- create authority context not granted by runtime
- display raw payloads beyond allowed visibility
- edit sealed, hidden, sacred, or authority fields
- bypass approval state
- bypass audit creation

Adapter-exposed health or diagnostics must state the evidence scope they represent.

#### Rust Helper

Rust helper may perform bounded native diagnostics and operations.

Rust helper must not:

- become a hidden authority path
- silently introduce filesystem, process, network, credential, IPC, or update access
- execute sensitive actions without capability / permission / approval / audit / recovery mapping
- return unstructured sensitive data

### 15. Repository-Specific Forbidden Patterns

Do not:

- put authority decisions in UI widgets
- let adapter metadata grant permissions
- let adapter metadata create authority context
- let memory, local cache, or previous state grant authority by itself
- display full content unless `content_visibility=full`
- edit authority, sealed, hidden, or sacred fields in approval payloads
- introduce hidden network, filesystem, process, credential, IPC, or update access
- silently broaden runtime permissions
- add BLUE-TANUKI-specific logic to Shell Core
- place core contracts inside Flutter-specific code
- claim release readiness without validation evidence
- treat first-run success as product completion
- treat product UI completion as contract completion
- create speculative features outside the roadmap
- perform broad refactors unless required by the task

### 16. Required Audit Mapping

Every sensitive action must map to:

- Capability
- Permission
- Approval state
- AuditEvent
- RecoveryAction on failure

Sensitive actions include:

- filesystem access
- process execution/control
- network access
- credential access
- IPC
- update verification
- runtime adapter actions
- approval payload edits
- audit export/inspection
- recovery execution
- installer state changes
- device pairing

### 17. Content Exposure Rules

Allowed content visibility values:

```text
none
hash_only
summary
redacted
full
```

Rules:

- `none`: do not display raw content
- `hash_only`: display only payload hash
- `summary`: display only approved summary
- `redacted`: display only redacted projection
- `full`: full content may be displayed

Only `full` permits full payload display.

### 18. Approval Edit Rules

Approval editing must be field-scoped.

Do not allow editing of:

- authority fields
- sealed fields
- hidden fields
- sacred domain fields
- runtime identity
- permission identity
- audit identity
- payload hash directly

After any allowed edit:

- rehash payload
- revalidate payload
- mark approval as requiring validation when needed
- emit audit event

### 19. Required Validation Before Commit

Run at minimum:

```bash
python tooling/schema_check/check_schemas.py
python tooling/conformance_tests/run_conformance_skeleton.py
```

If `python` is unavailable:

```bash
python3 tooling/schema_check/check_schemas.py
python3 tooling/conformance_tests/run_conformance_skeleton.py
```

If Rust is installed and Rust helper is touched:

```bash
cd native/rust_helper && cargo test
```

If Flutter is installed and Flutter app is touched:

```bash
cd apps/desktop_flutter && flutter analyze
cd apps/mobile_flutter && flutter analyze
```

If validation cannot run, report why.

Never claim validation passed unless it actually passed.

### 20. Git Operation Policy

This repository uses a direct-main owner workflow.

Every completed work block must be committed and pushed. Do not leave completed repository changes only in the local working tree unless the owner explicitly says not to commit or not to push.

Default workflow:

1. Work on `main`
2. Do not create feature branches or pull requests unless the owner explicitly asks
3. Before committing a completed work block on `main`, rotate the two-generation backup pair:
   - If `codex/backup-main` exists, force-update `codex/backup-main-prev` to `codex/backup-main`
   - Force-update `codex/backup-main` to current pre-commit `main`
4. Push backup branches when credentials allow
5. Commit the completed work block directly on `main`
6. Push `main` immediately after the commit
7. Verify `git status --short --branch` is clean and aligned with `origin/main`
8. If backup, commit, or push fails, report the exact failed command and reason

Backup branches:

```text
codex/backup-main
codex/backup-main-prev
```

Do not create additional backup generations.

Do not stage:

- secrets
- local runtime state
- Flutter build output
- Rust target output
- installer artifacts
- local caches
- generated logs unless explicitly requested

### 21. Completion Report and Release-Gate Classification

Every completed change report must include:

1. Summary
2. Changed files
3. Risk classification
4. Validation results
5. Release-gate classification
6. Remaining risks, classified
7. Commit hash, or `not committed`

Validation results must explicitly say which commands passed, failed, or were not run.

In this repository, "release" means completed product release.

Any final report, release report, validation report, or remaining-risk section must classify every unfinished item as:

- `release_blocker`
- `post_v1_scope`
- `known_limitation`

No unclassified "remaining risks", "still needed", "not run", "not implemented", "not verified", "TODO", "skeleton only", or "future work" item is allowed.

Any `release_blocker` prevents release claim.

Any `post_v1_scope` item must explicitly state why it is outside v1.0 scope.

Any `known_limitation` must be documented in `README.md`, `CLAIM.md`, or `RELEASE_CHECKLIST.md` before release.

Remaining risks format:

```text
- item:
  classification: release_blocker | post_v1_scope | known_limitation
  reason:
  required_action:
  blocks_release: yes | no
```

### 22. Documentation Language

Primary human-facing documentation language is Japanese.

English is allowed for:

- code comments where conventional
- schema identifiers
- protocol terms
- command names
- package metadata
- concise agent instruction text

Preserve established terms:

- GUI Shell
- Runtime Operation Shell
- Shell Core
- Adapter Contract
- Authority Strip Conformance
- Content Exposure Boundary
- FrameworkRiskProfile
- Approval
- AuditEvent
- RecoveryAction
- BLUE-TANUKI
- Rust helper

### 23. Product Stance

GUI Shell may make operation comfortable.

GUI Shell must not hide authority.

The UI is a surface, not the system authority.

Schemas and conformance are the contract gate.

Do not weaken approval, audit, visibility, or recovery requirements to improve comfort.

Do not use local owner operation as an excuse to reduce robustness.

Do not treat product UI completion as contract completion.

## Part III. Active Work and Phase Guidance

### 24. Source of Truth

Within the precedence model in Part I, use this repository-specific active guidance order:

1. `ROADMAP.md`
2. `docs/standards/gui-shell-extended-standard.md`
3. `specs/*.schema.json`
4. Existing tests and validation scripts
5. Existing implementation patterns

If conflict exists, choose the stricter rule that preserves Shell Core authority boundaries, schema integrity, conformance coverage, and operator safety.

### 25. Required Work Order

Unless the owner explicitly instructs otherwise, work in this order:

1. Read `docs/standards/gui-shell-extended-standard.md`
2. Read relevant schemas under `specs/`
3. Preserve Shell Core / UI / Adapter / Rust helper boundaries
4. Add or update schemas before implementation when contracts change
5. Add or update conformance tests before product UI
6. Implement minimal bounded code
7. Run validation
8. Report exact results

Do not optimize a local task in a way that makes later phases less safe, less inspectable, or harder to validate.

### 26. Active Instruction / Roadmap References

The canonical active roadmap is `ROADMAP.md`.

The extended standard is `docs/standards/gui-shell-extended-standard.md`.

The language and safety-boundary policy is `docs/LANGUAGE_POLICY.md`.

Schemas under `specs/` define the contract gate for runtime, adapter, capability, permission, approval, audit, recovery, diagnostic, update, content exposure, framework risk, runtime manifest, adapter manifest, and agent runtime surfaces.

### 27. Phase-Specific Rules

Follow the current phase and release boundary in `ROADMAP.md`.

Completed product release must not be claimed until the release-gate blockers in repository documentation are resolved, strict validation passes, and explicit owner GO exists.

Phase-specific implementation must preserve:

- schema-first contract changes;
- conformance-first coverage;
- Shell Core independence from Flutter;
- BLUE-TANUKI reference runtime isolation behind adapter boundaries;
- bounded Rust helper authority;
- Windows-first release evidence separation from Linux development evidence;
- macOS known-limitation handling until macOS host validation exists.
