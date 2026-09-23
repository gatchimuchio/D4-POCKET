# GUI操作面

状態基準日: 2026-09-23

GUI-ShellのGUI堅牢化では、権限をFlutterへ移さず、実証済みの操作パターンを取り込む。Flutterは状態と操作者の意図を表す操作面を描画し、Shell Coreは引き続き権限境界を担う。

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
  evidence: Agent Adapter Schemaはidentity、provider、version、model、capability宣言、workspace、tool／MCP／session／cancellation／metrics、認証方式、Host要件を検査し、未対応機能の理由を要求する。
  authority_boundary: Adapter宣言は対応状況の説明であり、Permission、Approval、trust、credential実値を生成しない。実物interface未確認のLauncherはunsupportedとして扱う。

- item: D4 Pocket Agent Launcher（Codex current scope）
  classification: required_for_v1
  status: implemented_for_current_scope
  evidence: ownerがBroker起動時に絶対executableとworkspaceを明示したCodex CLIだけをRust Adapterへ登録できる。登録時にversionと`codex exec --help`をLIVE_RUNTIMEで確認し、既存の実行系対話面から固定read-only JSONL実行、bounded output、取消、期限超過、失敗射影を通す。Windows実Brokerの認証付き通常IPCで`codex`実行系の列挙を確認した。
  authority_boundary: Flutterはexecutable、workspace、argv、environment、Permissionを指定せず、Approvalを自己承認しない。Adapterは`--sandbox read-only`、`--ephemeral`、environment allowlist、secret path拒否を強制し、汎用command dispatchを有効化しない。実taskのwrite実行、MCP、複数Agent比較、Handoff、Claude／Gemini接続は未成立として扱う。

- item: D4 Pocket Regression Case registration（C6 current scope）
  classification: required_for_v1
  status: implemented_for_current_scope
  evidence: owner controlの`回帰Case登録`は、完了済み通常対話の要求ID/hash、表示範囲、永続結果証跡、終了監査IDをRust Broker内で再照合し、owner明示のredacted定義をWindows ProtectedStoreのRegression purposeへ暗号化して保存する。公開receiptはhash-onlyで、入力本文・条件・参照・期待経路を返さない。
  authority_boundary: 登録payloadの入力方式、履歴、Profile、MCP metadata、Agent metadataは権限源ではない。元の対話本文を自動コピーせず、Known marker拒否は秘密不存在の証明ではない。Flutterのprivate登録画面、Case一覧、削除Recovery、C5への自動import、Windows実機登録証拠は未成立として扱う。

- item: D4 Pocket Credential Vault（C7 current scope）
  classification: required_for_v1
  status: implemented_for_current_scope
  evidence: owner controlの`資格情報登録`は新規資格情報をWindows ProtectedStoreのCredential purposeへDPAPI保管し、`資格情報一覧`は通常IPCへmetadata_onlyのreceiptだけを返す。同じIDの再登録、channel違反、保管欠落・改変はBrokerで拒否する。
  authority_boundary: 資格情報ID、metadata、暗号文hashはAuthority、Permission、Approvalを生成しない。秘密値はFlutter、snapshot、Audit、error、log、trace、CLI出力へ投影しない。秘密値の取得・Runtime／Tool／MCP／A2A注入、更新、失効、削除、接続先変更、GUI管理面は未成立として扱う。

- item: D4 Pocket MCP Contract（C8 current scope）
  classification: required_for_v1
  status: contract_only
  evidence: MCP Server、Tool、Resource、Prompt、Transport、Credential ref、Trust、Capability diffを`mcp_contract.schema.json`へ射影し、正常／権限混入／秘密値混入／Approval混入のfixtureをConformanceで検査する。
  authority_boundary: MCP metadata、Tool description、Trust、Capability diff、Credential refはAuthority、Permission、Approval、Credential実値を生成しない。discovery、connect、consent、Tool実行、timeout、disconnect、quarantineは未成立として扱う。

- item: Shell snapshot generator migration oracle
  classification: required_for_v1
  status: implemented
  evidence: python3 tooling/shell_snapshot.py --write .gui_shell/shell_snapshot.jsonは、development / inspection modeだけでShellCoreClient.local()が利用するlocal diagnostic JSONを生成する。製品のmain.dartはShellCoreClient.product()とbroker IPCを使用する。
  authority_boundary: snapshot生成は所有者用のmigration / development evidenceとしてShell CoreおよびSetup Doctorの状態を記録する。権限を付与せず、installed product runtimeの依存関係として残してはならない。

- item: Evidence bundle export
  classification: required_for_v1
  status: implemented
  evidence: python3 tooling/evidence_bundle.py --checkは、bundleがWindows installed-path blockerを保持し、tooling/release_runtime_assertions.py --checkを埋め込み、release readinessを主張しないことを検証する。
  authority_boundary: evidence exportは読み取り専用かつ非権限的である。

- item: Release runtime assertions
  classification: required_for_v1
  status: implemented
  evidence: python3 tooling/release_runtime_assertions.py --checkは、製品Flutter entryがbroker IPCを使用し、製品pathがPythonを起動せずPython snapshot生成も呼び出さないこと、authority surface scanにFlutter/Rust FFIまたはdirect bridge tokenがないこと、broker secret tokenがUI snapshotへ投影されないこと、およびfail-closed / restart persistence test coverageが存在することを検証する。
  authority_boundary: assertion出力はvalidation evidenceに限る。完成製品リリースの前にはWindows installed-path runtime proofが引き続き必要である。
~~~

## 残存リリースブロッカー

~~~yaml
- item: Windows installed-path evidence
  classification: release_blocker
  reason: この環境にはrelease_evidence/windows_installed_smoke.jsonがまだ存在しない。
  required_action: native Windows installed-path evidenceを収集し、python tooling/windows_release_evidence.pyを通す。
  blocks_release: yes
~~~
