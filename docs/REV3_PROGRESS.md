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

## R2追補 実`codex exec` MxC shell childのTEMP／TMP・終了時可視性（2026-09-29）

- `tooling/codex_mxc_exec_temp_probe.py`を明示実行専用のdevelopment probeとして追加した。Owner指定の実CLIを一時`CODEX_HOME`・合成Workspaceから起動し、loopback偽Responses APIだけをmodel providerとして使い、偽応答から実`exec_command`を実行する。Rust AdapterのTask permission設定相当として`windows.sandbox="mxc"`、`:root=deny`、network無効、Workspace内secret denyを設定する。Adapterやproduction pathの設定は変更しない。
- 実行command: `python -X utf8 tooling/codex_mxc_exec_temp_probe.py --exe <Owner指定のcodex.exe絶対path> --runs 3`。Codex CLI `0.158.0-alpha.2.1`で3/3回、実`codex exec`と実MxC shell tool child、合成Task scratchへの書込、TEMP marker書込、Codex turn正常終端を確認した。各runでResponses API要求2回、shell command exit 0。
- 3/3回、tool childのTEMPとTMPは互いに一致したが、Rust生成WorkspaceTaskScratchとは一致しなかった。値や絶対pathを保存・出力せず比較した結果、TEMP pathは`Packages…\AC\Temp`形式だった。shell child内で一意TEMP markerが存在した一方、Codex CLI終了後はmarkerとTEMP directoryの双方がhostから見えなかった。これは「終了後にhost可視で残っていない」という`LIVE_RUNTIME`観測であり、物理削除・一般的なcleanup保証へ昇格しない。
- 現在のCodex process tokenは非管理者（`IsUserAnAdmin=false`）。資格情報環境変数を子環境から除去し、実model／有料資格を使わず、非loopback通信はloopback proxyで拒否した（3 run合計12要求）。Codex設定、OS保護設定、Repository外の恒久データは変更していない。試験workspace／CODEX_HOMEは各run後に破棄した。
- 最初の試行はGit管理外の一時WorkspaceをCodexがuntrustedとしてmodel request前に拒否した。probe内の`--skip-git-repo-check`で一時Workspaceだけを対象化して解消し、製品command builderには追加していない。
- この証拠は直接Codex CLI＋MxCの正常終端に限る。Rust Broker consumer、Owner Approval、production Task、実model、取消／期限／crash、process群停止、Audit／Recovery、Content Exposureは通していない。TEMP/TMP scratch mismatchの扱いと異常終端時cleanupを含むrelease blockerを保持し、`task_execution=unsupported`、`release_ready=false`を維持する。

## R2追補 非Git登録Workspaceに対するCodex CLI起動条件（2026-09-29）

### 原因と局所修正

