# LLM Extension Contract Gap Analysis

Status: Block L1 contract sufficiency audit
Date: 2026-06-04
Scope: GUI Shell LLM-readable extension substrate contract analysis

## 1. 目的と境界

この文書は、GUI Shell が bounded な LLM-built extension を既存契約だけで安全に表現できるか、または `extension_manifest` / `extension_submission` のような新しい machine-readable contract が必要かを判定する。

この Block L1 は contract / documentation analysis であり、runtime 実装、schema 追加、conformance harness 追加、Flutter / Rust / installer 変更、Windows evidence 更新は行わない。

Evidence classification:

| evidence | class | proves | does not prove |
| --- | --- | --- | --- |
| `specs/*.schema.json` inspection | CONFIG | 既存 schema が表現可能な contract fields | runtime enforcement |
| `examples/contracts/*.json` inspection | FIXTURE | valid / invalid fixture の contract coverage | installed product behavior |
| `tooling/conformance_tests/run_conformance_skeleton.py` inspection | CONFIG / INTERNAL_STATE / FIXTURE | 既存 conformance target と negative checks | L3 reference extension completion |
| `packages/runtime_catalog` / `packages/agent_runtime` inspection | INTERNAL_STATE | catalog / agent helper の current validation semantics | Rust broker active product cutover |

Critical invariant:

> LLMs are first-class implementation and integration consumers of GUI Shell contracts, but are never authority sources.

## 2. GUI Shell における LLM-built extension の定義

LLM-built extension は、LLM development / integration agent が GUI Shell の既存 contract を読んで追加する bounded な runtime / tool / service / adapter / diagnostic integration である。

有効な表現は次のいずれかに束ねる。

- Runtime / tool / service target: `runtime.schema.json` または `runtime_manifest.schema.json`
- Adapter boundary: `adapter.schema.json` または `adapter_manifest.schema.json`
- Agent-operated development surface: `agent_runtime.schema.json`、`agent_session.schema.json`、`agent_workspace.schema.json`、`agent_task.schema.json`、`agent_tool_call.schema.json`、`agent_diff.schema.json`
- Governed action semantics: capability、permission、approval、audit、recovery、content exposure、update/install contracts

LLM-built extension は以下をしてはならない。

- LLM output、memory、tool response、adapter metadata、generated config、GUI state を authority source にする。
- 自分の sensitive action を自己承認する。
- permission / approval / audit / recovery / content exposure / runtime neutrality を bypass する。
- runtime-specific logic を Shell Core に持ち込む。
- BLUE-TANUKI-specific logic を Shell Core に持ち込む。

## 3. 監査した契約面

| contract family | inspected files | current coverage |
| --- | --- | --- |
| Runtime identity and neutrality | `specs/runtime.schema.json`, `specs/runtime_manifest.schema.json`, `examples/contracts/runtime_manifest.valid.json` | runtime kind / type、supported platforms、tools、ports、storage、network policy、capabilities、permissions、audit / recovery profile、trust profile、signed manifest を表現できる。 |
| Adapter boundary | `specs/adapter.schema.json`, `specs/adapter_manifest.schema.json`, `docs/specs/adapter-conformance.md` | `authority_strip=true`、transport、declared capabilities、content exposure policy、untrusted metadata、signed manifest を表現できる。 |
| Capability / permission | `specs/capability.schema.json`, `specs/permission.schema.json` | capability risk、default permission、permission decision、scope、source を表現できる。 |
| Approval | `specs/approval.schema.json` | approval status、content visibility、payload hash、editable fields、sealed / hidden / sacred / authority fields を表現できる。 |
| Audit | `specs/audit.schema.json` | event id、actor、action、target、result、payload hash、previous event hash、metadata を表現できる。 |
| Recovery | `specs/recovery.schema.json` | recovery class、severity、operator-visible message、retry safety、steps、user action requirement を表現できる。 |
| Content exposure | `specs/content_exposure.schema.json` | `none` default、allowed visibility、redaction requirement を表現できる。 |
| Update / install | `specs/update.schema.json` | channel、auto update、signature required、rollback enabled を表現できる。 |
| Agent runtime | `docs/specs/agent-runtime.md`, `specs/agent_*.schema.json`, `packages/agent_runtime/contract.py` | workspace boundary、secret path denial、tool call permission、git push approval、diff audit、rollback candidate、advisory-only auto permission を表現できる。 |
| Runtime catalog | `docs/specs/runtime-catalog.md`, `packages/runtime_catalog/catalog.py` | signed runtime / adapter manifest registration、metadata authority attempt detection、manifest cannot grant authority を表現できる。 |
| Reference adapter examples | `packages/blue_tanuki_adapter`, `examples/contracts/*.valid.json` | runtime-specific behavior を adapter boundary に置く既存例がある。 |

