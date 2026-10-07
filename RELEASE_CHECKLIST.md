# リリースチェックリスト

## rev5 Final QA既知制約（2026-10-07）

- item: Desktop Flutter analyzerのdeprecated API情報
  classification: known_limitation
  reason: Flutter 3.44.0の全体解析は`apps/desktop_flutter/lib/screens/agent_center.dart`の`RadioListTile.groupValue/onChanged`と`DropdownButtonFormField.value`に関するdeprecated API info 5件でexit 1。Desktop全204 Widget testとWindows Release compileは成功し、この診断によるruntime failureは観測していない。
  required_action: 現行Flutterが当該APIを削除する前に`RadioGroup`／`initialValue`へ移行し、Desktop analyzerを再実行する。
  blocks_release: no

## rev2の現在証拠（2026-09-10）

以下の旧Desktop v1.0記録は過去の基準面を含む。現行owner rev2ではMobile・端末連携・Android・Apple補助buildも作業対象であり、旧post_v1_scopeを理由に未完了を除外しない。現在の要求別監査は `docs/REV2_PROGRESS.md`、Mobile状態は `MOBILE_STATUS.md` を参照する。

macOS projectは追加済みで、手動補助run 34446194013（commit 27b8713fd1a9ecdb81abe1d4225b99b26bda84ba）でmacOS開発app・iOS Simulator appとMac Rust64単体・5統合がPASSした。「macOS validation environmentがない」という旧記録は、この補助buildの取得前を指す。実機での起動・安全保管・対話は未確認でありrelease_blockerである。実機の製品supportや完成を主張しない。

Android開発APK/AABは過去にbuild済みである。現行Mobile sourceは隔離Tempのfresh copyでclean・unit test・debug APK buildを通過した。実機install以降、Windowsの中断された画面回帰、installed-path証拠、正式配布、owner GOはrelease_blockerとして保持する。開発検査のPASSはこれらのgateを解消しない。

- item: OneDrive内Android build出力のACL制約
  classification: known_limitation
  reason: このWindows hostのOneDrive workspaceではignored `apps/mobile_flutter/build`配下へ`Everyone Deny DeleteSubdirectoriesAndFiles`が継承され、Gradle標準cleanupとresource packagingが拒否される。現行Android／共有UI source 117 fileをhash照合したfresh Temp copyではcleanup除外なしのclean・unit test・APK buildが成功した。製品source問題またはrelease失敗の証拠ではない。
  required_action: workspace pathでのin-place buildが必要な場合に限り、ACL管理者が継承元と範囲を確認して修正する。通常の検証はhash照合済みisolated Temp copyで実行できる。Codexはowner承認なしにACLを変更しない。
  blocks_release: no


このリポジトリで「release」とは、completed product release を意味する。skeleton、preview、alpha、beta、scaffold、contract-preview の状態は release state ではない。

public review snapshot として tag を付けた GitHub Release は、completed product release ではない。このリポジトリにおける完成製品の release readiness は、引き続き <code>release_blockers.registry.json</code> と明示的な owner GO を gate とする。

Windows-first v1.0 の <code>release_blocker</code> が一つでも残っている場合、completed product release を主張してはならない。GUI-Shell v1.0 は Windows-first である。Windows が第一対象、Linux が検証済みの development／verification slice、macOS が未検証の planned portability target である。

公開 Windows proof pack asset は、実測した Windows installed-path evidence から作成した redacted review copy である。canonical release evidence ではなく、この公開リポジトリ上の completed product の release blocker を閉じない。

GUI-Shell v1.0 は、検証済みの macOS support を主張しない。macOS は未検証（<code>unverified</code>）であり、macOS host で検証するまでは、macOS support を supported、ready、complete と宣伝してはならない。

Phase A の personal Windows trial operation は完了している。Phase B の owner-use operational hardening も完了している。この checklist は completed product の release gate のままであり、Phase B を理由に弱めてはならない。

Windows-first product path と LLM-readable substrate の demonstration path を統合した完了ロードマップの正本は <code>docs/implementation/GUI_SHELL_LLM_SUBSTRATE_COMPLETION_ROADMAP.md</code> である。

## リリース阻害項目（release blocker）

