# Windows インストール先

GUI-Shell v1.0 は Windows を優先する。インストール先は、broker 介在の runtime 経路を通じて Flutter UI を起動しなければならない。

```text
GUI-Shell.brokered.cmd
  -> GUI-Shell.brokered.ps1
  -> gui_shell_rust_helper.exe broker-server
  -> gui_shell_desktop.exe with GUI_SHELL_BROKER_ENDPOINT_JSON
```

`stage_installed_app.ps1` は、build 済みの Flutter Windows release directory と Windows 用 `gui_shell_rust_helper.exe` から、実行ごとに固有の staged install directory を作成する。既定の install root は `%LOCALAPPDATA%\GUI-Shell\installed-runs\<run_id>` である。生成 manifest は source commit、作業ツリーの clean 状態、artifact hash、分離した runtime/config/audit/store path を記録する。

最終的なインストール証拠を収集する前に `collect_broker_smoke.ps1` を使う。これは認証済み broker IPC、`127.0.0.1` へ制限された bind、永続 store の準備状態、broker 再起動後の replay 拒否、crash 時の fail-closed 接続挙動を検証する。No-Python/no-FFI 値は broker runtime 証拠ではなく、非正式な静的宣言としてのみ記録する。

`collect_setup_doctor.ps1` は外部 installer/config/broker 確認にのみ使い、正式な製品証拠として扱わない。`collect_installed_smoke.ps1` は、インストール済み Rust broker を起動し、`GUI_SHELL_BROKER_ENDPOINT_JSON` を与えてインストール済み Flutter `.exe` を起動する。また、`GUI_SHELL_SETUP_DOCTOR_EXPORT_JSON` でインストール済み app の環境診断製品出力を要求し、起動証拠用に `-NoPythonRuntime` PATH 無効化を適用する。`-VisibleSurfacesJson` が指定されない場合は UIAutomation の可視表層証拠と診断 tree 射影を取得し、app 初回起動証拠、app 生成の環境診断証拠、可視表層証拠、broker 証拠、由来、field 由来を `release_evidence/windows_installed_smoke.json` へ統合する。

通常の利用者に、terminal、WSL、npm、Git、port設定、runtime root 検出の手動操作を必須にしてはならない。staged `.cmd` launcher は、署名済み installer/MSIX wrapper を追加するまでの局所的な packaging 工程である。

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

- item: Windows可視surfaceの実観測
  classification: release_blocker
  reason: Flutter build registryだけでは可視性を証明できず、現環境の外部accessibility観測では個別widgetを取得できていない。
  required_action: 実配置の現在windowから個別surfaceの可視性を取得し、初回起動と由来の検証へ接続する。Computer Useの画面観測だけを既存collectorの厳格な機械証拠へ付け替えない。
  blocks_release: yes
