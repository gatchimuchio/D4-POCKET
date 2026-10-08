# macOS MCP資格情報結合

状態: IMPLEMENTING（P13 Product Build、2026-10-08）。正本は最新版rev5、`docs/REV5_PRODUCT_PROGRESS.md`と本有限契約。資格情報登録・保管・失効と資格情報なしMCPのCLOSED受入れを再構築しない。

## 対象と責任

操作者は既存metadata-only一覧から対象Server向けの有効な資格情報IDと環境変数名を選ぶ。FlutterはID・用途・対象・環境変数名だけを既存`mcp_connection.schema.json`版1の`Credential ref`へ送る。秘密値・承認bool・保管先・Owner資格を送らず、wire／保存形式／公開Contract名を変更しない。

Rust helperの別個native Owner確認は同一要求hash、実行file、Workspace、資格情報ID・対象・環境変数名を示す。Serverとその子孫が秘密値を読み取り外部送信できること、版交渉の限定再起動でも同じ相手へ渡る可能性、App Sandboxは委譲先を信頼済みにしないことを明示する。ID・metadataはPermissionやApprovalではない。

承認された同一要求だけを既存Brokerへ渡す。Brokerは永続Audit、現在登録record、用途`mcp_transport`、Server ID、失効状態、実暗号文hashを照合し、既存platform資格情報保存先からRust内の短命zeroizing bufferへ読み出す。Windows DPAPIの挙動・形式を維持し、Macは既存Keychainへ接続する。`env_clear`と既存allowlistの後、指定された安全な環境変数一つを所有stdio Serverへ渡す。D4親processや別Serverへ設定しない。Server自身の子孫への継承・外部送信は確認で開示する委譲範囲であり、第三者内部完全性を追加保証しない。

接続後に既存MCP資格情報使用Auditを確定し、公開最終使用時刻へ接続する。Audit失敗はprocess停止・未成立、読取／失効／対象不一致／未登録／改変は起動前拒否。秘密値をFlutter、Swift、応答、Audit、error、log、trace、test artifactへ出さない。自動fallback・自動再送・秘密読出しAPI・保管先探索は作らない。

作用分類は既存control／runtime経路。Capability=`mcp.connection.connect`、Permission=現在対象Serverと保管recordへの限定使用、Approval=同一要求hashの別個native Owner確認、Audit=ID・対象・環境変数名・使用時刻だけ、Recovery=保管／接続状態を確認して新規要求。Provider／Agent／A2A注入・物理削除・正式配布は別単位／既存release gate。

## 有限受入れ

| ID | 条件 | 状態 | 証拠 |
| --- | --- | --- | --- |
| MAC-MCP-CRED-1 | 製品metadata選択・公開参照要求・別個native確認の委譲説明 | CLOSED | source a51d005／run 37770724707の実選択・別個native承認・接続receipt、共有Owner summary直接試験 |
| MAC-MCP-CRED-2 | 現record／Keychain照合、対象stdio実受渡し、使用Audit・公開最終使用時刻 | CLOSED | source 5e850ae／run 37772178891の実接続、正の使用時刻表示、使用accepted Audit 1件・実Tool receipt、先行実Keychain直接試験 |
| MAC-MCP-CRED-3 | 未承認・対象／用途／失効／不安全な環境変数・秘密注入を拒否、秘密非公開 | CLOSED | Windows資格情報直接7件／必須全Rust、run 37767646092のMac現在対象・失効・不安全環境・Audit非漏洩直接試験1件、Conformance 241、既存native Owner gate・公開注入否定のCLOSED証拠 |
| MAC-MCP-CRED-4 | 対象build／直接依存試験、正常切断・終了・所有process／試験資産回収 | OPEN | 未取得 |

専用手動Actions `macos_mcp_credential_binding`で実Keychainと合成stdio Serverを接続する。既存native Debug fixtureが秘密を生成し、XCTestは秘密を入力・読取・コピーしない。Serverは受信値を内部照合し固定の非秘密結果だけを返す。秘密入力を含むxcresult attachment／動画／画面／全階層dumpをexportしない。登録・接続・Tool・切断は結合の必要な前提／後片付けで、CLOSED条件の強化証拠にしない。正常一回と新consumerの直接境界だけで閉じる。

未対応拒否はconsumer導入前の範囲。本追加契約がnative確認・現record・対象へ限定する。通常資格／参照単独・metadataからの作用拒否は維持する。通常Release `task_execution=unsupported`、`release_ready=false`を保持する。追加fault matrix・正式更新時のKeychain ACL継続・Formal Evidenceは後続／Final QAへ残す。

## 検証履歴

Windowsの`cargo check --locked --manifest-path native/rust_helper/Cargo.toml --lib`と直接依存の資格情報7件はPASS。必須全Rust `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --no-fail-fast -- --test-threads=1`はexit 0でPASS。対象Flutter2件は追跡sourceとSHA-256を照合した既存ASCII試験複製でPASS。初回複製commandは相対source pathを誤って古いsourceを試験したため、そのPASSは本変更の証拠として不採用。絶対pathと全対象byte一致を確認して対象試験を実行した。

