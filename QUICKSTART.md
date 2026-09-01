# GUI Shell クイックスタート

GUI Shell は現時点で release 強化用の骨格であり、製品 runtime ではない。クイックスタート経路は、製品主張より前に contract と conformance 骨格を検証する。

## 前提条件

- POSIX 類似 shell
- `python` または `python3` として利用可能な Python
- 任意: `native/rust_helper` 用の Rust
- 任意: `apps/desktop_flutter` 用の Flutter
- v1後の任意項目: `apps/mobile_flutter` 用の Flutter

## 契約検証

推奨 command:

```bash
python tooling/schema_check/check_schemas.py
python tooling/conformance_tests/run_conformance_skeleton.py
```

`python` が `PATH` にない場合の fallback:

```bash
python3 tooling/schema_check/check_schemas.py
python3 tooling/conformance_tests/run_conformance_skeleton.py
```

成功時の想定出力:

```text
schema checkが合格: schema 26件、example 26件、negative fixture 28件
conformance skeletonが合格: 141 件のcheck
```

## 段階Bの所有者起動

局所の所有者操作 snapshot を生成し、厳格 release 検証を実行せずに desktop shell を起動する。

```bash
bash scripts/launch_owner_desktop.sh
```

native Windows では次を実行する。

```powershell
powershell -ExecutionPolicy Bypass -File scripts\launch_owner_desktop.ps1
```

この経路は段階Bの所有者局所操作用である。`release_evidence/windows_installed_smoke.json` を作成せず、release readiness を主張せず、実測済み Windows インストール先 release 証拠を満たさない。

## 任意の Rust helper 検査

```bash
cd native/rust_helper
cargo test
```

Rust helper には、broker envelope 検証と拒否 audit 用の現行 Rust Security Broker 骨格がある。権限移行と IPC 統合の証拠が成立するまで、実際の外部 command dispatch は無効である。

## 任意の desktop Flutter 検査

```bash
cd apps/desktop_flutter
flutter analyze
```

Flutter は交換可能な UI 層である。UI widget は操作者入力の収集と状態の描画をできるが、権限、permission、approval、audit、recovery の意味を定義してはならない。

Mobile Flutter は `post_v1_scope` である。所有者が明示的に範囲を変更しない限り、v1.0 release gate と CI 製品主張から除外する。

## 次の実装順序

1. `docs/standards/gui-shell-extended-standard.md` を権限ある仕様として保つ。
2. `specs/` 下の JSON Schema を拡張する。
3. conformance test を追加または更新する。
4. contract を生成または更新する。
5. 主張文書を実際の検証証拠と整合させる。
