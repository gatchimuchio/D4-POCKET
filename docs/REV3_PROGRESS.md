# D4 Pocket / GUI-Shell 統合rev3 進捗

本書は、受領した統合仕様書rev3・開発工程表rev3の工程状態を履歴追加型で記録する。rev1／rev2の記録は書き換えず、旧工程のPASSをrev3の機能完成証拠として再利用しない。現行のrelease gateは既存`release_blockers.registry.json`が管理し、本書の検証記録だけで解除しない。

## R0 現行状態再固定（2026-09-29）

### Repository基準

- 対象: `gatchimuchio/GUI-Shell`
- 基準branch: `main`
- 検証対象commit: `ec9b11a1330c3626fe7e7c25ef5c4068241b1d28`
- 基準時の`main`, `origin/main`, remote `refs/heads/main`: 同一commit。
- 基準時のworking tree: clean。
- 基準commitは既存rev2作業の現行HEADであり、rev3機能の完成を意味しない。

### 再測定値

- Schema: 149件。
- 正常example: 149件。
- Negative fixture: 192件。
- 適合確認: 225件。
- Rust全target: Windows Server 2025／Rust 1.95.0の手動Actionsで12 test target、391 passed、0 failed、0 ignored。
- Flutter Desktop: Windows手動Actionsでanalyzeに問題なし、125 tests passed。
- Flutter Mobile: 現行workspaceで21 tests passed。Windows手動ActionsではMobile analyzeに問題なし。
- `release_evidence/`: `.gitkeep`のみ。`release_evidence/windows_installed_smoke.json`は存在しない。
- blocker registry: 16項目中、active unresolved 14項目、resolved inactive 2項目。全体`release_ready=false`。

### 実行した検証

- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`: exit 0。登録済み10 development checksがすべてpassed。日本語strict監査はrepository file 1110件・findings 0、Schema／Conformance／manifest／release gate／packaging portability／release smoke／evidence bundle／runtime assertions／既存C32開発監査を含む。evidence bundleはdevelopment evidenceであり、release readinessや製品実行を証明しない。
- `python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`: 225件合格。先行実行でOneDrive ReparsePoint読取時に`OSError: [Errno 22]`が出たが、再実行では再現しなかった。UTF-8 modeが障害原因を解消したとは判定していない。
- `cargo +1.95.0 check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`: 終了code 0。
- `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`: ローカルではWindows Application Controlがtest executableの起動前にOS error 4551で拒否。assertion failureではない。保護設定を変更せず、拒否fileの移動やtest除外も行っていない。
- 作業directory `apps/mobile_flutter/`で`flutter test --no-pub --reporter expanded`: 21件合格。
- Desktopの同形式Flutter testはOneDrive workspace内`build/unit_test_assets`の削除拒否で開始前に失敗。対象directoryにReparsePoint属性と削除拒否ACLを観測し、ACLは変更していない。
- Desktop／Mobileのローカル`flutter analyze --no-pub`はanalysis serverからの不正なLSP JSONで異常終了。これはanalyze成功として扱わない。

### 手動Windows Actions証拠

いずれも`workflow_dispatch`で、対象は`main`の上記正確なcommit。automatic CI、PR、branchは作成していない。

- [Windows Rust validation #19](https://github.com/gatchimuchio/GUI-Shell/actions/runs/36512064184): Windows Server 2025／Rust 1.95.0。checkout SHA一致、workflow対象Rust fileのrustfmt、全target `cargo check`／`cargo test`、試験後cleanが成功。12 test target、391 passed／0 failed／0 ignored。
- [Windows Desktop Flutter validation #2](https://github.com/gatchimuchio/GUI-Shell/actions/runs/36512206643): Windows Server 2025。Rust helper build、Desktop `flutter analyze`、Desktop `flutter test`（125件）、Mobile `flutter analyze`、試験後cleanが成功。両analyzeは`No issues found!`。
- 両runともActions artifactなし。Hosted runnerの成功は、OneDrive workspaceのACLやApplication Controlを解消したこと、installed product／実Agent／release readinessを証明しない。

### R0時点のblocker inventory

次のactive unresolved項目はR1で原因を再監査・分類する。ここでは従来の互換`classification: release_blocker`とstatusを保持し、まだ分類変更していない。

- `windows_evidence_provenance_isolation`
- `windows_installer_first_run_smoke`
- `windows_setup_doctor_smoke`
- `windows_broker_installed_smoke`
- `audit_anchor_external_tamper_evidence_proof`
- `owner_go`
- `rev2_mobile_device_evidence`
- `rev2_mobile_distribution`
- `comprehensive_extension_rev1_completion`
- `rev2_desktop_product_distribution`
- `rev2_flutter_broker_channel_boundary`
- `rev2_export_owner_ui_authority_path`
- `rev2_module_pruning_binary_and_measurement`
- `rev2_mobile_flutter_native_device_link_boundary`

resolved inactiveの`rev2_desktop_launch_regression`と`windows_rust_integration_test_execution_policy`は履歴として保持する。Windows ActionsによるRust test実行確認は、実Agent Task、production Broker経路、installed product、正式配布、release readinessを証明しない。

## 次工程

R1では全blockerへrev3原因分類を加え、Owner待ちと記述された項目を一件ずつ再審査する。Codexが実施できるroutine検証・技術作業はOwner専任扱いにしない。既存release gate互換fieldと既存履歴は維持する。