~~~yaml
- item: D4 Pocket rev2 Mobile Device Link Flutter境界が未成立
  classification: release_blocker
  registry_id: rev2_mobile_flutter_native_device_link_boundary
  reason: Dart直接経路は廃止し、Android Kotlin／iOS Swiftのnative招待・Keychain／Keystore・TLS経路とnative Workspace選択面を実装済み。iOS Simulator buildとnative XCTest 8件は手動Actions run #22でPASSしたが、実端末のOS保管・実TLS・lifecycle、Brokerとのnative LIVE_RUNTIME接続は未確認。過去Dart経路やSimulator fixtureだけではproduction経路の成立を証明しない。
  required_action: 秘密をDart/debug VM/log/artifactへ渡さないplatform-native harnessでBroker到達・拒否・失効・background停止を検証し、Workspace識別子の限定開示とBrokerのRuntime再照合を確認する。Android実機凍結・物理端末証拠・正式配布gateは別途維持する。
  blocks_release: yes

- item: D4 Pocket 更新取得のDNS期限・破損package Recovery・installed updateが未成立
  classification: release_blocker
  registry_id: rev2_desktop_product_distribution
  reason: native Owner確認後のlocal TLS download workerは実装したが、OS同期DNS resolverはtimeout／cancel不能であり、同digest名の破損packageは安全なrepair経路がない。実配布元を使うWindows installed update、install／rollback transaction、crash Recoveryも未検証。
  required_action: DNS resolutionを有限期限・cancel可能にし、破損cacheのAudit付きrecoveryを実装してから、Windows installed productで外部source download、tamper、same-volume install／rollback、失敗注入を検証する。
  blocks_release: yes

- item: D4 Pocket更新取得はsystem proxyを使わない
  classification: known_limitation
  reason: 現行Broker downloaderはproxy経由の権限・接続先変化を避けるためsystem proxyを明示的に無効化し、直接HTTPS接続だけを許可する。proxy必須networkでは取得できない。
  required_action: v1でproxy環境が必要な配布対象に含まれる場合、Broker所有のproxy trust／表示／AuditとSSRF・credential・failure境界を定義し、実proxy試験後に対応を有効にする。それまでは直接HTTPSが使えない環境を非対応として案内する。
  blocks_release: no

- item: language policy runtime convergence gate
  classification: release_blocker
  aggregate_of: windows_evidence_provenance_isolation, windows_installer_first_run_smoke, windows_broker_installed_smoke
  reason: 認証付き loopback IPC、永続 audit／replay／session store、Rust authority parity operation、Flutter product broker client code、release runtime static assertion を含む Rust Security Broker production IPC は存在する。product の <code>main.dart</code> は <code>ShellCoreClient.local()</code> ではなく <code>ShellCoreClient.product()</code> を使用する。<code>tooling/release_runtime_assertions.py --check</code> は、現在の product authority surface が Python process を起動せず broker IPC を使用し、no-ffi-authority direct-bridge assertion を満たすことを検証する。Windows Flutter analyze／test は <code>flutter.bat</code> 経由で通過した。しかし command dispatch は SUSPEND のままで、broker health は現在も <code>authority_cutover_status=not_active</code> を報告する。外部 Flutter SDK shell script が CRLF line ending であるため WSL から直接実行する <code>flutter</code> は引き続き失敗し、installed no-Python-runtime product evidence と Windows installed-path broker evidence は未完了である。
  required_action: <code>docs/implementation/RUST_SECURITY_BROKER_MIGRATION_PLAN.md</code> の migration plan を完了し、installed product runtime では Python が dev／test／migration oracle のみに限定されることを実証し、broker-mediated Windows installed-path evidence を収集し、strict release validation を再実行する。
  blocks_release: yes

- item: desktop flutter analyze gate
  classification: required_for_v1
  reason: completed desktop-first v1.0 release には desktop Flutter analyze が必要である。<code>unzip</code> が利用可能になった後、2026-05-25 の現在の run は通過した。
  required_action: release candidate で <code>cd apps/desktop_flutter && flutter analyze</code> を通過させる。
  blocks_release: no

