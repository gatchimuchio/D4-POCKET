# GUI操作面

状態基準日: 2026-09-24

GUI-ShellのGUI堅牢化では、権限をFlutterへ移さず、実証済みの操作パターンを取り込む。Flutterは状態と操作者の意図を表す操作面を描画し、Shell Coreは引き続き権限境界を担う。

## C31 現行文書境界（2026-09-24）

現行Desktop操作面は、C20のWindowsトレイ、C21のCommand Palette、C22の全体検索、C23のNavigation論理グループ、C19のAdapter管理、C18のHost操作面を含む。C24のMobile投影は既存Brokerの読み取り専用handlerへ限定し、Desktopと同じauthorityを持たない。

- production path: Flutter操作面 → 既存の認証済みBroker IPCまたは表示専用Snapshot → bounded projection。
- authority boundary: UIの選択、検索、グループ、通知、Host、Adapter metadata、Mobile projectionはPermission、Approval、Authority、Credentialを生成しない。
- content boundary: 対話本文、Approval payload、Audit raw reason、Credential実値は通常の表示投影へ出さない。全文表示は`content_visibility=full`だけに限定する。
- evidence boundary: C30のlocal回帰matrixは各面のcontract・fixture・Broker試験を示すが、Windows installed product、外部Runtime／Agent、実端末の操作証拠ではない。

未接続の実作用（外部artifactのdownload／install、任意Agent task、Tool実行、実停止、実端末連携）は、要求receiptや画面表示を完了扱いにせず、各文書の`release_blocker`または`known_limitation`として保持する。

## 実装済み操作面

~~~yaml
- item: Trust Center
  classification: required_for_v1
  status: implemented
  evidence: デスクトップアプリは、workspace、runtime、adapter、installerのtrust recordを、trusted、restricted、inherited、unknownの状態および遮断済み操作とともに公開する。
  authority_boundary: 将来のShell Core trust mutation操作がcapability、permission、approval、audit、recovery mappingを付与しない限り、表示専用である。

- item: Authority Map
  classification: required_for_v1
  status: implemented
  evidence: デスクトップアプリはRuntimeからCapability、Permission、Approval、AuditEvent、RecoveryActionへ至る対応関係を、warningおよびdangerフィールドとともに公開する。
  authority_boundary: 視覚的な対応図に限り、権限を付与しない。

- item: Audit Timeline
  classification: required_for_v1
  status: implemented
  evidence: Audit Viewerはruntime、adapter、approval、permission、setup_doctor、normalization、installer、およびerror、warning、blockedのフィルターと、copy、export、verify、jumpの操作語彙を含む。
  authority_boundary: verifyおよびexport操作はShell Coreの監査検証を根拠にしなければならない。

- item: Recovery Playbook
  classification: required_for_v1
  status: implemented
  evidence: Recovery Centerはseverity、retry state、pre_check、action_steps、post_check、rollback、およびaudit/recovery mappingの語彙を含む。
  authority_boundary: Shell Coreの認可なしには、いかなる復旧操作も実行しない。

- item: Adapter Catalog and Permission Diff
  classification: required_for_v1
  status: implemented
  evidence: Runtime Centerはadapterのpublisher、source、version、signature、hash、要求・付与・拒否されたcapability、trust status、risk、およびpermission diffを描画する。
  authority_boundary: install、disable、quarantine、removeは引き続きShell Coreの操作である。

- item: Settings UX
  classification: required_for_v1
  status: implemented
  evidence: Settings画面は検索filter、source/default/current/effective value、modified/dangerous/authority flag、reset/export語彙、およびcommand palette語彙を含む。
  authority_boundary: setting mutationはShell Coreが制御する操作として表現する。

- item: D4 Pocket Command Palette（C21 current scope）
  classification: required_for_v1
  status: implemented_for_current_scope
  evidence: Ctrl+K／Ctrl+PのパレットへRuntime、Agent、履歴検索、評価実行、MCP接続、通知表示、資源監視、資格情報、更新確認、Host切替、全Runtime停止要求の確認を明示登録し、選択時は対応画面だけを開く。
  authority_boundary: パレットは検索と画面遷移だけを行い、Broker IPC、owner承認、Permission、Approval、Authority、Credential、Runtime実行へ直接到達しない。各画面の実操作は既存の統治経路に従う。

