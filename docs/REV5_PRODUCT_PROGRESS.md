# D4 Pocket / GUI-Shell rev5 Product-First 進捗

更新日: 2026-10-08
工程正本: ユーザー提示「D4 Pocket / GUI-Shell 統合実装仕様書 rev5」「統合開発工程表 rev5」「Codex実装指示書 rev5」
現行mode: `PRODUCT_BUILD_MODE`（最新版rev5実装指示§2）。Windows側P12 `FEATURE COMPLETE`はCLOSEDのまま保持し、Mobile／Non-Windowsは独立するP13 trackとして構築する。P2 CompareとP3 HandoffもCLOSEDのまま再訪しない。Q0／Q1／Q2に記録済みのQA実行結果は履歴証拠として保持するが、Final QA queueは現在の作業schedulerではなく、追加QAは開始・継続しない。通常Release `task_execution=unsupported`と既存release gateを維持する。
初期rev5文書同期commit: `39dd3f4bc7aafe350ca94fce9392095f1064d2bc`。凍結commitとその後のQA記録は本書末尾の更新履歴を参照。

## 正本の選び方

常にユーザーが現在提示した最新版の仕様書・工程表・実装指示書と、そこへ同期したリポジトリ内の現行進捗を正本とする。旧版文書は、明示的に現行正本へ採用されない限り、履歴・補助証拠としてのみ使う。旧版の状態や要求を現行状態へ推定転記しない。

## 現行rev5指示による工程整理（2026-10-07）

最新版rev5実装指示§2が現在modeを`PRODUCT_BUILD_MODE`と明示しているため、P13 Mobile／Non-Windowsの製品構築を続ける。§23のFinal QA移行条件は、現行の明示modeを自己判断で上書きする許可ではない。P12 Windows Feature Complete、P2／P3のCLOSED状態、Q0／Q1の結果、Q2に蓄積した試験結果と過去FAILは変更せず、Final QAは別の現行指示があるまで延期する。

旧進捗文書に残る「P12 Feature Complete後にFinal QAへ移行済み」という記載は、その時点の判断履歴として保持するが、最新版の明示modeに反する現在状態の根拠にはしない。工程表P2 Compareは実装指示書R3、P3 HandoffはR4に対応し、両者はCLOSEDのため再実装・再検査しない。通常Release `task_execution`はfail-closedのまま維持する。

## P13 Mobile / Non-Windows — 現行track

#### macOS 作業領域のOS選択 — IMPLEMENTING（Product Build、2026-10-08）

手動Actions [run 37693823144](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37693823144)、commit `4d13e3d7b80ff999eac2f3172d54f5c26ec4f75e`はUI試験24.822秒でclick前の文字認識がFAIL。固定native診断は未到達で、終了根因は未確定。artifact `11514184088`、SHA-256 `0642ddf51eff0300b65b99cf234c9388ea25859c74ff06df97673e9988e8d47d`を照合した。保存videoの21秒frameは選択buttonを製品窓内に示すため、最初の診断clickだけを観測済み800px窓のbutton中央へ通常XCTest入力で送る。取消後のlayoutには適用せず、原因取得後に撤去する。通常build／対象Rust／Dart／後片付けはPASS、Acceptanceとmainは未変更。

手動Actions [run 37691846258](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37691846258)、commit `07ab9ec115ba75819666215c201a21af4f0a1bfb`は一時診断のSwift buildでFAIL。固定SDKでは`FileHandle.read(upToCount:)`がmacOS 10.15.4以降なのに、既存deployment target 10.15の可用性guardが欠落していた。診断だけを公開APIの可用性guard内に限定し、製品の最低OSやfilesystem／sandbox policyを変更しない。UIは未実行でnative終了段階は未取得。通常build／対象Rust／Dart／後片付けはPASS。artifact `11513548353`、SHA-256 `f390c53251463c262231b7775534f53aad8f465dce8b981ccf3dbe955f0eb07f`を照合・保存した。

一時診断追加後のWindows必須Rust全体は503 passed／6 failed／12 ignoredでFAIL。変更外のA2A読取1件、既存Update Download TLS 4件のOS 10054に加え、MCP stdio fixtureのdirectory削除にOS 32（他process使用中）を観測した。Mac専用の診断featureは既定無効で、当該Windows作用経路は変更していない。`release_blocker`である既存Final QAへ失敗履歴を送るが、fixture lockの根因解消・製品regression確定・全体PASSはいずれも主張しない。独立native crateのWindows入力拒否1件はPASS。

手動Actions [run 37689830310](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37689830310)、commit `303a6d12aab1fff2c905d90d9dbe3db8302b9a14`はUI試験50.828秒でFAIL。文字認識後の実clickまで進み、transportは`no_valid_reply`、helperはその後終了（NSRunningApplication PID=-1）、選択Auditは0件だった。認識失敗とは別のnative終了で、AppKit起動補足が解決したとは主張しない。artifact `11512996989`、SHA-256 `e7fe0b5020eeea5af47cad3ef3abd26626888ea163678bad4b8318f87df3efab`を照合・保存した。通常build／対象Rust／Dart／後片付けはPASS。開発専用`workspace-ui-diagnostic`をUI試験だけで使い、helper stderrをmemory内で固定段階・既知例外名だけへ射影する。raw stderr、入力、資格、pathは保存・log・artifactへ出さず、起動／panel／modalのどこで終了するかを局所化する。通常製品buildの成功とは分離し、原因確定後は一時featureと診断を撤去して通常helperで有限Acceptanceを検証する。

手動Actions [run 37688120883](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37688120883)、commit `7ba96736a85121b97402b3165224a21766a99bcb`もUI試験24.037秒、click前の文字認識でFAIL。先の「途中の漢字が欠落した」という説明は認識結果を未取得の推定で、原因確定ではなかった。識別子だけの再変更を続けず、Appleの公開APIで言語補正／製品の既知ラベル語彙を有効化し、再失敗時は合成画面のボタン行だけの認識結果を観測する。入力欄・画面全体・任意errorを診断へ追加せず、原因確定後に一時診断を撤去する。artifact `11511754157`、SHA-256 `29cec9f3b33cd9f7e7033237e0f4d3fdc15edf97c3b85e54278a5a6c8fbacbf1`を照合・保存した。対象Rust／Dart／通常build／後片付けはPASS。native起動修正と有限Acceptanceは未成立のまま。

手動Actions [run 37686729916](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37686729916)、commit `3eb60e35fab26e5d25d355526232ea42a938d7fc`はUI試験24.952秒でFAIL。保存videoの22秒frameにはOS選択buttonが可視だが、Visionの途中の漢字全一致条件でclick前に失敗した。試験だけを固有接頭ラベル「OSで」の一意な可視文字へ束縛し、製品・Authority・Acceptanceを変更しない。native起動修正の成否は未取得。artifact `11511962024`、SHA-256 `1604925c90fecb5019a1bee54e1ce77f724cd1c0b89f2facc4e3fe49469198e8`を照合・保存した。対象Rust／Dart／通常build／後片付けはPASS。Windows Desktop／Mobileの必須`flutter analyze --no-pub`は既知のLSP入力FormatException／server exit 255でexit 1。厳格日本語監査は変更外5 files／17 findingsでFAIL。各全体FAILは履歴・Final QAへ保持し、対象Mac製品PASSへ混同しない。

手動Actions [run 37684486166](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37684486166)、commit `17d858e2d5ea069dcdf1d3e13f8036f2039a8673`はUI試験41.015秒でFAIL。既存Audit fileの当該操作抽出は0件で、Brokerの選択Audit確定まで到達していない。nativeの固定失敗codeは取得できず、失敗原因を断定しない。artifact `11511250307`、SHA-256 `5e0ec03f7b746b013a8bbaa10b633018bcc6d20a6c85c36a4129b15171a15ac6`を照合・保存した。AppKit通常起動を通らないhelperの初回起動完了／表示activationを補い、資格が拒否された直接AX操作を公開画面認識／XCTest通常入力へ置換する。明示Debug試験flagだけで固定応答分類とhelper終了値を観測し、原因確定後に撤去する。境界・有限Acceptanceは変更せず、次の製品試験が成立するまでmainへ統合しない。

現在のSchema／正常例／負例は163／159／210 PASS、対象Rust 3件PASS。以下の全体FAILは履歴として保持する。

AppKit起動修正後のWindows必須Rust全体は505 passed／4 failed／12 ignoredでFAIL。変更外のA2A loopback 2件と既存Update Download TLS 2件に応答読取／timeout／OS 10054が残る。既存`FQ-TEST-LOOPBACK`へ保持し、全体PASSとはしない。独立native crateのWindows入力拒否1件はPASSで、AppKit実動作はMac専用試験へ限定する。

手動Actions [run 37671678735](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37671678735)、commit `06cd2257a688424c23de227775c585bf17a9978d`は新規native crateの型検査でFAIL。固定bindingの`NSApplicationActivationPolicy`が`NSRunningApplication` featureに束縛されており、NSSavePanelのtitle／prompt／message引数は`Option<&NSString>`だった。必要なfeatureとnullable引数だけを修正する。Mac実動作の成功は未取得、mainは変更前を保持し、runner後片付けはPASS。

手動Actions [run 37672536704](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37672536704)、commit `a3d4c0fcb32a7ee7e063edf5bfb17ae192a3cd20`はMac対象Rust 3件とhelper buildがPASS。Dart型errorはなく、変更外のdeprecated info 5件と新規catchのbrace info 1件を観測し、後者だけを修正した。待機時間testもPASSした。投影testのassertion後、手動platform overrideがFlutter bindingの終了不変条件に反してFAILしたため、標準`TargetPlatformVariant`へ修正する。製品build／OS UIは未実行、runner source cleanとfixture回収はPASS。Product authorityの変更や検査の弱体化は行わない。

手動Actions [run 37673927495](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37673927495)、commit `ffc133c81e66318d3a8833f2c6ab6da38969ef3f`はMac対象Rust 3件、helper build、変更Dart解析、待機時間／投影の2試験、通常Mac buildがPASS。解析は既存deprecated info 5件と新規不要import info 1件で、後者だけを削除した。UI試験は起動前にOS XPCサービスを通常appとして解決できずFAILした。試験専用の公開AX APIで固定同梱helper executableのPIDへ束縛し、実OS panelのボタン／folder入力を操作する。Broker入力／返答やOS選択結果を注入せず、製品の境界は変更しない。runner後片付けとfixture回収はPASS、実OS選択の成立はまだ未取得。

手動Actions [run 37676368839](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37676368839)、commit `b10ebd229de797324fee2053e3fb8f3a12a732d3`はUI試験40.523秒でFAIL。同梱helperのPIDは取得したがOS panelを観測できなかった。保存画面は秘密path欄のcaretとenabledの選択buttonを示し、試験のTab／Spaceはbuttonを開始していなかった。画面内の実buttonを直接clickするよう局所修正する。Mac対象Rust／Dart／通常buildと後片付けはPASS。artifact `11506364641`のSHA-256 `d1cf0bc59207635f5a655811742bb2cd4dc3a808a3e42e8863a124b5646a58a4`を照合しignored領域へ保存した。OS選択の成立へ読み替えず、mainは変更前を保持する。

手動Actions [run 37677877954](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37677877954)、commit `0c3d9723eae43706ac2f0799a7a07a5f30d4b37b`はUI試験36.622秒でFAIL。スクロール後も選択buttonのAX frameが高さ1pxのままで、click前の試験可視性assertionに失敗した。合成公開入力だけの画面をApple Visionで認識し、一意なOS選択buttonの可視文字中央を通常clickする試験専用処理へ修正する。画面文字・座標以外を製品へ渡さず、Broker／Authority境界は変更しない。対象Rust／Dart／通常buildと後片付けはPASS。実OS選択はまだ未到達で、Acceptanceとmainは未変更。

手動Actions [run 37679355859](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37679355859)、commit `c752d85371ca934e6deaa9c6a69a09041d962f3f`はUI試験24.760秒でFAIL。保存videoの20秒frameで、最終欄入力後の選択buttonがviewport外にあることを確認した。通常本文scrollを一回行ってから既存の画面認識／clickへ進める。対象build／試験と後片付けはPASS。artifact `11509007869`、SHA-256 `0a98019a3cb5d32f296e51b394215e9c49cfb42c8437719bade60aac52c3d2d9`を照合してignored領域へ保存した。Broker・Owner処理は変更せず、OS選択の成立はまだ未取得。

手動Actions [run 37680593751](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37680593751)、commit `13cc445fbee173dda69ce1cdbf841e7e777c8057`はUI試験45.071秒でFAIL。画面認識後のclickは実button中央へ到達したが、画面は「OS選択が成立していません」を返した。直接AXはtrusted=false／CannotCompleteで、その結果だけからchooser内部を断定しない。nativeの固定失敗分類を既存Broker拒否codeへ結合し、秘密・path・raw errorを露出せず原因を区別する。artifact `11507844827`、SHA-256 `618c9b469af0f878a6d6b1d36201bac74f4a70d0721e62435585dbf43da6922b`を照合・保存した。通常build／対象試験／後片付けはPASS。実OS選択の有限条件は未成立で、mainは変更前のまま。

固定失敗分類の局所修正後、対象Rust 3件とSchema 163／正常例159／負例210、Conformance 237件がPASS。必須Rust全体は505 passed／4 failed／12 ignoredでFAIL。今回の失敗は変更外のMINIDORA Content-Length読取、Codex loopback応答header読取、bounded catalog上限、既存TLS repairで、通信失敗／ConnectionReset／OS 10054を観測した。既存`FQ-TEST-LOOPBACK`と同分類へ保持し、全体PASSや根因解消とは扱わない。対象Mac操作はWindowsのこれらの通信経路を変更しない。

手動Actions [run 37682694157](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37682694157)、commit `451ce6ee7c9c0ef12083d2b928f5b13f9ae65fa3`はUI試験43.732秒でFAIL。親app AXがDisabledのため投影codeも読めず、native failureか試験の観測制約かをまだ断定しない。手動runnerの既存Audit fileから当該操作の判定とallowlist固定codeだけを取得する診断を追加する。秘密鍵・資格・path・raw contentを取得せず、chain／HMAC再検算や完成証拠には使わない。対象Rust／変更Dartの2試験／通常build、source cleanとfixture回収はPASS。artifact `11510581633`のSHA-256 `3644e44582766cd441490b92509330c0c07adf9b697f6c27ead510d6f7b57ebd`を照合・保存した。有限Acceptanceとmainは未変更。

有限Acceptanceは`docs/specs/macos-workspace-selection.md`。取消時の入力保持、実OS chooserによるsandbox外folder選択／投影、別個のOwner確認後の既存Broker登録、UI path／Authority注入拒否を一単位で施工する。選択はOS user-selected accessであり、D4のPermission／Approval／登録ではない。手動Actionsの`macos_workspace_selection`だけでMacを検証し、CLOSEDの登録／Adapter／Mobile試験は選択しない。sandbox外CLI実行・restart access・Credential・正式配布・Final QAはこの単位へ追加しない。

ローカル検証: Schema 162／正常例158／負例209、Conformance 237 checks PASS。必須`cargo test --locked --manifest-path native/rust_helper/Cargo.toml`は505 passed／4 failed／12 ignoredでFAIL。変更外のA2A loopback読取と3つの既存HTTPS fixtureが応答読取／OS 10054／InvalidContentTypeで失敗した。局所切分けではA2A 1 PASS、update_download群16 PASS／1 FAILで、`failed_replacement_keeps_the_existing_corrupt_package_unchanged`の既存TLS失敗が残る。対象新規試験の成功や全体PASSへ読み替えず、既存Final QAのloopback検査課題へ保持する。変更がMac専用OS選択へ限定され、これらの作用経路を変更していないことを差分で確認した。

Desktop／Mobileの必須`flutter analyze`は両方exit 1。OneDrive上の既存ephemeral `Packages/.packages`をFlutterが削除できず解析を開始できなかった。回避のため製品sandbox／filesystemを変更せず、変更Dartの解析・投影試験・通常Mac buildを手動Mac runnerで実行する。release gateとdiff検査はPASS。strict日本語監査の変更外の指摘は保持し、新規文書の英語link labelだけを日本語へ修正した。正式配布／最終QAは既存`release_blocker`のまま。

#### macOS Agent CLI／Workspace起動中登録 — CLOSED（Product Build、2026-10-08）

有限Acceptanceはcommit `c2ef88d6f11babdb6846ed332003c50dd6adc2f0`の手動Actions [run 37664507377](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37664507377)でPASS。macOS 15.7.9 arm64／Apple Virtual Machine／Xcode 16.4、固定Flutter 3.44.0／Rust 1.95.0。製品Agent Centerで6欄を入力・照合し、Rust所有OS確認の拒否時は登録されず、承認後は既存Brokerが実Codex CLI 0.159.2の固定probeとAPFS Workspace検査を行い、登録recordと`Task実行能力: unsupported`を表示した。製品XCUITest 1 passed／0 failed／0 skipped、44.570秒。Command-Q終了、helper残留なし、runner source clean、専用CLI／空Workspace回収もPASS。

同runの実行commandは`cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --lib macos_desktop_worker::tests::owner_agent_registration_ -- --test-threads=1`（1件）、helper build、変更Dart 2 file解析（No issues）、`flutter test --no-pub test/broker_client_payload_hash_test.dart`（7件）、試験表示flagを含まない通常`flutter build macos --debug --no-pub`、`xcodebuild test ... -only-testing:RunnerUITests/AdapterOwnerUITests/testProductAgentRegistration`。XCTestだけは既存`D4_MACOS_OWNER_UI_TEST`表示flag付きであり、通常Release全体や正式署名の証拠へ昇格しない。OS確認・登録は限定`LIVE_RUNTIME`、Rust Owner callback／合成CLIの否定・Audit検査は`FIXTURE`として区別する。

実CodexのSHA-256は`16593cc2f422d5f398a8e40f550ebbaf1245392528957be342c295920a300704`。artifact `11503460098`（76960 bytes、remote期限2026-10-10T18:13:53Z）をignored `release_evidence/p13-macos-registration-c2ef88d.zip`へ保存し、SHA-256 `d4b8689c90ad780f2a38fddefec8a6eae480e9e4c16a81135924a8cf1440ad8c`をGitHub digestと照合した。成功した正確なcommitだけをmainへfast-forward・pushし、remote HEAD一致を確認。検証branchはlocal／remote双方から回収した。2世代remote backup tagは`65fde55dd57479b91afe37ae6e21e44576779288`と`515081f024e3ade49a79486e270caa4509519dd2`、本単位のrollbackは前者。

本単位をCLOSEDとし、追加fixture・証拠強化・文書修正目的で再実行しない。Mac Credential保管、sandbox外選択、CLI配布UI、Task実行、正式配布はP13後続と既存`release_blocker`へ保持する。Permission・Approval・Credential・Agent trustを登録から生成せず、通常Release `task_execution=unsupported`、`release_ready=false`を維持する。ローカル必須Rust全体の既知FAIL、日本語監査の変更外指摘、Flutter LSP制約は下記の記録どおりで、今回の対象PASSへ読み替えない。

##### 検証履歴（閉鎖前）

以下は変更・失敗履歴であり、当時の「現在」「未統合」「再開条件」を含め、上記の閉鎖状態を上書きしない。

手動Actions [run 37662923774](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37662923774)、commit `64009defe2e77518dfe1514f822aad7480e3f2f7`は模型欄の入力値一致までPASSした。続く実行系欄は、videoではscroll後に全面表示されているが、XCTestの束縛AX frameが初期の21pxのままでFAIL（23.669秒）。固定フォームの通常Tab移動（認証方式2項目を通って実行系欄、以後は各入力欄）へ限定して、古い座標への依存を除去する。6欄のAX identityと各入力値照合は維持し、製品focusや状態を直接設定しない。対象build／試験と後片付けはPASS。artifact `11502206558`、SHA-256 `071f6d36c01a605146b452ddcd91f7689db8234cc17848fc4a7f96b98312eb92`を照合して保存した。初回downloadの外部`ServerBusyEgress`は再取得で解消し、製品failureへ分類しない。

手動Actions [run 37661491163](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37661491163)、commit `4c52dc9fa3bce8619176a222f572caebdd8eed3b`では6欄のAX identity束縛と模型欄クリックまで成立し、`field.typeText`が「Neither element nor any descendant has keyboard focus」でFAIL（17.885秒）。videoは模型欄の実cursorを示し、固定Flutter engineの`FlutterTextInputSemanticsObject.mm`はAX用NSTextFieldと別のFlutterTextInputPlugin editorを使っている。文字は既存のapp key入力へ送り、同じ欄の既定値除去・入力値照合を保持する。scroll前の表示待ちと固定identityは維持し、内容領域でclipされた3px／21pxの欄を操作しないよう可視高さ30pxを要求する。製品sourceやfocusを注入しない。その他のbuild／対象試験・helper残留なし・source clean・fixture回収はPASS。artifact `11500857378`、SHA-256 `513eb829d2da6574134e5515555ae11f4f61c8218fa0a2642a1e708b82426700`を照合して保存した。

2026-10-08再開: 手動Actions [run 37659178592](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37659178592)、commit `56dfb3e9f6b1f4771a4b0919a89d15663fab7134`はMac runner上で実行された。請求設定を変更しておらず、以前のjob起動拒否はこのrunでは発生していない。Rust対象1件／helper build、固定実Codex準備、変更Dart解析／transport 7件、通常Mac buildはPASS。製品UIは23.058秒でFAILし、初期AX表示を待たずにscrollしたため模型欄ではなく実行系欄へ入力し、次の欄で試験側の高さ30px仮定に失敗した。videoで`codex-localtest-model`を確認した。表示後の6入力欄をAX identityへ一度だけ束縛し、入力欄の正の面積・既定値除去・入力値一致を確認するharnessへ局所修正する。製品要求・Authority境界は変更しない。

同runのhelper残留なし／runner source clean／専用CLI・Workspace回収はPASS。artifact `11498929704`、SHA-256 `63e9787fc91c1c07f2b4adde76ed00dbfdb2be3fa901b1a6b7f61b647fb0ab1c`を照合しignored領域へ保存した。Owner確認と登録結果表示はまだ未到達で、AcceptanceをCLOSEDにしない。以下のjob拒否と初回FAILは履歴として保持する。

現在の差分は検証branch `codex/macos-agent-registration-verify`へ保存し、mainへ未統合。初回の製品UI入力欄探索FAILを修正した`bb9a57bb0113ee04f47dc38c4583ee4acf63a5a4`の手動Actions [run 37635033580](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37635033580)は、job開始前にGitHub側で拒否された。check annotationは「recent account payments have failed or your spending limit needs to be increased」。どちらかの原因を断定せず、請求／利用上限設定を変更しない。同Repositoryのself-hosted runnerは0件。step未開始・artifactなしで、修正後UIのPASS／FAIL証拠ではない。

再開時点でMac検証環境の利用を確認した。上記branchで同じ有限UI条件を確認し、PASSした正確なcommitだけをmainへfast-forward・push・remote照合してから検証branchを回収する。追加QAや新しい製品Acceptanceを作らない。main／rollback pointは`65fde55dd57479b91afe37ae6e21e44576779288`、backup-mainも同値、backup-main-prevは`515081f024e3ade49a79486e270caa4509519dd2`でremote tagを照合済み。現単位をCLOSEDとせず、通常Release能力と既存release blockersを維持する。

既存Agent Centerの登録要求をRust所有OS確認と既存Brokerへ接続する。有限Acceptanceは`docs/specs/macos-desktop-channel.md`。権限取得・Task実行・sandbox外へのaccessを追加せず、既存登録境界を共有する。手動Actionsの`macos_agent_registration`で、固定版実Codexのinterface確認と製品UI登録を実行する。既CLOSEDのAdapter／Mobile／Windows条件を再試験しない。

ローカル対象Rust試験は初回、非承認時の期待を`suspended`と誤記してFAIL。既存Brokerの`desktop_native_owner_confirmation_required`拒否へtest oracleを修正し、製品拒否を変更せず1件PASSした。Owner callbackとCLIは合成`FIXTURE`。非承認時の未登録、承認後の登録・Task非対応・Permission／Approval未生成、重複・Authority field・未知Adapter・不正秘密path・保護root・CLI不在の拒否、Workspace未変更、Audit非露出を確認した。Mac画面成立・実Codex成立はまだ主張しない。

Schema 161／正常例157／負例208とmanual workflow検査、diff検査はPASS。厳格日本語監査は既存の変更外4 file／15 findingsでFAIL（今回差分にfindingなし）。Desktop／Mobileの`flutter analyze --no-pub`は既知の日本語path上のLSP FormatExceptionで失敗した。既存release blockerを維持し、変更Dart解析／試験とMac実行は専用手動Actionsで補助する。Rust全体／Conformanceは実行中で、PASSとはまだ記録しない。

その後Conformance 236件／Manifest 1197件／Release GateはPASS。必須Rust全体は505 passed／1 failed／12 ignored（後続target未実行）。変更外の既知`local_tls_server_repairs_only_after_verified_package_bytes`がOS 10054／TLS InvalidContentTypeでFAILし、同一test単独再実行は1件PASS。既存`FQ-TEST-LOOPBACK`と同分類へ保持し、全体PASSや根因解消とは扱わない。共有した登録検査の既存直接依存3試験は全体run内でPASS。

初回Mac手動Actions [run 37633822477](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37633822477)、commit `51fdf224b15138f41bdf4342a12aef2c9111468a`はRust対象／helper build、変更Dart解析・transport試験、通常Mac build、CLI固定package／version確認がPASS。UIは登録form表示後の模型欄検索でAX全要素value評価がtimeoutしてFAIL。保存画像でformと模型欄の実在を確認し、試験の探索を固定formのtext field順へ限定する。製品要求や権限検査は変えない。helper残留なし・専用CLI／Workspace回収を確認。artifact `11487758142`、SHA-256 `6c9f60e3aed54dd588782fc964f3957b65c0f58f92891cfcf2198fc8b4ec6d81`を照合しignored領域に保存した。実CLI hashは`16593cc2f422d5f398a8e40f550ebbaf1245392528957be342c295920a300704`、版は0.159.2。製品Owner確認・登録は未到達である。

#### macOS既存Adapter状態管理のOwner接続 — CLOSED（Product Build、2026-10-07）

成功commit `7c4219f5202402ad574e81b81e32d6eb02c82cd9`の手動Actions [run 37629357493](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37629357493)で有限AcceptanceがPASS。macOS 15.7.9 arm64／Apple Virtual Machine／Xcode 16.4、固定Flutter 3.44.0／Rust 1.95.0。製品UI→Rust所有OS確認→既存Brokerで、検証要求の不正署名拒否、有効化要求の未検証拒否、無効化・隔離・削除の受理と画面表示、Command-Q正常終了をXCUITest 1件（46.676秒）で確認した。helper残留なし・runner source clean。導入は新しい状態管理試験の準備であり、旧CLOSED試験は選択していない。

同runでRust対象1件、変更Dart 5 fileの`dart analyze`、Owner待機期限test 1件、起動後catalog操作のWidget回帰test 1件、通常`flutter build macos --debug --no-pub`もPASS。macOS実操作は限定`LIVE_RUNTIME`。Rust Owner callbackとWidget応答は`FIXTURE`で、署名検証／有効化の正の状態遷移は既存Broker署名testの直接依存結果だけに限定する。UI試験は明示Debug表示用semantics flag付きであり、通常Releaseの全機能や製品Trust設定、外部artifact実行を証明しない。

