# 互換性対応表

## C31現行互換性要約（2026-09-24）

現行の開発検証対象はWindows host上のRust Broker、共有／Desktop／Mobile Flutter、Schema、Conformance、C27性能smoke、C28短時間運用smoke、C29障害注入smoke、C30回帰matrixである。Schema 108件、正常example 108件、negative fixture 129件、Conformance 179件、厳格日本語監査、manifest検査はPASSしている。

| 領域 | 現行成立範囲 | 未成立・分類 |
| --- | --- | --- |
| Windows開発環境 | Rust／Flutterの解析・試験、debug build、local Broker／fixture検証 | installed-pathのprovenance、first-run、Setup Doctor、Broker、Audit anchor外部改変証拠は`release_blocker`（aggregate_of=windows_evidence_provenance_isolation,windows_installer_first_run_smoke,windows_setup_doctor_smoke,windows_broker_installed_smoke,audit_anchor_external_tamper_evidence_proof） |
| モバイル | 9画面、Device Link projection、contract／失効／背景遷移fixture、Android debug APK／AAB | 実端末、native secure storage、TLS実接続、Windows installed連携、正式署名・配布は`release_blocker`（aggregate_of=rev2_mobile_device_evidence,rev2_mobile_distribution） |
| 外部Runtime／Agent／MCP／A2A | metadata／contract／loopbackまたはfixtureのbounded検証。C9はWindowsでnative Owner確認後のstdio接続、対象限定Credential注入、操作者向け一回Tool呼出しまでを実装対象とする | 実外部Server適合、AgentへのTool結果handoff、installed product上のCredential利用、複数Agent比較、A2A実接続・handoffは`release_blocker`（registry_id=comprehensive_extension_rev1_completion） |
| 長時間・障害・性能 | C27、C29、C30のlocal検証とC28の短時間smoke | C28の8時間実測、installed product負荷、外部サービス負荷は`release_blocker`（registry_id=comprehensive_extension_rev1_completion） |
| 非Windows platform | 文書・一部補助build／過去の補助結果を証拠範囲付きで保持 | 現行commitの実機・正式配布・Mac hostの総合証拠は`known_limitation`または`release_blocker`（aggregate_of=rev2_mobile_device_evidence,comprehensive_extension_rev1_completion） |

開発検証のPASSは互換性の全面保証、installed productのrelease proof、owner GOを意味しない。過去のrunはそのsource commitに結合した履歴として扱い、現行commitの証拠へ読み替えない。

## rev2開始時の証拠面（履歴）

以下の旧Desktop v1.0記録は過去の基準面を含む。現行owner rev2ではMobile・端末連携・Android・Apple補助buildも作業対象であり、旧post_v1_scopeを理由に未完了を除外しない。現在の要求別監査は `docs/REV2_PROGRESS.md`、Mobile状態は `MOBILE_STATUS.md` を参照する。

macOS projectは追加済みで、手動補助run 34446194013（commit 27b8713fd1a9ecdb81abe1d4225b99b26bda84ba）でmacOS開発app・iOS Simulator appとMac Rust64単体・5統合がPASSした。「macOS validation environmentがない」という旧記録は、この補助buildの取得前を指す。実機での起動・安全保管・対話は未確認でありrelease_blockerである。実機の製品supportや完成を主張しない。

Android開発APK/AABはbuild済み。実機install以降、Windowsの中断された画面回帰、installed-path証拠、正式配布、owner GOはrelease_blockerとして保持する。開発検査のPASSはこれらのgateを解消しない。


