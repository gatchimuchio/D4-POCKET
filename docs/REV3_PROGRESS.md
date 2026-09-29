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

## R1 正本・Blocker体系再編（2026-09-29）

### registry契約

- 既存互換の`classification: release_blocker`と`blocks_release: true`は全項目で保持し、release全体のgateを弱めていない。
- rev3の`cause_category`を全17項目へ追加し、原因とrelease分類を別fieldにした。許可値は`technical_blocker`、`interoperability_evidence`、`platform_evidence`、`physical_device_evidence`、`distribution_identity`、`production_secret`、`owner_decision`。
- `release_tracks`は`windows_v1`、`mobile`、`non_windows`を表す。`blocks_windows_technical_complete`はOwner最終化とWindows技術完成を分離する。
- 現行値はactive unresolved 15件、resolved inactive 2件、全体`release_ready=false`。Windows release scopeは12件、Windows Technical Completeを阻む項目は9件、Mobile release scopeは6件。

| 原因分類 | Active unresolved項目 |
| --- | --- |
| `technical_blocker` | `comprehensive_extension_rev1_completion`, `rev2_desktop_product_distribution`, `rev2_flutter_broker_channel_boundary`, `rev2_export_owner_ui_authority_path`, `rev2_module_pruning_binary_and_measurement`, `rev2_mobile_flutter_native_device_link_boundary` |
| `platform_evidence` | Windows installed／first-run／Setup Doctor／Broker evidenceの4項目。解決済みのWindows起動回帰とRust test実行方針も同分類で履歴保持。 |
| `physical_device_evidence` | `rev2_mobile_device_evidence` |
| `distribution_identity` | `rev2_mobile_distribution`, `windows_distribution_identity` |
| `production_secret` | `audit_anchor_external_tamper_evidence_proof` |
| `owner_decision` | `owner_go` |
| `interoperability_evidence` | 単独原因として分類すべきactive項目は現registryにはない。未検証の相互運用を合格扱いした意味ではない。 |

### Track境界・Owner待ち再監査

- Windows専用ではないMobile実機・配布・native Device Linkの3 blockerは`mobile`だけに割当て、Windows 1.0 strict releaseとTechnical Completeから除外した。Android実機凍結指示は変更していない。
- Windows正式配布identityは技術的Installer／Update作業から`windows_distribution_identity`へ分離した。test identity／unsigned artifactでR0–R14を進め、正式identityはTechnical Complete後のR16 Owner Finalizationまで要求しない。
- ExportのOwner No／Yes経路は隔離test identityで検証可能とし、Final GOをroutine試験条件から外した。
- Agent Task／Compare／Handoffの制御試験は非課金identity・multi-instance Codex・決定論的fixtureで進める。実Agent／provider相互運用の証拠をfixtureへ昇格させない。有料資格を使う試験をR0–R14のOwner待ち条件にしない。
- Offline Audit production keyと署名は真の`production_secret`、Final GOは真の`owner_decision`としてrelease gateに保持する。いずれもWindows Technical Completeを阻止しない。
- Windows Rust test方針のresolved項目は、古いrun #17を過去記録に残し、`current_validation`をR0のWindows Actions #19／commit `ec9b11a1330c3626fe7e7c25ef5c4068241b1d28`へ更新した。

### R1検証

- Registry構文・全項目cause／track scope検査: PASS。未知cause、未知track、非boolean Technical Complete flagはnegative testで拒否する。
- `python -X utf8 tooling/schema_check/check_schemas.py`: Schema 149件、正常example 149件、negative fixture 192件でPASS。
- `python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`: 225 checksでPASS。Windows／Mobile trackの分離、Owner-only blockerのTechnical Complete除外、strict Windows releaseのtrack選択を検査する。
- `python -X utf8 tooling/release_gate_check.py`: PASS。strict Windows releaseは未解決Windows blockerのため失敗し、Mobile専用項目を列挙しない。Windows Technical Complete gateは9件が未解決で、Owner-onlyとMobile項目を列挙しない。
- 先行した統合validatorはstrict日本語監査が新しい例外診断文1件を検出してexit 1。診断文を日本語化し、失敗を隠さず修正履歴として保持した。
- 修正後の`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`: exit 0。登録済み10 development checksが全件合格。strict日本語監査は1111 repository files／findings 0、Schema 149／正常example 149／negative fixture 192、Conformance 225。manifest、release gate、package portability、release smoke、evidence bundle、runtime assertions、既存C32開発監査も合格。`release_ready=false`とblockerは維持。
- 修正後の`python -X utf8 tooling/manifest.py --check`: PASS。`git diff --check`もPASS。

