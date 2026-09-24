# GUI Shell の主張境界

## rev2の現在証拠（2026-09-10）

以下の旧Desktop v1.0記録は過去の基準面を含む。現行owner rev2ではMobile・端末連携・Android・Apple補助buildも作業対象であり、旧post_v1_scopeを理由に未完了を除外しない。現在の要求別監査は `docs/REV2_PROGRESS.md`、Mobile状態は `MOBILE_STATUS.md` を参照する。

macOS projectは追加済みで、手動補助run 34446194013（commit 27b8713fd1a9ecdb81abe1d4225b99b26bda84ba）でmacOS開発app・iOS Simulator appとMac Rust64単体・5統合がPASSした。「macOS validation environmentがない」という旧記録は、この補助buildの取得前を指す。実機での起動・安全保管・対話は未確認でありrelease_blockerである。実機の製品supportや完成を主張しない。

Android開発APK/AABはbuild済み。実機install以降、Windowsの中断された画面回帰、installed-path証拠、正式配布、owner GOはrelease_blockerとして保持する。開発検査のPASSはこれらのgateを解消しない。


## 現在の状態

GUI-Shell は、まだ完成製品の release ではない。

現在の claim は、Phase B の owner-use が完了した PC-first AI Runtime / Agent Operation Shell である。

Public review snapshot として tag を付けた GitHub Release は、完成製品の release ではない。この repository における完成製品の release readiness は、<code>release_blockers.registry.json</code> と明示的な owner GO によって引き続き gate される。

Repository definition の更新により、GUI-Shell は LLM が読む「アプリケーション責任基盤」としても文書化されている。これは、LLM 開発/統合エージェントが GUI Shell Contract を読み、第一級の実装・統合面として使うことを意図する。LLM は引き続き権限を持たず、Human operator が最終的な Approval、Recovery、責任、release claim の権限を保持する。

構築過程の注記: 本プロジェクトは、プログラマーでもソフトウェア開発者でもない個人が、1か月未満の兼業作業で LLM に指示しながら構築した。この構築は、LLM が読む責任基盤という設計目標の限定的な実証であり、release readiness、広範な interoperability、または外部 endorsement の証明ではない。

Windows-first の product responsibility と LLM-readable substrate の実証を整合させる canonical completion roadmap は、<code>docs/implementation/GUI_SHELL_LLM_SUBSTRATE_COMPLETION_ROADMAP.md</code> である。

個人向け Windows trial operation である Phase A は完了している。Windows desktop build と native launch smoke は owner-trial の履歴として通過した。ただし、この過去の PASS は、隔離された provenance／evidence-bundle contract と aggregate な native surface shortcut の禁止より前の経路であるため、現在の厳格な R2 formal evidence としては無効である。Phase B の owner-use completion も完了しており、owner は desktop shell を日常の local operation に使い、status、problem、evidence、Recovery、Trust、Runtime、Authority の各 surface を確認できる。External claim hygiene、実測した Windows release evidence、OSS の release candidate claim、paid／product QC は後の Phase に残る。

GUI-Shell v1.0 は Windows-first である。現在の host 上の Linux validation は development / verification slice として通過し得るが、それだけでは最終的な product proof ではない。macOS は未検証（<code>unverified</code>）の planned portability target であり、BLUE-TANUKI は GUI-Shell の release dependency ではなく consumer / Reference Runtime であり続ける。

GUI-Shell v1.0 は、検証済みの macOS support を主張しない。macOS は未検証（<code>unverified</code>）であり、macOS host の validation evidence がない状態で、macOS support を supported、ready、complete として宣伝してはならない。

LLM-readable substrate の定義、範囲を限定した Reference Extension の Conformance、および一件の限定的な cross-agent reproduction report は、権限を持たない task に対する統制された LLM-readable extension の behavior を限定的に実証する。public standard への採用、広範な第三者 interoperability、installed-product behavior、ecosystem readiness は証明しない。現在の Windows-first product の release blocker は一つも解消しない。

公開 Windows proof pack には、実測 Windows installed-path evidence に由来する redacted review copy が含まれる。これらは canonical release evidence ではなく、この公開 repository 上の完成製品 release blocker を解消しない。

