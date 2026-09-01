# GUI操作面

状態基準日: 2026-05-26

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
