# GUI Shell 設定リファレンス

この文書は、Phase 0 / Phase 1 の設定リファレンスである。設定だけで権限を与えることなく、意図された設定面を説明する。

## C31現行設定境界（2026-09-24）

現行の設定面は、Runtime／Adapter、Operation Profile、Credential metadata、MCP／A2A metadata、Host、Update、通知、Mobile projectionの要求を、Schema検証済みのBroker経路へ渡すための宣言・表示面である。設定、Profile、metadata、履歴、UI stateはPermission、Approval、Authority、Credential実値を生成しない。

秘密値はFlutter、snapshot、Audit、error、log、trace、test artifactへ出さない。外部download、install、process、Tool実行、任意Agent task、実停止、実端末連携を設定の存在や要求receiptだけで完了扱いにしない。取得不能な資源値は0へ置換せず`unknown`として扱う。

C30の検証範囲はlocal Broker／Rust／Flutter／fixtureであり、Windows installed product、外部Runtime／Agent、実端末の設定実証ではない。未取得のinstalled-path証拠、正式署名、owner GO、正式releaseは`release_blocker`である。

## 検証コマンド

```bash
python tooling/schema_check/check_schemas.py
python tooling/conformance_tests/run_conformance_skeleton.py
python tooling/日本語基底監査.py --strict
python tooling/manifest.py --check
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
