# Runtime 責任主体監査

状態: Flutter broker 製品経路の開始後に更新した Phase 0 現状監査
日付: 2026-06-03
範囲: 言語方針の runtime 収束 / Rust Security Broker Migration v0.1

この文書は、現行 GUI-Shell の実行経路、検証経路、release 判定経路で、どの言語・どのファイルが責任を持っているかを記録する。ここでの「active」は completed product release を意味しない。現時点で存在する owner-use、development validation、release evidence 経路において実際に呼ばれている、または責任実装として扱われていることを指す。

## 1. 監査結論

- `packages/shell_core/*.py` は、現行の runtime registry、permission、approval、audit、recovery、content exposure、policy evaluation セマンティクスを実装している。
- Flutter desktop product entry は Python を直接実行しない。`main.dart` は `ShellCoreClient.product()` を呼び、`BrokerClient` が Rust broker process の endpoint file を読み、authenticated `127.0.0.1` loopback IPC で health / normalization / content projection / approval edit / command envelope status を取得する。
- `ShellCoreClient.local()` は `GUI_SHELL_SNAPSHOT_JSON`、`%LOCALAPPDATA%\GUI-Shell\shell_snapshot.json`、`.gui_shell/shell_snapshot.json` を読むが、development / diagnostic-only path として残され、product authority state には使われない。
- owner launch scripts は Rust broker を起動し、`GUI_SHELL_BROKER_ENDPOINT_JSON` を Flutter に渡す。Python `tooling/shell_snapshot.py` は local diagnostic / parity tooling として残る。ただし installed product no-Python-runtime proof は未取得である。
- Rust helper crate 内には Rust Security Broker の製品 process と authority parity path の初期実装がある。`broker-server`、authenticated `127.0.0.1` loopback IPC、durable audit/replay/session file store が存在する。
- Rust 側には health/shutdown の process test、stale/replay/malformed/authority-metadata rejection、restart replay rejection、tampered persisted-state rejection、audit hash-chain の restart verification が存在する。normalization / permission / approval / content projection / audit / recovery / command-envelope eligibility に対する Python oracle parity も存在する。
- Flutter broker の製品 path code と fail-closed test coverage は追加済みで、Windows Flutter `flutter.bat` 経由の analyze/test は通過した。`tooling/release_runtime_assertions.py --check` は `tooling/validate_all.py` と `tooling/evidence_bundle.py --check` に接続され、product authority path の Python process startup 不在、Python snapshot generator invocation 不在、no-ffi-authority direct-bridge assertion、broker-mediated authority operation を検証する。
- WSL から直接行う `flutter` は外部 Flutter shell script の CRLF 問題で exit 127 のため、`validate_all.py` の Flutter subcheck はこの環境では失敗する。Windows installed-session evidence と real execution gate は未完了である。
- BLUE-TANUKI は adapter mock/reference の範囲に留まっており、BLUE-TANUKI core の変更経路は確認されなかった。
- TypeScript / Node の GUI-Shell core runtime 導入は確認されなかった。
- authority path への `flutter_rust_bridge`、`dart:ffi`、`MethodChannel`、Rust FFI 直結は確認されなかった。

リリース関門の結論:

- item: active Shell Core の言語方針収束
  classification: release_blocker（分類）
  reason: Rust Security Broker process、製品 IPC としての authenticated loopback IPC、durable store、Python oracle parity harness、Flutter broker の製品 path code、release runtime assertion、Windows Flutter analyze/test evidence は存在する。ただし broker health は `authority_cutover_status=not_active`、command dispatch は suspended、installed no-Python-runtime evidence、Windows installed-path broker evidence は未完了である。
  required_action: Windows installed path で broker launch/connect/fail-closed を実測し、Python を dev/test/migration oracle に格下げした証拠と Windows installed-path broker evidence を追加する。
  blocks_release: yes（リリース阻止）

## 2. 実行経路監査

