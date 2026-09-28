# GUI Shell LLM 基盤完成ロードマップ

状態: C0 の定義・証拠閉包
範囲: Windows-first OSS v1.0 製品完成、実証済み LLM 可読拡張基盤能力、初回公開リリース、公開後の製品品質管理
現在の baseline: LLM 可読基盤の定義固定後の `main`

## 1. 製品定義

GUI Shell は汎用 Runtime Operation Shell であり、LLM 可読な application responsibility substrate である。

次の二つを第一級の役割として設計する。

人間の operator / owner:

- 製品と runtime state を観測する。
- approval を許可または拒否する。
- recovery を承認する。
- release claim を受け入れる。
- 最終責任主体であり続ける。
- 最終製品 GO の唯一の源であり続ける。

LLM 開発 / 統合エージェント:

- GUI Shell の構造、標準、schema、conformance rule、運用手順を読む。
- 限定された module、adapter、tool、または runtime integration を実装する。
- 宣言済み GUI Shell contract を介して拡張を接続する。
- validation を実行して evidence を報告する。
- 権限の生成、自身の sensitive operation の承認、permission の無言拡大、conformance の迂回、generated output・memory・tool output・metadata・UI state・generated configuration の信頼済み権限への変換をしてはならない。

重要な不変条件:

```text
LLMs are first-class implementation and integration consumers of GUI Shell contracts, but are never authority sources.
```

GUI Shell は汎用であり続け、BLUE-TANUKI 固有にはしない。BLUE-TANUKI は adapter 境界だけを介する最初の reference consumer/runtime であり続ける。Shell Core は runtime-neutral でなければならない。Flutter は operator-facing UI layer であり続け、権限を所有してはならない。Rust Security Broker は権限依存の製品境界であり続ける。Python は tooling、local validation、migration oracle、parity comparison、evidence validation に残してよいが、インストール済み active authority runtime の必須要件として残してはならない。

## 2. 検証済みの現在の開始点

完了済み / 成立済み:

- Phase A: 個人 Windows 試験運用は完了している。
- Phase B: owner-use の運用 hardening は完了している。
- GUI Shell は汎用 Runtime Operation Shell、control plane、LLM 可読 application responsibility substrate として文書化されている。
- 人間の最終権限は引き続き明示されている。
- BLUE-TANUKI は adapter 専用の reference consumer/runtime であり続ける。
- Flutter 製品経路は broker-mediated integration を開始済みである。
- 現在の範囲について、Rust Security Broker process、authenticated loopback IPC、durable audit/replay/session storage、authority parity operation、fail-closed handling、release runtime assertion が存在する。

現在の C0 validation evidence:

- 2026-09-28に変更前 `main` `31df760ae5085bf5ddc18c4f3eceb375cbae4f6c` を対象として再検査し、Schema 149件、正常 example 149件、negative fixture 192件、Conformance 225件が合格した。
- 同じ開始状態での最初の `python -X utf8 tooling/manifest.py --check` と `python -X utf8 tooling/release_gate_check.py` は、MANIFESTの追跡済みfile hash 25件不一致とsource file 2件の欠落により失敗した。これは変更前に観測した既存状態として記録し、MANIFESTを再生成した後に両検査を再実行する。
- `python -X utf8 tooling/manifest.py --write` はtracked source 1103件を再生成した。修正後のmanifest check、release gate、evidence bundleは合格し、evidence bundleはrelease blocker 5件、`release_ready=False`、`classification=development_evidence` を維持した。個別commandと終了値は `VALIDATION.txt` に記録した。

過去の `96 checks` entry は以前の証拠として維持する。以前のロードマップ草稿が現在値として記載した `139 checks` は、今回の再計測値と異なるため現行証拠として使わない。履歴の値を現在の値へ書き換えてはならない。

引き続き未完了 / 可視のまま維持する blocker:

