# D4 Pocket 最終品質保証項目（`FINAL_QA_QUEUE`）

状態: `Q2 ACTIVE`（Q0／Q1完了）
工程正本: ユーザー提示「D4 Pocket / GUI-Shell rev5 Product-First / Final-QA Separation」
現行製品phase: `docs/REV5_PRODUCT_PROGRESS.md`

## 0. 現在の凍結・進行状態

| 段階 | 状態 | 対象／証拠 |
| --- | --- | --- |
| Q0 QA Freeze | CLOSED（2026-10-07） | Final QA対象製品commit `ab746d4b33b003761f5ddcef60a88afa97132a40`。`main`へpushしremote HEAD一致を確認 |
| Q1 Codex Comprehensive QA | CLOSED（2026-10-07） | 凍結commitを基準に全Rust target、Desktop／Mobile Flutter全test、Schema／Conformance、Windows Release compile |
| Q2 Fault / Recovery | OPEN | crash／kill／deadline／cancel／stale／replay／partial write／corruption／orphan／Broker・frontend・network・provider failure |
| Q3–Q7 | QUEUED | 各前段階の閉鎖後に開始 |

製品コード変更が必要な場合、該当QA所見を証拠化して局所修正し、修正済み製品を新しいQA候補commitとして明示する。以後のQA結果を旧凍結commitへ誤って帰属させず、更新対象を再凍結する。Q0凍結はrelease readinessを意味せず、既存release blockerを変更しない。

## Q1 Codex Comprehensive QA — CLOSED（2026-10-07）

凍結製品commit `ab746d4b33b003761f5ddcef60a88afa97132a40`の横断回帰はRust全target 545 passed／13明示ignored、Desktop Flutter 204 passed、Mobile Flutter 21 passed。Schema 161／157／208、Conformance 236 checks、manual workflow check、Manifest、Windows v1 Release GateもPASS。ignoredのAgent Task production E2Eは`task_execution`の現行fail-closed gateを検査する専用testで、一般Release capabilityの対応済み証拠ではない。

Desktop Flutter Release compile成功（Flutter 3.44.0、binary SHA-256 `731da778f1c61e2d6597162876b79e604503ea511199f3d7ef94cc479919709d`、162304 bytes）。source-equivalent ASCII一時checkout上のcompileであり、formal provenance／installed product evidenceではない。`MSB8029`が一時出力directory由来で7件。Desktop analyzerはdeprecated API info 5件でexit 1、`RELEASE_CHECKLIST.md`にnon-blocking `known_limitation`として記録。Mobile analyzerはASCII一時checkoutで`No issues found`。

Rust初回・再runでA2A／Update TLS loopback fixtureの間欠failureを観測した後、対象単独testがPASSし、逐次`--no-fail-fast`全targetが0 failureで終了した。fixture root causeは確定していないので`FQ-TEST-LOOPBACK`として保持し、Q2のnetwork failure investigationへ引き継ぐ。Q1の完了はinstalled／formal evidence、全failure matrix、release readinessを意味しない。

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
| FQ-MODULE-PRUNING | P10の機能除外を最終統合製品で再確認し、hash固定artifactにおける画面／共有symbolの意味上の除去、安全Core保持、Rust／第三者Module除去、cold startup、実行時resourceを測定する | rev5 P10は機能除外のみ受入れ。binary・起動・resource評価をFinal QAへ移送 | いいえ |
| FQ-TEST-LOOPBACK | P4検証時のA2A／Codex接続fixtureに加え、P6全Rust試験でもCodex loopbackとUpdate Downloadのlocal TLS fixtureでOS error 10054 / `ConnectionReset`を観測。`failed_tool_result_is_not_replayed_as_another_exec_command`と`local_tls_server_repairs_only_after_verified_package_bytes`は単独再実行で成功した一方、`failed_replacement_keeps_the_existing_corrupt_package_unchanged`は単独でも同じ接続切断を再現。2026-10-05のP7 A2A全target試験では444件中435成功／2失敗／7 ignored。Codex loopback fixtureは単独再実行で成功、TLS修復fixtureは単独でも`InvalidContentType`とOS error 10054を再現。さらにP7 Adapter Manifest単位の`cargo test --locked -- --test-threads=1`は443成功／2失敗／9 ignoredで、このCodex loopback testとTLS修復testがConnectionResetしたが、両方とも個別再実行では各1件成功した。すべてP7 Adapter差分外である。Feature Complete後にWindowsで一括再実行し、fixture server／clientの切断競合を確認する。現行P7 acceptanceを阻止しない | 2026-10-05 P4／P6／P7検証 | いいえ |
| FQ-P11-UNINSTALL-FINALIZER-CLEANUP | Uninstaller成功時のランダム一時directory内finalizer executable／directoryの残存を解消し、削除失敗時の保持・回復も最終統合Windows製品上で確認する。現行実装は製品rootと一致shortcutを削除するが、finalizer自身の一時copy cleanupは未成立 | P11 Uninstaller実装時のcode inspection | いいえ |
| FQ-INTEGRATED | rev5で定めるQ0–Q7最終品質保証 | `rev5 Final QA` | いいえ |

