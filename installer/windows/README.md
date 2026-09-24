# Windows staged配置とrelease evidence

`stage_installed_app.ps1`はDeveloper／release-evidence用のstaging toolであり、利用者向けinstallerではない。Flutter Windows Release一式、Rust Broker helper、`gui_shell_desktop_launcher.exe`を新しい配置rootへ複製する。標準GUI entry pointはRust起動器であり、利用者へterminal、PowerShell、Python、Rust、Flutter、Gitの起動を要求しない。

```text
gui_shell_desktop_launcher.exe
  -> 同一Rust process内の管理Broker
  -> 通常認証Broker endpointを使うgui_shell_desktop.exe
```

起動器はBroker authorityをRust側に保ち、認証済みloopback IPCだけを使う。durable Broker stateは`%LOCALAPPDATA%\GUI-Shell\broker\desktop\store`へ保存し、通常終了時は起動時とbyteが一致する一時endpointだけを削除する。Owner資格を作成しない。これはstaged起動／Broker lifecycleの限定機能であり、正式installer、uninstaller、signed update、rollback、formal package identity、Download→Installを実装したことを意味しない。

## Buildとstaging

Windows開発hostで実行する。

```powershell
flutter build windows --release --no-pub
cargo build --release --locked --manifest-path native\rust_helper\Cargo.toml --bin gui_shell_rust_helper --bin gui_shell_desktop_launcher
.\installer\windows\stage_installed_app.ps1 `
  -FlutterReleaseDir .\apps\desktop_flutter\build\windows\x64\runner\Release `
  -BrokerHelperExe .\native\rust_helper\target\release\gui_shell_rust_helper.exe `
  -DesktopLauncherExe .\native\rust_helper\target\release\gui_shell_desktop_launcher.exe