初回Conformanceは資格情報Vault内の旧`Purpose::Credential`token位置が移りFAIL。検査を削除せず、実際のplatform保存先内の同tokenとVaultからの保存先／読取接続の双方へ検査責任を同期する。必須Desktop／Mobile `flutter analyze --no-pub`は日本語checkoutの既知LSP FormatException／analysis server exit 255で各exit 1。ignored logは`release_evidence/p13-macos-mcp-credential-*-analyze.txt`。環境FAILをPASSへ読み替えず、対象Mac解析へ送る。製品への回避を追加しない。Mac実受渡しと製品結合は専用手動runまでOPEN。

同期後Conformance 241件はPASS。日本語基底監査strictは変更前と同じ5 files／17 findingsでexit 1を保持し、この変更で新規指摘はない。広域の既存指摘修正やFinal QAは本consumerの工程へ持ち込まない。

source `62f0583f19a57da076bff3c511b493dfd00a17cf`、手動[run 37767646092](https://github.com/gatchimuchio/D4-POCKET/actions/runs/37767646092)は実Keychain直接試験1件・対象Flutter2件・対象解析・通常Mac buildでPASSした。製品UIは登録buttonのAX click後にnative入力画面を確認できず67.536秒でFAIL。秘密attachmentはexportしていない。artifact `11546961580`、SHA-256 `7848265c13d375084fb7db1495514b11b19422b3df511b634da52b4a391365db`を実byteへ照合。入力後scrollを伴うbutton操作を既存の公開可視文字clickへ統一し、失敗時は固定公開状態の有無だけを記録する。原因確定前の操作修正であり製品成立はまだ主張しない。Audit不足で同stepの後片付けまで到達しなかったため、回収を独立always stepへ分離する。既存PASS済みのKeychain／対象Widgetは同sourceの証拠を再利用し、再runはUI残件だけを処理する。

source `d8a1efc770f947b213af01b5f482a9609fbad697`、[run 37769130630](https://github.com/gatchimuchio/D4-POCKET/actions/runs/37769130630)はnative入力・別個Owner承認・Keychain登録・metadata一覧までPASS。続く公開選択肢のStaticText queryが未成立で88.082秒でFAILした。artifact `11547013659`、SHA-256 `1164e2bdc0fbb59c708c45deae354e302ad5d022b492318985c7af8d6c60dc5b`を実byteへ照合。独立always回収はPASS。登録後・秘密欄不在の公開選択行（種類と32桁ID）だけを既存と同じ可視OCR方式で操作する。秘密画像・階層dumpは保存しない。

`REGRESSION_REOPENED`（共有UIの直接依存のみ）: 原因commit `62f0583`の接続後使用時刻再取得がWindowsにも追加`資格情報一覧`を送り、既存`Desktop panelは対象Credential metadataと環境変数名だけを接続要求へ送る`が期待2要求／実際3要求でFAILした。破壊された条件はWindowsの既存公開要求順序。期待値を弱めずMac consumerだけに再取得を限定し、当該Windows試験と直接依存のMac選択Widgetの2件はPASS。Windows機能全体・CLOSED済みTool等は再開しない。

同じ現sourceとSHA-256を照合したASCII試験複製の対象`dart analyze`も、Analysis Serverの終了処理で`AppData/Local/Dart/perf/12312`を削除できないOS error 1920によりexit 1となった。製品解析PASSではなくhost環境FAILとして保持し、変更した共有Dartの対象解析だけを次のMac runで実施する。SDK／OSへの回避や追加product blockerは作らない。

source `a51d0052ea7d111fd902495fd3880c9fa47a0303`、[run 37770724707](https://github.com/gatchimuchio/D4-POCKET/actions/runs/37770724707)で公開IDの実選択・環境変数指定・別個native承認・資格情報付き実stdio接続receiptまで成立し、MAC-MCP-CRED-1をCLOSEDとする。続く最終使用時刻のStaticText queryが未成立で100.880秒でFAIL。対象Dart解析・通常Mac build・独立always回収はPASS。artifact `11547912264`、SHA-256 `f1018a59c864b1256e5077d4f21680808b989dba59ba698c64cd1e2a0eeaf4a4`を照合した。登録済み公開metadata内の正のUnixMillis表示だけを同じ可視文字方式で確認する。次の正常一回は残る使用時刻／Tool／終了の結合に必要な前提として通し、CLOSED確認の追加証拠にしない。資格情報使用Audit投影をTool完了判定より先に出し、後続UI失敗でも既成立の使用記録を隠さない。

source `5e850aeb62e8338f9a907025e7bf706e1ba5413a`、[run 37772178891](https://github.com/gatchimuchio/D4-POCKET/actions/runs/37772178891)で正の公開使用時刻、対象stdio実受渡し、実Toolのhash-only receiptが成立。使用accepted Auditは1件、Toolのapproved／consumed started／hash-only acceptedも各1件でAudit判定PASS。MAC-MCP-CRED-2／3をCLOSEDとし、既存直接試験・境界証拠を再利用する。終了前の切断可視文字探索だけが上方向8回で未成立となり156.708秒でFAIL。切断はconnection card headerにあり、追加Credential metadataを持つ今回の試験だけ下方向へ探索する。通常build／Audit／独立always回収はPASS。artifact `11548688377`、SHA-256 `69144d90059acc6ea3152d43dc2564f0984930faf497204d5064b48a124507f0`を照合。残るMAC-MCP-CRED-4の通常切断・quitだけを検収し、CLOSED条件は正常利用の前提として通す。

source `b1b01f346f57fc1fe5406381b7a62b9e748b7706`、[run 37773734593](https://github.com/gatchimuchio/D4-POCKET/actions/runs/37773734593)も切断文字探索だけ181.197秒でFAIL。下方向変更だけでは解消せず、方向だけが原因という仮説は不成立。Audit／通常build／回収はPASS。artifact `11548947608`、SHA-256 `7ec4d16c80934697da63f3b61ea583c89467f3322bab358f84c40f2d3d47e5a4`を照合。今回の切断に限り短いicon併記を許すlocatorと、失敗時だけの公開製品窓cropを追加する。cropは秘密native入力終了・使用時刻・hash-only結果の確認後、sheet／秘密欄不在を再検査して固定fixture内へ1 MiB未満で新規作成する。always回収はその固定PNGだけを限定保存・削除し、xcresult attachment／録画／全画面／階層dumpをexportしない。これは観測済みlocator FAILの診断専用で、正常PASS後の追加証拠にしない。

source `f468f8ab2cd97f3aaad11f22dd2b1a6fe27e1bcd`、[run 37775851114](https://github.com/gatchimuchio/D4-POCKET/actions/runs/37775851114)は短いicon許容でも切断探索が151.392秒でFAIL。artifact `11549384455`、SHA-256 `118178e1938eb8d921a5de5fb0e4dd73cead3d9d6c10e65db26759a706d17488`を照合。Audit／build／回収はPASSしたが公開PNGは存在せず、診断保存も未成立。保存失敗を黙殺せず非秘密の分類／codeだけを記録し、UITestから別app containerへ書く位置をsource固定のdev evidence directoryへ変更する。既知の切断buttonの存在・frameと既知dialog buttonの存在だけを診断し、入力値・全階層は取得しない。秘密入力・録画の非exportと1 MiB上限は維持する。

source `feb674ada1816aafd6baa6b97d9008842fff6a85`、[run 37778022707](https://github.com/gatchimuchio/D4-POCKET/actions/runs/37778022707)は131.092秒でFAIL。切断buttonは存在するがAX frameは`(759,179,93,1)`、既知の残存dialog buttonは全て不存在。公開画像保存はfilesystem code 513（権限）で未成立。artifact `11551397416`、SHA-256 `4f33dd545072dfd0d836f16a64afb329c6f88f9ed6b9baab3e9918298d05b4df`を照合。ボタンの不存在・残存dialogという仮説ではなく、画面外位置として扱う。UITest自身の一時領域へ固定公開PNGを保存し、always回収側は出力されたpathの固定basename・正本化した`/private/var/folders/`親・非symlink・1 MiB上限を検査してその一fileだけをコピー・削除する。sandbox／TCC設定は変更しない。既知切断buttonを含むscroll viewのframeだけを追加観測し、全階層・入力値は取得しない。

source `834e37f73519b3fd45fcf7804219494a8d25bd1d`、[run 37780315182](https://github.com/gatchimuchio/D4-POCKET/actions/runs/37780315182)は167.022秒で切断locator FAIL。既知button frameは同じ、AX ScrollView祖先は0件。公開PNGはUITest自身の`/Users/runner/Library/Containers/com.example.guiShellDesktop.RunnerUITests.xctrunner/Data/tmp`へ正常に保存されたが、回収側の想定親pathが異なり回収FAILとなった。所有process残存は0件、Audit／通常buildはPASS。artifact `11553000720`、SHA-256 `61e0a578be3e6039a494fb47ddeca001236edf06579553165d6ca55f468c3e43`を照合。実観測したUITest専用tmpの完全一致に回収先検査を限定し、他containerや任意親pathを許可しない。

source `1a422f64e662e71f32db5a08d1dd41a87f0a6e81`、[run 37781873419](https://github.com/gatchimuchio/D4-POCKET/actions/runs/37781873419)は151.177秒で切断locator FAIL、通常build／Audit／所有processとfixture回収はPASS。artifact `11552883187`、SHA-256 `988f0cfb3008197422f0728b8d4a52943d8bf1c91bf1cd33469ce94a30cb6b0d`を照合し、限定公開PNGを実視認した。画面はMCP面ではなく後続の設定・Windows書出し面であり、下方向探索の通り越しが観測された。既存の全体検索でMCP先頭へ戻し、既知切断buttonの実可視frame（高さ24px以上・製品window内）を上限10回の通常scrollで確認してclickする。既存資格情報なし試験・製品コード・Broker／native承認は変更しない。
