# D4 Pocket 最終品質保証項目（`FINAL_QA_QUEUE`）

状態: `Q2 ACTIVE`（P12 Feature Complete後にFinal QAへ移行。Q0／Q1完了）
工程正本: ユーザー提示「D4 Pocket / GUI-Shell rev5 Product-First / Final-QA Separation」
現行製品phase: `docs/REV5_PRODUCT_PROGRESS.md`

## 0. 現在の凍結・進行状態

| 段階 | 状態 | 対象／証拠 |
| --- | --- | --- |
| Q0 品質保証凍結 | 完了（2026-10-07） | Final QA対象製品commit `ab746d4b33b003761f5ddcef60a88afa97132a40`。`main`へpushしremote HEAD一致を確認 |
| Q1 Codex横断品質検査 | 完了（2026-10-07） | 凍結commitを基準に全Rust target、Desktop／Mobile Flutter全test、Schema／Conformance、Windows Release compile |
| Q2 障害・復旧 | 進行中 | crash／kill／deadline／cancel／stale／replay／partial write／corruption／process orphan／Broker・frontend・network・provider failureを検証 |
| Q3–Q7 後続品質保証 | 待機中 | 各前段階の閉鎖後に開始 |

製品コード変更が必要な場合、該当QA所見を証拠化して局所修正し、修正済み製品を新しいQA候補commitとして明示する。以後のQA結果を旧凍結commitへ誤って帰属させず、更新対象を再凍結する。Q0凍結はrelease readinessを意味せず、既存release blockerを変更しない。

## Q1 Codex横断品質検査 — 完了（2026-10-07）

凍結製品commit `ab746d4b33b003761f5ddcef60a88afa97132a40`の横断回帰はRust全target 545 passed／13明示ignored、Desktop Flutter 204 passed、Mobile Flutter 21 passed。Schema 161／157／208、Conformance 236 checks、manual workflow check、Manifest、Windows v1 Release GateもPASS。ignoredのAgent Task production E2Eは`task_execution`の現行fail-closed gateを検査する専用testで、一般Release capabilityの対応済み証拠ではない。

Desktop Flutter Release compile成功（Flutter 3.44.0、binary SHA-256 `731da778f1c61e2d6597162876b79e604503ea511199f3d7ef94cc479919709d`、162304 bytes）。source-equivalent ASCII一時checkout上のcompileであり、formal provenance／installed product evidenceではない。`MSB8029`が一時出力directory由来で7件。Desktop analyzerはdeprecated API info 5件でexit 1、`RELEASE_CHECKLIST.md`にnon-blocking `known_limitation`として記録。Mobile analyzerはASCII一時checkoutで`No issues found`。

Rust初回・再runでA2A／Update TLS loopback fixtureの間欠failureを観測した後、対象単独testがPASSし、逐次`--no-fail-fast`全targetが0 failureで終了した。fixture root causeは確定していないので`FQ-TEST-LOOPBACK`として保持し、Q2のnetwork failure investigationへ引き継ぐ。Q1の完了はinstalled／formal evidence、全failure matrix、release readinessを意味しない。

## 1. 運用規則

この一覧はProduct Build中にAcceptance外の最終保証作業を追加して現phaseを延長しないための移送先である。Product Build中はqueue項目を同phaseのblockerにしない。Feature Complete後はrev5のFinal QA工程としてQ0〜Q7を実行し、該当するqueue項目を現行QA phaseのAcceptanceとして扱う。

安全・Authority・Permission・Approval・Audit・Recovery・secret境界はFinal QAまで無効化・迂回しない。既存証拠と過去のFAILは履歴として参照するが、異なる製品状態に対する現行試験結果へ昇格しない。MxC、OS、Provider等の外部component内部完全性をD4 Pocketの保証へ取り込まない。

## 2. 検査項目

| 識別子 | 最終統合状態で確認する範囲 | 由来 | Product Buildをblockするか |
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

