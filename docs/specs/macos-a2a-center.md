# macOS A2A接続センター

状態: IMPLEMENTING（P13 Product Build）。正本は最新版rev5と`docs/REV5_PRODUCT_PROGRESS.md`。既存A2A契約・Windows製品受入れ・Mac通常接続はCLOSEDのまま再訪しない。

## 意味と責任

外部Agentの宣言情報を確認するため、操作者が製品A2A接続センターへ公開Agent IDとloopback接続先を入力する。接続はAgent Card取得とmetadata-only一覧への登録だけである。接続・宣言Capability・Agent CardはTrust、Task Permission、Approval、Credentialを生成しない。

既存`A2A接続`要求の版1、HTTP／IPv4 loopback、protocol version 1.0、未設定のCredential参照だけを使う。Flutterは要求JSONだけを既存Runner channelへ渡す。Rust helperは現在時刻・nonce・要求ID・発信metadata・Schema・同一payload hashを照合し、Rust所有の期限付きnative Owner確認にAgent ID、完全接続先、hashと非権限境界を表示する。既存Brokerのprocess内Owner receiverへ承認された同一要求だけを渡し、Brokerが現在の要求、接続数、永続Auditと既存取得・内容露出条件を再評価する。

未承認・入力注入は通常IPCへ戻り、Brokerが拒否・監査する。Flutter／Swiftの承認bool、Owner session、秘密値、独自bridge、自動再送を受け付けない。native確認は既存300秒、承認後のBroker待機は10秒、Mac Flutter待機は315秒に限定する。取得失敗・未確定応答・Audit失敗を接続成功へ昇格しない。

作用分類: control経路。Capability=`a2a.connection.connect`、Permission=`permission.a2a.connection.connect`、Approval=同一要求hashのnative Owner確認、Audit=既存受信／拒否／受理event、Recovery=`recover-a2a-connection`。App Sandboxを維持する。URI実値・Card本文・秘密をAuditへ保存しない。成功UIは既存receiptのendpoint hash、未信頼metadata、`pending_review`、接続Auditだけを表示し、URI入力を消す。Task／Message／Artifact／Stream操作、外部host／HTTPS、Credential注入、Vendor互換拡張は本単位に含めない。

## 有限受入れ

| ID | 条件 | 状態 | 証拠 |
| --- | --- | --- | --- |
| MAC-A2A-1 | 現要求のnative Owner入口、拒否・hash／session／承認注入否定 | CLOSED | Windows直接1件・Mac source 5a7294f／run 37794151100直接1件PASS |
| MAC-A2A-2 | 製品UI→別個native確認→既存Broker→実loopback取得→metadata-only／未審査表示・URI消去・Audit | OPEN | 専用手動Mac製品試験 |
| MAC-A2A-3 | 対象build・直接依存試験、Command-Q正常終了、helper／専用fixture回収 | OPEN | 対象検査と同じ有限製品試験 |

新入口の直接正常／否定と製品正常一回・native拒否だけで閉じる。専用`workflow_dispatch` target=`macos_a2a_center`は、固定の公開Agent Cardを返す開発専用Rust fixture、通常Mac Debug build、通常XCTest入力を使用する。秘密・実Provider・署名鍵を使用しない。fixtureはIPv4 loopbackだけへlistenし、要求上限と期限を持ち、一回応答後に終了する。証拠は受入れ用の限定Audit投影と対象logだけ。これは合成Card／ad-hoc identityによる有限`LIVE_RUNTIME`で、一般外部Agentの信頼・Task実行・正式配布・Final QAを証明しない。

通常Release `task_execution=unsupported`、`release_ready=false`を保持する。追加fixture、強化証拠、fault matrix、Formal Evidenceは本受入れへ追加しない。PASSした条件を直ちにCLOSEDとする。

## 検証履歴

source `5a7294f4d345fbd45c66327c59a11e25ea90b810`、[run 37794151100](https://github.com/gatchimuchio/D4-POCKET/actions/runs/37794151100)は新native入口直接1件、通常native／Mac build、変更Dart解析、待機試験とURI消去Widget各1件がPASS。MAC-A2A-1をCLOSED。製品UIは検索候補のprefix文字を重複検出し、22.014秒でFAIL。接続・native拒否に未到達なのでAudit受入れは未成立、helper／fixture残留0・source cleanはPASS。artifact `11558500960`（73138 bytes）、SHA-256 `504e5815d131af29e74eaae84441080e2c03a0385dcdd6d1f5771de86be59b91`を実byteへ照合。検索候補のA2A接続だけを完全一致に限定する局所locator修正を行い、`a2a_product_only=true`で残件の製品操作だけを再実行する。CLOSED直接境界・成立済み投影は再試験しない。通常buildは試験App生成の前提で、強化証拠にしない。

最初の手動run 37793767655（source a854b49）はUI実行前のnative準備中に中止した。新規証拠gateのshell否定を`set -e`だけへ任せず、Audit本文混入・process残存時に明示終了値1とする局所修正のためで、製品受入れPASS／FAIL証拠へ転用しない。

失敗libを短縮出力で再確認すると514 passed／1 failed／12 ignored。差分外の既知`failed_tool_result_is_not_replayed_as_another_exec_command`がHTTP応答途中のConnectionReset（expected 23274、received 0）で失敗し、同testだけの再実行は1 passed。根因未確定の既存`FQ-TEST-LOOPBACK`へ属し、本A2A入口の受入れ条件を破壊する証拠ではない。全Rust FAIL履歴を保持し、fixture安定性の探索・修正は延期中Final QAへ残す。

2026-10-08: Windows上の新Owner入口直接試験1件、専用fixtureの`cargo check --example macos_a2a_fixture`、Schema 166／正常162／否定213、Conformance 242と手動起動限定検査がPASS。必須全Rust実行はlib targetがFAIL、他targetはPASSし、失敗libの原因確認を継続する。成功と書換えない。ローカルDesktop／Mobileの`flutter analyze --no-pub`は既知の日本語pathを含むLSP frame解釈障害（FormatException、server exit 255）でFAIL。変更Dartの形式はPASS、Mac上の対象解析と製品試験は未実行。日本語strict監査は既存5 file／17 findingsのみでFAIL、新規findingなし。manifest未再生成時のgate hash不一致は生成同期後にPASSし、編集終了時に再同期する。