- 実Codex CLI `0.158.0-alpha.2.1`の`exec --help`は`--skip-git-repo-check`を提示する。これを付けずにGit管理外の合成Workspaceで`codex exec`を起動した実測では、CLIは`Not inside a trusted directory and --skip-git-repo-check was not specified.`をstderrへ出して終了値1となり、Responses API要求は0件だった。これはAdapterの実Workspace書込失敗ではなく、CLI自身のGit前提による起動拒否。
- DialogueとTaskで共用する`build_codex_command`のargvへ当該optionを固定し、実CLI能力検査でも必須optionとして確認する。Fake CLI、Rust単体試験、Conformanceを同期し、option欠落CLIではAdapter初期化を拒否する。read-only Dialogueは既存の`--sandbox read-only`を保ち、Task専用のPermission profileとApproval条件は変更しない。
- このoptionはCodex CLI自身のGit repository安全確認をDialogueとTaskの双方で迂回する。OpenAI公式説明は破壊的変更防止の確認であることと、安全な環境だと確信するときに限るoverrideであることを示す（[Non-interactive mode](https://learn.chatgpt.com/docs/non-interactive-mode?translationFallback=ja-JP)）。GUI-Shellでは登録WorkspaceとBrokerの現行Session照合を維持し、Dialogueはread-only sandbox、TaskはTask専用Workspace Permission・個別Owner Approval・sandbox設定を維持する。これらはGit確認迂回の別個の境界であり、production Agent Taskやfilesystem隔離の成立証拠ではない。

### 証拠・残存gate

- 既存の明示実行`tooling/codex_mxc_exec_temp_probe.py`は、同option付きの実CLI `codex exec`とMxC shell childを合成Workspace／loopback偽Responses APIで3/3回完了させている。前項のTEMP／TMP probeと同じく、real model・credential・Broker・Owner Approvalは用いない。optionなしの拒否比較も実CLI上でResponses API要求前に観測した。途中の通信切断1回は失敗履歴として保持し、後続成功で消去しない。
- Rust focused test（共通argvへの固定、CLI能力検査、欠落時のAdapter初期化拒否）とFake CLI／Conformanceは後述のコマンドで確認する。実Broker Task起動、Owner承認、Task隔離、secret／外部path拒否、取消・期限・crash後cleanupは未検証である。
- `--skip-git-repo-check`が回避するCLI側Git確認を明示した上で登録Workspaceに限定して使用する。Codex Agent Taskの`task_execution=unsupported`、該当`release_blocker`、`release_ready=false`を維持し、option追加だけでCapabilityやApprovalを昇格しない。

### 検証

- `cargo +1.95.0 check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`: 成功。
- `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`: 全12 targetで398 passed／0 failed／1 ignored。CLI interface検査、Task argv、Fake CLIによるBroker経由Task fixtureを含む。ignoredは所有者指定CLIを使う既存の合成sandbox診断probeで、今回の通常試験では起動しない。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`: exit 0。登録development check 10件成功、厳格日本語監査1113 repository files／0 findings、Schema 149／example 149／negative fixture 192、Conformance 225。Manifest、release gate、package portability（source ZIP内Conformanceを含む）、release smoke、evidence bundle、runtime assertions 12／0、C32開発監査も成功。
- 同検証は`release_ready=false`を維持。最終development auditは`release_blocker` 31件と`post_v1_scope` 1件を報告し、正式releaseやCodex Task隔離を主張しない。
- Rust変更はWindowsローカルで全target check／testまで通ったため、この単位では同じ範囲を重ねるGitHub Actionsを起動していない。ActionsやCI status checkを品質基準として追加していない。

## Windows現行RustブローカーのRelease実測（2026-09-29）

- commit `75412301dfd7c17c99237fb795b894ffde119b5f`時点の変更なし作業treeから、独立した一時Cargo出力先へ`cargo +1.95.0 build --locked --manifest-path native/rust_helper/Cargo.toml --release --bin gui_shell_rust_helper --target-dir <isolated temporary directory>`を実行し成功した。helperのSHA-256は`fbc43b584b6e4f75416b7a1931e63fb8e3c4f691d1d03367a94c7ef8af0b92ef`。release buildは既存`minidora.rs`の`dead_code` warning 2件を出したが、buildは成功した。
- helperに`installer/windows/collect_broker_smoke.ps1` collector version 5を実行し、通常接続資格とloopback bind、認証済みIPC、永続store準備、Broker再起動後の同nonce再利用拒否、新nonceのhealth受理、強制process終了後のIPC接続拒否、一時session資格file作成・削除をすべて観測した。collector statusは`passed`、errorsは空。
- build成果物、store、session file、collector出力は一意なsystem temporary directoryへ隔離した。session fileはcollectorが削除した。既存`release_evidence/windows_broker_smoke.json`およびinstalled evidenceは上書きしていない。helperとcollectorのruntime値にCredential実値は含まれない。
- 証拠classは現行commitのRust Release helper単体に対する`LIVE_RUNTIME`。これは正式Windows installed application、Flutter起動経路、別user profile、完全なsource/artifact provenance bundleの証拠ではないため、`windows_broker_installed_smoke`はunresolvedのまま。helper artifactが最終配布物と一致する実測を`release_evidence/windows_installed_smoke.json`へ統合し、strict validatorに通す必要がある。`release_ready=false`を維持する。
- 初回の統合validatorは追加した2見出しの英語語句をstrict日本語監査が検出してexit 1となった。見出しを日本語化し、監査器や例外台帳を変更せず`python -X utf8 tooling/日本語基底監査.py --strict`を再実行して1113 files／0 findingsでPASSした。初回FAILはこの履歴に残す。
- 修正後の`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`はexit 0。登録development check 10件、Schema 149／example 149／negative fixture 192、Conformance 225、Manifest、release gate、package portability、release smoke、evidence bundle、runtime assertions 12／0、C32開発監査が成功した。release evidence bundleは既存blocker 5件と`release_ready=false`を保持し、このstandalone smokeをinstalled evidenceへ昇格していない。

## R2追補 Broker対話制御からCodex Adapter fake Taskまでの縦断fixture（2026-09-29）

### 成立した局所証拠

- Windows専用Rust testで、BrokerのWorkspace登録・対話Session・Task制御から、実`CodexCliAdapter`、fake Codex CLI executable、Broker管理scratchまでを一つの縦断fixtureとして接続した。
- Permissionなしの拒否、本文hashに結合したOwner Approvalなしの拒否、fixture内でのPermission／Approval発行、Task開始時のApproval一回消費、終端状態のhash-only射影、本文・fake応答の非露出、scratchの正常完了後片付けを確認する。
- 縦断専用wrapperだけが`task_execution=supported`を返し、証拠源を`FIXTURE`と明記する。wrapperの内側にある製品`CodexCliAdapter` metadataは引き続き`task_execution=unsupported`であることをtest内で確認した。test用Rust CLIは実Codex CLIでも実modelでもない。
- この結果は、合成Adapter metadataを使ったBroker制御と実Rust Adapter実装の結合fixtureに限る。実Codex CLI、production Broker登録・認証済みIPC、installed product、Owner native確認画面、実Agent Taskの証拠へ昇格しない。

### 失敗履歴と検証

- focused testの初回はWorkspace登録secret pathをfake CLI fixtureの保護対象と一致させておらず失敗した。登録内容をfixtureの固定pathへ合わせて修正した。
- 次のfocused試行ではAudit assertionが操作IDとAudit callbackの実際の日本語event labelを取り違え失敗した。観測されたevent labelに対するassertionへ修正した。
- その後のfocused Windows testは1 passed。テスト専用一時directoryのpanic／失敗時残存を避けるRAII cleanupを加えた。修正前の2回の失敗試行が作成した一時directoryはrepository外に残り、実行環境の削除制御によりcleanup commandを実行できなかったため、local test artifactとして未解決である。
- `cargo +1.95.0 check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`: 成功。
- `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --lib 'broker::dialogue::tests::Broker制御からCodexAdapterを通るfakeTaskは承認を一回消費しscratchを片付ける_fixture' -- --exact --nocapture`: 成功、1 passed。
- `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`: test support分離後の最終全実行はlib 349 passed／0 failed／1 ignored、helper binary 10 passed、統合test 36 passed（計395 passed）。履歴では全suite 4回のうち2回、変更対象外のA2A loopback test 1件が`a2a_connection_failed`で失敗し、他の2回は全件成功した。同testの単独実行は最初の1回と続く8回連続が成功した。失敗の根本原因は確定していないため、最終全実行のPASSと過去の間欠失敗を両方記録し、原因を推定しない。
- `rustfmt +1.95.0 --edition 2021 --check native/rust_helper/src/broker/dialogue.rs`、`git diff --check`: 成功。
- 最初の統合validatorは日本語監査とSchema検査を通過した一方、Conformanceがunit test内の`std::process::Command`／`std::fs::write`を`src/**/*.rs`の禁止helper patternとして検出し、package portability検査も同じ違反を検出した。権限pattern検査を弱めず、fake CLI生成とtest一時directory管理を`native/rust_helper/tests/support/broker_codex_fixture.rs`へ移した。Broker制御fixture本体は`#[cfg(test)]`のまま保持し、Conformance／package portabilityは修正後に成功した。
- test support fileの追加・移動後、package portabilityの初回再実行は`MANIFEST.sha256.json`のdialogue source hash staleで失敗した。`python -X utf8 tooling/manifest.py --write`で1109 source fileを再生成し、`python -X utf8 tooling/manifest.py --check`、`python -X utf8 tooling/packaging_portability_check.py`は成功した。
- test support追加後の統合validatorは日本語監査が診断文字列1件を検出して失敗した。作業領域作成時の診断を日本語化し、`python -X utf8 tooling/日本語基底監査.py --strict`を1113 files／findings 0で再実行成功した。修正後の`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`はexit 0。登録済み10 development checks、日本語監査1113 files／0 findings、Schema 149／example 149／negative fixture 192、Conformance 225、package portability、release smoke、evidence bundle、runtime assertions、C32開発監査が成功した。Windows installed evidence等の既存release blockerを保持し、`release_ready=false`を維持した。初回の日本語監査失敗とその修正は上記履歴に残す。

### 次工程と残存gate

本追補は`FIXTURE`の局所縦断証拠であり、Ownerの実UI確認、認証済みDesktop IPCからの実`codex exec`、実model、AgentによるWorkspace書込、取消・期限・crash回復、結果diff／testのContent Exposure、Windows installed productを検証しない。これらのR2項目は`release_blocker`のまま保持し、製品Adapterの`task_execution=unsupported`と`release_ready=false`を変更しない。

## R2追補 Broker取消からfake Codex子孫停止・scratch回収（2026-09-29）

### 成立した局所証拠

- 既存Windows Rust縦断fixtureへ取消分岐を追加した。正常fixture Task完了後に新しい一回Permissionと本文hash結合Owner Approvalを発行し、Broker consumerから実`CodexCliAdapter`実装とfake CLIを起動する。
- fake CLIは試験用子processをspawnし、heartbeat fileを継続更新する。2 byte以上のheartbeatを観測して子孫が稼働中であることを確認後、Brokerへ`AgentTask取消`を要求する。
- 取消応答は`running`のまま、AdapterがWindows Job Object配下のprocess群を終了してworkerが戻った後だけterminal `cancelled`となる。terminal後にresult hashはなく、150 msの観測窓でheartbeatが増えず、Broker管理Task scratchもなく、開始・取消・terminal監査へ指示本文やheartbeat pathが露出しないことを確認する。
- 証拠源は合成metadata wrapper、fake CLI、unit test内Broker、in-memory scratch journalを使う`FIXTURE`である。実Codex CLI／model、production認証IPC、永続Audit／crash recovery、実Workspace変更、installed productを示さない。製品Codex Adapterの`task_execution=unsupported`は維持する。

### 検証・履歴

- 初回focused compileは既存監査callbackをtest途中でdropした後に再利用していたためRust借用検査で失敗。callbackを両Task終了後まで保持し、監査assertionを末尾へ移動して修正した。
- `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --lib 'broker::dialogue::tests::Broker制御からCodexAdapterを通るfakeTaskは承認を一回消費し正常完了・取消後にscratchを片付ける_fixture' -- --exact --nocapture`: 成功、1 passed。
- `cargo +1.95.0 check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`: 成功。
- `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`: 成功。Windows Rust全targetでlibrary 349 passed／0 failed／1 ignored、helper binary 10 passed、integration 36 passed（合計395 passed／0 failed／1 ignored）。以前から記録済みのA2A間欠失敗履歴は原因未確定のまま保持する。
- 初回strict日本語監査はprotocol operation識別子と短いtest診断語の2件を検出した。operation識別子はtest専用定数へ分離してwire値を保持し、診断語を日本語化した。再実行した`python -X utf8 tooling/日本語基底監査.py --strict`は1113 files／負債0で成功。
- `python -X utf8 tooling/manifest.py --write`で1110件を生成し、`python -X utf8 tooling/manifest.py --check`: 成功。`python -X utf8 tooling/schema_check/check_schemas.py`: Schema 149件、正常example 149件、negative fixture 192件で成功。`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`: 225 checksで成功。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`: exit 0。登録済みdevelopment check 10件、日本語監査、Schema、Conformance、Manifest、release gate、package portability、release smoke、evidence bundle、runtime assertions、C32開発監査が成功。validatorは`release_ready=false`と既存release blockerを報告し、解除していない。
- `rustfmt +1.95.0 --edition 2021 --check native/rust_helper/src/broker/dialogue.rs native/rust_helper/tests/support/broker_codex_fixture.rs`、`git diff --check`: 成功。初回focused compileの借用検査失敗は監査callbackの寿命を整理して解消した。現行Windowsで全target検証が成立したためGitHub Actionsは使用していない。
- Windows installed／Owner／production runtime証拠など既存release blockerを維持し、`release_ready=false`。GitHub Actionsは使っていない。今回必要なRust検査は現行Windows localで実行できた。

### 次工程と残存gate

本fixtureはAdapter経由のBroker取消・process群停止・scratch cleanupをfake CLI上で縦断確認したに過ぎない。production `codex exec`とreal Agent tool、Owner native UI、実model、実Filesystem隔離、deadline／crash／強制Broker終了、永続Audit／Recovery、Workspace diffとtest結果のContent Exposure、Windows installed productは未検証の`release_blocker`。`task_execution=unsupported`、`release_ready=false`を維持する。

## Windows installed collector v15 Control View追補（2026-09-29）

### 実装と観測範囲

- `installer/windows/collect_installed_smoke.ps1`のtray menu／surface取得をRaw ViewからControl Viewへ変更した。tray探索は対象frontend PIDでtop-level windowを絞り、surface projectionは10,000 element上限とruntime ID重複検出を持つ。上限到達または重複時は完全なsurface証拠と見なさない。collector versionは15。
- `tooling/windows_release_evidence.py`はUI Automation sourceに`full_uiautomation_tree_projection`、`tree_view=control`、`capture_limit=none`を要求する。`tooling/conformance_tests/run_conformance_skeleton.py`にはControl View／欠落tree拒否とtray PID境界のtestを追加し、既存accessibility-tree sourceの別policyは維持した。
- `installer/windows/README.md`、`docs/WINDOWS_RELEASE_EVIDENCE.md`、`release_blockers.registry.json`を実測と残存gateに合わせた。ROADMAPのC33失敗履歴は書き換えず、追補を追加した。
- 製品binaryはclean source commit `ae337eee62074229244fb0502fa498c3daa1a3e3`からstagingしたFlutter Windows Release、Rust Broker helper、launcher。dirtyなcollectorを含む現在の未commit作業treeから製品をbuildした証拠ではない。

### Windows実測（DiagnosticOnly）

- 実起動したfrontendのControl Viewは122 nodeで、Dashboard、NavigationRail、Runtime Status、Invariant Statusの4 surfaceすべてを検証器が受理した。trayからの通常終了、forced exitなし、launcher exit code 0、Broker endpoint除去を確認した。
- 初回config生成、Setup Doctor報告の成功、通常Broker正常性要求の受理、起動・終了Audit、config Audit hash一致、Pythonを指すPATHから15件を除去し残存0も観測した。これは限定された`LIVE_RUNTIME`実測で、Setup Doctor画面の可読性や全体release保証ではない。
- first-run結果は`diagnostic_only`で、profileは一時配置に使ったWindows userと同一。`validate_installer_first_run`はこの2条件を理由に拒否した。診断証拠fileのSHA-256: `EF203EB9CB98FC53FF97705113EFE837DA67C57893F49D72BFD7CB7BAACEA9F0`。画面領域投影fileのSHA-256: `5545D3D593248D0D8D214C6C18D19C0BD6ED05EA41ABB128EC0382503A0138D7`。Computer Useによる画面・accessibility観測は原因把握の補助に限り、正式collector証拠へ混ぜていない。
- Windows release evidence検証器は、profileと実行来歴の分離、完全な証拠一式、Setup Doctorの操作者向け可読性、総合証拠一式内のBroker smoke、外部監査基点が不足として不受理。単独Broker smokeのPASSは総合証拠内の証明に代用しない。

### 検証と履歴

- PowerShell parserでcollector構文を確認: 成功。
- `python -m py_compile tooling/windows_release_evidence.py tooling/conformance_tests/run_conformance_skeleton.py`: 成功。
- `python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`: 225 checks成功。初回は新positive fixtureの`capture_limit`欠落と旧Raw View前提の静的testで失敗し、fixtureと期待条件をControl View contractへ合わせた後に成功した。
- `_validate_surface_match_evidence(...)`: 4 surfaceを受理し、findingなし。first-run validatorは`diagnostic_only`と同一profileをrelease blockerとして正しく拒否した。
- DiagnosticOnly evidenceに対するfull Windows release evidence validatorは意図どおり不受理。成功したsurface subcheckを総合PASSへ昇格しない。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`の初回は、追補した8 fileのmanifest hash未更新でmanifest／release gate／package portabilityが失敗した。失敗履歴を残し、`python -X utf8 tooling/manifest.py --write`で1110 fileを再生成して再実行した。
- 再実行した統合validatorはexit 0。strict日本語監査（1113 file・finding 0）、Schema（149 schema／149 example／192 negative fixture）、Conformance 225 checks、Manifest、development release gate、package portability、release smoke、evidence bundle、runtime assertions（12成功／0失敗）、C32開発監査がすべて成功した。release evidence bundleは5件のrelease blockerを正しく保持し、`release_ready=false`。development release gateの検査成功を製品release可能の意味へ昇格しない。
- 文書更新後に再生成したManifestのcheck、strict日本語監査、Schema、Conformance、PowerShell parser、Python compile、`git diff --check`も成功。Windows hostで必要検証を実行できたためGitHub Actionsは使用していない。

### 残存gate

- `windows_installer_first_run_smoke`: `release_blocker` — 別Windows profileでの正式first-run証拠がなく、現artifactは`diagnostic_only`。
- `windows_evidence_provenance_isolation`: `release_blocker` — run固有provenanceと必須evidence bundleが未成立。
- `windows_setup_doctor_smoke`: `release_blocker` — Setup Doctor operator readability未確認。
- `windows_broker_installed_smoke`: `release_blocker` — standalone smokeはあるが、正式aggregate evidence bundle内のBroker証拠が未成立。
- `audit_anchor_external_tamper_evidence_proof`: `release_blocker` — external owner-controlled anchor/tamper proofが未成立。
- `release_ready=false`を維持。installed product総合証拠、owner GO、正式releaseは成立していない。

## R2追補 Owner Approvalの二重期限切れを有効Permissionから分離する否定試験（2026-09-29）

### 成立した局所証拠

- `native/rust_helper/src/broker/dialogue.rs`へ、Task用Owner Approvalの期限切れをWorkspace Permissionの有効状態から独立して拒否する否定試験を追加した。
- 2つの独立fixtureでApprovalのwall-clock期限だけ、またはmonotonic期限だけを失効させ、Permissionの両期限は有効に保つ。Brokerが実行を拒否し、Adapter呼出しが0回、Permission記録が残ることを確認する。
- 証拠範囲はBroker対話制御のRust `FIXTURE`に限る。実AdapterのTask実行、Owner native確認画面、production IPC、実Workspace隔離の証拠ではない。製品Adapterの`task_execution=unsupported`、関連`release_blocker`、`release_ready=false`を維持する。

### 検証と履歴

- focused試験command `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --lib 'broker::dialogue::tests::AgentTask実行は期限切れOwnerApprovalを有効Permissionから分離して拒否する' -- --exact --nocapture`: 1 passed。変更対象Rust fileの`rustfmt +1.95.0 --edition 2021 --check`と`git diff --check`も成功した。
- Windows local全target試験command `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`は2回とも全体PASSにならなかった。1回目は`adapters::minidora::tests::ContentLength付きJSONだけを期限内に取得する`が通信失敗で終了し、2回目（`GUI_SHELL_C28_DIAGNOSTICS=1`）は`a2a::tests::loopback_HTTPからAgent_Cardを取得してmetadata_onlyへ射影する`と`broker::a2a_center::tests::owner接続をBrokerで受理し通常IPC一覧へbounded射影する`が応答読取失敗となった。各失敗testは個別再実行で成功した。原因は確定していないため、通信系testの断続失敗として記録し、製品回帰・環境障害のいずれとも断定しない。
- この不確実性を対象commit上で補うため、所有者が許可した一時branchでworkflowを手動起動した。Windows Actions [run #20](https://github.com/gatchimuchio/GUI-Shell/actions/runs/36548519704)はcommit `95921dc549abb7d74386f45dadd98f8478d47b68`とcheckout SHAの一致、Windows Server 2025 image `win25-vs2026/20260922.246.2`／Rust 1.95.0、対象Rust fileのrustfmt、`cargo check --all-targets`、全Rust target test（12 target、396 passed／0 failed／1 ignored）、試験後cleanをすべて確認した。所要5分28秒、artifactなし。これは当該commitのhosted Windows Rust検査に限り、実Agent Task、実機installed product、production隔離、release readinessを証明しない。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`: exit 0。開発validator登録10項目、厳格日本語監査（1113 files／0 findings）、Schema（149／149／192 negative）、Conformance（225 checks）、Manifest、portable性、release smoke、evidence bundle、runtime assertions（12／0）、C32開発監査が成功した。evidence bundleは既存release blocker 5件と`release_ready=false`を保持した。

### 残存gate

Owner Approval期限の否定試験追加はAgent Task本番経路完成を意味しない。実Agent実行隔離、production Broker／IPC、Audit／Recovery、結果のContent Exposure、Windows installed product等の既存`release_blocker`を維持し、`task_execution=unsupported`および`release_ready=false`を維持する。

## R2追補 有効Owner Approvalが期限切れPermissionを補完しない否定試験（2026-09-29）

### 成立した局所証拠

- `native/rust_helper/src/broker/dialogue.rs`へ、Owner Approvalの本文hash・実行条件hash・二期限が有効でも、Workspace Permissionの期限切れを補完しない否定試験を追加した。
- 2つの独立fixtureでPermissionのwall-clock期限だけ、またはmonotonic期限だけを失効させ、Approval自体は本文・条件・二期限とも有効なことを照合する。Brokerは実行を拒否し、Adapter呼出し0回、Permissionと未消費Approvalの記録保持を確認する。
- 証拠範囲はBroker対話制御のRust `FIXTURE`に限る。実Adapter Task、Owner native UI、production IPC、実Workspace隔離の証拠ではない。`task_execution=unsupported`と既存`release_blocker`を維持する。

### 検証と履歴

- focused試験 `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --lib 'broker::dialogue::tests::AgentTask実行は有効OwnerApprovalを期限切れPermissionから分離して拒否する' -- --exact --nocapture`: 1 passed。
- 初回`rustfmt +1.95.0 --edition 2021 --check native/rust_helper/src/broker/dialogue.rs`は新規assertionの折返し差だけを検出したため修正し、再実行は成功。`cargo +1.95.0 check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`も成功。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`: exit 0。開発validator登録10項目、日本語厳格監査（1113 files／0 findings）、Schema（149／149／192 negative）、Conformance（225 checks）、Manifest、release gate、package portability、release smoke、evidence bundle、runtime assertions（12／0）、C32開発監査が成功した。Windows installed evidence等のrelease blocker 5件と`release_ready=false`を保持した。
- Owner許可に基づくWindows Actions [run #21](https://github.com/gatchimuchio/GUI-Shell/actions/runs/36552280495)は、一時branch上の正確なcommit `dc2ec0960421442f34e3c229451b1f4a76c105f5`をWindows Server 2025 image `win25-vs2026/20260922.246.2`／Rust 1.95.0で検査した。checkout SHA照合、対象Rust fileのrustfmt、全target `cargo check`／`cargo test`（12 target、397 passed／0 failed／1 ignored）、試験後cleanが成功した。所要6分24秒、artifactなし。成功した同一SHAを`main`へfast-forwardしてpushし、remote HEAD一致後に一時branchをlocal／remote双方から削除した。
- hosted Rust検査とfixture試験は、実Codex Agent Task、production Broker／IPC、Owner native UI、実Workspace隔離、installed product、release readinessを証明しない。

### 残存gate

有効Approvalと期限切れPermissionの分離試験は、Task実行隔離・production runtime・release readinessを証明しない。Codex Adapter metadataの`unsupported`、R2のTask／sandbox／Audit／Recovery／Content ExposureおよびWindows installed productの既存`release_blocker`、`release_ready=false`を維持する。

## R2追補 Owner拒否時にAgent Task権限を発行しないnative relay否定試験（2026-09-29）

### 成立した局所証拠

- Rust Desktop launcherのloopback Broker縦断fixtureへ、Agent Task Workspace PermissionとOwner Approvalそれぞれのnative確認に対するOwner拒否を追加した。
- Permission拒否と、本文を含まないhash表示のApproval拒否は、Owner専用IPCへ転送されず通常Broker要求へ戻り、どちらも`desktop_native_owner_confirmation_required`で拒否される。Broker Auditに両operationと拒否codeが残り、Approval拒否のTask本文は確認表示・responseへ出ない。
- 証拠はtest用loopback Brokerと合成Owner確認callbackを使う`FIXTURE`である。実Windows確認dialog、認証済みinstalled Desktop、実Agent Task、sandbox、release readinessを証明しない。`task_execution=unsupported`と既存`release_blocker`を維持する。

### 検証と履歴

- 対象を絞った試験 `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --lib 'desktop_launcher::tests::desktop_owner_allowlist_requires_native_confirmation_and_broker_audits_both_outcomes' -- --exact --nocapture --test-threads=1`: 1件成功。
- `cargo +1.95.0 check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`: 成功。追加したRust blockは`rustfmt +1.95.0 --edition 2021 --emit stdout`の該当範囲と一致した。`desktop_launcher.rs`全fileの`rustfmt --check`は変更していない複数箇所の既存整形差も検出してexit 1となったため、一括再整形は行わない。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`: exit 0。開発validator登録10項目、日本語厳格監査（1113 files／0 findings）、Schema（149／149／negative 192）、Conformance（225 checks）、Manifest、release gate、package portability、release smoke、evidence bundle、runtime assertions（12／0）、C32開発監査が成功した。Windows installed evidence等の既存release blocker 5件と`release_ready=false`を保持した。
- Owner許可によるWindows Actions [run #22](https://github.com/gatchimuchio/GUI-Shell/actions/runs/36556239415)は、一時branch上の正確なcommit `364552dfe118ccf5e6a49650bf992712dde74931`をWindows Server 2025系runner `windows-2025-vs2026`／Rust 1.95.0で検査した。checkout SHA照合、workflowに固定されたrustfmt対象群、全target `cargo check`／`cargo test`（12 target、397件成功／0失敗／1 ignored）、試験後cleanが成功した。所要5分34秒、artifactなし。新規変更箇所の整形は該当範囲で確認済みだが、workflowの固定rustfmt対象群に`desktop_launcher.rs`は含まれず、全fileの`rustfmt --check`は既存差分を含むため成功した扱いにしない。PASSした同一SHAを`main`へfast-forward・pushし、remote HEAD照合後に一時branchをlocal／remote双方から削除した。

### 残存gate

Owner拒否時のloopback relay拒否はnative dialogの実操作やproduction installed pathの証拠ではない。実Owner Yes／No経路、実Codex `exec`と隔離、取消／期限／crash後Recovery、Audit／Content Exposureの製品縦断、Windows installed productを未成立の`release_blocker`として保持し、`release_ready=false`を維持する。

## R2追補 native確認後も未登録WorkspaceへTask権限を発行しないrelay縦断試験（2026-09-29）

### 成立した局所証拠

- Rust Desktop起動器のloopback Broker fixtureで、Agent Task Workspace PermissionとOwner Approvalのnative確認callbackが肯定を返しても、未登録WorkspaceをBrokerが`作業領域不在`として拒否する試験を追加した。Owner専用process内IPCへ進んだ後のBroker再検証まで通す一方、通常request、metadata、native確認だけでは権限を発行しない。
- Task本文はhashだけをnative確認summaryへ射影し、responseとAuditに現れないことも確認する。先行する同一fixtureのOwner拒否では、両操作がOwner専用IPCへ進まず通常Broker経路で拒否される。
- 証拠はloopback Brokerと合成確認callbackを使う`FIXTURE`である。Workspace登録、実Windows確認dialog、実Agent Task、Task実行、sandbox、release readinessを証明しない。`task_execution=unsupported`と既存`release_blocker`を維持する。

### 検証と履歴

- 対象を絞ったDesktop起動器試験は1件成功、`cargo +1.95.0 check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`も成功した。追加blockは`rustfmt +1.95.0 --edition 2021 --emit stdout`の該当範囲と一致する。全fileの`rustfmt --check`は変更していない既存箇所の整形差で失敗するため、file全体は再整形しない。
- 初回focused試験では、未登録Agentが先に拒否されるという期待に対し、実際は上位のWorkspace結合検査が先に`作業領域不在`で拒否した。製品実装を変えず、fail-closedの実際の検査順を試験期待値へ反映して再試験成功した。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`は10開発検査すべて成功し、strict Japanese auditは1113 file／指摘0、Schemaは149/149とnegative 192件、Conformanceは225件成功した。release blocker 5件と`release_ready=false`は維持する。
- Windows Actions [run #23](https://github.com/gatchimuchio/GUI-Shell/actions/runs/36558967124)は一時branch上のcommit `8b31153952d077ef993f46c1e3adc7f9b5189419`をWindows Server 2025 image `windows-2025-vs2026/20260922.246.2`／Rust 1.95.0で検査した。checkout SHA照合、workflow固定対象のrustfmt、全target cargo check／test（12 test target、397 passed／0 failed／1 ignored）、試験後cleanが成功し、artifactなし。run所要6分33秒。`desktop_launcher.rs`はworkflow固定rustfmt対象外であり、新規追加範囲の局所rustfmt照合のみ実施した。PASSした同一SHAを`main`へfast-forward／pushし、remote HEAD一致を確認後、一時branchをlocal／remote双方から削除した。
- 初回focused testの期待値違いは実装欠陥ではなく、BrokerのWorkspace結合検査がAgent registry検査より先に`作業領域不在`で拒否する実順序だった。期待値を実装のfail-closed順序に合わせた後のfocused testとActions全target testが成功した。

### 残存gate

肯定callbackは実Owner dialogではなく、Workspaceも未登録であり、Brokerがgrantを拒否するfixtureである。run #23はhosted Rust検査の証拠に限る。実Owner Yes／No、登録済みsupported AdapterでのPermission／Approval発行、実Codex `exec`と隔離、取消／期限／crash後Recovery、Audit／Content Exposureの製品縦断、Windows installed productは未成立の`release_blocker`として保持し、`task_execution=unsupported`と`release_ready=false`を維持する。

## R2追補 Agent Task deadline停止をOwner取消と区別する（2026-09-29）

### 成立した局所証拠

- Broker workerの内部受信結果へ単調完了時刻を付け、Brokerが結果をpollした時刻ではなく、Task deadlineに対する実際の完了時刻でterminal結果を判定する。期限到達時には受信済み結果を先に処理し、未着の場合だけ停止を要求する。
- deadline前に完了したOwner取消結果は`cancelled`として保持し、deadline後に届いた取消応答または成功結果は`failed`／`期限超過`へ分類する。停止を確認できない`通信失敗`はdeadlineで覆い隠さない。Codex CLI Adapterもdeadlineをcancel flagより先に判定する。
- Rust単体試験は期限前Owner取消、deadline上の取消、期限後成功、期限後の停止不能を区別する。Windows fake CLI Broker縦断fixtureは成功・明示取消・deadline停止を通し、deadline停止時に子孫process停止、結果hash不採用、scratch cleanup、Task本文非露出を確認する。fixtureは内部Broker recordのdeadlineを短縮しており、実時間の900秒deadline、実Codex Task、production隔離の証拠ではない（証拠class: `FIXTURE`）。
- `task_execution=unsupported`、既存`release_blocker`、`release_ready=false`を維持する。この変更はAgent Taskのproduction execution、OS process群の任意条件下での強制停止、installed product、release readinessを成立させない。

### 検証と履歴

- `rustfmt +1.95.0 --edition 2021 --check native/rust_helper/src/broker/dialogue.rs native/rust_helper/src/adapters/codex_cli.rs`: 成功。
- `cargo +1.95.0 check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`: 成功。
- `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`: exit 0、398 passed／0 failed／1 ignored。これはWindowsローカル全target試験である。
- Windows fake CLI Broker縦断fixtureの単独実行: 1 passed。AgentTask focused試験: 13 passed。`git diff --check`: 成功。
- 最初の`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`は変更3 fileのManifest hash未更新によりManifest／release gate／package portabilityが失敗した。またpackage portability内のsource ZIP Conformanceが120秒でtimeoutし、そのZIPには`.git`がないためConformanceの`git ls-files` probeが`fatal: not a git repository`を出した。Cargo全target試験との同時実行下だったが、timeoutの根本原因は未確定としてこの失敗履歴を保持する。
- 1,110 fileのManifestを再生成した後、Cargo試験を並行させず同じ統合validatorを再実行しexit 0。strict日本語監査（1113 files／0 findings）、Schema（149 schema／149 example／192 negative fixture）、Conformance（225 checks）、Manifest、development release gate、package portability（source ZIP内Conformanceを含む）、release smoke、evidence bundle、runtime assertions（12成功／0失敗）、C32開発監査が成功した。source ZIPで`git ls-files` probeが出すfatal文言は残るが、この再実行ではpackage portabilityと統合validatorが成功したため、前回timeoutとの因果は立証していない。evidence bundleは既存release blocker 5件、`release_ready=false`を維持した。
- Windows Actions [run #3](https://github.com/gatchimuchio/GUI-Shell/actions/runs/36560909604)は変更前の`main` commit `f7c74f014650caf451abc8e8c758ef92e4ebe5fc`に対するbaselineであり、本変更の検証ではない。`workflow_dispatch`のWindows Server 2025 runnerでFlutter Desktop/Rust helper build、Desktop analyze/all tests、Mobile analyze、試験後cleanが成功し、artifactはない。所要7分09秒。今回のRust差分にはActions証拠を付けていない。

### 残存gate

Worker結果時刻のfixtureとAdapter試験は、実Agent executionのdeadline enforcementや全OS process descendantの実環境停止保証ではない。実Codex `exec`／隔離、cancel・deadline・crash後Recovery、production Audit／Content Exposure、Windows installed productの既存`release_blocker`を維持し、`task_execution=unsupported`および`release_ready=false`を維持する。

## R2追補 Owner取消受理と競合するAgent Task成功応答を採用しない（2026-09-29）

### 成立した局所証拠

- 期限完了時刻の再監査で、Owner取消flagをworkerの最終hash化前に検査した後、Brokerが結果を受け取るまでに取消要求が成立する競合を特定した。Broker進捗反映が取消受理時刻を知らないまま成功hashを採用し得る境界だった。
- BrokerはOwner取消Auditの成功後に単調取消受理時刻を内部Task recordへ記録する。workerの単調完了時刻が取消受理時刻以後なら成功結果を採用せず、worker終端を確認した後に`cancelled`とする。取消受理より前にworkerが完了済みなら、後続poll遅延だけを理由に完了結果を書き換えない。deadline判定を先に適用するため、期限後の結果は引き続き`failed`／`期限超過`となる。
- 決定論的試験で、Owner取消受理後の成功を取消へ分類し、取消受理前に完了済みの成功を維持する。既存Windows fake CLI Broker縦断fixtureは成功・明示取消・期限停止、子孫process停止、result hash不採用、scratch cleanup、Task本文非露出を検査する。これはRust内の時刻境界試験とfake CLI `FIXTURE`に限り、実Codex Taskやinstalled productの証拠ではない。
- `task_execution=unsupported`、既存`release_blocker`、`release_ready=false`を維持する。

### 検証と履歴

- `rustfmt +1.95.0 --edition 2021 --check native/rust_helper/src/broker/dialogue.rs native/rust_helper/src/adapters/codex_cli.rs`: 成功。
- `cargo +1.95.0 check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`: 成功。
- `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`: exit 0、398 passed／0 failed／1 ignored。Windows hostの全target試験であり、実Codex Agent Taskのproduction実行証拠ではない。
- `python -X utf8 tooling/manifest.py --write`でManifest 1110 fileを再生成し、`python -X utf8 tooling/manifest.py --check`と`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`がいずれもexit 0。この変更を含む統合検査は11項目すべて成功。strict日本語監査（1113 files／0 findings）、Schema（149／149／192 negative）、Conformance（225 checks）、package portability、release smoke、evidence bundle、runtime assertions（12成功／0失敗）、C32開発監査が成功した。既存release blocker 5件と`release_ready=false`を保持。

### 残存gate

時刻境界試験はcancel受理後に成功応答が競合する分類を検証する`FIXTURE`であり、実Codex `exec`、全OS process群の強制停止保証、実Workspace隔離、production Audit／Recovery、Windows installed productを証明しない。関連`release_blocker`、`task_execution=unsupported`および`release_ready=false`を維持する。

## R2追補 MxC shell childへscratch環境変数を固定できるか再検査（2026-09-29）

### 観測

- 既存の資格情報なしloopback偽Responses API probeを拡張し、Codex設定`shell_environment_policy.set`にも合成WorkspaceTaskScratch pathを`TEMP`／`TMP`として指定して、明示指定Codex CLI `0.158.0-alpha.2.1`の実`codex exec`とMxC shell childを3回実行した。
- 3/3回ともturnと固定shell commandは正常終了し、shell childのTEMP／TMPは互いに一致したが、設定したscratchとは一致しなかった。観測された値はPackages配下の`AC/Temp`形式で、絶対pathは保存しない。合成TEMP markerはchild内で書けたが、Codex終了後hostから見えなかった。これは物理削除の証拠ではない。
- 偽probe用modelはmodel metadata catalogに存在せず、Codexはfallback metadataを使う旨のwarning eventを出した。warningはprobe出力へ残し、固定commandとturnが成功した事実とは区別する。実modelでの挙動やfallback品質を証明しない。
- 追加設定は診断probe内だけであり、製品Adapter設定や永続Codex設定を変更していない。credential・実model・Rust Broker・Owner Approval・製品Task経路・取消／期限／crashは未使用。loopback以外の通信はproxyで拒否した。
- 証拠classは当該CLI／当該Windowsでの`LIVE_RUNTIME`限定観測。MxC child TEMP/TMPの設定機構、scratchとの不一致理由、異常終端時のchild一時領域cleanup、Rust Brokerからの連続Task経路は未解決である。Agent Adapterの`task_execution=unsupported`、関連`release_blocker`、`release_ready=false`を維持する。

### 検証

- `python -X utf8 tooling/codex_mxc_exec_temp_probe.py --exe <Owner指定のcodex.exe絶対path> --runs 3`: 成功、3/3回で同じ不一致を観測。
- `python -m py_compile tooling/codex_mxc_exec_temp_probe.py`と`python -m json.tool release_blockers.registry.json`: 成功。
- warning本文をprobe出力へ残す変更後の最初の再実行は、`--runs 3`の1回目でResponses API要求1件の後に`stream disconnected before completion: error sending request`となり、exit 1だった。proxyは非loopback接続を拒否し、server側stream write exceptionは0件。原因は未確定として失敗履歴を保持する。
- 同じ最終probeを順次再実行した`python -X utf8 tooling/codex_mxc_exec_temp_probe.py --exe <Owner指定のcodex.exe絶対path> --runs 3`はexit 0、3/3回でturnと固定commandが正常完了し、TEMP／TMP不一致とfallback metadata warningを再観測した。非loopback requestは計12件をloopback proxyで拒否。先行probe runおよび後続single-run retryも成功したが、stream断の根本原因は確定していない。
- `python -X utf8 tooling/manifest.py --write`は1110 fileを再生成し、`python -X utf8 tooling/manifest.py --check`が成功。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`: exit 0、登録済み全development check成功。strict日本語監査1113 files／0 findings、Schema 149／149・negative fixture 192、Conformance 225 checks、Manifest、release gate、package portability、release smoke、evidence bundle、runtime assertion 12成功／0失敗、C32監査が成功。release blocker 5件、`release_ready=false`は維持された。

## R2追補 MxC deny globと登録secret exact pathのread／write境界（2026-09-29）

### 観測

- 実Codex CLI `0.158.0-alpha.2.1`、実MxC `exec_command` child、loopback偽Responses API、資格情報なし、一時`CODEX_HOME`と合成Workspaceだけを使うprobeへ、通常file・`.env`／`.env.production`・`.ssh`・`secrets`・Rust生成相当の登録exact file／directory・Workspace外markerのread／write試験を追加した。初回の試験版はglob denyにもwrite拒否を要求してexit 1となったが、公式仕様と異なる期待値だったため、globとexact登録pathを別の境界として判定するよう補正した。この失敗履歴は保持する。
- 最終probeは3/3回成功。通常Workspace markerのreadは許可、`.env`、`.env.production`、`.ssh`、`secrets`のsynthetic readは各3/3回拒否された。一方、これらdeny globにmatchするsynthetic writeは各3/3回許可され、hostからmarker fileを確認した。Workspace外markerのread／writeはいずれも3/3回拒否された。
- Codexの公式`Permissions`仕様は、`:workspace_roots`下のglob `deny`をdeny-read ruleと説明する。従って上記write許可は当該仕様と整合し、既定globだけをwrite隔離として扱ってはならない。Owner登録secret pathをRustがliteral exact `deny`として生成する経路は別であり、synthetic登録fileのread／新規作成、登録directory descendantのread／新規作成を各3/3回拒否し、Workspace通常writeを許可した。
- 続けてRustの既存ignored Windows live testを拡張し、`build_codex_command`が実際に生成するconfig overridesを直接`codex sandbox`へ渡して、登録exact file／directory descendantのwrite否定を加えた。Windows local testは1 passed／0 failed。これはRust生成設定と実Codex sandbox childの直接`LIVE_RUNTIME`であり、Broker／Owner Approval／production `codex exec` Taskへ接続した証拠ではない。
- 追加probeはTEMP／TMP mismatchも再観測した。いずれの検査もsynthetic markerだけを使用し、Codex資格・実model・実secret・永続Codex設定・OS保護設定の変更はない。`task_execution=unsupported`、該当`release_blocker`、`release_ready=false`を維持する。

### 検証

- 期待値補正前の`python -X utf8 tooling/codex_mxc_exec_temp_probe.py --exe <Owner指定のcodex.exe絶対path> --runs 1`はexit 1。合成glob pathへのwriteをdenyと誤期待した検査不一致であり、Codex CLIの実挙動は観測結果に保存した。
- 補正後の同probe `--runs 1`と`--runs 3`はexit 0。最終3回では各glob read拒否／glob write許可、登録exact fileとdirectory descendantのread／write拒否、Workspace外read／write拒否が一致した。
- `GUI_SHELL_CODEX_SANDBOX_TEST_EXE`へOwner指定CLIを設定して実行した`cargo +1.95.0 test --manifest-path native/rust_helper/Cargo.toml Rust生成Task設定で実Windows隔離の登録secretを拒否する -- --ignored --nocapture`: 1 passed／0 failed。Cargoが他targetも起動したが、対象filter外testは実行していない。
- `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`: 398件成功／0件失敗／1件は`#[ignore]`指定のため未実行。
- 外部のWindows Actionsは今回未使用。ローカルWindows上で実CLIと実MxC childを実行できたため、補助hosted検査を必要としなかった。
- 最終ソースを含む`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`はexit 0。strict日本語監査1113 files／0 findings、Schema 149／149・negative fixture 192、Conformance 225 checks、Manifest、release gate、package portability、release smoke、evidence bundle、runtime assertion 12成功／0失敗、C32監査が成功。release blocker 5件、`release_ready=false`は維持された。

## R2追補 登録secretのNTFS hardlink alias拒否（2026-09-29）

### 観測と実装

- 完全一致の登録secret pathへMxC denyを設定しても、sandbox起動前に作成したNTFS hardlink aliasから合成secretを読め、旧probeはexit 45となった。path文字列単位のdenyだけでは同一fileの別名を保護しない。
- `WorkspaceReader::from_registered_dir`は登録secret fileと登録secret directory以下をhandle経由でbounded走査する。hardlink（regular fileのlink countが1以外）、reparse／volume境界等のunsafe entry、走査深さ64または合計4096 entryの上限超過をfail-closedで拒否する。未作成の登録pathは将来作成用として許可する。Codex AdapterはTask起動直前にpin済みroot identityを再照合して同じ検査を行い、登録後・spawn前のalias追加も拒否する。Rust unit testは`FIXTURE`である。
- 初回の動的作成追試はnested `cmd.exe`を起動できずexit 47で判定不能だった。その後、Rustが生成したTask permission overrideを用いる実Codex CLI `0.158.0-alpha.2.1`の直接MxC sandbox内で、PowerShellから合成secretのhardlink alias作成を試した。`New-Item -ItemType HardLink`は`UnauthorizedAccessException`、HRESULT `0x80070005`で拒否され、通常Workspace writeは成功した。これは直接sandbox childの`LIVE_RUNTIME`観測であり、実`codex exec` tool child／Broker／Owner Approval経路の証拠ではない。
- 同じ直接probeで登録secretの深さ40 pathおよび大文字・区切りの異なるpath aliasのread／writeを拒否した。MxC childのTEMP／TMPはRust `WorkspaceTaskScratch`と一致しなかった。synthetic markerだけを使用し、資格情報・実model・永続Codex設定・OS保護設定は変更していない。
- pre-existing alias迂回、Rust registration／Task preflight、直接MxC childでの新規alias作成拒否は異なる証拠である。Broker経由の実Task・tool child隔離、TEMP／TMPの不一致、cancel／deadline／crash、Audit／Recovery、scratch cleanupが未成立のため`task_execution=unsupported`、関連`release_blocker`、`release_ready=false`を維持する。

### 検証履歴

- `rustfmt +1.95.0 --edition 2021 --config skip_children=true --check native/rust_helper/src/adapters/codex_cli.rs`: exit 0。`workspace_reader.rs`全体には既存整形差があり、無関係な大量変更を避けて再整形していない。
- `cargo +1.95.0 check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`: 成功（終了コード0）。
- `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`: 12対象で391件成功、失敗0件、1件は無効化指定のため未実行。
- 明示起動したWindows ignored live test `Rust生成Task設定で実Windows隔離の登録secretを拒否する`: 1 passed／0 failed。観測範囲は直接`codex sandbox`と合成Workspaceまで。
- validator初回は記録文2件の日本語監査指摘と、新規fixtureの`std::fs::write`禁止patternで失敗した。文面を日本語基底へ直し、fixtureをFile作成と`write_all`へ変更後、厳格監査1113 files／0 findings、Conformance 225 checksを個別再実行して合格した。
- 統合validatorは初回に記録文と診断表示文の日本語監査、および新規fixtureの禁止patternを検出した。修正後はManifestが古く停止したため再生成し、最終の`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`は終了コード0で合格した。これは開発検証であり、release blocker 5件と`release_ready=false`を変更しない。
- Windows Actionsは未使用。現Windows hostでRust全targetと限定MxC実測を実行できたため、hosted補助検査は追加しなかった。

## R2追補 実`codex exec` MxC childで登録secretのhardlink作成を検査（2026-09-30）

### 観測

- 既存のdevelopment-only loopback偽Responses API probeを拡張し、実Codex CLI `0.158.0-alpha.2.1`の`exec`が固定`exec_command`を実MxC shell childで実行する間に、合成Workspaceの登録secret fileから未登録aliasへの`New-Item -ItemType HardLink`を試みた。隔離`CODEX_HOME`、合成Workspace、資格情報なしで実行し、偽API以外への通信要求12件はloopback proxyが拒否した。Codex processは非管理者として動作した。
- 3回の連続実行はすべて正常終端し、hardlink作成は3/3回 `Win32Exception`／HRESULT `0x80004005`／Win32 `NativeErrorCode 5`（アクセス拒否）で失敗した。alias経由readは作成失敗のため未実行で、alias fileは3/3回host側に存在しなかった。通常Workspace read／writeは許可された。hardlink作成中に合成secret本文を出力・保存していない。
- 初回matcherはHRESULT `0x80070005`のみを認識し、MxCが返したHRESULT wrapper `0x80004005`とNativeErrorCode 5の組を誤って未分類として1回目をfailにした。例外のnative codeも限定記録するよう補正後、single run 1/1および連続run 3/3が同じアクセス拒否で成立した。この初回は観測欠落であって、alias作成成功を意味しない。
- 既存probeのTEMP／TMPは引き続き互いには一致するがWorkspaceTaskScratchとは一致しない。child内TEMP markerは書け、CLI終了後hostから見えなかった。host非可視を物理削除保証へ昇格しない。
- これは手動構成したRust相当permission profileを使う直接Codex CLI／MxC childの`LIVE_RUNTIME`証拠である。Rust Adapterの生成値、Broker、Owner Approval、production Agent Task、実Workspace登録、cancel／deadline／crash、Audit／Recovery、他CLI版と別alias形式は通していない。`task_execution=unsupported`、関連`release_blocker`、`release_ready=false`を維持する。

### 検証

- `python -X utf8 -c "from pathlib import Path; p=Path('tooling/codex_mxc_exec_temp_probe.py'); compile(p.read_text(encoding='utf-8'), str(p), 'exec')"`: 成功。
- `python -X utf8 tooling/codex_mxc_exec_temp_probe.py --exe <Owner指定codex.exe絶対path> --runs 1`: 初回は過度に狭いHRESULT matcherでexit 1。native error記録を加えた後は1/1成功、アクセス拒否・alias不在。
- 同probe `--runs 3`: exit 0、3/3でCLI turn／固定command完了、hardlink createはnative error 5、aliasなし。非loopback要求12件拒否。
- `git diff --check`: 成功。統合validator、manifest確認は変更完了前に実行する。
- Windows Actionsは未使用。Rust sourceを変更せず、実CLI/MxC childのWindows局所probeを実行可能だったため hosted Rust検査の追加は不要。

## R2追補 実`codex exec` tool childにおける起動前hardlink alias読取（2026-09-30）

### 観測

- 同じloopback偽Responses API／実Codex CLI `0.158.0-alpha.2.1`／実MxC child probeに、起動前alias read検査を追加した。合成登録secret fileをPython `os.link`でCLI起動前にhardlink化し、`samefile`とlink count 2を確認する。alias pathは静的permission設定でdenyされていない。secret本文はprobe内で読み捨て、出力しない。
- 個別`--runs 1`の完了turn 3件すべてで、登録exact path readは拒否された一方、起動前alias readは`allowed`、host aliasも存在した。動的hardlink作成は別の登録exact-deny synthetic targetに対して3/3回 `Win32Exception`／HRESULT `0x80004005`／Win32 `NativeErrorCode 5`（アクセス拒否）で失敗し、dynamic aliasは作成されなかった。通常Workspace read／writeも成功した。
- これはpath完全一致denyだけでは既存hardlink aliasを塞がず、Rust `WorkspaceReader`登録時とTask起動直前のlink-count検査が実shell child前に必要であることを確認する限定`LIVE_RUNTIME`観測である。直接CLI probeは合成hardlinkを事前作成し、Rust registration／preflightを意図的に通さずにMxCだけを検査する。従って、登録／preflightを含むproduction Broker Taskでread可能になる証拠ではない。先行Rust fixtureの複数link拒否と、この直接child結果は別証拠として保持する。
- `--runs 3` batch実行は2回とも、その最初のCodex invocationでshell command自体はexit 0となった後、Responses streamが`error decoding response body`で切れてCLI turnは失敗した。これらを正常turn数へ加算しない。個別`--runs 1`を3回行い、3件ともturnまで正常完了した。全試行で実model／資格情報なし、非loopback要求はproxyで拒否、Codex processは非管理者。TEMP／TMP mismatchとhost非可視をphysical cleanupとしない制約は継続する。
- `task_execution=unsupported`、関連`release_blocker`、`release_ready=false`を維持する。Rust Broker、Owner Approval、Rust生成config、production Agent Task、取消／deadline／crash、Audit／Recoveryは本probeに含まれない。

### 検証

- `python -X utf8 -c "from pathlib import Path; p=Path('tooling/codex_mxc_exec_temp_probe.py'); compile(p.read_text(encoding='utf-8'), str(p), 'exec')"`: 成功。
- `python -X utf8 tooling/codex_mxc_exec_temp_probe.py --exe <Owner指定codex.exe絶対path> --runs 1`: exit 0、正常turn 3/3。各回で起動前alias read許可、registered exact read拒否、dynamic creationはnative error 5。
- 同probe `--runs 3`: 2回の試行はいずれも最初のCLI invocationでturn stream decode失敗。tool childはexit 0して報告fileを書いたが、turn完了要件を満たさないためprobe全体はexit 1。これらは成功反復へ含めない。
- `git diff --check`: 成功。統合validatorとManifest確認は編集完了前に実行する。
- Windows Actionsは未使用。local Windows上の実CLI／MxC childを限定実行でき、Rust sourceを変更していないため。

## R2追補 Broker Workspace登録入口でsecret hardlink aliasを拒否（2026-09-30）

### 成立した確認

- `WorkspaceRegistry::register`へ、登録対象の合成secret fileと同一NTFS fileを指す事前作成hardlink aliasを渡すRust fixtureを追加した。Broker登録入口が`WorkspaceReader::from_registered_dir`の検査結果を登録拒否へ変換し、登録entryを残さず、成功登録Audit callbackも呼ばないことを確認する。
- これは`WorkspaceReader`単独とCodex AdapterのTask事前検査だけでなく、Broker registry登録関数を直接通した`FIXTURE`証拠である。合成本文は試験だけで使用し、実秘密、実Codex CLI、Broker IPC、Owner Approval、製品Taskは使用しない。
- 既存のTask事前検査fixtureとWorkspaceReader登録検査も同じfocused実行で再通過した。MxC直接`codex exec` childでの起動前alias読取可能性は、登録／preflightを通さない別の`LIVE_RUNTIME`観測として維持し、両者を混同しない。
- このfixtureはhardlinkを含む登録対象secretについてfail-closed経路を確認するが、登録後に生じるfile差替・別alias形式の網羅、Task実行時のAdapter起動阻止、production IPC／Audit永続化、process lifecycle、Recovery、実Agent隔離を証明しない。`task_execution=unsupported`、関連`release_blocker`、`release_ready=false`を維持する。

### 検証

- `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --lib secret_hardlink_alias -- --test-threads=1`: 2件合格、0件失敗。
- `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`: 402件合格、0件失敗、1件は明示除外。全test targetを実行し、明示除外された実Codex CLI probeだけは実行していない。
- リポジトリ全体の`cargo +1.95.0 fmt --manifest-path native/rust_helper/Cargo.toml -- --check`: 失敗。今回のtest fileを含む広範な既存Rust fileの整形差分を検出したため、無関係な全体整形は適用しない。今回追加した関数はrustfmt出力に合わせて整形した。手動Windows Rust workflowの固定rustfmt対象にもこの既存test fileは含まれない。
- `cargo +1.95.0 check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`: 終了値0。
- `python -X utf8 tooling/manifest.py --check`: 合格。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`: 最終状態で終了値0。厳格日本語監査1113 file／0 findings、Schema 149／正常example 149／negative fixture 192、Conformance 225 checks、登録済みdevelopment check 10件が合格した。release blocker 5件と`release_ready=false`は維持された。初回実行ではこの記録中の英語検証結果表記を日本語化する検出が1件あり、修正後の再実行で解消した。
- Windows Actionsは未使用。現行Windows上でRust全target試験が実行可能であり、hosted runnerによる追加証拠は本作業範囲に必要ない。

## R2追補 Codex Adapterの実Task入口でhardlink aliasを起動前拒否（2026-09-30）

### 成立した確認

- 登録後に合成secret fileへのhardlink aliasを追加し、既存の事前検査helperに加えて`CodexCliAdapter::AgentTask実行`本体を直接呼ぶWindows Rust fixtureへ拡張した。対応可能な試験用Adapterとsynthetic Task contextを使い、実行結果が`作業領域不在`となることを確認する。
- Adapterの実行fileには意図的に不存在の合成pathを渡す。期待した拒否結果が返ることで、Task scratch作成後またはCLI起動時の別エラーではなく、AgentTask入口のsecret再検査で停止したことを区別する。合成secret本文はfixture内だけで使用する。
- 証拠classは`FIXTURE`。これはRust Adapter method内の事前検査接続を示すが、production Broker consumer／IPC、Owner Approval、Codex `exec`、実Agent、installed productの証拠ではない。実MxC childで事前alias読取が可能だった`LIVE_RUNTIME`観測は別記録として保持する。`task_execution=unsupported`、該当`release_blocker`、`release_ready=false`を維持する。

### 検証

- `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --lib '登録後に増えたsecret_hardlink_alias' -- --test-threads=1`: 1件合格、0件失敗。
- `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`: 402件合格、0件失敗、1件は明示除外。
- `cargo +1.95.0 check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`: 終了値0。
- `rustfmt +1.95.0 --edition 2021 --config skip_children=true --check native/rust_helper/src/adapters/codex_cli.rs`: 合格。
- `python -X utf8 tooling/manifest.py --check`: 合格。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`: 最終作業状態で終了値0。厳格日本語監査1113 file／0 findings、Schema 149／正常example 149／negative fixture 192、Conformance 225 checks、登録済みdevelopment check 10件が合格した。release blocker 5件と`release_ready=false`は維持された。
- Windows Actionsは未使用。対象Rustの全target検査を現行Windows上で完了可能だった。

## R2追補 scratch journal有効化失敗時の未起動領域回収（2026-09-30）

### 成立した変更

- Agent Task scratch journalの`activate`は、耐久保存に失敗した場合もメモリ上のentryを`active`へ変えたままにしていた。永続store側はatomic write失敗前の`reserved`を保持するため、稼働中Brokerと再起動後で記録状態がずれ得る。失敗時に変更前のbounded stateへ戻す。
- scratch directoryを開いた後のmetadata取得またはjournal有効化が失敗した場合、Task process起動前に、directory handleから当該未起動scratchを回収する。削除を確認できた場合だけ予約記録を完了する。回収不能時はrecordを保持し、未知のpathを推測削除しない。
- Adapter metadataは`task_execution=unsupported`のまま。MxC childのTEMP／TMP不一致、実BrokerからCodex CLIを起動する`LIVE_RUNTIME`検証およびrelease blockerは解消していない。

### 検証

- persistent journalのactivate耐久保存失敗fixtureで、メモリ状態と再読込後の永続状態がともに`reserved`へ戻ることを検査する。
- 未起動scratchのjournal有効化失敗fixtureで、handle経由のdirectory回収と記録解消を検査する。証拠classは`FIXTURE`であり、process crash後やinstalled productのRecovery証拠ではない。
- `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`: 合格、404 passed／0 failed／1 ignored。対象はRust helperのunit・integration testであり、Codex CLIの実broker起動やchild process isolationの`LIVE_RUNTIME`証拠ではない。
- `cargo +1.95.0 check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`: 合格。
- adapter focused testの初回は、test fixtureが開いたworkspace handleをroot削除前に閉じておらず、Windows sharing violationで終了した。fixtureを修正して再実行し合格。製品処理の失敗ではない。
- 初回の統合検査は追加試験の英語diagnostic 5箇所を厳格日本語監査が検出して失敗した。5箇所を日本語化し、`python -X utf8 tooling/日本語基底監査.py --strict`を再実行して1113 file／0 findingsで合格した。日本語fixture内容はRustのUTF-8文字列からbytesへ渡す形にし、focused Rust tests 2件も再合格。
- `rustfmt +1.95.0 --edition 2021 --config skip_children=true --check native/rust_helper/src/adapters/codex_cli.rs native/rust_helper/src/broker/agent_task_scratch.rs`: 合格。既存assertion 1箇所の改行だけをrustfmt準拠にした。
- `python -X utf8 tooling/manifest.py --check`および`git diff --check`: 合格。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`: 合格。日本語基底監査1113 file／0 findings、Schema 149／正常example 149／negative fixture 192、Conformance 225 checks、登録development check 10件。証拠bundleはrelease blocker 5件と`release_ready=false`を保持した。Windows installed-app evidenceは未収集であり、release gateは閉じない。
- Windows GitHub Actionsは未使用。local Windows Rust all-target検査を実行できたため、検査branchやActions artifactは作成していない。

## R2追補 MxC child TEMP／TMP経路の一次資料照合（2026-09-30）

### 成立した確認

- 実測対象Codex CLIと同じ`rust-v0.158.0-alpha.2.1`の公開sourceを確認した。`mxc-sandbox/src/windows.rs`はfilter済みlauncher環境を読み、`TEMP`／`TMP`を含むenvironmentをpolicy builderへ渡す。`policy.rs`はこの値を`:tmpdir`のfilesystem grantへ射影し、同じenvironmentを`ExecutionRequest`へ載せる。`native.rs`は`BaseContainerRunner`へ実行要求を渡す。これは当該CLI版の実装sourceに対する`EXTERNAL_EVIDENCE`であり、起動後childの実環境が渡した値を維持することの証明ではない。
- Microsoft LearnのAppContainer資料は、AppContainer profileにおける`TEMP`／`TMP`の配置例として`Packages\<profile>\AC\Temp`へのredirectを説明する。これは2026-09-29の直接CLI `LIVE_RUNTIME`観測（指定したWorkspace scratchとは異なるPackages配下`AC\Temp`）と整合する候補要因だが、当該Codex／MxC実行でそのOS動作が不一致の原因であること、実Tempのprofile単位・task単位の寿命、物理cleanupを確定しない。
- したがって、原因はなお未確定で、shell environment overrideがWorkspaceTaskScratchをchildの実Tempへ固定する契約として使えるとは扱わない。Adapterの`task_execution=unsupported`、scratch隔離／cleanupを含む`release_blocker`、`release_ready=false`を維持する。追加runtime probeは行わず、AppContainer tempへ合成markerを残す恐れのある同型試験も反復していない。

### 参照と証拠境界

- [Codex CLI 0.158.0-alpha.2.1のMxC policy実装](https://github.com/openai/codex/blob/rust-v0.158.0-alpha.2.1/codex-rs/mxc-sandbox/src/policy.rs)、[Windows起動処理](https://github.com/openai/codex/blob/rust-v0.158.0-alpha.2.1/codex-rs/mxc-sandbox/src/windows.rs)、[native実行処理](https://github.com/openai/codex/blob/rust-v0.158.0-alpha.2.1/codex-rs/mxc-sandbox/src/native.rs)。
- [Microsoft LearnのAppContainer起動手順](https://learn.microsoft.com/en-us/windows/win32/secauthz/implementing-an-appcontainer)。
- source読解は`EXTERNAL_EVIDENCE`。前項の直接CLI probeは合成入力の`LIVE_RUNTIME`のままであり、Rust Broker、Owner Approval、Rust生成scratch、実Agent Task、process群停止、Audit／Recoveryの証拠へ昇格しない。製品code・Windows設定・Task capabilityは変更していない。

## R2追補 信頼鍵を持たない更新署名入口のfail-closed化（2026-09-30）

### 成立した変更

- 公開Rust API `verify_update_signature`は更新IDと署名文字列しか受け取らず、署名対象byteもBroker所有信頼鍵も持たないのに、以前は署名文字列が空でなければ成功を返していた。Repository内のBroker本線からは呼ばれていないが、public moduleから利用できるため、署名の存在を真正性と誤認させる入口だった。
- 互換入口は空署名を`update_signature_required`で拒否し、非空署名も`update_signer_untrusted`で拒否する。署名真正性を成功にできるのは、Broker所有公開鍵とfingerprint、署名対象byte、Ed25519署名を照合する`verify_signed_update_signature`だけである。非空の任意文字列が成功にならない否定試験を追加した。
- これはRust APIの誤用防止であり、download、install、process起動、update適用、rollbackを追加しない。更新信頼鍵のproduction provisioning、Windows installed product、`rev2_desktop_product_distribution`、`release_ready=false`は未解決のまま保持する。

### 検証

- `rustfmt +1.95.0 --edition 2021 --check native/rust_helper/src/update_verification.rs`: 合格。
- `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`: exit 0。12 targetで395 passed／0 failed／1 ignored。追加した非空署名・信頼鍵なしの否定試験を含む。
- `python -X utf8 tooling/schema_check/check_schemas.py`: Schema 149件、正常example 149件、negative fixture 192件で合格。
- `python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`: 225 checksで合格。
- `python -X utf8 tooling/日本語基底監査.py --strict`: 1113 file／0 findingsで合格。
- `python -X utf8 tooling/manifest.py --write`および`--check`、`git diff --check`: 合格。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`: exit 0。10 development checksは全件合格。Windows installed evidenceの不足5件はrelease blockerとして残り、release gateの合格表示はdevelopment validation自体の判定であってrelease readinessではない。
- Windows Actionsは未使用。現行Windows hostでRust全targetと統合validationを実行できたため、追加hosted検査は不要だった。

## R2追補 Broker制御経路から実Codex CLIを実起動（2026-09-30）

### 成立した確認

- Windows ignored Rust試験を明示起動し、Owner指定のCodex CLI `0.158.0-alpha.2.1`を一時`CODEX_HOME`、資格情報なし、loopback偽Responses APIだけで実行した。CodexのApps／Plugins catalogはtest configで無効化し、loopback proxyが受けた非loopback要求は拒否した。実model、実credential、永続Codex設定、Windows保護設定は使わず／変更していない。
- 試験は実Rust `CodexCliAdapter`とprocess群監督、Broker libraryのWorkspace登録、Task consumer、`WorkspaceTaskScratch`を通る。試験専用`Broker統合CodexFixtureAdapter`が能力metadataだけを`supported`へ上書きして`FIXTURE`出所を付し、元Adapterの`task_execution=unsupported`を実行前に確認する。したがって実CLI／MxC processの観測は`LIVE_RUNTIME`だが、実行Authority、Owner確認、metadataはfixtureであり、製品Task対応やproduction IPCの成立を意味しない。
- Workspace Permissionまたは本文hash結合Owner Approvalがない要求、ならびに消費後再利用ではCodex CLIへの接続が起きないことを確認した。synthetic registered secret fileのreadとWorkspace外markerのread／writeは拒否され、許可Workspace内のmarker writeは成功した。Task結果にTask本文、secret本文、固定assistant本文が含まれないこと、Broker開始／完了Audit callback、正常終了後の`.d4p-tmp-*`scratch不在を確認した。callbackは試験内memoryでありdurable Auditの証拠ではない。scratch不在もMxC内部TEMPの物理cleanupを証明しない。
- API transport安定化のため、test providerに限りApps／Plugins取得を無効化し、stream retryを2回へ制限した。偽APIは固定toolをWorkspace markerの有無で冪等に返す。これはtest設定であり、production CLI retry policyの変更ではない。
- 初期試験serverはWindows listener由来のnonblocking socketを受け取り、`WSAEWOULDBLOCK`でHTTP request前に失敗した。socketをblockingへ戻した後もCodexのSSE body decodeで間欠失敗があり、Apps／Plugins外部取得を止めた上で限定retryを設定した。失敗runは成功回数へ含めず、観測履歴を保持する。最終構成のLIVE試験は3回連続で合格した。
- 未解決の範囲は、production `broker-server`／IPCとDesktop native Owner確認、durable Audit、OneDrive Cloud Files／通常NTFS双方、深度超過とhardlink aliasを含む実tool-child隔離、cancel／deadline／crash後のprocess群停止・Recovery、MxC内部TEMPの実体と物理cleanup、結果／diffの操作者表示、実provider相互運用である。`task_execution=unsupported`、該当`release_blocker`、`release_ready=false`を維持する。

### 検証

- PowerShell `$env:GUI_SHELL_CODEX_TASK_BROKER_TEST_EXE='<Owner指定codex.exe絶対path>'; cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --lib 'broker::dialogue::tests::Broker承認経路から実CodexCLIをloopback偽APIで実行し隔離とcleanupを確認する_LIVE_RUNTIME' -- --ignored --exact --nocapture`：Windows local `LIVE_RUNTIME`を3回個別起動し3/3 success。各回で固定tool、Responses往復、許可／拒否path、通常scratch cleanupを確認。test内のAuthority／Approvalは`FIXTURE`。
- `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`：初回OneDrive内`target`出力はMSVC `LNK1201`（PDB書込失敗）でbuild停止。C:空きは約106 GBだった。同じ検証を`CARGO_TARGET_DIR=C:\Users\ohira\AppData\Local\Temp\D4PocketRustTarget-<一時識別子>`へ出力して再実行し、12 targetで405 passed／0 failed／2 ignored。
- 同じ一時targetで`cargo +1.95.0 check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`: 成功。`rustfmt +1.95.0 --edition 2021 --config skip_children=true --check`を今回変更した3 Rust fileへ実行し成功。
- `python -X utf8 tooling/schema_check/check_schemas.py`: Schema 149、example 149、negative fixture 192で成功。`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`: 225 checksで成功。初回Conformanceは新規testの`std::fs::read/write`禁止patternを検出し、明示file handleへ直した後の再実行が合格した。
- `python -X utf8 tooling/日本語基底監査.py --strict`：初回はRust test内のCLI設定／HTTP・SSE機械書式を直前120文字の診断macroで誤検出し、9 findingsとなった。監査器のRust診断判定を文字列直前の呼出しへ限定し、CLI assignment・HTTP・SSE書式の自己回帰testを加えた。監査自己試験46件と最終strict監査（1114 file／0 findings）は合格。実際の人間向け診断と合成assistant本文も日本語化し、例外台帳は広げていない。
- `rustfmt +1.95.0 --edition 2021 --config skip_children=true --check`：今回変更したRust 3 fileで合格。`cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`：専用Temp targetで12 target、405 passed／0 failed／2 ignored。既存OneDrive `target`への初回buildはMSVC `LNK1201`のまま履歴保持。
- 最終sourceで明示LIVE試験を再実行したところ、最初の2回成功後に1回だけTask `failed`となった。観測はResponses API 3 POST、Workspace marker作成済み、`chatgpt.com` CONNECT拒否1件で、原因不明。試験専用の秘匿済み応答解析診断を追加し、その後は単発1回と連続5回が成功した。漏えい否定assertを最終fixture文面へ修正した後も追加5回連続で成功した。孤立失敗は消去せず未解明として記録し、成功反復へ加算しない。
- Schema 149／example 149／negative fixture 192、Conformance 225 checksは再合格。Manifest 1111件を再生成・照合した時点の`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`はexit 0、release gate validationはpassだった。その後、assert文面・検証履歴・registryを更新しながら開始したvalidator再試行ではManifest、release gate、packaging checkがfailedとなった。実行中に対象fileが変わったためManifestとの不一致による結果であり、そのFAIL履歴を保持する。差分・文書・Manifestを固定して再実行した最終validatorもexit 0、release gate validation passとなった。
- Final validatorではWindows installed evidenceが5項目欠け、各項目は既存`release_blocker`としてfailedのまま残る。validator／release gateの構造検査がpassしたことをrelease readinessへ読み替えず、`release_ready=false`とする。
- GitHub Actionsは未使用。現在のWindowsで実CLI／Rust全targetを検証できたため、OneDriveのPDB失敗はlocal Temp targetで補い、hosted runnerを不要なCIへ拡張しない。

## R2追補 通常Broker IPCからAgent Task権限を発行できないことのWindows実測（2026-09-30）

### 成立した確認

- `installer/windows/collect_broker_smoke.ps1`をversion 6へ更新し、実Broker processのnormal loopback credentialから`AgentTaskWorkspacePermissionGrant`と`AgentTaskOwnerApprovalGrant`を個別送信する。両要求が`desktop_native_owner_confirmation_required`で拒否されない場合はcollectorを失敗させる。応答本文や資格値はevidenceへ複写せず、拒否bool、固定error code、source provenanceだけを記録する。
- release evidence validatorは両操作の実測bool、固定error code、`LIVE_RUNTIME` provenanceを必須化した。Conformanceへcollector構造検査と、片方の拒否欠落／error code差替をrelease blockerとして検出する否定試験を追加した。
- 現行Rust source commit `e8ea0587031402a19fc9dff260d7c925403cca2e`からRelease helperをbuildし、isolated temporary store／sessionでcollectorを起動した。build時のworktreeは文書・tooling差分を含むdirty状態だったが、Rust sourceの差分はなかった。helper SHA-256は`e9f6e1c536f4e8a166fe4ff5d775bd649c89b0bba635e82ab42d83c252b80344`、collector output SHA-256は`e2b40602ef691373635d0cbf137d6aa4c0c4aac46944e796f7290333cdd191c9`。
- collectorは両grant要求を通常IPCから拒否し、restart後replay拒否、新規health受理、Broker強制終了後のfail-closed、session資格fileの生成後削除も成功した。collector resultは`passed`、errorは0件。

### 証拠境界

この`LIVE_RUNTIME`観測はstandalone Rust Broker processの通常IPC経路に限る。通常credentialでWorkspace Permission／Owner Approvalを直接発行できないことを示すが、Desktop起動器のnative Owner確認、承認後のTask起動、耐久製品Audit統合、installed package、別user profileを検証していない。`task_execution=unsupported`、`windows_broker_installed_smoke`とAgent Taskの`release_blocker`、`release_ready=false`を維持する。

### 検証

- Schema検査は成功し、149件のSchema、149件の正常例、192件の否定fixtureを確認した。実行コマンド: `python -X utf8 tooling/schema_check/check_schemas.py`。
- 適合性検査は成功し、227件のcheckを確認した。実行コマンド: `python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`。
- Windows PowerShellの構文解析器で収集器scriptを解析し、構文errorなしを確認した。対象: `installer/windows/collect_broker_smoke.ps1`。
- 変更差分に余分な空白や末尾空白がないことを確認した。実行コマンド: `git diff --check`。
- `cargo +1.95.0 build --locked --manifest-path native/rust_helper/Cargo.toml --release --bin gui_shell_rust_helper --target-dir <isolated temporary directory>`: exit 0。変更外のdead_code warning 2件。
- `powershell.exe -NoProfile -ExecutionPolicy Bypass -File installer\\windows\\collect_broker_smoke.ps1 -BrokerHelperExe <isolated Release helper> -OutputPath <isolated evidence path>`: exit 0。2つのAgent Task grant拒否と既存Broker smokeがすべて成功。
- 厳格日本語監査: exit 0。1114 file／0 findings。
- `python -X utf8 tooling/manifest.py --write`: exit 0。Manifestへ1111 fileを記録。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`: exit 0。登録済みdevelopment検査10件は合格。Windows installed evidence 5項目はrelease blockerとして残り、`release_ready=false`。
- GitHub Actionsは未使用。現WindowsでRelease helperのbuild・実Broker processを検証できたため不要。
- このblockはcollector／validatorの拡張でありRust source変更を含まないため、Rust test suiteは再実行していない。Flutter、installed product、Desktop Owner操作も未検証。

## R2追補 Agent Task native Owner確認のFlutter応答待ち（2026-09-30）

### 成立した変更

- Desktopの`BrokerClient`で`AgentTaskWorkspacePermissionGrant`と`AgentTaskOwnerApprovalGrant`が、Rust Desktopのnative Owner確認を待つ既存operation分類から漏れ、5秒の通常応答timeoutを使っていた。Flutter callerが先にtimeoutしてもnative確認要求を取り消したことにはならず、呼出側の結果が曖昧になる。
- 既存のOwner確認operation分類を共通timeout関数へ集約し、上記2 operationも305秒の応答待ちへ含めた。`AgentTask実行`など通常Broker operationは引き続き5秒。これはFlutterの待ち時間だけで、Owner同意・権限・Broker処理の取消を生成しない。timeoutは承認・拒否に読み替えず、再操作前にBroker状態を再照合する意味契約を`docs/specs/agent-runtime.md`へ追記した。
- Rust authority経路、Broker TTL、Adapter metadata、`task_execution=unsupported`は変更していない。Agent Taskは依然未実行であり、release blockerと`release_ready=false`を維持する。

### 検証

- ConformanceへFlutter operation timeout分類の構造検査を先行追加した。修正前の実行はAgent Task grant 2件の分類欠落とtimeout policy未接続を検出して失敗し、修正後は228 checks合格。
- `flutter test --no-pub --no-test-assets --reporter expanded test/broker_client_payload_hash_test.dart`: 6件合格。新しいpolicy testでWorkspace Permission／Owner Approvalが305秒、通常の`AgentTask実行`が5秒であることを確認した。
- `flutter test --no-pub --no-test-assets --reporter expanded`: Desktop全126 tests合格。
- `dart format lib/services/broker_client.dart test/broker_client_payload_hash_test.dart`: 成功、書式変更なし。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`: exit 0。strict日本語監査1114 file／0 findings、Schema 149／example 149／negative fixture 192、Conformance 228、登録development check 10件が合格。Windows installed evidence 5項目は従来の`release_blocker`のまま、`release_ready=false`。
- `flutter analyze --no-pub`: Analysis Serverが日本語を含むOneDrive project path上の不正LSP JSONで異常終了。exit 1、Analyzer成功とは扱わない。初回focused testも既存`build/unit_test_assets`削除拒否で開始できなかったが、`--no-test-assets`指定のfocused／全testは合格。
- `dart analyze lib/services/broker_client.dart test/broker_client_payload_hash_test.dart`: Analysis Server終了処理が`C:\Users\ohira\AppData\Local\Dart\perf\18552`を削除できずexit 1。静的Analyzer成功とは扱わない。GitHub CLIがこの環境にないため手動Actionsは起動していない。
- `python -X utf8 tooling/manifest.py --write`: 1111 fileを記録。`python -X utf8 tooling/manifest.py --check`と`git diff --check`はともに合格。
- Windows Actionsは未使用。ローカルDesktop全126 testsとPython統合validatorが合格し、残ったFlutter AnalyzerはOneDrive上のAnalysis Server障害である。`gh` CLIは未導入で、現行接続済みGitHub toolにも手動dispatch機能がないため、Actions runでの代替検証は実施していない。

## R2追補 Codex Adapter metadataのAuthority誤検知を修正しBroker経路を再確認（2026-09-30）

### 成立した変更

- Codex Adapterは`task_execution=unsupported`とする理由に「専用permission profileの実Taskは未検証」と通常の説明文を含む。BrokerのAdapter metadata走査器は自由文を含む任意文字列から`permission`等をAuthority keyとして扱っていたため、権限昇格のない正規metadataを`応答不正`で拒否し、Task要求以前のAgent Session開始を阻害していた。
- 自由文走査は`admin`、`all`、`approved`、`elevated`、`root`等の危険Authority値を拒否し、構造化object keyは従来どおりcanonical Authority／Permission keyを拒否するよう責務を分離した。説明文中の`permission profile`は受理しつつ、`{"permission":"workspace.write"}`、`permission=all`、`admin`は拒否する否定・境界testを追加した。Codex Adapter自身のmetadataを検査する回帰testも追加し、Task capabilityが`unsupported`のまま保持されることを確認する。
- Windows専用ignored統合testは、明示指定したインストール済みCodex CLIの登録probeと実Broker loopback IPCを通し、登録Workspaceに結合したAgent Session開始が成功することを確認する。Owner操作callbackを肯定にしてもWorkspace Permission／Owner Approvalの両要求が`AgentTask実行非対応`で拒否され、grantを返さず、永続Auditへ拒否を記録し、Task本文をsummary／response／Auditへ露出しない。CLIは登録probeに限り、実Agent Task・model・network接続・実Win32 Owner dialogは実行していない。Owner確認callbackはtest fixtureであり、native確認画面の製品動作証拠ではない。
- したがって本修正は通常metadataの誤拒否を解消するが、実Task実行を有効化しない。`task_execution=unsupported`、Agent隔離およびWindows installed証拠等の`release_blocker`、`release_ready=false`を維持する。

### 検証

- Codex Adapter metadataの回帰testと自由文／構造field境界testは、それぞれ1件合格した。実行command:

```powershell
cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml 'CodexAdapterのmetadataはAuthority入力検査に誤拒否されない' -- --test-threads=1
cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml authority_scan_distinguishes_structural_claims_from_explanatory_prose -- --test-threads=1
```
- 明示したWindows環境変数`GUI_SHELL_CODEX_TASK_BROKER_TEST_EXE`で起動したignored統合test `登録CodexへのnativeOwner確認後もAgentTask非対応gateを維持する`: 1 passed。実Codex CLIは登録probeのみ。Owner肯定操作はsynthetic callbackであり、Task・model・実native dialogは未実行。
- Rust全targetの初回再実行は12 test target中、主lib 360 passed／1 failed／3 ignored、他targetはすべて成功した。未変更のA2A loopback fixtureで`A2A Agent Card応答を読めない`が発生した。該当testの単独再実行は1 passed、続く同一Rust全target再実行は主lib 361 passed／0 failed／3 ignored、他targetもすべて成功し、全12 target合計407 passed／0 failed／3 ignoredでexit 0となった。初回failureは履歴として保持し、原因は確定していない。
- `git diff --check`: 合格。`desktop_launcher.rs`全体へのrustfmt `--check`は既存の無関係な整形差分も報告したため、ファイル全体の機械整形は行っていない。追加コードを手動確認し、差分全体へ無関係なformat変更を混入させていない。
- 初回失敗した統合testが作ったTemp directory 2件の削除は実行環境のpolicyに拒否された。対象はWindowsのTemp配下に限定された試験用directoryであり、repositoryには含まれない。削除を回避する別経路は試していない。

### 統合検査の追補

- 統合validator初回は、新規testの英語diagnostic 1箇所と進捗記録のcommand表記2行を日本語基底監査が検出し、2 file／3 findingsで失敗した。test診断を日本語化し、正確なcommandを独立した実行記録へ分離した後の厳格監査は1114 file／0 findingsで合格した。監査規則や許可例外は変更していない。
- 修正後の統合validatorは登録済みdevelopment検査10件すべて合格した。Schema 149件、正常例149件、negative fixture 192件、Conformance 228 checks、Manifest照合、release gate、packaging portability、release smoke、evidence bundle、runtime assertion、C32構造監査が合格した。Windows installed product evidence 5項目は既存`release_blocker`のままで、`release_ready=false`を維持する。
- 統合validatorの実行command:

```powershell
python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows
```

## Phase 21／C12追補 Broker更新package download worker（2026-09-30）

### 成立した変更

- Rust Desktop起動器のnative Owner確認を経た内部経路だけで、現在の候補hash、Broker trustで再検証した署名、配布URL、version/channel/summary、package SHA-256・正確なbyte長、request payload hashをBrokerが副作用直前に再照合し、download要求を受理する。通常IPCやOwner credential単独では受理しない。Flutterは既存Broker bridge経由で要求・状態取得だけを行い、network／filesystemを直接扱わない。
- Rust Brokerはserial IPC loop外の単一非同期workerでHTTPS取得する。proxy／redirect／retryを無効化し、DNS結果をpublic unicastに限定して取得addressへpinする。HTTP status、単一Content-Length、Transfer-Encoding／Content-Encodingの不在、実byte長、SHA-256を検査し、完全一致したfileだけをBroker固定directoryへcontent-addressed・create-onlyで公開する。処理は64 KiB単位、Job状態とfailure codeはbounded／Schema固定で、常駐pollingを行わない。partial cleanupと結果はBroker Auditへ接続する。
- 適用、process起動、rollback、破損package修復は未接続。OS同期DNS resolverには期限／cancelがなく、同一digestの破損packageは修復不能である。これらはrelease_blockerとして保持する。system proxy必須環境への非対応はknown_limitation。local TLS serverとBroker test storeはFIXTUREであり、実配布元／Windows installed productのLIVE_RUNTIME証拠ではない。

### 検証

- Rust全12 target（lib、launcher、binaryおよび統合test）: 428 passed／0 failed／3 ignored。全target compile checkも成功。
```powershell
cargo +1.95.0 check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets
cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1
```

## Phase 21／C12追補 同一digest破損packageのAudit付きrepair（2026-09-30）

### 成立した変更

- 既存`<digest>.pkg`をBroker固定directoryからnofollowで開き、通常file・非reparseを確認した上で全byteを再hashする。期待byte長またはdigestと異なる通常fileだけを破損候補とし、symlink／reparse／directory／不正entryは従来どおりfail-closedにした。
- native Owner確認済みdownloadが新しいresponseのstatus／header／byte長／SHA-256検査を通り、一時fileをfsyncした後にだけ、同一digest名の破損通常fileをatomic renameで置換する。保存先が欠損している場合はcreate-only hard linkを維持し、別digest／別packageは置換しない。
- queued Auditへ条件付きrepairの対象・RecoveryActionを先に記録し、置換成功を`recovered`結果としてBroker Auditへ記録する。download後もinstall／process／rollbackはsuspendedのまま。再起動後は`.part`回復とfinal全byte再検証を明示download要求で行う。
- 過去Phase 21／C12記録の「repair未成立」は当時の観測履歴として残し、本追補で現在状態を更新する。製品Broker／installed productでの修復証拠は未成立である。

### 検証

- 更新focused Rust試験は27 passed／0 failed。既存破損fileの分類・修復、non-file宛先拒否、通常新規時のcreate-only維持、Broker `recovered` Audit、local TLSで検証済みresponseだけを置換する経路を実行した。誤digest responseでは既存fileを変更せず、一時fileを除去する試験も合格した。
- focused試験の最初の全群実行でlocal TLS fixtureが一度ConnectionResetになり、26 passed／1 failedだった。失敗case単独再実行と、その後のfocused全群再実行はいずれも成功した。初回失敗の原因は特定できておらず履歴として保持する。
- Rust全targetは12 target、434 passed／0 failed／3 ignored。`cargo check --all-targets`も成功。Windows localで実行し、GitHub Actionsは使っていない。
- Schemaは150件／正常例150件／negative fixture 193件、Conformanceは229 checks。Conformance初回は移動後のcreate-only hard-link実装に旧tokenを要求して失敗したため、現行実装のtokenを検査するよう更新し、全229 checksを再実行して合格した。
- `python -X utf8 tooling/manifest.py --write`で1119 filesを記録し、統合validator内のmanifest checkも合格した。strict日本語監査はrepository 1122 files、debt 0、findings 0。`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`のdevelopment checksはすべてpassedで、release gate／portability／smoke／evidence bundle／runtime assertion／C32監査を確認した。`release_ready=false`とrelease blocker 5件は維持する。

```powershell
cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --lib broker::update_ -- --test-threads=1
cargo +1.95.0 check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets
cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1
python -X utf8 tooling/schema_check/check_schemas.py
python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py
python -X utf8 tooling/manifest.py --write
python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows
```

### 残存範囲

- item: Windows installed product／実配布元でのdownload・repair・failure injection、Installer／Uninstaller、install／rollback／crash recovery、正式trust provisioning
  classification: release_blocker
  reason: local Rust／TLS fixtureは実配布元・製品Broker・installed productを通らず、release経路を実証しない
  required_action: test identityの隔離Windows installed productへ同じ境界を接続し、実配布元tamperと失敗／crashを検証する
  blocks_release: yes
- item: Linux／macOSの期限付きcancel可能DNS resolver
  classification: post_v1_scope
  reason: 現行Windows 1.0以外のR15技術工程である
  required_action: R15で対象OSごとの期限・cancel・negative／runtime試験を定義する
  blocks_release: no
- item: system proxy必須環境のdownload
  classification: known_limitation
  reason: 現workerはsystem proxyを使わず、直接HTTPSが許可されない環境では取得できない
  required_action: 現行製品文書の制約記載を維持する
  blocks_release: no

## Phase 21 C12追補 Windows DNS期限・取消境界（2026-09-30）

### 成立した変更

- C12 package downloadが呼ぶ名前解決について、Windowsだけを対象にWindows DNS Client `DnsQueryEx`のcallback経路へ接続した。A／AAAAは逐次照会し、それぞれdownload全体deadlineと最大15秒のDNS deadlineの早い方を使用する。download cancel flagは20 ms間隔で確認する。
- deadline／cancel時は`DnsCancelQuery`を呼び、callback完了まで結果・cancel handle・query contextを保持する。callback回収は最大2秒とし、cancel失敗・callback未回収をstatic failureへ閉じる。未完了queryが残る間はprocess内pending slotを維持し、次のDNS queryを拒否する。既存のpublic-address判定と、検査済みaddressへの接続pinningは保持した。
- Rust helperの`#![forbid(unsafe_code)]`は変更していない。Windows DNS APIの`unsafe`とFFI共有状態は専用crate `native/windows_dns`へ隔離した。Windows外では既存`ToSocketAddrs`を維持し、期限付きcancel対応はWindows 1.0対象に限定する。
- Windows localの試験processから実Windows DNS Client APIをloopback DNS fixtureへ呼び出した。A／AAAA応答のparse、非対称IPv4 octet `1.2.3.4`、明示cancel、deadline超過後cancel、事前cancelを観測した。これはhelper APIの`LIVE_RUNTIME`証拠だが、製品Broker／native Owner確認／installed productや実配布元の証拠ではない。
- `release_blockers.registry.json`、C11のmachine-readable最終監査、`ROADMAP.md`、`docs/specs/update-center.md`へ成立範囲と残存blockerを反映した。破損package修復、installed download、install／rollback、実配布元は`release_blocker`を維持し、Linux／macOSの期限付きresolver差はR15の`post_v1_scope`として区別する。

### 検証

- Windows DNS helper独立試験: 4 passed／0 failed／0 ignored。loopback DNS serverは`127.0.0.1:53`を使用し、WindowsのDNS設定は変更していない。
- Rust helper全target: 12 test target、429 passed／0 failed／3 ignored。DNS API専用testは独立crate testで別途4件実行した。
- `cargo check --all-targets`、変更Rust fileの`rustfmt --check`、`git diff --check`、Blocker／最終監査JSON parse: 成功。
- Schema: 150件、normal example 150件、negative fixture 193件で成功。Conformance: 229 checksで成功。
- Conformanceの初回は旧resolver call表記の要求、二回目は書式改行と既存`#[cfg(test)]`分割による新resolver関数の見落しで失敗した。検査を削除・弱体化せず、空白正規化とresolver関数の独立抽出を加えてdeadline／Windows分岐／native cancel機構を必須検査にし、229 checksを再実行して合格した。
- 厳格日本語監査は最初に新crate内英語panic message 2件を検出した。該当診断を日本語化した後、repository_files 1122／debt files 0／findings 0で合格した。
- `python -X utf8 tooling/manifest.py --write`は1119件を書き込み、`python -X utf8 tooling/manifest.py --check`が合格した。新しいWindows DNS crateのmanifest coverageを`tooling/manifest.py`へ追加した。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`はexit 0。日本語基底、Schema、Conformance、Manifest、release gate、package portability、release smoke、evidence bundle、runtime assertion、C32最終開発監査の10検査がすべてpassed。Windows installed release evidenceの5項目は未成立としてfailedのままで、`release_ready=false`を維持する。
- GitHub Actionsは未使用。local WindowsでWindows DNS API試験を含む対象検証を実行できた。

### 残存範囲

- `release_blocker`: 破損package repair、Windows installed product上の実download、実配布元／失敗注入、Installer／Uninstaller、install／rollback／crash Recovery、正式trust provisioning。
- `post_v1_scope`: Linux／macOSの期限付きcancel可能なDNS resolver。非Windows技術工程R15で実装・検証する。
- `known_limitation`: system proxyを必要とする環境ではdownloadを利用できない。
- release blockerは未解消のものが残るため`release_ready=false`を維持する。GitHub Actionsは未使用。Windows localで必要なAPI／Rust試験が実行できた。
- Rust update-focused試験は21 passed／0 failed。local TLS fixture、実byte長／digest照合、HTTP header拒否、URL／IP境界、partial recovery、Broker Owner確認、stale request、競合要求、Auditを含む。
```powershell
cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --lib broker::update_ -- --test-threads=1
```
- JSON Schemaは150件、正常example150件、negative fixture193件。Conformanceは229 checks。
```powershell
python -X utf8 tooling/schema_check/check_schemas.py
python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py
```
- Desktop Flutter対象testは10件合格し、`flutter analyze --no-pub`は一時`Z:` drive aliasからの再実行で`No issues found!`。長いOneDrive原pathでの先行Analyzer parse失敗は環境制約として履歴に残す。一時aliasは解除し、保護設定は変更していない。
```powershell
flutter test --no-pub test/update_client_test.dart test/broker_client_payload_hash_test.dart
flutter analyze --no-pub
dart format --output=none --set-exit-if-changed lib/screens/settings.dart test/update_client_test.dart test/broker_client_payload_hash_test.dart
```
- Rust変更7 fileのrustfmt check、`git diff --check`、変更JSON 8 fileのparse、strict日本語基底監査1118 file／0 findings、packaging portability checkが成功した。portability試験はGit追跡sourceだけをZIP化するため、新規4 fileをindexへstageした状態で実行した。
- 統合validatorの初回実行は新Rust comment 7件の日本語基底違反と、index未登録だった新Schema例のZIP欠落を検出した。commentを日本語化し、必要な4 fileをstageしてManifestを1115 fileで再生成後、strict監査とpackaging portability checkを個別再実行して成功した。最初の失敗は検査規則を弱めず修正した。
- 最終`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`はexit 0。Manifest 1115 source files、strict日本語監査1118 file／0 findings、Schema 150／正常example150／negative fixture193、Conformance 229 checksを確認し、登録済みdevelopment check 10件が全件成功した。release gate整合、package portability、release smoke、evidence bundle、runtime assertions、C32構造監査を含む。
- このvalidatorのrelease gate整合passはrelease readinessを意味しない。release evidenceはCONFIG／FIXTURE範囲に留まり、`release_ready=false`と既存release blockerを維持する。
- Windows GitHub Actionsは未使用。対象Windows上で全Rust／Flutter検査とpackaging検査を実行でき、別runnerを追加する必要はなかった。

### 残存項目

- item: OS同期DNS解決の期限／cancel不能
  classification: release_blocker
  reason: HTTP client timeout開始前の同期resolverがworker停止を阻害し得る
  required_action: 有限期限・cancel可能なDNS解決を実装し、timeout／cancel failure injectionで検証する
  blocks_release: yes
- item: 破損済み同一digest packageの修復
  classification: release_blocker
  reason: create-only保存と既存file検査により既存packageを置換しないが、安全なrepair／Recovery経路がない
  required_action: Broker Audit／Recovery付きrepairを追加し、tamper・crash・再試行を試験する
  blocks_release: yes
- item: system proxy必須環境
  classification: known_limitation
  reason: download workerはsystem proxyを使用しない
  required_action: 直接HTTPSが許可されない環境では使用できない旨を製品文書へ維持する
  blocks_release: no
- item: 外部配布元・Windows installed productでのdownloadおよびinstall／rollback
  classification: release_blocker
  reason: local TLS fixtureは実配布元、配布物、installed update transactionを証明しない
  required_action: test identityと隔離Windows installed packageを用いて実配布元からの取得・tamper・install・rollback・crash Recoveryを実証する
  blocks_release: yes

## Phase 21／C11 Broker導出取得元projectionとDesktop表示（2026-09-30）

### 成立した変更

- 更新一覧の各候補に`取得元`を追加した。Brokerは現在の信頼設定で候補署名と候補hashを再検証し、版2候補の署名済み`channel`とBroker所有配布元だけから`base_url/update_id.pkg`を導出する。
- 配布元がない候補は`unconfigured`、旧候補・stale候補・trust不整合は`ineligible`としてURLを返さない。取得先URLのSchema上限とdot segment拒否を加え、候補由来の`package_url` fieldとdownload要求への呼出元URL持込をnegative testで拒否する。
- Desktop更新一覧にBrokerが返した配布元host、署名済みpackage byte長、hash prefixを表示する。これは`INTERNAL_STATE`の表示projectionであり、ネットワーク接続・Permission・Approvalを生成せず、download／install／rollbackは`suspended`のまま。
- 正本`docs/specs/update-center.md`、`規定/正本索引.json`、ROADMAP、release blocker履歴を更新した。既存release blockerと`release_ready=false`は維持する。

### 検証

- Rust全12 targetで419 passed／0 failed／3 ignored。新しいBroker取得元projectionと呼出元URL拒否focused testは1 passed。`cargo check --all-targets`成功。
- Schema 149件／正常example 149件／negative fixture 192件、Conformance 228 checks成功。取得元欠落、状態とURLの矛盾、dot segment、候補由来URLを拒否した。
- `flutter test --no-pub test/update_client_test.dart`は元OneDrive workspaceでは`build/unit_test_assets`削除失敗でtest前に停止した。同じcurrent sourceを短縮一時workspaceへ複製（`build`と`macos`生成物を除外）して3件passed。`flutter analyze --no-pub`も同workspaceで`No issues found`。この結果はFlutter sourceの検査でありWindows installed productの起動証拠ではない。
- Flutter検証用の一時workspace cleanupは、検証済みTemp直下pathへのPowerShell再帰削除commandが実行policyで拒否されたため完了していない。Repository外のTempにsource copyと`.dart_tool`生成物が残る。production credentialは複製していない。正確なpathとcleanup未完了は作業報告に明記する。
- Dart format checkで変更なし。Rustfmt check、`git diff --check`、変更JSON parseは成功。
- 最終URL Schema締め付け前の統合validatorはexit 0、登録10検査すべてpassed、strict日本語監査1114 file／0 findings。release gate検査はpassしたがrelease blocker 5件と`release_ready=false`を維持した。後続のSchema pattern／negative test変更後にもSchema 149／149／192とConformance 228 checksを再実行し合格した。
- Windows Actionsは未使用。必要なRust／Flutter／Schema検証がWindows localまたは短縮一時workspaceで成立したため、別runnerを要しなかった。

### 残存blocker

- item: 実HTTP download、redirect／DNS／TLS制御、実byte長・SHA-256照合、Broker保管、download失敗・crash Recovery、install／rollback
  classification: release_blocker
  reason: 本単位は配布元の表示projectionだけで、外部通信・実file・導入経路を実装していない。
  required_action: native Owner確認と一回限りPermission／Approval／Audit／Recoveryを通したbounded download consumerを実装し、続けてpackage検証、same-volume導入・rollback、Windows実測を行う。
  blocks_release: yes

## Phase 21／C11追補 永続候補の現在trust再検証（2026-09-30）

### 成立した変更

- 永続recordの`署名状態=verified`は過去に検査した結果であり、Broker再起動後・trust鍵変更後も現在有効とは限らない。更新一覧と延期receiptでは現在のBroker trustでEd25519署名を再検査し、候補全体から再計算したcanonical hashが保存hashと一致する場合だけ`verified`を表示する。それ以外は`verification_stale`へ降格する。
- download／適用／rollback要求の直前にも現在trust、署名、候補hashを再検査する。trust未設定、鍵ローテーション、署名後content改変、保存candidate hash差替えを拒否し、成功時も既存どおりAudit付き`suspended`で外部実行しない。
- この単位はBrokerの永続記録状態を用いたRust試験（証拠種別`FIXTURE`）である。外部攻撃者が製品保存領域を書き換えられる実環境、Windows導入済み製品、配布packageの取得・導入を証明しない。`release_ready=false`とWindows配布`release_blocker`を維持する。

### 検証

- Rust Update Centerの対象試験9件が成功。trust変更・trust未設定後の降格／拒否、候補内容変更、保存候補hash差替えを含む。後続のRust全target試験では全413件成功、失敗0件、無視3件。
```powershell
cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --lib broker::update_center::tests -- --test-threads=1
```
- Rust対象fileの整形検査と差分空白検査。
```powershell
rustfmt +1.95.0 --edition 2021 --config skip_children=true --check native/rust_helper/src/broker/update_center.rs
git diff --check
```

- Rust全target試験と全target検査も成功。
```powershell
cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1
cargo +1.95.0 check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets
```
- Schema 149件／正常例149件／negative fixture 192件、Conformance 228 checksが成功。Desktopの対象Flutter試験2件と、ASCII短縮・正しいworkspace配置の一時copy上での`flutter analyze`も成功。Dart formatterは対象3 fileに差分なし。
```powershell
python -X utf8 tooling/schema_check/check_schemas.py
python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py
flutter test --no-pub --no-test-assets test/update_client_test.dart
dart format --output=none --set-exit-if-changed lib/screens/settings.dart lib/services/update_client.dart test/update_client_test.dart
```
- 元OneDrive作業場所での`flutter analyze`は既存macOS生成物`macos/Flutter/ephemeral/Packages/.packages`をFlutterが削除できず失敗した。ACLや生成物を変更せず、macOS／build生成物を除いた短縮一時workspaceへ必要なFlutter appと`gui_shell_ui` packageを複製して同コマンドを実行し、`No issues found`を確認した。この解析はsourceの静的検査であり、Windows製品起動証拠ではない。
- 統合validatorの初回実行は、この節の日本語基底監査1件と、編集中9 fileのmanifest hash不一致で失敗した。説明の英語混在表現を日本語化し、厳格監査は1114 file／0 findingsで合格。MANIFESTを1111 fileで再生成した。
- 修正後の統合validatorは登録済み10検査すべて成功した。release gate検査、packaging portability、release smoke、evidence bundle、runtime assertion、C32構造監査も成功したが、正式Windows実機evidenceの5 release blockerと`release_ready=false`は維持された。初回失敗は履歴として保持し、成功へ書き換えていない。
```powershell
python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows
```

- Windows Actionsは未使用。download要求は依然`suspended`であり、現在trust再検証のtest結果をLIVE_RUNTIMEやpackage真正性へ昇格しない。

## Phase 21／C11追補 SchemaとBrokerの配布元channel一意性整合（2026-09-30）

### 成立した変更

- Broker起動時検証は同じchannelの複数sourceを拒否していたが、JSON SchemaはURLが異なる同一channel要素を受理していた。
- Schemaの`package_sources`へ`stable`／`beta`／`nightly`ごとの`maxContains: 1`を加え、異なるURLの重複channelをnegative conformanceで拒否する。既存Broker検証も同じ重複sourceを拒否するfocused Rust testで確認した。
- これは版2の設定contract適合補修であり、network、download、package byte照合、install／rollbackのproduction pathは追加しない。release blockerと`release_ready=false`は維持する。

### 検証

- Schema 149件、正常example 149件、negative fixture 192件で合格。
```powershell
python -X utf8 tooling/schema_check/check_schemas.py
```
- Conformance 228 checksで合格。異なるURLを持つ同一channelのsourceをJSON Schemaが拒否することを追加検査した。
```powershell
python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py
```
- Broker側の重複channel拒否focused testは1 passed／0 failed。
```powershell
cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --lib package_source_requires_canonical_https_base_and_unique_known_channels -- --test-threads=1
```
- 変更後のstrict日本語基底監査は1114 file／0 findingsで合格。統合validatorはexit 0で、登録development check 10件すべて合格した。Schema、Conformance、Manifest、release gate整合、packaging portability、release smoke、evidence bundle、runtime assertions、C32構造監査を含む。
- Release gate検査の合格はrelease readinessを意味しない。実download／package byte照合／install／rollbackのblockerを保持し、`release_ready=false`のままとした。
- Windows Actionsは未使用。対象はSchema／Conformance契約と既存Rust拒否testで、Windowsローカル検証が成立した。
```powershell
python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows
```
- `git diff --check`も合格。

## Phase 21／C11追補 Broker所有配布元設定contract（2026-09-30）

### 成立した変更

- `update_trust.json`版2へ、Broker所有のchannel別`package_sources`を追加した。旧版1は配布元なしの署名trustとして引き続き読込可能。初期設定は公開鍵未設定・配布元空の版2で、鍵未設定のまま配布元だけを有効化できない。
- channelは既存署名済み候補と同じ`stable`／`beta`／`nightly`に限定し、各channel一件・総数3件までとした。配布元はHTTPS、小文字ASCII DNS host、port／userinfo／query／fragmentなし、ASCII unreserved pathに限定し、IP literal、localhost、percent-encoding、dot segment、空segment、末尾slashを拒否する。
- Brokerはtrust fileを8 KiBまでに制限し、通常file／非reparse point、重複JSON fieldなし、版に対応した厳密field構成を確認してから再検証する。Flutter・更新候補からURLやfile pathを受け取らない。
- 次段download consumerが使う予定の決定式は、署名済み`channel`でBroker所有sourceを選び、`base_url`へ安全な`update_id`と固定`.pkg`を追加するものと定義した。現実装はsource設定の読込・検証までで、URL取得、redirect／DNS／TLS検証、package byte照合、archive展開、install／rollbackは未接続であり、実行要求は`suspended`のまま。
- 証拠classは`CONFIG`と`FIXTURE`に限る。これはWindows installed product、実配布元、実HTTP、製品release readinessの`LIVE_RUNTIME`証拠ではない。`rev2_desktop_product_distribution`と`release_ready=false`を維持する。

### 検証

- Rust全12 targetで418 passed／0 failed／3 ignored。Broker trust版1互換、版2起動読込、URL境界negative、重複JSON、上限超過、未構成source拒否を含む。
```powershell
cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1
```
- Rust全targetの静的compile検査に成功。
```powershell
cargo +1.95.0 check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets
```
- 変更Rust 3 fileのrustfmt checkと差分空白検査に成功。
```powershell
rustfmt +1.95.0 --edition 2021 --config skip_children=true --check native/rust_helper/src/broker/store.rs native/rust_helper/src/broker/update_center.rs native/rust_helper/src/broker/adapter_center.rs
git diff --check
```
- Schemaは149件、正常example149件、negative fixture192件。Conformanceは228 checksで合格。
```powershell
python -X utf8 tooling/schema_check/check_schemas.py
python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py
```
- 初回検査では、新Schema JSONの構文誤りとConformanceが禁止するfixture file書込patternを順に検出した。JSON構文を直し、fixture準備を固定test store上のfile操作へ変更してから上記最終検査を再実行し、全件合格を確認した。先行FAILは履歴として保持し、検査規則は弱めていない。
- 初回統合validatorはRustの試験用JSONを1行の長い文字列で埋め込んだ箇所をstrict日本語監査が1件検出し、残る9検査は合格した。fixtureを構造化JSON生成へ変更し、再度のstrict監査は1114 file／0 findingsで合格した。例外台帳と監査規則は変更していない。
- 最終`tooling/validate_all.py --python-only --desktop-platform windows`はexit 0。strict日本語監査1114 file／0 findings、Schema 149／149／192、Conformance 228 checks、Manifest、release gate整合、packaging portability、release smoke、evidence bundle、runtime assertions、C32構造監査を含む登録development check 10件がすべて合格した。
- 統合validator内のrelease gate整合検査とdevelopment smokeの合格はrelease readinessを意味しない。`release_ready=false`、Windows installed product／実package取得・照合・install・rollbackの既存`release_blocker`を維持する。
- Windows Actionsは未使用。Windowsローカルの全target試験とcheckが完了したため、今回は追加runnerを要しないと判断した。
```powershell
python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows
```

## Phase 21／C11追補 配布packageの署名結合（2026-09-30）

### 成立した変更

- 現行UpdateCandidate版1のEd25519署名は版・説明等のmetadataだけを結合し、将来取得する配布packageのbyte identityを含んでいなかった。UpdateCandidate版2では`package_sha256`（小文字hex SHA-256）と`package_size_bytes`（1〜4 GiB）を必須化し、Brokerが再構成するcanonical signed byteへ双方を含める。署名対象byteとfieldの不一致は候補保存前に拒否する。candidateから公開鍵、package path、実行commandは引き続き受け取らない。
- 版1の永続recordは削除せずBroker再起動後もdecode可能に保つ。一覧と延期receiptでは署名表示を`legacy_unbound`へ降格する。版1を新規候補として再受理せず、適用要求もpackage未結合として拒否する。
- これはC11のdownload前の信頼metadata改善であり、実際に取得したbyteの長さ／hash照合、archive展開、Installer／Uninstaller、update置換、rollback／crash Recoveryは未実装。更新download／適用／rollbackは従来どおり`suspended`、Windows配布blockerと`release_ready=false`を維持する。証拠classは`FIXTURE`で、Windows installed productの`LIVE_RUNTIME`証拠ではない。

### 検証

- Schema 149件、正常example 149件、negative fixture 192件。Candidate版2の不足hash、不正hash、空／上限超過size、未知pathをnegative検査し、旧receiptは`legacy_unbound`時だけ適合する。
```powershell
python -X utf8 tooling/schema_check/check_schemas.py
```
- 全Conformance 228 checksで合格。
```powershell
python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py
```
- focused Rust更新センターtestは7件合格。署名後hash差替え、署名後byte長差替え、旧candidate拒否、旧永続record保持・降格表示・適用拒否を含む。
```powershell
cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --lib broker::update_center::tests -- --test-threads=1
```
- Rust全targetの静的検査は合格。
```powershell
cargo +1.95.0 check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets
```
- Rust全target試験はWindows localで12 target、411 passed／0 failed／3 ignored。Cargo出力先は短い一時検証領域を再利用し、OneDrive配下targetのPDB問題を避けた。
```powershell
cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1
```
- 実施履歴: 全target試験の先行実行では既存A2A loopback test 1件が応答読取失敗となった。対象test単独の11回実行はすべて合格し、同一変更状態で全targetを直列再実行した結果も12 target、411 passed／0 failed／3 ignoredとなった。先行失敗は履歴として保持する。この再実行は間欠失敗が再発しないことや、実運用A2A接続の健全性を証明しない。
```powershell
cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --lib 'broker::a2a_center::tests::owner接続をBrokerで受理し通常IPC一覧へbounded射影する' -- --test-threads=1
cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1
```
- 対象Rust fileの整形検査と`git diff --check`は合格。
```powershell
rustfmt +1.95.0 --edition 2021 --config skip_children=true --check native/rust_helper/src/broker/update_center.rs
git diff --check
```
- GitHub Actionsは未使用。必要なRust／Python検査をWindows localで実行できた。署名候補の受理はpackage実byteの真正性を証明せず、正式release gateを閉じない。

### Analyzer短縮path再試験（2026-09-30）

- 前項のAnalyzer失敗をコード失敗と断定せず、Repositoryを移動せずに一時`Z:` drive aliasから`apps/desktop_flutter`を開いて`flutter analyze --no-pub`を再実行した。Analysis Serverは62秒で`No issues found!`を返し、Flutter Analyzerは成功した。一時drive aliasは実行後に解除し、Repository path・Windows保護設定は変更していない。
- `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml desktop_owner_allowlist_requires_native_confirmation_and_broker_audits_both_outcomes -- --test-threads=1`: 1件pass。Owner確認allowlistの既存Broker testを再実行しただけであり、登録済みCodex RuntimeへのOwner確認後、Agent Taskが非対応として拒否される統合経路は未試験。
- この追試は前記LSP parse障害を短縮pathで回避できることを示す環境限定の静的解析結果で、OneDrive原pathでの再発防止やinstalled productを証明しない。以前の失敗記録と`dart analyze`の終了処理失敗は履歴として保持する。GitHub Actionsは使用していない。

## R2追補 scratch作成後のsecret aliasをCLI起動直前に再検査（2026-09-30）

### 成立した変更

- 登録secretの初回検査はAdapter入口で実施済みだったが、その後Task scratchを作ってCLIをspawnするまでの間に、別のWorkspace writerがhardlink aliasを作成する競合窓があった。MxCの完全一致denyはalias名を検出しないため、起動直前にも同じ登録Workspace identityとsecret tree検査を行う。
- WorkspaceWrite用command構成後、process_tree::spawnの直前に検査し、読み取り専用Dialogueの経路は変更しない。Windows fixtureはscratchを先に作り、その後secret hardlink aliasを追加して、実spawn_codex_task入口がCLI process生成前に「作業領域不在」で止まることを確認する。
- この再検査はWorkspaceのcontentsを外部writerから原子的に固定せず、最終scanとprocess生成間の変更を排除しない。証拠classはFIXTUREであり、実MxC tool-child、production Broker IPC／Owner Approval、installed productの隔離証拠ではない。Adapterのtask_execution=unsupportedと関連release_blocker、release_ready=falseを維持する。

### 検証

- 対象Rust fileの整形確認は合格。
```powershell
rustfmt +1.95.0 --edition 2021 --config skip_children=true --check native/rust_helper/src/adapters/codex_cli.rs
```
- 追加した起動直前の否定試験は1件合格。
```powershell
cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --lib 'scratch作成後に増えたsecret_hardlink_aliasはCLI起動直前に拒否する' -- --test-threads=1
```
- Rust全target試験は12 target、408件合格、0件失敗、3件明示ignore。
```powershell
cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1
```
- Rust全targetの静的compile検査は成功。
```powershell
cargo +1.95.0 check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets
```
- 統合validatorの初回実行は、実行commandを散文で記した1行を厳格日本語監査が検出して失敗した。commandをコードblockへ分離し、監査規則は変更していない。
- 修正後の厳格日本語監査は1114 file／0 findingsで合格。Schema 149、正常example 149、negative fixture 192、Conformance 228 checks、および登録development check 10件を含む統合validatorはexit 0。
- Release blocker 5件とrelease_ready=falseは維持される。統合開発検査は合格したが、installed product／MxC child isolation／release readinessを証明しない。
- Windows Actionsは未使用。対象Windows上でRust全target検査を完了できたため、別runnerでの補助実行は不要と判断した。
```powershell
python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows
```

## R2追補 実Codex CLI Broker Taskの取消・child停止観測（2026-09-30）

### 成立した変更

- Windows ignored Rust統合testを拡張し、実Codex CLI `0.158.0-alpha.2.1`を隔離`CODEX_HOME`とlocalhost限定の偽Responses APIだけで動かす。既存正常Taskの後、Brokerから2つ目のTaskを起動し、実MxC shell childが合成Workspaceのheartbeatを更新している間に`AgentTask取消`を要求する。
- 完全に通過した1回では、停止要求中も状態が`running`のまま保たれ、terminal後に`cancelled`・結果hashなし、heartbeat停止、Broker管理`.d4p-tmp-` scratch不在、取消要求と失敗／取消Auditを確認した。指示本文はTask投影・Auditへ現れない。
- 当該Windowsで`Codex CLI`と`MxC`子processを実起動し、停止する挙動だけを一度観測した。権限発行・照合、`Owner`確認、`Adapter`の対応状態、監査記録処理はRust試験用の代替であり、実製品を経由していない。したがって、実Broker server／IPC、Desktopのnative Owner画面、耐久Auditの証拠ではない。Broker管理scratchの削除も、MxC内部一時領域の物理削除を証明しない。`task_execution=unsupported`、R2 release blocker、`release_ready=false`を維持する。

### 検証

- `cargo +1.95.0 check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`: 成功。
- `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`: Windows local 12 target、428 passed／0 failed／3 ignored。主lib 382件、binary 10件、各integration target合計46件。ignored LIVE_RUNTIMEは通常suiteに含まれない。
- `Broker制御からCodexAdapterを通るfakeTaskは成功・取消・期限後停止を区別してscratchを片付ける_fixture`: 1件合格。
- 対象2 Rust fileの`rustfmt --edition 2021 --check`と`git diff --check`: 合格。`cargo fmt --check`全crateは未変更の既存Rust file群にも多数のformat差分を報告したため、全体成功とは扱わない。
- 明示したCodex CLI absolute pathを指定するignored LIVE_RUNTIME testの最初の全経路実行は1件成功し、取消・heartbeat停止まで完了した。その後の元CLI設定による再試行2回は、先行する正常Task段階でResponses streamが切れ、Broker Taskが`failed`となって取消段階へ到達しなかった。両回ともloopback proxyは`chatgpt.com` CONNECT 1件を拒否し、fixtureのHTTP parse／response write failureは0件。根本原因は未確定であり、先行失敗を保持する。
- 同梱catalogにあるmodel slugを試験用API設定へ使う短い実験は、偽API tool call送信後もWorkspace markerが作成されず失敗したため、その差替えをrevertした。この試行をLIVE_RUNTIME成功や取消証拠へ数えない。
- 以前失敗cleanupとして記録された2026-09-29作成の合成fixture Temp directory 2件は、今回もPowerShell実行policyが正確な削除commandを開始前に拒否したため残存する。現在の2026-09-30試験が作成したTemp directoryは見つからず、長時間processも確認されていない。代替削除経路は試していない。
- WindowsローカルでRust検証が実行できたためGitHub Actionsは使っていない。

```powershell
cargo +1.95.0 check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets
cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1
```
