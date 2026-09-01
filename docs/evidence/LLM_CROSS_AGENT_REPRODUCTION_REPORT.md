# LLM エージェント間再現報告

状態: Block L5 の限定再現証拠
日付: 2026-06-04
範囲: 管理された LLM 可読拡張基盤の再現

## 1. 要約

同じ task packet と限定 task を使い、同一の baseline commit から独立した二つの LLM 開発エージェント実行を行った。

両エージェントは同一の限定差分を生成した。

- `packages/shell_core/permission_ledger.py` の `NON_AUTHORITY_SOURCES` に `model_output` を追加する。
- `MANIFEST.sha256.json` を更新する。
- 既存の限定拡張 conformance negative-case loop により、`model_output` が権限源になれないことを証明する。

結果:

```yaml
outcome: reproduced_successfully
evidence_scope: CONFIG | INTERNAL_STATE | FIXTURE
claim_scope: bounded_cross_agent_llm_readable_extension_behavior
```

本報告は、インストール済み製品の挙動、Windows リリース準備完了、公開標準への採用、広範な ecosystem 互換性、または第三者相互運用性を証明しない。

## 2. Baseline と task packet

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

## 3. エージェント実行

| エージェント | execution id（実行識別子） | baseline | commit | push | 結果 |
| --- | --- | --- | --- | --- | --- |
| エージェント A | `019e907a-4455-77f2-8616-7b1ae0981b77` | `48082469089e9a63ef939b51f864dfc26e4ae2c9` | not committed（未 commit） | not pushed（未 push） | reproduced successfully（再現成功） |
| エージェント B | `019e907a-9db9-7092-9d2a-0861a3277e6d` | `48082469089e9a63ef939b51f864dfc26e4ae2c9` | not committed（未 commit） | not pushed（未 push） | reproduced successfully（再現成功） |

両実行は許可された書込み範囲内に留まった。

```text
M	MANIFEST.sha256.json
M	packages/shell_core/permission_ledger.py
```

両実行から次が報告された。

- 無許可の範囲拡大を試みていない。
- 手動修復を必要としない。
- schema を変更していない。
- 製品 runtime 経路を追加していない。
- Rust、Flutter、installer、Windows evidence、raw evidence を変更していない。
- release blocker を解消していない。

## 4. 生成された差分

両エージェントは独立に同じ実質的な source 変更を生成した。

```diff
 NON_AUTHORITY_SOURCES = {
     "metadata",
+    "model_output",
     "previous_state",
 }
```

両エージェントは `packages/shell_core/permission_ledger.py` の manifest hash も更新した。

合意差分の要約:

```text
 MANIFEST.sha256.json                     | 2 +-
 packages/shell_core/permission_ledger.py | 1 +
 2 files changed, 2 insertions(+), 1 deletion(-)
```

## 5. 検証比較

| 検査 | エージェント A | エージェント B |
| --- | --- | --- |
| schema check（schema 検査） | `python3` fallback により合格: `25 schemas, 25 examples, 27 negative fixtures` | `python3` fallback により合格: `25 schemas, 25 examples, 27 negative fixtures` |
| conformance skeleton（適合検査） | `python3` fallback により合格: `102 checks` | `python3` fallback により合格: `102 checks` |
| manifest check（manifest 検査） | passed（合格） | passed（合格） |
| release gate check（関門検査） | passed（合格） | passed（合格） |
| evidence bundle（証拠 bundle） | passed（合格）: `3 release blockers preserved, release_ready=False, classification=development_evidence` | passed（合格）: `3 release blockers preserved, release_ready=False, classification=development_evidence` |
| release runtime assertion（runtime 表明検査） | passed（合格）: `9 passed, 0 failed, evidence_scope=CONFIG,FIXTURE,LIVE_RUNTIME` | passed（合格）: `9 passed, 0 failed, evidence_scope=CONFIG,FIXTURE,LIVE_RUNTIME` |
| diff whitespace check（空白検査） | not reported（未報告） | 出力なしで passed（合格） |

