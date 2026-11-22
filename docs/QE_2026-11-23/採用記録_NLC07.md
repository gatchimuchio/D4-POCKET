# D4Pocket v9 NLC07 三点・常設作業要領 採用記録

採用日: 2026-11-23（Asia/Tokyo）

対象: `gatchimuchio/D4-POCKET`

基準HEAD: `50610a99789d20a7ab1725aadd687cd873a82e46`

## 1. 採用指示と範囲

Ownerは、AGENTS.mdと現行三点を修正し、Owner本人の物理操作が本当に不可欠な実製品受入以外で開発作業を停止せず続行するよう明示した。本NLC07をその指示による現行正本として採用する。

NLC07は直前正本NLC06を基底とし、NLC06が定めた「通常開発検証に実製品Owner UI操作を要求しない」「実製品Owner受入と証拠を混同しない」境界を保持する。新たに、隔離Developer Runtime検証のOS/CLI前提環境を施工者が正規手順で準備する責任と、環境制約時の局所処理を明記する。実製品認可、製品受入、既存E1〜E6退出条件、本人GO、公開を変更しない。

## 2. 直前の観測と是正理由

Windows W-E03 manual workflow run #6（source `50610a99789d20a7ab1725aadd687cd873a82e46`）では、隔離test `CODEX_HOME`のsandbox前提が未準備でCodex CLIがProvider要求前に停止した。診断は `stderr_class=sandbox_setup`、`windows_sandbox_setup_error_code=not_present`、外部Provider requests `0` を記録した。実製品Owner操作の欠落ではなく、test runtime前提の準備責任が試験施工に接続されていなかったことを示す。

NLC07はこの不足だけを修正する。開発環境のOS権限・sandbox準備をOwner本人の製品UI操作へ転嫁せず、保護機構の無効化・弱化も認めない。Hosted runnerが必要な管理者権限を持たない場合はそのtestのみをFAIL/NOT_RUNとして原因分類し、他の独立施工は続ける。

## 3. 三点で固定する規則

- 仕様: 隔離Developer Runtimeに必要なtest identity/home/sandbox前提は施工者が使い捨て隔離環境と対象CLI/OSの公式手順で準備する。
- 工程: 環境準備にOwner UI・production authorityを求めない。準備不能は該当検証だけに局所化し、独立施工を継続する。
- 施工: WindowsのW-E03では管理者権限を確認した使い捨てHosted Actions runner上で、固定Codex CLIの公式 `sandbox setup --elevated --current-user --codex-home <test-home>` を各test専用homeへ適用する。sandboxを無効化・弱化せず、個人/production CODEX_HOMEを使用しない。
- 開発testのfixture/隔離identity/evidenceは実製品Owner認可、実Product UI受入、W-E02/W-E03全体、E5/E6、本人GOへ昇格しない。

## 4. NLC06履歴の保持

NLC06三点・採用記録・採用時証拠は変更せず、直前採用版として履歴に保持する。NLC06の有限な開発検証/Owner受入分離条件を再評価・撤回しない。

- 01_D4Pocket_SPEC_v9_NLC06.jsonl: `3f2f2875c3d1fda0971fe35a9d1b8ee94053bfbc25c0699c2587bc096066f863`
- 02_D4Pocket_PLAN_v9_NLC06.jsonl: `9826e58de24fbc39085cb9d47ff460474af97aa3061ad1aaf3926e5a527f5288`
- 03_D4Pocket_CODEX_v9_NLC06.jsonl: `05c3ebc1e7e97549e04aae56cbcba090a873cf3e34e02e1fd1e1294fa1fc3ae5`

## 5. NLC07三点の同一性

| 役割 | Repo内保存先 | 記録数 | bytes | SHA-256 |
|---|---|---:|---:|---|
| 製品仕様 | [01_D4Pocket_SPEC_v9_NLC07.jsonl](01_D4Pocket_SPEC_v9_NLC07.jsonl) | 438 | 269175 | `be140b09cebaec128cd1a4684f72f5d22afa9dc1fe39b5db81aefb5bbada96e9` |
| 統合工程 | [02_D4Pocket_PLAN_v9_NLC07.jsonl](02_D4Pocket_PLAN_v9_NLC07.jsonl) | 289 | 203890 | `1c2ab16fa9a70728443a320efd71bce1c3dbae299ad07a28da204c0393d72b41` |
| 施工指示 | [03_D4Pocket_CODEX_v9_NLC07.jsonl](03_D4Pocket_CODEX_v9_NLC07.jsonl) | 1453 | 1826219 | `0fdc00219aaaf8814a0690588b16f85b66e3d9f63305816f3a16412de603a391` |

## 6. 工程・製品状態への影響

本採用は施工方法の明確化であり、Windows W-E03 Developer verificationをPASSにしない。run #6のFAIL履歴を保持する。実製品UIからのOwner認可、元D4停止/アンインストール後の独立製品受入、E1〜E6、通常Release、署名、Publisher/Store、本人GO、公開は別判定のまま未成立状態を保持する。

## 7. Git基準・復旧点

採用前のlocal `main`、`origin/main`、GitHub `main`は `50610a99789d20a7ab1725aadd687cd873a82e46` で一致した。編集前に二世代のremote/local backup refを比較更新し、現mainと直前mainを保全した。NLC06原本と他worktreeの未反映差分は変更しない。

## 8. 限定検証

NLC07のJSONL構造、三点参照/hash、正本索引・AGENTS接続、NLC06原本非改変を検査する。文書検査はW-E03 runtime実行・実製品Owner受入・製品完成の証拠ではない。
