# macOS Workspace Inspectorの内容露出入口

状態: VALIDATING（P13 Product Build、2026-10-09）。現行rev5と`docs/REV5_PRODUCT_PROGRESS.md`に従う。

## 意味と責任

Agent Centerで起動中に登録したWorkspaceを、既存Inspectorから明示した内容露出範囲で読む。既存`作業領域承認`／`作業領域失効`／`作業領域全体基準点保存`だけをMac Rust native Owner確認へ結合する。現在要求のID・nonce・時刻・client・payload hash、Workspace ID・登録hash・表示範囲を確認し、承認した同一要求だけを既存Broker process内receiverへ渡す。入力によるOwner bool、資格、任意operation、任意root／pathを追加しない。

承認は現在登録に限定した5分の`workspace.inspect`読取Permission／Approvalであり、Task実行・書込・Credential・Agent trustを生成しない。既存Brokerが登録hash・現在登録・期限・root実体・secret除外・Auditを再評価する。`full`以外で全文を返さず、秘密除外は登録path境界であって一般的な秘密検出ではない。baselineは現在のfull承認下で上限付き既存取得器が読む。Workspaceあたり一つのBroker内baselineであり、Task固有成果やRecovery適用ではない。失効は読取grantとbaselineを破棄するだけでfileを書き換えない。

既存`workspace_inspection_request.schema.json`／`workspace_inspection_response.schema.json`、`workspace_diff.schema.json`、Rust `WorkspaceRegistry`／`WorkspaceReader`、Flutter `WorkspaceClient`／`WorkspaceInspector`を消費する。新しいwire／保存形式、filesystem bridge、OS scope付与は作らない。通常読取は既存認証Brokerだけへ送る。登録／OS選択はCLOSEDのまま、今回の実行前提としてだけ使う。

作用分類: control（native承認・失効・baseline保持）／runtime（既存bounded読取・差分投影）。Capability=`workspace.inspect`、Permission=`workspace.inspect.<approval_id>`、Approval=現在登録hash・内容露出範囲への独立native確認、Audit=既存workspace受信／recorded／accepted／rejected、Recovery=`workspace.reapprove`。OS確認300秒＋既存応答4秒、Swift／Flutter待機305秒。非承認・不正・期限・監査失敗・未確定を成功へ昇格せず、自動再送しない。App Sandbox・登録handle・Windows経路・通常Release能力を維持する。

## 有限受入れ

| ID | 条件 | 状態 | 証拠 |
| --- | --- | --- | --- |
| MAC-INSPECT-1 | 3操作の同一要求native入口、session／authority／hash／不正表示範囲否定、拒否時未配送 | CLOSED | Windows候補1件・入口1件、Mac入口1件はsource `e031745`／run `37875821384`でPASS。再実行しない |
| MAC-INSPECT-2 | Mac製品UIでfull読取確認→公開file表示→全体baseline確認→合成fixture変更→changed files／diff→失効確認・本文消去・Audit | FAIL | source `19036e6`でもXCTestのtmp書込をOS拒否。新規通知を除去し当該runのbaseline受理Audit後にhostが固定合成fileを更新する |
| MAC-INSPECT-3 | 対象build／解析、通常終了、helper／専用fixture回収 | OPEN | 手動Actions `macos_workspace_inspector`だけ |

公開合成fileとtest identityを使う。合成入力は`FIXTURE`、責任経路の静的確認は`CONFIG`、実native確認・Broker APFS読取・差分・正常終了は限定`LIVE_RUNTIME`。登録済み秘密canaryは本文・log・artifactへ出さない。本文は明示full後の公開fileだけを試験する。既存CLOSED試験、Alias／race／crash matrix、長時間、Formal Evidenceを再実行・追加せず、Acceptance外は延期中Final QA／既存gateへ送る。通常Release `task_execution=unsupported`、`release_ready=false`を保持し、PASS後は本単位をCLOSEDとして次へ進む。

## 検証履歴

2026-10-09: Windowsの新native候補試験`cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib macos_workspace内容露出_ -- --test-threads=1`が1 PASS、新platform限定Broker試験も1 PASS。3操作の同一payload配送、未承認の未配送、session／authority／hash変更・任意root・不正表示範囲の否定、Broker拒否の保持を確認した。試験名のRust snake-case警告を局所修正し、挙動を変えない。Mac側の未登録scope再評価は専用runnerで取得する。

必須`cargo test --locked --manifest-path native/rust_helper/Cargo.toml --no-fail-fast -- --test-threads=1 --format terse`はlib 518 PASS／1 FAIL／12 ignored、他target PASS。変更外`a2a::tests::loopback_HTTPからAgent_Cardを取得してmetadata_onlyへ射影する`が`a2a_connection_failed`／「A2A Agent Card応答を読めない」でFAIL。根因未確定として既存`FQ-TEST-LOOPBACK`へ保存し、全体FAILをPASSへ変更せず、同じ旧fixtureを追加再試験しない。