- item: Linux desktop build dependencies gate
  classification: required_for_v1
  reason: development／verification slice では Rust／Cargo、Flutter、<code>unzip</code>、Linux desktop build dependency が解決済みである。<code>flutter doctor -v</code> は clang 21.1.8、cmake 4.2.3、ninja 1.13.2、pkg-config 2.5.1 を報告する。
  required_action: development validation 用に Linux desktop build dependency の導入状態を保つ。Linux を最終 Windows-first product proof として扱わない。
  blocks_release: no

- item: Linux desktop project configuration gate
  classification: required_for_v1
  reason: Linux desktop project support は設定済みで、<code>cd apps/desktop_flutter && flutter build linux</code> は 2026-05-25 に通過し、<code>build/linux/x64/release/bundle/gui_shell_desktop</code> を生成した。
  required_action: development／verification slice として Linux build smoke を通過状態に保つ。
  blocks_release: no

- item: Linux desktop launch smoke gate
  classification: required_for_v1
  reason: <code>./build/linux/x64/release/bundle/gui_shell_desktop</code> は 2026-05-25 に WSLg 上で正常に起動し、最初の window には Dashboard、NavigationRail、Runtime Status、Invariant Status が表示された。
  required_action: Linux desktop launch smoke を通過状態に保つ。ただし、product release の前に Windows launch smoke を完了する。
  blocks_release: no

- item: WSLg libEGL/MESA graphics warnings
  classification: known_limitation
  reason: Linux desktop launch 中に WSLg が libEGL／MESA warning を出力したが、rendering と first-window stability は失敗しなかった。
  required_action: release-facing document に記録し続け、rendering または stability が失敗した場合は <code>release_blocker</code> に再分類する。
  blocks_release: no

- item: Windows desktop project support generated
  classification: required_for_v1
  reason: <code>flutter create --platforms=windows .</code> は、既存の <code>lib/</code> app code を上書きせずに <code>apps/desktop_flutter/windows</code> を生成した。
  required_action: Windows Flutter desktop project file を version control 配下に保つ。
  blocks_release: no

- item: conformance tautology fix
  classification: required_for_v1
  reason: authority stripping、approval edit guard、approval status、recovery ID の conformance check は現在、production Shell Core code を呼び出して通過する。mutation verification では、production authority strip または approval guard を弱めると conformance が失敗することを確認した。
  required_action: conformance test では production implementation を import し続ける。test-local な authority stripping または approval edit guard の copy を再導入しない。この surface を変更した場合は <code>docs/MUTATION_VERIFICATION.md</code> を更新する。
  blocks_release: no

- item: ghost invariant measurement
  classification: required_for_v1
  reason: state snapshot の invariant flag は、静的な false value ではなく、production の <code>InvariantEvaluator</code> による計測 check から得られる。
  required_action: invariant flag を計測値に保ち、invariant surface を変更した場合は意図的な違反を mutation-test する。
  blocks_release: no

- item: normalization firewall
  classification: required_for_v1
  reason: Shell Core は authority-bearing payload を authority strip の前に正規化する。PolicyEvaluator、AdapterLoader、RuntimeCatalog、BLUE-TANUKI authority trace は共有 normalization scanner を使用する。conformance は Unicode、大文字小文字、zero-width、alias、envelope、value-only の権限昇格試行を扱う。
  required_action: authority-bearing ingress path で、raw payload preservation、normalized projection、quarantine decision、normalization audit metadata、metadata value-only rejection を維持する。
  blocks_release: no

- item: Flutter local Shell Core client
  classification: required_for_v1
  reason: <code>ShellCoreClient.local()</code> は structured local snapshot JSON を読み取り、direct mock alias ではなくなった。mock mode は test／demo data 用として分離されたままである。
  required_action: Flutter test で local snapshot loading を引き続き扱い、release candidate では fallback diagnostics を installed app data に置き換える。
  blocks_release: no

- item: GUI operation surfaces
  classification: required_for_v1
  reason: desktop Flutter は、authority を Flutter へ移さずに Trust Center、Authority Map、Audit Timeline、Recovery Playbook、Adapter Catalog、Permission Diff、Problems Panel、Evidence Center、Settings UX、Command Palette、Status Bar の operation vocabulary を公開する。
  required_action: GUI surface を read-only または Shell Core-authorized に保ち、対応する conformance／evidence coverage を伴う場合に限って拡張する。
  blocks_release: no

