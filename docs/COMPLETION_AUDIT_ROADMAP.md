# 完了監査ロードマップ

状態日: 2026-05-26

## 監査結論

GUI-Shell は、control-plane Runtime Operation Shell に向けた強力な Windows-first v1.0 skeleton である。completed product release ではない。

Phase B の owner-use completion は現在 complete である。これは owner が desktop Shell を日常の local operation に使い、status、problem、evidence、recovery、trust、Runtime、authority の各 surface を確認できることを意味する。OSS v1.0 RC または paid／product completion を意味しない。

architecture は引き続き有効である。

- authority の責任主体は Shell Core である。
- Flutter は operator surface を描画し、authority を所有してはならない。
- Runtime と adapter の boundary は分離したままにする。
- BLUE-TANUKI は adapter boundary を介した consumer／reference Runtime のままであり、GUI-Shell の release dependency ではない。
- contract の責任主体は schema と conformance である。

strict Windows release validation が通過し owner GO が明示されるまでは、release completion を主張してはならない。

MANIFEST は Shell Core、tooling、schema、desktop Flutter、Rust helper、root governance／release document、docs を対象に含む。MANIFEST は completed product の release readiness を主張しない。

## Phase の完了 level

GUI-Shell は、三つの完了定義を分離して使用する。

~~~yaml
- item: owner-use completion
  classification: required_for_v1
  status: complete
  reason: Phase B completion は、owner が GUI-Shell を日常の local operation に使用し、status、problem、evidence、recovery guidance を確認できることを意味する。
  required_action: Phase C、D、E が complete になるまで release-not-claimed language を保持しながら、Phase B owner-use を complete に保つ。
  blocks_release: no

- item: OSS v1.0 RC completion
  classification: release_blocker
  status: later
  reason: OSS release candidate には claim hygiene、measured Windows installed-path evidence、strict Windows release validation、owner GO が必要である。
  required_action: OSS v1.0 RC を主張する前に Phase C、D、E を complete にする。
  blocks_release: yes

- item: paid/product completion
  classification: post_v1_scope
  status: later
  reason: paid／product QC には owner-use および OSS RC を越える support、rollback、long-run、legal、installer、third-party-user の quality gate が必要である。
  required_action: Phase F まで延期する。
  blocks_release: no
~~~

## 監査から実装済みの事項

~~~yaml
- item: measured invariant evaluator
  classification: required_for_v1
  status: implemented
  evidence: packages/shell_core/invariant_evaluator.py は import boundary、adapter metadata escalation、non-authority source grant、content projection、installer／setup authority、mobile／device authority を計測する。
  blocks_release: no

- item: state snapshot invariant measurement
  classification: required_for_v1
  status: implemented
  evidence: packages/shell_core/state_snapshot.py は static invariant flag を返す代わりに InvariantEvaluator().evaluate() を呼び出す。
  blocks_release: no

- item: normalization firewall
  classification: required_for_v1
  status: implemented
  evidence: packages/shell_core/normalization.py は raw payload を保持し、key を正規化し、authority alias を除去し、authority に類する値を検出し、曖昧な authority-bearing payload を隔離し、normalization audit event metadata を出力する。PolicyEvaluator、AdapterLoader、RuntimeCatalog、BLUE-TANUKI authority trace は、exact raw key matching ではなく共有 normalization authority scanner を使用する。
  blocks_release: no

- item: metadata value-only authority policy
  classification: required_for_v1
  status: implemented
  evidence: key stripping 後に authority に類する値を持つ adapter metadata は拒否され、PolicyEvaluator は value-only adapter metadata authority attempt を flag する。
  blocks_release: no

- item: Flutter local Shell Core client
  classification: required_for_v1
  status: implemented
  evidence: ShellCoreClient.local() は明示注入されたShellSnapshotだけを表示用に受け取り、未注入時は診断fallbackを返す。Flutter側の環境変数・filesystem読込はなく、入力の出所・鮮度はunknown、release claimは常に抑止する。製品状態はShellCoreClient.product()経由のBroker応答を使う。
  blocks_release: no

- item: GUI operation surfaces
  classification: required_for_v1
  status: implemented
  evidence: docs/GUI_OPERATION_SURFACES.md は Trust Center、Authority Map、Audit Timeline、Recovery Playbook、Adapter Catalog、Permission Diff、Settings UX、Problems Panel、Evidence Center、Command Palette、Status Bar の surface を記録する。
  blocks_release: no

- item: Shell snapshot generator migration oracle
  classification: required_for_v1
  status: implemented
  evidence: tooling/shell_snapshot.py は owner-use migration／development evidence のため、Flutter local mode が消費する structured local snapshot を生成する。そこには trust、authority、catalog、problem、evidence、settings、audit、recovery、Setup Doctor の field が含まれる。installed product runtime dependency として残してはならない。
  blocks_release: no

