# Desktop Platform Matrix

Status date: 2026-05-26

GUI-Shell v1.0 is Windows-first. Windows is the primary product target, Linux is the validated development/verification slice, and macOS is an unverified planned portability target. Current-host Linux validation can pass on Linux, but it is not final product proof by itself.

GUI-Shell v1.0 does not claim verified macOS support. macOS support must not be advertised as supported, ready, or complete until validated on a macOS host.

Mobile remains `post_v1_scope` unless the owner explicitly changes v1.0 scope. BLUE-TANUKI remains a consumer/reference runtime and is not a GUI-Shell release dependency.

| Platform | Priority | Project support | Required toolchain | Validation command | Build smoke | Launch smoke evidence | Installer / first-run status | Release classification |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Windows | Primary product target | generated | Flutter Windows desktop toolchain on Windows host, including Visual Studio Build Tools | `cd apps/desktop_flutter && flutter analyze && flutter test && flutter build windows` | historical owner-trial pass; strict R2 requires current artifact hashes | historical owner-trial pass; strict R2 requires per-surface UIAutomation/accessibility evidence from isolated installed run | evidence_missing: release_blocker; requires strict R2 `release_evidence/windows_installed_smoke.json` | release_blocker |
| macOS | Planned portability target | unverified_planned | Flutter macOS desktop toolchain on macOS host, including Xcode | `cd apps/desktop_flutter && flutter analyze && flutter build macos` before claiming support | unverified_planned: known_limitation | unverified_planned: known_limitation | unverified_planned: known_limitation | known_limitation; blocks_release: no |
| Linux | Development/verification slice | generated | Flutter Linux desktop toolchain: clang, cmake, ninja, pkg-config | `cd apps/desktop_flutter && flutter analyze && flutter test && flutter build linux` | passed on 2026-05-25 | passed under WSLg; first window opened; Dashboard, NavigationRail, Runtime Status, and Invariant Status visible | not_primary_release_gate: known_limitation | required_for_v1 development slice; current build and launch smoke blocks_release: no |

## Release Gate

- item: Linux desktop build smoke
  classification: required_for_v1
  reason: `cd apps/desktop_flutter && flutter build linux` passed and produced `build/linux/x64/release/bundle/gui_shell_desktop`.
  required_action: Keep Linux build smoke passing as a development/verification slice.
  blocks_release: no

- item: Linux desktop launch smoke
  classification: required_for_v1
  reason: `./build/linux/x64/release/bundle/gui_shell_desktop` launched under WSLg and first-window evidence was recorded; Linux is not final Windows-first product proof by itself.
  required_action: Keep Linux launch smoke passing while completing Windows product gates.
  blocks_release: no

- item: Windows desktop validation
  classification: release_blocker
  reason: Historical Windows project/analyze/test/build/launch smoke is owner-trial evidence only. Strict R2 installed-path evidence with source commit, clean worktree state, artifact hashes, UIAutomation diagnostic tree, broker measured field provenance, and installed-app generated Setup Doctor product export has not been recollected on native Windows.
  required_action: Complete isolated Windows installer/first-run validation on a native Windows host and pass `python tooling\windows_release_evidence.py`.
  blocks_release: yes

- item: macOS planned portability target
  classification: known_limitation
  reason: no macOS validation environment is currently available, so GUI-Shell v1.0 does not claim verified macOS support.
  required_action: Validate on a macOS host before claiming macOS support.
  blocks_release: no

- item: Windows Setup Doctor diagnostics
  classification: release_blocker
  reason: installed-app generated Setup Doctor product export support exists, but Windows product evidence has not been collected for the primary Windows product target.
  required_action: Pass Windows Setup Doctor product export evidence through `release_evidence/windows_installed_smoke.json`.
  blocks_release: yes

- item: mobile full release
  classification: post_v1_scope
  reason: mobile remains outside Windows-first v1.0 release scope unless the owner explicitly changes scope.
  required_action: No v1.0 action required.
  blocks_release: no

- item: BLUE-TANUKI product completion
  classification: post_v1_scope
  reason: BLUE-TANUKI is a consumer/reference runtime, not a GUI-Shell release dependency.
  required_action: No v1.0 action required.
  blocks_release: no
