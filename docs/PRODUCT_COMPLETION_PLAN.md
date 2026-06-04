# Product Completion Plan

GUI-Shell v1.0 means completed Windows-first PC desktop product release.

- item: skeleton, preview, alpha, beta, and scaffold states
  classification: release_blocker
  reason: these states are not completed product release states.
  blocks_release: yes

## Required For v1

Platform priority:

- Primary: Windows
- Planned portability target: macOS
- Development/verification slice: Linux

- item: Desktop app
  classification: required_for_v1
  reason: v1.0 desktop release is Windows-first, with Linux development verification and macOS planned portability.
  blocks_release: yes

- item: Linux desktop build and launch smoke
  classification: required_for_v1
  reason: current Linux build smoke and launch smoke passed on 2026-05-25 as development/verification proof, not final product proof by itself.
  blocks_release: no

- item: Windows desktop project support, analyze, test, build, launch, Setup Doctor, installer, and first-run smoke
  classification: release_blocker
  reason: Windows is the primary product target. Historical Windows project/toolchain/build/launch smoke is owner-trial history only; strict R2 evidence requires isolated installed-path provenance, artifact hashes, evidence bundle hashes, UIAutomation diagnostic tree, broker measured field provenance, and installed-app generated Setup Doctor product evidence.
  required_action: Generate `release_evidence/windows_installed_smoke.json` on native Windows from a unique staged run and pass `python tooling\windows_release_evidence.py`.
  blocks_release: yes

- item: macOS planned portability target
  classification: known_limitation
  reason: no macOS validation environment is currently available, so GUI-Shell v1.0 does not claim verified macOS support.
  required_action: Validate on a macOS host before claiming macOS support.
  blocks_release: no

- item: Windows Setup Doctor diagnostics
  classification: release_blocker
  reason: installed-app generated Setup Doctor product diagnostics are missing; the current PowerShell Setup Doctor collector is external probe evidence only.
  required_action: Add installed-app generated machine-readable Setup Doctor export evidence.
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

## Post-v1 Scope

- item: completed mobile companion
  classification: post_v1_scope
  reason: v1.0 is Windows-first PC desktop unless owner changes scope.
  blocks_release: no

- item: multi-user
  classification: post_v1_scope
  reason: v1.0 is single-user.
  blocks_release: no

- item: cloud service
  classification: post_v1_scope
  reason: v1.0 is local-first.
  blocks_release: no

- item: runtime marketplace
  classification: post_v1_scope
  reason: v1.0 excludes marketplace distribution.
  blocks_release: no

- item: BLUE-TANUKI product completion
  classification: post_v1_scope
  reason: BLUE-TANUKI is a consumer/reference runtime.
  blocks_release: no

- item: all live coding-agent adapters
  classification: post_v1_scope
  reason: v1.0 requires generic contract and mock/reference agent.
  blocks_release: no

- item: enterprise admin
  classification: post_v1_scope
  reason: v1.0 is single-user desktop.
  blocks_release: no

## Known Limitations

- item: local single-user mode
  classification: known_limitation
  reason: deliberate v1.0 product scope.
  blocks_release: no

- item: mock/reference runtime and agent as included references
  classification: known_limitation
  reason: live third-party integrations are outside v1.0 unless explicitly included.
  blocks_release: no
