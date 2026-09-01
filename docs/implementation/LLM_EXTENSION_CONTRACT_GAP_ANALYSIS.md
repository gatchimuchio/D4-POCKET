# LLM 拡張 contract の差分分析

状態: Block L1 contract 充足性監査
日付: 2026-06-04
範囲: GUI Shell の LLM 可読拡張基盤に対する contract 分析

## 1. 目的と境界

この文書は、GUI Shell が bounded な LLM-built extension を既存契約だけで安全に表現できるか、または `extension_manifest` / `extension_submission` のような新しい machine-readable contract が必要かを判定する。

この Block L1 は contract / documentation analysis であり、runtime 実装、schema 追加、conformance harness 追加、Flutter / Rust / installer 変更、Windows evidence 更新は行わない。

証拠分類:

| 証拠 | class | 証明すること | 証明しないこと |
| --- | --- | --- | --- |
| `specs/*.schema.json` の検査 | CONFIG（設定） | 既存 schema が表現可能な contract fields | runtime の強制 |
| `examples/contracts/*.json` の検査 | FIXTURE（固定標本） | valid / invalid fixture の contract coverage | インストール済み製品の挙動 |
| `tooling/conformance_tests/run_conformance_skeleton.py` の検査 | CONFIG / INTERNAL_STATE / FIXTURE（複合証拠） | 既存 conformance target と negative checks | L3 reference extension の完成 |
| `packages/runtime_catalog` / `packages/agent_runtime` の検査 | INTERNAL_STATE（内部状態） | catalog / agent helper の current validation semantics | Rust broker の active product cutover |

重要な不変条件:

> LLM は GUI Shell contract を利用する第一級の実装・統合主体だが、決して権限源ではない。

## 2. GUI Shell における LLM-built extension の定義

LLM-built extension は、LLM development / integration agent が GUI Shell の既存 contract を読んで追加する bounded な runtime / tool / service / adapter / diagnostic integration である。

有効な表現は次のいずれかに束ねる。

- Runtime / tool / service target: `runtime.schema.json` または `runtime_manifest.schema.json`
- Adapter boundary: `adapter.schema.json` または `adapter_manifest.schema.json`
- エージェント運用開発面: `agent_runtime.schema.json`、`agent_session.schema.json`、`agent_workspace.schema.json`、`agent_task.schema.json`、`agent_tool_call.schema.json`、`agent_diff.schema.json`
- 統制 action の意味論: capability、permission、approval、audit、recovery、content exposure、update/install contract

LLM-built extension は以下をしてはならない。

- LLM output、memory、tool response、adapter metadata、generated config、GUI state を authority source にする。
- 自分の sensitive action を自己承認する。
- permission / approval / audit / recovery / content exposure / runtime neutrality を bypass する。
- runtime-specific logic を Shell Core に持ち込む。
- BLUE-TANUKI-specific logic を Shell Core に持ち込む。

## 3. 監査した契約面

| contract family（契約族） | 監査したファイル | 現在の coverage |
| --- | --- | --- |
| Runtime の identity と neutrality | `specs/runtime.schema.json`, `specs/runtime_manifest.schema.json`, `examples/contracts/runtime_manifest.valid.json` | runtime kind / type、supported platforms、tools、ports、storage、network policy、capabilities、permissions、audit / recovery profile、trust profile、signed manifest を表現できる。 |
| Adapter の境界 | `specs/adapter.schema.json`, `specs/adapter_manifest.schema.json`, `docs/specs/adapter-conformance.md` | `authority_strip=true`、transport、declared capabilities、content exposure policy、untrusted metadata、signed manifest を表現できる。 |
| Capability / permission（能力・許可） | `specs/capability.schema.json`, `specs/permission.schema.json` | capability risk、default permission、permission decision、scope、source を表現できる。 |
| Approval（承認） | `specs/approval.schema.json` | approval status、content visibility、payload hash、editable fields、sealed / hidden / sacred / authority fields を表現できる。 |
| Audit（監査） | `specs/audit.schema.json` | event id、actor、action、target、result、payload hash、previous event hash、metadata を表現できる。 |
| Recovery（回復） | `specs/recovery.schema.json` | recovery class、severity、operator-visible message、retry safety、steps、user action requirement を表現できる。 |
| Content exposure（内容露出） | `specs/content_exposure.schema.json` | `none` default、allowed visibility、redaction requirement を表現できる。 |
| Update / install（更新・導入） | `specs/update.schema.json` | channel、auto update、signature required、rollback enabled を表現できる。 |
| Agent runtime（エージェント実行） | `docs/specs/agent-runtime.md`, `specs/agent_*.schema.json`, `packages/agent_runtime/contract.py` | workspace boundary、secret path denial、tool call permission、git push approval、diff audit、rollback candidate、advisory-only auto permission を表現できる。 |
| Runtime catalog（実行目録） | `docs/specs/runtime-catalog.md`, `packages/runtime_catalog/catalog.py` | signed runtime / adapter manifest の登録、metadata authority attempt の検出、manifest が authority を付与できないことを表現できる。 |
| Reference adapter の例 | `packages/blue_tanuki_adapter`, `examples/contracts/*.valid.json` | runtime-specific behavior を adapter boundary に置く既存例がある。 |