- item: Shell snapshot generator migration oracle
  classification: required_for_v1
  reason: <code>tooling/shell_snapshot.py</code> は owner-use migration と development evidence のために <code>ShellCoreClient.local()</code> が消費する structured local snapshot を生成する。そこには trust、authority、evidence、settings、Setup Doctor、audit、recovery、non-authoritative installer status が含まれる。これは installed product runtime dependency として残してはならない。
  required_action: migration 中は snapshot generation と Flutter model field／Shell Core authority boundary の整合を保ち、その後 completed product release の前に product runtime dependency を broker-mediated state へ置き換える。
  blocks_release: no

- item: evidence bundle export
  classification: required_for_v1
  reason: <code>tooling/evidence_bundle.py --check</code> は development evidence bundle を検証する。この bundle は Windows installed-path blocker を保持し、<code>release_ready=false</code> を維持し、broker-mediated Flutter authority、Python authority process startup の不在、FFI authority bridge の不在に関する release runtime assertion を埋め込む。
  required_action: Windows installed-path evidence と owner GO が通過するまで、evidence bundle export を non-authoritative に保つ。
  blocks_release: no

- item: no-Python runtime / no-FFI authority assertion
  classification: required_for_v1
  reason: <code>tooling/release_runtime_assertions.py --check</code> は <code>tooling/validate_all.py</code> の一部であり、product の <code>main.dart</code> が <code>ShellCoreClient.product()</code> に入り、Flutter authority operation が broker-mediated で、owner launch script が Python snapshot generation なしに <code>broker-server</code> を起動し、Flutter lib が authority のための Dart process-spawn API を使用せず、broker secret が UI snapshot に投影されず、authority surface scan に Flutter／Rust FFI または direct bridge token が現れないことを検証する。
  required_action: release runtime assertion を通過状態に保ち、authority-sensitive product surface を追加するたびに拡張する。
  blocks_release: no

- item: duplicate authority key definitions
  classification: required_for_v1
  reason: <code>packages/shell_core/authority_keys.py</code> は <code>AUTHORITY_KEYS</code> の唯一の production source である。authority key definition の重複が残る場合は <code>release_blocker</code> となる。
  required_action: production module が <code>packages.shell_core.authority_keys.AUTHORITY_KEYS</code> を import する状態を保つ。
  blocks_release: no

- item: Windows Flutter analyze gate
  classification: required_for_v1
  reason: native Windows host で過去の Flutter analyze は通過したが、strict R2 release evidence には、正確な implementation commit と結び付いた現在の release-candidate validation が必要である。
  required_action: Windows release candidate で <code>cd apps/desktop_flutter && flutter analyze</code> を通過状態に保ち、release promotion の前に現在 run の provenance を記録する。
  blocks_release: no

- item: Windows Flutter test gate
  classification: required_for_v1
  reason: native Windows host で過去の Flutter test は通過したが、strict R2 release evidence には、正確な implementation commit と結び付いた現在の release-candidate validation が必要である。
  required_action: Windows release candidate で <code>cd apps/desktop_flutter && flutter test</code> を通過状態に保ち、release promotion の前に現在 run の provenance を記録する。
  blocks_release: no

- item: Windows Flutter toolchain verified
  classification: required_for_v1
  reason: native Windows の Flutter analyze、test、build、launch smoke は owner-trial use のために過去に通過した。新しい exact-commit Windows run が記録されるまで、現在の strict R2 formal evidence としては無効である。
  required_action: release candidate で Windows Flutter toolchain validation を現行に保ち、isolated evidence run と結び付ける。
  blocks_release: no

- item: Windows desktop build smoke
  classification: required_for_v1
  reason: <code>flutter build windows</code> は native Windows host で通過し、<code>build\windows\x64\runner\Release\gui_shell_desktop.exe</code> を生成した。
  required_action: release candidate で Windows desktop build smoke を通過状態に保つ。
  blocks_release: no

- item: Windows desktop launch smoke
  classification: required_for_v1
  reason: <code>.\build\windows\x64\runner\Release\gui_shell_desktop.exe</code> は、過去の owner-trial evidence として native Windows 上で正常に起動した。aggregate native surface exposure と exact-run provenance の欠如が禁止されるため、この旧 launch smoke は現在の strict R2 proof では無効である。
  required_action: release candidate で Windows desktop launch smoke を通過状態に保ち、その後 isolated installed run から surface ごとの UIAutomation／accessibility evidence を再収集する。
  blocks_release: no

