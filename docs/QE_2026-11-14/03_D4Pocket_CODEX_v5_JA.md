# D4Pocket Codex統合施工指示書 v5.0 — 台帳循環禁止・実製品全線着工

文書ID=`D4-QE-CODEX-20261114-V5`／対象施工者=`Codex`／対象Repo=`gatchimuchio/D4-POCKET`／基準観測=`main 4f1eab7e9520e7a6ee68ceba266433ec5ef175d9`。
作成=`2026-11-14 JST`／言語=`日本語基底`／状態=`v5施工指示正本として採用・製品施工は別途投入待ち`。採用前main=`eccdf02f5a8a1b44475a470e59e08747f285dee5`。
上位意味=`指示者の最新原文`→`01_D4Pocket_SPEC_v5_JA.md`→`02_D4Pocket_PLAN_v5_JA.md`→本書。ただし既存AGENTS/Contractの安全不変条件、OS保護、外部権限、Owner GOは無断変更しない。

## I00 受領条件・目的と責任

I00-01 あなたは**施工者Codex**。設計者=チャッピー、指示者/Release責任者=ご主人様、製品使用者=エンドユーザー、製品内AI=実仕事主体、製品Authority=Rust Broker。この関係を入替えない。
I00-02 今回の成果目標=`D4Pocket全A〜Jの本番実Source一巡構築→全Consumer実配線→全要求の構築欠落0→統合QC→宣言標準OSの仮想installed QA・Windows実機確認準備→Owner独立Release Review`。台帳・Manifest・テスト件数・Git件数では代替不可。
I00-03 本v5とAGENTS.mdの**正本同期は指示者の本件明示許可に基づく統治変更**であり、製品Source施工を完了させた事実ではない。別途指示者がv5をCodexへ製品施工投入したら、投入された対象・操作権限・保護Gateを確認し、既承認の通常施工を自動継続。通常施工の逐次確認を増やさない。
I00-04 使用者体験=`各OSに合うInstaller/Packageをダウンロード→OS通常インストール→起動→基本サービス→AI実仕事/履歴/停止/復旧/更新/撤去`。Windowsは`.exe`等、非Windowsは各OSの正規形式。同一exeを全OSへ流用しない。一般ユーザーにCargo/Flutter/Python/SDK/Source Buildを要求しない。
I00-05 物理端末検証=`指示者のWindows PCで完成品を最終確認`のみ。macOS/Linux/Android/iOSは宣言するOS標準構成をVM/hosted/Emulator/Simulatorで検証。物理Mac・Linux別PC・Android/iPhoneを要求せず、それをソース施工停止理由にしない。未実証のOSは正直に`QA_NOT_ESTABLISHED`。

## I01 最初に行う読込：一度の着工同期、以後差分

I01-01 取得=`最新指示者原文, AGENTS.md, 規定/00_日本語基底規定.md, 規定/正本索引.json, v5三点, v4採用記録/進捗, HEAD/remote/local, dirty/stash/worktree/他writer,2世代backup,Release Registry`。HEADが基準から前進していれば現在の実コード・検査・残件を再観測。過去snapshotを最新値と偽らない。
I01-02 原典索引=`docs/総合機能拡張_rev1/, docs/D4_POCKET_PHASE_MAPPING.md, docs/PRODUCT_COMPLETION_PLAN.md, docs/REV2_PROGRESS.md, docs/REV3_PROGRESS.md, docs/REV4_ACCEPTANCE_LEDGER.md, docs/REV5_PRODUCT_PROGRESS.md, docs/QE_2026-10-22/, docs/QE_2026-11-05/, docs/QE_2026-11-08/, docs/QE_2026-11-09/, docs/QE_2026-11-13/, docs/緩衝基盤_rev5補遺_QC/, docs/FINAL_QA_QUEUE.md, docs/specs/, specs/, packages/, native/, apps/, installer/, tooling/, .github/workflows/, README, QUICKSTART, CLAIM, COMPATIBILITY_MATRIX, MOBILE_STATUS, INSTALLER_STATUS, RELEASE_CHECKLIST, SECURITY`。
I01-03 **読み方の命令**：全原典・全Sourceの再逐条写経は禁止。既存v4 `E0_要求consumer台帳`・K01〜K14の85コミットの解析成果・既存W00/F07証拠を**現在の差分に限定して再利用**。必要な原文・契約・呼出側/消費側は該当作用のSource施工時に読む。参照先不存在と未取得は`UNKNOWN`、無断補完なし。
I01-04 `E0`で固定するものは**4つのみ**：①現HEAD/正本/保護差分、②全要求ID範囲と原典、③A〜J共通契約・Production実装先とBroker安全境界、④安全上停止すべき作用・独立継続可能線。個別IDの21列・全SourceSHA・全consumerの完全確定は着工条件でない。
I01-05 設計上の安全衝突を除き、E0完了を「K15〜K20の次行を記録した」など台帳進度へ転換することを禁止。E0の最終アクションは**E1の最初の本番Source編集へ移ること**。

