# GUI-Shell 製品品質統合是正実装台帳
## v4.0 — Canonical Ledger / 独立ダブルチェック統合・実装修正確定版

- 作成日: 2026-06-05
- 対象スナップショット: `GUI-Shell-main (3).zip`
- 本書の効力: 今後の実装修正・再試験・claim管理・Codex指示の単一 canonical ledger
- 旧文書の扱い:
  - v2: Layer 0〜3 の原監査証拠として保持
  - v3: 分類・claim境界・製品gate設計の原型として保持
  - v3基準再監査報告書: 新規P0/P1と修正順序再編の証拠として保持
  - クロちゃん独立報告: 裏取り・実装切り分け・健全箇所の回帰基準として保持
  - 実施順序・完了条件・Codex指示は本 v4 が優先

## 0. 最終統合判定

GUI-Shell の設計方向と製品像は維持する。
一方、現行コードをフル製品スコープの `release ready` 又は `formal Windows acceptance passed` と扱うことはできない。

最初に閉じるべき欠陥は、ActionEnvelope の relation 不足そのものではない。
その一段上流にある authority source-of-truth である。

```text
現行の危険構造:
caller
  └─ payload.state / payload.action を構成
       └─ broker authority evaluator がその state を判定入力として参照

必要な製品構造:
caller
  └─ action request のみ提出
       └─ broker-owned registries が runtime / capability / permission /
          approval / recovery / audit を解決
              └─ canonical ActionEnvelope relation を判定
                   └─ broker が decision audit を生成・保存
```

現時点では dispatch が封印されているため被害経路は稼働していない。
しかし、authority cutover 又は実操作接続を有効化する前の絶対 blocker である。

## 1. 製品スコープ・claim境界・監査規則

本書では GUI-Shell の最終スコープを次として扱う。

> Real Operation Shell:
> 実際の Runtime / Adapter / Capability / Permission / Approval / Audit / Recovery / Formal Evidence state を、人間 operator と LLM 開発エージェントが安全に観測・操作できる、Windows-first の汎用 GUI 基盤。

現行の diagnostic / probe surface は途中観測用途として保持できるが、最終製品 claim と混同しない。

### 製品責任レイヤー

```text
Layer 0: 一般の商用デスクトップ GUI 製品としての品質 gate
Layer 1: Runtime Operation Shell としての実状態・ingress・表示真実性
Layer 2: LLM / Authority Control としての権限・完全性・監査
Layer 3: Formal Evidence / Distribution / Release Claim の閉包
```

### 分類

| 分類 | 意味 |
| --- | --- |
| `D-P0` | 既存実装に確認された、責任境界又は claim integrity を成立不能にする欠陥 |
| `D-P1` | 堅牢性・再現性・安全可用性を損なう実装欠陥 |
| `G-P0` | フル製品 release 前に成立必須だが、現在未成立の工程／証跡 |
| `G-P1` | 配布品質を閉じるための重要 gate |
| `S` | owner が v1 scope として確定する判断 |
| `V` | Windows 実機等で確認すべき事項 |

### 凍結 claim

以下は対応 gate が PASS するまで使用禁止とする。

| 凍結表現 | 解凍条件 |
| --- | --- |
| `release ready` / `production ready` | 対象 scope の D-P0 / G-P0 / release P1 が閉鎖し、formal acceptance PASS |
| `formal Windows acceptance passed` | exact clean commit 起点の raw evidence bundle を検証済み |
| `real operational dashboard complete` | synthetic/probe state を除去し real state 接続と integration PASS |
| `authority enforcement complete` | broker-owned state、relation、hash、audit、origin、ingress closure PASS |
| `tamper-proof audit` / `authentic evidence` | anchor / cryptographic verification / bundle provenance PASS |
| `signed / verified update` | 実暗号検証と distribution acceptance PASS |

## 2. 回帰破壊禁止の健全箇所

