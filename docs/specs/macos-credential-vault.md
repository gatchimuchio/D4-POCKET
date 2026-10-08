# macOS 資格情報保管庫

## 意味と有限Acceptance

状態: IMPLEMENTING（P13 Product Build）。既存の資格情報登録・公開一覧・論理失効を、macOS Keychainと製品のnative秘密入力へ接続する。Provider／MCPへの秘密注入、Task、物理削除、正式identity、最終QAは別単位。登録済みの秘密値をFlutterへ読み戻す機能は作らない。

有限条件: native秘密入力の取消は未登録、別個Owner確認後の登録、metadata-only一覧、現在metadataに束縛したOwner確認後の論理失効、通常要求による登録・秘密読出し・承認注入の拒否、秘密値の画面・応答・Audit非露出、通常終了。正常経路一回と対象境界試験で閉じ、CLOSED済みCLI／Workspace／Mobile条件を再試験しない。

## 資格情報保存先

資格情報保存先は、登録・保管対象の点検・許可済み内部読取・新規登録失敗時の回収だけを担う。Permission、Approval、用途・相手への委譲可否、Auditは既存Brokerが所有する。Windows DPAPIの保存形式・検査・権限は変更しない。macOSの保管方式は`macos_keychain`で、既存receiptの`暗号文hash`を平文やKeychain参照のhashへ読み替えない。

macOSでは既存ringのAES-256-GCMで秘密値を暗号化し、鍵と暗号文を別のKeychain itemに保存する。暗号文へversion・nonceを格納し、名前空間／資格情報IDを認証対象へ結合する。用途／接続対象と実暗号文hashは既存Audit recordへ一体で結合する。公開receiptは実暗号文のhashだけを返す。KeychainのserviceはRustが検証したBroker storeごとの名前空間、accountは固定の資格情報IDと鍵／暗号文の区別へ限定する。外部payloadからservice、access group、保管先を受け付けず、他App／Audit storeのitemを探索しない。