## I02 既知の設計失敗を適用時に遮断する

I02-01 v4の失敗=`E0で全要求ごとの21項目台帳・Consumer・証拠を完全に定義してからE1着工`。実観測=`v4採用後85コミットでK01〜K14の台帳周回、製品Source差分0`。これは**施工規律の欠陥**であり、文書反復を続ける根拠にしてはならない。
I02-02 v5の是正=`要求母集団の存在はE0で保全、詳細要求→Source→実Consumer→証拠の全数完成はE3〜E5へ移す`。合格下限・安全試験・18Blockerは削らない。
I02-03 v4 `I12-06`の「旧文書とAGENTSの完全一致後にE1」は**旧工程残差**として廃止。正式採用の統治記録は1単位で扱い、非安全の細部文書不整合は`MIGRATION_PENDING`としてBUILDへ進める。安全矛盾は対象作用だけ即停止・改修し、独立線は続行。
I02-04 AGENTS.md §3の`task最小変更`は**局所安全と対象範囲を守る意味**であって、製品の全体施工を一画面/一機能の逐次完了へ縮退させない。§4/§19/§20/§25のGit・検査閉包は**意味ある横断BUILD_BATCH**へ適用。相反する現AGENTS条文を採用時に**差分同期**し、古い規定を黙って残さない。
I02-05 カスタムプロンプトは実際にCodexへ適用されている内容/効果が確認できる範囲のみ参照。クラウドで届かない指示があっても本3点とRepo AGENTSから必要な目的・権限・順序を復元。共通プロンプトを製品仕様の上位へ勝手に昇格しない。
I02-06 「安全上の不明」=該当作用のfail-closed/局所停止。「非安全の要求詳細UNKNOWN」=全体系BUILD継続・E3で回収。「OS/資格/Signing未取得」=該当QA/Releaseの保留。「他writer/未commit資産衝突」=その書込み停止・保全、独立施工設計継続。いずれも全作業を一律停止にするな。

## I03 工程実行の主命令：**今は製品を作る**

I03-01 製造順=`E0着工同期→E1全線実Source一巡→E2全Consumer結合→E3全数台帳/BUILD_COVERED→E4統合QC→E5OS別QA→E6Owner Review`。D4のW00〜W08・K01〜K20・P/Phase・Q/FQは**製造順を指示する番号でない**。
I03-02 E1最初の一巡に**A〜Jの全線と5OSの本番構築を投入**。唯一の安全Broker/共通Task契約を基線に、A（AI実仕事）、B（異種Runtime/A2A）、C（製品UI）、D（Compose/AI Edit）、E（独立Export/Module pruning）、F（Broker/Buffer）、G（Windows Installer）、H（Linux/macOS native Package）、I（Android/iOS）、J（Evidence/Release）を同一構成へ実装。UI/Manifest/Docsだけで進んだとは認定しない。
I03-03 完成した旧Source/契約は継承し、新たなダミー書換えを作らない。欠落線の本番Sourceを追加/結合し、Source差分と実Consumerへの効果がある施工を進める。技術依存による局所順序は許すが、W02/G2/Q0/非Windows実機結果を他の線の着工Gateにしない。
I03-04 初回一巡が不完全でも**台帳の補完を目的としたK個別連続コミットへ戻らない**。追加の詳細解析は必ず特定の実装作業・安全修復に必要な限度に結び、実Source変更へ帰着させる。
I03-05 `E1_EXIT=全A〜Jに既存または新規の実Product Sourceおよび次Consumerへの有効接続先が存在`。実行中未配線はE2 `BUILD_GAP`。空Stubや新しいSchemaだけではE1終了でない。製造完了はE3全数の欠落0で判定。
I03-06 `E2`は全Producer→実Consumerを同一責任構造へ配線する。AIの実MCP Tool要求→Broker認可→元Tool→実結果→同一AIが結果依存で追加Task→Audit/Recovery/画面表示を実動。FlutterにAgent内部意思決定を実装して代理させない。
I03-07 `E3`で**初めて21項目級の要求台帳を全数完成**させる。C0..C34/Phase0..45の実消費、N/K/P/BUF/B/QE/FQ/OS/18Blockerを徹底照合し、対応実Sourceが不足ならE1/E2へ戻して実コードで補う。台帳だけの改善でBUILD_GAPを閉じるな。
I03-08 本番Installer/OS配布のSource設計・Build recipe・Update/Repair/Uninstall経路を初回一巡で作る。OS環境での正式インストール検査はE4/E5だが、パッケージ生成機構をQA後回しにしない。