- item: Evidence bundle export
  classification: required_for_v1
  status: implemented
  evidence: tooling/evidence_bundle.py --check は release blocker を保持し release_ready=false とした development evidence bundle を検証する。
  blocks_release: no

- item: MANIFEST integrity artifact
  classification: required_for_v1
  status: implemented
  evidence: tooling/manifest.py --write は MANIFEST.sha256.json を生成する。tooling/manifest.py --check は hash、listed file の missing、required source coverage、forbidden generated file、Shell Core の存在を検証する。MANIFEST.sha256.json は自身の file list から除外される。
  blocks_release: no

- item: conformance coverage
  classification: required_for_v1
  status: implemented
  evidence: conformance は Unicode／case／zero-width／camelCase／alias／value-only authority attempt と、意図的な invariant import violation detection を対象にする。
  blocks_release: no

- item: Shell Core persistence, audit, approval, and recovery smoke
  classification: required_for_v1
  status: implemented
  evidence: packages/shell_core/release_smoke.py は state snapshot save／load、append-only audit verification、audit tamper detection、approval edit rehash／revalidation、recovery_id policy verification を対象にする。
  blocks_release: no

- item: implementation first-run and Setup Doctor smoke
  classification: required_for_v1
  status: implemented
  evidence: tooling/release_smoke.py は first-run config／audit initialization と structured Setup Doctor non-authority check を実行する。
  blocks_release: no

- item: Runtime Catalog and Agent Runtime reference smoke
  classification: required_for_v1
  status: implemented
  evidence: tooling/release_smoke.py は RuntimeCatalog を介して reference manifest を登録し、Agent Runtime workspace、secret path、permission mapping、auditable diff behavior を検証する。
  blocks_release: no

- item: Windows installed-path evidence validator
  classification: required_for_v1
  status: implemented
  evidence: tooling/windows_release_evidence.py は release_evidence/windows_installed_smoke.json に対し、installed executable hash、broker-mediated installed Flutter .exe first run、No-Python launch evidence、non-zero window handle、UIAutomation または accessibility-tree の visible-surface evidence source、config JSON parsing、audit write／read／delete probe、non-synthetic Setup Doctor non-authority diagnostics、broker restricted loopback bind、broker authenticated IPC／restart／crash evidence を検証する。
  blocks_release: no

- item: Native Windows build and launch smoke
  classification: required_for_v1
  status: development_evidence
  evidence: native Windows build／launch smoke は development build path で通過済みである。native Windows launch smoke は development evidence であり、measured installed-path release evidence ではない。
  blocks_release: no
~~~

## 残る release blocker

~~~yaml
- item: Windows installer and first-run smoke
  classification: release_blocker
  reason: installed app path の first-run evidence が release_evidence/windows_installed_smoke.json に存在しない。
  required_action: collect_broker_smoke.ps1、collect_setup_doctor.ps1、collect_installed_smoke.ps1 -BrokerHelperExe -NoPythonRuntime を使って native Windows installed smoke collection を実行し、python tooling/windows_release_evidence.py を通過させる。計測済み windows_installed_smoke.json が存在するまでは strict release を引き続き失敗させる。
  blocks_release: yes

- item: Windows Setup Doctor real diagnostics smoke
  classification: release_blocker
  reason: evidence が存在しないため、Setup Doctor は installed Windows app path から未通過である。
  required_action: installed-path Setup Doctor diagnostics を記録し、python tooling/windows_release_evidence.py を通過させる。non-synthetic installed-path Setup Doctor evidence が存在するまでは strict release を引き続き失敗させる。
  blocks_release: yes

- item: Owner GO
  classification: release_blocker
  reason: completed product release には明示的な owner approval が必要である。
  required_action: すべての release blocker が通過した後に限り owner GO を得る。owner GO が存在するまでは strict release を引き続き失敗させる。
  blocks_release: yes
~~~

## 次の実行順序

1. Phase C: OSS claim hygiene のため、README、CLAIM、release checklist、audit、installer、security、strategy document の整合を保つ。
2. Phase D: measured Windows installed-path evidence を収集し、<code>python3 tooling/validate_all.py --strict-release --desktop-platform=windows</code> を通過させる。
3. Phase E: Phase C／D が通過し owner GO が明示された後に限り、OSS v1.0 RC を準備する。
4. Phase F: money または third-party support が scope に入るまで paid／product QC を延期する。

## release 規則

次を満たすまでは、release-ready と述べない。

- strict Windows release gate が通過する。
- installer／first-run smoke が通過する。
- Setup Doctor の real diagnostics smoke が通過する。
- persistence／audit／approval／content visibility／Runtime／agent の smoke が通過する。
- README／CLAIM／release document が evidence と一致する。
- owner GO が明示される。
