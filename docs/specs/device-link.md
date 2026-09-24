# 端末連携の意味正本

状態: Rustの暗号化端末経路とowner制御、既存Mobile client・保管・lifecycleの接続は実装済み。現行Dart製品経路は資格とTLS/networkをFlutter側で扱うため、D4 Pocket rev2のFlutter禁止境界には未適合であり、native transport／保管への移行が必要。過去のDart TLS実行証拠を現在の適合証拠へ読み替えない。契約試験はFIXTUREであり、Android/iOSの安全保管や端末実機の証拠とは区別する。

## 対象と責任

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

復帰・手動再確認では安全保管の端末IDと資格を再読取し、構造・期限と現在の結合内容を照合してから通信する。削除・破損・変更・読取障害時にメモリ上の旧資格を代用せず、変更された資格を自動採用しない。読取や接続確認の途中でbackgroundに移った確認結果を、後の復帰の接続成功に転用しない。

通常の解除では有効なclientから端末離脱の応答を確認し、client不在を解除成功にしない。保存資格の削除後は同じ保管経路で再読取し、不在を確認してから端末内の削除完了を表示する。資格が残る場合や再読取に失敗した場合は通信停止を維持し、削除未確認として再操作を案内する。これは保管APIを通した観測であり、媒体上の物理消去や別processによる後続の再書込み防止を証明しない。

既存の概要・確認・通知・実行系・停止・復旧画面は維持する。接続前の固定preview値を実Runtime状態として表示せず、未接続を明示する。未実装の承認や停止操作を成功表示しない。

## D4 Pocket rev2のFlutter／native境界

Flutterは接続状態と許可済みprojectionだけを表示する。招待JSONをDart TextFieldへ入力しない。native側が招待入力・Host／HostID／証明書hashの確認・結合確認を行い、成功時はnative側で招待秘密を破棄して結合資格をOS安全保管へ保存する。Flutterには端末IDと資格を含まない接続状態projectionだけを返す。

Flutterからnativeへのchannel要求は、版、固定method名、必要な場合の既存Device Link operationとその業務payloadだけから成る。pairing要求は引数なしであり、招待・資格・secret・token・session credentialをchannel越しに渡さない。nativeの通常要求経路は既存Device Link TLSの宛先へ接続し、既存Rust Brokerの資格検査・nonce・allowlist・所有関係・Audit・Approval・Recoveryへ必ず到達する。native側に権限判断、Owner操作、任意host／URL、任意commandを追加しない。

AndroidはOS Keystore保護のnative暗号化保管、iOSはThisDeviceOnly Keychain保管を使う。OS保管が利用できない場合はfail-closedとし、Dart保管・平文保存・backup復元へfallbackしない。TLS証明書hashと有効期間、有限timeout、bounded frame、background中のsocket停止、要求一回限りの扱いを既存契約から弱めない。native応答はoperationごとに検証し、資格field／資格実値を再帰的に除去または拒否してからFlutterへ渡す。例外・system log・test artifactにも秘密値を含めない。

このchannelはMobile製品内のnative transport／保管境界であり、Rust Brokerを迂回する別bridgeではない。Schema・fixture・静的conformanceは契約形状の証拠に限る。実Device Link、Android/iOS OS保管、TLS、background停止のLIVE_RUNTIME証拠とは区別する。

## 受入試験と残存境界

Schemaとconformanceは招待・保管資格・要求の構造と禁止操作を検査する。Rustの実経路では正常結合、再接続、招待取消、端末失効、期限切れ、不正資格、nonce再使用、Host不一致、他端末の所有物操作、owner昇格、監査障害を検査する。TLS実接続、異なる証明書拒否、Mobile安全保管、MINIDORA実対話、lifecycleは別のLIVE_RUNTIME証拠を必要とする。

