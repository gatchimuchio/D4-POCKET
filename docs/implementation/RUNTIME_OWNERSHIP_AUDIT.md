# Runtime Ownership Audit

Status: Phase 0 current-state audit
Date: 2026-06-01
Scope: Language Policy Runtime Convergence / Rust Security Broker Migration v0.1

この文書は、現行 GUI-Shell の実行経路、検証経路、release 判定経路で、どの言語・どのファイルが責任を持っているかを記録する。ここでの「active」は completed product release を意味しない。現時点で存在する owner-use、development validation、release evidence 経路において実際に呼ばれている、または責任実装として扱われていることを指す。

## 1. 監査結論

- `packages/shell_core/*.py` は、現行の runtime registry、permission、approval、audit、recovery、content exposure、policy evaluation セマンティクスを実装している。
- Flutter desktop app は Python を直接実行しない。`ShellCoreClient.local()` が `GUI_SHELL_SNAPSHOT_JSON`、`%LOCALAPPDATA%\GUI-Shell\shell_snapshot.json`、`.gui_shell/shell_snapshot.json` を読む。
- owner launch scripts は Flutter 起動前に Python `tooling/shell_snapshot.py` を実行し、その中で Python Shell Core を使う。
- Rust helper は library crate であり、独立 process の Rust Security Broker ではない。`src/main.rs`、broker lifecycle、authenticated IPC endpoint、approval / audit finalization endpoint は存在しない。
- BLUE-TANUKI は adapter mock/reference の範囲に留まっており、BLUE-TANUKI core の変更経路は確認されなかった。
- TypeScript / Node の GUI-Shell core runtime 導入は確認されなかった。
- authority path への `flutter_rust_bridge`、`dart:ffi`、`MethodChannel`、Rust FFI 直結は確認されなかった。

Release-gate conclusion:

- item: active Shell Core language policy convergence
  classification: release_blocker
  reason: 現行の唯一の Shell Core authority-sensitive 実装は Python で、Rust Security Broker の independent process runtime が存在しない。
  required_action: authority-sensitive active production path を Rust broker に移し、Python を dev/test/migration oracle に格下げした証拠を追加する。
  blocks_release: yes

## 2. 実行経路監査

| entrypoint | current path | language | classification | evidence | policy gap |
| --- | --- | --- | --- | --- | --- |
| `apps/desktop_flutter` | `main.dart` -> `ShellCoreClient.local()` -> local JSON file / fallback snapshot | Dart | active owner-use UI path | `apps/desktop_flutter/lib/main.dart`, `apps/desktop_flutter/lib/services/shell_core_client.dart` | UI は authority を所有しないが、broker IPC には未接続 |
| owner desktop launch | script creates `.gui_shell/shell_snapshot.json`, exports `GUI_SHELL_SNAPSHOT_JSON`, then runs Flutter | Python + Dart | active owner-use launch path | `scripts/launch_owner_desktop.sh`, `scripts/launch_owner_desktop.ps1` | Shell snapshot generation depends on Python Shell Core |
| `apps/mobile_flutter` | static Flutter companion screens | Dart | post_v1 / display-only scaffold | `apps/mobile_flutter/lib/main.dart`, `apps/mobile_flutter/lib/screens/*.dart` | broker / Shell Core IPC is not implemented |
| `installer/windows` | installed exe launch collector consumes Setup Doctor JSON and UI evidence JSON, writes release evidence | PowerShell | release-only evidence collector | `installer/windows/collect_installed_smoke.ps1` | product Setup Doctor path is not proven and broker is absent |
| `packages/shell_core` | framework-independent Shell Core semantics | Python | active development / owner-use snapshot / validation oracle | `packages/shell_core/*.py`, `tooling/shell_snapshot.py`, `tooling/conformance_tests/run_conformance_skeleton.py` | Python owns current authority-sensitive implementation |
| `packages/blue_tanuki_adapter` | reference adapter mock outputs | Python | test/reference adapter scaffold | `packages/blue_tanuki_adapter/*.py` | allowed only as adapter oracle; not release runtime owner |
| `packages/runtime_catalog` | manifest registration and metadata authority checks | Python | validation / migration oracle | `packages/runtime_catalog/catalog.py`, `tooling/release_smoke.py` | Rust broker target needed for active release path |
| `packages/agent_runtime` | workspace and tool-call contract helper | Python | validation / migration oracle | `packages/agent_runtime/contract.py`, `tooling/release_smoke.py` | Rust broker target needed for command eligibility |
| `native/rust_helper` | helper library modules and cargo tests | Rust | bounded helper / non-broker | `native/rust_helper/Cargo.toml`, `native/rust_helper/src/*.rs` | no independent broker process or authority owner |
| `tooling/release_smoke.py` | integrated Shell Core, installer, runtime catalog, agent runtime smoke | Python | development validation | `tooling/release_smoke.py` | valid as CI/tooling, not product runtime proof |
| `tooling/validate_all.py` | subprocess validation orchestrator | Python | CI / validation tooling | `tooling/validate_all.py` | allowed as CI support |
| `.github/workflows/validation.yml` | split Python core, Rust helper, Flutter, Windows build jobs | YAML + Python/Rust/Dart | CI | `.github/workflows/validation.yml` | no no-Python-runtime assertion yet |

