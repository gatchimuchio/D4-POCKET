# GUI Shell ロードマップ

状態: Phase B は owner-use complete。v1.0 デスクトップリリースに向けたロードマップ
プロジェクト: GUI Shell / Runtime Operation Shell / `LLM-readable application responsibility substrate`（LLM が読むアプリケーション責任基盤）
参照コンシューマー／Runtime: adapter のみを介した BLUE-TANUKI
主要実装経路: 権限に関わる本番収束は Flutter UI + Rust Security Broker。Rust helper は、権限の外側にある限定的な native 診断／操作に留める。

## 現行 owner 指示 rev2（2026-09-10）

現行追加工程は、統治規則 → Baseline → 日本語意味正本 → 契約 → Conformance → Shell Core → MINIDORA Adapter → Desktop 実行系操作 → 実行系比較 → Flutter 共通化 → Mobile → 端末連携 → Android → macOS → iOS → Windows 回帰 → Linux 回帰 → 全数監査 → 文書の順とする。各単位はローカルで実装・試験・監査し、完成分を順次 main へ commit / push して remote HEAD を確認する。

最初の単位は `AGENTS.md` 3.1 と運用モデルの統治変更である。自動 CI は禁止を維持し、GitHub Actions は `workflow_dispatch` の手動補助のみ許可する。今回の統治単位は workflow を追加・実行しない。品質基準、安全境界、release gate、owner GO は維持する。Mobile と端末連携の開発着手は許可されたが、未検証 platform を完成・verified と扱わず、Windows-first release の証拠を代替しない。

実行系対話要求・応答・セッション・比較結果を日本語で先に定義し、Flutter → Shell Core / Rust broker → Adapter → Runtime の責任を維持する。MINIDORA の実物 API は実装前に確認し、内部実装を Shell Core へ輸入しない。

## 完了監査の優先事項

~~~yaml
- item: Ghost Invariants
  classification: required_for_v1
  status: implemented_for_current_scope
  reason: <code>packages/shell_core/state_snapshot.py</code> は、静的な invariant flag ではなく、計測した <code>InvariantEvaluator</code> の結果を報告するようになった。
  required_action: 意図的な違反テストを conformance に維持し、新しい invariant surface の追加に合わせて拡張する。
  blocks_release: no

- item: Normalization Firewall
  classification: required_for_v1
  status: implemented_for_current_scope
  reason: Shell Core は、生の受信 payload を保持し、key を正規化し、authority alias を除去し、authority に類する値を検出し、曖昧な payload を隔離し、正規化 audit metadata を記録する。
  required_action: Unicode／大文字小文字／zero-width／camelCase／envelope／value-only の権限昇格テストを通過状態に保つ。
  blocks_release: no

- item: Language policy runtime convergence
  classification: release_blocker
  status: partially_started_not_passed
  reason: <code>native/rust_helper</code> 配下で Rust broker の production IPC／process 作業を開始している。認証付き <code>127.0.0.1</code> loopback IPC、process ごとの暗号学的 session secret、request size limit、永続 audit／replay／session file store、再起動後の replay 拒否、audit hash-chain の再起動時検証、改竄／不正形式の永続状態の拒否、IPC negative test、および normalization、policy eligibility、approval edit／rehash、content projection、audit verification、recovery mapping、command-envelope eligibility に対する Python-oracle parity は実装済みである。Flutter の <code>main.dart</code> は product authority status に <code>ShellCoreClient.product()</code> と認証付き broker IPC を使用するようになり、broker unavailable／auth／stale／malformed response の経路は、local JSON authority ではなく SUSPEND／fail-closed snapshot として表現される。<code>tooling/release_runtime_assertions.py --check</code> は <code>tooling/validate_all.py</code> と <code>tooling/evidence_bundle.py --check</code> に接続され、現在の product authority surface に Python authority process startup、Python snapshot generator invocation、no-FFI-authority direct bridge token が存在せず、authority operation が broker-mediated であることを検証する。Windows の Flutter analyze／test は <code>cmd /c pushd</code> 経由の <code>flutter.bat</code> で通過する。外部 Flutter shell script が CRLF line ending であるため、WSL から直接実行する <code>flutter</code> は引き続き失敗する。<code>ShellCoreClient.local()</code> は development／diagnostic 専用のままである。Command dispatch は SUSPEND のままで、broker health は現在も <code>authority_cutover_status=not_active</code> を報告し、installed no-Python-runtime evidence と Windows installed-path broker proof は未完了である。
  required_action: installed Windows app evidence から broker path を実証し、active dispatch の前に command-envelope execution gate を完成させ、installed product runtime では Python が dev／test／migration oracle のみに限定されることを実証し、completed product release の前に Windows installed-path broker evidence を収集する。
  blocks_release: yes

