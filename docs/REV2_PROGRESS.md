# rev2 実装進捗と証拠境界

各節は作業時点の履歴である。現在状態は次の現況節と対象commitに結合した実証拠で確認し、過去の未実装記述を現在の状態へ読み替えない。

## D4 Pocket 第5段階 GUI Shell構成（2026-09-24）

GUI Shell構成のManifest-only経路を追加した。実行基盤、エージェント、ツール、MCP接続、表示テーマ、機能要件、設定を選択要求へまとめ、Rust Brokerが構造・重複・継承禁止を再検証して`GUI Shell構成`Receiptを返す。Desktop設定面は認証済みBrokerへ接続し、結果Manifestを表示する。

- Contract: `specs/gui_shell_compose.schema.json`と`specs/gui_shell_compose_receipt.schema.json`を追加し、`manifest_only`、`not_started`、`not_generated`、権限非生成、Authority Strip、継承値`none`を固定した。
- Production path: Desktop Settings → `ComposeClient` → 認証済みRust Broker `GUI Shell構成` → 構成Manifestの再検証 → `INTERNAL_STATE` AuditEvent付きReceipt。ビルド、独立アプリ識別子、資格情報、Permission、Approval、Audit chainは実行・継承しない。
- Negative boundary: 未知field、Permission継承、重複選択、不正locale、ReceiptのPermission生成を拒否する。Capability requirementは要求説明に留まり、権限を生成しない。
- 未成立分類: Windows Export、独立アプリ識別子／監査ストア／設定／実行基盤Manifestの生成、Preview／rollback、Module Pruning、Distributionは`release_blocker`。ComposeのManifest表示だけで独立アプリ完成を主張しない。

## D4 Pocket C33: Windows最大到達点（2026-09-24）

Windowsで実行可能な範囲を最大化するため、release build、Rust helper release build、Broker smoke、installed smokeの証拠収集経路を整備した。`installer/windows/collect_broker_smoke.ps1`はWindows Rust testのfull-duplex応答を維持して読み取り、`collect_installed_smoke.ps1`はUTF-8の製品JSONを明示的に読み取る。UI surface収集は親要素と同じRaw UI Automation treeを再帰走査する。いずれも、証拠collectorの責務であり、Flutterへ権限経路を追加しない。

- Validation: `flutter build windows --release`、`cargo build --release --locked`、`python tooling/conformance_tests/run_conformance_skeleton.py`（182 checks）、`python tooling/validate_all.py --python-only --desktop-platform windows`（exit 0）はPASS。`python tooling/windows_release_evidence.py --evidence C:\Users\ohira\AppData\Local\GUI-Shell\installed-runs\c33-2c8ad7d\runtime\evidence\windows_installed_smoke.json`は、正式Windows証拠が未成立のためexit 1となった。
- Clean isolated run: source commitは`2c8ad7ddd033ef97d90989341450b37f84a87d76`、source worktreeはclean。Broker smokeはcollector v4で`passed`となり、認証IPC、永続store、replay拒否、再起動後health、crash fail-closedを確認した。installed smoke自体はexit 0だが、`installed_status=failed`、`visible_surfaces=[]`、`visible_complete=false`、`setup_doctor_status=warning`、`setup_doctor_path=false`である。
- Evidence boundary: Windows UI Automationの正式collectorはroot／Flutter viewの2要素だけを観測し、個別surfaceを証明できない。Computer Useで個別semantic surfaceを補助観測できても、正式collectorのLIVE_RUNTIME／EXTERNAL_EVIDENCEを代替しない。Setup Doctorは製品プロセスのresolved executableがCodexホストのLocalCache配下となり、installed contextの厳密なpath一致を満たさない。path比較を緩めて成功扱いにはしない。
- `release_blocker`: `windows_evidence_provenance_isolation`、`windows_installer_first_run_smoke`、`windows_setup_doctor_smoke`、`audit_anchor_external_tamper_evidence_proof`、C28の8時間実測、外部Runtime／Agent／MCP／A2A、実端末、正式署名・配布、owner GO、正式release。
- `known_limitation`: Broker installed smokeの個別sub-gateはPASSしたが、総合installed evidenceの成立へは昇格させない。UI AutomationのFlutter surface取得限界とCodexホストの実行path virtualizationは、現環境で観測した制約である。

この単位はWindows最大到達点の開発候補であり、製品releaseやC34正式release前作業の完了を意味しない。

## D4 Pocket C31: 文書更新（2026-09-24）

README、ROADMAP、GUI操作面、SECURITY、CONFIG、MOBILE_STATUS、COMPATIBILITY_MATRIX、および本書を、C24〜C30の現行実装・検証範囲へ更新した。更新の目的は、製品の未検証範囲を隠さず、各面のproduction path、Authority境界、Content Exposure境界、証拠class、残存分類を同じ状態へそろえることである。過去の文書記録に含まれる旧Schema／Conformance件数、旧platform証拠、旧工程状態は履歴として保持し、現行commitのPASSへ読み替えない。

- 現行基準: Schema 108件、正常example 108件、negative fixture 129件、Conformance 182件、厳格日本語監査、manifest検査はPASS。
- 現行回帰: C27性能smoke、C28 30秒運用smoke、C29障害注入8件、C30回帰matrixはPASS。C28の8時間実測は途中で通常対話通信失敗となり、PASSへ昇格していない。
- 証拠境界: C30 Agent probeはCodex CLI version/help interfaceだけ、Compare／Device Linkはfixture、Runtime／Dialogueはlocalhost fixtureを含む開発経路であり、installed product、外部Runtime／Agent、実端末を証明しない。
- `release_blocker`: Windows installed productの総合証拠（provenance、first-run、Setup Doctor、Audit anchor外部改変）、8時間実測、外部Runtime／Agent／MCP／A2A、実端末、正式署名・配布、C32〜C34、owner GO、正式release。Broker installed smokeの個別sub-gateはC33でPASSしたが、総合evidenceの成立条件は満たしていない。
- `known_limitation`: MobileのMCP live一覧非提供、fixtureだけの比較／Device Link、外部接続未成立、実測不能値の`unknown`表示。

検証入口: `python tooling/full_regression_validation.py` および `python tooling/validate_all.py --desktop-platform windows --include-mobile-release`。いずれも開発検証であり、正式releaseの許可ではない。

## D4 Pocket C24: Mobile対応（2026-09-24）

Desktopの既存Broker契約から、Mobileへ必要な状態確認と復旧導線を選択的に投影した。MobileのNavigationは既存9画面を保持したまま、資源概要、履歴、MCP状態を追加した。端末TLS経路は、Runtime lifecycle状態、資源観測、通知summary、停止receipt、現在owner承認に結合した履歴metadataだけを既存Rust Broker handlerへ渡す。

- Production path: Mobile Flutter → Device Link TLS → Desktop Rust Broker → 既存読み取り専用handler → bounded projection。Mobile専用の権限判定、owner操作、別bridge、別監査storeは追加していない。
- Authority boundary: MobileはApprovalを発行・編集・延長・失効せず、MCP接続・Tool実行・Credential参照・実停止を行わない。停止画面はowner再承認待ちreceiptだけを表示し、資源のunknownを0へ変換しない。
- Content boundary: 通知はsummary、履歴は現在承認のmetadataだけで、対話本文・Approval payload・Audit raw reason・Credential実値を投影しない。Host表示は既存保存資格のmetadataに閉じる。
- Validation target: Mobile flutter analyze、flutter test、Rust端末統治試験、C24 Conformance、Schema、厳格日本語監査を実行する。
- 未成立分類: Mobile実機・Android/iOS安全保管・TLS実接続、Windows installed productでのMobile連携、長時間運用・障害注入、owner GO、C0-C34全数完成、正式releaseはrelease_blocker。MCP live一覧をMobileへ出さないことはowner専用管理面を守るknown_limitation。

## D4 Pocket C23: Desktop UX統合（2026-09-24）

既存20画面を削除せず、Desktop Navigationを`運用`、`安全`、`開発`、`設定`、`すべて`の論理グループへ整理した。グループ選択はFlutterの表示状態だけを変更し、別グループの画面へ移動した場合は対象グループへ切り替える。画面本体と既存のBroker、owner control、Approval、Audit、Recovery経路は置き換えていない。

- Production path: 操作者の操作グループ選択 → Flutter `NavigationRail`の表示対象絞り込み → 既存画面の選択。全体表示では既存20画面を現在の順序で表示する。
- Authority boundary: グループ名と選択状態はUI状態であり、Authority、Permission、Approval、Credential、Broker IPC、filesystem、process、networkを生成・実行しない。
- Validation: Desktop Flutter全93試験、`flutter analyze`、`flutter build windows --debug`、Conformance 178 checks、厳格日本語監査、Schema検査を実行しPASSした。Rustは変更していないため今回のRust全試験は未実行。
- 未成立分類: Windows installed productでの4グループ実画面操作evidence、C0-C34全数完成、正式release、owner GOは`release_blocker`。Mobile Navigationへの投影はC24の対象で`known_limitation`。

## D4 Pocket C22: 全体検索（2026-09-24）

既存の操作面を横断する読み取り専用の全体検索を追加した。`Ctrl+Shift+F`と画面上の全体検索ボタンから開き、現在の`ShellSnapshot`に含まれるbounded metadataと、MCP／A2A／評価／通知などのsurface entryを検索する。検索結果の選択は対象画面への移動だけであり、検索から権限作用へ到達しない。

- Production path: Desktopの全体検索 → `GlobalSearchIndex` → Snapshotの表示用metadata → 対象画面へのGUI navigation。indexは512件、検索語は128文字、結果は30件にboundedする。
- Authority boundary: Agent／Runtime／Session／Approval／Audit／MCP／A2A等のmetadata、History、Profile、TelemetryはAuthority、Permission、Approval、Credentialを生成・再利用しない。FlutterのindexはBroker IPC、filesystem、process、network、credential、Clipboardへ直接到達しない。
- Content boundary: 対話本文、Approval payload、Audit raw payload、Credential実値、秘密値、未許可のfull content、任意pathをindexへ登録しない。Broker由来と確認できないsnapshotは証拠範囲を`不明`と表示する。
- 未成立分類: 実Runtime・Agent・MCP・A2A・Hostの全surfaceから取得したinstalled productでの横断検索実機evidence、clean installed artifactとの結合、owner GOは`release_blocker`。外部surfaceのlive再取得、全文検索、本文検索、検索結果からの操作実行を提供しないことは`known_limitation`。

## D4 Pocket C20: Windows常駐トレイ（2026-09-24）

Windows native trayを、表示と操作入口に限定したWin32実装として追加した。トレイからD4 Pocketの前面化、実行系状態、保留承認件数、重大通知件数、全Runtime停止要求、終了を操作できる。表示射影はBroker由来値だけを採用し、取得不能値を0へ変換しない。

- Production path: Windows Win32 tray → Flutterの固定`gui_shell/tray`表示チャネル → Snapshot／通常Broker IPC。停止要求は`全Runtime停止要求`としてRust Brokerへ渡し、lifecycle registryの対象をboundedに列挙したreceiptを返す。
- Authority boundary: Win32 trayとFlutterのトレイチャネルはAuthority、Permission、Approval、Credential、Runtime操作を所有しない。停止receiptは`停止実行済み=false`、`承認状態=owner_reapproval_required`、`権限生成=なし`に固定し、直接killとowner承認生成を行わない。
- Validation: C20 Schema 3件、正常例3件、負例3件、Conformance 175 checks、厳格日本語監査、release runtime assertion、Rust全試験（lib 218、owner CLI 7、IPC 9、canonical hash 1、checkpoint 8、protected data 2、protected startup 1、protected store 3、workspace diff 2、workspace reader 2、workspace startup 7）、Desktop Flutter解析・86試験、Windows debug buildを実行しPASSした。
- 未成立分類: Windows installed productでのトレイアイコン・前面化・常駐・終了・Broker停止要求の実機evidence、owner再承認後の全Runtime実停止lifecycle統合、正式releaseは`release_blocker`。Windows以外のnative tray未提供は`known_limitation`。

## D4 Pocket C19: Adapter管理操作（2026-09-24）

Runtime CenterのAdapter catalogをRust Brokerのbounded管理経路へ接続した。`アダプター一覧`は通常IPCのmetadata-only projection、導入・検証・有効化・無効化・隔離・更新・削除はowner controlだけが状態を変更する。通常IPCの変更要求は`owner_reapproval_required`のsuspendedとして監査し、状態を変更しない。

- Production path: Desktop Runtime Center → `アダプター一覧`／Adapter管理要求 → Rust Adapter Center → `adapters.json` → metadata-only receipt。導入・更新はManifestをcatalogへ登録するだけで、外部download、filesystem、processは実行しない。
- Authority boundary: 署名はBroker所有Ed25519 trustとManifest正本byteで検証し、未検証Adapterは有効化できない。Adapter metadata、署名、hash、過去状態からPermission、Approval、Authority、Credentialを生成しない。
- Quarantine boundary: 隔離済みRuntime IDを実行系登録、lifecycle、資源観測、対話で再利用しない。削除はcatalog recordだけを除去し、外部artifactの実削除を主張しない。
- Validation target: Adapter管理Schema、正常／負例fixture、Rust state transition／signature path／restart state、Conformance、Desktop Runtime Centerを接続する。
- 未成立分類: 外部artifactの実download・filesystem導入・process起動・実削除、Windows installed productのAdapter管理実証は`release_blocker`。C19だけでAdapter製品機能全体または正式releaseを主張しない。

## D4 Pocket C18: Host操作面（2026-09-24）

C17のHost registryをDesktopのHost操作面へ接続した。`ShellCoreClient.product()`は通常認証済みBroker IPCの`Host一覧`を取得し、Host metadataをSnapshotへ投影する。DesktopはHost一覧、接続状態、Trust、Runtime／Agent summary、Host切替を表示する。

- Production path: `Host一覧` → Rust Brokerのbounded metadata-only receipt → Desktop Snapshot → Host操作面。`Host切替`は通常IPCでregistryのHost IDを再照合し、Audit確定後に表示コンテキストのreceiptを返す。
- Authority boundary: Host切替は`権限生成=なし`、`authority_strip=true`、`承認状態=not_reused`に固定する。Host AのPermission、Approval、AuthorityをHost Bへ再利用しない。Host registryのRuntime／Agent件数は`INTERNAL_STATE` summaryであり、個別live一覧の証拠ではない。
- Observation boundary: 選択Hostが現在のBroker観測Hostと一致するときだけ現行SnapshotのRuntime／Agentを表示する。それ以外はsummaryと`未観測`だけを表示し、remoteの個別状態を推測しない。
- Validation target: Host切替の未知Host、未知field、owner channel、Host間非混線をRust unit test、Schema、Conformance、Desktop widget surfaceへ接続する。
- 未成立分類: live Host再接続、Trust検証、Host別remote Runtime／Agent discovery、Host間Workspace隔離、Device Link実認証、Desktop installed evidenceは`release_blocker`。C18だけでHostの実接続や正式releaseを主張しない。

## D4 Pocket C17: 複数Host registry（2026-09-24）

C17のHost metadata registryをRust Security Brokerへ接続した。owner controlだけがHostを登録し、通常認証済みIPCだけがHost一覧を参照する。Host ID、表示名、Platform、接続状態、Trust、証明書／identity hash、Runtime summary、最終接続をboundedなmetadata-only receiptへ射影し、`hosts.json`へatomicに永続化する。登録時のTrustと接続状態は`pending_review`に固定し、Host metadataからPermission、Approval、Authority、Credentialを生成しない。

- Production path: owner control → `Host登録` → Rust Host registry → hash-only identity／bounded summary → `hosts.json`。通常認証済みIPCの`Host一覧`はBroker内部registryを`INTERNAL_STATE` metadata-onlyとして返す。
- Boundary: 通常IPCからの登録、owner controlからの一覧、未知field、重複Host ID、identity実値、`permission_id`／`approval_id`／`authority`、connected／verifiedの自己申告、state fileのmalformed／重複field／上限超過を拒否する。Host Aのmetadata・承認・PermissionをHost Bへ共有しない。
- Validation: Schema 99件／正常例99件／負例118件、Conformance 172 checks、Rust全試験251件（lib 209、owner CLI 7、IPC 9、canonical hash 1、checkpoint 8、protected data 2、protected startup 1、protected store 3、workspace diff 2、workspace reader 2、workspace startup 7）、厳格日本語監査、manifest、packaging portability、Windows開発集約検証をPASSした。集約検証はdevelopment modeであり、既存のWindows installed証拠5件を`release_blocker`として保持する。
- 未成立分類: Host切替、HostごとのRuntime／Agent一覧、live接続再確認、Device Linkの実認証・失効・quarantine、Host間Workspace隔離、複数Agent比較／Handoff、Desktop Host操作面、installed product証拠、長時間運用・障害注入は`release_blocker`。C17だけで複数Host製品機能全体または正式releaseを主張しない。

## D4 Pocket C16: A2A接続センター初期経路（2026-09-23）

