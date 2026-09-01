# GUI-Shell 完了戦略指示

状態: Phase B の owner-use は complete。OSS v1.0 RC または paid／product release ではない
repository の識別子: <code>gatchimuchio/GUI-Shell</code>
主要目標: GUI-Shell を desktop-first AI Runtime／Agent Operation Shell として完成させる
参照 consumer／Runtime: adapter contract のみを介した BLUE-TANUKI

## 0. 現在位置

GUI-Shell には現在、次が存在する。

- Phase 0 の standard／selection lock
- Phase 1 の schema／contract skeleton
- Phase 2 の conformance skeleton
- Phase 3H の Shell Core hardening skeleton
- Phase 4 の Rust helper boundary skeleton
- Phase 5 の BLUE-TANUKI mock reference adapter の skeleton
- Phase 6 の desktop 用 Flutter operator Shell skeleton
- Phase 7 の Installer／Setup Doctor skeleton
- Phase 8 の mobile companion skeleton
- Phase 9 の release-hardening document
- schema 数: 19
- valid example 数: 19
- negative fixture 数: 19
- conformance check 数: 67

現在の claim boundary:

~~~text
Phase B の owner-use completion は存在する。
OSS v1.0 RC には未到達である。
paid／product QC には未到達である。
live BLUE-TANUKI adapter completion には未到達である。
measured Windows installed-path release evidence は未成立である。
mobile は release ready ではない。
~~~

## 1. 譲れない規則

すべての Phase で次の規則に従う。

1. 安全性を第一とする。
2. 堅牢性を第二とする。
3. operator にとっての明瞭さ／UX を第三とする。
4. 製品機能を第四とする。
5. 利便性を最後とする。

快適性のために authority boundary を弱めない。

Shell Core に Flutter を import しない。

Shell Core に BLUE-TANUKI の内部実装を import しない。

owner が明示的に指示しない限り、GUI-Shell の利便性のために BLUE-TANUKI Core を変更しない。

adapter metadata、memory、cache、previous state、local UI state、installer state、mobile state を authority として扱わない。

すべての sensitive action を次へ対応付けなければならない。

- 能力宣言（Capability）
- 許可（Permission）
- 承認状態（Approval state）
- 監査事象（AuditEvent）
- 修復手順（RecoveryAction）

validation が実際に通過していない限り、通過したと主張しない。

## 2. 実装済み skeleton の地図

### Shell Core

関連 file:

~~~text
packages/shell_core/error_taxonomy.py
packages/shell_core/policy_evaluator.py
packages/shell_core/runtime_state.py
packages/shell_core/state_snapshot.py
packages/shell_core/permission_ledger.py
packages/shell_core/approval_queue.py
packages/shell_core/audit_store.py
packages/shell_core/recovery_catalog.py
~~~

Shell Core skeleton は framework-independent かつ BLUE-TANUKI-internal-free の状態を維持しなければならない。

### Rust helper

関連 file:

~~~text
native/rust_helper/src/lib.rs
native/rust_helper/src/process.rs
native/rust_helper/src/filesystem.rs
native/rust_helper/src/network.rs
native/rust_helper/src/diagnostics.rs
native/rust_helper/src/update_verification.rs
native/rust_helper/src/audit_hash.rs
native/rust_helper/src/ipc.rs
~~~

Rust helper は限定的な helper surface にすぎない。authority path になってはならない。

### BLUE-TANUKI 用 Adapter

関連 file:

~~~text
packages/blue_tanuki_adapter/adapter.py
packages/blue_tanuki_adapter/health.py
packages/blue_tanuki_adapter/runtime_snapshot.py
packages/blue_tanuki_adapter/authority_trace.py
packages/blue_tanuki_adapter/notifications.py
packages/blue_tanuki_adapter/approvals.py
packages/blue_tanuki_adapter/audit_export.py
packages/blue_tanuki_adapter/diagnostics.py
packages/blue_tanuki_adapter/recovery.py
~~~

現在の adapter は mock-contract に基づく。live Runtime integration は complete ではない。

### desktop Flutter

関連 file:

