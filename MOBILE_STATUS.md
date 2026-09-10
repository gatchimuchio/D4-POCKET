# Mobile 状態

## 現行rev2の対象

2026-09-10のowner rev2によりMobile正式project・端末連携・Android/iOS検証は現行作業の対象になった。以前のDesktop v1.0境界を理由に、この要求をpost_v1_scopeへ退避しない。完成製品releaseは未成立である。

既存6画面を保持し、実行系対話・接続先Host・端末結合・設定を追加した。共有表示は `packages/gui_shell_ui`、暗号化端末経路はDesktop Rust brokerを通る。Flutterと端末metadataは権限源にならない。Mobileのpubspec.lockは追跡済みであり、既存MANIFESTの除外規約とは別である。

## 観測した検証

- Mobile analyzeと14試験、共有6試験、Desktop32試験はPASS。
- Windows/Linux上のMobile製品Dart client → TLS → Rust Core → 固定参照MINIDORAの実対話はPASS。証明書拒否・停止・再確認・失効も検査した。
- Android開発APK/AABのbuild、APK署名・alignment・manifest、bundletoolによるAAB消費はPASS。
- iOS Simulator appは手動補助run `34446194013`、commit `27b8713fd1a9ecdb81abe1d4225b99b26bda84ba` でbuild成功。実機起動を証明しない。

## 未完了項目

- item: Android/iOS実機のinstall・launch・結合・対話・安全保管・lifecycle
  registry_id: rev2_mobile_device_evidence
  classification: release_blocker
  reason: ADBで接続済み実機を検出していない。host上のDart実通信とfixtureはOS安全保管や端末画面の証拠ではない。
  required_action: 実機を接続し、実際の資格保存・再起動・Host照合・対話・失効・復帰を測定する。
  blocks_release: yes

- item: 正式配布識別子・署名・配布経路
  registry_id: rev2_mobile_distribution
  classification: release_blocker
  reason: 開発識別子を使用中。AABの汎用JAR stream検証差の警告も保持する。
  required_action: ownerの正式識別子と配布署名を確定し、配布経路上の成果物を検証する。
  blocks_release: yes

- item: OS push通知
  classification: post_v1_scope
  reason: rev2の通知画面保持は実装したが、OS push配信基盤は今回の明示要求に含まれない。画面はlocal接続状態だけを表示する。
  required_action: 別途要求時に通知契約・同意・配信経路を定義する。
  blocks_release: no

詳細なコマンド・成果物hash・失敗履歴は `docs/REV2_PROGRESS.md` と `apps/mobile_flutter/README.md` を参照する。
