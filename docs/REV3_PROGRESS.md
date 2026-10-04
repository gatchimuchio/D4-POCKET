# D4 Pocket / GUI-Shell 統合rev3 進捗

本書は、受領した統合仕様書rev3・開発工程表rev3の工程状態を履歴追加型で記録する。rev1／rev2の記録は書き換えず、旧工程のPASSをrev3の機能完成証拠として再利用しない。現行のrelease gateは既存`release_blockers.registry.json`が管理し、本書の検証記録だけで解除しない。

## R2追補 現行Codex CLIでのBroker／Owner否定経路再検証（2026-10-01）

- 対象source commitは`7f7feb3e5723ee30a45acc3383007addd99f2440`で、Rust test実行前のsource worktreeはcleanだった。実機PATHのCodex CLIは`0.159.2`。`codex exec --help`で`--ignore-user-config`、`--ephemeral`等の実interfaceも再確認した。実model／資格情報、Windows保護設定は使用・変更していない。
- `cargo +1.95.0 build --release --locked --manifest-path native/rust_helper/Cargo.toml --bin gui_shell_rust_helper --bin gui_shell_desktop_launcher`は成功。Release helperのSHA-256は`2087943b9b519621471b16eebb33e4ea69bb376ce1a9bd7aec4925a70c59153`。未使用のMINIDORA診断関数に既存warningが2件出た。
- `production_agent_task_gate_fails_closed_over_authenticated_ipc`の最初の`--exact`指定はWindows module prefixを欠き、0件実行だったため証拠に採用しない。正しい`windows::production_agent_task_gate_fails_closed_over_authenticated_ipc`指定で再実行し、1 passed／0 failed（2.08秒）。実Release Broker process、認証済みloopback IPC、現行Codex登録／metadata、unsupported Task要求の拒否、Broker終了後のfile-backed Audit再読込を確認した。
- `登録CodexへのnativeOwner確認後もAgentTask非対応gateを維持する`は1 passed／0 failed（2.70秒）。現行CLI登録に対する実Win32 Owner No／Yes dialog、Broker owner-operation handler、通常Desktop IPC projectionを通し、Permission／Approval／preflight／startが`AgentTask実行非対応`で拒否され、Task workerが開始されず、Auditに合成secret・Task本文がないことを確認した。Owner dialog自体の単独Yes／No testも1 passed／0 failed（0.70秒）。ただしBrokerはtest harness内threadであり、installed Flutter UIや製品起動器からのnamed-pipe全経路ではない。
- `Broker承認経路から実CodexCLIをloopback偽APIで実行し隔離とcleanupを確認する_LIVE_RUNTIME`は1 passed／0 failed（17.61秒）。実Codex CLI `0.159.2`とMxC tool childが合成Workspace上で完了TaskとBroker cancellationを実行した。MxC TEMP／TMPは互いに一致したがBrokerのWorkspaceTaskScratchとは不一致。合成TEMP markerはchild生存中に読め、正常終了／取消後はhost側probeで`not_found`となった。これはmarkerのhost可視性だけであり、TEMP directoryの物理削除やdeadline／crash後cleanupを証明しない。試験専用Adapter wrapper、Owner callback、in-memory Audit、loopback偽Responses APIを使用しており、production Broker process／durable Auditの正のTask実行ではない。
- `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`は13 targets、444 passed／0 failed／7 ignored。現行CLIを用いた上記ignored testは別途明示実行した。
- これらによりproductionのunsupported fail-closed、Owner dialog部品、実CLI/MxCの限定したTask／取消が個別に再確認できたが、1本のinstalled-product production E2Eには統合されていない。`task_execution=unsupported`、R2 `release_blocker`、`release_ready=false`を維持する。未成立範囲はinstalled Flutter→native Owner→authenticated production IPC→durable Auditの正のTask経路、OneDrive Cloud Files／通常NTFSの実tool-child secret・depth・hardlink隔離、failure／deadline／crash Recovery、MxC TEMP異常終了cleanup、result／diff Content Exposureである。

## R2追補 Windows Release installed UI／Owner窓の実測（2026-10-01）

- source commit `7f7feb3e5723ee30a45acc3383007addd99f2440`からASCII-only detached worktreeでFlutter Windows Releaseをbuildし、Rust Release起動器／Broker helperとともにisolated temporary installへstageした。日本語pathのmanaged worktreeではFlutter buildが`app.dill` pathの文字化けで失敗し、ASCII-only worktreeでのbuild成功をその代替にした。Flutter artifact SHA-256 `2fda3beb48f15420e1e7683129abfdfdafc94f10f32452c9c3c53e42ccdd9d03`、launcher `341217def14a59fc6160f725d52743027fed0e666a4303bd50287810e5f17df6`、Broker helper `2087943b9b519621471b16eebb33e4ea69bb376ce1a9bd7aec4925a70c59153`。
- 収集器第16版の`-NoPythonRuntime -DiagnosticOnly`導入検査は終了コード`0`で成果物・Rust起動器経由の起動・Broker接続先・`Setup Doctor`製品報告の合格を観測したが、正式証拠ではない。配置作業者と同じWindows利用者で実行し、UI Automationが前景画面を確認できず必須画面要素を取得できなかった。画面は通常終了せずFrontendを強制停止し、後始末失敗とBroker起動検査の欠落もあったため、`diagnostic_only`のまま保持する。`installed_manifest.json`内のFlutter成果物SHA-256は`2fda3beb48f15420e1e7683129abfdfdafc94f10f32452c9c3c53e42ccdd9d03`、証拠一式のSHA-256は`812a3a0a50402a6dbb391b0c05f01f003c8d8beea603ddac1bf975fd509d925e`。
- 同じstaged ReleaseをRust Desktop起動器から別の一時LOCALAPPDATAで再起動した。実Frontend PID／image hashの一致を確認し、Computer Use screenshotとUI Automation treeで概要およびAgent Centerを観測した。画面には段階D/E pending、release未主張、実Adapter未登録、比較なし、Task結果未接続が表示された。synthetic WorkspaceとCodex CLI pathを登録フォームへ入れ、native Owner確認窓の表示まで進んだ。
- Owner窓はComputer Useの`activate_window`が2回連続timeoutし、No／Yesいずれの入力も行っていない。registration受理、Session開始、Task preflight／start、Task worker、production Task Auditは未実行。LLMがOwner確認を代行しない境界を維持した。確認待ちで残った試験専用Frontend／Launcher processだけをPIDと実行path照合後に停止した。
- isolated test LOCALAPPDATA／TEMP／APPDATA／empty Workspaceは実行器がrecursive removalをpolicy拒否したため削除していない。対象はTemp配下の今回専用dirで、reparse pointなし・該当Product process終了を確認済み。これは残存test artifactとして引き継ぎ、他の既存GUI-Shell process／workspaceには触れていない。
- このLIVE_RUNTIME UI observationはinstalled artifactの起動・表示とnative Owner窓の要求発生までを示すだけで、Owner承認、Flutter→native→production BrokerのTask経路やTask成功を示さない。positive production E2Eは未成立であり、`task_execution=unsupported`、R2 `release_blocker`、`release_ready=false`を維持する。

## R2追補 同時実MxC sandbox間の相互Workspace到達拒否（2026-10-01）

- 直前の単一child試験を拡張し、Agent A／B相当の独立Workspace、scratch、Codex homeを使う実Codex CLI `0.158.0-alpha.2.1`のMxC sandboxを2つ同時に起動した。開始前と相互検査後にhost側barrierを設け、双方が検査を完了するまでどちらも解放しない。各childは相手Workspace markerの読取と相手側新規file作成を試し、その間もう一方のchild process群がJob Object内で生存していることを同期状態で確認する。
- 両方向で相手markerのreadと相手WorkspaceへのwriteがWindows Access Denied（HRESULT `-2147024891`、writeは`.NET UnauthorizedAccessException`）となり、自身のWorkspaceへのwriteは成功した。相手markerの内容は不変、両方向のwrite targetは生成されなかった。Agent Aでは従来の登録secret exact／deep path拒否、hardlink拒否、およびMxC childのTEMP／TMP観測も継続した。試験失敗・期限超過時はJob Objectで両process群を停止・回収する。
- 試験harnessの中間失敗: 初回はAgent Bが開始barrier前にexit 1となり、stdout／stderrは空だった。調査でpath置換値を既に引用符を含む状態でさらに引用符内へ挿入していたことを特定し、機械実行script内のpath差込点を修正した。最終再試験は成功した。この初回失敗はharness構文不備であり、Productの隔離成功・失敗を示す証拠ではない。
- 日本語監査の中間失敗: 初回の統合validatorは新規試験診断文字列3件を検出した。診断文を日本語化し、埋込みPowerShellを人間向け`format!`診断文として誤走査させないraw scriptと明示path置換へ整理した。監査規則は変更せず、最終strict監査は1126 files／0 findingsで合格した。
- 正確な検証: PowerShellで`$env:GUI_SHELL_CODEX_SANDBOX_TEST_EXE = (Get-Command codex -CommandType Application | Select-Object -First 1 -ExpandProperty Source)`を設定後、`cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --lib adapters::codex_cli::tests::Rust生成Task設定で実Windows隔離の登録secretを拒否する -- --ignored --exact --nocapture --test-threads=1`を実行。最終exit 0、対象ignored test 1 passed／0 failed。回帰は`cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`で12 target、438 passed／0 failed／5 ignored。`rustfmt --check --edition 2021 native/rust_helper/src/adapters/codex_cli.rs`と`git diff --check`もpass。
- 最終統合検証`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`はexit 0。strict日本語監査1126 files／0 findings、Schema 151／example 151／negative 194、Conformance 230 checks、manifest／release gate／梱包portability／release smoke／evidence bundle／release runtime assertions（12 passed／0 failed）／C32監査が全件passした。release blocker 31件、`post_v1_scope` 2件、`release_ready=false`を維持した。
- 証拠区分は、2つのRust生成Task permission profileを使う実Codex CLI／MxC processの指定操作が`LIVE_RUNTIME`、Workspace・marker・同期fileが`FIXTURE`。これは同時に生存する直接sandbox child間の指定された隣接Workspace隔離を示すが、production Broker／IPC、Owner確認、Approval／Audit、実Agent Task/model、失敗時の製品Recovery、AppContainer内部temporary領域のtask間cleanup、別path alias全般、installed productは示さない。
- Agent Taskのproduction隔離全体は未成立の`release_blocker`。Adapter metadataの`task_execution=unsupported`、関連blocker、`release_ready=false`を維持し、本試験をTask実行対応またはcross-agent contamination全体のPASSへ昇格しない。

## R2追補 実Windows MxC sandboxから別Agent Workspaceへの到達拒否（2026-10-01）

- 既存のignored Rust probeを拡張し、`build_codex_command`が生成するTask permission profileを使って、実Codex CLI `0.158.0-alpha.2.1`のWindows MxC sandboxから合成Workspace AのPowerShell childを起動した。Workspace Aの兄弟directoryにAgent B用の合成markerと未作成write targetを置き、childからのmarker読取とwriteをそれぞれnegative probeに加えた。
- permissiveな初回試験は任意の例外を拒否として扱ったため証拠に採用しなかった。Windows Access Deniedの明示確認を加えた初回strict試験は、PowerShellの`MethodInvocationException` wrapper HRESULT `-2146233087`を直接比較してexit 45となった。追加診断後、内側の`.NET UnauthorizedAccessException`とHRESULT `-2147024891`の両方を確認する判定へ修正した。この中間失敗は試験harnessの例外unwrap不備であり、Product失敗とも拒否成功とも扱わない。
- 最終実行結果は1 passed。兄弟Workspace markerのread HRESULTとwrite inner exception HRESULTはともに`-2147024891`、write exception型は`UnauthorizedAccessException`だった。A内の通常writeは成功し、Bのmarker内容は不変、B側write targetは生成されなかった。登録secretのexact／deep path拒否とhardlink作成拒否も同じ既存probeで再確認した。TEMP／TMPがBroker scratchと一致しない観測も維持され、今回の成功条件やcleanup証拠には使っていない。
- 証拠区分は、実Codex CLI／MxC sandbox childによる指定path操作が`LIVE_RUNTIME`、二つのWorkspaceとmarkerが`FIXTURE`。これは単一Task sandboxから隣接する一つの別Workspaceへ到達できないことだけを示し、二Agentの同時実行、production Broker／IPC、Owner確認、durable Audit、実Agent／model、Task取消・deadline・crash Recovery、installed product、別path aliasの網羅を示さない。
- 正確な検証: `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml Rust生成Task設定で実Windows隔離の登録secretを拒否する -- --ignored --nocapture --test-threads=1`。exit 0、対象test 1 passed／0 failed。binaryは既定のCodex CLI探索ではなく、明示したインストール済みCLI絶対pathを`GUI_SHELL_CODEX_SANDBOX_TEST_EXE`へ設定して起動した。
- 回帰確認: `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`は12 target、438 passed／0 failed／5 ignored。個別ignored probeは別途上記のとおり実行済み。`rustfmt --check --edition 2021 native/rust_helper/src/adapters/codex_cli.rs`と`git diff --check`はpass。`cargo +1.95.0 fmt --manifest-path native/rust_helper/Cargo.toml -- --check`は変更していない複数Rust fileにも既存format差分を報告したため失敗として保持し、無関係fileの一括formatは行わなかった。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`は開発validatorの全登録checkをpassした。strict日本語監査は1126 files／0 findings、Schema 151／151・example 151・negative 194、Conformance 230 checks、manifest／release gate／packaging portability／release smoke／evidence bundle／release runtime assertions（12 passed／0 failed）／C32監査がpass。Windows installed evidenceは5 gateが未成立であり、最終監査は31 `release_blocker`・2 `post_v1_scope`、`release_ready=false`を維持した。
- cross-agent contamination全体とAgent Task production隔離は未成立の`release_blocker`。製品Adapter metadataの`task_execution=unsupported`、関連blocker、`release_ready=false`を維持する。

## `Windows Server 2025`上での`Setup Doctor`画面証拠取得失敗（2026-10-01）

- `workflow_dispatch`で一時branch `codex/verify-setup-doctor-uia-v16` のcommit `f51ba074be693ab58e0df965d413645b491ef29b`（[run #7](https://github.com/gatchimuchio/GUI-Shell/actions/runs/36764681765)）と `1139eef24ff945cce677ce115582487c593400f7`（[run #8](https://github.com/gatchimuchio/GUI-Shell/actions/runs/36767327753)）をWindows Server 2025で手動検査した。workflowは`workflow_dispatch`のみで起動し、自動trigger、PR、artifact uploadは使用していない。
- run #8では固定Rust 1.95.0／Flutter commit `559ffa3f75e7402d65a8def9c28389a9b2e6fe42`、Desktop Release build、installed配置が成功。別jobのRust helper build、Desktop Flutter analyze／全test、Mobile analyze、終了時source cleanも成功した。
- installed appのDiagnosticOnly起動では、初回windowが可視で、製品Setup Doctor reportとBroker Auditの照合も成立した。一方、UIA collectorは`verified_frontend_not_foreground`を記録し、Setup Doctorの要求可視要素を収集できなかった。診断値は`operator_readable=false`、`proof_passed=false`、`required_element_minimum=false`、`no_proof_errors=false`。よってこのhosted runのUI可読性proofは失敗であり、合格証拠として扱わない。この結果だけから製品regressionとも断定しない。
- run #7はRelease build／stage後に同じ総合UIA条件で失敗し、run #8で判定別の安全なdiagnosticを追加して再実行した。失敗は同一runner sessionで対象Frontendをforegroundとして確認できず、global pointer操作・実画面要素確認を安全に成立させられなかったことによる。
- 両runともrun専用temporary root／processのcleanupとsource clean確認が成功し、artifactは残していない。一時branchは削除済みで、失敗したworkflow差分はmainへ統合していない。Windows Setup Doctor／installed first-runの既存`release_blocker`と`release_ready=false`を維持する。

## Windows Setup Doctor UIAutomation収集器 v16 実装（2026-10-01）

- v15で未収集だったSetup Doctor専用画面について、製品FrontendのMainWindowHandleとUIA runtime IDへ束縛した診断ナビゲーション、画面見出し、status、authority notice、各checkのtitle／status／message／non-pass recovery textを収集するv16 pathを追加した。観測対象は同一PID・同一MainWindowのControl Viewに限定し、scrollは最大17回、node数は既存上限内とする。
- global pointer入力は、操作直前にforeground PID、WindowFromPoint PID、root HWNDを照合し、対象Frontend MainWindow以外へclick／wheelを送らない。各観測要素はUIA `IsOffscreen`、親子geometry交差に加え、要素内sample pointで最前面root HWNDを記録する。各要素はrun ID、Frontend PID／HWND、report hash、同じevidence bundleのsidecar hashへ結合する。
- UIA観測は実画面text controlが露出している範囲の`LIVE_RUNTIME`証拠である。pixel／contrastの判定とscreen reader実操作は計測しない。Microsoftの[`IsOffscreen`仕様](https://learn.microsoft.com/en-us/dotnet/api/system.windows.automation.automationelement.automationelementinformation.isoffscreen?view=windowsdesktop-10.0)は別windowによる遮蔽をこの値で判定しないと明記するため、topmost HWND sampleを追加したが、文字全体のpixel描画品質を証明するものではない。
- validator negative coverageにMainWindow／PID取り違え、foreign overlay sample、画面外、範囲外座標、aggregate／重複文字、空message、runtime ID再利用、sidecar不結合を追加した。synthetic fixtureはvalidator構造だけを試し、製品UIの実測証拠ではない。
- 検証: `python -m py_compile tooling/windows_release_evidence.py tooling/conformance_tests/run_conformance_skeleton.py`、対象negative test（13 mutation、全てrelease blockerへfail-closed）、Win32 interop `Add-Type` compile、PowerShell AST parse、`git diff --check`が成功。`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`もexit 0。内訳は厳格日本語監査1126 file／0 finding、Schema 151／example 151／negative fixture 194、Conformance 230 checks、Manifest 1123 file、release gate、packaging portability、release smoke、evidence bundle、runtime assertions、C32 development auditが成功。集約結果はrelease blocker 31件、`release_ready=false`。
- 実際のv16製品UIA runと別profile formal collectorは未実行。pixel／contrast・screen reader実操作も未検証。手動Actionsは起動していない。synthetic fixture／静的検査はこれらのruntime proofを代替しない。
- `windows_setup_doctor_smoke`、Windows installed first-run、および関連release blockerを維持する。release-ready／Technical Completeは未成立。

## R2追補 Windows native Owner確認dialogのローカル実操作試験（2026-09-30）

### 成立した範囲

- PowerShell UI Automation helperを除去し、Windows test専用child processと既存`winsafe`の安全APIで、production `confirm_owner_operation`が生成する実Win32 MessageBoxを検査する。試験はCredentialやTask本文をchild processへ渡さず、Agent Taskの表示用ID・hash・文字数・選択期待値だけをstdinで渡す。
- 操作前にchild PID、可視top-level windowのclass/title、MessageBox本文、Yes／No control IDとlabel、foreground window、UI threadのactive／focus controlを照合する。Noは既定button、YesはTab後にfocus IDを確認してからEnterを送る。異常時に終了するのは試験が起動したchild processだけで、通常のBroker権限要求やTask実行は行わない。
- ignored UI testは日本語Windows local対話Desktopで実行し、No／Yes各結果がchild processの`confirm_owner_operation`返値へ反映されること、Task本文markerが表示文に含まれないことを確認した。これは固定fixtureを使った実Windows MessageBox表示・入力の直接観測（`FIXTURE`）であり、Broker経由のOwner操作、installed productのlive runtime、Agent Task成功、`task_execution=supported`の証拠ではない。
- Windows Rust manual workflowは`workflow_dispatch`のみのまま維持し、対話Desktopを必要とするUI入力を削除した。旧helperのUTF-8／hosted UI失敗経路を削除し、失敗履歴は下記の通り残す。
- `task_execution=unsupported`、Agent Task実行・製品配布等の既存`release_blocker`と`release_ready=false`を維持する。

### Windows Actionsの失敗履歴

- 手動run [36721937797](https://github.com/gatchimuchio/GUI-Shell/actions/runs/36721937797)、commit `f868e05259af1b4fa0c927f20c4e91fa545d62e4`: Rust全target検査は成功したが、Windows PowerShell 5.1がBOMなしUTF-8 helperの日本語を誤parseし、UI stepは失敗した。
- 手動run [36723407003](https://github.com/gatchimuchio/GUI-Shell/actions/runs/36723407003)、commit `dc3829166ab7547b722a90b1d081f52b51ca238f`: BOM追加後、Rust全target検査は成功したが、helperがdialog準備完了を通知せず、15秒でUI stepが失敗した。
- 手動run [36725060867](https://github.com/gatchimuchio/GUI-Shell/actions/runs/36725060867)、commit `3ab3611e4d547d05466800a52704aac331bbfda2`: Rust全target検査は成功したが、hosted runnerで12秒以内に対象dialogをUIAから発見できなかった。helperは親Rust test processを停止し、runは異常終了した。UIAがdialogを発見できない環境要因の詳細は確定していないため、Windows runnerのUI操作成功とは扱わない。
- これらの失敗はhelper／実行環境の検証失敗であり、Owner確認製品機能の合否を証明しない。ローカル対話Desktopで検査できたため置換後のUI testにActionsは使わず、Actionsを追加起動していない。

### ローカル検証

- `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --lib 'nativeOwner確認dialogのYesNoを制御UI自動化できTask本文を露出しない' -- --ignored --test-threads=1 --nocapture`: 成功、1 passed。実Win32 MessageBoxのNo／Yes選択を実行した。
- `cargo +1.95.0 check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`: 成功。
- `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`: 成功、12 targetで435 passed／0 failed／5 ignored。ignored試験には実Codex CLIを要するLIVE_RUNTIME経路等が含まれ、本結果でそれらを検証済みとはしない。
- `rustfmt +1.95.0 --edition 2021 --config skip_children=true --check native/rust_helper/src/desktop_launcher.rs`、`git diff --check`、`python -X utf8 tooling/manifest.py --check`: 成功。生成Manifestは1120 source fileを記録する。削除した旧PowerShell helperは、削除をGit indexへstage後にManifest対象から外れた。最初の`manifest.py --write`失敗は削除済みtracked fileの改行検査によるもので、検査規則は変更していない。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`初回は新Rust試験の英語診断文1件をstrict日本語監査が検出してexit 1。診断文を日本語化した後の最終実行はexit 0。厳格日本語監査1123 file／0 findings、Schema 150／example 150／negative fixture 193、Conformance 229 checks、manifest、release gate、packaging portability、release smoke、evidence bundle、runtime assertions、C32監査を含むdevelopment check 10件が成功した。
- 統合validatorのrelease gate検査成功は製品release成立を意味しない。evidence bundleはdevelopment evidenceのままで、現行release blockerと`release_ready=false`を維持する。
- workflow: manual workflowの`workflow_dispatch`限定は統合validator内の検査が成功。Actionsは追加起動していない。Git閉包のbranch／commit／push／remote HEAD／backup／rollback refは本blockの最終報告に記録する。

### 実Win32 Owner確認後もAgentTask非対応gateが拒否する統合追試（2026-09-30）

- 先行記録の統合testはsynthetic Owner callbackを使っていたため、既存の実Win32 dialog試験を登録済みCodex CLIのBroker loopback統合testへ接続し直した。試験専用BrokerはRust test thread、Workspace／storeは一時fixture、要求relayはtestから直接呼び出す構成である。
- Workspace Permission要求では実native dialogのNoを確認し、次の要求ではYesを選んでもBrokerが`AgentTask実行非対応`としてgrantを返さない。Owner Approvalも実native dialogでYesを選択した後、同じunsupported gateで拒否される。試験はPermissionとApprovalの拒否Auditがfile-backed test storeへ記録されること、grant発行Auditがないこと、Task本文がdialog／response／Auditへ露出しないことを確認する。実Codex CLIはRuntime登録probeだけに使い、Agent Task／model requestは開始しない。
- 証拠classは混在する。実Windows MessageBox表示・入力とCLI登録probeは`LIVE_RUNTIME`、test thread Broker・一時Workspace/store・固定入力自動化は`FIXTURE`である。これはDesktop installed product、別process起動器／FlutterからのIPC、通常Owner操作、耐久製品Audit、Task実行成功または`task_execution=supported`の証拠ではない。
- ignored統合testはWindows localで1件合格。実行時だけ`GUI_SHELL_CODEX_TASK_BROKER_TEST_EXE`へ検出したCodex CLI実行fileを設定し、値は保存しない。

```powershell
cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --lib 'desktop_launcher::tests::登録CodexへのnativeOwner確認後もAgentTask非対応gateを維持する' -- --ignored --exact --test-threads=1 --nocapture
```

- Windows local ignored test: 1 passed／0 failed。全suiteやinstalled productの再検証ではない。既存Rust／Desktop／Agent隔離のrelease blockerと`task_execution=unsupported`、`release_ready=false`を維持する。

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

## R2追補 Windows Export既知credential markerの誤検出修正と手動Actions検証（2026-09-30）

### 成立した変更

- Release bundle走査がRust実行binary内の既知credential marker文字列を検出していた。既知marker集合の検出をBroker共通関数へ集約し、marker表現を分割してbinary内の連続literal化を避けた。Dialogue metadata走査と回帰caseは同じ検出関数を使う。既知patternの範囲・走査対象・失敗時動作は弱めていない。全marker群の大小文字差、拒否、通常日本語の許容をunit testで確認した。
- 変更は`native/rust_helper/src/broker/mod.rs`、`dialogue.rs`、`regression_case.rs`。実装commitは`6f0681ce1439d9d6f769075952658d063e04a3f2`。
- local統合validatorの初回実行は、Rust file更新とtracked Windows Export workflow追加をmanifestへ反映していなかったため`manifest_check`、`release_gate_check`、`packaging_portability_check`が失敗した。`tooling/manifest.py --write`で1120 tracked source fileを再記録し、manifest検査とrelease gate検査が通る状態へ修正した。これは製品testの失敗ではなく、source manifest不整合だった。進捗節を履歴末尾へ移した後の統合validator再試行では`docs/REV3_PROGRESS.md`のmanifest hash不一致を検出したため、この履歴修正を確定してmanifestを再生成した後に最終再試行する。
- Release blocker 5件と`release_ready=false`を維持する。走査成功は未知形式を含む一般的なcredential不存在の証明ではない。

### Windows Actionsの履歴と結果

