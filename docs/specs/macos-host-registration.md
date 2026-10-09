# macOS Host登録の製品入口

状態: CLOSED（P13 Product Build、2026-10-09）。最新版rev5と`docs/REV5_PRODUCT_PROGRESS.md`に従う。

## 意味と責任

操作者がHost操作面から公開Host ID、表示名、Platform、identity hash、申告Runtime／Agent件数を入力し、未審査metadataとして登録する。件数は申告値であって実測値ではない。取得不能値を0へ補完しない。endpoint、証明書実値、Credential、Permission、Approvalは入力・保存しない。Host切替は表示コンテキストだけで、remote接続や実行先変更ではない。

既存版1の`Host登録`／`Host一覧`／`Host切替`とSchema・Broker registryを再利用する。新しいwire契約・権限源を作らない。Mac UIは公開入力を既存channelへ送るだけ。Rust helperは現在要求のID、nonce、時刻、client metadata、payload hashと既存Host構造検査を使い、ID、名前、Platform、identity hash、申告件数、要求hashと非権限境界を別個の期限付きnative Owner確認へ表示する。承認された同一要求だけを既存process内Owner receiverへ送り、Brokerが重複、上限、現在構造、永続state／Auditを再評価する。拒否・不正入力は通常IPCへ戻り、Brokerが拒否・監査する。

作用分類: control経路。Capability=`host.registry.register`、Permission=`permission.host.registry.register`、Approval=同一要求のnative Owner確認、Audit=既存受信／拒否／受理event、Recovery=`recover-host-registration`。Owner確認300秒＋応答4秒、Flutter／Swift待機305秒。自動再送、UI承認bool、Owner session、独自bridge、App Sandbox解除は禁止する。Windows入口・保存形式・通常Release能力は変えない。

登録成功後に通常IPCで一覧を読み直す。現在の検証済みmetadataを表示・切替候補へ渡し、起動時snapshotだけを理由に新規Hostを拒否しない。受理receiptの同一ID、Audit参照、`metadata_only`、`pending_review`、`authority_strip`、権限非生成を確認する。未確定・欠落・不正receiptを成功へ昇格しない。

## 有限受入れ

| ID | 条件 | 状態 | 証拠 |
| --- | --- | --- | --- |
| MAC-HOST-1 | 同一要求native入口、拒否／hash・session・authority注入否定 | CLOSED | Windows直接1件PASS。新入口の構造・同一配送・Broker拒否保持 |
| MAC-HOST-2 | 公開GUI入力→別個native確認→Broker登録→通常一覧更新→未審査表示・表示切替・Audit | CLOSED | source `7220847`、run 37871901769の製品XCTest 1 PASS。登録／切替の受理Audit各1件。新Widget4 PASSはrun 37871125858を再利用 |
| MAC-HOST-3 | 対象解析／build、通常終了・helper回収 | CLOSED | run 37871901769の対象Dart解析No issues、通常native／Mac build、Command-Q終了、helper残留0・source clean |

公開合成metadataとtest identityで有限正常経路を一回確認する。入力／投影は`FIXTURE`、構造検査は`CONFIG`、実native確認・Broker・製品操作は限定`LIVE_RUNTIME`。既存CLOSED Host／Mac接続条件を再証明しない。remote connectivity、Task、Trust昇格、Credential、正式配布、fault matrix、最終QAは本単位に含めず、通常Release `task_execution=unsupported`／`release_ready=false`を保持する。PASSした条件をCLOSEDとし、追加検査しない。

## 検証履歴

2026-10-09: 新native入口1件とWindows側Broker platform限定入口1件がPASS。正常登録・要求field限定・同一ID／Audit・未知件数拒否・再送しない拒否表示・起動後Host選択の新Dart試験4件はASCII一時複製でPASS。初回Widgetのplatform override復旧時点と同文messageの二箇所表示に対するfinderが不正で2件FAILし、frameworkのplatform variantと複数表示finderへ局所修正した。正常機能の条件は変更しない。

通常checkoutのFlutter testは既知OneDrive build資産削除拒否で起動前FAIL、Desktop／Mobile必須解析は既知LSP不完全JSON／server exit 255でFAIL。追跡source・今回新DartだけをASCII Tempへ複製し、固定lockでpub get／対象testを実行した。製品環境・経路を変える回避策ではなく、複製は当単位終了時に回収する。複製上の対象Dart解析も既知Dart perf file削除OS error 1920でFAILし、Mac対象解析で別環境の結果を取得する。失敗をPASSへ書換えない。

最初のWindows必須全Rustはexit 0、lib 516 PASS／12 ignored、他targetもPASS。新HostだけのMac Owner許可を追加した最終sourceで必須全Rustを確認する。conformance初回はWindows RunnerとFlutter Owner待機集合の不一致でFAILしたためHost待機をMac限定へ修正し、Windows条件を保持した。新規日本語監査finding 2件は固定外部field参照と日本語UI labelへ局所修正した。既存5 file／17 findingsは履歴として保持する。

最終sourceのWindows全Rustはlib 516 PASS／1 FAIL／12 ignored、他target PASS。変更外`broker::update_download::tests::bounded_catalog_fetch_rejects_declared_document_over_limit`がlocalhost TLSのOS error 10054／ConnectionResetでFAILし、同対象の単独再実行は1 PASS。根因未確定の既存`FQ-TEST-LOOPBACK`へ同分類で記録する。全体FAILを成功へ変更せず、Hostの製品受入れ外のfixture探索は延期する。Conformance 243／Schema 166・162・213／手動起動限定／Manifest 1233／release gateがPASS。日本語strictは既存5 file／17 findingsだけでFAIL、新しいfindingは0件。