- item: Windows installer, first-run, and real Setup Doctor
  classification: release_blocker
  status: not_passed
  reason: installed app path の Setup Doctor と Windows installer／first-run smoke は、現在も Windows-first product の主要 blocker である。
  required_action: installed app path diagnostics、Windows installer／first-run flow、artifact／hash evidence、および strict Windows validation を実装する。
  blocks_release: yes
~~~

## 0. 製品定義

GUI Shell は、local Runtime、agent、tool、service を対象とする PC-first の AI Runtime／Agent Operation Shell であり、`LLM-readable application responsibility substrate`（LLM が読むアプリケーション責任基盤）である。

BLUE-TANUKI 専用 GUI ではない。

BLUE-TANUKI は参照コンシューマー／Runtime であり、adapter boundary を介して接続しなければならない。GUI Shell Core に BLUE-TANUKI 固有 logic を含めてはならない。BLUE-TANUKI の live integration は GUI-Shell v1.0 の release dependency ではない。

## v1.0 デスクトップリリースの範囲

GUI-Shell v1.0 は Windows-first とする。

platform の優先順位:

- 第一対象: Windows
- portability の計画対象: macOS
- 開発／検証用 slice: Linux

Linux の build と launch smoke は通過しており有用だが、それだけでは最終 product proof にならない。主要な product gate は Windows である。

GUI-Shell v1.0 は、検証済みの macOS support を主張しない。macOS host で検証するまでは、macOS support を supported、ready、complete と宣伝してはならない。

~~~yaml
- item: Linux desktop build smoke
  classification: required_for_v1
  reason: Linux development／verification build smoke は 2026-05-25 に通過した。
  required_action: development verification slice として <code>cd apps/desktop_flutter && flutter build linux</code> を通過状態に保つ。
  blocks_release: no

- item: Linux desktop launch smoke
  classification: required_for_v1
  reason: Linux launch smoke は 2026-05-25 に WSLg 上で通過し、Dashboard、NavigationRail、Runtime Status、Invariant Status の表示を確認した。ただし、これは Windows-first release evidence の代替ではない。
  required_action: Windows product gate の完了中も Linux launch smoke evidence を現行に保つ。
  blocks_release: no

- item: Windows desktop release gates
  classification: release_blocker
  reason: Windows が主要 product target である。Windows project support、Flutter analyze、Flutter test、Windows build、native launch smoke は development evidence として通過済みであり、Block E は staged installed app、broker smoke、Setup Doctor、installed first-run の collector を定義済みである。installed-path first-run evidence、installed-path Setup Doctor evidence、broker-mediated installed Flutter <code>.exe</code> launch evidence、No-Python launch evidence、strict Windows release validation、owner GO は未通過である。
  required_action: Windows development smoke を現行に保ち、その後 staged installed app launch、installed-path first-run、installed-path Setup Doctor、broker authenticated IPC／restart／crash／no-Python／no-FFI evidence、No-Python installed Flutter <code>.exe</code> launch evidence、strict Windows release validation、owner GO を通過させる。
  blocks_release: yes

- item: macOS planned portability target
  classification: known_limitation
  reason: 現在利用できる macOS validation environment がないため、GUI-Shell v1.0 は検証済み macOS support を主張しない。
  required_action: macOS support を supported、ready、complete と主張する前に、macOS host で検証する。
  blocks_release: no

- item: Windows Setup Doctor diagnostics
  classification: release_blocker
  reason: Windows 固有の Setup Doctor diagnostics は主要 product gate の一部である。
  required_action: app path から Windows Setup Doctor diagnostics smoke を通過させる。
  blocks_release: yes