2026-10-07 Q2初回検査: 実Codex CLI `0.160.0`を資格情報なしのloopback偽Responses APIへ接続するignored LIVE_RUNTIME testは、Task完了／取消、MxC child中のTEMP marker観測と終了後の不在、child heartbeat停止、Broker管理WorkspaceTaskScratch cleanupまで進んだ。旧表示名Audit assertionが現行`AgentTask履歴:`構造化記録と不一致で失敗したため、同testを構造化Task ID／stage／status照合へ修正し、再実行1 passed。これは実CLI／MxC childの限定LIVE_RUNTIMEとin-process Broker・合成Owner／Audit fixtureの範囲であり、installed production IPC／durable Audit／deadline／crash回復の証拠ではない。別途、Broker control→fake Codex cancellation／deadline／descendant停止testは1 passed、`AgentTaskScratchJournal` focused testsは8 passed（いずれもFIXTURE）。

同sourceの`cargo test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets --no-fail-fast -- --test-threads=1`はexit 101。Rust library 496 passed／3 failed／12 ignored、他targetは全PASS（総計542 passed／3 failed／12 ignored）。失敗はCodex loopback fixtureのConnectionReset 1件とUpdate Download TLS fixtureのWSA 10054／ConnectionReset 2件。TLS focused 17件は初回PASS後、2件の反復でround 2に再発し、単一test反復ではround 10にTLS `InvalidContentType`とserver-side ConnectionResetを再発した。根本原因は未確定のため`FQ-TEST-LOOPBACK`はOPEN。Q2のcancel／deadline／crash installed-product Acceptanceは未成立であり、FQ-R2-A/B/Fを閉じない。通常Release `task_execution=unsupported`、既存release blocker、`release_ready=false`は維持。

2026-10-07 Windows hosted補助run #39（commit `de3be16e808ba42ea39d0456046cb4e1f5c355a4`）はWindows runner上のrustfmt gateで失敗し、Rust全target試験へ到達しなかった（[run #39](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37505161510)）。既存のformat差分を検査対象4 fileだけでRust 1.95.0により整形し、workflow対象8 fileの整形確認は通過した。

同整形commit `97c30fac8eb7d51fcaa3b341877c4a93f5ad84c8`に対するWindows手動補助検査 #40は全工程成功（実行17分20秒／全体17分24秒、実行環境`win25-vs2026/20260925.250.1`、Rust 1.95.0）。Rust全対象試験は13集計欄の合計で成功545件／失敗0件／明示除外13件。Release Broker生成も成功し、SHA-256は`4390f8c7a248c655d9bdcc7070f893e3eb0b0cf56c9408ef3a8e045e8427de46`。Broker実行体を独立起動した通常IPC簡易試験は、非合成収集器v6を使って成功し、認証済みloopback IPC、永続保存先の準備完了、通常Task権限付与の拒否2件、再起動後の再送拒否、新しい正常状態、強制停止時のfail-closed、session fileの削除を観測した。実行環境内の検査物後片付けと作業tree清掃も成功し、成果物のuploadはない（[run #40](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37507251175)）。これは同commitにおけるWindows全対象試験と独立Broker簡易試験の証拠に限られ、導入済みDesktop製品、OS側Owner操作、実Agent Task、正式release証拠、release readinessを証明しない。ローカルで再現したloopback fixture失敗との原因差は確定していないため`FQ-TEST-LOOPBACK`はOPENのままとし、通常Release `task_execution=unsupported`および既存release gateを変更しない。

