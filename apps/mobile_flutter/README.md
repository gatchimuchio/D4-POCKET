# モバイルFlutter app

2026-09-11のowner指示によりAndroid実機検証を凍結する。再開指示まで端末接続の要求や実機install・launch・結合・安全保管・lifecycle検証を行わない。APK/AABの既存成果物とbuild結果は保持する。凍結は実機検証の合格を意味せず、既存のrelease_blockerを保持する。iOSはこの凍結の対象外。

owner rev2指示によりAndroid/iOSの正式projectを追加した。Flutter共通表示層は `../../packages/gui_shell_ui` を使用する。
概要・確認・通知・実行系・停止・復旧を維持し、対話・接続先・設定を追加した。接続前は未接続を表示する。実行系一覧はDesktopの登録観測であり稼働保証ではない。対話はMobile → TLS → Desktop Rust → Core → Adapter → Runtimeだけを通る。

Agent画面は既存Device Link経由でAgent Adapter metadataを読み取り、状態と宣言Capabilityを表示する。Agent起動、Permission、Approval、Credential、Tool実行は扱わず、`ready`表示も実task成功を意味しない。

接続先画面の端末IDをDesktop ownerへ渡し、owner CLIで招待を新規fileへ発行する。安全な対面手段で受け取った招待JSONを入力し、Hostと証明書hashを照合して結合する。資格はAndroidの安全保管／iOS Keychainへ書き、再読取で保存を確認する。保管エラー時は平文へfallbackしない。招待秘密・本文を永続保存しない。OSのbackupとdevice transferからapp dataを除外する。

画面復帰時は端末確認後に同じ保留要求を照会する。background中の通信と自動再送は行わない。Host変更は旧結合解除と新しい招待を必要とする。設定の通常解除はDesktop失効を先に確認し、通信不能時の端末内だけの削除とは区別する。

`flutter analyze`、`flutter test` は構造・FIXTURE検証。`python tooling/minidora_live_check.py --reference <固定参照clone> --dart-mobile-client` はDart製品TLS clientから実MINIDORAへのLIVE_RUNTIME検証であり、Android/iOSの安全保管や実機表示を証明しない。

Androidは安全保管plugin 11の要件に従いcompile SDK 37、AGP 9.1.1、Gradle 9.3.1、JDK 17を使用する。min SDK 24とtarget SDKはFlutter 3.44の指定を保持する。開発試験は `flutter build apk --debug`、`flutter build appbundle --debug`。開発識別子は `com.example.gui_shell_mobile`。releaseへdebug署名を流用しない。
iOSはmacOSとXcodeでのbuildおよび端末証拠を別途必要とする。

Flutter生成時のGradle設定（heap 8GB、metaspace 4GB）は約6GBメモリのWindows開発hostでnative allocation失敗を起こした。標準Gradle設定をheap 2GB、metaspace 768MB、worker 2へ限定した。独自build経路は追加しない。これは開発buildの資源設定であり製品の動作条件ではない。より大きい設定が必要になった場合は実測して変更する。

- item: 端末連携とplatform別実動作
  classification: release_blocker
  reason: Dart製品TLS経路の実動作はAndroid/iOSのOS安全保管・実機lifecycleの証拠ではない。
  required_action: 各platformでbuild・install・起動・安全保管・対話・失効・復帰を検証する。
  blocks_release: yes

一次資料: [Flutter Secure Storage](https://pub.dev/packages/flutter_secure_storage)、[Dart SecureSocket](https://api.dart.dev/dart-io/SecureSocket/connect.html)。
- item: 公開配布の識別子と署名
  classification: release_blocker
  reason: 現在の識別子と署名は開発用。AABには汎用jarsignerのstream検証差の警告もあり、所有者の配布署名と配布経路は未検証。
  required_action: 公開配布前に正式識別子と署名を設定し、利用するAndroid配布経路でAABと派生APKの署名・内容・動作を検証する。
  blocks_release: yes

開発成果物の確認: APKとAABのbuild、APK署名・alignment・manifest、bundletool 1.18.3によるAAB検証とAPK変換を実行してPASS。汎用JAR検証の警告は上記配布前blockerに保持する。実機install・launch・安全保管は未確認であり上記実動作blockerを解消しない。
