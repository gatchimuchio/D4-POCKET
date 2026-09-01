# Windowsリリース計画

状態基準日: 2026-05-26

GUI-Shell v1.0はWindows優先である。Linuxのビルド・起動スモークは開発検証に有用だが、それだけでは最終製品の証明にならない。BLUE-TANUKIは引き続きconsumer/reference runtimeであり、GUI-Shellのリリース依存関係ではない。

macOSは未検証の移植予定対象である。GUI-Shell v1.0は検証済みのmacOS対応を主張しない。

所有者がv1.0の範囲を明示的に変更しない限り、モバイルは`post_v1_scope`のままとする。

## ツールチェーン要求

~~~yaml
- item: Flutter Windows desktop SDK
  classification: required_for_v1
  reason: native Windows上のFlutter analyze、test、build、およびlaunch smokeは合格した。
  required_action: release candidateでWindows Flutter desktop toolchain validationを現行状態に保つ。
  blocks_release: no

- item: Visual Studio Build Tools
  classification: required_for_v1
  reason: native Windows上のflutter build windowsは合格し、build\windows\x64\runner\Release\gui_shell_desktop.exeを生成した。
  required_action: Windows desktop release-candidate buildでVisual Studio Build Toolsを利用可能に保つ。
  blocks_release: no

- item: Windows desktop project support
  classification: required_for_v1
  reason: flutter create --platforms=windows .は、既存のlib/ app codeを上書きせずapps/desktop_flutter/windowsを生成した。
  required_action: Windows Flutter desktop project fileをversion control下に保つ。
  blocks_release: no
~~~

## 検証コマンド

~~~yaml
- item: Windows Flutter analyze
  classification: required_for_v1
  reason: Windows Flutter analyzeは過去にnative Windowsホストで合格した。strict R2 release promotionには現行release-candidate provenanceが必要である。
  required_action: Windows release candidateでcd apps/desktop_flutter && flutter analyzeを合格状態に保ち、現行validationを正確なsource commitへ結び付ける。
  blocks_release: no

- item: Windows Flutter test
  classification: required_for_v1
  reason: Windows Flutter testは過去にnative Windowsホストで合格した。strict R2 release promotionには現行release-candidate provenanceが必要である。
  required_action: Windows release candidateでcd apps/desktop_flutter && flutter testを合格状態に保ち、現行validationを正確なsource commitへ結び付ける。
  blocks_release: no

- item: Windows build smoke
  classification: required_for_v1
  reason: cd apps/desktop_flutter && flutter build windowsは過去にnative Windowsホストで合格した。strict R2には現行app artifact hashとの結び付きが必要である。
  required_action: release candidateでWindows build smokeを合格状態に保ち、isolated staged manifestへapp artifact hashを記録する。
  blocks_release: no
~~~

## 起動スモーク証拠の要求

~~~yaml
- item: Windows launch smoke
  classification: required_for_v1
  reason: 履歴上のWindows launch smokeは所有者試行用として合格した。strict R2 formal proofにはaggregate native surface exposureではなく、isolated installed runからsurfaceごとのUIAutomation/accessibility evidenceが必要である。
  required_action: release candidateでWindows launch smokeを合格状態に保ち、isolated installed pathからstrict visible-surface evidenceを再収集する。
  blocks_release: no
~~~

## インストーラーと初回実行の要求

~~~yaml
- item: implementation first-run and Setup Doctor smoke
  classification: required_for_v1
  reason: cross-platform implementation smokeはfirst-run config/audit pathを作成し、audit writabilityを検証し、structured Setup Doctor diagnosticsを実行し、installer/setup stateがauthorityを付与せずpermissionをsilent approveしないことを確認する。
  required_action: native Windows installed-path validationを完了させながら、python3 tooling/release_smoke.pyを合格状態に保つ。
  blocks_release: no

- item: Windows installer and first-run smoke
  classification: release_blocker
  registry_id: windows_installer_first_run_smoke
  reason: native Windowsのisolated installed-path installerおよびfirst-run evidenceがrelease_evidence/windows_installed_smoke.jsonに存在しない。
  required_action: 固有のstaged Windows pathを通してinstallし、installed Rust brokerを介してinstalled Flutter .exeを起動する。installer\windows\collect_broker_smoke.ps1を実行し、-BrokerHelperExe、-NoPythonRuntime、installed manifest、measured UIAutomation diagnostic tree、config、audit、およびbroker field-provenance inputを指定してinstaller\windows\collect_installed_smoke.ps1を実行し、python tooling\windows_release_evidence.pyを通す。
  blocks_release: yes