2026-10-06追補: `cargo test --locked --manifest-path native/rust_helper/Cargo.toml -- --test-threads=1`は449 passed／1 failed／9 ignored。失敗は`a2a::tests::loopback_HTTPからAgent_Cardを取得してmetadata_onlyへ射影する`の応答期限超過で、同testの単独再実行は1 passed。並列全試験では同A2A fixtureとUpdate Download TLS fixtureの2件が失敗し、逐次実行でTLS fixtureはPASSした。fixture server／client競合の根本原因は未確定で、最終統合後の全Rust試験として再確認する。現phaseはblockしない。

2026-10-06 P11追加記録: reader変更時の逐次全Rust試験は453 passed／1 failed／9 ignoredで、`broker::update_download::tests::local_tls_server_repairs_only_after_verified_package_bytes`がOS error 10054／`ConnectionReset`で失敗した。同testの単独再実行は1 passed。既存`FQ-TEST-LOOPBACK`のfixture安定性と同分類で、P11 reader差分外かつ現phaseをblockしない。

2026-10-06 P11 capability-reader追試: 非選別の逐次Rust suiteは465件中454 passed／2 failed／9 ignored。失敗は`adapters::minidora::tests::ContentLength付きJSONだけを期限内に取得する`と上記Update TLS fixtureで、どちらも現行reader差分外。MINIDORA fixture単独再実行は1 passed、Update TLS fixture単独再実行はOS error 10054／`ConnectionReset`で再失敗した。`FQ-TEST-LOOPBACK`の既存対象を更新し、P11 blockerへ追加しない。

2026-10-06 P11 Broker version-staging consumer回帰: `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`はRust lib test 469件中456 passed／3 failed／10 ignored。A2A loopbackと`failed_replacement_keeps_the_existing_corrupt_package_unchanged`は各単独再実行でPASS。`local_tls_server_repairs_only_after_verified_package_bytes`は単独でもOS error 10054／`ConnectionReset`で失敗し、root cause未確定。すべて既存`FQ-TEST-LOOPBACK`のlocalhost fixture問題で、今回のversion staging差分との因果は観測されていない。現行P11 blockerへ追加せず、最終統合後の全Rust試験で再確認する。

2026-10-06 P11 Rollback検証時: 集約Rust全targetの中間回で既存`broker::dialogue::broker_codex_loopback_support::tests::failed_tool_result_is_not_replayed_as_another_exec_command`がHTTP response header到達前のOS `ConnectionReset`で失敗した。focused再実行は1 passed、対象名を保持した後続all-targetsはlib 476 passed／0 failed／12 ignored、全体522 passed／0 failed／13 ignored。別中間回の`broker_ipc` targetも9 passed／1 failedを観測したが、同じ後続all-targetsで10 passed／0 failed。根因は確認できず、P11 Broker Rollback差分との因果も観測されない。HTTP loopback不安定性は既存`FQ-TEST-LOOPBACK`の最終統合時確認へ残し、現phaseをblockしない。

2026-10-06 P11初回Install／Update導線検証: `cargo test --locked --manifest-path native/rust_helper/Cargo.toml -- --test-threads=1`は474 passed／2 failed／12 ignored。失敗は`a2a::tests::loopback_HTTPからAgent_Cardを取得してmetadata_onlyへ射影する`と`adapters::minidora::tests::ContentLength付きJSONだけを期限内に取得する`で、各focused再実行は1 passed。今回のBroker update projection／package install UI差分との因果は観測されず、既存`FQ-TEST-LOOPBACK`の最終統合試験で確認し、現phaseはblockしない。

2026-10-06 P11 portable起点Install起動遷移検証: `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`のRust library suiteは484 passed／1 failed／12 ignoredとなり、既知`broker::update_download::tests::local_tls_server_repairs_only_after_verified_package_bytes`がWindows loopback TLSの`ConnectionReset`／`InvalidContentType`で失敗してCargoがnonzero終了した。後続targetのPASSは主張しない。失敗箇所は今回の変更file外で、既存`FQ-TEST-LOOPBACK`に属するためP11受入れをblockしない。最終統合後に既存queueの対象として再確認する。

2026-10-06 P11 Uninstaller実装確認: 逐次Rust全lib suiteは489 passed／1 failed／12 ignored。唯一の失敗は既知A2A loopback fixtureで、単独再実行は1 passed。今回のUninstaller focused Rust test 5件とRust Release buildはPASS。loopback fixtureの最終統合後確認は本queueに保持し、P11 Uninstallerの局所成立をblockしない。

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