- item: R2 Windows formal evidence path reset
  classification: release_blocker
  aggregate_of: windows_evidence_provenance_isolation, windows_installer_first_run_smoke, windows_setup_doctor_smoke, windows_broker_installed_smoke
  reason: 現在の strict Windows evidence には、isolated run provenance、source commit、clean worktree state、app／broker artifact hash、evidence bundle hash、field provenance、完全な UIAutomation diagnostic tree、計測した broker IPC／restart／crash field、installed-app generated Setup Doctor product export が必要である。過去の PASS と external probe report はこの gate では無効である。
  required_action: 再設計した Windows evidence collection path を完了し、native Windows 上で strict Windows validation を実行する。
  blocks_release: yes

- item: Windows installer first-run smoke not passed
  classification: release_blocker
  registry_id: windows_installer_first_run_smoke
  reason: DiagnosticOnly installed runでは初回設定の未作成→生成、read-only取得accepted Audit、設定hash一致を観測したが、stage時と異なるWindows user profileによるformal evidenceはなく、strict R2 evidenceにも記録されていない。
  required_action: stage時と異なるWindows user profile（SID）から <code>collect_installed_smoke.ps1 -DesktopLauncherExe -UseCurrentWindowsProfile -NoPythonRuntime</code> を実行し、起動器の実runtime、lifecycle Audit、通常資格で受理されたhealth要求Audit、endpoint cleanup、UIAutomationを収集する。health Auditはclient応答受信やPID帰属を証明しない。clean-source実測と初回config生成が成立するまではrelease blockerを維持し、<code>python tooling\windows_release_evidence.py</code>を通過させる。
  blocks_release: yes

- item: GUI Shell書出しのOwner UI LIVE_RUNTIME証拠が未成立
  classification: release_blocker
  registry_id: rev2_export_owner_ui_authority_path
  reason: Rust起動器のnative Owner確認、process内allowlist、Broker再検証、通常資格拒否の自動試験とcreate-only Manifest file生成は実装済み。Schema・hash・byte長を再照合してunsigned Windows bundleを組む開発toolも追加した。clean pushed commitからの試行ではFlutter Releaseは成功したが、Cargo build scriptがWindows Application ControlのOS error 4551で拒否されbundle未生成。host policyは変更・回避しない。Receiptの出所・Owner操作は認証せず、Credential artifact scanは未実施、runtime Manifest消費・起動・formal installed-path evidenceも未確認。
  required_action: Windows Application Controlを弱めずにCargo build scriptを実行できる承認済みWindows環境で、clean-sourceのportable bundle構築を完遂する。clean-sourceのinstalled productをformal collectorで起動し、Owner No拒否と永続Auditを再確認する。実際のBroker生成Receipt／Manifestから作る全package hashと製品別runtime／Audit storeを検証し、Credential非露出を監査する。Ownerが明示GOした場合だけnative Yesを操作し、生成fileのpath／hash、完了Audit、Authority／Credential非継承を同じsource commitと全package hashへ結合して検証する。
  blocks_release: yes

- item: Export Moduleの実binary除去と比較計測が未成立
  classification: release_blocker
  registry_id: rev2_module_pruning_binary_and_measurement
  reason: 開発Export toolは選択Module defineをunsigned Windows bundleへ接続するが、選択記録やAOT node差、bundle生成はbinary pruningの証拠ではない。最終artifactの意味上の除去、安全Core保持、Rust／第三者module境界、製品起動・性能は未検証。
  required_action: clean sourceから同一Windows／toolchain条件のall-enabledと選択bundleを作り、hash固定artifact上で必須安全Core保持と除外対象のpositive／negative／回帰実挙動を検証する。製品サイズ、cold startup、bounded resourceも比較測定する。
  blocks_release: yes