## I04 A〜Jの具体的な欠落解消命令

I04-A `使用者目的→AI Task/Session/Runtime→認証Broker→実MCP/CLI Tool→実Result→同AIの結果依存後続Task→停止/Audit/Recovery`を本番製品入口で結線。W01旧Windows/Codex限定CLOSEDを保持。W02の通常installed後続Task/UI hangを根因修正。Mobile側AI Task/MCP Consumerの欠落も対象。
I04-B 第2の独立実RuntimeのAgent Task capabilityと失敗/拒否、Codex CLIとは異なるRuntime Adapter、外部A2AのAgent Card/Task/Message/Artifact/Stream、Compare/Handoff/Host/Workspace隔離を実消費。MINIDORA等は外部契約Adapter、内部Brainを移植しない。
I04-C Flutter/NativeのAgent Center/Workspace Inspector/History/Diff/Eval/Process/Trace/Metric/Problems/Tray/Quick Stop/Palette/Search/Notification/Recoveryを**Broker実状態**へ接続。UI描画、短命UI Timer、固定文字列、FixtureのActionは代替不可。ハングはTransport/Session/Thread/Processの根因へ戻す。
I04-D 能力の実Catalog→依存/互換/Permission→Compose→実元Tool。AI Edit proposal→独立審査・別Workspace/Session/新認可→controlled diff apply→Rollback/結果監査。Proposal/Previewと適用実体を区別。
I04-E Compose構成を本当に取り込んだ別identity/Broker/Runtime/必要ModuleのStandalone App/Installer。不要Moduleを実binary/resourceから除外し、利用機能/size/startup/権限を計測可能にする。Developer Exporterだけでは未完。
I04-F Rust Brokerの唯一Authority、OS-native Credential、認証IPC、Approval一回/期限、Content Exposure、Path/Host/Workspace/Task隔離、追記Audit/外部tamper evidence、Process tree Stop/Recovery、B5開発自己Buffer、B6結合、BUF-T01..24/効果・副作用の実作用。
I04-G Windows一般ユーザーの`.exe`等Installer→normal Launcher/初回profile/Setup Doctor/GUI Provider設定→AI実仕事→Update/Repair/Rollback/Uninstall。Portable/Stage/Debug/内部Toolを正規installerへ昇格しない。
I04-H Linux native distro/arch Package/GUI X11・Wayland条件/秘密/IPC/Update/Uninstall、macOSの正規Bundle/Native Keychain/sandbox/Permission/IPC/Packageを実装。WSL/Xvfbでの観測をnative製品受入へ転用不可。
I04-I Android/Flutter/Keystore/Device Link/lifecycle/permission、iOS/Keychain/Native Device Link/Entitlement/Permission/OS正規配布、Broker/Agent Task/Stop/Recoveryを実製品利用経路へ繋ぐ。Android Emulator/iOS Simulatorと配布用Artifactの相違を保持。
I04-J Source/契約→実Consumer/実Tool/Audit/Recovery→Artifact/OS/Requirement ID→Evidence/18 Registry/Strict Validator/Final QAへ接続。旧証拠と新候補、開発/署名/ストア、実行不能/未実施を分離。Docsのみの承認系を作らない。

## I05 変更を閉じる単位／反復禁止