- Rust Broker の製品権限 cutover は未完了のままである。`authority_cutover_status=not_active`、実際の external command dispatch は suspended、process / credential / update gated execution は未完了のままである。
- インストール済み製品の証拠は未完了のままである。`installed no-Python-runtime evidence`、`Windows installed-path broker proof`、`installed first-run proof`、`installed Setup Doctor proof`、`strict Windows release validation` が含まれる。
- Owner GO は記録されていない。
- LLM 可読基盤は定義を固定済みで、現在の contract/conformance layer に限定 reference extension conformance が存在し、管理された非権限 extension task について一件の限定 cross-agent reproduction report が存在する。公開用の外部表明証拠は現在の証拠範囲外のままである。

## 3. 完成目標の定義

### 目標 T0: 定義と証拠の閉包

製品定義は内部で整合する。LLM 可読基盤の表現は統治文書間で整合する。現在の validation evidence は内部で整合する。根拠のない製品または ecosystem の表明は存在しない。

T0 は documentation/governance の閉包に限られる。runtime completion または LLM extension functionality を証明しない。

### 目標 T1: LLM 可読拡張基盤の実証

GUI Shell は限定された LLM-built extension に十分な、明示的な extension/integration contract coverage を持つ。限定 reference module または adapter を宣言済み contract を介して追加できる。Conformance は、extension が authority の生成・昇格、approval の迂回、content exposure rule の迂回、audit の迂回、recovery の迂回、runtime neutrality の迂回をできないことを証明する。少なくとも二つの独立 LLM 開発エージェントが、分離した workspace で同じ限定 extension task を実行し、必須境界を維持できる。

T1 後に限り許可する表明:

```text
GUI Shell demonstrates a contract-governed LLM-readable extension substrate for bounded reference integrations.
```

公開標準としての状態または広範な ecosystem adoption を表明してはならない。

### 目標 T2: Windows-First OSS v1.0 完成製品リリース

Rust Security Broker の権限依存製品経路が v1.0 範囲について完了している。インストール済み製品は active authority runtime に Python を必要としない。Flutter/Rust FFI の authority bypass が存在しない。Broker failure path は fail closed する。Windows installed-path の first-run、Setup Doctor、broker、UIAutomation、artifact hash、no-Python-runtime、audit probe、recovery evidence が合格する。厳格な Windows release validation が v1.0 release blocker なしで合格する。Owner が明示的に GO を与える。

### 目標 T3: LLM 基盤を中核に位置付ける初回公開リリース

既定リリース戦略: T1 と T2 の両方が合格するまで、公開 OSS v1.0 を新しい LLM 可読基盤 identity を中核として位置付けるべきではない。

owner が T1 より前に T2 を release することを明示的に選ぶ場合、外部向け表現は、構造が LLM-readable extension 用に設計されていることと、実証済み cross-agent extension evidence は未完了であることだけを述べなければならない。

### 目標 T4: 公開後の商用 / 製品品質管理準備

T4 は product support boundary、official distribution policy、paid/free scope separation、dependency/license/legal review、long-run stability、rollback/update servicing、user-facing failure recovery、commercial responsibility boundary、owner が選ぶ enterprise または third-party integration scope を扱う。

owner が release strategy を明示的に変更しない限り、T4 は初回公開リリース後に計画する。

## 4. 二経路の実行構造

Track R: Runtime / 製品責任の完成。

目的: 実際の Windows-first 製品責任経路を完成させる。

Track R は `Rust Security Broker production convergence`、authority cutover、command/process/credential/update gating、fail-closed runtime behavior、installed-path Windows evidence、installer / Setup Doctor / first-run proof、strict release validation、owner GO に責任を持つ。

Track L: LLM 可読拡張基盤の実証。

目的: GUI Shell が LLM 開発エージェントにとって安全な contract reference および extension substrate として利用可能であることを証明する。

Track L は contract sufficiency audit、extension/module onboarding model、bounded extension conformance、negative test、agent-facing task packet、cross-agent independent reproduction、claim promotion evidence に責任を持つ。

統合規則:

- Track L は Track R の完了前に development/fixture/conformance evidence を使って開発してよい。
- Track L evidence を installed-product proof と説明してはならない。
- Track R の製品完成を LLM-extension proof と説明してはならない。
- 実証済み LLM-readable substrate として GUI Shell を中核に据える公開表明には、owner がより狭い表現を明示的に許可しない限り、Track L と Track R の両方の完了を必要とする。