- item: Windows installer and first-run plan
  classification: release_blocker
  reason: Windows installer と first-run behavior は主要 product gate の一部である。
  required_action: Windows installer／first-run flow を完成させて検証する。
  blocks_release: yes
~~~

## 1. 譲れない優先順位

1. 安全性
2. 堅牢性
3. operator にとっての明瞭さ／UX
4. 製品機能
5. 利便性

feature の完成を、安全性、authority boundary、auditability、recovery、operator visibility より優先してはならない。

## 2. 中核原則

GUI Shell は control plane であり、visual wrapper ではない。

UI は Runtime state を表示し、operator input を収集してよい。
UI は authority を生成してはならず、permission を付与してはならず、trust を再解釈してはならず、adapter conformance を迂回してはならず、sensitive action を隠してはならない。

contract の責任主体は schema と conformance である。

## 3. Phase 0 で固定した決定

現在の実行経路では、次の決定を固定する。

- 汎用 Runtime Operation Shell の方向性
- BLUE-TANUKI は参照 Runtime のみ
- BLUE-TANUKI は adapter boundary を介して接続
- authority-sensitive implementation path は Flutter UI + Rust Security Broker
- Rust helper は限定された non-authority native diagnostics／operations
- Compose Multiplatform は watchlist candidate
- Tauri は desktop-heavy fallback
- contract model は JSON Schema-first
- 実装順序は Conformance-first
- UI framework の governance risk には FrameworkRiskProfile
- 権限除去の適合規則（Authority Strip Conformance）
- 内容露出の境界（Content Exposure Boundary）
- 承認の表示／編集境界（Approval visibility／edit boundary）

## 4. 目標 architecture

~~~text
Runtime / Agent / Tool / Local Service
  -> Adapter
      -> schema validation
      -> authority strip
      -> content exposure policy
      -> capability declaration
      -> diagnostic normalization
  -> Rust Security Broker
      -> restricted IPC endpoint
      -> schema-validated broker envelope
      -> runtime registry authority path
      -> capability / permission eligibility
      -> approval validation and protected-field enforcement
      -> audit append / verification authority
      -> recovery classification
      -> command-envelope eligibility
      -> process / credential / update gated execution
  -> Shell Core Contracts
      -> runtime-neutral JSON Schema / protocol semantics
      -> migration oracle parity fixtures until cutover
  -> Flutter UI Layer
      -> Flutter rendering
      -> operator input
      -> navigation
      -> local UI state
  -> Rust Helper
      -> bounded non-authority native diagnostics
      -> bounded non-authority native operations
~~~

## 5. Phase 計画

正本となる Phase 定義は [docs/PHASE_STRATEGY.md](./docs/PHASE_STRATEGY.md) に置く。このファイルは実行ロードマップだけを保持する。

- Phase A の状態: complete
- Phase B の状態: owner-use complete
- Phase C: 次は OSS claim hygiene
- Phase D: 実測した Windows installed-path release evidence は後続
- Phase E: OSS v1.0 RC は後続
- Phase F: paid／product QC は後続

completed product release は主張していない。release-ready claim の前には、実装言語方針の Runtime convergence、Phase D の実測した Windows installed-path evidence、explicit owner GO が引き続き <code>release_blocker</code> である。

現在状態から、Windows-first OSS v1.0 の product release、実証済み LLM-readable extension substrate の capability、initial public release、post-public product QC までを統合した正本の実行ロードマップは次のとおりである。

~~~text
docs/implementation/GUI_SHELL_LLM_SUBSTRATE_COMPLETION_ROADMAP.md
~~~

統合 C／L／R／P／F block model には、そのロードマップを使用する。このファイルは Phase roadmap と現在の release blocker を保持する。

## LLM-readable extension surface の作業流

この作業流は、GUI Shell の contract を LLM development／integration agent が第一級の implementation／integration surface として読み、利用するという architecture の方向性を追加する。

LLM は GUI Shell contract の第一級 implementation／integration consumer だが、authority source では決してない。

この作業流は、Windows-first release gate および Rust Security Broker の convergence blocker と併存する。現在の <code>release_blocker</code> を隠したり、改名したり、完了扱いにしたりしてはならない。

