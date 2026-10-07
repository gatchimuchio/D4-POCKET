# Mobile Flutterアプリ

Mobileは共通UIをFlutterで描画し、Device Linkの端末識別子・招待・結合資格・保管・TLS通信をplatform-native層へ閉じる。

## 現在の実装範囲

- Androidには固定MethodChannel、native招待画面、Android Keystoreで暗号化した資格保管、証明書hash固定TLS、有限timeout、Activity lifecycleからのbackground socket停止を実装した。API 35 Emulatorで製品UIから実Rust Brokerへの結合・Runtime表示・HOME復帰・切断が成立した。物理端末でのTLS・Keystore動作は未検証。
- Flutterへ返す接続状態は閉じたprojectionであり、端末ID・Host・招待・資格・secretを含めない。通常操作は既存Desktop Rust Brokerを通る。
- iOSにはSwift製Device Link channel handler、ThisDeviceOnly Keychain保管、証明書hash固定TLS client、厳格JSON検証、native XCTestが実装済みで、implicit Flutter engineへ登録される。Simulator build／XCTestの現行source検証状況は`docs/REV5_PRODUCT_PROGRESS.md`を参照する。Simulator buildやunit testは、Brokerへの実TLS接続・実端末動作の証拠ではない。
- 過去のdebug VM integration driverは招待をDartへ渡していたため廃止した。Androidは招待入力・保管・TLSをnative側に保つinstrumentation harnessで基本製品経路を確認した。iOSも秘密をDart、log、artifactへ渡さないnative harnessから実Brokerへ接続する。

## ローカル確認

```text
flutter pub get
flutter analyze --no-pub
flutter test --no-pub --reporter expanded
flutter build apk --debug --no-pub
```

Android実機のinstall・launch・結合・保管・復帰検証は2026-09-11のowner指示で凍結中。凍結は合格証拠ではない。Windows上でiOS buildはできないため、`.github/workflows/apple-manual-build.yml`の`ios_mobile`手動範囲でFlutter iOS Simulator buildとiOS native XCTestを検証する。このworkflowは実端末、Brokerとの実TLS接続、production identity、release readinessを証明しない。

`tooling/minidora_live_check.py --mobile-client` は開発用Python clientでRust Device Link wire pathを検証する。Mobile製品native channel、OS安全保管、実機lifecycleの証拠ではない。

### Android native製品経路の開発用試験

`tooling/minidora_live_check.py --android-native --android-serial emulator-5554 --reference <固定MINIDORA checkout>`は、AndroidJUnitRunnerから製品Flutter画面を操作し、native招待画面・Android Keystore・既存TLS・実Rust Brokerを通す。専用AVD `gui_shell_native_test`かつ`ro.kernel.qemu=1`だけを対象とし、物理端末では実行しない。先に同じsourceでdebug APKをbuildする。試験用APKはhelperが`:app:assembleDebugAndroidTest`で生成する。日本語pathをAndroid Gradle Pluginが拒否する場合は、sourceを変更せずASCIIの隔離worktreeでbuildする。

招待は、隔離Brokerのtest Ownerが発行した一時資格をhost loopbackからnative instrumentationだけへ渡す。shell引数・環境変数・Flutter・debug VM・出力artifactへ秘密を通さない。試験は誤pin拒否、結合、Runtime表示、HOMEからの復帰、製品画面での結合解除を確認し、Ownerの端末一覧と照合する。終了時は試験用招待・結合、ADB reverse、専用AVD内の製品／試験APKを回収する。専用AVDを止めるのは呼出元の責任とする。

証拠はEmulator上の基本製品経路に限定する。物理端末・配布identity・background通信の全timing・網羅的障害試験は証明しない。手動Actionsでも同じnative試験を追加実行でき、既存Python wire検査は独立して保持する。実行結果と未成立条件は`docs/REV5_PRODUCT_PROGRESS.md`を正本とする。

commit `0ca5dfa84dc1ca68ca8f01bae8ca456e1a9a12ba`の手動Actions [run 37580429609](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37580429609)で上記基本経路と誤pin拒否がPASSした。このProduct Build単位はCLOSEDであり、追加の証拠強化を次工程の条件にしない。

### iOS native接続の開発用試験

macOSで`tooling/minidora_live_check.py --reference <固定MINIDORA checkout> --ios-simulator <Simulator UUID> --ios-derived-data <専用build directory> --ios-result-bundle <専用xcresult path>`を使う。既存のSwift TLS／Keychain実装をnative XCTestから実Rust Brokerへ接続し、結合・再読・Runtime取得・離脱と誤pin拒否を確認する。物理端末を受け付けず、production identityや通常アプリのKeychain項目を使わない。

一時招待はnative test専用loopback受渡し内に限定する。[Appleの環境変数転送規約](https://developer.apple.com/documentation/xcode/environment-variable-reference)に従う`TEST_RUNNER_`には非秘密のportだけを渡す。XCTestは資格値・応答本文・例外本文をassertionへ表示せず、固定stageだけを失敗出力する。test用Keychain、Broker招待・結合、listenerを終了時に回収する。これはtransport／保管部品の実接続であり、Flutter UI操作・native確認dialog・OS lifecycle・物理端末の証拠ではない。実行結果は現行rev5進捗を参照する。

## リリース阻害項目

- item: iOS端末連携のnative実行時統合
  classification: release_blocker
  reason: 現行iOS sourceのSimulator buildとnative XCTest 8件は手動run #22でPASSした。これはSimulator内のKeychain試験に限り、物理端末・Desktop Rust Brokerへのnative LIVE_RUNTIME接続・TLS・lifecycleを証明しない。
  required_action: 秘密をFlutter／debug VM／log／artifactへ渡さないplatform-native harnessでDesktop Rust BrokerへのTLS接続・拒否・失効・background停止を検証し、iOS実機証拠を取得する。Android実機試験はowner指示の凍結を維持する。
  blocks_release: yes
- item: Android/iOS Device Linkの実動作証拠
  classification: release_blocker
  reason: Android Emulatorのnative製品経路は実Brokerへの結合・Keystore・Runtime表示・HOME復帰・切断を確認したが、物理端末とiOS実Broker接続は未成立。Android実機試験は凍結中。
  required_action: iOS native実Broker接続を成立させ、最終品質保証で必要なplatform境界を検証する。Android実機凍結の解除後に実機証拠を取得し、Emulatorの成功で代替しない。
  blocks_release: yes
- item: 公開配布の識別子と署名
  classification: release_blocker
  reason: 現在の識別子と署名は開発用。配布署名・経路は未検証。
  required_action: 公開配布前に正式識別子と署名を設定し、配布経路で成果物と動作を検証する。
  blocks_release: yes