OS呼出しは独立Rust部品に限定する。[Apple SecItem API](https://developer.apple.com/documentation/security/adding-a-password-to-the-keychain)でmacOSの現在user default Keychainを明示選択する。追加はそのKeychainだけ、読取・点検・回収も同じdefault一つをsearch listに固定し、任意Keychain path・全search list・任意Appのitemを対象にしない。[SecAccessCreate](https://developer.apple.com/documentation/security/secaccesscreate(_:_:_:))のtrustedlist=nilで、秘密の読取は呼出し元Appだけへ限定する。同期なし・追加専用・重複拒否・無言fallbackなし。Broker専用processのKeychain UIを許可せず、locked／ACL拒否は固定分類でfail-closedとし、通常のD4 native Owner確認を代替しない。平文、鍵、OSの詳細errorをlogへ出さない。新規登録の途中失敗は今回作成したitemだけを回収し、回収失敗はRecoveryが必要な未成立状態にする。

選択修正の理由: run `37719156673`／source `1c519d0`でData Protection Keychainが`署名identity未成立`を返した。Apple TN3137上、この保存先にはprofile認可されたentitlementとapp-like構造が必要だが、現行製品のBrokerは独立CLIである。正式署名・profileを機能開発の条件にした最初の選択を撤回し、CLIにも対応するuser Keychainと呼出し元App ACLを固定採用する。これは試験だけのalternate backendや実行時fallbackではなく、現行Mac製品の明示保存契約である。`ThisDeviceOnly`を主張せず、Windows DPAPI・Authority・Owner確認・Audit・秘密非公開は変更しない。安定した正式signerと更新後のKeychain ACL継続は既存署名／配布Release Gateへ残す。

## 製品経路とAuthority

macOS資格情報native入力要求は`資格情報登録`の公開payloadであり、`版`・`資格情報ID`・`用途`・`接続対象`・`種類`だけを含む。公開Schemaは`specs/macos_credential_input.schema.json`。秘密値・Owner資格・保管先は公開要求に含めない。固定Rust UIが既存匿名pipeへ書く`native_credential_input`はRunnerが公開入力から拒否し、子Rustが元要求hash・nonce・期限を照合する。既存の完全登録payloadはこの照合と別個Owner確認を通った場合だけ内部生成する。

Flutterは非秘密metadataだけを送る。通常GUI親processの固定Rust UIが秘密入力を受け、既存helperへのprivate pipeだけに渡す。Swift／Dartへ秘密値を返さず、公開frameからprivate入力を拒否する。秘密入力はOwner承認ではない。子Rust helperの別個期限付きOwner確認後、現在の要求・hash・IDを再評価して既存Brokerのprocess内receiverへ配送する。既存の登録・Audit確定・一覧・失効を再利用し、別Authority経路や資格探索を作らない。

作用対応: Capability=資格情報の登録／metadata一覧／論理失効、Permission=Broker固定Keychainの対象だけ、Approval=対象metadataに束縛したnative Owner確認、Audit=公開metadataと実暗号文hash、Recovery=入力取消・拒否は未登録、部分登録は今回分のみ回収して未成立を保持。論理失効は取消不可、秘密注入を許可しない。

## 検証範囲

ローカルは既存Windows保存先の直接依存回帰、新公開payload・秘密／Authority注入拒否、Widget投影を対象とする。手動Actions `macos_credential_vault`でKeychain実API、通常Mac build、製品native入力・別個確認・一覧・失効・終了を検証する。合成秘密値はnative試験内で生成し、CLI引数・環境変数・XCTest入力log・動画・artifactへ渡さない。

native入力は16 KiB・300秒以内、別個Owner確認も300秒以内に限定し、元要求のBroker freshness windowは更新しない。合成native入力を用いる`credential-ui-fixture`はDebug専用で、通常buildには含めない。

検証履歴: 初回手動run `37716975012`／source `c5c0f7b`は新しいMac dependencyのlock更新漏れでbuild前FAIL。Keychain実APIの成功・失敗の証拠ではない。lockを更新して同一検証branchで再試験する。

run `37717772590`／source `a28625c`はNSAlertのcallback型がOptionである点の不一致でcompile FAIL。OS動作の失敗とは扱わず、実際の固定bindingに合わせて局所修正する。

run `37718076742`／source `a07db57`はBrokerが要求するDebug trait不足でcompile FAIL。保存先のDebugには固定部品名だけを返し、item・秘密・鍵を出さない。ローカル新Dart 3試験はplatform override復元時点のtest-only不一致を修正してPASS。通常checkoutの両Flutter analyzeは既存LSP FormatException／server exit 255、ASCII解析もDart perf fileのOS error 1920で未成立。Windows全Rustは556 PASS／1 FAIL／13 ignored（A2A、変更外）で、全体PASSへ読み替えない。

run `37718454089`／source `8b91a57`はMac compile成立後、Keychain追加実APIでFAIL。秘密を出さない固定OS拒否分類を追加して根因を特定する。Data Protection Keychainは[Apple TN3137](https://developer.apple.com/documentation/technotes/tn3137-on-mac-keychains)上、provisioning profileで認可された署名entitlementとapp-like構造を必要とする。現在の同梱helperは独立CLIであるため、実OS分類を確認してこの実装選択と現行構造の適合を再評価する。無署名を成功へ昇格したり、fake entitlementやOS保護の変更で回避したりしない。

run `37719651894`／source `a5ff90f`は固定user Keychainの実API試験1件、native公開境界1件、新Dart解析・3試験がPASS。通常Mac buildでSwiftのsync overloadがVoidを推論してcompile FAIL。既存OS選択と同じInt32返り値へ型を固定し、入力・Owner・Brokerの挙動は変更しない。製品経路はまだ未成立である。最終sourceのWindows全Rustは555 PASS／2 FAIL／13 ignored、変更外Update HTTPS試験のConnectionResetであり、全体PASSへ読み替えない。日本語厳格監査は既存5 files／17 findingsを保持し、本変更の新規負債は解消した。

run `37720338833`／source `8ffd3d5`は通常Mac buildまでPASS。XCUITestは初期focusでCtrl+Kが届かず、palette入力の存在確認で16.615秒のFAIL。秘密入力・Owner・登録には未到達であり、製品保存のfailureと混同しない。試験だけを既存の公開palette buttonによる通常mouse操作へ修正する。

run `37721120599`／source `d6609f2`は部品試験・解析・通常Mac buildがPASS、初期palette buttonのlabel限定queryで20.483秒のFAIL。秘密入力前であり、既存のlabel／value共通照合へ試験queryを変更し、未識別時だけ公開button名を最大40件記録する。入力値・秘密欄・全階層dumpは記録しない。

run `37722143281`／source `36c72e4`は通常Mac buildまでPASS、palette未識別で20.095秒のFAIL。限定診断により、コンパクト表示のicon buttonはAX名なし、操作groupは`操作グループ選択`として取得できることを観測した。現行Rowのpalette→全体検索→groupという固定順序と、実frameの同一行・可視・2件一致を照合して通常mouse入力する試験へ変更する。曖昧なら停止し、秘密入力前の公開button名・frameだけを記録する。GUI stateやBroker要求を直接設定しない。

run `37723289904`／source `72451dc`はpaletteの2 icon照合と通常clickが成立。検索TextFieldは実frameを持つがAX hit判定が偽となり、clickで15.189秒のFAIL。現行paletteのautofocusへ通常keyを送り、公開result／server欄は既存フォーム試験と同じ実frameのmouse操作を使う。入力内容の直接設定、秘密値の読取、Owner／Broker挙動の変更は行わない。

run `37724269791`／source `c603df8`はpaletteを開き検索keyを送るまで成立。elementTypeを含む全階層predicateの評価中にsnapshot timeoutとなり132.270秒のFAIL。原因は未確定であり、複雑predicateを除去し、ASCII `MCP`検索と既存の単純な候補label／value照合へ局所変更する。秘密入力・登録には未到達で、未成立をPASSへ昇格しない。

run `37725337534`／source `e2d4965`はASCII検索後も全階層snapshot timeoutで130.288秒のFAIL。複雑predicateだけが原因という仮定を撤回する。既存Manifest試験が扱うFlutterView背面のnative editorへ実座標clickしてから通常keyを送り、候補選択だけは既存CLI試験のVision方式を秘密入力前の公開文字に限定して使用する。画像を保存・添付・出力せず、候補が一意かつ製品窓内でなければ停止する。製品要求・秘密値・Owner確認を代替しない。

run `37726668234`／source `e046a0e`は公開MCP候補の画面照合とclickまで成立。サーバー識別子のnative editorをlabel queryで取得できず、21.633秒でFAILした。秘密入力前の公開ラベルを画面から一意に取得し、通常scroll／mouseで公開IDを入力する試験へ変更する。Keychain実API・対象境界・通常build・後片付けはPASS、製品登録は未到達であり未成立を保持する。artifact `11527987964`のSHA-256 `0a4a263ca3e42863e467c80535d84265a54f84631908ae279505b076d1cceffb`を照合した。証拠取得時の認証拒否はRepository移転のredirectで、現行repository IDへ直接要求して解消した。製品要求・Authority・有限条件は変更しない。

run `37727884223`／source `ce443b6`は公開server ID入力、Rust所有のnative秘密入力sheet表示、取消まで成立。取消後の全階層AX文言queryがsnapshot timeoutとなり153.466秒でFAILした。製品の取消応答のfailureと確定せず、資格情報状態文のqueryをStaticTextだけへ限定する。登録・失効は未到達、有限Acceptance全体を未成立のまま保持する。Keychain実API・対象境界・通常build・後片付けはPASS。artifact `11528582144`のSHA-256 `77fe8414d456594d2ee5c9b5eadfec67c54db466f3abec28a24cb8debd1ecc9a`を照合した。製品source・Authority・入力・有限条件は変更しない。

run `37728913821`／source `c5d36fd`は取消後の未成立投影、二度目のnative秘密入力、別個Owner承認まで成立。登録成功文が取得できず50.159秒でFAILした。初期時点で拒否・保存・receipt・投影のどこが原因かは未確定だったが、現行codeを追跡し、登録allowlistを通常Desktop Ownerではなくr2-e2e専用関数へ誤追加していた根因を確認した。通常Desktop OwnerだけへMac登録を限定追加し、試験専用の誤拡大を撤去する。通常資格・session／metadata注入・試験専用Ownerからの登録拒否と、実Owner受信gateへの到達を対象試験へ追加する。次の実行では既存Auditの資格情報3操作から最大64件の固定code／decisionだけを保存し、秘密値・record本文・hash・IDをartifactへ出さない。試験も固定の公開failure文の有無だけを診断する。Keychain実API・対象境界・通常build・後片付けはPASS。artifact `11529670126`のSHA-256 `9b56519e6447215780f5aa824cc722617c4382f6e23c5dcbf3873c7fd6e3d720`を照合した。有限Acceptance全体は未成立であり、保存方式・Authority・正常条件は変更しない。

通常Owner gate修正のローカル検証: 新gate 1件と資格情報直接依存7件はPASS。Windowsの`r2-e2e` feature付き新gate試験も1件PASSし、試験専用Ownerの登録拒否を確認した。Macは通常featureだけの新gate試験と製品操作を次の手動runnerで確認する。新試験の初回compileは存在しない通常処理method名でFAILし、既存`handle`へ修正した。対象試験と全体試験の同時実行では同じtest exeへのlinkがLNK1104でFAILし、全体試験終了後の対象再実行で解消した。環境保護や製品codeで回避しない。必須Windows全Rustは556 PASS／2 FAIL／13 ignored、変更外A2A応答読取failureとCodex loopback途中ConnectionReset（ignored `release_evidence/p13-macos-credential-windows-owner-fix.txt`）。既存Final QAの`release_blocker`履歴へ残し、全体PASSや根因解消へ読み替えない。Schema 166／正常162／負例213、Conformance 239、手動起動限定・Manifest・差分検査はPASS。

run `37731078599`／source `92b3bba`は新しい通常Mac Owner gate試験、実Keychain API、native公開境界・Dart 3件・通常buildがPASS。製品UIでは取消→未成立投影→native秘密入力→別個Owner承認→登録成功文まで成立。既存Auditの固定診断は取消を`credential_owner_required`、登録・一覧を`accepted`と記録した。ListTileの公開状態titleをStaticTextとして取得できず37.174秒でFAIL。状態titleを持つgroupのlabelだけへ試験を限定し、値・全階層・秘密入力を取得しない。論理失効・通常終了を含む有限Acceptance全体は未成立のまま保持する。artifact `11530361014`のSHA-256 `53347c0c6d42e8f0db8a50f7be8efa31f5954011433540111394f41767ea65a6`を照合した。helper回収PASS、製品source・Authority・保存方式は変更しない。

run `37732126321`／source `8062109`も登録・一覧が既存Auditでaccepted、UI登録成功文まで成立。状態titleのgroup queryも取得できず38.453秒でFAILし、groupとして取得可能という推定を撤回する。metadata-only成功後、native sheet／秘密入力欄の不存在を確認した場合だけ、製品窓の公開状態titleを画像認識で一意に照合する試験へ変更する。画像・認識文字を保存・添付・出力せず、秘密入力中や他Appの画面は取得しない。通常scrollによる実表示だけを使い、Broker応答・Owner・秘密値を代替しない。前段の公開入力用画像認識とは責任を分離する。artifact `11530351705`のSHA-256 `ee85c8ab3c4ceaaec2267f1027e1ffaf96a570ecca983b64ef233a81d13e5043`を照合した。helper回収PASS、論理失効・通常終了を含む全体Acceptanceは未成立。

run `37733367206`／source `3a0579b`は登録・metadata一覧と実表示の`api_key ・ 有効`の一意照合までPASS。失効iconのAX名を取得できず56.895秒でFAILした。既存失効操作を独立Semantics nodeにし、button・日本語名を明示する。Tooltipの意味重複だけを除き、既存onPressed・無効状態・対象metadata・別個Owner確認は変更しない。既存Widget失効試験でbuttonと通常tap actionを確認し、未成立の失効・終了を同じ製品試験で続行する。artifact `11531031357`のSHA-256 `12a33f0c230ccc70bc4967b099e52c13e8dd210e69843229202afb3954d9d736`を照合した。helper回収PASS、有限Acceptance全体は未成立。

失効表示の局所検証: 独立Semanticsの初回は名前とtap actionが別nodeとなり既存Widget試験でFAIL。MergeSemanticsで日本語名・button・既存tapを同じnodeに束縛した。次の試験は挙動成立後にSemanticsHandle回収時点でFAILし、試験内部のfinallyへ回収を移した。修正後、ASCII一時checkoutの既存失効Widget試験1件がPASSし、button／tap・対象metadata・一覧更新・秘密非表示を確認した（FIXTURE、installed証拠ではない）。両Flutter `analyze --no-pub`は既存LSP server exit 255でexit 1（ignored log `p13-macos-credential-semantics-desktop-analyze.txt`／`p13-macos-credential-semantics-mobile-analyze.txt`）。Manifestは検証中の局所編集で一度不一致を返し、再生成後のcheckでPASS。正常操作・Authority・公開状態確認条件を削除しない。

run `37735176741`／source `2a421cc`は対象Dart解析・4 Widget試験、通常build、公開登録・有効状態がPASSし、失効buttonのAX名も取得できた。AX frameが高さ1pxのままのため、可視確認で42.359秒のFAIL。既存失効buttonを名前が読めるTextButtonへ変更し、秘密入力の終了後だけ、製品窓の公開状態と同じ行にある日本語操作名を一意に照合して通常mouse clickする。AX名の存在確認・状態確認・別個Owner確認・失効結果を保持し、1px frameをclick座標へ使わない。画像・認識文字を保存・添付・出力せず、GUI state／Broker応答／Ownerを直接設定しない。artifact `11531049345`のSHA-256 `4407122315b532c7f93ff367eaaf03b9caf0b14588ed3c97bb36fd4511a8ddaf`を照合した。後片付けPASS、有限Acceptance全体は未成立。

run `37736125289`／source `0f09323`は通常build・対象解析／試験・登録／一覧がPASS。Window screenshotへ変更した公開状態／操作の同時照合が未成立で69.679秒のFAILとなった。状態自体か操作文字か撮影対象のどこが原因かは未確定。先に状態照合が成立したApp screenshotへ戻し、通常activateで製品を前面に置く。秘密入力終了後の状態／操作の件数とwindow／image geometryだけを最大8件記録し、文字・ID・入力値・画像を出さない。artifact `11532460461`のSHA-256 `eef80d7f5274b0b1d6f47eeb14eb4de9c57d915b725fc0521ad02fbc126710e9`を照合した。helper回収PASS、失効・終了を未成立のまま保持する。

run `37737228953`／source `9d1a730`は登録・一覧accepted、公開有効状態は試行2〜4で一意に認識したが、失効操作文字は8回とも0件で95.570秒のFAIL。App screenshotは1024×768、製品窓は(112,51,800,628)であり、App画像を窓だけと扱った仮定を撤回する。秘密入力終了後の実window boundsへ画像認識を切り取り、clickを同じ座標系へ束縛する。固定公開ラベルの空白・改行だけを正本化し、診断は件数・geometryと固定3断片の一致件数に限る。画像・認識文字・ID・秘密値を保存・出力しない。artifact `11532860512`のSHA-256 `7ae8caac870c9cc37b00ba6eaea23019767b2bc3c3352250a4c78504736f60d0`を照合した。helper回収PASS、失効・通常終了を含む有限Acceptanceは未成立であり、別fixture・追加安全条件へ拡張しない。

run `37740859766`／source `1fe31c5`は窓内800×628の切り取りと公開有効状態の照合が成立したが、操作文字／固定断片は0件で77.725秒のFAIL。登録・一覧・対象試験・通常build・helper回収はPASS。artifact `11533539696`、SHA-256 `023fb009be8e317cede27aeaaf60789e4bbb6b9d820e9cc40d98a346765841b2`を照合した。失効操作の根因は未確定で、窓の切り取りだけでは解消しなかった。

現在FAILの診断に限り、秘密入力欄／sheet不存在と既存Brokerのmetadata-only成功を確認した後、認識済み公開metadata行の上下32pxだけを一枚保存する。最大256 KiB、画面全体・他App・native秘密入力・xcresultの全attachments／映像をexportしない。公開資格情報IDは秘密値ではなく、Brokerが表示許可したmetadataの範囲に限る。従来の画像非保存方針のこの診断だけを明示変更する。推測的fixture追加ではなく現在の操作未成立の原因確認であり、原因解消時に診断出力を撤去する。操作の認識は[Appleの文字範囲box API](https://developer.apple.com/documentation/vision/vnrecognizedtext/boundingbox(for:))で固定操作文字だけの位置へ束縛し、対象状態の右・同じ行・一意性を維持する。画像から秘密値・承認を生成せず、通常mouse・native Owner・Brokerの製品経路は変更しない。

`release_blocker`: MacでのProvider／MCP注入、正式署名identity・配布、Final QA。通常Release `task_execution=unsupported`、`release_ready=false`を保持する。秘密値の表示、保管方式fallback、Windowsの保存形式migrationを本単位の便宜で追加しない。
