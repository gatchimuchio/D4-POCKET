# Mobile Flutterアプリ

Mobileは共通UIをFlutterで描画し、Device Linkの端末識別子・招待・結合資格・保管・TLS通信をplatform-native層へ閉じる。

## 現在の実装範囲

- Androidには固定MethodChannel、native招待画面、Android Keystoreで暗号化した資格保管、証明書hash固定TLS、有限timeout、Activity lifecycleからのbackground socket停止を実装した。API 35 Emulatorで製品UIから実Rust Brokerへの結合・Runtime表示・HOME復帰・切断が成立した。物理端末でのTLS・Keystore動作は未検証。
- Flutterへ返す接続状態は閉じたprojectionであり、端末ID・Host・招待・資格・secretを含めない。通常操作は既存Desktop Rust Brokerを通る。
- iOSにはSwift製Device Link channel handler、ThisDeviceOnly Keychain保管、証明書hash固定TLS client、厳格JSON検証、native XCTestが実装済みで、implicit Flutter engineへ登録される。Simulatorの部品接続に加え、製品Flutter UI→native招待／確認→実Rust Broker、Runtime表示、HOME復帰、画面からの離脱がXCUITestで成立した。試験入力flag付きSimulatorの証拠であり、通常Release・実端末・正式配布とは区別する。現行結果は`docs/REV5_PRODUCT_PROGRESS.md`を参照する。
- 過去のdebug VM integration driverは招待をDartへ渡していたため廃止した。Androidは招待入力・保管・TLSをnative側に保つinstrumentation harnessで基本製品経路を確認した。iOSも秘密をDart、log、artifactへ渡さないnative harnessから実Brokerへの部品接続を確認した。

## ローカル確認

```text
flutter pub get
flutter analyze --no-pub
flutter test --no-pub --reporter expanded
flutter build apk --debug --no-pub
```

Android実機のinstall・launch・結合・保管・復帰検証は2026-09-11のowner指示で凍結中。凍結は合格証拠ではない。Windows上でiOS buildはできないため、`.github/workflows/apple-manual-build.yml`の`ios_mobile`手動範囲でFlutter iOS Simulator buildと実Brokerへ接続するiOS native XCTestを検証する。このworkflowは製品UI全体、実端末、production identity、release readinessを証明しない。

`tooling/minidora_live_check.py --mobile-client` は開発用Python clientでRust Device Link wire pathを検証する。Mobile製品native channel、OS安全保管、実機lifecycleの証拠ではない。

### Android native製品経路の開発用試験

`tooling/minidora_live_check.py --android-native --android-serial emulator-5554 --reference <固定MINIDORA checkout>`は、AndroidJUnitRunnerから製品Flutter画面を操作し、native招待画面・Android Keystore・既存TLS・実Rust Brokerを通す。専用AVD `gui_shell_native_test`かつ`ro.kernel.qemu=1`だけを対象とし、物理端末では実行しない。先に同じsourceでdebug APKをbuildする。試験用APKはhelperが`:app:assembleDebugAndroidTest`で生成する。日本語pathをAndroid Gradle Pluginが拒否する場合は、sourceを変更せずASCIIの隔離worktreeでbuildする。

招待は、隔離Brokerのtest Ownerが発行した一時資格をhost loopbackからnative instrumentationだけへ渡す。shell引数・環境変数・Flutter・debug VM・出力artifactへ秘密を通さない。試験は誤pin拒否、結合、Runtime表示、HOMEからの復帰、製品画面での結合解除を確認し、Ownerの端末一覧と照合する。終了時は試験用招待・結合、ADB reverse、専用AVD内の製品／試験APKを回収する。専用AVDを止めるのは呼出元の責任とする。

証拠はEmulator上の基本製品経路に限定する。物理端末・配布identity・background通信の全timing・網羅的障害試験は証明しない。手動Actionsでも同じnative試験を追加実行でき、既存Python wire検査は独立して保持する。実行結果と未成立条件は`docs/REV5_PRODUCT_PROGRESS.md`を正本とする。

commit `0ca5dfa84dc1ca68ca8f01bae8ca456e1a9a12ba`の手動Actions [run 37580429609](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37580429609)で上記基本経路と誤pin拒否がPASSした。このProduct Build単位はCLOSEDであり、追加の証拠強化を次工程の条件にしない。

### iOS native接続の開発用試験

