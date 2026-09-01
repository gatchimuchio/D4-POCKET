# shell_core 責任境界

framework に依存しない Shell Core。

所有する責任:

- runtime registry の意味
- permission ledger の意味
- approval queue の意味
- audit event の作成
- recovery の分類
- content exposure の検証
- framework risk profile の取扱い

Flutter に依存してはならない。

段階3の骨格 module:

- `runtime_registry`
- `adapter_loader`
- `permission_ledger`
- `approval_queue`
- `audit_store`
- `recovery_catalog`
- `update_policy_store`
- `content_exposure`

実装は意図的に必要最小限かつ決定的とする。Flutter または BLUE-TANUKI 内部は import しない。