### Stage L0: 定義の固定

- README／AGENTS／standard の定義を整合させる。
- human-authority と LLM-extension-agent の boundary を明示する。
- 非主張事項を記録する。
- 既存の Phase A／Phase B status は変更しない。

### Stage L1: contract 設計

- 明示的な machine-readable extension／module integration contract が必要か判断する。
- 既存 schema を、LLM-built module の onboarding requirement に対応付ける。
- 必要な negative case と failure behavior を定義する。
- schema surface を追加する代わりに既存 contract で十分な場合は、その旨を報告する。

### Stage L2: conformance の試験基盤

- 限定された reference extension／adapter scenario を追加する。
- authority を昇格できないことを実証する。
- approval を迂回できないことを実証する。
- audit evidence を出力することを実証する。
- 必要な場合に failure が RecoveryAction または SUSPEND へ対応付くことを実証する。

### Stage L3: agent 間再現 evidence

- 複数の development agent に repository を独立して読ませ、同じ限定的 extension task を実装させる。
- 両者が contract boundary を保持し conformance を通過するか比較する。
- 差分と failure mode を報告する。

### Stage L4: 公開標準／ecosystem の主張 gate

- evidence が存在して初めて、GUI Shell が development agent 間で実証済みの LLM-readable extension substrate であると主張できるか判断する。
- measured reproduction evidence と owner approval より前に、public standard status、ecosystem adoption、proven interoperability を主張しない。

### Phase 0: standard／選定の固定

目標: 概念上および技術上の選定 boundary を固定する。

成果物:

- <code>docs/specs/gui-shell-spec-v1.md</code>
- <code>docs/standards/gui-shell-extended-standard.md</code>
- <code>docs/research/flutter-governance-risk.md</code>
- <code>docs/research/compose-mp-watchlist.md</code>
- <code>docs/research/tauri-fallback.md</code>
- <code>CLAIM.md</code>
- <code>CONFIG.md</code>
- <code>AUDIT.md</code>
- <code>SECURITY.md</code>
- <code>TROUBLESHOOTING.md</code>

終了条件:

- GUI Shell が汎用 Shell として明確に定義されている。
- BLUE-TANUKI 固有 logic は Shell Core で禁止される。
- Flutter risk と migration condition が文書化されている。
- Phase 0 claim boundary が明示されている。

### Phase 1: schema／contract の閉包

目標: UI 実装前に、すべての core contract を定義する。

成果物:

- <code>specs/runtime.schema.json</code>
- <code>specs/adapter.schema.json</code>
- <code>specs/capability.schema.json</code>
- <code>specs/permission.schema.json</code>
- <code>specs/approval.schema.json</code>
- <code>specs/audit.schema.json</code>
- <code>specs/recovery.schema.json</code>
- <code>specs/diagnostic.schema.json</code>
- <code>specs/update.schema.json</code>
- <code>specs/content_exposure.schema.json</code>
- <code>specs/framework_risk_profile.schema.json</code>
- <code>tooling/schema_check/check_schemas.py</code>

必要な invariant:

- すべての schema に <code>$schema</code>、<code>$id</code>、<code>title</code>、<code>type</code> が含まれなければならない。
- Adapter metadata は untrusted とする。
- adapter では <code>authority_strip=true</code> を必須とする。
- content visibility は <code>none</code>、<code>hash_only</code>、<code>summary</code>、<code>redacted</code>、<code>full</code> を扱わなければならない。
- Approval payload には tagged SHA-256 hash を使用しなければならない。
- Framework risk を明示的に表現しなければならない。

終了条件:

~~~bash
python3 tooling/schema_check/check_schemas.py
~~~

が通過する。

### Phase 2: conformance の閉包

目標: product UI が safety contract に先行することを防ぐ。

成果物:

- <code>tooling/conformance_tests/run_conformance_skeleton.py</code>
- <code>docs/specs/gui-shell-spec-v1.md</code>
- <code>docs/specs/adapter-conformance.md</code>
- <code>docs/specs/content-exposure-policy.md</code>
- <code>docs/specs/approval-visibility-boundary.md</code>
- <code>docs/specs/authority-strip-conformance.md</code>

必須の conformance check:

