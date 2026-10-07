# 端末連携の意味正本

## 2026-09-25時点の状態記録（履歴）

状態: Rustの暗号化端末経路とowner制御は実装済み。Android KotlinとiOS Swiftのnative経路、端末内回復記録の保存・読み取り実装を追加した。Flutter test 16件・analyzeは成功。現行Android／共有UI source 117 fileのhash一致を確認したfresh Temp copyでGradle clean・unit test・debug APK buildが成功し、JUnit 8件はすべて成功した。APKは146,311,641 bytes、SHA-256 `2D40701A90A518261D5E9E7E5E96AADF036D1A78354B9181B0E001E9A6632129`。元OneDrive workspaceのignored build outputは継承ACLの削除denyにより通常cleanup／resource packagingが失敗する。sourceやACLは変更せずTemp copyで検証したhost-localなknown limitationである。iOS native handlerとSimulator XCTestはApple toolchainによるcompile／testが未確認。通信不能時の端末内だけの削除はOS保護状態内へ最大32件の非権威回復記録を同時保存し、Flutterは限定された情報を読み取り表示するが、Desktop Rust Brokerの監査連鎖とは別である。nativeから同じRust Brokerへ到達する`LIVE_RUNTIME`試験、実機の安全保管・lifecycle証拠も未成立である。過去のDart TLS実行証拠を現在のFlutter境界適合証拠へ読み替えない。契約試験・native単体試験・Simulator buildは実機証拠とは区別する。

## 現在状態（2026-10-07）