## 現在完了している領域

~~~yaml
- item: schema と conformance skeleton
  classification: required_for_v1
  status: 現在の development validation は 188 の conformance check とともに通過している。過去の check count は <code>VALIDATION.txt</code> に履歴として保持する。Conformance の tautology は production の Authority Strip と ApprovalQueue の挙動を検査することで解消した。ghost invariant は production の InvariantEvaluator が測定する。Normalization Firewall の Conformance は PolicyEvaluator と Adapter metadata ingress を対象に含む。Broker IPC Contract、static な no-FFI / no-Python-spawn assertion、構造化された release blocker registry、release-facing blocker と文書の同期、packaging portability、および範囲を限定した LLM-readable extension の Contract / Conformance check を対象に含む。

- item: 範囲を限定した cross-agent LLM-readable extension reproduction
  classification: required_for_v1
  status: <code>docs/evidence/LLM_CROSS_AGENT_REPRODUCTION_REPORT.md</code> は、baseline <code>48082469089e9a63ef939b51f864dfc26e4ae2c9</code> から二つの独立した Agent execution が同じ bounded な <code>model_output</code> non-authority-source diff を生成し、validation が通過したことを記録する。これは統制された task に限定され、public standard への採用、広範な interoperability、installed-product behavior を証明しない。

- item: 個人向け Windows trial operation
  classification: required_for_v1
  status: Windows build と native launch smoke は owner trial use として通過したが、完成製品の release readiness を満たさない。

- item: Flutter local Shell Core client
  classification: required_for_v1
  status: <code>ShellCoreClient.local()</code> は構造化された local snapshot JSON を読み、direct mock alias ではなくなった。test / demo 用には mock mode を残す。

- item: GUI operation surface
  classification: required_for_v1
  status: Trust Center、Authority Map、Audit Timeline、Recovery Playbook、Adapter Catalog、Permission Diff、Problems Panel、Evidence Center、Settings UX、Command Palette、Status Bar の語彙が、Shell Core に接続された operator surface として存在する。

- item: Shell snapshot と evidence bundle
  classification: required_for_v1
  status: <code>tooling/shell_snapshot.py</code> は owner-use migration / development evidence 用の構造化された local GUI state を提供する。<code>tooling/evidence_bundle.py --check</code> は development evidence bundle を検証し、Windows installed-path と Audit anchor external tamper-evidence の blocker、および <code>release_ready=false</code> を維持する。snapshot generator を installed product runtime dependency として残してはならない。

- item: Shell Core hardening skeleton
  classification: required_for_v1
  status: Contract level の実装が存在する。

- item: Runtime Catalog skeleton
  classification: required_for_v1
  status: schema、fixture、package、Conformance が存在する。

- item: Agent Runtime skeleton
  classification: required_for_v1
  status: schema、fixture、package、Conformance が存在する。

- item: Rust helper boundary skeleton
  classification: required_for_v1
  status: Helper 実装と Rust Security Broker skeleton が存在する。2026-06-01 に <code>cd native/rust_helper && cargo test</code> が Broker JSON envelope / rejection test とともに通過した。

- item: Desktop Flutter skeleton
  classification: required_for_v1
  status: 実装が存在する。2026-05-25 に <code>cd apps/desktop_flutter && flutter analyze</code>、<code>flutter test</code>、<code>flutter build linux</code>、Linux launch smoke が通過した。

- item: Setup Doctor skeleton
  classification: required_for_v1
  status: 実装が存在する。

- item: release-hardening document
  classification: required_for_v1
  status: 実装が存在する。
~~~

## 現在の release blocker

