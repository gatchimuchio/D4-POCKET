# LLM Extension Task Packet

Status: Block L4 task packet for independent agent reproduction
Date: 2026-06-04
Scope: Bounded LLM-readable extension substrate reproduction task

## 1. Purpose

This packet is the bounded task input for an independent LLM development / integration agent.

The task proves whether an agent can read GUI Shell repository contracts and add or modify a bounded reference extension scenario without breaking authority, approval, audit, recovery, content exposure, or runtime neutrality constraints.

This packet is not an SDK, plugin registry, marketplace, module loader, live agent integration, installed-product proof, Windows release evidence, or public standard claim.

Critical invariant:

> LLMs are first-class implementation and integration consumers of GUI Shell contracts, but are never authority sources.

## 2. Required Reading

Before editing, the agent must read:

- `AGENTS.md`
- `README.md`
- `CLAIM.md`
- `ROADMAP.md`
- `VALIDATION.txt`
- `docs/OPERATING_MODEL.md`
- `docs/PHASE_STRATEGY.md`
- `docs/LANGUAGE_POLICY.md`
- `docs/standards/gui-shell-extended-standard.md`
- `docs/standards/llm-readable-extension-surface.md`
- `docs/implementation/GUI_SHELL_LLM_SUBSTRATE_COMPLETION_ROADMAP.md`
- `docs/implementation/LLM_EXTENSION_CONTRACT_GAP_ANALYSIS.md`
- `examples/contracts/llm_bounded_extension.valid.json`
- `tooling/conformance_tests/run_conformance_skeleton.py`

If a rule appears to conflict, preserve the stricter interpretation that protects authority boundaries, validation evidence, auditability, recovery, and runtime neutrality.

## 3. Baseline Requirements

The agent must start from the owner-provided baseline commit.

Before editing:

```bash
git status --short --branch
git branch --show-current
git rev-parse HEAD
git rev-parse origin/main
git fetch origin main --prune --tags
```

Expected branch: `main`, unless the owner provides an isolated reproduction branch or clone.

The working tree must be clean before the task starts. If it is not clean, stop and report the uncommitted state.

## 4. Task Objective

Implement one bounded contract/conformance improvement for the existing LLM-readable reference extension surface.

Allowed task shapes:

- add a negative conformance case for `examples/contracts/llm_bounded_extension.valid.json`;
- add a positive conformance check that ties an existing record in that fixture to an existing schema or governed path;
- add a small fixture-only mutation inside `tooling/conformance_tests/run_conformance_skeleton.py`;
- add documentation that clarifies the bounded extension reproduction evidence format;
- tighten non-authority source handling where the repository already defines the source as non-authoritative.

The task must preserve the existing reference extension as:

- non-authoritative;
- mock or fixture-based;
- contract/conformance evidence only;
- runtime-neutral;
- not BLUE-TANUKI-specific;
- not installed-product proof;
- not cross-agent reproduction proof by itself.

## 5. Allowed Paths

The agent may change only the smallest necessary subset of:

- `examples/contracts/llm_bounded_extension.valid.json`
- `tooling/conformance_tests/run_conformance_skeleton.py`
- `packages/shell_core/permission_ledger.py`
- `docs/implementation/LLM_EXTENSION_TASK_PACKET.md`
- `docs/implementation/LLM_EXTENSION_EVIDENCE_TEMPLATE.md`
- `docs/implementation/GUI_SHELL_LLM_SUBSTRATE_COMPLETION_ROADMAP.md`
- `docs/implementation/LLM_EXTENSION_CONTRACT_GAP_ANALYSIS.md`
- `README.md`
- `CLAIM.md`
- `VALIDATION.txt`
- `MANIFEST.sha256.json`

If another file appears necessary, stop before editing it and report why the existing allowed surface is insufficient.

## 6. Forbidden Changes

Do not change:

- production runtime execution paths;
- Rust broker implementation;
- Flutter UI;
- installer scripts;
- Windows evidence collectors;
- release evidence JSON;
- schema files under `specs/`;
- generic schema checker behavior;
- BLUE-TANUKI runtime code;
- authority boundary design;
- approval semantics;
- audit semantics;
- recovery semantics;
- content exposure policy semantics;
- repository backup discipline.