- item: Windows Setup Doctor smoke not passed
  classification: release_blocker
  registry_id: windows_setup_doctor_smoke
  reason: Flutter 3.44.0のRelease診断では明示的IAccessibleEx選択後にUIAutomation tree 121 nodeと必須4 surface、および通常tray終了を観測した。ただしDiagnosticOnlyの同一SID実行であり、Setup Doctor画面そのもののoperator readability、別profileのformal evidence、strict validator合格は未成立。
  required_action: collector v16をclean-source Windows Releaseとstage時と異なるuser profileで実行し、MainWindowへ結合したSetup Doctor UIAutomation text／topmost sample、config／report hash、accepted AuditEvent、通常終了、cleanup errorなしを同一bundleで収集して<code>python tooling\windows_release_evidence.py</code>へ通す。v16実UIA、pixel／contrast、screen readerの未実施範囲は合格へ昇格せず、Flutter engine更新時も互換性を再確認する。
  blocks_release: yes

- item: Flutter Windows UIAのexperimental IAccessibleEx依存
  classification: known_limitation
  registry_id: windows_flutter_iaccessible_experimental_mode
  reason: 必須surfaceのUIA公開にFlutter Windows Embedderのexperimental IAccessibleEx modeを使う。Release treeはFlutter 3.44.0でのみ今回実測し、他engine版および実screen readerは未確認。
  required_action: 配布対象Flutter engine版を固定・文書化し、engine更新時にWindows UIA surface、tray終了、実screen readerを再検証する。
  blocks_release: no

- item: macOS planned portability target unverified
  classification: known_limitation
  reason: 現在利用できる macOS validation environment がないため、GUI-Shell v1.0 は検証済み macOS support を主張しない。
  required_action: macOS が unverified の間は support を supported、ready、complete と主張せず、macOS host で検証する。
  blocks_release: no

- item: Windows installed-path evidence validator
  classification: required_for_v1
  reason: <code>tooling/windows_release_evidence.py</code> は現在、installed executable hash、正確な source commit provenance、clean worktree state、isolated run path、app／broker artifact hash linkage、evidence bundle hash、field provenance、installed Flutter <code>.exe</code> launch evidence、broker-mediated first-run endpoint evidence、No-Python launch evidence、非ゼロ window handle、visible-surface source と diagnostic tree、first-run config JSON parsing、audit write／read／delete probe、installed-app generated Setup Doctor product evidence、broker authenticated IPC／restart／crash の計測 field provenance を検証する。
  required_action: evidence validation を fail-closed に保ち、copied、edited、synthetic、manually confirmed、shallow、aggregate-surface、non-Windows、external-probe-as-product、unmeasured-declaration の evidence を拒否する。
  blocks_release: no

- item: Windows Setup Doctor diagnostics evidence not passed
  classification: release_blocker
  registry_id: windows_setup_doctor_smoke
  reason: installed-app generated Windows Setup Doctor product evidence は、Windows-first product target で未通過である。external probe evidence はこの gate では無効である。
  required_action: <code>collect_installed_smoke.ps1</code> から Windows Setup Doctor product export evidence を通過させる。macOS diagnostics は planned portability validation のままである。
  blocks_release: yes

- item: validate_all.py strict release mode not passed
  classification: release_blocker
  aggregate_of: windows_evidence_provenance_isolation, windows_installer_first_run_smoke, windows_setup_doctor_smoke, windows_broker_installed_smoke, audit_anchor_external_tamper_evidence_proof, owner_go
  reason: current-host Linux validation が通過しても、completed product release の前には Windows-first strict release mode が release blocker を報告しない状態でなければならない。
  required_action: <code>python3 tooling/validate_all.py --strict-release --desktop-platform=windows</code> を通過させる。macOS が未検証のため <code>--desktop-platform=all</code> は引き続き失敗し得るが、それは Windows-first v1.0 を block しない。
  blocks_release: yes

- item: implementation first-run smoke
  classification: required_for_v1
  reason: <code>tooling/release_smoke.py</code> は first-run config／audit path を作成し、audit directory の writability を検証し、installer／setup state が authority を付与せず、permission を暗黙に approve しないことを確認する。
  required_action: implementation first-run smoke を通過状態に保つ。native Windows installed-path first-run smoke は別の release blocker のままである。
  blocks_release: no

- item: implementation Setup Doctor diagnostics smoke
  classification: required_for_v1
  reason: <code>tooling/release_smoke.py</code> は structured Setup Doctor diagnostics を実行し、すべての check が non-authoritative のままであることを検証する。
  required_action: implementation Setup Doctor smoke を通過状態に保つ。native Windows installed-path Setup Doctor smoke は別の release blocker のままである。
  blocks_release: no

