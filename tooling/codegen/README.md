# Codegen 境界

schema 駆動の予約済み code generation 境界。

classification: post_v1_scope
reason: 現行 v1.0 gate は生成済み contract artifact を登録済みで保ち、より広い codegen 自動化は延期する。
blocks_release: no

対象:

- Dart contract
- Rust struct
- 必要な場合の TypeScript client
- 必要な場合の OpenAPI bridge

`specs/` 下の schema を機械契約の正本とする。