- item: D4 Pocket 全体検索（C22 current scope）
  classification: required_for_v1
  status: implemented_for_current_scope
  evidence: Ctrl+Shift+Fまたは全体検索ボタンから、Runtime、Agent、Session、Permission、Approval、Audit、Recovery、Problem、Evidence、MCP、A2A、Host、Adapter、Profile、Evaluation、Notificationのbounded表示用metadataとsurface entryを横断検索できる。indexは512件、queryは128文字、結果は30件へ制限する。
  authority_boundary: 全体検索は読み取り専用であり、対話本文、Approval payload、Audit raw payload、Credential実値、秘密値を検索対象にしない。検索結果の選択は画面遷移だけで、Broker IPC、filesystem、process、network、credential、Clipboard、Permission、Approval、Authorityへ直接到達しない。

- item: D4 Pocket Desktop UX統合（C23 current scope）
  classification: required_for_v1
  status: implemented_for_current_scope
  evidence: Desktop Navigationは既存20画面を保持したまま、運用、安全、開発、設定、すべての論理グループでNavigationRailを絞り込める。別グループの画面へ移動すると対象グループへ表示を切り替える。
  authority_boundary: グループ選択はFlutterの表示状態だけを変更し、画面の権限、Broker IPC、owner承認、Permission、Approval、Authority、Credential、Runtime実行へ直接到達しない。画面本体と既存統治経路は削除・置換しない。

- item: Problems Panel and Evidence Center
  classification: required_for_v1
  status: implemented
  evidence: Dashboardはrelease blocker、problem、evidence statusを描画し、Setup Doctorはinstalled-path evidenceを描画する。
  authority_boundary: 機械検証済みのWindows installed-path evidenceがなければ、evidence表示はrelease readinessを満たさない。

- item: Status Bar
  classification: required_for_v1
  status: implemented
  evidence: 常時表示のstatus barはruntime status、trust status、pending approval、audit chain status、network exposure、およびrelease blocker countを描画する。
  authority_boundary: status barは読み取り専用である。

- item: D4 Pocket Host Capability
  classification: required_for_v1
  status: implemented_for_current_scope
  evidence: D4 Pocketのデスクトップ操作面はRust Brokerの認証済み「ホスト能力」応答を読み取り、platform、host、能力状態、証拠種別、理由を表示する。能力状態はunknown／unavailableを0へ変換しない。
  authority_boundary: Host Capabilityは観測結果であり、Permission、Approval、Capability grant、runtime trustを生成しない。画面は読み取り専用である。

- item: Agent Adapter contract
  classification: required_for_v1
  status: implemented_for_current_scope
  evidence: Agent Adapter Schemaはidentity、provider、version、model、capability宣言、workspace、tool／MCP／session／cancellation／metrics、認証方式、Host要件を検査し、未対応機能の理由を要求する。Rust Brokerの認証済み`Agent一覧`は、起動時に実物interfaceを確認したread-only Codex Adapterのmetadata-only投影をDesktopへ返す。
  authority_boundary: Adapter宣言は対応状況の説明であり、Permission、Approval、trust、credential実値を生成しない。DesktopのAgent Adapter状態は`LIVE_RUNTIME`のinterface確認範囲だけを表示し、実task/write dispatch、任意executable、Workspace、CredentialをUIから受け付けない。実物interface未確認のLauncherはunsupportedとして扱う。

