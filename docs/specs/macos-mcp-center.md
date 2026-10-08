# macOS MCP接続・Tool実行・切断

状態: IMPLEMENTING（P13 Product Build）。現行正本はrev5、`docs/REV5_PRODUCT_PROGRESS.md`と本有限契約。Windows MCP・Mac資格情報保管・CLI／Workspace選択のCLOSED条件を再訪しない。

## 意味と責任

既存MCP stdio clientとBrokerの接続・catalog・一回Tool実行・切断を、Mac製品の別個native Owner確認へ接続する。新Protocolや別Authority bridgeを作らない。Flutter／Swiftは公開要求と表示・待機だけ、Rustが同一要求hashを確認し、既存Brokerが現在条件を再評価する。

接続は実行file・Workspace・引数件数／hashを示す。ToolはServer／Tool ID・名前・引数件数／hashを示し、本文は事前の製品UIだけで確認する。切断はServer IDへ束縛する。既定拒否、取消／期限は不承認、自動再送なし。通常IPC・Owner資格単独・metadata・過去確認から作用を生成しない。

既存JSON Schemaとwire仕様を変更せず、`mcp-connection-center.md`／`mcp-contract.md`のサイズ・Schema・秘密／Authority field拒否・一回Permission・Audit・hash-only結果を保持する。資格情報注入はこの単位では引き続き拒否する。Provider／MCP注入、本文のAgent引渡しは別の未成立製品範囲へ残す。

## processとOS境界

Rustの`Command::process_group(0)`で新childを独立process groupへ置く。group IDは実際のChild PIDからだけ生成し、1以下・親group・不一致を拒否する。既存固定rustixの安全APIだけを使い、Coreへunsafeを追加しない。終了は所有groupへSIGKILL、root回収、OSのgroup不存在確認を最大5秒で行う。失敗時は接続記録を保持し、停止成功やAudit成功へ昇格しない。完了したgroupへ再度signalを送らない。

この停止範囲は同じgroupのprocessだけ。Windows Job Objectの全子孫監督・Broker crash時の自動停止・別groupへ離脱した第三者daemonまでの保証をMacへ転用しない。App Sandboxは親から継承し、権限・entitlementを拡張しない。追加crash／group離脱保証はFinal QA／既存platform関門へ残し、現在の正常接続・Tool・明示切断の施工を止めない。

## 有限受入れ

| ID | 条件 | 状態 | 証拠 |
| --- | --- | --- | --- |
| MAC-MCP-1 | native Owner拒否→未接続、承認→実stdio discovery／metadata一覧 | OPEN | 未取得 |
| MAC-MCP-2 | 現Tool／Schema再照合、一回Permission・Audit、hash-only実呼出し表示 | OPEN | 未取得 |
| MAC-MCP-3 | 別個Owner切断、所有group停止・記録解消、通常終了 | OPEN | 未取得 |
| MAC-MCP-4 | normal／Owner資格単独、session・Authority／秘密注入、configured Credentialの拒否 | CLOSED | source 94ea619、run 37749059105の対象3件／共有Windows18件PASS |
| MAC-MCP-5 | 対象test、通常Mac build、Windows直接依存回帰 | CLOSED | 同run対象解析・Flutter3件・通常build PASS、Windows18件PASS |

合成Rust stdio Serverとad-hoc test identityを手動Actions `macos_mcp_center`で使用する。秘密入力や資格情報の再登録を行わず、CLOSED試験を選択しない。正常製品経路一回と対象境界で閉じ、長時間・fault matrix・Formal Evidenceを追加しない。source／build／対象試験／happy path／Authority結果だけを記録する。

通常Release `task_execution=unsupported`、`release_ready=false`を保持する。正式署名・配布・production鍵・Final GOは別関門であり、開発を止めない。

## 検証履歴（閉鎖前）