C15のA2A外部概念契約を、owner controlから実Agent Cardを取得するRust Security Broker経路へ接続した。現行単位はloopback HTTPだけを許可し、Agent Cardのbounded検証と`LIVE_RUNTIME` metadata-only receipt、通常IPCの接続一覧を成立させる。外部Agentの宣言からTrust、Permission、Approval、Authorityを生成しない。

- Production path: owner CLI／control → `A2A接続` → Rust A2A接続センター → loopback HTTP Agent Card → Agent Card検証 → receipt。通常認証済みIPCの`A2A接続一覧`はBroker内部接続状態を`INTERNAL_STATE`として返す。
- Boundary: HTTPS、非loopback HTTP、redirect、Transfer-Encoding、Content-Encoding、重複Content-Length、巨大header／body、credential実値、Agent Card endpoint実値、Task／Message／Artifact raw contentは拒否または非投影。Trustは`pending_review`、Capability diffは`not_evaluated`、`authority_strip=true`、`権限生成=なし`、`公開範囲=metadata_only`に固定する。
- Bounded behavior: Agent Card body 1MiB、header 16KiB、URI 2048 bytes、接続一覧64件、接続・読取期限bounded。同一AgentIDの再接続、ownerからの一覧、通常IPCからの接続要求を拒否する。
- Validation: Schema 96件／正常例96件／負例115件、Conformance 171 checks、Rust全試験247件（lib 205、owner CLI 7、IPC 9、canonical hash 1、checkpoint 8、protected data 2、protected startup 1、protected store 3、workspace diff 2、workspace reader 2、workspace startup 7）、厳格日本語監査をこの単位で実行しPASSした。Desktop専用接続画面、HTTPS実接続、実A2A Test Harnessは未検証または未接続として別分類する。
- 未成立分類: HTTPS TLS、公開endpoint discovery、認証実行、Task／Message送信、Artifact本文、Stream購読、cancel、再接続、quarantine、複数Agent比較、Handoff、Desktop接続画面、Windows installed product証拠、長時間運用・障害注入は`release_blocker`。C16初期経路だけでA2A製品機能全体または正式releaseを主張しない。

## D4 Pocket C16補完: A2A接続metadataの再起動復元（2026-09-24）

C16のsession memoryだったA2A接続registryを、Broker永続storeの`a2a_connections.json`へbounded・atomicに保存し、再起動時にstrict再検証して復元する経路を追加した。URI実値、credential実値、raw content、未知field、重複AgentID、上限超過は保存または起動時復元しない。

- Production path: `A2A接続` → Agent Cardのmetadata-only receipt → Rust Brokerの永続state → Broker再起動 → strict state検証 → 通常IPCの`A2A接続一覧`。
- Safety boundary: 復元receiptは`接続状態=restored_pending_review`、`証拠種別=INTERNAL_STATE`、`承認状態=owner_reapproval_required`へ降格する。過去のowner承認を再利用せず、復元stateからPermission、Approval、Authority、credential実値を生成しない。
- Validation: 再起動を模したRust Broker再open試験で1件のreceipt復元、状態降格、state file内のendpoint実値非保持、malformed stateのfail-closedを確認した。最終Rust試験は248件（lib 206、owner CLI 7、IPC 9、canonical hash 1、checkpoint 8、protected data 2、protected startup 1、protected store 3、workspace diff 2、workspace reader 2、workspace startup 7）で全件PASSした。
- 未成立分類: TLS再接続、Agent Card再検証、disconnect、quarantine、Task／Message送信、Artifact本文、Stream購読、複数Agent比較、Desktop接続画面、実A2A Test Harnessは`release_blocker`。復元は表示registryの再構成であり、外部Agentとのlive connection復旧ではない。

## D4 Pocket C14: 追跡情報閲覧（2026-09-23）

C13の`観測一覧`を再利用する読み取り専用Trace InspectorをDesktopへ接続した。Broker内部で実測されたSpanを、開始・終了・所要時間・状態・親Span・関連Audit・エラー分類のbounded waterfallとして表示する。

- Production path: Desktop Trace Inspector → 通常認証済みBroker IPC → Rust Observation Center → `観測一覧`。TraceID filter、手動更新、Broker接続なしのfail-closed表示を接続した。
- Evidence boundary: 現在の実測対象はBrokerだけである。Runtime、Adapter、Tool、外部通信はproduction観測経路が接続されるまで実測済みと表示しない。表示は`INTERNAL_STATE`であり、Auditのreason、payload、metadata、秘密値を表示しない。
- Authority boundary: Trace表示からPermission、Approval、Authority、Capability、Credentialを生成しない。Traceは監視表示であり、承認・実行・権限判断の経路ではない。
- 検証: Desktop NavigationRail／追跡情報画面を含む全83 Flutter試験、Flutter analyze、Schema 92件／正常例92件／負例109件、Conformance 169 checks、厳格日本語監査、manifest検査を実行し、PASSした。`python tooling/validate_all.py --python-only --desktop-platform windows`も開発モードでPASSした。
- 未成立分類: Runtime／Adapter／Tool／外部通信のproduction trace、OpenTelemetry export、外部collector、long-term trace、8時間運用、Windows installed product証拠は`release_blocker`。C14はBroker内部観測の閲覧範囲だけを主張する。

## D4 Pocket C15: A2A外部概念射影契約（2026-09-23）

A2AのAgent Card、Task、Message、Artifact、Streamを、実接続なしのboundedな`metadata_only`契約へ射影した。C15はSchema、valid／negative fixture、Conformance、正本文書だけを追加し、外部Agentへの接続やTask実行経路は追加していない。

- Contract path: A2A外部概念 → `a2a_contract.schema.json` → Schema／fixture／Conformance。`protocol_version`、Agent Cardのinterface／capability／skill／authentication metadata、Task状態、Message／Artifactのpart kind・件数・hash、Stream状態を固定した。
- Authority boundary: Agent CardはTrustではなく、TaskはApprovalではなく、MessageはPermissionではなく、ArtifactはAuthorityではなく、StreamはCapability grantではない。全概念へ`authority_strip=true`を要求し、`権限生成=なし`、`公開範囲=metadata_only`、認証実値・endpoint実値・raw content非保持を検証する。
- Evidence boundary: valid fixtureは`FIXTURE`であり、実Agent、実endpoint、実認証、実Task、実Streamの証拠ではない。Trustはoperator review待ち、Capability diffの追加・変更・削除はoperator review必須とする。
- Validation: Schema 93件／正常例93件／負例112件、Conformance 170 checks、厳格日本語監査、manifest検査を実行する。Rust／Flutterは変更していないため、この単位では未実行とする。
- 未成立分類: Agent Card discovery、A2A binding、外部Agent登録、credential注入、Task／Message送信、Artifact本文、Stream購読、timeout、取消、再接続、quarantine、複数Agent比較、Handoffは`release_blocker`。C15契約だけでA2A接続や製品完成を主張しない。

## D4 Pocket C13: 観測センター（2026-09-23）

BrokerのAudit確定処理を、Auditとは別の内部観測としてboundedなSpan、Trace、Metricへ射影し、Desktop観測センターへ接続した。観測はsession内memoryに限定し、caller登録、外部export、権限生成を持たない。

- Production path: Desktop観測センター → 通常認証済みBroker IPC → Rust Observation Center → Brokerが確定したAudit eventの処理時間。`観測一覧`の上限とTraceID filterを接続した。
- Authority boundary: 応答は`INTERNAL_STATE`に固定し、Auditのreason、payload hash、metadata、credential、raw contentを返さない。観測からPermission、Approval、Authority、Capabilityを生成しない。
- Bounded behavior: Broker保持1024Span、IPC返却256Span／256Trace／16Metric。測定不能なdurationは0ではなく`unknown`として扱い、OpenTelemetry exportは`unsupported`である。
- Validation: 観測Schema 4件、正常／負例fixture、Rust内部観測・bounded・Metric・Broker経路試験、Desktop client／NavigationRail／全Flutter試験を接続した。
- 未成立分類: C14 Trace Inspectorのwaterfall、親子Span、OpenTelemetry export、外部collector、Runtime全体の実測、Windows installed productでの観測証拠、8時間運用は`release_blocker`。本単位はBroker Audit確定処理の内部観測だけを主張する。

## D4 Pocket C12: 通知センター（2026-09-23）

監査eventから通知summaryを限定射影するRust Notification Centerを追加し、既読・破棄状態をBroker所有の`notifications.json`へhash結合して保存する。通知をcallerが登録する操作はなく、通知操作自身も通知sourceへ射影しない。

- Production path: Desktop通知画面 → 通常認証済みBroker IPC → Rust Notification Center → 監査event／表示状態。`通知一覧`、`通知既読`、`通知破棄`、`通知全既読`を接続した。
- Authority boundary: 通知は`INTERNAL_STATE`のsummaryであり、Permission、Approval、Authority、Capability、Credentialを生成しない。理由、payload、metadata、秘密値を返さず、開く操作は画面navigationだけである。
- Bounded behavior: 監査走査は4096件、通知返却は256件、表示状態はhash付き4096件まで。malformed state、未知field、重複ID、stale hashはfail-closedとする。
- Validation: 通知Schema 3件、正常／負例fixture、Rust 4単体試験、Desktop通知画面／NavigationRail、Schema／Conformanceを接続する。
- 未成立分類: Windows native toast投影、Windows installed productでの通知実証、8時間運用、全通知sourceの実Runtime証拠は`release_blocker`。本単位はin-app通知センターの完成範囲だけを主張する。

## D4 Pocket C11: 更新センター（2026-09-23）

更新候補のSchema、Rust Broker署名検査、永続一覧、延期、download／適用／rollback要求、Desktop設定画面を接続した。候補metadataから署名対象byteを再構成し、Broker所有のEd25519公開鍵とfingerprintを使う。候補側の公開鍵、Profile、MCP metadata、履歴、UI stateは信頼源にならない。信頼設定未構成または署名不正の候補は保存しない。

- Production path: Desktop設定画面 → 通常Broker IPC → Rust Update Center → `updates.json`／Audit。`更新一覧`、`更新署名検査`、`更新確認`、`更新延期`、`更新download要求`、`更新適用要求`、`更新rollback要求`を接続した。
- Authority boundary: 更新署名の信頼はBroker所有設定だけから成立する。署名済み候補もPermission、Approval、Authority、Credential、外部process実行権限を生成しない。candidate hashのstale照合を行う。
- Execution boundary: download、install、process、rollbackの外部実行はsuspendedである。要求receiptを実行完了と報告しない。
- Validation: UpdateCandidate／Receipt／ListのSchema、negative fixture、Ed25519検証、信頼設定未構成、候補hash、永続化、実行要求suspended、Desktop UpdateClientを検証する。
- 未成立分類: 外部download、install、process、rollback適用、owner公開鍵の本番provisioning、Windows installed productの更新実証は`release_blocker`。C11のBroker要求受付と署名検査だけで更新製品機能または正式releaseを主張しない。

## D4 Pocket C10: 運用プロファイル（2026-09-23）

運用プロファイルをSchema-firstで定義し、Rust Brokerの永続状態と通常認証済みIPCへ接続した。ProfileはRuntime、Adapter、要求Capability、Content Exposure、network exposure、resource limit、UI preferenceだけを保持する。`プロファイル適用要求`はProfile hashを照合して適用意図をAuditへ記録するだけで、Permission、Approval、Authority registry、Runtime操作を変更しない。

- Production path: Desktop設定画面 → 通常Broker IPC → Rust Profile Center → `profiles.json`のatomic write／起動時再検証。作成、複製、適用要求、削除、export、import、一覧を実装した。
- Authority boundary: Profileは要求configurationであり、権限源ではない。Schemaの追加field拒否とBrokerの`deny_unknown_fields`でPermission、Approval、Authority、Credential実値、Audit identityの混入を拒否する。
- Validation: Profileの正常／負例、作成・適用要求・再起動後再読込・禁止field拒否をRust試験へ追加した。Schema／Conformance／日本語基底監査／Desktop Flutter解析を実行する。
- 未成立分類: 専用ファイル選択UI、ProfileからRuntimeへ実設定を反映する操作、ProfileのPermission・Credential・MCP・A2A接続は`release_blocker`。現在のC10は要求設定の保存・監査・表示までであり、Profile適用による権限作用を主張しない。

## D4 Pocket Phase 4: Codex AdapterのBroker登録とread-only Launcher基盤（2026-09-23）

Windowsで確認した実物Codex CLIを、Rust Brokerの明示起動設定から既存の実行系対話経路へ接続した。`--codex-runtime <ID=絶対executable path=絶対workspace path>` はowner起動時だけ受け付け、Adapterの登録時にexecutable／workspaceの絶対path、通常file／directory、secret pathでないこと、`codex --version`、`codex exec --help`を確認する。IPCから任意の実行path、argv、environment、workspaceを受け取る経路は追加していない。

- Production path: `実行系挙`で登録されたCodexを既存の `対話開始`、owner承認付きの`対話送信`、`対話取得`、`対話中止`、`対話終了`から利用する。汎用`command_envelope` dispatchは引き続き停止中で、Flutterはprocess／filesystem／credential／networkを直接実行しない。
- Runtime boundary: 実行は固定された `codex exec --json --ephemeral --ignore-user-config --sandbox read-only --color never --cd <workspace> -` に限定し、環境変数はallowlistだけを渡す。JSONLのthread、agent message、turn完了を検証し、失敗event、不正応答、出力上限超過、取消、期限超過を成功へ昇格しない。Broker起動と認証付き通常IPCによる`codex`実行系列挙をWindowsで実行確認した。
- Evidence boundary: Codex CLIのread-only JSONL応答`READY`は、独立したephemeral／read-only smokeで実物interfaceを確認したもの。Broker登録と実行系列挙はLIVE_RUNTIMEの実Broker証拠である。静的conformanceはsource boundaryを検査するが、write-capable Agent、MCP、複数Agent比較、Handoff、長時間運用、installed productを証明しない。
- 未成立分類: write-capable Agent execution、実taskのProduct UI完結、MCP／A2A、複数Agent比較／Handoff、Usage／Cost、Claude／Gemini接続は`release_blocker`。Codex Adapterのread-only限定、`--codex-runtime` executable path内の`=`未対応、未導入Vendorは`known_limitation`として保持する。

## D4 Pocket C7: 資格情報保管庫のowner登録と公開metadata（2026-09-23）

C7の最初の完結単位として、owner controlから新規資格情報をWindows ProtectedStoreの`Purpose::Credential`へDPAPI保管し、通常IPCへ検証済みmetadata一覧を返すContractとBroker経路を接続した。同じ資格情報IDの再登録、normal IPCからの追加、owner channelからの一覧、保管先未登録、暗号文欠落・改変はfail-closedで拒否する。

- Contract: `credential_registration.schema.json`、`credential_receipt.schema.json`、`credential_list.schema.json`、正常例、権限field混入・秘密値混入・件数負値の負例、`docs/specs/credential-vault.md`を追加した。秘密値は登録payloadからDPAPIへ渡すだけで、receipt、一覧、Audit reason、CLI出力へ投影しない。
- Production path: `資格情報登録`はowner資格経路だけ、`資格情報一覧`は通常資格経路だけを受け付ける。Flutter、Adapter metadata、Profile、History、MCP metadata、A2A Agent Cardは資格情報の権限源にも登録経路にもならない。資格情報のRuntime／Tool／MCP／A2A注入はまだ接続していない。
- Validation: `python tooling/schema_check/check_schemas.py` はSchema 74件／正常例74件／負例89件、`python tooling/conformance_tests/run_conformance_skeleton.py` はConformance 163 checks、`python tooling/日本語基底監査.py --strict`、`python tooling/validate_all.py --python-only --desktop-platform windows`、manifest、release gate、packaging portability、release smoke、evidence bundle、release runtime assertionsがPASSした。C7専用Rust試験は3件、Rust全試験はlib 176件、main 5件、統合35件、合計216件がPASSした。今回のC7専用試験ではowner/normal channel、秘密値非投影、暗号文欠落時の部分一覧拒否を確認した。なお、未stage状態ではpackaging対象が追跡file一覧に限定されるため、C7新規fileをstageした状態でpackaging portabilityを実行した。
- 未成立分類: 資格情報の取得・Runtime／Tool／MCP／A2A注入、更新、失効、削除、接続先変更、Recovery操作、GUI管理面、Windows実機owner登録証拠、非Windows安全保管は`release_blocker`。登録と一覧だけで資格情報保管庫全体または製品releaseを主張しない。

## D4 Pocket C8: MCP外部概念射影契約（2026-09-23）

MCP実接続に先行して、外部metadataをGUI-Shellの境界付き契約へ射影した。`Server`、`Tool`、`Resource`、`Prompt`、`Transport`、`Credential ref`、`Trust`、`Capability diff`を`metadata_only`として表し、MCP metadataからAuthority、Permission、Approvalを生成しない。C8では外部Serverの発見・接続・Tool実行を開始していない。