## 5. Block 別ロードマップ

repository state を変更する各 block は、二世代 backup 規約により push 済み変更前状態を保存し、対象範囲の変更だけを実装し、validate し、tracked file が変わった場合は manifest を更新し、commit、push、remote `main` の検証、両 backup-generation tag の検証、clean/aligned working tree の検証を行い、rollback point を報告しなければならない。

### Block C0: Validation evidence とロードマップの閉包

目的: 現在の証拠不整合を閉じ、この正本完成ロードマップを成立させる。

許可範囲: `docs/implementation/GUI_SHELL_LLM_SUBSTRATE_COMPLETION_ROADMAP.md`、`ROADMAP.md`、`docs/PHASE_STRATEGY.md`、`RELEASE_CHECKLIST.md`、`CLAIM.md`、`README.md`、`VALIDATION.txt`、`MANIFEST.sha256.json`。

期待成果物:

- 正本 roadmap 文書が存在する。
- 対象commit上で再実行した最新のSchema／Conformance結果を、正確な出力とともに記録する。check数は固定値を正本にせず、毎回の実行結果を記録する。
- 過去の96件 check evidence を履歴として維持する。
- manifest、release gate、evidence bundle の結果を再生成後に記録する。
- release blocker を open のまま維持する。
- manifest が合格する。

禁止する範囲拡大: Rust、Flutter、installer、schema、conformance feature implementation、extension/module loader、agent integration、release claim promotion を行わない。

検証:

```bash
python -X utf8 tooling/schema_check/check_schemas.py
python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py
python -X utf8 tooling/manifest.py --check
python -X utf8 tooling/release_gate_check.py
python -X utf8 tooling/evidence_bundle.py --check
```

終了条件: 現在の validation evidence が追記され内部で整合する。過去の evidence を偽装しない。既存 release blocker が open のままである。commit/push/remote verification/backup verification が完了する。

リリース / 表明への影響: T0 の進行だけである。runtime または LLM-substrate demonstration claim を昇格させない。

ロールバック地点: 変更前の `refs/tags/codex/backup-main`。

### Block L1: LLM 拡張に対する既存 contract 充足性監査

目的: GUI Shell が限定 LLM-built extension に十分な contract family を既に持つか、または最小限の明示的 extension/integration contract が必要かを判断する。

許可範囲: documentation と contract analysis だけ。

期待成果物: `docs/implementation/LLM_EXTENSION_CONTRACT_GAP_ANALYSIS.md`。

必須分析範囲: runtime、adapter、capability、permission、approval、audit、recovery、content exposure、update/install contract、Agent Runtime Contract、Runtime Catalog、既存 adapter example、LLM-readable standard。

終了条件: contract gap decision が文書化され、必要性なしに推測的 schema を追加せず、次の正確な conformance block が定義される。

リリース / 表明への影響: 解決まで実証済み LLM-substrate claim を阻止する。範囲を狭く説明した Windows desktop product release を自動的には阻止しない。

### Block L2: 最小拡張 contract の閉包

L1 が contract gap を証明した場合だけ実行する。

目的: 限定 LLM-built extension の onboarding を安全に表現するために必要な最小の machine-readable contract を導入する。

正当化された場合に限る候補 contract: `specs/extension_manifest.schema.json` または `specs/extension_submission.schema.json`。区別の必要性が証明されない限り両方を作成しない。

必須 negative coverage: authority escalation metadata、self-approved sensitive behavior、undeclared capability use、undeclared permission use、audit omission、recovery omission、content exposure bypass、Shell Core に注入された runtime-specific logic、authority 生成を試みる generated configuration。

終了条件: schema/fixture/negative fixture coverage は必要箇所だけに存在し、conformance check が実際の統治 contract path を使い、新規 runtime execution path を有効化しない。

### Block L3: 限定参照拡張の conformance 検査基盤

目的: contract/conformance layer で LLM-readable substrate claim を証明する。