- item: D4 Pocket Agent Launcher（Codex current scope）
  classification: required_for_v1
  status: implemented_for_current_scope
  evidence: ownerがBroker起動時に絶対executableとworkspaceを明示したCodex CLIだけをRust Adapterへ登録できる。登録時にversionと`codex exec --help`をLIVE_RUNTIMEで確認し、既存の実行系対話面から固定read-only JSONL実行、bounded output、取消、期限超過、失敗射影を通す。Windows実Brokerの認証付き通常IPCで`codex`実行系の列挙を確認した。Agent Adapterへの対話開始ではWorkspace IDを必須とし、Brokerが同じRuntime IDのWorkspace登録と照合してからSessionを作る。Desktop対話面は既存BrokerのWorkspace一覧から同一RuntimeのIDを表示し、操作者が選択した値を通常要求へ渡す。開始Audit hashは維持しつつ登録hashを含むWorkspace結合Auditを追加し、Desktopの`対話セッション一覧`はSession／Runtime／状態／作成Audit／Workspace ID／結合Auditを受け取る。
  authority_boundary: Flutterはexecutable、workspace path、argv、environment、Permissionを指定せず、Approvalを自己承認しない。Adapterは`--sandbox read-only`、`--ephemeral`、environment allowlist、secret path拒否を強制し、汎用command dispatchを有効化しない。Session-to-Workspace対応はBroker metadataの登録対応に限られ、Agent専用実行Session、実行directory、書込み隔離、実taskのwrite実行を証明しない。MCP、実Agentの複数比較、実Handoff、Claude／Gemini接続は未成立として扱う。session一覧は`INTERNAL_STATE`であり、実Agentの稼働やWorkspace隔離を証明しない。Mobile Device LinkにはWorkspace選択面がないためAgent対話開始は拒否する。比較projectionのsnapshot参照は同一Agent runtime ID／Workspace参照をfail-closedで拒否し、異なる宣言値を実Agent identityや実Workspace隔離の証明へ昇格しない。

- item: D4 Pocket GUI Shell Compose（Phase 29 current scope）
  classification: required_for_v1
  status: implemented_for_current_scope
  evidence: 認証済みBrokerの`GUI Shell構成`へRuntime、Agent、Tool、MCP、Theme、Capability、Settingsを渡し、Manifest-onlyの構成Receiptを返す。Brokerが構造、重複、表示範囲、継承禁止を再検証し、Desktop設定画面はManifest結果を表示する。
  authority_boundary: 構成のCapability requirementはPermissionではない。Authority、Permission、Approval、Credential、Audit chainは継承せず、build、独立App identity、filesystem、process、network、credential実値の経路を持たない。Windows Export、Preview／rollback、Module Pruning、Distributionは未成立の`release_blocker`として保持する。

- item: D4 Pocket GUI Shell構成Preview（Phase 30 current scope）
  classification: required_for_v1
  status: implemented_for_current_scope
  evidence: 認証済みBrokerの`GUI Shell構成Preview`へ現在Manifestと候補Manifest、対象platformを渡し、構成差分、Capability requirement、版rollbackの可否をPreview Receiptへ射影する。Desktop設定画面は構成Preview結果を表示する。
  authority_boundary: Previewは読み取り専用であり、Permission、Approval、Authority、Credential、Audit chainを生成・継承しない。`rollback_available=false`、build／Exportは未開始に固定し、Previewを実rollback、実build、独立App生成へ昇格させない。Windows Export、Module Pruning、Distributionは未成立の`release_blocker`として保持する。

- item: D4 Pocket GUI Shell編集提案（Phase 31 current scope）
  classification: required_for_v1
  status: implemented_for_current_scope
  evidence: Owner／Developer明示の`GUI Shell編集提案`を認証済みBrokerへ渡し、規約確認済みの対象path、予定変更、提案hashを審査待ちReceiptへ射影する。Desktop設定画面は提案結果を表示する。
  authority_boundary: `proposal_only`、`review_required=true`、`files_written=false`に固定し、製品Runtimeの自己変更、自動apply、自己承認、Permission生成、Approval迂回、Credential取得を行わない。実装差分の適用とOwner確認は別の開発作業として残る`release_blocker`である。

- item: D4 Pocket GUI Shell Windows書出し（Phase 32 current scope）
  classification: required_for_v1
  status: implemented_for_current_scope
  evidence: Desktop設定面はCompose Manifestと任意画面Module選択を通常資格のBroker IPC要求として送り、Rust起動器が検証済みsummaryとpayload hashをWindows native確認へ表示する。許可要求だけが既存process内Owner channelを通り、Brokerが再検証・Audit後、128-bit新規App IDを含むManifest fileを固定Export directoryへcreate-only生成する。Receiptには実fileのpath／SHA-256／byte長を含む。
  authority_boundary: Owner資格・privileged IPCはFlutterへ渡さない。書出し元のAuthority、Permission、Approval、Credential、Audit chainを継承せず、`authority_strip=true`と各`*_inherited=false`を固定する。Manifest内identity／Audit store IDは将来用の識別子で、実行可能packageや物理Runtime／Audit storeではない。Module選択は計画のみで、binary除去、独立App起動、Installer、署名、Distribution、rollbackは未成立の`release_blocker`として保持する。固定Export directory以外のpathを要求payloadから受け取らない。