artifact `11486335093`（255161 bytes、有効期限2026-10-10T13:36:53Z）をignored `release_evidence/p13-macos-adapter-7c4219f.zip`へ保存し、SHA-256 `23fa9afeb17bfdf089118a320fe003571fce66accd826d81b0d7f578794b4158`をGitHub digestと照合した。既存release blockers、通常Release `task_execution=unsupported`、`release_ready=false`は維持する。次はP13の残る製品機能であり、このOwner接続へ追加の証拠強化を要求しない。以下のFAILと修正履歴は保持する。

ローカルの従来Adapter非承認表示testもASCII複製で1件PASS。複製`C:\Users\ohira\AppData\Local\Temp\d4p-adapter-ui-feb74a8c27d44c7d9eafb68dd5dafd0a`は試験後の正確なpath確認付き削除が実行ポリシーで拒否され、製品外に残った。削除を迂回しない。これは開発環境の`known_limitation`であり、製品への同梱物・新しいrelease blockerではない。元のコードの複製とbuild成果物は再生成可能である。

検証・有効化・無効化・隔離・削除の既存ボタンをRust所有OS確認とBrokerへ接続する。導入・更新CLOSED条件を再開せず、catalog・Audit内の作用だけを扱う。既存Schemaを再利用し、対象ID／hash／要求hashを束縛する。署名Trust未設定または不正なAdapterは有効化しない。有限Acceptanceは`docs/specs/macos-desktop-channel.md`。手動Actionsの専用targetを使い、既CLOSEDのUI試験を選択しない。macOS製品Trust設定・Credentials／Agent／Workspace・正式配布・Final QAは別の`release_blocker`範囲へ保持する。

ローカル対象`cargo +1.95.0 test --locked --offline --manifest-path native/rust_helper/Cargo.toml --lib macos_desktop_worker::tests::owner_adapter_lifecycle -- --test-threads=1`は1件PASS。実Brokerと永続Auditを通し、Owner拒否時不変、不正署名・未検証有効化拒否、無効化・隔離・削除、古いhash・追加field・操作不一致拒否を確認した。Owner選択は合成callbackの`FIXTURE`で、Mac画面成立の証拠ではない。

必須Rust全体`cargo +1.95.0 test --locked --offline --manifest-path native/rust_helper/Cargo.toml -- --test-threads=1`はlibrary 504 passed／1 failed／12 ignored。変更外の`failed_tool_result_is_not_replayed_as_another_exec_command`でHTTP本文途中のConnectionReset（received=0／expected=23278）を観測し、同一test単独再実行はPASS。既存`FQ-TEST-LOOPBACK`へ記録し、全体PASSや原因解消とは扱わない。library失敗で後続targetは未実行。直接依存の`broker_owned_signature_verification_is_required_before_enable`は同じ全体runでPASS。

Schema 161／157／208はPASS。初回Conformanceは待機操作の期待集合不足でFAIL（Windows Runnerは既に5操作を含んでいた）。Flutterの305秒対象と期待集合へ5操作を追加し、Conformance 236件はPASS。Windows transportの実装は変更していない。厳格日本語監査は変更外の既存4 file／15 findingsでFAIL、Desktop／Mobileの`flutter analyze --no-pub`は日本語pathでLSP FormatException／server exit 255／CLI exit 1。これら既存`release_blocker`は保持し、変更したDartの検査とMac build／UIは専用手動Actionsで行う。

初回手動Actions [run 37627578133](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37627578133)、commit `ec48769d8e36ecf47de0393604649c75677d3b3e`はRust対象・Dart解析／待機試験・通常buildがPASS、状態管理UIがFAIL。新規Adapter導入後の「検証」を押してもOwner画面に到達しなかった。画面は新しいcatalogを表示する一方、`manageAdapter`は起動時snapshotだけを検索しており、新規recordを拒否していた。画面で選択したrecordのID／hashへ操作要求を束縛し、Brokerの現行hash・Owner・署名再評価は維持する。導入CLOSED条件は破壊されておらず再開しない。artifact `11484996544`、SHA-256 `3ad1d5f2f6c15d9086ec04a8d1bfc7e903e4c55784269c952200ed3bbb005f9c`を照合して保存した。helper残留なし／runner source cleanも確認したが、正常終了UIの証拠とは扱わない。

修正後の「起動後のAdapter一覧に表示されたrecordを現在hashへ束縛して操作する」Widget回帰試験は、同一sourceのASCII一時複製で1件PASS。専用test identityの一覧更新→選択→要求ID／hash→Broker非承認receipt表示を確認した`FIXTURE`である。局所`dart analyze`は終了処理でDart perf file削除のOS 1920／SERVER_ERRORとなり、成功扱いしない。Mac手動Actionsの変更Dart解析で補助検証し、ローカル解析環境の既存release blockerは保持する。

#### macOS Rust Owner確認・Adapter導入／更新 — CLOSED（Product Build、2026-10-07）

成功commit `0729f07437f308c71c1f4dc694e62016ea58ad3e`の手動Actions [run 37623944221](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37623944221)はPASS。macOS 15.7.9 arm64（Apple Virtual Machine）／Xcode 16.4／Rust 1.95.0／Flutter 3.44.0で、製品UI→既存MethodChannel／匿名pipe→Rust検査→OS Owner確認→既存Broker receiver→catalog登録→製品表示を通した。OS画面で最初に「承認しない」を選ぶと未登録を維持し、次の新規要求で「今回の操作を承認」を選ぶと登録が表示された。XCUITest 1 passed／0 failed／0 skipped、38.953秒。Command-Qで通常終了し、helper残留なし・runner追跡source cleanを確認した。

同runの`cargo +1.95.0 test --locked --manifest-path native/macos_owner/Cargo.toml`は2件PASS（入力拒否、実OS期限切れ非承認）。`cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --lib macos_desktop_worker::tests::owner_adapter_roundtrip -- --test-threads=1`は1件PASS（実Broker導入／更新・永続catalog、非承認時不変、replay／session注入／不正hash拒否）。後者のOwner選択は合成callbackであり、画面証拠は前記XCUITestに限定する。`flutter build macos --debug --no-pub`は通常の試験表示flagなしでPASS。UI検査の`xcodebuild test ... -only-testing:RunnerUITests SWIFT_ACTIVE_COMPILATION_CONDITIONS='DEBUG D4_MACOS_OWNER_UI_TEST'`だけは既述の表示用semantics補助を含む。秘密・Authority・UI要求の迂回は追加していない。

artifact `11482839193`（remote期限2026-10-10T12:54:31Z）をignored `release_evidence/p13-macos-owner-0729f07.zip`へ保存し、SHA-256 `c337857b41899ac5b41e26b6bbc2402f859ad58a101060fea1273cbd245374d8`を照合した。成功した正確なcommitを`main`へfast-forward・pushし、remote HEAD一致を確認した。rollback point／backup-mainは`2d5c74af31bf0a09c2f3d30f6553064771f16a39`、前世代は`91107dec2ab5ef606b26dd6276b60756ab0a6c84`、remoteは同名tag。検証branchは本閉鎖記録の同期後にlocal／remoteとも回収する。

証拠はhosted Macの限定`LIVE_RUNTIME`と上記callback試験の範囲だけ。macOS全機能、他Owner操作、Credentials／Agent Task、通常Release／正式署名配布、Final QAは`release_blocker`として保持する。変更外のローカルRust TLS fixture failure・既存日本語監査指摘・Flutter LSP制約も解消したと扱わない。新しい仮説的riskをこのCLOSED単位へ追加せず、次のP13製品差分へ進む。以下は実装・失敗・修正の履歴である。

通常接続CLOSED後の不足であるOwner確認を、Rust所有OS確認→既存Broker process内receiverへ接続する。最初の実作用は既存Adapter catalog導入／更新。Flutter／SwiftへAuthorityを移さず、非承認と不正入力はnormal拒否・Auditへ通す。固定hashと現在envelopeを再評価し、通常Release Task能力・正式配布・他Owner操作を昇格しない。有限AcceptanceとFFI局所レビューは`docs/specs/macos-desktop-channel.md`。macOSがないため手動Actionsの専用Owner検査を使い、既CLOSEDの通常bootstrapを再試験しない。

初回のRust統合testは一覧fieldを`adapters`、保存fileを`adapter_state.json`と誤ったtest oracleでFAILした。現行Brokerの`Adapter一覧`／`adapters.json`へ合わせ、製品挙動は変更しない。試験上のOwner選択は合成callbackであり、Mac OS画面の証拠と混同しない。

修正後のOwner対象Rust統合1件とmacOS確認crateの入力拒否1件はWindowsでPASS。必須`cargo +1.95.0 test --locked --offline --manifest-path native/rust_helper/Cargo.toml -- --test-threads=1`はlibrary 503 passed／1 failed／12 ignored。失敗は変更外のUpdate Download TLS fixture `local_tls_server_repairs_only_after_verified_package_bytes`のOS 10054で、既存`FQ-TEST-LOOPBACK`へ保持する。後続targetはlibrary失敗で未実行。Schema 161／157／208、Conformance 236、Manifest 1197、差分検査はPASS。厳格日本語監査は既存4 file／15 findingsでexit 1。Desktop／Mobile `flutter analyze --no-pub`は既知の日本語path環境でLSP FormatException／analysis server exit 255／CLI exit 1。全体PASSとせず、Mac製品経路は手動Actionsで確認する。

初回Mac [run 37602624819](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37602624819)、commit `1bf2e2c772367dc03f1c1c605b9cefa3b9d4589d`はRust確認crate 2件（実OS期限切れ拒否含む）、Broker対象1件、同梱製品buildがPASS。XCUITestは初期「実行系」要素を取得できず1件FAIL、helper残留なし。Flutter固定3.44 sourceがOSの`AXEnhancedUserInterface`通知からsemanticsを有効にするため、試験側から当該appの既存アクセシビリティ接続を要求し、要素のlabel／value差を扱う。製品codeは変更せず、初期画面の限定診断で接続不能を隠さない。OS確認への到達はまだ未成立。

次の[run 37603945218](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37603945218)、commit `4626c551b0fe67a22405556ba5a5f8b4da638f23`もRust／製品buildはPASS、UIは初期要素取得でFAIL。診断ではnative窓だけが存在しFlutter子要素がなく、外部AX APIは`-25211`で拒否された。無効だった外部AX要求と初期tree診断を撤去し、明示Debug試験buildに表示用semantics有効化だけを追加する。通常製品のAuthority／資格／Owner確認への変更やOS保護設定変更はしない。試験表示補助の責任・除去条件・証拠範囲はmacOS接続contractへ記録する。

表示補助初版の[run 37604950948](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37604950948)、commit `90fe144f624daadf6701b0353a7c02bb818391d8`もbuild成功／初期要素取得FAIL。固定Flutter sourceではEngine起動が`viewWillAppear`にあり、windowの`awakeFromNib`からの通知はEngine起動前に届く可能性がある。通知を既存の初期Broker要求完了後へ移し、起動済みEngineに一回だけ接続する。この原因仮説の成否は再試験で確認し、通常要求・Owner経路は変更しない。

起動後通知の[run 37605913898](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37605913898)、commit `e43e6ad2e44dae155aa9cf5f7200e2ad4a3e0f73`も初期要素取得FAILとなり、通知時点だけでは解消しなかった。通常buildと試験buildのbundle IDが同一なため、XCTestはID検索ではなく既定の試験対象appを起動するように限定する。初期失敗時のみ新規runner・入力前の画面画像とtreeを記録し、起動先／描画状態の推測を避ける。後片付けは各runで成功しているが、Owner画面のAcceptanceは未成立のまま。

既定target起動の[run 37606764030](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37606764030)、commit `67d30ac86099793e94e5fbf61c6ac87d95cecc1d`も初期要素がなく、画像を`/tmp`へ書く試験処理もOSに拒否された。標準XCTest runnerのsandboxは維持し、画像はXCTAttachmentへ変更する。入力前の固定bootstrap完了件数だけを試験窓titleへ投影し、Broker接続と表示補助のどちらが未成立かを区別する。製品状態・要求本文・秘密は診断へ出さず、通常buildはこの表示を含まない。

[run 37607914073](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37607914073)、commit `2975d9e5dd483050d05d28e227793ea6ad950972`の診断で、初期応答11/11とFlutterの全画面要素が存在することを確認。残る失敗は検索側が`value: 実行系\nタブ: 4/21`を完全一致で探したためだった。valueにも改行区切りの接頭一致を適用し、不要になった窓title診断を撤去する。artifact `11476530090`をignored `release_evidence/p13-macos-owner-2975d9e.zip`へ保存し、SHA-256 `c37bd515c79f3383ebd32e21499b0601777945255bce1754cca3c1bfa53e6834`を照合済み。画像の`--only-failures` exportは手動添付を除外していたため、当該合成UI試験の添付を通常exportする。Owner操作そのものはまだ未到達であり、PASSとはしない。

[run 37608903018](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37608903018)、commit `7a49aef81121d4ff170c6175dff56953dc53d2c8`は実行系画面と導入ボタンの操作まで到達し、入力欄の全要素label／value検索がsnapshot timeoutでFAIL。Rust対象試験と通常buildはPASS。artifact `11477525425`をignored領域へ保存し、SHA-256 `e6cf01c419351556f32e8a020cf16a4aeb79c2a29e328abf98157267a259bb46`を照合した。固定FlutterのmacOS編集欄は`NSTextField`であるため、試験側をdialogの単一text field検索へ限定し、入力前の画面だけを添付する。検索timeoutの解消は次の試験で判定し、Owner確認を成功扱いしない。製品挙動・OS保護設定は変更しない。

[run 37610479903](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37610479903)、commit `c94a548399dada3a97a47614a27db28366039a47`ではquery timeoutがなくなり、入力欄不存在でFAIL。保存画像とAX treeにより、実際にはdialogは開いておらず、導入ボタンが画面外で高さ1pxに切り詰められたまま`isHittable=true`になっていたと分かった。前runの「dialog表示後」は推測であり、実操作成立の証拠ではなかった。試験をボタンの表示高30px以上まで実スクロールする条件へ修正する。artifact `11478151399`、SHA-256 `95c2383dc18bbd566ebbdb38c03739a34c2ce289a5a127c731879c349087cedb`を照合済み。Rust対象・通常build・runner後片付けは成功、Owner確認は未到達。

[run 37611271939](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37611271939)、commit `2dbf5ea544444c0786bad7b275f7d5118f2cd164`は導入ボタンの表示高1pxを検出してFAIL。XCTestの保存eventを読むと、Windowの既定scroll位置は本文ではなくtitle bar `(182, 65)` だった。画像・AX frameで確認した本文の相対位置へ標準XCUICoordinate scrollを送るよう修正する。artifact `11477987787`、SHA-256 `f28d8bde101fe7ad7bf8c9fddc20923a0f6bd5028303e7c7fa1dd7902bb35600`を照合済み。製品の操作・権限経路は変更せず、OS確認Acceptanceは未成立を保持する。

[run 37612168584](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37612168584)、commit `e9da6dc0f8a353be1b9c83003acca3dd71899ba1`で実スクロール・dialog遷移・text field取得まで到達。AX treeには入力欄 `{{193,322.5},{638,166}}` とsubmitが存在したが、native欄の`click()`はnot hittableでFAILした。固定Flutter sourceはnative編集欄をFlutterView背面へ置くため、入力欄が窓内にあることを確認し、その中央への通常coordinate clickへ変更する。UI stateやBroker要求への注入は行わない。artifact `11478294105`、SHA-256 `ba3eff2497cc49e81c580eaec25bbc119f67206c12cbd5b052ed44b4f1bb72d0`を照合済み。Owner確認そのものは引き続き未到達。

[run 37613082451](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37613082451)、commit `5c56b47a27941069ee3e32fea4ff4592f7cfd996`で入力欄へのclickと合成Manifestの入力まで到達したが、dialog内submitの`click()`もnot hittableでFAIL。観測されたボタンframe `{{654,547},{178,33}}`を窓内確認し、同じ通常coordinate clickへ揃える。OS確認の選択・Broker受理はまだ未成立であり、製品を変更せず試験操作だけを修正する。

[run 37614000665](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37614000665)、commit `59b42ea9c323784913311490636200c4d88c1f8b`で入力・submit・dialog閉鎖まで成立したが、OS確認ボタン不存在でFAIL。artifact `11478912301`、SHA-256 `a4dbb06139f61fe9fc0ecb1151e874fd17b73289815a6aaa0c34893d815b3594`を照合し、画面記録でOS確認が表示されていないことを確認した。入力受理・Broker拒否・OS表示を切り分けるため、同じ合成試験の失敗時に限り既存製品UIの拒否理由と全画面を記録する。確認不能を承認へ変えるfallbackやOS保護設定変更はしない。

[run 37615419157](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37615419157)、commit `7aa1d0d0f551c1800a2242688686a2696971054e`もOS確認ボタン不存在でFAIL。artifact `11479865081`、SHA-256 `a4db0bf10b916510bda97748a0af85fe23918208667d363fb06040fa29b6cb19`を照合した。入力snapshotのJSON全564文字・署名128文字は意図した合成Manifestと一致する。製品UIにはAdapter ID付き処理結果があるがtree表示で本文が省略されていたため、既存status値を最大1024文字だけ読み取る。専用runnerの当該helperに対するOS拒否logも限定取得し、原因未確定のまま製品やsandboxを変更しない。Rust確認crate 2件・実Broker対象1件・通常製品buildはPASS、Owner製品経路は未成立のまま。

[run 37617227763](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37617227763)、commit `8259281cd15e33509b1418599e9a1806d128cc79`の既存UI結果は`rejected / Adapter管理要求の構造または操作名が不正 / broker-audit-23`だった。確認画面より前の入力構造拒否であり、OS sandbox失敗という仮説は採用しない。artifact `11480881994`、SHA-256 `4a8ccf8ef30f99d847eb5b798e57bd6e3b4511c09dd3592e33daed930e96769d`を照合。native入力snapshotは前回と同じ完全なJSONだが、Flutter controllerから送られた要求との差は未確認。不要なOS log取得を撤去し、明示Debug UI試験flag内だけで既知fieldの存在・型・件数と操作名一致を一時観測する。本文・署名・資格は出力せず、原因確定後にこの型観測を撤去する。通常製品の要求と判断経路は変更しない。

[run 37618440784](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37618440784)、commit `3040b0e93e23d4fb736b32b6ba06c840b9a7489a`は同じ構造拒否でFAIL。artifact `11480138893`、SHA-256 `115e231d91fadc29c104d894660e2f98a8b1f9f36fdc61d2e0047083ce33a6ca`を照合。画像上の型診断はroot 3 field・操作名一致・版integerを示したが、XCTestで窓titleのlabelを読んだ値は空だった。観測済みtitleをAX valueから読むよう試験側を訂正し、製品の拒否を変更しない。

[run 37619551427](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37619551427)、commit `62b7495429ee72428e035e9e30fd5c2f87a0f577`も同じ構造拒否でFAIL。Flutterの窓AX valueも空であり、型情報の取得手段として不適切だった。窓title改変を撤去し、同じ明示Debug UI試験flag内で既知fieldの型情報だけをOS標準logへ出す。workflowは固定診断prefixに限定して読み取る。製品要求・Owner選択・本文には触れず、診断の撤去条件も維持する。

[run 37620401839](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37620401839)、commit `cad35a686c373c9442b9c1e70ee6b2b06c5c60f1`のOS logではroot 3 field・操作一致・Manifest 17 fieldが取得でき、全既知fieldの大分類は一致した。構造拒否は継続している。次は全payload値が合成公開fixtureと等しい場合に限り、JSON envelopeから切り出したpayloadを再parse・再照合して送信byte表現を一時観測する。任意入力・envelope・資格を記録せず、fixtureの署名は全0固定の公開値である。Owner確認前の拒否が解消した証拠はまだない。

[run 37621652477](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37621652477)、commit `9664cb24b147ff3a4d1544193584b4a33956d3e3`のwire観測で原因を確定した。macOS `typeText`を経由した日本語keyが、`既`（U+65E2）から`既`（U+FA42）、`署`（U+7F72）から`署`（U+FA5A）へ変化していた。Swift／Foundationの辞書等価比較ではfixture一致となる一方、Rustの厳密なfield名では未知field／必須field欠落となる。root／Manifestの版tokenは整数1で、OS確認失敗・数値型不一致ではなかった。試験側のJSON入力だけをUTF-16単位のUnicode escapeへ変換してASCIIで入力する。日本語keyの意味と既存UI操作は保持し、製品の署名・hash・Schema検査や未知key拒否は変更しない。一時的なSwift型／wire log、重複fixture、workflow診断を全て撤去した。修正後のOS確認と登録の成否は次の対象runで判定する。

Unicode keyの原因証拠artifact `11481784642`をignored `release_evidence/p13-macos-owner-9664cb2.zip`へ保存し、SHA-256 `a4564eb08915ef570d3fdf6856588d688e54dd345771f6ec8f97caf62112c7a4`を照合した。保存logをPythonでparseし、field名にU+FA42／U+FA5Aが実在することも確認した。

[run 37623072404](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37623072404)、commit `89e8764ff99361bbe6cab382b69e498cdcc6b09b`では入力構造拒否が解消し、OS確認dialogが出現した。XCTestの「承認しない」がdialogとTouch Barの同名2要素へ一致してclick FAILとなったため、検索を実dialog内へ限定する。画面未承認のまま試験が中断し、runnerの残留helper回収stepもexit 1だった。正常終了の証拠とはせず、実承認・登録・正常終了は次のrunで判定する。

#### macOS Desktop通常Broker接続 — CLOSED（Product Build、2026-10-07）

開始時のmacOS Runnerには`gui_shell/broker`登録がなく、Flutterの通常起動から実Brokerへ到達しなかった。本有限単位を、同梱Rust helperへの匿名pipe、既存認証Broker normal経路、初回設定等の製品bootstrap、正常終了へ限定した。Apple App Sandboxを継承し、Flutter／Runnerへ資格を渡さない。Owner専用操作・Windows固有操作は現状の拒否を保持し、macOS全機能完成とは扱わない。責任と未成立範囲は`docs/specs/macos-desktop-channel.md`。手動Actionsの独立`macos_product`を使い、iOS／AndroidのCLOSED試験は起動していない。

初回Rust対象試験は2件中1件PASS／1件FAIL。normal資格で許可される既存`shutdown`をOwner専用と誤ったtestの前提が原因で、Owner専用の`作業領域承認`と正確な拒否codeへ修正する。Broker本体の権限規則は変更しない。

修正後のWindows `cargo +1.95.0 test --locked --offline --manifest-path native/rust_helper/Cargo.toml -- --test-threads=1`はlibrary 501 passed／1 failed／12 ignored。新規macOS接続test 2件はPASS。失敗は変更外の`bounded_catalog_fetch_rejects_declared_document_over_limit`でTLS fixtureのOS 10054／InvalidContentType、既存`FQ-TEST-LOOPBACK`範囲に保持する。library失敗後の他targetは未実行であり、Rust全数PASSとは扱わない。Schema 161／157／208、Conformance 236件、Manifest 1192件、`git diff --check`はPASS。厳格日本語監査は既存4 file／15 findingsによりexit 1、今回の追加fileにfindingはない。Mac側のbuild／正常経路は手動Actionsで確認するまで未成立。

初回手動Actions [run 37596139461](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37596139461)、commit `5b6e1c9d34f8d364454bcb657adf9f2a5645a09d`ではMac上のRust対象2試験とhelper buildがPASS。Swiftの`write(contentsOf:)`／`read(upToCount:)`がmacOS 10.15.4以上を要求し、既存最低OS設定に対してcompile FAILとなった。最低OSを引き上げずDarwin pipe I/Oへ修正し、EINTR、書込切断、上限、親側の不要pipe端のcloseを扱う。native製品試験は未実行。この間のDesktop／Mobile `flutter analyze --no-pub`は既知のOneDrive日本語pathでLSP JSON FormatException、analysis server exit 255／CLI exit 1となり、host-local制約として保持した。生成された今回のcrash log 2 fileは回収済みで、製品sourceへ回避策を追加しない。

次の手動Actions [run 37597211261](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37597211261)、commit `7f756250c5d290eafc1afd4a01ba961c1f7bf171`はRust対象2件と製品buildがPASS、native XCTestは1 passed／1 failed。製品bootstrap待機が20秒で失敗し、終了assertionは通過した。code確認ではmacOS helperが固定package配置検査を行わず、Brokerの`初回設定取得`が未初期化で拒否される未接続を確認した。Rust側で固定bundle配置を検査して既存の非権限UI設定初期化へ接続し、対象試験へ初回設定取得を追加する。bootstrap観測はDebugだけへ限定し、固定操作名だけを失敗診断へ使う。Owner資格・承認経路は追加しない。

修正commit `a388e1d648536ce6f45ede48bc7d3a453075f6a7`の手動Actions [run 37598578513](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37598578513)はPASS。macOS 15.7.9 arm64（Apple Virtual Machine）／Xcode 16.4／Flutter 3.44.0／Rust 1.95.0で、製品Debug appにRust helperを同梱し、Flutter起動要求→native匿名pipe→実Brokerの初期11操作とhelper正常終了を確認した。native XCTest 2 passed／0 failed／0 skipped、接続test自体1.811秒。Rust対象3件もPASSし、normal Brokerの初回設定、session注入・replay・Owner操作拒否、frame上限、起動lock競合、固定配置不足の拒否を確認。helper残留なしと対象source cleanを確認した。これはhosted Mac上の実製品bootstrap／transportの限定`LIVE_RUNTIME`であり、画面全機能の操作・Owner確認・実Agent・通常Release・正式配布の証拠ではない。

artifact `11471778223`（期限2026-10-10T09:11:45Z）のSHA-256 `73cb24fbaa9bcf70bdd5cb14121a22db81b4ae54f7dace117783ade0e60f6888`をローカル保存物と照合した。保存先はignored `release_evidence/p13-macos-product-a388e1d.zip`。成功した正確なcode commitを`main`へfast-forward・pushしremote HEAD一致を確認した。復旧点は`codex/backup-main=91107dec2ab5ef606b26dd6276b60756ab0a6c84`、前世代`codex/backup-main-prev=9ed7e71e1e77be00cc71cb65a06e8f134504240f`で、remoteは同名tag。検証用branchは文書同期後にlocal／remoteとも回収する。

今回修正後のローカル必須Rust全体はlibrary 501 passed／2 failed／12 ignored。変更外のUpdate Download TLS fixture `bounded_catalog_fetch_rejects_declared_document_over_limit`、`local_tls_server_repairs_only_after_verified_package_bytes`がOS 10054／InvalidContentTypeで失敗し、既存`FQ-TEST-LOOPBACK`の未解決履歴に保持する。他targetはlibrary失敗により未実行。対象3件、Schema 161／157／208、Conformance 236、Manifest 1192と差分形式検査はPASS。既知の全体Rust失敗・日本語監査指摘・ローカルFlutter LSP制約を全体PASSへ読み替えない。

次はP13の残る製品機能差分で、macOS Owner確認・Credentials・Agent／Workspace・Taskのplatform対応は`release_blocker`として残る。今回CLOSEDした通常接続を追加証拠目的で再開しない。起動lockの異常終了後Recovery、正式identity／署名／配布、全platformの最終品質保証も別gateであり、通常Release `task_execution=unsupported`、`release_ready=false`を変更しない。