## 4. 既存契約で表現できること

bounded extension を runtime / adapter / tool / service target として扱う場合、既存 contract families で以下を表現できる。

- extension identity: `runtime_id`、`adapter_id`、`agent_runtime_id`
- target type: runtime type / kind、transport、supported platform
- capability declaration: `capabilities`、`declared_capabilities`
- permission requirements: permission IDs、scope、decision、source
- approval requirements: approval status、payload hash、editable fields、protected fields
- content exposure: content exposure policy id、visibility enum、redaction requirement
- audit mapping: audit event id、payload hash、hash-chain link fields
- recovery mapping: recovery id、class、message、steps、safe retry
- adapter / runtime boundary: signed manifest、authority strip、untrusted metadata、transport
- update/install responsibility: signature required、rollback enabled、channel
- agent execution surface: workspace confinement、tool call permission、git push approval、diff audit、rollback candidate

このため、Block L3 の bounded reference extension は、新しい schema を作らずに、既存の runtime manifest + adapter manifest + capability / permission / approval / audit / recovery / content exposure records として定義できる。

## 5. 既存 negative coverage

既存 fixture / conformance には、L3 の初期 proof target に必要な負例の多くが存在する。

| required negative case | existing coverage |
| --- | --- |
| authority escalation metadata | `examples/contracts/invalid/adapter_authority_escalation.invalid.json`, `adapter_manifest_authority_escalation.invalid.json`, `test_adapter_manifest_authority_escalation_rejected`, `test_policy_evaluator_ignores_adapter_metadata_authority` |
| unsigned runtime manifest | `examples/contracts/invalid/runtime_manifest_unsigned.invalid.json`, `test_runtime_manifest_invalid_fixture_rejected` |
| adapter without authority strip | `adapter.schema.json`, `adapter_manifest.schema.json`, `test_adapter_authority_strip_schema` |
| undeclared or missing permission mapping | `examples/contracts/invalid/agent_tool_call_missing_permission.invalid.json`, `test_agent_shell_command_requires_permission_mapping`, policy evaluator unknown / denied permission tests |
| self-reported approval | policy evaluator tests reject missing / unknown / unapproved RuntimeState approval and do not trust action self-report |
| audit omission | `agent_session_missing_audit.invalid.json`, `agent_task_missing_audit.invalid.json`, `test_policy_evaluator_rejects_missing_audit_event`, `test_agent_generated_diff_must_be_auditable` |
| recovery omission | `test_policy_evaluator_rejects_missing_recovery_action`, `test_policy_evaluator_rejects_unknown_recovery_id` |
| content exposure bypass | `content_exposure_default_full.invalid.json`, content projection / visibility conformance |
| workspace escape | `agent_workspace_allows_outside.invalid.json`, `test_agent_workspace_outside_access_default_deny` |
| secret path access | `test_agent_secret_path_read_default_deny` |
| generated diff without rollback | `agent_diff_missing_rollback.invalid.json`, `AgentRuntimeContract.state_change_has_rollback` |
| generated config attempting authority | runtime catalog and normalization metadata authority detection paths |

## 6. Gap 判定

### 6.1 Blocking gap は現時点ではない

bounded reference extension を「GUI Shell contract に登録される runtime / adapter / tool / service / agent-runtime integration」と定義する限り、既存 schema families は L3 conformance harness の初期設計に十分である。

したがって、この L1 判定では新規 schema を追加しない。Block L2 は現時点では実行不要である。

### 6.2 ただし L4 / L5 用の提出・再現証跡 surface は未整備

既存 contract は extension runtime behavior と authority boundary を表現できる。一方で、cross-agent reproduction の運用証跡として必要になる次の情報は、現時点では専用の machine-readable contract ではない。