- item: Shell Core persistence smoke
  classification: required_for_v1
  reason: integrated Shell Core release smoke は deterministic state snapshot を保存し、読み込む。
  required_action: integrated persistence smoke を通過状態に保つ。
  blocks_release: no

- item: audit chain and local anchor verification smoke
  classification: required_for_v1
  reason: integrated Shell Core release smoke は JSONL audit event を append し、hash chain linkage と HMAC audit anchor を検証し、tampering を検出する。
  required_action: integrated audit chain／local anchor smoke を通過状態に保つ。
  blocks_release: no

- item: audit anchor external tamper-evidence proof
  classification: release_blocker
  registry_id: audit_anchor_external_tamper_evidence_proof
  reason: local HMAC audit anchor は corruption と partial tamper を検出する。しかし local file authority boundary を越える same-user tamper evidence を主張するには、completed product release の前に計測した Windows ACL／DPAPI、external anchor、または signed-evidence proof が必要である。
  required_action: installed-path audit anchor key-protection または external-anchor evidence を記録し、strict Windows release validation を通過させる。
  blocks_release: yes

- item: approval edit to rehash to revalidation smoke
  classification: required_for_v1
  reason: integrated Shell Core release smoke は許可された approval field を編集し、payload hash を再計算し、approval を <code>requires_validation</code> として mark する。
  required_action: approval lifecycle smoke を通過状態に保つ。
  blocks_release: no

- item: content_visibility UI enforcement smoke
  classification: required_for_v1
  reason: desktop Flutter widget smoke は、redacted approval projection が表示され、非表示の full payload content が render されないことを確認する。
  required_action: UI projection smoke を通過状態に保つ。
  blocks_release: no

- item: Runtime Catalog validation and use smoke
  classification: required_for_v1
  reason: <code>tooling/release_smoke.py</code> は production RuntimeCatalog を介して Runtime／adapter manifest を登録し、catalog が authority を付与しないことを確認する。
  required_action: Runtime Catalog smoke を通過状態に保つ。
  blocks_release: no

- item: Agent Runtime Contract validation and reference smoke
  classification: required_for_v1
  reason: <code>tooling/release_smoke.py</code> は production AgentRuntimeContract を介して workspace boundary、secret path denial、shell permission mapping、auditable diff behavior を確認する。
  required_action: Agent Runtime reference smoke を通過状態に保つ。
  blocks_release: no

- item: owner GO missing
  classification: release_blocker
  registry_id: owner_go
  required_action: 明示的な owner GO を得る。
  blocks_release: yes
~~~

## LLM-readable substrate の claim gate

~~~yaml
- item: LLM extension contract sufficiency unresolved
  classification: known_limitation
  reason: GUI Shell は LLM-readable substrate として定義されているが、既存の contract family は、限定された LLM-built extension onboarding の観点で未監査である。
  required_action: <code>docs/implementation/GUI_SHELL_LLM_SUBSTRATE_COMPLETION_ROADMAP.md</code> の Block L1 を完了する。
  blocks_release: no

- item: bounded extension conformance not passed
  classification: known_limitation
  reason: LLM-built integration が authority を昇格せず、approval／content exposure を迂回せず、audit／recovery を省略せず、Runtime neutrality を壊さないことを実証する限定的な reference extension／adapter conformance harness は、まだ存在しない。
  required_action: L1 gap decision で必要とされた Block L2／L3 を完了する。
  blocks_release: no

- item: cross-agent reproduction not passed
  classification: known_limitation
  reason: 複数の独立した LLM development agent は、repository contract から同じ限定的 extension task をまだ再現していない。
  required_action: cross-agent LLM-readable substrate claim の前に Block L4／L5 を完了する。
  blocks_release: no
~~~

これらは、実証済み LLM-readable substrate という public claim を block する。owner が正本ロードマップの default の combined public positioning を選ばない限り、狭く記述した Windows-first desktop の product release を自動的に block するものではない。

## v1 後の既定範囲

~~~yaml
- item: mobile full release
  classification: post_v1_scope
  reason: owner が mobile を release scope に明示的に含めない限り、v1.0 は Windows-first PC desktop である。
  blocks_release: no