2026-10-07 Q2追加証拠: 別プロセスのBrokerライブラリ試験を強制終了し、別Broker実体の起動登録で永続scratch領域のbinding照合回収・中断Taskの監査記録・回復監査記録を確認するfocused Rust testは1 passed／0 failed（`cargo +1.95.0 test --locked --offline --manifest-path native/rust_helper/Cargo.toml --lib broker::protocol::tests::broker強制終了後の別process起動登録で永続scratchを監査付き回収する -- --exact --nocapture --test-threads=1`）。これは`FIXTURE`であり、製品broker-server／IPC、導入済み製品、Approval再利用防止、子孫プロセス停止の実証ではない。`FQ-R2-B`／`FQ-R2-F`は閉じない。

同日、TLS fixture不安定性を減らす試験限定IPv4化／accept-loop案は、focused TLS test 30回中6回失敗、追加診断10回中3回失敗で改善せず、採用せず破棄した。根因は未特定である。これは製品コード修正ではなく、`FQ-TEST-LOOPBACK`をOPENのまま保持する。

2026-10-07 Windows hosted補助run #41: `workflow_dispatch`による手動実行でcommit `2bb50217c02fcf6dd6cf757c43f126c6a2f1f1f8`（当時の`main`先端と同一）を検証し、全体10分59秒で全工程が成功した。workflow指定Rust sourceの書式検査、Rust全target 13集計欄（545件成功／0件失敗／13件除外）、Broker実行体のRelease build、Broker独立起動と通常IPCの簡易疎通確認、実行環境内の検査用一時物の後片付け、作業tree清掃確認がすべて成功。生成物のアップロードなし（[run #41](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37518774824)）。証拠は対象commitのWindows hosted Rust検査／Broker簡易疎通確認に限られ、導入済み製品、実Agent Task、正式リリース証拠、リリース可能性を証明しない。ローカルloopback fixture failureとの根因差は未確定なので`FQ-TEST-LOOPBACK`はOPENのままとし、通常Release `task_execution=unsupported`およびrelease gateを変更しない。対象commitが既に`main`と同一だったため統合は不要であり、検証用branchはlocal／remote双方から削除済み。

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

2026-10-07 Q2 Broker process強制終了後の実Agent回復検証: Windowsの`r2-e2e` feature専用Broker server／認証済みloopback IPCを起動し、実Codex CLI `0.160.0`／実MxC childを資格情報なしのlocalhost偽Responses APIへ接続したignored test `broker_kill_during_active_codex_task_stops_descendants_recovers_scratch_and_does_not_reuse_approval`は1 passed／0 failed。実tool call開始とchild heartbeat継続を観測した後、Broker processをhard killし、heartbeat停止、停止直後のscratch保持、Broker再起動・Workspace再登録時のscratch回収、中断Taskのdurable Audit、古いIPC credential拒否、新Sessionで新Permissionだけでは旧Approvalを使えないこと、tool result／完了writeがないことを確認した。監査本文にTask指示・合成secretがなく、synthetic OwnerによるTask Permission／Approval付与監査は`FIXTURE`、実process／scratch観測はその観測範囲で`LIVE_RUNTIME`とした。synthetic Ownerはr2-e2e buildに限定され、Task Permission／Approvalの2操作だけを許可し、MCP実行と結果表示Approvalは拒否された。実Broker process／IPCとCodex/MxC子孫を含む限定crash回復証拠だが、installed Flutter／native Owner UI、通常Release、Launcher自身またはCodex親process単体crashを証明しない。従って`FQ-R2-B`および`FQ-R2-F`全体はOPENのまま、`task_execution=unsupported`とrelease gateを維持する。実行: `cargo +1.95.0 test --locked --offline --features r2-e2e --test agent_task_crash_recovery_e2e -- --ignored --exact broker_kill_during_active_codex_task_stops_descendants_recovers_scratch_and_does_not_reuse_approval --nocapture`（環境変数`GUI_SHELL_CODEX_TASK_CRASH_E2E_CLI`にCodex CLI絶対pathを指定）。