参照 task: 宣言済み capability だけを持ち、privileged execution がなく、audit evidence、RecoveryAction または SUSPEND への failure mapping、runtime neutrality を備えた限定 reference adapter または非権限 diagnostic module を追加する。

必須 test: 限定 extension は宣言済み contract だけを介して登録でき、authority の生成、permission の付与、自己承認、方針なしの full content 露出、audit evidence の省略、recovery mapping の省略、metadata/memory/generated config/tool response/UI state の authority としての利用、runtime-specific logic の Shell Core への持込み、malformed または unauthorized 状態での合格ができないこと。

証拠分類: `contract/conformance demonstration`。`installed-product evidence`、`cross-agent reproduction evidence`、`public standard adoption evidence` ではない。

### Block L4: LLM エージェント task packet と repository 読取り面

目的: 人間による口頭説明を必要とせず、独立 LLM 開発エージェントが repository を利用できるようにする。

期待成果物: `docs/implementation/LLM_EXTENSION_TASK_PACKET.md` と `docs/implementation/LLM_EXTENSION_EVIDENCE_TEMPLATE.md`、または同等の限定 documentation set。

制約: SDK、marketplace、plugin registry、live Codex/Claude/Copilot integration ではない。

終了条件: 独立 LLM エージェントに repository、task packet、通常の repository instruction を与えることで、owner の追加説明なしに限定 extension を試行できる。

### Block L5: エージェント間再現証拠

目的: 複数の独立 LLM 開発エージェントが GUI Shell contract を読み、責任境界を維持しながら同じ限定 extension を実装できるか検査する。

方法: 同じ baseline commit から分離 branch、clone、または再現可能 workspace で、少なくとも二つの独立 development-agent execution を行う。

期待報告: `docs/evidence/LLM_CROSS_AGENT_REPRODUCTION_REPORT.md`。

必須証拠: baseline commit、task packet、resulting diff、validation command と output、boundary failure、manual repair status、`unauthorized scope expansion attempt`。

表明規則: reproduction が合格した場合に限り、public documentation で実証済み bounded cross-agent LLM-readable extension behavior を表明してよい。industry standard status または general ecosystem compatibility を表明してはならない。

### Block R1: Rust Security Broker 責任切替えの閉包

目的: Windows-first v1.0 に必要な権限依存 product runtime path を完成させる。

移行順序:

1. authority の正規化 / 除去 / 隔離。
2. capability と permission の eligibility。
3. approval 検証 / 保護 field の強制 / rehash。
4. content visibility の強制。
5. audit 追記 / hash-chain 検証 / 改ざん拒否。
6. recovery の分類。
7. command-envelope の eligibility 判定。
8. process / credential / update の関門付き実行。

各責任の必須規則: 該当する場合は Python oracle behavior を列挙し、Rust implementation が存在し、accepted case の parity evidence が合格し、Rust が少なくとも同等の negative case を拒否し、Rust 固有 IPC/session/replay failure を監査し、その責任に関する active product invocation が Python に依存せず、rollback point を記録する。

禁止する近道: FFI authority bridge、隠れた Python runtime authority path、UI 所有 authority、metadata 生成 authority を設けず、eligibility、audit、recovery、rollback evidence が揃う前に command dispatch を有効化しない。

終了条件: `authority_cutover_status` は測定済み証拠により正当化された場合だけ active にできる。

### Block R2: Windows インストール済み経路の製品証拠

目的: local development behavior だけでなく、実際の installed Windows application path を証明する。

必須の測定済み証拠は次のとおり。

- インストール済み application の artifact hash（`installed application artifact hash`）。
- インストール済み executable の起動（`installed executable launch`）。
- broker を介した製品起動（`broker-mediated product launch`）。
- Python runtime 非依存の active authority evidence（`no-Python-runtime active authority evidence`）。
- 非ゼロの main window handle。
- 実際の surface ごとの UIAutomation evidence（`UIAutomation per-surface evidence`）。
- aggregate/native fake surface の近道がないこと。
- first-run config の作成と JSON parsing。
- audit directory の write/read/delete probe。
- installed app path からの Setup Doctor。
- broker の authenticated IPC。
- restricted loopback または approved transport。
- durable store の readiness。
- restart 時の replay rejection。
- crash 時の fail-closed behavior。
- Flutter/Rust FFI authority bridge がないこと。
- recovery の evidence。