## R2 Codexエージェント作業実行の本番経路 — Windows隔離規則の初回是正（2026-09-29）

### 再確認で観測した差

- 実インストール済みCodex CLIは`codex-cli 0.158.0-alpha.2.1`。`codex exec --help`でJSONL、ephemeral、`--ignore-user-config`、`--cd`、`workspace-write`を含む現行interfaceを確認した。
- Adapterの現行設定をCodex Windows restricted-token sandboxへ直接適用した合成marker試験では、Task時の`TEMP`／`TMP`をWorkspace scratchへ向けると`.env`、`.ssh`、`secrets/`およびWorkspace外markerを拒否した。ただし`.env.production`と深さ10の`.env`は許可された。TEMP／TMPをscratchへ向けない比較ではTEMP配下の外部markerも許可された。
- Adapterの固定profileを更新し、Workspace root内`**/.env.*` denyと`glob_scan_max_depth=32`を追加した。同じ直接sandboxで`.env.production`、深さ10の`.env`、scratch外のmarkerは拒否された。一方、任意別名として用いた`config/credential-backup.txt`は許可された。
- この再検査はCodex CLI Windows sandboxの`LIVE_RUNTIME`証拠に限る。`codex exec`実Task、Rust Broker、Owner Approval、Agentによる書込、Task lifecycleを通しておらず、能力宣言を`supported`へ変更していない。深さ32超と未列挙secret名も未保証。

### 変更した実装境界

- `CodexCliAdapter`のTask専用filesystem profileへ`.env.*` denyを追加し、glob走査上限を8から32へ変更。Network無効、`:root=deny`、`:minimal=read`、Workspace scratch固定は維持。
- Rust unit testとConformanceでprofile内の追加pattern、走査上限、filesystem overrideの単一table条件を検査する。
- `docs/specs/agent-runtime.md`の現行設定と直接sandboxで観測した範囲を更新。過去のR0／rev2検証履歴は変更していない。

### この作業単位の検証結果