2026-10-07 Q2 Codex root異常終了の限定LIVE_RUNTIME検証: ignored test `codex_root_crash_keeps_broker_alive_stops_descendants_recovers_task_and_does_not_reuse_approval`はWindowsで1 passed／0 failed。実Codex CLI `0.160.0`の新規processを、試験Broker PIDを親に持つことと指定実行体の完全path一致で一意に特定し、そのrootだけを終了。Broker processは生存し、実MxC child heartbeatは終了後に停止、Taskは`failed`／Audit上`failure_class=通信失敗`で終端化、D4-owned scratch回収、Workspace完了marker／tool resultなし、永続Audit chain再読込、Task本文非記録を確認した。新しいWorkspace Permissionを発行しても、異常終了Taskで消費済みApprovalは再利用できなかった。実Codex／MxCとBroker IPCを使うが、合成Owner／localhost偽Responses API／試験用Workspaceの証拠であり、installed UI／native Owner、通常Release、Launcher crash、別環境やrelease readinessは証明しない。これは`FQ-R2-B`／`FQ-R2-F`のCodex親process分岐の局所証拠で、同条件全体はOPENのまま。通常Release `task_execution=unsupported`とrelease gateを維持する。実行: `cargo +1.95.0 test --locked --offline --features r2-e2e --test agent_task_crash_recovery_e2e -- --ignored --exact codex_root_crash_keeps_broker_alive_stops_descendants_recovers_task_and_does_not_reuse_approval --nocapture`（`GUI_SHELL_CODEX_TASK_CRASH_E2E_CLI`に実Codex CLI絶対pathを指定）。

2026-10-07 Q2 active Task cancel／deadlineの限定LIVE_RUNTIME検証: 実Codex CLI `0.160.0`／MxC child、feature専用Broker server、認証済みloopback IPC、資格情報なしlocalhost偽Responses APIによるignored test `owner_cancellation_of_active_codex_task_stops_descendants_and_recovers_scratch`と`deadline_of_active_codex_task_stops_descendants_and_recovers_scratch`が各1 passed。取消要求後も実process treeの停止確認までTask statusは`running`を保ち、deadline試験ではfeature専用のtest-only 20秒execution limitを使用した。両試験で子孫process／heartbeat停止、terminal status（cancel=`cancelled`、deadline=`failed`）、result hash／完了markerなし、Broker-owned WorkspaceTaskScratch回収、file-backed Audit reopen後のterminal記録、Workspace Permissionだけの再発行では消費済みApprovalを再利用できないことを確認した。synthetic OwnerはTask Workspace Permission／Task Approvalだけを発行する`FIXTURE`、実Broker／Codex／MxC processの観測を限定`LIVE_RUNTIME`とする。Broker crash／Codex root crashの既存試験も合わせ、4 ignored E2Eは一括実行で4 passed／0 failed。これらは各failure branchの限定証拠で、Launcher crash、installed Desktop UI／native Owner、通常Release、外部Provider／実資格、release readinessを示さない。`FQ-R2-A/B/F`は現行queueの統合範囲が残るためOPENのまま、通常Release `task_execution=unsupported`とrelease gateを維持する。

同じ差分のRust回帰は`cargo +1.95.0 test --quiet --locked --offline --manifest-path native/rust_helper/Cargo.toml --all-targets --no-fail-fast -- --test-threads=1`で545 passed／0 failed／13 ignored。featureなしおよび`r2-e2e`ありの全target `cargo check`、Schema 161／157／208、Conformance 236 checksもPASS。全suite初回runではfixture内の既存`.NET [IO.File]`使用と否定assertionの矛盾が1件顕在化し、shell probeを`Set-Content`／`Add-Content`へ揃えてfocused test PASS。既知Update TLS loopback fixtureは今回の初回全suiteでOS error 10054を一度返したがfocused retryと最終全suiteはPASSし、根因は未確定なので`FQ-TEST-LOOPBACK`はOPEN。test変更は`r2-e2e` harnessに限定し、通常Release capability・release gate・release blocker registryは変更しない。