- Contract: `mcp_contract.schema.json`、正常fixture、ToolへのPermission混入、Credential refへの秘密値混入、Authority／Approval混入の負例、`docs/specs/mcp-contract.md`を追加した。Transportの接続先はhashのみ、Credential refはIDと用途・対象・必要性・状態だけを持つ。
- Conformance: 外部概念の必須射影、`権限生成=なし`、`公開範囲=metadata_only`、Trust未確定、Capability diffのoperator review要求、秘密値／権限fieldの拒否を検査する。
- Validation: `python tooling/schema_check/check_schemas.py` はSchema 75件／正常例75件／負例92件、`python tooling/conformance_tests/run_conformance_skeleton.py` はConformance 164 checks、`python tooling/日本語基底監査.py --strict`、`python tooling/validate_all.py --python-only --desktop-platform windows`、manifest、packaging portability、release gateがPASSした。Flutter／Rustの実装変更はないためFlutter解析とRust試験は未実行とした。
- 未成立分類: MCP discovery、connect、authentication、consent、Tool／Resource／Promptの実取得、Broker経由Tool実行、timeout、server unavailable、disconnect、quarantine、実MCP Test Harnessは`release_blocker`。契約射影だけでMCP接続または製品releaseを主張しない。

## D4 Pocket C9: MCP stdio接続センター（2026-09-23）

C8の契約をRust Brokerのowner control接続経路へ結合した。`MCP接続`はownerが指定した絶対executable、workspace、引数、stdio transport、Credential refだけを受け付け、Rust process boundaryから現行`server/discover`を試行する。旧`initialize`／`notifications/initialized`はlegacy protocolとして明示fallbackする。discovery後はcapabilityに従ってTool／Resource／Prompt一覧を取得し、Schema検証済みmetadata-only receiptへ射影する。

- Production path: owner control → `MCP接続` → Rust Broker → MCP stdio child → discovery／list →永続Audit付きreceipt。`MCP接続一覧`は通常IPC専用であり、接続processが終了・timeout・不整合となった場合は一覧を返さない。
- Authority boundary: MCP metadata、Tool description、Trust、Capability diff、Credential refはAuthority、Permission、Approval、Credential実値を生成しない。Credential実値注入とTool実行は未接続であり、必須Credentialを要求するServerは`mcp_credential_unavailable`で拒否する。
- Validation boundary: malformed JSON-RPC、response id mismatch、unknown tool、authority／secret field、重複metadata、未処理pagination、timeout、Server終了をRust unit test／Conformanceへ接続した。Rust processの実MCP Test Harness、Tool実行、Streamable HTTP、OAuth、consent、disconnect、quarantineは未成立として扱う。
- 未成立分類: Tool／Resource／Prompt実取得を用いた実Broker運用、Tool execution、Credential injection、Streamable HTTP、OAuth、consent、disconnect、quarantine、実MCP Test Harness、Windows installed product証拠は`release_blocker`。C9 stdio catalog接続だけでMCP全体またはD4 Pocket正式releaseを主張しない。

## D4 Pocket C6: 対話結果からの回帰Case owner登録（2026-09-23）

C5の評価Datasetと混ぜず、完了済み通常対話をownerが明示的に回帰Caseへ登録する独立Contractを追加した。Rust Brokerは要求ID/hash、`表示範囲=full`、永続結果証跡、終了監査ID、結果状態を現在の対話制御で再照合する。元の対話入力・応答本文は自動コピーせず、ownerのredacted定義を`ProtectedStore::Purpose::Regression`へ暗号化し、CLIにはhash-only receiptだけを返す。

- Contract: `regression_case_registration.schema.json`、`regression_case_receipt.schema.json`、正常例、権限field混入とraw receipt混入の負例、`docs/specs/regression-case.md`を追加した。C5の`Purpose::Evaluation`とは保管purposeを分離した。
- Production path: `回帰Case登録`はowner資格経路からBrokerへ入り、現行対話証跡と明示定義を結合してAudit確定する。通常IPC、履歴、Profile、MCP metadata、Agent metadataは登録資格または権限を生成しない。
- Validation: `python tooling/schema_check/check_schemas.py` はSchema 71件／正常例71件／負例86件、`python tooling/conformance_tests/run_conformance_skeleton.py` はConformance 162 checks、`python tooling/日本語基底監査.py --strict`、Rust全試験（lib 173件、main 4件、統合35件、合計212件）、`python tooling/validate_all.py --python-only --desktop-platform windows`、manifest、release gate、packaging portabilityをPASSした。Windows ProtectedStoreのRegression purpose分離試験もPASSした。Flutter解析・UI試験はFlutter変更がないため未実行であり、Windows installed productの実機証拠は別のrelease blockerとして保持する。
- 未成立分類: GUIのprivate登録画面、Case一覧、削除Recovery、C5 Dataset revisionへの明示import、Windows実機でのowner登録証拠は`release_blocker`。known marker拒否は秘密不存在の証明ではなく、owner redaction責任を置き換えないため`known_limitation`として保持する。

## D4 Pocket Phase 4前提: Codex CLI実物interface probe（2026-09-23）

Phase 4 Agent Launcherの前提確認として、Windowsで実際にPATHへ存在するCodex CLIのinterfaceをdevelopment-only probeから読み取り専用で確認した。`codex --version`は`codex-cli 0.155.0-alpha.16`、`codex exec --help`は`exec`、`resume`、`fork`、`review`等のsurfaceを返した。probeはversion/help以外を呼ばず、prompt、credential、workspace変更、task実行、process dispatchを行わない。

- Adapter evidence: version/helpの観測は`LIVE_RUNTIME`として記録し、Codex Adapterは`degraded`とする。Brokerのcommand dispatchが停止中のため、task executionは`unknown`、process spawnは`unsupported`である。help表示だけではsession操作の実動作を証明しないため、session control/session supportも`unknown`とする。
- Boundary: Claude、Gemini、その他のCLI/APIはこのWindows環境で未導入のため、存在・対応を推測しない。probeは秘密値を環境から子processへ渡さず、Adapter recordにも実値を保持しない。
- 未成立: Agent Launcherのproduction path、実task実行、Cancellation、Tool／MCP、Usage／Cost、Provider／Model接続、複数Agent比較、Handoffは未完成である。BrokerのApproval／Audit／Recovery統治を迂回する実行経路は追加していない。

この節はCodex CLIのinterface観測を証明するが、D4 PocketのAgent実行または完成製品releaseを証明しない。既存のrelease_blocker分類を保持する。

## D4 Pocket Phase 4 bounded projection: Agent比較・Handoff境界（2026-09-24）

実Agentの起動・書込・外部接続を推測せず、複数Agent比較とHandoffで公開できる情報のContract、conformance、Desktop投影を追加した。既存のCodex Adapter read-only経路、Rust Broker経路、rev1進捗を変更していない。

- Contract: `specs/agent_comparison.schema.json`は異なるWorkspaceの2〜8セッション、公開結果概要、bounded metric、`unknown`値を定義する。`specs/agent_handoff.schema.json`はTask／差分／試験／公開実行概要だけを渡し、Authority、Permission、Approval、Credential、hidden contextを固定拒否する。
- Conformance: 同一Workspace、同一セッション重複、Authority／Approval再利用、取得不能値の0補完、同一Agent Handoff、公開概要の秘密値をfail-closedで検査する。証拠種別は`INTERNAL_STATE`であり、実Agent比較・実Handoffの証拠ではない。
- Desktop path: Agent Centerへ、snapshotから比較可否と公開Handoff概要を読み取る表示を追加した。FlutterからAgent起動、process、filesystem、network、credential、権限付与へ到達する経路は追加していない。
- 未成立分類: 実Agentの独立Workspace起動、実結果の比較、target AgentへのHandoff・再評価・取消・失敗隔離・Recoveryは`release_blocker`。既存のwrite-capable Agent、MCP／A2A、Provider／Model、Claude／Gemini未接続の分類は変更しない。

## D4 Pocket Phase 3: Agent Adapter契約のSchema接続（2026-09-23）

Agent Adapter契約を追加した。`specs/agent_adapter.schema.json`はAgent identity、Provider、Version、Model、capability宣言、Workspace要件、Tool／MCP／Session／Cancellation／Usage／Cost対応、認証方式、Host要件を定義する。対応状態は`supported`、`unsupported`、`unknown`と理由を必須にし、認証は参照方式だけを許可してsecret実値を持たせない。

- Contract: valid exampleと`permission_id`混入のnegative fixtureを追加した。`additionalProperties=false`と認証の`secret_value_present=false`で、Agent Adapter宣言をPermission／Approval／trustへ昇格させない。
- Conformance: Agent Adapterが宣言専用であり、unsupported／unknownに理由があり、空理由が拒否されることを検査する。Schema 69件／valid example 69件／negative fixture 84件、Conformance 159 checksへ更新した。
- Production boundary: 今回は契約と開発時conformanceまでで、Vendor CLI／APIのLauncherは接続していない。実物interfaceを確認するまでunsupportedとし、認証値・process起動・network接続を推測で追加しない。

この節でAgent実行、複数Agent比較、Handoff、MCP接続、Provider／Model接続の完成を主張しない。これらはPhase 4以降の実物interface確認とBroker統治経路の検証対象であり、未成立範囲は既存のrelease_blocker分類を保持する。

## D4 Pocket Phase 2: Host Capabilityの契約接続（2026-09-23）

D4 Pocketのブランド表面を追加し、rev2 Phase 2のHost Capabilityを、Schema → Rust Broker → 認証付きIPC → Flutter読み取り専用操作面まで接続した。既存のGUI-Shell技術契約、Shell Coreの権限境界、rev1の進捗履歴は変更していない。

- Contract: `specs/host_capability.schema.json`、valid example、`Permission`混入のnegative fixtureを追加した。能力の状態と証拠種別（`CONFIG`、`INTERNAL_STATE`、`LIVE_RUNTIME`、`EXTERNAL_EVIDENCE`、`FIXTURE`）を分離し、Host Capability自身にPermission／Approval fieldを許可しない。
- Rust production path: Broker operation `ホスト能力`を既存の認証付きIPCへ追加した。payloadはnullだけを受け付け、非null payloadを拒否する。Brokerが観測できる範囲だけを返し、観測結果から権限を生成しない。
- Desktop product path: `ShellCoreClient.product()`がhealth後にHost Capabilityを取得し、`D4 Pocket ホスト能力`の読み取り専用画面へ投影する。UIはRust Brokerを呼び出すが、Permission、Approval、filesystem、process、network、credentialを直接扱わない。
- Evidence: Schema 68件／valid example 68件／negative fixture 83件、Conformance 158 checks、Host Capability unit test、Rust全test 204件（Broker IPC 9件を含む）、Desktop analyze／全76 tests、共有UI analyze／39 tests、Mobile analyze／29 testsがこの作業単位でPASSした。`Permission`混入と不正payloadの拒否も実行した。Python-only集約検証、manifest、release gate、packaging portability、release smoke、evidence bundle、release runtime assertions、日本語基底監査もPASSした。

この節でrev2全体の完成を主張しない。Agent Adapter／Launcher／Dashboard／Compare／Handoff、Provider／Model、MCP／A2A、Multi-host、Compose／Preview／Export／Pruning、長時間運用、Windows installed productの全数証拠は後続作業または既存release gateに残る。既存のrelease_blockerをpost_v1_scopeへ読み替えない。

## D4 Pocket 第4段階 エージェント接続表示盤（2026-09-24）

起動時に実物Codex CLIのversion／help interfaceを確認済みのAdapterだけを、Rust Brokerの認証済み`Agent一覧`からmetadata-onlyでDesktopへ投影する経路を追加した。既存の`実行系列挙`とAgent Adapterを混同せず、通常Runtime AdapterはAgent権限へ昇格させない。

- Production path: owner起動設定 → Rust `CodexCliAdapter::new` → version／help probe → `Agent一覧` → 認証済みDesktop IPC → Agent CenterのProvider／Model／状態／証拠種別／能力表示。
- Evidence boundary: interface確認は`LIVE_RUNTIME`だが、Codex Adapter状態は`degraded`で固定し、task execution、session control、Tool、MCP、Cancellation、Usage、Cost、process spawnはunknownまたはunsupportedとする。Credential実値、executable path、Workspace pathはAgent一覧receiptへ返さない。
- Validation: Rust全試験でAgent metadataのread-only、秘密値非保持、process spawn停止を確認し、Desktop Agent Centerは認証Broker応答が返すAgent Adapter一覧だけを描画する。
- 未成立分類: write-capable Agent execution、Agentの選択・実task・取消、Claude／Gemini接続、複数Agentの実比較、実Handoff、MCP／A2Aは`release_blocker`。Agent Adapter Dashboardのmetadata表示だけでAgent製品機能の完成を主張しない。

## 現況：Windows実機を基準とした開発継続（2026-09-13）

owner指示により、現在実機検証できるOSはWindowsだけとする。Android実機は凍結、その他の非Windows実機も未検証として保持する。実装・build・自動試験・利用可能な仮想環境の検証を先に進め、実機の不在だけを開発停止条件にしない。実運用監査署名は正式release直前まで延期する。具体的な運用はROADMAPの「現在の開発・検証条件」を参照する。

次の保存済み結果を今回読み直した。対象sourceは `7d4766d0969a335e57bcba997540851c2e25aa6c` であり、この文書更新後の新commitの実行証拠へ読み替えない。

- Windows: `%LOCALAPPDATA%/GUI-Shell/installed-runs/rev2-7d4766d-20260911/runtime/evidence/validation-result.json` では、provenance分離、installed初回起動、Setup Doctor、brokerの4関門がpassed。実運用署名の関門はfailed。末尾に残る「正式Windows再収集が必要」という履歴項目は、このsourceの通常collectorによる収集で解消した。
- Apple: Repository外の `GUI-Shell-apple-7d4766d/verified-result.json` では手動run `34560858168`、artifact `10184321688` がpassed。Mac上Rust試験64単体・5 IPC・7 checkpoint、macOS開発appとiOS Simulator appのbuildを確認した記録である。tarのSHA-256は `370851b078b74b266deadce4c4a08ffeba4ba0ef2381a1d84f06768111cb2da9`。実機起動・Keychain・正式配布を証明しない。

これらは保存済み検証結果の確認であり、今回の文書修正で製品を再実行した結果ではない。履歴を消して過去の判断を隠さず、対象sourceと証拠範囲を固定して更新する。

- item: 非Windows実機証拠・Mobile正式配布・実運用監査署名・owner GO
  classification: release_blocker
  reason: 実機はWindowsのみ利用可能で、Android実機は凍結中。運用署名は正式release直前まで延期、正式配布条件とowner GOも未成立。利用可能な環境での開発継続を妨げる条件ではない。
  required_action: 現在は実装と利用可能な検証を継続する。対象環境の提供またはownerの再開指示後に該当証拠を収集し、正式release時に署名と配布条件を確定してstrict検証後にowner GOを得る。
  blocks_release: yes

## Android仮想端末とApple補助結果の確定（2026-09-13）

source `9c4d89efcb5dedf2b5d229dcd9fd4ef0b2b47bc7` の手動Android run `34743675304` とApple run `34743676639` がsuccessで終了し、取得成果物のenvironmentとsourceを照合した。双方とも追跡差分は空、未追跡表示は当該jobのevidence directoryだけである。Androidはnative保管2件とMobile29件・解析がPASS。実TLS統合でnative再読取・controller再生成・OS背景停止と復帰再接続・二つの実MINIDORA応答・失効資格拒否がPASSした。従来のAVD容量不足とCRLF応答による検証未到達はこのsourceで解消した。

AppleはRust74単体・5 IPC・7 checkpoint・2差分・2取得器統合、共有18・Desktop33・Mobile29、native保管2件、実TLS統合とmacOS/iOS Simulator buildがPASS。今回追加したBroker読取制御より前のsourceであり、その新実装のApple証拠へ読み替えない。Apple tar SHA-256は `09d6fb2677ecf68fcbf4e43107522827cc360c03937a245195f818b6dada20f8`、Android成果物各fileのhash集合のSHA-256は `36879e4d1703196033383ae1ef7f41c94ec7a82553a04210a5570ba4af913d6e`。Repository外のGUI-Shell-apple-9c4d89eとGUI-Shell-android-emulator-9c4d89eへ原成果物とverified-result.jsonを保存した。physical_device_verified=falseを維持し、実機・正式署名配布・owner GOはrelease_blockerのままとする。

## Android補助hostの容量と終了待機を修正（2026-09-13）

9a60cdd/run `34742562494` は有効configでuserdata=2Gとなっていたが、emulatorの要求は7372.80MBのままで、空き6986.07MBに対して再び起動前FATALとなった。設定変更が実容量を減らしたとは言えず、2G指定を除去する。元のAPI35 imageに必要な領域を確保する。SDK記録にはNDK27.3・28.2・29.0が同居し、固定Flutter3.44.0のFlutterExtension.ktとapp設定の消費は28.2.13676358である。隔離された手動runnerだけでSDK managerのuninstallにより未使用の27.3.13750724と29.0.14206865を除き、28.2の存在を検査する。対象版以外を計算して削除しない。変更前後のSDK一覧と工具結果、AVD有効設定、空き容量を保存する。ローカルhost・ownerのfile・製品依存を削除せず、必要NDKが変わるときはこの固定照合を再検討する。

