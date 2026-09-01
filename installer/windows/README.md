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
