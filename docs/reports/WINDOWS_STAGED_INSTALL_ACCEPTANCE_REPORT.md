# Windows段階導入受入報告

## 0. 現行R2証拠分類

~~~yaml
- item: this historical report
  classification: release_blocker
  reason: この報告は履歴上のowner-trial evidenceとしてだけ保存する。旧runはaggregate native surface exposureを使用し、isolated run provenance、正確なsource/artifact/evidence bundle linkageを欠き、external Setup Doctor probe outputをformal evidenceとして扱ったため、現行strict R2 formal proofには無効である。
  required_action: completed product releaseを主張する前に、strict R2 isolated installed-path flowを通してWindows evidenceを再収集する。
  blocks_release: yes
~~~

## 1. 結論

- 結果: `historical PASS, invalid for current strict R2 formal proof`
- 検証した実装コミット: `3e7d6079eaeb3af0dd1cd0fc38c84ba095dad9f7`
- ブランチ: `main`
- 日時: `2026-06-03T14:24:01.8473316+09:00`
- 実行環境: native Windows PowerShell 7.6.1、Microsoft Windows 10.0.26200
- 範囲: Windows段階導入の受入だけ
- 検証コマンド: `python tooling\windows_release_evidence.py`
- 検証終了コード: `0`

この結果が証明するのは、検証対象コミットについて、旧validatorの下で履歴上の段階的Windows導入証拠ゲートが合格したことだけである。現行R2の正式リリース証拠としては、すでに無効である。

## 2. 修正した原因分類

| 原因 | 分類 | 修正 |
|---|---|---|
| ランタイム表明の証拠不一致 | 証拠形式の不具合 | `tooling\release_runtime_assertions.py --json`の出力を、インストール済みスモーク収集器へ渡す証拠ファイルとした。`--check`は別のゲートとして維持した。 |
| 証拠ファイル内のPowerShell JSON BOM | 収集器の直列化不具合 | Windows収集器は.NETの`UTF8Encoding(false)`を用いて機械証拠JSONを書き出すため、Pythonの`encoding=\"utf-8\"`検証で解析できる。 |
| Flutter UIAutomation操作面の可視性欠落 | Windowsアクセシビリティ公開の不具合 | Flutterのsemantic labelを追加・有効化し、Windows runnerはfirst-run visible surface labelをnative FlutterView accessible name経由で公開するため、UIAutomationで記録できる。 |

検証条件、権限境界、ブローカー設計は弱めていない。

## 3. 実行概要

| 手順 | コマンド・対象 | 結果 | 証拠 |
|---|---|---|---|
| Rustリリースビルド | `native\rust_helper`内で`cargo build --release` | PASS | release profileの完了。 |
| Flutter Windowsリリースビルド | `apps\desktop_flutter`内で`flutter build windows --release` | PASS | `build\windows\x64\runner\Release\gui_shell_desktop.exe`をビルド。 |
| 段階導入 | `installer\windows\stage_installed_app.ps1` | PASS | アプリを`%LOCALAPPDATA%\GUI-Shell\installed`へ配置。 |
| ランタイム表明 | `python tooling\release_runtime_assertions.py --check` | PASS | `9 passed, 0 failed`、証拠範囲は`CONFIG,FIXTURE,LIVE_RUNTIME`。 |
| ブローカー・スモーク | `installer\windows\collect_broker_smoke.ps1` | PASS | `release_evidence\windows_broker_smoke.json`、broker statusは`passed`。 |
| Setup Doctorスモーク | `installer\windows\collect_setup_doctor.ps1` | PASS | `release_evidence\setup_doctor_installed.json`、setup statusは`pass`。 |
| インストール済みスモーク | `installer\windows\collect_installed_smoke.ps1` | PASS | `release_evidence\windows_installed_smoke.json`、first-run statusは`passed`。 |
| 検証器 | `python tooling\windows_release_evidence.py` | PASS | 終了コード`0`。リリース証拠の3検査すべてが合格。 |

## 4. リリースゲート結果

| ゲート | 結果 | 検証器の分類 |
|---|---|---|
| `windows_installer_first_run_smoke` | PASS | `none`、`blocks_release=no` |
| `windows_setup_doctor_smoke` | PASS | `none`、`blocks_release=no` |
| `windows_broker_installed_smoke` | PASS | `none`、`blocks_release=no` |

## 5. 受入基準の結果

