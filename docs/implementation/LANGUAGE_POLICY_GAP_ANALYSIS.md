# Language Policy Gap Analysis

Status: Phase 0 gap analysis updated after Flutter broker product path start
Date: 2026-06-03
Scope: GUI-Shell language policy convergence

## 1. 判定基準

確定済み方針:

- UI product layer: Flutter / Dart
- authority-sensitive safety boundary: Rust
- contracts: JSON Schema / protocol contracts
- Python: dev-only tooling / CI support / migration oracle / temporary validation only
- TypeScript / Node: external SDK / adapter sample / protocol client sample only
- authority-sensitive Flutter-Rust connection: independent process + restricted IPC

## 2. Gap Summary

| item | current fact | policy status | classification |
| --- | --- | --- | --- |
| Shell Core implementation language | `packages/shell_core/*.py` が registry、policy、approval、audit、recovery、content exposure、snapshot semantics を実装している | product authority path として使う場合は言語方針に抵触する | release_blocker |
| owner-use state generation | `scripts/launch_owner_desktop.*` は Rust broker を起動し `GUI_SHELL_BROKER_ENDPOINT_JSON` を Flutter に渡す。`tooling/shell_snapshot.py` は local diagnostic / parity tooling として残る。 | installed no-Python-runtime proof は未取得だが product launch path は broker-mediated に移行済み | release_blocker |
| Rust security boundary | `native/rust_helper` に broker process、authenticated loopback IPC、durable store、authority parity operations がある | command dispatch / active cutover / installed proof が未完 | release_blocker |
| Flutter authority ownership | Flutter は JSON を読み projection を表示するだけで、policy mutation / dispatch は見つからない | current scope では整合 | none |
| FFI authority path | `dart:ffi`、`flutter_rust_bridge`、`MethodChannel`、Rust FFI authority path は見つからない | current scope では整合 | none |
| TypeScript / Node core runtime | 見つからない | 整合 | none |
| BLUE-TANUKI boundary | `packages/blue_tanuki_adapter` の mock/reference adapter に留まり、Shell Core は BLUE-TANUKI internals を import していない | reference adapter scaffold として整合 | none |
| Windows installed-path evidence | `release_evidence/windows_installed_smoke.json` が存在しない | 既存 release blocker 継続 | release_blocker |

## 3. Language Policy Conflicts

- item: Python Shell Core is current authority-sensitive implementation
  classification: release_blocker
  reason: `PolicyEvaluator`、`ApprovalQueue`、`AuditStore` / `JsonPersistence`、`RecoveryCatalog`、`ContentExposure`、`StateSnapshot` が Python で実装され、現行 Shell Core behavior として conformance と owner-use snapshot generation に使われている。
  required_action: 等価な Rust broker responsibilities を parity tests 付きで実装し、Python を active product invocation path から外して migration oracle / tooling に限定する。
  blocks_release: yes

- item: Rust Security Broker production path is incomplete
  classification: release_blocker
  reason: `native/rust_helper` に broker process、authenticated loopback IPC、durable store、authority parity operations があり、Flutter product client は endpoint file 経由で broker IPC を使う。ただし command dispatch は suspended、`authority_cutover_status=not_active`、Windows installed-path proof がない。
  required_action: active authority cutover 前に command-envelope execution gates、no-Python/no-FFI assertions、broker unavailable / crash / stale-session fail-closed behavior、Windows installed proof を検証する。
  blocks_release: yes

- item: Flutter broker integration runnable proof is incomplete
  classification: release_blocker
  reason: `main.dart` は `ShellCoreClient.product()` を使い、broker unavailable / auth / stale / malformed response は SUSPEND snapshot になる。Windows Flutter analyze/test は `flutter.bat` 経由で通過した。ただし Windows installed path の broker launch/connect/fail-closed evidence は未取得である。
  required_action: Windows installed path で broker client launch/connect/fail-closed evidence を取得する。
  blocks_release: yes

- item: Python Setup Doctor installed-path dependency is unresolved
  classification: release_blocker
  reason: current `installer/setup_doctor.py` は Python diagnostics script であり、Windows installed-path Setup Doctor evidence がない。
  required_action: installed product diagnostics の authority evidence を Rust broker / product path へ移すか、Python diagnostics が release-only tooling で installed GUI-Shell runtime に必須ではないことを証明する。
  blocks_release: yes

- item: no-Python-runtime product assertion is incomplete
  classification: release_blocker
  reason: Python が tooling / CI / migration oracle に限定されるという product artifact assertion が strict release gate でまだ証明されていない。
  required_action: installed/runtime artifact に Python interpreter または Python production dependency が不要であることを検証する no-Python-runtime check を strict release validation に追加する。
  blocks_release: yes

- item: no-FFI-authority release assertion is incomplete
  classification: release_blocker
  reason: current static scan では `dart:ffi` / `flutter_rust_bridge` authority path は見つからないが、authority path が restricted IPC だけであることを release gate として固定する検証がまだ十分ではない。
  required_action: Flutter authority operations、Rust broker boundary、future bridge code を対象に no-FFI-authority assertion を strict release validation へ追加する。
  blocks_release: yes

## 4. Non-conflicts Observed

- item: TypeScript / Node core runtime
  classification: none
  reason: GUI-Shell core runtime の TypeScript / Node path は見つからなかった。
  required_action: TypeScript / Node は future SDK/sample scope に限定し続ける。
  blocks_release: no

- item: BLUE-TANUKI core modification
  classification: none
  reason: 現行 repository には GUI-Shell 側 adapter mock/reference package があり、BLUE-TANUKI core modification path は確認されなかった。
  required_action: BLUE-TANUKI runtime-specific mapping は adapter boundary の内側に留める。
  blocks_release: no

- item: FFI authority bridge
  classification: none
  reason: current Flutter-Rust FFI authority bridge は見つからなかった。
  required_action: authority、approval-token、audit finalization、external command dispatch、credential、recovery authorization path では FFI 禁止を維持する。
  blocks_release: no

## 5. Required Release-Gate Change

既存の Windows installed-path evidence、Setup Doctor evidence、strict Windows validation、owner GO の release blockers は有効なまま残る。ただし、それらだけでは completed product release claim の条件として不足する。

Completed product release claim 前に必要な条件:

1. active authority runtime が Rust broker である、または product path に authority-sensitive active runtime が存在しないことを証明する。
2. Python Shell Core が dev/test/migration oracle のみに分類・強制されていることを証明する。
3. Flutter authority operations が broker-mediated であり、broker unavailable 時に fail closed することを証明する。
4. no-Python-runtime、no-FFI-authority、malformed IPC rejection、replay rejection、audit-on-rejection checks を追加して通す。
5. Windows installed-path evidence は broker path が active になった後に取り直す。

## 6. Evidence Limitations

- schema / conformance success は現時点で Python contract behavior を証明するが、Rust broker behavior は証明しない。
- Rust helper cargo tests は broker skeleton の JSON envelope / rejection behavior を証明するが、production IPC transport、Flutter integration、installed product authority ownership は証明しない。
- Flutter tests は display projection behavior を証明するが、authority ownership は証明しない。
- Linux/WSLg smoke は development evidence であり、Windows product proof に昇格できない。
- installed runtime artifact が Python を不要とする証拠はまだない。

These limitations:

- item: language policy runtime proof absent
  classification: release_blocker
  reason: authority、runtime language policy、IPC、audit、recovery boundary に影響する証拠不足である。
  required_action: Rust broker active path と no-Python-runtime evidence を追加する。
  blocks_release: yes