~~~text
apps/desktop_flutter/lib/main.dart
apps/desktop_flutter/lib/screens/
apps/desktop_flutter/lib/services/shell_core_client.dart
apps/desktop_flutter/lib/models/generated_contracts.dart
~~~

Flutter は operator surface にすぎない。authority、permission semantics、approval semantics、audit semantics、recovery semantics を定義してはならない。

### 導入部（Installer／Setup Doctor）

関連 file:

~~~text
installer/setup_doctor.py
docs/FIRST_RUN.md
docs/SETUP_DOCTOR.md
docs/INSTALLER_BOUNDARY.md
~~~

Installer state は authority を付与したり、permission を暗黙に approve したりしてはならない。

### モバイル companion

関連 file:

~~~text
apps/mobile_flutter/lib/main.dart
apps/mobile_flutter/lib/screens/
~~~

mobile は observe、review、notify、emergency stop request、recovery instruction の表示を行ってよい。独立した authority になってはならない。

### release の強化

関連 file:

~~~text
RELEASE_CHECKLIST.md
SECURITY_REVIEW.md
COMPATIBILITY_MATRIX.md
CONFORMANCE_REPORT.md
AUDIT_EVIDENCE.md
INSTALLER_STATUS.md
MOBILE_STATUS.md
VALIDATION.txt
~~~

release claim の promotion には owner GO と直接 evidence が必要である。

## 3. 直近の次作業

skeleton から production-grade behavior へ、次の順序で進める。

~~~text
1. Rust helper を compile 可能にし、cargo test を通過させる。
2. desktop／mobile の Flutter analyze を実行する。
3. mock の Shell Core client を real local Shell Core boundary に置き換える。
4. durable audit storage と hash-chain verification を追加する。
5. 汎用 contract のみを介した live BLUE-TANUKI adapter integration を追加する。
6. Setup Doctor behavior の安定後に installer packaging を追加する。
7. audit、revocation、recovery path を伴う real mobile pairing を追加する。
8. evidence が存在した後に release claim review を再実行する。
~~~

toolchain validation を省略しない。tool を利用できない場合は、正確な理由とともに <code>not run</code> を報告する。

## 4. production hardening の要件

### 4.1 方針評価（policy evaluation）

<code>PolicyEvaluator</code> は、次を拒否しなければならない。

- 未知の Runtime
- 未知の Capability
- 未知の Permission
- deny された Permission
- Approval の欠落
- 無効な Approval state
- audit event の欠落
- payload が存在する場合の audit payload hash の欠落
- recovery action の欠落
- adapter metadata による authority claim
- non-authority source による試行

出力形状:

~~~python
{
  "allowed": bool,
  "errors": [...],
  "required_recovery": dict | None,
  "audit_required": bool
}
~~~

### 4.2 状態 snapshot

state snapshot は deterministic で、次を含まなければならない。

- 実行対象（Runtime）
- adapter
- Permission
- 承認待ち（pending Approval）
- audit の summary
- recovery catalog の summary
- update policy の summary
- invariant の flag

invariant flag は次を含まなければならない。

~~~text
flutter_imported_by_shell_core=false
blue_tanuki_imported_by_shell_core=false
adapter_metadata_can_escalate_authority=false
memory_cache_previous_state_can_grant_authority=false
full_payload_projected_without_full_visibility=false
~~~

### 4.3 Rust helper

許可する責任:

- process の診断
- filesystem の診断
- port／network の診断
- audit hash 用 utility
- update signature の検証
- 安全な IPC message framing
- recovery helper の stub

禁止事項:

- authority path になること
- 任意 command の実行
- 既定で任意 file content を読むこと
- 任意 file への書込み
- 既定で任意の external fetch を行うこと
- 明示 contract なしに credential へ access すること
- audit を迂回すること
- recovery mapping を迂回すること

helper response の形状:

~~~json
{
  "ok": true,
  "operation": "string",
  "result": {},
  "diagnostics": [],
  "error": null
}
~~~

failure の形状:

~~~json
{
  "ok": false,
  "operation": "string",
  "result": null,
  "diagnostics": [],
  "error": {
    "code": "string",
    "message": "string",
    "recoverable": true
  }
}
~~~

