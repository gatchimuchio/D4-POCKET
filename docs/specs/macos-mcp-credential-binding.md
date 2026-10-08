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
| MAC-MCP-CRED-1 | 製品metadata選択・公開参照要求・別個native確認の委譲説明 | OPEN | 未取得 |
| MAC-MCP-CRED-2 | 現record／Keychain照合、対象stdio実受渡し、使用Audit・公開最終使用時刻 | OPEN | 未取得 |
| MAC-MCP-CRED-3 | 未承認・対象／用途／失効／不安全な環境変数・秘密注入を拒否、秘密非公開 | OPEN | 未取得 |
| MAC-MCP-CRED-4 | 対象build／直接依存試験、正常切断・終了・所有process／試験資産回収 | OPEN | 未取得 |

専用手動Actions `macos_mcp_credential_binding`で実Keychainと合成stdio Serverを接続する。既存native Debug fixtureが秘密を生成し、XCTestは秘密を入力・読取・コピーしない。Serverは受信値を内部照合し固定の非秘密結果だけを返す。秘密入力を含むxcresult attachment／動画／画面／全階層dumpをexportしない。登録・接続・Tool・切断は結合の必要な前提／後片付けで、CLOSED条件の強化証拠にしない。正常一回と新consumerの直接境界だけで閉じる。

未対応拒否はconsumer導入前の範囲。本追加契約がnative確認・現record・対象へ限定する。通常資格／参照単独・metadataからの作用拒否は維持する。通常Release `task_execution=unsupported`、`release_ready=false`を保持する。追加fault matrix・正式更新時のKeychain ACL継続・Formal Evidenceは後続／Final QAへ残す。

## 検証履歴

Windowsの`cargo check --locked --manifest-path native/rust_helper/Cargo.toml --lib`と直接依存の資格情報7件はPASS。必須全Rust `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --no-fail-fast -- --test-threads=1`はexit 0でPASS。対象Flutter2件は追跡sourceとSHA-256を照合した既存ASCII試験複製でPASS。初回複製commandは相対source pathを誤って古いsourceを試験したため、そのPASSは本変更の証拠として不採用。絶対pathと全対象byte一致を確認して対象試験を実行した。

初回Conformanceは資格情報Vault内の旧`Purpose::Credential`token位置が移りFAIL。検査を削除せず、実際のplatform保存先内の同tokenとVaultからの保存先／読取接続の双方へ検査責任を同期する。必須Desktop／Mobile `flutter analyze --no-pub`は日本語checkoutの既知LSP FormatException／analysis server exit 255で各exit 1。ignored logは`release_evidence/p13-macos-mcp-credential-*-analyze.txt`。環境FAILをPASSへ読み替えず、対象Mac解析へ送る。製品への回避を追加しない。Mac実受渡しと製品結合は専用手動runまでOPEN。

同期後Conformance 241件はPASS。日本語基底監査strictは変更前と同じ5 files／17 findingsでexit 1を保持し、この変更で新規指摘はない。広域の既存指摘修正やFinal QAは本consumerの工程へ持ち込まない。
