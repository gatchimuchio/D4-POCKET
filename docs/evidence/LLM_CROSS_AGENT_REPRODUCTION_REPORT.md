# LLM Cross-Agent Reproduction Report

Status: Block L5 bounded reproduction evidence
Date: 2026-06-04
Scope: Controlled LLM-readable extension substrate reproduction

## 1. Summary

Two independent LLM development-agent executions were run from the same baseline commit using the same task packet and bounded task.

Both agents produced the same bounded diff:

- add `model_output` to `NON_AUTHORITY_SOURCES` in `packages/shell_core/permission_ledger.py`;
- update `MANIFEST.sha256.json`;
- rely on the existing bounded extension conformance negative-case loop to prove `model_output` cannot become an authority source.

Outcome:

```yaml
outcome: reproduced_successfully
evidence_scope: CONFIG | INTERNAL_STATE | FIXTURE
claim_scope: bounded_cross_agent_llm_readable_extension_behavior
```

This report does not prove installed-product behavior, Windows release readiness, public standard adoption, broad ecosystem compatibility, or third-party interoperability.

## 2. Baseline and Task Packet

```yaml
baseline_commit: 48082469089e9a63ef939b51f864dfc26e4ae2c9
task_packet: docs/implementation/LLM_EXTENSION_TASK_PACKET.md
evidence_template: docs/implementation/LLM_EXTENSION_EVIDENCE_TEMPLATE.md
requested_task: add model_output as a non-authority source and verify the existing bounded extension negative-case path covers it
allowed_scope:
  - packages/shell_core/permission_ledger.py
  - tooling/conformance_tests/run_conformance_skeleton.py if necessary
  - README.md / CLAIM.md / VALIDATION.txt / MANIFEST.sha256.json if necessary
forbidden_scope:
  - schemas
  - production runtime expansion
  - Rust Broker implementation
  - Flutter UI
  - installer
  - Windows evidence
  - raw evidence
```

## 3. Agent Executions

| agent | execution id | baseline | commit | push | outcome |
| --- | --- | --- | --- | --- | --- |
| Agent A | `019e907a-4455-77f2-8616-7b1ae0981b77` | `48082469089e9a63ef939b51f864dfc26e4ae2c9` | not committed | not pushed | reproduced successfully |
| Agent B | `019e907a-9db9-7092-9d2a-0861a3277e6d` | `48082469089e9a63ef939b51f864dfc26e4ae2c9` | not committed | not pushed | reproduced successfully |

Both executions stayed inside the allowed write surface:

```text
M	MANIFEST.sha256.json
M	packages/shell_core/permission_ledger.py
```

Both executions reported:

- no unauthorized scope expansion attempt;
- no manual repair requirement;
- no schema change;
- no production runtime path addition;
- no Rust, Flutter, installer, Windows evidence, or raw evidence change;
- no release blocker closure.

## 4. Resulting Diff

Both agents independently produced the same substantive source change:

```diff
 NON_AUTHORITY_SOURCES = {
     "metadata",
+    "model_output",
     "previous_state",
 }
```

Both agents also updated the manifest hash for `packages/shell_core/permission_ledger.py`.

Consensus diff summary:

```text
 MANIFEST.sha256.json                     | 2 +-
 packages/shell_core/permission_ledger.py | 1 +
 2 files changed, 2 insertions(+), 1 deletion(-)
```

## 5. Validation Comparison

| check | Agent A | Agent B |
| --- | --- | --- |
| schema check | passed via `python3` fallback: `25 schemas, 25 examples, 27 negative fixtures` | passed via `python3` fallback: `25 schemas, 25 examples, 27 negative fixtures` |
| conformance skeleton | passed via `python3` fallback: `102 checks` | passed via `python3` fallback: `102 checks` |
| manifest check | passed | passed |
| release gate check | passed | passed |
| evidence bundle | passed: `3 release blockers preserved, release_ready=False, classification=development_evidence` | passed: `3 release blockers preserved, release_ready=False, classification=development_evidence` |
| release runtime assertions | passed: `9 passed, 0 failed, evidence_scope=CONFIG,FIXTURE,LIVE_RUNTIME` | passed: `9 passed, 0 failed, evidence_scope=CONFIG,FIXTURE,LIVE_RUNTIME` |
| diff whitespace check | not reported | passed with no output |

Agent B also ran a targeted probe:

```text
model_output_in_non_authority_sources=True
ledger_model_output_can_grant_authority=False
policy_allowed=False
policy_error_codes=non_authority_source_attempt
```

## 6. Boundary Comparison

| boundary | Agent A | Agent B | result |
| --- | --- | --- | --- |
| LLM output as authority | rejected through `NON_AUTHORITY_SOURCES` | rejected through `NON_AUTHORITY_SOURCES` plus targeted probe | reproduced |
| self-approval | no weakening | no weakening | preserved |
| permission widening | no widening | no widening | preserved |
| audit mapping | no weakening | no weakening | preserved |
| recovery mapping | no weakening | no weakening | preserved |
| content exposure | no weakening | no weakening | preserved |
| runtime neutrality | no runtime-specific logic added | no runtime-specific logic added | preserved |
| Shell Core BLUE-TANUKI coupling | no coupling added | no coupling added | preserved |
| hidden runtime path | no path added | no path added | preserved |

## 7. Classification

```yaml
classification:
  reproduced_successfully: true
  reproduced_with_bounded_differences: false
  failed_safely: false
  failed_through_boundary_violation: false
  inconclusive: false
```

Evidence classification:

```yaml
primary: CONFIG | INTERNAL_STATE | FIXTURE
not_installed_product_evidence: true
not_windows_release_evidence: true
not_public_standard_adoption_evidence: true
not_broad_ecosystem_compatibility_evidence: true
```

Claim enabled by this report:

```text
GUI Shell has demonstrated bounded cross-agent LLM-readable extension behavior for one controlled non-authoritative extension task under its declared responsibility contracts.
```

Claims not enabled by this report:

- public standard status;
- broad ecosystem compatibility;
- third-party runtime interoperability;
- installed Windows product readiness;
- Rust Broker production authority cutover;
- owner GO.

## 8. Release Impact

- item: bounded cross-agent LLM-readable extension behavior
  classification: required_for_v1
  reason: two independent agent executions from the same baseline produced the same bounded diff and passed the required validation set.
  required_action: preserve this report and repeat or broaden reproduction only when claim scope expands.
  blocks_release: no

- item: Windows installed-path product evidence
  classification: release_blocker
  reason: this L5 report does not provide installed no-Python-runtime evidence, Windows installed-path broker proof, first-run proof, Setup Doctor proof, strict Windows validation, or owner GO.
  required_action: complete Track R product evidence before Windows-first product release.
  blocks_release: yes

- item: public standard or ecosystem claim
  classification: known_limitation
  reason: this report covers one controlled bounded extension task, not broad external adoption or ecosystem compatibility.
  required_action: collect broader external evidence only if the owner chooses to promote that claim.
  blocks_release: no

## 9. Final Decision

Block L5 passes for the bounded task defined in this report.

The result should be used only as bounded cross-agent LLM-readable extension evidence. It must not be promoted into product release proof, installed Windows evidence, public standard status, or broad ecosystem compatibility.