- command dispatch / authority cutover は `command_dispatch_enabled=false` / `authority_cutover_status=not_active` のまま維持する。
- `ApprovalQueue.protected_fields` の protected field 非編集性を維持する。
- permission decision vocabulary は `allow` / `approved` のみ許可し、語彙を緩めない。
- adapter は承認僭称や authority elevation を行えない状態を維持する。
- diagnostic filesystem/network/process paths は operation 接続時に混同しない。
- Rust `#![forbid(unsafe_code)]` は解除禁止。
- Windows release evidence validator の run isolation、field provenance、hash、aggregate surface shortcut rejection を維持する。
- Python `tooling/shell_snapshot.py` の正直な不足 evidence 表示は流用候補であり、synthetic 化しない。

## 3. D-P0 — Critical Implementation Defects

### D-P0-00: Authority decision が caller-supplied state を信頼する

Rust broker は authority 評価時、request payload に含まれる `state` と `action` を evaluator へ渡す。
persistent state は audit log / nonce 等を保持するが、runtime / capability / permission / approval / recovery の authoritative registry として判定に使われていない。

是正要件:

1. production broker-owned authoritative state model を定義する。
2. production authority operation から caller-supplied `state` を排除する。
3. request は action payload / operation request の提出に限定する。
4. runtime / capability / permission / approval / recovery は broker が内部 registry から解決する。
5. 既存 registry を流用できるか評価し、重複実装を避ける。
6. fixture/parity evaluator が必要なら production operation と隔離する。
7. closure 完了まで command dispatch と authority cutover は無効のまま維持する。

完了試験:

- caller が approved state を提出しても authorize できない。
- production operation に `state` を渡しても reject 又は無視される。
- broker-owned registry に存在しない permission / approval / recovery で decision が成立しない。

### D-P0-01: Authority origin が allowlist でなく denylist

authority source を caller 自己申告値として扱わず、broker 内部 issuer / provenance として付与する。
allowlist 又は内部 state relation に存在しない source は deny/SUSPEND する。

### D-P0-02: Canonical ActionEnvelope relation が未閉包

Work Package 01 の対象。runtime/capability/operation/permission/approval/recovery/scope relation を schema + evaluator + broker resolution の三層で閉じる。

### D-P0-03: Payload hash が payload 内容へ結合されていない

Work Package 01 の対象。canonical JSON / canonical bytes の仕様を固定し、broker が payload から hash を再計算する。

### D-P0-04: Schema-first contract が runtime ingress へ強制されていない

Work Package 01 の対象。runtime/capability/permission/approval/recovery/audit/adapter/update/IPC/persisted state の ingress-to-schema map を定義する。

### D-P0-05: Adapter registration が trust boundary を迂回できる

Work Package 01 の対象。raw dict の public store route を廃止又は fixture-only 化し、validate → normalize → authority strip → store を単一路にする。

### D-P0-06: SensitiveActionRouter が evaluator 不在で fail-open

Work Package 01 の対象。production router は evaluator / authoritative state 必須にし、解決失敗を deny/SUSPEND する。

### D-P0-07: Audit mapping を caller が捏造でき、decision relation を保持しない

WP00 では caller fabricated audit mapping を decision 条件として使わせない前提を閉じる。
完全な broker-emitted audit relation / append-only store は Block 2 対象。

### D-P0-08: Broker audit の `accepted` と authority decision が混同される

WP00 では event model 前提を分ける。
完全な transport receipt / authority decision event 分離は Block 2 対象。

### D-P0-09: Audit chain の真正性・append-only性が閉じていない

Block 2 対象。append-only、duplicate reject、genesis/event count/latest hash anchor を閉じる。

### D-P0-10: Product UI が synthetic/probe state を製品状態として表示し、validator が見抜かない

Block 2 対象。Dart `ShellCoreClient.product()` の synthetic truth を real state/export 接続へ修正する。

## 4. D-P1 — High Implementation Defects / Hardening

