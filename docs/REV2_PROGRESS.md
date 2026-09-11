# rev2 実装進捗と証拠境界

各節は作業時点の履歴である。現在状態は末尾の「現時点の要求監査」を優先し、過去の未実装記述を現在の状態へ読み替えない。

## 統治変更（2026-09-10）

対象はローカル品質判定と手動補助 Actions の分離。製品 runtime の権限・実行経路は変更しない。自動 CI は禁止を維持し、手動起動条件を構造として検査する。証拠分類は CONFIG / FIXTURE であり、外部実行や branch protection の保証ではない。

`python tooling/schema_check/check_schemas.py`、`python tooling/conformance_tests/run_conformance_skeleton.py`、`python tooling/日本語基底監査.py --strict` は PASS。Conformance は142項目。手動起動の文字列・列挙・対応形式、不在、実ファイル読取りを確認し、自動起動の混在、重複鍵、不正 YAML、独自タグを拒否した。

`PYTHONUTF8=1` を設定した Windows の `python tooling/validate_all.py --desktop-platform=windows` は FAIL。既存の `packaging_portability_check` だけが失敗し、他の13検査は PASS。Rust は42単体＋4統合、Flutter は31試験、desktop/mobile analyze と broker parity も PASS。日本語4ファイルの展開名不整合は変更前から存在し、独立した最小 ZIP でも再現した。UTF-8 locale 指定でも改善しない。統治変更に起因する失敗ではない。

WSL/Linux の `python3 tooling/validate_all.py --python-only --desktop-platform=linux` は9検査すべて PASS。これは Python 側の検証であり、reporter が併記する過去の Linux build / launch 記録を今回の実行証拠とは扱わない。Windows の環境依存の失敗は以下へ分離して保持する。

- item: Windows の ZIP 展開名不整合
  classification: release_blocker
  reason: Git for Windows 同梱 unzip で日本語名が変化する環境依存の失敗。統治変更で検査を除外しない。
  required_action: 次の Baseline 単位で標準展開機構を確認し、同一の manifest / conformance / release gate 検査を維持して修正する。
  blocks_release: yes

- item: installed-path 証拠・実機検証・owner GO
  classification: release_blocker
  reason: 統治単位は製品の完成証拠を作成しない。strict release は未実行である。
  required_action: 製品単位で正式な実機証拠と明示承認を揃える。
  blocks_release: yes

後続の実行系対話、MINIDORA Adapter、比較、Mobile、端末連携、各 platform の実装と検証は未完了。今回の統治単位の完成をそれらの完成へ昇格しない。これらの owner 指示内の未実装は rev2 完成に対する release_blocker として次単位以降で扱う。GitHub Actions は未使用。

## Baseline の Windows 配布検証修正

Git for Windows 同梱 unzip 6.00 は最小 ZIP の日本語名も文字化けさせた。`LC_ALL=C.UTF-8` と `-UU` でも再現した。一方、Windows 標準 tar.exe（bsdtar 3.8.8）は `LC_ALL=C` のまま同じ ZIP の名前を保持した。

検証専用経路を Windows は System32/tar.exe、POSIX は従来の unzip に分けた。独自 wrapper、代替の製品 runtime、検査除外は追加しない。展開後の manifest、conformance、release gate 検査は維持する。適用範囲は dev / release validation の source ZIP 展開のみである。新しい日本語名・空白を含むパスの実展開、内容hash一致、破損ZIPの拒否、展開器不在を試験する。これは当該 OS の外部展開器に対する EXTERNAL_EVIDENCE であり installed 製品証拠ではない。

本修正は Windows 標準機構の恒久的な選択である。将来対応OSの標準展開器が変わる場合は、同じ名前・内容・失敗の試験と実展開後検査を成立させて選択を見直す。

検証結果: Windows の `python tooling/validate_all.py --desktop-platform=windows` は14検査すべて PASS（終了値0）。WSL/Linux の `python3 tooling/validate_all.py --python-only --desktop-platform=linux` は9検査すべて PASS。Conformance は143項目。上記の Windows ZIP 展開名不整合はこの修正で解消した。installed-path 証拠と owner GO は未解消の release_blocker のままであり、開発用集約結果の pass を製品 release の許可にしない。

## 日本語意味正本・契約・Conformance

`docs/specs/runtime-dialogue.md` で要求・応答・セッション・比較、取消の限界、表示境界、権限と監査の前提を定義した。MINIDORA API の実コードは commit `3400a3bb68b37efa1dc14ee8aaa28fda779bf1f8` へ固定した。参照系のコードは変更しない。

四つの Schema、正常・権限混入の否定fixture、開発専用の関係検査を接続した。要求上限、本文上限、参照数上限、空白入力、識別子末尾の改行、未知field、閉じたセッション、応答対応、非全文表示への漏洩、左右成功・片側失敗・両失敗・応答入替え・セッション共有・表示許可転用を検査する。これらは CONFIG / FIXTURE 証拠であり、MINIDORA の live 動作、実行系停止、権限発行の証拠ではない。

- item: 製品対話の実装接続
  classification: release_blocker
  reason: 現単位の消費経路は Schema catalog と開発用 conformance。Rust 外部送信、UI、端末連携は未接続。
  required_action: 次単位で統治済み Rust 経路と Adapter、UI を実装し、実物通信と失敗・権限否定・取消を検証する。
  blocks_release: yes

契約単位の検証: Schema 30件、正常例30件、否定例32件、Conformance 145項目は PASS。Windows 開発用一括検証14検査と WSL/Linux の Python 側9検査は PASS。比較単独での空白入力の見逃しを追加監査で検出し、応答・比較の関係検査も拒否するよう補強した。権限判定や許可発行をこの開発検証へ持ち込まない。

製品接続の前提確認: 現行 `native/rust_helper/src/broker/authority.rs` の production_default は decision=deny、approvals=[] であり、owner 承認の登録経路がない。継続指示に基づき専用の Rust ローカルowner制御操作を採用した。以下の単位で通常UI資格と分離する。既存汎用commandのdeny/suspendを解除する変更ではない。

## Rust対話Core・MINIDORA Adapter・owner制御

専用の対話要求を通常IPCで保留し、別資格のowner CLIによる要求hash一致・期限内・一回限りの承認後だけ固定loopback APIを呼ぶ。Shell Coreは汎用traitを使い、MINIDORA固有のHTTPと応答射影はAdapterへ分離した。Flutter、Python、metadataから権限を発生させない。既存汎用command dispatchはsuspendedを保持する。

通信期限、要求・受信上限、worker数上限、セッション隔離、取消後の採用防止、raw受信と表示射影の分離を実装した。監査失敗時は送信または結果公開を拒否する。成功Adapter応答もCoreでセッションと構造を再検査する。owner資格は同一OS利用者に対する強い隔離や人間本人性の証明ではない。Windows installed-path保護の未検証を隠さない。

検証: `cargo test --manifest-path native/rust_helper/Cargo.toml` は53単体・5統合がPASS。`python tooling/minidora_live_check.py --reference C:/Users/mzcum/codex-work/MINIDORA-reference` は固定commitの実MINIDORA二processを用いPASS。通常資格拒否、owner CLI承認、基本会話、trace整合、表示分離、保留、片側停止・両側停止、監査chainの再起動読取を実行した。実API証拠はLIVE_RUNTIMEだが、基礎Core・外部検索能力の保証へ拡張しない。Schema31件・正常例31件・否定例33件、Conformance146項目、日本語厳格監査もPASS。