- item: Windows Setup Doctor smoke
  classification: release_blocker
  registry_id: windows_setup_doctor_smoke
  reason: installed-app generated Setup Doctor product exportへの対応は存在するが、native Windows isolated-run evidenceが存在しない。PowerShell Setup Doctor collectorはexternal probe evidenceであり、product proofとして拒否される。
  required_action: collect_installed_smoke.ps1を実行してinstalled appにmachine-readable Setup Doctor product export evidenceを書き出させ、python tooling\windows_release_evidence.pyを通す。
  blocks_release: yes

- item: Windows installed evidence validator
  classification: required_for_v1
  reason: tooling/windows_release_evidence.pyは、正確なsource commit provenance、clean worktree state、isolated run path、app/broker artifact hash linkage、evidence bundle hash、field provenance、installed Flutter .exe launch、broker-mediated first-run endpoint evidence、No-Python launch evidence、non-zero window handle、visible-surface sourceとdiagnostic tree、config JSON parse、audit write/read/delete probe、broker restricted loopback bind、broker authenticated IPC/restart/crash evidence、およびinstalled-app generated Setup Doctor product exportに基づいて、Windows installer/first-runおよびSetup Doctor release evidenceをgateする。
  required_action: owner GOの前に、copied、edited、synthetic、manually confirmed、aggregate-surface、external-probe-as-product、unmeasured-declaration、またはnon-Windows evidenceを拒否できる厳格性をvalidatorに維持する。
  blocks_release: no
~~~

## Windows固有の失敗モード

~~~yaml
- item: PATH resolution
  classification: release_blocker
  aggregate_of: windows_evidence_provenance_isolation, windows_installer_first_run_smoke, windows_setup_doctor_smoke
  reason: Flutter、Git、runtime、helperの各commandはPowerShell、CMD、installer environment、user shellの間で異なる解決結果になり得る。
  required_action: installed app pathおよびSetup DoctorからPATHを検証する。
  blocks_release: yes

- item: PowerShell policy
  classification: release_blocker
  aggregate_of: windows_installer_first_run_smoke, windows_setup_doctor_smoke
  reason: execution policyはscriptまたはhelper launch pathを遮断し得る。
  required_action: authorityを暗黙に拡大せず、policy issueを検出して報告する。
  blocks_release: yes

- item: Visual Studio Build Tools
  classification: release_blocker
  aggregate_of: windows_evidence_provenance_isolation
  reason: C++ workloadまたはWindows SDKの欠落はflutter build windowsを遮断する。
  required_action: build toolの欠落を検出し、operator-visible recovery guidanceを提示する。
  blocks_release: yes

- item: Windows Defender
  classification: release_blocker
  aggregate_of: windows_installer_first_run_smoke, windows_broker_installed_smoke
  reason: quarantineまたはcontrolled-folder accessはhelper、installer、cache、runtime fileを遮断し得る。
  required_action: Defender interferenceの可能性を検出し、recovery stepを分類する。
  blocks_release: yes

- item: WSL boundary confusion
  classification: release_blocker
  aggregate_of: windows_evidence_provenance_isolation
  reason: WSL pathとWindows pathはauthorityおよびfilesystemの想定をまたぎ得る。
  required_action: Windows release validationをnative Windows app path上に保ち、WSLの使用を別に分類する。
  blocks_release: yes

- item: filesystem permission
  classification: release_blocker
  aggregate_of: windows_installer_first_run_smoke, windows_broker_installed_smoke, audit_anchor_external_tamper_evidence_proof
  reason: Program Files、user profile、temp、およびworkspaceのpermissionは互いに異なり得る。
  required_action: Shell Coreのpermission、approval、audit、およびrecovery mappingを通してfilesystem diagnosticを検証する。
  blocks_release: yes

- item: Git credential / SSH credential confusion
  classification: release_blocker
  aggregate_of: windows_setup_doctor_smoke
  reason: Windows Credential Manager、SSH agent、Git config、およびWSL credentialは乖離し得る。
  required_action: secretを公開せず、credentialをauthorityとして扱わずに、credential-surface ambiguityを検出する。
  blocks_release: yes
~~~
