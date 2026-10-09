# macOS Host登録の製品入口

状態: VALIDATING（P13 Product Build、2026-10-09）。最新版rev5と`docs/REV5_PRODUCT_PROGRESS.md`に従う。

## 意味と責任

操作者がHost操作面から公開Host ID、表示名、Platform、identity hash、申告Runtime／Agent件数を入力し、未審査metadataとして登録する。件数は申告値であって実測値ではない。取得不能値を0へ補完しない。endpoint、証明書実値、Credential、Permission、Approvalは入力・保存しない。Host切替は表示コンテキストだけで、remote接続や実行先変更ではない。

既存版1の`Host登録`／`Host一覧`／`Host切替`とSchema・Broker registryを再利用する。新しいwire契約・権限源を作らない。Mac UIは公開入力を既存channelへ送るだけ。Rust helperは現在要求のID、nonce、時刻、client metadata、payload hashと既存Host構造検査を使い、ID、名前、Platform、identity hash、申告件数、要求hashと非権限境界を別個の期限付きnative Owner確認へ表示する。承認された同一要求だけを既存process内Owner receiverへ送り、Brokerが重複、上限、現在構造、永続state／Auditを再評価する。拒否・不正入力は通常IPCへ戻り、Brokerが拒否・監査する。

作用分類: control経路。Capability=`host.registry.register`、Permission=`permission.host.registry.register`、Approval=同一要求のnative Owner確認、Audit=既存受信／拒否／受理event、Recovery=`recover-host-registration`。Owner確認300秒＋応答4秒、Flutter／Swift待機305秒。自動再送、UI承認bool、Owner session、独自bridge、App Sandbox解除は禁止する。Windows入口・保存形式・通常Release能力は変えない。

登録成功後に通常IPCで一覧を読み直す。現在の検証済みmetadataを表示・切替候補へ渡し、起動時snapshotだけを理由に新規Hostを拒否しない。受理receiptの同一ID、Audit参照、`metadata_only`、`pending_review`、`authority_strip`、権限非生成を確認する。未確定・欠落・不正receiptを成功へ昇格しない。

## 有限受入れ

| ID | 条件 | 状態 | 証拠 |
| --- | --- | --- | --- |
| MAC-HOST-1 | 同一要求native入口、拒否／hash・session・authority注入否定 | CLOSED | Windows直接1件PASS。新入口の構造・同一配送・Broker拒否保持 |
| MAC-HOST-2 | 公開GUI入力→別個native確認→Broker登録→通常一覧更新→未審査表示・表示切替・Audit | OPEN | 新Widget正常／拒否とMac製品正常一回 |
| MAC-HOST-3 | 対象解析／build、通常終了・helper回収 | OPEN | 手動Actions `macos_host_registration` |

公開合成metadataとtest identityで有限正常経路を一回確認する。入力／投影は`FIXTURE`、構造検査は`CONFIG`、実native確認・Broker・製品操作は限定`LIVE_RUNTIME`。既存CLOSED Host／Mac接続条件を再証明しない。remote connectivity、Task、Trust昇格、Credential、正式配布、fault matrix、最終QAは本単位に含めず、通常Release `task_execution=unsupported`／`release_ready=false`を保持する。PASSした条件をCLOSEDとし、追加検査しない。

## 検証履歴

2026-10-09: 新native入口1件とWindows側Broker platform限定入口1件がPASS。正常登録・要求field限定・同一ID／Audit・未知件数拒否・再送しない拒否表示・起動後Host選択の新Dart試験4件はASCII一時複製でPASS。初回Widgetのplatform override復旧時点と同文messageの二箇所表示に対するfinderが不正で2件FAILし、frameworkのplatform variantと複数表示finderへ局所修正した。正常機能の条件は変更しない。

通常checkoutのFlutter testは既知OneDrive build資産削除拒否で起動前FAIL、Desktop／Mobile必須解析は既知LSP不完全JSON／server exit 255でFAIL。追跡source・今回新DartだけをASCII Tempへ複製し、固定lockでpub get／対象testを実行した。製品環境・経路を変える回避策ではなく、複製は当単位終了時に回収する。複製上の対象Dart解析も既知Dart perf file削除OS error 1920でFAILし、Mac対象解析で別環境の結果を取得する。失敗をPASSへ書換えない。

最初のWindows必須全Rustはexit 0、lib 516 PASS／12 ignored、他targetもPASS。新HostだけのMac Owner許可を追加した最終sourceで必須全Rustを確認する。conformance初回はWindows RunnerとFlutter Owner待機集合の不一致でFAILしたためHost待機をMac限定へ修正し、Windows条件を保持した。新規日本語監査finding 2件は固定外部field参照と日本語UI labelへ局所修正した。既存5 file／17 findingsは履歴として保持する。

最終sourceのWindows全Rustはlib 516 PASS／1 FAIL／12 ignored、他target PASS。変更外`broker::update_download::tests::bounded_catalog_fetch_rejects_declared_document_over_limit`がlocalhost TLSのOS error 10054／ConnectionResetでFAILし、同対象の単独再実行は1 PASS。根因未確定の既存`FQ-TEST-LOOPBACK`へ同分類で記録する。全体FAILを成功へ変更せず、Hostの製品受入れ外のfixture探索は延期する。Conformance 243／Schema 166・162・213／手動起動限定／Manifest 1233／release gateがPASS。日本語strictは既存5 file／17 findingsだけでFAIL、新しいfindingは0件。