Do not add:

- a plugin registry;
- a module loader;
- an SDK;
- marketplace behavior;
- live third-party agent integration;
- new dependencies;
- runtime command dispatch;
- privileged filesystem, process, network, credential, IPC, or update behavior.

## 7. Required Contract Surface

Any accepted change must account for the existing contract families:

- runtime;
- adapter;
- runtime manifest;
- adapter manifest;
- capability;
- permission;
- approval;
- audit;
- recovery;
- content exposure;
- update/install;
- agent runtime when the change concerns agent-operated development behavior.

If the proposed change cannot be represented by existing contracts, the correct output is a contract gap report, not an improvised implementation.

## 8. Required Negative Cases

At least one governed negative case must remain or be added for the changed surface.

Valid negative case categories:

- authority escalation metadata;
- self-approved sensitive behavior;
- undeclared capability;
- undeclared permission;
- missing audit evidence;
- missing recovery mapping;
- full content exposure without policy;
- memory, generated output, generated config, metadata, tool response, GUI state, or previous state attempting to become authority;
- runtime-specific logic entering Shell Core;
- BLUE-TANUKI-specific logic entering Shell Core.

The negative case must fail closed through schema validation, conformance, policy evaluation, catalog rejection, normalization quarantine, or equivalent existing validation path.

## 9. Required Validation

Run at minimum:

```bash
python tooling/schema_check/check_schemas.py || python3 tooling/schema_check/check_schemas.py
python tooling/conformance_tests/run_conformance_skeleton.py || python3 tooling/conformance_tests/run_conformance_skeleton.py
python3 tooling/manifest.py --check
python3 tooling/release_gate_check.py
python3 tooling/evidence_bundle.py --check
```

If runtime-boundary code is touched, also run:

```bash
python3 tooling/release_runtime_assertions.py --check
```

If Rust helper or Flutter files are touched, the task has exceeded this packet unless the owner explicitly allowed that scope. Stop and report before continuing.

## 10. Expected Output

The agent must produce:

- a bounded diff;
- validation command outputs;
- evidence classification;
- remaining blocker classification;
- a statement that no installed-product proof or cross-agent reproduction claim is made;
- a completed `docs/implementation/LLM_EXTENSION_EVIDENCE_TEMPLATE.md`-compatible report.

## 11. Completion Report Format

The completion report must include:

1. Summary
2. Baseline commit
3. Task packet path and version/date
4. Changed files
5. Contract path exercised
6. Negative case added or preserved
7. Validation commands and exact outputs
8. Evidence classification
9. Production/runtime behavior change status
10. Release-gate impact
11. LLM-substrate claim impact
12. Unauthorized scope expansion attempts, if any
13. Manual repair required, if any
14. Remaining risks with repository classifications
15. Working branch
16. Commit hash, or `not committed`
17. Push result, or `not pushed`
18. Remote HEAD verification, if pushed
19. Backup refs, if repository-state modifying direct-main work was performed
20. Rollback point

## 12. Evidence Classification Rules

Allowed classifications for this packet:

- CONFIG
- INTERNAL_STATE
- FIXTURE

Do not classify this task as:

- installed-product evidence;
- Windows release evidence;
- external evidence;
- cross-agent reproduction evidence by itself;
- public standard adoption evidence.

Cross-agent reproduction evidence exists only after the owner runs at least two independent agent executions from the same baseline and compares the resulting diffs and validation results.

## 13. Stop Conditions

Stop and report before editing further if:

- a new schema appears necessary;
- production runtime behavior appears necessary;
- authority semantics need to change;
- approval, audit, recovery, or content exposure rules appear insufficient;
- Rust, Flutter, installer, Windows evidence, or release evidence files appear necessary;
- validation fails and the fix would exceed allowed paths;
- the working tree is dirty with unrelated changes;
- remote push or backup verification fails.

## 14. Success Criteria

The task succeeds only if:

- the diff stays inside allowed scope;
- existing contracts are used instead of bypassed;
- at least one relevant negative path is validated;
- validation passes;
- no release blockers are incorrectly closed;
- no installed-product or cross-agent claim is promoted;
- evidence is recorded in the template-compatible format.