source `46e7380e78297dfc6f2cdc1d9df26d95b7148e4e`の[run 37871125858](https://github.com/gatchimuchio/D4-POCKET/actions/runs/37871125858)はMac Broker入口1件・新Widget4件・通常native／Mac buildがPASS。対象Dart解析はerror／warningなし、今回追加したifのbraces info 6件を局所修正する。製品XCTestは検索入力と候補が同じ「Host切替」でOCRを重複検出し、29.496秒でFAIL、登録未到達のためAudit受入れも未成立。検索語を「Host」へ変え、成立済み境界／Widgetは`host_product_only=true`で再実行せず、残件の製品入力・登録・切替・終了だけを検収する。通常buildは実行App生成の前提で強化証拠ではない。artifact `11590691382`、72073 bytes、SHA-256 `e2e0f1556f64f78eb538984e75da55f0e65944903ec6637548993664ed8fa813`を実byteへ照合し、ignored `release_evidence/p13-macos-host-46e7380/`へ保存した。

## 閉鎖証拠と復旧点

source `7220847bc775bec23f8088fc9d421209eb1ddc93`の[run 37871901769](https://github.com/gatchimuchio/D4-POCKET/actions/runs/37871901769)、`workflow_dispatch`／`macos_host_registration`／`host_product_only=true`でMAC-HOST-2〜3がPASS。macOS 15.7.9 arm64／Apple Virtual Machine 1／Xcode 16.4、Rust 1.95.0／Flutter 3.44.0。実製品の公開入力→別個Rust native Owner確認→既存Broker登録→通常一覧更新→起動後Hostの表示切替→Command-Q正常終了を一回実行した。XCTest 1 passed／0 failed／0 skipped、72.616秒。実Auditの登録／切替received・acceptedは各1件、helper残留0、runner source clean。本文・秘密・raw Auditはartifactへ保存しない。

正確な対象commandは`dart analyze lib/services/host_registration_client.dart lib/services/shell_core_client.dart lib/services/broker_client.dart lib/screens/host_operation_center.dart test/host_registration_test.dart`（No issues）、通常`cargo +1.95.0 build --locked --manifest-path native/rust_helper/Cargo.toml --bin gui_shell_macos_broker`と`cargo +1.95.0 build --locked --manifest-path native/macos_owner/Cargo.toml --lib`、`flutter build macos --debug --no-pub`、`xcodebuild test -project apps/desktop_flutter/macos/Runner.xcodeproj -scheme Runner -destination platform=macOS -parallel-testing-enabled NO -only-testing:RunnerUITests/AdapterOwnerUITests/testProductMacHostRegistration -derivedDataPath "$RUNNER_TEMP/macos-host-tests" -resultBundlePath "$RUNNER_TEMP/macos-host-tests.xcresult" CODE_SIGN_IDENTITY=- CODE_SIGN_STYLE=Manual SWIFT_ACTIVE_COMPILATION_CONDITIONS='DEBUG D4_MACOS_OWNER_UI_TEST'`。最後だけ既存のDebug表示補助を使い、確認・権限・資格を迂回しない。初回runのCLOSED native／Widget証拠は再実行しない。

artifact `11590738836`、70341 bytes、SHA-256 `612c61a8d71918b712c4692d6c53543de81be9c347d09a42e92c8485ab787c48`を実byteへ照合し、ignored `release_evidence/p13-macos-host-7220847/`へ保存した。成功sourceをmainへfast-forward・pushしremote HEADを照合、一時`codex/macos-host-verify` branchをlocal／remote双方で削除した。閉鎖文書変更前の2世代remote tagは`refs/tags/codex/backup-main`=`7220847bc775bec23f8088fc9d421209eb1ddc93`、`refs/tags/codex/backup-main-prev`=`8360e90e8fc074dbcbc6d37703a831f7ea3e7905`。文書rollbackは前者、本単位全体のrollbackは後者。

有限条件はすべてCLOSED。公開合成metadata・ad-hoc identityの限定`LIVE_RUNTIME`であり、申告件数を実測、表示切替をremote接続、metadataをAuthorityへ昇格しない。正常経路の回収だけを証明し、正式配布・crash／fault matrix・第三者内部保証は別gate／延期中Final QA。Windows全Rustの既知loopback FAIL、ローカルFlutter解析障害、日本語strictの既存指摘は履歴に保持する。通常Release `task_execution=unsupported`、`release_ready=false`は変更しない。

閉鎖時のローカル後片付け: 一時検証branchは双方回収済み。専用ASCII作業複製`C:/Users/ohira/AppData/Local/Temp/d4-host-e9b11b5d694e469a955b079bcc8f8c0e`と今回の`apps/desktop_flutter/flutter_42.log`／`apps/mobile_flutter/flutter_28.log`だけを対象に、解決済み絶対path・Temp範囲・非reparse root確認後の`Remove-Item -LiteralPath $d4TempResolved -Recurse -Force -ErrorAction Stop`を要求したが、command全体が実行前に`rejected: blocked by policy`となった。削除の再経路化はせず、ignored local資産を保持する。これは開発環境の`known_limitation`であり、Mac製品helper残留0とは別事実。source・証拠ZIP・利用者dataは削除していない。
