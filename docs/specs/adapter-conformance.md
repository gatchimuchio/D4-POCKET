# Adapter の適合規則（Adapter Conformance）

## 目的

Adapter は、境界を越えて権限を漏出させることなく、GUI Shell を外部 Runtime へ接続する。

## 必須（MUST）

- inbound authority key を除去する。
- external authority escalation を拒否する。
- すべての Adapter message を `adapter.schema.json` で検証する。
- Runtime が宣言した Permission の境界を維持する。
- Content Exposure Policy を維持する。
- 機密性の高い Adapter action について AuditEvent を発行する。
- sealed field を露出せずに diagnostic information を返す。
- inbound Adapter payload に入れ子で含まれる authority-like key も除去する。
- grant-like key を含む場合でも、`metadata` は説明用の信頼できないデータとして扱う。

## 禁止（MUST NOT）

- Runtime の `metadata` を信頼できる権限として扱う。
- Runtime が付与していない `authority_context` を作成する。
- 許可された visibility を越えて未加工の payload を表示する。
- sealed、hidden、sacred、または authority field を編集する。
- `metadata`、memory、cache、または previous state を Permission に変換する。
