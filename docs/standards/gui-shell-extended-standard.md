# GUI Shell 拡張標準 v0.3.1

状態: Phase 0 Lock
適用範囲: GUI Shell / cross-device Runtime Operation Shell
主要決定: Flutter + Rust helper
参照 Runtime: BLUE-TANUKI
重要事項: BLUE-TANUKI は、Phase 0 における Reference Runtime Contract の対象として固定する。

GUI-Shell v1.0 の正式な実装制約は `docs/specs/gui-shell-spec-v1.md` に置く。この Phase 0 拡張標準は、技術選定と設計姿勢の記録として維持する。

## 1. 定義

GUI Shell は、複数の Runtime / Agent / Tool / Local Service を対象とする汎用 Runtime Operation Shell である。

GUI Shell は、LLM が読む「アプリケーション責任基盤」でもある。LLM 開発/統合エージェントは GUI Shell の contract を第一級の実装・統合面として読み、利用できるが、LLM は決して権限源ではない。

GUI Shell は、次を提供する。

- install から起動までの体験
- Runtime の起動と status monitoring
- Capability と Permission の制御
- Approval の作業流
- Audit の追跡記録
- Recovery の作業流
- Update の方針
- device 間の調整
- Adapter による Runtime 統合

## 2. 対象外

GUI Shell は、次を行ってはならない。

- BLUE-TANUKI 専用の MVP になる。
- BLUE-TANUKI 固有の logic を Shell Core に組み込む。
- 低水準の CLI / WSL / npm / Git / Runtime の複雑性を通常のユーザーへ露出する。
- terminal wrapper になる。
- LLM / Agent の output を権限として扱う。
- LLM による実装作業を、release readiness または外部標準への採用を示す証拠として扱う。
- 中核 asset を Flutter 固有の code に保存する。

## 3. 中核 asset

次の asset は、framework から独立した状態を維持しなければならない。

- 契約 schema
- Adapter の契約（Adapter Contract）
- Runtime のモデル（Runtime Model）
- Capability のモデル
- Permission のモデル
- Approval のモデル
- Audit event の形式
- Recovery action の形式
- 内容露出の境界（Content Exposure Boundary）
- 権限除去の適合規則（Authority Strip Conformance）
- Rust helper の境界
- conformance の test

## 4. UI framework の境界

Flutter は、次を所有してよい。

- 画面の rendering
- user input の収集
- UI state の保持
- 画面の navigation
- 表示 theme
- 表示の localization
- accessibility 対応

Flutter は、次を所有してはならない。

- authority の decision
- Permission の semantics
- Audit の semantics
- Adapter の Conformance
- Recovery の classification
- Content Visibility の rule
- Runtime Trust の rule

## 5. Phase 0 Lock の決定事項

- 汎用 GUI Shell の方向性: locked
- BLUE-TANUKI を Reference Runtime のみにする: locked
- BLUE-TANUKI を Phase 0 Reference Runtime Contract の対象にする: locked
- Flutter + Rust helper を第一候補にする: locked
- Compose MP を watchlist candidate にする: locked
- Tauri を desktop-heavy fallback にする: locked
- schema の作業順序は Schema-first: locked
- 適合性の作業順序は Conformance-first: locked

## 6. Runtime の安全 invariant

Shell は、次を強制または検査しなければならない。

- inbound authority key を除去する。
- external metadata は権限を昇格できない。
- Runtime が許可していない `authority_context` を GUI から作成できない。
- GUI input は権限ではない。
- Content Visibility を遵守する。
- Approval の編集範囲は field 単位に限定する。
- 編集後の payload を再 hash 化し、再検証する。
- すべての sensitive action について AuditEvent を作成する。