- `apps/desktop_flutter`: 現在経路は `main.dart` -> `ShellCoreClient.product()` -> `BrokerClient` -> authenticated loopback IPC -> Rust broker response / fail-closed snapshot。言語は Dart + Rust IPC。分類は製品 UI 経路開始済み。証拠は `apps/desktop_flutter/lib/main.dart`、`apps/desktop_flutter/lib/services/broker_client.dart`、`apps/desktop_flutter/lib/services/shell_core_client.dart`。方針差分は command dispatch が suspended、broker が `authority_cutover_status=not_active` を報告し、この session の Flutter validation evidence が環境要因で blocked であること。
- owner desktop launch: 現在経路は script が Rust broker を起動し、`GUI_SHELL_BROKER_ENDPOINT_JSON` を export して Flutter を実行する。言語は Shell / PowerShell + Rust + Dart。分類は development product-path launch。証拠は `scripts/launch_owner_desktop.sh`、`scripts/launch_owner_desktop.ps1`、`apps/desktop_flutter/lib/services/broker_client.dart`。方針差分は installed Windows service/supervisor proof の欠如。
- `ShellCoreClient.local()`: 現在経路は test/dev で任意の local diagnostic client が `.gui_shell/shell_snapshot.json` または `GUI_SHELL_SNAPSHOT_JSON` を読む。言語は Python tooling + Dart 診断 client。分類は development / diagnostic-only path。証拠は `tooling/shell_snapshot.py`、`apps/desktop_flutter/lib/services/shell_core_client.dart`。方針差分は Shell snapshot の生成が Python Shell Core に依存するが、インストール済み製品の権限主体であってはならないこと。
- `apps/mobile_flutter`: 現在経路は静的 Flutter companion screen。言語は Dart。分類は post_v1 / display-only scaffold。証拠は `apps/mobile_flutter/lib/main.dart`、`apps/mobile_flutter/lib/screens/*.dart`。方針差分は broker / Shell Core IPC が未実装であること。
- `installer/windows`: 現在経路は installed broker smoke、Setup Doctor evidence、UIAutomation visible-surface capture、インストール済み exe の launch evidence。言語は PowerShell。分類は release-only evidence collector。証拠は `installer/windows/collect_broker_smoke.ps1`、`installer/windows/collect_setup_doctor.ps1`、`installer/windows/collect_installed_smoke.ps1`。方針差分は release claim 前に測定済み native Windows evidence の収集が必要であること。
- `packages/shell_core`: 現在経路は framework 非依存の Shell Core semantics と parity oracle。言語は Python。分類は development / conformance / migration oracle。証拠は `packages/shell_core/*.py`、`tooling/shell_snapshot.py`、`tooling/conformance_tests/run_conformance_skeleton.py`、`tooling/broker_parity/run_authority_parity.py`。方針差分は Rust parity proof 完了まで Python を oracle として維持する一方、インストール済み製品の権限主体にはしないこと。
- `packages/blue_tanuki_adapter`: 現在経路は reference adapter の mock output。言語は Python。分類は test/reference adapter scaffold。証拠は `packages/blue_tanuki_adapter/*.py`。方針差分は adapter oracle としてだけ許可し、release runtime owner にはしないこと。
- `packages/runtime_catalog`: 現在経路は manifest registration と metadata authority check。言語は Python。分類は validation / migration oracle。証拠は `packages/runtime_catalog/catalog.py`、`tooling/release_smoke.py`。方針差分は active release path に Rust broker target が必要であること。
- `packages/agent_runtime`: 現在経路は workspace と tool-call contract helper。言語は Python。分類は validation / migration oracle。証拠は `packages/agent_runtime/contract.py`、`tooling/release_smoke.py`。方針差分は command eligibility に Rust broker target が必要であること。
- `native/rust_helper`: 現在経路は bounded helper module に加え、broker-server binary / authenticated loopback IPC / durable store / authority parity operation。言語は Rust。分類は broker authority path 開始済みで製品 UI client の target。証拠は `native/rust_helper/Cargo.toml`、`native/rust_helper/src/main.rs`、`native/rust_helper/src/broker/*.rs`、`native/rust_helper/tests/broker_ipc.rs`、`tooling/broker_parity/run_authority_parity.py`、`tooling/release_runtime_assertions.py`。方針差分は command dispatch が suspended で、installed no-Python-runtime proof と Windows installed-path proof がないこと。
- `tooling/release_smoke.py`: 現在経路は統合 Shell Core / installer / runtime catalog / agent runtime smoke。言語は Python。分類は development validation。証拠は `tooling/release_smoke.py`。方針差分は tooling として有効だが product runtime proof ではないこと。
- `tooling/validate_all.py`: 現在経路は release runtime assertion を含む subprocess validation orchestrator。言語は Python。分類は local validation tooling。証拠は `tooling/validate_all.py`、`tooling/release_runtime_assertions.py`。方針差分は明示的 validation tooling として許可すること。
- `.github/workflows/*.yml`: 現在経路は意図的に不在で、GitHub Actions / CI workflow は quality gate ではない。言語は none。分類は absent CI path。証拠は conformance が `.github/workflows` 配下の workflow YAML を拒否すること。方針差分は local validation と installed Windows evidence が quality の基礎であり続けること。