- `D-P1-01`: 正規化キー衝突 reject/quarantine
- `D-P1-02`: Python audit event ID append-only + duplicate reject
- `D-P1-03`: symlink / junction / reparse secret path boundary
- `D-P1-04`: IPC incremental bounded read
- `D-P1-05`: transient IPC error で listener を停止しない
- `D-P1-06`: Windows endpoint/session secret ACL
- `D-P1-07`: Flutter broker response EOF 依存
- `D-P1-08`: replay nonce TTL/compaction
- `D-P1-09`: `content_visibility` 欠落 fail-closed
- `D-P1-10`: audit JSONL corruption isolation
- `D-P1-11`: recursive invariant scan
- `D-P1-12`: error taxonomy 接続又は削除
- `D-P1-13`: `--python-only` と cargo parity mode 分離
- `D-P1-14`: `.gitattributes` EOL/binary rule
- `D-P1-15`: Flutter SDK / action pin
- `D-P1-16`: dev stdin smoke 経路の diagnostic/test namespace 隔離

## 5. G-P0 / G-P1 — release 前 gate

G-P0:

- Product UI real-state closure
- Windows primary integration
- Accessibility acceptance
- Distribution / installer / lifecycle
- Product-generated Setup Doctor evidence
- Formal evidence bundle integrity
- Formal provenance manifest coverage
- Signature / update scope

G-P1:

- visual/layout regression
- keyboard/focus desktop interaction
- fatal UI error / crash recovery
- performance / long-run
- failure injection
- clean build/install isolation
- governance manifest / current claim hygiene

## 6. 確定実装ロードマップ

### Pre-Block G: Governance / Claim Freeze / 作業規律

- 本 v4 を repository の canonical ledger として配置する。
- v2/v3/再監査報告は evidence/source として保持し、実施順序は v4 に委譲する。
- 凍結 claim を README、応募素材、docs、screenshot caption へ反映する。
- `AGENTS.md` 又は作業規律文書に、本書を読むこと、formal claim 禁止条件、validation 報告形式、commit/push、二世代 backup refs 更新、便乗 refactor 禁止を固定する。

### Block -1: Authority Source-of-Truth Closure

対象:

- `D-P0-00`
- `D-P0-01`
- `D-P0-07` の設計前提
- `D-P0-08` の event model 前提

方針:

既存 `RuntimeRegistry` / `AuditStore` / `RecoveryCatalog` / `UpdatePolicyStore` を候補部品として評価し、production broker の authoritative state path に配線する。
caller-supplied `RuntimeState` を product authority decision の根拠として使わない。

必須作業:

1. broker-owned authoritative registry model を定義。
2. production `authority_evaluate` contract から `payload.state` を排除。
3. test/parity fixture evaluator を production namespace から隔離。
4. authority provenance / issuer を broker 内部情報へ移管。
5. audit を broker emission として設計。
6. command dispatch と cutover を suspended/not_active のまま維持。

完了条件:

- forged approved state が decision を成立させない。
- unknown / caller-forged authority source が deny/SUSPEND。
- caller fabricated audit event が decision 条件にならない。
- broker state に無い relation は許可されない。
- 本 Block の修正で command dispatch は有効化されていない。

### Block 1: Contract / Schema / ActionEnvelope Relation Closure

WP00 完了後に実施する。ActionEnvelope schema、permission runtime relation、canonical payload hash、normalization collision rejection、adapter ingress closure、router fail-closed はここで扱う。

### Block 2: Audit Truth / Product Truth / Validation Truth Closure

transport receipt と authority decision event の分離、broker-emitted audit relation、product UI real-state connection、synthetic state validator、formal validator 群の拡張を扱う。

### Block 3: Reliability / Platform Boundary Hardening

secret path boundary、IPC bounded read/listener availability、ACL、nonce compaction、Python error handling、CI/EOL/SDK pin を扱う。

