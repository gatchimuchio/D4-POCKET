# Mobile Flutterアプリ

Mobileは共通UIをFlutterで描画し、Device Linkの端末識別子・招待・結合資格・保管・TLS通信をplatform-native層へ閉じる。

## 現在の実装範囲

- Androidには固定MethodChannel、native招待画面、Android Keystoreで暗号化した資格保管、証明書hash固定TLS、有限timeout、Activity lifecycleからのbackground socket停止を実装した。Kotlin compile、unit test、debug APK/AAB buildは成功したが、実TLS・Keystoreの実端末動作は未検証。
- Flutterへ返す接続状態は閉じたprojectionであり、端末ID・Host・招待・資格・secretを含めない。通常操作は既存Desktop Rust Brokerを通る。
- iOSのnative Device Link handlerは未実装。channel未登録時はFlutterがfail-closedする。iOS build成功を接続機能の証明にしない。
- 過去のdebug VM integration driverは招待をDartへ渡していたため廃止した。新しいLIVE_RUNTIME試験は、招待入力・保管・TLSをnative側に保ち、秘密をDart、log、artifactへ渡さないplatform test harnessが成立するまで実行しない。

## ローカル確認

```text
flutter pub get
flutter analyze --no-pub
flutter test --no-pub --reporter expanded
flutter build apk --debug --no-pub
```

Android実機のinstall・launch・結合・保管・復帰検証は2026-09-11のowner指示で凍結中。凍結は合格証拠ではない。iOSはWindows上でbuildできず、Apple platformの手動補助workflowはbuildのみである。

`tooling/minidora_live_check.py --mobile-client` は開発用Python clientでRust Device Link wire pathを検証する。Mobile製品native channel、OS安全保管、実機lifecycleの証拠ではない。

## リリース阻害項目

- item: iOS Device Link native経路
  classification: release_blocker
  reason: iOSはKeychain・TLS・native channel handlerが未実装。
  required_action: Androidと同じ境界を満たすSwift handlerとKeychain保管を実装し、Mac上でbuild・試験する。
  blocks_release: yes
- item: Android/iOS Device Linkの実動作証拠
  classification: release_blocker
  reason: Kotlin unit testとAPK/AAB buildは実OS保管、実TLS、結合、失効、background lifecycleを証明しない。Android実機試験は凍結中。
  required_action: 秘密をDartへ露出しないnative test harnessで各platformのOS保管・接続・失効・background復帰を検証し、Android実機凍結の解除後に実機証拠を取得する。
  blocks_release: yes
- item: 公開配布の識別子と署名
  classification: release_blocker
  reason: 現在の識別子と署名は開発用。配布署名・経路は未検証。
  required_action: 公開配布前に正式識別子と署名を設定し、配布経路で成果物と動作を検証する。
  blocks_release: yes
