# 契約例

これらのファイルは `specs/*.schema.json` に対する有効な例である。

`tooling/schema_check/check_schemas.py` は、repository 内蔵の schema subset 検査器を使い、すべての `*.valid.json` を対応する schema に対して検証する。

`invalid/` directory には、schema 検証に失敗しなければならない negative fixture を置く。各 schema に少なくとも1つ必要である。

これらの例は runtime state ではなく、権限を与えない。製品 UI の成立前に段階1の契約を実行可能に保つために置く。

対応:

```text
runtime.valid.json -> specs/runtime.schema.json
adapter.valid.json -> specs/adapter.schema.json
capability.valid.json -> specs/capability.schema.json
permission.valid.json -> specs/permission.schema.json
approval.valid.json -> specs/approval.schema.json
audit.valid.json -> specs/audit.schema.json
recovery.valid.json -> specs/recovery.schema.json
diagnostic.valid.json -> specs/diagnostic.schema.json
update.valid.json -> specs/update.schema.json
content_exposure.valid.json -> specs/content_exposure.schema.json
framework_risk_profile.valid.json -> specs/framework_risk_profile.schema.json
```

Negative fixture の名称は schema の basename を接頭辞に使う。

```text
runtime_*.invalid.json -> specs/runtime.schema.json
adapter_*.invalid.json -> specs/adapter.schema.json
capability_*.invalid.json -> specs/capability.schema.json
permission_*.invalid.json -> specs/permission.schema.json
approval_*.invalid.json -> specs/approval.schema.json
audit_*.invalid.json -> specs/audit.schema.json
recovery_*.invalid.json -> specs/recovery.schema.json
diagnostic_*.invalid.json -> specs/diagnostic.schema.json
update_*.invalid.json -> specs/update.schema.json
content_exposure_*.invalid.json -> specs/content_exposure.schema.json
framework_risk_profile_*.invalid.json -> specs/framework_risk_profile.schema.json
```
