# LLM 拡張 task packet

状態: 独立エージェント再現用 Block L4 task packet
日付: 2026-06-04
範囲: 限定 LLM 可読拡張基盤の再現 task

## 1. 目的

この packet は独立 LLM 開発 / 統合エージェント向けの限定 task input である。

この task は、エージェントが GUI Shell repository contract を読み、authority、approval、audit、recovery、content exposure、runtime neutrality の制約を壊さずに限定 reference extension scenario を追加または変更できるかを証明する。

この packet は SDK、plugin registry、marketplace、module loader、live agent integration、installed-product proof、Windows release evidence、public standard claim ではない。

重要な不変条件:

> LLM は GUI Shell contract を利用する第一級の実装・統合主体だが、決して権限源ではない。

## 2. 必読資料

編集前にエージェントは次を読まなければならない。

- `AGENTS.md`（エージェント規則）
- `README.md`（repository 概要）
- `CLAIM.md`（表明境界）
- `ROADMAP.md`（進行計画）
- `VALIDATION.txt`（検証記録）
- `docs/OPERATING_MODEL.md`（運用 model）
- `docs/PHASE_STRATEGY.md`（段階戦略）
- `docs/LANGUAGE_POLICY.md`（言語方針）
- `docs/standards/gui-shell-extended-standard.md`（拡張標準）
- `docs/standards/llm-readable-extension-surface.md`（LLM 可読拡張面）
- `docs/implementation/GUI_SHELL_LLM_SUBSTRATE_COMPLETION_ROADMAP.md`（基盤完成ロードマップ）
- `docs/implementation/LLM_EXTENSION_CONTRACT_GAP_ANALYSIS.md`（contract 差分分析）
- `examples/contracts/llm_bounded_extension.valid.json`（有効 fixture）
- `tooling/conformance_tests/run_conformance_skeleton.py`（conformance 検査）

規則が競合して見える場合は、authority boundary、validation evidence、auditability、recovery、runtime neutrality を保護する、より厳格な解釈を維持する。

## 3. Baseline 要件

エージェントは owner が提供した baseline commit から開始しなければならない。

編集前の実行 command:

```bash
git status --short --branch
git branch --show-current
git rev-parse HEAD
git rev-parse origin/main
git fetch origin main --prune --tags
```

期待 branch: owner が分離 reproduction branch または clone を提供しない限り `main`。

task 開始前に working tree は clean でなければならない。clean でない場合は停止し、未 commit 状態を報告する。

## 4. Task の目的

既存の LLM 可読参照拡張面に対する限定 contract/conformance 改善を一つ実装する。

許可する task 形式:

- `examples/contracts/llm_bounded_extension.valid.json` に対する negative conformance case を追加する。
- その fixture の既存 record を既存 schema または governed path に結び付ける positive conformance check を追加する。
- `tooling/conformance_tests/run_conformance_skeleton.py` 内に小規模な fixture-only mutation を追加する。
- 限定拡張の再現証拠形式を明確にする文書を追加する。
- repository が既に non-authoritative と定義した source の non-authority source handling を強化する。

task は既存 reference extension の次の性質を維持しなければならない。

- non-authoritative（非権限）。
- mock または fixture-based（fixture 基底）。
- contract/conformance evidence に限る。
- runtime-neutral（runtime 中立）。
- BLUE-TANUKI 固有ではない。
- installed-product proof ではない。
- 単独では cross-agent reproduction proof ではない。

## 5. 許可する path

エージェントは次のうち必要最小限の subset だけを変更してよい。

- `examples/contracts/llm_bounded_extension.valid.json`（限定 extension fixture）
- `tooling/conformance_tests/run_conformance_skeleton.py`（conformance 検査）
- `packages/shell_core/permission_ledger.py`（許可 ledger）
- `docs/implementation/LLM_EXTENSION_TASK_PACKET.md`（作業指示 packet）
- `docs/implementation/LLM_EXTENSION_EVIDENCE_TEMPLATE.md`（証拠 template）
- `docs/implementation/GUI_SHELL_LLM_SUBSTRATE_COMPLETION_ROADMAP.md`（完成ロードマップ）
- `docs/implementation/LLM_EXTENSION_CONTRACT_GAP_ANALYSIS.md`（contract 差分分析）
- `README.md`（repository 概要）
- `CLAIM.md`（表明境界）
- `VALIDATION.txt`（検証記録）
- `MANIFEST.sha256.json`（収録 manifest）

別ファイルが必要に見える場合は、編集前に停止し、既存の許可範囲が不十分である理由を報告する。

## 6. 禁止する変更

次を変更してはならない。

- 製品 runtime の実行経路。
- Rust broker implementation（broker 実装）。
- Flutter UI（利用者 UI）。
- installer script（導入 script）。
- Windows evidence collector（証拠収集器）。
- release evidence JSON（リリース証拠）。
- `specs/` 配下の schema file。
- 汎用 schema checker の挙動。
- BLUE-TANUKI runtime code（runtime 実装）。
- authority boundary design（権限境界設計）。
- approval semantics（承認意味論）。
- audit semantics（監査意味論）。
- recovery semantics（回復意味論）。
- 内容露出方針の意味論。
- repository backup discipline（backup 規律）。

次を追加してはならない。