- item: D4 Pocket Regression Case registration（C6 current scope）
  classification: required_for_v1
  status: implemented_for_current_scope
  evidence: Desktop対話paneは送信時に受け取った要求ID/hashを固定し、full表示・成功または保留結果・終了監査記録が揃う場合だけ登録面を提供する。ownerが明示記入したredacted定義をRust起動器の固定allowlist・native確認とBroker再照合へ通し、Windows ProtectedStoreのRegression purposeへ暗号化保存する。通常Broker IPCとDesktop回帰Case tabはreceipt・暗号文hash由来のmetadata-only一覧を表示する。Owner CLIとDesktop native確認付き削除／中断Recoveryは現在状態とAuditを再照合する。
  authority_boundary: 要求hashは相関値でありOwner確認ではない。登録payload、履歴、Profile、MCP metadata、Agent metadataは権限源ではない。元の対話本文を自動コピーせず、登録のnative確認には入力本文・条件・参照・期待経路を出さない。Known marker拒否は秘密不存在の証明ではない。Flutterは通常資格とOwnerが明示記入した定義だけを要求として送り、Owner資格・Owner roleは保持しない。登録／削除／中断照合のOwner扱いはRust起動器のoperation別native確認とBroker再検証に限る。private定義の閲覧、実行、C5自動import、物理消去は行わない。C5への明示importとWindows installed productでのOwner実操作証拠は未成立として扱う。

- item: D4 Pocket Credential Vault（C7 current scope）
  classification: required_for_v1
  status: implemented_for_current_scope
  evidence: owner controlの`資格情報登録`は新規資格情報をWindows ProtectedStoreのCredential purposeへDPAPI保管し、`資格情報一覧`は通常IPCへmetadata_onlyのreceiptだけを返す。同じIDの再登録、channel違反、保管欠落・改変はBrokerで拒否する。
  authority_boundary: 資格情報ID、metadata、暗号文hashはAuthority、Permission、Approvalを生成しない。秘密値はFlutter、snapshot、Audit、error、log、trace、CLI出力へ投影しない。秘密値の取得・Runtime／Tool／MCP／A2A注入、更新、失効、削除、接続先変更、GUI管理面は未成立として扱う。

- item: D4 Pocket MCP Connection Center（C9 current scope）
  classification: required_for_v1
  status: implemented_for_current_scope
  evidence: C8のMCP外部概念射影契約をRust Brokerのowner専用`MCP接続`／`MCP切断`へ接続した。Desktop設定のMCP接続センターはWindows絶対pathとstdio引数から接続要求を作り、既存Rust起動器のdefault No native Owner確認後にBrokerへ送る。通常IPCでmetadata一覧を明示取得し、Server ID・表示名・stdio種別・catalog件数を表示する。展開したTool一覧はBrokerの検証済みmetadataからTool名・Tool ID・入力Schema hashだけを表示し、説明文・Schema本文・実行操作は表示しない。個別切断もnative Owner確認後にBrokerへ届き、停止確認・永続Audit後に接続記録を解消する。
  authority_boundary: MCP接続と切断はBroker認証とWindows native Owner確認を通し、FlutterはOwner資格・session・Approval・Credential実値を保持しない。Tool catalogは`INTERNAL_STATE` metadataだけで、Trust・Permission・実行権を生成しない。MCP一覧は現在接続・Trustの保証へ昇格させない。MCP metadata、Tool description、Trust、Capability diff、Credential refはAuthority、Permission、Approval、Credential実値を生成しない。Credential入力とTool実行、Resource／Prompt実取得、Streamable HTTP、OAuth、consent、quarantine、非Windows process群停止は未成立として扱う。