HTTP固定ヘッダーとCRLFだけを `JBE-007` の局所固定表記へ登録した。監査器の検出は弱めず、説明と診断は日本語のまま残す。Brokerは非同期状態を所有するためClone/Eqの導出を廃止した。生存workerや承認待ち状態を複製する内部APIは提供しない。

- item: Desktop比較UI・Mobile・端末連携・platform別実機証拠
  classification: release_blocker
  reason: この単位で接続したのはRust対話経路と実API検証。UI実装、secure storage、端末資格、Android/iOSの実動作は後続単位に残る。
  required_action: Flutter共有化と対話表示、二実行系比較、端末連携、platform別buildと実機検証を順次実装する。
  blocks_release: yes

- item: owner資格のOS保護とinstalled-path検証
  classification: release_blocker
  reason: ローカル開発資格の分離は同一利用者の任意processや管理者への耐性を証明しない。
  required_action: 既存の資格保護・監査anchorのrelease gateに従いWindows installed-path証拠を収集する。
  blocks_release: yes

集約検証: Windowsの `python tooling/validate_all.py --desktop-platform=windows` は14検査すべてPASS（終了値0）。WSL/Linuxでも `cargo test --manifest-path native/rust_helper/Cargo.toml` の53単体・5統合がPASSし、Linux binaryとLinux側の同一commit参照cloneによる `tooling/minidora_live_check.py` がPASSした。Windows cloneをWSL Gitで確認した際の改行差分は参照コードを書き換えず別cloneで分離した。GitHub Actionsは未使用。これらは開発環境の検証であり、installed-pathやMobile実機、owner GOのrelease_blockerを解除しない。

## Desktop対話・二実行系比較

Desktopのナビゲーションとコマンドパレットに対話を追加した。新規セッション、実行系選択、入力、送信、中止、応答、参照、能力、経路、追跡、失敗・復旧を表示する。比較では異なる実行系とセッションへ同じ入力を独立送信する。通常clientの操作allowlistにowner承認を含めず、応答の要求・実行系・session・表示境界を検査する。画面遷移で対話を保持し、デモ表示では新しい操作を無効にする。下部バーは対話の状態を起動時snapshotから推定しない。

接続資格のhostは127.0.0.1、TCP port・乱数secretの構造を検証し、Flutterの受信上限を4MiBにした。実行系への直接通信は追加していない。

Windowsで `flutter analyze`、`flutter test`（37件）、`flutter build windows --debug` を実行した。画面試験は左右の同一入力、独立した失敗表示、中止、デモの送信禁止を含む。製品Dart clientによる `python tooling/minidora_live_check.py --reference C:/Users/mzcum/codex-work/MINIDORA-reference --dart-client` もPASS。

Windowsの開発binaryを実起動し、Computer Useでコマンドパレットから対話画面へ移動、比較を選び「こんにちは」を送信した。左右に異なるsession・要求が作られ、owner CLI承認後に両方の実MINIDORA応答と異なる追跡IDが画面表示されたことを観測した。これは開発binaryのLIVE_RUNTIME観測であり、installer配布・installed-pathのrelease証拠ではない。

Dartの自動修正toolは終了時にperf一時file削除のOS Error 1920で失敗したため、lint指摘をソースで修正し、その後のanalyze/testで検証した。製品コードにtool障害の回避層は入れていない。

次単位のbackup更新で短いref名がbranch/tagの同名と衝突した。`git push -f origin codex/backup-main-prev:refs/tags/codex/backup-main-prev codex/backup-main:refs/tags/codex/backup-main` は曖昧なrefとして失敗した。完全な `refs/heads/...` 指定で修復し、remote tagの新世代5499637、前世代9bd8838を確認してから編集を開始した。今後も完全なref名で区別する。

- item: Flutter共通化・Mobile・端末連携・platform別最終証拠
  classification: release_blocker
  reason: Desktop対話の実装と検証をMobileやinstalled-pathの成立へ昇格しない。
  required_action: 次の共通化、Mobile正式project、端末認証と安全保管、各platformの検証を実施する。
  blocks_release: yes

Desktop単位の最終検証: Windowsの一括14検査はすべてPASS（終了値0）。Linux側の同一差分でも `flutter analyze`、`flutter test`（37件）、`flutter build linux --debug` がPASS。Linux製品Dart client→Rust→実MINIDORAの試験もPASSした。WSLgの `GDK_BACKEND=x11` でLinux開発binaryを起動し、broker接続とX11のIsViewableウィンドウを観測した。既定WaylandのウィンドウをX11検査器で検出できなかったことは、製品regressionとは分類しない。X11指定は試験の環境設定だけであり製品に追加していない。実起動証拠の環境範囲はREADMEのknown_limitationへ記録した。

## Flutter共通表示とMobile platform構成

対話画面・安全な応答検査・通常transport契約を `packages/gui_shell_ui` へ移した。接続は注入し、Desktopの資格読取とTCPはDesktop側へ残した。Dartのみのclient入口とFlutter表示入口を分け、dev-only実API検証がFlutter engineを必要としない構成を保持した。共通5試験とDesktop32試験で元の37試験の責任を保持する。

既存Mobileのlibを保持したまま、Flutter標準生成元からAndroid/iOS projectを追加した。Mobile依存lockfileはGit追跡するが、既存規約どおりMANIFEST対象から除外する。releaseの開発鍵流用は除去した。開発識別子・配布署名・実機証拠はMobile READMEのrelease_blockerとして明記した。

実API回帰の初回はFlutter表示層の推移的importで起動期限を超過した。client入口の分離後、`python tooling/minidora_live_check.py --reference C:/Users/mzcum/codex-work/MINIDORA-reference --dart-client` はPASS。Windows bat親processの終了だけでは子Dartが試験logを保持したため、検証toolはFlutter同梱のDart executableを直接起動するようにした。製品起動経路への回避層追加ではない。

集約検証の初回は16項目中14項目がPASS。Mobile lockfileをMANIFESTへ含めた変更が既存conformanceと衝突し、conformanceとZIP展開後conformanceが失敗した。MANIFESTの既存除外規約を維持する形に戻した。検査の削除・弱体化は行っていない。

- item: Mobile端末連携・安全保管・Android/iOS実機証拠
  classification: release_blocker
  reason: この単位は共有表示とplatform構成。Mobile libは従来の試作画面であり、実状態ではない。
  required_action: 端末資格、Desktopへの暗号化接続、失効、設定、lifecycleを実装して実機検証する。
  blocks_release: yes

修正後の conformance（146件）と packaging portability はPASS。共通・Desktop・Mobile analyze、共通5件・Desktop32件test、Rust53単体・5統合は集約検証でPASSした。

共有化後の lutter build windows --debug もPASS。Android/iOSのnative buildはこの単位では未検証であり、上記platform別release_blockerに含める。

## 端末連携の契約

招待・結合資格・暗号化要求の三Schemaと日本語意味正本を追加した。通常操作のallowlist、Host証明書固定、端末所有関係、招待300秒・結合8時間、nonce再使用拒否、失効・lifecycle・安全保管を定義した。port上限を機械検証するためSchema検証器にmaximum検査を追加した。

`python tooling/schema_check/check_schemas.py` は34 Schema・34正常例・36否定例、`python tooling/conformance_tests/run_conformance_skeleton.py` は147項目でPASS。必須field欠落、未知権限field、不正型、port境界、禁止操作、操作と内容の不一致を含む。証拠classはFIXTUREであり、暗号化や端末認証の実動作を証明しない。