| 基準 | 結果 | 証拠 |
|---|---|---|
| `python tooling\windows_release_evidence.py`の終了コードが`0` | PASS | 検証終了コード`0`。 |
| `first_run.status = passed` | PASS | `windows_installed_smoke.json`。 |
| `main_window_handle`がゼロではない | PASS | `6358766`。 |
| `process_running_after_launch = true` | PASS | `first_run`へ記録。 |
| `first_window_visible = true` | PASS | `first_run`へ記録。 |
| `broker_mediated_launch = true` | PASS | `first_run`へ記録。 |
| `broker_transport = authenticated_loopback_tcp` | PASS | `first_run`へ記録。 |
| `no_python_runtime_requested = true` | PASS | `first_run`へ記録。 |
| `python_path_entries_remaining_count = 0` | PASS | `first_run`へ記録。 |
| `python_commands_visible_after_scrub = []` | PASS | `first_run`へ記録。 |
| `visible_surfaces_complete = true` | PASS | UIAutomation証拠が必須labelをすべて記録。 |
| `Dashboard`を記録 | PASS | `visible_surfaces.json`。 |
| `NavigationRail`を記録 | PASS | `visible_surfaces.json`。 |
| `Runtime Status`を記録 | PASS | `visible_surfaces.json`。 |
| `Invariant Status`を記録 | PASS | `visible_surfaces.json`。 |
| `installer_grants_authority = false` | PASS | first-runおよびSetup Doctorの証拠へ記録。 |
| `installer_silently_approves_permissions = false` | PASS | first-runおよびSetup Doctorの証拠へ記録。 |

## 6. ローカルに保持した証拠ファイル

| 証拠ファイル | SHA-256 | 状態 | 注記 |
|---|---|---|---|
| `release_evidence\windows_acceptance_transcript.txt` | `945BB8237D17062A407CB1C53CAE413F61A038CC206BA0800777F9AA954ADCBE` | Generated（生成済み） | transcriptをローカルに保持。 |
| `release_evidence\release_runtime_assertions.json` | `EA6B36A77BFD13DDA8431742D6EC703A372181DA8FCEC20C09DE67189822A25A` | Generated（生成済み） | `--json`によるJSON証拠。`--check`も合格。 |
| `release_evidence\windows_broker_smoke.json` | `A0E99FC143C2EBF69CC19E3B18CFFF0D58BE391415BCEB40D7772CF1B145041A` | Generated（生成済み） | broker smoke statusは`passed`。 |
| `release_evidence\setup_doctor_installed.json` | `885E7BE86996D076E1F4E2A70FCEB07760DC22CCB127B94A8A1FA2AA00580DD3` | Generated（生成済み） | Setup Doctor statusは`pass`。 |
| `release_evidence\visible_surfaces.json` | `8C8D5F3B09418D73F3C0B62FF97562E6B89707B98A0DBCB81C768AB2064C28B6` | Generated（生成済み） | UIAutomationが情報源で、必須のsurface labelをすべて記録。 |
| `release_evidence\windows_installed_smoke.json` | `B7B48A72AF6AE88776E4B5F24A2624819C04C6CCCB9B55C0060CF9DAB071466B` | Generated（生成済み） | 最終installed smoke evidenceであり、validator inputである。 |

生の証拠ファイルは意図どおりローカルだけに保持し、コミットしていない。

## 7. 安全・権限に関する注記

- インストーラーとSetup Doctorの証拠は`installer_grants_authority=false`を報告する。
- インストーラーとSetup Doctorの証拠は`installer_silently_approves_permissions=false`を報告する。
- ブローカー証拠は、制限済みの`127.0.0.1` loopback bind、authenticated IPC、durable store readiness、restart replay rejection、crash fail-closed behavior、Python authority runtimeを不要とすること、およびFlutter/Rust FFI authority bridgeがないことを報告する。
- runtime assertionは引き続き合格する。製品Flutter entryは`ShellCoreClient.product()`を使用し、broker-mediated authority operationはfail closedのままで、Python authority process startupを検出せず、FFI/direct bridge authority pathも検出しない。

## 8. 残存リスク

~~~yaml
- item: distributable installer install/uninstall/repair/update evidence
  classification: release_blocker
  reason: この報告が検証するのはstaged-install acceptanceだけであり、packaged installer lifecycleではない。
  required_action: completed product releaseの前に、distributable installer lifecycleを実行して検証する。
  blocks_release: yes
~~~

## 9. リポジトリ変更境界

- ソース変更はこの報告とは別にコミットした。
- 報告の更新対象は`docs\reports\WINDOWS_STAGED_INSTALL_ACCEPTANCE_REPORT.md`である。
- 生の証拠対象は`release_evidence\*`であり、ローカル専用でコミットしない。
- build/stage artifactはローカル専用でコミットしない。

## 10. 最終判定

- コミット`3e7d6079eaeb3af0dd1cd0fc38c84ba095dad9f7`についてのWindows staged-install acceptance: PASS。
- 完成製品のリリース準備完了は主張しない。
