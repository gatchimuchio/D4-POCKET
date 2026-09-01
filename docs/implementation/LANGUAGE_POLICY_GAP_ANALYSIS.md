# 言語方針の差分分析

状態: Flutter broker 製品経路の開始後に更新した Phase 0 差分分析
日付: 2026-06-03
範囲: GUI-Shell の言語方針収束

## 1. 判定基準

確定済み方針:

- UI 製品 layer: Flutter / Dart
- 権限依存の安全境界: Rust
- 契約: JSON Schema / protocol contract
- Python: dev-only tooling / local validation / migration oracle / temporary validation に限定
- TypeScript / Node: external SDK / adapter sample / protocol client sample に限定
- 権限依存の Flutter-Rust 接続: independent process + restricted IPC

## 2. 差分の要約

| 項目 | 現在の事実 | 方針上の状態 | 分類 |
| --- | --- | --- | --- |
| Shell Core の実装言語 | `packages/shell_core/*.py` が registry、policy、approval、audit、recovery、content exposure、snapshot semantics を実装している | product authority path として使う場合は言語方針に抵触する | release_blocker |
| owner-use state の生成 | `scripts/launch_owner_desktop.*` は Rust broker を起動し `GUI_SHELL_BROKER_ENDPOINT_JSON` を Flutter に渡す。`tooling/shell_snapshot.py` は local diagnostic / parity tooling として残る。`tooling/release_runtime_assertions.py --check` は product Flutter / owner launch path に Python snapshot generator invocation がないことを検証する。 | installed no-Python-runtime proof は未取得だが product launch path は broker-mediated に移行済み | release_blocker |
| Rust の security 境界 | `native/rust_helper` に broker process、authenticated loopback IPC、durable store、authority parity operations がある | command dispatch / active cutover / installed proof が未完 | release_blocker |
| Flutter の authority ownership | Flutter は JSON を読み projection を表示するだけで、policy mutation / dispatch は見つからない | current scope では整合 | none |
| FFI の authority path | `tooling/release_runtime_assertions.py --check` が `dart:ffi`、`flutter_rust_bridge`、`MethodChannel`、Rust FFI export token を authority surface scan で拒否する | current scope では整合 | none |
| TypeScript / Node の core runtime | 見つからない | 整合 | none |
| BLUE-TANUKI boundary | `packages/blue_tanuki_adapter` の mock/reference adapter に留まり、Shell Core は BLUE-TANUKI internals を import していない | reference adapter scaffold として整合 | none |
| Windows installed-path の evidence | `release_evidence/windows_installed_smoke.json` が存在しない | 既存 release blocker 継続 | release_blocker |

## 3. 言語方針との不整合

- item: Python Shell Core が現在の権限依存実装である
  classification: release_blocker（分類）
  reason: `PolicyEvaluator`、`ApprovalQueue`、`AuditStore` / `JsonPersistence`、`RecoveryCatalog`、`ContentExposure`、`StateSnapshot` が Python で実装され、現行 Shell Core behavior として conformance と owner-use snapshot generation に使われている。
  required_action: 等価な Rust broker の責任を parity test 付きで実装し、Python を active な製品呼出し経路から外して migration oracle / tooling に限定する。
  blocks_release: yes（リリース阻止）

- item: Rust Security Broker の製品経路が未完了である
  classification: release_blocker（分類）
  reason: `native/rust_helper` に broker process、authenticated loopback IPC、durable store、authority parity operations があり、Flutter product client は endpoint file 経由で broker IPC を使う。ただし command dispatch は suspended、`authority_cutover_status=not_active`、Windows installed-path proof がない。
  required_action: active authority cutover 前に command-envelope execution gates、no-Python/no-FFI assertions、broker unavailable / crash / stale-session fail-closed behavior、Windows installed proof を検証する。
  blocks_release: yes（リリース阻止）

- item: Flutter broker 統合の実行可能性証拠が未完了である
  classification: release_blocker（分類）
  reason: `main.dart` は `ShellCoreClient.product()` を使い、broker unavailable / auth / stale / malformed response は SUSPEND snapshot になる。Windows Flutter analyze/test は `flutter.bat` 経由で通過した。ただし Windows installed path の broker launch/connect/fail-closed evidence は未取得である。
  required_action: Windows installed path で broker client launch/connect/fail-closed evidence を取得する。
  blocks_release: yes（リリース阻止）