- 受信した authority key を除去する。
- 外部 metadata が authority を昇格させることはできない。
- GUI input が Runtime で許可されていない authority context を生成することはできない。
- memory、cache、previous state だけで authority を付与することはできない。
- <code>content_visibility=full</code> でない限り、full content を表示できない。
- <code>authority_fields</code>、<code>sealed_fields</code>、<code>hidden_fields</code>、<code>sacred_fields</code> は編集できない。
- 編集した approval payload を再 hash／再検証する。
- sensitive action は capability、permission、approval state、AuditEvent、および失敗時の RecoveryAction に対応付けなければならない。

終了条件:

~~~bash
python3 tooling/conformance_tests/run_conformance_skeleton.py
~~~

が、意味のある failure-case coverage を伴って通過する。

### Phase 3: Shell Core の skeleton

目標: framework-independent な Shell Core を構築する。

成果物:

- <code>packages/shell_core/</code>
- <code>packages/shell_contracts/</code>
- framework-neutral な UI state abstraction に限る <code>packages/shell_ui/</code>
- Runtime 登録簿（Runtime Registry）
- Adapter 読込器（Adapter Loader）
- Permission 台帳（Permission Ledger）
- Approval 待ち行列（Approval Queue）
- Audit 保管庫（Audit Store）
- Recovery 目録（Recovery Catalog）
- Update 方針保管庫（Update Policy Store）

必須規則:

- Shell Core は Flutter を import してはならない。
- Shell Core は BLUE-TANUKI の内部実装を import してはならない。
- Shell Core は adapter metadata を信頼してはならない。
- Shell Core は memory／cache を authority として扱ってはならない。
- Shell Core は inspection 用の deterministic state snapshot を公開しなければならない。

終了条件:

- core test が通過する。
- sensitive action routing を Flutter なしで test できる。
- BLUE-TANUKI adapter を Shell Core の変更なしに開発できる。

### Phase 4: Rust helper の境界

目標: hidden authority を作らず、限定的な native capability を追加する。

成果物:

- <code>native/rust_helper/</code>
- process の診断
- filesystem の診断
- network の診断
- update の検証
- audit の hash 化
- 安全な IPC
- 構造化された helper response

必須規則:

- Rust helper を独立した authority path にしてはならない。
- すべての helper action は capability-scoped でなければならない。
- sensitive な helper action はすべて permission／approval linkage を要求しなければならない。
- helper output は schema-valid でなければならない。
- helper failure は RecoveryAction に対応付けなければならない。

終了条件:

~~~bash
cd native/rust_helper && cargo test
~~~

が Rust の導入環境で通過する。

### Phase 5: BLUE-TANUKI 参照 adapter

目標: BLUE-TANUKI を最初の Runtime として adapter のみを介して接続する。

成果物:

- <code>packages/blue_tanuki_adapter/</code>
- health 用 adapter
- ready 用 adapter
- Runtime snapshot 用 adapter
- authority trace 用 adapter
- notification 用 adapter
- approval 用 adapter
- audit export 用 adapter
- diagnostics 用 adapter
- recovery 用 adapter

必須規則:

- GUI Shell の利便性のために BLUE-TANUKI Core を変更しない。
- Shell Core に BLUE-TANUKI の内部実装を import しない。
- BLUE-TANUKI adapter は Runtime state を汎用 GUI Shell schema へ正規化しなければならない。
- Runtime 固有概念は adapter layer 内に留めなければならない。

終了条件:

- Adapter conformance test が通過する。
- BLUE-TANUKI state を汎用的に表示できる。
- BLUE-TANUKI 固有 authority logic が Shell Core に存在しない。

### Phase 6: デスクトップ Flutter operator Shell

目標: 最初の可視化された operator Shell を構築する。

成果物:

- <code>apps/desktop_flutter/</code>
- 概況画面（Dashboard）
- 診断画面（Setup Doctor）
- Runtime 管理画面（Runtime Center）
- Permission 管理画面（Permission Center）
- Approval 管理画面（Approval Center）
- Audit 閲覧画面（Audit Viewer）
- Recovery 管理画面（Recovery Center）
- 設定画面（Settings）
- Runtime invariant の表示面