### Block 4: GUI Product Baseline Closure

window title/minimum size/theme/fatal error boundary、integration/golden/accessibility/failure/performance evidence を扱う。

### Block 5: Distribution / Formal Evidence / Windows Diagnostic

distribution format、install lifecycle、product-generated Setup Doctor export、clean build/package/install/run/collect/verify orchestrator、raw/public evidence 分離を扱う。

### Block 6: Windows Formal Acceptance / Release Decision

D-P0/G-P0/relevant P1 が閉じ、Windows diagnostic 追加欠陥が処理済みで、docs/evidence/claims が一致し、owner GO がある場合のみ実行する。

## 7. Codex Work Package 00: Authority Source-of-Truth Closure

Scope:

- This work package closes the upstream authority source-of-truth defect only.
- Do not enable command dispatch.
- Do not set authority cutover active.
- Do not claim release readiness or Windows formal acceptance.
- Do not perform unrelated UI polish or opportunistic refactors.

Confirmed problem:

Production authority evaluation currently consumes caller-supplied `payload.state` and `payload.action`.
The broker does not yet resolve runtime/capability/permission/approval/recovery from a broker-owned authoritative registry before deciding.
Existing Python-side components such as `RuntimeRegistry`, `AuditStore`, `RecoveryCatalog`, and `UpdatePolicyStore` exist but are not wired as the production authority source of truth.

Objective:

Make the broker/product authority decision depend only on broker-owned authoritative state and an action request submitted by the caller.

Required design report before editing:

1. List every affected Python, Rust, Dart, schema, fixture, and test symbol.
2. Identify existing registry/store components that can be reused and any missing broker-owned state components.
3. Define the production authority input contract after removing caller-supplied `state`.
4. Define how test/parity fixture evaluation will be isolated from production operations.
5. Define internal authority provenance/issuer rules.
6. Define broker-emitted audit sequencing and failure behavior.
7. State migration impact and what remains intentionally deferred to Work Package 01.

Required implementation:

1. Remove or reject caller-supplied state in the production authority decision operation.
2. Resolve authority-relevant state from broker-owned registries/stores.
3. Prevent caller-declared or unknown authority sources from authorizing an action.
4. Ensure caller-submitted fabricated audit mappings cannot satisfy authority conditions.
5. Keep command dispatch suspended and authority cutover inactive.
6. Preserve existing healthy defenses.

Required negative tests:

- forged approved state sent by caller cannot authorize an action
- missing internal registry record denies or suspends
- unknown/caller-forged authority source denies or suspends
- fabricated caller audit mapping cannot satisfy a decision
- production operation does not accept fixture state
- command dispatch remains suspended after the change

## 8. Work Package 01 予告境界

WP00 に混ぜない。

- ActionEnvelope schema 新設
- permission.runtime_id 等の必須 field 化
- runtime/capability/operation/permission/approval/recovery/scope relation
- canonical payload hash
- normalization collision rejection
- adapter ingress closure
- router fail-closed
- cross-language parity / negative fixtures

## 9. 最終リリース判定

| 行動 | 現時点 | 解禁条件 |
| --- | --- | --- |
| v4 を基準に Ubuntu Codex へ WP00 投入 | GO | 本書を repo に置き、作業規律を読ませる |
| UI polish の先行 | 原則 STOP | D-P0 修正と競合しない軽量項目のみ並行可 |
| Windows diagnostic | STOP | Blocks -1〜3 の対象境界完了後 |
| Windows formal acceptance | STOP | D-P0 / G-P0 / diagnostic closure 後 |
| README 外向け current facts 修正 | GO | 凍結 claim を守る |
| 応募用 completed product claim | STOP | formal acceptance + owner GO |
| Diagnostic 限定公開 | SUSPEND | D-P0 閉鎖後、限定表現を owner 判断 |
| 製品 release | STOP | formal gate 全閉鎖 + owner GO |
