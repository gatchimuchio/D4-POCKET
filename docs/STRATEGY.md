# GUI-Shell の戦略

GUI-Shell v1.0 の release は、Windows-first の PC desktop 向け product release とする。

~~~yaml
- item: skeleton、preview、alpha、beta、scaffold の各状態
  classification: release_blocker
  reason: これらの状態は completed product release の状態ではない。
  blocks_release: yes
~~~

BLUE-TANUKI は consumer／reference Runtime であり、GUI-Shell の release gate ではない。

Agent Runtime support は汎用 Shell 戦略の一部である。ただし live third-party agent の integration は、owner instruction で明示的に含めない限り <code>post_v1_scope</code> としてよい。

platform の優先順位:

- 第一対象: Windows
- portability の計画対象: macOS
- development／verification 用 slice: Linux

GUI-Shell v1.0 は検証済みの macOS support を主張しない。macOS は未検証（<code>unverified</code>）であり、macOS host で検証するまでは macOS support を supported、ready、complete と宣伝してはならない。

## 製品定義

~~~yaml
- item: desktop operator app
  classification: required_for_v1
  reason: Windows が主要 product target、macOS が未検証の portability 計画対象、Linux が development verification である。
  blocks_release: yes

- item: Linux desktop build and launch smoke
  classification: required_for_v1
  reason: Linux desktop build／launch smoke は 2026-05-25 に development／verification proof として通過したが、それだけでは最終 product proof ではない。
  blocks_release: no

- item: Windows desktop release validation
  classification: release_blocker
  reason: 過去の Windows project／toolchain／build／launch smoke は owner-trial の履歴にすぎない。strict R2 release validation には、source commit、clean worktree state、artifact hash、UIAutomation diagnostic tree、broker の measured field provenance、installed-app generated Setup Doctor product evidence を伴う isolated installed-path evidence が必要である。
  required_action: native Windows の一意な staged run から release_evidence/windows_installed_smoke.json を生成し、python tooling/windows_release_evidence.py を通過させる。
  blocks_release: yes

- item: macOS planned portability target
  classification: known_limitation
  reason: 現在利用できる macOS validation environment がないため、GUI-Shell v1.0 は検証済み macOS support を主張しない。
  required_action: macOS support を主張する前に macOS host で検証する。
  blocks_release: no

- item: Windows Setup Doctor diagnostics
  classification: release_blocker
  reason: installed app は machine-readable Setup Doctor product export を扱うが、native Windows product evidence は未収集である。PowerShell Setup Doctor collector は external probe evidence であり、formal product evidence としては無効である。
  required_action: isolated Windows installed smoke を介して、installed-app generated machine-readable Setup Doctor export evidence を収集する。
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
~~~

## 範囲の分類

~~~yaml
- item: Windows-first PC desktop single-user local-first release
  classification: known_limitation
  reason: 意図的な v1.0 scope である。
  blocks_release: no

- item: mobile full release
  classification: post_v1_scope
  reason: owner が scope を変更しない限り v1.0 の対象外である。
  blocks_release: no

- item: multi-user, cloud service, marketplace, enterprise admin
  classification: post_v1_scope
  reason: v1.0 desktop product scope の対象外である。
  blocks_release: no

- item: BLUE-TANUKI product completion
  classification: post_v1_scope
  reason: BLUE-TANUKI は adapter contract を介して GUI-Shell を消費する。
  blocks_release: no
~~~