先行c86da93/run `34741135813` はcancelledで終了したが、artifactのemulator.txtにboot 61356ms、avd-name.txtに正しいAVD名とOKのCRLF応答が残り、Flutter試験logは存在しなかった。既存grep -Fxは末尾CRを含む値と一致せず、cleanupのwaitには期限がなかった。AVD名は外部consoleの行末CRだけを除去してから完全一致を要求し、ADBの観測にも10秒期限を付ける。cleanupはこのstepが起動したPIDだけへ終了を要求し、10秒後も残る場合はKILLして回収する。boot条件・仮想属性・名前・native試験を省略しない。未成立のAndroid統合はrelease_blockerとして維持する。

Schema37/正常37/負例39、conformance151件、厳格日本語監査、YAML読取とworkflow6 stepのbash構文検査はPASS。隔離fixtureでCRLF応答の正常一致・別名拒否、TERMを無視する自分の子processがcleanupで15秒以内に回収されることを実行確認した。SDK削除はローカルでは実行せず、修正後の手動runnerで結果と容量を確認する。これらの局所検証をAndroid native統合成功へ昇格しない。

## Android専用AVDのuserdata容量を明示（2026-09-13）

3431173/run `34741774416` はemulator起動前に失敗し、Flutter native試験には未到達だった。artifactのemulator.txtで、専用AVD directoryの空き6987.37MBに対しuserdata作成が7372.80MBを要求したFATALを確認した。KVM・AVD存在・system image検出は成立しており、前回のAVD未検出とは異なる容量不足である。Repository外のGUI-Shell-android-emulator-3431173へ原logを保存した。

