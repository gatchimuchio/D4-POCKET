# Runtime Ownership Audit

Status: Phase 0 current-state audit updated after Flutter broker product path start
Date: 2026-06-03
Scope: Language Policy Runtime Convergence / Rust Security Broker Migration v0.1

この文書は、現行 GUI-Shell の実行経路、検証経路、release 判定経路で、どの言語・どのファイルが責任を持っているかを記録する。ここでの「active」は completed product release を意味しない。現時点で存在する owner-use、development validation、release evidence 経路において実際に呼ばれている、または責任実装として扱われていることを指す。

## 1. 監査結論

- `packages/shell_core/*.py` は、現行の runtime registry、permission、approval、audit、recovery、content exposure、policy evaluation セマンティクスを実装している。
- Flutter desktop product entry は Python を直接実行しない。`main.dart` は `ShellCoreClient.product()` を呼び、`BrokerClient` が Rust broker process の endpoint file を読み、authenticated `127.0.0.1` loopback IPC で health / normalization / content projection / approval edit / command envelope status を取得する。
- `ShellCoreClient.local()` は `GUI_SHELL_SNAPSHOT_JSON`、`%LOCALAPPDATA%\GUI-Shell\shell_snapshot.json`、`.gui_shell/shell_snapshot.json` を読むが、development / diagnostic-only path として残され、product authority state には使われない。
- owner launch scripts は Rust broker を起動し、`GUI_SHELL_BROKER_ENDPOINT_JSON` を Flutter に渡す。Python `tooling/shell_snapshot.py` は local diagnostic / parity tooling として残る。ただし installed product no-Python-runtime proof は未取得である。
- Rust helper crate 内に Rust Security Broker production process と authority parity path の初期実装が追加され、`broker-server`、authenticated `127.0.0.1` loopback IPC、durable audit/replay/session file store、health/shutdown process tests、stale/replay/malformed/authority-metadata rejection、restart replay rejection、tampered persisted-state rejection、audit hash-chain restart verification、Python oracle parity for normalization / permission / approval / content projection / audit / recovery / command-envelope eligibility が存在する。Flutter broker product path code と fail-closed test coverage は追加され、Windows Flutter `flutter.bat` 経由の analyze/test は通過した。`tooling/release_runtime_assertions.py --check` は `tooling/validate_all.py` と `tooling/evidence_bundle.py --check` に接続され、product authority path の Python process startup 不在、Python snapshot generator invocation 不在、no-ffi-authority direct-bridge assertion、broker-mediated authority operations を検証する。WSL direct `flutter` は外部 Flutter shell scripts の CRLF 問題で exit 127 のため、`validate_all.py` の Flutter subchecks はこの環境では失敗する。Windows installed-session evidence、real execution gates は未完了である。
- BLUE-TANUKI は adapter mock/reference の範囲に留まっており、BLUE-TANUKI core の変更経路は確認されなかった。
- TypeScript / Node の GUI-Shell core runtime 導入は確認されなかった。
- authority path への `flutter_rust_bridge`、`dart:ffi`、`MethodChannel`、Rust FFI 直結は確認されなかった。

Release-gate conclusion:

- item: active Shell Core language policy convergence
  classification: release_blocker
  reason: Rust Security Broker process、production IPC としての authenticated loopback IPC、durable store、Python oracle parity harness、Flutter broker product path code、release runtime assertions、Windows Flutter analyze/test evidence は存在する。ただし broker health は `authority_cutover_status=not_active`、command dispatch は suspended、installed no-Python-runtime evidence、Windows installed-path broker evidence は未完了である。
  required_action: Windows installed path で broker launch/connect/fail-closed を実測し、Python を dev/test/migration oracle に格下げした証拠と Windows installed-path broker evidence を追加する。
  blocks_release: yes

## 2. 実行経路監査