- baseline commit
- agent identity / agent run identifier
- prompt / task packet identity
- allowed paths / forbidden paths
- produced diff identity
- validation command outputs
- manual repair status
- unauthorized scope expansion attempt
- comparison result between independent agents

これは Block L4 の task packet / evidence template でまず文書化すべき reproduction evidence surface であり、Block L3 の bounded extension contract を始める前に新 schema を作る理由にはならない。

L4 / L5 実行時に、文書テンプレートでは不十分で validation 可能な structured evidence が必要だと確認された場合のみ、最小の `extension_submission` または reproduction evidence schema を検討する。

## 7. Decision

Decision: existing contracts suffice for Block L3 when an LLM-built extension is represented as a bounded runtime / adapter / tool / service / agent-runtime integration.

Immediate consequence:

- Do not create `specs/extension_manifest.schema.json` now.
- Do not create `specs/extension_submission.schema.json` now.
- Skip Block L2 for the current roadmap path.
- Proceed to Block L3 using existing contract families.

Reopen Block L2 only if Block L3 proves that an essential bounded extension property cannot be represented by existing runtime, adapter, capability, permission, approval, audit, recovery, content exposure, update/install, or agent-runtime contracts.

## 8. Exact next conformance block: Block L3

Recommended L3 objective:

Create a deterministic bounded reference extension conformance scenario using existing contracts only.

Recommended reference shape:

- `runtime_manifest`: `tool_runtime` or `agent_runtime` with signed manifest, localhost/mock transport assumptions, declared diagnostic capability, audit profile, recovery profile, and rollback/update policy references.
- `adapter_manifest`: matching adapter with `authority_strip=true`, signed manifest, declared capabilities, and content exposure policy id.
- `capability`: one non-authoritative diagnostic/read capability.
- `permission`: explicit permission record for that capability, with source constrained to user/policy/system_default as existing schema permits.
- `approval`: required only if the chosen action is sensitive; self-reported approval must not be accepted.
- `audit`: every accepted or rejected governed action maps to an audit event id and payload hash.
- `recovery`: malformed or unauthorized extension state maps to RecoveryAction or SUSPEND-equivalent failure.
- `content_exposure`: default visibility remains `none`; full content is not projected unless explicitly allowed.

Required positive path:

- A bounded reference extension can be represented by runtime and adapter manifests.
- It can expose only declared capability.
- It emits audit evidence.
- It maps failure to recovery.
- It does not put runtime-specific behavior in Shell Core.

Required negative paths:

- Reject adapter metadata that attempts permission / trust / authority escalation.
- Reject missing or unsigned manifests.
- Reject undeclared capability or permission use.
- Reject missing audit mapping.
- Reject missing recovery mapping.
- Reject self-approved sensitive behavior.
- Reject full content exposure without policy.
- Reject generated config, metadata, memory, tool response, or UI state as authority.
- Reject Shell Core imports or code paths that make the bounded extension runtime-specific.

Evidence classification for L3 must remain contract/conformance demonstration. It must not be promoted to installed-product evidence, Windows evidence, public standard adoption evidence, or cross-agent reproduction evidence.

## 9. Release and claim impact

This L1 analysis does not close any Windows-first product release blocker.

Preserved blockers:

- Rust Security Broker production authority cutover remains incomplete.
- Installed no-Python-runtime evidence remains incomplete.
- Windows installed-path first-run / broker / Setup Doctor / UIAutomation evidence remains incomplete.
- Strict Windows release validation remains incomplete.
- Owner GO is not recorded.
- LLM-readable substrate is definition-locked but not yet demonstrated by bounded extension conformance or cross-agent reproduction.

Claim impact:

- Architecture-defined claim remains allowed: GUI Shell is designed as an LLM-readable responsibility substrate.
- Demonstrated bounded extension claim remains blocked until L3 passes.
- Cross-agent reproduction claim remains blocked until L5 passes.
- Public standard / ecosystem claim remains unproven.

## 10. Files intentionally unchanged

No runtime, schema, conformance, Flutter, Rust, installer, evidence, or report result files are changed by this analysis.

Intentionally unchanged surfaces:

- `specs/*.schema.json`
- `examples/contracts/*.json`
- `tooling/conformance_tests/run_conformance_skeleton.py`
- `packages/*`
- `native/*`
- `apps/*`
- `installer/*`
- `docs/reports/*`