Agent B は対象を絞った probe も実行した。

```text
model_output_in_non_authority_sources=True
ledger_model_output_can_grant_authority=False
policy_allowed=False
policy_error_codes=non_authority_source_attempt
```

## 6. 境界比較

| 境界 | エージェント A | エージェント B | 結果 |
| --- | --- | --- | --- |
| LLM output の authority 化 | `NON_AUTHORITY_SOURCES` を通じて拒否 | `NON_AUTHORITY_SOURCES` と対象 probe を通じて拒否 | reproduced（再現） |
| self-approval（自己承認） | no weakening（弱化なし） | no weakening（弱化なし） | preserved（維持） |
| permission widening（許可拡大） | no widening（拡大なし） | no widening（拡大なし） | preserved（維持） |
| audit mapping（監査対応） | no weakening（弱化なし） | no weakening（弱化なし） | preserved（維持） |
| recovery mapping（回復対応） | no weakening（弱化なし） | no weakening（弱化なし） | preserved（維持） |
| content exposure（内容露出） | no weakening（弱化なし） | no weakening（弱化なし） | preserved（維持） |
| runtime neutrality（runtime 中立性） | runtime 固有 logic の追加なし | runtime 固有 logic の追加なし | preserved（維持） |
| Shell Core と BLUE-TANUKI の coupling | coupling の追加なし | coupling の追加なし | preserved（維持） |
| hidden runtime path（隠れた経路） | path の追加なし | path の追加なし | preserved（維持） |

## 7. 分類

```yaml
classification:
  reproduced_successfully: true
  reproduced_with_bounded_differences: false
  failed_safely: false
  failed_through_boundary_violation: false
  inconclusive: false
```

証拠分類:

```yaml
primary: CONFIG | INTERNAL_STATE | FIXTURE
not_installed_product_evidence: true
not_windows_release_evidence: true
not_public_standard_adoption_evidence: true
not_broad_ecosystem_compatibility_evidence: true
```

本報告により可能となる表明:

```text
GUI Shell has demonstrated bounded cross-agent LLM-readable extension behavior for one controlled non-authoritative extension task under its declared responsibility contracts.
```

本報告では可能とならない表明:

- 公開標準としての状態。
- 広範な ecosystem 互換性。
- 第三者 runtime の相互運用性。
- インストール済み Windows 製品の準備完了。
- Rust Broker の製品権限 cutover。
- 所有者の GO。

## 8. リリースへの影響

- item: 限定されたエージェント間 LLM 可読拡張挙動
  classification: required_for_v1（分類）
  reason: 同じ baseline からの二つの独立 agent execution が、同一の限定 diff を生成し、必須 validation set に合格した。
  required_action: 本報告を維持し、claim scope が拡大する場合に限り reproduction を反復または拡張する。
  blocks_release: no（リリース阻止）

- item: Windows インストール済み経路の製品証拠
  classification: release_blocker（分類）
  reason: この L5 report は installed no-Python-runtime evidence、Windows installed-path broker proof、first-run proof、Setup Doctor proof、strict Windows validation、owner GO を提供しない。
  required_action: Windows-first product release 前に Track R の製品証拠を完成させる。
  blocks_release: yes（リリース阻止）

- item: 公開標準または ecosystem に関する表明
  classification: known_limitation（分類）
  reason: この report が扱うのは一つの管理された限定 extension task であり、広範な外部採用または ecosystem compatibility ではない。
  required_action: owner がその claim の昇格を選ぶ場合に限り、より広範な external evidence を収集する。
  blocks_release: no（リリース阻止）

## 9. 最終判断

本報告で定義した限定 task について Block L5 は合格する。

この結果は、限定されたエージェント間 LLM 可読拡張の証拠としてのみ使う。製品リリース証拠、インストール済み Windows 証拠、公開標準としての状態、または広範な ecosystem 互換性へ昇格させてはならない。