| entrypoint | current path | language | classification | evidence | policy gap |
| --- | --- | --- | --- | --- | --- |
| `apps/desktop_flutter` | `main.dart` -> `ShellCoreClient.product()` -> `BrokerClient` -> authenticated loopback IPC -> Rust broker response / fail-closed snapshot | Dart + Rust IPC | active product UI path started | `apps/desktop_flutter/lib/main.dart`, `apps/desktop_flutter/lib/services/broker_client.dart`, `apps/desktop_flutter/lib/services/shell_core_client.dart` | command dispatch suspended; broker reports `authority_cutover_status=not_active`; Flutter validation evidence is environment-blocked in this session |
| owner desktop launch | script starts Rust broker, exports `GUI_SHELL_BROKER_ENDPOINT_JSON`, then runs Flutter | Shell / PowerShell + Rust + Dart | development product-path launch | `scripts/launch_owner_desktop.sh`, `scripts/launch_owner_desktop.ps1`, `apps/desktop_flutter/lib/services/broker_client.dart` | installed Windows service/supervisor proof absent |
| `ShellCoreClient.local()` | optional local diagnostic client reads `.gui_shell/shell_snapshot.json` or `GUI_SHELL_SNAPSHOT_JSON` in tests/dev | Python tooling + Dart diagnostic client | development / diagnostic-only path | `tooling/shell_snapshot.py`, `apps/desktop_flutter/lib/services/shell_core_client.dart` | Shell snapshot generation depends on Python Shell Core but must not be installed product authority |
| `apps/mobile_flutter` | static Flutter companion screens | Dart | post_v1 / display-only scaffold | `apps/mobile_flutter/lib/main.dart`, `apps/mobile_flutter/lib/screens/*.dart` | broker / Shell Core IPC is not implemented |
| `installer/windows` | installed broker smoke, Setup Doctor evidence, UIAutomation visible-surface capture, and installed exe launch evidence | PowerShell | release-only evidence collector | `installer/windows/collect_broker_smoke.ps1`, `installer/windows/collect_setup_doctor.ps1`, `installer/windows/collect_installed_smoke.ps1` | measured native Windows evidence must be collected before release claim |
| `packages/shell_core` | framework-independent Shell Core semantics and parity oracle | Python | development / conformance / migration oracle | `packages/shell_core/*.py`, `tooling/shell_snapshot.py`, `tooling/conformance_tests/run_conformance_skeleton.py`, `tooling/broker_parity/run_authority_parity.py` | Python must remain oracle until Rust parity proof is complete, but must not be installed product authority |
| `packages/blue_tanuki_adapter` | reference adapter mock outputs | Python | test/reference adapter scaffold | `packages/blue_tanuki_adapter/*.py` | allowed only as adapter oracle; not release runtime owner |
| `packages/runtime_catalog` | manifest registration and metadata authority checks | Python | validation / migration oracle | `packages/runtime_catalog/catalog.py`, `tooling/release_smoke.py` | Rust broker target needed for active release path |
| `packages/agent_runtime` | workspace and tool-call contract helper | Python | validation / migration oracle | `packages/agent_runtime/contract.py`, `tooling/release_smoke.py` | Rust broker target needed for command eligibility |
| `native/rust_helper` | bounded helper modules plus broker-server binary / authenticated loopback IPC / durable store / authority parity operations | Rust | broker authority path started and product UI client target | `native/rust_helper/Cargo.toml`, `native/rust_helper/src/main.rs`, `native/rust_helper/src/broker/*.rs`, `native/rust_helper/tests/broker_ipc.rs`, `tooling/broker_parity/run_authority_parity.py`, `tooling/release_runtime_assertions.py` | command dispatch suspended; installed no-Python-runtime proof and Windows installed-path proof absent |
| `tooling/release_smoke.py` | integrated Shell Core, installer, runtime catalog, agent runtime smoke | Python | development validation | `tooling/release_smoke.py` | valid as CI/tooling, not product runtime proof |
| `tooling/validate_all.py` | subprocess validation orchestrator, including release runtime assertions | Python | CI / validation tooling | `tooling/validate_all.py`, `tooling/release_runtime_assertions.py` | allowed as CI support |
| `.github/workflows/validation.yml` | split Python core, Rust helper, Flutter, Windows build jobs | YAML + Python/Rust/Dart | CI | `.github/workflows/validation.yml` | release runtime assertion is in `validate_all.py`; installed no-Python-runtime smoke remains Windows evidence scope |