~~~yaml
- item: 実装言語方針の Runtime convergence が未通過
  classification: release_blocker
  aggregate_of: windows_evidence_provenance_isolation, windows_installer_first_run_smoke, windows_broker_installed_smoke
  reason: Rust Security Broker skeleton と JSON envelope test は存在するが、現在の Shell Core の authority-sensitive behavior は <code>packages/shell_core/*.py</code> の Python 実装であり、owner-use snapshot generation / validation に使われている。production IPC transport、Flutter の Broker-mediated authority path、no-Python-runtime product evidence、no-FFI-authority release assertion は完了していない。
  required_action: authority-sensitive な active Runtime responsibility を Rust Security Broker へ移し、Python は dev / test / migration oracle だけに保つ。完成製品の release より前に、Flutter の Authority operation が Broker-mediated であることを証明する。
  blocks_release: yes

- item: Linux desktop build / launch smoke
  classification: required_for_v1
  reason: Linux desktop build smoke と launch smoke は 2026-05-25 に development / verification proof として通過した。
  required_action: Linux build / launch smoke を通過状態に保つが、Windows-first product proof として扱わない。
  blocks_release: no

- item: Windows Installer、first-run、Setup Doctor の release validation が未通過
  classification: release_blocker
  aggregate_of: windows_evidence_provenance_isolation, windows_installer_first_run_smoke, windows_setup_doctor_smoke, windows_broker_installed_smoke
  reason: Windows project support と過去の owner-trial launch smoke は保持されているが、現在の strict R2 evidence には、native Windows の新しい隔離 installed run が必要である。その run は source commit、clean worktree state、app / Broker artifact hash、evidence bundle hash、UIAutomation diagnostic tree、Broker の measured field provenance、installed app が生成した Setup Doctor product export を含まなければならない。product export path は存在するが、<code>release_evidence/windows_installed_smoke.json</code> がない。
  required_action: 隔離された staged run から native Windows installed smoke collection を実行し、measured window、visible-surface diagnostic tree、config JSON、Audit の write / read / delete、Broker IPC / restart / crash の field provenance、installed app が生成した Setup Doctor product evidence を収集する。その後 <code>python tooling\windows_release_evidence.py</code> を通過させる。
  blocks_release: yes

- item: GUI Shell書出しのOwner認可経路がDesktop UIに未接続
  classification: release_blocker
  registry_id: rev2_export_owner_ui_authority_path
  reason: Desktop ExportClientは通常Broker資格を使うが、Broker書出し操作はOwner制御資格を要求するため、現行GUIからの要求は拒否される。
  required_action: FlutterへOwner秘密値やprivileged IPCを渡さず、Owner明示操作をBrokerで再認証・監査する統治経路を実装し、通常資格拒否も検証する。
  blocks_release: yes

- item: Export Moduleの実binary除去と比較計測が未成立
  classification: release_blocker
  registry_id: rev2_module_pruning_binary_and_measurement
  reason: 現在のModulePlanはManifest上の選択だけで、artifactからの除去、サイズ、cold startup、resourceの実測を行っていない。
  required_action: 安全基盤を保持する実build経路を接続し、同一条件のpruned／unpruned Windows artifactをhash結合して比較測定する。
  blocks_release: yes

- item: macOS planned portability target が未検証
  classification: known_limitation
  reason: 現在利用できる macOS validation environment がないため、GUI-Shell v1.0 は検証済みの macOS support を主張しない。
  required_action: macOS support を主張する前に macOS host で検証する。
  blocks_release: no

- item: Windows Setup Doctor diagnostics が未通過
  classification: release_blocker
  registry_id: windows_setup_doctor_smoke
  reason: installed app は machine-readable な Setup Doctor product export を提供するが、その evidence は native Windows で収集されておらず、strict validator も通過していない。PowerShell の Setup Doctor collector は external probe evidence にすぎない。
  required_action: 隔離された Windows installed smoke を通じて app-generated Setup Doctor product export を収集し、<code>python tooling\windows_release_evidence.py</code> を通過させる。
  blocks_release: yes

- item: Audit anchor の external tamper-evidence proof がない
  classification: release_blocker
  registry_id: audit_anchor_external_tamper_evidence_proof
  reason: local HMAC Audit anchor は local corruption と partial tamper を検出するが、Windows ACL / DPAPI、external anchor、または signed evidence がなければ、same-user または administrator / root による rewrite への耐性を証明できない。
  required_action: Windows installed-path の Audit anchor key-protection または external-anchor proof を収集し、strict Windows release validation を通過させる。
  blocks_release: yes

- item: implementation first-run smoke
  classification: required_for_v1
  reason: implementation first-run smoke は config / Audit path を作成し、Installer / setup state が権限を持たないことを検証する。
  required_action: implementation first-run smoke を通過状態に保つ。native Windows installed-path first-run は引き続き release blocker である。
  blocks_release: no

- item: Shell Core persistence smoke
  classification: required_for_v1
  reason: 統合された Shell Core release smoke は state snapshot を保存・読込する。
  required_action: release candidate で persistence smoke を通過状態に保つ。
  blocks_release: no

- item: Audit chain verification smoke
  classification: required_for_v1
  reason: 統合された Shell Core release smoke は Audit chain linkage、HMAC Audit anchor verification、tamper detection を検証する。
  required_action: release candidate で Audit chain / local anchor smoke を通過状態に保つ。
  blocks_release: no

- item: Runtime Catalog live/use smoke
  classification: required_for_v1
  reason: release smoke は RuntimeCatalog を介して Runtime / Adapter manifest を登録し、Catalog の authority が false のままであることを確認する。
  required_action: Runtime Catalog smoke を通過状態に保つ。
  blocks_release: no

- item: Agent Runtime mock/reference smoke
  classification: required_for_v1
  reason: release smoke は workspace boundary、secret path denial、shell command の Permission mapping、auditable diff behavior を検証する。
  required_action: Agent Runtime reference smoke を通過状態に保つ。
  blocks_release: no

- item: Strict release validation が未通過
  classification: release_blocker
  aggregate_of: windows_evidence_provenance_isolation, windows_installer_first_run_smoke, windows_setup_doctor_smoke, windows_broker_installed_smoke, audit_anchor_external_tamper_evidence_proof, owner_go
  reason: 完成した Windows-first product release には Windows strict validation が必要である。
  required_action: <code>python3 tooling/validate_all.py --strict-release --desktop-platform=windows</code> を通過させる。<code>--desktop-platform=all</code> は macOS が未検証であるため失敗し得るが、そのことは Windows-first v1.0 を block しない。
  blocks_release: yes

- item: Owner GO がない
  classification: release_blocker
  registry_id: owner_go
  reason: release claim の promotion には owner approval が必要である。
  required_action: 明示的な owner GO を取得する。
  blocks_release: yes
~~~

## v1 後の範囲

~~~yaml
- item: Mobile の full release
  classification: post_v1_scope
  reason: owner が明示的に mobile を含めない限り、v1.0 の scope は Windows-first PC desktop である。
  required_action: v1.0 後に完了するか、owner instruction によって scope を更新する。
  blocks_release: no

- item: multi-user mode
  classification: post_v1_scope
  reason: v1.0 は single-user である。
  required_action: post-v1 planning まで延期する。
  blocks_release: no

- item: Cloud Service
  classification: post_v1_scope
  reason: v1.0 は local-first である。
  required_action: post-v1 planning まで延期する。
  blocks_release: no

- item: Marketplace
  classification: post_v1_scope
  reason: v1.0 は Runtime marketplace distribution を含まない。
  required_action: post-v1 planning まで延期する。
  blocks_release: no

- item: Enterprise administration
  classification: post_v1_scope
  reason: v1.0 は single-user desktop である。
  required_action: post-v1 planning まで延期する。
  blocks_release: no

- item: 完全な live third-party Agent integration
  classification: post_v1_scope
  reason: v1.0 に必要なのは汎用 Agent Runtime Contract と mock / Reference Agent であり、すべての live Adapter ではない。
  required_action: v1.0 後に Adapter work として追加する。
  blocks_release: no

- item: BLUE-TANUKI の product completion
  classification: post_v1_scope
  reason: BLUE-TANUKI は consumer / Reference Runtime であり、GUI-Shell の release gate ではない。
  required_action: GUI-Shell v1.0 gate の後に consumer integration として完了する。
  blocks_release: no
~~~

## 既知制約

~~~yaml
- item: local single-user のみ
  classification: known_limitation
  reason: v1.0 product scope は Windows-first PC desktop、single-user、local-first である。
  required_action: README、CLAIM、RELEASE_CHECKLIST の整合を維持する。
  blocks_release: no
~~~