iOS Simulator buildとnative XCTestは、手動Actions run [#22](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37558500619)、commit `b6dbf10c4ea8b34e103b8826f2d23f25930d8ca9`でPASSした。Flutter analyze、21件のFlutter test、Simulator build、8件のnative XCTest（8 passed／0 failed）を確認した。Keychain項目のThisDeviceOnly保護、保存・読戻し・削除、旧version stateのmigrationをSimulator上で検査した。test用ad-hoc署名であり、production identity・物理端末・実Broker接続・実機lifecycleの証拠ではない。これらの範囲と失敗履歴は`docs/REV5_PRODUCT_PROGRESS.md`に記録する。native Rust Brokerへの`LIVE_RUNTIME`接続と実機証拠は未成立である。

## 対象と責任

nativeが受け取るBrokerのerror codeは、既存の日本語識別子（例: `端末要求拒否`）を含む文字・数字と`_`、`.`、`-`に限定する。iOSではUTF-8長1〜96 byteを保ち、空文字・空白・制御文字・超過値を拒否する。codeは失敗識別用の限定metadataであり、Authorityや成功へ変換しない。自由文messageは従来どおりFlutterへ渡さない。

端末招待は、Desktopのownerが指定した端末を一回だけ結合するための期限付き資格である。端末結合資格は、そのDesktop起動世代と端末に結合する通常操作資格である。HostIDはDesktopの起動世代を、証明書hashは暗号化接続先の公開証明書を識別する。接続先Hostとportは到達先であり、それだけで信頼や権限を生じない。

経路は Mobile → 暗号化端末連携 → Desktop Rust broker → Shell Core → Adapter → Runtime とする。Mobileへloopback資格やowner制御資格を渡さない。端末資格はowner承認、汎用command、任意URL送信、Permission変更、監査確定を許可しない。Runtimeへの送信は既存の要求hashへのowner承認を引き続き必要とする。

経路分類はcontrol経路。Capabilityは端末招待・結合・通常対話、Permissionは指定Host・端末・当該端末が作った対話、Approvalは招待時のowner操作と対話ごとの既存承認、AuditEventは発行・結合・拒否・取消・失効・通常操作、RecoveryActionは再結合・資格削除・新規対話・監査修復に対応する。

owner制御操作は `端末招待`（端末ID・接続先Host）、`端末一覧`、`端末招待取消`（招待ID）、`端末失効`（結合ID）に限定する。通常loopback資格と端末資格から呼べない。未失効の同一端末を重複結合しない。再結合前にownerが旧資格を失効させる。

## 招待と結合

Desktopは明示的な起動設定がある場合だけ、指定したprivate IPv4またはloopbackで端末経路を開く。wildcard bind、公衆IP、名前解決、UPnP、firewall変更、自動探索を行わない。エミュレータ用の到達先とbind先が異なる場合もownerが明示する。

ownerはMobileで生成した128bit以上の端末IDを確認して招待を発行する。招待IDは128bit以上、招待秘密は256bit以上の乱数とし、発行から300秒で失効する。招待は端末ID、HostID、証明書hash、到達先に固定する。ownerからMobileへ安全な対面手段で渡すことが最初の信頼の根拠であり、networkやmetadataから招待を自動採用しない。操作者にHostと証明書hashを表示して確認を求める。

端末結合の成功時に招待を消費し、新しい256bit以上の端末秘密を返す。Desktopは秘密のhashだけを保持する。結合資格は最大8時間で失効し、自動延長しない。同じ招待の再送・別端末での使用・取消済み招待を拒否する。結合応答を受け取れなかった場合も招待は再使用せず、ownerが新しい招待を発行する。

最大32件の未失効招待と32件の結合を許容し、上限時は追加を拒否する。端末IDの自己申告を人間本人性の証明と扱わない。秘密の所持とownerの招待は同一OS利用者の悪意あるprocessや管理者への耐性を保証しない。

## 暗号化と入力

TLSの確立後、application資格を送る前に招待／保管値の証明書hashと実peer証明書を照合する。OSが証明書を信頼している場合も固定hashを省略しない。検証無効化や任意証明書の受理を禁止する。証明書の有効期間も確認する。秘密を平文TCP、URL、log、監査本文へ出力しない。

要求は64KiB以内のUTF-8 JSON一行、応答は4MiB以内とし、読書きと接続に有限の期限を設ける。未知field、重複field、不正型、未知操作、過大入力を拒否する。受信内容のhashをparse前に計算して監査に結合し、生資格を含むrawは永続logへ書かない。

要求のHostID・端末ID・資格ID・秘密・有効期限をDesktopで照合する。発行時刻はserver時刻との差60秒以内とし、nonceは128bit以上の乱数を使用する。同一結合でnonceを再使用しない。最大4096nonceを保持し、上限時は安全側に拒否して再結合を求める。再接続でもnonce履歴と所有関係を維持する。入力の構造適合やMobile保存状態だけで認証成功を推定しない。

## 所有関係と失効

端末経路からの対話開始で作られたsessionは結合IDに帰属し、送信要求も同じ結合へ帰属する。別端末・別結合・Desktopで作ったsessionと要求の取得、送信、中止、終了を拒否する。実行系の列挙は接続先の観測だけであり送信権限を付与しない。応答の表示範囲は既存Coreの射影を保持する。

ownerの招待取消は未使用招待を失効させる。ownerの端末失効およびMobileの端末離脱は以降の全操作を拒否し、当該結合の対話を終了・隔離して未送信の送信と遅延結果の採用を防ぐ。送信済みRuntimeの計算停止を保証したとは報告しない。監査永続化に失敗した場合は資格付与や結果公開を拒否する。安全側の利用停止は監査障害でも維持し、通常利用への復帰はownerによる復旧を必要とする。

Desktop再起動時はHostIDと証明書を更新し、全端末資格・招待・未完了対話を失効させる。これは起動世代を跨ぐ復元を実装しない明示的境界である。古い資格から自動再結合・自動再送しない。

## Mobileの保管とlifecycle

Mobileは端末ID、Hostの固定情報、結合ID、端末秘密、有効期限だけをAndroidの安全保管／iOS Keychainへ保存する。招待秘密は結合後に消去する。一般設定、標準preferences、log、clipboardへの自動書出し、バックアップからの自動復元へ秘密を置かない。保管失敗時に平文へfallbackしない。

起動・復帰は資格の読取後に端末確認を行い、失効・期限切れ・Host不一致では入力と送信を無効にする。background中はpollingと新規送信を停止する。応答待ちは復帰後に同じ要求を照会し、自動再送しない。離脱はserver失効を先に要求する。通信不能時のlocal削除はserver失効と区別し、owner側の失効操作を案内する。

`local_delete`は通信不能時に端末内資格だけを消す回復操作であり、Desktop Rust Brokerの`AuditEvent`を生成・永続化できない。Flutterの確認dialogや成功表示は監査証拠ではない。このためnative OS保護領域内の`mobile_local_recovery`記録を、端末内資格の削除と同じ暗号化状態へ一回の更新で追記する。記録は`mobile_local_credential_delete`、`actor=shell`、`operator_identity=unverified`、結果`success`、`evidence_source=INTERNAL_STATE`、`desktop_revocation=unconfirmed`、`authority_effect=none`に固定し、UI操作から操作者本人を証明せず、資格値、Host、端末ID、自由文を含めない。直近32件を上限に古い記録からrotateし、削除操作のhashは固定の公開操作識別子から計算する。

この記録はDesktop Brokerの監査連鎖、外部収集証拠、改変耐性、Desktop側失効、本人性または権限を証明しない。記録と資格削除を一緒に再読取確認できない場合はnative操作を成功として返さない。これは資格削除の回復記録であり、端末資格が既にない場合に架空の削除記録を追加しない。通常の`disconnect`はDesktop失効Auditと削除後readbackを引き続き必要とし、端末内だけの削除と混同しない。未確認のDesktop失効はOwnerへ別操作を案内する。

`read_recovery_audit`はversionだけを受け取る読み取り専用のnative channel methodであり、保存済み記録を最大32件返す。記録がない場合は空配列を返し、端末IDを新規生成しない。日時はUTC `Z`表記の秒精度または3桁ミリ秒精度に固定し、nativeとFlutter双方がfield集合、定数値、hash、実在日時、件数、event IDの一意性を再検証してから表示する。表示は内部状態の投影でありAudit chainへの書込み、復旧承認、資格復元を実行しない。

復帰・手動再確認では安全保管の端末IDと資格を再読取し、構造・期限と現在の結合内容を照合してから通信する。削除・破損・変更・読取障害時にメモリ上の旧資格を代用せず、変更された資格を自動採用しない。読取や接続確認の途中でbackgroundに移った確認結果を、後の復帰の接続成功に転用しない。

通常の解除では有効なclientから端末離脱の応答を確認し、client不在を解除成功にしない。保存資格の削除後は同じ保管経路で再読取し、不在を確認してから端末内の削除完了を表示する。資格が残る場合や再読取に失敗した場合は通信停止を維持し、削除未確認として再操作を案内する。これは保管APIを通した観測であり、媒体上の物理消去や別processによる後続の再書込み防止を証明しない。

既存の概要・確認・通知・実行系・停止・復旧画面は維持する。接続前の固定preview値を実Runtime状態として表示せず、未接続を明示する。未実装の承認や停止操作を成功表示しない。

## D4 Pocket rev2のFlutter／native境界

Flutterは接続状態と許可済みprojectionだけを表示する。招待JSONをDart TextFieldへ入力しない。native側が招待入力・Host／HostID／証明書hashの確認・結合確認を行い、成功時はnative側で招待秘密を破棄して結合資格をOS安全保管へ保存する。Flutterには端末IDと資格を含まない接続状態projectionだけを返す。

Flutterからnativeへのchannel要求は、版、固定method名、必要な場合の既存Device Link operationとその業務payloadだけから成る。pairing要求は引数なしであり、招待・資格・secret・token・session credentialをchannel越しに渡さない。nativeの通常要求経路は既存Device Link TLSの宛先へ接続し、既存Rust Brokerの資格検査・nonce・allowlist・所有関係・Audit・Approval・Recoveryへ必ず到達する。native側に権限判断、Owner操作、任意host／URL、任意commandを追加しない。

`read_recovery_audit`はこのnative channel内のローカル読み取り専用例外であり、要求fieldは`version`だけ。返却は上記Schemaに適合する最大32件に固定し、資格値を含めず、Broker要求やauthority判断を行わない。

foreground状態の正本はAndroid Activity／iOS sceneなどplatform-native lifecycle観測とする。Flutterから`set_foreground`等の状態設定要求を受け付けず、Flutterのlifecycle通知だけでnative通信を再開しない。復帰時はnative自身がforeground状態を確認したうえで端末資格を再照合する。Flutter側のlifecycle状態は、UI要求を止める保守的な追加条件としてのみ使う。

履歴閲覧時だけ、現在のowner承認状態を照合するための`approval_id`参照と閉じた履歴queryをnative channel経由でBrokerへ渡せる。これはApproval本文・token・発行操作ではなく、native側は権限や承認有効性を解釈しない。許可はRust Brokerが保持する現在のowner grant、期限、実行系範囲の照合だけで決まる。他操作のpayloadでは`approval_id`を含む権限・監査fieldを引き続き拒否する。

Android実装はAndroid Keystore保護のnative暗号化保管を使い、iOSはThisDeviceOnly Keychain handlerの実装を要する。OS保管が利用できない場合はfail-closedとし、Dart保管・平文保存・backup復元へfallbackしない。TLS証明書hashと有効期間、有限timeout、bounded frame、background中のsocket停止、要求一回限りの扱いを既存契約から弱めない。native応答はoperationごとに検証し、資格field／資格実値を再帰的に除去または拒否してからFlutterへ渡す。例外・system log・test artifactにも秘密値を含めない。

このchannelはMobile製品内のnative transport／保管境界であり、Rust Brokerを迂回する別bridgeではない。Schema・fixture・静的conformanceは契約形状の証拠に限る。実Device Link、Android/iOS OS保管、TLS、background停止のLIVE_RUNTIME証拠とは区別する。

端末wire要求の操作集合はRust `device_link.rs`の通常資格allowlistと一致させる。招待資格では空payloadの`端末結合`だけを使い、結合資格では空payloadの`端末確認`、`端末離脱`、`実行系列挙`、`Agent一覧`、`対話履歴閲覧状態`を許可する。`対話開始`は`実行系ID`、`対話送信`は`対話セッションID`と`入力`、`対話取得`／`対話中止`は`要求ID`、`対話終了`は`対話セッションID`を受け取る。Runtime状態・資源観測は`版`と`実行系ID`、通知一覧は`版`と任意の`未読のみ`／`上限`、全Runtime停止要求は`版`だけを受け取る。履歴閲覧は`approval_id`とbounded `query`を受け取り、既存のowner承認が現在有効な範囲だけを閲覧する。owner専用の端末管理・承認操作はこの集合に含めない。

`specs/device_link_request.schema.json`は、要求全体と操作別payloadを閉じたSchemaとして表す。ConformanceはSchemaの操作集合とRust通常資格allowlistの差分、各操作の正例、未知field・owner昇格・不一致payload・範囲外値の拒否を検査する。Schema適合はRustでの資格、所有関係、現在のApproval、Audit成立を代替しない。

## 受入試験と残存境界

Schemaとconformanceは招待・保管資格・要求の構造と禁止操作を検査する。Rustの実経路では正常結合、再接続、招待取消、端末失効、期限切れ、不正資格、nonce再使用、Host不一致、他端末の所有物操作、owner昇格、監査障害を検査する。TLS実接続、異なる証明書拒否、Mobile安全保管、MINIDORA実対話、lifecycleは別のLIVE_RUNTIME証拠を必要とする。

- item: native Device Linkの実機・統合検証
  classification: release_blocker
  reason: iOS Simulator buildとnative XCTest 8件は2026-10-07の手動run #22でPASSしたが、Simulator証拠は物理端末のKeychain／TLS／lifecycleを証明しない。Androidの現行sourceはfresh Temp copyでclean・unit test・APK buildが成功しJUnit 8件が通過した一方、Android実機試験はowner指示で凍結中。両platformともDesktop Rust Brokerへのnative LIVE_RUNTIME接続、失効・background停止、実端末証拠は未成立。OneDrive内Android ignored outputの削除deny ACLはhost-local limitationとして維持し、ACLは変更していない。
  required_action: 招待秘密をFlutter／debug VM／log／artifactへ露出させないplatform-native LIVE_RUNTIME harnessで、同じDesktop Rust BrokerへのTLS接続・拒否・失効・background停止とWorkspace識別子の限定開示を検証する。Android実機は凍結解除後に、iOSを含む物理端末証拠を別途取得する。in-place Android buildが必要な場合だけACL管理者の判断を得る。端末内回復記録はDesktop Broker AuditEventと区別する。
  blocks_release: yes

技術接続の一次資料: [rustlsのserver設定](https://docs.rs/rustls/latest/rustls/server/struct.ServerConfig.html)。これは暗号化機構のAPI資料でありGUI Shellの権限源ではない。

## Rustの実接続と操作

`broker-server` に `--owner-session-file <owner資格file> --mobile-bind <private IPv4:port>` を指定する。未指定時は従来のloopbackだけを公開する。端末TLSではrustls、起動世代ごとの証明書生成にはrcgen、秘密hash比較にはsubtleを使う。独自暗号やPython runtime依存を追加しない。TLS early dataとticket再開を無効にし、各要求で端末資格を再検査する。

owner CLIは `対話承認操作 --session-file <owner資格file> 端末招待 <端末ID> <接続先Host> <新規招待file>`、`端末一覧`、`端末招待取消 <招待ID>`、`端末失効 <結合ID>`。招待秘密を標準出力へ表示せず新規fileへ保存する。既存fileを上書きしない。通常UIはowner資格を読まない。

`python tooling/minidora_live_check.py --reference <固定参照clone> --mobile-client --dart-client` は実TLSから実MINIDORAへの経路を実行する。テストのPython clientは開発専用であり、Mobile製品client・安全保管・実機lifecycleの証拠にはしない。

## 廃止済みのMobile live integration harness

以前のSimulator/仮想端末driverは、招待JSONをhost processからFlutter debug VM extensionを介してDart integration testへ渡していた。これはrev2の「招待・資格をDartへ渡さない」という境界と整合しないため、driver、test、`--mobile-simulator`、`--android-emulator`、`--dart-mobile-client`の経路を廃止した。履歴上の過去PASSは過去のDart client／保管test実行記録であり、現行native経路の証拠ではない。

再導入する場合は、招待入力からnative保管・TLSまで秘密をFlutter/Dart、debug VM extension、shell argument、log、test artifactへ渡さないplatform-native test harnessを先に設計する。Android emulator上の起動確認およびApple workflowのbuildは補助証拠であり、実機やKeychain/Keystoreのrelease証明ではない。