macOSで`tooling/minidora_live_check.py --reference <固定MINIDORA checkout> --ios-simulator <Simulator UUID> --ios-derived-data <専用build directory> --ios-result-bundle <専用xcresult path>`を使う。既存のSwift TLS／Keychain実装をnative XCTestから実Rust Brokerへ接続し、結合・再読・Runtime取得・離脱と誤pin拒否を確認する。物理端末を受け付けず、production identityや通常アプリのKeychain項目を使わない。

一時招待はnative test専用loopback受渡し内に限定する。[Appleの環境変数転送規約](https://developer.apple.com/documentation/xcode/environment-variable-reference)に従う`TEST_RUNNER_`には非秘密のportだけを渡す。XCTestは資格値・応答本文・例外本文をassertionへ表示せず、固定stageだけを失敗出力する。test用Keychain、Broker招待・結合、listenerを終了時に回収する。これはtransport／保管部品の実接続であり、Flutter UI操作・native確認dialog・OS lifecycle・物理端末の証拠ではない。実行結果は現行rev5進捗を参照する。

commit `599359be236962498f93842ec66145339290380f`の手動Actions [run 37584511577](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37584511577)で上記部品接続がPASSし、この単位はCLOSED。日本語Broker error codeの互換修正を含むnative XCTest 10件、Flutter test 21件、Mobile解析、Simulator buildが成功した。次は製品UI・native service・基本lifecycleの未成立部分を接続し、部品の証拠強化を開始条件にしない。

### iOS製品UI接続の開発用試験

上記iOS commandへ`--ios-product-ui`を付けると、RunnerUITests内の製品接続testだけを選択する。対象は`D4PocketNativeProduct-`で始まる新規専用Simulatorに限定し、通常端末の資格を操作しない。標準XCUITestが画面と確認ボタンを操作する。秘密を`typeText`のlogへ渡さず、明示compile flag `D4_IOS_PRODUCT_TEST`のnative入力helperだけが既存secure fieldを埋める。環境変数は非秘密portのみ。通常buildではhelperと起動呼出しをcompile除外し、同flagによる物理端末buildはcompile errorにする。製品serviceやforeground状態の直接書換えはしない。HOMEと製品復帰はXCUITestが操作する。試験用資格・招待を回収し、専用Simulator内のKeychainを含む環境の停止・削除は呼出側が行う。

手動Actionsの`target=ios_product_ui`がこの専用Simulatorを作成・回収する。既存の`ios_mobile`は部品試験として保持するが、CLOSEDした部品の証拠強化のためには実行しない。製品試験の失敗は固定stageで識別し、招待実値をUI runnerへ返さない。

commit `1188ed96439a56b698f0bca52f0ab4466b115aa8`の手動Actions [run 37592632518](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37592632518)で基本製品経路がPASS（XCUITest 1 passed／0 failed／0 skipped）。招待・結合の回収、秘密出力の不存在、専用Simulator停止・削除も確認した。この有限単位はCLOSED。通常Simulator build・Mobile解析・Flutter test 21件も成功したが、製品接続試験自体は専用入力flag付きであり、通常Releaseの証明へ読み替えない。

## リリース阻害項目

- item: iOS端末連携の最終platform保証
  classification: release_blocker
  reason: Simulator上の製品UI・native確認・実Broker接続・HOME復帰・離脱は成立したが、通常Release・物理端末・全lifecycle timingの証拠ではない。
  required_action: 最終品質保証で必要なplatform境界・実機証拠を取得する。基本経路は再開せず、Android実機試験はowner指示の凍結を維持する。
  blocks_release: yes
- item: Android/iOS Device Linkの実動作証拠
  classification: release_blocker
  reason: Android EmulatorとiOS Simulatorの基本製品経路は実Brokerへ到達したが、物理端末の安全保管・通信・lifecycleは未成立。Android実機試験は凍結中。
  required_action: 最終品質保証で必要なplatform境界を検証する。Android実機凍結の解除後に実機証拠を取得し、Emulator／Simulatorの成功で代替しない。
  blocks_release: yes
- item: 公開配布の識別子と署名
  classification: release_blocker
  reason: 現在の識別子と署名は開発用。配布署名・経路は未検証。
  required_action: 公開配布前に正式識別子と署名を設定し、配布経路で成果物と動作を検証する。
  blocks_release: yes