```

staged manifestの`launcher_runtime`は起動器が使う`%LOCALAPPDATA%\GUI-Shell\broker\desktop`を示す`CONFIG`宣言で、per-user scope、`isolated=false`、`formal_runtime_proof=false`を明記する。manifestの`runtime_dir`／`store_dir`／`config_dir`／`audit_dir`は証拠collector用の分離scratch pathであり、標準起動器の保存先ではない。この差をWindows installed evidenceで解消せず、二つのpathを同じruntimeとして報告してはならない。

`collect_broker_smoke.ps1`と`collect_installed_smoke.ps1`は別個のevidence collectorである。現行`collect_installed_smoke.ps1`は診断出力収集のためFlutter executableを直接起動しており、標準Rust起動器の経路を実証しない。collectorを起動器経由へ接続し、分離Windows環境からclean sourceの証拠を採るまで該当release gateを維持する。

`collect_broker_smoke.ps1`は認証IPC、`127.0.0.1`限定bind、`credential_role=normal`、永続store準備、Broker restart後のreplay拒否、crash時のfail-closedを検証する。これはBroker単体のLIVE_RUNTIME証拠であり、Desktop起動器、installed product、正式releaseを証明しない。No-Python／no-FFI値は非正式なstatic declarationに限る。

`collect_setup_doctor.ps1`は外部installer／config／Broker確認だけを行い、正式な製品証拠として扱わない。`collect_installed_smoke.ps1`もこの外部probeを証拠へ格納するだけで、Flutterへ診断出力や設定生成を要求しない。`-NoPythonRuntime`は対象PATHを無効化する。正式な初回起動証拠へは、Rust起動器の利用、実画面、Broker統治された製品Setup Doctor、初回設定、監査、field provenanceを接続する必要がある。

## 監査アンカー収集器の証拠範囲

`collect_audit_anchor_proof.ps1` のACL検査は広範な主体への書込ACEを調べ、`dpapi_available` は固定文字列でOSの往復機能を調べる。いずれも同一ユーザーによる監査鍵・アンカー・ログの一括書換えを防ぐ証拠ではない。任意外部fileの存在/hash、任意fileのAuthenticode署名も、対象chain・独立保管・信頼済み署名者との結合を証明しない。この範囲だけでは保護成立を報告せずfailedを返す。owner指定のオフラインEd25519 checkpointは別の実検証経路で扱う。過去のpassedを独立した保護証拠として使用しない。

- item: 監査アンカーの独立した保護境界
  classification: release_blocker
  reason: オフライン署名checkpointの実検証経路は実装したが、owner公開鍵固定・実署名・独立した最新継続性記録は未取得。
  required_action: ownerが外部媒体で鍵生成・署名し、公開物だけを渡して実installed証拠を検証する。
  blocks_release: yes

Windows回帰試験: `python -m unittest tooling.conformance_tests.test_windows_anchor_collector`。書込可能な検証用storeと無関係な外部file・署名fileを実collectorへ渡し、不正な合格と内容変更がないことを確認する。

release検証器も旧形式のアンカー保護宣言だけでは受理しない。旧collectorのpassed、external_anchor / signed_evidenceへのsource_kind変更、verified=trueの指定だけでは解除しない。アンカー以外の独立した検証結果は維持する。受理を有効化するには、対象chainと独立した信頼基点・保管先を実際に検証するcontractと消費経路が必要である。

## オフラインEd25519署名checkpoint

正本は `docs/specs/audit-checkpoint.md`。Collectorへ `-CheckpointBundle <directory> -TrustedHeadPath <owner公開継続性記録> -PreviousCheckpointBundle <直前bundleまたは->` を指定する。停止したbroker store、現在commitと一致するclean sourceから配置したappを対象とする。公開鍵fingerprintはRepositoryの `config/audit_signing_trust.json` に固定する。未設定は拒否する。

release再検証は `GUI_SHELL_AUDIT_TRUSTED_HEAD` にowner管理の公開継続性記録のpathを指定する。これは秘密鍵ではない。証拠JSONが指定する過去floorを採用せず、現在のfileとRepository固定公開鍵をRustで再検証する。`cargo build --locked --manifest-path native/rust_helper/Cargo.toml` で現在の検証器を構築してから実行する。外部継続性記録をlocal証拠と一緒に巻き戻さない。

署名bundleはcheckpoint.json、signature.bin、public-key.derで構成し、Collector出力にfingerprint、署名済みhash、previous hash、sequence、verification resultを保存する。秘密鍵は扱わない。ownerの手動コマンドは `docs/OFFLINE_SIGNING_OWNER.md` に記載する。

## 可視画面証拠の限界

`flutter_semantics_runtime_export` はwidgetのbuild時に登録した名前一覧であり、非表示・破棄済みwidget、画面外、実際の描画を区別しない。`is_offscreen=false` も測定値ではない。収集器はこれを `INTERNAL_STATE` の診断資料として保持するが、可視surface・初回起動の合格へ昇格しない。release検証器は旧collectorのpassed、およびsourceだけをaccessibility_tree等へ変更した既知のbuild registry形式も拒否する。

- item: Windows visible surfaceの実観測
  classification: release_blocker
  reason: Flutter build registryだけでは可視性を証明できず、現環境の外部accessibility観測では個別widgetを取得できていない。
  required_action: 実配置の現在windowから個別surfaceの可視性を取得し、初回起動と由来の検証へ接続する。Computer Useの画面観測だけを既存collectorの厳格な機械証拠へ付け替えない。
  blocks_release: yes

製品側のbuild registry診断出力も `evidence_class=INTERNAL_STATE`、`visibility_measured=false`、`formal_release_input=false` を明記する。登録履歴は `registered_surfaces` と `registered_identifiers` に残すが、`visible_surfaces`・`surface_matches`・観測nodeは空とし、座標・非表示判定・実Semantics node IDを捏造しない。非表示でbuildされた要素や破棄済みの登録が残り得るため、現在画面の代用にしない。

署名checkpoint version 2のinstalled_artifact_sha256はexe単体ではなく、app/broker directory全体とrootの`gui_shell_desktop_launcher.exe`を含むcanonical一覧を結合する。Rustの `監査チェックポイント artifact-manifest <installed root>` で署名前の一覧を確認できる。prepare / verifyへ渡す成果物引数もinstalled rootとする。旧version 1のexe単体署名は受理しない。実運用鍵はowner指示に従い正式release直前まで外部条件待ちとする。