#### iOS製品UI／native service／基本lifecycle — CLOSED（Product Build、2026-10-07）

有限Acceptanceは、新規専用Simulator上の製品Flutter画面からnative招待画面・接続先確認・既存serviceへ進み、実Broker結合、Runtime表示、OSによるbackground／復帰、製品画面での離脱を一度成立させること。現行harnessは標準XCUITestで画面を操作する。招待だけは明示的なSimulator試験buildの`D4_IOS_PRODUCT_TEST`内でnative secure fieldへ渡し、Flutter／shell／環境変数／UI操作logへ秘密を渡さない。service直接呼出し・foreground偽装・承認迂回をしない。通常buildでは試験入力helperと起動呼出しをcompileから除外し、同flagを付けた物理端末buildはcompile errorにする。`--ios-product-ui`は新規製品接続試験だけを選択し、CLOSEDの部品試験を再実行しない。物理端末、網羅的lifecycle timing、Final QAは本単位へ追加しない。

初回手動Actions [run 37587096021](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37587096021)、commit `3f4d0f709125e771d105fc1bcba5cb13b29991f8`ではRust build、Mobile解析・21試験、Simulator buildは成功したが、XCTest sourceで非Optionalな`FlutterEngine`にoptional chainingを使ったためcompile失敗し、製品接続testは未実行だった。公開型に合わせた1行修正を行う。製品sourceは変更していない。ローカルSchema 161／157／208、Conformance 236、手動workflow限定・Manifest検査はPASS。厳格日本語監査は変更対象外4 file／15 findingsでexit 1を保持する。

次の[run 37587678571](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37587678571)、commit `c8326aa283c17a0053d7cd5a64445b57c4351e5f`はcompile成功、製品XCTest 1件が`initial_screen`でFAIL。FlutterのNavigationDrawer sourceは選択項目にbutton traitを付けておらず、harnessのbutton限定検索とは不一致だった。label一致と公開`accessibilityActivate`の結果による操作へ修正し、初期段階もroot／semantics／Keychain／navigationへ分けた。UIの操作可能性や製品成立をまだPASSとは扱わない。

[run 37588613274](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37588613274)、commit `cdcd0baa3fe8d0e5008807fc2f1cda737246a36c`は画面遷移・native招待画面起動まで到達し、`native_invitation`でFAILした。招待受信／形式検査／確認操作を別段階へ分け、UIKit controlの既存登録actionを送る公開経路と、操作失敗時のclass名だけの限定診断を追加する。service・承認・秘密保管の直接書換えは行わない。

[run 37589607834](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37589607834)、commit `27a57ae2f09a7aadaf37b837a8b7e4d621f2b0fc`で招待受信・形式検証まで進み、`invitation_submit`でFAIL。hosted XCTestのin-process accessibility呼出しではUIKit確認操作を成立させられなかったため、この試験経路と診断を撤去し、標準RunnerUITests targetへ置き換える。招待秘密を`typeText`の操作logへ出さないため、native入力helperだけを明示Simulator試験buildへ限定する。通常製品の新しい操作能力・Authority・credential経路ではない。確認ボタン、HOME、製品復帰、離脱はXCUITest自身が操作する。このharnessはdevelopment専用として保持し、UIKitの通常UI testだけで秘密を安全に入力可能になった場合に補助入力部分を除去できる。

XCUITestへ切り替えた[run 37591398458](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37591398458)、commit `d8e30d246f9c85de9af6ce7b2a92b4f081c31e30`は通常Simulator buildと試験構成のcompile・起動が成功したが、`initial_screen`で1件FAIL。Flutter見出しの要素種別をstaticTextへ固定した検索をlabel基準へ揃え、初期header／navigation段階を分離した。招待発行前に限り固定4 labelの存在・型・操作可否だけを出力し、画面tree・入力値は記録しない。

修正commit `1188ed96439a56b698f0bca52f0ab4466b115aa8`の手動Actions [run 37592632518](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37592632518)はPASS。macOS 15.7.9／Xcode 16.4、iPhone 16 Pro／iOS 18.5の新規専用Simulatorで、Flutter画面からnative招待入力・接続先確認・既存Swift service・実Rust Brokerを通し、資格保管再読、Runtime left／right表示、HOMEによるbackgroundと製品復帰、設定画面からの離脱まで成立した。XCUITest 1 passed／0 failed／0 skipped、実行時間44.289秒。Owner側の招待・結合不存在、招待秘密がXCTest出力・Broker log・durable Auditにないことも確認した。Rust Broker build、Mobile解析（No issues）、Flutter test 21件、試験入力helperを含まない通常Simulator buildが成功した。CLOSEDの部品試験は実行していない。

正確な実行経路は`python3 tooling/minidora_live_check.py --reference <固定MINIDORA checkout> --ios-simulator <専用UUID> --ios-derived-data <専用directory> --ios-result-bundle <専用xcresult> --ios-product-ui`からの`xcodebuild test -only-testing:RunnerUITests/DeviceLinkProductUITests`。MINIDORA参照は`3400a3bb68b37efa1dc14ee8aaa28fda779bf1f8`。証拠は試験入力flag付きSimulator buildの基本製品経路に限る`LIVE_RUNTIME`であり、通常Release、物理端末、網羅的lifecycle／障害、正式配布を証明しない。Authority・Approval・秘密保管・通信の製品経路は変更していない。

artifact ID `11468904894`、SHA-256 `90ff1cce5a16d6fe24eee97ea01906a9f9acb1b8c13dc32b196d4018621bffc0`、remote保存期限2026-10-10T08:20:41Z。同hashのZIPをGit対象外`release_evidence/p13-ios-product-1188ed9.zip`へ保存し、要約とPASS内容を照合した。runner追跡source差分なし、Broker招待・結合回収、専用Simulator停止・削除が成功した。成功した正確なsource commitを`main`へfast-forward・pushし、remote HEAD一致を確認した。変更前rollbackは`9ed7e71e1e77be00cc71cb65a06e8f134504240f`、前世代は`b268add4a40ed1095902fc6bd056e0a52fd0d22c`。

この有限AcceptanceをCLOSEDとし、証拠強化のために再実行しない。P13の残る製品機能差分へ進む。Mobile実機／正式identityとFinal QAの`release_blocker`、Android実機凍結、通常Release `task_execution=unsupported`、`release_ready=false`を保持する。過去のFAILを消去せず、文書同期のためのruntime再試験もしない。

閉鎖文書の局所検査は`python -X utf8 tooling/schema_check/check_schemas.py`（161 schema／157正常例／208負例）、`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`（236件）、`python -X utf8 tooling/release_gate_check.py --release-track windows_v1`、Manifest 1187件、`git diff --check`がPASS。厳格日本語監査の既存4 file／15 findingsは変更外の最終QA `release_blocker`として保持する。Rust製品codeとDartは本単位で変更しておらず、Rust全数・Desktop解析・物理端末試験は再実行対象に追加していない。

#### iOS native TLS／Keychainと実Brokerの接続 — CLOSED（Product Build、2026-10-07）

今回の有限単位は、Simulator native XCTestから既存Swift `DeviceLinkTLSClient`／`DeviceLinkNativeStore`を実Rust Brokerへ接続し、招待結合、Keychain保存・再読、Runtime一覧、離脱・失効を成立させること。誤pin拒否も確認する。試験用Ownerは既存Broker制御経路で一時招待を発行し、専用loopback listenerからnative test内へだけ渡す。公開する環境変数はlistener portだけとし、招待・端末資格をDart、shell引数、環境変数、診断、artifactへ含めない。これはdevelopment専用harnessで、production通信やAuthorityを追加しない。製品Flutter UIの操作・native確認dialog・OS lifecycle・物理端末は別の未成立範囲として保持し、この接続単位の成功へ含めない。既存8 XCTestは変更test targetの直接依存検査として維持し、Android CLOSED経路を再実行しない。macOSがlocalにないため既存`apple-manual-build.yml`の`ios_mobile`手動起動で補助検証する。

Androidの実装・検証履歴は維持し、Android実機試験の凍結を尊重する。iOSにはSwift Device Link handler、ThisDeviceOnly Keychain保管、TLS client、strict JSON処理、native XCTestが実装されている。Windows hostではApple toolchainを実行できないため、既存の`.github/workflows/apple-manual-build.yml`の独立`ios_mobile` jobをP13の局所build／testに使った。今回の実Broker接続はSimulator内のnative部品を通す範囲に限定し、製品UI全体・実端末・配布・Final QAの証拠へ昇格させない。

今回のharness追加後、Python 2 fileの`py_compile`、Schema 161／157／208、Conformance 236件、手動workflow限定検査、`git diff --check`はPASS。OneDrive日本語path上のDesktop／Mobile `flutter analyze --no-pub`は双方ともLSP JSONの`FormatException: Unterminated string`でanalysis serverがexit 255となり、CLIはexit 1だった。既知のhost-local解析制約として失敗を保持し、今回Swiftのcompile成功とは扱わない。この初回harness追加ではDart／production Swiftは変更しておらず、続く実接続FAILに対して下記Swift局所修正を行った。

手動Actions [run 37583469599](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37583469599)（`db76d9c10dbc44e0908dcf8ed0621e3c819e9f11`）ではmacOS Rust build、Mobile解析・21 Flutter test、Simulator build、既存8 XCTestはPASSし、新規native接続試験は最後の`revoked_credential`でFAILした。先行する結合・Keychain再読・Runtime一覧・離脱までは到達した。Brokerの既存拒否codeは`端末要求拒否`だが、iOS側だけがASCIIに限定していたため、正しい拒否応答を不正frameとして扱っていた。文字・数字と`_.-`、UTF-8長1〜96 byteの限定検査へ局所修正し、空白・制御文字・長さ超過を引き続き拒否する。単体testと同じ実接続試験で修正を確認する。拒否をacceptedに変換せず、自由文errorや資格をUIへ渡さない。

修正commit `599359be236962498f93842ec66145339290380f`の手動Actions [run 37584511577](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37584511577)はPASS。macOS 15.7.9／Xcode 16.4、iPhone 16 Pro／iOS 18.5 SimulatorでRust 1.95.0 Broker build、Mobile `flutter analyze --no-pub`（No issues）、`flutter test --no-pub --reporter expanded`（21件）、`flutter build ios --simulator --debug --no-pub`が成功した。上記`tooling/minidora_live_check.py --ios-simulator ... --ios-derived-data ... --ios-result-bundle ...`から実行したnative XCTestは10 passed／0 failed／0 skipped。新規実接続1件、既存8件、日本語error code正常・不正境界1件を含む。実Brokerへの結合、Keychain保存・再読・削除、Runtime取得、離脱、失効済み資格の拒否、誤pin拒否、Owner側の招待・結合不存在を確認した。MINIDORA参照は`3400a3bb68b37efa1dc14ee8aaa28fda779bf1f8`に固定。招待秘密がXCTest出力・Broker log・durable Auditに含まれないことも確認した。この有限接続単位をCLOSEDとし、追加証拠目的では再実行しない。

証拠はSimulator native部品から実Rust Brokerへ接続した`LIVE_RUNTIME`であり、Flutter画面・native確認dialog・OS lifecycle・物理端末の成立を示さない。artifact ID `11465318529`、SHA-256 `9888bd0c3bb84b9f936cbc7131b3282796847ec474d2befd5a151c0dce696694`、remote保存期限2026-10-10T07:02:09Z。同hashのZIPをGit対象外の`release_evidence/p13-ios-native-599359b.zip`に保存した。runner追跡source差分なし、Simulator終了step成功、test KeychainとBroker招待・結合の回収を確認した。失敗runのartifact ID `11466056894`と局所LSP失敗は履歴として保持する。

main反映後の文書同期では、`python -X utf8 tooling/schema_check/check_schemas.py`（161 schema／157正常例／208負例）、`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`（236件）、`python -X utf8 tooling/release_gate_check.py --release-track windows_v1`、Manifest 1185件、`git diff --check`がPASSした。`python -X utf8 tooling/日本語基底監査.py --strict`は初回16 findingsでFAIL。今回追加した開発用helperの失敗診断1行だけを日本語化し、Python 2 fileの`py_compile`と再監査を実行した。残る15 findingsは変更対象外の旧rev3／rev4履歴各1、既存Codex CLI診断1、Android開発helper診断12で、全体exit 1を保持する（最終strict監査の`release_blocker`。現在のiOS部品接続Acceptanceは再開しない）。成功run後の実行経路変更はなく、診断文言と文書だけの更新に対するruntime再試験は行わない。失敗run・PASS runの対象commitを混同しない。

P13／Mobile `release_blocker`、通常Release `task_execution=unsupported`、`release_ready=false`は維持する。次のProduct Build単位はiOS製品UI・native service・基本lifecycleの接続であり、CLOSEDしたtransport部品を再証明するための追加fixtureや最終QAを開始しない。物理端末・配布identity・網羅的platform保証は既存release gateとして別管理する。

#### iOS Simulator build／native XCTest — 局所検証PASS（2026-10-07）