## 3. Responsibility Ownership Table

| responsibility | current implementation file | current language | current classification | target owner | migration required | evidence |
| --- | --- | --- | --- | --- | --- | --- |
| runtime registry | `packages/shell_core/runtime_registry.py`, `packages/shell_core/runtime_state.py` | Python | active development / owner-use snapshot / validation oracle | Rust Security Broker plus JSON Schema contracts | yes | imported by `packages/shell_core/__init__.py`; exercised by conformance and release smoke |
| capability evaluation | `packages/shell_core/policy_evaluator.py` | Python | active validation oracle | Rust Security Broker | yes | conformance calls `PolicyEvaluator` negative cases |
| permission ledger | `packages/shell_core/permission_ledger.py`, `packages/shell_core/runtime_state.py` | Python | active validation oracle | Rust Security Broker | yes | `PermissionLedger`, `RuntimeState.record_permission`, conformance policy tests |
| approval queue | `packages/shell_core/approval_queue.py` | Python | active validation oracle | Rust Security Broker | yes | protected edit and rehash conformance tests import `ApprovalQueue` |
| approval finalization | `packages/shell_core/policy_evaluator.py`, `packages/shell_core/approval_queue.py` | Python | partial validation oracle | Rust Security Broker | yes | current code validates `status == approved`; no broker finalization endpoint |
| audit append | `packages/shell_core/audit_store.py`, `packages/shell_core/persistence.py` | Python | development smoke | Rust Security Broker | yes | `run_shell_core_release_smoke()` appends JSONL audit events |
| audit chain verification | `packages/shell_core/audit_chain.py`, `packages/shell_core/persistence.py` | Python | development smoke | Rust Security Broker | yes | release smoke verifies chain and tamper detection |
| recovery classification | `packages/shell_core/recovery_catalog.py`, `packages/blue_tanuki_adapter/recovery.py` | Python | validation / adapter oracle | Rust Security Broker for governed path; adapter may propose candidates | yes | conformance requires recovery IDs and adapter recovery fixture |
| command dispatch eligibility | `packages/shell_core/policy_evaluator.py`, `packages/shell_core/sensitive_action_router.py`, `packages/agent_runtime/contract.py` | Python | validation oracle; no real dispatch | Rust Security Broker | yes | command permission mapping tested; real external command dispatch is disabled |
| content visibility enforcement | `packages/shell_core/content_exposure.py`, `packages/blue_tanuki_adapter/approvals.py`, projected snapshot consumed by Flutter | Python + Dart display | active display projection / validation oracle | Rust Security Broker for projection authority; Flutter display only | yes | conformance and Flutter tests confirm hidden payload is not rendered |
| update verification | `packages/shell_core/update_policy_store.py`, `native/rust_helper/src/update_verification.rs` | Python + Rust | partial helper / validation | Rust Security Broker | yes | Rust checks signature presence only; Shell Core policy is Python |
| credential/keychain access | none | none | not implemented in current path | Rust Security Broker | yes before credential feature | no credential implementation found |
| process supervision | `native/rust_helper/src/process.rs`; Windows smoke uses `Start-Process` | Rust + PowerShell | helper diagnostics / release-only collector | Rust Security Broker | yes | Rust helper refuses arbitrary command execution; collector is release evidence only |
| IPC endpoint ownership | `native/rust_helper/src/ipc.rs` | Rust | frame validation helper only | Rust Security Broker | yes | no listener, session, authentication, replay protection, or crash handling endpoint |
| adapter conformance enforcement | `packages/shell_core/adapter_loader.py`, conformance tests | Python | validation oracle | Rust Security Broker plus schema/conformance | yes | adapter metadata authority stripping is Python |
| evidence reporting | `tooling/evidence_bundle.py`, `tooling/windows_release_evidence.py`, `tooling/validate_all.py` | Python | CI / release evidence tooling | Python allowed as tooling | no for tooling | no product runtime authority claim |