実行境界: Ubuntu 側 Codex が source と script を準備する。Native Windows が evidence collection を実行する。Native Windows は証拠を合格させるための場当たり的な code change を導入してはならない。

終了条件: Windows evidence validator が、正確な implementation commit に結び付く non-synthetic evidence で合格する。

### Block R3: 厳格 Windows release candidate 関門

目的: すべての Windows-first product blocker を閉じた OSS v1.0 release candidate を生成する。

必須 validation は schema check、conformance skeleton、manifest check、release gate check、evidence bundle check、release runtime assertion、Windows release evidence、`validate_all.py --strict-release --desktop-platform=windows`、Rust `cargo fmt --check`、Rust `cargo test`、必須 native Windows Flutter build/test/launch validation を含む。

終了条件: owner GO を除き Windows-first `release_blocker` が残らない。claim documentation が evidence と正確に一致する。known limitation を可視のまま維持する。別途 validation しない限り macOS を表明対象にしない。BLUE-TANUKI は非阻止 reference consumer/runtime であり続ける。

### Block P1: 公開表明の健全性とリリース packaging

目的: 過剰表明なしに public OSS-facing material を準備する。

必須公開面: README、CLAIM、RELEASE_CHECKLIST、ROADMAP、PHASE_STRATEGY、AUDIT、SECURITY、installation instruction、evidence report、release note、manifest、rollback instruction。

必須の表明段階:

1. 構造定義済み: LLM 可読な responsibility substrate として設計されている。
2. conformance 実証済み: 限定 extension behavior を contract/conformance で検査済みである。
3. cross-agent 実証済み: 複数の独立 LLM エージェントが限定 extension behavior を再現した。
4. Windows-first 完成製品リリース: installed product evidence と strict release gate が合格した。

証拠なしにこれらの段階を統合してはならない。

### Block P2: Owner GO と初回公開 OSS リリース

目的: 必須証拠を閉じた後に限り初回公開リリースを行う。

既定 owner-GO 前提: T1 合格、T2 合格、public claim 整合、manifest と rollback point の維持、最終 owner review、明示的 GO。

Owner 管理例外: 公開表現を設計意図に限定し、独立 cross-agent reproduction evidence が未完了であると明記する場合に限り、owner は cross-agent reproduction 前の public release を明示的に承認できる。

### Block F1: 公開後の商用 / 製品品質管理

目的: owner がその経路を選んだ後に限り、open/public initial release から commercial product responsibility へ移行する。

範囲候補: `official distribution channel`、paid/free boundary、enterprise deployment boundary、support と incident の責任、installer/update servicing policy、long-run operation testing、dependency と license の監査、vulnerability response、導入する場合の telemetry/privacy policy、commercial integration、追加 runtime または agent product。

規則: 推測的 enterprise feature で initial open/public release に早すぎる負担を課さない。commercial distribution を選んだ後は product responsibility work を省略しない。

## 6. 依存関係と統合関門の model

