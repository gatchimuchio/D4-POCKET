# D4Pocket v9 NLC08 三点・常設作業要領 採用記録

採用日: 2026-11-23（Asia/Tokyo）

対象: gatchimuchio/D4-POCKET

基準HEAD: e57c595ebcb2ba79adbd6b342498ea35436c4080

## 1. 採用指示と範囲

指示者は、Owner本人の物理操作が不可欠な製品受入だけに本人介入を限定し、AGENTS.mdと現行三点を是正したうえで開発を続行するよう明示した。本NLC08を現行正本として採用する。

NLC08はNLC07で確立した通常開発検証と実製品Owner受入の分離、ならびに施工者が隔離検証環境を準備する責任を保持する。実製品のOwner認可・受入、製品要求、E1〜E6、本人GO、公開の条件は変更しない。

## 2. run #8で観測した不成立と根因

Windows manual workflow run #8 (https://github.com/gatchimuchio/D4-POCKET/actions/runs/45085957427)、job 137404512035、source e57c595ebcb2ba79adbd6b342498ea35436c4080 では、固定Codex CLIの公式Windows sandbox setupが成功した後、production BrokerのR2専用検査がtest専用CODEX_HOMEに既存entryがあるとして終了した。

観測された診断は「broker-serverが失敗: R2 loopback試験CODEX_HOMEに既存fileがある」。setupの直前に新規homeが空であったことと、setup成功後に公式sandbox制御ファイルが作られることを、検査器が別状態として扱っていなかった。したがって失敗原因はOwner画面の操作不足ではなく、開発用test前提の作成側とBroker検査側の契約不一致である。run #8のFAIL履歴は変更せず保持する。

## 3. NLC08の限定是正

NLC08はtest専用CODEX_HOMEを初期化前後の二段階で定義する。

- 公式setup直前は新規・隔離・空のdirectoryであることを確認する。
- 固定版Codex CLIの公式setup成功後は、公式sandbox状態の限定集合だけを許す。elevated sandbox設定、setup marker、sandbox log、sandbox user metadata、setup直後に空のhelper binを型とreparse有無込みで検査する。
- auth/Credential/token/session/history/transcript/未知entry/reparse objectは拒否し、file本文やCredentialをログ・証拠へ出さない。保護機構を弱化しない。
- 開発test identityやfixtureはproduction Authority/Owner receipt/Audit chainとして扱わない。
- Rustのtest-only Broker gateと、その隔離初期化を担うE2E testを同じ二段階契約へ合わせる。test用の許可集合は通常Releaseへ含めない。

この是正はOwner本人へのクリック依頼を開発環境準備の代替手段として用いない。実製品Owner認可が要求される製品受入は引き続き本人にだけ帰属し、実製品UI経路での成立まで未成立とする。Owner介入が不要なSource・Builder・Package・隔離Runtime施工は継続する。

## 4. NLC07以前の履歴保持

NLC07三点・採用記録・run #6のFAIL・その他の旧証拠を変更せず保持する。NLC07は直前採用版、NLC06以前は既存の履歴境界を保つ。

- 01_D4Pocket_SPEC_v9_NLC07.jsonl: be140b09cebaec128cd1a4684f72f5d22afa9dc1fe39b5db81aefb5bbada96e9
- 02_D4Pocket_PLAN_v9_NLC07.jsonl: 1c2ab16fa9a70728443a320efd71bce1c3dbae299ad07a28da204c0393d72b41
- 03_D4Pocket_CODEX_v9_NLC07.jsonl: 0fdc00219aaaf8814a0690588b16f85b66e3d9f63305816f3a16412de603a391

## 5. NLC08三点の同一性

| 役割 | Repo内保存先 | 記録数 | bytes | SHA-256 |
|---|---|---:|---:|---|
| 製品仕様 | [01_D4Pocket_SPEC_v9_NLC08.jsonl](01_D4Pocket_SPEC_v9_NLC08.jsonl) | 438 | 270657 | e5ed0b21e38417f94221cf705ff99c4f3dd003c34e76da19d65ce695d99b0730 |
| 統合工程 | [02_D4Pocket_PLAN_v9_NLC08.jsonl](02_D4Pocket_PLAN_v9_NLC08.jsonl) | 289 | 204927 | ecc18f5fe83bc0c3ca6d1669ecb5514145f5a858cda8dafce973ac5ad8738e47 |
| 施工指示 | [03_D4Pocket_CODEX_v9_NLC08.jsonl](03_D4Pocket_CODEX_v9_NLC08.jsonl) | 1453 | 1830847 | 03cbedcef707025a16869292d120f515ba3e6783b7027a158c12a92c207b305e |

## 6. 製品状態と証拠の境界

NLC08採用、Rust test-only検査の修正、unit test、Hosted Windows Actionsの成功は製品Owner認可、実製品UI受入、W-E02/W-E03全体、E1〜E6、E5/E6、通常Release、署名、Publisher/Store、本人GO、公開の代用ではない。run #8の失敗とNLC08後の各検査結果は別時点の証拠として追記する。製品進捗やRelease Registryの未解決状態は証拠が直接成立させない限り維持する。

## 7. Git・復旧基準

NLC08採用前のlocal main、origin/main、GitHub main は e57c595ebcb2ba79adbd6b342498ea35436c4080 で一致した。編集前に二世代のlocal/remote backup tagを期待SHAに対する比較更新で保全した。

- codex/backup-main: e57c595ebcb2ba79adbd6b342498ea35436c4080
- codex/backup-main-prev: a14fdda52c71d49b1638b7a2c142d6fa9d8740c3

他worktreeの差分は対象外として保持する。push時にはremote先端を再確認し、non-fast-forwardを強制しない。

## 8. 検証

NLC08三点のJSONL構造・内部参照・入力Hash・索引/AGENTS/採用記録接続を現行governance checkerで検査する。Rust test-only CODEX_HOME許可集合は正常状態とcredential/history/未知entryの負例で検査する。Hosted Windows Actions runは、実際に実行したsource/CLI/Windows環境と出力の範囲だけを記録し、実製品QAへ昇格しない。
