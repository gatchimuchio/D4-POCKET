# Mobile 状態

所有者が明示的に範囲を変更しない限り、Mobile はv1.0完成製品 release に含めない。

Mobile は `post_v1_scope` である。mobile Flutter app は境界付き companion 表層として残せるが、この整理は mobile を改良せず、`apps/mobile_flutter/pubspec.lock` を追跡しない。mobile 限定作業中に局所 Flutter tooling がこれを再生成することがある。

所有者が mobile を release 範囲へ明示的に追加しない限り、v1.0 release gate、CI 製品主張、広告する support 表層から除外する。

## 実装済み領域

- item: mobile 概要画面
  classification: post_v1_scope
  reason: mobile 完全 release はv1.0 desktop 範囲外である。
  blocks_release: no

- item: approval 確認表層
  classification: post_v1_scope
  reason: mobile 完全 release はv1.0 desktop 範囲外である。
  blocks_release: no

- item: 通知表層
  classification: post_v1_scope
  reason: mobile 完全 release はv1.0 desktop 範囲外である。
  blocks_release: no

- item: runtime 状態表層
  classification: post_v1_scope
  reason: mobile 完全 release はv1.0 desktop 範囲外である。
  blocks_release: no

- item: 緊急停止要求表層
  classification: post_v1_scope
  reason: mobile 完全 release はv1.0 desktop 範囲外である。
  blocks_release: no

## v1後の範囲

- item: 実端末 pairing
  classification: post_v1_scope
  reason: mobile 完全 release はv1.0 desktop 範囲外である。
  required_action: mobile release 段階で完了する。
  blocks_release: no

- item: push 通知
  classification: post_v1_scope
  reason: mobile 完全 release はv1.0 desktop 範囲外である。
  required_action: mobile release 段階で完了する。
  blocks_release: no

- item: mobile 用 Shell Core IPC
  classification: post_v1_scope
  reason: mobile 完全 release はv1.0 desktop 範囲外である。
  required_action: mobile release 段階で完了する。
  blocks_release: no

- item: 暗号学的な端末結合
  classification: post_v1_scope
  reason: mobile 完全 release はv1.0 desktop 範囲外である。
  required_action: mobile release 段階で完了する。
  blocks_release: no

- item: mobile release 用 packaging
  classification: post_v1_scope
  reason: mobile 完全 release はv1.0 desktop 範囲外である。
  required_action: mobile release 段階で完了する。
  blocks_release: no