- item: D4 Pocket 運用プロファイル（C10 current scope）
  classification: required_for_v1
  status: implemented_for_current_scope
  evidence: 設定画面からProfile一覧、作成、適用要求、export、削除を通常Broker IPCへ接続し、Rust BrokerがSchema検証済みProfileを永続化する。import契約もBroker経路に接続している。
  authority_boundary: Profileは再利用可能な要求設定だけであり、Permission、Approval、Authority、Credentialを生成しない。適用要求はhash照合とAudit記録に限定し、Runtime操作へ到達しない。

- item: D4 Pocket 更新センター（C11 current scope）
  classification: required_for_v1
  status: implemented_for_current_scope
  evidence: 通常Broker IPCから更新候補の一覧、Broker所有Ed25519 trustによる署名検査、署名済み候補の永続化、延期、download／適用／rollback要求をRust Update Centerへ接続し、Desktop設定画面へ表示した。
  authority_boundary: 更新候補の公開鍵、MCP metadata、Profile、履歴、UI stateは信頼源ではない。未署名・未信頼候補は保存せず、download、install、process、rollbackの外部実行はsuspendedのままとする。信頼設定未構成とWindows installed product証拠は未成立として扱う。

- item: D4 Pocket 通知センター（C12 current scope）
  classification: required_for_v1
  status: implemented_for_current_scope
  evidence: Rust Brokerはcaller登録ではなく監査eventのsource／decision／event hashからsummary通知を生成し、通常IPCの一覧・既読・破棄・全既読をDesktop通知画面へ返す。通知件数と監査走査をboundedにし、表示状態をhash結合して永続化する。
  authority_boundary: 通知はINTERNAL_STATEのnavigation-only表示であり、reason、payload、metadata、秘密値、Permission、Approval、Authorityを公開・生成しない。通知の開く操作はGUI navigationだけである。Windows native toastとinstalled product実証は未成立として扱う。

- item: D4 Pocket 観測センター（C13 current scope）
  classification: required_for_v1
  status: implemented_for_current_scope
  evidence: Rust BrokerはAudit確定処理の実測durationをboundedなSpan、Trace、Metricへ内部射影し、通常IPCの観測一覧とDesktop観測画面へ返す。保持1024Span、返却256Span／256Trace／16Metricで、TraceID filterと測定不能値のunknown境界を持つ。
  authority_boundary: 観測はINTERNAL_STATEの運用表示であり、Auditのreason、payload hash、metadata、秘密値を公開せず、Permission、Approval、Authority、Capabilityを生成しない。OpenTelemetry export、C14 Trace Inspector、Runtime全体の実測、installed product証拠は未成立として扱う。

- item: D4 Pocket Trace Inspector（C14 current scope）
  classification: required_for_v1
  status: implemented_for_current_scope
  evidence: C13の通常認証済み`観測一覧`を読み取り、Broker内部Spanの開始・終了・所要時間・状態・親Span・関連Audit・エラー分類をbounded waterfallで表示する。TraceID filter、更新、Broker接続なしのfail-closed表示を持つ。
  authority_boundary: 現在の実測対象はBrokerだけであり、Runtime、Adapter、Tool、外部通信を実測済みとして表示しない。Trace表示はINTERNAL_STATEに限定し、Auditのreason、payload、metadata、秘密値を公開せず、Permission、Approval、Authority、Capability、Credentialを生成しない。OpenTelemetry export、Runtime全体の実測、installed product証拠は未成立として扱う。

- item: Shell snapshot generator migration oracle
  classification: required_for_v1
  status: implemented
  evidence: python3 tooling/shell_snapshot.py --write .gui_shell/shell_snapshot.jsonは開発・移行検証用JSONを生成する。Flutterはこのfileを読み込まない。ShellCoreClient.local()は明示注入されたメモリ内ShellSnapshotだけを表示し、製品のmain.dartはShellCoreClient.product()とBroker IPCを使用する。
  authority_boundary: 生成JSONは外部の開発・移行検証資料であり、Flutterへの暗黙入力や権限源にしない。明示注入値も出所・鮮度unknown、release claim抑止の表示専用データとして扱う。installed product runtimeの依存関係にPython生成物を残してはならない。

