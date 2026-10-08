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

`release_blocker`: MacでのProvider／MCP注入、正式署名identity・配布、Final QA。通常Release `task_execution=unsupported`、`release_ready=false`を保持する。秘密値の表示、保管方式fallback、Windowsの保存形式migrationを本単位の便宜で追加しない。