## 3. Responsibility Ownership Table

| responsibility | current implementation file | current language | current classification | target owner | migration required | evidence |
| --- | --- | --- | --- | --- | --- | --- |
| runtime registry | `packages/shell_core/runtime_registry.py`, `packages/shell_core/runtime_state.py` | Python | active development / owner-use snapshot / validation oracle | Rust Security Broker plus JSON Schema contracts | yes | imported by `packages/shell_core/__init__.py`; exercised by conformance and release smoke |
| capability evaluation | `packages/shell_core/policy_evaluator.py` | Python | active validation oracle | Rust Security Broker | yes | conformance calls `PolicyEvaluator` negative cases |
| permission ledger | `packages/shell_core/permission_ledger.py`, `packages/shell_core/runtime_state.py` | Python | active validation oracle | Rust Security Broker | yes | `PermissionLedger`, `RuntimeState.record_permission`, conformance policy tests |
| approval queue | `packages/shell_core/approval_queue.py` | Python | active validation oracle | Rust Security Broker | yes | protected edit and rehash conformance tests import `ApprovalQueue` |
| approval finalization | Rust broker `approval_edit` plus Python oracle parity | Rust broker path + Python oracle | product UI probe / validation oracle | Rust Security Broker | yes for active dispatch | broker parity validates approved/pending state, protected edit rejection, rehash, and requires_validation transition; Flutter product client probes protected-field rejection |
| audit append | `packages/shell_core/audit_store.py`, `packages/shell_core/persistence.py` | Python | development smoke | Rust Security Broker | yes | `run_shell_core_release_smoke()` appends JSONL audit events |
| audit chain verification | `packages/shell_core/audit_chain.py`, `packages/shell_core/persistence.py` | Python | development smoke | Rust Security Broker | yes | release smoke verifies chain and tamper detection |
| recovery classification | `packages/shell_core/recovery_catalog.py`, `packages/blue_tanuki_adapter/recovery.py` | Python | validation / adapter oracle | Rust Security Broker for governed path; adapter may propose candidates | yes | conformance requires recovery IDs and adapter recovery fixture |
| command dispatch eligibility | `packages/shell_core/policy_evaluator.py`, `packages/shell_core/sensitive_action_router.py`, `packages/agent_runtime/contract.py` | Python | validation oracle; no real dispatch | Rust Security Broker | yes | command permission mapping tested; real external command dispatch is disabled |
| content visibility enforcement | Rust broker `content_projection` plus Python oracle parity; projected snapshot consumed by Flutter | Rust broker path + Python oracle + Dart display | product UI projection / validation oracle | Rust Security Broker for projection authority; Flutter display only | yes for installed proof | broker parity and Flutter product client fixture enforce redacted projection; installed Windows proof absent |
| update verification | `packages/shell_core/update_policy_store.py`, `native/rust_helper/src/update_verification.rs` | Python + Rust | partial helper / validation | Rust Security Broker | yes | Rust checks signature presence only; Shell Core policy is Python |
| credential/keychain access | none | none | not implemented in current path | Rust Security Broker | yes before credential feature | no credential implementation found |
| process supervision | `native/rust_helper/src/process.rs`; Windows smoke uses `Start-Process` | Rust + PowerShell | helper diagnostics / release-only collector | Rust Security Broker | yes | Rust helper refuses arbitrary command execution; collector is release evidence only |
| IPC endpoint ownership | `native/rust_helper/src/ipc.rs`, `native/rust_helper/src/broker/protocol.rs`, `native/rust_helper/src/broker/ipc_server.rs`, `native/rust_helper/src/main.rs`, `apps/desktop_flutter/lib/services/broker_client.dart` | Rust + Dart IPC client | authenticated loopback broker-server with Flutter product client | Rust Security Broker | yes for installed proof | JSON envelope / replay rejection, production listener, authenticated local session, shutdown, and unavailable-after-shutdown tests exist; Flutter client code and fail-closed tests exist; Windows installed-session evidence is absent |
| adapter conformance enforcement | `packages/shell_core/adapter_loader.py`, conformance tests | Python | validation oracle | Rust Security Broker plus schema/conformance | yes | adapter metadata authority stripping is Python |
| evidence reporting | `tooling/evidence_bundle.py`, `tooling/windows_release_evidence.py`, `tooling/validate_all.py` | Python | CI / release evidence tooling | Python allowed as tooling | no for tooling | no product runtime authority claim |

