# D4Pocket v9 NLC0.6 三点・常設作業要領 採用記録

採用日: 2026-11-22（Asia/Tokyo）

対象: `gatchimuchio/D4-POCKET`

基準HEAD: `67309a21d0c371099ec22e2f68412cb737430e7c`

## 1. 採用指示と改訂範囲

Ownerは、実製品のOwner認可を保持しながら、Codexに通常の開発施工を物理操作待ちで止めさせないため、`AGENTS.md`と現行三点を是正して続行するよう明示した。本NLC06を、その指示による現行正本として採用する。

本改訂は直前の現行正本NLC05を基底とし、実製品Owner認可と開発検証の境界を施工停止規則・Export計画操作まで一貫させる。NLC05の三点原本、採用記録、証拠は変更せず、[NLC05採用記録](採用記録.md)を履歴として保持する。

## 2. 是正した設計上の矛盾

NLC05には開発検証をOwner画面待ちで止めない規則があった一方、次の具体手順が残っていた。

- `施工/停止再開`が対象を限定しない一般的な「本人Gate」を停止条件にしていた。
- `操作/Export計画`が開発用Builder検証と実製品作用を区別せず、操作列でNative Owner確認を常時要求していた。
- このため、実製品Owner受入の未実施が、Manifest-only Builder比較や隔離した開発施工まで止め得た。

現行Sourceでも、製品BrokerのExport Manifest書出しにはOwner確認が要求される一方、`tooling/compare_module_builds_windows.py --manifest-file`には未信頼ManifestをBuilder比較へ渡す経路がある。両者の責任と証拠の射程を同一にしない。

## 3. NLC06の分離契約

### 開発検証

- 承認済み施工委任の範囲で、Source、Contract/Consumer、Builder、Package、隔離Runtimeを進める。実製品Owner画面のクリックやproduction Permission/Approval待ちを通常の開発作業へ課さない。
- 既存の未信頼Manifest-only Builder入口またはtest専用Broker経路を使い、test identity/rootと出力を実製品から隔離する。
- test経路はproduction Credential/Permission/Approval/Audit chainを使わず、production authorityを作らない。証拠に`real_owner_authority=false`と実際の証拠classを記録する。
- 開発証拠はBuilder等の実測範囲に限る。実製品UI生成、Owner認可、実Effect/Audit、W-E02/W-E03全体、E5/E6の成立へ昇格しない。

### 実製品操作と受入

- 実製品上の危険作用と、実Product UI/本人認可を受入条件とする範囲では、正規Native Owner確認と現在のBroker認可を維持する。
- test harnessのYes応答、fixture、Manifest metadataでOwner認可を模擬・代行しない。
- 実製品Owner操作が受入契約上不可欠な場合、その該当項目だけを未成立として保持する。他の独立施工は続ける。本人の物理操作が本当に残るときは、技術作業を終えた後の最終報告で必要事項をまとめる。
- W-E02/W-E03全体の受入母集団、安全条件、E1〜E6退出条件、署名、Ownerの最終GO、公開条件を変更しない。

## 4. NLC06三点の同一性

NLC06はNLC0.4形式の自己完結JSONLである。仕様・工程・施工指示の各文書において、NLC05の同じ識別を持つ境界規則を改訂し、施工指示ではさらに`施工/停止再開`と`操作/Export計画`を明示的に整合させた。新しい製品機能や受入IDは追加していない。

| 役割 | Repo内保存先 | 記録数 | bytes | SHA-256 |
|---|---|---:|---:|---|
| 製品仕様 | [01_D4Pocket_SPEC_v9_NLC06.jsonl](01_D4Pocket_SPEC_v9_NLC06.jsonl) | 438 | 268505 | `3f2f2875c3d1fda0971fe35a9d1b8ee94053bfbc25c0699c2587bc096066f863` |
| 統合工程 | [02_D4Pocket_PLAN_v9_NLC06.jsonl](02_D4Pocket_PLAN_v9_NLC06.jsonl) | 289 | 203198 | `9826e58de24fbc39085cb9d47ff460474af97aa3061ad1aaf3926e5a527f5288` |
| 施工指示 | [03_D4Pocket_CODEX_v9_NLC06.jsonl](03_D4Pocket_CODEX_v9_NLC06.jsonl) | 1453 | 1822183 | `05c3ebc1e7e97549e04aae56cbcba090a873cf3e34e02e1fd1e1294fa1fc3ae5` |

NLC05直前正本のSHA-256は、仕様 `a308f819d7081a7d1eb479d08cbb9e3b9545a9f87d502997d675dae5f2f15374`、工程 `577c8028157462c8314882074c5e002ed3ce0c7b72914e3456563a77cdb57b2d`、施工指示 `80b41343b6c374befd6c932cbe0fa1a48b7b690d1733a872f30c1cfbb1409f1a` である。旧三点はそのまま保持する。

## 5. 現行工程・製品状態への影響

この採用は設計文書の是正であり、製品実装、W-E02/W-E03受入、E1〜E6、Release Blocker、OS QA、署名、Publisher/Store、Owner GO、公開を再評価・完了認定しない。受入条件に実製品Owner UIが必要な部分は従来どおり未成立を保持する。隔離Developer verificationだけでW-E02/W-E03をCLOSEDにしない。W-E02の受入待ちを理由に独立するW-E03施工を止めない。

## 6. Git基準・復旧点

採用前のlocal `main`、remote `main`、`origin/main`は `67309a21d0c371099ec22e2f68412cb737430e7c` で一致していた。作業開始時のremote復旧tagは `codex/backup-main=`67309a21d0c371099ec22e2f68412cb737430e7c、`codex/backup-main-prev=`2d1549b30aaad1a46807dbb3401587851076d030。この基準は文書更新前の状態であり、更新後のcommitを意味しない。文書採用差分のみを限定してGit確定し、既存のWindows W-E04実装差分は独立した施工単位として保持する。

## 7. 限定検証

NLC06の採用時点ではJSONL構造・参照・NLC05原本の非改変を対象に検査する。検査command、exit、対象commitは当該作業のGit履歴へ結び付ける。文書検査は製品実行・Owner認可・OS QAの証拠ではない。