2026-10-08のWindows局所検査: Schema 166／正常例162／負例213、Conformance 240、Manifest 1227、手動workflow限定検査、release gate、diff checkはPASS。`cargo check --locked --manifest-path native/rust_helper/Cargo.toml --lib`、新Owner境界2件、共有MCP直接依存18件はPASS。必須全Rust `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --no-fail-fast -- --test-threads=1`はlibrary targetがFAIL（exit 1）であり、全体PASSとはしない。既存Final QAの全Rust失敗履歴を上書きせず、今回のMac有限Acceptanceとは分離する。

必須両appの`flutter analyze --no-pub`は通常checkoutの既知LSP FormatException／server exit 255でFAIL。sourceだけを複製した既存ASCII一時checkoutでも対象`dart analyze`がperf witness file削除不能／server shutdownでFAILした。回避をproductionへ追加せず、Macの対象解析へ送る。ASCII上の新Mac接続／投影2件、待機1件はPASS。投影test初回はdebug platform変数の復元時点が遅くFAILし、test body終了前へ復元を移して対象だけ修正した。静的日本語監査の新fixture診断1指摘は修正済み、残り既存5 file／17指摘はFAILのまま。全体rustfmt checkの既存広範差分は一括整形せず保持する。Mac実process group・native UIの有限条件は専用手動Actionsまで未成立。

source `94ea61991060a2d47760e44c259225d2077222b9`、手動Actions [run 37749059105](https://github.com/gatchimuchio/D4-POCKET/actions/runs/37749059105)はgroup実停止1件、Mac Owner対象3件、対象Dart解析、Flutter3件、通常Mac buildがPASS。MAC-MCP-4／5はCLOSED。製品UIは検索候補`MCP接続`のsubstringが複数に一致して21.095秒でFAILし、接続前に終了した。候補全文一致へ試験だけ局所修正する。artifact `11537058410`（76906 bytes）、SHA-256 `4ca0d1d36e2d14a47063e9dfc72c61149c34afc3721f636d66223692a5dafc7c`を実byteへ照合済み。以後の`mcp_product_only=true`はMAC-MCP-1〜3だけを検証し、4／5の試験を再訪しない。新runner上のcompileは製品UI試験の前提準備であり、成立済み条件の代替証拠を増やす目的ではない。

source `357432b06e7fb712aab6a2a9e4b18f38f68ba656`、[run 37750370848](https://github.com/gatchimuchio/D4-POCKET/actions/runs/37750370848)は検索選択が成立し、native Owner拒否・承認、実stdio discoveryとBroker接続accepted Auditまで到達したが、製品UIの接続receipt表示で65.363秒FAIL。artifact `11538585624`（68915 bytes）、SHA-256 `d7aaf2039409648f17e745fa19df8de28afb660f1d48bed0ba6d433621e43600`を実byteへ照合。

`REGRESSION_REOPENED`の局所対象は共有MCP接続receiptのSchema適合だけ。原因commit `5d45baa2e2d7a156b1439a6ad0834cd0817c492b`が外部発見metadataの`LIVE_RUNTIME`を接続receipt内へ残し、外側の`INTERNAL_STATE`、現行Schemaのconst、Flutter受入れと不整合だった。破壊条件はMAC-MCP-1の製品metadata表示／`mcp_connection_receipt.schema.json`。既存Windows実stdio testへ当該assertを追加して0 passed／1 failed（left LIVE_RUNTIME、right INTERNAL_STATE）を再現した。Brokerで保存済みmetadataへ正しく分類し、UI／Schema検査を緩めない。MAC-MCP-4／5や他CLOSED工程全体を再開しない。検査追加の理由は観測済み契約破壊であり、証拠強化ではない。

同局所修正後の必須Windows全Rustは560 passed／0 failed／13 ignored（exit 0）。接続receiptの実stdio回帰assertを含む。Schema 166／正常例162／負例213、Conformance 240もPASS。先行全RustのFAILや既存Final QAの間欠的失敗を消さず、このPASSをその根因修正の証拠へ流用しない。MacのMAC-MCP-1〜3は製品UI確認まで未成立。