| 構成要素 | 状態 | 分類 | リリース遮断 | 理由 | 必須対応 |
| --- | --- | --- | --- | --- | --- |
| Python Shell Core | passed | none | no | `schema_check`と`conformance_skeleton`は現行環境で合格した。 | リリース前にこれらの検査を引き続き実行する。 |
| Rust Helper（Rustヘルパー） | passed | none | no | Rust/Cargoが存在し、`cd native/rust_helper && cargo test`は合格した。 | 将来のshellで`$HOME/.cargo/bin`がPATHに含まれることを確認し、リリース前にcargo testを再実行する。 |
| Desktop Flutterの静的解析 | passed | none | no | Flutter SDKは`$HOME/dev/flutter`に存在し、`unzip`は`/usr/bin/unzip`に存在する。`flutter --version`と`cd apps/desktop_flutter && flutter analyze`は2026-05-25に合格した。 | v1.0検証にデスクトップ静的解析を維持する。 |
| Desktop FlutterのLinuxツールチェーン | passed | none | no | `flutter doctor -v`はLinuxデスクトップツールチェーンとしてclang 21.1.8、cmake 4.2.3、ninja 1.13.2、pkg-config 2.5.1が存在すると報告する。 | リリース候補のためにLinuxデスクトップビルド依存関係をインストール済みに保つ。 |
| Desktop Flutterのwidget test | passed | none | no | 複数の`Dashboard` labelに合わせてsmoke testを更新した後、`cd apps/desktop_flutter && flutter test`は2026-05-25に合格した。 | desktop validationにwidget smokeを維持する。 |
| Desktop FlutterのLinuxビルド | passed | none | no | `cd apps/desktop_flutter && flutter build linux`は2026-05-25に合格し、`build/linux/x64/release/bundle/gui_shell_desktop`を生成した。 | v1.0検証にLinuxデスクトップbuild smokeを維持する。 |
| Desktop FlutterのLinux起動 | passed | none | no | `./build/linux/x64/release/bundle/gui_shell_desktop`は2026-05-25にWSLg上で正常に起動した。最初のwindowが開き、Dashboard、NavigationRail、Runtime Status、Invariant Statusを視認した。 | v1.0検証にLinuxデスクトップlaunch smokeを維持する。 |
| Linux最終製品証明 | not_sufficient | known_limitation | no | Linuxは検証済みのdevelopment/verification sliceであり、それだけではWindows優先のfinal product proofにならない。 | product release claimの前にWindows release gateを完了する。 |
| WSLgのグラフィックス警告 | observed | known_limitation | no | WSLg上で端末にlibEGL/MESA警告が出たが、app windowの起動やfirst screen evidenceの描画を妨げなかった。 | 描画または安定性が失敗しない限り、非遮断として扱う。 |
| Desktop FlutterのWindowsプロジェクト | generated | none | no | `flutter create --platforms=windows .`は既存の`lib/` app codeを変更せず、`apps/desktop_flutter/windows`を生成した。 | Windows Flutterデスクトッププロジェクトファイルをversion control下に保つ。 |
| Desktop FlutterのWindowsツールチェーン | historical_owner_trial | none | no | native WindowsのFlutterデスクトップanalyze、test、build、launch smokeは過去に合格した。これはowner-trial historyであり、現行strict R2 formal evidenceではない。 | isolated run provenanceを伴う現行native Windows release-candidate evidenceを再収集する。 |
| Desktop FlutterのWindows静的解析 | historical_owner_trial | none | no | Windows Flutter analyzeは過去にnative Windowsホストで合格した。 | Windows release candidateで`flutter analyze`を合格状態に保ち、現行proofをisolated evidence runへ結び付ける。 |
| Desktop FlutterのWindowsテスト | historical_owner_trial | none | no | Windows Flutter testは過去にnative Windowsホストで合格した。 | Windows release candidateで`flutter test`を合格状態に保ち、現行proofをisolated evidence runへ結び付ける。 |
| Desktop FlutterのWindowsビルド | historical_owner_trial | none | no | `flutter build windows`は過去に合格し、`build\windows\x64\runner\Release\gui_shell_desktop.exe`を生成した。 | 正確なrelease candidateを再ビルドし、staged manifestへartifact hashを記録する。 |
| Desktop FlutterのWindows起動 | historical_owner_trial | none | no | native Windows launch smokeは過去に合格したが、旧aggregate-surface evidenceおよびmissing-provenance evidenceはstrict R2 proofには無効である。 | isolated installed runからsurfaceごとのUIAutomation/accessibility evidenceを再収集する。 |
| Windows Setup Doctor（Windows診断） | implementation_gap | release_blocker | yes | registry_id=windows_setup_doctor_smoke 現行Rust起動器はcollector注入のexport環境変数をFlutter childから除去し、正式なBroker統治product export経路は未接続。external probe evidenceはproduct proofとして拒否する。 | Broker統治されたproduct export contractと通常起動経路を接続し、native Windows installed runから収集して`python tooling\windows_release_evidence.py`を通す。 |
| Windowsのインストーラー・初回実行 | evidence_missing | release_blocker | yes | registry_id=windows_installer_first_run_smoke aggregate_of=windows_evidence_provenance_isolation,windows_broker_installed_smoke 起動器基準collectorは実装したが、clean-sourceのisolated Windows evidenceがない。collectorは通常起動時のhealth受理Auditを記録できるが、client応答受信・呼出し元PIDの証拠ではない。初回config生成も未成立。 | stageとは異なるWindows user profileからRust Desktop起動器を実行してruntime／lifecycle／health受理Auditを計測し、初回config生成と合わせて`installer\windows\collect_installed_smoke.ps1`と`python tooling\windows_release_evidence.py`を通す。 |
| Desktop FlutterのmacOSプロジェクト | unverified_planned | known_limitation | no | 現在macOS validation environmentを利用できず、GUI-Shell v1.0はverified macOS supportを主張しない。 | macOS supportを主張する前にmacOSホストで検証する。 |
| Desktop FlutterのmacOSツールチェーン | unverified_planned | known_limitation | no | 現在macOS validation environmentを利用できない。 | supportを主張する前にmacOS上でmacOS Flutter toolchainを検証する。 |
| Desktop FlutterのmacOSビルド | unverified_planned | known_limitation | no | macOS validation environmentを利用できないため、macOS build smokeは未実行である。 | supportを主張する前にmacOSホストで`flutter build macos`を通す。 |
| Desktop FlutterのmacOS起動 | unverified_planned | known_limitation | no | macOS validation environmentを利用できないため、macOS launch smoke evidenceは未記録である。 | supportを主張する前にmacOS artifactを起動してevidenceを記録する。 |
| macOSのpackaging・notarization | unverified_planned | known_limitation | no | macOS packaging/notarizationは移植検証の予定に留まる。 | macOS上で文書化し、検証する。 |
| macOSのインストーラー・初回実行 | unverified_planned | known_limitation | no | macOS installer/first-run smokeはWindows優先v1.0 release gateに含まれない。 | macOSホスト上で検証する。 |
| OS固有のSetup Doctor | implementation_gap | release_blocker | yes | registry_id=windows_setup_doctor_smoke Windows優先product targetに正式なBroker統治Setup Doctor export経路が未接続である。 | Contract・production path接続後にWindows固有product evidenceを収集し、validatorで確認する。 |
| 実装リリース・スモーク | passed | none | no | `python3 tooling/release_smoke.py`はShell Coreのpersistence・audit・approval・recovery smoke、first-run/Setup Doctorのnon-authority smoke、Runtime Catalogのsmoke、およびAgent Runtimeのreference smokeを通す。 | implementation smokeを合格状態に保つ。native Windows installed-path installer/Setup Doctor smokeは別に扱う。 |
| Mobile Flutter（モバイルUI） | passed_optional | post_v1_scope | no | Mobileの`flutter analyze`は現在合格するが、所有者がmobileを明示的に含めない限り、mobile full releaseはdesktop-first v1.0の範囲外である。 | 所有者がrelease scopeを変更しない限り、v1.0の対応は不要である。 |
| BLUE-TANUKI Adapter（アダプター） | mock_checked | post_v1_scope | no | BLUE-TANUKI product completionはGUI-Shell v1.0 gateの範囲外であり、現行validationはmock/contract orientedである。 | 所有者が範囲を変更しない限り、v1.0の対応は不要である。 |
| Installer / Setup Doctor（導入・診断） | development_checked | release_blocker | yes | aggregate_of=windows_installer_first_run_smoke,windows_setup_doctor_smoke 起動器基準collectorはBrokerのhealth受理Auditを記録するが、初回config生成・Broker統治Setup Doctor exportのproduction pathとnative evidenceが未成立。health Audit単体はclient応答受信・PID帰属を証明しない。 | 初回config生成とBroker統治Setup Doctor product exportを接続した後、分離profileのinstalled smokeで検証する。 |
| Runtime Catalog（ランタイム一覧） | passed | required_for_v1 | no | runtimeおよびadapter manifestはproduction RuntimeCatalogを通して登録され、authorityは非付与のままである。 | Runtime Catalog smokeを合格状態に保つ。 |
| Agent Runtime（エージェント実行環境） | passed | required_for_v1 | no | Agent Runtime reference smokeはworkspace boundary、secret path denial、shell permission mapping、およびauditable diff behaviorを検証する。 | Agent Runtime Contract smokeを合格状態に保つ。 |
