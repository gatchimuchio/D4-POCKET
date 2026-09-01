# 製品完成計画

GUI-Shell v1.0 は、完成した Windows-first PC desktop の product release を意味する。

~~~yaml
- item: skeleton、preview、alpha、beta、scaffold の各状態
  classification: release_blocker
  example: true
  reason: これらの状態は completed product release の状態ではない。
  blocks_release: yes
~~~

## v1 の必須事項

platform の優先順位:

- 第一対象: Windows
- portability の計画対象: macOS
- development／verification 用 slice: Linux

~~~yaml
- item: Desktop app
  classification: required_for_v1
  reason: v1.0 desktop release は Windows-first とし、Linux を development verification、macOS を portability の計画対象とする。
  blocks_release: yes

- item: Linux desktop build and launch smoke
  classification: required_for_v1
  reason: 現在の Linux build／launch smoke は 2026-05-25 に development／verification proof として通過したが、それだけでは最終 product proof ではない。
  blocks_release: no

- item: Windows desktop project support, analyze, test, build, launch, Setup Doctor, installer, and first-run smoke
  classification: release_blocker
  aggregate_of: windows_evidence_provenance_isolation, windows_installer_first_run_smoke, windows_setup_doctor_smoke, windows_broker_installed_smoke
  reason: Windows が主要 product target である。過去の Windows project／toolchain／build／launch smoke は owner-trial の履歴にすぎない。strict R2 evidence には isolated installed-path provenance、artifact hash、evidence bundle hash、UIAutomation diagnostic tree、broker の measured field provenance、installed-app generated Setup Doctor product evidence が必要である。
  required_action: native Windows の一意な staged run から release_evidence/windows_installed_smoke.json を生成し、python tooling/windows_release_evidence.py を通過させる。
  blocks_release: yes

- item: macOS planned portability target
  classification: known_limitation
  reason: 現在利用できる macOS validation environment がないため、macOS は unverified であり、GUI-Shell v1.0 は検証済み macOS support を主張しない。
  required_action: macOS support を主張する前に macOS host で検証する。
  blocks_release: no

- item: Windows Setup Doctor diagnostics
  classification: release_blocker
  registry_id: windows_setup_doctor_smoke
  reason: installed-app generated Setup Doctor product export の support は存在するが、native Windows product diagnostics evidence は未収集である。現在の PowerShell Setup Doctor collector は external probe evidence にすぎない。
  required_action: isolated Windows installed smoke を介して、installed-app generated machine-readable Setup Doctor export evidence を収集する。
  blocks_release: yes

- item: Single-user local-first mode
  classification: required_for_v1
  blocks_release: yes

- item: Installer first-run flow
  classification: required_for_v1
  blocks_release: yes

- item: Setup Doctor
  classification: required_for_v1
  blocks_release: yes

- item: Runtime Catalog
  classification: required_for_v1
  blocks_release: yes

- item: Agent Runtime Contract
  classification: required_for_v1
  blocks_release: yes

- item: Shell Core persistence
  classification: required_for_v1
  blocks_release: yes

- item: Permission / Approval / Audit / Recovery
  classification: required_for_v1
  blocks_release: yes

- item: Audit chain verification
  classification: required_for_v1
  blocks_release: yes

- item: Rust helper validation
  classification: required_for_v1
  blocks_release: yes

- item: Desktop Flutter validation
  classification: required_for_v1
  blocks_release: yes

- item: Mock/reference runtime
  classification: required_for_v1
  blocks_release: yes

- item: Mock/reference agent
  classification: required_for_v1
  blocks_release: yes
~~~

## v1 後の範囲

~~~yaml
- item: completed mobile companion
  classification: post_v1_scope
  reason: owner が scope を変更しない限り、v1.0 は Windows-first PC desktop である。
  blocks_release: no

- item: multi-user
  classification: post_v1_scope
  reason: v1.0 は single-user である。
  blocks_release: no

- item: cloud service
  classification: post_v1_scope
  reason: v1.0 は local-first である。
  blocks_release: no

- item: runtime marketplace
  classification: post_v1_scope
  reason: v1.0 は marketplace distribution を除外する。
  blocks_release: no

- item: BLUE-TANUKI product completion
  classification: post_v1_scope
  reason: BLUE-TANUKI は consumer／reference Runtime である。
  blocks_release: no

- item: all live coding-agent adapters
  classification: post_v1_scope
  reason: v1.0 に必要なのは汎用 contract と mock／reference agent である。
  blocks_release: no

- item: enterprise admin
  classification: post_v1_scope
  reason: v1.0 は single-user desktop である。
  blocks_release: no
~~~

## 既知制約

~~~yaml
- item: local single-user mode
  classification: known_limitation
  reason: 意図的な v1.0 product scope である。
  blocks_release: no

- item: mock/reference runtime and agent as included references
  classification: known_limitation
  reason: owner が明示的に含めない限り、live third-party integration は v1.0 の対象外である。
  blocks_release: no
~~~