- item: 端末連携の実装と実機検証
  classification: release_blocker
  reason: 契約定義とfixtureは製品接続の成立を証明しない。
  required_action: Rust・Flutterの消費経路と否定経路を実装し、Android/iOSのbuild・install・launch・結合・対話・lifecycleを測定する。
  blocks_release: yes

技術接続の一次資料: [rustlsのserver設定](https://docs.rs/rustls/latest/rustls/server/struct.ServerConfig.html)、[DartのSecureSocket](https://api.dart.dev/dart-io/SecureSocket/connect.html)。これらは暗号化機構のAPI資料でありGUI Shellの権限源ではない。

## Rustの実接続と操作

`broker-server` に `--owner-session-file <owner資格file> --mobile-bind <private IPv4:port>` を指定する。未指定時は従来のloopbackだけを公開する。端末TLSではrustls、起動世代ごとの証明書生成にはrcgen、秘密hash比較にはsubtleを使う。独自暗号やPython runtime依存を追加しない。TLS early dataとticket再開を無効にし、各要求で端末資格を再検査する。

owner CLIは `対話承認操作 --session-file <owner資格file> 端末招待 <端末ID> <接続先Host> <新規招待file>`、`端末一覧`、`端末招待取消 <招待ID>`、`端末失効 <結合ID>`。招待秘密を標準出力へ表示せず新規fileへ保存する。既存fileを上書きしない。通常UIはowner資格を読まない。

`python tooling/minidora_live_check.py --reference <固定参照clone> --mobile-client --dart-client` は実TLSから実MINIDORAへの経路を実行する。テストのPython clientは開発専用であり、Mobile製品client・安全保管・実機lifecycleの証拠にはしない。

## 仮想端末でのnative保管検証

`apps/mobile_flutter/integration_test/native_store_test.dart` はdevelopment専用で、製品のSecureDeviceStoreを実行する。試験ごとのprefixだけを付け、既存端末IDや実資格を読取・上書きしない。非秘密の試験値だけを保存し、終了時に削除を再読取で確認する。平文fallbackやMethodChannel置換を行わない。

別instanceからの読取・更新・削除、端末IDのcontroller再生成後の一致、資格なし・破損資格の接続拒否を検査する。前景切替はcontroller呼出しによるINTERNAL_STATE検査であり、OSのbackground遷移やprocess再起動の証明ではない。保管APIの実行結果は仮想端末上のLIVE_RUNTIME証拠に限定し、実機Keychain保護、端末lock、媒体消去、実TLS再接続へ昇格しない。残る実機・実TLS・OS lifecycleの検証は上記release_blockerに保持する。

## native保管と実TLSを通すSimulator統合

開発専用の `tooling/minidora_live_check.py --mobile-simulator <UDID>` は、固定参照MINIDORAの二実API・一時Rust broker・iOS Simulatorを接続する。通常の製品起動には追加しない。招待は一時試験資格だけを使用し、host側driverからFlutterの認証付きdebug VM接続へ渡す。秘密をdart-define、ソース、一般設定、log、成果物へ埋め込まない。owner制御資格はhostの既存試験harnessだけが保持し、Mobileへ渡さない。

native安全保管後のcontroller再生成・実TLS再確認、OSによる背景移動と復帰、同じ要求の応答照会、保存資格消失時の通信停止、正常な端末離脱とnative削除を検査する。OS遷移は専用Simulator上で設定appを前景化し、元appへ戻す。MobileHomeの実WidgetsBindingObserverとcontrollerの変化を観測する。実APIの応答は既存の一時owner検証経路で当該試験要求だけを承認する。これはSimulator内のLIVE_RUNTIME証拠であり、実機のlock・OS強制終了・物理Keychain保護・正式配布の証拠ではない。

driverの拡張はintegration_test内だけに登録し、開発デバッグ資格以上の製品権限を付与しない。試験には起動・応答・全体の期限を設ける。失敗時も一時processを停止し、一時資格を除去する。記録は対象commit、試験状態、要求ID、時刻、非秘密の遷移事実に限定する。