## 4. Python Classification

| path | classification | allowed during migration | evidence | release impact |
| --- | --- | --- | --- | --- |
| `tooling/schema_check/check_schemas.py` | A. dev-only allowed | yes | schema validation only | none |
| `tooling/conformance_tests/run_conformance_skeleton.py` | A. dev-only allowed | yes | imports production Python code as conformance target | none as tooling; must not be product runtime proof |
| `tooling/validate_all.py` | A. CI support allowed | yes | subprocess validation orchestrator including release runtime assertions | none as tooling |
| `tooling/release_runtime_assertions.py` | A. CI/release validation support allowed | yes | static product authority path assertions for no Python authority process startup, no Python snapshot generator invocation, no FFI/direct bridge, broker-mediated authority operations, fail-closed tests, and broker restart/crash coverage | none as tooling |
| `tooling/windows_release_evidence.py` | A. release evidence validator allowed | yes | validates installed smoke JSON | none as tooling |
| `tooling/manifest.py` | A. CI support allowed | yes | source hash manifest | none |
| `tooling/shell_snapshot.py` | B. temporary migration oracle / development diagnostic generator | yes temporarily | local diagnostic client and tooling may call it; product `main.dart` no longer reads it for authority | release_blocker until installed no-Python-runtime proof shows it is not required |
| `packages/shell_core/*.py` | B/C. migration parity oracle and development tooling | yes temporarily | implements policy, approval, audit, recovery, snapshot, content visibility; Rust broker parity compares against it | release_blocker until installed product proof excludes Python authority runtime |
| `packages/runtime_catalog/*.py` | B. migration oracle | yes temporarily | release smoke and conformance import it | must move governed active path to Rust broker |
| `packages/agent_runtime/*.py` | B. migration oracle | yes temporarily | release smoke validates command/workspace contract | must move command eligibility path to Rust broker |
| `packages/blue_tanuki_adapter/*.py` | B. reference adapter oracle | yes temporarily | mock adapter and conformance only | not a GUI-Shell authority owner |
| `installer/setup_doctor.py`, `installer/first_run.py` | A/B. development and evidence smoke tooling | yes temporarily | release smoke imports them; installed-path evidence is not present | product Setup Doctor must not require Python runtime unless explicitly classified and excluded |

## 5. Evidence Source Classification