- `cargo +1.95.0 check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`: 成功。
- `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`: PASS。lib 345件、helper binary 10件、統合test 36件、合計391件成功。初回全体実行では旧profileを固定期待するFake CLI fixture 1件が失敗したためfixtureを現行契約へ更新し、該当test単独と全targetを再実行した。
- `rustfmt +1.95.0 --edition 2021 --check native/rust_helper/src/adapters/codex_cli.rs native/rust_helper/tests/fixtures/fake_codex_cli.rs`: 成功。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`: PASS。日本語基底監査1111 files／findings 0、Schema 149／example 149／negative fixture 192、Conformance 225、登録済み10 development checks全件pass。`release_ready=false`およびWindows installed-path等の既存release blockerは維持。
- 記録追記後にmanifestを再生成し、`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`を再実行して登録済み10項目すべて成功。`python -X utf8 tooling/manifest.py --check`と`git diff --check`も成功。
- 補助確認の`cargo +1.95.0 fmt --manifest-path native/rust_helper/Cargo.toml -- --check`は、変更対象外の既存Rust file群に対するformat差分でFAIL。`--check`のためfile変更なし。変更した2 Rust fileの個別checkは上記の通りPASS。
- 初回の厳格日本語監査はこの節の見出しに英語語順が残りFAILしたため、日本語見出しへ修正して再実行しPASS。FAIL履歴は隠さず記録した。

### 次工程

R2を継続し、登録済み任意secret pathのTask sandboxへの伝播、Owner controlled Yes／No／stale／replay、実`codex exec`を通るBroker production path、取消／crash後回復、diff／test結果／Content Exposureの接続を検証する。これらのLIVE_RUNTIME証拠が成立するまで`task_execution=unsupported`と関連release blockerを維持する。

## R2追補 Workspace登録secret pathのTask sandbox伝播（2026-09-29）

### 成立した変更

- `WorkspaceReader`が保持する正規化済み登録secret pathを、Brokerの`DialogueWorkspaceBinding`からAgent Task専用揮発contextへ渡し、Codex Taskの単一filesystem overrideへ決定的に追加する。
- 各登録pathの完全一致と子孫をdenyし、登録可能なglob構文記号`[`、`]`、`{`、`}`はliteral globへescapeする。pathを再検証し、256件または12 KiBを超える設定をprocess spawn前にfail-closedで拒否する。
- 既存の`.env`等の固定deny、`:root=deny`、`:minimal=read`、glob深度32、network無効を保つ。context Debugはpath名でなく件数だけを示し、回復journalへpath名を保存しない。CLI引数にglob設定を渡すため、ローカルprocess command lineからpath名が見える可能性は残る。
- Codex fake CLI fixtureは登録secret pathと固定denyの両方を含む正確なfilesystem overrideを要求する。Adapter metadataは引き続き`task_execution=unsupported`。

### 証拠・検証履歴

- `cargo +1.95.0 check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`: 最終実装で成功。
- `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`: 最終実装で394件成功（lib 348、helper binary 10、統合test 36）。Task実行はWindows fake CLI fixtureであり、実Codex `exec`／Broker経路ではない。
- 登録pathを通すWorkspace bindingのfocused Rust testは、テスト整形後の最終実行でも1件成功。
- `rustfmt +1.95.0 --edition 2021 --check native/rust_helper/src/adapters/codex_cli.rs native/rust_helper/tests/fixtures/fake_codex_cli.rs`: 成功。補助のcrate全体／複数legacy file形式checkは既存未整形箇所を多数検出したため不成功。広範な無関係再整形は行わず、変更したAdapterとfake CLIのcheckを個別に成立させた。
- `python -X utf8 tooling/日本語基底監査.py --strict`: 初回は新規Rustの3文字列を機械識別子の誤検出として検出。設定templateを分離し、test assertionの近接文字列を整理後、1111 repository files／findings 0で成功。初回失敗はこの履歴に保持する。
- `python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`: 最初の単独試行はOneDrive上のSchema読込で一時的な`OSError [Errno 22]`。file単体読込を確認後の統合validatorではConformance 225件すべて成功し、Codex Adapter固有Conformanceも空errorで成功した。
- strict監査修正前の初回`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`は、日本語監査3件と変更後manifest未再生成を検出した。Schema 149／example 149／negative fixture 192、Conformance 225、release smoke、evidence bundle、runtime assertion、開発監査は成功していた。strict監査を0件へ修正し、`python -X utf8 tooling/manifest.py --write`で1108件を再生成した後、同じ統合validatorを再実行してexit 0。登録済みdevelopment check 10件すべて成功し、Schema 149／example 149／negative fixture 192、Conformance 225、release smoke、evidence bundle、runtime assertion、C32開発監査も成功した。別のfinal development auditは既存release blocker 5件と`release_ready=false`を報告し、これらを解除しない。
- 合成pathのWindows mxc直接probeは手動構成した候補globで実施し、登録pathの完全一致・子孫・深さ64、およびliteral bracket／brace pathのread拒否と、非登録decoyのread成功を観測した。この`LIVE_RUNTIME`結果はRust生成argv、実`codex exec`、Broker、Owner Approval、Agent Taskを検証しない。

### 次工程と残存gate

本追補で登録除外path伝播とsandbox globの局所単位を閉じた。Owner Yes／No／stale／replay、実`codex exec`を通るBroker production path、実行失敗・取消・crash後回復、diff／test結果のContent Exposure接続は未成立であり、関連項目を`release_blocker`として保持する。Windows実機のCodex Taskは未実行で、`task_execution=unsupported`を維持する。

## R2追補 Rust生成profileのWindows直接sandbox検証（2026-09-29）

### 成立した局所証拠

- Rust Adapter testに明示実行・既定ignoredのWindows probeを追加した。`GUI_SHELL_CODEX_SANDBOX_TEST_EXE`で所有者が指定した絶対pathのCodex CLIだけを使い、一時Workspaceと分離した一時`CODEX_HOME`内の合成markerを読む。Codex model、`codex exec`、credential、network要求は起動しない。
- `build_codex_command`の出力からTask用`-c`設定を直接抽出し、生成された`default_permissions`値から得たprofile名とともに実CLIの`codex sandbox --permission-profile`へ渡す。登録file完全一致、登録directoryのnested file、literal bracket／brace pathのread拒否、glob decoyのread許可、Workspace内writeを1回の実sandbox processで確認した。
- 実Codex CLI `0.158.0-alpha.2.1`、Windowsでignored test 1件が成功。これはRustが生成するfilesystem profileの直接sandboxでの適用意味を示す`LIVE_RUNTIME`証拠に限る。

### 失敗履歴と証拠境界

- 初回test実行は`codex sandbox`が必須とする`--permission-profile`未指定のためusage error。生成済み`default_permissions`からprofile名を読み取り明示するようtestを修正した。
- TEMP scratchにもRust生成のTEMP／TMP値を与える試行はCodex sandbox process自体が成功したが、期待Workspace scratch内のmarkerが見つからなかった。helperから子processへの環境伝播が未確認なので、このassertionはtestから除き、scratch環境伝播の証拠へ昇格しない。これは`codex exec`本番経路のfailureとは判定しない。
- strict日本語監査の初回は合成bracket pathとinline PowerShell文字列の2箇所を未局所化表記として検出し、統合validatorもこの1検査だけ失敗した。合成file名へ日本語を含め、PowerShell probe labelを日本語化して再監査する。例外台帳や監査条件の緩和は行わない。
- testは直接`codex sandbox`だけを起動し、Broker、Owner Yes／No／stale／replay、production `codex exec`、Rust生成WorkspaceTaskScratchの実ライフサイクル、process終了、Audit／Recovery、diff／test結果のContent Exposureを通さない。`task_execution=unsupported`、`release_ready=false`と関連`release_blocker`を維持する。
- 初回profile指定不足の実行と、scratch marker不在の試行は失敗履歴として残し、成功した最終probeと別に扱う。

### 検証

- `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --lib --no-run`: 成功。
- 明示指定Codex CLIで対象ignored testを起動: 1 passed。通常のRust test suiteでは自動起動しない診断probeであり、Ownerが指定したCLI pathでのみ実行する。
- `cargo +1.95.0 check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`: 成功。最終のpath例示変更後はignored testを再compile・再実行して1 passed。
- `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`: 394 passed／0 failed／1 ignored。ignoredは実Codex sandboxの明示実行probeで、別途実行して成功した。
- `rustfmt +1.95.0 --edition 2021 --check native/rust_helper/src/adapters/codex_cli.rs`、`python -X utf8 tooling/manifest.py --check`、`git diff --check`: 成功。
- 変更後の統合validatorは初回strict日本語監査だけが失敗し、修正後の再実行はexit 0。登録development check 10件すべて成功し、日本語監査1111 files／findings 0、Schema 149／example 149／negative fixture 192、Conformance 225を確認した。release gateは既存blockerを検出したまま`release_ready=false`を維持する。

## R2追補 Rust生成TEMP／TMP値のMxC子process照合（2026-09-29）

### 成立した局所証拠

- 前節のmarker不在probeは、`build_codex_command`由来のTEMP／TMPをprobe起動commandへ明示伝達したことを記録していなかった。この結果だけでは子processへの値伝播を判定できないため、診断testを補正した。
- 補正testは本番command builderのconfig overrideとTEMP／TMPの2値を抽出し、認証・model起動なしで同じ値を実Codex CLI `codex sandbox`の起動environmentへ渡す。mxc配下の合成PowerShell childは値そのものやpathを保存せず、各値がRust生成WorkspaceTaskScratchと一致するかだけを合成Workspace内へ記録する。
- 明示指定Codex CLI `0.158.0-alpha.2.1`でignored test 1件が成功し、TEMP／TMPはいずれもWorkspaceTaskScratchと一致しなかった。これは直接`codex sandbox` childの`LIVE_RUNTIME`観測であって、production `codex exec`内で起動するAgent tool childの環境やcleanupを証明しない。AppContainer領域のcleanup保証も未確認である。
- 失敗履歴として、最初の補正test buildは`Command::envs`へ参照tupleを渡した型不一致で失敗し、owned pair iteratorへ直してから実行した。production codeやOS保護設定は変更していない。

### 検証と残存境界

- `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --lib 'adapters::codex_cli::tests::Rust生成Task設定で実Windows隔離の登録secretを拒否する' -- --ignored --exact --nocapture`: 成功、1 passed。これは明示指定されたCLIを使う限定診断probeであり、通常suiteではignored。
- `cargo +1.95.0 check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`: 成功。`cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`: 12 targetで394 passed／0 failed／1 ignored。ignoredの直接sandbox probeは別途明示実行して1 passed。
- `rustfmt +1.95.0 --edition 2021 --check native/rust_helper/src/adapters/codex_cli.rs`、`git diff --check`: 成功。
- 最初の統合validator実行は、編集5 fileのMANIFEST hash未更新を検出した。`python -X utf8 tooling/manifest.py --write`で1108 fileを再生成後、`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`はexit 0。厳格日本語監査1111 file／finding 0、Schema 149／example 149／negative fixture 192、Conformance 225、登録development check 10件すべて成功した。
- 同validatorの製品証拠面はWindows installed evidenceが未収集でrelease gateとrelease readinessを解除していない。`release_ready=false`と既存`release_blocker`を維持する。
- Broker／Owner Approval／production `codex exec`／実Agent tool、Taskの正常終了・取消・crash cleanup、Audit／Recovery、結果のContent Exposureは通していない。scratch有効性を主張せず、`task_execution=unsupported`、関連`release_blocker`、`release_ready=false`を維持する。