Desktop／Mobileの必須`flutter analyze --no-pub`は既知の日本語path LSP `Unterminated string`／analysis server exit 255でFAIL。回避source／一時複製は追加せず、Mac対象解析を別環境証拠として取得する。Schema 166／正常162／否定213、Conformance 244、手動起動限定、Manifest 1234、release gate、差分改行検査はPASS。日本語strictは既存5 file／17 findingsでFAIL、新規0。release-ready・正式配布の検査を開始しない。

編集前backupの最初の短いref指定`git push -f origin codex/backup-main-prev:refs/tags/codex/backup-main-prev codex/backup-main:refs/tags/codex/backup-main`はlocal branch／tagの同名refで曖昧なためFAIL。既存Git機構でsourceを`refs/heads/...`へ明示し、2世代remote tagを照合してから編集を開始した。rollback=`cfacc3fec8793b2a249fa5a4cc9b60437dcb5dec`、前世代=`7220847bc775bec23f8088fc9d421209eb1ddc93`。追加backup世代・別環境wrapper・安全設定変更はない。

初回Mac run [`37874900581`](https://github.com/gatchimuchio/D4-POCKET/actions/runs/37874900581)、source `02886de7b3966c6c599fd59ab8a7dd6275b03c01`は新しいBroker入口試験でFAIL。メモリ専用`test_broker()`が`broker_persistence_unavailable`で先に拒否され、意図した未登録Workspace拒否へ届かなかった。製品不具合の観測ではなく試験fixtureの誤りとして、確認済み入口だけを既存`persistent_test_broker`へ局所修正する。通常要求のnative確認必須と未登録scope拒否の期待を維持する。build／製品UIは未実行。artifact `11592046788`のSHA256=`7b00fffcc2f75b0805727726bd17104c7e9018d42660ad26a39f18081e3e987c`、専用fixture cleanup成功、helper残留なし。対象FAILだけを再検収する。

fixture修正後のWindows対象command `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib macos_workspace_inspector_broker入口 -- --test-threads=1`は1 PASS。Schema 166／162／213、Conformance 244、手動起動限定、Manifest 1234、差分検査もPASS。製品実装・Acceptance・既存全体FAILは変更しない。

Mac run [`37875821384`](https://github.com/gatchimuchio/D4-POCKET/actions/runs/37875821384)、source `e0317456e8a0b29b85b0885dcdb9c5f884a493cd`で新入口1件・Rust通常build・対象Dart解析・Mac製品buildはPASS、MAC-INSPECT-1をCLOSEDとする。UIは登録前提の`native Owner確認へ進む`buttonが画面外でnot hittableとなりFAIL（27.138秒）。既存`reveal`で可視位置へscrollしてから通常clickする対象試験だけを修正する。次runは`inspector_product_only=true`としてCLOSED入口を再実行しない。artifact `11591978581`のSHA256=`a5968e064f68bb2b69cd7abba563ffb429ddafffb9131b3616b178bd8dbe218a`をbyte照合済み。helper残留0・専用fixture回収成功、UI正常終了／内容露出の完了は未成立。

Mac run [`37876554147`](https://github.com/gatchimuchio/D4-POCKET/actions/runs/37876554147)、source `4f7802b3cfeb704ab78114c1b09f22e82e69293c`はCLOSED入口をskipし、通常build／解析PASS。UIは同じ登録前提buttonでFAIL（28.801秒）。`reveal`後にAX frameが窓内でも`not hittable`となったため、先行runの「画面外」は原因として未確定である。既存登録試験の通常coordinate clickへ、この新試験の操作だけを揃える。App／native承認／Broker実装は変更しない。artifact `11593076178`のSHA256=`1d5b2ab1b8b5ac629eb8523d5c5aea9b3a4541c2f7917f05493fe97c56f3e7e0`をbyte照合済み。helper残留0・fixture回収成功。

Mac run [`37877200890`](https://github.com/gatchimuchio/D4-POCKET/actions/runs/37877200890)、source `00af2c418b6006d05d448fc35de97db8eb80d09b`は登録前提を通過し、読取承認Auditがreceived／recorded／accepted。UIは長い承認成功文言のOCR照合でFAIL（95.825秒）。改行に依存しない一意の短い成功部分へ照合を変更し、承認表示停止の診断を固定status codeだけへ限定する。まだ未観測の全文表示／baseline／diff／失効をPASSへ補完しない。artifact `11592956964`のSHA256=`d2db7029ce92c25bf723c44df95132b293e427c2409f0eddd33ac47104862950`をbyte照合済み。CLOSED入口skip、通常build／解析PASS、helper残留0・fixture回収成功。

Mac run [`37878209622`](https://github.com/gatchimuchio/D4-POCKET/actions/runs/37878209622)、source `5d927f92f6039a87f97a36f83ffe8114494bbef1`は短い成功文言でもFAIL（78.947秒）。固定診断`registration_refreshed`を観測し、読取承認Auditはaccepted。文字列の長さが根因という推定を訂正する。Agent CenterとInspectorの既存lifecycleはnative確認中に表示を破棄し、復帰時に現在登録を再取得するため、一時的な成功文言の固定期待が不適切だった。新試験を現在登録の選択→公開file読取、baseline保存後の既存比較範囲読戻し、失効後の現在一覧・承認操作不在と本文消去へ局所修正する。これらは既存UI／既存Brokerだけを使い、画面復帰境界や権限を変更しない。診断の追加codeは役目を終えたため除去する。artifact `11593696229`のSHA256=`1d828ddad41b17d84f51c9afc4ef247a7b88b0d47805c5cfa42a6f2d8328414d`をbyte照合済み。CLOSED入口skip、通常build／解析PASS、helper残留0・fixture回収成功。

Mac run [`37879704542`](https://github.com/gatchimuchio/D4-POCKET/actions/runs/37879704542)、source `cc7d36255213a1f4eb94ec98cccf13e74b977aad`は公開fileの本文表示と読取Audit acceptedまで成立した。baseline buttonのAX click後にnative確認が現れずFAIL（124.470秒）。baseline操作の受信Auditもなく、製品側の拒否やクリック不達の根因はまだ確定しない。新試験の当該操作だけを、現在の可視button文字・enabled状態を確認した通常mouse入力へ変更する。既存lifecycle・Broker・Authority・有限条件は変えない。artifact `11593294412`のSHA256=`7d295a1846d752159844d6850c4d9024b87d1f246033a7576772e1ee3ae2eb79`をbyte照合済み。CLOSED入口skip、通常build／解析PASS、helper残留0・fixture回収成功。diff／失効／正常終了は未成立のまま。

Mac run [`37880815733`](https://github.com/gatchimuchio/D4-POCKET/actions/runs/37880815733)、source `364798fa4638463d3bb46ccdef7dba9450fbd7ce`で可視buttonからbaseline確認が開始し、全体baselineの受理Auditと既存比較範囲の製品表示まで成立した。XCTestによる別app container内`public.txt`書込がOSのOperation not permittedでFAIL（107.860秒）。製品に書込権やsandbox例外を与えず、試験hostが所有する固定合成public fileだけを更新する限定handshakeへ修正する。試験用/tmpディレクトリを専用作成し、既定文字の一回要求、既定BEFOREの通常file確認、AFTERへの更新、10秒の応答待ち、worker／専用signal回収を行う。任意path・command・秘密・Broker要求を受けず、製品bundleやruntime依存に追加しない。本来の差分・失効条件は維持し、fixture更新の完了だけでPASSへ昇格しない。artifact `11595100431`のSHA256=`043df9a010d4c22627efa7480b85635c935d601bc948c6a78fa4edb857cc1565`をbyte照合済み。CLOSED入口skip、通常build／解析PASS、helper 0・既存fixture回収成功。このhost制御はMac検収用fixtureに限定して保持し、XCTestが正規OS権限で更新できる既存fixture経路へ置換する場合だけ除去する。

Mac run [`37882650318`](https://github.com/gatchimuchio/D4-POCKET/actions/runs/37882650318)、source `19036e64f7603266ce079893beed0a110427ab6c`はtmp要求file作成もOSが拒否してFAIL（124.721秒）。新規handshake・tmp directoryを撤去し、hostで読取り成立済みの既存Auditの当該runの全体baseline受理を固定合成file更新の順序信号として使う。開始時はそのAudit不存在、取得時は通常file・4 MiB未満を確認し、合成public fileのBEFORE一致・非symlink・固定rootだけを更新する。Auditを製品権限へ転用せず、XCTestの書込権・通知・stdout印・新しいbridgeを追加しない。compileを含む試験stepは220秒だったため、worker待機は上限10分へ束縛し、成功終了値を検査する。失敗時は所有workerだけを停止する。製品changed files／diffと失効条件は同一のまま残す。artifact `11594453989`（73803 bytes）のSHA256=`d3b5f195075b6edbb7c49acbeea1a5ad2320aef06296ade0a6c98ada829e8690`をbyte照合済み。通常build／解析・helper 0・既存fixture回収PASS、結果検査は未到達の差分条件によりFAILを保持する。