| evidence | source class | proves | does not prove |
| --- | --- | --- | --- |
| schema check | CONFIG / FIXTURE | schemas and examples parse and reject invalid fixtures | live runtime safety |
| conformance skeleton | CONFIG / INTERNAL_STATE / FIXTURE | Python implementation preserves current contract behavior | Rust broker behavior or installed product behavior |
| Shell snapshot | INTERNAL_STATE / FIXTURE | generated local diagnostic projection | live runtime state from broker |
| Flutter widget tests | INTERNAL_STATE / FIXTURE | UI renders projections, hides hidden payload in test/fallback data, and includes broker fail-closed/product-path fake coverage when Flutter tooling runs | authority decision correctness or Windows installed-path proof |
| Rust helper cargo tests | INTERNAL_STATE / LIVE_RUNTIME for local broker process tests | helper functions and broker-server reject malformed/stale/replayed/authority-like JSON envelopes, authenticate IPC, enforce request size, persist audit/replay/session state, reject restart replay, and suspend command dispatch | Windows installed-path broker proof and Flutter integration |
| broker authority parity | FIXTURE / LIVE_RUNTIME for local broker process tests | Python oracle fixtures compare against Rust broker IPC for accepted/rejected policy, normalization, approval edit/rehash, content projection, audit verification, recovery mapping, and command-envelope eligibility | installed product runtime and Windows installed-path proof |
| release runtime assertions | CONFIG / FIXTURE / LIVE_RUNTIME for local broker process test presence | product Flutter entry is broker-mediated, Python authority process startup and Python snapshot generator invocation are absent from product path, FFI/direct bridge tokens are absent from authority surface scan, broker fail-closed tests and local restart persistence tests exist | installed Windows product runtime and Python-not-installed artifact proof |
| Flutter broker product client | INTERNAL_STATE / FIXTURE until Flutter tests run; intended LIVE_RUNTIME when app launches | Product UI code routes through broker client and fail-closes broker unavailable/auth/stale/malformed states | Windows installed-path proof and active command dispatch |
| Windows installed smoke validator | EXTERNAL_EVIDENCE when real JSON exists | installed-path Windows evidence if non-synthetic JSON exists | current repository has no such evidence file |

No current evidence source proves installed-path `LIVE_RUNTIME` Rust Security Broker authority ownership.

## 6. Direct Answers A-F

A. `packages/shell_core/*.py` は active product runtime か。
completed product runtime は現時点で成立していない。Flutter product entry は broker-mediated path に切り替わったが、installed no-Python-runtime proof と Windows installed-path proof が未取得であるため、`packages/shell_core/*.py` は migration parity oracle / development tooling として残す。

B. Python Shell Core は test oracle / fixture / scaffold のみか。
product `main.dart` の authority state では直接使わない。local diagnostic snapshot / conformance / parity oracle ではまだ使われるため、installed product no-Python-runtime proof が必要である。

C. Flutter desktop app は現在どの実行主体から状態を取得しているか。
Dart の `ShellCoreClient.product()` が `BrokerClient` 経由で Rust broker response を読む。broker unavailable / auth failure / stale session / malformed response 時は broker_unavailable SUSPEND snapshot を返し、local JSON を authority として使わない。`ShellCoreClient.local()` は diagnostic-only で残る。

D. Rust helper は standalone process か、library/FFI 前提か、まだ未接続か。
Rust helper crate に broker-server binary が追加され、Flutter product client はその independent process へ authenticated loopback IPC で接続する。authority-sensitive path に FFI は使っていない。Windows installed-path 接続証拠はまだない。

E. approval / permission / audit / recovery の最終判定主体が現時点で何か。
Flutter product path 上の broker health / content projection / approval protected-field edit probe / command-envelope eligibility は Rust broker response 由来である。real external command dispatch はまだ suspended であり、completed product runtime の最終判定主体として release-ready claim はできない。

F. Python を除去せずに runtime 非依存へ格下げできる範囲と、Rust へ移植が必要な範囲はどこか。
schema validation、conformance、release evidence、migration parity oracle は Python のまま残せる。authority key normalization、permission eligibility、approval protected-field enforcement、content visibility projection、audit verification、recovery classification、command-envelope eligibility は Rust broker parity path に移植済みで、Flutter product client からも broker response として取得する。product path に Python authority process startup がないことと no-FFI/direct-bridge assertion は `tooling/release_runtime_assertions.py --check` で固定済みである。process/credential/update gated execution、installed no-Python-runtime proof、Windows installed-path proof は未実装である。
