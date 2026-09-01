# Flutter governance risk 台帳

状態: 能動監視項目

## 危険性

Flutter は Google 主導であり、次の影響を受ける可能性がある。

- 組織再編
- 優先順位の変更
- desktop の優先度低下
- Dart/Flutter team の資源変更
- ecosystem の減速

## 影響

想定する影響領域:

- desktop support 品質
- tooling 品質
- platform channel 安定性
- package ecosystem の健全性
- issue 解決速度

## 緩和策

- core schema を framework 非依存に保つ。
- Rust helper を Flutter の外に保つ。
- Adapter Contract を Flutter の外に保つ。
- Dart を UI 層に限定する。
- Flutter に依存しない conformance test を維持する。
- `FrameworkRiskProfile` を通じて6ヶ月ごとに再評価する。

## 離脱信号

- desktop support が実質的に劣化する
- 公式 roadmap で desktop の優先度が下がる
- Flutter 起因の重大な GUI Shell bug を回避できなくなる
- ecosystem の健全性が安定した製品提供を遮断するまで低下する
