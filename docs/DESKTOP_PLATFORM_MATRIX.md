# デスクトッププラットフォーム対応表

状態基準日: 2026-05-26

GUI-Shell v1.0はWindows優先である。Windowsを主要製品対象、Linuxを検証済みの開発・検証区分、macOSを未検証の移植予定対象とする。現在のLinuxホストでの検証はLinux上で合格できるが、それだけでは最終製品の証明にならない。

GUI-Shell v1.0は検証済みのmacOS対応を主張しない。macOSホストで検証するまで、macOS対応をsupported、ready、completeとして広告してはならない。

所有者がv1.0の範囲を明示的に変更しない限り、モバイルは`post_v1_scope`のままとする。BLUE-TANUKIは引き続きconsumer/reference runtimeであり、GUI-Shellのリリース依存関係ではない。

| プラットフォーム | 優先度 | プロジェクト対応 | 必須ツールチェーン | 検証コマンド | ビルド・スモーク | 起動スモーク証拠 | インストーラー・初回実行状態 | リリース分類 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Windows | 主要製品対象 | generated（生成済み） | Windowsホスト上のFlutter Windowsデスクトップツールチェーン。Visual Studio Build Toolsを含む。 | `cd apps/desktop_flutter && flutter analyze && flutter test && flutter build windows` | historical owner-trial pass（履歴上の所有者試行は合格）。strict R2には現行の成果物hashが必要。 | historical owner-trial pass（履歴上の所有者試行は合格）。strict R2にはisolated installed runからのper-surface UIAutomation/accessibility evidenceが必要。 | evidence_missing: release_blocker。strict R2の`release_evidence/windows_installed_smoke.json`が必要。 | release_blocker |
| macOS | 移植予定対象 | unverified_planned（未検証の予定） | macOSホスト上のFlutter macOSデスクトップツールチェーン。Xcodeを含む。 | 対応を主張する前に`cd apps/desktop_flutter && flutter analyze && flutter build macos` | unverified_planned: known_limitation（未検証の既知制限） | unverified_planned: known_limitation（未検証の既知制限） | unverified_planned: known_limitation（未検証の既知制限） | `known_limitation; blocks_release: no`（リリースを遮断しない既知制限） |
| Linux | 開発・検証区分 | generated（生成済み） | Flutter Linuxデスクトップツールチェーン: clang、cmake、ninja、pkg-config | `cd apps/desktop_flutter && flutter analyze && flutter test && flutter build linux` | 2026-05-25に合格 | WSLg上で合格。最初のウィンドウが開き、Dashboard、NavigationRail、Runtime Status、Invariant Statusを視認。 | not_primary_release_gate: known_limitation（主要リリースゲートではない既知制限） | required_for_v1の開発区分。現行のbuildおよびlaunch smokeはblocks_release: no |

## リリースゲート

~~~yaml
- item: Linux desktop build smoke
  classification: required_for_v1
  reason: cd apps/desktop_flutter && flutter build linuxは合格し、build/linux/x64/release/bundle/gui_shell_desktopを生成した。
  required_action: 開発・検証区分としてLinux build smokeを合格状態に保つ。
  blocks_release: no

- item: Linux desktop launch smoke
  classification: required_for_v1
  reason: ./build/linux/x64/release/bundle/gui_shell_desktopはWSLg上で起動し、最初のウィンドウの証拠を記録した。LinuxだけではWindows優先の最終製品証明にならない。
  required_action: Windows製品ゲートを完了させながら、Linux launch smokeを合格状態に保つ。
  blocks_release: no

- item: Windows desktop validation
  classification: release_blocker
  reason: Windowsのproject、analyze、test、build、launch smokeの履歴は所有者試行の証拠に限る。source commit、clean worktree state、artifact hash、UIAutomation diagnostic tree、broker measured field provenance、およびinstalled-app generated Setup Doctor product exportを伴うstrict R2 installed-path evidenceはnative Windows上で再収集されていない。
  required_action: native Windowsホストでisolated Windows installer/first-run validationを完了し、python tooling/windows_release_evidence.pyを通す。
  blocks_release: yes

- item: macOS planned portability target
  classification: known_limitation
  reason: 現在macOS検証環境を利用できないため、GUI-Shell v1.0は検証済みmacOS対応を主張しない。
  required_action: macOS対応を主張する前にmacOSホストで検証する。
  blocks_release: no

- item: Windows Setup Doctor diagnostics
  classification: release_blocker
  reason: installed-app generated Setup Doctor product exportへの対応は存在するが、主要Windows製品対象についてのWindows product evidenceは収集されていない。
  required_action: release_evidence/windows_installed_smoke.jsonを通してWindows Setup Doctor product export evidenceを合格させる。
  blocks_release: yes

- item: mobile full release
  classification: post_v1_scope
  reason: 所有者が範囲を明示的に変更しない限り、mobileはWindows優先v1.0 release scopeの外に留まる。
  required_action: v1.0の対応は不要である。
  blocks_release: no

- item: BLUE-TANUKI product completion
  classification: post_v1_scope
  reason: BLUE-TANUKIはconsumer/reference runtimeであり、GUI-Shellのrelease dependencyではない。
  required_action: v1.0の対応は不要である。
  blocks_release: no
~~~