I05-01 `小単位=独立編集・局所再同期`、`BUILD_BATCH=共有契約+影響Producer/Consumer+必要安全検査+製品実Source+復旧点を束ねた意味ある施工単位`。小単位開始ごとに必要な指示/HEAD/Consumerのみ確認し、全履歴/全Suite/全タグを読み直さない。
I05-02 **Docs-only/ledger-only/schema-only/fixture-only/manifest-onlyは製品BUILDの終了単位ではない**。正式採用の最小統治同期や局所安全停止記録は独立分類し、複数の同型資料更新を進捗代わりに継続禁止。
I05-03 毎回のManifest 1415 file再生成、Schema 179/Conformance 272・JSON parse・`git diff --check`一式を**K一行ごとに再実行しない**。関連Code/Contractを編集したバッチだけ影響範囲検査、全suite/OS QAはE4/E5の同一候補へ集約。ただし危険作用の必要な負例は即時。
I05-04 `CONSTRUCTION_LIVENESS_FAIL`判定：連続する意味ある作業閉包で新しいProduct Source/実Consumerへの実効果が無く、台帳/文書/Manifestだけが増えるなら**その閉包を成果としない**。K次行に進まず、現HEADの不足Consumerと実コード変更の仕事へ戻る。単にダミーコード1行を追加して逃げない。
I05-05 `PHASE_ORDER_DRIFT`：W/K/P/Phase/Qの数字順のCLOSED待ち、A完成までB着工停止、Windows全QAまでmacOS/Linux source待機、Registry全文同期までSource編集禁止を検出した場合は、実依存と安全作用以外の待機を解除。衝突があれば影響境界を記録。
I05-06 バッチ間引継ぎには必ず`次の実Producer→Consumer、影響Source位置、直前Contract、残余、安全停止、HEAD/復旧点`を残す。AIの隠れた記憶、過去会話・前モデルの自己報告だけへ依存しない。

## I06 検証の配置・実際に使うコマンド

I06-01 QE_BUILD中=`変更Sourceの型/compile、該当Contract/Consumer照合、必要な短い疎通、Authority/秘密/停止に影響する差分の即時否定検査`。単にTEST greenを増やすのは目的外。
I06-02 E4/E5で=`Rust/Flutter/Schema/Conformance/OS installed/異常/負例/長期/資源/Pruning/AI実仕事/外部A2A/Export/署名準備`を同一候補で集中検査。Q0候補固定→Q1正常→Q2故障/Recovery→Q3安全→Q4性能/耐久→Q5標準OS installed→Q6証拠→Q7Owner判定。Q0..Q7の正式始動は現規定・指示者の適用開始条件と照合。
I06-03 現Repo実体と実行環境に応じて選ぶコマンド例：`python -B -X utf8 tooling/manifest.py --check`、`python tooling/schema_check/check_schemas.py`、`python tooling/conformance_tests/run_conformance_skeleton.py`、`python tooling/validate_all.py --python-only`、`python -X utf8 tooling/run_cargo_isolated.py test --locked --offline --manifest-path native/rust_helper/Cargo.toml`、`python tooling/validate_all.py --flutter-step desktop_flutter_analyze`、`python tooling/validate_all.py --flutter-step mobile_flutter_analyze`、`python tooling/release_gate_check.py --strict-release`、`git diff --check`。コマンド実在/引数/OS/時点を確認。例を盲実行しない。
I06-04 `Strict Release FAIL`は現在の未出荷を正常に検出する場合がある。Validator/安全テストを弱くしてGOを作らず、何が未成立かを根因と証拠で記録。
I06-05 `PASS/FAIL/NOT_RUN/UNKNOWN/ENVIRONMENT_UNAVAILABLE/BLOCKED_AUTHORITY/QA_NOT_ESTABLISHED`を区別。独立再実行で偶然成功しても同条件の原因除去を証明しない。実exec/exit/SourceSHA/ArtifactSHA/Host/OS/Scopeを保存。

## I07 OS標準環境保証・物理試験の限定