- item: Evidence bundle export
  classification: required_for_v1
  status: implemented
  evidence: python3 tooling/evidence_bundle.py --checkは、bundleがWindows installed-path blockerを保持し、tooling/release_runtime_assertions.py --checkを埋め込み、release readinessを主張しないことを検証する。
  authority_boundary: evidence exportは読み取り専用かつ非権限的である。

- item: D4 Pocket Mobile projection（C24 + Phase 35 Agent status）
  classification: required_for_v1
  status: implemented_for_current_scope
  evidence: Mobileは既存Device Link TLSから、Agent Adapterの表示専用metadata、Runtime lifecycle状態、資源観測、通知summary、owner再承認待ち停止receipt、現在owner承認に結合した履歴metadataを既存Rust Brokerへ要求する。資源概要はunknownを0へ変換せず、MCPはowner専用Desktop管理面として未観測を表示する。
  authority_boundary: MobileはApproval、Permission、Authority、Credential、MCP接続、Tool実行、実停止を所有しない。Agent metadataを実task実行性・Trust・Permissionへ昇格させず、対話本文、Approval payload、Audit raw reason、Credential実値、Workspace pathを投影しない。Workspace選択面がない間、Agent Adapterの対話開始はBrokerがWorkspace未指定として拒否する。既存Broker handler以外のbridgeを追加しない。Mobile実機、TLS実接続、Android/iOS安全保管、Windows installed product証拠、長時間運用、障害注入は未成立として扱う。

- item: Release runtime assertions
  classification: required_for_v1
  status: implemented
  evidence: python3 tooling/release_runtime_assertions.py --checkは、製品Flutter entryがbroker IPCを使用し、製品pathがPythonを起動せずPython snapshot生成も呼び出さないこと、authority surface scanにFlutter/Rust FFIまたはdirect bridge tokenがないこと、broker secret tokenがUI snapshotへ投影されないこと、およびfail-closed / restart persistence test coverageが存在することを検証する。
  authority_boundary: assertion出力はvalidation evidenceに限る。完成製品リリースの前にはWindows installed-path runtime proofが引き続き必要である。

- item: D4 Pocket Phase 34 Windows staged Desktop起動器
  classification: release_blocker
  status: implemented_for_staged_launch_management
  evidence: staged product rootにWindows GUI subsystemのRust起動器を置き、同一process内で既存Rust Brokerを起動し、固定配置Flutter executableのMethodChannel要求をRust起動器owned PID-bound pipeからBrokerへrelayする。起動器はFlutter child環境をOS実行allowlistとpipe接続先札だけに制限する。2026-09-24のWindows staged smokeでは画面が実Broker snapshotまで初期化し、Broker操作と起動・終了Audit、endpoint cleanup、durable store保持を実測した。manifestはsource commit a3c8dc573afe609e8b0c5400584d2165a5b9a11aだがdirty worktreeと記録しており、clean-sourceまたはformal installed evidenceではない。詳細なartifact hashと限界はdocs/REV2_PROGRESS.mdのPhase 34 staged起動実測追補を参照。
  authority_boundary: UI／起動器はOwner資格、Permission、privileged Approvalを生成しない。Dartからendpoint file、Broker資格、Socket、networkへ直接到達せず、許可されたMethodChannel JSON要求だけをPID照合pipeでBrokerへ運ぶ。親processの任意環境変数もFlutterへ継承しない。staged manifestのcollector scratch runtimeと起動器のper-user runtimeは別pathで、後者は分離profileで未検証。Windows installed smoke collectorも直接Flutterを起動し、manual Developer stage runやRust環境分離unit testをformal installed evidenceへ昇格させない。clean productの正式channel／environment境界LIVE_RUNTIME証拠、formal Installer、Uninstaller、update、rollback、formal identity・署名は別blocker。
~~~

## 残存リリースブロッカー

~~~yaml
- item: Windows installed-path evidence
  classification: release_blocker
  reason: この環境にはrelease_evidence/windows_installed_smoke.jsonがまだ存在しない。
  required_action: native Windows installed-path evidenceを収集し、python tooling/windows_release_evidence.pyを通す。
  blocks_release: yes
~~~