| Block（工程） | 依存先 | 製品リリース前に実行可能か | 中核 LLM 基盤の公開表明に必須か | Windows OSS v1.0 製品リリースに必須か |
| ----- | ---------- | ------------------------------: | -----------------------------------------------: | ----------------------------------------------: |
| C0（定義・証拠閉包） | 現在の `main` | Yes（可） | Yes（必須） | Yes（必須） |
| L1（contract 監査） | C0（完了後） | Yes（可） | Yes（必須） | No（表明を使う場合を除き不要） |
| L2（contract 閉包） | L1 の gap 判断 | Yes（可） | gap が存在する場合に必須 | No（表明を使う場合を除き不要） |
| L3（限定 conformance） | L1/L2（該当完了後） | Yes（可） | Yes（必須） | No（表明を使う場合を除き不要） |
| L4（作業指示 packet） | L3（完了後） | Yes（可） | Yes（必須） | No（不要） |
| L5（エージェント間再現） | L4（完了後） | Yes（可） | Yes（必須） | owner が既定の統合 release を採用しない限り No（不要） |
| R1（責任切替え） | C0（完了後） | Yes（可） | installed/product-backed positioning には Yes（必須） | Yes（必須） |
| R2（Windows 証拠） | R1（完了後） | 合格前の最終表明は No（不可） | product-backed positioning には Yes（必須） | Yes（必須） |
| R3（厳格関門） | R2（完了後） | No（不可） | completed product claim には Yes（必須） | Yes（必須） |
| P1（公開準備） | 該当する L/R evidence | Yes（可） | Yes（必須） | Yes（必須） |
| P2（公開リリース） | P1 + owner GO（完了後） | release event（リリース時） | 既定戦略では Yes（必須） | Yes（必須） |
| F1（公開後品質管理） | public release / owner decision（完了後） | post-release（公開後） | No（不要） | No（不要） |

## 7. 証拠と検証の model

今後の各 block は次を区別しなければならない。

- CONFIG evidence（設定証拠）。
- INTERNAL_STATE evidence（内部状態証拠）。
- FIXTURE evidence（fixture 証拠）。
- LIVE_RUNTIME evidence（実 runtime 証拠）。
- EXTERNAL_EVIDENCE（外部証拠）。
- cross-agent reproduction evidence（エージェント間再現証拠）。

documentation を runtime proof に、fixture success を installed-product proof に、一つの LLM による successful diff を cross-agent reproducibility に、local Windows build を installed-path release proof に、architecture definition を public standard adoption に決して昇格させない。

各 block は implemented behavior、実行した contract path、実行した runtime/governed path、拒否した negative case、実行 command、正確な output、未検証 claim、残存 blocker、rollback point を記録しなければならない。

## 8. 表明昇格規則

既存の product release blocker は release blocker のまま維持する。

- Rust Security Broker の製品収束。
- installed no-Python-runtime evidence（Python 非依存証拠）。
- Windows installed-path first-run evidence（初回起動証拠）。
- Setup Doctor evidence（診断証拠）。
- strict Windows release validation（厳格検証）。
- owner GO（所有者承認）。

LLM-readable claim の blocker:

- LLM extension contract sufficiency の未解決は実証済み LLM-substrate claim を阻止するが、範囲を狭く説明した desktop product release を自動的には阻止しない。
- Bounded extension conformance の未合格は、LLM-built integration が実証済みであるとの表明を阻止する。
- Cross-agent reproduction の未合格は、独立 LLM エージェント間で substrate が実証済みであるとの表明を阻止する。

既定の統合公開リリース規則:

- LLM-readable substrate が現在の中核 product positioning であるため、product runtime proof と LLM extension proof の両方が合格するまで、中核の public product claim にしてはならない。
- Owner は、より狭い表明を伴って明示する場合だけ上書きできる。

## 9. ロールバックと backup の要件

repository state を変更するすべての block は二世代 backup 規約に従わなければならない。

- `refs/tags/codex/backup-main`;
- `refs/tags/codex/backup-main-prev`.

owner が緊急 handoff を明示的に要求しない限り、remote backup branch を保持してはならない。push、remote HEAD verification、または backup verification に失敗した場合、その block は未完了である。

各 block の rollback point は、completion report で報告した変更前 `refs/tags/codex/backup-main` hash である。

## 10. 明示的な範囲外 / 公開後の項目

owner が明示的に昇格させない限り、C0 から初回公開リリースまでの範囲外とする項目:

- plugin registry（登録機構）。
- marketplace（市場機構）。
- SDK（開発 kit）。
- enterprise administration（企業管理）。
- commercial integration（商用統合）。
- cloud service（クラウドサービス）。
- mobile full release（mobile 完全リリース）。
- 広範な third-party runtime catalog。
- 公開標準採用の表明。
- telemetry を導入しない限り telemetry/privacy policy。
- GUI Shell release の依存条件としての BLUE-TANUKI product completion。