I07-01 各OSの一般的対象version/arch/ABI/API、GUI/secret/security/Permission、Installer、最小resource、VM/Emulator/Simulator/hosted環境と非対応条件を候補別に定義。最低値や版数は観測/公式契約無しに創作しない。8GB Windows代表環境を包含。
I07-02 Windows=`実Windows VMへ本番相当Installer導入→first-run/SetupDoctor/AI Task/Broker/Audit/Update/Uninstall→指示者所有PCで完成候補の最終確認`。実機確認は設計成立の手合わせ調整でなく、仕様通りかの確認。
I07-03 Linux=`対象distro/arch native desktop VM/hosted + X11/Wayland/secret/IPC/package`; macOS=`適法VM/hosted GUI + Keychain/sandbox/bundle`; Android=`Android Emulator + Keystore/DeviceLink`; iOS=`iOS Simulator + Keychain/Permission`。非Windowsの物理PC/端末を要求しない。
I07-04 別OSの環境・権限が得られない場合はそのOSの`QA_NOT_ESTABLISHED`であり、全線のSource施工停止/実機調達義務ではない。Windows/WSL/Fixture/Source-onlyを非Windows installed合格へ偽装しない。
I07-05 iOSのSimulator実行Binaryと配布用IPA/device Binaryは別Artifact、共通Source/Contract/BuildConfigと各ABI/署名/Entitlementの関係を検査する。Simulatorで実機固有機能を観測したと主張禁止。ストア審査/Developer資格/公開はOwnerの別判断。
I07-06 各OSで通常利用者の入口→実Broker/AI/Tool→認可・失敗・Audit/Recovery→Update/Repair/Rollback/Uninstallを観測。規格上の制約をSecurity回避や無署名警告の無視指示で解決しない。

## I08 全歴史ID・Release Blockerの保全と移行

I08-01 実装対象全数=`N01..11/K01..20/P0..13/C0..34/Phase0..45/BUF-R01..09/BUF-T01..24/B0..6/D4-QE-R01..13/D4-LNX-T01..08/D4-AUD-F01..07/FQ-R2-A..H/TEMP/MODULE-PRUNING/TEST-LOOPBACK/P11-UNINSTALL-FINALIZER-CLEANUP/INTEGRATED/W00..08/Q0..07/OS五種/Registry18`。詳細は仕様S04〜S07、旧原典の現行採用状態から一行も落とさない。
I08-02 18機械IDを保全：`windows_evidence_provenance_isolation`,`windows_installer_first_run_smoke`,`windows_setup_doctor_smoke`,`windows_broker_installed_smoke`,`audit_anchor_external_tamper_evidence_proof`,`owner_go`,`linux_native_product_support`,`rev2_mobile_device_evidence`,`rev2_mobile_distribution`,`rev2_desktop_launch_regression`,`comprehensive_extension_rev1_completion`,`rev2_desktop_product_distribution`,`windows_distribution_identity`,`rev2_flutter_broker_channel_boundary`,`rev2_export_owner_ui_authority_path`,`rev2_module_pruning_binary_and_measurement`,`windows_rust_integration_test_execution_policy`,`rev2_mobile_flutter_native_device_link_boundary`。
I08-03 旧基準=`16 unresolved/2 resolved;release_ready=false`。新HEADで事実を照合し、旧`rev2_desktop_launch_regression`・`windows_rust_integration_test_execution_policy`のresolved履歴と旧B0..B4/W01/Qの限定証拠は当時の範囲に限る。
I08-04 物理デバイス必須という旧参照は、新`標準OS仮想installed実製品証拠`へ対象・定義・Validator・Registry・README/CLAIM/COMPATIBILITY等Consumerを整合変更。`rev2_mobile_flutter_native_device_link_boundary`と`linux_native_product_support`の**安全/Native完全性**は維持。IDを改名/削除/偽resolvedにしない。
I08-05 上記対応表の**全件詳細証拠の記入時点はE3以降**。実Consumerの不存在を隠すために文書・表・計測値だけ修正しない。

## I09 Git/権限/資産保全