## 3. 責任主体の対応表

- runtime registry の責任: 現在の実装は `packages/shell_core/runtime_registry.py`、`packages/shell_core/runtime_state.py`。言語は Python。分類は active development / owner-use snapshot / validation oracle。目標責任主体は Rust Security Broker + JSON Schema contract。移行は yes。証拠は `packages/shell_core/__init__.py` から import され、conformance と release smoke で実行されること。
- capability evaluation の責任: 現在の実装は `packages/shell_core/policy_evaluator.py`。言語は Python。分類は active validation oracle。目標責任主体は Rust Security Broker。移行は yes。証拠は conformance が `PolicyEvaluator` の negative case を呼ぶこと。
- permission ledger の責任: 現在の実装は `packages/shell_core/permission_ledger.py`、`packages/shell_core/runtime_state.py`。言語は Python。分類は active validation oracle。目標責任主体は Rust Security Broker。移行は yes。証拠は `PermissionLedger`、`RuntimeState.record_permission`、conformance policy test。
- approval queue の責任: 現在の実装は `packages/shell_core/approval_queue.py`。言語は Python。分類は active validation oracle。目標責任主体は Rust Security Broker。移行は yes。証拠は protected edit と rehash の conformance test が `ApprovalQueue` を import すること。
- approval finalization の責任: 現在の実装は Rust broker `approval_edit` + Python oracle parity。言語は Rust broker path + Python oracle。分類は product UI probe / validation oracle。目標責任主体は Rust Security Broker。active dispatch には移行が yes。証拠は broker parity が approved/pending state、protected edit rejection、rehash、`requires_validation` transition を検証し、Flutter product client が protected-field rejection を probe すること。
- audit append の責任: 現在の実装は `packages/shell_core/audit_store.py`、`packages/shell_core/persistence.py`。言語は Python。分類は development smoke。目標責任主体は Rust Security Broker。移行は yes。証拠は `run_shell_core_release_smoke()` が JSONL audit event を追記すること。
- audit chain verification の責任: 現在の実装は `packages/shell_core/audit_chain.py`、`packages/shell_core/persistence.py`。言語は Python。分類は development smoke。目標責任主体は Rust Security Broker。移行は yes。証拠は release smoke が chain と tamper detection を検証すること。
- recovery classification の責任: 現在の実装は `packages/shell_core/recovery_catalog.py`、`packages/blue_tanuki_adapter/recovery.py`。言語は Python。分類は validation / adapter oracle。目標責任主体は governed path の Rust Security Broker で、adapter は candidate を提案してよい。移行は yes。証拠は conformance が recovery ID と adapter recovery fixture を要求すること。
- command dispatch eligibility の責任: 現在の実装は `packages/shell_core/policy_evaluator.py`、`packages/shell_core/sensitive_action_router.py`、`packages/agent_runtime/contract.py`。言語は Python。分類は validation oracle で real dispatch はない。目標責任主体は Rust Security Broker。移行は yes。証拠は command permission mapping の test と、実際の external command dispatch が disabled であること。
- content visibility enforcement の責任: 現在の実装は Rust broker `content_projection` + Python oracle parity で、projected snapshot を Flutter が利用する。言語は Rust broker path + Python oracle + Dart display。分類は product UI projection / validation oracle。目標責任主体は projection authority の Rust Security Broker で、Flutter は display 専用。installed proof には移行が yes。証拠は broker parity と Flutter product client fixture が redacted projection を強制し、installed Windows proof はないこと。
- update verification の責任: 現在の実装は `packages/shell_core/update_policy_store.py`、`native/rust_helper/src/update_verification.rs`。言語は Python + Rust。分類は partial helper / validation。目標責任主体は Rust Security Broker。移行は yes。証拠は Rust が signature presence だけを検査し、Shell Core policy は Python であること。
- credential/keychain access の責任: 現在の実装と言語は none。分類は現在の経路で未実装。目標責任主体は Rust Security Broker。credential feature 前の移行は yes。証拠は credential implementation が見つからないこと。
- process supervision の責任: 現在の実装は `native/rust_helper/src/process.rs` で、Windows smoke は `Start-Process` を使う。言語は Rust + PowerShell。分類は helper diagnostics / release-only collector。目標責任主体は Rust Security Broker。移行は yes。証拠は Rust helper が任意 command execution を拒否し、collector は release evidence 専用であること。
- IPC endpoint ownership の責任: 現在の実装は `native/rust_helper/src/ipc.rs`、`native/rust_helper/src/broker/protocol.rs`、`native/rust_helper/src/broker/ipc_server.rs`、`native/rust_helper/src/main.rs`、`apps/desktop_flutter/lib/services/broker_client.dart`。言語は Rust + Dart IPC client。分類は authenticated loopback broker-server + Flutter product client。目標責任主体は Rust Security Broker。installed proof には移行が yes。証拠は JSON envelope / replay rejection、production listener、authenticated local session、shutdown、unavailable-after-shutdown test が存在し、Flutter client code と fail-closed test が存在する一方、Windows installed-session evidence がないこと。
- adapter conformance enforcement の責任: 現在の実装は `packages/shell_core/adapter_loader.py` と conformance test。言語は Python。分類は validation oracle。目標責任主体は Rust Security Broker + schema/conformance。移行は yes。証拠は adapter metadata の authority stripping が Python であること。
- evidence reporting の責任: 現在の実装は `tooling/evidence_bundle.py`、`tooling/windows_release_evidence.py`、`tooling/validate_all.py`。言語は Python。分類は local validation / release evidence tooling。目標責任主体は tooling として許可された Python。tooling について移行は no。証拠は product runtime の authority claim がないこと。

