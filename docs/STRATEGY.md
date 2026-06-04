# GUI-Shell Strategy

GUI-Shell v1.0 release is Windows-first PC desktop product release.

- item: skeleton, preview, alpha, beta, and scaffold states
  classification: release_blocker
  reason: these states are not completed product release states.
  blocks_release: yes

BLUE-TANUKI is a consumer/reference runtime and not a GUI-Shell release gate.

Agent Runtime support is part of the generic Shell strategy, but live third-party agent integrations may be `post_v1_scope` unless explicitly included by owner instruction.

Platform priority:

- Primary: Windows
- Planned portability target: macOS
- Development/verification slice: Linux

GUI-Shell v1.0 does not claim verified macOS support. macOS support must not be advertised as supported, ready, or complete until validated on a macOS host.

## Product Definition

- item: desktop operator app
  classification: required_for_v1
  reason: Windows is the primary product target, macOS is planned unverified portability, and Linux is development verification.
  blocks_release: yes

- item: Linux desktop build and launch smoke
  classification: required_for_v1
  reason: Linux desktop build and launch smoke passed on 2026-05-25 as development/verification proof, not final product proof by itself.
  blocks_release: no

- item: Windows desktop release validation
  classification: release_blocker
  reason: Historical Windows project/toolchain/build/launch smoke is owner-trial history only. Strict R2 release validation requires isolated installed-path evidence with source commit, clean worktree state, artifact hashes, UIAutomation diagnostic tree, broker measured field provenance, and installed-app generated Setup Doctor product evidence.
  required_action: Generate `release_evidence/windows_installed_smoke.json` on native Windows from a unique staged run and pass `python tooling\windows_release_evidence.py`.
  blocks_release: yes

- item: macOS planned portability target
  classification: known_limitation
  reason: no macOS validation environment is currently available, so GUI-Shell v1.0 does not claim verified macOS support.
  required_action: Validate on a macOS host before claiming macOS support.
  blocks_release: no

- item: Windows Setup Doctor diagnostics
  classification: release_blocker
  reason: The current PowerShell Setup Doctor collector is external probe evidence and is invalid as formal product evidence.
  required_action: Add installed-app generated machine-readable Setup Doctor export evidence.
  blocks_release: yes

- item: installer and first-run Setup Doctor
  classification: required_for_v1
  blocks_release: yes

- item: runtime catalog
  classification: required_for_v1
  blocks_release: yes

- item: agent runtime contract
  classification: required_for_v1
  blocks_release: yes

- item: permission, approval, audit, and recovery control plane
  classification: required_for_v1
  blocks_release: yes

- item: Shell Core persistence
  classification: required_for_v1
  blocks_release: yes

- item: append-only audit chain verification
  classification: required_for_v1
  blocks_release: yes

- item: adapter contract for arbitrary runtimes and agents
  classification: required_for_v1
  blocks_release: yes

## Scope Classification

- item: Windows-first PC desktop single-user local-first release
  classification: known_limitation
  reason: deliberate v1.0 scope.
  blocks_release: no

- item: mobile full release
  classification: post_v1_scope
  reason: outside v1.0 unless owner changes scope.
  blocks_release: no

- item: multi-user, cloud service, marketplace, enterprise admin
  classification: post_v1_scope
  reason: outside v1.0 desktop product scope.
  blocks_release: no

- item: BLUE-TANUKI product completion
  classification: post_v1_scope
  reason: BLUE-TANUKI consumes GUI-Shell through adapter contracts.
  blocks_release: no