- item: multi-user mode
  classification: post_v1_scope
  reason: v1.0 は single-user である。
  blocks_release: no

- item: cloud sync
  classification: post_v1_scope
  reason: v1.0 は local-first である。
  blocks_release: no

- item: marketplace
  classification: post_v1_scope
  reason: v1.0 は Runtime marketplace を除外する。
  blocks_release: no

- item: enterprise admin
  classification: post_v1_scope
  reason: v1.0 は enterprise admin scope ではない。
  blocks_release: no

- item: full live Codex / Claude Code / Copilot / Cursor / Devin / OpenHands integrations
  classification: post_v1_scope
  reason: v1.0 が必要とするのは generic Agent Runtime contract と mock／reference agent のみである。
  blocks_release: no

- item: BLUE-TANUKI product completion
  classification: post_v1_scope
  reason: BLUE-TANUKI は consumer／reference Runtime であり、GUI-Shell release gate ではない。
  blocks_release: no
~~~

## 既知制約の規則

known limitation を認める条件は、次のとおりである。

~~~yaml
- classification: known_limitation
  reason: 制約が v1.0 release criteria に違反しない
  required_action: <code>README.md</code> と <code>CLAIM.md</code> に記録する
  blocks_release: no

- classification: known_limitation
  reason: 制約が safety、authority、audit、recovery、installer、validation の failure を隠さない
  required_action: release-facing documentation で明示し続ける
  blocks_release: no
~~~

## Windows Rust検証gateと既知の実行揺らぎ

~~~yaml
- item: Windows Rust全target試験の実行gate
  classification: release_blocker
  registry_id: windows_rust_integration_test_execution_policy
  status: resolved
  reason: Rust source commit `6441ae8b2827d2afa01f9963ff2472fd2b889ce2`に対する手動run [36475650538](https://github.com/gatchimuchio/GUI-Shell/actions/runs/36475650538) で、Windows Server 2025／Rust 1.95.0の`cargo check --all-targets`と12 target／391件の`cargo test --all-targets`が成功し、rustfmtと試験後cleanも確認した。commitは`main`へ統合済み。証拠はこのcommitのhosted Windows検査に限り、installed product、実Agent Task、ローカルApplication Control、release readinessを示さない。過去commitの失敗・成功記録は`docs/REV2_PROGRESS.md`に保持する。
  required_action: 後続のRust変更も必要に応じて`.github/workflows/windows-manual-rust-validation.yml`をworkflow_dispatchで対象commitへ実行する。ローカルWindowsのApplication Controlは弱めず、拒否file移動やtest除外で回避しない。
  blocks_release: yes

- item: 開発hostのApplication ControlによるCargo生成executable実行拒否
  classification: known_limitation
  reason: 現在のローカルWindows hostでは一部のCargo生成executableがOS error 4551で起動拒否される。Windows runnerでは同じsourceの全Rust targetを検査できたため、このhost制約は開発検証継続を妨げない。端末保護policy自体は変更しない。
  required_action: 必要なRust Windows検査は手動`workflow_dispatch` runnerで対象commitを指定して行う。
  blocks_release: no

- item: Windows loopback HTTP fixture試験のConnectionReset揺らぎ
  classification: known_limitation
  reason: 2026-09-27、全target標準並列実行でMinidora／A2A loopback試験が間欠失敗し、診断有効時はMinidora `read_headers`のOS error `ConnectionReset`を観測した。A2A単独反復でも失敗が残る。要求headerを完全に読み、応答を一括送信し、送信半閉鎖後に相手のcloseを待つよう3つのtest fixtureを修正した。A2A Broker fixtureの欠陥修正後はBroker projection単独と直列全target 335件が成功した一方、今回の直列全target試行ではA2A Broker loopback fixtureが`a2a_connection_failed`となりlibrary 291/292件で失敗した。後続の直列library再実行では292件すべて成功した。発生源は未確定であり、test fixture結果は実外部Agent接続の証拠ではない。
  required_action: Windows loopback TCP resetの発生条件を引き続き調べる。失敗を直列実行への置換、retry、結果抑制で隠さず、未完了の全target runを成功扱いせず、fixture結果を実外部Runtimeの証拠へ昇格しない。
  blocks_release: no
~~~
