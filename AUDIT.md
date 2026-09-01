# GUI Shell 監査

GUI Shell の監査要件は schema-first かつ conformance-first である。

## 必須の Audit mapping

すべての sensitive action は、次へ対応付けなければならない。

- 能力宣言（Capability）
- 許可（Permission）
- 承認状態（Approval state）
- 監査事象（`AuditEvent`）
- 失敗時の `RecoveryAction`

## Core invariant の検査

このリポジトリは、次の invariant を維持しなければならない。

- inbound authority key を除去する。
- external metadata は権限を昇格できない。
- Runtime が許可していない `authority_context` を GUI input から作成できない。
- GUI input は権限ではない。
- memory、cache、previous state は、それ自体では権限ではない。
- `content_visibility` を遵守する。
- `full_payload` を保存していても、`content_visibility=full` でない限り UI への露出を意味しない。
- Approval の編集範囲は field 単位に限定する。
- 編集後の payload を再 hash 化し、再検証する。
- すべての sensitive action について AuditEvent を作成する。

## Contract の所在

Audit に関係する contract は、次に置く。

```text
specs/audit.schema.json
specs/approval.schema.json
specs/capability.schema.json
specs/permission.schema.json
specs/recovery.schema.json
docs/specs/adapter-conformance.md
docs/specs/approval-visibility-boundary.md
docs/specs/content-exposure-policy.md
```

## 検証

```bash
python tooling/schema_check/check_schemas.py
python tooling/conformance_tests/run_conformance_skeleton.py
```

conformance skeleton は、完成した Audit 実装ではない。これは、product UI が Authority、Visibility、Approval、Recovery の contract を追い越すことを防ぐ最初の gate である。