## 4. Python の分類

- `tooling/schema_check/check_schemas.py`: 分類 A の dev-only allowed。移行中は yes。証拠は schema validation 専用。リリースへの影響は none。
- `tooling/conformance_tests/run_conformance_skeleton.py`: 分類 A の dev-only allowed。移行中は yes。証拠は production Python code を conformance target として import すること。tooling としての影響は none だが、product runtime proof にしてはならない。
- `tooling/validate_all.py`: 分類 A の local validation allowed。移行中は yes。証拠は release runtime assertion を含む subprocess validation orchestrator。tooling としての影響は none。
- `tooling/release_runtime_assertions.py`: 分類 A の local/release validation support として許可する。移行中は yes。証拠は Python authority process を起動しないこと、Python snapshot generator を呼び出さないこと、FFI/direct bridge がないこと、broker-mediated authority operation、fail-closed test、broker restart/crash coverage に対する静的な製品 authority path assertion。tooling としての影響は none。
- `tooling/windows_release_evidence.py`: 分類 A の release evidence validator として許可する。移行中は yes。証拠は installed smoke JSON の validation。tooling としての影響は none。
- `tooling/manifest.py`: 分類 A の local validation allowed。移行中は yes。証拠は source hash manifest。リリースへの影響は none。
- `tooling/shell_snapshot.py`: 分類 B の temporary migration oracle / development diagnostic generator。移行中は temporarily yes。証拠は local diagnostic client と tooling が呼び出せる一方、製品 `main.dart` は authority のために読まなくなったこと。不要であることを installed no-Python-runtime proof が示すまで `release_blocker`。
- `packages/shell_core/*.py`: 分類 B/C の migration parity oracle + development tooling。移行中は temporarily yes。証拠は policy、approval、audit、recovery、snapshot、content visibility を実装し、Rust broker parity が比較すること。installed product proof が Python authority runtime を除外するまで `release_blocker`。
- `packages/runtime_catalog/*.py`: 分類 B の migration oracle。移行中は temporarily yes。証拠は release smoke と conformance が import すること。governed active path を Rust broker へ移す必要がある。
- `packages/agent_runtime/*.py`: 分類 B の migration oracle。移行中は temporarily yes。証拠は release smoke が command/workspace contract を検証すること。command eligibility path を Rust broker へ移す必要がある。
- `packages/blue_tanuki_adapter/*.py`: 分類 B の reference adapter oracle。移行中は temporarily yes。証拠は mock adapter と conformance に限ること。GUI-Shell authority owner ではない。
- `installer/setup_doctor.py`、`installer/first_run.py`: 分類 A/B の development + evidence smoke tooling。移行中は temporarily yes。証拠は release smoke が import し、installed-path evidence が存在しないこと。明示的に分類して除外しない限り、製品 Setup Doctor が Python runtime を必要としてはならない。

