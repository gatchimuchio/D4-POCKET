# GUI Shell 設定リファレンス

この文書は、Phase 0 / Phase 1 の設定リファレンスである。設定だけで権限を与えることなく、意図された設定面を説明する。

## 検証コマンド

```bash
python tooling/schema_check/check_schemas.py
python tooling/conformance_tests/run_conformance_skeleton.py
```

必要な場合:

```bash
python3 tooling/schema_check/check_schemas.py
python3 tooling/conformance_tests/run_conformance_skeleton.py
```

## Runtime Adapter の設定

Adapter は、schema と conformance 規則によって宣言する。Adapter の `metadata` は信頼しない。

必須の Adapter プロパティは、次で定義する。

```text
specs/adapter.schema.json
docs/specs/adapter-conformance.md
```

Adapter の設定は、次を行ってはならない。

- `metadata` によって Permission を付与する。
- Runtime が許可していない `authority_context` を作成する。
- Content Exposure Policy を迂回する。
- Approval 状態を迂回する。
- network、filesystem、process、credential、IPC へのアクセスを暗黙に追加する。

## Content Exposure の設定

`content_visibility` の値は次のとおりである。

```text
none
hash_only
summary
redacted
full
```

全文表示を許可するのは `full` のみである。それ以外のモードでは、許可された projection だけを表示しなければならない。

## Native Helper の設定

Rust helper の境界は、範囲を限定した次の native diagnostics / operations のために確保する。

- process の検査
- filesystem の診断
- network の診断
- update の検証
- audit の hash 化
- 安全な IPC

Native Helper の設定は、Capability、Permission、Approval、Audit、Recovery の各 contract に常に従属しなければならない。
