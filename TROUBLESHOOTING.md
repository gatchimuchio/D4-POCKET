# GUI Shell トラブルシューティング

## `python: command not found`

一部の host は Python を `python3` として公開する。

次を使う。

```bash
python3 tooling/schema_check/check_schemas.py
python3 tooling/conformance_tests/run_conformance_skeleton.py
```

## Schema 検証が失敗する

次を確認する。

- `specs/*.schema.json` の JSON 構文
- 必須の draft 宣言
- 重複または不正な `$id`
- core contract に誤って入った Flutter 固有または BLUE-TANUKI 固有 field

その後、次を再実行する。

```bash
python tooling/schema_check/check_schemas.py
```

## Conformance 骨格が失敗する

conformance が引き続き次を検査することを確認する。

- adapter 権限除去
- content exposure 境界
- approval 編集制限
- 機密 action の audit/recovery 対応
- policy evaluator の失敗事例
- Rust helper 境界構造
- BLUE-TANUKI adapter contract 対応
- desktop/mobile 権限境界
- installer の非権限状態
- release 主張境界

その後、次を再実行する。

```bash
python tooling/conformance_tests/run_conformance_skeleton.py
```

## Rust がインストールされていない

条件付き Rust command を実行せず、未実行と報告する。

```bash
cd native/rust_helper && cargo test
```

## Flutter がインストールされていない

条件付き Flutter command を実行せず、未実行と報告する。

```bash
cd apps/desktop_flutter && flutter analyze
cd apps/mobile_flutter && flutter analyze
```

## 利用可能なすべての検証を実行する

集約報告器を使い、成功、失敗、未実行の結果を収集する。

```bash
python3 tooling/validate_all.py
```

## 製品 UI 作業が停滞しているように見える

schema または conformance test が未完了な場合は想定される状態である。GUI Shell は schema と conformance を先行させるため、次の task は通常、UI 画面ではなく最小の欠落 schema または conformance check とする。