### 4.4 BLUE-TANUKI 用 Adapter

adapter の surface:

- health の確認
- ready 状態
- Runtime の snapshot
- authority の trace
- notification
- Approval
- audit の event
- diagnostics の export
- recovery の action

規則:

- Runtime 固有 mapping は <code>packages/blue_tanuki_adapter/</code> 内に留める。
- Adapter metadata は untrusted のままとする。
- Adapter は Permission を付与できない。
- Adapter は action を approve できない。
- Adapter は content exposure policy を迂回できない。
- Adapter は audit を迂回できない。

BLUE-TANUKI Runtime を利用できない場合は mock fixture を使用する。live Runtime を理由に contract test を block しない。

### 4.5 デスクトップ Flutter

必須画面:

- 概況画面（Dashboard）
- 診断画面（Setup Doctor）
- Runtime 管理画面（Runtime Center）
- Permission 管理画面（Permission Center）
- Approval 管理画面（Approval Center）
- Audit 閲覧画面（Audit Viewer）
- Recovery 管理画面（Recovery Center）
- 設定画面（Settings）

UI の禁止規則:

- Flutter は authority を定義してはならない。
- Flutter は permission semantics を定義してはならない。
- Flutter は Shell Core なしに approve してはならない。
- Shell Core projection が許可しない限り、Flutter は <code>full_payload</code> を表示してはならない。
- Flutter は adapter conformance を迂回してはならない。
- Flutter は audit creation を迂回してはならない。
- Flutter は保護された approval field を変更してはならない。

### 4.6 Installer／Setup Doctor の診断

Setup Doctor は次を検査しなければならない。

- Python の availability
- 必要な場合の Rust availability
- 必要な場合の Flutter availability
- Runtime の connection
- local Permission
- update の policy
- audit の storage
- recovery の catalog
- adapter の readiness

規則:

- Installer state は authority を付与してはならない。
- Installer は permission を暗黙に approve してはならない。
- Installer は failure を隠してはならない。
- failure message は operator-readable でなければならない。

### 4.7 モバイル companion

mobile で許可する事項:

- Runtime status の表示
- notification の受信
- Approval の review
- emergency stop の request
- recovery instruction の表示

mobile で禁止する事項:

- Shell Core の迂回
- Approval visibility の迂回
- protected field rule の迂回
- 独立した authority になること
- device を暗黙に pair すること
- hidden payload を approve すること

device pairing は次を含まなければならない。

- <code>device_id</code>
- <code>pairing_id</code>
- operator の確認
- audit の event
- revocation の path
- recovery の path

## 5. validation の command

常に次を実行する。

~~~bash
python3 tooling/schema_check/check_schemas.py
python3 tooling/conformance_tests/run_conformance_skeleton.py
~~~

集約 reporter:

~~~bash
python3 tooling/validate_all.py
~~~

Rust を利用できる場合:

~~~bash
cd native/rust_helper && cargo test
~~~

Flutter を利用できる場合:

~~~bash
cd apps/desktop_flutter && flutter analyze
cd apps/mobile_flutter && flutter analyze
~~~

現在知られている environment result:

~~~text
schema check passed: 19 schemas, 19 examples, 19 negative fixtures
conformance skeleton passed: 67 checks
cargo test: not run, cargo not found on PATH
desktop flutter analyze: not run, flutter not found on PATH
mobile flutter analyze: not run, flutter not found on PATH
~~~

## 6. release claim の規則

evidence が存在しない限り、次を主張しない。

- production の readiness
- installer の readiness
- mobile の readiness
- 安定した Runtime support
- security の completeness

release claim を promotion する前には owner GO が必要である。

## 7. 必須の最終報告形式

完了した各 work block では、次を報告しなければならない。

1. 概要
2. 変更 file
3. risk 分類
4. validation 結果
5. 残存 risk
6. commit hash、または <code>not committed</code>

validation では、次を明示しなければならない。

- passed
- failed
- 未実行（not run）
- 正確な command
- not run の場合は正確な理由
