# D4 Pocket 最終品質保証項目（`FINAL_QA_QUEUE`）

状態: `QUEUED`（Feature Complete前は実施・拡張しない）
工程正本: ユーザー提示「D4 Pocket / GUI-Shell rev5 Product-First / Final-QA Separation」
現行製品phase: `docs/REV5_PRODUCT_PROGRESS.md`

## 1. 運用規則

この一覧は、製品開発中にAcceptance外の最終保証作業を追加して現phaseを延長しないための移送先である。queue中の項目は現在の製品phaseのblockerではない。Feature Complete後、rev5で定めるFinal QA工程として現行統合製品に対して実行する。

安全・Authority・Permission・Approval・Audit・Recovery・secret境界はFinal QAまで無効化・迂回しない。既存証拠と過去のFAILは履歴として参照するが、異なる製品状態に対する現行試験結果へ昇格しない。MxC、OS、Provider等の外部component内部完全性をD4 Pocketの保証へ取り込まない。

## 2. 検査項目

| 識別子 | 最終統合状態で確認する範囲 | 由来 | 現在phaseをblockするか |
| --- | --- | --- | --- |
| FQ-R2-A | Task cancel／deadline後のprocess群停止、Recovery、旧Authority非再利用 | rev4 R2-A受入れの最終製品回帰 | いいえ |
| FQ-R2-B | Codex／Broker／Launcher異常終了後のprocess群停止、Recovery、旧Approval非再利用 | rev4 R2-B未完了条件・障害履歴 | いいえ |
| FQ-R2-C | Task／Agent間scratch・状態非汚染 | `rev4 R2-C` | いいえ |
| FQ-R2-D | result／diff／changed files／test resultのbounded Content Exposureとsecret境界 | `rev4 R2-D` | いいえ |
| FQ-R2-E | installed productの通常NTFS／OneDrive Cloud Filesにおけるsecret・Workspace境界 | `rev4 R2-E` | いいえ |
| FQ-R2-F | D4-owned `WorkspaceTaskScratch`のTask binding、正常／cancel／deadline／crash回収、失敗時fail-closed | `rev4 R2-F` | いいえ |
| FQ-R2-G | 通常Release `task_execution` capabilityのrelease昇格条件 | rev4 R2-G、通常Releaseをfail-closedに維持 | いいえ（release gateは別途維持） |
| FQ-R2-H | source／app／launcher／Broker hash、runtime identity、installed root、Audit store identityの正式evidence結合 | `rev4 R2-H` | いいえ |
| FQ-R2-TEMP | MxC／Windows TEMPの観測と責任境界。D4-owned scratch保証と混同しない。外部componentの物理削除をD4の保証条件にしない | rev3/rev4の観測履歴 | いいえ |
| FQ-TEST-LOOPBACK | P4検証時のA2A／Codex接続fixtureに加え、P6全Rust試験でもCodex loopbackとUpdate Downloadのlocal TLS fixtureでOS error 10054 / `ConnectionReset`を観測。`failed_tool_result_is_not_replayed_as_another_exec_command`と`local_tls_server_repairs_only_after_verified_package_bytes`は単独再実行で成功した一方、`failed_replacement_keeps_the_existing_corrupt_package_unchanged`は単独でも同じ接続切断を再現。2026-10-05のP7 A2A全target試験では444件中435成功／2失敗／7 ignored。Codex loopback fixtureは単独再実行で成功、TLS修復fixtureは単独でも`InvalidContentType`とOS error 10054を再現。さらにP7 Adapter Manifest単位の`cargo test --locked -- --test-threads=1`は443成功／2失敗／9 ignoredで、このCodex loopback testとTLS修復testがConnectionResetしたが、両方とも個別再実行では各1件成功した。すべてP7 Adapter差分外である。Feature Complete後にWindowsで一括再実行し、fixture server／clientの切断競合を確認する。現行P7 acceptanceを阻止しない | 2026-10-05 P4／P6／P7検証 | いいえ |
| FQ-INTEGRATED | rev5で定めるQ0–Q7最終品質保証 | `rev5 Final QA` | いいえ |

## 3. Final QA段階

| 段階 | 対象 | 開始条件 |
| --- | --- | --- |
| Q0 品質保証凍結 | 製品構造を凍結し、対象commitを固定 | 機能統合完了 |
| Q1 Codex横断品質検査 | Agent Task、Compare、Handoff、Workspace、MCP、Provider、Host、Export、Installer、Update、Rollbackの横断検査 | Q0完了 |
| Q2 障害・復旧 | crash、kill、deadline、cancel、stale、replay、partial write、corruption、process orphan、Broker／frontend／network／provider failure等 | Q1完了 |
| Q3 安全・権限境界 | Permission、Approval、Credential、secret、filesystem、hardlink／alias、Workspace escape、process／IPC境界、Agent isolation、Handoff／ExportのAuthority | Q2完了 |
| Q4 長時間・性能 | 8h／24h、反復Task／Compare／Handoff、memory／CPU／handle／process／disk／Audit増加 | Q3完了 |
| Q5 配置済み製品品質検査 | 別Windows user、clean install、first run、update、rollback、uninstall／reinstall、cache破損、crash recovery、Setup Doctor、UI automation | Q4完了 |
| Q6 正式証拠 | 品質検査通過済み同一Release Candidateのcommit、binary／installer hash、manifest、Audit／runtime identity、evidence bundle、provenance | Q5完了 |
| Q7 Owner最終確定 | production Publisher／signing identity、production Audit key、不可逆な事業判断、Final GO | Codex技術工程完了 |

## 4. 対応する履歴

- `docs/REV4_ACCEPTANCE_LEDGER.md`: rev4当時のAcceptance状態とR2-A受入れ結果
- `docs/REV3_PROGRESS.md`: 過去のrun、失敗、回復、実証範囲
- `release_blockers.registry.json`: 現在保持するrelease gateと証拠参照

このqueueを作成したことは、各項目のPASS、R2再開、通常Release capabilityの昇格を意味しない。