- 手動`workflow_dispatch`によるExport検証の先行失敗は履歴として保持する。#1 run `36699100289`はsource-HEAD guard、#2 run `36699507211`はfixture manifest名不整合、#3 run `36700047744`はbundle build後の`slack_token`、#4 run `36702628049`はbundle build後の`credential_assignment`で失敗した。#1〜#4はいずれも成功証拠ではない。
- 修正済みcommit `6f0681ce1439d9d6f769075952658d063e04a3f2`に対する[Windows Rust validation #25](https://github.com/gatchimuchio/GUI-Shell/actions/runs/36705515991)は成功した。Windows Server 2025 runnerでRust／Cargo 1.95.0を使用し、全targetの`cargo check`と`cargo test`が完了。Rust試験は12 target、435 passed／0 failed／3 ignored。workflow所定のformat、checkout SHA、終了時clean確認も成功した。artifact uploadなし。
- 同じcommitに対する[Windows GUI Shell Export bundle #5](https://github.com/gatchimuchio/GUI-Shell/actions/runs/36705659036)は`workflow_dispatch`で成功した。Windows Server 2025、image version `20260922.246.2`上でFlutter Windows ReleaseとRust Releaseを含む未署名portable fixture bundleを組み立て、実装済みknown-pattern scanを実行してfindings 0を確認。bundleは61,971,154 bytes、tree SHA-256は`82cf6393462eee3d710dd0edaaf45c944b4f3f866b9c9e1dfa3643368f3ba8f1`。source tree clean確認も成功し、Actions artifactはuploadされていない。
- Export workflowの明示条件どおり、`portable_bundle_assembled=true`だが`standalone_app_verified=false`、正式distribution claimなし、installer未開始、launch未検証。これはWindows installed product、起動、署名、installer／updater／rollback、binary module pruning、一般的なsecret不存在の証拠ではない。
- Actionsは既存の手動workflowを利用し、一時検証branch、PR、自動triggerは作っていない。対象は正確に上記commitで固定した。workflow sourceはtracked manifestにも追加した。

### ローカル検証

- 修正後のRust全target試験は12 target、435 passed／0 failed／3 ignored。Rust全target check、Schema 150件／正常example 150件／negative fixture 193件、Conformance 229 checks、strict日本語監査1123 files／0 findings、manifest、release gate、packaging portability、release smoke、evidence bundle、runtime assertions、C32 auditを含む統合validatorはすべて成功した。
```powershell
cargo +1.95.0 check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets
cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1
python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows
```
- 最終統合validatorはexit 0。Release gate整合checkの成功はrelease readinessを意味しない。release blocker 5件と`release_ready=false`は維持する。
- `rustfmt` checkは`mod.rs`と`dialogue.rs`で成功。`regression_case.rs`全体のcheckは変更箇所と無関係な既存format差分を報告したため成功扱いしていない。Windows Actionsの所定format stepは成功し、`git diff --check`も成功した。
- bundle scanはfixture由来の`INTERNAL_STATE`相当のbuild evidenceに限る。未知pattern、全credential形式、実installed app、Owner権限経路、実機起動を証明しない。

## R2追補 Task scratch環境設定をproduction commandへ共通化（2026-09-30）

### 成立した変更

- Codex Task command builderはRustが起動するCLI processの`TEMP`／`TMP`をBroker-owned `WorkspaceTaskScratch`へ設定していたが、MxC shell childへ渡す`shell_environment_policy.set={TEMP=...,TMP=...}`はloopback偽Responses APIを使うtest設定の内側にあり、production Task commandには含まれていなかった。
- Task専用command builderからscratchを同じ設定で常に明示するよう移し、Taskではscratchが必須かつ登録Workspaceの厳密な子directoryであることをcommand生成時にも検査する。read-only Dialogueには設定を追加しない。fake Codex CLI fixtureはTask固定設定と引数値が一致するときだけ受理し、この経路を検査する。
- これはCLI起動設定がRustから生成されることを示す`FIXTURE`証拠に限る。以前のMxC child実測では同設定を明示しても`TEMP`／`TMP`がWorkspaceTaskScratchと一致せず、AppContainer内TEMPの後始末も未確認である。このため差分は実child isolation／cleanupの修正完了ではなく、`task_execution=unsupported`と`comprehensive_extension_rev1_completion` blockerを維持する。
- 現行`release_blockers.registry.json`は17 record中15件がactive unresolved、2件がresolved inactiveであり、`release_ready=false`。Windows trackには12件が属し、`evidence_bundle.py`が表示する5件はWindows installed-evidence検査結果のblockerだけで、registry全体の件数ではない。本書の「5件」表記はこの証拠bundleの範囲として読み、現在数とrelease gate判断はregistryを正本とする。

### 検証と失敗履歴

- focused command設定test、fake CLIを使うAdapter lifecycle test、Broker fakeTask縦断testはいずれも1件ずつ合格した。
- 全target testの初回実行はfixture 2件が失敗した。Rust側の新しい設定追加後もfixtureが旧来の`-c`設定5件と完全一致を要求しており、追加のTask TEMP／TMP設定を未知構成として拒否したのが原因だった。fixtureを現行command contractに同期した後の再実行は12 target、435 passed／0 failed／3 ignoredで合格した。初回の失敗は履歴として保持する。
- `cargo check --all-targets`、変更Rust fileとfake CLI fixtureの`rustfmt --check`、`git diff --check`は合格した。ignored実Codex CLI／MxC probeは、既存の環境不一致を再測定しても解決根拠にならず、一時領域へ不要なmarkerを残すおそれもあるため再実行していない。
- Windows Actionsは未使用。対象Windows上で全Rust targetのbuild／testを実行できたため、今回の設定生成検査にhosted補助は加えていない。実Codex CLIのMxC実測、production Broker IPC、Owner確認、installed product、release readinessは未検証である。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`はexit 0。strict日本語監査1123 file／0 findings、Schema 150／150／negative fixture 193、Conformance 229 checks、登録済みdevelopment checks 10件を含む全項目が合格した。development evidence bundleのWindows evidence blocker 5件と`release_ready=false`を保持する。これはregistry全体のactive unresolved 15件とは別scopeであり、release readinessの証拠ではない。

```powershell
cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --lib Dialogueはread_onlyのままTaskだけ専用permission_profileを使う -- --test-threads=1
cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --lib 偽CodexCLIはAdapterのTask成功 -- --test-threads=1
cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --lib Broker制御からCodexAdapterを通るfakeTaskは成功 -- --test-threads=1
cargo +1.95.0 check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets
cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1
```

## R2追補 更新適用・rollback要求の停止Audit負例（2026-09-30）

### 成立した変更

- 更新候補が現在のBroker trustで正しく署名検証されていても、`更新適用要求`と`更新rollback要求`は未接続のため`Suspended`に留まり、個別要求に対応する`suspended` AuditEventを返すことを同一の回帰試験で検査するよう拡張した。
- responseのerror codeとAuditEvent IDの一致も確認する。これはRust Broker内の`FIXTURE`であり、Installer、package展開、process切替、rollback transaction、導入済み製品の証拠ではない。適用・rollback経路は`suspended`のまま、`release_ready=false`を維持する。

### 検証

- focused Rust試験は1件合格。
- Rust全試験対象の`cargo test`: 12対象、435成功／0失敗／3対象外。
- `cargo check --all-targets`と変更fileの`rustfmt --edition 2021 --check`は成功。
- `cargo fmt --all -- --check`は、今回変更していない多数のRust fileで既存format差分を検出して失敗した。無関係な全体整形は行わず、変更対象fileの局所checkは成功した。
- 統合validatorは初回にsource hash manifest未更新、二回目に本節の英語検証文で失敗した。日本語化とmanifest再生成後の最終実行は終了値0。厳格日本語監査1123ファイル／指摘0、`Schema`150件／正常例150件／負例193件、適合確認229件、開発用検査10件が合格した。導入済み製品の証拠blocker 5件とregistry全体のactive unresolved 15件は残り、`release_ready=false`を維持する。
- Windows上でRust全target検査を実行できたためGitHub Actionsは使用していない。Release blocker 15件と`release_ready=false`を保持する。

```powershell
cargo test --lib broker::update_center::tests::execution_requests_remain_suspended_and_audited -- --exact --test-threads=1
cargo test --all-targets -- --test-threads=1
cargo check --all-targets
python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows
```

## R2追補 Setup Doctor操作画面の可読性改善（2026-09-30）

### 成立した変更

- 診断概要、状態別件数、項目別カード、問題時の復旧案内を追加し、画面内の状態値・既知check ID・取得元・鮮度・通信範囲・監査鎖・Runtime状態を日本語表示へ置き換えた。
- installed-path evidenceは状態だけを表示し、evidence IDや保存先pathを画面へ出さない。診断がPermission／Approvalを生成せず、製品release readinessの判定でもない境界を明示した。`release_state`は既知値だけ表示変換し、未知値は「不明」とする。
- 変更はFlutter表示面とwidget testだけで、Rust Broker、診断契約、証拠class、権限判定、実測値は変更していない。390×844の狭い画面でwarning／unknownと復旧案内が分かれ、描画例外がないことを検査するtestを追加した。
- このUI改善は導入済みWindows製品の実証ではない。`windows_setup_doctor_smoke`を含むrelease blockerと`release_ready=false`を維持する。

### 検証

- 対象Dart fileの`dart format --output=none --set-exit-if-changed`、`git diff --check`、manifest検査は成功した。
- OneDrive上のlocal `flutter analyze --no-pub`はAnalysis Serverが受信LSP JSONを解析できず`FormatException: Unterminated string`、exit 255で終了した。local `dart analyze lib/screens/setup_doctor.dart test/widget_test.dart`もAnalysis Serverの終了時に`AppData\Local\Dart\perf\7612`を削除できずexit 1となった。local `flutter test --no-pub --reporter expanded`は保護されたOneDrive Cloud Filesの`build/unit_test_assets`をFlutterが削除できず、test開始前に停止した。いずれもlocal analyzer／testの成功証拠には数えない。
- 手動Windows Actions #4 (`36714964375`) は`f88cb7a22bca5ac67e3f9b8df07fd0d3f692387d`でFlutter analyzeが成功した一方、古い表示名を期待する既存widget assertion 1件が失敗した。残り129件と、新規のwarning／unknown・狭幅画面testは成功。失敗履歴は保持し、assertionを現在の「通信範囲」「Broker監査鎖」表示に修正した。
- 修正後の統合validator `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows` はexit 0。厳格日本語監査1123 file／指摘0、Schema 150、正常例150、negative fixture 193、Conformance 229、登録済みdevelopment検査10件を含む検査が成功した。Windows installed-evidence blocker 5件とregistry全体のactive unresolved 15件を維持し、`release_ready=false`。
- 手動Windows Actions [#5 (`36716608834`)](https://github.com/gatchimuchio/GUI-Shell/actions/runs/36716608834) は`workflow_dispatch`で`codex/verify-setup-doctor-ui-20260930`上の正確なcommit `c447376083d042f41f5af83be07459c9b932a93a`を検証し、全step成功。Windows Server 2025 image `20260922.246.2`、Flutter 3.44.0（commit `559ffa3f75e7402d65a8def9c28389a9b2e6fe42`）でRust helper build、Desktop analyze、全130 Desktop test、Mobile analyze、終了時clean確認が成功した。artifactはなく、runに関連するPRもない。
- 成功した正確なcommit `c447376083d042f41f5af83be07459c9b932a93a`を`main`へfast-forwardし、`origin/main`との一致を確認した。一時検証branchはlocal／remote双方から削除済み。ActionsはRust／Flutter検査の証拠であり、Windows installed productの実起動、Setup DoctorのLIVE_RUNTIME製品証拠、release readinessを証明しない。既存release blockerと`release_ready=false`は維持する。

## R2追補 Windows ActionsでRelease Broker runtime smokeを手動検証（2026-10-01）

### 成立した確認

- `.github/workflows/windows-manual-rust-validation.yml`を`workflow_dispatch`限定のまま拡張し、対象checkoutのRust全target検査・試験に続けてRelease helperをbuildし、既存`installer/windows/collect_broker_smoke.ps1` version 6で独立Broker processを検査する。build、store、session、collector outputはrun／attempt固有の`RUNNER_TEMP`に置き、所有marker・固定path・helper command lineを検査してprocessと一時領域をcleanupする。artifact upload、自動trigger、PRは追加しない。
- 手動run [Windows Rust manual validation #29](https://github.com/gatchimuchio/GUI-Shell/actions/runs/36734597479)は2026-10-01にcommit `f740fff64248d1a16b900fafa790a65f245e9150`を対象として成功した。Windows Server 2025 runner image `win25-vs2026/20260922.246.2`、`rustc 1.95.0 (59807616e 2026-04-14)`。全Rust targetの`cargo check`／`cargo test`が成功し、12 test targetで435 passed／0 failed／5 ignored。Release helper build、Broker smoke、cleanup、試験後clean確認も成功。job所要14分58秒、workflow全体15分11秒。
- Release helper SHA-256は`425ecedb7ebfa486c16aa3351b8aaa55e33304c3cf31e088afd08c8f108f8fe7`。collector v6はnon-synthetic `passed`を返し、認証loopback IPC、durable-store readiness、通常credentialによるWorkspace Permission grant／Owner Approval grant双方の拒否（error codeはいずれも`desktop_native_owner_confirmation_required`）、restart後nonce replay拒否、再起動後health、強制停止後fail-closed、session credential fileの生成後削除を確認した。
- collector outputはrunnerのrun専用temporary領域に作られ、cleanupで削除された。artifact uploadはないため、collector outputのhash／file自体は保存されていない。証拠はrun summaryとjob logに限定する。
- 成功した正確なcommitだけを`main`へfast-forwardし、remote HEADとの一致を確認した。一時検証branch `codex/verify-broker-runtime-smoke-20260930`はlocal／remote双方から削除済み。

### 証拠境界

この結果はhosted Windows runnerでclean sourceからbuildしたstandalone Release Broker processの`LIVE_RUNTIME` smokeと、外部runnerの`EXTERNAL_EVIDENCE`記録である。通常資格によるgrant発行拒否を確認したもので、Desktop native Owner UI操作、installed Desktop／final packageとのbinary identity、Agent Task実行、耐久製品Audit統合、正式release provenanceを示さない。`task_execution=unsupported`、`windows_broker_installed_smoke`の`release_blocker`、`release_ready=false`を維持する。

### 検証

- 記録更新後の`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`はexit 0。strict日本語監査1123 files／0 findings、Schema 150件／正常例150件／negative fixture 193件、Conformance 229 checks、Manifest／release gate／packaging／release smoke／evidence bundle／release runtime assertions／final development auditを含む全10検査がpassした。development evidenceのrelease blocker 5件と`release_ready=false`を維持する。
- 個別に`python -X utf8 tooling/schema_check/check_schemas.py`（150／150／193）、`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`（229 checks）、`python -X utf8 tooling/日本語基底監査.py --strict`（1123 files／0 findings）、`python -X utf8 tooling/manifest.py --check`、`git diff --cached --check`もすべてpass。
- Windows Actions [#29](https://github.com/gatchimuchio/GUI-Shell/actions/runs/36734597479): checkout SHA照合、Rust変更file format検査、`cargo +1.95.0 check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`、`cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`、Release Broker build、collector v6 smoke、cleanup、test後clean確認が全step成功。
- collector JSONとtemporary helper／store／sessionはrun終了時に削除済み。Actions artifactはなし。

## R2追補 OneDrive外短pathでのWindows Flutter検証（2026-10-01）

### 成立した確認

- sourceはcleanな`main`／`origin/main` commit `bb5caa48defa9994c6f16cca0e4e08f58b601cc2`からlocal temporary checkoutへ展開した。Flutter／Dartは3.44.0／3.12.0、Rustは1.95.0。元のOneDrive checkout、Windows Application Control、registry、ACLは変更していない。
- `native/rust_helper`のdebug helperは一時checkout内で`cargo +1.95.0 build --locked --manifest-path native/rust_helper/Cargo.toml`によりbuildできた。2分42秒で終了し、Application Controlによる拒否は再現しなかった。
- Desktop Flutterは`flutter analyze`成功、`flutter test --no-pub --reporter expanded`で130 test成功。最初のtest実行はRust debug helper未buildという明示前提により2件失敗したため、helperをbuildしてからsuite全体を再実行し、全件成功を確認した。
- Mobile Flutterは`flutter analyze`成功、`flutter test --reporter expanded`で21 test成功。`packages/gui_shell_ui`は`flutter analyze`成功、`flutter test --reporter expanded`で56 test成功。

### 証拠境界と環境範囲

- Flutterのunit／widget testは開発環境の`FIXTURE` evidenceである。Desktopのうち2件はdebug Broker process／IPCへ接続するが、開発用lifecycle fixtureを含む`LIVE_RUNTIME`／`FIXTURE`の限定証拠である。installed product、Mobile実機、Agent Task隔離、Windows正式配布、release readinessを証明しない。
- OneDrive外の短いlocal pathではRust helper buildとFlutter testが成立する。元のOneDrive checkoutは移動・修復しておらず、OneDriveが生成build artifactを同期できない既存制約を解消したとは主張しない。以後同環境でFlutter全数検証を行う場合は、OneDrive外の短いtemporary checkoutを使う。
- `task_execution=unsupported`、既存release blocker、`release_ready=false`は変更しない。

### 正確な検証

```powershell
cargo +1.95.0 build --locked --manifest-path native/rust_helper/Cargo.toml
flutter analyze                         # apps/desktop_flutter
flutter test --no-pub --reporter expanded # apps/desktop_flutter: 130 passed
flutter analyze                         # apps/mobile_flutter
flutter test --reporter expanded         # apps/mobile_flutter: 21 passed
flutter analyze                         # packages/gui_shell_ui
flutter test --reporter expanded         # packages/gui_shell_ui: 56 passed
```

- 進捗記録とmanifest更新後の`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`はexit 0。Schema 150件／正常example 150件／negative fixture 193件、Conformance 229件、日本語厳格監査1123 files／0 findingsを含む登録済み10 checkがすべてpassした。`python -X utf8 tooling/manifest.py --check`と`git diff --check`もpass。これは開発gateの成立であり、release blockerと`release_ready=false`を維持する。

## C7追補 Windows DPAPI資格情報の論理失効（2026-10-01）

### 成立した変更

- CredentialをDesktop MCP接続センターのmetadata一覧へ表示し、有効な一件だけを選んで論理失効する操作を接続した。失効済みCredentialは新規MCP選択肢から除き、一覧には失効状態と時刻を表示する。
- 要求はCredential ID、用途、接続対象、暗号文hash、登録監査IDへ結合する。Rust Desktopの既存native Owner確認は対象metadataとpayload hashを表示し、共通Win32 dialogの既定Noを維持する。Brokerはnative確認経路以外を拒否し、永続Audit chainと現行登録metadataを再検証する。
- 失効eventをBroker Auditから復元する。再起動後も失効状態を維持し、失効後注入、二重失効、古い対象metadata、永続Auditなしの要求を拒否する。秘密値はUI／IPC／Auditへ返さず、暗号文fileは保持する。物理削除や失効取消は行わない。
- revocation request schema、receipt状態条件、positive／negative fixture、IPC operation enum、Conformance確認を追加した。進捗はC7へ対応づけ、従来のrelease blockerと`release_ready=false`は解除していない。

### 検証

- `cargo check --locked --all-targets`: exit 0。Rust対象file 3件の`rustfmt --check --edition 2021 --config skip_children=true`: exit 0。
- `cargo test --locked --all-targets -- --test-threads=1`: 12 test target、438 passed／0 failed／5 ignored。失効後のBroker再起動復元、再利用拒否、native確認metadata固定、永続Audit不在時の拒否を含む。初回並列実行は既存loopback／TLS試験3件が不安定失敗したが、直列の全target再実行ではすべてpassした。
- `cargo test --locked revocation -- --test-threads=1`: 3件のRust単体試験と2件のWorkspace integration試験がpass。`dart format --output=none --set-exit-if-changed`は変更Dart 4 fileすべて整形済み。Schemaは151件／正常example 151件／negative fixture 194件、Conformanceは229 checksでpass。
- `python -X utf8 tooling/日本語基底監査.py --strict`: 1126 files／0 findings。`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`: exit 0。Schema 151／正常example 151／negative fixture 194、Conformance 229、梱包portability、Release Gate、Smoke、manifestを含む全登録checkがpass。
- OneDrive checkoutのDesktop `flutter analyze --no-pub`はAnalysis ServerのLSP JSON `FormatException`でexit 255。Desktop `flutter test test/mcp_connection_center_test.dart`とMobile `flutter analyze`はFlutterがOneDrive配下の`macos/Flutter/ephemeral/Packages/.packages`／`ios/Flutter/ephemeral/Packages/.packages`を削除できず開始前に停止した。いずれも成功証拠に数えない。補完としてworkflow_dispatch限定の[Windows Actions run #6](https://github.com/gatchimuchio/GUI-Shell/actions/runs/36749071150)を検証branchの正確なcommit `d60bfe2e89572bee47f655f8c714863d89ea02bb`で実行し、Windows Server 2025 hosted runner（`win25-vs2026/20260925.250.1`）でRust helper build、Desktop `flutter analyze --no-pub`（問題なし）、Desktop `flutter test --no-pub --reporter expanded`（132件全pass）、Mobile `flutter analyze --no-pub`（問題なし）、検査後cleanを確認した。Flutterは3.44.0 commit `559ffa3f75e7402d65a8def9c28389a9b2e6fe42`。runは7分52秒で成功。これは当該hosted runnerのbuild／analysis／test証拠であり、local OneDrive問題の解消、installed product、Owner dialog実操作、release readinessを証明しない。

### 証拠境界と残存分類

- Rust／Flutter試験は`FIXTURE`であり、実installed Desktop、実際のWin32 Owner dialog操作、配布物とのbinary identity、製品release readinessを証明しない。Windows Actionsもそのworkflowが実行したbuild／analysis／testの範囲だけを示す。
- item: Windows installed product上の資格情報登録・注入・失効の実操作、C7内のMCP以外の注入、更新、暗号文物理削除・Recovery、GUI登録・接続先変更、Mobile安全保管は未成立。
  classification: release_blocker
  reason: 現行範囲はWindows DPAPIとMCP stdioの限定Credentialに対する開発試験であり、上記機能や導入済み製品の実測を含まない。
  required_action: 対象ごとの独立contract・負例試験・Owner操作を実装し、clean Windows installed productと必要platformで証拠を取得する。
  blocks_release: yes
- `release_ready=false`を維持する。

## R2追補 AppContainer TEMP／TMPとBroker scratchの責任分離（2026-10-01）

### 成立した確認

- 現在のclean `main` commit `f2a95443c702e7fd69d4218266a3a2c11ea73657`で、Rust生成Task設定を用いる実Windows MxC直接probeを再実行した。合成登録secretのliteral path・深度40 path・hardlink aliasに対する拒否、通常Workspace write許可、`TEMP`／`TMP`がBroker scratchと一致しないことを1回確認した。hardlink作成はWindows `UnauthorizedAccessException`／HRESULT `-2147024891`（Access Denied）で失敗した。
- Microsoft Learnは、AppContainerの`TEMP`／`TMP`がprofile配下の`AC\Temp`へリダイレクトされる例を明記している。Codex CLI `0.158.0-alpha.2.1`のpinned MxC sourceでは、受信`TEMP`／`TMP`を`:tmpdir` policyへ射影した後、Windows `BaseContainerRunner`で実行する。従って実childの値がBroker指定pathと一致しない観測は、少なくとも「RustからCLIへの設定漏れ」を意味せず、AppContainerによるOS側redirectと整合する。
- Microsoft MxCの同じpinned sourceではBaseContainerが対応hostでPSEC経路を優先し、子process終了後に`ProcessSecurityEnvironment` handleをcloseする。これはserver-side security-environment stateの終了を示すが、profile配下の実TEMP dataの物理削除や、task間・同時task間の一時領域分離を証明しない。hostから終了後にpathが見えない既存probeも同じく削除証拠へ昇格しない。
- 設計上、Broker-owned `WorkspaceTaskScratch`とMxC／Windowsが提供するAppContainer一時領域を別責任にした。tool childの`TEMP`／`TMP`とBroker scratchの完全一致は要求せず、sandbox内実効分離、task間非共有、通常終了／取消／期限超過／crash時cleanupを実証対象とする。これは検証条件の正確化であり、実Agent Taskを起動可能にする変更ではない。

### 正確な検証と証拠境界

- `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml Rust生成Task設定で実Windows隔離の登録secretを拒否する -- --ignored --nocapture --test-threads=1`: exit 0、1 passed／0 failed。インストール済みCodex CLI `0.158.0-alpha.2.1`と合成Workspace／secretを使う直接MxC sandbox probeであり、実Agent、Rust Broker、Owner Approval、production Task、cleanup lifecycleの試験ではない。
- 外部資料の確認（`EXTERNAL_EVIDENCE`）: [Microsoft Learn AppContainer資料](https://learn.microsoft.com/en-us/windows/win32/secauthz/implementing-an-appcontainer)、[Codex CLIの依存版固定箇所](https://github.com/openai/codex/blob/rust-v0.158.0-alpha.2.1/codex-rs/Cargo.toml)、[Codex CLIのMxC権限変換処理](https://github.com/openai/codex/blob/rust-v0.158.0-alpha.2.1/codex-rs/mxc-sandbox/src/policy.rs)、[Codex CLIのWindows起動処理](https://github.com/openai/codex/blob/rust-v0.158.0-alpha.2.1/codex-rs/mxc-sandbox/src/windows.rs)、[Microsoft MxC BaseContainerの起動終了処理](https://github.com/microsoft/mxc/blob/6cd3d58f05d3447e67109cfb75e042803b843ca4/src/backends/appcontainer/common/src/base_container_runner.rs)、[PSEC環境handleの寿命処理](https://github.com/microsoft/mxc/blob/6cd3d58f05d3447e67109cfb75e042803b843ca4/src/backends/learning_mode/windows/src/secenv.rs)。source確認は実行環境の`LIVE_RUNTIME`証拠ではない。
- production Broker／IPC経由の実Task、AppContainer tempの実体・task間分離・物理cleanup、取消／期限／crash後Recovery、installed product、Audit、result／diffのContent Exposureは未成立。TEMP pathの完全一致をblocker条件から外したが、AppContainer一時領域の寿命とRecovery検証は`release_blocker`として残し、`task_execution=unsupported`および`release_ready=false`を維持する。

## R2追補 Windows Job Objectによる子孫process停止試験（2026-10-01）

### 成立した確認

- `native/process_supervision/src/windows_job.rs`のWindows試験fixtureを、Job Objectへ割り当てた子processがさらに永続孫processを起動する構造へ拡張した。Broker相当ownerの強制終了試験は子・孫双方のprocess IDを記録し、Job handle close後に両方の終了をWindows process handleで待ち合わせる。
- 通常取消経路に対応する新しい試験では、実Windows Job Objectの`terminate_and_wait`を実行し、Job内process数が0になることと、子・孫の終了をそれぞれ確認する。合成test executableだけを用い、外部network、資格情報、実Workspace内容には触れない。
- 証拠はWindows上のprocess／Job Object操作に対する`LIVE_RUNTIME`と、合成child fixtureの`FIXTURE`を組み合わせたもの。Brokerのprocess supervision primitiveで子孫process停止を直接確認するが、実Codex／MxC Task経路へ接続した証拠ではない。

### 正確な検証と残存範囲

- `cargo +1.95.0 test --locked --manifest-path native/process_supervision/Cargo.toml -- --test-threads=1`: exit 0、4 passed／0 failed。通常停止、Broker相当owner終了後の子・孫process停止を確認した。
- `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`: exit 0、11 test target、438 passed／0 failed／5 ignored。process supervision変更を含むRust helper全targetが成功した。
- 初回の統合validatorは`.gitignore`変更後のmanifest hash不一致を検出したため、manifestを再生成して再実行した。最終の`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`はexit 0。strict日本語監査1126 files／0 findings、Schema 151／151・negative 194、Conformance 229、登録済み10 checkがすべてpassした。development evidenceのrelease blocker 5件と`release_ready=false`は維持する。
- この試験はJob Objectのprocess群停止だけを示す。production Broker IPC／Desktop native Owner確認、durable Audit、実Codex tool childの隔離、deadline／crash後のRecovery、AppContainer TEMPの分離・cleanupは未成立のままである。`task_execution=unsupported`、関連`release_blocker`、`release_ready=false`を維持する。

## R2追補 同時MxC child間TEMP目印の相互可視性（2026-10-01）

### 成立した確認

- 既存のWindows ignored probeを拡張し、Rustが生成したTask permission profile／launcher TEMP・TMPを使って、Codex CLI `0.158.0-alpha.2.1`の実MxC sandbox childを2つ同時起動した。各childは独立した合成Workspace、scratch、`CODEX_HOME`を使用し、host側barrierにより両方の検査が終わるまで双方を稼働させた。
- 各childは自身のMxC `TEMP`へ一意な合成目印を書いて読み戻せた。相手childのTEMPに対する読み取りは、両方向とも`System.IO.FileNotFoundException`／HRESULT `-2147024894`となった。TEMPの実pathとmarker本文は保存・出力していない。隣接Workspaceを使う既存controlでは相互read／writeがAccess Denied（HRESULT `-2147024891`）、各自Workspace writeは成功した。
- 子process診断出力に不正UTF-8が含まれた試行を一度観測したため、合成データだけを表示する当該ignored testのpipe診断をlossy UTF-8 decodeへ変更した。これはCLI本体やproduction経路の修正ではない。
- 初回の相手TEMP読取はPowerShell `MethodInvocationException`の外側HRESULTを観測したため、inner exceptionの型とHRESULTを判定するようprobeを補正した。最終試行では双方で`System.IO.FileNotFoundException`を確認した。
- Codex CLI childとWindows MxC sandboxの動作は当該runの`LIVE_RUNTIME`、Workspace／TEMP目印は`FIXTURE`証拠である。実model・credential・network接続、永続Codex設定、Windows保護設定の変更はない。

### 検証・失敗履歴

- `cargo +1.95.0 check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`は成功（終了値0）。
- `GUI_SHELL_CODEX_SANDBOX_TEST_EXE`にOwner指定Codex CLI実行fileを設定した後のfocused ignored test `Rust生成Task設定で実Windows隔離の登録secretを拒否する -- --ignored --nocapture --test-threads=1`: exit 0、1 passed。相手TEMP両方向の例外型・HRESULTもassertした。
- `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`: 初回はlocalhost TLS更新試験2件がWindows `ConnectionReset`／10054で失敗。個別再実行は双方passし、同じ全target commandの再実行は12 targets、438 passed／0 failed／5 ignoredでexit 0。初回失敗を成功履歴で置き換えず残す。
- Windows Actionsはこのblockでは未使用。local Windowsでfocused LIVE_RUNTIME probeおよびRust全target検査を実行した。Application Control、registry、ACLを変更していない。
- 初回の統合validatorは日本語厳格監査が新規診断文字列3件と進捗行1件を未局所化として指摘し、残る9検査はpassした。PowerShell block comment内の日本語記述と進捗文を補い、再度すべての開発検査を実行した。
- 最終`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`はexit 0。厳格日本語監査1126 files／0 findings、Schema 151件・正常example 151件・negative fixture 194件、Conformance 230 checks、登録済み10検査すべてpass。release blockerと`release_ready=false`は維持する。

### 証拠境界と未解決範囲

- 相手TEMP markerが見えなかった限定結果は、TEMP rootの物理的一意性を示さない。異なる`CODEX_HOME`を使う2つの直接sandbox childに限られ、同じ`CODEX_HOME`のAgent、順次Task、再起動後のmarker残存、AppContainer TEMPの物理cleanupは未検証である。
- production Broker／IPC、native Owner Approval、durable Audit、実Agent Task、cancel／deadline／crash時Recovery、installed product、result／diff Content Exposureは通していない。従ってcross-agent contamination全体とMxC TEMP cleanupは`release_blocker`のままで、`task_execution=unsupported`および`release_ready=false`を維持する。

## R2追補 同一CODEX_HOMEでの同時・後続MxC TEMP目印照合（2026-10-01）

### 成立した確認

- 基準commit `b3fb1cb605f9c074dc9cfff34eb22a2d099026a0`からignored Windows Rust probeを拡張した。異なる合成Workspace／scratchを持つ2つの同時Codex CLI `0.158.0-alpha.2.1`／MxC sandbox childへ、同一の合成`CODEX_HOME`とRust生成Task permission profileを与えた。同時child間のTEMP目印相互読取は合計7回すべて`System.IO.FileNotFoundException`／HRESULT `-2147024894`だった。
- 両child終了後、同じ`CODEX_HOME`で第三のsandbox childを順次起動し、自身のTEMPから先行childの目印名を照合した。後続childは両目印とも3回中3回`System.IO.FileNotFoundException`／HRESULT `-2147024894`と報告した。TEMP pathと目印本文はprobe結果へ保存・出力していない。
- これらは指定した実Codex CLI／MxC child操作の`LIVE_RUNTIME`と、合成Workspace／scratch／markerの`FIXTURE`証拠である。実model・credential・API networkを使わず、永続Codex設定やWindows保護設定も変更していない。

### 検証・失敗履歴

- 最初の後続child probeはPowerShell script内のreport pathにquoteを二重付与しParserErrorとなった。`quote_path`が返すquoted literalをそのままplaceholder位置へ挿入するよう修正し、その後のfocused probeは3回中3回成功した。ParserErrorの試行は成功回数に含めない。
- 既存の同一home・同時child focused probeは4回成功済み。後続child検査を含むfocused ignored testの正確なcommandは`$env:GUI_SHELL_CODEX_SANDBOX_TEST_EXE='C:\Users\ohira\AppData\Local\OpenAI\Codex\bin\faa963e871dd422c\codex.exe'; cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --lib 'Rust生成Task設定で実Windows隔離の登録secretを拒否する' -- --ignored --nocapture --test-threads=1`で、exit 0、1 passed／0 failed。後続childを含む実行と2回の再実行の計3回で両目印の非可視を確認した。
- `rustfmt +1.95.0 --edition 2021 --config skip_children=true native/rust_helper/src/adapters/codex_cli.rs`、`cargo +1.95.0 check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`は成功。
- `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`はexit 0、12 targets、438 passed／0 failed／5 ignored。Windows Actionsは未使用。Application Control、registry、ACLを変更していない。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`はexit 0。日本語strict監査1126 files／0 findings、Schema 151件／正常example 151件／negative fixture 194件、Conformance 230 checks、登録済み10検査がすべてpassした。release blocker 5件、`task_execution=unsupported`、`release_ready=false`を維持する。

### 証拠境界と未解決範囲

- 同一`CODEX_HOME`条件の相互・後続child非可視性を示すが、実TEMP rootの一意性、child終了時の物理削除、同じ実Agent profileでの隔離、再起動後のmarker残存、MxC内部temporary dataのcleanupを示さない。後続child自身のTEMPから先行markerを読めなかったことは削除証拠ではない。
- production Broker／IPC、native Owner Approval、durable Audit／Recovery、実Agent Task、cancel／deadline／crash時のcleanup、installed product、result／diff Content Exposureは通していない。cross-agent contamination全体とMxC TEMP cleanupは`release_blocker`のまま、`task_execution=unsupported`および`release_ready=false`を維持する。

## R2追補 Windows Export credential scan診断と実Codex CLI再試験（2026-10-01）

### Windows Export手動検証

- [手動Windows Export run #6](https://github.com/gatchimuchio/GUI-Shell/actions/runs/36784185786)は`workflow_dispatch`でclean `main`のcommit `1468bbaec77991bd6bf82e12bee9bf32279d8851`を検査した。checkout SHA／`origin/main`一致、clean source、固定Rust 1.95.0／Flutter 3.44.0 toolchain準備を確認し、Flutter Windows ReleaseとRust helper／launcher Release buildまで到達した。
- その後Export bundleの既知Credential走査が`broker/gui_shell_rust_helper.exe`に`credential_assignment`を検出して停止した。検出一致が本物のcredentialかbinary内の別文字列との誤一致かは、このrunの診断では特定できない。fail-closed判定を維持し、bundle成功へ読み替えない。artifact uploadなし。前段失敗によりsource clean終了stepはskipされたが、hosted runnerは終了済みで一時workspace／build出力は保持されていない。
- `tooling/export_windows_product.py`のCredential判定自体は変更せず、次回調査に限って一致field、artifact内byte offset、値長を診断し、候補値本文は出力しないようにした。conformance negative testはsynthetic候補値がdiagnosticへ漏れないことも検査する。原因が確定するまではscanを緩めない。
- `python -X utf8 -m py_compile tooling/export_windows_product.py tooling/conformance_tests/run_conformance_skeleton.py`は成功。`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`は230 checks合格。`python -X utf8 tooling/manifest.py --write`でtracked manifest 1123件を更新後、履歴追記を含む最終`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`もexit 0。strict日本語監査1126 files／0 findings、Schema 151／example 151／negative 194、Conformance 230、manifest、release gate、packaging portability、release smoke、evidence bundle、release runtime assertions、C32監査の全登録checkがpassした。product Windows installed evidenceの5 gateは未成立、release blocker 31件、`post_v1_scope` 2件、`release_ready=false`を維持する。`git diff --check`もpass。

### 実Codex CLI Broker Taskの再現性追試

- Windows localの明示Codex CLI `0.158.0-alpha.2.1`、隔離CODEX_HOME、localhost偽Responses APIだけを使ったignored LIVE_RUNTIME testを、同じHEAD `1468bba`でこの継続中に3回実行した結果は2 passed／1 failed。失敗runではBroker Taskが`failed`となり取消段階へ届かず、CLI eventは`stream disconnected before completion`。server側のHTTP request parse／response write failureは0、Workspace内markerは作成済み、Workspace外writeなし、loopback proxyは`chatgpt.com` CONNECT 1件を遮断した。偽API server write成功記録はCLIがbodyを受領した証拠ではないため、原因は未確定とする。
- 正確な実行は`$env:GUI_SHELL_CODEX_TASK_BROKER_TEST_EXE = (Get-Command codex.exe -CommandType Application | Select-Object -First 1 -ExpandProperty Source); cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --lib broker::dialogue::tests::Broker承認経路から実CodexCLIをloopback偽APIで実行し隔離とcleanupを確認する_LIVE_RUNTIME -- --ignored --exact --nocapture --test-threads=1`。実Codex CLI `0.158.0-alpha.2.1`を明示した絶対pathで指定し、実model・資格情報は使っていない。
- これはRust test-thread内Broker／synthetic Owner・Audit fixtureと実Codex CLI／MxC childに限る。3回中の失敗を通過runで相殺せず、production Adapterは`task_execution=unsupported`、Agent Task production隔離とCancellationの`release_blocker`、`release_ready=false`を維持する。

### Windows Temp残存cleanup

- 2026-09-29作成の専用fixture Temp directory 2件は、通常directory配下にfixture source／fake CLI executable／debug symbolだけがあり、reparse pointと実行中のfixture processはなかった。
- 正確な2対象へ限定したrecursive `Remove-Item` commandはshell tool policyによりprocess起動前に拒否された。何も削除されていない。policyを迂回する別経路は試さず、cleanup未完了として残す。

## R2追補 Windows Export credential scan再診断とマーカー保持形式変更（2026-10-01）

- [手動Windows Export run #7](https://github.com/gatchimuchio/GUI-Shell/actions/runs/36786755352)は`workflow_dispatch`でclean `main`のcommit `d2107c25fb8cbc2815534677d38723b373bf5ca6`をWindows Server 2025上で検査した。固定toolchain、Flutter Windows Release、Rust helper／launcher Release build、および開始時source cleanは成功した。
- Exportのfail-closed既知Credential走査は`broker/gui_shell_rust_helper.exe`に`credential_assignment`を検出した。診断は`field=api_key`、artifact内byte offset `42737273`、一致値長`25`だけを記録し、候補値本文は出力していない。bundleは不成立、artifact uploadなし。scan後段のsource clean確認は前段失敗でskipされた。
- [手動Windows Export run #8](https://github.com/gatchimuchio/GUI-Shell/actions/runs/36789422406)は`workflow_dispatch`でclean `main`のcommit `f5bbd67e279fecb473e8cb3b261f8b6370d70741`をWindows Server 2025 image `20260922.246.2`上で検査した。固定Rust／Cargo 1.95.0とFlutter commit `559ffa3f75e7402d65a8def9c28389a9b2e6fe42`の準備、checkout SHA／remote main一致、Flutter／Rust Windows Release build、未署名fixture bundle組立、credential scan、最終source clean確認がすべて成功した。
- run #8のfixture bundleは61,974,738 bytes、tree SHA-256 `dfa1233410ad75b3dd65ddb66b7d7bd3788e93c1a14aba03af41b03b7d4c7e69`。workflowは`passed_known_patterns`かつ0 findingsを検査して成功し、artifactはuploadされていない。既知patternで今回の停止は解消したが、run #7の一致byte列を取得していないため、数値marker変更との因果を確定したりrun #7をfalse positiveと断定したりはしない。
- この一致が実Credentialかbinary内byte列による誤検出かは未確定であり、run #7をfalse positiveと断定しない。commit `6f0681c`で追加された既知marker片の文字列配置が関連する可能性を調べ、検出field・offset・長さとmarker構成を照合したが、hosted executableの一致byte列そのものは取得していない。
- `native/rust_helper/src/broker/mod.rs`では14個の既知marker片を連続ASCII文字列から数値`u32`配列へ移し、同じASCII byte列との一致判定を維持した。これは実行fileに検査対象markerそのものを連続埋め込みしないための変更であり、credential scanの拒否基準を緩和する変更ではない。run #8は変更後のRelease bundleでscanが0件になることを確認した。
- Rust全target試験の初回は、既存`failed_replacement_keeps_the_existing_corrupt_package_unchanged`がlocalhost HTTPS試験中に接続resetで失敗し、391 passed／1 failed／5 ignoredとなった。同testの単独再実行は1 passed／0 failed、その後の全再実行は12 target、438 passed／0 failed／5 ignoredで完了した。初回失敗を成功へ読み替えず、再現しなかったnetwork試験失敗として履歴に保持する。
- `cargo +1.95.0 check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`、`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`（230 checks）、局所`rustfmt` check、`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`がすべて合格した。strict日本語監査1126 files／0 findings、Schema 151／example 151／negative 194、release blocker 31、`post_v1_scope` 2、`release_ready=false`を維持した。
- run #8はfixtureのReceipt／Manifestだけを使い、Broker生成・Owner authority・runtime Manifest消費・署名・installer・installed起動を検証していない。release blockerは解除せず、次工程はBroker生成Receipt／Manifestによるunsigned test bundleとOwner No／Yes、隔離profile、Audit／identity分離の実証である。

## R2追補 Windows起動器正本のExport build接続記述を同期（2026-10-01）

- 現行`docs/specs/gui-shell-export.md`、`docs/specs/gui-shell-module-pruning.md`および`tooling/export_windows_product.py`は、Schema検証済みManifestのApp ID／Audit store IDをRust compile-timeへ渡すDeveloper専用Windows bundle buildを定義・実装している。一方、`docs/specs/windows-desktop-launcher.md`にはbuild tool未接続・将来実装とする旧記述が残り、同じ正本索引内で矛盾していた。
- 起動器正本を実装とExport意味正本へ同期し、現在のDeveloper build補助はcompile-time identityとidentity別Cargo targetを使う一方、runtime Manifest消費・Owner承認済みproduction Export・installed productを成立させない境界を明記した。
- これは文書整合性の修正であり、Export Owner path、正式製品build、installed runtime、署名・Installerのrelease blocker状態を変更しない。
- 記述変更直後の`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`はConformanceとpackaging portabilityが失敗した。`run_conformance_skeleton.py`の起動器境界検査が未接続／将来実装という旧文言を要求していた。packaging portabilityのnested Conformance実行も同じ旧assertionを検出し、`fatal: not a git repository`診断を含め失敗した。
- Conformanceの境界検査を弱めず、現在のDeveloper compile-time接続と、runtime Manifest消費／Owner承認済みproduction Exportが未成立であることを要求するassertionへ改めた。`python -X utf8 -m py_compile tooling/conformance_tests/run_conformance_skeleton.py`、`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`（230 checks）、最終の同`validate_all.py`はexit 0となり、packaging portabilityも合格した。release blocker 31件、`post_v1_scope` 2件、`release_ready=false`に変更はない。

## R2追補 Exportの通常Owner資格を拒否しネイティブ確認境界へ固定（2026-10-01）

- 最新Rust経路を追跡した結果、`owner要求処理`からのOwner資格付き要求が`OwnerConfirmationSource::OwnerCredential`として`GUI Shell書出し`へ渡り、Broker protocolにnative確認必須gateがないため、通常Owner資格だけでExport Manifestを作成し得ることを確認した。これは`docs/specs/gui-shell-export.md`のRust Desktop起動器native確認必須契約と不整合だった。
- `native/rust_helper/src/broker/protocol.rs`へ中央dispatch前のExport gateを追加した。非Ownerは従来どおり`owner_required`、Owner資格だけの要求は`desktop_native_owner_confirmation_required`で拒否し、`DesktopNativeConfirmation`経路だけが既存Export handlerへ到達する。拒否はManifest file作成前に記録される。
- Rust Export testを成功ケースではnative Owner operation helper経由に揃え、通常Owner資格だけの要求が拒否Auditを残し、Export directoryを空のままにするnegative testを追加した。ConformanceにもgateのExport dispatch前配置とnegative testの存在を検査するassertionを追加した。
- 初回全Rust runはlib 398件中390 passed／3 failed／5 ignoredとなった。失敗のうち1件は新gateが非Ownerの既存`owner_required` error codeを変えた回帰であり、Owner=trueの場合だけnative-confirmation gateを適用するよう修正した。残るA2A接続とlocalhost TLS server testは単独再試験でA2A pass、TLSは1回失敗後の再実行でpassとなった。途中の失敗履歴は維持する。
- 修正後の`cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`は12 targetで439 passed／0 failed／5 ignored。`cargo +1.95.0 check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`、Export module 13件、Owner allowlist回帰test、`python -X utf8 -m py_compile tooling/conformance_tests/run_conformance_skeleton.py`、Conformance 230 checksもpassした。`rustfmt`の`protocol.rs`単独checkはpass。`protocol.rs`と`export_center.rs`全体を対象にしたcheckは、変更行外にある既存`export_center.rs`の書式差分で失敗したため、無関係な全体整形は行っていない。
- 現在のtestはBroker内のsynthetic Owner/native confirmation経路を検証するもので、実Win32 dialogのNo／Yes操作、Broker生成Receipt／ManifestからのWindows unsigned bundle、別profile installed runtime、Audit store分離を証明しない。`rev2_export_owner_ui_authority_path`はunresolvedのままであり、これらのLIVE_RUNTIME証拠を得るまで解除しない。`release_ready=false`を維持する。

## R2追補 Windows Export Owner確認dialogのNo／Yes UI試験（2026-10-01）

- `native/rust_helper/src/desktop_launcher.rs`の既存対話型Windows ignored test harnessへ、`GuiShellExport`確認summaryの子process直列化・復元を追加した。合成Export summaryを渡し、実Win32 Owner確認dialogを「いいえ」「はい」で自動操作する専用ignored testを追加した。private payload markerがdialog表示へ漏れないことも同じ試験で確認する。
- この端末で実行した正確なコマンド `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --lib desktop_launcher::tests::nativeOwner確認dialogのExportNoYesを制御UI自動化できpayload本文を露出しない -- --ignored --exact --test-threads=1 --nocapture` は終了値0、1件成功。既存のAgent Task向けOwner確認画面の試験 `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --lib desktop_launcher::tests::nativeOwner確認dialogのYesNoを制御UI自動化できTask本文を露出しない -- --ignored --exact --test-threads=1 --nocapture` も終了値0、1件成功。
- このUI試験はlocal Windows上のWin32 dialog操作について`LIVE_RUNTIME`証拠であり、dialogへ渡したsummary／payload markerは合成`FIXTURE`である。対話はtest用child process内で行われ、本試験はBrokerへExport requestを送らず、Receipt／Manifest／Audit fileを生成しない。製品のExport処理全体やinstalled productの証拠ではない。
- `tooling/conformance_tests/run_conformance_skeleton.py`へ、Export No／Yes UI試験と子process discriminatorの存在を確認するassertionを追加した。`python -X utf8 -m py_compile tooling/conformance_tests/run_conformance_skeleton.py`と`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`（230 checks）はpass。`rustfmt +1.95.0 --edition 2021 --config skip_children=true --check native/rust_helper/src/desktop_launcher.rs`もpassした。
- `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`はexit 0。lib 393 passed／0 failed／6 ignored、main 10、Broker IPC 10、その他integration targets 36、合計439 passed／0 failed／6 ignored。Windows Actionsは未使用であり、interactive Win32 dialogはlocal Windowsで直接検査した。
- `rev2_export_owner_ui_authority_path`は引き続きunresolvedである。Brokerが生成したReceipt／Manifestを使うExport・unsigned bundle化、別profileでのinstalled product起動、独立Identity／Audit store、Owner判断とBroker結果の一連のproduction pathは未証明であり、release blockerを解除しない。`release_ready=false`を維持する。

## R2追補 Export Owner確認からBroker生成Receipt／ManifestのWindows Bundle検証（2026-10-01）

### 成立した確認

- Export用ignored Windows testを、実Win32 Owner確認dialogのNo／Yes操作からBroker処理まで接続した。Noは`owner_required`で拒否されManifestを作らず、YesはBroker test経路がaccepted ReceiptとManifestを生成する。Receipt／ManifestのApp ID、Audit store ID、hashを照合し、Manifestは`build_status=not_started`／`artifact_status=not_built`である。Compose側synthetic ID markerは確認summaryおよびBroker Auditへ出ない。
- Win32 dialog操作はlocal `LIVE_RUNTIME`、request payload・Workspace・Broker storeはsynthetic `FIXTURE`。Broker生成Receipt／Manifestはtest Broker経路でのfixture-backed結果であり、実production Owner identityやinstalled runtimeの証拠ではない。
- 成功したReceipt／Manifestをclean source commit `3d312b27d2a4b53575de5561790ecc7fc8f2439c`のWindows Export buildへ渡し、14-file unsigned development bundleを生成した。bundleは62,023,973 bytes、tree SHA-256 `d154308a123315a82ed59ebb6d57fb128ff24c2b8052ce65da86f37d2713af36`。Receipt内Manifest SHA-256 `ba49850e15aea12a27f28007d418230c92b6ee897e78117f9743f6c08fdcd5b6`とbundle内`product_manifest.json`のhashは一致し、Receipt raw SHA-256は`32fb15004c2531fd44631af8edca23b17105dade097cb0107d10d0fc3b846ad5`。Build identityはApp ID `d4-pocket-app-aded8a8e3c38d9dcc233bb44786d87b7`、Audit store ID `audit-store-cf50852e45f1fcfe5e04d485d112ab2b`、Export ID `export-desktop-test`。
- Exporterは既知Credential pattern走査`passed_known_patterns`、0 findings。これは走査対象patternに対する結果であり、秘密が一般に存在しない保証ではない。証拠はbundle生成の`INTERNAL_STATE`とsynthetic Broker／Workspaceの`FIXTURE`であり、Owner authorization、source authority、runtime Manifest消費、installed起動、署名、Installer完了はいずれも未確認。artifactはActionsにもRepositoryにもuploadしていない。

### 検証と履歴

- 局所試験は終了値0で1件成功した。実行command: `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --lib desktop_launcher::tests::nativeOwner確認dialogのExportNoYesをBrokerReceiptManifestまで接続する -- --ignored --exact --test-threads=1 --nocapture`。
- `rustfmt +1.95.0 --edition 2021 --config skip_children=true --check native/rust_helper/src/desktop_launcher.rs`および`native/rust_helper/src/broker/update_download.rs`: pass。`cargo +1.95.0 check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`: pass。`cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`: 12 targets、439 passed／0 failed／6 ignored。
- `cargo +1.95.0 fmt --manifest-path native/rust_helper/Cargo.toml --all -- --check`は変更対象外の既存Rust fileの書式差分でfail。無関係な全体整形は行わず、対象fileの局所rustfmt checkとActionsの変更Rust file整形checkを使用した。
- `python -X utf8 tooling/schema_check/check_schemas.py`はSchema 151件、正常例151件、否定例194件で成功。`python -X utf8 -m py_compile tooling/conformance_tests/run_conformance_skeleton.py`と`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`（適合検査230件）も成功。
- Workflow編集直後の`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`はMANIFESTのhash不一致で失敗。その後の実行では、この追記に含まれる英語散文4箇所が厳格日本語監査で検出された。該当記述を日本語化した後、同`validate_all.py`は終了値0となった。監査対象1126件・指摘0件、登録10検査すべて成功。内訳はSchema、適合検査、MANIFEST、リリースゲート、配布形式、起動確認、証拠束、runtime表明、C32を含む。Release blocker 31件、`post_v1_scope` 2件、`release_ready=false`は維持。
- [Windows手動検査 第30回](https://github.com/gatchimuchio/GUI-Shell/actions/runs/36798761327)は`workflow_dispatch`、一時branch `codex/export-owner-broker-validation`上のcommit `3d312b27d2a4b53575de5561790ecc7fc8f2439c`を対象として9工程すべて成功、所要10分25秒。Rust整形・全target確認／試験、Broker helperのrelease build、単体Broker実行試験、後片付け、作業tree cleanを確認した。Runnerは`win25-vs2026/20260925.250.1`、Rust `1.95.0`、helper SHA-256 `ca9a0f335380d141ffcf469d9f8e119d882254ce79af09f0c6d37c6403e8dc17`。実測した内容は認証済みloopback IPC、永続store、通常Task grant拒否、再起動後replay拒否、新鮮なhealth、強制停止後fail-closed、session file cleanup。これは単体Brokerと検証runnerの証拠で、Owner Export・installed product・正式release evidenceではなく、artifact uploadもない。
- Windows Export buildは、Receiptが指定するManifest filenameと合わない初回入力をbuild前に拒否した。Temp上で期待filenameへ置いた後の再実行は成功。build中にMSBuild `MSB8029`および変更外Rust dead-code warningが出たが、buildは完了した。Temp生成物の後片付けは環境policyが`Remove-Item -Recurse`を起動前に拒否したため未完了で、削除は一件も行われていない。対象は`D4PocketBrokerExportCapture-9874e540`、`D4PocketBrokerExportCapture-9874e540-retry1`、`D4PocketBrokerExportBuild-3d312b2`、`d4-pocket-app-aded8a8e3c38d9dcc233bb44786d87b7.json`。4対象はTemp直下にあり、再解析pointがなく、参照中processもないことを確認した。
- 手動Actions検査後、成功した同一commitだけを`main`へfast-forwardし、pushとremote HEAD一致を確認した。一時branchはlocal／remote双方で削除し、PR・自動trigger・artifact uploadは作成していない。

### 境界と残存範囲

- 今回成立したのはWin32確認画面、synthetic fixtureを用いたBroker Receipt／Manifest生成、未署名の開発用bundle組立、既知の資格情報pattern検査、単体Broker実行確認まで。`rev2_export_owner_ui_authority_path`は未解決の`release_blocker`として維持する。
- 別Windows user profileからのinstalled app起動、正式Owner authority、実installed Audit storeの分離と完了Audit、非継承Receipt、実製品launch、署名／Installer、release readinessは未成立。現行`docs/specs/windows-desktop-launcher.md`はruntimeでManifestを再読込してpath／authority選択へ使うことを認めず、compile-time identityを使う設計である。従ってruntime Manifest消費は未完了項目や次工程条件にしない。必要な次の証拠はManifest由来compile-time IDを含むBundleの別profile installed-path実行とAudit境界である。`release_ready=false`を維持する。

## R2追補 Release Broker processでのAgent Task未対応gate E2E（2026-10-01）

### 成立した確認

- Rust source／testがcommit `8ec3434b3a713a00654b97c14d19e21cb31e69a7`と一致する状態で、Windows Release helperを別process起動し、実Codex CLI `0.159.2`をruntimeとして登録するignored integration testを実行した。実行時点では進捗・仕様・Blocker registry・Manifestだけに未commit差分があり、Rust source差分はなかった。通常loopback資格とOwner資格fileは別々に生成され、通常認証IPCでAgent一覧とBroker登録Workspaceへ結合したAgent Sessionを取得できる。
- Agent metadataの`task_execution=unsupported`を実Broker projectionで確認し、`Agent作業要求検査`と`AgentTask実行`を同じSession／Workspace／合成指示本文で送ったところ、両方が`AgentTask実行非対応`として拒否された。指示本文は応答へ返らない。
- Workspace PermissionとOwner Approvalの発行要求は通常資格で通常endpointへ送り、両方が`desktop_native_owner_confirmation_required`として拒否された。BrokerはTask本文を返さず、既存のnative Owner確認endpointを迂回する動作はなかった。
- Brokerを正常終了した後、永続Audit storeを再openし、Session・両grant拒否・Task検査／実行拒否のAudit記録を照合した。指示本文と合成secret markerはAuditに含まれない。test専用一時rootは終了後に削除される。
- 証拠classはRelease Broker process／認証loopback IPC／実CLI登録と拒否Auditの`LIVE_RUNTIME`、合成Workspace・secret・storeの`FIXTURE`。実model、API資格情報、Windows保護設定変更は使用していない。実helper SHA-256は`C816AFA894A45613AA5683A87472DA3E5815D29F640FFCF507C9D7E5B49DC73B`。

### 正確な検証と履歴

- Release helper build: `cargo +1.95.0 build --locked --manifest-path native/rust_helper/Cargo.toml --release --bin gui_shell_rust_helper` — 成功。既存`native/rust_helper/src/adapters/minidora.rs`の未使用項目warningが2件。
- E2E: `GUI_SHELL_AGENT_TASK_E2E_HELPER_EXE`へ上記Release helperの絶対path、`GUI_SHELL_CODEX_TASK_E2E_CLI`へ`Get-Command codex`の絶対pathを設定し、`cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --test agent_task_production_e2e 'windows::production_agent_task_gate_fails_closed_over_authenticated_ipc' -- --ignored --exact --nocapture` — HEAD `8ec3434`、Rust source差分なし（文書／Manifest差分あり）の作業treeで1 passed／0 failed。
- Rust検証: `cargo +1.95.0 check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`成功。`cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`の再実行は13 target、439 passed／0 failed／7 ignored。初回全target実行では既存localhost TLS試験`local_tls_server_repairs_only_after_verified_package_bytes`がConnectionResetで1回失敗したが、単独再実行1件成功後の全serial再実行は成功した。初回失敗は消去せず履歴へ残す。
- `python -X utf8 tooling/schema_check/check_schemas.py`: Schema 151件、正常例151件、negative fixture 194件で成功。`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`: 230 checks成功。追加Rust testの`rustfmt +1.95.0 --edition 2021 --check native/rust_helper/tests/agent_task_production_e2e.rs`と`git diff --check`も成功。
- 初回の統合`validate_all.py`は追加testの英語failure message 2箇所を厳格日本語監査が検出して失敗したため、該当messageを日本語化した。修正後の厳格監査は1127 files／0 findings、`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`は終了値0となり、登録済みdevelopment check 10件すべて成功した。Manifestは1124 files。release blocker 31件と`release_ready=false`を維持する。
- commit `8ec3434b3a713a00654b97c14d19e21cb31e69a7`は`main`へpushされ、remote HEAD一致を確認した。commit時点のworking treeにはこの進捗・仕様・registry／Manifest更新が未commitで残っていたためcleanではない。Rust source差分はなかった。backup: `codex/backup-main`=`8ec3434b3a713a00654b97c14d19e21cb31e69a7`、`codex/backup-main-prev`=`7117aaf58678b44ebc34fd01a751252299c396ea`。

### 証拠境界と未解決範囲

- 本testは未対応gateの本番Release Broker拒否経路を検証する負のE2Eであり、Agent Taskを起動しない。Owner資格endpointおよびWin32確認画面は使わず、実Task process、Workspace変更、Task scratch作成・削除、sandbox拒否、取消／期限／crash、result／diff UIを検証していない。
- `release_blocker`: clean installed Desktop UIからnative Owner判断をBrokerへ結合した正のTask実行、実tool-child隔離、OneDrive Cloud Files／通常NTFSのsecret・深度・alias検査、Task間分離、Audit tamper検証、停止とRecovery、Broker scratch／AppContainer一時領域の後始末、result／diffのContent Exposure。
- `task_execution=unsupported`と`release_ready=false`を維持する。新しい拒否probeを正のproduction Task完了またはrelease gate解除へ昇格しない。

## R2追補 実Codex CLI登録からnative Owner確認・Task拒否までのE2E補強（2026-10-01）

### 成立した範囲

- Agent CenterのCodex CLI登録UIからBroker要求へ渡すfieldをWidget testで固定した。runtime／adapter／Workspace／secret pathだけを送信し、Permission、Approval ID、Credential実値は送らない。表示されるTask能力は`unsupported`のままである。
- 実Windows上のignored Rust testでは、実Codex CLIをBroker登録要求へ接続し、実Win32 Owner確認dialogのNo／Yesをそれぞれ操作した。Noの後はAgent未登録、Yesの後はBroker Agent一覧に登録されたことを確認した。確認summaryと監査へsecret本文が出ず、登録応答はPermission／Approval／Credentialを生成しない。
- 同じtest内で、通常Desktop relayからのWorkspace Permission／Owner Approval要求、Task preflight、Task startを送信した。Taskはworker起動前に`AgentTask実行非対応`で拒否され、応答本文・監査にTask指示markerまたはsecret markerがない。終了後にfile-backed Auditを照合し、成功した登録と拒否結果だけが残ることを確認した。
- 別のignored integration testでは、現在sourceからbuildしたWindows Release Broker helperを別process起動し、実Codex CLIの起動時登録を含む認証loopback IPCでTask能力の`unsupported`、通常資格によるPermission／Approval発行拒否、Task preflight／start拒否、Broker終了後のAudit再openを確認した。これはOwner dialog経路とは別probeであり、二つの試験を単一installed-product E2Eとは扱わない。
- 証拠class: 実CLI probeと実Win32 dialogの`LIVE_RUNTIME`、test thread Broker・Release Broker用合成Workspace／store・自動入力の`FIXTURE`。実model、実Credential、課金要求、Windows保護設定変更は使っていない。

### 検証結果と境界

- focused native Owner／登録test `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --lib 'desktop_launcher::tests::登録CodexへのnativeOwner確認後もAgentTask非対応gateを維持する' -- --ignored --exact --nocapture --test-threads=1`はexit 0、1 passed。Widget testのfixtureは画面初期化時のWorkspace一覧応答を補い、登録要求への応答とstatusもassertする。短い一時pathでの`flutter analyze --no-pub`と`flutter test --no-pub --plain-name 'Codex登録UIは指定値だけをnative Owner確認Broker要求へ渡す'`はpassした。
- Rust `cargo check --all-targets`、serial `cargo test --all-targets`（13 targets、444 passed／0 failed／7 ignored）、Schema（152／正常example 152／negative fixture 195）、Conformance（231 checks）はpassした。
- 初回`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`は、新規進捗行1件が厳格日本語監査へ検出されexit 1。該当表現を日本語化し、`python -X utf8 tooling/日本語基底監査.py --strict`を再実行して1130 files／0 findingsでpassした。初回失敗を成功へ読み替えない。
- `cargo +1.95.0 build --locked --manifest-path native/rust_helper/Cargo.toml --release --bin gui_shell_rust_helper --target-dir C:\\d4-r2-target`による現行Release helperのbuildは成功した。既存MINIDORAの未使用項目に関するwarningは2件。Release helper SHA-256 `56bc5d14b4bd94361802f01d9a908b311d7d06f3119f289f587ddffd5f92710d`。実Codex CLIは`codex-cli 0.159.3`。`GUI_SHELL_AGENT_TASK_E2E_HELPER_EXE`と`GUI_SHELL_CODEX_TASK_E2E_CLI`へそれぞれ当該Release helper／CLIの絶対pathを設定した`cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --test agent_task_production_e2e 'windows::production_agent_task_gate_fails_closed_over_authenticated_ipc' -- --ignored --exact --nocapture --test-threads=1`はexit 0、1 passed。
- OneDrive checkout上のFlutter SDKはephemeral directoryを削除できず、local Desktop analyzeとfocused Widget testだけを短い一時pathで実行した。ACLは変更していない。このhost制約の補助として、対象source commit `fc8144c743c980b004637952f31734e4ca9910da`を一時branch `codex/r2-agent-registration-e2e`へ固定し、Windows hosted Actionsを`workflow_dispatch`で実行した。Rust manual validation [run #32](https://github.com/gatchimuchio/GUI-Shell/actions/runs/36835238543)とDesktop Flutter manual validation [run #10](https://github.com/gatchimuchio/GUI-Shell/actions/runs/36835318511)は、両方とも同一commit checkout、全step成功、試験後source clean確認まで成立した。
- Rustの手動Windows検証run #32は`windows-2025`、Rust／Cargo 1.95.0で、変更Rustファイルの`rustfmt`整形検査、`cargo check --all-targets`、直列実行の`cargo test --all-targets`、Release Broker helperのrelease版ビルド、通常認証loopback IPCによる独立Broker動作確認を実行した。全対象試験は6分32秒、release版ビルドは8分45秒、Broker動作確認は7秒、runner後片付けは2秒、job全体は16分04秒。Broker動作確認の観測範囲は通常IPC、通常資格によるTask grant拒否、再起動後のreplay拒否、新規稼働状態の確認、強制停止時のfail-closed、session fileの削除であり、Owner確認画面やAgent Task実行を含まない。
- Desktop／Mobile向け手動Windows検証run #10は固定Flutter 3.44.0でRust helperのビルド、Desktopの`flutter analyze --no-pub`と全テスト、Mobileの`flutter analyze --no-pub`を実行し、全て成功した。job全体は8分03秒で、生成物のuploadはなく、検査後にsource treeがcleanであることも確認した。この検査結果は自動検証の必須条件、installed product起動、Owner UI／named pipe結合、正式公開の準備完了を証明しない。
- 検証成功後、対象commitだけを`main`へfast-forwardし、local／remote HEAD一致を確認して一時branchをlocal／remote双方から削除した。Actions実行とbranch cleanupの詳細を`release_blockers.registry.json`にも記録した。
- 手動検証の結果追記後の初回統合検査では、報告文1行の英語混在を日本語厳格監査が検出し、終了値1となった。該当散文を日本語化した後、厳格監査単体は1130ファイル／検出0件で成功した。初回失敗は履歴に保持する。`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`の再実行は終了値0となり、登録済み10検査が全て成功した。内訳は厳格監査1130ファイル／検出0件、Schema 152件／正常例152件／negative fixture 195件、Conformance 231件、Manifest 1127ファイル、release gate、packaging、release smoke、evidence bundle、runtime assertion、最終開発監査の成功である。`release_ready=false`と未解決release blockerは維持する。
- 本E2EでAgent Taskを起動しない。clean installed Flutter／Desktop UIと実native pipe／Brokerを通した一体動作、Task成功、OneDrive Cloud Files／通常NTFSの実隔離matrix、Task間分離、Audit改変試験、failure／deadline／crash Recovery、Broker scratchとMxC TEMP実体のcleanup、result／diff Content Exposureは未成立の`release_blocker`である。`task_execution=unsupported`と`release_ready=false`を保持する。

## R2追補 現行Codex CLIによるBroker Task試験の再現性補強（2026-10-01）

### 追加で成立した範囲

- 実Codex CLI `0.159.2`を資格情報のない隔離`CODEX_HOME`とloopback偽Responses APIへ接続し、Rust生成のTask command／MxC profileを通してBroker dialogue consumerから合成Taskを実行した。synthetic registered secretの読取とWorkspace外の読書込を拒否し、登録Workspace内のmarker書込を確認した。成功Taskの結果投影に指示本文・secret本文・偽API応答本文が含まれないこと、Broker scratchが残らないこともassertした。
- 同じ試験でheartbeatを書き続ける二つ目のTaskを取消し、terminal後にheartbeatが増えないこと、result hashがないこと、scratchが残らないことを確認した。
- 現行CLIで偽API応答の不安定が続いたため、試験専用model識別子とSSE応答をCodex CLIの標準例に沿う最小形へ変更した。変更後は同じfocused testが3回連続で成功した。先行する3回の失敗は履歴に保持し、根本原因が完全に証明されたとは扱わない。
- 証拠classは実CLI／MxC tool childの指定操作に対する`LIVE_RUNTIME`、Broker Owner判断・Adapter能力上書き・Audit callback・Workspace／secretの`FIXTURE`である。実model、資格情報、課金要求、保護設定変更はない。

### 未成立範囲と検証

- 本試験はin-process Broker consumerを通るfixtureであり、production Release Broker／authenticated IPC、installed Flutter／Desktop native Owner確認、durable Audit、OneDrive Cloud Filesと通常NTFS双方、深度・hardlink alias隔離matrix、deadline／crash Recovery、MxC内部TEMPの実体・物理cleanup、result／diff UIを証明しない。製品Adapter metadataは`task_execution=unsupported`のままである。
- 別の直接MxC probe `python -X utf8 tooling/codex_mxc_exec_temp_probe.py --exe <codex.exe絶対path> --runs 1`は現行CLIでexit 1。loopback API応答とtool callは観測したが、shell childが`-2147450747`（`0x80008085`）で失敗してprobe reportが生成されなかった。model識別子を変えた追試でも解消せず、原因未確定として保持する。
- focused ignored test `cargo +1.95.0 test --locked --offline --manifest-path native/rust_helper/Cargo.toml --lib 'broker::dialogue::tests::Broker承認経路から実CodexCLIをloopback偽APIで実行し隔離とcleanupを確認する_LIVE_RUNTIME' -- --ignored --exact --nocapture --test-threads=1`は、fixture変更後3回連続で1 passed。
- `cargo +1.95.0 check --locked --offline --manifest-path native/rust_helper/Cargo.toml --all-targets`は成功。全target serial Rust試験の最終runは13 targets、444 passed／0 failed／7 ignored。先行runではlocal TLS更新fixtureがConnectionResetし、focused再実行も一度`Network`対`DigestMismatch`で失敗した後に1 passedとなった。失敗履歴は保持する。
- Schemaは152件／正常例152件／negative fixture 195件、Conformanceは231 checksで成功。変更2 Rust fileの`rustfmt +1.95.0 --edition 2021 --config skip_children=true --check ...`と`git diff --check`は成功した。workspace全体の`cargo fmt --all -- --check`は既存多数fileのformat差分を検出してexit 1となったため、全体一括formatは行わず変更fileだけ検査した。
- R2のproduction E2E、`task_execution=unsupported`、該当`release_blocker`、`release_ready=false`を維持する。

## R2追補 実Codex Broker fixtureにおけるMxC TEMP正常終了・取消後観測（2026-10-01）

### 成立した観測

- 実Codex CLI `0.159.2`のMxC tool childから、`TEMP`／`TMP`と合成marker名を報告させた。実行中はBroker test親processから当該marker本文を読み取れたが、通常Task完了後は同じmarkerが`not_found`となった。Cancellation TaskでもBroker親から実行中markerを読み取れ、Job Object経由の取消でchild停止を確認した後は同markerが`not_found`となった。絶対pathは保存・表示しない。
- analytics無効化後の7回連続成功runすべてで、`TEMP=TMP`、`TEMP`とBroker `.d4p-tmp-*` scratchは不一致、通常完了／取消の両方でMxC TEMP markerは実行中`readable_match`・child終了後`not_found`、Broker scratchはterminal後に不在だった。
- stream切断と非loopback `chatgpt.com` CONNECTが発生した失敗履歴は保持する。loopback test profileだけへ`analytics.enabled=false`を設定した後は7回連続成功し、各runの遮断対象外接続数は0だった。この設定は`#[cfg(test)]`内だけで、製品設定・ユーザー設定・本番Adapter commandを変更しない。
- 実CLI／MxC tool childのmarker可視性とTask process終端は`LIVE_RUNTIME`。Broker Owner判断、Adapter能力fixture、Audit callback、Workspace、markerは`FIXTURE`。実model、credential、課金要求、Windows保護設定変更はない。

### 検証結果と境界

- `GUI_SHELL_CODEX_TASK_BROKER_TEST_EXE`へ実CLI絶対pathを設定したfocused ignored test `cargo +1.95.0 test --locked --offline --manifest-path native/rust_helper/Cargo.toml --lib 'broker::dialogue::tests::Broker承認経路から実CodexCLIをloopback偽APIで実行し隔離とcleanupを確認する_LIVE_RUNTIME' -- --ignored --exact --nocapture --test-threads=1`は、loopback test profileのanalytics無効化後に7回連続で1 passed。修正前には同じ変更途中のfocused runで2件がstream切断により失敗しており、応答送信・HTTP request parseの失敗は0、`chatgpt.com` CONNECT遮断を各失敗で1件観測した。初回失敗を成功へ読み替えない。
- `cargo +1.95.0 check --locked --offline --manifest-path native/rust_helper/Cargo.toml --all-targets`は成功した。全target試験の初回は既知のlocalhost TLS更新fixture `local_tls_server_repairs_only_after_verified_package_bytes` がWindows error 10054 `ConnectionReset`で1件失敗し、直後の単独再試験1件は成功、serial全target再実行は13 targetsで444 passed／0 failed／7 ignoredだった。初回失敗は保持する。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`は終了値0、登録済みdevelopment check 10件すべて成功。厳格日本語監査1130 file／0 findings、Schema 152／正常例152／negative fixture 195、Conformance 231 checks、Manifest 1127 files、release gate、packaging、smoke、evidence bundle、runtime assertion、最終開発監査が成功した。`release_ready=false`と31件のrelease blockerは維持する。
- `rustfmt +1.95.0 --edition 2021 --config skip_children=true --check`を変更3 Rust fileへ実行し成功した。`git diff --check`も成功した。
- 観測したのは専用synthetic marker fileの正常Task終了後／取消後の消失だけである。MxC TEMP directory全体の全entry、deadline／process crash／電源断後cleanup、production Broker IPC、installed Desktop UIとnative Owner confirmation、耐久Audit／tamper検証、OneDrive Cloud Filesと通常NTFSの隔離matrix、result／diff Content Exposureは未確認。製品Adapter metadataは`task_execution=unsupported`、R2 release blockerと`release_ready=false`を維持する。

## R2追補 Owner Approval固定ポリシーIDの契約整合（2026-10-01）

### 成立した変更

- 現行Brokerとnative Owner確認画面が示す固定ID `gui-shell-agent-task-sandbox-v1-max-runtime-900s` に対し、Owner Approval Schemaと正常例だけが旧ID `gui-shell-agent-task-sandbox-v1` を保持していた不整合を修正した。
- Schema・正常例をBroker／native確認画面と同じ固定IDへ更新し、旧IDのreceiptを拒否するnegative fixtureを追加した。日本語の実行系仕様に900秒の最大実行条件と、ID一致だけでは隔離・Task対応を証明しない境界を記録した。ConformanceはSchema／正常例の値、Broker／native sourceの値、および旧ID rejectionを検査する。
- 変更前の作業基点はcommit `4e8282f610c13d79b01cf7e32717c7a82a389b22`。試験対象はSchema、fixture、仕様、Conformanceの作業tree差分である。

### 検証結果と境界

- `python -X utf8 tooling/schema_check/check_schemas.py`: 成功、Schema 152件／正常例152件／negative fixture 196件。
- `python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`: 成功、231 checks。
- 初回の`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`は、未追跡だった追加negative fixtureがGit追跡fileだけを束ねるpackaging portability検査のsource archiveから欠落し、その展開先Conformanceで`FileNotFoundError`となって失敗した。fixtureをintent-to-addで追跡対象へ含めてManifestを1128 fileで再生成し、`python -X utf8 tooling/packaging_portability_check.py`でsource bundle検査が成功した後、同じ統合validatorを再実行して終了値0を確認した。厳格日本語監査、Schema、Conformance、Manifest、release gate、配布形式、release smoke、証拠束、runtime assertion、最終開発監査の登録10検査がすべて成功し、`release_ready=false`と既存release blockerを維持した。初回失敗は履歴として保持する。
- この修正はreceipt上の固定条件識別子と契約間整合だけを確認する。Task worker起動、Broker production IPC、installed Flutter／native Ownerからの正の実行、filesystem隔離、永続Audit、Recovery、結果／diff表示を検証していない。
- `task_execution=unsupported`、R2 production E2E `release_blocker`、`release_ready=false`を維持する。

## R2追補 MxC TEMP実体の正常終了・取消後照合（2026-10-01）

### 成立した観測

- ignored Rust統合test `Broker承認経路から実CodexCLIをloopback偽APIで実行し隔離とcleanupを確認する_LIVE_RUNTIME`を拡張し、実Codex CLI `0.159.2`のMxC childから報告されたTEMP pathをterminal後に単一directoryとして照合する。正常完了とBroker取消を各1回実行し、実行中は合成markerをhost test processから読め、各terminal後はmarkerとTEMP directory rootの双方が`NotFound`となった。
- 試験出力はmarker状態、TEMP root状態、directory内entry件数だけとし、TEMP path、entry名、内容は出力・保存しない。TEMP／TMPの一致とBroker `.d4p-tmp-*` scratchとの不一致、loopback外接続拒否0件も同runで確認した。
- Codex CLI／MxC child終端後のdirectory不在は当該実機・版における`LIVE_RUNTIME`。Broker consumer、Owner判断、Audit callback、Workspace、CODEX_HOME、fake Responses APIは`FIXTURE`。実model、credential、課金要求、Windows保護設定変更はない。

### 検証と未成立境界

- focused ignored testはexit 0、1 passed。環境変数`GUI_SHELL_CODEX_TASK_BROKER_TEST_EXE`には`C:\Users\ohira\AppData\Local\OpenAI\Codex\bin\c6fe824d725f02d7\codex.exe`（`codex-cli 0.159.2`）を指定した。
- `cargo +1.95.0 test --locked --offline --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`: 13対象を実行し、成功444件／失敗0件／無視7件。
- `rustfmt +1.95.0 --edition 2021 --config skip_children=true --check native/rust_helper/src/broker/dialogue.rs`と`git diff --check`は成功。
- source更新直後の最初の統合validatorはManifest hash不一致で失敗した。文書追補後の再検証では進捗記録1652行目の英語比率が日本語基底監査に検出されたため、試験件数の説明を日本語化した。`python -X utf8 tooling/manifest.py --write`で1130 filesへ再生成後、`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`はexit 0。厳格日本語監査1133 files／検出0件、Schema 152／正常例152／negative fixture 196、Conformance 231 checks、登録10検査が成功し、`release_ready=false`、31件のrelease blockerを維持した。
- 一度ずつの正常完了／取消後にTEMP rootが不在だった結果は、反復性、同時・後続Task間分離、deadline／process crash／電源断後のcleanupを示さない。production Release Broker／authenticated IPCと正Task、installed Flutter／native Owner confirmation、durable positive Audit／改変検証、OneDrive Cloud Files／通常NTFS隔離matrix、result／diff Content Exposureも未成立。Codex Adapterの`task_execution=unsupported`とR2 `release_blocker`を維持する。

## R2追補 Agent CenterのBroker Task操作contract配線（2026-10-01）

### 成立した範囲

- Agent Centerから既存Broker transportを使ってAgent Session開始・再読込を要求し、Broker投影結果が登録済みRuntime／Workspaceへ一致することを照合する。
- Task instructionを実行せず事前検査へ送り、応答の識別子、hash、未実行状態を照合する。Brokerがunsupportedを返した場合はWorkspace Permission／Owner Approval／Task開始へ進まず、本文をエラー表示しない。
- Workspace Permissionと一回Task Owner Approvalを別操作として要求し、Brokerの厳密なreceipt受理後だけTask開始を表示する。Task start／state／cancel応答のrecord version、ID、status、hash、監査参照を検証し、結果本文は表示しない。Workspace差分は既存の独立経路のままとする。
- 成功経路のwidget testはcapabilityを`supported`と返すfake Brokerを使う。これは画面とserviceの接続確認用`FIXTURE`であり、製品AdapterのCapabilityを変更せず、production実行を証明しない。
- Task requestはBroker受理後にUI stateから破棄し、画面dispose時にも残存Task UI stateをclearする。

### 検証と未成立境界

- `flutter test --no-pub --no-test-assets --reporter expanded`（`apps/desktop_flutter`）: 成功、142 passed／0 failed。
- `flutter test --no-pub --no-test-assets --reporter expanded test/widget_test.dart --plain-name 'Agent CenterはBroker事前検査後に分離Owner確認とTask状態照会を使う'`（`apps/desktop_flutter`）: 成功、1 passed。
- `git diff --check`: 成功。`flutter analyze --no-pub`のローカル実行はOneDrive日本語pathを含むLSP応答のdecode中にAnalysis Serverがcrashし、Dart診断結果を返さなかった。これはanalyze成功ではない。Hosted Windows手動Actionsで同一commitのanalyzeとFlutter suiteを補完する。
- 初回`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`は日本語基底監査が画面英語ラベル3件とfake Broker error文字列1件を検出して失敗した。表示ラベルとfixture errorを日本語化して失敗履歴を保持し、`python -X utf8 tooling/manifest.py --write`で1128 fileのManifestを再生成した後、同じ統合validatorを再実行して成功した。厳格日本語監査1133 files／0 findings、Schema 152／正常例152／negative fixture 196、Conformance 231 checks、release gate、source packaging、smoke、証拠束、runtime assertion、最終開発監査の登録10検査がすべて成功し、`release_ready=false`と31件のrelease blockerを維持した。
- analyzer指摘を修正した後、`flutter test --no-pub --no-test-assets --reporter expanded`を再実行し142 passed／0 failed、`dart format --output=none --set-exit-if-changed`と`git diff --check`も成功した。最新のManifestは1130 file。`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`は終了値0で、厳格日本語監査1133 files／0 findings、Schema 152／正常例152／negative fixture 196、Conformance 231 checksと登録10検査すべてが成功し、31件のrelease blockerと`release_ready=false`を維持した。
- Windows手動workflow run #11（`workflow_dispatch`、2026-10-01、対象commit `be12e4e6140ff6e612ff9ef1897aca072b709e92`）はRust helper build成功後にDesktop `flutter analyze`で失敗した。追加testの相対`lib` import 1件と`const`不足2件が原因であり、Desktop全test・Mobile analyze・試験後clean確認は未実行、artifactはなし。失敗履歴を保持し、指摘修正をcommit `459369a637ff60b92f7ce1550d78ad3cb41548c2`へ記録した。
- 同じbranch上の後続Windows手動workflow run #12（`workflow_dispatch`、[run 36864800422](https://github.com/gatchimuchio/GUI-Shell/actions/runs/36864800422)、対象commit `459369a637ff60b92f7ce1550d78ad3cb41548c2`、runner `windows-2025`／`win25-vs2026/20260925.250.1`）は全体成功、7m36s、artifactなし。Rust helper build成功、Desktop analyzeは`No issues found`、Desktop全testは142 passed／0 failed、Mobile analyzeは`No issues found`、検査後の作業tree clean確認も成功した。これはhosted Windowsでのbuild／Flutter検査だけで、installed product・native Owner実操作・Agent Task production E2Eを証明しない。
- このUI／service testは`FIXTURE`のみ。installed Desktop UI、実native Owner confirmation、authenticated production IPC、Agent Task process、durable Audit／Recovery、OneDrive Cloud Files／NTFS隔離、result／diff Content Exposureは未実証。
- `task_execution=unsupported`、R2 production E2E `release_blocker`、`release_ready=false`を維持する。

## R2追補 staged Desktop UIのOwner登録・Session・Task拒否までのWindows実測（2026-10-02）

### 成立した範囲

- clean source commit `c0b15ae82a46a298e42bc2aaf54615d4c32682dd`からbuildしたstaged Windows Releaseを、Rust Desktop launcher経由で起動した。manifest上のapp／Broker helper／launcher SHA-256は順に`5d02c1f144dc5eddd7c3d6abcee1aa85a251bec880c8e64b93e5e7270878510b`、`d98a430d3714c1f29d1c98fa5fce536e70cf0679d8503278d9e574424b6fb1b8`、`c9da06cd11c960617b045fb29bf12d0c8637c267c29f3b2105109c7623b8fc0a`。
- Agent Centerの登録UIから実Codex CLIの登録要求を行い、native Owner確認を承認した。登録は合成NTFS Workspace、CLIの`--version`／`exec --help`確認、secret pathなしに限定し、Task・Permission・Approval・Trust・Credentialは生成されなかった。UIはRuntime `codex-local`、Workspace `workspace-local`、Task `unsupported`を表示した。
- Broker Session開始とWorkspace結合をUIから実行し、Session `240fda7256cf022e06cb92e47d5dc44a`、Audit `broker-audit-39/40`を確認した。Task事前検査には外部接続・file変更を指示しない合成本文だけを入力した。production UI／Broker経路は`AgentTask実行非対応`で拒否し、Audit `broker-audit-48/49`に受信／拒否を記録した。Worker、Task、Workspace変更、Task Permission／Approvalは発生しなかった。
- Product Audit UIは`durable_file_store`／`hash_chain`を表示した。保存JSONLのRust Broker固定field連結hashを79件について再計算し、連続hash、anchor件数、head一致を確認した。Task本文の完全一致はAuditに存在しない。Rust BrokerはAudit storeを開いて稼働し、UIからの追加要求を処理した。HMAC鍵そのものは読み出していない。
- staged manifestは`broker_mediated=true`だが、launcher runtimeのscopeは`per_user`、`isolated=false`と宣言する。このため、専用staging artifactからの起動は確認したが、完全隔離runtime／fresh user profile上のinstalled-product証拠へ一般化しない。終了時に当該staging frontend processは停止した。ほかのGUI-Shell processは停止していない。

### 検証境界

- 証拠classは製品UI／native confirmation／Broker動作の`LIVE_RUNTIME`、Broker Session／Audit stateの`INTERNAL_STATE`、CLI／Workspace／Task本文の`FIXTURE`。実model、実Credential、課金API、Task実行、OS保護設定変更はない。外部通信は独立計測していない。
- このE2Eは登録とSession作成後、現在の`task_execution=unsupported` gateが実Taskより前にfail-closedとなる負経路である。Workspace Permission／Owner ApprovalのTask用確認dialog、正のTask、実tool-child、secret／depth／hardlink隔離、OneDrive Cloud Files、通常NTFSのTask書込、失敗／deadline／crash Recovery、MxC TEMP後始末、結果／diff Content Exposureは未実証。
- Python `verify_audit_chain`はcanonical JSON hashを使いRust Broker形式と互換でないため、このstoreの検査器としては不適用。Rust実装の固定field連結方式でhashを照合した。HMAC anchorの鍵を使った独立再計算は行っていない。
- `task_execution=unsupported`、R2 production positive E2E `release_blocker`、`release_ready=false`を維持する。OneDrive Cloud Files検査のため同期rootへ合成fixtureを作成する操作は、所有者が別途許可するまで保留する。

### Repository検証

- `python -X utf8 tooling/schema_check/check_schemas.py`: 成功、Schema 152件／normal example 152件／negative fixture 196件。
- `python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`: 成功、231 checks。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`: exit 0。登録済みdevelopment check 10件すべて成功、日本語strict監査1133 files／0 findings、Manifest 1130 files。`release_ready=false`と既存release blockerは維持。
- `python -X utf8 tooling/manifest.py --write`、統合validator内のmanifest check、および`git diff --check`: 成功。
- このblockは文書・Manifest更新のみでRust／Flutter source変更なし。Rust／Flutter全testは今回再実行していない。

## R2追補 実Codex CLIのOneDrive Cloud Files／通常NTFS隔離probe（2026-10-02）

### 成立した範囲

- オーナーが合成fixtureのみの作成・使用を承認したため、専用OneDrive fixture内の合成markerだけを対象にした。実Credential、実Workspace data、実Model、課金APIは使用していない。
- Rust Adapterが生成するTask用sandbox profileと実Codex CLI `0.159.2`を使い、実MxC tool childから合成filesystem probeを実行した。OneDrive markerは実行前後ともWindows attribute `0x00501620`で、`UNPINNED`／`RECALL_ON_DATA_ACCESS`／`OFFLINE` bitsが立ち、`fsutil`のCloud Files reparse tagは`0x9000401A`だった。MxC childからの読取は`UnauthorizedAccessException`／HRESULT `-2147024891`（Access Denied）となった。
- 同じRust生成Task用profileの通常NTFS fixtureでは、登録合成secret file／directoryと深さ40のsecret pathの読取・書込拒否、Workspace内通常fileの読書込、別Agent Workspaceへの読書込拒否、動的hardlink作成拒否、相手MxC TEMP markerの不可視を確認した。TEMP／TMPはBroker scratchと一致しなかった。
- これは実Codex／MxC childの`LIVE_RUNTIME`と合成Workspace／markerの`FIXTURE`に分かれる。OneDrive Cloud Filesと通常NTFSの両環境で同じignored Rust probeを各1回成功させた。

### 検証境界と失敗履歴

- 初回OneDrive probeは、長いOneDrive絶対pathと深いfixture pathの合計でMxCがTask開始前にfail-closedとなった。fixtureの深いdirectory componentを1文字へ短縮し、試験の隔離要件を変えずに再実行して成功した。この初回失敗を成功へ読み替えない。
- 初回統合validatorはstrict日本語監査がRust test内の英語異常時diagnostic 1件を検出して失敗した。さらにPowerShell script全体をRust `format!` literalに置いた場合も監査器が診断文字列として扱ったため、異常時文面を日本語化した上でCloud File pathだけを安全なplaceholder置換で結合する形に分離した。二度のvalidator失敗履歴を保持する。
- `cargo test --locked --lib -- --ignored Rust生成Task設定で実Windows隔離の登録secretを拒否する --nocapture`はOneDrive Cloud Files、通常NTFSそれぞれ1 passed。`cargo test --locked --all-targets -- --test-threads=1`は12 target、444 passed／0 failed／7 ignored。Rust変更fileの`rustfmt 1.95.0 --edition 2021 --config skip_children=true --check`と`git diff --check`も成功した。
- 実Codex CLIとloopback fake Responses APIを使う別のignored試験は1 passedで、in-process Broker fixture経由の通常完了・取消、MxC TEMP marker／rootの終了後不在、外部接続遮断0件を確認した。Broker、Owner判断、Audit callback、Workspace、APIはfixtureであり、production Broker server／authenticated IPCではない。
- 本probeは製品Adapterを実行可能へ昇格せず、`task_execution=unsupported`を変更しない。登録前に作られたhardlink aliasをMxC childが読めないこと、installed Flutter UIからnative Owner確認、production IPC／Broker経由の正Task、永続positive Audit、deadline／crash Recovery、結果／diffのContent Exposureは未確認であり、R2 `release_blocker`と`release_ready=false`を維持する。

## R2追補 統合E2E用localhost偽Responses API経路の準備（2026-10-02）

### 実装範囲

- 通常のCargo既定featureは空のまま、明示指定時だけ有効な`r2-e2e`検証featureと、合成Workspace専用の`gui_shell_r2_e2e_responses`を追加した。検証featureはtask用既存Adapter／Broker／Owner操作経路を通すための局所的なbuild設定で、通常ReleaseのTask対応を有効化しない。
- 検証launcherのTask capabilityは、loopback偽API用portと、オーナー承認済み合成OneDrive root直下にある空の通常`codex-home`が両方検証できた場合だけ`supported`を返す。片方欠落、不正port、非通常directory、root／承認marker不一致、既存内容ありの場合は開始前に停止する。偽APIの接続先は`127.0.0.1`に固定し、通常`CODEX_HOME`やCredentialは引き継がない。
- 専用fixture serverの証跡は要求数、固定tool callの提示・送信、破損要求、途中要求、応答書込失敗、非loopback要求、合成Workspace内markerの有無だけであり、task本文やfile内容は出力しない。
- この準備blockではinstalled Flutter／native launcher／Brokerを通したTaskをまだ実行していない。fake API buildは通常Releaseやprovider接続の製品証拠ではなく、R2 production positive E2Eを閉じない。

### 検証

- `cargo +1.95.0 test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`: 成功。11 target、444 passed／0 failed／6 ignored。
- `cargo +1.95.0 check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`: 成功。
- `cargo +1.95.0 check --locked --manifest-path native/rust_helper/Cargo.toml --features r2-e2e --release --bins`: 成功。既存`minidora.rs`のdead-code warning 2件のみ。
- `python tooling/schema_check/check_schemas.py`: 成功。Schema 152件／通常例152件／negative fixture 196件。
- `python tooling/conformance_tests/run_conformance_skeleton.py`: 成功。231 checks。Codex Taskの通常Release gateはdefault-off検証featureと区別して検査する。
- `rustfmt +1.95.0 --edition 2021 --config skip_children=true --check native/rust_helper/src/adapters/codex_cli.rs native/rust_helper/src/bin/r2_e2e_responses.rs`および`git diff --check`: 成功。
- 途中の全target再実行ではWindows loopback／TLS接続resetが2回、別の単独再実行ではConformance並走下でA2A loopback読取失敗が1回あった。該当A2A試験のfocused retry 3回と最終全target再実行は成功し、最終全target結果を採用する。失敗原因は確定していないため失敗履歴を保持する。

### 維持する境界

- `task_execution=unsupported`、R2 production positive E2E `release_blocker`、`release_ready=false`を維持する。Owner／Permission／Approval、production IPC、positive durable Audit、OneDrive Cloud Files／通常NTFSの統合実行、failure／deadline／crash Recovery、MxC TEMP cleanup、result／diff Content Exposure、通常ReleaseでのTask能力は未検証である。

## R2追補 native Owner登録dialogと試験build capability表示の整合（2026-10-02）

- source commit `f69ec7abfc5072dec2937f64eddeb03d0eaeab80`からFlutter Windows Release、通常Broker helper Release、`r2-e2e` feature付きRust Desktop launcher／loopback serverをbuildし、新規の隔離installed layoutへstageした。試験IdentityとLOCALAPPDATAはrun固有で、通常利用者の設定・Audit領域へ混ぜていない。
- オーナー承認済みの合成OneDrive fixtureだけを使い、D4 Pocketの実Agent画面からCodex CLI／Workspace登録を開始した。native Win32 Owner dialogを実際に取得すると、feature付き試験buildが条件成立時にTask capabilityを`supported`として返す一方、「unsupportedのまま」と表示していた。この不整合を確認して登録要求をNoで拒否した。Runtime登録、Permission、Task Approval、Task実行、Task Auditは発生していない。
- `desktop_launcher.rs`の登録確認文をCargo featureに合わせ、通常buildは従来どおりunsupportedを表示し、`r2-e2e` buildはloopback API・隔離CODEX_HOME・合成Workspace専用の検証buildであり、登録だけではPermission／Approvalを与えない旨を示すよう修正した。通常product capabilityやdefault featureは変更していない。双方の表示をRust testで固定した。
- computer-useの`activate_window`は対象Owner dialogに対して2回timeoutしたが、返却済みwindow handleで直接状態取得することで、実dialog本文とNo選択後のアプリ状態を確認できた。No後にAgent登録が存在しない表示を確認した。
- この修正後の再build・validationと、fresh installed appでのOwner登録／Workspace Permission／Task Approval／Task実行／positive durable Auditは未実行。前回stage artifactは修正前sourceに由来するため再使用しない。R2 `release_blocker`、通常buildの`task_execution=unsupported`、`release_ready=false`を維持する。

## R2追補 installed positive E2E試行で発見したWorkspace Permission receipt不整合の修正（2026-10-02）

- source base `1a1f27664e5763a15ba44b2900e6cacb636af381`から作ったfresh staged `r2-e2e` installed runで、実Flutter UI、native Owner登録、Broker Session／Workspace結合まで進んだ。Owner確認付きWorkspace Permission発行後、Flutterは「receipt不正」と表示したが、読み取り専用の再事前検査ではBroker内に一回・短期限のactive Permissionが存在した。Task start前に停止し、Task process／model request／Tool call／Workspace writeは行われなかった。fake API観測は要求0、tool提示／送信なし、markerなし。run専用frontendを停止するとBrokerのprocess-local Permissionも消失した。
- 原因はFlutterが`{uses_remaining,status=issued_unconsumed}`だけを期待する一方、Brokerが`agent_task_workspace_permission.schema.json`準拠の13-field receipt（status=`active`）を返していたこと。`agent_task_client.dart`を実Broker contractに合わせ、未知field、形式不正、Runtime／Session／Workspace不一致、Registration hash不正、operation／scope／decision／source不一致、期限・回数・状態不一致をfail-closedで拒否する厳密照合へ修正した。Permission IDはUIへ露出しない。widget fixtureもSchema準拠に更新し、正常receiptと14種の異常receiptをclient testで固定した。
- 失敗runのPermission発行を成功扱いせず、Task実行へ進めなかった。Owner承認のないままの実行、外部API要求、Credential利用、製品設定変更はない。以前の失敗はこの記録に保持し、修正後のfresh runと混同しない。

### 検証

- `dart format`、`git diff --check`: 成功。
- `flutter pub get --enforce-lockfile`（Desktop／Mobile）: 成功。lockfile変更なし。
- `flutter analyze --no-pub`（Desktop／Mobile）: 成功、No issues found。OneDrive日本語path上の旧LSP JSON parse失敗を避け、短い一時source copyで実行した。
- 修正後のDesktop `flutter test --no-pub --reporter compact`: Rust helperを同一sourceからdebug buildした後、全144件成功。先行するtest asset無効runはShader asset欠落で失敗し、assets有効runの最初の試行は必要debug helper未buildのため既存Rust連携2件が失敗した。いずれも条件を正して全suiteを再実行し、全件PASSを確認した。
- `python -X utf8 tooling/schema_check/check_schemas.py`: Schema 152件／通常例152件／negative fixture 196件で成功。
- `python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`: 231 checks成功。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`: 成功。Windows product evidence欠落と31件のrelease blockerを検出し、`release_ready=false`を維持した。これはvalidator自体の検査成功であり、各release blockerの解消を意味しない。

### 維持する境界

- 修正後のソースによる新規配置製品の実行経路でのTask成功、永続化された成功記録と再検証、隔離条件の組合せ試験、失敗・期限切れ・異常終了時の復旧、MxC一時領域の後始末、結果／差分の内容露出は次回runで未確認。通常Releaseの`task_execution=unsupported`、R2 `release_blocker`、`release_ready=false`を維持する。`r2-e2e`偽Responses APIは合成試験専用であり、実providerとの相互運用や通常配布用機能の成立を証明しない。

## R2追補 修正commitからのfresh installed run4とUI操作基盤停止（2026-10-02）

- clean source commit `7dbeba1361d852180dd12c4165fffcab6e355da9`から作ったsource archiveを使用し、変更対象5 fileのSHA-256がcommit済み作業ツリーと一致することを確認した。Flutter 3.44.0の`flutter build windows --release --no-pub`は成功（271.8秒）。Rust 1.95.0の`cargo +1.95.0 build --locked --release --features r2-e2e --manifest-path native/rust_helper/Cargo.toml --bins`も成功（7分35秒、既存MINIDORA dead-code warning 2件）。
- run `r2-synthetic-task-e2e-7dbeba1-run4`を`C:\d4p-r2-install-7dbeba1-run4`へfresh stageし、local dataとTEMP／TMPをrun専用rootへ分離した。Flutter UI、Rust launcher、feature付きBroker helper、loopback偽Responses APIのSHA-256はregistryの同名probeに記録した。
- CLI `0.159.2`の`--version`だけをローカル確認した。Owner承認済み合成fixture内の空CODEX_HOMEを指定し、偽API serverはloopback `127.0.0.1:49630`で待機した。OneDrive Workspace内の`private/credential-backup.txt`は存在だけを確認し、内容は読んでいない。
- installed `D4 Pocket` overview画面の表示までは`LIVE_RUNTIME`で観測した。Agent Centerへの画面操作後、Computer Useのclick／画面状態更新が失敗し、fresh windowを再選択した復旧後も`Cannot read properties of null (reading 'window')`で状態取得できなかった。UI helperの再試行条件に達したため操作を停止し、native Owner dialog、Runtime登録、Session、Permission／Approval、Taskには進まなかった。
- 偽API停止時の証跡はrequests／models／tool提示／tool送信／不正body／途中request／応答失敗／外部要求がすべて0、Workspace markerなし。Task childは起動していない。実credentialと実modelは不使用。OS保護設定は変更していない。
- このrunで起動したlauncher PID 18044とFlutter frontend PID 17844だけを停止した。run専用rootに残った`broker_session.json`（endpoint／secretを含むsession file）と`desktop_launcher.lock`を完全path照合後に削除し、Audit storeは保持した。`audit_anchor.key`は読んでいない。fake API listener終了を確認した。

### 判定

- 本runはclean installed artifactの起動表示と合成APIの無要求を示すだけで、R2 production positive E2Eの完了証拠ではない。UI automation failureによりOwner以降へ到達しておらず、source側receipt修正のproduction経路検証も未成立。R2 `release_blocker`、通常Releaseの`task_execution=unsupported`、`release_ready=false`を維持する。UI操作基盤が復旧した後、別のfresh installed runで登録からTask・durable Auditまで再実施する。

## R2追補 fresh installed positive Task試行run7と偽API再送ループ（2026-10-02）

- source commit `7dbeba1361d852180dd12c4165fffcab6e355da9`由来の`r2-e2e` staged app／launcher／Broker／localhost偽Responses APIを使用し、通常NTFS上のrun専用Workspace、LOCALAPPDATA、TEMP／TMP、空CODEX_HOMEで実施した。実Codex CLI `0.159.2`を起動し、API接続先はloopback `127.0.0.1:54155`だけだった。実provider、実credential、課金要求、外部通信は使用していない。
- 実Flutter UIからRuntime `codex-local`とWorkspace `workspace-local`をnative Owner確認付きで登録し、Broker Session `179daa7b113a0ede282f79fe332e901e`を作成した。Workspace `agent_task.execute` Permission（同Session／Workspace、1回、短期限）と別個のTask Approval（同Session／Workspace、1回、最大15分）をそれぞれOwner確認で発行した。合成Task `76f0d99f6c34a1142d314d6f0f6af430`をUIからBroker経由で開始し、開始Audit参照は`broker-audit-58`、終端状態は`failed`（Audit参照`broker-audit-68`）となった。取消要求は`broker-audit-69`で拒否されており、取消成功とは扱わない。
- CLI childはMxC経由でPowerShell tool childを起動した。偽Responses APIはtoolを提示・送信した後、同一tool callをmarker未作成のまま128 POSTにわたり再送した。期待したWorkspace report、TEMP observer marker、完了marker、継続marker、Workspace外書込markerはいずれも存在しなかった。したがってpositive Task、Workspace差分、TEMP書込成否は成立・観測できず、task成功とは扱わない。
- 終了後、scratch recovery journalはHMAC付きで空entryだった。合成secret fileの存在のみを扱い内容は読んでいない。Task childと偽API listenerは終了し、このrunのlauncher／Broker processも個別確認後に停止した。Task結果とAudit参照は記録したが、run7についてAudit chainの独立再計算はしていない。run専用の失敗証跡は削除せず保持した。

### 判定

- run7はUI→Broker→実CLI／MxC→tool送信までの`LIVE_RUNTIME`経路を示すが、Taskは失敗しpositive結果ではない。128回の再送は偽Responses API fixtureの無制限再提示を示す一方、最初のPowerShell toolがreportを書けなかった根本原因は未確定である。次はfixtureを一回限り・fail-fastにし、Workspace reportをTEMP操作より先に記録して原因を特定する。R2 `release_blocker`、通常Releaseの`task_execution=unsupported`、`release_ready=false`を維持する。

## R2追補 偽Responses API再送停止とMxC TEMP probe安定化（2026-10-02）

- run7後のfixture修正では、Responses APIのSSEを`response.created`／`response.in_progress`／sequence number付きの正式なresponse lifecycleへ合わせ、function call item／call IDと`function_call_output.call_id`を厳密に結合した。結果を受け取れない同一tool callは再提示せずHTTP 409でfail-fastし、要求形状はitem種別数だけを記録して本文・commandを残さない。正常応答はHTTP/1.1の明示的close／half-closeで送信し、Windows loopbackの応答直後RSTを避ける。
- ignored Rust統合試験は実Codex CLI `0.159.2`を起動し、loopback偽Responses API、in-process Broker／Owner／Audit fixture、合成NTFS Workspaceを通した。正常完了とBroker取消の両TaskでMxC TEMP書込・限定scopeを確認し、child終了後に合成markerとTEMP directoryが不在、外部要求が0となることを確認した。これはproduction Broker IPCやdurable product Auditではない。
- TEMP probeは最初にstage=`temp_checked`の確定reportを待ち、TEMP／TMPがBroker Workspace scratchまたはMxC AppContainerの`...\\sandbox.{GUID}\\AC\\Temp`に一致しなければ書込み前に停止する。取消probeも同じ二つの許可scope以外へ書かない。

### 検証と失敗履歴

- `cargo +1.95.0 test --locked failed_tool_result_is_not_replayed_as_another_exec_command -- --nocapture --test-threads=1`: 成功、1 passed。初回2回はWindows loopback WSA 10054で失敗し、半閉鎖を追加する前のserver shutdownではHTTP body受信前のRSTも観測した。成功応答後のwrite-half shutdownとpeer drainを追加してfocused retryを通した。
- `GUI_SHELL_CODEX_TASK_BROKER_TEST_EXE`にCodex CLI `0.159.2`を指定した`r2-e2e` ignored統合試験: 成功、1 passed。修正前のfixtureでは同tool call再提示、function output重複、Responses stream切断が複数回発生した。正規SSE lifecycle／sequence number／full completed response導入後に3回連続成功し、half-close後の最終再実行も正常完了と取消を通した。
- `cargo +1.95.0 test --locked --all-targets -- --test-threads=1`: 成功、446 passed／0 failed／7 ignored。直前の全target試行では既存MINIDORA loopback試験が一度失敗してfocused retryは成功した一方、test server shutdown時のRSTでfixture unit試験が失敗した。これらの失敗は保持し、最終全target再実行を採用した。
- `rustfmt +1.95.0 --check`（変更Rust 3 file）および`git diff --check`: 成功。
- 初回`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`はstrict日本語監査が一時領域診断、要求形状診断、HTTP wire responseの3箇所を検出して失敗した。診断ラベルを日本語化し、既存の機械形式例外に合うHTTP/1.0 wire形式へ限定変更した後、`python -X utf8 tooling/日本語基底監査.py --strict`は0 findingsで成功した。統合validatorは修正後に再実行する。
- HTTP wire形式を1.0へ合わせた後の最初のfocused unit再試行は、試験fixtureのstatus-line期待値だけが1.1のままで失敗した。成功／409確認をHTTP/1.0へ揃えたため、test codeと実装の不一致を解消した状態で再実行する。
- HTTP/1.0 wire変更後のfocused unitは1 passed。続く直列全target実行ではfixture loopback unitと既存update package TLS loopback testがWSA 10054で各1回失敗し、原因は確定していない。fixture unit focused retryは成功し、再度の直列全target実行は446 passed／0 failed／7 ignored。最初のHTTP/1.1 response期待値不一致と先行loopback resetも失敗履歴として残す。

### 維持する境界

- このblockはRust test fixture／ignored integration testだけの修正で、production implementation、Flutter、通常Release capabilityを変更していない。実installed productのOwner／Permission／ApprovalからTask成功、file-backed positive Audit chainの独立検証、Cloud Filesと通常NTFSの隔離matrix、failure／deadline／crash Recovery、result／diff Content Exposureは未成立。`task_execution=unsupported`、R2 production E2E `release_blocker`、`release_ready=false`を維持し、次はこのcommitからfresh installed runを行う。

## R2追補 fresh installed run16／run17のTask失敗と安全なtool出力診断（2026-10-02）

### 失敗履歴・観測

- run16／run17は、当時の作業branch `codex/r2-inspector-owner-confirmation-20261002`のsource commit `fca1d0c9d83f03108ebaaf64ea742ca38079b4fc`からstageした試験buildであり、指定基準HEAD `c306548e559d069fafd7dfb20088882521b04bb1`そのものではない。後続の診断buildは基準HEADへ戻した`main`で作成する。
- fresh install run16では、合成fixture、native Owner確認、Session／Workspace結合、Task preflightまで進んだ。UIに残った古いsnapshotからの遅延操作によりPermission確認のissued-at期限が切れ、Brokerは`broker_issued_at_invalid`として拒否した。Task、偽API要求、tool実行、Workspace変更は発生せず、当該失敗Auditを保持した。
- fresh install run17では、native Owner登録、Session／Workspace結合、preflight、Workspace Permission、別個のTask Owner Approval、Task開始までをinstalled UI→Rust Brokerで実行した。Task開始Audit参照は`broker-audit-43/44`、終了状態は`failed`・Audit参照`broker-audit-53`である。Task結果／diffはBrokerから未取得であり、成功とは扱わない。
- run17のloopback偽Responses APIは4 POSTを受け、tool offer／call／function output受領が各成立した。Workspace完了markerがないためAPIが409を返し、同一結果を含む後続requestが3回観測された。これはtoolを3回再実行した証拠ではなく、API応答後のrequest再試行である。外部要求、不正body、途中request、応答書込み失敗は0件。TEMP reportとWorkspace完了markerはいずれも存在しない。
- run17 Broker Audit JSONLは終了後101件。Rust `BrokerAuditEvent`の連結hash規則を独立再計算し、event ID一意性、全previous hash、全event hash、anchorの件数とhead一致を確認した。anchor HMACは鍵を読まず未検証であり、Broker再起動後の復元検証を示すものではない。Audit本文へTask instruction／credential値は追加保存していない。
- run17のscratch recovery journalは0件、Workspace内には登録済みの空synthetic secret fixtureだけが残り、Workspace外書込みmarkerとTEMP内容はない。偽API、run17のlauncher／frontend processは停止した。強制停止で残った隔離run専用`broker_session.json`と空launcher lockは保持Auditとは分離された一時状態であり、直接削除はホスト実行ポリシーに拒否されたため、後続の管理されたlauncher cleanup経路で処理する。一般ユーザーのLOCALAPPDATAや資格情報には触れていない。

### 診断fixture変更・検証

- 偽APIが受け取った`function_call_output`本文を保存しないまま、出力形式・byte数・固定済み失敗分類・合成開始marker有無だけを返す診断を追加した。固定分類はHostFxr起動失敗、shell解決失敗、sandboxアクセス拒否、期限超過、その他error、出力なし／error兆候なしに限る。要求本文、stdout／stderr、path、secretは診断値へ含めない。PowerShell probeの先頭へ固定合成開始markerを追加した。
- `cargo +1.95.0 test --locked --offline --manifest-path native/rust_helper/Cargo.toml --features r2-e2e --bin gui_shell_r2_e2e_responses tool_output_diagnostic_records_only_bounded_safe_classification -- --nocapture --test-threads=1`: 1 passed。synthetic secretとlocal pathを含む入力が診断出力へ漏れないnegative testを含む。
- `GUI_SHELL_CODEX_TASK_BROKER_TEST_EXE`へCodex CLI `0.159.2`を指定したignored Rust integration test: 1 passed。実CLI／MxC childのTEMP marker消失、Broker scratchとの分離、取消後cleanup、外部要求0を再確認した。これはin-process Broker／test fixtureであり、installed production Broker E2Eではない。
- `cargo +1.95.0 check --locked --offline --manifest-path native/rust_helper/Cargo.toml --features r2-e2e --bin gui_shell_r2_e2e_responses`: 成功。変更Rust fileの`rustfmt +1.95.0 --edition 2021 --config skip_children=true --check`と`git diff --check`も成功。
- 初回focused試験呼出しはbinary target名、feature指定、exact test名の不一致で実行前に拒否または0件となった。正しい`gui_shell_r2_e2e_responses` targetと`r2-e2e` featureを指定した後に1件実行し成功した。Rust source／production path failureへ読み替えない。

### 次の検証と維持する境界

- 安全な出力分類は診断専用であり、run17の失敗根本原因は未確定。次に指定基準`c306548`のsourceからfresh installed buildを作り、diagnostic summaryを使ってtool実行の失敗段階を切り分ける。
- このfixture追加はtest-onlyで、通常ReleaseのCodex Task能力を有効化しない。正のproduction Broker E2E、restart後のAudit anchor HMAC検証、OneDrive Cloud Files／通常NTFS隔離matrix、failure／deadline／crash Recovery、result／diff Content Exposureは未成立。`task_execution=unsupported`、R2 production E2E `release_blocker`、`release_ready=false`を維持する。

## R2追補 fresh installed run18の登録拒否原因と試験home検査順修正（2026-10-02）

### run18の観測と失敗履歴

- source commit `f71b7b9a9ba848f33c78a8381e9af2d4fbd7bd25`からFlutter Windows Releaseと`r2-e2e` Rust launcher／Broker／loopback偽APIをbuildし、run固有の新規installed layoutへstageした。実Flutter Agent CenterとWin32 native Owner確認dialogで、合成WorkspaceとCodex CLI `0.159.2`の登録を開始し、Owner確認はYesで確定した。
- installed run18 Auditでは登録要求`broker-audit-46`がreceived、interface検査後の登録`broker-audit-47`と集約拒否`broker-audit-48`がrejectedだった。UIには「Owner確認後のAgent CLI interface検査に失敗」と表示された。Runtime／Workspace登録は成立せず、Workspace Permission、Task Approval、Task、model／tool要求、Workspace writeは発生していない。native確認は登録要求だけで、Task権限を与えたものではない。
- 隔離run18のloopback偽Responses APIは停止時に要求0、tool提示／送信なし、tool結果なし、Workspace markerなしを返した。安全な同一環境probeでは`--version`成功、`exec --help`成功、必要な8 interface tokenをすべて確認した。実Codex CLI probe後に試験用`CODEX_HOME`へ`tmp` directoryが生成されていた。
- 根本原因は、CLI version/help probeがCODEX_HOMEを初期化した後に、登録経路が「試験用CODEX_HOMEは空」と検査して自ら拒否する順序だった。TaskやAuthorityの拒否ではなく、試験初期状態検査と実CLI probeの副作用が衝突していた。

### 修正と検証

- `native/rust_helper/src/adapters/codex_cli.rs`で、`r2-e2e`に限りloopback／空CODEX_HOME検査をCLI probeより前へ移動した。通常buildとdefault feature、production Task capabilityは変更していない。probeは引き続き隔離run専用CODEX_HOMEだけを継承し、利用者の通常Credential／homeを使わない。
- `cargo +1.95.0 test --locked --offline --all-targets --features r2-e2e -- --test-threads=1`: 初回は既存`broker::update_download::tests::failed_replacement_keeps_the_existing_corrupt_package_unchanged`がWindows localhost接続reset WSA 10054で失敗。該当testの単独再実行は1 passed。全target再実行は449 passed、0 failed、7 ignored。
- `python -X utf8 tooling/schema_check/check_schemas.py`: schema 152、example 152、negative fixture 196で成功。`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`: 231 checks成功。
- 変更前の`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`は作業中sourceのmanifest hash不一致でmanifest／release gate／packaging checkが失敗した。`python -X utf8 tooling/manifest.py --write`後の同validator再実行はexit 0で全検査項目が成功した。厳格日本語監査も0 findingsで成功。開発validator成功は製品完成を意味せず、既存release blocker 31件、`release_ready=false`は維持する。
- run18の失敗Auditと隔離状態は失敗履歴として保持し、アプリと偽APIは停止した。`audit_anchor.key`は読んでいない。Audit chain／anchor HMAC／再起動後耐久性はこのrunでは独立検証していない。

### 維持する境界

- 上記修正commitからのfresh installed再試行、登録後のPermission／Approval／positive Task、Workspace marker／result／diff、durable Auditと再起動後のchain／HMAC検証は未実行であり、本節時点では成立していない。OneDrive Cloud Files／通常NTFSの隔離matrix、failure／deadline／crash Recovery、MxC TEMP cleanup、result／diff Content Exposure、通常ReleaseのTask能力も未完了。`task_execution=unsupported`、R2 production E2E `release_blocker`、`release_ready=false`を維持する。

## R2追補 fresh installed run19の登録再失敗と失敗理由の安全な細分化（2026-10-02）

- source commit `ae95ddf1a53f547e655ddff0464f42966d8b7ddc`由来としてbuild／stageしたrun19のFlutter、launcher、Rust Broker helperは、直前のbuild artifact hashとそれぞれ一致した。実Flutter Agent Centerで合成Workspace／Codex CLI `0.159.2`の登録を開始し、除外pathの入力形式を正規化した後、実Win32 native Owner確認dialogでOwner Yesを選択した。
- run19 Auditは`broker-audit-36`で要求を受領し、`broker-audit-37`で拒否、`broker-audit-38`で集約拒否を記録した。UIは「Owner確認後のAgent CLI interface検査に失敗」。Runtime／Workspace登録、Permission、Task Approval、Task、model／tool request、Workspace writeは発生していない。loopback偽APIへのrequestと完了markerもなかった。
- 実行後の隔離CODEX_HOMEに`tmp`が存在した。一方、別の新規合成環境で実Codex CLIを同じ許可環境変数集合から直接起動したprobeは`--version`／`exec --help`ともexit 0で、要求interface tokenと`workspace-write`を確認した。この直接probeは`LIVE_RUNTIME`だが、Broker Adapterが同じ応答を読んだことの証拠ではない。
- run18で推定した「空CODEX_HOME確認とCLI probeの順序衝突」の修正だけでは、run19登録を閉じたと確認できなかった。既存protocolは複数のAdapter初期化失敗を同一表示へ丸めるため、どの固定検査段階で拒否したかは未確定のまま保持する。
- `native/rust_helper/src/adapters/mod.rs`に固定文言だけを返す失敗理由射影を追加し、version probe、exec help probe、probe起動／期限／出力上限の失敗を識別する。CLI stdout／stderr、path、OS errorなどの可変値はAudit／UIへ反映しない。protocolは既知理由だけをこの射影へ通し、未知理由は従来の一般拒否へ閉じる。
- 診断理由testを加えた最初のRust全target実行では`broker::update_download::tests::local_tls_server_repairs_only_after_verified_package_bytes`がWindows WSA 10054で失敗した。対象test単独再実行は1 passed。manifest更新後の全target再実行は450 passed／0 failed／7 ignored（library 402 passed／6 ignoredを含む）。`python -X utf8 tooling/schema_check/check_schemas.py`は152 schema／152 example／196 negative fixture、conformanceは231 checksで成功。strict日本語監査は1134 files／0 findingsで成功。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`のmanifest再生成前実行は、変更後の`native/rust_helper/src/broker/protocol.rs` hash不一致だけでmanifest／release gate／packaging portabilityが失敗した。`python -X utf8 tooling/manifest.py --write`後の同validator再実行はexit 0で全development checkが成功した。validator内のrelease blocker 31件、`release_ready=false`はそのまま。
- 実装中に最初のRust compileで`map_err`の`String`／`&str`型不一致が見つかり、借用closureへ修正してから上記全target再実行を行った。
- この修正は登録失敗の識別性だけを改善し、Permission、Approval、Trust、Task capabilityを変更しない。fresh installed再試行、Permission／Approval／positive Task、durable Auditのrestart後検証はこの記録時点で未成立。`task_execution=unsupported`、R2 `release_blocker`、`release_ready=false`を維持する。

## R2追補 fresh installed run20のTask失敗とprobe境界分離（2026-10-02）

- run20はsource commit `1e90ad1014e67a8d8ab22427966c32c168807636`からFlutter Windows Releaseと`r2-e2e` Rust helper／launcherをbuildし、run固有layoutへstageした。アプリ・Broker・launcher artifact hashはinstalled manifestに固定した。実Flutter Agent Centerから実Codex CLI `0.159.2`と合成NTFS Workspaceを登録し、native Owner確認後にSession、Workspace Permission、別個の一回限りOwner Approvalを発行した。通常ReleaseのTask capabilityは変更していない。
- Task `4d27416c282e47581836c77f395842dc`はproduction IPC経路から開始Audit `broker-audit-94`まで到達した。loopback偽Responses APIは4要求を受け、`exec_command` tool offer／call／function outputを各観測した。応答後の同一要求再試行3件をHTTP 409で拒否し、非loopback要求、不正body、途中request、応答書込み失敗は0件。実model、資格情報、外部APIは使っていない。
- Task toolはWorkspaceへtemp診断reportを書いた。reportはTEMP／TMP設定済み、scope=`mxc_appcontainer`、temp書込み=`failed`、例外型`System.Management.Automation.RuntimeException`、HRESULT `-2146233087`を記録した。失敗stepを記録していなかったため、Test-Pathと一時file書込みのどちらで失敗したかは未確定。host watcherはtemp書込み失敗を理由にcontinuation markerを拒否し、secret read、Workspace外read/write、Task完了markerまで進めなかった。Taskは失敗・取消、最終Audit `broker-audit-104`。synthetic secretは空fileであり読取りを試みず、outside write markerとTask completion markerは生成されていない。
- Agent CenterはTask `failed`を表示した。実行directory、変更file、tool／command、試験状態、diff、resultのBroker projectionは未接続のまま。Auditはrun専用file-backed storeに残したが、本runのchain再計算、anchor HMAC、Broker再起動後の再読込は未検証。
- `native/rust_helper/tests/support/codex_loopback_responses.rs`の試験専用probeを修正した。TEMP診断に固定`temp_step`を加え、許可されたTEMP scopeを確認した上で、TEMP書込み結果とは独立にsecret／Workspace外境界とWorkspace書込みを試せるようにする。許可されないscopeではhost continuationを出さず、拒否試験へ進まない。TEMP失敗はそのまま記録し、成功へ読み替えない。この変更は通常Release、production Task capability、Permission／Approvalを変更しない。
- 検証: `cargo +1.95.0 test --locked --offline --manifest-path native/rust_helper/Cargo.toml --features r2-e2e --bin gui_shell_r2_e2e_responses temp_probe_keeps_boundary_checks_independent_from_temp_diagnostic -- --test-threads=1`は1 passed。対象Rust fileの`rustfmt +1.95.0 --edition 2021 --config skip_children=true --check`と`git diff --check`は成功。全target／統合validatorはこの追補のcommit前に実行する。
- 維持する境界: run20は実installed UI／native Owner／production Broker Taskを開始したがTask成功ではない。durable positive Audit、restart検証、result／diff Content Exposure、MxC TEMP cleanup、OneDrive Cloud Files／通常NTFS隔離matrix、failure／deadline／crash Recoveryは未成立。`task_execution=unsupported`、R2 production E2E `release_blocker`、`release_ready=false`を維持する。次はこのfixture修正commitからfresh installed runを行う。

## R2追補 fresh staged run23のpositive Taskとfile-backed Audit検証（2026-10-02）

- remote `main`と一致したclean source commit `19bab4e05b152b99ac1a01308f6be17f7d7721b0`からstageしたrun23で、実Flutter UI、Rust Desktop起動器／PID照合付きnamed-pipe relay、production Broker、実Codex CLI `0.159.2`／MxC childを通した。製品artifactには試験専用`r2-e2e`機能を有効化しており、通常ReleaseのTask能力は変更していない。app／launcher／Broker helper／localhost偽Responses APIのSHA-256は`release_blockers.registry.json`へ固定した。
- 合成NTFS Workspaceと空の隔離CODEX_HOMEを使用し、APIは`127.0.0.1:58556`の偽Responses APIだけに向けた。実provider、実model、実資格情報、外部API要求は不使用。偽APIは2 requestを受け、tool提示・送信・tool結果受領を確認し、不正body、途中request、応答書込み失敗、非loopback／外部要求はいずれも0件。
- installed UIからRuntime／Workspace登録とSession `566fb8538a6ac3afa6fe026fa831d83e`を行い、native Owner確認で登録、`agent_task.execute`の一回Workspace Permission（Audit `broker-audit-83`）、別個の一回Task Approval（Audit `broker-audit-85`）を発行した。Task `4e0b5663267328232d6156ebea9e9134`はBroker経由で開始され、UIは`completed`、結果hash `sha256:55c569826e219f9d375b6ec4b1e4a15d2d569e81a4ff75403e579e18c53787ab`を表示した。Audit `broker-audit-94`の理由は`Agent Task完了（結果本文非保存・hashのみ）`であり、本文ではなくhashだけが記録された。
- file-backed Audit JSONLの2026-10-02 13:54 UTC時点の検査snapshot 139件について、Rust `BrokerAuditEvent`の実際のfield連結hash方式で全event hash、previous link、event ID一意性を独立再計算し、不一致0件を確認した。headは`sha256:0aeedfdb46285c1014689d09cfc9ae9773b31ee1b8c5d9fdf9a2f6fc10f4aaaf`。anchorのevent count／head一致とHMACも確認し、Task指示本文はAuditに存在しなかった。これは稼働中store snapshotの独立検証であり、Broker停止後の再起動・再読込検証ではない。
- Workspaceには合成完了marker（20 byte）とTEMP診断report（769 byte）が作成された。空のsynthetic secret canaryは0 byteのままで、Workspace外write markerは存在しない。reportはTEMP書込み`failed`、scope=`mxc_appcontainer`、step=`temp_directory_check`、例外`System.Management.Automation.RuntimeException`、HRESULT `-2146233087`を示す。TEMP物理cleanup成功やsecret／外部read拒否は、このpositive Taskから推論しない。
- Agent CenterにはTask状態・結果hash・Audit参照が表示されたが、画面自身が実行directory、変更file、tool／command、test、result本文、diffのBroker projection未接続を明示している。Workspace diff／Content Exposure経路の証拠にはならない。

### 判定と残存範囲

- run23は試験専用feature付きstaged appでのpositive Taskおよびfile-backed positive completion Auditを成立させた。通常ReleaseのTask実行能力、Auditの再起動後検証、result／diff表示、MxC TEMP cleanup、OneDrive Cloud Files／通常NTFSの隔離matrix、失敗／期限／crash Recoveryは未成立。偽APIによるLIVE_RUNTIME試験は実provider相互運用の証拠ではない。
- 偽API listenerとTask childは停止済み。合成runのAudit／Workspace証跡は保持した。D4 Pocket launcher／frontendはまだ実行中であり、通常終了はtrayの`終了`操作が必要だが、この実行環境から通知領域を操作できなかった。未確認の強制終了は行っていない。
- したがって本runで「positive Task completion」検証点は閉じるが、R2 production E2E全体は閉じない。`task_execution=unsupported`、R2 `release_blocker`、`release_ready=false`を維持する。

## R2追補 現行Release Broker process再起動後のAudit保持と旧Session拒否（2026-10-03）

### 実装と観測

- `native/rust_helper/tests/agent_task_production_e2e.rs`のignored Windows E2Eを拡張し、同じ合成Workspace／file-backed storeを使うRelease Brokerを順次2 process起動する。1つ目の通常終了後に同じstoreから2つ目を起動し、通常Session IDと資格が新しくなること、再起動前の要求は拒否され本文を返さないこと、新Sessionからのfresh要求は受理されることを検査する。
- 2つ目のBroker終了後に永続storeを開き直す。再起動前後双方の応答Audit参照と、旧Session要求の拒否Auditがhash chain上に残ること、合成Task指示・secret markerがAudit本文にないことを確認する。
- 現行Release Broker helperはproduction Rust source commit `59b32108dc0003a6834d4be9059970c9f95b898c`からbuildし、実Codex CLI `0.160.0`のinterface検査を行った。Windows `LIVE_RUNTIME`はstandalone Release Broker／loopback IPC／実CLI probeの範囲、Workspace・store・markerは`FIXTURE`。実Task、model／Credential、installed Flutter UI、Owner画面、外部通信は使っていない。
- 同日、Ownerのtray `終了`操作後にrun23の既知Launcher PID `16268`とFlutter PID `8268`が不在、偽API port `58556`のlistener不在を確認した。run23の元cleanup recordは試験終了時点の記録として保持し、後追い終了確認をregistryへ追記した。

### 検証と境界

- `cargo +1.95.0 test --locked --offline --manifest-path native/rust_helper/Cargo.toml --test agent_task_production_e2e -- --ignored --exact windows::production_agent_task_gate_fails_closed_over_authenticated_ipc --nocapture --test-threads=1`: 成功、1 passed／0 failed。
- `cargo +1.95.0 test --locked --offline --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`: 成功、449 passed／0 failed／7 ignored。
- `python -X utf8 tooling/schema_check/check_schemas.py`: 成功、Schema 152件／normal example 152件／negative fixture 196件。`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`: 成功、231 checks。
- `rustfmt +1.95.0 --edition 2021 --config skip_children=true --check native/rust_helper/tests/agent_task_production_e2e.rs`と`git diff --check`: 成功。`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`: 成功、development check 10件すべてpass。厳格日本語監査1134 files／0 findings、Schema 152件／example 152件／negative fixture 196件、Conformance 231 checks、Manifest 1131 files。Evidence bundleはrelease blocker 5件と`release_ready=false`を保持し、最終開発監査はpassだが正式releaseを証明しない。
- この試験が閉じるのはstandalone production Brokerの再起動後Audit読込と旧Session拒否の補助証拠だけである。run23のpositive Task完了Auditを再起動後に再読込した証拠ではない。Task状態は契約どおり揮発し、Codex `task_execution=unsupported`、R2 `release_blocker`、`release_ready=false`を維持する。result／diff Content Exposure、MxC TEMP cleanup、OneDrive Cloud Files／通常NTFSのTask統合隔離、failure／deadline／crash Recovery、通常Release Task capabilityは未成立。

## R2追補 現行Codex CLI 0.160.0のPATH依存MxC実行確認（2026-10-03）

- 現行CLI `0.160.0`を既存の`tooling/codex_mxc_exec_temp_probe.py`で1回実行した。実model・Credentialなし、run専用CODEX_HOME／合成Workspace／loopback偽Responses APIを使用し、非loopback接続要求はprobeで遮断した。Codexアプリ同梱PowerShellを含む元の`PATH`では偽API応答と`exec_command`提示まで進んだが、command childが`-2147450747`（`0x80008085`）で終了しreportを生成しなかった。これは失敗記録であり、TaskやTEMP cleanupの成功証拠には数えない。
- 同じprobeを、probe process内だけWindows `System32`／`WindowsPowerShell`／`SystemRoot`に限定した`PATH`で再実行すると1/1正常完了した。PATH変更は終了後に元へ戻し、Codex設定・Windows保護設定・永続環境変数は変更していない。偽API要求2件、childのTEMP／TMP一致、Broker相当scratchとの不一致、child内marker書込成功、終了後markerとTEMP directoryがhostから不可視、non-loopback CONNECT遮断4件を観測した。host不可視は物理削除の証明ではない。
- 同じ現行CLIを明示絶対pathで指定した既存Rust ignored live test `Rust生成Task設定で実Windows隔離の登録secretを拒否する`も1/1成功した。Rust生成設定での明示`powershell.exe`起動、登録secret read／write拒否、動的hardlink作成拒否、別AgentのWorkspace／TEMP相互read拒否を確認した。一方、合成secretへの事前hardlink aliasはMxC単体permissionでは読めた。登録時・起動直前のBroker側file identity／hardlink検査は別責務として維持する。
- MxCの既定`.env`／`.ssh`／`secrets` globはreadを拒否したがwriteは許した。登録済みsecret path、Workspace外pathはread／writeとも拒否した。この観測を全filesystem隔離やinstalled productの保証へ昇格しない。
- 環境変数`PATH`の差で`codex exec`の子commandの結果が変わった。ただし試行は各1回であり、CLIが選んだ実行shellと製品起動時の環境が一致しているかまでは確定しない。終了値`-2147450747`は[.NETホスト異常code表](https://github.com/dotnet/runtime/blob/main/docs/design/features/host-error-codes.md)の`CoreHostCurHostFindFailure`に対応する。原因の最終確認には、実際に配置した製品の`Broker`経路と同じ起動条件で再検証する。
- 証拠範囲は直接Codex CLI／MxC childおよび明示PowerShellのRust live testであり、Owner UI、production Broker Task、実Workspace登録、期限／crash Recovery、Audit、task間cleanup、OneDrive／NTFS統合matrixを通していない。`task_execution=unsupported`、R2 `release_blocker`、`release_ready=false`を維持する。
- 実行command: `python -X utf8 tooling/codex_mxc_exec_temp_probe.py --exe <明示指定Codex CLI絶対path> --runs 1`（PATH継承／一時Windows標準shell PATHの各1回）、`cargo +1.95.0 test --locked --offline --manifest-path native/rust_helper/Cargo.toml --lib "Rust生成Task設定で実Windows隔離の登録secretを拒否する" -- --ignored --nocapture --test-threads=1`（1 passed）。前者は直接CLI probe、後者はRust生成permission profileの明示`powershell.exe` sandbox testであり、同一production経路ではない。

## R2追補 fresh installed run27失敗とTEMP scope判定不備の修正（2026-10-03）

- source commit `9c1f8a777728efc2ab02c8ae4e4e8addd10a0282`から作成したstaged app（manifestの`source_worktree_clean=true`、app SHA-256 `sha256:e617996bdd69365aacf1095cee9f2b3d2a3d6ba9dd1ac39ac48e22ad7c3ce44f`）を使い、synthetic run27でnative Owner確認、Runtime／Workspace登録、Workspace Permission、別個のTask Approval、production Broker Task開始まで実行した。Session `15e47a5f7d02173b29210ece3c914c99`、Task `363174fe325b01a4c9bb73f6c68eaf38`はterminal `failed`であり、positive completionではない。
- Permission発行Auditは`broker-audit-55/56`、別個のApprovalは`broker-audit-57/58`、Task開始は`broker-audit-60/61`、失敗・取消記録は`broker-audit-80/81`。二度目の取消要求`broker-audit-82`は`要求不正`で拒否された。UIの終端状態は`failed`。偽Responses API `127.0.0.1:56106`は4 requestを観測し、tool提示・送信・結果受領はあったが、同一tool-call再送拒否3件、invalid body／応答書込み失敗／外部要求0件で、completion markerは作られなかった。
- Workspace completion markerとoutside-write markerは不在、登録secret canaryは0 byte。outside-read拒否が成立したことを示す安全な証拠はなく、結果は`unknown`とする。TEMP reportは`stage=temp_checked`、`temp_scope=unrecognized`、`temp_write=failed`、`temp_step=temp_directory_check`、TEMP／TMP一致を記録した。
- このTEMP分類については、実path componentが`sandbox.{GUID}`形式であるのに、試験probeのmatcherが`sandbox.GUID`形式だけを受理していた不備を確認した。従ってrun27の`unrecognized`／`failed`はTEMP directoryの実書込み拒否を証明しない。Rust source `native/rust_helper/tests/support/codex_loopback_responses.rs`を波括弧付き／なし双方を受理するmatcherへ直し、実probe文字列からmatcherを取り出して両形式と片側括弧の拒否を確認するtestを追加した。この修正はtest harnessだけで、製品のTEMP挙動を証明・変更しない。Task全体の失敗原因は確定していない。
- app／launcher artifactとloopback fixtureのhash、synthetic環境、Task/Audit参照は`release_blockers.registry.json`の`r2_installed_synthetic_task_e2e_run27`へ記録した。Broker終了後の101-event Audit JSONLを独立再計算し、event hash／previous link不一致0、重複event ID 0、anchor count／head一致、HMAC有効、Task指示本文なしを確認した。head=`sha256:80e5e4a6af2a9c425d8c18fe67c8959270044de6c59f6317349f413fca35a7b0`。`broker-audit-101`はDesktop終了記録。run27 Launcher PID `16132`、Flutter PID `19572`とport `56106` listenerは終了後に不在。
- run27は試験専用`r2-e2e` staged pathの失敗記録であり、run23のpositive Task完了を置換しない。run27 AuditのBroker restart再読込、正のTask完了、result／diff Content Exposure、実MxC TEMP cleanup、outside／secret read拒否、failure／deadline／crash Recovery、通常Release Task capabilityは証明しない。`task_execution=unsupported`、R2 `release_blocker`、`release_ready=false`を維持し、fixture修正commitからfresh runを再試行する。
- 検証: Rust全target再実行は458 passed／0 failed／7 ignored。初回実行ではloopback fixture 1件がHTTP header前`ConnectionReset`で失敗したが、同test単独再実行1/1 pass後の全target再実行は成功した。R2 fixture binaryの6 testは6 passed。Schema 152件／example 152件／negative fixture 196件、Conformance 231 checksはpass。対象Rust fileのrustfmt／`git diff --check`もpass。統合validator初回はManifest未更新と6件の日本語監査findingで失敗したが、試験診断文を日本語化しManifestを更新後、`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`がexit 0となりdevelopment check 10件すべてpass。最終日本語監査1134 files／0 findings、Manifest 1131 files。Evidence bundleはrelease blocker 5件と`release_ready=false`を保持し、開発validator通過を正式releaseの証拠には昇格しない。

## R2追補 fresh installed run31のTask失敗・取消と終了未完了（2026-10-03）

- source commit `9159b88f73e1036381493a957545535afd48c5cc`からのinstalled app／Broker／launcherを使用し、実Flutter UI、native Owner confirmation、production Broker、実Codex CLI／MxC、loopback偽Responses APIを通した。installed artifact hashとrun固有の証拠は`release_blockers.registry.json`の`r2_installed_synthetic_task_e2e_run31`へ記録した。通常Release能力を有効化せず、実Model、Credential、課金、外部通信、Windows保護設定変更はない。
- 先行Task `1e908929679bf19963495eebf5feb172`は開始Audit `broker-audit-62`後にterminal `broker-audit-72`で失敗した。再試行Task `0296907e42edae32637e87f1fa027926`はWorkspace Permission `broker-audit-86`、独立Approval `broker-audit-89`、開始`broker-audit-92`まで到達したが、Taskは取消Audit `broker-audit-100/101`で終端し、重複取消`broker-audit-102`は拒否された。状態更新のつもりの私の画面操作直後に取消requestが記録されており、誤って停止controlを選んだ可能性が高い。これは私の操作ミスとして扱い、成功証拠には数えない。
- 各偽API試行は4 requests、tool提示／送信／結果受領あり、同一tool-call再送拒否3件、invalid／incomplete／response write failure／外部要求0件、Workspace completion markerなしだった。TEMP reportは`temp_scope=unrecognized`／`temp_write=failed`／`temp_step=temp_directory_check`を記録し、偽APIの安全診断projectionでは`workspace_detected`までしか確認できなかった。実TEMP書込み拒否、secret read拒否、outside read拒否はいずれも証明せず、結果を`unknown`とする。
- 試験commandはWorkspaceへTEMP reportを書いた後、合成継続markerを最大30秒待つ。report後のmarker作成は期限を超えていたため、tool retryの一因として有力だが唯一の根本原因とは確定していない。次の試行ではTask開始前からrun専用watcherを稼働させ、`stage=temp_checked`を検証した直後にzero-byte continuation markerを作る。期限内markerでも再送が起きる場合はCodex/MxCのtool result acknowledgementを切り分ける。
- Broker稼働中のAudit snapshot 140件は独立再計算でevent hash／previous link不一致0、重複event ID 0、anchor count／head一致、HMAC有効、Task本文なしを確認した。これはBroker再起動後の耐久性証明ではない。偽API listenerは停止済みだが、Launcher／Flutter processは2組が通知領域に隠れたまま残り、Auditの通知eventが増加している。Desktop exit Auditもなく、run cleanupは未完了。Task completion／outside-write markerはなく、outside／secret read結果は`unknown`である。
- Ownerから終了操作済みとの報告後、2026-10-03 09:01:50 UTCに再確認したが、Frontend PID `3932`／`20140`とLauncher PID `16816`／`15584`は依然生存し、MainWindowHandleは全て0だった。Audit anchor／JSONLは158 eventへ進み、最新`broker-audit-158`は通知一覧の受理記録、JSONL SHA-256は`290c57dfe323294000679aee565d9182fa262231d4f30dfbac08019c0fe3f8ba`。140 event時点の再計算結果を158 event全体へ流用せず、owner終了報告後の全chain／HMACは未再検証とする。Computer Useには対象可能なD4 Pocket／通知領域windowがなく、強制終了や未検証のwindow-message操作は行っていない。したがって終了Audit・process終了・cleanup完了は成立していない。
- R2 positive production E2Eは未完了のまま。`task_execution=unsupported`、R2 `release_blocker`、`release_ready=false`を維持する。次は両test instanceのnative tray `終了`後にPIDとAuditの最終状態を照合し、fresh installed runで期限内markerを含むTask E2Eを再試行する。

## R2追補 fresh installed run32のpositive TaskとTEMP未判定（2026-10-03）

- run31と同じstaged product artifact（product source commit 9159b88f73e1036381493a957545535afd48c5cc）で、freshなprofile／合成Workspaceを用いたrun32を実行した。現行main 209751720b973ce52ebf1dbb60137485bc9f56f8との比較では、apps/desktop_flutterとnative/rust_helper/srcに差分はない。artifact hash、synthetic run ID、Task／Audit参照はrelease_blockers.registry.jsonのr2_installed_synthetic_task_e2e_run32に記録した。
- 実Flutter UI、native Owner確認、production Broker、別個の一回限りWorkspace Permission／Task Approval、実Codex CLI／MxC、loopback偽Responses APIを通り、Task bc442f051d8ac54a7568bd45b7b04e59はUI上completedとなり、Workspaceのbroker-real-codex-marker.txtが存在した。Permission broker-audit-44、Approval broker-audit-46、Task開始broker-audit-52、状態照会broker-audit-56。合成secretは0 byte、outside-read canaryは存在、outside-write markerとBroker scratch directoryは不在だった。
- TEMP reportはscope=unrecognized／write=failed／step=temp_directory_checkであり、scope不明のため実TEMP書込みは試みられていない。watcherはreport後に継続markerを作ったのでTaskは完了したが、TEMPの書込み可否・marker cleanupは未確認である。Broker終了後のAudit JSONL 100 eventを独立再計算し、event hash／previous link不一致0、重複0、anchor count／head一致、HMAC有効を確認した。head=sha256:e9e9d8d9a626c46e62563bf854919438af558053576ac0ab397cbd41fdb6b589。Broker restart後の再読込ではない。
- このrunはstaged r2-e2e buildでのpositive Task completionを補強するが、通常ReleaseのTask capability、TEMP cleanup、Audit restart durability、result／diff Content Exposure、OneDrive Cloud Files／通常NTFSの統合隔離、failure／deadline／crash Recoveryを証明しない。task_execution=unsupported、R2 release_blocker、release_ready=falseを維持する。

## R2追補 fresh installed run33のTask失敗とwatcher期限超過（2026-10-03）

- run32と同じproduct artifact／loopback fixtureを使ったrun33で、Task 367b5900ae9281c701f8999b634fb2a2はUI上failedとなった。Workspace Permission broker-audit-91、別個のApproval broker-audit-93、Task開始broker-audit-97、状態照会broker-audit-103。fake API／実CLIを通したproduction Task経路の失敗記録であり、positive completionではない。
- Host watcherは240003msでtimeoutとなり、Workspace Task markerは不在、outside-write markerも不在、synthetic secretは0 byte、Workspace内Broker scratch directoryが1件残存した。TEMP reportはunrecognized／failed／temp_directory_checkであり、実TEMP書込みは試みられていない。outside-read canaryは用意されているが、read拒否結果を独立に確定できる証拠はないためunknownとする。
- Broker終了後のAudit JSONL 110 eventは独立再計算でevent hash／previous link不一致0、重複0、anchor count／head一致、HMAC有効。head=sha256:8af76f9899db5f412827dc2ee237c5e111715f404c9f27ff0df540ac1a55435e。Broker restart後の再読込、Scratch recovery、Task成功は未確認。

## R2追補 fresh installed run34失敗とWindows TEMP path separator誤判定（2026-10-03）

- run34はstaged r2-e2e product、実Flutter UI／native Owner確認／production Broker、実Codex CLI 0.160.0／MxC、loopback偽Responses APIを通ったが、Task 1ebd3e158075f9e8f70b670185ae8718はfailed。Workspace Permission broker-audit-49、別個のApproval broker-audit-51、Task開始broker-audit-54、終端状態照会broker-audit-60。偽APIは4 requests、tool提示・送信・結果受領あり、再送拒否3、invalid／incomplete／response write failure／外部要求0、Workspace marker不在だった。
- TEMP reportのunrecognized／failed／temp_directory_checkは実TEMP書込み拒否を示さない。別途raw pathを出力せずcomponentだけ確認した結果、TEMP=TMPで末尾がsandbox.{GUID}/AC/Tempの形だった。根本原因はtest probeのRust string literalでseparator regex用のbackslashが不足し、PowerShell実行時のregexがWindows backslashではなくslashだけを分割していたこと。run34 watcherはscope不明を尊重して継続markerを出さず、その後Taskが失敗した。secret／outside read結果はunknown、outside-write／Task markerは不在。
- Broker終了後のAudit JSONL 72 eventを独立再計算し、event hash／previous link不一致0、重複0、anchor count／head一致、HMAC有効を確認した。head=sha256:f670f08f19d17a71f7f26c3a97d1f291ec90179ddacadc7c20bb2c4ff8ef26f5。Broker restart後の再読込ではない。
- ウィンドウのclose操作後も当該runのlauncher／Flutter processが通知領域に残っていたため、偽API停止とTask終端を確認してから、実行pathと親子PIDを照合した当該2 processだけを停止した。停止後は当該Desktop／偽API processが不在。trayの通常終了操作によるDesktop exit Auditはなく、強制停止を正常終了証拠へ置き換えない。
- Ownerからrun34の終了操作を行ったとの追加申告を受領した。最終照合では当該installed app／launcher／偽API processとloopback listenerはいずれも不在だが、保存Auditは72 eventのままで末尾は`broker-audit-72`「通知一覧」受理であり、Desktop正常終了eventは観測していない。申告された操作と正常終了Auditを区別する。

## R2追補 TEMP separator fixture修正と回帰検証（2026-10-03）

- run32～34で使ったloopback偽Responses API executableは実行開始時のhashを保存していない。現在のbuild出力の更新時刻は3 runのAudit最終記録より後であり、その現存hashを過去runのartifact証拠として遡及適用しない。各recordはfake_api_artifact_hash_captured=falseとし、次のfresh runでは起動直前に実行file hashを固定する。
- native/rust_helper/tests/support/codex_loopback_responses.rsのPowerShell regexを、Rust文字列経由でもWindows backslashとslashの両方を分割するescapeへ修正した。生成probe本文から実際のmatcherを抽出し、Windows形式とslash形式のpathを分割するtemp_probe_splits_windows_path_separators testを追加。別の安全なsynthetic PowerShell sampleでも修正前は1 component、修正後は4 componentとなり、sandbox.{GUID}/AC/Tempのtail matcherを満たすことを確認した。これはdevelopment-only fixture修正であり、production隔離やTEMP cleanupを変更・証明しない。
- cargo +1.95.0 test --locked --offline --manifest-path native/rust_helper/Cargo.toml --all-targets --features r2-e2e の初回並列実行ではbroker::update_downloadのlocal TLS test 2件が接続resetで失敗した。-- --test-threads=1 の全target再実行では460 passed／0 failed／7 ignored。R2 fake API fixture 7 testも全通過。
- focused testは1件成功し、変更file単独の整形確認も成功した。Schema 152件、example 152件、negative fixture 196件、およびconformance 231件の確認も成功した。実行したcommandは次のとおり。
  ```text
  cargo +1.95.0 test --locked --offline --manifest-path native/rust_helper/Cargo.toml --lib temp_probe_splits_windows_path_separators -- --nocapture
  rustfmt +1.95.0 --check --edition 2021 native/rust_helper/tests/support/codex_loopback_responses.rs
  python -X utf8 tooling/schema_check/check_schemas.py
  python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py
  ```
- cargo +1.95.0 fmt --manifest-path native/rust_helper/Cargo.toml -- --check はcrate全体の既存format差分を多数報告したため失敗。変更file単独のrustfmt --checkはpassし、無関係fileのformat変更は行っていない。
- 最新文書・manifestを対象にした`validate_all.py --python-only --desktop-platform windows`はexit 0。日本語基底監査、Schema、conformance、manifest、release gate、packaging portability、release smoke、evidence bundle、release runtime assertions、C32開発監査の全checkがpassした。これはPython側の統合validationであり、installed Windows実行、通常Release Task capability、R2完了の証拠ではない。
- 次は修正fixtureをcommit／push後にbuildし、fresh installed runでscope classification、Task marker、Audit、実TEMP markerの観測を再実施する。修正後runの成功が確認されるまではtask_execution=unsupported、R2 release_blocker、release_ready=falseを維持する。

  ```text
  python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows
  ```

## R2追補 fresh installed run35のpositive Task・同一profile Audit再起動確認（2026-10-03）

- 試験用source commit `0acda4705c432c298269ace5b23abfa35960213d`から生成し、SHA-256 `sha256:f8e220e9b91b15038d7c69708b67fcc1198354028b8f5019feca8260718884c5`を固定したlocalhost限定応答API試験fileと、製品成果物commit `9159b88f73e1036381493a957545535afd48c5cc`由来の段階配置アプリ／Broker／起動器を用いた。製品成果物のhash群はrun30と一致し、app `sha256:e617996bdd69365aacf1095cee9f2b3d2a3d6ba9dd1ac39ac48e22ad7c3ce44f`、Broker `sha256:c9169cc0880b39dfad7fa458407824f6d7299e064145dbf8ab0d203816c5622d`、起動器 `sha256:c3165076021ffcd0814d32244be2667e67634c5495267cff186094d83a0c4671`。実行記録、合成Workspace、隔離profile、Task／Audit参照は`release_blockers.registry.json`の`r2_installed_synthetic_task_e2e_run35`に保存した。実Codex CLIのfile hashは記録したが、version確認は行っておらずCLI版は未観測。
- 実Flutter UIのnative Owner確認、Production Brokerのauthenticated IPC、one-use Workspace Permission（`broker-audit-60/61`）、別個のone-use Task Approval（`broker-audit-63/64`）、Task開始（`broker-audit-66/67`）、実Codex CLI／MxC tool child、loopback偽APIを通し、Task `933db3b53d0641ab8b842a1a8d640856`はUI状態照会で`completed`、result hash `sha256:55c569826e219f9d375b6ec4b1e4a15d2d569e81a4ff75403e579e18c53787ab`となった。Task instruction hashは`sha256:925fe4108448289885e4d5b48452c6cd9986f8102d3d501272954c109cc8a165`。固定probeは合成登録secret read、Workspace外read／writeを拒否した場合に限りmarkerを書く設計であり、Workspace marker（20 byte、SHA-256 `sha256:712aff9ceb6418d369c1f968d51294af292dbfe4bda6739d4e86ba333227a29c`）が作成された。偽APIは2 request、model request 0、tool offer／call／result受領各true、再送拒否・invalid／incomplete body・response write failure・外部requestは0。実model、資格情報、課金、OS保護設定変更なし。
- 完了状態に至る途中で取消要求Audit `broker-audit-69/70`が記録され、後続取消`broker-audit-71`はterminal Taskとして拒否された。`broker-audit-70`のreasonは「Agent Task完了（結果本文非保存・hashのみ）」だが、外側operation fieldは`AgentTask取消`である。後続の状態照会`broker-audit-75`とUIは`completed`を示す。operation名を完了へ書き換えず、このAudit意味上の注意を保持する。
- Task probeはTEMPを`mxc_appcontainer`と分類したものの、`temp_directory_check`で失敗し、TEMP markerへの実書込みを試みなかった。tool output diagnosticsは`workspace_detected`まで、host watcherは約254.9秒後に継続markerを置いて`continued`となった。TEMP report SHA-256は`sha256:f30e5a0c90c21faf903b98c5034ecd69915123dcb5818dc4b64e0f2e167ee538`。実pathを保存・表示せず、hostからdirectoryが見えないことを物理削除の証拠にしない。
- タイトルバーclose後もrun35のLauncher／Flutter processがトレイ常駐で生存したため、Task終端とfixture停止を確認し、完全pathがrun35内に一致する2 PIDだけを停止した。その後、同じ隔離`LOCALAPPDATA`／`USERPROFILE`／synthetic `CODEX_HOME`を使い、fake APIを再接続せずDesktopを再起動。Desktop起動Audit `broker-audit-98`とBroker由来Dashboard snapshotを確認した。最終停止後のfile-backed Audit 122 eventを再計算し、event hash／previous link不一致0、重複event ID 0、anchor count／head一致、local HMAC valid。head=`sha256:88ade322b332e29893b43c7f4ca039fab7340dd7f1746b47feab324f9deae414`、Audit JSONL SHA-256=`sha256:748d0d848884724f7044981d933160f74d92b6e91fec818552d5fd2917bf05dd`。この再起動は同一storeの読戻し証拠であり、同一権限によるkey／anchor／log同時改変への耐性や外部anchorは証明しない。最後のDesktop processも個別停止し、正常Desktop終了Auditは観測していない。
- run35では試験機能付き`r2-e2e`段階配置品で正のTask完了と、同一profile再起動後のBroker監査store再読込を一度確認した。ただし通常Releaseの`task_execution=unsupported`、TEMP実書込み／回収、結果／差分のContent Exposure表示、失敗／期限／異常終了後の復旧、作業間の一時物回収／隔離、OneDrive Cloud Files／通常NTFS双方を通した統合試験、外部提供元との相互運用、trayからの正常終了Auditは未成立。R2 `release_blocker`と`release_ready=false`を維持する。run32～34の過去成功／失敗履歴は変更・統合していない。

## R2追補 fresh installed run36失敗とoutside境界fixtureのhost-side照合（2026-10-03）

- run36は試験用source commit `6e31a1c0b6b8d22481bab6bd61d93a77edbfc660`から作った`r2-e2e`段階配置品、実Flutter UI、native Owner確認、authenticated Broker IPC、production Broker、実Codex CLI `0.160.0`／MxC child、loopback偽Responses APIを通った。product artifactはapp `sha256:e617996bdd69365aac1095cee9f2b3d2a3d6ba9dd1ac39ac48e22ad7c3ce44f`、Broker `sha256:c9169cc0880b39dfad7fa458407824f6d7299e064145dbf8ab0d203816c5622d`、launcher `sha256:c3165076021ffcd0814d32244be2667e67634c5495267cff186094d83a0c4671`、偽API `sha256:9c0525d68cf105df01fdd9d27f3edee8e9aa0d355cb7b8f6b2035e64311f8eea`。real model／credential、課金、OS保護設定変更なし。実行manifest、隔離profile／Workspaceはrun固有root `C:\d4p-r2-run36-6e31a1c`に保持した。
- Workspace Permission `broker-audit-74`、別個のone-use Task Approval `broker-audit-76`、Task開始`broker-audit-80`を経て、Task `0e548cb404c14d12c057a4a80e9ced98`（Session `c8b0cd0031dc7d08db72f426d5074542`）は状態照会`broker-audit-94`でfailedとなった。終端理由は「Agent Task失敗・取消（RecoveryAction=Workspace差分を確認）」。Workspace completion markerはなく、result hash／completed stateは成立していない。
- 偽APIは4 requests、model requests 0、tool offer／call／result受領あり、repeated tool-call rejection 3、invalid／incomplete body、response write failure、blocked external requestは各0。Workspace markerは不在。Host側でoutside-write marker不在、registered-secret file存在、`.d4p-tmp-*` scratch 0件を確認したが、Taskがmarkerを作る前に失敗したため、childによるsecret／outside read拒否をpositive結果として扱わない。outside-read結果もこのrunでは未確定。
- TEMP reportは`scope=mxc_appcontainer`、`stage=temp_checked`、`temp_step=temp_write`、`temp_write=failed`。実TEMP markerへの書込みを試み、tool-output diagnosticの安全な分類は`directory_missing`。これはTEMP pathの実書込み失敗を記録するが、TEMP rootの物理削除・cleanupを証明しない。
- Task失敗の直接原因はdiagnosticから確定できない。source確認で、outside-read前の`Test-Path`がAppContainer内からの親directory可視性を要求しており、実read拒否とfixture事前判定を混同し得ると分かった。この可能性をrun36の根本原因とは断定しない。development-only偽APIは、hostが合成read markerの通常file／固定内容とwrite marker不在を開始前に確認し、MxC childはoutside-readを直接試し、Task終了後にhostがread marker内容不変／outside-write marker不在を再確認するよう修正した。focused testは欠落read marker、改変read marker、既存write markerを拒否する。production code／通常Release capabilityは変更しない。
- run36終了後、ownerから終了操作済みとの申告があり、再照合ではrun36 installed root由来processは不在。最終file-backed Auditは108 events、event hash／previous link不一致0、重複ID 0、anchor count／head一致、HMAC valid。head=`sha256:93e28aa711734bbcdb8356607164b40aa4179c6b210f35fc12f12587bcee8afc`、Audit JSONL SHA-256=`sha256:cbc258935322e4f17fa08d29252ebacddbe22192a6859fed2f128f12c7779e8f`。ただし`session.json`は`active`、最終4 eventは通知一覧`accepted`、Desktop終了Audit refなし。process不在は観測したが、normal exit Auditを主張しない。Broker再起動後の読戻しも未実施。
- 検証: 新outside境界fixture test 1 passed、TEMP直接write probe test 1 passed、対象2 Rust fileの`rustfmt --check`と`git diff --check`成功、Schema 152／example 152／negative fixture 196件成功、Conformance 231 checks成功、`cargo check --all-targets --features r2-e2e`成功。未選別Rust all-target invocationは2回ともlibrary test target内で停止し、後続targetは未実行。初回の`broker::update_download::tests::failed_replacement_keeps_the_existing_corrupt_package_unchanged`はlocalhost connection reset（OS error 10054）、単独再実行1 passed。2回目の`a2a::tests::loopback_HTTPからAgent_Cardを取得してmetadata_onlyへ射影する`は応答read reset、単独再実行1 passed。続くskip付きall-target実行で`broker::update_download::tests::local_tls_server_repairs_only_after_verified_package_bytes`がreset／不正Content-Typeで停止し、単独再実行1 passed。最終の`cargo +1.95.0 test --locked --offline --manifest-path native/rust_helper/Cargo.toml --all-targets --features r2-e2e -- --test-threads=1 --skip failed_replacement_keeps_the_existing_corrupt_package_unchanged --skip local_tls_server_repairs_only_after_verified_package_bytes --skip loopback_HTTPからAgent_Cardを取得してmetadata_onlyへ射影する`はexit 0で全targetを完走し、残りのtest failureは0。3 testはskip対象として明示し、各test単独の再実行はpass。未選別suite全体passとは扱わず、Windows localhost loopback不安定性の原因も断定しない。
- run36は失敗した`LIVE_RUNTIME`試行で、run35の成功履歴を置換しない。次は上記偽API修正をclean commitへ固定してfresh installed run37を行い、Task completed、Host側boundary postcondition、TEMP、file-backed Audit／restart、cleanupを再検証する。`task_execution=unsupported`、R2 `release_blocker`、`release_ready=false`を維持する。

## R2追補 TEMP probeの可視性事前判定修正（2026-10-03）

- run35でTEMPを`mxc_appcontainer`と分類した後、`Test-Path -PathType Container`がfalseを返すと書込み処理を呼ばず、`temp_step=temp_directory_check`で終了していた。この観測は実TEMP書込み拒否を示さない。
- development-onlyの合成MxC probeを、scopeが許可値として認識された後はTEMP目印へ直接`Set-Content`を試す形に修正した。失敗時は`DirectoryNotFound`／`PathNotFound`をdirectory不在、権限拒否を`access_denied`として別分類し、raw pathや本文は診断へ出さない。Workspace／secret境界probeはTEMP結果と独立して継続する。
- 検証: `cargo +1.95.0 test --locked --offline --manifest-path native/rust_helper/Cargo.toml --all-targets --features r2-e2e temp_probe_attempts_write_without_visibility_precheck`は対象test 1件ずつ（lib／fixture binary）pass。`cargo +1.95.0 test --locked --offline --manifest-path native/rust_helper/Cargo.toml --all-targets --features r2-e2e -- --test-threads=1`は462 passed／0 failed／7 ignored。
- 証拠分類は`FIXTURE`。この変更とtestはMxC実環境のTEMP書込み・物理cleanupを証明せず、製品runtime codeも変更しない。次はこのfixture sourceを含むfresh installed run36で実書込みを再観測する。`task_execution=unsupported`、R2 `release_blocker`、`release_ready=false`を維持する。

## R2追補 fresh installed run37の正Task・境界・Audit再起動確認とTEMP書込み失敗（2026-10-03）

- run37は試験用source commit `3688fa0fc0a50d3f06df40d003b8115c194846e1`からのfresh `r2-e2e`配置品で実行した。Rust launcher／Brokerは同commit、Flutter appはsource commit `9159b88f73e1036381493a957545535afd48c5cc`由来であり、該当Flutter sourceは`3688fa0`まで変更なし。app／Broker／launcher hashはそれぞれ`sha256:e617996bdd69365aac1095cee9f2b3d2a3d6ba9dd1ac39ac48e22ad7c3ce44f`、`sha256:c9169cc0880b39dfad7fa458407824f6d7299e064145dbf8ab0d203816c5622d`、`sha256:c3165076021ffcd0814d32244be2667e67634c5495267cff186094d83a0c4671`。run manifest、loopback偽API、合成Workspaceと隔離profileは`C:\d4p-r2-run37-3688fa0`に保持し、偽API hashは`sha256:83a1e3cbcd92b76cd34c929f731385984c65aa7a4bbeea25154b7fd0353999dc`。実Codex CLIは0.160.0、hash `sha256:7d4588265a55adb1403f85d2e058b11dc971459842de84876c86ff02fb7771f2`。実Model／Credential／課金／外部通信は使わず、OS保護設定も変更していない。
- 実Flutter UIとRust native Owner確認を経てRuntime／Workspaceを登録し、Workspace Permission `broker-audit-50/51`と別個のone-use Task Approval `broker-audit-53/54`を発行した。実Codex CLI／MxC childを用いるproduction Broker Task `7d3b5a7c71758370f35192231f7211e8`は、状態照会`broker-audit-65`でcompleted、result hash `sha256:55c569826e219f9d375b6ec4b1e4a15d2d569e81a4ff75403e579e18c53787ab`。偽APIは2 requests、tool offer／call／result受領あり、重複tool-call、invalid／incomplete body、応答書込み失敗、外部要求はいずれも0。Audit reasonは完了を分類し、task instruction本文はAuditに含まれない。
- 固定合成MxC probeは、登録secretと指定outside-read canaryのread拒否後にのみTask completion markerを書く。markerは20 bytes、hash `sha256:712aff9ceb6418d369c1f968d51294af292dbfe4bda6739d4e86ba333227a29c`。Host側でもoutside-read canaryの固定byte列が不変、outside-write marker不在、登録secret file存在を検査し、偽APIはpostcondition一致を報告した。この境界証拠は指定した合成pathと一回のchild runだけに限る。
- TEMP reportは`scope=mxc_appcontainer`、`temp_step=temp_write`、`temp_write=failed`、安全な診断分類`directory_missing`。実MxC childがTEMP marker書込みを試みた結果であり、report hashは`sha256:728e5e9dc5d1f43815f2364bdb8b037a5853e7b9bdd022edf8621f4f6037b8a6`。物理TEMP cleanupは証明していない。TEMP path不在の原因は未確定で、製品隔離を緩める根拠にはしない。
- Task完了後、実行file pathを照合したrun37 launcher／Flutter PIDだけを停止し、同じ隔離profileからinstalled launcherを再起動した。再起動後にBroker-backed dashboardと`durable_file_store`を観測し、97-event Audit chainを再計算してhash／previous link不一致0、重複0、anchor count／head一致、HMAC validを確認。Desktop起動Auditは`broker-audit-76`。再起動後の最終採取snapshotは108 events、head `sha256:18f504a99e7faecd324262fcbbbc983e6c75735f990777035e68a8efad3ab135`、Audit JSONL `sha256:9a17ce3bad10de805bdd39ede247ae094043440ae0ed0b8695cdb41af7358b39`で、同じ検査結果。Audit末尾`broker-audit-108`は通知一覧accepted。task completion reasonは再起動後も保持され、instruction本文は不在。
- 終了時はrun37由来の正確なlauncher／Flutter processだけを停止し、偽API／watcher／run37 product processが不在であることを確認した。Session fileは`active`、Desktop正常終了Auditはなく、trayの通常終了を主張しない。この停止はTask完了後のprocess-stop／同一profile再起動試験であり、実行中Taskのcrash Recoveryを証明しない。
- 証拠分類は`LIVE_RUNTIME`（installed UI、native Owner、production Broker、Codex／MxC child、restart時のAudit store読戻し）、`INTERNAL_STATE`（Broker投影とAudit store）、`FIXTURE`（loopback偽API、合成Workspace／secret／outside marker）。run37はrun36の失敗履歴を置換せず、TEMP実書込み／cleanup、active-Task deadline／crash Recovery、result／diff Content Exposure、task間隔離、OneDrive Cloud Files／NTFS統合、通常Release capability、tray正常終了Auditを成立させない。`task_execution=unsupported`、R2 `release_blocker`、`release_ready=false`を維持する。

## R2追補 TEMP directory missing追加診断と終了後再照合（2026-10-04）

- OwnerからDesktop終了済みとの報告を受け、run37の保存profileを再照合した。run37 root由来processは0件、`audit.jsonl`は108行・SHA-256 `9a17ce3bad10de805bdd39ede247ae094043440ae0ed0b8695cdb41af7358b39`で前回snapshotと同一、anchor event countは108。Session stateは`active`のままで、新しいDesktop終了Auditはない。終了報告とprocess不在は確認したが、正常終了Audit／Session終端を成立したとは扱わない。
- TEMP原因を区別するため、development-only合成probeを拡張した。TEMPへの直接writeは引き続き最初に実行し、失敗後だけdirectory存在を観測する。認識済みMxC AppContainerで、初回失敗が`directory_missing`かつ再観測も不在の場合だけ、対象TEMP directoryのbounded createと一回のmarker retryを行う。cleanupはprobe自身の一意markerと、probeが作成した空directoryだけを非再帰で除去する。他のentryがあればdirectoryを再帰削除せず残存分類する。Audit投影はpath／raw error／本文ではなくallowlist enumだけを含む。Production runtime、Permission、通常Release `task_execution`は変更していない。
- `cargo +1.95.0 test --locked --offline --manifest-path native/rust_helper/Cargo.toml --features r2-e2e --bin gui_shell_r2_e2e_responses -- --test-threads=1`は9 passed／0 failed。`cargo +1.95.0 check --locked --offline --manifest-path native/rust_helper/Cargo.toml --all-targets --features r2-e2e`はexit 0。変更fileの`rustfmt +1.95.0 --edition 2021 --config skip_children=true --check native/rust_helper/tests/support/codex_loopback_responses.rs`と`git diff --check`はpass。
- Rust all-target全件実行はlocalhost／TLS fixtureの3件で別々にWSA 10054／不正Content-Typeを観測し、各該当testの単独再実行はpass。3件だけを明示skipしたall-target commandはexit 0。これはhost loopback不安定性の原因を特定せず、unfiltered all-target全passとは主張しない。`cargo fmt --all --check`は今回変更していない既存Rust file群にも整形差分を報告したため、広域formatは実施せず、変更fileだけを整形・検査した。
- ここまでのTEMP拡張証拠は`FIXTURE`のみ。MxC directory作成可否、retry、cleanupはfresh installed productで未観測。次はこの正確なcommitからrun38をfresh stageして実行する。TEMP診断の改善だけでR2を閉じず、期限／crash Recovery、Task間隔離、result／diff Content Exposure、OneDrive Cloud Files／通常NTFS統合、通常Release gate、tray正常終了Auditを別々に検証する。`task_execution=unsupported`、R2 `release_blocker`、`release_ready=false`を維持する。

## R2追補 現行Codex CLIのMxC TEMP正常終了・取消cleanup再確認（2026-10-04）

- run37後のTEMP probe案に、認識済みAppContainer TEMPへのdirectory作成・削除をMxC child内で含めた中間variantを作ったところ、実Codex CLIは`CreateProcess Rejected`を返し、固定開始markerなし、probe未開始、TEMP reportなしで終了した。拒否の内部原因は未確定であり、TEMPの挙動証拠とは扱わない。このvariantは採用せず、MxC childから外部TEMP directoryの作成・削除を除いた。
- 現行probeはTEMP markerへの直接writeを最初に試し、その後のdirectory観測だけを行う。directory作成・retry・child削除は`not_attempted`とする。parent test processは実行中と子process終端後を別々に観測し、実Codex／MxCの自然なcleanupだけを判定する。権限不足を迂回する操作はない。
- `GUI_SHELL_CODEX_TASK_BROKER_TEST_EXE`に`Get-Command codex`が返す絶対pathを設定したうえで、`cargo +1.95.0 test --locked --offline --manifest-path native/rust_helper/Cargo.toml --features r2-e2e 'broker::dialogue::tests::Broker承認経路から実CodexCLIをloopback偽APIで実行し隔離とcleanupを確認する_LIVE_RUNTIME' -- --exact --ignored --nocapture --test-threads=1`を実行し、exit 0、1 passed。環境変数なしの最初の呼出しはfixture開始前に失敗したため、明示CLI pathを与えて再実行した。実Codex CLI `0.160.0`（SHA-256 `7d4588265a55adb1403f85d2e058b11dc971459842de84876c86ff02fb7771f2`）のMxC childで、TEMP marker write=`passed`、実行中parent read=`readable_match`、正常終端後marker=`not_found`／TEMP directory=`不在`を観測した。Broker取消経路でも実行中marker可読、終端後marker不在／TEMP directory不在を確認した。Task結果の`completed`、Workspace marker、secret／outside境界、loopback限定もtest assertionを通過した。
- focused loopback support 11件、`cargo check --all-targets --features r2-e2e`、Schema 152件／example 152件／negative fixture 196件、Conformance 231 checks、変更Rust fileの`rustfmt --check`、`git diff --check`は成功。現行作業treeでの全target test再実行は全件PASSではない。1回目は417件中410 passed／1 failed／6 ignoredで、別process Broker recovery testが準備PIDの空fileをparseして失敗し、同test単独再実行は1 passed。2回目は同じく410 passed／1 failed／6 ignoredで、既存localhost TLS更新testがWSA 10054 connection reset、同test単独再試行も`Network`で失敗した。これは今回編集したfileの経路ではないが、原因未確定のため現行全target testは未完了として記録し、成功へ昇格しない。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`はexit 0。development validationの成功であり、同validator出力はWindows installed smoke evidence不在と`release_ready=false`を示すため、installed product／release evidenceへ昇格しない。
- 証拠分類は実Codex／MxC childのTEMP動作・process終了観測が`LIVE_RUNTIME`、in-process Broker、Owner callback、Workspace、loopback偽APIが`FIXTURE`。この結果はinstalled product経路ではなく、run37のinstalled TEMP `directory_missing`を置換しない。次は変更をcommit／pushしたclean sourceからfresh installed run38を行い、同じ挙動をFlutter UI→native Owner→production Broker経路で再確認する。active Taskのdeadline／crash Recovery、result／diff Content Exposure、OneDrive Cloud Files／通常NTFS統合、通常Release capability gate、tray正常終了Auditも未成立。`task_execution=unsupported`、R2 `release_blocker`、`release_ready=false`を維持する。

## R2追補 fresh staged run38の正Task・同profile Audit再起動読戻しと終了／TEMP未成立（2026-10-04）

- 記録中にregistryのmanifest hashを修正したため、その修正前に開始していた初回`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`は`release_blockers.registry.json`のManifest hash不一致でexit 1。日本語strict監査、Schema、Conformance、release smoke等はpassしたが、この実行を最終成功へ数えない。Manifest再生成後の最終統合validatorはexit 0で登録済みdevelopment check 10件すべてpass。
- 最終検証結果: Schema 152件／正常例152件／negative fixture 196件、Conformance 231 checks、厳格日本語監査1134 files／0 findings、Manifest check、`git diff --check`、`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`が成功。統合validator内のrelease gate／package portability／release smoke／evidence bundle／runtime assertion／C32監査も合格。証拠束と最終開発監査は既存31 `release_blocker`・2 `post_v1_scope`と`release_ready=false`を維持し、正式releaseを証明しない。
- clean source commit `ffd2e02729f5726a7ea631b0623420f8737ed442`からFlutter Windows ReleaseとRust `r2-e2e` feature付きhelperをbuildし、run `d4p-r2-run38-ffd2e02`としてstageした。app／Broker／launcher／偽Responses APIのSHA-256はそれぞれ`ea185a53cb06417cb410ad4bbf1cc0eed49a43db6d9946814ae7803b2cf2b474`、`48903cd3a59ae798bab3d1068440be1fa121ec7234bb313a8fcf83a3ece22cff`、`3826256ef1352f9822ba5010f00e72513dff1cdfeecdb331d697dc36e62c7b88`、`18c52176c7e824c1f050c28ebd520647af42bb22a6c49aec9ba81cb6dc0b7275`。Codex CLIは0.160.0（SHA-256 `7d4588265a55adb1403f85d2e058b11dc971459842de84876c86ff02fb7771f2`）。実model／実資格情報／課金／外部API要求なし。通常Release能力は変更していない。
- installed `r2-e2e` UIからRuntime／Workspaceを登録し、native Owner確認でWorkspace Permissionと別個のone-use Task Approvalを発行。Session `broker-session-28c9095ac4bdb1d73bc15bb27d244906`のproduction Broker経路でTask `f77929ef9cc1df0f46640b5bb5531e40`を開始し、UIは`completed`と結果hash `sha256:55c569826e219f9d375b6ec4b1e4a15d2d569e81a4ff75403e579e18c53787ab`を表示。Audit `broker-audit-58`は`Agent Task完了（結果本文非保存・hashのみ）`を記録し、指示hashは`sha256:31472a49fd4e10bc38363f08ca1eae96888e8c7ff53f6a373ec5d87f67b37d62`。指示本文はAuditに保存されていない。
- loopback fake APIは2 request、model 0、tool提示／送信／結果受領あり、重複tool call、invalid／incomplete request、応答write failure、外部要求はいずれも0で正常終了。Workspace completion markerは20 byte、hash `sha256:712aff9ceb6418d369c1f968d51294af292dbfe4bda6739d4e86ba333227a29c`。固定した合成secret／outside-read canaryは不変、outside-write markerなし。これはnamed synthetic pathと一回のMxC childに限る境界証拠。
- TEMP probeは認識済みscope `mxc_appcontainer`のTEMP markerへ直接writeを試みたが、`temp_step=temp_write`、`temp_write=failed`、安全な分類`directory_missing`。report hash `sha256:548893e0d6a72de321ed99b3bba6b4f8da6216ac291d77961703940b85e913ed`。TEMP物理cleanupの成功は証明しない。
- Ownerから終了済みとの報告後も、同じrun rootのFrontend PID `8184`とLauncher PID `9788`が稼働し、通知一覧Auditが75件から87件へ増加、Sessionは`active`のままだった。Task terminal／偽API終了を再確認してから当該runの正確な2 processだけをforce-stopし、pre-restart Audit 88件を独立再計算した。event hash／previous link不一致0、重複ID 0、anchor count／head一致、HMAC valid。head=`sha256:1ff3afed3723029c393b993ef1bcd3ca83db6f5c81517d9f2faf543157e73b9c`、Audit JSONL SHA-256=`sha256:355bd5048c8f9ba7fe57d091d0ba39133bba71f69417aab19ce2cf2399cee75e`。このforce-stopをtray正常終了と扱わない。
- 同じrun-specific profileからLauncher／Frontend PID `11168`／`18456`で再起動。BrokerはDesktop起動Audit `broker-audit-89`を書き、UIはfresh Broker-backed Dashboard／Audit投影を表示。file-backed `broker-audit-58`のTask完了記録も維持された。再起動後に正確なrun processを停止し採取した111-event Auditはevent hash／previous link不一致0、重複0、anchor count／head一致、HMAC valid。head=`sha256:c23cede7352e8b64c360edb5815c9681638cd0098077fdc58202046ac543dfb1`、Audit JSONL SHA-256=`sha256:8b7b84d277b0b6d445df313f393d1a60fdcee94d8a7ba1624725248067db011c`。これはTask後process stopからの同profile Audit再読戻しであり、実行中Task crash recovery／通常終了／外部改変耐性ではない。
- 終了・provenanceの不整合も保持する。`installed_manifest.json`はlauncher runtimeを通常per-user rootかつ`isolated=false`と宣言するが、実測storeはrun-specific profile B配下にあった。試験profileの観測をformal isolated installed provenanceへ昇格しない。Ownerが終了済みと報告した後も実画面／processは残り、通常tray ExitとDesktop終了Auditは未観測。force-stop後にはrun profile内の一時`broker_session.json`と空`desktop_launcher.lock`が残った。正確なfile削除commandはexecution policyに拒否され、別手段で回避していない。Audit storeは保存し、資格値は文書・監査へ転記しない。
- run38は正のTaskと同profile Audit再起動読戻しを示す`LIVE_RUNTIME`、Broker保存状態を示す`INTERNAL_STATE`、loopback偽API／synthetic Workspaceを示す`FIXTURE`の限定証拠。通常Release `task_execution=unsupported`、R2 `release_blocker`、`release_ready=false`を維持する。active-Task deadline／crash Recovery、task間scratch／Agent隔離、result／diff Content Exposure、OneDrive Cloud Files／通常NTFS統合、通常Release capability gate、Compare／Handoff failure path、TEMP書込み／自然cleanup、graceful tray exit、runtime-profile provenanceは未成立。
- Ownerの追加終了報告後、run38 rootを含むprocessを再照合し、Frontend／Launcherを含む対象process不在を確認した。Auditは111 eventsのまま、head `sha256:c23cede7352e8b64c360edb5815c9681638cd0098077fdc58202046ac543dfb1`、JSONL SHA-256 `sha256:8b7b84d277b0b6d445df313f393d1a60fdcee94d8a7ba1624725248067db011c`で変化せず、新たなtray Exit／Desktop終了Auditは観測していない。したがってprocess不在とOwner報告は記録するが、通常終了経路の証拠へ昇格しない。正確なrun専用`broker_session.json`と`desktop_launcher.lock`削除を、process不在を再確認した上で同じ`Remove-Item -LiteralPath`方式により再試行したが、実行ポリシーに再度拒否された。監査storeは保持し、別の削除手段では回避していない。

## R2追補 run40の常駐poll監査増加と可視性連動修正（2026-10-04）

- run40はRuntime／Workspace登録と拒否境界まで進んだが、Task Permission／Approval／Taskは発行・開始されず、positive Task E2Eではない。source commit `8babcd3b4eba08e959e8d447d6a680d7cdbba994`のinstalled test appがwindow非表示のまま稼働し、2026-10-04 06:22:44 +09:00時点94件から06:41:35時点132件へAuditが増加した。増加分は`通知一覧`／`accepted` 38件。最終snapshotのAudit JSONL SHA-256は`sha256:2e2e756f928704aa6faf547465ccc2f72727fe76a4f3afa8389793aa10f3e9ee`、anchorは132 events／head `sha256:bd7b51a2ce29e2c1e2bc22cbe3459564f615e18c8fda49a3437b5eb7473dbaa1`。この最終snapshotの全chain／HMAC再計算は未実施であり、有効性を主張しない。
- Ownerから終了操作済みとの報告を受けたが、06:41:35時点でもrun40 Launcher PID `14376`とFrontend PID `2388`が稼働し、Frontendは非表示、fake API listener `127.0.0.1:56601`（PID `4624`）も待受中だった。通常終了Auditはなく、報告対象と試験用run40 processが一致したかは不明。process終了・graceful tray exit・cleanupを成立扱いしない。古いappはこの間もpollを続けたため、追加の通知Auditが生じ得る。
- 追加の終了報告後、06:55:02 +09:00に再確認しても同じrun40 Launcher／Frontend PIDとfake API listenerが残っていた。Auditは132件から159件へ27件増え、最新は`broker-audit-159`／`通知一覧`／`accepted`、JSONL SHA-256は`sha256:68532a0528d1a959c13e5a57eed07e2682fa085b597d2170d6a5e959dc9934fc`、anchor count 159／head `sha256:b412a686390a94ae3a7d36c063bcea3352a25358d139226679e094b85f7dec25`。この時点のchain／HMAC再計算は未実施。報告された終了操作がrun40 instanceへ届いていないことを再度確認し、通常終了は未成立のままとする。
- 原因はDesktop `_trayRefreshTimer`がwindowを隠した後も30秒周期pollを続けること。Win32の`WM_SHOWWINDOW`から実可視状態変化をMethodChannelで通知し、Flutter側は非表示時にtimerを即時cancel、再表示時に現在状態を再取得するよう修正した。Broker取得前後にもnative visibilityを確認し、非表示になったrequestの古いprojectionを捨てる。native側も非表示時点でprojectionを`不明`へ落とし、非表示中の再投影を拒否する。tray未対応／visibility照会不可時にはpollを開始しない。AuthorityやTask capabilityは変更していない。
- 検証: `flutter test --no-pub --no-test-assets test/windows_tray_client_test.dart` 6 passed。通常test-assets付きの初回起動はFlutterが`build\\unit_test_assets`を削除できず、test開始前に終了した。`flutter analyze --no-pub`はDesktop／Mobile双方で成功（日本語workspace path直接指定ではFlutter analysis serverがFormatExceptionで異常終了したため、同一workspaceの既存`Z:` mapping経由で実施）。`flutter build windows --release --no-pub`も同mapping経由でRelease build成功。日本語path直接指定では`app.dill`読込に失敗し、mapping経由の一度目は残存CMake build processとの競合でwrapper sourceを欠いた。重複process終了後の単独再実行は成功した。Schema 152件／example 152件／negative fixture 196件、strict Japanese audit 1134 files／0 findings、Manifest check、統合validatorのdevelopment check 10件は成功。Conformanceを`Z:` mapping経由で実行した試行はagent_runtimeの合成Workspace path checkに失敗したため、path-sensitive検証は物理`C:\\Users\\ohira\\OneDrive\\...` rootから再実行し231 checks passを確認した。統合validatorは物理rootからexit 0。
- この修正の新installed runでのwindow hide／tray reopen挙動はまだ未確認であり、source testとRelease compileを`LIVE_RUNTIME`証拠へ昇格しない。run40の正常終了、TEMP write／cleanup、active Task deadline／crash Recovery、Agent／scratch隔離、result／diff Content Exposure、OneDrive Cloud Files／通常NTFS統合、通常Release capability、Compare／Handoff failure pathも未成立。`task_execution=unsupported`、R2 `release_blocker`、`release_ready=false`を維持する。

## R2追補 run40のトレイ正常終了と最終Audit再検証（2026-10-04）

- 2026-10-04 07:43:22 +09:00の再照合で、source commit `8babcd3b4eba08e959e8d447d6a680d7cdbba994`のrun40 root由来processは0件、fake API listener `127.0.0.1:56601`も不在だった。run manifestはsource worktree cleanを示す一方、runtime `isolated=false`／`formal_runtime_proof=false`を宣言する。
- 最終file-backed Auditは188 event。末尾`broker-audit-188`はnative trayからのDesktop終了要求を`D4 Pocket Desktop終了`／`recorded`／`LIVE_RUNTIME`として記録し、event hashは`sha256:3915510aaa5448ad1f813a11a715bd2f5cb99dc9c6ec42d9841b0033ad57e283`。
- Rust `BrokerAuditEvent`の固定field連結hash規則で188 eventを独立再計算し、連番、重複ID、previous link、event hashの不一致はいずれも0。anchor event count／head一致、HMAC有効を確認した。headは末尾event hash、Audit JSONL SHA-256は`sha256:1463ee37fef4ef0ae91d14c1e751864334a5f00a9458d27289d61ae8445ce29c`。HMAC鍵値は表示・保存していない。
- run40ではTask start、Workspace Permission、Task Owner Approvalは発行されていない。したがって本追補はrun40の通常tray終了と最終Audit整合だけを閉じ、positive Task E2E、run40 sourceより後の可視性連動修正のinstalled検証、正式隔離runtime、TEMP cleanup、deadline／crash Recovery、結果／差分表示を証明しない。`task_execution=unsupported`、R2 `release_blocker`、`release_ready=false`を維持する。

## R2追補 起動器の異常終了後session回収境界（2026-10-04）

- production起動順はruntime instance lock取得後にBrokerを開始し、`RunningBroker::start`内の`prepare_session_paths`が前回残った`broker_session.json`と不完全な`broker_session.json.tmp`だけを検査・回収する。通常終了時は起動時に読んだendpoint bytesと一致する場合だけsession fileを削除する。Audit storeは起動時回収の対象外。
- この境界を固定するため、正常fixtureではstale endpoint／temporary fileの除去と永続Audit markerの不変を検査し、negative fixtureではfileでないendpoint pathを`SESSION_FILE_INVALID`で拒否し、endpoint pathと隣接temporary file双方を変更しないことを検査する。既存instance-lock fixtureも、lock fileが0 byteの安定したanchorとして残り、handle解放後に次のlock取得が可能なことを検査する。lock fileを毎回削除すると、別processが異なるfile identityをlockする競合を作り得るため削除しない。
- Validation: `cargo +1.95.0 test --locked --offline --manifest-path native/rust_helper/Cargo.toml --lib desktop_launcher::tests:: -- --test-threads=1`は28 passed／0 failed／4 ignored。`endpoint_cleanup_removes_only_the_original_bytes`は1 passed。変更file限定の`rustfmt +1.95.0 --check --edition 2021 --config skip_children=true native/rust_helper/src/desktop_launcher.rs`と`git diff --check`は成功。crate全体の`cargo +1.95.0 fmt --manifest-path native/rust_helper/Cargo.toml -- --check`はexit 1で494 format-diff sectionを報告したが、変更した`desktop_launcher.rs`の差分sectionは0。未変更Rust fileの既存format差分は修正していない。
- 証拠分類は`FIXTURE`。force-stop直後に残るrun38のpathをこのtestで削除・変更していない。実installed起動器が異常終了後に再起動し、stale endpointを回収しつつAuditを維持する`LIVE_RUNTIME`証拠は未取得。この回収fixtureはR2のinstalled provenance、TEMP、Task Recovery等を閉じず、`task_execution=unsupported`、R2 `release_blocker`、`release_ready=false`を維持する。

## R2追補 fresh installed run43のpositive Task・TEMP失敗・終了後監査（2026-10-04）

- run43はsource commit `38624dda3fe80d2ec955728ee3cb98d13db7b0c8`、manifest上`source_worktree_clean=true`のFlutter 3.44.0 Windows ReleaseとRust 1.95.0 Release `r2-e2e` feature buildからfreshにstageした。installed artifact、loopback fake Responses API、Codex CLI、manifestのhashとTask/Audit参照は`release_blockers.registry.json`の`r2_installed_synthetic_task_e2e_run43`に固定した。Codex CLIは実物0.160.0／SHA-256 `7d4588265a55adb1403f85d2e058b11dc971459842de84876c86ff02fb7771f2`。実model、実credential、課金、外部API要求は使用せず、Windows保護設定も変更していない。
- installed Flutter UIからRuntime／Workspaceを登録し、native Owner confirmationでWorkspace Permissionと独立したone-use Task Approvalを発行した。production Broker IPC経由、Session `b565cf151147d98ae35dd6ac261396ea`でTask `4e5d884a4d0798e9d67fc1a0b80a6c26`は`completed`となり、結果hash `sha256:55c569826e219f9d375b6ec4b1e4a15d2d569e81a4ff75403e579e18c53787ab`をUIで確認した。Permission Auditは`broker-audit-52/53`、Approvalは`broker-audit-54/55`、開始は`broker-audit-57/58`、完了状態の記録は`broker-audit-61`。指示本文はAuditに保存されず、製品UIは結果本文／Diffを表示せずhashのみを投影する。
- 実Codex CLI／MxC childを通したloopback-only fake Responses APIは2 requests、tool提示・送信・結果受領あり、重複tool call拒否0、invalid／incomplete request 0、応答write failure 0、外部要求0で終了した。Task完了時には合成Workspace completion markerを観測し、合成secret／Workspace外境界のprobe assertionは当該pathに限りpass、outside-write markerなし。これは一回の合成probeであり、広域filesystem隔離の証拠ではない。
- TEMP probeは認識済みscope `mxc_appcontainer`に対する直接writeを試みたが`temp_step=temp_write`／`temp_write=failed`／安全な診断`directory_missing`。directory作成とretryは未実施で、MxC物理TEMP cleanupは証明しない。Task本体のpositive completionとは分離して記録する。
- オーナーから終了済みとの報告後も、run43起動器（PID `19724`）とFlutter画面本体（PID `1920`）は残っていた。Taskの終端と偽APIの終了、停止前Auditの整合を確認し、このrunの実行pathに一致する2 processだけをforce-stopした。通常のトレイ終了操作とDesktop終了Auditは観測していないため、force-stopを正常終了の証拠として扱わない。
- 停止後にfile-backed Audit 70件を独立再計算し、連番／event hash／previous linkの不一致0、重複ID 0、anchor count／head一致、HMAC有効、回復journal 0件を確認した。末尾`broker-audit-70`は通知一覧の受理記録で、Desktop終了記録ではない。head `sha256:7e7e725095f9cf9037cd8d2bd36c7eff67e5a3762b36766e8960147c32ba58fc`、JSONL SHA-256 `sha256:06b3f35c362c7c1bfe9503b8ac88ac1ac1b2455a4fb4a798061833ae81bd0524`。
- 同一profileでの再起動はrun43では行っていない。したがってこのrunからAudit restart durabilityの追加証拠は主張しない。
- 終了後、run専用profileの`broker_session.json`（274 bytes）が残った。process不在と正確なrun43 profile内の単一fileであることを照合した削除試行は実行policyに拒否されたため、別手段で迂回せず保全した。0-byte `desktop_launcher.lock`は安定したlock anchorとして保持。raw TEMP report／継続markerは不在、Workspace file 0、`.d4p-tmp-*` 0、run process 0。Audit storeとsanitized TEMP observationは保持した。
- runtime provenanceも未成立。実storeはrun-specific `LOCALAPPDATA`配下で観測されたが、同梱`installed_manifest.json`はruntime scope `per_user`／`isolated=false`／`formal_runtime_proof=false`を宣言している。試験用environmentの隔離を製品manifestの保証へ昇格しない。
- 証拠区分はinstalled製品経路の観測を`LIVE_RUNTIME`、保存Auditの投影を`INTERNAL_STATE`、loopback偽API／合成Workspaceを`FIXTURE`とした。artifactとAudit JSONLのSHA-256、停止後Audit 70件の連番／event hash／previous link／重複ID、anchor HMACを再計算した。
- 文書更新後の統合validator初回実行は、日本語基底監査がこの記録の長い英語偏重段落を1件検出してexit 1となった。該当段落を日本語中心に修正し、厳格監査を再実行して1134 files／0 findingsを確認した。初回失敗は履歴に保持する。
- 修正後の最終確認: `python -X utf8 tooling/manifest.py --write`は1131 filesを書込み、`python -X utf8 tooling/manifest.py --check`は合格。Schemaは152 schemas／152 examples／196 negative fixtures、Conformanceは231 checks、`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`はexit 0で登録済み開発検査10件すべて合格した。`release_ready=false`は保持する。
- run43は一回のpositive Task completionを再確認するが、R2全体は閉じない。通常Release `task_execution=unsupported`、R2 `release_blocker`、`release_ready=false`を維持する。TEMP write／cleanup、deadline／crash Recovery、task間隔離、result／diff Content Exposure、OneDrive Cloud Files／通常NTFS統合、通常Release gate、Compare／Handoff failure、formal runtime provenance、graceful tray exitは別gateとして残る。

## R2追補 run44の同期失敗とrun45のpositive Task・MxC TEMP write（2026-10-04）

- 作業開始時にremote `main`をfetch／pruneし、local `main`／`origin/main`がともに`7340e27566b11f5fa729fbf4212ff82d5380f595`、working tree cleanで一致することを確認した。run44・45で使用した段階配置品のmanifestも同commit、`source_worktree_clean=true`。run45はrun44用stage artifactを再利用し、fresh synthetic profile／Workspaceで実行した。したがって新しい配布・正式isolated installの証拠ではない。artifact hashとmanifest hashは`release_blockers.registry.json`へ記録した。
- run44 Task `70dcb419a6c62140189bb79a7b6545fe`はfailed、終端状態照会は`broker-audit-66`。実Codex CLI／MxC childはTEMPを`mxc_appcontainer`と分類しmarker書込みに成功したが、installed run用host coordinatorがTEMP検査後に`broker-real-codex-task-continue`を作らず、childは期限切れで終了した。偽APIは4 requests、tool offer／call／resultあり、同一tool再送拒否3回、Workspace成功markerなし。これはrun44の試験同期欠落として記録し、Task成功やboundary assertion成功へ読み替えない。実行APIはrun44開始時hashを保存していない。
- run45は同じ段階配置品に新しい合成Workspace・profileを用いた。実Flutter UI、native Owner確認、authenticated production Broker IPC、実Codex CLI 0.160.0／MxC child、loopback偽Responses APIを通した。登録時にはOwner承認fixture marker欠落をBrokerが拒否（`broker-audit-110`）し、Permission／Approval／Taskは作られなかった。fixtureを補ってからRuntime／Workspace登録とSession結合が成立し、Sessionは`b826360ab7340fbbb3d2eaa5df9f4feb`。Workspace Permission（`broker-audit-135/136`）と別個のTask Owner Approval（`broker-audit-138/139`）をnative確認で発行した。
- Task `57c484a483c285f94ff3eec084b8394b`は状態照会`broker-audit-147`と製品UIで`completed`、結果hash `sha256:55c569826e219f9d375b6ec4b1e4a15d2d569e81a4ff75403e579e18c53787ab`。開始記録は`broker-audit-143/144`。指示hashは`sha256:992f746b8277bdb5927909bfa14b2e17fc285b0703450551cc3b1ad2d0925233`。UIはhashのみを表示し、結果本文／DiffはBrokerから未取得。偽APIは2 requests、tool offer／call／result受領あり、再送拒否0、invalid／incomplete request 0、response write failure 0、外部要求0、process exit 0。
- 固定合成probeは登録secret読取、Workspace外read／writeを拒否した後にのみTask markerを書く。markerは20 bytes、SHA-256 `sha256:712aff9ceb6418d369c1f968d51294af292dbfe4bda6739d4e86ba333227a29c`。host側でsecret fixtureのhash不変とoutside-write marker不在を確認した。この一つの名前付きprobeを越える広域filesystem隔離は主張しない。
- TEMP reportは`mxc_appcontainer`、`temp_write=passed`／`temp_step=temp_write_completed`。child実行中のTEMP marker内容をhostから一致確認できた。Task終了後はhostからmarkerとTEMP directoryの双方が見えず、Workspace Broker scratchは0件。ただしprobe自身の`temp_cleanup`は`not_attempted`であり、終了後の非可視だけでは物理削除を特定できないため、MxC物理cleanup合格にはしない。TEMP reportのhashは`sha256:0dab4805b611b3ff2fcfcacb9474d6c904dc10d081339bf957e523545555372b`。
- 偽Responses APIはloopback限定の合成fixture。実model／実credential／課金／外部API要求は不使用、Windows保護設定も変更していない。Codex CLIは実物`0.160.0`、SHA-256 `sha256:7d4588265a55adb1403f85d2e058b11dc971459842de84876c86ff02fb7771f2`。製品manifestはruntime scope `per_user`、`isolated=false`、`formal_runtime_proof=false`を宣言し、今回のrun profileを製品隔離保証へ昇格しない。
- Ownerはrun45の終了を報告したが、その後のLIVE_RUNTIME再確認でもLauncher PID `6152`とFlutter frontend PID `17656`が稼働し、D4 Pocket windowが列挙された。Brokerの`session.json`は`active`、最新確認時点のAudit末尾は`broker-audit-207`（`通知一覧`／`accepted`）で、終了報告後も通知照会Auditが進んでいた。したがってnative tray通常終了は観測されず、Desktop終了Auditもない。Computer Useでは通知領域を選択できず、force-stopは行わない。通常終了が成立するまで最終Audit chain／HMAC検証、session fileとsynthetic profileのcleanupを保留する。Owner報告と実測の不一致を隠さず記録し、残る物理操作はtray menuの「終了」を成立させること。
- run45は`r2-e2e`試験feature付き段階配置品でのpositive Taskと実TEMP writeを示すが、通常Release capabilityではない。`task_execution=unsupported`、R2 `release_blocker`、`release_ready=false`を維持する。通常Release昇格、正式runtime provenance、Audit restart durability、failure／deadline／crash Recovery、result／diff Content Exposure、task間隔離、OneDrive Cloud Files／通常NTFS統合、Compare／Handoff等は未成立のまま保持する。

## R2追補 run45のタイトルバー×操作に関するOwner訂正（2026-10-04）

- Ownerから、run45時の操作はタイトルバーの×を押して画面を消したものであり、トレイメニューから製品を終了したのではないと確認した。前節の「Ownerの終了報告」と「残る物理操作はtray menuの『終了』」という当時の解釈は訂正する。クリック直後にLauncher／frontend processと通知照会が継続していたのは、×でトレイへ隠れる仕様と整合する。
- その後の確認ではrun45のprocessは不在だったが、介在操作と停止原因を観測していないため、×がprocessを終了させたとも正常終了したとも推定しない。最終Auditは231件で末尾は`broker-audit-231`／`通知一覧`、anchor count／head一致は確認済み。全event chainとanchor HMACは再計算しておらず、`broker_session.json`も残存しているため、run45のgraceful tray exit、最終Audit耐久性、endpoint cleanup、profile cleanupは未確認のまま。
- 追加のOwner物理操作は開発試験の前提ではない。通常tray exitの検証は既存controlled UI automation／product test pathで行う。run45のpositive Task・合成secret／Workspace外path probe・TEMP writeの限定証拠は維持するが、reused staged artifact、`r2-e2e` feature、`isolated=false`／`formal_runtime_proof=false`を明示し、通常Release capabilityへ昇格しない。
- source `64c608465982407a759f6059ef00d900fb30139e`はAgent CenterのTask状態表示とrun45操作記録を訂正した。これはfresh installed artifactやTask結果本文／diff経路、終了Auditの追加証拠ではない。

通常Releaseの`task_execution=unsupported`、R2 `release_blocker`、`release_ready=false`を維持する。

## R2追補 タイトルバー×による非表示中のトレイpoll回帰fixture（2026-10-04）

- Owner訂正どおり、Windows native `WM_CLOSE`はトレイの終了要求がない場合に`ShowWindow(SW_HIDE)`を行い、`WM_SHOWWINDOW`で可視性変化をFlutterへ通知する。Flutterの常駐トレイ更新は可視中だけ30秒周期で`通知一覧`を読み、非表示通知でtimerを停止して表示値を`不明`へ戻す。再表示通知後は現在状態を再取得してから投影する。
- `ShellHomePage`を通るFlutter testを追加した。試験用native channelで可視／非表示を遷移させ、試験用Broker transportが受ける`通知一覧`要求数を照合する。可視中の周期更新、非表示60秒間に追加pollがないこと、再表示直後と次周期の更新再開を確認した。さらに通知要求の応答を保留して非表示化し、応答解放後に古い射影が再publishされず`不明`表示が保持されることを確認した。
- OneDrive内checkoutではFlutterが生成物`macos/Flutter/ephemeral/Packages/.packages`と`build/unit_test_assets`を削除できず、Flutter commandが起動前に停止した。ACL、OS保護設定、製品pathは変更せず、tracked sourceをASCII名の一時copyへ複写して同じFlutter SDKで検査した。tray testは8 passed／0 failed、Desktop全体とMobileの`flutter analyze`は問題なし、`dart format`差分なし。
- この回帰検査の証拠区分は`FIXTURE`。native channelとBroker transportは試験用であり、installed Windowsの実際の×操作、実Rust Broker、実Audit増分停止を証明しない。起動中の一件は完了してよいが、非表示windowへ古い値を再投影しない境界のfixture検査である。
- `release_blocker`: Windows installed productでのtray icon、×によるhide／再表示、通常終了、Broker-mediated stop requestの一体実証は未成立。今回のtest追加はC20のrelease gate、通常Release `task_execution=unsupported`、R2 `release_blocker`、`release_ready=false`を変更しない。

## R2追補 run44 installed smokeのruntime identity path不一致とcollector修正（2026-10-04）

- run44由来のinstalled product artifactを独立hash照合した。App SHA-256 `sha256:4f251459fc7388fb779da6006e29923d41280d39ced995869b09b4555059745b`、Broker SHA-256 `sha256:d677eddb9169e243530478a71571cd4e52d9f1152c073bdc4183e4c1c131cc40`、launcher SHA-256 `sha256:8e6bc2c88c7bf329ecd2d0787c2994f83ee3bfbb2927fb6a9039bbba538d6391`はstaged manifestと一致し、source commitは`7340e27566b11f5fa729fbf4212ff82d5380f595`。staged manifestは`manifest_version=2`でgeneric GUI-Shell pathを宣言していた一方、launcher binaryにはApp ID `d4-pocket-app-d1474f66da93456d958f1d31f587db69`とAudit store ID `audit-store-f314a97880754e509c4d09ae92aa47bf`が埋め込まれていた。
- `collect_installed_smoke.ps1`のDiagnosticOnly実行は、Rust Desktop起動器が宣言済みruntime endpointを作らないとしてcollector timeoutになった。collector期待pathが`%LOCALAPPDATA%\GUI-Shell\broker\desktop`だったのに対し、LIVE_RUNTIME Auditはcompile-time identityに対応する`%LOCALAPPDATA%\D4Pocket\apps\<App ID>\stores\<Audit store ID>`側に作成されていたため、製品実行成功とcollectorのpath前提不一致を切り分けた。temp profileのAuditは24 event、末尾`broker-audit-24 / D4 Pocket Desktop終了 / recorded / LIVE_RUNTIME`で、anchor head/count一致と`broker_session.json`不在を観測した。event hash chain全件とanchor HMACは再計算しておらず、この失敗runをformal passへ昇格しない。Product Manifestは旧stagingに保存されていない。
- 配置処理に製品定義書の受取りと、起動器実行file内のコンパイル時識別子との照合を追加し、識別子ごとの保存先と第3版の配置記録を生成する。Broker単体検査用の接続先は従来どおり一時領域に置き、製品runtimeの`launcher_runtime.session_file`から分離した。証拠収集器は第2版との互換を保ち、製品第3版では製品定義書とそのSHA-256、コンパイル時識別子から実保存先を求め、製品定義書を証拠一式へ結合する。厳格検証器は識別子種別、ID、実保存先末尾、配置済みManifest path、証拠一式の不一致を`release_blocker`にする。保存先識別情報の証拠区分は`CONFIG`であり、製品隔離や正式release証明を実測した証拠の代用ではない。
- 規約適合試験に、旧形式との互換、正しい製品識別子の受理、不正識別子の拒否を追加した。現行ソースから製品を再生成し、新しい配置・実行で再検証する作業は未実施である。本記録は診断と修正を記録するもので、Windows配置後の初回実行判定、R2、`task_execution=unsupported`、`release_ready=false`を変更しない。次は製品定義書を含む新規D4 Pocket書出しから実行収集器第17版を再実行し、実保存先、操作画面の読取、通常終了、接続情報ファイルの整理を再観測する。

## R2追補 Broker再起動時の中断Task隔離（2026-10-04）

- 着手時の正本は`main`=`origin/main`=`52f7817fc10e9f7870338801d1e4272146d76b0b`で、working treeはcleanだった。編集前rollback pointとしてlocal `codex/backup-main`／`codex/backup-main-prev`とremote tag `refs/tags/codex/backup-main`／`refs/tags/codex/backup-main-prev`を同commitへ保存した。
- Owner許可済みの手動GitHub Actionsをrun `37177787371`（[実行記録](https://github.com/gatchimuchio/GUI-Shell/actions/runs/37177787371)）で、対象commit `52f7817fc10e9f7870338801d1e4272146d76b0b`に対して実行し、Windows runnerの結果はsuccessだった。対象はfixture由来の未署名Windows Release bundle構築・検証であり、artifactは生成・保持されず、installed product、active Task recovery、Release readinessの証拠ではない。triggerは`workflow_dispatch`のみで、自動CIは追加していない。
- `Broker::new_persistent`は読戻したAuditEventからPermission／Owner Approval一回消費済みのTask開始を照合し、terminal eventを持たないTaskを再開せず`suspended`として一度だけRecovery Auditへ記録する。Permission／Approvalを復元せず、RecoveryActionはWorkspace差分確認を要求する。Completed／failed／cancelled等のterminal Taskは再分類しない。task本文ではなくtask IDと開始Audit hashを材料にする。観測根拠は耐久Audit記録の照合であり、Broker再起動だけから元のcrash原因を断定しない。
- 正常・境界・negative／idempotency確認: `cargo test --locked --manifest-path native/rust_helper/Cargo.toml task_crash_recovery_is_idempotent_and_fails_closed` 1 passed。Broker process kill後のAudit付きscratch回収fixture: `cargo test --locked --manifest-path native/rust_helper/Cargo.toml broker強制終了後の別process起動登録で永続scratchを監査付き回収する` 1 passed。偽Codex CLIのdeadline／子孫停止／scratch cleanup fixture: `cargo test --locked --manifest-path native/rust_helper/Cargo.toml 偽CodexCLIはAdapterのTask成功・期限超過cleanup・子孫停止を通る` 1 passed。Windows Job ObjectのBroker owner終了後descendant／grandchild停止fixture: `cargo test --locked --manifest-path native/process_supervision/Cargo.toml abrupt_broker_exit_closes_job_and_terminates_descendants` 1 passed。
- 全Rust helper target: `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1` success、合計460 passed／0 failed／7 ignored（既存のignored testを含む）。`rustfmt --edition 2021 --config skip_children=true --check native/rust_helper/src/broker/protocol.rs` success。workspace全体の`cargo fmt --manifest-path native/rust_helper/Cargo.toml -- --check`は既存の多数の無関係Rust fileのformat差分でfailedしたため一括整形はしていない。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows` exit 0。日本語基底strict、152 Schema／example、232 conformance check、manifest、release gate、packaging portability、release smoke、evidence bundle、release runtime assertions、development auditは実行結果上passed。validator内のWindows installed evidenceはcurrent provenance、installed first-run、Setup Doctor、Broker installed smoke、external Audit tamper evidenceを未成立として列挙し、`release_ready=false`を保持した。これはinstalled R2 E2Eの成功ではない。
- 今回のBroker再起動照合とscratch再回収は`FIXTURE`／永続状態の`INTERNAL_STATE`範囲、偽CLI deadlineとJob Object killはそれぞれ局所fixtureの`LIVE_RUNTIME`範囲に限る。D4 Pocket installed product上でのactive Task deadline停止・Recovery、Codex／Broker／Launcher各crash後の一体Recovery、Agent CenterからのRecovery状態再取得は未実証であり、この追補だけでユーザー指定R2項目を閉じない。
- R2は継続、通常Releaseの`task_execution=unsupported`、`release_ready=false`を維持する。既存positive Task、権限一回消費、Workspace／secret境界、Audit耐久性、Desktop終了、cross-Workspace局所LIVE_RUNTIME証拠は履歴どおり保持し、同じ試験を別名で反復しない。残る有限R2項目は現行指示に沿って順次統合検証し、明示条件が成立した時点でR2を終了してR3へ進む。

## R2追補 run50 installed Launcher crash試験の失敗とAudit記録形式修正（2026-10-04）

- run50はclean source commit 1dc2014c6a71b5dd1cf094a2f313608ab06b9160から作られたr2-e2e段階配置品。artifact hashはapp sha256:4cdf0b3cf55d8c034d87b06b0ba297860581be6ff8f2231a347ec2d22197ac46、Broker sha256:d208cf43d29c6bd2dc6eb3c411056bb37030faaf9fdfb45f4163aace431ba64a、Launcher sha256:f63380d7b38ecc4dda7727a07cd2d10874fd8139f9d06078254e0dd6c4db1f9f。Manifestはidentity_kind=gui_shell、isolated=false、formal_runtime_proof=falseであり、Release provenance evidenceではない。
- installed Flutter UIから合成Runtime／Workspace、native Owner確認、Workspace Permission、独立one-use Task Approvalを通し、Task fdf5688cc878b9574978f3c08d837953を開始した。永続Audit broker-audit-59はoperation=AgentTask実行、decision=recorded、reason=Agent Task実行開始（Permission／Owner Approval一回消費）。Broker restart後のAudit JSONLは108 eventで、当該Task IDは開始event一件だけ、Agent Task中断回復eventは0件だった。これはinstalled productのLIVE_RUNTIME失敗証拠である。
- 根因は、起動Recoveryが意味markerをAuditEvent.operationだけから検索していた一方、production AuditではBroker operation名がoperation、Task開始／完了等の意味markerがreasonに保存されるprojection差。したがって前commitの単体fixture PASSはproduction形式を再現していなかった。task_execution=unsupportedとR2 release blockerは維持する。
- run50のfake Responses APIでは4 request、3 repeated-tool-call rejection、completion markerなし。MxC TEMPはmxc_appcontainer、write failed／directory_missing。Codex／PowerShell descendantsはLauncherを停止する時点ですでに消えていたため、crash時process-group stopを検証したとは扱わない。Launcher停止後にFlutter childが残り、確認したrun50の2 processだけをcleanupした。通常終了の証拠ではない。
- Recovery実装は、Task開始／終端markerをoperationまたはreasonのいずれにも認識するよう修正し、production形式（開始AgentTask実行／終端AgentTask取消と意味marker in reason）、旧直接形式、完了Task非回復、再起動冪等性を同じ回帰testへ加えた。focused recovery testは1 passed。crash fixtureのPID札も一時fileからrenameで公開し、親processが空の準備fileを読むraceを除去した。この修正後のBroker crash fixture単体は1 passed。
- 次は修正後sourceからfresh staged productを作り、Task tool childが実行中のProcess chainを確認してからLauncher／Broker crashを起こし、同じAudit storeの再起動後Recoveryとprocess終了を照合する。live再検証がPASSするまでcrash Recovery条件は未成立。通常Release task_execution=unsupported、release_ready=falseを維持する。

## R2追補 fresh installed run51のBroker crash Recovery監査とprocess cleanup未判定（2026-10-04）

- run51はclean source commit `b19a4f05b9d38e0057f15ae61d02a67f386898c7`から作ったfresh `r2-e2e` staged product。stage rootは`C:\Users\ohira\AppData\Local\Temp\d4p-r2-installed-run51`、manifest SHA-256は`sha256:de67d49d7d3e90b6044f57fba5433c93ef3df2091bbf2dda80228d967f253de9`。App／Broker／Launcher hashは`sha256:4cdf0b3cf55d8c034d87b06b0ba297860581be6ff8f2231a347ec2d22197ac46`／`sha256:b35cdedc3479e6a651a7d6b0f121a734b2ce24ed2091455fab5d8fee2a30a339`／`sha256:28b3ebaa51d4577bd5df3918e39130ec8e81eac7870377a0b4bee95ede63b405`。Manifestは`identity_kind=gui_shell`、`scope=per_user`、`isolated=false`、`formal_runtime_proof=false`、`product_manifest=null`を宣言する。Process環境をrun専用profile／`LOCALAPPDATA`へ上書きしてstoreを隔離したが、正式Runtime provenanceの証拠ではない。fake API executableの実行開始時hashは記録していない。
- installed Flutter UI、native Owner確認、Production Broker経路でRuntime／Workspace登録を行い、Workspace Permission `broker-audit-57`、別個のTask Approval `broker-audit-62`、Task開始`broker-audit-70`を確認した。Task IDは`7ec80a7ebc87995f3f2d4c68ee44c4ec`、Task本文hashは`sha256:1e3b6477058a6aba51d759188018139b28838382630af5abd0041a604d375be8`。Task開始eventは`operation=AgentTask実行`／`reason=Agent Task実行開始（Permission／Owner Approval一回消費）`。
- 偽Responses APIの停止時記録では要求4件、`models=0`、tool offer／call／resultの受信あり、重複tool-call拒否3件、外部要求0件だった。合成Workspace境界の事後確認は`true`と報告された一方、Workspace完了目印は存在しなかった。MxC一時領域の記録は`scope=mxc_appcontainer`、`temp_write=failed`、`directory_missing`。Agent Task終端結果はBrokerに返らず、この限定fixtureの結果を、Agent Task成功、一時領域の回収、秘密情報・作業領域の一般保証へ拡張しない。
- 正確なrun51 Launcher PID／path／hashを直前照合してBroker／Launcherを強制終了した時点で、Codex／PowerShell tool descendantsは既に不在、Flutter childは残存していた。従ってLauncher crash後のFlutter残存は観測したが、active tool process群をcrash時に止めた証拠ではない。Flutter childはpath・hash・parent PIDを再照合後に試験cleanupした。Fake APIは明示stopした。
- 同じ隔離profile／Audit storeでLauncherを再起動すると、Audit開始時点のTask `broker-audit-70`に対応する中断Recovery `broker-audit-78`が一件追加された。request IDはTask開始eventと一致し、decision=`suspended`、evidence=`INTERNAL_STATE`。理由は永続開始にterminal記録がないTaskを再開せず隔離し、既存Permission／Approvalを復元しないというもの。Broker再起動後のAudit読戻しとrecovery event追記は`LIVE_RUNTIME`で成立し、起動時にBrokerが同一storeを受理した。終端Task結果、process-group停止、再利用拒否の追加IPC試行は確認していない。
- 最終file-backed Auditは113 event、head `sha256:61337208a56408648038b2fd62f7b816fa78dd4141912cda8d0599651bb55fc8`、JSONL SHA-256 `sha256:76f37776657aee195ba06e16e1124c097e1a5bcb088a4f15bb75aea0fb9fe22c`。anchor count=113かつhead一致。Broker再起動が保存済みstoreを読み込んで新eventを追記したことは確認したが、このrunの独立HMAC再計算はしていない。
- 再起動後のAudit UIでRecovery marker検索は「一致する監査事象なし」、Recovery画面にもTask固有項目は表示されなかった。Agent Centerへ戻った後、登録開始操作の直後にFlutter windowがWindows上で「応答なし」と表示された。secondary native dialogの有無を確認する前に、hash／pathを照合したrun51 Launcher／Flutterだけを停止したため、ハング原因は未確定。このrunで通常tray exit／終了Auditは成立していない。Audit storeとsynthetic profileは保持した。
- 判定: installed Broker再起動後のAuditによるTask RecoveryはPASS。active Task deadline、crash瞬間の子孫停止、MxC TEMP cleanup、Agent CenterからのRecovery／結果投影、通常Release capability、installed artifact provenanceは未成立。`task_execution=unsupported`、R2 `release_blocker`、`release_ready=false`を維持し、このUI無応答の原因を確認するまでは同じrunの再試験で隠さない。

## R2追補 active Task deadline観測用fixtureの有界化（2026-10-04）

- 通常の合成Workspace probeはこれまでどおり最大30秒で継続markerを待つ。専用の合成Workspaceに`broker-real-codex-task-extended-supervision` markerが明示された場合に限り、最大40,000回×25ms（1,000秒）まで待機する試験専用分岐を追加した。production Brokerの900秒Task期限、process監督、権限、通常Release capabilityは変更していない。
- focused validation: `cargo +1.95.0 test --locked --offline --manifest-path native/rust_helper/Cargo.toml --all-targets --features r2-e2e deadline_supervision_probe_waits_past_broker_limit_only_with_explicit_marker -- --nocapture` — unit test 1 passed、および同名の`r2_e2e_responses` fixture test 1 passed。`rustfmt +1.95.0 --check --edition 2021 native/rust_helper/tests/support/codex_loopback_responses.rs`もpassed。
- 証拠分類は`FIXTURE`。この変更はinstalled Taskの期限到達、Brokerの期限処理、Job Objectによるprocess群停止、Recovery／Auditを証明しない。次の専用installed runで実Taskが900秒期限を越えて生存を継続するよう同期し、期限到達後の終端・子孫停止・Recoveryを一体観測するための試験制御だけを提供する。
- `task_execution=unsupported`、R2 `release_blocker`、`release_ready=false`を維持する。R2期限条件はinstalled `LIVE_RUNTIME`の期限到達／停止／Recovery証拠でのみ閉じる。

## R2有限完了条件の固定（2026-10-04）

- 基準Repositoryはremoteと一致した`main` HEAD `5276ea6f0a89c7ec644640f351aa12eefcbb2070`。この記録はscopeと完了条件を同期するだけで、新たなinstalled実行証拠を追加しない。
- ユーザーが実証済みと指定したpositive Task completion、Permission／Approval一回消費、production Broker／実Codex CLI／MxC経路、Workspace書込みとresult hash、durable Auditとchain／HMAC、Broker再起動読戻し、tray正常終了、局所cross-Workspace隔離は既存の追補を再利用する。同じ条件を別run名・別fixtureで反復しない。
- 残件はregistryの`r2_bounded_exit_gate.remaining_conditions`に列挙した8条件に限定する。内容は、実Taskの取消・期限監督と子孫停止・復旧、Codex／Broker／Launcher異常終了時の復旧、Task／Agent間の隔離、Agent Centerにおける結果・差分・変更file・試験結果の内容露出制御、OneDrive Cloud Filesと通常NTFSの実製品経路における秘密・作業領域境界、Broker所有の一時作業領域の保証範囲、通常Release能力の有限昇格条件、実行由来と配置成果物識別子の結合である。R2外の仮説的riskはblockerへ積み増さない。
- D4 Pocketが負うTEMP保証はBroker-owned `WorkspaceTaskScratch`に限る。MxC／Windows AppContainer内部`TEMP`の物理削除をD4の保証と主張せず、それを理由に無期限でR2を延長しない。
- 全有限条件がPASSしたら通常Release `task_execution=supported`へ昇格し、直ちにR3 Multi-Agent Compareへ進む。R3の完了後はR4 Handoffを続ける。
- このscope同期時点では`task_execution=unsupported`、R2 `release_blocker`、`release_ready=false`を維持する。以後の各条件はLIVE_RUNTIME等の適切な実証と作業単位のcommit／pushで履歴追記する。

## R2追補 deadline試験のfixture失敗記録と継続要求対応（2026-10-04）

- run52はrun51段階配置品（source commit `b19a4f05b9d38e0057f15ae61d02a67f386898c7`）を再利用したinstalled synthetic Task試行。Agent CenterのTask IDは`c9c89e299d261afdf1c92ebbde61c943`、Task開始Auditは`broker-audit-229`、終端Auditは`broker-audit-238`。状態更新後にUIは`failed`となり、Codex／PowerShell子孫とWorkspaceTaskScratchは不在、journalのactive entryは0だった。終端Auditは失敗／取消の理由を示すが期限到達を確定しないため、この結果をdeadline成功へ昇格しない。
- 偽Responses APIは4 request、tool offer／call／result各あり、repeated tool-call rejection 3件、Workspace完了markerなし、外部要求なしを報告した。試験probeはBroker期限を越えて最大1,000秒待つ一方、Codex tool callは30秒でyieldする。loopback fixtureはWorkspace完了markerより前の継続要求を拒否するため、期限前に試験経路が失敗したことが根因と判定した。このrunではBroker期限処理、process群停止の原因、Recoveryを検証したとは扱わない。MxC AppContainer TEMPはwrite=`failed`／`directory_missing`で、D4-owned scratch保証とは分離する。
- このfixture矛盾を閉じるため、`r2-e2e` feature buildに限って`GUI_SHELL_R2_E2E_TASK_EXECUTION_LIMIT_MS`による短縮期限を追加した。有効範囲は1–899,999msで、未指定／不正値は固定900秒へ戻る。通常Releaseはこの環境変数を読まず、Ownerの`max-runtime-900s`上限とproduction期限は変えない。偽Responses API helperはdeadline期待modeを追加し、継続拒否・tool結果・Workspace書込みが起きていないことを検査する。既存positive modeは2 request、tool結果、Workspace markerを引き続き要求する。
- focused validationでは期限parser 1 passed、loopback helper 15 passed。全Rust target試験は1回目415 passed／1 failed／6 ignoredで、失敗は今回の変更外のA2A loopback response-read testだった。同test単独再実行は1 passed。全target再実行では同testだけを明示skipし、lib 415 passed／0 failed／6 ignored／1 filtered、r2_e2e_responses 15 passed、main 10、broker_ipc 10、canonical_decimal_hash 1、checkpoint 8、protected_data 2、protected_startup 1、protected_store 3、workspace_diff 2、workspace_reader 2、workspace_startup 7がすべてpassed。default／`r2-e2e`双方の`cargo check --all-targets`もpassed。
- これは試験制御とFIXTUREの修正であり、installed deadline Recovery・process群停止をまだ証明しない。次はこの修正を含むclean commitからfresh staged artifactを作り、短縮deadlineのLIVE_RUNTIMEを行う。deadline項目は未成立、R2 `release_blocker`、通常Release `task_execution=unsupported`、`release_ready=false`を維持する。詳細と不成立runの履歴は`release_blockers.registry.json`へ追記した。

## R2追補 Agent Center 内容露出制御の実装とローカル検査（2026-10-04）

- 基準HEAD `2c24ab46b36015000e73759765130342b9667ed5`からの未commit作業treeに、Task結果とWorkspace内容を別々の権限経路として接続した。Agent結果本文はBroker process内だけに保持し、1本文1 MiB以下・最大8本文・900秒の取得期限とする。native Owner確認はTask ID／result hash／visibilityに結合した5分・一回限りで、本文取得前に消費し、永続Auditにはhashと状態だけを記録する。期限後の本文取得は拒否するが、無通信中の物理メモリ消去時刻は保証せず、期限回収は次のBroker要求時またはBroker終了時となる。保持数と本文長は上限内である。
- `full`だけがAgent出力本文を返し、`hash_only`はhashのみ、`none`は本文なし、`summary`／`redacted`は承認済みrenderer未実装を固定表示する。Agent Centerは出力を未検証のAgent自己申告として表示し、Agentが主張した試験結果をBroker実行済み試験へ昇格しない。実行済み試験recordがない状態は`unknown`と明示する。
- 変更file／diffは結果表示Approvalを流用せず、選択中のRuntime／Workspaceに結合したWorkspace Inspectorの独立読取Approvalとnative Owner確認を要求する。secret除外は登録済みpathだけを対象にする。Workspace baselineはWorkspaceあたり一つでTask IDに結合されないため、UIは特定Taskの変更証拠と表示しない。full Workspace読取が許可されれば登録除外path以外のfile内容を露出し得ることも表示する。
- Contract、Schema、正例／負例、Broker／native confirmation、Dart client、Agent Center、Workspace Inspectorの変更を追加し、`task_execution=unsupported`、R2 gate未完了、`release_ready=false`は変更していない。
- 検査結果: `python -X utf8 tooling/schema_check/check_schemas.py`はSchema 153、正例153、負例197でPASS。`python -X utf8 tooling/conformance_tests/run_conformance_skeleton.py`は232 checksでPASS。`python -X utf8 tooling/日本語基底監査.py --strict`は1137 repository files／findings 0でPASS。Focused RustはAgent Task結果4 passed、Workspace／secret境界・diff等34 passed。Desktop Flutter全suiteは158件中157 passed／1 failed（実Broker loopbackのRuntime lifecycle testでconnection reset）。同一test単独再実行は1 passed。Workspace Inspectorを含む他caseは全て完了し、この変更のWidget／client focused testはPASS。
- ローカル`flutter analyze --no-pub`はDesktop一時検証copy上で143 issuesとなり、source URI不在と現行sourceにない`selected`参照を報告した。Mobile側の同コマンドもOneDrive配下でAnalysis Serverが不正JSON parse errorにより終了した。Dart unit testはDesktop copyから実行可能だが、どちらの解析もclean判定として採用しない。固定toolchain・clean checkoutの手動Windows Actionsで解析と全suiteを別途確認する。Actions未実行であり、Windows installed product／native Ownerからの本文開示／diff投影の`LIVE_RUNTIME`証拠も未取得。
- この作業は内容露出制御の実装・Contract／fixture／unit検査までであり、R2条件4をPASSへ閉じない。該当条件はinstalled product経路の正例とvisibility／expiry／secret否定を観測するまで未成立とする。他の有限残件を増やさない。通常Release `task_execution=unsupported`を維持する。

## R2追補 Workspace起動統合testのOwner確認契約是正（2026-10-04）

- Windows手動Rust Actions run #37（`workflow_dispatch`、対象commit `0977bb1f34914a4b469cb622cece55081a1e368c`）は失敗した。`workspace_startup`の4 testが、通常Owner資格CLIだけでWorkspace内容Approval／基準点保存を成功できる旧挙動を期待していたが、現行BrokerはRust Desktop起動器のnative確認がない要求を`desktop_native_owner_confirmation_required`でfail-closed拒否した。失敗test名とrun URLは`release_blockers.registry.json`へ追記する。
- 該当するプロセス結合試験を、native確認のない通常IPC／Owner CLIではApproval、読取、基準点、変更一覧、diffを許可しない拒否契約へ修正した。製品の権限判定は変更していない。Workspace読取・差分の成功試験は既存targetに残し、Agent Centerからの成功結果投影はR2 Content Exposure条件で`LIVE_RUNTIME`確認する。
- 修正後の`workspace_startup`対象試験は7 passed／0 failed。Schema 153件／正例153件／負例197件、Conformance 232件、日本語厳格監査1137 file／findings 0、Manifest 1134件の再生成と照合、`git diff --check`は成功した。ローカル全target実行の一部localhost/TLS試験は別の実行でWindowsのsocket resetが発生しており、全target無失敗とは報告しない。Windows Actions再実行は修正commit作成後に手動dispatchし、その正確な結果を追記する。
- この記録は試験契約の是正に限られ、配置済みAgent Centerでの内容提示およびR2条件4の許可経路を実稼働で証明しない。`task_execution=unsupported`、R2 gate未完了、`release_ready=false`を維持する。
