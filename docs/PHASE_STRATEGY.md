# Phase 戦略

GUI-Shell は、owner-use の進捗と completed product の release readiness を混同しないよう、Phase に基づく readiness 表現を使用する。

## 現在の Phase

~~~yaml
- phase: A
  name: personal Windows trial operation
  status: complete
  evidence: Windows desktop build と native launch smoke は通過し、Dashboard、NavigationRail、Runtime Status、Invariant Status の表示を確認した。

- phase: B
  name: owner-use operational hardening
  status: complete
  goal: authority、audit、approval、recovery、evidence の boundary を保持しながら、GUI-Shell を日常の personal operation に役立つ状態にする。
  current_surfaces: Dashboard の Phase status、persistent status bar、Problems Panel、Evidence Center、Recovery Playbook、Trust Center、Runtime Center、Authority Map は、表示専用の owner-operation surface として利用できる。
~~~

## Phase B ロードマップ

~~~yaml
- item: B-1 owner operation console
  classification: required_for_v1
  status: complete
  evidence: Dashboard の Phase status、persistent status bar、Problems Panel、Evidence Center、Recovery Playbook、release-not-claimed UI を実装済みである。
  blocks_release: no

- item: B-2 local snapshot / local runtime wiring
  classification: required_for_v1
  status: complete
  evidence: ShellCoreClient.local() は明示注入されたShellSnapshotだけを表示用に受け取り、未注入時は安全なfallbackを返す。Flutterからの環境変数・filesystem読込は行わず、入力の出所・鮮度はunknown、release claimは抑止する。ファイル生成物をFlutterへ取り込む経路は本機能に含めず、製品状態はBroker経由とする。
  blocks_release: no

- item: B-3 owner launch flow
  classification: required_for_v1
  status: complete
  evidence: scripts/launch_owner_desktop.sh と scripts/launch_owner_desktop.ps1 は Rust broker を起動し、GUI_SHELL_BROKER_ENDPOINT_JSON を export し、strict release validation または release evidence generation なしで Flutter desktop を起動する。
  blocks_release: no

- item: B-4 Problems to Recovery loop
  classification: required_for_v1
  status: complete
  evidence: Problems の row は recovery_id、safe_to_ignore_for_phase_b、blocks_owner_use、blocks_completed_product_release を保持する。Problems Panel は privileged auto-fix を実行せず、対応する recovery command／path を表示する。
  blocks_release: no

- item: B-5 Trust / Authority / Runtime Map
  classification: required_for_v1
  status: complete
  evidence: Trust Center と Authority Map は desktop navigation に復元されている。Runtime Center は Flutter の display-only authority boundary を保持しながら Runtime -> Capability -> Permission -> Approval -> Audit -> Recovery を表示する。
  blocks_release: no

- item: B-6 owner-use completion gate
  classification: required_for_v1
  status: complete
  evidence: owner launch helper は local snapshot を生成して desktop Shell を開く。status、problem、evidence、recovery、trust、Runtime、authority の各 surface が表示され、local snapshot／fallback は release_state: not claimed を保持する。
  blocks_release: no
~~~

## 後続 Phase

~~~yaml
- phase: C
  name: OSS claim hygiene
  status: next
  goal: README、CLAIM、release checklist、audit、installer、security、strategy document と language-policy convergence blocker の整合を保ち、外部読者が Phase B を release readiness と誤認できないようにする。

- phase: D
  name: measured Windows release evidence
  status: later
  goal: native Windows の installed-path evidence を収集し、強化した Windows evidence validator を通過させる。

- phase: E
  name: OSS v1.0 release candidate
  status: later
  goal: strict Windows release validation を通過し、known limitation を保持し、owner GO を待つ。

- phase: F
  name: paid/product QC
  status: later
  goal: support、rollback、long-run、legal、dependency、installer、third-party-user の quality gate を完了する。
~~~

## 統合完了ロードマップ

現在の Phase B における owner-use state から Windows-first product の completion、LLM-readable substrate の demonstration、initial public release、post-public product QC へ進む正本の実行ロードマップは、次に置く。

~~~text
docs/implementation/GUI_SHELL_LLM_SUBSTRATE_COMPLETION_ROADMAP.md
~~~

このロードマップは、完了済みの Phase A／Phase B status を変更せず、C／L／R／P／F の execution block を追加する。その LLM-readable substrate の claim gate によって、既存の Rust Broker、Windows installed-path evidence、strict release validation、owner GO の release blocker を隠してはならない。

## release 規則

実装言語方針の Runtime convergence が実証され、strict Windows release validation が通過し、owner GO が明示されるまでは、completed product release を主張しない。

Phase B は、strict release gate を弱めずに owner usability を改善してよい。

## 実装言語方針の gate

~~~yaml
- item: Rust Security Broker migration
  classification: release_blocker
  status: partially_started_not_passed
  reason: Rust broker skeleton と JSON envelope rejection test は存在するが、現在の owner-use および validation Shell Core behavior は引き続き Python で実装され、broker はまだ production authority path ではない。
  required_action: Phase D evidence が completed product release claim を支えられるようになる前に、production IPC transport、parity migration、Flutter broker-mediated authority path、no-Python-runtime assertion、no-FFI-authority assertion を完成させる。
  blocks_release: yes
~~~