## 4. 既存契約で表現できること

bounded extension を runtime / adapter / tool / service target として扱う場合、既存 contract families で以下を表現できる。

- extension の identity: `runtime_id`、`adapter_id`、`agent_runtime_id`
- target の type: runtime type / kind、transport、supported platform
- capability の宣言: `capabilities`、`declared_capabilities`
- permission の要件: permission ID、scope、decision、source
- approval の要件: approval status、payload hash、editable field、protected field
- content exposure（内容露出）: policy id、visibility enum、redaction requirement
- audit の対応付け: audit event id、payload hash、hash-chain link field
- recovery の対応付け: recovery id、class、message、step、safe retry
- adapter / runtime の境界: signed manifest、authority strip、untrusted metadata、transport
- update/install の責任: signature required、rollback enabled、channel
- agent の実行面: workspace confinement、tool call permission、git push approval、diff audit、rollback candidate

このため、Block L3 の bounded reference extension は、新しい schema を作らずに、既存の runtime manifest + adapter manifest + capability / permission / approval / audit / recovery / content exposure records として定義できる。

## 5. 既存 negative coverage

既存 fixture / conformance には、L3 の初期 proof target に必要な負例の多くが存在する。

| 必須の negative case | 既存の coverage |
| --- | --- |
| authority escalation metadata（権限昇格） | `examples/contracts/invalid/adapter_authority_escalation.invalid.json`, `adapter_manifest_authority_escalation.invalid.json`, `test_adapter_manifest_authority_escalation_rejected`, `test_policy_evaluator_ignores_adapter_metadata_authority` |
| unsigned runtime manifest（未署名） | `examples/contracts/invalid/runtime_manifest_unsigned.invalid.json`, `test_runtime_manifest_invalid_fixture_rejected` |
| authority strip のない adapter | `adapter.schema.json`, `adapter_manifest.schema.json`, `test_adapter_authority_strip_schema` |
| 未宣言または欠落した permission mapping | `examples/contracts/invalid/agent_tool_call_missing_permission.invalid.json`, `test_agent_shell_command_requires_permission_mapping`, policy evaluator の unknown / denied permission test |
| self-reported approval（自己申告承認） | policy evaluator test は missing / unknown / unapproved の RuntimeState approval を拒否し、action の自己申告を信頼しない |
| audit の省略 | `agent_session_missing_audit.invalid.json`, `agent_task_missing_audit.invalid.json`, `test_policy_evaluator_rejects_missing_audit_event`, `test_agent_generated_diff_must_be_auditable` |
| recovery の省略 | `test_policy_evaluator_rejects_missing_recovery_action`, `test_policy_evaluator_rejects_unknown_recovery_id` |
| content exposure の迂回 | `content_exposure_default_full.invalid.json`, content projection / visibility の conformance |
| workspace の逸脱 | `agent_workspace_allows_outside.invalid.json`, `test_agent_workspace_outside_access_default_deny` |
| secret path への access | `test_agent_secret_path_read_default_deny` |
| rollback のない generated diff | `agent_diff_missing_rollback.invalid.json`, `AgentRuntimeContract.state_change_has_rollback` |
| authority を試みる generated config | runtime catalog と normalization metadata における authority 検出経路 |

## 6. Gap 判定

### 6.1 Blocking gap は現時点ではない

bounded reference extension を「GUI Shell contract に登録される runtime / adapter / tool / service / agent-runtime integration」と定義する限り、既存 schema families は L3 conformance harness の初期設計に十分である。

したがって、この L1 判定では新規 schema を追加しない。Block L2 は現時点では実行不要である。

### 6.2 ただし L4 / L5 用の提出・再現証跡 surface は未整備

既存 contract は extension runtime behavior と authority boundary を表現できる。一方で、cross-agent reproduction の運用証跡として必要になる次の情報は、現時点では専用の machine-readable contract ではない。

- baseline commit（基準 commit）
- agent identity / agent run identifier（実行識別）
- prompt / task packet identity（指示識別）
- allowed path / forbidden path（範囲）
- produced diff identity（差分識別）
- validation command output（検証出力）
- manual repair status（手動修復状態）
- 無許可の scope expansion attempt
- independent agent 間の comparison result（比較結果）