## 5. 証拠源の分類

- schema check の証拠: source class は CONFIG / FIXTURE。schema と example の parse、および invalid fixture の拒否を証明する。live runtime safety は証明しない。
- conformance skeleton の証拠: source class は CONFIG / INTERNAL_STATE / FIXTURE。Python implementation が current contract behavior を維持することを証明する。Rust broker behavior または installed product behavior は証明しない。
- Shell snapshot の証拠: source class は INTERNAL_STATE / FIXTURE。生成した local diagnostic projection を証明する。broker 由来の live runtime state は証明しない。
- Flutter widget test の証拠: source class は INTERNAL_STATE / FIXTURE。Flutter tooling 実行時に UI が projection を描画し、test/fallback data の hidden payload を隠し、broker fail-closed/product-path fake coverage を含むことを証明する。authority decision の正しさまたは Windows installed-path proof は証明しない。
- Rust helper cargo test の証拠: source class は local broker process の test に対する INTERNAL_STATE / LIVE_RUNTIME。helper function と broker-server が malformed/stale/replayed/authority-like JSON envelope を拒否し、IPC を authenticate し、request size を強制し、audit/replay/session state を永続化し、restart replay を拒否し、command dispatch を suspend することを証明する。Windows installed-path の broker proof と Flutter integration は証明しない。
- broker authority parity の証拠: source class は local broker process の test に対する FIXTURE / LIVE_RUNTIME。Python oracle fixture を Rust broker IPC と比較し、accepted/rejected policy、normalization、approval edit/rehash、content projection、audit verification、recovery mapping、command-envelope eligibility を証明する。installed product runtime と Windows installed-path proof は証明しない。
- release runtime assertion の証拠: source class は local broker process の test 存在に対する CONFIG / FIXTURE / LIVE_RUNTIME。製品 Flutter entry が broker-mediated であること、Python authority process の startup と Python snapshot generator の invocation が製品 path にないこと、FFI/direct bridge token が authority surface scan にないこと、broker fail-closed test と local restart の persistence test が存在することを証明する。installed Windows の product runtime と Python-not-installed artifact proof は証明しない。
- Flutter broker product client の証拠: source class は Flutter test 実行まで INTERNAL_STATE / FIXTURE で、app launch 時に意図するのは LIVE_RUNTIME。製品 UI code が broker client を通り、broker unavailable/auth/stale/malformed state で fail closed することを証明する。Windows installed-path proof と active command dispatch は証明しない。
- Windows installed smoke validator の証拠: real JSON が存在する場合の source class は EXTERNAL_EVIDENCE。non-synthetic JSON が存在する場合の installed-path Windows evidence を証明する。現在の repository にその evidence file があることは証明しない。

