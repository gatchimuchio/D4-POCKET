# Tauri fallback

状態: 監視表 / desktop 偏重時の fallback
第一候補は引き続き Flutter + Rust helper
Fallback 開始条件: Flutter desktop の実用性が実質的に劣化するか、戦略が desktop 偏重になる

## 位置づけ

Tauri は GUI Shell 実装の第一候補ではない。

次を提供するため、desktop 偏重の方向に対する fallback として残す。

- Rust backend との整合
- 明示的な capability style の考え方
- Electron より小さい desktop footprint
- 局所 desktop host 表層への高い適合性

## 第一候補でない理由

GUI Shell は PC と mobile を同等に重視する。

現行対象に対し、Tauri は次の理由で Flutter より弱い。

- mobile 等価性により多くの検証が必要
- WebView / JavaScript / native bridge audit の負担が大きい
- desktop と mobile の UI 一貫性がより間接的
- どの場合でも Shell Core contract はframework非依存に保つ必要がある

## 再評価条件

次の場合に Tauri を再評価する。

- GUI Shell 戦略が desktop host 優先へ変わる
- mobile が companion 専用になる
- Flutter desktop support が実質的に劣化する
- Flutter tooling または package ecosystem が不適合になる
- TypeScript asset の再利用が統合 Flutter UI より重要になる

## 境界規則

後で Tauri を採用しても、UI/runtime host 層の責任だけに留める。

次の asset を Tauri 固有 code へ移してはならない。

- JSON Schema
- Adapter Contract
- Permission model
- Approval model
- AuditEvent model
- RecoveryAction model
- Content Exposure Boundary
- Authority Strip Conformance
- Shell Core の権限判定
