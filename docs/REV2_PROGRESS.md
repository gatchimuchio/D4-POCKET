# rev2 実装進捗と証拠境界

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