現在のどの証拠源も、インストール済み経路の `LIVE_RUNTIME` Rust Security Broker 権限責任主体を証明しない。

## 6. A-F への直接回答

A. `packages/shell_core/*.py` は active product runtime か。
completed product runtime は現時点で成立していない。Flutter product entry は broker-mediated path に切り替わったが、installed no-Python-runtime proof と Windows installed-path proof が未取得であるため、`packages/shell_core/*.py` は migration parity oracle / development tooling として残す。

B. Python Shell Core は test oracle / fixture / scaffold のみか。
product `main.dart` の authority state では直接使わない。local diagnostic snapshot / conformance / parity oracle ではまだ使われるため、installed product no-Python-runtime proof が必要である。

C. Flutter desktop app は現在どの実行主体から状態を取得しているか。
Dart の `ShellCoreClient.product()` が `BrokerClient` 経由で Rust broker response を読む。broker unavailable / auth failure / stale session / malformed response 時は broker_unavailable SUSPEND snapshot を返し、local JSON を authority として使わない。`ShellCoreClient.local()` は diagnostic-only で残る。

D. Rust helper は standalone process か、library/FFI 前提か、まだ未接続か。
Rust helper crate に broker-server binary が追加され、Flutter product client はその independent process へ authenticated loopback IPC で接続する。authority-sensitive path に FFI は使っていない。Windows installed-path 接続証拠はまだない。

E. approval / permission / audit / recovery の最終判定主体が現時点で何か。
Flutter product path 上の broker health / content projection / approval の protected-field edit probe / command-envelope eligibility は Rust broker response 由来である。実際の external command dispatch はまだ suspended であり、completed product runtime の最終判定主体として release-ready claim はできない。

F. Python を除去せずに runtime 非依存へ格下げできる範囲と、Rust へ移植が必要な範囲はどこか。
schema validation、conformance、release evidence、migration parity oracle は Python のまま残せる。authority key normalization、permission eligibility、approval protected-field enforcement、content visibility projection、audit verification、recovery classification、command-envelope eligibility は Rust broker parity path に移植済みで、Flutter product client からも broker response として取得する。product path に Python authority process startup がないことと no-FFI/direct-bridge assertion は `tooling/release_runtime_assertions.py --check` で固定済みである。process/credential/update gated execution、installed no-Python-runtime proof、Windows installed-path proof は未実装である。

## 2026-09-25 更新：local診断snapshot境界

本監査の2026-06-03時点の観測履歴は保持する。現行コードでは`ShellCoreClient.local()`の環境変数・filesystem読込を除去し、明示注入されたメモリ内`ShellSnapshot`だけを診断表示に使う。入力出所と鮮度はunknown、release claimは抑止する。製品状態は引き続きBroker経路である。

Surface Semanticsのbuild registry file出力も製品起動経路から除去した。登録状態はwidget testでだけ検査し、可視性は外部UIAutomation treeで測定する。Flutter内の直接file read/writeは`setup_doctor_export.dart`に残り、この監査・移譲を別単位で要する。