専用AVDの標準設定disk.dataPartition.sizeを2Gに明示する。[Android公式の仮想端末設定例](https://android.googlesource.com/platform/external/adt-infra/+/refs/heads/emu-master-dev/emu-image/templates/avd/Pixel2.avd/config.ini)にもある通常設定で、製品runtimeの代替実装や容量検査の抑止ではない。この短時間のnative保管・実TLS試験用AVDに限る。作成時設定と有効設定、起動前の空き容量を証拠へ保存し、emulator側の容量検査、boot、仮想属性、実試験の判定は維持する。既存fileやSDKを削除せず、実機凍結にも変更はない。大量data試験を追加するときは容量と取得証拠の範囲を再検討する。2Gでの実起動と試験は修正後の手動runで確認し、未成立の間はAndroid仮想端末統合をrelease_blockerに保持する。

Schema37/正常37/負例39、conformance151件、厳格日本語監査、workflow6 stepのbash構文検査はPASS。設定更新部分は一時fixtureで正常更新・key欠落拒否・重複拒否を実行し、対象key以外が変わらないことと拒否時にfileを変更しないことを確認した。これらはAVD実起動の証拠ではない。

## Apple実TLS統合の証拠確定とAndroidへの接続（2026-09-13）

fd1b23e/run `34740828826` は手動Mac job全体が成功した。保存したartifactのtar SHA-256は `3f62ce784508b8dcfdd0ecb3bd3fba86b5e287570309ca6f7b6fea9f7f94e9b9` と一致し、追跡差分は空。共有18・Desktop33・Mobile29試験と解析、native保管2試験、macOS/iOS Simulator buildを確認した。実TLS統合結果はnative再読取、controller再生成、OS背景時停止と復帰再接続、二つの実MINIDORA応答、失効後拒否がPASS。Repository外のGUI-Shell-apple-fd1b23e/verified-result.jsonへ対象sourceと範囲を保存した。過去のMac起動期限超過はこのsourceで解消した。physical_device_verified=falseであり、正式配布・実機の保護特性・owner GOは証明しない。

同じdevelopment専用統合をAndroidへ接続する。Linux hostとemulator-5554を固定し、harnessとdriverの双方がro.kernel.qemu=1と専用AVD名gui_shell_native_testを確認してから進む。端末列挙や実機操作は行わない。OSのHome遷移と明示MainActivityの再前景化を使い、製品のlifecycle observerで停止・復帰を観測する。接続先には[公式のhost loopback alias](https://developer.android.com/studio/run/emulator-networking-address)である10.0.2.2を一時招待生成時に指定する。brokerは127.0.0.1へbindしたまま、証明書hash照合・認証・Approval・Auditを通す。製品network境界の変更、平文化、証明書検証の省略はない。接続情報は一時fileと認証付きdebug VMで渡し、成果物へ資格を保存しない。

変更後のMobile `flutter analyze --no-pub` と `flutter test --no-pub --reporter expanded` は29件PASS。Schema36/正常36/負例38、conformance150件、厳格日本語監査、手動workflowの6 stepのbash構文検査がPASS。既存Windows実API・実TLS・両Dart client・監査再読取は同じharnessの全client指定でPASSし、Repository外のGUI-Shell-android-native-harness-regression.txtに保存した。host不一致・実機serial・非仮想属性・異AVD名の拒否4件は呼出を差し替えたFIXTUREとして検証し、実機操作を行っていない。新Android経路の実行証拠とは区別する。

- item: Android仮想端末のnative保管・実TLS・OS復帰の実行結果
  classification: release_blocker
  reason: この追加経路はローカル解析だけでは仮想端末で成立したと判断できない。
  required_action: 対象commitを手動runnerで実行し、専用AVD属性・実結果・対象sourceを照合する。非Windows実機の延期は維持する。
  blocks_release: yes

## Android workflowの変数評価位置を修正（2026-09-13）

fd1b23eのAndroid手動dispatchはHTTP422で拒否された。jobのenvでrunner.tempを参照した位置ではrunner contextを使用できず、GitHubのworkflow検査を通らなかった。YAML・bashの構文検査はこのGitHub固有のcontext制約を検証していなかった。ANDROID_AVD_HOMEの値は、実行開始stepでRUNNER_TEMPを使ってGITHUB_ENVへ設定し、後続の作成・起動stepへ同じ値を渡す。起動条件、保存先の境界、AVD存在検査、仮想端末属性確認を変更しない。手動dispatchと実行結果は修正後commitで確認する。未実行のAndroid native保管は上記release_blockerとして保持する。Schema36/正常36/負例38、conformance150件、厳格日本語監査は修正後もPASS。

## 仮想端末検証hostの起動前提を修正（2026-09-13）

Macのcf5c771/run `34739605831` attempt2は、参照のimportが0.096秒以内に終わり、HTTP bind中の `socket.getfqdn` で10秒後も待機していた。stackはPython3.14のHTTPServer.server_bindからの逆引きを示した。attempt1のGitHub DNS取得失敗は別の環境失敗として保持する。Windowsの同じ経路は約0.47秒／0.38秒で起動している。

このharnessはliteralな127.0.0.1だけへbindし、参照のAPIHandler・製品チャットはserver_nameを権限や接続先として利用しない。通常constructorには逆引きを省略する指定がないため、stdlibのbind_and_activate=False、TCPServer.server_bind、server_activateを使用し、表示用server_nameも実bind先の127.0.0.1へ固定する。試験の期限、MINIDORA本体、API/trace、Rust broker、実送信経路は変えない。これは全host共通のdevelopment限定初期化であり、通常製品runtimeへのwrapperではない。DNS名による参照server提供を追加する場合はこの前提を再検討する。

Androidのd952991/run `34739883443` はKVM利用可能とSDK準備を確認したが、emulatorがAVD名を見つけられず終了し、ADB待機が期限切れとなった。標準のANDROID_AVD_HOMEをjob内の専用directoryへ固定し、avdmanagerの作成path、iniの存在、emulatorの列挙を照合してから起動する。起動成功を推定せず、端末属性・AVD名・native試験の既存判定も維持する。環境変数の定義は[Android公式資料](https://developer.android.com/tools/variables)を参照した。 初回の圧縮容量2.20GBはpreview channelのemulator 37.2.8を拾っていたため補正する。安定版37.1.11は441,926,448 bytes、API35 imageとの合計は2,180,742,351 bytes（約2.18GB）。観測時空き容量約2.19GBとほぼ同量で、別途必要な展開領域の余裕がないという判断は保持する。

逆引きを必ず例外にする故障注入で、harness内の実server起動コードを実行し、固定MINIDORAのcapabilities APIがHTTP200を返すことを確認した。証拠はRepository外のGUI-Shell-no-reverse-dns-proof.json。Windowsの実API・実TLS・両Dart client・監査chain再読取もPASSし、GUI-Shell-loopback-bind-regression.txtへ保存した。Schema36/正常36/負例38、conformance150件、日本語監査、Android workflowの5 step構文検査もPASS。

- item: 修正後のMac実TLS統合とAndroid native保管
  classification: release_blocker
  reason: 原因に対応した検証環境修正であり、修正後の手動runが成功した証拠はまだない。
  required_action: ローカル回帰後に両手動runを実行し、対象commitと実結果を確認する。
  blocks_release: yes

## Android仮想端末の手動補助経路（2026-09-13）

Windows SDK managerの一覧にemulator 37.1.11とAPI35 Google APIs x86_64 revision9を確認した。SDK catalogで確認した圧縮imageは1,738,815,903 bytes、Windows emulator候補は459,029,121 bytes。空き容量約2.19GBのhostでは展開余地がなく、ローカル導入を強行しない。中間生成物削除の自動承認拒否を迂回せず、許可された手動補助環境で仮想端末検証を進める。

Ubuntu 24.04 runner上で専用AVDを作成し、Mobile解析・29既存試験とnative安全保管2試験を実行するworkflowを追加する。triggerは手動限定、contentsはread、Flutter/actionは既存と同じ固定点、証拠保管3日。ADB対象を明示作成のemulatorに固定し、qemu属性とAVD名を照合する。KVMの利用者限定権限設定は隔離されたdevelopment hostのための設定であり、製品Permissionへ転用しない。SDK licenseの自動承認は追加せず、既存runnerで不足する場合は失敗として保持する。 ローカルではSchema36/正常36/負例38、conformance150件、厳格日本語監査、5つのrun stepの `bash -n` がPASS。これらはCONFIG・静的検査であり仮想端末の実行証拠ではない。

参考一次資料: [Android Emulatorの加速方式](https://developer.android.com/studio/run/emulator-acceleration)、[Ubuntu 24.04 runner image](https://github.com/actions/runner-images/blob/main/images/ubuntu/Ubuntu2404-Readme.md)。これらの記載を実行証拠にせず、使用toolchainと起動結果はrunごとに記録する。

- item: Android仮想端末のnative保管実行証拠
  classification: release_blocker
  reason: workflow追加だけでは仮想端末の起動・native API成功を証明しない。
  required_action: push後の手動runで実行し、対象commitと結果を照合する。Android実機凍結は維持する。
  blocks_release: yes

## Mac上の参照Runtime起動期限超過を診断（2026-09-13）

c5aa1ecの手動run `34739077800` はnative保管2試験がPASSした後、最初のMINIDORA参照APIの起動確認で20秒期限を超過した。Simulator実TLS試験は未到達でありPASSにしない。Windowsの同じ参照・harnessの実接続は成功しているが、Mac環境の根本原因を確定したものではない。

開発専用server起動コードへ、import、HTTP bind、製品初期化、readyの経過秒記録と10秒時点のstack採取を追加する。期限、参照製品、実API、権限経路を変更せず、待機箇所を観測する。失敗時は段階名・経過秒と起動前stackだけを出力し、資格や要求本文、localsを出力しない。通常製品runtimeには追加しない。Windowsの実API二実行系・両Dart client・実TLS・監査chain再読取は計測追加後もPASS。Schema36/正常36/負例38、conformance150件、厳格日本語監査もPASS。ログはRepository外のGUI-Shell-runtime-startup-diagnostics-regression.txt。

- item: Mac上の参照Runtime起動とSimulator実TLS統合
  classification: release_blocker
  reason: 起動期限超過の原因は未確定で、実TLS統合へ到達していない。
  required_action: 計測結果から原因を特定し、根拠のある修正後に同じ統合試験を再実行する。
  blocks_release: yes

## native保管・実TLS・実API・OS復帰の統合試験（2026-09-13）

先行d9f23cdの手動run `34738442843` は、Simulator上のnative安全保管2試験を含め成功した。これを実TLSやOS lifecycleの証拠へ読み替えず、次の統合試験を追加する。

既存minidora_live_checkへMac専用 `--mobile-simulator <UDID>` を追加した。固定3400a3bの実API二実行系と一時Rust brokerを使い、試験招待だけを認証付きFlutter debug VMからintegration_testへ渡す。native保管後のcontroller再生成、証明書不一致の拒否、製品MobileHomeへのOS背景・復帰通知、左右の実応答、保存資格消失時の停止、端末離脱とnative削除・失効後拒否を検査する。追加経路はdevelopment専用で、運用資格・実鍵・製品runtimeを変更しない。試験の失敗をAPI成功へ昇格せず、期限とprocess cleanupを既存harnessへ接続する。

Windows上の `flutter analyze --no-pub` とMobile29試験はPASS。変更後の `python tooling/minidora_live_check.py --reference C:/Users/mzcum/codex-work/MINIDORA-reference --binary C:/Users/mzcum/codex-work/GUI-Shell/native/rust_helper/target/release/gui_shell_rust_helper.exe --dart-client --mobile-client --dart-mobile-client` はPASSし、既存の実API・実TLS・両Dart client・監査chain再読取の回帰なしを確認した。ログはRepository外のGUI-Shell-native-integration-harness-regression.txt。新Simulator経路の実行証拠はpush後の手動runnerで別途確認する。

- item: Simulatorでのnative実TLS統合の実行証拠
  classification: release_blocker
  reason: Windows解析・試験はMac上の追加統合経路を実行していない。
  required_action: 対象commitの手動runで統合経路を実行し、実結果と保管済み成果物を確認する。実機限定事項は延期を維持する。
  blocks_release: yes

## Mobile native安全保管の仮想端末試験経路（2026-09-13）

SDK付属integration_testをdevelopment依存に追加し、実SecureDeviceStoreを使う2試験を作成した。書込・別instance読取・更新・削除、端末IDの再読取、資格なし・破損資格の通信停止を検査する。試験キーはprefixで分離し、既存製品資格を読まない。前景切替はcontroller入力でありOS lifecycleではない。手動Apple補助は利用可能なiPhone Simulatorを選択・起動し、試験後に終了する。契約・本番権限・平文fallbackは変更しない。

変更前b07dba3の手動run `34738095886` は共有18・Desktop33・Mobile29試験と解析、Rust64単体・5 IPC・7 checkpoint、macOS/iOS Simulator buildが成功した。ログはRepository外のGUI-Shell-apple-b07dba3-log.txt。今回追加したnative試験の結果とは区別する。成果物の最初の取得とローカルMobile試験はディスク容量不足で失敗した。処理終了後の空き容量回復を確認し、Mobile試験を再実行して29件PASSを確認した。追加試験を含む `flutter analyze --no-pub`、Schema36/正常36/負例38、conformance150件、厳格日本語監査もPASS。再生成物の削除は自動承認レビューで拒否され、迂回していない。

- item: native安全保管のSimulator実行と実接続・OS lifecycle
  classification: release_blocker
  reason: native試験の実行はpush後の手動runnerで確認する必要があり、controller入力だけではOS lifecycleやTLS再接続を証明できない。
  required_action: Simulator上の実行結果を収集し、次に実TLS接続とOS lifecycleの試験を接続する。非Windows実機は指定どおり延期する。
  blocks_release: yes

## Apple手動補助へFlutter解析・試験を追加（2026-09-13）

手動workflowはRust試験とapp buildのみで、共有UI・Desktop・MobileのFlutter試験を実行していなかった。各packageの解析と試験を追加し、依存解決・解析・試験のログを対象commitの補助成果物へ保存する。pipefailにより解析・試験の失敗をbuild成功で隠さない。固定toolchain・手動起動限定・read権限・保管期間・追跡差分拒否は保持する。これはdevelopment専用経路であり、製品の権限や外部送信経路を変更しない。

変更前46f384fで `python tooling/validate_all.py --desktop-platform windows --include-mobile-release` は正常終了し、共有18・Desktop33・Mobile29試験を確認した。ログはRepository外のGUI-Shell-extension-c0-baseline.txt。Mac上の実行結果はこの変更をpush後に手動実行して対象commitと結合して確認する。

- item: Mobile native保管・lifecycle・再接続の仮想環境統合検証
  classification: release_blocker
  reason: この追加はMac host上のFlutter試験であり、SimulatorやAndroidエミュレータのnative実行証拠ではない。
  required_action: 実機凍結を維持し、利用可能な仮想環境でnative統合試験を実装・実行する。
  blocks_release: yes

## 比較画面の応答照会を左右独立に実行（2026-09-13）

共有Widgetの応答照会が左右直列で、左の通信待機中は右の完了応答を表示できないことを再現した。照会中フラグを左右別に持ち、各側の周期照会を独立させる。同じ側への重複照会を防ぎ、旧要求の遅延応答・失敗を現在表示へ転用しない条件は維持する。変更はUIの表示取得経路で、Rustの権限・Approval・監査・外部送信は変更しない。

左右それぞれを待機させる製品Widget試験で、他側の先行表示、待機中の重複照会なし、待機解除後の両結果表示を確認した。`python tooling/validate_all.py --desktop-platform windows --include-mobile-release` は開発検証17項目PASS。共有18・Desktop33・Mobile29試験、3か所の解析、Schema36/正常36/負例38、conformance150件、Rust63単体・5 IPC・8 checkpoint、厳格日本語監査を含む。ログはRepository外のGUI-Shell-independent-poll-validation.txt。埋込installed証拠は変更前b84bbbdのものであり、この修正後の実機証拠ではない。証拠は通信を遅延させたFIXTUREであり、実機の通信遅延測定ではない。非Windows実機・正式配布・運用署名・owner GOの延期は冒頭のrelease_blockerに保持する。

## 実行系列挙の重複による選択欄の不整合を拒否（2026-09-13）

共有clientは実行系IDの型・形式・件数だけを検証しており、重複を受理していた。leftを2件返す応答でclientの誤受理とFlutter Dropdownのassertionを別々に再現した。実行系列挙の一意性をclientで検証し、重複応答は既存の接続エラー表示と送信停止へ接続する。重複を黙って除去せず、正常な返却順序と空一覧を保持する。Rustの登録・権限・列挙内容は変更しない。

共有packageの `flutter test --no-pub --reporter expanded` は16件PASS。隣接・非隣接の重複拒否、正常順序、空一覧、製品Widgetの接続エラーと送信無効を確認した。共有・Desktop・Mobileの `flutter analyze --no-pub` はPASS。証拠は不正応答を注入した製品client／WidgetのFIXTUREである。非Windows実機・正式配布・運用署名・owner GOは冒頭のrelease_blockerとして延期を保持する。

## 対話結果の参照・能力配列を検証後に固定（2026-09-13）

DialogueResultの外側のMapは変更不可だったが、参照・能力のListは受信元と共有していた。検証後に元の空配列へ要素を追加するとnoneの表示結果にも現れ、公開getterからも配列を変更できることを2試験で再現した。検証通過後に両配列をコピーして変更不可にし、文字列とMapだけでなく結果全体の検証済み内容を固定する。新しい公開内容には再検証を要求し、表示資格の判定を増やさない。

共有の `flutter test --no-pub --reporter expanded` は14件PASS。none/fullで元配列の変更が伝播しないこと、getterとfieldsの両経路からの変更が拒否されること、既存の公開参照・能力が保持されることを確認した。共有・Desktop・Mobileの `flutter analyze --no-pub`、Schema36/正常36/負例38、conformance150件もPASS。証拠は製品parserとWidgetを実行するFIXTUREである。broker・TLS・実機保管は変更していない。非Windows実機・正式配布・運用署名・owner GOは冒頭のrelease_blockerとして延期を保持する。

## 中止進捗と内包結果の状態を照合（2026-09-13）

共有対話clientで、進捗が中止なのに結果が成功で本文を持つ応答を受理することを再現した。Rustの中止処理は中止結果を返すため、clientにも進捗と結果の対応検証を追加した。中止進捗に成功・保留・失敗が混在する3負例を拒否し、正しい中止結果を受理する。Rustの採否・中止処理や表示資格を変更しない。

共有packageの `flutter test --no-pub --reporter expanded` は12件PASS。共有・Desktop・Mobileの `flutter analyze --no-pub`、Schema36/正常36/負例38、conformance150件、日本語基底監査はPASS。変更中の作業ツリーで `python tooling/minidora_live_check.py --reference C:/Users/mzcum/codex-work/MINIDORA-reference --binary C:/Users/mzcum/codex-work/GUI-Shell/native/rust_helper/target/release/gui_shell_rust_helper.exe --dart-client --mobile-client --dart-mobile-client` もPASSした。固定参照3400a3bの実API二実行系、owner CLI承認、通常資格拒否、表示分離、trace、保留、片側失敗、両失敗、監査chain再読取、両製品Dart clientとTLS経路を確認した。ログはRepository外のGUI-Shell-dialogue-cancel-consistency-live.txt。

不正状態の注入はFIXTURE、通常通信の回帰はLIVE_RUNTIMEである。Mobile実機安全保管・OS lifecycleの証拠ではない。非Windows実機・正式配布・運用署名・owner GOは冒頭のrelease_blockerとして延期を保持する。

## セッション切替の部分失敗と旧結果表示（2026-09-13）

旧セッションの終了成功後に新規開始が失敗すると、sessionだけが空になり旧要求・応答・完了状態が残る不具合を製品Widgetで再現した。旧セッションの終了成功時点で現在表示を未開始へ戻し、旧要求と応答を外す。終了自体が失敗した場合は旧sessionと結果を維持し、終了成功を推定しない。監査記録やbrokerのsession管理は変更しない。

開始失敗・終了失敗の2試験を追加した。変更前は開始失敗の旧応答消去がFAIL、終了失敗時の保持はPASS。変更後は両方PASSで、その後の手動再試行による新規開始も確認した。共有・Desktop・Mobileの `flutter test --no-pub --reporter expanded` は11・33・29件PASS、3か所の `flutter analyze --no-pub` もPASS。Schema36/正常36/負例38、conformance150件を確認した。

証拠範囲は製品Widgetを実行するFIXTUREである。新しい実機操作・ビルド証拠を今回作ったとは主張しない。非Windows実機・正式配布・運用署名・owner GOは冒頭のrelease_blockerとして延期を保持する。

## 共有対話画面の遅延失敗を元の要求へ限定（2026-09-13）

応答照会中に中止して新規セッションへ切り替えると、旧要求の遅延通信失敗が新しいセッションのerrorを上書きすることをWidget試験で再現した。成功応答は既に要求IDとpendingを検査していたが、catch側には同じ結合確認がなかった。例外の表示にも現在の要求ID一致と待機中の条件を要求した。経路はDesktop/Mobile共有UIの表示処理で、brokerの採否・取消・監査は変更しない。

初回の再現試験は遅延を設定する前に周期照会が完了し、取消状態の前提を満たせなかった。送信前に応答の待機を設定して順序を固定すると、修正前は遅延成功の除外がPASS、遅延失敗の除外がFAILとなった。修正後は両方PASSで、現在待機中の要求の失敗は表示する対照試験もPASSした。これは製品Widgetを実行するFIXTUREであり、実ネットワーク障害を起こした実機証拠ではない。

共有package・Desktop・Mobileで `flutter test --no-pub --reporter expanded` がそれぞれ9・33・29件PASS、3か所の `flutter analyze --no-pub` もPASS。`python tooling/schema_check/check_schemas.py`（Schema36/正常36/負例38）、`python tooling/conformance_tests/run_conformance_skeleton.py`（150件）、`python tooling/日本語基底監査.py --strict` はPASS。非Windows実機と実運用署名の延期は冒頭のrelease_blockerに保持する。

## Mobile復帰時の安全保管の再確認（2026-09-13）

仕様が求める復帰時の資格読取に対し、製品controllerはメモリ上の資格だけで端末確認を再開していた。保存資格の削除・破損・秘密変更・Host変更・期限変更・端末ID変更・読取障害の7負例すべてで、変更前に接続再開を再現した。

起動・復帰・手動再確認の共通経路で端末IDと保存資格を再読取し、構造・期限・現在の結合内容の一致を確認してから通信する。保存値から別資格を自動採用せず、不一致や読取障害では停止する。非同期の保管読取・端末確認・実行系列挙の間にbackgroundへ移った結果を後の復帰へ転用しないよう、前面状態の世代を照合する。読取中のbackground移行では通信せず、その後の復帰で再確認できる試験も追加した。

`flutter test --no-pub --reporter expanded` はMobile全29件PASS。Desktop/Mobileの `flutter analyze --no-pub`、`python tooling/schema_check/check_schemas.py`（Schema36/正常36/負例38）、`python tooling/conformance_tests/run_conformance_skeleton.py`（150件）、`python tooling/日本語基底監査.py --strict` はPASS。保管と通信の障害注入はFIXTUREで、製品controllerの復帰制御を検証した範囲である。非Windows実機の安全保管・OS lifecycleは冒頭のrelease_blockerとして延期を保持する。

同じ変更中の作業ツリーからWindows上で `flutter build apk --debug --no-pub` が成功した（Gradle 187.1秒）。生成APKは175742389 byte、SHA-256 `6fe9a83c2ab31a80e47729f0c02daab7a0ba1aa00ef5984b54b052ed8713cc66`。debug buildの成立範囲であり、実機install・起動・正式配布の証拠ではない。artifactはignoreされたbuild領域に保持する。

## Mobileの解除・保存資格削除の確認（2026-09-13）

Mobileのcontrol経路で、保存APIのdeleteが例外なく戻るだけで削除完了と表示していた。通常解除と端末内だけの削除の両方で、資格残存・削除後の読取障害を注入した4負例が変更前に失敗した。またclient不在でdisconnectを直接呼ぶと、端末離脱を送らずDesktop解除成功と表示する負例も再現した。既存UIはclientを作れない破損資格で通常解除を無効にするが、controller自身にも拒否を置いた。

通常解除には端末離脱の確認を要求し、保存資格はdelete後に同じDeviceStoreから再読取して不在を確認する。失敗時は資格の管理状態を保持して通信を止め、未確認と表示する。通常解除・端末内削除の正常2例と負例5例を追加し、製品controllerを呼ぶMobile試験は全21件PASS。Desktop/Mobileの `flutter analyze --no-pub`、Schema36/正常36/負例38、conformance150件、日本語基底監査もPASS。Mobile試験commandは `flutter test --no-pub --reporter expanded`。

試験の保管・通信は障害を注入するFIXTUREで、productionのDeviceLinkControllerの採否・表示・通信停止を検証した。Rustの失効・監査経路、TLS、保管pluginは変更していない。Android/iOSの実機安全保管の証拠へは昇格しない。実機未検証は冒頭のrelease_blockerに保持し、現在の開発を継続する。

## 統治変更（2026-09-10）

対象はローカル品質判定と手動補助 Actions の分離。製品 runtime の権限・実行経路は変更しない。自動 CI は禁止を維持し、手動起動条件を構造として検査する。証拠分類は CONFIG / FIXTURE であり、外部実行や branch protection の保証ではない。

`python tooling/schema_check/check_schemas.py`、`python tooling/conformance_tests/run_conformance_skeleton.py`、`python tooling/日本語基底監査.py --strict` は PASS。Conformance は142項目。手動起動の文字列・列挙・対応形式、不在、実ファイル読取りを確認し、自動起動の混在、重複鍵、不正 YAML、独自タグを拒否した。

`PYTHONUTF8=1` を設定した Windows の `python tooling/validate_all.py --desktop-platform=windows` は FAIL。既存の `packaging_portability_check` だけが失敗し、他の13検査は PASS。Rust は42単体＋4統合、Flutter は31試験、desktop/mobile analyze と broker parity も PASS。日本語4ファイルの展開名不整合は変更前から存在し、独立した最小 ZIP でも再現した。UTF-8 locale 指定でも改善しない。統治変更に起因する失敗ではない。

WSL/Linux の `python3 tooling/validate_all.py --python-only --desktop-platform=linux` は9検査すべて PASS。これは Python 側の検証であり、reporter が併記する過去の Linux build / launch 記録を今回の実行証拠とは扱わない。Windows の環境依存の失敗は以下へ分離して保持する。

- item: Windows の ZIP 展開名不整合
  classification: release_blocker
  reason: Git for Windows 同梱 unzip で日本語名が変化する環境依存の失敗。統治変更で検査を除外しない。
  required_action: 次の Baseline 単位で標準展開機構を確認し、同一の manifest / conformance / release gate 検査を維持して修正する。
  blocks_release: yes

- item: installed-path 証拠・実機検証・owner GO
  classification: release_blocker
  reason: 統治単位は製品の完成証拠を作成しない。strict release は未実行である。
  required_action: 製品単位で正式な実機証拠と明示承認を揃える。
  blocks_release: yes

後続の実行系対話、MINIDORA Adapter、比較、Mobile、端末連携、各 platform の実装と検証は未完了。今回の統治単位の完成をそれらの完成へ昇格しない。これらの owner 指示内の未実装は rev2 完成に対する release_blocker として次単位以降で扱う。GitHub Actions は未使用。

## Baseline の Windows 配布検証修正

Git for Windows 同梱 unzip 6.00 は最小 ZIP の日本語名も文字化けさせた。`LC_ALL=C.UTF-8` と `-UU` でも再現した。一方、Windows 標準 tar.exe（bsdtar 3.8.8）は `LC_ALL=C` のまま同じ ZIP の名前を保持した。

検証専用経路を Windows は System32/tar.exe、POSIX は従来の unzip に分けた。独自 wrapper、代替の製品 runtime、検査除外は追加しない。展開後の manifest、conformance、release gate 検査は維持する。適用範囲は dev / release validation の source ZIP 展開のみである。新しい日本語名・空白を含むパスの実展開、内容hash一致、破損ZIPの拒否、展開器不在を試験する。これは当該 OS の外部展開器に対する EXTERNAL_EVIDENCE であり installed 製品証拠ではない。

本修正は Windows 標準機構の恒久的な選択である。将来対応OSの標準展開器が変わる場合は、同じ名前・内容・失敗の試験と実展開後検査を成立させて選択を見直す。

検証結果: Windows の `python tooling/validate_all.py --desktop-platform=windows` は14検査すべて PASS（終了値0）。WSL/Linux の `python3 tooling/validate_all.py --python-only --desktop-platform=linux` は9検査すべて PASS。Conformance は143項目。上記の Windows ZIP 展開名不整合はこの修正で解消した。installed-path 証拠と owner GO は未解消の release_blocker のままであり、開発用集約結果の pass を製品 release の許可にしない。

## 日本語意味正本・契約・Conformance

`docs/specs/runtime-dialogue.md` で要求・応答・セッション・比較、取消の限界、表示境界、権限と監査の前提を定義した。MINIDORA API の実コードは commit `3400a3bb68b37efa1dc14ee8aaa28fda779bf1f8` へ固定した。参照系のコードは変更しない。

四つの Schema、正常・権限混入の否定fixture、開発専用の関係検査を接続した。要求上限、本文上限、参照数上限、空白入力、識別子末尾の改行、未知field、閉じたセッション、応答対応、非全文表示への漏洩、左右成功・片側失敗・両失敗・応答入替え・セッション共有・表示許可転用を検査する。これらは CONFIG / FIXTURE 証拠であり、MINIDORA の live 動作、実行系停止、権限発行の証拠ではない。

- item: 製品対話の実装接続
  classification: release_blocker
  reason: 現単位の消費経路は Schema catalog と開発用 conformance。Rust 外部送信、UI、端末連携は未接続。
  required_action: 次単位で統治済み Rust 経路と Adapter、UI を実装し、実物通信と失敗・権限否定・取消を検証する。
  blocks_release: yes

契約単位の検証: Schema 30件、正常例30件、否定例32件、Conformance 145項目は PASS。Windows 開発用一括検証14検査と WSL/Linux の Python 側9検査は PASS。比較単独での空白入力の見逃しを追加監査で検出し、応答・比較の関係検査も拒否するよう補強した。権限判定や許可発行をこの開発検証へ持ち込まない。

製品接続の前提確認: 現行 `native/rust_helper/src/broker/authority.rs` の production_default は decision=deny、approvals=[] であり、owner 承認の登録経路がない。継続指示に基づき専用の Rust ローカルowner制御操作を採用した。以下の単位で通常UI資格と分離する。既存汎用commandのdeny/suspendを解除する変更ではない。

## Rust対話Core・MINIDORA Adapter・owner制御

専用の対話要求を通常IPCで保留し、別資格のowner CLIによる要求hash一致・期限内・一回限りの承認後だけ固定loopback APIを呼ぶ。Shell Coreは汎用traitを使い、MINIDORA固有のHTTPと応答射影はAdapterへ分離した。Flutter、Python、metadataから権限を発生させない。既存汎用command dispatchはsuspendedを保持する。

通信期限、要求・受信上限、worker数上限、セッション隔離、取消後の採用防止、raw受信と表示射影の分離を実装した。監査失敗時は送信または結果公開を拒否する。成功Adapter応答もCoreでセッションと構造を再検査する。owner資格は同一OS利用者に対する強い隔離や人間本人性の証明ではない。Windows installed-path保護の未検証を隠さない。

検証: `cargo test --manifest-path native/rust_helper/Cargo.toml` は53単体・5統合がPASS。`python tooling/minidora_live_check.py --reference C:/Users/mzcum/codex-work/MINIDORA-reference` は固定commitの実MINIDORA二processを用いPASS。通常資格拒否、owner CLI承認、基本会話、trace整合、表示分離、保留、片側停止・両側停止、監査chainの再起動読取を実行した。実API証拠はLIVE_RUNTIMEだが、基礎Core・外部検索能力の保証へ拡張しない。Schema31件・正常例31件・否定例33件、Conformance146項目、日本語厳格監査もPASS。

HTTP固定ヘッダーとCRLFだけを `JBE-007` の局所固定表記へ登録した。監査器の検出は弱めず、説明と診断は日本語のまま残す。Brokerは非同期状態を所有するためClone/Eqの導出を廃止した。生存workerや承認待ち状態を複製する内部APIは提供しない。

- item: Desktop比較UI・Mobile・端末連携・platform別実機証拠
  classification: release_blocker
  reason: この単位で接続したのはRust対話経路と実API検証。UI実装、secure storage、端末資格、Android/iOSの実動作は後続単位に残る。
  required_action: Flutter共有化と対話表示、二実行系比較、端末連携、platform別buildと実機検証を順次実装する。
  blocks_release: yes

- item: owner資格のOS保護とinstalled-path検証
  classification: release_blocker
  reason: ローカル開発資格の分離は同一利用者の任意processや管理者への耐性を証明しない。
  required_action: 既存の資格保護・監査anchorのrelease gateに従いWindows installed-path証拠を収集する。
  blocks_release: yes

集約検証: Windowsの `python tooling/validate_all.py --desktop-platform=windows` は14検査すべてPASS（終了値0）。WSL/Linuxでも `cargo test --manifest-path native/rust_helper/Cargo.toml` の53単体・5統合がPASSし、Linux binaryとLinux側の同一commit参照cloneによる `tooling/minidora_live_check.py` がPASSした。Windows cloneをWSL Gitで確認した際の改行差分は参照コードを書き換えず別cloneで分離した。GitHub Actionsは未使用。これらは開発環境の検証であり、installed-pathやMobile実機、owner GOのrelease_blockerを解除しない。

## Desktop対話・二実行系比較

Desktopのナビゲーションとコマンドパレットに対話を追加した。新規セッション、実行系選択、入力、送信、中止、応答、参照、能力、経路、追跡、失敗・復旧を表示する。比較では異なる実行系とセッションへ同じ入力を独立送信する。通常clientの操作allowlistにowner承認を含めず、応答の要求・実行系・session・表示境界を検査する。画面遷移で対話を保持し、デモ表示では新しい操作を無効にする。下部バーは対話の状態を起動時snapshotから推定しない。

接続資格のhostは127.0.0.1、TCP port・乱数secretの構造を検証し、Flutterの受信上限を4MiBにした。実行系への直接通信は追加していない。

Windowsで `flutter analyze`、`flutter test`（37件）、`flutter build windows --debug` を実行した。画面試験は左右の同一入力、独立した失敗表示、中止、デモの送信禁止を含む。製品Dart clientによる `python tooling/minidora_live_check.py --reference C:/Users/mzcum/codex-work/MINIDORA-reference --dart-client` もPASS。

Windowsの開発binaryを実起動し、Computer Useでコマンドパレットから対話画面へ移動、比較を選び「こんにちは」を送信した。左右に異なるsession・要求が作られ、owner CLI承認後に両方の実MINIDORA応答と異なる追跡IDが画面表示されたことを観測した。これは開発binaryのLIVE_RUNTIME観測であり、installer配布・installed-pathのrelease証拠ではない。

Dartの自動修正toolは終了時にperf一時file削除のOS Error 1920で失敗したため、lint指摘をソースで修正し、その後のanalyze/testで検証した。製品コードにtool障害の回避層は入れていない。

次単位のbackup更新で短いref名がbranch/tagの同名と衝突した。`git push -f origin codex/backup-main-prev:refs/tags/codex/backup-main-prev codex/backup-main:refs/tags/codex/backup-main` は曖昧なrefとして失敗した。完全な `refs/heads/...` 指定で修復し、remote tagの新世代5499637、前世代9bd8838を確認してから編集を開始した。今後も完全なref名で区別する。

- item: Flutter共通化・Mobile・端末連携・platform別最終証拠
  classification: release_blocker
  reason: Desktop対話の実装と検証をMobileやinstalled-pathの成立へ昇格しない。
  required_action: 次の共通化、Mobile正式project、端末認証と安全保管、各platformの検証を実施する。
  blocks_release: yes

Desktop単位の最終検証: Windowsの一括14検査はすべてPASS（終了値0）。Linux側の同一差分でも `flutter analyze`、`flutter test`（37件）、`flutter build linux --debug` がPASS。Linux製品Dart client→Rust→実MINIDORAの試験もPASSした。WSLgの `GDK_BACKEND=x11` でLinux開発binaryを起動し、broker接続とX11のIsViewableウィンドウを観測した。既定WaylandのウィンドウをX11検査器で検出できなかったことは、製品regressionとは分類しない。X11指定は試験の環境設定だけであり製品に追加していない。実起動証拠の環境範囲はREADMEのknown_limitationへ記録した。

## Flutter共通表示とMobile platform構成

対話画面・安全な応答検査・通常transport契約を `packages/gui_shell_ui` へ移した。接続は注入し、Desktopの資格読取とTCPはDesktop側へ残した。Dartのみのclient入口とFlutter表示入口を分け、dev-only実API検証がFlutter engineを必要としない構成を保持した。共通5試験とDesktop32試験で元の37試験の責任を保持する。

既存Mobileのlibを保持したまま、Flutter標準生成元からAndroid/iOS projectを追加した。Mobile依存lockfileはGit追跡するが、既存規約どおりMANIFEST対象から除外する。releaseの開発鍵流用は除去した。開発識別子・配布署名・実機証拠はMobile READMEのrelease_blockerとして明記した。

実API回帰の初回はFlutter表示層の推移的importで起動期限を超過した。client入口の分離後、`python tooling/minidora_live_check.py --reference C:/Users/mzcum/codex-work/MINIDORA-reference --dart-client` はPASS。Windows bat親processの終了だけでは子Dartが試験logを保持したため、検証toolはFlutter同梱のDart executableを直接起動するようにした。製品起動経路への回避層追加ではない。

集約検証の初回は16項目中14項目がPASS。Mobile lockfileをMANIFESTへ含めた変更が既存conformanceと衝突し、conformanceとZIP展開後conformanceが失敗した。MANIFESTの既存除外規約を維持する形に戻した。検査の削除・弱体化は行っていない。

- item: Mobile端末連携・安全保管・Android/iOS実機証拠
  classification: release_blocker
  reason: この単位は共有表示とplatform構成。Mobile libは従来の試作画面であり、実状態ではない。
  required_action: 端末資格、Desktopへの暗号化接続、失効、設定、lifecycleを実装して実機検証する。
  blocks_release: yes

修正後の conformance（146件）と packaging portability はPASS。共通・Desktop・Mobile analyze、共通5件・Desktop32件test、Rust53単体・5統合は集約検証でPASSした。

共有化後の lutter build windows --debug もPASS。Android/iOSのnative buildはこの単位では未検証であり、上記platform別release_blockerに含める。

## 端末連携の契約

招待・結合資格・暗号化要求の三Schemaと日本語意味正本を追加した。通常操作のallowlist、Host証明書固定、端末所有関係、招待300秒・結合8時間、nonce再使用拒否、失効・lifecycle・安全保管を定義した。port上限を機械検証するためSchema検証器にmaximum検査を追加した。

`python tooling/schema_check/check_schemas.py` は34 Schema・34正常例・36否定例、`python tooling/conformance_tests/run_conformance_skeleton.py` は147項目でPASS。必須field欠落、未知権限field、不正型、port境界、禁止操作、操作と内容の不一致を含む。証拠classはFIXTUREであり、暗号化や端末認証の実動作を証明しない。

- item: 端末連携の製品消費経路
  classification: release_blocker
  reason: この単位は契約と構造検査。TLS・資格・所有関係・安全保管の製品経路は次単位で実装する。
  required_action: RustとMobileの正常・否定経路および実通信を検証する。
  blocks_release: yes

## Rustの端末TLS経路

明示bind時だけ公開するTLS経路を同じDesktop Rust brokerへ接続した。通常loopbackとowner資格の分離を保持し、端末操作は対話・確認・離脱だけを許可する。招待の消費、秘密hash照合、期限、nonce、Host・端末結合、対話所有関係をRustで強制する。入力は全階層の重複fieldを拒否し、受信上限と有限期限を持つ。owner招待秘密は新規fileへ保存して標準出力へ出さない。

既存Rust53単体・5統合は変更後にPASS。端末状態機械の6否定／正常試験と、永続監査障害・期限処理を実Coreへ通す2試験もPASS。実TLSのdev-only clientによる正常結合、再接続、再使用拒否、不正資格、Host不一致、権限昇格拒否、他端末の対話・要求アクセス拒否、実MINIDORA応答、失効後の保留承認拒否、招待取消、離脱を実行した。製品Desktop Dart clientとの併用もPASS。

新依存はTLS・証明書生成・秘密hash比較に限定し、Cargo.lockへ固定した。通常RuntimeのMINIDORA接続は引き続き既存Adapter経路だけである。TLSの実通信証拠をMobile製品画面やOS安全保管の成立へ昇格しない。

- item: Mobile製品clientと安全保管・lifecycle
  classification: release_blocker
  reason: Rustの端末経路は実動作したが、Mobile libへの接続とAndroid/iOS実機証拠は未完成。
  required_action: MobileでHost照合・安全保管・対話画面・復帰処理を接続し検証する。
  blocks_release: yes

端末失効の反復試験では、当初64回で対話枠が尽きる資源保持を再現した。失効済みsessionを解放し、送信中workerだけは遅延応答のhash監査まで保持する修正を行った。修正後の実TLSによる結合・対話開始・失効70回はすべてPASS。遅延応答の破棄監査と資源解放の回帰試験も追加した。最終Rust検証は63単体・5統合でPASS。

集約検証は14項目PASS、2項目（broker authority parity、cargo test）が起動中のWindows binaryと再buildの競合によるOS error 5で失敗した。実通信processの終了後に順番を分け、両項目を再実行してPASS。製品や検査に回避層は追加していない。修正後の実TLS＋実MINIDORA＋製品Dart clientも再実行してPASSした。

最終監査では、試験内のraw file読取がhelper境界の禁止patternに該当したため、既存の `BrokerPersistentStore` 再読取・chain検証経路で監査記録を確認する試験へ置換した。検出器は変更していない。また高負荷時に既存取消試験の固定150ms待機が不足したため、取消の即時結果と空本文のassertionを保持し、2秒以内の実受信完了を待ってraw保持を検査する形へ修正した。再実行は63単体・5統合でPASS。TLS session再開も無効にし、毎接続の端末資格検査を維持した。


## Mobile製品client・安全保管・lifecycle

MobileのTLS clientは招待のHost・端末ID・証明書hashを固定し、実peer照合と証明書有効期間の確認より前にapplication資格を送らない。重複field・未知field・不正型・期限・private IPv4境界を拒否する。通常操作allowlistと有限通信期限を持ち、owner資格やRuntime直結を導入しない。

Androidの安全保管とiOS Keychainを接続し、書込後の再読取を必須にした。保管失敗時の平文fallbackはない。Android backup・device transferを除外した。復帰時は資格を再確認し、background中の通信と保留入力の自動再送を停止する。失効・不正資格・通信失敗時は入力を止め、正常解除と通信不能時のlocal削除を区別する。

既存6画面を維持し、対話・接続先・設定を追加した。固定previewの準備完了・承認件数を除去した。共有対話画面はinactive時に接続・polling・送信を停止し、再開時に照会する。旧previewのdevice_id / pairing_idと現行端末ID / 結合IDの対応を復旧画面ソースへ記録し、既存conformanceの用語検査を保持した。

`python tooling/minidora_live_check.py --reference C:/Users/mzcum/codex-work/MINIDORA-reference --dart-mobile-client` はPASS。Mobile製品Dart client → 実TLS → Rust Core → 実MINIDORA二processで本文・追跡・owner承認を検証した。異なる証明書の拒否、停止・再確認、失効後の拒否もPASS。OS安全保管と実機lifecycleの証拠には昇格しない。

`flutter analyze` と `flutter test`（Mobile12件）はPASS。初回widget試験は概要とdrawerの同名表示を両方拾って失敗したため、検査対象を実際のNavigationDrawerに限定して再実行した。安全保管失敗、期限・構造・Host不一致、資格確認と復帰の競合、local削除、破損資格の上書き拒否を含むFIXTURE検証である。

Android初回buildはFlutter生成値のGradle heap 8GBでnative memory allocationに失敗した（環境要因）。heap 2GB、metaspace 768MB、worker 2に限定した。次のbuildではflutter_secure_storage 11がSDK 37を要求し、AGP 9.0.1が新しいandroid-37.0を解決できず失敗した（toolchain互換性）。標準のSDK packageが不存在の`platforms;android-37`を要求するsdkmanagerコマンドも失敗した。公式対応表に従いAGP 9.1.1 / Gradle 9.3.1へ更新した。SDK directoryの偽装・pluginソース改変・依存検査の抑止は行わない。

一次資料: [AGP 9.1.1の対応範囲](https://developer.android.com/build/releases/agp-9-1-0-release-notes)、[FlutterのAGP 9移行](https://docs.flutter.dev/release/breaking-changes/migrate-to-built-in-kotlin)。

- item: Android/iOSのOS安全保管・実機install・launch・lifecycle
  classification: release_blocker
  reason: host上のDart実通信とFlutter fixtureは実機での成立を証明しない。ADB接続済み実機はまだ検出されていない。
  required_action: buildを成立させ、実機で保存・再起動・結合・対話・失効・復帰を測定する。
  blocks_release: yes

集約検証は17項目中16項目がPASSし、共有UI analyzeの波括弧lintだけが失敗した。修正後の共有analyzeはPASS。共有6試験・Desktop32試験・Rust63単体と5統合・broker parity・packaging・Schema・conformanceは集約時にPASSした。MobileはHost照合前の送信禁止と保管障害からの復帰試験を追加し、最終analyze・14試験がPASSした。


AndroidのAGP修正後、`flutter build apk --debug` はPASS（初回1778.1秒）。IME学習・自動入力の無効指定を含む最終ソースで再buildし38.5秒、`flutter build appbundle --debug` は42.0秒でPASS。SDK 35とCMake 3.22.1もpluginの標準依存として導入された。Kotlinの生存markerは`.kotlin` cacheとしてignoreし、生成物をcommitしない。

開発APKは175738543 bytes、SHA-256 `e5c5f30108f812d92e444993087c417a812806c2f47f3a445454035743dffdb6`。開発AABは71596894 bytes、SHA-256 `0ce3fdb996e048c98e665c5c770bac8a188a5e39dc3b57b115a395da35764586`。APKはarm64-v8a、armeabi-v7a、x86_64を含む。`apksigner verify --verbose --print-certs` はPASS、Android DebugのRSA 2048 / v2署名であり公開配布署名ではない。`zipalign -c -P 16 4` はPASS。`apkanalyzer manifest print` でmin SDK 24、target SDK 36、debuggable=true、allowBackup=false、fullBackupContent=false、usesCleartextTraffic=false、dataExtractionRulesの実格納を確認した。これは生成物のCONFIG / EXTERNAL_EVIDENCEであり実機の動作証拠ではない。

JDK17の`jarsigner -verify`は終了値0だが、自己署名・timestampなし・POSIX属性・JarFileとJarInputStreamの検証差について警告した。警告を削除するための再梱包は行わない。公式bundletool 1.18.3（公開asset SHA-256 `a099cfa1543f55593bc2ed16a70a7c67fe54b1747bb7301f37fdfd6d91028e29` を照合）の`validate --bundle=...`と`build-apks --bundle=... --mode=universal --output=...`はいずれもPASS。変換したuniversal APKのapksigner検証もPASS。AABのAndroid工具による消費は確認できたが、正式配布・署名・汎用JAR stream検証差は次の配布前確認へ保持する。

## Apple platformの手動補助build

このローカルhostはWindows/WSLであり、ローカルMacはない。owner rev2とAGENTS 3.1に従い、workflow_dispatchだけの補助workflowを追加した。Windowsで検証したFlutter commit `559ffa3f75e7402d65a8def9c28389a9b2e6fe42` とRust 1.95.0を用い、macOS開発app・iOS Simulator app・Rust helperのbuildを対象とする。GitHubの品質必須statusや自動CIは追加しない。対象commitと環境情報、log、tar成果物、hash、buildによるソース差分を収集する。追跡ソースが自動変更された場合は成功とせず、差分を確認する。

- item: Apple補助実行と実機証拠
  classification: release_blocker
  reason: workflowの追加は外部実行の成功やMac/iOS実機の成立を証明しない。
  required_action: 手動実行の対象commit・結果・artifactを確認し、実機install・launch・安全保管・lifecycleは別途測定する。
  blocks_release: yes

workflow追加のlocal検証はSchema（35正常・37否定）、conformance（147項目）、日本語厳格監査がPASS。これは手動起動限定のCONFIG検証であり、外部build結果は手動実行後に記録する。


Apple初回補助実行（run 34445302630、対象52bcbd2cc82519f5f6ebc6c80c8c60f96a1e12af）はRust64単体・5統合およびarm64 Mach-O buildがPASS。macOS project未追加によりFlutter buildがFAILし、iOSは未実行となった。分類はproduct regressionではなく未実装構成の検出であり、この時点のApple buildはrelease_blockerである。固定Flutter標準生成元からmacOS projectを追加し、Sandboxを保持したまま既存broker用network.clientを指定する。正式配布・実機資格配置はDesktop READMEのrelease_blockerへ保持する。


## 複数OS間のmanifest修復

Linux最新checkoutの集約検証ではFlutter/Rust関連検査はPASSしたが、manifest・release gate・梱包の3検査が失敗した。Windows編集時のCRLFをraw hashへ記録し、Gitが既存.gitattributesに従ってLFへ保存したことが原因である。検証toolのhash照合は変更せず、作業fileを既存の改行規約へ戻してmanifestを再生成する。生成時にはGitのeol属性と作業byteの不一致を拒否し、無言の正規化やhash比較の緩和を行わない。Git実repositoryを使いLF、明示CRLF、binary、混在改行、修復後の正常化を検証する。


## 現時点の要求監査（2026-09-10）

rev2全体は未完了。実装済みの対話Core・MINIDORA Adapter・Desktop比較・Mobile端末連携と、実機で未確認の範囲を分離する。完成製品releaseとowner GOは主張しない。

|要求|確認した経路・結果|証拠の限界|
|---|---|---|
|統治・日本語意味正本・Schema|手動Actions限定、対話・比較・端末連携の正本、35 Schema、35正常・37否定fixture PASS|CONFIG/FIXTURE|
|CoreとMINIDORA Adapter|Rustの要求・承認・監査・取消・隔離試験、固定参照commit 3400a3bb68b37efa1dc14ee8aaa28fda779bf1f8の実API PASS|基本会話と保留の範囲。基礎Core能力の保証ではない|
|対話・比較|共有6件、Desktop32件、Mobile14件、左右成功・片側失敗・両失敗・session/権限非混線の試験 PASS|画面fixtureとhost上の実通信を区別|
|端末連携|製品Dart TLS client、招待・失効・replay・Host・権限否定、実MINIDORA PASS|OS安全保管は実機未確認|
|Android版|analyze/test、APK/AAB、署名・alignment・bundletool PASS|実機install以降はrelease_blocker|
|Apple版|手動実行34446194013、対象`27b8713fd1a9ecdb81abe1d4225b99b26bda84ba`でRust単体64・統合5試験、macOS開発app・iOS Simulator appのビルドに合格|実機launch・Keychain・対話はrelease_blocker|
|Windows版|共有化・端末連携後の集約検査でlint修正後PASS、最新debug build PASS|2026-09-11に画面回帰を再開し下記の範囲で確認。installed-path証拠はrelease_blocker|
|Linux版|最新対話実装のFlutter build、各analyze/test、Rust試験、Desktop/Mobile製品Dart clientと実MINIDORA PASS|最新release起動とPID一致の可視window・broker監査も確認。WSLg X11の範囲|
|manifest・梱包|改行修復後のWindows/Linux Python系9検査 PASS。conformance148件|開発検証のpassはstrict releaseのpassではない|

Apple成果物は外部artifact `10139803376`（86677901 bytes）を取得しZIP SHA-256 `3110e6493467eef9c38dc371746655dbbbb7efeb89696ba03421c4916752a9ac` を照合した。内部tar SHA-256 `59496c1c87f3b36f4bb3d7f592457df3315af3099155066f24b0200495041cca` も一致し、両app・Rust実行file・iOS安全保管plugin資産を確認した。環境はmacOS15.7.9 arm64、Xcode16.4、iOS Simulator SDK18.5。追跡差分patchは空。Flutterが生成したmacOS registrantは既存Windows/Linuxと同じく追跡対象へ追加する。生成内容を手編集せず、依存の生成元はpubspecである。

artifact取得の初回HTTP直取得はredirect先で401となり、認証headerを別hostへ引き継がない取得で回復した。ログ表示のcp932 UnicodeEncodeErrorはPYTHONUTF8=1で回復した。いずれも製品build失敗ではない。

Windowsの画面回帰は当初Escで中断したが、2026-09-11のowner再開指示後に下記の開発回帰を実測した。installed-pathの独立したrelease blockerは保持する。

- item: 実機・installed-path・正式配布・owner GO
  classification: release_blocker
  reason: Android実機検証はowner指示で凍結中。Mac/iOS実機なし。Windows installed-path全体の証拠と正式配布署名も未成立。
  required_action: Androidはownerの再開指示を待つ。それ以外の必要な実機証拠・配布指定とstrict validationは独立して扱う。
  blocks_release: yes


## Windows隔離配置・Linux最新起動の追加証拠

対象ソースはcleanな `47299ce839f51f0bcb39cd3d19d98f69f1510003`。Windowsの `cargo build --release --locked --manifest-path native/rust_helper/Cargo.toml` は95秒、`flutter build windows --release` は92.4秒でPASS。`installer/windows/stage_installed_app.ps1` で新規run `rev2-47299ce-20260910` を作成し、source_worktree_clean=trueと生成manifestの実artifact hash一致を確認した。Flutter exe SHA-256は `e671cd41eeb1c1d9c147178f5a4ae5a53052ac375807f421c9bc31e49769f7e6`、Rust exeは `d2015557a120f6552aceb518fc6e30fce36c744b4b384e19bd97161548dbbf07`。

この配置に対する `installer/windows/collect_broker_smoke.ps1` はstatus=passed、errors=[]。制限loopback、認証付きIPC、永続store、再起動後のnonce再使用拒否、新規要求の受理、強制終了後の接続拒否を実測した。証拠は `%LOCALAPPDATA%/GUI-Shell/installed-runs/rev2-47299ce-20260910/runtime/evidence/windows_broker_smoke.json`。画面・Setup Doctor・監査アンカー保護を測定していないため、Windows installed製品全体のgateは解除しない。

Linuxは同じソース系統のrelease binary（SHA-256 `6582fd0acc0b94f0a1c09239af5626c1ada4180150f1ff00a9d25a7bc47268c8`）を実MINIDORA二processとRust brokerに接続して起動した。`GDK_BACKEND=x11` のWSLg環境で、`xwininfo -root -tree`、対象の`xprop -id <観測ID> _NET_WM_PID`、`xwininfo -id <観測ID>` により起動PID411との一致とIsViewableを観測した。app終了前に生存も確認し、検証後に自身のprocessを終了した。

永続監査にはflutter-request-1から5が記録された。health、normalize_payload、content_projection、approval_editの受理と、command_envelopeのsuspendedを確認した。これは起動時の既存broker経路の証拠であり、画面からの対話入力・表示内容全体の証拠ではない。Desktop/Mobile製品Dart clientから実MINIDORAまでの経路は別の実通信検証でPASSしている。DRI3 deviceを取得できないlibEGL警告は出たが起動は成立した。描画性能・Wayland・物理Linux端末への同等性は主張しない。


## Windows画面回帰の再開・Android実機検証の凍結（2026-09-11）

ownerの明示指示でWindows画面検証を再開した。対象は `d5341f96f207b57085453af4797536abe125a5ed` の実装を持つWindows debug app、Rust broker、固定参照MINIDORA二process。Computer Useによる可視画面の操作・観測であり、検証入力は「こんにちは」だけとした。設定・権限の変更は行っていない。

概要、環境診断、信頼、実行系、権限、agent、承認、監査、復旧、問題、証拠、設定、対話の13画面へ移動し、表示を確認した。既存画面から対話へ戻った際も入力と左右のsession状態を保持した。監査画面の既存projectionを新しい対話監査の表示証拠へ読み替えていない。

二実行系比較で同一入力を送信し、左右の異なるsession・要求IDと承認待ちを観測した。検証用owner CLIの `対話承認操作 --session-file <検証用owner file> 承認 <要求ID> <要求hash> full` で各要求を承認した後、双方の完了、成功、full、基本会話の応答、別々のtrace/hashを画面で確認した。通常UIへowner資格は渡していない。

次の要求では左を中止しても右の承認待ちが維持され、右も個別に中止できた。左だけ新規sessionへ切り替え、右の中止済みsessionを残して再送すると、左は承認待ち、右は送信失敗と自動再送しない旨を表示した。最後に左も中止した。検証用brokerの永続監査でも対話送信・承認・取得・中止を確認し、検証後は自身が起動したprocessを終了した。これはLIVE_RUNTIMEの開発経路証拠であり、installed製品全体の証拠ではない。

操作上、下方へscrollした位置で座標指定の送信clickに反応を観測できない試行があった。入力欄からTab・Enterで送信でき、その後は上方に表示した送信buttonのmouse clickでも要求発行を確認した。座標操作と製品側のどちらが原因かは確定していない。全画面サイズ・全入力装置の成立は主張しない。初回window列挙のtimeoutは待機後の再取得で回復した。

Androidは実機検証だけを凍結し、ownerの再開指示まで端末接続要求、install、launch、結合、対話、安全保管、lifecycleの実機試験を行わない。APK/AABと既存build結果を保持する。iOSは凍結対象外。凍結を合格やrelease scopeの削除へ置き換えない。

- item: Windows全表示条件とinstalled製品証拠
  classification: release_blocker
  reason: 今回の開発画面観測は、全入力・表示条件やinstalled-path、Setup Doctor、外部監査アンカーの証拠を満たさない。scroll後の座標click不成立の原因も未確定。
  required_action: installed製品の検証時に入力位置と反応を再現確認し、既存の機械検証可能なrelease evidenceを収集する。
  blocks_release: yes

この文書更新の初回release gate検査は、解決済み項目のblocks_releaseをfalseへ変更した台帳記述を拒否した。既存contractでは分類属性をtrueのまま保持し、status=resolved / active=falseで解決を表すため、台帳だけを修正した。検査器は変更していない。


## インストール済みSetup Doctorの製品出力（2026-09-11）

既存隔離配置 `rev2-47299ce-20260910` のrelease Flutter appとRust brokerを、新規runtime領域 `runtime/setup-doctor-20260911` で実行した。app SHA-256は `e671cd41eeb1c1d9c147178f5a4ae5a53052ac375807f421c9bc31e49769f7e6`、brokerは `d2015557a120f6552aceb518fc6e30fce36c744b4b384e19bd97161548dbbf07` で、以前の隔離配置の値と一致した。

実行commandはrepository外の `python ../GUI-Shell-installed-export-check.py`。この補助は既存のbroker-serverを起動し、実測path・hashを製品出力contextへ渡し、既存の `GUI_SHELL_SETUP_DOCTOR_CONTEXT_JSON` / `GUI_SHELL_SETUP_DOCTOR_EXPORT_JSON` 経路を起動するだけで、diagnostic checkを生成・改変しない。collector自体はPythonを使用するが、起動したappのPATHはWindowsとSystem32に限定した。この環境指定だけをPython非依存の完全な証拠には扱わない。製品経路の置換やUI操作の代替ではなく、既存製品出力に限定した開発用測定である。統合collectorの正式証拠へ接続した時点で補助測定は不要になる。

製品が書き出した `product.json` はSHA-256 `903e46b4f574e1b5768eb5e32be922b780eb3a175ada777777adf445f727c501`。`tooling.windows_release_evidence.validate_setup_doctor` へそのまま渡し、windows_setup_doctor_smoke=passedを得た。製品内の10項目はすべてpassであり、設定生成・監査領域書込み・認証付きbrokerと永続化の接続を含む。出力時のapp PID4904の生存を確認し、検証終了時に自身のappとbrokerを終了した。証拠は `%LOCALAPPDATA%/GUI-Shell/installed-runs/rev2-47299ce-20260910/runtime/setup-doctor-20260911/result.json` と同directoryの製品出力である。

- item: Windows統合release evidence
  classification: release_blocker
  reason: Setup Doctor単独の製品出力は得たが、可視画面、全体provenance、監査アンカー保護を含むcanonicalなwindows_installed_smoke.jsonは未成立。製品context由来のCONFIGとbroker実観測を全体保証へ昇格しない。
  required_action: 同一隔離runに結び付く統合証拠を収集してWindows release evidence全項目を検証する。
  blocks_release: yes


## 監査アンカー収集器の誤った保護成立判定を修正（2026-09-11）

実installed storeに対し `collect_audit_anchor_proof.ps1` がpassed / key_anchor_log_same_user_rewrite_mitigated=trueを返した。しかし同じユーザーでaudit_anchor.key、audit_anchor.json、audit.jsonlのすべてをFileMode.Open / FileAccess.Writeで開けた。byteは書いていない。広範な主体へのwrite ACEがないことを、所有者自身による一括書換え防護へ誤って昇格する既存不具合だった。

収集器はrelease用診断経路に限定して修正した。DPAPI固定文字列の往復はdpapi_availableへ区別し、監査鍵保護を表すdpapi_verifiedを成立させない。外部fileの存在/hashと任意fileのAuthenticode検証も対象chain・独立保管・信頼済み署名者の結合を証明しないため、外部アンカーや署名済み監査証拠の合格へ昇格しない。現行実装に同一ユーザーの書換えを防ぐ独立境界の検証はないため、保護成立を主張せずfailedを返す。runtimeの鍵・ACL・権限は変更していない。

`python -m unittest tooling.conformance_tests.test_windows_anchor_collector` は実PowerShell collectorを3ケース（書込可能なstore、無関係な外部file、署名file）で実行しPASS。Windows conformanceの既存collector接続検査にも組み込んだ。初回はWindows PowerShell 5.1と継承module環境の不一致によりGet-FileHash等の読込みが失敗し、利用可能なPowerShell 7を優先する試験起動へ修正した。製品検査を弱めていない。

同一の実installed storeで修正後に再実行しstatus=failed、same_user_rewrite_mitigated=false、dpapi_verified=false、dpapi_available=trueを観測した。`runtime/setup-doctor-20260911/anchor-before.json` と `anchor-after.json` に比較証拠がある。旧passedはreleaseの根拠に使わない。

- item: 監査アンカーの同一ユーザー書換え防護
  classification: release_blocker
  reason: 誤判定は修正したが、独立した保管・信頼基点・chain結合の実装と証拠は未成立。
  required_action: 独立境界と巻戻し・置換・改変の検出経路を定義して実装・実測する。
  blocks_release: yes


## 旧アンカー合格記録のrelease受入れを拒否（2026-09-11）

前項の収集器修正だけでは、修正前の `anchor-before.json` をWindows release検証器へ渡すとwindows audit anchor gateがpassedとなった。実測recordを検証用provenanceへ組み込んだ検証器単位の再現であり、統合releaseの成功を示すものではない。

`validate_audit_anchor_external_tamper_evidence` は、現行形式のcollector自己申告だけでは対象chainと独立した信頼基点の結合を検証できないことを明示し、release_blockerを返すよう修正した。同じ実測recordを再投入するとfailedへ変わった。external_anchor / signed_evidenceへsource_kindを付け替えてverified=trueとする3種類のfixtureも拒否し、他のWindows gateの正常fixtureは合格を維持した。元の全項目合格fixtureはこの未検証の保証を誤って正常扱いしていたため、他の正常経路の成立とアンカーの拒否を別々にassertする試験へ修正した。

- item: 独立したアンカー証拠の受理経路
  classification: release_blocker
  reason: 現行形式に信頼済み署名者・対象chain・置換や巻戻しを検証する消費経路がない。過去のpassedも保護を証明しない。
  required_action: 独立した信頼基点と保管先の境界を決め、chainへ結合した証拠を実際に検証する経路と否定試験を実装する。
  blocks_release: yes


## owner確定方式のオフライン署名checkpoint（2026-09-11）

ownerが指定したEd25519オフライン署名方式を、Rustのrelease専用CLI・Schema・Collector・release再検証へ接続した。固定順序canonical checkpointは監査head、log/anchorのraw SHA-256、source commit、実artifact hash、時刻、sequence、前署名checkpoint hash、versionを署名対象にする。Repositoryの公開鍵fingerprintが未固定なら拒否する。通常brokerのIPC・runtimeから到達せず、秘密鍵の読取り・署名APIを追加していない。

継続性記録を証拠から自己採用しない。owner管理の最新sequence/hashを別に与え、直前署名の検証と連続性・同番号置換・後退を検査する。検証対象と継続性記録の両方を巻き戻す場合の限界、ownerの意図的再署名・物理侵害の対象外、administrator_root_resistance_claimed=falseを正本へ明記した。

Rust試験はメモリ上の使い捨て鍵を用い、正常署名、不正署名、別鍵、未固定鍵、checkpoint byte改変・非canonical・重複field、log/anchor/head/source/artifactの不一致、時刻、previous不一致、巻戻し・同sequence別署名を検証した。実fileの変更と、実Rust CLI → PowerShell Collector → Python release consumer → Rust再検証も検証し、収集後のartifact改変で拒否した。これらはFIXTUREの実実行でありowner署名証拠ではない。実秘密鍵は生成・保存・読取りしていない。

最初の編集commandでUTF-8 fileをcp932として読もうとして失敗した。書込み前の失敗で、明示UTF-8で再実行した。追加暗号依存は既存rustls依存でも使用しているring 0.17.14を直接参照しただけで、独自暗号方式を追加していない。

- item: 実ownerの公開鍵固定と署名済み証拠
  classification: release_blocker
  reason: 実装と試験用署名経路は成立したが、実運用fingerprintはnullであり、実owner署名・外部媒体の継続性記録は未取得。
  required_action: docs/OFFLINE_SIGNING_OWNER.mdのownerローカル操作を実施し、公開鍵・署名証拠だけを取得して実配置で検証する。
  blocks_release: yes

この単位の初回集約検証は配布パス検査だけがFAILした。新しい公開鍵固定fileの日本語pathが既存ASCII配布規約に合わなかったため、機械読取りpathをconfig/audit_signing_trust.jsonへ変更し、日本語意味正本と責任索引を保持した。検査allowlistは拡大していない。修正後の署名6試験（Rust CLI・Collector・release再検証を含む）はPASSし、ringで署名したcheckpointをOpenSSL標準検証でも受理することを確認した。

最終の `python tooling/validate_all.py --desktop-platform windows --include-mobile-release` は開発検証17項目すべてPASS。Rust63単体・5 IPC・6 checkpoint試験、共有6・Desktop32・Mobile14試験を含む。これは実owner署名とWindows統合release証拠の合格ではなく、owner手動操作の前で停止する。


## 実運用鍵を延期してWindows統合収集を検証（2026-09-11）

owner指示により、監査アンカーの実運用鍵・実署名は正式release直前まで外部条件待ちとする。Android実機検証の凍結も維持する。他の実装・検証は継続する。

source `7a4afd838aed44489c71d560dcd6a876b53ce9dd` をLinuxへfast-forwardし、`CARGO_TARGET_DIR=/home/mzcum/.cache/gui-shell-rev2-target python3 tooling/validate_all.py --desktop-platform linux --include-mobile-release` は18項目PASS（Rust64単体、5 IPC、5 checkpointを含む）。Windowsは `flutter build windows --release` と `cargo build --locked --release` から `rev2-7a4afd8-20260911` へ分離配置した。実broker検証とSetup Doctor製品出力10項目はPASS。Computer Useで配置先windowの概要・環境診断への遷移とscrollを観測した。

同配置で `collect_installed_smoke.ps1 -NoPythonRuntime` の `-VisibleSurfacesJson` に、その起動で製品が生成するsurface_semantics_export.jsonを指定した。UIAutomationは実行していない。旧検証器はWindows4関門をPASSとしたが、出力の生成元を確認すると `SurfaceSemanticsRegistry` はbuild時の名前を蓄積するだけで、現在の描画・可視性・破棄を観測していなかった。したがって、この初回起動PASSを完成証拠には採用しない。

収集器はこの既知のbuild registry形式をINTERNAL_STATEとして保存し、可視surface・初回起動を合格へ昇格しない。release検証器も旧collectorのpassedと、source名だけを変更した同形式を拒否する。既存の受入れ試験はこの誤った保証を正常扱いしていたため、拒否の回帰試験に置き換えた。初回の試験編集では別の不足fieldによる拒否を拾っていたため、元fixtureのfieldを保持して再実行し、修正前FAIL・修正後PASSを確認した。

実collector再実行は `runtime/registry-rejection/windows_installed_smoke.json` に保存した。first_run=failed、visible_surfacesのevidence_class=INTERNAL_STATE、formal_release_input=falseを観測。旧記録の再投入もfirst-run=failedとなり、両記録でSetup DoctorとbrokerはPASSを維持した。ログ・実配置・証拠・秘密をRepositoryへstageしていない。

- item: Windows初回起動の可視surface証拠
  classification: release_blocker
  reason: build registryによる誤受理は修正したが、現windowの個別surfaceを外部から確認する厳格な統合証拠は未成立。
  required_action: 実描画・可視性を観測する経路を初回起動と結合し、非表示・破棄・別起動の負例も検証する。
  blocks_release: yes


## 可視性を捏造しない製品診断出力（2026-09-11）

固定Flutter 3.44.0のWindows engineソース `flutter_window.cc::OnGetObject` を確認した。UIAutomation応答は `FLUTTER_ENGINE_USE_UIA` のコンパイル条件内で、MSAA応答は別経路にある。公式の背景説明は [Flutter issue 114547](https://github.com/flutter/flutter/issues/114547) にある。これは個別widgetを取得できない既観測と整合するが、配布済みDLLのコンパイル条件や外部tool側の挙動まで確認した証拠ではない。独自engineへの差替えは行っていない。

前単位で受入れ側を修正したbuild registry出力について、生成元にも残っていた架空の座標、is_offscreen=false、node ID、可視surfaceの合格を除去した。登録履歴はregistered_surfaces / registered_identifiersへ保存し、起動PID、INTERNAL_STATE、visibility_measured=false、formal_release_input=falseを明示する。旧source / diagnostic mode識別子を維持し、既存collectorとrelease検証器で診断資料として扱い、可視証拠への昇格を拒否する。通常UIの描画・Semantics識別子・権限経路は変更しない。

非表示Offstageのwidgetを実際にbuildした試験で、旧出力が必須4surfaceをすべてvisible_surfacesへ入れることを再現した。修正後は非表示時と破棄後の双方でvisible_surfaces・surface_matches・観測nodeが空となり、登録履歴だけを残す。Desktop全33試験とanalyze、Schema36/正常36/負例38、conformance148がPASS。

- item: 外部から観測したWindows個別surface
  classification: release_blocker
  reason: 診断出力の虚偽の可視性表現は除去したが、実画面の外部観測を厳格な初回起動証拠へ結合する経路は未成立。
  required_action: 固定toolchainの対応アクセシビリティ経路と利用可能な観測toolの接続を確認し、現在window・個別surface・非表示の負例を実測する。
  blocks_release: yes


Windows releaseビルド後、実broker経由でappを起動し、生成されたsurface.jsonのprocess_idが起動PID 14112と一致することを確認した。登録履歴は存在するが可視surface・match・観測nodeは空で、INTERNAL_STATE / visibility_measured=false / formal_release_input=falseを確認した。実測は `%TEMP%/gui-shell-build-registry-zaynljnf/result.json` に保存し、起動したappとbrokerは終了した。Mobile analyzeもPASS。これは製品診断出力の実行証拠であり、外部可視性の証明ではない。

このビルドでrunner exeのSHA-256は既存配置と同じ `e671cd41eeb1c1d9c147178f5a4ae5a53052ac375807f421c9bc31e49769f7e6` だった。一方、Dart AOTのdata/app.soは旧配置 `1c8776ff3a88b4af1bbaf6b9902a7231053be36da6d6c1f8eafe0f1fc0da513b` から `03590c9ea67be5fc603c1bcb32e1e7a7a5a4e3f1bcf5cf3440f786753f4af157` へ変化した。native/rust_helper/src/checkpoint.rsのmeasureはartifact引数の単一fileだけをhashし、現在のWindows collectorはapp exeを渡している。source commitの一致だけでは配置後のapp.so改変を検出しない。

- item: 監査checkpointの配置成果物hash範囲
  classification: release_blocker
  reason: 現行のexe単体hashではDart AOT、engine、plugin等の実行配布物の置換を検出できない。
  required_action: 配置app/brokerの実行配布物一式をcanonical manifest等へ結合し、欠落・追加・置換・path改変の負例を含めてcheckpointとrelease再検証へ接続する。
  blocks_release: yes


## checkpointをWindows配布物一式へ結合（2026-09-11）

checkpoint version 2へ更新し、installed_artifact_sha256をRustがinstalled rootから実測するcanonical配布物一覧のhashへ変更した。app/broker以下の全file・directoryと両launcherを含み、path・種別・size・内容hashを結合する。runtimeとinstalled_manifest.json以外のroot追加、必須成果物欠落、規約外path、symbolic link/junction/reparse point、特殊file、大小文字衝突を拒否する。上限付きで二度走査し、変化すれば失敗する。旧exe単体署名version 1は拒否する。

Schemaと正本を先に更新し、prepare / verifyの引数をinstalled rootへ変更、artifact-manifest CLIを追加した。CollectorとPython release再検証も同じrootを渡し、version 2の実検証結果のみ受理する。通常runtimeの経路や新しい依存は増やしていない。ownerの実運用鍵・署名は正式release直前まで外部条件待ちのまま。

Rustは63単体・5 IPC・8 checkpoint試験PASS。既存の署名・chain・sequence・source不一致に加え、AOT/engine/broker/launcherの改変・削除、追加file、rename、root追加、junction、規約外path、旧version拒否を確認した。実CLI → OpenSSL公開鍵検証 → PowerShell Collector → Python release consumer → Rust再検証で、runnerを変えずにAOTだけを改変すると拒否する。秘密の試験鍵はprocessメモリ内だけに生成し、実運用鍵を扱っていない。初回は試験内のJSON型代入がcompile errorとなり修正した。junction試験のmklink引数にforward slashが残っていたためInvalid switchとなり、Windows path componentの組立てを修正して全試験を再実行した。

実配置 `rev2-7a4afd8-20260911` の15 file / 21 entryをRustで測定し、Pythonで独立に列挙したpath・size・SHA-256と全件一致した。別の検証用コピーでapp.soだけを改変し、runner hashが変わらないまま成果物一覧hashが `027f460e382cf4913d701d9d605a108111db89cbef00dc1c038cdced2de72238` から `308feef71c18b8c4c3c52be4ebbebdfa2ca6b94eee9827e261cdb672a12457d7` へ変化した。実測は `%TEMP%/gui-shell-artifact-actual-tafn4o16/runtime/result.json` とbefore/after一覧へ保存した。これは未署名の実配布物測定であり、owner署名証拠ではない。

Schema36/正常36/負例38、conformance148もPASS。配布物の変更を止めた検証時点のsnapshotを扱い、原子的なfilesystem snapshotや検証後の実行時差替え防止は主張しない。正本にその境界を明記した。

- item: 実運用署名済み配布物checkpoint
  classification: release_blocker
  reason: 配布物一式を結合する実装・負例は検証したが、owner公開鍵固定と実署名は延期中。
  required_action: 正式release直前にclean commitから配置し、外部媒体上のowner鍵でversion 2を署名して現配置を再検証する。
  blocks_release: yes


## 現在commitの集約回帰とfilesystem境界試験（2026-09-11）

source `0cdbab7a13629cc6d96618160883286e7a718336` で `python tooling/validate_all.py --desktop-platform windows --include-mobile-release` の開発検証17項目、Linuxで同commandのplatform=linuxを指定した18項目が全PASSした。Windowsの正式統合証拠がRepository既定pathに存在しないことによる個別release blockerは残る。開発モードのrelease_gate: passを厳格releaseの合格と扱わない。ログはRepository外のGUI-Shell-0cdbab7-windows-validation.txt / GUI-Shell-0cdbab7-linux-validation.txtへ保存した。

配布物のpath試験にあった「UnixならCASE/caseを別entryとして作れる」という前提を除去した。実directoryを列挙して2 entryなら大小文字衝突を拒否、1 entryなら一つだけを測定し最後に書いたbyteのhashと一致することを検査する。OSによる試験除外も不要とした。production codeと拒否要件は変更しない。変更後のcheckpoint試験はWindows8件・Linux7件PASS、Schema36/正常36/負例38・conformance148もPASS。


## WindowsのSemantics起動順序を修正（2026-09-11）

DesktopのmainでOS要求より先に保持していたSemanticsHandleを除去し、Flutter標準のplatform lifecycleに有効化を任せた。独自engine、UIAutomation wrapper、依存追加はない。固定Flutter 3.44.0のbindingとWindows engineを調べると、手動handleがある場合は後続のOS要求でDart側の有効状態が変化しない。この順序が初回treeの欠落に関係するという仮説で、同一broker起動ハーネスとreleaseビルドを比較した。engine内部の通知順序をtraceしたわけではないため、詳細な因果経路は推論として保持する。

変更前のPID15840 / window3999218では外部アクセシビリティ照会にwindow枠だけが現れた。8行の起動処理を除去した比較版PID6884 / window1770286では、初回に108要素を観測し、概要、ナビゲーション、実行系状態、不変条件の個別groupと内容を取得できた。診断へのクリック後は環境診断、broker IPC、保護項目拒否を含むtreeへ切り替わり、概要へ戻ると実行系・不変条件の内容が再出現した。再描画で要素番号が失効したクリック2回は未成立として扱い、画面を再取得して座標操作後に遷移完了を再観測した。先行文書のUIAコンパイル条件は今回の原因を確定する証拠ではなく、この標準engineでも個別widgetを取得できることが新たな観測である。

観測はComputer Useの外部treeであり、製品build registryから合成していない。Repository外のGUI-Shell-semantics-external-observation.jsonに診断と概要復帰の実treeを保存した。ハーネスの診断出力は引き続きINTERNAL_STATE、可視surface空、formal_release_input=falseを確認した。署名鍵は使用していない。

`flutter build windows --release` とDesktopの `flutter test --reporter compact`（33件）、Desktop/Mobileの `flutter analyze`、Schema36/正常36/負例38、conformance148はPASS。厳格なinstalled collectorの再収集をこの開発用起動で代替しない。

- item: 現行配置からの厳格なWindows可視surface証拠
  classification: release_blocker
  reason: 外部tree取得と画面切替を実測したが、clean sourceと分離配置に結合した正式collectorの証拠は未成立。
  required_action: 現行commitを分離配置し、個別surfaceの実観測と非表示の負例を厳格な収集・検証経路で確認する。
  blocks_release: yes


## 可視surfaceの宣言と実観測の結合（2026-09-11）

clean source 53182cd4cab77e025c61913357cf527270f4dd11をrev2-53182cd-20260911へ分離配置した。Windows releaseビルドとRust releaseビルド、配置先broker smokeは成功し、正式consumerではSetup Doctorとbrokerの2関門がPASS。初回起動と可視証拠provenanceはFAIL、実運用署名は延期によるFAILを維持した。実証拠とvalidation-result.jsonは配置先runtime/evidenceに保存している。

標準MSAA APIの実測ではwidgetの名前・役割・座標を取得できたが、画面外要素も非表示bitなしで返った。既存collectorは名前だけで候補を選び、consumerはmatchを観測treeへ結合せず、診断keyも先頭20件しか調べていなかった。正常fixtureも4surface宣言に対しrootと1surfaceしか記録していなかった。今回の単位はbuild/release経路のこの誤受理を修正する。

正本を更新し、全観測要素の識別子・root・親子edge・件数を検証する。matchは同じelement_keyの実観測属性と一致し、root/containerでないことを要求する。要素からrootまで明示的なis_offscreen=falseと有限・正の矩形を要求し、共通領域が正面積の候補だけを可視とする。collectorも同じ幾何条件で候補を選び、状態取得失敗はnullとして保持する。親列挙をRawViewWalkerへ揃えた。

17負例は変更前すべて誤受理、変更後すべて拒否した。過大整数座標の例外化も拒否する試験を追加し、負例は計18件。画面外、親領域外、非表示、状態・座標欠落、ゼロ面積、非有限座標、親欠落・循環、識別子重複、match差替え、未観測match、件数・edge不一致、container流用を含む。実collectorの純粋判定関数も正常と11負例で試験した。Windows PowerShell 5.1ではUTF-8 BOMなしの日本語scriptを既定encodingで読むと構文エラーになるため、試験入口とAST読込みでUTF-8を明示した。production用wrapperや新依存は追加していない。Schema36/正常36/負例38、conformance149件、日本語基底監査はPASS。

これは収集済みtree上の表示領域交差の検証であり、別windowによる遮蔽やpixel内容の証明ではない。MSAA実測を正式collectorへ接続する作業は、この条件を省略せず続行する。

- item: MSAA実測と正式Windows収集経路の接続
  classification: release_blocker
  reason: UIAutomationの現経路ではwindow枠しか取得できず、MSAAは診断実測のみ。今回の可視性負例はFIXTUREであり、正式installed初回起動の合格ではない。
  required_action: MSAAの要素同一性・親子関係・実矩形と日本語surfaceを正式収集へ結合し、実画面と負例を再検証する。
  blocks_release: yes


## 日本語surfaceを既存UIAutomation経路で実測（2026-09-11）

前単位ではMSAA接続が必要と判断したが、新しい診断runでは日本語label対応を追加した既存UIAutomation経路だけで108要素を取得した。別起動とMSAA追加コード除去後も計3 runで必須4surfaceと観測tree・矩形交差の検証が通った。前のwindow枠2要素との時間・観測経路による差の詳細は未確定であり、日本語対応がOS側tree公開自体を変えたとは主張しない。MSAAの独自collectorは未使用で、必要性を実証できないため採用せず除去した。新言語・依存・実行経路は追加しない。

修正はcollectorとconsumerの意味対応に限る。概要、ナビゲーション、実行系状態、不変条件状態、および既存Dart安定identifierを固定し、日本語名は空白正規化後の完全一致または見出し二重連結だけを許可する。元の観測名、UIA element key、runtime ID、親子関係、座標を保持する。説明文やTab名、identifierの余分なsuffixは拒否する。

診断実測はrev2-53182cd-20260911/runtime内のmsaa-check、uia-japanese-repeat、uia-only-finalへ保存した。すべてsource=uiautomationであり、製品registryでもMSAAの合成出力でもない。診断専用のため正式初回起動の合格と扱わない。Schema36/正常36/負例38、conformance150件はPASS。MSAA接続そのものを残作業とした直前の判断は撤回し、現行commitのcleanな分離配置からの正式再収集を次の境界とする。

- item: 日本語surface対応後の正式Windows再収集
  classification: release_blocker
  reason: 現行collectorの個別surface実測は成功したが、上記は変更中の診断専用runであり、現行clean commitからの正式統合runは別に必要。
  required_action: commit後に分離配置して同collectorを通常モードで実行し、署名を除く各関門の実結果を確認する。
  blocks_release: yes