- plugin registry（登録機構）。
- module loader（module 読込み機構）。
- SDK（開発 kit）。
- marketplace behavior（市場機構の挙動）。
- 実動する第三者 agent 統合。
- new dependency（新規依存）。
- runtime command dispatch（runtime command 配送）。
- privileged filesystem、process、network、credential、IPC、または update behavior（特権挙動）。

## 7. 必須 contract 面

採用する変更は既存の次の contract family を考慮しなければならない。

- runtime（実行主体）。
- adapter（接続境界）。
- runtime manifest（runtime 宣言）。
- adapter manifest（adapter 宣言）。
- capability（能力）。
- permission（権限許可）。
- approval（承認）。
- audit（監査）。
- recovery（回復）。
- content exposure（内容露出）。
- update/install（更新・導入）。
- 変更が agent-operated development behavior に関係する場合の agent runtime。

提案する変更を既存 contract で表現できない場合、正しい出力は contract gap report であり、即興 implementation ではない。

## 8. 必須 negative case

変更面について、少なくとも一つの governed negative case を維持または追加しなければならない。

有効な negative case category:

- authority escalation metadata（権限昇格 metadata）。
- self-approved sensitive behavior（自己承認した機微挙動）。
- undeclared capability（未宣言能力）。
- undeclared permission（未宣言許可）。
- missing audit evidence（監査証拠欠落）。
- missing recovery mapping（回復対応欠落）。
- 方針のない full content exposure。
- memory、generated output、generated config、metadata、tool response、GUI state、または previous state による authority 化の試行。
- Shell Core に入る runtime-specific logic。
- Shell Core に入る BLUE-TANUKI-specific logic。

negative case は schema validation、conformance、policy evaluation、catalog rejection、normalization quarantine、または同等の既存 validation path を通じて fail closed しなければならない。

## 9. 必須検証

最低限、次を実行する。

```bash
python tooling/schema_check/check_schemas.py || python3 tooling/schema_check/check_schemas.py
python tooling/conformance_tests/run_conformance_skeleton.py || python3 tooling/conformance_tests/run_conformance_skeleton.py
python3 tooling/manifest.py --check
python3 tooling/release_gate_check.py
python3 tooling/evidence_bundle.py --check
```

runtime-boundary code に触れた場合は次も実行する。

```bash
python3 tooling/release_runtime_assertions.py --check
```

Rust helper または Flutter file に触れた場合、owner がその範囲を明示的に許可していない限り task はこの packet を逸脱している。続行前に停止して報告する。

## 10. 期待出力

エージェントは次を生成しなければならない。

- bounded diff（限定差分）。
- validation command output（検証 command 出力）。
- evidence classification（証拠分類）。
- remaining blocker classification（残存 blocker 分類）。
- installed-product proof または cross-agent reproduction claim を行わないとの明記。
- 完成した `docs/implementation/LLM_EXTENSION_EVIDENCE_TEMPLATE.md` 互換 report。

## 11. 完了報告形式

完了報告は次を含まなければならない。

1. 要約（Summary）
2. Baseline commit（基準 commit）
3. Task packet の path と version/date
4. 変更ファイル（Changed files）
5. 実行した contract path
6. 追加または維持した negative case
7. Validation command と正確な output
8. Evidence classification（証拠分類）
9. Production/runtime behavior の変更状態
10. Release-gate への影響
11. LLM-substrate claim への影響
12. 存在する場合、無許可の範囲拡大試行
13. 存在する場合、必要な manual repair
14. Repository classification を伴う remaining risk
15. Working branch（作業 branch）
16. Commit hash、または `not committed`
17. Push result、または `not pushed`
18. push した場合の Remote HEAD verification
19. repository state を変更する main 直接作業を行った場合の backup ref
20. Rollback point（ロールバック地点）

## 12. 証拠分類規則

この packet で許可する分類:

- CONFIG（設定証拠）
- INTERNAL_STATE（内部状態証拠）
- FIXTURE（fixture 証拠）

この task を次に分類してはならない。

- installed-product evidence（インストール済み製品証拠）。
- Windows release evidence（Windows リリース証拠）。
- external evidence（外部証拠）。
- 単独での cross-agent reproduction evidence。
- 公開標準採用の証拠。

Cross-agent reproduction evidence は、owner が同じ baseline から少なくとも二つの独立 agent execution を実行し、生成 diff と validation result を比較した後に限り成立する。

## 13. 停止条件

次の場合は追加編集前に停止して報告する。

- 新規 schema が必要に見える。
- production runtime behavior が必要に見える。
- authority semantics の変更が必要である。
- approval、audit、recovery、または content exposure rule が不十分に見える。
- Rust、Flutter、installer、Windows evidence、または release evidence file が必要に見える。
- validation が失敗し、修正が allowed path を越える。
- working tree に無関係な dirty change がある。
- remote push または backup verification が失敗する。

## 14. 成功条件

task は次をすべて満たす場合に限り成功する。

- diff が allowed scope 内に留まる。
- 既存 contract を迂回せず使用する。
- 少なくとも一つの関連 negative path を検証する。
- validation が合格する。
- release blocker を誤って閉じない。
- installed-product または cross-agent claim を昇格させない。
- evidence を template 互換形式で記録する。