- item: Python Setup Doctor のインストール済み経路依存が未解決である
  classification: release_blocker（分類）
  reason: 現行 `installer/setup_doctor.py` は Python diagnostics script であり、Windows installed-path の Setup Doctor evidence がない。
  required_action: installed product diagnostics の authority evidence を Rust broker / product path へ移すか、Python diagnostics が release-only tooling で installed GUI-Shell runtime に必須ではないことを証明する。
  blocks_release: yes（リリース阻止）

- item: Python runtime 非依存の製品表明が未完了である
  classification: release_blocker（分類）
  reason: `tooling/release_runtime_assertions.py --check` は product Flutter / owner launch path が Python process startup や Python snapshot generator invocation を使わないことを検証し、`tooling/validate_all.py` と `tooling/evidence_bundle.py --check` に接続済みである。ただし installed Windows artifact 上で Python 未導入状態の authority path 起動証拠はまだない。
  required_action: installed/runtime artifact に Python interpreter または Python production dependency が不要であることを Windows installed-path smoke で検証する。
  blocks_release: yes（リリース阻止）

- item: 権限経路で FFI を使わないことのリリース表明
  classification: required_for_v1（分類）
  reason: `tooling/release_runtime_assertions.py --check` が Flutter authority operations、Rust broker boundary、future bridge code を対象に no-FFI-authority assertion を実行し、restricted IPC path のみを許可する。
  required_action: 新しい authority-sensitive Flutter/Rust 接続を追加する場合は assertion 対象を拡張し、FFI/direct bridge を authority、approval-token、audit finalization、external command dispatch、credential、recovery authorization path に入れない。
  blocks_release: no（リリース阻止）

## 4. 観測された非競合事項

- item: TypeScript / Node の core runtime
  classification: none（分類）
  reason: GUI-Shell core runtime の TypeScript / Node path は見つからなかった。
  required_action: TypeScript / Node は future SDK/sample scope に限定し続ける。
  blocks_release: no（リリース阻止）

- item: BLUE-TANUKI core の変更
  classification: none（分類）
  reason: 現行 repository には GUI-Shell 側 adapter mock/reference package があり、BLUE-TANUKI core modification path は確認されなかった。
  required_action: BLUE-TANUKI runtime-specific mapping は adapter boundary の内側に留める。
  blocks_release: no（リリース阻止）

- item: FFI 権限 bridge
  classification: none（分類）
  reason: current Flutter-Rust FFI authority bridge は見つからなかった。
  required_action: authority、approval-token、audit finalization、external command dispatch、credential、recovery authorization path では FFI 禁止を維持する。
  blocks_release: no（リリース阻止）

## 5. 必要なリリース関門の変更

既存の Windows installed-path evidence、Setup Doctor evidence、strict Windows validation、owner GO の release blocker は有効なまま残る。ただし、それらだけでは製品完成の release claim 条件として不足する。

製品完成のリリース表明前に必要な条件:

1. active authority runtime が Rust broker である、または product path に authority-sensitive active runtime が存在しないことを証明する。
2. Python Shell Core が dev/test/migration oracle のみに分類・強制されていることを証明する。
3. Flutter authority operations が broker-mediated であり、broker unavailable 時に fail closed することを証明する。
4. no-Python-runtime static assertion、no-FFI-authority static assertion、malformed IPC rejection、replay rejection、audit-on-rejection checks を維持して通す。
5. Windows installed-path evidence は broker path が active になった後に取り直す。

## 6. 証拠の限界

- schema / conformance success は現時点で Python contract behavior を証明するが、Rust broker behavior は証明しない。
- Rust helper cargo test は broker skeleton の JSON envelope / rejection behavior を証明するが、production IPC transport、Flutter integration、インストール済み製品の authority ownership は証明しない。
- Flutter tests は display projection behavior を証明するが、authority ownership は証明しない。
- Linux/WSLg smoke は development evidence であり、Windows product proof に昇格できない。
- installed runtime artifact が Python を不要とする Windows evidence はまだない。

これらの限界により、次の項目は未解消である。

- item: 言語方針に適合する runtime 証拠の欠如
  classification: release_blocker（分類）
  reason: release runtime assertions と local broker IPC/parity evidence はあるが、installed Windows product path の no-Python-runtime / broker-mediated LIVE_RUNTIME evidence がまだない。
  required_action: Windows installed path で Rust broker active path と no-Python-runtime evidence を追加する。
  blocks_release: yes（リリース阻止）
