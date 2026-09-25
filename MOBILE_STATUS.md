# Mobile 状態

## D4 Pocket rev2 最新境界確認（2026-09-25）

旧Dart製品client、資格モデル、安全保管API、SecureSocket、招待をDartへ注入するdebug VM integration harnessを廃止した。Flutterは固定MethodChannelを通じて状態projectionとallowlist済みBroker operationだけを送受信する。Android Kotlin／iOS Swiftへnative store・serviceを実装し、`local_delete`は資格削除と同じOS保護状態へ最大32件の非権威回復記録を保存する。Flutterにはversionだけで読む固定methodと、秘密fieldを拒む限定表示を追加した。Flutter試験16件・analyzeは成功。現行のAndroid sourceは対象のMobileアプリと共有UI計117ファイルを一時検証先へ複製し、内容hash差分0件を確認した検証先で、`gradlew.bat clean :app:testDebugUnitTest :app:assembleDebug --no-daemon --offline --console=plain`が成功した。JUnit 8件はすべて成功し、debug APKは146,311,641 bytes、SHA-256 `2D40701A90A518261D5E9E7E5E96AADF036D1A78354B9181B0E001E9A6632129`。これは現行sourceの隔離clean build証拠で、実機・実TLS・製品releaseの証拠ではない。元OneDrive作業場所の除外対象`apps/mobile_flutter/build`には親からの`Everyone Deny DeleteSubdirectoriesAndFiles`が継承され、標準出力先では後片付けとresource生成が`AccessDenied`となる。ACLは変更せず、一時検証先で試験した。iOS native sourceは追加済みだがApple toolchain compile／XCTest未確認。実TLS・Keystore/Keychain実機動作は未検証である。Flutterからnative foreground状態を偽装できる`set_foreground` channelは設けない。

- `release_blocker`: rev2_mobile_flutter_native_device_link_boundary。Androidの隔離clean build、unit test 8件、debug APK buildは成功した。iOS native compile／XCTest、Rust Brokerへのnative LIVE_RUNTIME harness、実機のOS保管・TLS・lifecycle証拠は未成立。local_delete記録は端末内INTERNAL_STATEだけでDesktop Audit chain・失効証明ではない。
- `known_limitation`: OneDrive内の除外対象Android build outputに削除deny ACLが継承され、標準Gradle output pathで後片付け／resource packagingが失敗する。ACLを変更せず、hash照合した一時検証先でclean build／test／APKを検証した。in-place buildが必要な場合だけACL管理者の判断を得る。blocks_release: no。
- Android実機検証凍結、rev2_mobile_device_evidence、正式配布blockerは維持する。Flutter/Kotlin unit test・APK build・emulator起動は実機証拠へ昇格しない。
- 旧Dart client・Simulator integrationのPASSは履歴のまま保持し、現行native経路の適合根拠に使用しない。

## C31現況（2026-09-24）

Mobileは既存の9画面を保持し、C24で通知summary、Runtime lifecycle状態、資源概要、owner再承認待ち停止receipt、現在owner承認に結合した履歴metadataを既存Rust Brokerの読み取り専用経路へ投影する。MobileはApproval、Permission、Authority、Credential、MCP接続、Tool実行、実停止を所有しない。取得不能な資源値は`unknown`のままとし、MCP live一覧はDesktop owner管理面の境界上、未観測として表示する。

Windows hostではMobile Flutterの解析と試験、Android debug APK／AABのbuildを確認している。C30のDevice Link行はcontract・失効・背景遷移fixtureでPASSしたが、実端末、native secure storage、TLS実接続、Windows installed productからのMobile連携、正式署名・配布は証明していない。

- `release_blocker`: Android／iOS実機、native安全保管、OS lifecycle、Windows installed productとのMobile連携、正式識別子・署名・配布、owner GO、正式release。
- `known_limitation`: Mobileはowner操作とMCP live一覧を持たず、Device Link／比較のlocal fixtureは実端末・外部Runtimeの証拠ではない。

## rev2開始時の対象（履歴）

2026-09-10のowner rev2によりMobile正式project・端末連携・Android/iOS検証は現行作業の対象になった。以前のDesktop v1.0境界を理由に、この要求をpost_v1_scopeへ退避しない。完成製品releaseは未成立である。

既存6画面を保持し、実行系対話・接続先Host・端末結合・設定を追加した。共有表示は `packages/gui_shell_ui`、暗号化端末経路はDesktop Rust brokerを通る。Flutterと端末metadataは権限源にならない。Mobileのpubspec.lockは追跡済みであり、既存MANIFESTの除外規約とは別である。

## Android実機検証の凍結（2026-09-11）

owner指示により、Androidの実機install・launch・端末結合・対話・安全保管・lifecycle検証を凍結する。再開指示まで端末接続を要求せず、実機検証を実施しない。ビルド済みAPK/AABとhost上の検証結果は保持する。凍結は実機合格やrelease許可を意味せず、未検証のrelease_blockerを保持する。iOS実機検証はこの凍結指示の対象外である。

## 観測した検証

- Mobile analyzeと14試験、共有6試験、Desktop32試験はPASS。
- Windows/Linux上のMobile製品Dart client → TLS → Rust Core → 固定参照MINIDORAの実対話はPASS。証明書拒否・停止・再確認・失効も検査した。
- Android開発APK/AABのbuild、APK署名・alignment・manifest、bundletoolによるAAB消費はPASS。
- iOS Simulator appは手動補助run `34446194013`、commit `27b8713fd1a9ecdb81abe1d4225b99b26bda84ba` でbuild成功。実機起動を証明しない。

## 未完了項目

- item: Android/iOS実機のinstall・launch・結合・対話・安全保管・lifecycle
  registry_id: rev2_mobile_device_evidence
  classification: release_blocker
  reason: Android実機検証はowner指示で凍結中。iOS実機の証拠も未取得。host上のDart実通信とfixtureはOS安全保管や端末画面の証拠ではない。
  required_action: Androidはownerの再開指示後に検証する。iOSを含む各platformの実機で資格保存・再起動・Host照合・対話・失効・復帰を測定する。
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