手動Actions run [#22](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37558500619)はcommit `b6dbf10c4ea8b34e103b8826f2d23f25930d8ca9`で完了した。macOS 15.7.9／Xcode 16.4、iPhone 16 Pro／iOS 18.5 Simulator上で、Flutter 3.44.0のMobile analyze（問題なし）、Flutter test 21件、iOS Simulator build、native XCTest 8件（8 passed／0 failed）がPASSした。Keychain試験はThisDeviceOnly属性、保存・再読・削除、version 1から2へのmigrationを確認した。XCTestはproduction identityを使わず、Simulator専用ad-hoc identity `-`で署名した。Workflowが明示した`FAKETEAMID.com.example.guiShellMobile`はtest用の生成identifierであり、配布署名ではない。

証拠artifact ID `11456131236`、SHA-256 `2fdeb352fd5af5f2226b595cb9d58c6ae26bb95ac6465c2168da57f4aa281513`（2026-10-10に期限切れ）。run #19（`c6de882`）は`try`漏れ、run #20（`986349b`）は`NWConnection.SendCompletion`呼出形式でcompile失敗した。run #21（`7889109`）はSimulator buildがPASSした一方、署名無効化による`errSecMissingEntitlement (-34018)`でKeychain XCTest 2件が失敗した。run #22ではad-hoc Simulator署名へ修正して全8件がPASSした。これらの履歴を消去・上書きしない。

これはSimulator上の`LIVE_RUNTIME`証拠であり、物理iOS端末、実端末Keychain、Desktop Rust Brokerとのnative LIVE_RUNTIME TLS、端末lifecycle、公開配布、release readinessを証明しない。P13とMobile release blockerはOPENのまま維持する。

#### Android current-source build／test — 局所検証PASS（2026-10-07）

commit `54229000aa50896af37e5f8d391d9a7997b52e6f`を対象にFlutter 3.44.0で`flutter analyze --no-pub`（No issues）、`flutter test --no-pub --reporter expanded`（21件PASS）を実行した。OneDrive checkout内の`flutter build apk --debug --no-pub`は既存`build/app/intermediates/assets/debug/mergeDebugAssets`の削除で停止したため、権限やtracked sourceを変更せず、同じcommitの隔離detached worktreeで`flutter pub get`とAPK buildを再実行し成功した。APKは146,353,505 bytes、SHA-256 `9292d825a140c31c4eb504084940a7b2a0b3da792c5f9c83c9502c5b12613e6b`。同worktreeで`android/gradlew.bat :app:testDebugUnitTest`も`BUILD SUCCESSFUL`。Gradle／Android SDKのKotlin DSLおよびSDK XML互換warningは出たがbuild失敗ではない。

同じAPKをWindows Android Emulator 37.2.12上のAndroid 15／API 35 Google APIs x86_64 AVD（WHPX）へinstallし、`adb install -r`が`Success`を返した。Launcher intentを1件送り、`dumpsys activity activities`で`com.example.gui_shell_mobile/.MainActivity`が`topResumedActivity`であることを確認した。起動直後の`logcat -d -t 400 -s AndroidRuntime:E Flutter:E`には該当error entryがなかった。AVD／Android user homeは`%LOCALAPPDATA%\Temp\d4p-p13-emulator-7ab8a06`配下に隔離し、repository外に置いた。これはAndroid Emulator上の製品APK起動に対する`LIVE_RUNTIME` smokeである。

この証拠はAndroid Emulator上での起動までに限る。物理Android端末、実端末Keystore／OS安全保管、Desktop Rust Brokerとの実TLS・Device Link、配布identityを証明しない。OneDrive checkout内build出力の削除失敗原因は未確定であり、環境設定を変更せずisolated worktreeを使った。Android実機試験凍結とP13／Mobile release blockerは維持する。

#### Android native Device Link基本製品経路 — CLOSED（Product Build、2026-10-07）

P13の不足である製品Flutter UI→Android native招待画面／Keystore→実Rust Brokerの基本経路を、専用EmulatorのAndroidJUnitRunnerから操作するharnessを追加した。招待は隔離Brokerのtest Ownerが発行し、native instrumentation専用のloopback接続で受け渡す。Dart、debug VM、shell引数、環境変数、一般logへ資格を流さない。`--android-native`単独指定は結合・Runtime表示・HOMEからの復帰・切断と基本的な誤pin拒否だけを対象とし、既存MINIDORA総合検査を繰り返さない。既存`--mobile-client`のPython wire検査は保持する。

Windows hostのAPI 35 Emulatorでは、製品起動を妨げるSystem UIのANR dialogを観測し、待機操作で解消した。その後の試験は結合／端末確認のBroker受理まで進んだが、応答frameの終端改行を受け取らずEOFとなる例があり、nativeの厳格読取りが拒否した。診断は隔離worktreeだけで処理段階・件数・固定例外分類を記録し、秘密・本文を出さず、診断変更は撤去した。欠落箇所の根因は未確定であり、Android正常経路PASSや製品regressionの断定はしない。Windows host経路との切分けのため、既存`android-manual-emulator.yml`へ同じnative試験を追加し、手動ActionsのUbuntu／Android Emulator上で補助検証する。自動trigger・正式identity・物理端末は使わない。P13とMobile release blockerはOPENのまま。

手動Actions [run 37578711893](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37578711893)は検証commit `7c5cd76f082ede0ee085c13aa72b634b9ecb61fe`のLinux Rust buildで停止し、Android試験は未実行だった。更新package保管処理がWindows専用`file_attributes`を無条件で使用し、Unixでは同名`MetadataExt`の`ino`／`dev`も競合していた。既存platform別link検査を共用し、portable traitを明示する局所修正を行う。Windowsのreparse拒否を保持し、Unixではsymbolic linkの保管先／packageを拒否する対象試験を追加する。これはP13を実行可能にする互換修正であり、UpdateのCLOSED Acceptanceを再開して機能拡張するものではない。

局所互換修正後、Windowsの`cargo +1.95.0 test --locked --offline --manifest-path native/rust_helper/Cargo.toml -- --test-threads=1`はcompile成功、library 497 passed／3 failed／12 ignored。変更箇所のpackage保管・一致hash削除・修復・非通常file拒否・create-only公開の5件はPASSした。全体の失敗はA2A Agent Card読取1件とUpdate TLS fixtureのConnectionReset（OS 10054）2件で、既存`FQ-TEST-LOOPBACK`の対象に残す。全Rust PASSとは報告せず、library失敗後の他targetも未実行として区別する。Schema 161／157／208、Conformance 236件はPASS。Mobile analyzeはNo issues、Desktop analyzeは依存取得後に既存deprecated info 5件だけで終了値1（既存known limitation）。

手動Actions [run 37579840231](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37579840231)（`da0b95172a7c79ae8c3fe82bdd1855f45636f6a8`）では、前記metadataのcompile errorは解消したが、Windows専用`ActiveVersionDescriptor`を参照する投影関数の`cfg(windows)`漏れ1件が残り、Android試験は未実行だった。既存Windows専用呼出元と同じ条件へ投影関数を限定し、非Windowsへ更新適用能力を追加しない。

条件分岐修正後の`cargo +1.95.0 test --locked --offline --manifest-path native/rust_helper/Cargo.toml --lib broker::update_center::tests:: -- --test-threads=1`は25 passed／0 failed。Schema 161／157／208とConformance 236件もPASSした。先の全library試験で記録した3失敗をこの対象試験の成功で上書きしない。

手動Actions [run 37580429609](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37580429609)は、commit `0ca5dfa84dc1ca68ca8f01bae8ca456e1a9a12ba`をUbuntu 24.04／Android 15 API 35 Emulatorで実行してPASSした。`cargo +1.95.0 build --locked --manifest-path native/rust_helper/Cargo.toml`、追加したUnix symbolic link拒否試験1件、Mobile `flutter analyze --no-pub`（No issues）、`flutter test --no-pub --reporter expanded`（21件）、debug APK／AAB build、APK install／起動、既存Python wire検査が成功した。`python3 tooling/minidora_live_check.py --reference "$RUNNER_TEMP/minidora-reference" --android-native --android-serial emulator-5554`では、製品Flutter UI→native招待画面→Keystore／証明書pin付きTLS→実Rust Brokerの結合、登録Runtime left／rightの表示、HOMEからの復帰、製品画面での切断、Broker側結合・招待の不存在が成立した。誤pin拒否、招待秘密がinstrumentation出力・Broker log・durable Auditにないことも確認した。正常試験1件の`LIVE_RUNTIME`証拠としてこの基本経路をCLOSEDとし、強化目的の再実行は行わない。Android／Flutterのproduction経路は変更していない。

証拠artifact ID `11464788270`、SHA-256 `facb70155cc4c2c04739e8f330517b9d89223c5e23eb8accdd5a0ed243c0fc10`。remote保存期限は2026-10-10T06:29:36Zで、同hashのZIPをGit対象外の`release_evidence/p13-android-native-0ca5dfa.zip`にも保存した。runnerの追跡差分なし、Emulator停止を確認した。ローカル試験の招待・結合・APK・ADB reverseは回収済みで、Emulatorも停止済み。一方、診断用ASCII worktree・参照clone・一時AVD directoryの削除commandは実行側policyに拒否され、これらの停止済み作業資材は残っている（`known_limitation`、製品releaseを阻害しない。削除権限を迂回しない）。診断用source差分は撤去済みで、未配布の診断buildを製品証拠へ使用しない。

Windows host側で観測した終端欠落の根因、物理端末、iOS native実Broker接続、全lifecycle timing、配布identity、release readinessはこのPASSから推論しない。既存Mobile `release_blocker`、Android実機凍結、通常Release `task_execution=unsupported`、`release_ready=false`を維持する。次はP13のiOS native実Broker接続の不足だけを施工する。

## Q0 品質保証凍結 — 完了（過去のFinal QA記録）

2026-10-07、P12 Product Build受入れcommit `ab746d4b33b003761f5ddcef60a88afa97132a40`をFinal QA対象製品状態として固定した。対象commitは`main`へpush済みで、Q0記録時点のremote `main`と一致。対象はWindows Feature Complete製品構成であり、release-readyや正式配布可能状態を意味しない。以後のQ1〜Q7 evidenceはこの凍結commitを対象とし、製品コード修正が必要になった場合は修正commitを新たなQA候補として明示し、対象を再凍結する。

## Q1 Codex横断品質検査 — 完了（過去のFinal QA記録）

2026-10-07、凍結製品のAgent Task／Compare／Handoff／Workspace／MCP／Provider／Host／Export／Installer／Update／Rollback横断を既存全Rust targetとDesktop／Mobile Flutter testで確認した。`cargo test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets --no-fail-fast -- --test-threads=1`最終runはexit 0: Rust lib 499 passed／12 ignored、bin 10 passed、Broker IPC 10 passed、その他hash／checkpoint／保護保管／Workspace integration 26 passed、production Agent Task E2E 1 ignored（総計545 passed／13 ignored／0 failed）。Ignoredは明示的な実Agent／Owner-dialogue専用testで、production Agent Taskの一般Release capabilityを有効にしない。

Desktop `flutter test --no-pub --dart-define=GUI_SHELL_PRODUCT_VERSION=1.2.3`は204 passed、Mobile `flutter test --no-pub`は21 passed。OneDrive日本語pathからMobile analyzerを起動した初回はLSP JSON `FormatException`で停止したため、同一sourceのASCII一時checkoutで`flutter analyze --no-pub`を実行し`No issues found`を確認した。Desktop analyzerはexit 1で、既存`agent_center.dart`のFlutter 3.44 deprecated API info 5件だけを報告。これをknown limitationとして`RELEASE_CHECKLIST.md`に分類し、現行APIが削除される前にRadioGroup／initialValueへ移行する。Desktop Release buildは同じsource-equivalent一時checkoutで成功し、実行artifact SHA-256 `731da778f1c61e2d6597162876b79e604503ea511199f3d7ef94cc479919709d`、size 162304 bytes。Temporary output directory由来のMSB8029 warning 7件を記録。これはcompile evidenceであり、installed product、formal provenance、配布可能性の証拠ではない。

Q1初回Rust runで既知localhost fixtureのConnectionReset 3件、次runでA2A fixture 1件を観測したが、該当fixture単独testはPASSし、fixture競合の後の`--no-fail-fast`全target runは0 failureで完了した。これは間欠test fixture揺らぎを解消した証拠ではないため`FQ-TEST-LOOPBACK`へ保持し、Q2 network/fault調査で扱う。installed end-to-end、実Provider資格、正式証拠、Q2以降の障害／Recovery、release blockersは未成立のまま維持する。

## Q2 障害・復旧 — 過去の検証記録（現行modeでは延期）

以下は2026-10-07までに実行した当時のQ2検証履歴である。観測結果と範囲は保持するが、現行Product Buildのactive taskや完了条件として扱わない。

2026-10-07の最初のQ2検査単位では、実Codex CLI `0.160.0`を資格情報なしのloopback偽Responses APIへ接続するignored LIVE_RUNTIME testを修正・実行した。Task完了、MxC実行中TEMP markerのBroker親可読、CLI終了後markerの不在、取消後のterminal化、child heartbeat停止、取消後marker不在、Broker管理WorkspaceTaskScratch不在を確認し、修正後のtestは1 passed。実Network model／資格情報は使っていない。MxC／Owner／Audit以外はtest fixtureのin-process Brokerと合成Owner callbackであり、installed Flutter→native Owner→production IPC、永続Audit、deadline／crash recoveryを証明しない。TEMPの観測はこのchild markerの範囲だけで、外部component内の物理削除保証ではない。

同じsourceのBroker control→fake Codex fixtureは1 passedで、正常終了／cancel／deadline後に子孫processが稼働を継続しないこと、cancel／deadline terminal後にWorkspaceTaskScratchを残さないことを確認した。永続`AgentTaskScratchJournal`のfocused testは8 passedで、再起動後の一致scratch回収、不一致／identity変更時の保持、journal改竄拒否を確認した。これらは`FIXTURE` evidenceである。

現行sourceの全target逐次実行`cargo test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets --no-fail-fast -- --test-threads=1`はexit 101。Rust library 511件は496 passed／3 failed／12 ignored、他targetは全て成功し、全体は542 passed／3 failed／12 ignored。失敗はCodex loopback responseのConnectionReset 1件とUpdate Download TLS fixtureのWSA 10054／ConnectionReset 2件で、失敗時にHTTP応答本文0／22514 byte、またはTLS request送信失敗を観測した。Update Download focused 17件は初回PASSしたが、同2件の反復ではround 2に再発し、単一testの反復ではround 10に`InvalidContentType`とserver側ConnectionResetを再発した。fixture root causeは未確定で、Q2 network failure項目`FQ-TEST-LOOPBACK`はOPENのまま維持する。loopback不安定性を製品故障とも環境だけの問題とも断定せず、影響のない製品コード変更で隠さない。

この検査単位はTask cancellation／scratch処理の限定LIVE_RUNTIME・FIXTURE evidenceとtest assertion修正を記録するもので、FQ-R2-A/B/Fを閉じない。production installed path、deadline／Codex・Broker・Launcher crash回復、Approval非再利用、durable Audit、OneDrive／NTFS統合境界、result projection、通常Release capabilityは引き続きQ2以降の未成立範囲である。通常Release `task_execution=unsupported`とrelease gateは変更しない。

2026-10-07 Windows hosted Rust補助検査の初回run #39は`workflow_dispatch`でcommit `de3be16e808ba42ea39d0456046cb4e1f5c355a4`をWindows runner `win25-vs2026/20260925.250.1`／Rust 1.95.0上で実行したが、Rust試験開始前の整形gateがexit 1となり、全target試験・Release helper build・IPC smokeは未実行だった（[run #39](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37505161510)）。workflowが明示検査する8 fileのうち、`codex_cli.rs`、`dialogue.rs`、`desktop_launcher.rs`、`fake_codex_cli.rs`に現行Rust 1.95.0の整形差分があったため、この4 fileだけを同版`rustfmt`で機械整形した。workflow指定と同一の8 file整形検査は全てPASS。これは検査入口を通す書式統一であり、Task／loopback挙動や`FQ-TEST-LOOPBACK`の合否を証明しない。

整形後、同じ実Codex CLI限定LIVE_RUNTIME testをWindows上で再実行した。最初の呼出しはtest必須環境変数を欠き、test body開始前に失敗した。絶対pathのCodex CLIを指定した正しい呼出しは24.65秒で1 passedとなり、前記のTask／TEMP／cancel／scratch観測を再現した。これは前記の同じ限定範囲を補強するだけで、installed productやQ2全体の完了証拠ではない。

整形commit `97c30fac8eb7d51fcaa3b341877c4a93f5ad84c8`に対するWindows hosted run #40は全step PASS（17分20秒job／17分24秒run、runner `win25-vs2026/20260925.250.1`、Rust 1.95.0）。Rust全targetは13個の結果block合計545 passed／0 failed／13 ignored。Release Broker helper buildもPASSし、SHA-256は`4390f8c7a248c655d9bdcc7070f893e3eb0b0cf56c9408ef3a8e045e8427de46`。実Broker process／通常IPC smokeはnon-synthetic collector v6でPASSし、認証loopback IPC、durable-store readiness、通常Task grant denial 2件、restart replay拒否、fresh health、forced-stop fail-closed、session file cleanupを観測した。runner上の検査物cleanupとclean worktree確認もPASS、artifact uploadなし（[run #40](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37507251175)）。これは対象commit上のhosted Windows全target／standalone Release Broker smoke証拠であり、installed Desktop product、native Owner操作、実Agent Task、正式release evidence、release readinessの証明ではない。直前のローカル全targetで起きたloopback fixture 3失敗との原因差は確定していないため`FQ-TEST-LOOPBACK`はOPENのまま維持し、通常Release `task_execution=unsupported`と既存release gateを変更しない。

2026-10-07 Q2 Broker強制終了後の復旧試験: `cargo +1.95.0 test --locked --offline --manifest-path native/rust_helper/Cargo.toml --lib broker::protocol::tests::broker強制終了後の別process起動登録で永続scratchを監査付き回収する -- --exact --nocapture --test-threads=1`は1 passed／0 failed。子試験プロセスが永続journalへ合成稼働Taskと有効化済みscratch領域を記録した後に強制終了し、別のBroker実体が同じstore／Workspaceを開いて中断Taskの監査記録を保持し、現在の登録bindingと一致するscratch領域だけを回収し、回復結果を監査記録することを確認した。これは`FIXTURE`（Brokerライブラリを実行する試験プロセス）で、製品`broker-server`／IPC、導入済みUI、過去Approvalの非再利用、子孫プロセス停止を証明しない。既存試験自身も製品サーバーの証拠ではないと明記する。Q2は継続し、通常Release `task_execution=unsupported`とrelease gateを維持する。

同日、`FQ-TEST-LOOPBACK`の原因切分けとしてUpdate Download TLS test fixtureだけを一時変更し、接続先をIPv4へ固定して接続resetを拒否するaccept loopを追加した。focused TLS testの反復は30回中6失敗、別diagnostic反復は10回中3失敗となり、改善・原因確定に至らなかった。追加したplaintext-probe試験は1回通過したが、この変更は検証用に限り、失敗を隠す可能性があるため採用せず全て破棄した。working treeはcleanへ復帰。`FQ-TEST-LOOPBACK`はOPENのまま扱い、製品原因／host原因のいずれとも断定しない。

Windows hosted補助run #41は`workflow_dispatch`による手動実行でcommit `2bb50217c02fcf6dd6cf757c43f126c6a2f1f1f8`（当時の`main`先端と同一）を検証し、全体10分59秒で全工程が成功した。workflow指定Rust sourceの書式検査、Rust全target 13集計欄（545件成功／0件失敗／13件除外）、Broker実行体のRelease build、Broker独立起動と通常IPCの簡易疎通確認、実行環境内の検査用一時物の後片付け、作業tree清掃確認がすべて成功。生成物のアップロードなし（[run #41](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37518774824)）。これは当該commitのWindows hosted Rust検査／Broker簡易疎通確認の証拠であり、導入済み製品、実Agent Task、正式リリース証拠、リリース可能性の証明ではない。ローカルで観測したloopback fixture failureの根因は未確定のため`FQ-TEST-LOOPBACK`はOPENのまま維持し、`task_execution=unsupported`、既存release gate、`release_ready=false`を変更しない。検証用branch `codex/q2-windows-rust-2026-10-07`は対象commitが`main`と同一であることを照合後、local／remote双方から削除した。

文書同期後の統合確認`python -X utf8 tooling/validate_all.py --python-only --desktop-platform=windows`は終了値1。登録済み10検査のうち、Schema 161／157／208、Conformance 236、`manifest_check`、`release_gate_check`、`packaging_portability_check`、`release_smoke`、`evidence_bundle`、`release_runtime_assertions`、`final_development_audit`の9件は合格し、`japanese_base_audit`のみ既知指摘により終了値1となった。既知指摘は旧履歴`docs/REV3_PROGRESS.md`と`docs/REV4_ACCEPTANCE_LEDGER.md`、現行source `native/rust_helper/src/adapters/codex_cli.rs`に各1件。今回更新した`ROADMAP.md`、本書、`docs/FINAL_QA_QUEUE.md`に新規指摘はない。旧履歴は変更せず、Q2とrelease gateの状態も変更しない。

2026-10-07 Q2 Broker crash回復の限定LIVE_RUNTIME検証: feature専用Broker serverを実processとして起動し、認証済みloopback IPCを通じて実Codex CLI `0.160.0`／MxC childへ、実資格情報なしのlocalhost偽Responses APIから合成Task tool callを渡した。synthetic Owner fixtureはTask Workspace Permission／Task Approvalだけに固定し、MCP Tool実行・結果表示Approvalを拒否した。Task Permission／Approval付与監査は`FIXTURE`として記録した。実tool call後にBrokerをhard killし、実child heartbeat停止、Task scratchのBroker再起動前保持とWorkspace再登録時回収、永続Audit上の中断Task／回復記録、旧IPC資格拒否、新Permissionだけでは旧Approvalを再使用できないこと、tool result／完了writeなしを確認した。`cargo +1.95.0 check --locked --offline --features r2-e2e --all-targets`とfeatureなし`cargo +1.95.0 check --locked --offline --no-default-features --all-targets`は双方warningなしでPASS。実E2E focused testは1 passed、native Owner result-approval否定回帰は1 passed、Schema 161／157／208とConformance 236 checksもPASS。対象は実Broker／Codex／MxC processのBroker-hard-kill分岐に限る。synthetic Owner、偽API、試験専用build／Workspaceを使い、installed Desktop UI・native Owner操作、通常Release、Launcher crash、Codex CLI親だけのcrashを証明しない。よって`FQ-R2-B`／`FQ-R2-F`はOPENのまま維持し、`task_execution=unsupported`とrelease gateは変更しない。現行Q2検収を延長せず、本証拠を該当条件の一局所所見として追加する。

2026-10-07 Q2 Codex root異常終了の限定LIVE_RUNTIME検証: ignored test `codex_root_crash_keeps_broker_alive_stops_descendants_recovers_task_and_does_not_reuse_approval`はWindowsで1 passed／0 failed。実Codex CLI `0.160.0`の新規processを、試験Broker PIDを親に持つことと指定実行体の完全path一致で一意に特定し、そのrootだけを終了した。Broker processの存続、実MxC child heartbeatの終了後停止、Taskの`failed`終端、D4-owned scratch回収、完了marker／tool resultなしを確認。永続Audit storeを閉じて再読込しchainを検証、Task本文がAuditにないこと、terminal recordが`failure_class=通信失敗`であることを確認した。Task／Permission／Approvalは専用試験Owner fixtureで付与し、Tool／APIはlocalhost偽Responses API、Workspaceは一時隔離領域で、実資格情報・実課金・外部APIを使用していない。消費済みApprovalは新しいWorkspace Permissionだけでは再利用できなかった。最初の2実行はtest oracleの欠陥（heartbeat比較基準の固定、Status投影に存在しないfailure field参照）で失敗し、実装挙動のFAILとは区別してassertionを修正した後の正確な実行は1 passed。検証command: `cargo +1.95.0 test --locked --offline --features r2-e2e --test agent_task_crash_recovery_e2e -- --ignored --exact codex_root_crash_keeps_broker_alive_stops_descendants_recovers_task_and_does_not_reuse_approval --nocapture`、Rust test compile check、同fileのscoped rustfmt check。installed UI／native Owner操作、通常Release、Launcher crash、正式配布環境は対象外。これは`FQ-R2-B`／`FQ-R2-F`のCodex親process分岐だけの局所証拠であり、両queue全体を閉じず、通常Release `task_execution=unsupported`とrelease gateを維持する。

2026-10-07 Q2 active Task中断の限定LIVE_RUNTIME検証: 実Codex CLI `0.160.0`／MxC child、feature専用Broker process、認証済みloopback IPC、資格情報なしlocalhost偽Responses APIを使うcancel／deadline試験が各1 passed。Owner取消はIPC要求後も子孫終了までTaskをterminal化せず、test-only deadline override `GUI_SHELL_R2_E2E_TASK_EXECUTION_LIMIT_MS=20000`は期限超過で停止した。両経路でprocess treeとchild heartbeat停止、`cancelled`／`failed`終端、result hash／完了markerなし、WorkspaceTaskScratch回収、永続Audit再読込、新Workspace Permissionだけでは消費済みApprovalを再利用できないことを確認した。Permission／Approval grantはsynthetic Owner `FIXTURE`、process／実CLI／MxCの観測だけを限定`LIVE_RUNTIME`とする。併せてBroker hard-killとCodex root crashの既存2試験を同じ実行で再通過し、4試験合計4 passed／0 failed。検証command: `cargo +1.95.0 test --locked --offline --features r2-e2e --manifest-path native/rust_helper/Cargo.toml --test agent_task_crash_recovery_e2e -- --ignored --test-threads=1`（`GUI_SHELL_CODEX_TASK_CRASH_E2E_CLI`にCodex CLI絶対pathを設定）。

検証補足: MxC互換shell probeの既存unit assertionと`.NET [IO.File]`直呼出しが矛盾していたため、heartbeat操作を`Set-Content`／`Add-Content`へ揃え、対象unit testは1 passed。featureなし／`r2-e2e`ありの全target `cargo check`、逐次Rust全target test（545 passed／0 failed／13 ignored）、Schema 161／157／208、Conformance 236 checks、対象3 Rust fileのscoped `rustfmt --check`はPASS。逐次全targetの途中で既知Update TLS loopback testがOS error 10054を一度返したが、単独再実行および最終全target runはPASS。loopback fixtureのroot causeは未確定であり`FQ-TEST-LOOPBACK`はOPENのまま。installed Desktop UI／native Owner、Launcher crash、通常Release、正式provenanceは未検証のため、`FQ-R2-A/B/F`全体を閉じず、通常Release `task_execution=unsupported`とrelease gateを維持する。

併用検査: `cargo +1.95.0 check --locked --offline --features r2-e2e --all-targets`と`cargo +1.95.0 check --locked --offline --no-default-features --all-targets`はwarningなしでPASS。対象testのscoped `rustfmt --check`、`git diff --check`、Schema 161／example 157／negative fixture 208、Conformance 236、`manifest.py --write`後の`manifest.py --check`（1181 files）、`release_gate_check.py --release-track windows_v1`もPASS。厳格日本語監査は終了値1だが、新規findingはなく、既存3件（旧rev3／rev4履歴と未変更Codex CLI診断文字列）のみ。

同差分の検査記録: test内panic説明14箇所を日本語化した後、`tooling/日本語基底監査.py --strict`の新規findingは0となり、残る3 findingは既存の`docs/REV3_PROGRESS.md`、`docs/REV4_ACCEPTANCE_LEDGER.md`、未変更`native/rust_helper/src/adapters/codex_cli.rs`に限られた。初回`validate_all.py --python-only --desktop-platform=windows`はmanifest更新前かつ日本語監査修正前に実行したため、監査findingとstale manifestを報告した。その後、`tooling/manifest.py --write`で1180 fileを再記録し、修正後の同統合validatorを再実行した。Schema、Conformance、manifest、release gate check、packaging portability、release smoke、evidence bundle、release runtime assertions、final development auditはPASSし、終了値1の唯一の工程は日本語基底監査で、前記3件の既存findingだけが残る。`release_gate_check`の検査自体はPASSだが`release_ready=false`であり、既存31 release blockerは維持する。

2026-10-07 Q2 Provider HTTP 503後のTask停止・復旧の限定LIVE_RUNTIME検証: ignored test `provider_http_503_after_active_codex_task_stops_descendants_and_recovers_scratch`をWindowsで実行し1 passed。実Codex CLI `0.160.0`／MxC childと`r2-e2e` feature専用Broker processを認証済みloopback IPCで接続し、資格情報なしlocalhost偽Responses APIは実tool result受信後に合成HTTP 503を返す。Taskは`通信失敗`で終端し、子孫process／heartbeat停止、WorkspaceTaskScratch cleanup、result hash／完了markerなし、file-backed Audit再読込、Workspace Permissionだけでは消費済みApprovalを再利用できないことを確認した。5つのactive-task LIVE_RUNTIME failure試験（Broker kill、Codex root crash、cancel、deadline、Provider 503）は一括実行で5 passed／0 failed。実process観測を限定`LIVE_RUNTIME`、Owner grantとAPI応答を`FIXTURE`とし、外部Provider・実資格・installed Desktop UI・通常Release・release readinessは対象外。通常Release `task_execution=unsupported`とrelease gateを維持する。

同差分のfeature-enabled全target Rust試験は499 passed／1 failed／12 ignoredで終了値101。唯一の失敗は既知loopback競合項目`broker::a2a_center::tests::owner接続をBrokerで受理し通常IPC一覧へbounded射影する`のAgent Card応答接続resetで、同一testの単独再実行は1 passed。これは`FQ-TEST-LOOPBACK`を解消せず、根因も確定しない。featureなし／`r2-e2e`ありの全target compile check、Provider 503 E2E、既存4 failure E2E、Rust bin／Broker IPC test、Schema／ConformanceはPASS。Q2 acceptance行とrelease gateは変更しない。

2026-10-07 Q2 active AgentTaskのIPC request replay拒否 — 限定LIVE_RUNTIME: cancel／deadline／Provider failureの3実Agent E2Eで、同じAgentTask request envelope（request ID／nonce／payloadを含む同一JSON）を認証済みloopback IPCへ2回送る。初回は`running`として受理し、再送は`broker_replay_detected`で拒否され、その拒否AuditがBroker永続store再読込後も同一request ID／operation／decision／reasonで残ることを確認した。各caseで実Codex rootは一意、5本のactive-task interruption E2E（Broker kill、Codex root crash、cancel、deadline、Provider 503）は合計5 passed／0 failed。Owner grantとResponses APIは`FIXTURE`、process／Broker／IPC観測は限定`LIVE_RUNTIME`。このtest-only変更はstale IPC／消費済みApprovalの既存検査を置き換えず、Q2 queue全体を閉じない。通常Release `task_execution=unsupported`とrelease gateを維持する。検証: `cargo +1.95.0 check --locked --offline --features r2-e2e --test agent_task_crash_recovery_e2e --manifest-path native/rust_helper/Cargo.toml` PASS、対象ignored E2E 5件 PASS、Rust 1.95.0 scoped rustfmt PASS。全Rust all-target testはこのtest-only差分では再実行しておらず、先行runのloopback A2A failureは`FQ-TEST-LOOPBACK`として未解決のまま保持する。

2026-10-07 Q2 改竄WorkspaceTaskScratch journalによるBroker起動fail-closed — 限定LIVE_RUNTIME: 実Codex CLI `0.160.0`／MxC childをfeature専用Broker processと認証loopback IPCでactive Taskとして起動し、Broker hard kill後に子孫process停止と孤立scratch保持を確認した。試験fixtureが永続journalのTask bindingだけを改変し既存HMACを保持した状態で実`broker-server`を再起動すると、HMAC認証失敗の診断を出して非0終了し、通常／Owner IPC endpointを生成しなかった。改竄journalのbyte列と孤立scratch directoryはそのまま保持され、tool result／Workspace完了markerは存在しない。これはHMAC改竄fixtureを用いる限定LIVE_RUNTIMEで、installed製品、電源断、正式Audit signature、通常Release capabilityを証明しない。改竄時に孤立scratchを削除せずBroker起動を止めるfail-closed挙動がこの条件の結果であり、`FQ-R2-B`／`FQ-R2-F`全体はOPENのまま、`task_execution=unsupported`と既存release gateを維持する。検証: `cargo +1.95.0 check --locked --offline --features r2-e2e --test agent_task_crash_recovery_e2e --manifest-path native/rust_helper/Cargo.toml`、改竄journal test 1 passed、通常Broker crash回復の直接依存test 1 passed、Rust 1.95.0 scoped rustfmt check。

同差分の統合`python -X utf8 tooling/validate_all.py --python-only --desktop-platform=windows`は10登録検査中9件がPASSし、終了値1の唯一の検査は厳格日本語監査だった。findingは変更外の`docs/REV3_PROGRESS.md`、`docs/REV4_ACCEPTANCE_LEDGER.md`、未変更`native/rust_helper/src/adapters/codex_cli.rs`に各1件で、今回差分由来のfindingはない。Schema、Conformance 236 checks、Manifest、Release Gate、packaging portability、release smoke、evidence bundle、release runtime assertions、final development auditはPASS。通常Release `task_execution=unsupported`と`release_ready=false`は維持する。

2026-10-07 Q2 Launcher異常終了時の画面process群停止 — 限定LIVE_RUNTIME: ignored test `desktop_launcher::tests::launcher_crash_stops_frontend_and_descendants`はWindowsで4回連続PASS。隔離owner process内でproduction `launch_frontend`を実行し、test-only frontendと孫processをJob Objectへ登録した後、ownerを強制終了。frontendと孫processの両方が5秒以内に終了し、試験後にfixture process／directoryの残留なし。process tree停止とproduction `launch_frontend`接続は`LIVE_RUNTIME`、fake frontendとBroker workerは`FIXTURE`。これはLauncher crash時の画面子孫停止だけを確認し、active Agent Task、Broker永続Audit／Recovery、installed product、Launcher自体を含むTask全体の復旧を証明しない。`FQ-R2-B`／`FQ-R2-F`はOPENのまま、通常Release `task_execution=unsupported`とrelease gateを維持する。検証: feature-enabled／featureなしのRust全target `cargo check`、focused crash E2E 4回、scoped Rust format、`git diff --check`、Schema 161／157／208、Conformance 236、Manifest、Windows release gate check。strict日本語監査は既存の変更対象外3指摘により全体exit 1だが、今回変更したsource／文書にfindingなし。

2026-10-07 Q2並行Task scratch分離 — Adapter経路FIXTURE: Windows `adapters::codex_cli::tests::同一Workspaceの並行Taskはscratchとcleanupを分離する`は1 passed。異なるTask IDを持つ二つの同時`CodexCliAdapter::AgentTask`へ同一Workspaceと共有journalを与え、別scratch directory／agent識別marker、Agent A完了時のAだけのcleanupとB scratch保持、Agent B完了後の両scratch不在を確認した。production Adapter／WorkspaceTaskScratch処理を通るがCLIはtest fixtureであり、実Codex／Compare UI／installed product／Broker IPC／agent内部状態の分離保証ではない。したがって`FQ-R2-C`はOPENのまま。検証: focused Rust test 1件、Rust 1.95 scoped format、`git diff --check`、Schema 161／157／208、Conformance 236、Manifest 1182 files、Windows v1 Release Gate check。strict日本語監査は3 findingsでexit 1（旧rev3／rev4履歴文書と、変更外の既存Codex CLI PowerShell診断文字列`$ErrorActionPreference='Stop'`）；今回追加したtest／fixture／文書にfindingなし。通常Release capabilityとrelease gateは変更しない。

同じ並行scratch差分の直接依存回帰: `cargo +1.95.0 test --locked --offline --manifest-path native/rust_helper/Cargo.toml --lib adapters::codex_cli::tests:: -- --test-threads=1`は23 passed／0 failed／2 ignored。Taskのscratch creation／drop cleanup、期限超過後の子孫停止、secret／hardlink拒否、Model／Provider境界を含むAdapter test群を逐次確認した。ignored 2件は明示的な実Codex CLI資格・隔離検証で、今回実行していない。fixture suiteはQ2-Cのinstalled／実Agent間隔離証拠ではなく、`FQ-R2-C`をOPENのままとする。

## 1. 工程方針

rev5は、完成前の製品機能開発と、Feature Complete後の最終品質保証を分離する。旧R2 Agent TaskはPRODUCT BUILD上`FUNCTIONALLY ESTABLISHED`、P1は`DONE FOR PRODUCT BUILD`とする。R2で得た実Task・Permission／Approval・Audit・Workspace境界等の証拠は保持する。R2-A〜Hの追加探索・出荷級の再確認はP2以降を止めず、必要なものを`docs/FINAL_QA_QUEUE.md`へ送る。

これはR2-A〜Hの全項目がPASSした、通常Release capabilityが有効になった、またはrelease-readyであるという意味ではない。通常Releaseの`task_execution`および明示release gateは現状のfail-closed状態を維持する。P2の検証は、実資格・実課金・外部副作用を使わない隔離試験経路で行う。通常Owner操作を待ち条件にせず、テスト専用identityまたはfixtureを使うが、実際のnative Owner確認経路を自動承認する回避策は作らない。

`docs/REV4_ACCEPTANCE_LEDGER.md`はrev4時点の履歴として保持する。そこに記録されたPASS／CLOSED、OPEN、FAILをrev5の受入れ結果へ転記・改変しない。

rev5工程表は段階を`P0`〜`P13`、同rev5実装指示書は次のCompareを`R3`と表記する。両者のCompare機能範囲は一致するため、本進捗では工程表の`P2`を現行phase名として使い、`R3`を同一範囲の別表記として記録する。

## 2. 現在の製品工程

### P0 基盤 — 既成立部分を再利用

Rust Broker、Authority、Permission、Approval、Audit、Runtime／Workspace登録、Desktop shell、Launcher、IPCを再構築しない。

### P1 Agent Task — 製品開発上は完了

成立済み正常Task経路を利用する。追加crash matrix等の深掘りをP2の開始条件にしない。rev4 ledgerの未完了・失敗履歴は保持し、統合後の検査対象をFinal QA queueへ送る。

### P2 複数Agent比較 — CLOSED（Product Build）

対象は同一Codex CLI／同一providerを使う2つのAgent instanceである。Agent A/Bは別のSession、Workspace、Task、実行scratchを持ち、同一Taskを独立に実行する。Vendor／Modelの種類が揃うことを待たない。

| P2受入れ条件 | 状態 | 現在確認できた実装 |
| --- | --- | --- |
| Agent A／Bを独立instanceとして用意 | PASS | Agent CenterがBrokerの別Runtime／Workspaceに結合したactive SessionをA/Bとして選択する。同じCodex CLI／providerを別Runtime IDで登録できる。Broker契約上Runtime登録はWorkspaceへ結合される |
| 同一Taskを独立Workspace／Session／Taskへ投入 | 合格（証拠区分: `FIXTURE`＝試験用模擬） | 同じ入力hashを別要求として事前検査し、Permission／Approval／Task状態をSessionごとに分離するWidget試験 |
| 2つのTaskを同時実行 | 合格（証拠区分: `FIXTURE`＝試験用模擬） | Compare調停serviceが両Taskを並行dispatch。2件の実行要求と別Task IDをWidget試験で確認 |
| result／diff／tests／duration／failure／resource／Auditの比較表示 | PASS（Product Build範囲） | Task状態・result hash・Audit参照をA/B別表示。結果本文は個別Content Exposure後に表示、変更file／diffは独立Workspace Inspector、Agentのtest報告は未検証の結果本文として扱い、独立test recordとresource実測値は`unknown`、時間はUI観測値と明示 |
| 比較結果の選択 | 合格（証拠区分: `FIXTURE`＝試験用模擬） | completed候補だけ選択でき、選択は権限を生成しない |
| 選択した結果だけを適用 | 合格（証拠区分: `FIXTURE`＝試験用模擬） | 選択元のfull結果を未信頼入力としてA/Bと別Workspaceのtargetへ新しいAgent Task要求で渡す。target Permission／Approvalを新規取得し、直接file writeは行わない |
| 片側failure／cancelの相互隔離 | 合格（証拠区分: `FIXTURE`＝試験用模擬） | start outcomeをAgentごとに保持し、片側failureを他方へ伝播せず、cancelは各Task IDに個別送信 |

P2は2026-10-05、Agent CenterのWidget正常経路試験で、同一Taskの独立事前検査、別個のPermission／Approval、二重Taskの並行開始、完了結果とAudit参照、結果選択、別Workspaceへの新規Task準備を一度成立させたため閉鎖する。rev5は製品構築工程で試験専用Owner識別子、試験用模擬経路、内部開発経路を認め、通常Releaseの`task_execution=supported`や導入済み製品上の実Codex横断証明をP2のgateにしていない。選択はAuthorityを生成せず、適用先は新しいWorkspace Permission／Task Approval／Auditを通る。比較間でAuthorityは共有・移送しない。

2026-10-05のCompare実装更新: Agent Centerからactiveな別SessionをA/Bに選択し、同一Taskを別Sessionへ事前検査し、各Permission／Approvalを独立取得した後に2 Taskを並行起動できる。Task状態・result hash・終端Auditを比較表示し、completed候補だけを選択する。選択結果の適用はfull Content Exposureを別途承認後、A/Bから独立したtarget Sessionへ新Taskとして渡す。target側は通常のBroker Workspace Permission／Owner Approvalを別途取得する。選択済みsource結果は未信頼データとしてTask本文へ格納し、Authorityを移送しない。UI／Flutterはfileを書き込まない。

差分はA/Bそれぞれの登録Runtime／Workspaceに絞ったWorkspace Inspectorから別Approval・基準点で確認する。時間値は開始要求からterminal状態観測までのUI時計値のみで、Broker実行時間ではない。Agent test報告は未検証の結果本文、独立test recordとresource実測値は`unknown`。Widget／service試験は`FIXTURE`であり、実Codex CLI・通常Release capabilityを通した`LIVE_RUNTIME` Compareや製品releaseの証拠ではない。これらの統合・installed確認は`docs/FINAL_QA_QUEUE.md`へ送り、P2を再開しない。

検証済み: Desktop `flutter analyze --no-pub`、Compare調停service試験5件、Compare／選択／適用Widget試験1件、Desktop全test 162件、Windows Desktop debug build、Mobile `flutter analyze --no-pub`、Schema検査153 schema／153 example／197 negative fixture、Conformance 232 checks。日本語基底監査は終了コード0。今回追加分に関する指摘は表示語・comment・test文言を日本語化した後の再監査で0件。残る2件は旧rev3／rev4履歴文書だけの過去記録であり、rev5の現行Acceptanceへ転記・書換えしない。Flutter検証はASCII一時cloneで実行し、OneDrive上のFlutter Analyzer LSP障害と区別した。Windows buildは成功し、一時directory出力に関するMSB8029警告だけを記録した。Rust sourceは変更していない。

### P3 エージェント引継ぎ — CLOSED（Product Build）

| P3受入れ条件 | 状態 | 現在成立した実装 |
| --- | --- | --- |
| sender／receiver Agentを別Session・Runtime・Workspaceへ結合 | PASS | Agent Centerは完了Taskを持つ送信Sessionと、未使用で独立した受信Sessionだけを候補にする |
| Task contextと承認済みresultを転送 | PASS | 元Taskへ送った明示instructionと、Broker Content Exposureでnative Ownerが`full`を許可した同一result/hashをbounded packageへ含める |
| artifact／changed files／diff／testsを転送 | PASS | 承認済みresult本文内の厳密JSON形式を検証して抽出し、artifact内容・相対path・diff・test申告をpackageへ含める |
| Permission／Approval／Credential／Authority／Trust／hidden stateを非継承 | PASS | package SchemaはAuthority fieldを禁止し、非継承flagを固定。受信側は別Task要求としてBroker事前検査・新規grant経路へ進む |
| Handoff receiptとAudit参照 | PASS | 受信Taskのstart responseからHandoff receiptを生成。Audit IDは受信Task開始Auditへの参照であり、独立Handoff Broker operation／Auditではない |

2026-10-05、Agent Center Widget試験で送信元TaskをBroker fixture経路から完了し、resultのfull Content Exposureを取得、Handoff公開内容をpreviewした後、異なるRuntime／Session／Workspaceへ新TaskとしてBroker事前検査した。受信側のWorkspace PermissionとTask Owner Approvalを別途取得し、Taskを開始、開始Auditを参照するreceiptを表示した。要求payloadに旧grant ID／credential／authorityを含まないことも確認した。service試験は正常package生成、未完了・full未承認・hash不一致・同一Runtime・Authority混入を拒否した。Schema／fixtureとConformanceも接続した。証拠は`FIXTURE`であり、実Codex間・通常Release・installed productのHandoff証拠ではない。

Handoff受入れは有限条件を満たしたため閉鎖する。厳密なAgent result JSON形式でない出力、32,768-runeを超える受信Task package、agent-reported artifact/diff/testの独立検証、別個のHandoff Broker Audit operationは本Acceptance外であり、既存scopeを拡張しない。Agent申告は受信側へ未信頼として渡され、実行済みtestの証拠へ昇格しない。CompareのCLOSED条件は再試験・変更していない。

検証結果: `python tooling/schema_check/check_schemas.py` PASS（schema 154件／正常例154件／負例198件）、`python tooling/conformance_tests/run_conformance_skeleton.py` PASS（適合確認232件）、Desktop `flutter analyze --no-pub` PASS、Desktop全165試験 `flutter test --no-pub` PASS（Task context寿命制御を加える前に一度実行）、引継ぎ処理の単体試験2件PASS（未完了／full未承認／hash不一致／同一Runtime／不正相対path／上限超過を拒否）、引継ぎ正常系の画面試験1件PASS（最終表示語変更・Task context寿命制御後に再実行）、寿命制御変更後のDesktop `flutter analyze --no-pub`もPASS、携帯版 `flutter analyze --no-pub` PASS、Windows `flutter build windows --debug --no-pub` PASS、`python tooling/manifest.py --check` PASS（対象file 1142件）、`git diff --check` PASS。Flutter試験・buildは英数字のみの一時作業複製で実行し、OneDrive日本語pathで既知のFlutter静的解析問題と分離した。Windows buildはMSB8029（一時directory中間fileに関する警告）を出したが実行fileを生成して終了コード0。

`python -X utf8 tooling/日本語基底監査.py --strict`は終了コード1。今回の変更fileに指摘はなく、残る2 findingは既存履歴`docs/REV3_PROGRESS.md`と`docs/REV4_ACCEPTANCE_LEDGER.md`各1件のみ。過去記録を改変せず保持する。従ってstrict監査全体はPASS扱いにしない。

### P4 提供元・模型管理（Provider / Model Center）— CLOSED（Product Build）

| P4受入れ条件 | 状態 | 成立範囲 |
| --- | --- | --- |
| 提供元registry／選択 | PASS | 実装済みの1経路 `openai_codex_cli`（OpenAI via Codex CLI）をAgent Centerから選択・登録する。全Vendor選択は要求しない |
| 模型registry／選択 | PASS | 登録単位で利用者指定の模型識別子を保持し、1〜128文字のASCII許可文字集合へ制限する。Codex CLIの`--model`へ同じ値を渡す。live catalogや利用可否は偽装しない |
| BYOK認証 | PASS（既存CLI設定への委譲） | 利用者がCodex CLIへ設定した認証を使う。D4 Pocketは資格値を入力・読取・保持せず、metadataも`secret_value_present=false`を維持する |
| capability表示 | PASS | `provider_selection`／`model_selection`は対応済み、通常Releaseの`task_execution`は既存gateどおり`unsupported`のまま |
| health／fallback表示 | PASS | Provider接続・模型利用可否は証拠がないため`unknown`。自動fallbackは無効。Owner確認とAgent Center双方へ投影する |
| 提供元・模型選択の試験経路 | PASS（試験用提供元） | 試験用の偽Codex CLIを実Rust実行系Adapterから起動し、指定模型識別子を受領して作業完了結果へ返す |

P4は一つのProvider経路を選択可能にし、選択模型をBroker登録要求、native Owner確認、Codex CLI実行引数へ一貫して結合したため閉鎖する。未知Provider、権限類似field、自動fallback有効値、不正模型識別子は拒否する。複数Runtime登録はそれぞれの模型選択を保持できる。

実WindowsのCodex CLI `0.160.0`で`--version`と`exec --help`を読み、Adapterが必要とする`--cd`、`--json`、`--ephemeral`、`--ignore-user-config`、`--skip-git-repo-check`、`--sandbox`、`workspace-write`、`--model`を確認した。これはCLI interfaceの存在だけを示し、認証状態、Provider接続、模型の実在性・利用権限、実Provider Taskを証明しない。実Provider呼出しは行っていない。実行時状態は引き続き`unknown`として扱う。

検証: `python tooling/schema_check/check_schemas.py` PASS（155 schema／155正常例／199負例）、`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py` PASS（233 checks）、Agent Center対象Widget 50件PASS、Desktop全test 166件PASS、Desktop／Mobile `flutter analyze --no-pub` PASS、Windows `flutter build windows --debug --no-pub` PASS（MSB8029の一時directory警告あり）、Debug／Release Rust build PASS、P4登録native test 4件PASS、偽Codex CLI模型引渡しtest PASS。Flutter検証はASCII一時複製で実行しOneDrive同期対象へ深いbuild outputを作らなかった。

Rust全件`cargo test -- --test-threads=1`は423 passed／2 failed／6 ignored。失敗したA2AとCodex loopback fixtureは両方とも個別再実行で各1件PASSし、P4変更file・経路には該当しない。再現性のある単独失敗とは観測されなかったためP4を止めず、集約Rust suiteのWindows loopback安定性を`docs/FINAL_QA_QUEUE.md`へ送った。strict日本語監査の現行変更fileは0 finding。全体終了コードは1で、残る2件は旧rev3／rev4履歴文書内のみ。履歴は書き換えない。

### P5 作業領域／履歴／評価（Workspace / History / Evaluation）— CLOSED（Product Build）

| 受入れ条件 | 状態 | 成立範囲・証拠 |
| --- | --- | --- |
| 作業領域検査（`Workspace Inspector`） | 合格 | 実行系／作業領域に束縛した変更file・diffの投影。既存Desktop対象試験22件合格 |
| Agent Task履歴 | 合格 | Brokerの構造化Auditから状態を再構成。現在の実行系履歴閲覧承認を再照合し、本文・Authorityを返さない。Rust対象4件、Desktop履歴画面16件、Desktop／共通UI解析合格 |
| 対話履歴／再実行／分岐 | 合格 | 現行の履歴承認に束縛した一覧と新規要求準備。履歴画面対象14件、Rust P5対象51件合格 |
| 結果／差分／変更file／試験結果 | 合格（`Product Build`範囲） | 作業領域検査／Agent管理画面の上限付き表示。Agentが申告した試験結果は独立検証済みへ昇格しない |
| Regression Case（回帰Case） | 合格 | Owner登録・一覧・操作面の既存対象試験合格（共通UIクライアント6件、2件） |
| Evaluation Case／評価環境 | 合格 | Case登録と評価画面の既存対象試験合格（画面5件、共通UIクライアント10件） |

Task履歴のAudit記録は本文を含めず、Task開始・終端の照合に必要な識別子の要約値、状態、時刻、Audit参照だけを保持する。開始Auditが重複・欠落・不一致、状態と失敗分類が矛盾、実行系が異なる場合は表示・回復投影を拒否する。Broker再起動時の未終端Taskは`suspended`へ隔離する。閲覧は既存の期限付き実行系別履歴承認を再利用し、Flutterは権限・本文を復元しない。構造化記録導入前のTask Auditは安全な実行系別再構成ができないため、既存Auditを保持したまま新一覧の対象外とする。画面でこの範囲を明示する。

今回の追加検証: `cargo test --locked --lib agent_task_history -- --test-threads=1` PASS（4件）、`cargo build --locked` PASS、Desktop履歴画面 `flutter test --no-pub test/history_screen_test.dart` PASS（16件）、Desktopと共通UIの`flutter analyze --no-pub` PASS、Schema検査PASS（158 schema／155正常例／201負例）、Conformance PASS（234 checks）。Flutter試験はOneDrive上の既知のbuild cleanup問題を避けたASCII一時cloneで実行し、検査対象の3 Dart fileだけを同期した。`python -X utf8 tooling/日本語基底監査.py --strict`はFAILのまま。今回変更fileは0 findingであり、残る2 findingは旧rev3／rev4履歴文書の既存記録で、現行意味正本へ昇格・書換えしない。既存P5 focused tests: Workspace Inspector 22、履歴／内容表示14+4、Evaluation画面5、対話履歴27、Regression Case Client 6+2、Evaluation Client 10、Rust P5対象51（合計141件）PASS。これはProduct Build受入れであり、通常Release capability、installed product、広域regression、最終出荷保証を意味しない。大規模evaluationとFeature Complete後の横断品質保証はFinal QAへ送る。

### P6 資格情報／MCP — CLOSED（Product Build）

| 受入れ条件 | 状態 | 成立範囲・証拠 |
| --- | --- | --- |
| 資格情報のOwner登録・一覧・失効 | 合格（Broker経路） | 登録はOwner control、一覧はmetadata-only、失効はRust Desktop native確認と永続Auditを要求。資格情報Broker試験6件PASS |
| 提供元への資格情報結合 | 合格（Product Build fixture） | `openai_codex_cli`の登録済みCredential IDをBrokerが現在のTask Permission／Approvalと照合し、DPAPIから解決した合成秘密値をCodex CLI親process環境へだけ渡す。偽Codex CLI fixtureは引数への非露出とshell環境除外設定を検査 |
| MCP Credential結合 | 合格（Windows製品経路） | 登録済みCredential ID・用途・対象Server・状態をDPAPI保管と照合し、対象stdio childだけへ短命値を渡す。別Server再利用を拒否 |
| MCP接続・Tool一覧 | 合格 | BrokerがWindows stdio childからdiscovery／Tool Schemaを取得し、通常IPCへmetadata-onlyで投影。Trustや権限は生成しない |
| MCP Tool実行・監査 | 合格（Product Build範囲） | Rust Broker・DPAPI・実`cmd.exe` fixture processを通すWindows統合試験で一回限りPermission、Owner-confirmation要求、永続Auditへの記録、hash-only結果、切断を確認 |

開始時点の記録: P6のMCP基本経路は2026-10-05に先行実装済みだったが、Provider credential bindingは未接続だった。MCP統合試験は合成Credentialだけを使い、実値とTool本文markerがBroker応答・Auditに現れないことを確認した。Tool応答の`LIVE_RUNTIME`はfixture child processとの実通信を示すが、Windows確認dialogを表示するDesktop製品操作の証拠ではない。

2026-10-05のP6完了単位で、Broker資格情報解決を現在要求のWorkspace Permission／Task Approval後に限定し、永続使用Audit確定後だけDPAPI秘密値を短命bufferでCodex Adapterへ渡す経路を追加した。Codex CLI資格情報modeは既存CLI管理認証modeと分離し、自動fallbackしない。Provider credential binding、Owner登録／一覧／失効、MCP接続／Tool一覧／Tool実行のProduct Build基本経路が揃ったためP6をCLOSEDとする。

Provider結合の`FIXTURE`試験では、合成CredentialがCodex CLI親process環境へ届き、CLI引数・結果・Auditへ出ず、tool shellから除外する設定が渡ることを偽Codex CLIが確認した。Broker試験ではPermission／Approval欠落と本文差替時にResolverが呼ばれず、有効grant後に一度だけCredentialがAdapterへ届く。これは実Codex CLI／実Provider APIの成功証拠ではない。実Codex CLI 0.160.0を合成loopback APIへ接続する`LIVE_RUNTIME`試験は、API要求前にMxCが`CreateProcessSecurityEnvironment`／HRESULT `0x80070003`で停止した。Provider接続・模型利用可否は`unknown`のままとし、自動fallbackを無効、通常Releaseの`task_execution=unsupported`を維持する。実CLI／MxCの統合証拠と別profile installed経路はFinal QA／release evidenceで確認する。

検証結果: JSON Schema 158件、正常例155件、負例fixture 203件、適合確認234項目に合格。資格情報保管庫のRust対象試験7件、Broker資格情報解決の許可境界試験1件、偽Codex CLIへの合成資格情報引渡し試験1件、MCP Broker試験2件、MCP stdio試験5件、DesktopのMCP metadata試験12件、資格情報IDだけを登録する画面試験1件もそれぞれ合格。`cargo build --locked`も合格。

非選別の`cargo test --locked -- --test-threads=1`はlib試験432件成功／2件失敗／7件ignoredで終了した。失敗は既存Windows HTTP／TLS接続fixtureの接続切断（OS error 10054／`ConnectionReset`）で、Codex loopback fixtureは単独再実行で成功、Update DownloadのTLS修復fixtureも単独再実行で成功した一方、既存package置換fixtureは単独でも接続切断を再現した。現行差分の他targetまで結果を得るため、この3試験だけを`--skip failed_tool_result_is_not_replayed_as_another_exec_command --skip failed_replacement_keeps_the_existing_corrupt_package_unchanged --skip local_tls_server_repairs_only_after_verified_package_bytes`で明示除外した全target実行を行い、lib 431件成功／0失敗／7 ignored／3 filteredとなり、他の全実行targetも失敗なしで完了した。3試験は削除・変更せず、集約loopback安定性を既存`FQ-TEST-LOOPBACK`へ記録する。これはP6資格情報変更経路の失敗ではなく、P6 Acceptanceを阻止しない。通常Release capabilityや正式配布可否へは昇格しない。

日本語基底strict監査は終了code 1で、旧rev3／rev4履歴文書の2件と、試験内の外部Authorization protocol値`Bearer`に対する静的heuristic finding 1件が残る。今回のテスト説明文は日本語化済み。過去履歴は改変せず、protocol値も監査回避のために変形しない。この監査結果は日本語文書負債として記録し、P6のCredential authority境界・Provider結合Acceptanceとは分離する。

### P7 対A2A連携／複数Host／Adapter管理 — CLOSED（Product Build）

rev5の範囲はA2A、Multi Host、Adapter Manager、Host capability、接続性、degraded mode、local／remote Runtimeの区別である。実接続または決定的fixtureで主要経路を成立させる受入れ条件を満たしたため、P7 Product Buildを閉鎖する。各機能の経路・証拠は次の完結単位に記録する。

#### P7内の完結単位: A2A loopback接続面 — CLOSED（Product Build）

- 必須`shell.agent_operation`へDesktop専用A2A接続センターを追加。専用画面、全体検索、コマンドパレット、NavigationRailから到達し、optional Moduleを全て除いたbuildでも表示する。
- 接続要求はFlutterから既存Broker IPCへ送り、Rust起動器のnative Owner確認を経る。BrokerもA2A接続をnative確認専用とし、通常IPC／Owner credential要求を拒否する。Owner対象表示とBroker helper双方で、loopback IPv4 HTTP、query／fragment／userinfoなし、未対応Credential ref、Task等を送らない境界を検査する。Windows Runnerのnative確認応答timeout allowlistにもA2A操作を追加。
- Brokerが既に持つAgent Card取得・永続Audit・接続state・metadata-only一覧をDesktopへ接続。画面は接続先URIを成功後に消去し、endpoint hashとbounded metadataを表示する。Trustは`pending_review`を維持し、Permission／Approval／AuthorityやA2A Task実行へ昇格させない。
- Agent Card由来の表示文字列は未信頼として表示し、改行／方向制御文字を含む値をUI clientで拒否する。認証実値は要求へ含めない。
- 個別検証結果: RustのA2A境界試験9件、native Owner確認候補1件、Brokerのnative確認専用gate 1件、Flutter接続service／画面／検索試験9件、任意Moduleをすべて無効にしたNavigationRail試験2件、`Conformance` 234件、Windows版Desktopのdebug build成功（MSB8029の一時作業フォルダー警告あり）。Flutter試験とWindows buildはOneDrive外の`ASCII`一時複製で実行。
- Rust全targetは444件中435成功／2失敗／7 ignored。今回のA2A関連11件は全件成功。失敗のうち`failed_tool_result_is_not_replayed_as_another_exec_command`は単独再試験で成功し、`local_tls_server_repairs_only_after_verified_package_bytes`は単独でもHTTP/TLS fixtureの応答不整合とConnectionResetで失敗した。いずれも今回変更したA2A経路外で、既存`FQ-TEST-LOOPBACK`へ追加記録し、このA2A受入れを拡張しない。
- Strict日本語基底監査は終了コード1で、現行変更fileは0 finding。既存findingは3件（旧rev3文書1、旧rev4文書1、未変更Codex CLI診断文字列1）。過去文書・無関係Adapter診断は今回のA2A Acceptance外として保持し、監査全体をPASS扱いしない。
- rev5 Product Buildの基準（実装、build、基本正常経路、次工程からの利用、Authority非破壊）を満たしたため、このloopback UI接続単位をCLOSEDとする。C16の既存Desktop画面未成立記述を現行状態へ訂正した。外部A2A transport／Task実作用とinstalled product証拠は別gateであり、この受入れを拡張しない。P7全体の受入れ判定は本章末尾に記録する。

#### P7内の完結単位: Host所在・能力・degraded表示 — CLOSED（Product Build）

- Host operation surfaceは、`snapshot_source=broker`として受理された製品snapshot内で、Host capabilityのHost IDとHost registry IDが一致した場合だけ、そのHostを現在Brokerの実行場所（ローカル）と表示し、Broker由来Runtime／Agent一覧を接続する。
- `mock`、fallback、in-memory diagnostic、未検証snapshotはlocal／remoteまたは実測能力の証拠にしない。別Hostは登録metadataのruntime／agent件数だけを表示し、remote接続と個別一覧は未観測に保つ。Host registryの接続／Trust状態と現在Brokerの実測を別表示する。
- Host capability画面では、受理済みBroker snapshotの`degraded`と機能別証拠を表示し、権限生成へ昇格させない。非Broker snapshotでは能力値を未観測の表示用状態として明示する。
- 検証: 画面試験54件は全て合格（模擬／診断snapshotの昇格拒否、受理済みBroker snapshotによるローカルHost判定、別Host要約の分離、縮退表示を含む）。変更対象のDart 3 fileの静的解析とWindows向けデバッグbuildも合格した。`MSB8029`は`ASCII`一時検証先へのbuild出力警告。試験とbuildは`OneDrive`外の`ASCII`一時複製で実行した。
- この完結単位はHost状態の製品表示・snapshot分類を閉じる。実Host間接続、Device Link認証、remote capability discovery、Host間Workspace隔離を証明するものではない。これらの未成立範囲はrelease gateで保持し、P7 Product Buildのfixture受入れとは分離する。試験中に検出したNavigationRail件数3箇所の旧値（20）は新しい必須A2A画面を含む現行値（21）へ同期した。

#### P7内の完結単位: Adapter既存recordのDesktop操作 — CLOSED（Product Build）

- Desktop Runtime Centerの`検証`、`有効化`、`無効化`、`隔離`、`削除`を、既存のPID結合named pipe、Rust Desktop起動器のnative Owner確認、Broker Owner操作queueへ接続した。操作名、Adapter ID、現在hash、payload hashを確認対象に固定し、未知field、操作不一致、形式不正、古いpayload hashは確認候補から拒否する。
- native Owner確認で拒否すると状態変更なしの`Suspended` receiptとAuditで終わる。承認後はBrokerが現行catalog recordのhashと状態条件を再照合する。変更範囲はBroker内catalogとAuditに限り、Permission／Approval／Credential／Trust、外部file、process、既存processへ作用しない。通常IPCの直接変更要求は引き続き`owner_reapproval_required`で停止する。
- Install／UpdateのManifest経路、外部artifactのdownload／filesystem導入・削除、process起動、Windows installed product証拠はこの完結単位に含まれない。C19の既存release blockerを解消したとは扱わない。
- 個別検証: Adapter Center Rust試験7件、Desktop Owner候補／relay試験2件、実Win32 Owner dialog自動化（拒否・承認）1件、既存Flutter通常IPC拒否widget試験1件が成功。ASCII一時複製で`flutter build windows --debug --no-pub`も成功し、Windows Runnerの変更を含むcompileを確認した。MSB8029一時出力先警告あり。試験自動化がdialogを操作し、Ownerの画面操作は不要だった。
- この既存record操作のProduct Build受入れを閉じた。旧rev3／rev4の進捗記録は変更せず、現行状態の判定には本rev5進捗と現行codeを使う。

#### P7内の完結単位: Adapter Manifest導入・更新のDesktop metadata操作 — CLOSED（Product Build）

- Runtime CenterからManifestを上限48 KiBで既存Broker transportへ送り、PID結合pipe、Rust Desktop起動器のnative Owner確認、Broker Owner操作queueを通じて、Broker catalogのmetadataだけを登録・更新する。通常IPCからの直接変更は停止する。
- Owner確認は申告Capability／許可差分／危険・互換性・署名metadataと署名対象／signature hashを示すが、署名実値は表示しない。Owner確認はTrust、署名検証、Permission、Approval、Credential、Authorityを生成しない。未検証Adapterは有効化できない。
- 更新は対象Adapter IDと現在hashを要求に束縛し、Brokerが処理時にhash・disabled・非隔離状態を再確認する。古いhash、有効化中、隔離済み、対象ID不一致は変更前に拒否する。拒否・受理はBroker Audit receiptへ結合する。
- 受入れ範囲はmetadata登録・更新であり、外部download、filesystem install／remove、process起動・管理、Windows installed product evidenceを含まない。これらC19のrelease blockerは解消していない。P7 Product Buildの総合判定は本章末尾へ分離し、この完結単位とC19全要件を同一視しない。
- 検証: Rust Adapter Center対象試験9件合格、Manifest Owner候補／中継試験合格、実Win32 Owner確認画面の自動操作（拒否・承認）1件合格、Desktop全画面試験57件合格（日本語表示ラベル修正後に対象Widgetを再実行）、変更したDart 3 fileの静的解析合格、Schema 158件／正常例155件／負例205件合格、適合確認234件合格、Windows Desktop debug build合格。全Desktop静的解析は未変更の`agent_center.dart`に廃止予定APIの情報指摘5件があり終了コード1、今回変更したDart fileに問題はない。BuildはASCII一時複製で行い、MSB8029一時出力先警告を記録。厳格日本語監査は今回変更fileの指摘0件、全体では既存の3指摘（rev3履歴、rev4履歴、未変更Codex CLI診断文字列）により終了コード1。旧記録・既存指摘は保持し、本完結単位へ転記・修正しない。
- Rust全体試験`cargo test --locked -- --test-threads=1`は443件合格／2件失敗／9件ignored。失敗は既存Codex loopbackとUpdate Downloadのlocal TLS fixtureでConnectionResetを観測したもの。2件を個別再実行すると各1件合格し、Adapter差分外だったため、既存`FQ-TEST-LOOPBACK`へ追記した。Adapter対象focused試験と本単位の受入れは合格であり、P7を阻止しない。
- 本単位をCLOSEDとする。Host実接続／remote capabilityと外部artifactのfilesystem・process作用は、C17〜C19のrelease blockerとして別管理する。rev5 Product Build受入れとrelease gateを混同せず、PASS済み単位は再開しない。

#### P7 rev5 Product Build受入れ — CLOSED

| 受入れ領域 | 結果 | 現行証拠・境界 |
| --- | --- | --- |
| A2A／接続性 | PASS | Rust Brokerのloopback接続とDesktop接続面を実装・試験済み。外部A2A transport／Task実作用は未成立。 |
| Multi Host／local・remote区別 | PASS | Host registryとHost操作面を接続。localは受理済みBroker Host ID一致時だけ、他Hostは登録metadataとして表示し個別状態を未観測に保つ。 |
| ホスト能力と縮退表示 | 合格 | Broker snapshotの証拠種別と制限状態を表示し、試験用fixture／診断値を実測値へ読み替えない。 |
| Adapter管理 | 合格 | 既存記録の検証・状態制御・削除と、native Owner確認経路によるManifest metadata導入／更新を実装。署名未検証では有効化を拒否する。 |
| Authority非生成 | PASS | Host／Adapter metadata、Owner確認、接続表示からPermission／Approval／Trust／Credential／Authorityを生成しない。 |

rev5工程表の「実接続または決定的fixtureで主要経路成立」に対し、A2A loopbackの実接続とHost／degraded／remote未観測の決定的fixture、Broker Owner経路のAdapter管理を確認した。これによりP7のProduct Build機能範囲を満たし、P7をCLOSEDとする。

実Host間の認証付き通信・remote capability discovery・Host間Workspace隔離、外部Adapter artifactのdownload／filesystem導入・削除・process管理、Windows installed product evidenceは未成立のままrelease blockerとして保持する。これらを成立済み、C17〜C19の全要件完了、またはrelease-readyへ読み替えない。Feature Complete後の横断保証はFinal QA queueに従う。

### P8 GUI-Shell構成 — CLOSED（製品構築受入れ完了）

| 受入れ項目 | 結果 | 現行証拠・境界 |
| --- | --- | --- |
| Runtime／Agent／Tool／MCP選択 | 合格 | Desktop設定画面で参照IDを入力しManifestへ正規化してBrokerへ送る。実在性・接続・Agent trustの発見や判定は行わない。 |
| UI構成 | 合格 | Theme mode、表示密度、内容表示要求、Capability requirementを画面から選択しManifestへ反映。Theme IDとLocaleは現行契約により固定。これらの要求はPermission／Authorityを生成しない。 |
| Module選択 | 合格 | 任意Module checkboxをCompose Manifestと同じ書出し操作へ渡し、BrokerがExport ReceiptのModule planを生成する。Compose Manifest本体とは別fieldであり、実binary pruningを意味しない。 |
| 事前表示（Preview） | 合格 | 直近受理済みManifestと候補をBrokerへ送り差分を計算。rollback、build、Exportを実行しない。 |
| AI編集提案（AI Edit） | 合格 | 明示指示を審査待ちproposalとしてBrokerへ送り、自己承認とfile writeをしない。 |
| 一構成を作成 | 合格 | Desktop設定画面からManifest作成・Preview・Module選択・proposal要求へ接続。Compose結果はManifest-onlyであり、独立App／executableは生成しない。 |

検証結果: Compose関連Desktop client／Widget試験12件合格（800px幅のSettings画面統合fixtureを含む）。Rust Broker focused試験はCompose／Preview 7件、Export／Module plan 13件、AI Edit 2件が合格。Schema検査158 schema／155正常例／205負例、Conformance 234 checksが合格。変更Dart 4 fileの`flutter analyze --no-pub`は問題なし、Mobile全体の`flutter analyze --no-pub`は問題なし。Desktop全体解析は終了値1で、未変更`agent_center.dart`の既存deprecated API情報5件だけ。Windows Desktop debug buildはASCII一時複製で成功（MSB8029一時directory警告あり）。OneDrive checkoutでのFlutter試験起動は`build\unit_test_assets`削除失敗により不可だったため、Flutter試験／buildはASCII一時複製で実行した。これはfixtureとlocal buildのProduct Build証拠で、Windows installed／standalone product実起動証拠ではない。

これをrev5工程表のP8範囲に限ったProduct Build受入れとしてCLOSEDにする。実executable、portable package、installed product、実Module binary除去はP9以降またはrelease gateで扱い、P8へ逆流させない。

### P9 Standalone Export — CLOSED（製品構築受入れ完了）

| 受入れ項目 | 結果 | 現行証拠・境界 |
| --- | --- | --- |
| 独立App ID／Audit Store IDとManifest | 合格 | clean `main` commit `77821953a70ee3d1ff1059835dad56fc658a35b3`のchecked-in test Receipt／Manifestから、App ID `d4-pocket-app-11111111111111111111111111111111`、Audit Store ID `audit-store-22222222222222222222222222222222`を含む製品をbuild。入力は合成fixtureであり、Owner承認済みproduction Exportではない。 |
| executable／Rust Broker／選択Module／portable package | 合格 | Windows Release bundleにRust起動器、Flutter executable／assets、Rust Broker、byte-for-byte Manifest copyを含む14 filesを生成。要求された`shell.trace_inspector`をincludeし、optional module planをFlutter compile defineへ渡した。 |
| 生成物一覧／秘密情報の既知パターン検査 | 合格 | 62,856,402 bytes、tree SHA-256 `d5624b93eb73e3f1f9b9a1102e43ea32394fb76404be67f8e581ea4d1a73360e`。既知パターン検査は14 files／0件。未知形式を含むCredential不存在やbinary pruningは証明しない。 |
| 元D4 Pocketと分離した製品起動 | 合格 | 生成bundleのRust起動器から同bundle内Flutter childが起動し、Windows process parent／image path、応答中の`D4 Pocket` main window、製品ID別runtime pathの新規生成を`LIVE_RUNTIME`で観測。runtime rootは起動前に不在で、起動後は`%LOCALAPPDATA%\D4Pocket\apps\<App ID>\stores\<Audit Store ID>`に専用Store／新規Audit anchorができ、generic GUI-Shell Broker pathと異なる。 |
| Credential／Permission／Approval／Audit chain／Authority非継承 | 合格 | fixture Manifest／Receiptのauthority stripと全`*_inherited=false`、build evidenceの同境界、起動前に不在だった専用runtime root、新規Audit anchorを確認。Credential実値は入力していない。これはfixture起動範囲であり、production Owner operation、署名、一般的なsecret不存在の証拠ではない。 |

実行したbuild commandは`python -X utf8 tooling/export_windows_product.py`（checked-in test Receiptと対応Manifestを使い、一意なTemp input／output directoryを指定）。Flutter 3.44.0／Dart 3.12.0、Rust/Cargo 1.95.0でRelease build・Schema／Receipt／Manifest照合・Module plan解決・inventory再計算・known-pattern scanまで成功した。Manifestはpackage rootの`product_manifest.json`にbyte-for-byteで含まれる。Rust起動器のStore選択は現行launcher contractどおりManifest再読込ではなくbuild時のSchema検証済みIDを使用する。

Windows Computer Useはこの2個目の同名Flutter windowをtargetable windowとして返さなかったため、画面screenshotや視覚的UI確認は主張しない。Windows process metadataではchildが`Responding=true`でmain windowを生成したことを観測した。`WM_CLOSE`は現行Windows Runner仕様に従い通知領域へ隠す動作だった。normal tray exitはP9の受入れ項目ではなく、この試験では未確認。試験cleanupでは対象bundleのfrontendだけを停止し、起動器が終了Error dialogを出した後、対象launcherも停止した。これはnormal exit evidenceに数えず、P12統合Productのnormal exit確認へ送る。Broker session fileは消え、専用Audit Storeは独立確認用のtest stateとして保持した。

これをrev5工程表P9の有限なProduct Build条件の完了としてCLOSEDにする。実Owner確認によるExport、別user profile／installed artifact同一性、署名済み配布、正式Installer、runtime Manifestのproduction trust、binary pruning、normal exit、release readinessは成立扱いにせず、既存release blockerまたは後続P12／Final QAの範囲で保持する。現在phaseは直ちにP10 Module Selection / Pruningとする。

### P10 Module Selection / Pruning — CLOSED（機能上の除外）

| 受入れ項目 | 結果 | 現行証拠・境界 |
| --- | --- | --- |
| 選択Moduleと依存閉包 | 合格 | `python -X utf8 tooling/build_module_pruned_windows.py --receipt examples/contracts/gui_shell_export_receipt.valid.json --plan-only`が追跡画面と依存先の観測画面だけを含め、任意画面6件を除外した。固定安全Coreと必須画面はすべて保持し、Authority検証false、binary pruning未実施を出力した。 |
| 書出し製品への選択反映 | 合格 | P9 Windows Release bundleのbuild receiptは、選択された追跡画面と依存先にtrue、除外6画面にfalseのcompile-time defineを記録する。これはbuild設定の証拠であり、AOT／実行可能fileからのbinary除去証拠ではない。 |
| 製品画面の機能上の除外 | 合格 | 全8任意画面を無効にしたFlutter Widget試験2件が、任意画面のナビゲーション非表示、必須画面13件の保持、除外後の操作で例外・未搭載画面が出ないこと、除外されたHost操作Commandが出ないことを確認した。証拠源は`FIXTURE`。 |

実行した試験は、Desktop projectで`flutter test --no-pub --no-test-assets --dart-define=GUI_SHELL_MODULE_SETUP_DOCTOR=false --dart-define=GUI_SHELL_MODULE_HISTORY=false --dart-define=GUI_SHELL_MODULE_EVALUATION_LAB=false --dart-define=GUI_SHELL_MODULE_HOST_CAPABILITIES=false --dart-define=GUI_SHELL_MODULE_NOTIFICATIONS=false --dart-define=GUI_SHELL_MODULE_OBSERVABILITY=false --dart-define=GUI_SHELL_MODULE_TRACE_INSPECTOR=false --dart-define=GUI_SHELL_MODULE_HOST_OPERATIONS=false test/module_pruning_test.dart`（2件合格）、`python -X utf8 tooling/build_module_pruned_windows.py --receipt examples/contracts/gui_shell_export_receipt.valid.json --plan-only`、Schema検査（158 Schema／155正常例／205負例）、Conformance（234 checks）。最初のasset付きFlutter試験は、OneDrive上の`build/unit_test_assets`をFlutterが削除できず、試験本体開始前に失敗した。assetを要しない対象試験を`--no-test-assets`で再実行して合格した。

P9 portable bundleのWindows再起動時、Computer Use helperは同名製品windowを対象可能な窓として返さなかった。process path／parentは対象bundleを識別できたが、視覚的な画面確認は主張しない。P10の機能受入れはcompile-time define、製品build記録、Widget試験の範囲で閉じる。AOT／binaryの意味上の除去、Rust／第三者依存の除去、サイズ、cold startup、実行時resource、最終統合製品上の再確認は`docs/FINAL_QA_QUEUE.md`と既存release blockerへ送る。`binary_pruning_verified=false`は維持する。現行phaseは直ちにP11 Windows Productizationとする。

### P11 Windows製品化 — CLOSED（rev5製品機能受入れ）

P11のInstaller entryは、P9 portable ZIPをbootstrapとしてD4 Pocketを起動し、製品内Update CenterからBroker所有trustで検証されたpackageをdownloadして初回Installする経路と定義した。別個のMSI／Setup.exeや署名済みrelease artifactを作成したとは主張しない。利用者のInstall操作はRust Brokerのnative Owner確認、固定product root、durable Audit、active version record、Start Menu登録、終了後の導入版起動引継ぎを通る。Uninstaller、first-run／Setup Doctor、Update、Rollback、固定root／payload欠損の限定Repair、version表示も既存製品経路へ接続済み。

閉鎖根拠は、現行作業でのFlutter `settings_install_flow_test.dart`／`settings_compose_test.dart`／`update_client_test.dart` 17件PASS、変更Dart 3 fileの解析PASS、Rust focused `update_download_requires_current_native_confirmation_and_runs_as_bounded_job`・`native_owner_apply_stages_exact_signed_package_without_activation_or_overwrite`・`portable_activation_requests_installed_launch_only_after_accepted_record_switch`各1件PASS、および既存progressに記録したUninstaller／Repair／first-run／Setup Doctorの局所testとcompile証拠。Brokerのstage・active version切替・Rollbackは同じfixture test内で、Downloadとportable起動引継ぎも独立したRust testで成立する。Schema 161／157／208、Conformance 236 checks、Manifest、Release GateもPASS。

これは有限なProduct Build機能受入れであり、Feature Complete、別profile installed LIVE_RUNTIME全経路、standalone MSI／Setup.exe、正式署名／Publisher identity、production update trust、crash／電源断の網羅、完全Repair、release readinessを意味しない。全機能の一本化はP12、広いinstalled／failure／formal evidenceはFeature Complete後のQ工程と既存release blockerで扱う。これらを理由にP11を再開しない。

### P12 製品全体統合 — CLOSED（Windows FEATURE COMPLETE）

rev5工程表の15段階の正常利用scenarioを、D4 Pocketの一つの製品経路として接続する。既にCLOSEDのP2〜P11の局所条件を再実装・再試験せず、未接続の交点だけを埋める。最初の統合blockはInstall後の起動／first-runからRuntime登録へ進み、以後Provider／Model、Agent／Workspace、Task、Compare、Handoff、History／Diff、MCP、Compose／Export、Update、normal exitの順に接続する。実資格・実課金・外部副作用は使わず、通常Owner操作待ちにせず、決定的test identityを利用する。

#### P12接続単位1: first-runからRuntime／Agent登録UI — 完了

Dashboardに、first-run後の次の操作として既存Agent Centerを開く明示導線を追加した。Runtime／Workspace登録のBroker要求、native Owner確認、Permission、Task Approvalの実装は変更せず、FlutterはBroker未接続時に登録を実行しない。製品modeのFake Brokerで初回設定取得・Setup Doctor取得後にAgent Centerへ遷移するWidget試験と、既存のCodex登録UIが指定値だけをBrokerへ送りpermission／approval fieldを送らないWidget試験を個別に実行した。

検証: Dashboard遷移と既存登録UIのfocused Flutter Widget test各1件PASS（`--no-pub --no-test-assets --dart-define=GUI_SHELL_PRODUCT_VERSION=1.2.3`）。`flutter analyze --no-pub lib/main.dart lib/screens/dashboard.dart test/widget_test.dart`は3項目No issues found（日本語を含むOneDrive pathでLSP messageが壊れたため一時drive aliasから実行）。`dart format lib/main.dart lib/screens/dashboard.dart test/widget_test.dart`は変更なし。Schema 161／正常例157／負例208、Conformance 236 checks PASS。試験証拠は`FIXTURE`であり、実installed product、native Owner dialog、実Codex processの証拠ではない。当時の次工程Provider／Modelは接続単位2で確認済み。

#### P12接続単位2: first-runからProvider／ModelとRuntime／Workspace登録 — 完了

DashboardからAgent Centerへ進み、既存登録画面でProvider／Modelを設定し、Runtime／Workspace登録要求まで到達する製品経路Widgetを追加した。P4でCLOSED済みのProvider／Model選択実装は変更せず、`openai_codex_cli`、合成模型ID、Runtime ID、Workspace ID／rootが同じBroker登録要求へ結合される接続だけを確認する。Permission／Approvalを登録payloadへ追加しない。P12の現行フォームはRuntime／Agent Adapter／Workspace登録を一つにまとめるため、独立したProvider CenterやWorkspace pickerは追加していない。

検証: `flutter test --no-pub --no-test-assets --dart-define=GUI_SHELL_PRODUCT_VERSION=1.2.3 --plain-name "first-runからProvider／Modelを設定してBroker統治登録へ進める" test/widget_test.dart` 1件PASS。Dashboard first-run設定・Setup Doctor取得から登録画面へ進み、明示したUI操作後にFake Brokerが受理する要求のProvider／Model、Runtime、Workspace bindingとAuthority field不在を確認した。試験応答は`FIXTURE`であり、画面上のOwner確認遷移は実native dialogではなく、実Codex CLI／Provider呼出し・installed productを証明しない。P4の既存条件は再試験していない。次は同じ製品経路から登録Agent Sessionを開始し、normal Agent Taskへ接続する。

#### P12接続単位3: 登録Agent Sessionから通常Agent Task完了 — 完了

first-run後のDashboardからAgent Centerへ進み、既存のProvider／Model設定、Runtime／Workspace登録を経て、登録Workspaceへ結合したAgent Sessionを開始する。そのSessionへ通常Task UIから要求し、Broker事前検査、Workspace Permission、Task一回Approvalを個別要求し、`running`から`completed`のTask status／result hashを表示するまで、一本のWidget経路で接続した。P1でCLOSEDのTask機能は再実装せず、通常Releaseの`task_execution=unsupported`やrelease gateも変更しない。試験内の`task_execution=supported`応答は、Task UI経路を通すためのFake Broker fixtureに限定する。

検証: `flutter test --no-pub --no-test-assets --dart-define=GUI_SHELL_PRODUCT_VERSION=1.2.3 --plain-name "first-runからProvider／Model設定後に登録AgentのTaskを完了する" test/widget_test.dart` 1件PASS。Fake Brokerは合成Runtime／Workspace／Session／Taskと合成grantを返す。Clientはsession binding、Permission／Approval receipt、Task recordを要求へ照合し、試験は操作順とcompleted/result hashを確認する。証拠種別は`FIXTURE`で、native Owner dialog、自動承認、実Codex／Provider実行、Taskの実Workspace書込み、installed製品を証明しない。P12はOPENを維持し、次はCompare接続へ進む。

#### P12接続単位4: DashboardからCompare事前検査への接続 — 完了

製品AppのDashboardからAgent Centerへ遷移し、Broker Session一覧由来の異なるRuntime／Workspace／SessionをCompare候補として表示・選択、同一Task本文で2件のBroker事前検査要求を行う交点をWidgetで確認した。両要求が各Session／Workspaceへ独立に結合し、Permission／Approval状態を共有しないことを応答投影で確認する。Compare UIの候補／事前検査接続が対象であり、CLOSED済みP2 Compareの同時実行、結果選択、negative試験は再実行しない。

検証: `flutter test --no-pub --no-test-assets --dart-define=GUI_SHELL_PRODUCT_VERSION=1.2.3 --plain-name "DashboardからCompare候補を選び独立Broker事前検査へ接続する" test/widget_test.dart` 1件PASS。Fake Brokerが返す2件のSessionを用いた`FIXTURE`であり、実Codex並行実行、Permission／Approval発行、Task実行・結果比較の新しい証拠ではない。P12はOPENを維持し、次はHandoff接続へ進む。

#### P12接続単位5: 完了Taskから受信SessionへのHandoff接続 — 完了

DashboardからAgent Centerへ進み、Provider／ModelとRuntime／Workspace登録、Session開始、通常Taskの完了、Task結果の`full` Content Exposure表示を経て、Handoff公開packageを別Runtime／Workspaceの未使用受信Sessionへ新規Taskとして事前検査する一本のWidget経路を確認した。Handoff packageは元Task／Sessionと承認済みresult hashに結合し、authority再評価を要求、Permission／Approval／Credential／hidden context非継承を固定する。受信側はTask事前検査のみ行い、新しいPermission／Approvalは未付与のまま。

検証: `flutter test --no-pub --no-test-assets --dart-define=GUI_SHELL_PRODUCT_VERSION=1.2.3 --plain-name "first-runから登録Agent Taskを完了しHandoffを受信側へ接続する" test/widget_test.dart` 1件PASS。OneDrive上の元worktreeではFlutterがmacOS／iOSのignored `ephemeral/Packages/.packages` Cloud Files reparse pointを削除できない旨を出したが、対象testの終了値は0。ASCII一時worktreeへ同じ変更testを複製したFlutter解析は対象Widget `No issues found!`、Mobile全体`No issues found!`。Desktop全体解析は今回未変更の`agent_center.dart`に既存deprecated API info 5件があり終了値1。日本語pathの元worktreeからのFlutter解析はCloud Files cleanup失敗で完了しなかった。`dart format --output=none --set-exit-if-changed test/widget_test.dart`は変更なし。Schema 161／正常例157／負例208、Conformance 236 checksがPASS。`tooling/日本語基底監査.py --strict`は変更対象にfindingなし、履歴／未変更CLI診断文の既存3 findingsにより全体終了値1。Manifest 1180 filesとWindows v1 Release Gate、`git diff --check`はPASS。Fake Brokerの合成Task、full結果承認／投影、受信Sessionを使った`FIXTURE`であり、表示操作はnative Owner dialogを実演せず、実Codex間転送、実Workspace書込み、installed製品を証明しない。P3 CLOSEDのHandoff処理やnegative testは再実装・再試験していない。P12はOPENを維持し、次はHistory／Diff接続へ進む。

#### P12接続単位6: Task結果から履歴／Workspace差分面への接続 — 完了

Desktop Shellは履歴画面を開いたとき、既に接続済みの`ShellCoreClient`が保持するBrokerTransportを既存`HistoryClient`へ渡す。BrokerTransportがない診断起動時は従来の接続処理を残す。Agent Centerの完了Taskからfull Content Exposureで得たAgent申告の`changed_files`／`diff`／`test_result`を同一利用scenarioで確認し、履歴画面では独立した現在の履歴閲覧GrantでTask記録、result hash、監査参照だけを読む。履歴にはTask本文・過去Permission／Approval／Credentialを復元せず、画面からAgent Centerへ戻ると別Broker承認・baselineを使うWorkspace Inspectorで作業領域差分を見る案内を保つ。Task結果のAgent申告とWorkspaceの独立観測を同一視しない。

検証: `flutter test --no-pub --dart-define=GUI_SHELL_PRODUCT_VERSION=1.2.3 --plain-name "first-runから登録Agent Taskを完了しHandoffと履歴へ接続する" test/widget_test.dart` 1件PASS。ASCII一時worktreeでlockfileどおり依存解決後に実行し、変更Flutter 2 fileの`flutter analyze`は`No issues found!`。Desktop全体解析は未変更`agent_center.dart`の既存deprecated API info 5件で終了値1、Mobile全体は`No issues found!`。一時検証の初回`--no-pub`では無視対象`.dart_tool`がなくtestを起動できず、`--no-test-assets`実行ではInk shaderが不足した。通常のtest asset生成へ切替後、重複表示されるNavigation／IndexedStack文言をtestが1件と仮定していた二箇所を修正し最終PASSした。Schema 161／正常例157／負例208、Conformance 236 checksはPASS。strict日本語監査は今回変更fileにfindingなし、旧rev3／rev4履歴と未変更Codex CLI診断文の既存3 findingsにより終了値1。Manifest 1180 filesとWindows v1 Release GateはPASS。試験はFake Brokerの`FIXTURE`で、履歴Grantは既存の現在承認を返す応答でありOwner UI、実Broker、実Agent、Workspaceの実差分・installed製品証拠ではない。P5 CLOSEDのHistory／Workspace Inspector本体・negative試験は再実装していない。P12はOPENを維持し、次はMCP接続へ進む。

#### P12接続単位7: MCP接続センターへの製品導線 — 完了

全体検索のMCP結果とコマンドパレットのMCP接続項目から、設定画面の先頭ではなく既存MCP接続センターへ直接移動する経路を接続した。MCPセンター見出しを基準に両親Scroll領域へ表示位置を合わせ、同じ`ShellCoreClient.brokerTransport`を使って、操作者が明示した「接続一覧を取得」だけを既存`McpConnectionClient`へ送る。Broker未接続時も同じ見出しと取得不能理由を表示する。MCP接続開始、Tool実行、Credential選択・操作、native Owner確認、Broker権限経路は変更していない。

検証: `flutter test --no-pub --dart-define=GUI_SHELL_PRODUCT_VERSION=1.2.3 --plain-name "MCP検索とコマンドからMCP欄へ移動し同じBroker transportで一覧を読む" test/widget_test.dart` 1件PASS。ASCII一時worktreeで`flutter pub get --enforce-lockfile`後に実行。Fake Brokerの`FIXTURE`で、検索／コマンド両導線のMCPセンター表示、Brokerへの一覧要求と`{'版': 1}` payloadを確認し、MCP接続／切断／Tool実行を発行しないことを確認した。元のOneDrive checkoutではFlutterがignored `build/unit_test_assets`を削除できずtestを開始できなかったため、一時worktreeで同じ変更ファイルを再現して検証した。Schema 161／正常例157／負例208、Conformance 236 checks、Manifest 1180 files、Windows v1 Release Gate、`git diff --check`はPASS。Mobile全体`flutter analyze --no-pub`は`No issues found!`。Desktop全体解析は未変更`agent_center.dart`のdeprecated API情報5件のみで終了値1。変更Flutter fileの`dart analyze`再試行はユーザーprofileのDart perf cache削除OS error 1920でAnalysis Serverが停止し結果未確定。strict日本語監査は今回変更fileにfindingなし、旧rev3／rev4履歴と未変更Codex CLI診断文の既存3 findingsにより終了値1。これは実Broker／Owner dialog／stdio child／installed製品の証拠ではない。P6 CLOSEDのMCP契約・権限・Credential試験は再実行していない。単位7終了時点で次はCompose／Export接続であり、結果を単位8に記録する。

#### P12接続単位8: Compose／Exportへの製品導線 — 完了

全体検索の「GUI Shell構成」から既存Settingsの構成Manifest面へ、コマンドパレットの「独立Appを書き出す」からWindows書出し面へ直接移動できる導線を追加した。Settings画面は各対象見出しへscrollし、Broker未接続時にも対象面と操作不能理由を表示する。ComposeとExportは既存の同じManifest draft関数と`ShellCoreClient.brokerTransport`を使用し、Export要求の`compose_manifest`がCompose要求payloadと一致することを一つの製品Widget試験で確認する。P8／P9のCompose・Export機能、Manifest契約、権限、native Owner確認、Broker実装は変更していない。

検証: `flutter test test/widget_test.dart --plain-name '全体検索からCompose、コマンドからExportへ進み同一ManifestをBrokerへ渡す'` 1件PASS。ASCII一時worktreeを作り、変更4 Dart fileを再現して`flutter pub get --enforce-lockfile`後に実行した。Fake BrokerはCompose Manifestを受け付け、Export要求とfixture Receiptを返す`FIXTURE`であり、native Owner確認、実Broker、実Manifest file作成、installed Product、独立App生成を証明しない。試験開発中に候補textのFinderが入力文字と競合した2回の失敗を記録し、Text widget predicateへ限定後にPASSした。Schema 161／正常例157／負例208、Conformance 236 checks、manual workflow checkはPASS。Mobile全体`flutter analyze --no-pub`は`No issues found!`。Desktop全体解析は変更外`agent_center.dart`の既存deprecated API情報5件で終了値1。変更file限定`dart analyze`はAnalysis Server shutdown時にDart perf cache pathを削除できずWindows OS error 1920で結果未確定。`dart format`と`git diff --check`はPASS。日本語strict監査は今回差分のfindingなし、旧rev3／rev4履歴と未変更CLI診断文の既存3 findingsで終了値1。ManifestとWindows v1 Release Gateは文書同期後に再実行する。P12はOPENを維持し、次は既存Update／通常終了経路の接続へ進む。

#### P12接続単位9: DashboardからUpdate Centerへの接続 — 完了

製品Dashboardの「インストール・更新センターを開く」導線からSettingsのUpdate Centerへ移動し、明示的に画面を開いた後だけ既存Broker transportで更新一覧を読む経路を接続した。初回Shell表示では一覧要求を送らず、遷移後に`更新一覧`を1回だけ送り、`{'版': 1}` payloadと空の候補一覧を表示する。Fake Brokerの応答はtrust未設定、download／apply／rollback停止の`FIXTURE`であり、候補取得・署名検証・download・install・updateを実行しない。

検証: `flutter test test/widget_test.dart --plain-name 'DashboardからUpdate Centerを開き既存Brokerで候補一覧を読む'` 1件PASS。OneDrive上のcheckoutではFlutterがignored `macos/Flutter/ephemeral/Packages/.packages`を削除できず、試験開始前に停止したため、既存ASCII一時checkout上で現行testを実行しPASSした。これはFake Brokerによる`FIXTURE`証拠で、実Broker、installed product、実配布元、署名済み候補、Owner確認、実更新は示さない。P11 Update機能を再試験せず、P12はOPENを維持して最後のnormal exit接続へ進む。

#### P12接続単位10: MCP Toolの製品経路 — 完了

Dashboardの全体検索からMCP接続センターへ移動し、明示的な一覧取得、metadata-only Tool選択、arguments全文確認、Windows確認へ進む操作、既存Broker transportへの`MCP Tool実行`要求、result hash／content type／Audit IDだけのreceipt表示までをWidgetで接続した。Tool result本文・credential・PermissionはFlutterへ返さない。実Broker/native Owner確認の認可経路やP6 MCP contractは変更していない。

検証: `flutter test test/widget_test.dart --plain-name 'DashboardからMCP Toolを明示確認後にBrokerへ要求しhash receiptを表示する'` 1件PASS。起動時にはMCP操作を送らず、一覧取得とTool実行だけを明示要求し、Tool実行は引数確認前には発行されない。Fake BrokerはClientのaccepted receipt形式に合わせて`LIVE_RUNTIME` wire fieldを返すが、実際の証拠分類は`FIXTURE`であり、実stdio MCP process、Windows native Owner dialog、外部Tool副作用、実Auditを示さない。試験初回では起動時のprofile／notification等の非MCP要求も同じtransportに記録されるため、初期期待をMCP operationだけの評価へ修正し、対象testはPASSした。P6 CLOSED機能を再試験していない。

#### P12接続単位11: Windows native trayからのnormal exit — 完了

現行Windows Runnerのnative tray「終了」項目からFlutter Runnerを終了する経路を確認した。対象Debug binaryは2026-10-06T15:50:05Z生成、SHA-256 `68ad367d0d6053ea44e179d5dce9dc53b5ee5795889879178f0858aa658550a0`、source commit `eaea63b52a5a4c35c44117b50b7979dbc2c57024`由来。PID-bound test runはtray context menuの「終了」を選択し、対象processがexit code 0で終了した。別のUI helperが起動していた同一artifactの試験processも同じ経路で終了させた。

証拠区分は`LIVE_RUNTIME`だが範囲はDebug Flutter Runnerのnative tray／window lifecycleに限る。Rust LauncherのBroker finish、最終Audit、installed productでの起動・終了は実証していない。これらの横断・installed検証はFinal QAへ送る。正常終了以外のprocess failure matrixはここで拡張しない。

#### P12受入れ判定 — CLOSED

P11で閉鎖したInstall／Update機能を再実装せず、P12でDashboard／Agent Center／Settings／既存Broker transportの接続を15段階の正常利用scenarioへ揃えた。Task、Compare、Handoff、History／Diff、MCP Tool、Compose／Export、Updateの製品UI経路はfocused `FIXTURE` integration test、最後のtray正常終了はDebug Runnerの`LIVE_RUNTIME`で成立したため、2026-10-07にProduct Build上のWindows `FEATURE COMPLETE`とする。15段階を一回のinstalled-product LIVE_RUNTIME試験で通したとは主張しない。installed end-to-end、実MCP／Provider、署名済みUpdate、正式証拠と広範なfailure／security／long-run検査はQ1以降および既存release blockerで扱い、P12を再開しない。次工程は`Q0 Final QA Freeze`。

## P11作業履歴（以下の記録は当時の状態。現行状態は上記CLOSEDが正本）

#### 2026-10-06 P11 Exportから製品Update trust設定への接続

SettingsのWindows書出し詳細欄から、製品Update用のEd25519公開鍵DERとchannel別HTTPS配布元を任意設定できるようにした。Flutterは公開値だけをBroker要求へ渡す。Rust BrokerはSPKI形式、channel重複、配布元URLを検証し、fingerprintを算出してversion 2 trustへ正規化する。native Owner確認にはfingerprintとchannelごとの完全な配布元URLを表示し、完了後に限りExport Manifestへtrustを記録する。既存ReceiptはManifest fileの長さ・hashを結合する。空欄時は従来どおりtrust未設定のManifestとなり、製品Updateはfail-closedを保つ。秘密署名鍵を入力、表示、保存する欄はない。

検証: `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib broker::export_center::tests:: -- --test-threads=1`（15 passed）、`cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib desktop_launcher::tests::product_export_confirmation_shows_its_isolated_store_path -- --test-threads=1`（1 passed）、Desktop Flutter `flutter test --no-pub --no-test-assets --dart-define=GUI_SHELL_PRODUCT_VERSION=1.2.3 test/export_client_test.dart test/settings_compose_test.dart`（7 passed）、対象2 fileの`dart analyze`（No issues found）、Schema 161／正常例157／負例208、Conformance 236 checksがPASS。Rust fixtureはExport Manifestへの正規化、fingerprint／channel別URLのOwner表示、不正鍵とuserinfoを含むURLの拒否を確認した。これはBroker／UI fixtureと既存compile-time trust build経路の接続証拠であり、当該UI生成Manifestを使う製品Release build、実配布元、署名運用、installed updateの証拠ではない。P11はOPENを維持し、Installer／installed product経路を継続する。

追加確認: ignored Windows native Owner dialog統合試験を`cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib nativeOwner確認dialogのExportNoYesをBrokerReceiptManifestまで接続する -- --ignored --test-threads=1`で自動操作し、拒否・承認・Broker Receipt／Manifest接続が1件PASSした。Ownerの物理操作は不要だった。対象Rust Release `cargo check`もPASS。後続の`dart analyze`再試行は、診断出力前後のAnalysis Server終了時に`%LOCALAPPDATA%\Dart\perf\<pid>`を削除できないWindows OS error 1920で終了した。先行実行では`No issues found`を得ており、Flutter対象testは今回も7件PASSだが、後続Analyzer再試行はhost環境制約として記録する。Windows Debug buildは既存の追跡外CMake cacheが旧`Z:/apps/desktop_flutter/...`を参照して最初に停止したため、Z:が未使用であることを確認し、同じcheckoutへの一時drive aliasをbuild中だけ設定して再実行した。`flutter build windows --debug --no-pub --dart-define=GUI_SHELL_PRODUCT_VERSION=1.2.3`は91.8秒で`gui_shell_desktop.exe`を生成しPASS（既存MSB8028中間directory共有warning 3件）。finallyでZ:を解除したことも確認し、CMake cache削除・恒久OS設定・OneDrive設定変更は行っていない。日本語基底strict監査は今回差分のfinding 0件、既存のrev3／rev4履歴およびCodex CLI診断の3 findingsを保持して終了値1。いずれもP11の製品受入れを追加せず、Installer／installed product経路の継続を妨げない。

#### 2026-10-06 P11 Uninstaller接続 — 実装済み（installed end-to-end未実証）

- SettingsのUninstaller要求は既存Rust Desktop native Owner確認→既存Broker operationへ接続した。Brokerはinstalled-path evidence、埋込みApp／Audit Store identity、Windows Known Folderから再導出した固定root／shortcut先、native確認時のpayload hashを照合し、durable削除意図Audit確定後だけ一回ticketを返す。LauncherはticketをFlutter応答から除き、正常終了後に起動した同一launcher hashのhelperへstdinで引き渡し、親Launcher終了後に保持されるBroker Audit Storeで未使用ticketを再検証してから固定App ID rootとそのrootを指すshortcutだけを削除する。Credential／設定／Workspace／Audit Storeは削除対象外。完了／失敗をAuditへ記録し、開始後の失敗・crash後に同ticketを再使用せず、新しいOwner確認を要求する。
- 正常fixtureは製品root・一致shortcutの削除とAppData Audit fileの保持を確認し、shortcut競合fixtureは製品rootとshortcutを保持して停止する。Broker fixtureはOwner確認なし・不正／未知ticket・開始前完了・失敗ticket再利用・完了後再利用を拒否し、失敗後の新Owner ticketとBroker再起動を挟んだticket非再利用を確認する。Launcher試験はOwner確認表示に固定root・保持dataを示し、raw ticketをFlutter応答へ出さないことを確認する。
- 検証: `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib uninstall_ -- --test-threads=1`は初回のAudit識別marker不一致を修正後、5件合格／0件失敗。`cargo build --locked --release --manifest-path native/rust_helper/Cargo.toml` PASS（未変更`minidora.rs`に`dead_code`警告2件）。Schema 161／正常例157／負例208、Conformance 236 checks PASS。Flutter更新画面試験8件、変更対象Dart 3 fileの静的解析、Mobile全体静的解析、Windows版Desktop Debug buildもPASS（MSB8028中間directory共有warning）。Dart形式確認は変更対象3 file・変更なし。Rust全体試験は489件合格／1件失敗／12件除外。唯一の失敗は差分外の既知A2A loopback fixtureで、単独再実行1件PASSし、既存`FQ-TEST-LOOPBACK`に追加記録する。Desktop全体の`flutter analyze --no-pub`は終了値1で、未変更`agent_center.dart`内の廃止予定API案内5件のみ。変更対象Dartの解析はPASS。厳格日本語監査は今回変更fileにfindingなし、現行差分外の履歴／診断3件を検出して全体終了値1。manifest 1178 file check、Release Gate check、`git diff --check`はいずれもPASS。
- これはProduct Buildの実装・局所Windows fixture・compile証拠であり、正式Installerからの導入、実installed product上のnative確認からuninstall完了までの`LIVE_RUNTIME`、外部プロセスcrash／電源断復旧を示さない。正式Installer、導入から次回起動・Uninstallerを含む統合経路、Repair、実installed Recoveryは引き続きP11残件。現finalizerはTemp内へ検証済み一時copyを作るが、処理後の自身の一時directory cleanupはまだ行わないため、`FQ-P11-UNINSTALL-FINALIZER-CLEANUP`へ送り、製品統合後に処理する。

#### Package format／reader基盤 — Broker fixtureへ接続済み（installed product未実証）

- `D4PKG01`固定無圧縮format、JSON Schema、Developer専用portable Export packager、Rust package readerを追加した。
- Rust readerは上限付きpackage／manifest、case-insensitive path順序・一意性、Windows path traversal／予約名／reparse point拒否、required payload、各file byte length／SHA-256、trailing byte、同梱Product ManifestのApp ID／Audit Store ID／新規Audit／Authority非継承を検証し、未存在stage directoryだけへ展開する。
- Python packagerも同梱Product Manifestのidentity／Authority条件を照合する。Builder／readerとも配布元署名を検証せず、trust・Permission・Approvalを生成しない。
- 個別証拠: `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib product_package -- --test-threads=1` 4件合格。`python -X utf8 tooling/schema_check/check_schemas.py`はSchema 159件／正常例156件／負例206件で合格。`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`は235 checksで合格。
- portable ExportのSettingsへsource `pubspec.yaml`由来の製品版を表示し、package metadataとFlutter compile defineに同じsource値を渡す局所経路を実装した。`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`（235 checks）、ASCII一時複製での`flutter test --no-pub --dart-define=GUI_SHELL_PRODUCT_VERSION=1.2.3 test/settings_compose_test.dart`（1件）、`flutter analyze --no-pub`（Mobileは問題なし、Desktopは未変更`agent_center.dart`の既存deprecated API情報5件で終了値1）、および変更Dart fileのformatter確認が得られた。OneDrive直下のFlutter testは`build/unit_test_assets`削除前に停止した。Desktop全体解析はchanged fileにfindingを出しておらず、対象Widgetはcompile・表示を通過した。日本語厳格監査は既存3 findings（旧rev3／rev4履歴および未変更CLI診断文字列）のため終了値1。version表示はbuild identityの投影だけで、installed product、署名・trustの証拠ではない。
- Installer／Uninstaller／正式導入first-run／installed product上のStart Menu／Rollback／Repairおよびinstalled product全体経路は未成立。署名済みpackageのstaging、active record／Start Menu切替、1世代Rollbackは既存BrokerのCapability／Permission／独立native Owner Approval／durable Audit経路へBroker fixtureとして接続済み。追加fixtureはstage prefix再開、完全stage再照合、相違prefix／余分なentry拒否を確認した。実installed productでの強制crash／電源断LIVE_RUNTIME復旧証拠は未成立。standalone SetupがBrokerを迂回してfilesystem／HKCUへ作用する案は廃止し、fixture単位の成立だけでP11受入れ、Windows Feature Complete、正式配布可を主張しない。
- 起動時のpackage layout検証とinstalled root検証を別状態へ分離した。portable bundleは固定package配置を満たせばBroker登録／固定storeの初回UI設定を利用できるが、installed pathのSetup Doctor項目は独立したinstalled-root evidenceがない限り`unknown`である。installed evidenceの昇格、Installer、実installed Recovery／Rollbackは未成立。active version切替は下記のBroker fixture経路へ接続したが、installed起動証拠ではない。
- 2026-10-06 P11導入先基礎: Windows Known Folder由来のLocalAppData、App ID、製品版、package SHA-256からper-user固定install pathを決定するtest-only計画modelと拒否試験を追加した。通常Releaseでは未接続であり、package展開、native Owner確認、Broker Permission／Audit、Start Menu、実installed起動の成立を意味しない。P11 OPENを維持する。
- 2026-10-06 P11 package consumer基礎: Broker consumer向けRust readerにcapabilityで開いた同一file handleの全package SHA-256検証とstage展開を追加し、digest不一致時のstage除去を試験した。これ自体は配布元署名、Broker Owner確認、install root公開、Update applyを実装したことを意味しない。
- 2026-10-06 P11 reader書込境界: 出力parent `Dir` capabilityと単一componentのstage名を受ける展開APIを追加した。stage directory identityを開いたhandleと照合し、package内directory／fileをno-followの相対操作だけで作成する。既存stage衝突を保持し、digest不一致およびfile/directory衝突ではidentity照合済みの自身のstageだけをhandle経由でcleanupする。stage名のtraversal／Windows予約名も拒否する。focused reader test 7件PASS、対象fileの`rustfmt --check` PASS。これはInstall／Update consumerへの接続、Broker Owner確認、Permission／Audit、installed productの証拠ではない。
- 2026-10-06 P11 capability-reader回帰: 非選別の逐次Rust suiteは465件中454 passed／2 failed／9 ignored。既存MINIDORA loopback fixtureは単独再実行でPASS、既存Update Download TLS fixtureは単独でもOS error 10054／`ConnectionReset`でFAIL。双方とも本差分外として既存`FQ-TEST-LOOPBACK`へ記録し、P11受入れのblockerにはしない。Schemaは159 schema／156 example／206 negative fixture、Conformanceは235 checksでPASS。厳格日本語監査は終了値1の既知3 findingsのみで、今回変更した現行文書・readerにfindingなし。
- 2026-10-06 package consumer reader検証: `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib product_package::tests -- --test-threads=1`は5件合格。非選別のRust全試験は453件合格／1件失敗／9件ignoredで、失敗は既存`broker::update_download::tests::local_tls_server_repairs_only_after_verified_package_bytes`のOS error 10054／`ConnectionReset`。この同じ試験を単独実行すると1件合格した。fixture競合の原因は未確定で、現行P11のreader差分外かつ既存`FQ-TEST-LOOPBACK`に属するため、P11 blockerへ追加しない。Schema（159件／正常156件／負例206件）、Conformance（235 checks）、変更fileの日本語監査、`git diff --check`は合格。
- 2026-10-06検証: portable判定Rust試験1件、Schema（159件／正常156件／負例206件）、Conformance（235 checks）はPASS。Rust全試験の逐次実行は449 passed／1 failed／9 ignoredで、失敗は既存P7 A2A loopback fixtureの応答期限超過。同test単独再実行はPASSし、原因は未確定。並列全試験のUpdate Download TLS fixture接続resetも逐次再実行では再現しなかった。現phase blockerにはせず`docs/FINAL_QA_QUEUE.md`へ記録。厳格日本語監査は既知3 findingsのみで、変更fileにfindingなし。`git diff --check`はPASS。Rust formatter確認は既存コード領域の未整形差分で不合格となり、広域整形は行っていない。

2026-10-06 P11更新配布物の未起動版保存処理: `更新適用要求`をRust Desktop起動器のOwner確認からRust Brokerへ接続した。Brokerは現在の信頼設定と署名済み候補、build時に埋め込んだApp ID／Audit Store ID、Windows既知フォルダーAPIから導出する導入先を再照合する。展開前後の永続Auditが確定した後だけ、利用者単位の固定`versions` Capability内へ、同一ファイルハンドルから配布物を展開する。成功状態は`version_staged`、有効化状態は`suspended`。既存版は上書きせず、Start Menu、有効版切替、プロセス起動、rollbackは行わない。Flutter表示も未起動版の展開完了と、切替／起動／rollbackの保留を示す。

対象Rustは`cargo check --locked --manifest-path native/rust_helper/Cargo.toml`と`cargo build --locked --release --manifest-path native/rust_helper/Cargo.toml`がPASS。Release buildは未変更`minidora.rs`のdead_code warning 2件を出したが生成は完了。署名済み配布物のBroker staging境界test 1件、固定versions capability 4件、Owner確認文test 1件PASS。実Win32 Owner dialogのNo／Yes自動操作testも明示実行で1件PASS。Flutter `update_client_test.dart` 4件PASS、変更Dart fileのformatter PASS。Desktop analyzeはrepository-root ASCII junction経由で全体実行し、今回変更file finding 0、未変更`agent_center.dart`にdeprecated API info 5件で終了値1。Mobile analyzeは同経路で問題なし。

Rust全target逐次実行はlib 469件中456 passed／3 failed／10 ignored。A2A loopbackと`failed_replacement_keeps_the_existing_corrupt_package_unchanged`は各単独再実行PASS、`local_tls_server_repairs_only_after_verified_package_bytes`は単独でもOS error 10054／`ConnectionReset`で失敗。原因未確定の既存localhost fixture問題として`FQ-TEST-LOOPBACK`へ記録し、今回のstaging変更による回帰とは観測されず、P11受入れblockerへ加えない。

Schemaは159件／正常156例／負例206件、Conformanceは235 checksでPASS。strict日本語監査は既存履歴／診断文の3 findingsで全体終了値1、今回変更fileのfindingsは0。6つの変更Rust source fileは子moduleを展開しない`rustfmt --check`でPASS。既存書式を保った`workspace_protocol.rs`は呼出し引数1行だけを追随修正し、`git diff --check`もPASS。正式installed product、実配布元download、installed process crash／電源断後のLIVE_RUNTIME Recovery、有効版／Start Menu切替、Installer／Uninstaller、起動／Rollbackの証拠は未成立で、既存Windows配布`release_blocker`を維持する。

2026-10-06 P11部分展開Recovery: Broker consumerは、native Owner確認後の再適用時に固定destination内のstage directoryをno-follow capabilityで開き、packageの各file byte列と既存fileの全prefixを逐次比較する。一致する途中fileだけを残りbyte追記で完了し、完全stageは同じhash／manifest検査を冪等に再実行する。stage全体を走査してpackage外file／directoryやreparse pointも拒否する。相違prefix、異常file、余分なentryは上書き・削除せず保持し、Audit付き失敗でfail-closed。`resume=true`は応答とcompletion Auditへ記録する。権限境界は従来どおり毎回のnative Owner確認、現在trust再照合、Permission、durable Auditを維持する。

部分stage試験: `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib product_package::tests -- --test-threads=1`（8 passed）、`cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib native_owner_apply_stages_exact_signed_package_without_activation_or_overwrite -- --test-threads=1`（1 passed）。Rust test fixtureで途中file prefix、完全stage再要求、相違prefixを保持して拒否、余分entryを保持して拒否、およびBroker consumer経由の既存stage再開を確認。これは実process強制終了・電源断を起こすinstalled product `LIVE_RUNTIME`証拠ではない。

再開処理の統合検証: `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`は全targetで506 passed／0 failed／11 ignored。既存loopback試験を含め今回の実行は全成功。`cargo build --locked --release --manifest-path native/rust_helper/Cargo.toml`はPASSし、未変更`minidora.rs`のdead_code warning 2件を記録。Schema（159件／正常156例／負例206件）、Conformance（235 checks）、Manifest（1171 file）、変更Rust 2 fileの`rustfmt --check`、`git diff --check`もPASS。strict日本語監査は既存3 findings（rev3履歴、rev4履歴、未変更CLI診断文）のみで、今回の変更fileは0 finding。部分stage fixture合格は実installed process crash／電源断の証拠へ昇格しない。

2026-10-06 P11固定root Bootstrapper読取経路: `d4_pocket_active_version.schema.json`と有効版recordの正常／任意executable path負例を追加し、Windows起動器がKnown Folder由来の固定製品rootでのみrecordをno-follow capabilityから読み、version／package hashで導出したdirectory内のversion-local launcherとProduct Manifest hashを照合して起動する分岐を接続した。portable配置とversion-local起動は既存sibling layoutのまま。root／stageのnon-reparse・file identity・同一volume、recordの重複／未知JSON field、identity、version、hash mismatch、required package layoutをfail-closedで検査する。`open_existing_product_root`は起動時にdirectoryを新規作成しない。active recordはAuthorityではない。

局所試験: `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib product_bootstrapper::tests -- --test-threads=1`（5 passed）、`cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib broker::product_install::tests -- --test-threads=1`（5 passed）、`cargo test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`（512 passed／0 failed／11 ignored）、`cargo build --locked --release --manifest-path native/rust_helper/Cargo.toml`、Schema（160 schema／157正常例／207負例）、Conformance（236 checks）、Manifest（1171 file）、`git diff --check`がPASS。初回全targetでは入口識別fixtureに固定起動器fileがなく1件失敗したため、fixtureへ実fileを追加し、focused 5件と全targetを再実行してPASSした。Release buildは未変更`minidora.rs`のdead_code warning 2件を出した。全crate `cargo fmt --check`は今回差分外の多数の既存未整形箇所で終了値1となった一方、変更した`product_bootstrapper.rs`と`product_install.rs`の局所`rustfmt --check`はPASS。strict日本語監査は全体終了値1で既存3件（rev3履歴、rev4履歴、未変更CLI診断）を報告し、今回の変更fileは0件。ここはBroker有効版record writer、native Owner切替操作、初回Bootstrapper配置を接続する前の履歴であり、現行状態を示す記述ではない。

2026-10-06 P11独立有効版record切替: `更新有効版切替要求`をstage operationとは別のBroker operation・native Owner確認で接続した。Brokerは現在の署名候補／trust、package digest／byte長、App／Audit identity、Known Folder由来の固定destination、永続Auditを再照合し、同じ署名packageと既存stageの全byte／inventoryをread-onlyで比較する。不一致stageは修復・削除しない。一致時のみ固定root Bootstrapperを未配置なら配置し、`active_version.json`をtemporary file同期後に同一root renameで公開する。応答はrecord公開までで、process起動、Start Menu変更、旧版削除、Rollbackは行わない。

検証: Windows native Owner dialogのNo／Yes自動操作1件PASS、Bootstrapper初回配置／既存配置保持とactive record公開・置換2件PASS、Broker stage-only／activation分離fixture 1件PASS、read-only stage検証1件PASS、Flutter `test/update_client_test.dart` 4件PASS、変更Dart 4 file format確認PASS。最初の全Rust実行では新規Windows test fixtureがdirectory capabilityを開いたまま掃除して2件失敗し、handleを閉じる修正後にfocused 2件PASS。既存download TLS testはWindowsでbody受信前にserver threadをjoinするとWSA 10054となったため、test fixtureの順序だけを修正しfocused testはPASS。最初の統合再試行で変更外MINIDORA localhost testが一度失敗したが、focused再実行と全target再試行ではPASS。最新`cargo test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`は517 passed／0 failed／12 ignored。通常installed productのnext-launch、Installer／Uninstaller、Start Menu、crash／電源断Recovery、Rollbackおよびrelease readinessの証拠ではなく、P11はOPENを維持する。

追加検証: `python -X utf8 tooling/schema_check/check_schemas.py`は160 schema／157正常例／207負例、`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`は236 checks、`python -X utf8 tooling/manifest.py --write`後の`--check`は1175 fileでPASS。変更Dart 3 fileの`flutter analyze --no-pub`は問題なし、Windows `flutter build windows --debug --no-pub`は成功しRunner C++を含むdebug executableを生成した。既存build cacheが過去の`Z:/apps/desktop_flutter`絶対pathを保持していたため、削除せず一時外部build directoryを使い、Flutter設定を元の未設定状態へ復元した。Flutter全体analyzeは変更外`agent_center.dart`の既存deprecated API info 5件で終了値1。Windows debug buildはinstalled product、署名済み配布物、next-launch、Installer、LIVE_RUNTIMEの証拠ではない。`git diff --check`もPASS。

2026-10-06 P11有効版切替のStart Menu接続: native Owner確認に固定Start Menu shortcut登録先を追加し、Brokerが実行時に現在利用者のKnown Folder、App ID、固定rootと照合する。完全検証済みstageから固定root Bootstrapperを配置し、有効版recordを公開した後、そのBootstrapperを指すshortcutをBroker capability経由でcreate-only登録する。同targetへの再実行は冪等、既存の別target shortcutは上書きせず拒否する。登録失敗時は失敗Auditを残し、stage／active record／既存shortcutを自動削除・復元しない。COMへ渡すpathだけextended Windows path prefixをShell互換形式へ正規化し、path比較・Broker境界の検証値は変えない。

検証: `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib broker::product_install::tests -- --test-threads=1`は8 passed（shortcut作成、target照合、冪等再試行、競合非上書きを含む）。`cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib broker::update_center::tests::native_owner_apply_stages_exact_signed_package_without_activation_or_overwrite -- --test-threads=1`は1 passed（stale shortcut先拒否、有効版record／shortcut登録、再試行）。native Owner確認の実Win32 dialog自動操作も1 passed（No／Yes）。`cargo test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`は517 passed／0 failed／12 ignored。最初のshortcut作成試験ではWindows canonical pathの`\\?\` prefixをShell Link `SetPath`が拒否したため、COMへ渡すpathのみ正規化してfocused試験と全targetを再実行した。

Desktopの`update_client_test.dart`は4 passed。通常OneDrive worktreeからのFlutter testはCloud Files reparse placeholder `build/unit_test_assets`をFlutterが削除できず終了したため、このdirectoryを変更せず、同一source／testとlocal packageだけを一時scratchへ複製して再実行しPASSした。変更対象Dartの`dart format --output=none --set-exit-if-changed`と限定`flutter analyze`は変更なし／問題なし。Desktop全体analyzeは未変更`agent_center.dart`の既存deprecated情報5件で終了値1、Mobile全体analyzeは問題なし。Windows `flutter build windows --debug --no-pub`はPASSし、外部scratchから生成したdebug executableでDesktop変更をcompile確認した。Flutter build-dir設定は元の未設定へ復元した。Schema（160／157／207）、Conformance（236）、Manifest（1175）もPASS。厳格日本語監査は既存履歴／診断文3 findingsのみで、今回変更fileに新規findingなし。これらはBroker fixture／debug buildの証拠であり、installed product上のStart Menu・次回起動、Installer／Uninstaller、Rollbackの証拠ではない。P11はOPENを維持する。

2026-10-06 P11 Broker記録の直前版への復帰: Windows版Rust起動器で独立したnative所有者確認を行い、Brokerが現行版・直前版候補の署名と内容識別値、製品／監査識別子、固定導入先、両版の展開内容を再検証する。永続監査に意図を記録した後、`active_version.json`だけを原子的に切り替える。統合試験では1.1.0から1.0.0への復帰、古い対象と通常IPCの拒否、展開済み版の保持、Start Menu不変更、完了後の正確な内容識別配布物だけの除去を確認した。native確認画面の拒否／承認試験1件、更新制御15件、起動器8件、配布物除去1件、Flutter更新画面4件が合格。最終`cargo test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`はRust library 476 passed／0 failed／12 ignored、全target合計522 passed／0 failed／13 ignored。中間集約試験ではCodex loopbackがHTTP応答見出しの到着前に`ConnectionReset`となったが、単独再試験1件と後続全targetは合格した。別の中間回では`broker_ipc`が9 passed／1 failedだったが、後続全targetでは10件すべて合格した。P11変更との因果は観測されず、loopback事象は既存`FQ-TEST-LOOPBACK`へ記録する。Rust Release buildは合格（未変更`minidora.rs`の未使用code warning 2件）。Conformanceは当初、inline `#[cfg(test)]`で実装sourceを誤って切り落としていたため検査器をtest module境界で分割し直し、236 checks合格。Schema 160／157／207も合格。Desktop変更対象の限定analyzeとDart形式確認は合格。全crate Rust形式確認は既存の多数の未整形差分により終了値1で、全体整形は行っていない。厳格日本語監査は既知3 findingsのみ。これはBroker試験とnative画面境界の証拠であり、installed productでの復帰／次回起動、Installer／Uninstaller、P11完了を意味しない。P11はOPEN、配布`release_blocker`を維持する。

2026-10-06 P11 version-local起動器の導入path照合: Rust起動器の通常起動経路で、自身の実行fileをKnown Folder由来の固定product rootにあるactive recordの選択launcherと照合し、App ID／Audit Store ID、版directory、launcher／Product Manifest hashを再検証する。一致時だけBrokerのSetup Doctor向けinstalled-path観測を`LIVE_RUNTIME`として構成する。portable起動、別identity、root Bootstrapper本体、無関係launcherは未確認のままにする。この観測はSetup Doctor表示専用で、Authorityや配布元trustを作らない。focused test 1件PASS、Schema 160／157／207 PASS、Conformance 236 checks PASS、Manifest 1175 file PASS、Release build PASS（未変更`minidora.rs` dead_code warning 2件）。Rust helper全testは475 passed／1 failed／12 ignoredで、失敗した既存Codex loopback testは単独再実行で1 passedし、既存`FQ-TEST-LOOPBACK`と同分類。strict日本語監査は今回変更fileにfindingなし、既存履歴・診断3 findingsで終了値1。これはinstalled productの一連の起動LIVE_RUNTIME証拠ではない。正式Installer／Uninstaller、Start Menuからの製品起動、Repair、P11受入れは引き続き未成立。

2026-10-06 P11初回Install／Update導線: BrokerはRollback先のないactive versionも現行candidate／trust照合後に`現在版`へ投影する。Flutterは`unavailable + 現在版=null`を初回Install、active versionありをUpdateとして区別し、同じdownload jobの更新ID・候補hash・package SHA-256が一致しないstage要求を無効化する。Brokerが`version_staged`を返した場合だけ画面内のstage印を置き、別Owner確認による有効版切替を有効化する。このUI stateは表示制御だけでAuthorityではなく、Brokerは各操作で再検証する。Flutter update client 7件、Broker導入fixture 1件、変更file analyze、Dart形式、Schema 160／157／207、Conformance 236 checks、Release build、Manifest 1175 fileがPASS。Mobile全体analyzeはPASS。Desktop全体analyzeは未変更`agent_center.dart`のdeprecated info 5件のみで終了値1、変更対象`settings.dart`／`update_client.dart` analyzeはPASS。Rust全testは474 passed／2 failed／12 ignored。失敗した既存A2A／MINIDORA loopback testは個別再実行で各1 passedし、`FQ-TEST-LOOPBACK`へ送る。strict日本語監査は今回変更fileにfindingなし、旧履歴・既存診断の3 findingsで終了値1。初回導入からinstalled product起動までのLIVE_RUNTIME、正式Installer／Uninstaller、Repairは未成立で、P11 OPENを維持する。

2026-10-06 P11 配布元catalog取得導線: Desktop Update Centerの明示操作から新しいBroker operation `更新候補取得`を呼び出し、Broker所有trustに事前登録されたchannel別HTTPS配布元の`updates.json`だけを取得する経路を追加した。Flutterからの直接network access、候補指定URL、redirect、system proxy、自動retry、圧縮応答を使わず、HTTPS証明書／hostname検証、public-address DNS pinning、各catalog 64 KiB／一回の全配布元取得30秒の上限を適用する。catalogはSchema版1・重複field拒否で解析し、全sourceの取得に成功した後、各候補のchannel／更新ID一意性とBroker Ed25519署名を検証したものだけを一括永続化する。いずれかのcatalogが不正なら既存候補を変更しない。利用者明示のread-only metadata取得を`更新候補取得` capability、Broker所有`package_sources`／HTTPS／public-address境界をpermissionとして記録する。package download／Install Approvalは不要なまま独立native Owner確認を維持する。Auditは配布元件数・catalog hash・候補件数だけを記録し、本文や資格情報を保存しない。失敗時は既存候補を保持し、trust／配布元確認後の明示再試行をRecoveryActionとする。未構成時は通信せず`unconfigured`を返す。

検証: Rustの`broker::update_center::tests`19件と`broker::update_download::tests`17件がPASS。Schema 161件／正常例157件／負例208件、Conformance 236 checks、Manifest 1175 fileもPASS。Rust Release buildはPASS（未変更`minidora.rs`にdead_code警告2件）。Desktopの`settings_compose_test.dart`と`update_client_test.dart`は製品版識別値を明示した一時作業複製で計9件PASS。変更対象Dart 2 fileの`flutter analyze --no-pub`とWindows Desktop Debug buildもPASS（中間出力先に関するMSB8029警告）。初回のFlutter実行では既存Compose試験が期待する製品版識別値を渡さず1件失敗したため、値を明示して再実行した。HTTPS上限の異常系試験はCargo試験並行中に一度Windows socket error 10054で接続切断したが、単独および逐次再実行でPASS。厳格日本語基底監査は全体終了値1で、既存の旧履歴2件と既存Codex CLI診断1件のみが残り、今回変更fileの指摘は0件。導入済み製品の実配布元接続、正式Installer／Uninstaller、導入済み起動・Rollback、Repair、正式配布判定を証明するものではなく、P11はOPENを維持する。

2026-10-06 P11 Windows payloadの現行main再build: clean `main`／`origin/main`一致commit `3a578519a2d4364a2f6fc0478b49474612fce2e3`から、checked-in合成Export Receipt／Manifest fixtureを用いて`tooling/export_windows_product.py`を実行した。Receipt指定のManifest file名と実file名が異なる初回試行は入力検査で停止し、内容を変えずに一時directory内で期待basenameへ複製して再実行した。Flutter Windows Releaseはbuild成功（MSB8029一時directory警告）、Rust Release Broker／起動器もcompile成功（未変更`minidora.rs`のdead_code警告2件）。生成bundleは14 file、63,571,666 bytes、artifact tree SHA-256 `8326087ea2191dec71421e1af99da0c7b4cd7964b4efe83dab1535c19aae9fb2`。続けて`tooling/package_windows_product.py`がD4PKG01を生成し、14 file／63,573,871 bytes、package SHA-256 `8ecee8358325d5ac0ac167eb1efa92e5c90301c2dc27901574fd927a94a6126a`を報告した。成果物はTemp内の隔離開発artifactで、synthetic Receipt／Manifest由来の試験identityを持ち、Owner承認済みExport、package署名、実インストール、起動、Update／Rollback、正式配布の証拠ではない。P11はOPENを維持する。

2026-10-06 P11 portable起点Install後の導入版起動遷移: Brokerが`更新有効版切替要求`を`Accepted`し、`有効化=active_version_recorded`を返した場合に限り、portable起動元のRust起動器が応答へ`起動=after_current_exit`を付加する。製品identityがないbuild、導入済み版からの更新、rollback、拒否応答では付加しない。Flutterはこの明示応答後に案内を表示して画面を正常終了し、Rust起動器はBroker停止と現在processの正常終了を待つ。続いて固定rootからactive version launcher／package layoutを再照合し、instance lock解放後に限定環境で起動する。起動先Brokerは検証済みinstalled entrypointの起動を`LIVE_RUNTIME` Auditへ記録する。初回Install後の起動遷移とSetup Doctor provenance表示をつなぐ実装であり、実installed productを通した一連のLIVE_RUNTIME成功を示すものではない。

検証: Rust focused `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib activation -- --test-threads=1`は7 passed／0 failed／1 ignored。`cargo build --locked --release --manifest-path native/rust_helper/Cargo.toml`、Flutter `flutter test --no-pub --dart-define=GUI_SHELL_PRODUCT_VERSION=1.2.3 test/update_client_test.dart`（8件）、変更Dart 2 fileの`flutter analyze --no-pub`、変更3 fileのDart format確認、`flutter build windows --debug --no-pub --dart-define=GUI_SHELL_PRODUCT_VERSION=1.2.3`、Schema（161／157／208）、Conformance（236 checks）、Manifest（1178 file）がPASS。Debug buildはTemp作業複製から成功し、MSB8029（一時directory build warning）を出した。Rust `--all-targets`逐次実行のlibrary suiteは484 passed／1 failed／12 ignoredとなり、Cargoは当該library targetの失敗後に非zero終了した。失敗は今回の変更file外の既知`broker::update_download::tests::local_tls_server_repairs_only_after_verified_package_bytes`（Windows loopback TLSの`ConnectionReset`／`InvalidContentType`）であり、現行`FQ-TEST-LOOPBACK` Final QA分類に留める。日本語基底strict監査は今回変更fileにfindingなし、既存rev3／rev4履歴とCLI診断文の3 findingsにより全体終了値1。installed product実起動、正式Installer／Uninstaller、Repair、実配布署名は未成立で、P11 OPENとWindows distribution `release_blocker`を維持する。

起動許可はBroker応答status・active record公開・元要求とのrequest ID／operation一致へ結合し、staleまたは別operation応答からは生成しない。

2026-10-06 P11 初回Install引継ぎの重複防止と終了失敗表示: Update Centerの状態変更を一件に直列化し、要求中はcatalog取得、download、stage、active version切替、延期、rollback、Uninstallを無効化する。Brokerが初回Installのactive version record／Start Menu登録を受理し`起動=after_current_exit`を返した場合、Flutterは必須終了を一度だけ要求する。終了が拒否／失敗した場合は導入操作そのものを失敗と誤認せず、画面内操作を再開してStart Menu起動を案内する。`_loadUpdates`の`setState` callbackからFutureが返る例外も修正した。

検証: `flutter test --no-pub --no-test-assets --dart-define=GUI_SHELL_PRODUCT_VERSION=1.2.3 test/settings_install_flow_test.dart test/settings_compose_test.dart test/update_client_test.dart`は12件PASS。testはBroker transport fixtureと注入した終了応答を使い、初回stage→切替→必須終了要求、二重tapの単一要求化、Uninstall中の状態変更直列化、終了拒否後の未削除表示と画面操作復帰を検証する。実Broker、Rust起動器、installed product起動のLIVE_RUNTIME証拠ではない。Schema 161件／正常例157件／負例208件、Conformance 236 checks PASS。Mobile全体`flutter analyze --no-pub` PASS。Desktop全体解析は変更外`agent_center.dart`の既存deprecated API info 5件で終了値1、変更ファイルにfindingなし。OneDrive上のFlutter C++ wrapperがCloud Files placeholderとなり通常checkoutのWindows buildは生成source file欠落で失敗した（source repoとOneDrive設定は変更せず）。tracked Flutter appとpath dependencyだけを生成cache・ephemeral directoryを除外してASCII Tempへ複製し、`flutter pub get`と`flutter build windows --debug --no-pub --dart-define=GUI_SHELL_PRODUCT_VERSION=1.2.3`を実行、Windows Debug executable生成までPASSした。MSB8029一時build directory警告あり。これはhost-local compile証拠でありinstalled product起動やLIVE_RUNTIME証拠ではない。Strict日本語監査は既存履歴／診断3 findingsのみで全体終了値1、今回変更fileのfindingsは0。これはOneDriveの同期／生成物配置制約で、製品回帰やrelease blockerには分類しない。

#### 2026-10-06 P11 固定起動項目の限定修復 — Broker fixture／UI接続済み

Settingsにある明示的な「起動項目を修復」操作を、Rust Desktop起動器のnative Owner確認、既存の認証Broker、Brokerの現在trust／active record再照合、永続Auditへ接続した。修復対象は欠損した固定root Bootstrapperと、同rootを指す固定Start Menu shortcutだけである。Brokerは現行候補の署名trust、active record、製品identity、Known Folder由来のroot／shortcut先、要求hashを照合し、さらに展開済みstage全体から固定形式D4PKG01 manifestとouter SHA-256を再構成して現在trust済み候補へ照合する。これによりactive record内の自己申告hashだけを修復根拠にしない。Bootstrapperはpackage digest一致後、欠損時だけ現行stageからhard linkで復元し、既存fileやshortcutが別内容・別targetなら上書きせず停止する。版本体、任意stage file、active record、Credential、設定、Workspace、Audit Storeの復元・変更はしない。

検証: `cargo test --locked --manifest-path native/rust_helper/Cargo.toml product_launch_repair_restores_only_missing_entries_after_current_trust_and_owner_confirmation`、同manifestの`staged_tree_reconstructs_signed_package_digest_and_rejects_payload_changes`、`product_repair_confirmation_limits_repair_to_missing_launch_entries`は各1件PASS。stage treeから署名対象package outer digestを再構成し、payload改変・余分なfileを拒否する試験、Brokerの通常IPC拒否・古いpath確認拒否・非launcher stage payload改変拒否・起動器／shortcut復元・active record不変・Audit・冪等再要求・既存競合非上書き試験、native Owner確認文scope試験が成立した。Flutter `settings_install_flow_test.dart`／`update_client_test.dart`は11件PASS。ASCII一時作業複製上の変更Dart限定解析、Windows Runnerを含むDesktop Debug build、Schema 161件／正常例157件／負例208件、Conformance 236 checks、Rust Release buildもPASS。OneDriveの日本語pathからのFlutter analyzerはLSP応答JSON parse errorで異常終了したためASCII scratchで再実行した。scratchで`--no-test-assets`を付けた初回試験はshader test asset欠落で失敗し、test assetを通常生成する再実行でPASSした。package reconstruction追加時、試験fixtureのJSON field順序がPython packagerと不一致でfocused試験が一度失敗したためfixtureを正本順へ修正し再実行PASS。より前の試作で不完全なtoy stageを使った単体fixtureは実D4PKG01検証条件を満たさず、実packageを展開するBroker統合fixtureへ集約した。Rust Release buildの今回差分外warningは既存MINIDORA helper 2件。`product_package.rs`単体の`rustfmt --check`はPASS。変更Rust file全体の`rustfmt --check`は隣接する既存未整形箇所も含むため終了値1で、広域formatは行っていない。strict日本語監査は全体終了値1、既存finding 3件（旧rev3文書、旧rev4文書、未変更Codex CLI診断文字列）のみで、今回変更fileのfindingは0件。Manifest 1179 fileとRelease Gate checkはPASS。この操作は合成installed-root fixtureとWindows compileの証拠であり、実installed productからのLIVE_RUNTIME修復、Installer、全payloadの再取得・修復、破損active recordのRecoveryを証明しない。完全Repairとinstalled end-to-endはP11の後続作業として残し、P11はOPENを維持する。

#### 2026-10-06 P11 有効版payloadの限定Repair — Broker fixture／Settings接続済み

Settingsの「有効版を修復」は、現在activeな候補と完全一致する署名検証済みdownloadだけを選び、既存のnative Owner確認付きBroker `更新適用要求`へ接続する。Brokerは実行直前に現在trust、candidate、package hash／長さ、固定version destinationを再検証し、durable intent／結果Auditを記録する。既存fileとpackage byte列が一致する場合だけ欠損byteを補い、欠損fileを復元する。異なるbyteの既存fileは拒否し、上書きしない。有効版record、Start Menu、process起動／終了、別版への切替はこの操作で変更しない。完全なpayload byte修復やactive record再構築ではない。

検証: `cargo test --locked --manifest-path native/rust_helper/Cargo.toml native_owner_apply_stages_exact_signed_package_without_activation_or_overwrite -- --test-threads=1`は1件PASS。この既存Broker integration fixtureに、active versionのfile欠損回復、同一長の改変byte拒否・未上書き、有効版record／launcher解決の不変、成功／失敗Auditを追加した。初回試行では長さ違いのtamperがより早いinvalid-file判定に到達したため、同じ長さの異なるbyteへfixtureを修正し、目的のmismatch拒否を確認した。Flutterの`flutter test --no-pub test/update_client_test.dart test/settings_install_flow_test.dart`は13件PASS。ASCII scratch上で変更Dart 4 fileの`flutter analyze --no-pub ...`はNo issues found。`flutter build windows --debug --no-pub --dart-define=GUI_SHELL_PRODUCT_VERSION=1.2.3`もASCII scratch上でPASSし、Debug executableを生成した（MSB8029: Temp中間directoryのwarning）。Dart format checkは4 file変更なし。変更Rust file全体の`rustfmt --check`は既存の未整形箇所を含み終了値1のため広域整形せず、focused Cargo testは通過した。これはBroker／Flutter fixtureとcompile証拠であり、installed productのLIVE_RUNTIME修復やP11全体の完了を示さない。P11はOPENを維持する。

#### 2026-10-06 P11 portable ZIP packaging — 実装／fixture適合確認済み

検証済みWindows Export配置一式から、`D4 Pocket/` root付き標準ZIPを作る開発専用tool `tooling/package_windows_portable.py`を追加した。全payloadをExport build receiptのinventory／hash／限定Credential scanへ照合し、現在のclean `main` commitと製品版も確認する。ZIPにはpayload、byte-for-byte build receipt、日本語起動案内を含め、entry名／件数／byte長／SHA-256／CRCを読み戻して検査する。既存outputを上書きせず、Repository／OneDrive／入力bundle外だけへ出力する。既存の手動Windows Export workflowにも接続し、workflow_dispatchの一run内でfixture由来Windows Release bundle→ZIP生成→ZIP hash照合を行う。ZIP／bundle／fixture Manifestは一時runner内で削除し、artifact uploadはしない。

ローカル証拠: `python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`は236 checks PASS。正常fixtureのZIP構成・receipt保持・hash、保護注意文、未署名／正式配布未主張、およびreceiptと異なる改変payloadの拒否を確認した。`python -X utf8 tooling/schema_check/check_schemas.py`は161 schema／157 example／208 negative fixture PASS、`python -X utf8 -m py_compile tooling/package_windows_portable.py tooling/conformance_tests/run_conformance_skeleton.py`、`python -X utf8 tooling/manual_workflow_check.py`、`python -X utf8 tooling/release_gate_check.py --release-track windows_v1`、`python -X utf8 tooling/manifest.py --check`、`git diff --cached --check`もPASS。厳格日本語監査は今回変更fileにfindingなし、旧rev3／rev4履歴および未変更CLI診断の既知3 findingsにより全体終了値1。

手動Windows hosted証拠: [workflow run #10](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37443194498)は`workflow_dispatch`でcommit `b87544b022e9a9386952b59700672ef42dadefd0`を固定し、Windows runner `win25-vs2026/20260925.250.1`で11分5秒、全step成功。合成Receipt／ManifestからWindows Release bundle 14 file／63,785,682 bytes（tree SHA-256 `b5c7584d26f3bb8857b70df4f50bd7bbd64e65b3ad58db8c3d038a0b5706176d`）を生成し、portable ZIP 16 entries／24,497,099 bytes（SHA-256 `123fc5b4c8ce57ffd813911f20f285217447ec431b9301e7103a02387613f1ab`）を検証した。Credential scanは実装済みknown-pattern集合で0 finding。ZIP、bundle、fixture Manifestはrunnerの一時領域から削除され、artifact uploadなし。これはfixture入力によるRelease bundle／ZIP生成のWindows hosted証拠であり、Owner権限、正式Export、installed product、実起動、署名、Installer、Update／Rollback、Credential不存在一般、release readinessを証明しない。P11はOPENのままとする。

#### 2026-10-06 P11 製品Exportのcompile-time update trust境界

製品Export Manifest fileに任意の`update_trust`を追加できるSchemaとRust build経路を接続した。指定時は版2 Ed25519公開鍵の固定DER prefix／fingerprint一致、固定HTTPS source、channel一意性をExport前と製品Broker起動時に検査し、公開鍵・配布元JSONをRust Brokerのcompile-time設定へ渡す。Build evidenceにはfingerprintとsource件数だけを記録し、秘密署名鍵は入力・bundle・evidenceへ含めない。製品Brokerはcompile-time product identityがあるbuildでは永続`update_trust.json`をtrust源として読まず、Manifest trust未指定なら永続fileに値があっても未構成を維持する。部分identity、破損trust、版1／配布元なしは起動時fail-closed。汎用GUI Shellは従来の永続trust読込を保つ。

検証: `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib broker::update_center::tests:: -- --test-threads=1`は25件PASS（compile-time trust優先、製品trust未設定時の永続file無視、汎用Broker互換、破損／部分設定拒否を含む）。合成Ed25519公開鍵・fingerprint・sourceを環境へ渡したWindows `cargo check --locked --manifest-path native/rust_helper/Cargo.toml --lib`もPASSし、production binaryにはせずRust compile-time設定を型検査した。Rust全体試験は509件対象で496 passed／1 failed／12 ignored。失敗は変更外の`broker::update_download::tests::failed_replacement_keeps_the_existing_corrupt_package_unchanged`でWindows loopback TLSがsocket 10054 `ConnectionReset`となったもの。該当単独再実行は1 passedで、原因未確定の最初の失敗を保持する。`python -X utf8 tooling/schema_check/check_schemas.py`は161 schema／157 example／208 negative fixture PASS、`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`は236 checks PASS、`python -X utf8 -m py_compile tooling/export_windows_product.py tooling/conformance_tests/run_conformance_skeleton.py` PASS。ConformanceではExport Receipt hash binding、公開鍵fingerprint一致、evidenceとbyte-for-byte product Manifestの結合、改ざんsummary拒否を確認した。`python -X utf8 tooling/manual_workflow_check.py`、`python -X utf8 tooling/release_gate_check.py --release-track windows_v1`、`python -X utf8 tooling/manifest.py --check`、`git diff --check`もPASS。厳格日本語基底監査は全体3 findings／3 filesで終了値1、該当は旧rev3／rev4履歴文書と未変更Codex CLI診断文字列のみで、今回変更fileは0 finding。`rustfmt --edition 2021 --check`はRust module tree上の既存未整形箇所で終了値1となり、一括整形はしていない。これらはunit／fixture／compile／構造検査であり、設定trustを使う製品起動、配布元到達、実際の更新・署名運用、正式Publisher identityを証明しない。Production trust provisioning・installed Update・Installerはrelease blockerのままで、P11はOPEN。

#### 2026-10-06 P11 概要から初回導入・更新センターへの導線

Dashboardに「インストール・更新センターを開く」を追加し、設定内Update Centerへ直接移動して該当位置を表示する。これはFlutter内の画面遷移だけで、catalog取得やfile／process操作を自動開始しない。実際の候補取得は利用者の明示操作、導入・更新は既存Rust Brokerとnative Owner確認を維持する。Update CenterのBroker未接続時は操作不可の説明だけを表示する。

検証: Desktop Flutterで`flutter test --no-pub --no-test-assets --dart-define=GUI_SHELL_PRODUCT_VERSION=1.2.3 test/settings_install_flow_test.dart test/settings_compose_test.dart test/update_client_test.dart test/widget_test.dart`を実行し74件PASS（初回Install／Update関連、画面遷移を含む）。`python -X utf8 tooling/schema_check/check_schemas.py`は161 Schema／157正常例／208負例PASS、`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`は236 checks PASS。変更4 Dart fileの`dart format`確認PASS。対象fileの`dart analyze`は診断表示前にAnalysis Server shutdownが`%LOCALAPPDATA%/Dart/perf/<pid>`の削除でWindows OS error 1920となり、解析結果を得られなかった。strict日本語監査は既存履歴2件と未変更Codex CLI診断の3 findingsだけで終了値1となり、今回変更fileにはfindingなし。manual workflow、release gate、manifest、diff検査はPASS。host解析制約はそのまま記録する。画面導線は`FIXTURE` evidenceであり、正式Installer、実配布元、installed productのInstall／起動／Update／Rollback／Repairを証明しない。P11はOPENを維持する。

#### 2026-10-06 P11 Update Center一覧の遅延読込

portable製品のLIVE_RUNTIME確認で、未選択のSettingsが`IndexedStack`により起動時点でmountされ、`initState`からUpdate一覧Broker IPCを先行発行していた。初期表示には「安全Brokerとの通信に失敗しました」が出た。後の明示的な候補取得は、製品Manifestにupdate trustが未設定のためBrokerが`update_trust_unconfigured`として正しく拒否し、その直後の一覧要求はacceptedとしてfile-backed Auditへ記録された。trust未設定のfail-closed挙動は維持しつつ、Settingsが実際に選択された時だけBroker一覧を読むよう遅延化した。画面を開くたびのpollingやretryは追加していない。

検証: `SettingsScreen.active`を追加し、製品IndexedStackから`selectedIndex == 11`を渡す。widget回帰testはinactive mount時に`更新一覧`が0回、active切替で1回だけ要求され、configured trust状態が画面へ表示されることを確認する。通常OneDrive checkoutのFlutter testは開始前の`build\unit_test_assets`削除に失敗した。tracked Desktop app／`packages/gui_shell_ui`だけをASCII Tempへ複製し、生成済み`build`・`.dart_tool`・platform `ephemeral`を除外したscratchで`flutter pub get`後、`flutter test --no-pub --dart-define=GUI_SHELL_PRODUCT_VERSION=1.2.3 test/settings_install_flow_test.dart test/settings_compose_test.dart test/update_client_test.dart`は17件PASS。scratch上の`dart analyze lib/main.dart lib/screens/settings.dart test/settings_install_flow_test.dart`は`No issues found`、3 Dart fileの`dart format`は変更なし。`python -X utf8 tooling/schema_check/check_schemas.py`は161 schema／157 example／208 negative fixture PASS、`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`は236 checks PASS。`python -X utf8 tooling/manifest.py --write/--check`および`python -X utf8 tooling/release_gate_check.py --release-track windows_v1`もPASS。`python -X utf8 tooling/日本語基底監査.py --strict`は今回変更外のrev3／rev4履歴とCodex CLI診断文字列にある既存3 findingsで終了値1、今回変更fileのfindingはない。修正後のWindows LIVE_RUNTIME再実行はしておらず、今回の新しい証拠はwidget `FIXTURE`、解析、およびSchema／Conformanceに限る。既存Authority／Broker trust／Audit境界の変更はない。P11はOPENを維持する。

## 3. 後続製品工程

P2 Multi-Agent Compare、P3 Handoff、P4 Provider / Model Center、P5 Workspace / History / Evaluation、P6 Credential / MCP、P7 A2A / Host / Adapter、P8 GUI-Shell Compose、P9 Standalone Export、P10 Module Selection / Pruning、P11 Windows Productization、P12 Product IntegrationはProduct Build受入れを閉鎖し、Windows `FEATURE COMPLETE`を維持する。最新版実装指示§2により現在は`PRODUCT_BUILD_MODE`でP13 Mobile / Non-Windowsを独立trackとして進める。Q0〜Q7 Final QA記録は履歴として保持し、現行schedulerにはしない。Owner Finalizationではproduction Publisher／signing identity、production Audit key、不可逆な事業判断、Final GOだけをOwnerへ戻す。

## 4. 関連正本

- `ROADMAP.md`: rev5工程への入口と過去作業履歴
- `docs/FINAL_QA_QUEUE.md`: Feature Complete後へ移送した検査項目
- `docs/REV4_ACCEPTANCE_LEDGER.md`: rev4受入れ履歴。rev5現行状態の正本ではない
- `docs/REV3_PROGRESS.md`: rev2/rev3の作業・失敗・実行証拠の履歴
- `release_blockers.registry.json`: release gateの証拠・分類。開発phaseの進行順と同一視しない