必須規則:

- Flutter の責任は rendering のみとする。
- Flutter は permission semantics を定義してはならない。
- Flutter は audit semantics を定義してはならない。
- Flutter は authority を付与してはならない。
- contract が許可しない限り、Flutter は full content を表示してはならない。
- Flutter UI action は Shell Core API を通さなければならない。

終了条件:

~~~bash
cd apps/desktop_flutter && flutter analyze
~~~

が Flutter の導入環境で通過する。

### Phase 7: installer／first-run 経路

目標: low-level complexity を露出せず Shell を利用可能にする。

成果物:

- <code>installer/windows/</code>
- <code>installer/macos/</code>
- <code>installer/linux/</code>
- 初回実行 wizard
- environment の診断
- dependency の検査
- Runtime 接続の検査
- recovery の手順

必須規則:

- 通常ユーザーの主要経路として CLI／WSL／npm／Git の複雑性を露出しない。
- Setup Doctor は failure を operator 向けの言葉で説明しなければならない。
- installation は permission を暗黙に付与してはならない。
- installer state を authority にしてはならない。

終了条件:

- 非 expert user が app path から install、launch し、Runtime state を確認できる。
- failure が分類され、recoverable である。

### Phase 8: モバイル Shell／companion

目標: desktop authority を迂回せず mobile participation を追加する。

成果物:

- <code>apps/mobile_flutter/</code>
- device の pairing
- notification の表示
- approval の review
- Runtime の状態
- emergency stop の request
- recovery 手順の表示

必須規則:

- mobile は Shell Core を迂回してはならない。
- mobile approval は field visibility／edit constraint を維持しなければならない。
- mobile device identity を明示しなければならない。
- device pairing は auditable でなければならない。

終了条件:

- mobile は policy の範囲内で observe／approve できる。
- mobile は hidden authority path を生成できない。

### Phase 9: release の強化

目標: 明示的な claim boundary を伴う OSS release を準備する。

成果物:

- release の checklist
- security の review
- license の検証
- signed build の計画
- update の検証
- 互換性 matrix
- 適合性 report
- 監査 evidence bundle

終了条件:

- owner が release claim を明示的に承認する。
- 適用されるすべての validation が通過する。
- 公開 README の claim が実際の implementation state と一致する。

## 6. 現在の claim boundary

後続の promotion までは、GUI Shell は次の事項だけを主張する。

- v1.0 product-completion scaffolding を備えた desktop-first AI Runtime／Agent Operation Shell の skeleton
- schema-first の contract
- conformance-first の作業順序
- 最初の implementation candidate としての Flutter + Rust helper
- adapter のみを介した参照 Runtime としての BLUE-TANUKI
- permission、approval、audit、recovery、policy evaluation、deterministic state snapshot、content exposure のための framework-independent core asset

現時点では、次の事項を主張しない。

- production readiness（本番準備完了）
- signed installer readiness（署名済み installer の準備完了）
- stable mobile readiness（安定した mobile の準備完了）
- BLUE-TANUKI integration の完全な実装
- Rust helper の完全な実装
- Flutter product UI の完全な実装
- security の完全性

## 7. 必須 validation

各完了作業報告の前に、最低限、次を検証する。

~~~bash
python tooling/schema_check/check_schemas.py
python tooling/conformance_tests/run_conformance_skeleton.py
~~~

<code>python</code> が利用できない場合:

~~~bash
python3 tooling/schema_check/check_schemas.py
python3 tooling/conformance_tests/run_conformance_skeleton.py
~~~

Rust が導入されている場合:

~~~bash
cd native/rust_helper && cargo test
~~~

Flutter が導入されている場合:

~~~bash
cd apps/desktop_flutter && flutter analyze
cd apps/mobile_flutter && flutter analyze
~~~

集約 reporter:

~~~bash
python3 tooling/validate_all.py
~~~

## 8. release 規則

次の条件を満たすまでは release readiness を主張しない。

- schema validation が通過する
- conformance validation が通過する
- 適用される場合は Rust helper test が通過する
- 適用される場合は Flutter analysis が通過する
- sensitive action の audit evidence が存在する
- installer behavior が検証されている
- owner が release promotion を明示的に承認する