- item: 端末連携の製品消費経路
  classification: release_blocker
  reason: この単位は契約と構造検査。TLS・資格・所有関係・安全保管の製品経路は次単位で実装する。
  required_action: RustとMobileの正常・否定経路および実通信を検証する。
  blocks_release: yes

## Rustの端末TLS経路

明示bind時だけ公開するTLS経路を同じDesktop Rust brokerへ接続した。通常loopbackとowner資格の分離を保持し、端末操作は対話・確認・離脱だけを許可する。招待の消費、秘密hash照合、期限、nonce、Host・端末結合、対話所有関係をRustで強制する。入力は全階層の重複fieldを拒否し、受信上限と有限期限を持つ。owner招待秘密は新規fileへ保存して標準出力へ出さない。

既存Rust53単体・5統合は変更後にPASS。端末状態機械の6否定／正常試験と、永続監査障害・期限処理を実Coreへ通す2試験もPASS。実TLSのdev-only clientによる正常結合、再接続、再使用拒否、不正資格、Host不一致、権限昇格拒否、他端末の対話・要求アクセス拒否、実MINIDORA応答、失効後の保留承認拒否、招待取消、離脱を実行した。製品Desktop Dart clientとの併用もPASS。

新依存はTLS・証明書生成・秘密hash比較に限定し、Cargo.lockへ固定した。通常RuntimeのMINIDORA接続は引き続き既存Adapter経路だけである。TLSの実通信証拠をMobile製品画面やOS安全保管の成立へ昇格しない。

- item: Mobile製品clientと安全保管・lifecycle
  classification: release_blocker
  reason: Rustの端末経路は実動作したが、Mobile libへの接続とAndroid/iOS実機証拠は未完成。
  required_action: MobileでHost照合・安全保管・対話画面・復帰処理を接続し検証する。
  blocks_release: yes

端末失効の反復試験では、当初64回で対話枠が尽きる資源保持を再現した。失効済みsessionを解放し、送信中workerだけは遅延応答のhash監査まで保持する修正を行った。修正後の実TLSによる結合・対話開始・失効70回はすべてPASS。遅延応答の破棄監査と資源解放の回帰試験も追加した。最終Rust検証は63単体・5統合でPASS。

集約検証は14項目PASS、2項目（broker authority parity、cargo test）が起動中のWindows binaryと再buildの競合によるOS error 5で失敗した。実通信processの終了後に順番を分け、両項目を再実行してPASS。製品や検査に回避層は追加していない。修正後の実TLS＋実MINIDORA＋製品Dart clientも再実行してPASSした。

最終監査では、試験内のraw file読取がhelper境界の禁止patternに該当したため、既存の `BrokerPersistentStore` 再読取・chain検証経路で監査記録を確認する試験へ置換した。検出器は変更していない。また高負荷時に既存取消試験の固定150ms待機が不足したため、取消の即時結果と空本文のassertionを保持し、2秒以内の実受信完了を待ってraw保持を検査する形へ修正した。再実行は63単体・5統合でPASS。TLS session再開も無効にし、毎接続の端末資格検査を維持した。


## Mobile製品client・安全保管・lifecycle

MobileのTLS clientは招待のHost・端末ID・証明書hashを固定し、実peer照合と証明書有効期間の確認より前にapplication資格を送らない。重複field・未知field・不正型・期限・private IPv4境界を拒否する。通常操作allowlistと有限通信期限を持ち、owner資格やRuntime直結を導入しない。

Androidの安全保管とiOS Keychainを接続し、書込後の再読取を必須にした。保管失敗時の平文fallbackはない。Android backup・device transferを除外した。復帰時は資格を再確認し、background中の通信と保留入力の自動再送を停止する。失効・不正資格・通信失敗時は入力を止め、正常解除と通信不能時のlocal削除を区別する。

既存6画面を維持し、対話・接続先・設定を追加した。固定previewの準備完了・承認件数を除去した。共有対話画面はinactive時に接続・polling・送信を停止し、再開時に照会する。旧previewのdevice_id / pairing_idと現行端末ID / 結合IDの対応を復旧画面ソースへ記録し、既存conformanceの用語検査を保持した。

`python tooling/minidora_live_check.py --reference C:/Users/mzcum/codex-work/MINIDORA-reference --dart-mobile-client` はPASS。Mobile製品Dart client → 実TLS → Rust Core → 実MINIDORA二processで本文・追跡・owner承認を検証した。異なる証明書の拒否、停止・再確認、失効後の拒否もPASS。OS安全保管と実機lifecycleの証拠には昇格しない。

`flutter analyze` と `flutter test`（Mobile12件）はPASS。初回widget試験は概要とdrawerの同名表示を両方拾って失敗したため、検査対象を実際のNavigationDrawerに限定して再実行した。安全保管失敗、期限・構造・Host不一致、資格確認と復帰の競合、local削除、破損資格の上書き拒否を含むFIXTURE検証である。

Android初回buildはFlutter生成値のGradle heap 8GBでnative memory allocationに失敗した（環境要因）。heap 2GB、metaspace 768MB、worker 2に限定した。次のbuildではflutter_secure_storage 11がSDK 37を要求し、AGP 9.0.1が新しいandroid-37.0を解決できず失敗した（toolchain互換性）。標準のSDK packageが不存在の`platforms;android-37`を要求するsdkmanagerコマンドも失敗した。公式対応表に従いAGP 9.1.1 / Gradle 9.3.1へ更新した。SDK directoryの偽装・pluginソース改変・依存検査の抑止は行わない。

