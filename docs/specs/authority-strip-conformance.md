# Authority Strip の適合規則（Authority Strip Conformance）

Authority Strip Conformance は、inbound Runtime、Adapter、UI、memory、cache、または `metadata` の値が GUI Shell 内部に権限を作ることを防ぐ。

## 必須（MUST）

- Shell Core が request を評価する前に、inbound authority key を除去する。
- Adapter の `metadata` を、信頼できない説明用データとして扱う。
- Permission、Approval、grant、actor、または `authority_context` を外部から設定しようとする試みを拒否または無視する。
- Runtime が宣言した境界を拡大せずに維持する。
- 安全に実施できる場合は、除去した機密性の高い authority material の Audit evidence を記録する。

## 禁止（MUST NOT）

- GUI input に、Runtime が許可していない `authority_context` を作らせる。
- Adapter の `metadata` に Permission を付与させる。
- memory、local cache、complete history、previous state、または記憶された UI state に、それ自体で権限を与えさせる。
- 過去の表示、選択、または local UI state から Approval を推論する。
- 明示的な schema と conformance coverage なしに、Runtime 固有の authority concept を Shell Core の権限へ変換する。

## Authority-like な inbound key

Conformance baseline では、次の inbound key を authority-like として扱い、検証済みの Shell Core Contract が明示的に所有していない限り除去する。

```text
authority
authority_context
authority_trace
approval_state
approved_by
permission_grant
permission_override
role
scope_escalation
trust_level
```

入れ子の authority-like key も除去しなければならない。

## 合格条件

除去後の request は、payload、Runtime identity、operation identity、安全な `metadata`、および hash を保持してよい。Permission、Approval、privilege、または trust を付与できる inbound authority-like key を保持してはならない。