## 4. Python Classification

| path | classification | allowed during migration | evidence | release impact |
| --- | --- | --- | --- | --- |
| `tooling/schema_check/check_schemas.py` | A. dev-only allowed | yes | schema validation only | none |
| `tooling/conformance_tests/run_conformance_skeleton.py` | A. dev-only allowed | yes | imports production Python code as conformance target | none as tooling; must not be product runtime proof |
| `tooling/validate_all.py` | A. CI support allowed | yes | subprocess validation orchestrator | none as tooling |
| `tooling/windows_release_evidence.py` | A. release evidence validator allowed | yes | validates installed smoke JSON | none as tooling |
| `tooling/manifest.py` | A. CI support allowed | yes | source hash manifest | none |
| `tooling/shell_snapshot.py` | B. temporary migration oracle; currently active owner-use generator | yes temporarily | launch scripts call it before Flutter | release_blocker until product path stops depending on it |
| `packages/shell_core/*.py` | B/C. migration oracle, but current authority-sensitive implementation | yes temporarily | implements policy, approval, audit, recovery, snapshot, content visibility | release_blocker for completed product release |
| `packages/runtime_catalog/*.py` | B. migration oracle | yes temporarily | release smoke and conformance import it | must move governed active path to Rust broker |
| `packages/agent_runtime/*.py` | B. migration oracle | yes temporarily | release smoke validates command/workspace contract | must move command eligibility path to Rust broker |
| `packages/blue_tanuki_adapter/*.py` | B. reference adapter oracle | yes temporarily | mock adapter and conformance only | not a GUI-Shell authority owner |
| `installer/setup_doctor.py`, `installer/first_run.py` | A/B. development and evidence smoke tooling | yes temporarily | release smoke imports them; installed-path evidence is not present | product Setup Doctor must not require Python runtime unless explicitly classified and excluded |

## 5. Evidence Source Classification

| evidence | source class | proves | does not prove |
| --- | --- | --- | --- |
| schema check | CONFIG / FIXTURE | schemas and examples parse and reject invalid fixtures | live runtime safety |
| conformance skeleton | CONFIG / INTERNAL_STATE / FIXTURE | Python implementation preserves current contract behavior | Rust broker behavior or installed product behavior |
| Shell snapshot | INTERNAL_STATE / FIXTURE | generated local owner-operation projection | live runtime state from broker |
| Flutter widget tests | INTERNAL_STATE / FIXTURE | UI renders projections and hides hidden payload in test/fallback data | authority decision correctness |
| Rust helper cargo tests | INTERNAL_STATE | helper library functions reject some unsafe helper actions | broker lifecycle, authenticated IPC, approval/audit finalization |
| Windows installed smoke validator | EXTERNAL_EVIDENCE when real JSON exists | installed-path Windows evidence if non-synthetic JSON exists | current repository has no such evidence file |

No current evidence source proves `LIVE_RUNTIME` Rust Security Broker authority ownership.

## 6. Direct Answers A-F

A. `packages/shell_core/*.py` は active product runtime か。
completed product runtime は現時点で成立していない。ただし owner-use active path と validation path の唯一の Shell Core 実装であり、authority-sensitive 実装として release-ready 前に Rust broker へ移管または oracle 化が必要。

B. Python Shell Core は test oracle / fixture / scaffold のみか。
いいえ。conformance/test oracle であるだけでなく、owner launch scripts が生成する local snapshot の元実装でもある。

C. Flutter desktop app は現在どの実行主体から状態を取得しているか。
Dart の `ShellCoreClient.local()` が JSON file を読む。JSON は `tooling/shell_snapshot.py` で生成される場合があり、missing/parse failure 時は Dart fallback snapshot を使う。

D. Rust helper は standalone process か、library/FFI 前提か、まだ未接続か。
現状は Rust library crate で、standalone process ではない。Flutter との FFI も IPC endpoint も未接続。

E. approval / permission / audit / recovery の最終判定主体が現時点で何か。
現行コード上は Python Shell Core が判定主体である。Flutter は表示のみ。Rust helper は最終判定主体ではない。

F. Python を除去せずに runtime 非依存へ格下げできる範囲と、Rust へ移植が必要な範囲はどこか。
schema validation、conformance、release evidence、migration parity oracle は Python のまま残せる。authority key normalization、permission eligibility、approval protected-field enforcement、content visibility projection、audit append/hash-chain、recovery classification、command-envelope eligibility、process/credential/update gated execution は Rust broker への移植が必要。