これは Block L4 の task packet / evidence template でまず文書化すべき reproduction evidence surface であり、Block L3 の bounded extension contract を始める前に新 schema を作る理由にはならない。

L4 / L5 実行時に、文書テンプレートでは不十分で validation 可能な structured evidence が必要だと確認された場合のみ、最小の `extension_submission` または reproduction evidence schema を検討する。

## 7. 判断

判断: LLM-built extension を限定された runtime / adapter / tool / service / agent-runtime integration として表現する場合、既存 contract で Block L3 に足りる。

直ちに適用する帰結:

- 現時点では `specs/extension_manifest.schema.json` を作成しない。
- 現時点では `specs/extension_submission.schema.json` を作成しない。
- 現行 roadmap 経路では Block L2 を飛ばす。
- 既存 contract family を使って Block L3 へ進む。

Block L3 により、限定拡張に必須の性質を既存の runtime、adapter、capability、permission、approval、audit、recovery、content exposure、update/install、または agent-runtime contract では表現できないと判明した場合に限り、Block L2 を再開する。

## 8. 次に行う正確な conformance block: Block L3

推奨する L3 の目的:

既存 contract だけを使い、決定的で限定された参照拡張の conformance scenario を作る。

推奨する参照形式:

- `runtime_manifest`: `tool_runtime` または `agent_runtime`。signed manifest、localhost/mock transport assumption、declared diagnostic capability、audit profile、recovery profile、rollback/update policy reference を伴う。
- `adapter_manifest`: 対応する adapter。`authority_strip=true`、signed manifest、declared capability、content exposure の policy id を伴う。
- `capability`: 非権限の diagnostic/read capability 一つ。
- `permission`: その capability に対する明示的 permission record。既存 schema が許す user/policy/system_default に source を制限する。
- `approval`: 選んだ action が sensitive な場合だけ必須。self-reported approval を受理してはならない。
- `audit`: accepted または rejected の各 governed action を audit event id と payload hash に対応付ける。
- `recovery`: malformed または unauthorized の extension state を RecoveryAction または SUSPEND 相当 failure に対応付ける。
- `content_exposure`: default visibility は `none` のまま。明示的に許可しない限り full content を projection しない。

必須の正常経路:

- 限定 reference extension を runtime manifest と adapter manifest で表現できる。
- 宣言した capability だけを露出できる。
- audit evidence を出力する。
- failure を recovery に対応付ける。
- runtime 固有の挙動を Shell Core に置かない。

必須の拒否経路:

- permission / trust / authority の昇格を試みる adapter metadata を拒否する。
- 欠落または未署名の manifest を拒否する。
- 未宣言の capability または permission 使用を拒否する。
- audit mapping の欠落を拒否する。
- recovery mapping の欠落を拒否する。
- 自己承認した sensitive behavior を拒否する。
- 方針のない full content exposure を拒否する。
- generated config、metadata、memory、tool response、または UI state を権限とすることを拒否する。
- 限定拡張を runtime 固有にする Shell Core import または code path を拒否する。

L3 の証拠分類は `contract/conformance demonstration` のままにする。`installed-product evidence`、Windows evidence、`public standard adoption evidence`、または `cross-agent reproduction evidence` へ昇格させてはならない。

## 9. リリースと表明への影響

この L1 分析は Windows-first 製品の release blocker を一つも解消しない。

維持する blocker:

- Rust Security Broker の製品権限 cutover は未完了のままである。
- インストール済み製品の Python runtime 非依存証拠は未完了のままである。
- Windows インストール済み経路の first-run / broker / Setup Doctor / UIAutomation 証拠は未完了のままである。
- 厳格な Windows リリース検証は未完了のままである。
- Owner GO は記録されていない。
- LLM 可読基盤は定義が固定されている。その後、限定 reference extension conformance は contract/conformance layer に追加されたが、エージェント間再現は未完了のままである。

表明への影響:

- 構造定義上の表明は引き続き許可される。すなわち、GUI Shell は LLM 可読な責任基盤として設計されている。
- 実証済み限定拡張という表明は、L3 が合格するまで blocked のままである。
- エージェント間再現の表明は、L5 が合格するまで blocked のままである。
- 公開標準 / ecosystem に関する表明は未証明のままである。

## 10. 意図的に変更しないファイル

この分析では runtime、schema、conformance、Flutter、Rust、installer、evidence、report result の各ファイルを変更しない。

意図的に変更しない範囲:

- `specs/*.schema.json`
- `examples/contracts/*.json`
- `tooling/conformance_tests/run_conformance_skeleton.py`
- `packages/*`
- `native/*`
- `apps/*`
- `installer/*`
- `docs/reports/*`