一次資料: [AGP 9.1.1の対応範囲](https://developer.android.com/build/releases/agp-9-1-0-release-notes)、[FlutterのAGP 9移行](https://docs.flutter.dev/release/breaking-changes/migrate-to-built-in-kotlin)。

- item: Android/iOSのOS安全保管・実機install・launch・lifecycle
  classification: release_blocker
  reason: host上のDart実通信とFlutter fixtureは実機での成立を証明しない。ADB接続済み実機はまだ検出されていない。
  required_action: buildを成立させ、実機で保存・再起動・結合・対話・失効・復帰を測定する。
  blocks_release: yes

集約検証は17項目中16項目がPASSし、共有UI analyzeの波括弧lintだけが失敗した。修正後の共有analyzeはPASS。共有6試験・Desktop32試験・Rust63単体と5統合・broker parity・packaging・Schema・conformanceは集約時にPASSした。MobileはHost照合前の送信禁止と保管障害からの復帰試験を追加し、最終analyze・14試験がPASSした。


AndroidのAGP修正後、`flutter build apk --debug` はPASS（初回1778.1秒）。IME学習・自動入力の無効指定を含む最終ソースで再buildし38.5秒、`flutter build appbundle --debug` は42.0秒でPASS。SDK 35とCMake 3.22.1もpluginの標準依存として導入された。Kotlinの生存markerは`.kotlin` cacheとしてignoreし、生成物をcommitしない。

開発APKは175738543 bytes、SHA-256 `e5c5f30108f812d92e444993087c417a812806c2f47f3a445454035743dffdb6`。開発AABは71596894 bytes、SHA-256 `0ce3fdb996e048c98e665c5c770bac8a188a5e39dc3b57b115a395da35764586`。APKはarm64-v8a、armeabi-v7a、x86_64を含む。`apksigner verify --verbose --print-certs` はPASS、Android DebugのRSA 2048 / v2署名であり公開配布署名ではない。`zipalign -c -P 16 4` はPASS。`apkanalyzer manifest print` でmin SDK 24、target SDK 36、debuggable=true、allowBackup=false、fullBackupContent=false、usesCleartextTraffic=false、dataExtractionRulesの実格納を確認した。これは生成物のCONFIG / EXTERNAL_EVIDENCEであり実機の動作証拠ではない。

JDK17の`jarsigner -verify`は終了値0だが、自己署名・timestampなし・POSIX属性・JarFileとJarInputStreamの検証差について警告した。警告を削除するための再梱包は行わない。公式bundletool 1.18.3（公開asset SHA-256 `a099cfa1543f55593bc2ed16a70a7c67fe54b1747bb7301f37fdfd6d91028e29` を照合）の`validate --bundle=...`と`build-apks --bundle=... --mode=universal --output=...`はいずれもPASS。変換したuniversal APKのapksigner検証もPASS。AABのAndroid工具による消費は確認できたが、正式配布・署名・汎用JAR stream検証差は次の配布前確認へ保持する。

## Apple platformの手動補助build

このローカルhostはWindows/WSLであり、ローカルMacはない。owner rev2とAGENTS 3.1に従い、workflow_dispatchだけの補助workflowを追加した。Windowsで検証したFlutter commit `559ffa3f75e7402d65a8def9c28389a9b2e6fe42` とRust 1.95.0を用い、macOS開発app・iOS Simulator app・Rust helperのbuildを対象とする。GitHubの品質必須statusや自動CIは追加しない。対象commitと環境情報、log、tar成果物、hash、buildによるソース差分を収集する。追跡ソースが自動変更された場合は成功とせず、差分を確認する。

- item: Apple補助実行と実機証拠
  classification: release_blocker
  reason: workflowの追加は外部実行の成功やMac/iOS実機の成立を証明しない。
  required_action: 手動実行の対象commit・結果・artifactを確認し、実機install・launch・安全保管・lifecycleは別途測定する。
  blocks_release: yes

workflow追加のlocal検証はSchema（35正常・37否定）、conformance（147項目）、日本語厳格監査がPASS。これは手動起動限定のCONFIG検証であり、外部build結果は手動実行後に記録する。


Apple初回補助実行（run 34445302630、対象52bcbd2cc82519f5f6ebc6c80c8c60f96a1e12af）はRust64単体・5統合およびarm64 Mach-O buildがPASS。macOS project未追加によりFlutter buildがFAILし、iOSは未実行となった。分類はproduct regressionではなく未実装構成の検出であり、この時点のApple buildはrelease_blockerである。固定Flutter標準生成元からmacOS projectを追加し、Sandboxを保持したまま既存broker用network.clientを指定する。正式配布・実機資格配置はDesktop READMEのrelease_blockerへ保持する。


## 複数OS間のmanifest修復

Linux最新checkoutの集約検証ではFlutter/Rust関連検査はPASSしたが、manifest・release gate・梱包の3検査が失敗した。Windows編集時のCRLFをraw hashへ記録し、Gitが既存.gitattributesに従ってLFへ保存したことが原因である。検証toolのhash照合は変更せず、作業fileを既存の改行規約へ戻してmanifestを再生成する。生成時にはGitのeol属性と作業byteの不一致を拒否し、無言の正規化やhash比較の緩和を行わない。Git実repositoryを使いLF、明示CRLF、binary、混在改行、修復後の正常化を検証する。


## 現時点の要求監査（2026-09-10）

rev2全体は未完了。実装済みの対話Core・MINIDORA Adapter・Desktop比較・Mobile端末連携と、実機で未確認の範囲を分離する。完成製品releaseとowner GOは主張しない。

|要求|確認した経路・結果|証拠の限界|
|---|---|---|
|統治・日本語意味正本・Schema|手動Actions限定、対話・比較・端末連携の正本、35 Schema、35正常・37否定fixture PASS|CONFIG/FIXTURE|
|CoreとMINIDORA Adapter|Rustの要求・承認・監査・取消・隔離試験、固定参照commit 3400a3bb68b37efa1dc14ee8aaa28fda779bf1f8の実API PASS|基本会話と保留の範囲。基礎Core能力の保証ではない|
|対話・比較|共有6件、Desktop32件、Mobile14件、左右成功・片側失敗・両失敗・session/権限非混線の試験 PASS|画面fixtureとhost上の実通信を区別|
|端末連携|製品Dart TLS client、招待・失効・replay・Host・権限否定、実MINIDORA PASS|OS安全保管は実機未確認|
|Android版|analyze/test、APK/AAB、署名・alignment・bundletool PASS|実機install以降はrelease_blocker|
|Apple版|手動実行34446194013、対象`27b8713fd1a9ecdb81abe1d4225b99b26bda84ba`でRust単体64・統合5試験、macOS開発app・iOS Simulator appのビルドに合格|実機launch・Keychain・対話はrelease_blocker|
|Windows版|共有化・端末連携後の集約検査でlint修正後PASS、最新debug build PASS|2026-09-11に画面回帰を再開し下記の範囲で確認。installed-path証拠はrelease_blocker|
|Linux版|最新対話実装のFlutter build、各analyze/test、Rust試験、Desktop/Mobile製品Dart clientと実MINIDORA PASS|最新release起動とPID一致の可視window・broker監査も確認。WSLg X11の範囲|
|manifest・梱包|改行修復後のWindows/Linux Python系9検査 PASS。conformance148件|開発検証のpassはstrict releaseのpassではない|

Apple成果物は外部artifact `10139803376`（86677901 bytes）を取得しZIP SHA-256 `3110e6493467eef9c38dc371746655dbbbb7efeb89696ba03421c4916752a9ac` を照合した。内部tar SHA-256 `59496c1c87f3b36f4bb3d7f592457df3315af3099155066f24b0200495041cca` も一致し、両app・Rust実行file・iOS安全保管plugin資産を確認した。環境はmacOS15.7.9 arm64、Xcode16.4、iOS Simulator SDK18.5。追跡差分patchは空。Flutterが生成したmacOS registrantは既存Windows/Linuxと同じく追跡対象へ追加する。生成内容を手編集せず、依存の生成元はpubspecである。

artifact取得の初回HTTP直取得はredirect先で401となり、認証headerを別hostへ引き継がない取得で回復した。ログ表示のcp932 UnicodeEncodeErrorはPYTHONUTF8=1で回復した。いずれも製品build失敗ではない。

Windowsの画面回帰は当初Escで中断したが、2026-09-11のowner再開指示後に下記の開発回帰を実測した。installed-pathの独立したrelease blockerは保持する。

- item: 実機・installed-path・正式配布・owner GO
  classification: release_blocker
  reason: Android実機検証はowner指示で凍結中。Mac/iOS実機なし。Windows installed-path全体の証拠と正式配布署名も未成立。
  required_action: Androidはownerの再開指示を待つ。それ以外の必要な実機証拠・配布指定とstrict validationは独立して扱う。
  blocks_release: yes


## Windows隔離配置・Linux最新起動の追加証拠

対象ソースはcleanな `47299ce839f51f0bcb39cd3d19d98f69f1510003`。Windowsの `cargo build --release --locked --manifest-path native/rust_helper/Cargo.toml` は95秒、`flutter build windows --release` は92.4秒でPASS。`installer/windows/stage_installed_app.ps1` で新規run `rev2-47299ce-20260910` を作成し、source_worktree_clean=trueと生成manifestの実artifact hash一致を確認した。Flutter exe SHA-256は `e671cd41eeb1c1d9c147178f5a4ae5a53052ac375807f421c9bc31e49769f7e6`、Rust exeは `d2015557a120f6552aceb518fc6e30fce36c744b4b384e19bd97161548dbbf07`。

この配置に対する `installer/windows/collect_broker_smoke.ps1` はstatus=passed、errors=[]。制限loopback、認証付きIPC、永続store、再起動後のnonce再使用拒否、新規要求の受理、強制終了後の接続拒否を実測した。証拠は `%LOCALAPPDATA%/GUI-Shell/installed-runs/rev2-47299ce-20260910/runtime/evidence/windows_broker_smoke.json`。画面・Setup Doctor・監査アンカー保護を測定していないため、Windows installed製品全体のgateは解除しない。

Linuxは同じソース系統のrelease binary（SHA-256 `6582fd0acc0b94f0a1c09239af5626c1ada4180150f1ff00a9d25a7bc47268c8`）を実MINIDORA二processとRust brokerに接続して起動した。`GDK_BACKEND=x11` のWSLg環境で、`xwininfo -root -tree`、対象の`xprop -id <観測ID> _NET_WM_PID`、`xwininfo -id <観測ID>` により起動PID411との一致とIsViewableを観測した。app終了前に生存も確認し、検証後に自身のprocessを終了した。

永続監査にはflutter-request-1から5が記録された。health、normalize_payload、content_projection、approval_editの受理と、command_envelopeのsuspendedを確認した。これは起動時の既存broker経路の証拠であり、画面からの対話入力・表示内容全体の証拠ではない。Desktop/Mobile製品Dart clientから実MINIDORAまでの経路は別の実通信検証でPASSしている。DRI3 deviceを取得できないlibEGL警告は出たが起動は成立した。描画性能・Wayland・物理Linux端末への同等性は主張しない。


## Windows画面回帰の再開・Android実機検証の凍結（2026-09-11）

ownerの明示指示でWindows画面検証を再開した。対象は `d5341f96f207b57085453af4797536abe125a5ed` の実装を持つWindows debug app、Rust broker、固定参照MINIDORA二process。Computer Useによる可視画面の操作・観測であり、検証入力は「こんにちは」だけとした。設定・権限の変更は行っていない。

概要、環境診断、信頼、実行系、権限、agent、承認、監査、復旧、問題、証拠、設定、対話の13画面へ移動し、表示を確認した。既存画面から対話へ戻った際も入力と左右のsession状態を保持した。監査画面の既存projectionを新しい対話監査の表示証拠へ読み替えていない。

二実行系比較で同一入力を送信し、左右の異なるsession・要求IDと承認待ちを観測した。検証用owner CLIの `対話承認操作 --session-file <検証用owner file> 承認 <要求ID> <要求hash> full` で各要求を承認した後、双方の完了、成功、full、基本会話の応答、別々のtrace/hashを画面で確認した。通常UIへowner資格は渡していない。

次の要求では左を中止しても右の承認待ちが維持され、右も個別に中止できた。左だけ新規sessionへ切り替え、右の中止済みsessionを残して再送すると、左は承認待ち、右は送信失敗と自動再送しない旨を表示した。最後に左も中止した。検証用brokerの永続監査でも対話送信・承認・取得・中止を確認し、検証後は自身が起動したprocessを終了した。これはLIVE_RUNTIMEの開発経路証拠であり、installed製品全体の証拠ではない。

操作上、下方へscrollした位置で座標指定の送信clickに反応を観測できない試行があった。入力欄からTab・Enterで送信でき、その後は上方に表示した送信buttonのmouse clickでも要求発行を確認した。座標操作と製品側のどちらが原因かは確定していない。全画面サイズ・全入力装置の成立は主張しない。初回window列挙のtimeoutは待機後の再取得で回復した。

Androidは実機検証だけを凍結し、ownerの再開指示まで端末接続要求、install、launch、結合、対話、安全保管、lifecycleの実機試験を行わない。APK/AABと既存build結果を保持する。iOSは凍結対象外。凍結を合格やrelease scopeの削除へ置き換えない。

- item: Windows全表示条件とinstalled製品証拠
  classification: release_blocker
  reason: 今回の開発画面観測は、全入力・表示条件やinstalled-path、Setup Doctor、外部監査アンカーの証拠を満たさない。scroll後の座標click不成立の原因も未確定。
  required_action: installed製品の検証時に入力位置と反応を再現確認し、既存の機械検証可能なrelease evidenceを収集する。
  blocks_release: yes

この文書更新の初回release gate検査は、解決済み項目のblocks_releaseをfalseへ変更した台帳記述を拒否した。既存contractでは分類属性をtrueのまま保持し、status=resolved / active=falseで解決を表すため、台帳だけを修正した。検査器は変更していない。


## インストール済みSetup Doctorの製品出力（2026-09-11）

既存隔離配置 `rev2-47299ce-20260910` のrelease Flutter appとRust brokerを、新規runtime領域 `runtime/setup-doctor-20260911` で実行した。app SHA-256は `e671cd41eeb1c1d9c147178f5a4ae5a53052ac375807f421c9bc31e49769f7e6`、brokerは `d2015557a120f6552aceb518fc6e30fce36c744b4b384e19bd97161548dbbf07` で、以前の隔離配置の値と一致した。

実行commandはrepository外の `python ../GUI-Shell-installed-export-check.py`。この補助は既存のbroker-serverを起動し、実測path・hashを製品出力contextへ渡し、既存の `GUI_SHELL_SETUP_DOCTOR_CONTEXT_JSON` / `GUI_SHELL_SETUP_DOCTOR_EXPORT_JSON` 経路を起動するだけで、diagnostic checkを生成・改変しない。collector自体はPythonを使用するが、起動したappのPATHはWindowsとSystem32に限定した。この環境指定だけをPython非依存の完全な証拠には扱わない。製品経路の置換やUI操作の代替ではなく、既存製品出力に限定した開発用測定である。統合collectorの正式証拠へ接続した時点で補助測定は不要になる。

製品が書き出した `product.json` はSHA-256 `903e46b4f574e1b5768eb5e32be922b780eb3a175ada777777adf445f727c501`。`tooling.windows_release_evidence.validate_setup_doctor` へそのまま渡し、windows_setup_doctor_smoke=passedを得た。製品内の10項目はすべてpassであり、設定生成・監査領域書込み・認証付きbrokerと永続化の接続を含む。出力時のapp PID4904の生存を確認し、検証終了時に自身のappとbrokerを終了した。証拠は `%LOCALAPPDATA%/GUI-Shell/installed-runs/rev2-47299ce-20260910/runtime/setup-doctor-20260911/result.json` と同directoryの製品出力である。

- item: Windows統合release evidence
  classification: release_blocker
  reason: Setup Doctor単独の製品出力は得たが、可視画面、全体provenance、監査アンカー保護を含むcanonicalなwindows_installed_smoke.jsonは未成立。製品context由来のCONFIGとbroker実観測を全体保証へ昇格しない。
  required_action: 同一隔離runに結び付く統合証拠を収集してWindows release evidence全項目を検証する。
  blocks_release: yes


## 監査アンカー収集器の誤った保護成立判定を修正（2026-09-11）

実installed storeに対し `collect_audit_anchor_proof.ps1` がpassed / key_anchor_log_same_user_rewrite_mitigated=trueを返した。しかし同じユーザーでaudit_anchor.key、audit_anchor.json、audit.jsonlのすべてをFileMode.Open / FileAccess.Writeで開けた。byteは書いていない。広範な主体へのwrite ACEがないことを、所有者自身による一括書換え防護へ誤って昇格する既存不具合だった。

収集器はrelease用診断経路に限定して修正した。DPAPI固定文字列の往復はdpapi_availableへ区別し、監査鍵保護を表すdpapi_verifiedを成立させない。外部fileの存在/hashと任意fileのAuthenticode検証も対象chain・独立保管・信頼済み署名者の結合を証明しないため、外部アンカーや署名済み監査証拠の合格へ昇格しない。現行実装に同一ユーザーの書換えを防ぐ独立境界の検証はないため、保護成立を主張せずfailedを返す。runtimeの鍵・ACL・権限は変更していない。

`python -m unittest tooling.conformance_tests.test_windows_anchor_collector` は実PowerShell collectorを3ケース（書込可能なstore、無関係な外部file、署名file）で実行しPASS。Windows conformanceの既存collector接続検査にも組み込んだ。初回はWindows PowerShell 5.1と継承module環境の不一致によりGet-FileHash等の読込みが失敗し、利用可能なPowerShell 7を優先する試験起動へ修正した。製品検査を弱めていない。

同一の実installed storeで修正後に再実行しstatus=failed、same_user_rewrite_mitigated=false、dpapi_verified=false、dpapi_available=trueを観測した。`runtime/setup-doctor-20260911/anchor-before.json` と `anchor-after.json` に比較証拠がある。旧passedはreleaseの根拠に使わない。

- item: 監査アンカーの同一ユーザー書換え防護
  classification: release_blocker
  reason: 誤判定は修正したが、独立した保管・信頼基点・chain結合の実装と証拠は未成立。
  required_action: 独立境界と巻戻し・置換・改変の検出経路を定義して実装・実測する。
  blocks_release: yes


## 旧アンカー合格記録のrelease受入れを拒否（2026-09-11）

前項の収集器修正だけでは、修正前の `anchor-before.json` をWindows release検証器へ渡すとwindows audit anchor gateがpassedとなった。実測recordを検証用provenanceへ組み込んだ検証器単位の再現であり、統合releaseの成功を示すものではない。

`validate_audit_anchor_external_tamper_evidence` は、現行形式のcollector自己申告だけでは対象chainと独立した信頼基点の結合を検証できないことを明示し、release_blockerを返すよう修正した。同じ実測recordを再投入するとfailedへ変わった。external_anchor / signed_evidenceへsource_kindを付け替えてverified=trueとする3種類のfixtureも拒否し、他のWindows gateの正常fixtureは合格を維持した。元の全項目合格fixtureはこの未検証の保証を誤って正常扱いしていたため、他の正常経路の成立とアンカーの拒否を別々にassertする試験へ修正した。

- item: 独立したアンカー証拠の受理経路
  classification: release_blocker
  reason: 現行形式に信頼済み署名者・対象chain・置換や巻戻しを検証する消費経路がない。過去のpassedも保護を証明しない。
  required_action: 独立した信頼基点と保管先の境界を決め、chainへ結合した証拠を実際に検証する経路と否定試験を実装する。
  blocks_release: yes


## owner確定方式のオフライン署名checkpoint（2026-09-11）

ownerが指定したEd25519オフライン署名方式を、Rustのrelease専用CLI・Schema・Collector・release再検証へ接続した。固定順序canonical checkpointは監査head、log/anchorのraw SHA-256、source commit、実artifact hash、時刻、sequence、前署名checkpoint hash、versionを署名対象にする。Repositoryの公開鍵fingerprintが未固定なら拒否する。通常brokerのIPC・runtimeから到達せず、秘密鍵の読取り・署名APIを追加していない。

継続性記録を証拠から自己採用しない。owner管理の最新sequence/hashを別に与え、直前署名の検証と連続性・同番号置換・後退を検査する。検証対象と継続性記録の両方を巻き戻す場合の限界、ownerの意図的再署名・物理侵害の対象外、administrator_root_resistance_claimed=falseを正本へ明記した。

Rust試験はメモリ上の使い捨て鍵を用い、正常署名、不正署名、別鍵、未固定鍵、checkpoint byte改変・非canonical・重複field、log/anchor/head/source/artifactの不一致、時刻、previous不一致、巻戻し・同sequence別署名を検証した。実fileの変更と、実Rust CLI → PowerShell Collector → Python release consumer → Rust再検証も検証し、収集後のartifact改変で拒否した。これらはFIXTUREの実実行でありowner署名証拠ではない。実秘密鍵は生成・保存・読取りしていない。

最初の編集commandでUTF-8 fileをcp932として読もうとして失敗した。書込み前の失敗で、明示UTF-8で再実行した。追加暗号依存は既存rustls依存でも使用しているring 0.17.14を直接参照しただけで、独自暗号方式を追加していない。

- item: 実ownerの公開鍵固定と署名済み証拠
  classification: release_blocker
  reason: 実装と試験用署名経路は成立したが、実運用fingerprintはnullであり、実owner署名・外部媒体の継続性記録は未取得。
  required_action: docs/OFFLINE_SIGNING_OWNER.mdのownerローカル操作を実施し、公開鍵・署名証拠だけを取得して実配置で検証する。
  blocks_release: yes

この単位の初回集約検証は配布パス検査だけがFAILした。新しい公開鍵固定fileの日本語pathが既存ASCII配布規約に合わなかったため、機械読取りpathをconfig/audit_signing_trust.jsonへ変更し、日本語意味正本と責任索引を保持した。検査allowlistは拡大していない。修正後の署名6試験（Rust CLI・Collector・release再検証を含む）はPASSし、ringで署名したcheckpointをOpenSSL標準検証でも受理することを確認した。

最終の `python tooling/validate_all.py --desktop-platform windows --include-mobile-release` は開発検証17項目すべてPASS。Rust63単体・5 IPC・6 checkpoint試験、共有6・Desktop32・Mobile14試験を含む。これは実owner署名とWindows統合release証拠の合格ではなく、owner手動操作の前で停止する。


## 実運用鍵を延期してWindows統合収集を検証（2026-09-11）

owner指示により、監査アンカーの実運用鍵・実署名は正式release直前まで外部条件待ちとする。Android実機検証の凍結も維持する。他の実装・検証は継続する。

source `7a4afd838aed44489c71d560dcd6a876b53ce9dd` をLinuxへfast-forwardし、`CARGO_TARGET_DIR=/home/mzcum/.cache/gui-shell-rev2-target python3 tooling/validate_all.py --desktop-platform linux --include-mobile-release` は18項目PASS（Rust64単体、5 IPC、5 checkpointを含む）。Windowsは `flutter build windows --release` と `cargo build --locked --release` から `rev2-7a4afd8-20260911` へ分離配置した。実broker検証とSetup Doctor製品出力10項目はPASS。Computer Useで配置先windowの概要・環境診断への遷移とscrollを観測した。

同配置で `collect_installed_smoke.ps1 -NoPythonRuntime` の `-VisibleSurfacesJson` に、その起動で製品が生成するsurface_semantics_export.jsonを指定した。UIAutomationは実行していない。旧検証器はWindows4関門をPASSとしたが、出力の生成元を確認すると `SurfaceSemanticsRegistry` はbuild時の名前を蓄積するだけで、現在の描画・可視性・破棄を観測していなかった。したがって、この初回起動PASSを完成証拠には採用しない。

収集器はこの既知のbuild registry形式をINTERNAL_STATEとして保存し、可視surface・初回起動を合格へ昇格しない。release検証器も旧collectorのpassedと、source名だけを変更した同形式を拒否する。既存の受入れ試験はこの誤った保証を正常扱いしていたため、拒否の回帰試験に置き換えた。初回の試験編集では別の不足fieldによる拒否を拾っていたため、元fixtureのfieldを保持して再実行し、修正前FAIL・修正後PASSを確認した。

実collector再実行は `runtime/registry-rejection/windows_installed_smoke.json` に保存した。first_run=failed、visible_surfacesのevidence_class=INTERNAL_STATE、formal_release_input=falseを観測。旧記録の再投入もfirst-run=failedとなり、両記録でSetup DoctorとbrokerはPASSを維持した。ログ・実配置・証拠・秘密をRepositoryへstageしていない。

- item: Windows初回起動の可視surface証拠
  classification: release_blocker
  reason: build registryによる誤受理は修正したが、現windowの個別surfaceを外部から確認する厳格な統合証拠は未成立。
  required_action: 実描画・可視性を観測する経路を初回起動と結合し、非表示・破棄・別起動の負例も検証する。
  blocks_release: yes


## 可視性を捏造しない製品診断出力（2026-09-11）

固定Flutter 3.44.0のWindows engineソース `flutter_window.cc::OnGetObject` を確認した。UIAutomation応答は `FLUTTER_ENGINE_USE_UIA` のコンパイル条件内で、MSAA応答は別経路にある。公式の背景説明は [Flutter issue 114547](https://github.com/flutter/flutter/issues/114547) にある。これは個別widgetを取得できない既観測と整合するが、配布済みDLLのコンパイル条件や外部tool側の挙動まで確認した証拠ではない。独自engineへの差替えは行っていない。

前単位で受入れ側を修正したbuild registry出力について、生成元にも残っていた架空の座標、is_offscreen=false、node ID、可視surfaceの合格を除去した。登録履歴はregistered_surfaces / registered_identifiersへ保存し、起動PID、INTERNAL_STATE、visibility_measured=false、formal_release_input=falseを明示する。旧source / diagnostic mode識別子を維持し、既存collectorとrelease検証器で診断資料として扱い、可視証拠への昇格を拒否する。通常UIの描画・Semantics識別子・権限経路は変更しない。

非表示Offstageのwidgetを実際にbuildした試験で、旧出力が必須4surfaceをすべてvisible_surfacesへ入れることを再現した。修正後は非表示時と破棄後の双方でvisible_surfaces・surface_matches・観測nodeが空となり、登録履歴だけを残す。Desktop全33試験とanalyze、Schema36/正常36/負例38、conformance148がPASS。

- item: 外部から観測したWindows個別surface
  classification: release_blocker
  reason: 診断出力の虚偽の可視性表現は除去したが、実画面の外部観測を厳格な初回起動証拠へ結合する経路は未成立。
  required_action: 固定toolchainの対応アクセシビリティ経路と利用可能な観測toolの接続を確認し、現在window・個別surface・非表示の負例を実測する。
  blocks_release: yes


Windows releaseビルド後、実broker経由でappを起動し、生成されたsurface.jsonのprocess_idが起動PID 14112と一致することを確認した。登録履歴は存在するが可視surface・match・観測nodeは空で、INTERNAL_STATE / visibility_measured=false / formal_release_input=falseを確認した。実測は `%TEMP%/gui-shell-build-registry-zaynljnf/result.json` に保存し、起動したappとbrokerは終了した。Mobile analyzeもPASS。これは製品診断出力の実行証拠であり、外部可視性の証明ではない。

このビルドでrunner exeのSHA-256は既存配置と同じ `e671cd41eeb1c1d9c147178f5a4ae5a53052ac375807f421c9bc31e49769f7e6` だった。一方、Dart AOTのdata/app.soは旧配置 `1c8776ff3a88b4af1bbaf6b9902a7231053be36da6d6c1f8eafe0f1fc0da513b` から `03590c9ea67be5fc603c1bcb32e1e7a7a5a4e3f1bcf5cf3440f786753f4af157` へ変化した。native/rust_helper/src/checkpoint.rsのmeasureはartifact引数の単一fileだけをhashし、現在のWindows collectorはapp exeを渡している。source commitの一致だけでは配置後のapp.so改変を検出しない。

- item: 監査checkpointの配置成果物hash範囲
  classification: release_blocker
  reason: 現行のexe単体hashではDart AOT、engine、plugin等の実行配布物の置換を検出できない。
  required_action: 配置app/brokerの実行配布物一式をcanonical manifest等へ結合し、欠落・追加・置換・path改変の負例を含めてcheckpointとrelease再検証へ接続する。
  blocks_release: yes


## checkpointをWindows配布物一式へ結合（2026-09-11）

checkpoint version 2へ更新し、installed_artifact_sha256をRustがinstalled rootから実測するcanonical配布物一覧のhashへ変更した。app/broker以下の全file・directoryと両launcherを含み、path・種別・size・内容hashを結合する。runtimeとinstalled_manifest.json以外のroot追加、必須成果物欠落、規約外path、symbolic link/junction/reparse point、特殊file、大小文字衝突を拒否する。上限付きで二度走査し、変化すれば失敗する。旧exe単体署名version 1は拒否する。

Schemaと正本を先に更新し、prepare / verifyの引数をinstalled rootへ変更、artifact-manifest CLIを追加した。CollectorとPython release再検証も同じrootを渡し、version 2の実検証結果のみ受理する。通常runtimeの経路や新しい依存は増やしていない。ownerの実運用鍵・署名は正式release直前まで外部条件待ちのまま。

Rustは63単体・5 IPC・8 checkpoint試験PASS。既存の署名・chain・sequence・source不一致に加え、AOT/engine/broker/launcherの改変・削除、追加file、rename、root追加、junction、規約外path、旧version拒否を確認した。実CLI → OpenSSL公開鍵検証 → PowerShell Collector → Python release consumer → Rust再検証で、runnerを変えずにAOTだけを改変すると拒否する。秘密の試験鍵はprocessメモリ内だけに生成し、実運用鍵を扱っていない。初回は試験内のJSON型代入がcompile errorとなり修正した。junction試験のmklink引数にforward slashが残っていたためInvalid switchとなり、Windows path componentの組立てを修正して全試験を再実行した。

実配置 `rev2-7a4afd8-20260911` の15 file / 21 entryをRustで測定し、Pythonで独立に列挙したpath・size・SHA-256と全件一致した。別の検証用コピーでapp.soだけを改変し、runner hashが変わらないまま成果物一覧hashが `027f460e382cf4913d701d9d605a108111db89cbef00dc1c038cdced2de72238` から `308feef71c18b8c4c3c52be4ebbebdfa2ca6b94eee9827e261cdb672a12457d7` へ変化した。実測は `%TEMP%/gui-shell-artifact-actual-tafn4o16/runtime/result.json` とbefore/after一覧へ保存した。これは未署名の実配布物測定であり、owner署名証拠ではない。

Schema36/正常36/負例38、conformance148もPASS。配布物の変更を止めた検証時点のsnapshotを扱い、原子的なfilesystem snapshotや検証後の実行時差替え防止は主張しない。正本にその境界を明記した。

- item: 実運用署名済み配布物checkpoint
  classification: release_blocker
  reason: 配布物一式を結合する実装・負例は検証したが、owner公開鍵固定と実署名は延期中。
  required_action: 正式release直前にclean commitから配置し、外部媒体上のowner鍵でversion 2を署名して現配置を再検証する。
  blocks_release: yes


## 現在commitの集約回帰とfilesystem境界試験（2026-09-11）

source `0cdbab7a13629cc6d96618160883286e7a718336` で `python tooling/validate_all.py --desktop-platform windows --include-mobile-release` の開発検証17項目、Linuxで同commandのplatform=linuxを指定した18項目が全PASSした。Windowsの正式統合証拠がRepository既定pathに存在しないことによる個別release blockerは残る。開発モードのrelease_gate: passを厳格releaseの合格と扱わない。ログはRepository外のGUI-Shell-0cdbab7-windows-validation.txt / GUI-Shell-0cdbab7-linux-validation.txtへ保存した。

配布物のpath試験にあった「UnixならCASE/caseを別entryとして作れる」という前提を除去した。実directoryを列挙して2 entryなら大小文字衝突を拒否、1 entryなら一つだけを測定し最後に書いたbyteのhashと一致することを検査する。OSによる試験除外も不要とした。production codeと拒否要件は変更しない。変更後のcheckpoint試験はWindows8件・Linux7件PASS、Schema36/正常36/負例38・conformance148もPASS。


## WindowsのSemantics起動順序を修正（2026-09-11）

DesktopのmainでOS要求より先に保持していたSemanticsHandleを除去し、Flutter標準のplatform lifecycleに有効化を任せた。独自engine、UIAutomation wrapper、依存追加はない。固定Flutter 3.44.0のbindingとWindows engineを調べると、手動handleがある場合は後続のOS要求でDart側の有効状態が変化しない。この順序が初回treeの欠落に関係するという仮説で、同一broker起動ハーネスとreleaseビルドを比較した。engine内部の通知順序をtraceしたわけではないため、詳細な因果経路は推論として保持する。

変更前のPID15840 / window3999218では外部アクセシビリティ照会にwindow枠だけが現れた。8行の起動処理を除去した比較版PID6884 / window1770286では、初回に108要素を観測し、概要、ナビゲーション、実行系状態、不変条件の個別groupと内容を取得できた。診断へのクリック後は環境診断、broker IPC、保護項目拒否を含むtreeへ切り替わり、概要へ戻ると実行系・不変条件の内容が再出現した。再描画で要素番号が失効したクリック2回は未成立として扱い、画面を再取得して座標操作後に遷移完了を再観測した。先行文書のUIAコンパイル条件は今回の原因を確定する証拠ではなく、この標準engineでも個別widgetを取得できることが新たな観測である。

観測はComputer Useの外部treeであり、製品build registryから合成していない。Repository外のGUI-Shell-semantics-external-observation.jsonに診断と概要復帰の実treeを保存した。ハーネスの診断出力は引き続きINTERNAL_STATE、可視surface空、formal_release_input=falseを確認した。署名鍵は使用していない。

`flutter build windows --release` とDesktopの `flutter test --reporter compact`（33件）、Desktop/Mobileの `flutter analyze`、Schema36/正常36/負例38、conformance148はPASS。厳格なinstalled collectorの再収集をこの開発用起動で代替しない。

- item: 現行配置からの厳格なWindows可視surface証拠
  classification: release_blocker
  reason: 外部tree取得と画面切替を実測したが、clean sourceと分離配置に結合した正式collectorの証拠は未成立。
  required_action: 現行commitを分離配置し、個別surfaceの実観測と非表示の負例を厳格な収集・検証経路で確認する。
  blocks_release: yes


## 可視surfaceの宣言と実観測の結合（2026-09-11）

clean source 53182cd4cab77e025c61913357cf527270f4dd11をrev2-53182cd-20260911へ分離配置した。Windows releaseビルドとRust releaseビルド、配置先broker smokeは成功し、正式consumerではSetup Doctorとbrokerの2関門がPASS。初回起動と可視証拠provenanceはFAIL、実運用署名は延期によるFAILを維持した。実証拠とvalidation-result.jsonは配置先runtime/evidenceに保存している。

標準MSAA APIの実測ではwidgetの名前・役割・座標を取得できたが、画面外要素も非表示bitなしで返った。既存collectorは名前だけで候補を選び、consumerはmatchを観測treeへ結合せず、診断keyも先頭20件しか調べていなかった。正常fixtureも4surface宣言に対しrootと1surfaceしか記録していなかった。今回の単位はbuild/release経路のこの誤受理を修正する。

正本を更新し、全観測要素の識別子・root・親子edge・件数を検証する。matchは同じelement_keyの実観測属性と一致し、root/containerでないことを要求する。要素からrootまで明示的なis_offscreen=falseと有限・正の矩形を要求し、共通領域が正面積の候補だけを可視とする。collectorも同じ幾何条件で候補を選び、状態取得失敗はnullとして保持する。親列挙をRawViewWalkerへ揃えた。

17負例は変更前すべて誤受理、変更後すべて拒否した。過大整数座標の例外化も拒否する試験を追加し、負例は計18件。画面外、親領域外、非表示、状態・座標欠落、ゼロ面積、非有限座標、親欠落・循環、識別子重複、match差替え、未観測match、件数・edge不一致、container流用を含む。実collectorの純粋判定関数も正常と11負例で試験した。Windows PowerShell 5.1ではUTF-8 BOMなしの日本語scriptを既定encodingで読むと構文エラーになるため、試験入口とAST読込みでUTF-8を明示した。production用wrapperや新依存は追加していない。Schema36/正常36/負例38、conformance149件、日本語基底監査はPASS。

これは収集済みtree上の表示領域交差の検証であり、別windowによる遮蔽やpixel内容の証明ではない。MSAA実測を正式collectorへ接続する作業は、この条件を省略せず続行する。

- item: MSAA実測と正式Windows収集経路の接続
  classification: release_blocker
  reason: UIAutomationの現経路ではwindow枠しか取得できず、MSAAは診断実測のみ。今回の可視性負例はFIXTUREであり、正式installed初回起動の合格ではない。
  required_action: MSAAの要素同一性・親子関係・実矩形と日本語surfaceを正式収集へ結合し、実画面と負例を再検証する。
  blocks_release: yes


## 日本語surfaceを既存UIAutomation経路で実測（2026-09-11）

前単位ではMSAA接続が必要と判断したが、新しい診断runでは日本語label対応を追加した既存UIAutomation経路だけで108要素を取得した。別起動とMSAA追加コード除去後も計3 runで必須4surfaceと観測tree・矩形交差の検証が通った。前のwindow枠2要素との時間・観測経路による差の詳細は未確定であり、日本語対応がOS側tree公開自体を変えたとは主張しない。MSAAの独自collectorは未使用で、必要性を実証できないため採用せず除去した。新言語・依存・実行経路は追加しない。

修正はcollectorとconsumerの意味対応に限る。概要、ナビゲーション、実行系状態、不変条件状態、および既存Dart安定identifierを固定し、日本語名は空白正規化後の完全一致または見出し二重連結だけを許可する。元の観測名、UIA element key、runtime ID、親子関係、座標を保持する。説明文やTab名、identifierの余分なsuffixは拒否する。

診断実測はrev2-53182cd-20260911/runtime内のmsaa-check、uia-japanese-repeat、uia-only-finalへ保存した。すべてsource=uiautomationであり、製品registryでもMSAAの合成出力でもない。診断専用のため正式初回起動の合格と扱わない。Schema36/正常36/負例38、conformance150件はPASS。MSAA接続そのものを残作業とした直前の判断は撤回し、現行commitのcleanな分離配置からの正式再収集を次の境界とする。

- item: 日本語surface対応後の正式Windows再収集
  classification: release_blocker
  reason: 現行collectorの個別surface実測は成功したが、上記は変更中の診断専用runであり、現行clean commitからの正式統合runは別に必要。
  required_action: commit後に分離配置して同collectorを通常モードで実行し、署名を除く各関門の実結果を確認する。
  blocks_release: yes
