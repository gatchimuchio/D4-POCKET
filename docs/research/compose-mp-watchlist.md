# Compose Multiplatform 監視表

状態: 対案候補

## 位置づけ

Flutter が不適合になった場合、Compose Multiplatform + Kotlin + Rust helper を第一の移行候補とする。

## 強み

- JetBrains による governance
- Kotlin の ecosystem
- Android 基盤
- desktop 連続性
- 長期的な言語/tooling 安定性

## 現行の遮断要因

- iOS stable の成熟期間が Flutter より短い
- GUI Shell の規模に対する製品参照事例が少ない
- API 調整 risk が Flutter より高い

## 監視条件

次の場合に再評価する。

- Compose MP が追加の製品証拠を1〜2年蓄積する
- iOS の安定性が実質的に向上する
- Flutter desktop の優先度が低下する
- Flutter governance risk が増加する