I09-01 `A:委任済み通常施工`=コード/契約/テスト/必要な通常OS操作/既承認のdirect-main Git/局所安全是正。3点セットが正式施工投入されていれば逐次人間確認不要。
I09-02 `B:外部で実際に必要なInteractive Gate`=OS/Computer Use/他サービスの本人認証・Permissionクリック。対象一操作のみ停止し、必要性・対象・次の自動継続を示す。回避禁止。独立線は続行。
I09-03 `C:新Owner判断`=正式署名/鍵投入/Store公開/外部送信/未委任変更/破壊的資産削除/Owner GO。技術準備まで実施し、確定作用のみ停止。
I09-04 **製品内Permission/Approval**は上のA/B/Cの開発者権限と独立。AI/Toolの結果やCodex Full AccessをRust Brokerの許可へ転用不可。
I09-05 Git正本=GitHub main、single-writer、PR不要、2世代remote tag=`refs/tags/codex/backup-main`と`refs/tags/codex/backup-main-prev`。編集前にHEAD/remote/dirty/他worktreeと2世代の別SHAを照合・保全。既存ユーザー差分・staged・一時branch・旧証拠を無断reset/clean/deleteしない。
I09-06 **意味あるBUILD_BATCH末**に、実Source/Consumer/安全の限定検査→commit→direct-main push→remote SHA一致確認→2世代tagとrollback point確認。小単位やK台帳一行ごとのtag回転/commit/pushを禁止。保護branch/権限不足は迂回しない。
I09-07 一時検証branchは現AGENTSが認めた手動`workflow_dispatch`検査に限定、成功した正確なcommitのみmainへfast-forwardし、既存無関係branchを削除しない。OneDriveはbackup、Cargo/Flutter build/cache/tempは同期領域外、危険な削除は対象・回復性・権限で判断。

## I10 報告・停止・終了判定

I10-01 各報告の先頭は**製品Sourceに何を施工し、どの本番Producer→Consumerと実利用入口を前進させたか**。実装ゼロなら`PRODUCT_SOURCE_CHANGE=0`と明記し、台帳行増加を完成進捗に使わない。
I10-02 機械記録=`{JST,HEAD,採用正本,実Source差分,実Producer,実Consumer,Contract,A..J:実体/未接続,N01..11,全旧ID維持,BUILD_GAP,安全即時負例,OS別配布候補/仮想証拠,Windows物理確認,Q0..Q7,18Blocker,release_ready,権限待ち,未観測,原因,Git commit/push/backup2,次の実Source施工}`。
I10-03 `CONSTRUCTION_COMPLETE=(E3 BUILD_COVERED=true AND 必須実Consumer/配布Source欠落0)`。`PRODUCT_QA_COMPLETE=(同一候補E4 QC合格+宣言OS別E5実証+Windows完成品確認記録)`。`RELEASE_AUTHORIZED=(18Registry/安全/配布/署名資格/正式Owner GO成立)`。三つを非同一のまま報告。
I10-04 未完成/危険/環境不足/本人Gateのいずれも原因・対象・影響・再開条件・保護状態で説明。文書整備の無期限循環や新しい3点セット作成要求の再提出で施工を中断しない。新たな上流設計変更が必要な場合は原文・根拠・影響のある範囲だけ提案。
I10-05 完了の主張は禁止条件=`Mock/Stub/Manifest/Schemaだけ、履歴PASS転記、OS実機未試験だけを普遍保証、Owner GO未取得でrelease_ready=true、Source差分ゼロの台帳反復をBUILDと称する`。

## I11 初回施工者への明示的な再開命令

1. 本v5が**正式施工投入された時点**で現HEAD/保護された3 worktree/旧v4採用状態を再取得せよ。現v4のK01〜K14台帳を再作成するな。
2. 共通Broker/Task/Result/Permission/Approval/IPCの実ContractとA〜Jの現在のSource・実Consumerを**最小限の同期記録**で確定せよ。安全衝突を個別隔離し、非安全の詳細欠落はE3へ引継げ。
3. **次の意味ある施工成果をproduction Sourceの実改変または既存実体の実Consumer結合とせよ。** K15台帳を次の成果とするな。開始ラインは現実の未接続を観測して決め、全A〜Jを同じ初回BUILD期間に含めよ。
4. A〜Jの共通意味・共有契約を保持し、Agent実Tool→AI次Task、2種目Runtime、Compose/AI Edit/Export、全OS Installer/Native/Buffer/Evidenceを欠落なく本番コードへ一巡施工する。各小編集でQ/全Suiteを周回しない。
5. 全線Source一巡→E2実接続→E3全数詳細監査→E4統合QC→E5仮想OS QA/Windows実機確認→E6責任者判断で局所的に閉じよ。要求・安全・保証の合格下限を下げて完成を演出するな。

本指示書の存在自体は、コードを変更した事実でも作業の正式承認でもない。**目的と実行条件が承認された後、ドキュメントに滞留せず、製品を実装すること。**
