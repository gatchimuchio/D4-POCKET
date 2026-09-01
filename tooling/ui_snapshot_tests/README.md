# UI スナップショット試験

v1後に計画する Flutter UI snapshot test 境界。

classification: post_v1_scope
reason: 現行 v1.0 gate は Flutter analyze/test と conformance に裏付けられた操作表層検査を使い、完全な snapshot test は延期する。
blocks_release: no

目的:

- 意図しない UI regression を防ぐ
- 移行費用を可視に保つ
- 期待する製品画面を文書化する
