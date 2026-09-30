# GUI Shell ロードマップ

状態: D4 Pocket / GUI-Shell統合rev3に基づく開発中。R0現行状態再固定とR1正本・Blocker体系再編を完了し、R2 Agent Task本番経路を進行中。Technical Completeおよび正式releaseは未成立。

## D4 Pocket統合 rev3 現行工程

D4 Pocket Phase 21／rev1 C11追補（2026-09-30）: 更新候補Contractを版2へ進め、Broker所有Ed25519署名の正本byteへ配布package全体のSHA-256と正確なbyte長を含める。旧版候補は保存記録として保持しつつ`legacy_unbound`へ降格表示し、新規受理・適用を拒否する。署名検査はpackage実byte取得・照合やdownload／install／rollbackを実装したことを意味しない。Windows配布blockerと`release_ready=false`を維持し、次は取得byte照合とsame-volume導入／rollback／crash Recoveryへ進む。詳細・検証結果は`docs/REV3_PROGRESS.md`、契約は`docs/specs/update-center.md`。

R2追補（2026-09-30）: Windows AdapterのWorkspaceWrite spawn経路で、Task scratch作成後かつCLI process生成直前にも登録Workspace／secret hardlink aliasを再検査する。追加したWindows fixtureは実spawn関数でCLI起動前の拒否を確認するが、外部同時変更との原子性、実MxC tool-child隔離、production Task経路の証拠ではない。`task_execution=unsupported`、Agent隔離等の`release_blocker`、`release_ready=false`を維持する。詳細と検証結果は`docs/REV3_PROGRESS.md`。

R2追補（2026-09-30）: Codex Adapter metadataの説明文にある通常語`permission`をAuthority field名として誤検出し、登録済みCodexとのAgent Session開始を`応答不正`で止めていたscannerを修正した。自由文では危険Authority値を引き続き拒否し、構造化objectのAuthority／Permission key拒否を維持する。Windows実Codex CLI登録を伴うBroker経路でSession開始後もWorkspace Permission／Owner Approvalの両grantを`AgentTask実行非対応`として拒否し、永続AuditとTask本文非露出を確認した。native Owner確認UI・実Agent Task・installed製品の証拠ではない。Rust全12 targetで407 passed／0 failed／3 ignored（主libは361 passed／0 failed／3 ignored）。初回はA2A loopback fixture 1件が間欠失敗し、単独再試験と全体再試験は成功した。`task_execution=unsupported`、関連`release_blocker`、`release_ready=false`を維持する。詳細は`docs/REV3_PROGRESS.md`。

R2追補（2026-09-30）: Desktop BrokerClientでAgent Task Workspace Permission／Owner Approval発行をnative Owner確認待ちoperationへ加え、5秒の通常timeoutから305秒の専用待ちへ修正した。これはFlutter応答待ちだけの変更で、native Owner確認・Broker authorityを代替せず、timeoutもBroker要求取消を意味しない。Adapterの`task_execution=unsupported`、Agent Task実行隔離blocker、`release_ready=false`は維持する。回帰testと検証は`docs/REV3_PROGRESS.md`、timeout意味契約は`docs/specs/agent-runtime.md`。

R2追補（2026-09-30）: OneDrive配下の長い日本語pathでAnalysis Serverが異常終了したため、一時`Z:` drive aliasから`flutter analyze --no-pub`を再実行し、`No issues found!`を確認した。aliasは解除済みで、元のRepository pathでの再発防止を意味しない。以前の失敗履歴と検証範囲は`docs/REV3_PROGRESS.md`。

R2追補（2026-09-29）: 登録secret pathのliteral denyが事前作成hardlink aliasから迂回された直接probeを受け、WorkspaceReader registrationとTask直前にhardlink／unsafe entryをfail-closedで検査する。実CLIの直接MxC sandbox childでは合成secretへの新規hardlink作成がWindows access deniedとなったが、production `codex exec` tool child／Broker／Owner Approval経路の証拠ではない。登録secret深度40と大文字・区切りaliasの直接probeも拒否した。`task_execution=unsupported`、Agent隔離等の`release_blocker`、`release_ready=false`を維持する。詳細は`docs/REV3_PROGRESS.md`と`docs/specs/agent-runtime.md`。

R2追補（2026-09-30）: 実Codex CLI `exec`とMxC shell tool childをloopback偽Responses APIから3回起動し、合成登録secretへのhardlink作成が3/3回Win32 NativeErrorCode 5（access denied）で失敗、alias不在、通常Workspace read／write許可を観測した。これは手動構成したRust相当profileによる直接`LIVE_RUNTIME`であり、Rust生成profile・Broker・Owner Approval・production Agent Taskを通した証拠ではない。TEMP／TMPとWorkspaceTaskScratchの不一致等も残るため、`task_execution=unsupported`と関連`release_blocker`、`release_ready=false`を維持する。詳細は`docs/REV3_PROGRESS.md`、`docs/specs/agent-runtime.md`、`release_blockers.registry.json`。

R2追補（2026-09-30）: 起動前に作成した合成secretのhardlink aliasは、実`codex exec`のMxC childから3/3回readできた一方、登録exact pathのreadは拒否され、別targetへの動的hardlink作成も3/3回access deniedとなった。これはRust WorkspaceReaderの複数link登録／Task直前検査が必要な理由を直接tool childで再確認した結果であり、probe自体はその登録preflightを意図的に迂回している。Rust Broker・Owner Approval・production Agent TaskのLIVE_RUNTIME証拠ではなく、`task_execution=unsupported`とrelease blockerを維持する。Responses stream decode failureを含む試験履歴と正確なscopeは`docs/REV3_PROGRESS.md`、`docs/specs/agent-runtime.md`、`release_blockers.registry.json`に記録する。

R2追補（2026-09-30）: Agent Task scratch journalの`activate`耐久保存失敗時にmemory stateを永続`reserved`と整合させ、Task未起動のscratch metadata／activation failureでは開いたdirectory handleを使って回収するfixtureを追加した。削除成功時だけ予約を完了し、回収不能ならjournalを保持する。証拠classは`FIXTURE`で、実Broker経由Agent Task、MxC TEMP／TMP管理、installed product Recoveryの証拠ではない。`task_execution=unsupported`とrelease blockerは維持する。詳細と試験結果は`docs/REV3_PROGRESS.md`。

R2追補（2026-09-30）: Rust公開APIの旧更新署名入口は署名値が空でないだけで成功していたため、Broker所有の信頼鍵・署名対象byteを受け取らない限り拒否するよう変更し、否定試験を追加した。実Broker経路のEd25519検証は維持する。これは安全な署名検証入口の補強であり、Download／Install／Update／Rollback経路や`rev2_desktop_product_distribution`のblockerを閉じない。検証記録は`docs/REV3_PROGRESS.md`。

R2追補（2026-09-30）: Windows Broker smoke collector v6は実Rust Broker processへ通常loopback credentialでAgent Task Workspace Permission／Owner Approvalの発行要求を送り、両方が`desktop_native_owner_confirmation_required`で拒否されることを実測・必須化した。現行Rust source commit `e8ea0587031402a19fc9dff260d7c925403cca2e`由来のstandalone Release helperでLIVE_RUNTIME成功したが、作業treeの文書／tooling差分を含む状態でのbuildであり、installed product、Desktop native Owner操作、Agent Task実行、release evidence provenanceではない。Task対応は`unsupported`、Windows installed-smoke blockerと`release_ready=false`を維持する。詳細は`docs/REV3_PROGRESS.md`および`release_blockers.registry.json`。

2026-09-29に受領したrev3統合仕様・工程表を現行開発基準とする。R0で再測定した基準commit、検証結果、環境差、formal evidence、既存blockerの一覧は[`docs/REV3_PROGRESS.md`](docs/REV3_PROGRESS.md)に追記する。旧rev1／rev2の進捗記録は履歴として保持し、rev3の完成証拠へ読み替えない。

rev3工程はR0からR16までを履歴追加型で進める。Windows 1.0のTechnical CompleteはR0–R14の技術工程で判定し、R15の非Windows工程およびR16のOwner Finalizationと混同しない。Owner操作を routine 検証の前提にせず、production identity／署名、production Audit key、Final GOはTechnical Complete後のOwner Finalizationに限定する。

`release_blockers.registry.json`は互換性のため全体の`classification: release_blocker`と`blocks_release`を保持する。原因は`cause_category`、対象は`release_tracks`で別に示し、Windows技術完成への影響は`blocks_windows_technical_complete`で独立判定する。Mobile実機・配布blockerをWindows 1.0 gateへ混ぜず、global `release_ready=false`も維持する。

~~~yaml
- item: Windows production distribution identity
  classification: release_blocker
  registry_id: windows_distribution_identity
  reason: 正式Publisher、Windows package／app identity、production code-signing identityはOwner決定を要する。これはR0–R14のTechnical Completeを止めず、正式配布を阻止する。
  required_action: R0–R14のTechnical Complete後にOwnerが正式identityを確定する。それまではtest identityまたはunsigned artifactで技術検証する。
  blocks_release: yes
~~~

プロジェクト: GUI Shell / Runtime Operation Shell / `LLM-readable application responsibility substrate`（LLM が読むアプリケーション責任基盤）
参照コンシューマー／Runtime: adapter のみを介した BLUE-TANUKI
主要実装経路: 権限に関わる本番収束は Flutter UI + Rust Security Broker。Rust helper は、権限の外側にある限定的な native 診断／操作に留める。

## C0 validation baseline履歴と最新Rust gate（2026-09-29）

2026-09-29、`main` commit `c770680e1868675d710657a11f300f9b38197578`を基点とする作業ツリーで、`python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`がexit 0となり、厳格日本語監査、Schema 149件／正常example 149件／negative fixture 192件、Conformance 225件を含む登録済み10検査が合格した。Evidence bundleはdevelopment evidenceとして合格し、release blocker 5件と`release_ready=false`を維持する。同commitへのWindows Actions #13ではRust全target check／testも成功したが、hosted Windows上のRust検査に限る。いずれの結果もrelease readinessへ昇格しない。個別commandと失敗履歴は`VALIDATION.txt`に記録する。

この基準後の`main`には、Codex Adapterの成功・期限超過後cleanupを偽CLIで検査する`FIXTURE` testを含むcommit `bd36dab45eb8da9d76caeebcd3a0b97fe29579b4`を統合した。作業treeでの統合開発検査はexit 0、Windows Actions #16は同commitに対して12 target／391 passedで成功した。これはCodex実Agent、Broker consumer経由起動、実sandbox隔離を証明せず、`task_execution=unsupported`とrelease blockerを維持する。

Windows Rust手動補助検証 [run #17](https://github.com/gatchimuchio/GUI-Shell/actions/runs/36475650538)は、一時branch上のRust source commit `6441ae8b2827d2afa01f9963ff2472fd2b889ce2`をWindows Server 2025 image `win25-vs2026/20260922.246.2`／Rust 1.95.0で検査した。checkout SHA、対象Rust fileのrustfmt、全target `cargo check`／`cargo test`（12 target、391 passed／0 failed／0 ignored）、試験後cleanが成功し、子孫process停止を検査する偽CLI fixtureも通過した。同じcommitを`main`へfast-forward pushし、remote HEAD一致後に一時branchをlocal／remoteから削除した。追加証拠は直接Adapter呼出しの`FIXTURE`であり、production Broker Task、実Agent sandbox／Workspace隔離を証明しない。ローカルの新fixture focused testはCLI probe起動前に失敗し、OS error番号は採取できなかった。`task_execution=unsupported`、Agent隔離等の`release_blocker`、`release_ready=false`を維持する。

履歴: 2026-09-28の基準更新では、変更前のclean `main` `31df760ae5085bf5ddc18c4f3eceb375cbae4f6c`にMANIFESTのhash不一致25件と必須source file欠落2件があったため、正規toolで1103件を再生成し、manifest checkとrelease gateを合格させた。この記録は当時の観測であり、現行`main`の状態を示すものではない。

## Windows Rust手動補助検証（2026-09-29追補）

Windows runnerでRustのbuild／test／検査が必要な作業では、`.github/workflows/windows-manual-rust-validation.yml`をownerが`workflow_dispatch`で起動できる。ローカルWindowsのApplication ControlがCargo生成executableをOS error 4551で拒否する場合も、端末保護を変更せず検証を継続する。選択ref上のRust 1.95.0に対して変更Rust fileの整形、Rust全targetのcheck／test、試験後作業treeのcleanを検査する。自動trigger、書込権限、artifact uploadは設けない。この結果はRunner上の対象commitに対するRust検査だけを示し、installed製品挙動、ローカルWindowsでの実行可否、release readinessを証明しない。現在sourceのRust結果は`docs/REV2_PROGRESS.md`と`release_blockers.registry.json`に対象commitと結合して記録し、未成功の全target試験を成功に読み替えない。

Windows runner初回試験ではsystem `TEMP`／`TMP`が`C:\Users\RUNNER~1\...`のDOS短縮名となり、Workspace rootの短縮名迂回防止規則がtest fixtureを拒否した。製品側のtilde拒否は維持し、workflowのRust検査step内だけ`TEMP`／`TMP`を`RUNNER_TEMP`配下の通常pathへ向ける。これはtest環境設定であり、製品rootの検査規則を緩和しない。

前回の手動run [Windows Rust manual validation #11](https://github.com/gatchimuchio/GUI-Shell/actions/runs/36424314077)は一時branch `codex/japanese-diagnostic`のcommit `84d402a930af48a6d29ce5075ba8692b02bddfd4`に対する検査履歴として保持する。Windows Server 2025／Rust 1.95.0、全target check／testと試験後clean確認が成功（12 target、389 passed）。

前回の手動run [Windows Rust manual validation #12](https://github.com/gatchimuchio/GUI-Shell/actions/runs/36433328263)は一時branch `codex/task-profile-boundary`のcommit `3bcec331f66ae9664ab8a86e6efe8558649f0e8a`をWindows Server 2025 image `win25-vs2026/20260922.246.2`／Rust 1.95.0で検査し、checkout SHA照合、変更Rust fileを含むrustfmt、全target check／test、試験後clean確認が成功した（12 target、389 passed／0 failed／0 ignored）。Rust test `TEMP`／`TMP`は`D:\a\_temp\gui-shell-test-temp`。この過去記録は当該commitの証拠として保持する。

最新の手動run [Windows Rust manual validation #13](https://github.com/gatchimuchio/GUI-Shell/actions/runs/36445136387)は`main`のcommit `c770680e1868675d710657a11f300f9b38197578`をWindows Server 2025 image `win25-vs2026/20260922.246.2`／Rust 1.95.0で検査し、checkout SHA照合、rustfmt、`cargo check --all-targets`、全target test（12 target、389 passed／0 failed／0 ignored）、試験後clean確認が成功した。これによりWindows Rust全target試験の実行gateは現行mainに対して解消した。これはhosted Windows Rust検査の補助証拠だけであり、installed product、実Agent Task、ローカルApplication Control、release readinessは証明しない。詳細は`docs/REV2_PROGRESS.md`と`VALIDATION.txt`。

## Phase 7 Codex Task mxc sandbox直接CLI probe（2026-09-29）

Codex Taskだけが使うWindows sandbox方式を`mxc`に固定し、同じ一つのfilesystem override tableに`:root=deny`を含めた。`codex sandbox`直接実行による合成path smokeでは、OneDrive `.env` ReparsePointと合成Workspace外pathの読取、任意外部pathへの書込が拒否され、Workspace通常fileおよびTask scratch相当pathの読書込は許可された。一方、`:minimal=read`の範囲では`C:/Windows/win.ini`が読めたため、全外部read denyとは主張しない。証拠は直接CLIと合成pathに限り、Rust Broker、`codex exec`、Owner Approval、実Agent Taskのproduction経路を通していない。Task実行能力は`unsupported`のまま、Agent隔離等の`release_blocker`と`release_ready=false`を維持する。対象commit、試験、手動Actions結果は`docs/REV2_PROGRESS.md`と`VALIDATION.txt`に記録する。

2026-09-29の追試では、同CLIのfilesystem tableを`:minimal=deny`へ変更しても`C:\Windows\win.ini`の`type`は成功し、同fileにexact denyを追加すると拒否された。この結果は当該一fileと直接`codex sandbox`呼出しだけの観測で、`:minimal` denialが他のpathに与える効果やBroker経由Taskの挙動を証明しない。`task_execution=unsupported`を維持する。

同日の候補検査で`C:/Windows="deny"`は`win.ini`を拒否しWorkspace fileを読めたが、同sandbox内の`codex --version`も起動拒否となったためTask profileへ採用しない。より狭い限定試験であり、Agent Taskの実行隔離は未成立のまま維持する。

対象Rust変更commit `8fba0e1ce5b0abc862b53cfbcbaf8885a4f07bb9`に対する手動Windows Actions [run #14](https://github.com/gatchimuchio/GUI-Shell/actions/runs/36453922841)は成功した。Windows Server 2025／Rust 1.95.0でcheckout SHA照合、対象Rust fileのrustfmt、全target `cargo check`／`cargo test`（12 target、389 passed／0 failed／0 ignored）、試験後clean確認が成功した。これは当該commitのhosted Windows Rust検査であり、実Agent Task、Broker隔離、ローカルApplication Controlまたはrelease readinessを証明しない。

最新のWindows Rust手動補助検証 [run #15](https://github.com/gatchimuchio/GUI-Shell/actions/runs/36465355560)はtemporary branch上のcommit `33787c35ea26bdbb2bae3ad0376bb5c8baa9792b`を検査した。Windows Server 2025／Rust 1.95.0でcheckout SHA照合、対象Rust fileのrustfmt、全target `cargo check`／`cargo test`（12 target、390 passed／0 failed／0 ignored）、試験後cleanが成功し、この同じcommitを`main`へfast-forward統合した。追加されたAgent Workspace binding／cross-agent試験は`FIXTURE`に限り、実Agent隔離・production Task実行を証明しない。ローカルApplication Controlと`task_execution=unsupported`、release blocker、`release_ready=false`は維持する。

最新のWindows Rust手動補助検証 [run #16](https://github.com/gatchimuchio/GUI-Shell/actions/runs/36471921842)は一時branch `codex/codex-adapter-fixture`上のcommit `bd36dab45eb8da9d76caeebcd3a0b97fe29579b4`を検査した。Windows Server 2025／Rust 1.95.0でcheckout SHA照合、Rust fileのrustfmt、全target `cargo check`／`cargo test`（12 target、391 passed／0 failed／0 ignored）、試験後cleanがすべて成功し、`main`へfast-forward統合後に一時branchをlocal／remoteから削除した。追加testは偽Codex CLI processを実Adapterへ接続する`FIXTURE`であり、実Codex Agent、Broker consumer、実Windows sandbox／filesystem隔離の証拠ではない。`task_execution=unsupported`、release blocker、`release_ready=false`を維持する。詳細は`docs/REV2_PROGRESS.md`と`VALIDATION.txt`。

2026-09-29の最新Rust生成profile probeでは、`build_codex_command`がTask用に設定するTEMP／TMPの値を直接`codex sandbox`起動へ渡し、mxc child側の値がWorkspaceTaskScratchと一致しないことを観測した。これは直接sandbox childの範囲に限られ、production `codex exec`やBroker Taskの実Agent tool childにおける一時領域と後始末は未検証である。scratchを有効な隔離境界と扱わず、`task_execution=unsupported`とrelease blockerを維持する。詳細と試験scopeは`docs/REV3_PROGRESS.md`および`release_blockers.registry.json`に追記する。

同日の明示実行development probeは実Codex CLI `exec`から偽Responses API経由で実MxC `exec_command` childを3回起動した。3/3回でTEMP／TMPは互いに一致する一方WorkspaceTaskScratchとは一致せず、MxC TEMP markerはtool child中に書け、CLI終了後はhostから見えなかった。Workspace scratch writeとCodex turn正常終端も3/3回成功した。これは直接CLI上の`LIVE_RUNTIME`に限る。host非可視は物理削除保証ではなく、Rust Broker／Owner Approval／production Task、cancel／timeout／crash cleanup、Audit／Recoveryの証拠ではない。Task capabilityとrelease blockerを維持する。

同probeをfilesystem境界検査へ広げた結果、`.env`等のdeny globは3/3回で読取を拒否したが、matching pathへの合成writeは3/3回で許可された。これはCodex CLIのglob denyがread専用である公式仕様と一致する。Owner登録secret pathからRustが生成するliteral exact denyでは、実Windows testが登録file／directory descendantのreadと新規writeを拒否した。通常Workspace writeは許可した。証拠は実CLI shell childとRust生成設定を直接sandboxへ渡した合成試験に限られ、Brokerからのproduction Task実行ではない。登録されていないalias path、TEMP/TMP mismatch、MxC temporary cleanup、process lifecycleは未解決のため`task_execution=unsupported`とrelease blockerを維持する。詳細は`docs/REV3_PROGRESS.md`および`release_blockers.registry.json`。

2026-09-29のWindows Rust縦断fixture追補では、Broker `AgentTask取消`から実`CodexCliAdapter`実装・fake CLI process tree停止・terminal `cancelled`・WorkspaceTaskScratch cleanupまでを接続した。稼働中の子孫heartbeatがterminal後に増えず、取消中は`running`を保ち、result hashとTask本文／試験pathは監査・状態へ出ないことを確認した。Rust全targetは395 passed／0 failed／1 ignored。証拠はfake CLI・試験Brokerによる`FIXTURE`に限り、production `codex exec`、実Agent隔離、永続Audit／Recovery、installed productを証明しない。`task_execution=unsupported`、release blocker、`release_ready=false`を維持する。詳細は`docs/REV3_PROGRESS.md`の最新R2追補。

同日のOwner Approval期限否定試験は、wall-clockとmonotonic期限を個別に失効させ、Workspace Permissionが有効でもBroker実行を拒否し、Adapterを呼ばずPermissionを保持する2 fixtureを確認した。focused試験は成功した一方、Windows local全targetは通信系testの間欠失敗により2回とも完走PASSしていない。MINIDORAとA2Aの該当試験は個別再実行で成功した。対象commitを固定する手動Windows Actionsで全targetを確認する。これは`FIXTURE`であり、実Agent Taskやrelease readinessの証拠ではない。`task_execution=unsupported`と`release_ready=false`を維持する。検査と失敗履歴は`docs/REV3_PROGRESS.md`の最新追補に記録する。

追補: 直前記載のOwner Approval期限否定試験に対するWindows Actions [run #20](https://github.com/gatchimuchio/GUI-Shell/actions/runs/36548519704)は、正確なcommit `95921dc549abb7d74386f45dadd98f8478d47b68`をWindows Server 2025／Rust 1.95.0で検査し、rustfmt、全target check／test（12 target、396 passed／0 failed／1 ignored）、試験後cleanが成功した。証拠は`FIXTURE`とhosted Windows Rust検査に限られ、実Agent Taskやrelease readinessを証明しない。`task_execution=unsupported`と`release_ready=false`を維持する。詳細とlocal失敗履歴は`docs/REV3_PROGRESS.md`の最新R2追補に記録する。

続く対称negative fixtureでは、Owner Approvalの本文・条件・両期限が有効でも、Workspace Permissionのwall-clock期限またはmonotonic期限が切れていればTaskを拒否し、Adapterを呼ばずgrantを保持することを確認した。focused試験と全target compileは成功し、対象Rust commit上のhosted全target試験を実行する。`FIXTURE`の局所証拠であり、`task_execution=unsupported`と`release_ready=false`を維持する。完了結果は`docs/REV3_PROGRESS.md`の最新R2追補に記録する。

追補: Windows Actions [run #21](https://github.com/gatchimuchio/GUI-Shell/actions/runs/36552280495)は一時branch上のcommit `dc2ec0960421442f34e3c229451b1f4a76c105f5`をWindows Server 2025／Rust 1.95.0で検査し、checkout SHA照合、対象Rust fileのrustfmt、全target cargo check／test（12 target、397 passed／0 failed／1 ignored）、試験後cleanに成功した。所要6分24秒、artifactなし。PASSした同一SHAのみを`main`へfast-forward・pushし、remote HEADを照合後、一時branchをlocal／remote双方から削除した。局所`FIXTURE`／hosted Rust証拠の範囲を超えて実Agent Taskやrelease readinessを証明せず、`task_execution=unsupported`と`release_ready=false`を維持する。詳細は`docs/REV3_PROGRESS.md`と`release_blockers.registry.json`。

Rust Desktop起動器経由でOwnerがAgent Task Workspace Permission／Owner Approvalを拒否した場合、Owner専用IPCへ進まず通常Broker経路で`desktop_native_owner_confirmation_required`として拒否されることをloopback fixtureで検査した。合成確認callbackによる`FIXTURE`であり、実Owner dialogやproduction installed pathではない。Windows Actions [run #22](https://github.com/gatchimuchio/GUI-Shell/actions/runs/36556239415)と統合validatorは成功し、`main`反映後に一時branchを削除した。詳細、正確な試験範囲、整形検査の制約は`docs/REV3_PROGRESS.md`の最新追補へ記録する。

追補: native確認callbackが肯定でも、Task Permission／Owner ApprovalがOwner専用process内IPCへ進んだ後、未登録Workspaceを実Brokerが`作業領域不在`で拒否するloopback fixtureを追加した。Task本文のsummary／response／Audit非露出も検査する。合成callbackと未登録Workspaceによる`FIXTURE`であり、実Owner dialogやgrant成功の証拠ではない。統合validatorは開発検査10件、strict Japanese audit 1113 file／指摘0、Schema 149/149・negative 192、Conformance 225が成功した。Windows Actions [run #23](https://github.com/gatchimuchio/GUI-Shell/actions/runs/36558967124)は一時branchのcommit `8b31153952d077ef993f46c1e3adc7f9b5189419`に対しWindows Server 2025／Rust 1.95.0で12 test target、397 passed／0 failed／1 ignored、workflow固定rustfmt、checkout SHA照合、試験後cleanに成功した。PASSした同一SHAを`main`へfast-forward・pushし、remote HEADを確認後、一時branchを削除した。`desktop_launcher.rs`はworkflow固定rustfmt対象外で、追加範囲の局所確認を行った。証拠はhosted Rust検査とfixtureに限り、実Owner dialog、実Agent Task、grant成功、release readinessを示さない。`task_execution=unsupported`と`release_ready=false`を維持する。

## Windows Desktop Flutter手動補助検証（2026-09-28、09-29追補）

OneDrive外のmanaged worktreeでも、Desktop Flutter全testはRust helper未buildを前提にする2件が失敗した。helperを同worktreeでbuildすると、Cargo build script executableがWindows Application ControlのOS error 4551で拒否された。Desktop／Mobileの`flutter analyze`もanalysis serverが途中で切れたJSON応答を受けてexit 1となった。これらの端末保護・test前提を緩和せず検証するため、`.github/workflows/windows-manual-desktop-flutter-validation.yml`を`workflow_dispatch`限定で追加した。Windows Actions [run #1](https://github.com/gatchimuchio/GUI-Shell/actions/runs/36440922358)は一時branch `codex/global-search-provenance`のcommit `33d0287b861f500d636904e4a2a0edfbeea576eb`をWindows Server 2025 image `win25-vs2026/20260922.246.2`で検査し、checkout SHA照合、Flutter 3.44.0固定commit `559ffa3f75e7402d65a8def9c28389a9b2e6fe42`、Rust helper build、Desktop `flutter analyze`（問題なし）／全test（125 passed）、Mobile `flutter analyze`（問題なし）、試験後clean確認がすべて成功した。所要6分49秒、artifactなし。これはhosted Windows上の指定commitに対するbuild／test補助証拠であり、ローカルApplication Control、installed product、実Agent隔離、release readinessを証明しない。端末保護設定とrelease blockerは変更していない。詳細は`docs/REV2_PROGRESS.md`と`VALIDATION.txt`。

## Phase 7 Codex Task permission profileの現行観測（2026-09-28）

Codex Task filesystem overrideを単一tableへ統合し、`glob_scan_max_depth=8`が後続設定に置き換えられないことをConformanceで固定した。Windows sandbox helperの最新合成marker観測では、通常NTFSのWorkspace内`.env`読取とWorkspace外書込は拒否、Workspace内書込は許可されたが、UserProfile／LocalAppData内の外部readとOneDrive `ReparsePoint` `.env`読取は許可された。`:root=deny`は現行Windows helperのelevated／unelevated両backendで起動不可だった。手動Windows Actions run #12はRust全target検査に成功したが、この結果はhelper境界の欠陥や実Broker Task経路を解消しない。従って広域read隔離は未成立であり、Adapterの`task_execution=unsupported`を維持する。helper検査は実Broker Agent Taskではなく、Windows Application Control等の保護設定も変更していない。

## 現行D4 Pocket統合単位（2026-09-28）

以下の最新追補を現況正本とし、それより後ろの同日付Phase 7記録は各作業時点の履歴として保持する。旧記録にある「Task未接続」は、ここに記載する固定Adapter実装前の状態を示す。

### R2追補 Codex CLIの非Git登録Workspace起動（2026-09-29）

実Codex CLIはGit管理外Workspaceを`--skip-git-repo-check`なしで拒否するため、read-only DialogueとTaskが共用する`exec` command builder／CLI interface検査へ固定optionを追加する。これはCLI自身のGit repository安全確認を迂回するため、Dialogueのread-only sandbox、Taskの登録Workspace・Broker Session／Permission／Owner Approval検査を維持する。非対応CLIはfail-closedにする。直接CLIのloopback probeは実行可能性だけを示し、実Broker Task／隔離を証明しない。`task_execution=unsupported`、`release_ready=false`および既存blockerを維持する。詳細・公式根拠・検証履歴は`docs/specs/agent-runtime.md`および`docs/REV3_PROGRESS.md`を参照する。

### R2追補 現行RustブローカーのRelease実測（2026-09-29）

現行main `75412301dfd7c17c99237fb795b894ffde119b5f`から独立したCargo出力先へRelease helperをbuildし、collector version 5で認証済みプロセス間通信・永続store準備・再起動後のnonce再利用拒否・新nonceのhealth受理・強制終了後の接続拒否・一時session資格file削除を局所`LIVE_RUNTIME`として確認した。helperのhashはblocker registryへ記録した。この検査はhelper単体の直接起動であり、正式Windows配布物とのbinary identityは未確認のため、正式証拠bundleと`windows_broker_installed_smoke` gateはunresolvedのまま。詳細は`docs/REV3_PROGRESS.md`と`release_blockers.registry.json`。

### C9 MCP stdio監督とOwner切断（2026-09-28）

MCP stdio childは既存Rust Job Object監督経路で起動し、Desktop設定にmetadata-only接続一覧、新規stdio接続設定、Owner専用切断を実装した。接続開始・切断・Tool呼出しは既存Windows Rust起動器のdefault No native Owner確認を経てBrokerへ届き、Brokerが現在Catalogと要求を再検証する。接続は固定missing Credential refまたは、通常IPCの検証付き一覧で対象Server・用途が一致したCredential IDと環境変数名だけを受け付ける。Brokerは永続登録Audit／DPAPI保管hash／targetを照合し、Owner確認後に選択値を対象stdio childだけへ渡し、使用Auditを記録する。確認画面はServer processが秘密値を読取り・外部送信でき、Job Objectはsandboxではないことを示す。Tool呼出しは操作者が画面上でJSON arguments全文を確認した後に限り、一回限りPermissionを消費して同一stdio childへ`tools/call`を一度だけ送る。結果本文を保存・表示せずhash-only receiptを返し、応答不明・不正・Audit失敗は接続をquarantineして自動再送しない。切断はprocess群停止・永続`LIVE_RUNTIME` Audit後に記録解消する。2026-07-28形式、legacy fallback条件、256 KiB逐次stdout上限、Tool inputSchema Draft 2020-12検証を維持する。Tool一覧等はmetadata-onlyで、Resource／Prompt本文取得を行わない。この単位はOwner向けWindows stdio操作経路であり、Agentへの結果引渡し、MCP以外のCredential実値注入、Resource／Prompt内容取得、Streamable HTTP、OAuth、外部MCP Server適合、installed product Credential運用証拠、非Windows process群監督は未接続の`release_blocker`。Rust／Flutterのローカル検証と手動Windows Actionsの対象commit・結果は`docs/REV2_PROGRESS.md`に記録する。ActionsはRust check/testの補助証拠に限り、Windows installed product、外部Server、release readinessを証明しない。`release_ready=false`を維持する。

### Phase 18／C29 MCP stdio failure fixtureの現行Contract同期（2026-09-28）

現行Brokerへ接続する開発用障害注入器のCredential refが旧値のままで、MCP timeoutとCredential未設定の試験はどちらも`mcp_credential_ref_invalid`で早期終了していた。refを`purpose=mcp_transport`・対象Server IDへ同期し、timeout fixtureが実起動したmarker、Draft 7 inputSchemaの拒否、およびrequired/missing Credentialでprocess未起動を確認する3ケースへ修正した。`python -X utf8 tooling/failure_injection_validation.py`は全9ケース成功。これは一時fake stdio Serverを使う開発Broker／FIXTURE証拠であり、HTTP／OAuth harness、外部MCP Server適合、installed product、C9全体の完成を証明しない。MCP HarnessとAgent結果handoff等の`release_blocker`、`release_ready=false`を保持する。詳細は`docs/REV2_PROGRESS.md`の本節を参照する。

### Phase 11 Mobile資源観測の表示・要求を選択中に限定（2026-09-28）

`IndexedStack`内の非表示Mobile資源画面がbuild時にBroker観測を開始していたため、選択中・接続中だけ一度観測し、離脱・接続変更で表示と遅延結果を無効化する。複数Runtimeは逐次最大16件、同時1件とし、再入場後に新規観測する。非OneDrive managed worktreeでMobile test全20件が成功した。Desktop/Mobileの`flutter analyze`はLSP初期化JSON欠損によるanalysis server異常で終了し、成功扱いしない。実機・Device Link統合・release evidenceは未成立。詳細は`docs/REV2_PROGRESS.md`の本項を参照。

### Phase 11追補 Mobile Runtime scope変更時の要求直列化（2026-09-28）

遅延応答中にRuntime一覧が変わる場合も、旧要求完了前に新要求をBrokerへ送らない直列化を追加した。非OneDrive managed worktreeでfocused test 3件、Mobile全体21件が成功。Desktop/Mobileの`flutter analyze`はLSP初期化JSON切れでexit 1。現行release blockerと実機証拠境界を維持する。詳細は`docs/REV2_PROGRESS.md`の最新追補を参照。

### Phase 9 実行履歴の期限監視を期限駆動へ変更（2026-09-28）

履歴画面の100ms周期監視を廃止し、承認の壁時計・単調時計の早い方で一度限りの期限timerを設定する。2秒ごとの承認／履歴再確認と失効時の表示消去は維持し、権限・承認意味は変更しない。変更前commit `686a2a4e3e043d79e07b6d2bca248bcfccdfd18a`を基点にした非OneDrive managed worktreeで対象Flutter test 18件成功。元OneDrive checkoutではephemeral `.packages`への削除アクセス拒否、両appのFlutter/Dart analyzeは環境側analysis server異常で終了したため、これらを成功扱いしない。詳細は`docs/REV2_PROGRESS.md`の本項を参照する。Phase 9の他の未実装範囲と既存release blockerは維持し、`release_ready=false`。

### Phase 7 Task用Windows sandbox方式の明示（2026-09-28）

Codex Task commandは`--ignore-user-config`によりuser configを読み込まないため、Task専用`-c windows.sandbox="elevated"` overrideを追加し、Task時だけ強いWindows sandbox方式を明示する。read-only Dialogueへのoverride追加や一般権限の拡張は行わない。これは方式選択の固定であり、deny-read ACLの実効、外部path拒否、`codex exec`実Agentを証明しない。helperでの合成`.env`読取成功・外部path deny失敗を踏まえ、`task_execution=unsupported`とrelease blockerは維持する。詳細は`docs/REV2_PROGRESS.md`の最新Phase 7追補を参照する。

### Phase 7 Broker crash後のAgent Task scratch回復（2026-09-28）

Agent Task専用scratchを、作成前にBroker永続storeへ予約し、nofollow open後のroot／scratch identity取得を経て`active`へ進めるbounded HMAC-authenticated journalを追加した。再起動ごとに変わる権限用registration hashとは分離した安定`recovery_binding_hash`を、Runtime／Workspace ID・除外指定・root／祖先identityからBrokerが再計算する。起動時はWorkspace登録後・IPC listener開始前に、現在のbindingとidentityがすべて一致する直接子scratchだけをhandle経由で削除し、欠損pathは記録のみ解消する。予約だけで実体identityがないpath、別登録、reparse、identity不一致、破損journalは削除せず保持し、未解決WorkspaceのTaskを拒否する。本文、絶対path、Credential、Permission、Approval、出力はjournalへ保存しない。

回復契約、Schema、否定用例およびRust試験を追加した。`Broker`の別instanceを同一の永続storeから再生成し、実`Workspace`起動登録経路でscratch削除・journal解消・回復`Audit`を確認する試験と、IPC listener受付開始前に回復処理が行われることを確認する適合検査を追加した。スキーマ145件、正常例145件、否定用例179件、適合検査225件、日本語厳格監査、開発検証10項目、`cargo check --all-targets`は成功。最新Rust全対象試験は合計366件（単体321件、`CLI` 9件、`IPC` 10件、結合試験26件、Desktop起動器0件）すべて成功した。先行実行で発生した`Windows Application Control`の`OS error 4551`による起動拒否は、最新コードの実行では再現せず、この検証関門を解消した。過去の拒否記録は履歴に保持する。`task_execution=unsupported`、有効な未解決release blocker 14件、`release_ready=false`を維持する。この回復試験は同一試験プロセス内で`Broker`を再生成したもので、Windows上のBrokerプロセス強制終了／電源断後の`LIVE_RUNTIME`回復証拠ではない。Agent Taskの製品環境での一時領域後始末保証は主張しない。

追補: Rust test harnessの子processでBroker libraryと永続storeを起動し、scratchをjournalへ有効化後、親からそのOS processを強制終了する試験を追加した。親processが新BrokerのWorkspace起動登録を通じてscratch削除・journal解消・回復Auditを確認し、focused試験1件と全target 366件が成功した。これはprocess-boundaryを含む`FIXTURE`証拠であり、production `broker-server`／IPC、実Agent Task、電源断後の製品回復を証明しない。旧試験の同一process内再生成記録は履歴として保持する。詳細は`docs/REV2_PROGRESS.md`の最新Phase 7追補を参照する。

### Phase 7 Windows permission profile deny-read実測失敗（2026-09-28）

Codex sandbox helperへ`d4p-agent-task` profileを明示し、非秘密の合成`probe.env`を読む負例を、OneDrive配下と非OneDriveの短いmanaged worktreeの両方で実行した。どちらもmarker本文が読めた。非OneDrive fileのACLには`CodexSandboxUsers`の継承`Modify`／`Read & execute`許可より後に`DENY(Read)`があり、読取denyは実効していない。OneDrive側fileはCloud Filesの`ReparsePoint`でもあり、別の証拠境界として扱う。

`C:\Windows\win.ini`へのexact denyはhelperが`apply deny-read ACLs`で失敗した。従って、現環境でpermission profileが秘密file読取を防ぐとは主張できず、実Codex Taskへは使用しない。`task_execution=unsupported`を維持し、deny ACL生成・適用順序と非OneDrive／OneDrive双方のLIVE_RUNTIME負例を解決するまで本項目を`release_blocker`とする。詳細は`docs/REV2_PROGRESS.md`の本節。

### Phase 7 Codex Agent Task専用permission profile（2026-09-28）

実インストール済みCodex CLI `0.158.0-alpha.2.1`の`version`／`exec --help`を確認し、read-only Dialogueは従来どおり`--sandbox read-only`、Taskだけは固定`d4p-agent-task` permission profileで起動する構成にした。profileは`:workspace`を継承し、glob走査深度8までのWorkspace内`**/*.env`、`**/.ssh/**`、`**/secrets/**`をdenyし、networkを無効にする。深度8超や別名の秘密fileを包括的に拒否するものではない。JSONL応答上限、Task専用Workspace内TEMP／TMP scratch、Windows Job Objectによるprocess群監督と通常終端cleanupは維持する。Task専用profileの構成testは追加したが、実Codex `exec`でdenyを試していない。

Task runnerとBroker Consumerは接続済みでも、Adapter capability metadataは`task_execution=unsupported`のままであり、Brokerから実Taskを起動できない。permission profileを指定したsandbox helperから`C:\Windows\win.ini`を読み取れたため、外部path拒否は未成立である。Workspace内deny globの実効性、Broker crash／電源断後のscratch回収、実Agent隔離書込、失敗／取消／期限の実runtime証拠、結果／diffの`Content Exposure`表示は未成立の`release_blocker`。有料資格によるModel実行は行っていない。Windows Job Objectはprocess群停止でありfilesystem sandboxではない。

- `cargo test --locked --manifest-path native/rust_helper/Cargo.toml --lib Dialogueはread_onlyのままTaskだけ専用permission_profileを使う -- --test-threads=1`：対象のRust単体試験1件が成功。`cargo check --locked --manifest-path native/rust_helper/Cargo.toml --all-targets`成功。最新の`--all-targets`実行はRustライブラリ試験313件、`CLI`試験9件、`Broker IPC`試験10件の後、`canonical_decimal_hash`の結合試験実行ファイルがWindowsの`Application Control`にOSエラー4551で起動前に拒否され、終了値1。全対象試験成功とは記録しない。

### R2追補 Workspace登録secret pathのTask sandbox伝播（2026-09-29）

Workspace登録時に保持する`secret_paths`をBrokerの`DialogueWorkspaceBinding`からTask専用の揮発contextへ渡し、Codex `workspace-write`のfilesystem overrideへ完全一致と子孫globのdenyとして決定的に射影する。pathはWorkspace相対形式を再検証し、glob metacharacterのうち登録可能な`[`、`]`、`{`、`}`はliteral escapeする。登録数256件・生成設定12 KiBを超える場合は起動前に拒否する。既存の`.env`等のdeny、`:root=deny`、`:minimal=read`、glob走査深度32、network無効は維持する。secret path名はTask contextのDebugと回復journalへ含めない。

ローカルmxc直接sandbox probeでは、候補規則の手動設定で登録した合成secretの完全一致・子孫および深さ64のpathが拒否され、隣接decoyは読めた。`[]`／`{}`を含むliteral pathも拒否対象とし、glob wildcard扱いのdecoyは許可された。これは手動設定を用いたCLI sandboxの限定`LIVE_RUNTIME`証拠であり、Rust生成値、`codex exec`、Broker、Owner Approval、実Agent Taskは通していない。Rust全target 394件、Conformance 225件、strict日本語監査1111 files／findings 0、および統合validatorの登録済みdevelopment check 10件はすべて成功した。global `release_ready=false`、既存release blocker、Codex Taskのunsupportedは維持する。

追加した明示実行Windows Rust probeは、Rust command builder生成のTask `-c`設定を実Codex `codex sandbox --permission-profile`へ渡し、合成登録pathの完全一致・子孫・literal bracket／braceを拒否し、decoy読取とWorkspace writeを確認した。これはdirect sandboxの限定`LIVE_RUNTIME`証拠で、`codex exec`、Broker、Owner Approval、TEMP scratch伝播は検証しない。詳細と初回helper呼出しの失敗履歴は`docs/REV3_PROGRESS.md`および`docs/specs/agent-runtime.md`へ記録し、Task能力は`unsupported`のまま保つ。
- その時点では、現行source commit `f4bf6b0e65e751da8b662595f2e9865970c6f460`を非OneDriveの短いmanaged worktreeから`cargo test --locked --manifest-path native/rust_helper/Cargo.toml --all-targets -- --test-threads=1`で再検証したが、`io-lifetimes` build scriptがOS error 4551で起動前に拒否された。さらに`--target-dir C:\\D4Pocket\\codex-f4-target`を指定した同全target試験でも`generic-array`／`io-extras` build scriptが同errorで拒否された。どちらもtest executableまで到達せず、Cargo出力先の変更では解消しなかった。Application Control変更・test除外・拒否file移動は行わず、その時点で`windows_rust_integration_test_execution_policy`を未解決へ戻した。この判断は後続の現行mainに対するrun #13で更新された。
- `python -X utf8 tooling/validate_all.py --python-only --desktop-platform windows`：exit 0。strict日本語監査、Schema 144／144／178、Conformance 224、Manifest、release gate、packaging portability、release smoke等10検査が合格。development validatorの結果は`release_ready=false`、`release_blocker` 31件であり、正式releaseを意味しない。
- 詳細な実装範囲・検証・証拠限界は`docs/REV2_PROGRESS.md`の本節、契約は`docs/specs/agent-runtime.md`および`docs/specs/process-supervision.md`を参照する。

### Phase 7 Windows Codex process群の終了管理（2026-09-28）

既存Codex read-only対話processは、Windows上で起動threadを停止中に専用Job Objectへ割り当て、Job handle close時の全process終了を有効にしてから再開する。取消・期限超過・root終了後の残存processをJob単位で停止・確認し、pipe readerを解放する。Win32 unsafeは明示レビュー契約付きの独立process-supervision crate内へ分離し、Conformanceで一file境界と根拠数を検査する。強制Broker終了を使うWindows実process試験とRust全target 355件が成功した。

この基盤は既存read-only対話Adapterのみに接続し、書込Task経路は未接続のまま`task_execution=unsupported`を維持する。Adapter側取消接続の実process試験とRust全target 354件が成功した。Task向けprocess supervision、Codex `workspace-write`、Workspace scratch隔離・cleanup、実Agent隔離書込と失敗注入は`release_blocker`。Job Objectはsandboxの証拠ではなく、`release_ready=false`を維持する。

### Phase 7 Agent Task Broker Consumerの一回消費・bounded状態（2026-09-28）

Rust Brokerに独立した`AgentTask実行`／`AgentTask状態`／`AgentTask取消`を接続した。実行要求ごとにmetadataとAdapter実装双方の対応、現行Session、Workspace登録hash・固定root identity、Permission／Owner Approvalの本文hash・条件hash・wall／monotonic期限を再照合し、開始Audit確定後にPermissionとApprovalを同じBroker排他区間で一回消費してworkerを開始する。Task状態は独立projectionで、出力本文を保存せずBroker側でhash化し、同一Session／Workspaceの並行Task、全体同時4件、状態履歴128件へboundedとした。取消応答はworker停止を保証せず、terminal応答までrunningを維持する。

Codex Adapterは引き続き`task_execution=unsupported`であり、このConsumerから起動されない。実Codex `workspace-write`、Workspace内TEMP／TMP scratch隔離・cleanup、process-tree終了、実Agentの隔離書込、結果/diff表示・LIVE_RUNTIME失敗試験は`release_blocker`。Rust `cargo check --all-targets`と、短縮checkout上の全target 353件は成功した。以前の実行でWindows Application ControlのOS error 4551により起動拒否された履歴は進捗記録に残すが、現行のRust試験結果とは区別する。Conformance・Schema・manifest・strict日本語監査の最新結果は作業進捗へ記録し、未実行testを成功に昇格しない。

### Phase 7 Agent Task未対応Runtimeへの権限発行拒否（2026-09-28）

Rust BrokerのTask要求検査、Task Workspace Permission発行、Task Owner Approval発行は、現在の構造検査済みAdapter metadataに`task_execution=supported`があり、Adapterが固定したWorkspace device/file IDがBroker登録rootと一致する場合だけ続行する。capability欠落・`unknown`・`unsupported`、root識別子の欠落・不一致はfail-closedに拒否する。metadataはPermission／Approvalを生成せず、root一致もsandbox実効性や実行証拠へ昇格しない。Codex Adapterは引き続きunsupportedで、実Task Consumer、隔離Workspace、実行前後Audit／Recovery、結果/diffは`release_blocker`。試験はRust Broker内の試験Adapterに限定し、`FIXTURE`境界を維持する。

### Phase 7 Agent対話SessionとWorkspace登録の明示結合・Mobile選択経路（2026-09-27）

Agent Adapterの対話開始にWorkspace IDの明示選択を必須化し、Rust Brokerが現在登録中のWorkspace IDとRuntime IDを照合してからSessionを作る。Desktop対話面は既存のBroker Workspace一覧から同一RuntimeのIDを選び、通常要求へ渡す。作成Audit hashの既存射影は維持し、Session・Runtime・Workspace・登録hashの関係を別AuditEventへ記録する。通常Desktop一覧は作成Audit参照とWorkspace結合Audit参照を区別して返し、UIはBroker登録のmetadata対応として表示する。MobileはDevice Link TLS上の既存Rust Broker Workspace一覧handlerを利用し、応答をWorkspace ID／Runtime IDだけへ限定して対話画面の選択に使う。Desktop Agent Centerはproduct modeでBroker session metadataだけを表示し、local／mock fixtureのTask・diff・Tool・commandを実結果として表示しない。Handoffは未接続の固定状態とし、regex redactionから公開概要を合成しない。Agent専用実行Session、実際の作業directory、書込み隔離、Task実行、比較・Handoffは未成立で、実Agent実行、独立Workspace隔離、実機TLS統合を`release_blocker`として維持し、`release_ready=false`を保つ。

Phase 7のTask要求境界は`agent_task_request.schema.json`をRust Brokerの`Agent作業要求検査`へ接続し、現在Agent Adapter・利用中Session・Session作成時と同じWorkspace登録hashを照合して、指示本文を返さずBroker計算hashだけを返す。Task検査とPermission／Approval発行はAdapterの固定root識別子とBroker登録Workspaceの物理root識別子も照合し、欠落・不一致を拒否する。Permission状態はBroker内の現行grantと二つの期限、Session／Workspace登録hashを再照合して示し、Permission IDは露出しない。別のowner-native confirmation経路はRuntime／Session／Workspaceに結合したTask用Workspace Permissionを5分・1回で発行し、通常IPC、request由来の権限値、重複active grantを拒否してAuditする。追加の`AgentTaskOwnerApprovalGrant`はRust Desktop native確認だけで受け付け、本文hashとWorkspace登録・Permission内部識別子・固定実行条件policyからBrokerが計算した条件hashへ結合した揮発Approvalを5分発行する。Task preflightは本文・条件・二つの期限を再照合する。発行receiptは専用Schema・fixture・negative Conformanceに固定した。Session隔離やWorkspace Permission置換ではApprovalも破棄される。四つのAgent Broker操作はIPC要求／応答SchemaとConformanceで同期を検査する。このroot照合は登録関係の内部状態検査であり、sandbox・隔離書込実行の証明ではない。この段落はconsumer実装前の2026-09-27時点の履歴であり、現在は冒頭の2026-09-28追補でBroker consumerを追加した。sandboxの実証、実行前後Audit／Recovery、隔離書込実行・結果保存はなお未成立。Codex Adapterはread-only、実Agent Task／書込み隔離は`release_blocker`のまま。

### Phase 7 Agent対話セッションmetadata投影の先行単位（2026-09-27）

先行単位ではRust Brokerの通常認証IPCに`対話セッション一覧`を追加し、Schema適合Agent Adapterの現在sessionからmetadataを上限64件でDesktopへ投影した。今回の追補で、Workspace IDと結合監査参照を追加する統治経路へ進んだ。Agent metadataは分類に限り、AuthorityやTrustを与えない。証拠は`INTERNAL_STATE`であり、実Agent稼働・Workspace隔離・Task実行を示さない。以前の検証失敗・回復と証拠範囲は`docs/REV2_PROGRESS.md`の該当履歴に保持する。

### Phase 7 Agent間Workspace root範囲重複の拒否（2026-09-27）

Rust Workspace registryは、異なるRuntime ID間の同一物理rootと通常pathで観測できる親子rootの重複をfail-closedで拒否する。起動時にnofollowで開いたdirectory identity列を二度のpath解決で照合し、負例は親→子・子→親の両順、識別範囲不明のhandle-only登録、Broker拒否Auditを確認する。独立rootは登録できる。証拠は一時directoryと試験Brokerによる`FIXTURE`であり、bind mount等の別名範囲や実Agentの同時書込み・比較・Handoff隔離を示さない。比較は未接続のまま維持する。仕様と残存境界は`docs/specs/workspace-inspection.md`、`docs/specs/agent-coordination.md`、`docs/REV2_PROGRESS.md`を参照する。

同Phaseの先行単位として、owner起動設定内のCodex runtimeと同じruntime IDを持つWorkspace rootを、Codex Adapterの固定作業pathと物理directory identityで照合する。不一致や識別不能rootはRuntime probe前に拒否する。Agent対話Session開始時には現在登録Workspaceを明示選択してID対応を結合する。後続の追補で通常Windows NTFS pathのtask spawn中に限る差し替え対策を加えたが、Unixのraceとmount等の別名範囲、Agent専用実行Session、実書込み隔離、Agent比較・Handoff・cross-agent isolationは未成立である。詳細と検査証拠は`docs/REV2_PROGRESS.md`の対応追補を参照する。

Phase 7のpath差替え対策追補では、Codex Adapter登録時のWorkspace directory identityをtask spawn直前に再照合し、Windowsではnofollowで開いたvolume rootからWorkspaceまでのdirectory handleをprocess spawn完了まで保持する。通常のNTFS path上でのrename／delete競合を防ぐ範囲に限る。Unixのcheck-to-spawn race、mount等の別名範囲、実Agent隔離は未成立で、`comprehensive_extension_rev1_completion`のrelease blockerを維持する。試験証拠・制約は`docs/REV2_PROGRESS.md`に記録する。

### Phase 32 Windows Export build tool追加（2026-09-26）

Brokerが生成するReceipt／ManifestをSchema・hash・byte長・identityで再照合し、cleanでremote `main`と一致するcommitからWindows Flutter ReleaseとRust Broker／起動器を組み立てる開発専用toolを追加した。最初のclean commit実構築ではFlutter Windows Releaseが成功した一方、Cargo linkerの出力pathが260文字となり、LNK1104で失敗した。短縮path修正後の再実行ではFlutter Windows Releaseが成功し、Cargoは`proc-macro2` build scriptがWindows Application ControlのOS error 4551で拒否されたため停止した。portable bundleは未生成であり、host policyを弱めずCargo構築を完遂できるWindows環境で再検証する。Owner／source authority、Credential artifact scan、runtime Manifest消費、binary pruning、製品起動、Installer／署名／配布は証明せず、release blockerを維持する。詳細は`docs/specs/gui-shell-export.md`と`docs/REV2_PROGRESS.md`を参照する。

Mobileはrev2の現行対象であり、旧v1 post scopeへ退避しない。Device Linkの資格・暗号通信処理はAndroidのKotlin実装／iOSのSwift実装へ移し、Flutterとの接続口は固定要求と安全な状態表示に限定する。通信不能時の「端末内削除」は資格削除と同じOS保護状態に最大32件の秘密なし回復記録を保存し、読み取り専用方式で限定表示する。この記録は`INTERNAL_STATE`であり、Desktop Brokerの監査連鎖・失効・操作者本人性を証明しない。Flutterの16試験・解析、日本語／Schema／conformance／配布互換性を含むPython統合検証は成功した。Androidの現行sourceと共有UI 117 fileはfresh Temp copyとSHA-256で一致を確認し、cleanup除外なしの`gradlew.bat clean :app:testDebugUnitTest :app:assembleDebug --no-daemon --offline --console=plain`が成功した。JUnit 8件とdebug APK組立が成功し、artifact hashは`2D40701A90A518261D5E9E7E5E96AADF036D1A78354B9181B0E001E9A6632129`。元OneDrive workspaceのignored build outputには親から継承された削除deny ACLが残り、標準output pathではcleanupとresource packagingが停止するため、ACLを変更せず一時copyで検証した。これは製品source defectとは区別する開発環境上の`known_limitation`である。iOS Swift sourceのApple環境compile／XCTestは未確認。Rust Brokerへのnative LIVE_RUNTIME harness、実機・lifecycle・配布証拠も未成立であり、`rev2_mobile_flutter_native_device_link_boundary`と`rev2_mobile_device_evidence`を`release_blocker`として維持する。

D4 Pocket Phase 33では`docs/specs/gui-shell-module-pruning.md`と機械可読Module一覧を追加し、Brokerが必須Moduleを維持しながら任意画面の選択計画・依存閉包をReceiptへ記録する。Desktop画面はcompile-time defineへ接続済み。cleanなsource commit `aa3f2eac4f829d230a782fbd5f5cf7fc58d79c6c`から同一Windows／Flutter toolchainのall-enabled baselineと選択buildを実行し、Flutter AOT report上で選択外6つのsurface library nodeが不在、選択2つが存在すること、artifact総量差229,376 bytes（224 KiB）、hashを確認した。JSON Receiptは画面選択としてだけ読み、出所・Owner権限は検証しない。これはDeveloper用Flutter UI build／compiler reportの証拠に限られ、共有symbolや画面意味全体、最終製品の安全Core、Rust／third-party Moduleの保持・除去を証明しないため、`binary_pruning_verified=false`を維持する。製品cold startup・実行時resourceも未測定である。Owner確認経路はRust起動器のnative確認、process内allowlist、Broker再検証として実装され、自動試験済みだが、clean installed product上の実クリックとformal収集証拠は未成立。Owner確認後のManifest file生成は固定Export directoryへ接続したが、これは構成fileだけで、実行可能package、別Runtime／物理Audit store、Module pruningを生成しない。独立Export bundle上の安全Core／選択境界検証、製品測定、Installer、署名、配布、書出し先起動は`release_blocker`。Phase 33およびreleaseは未完了である。工程対応は`docs/D4_POCKET_PHASE_MAPPING.md`、toolchain・artifact・report hashと検査結果は`docs/REV2_PROGRESS.md`の最新Phase 33追補を参照する。

D4 Pocket Phase 34の限定単位として`docs/specs/windows-desktop-launcher.md`を正本に、staged Windows配置からterminal不要でDesktopを開くRust起動器を追加した。起動器は同一Rust process内で既存Brokerを管理し、固定Flutter executableを起動してMethodChannel→PID-bound pipe→Broker relayを提供し、起動・終了をBroker永続Auditへ記録する。Flutter child環境はOS実行に必要な限定allowlistとpipe接続先札だけに絞り、親processの資格情報候補、PATH、Broker runtime／endpoint／session環境値を継承しない。Windows staged smokeでは実画面がBroker snapshotへ到達した。ただしstage manifestはdirty worktreeを記録し、clean-source／formal installed product evidenceではない。staged manifestはcollector用isolated scratch pathと、起動器が実際に使うper-user `%LOCALAPPDATA%\GUI-Shell\broker\desktop`を区別し、後者の分離profile実証は未成立である。Owner資格・追加権限経路・任意commandは導入しない。正式Installer、Uninstaller、Signed Update、Rollback、Download→Install、正式identity・署名、official collectorを通るinstalled evidenceは未成立である。Phase 33のModule Pruning blockerと`rev2_flutter_broker_channel_boundary`は独立して保持する。検証結果とartifact hashは`docs/REV2_PROGRESS.md`の最新Phase 34追補に記録する。

D4 Pocket Phase 35の開発単位として、既存C24 Mobile投影にAgent状態の読み取り面を追加する。MobileはDevice Link TLSからBroker既存`Agent一覧`を読み、AgentAdapter metadataを`mobile_agent_list`／`agent_adapter` contractで検証して要約表示する。これはINTERNAL_STATEのBroker応答とAdapter metadataの表示に限り、Agent起動・実task実行性・Trust・Permission・Approvalを証明しない。Agent一覧のTLS接続、Android/iOS実機、安全保管lifecycle、Windows installed productからの統合証拠は未成立のまま保持する。Phase 35全体およびreleaseは未完了である。

C24ではdocs/specs/mobile-surface.mdを正本として、Mobileの既存9画面を保持しながら資源概要、履歴、MCP状態を追加した。通知summary、Runtime lifecycle状態、資源観測、owner再承認待ち停止receipt、現在owner承認に結合した履歴metadataだけを、Device Linkから既存Rust Brokerの読み取り専用handlerへ接続する。MobileはApproval、Permission、Authority、Credential、MCP接続、Tool実行、実停止を所有しない。Mobile実機、TLS実接続、Windows installed product証拠、長時間運用、障害注入、owner GO、正式releaseはrelease_blockerとして保持する。

rev2のHost操作面とC19 Adapter管理操作を、既存のGUI-Shell契約とRust Broker経路へ接続した。`docs/specs/host-operation-surface.md`と`docs/specs/adapter-management-surface.md`を正本とし、Hostの表示コンテキスト操作、Adapter metadata一覧、owner限定のAdapter状態管理、署名検査、隔離再利用拒否を実装・検証する。Host metadataとAdapter metadataはPermission・Approval・Authority・Credentialを生成しない。Adapter導入・更新・削除は現時点ではBroker catalogのmetadata操作に限定し、外部artifactのdownload、filesystem操作、process起動を完了扱いにしない。

C27では`docs/specs/performance-validation.md`を正本として、開発用snapshot生成とDesktopのbounded projectionを5秒watchdog付きで測定する。測定は`INTERNAL_STATE`または`FIXTURE`に限定し、実installed製品の起動時間、GPU frame、実Runtime負荷、8時間運用へ昇格させない。C27の開発測定はPASSしたが、C0-C34全数完成、Windows installed product証拠、owner GO、正式releaseは未成立である。

C28では`docs/specs/long-run-validation.md`を正本として、実Broker IPCによる対話反復、localhost Runtime fixtureの再起動、開発用lifecycle fixtureの再起動、Broker再起動、接続断・再接続、bounded履歴読取、Broker working set／永続store観測を検証する。通常検証には30秒smokeを接続し、8時間実測は`--duration-hours 8`の明示実行だけを証拠とする。clean commit `6a7ccff`からの8時間試行は22.766秒後にRuntime相当再起動中の通信失敗で終了した。失敗codeが欠けており根本条件は未特定のため、検証器へ固定失敗分類と作業段階内に限定した診断読取を追加し、12件の回帰試験と120秒stressはPASSした。これは再試行準備と短時間回帰の証拠であり、8時間完遂の代替ではない。8時間、installed product、外部Runtime／Agent負荷、正式releaseは未成立である。

C29では`docs/specs/failure-injection-validation.md`を正本として、Runtime停止相当、Broker process停止、MCP／A2A timeout、Credential unavailable、store書込不能simulation、Audit書込失敗、malformed stateを実Broker経路へ注入し、fail-closedを確認する。通常検証にはC29 smokeを接続する。fixtureとtemporary storeの結果はinstalled製品・外部サービスの障害耐性証拠へ昇格させない。

C30では`docs/specs/full-regression-validation.md`を正本として、Trust、Authority、Permission、Approval、Audit、Recovery、Evidence、Runtime、Agent、Dialogue、Compare、Device Linkを既存のRust／Broker／Flutter／fixture検証へ対応付ける。回帰matrixはlocal validationの範囲を報告し、未導入Agent、実端末、外部Runtime、installed productをPASSへ昇格させない。

C31ではREADME、GUI操作面、SECURITY、CONFIG、MOBILE_STATUS、COMPATIBILITY_MATRIX、REV2_PROGRESSを、C24〜C30の現行実装・検証範囲へ更新した。文書更新は機能実装やrelease evidenceの代替ではなく、各文書にproduction path、authority境界、証拠範囲、残存分類を明示するための監査単位である。C31時点の基準値はSchema 108件、Conformance 182件であり、C30 matrixはPASSしたが、C28の8時間実測、Windows installed productの総合証拠、外部Runtime／Agent、実端末、owner GO、正式releaseは未成立である。

C32では`docs/specs/final-development-audit.json`を正本データとして、C0〜C31を意味正本、Contract、Code、Production path、Test、Negative、Recovery、Audit、UI、Evidenceへ対応付ける監査器を追加した。監査器は参照pathと工程番号を検査するが、対応表の成立を機能完成・release readinessへ昇格させない。C32の未監査範囲はC33 Windows最大到達点、C34正式release前作業、およびWindows installed／外部Runtime／Agent／実端末証拠である。

C33ではWindows最大到達点に向け、Windows release build、Rust helper release build、Broker smoke collector、installed smoke collectorのWindows固有検証を整備した。`flutter build windows --release`、`cargo build --release --locked`、およびclean isolated run `c33-2c8ad7d`のBroker smokeはPASSした。Broker smokeは認証IPC、永続store、replay拒否、再起動後health、crash fail-closedを実測した。一方、installed productの総合validatorは、Raw UI Automationでroot／Flutter view以外のsurfaceを取得できないこと、Setup DoctorがCodexホストのLocalCache実行パスとinstalled contextを一致させられないこと、外部Audit anchorが未提供であることから失敗した。これらをComputer Useの補助観測やBroker単体PASSで代替せず、Windows installed productの総合evidenceと正式releaseは未成立のまま保持する。

D4 Pocket統合の次単位では、GUI Shell構成ManifestをSchema-firstで追加した。Desktop設定面からRuntime、Agent、Tool、MCP、Theme、Capability、Settingsを選択要求し、認証済みRust Brokerが再検証済みのManifest-only Receiptを返す。Authority、Permission、Approval、Credential、Audit chainは継承せず、build、独立App identity、filesystem、process、network、credential実値も生成しない。GUI Shell Export、Preview／rollback、Module Pruning、Distributionは後続の`release_blocker`である。現行基準値はSchema 112件、Conformance 185件であり、Rust全体試験とDesktop Flutter全体試験はこの単位で再実行する。

続くGUI Shell構成Preview単位では、現在Manifestと候補Manifestの差分、機能要件、対象platform、版rollbackの可否を認証済みRust Brokerで計算し、Desktopへ読み取り専用のPreview Receiptを返す。`rollback_available=false`、build／Export未開始、権限非生成、継承禁止を固定し、Previewを実rollbackや独立App生成へ昇格させない。現行基準値はSchema 114件、Conformance 186件であり、Windows Export、Module Pruning、Distribution、owner GO、正式releaseは未成立である。

続くGUI Shell編集提案単位では、Owner／Developerが明示開始した構成・UI・Contract変更候補を、許可path、規約確認、自己承認禁止、`proposal_only`でBrokerへ接続する。Receiptは審査待ち、未適用、未書込、権限非生成を固定し、製品Runtimeの自己変更や自動applyを行わない。実差分の適用、GUI Shell Export、Module Pruning、Distribution、owner GO、正式releaseは未成立である。後続単位でWindows書出しへ固定Manifest fileのcreate-only生成を追加済みだが、実行可能Appや独立Runtimeは生成しない。

この単位はrev2全体、総合機能拡張rev1 C0-C34、Windows installed product、owner GOの完成を意味しない。未完了範囲と既存release_blockerは`docs/REV2_PROGRESS.md`、`release_blockers.registry.json`、各正本の分類を保持する。

Agent Adapter契約をSchema-firstで接続し、Windows上の実物Codex CLI（`codex-cli 0.155.0-alpha.16`）について、versionと`codex exec --help`をBroker登録時にも確認するRust Adapterを追加した。ownerが絶対executableとworkspaceを明示した場合だけ、既存の実行系対話・owner承認経路から固定read-only JSONL実行を行い、Windowsの実Broker通常IPCで`codex`実行系列挙まで確認した。Broker command dispatchは停止中のままであり、任意command、write-capable Agent、MCP、複数Agent比較、Handoffは追加していない。Claude／Gemini等の未導入Agentは存在を推測しない。これはAgent Launcher基盤の現行限定実装であり、製品releaseや全Agent機能の完成を意味しない。

続くC6単位では、`docs/specs/regression-case.md`を正本とするowner専用の`回帰Case登録`、通常IPCの限定metadataページ一覧、既存Owner CLI→Rust Broker経路での一件削除・中断Recoveryを接続した。登録は完了済み通常対話の要求ID/hash、全文表示、結果証跡、終了監査を照合し、owner明示のredacted定義をC5と別purposeのWindows ProtectedStoreへ保存する。一覧は公開receiptと暗号文hashを照合し、private本文を返さない。削除は永続Audit後に限りfileを操作し、削除後不在の再観測と結果Audit確定を要求する。中断Recoveryは現在状態を再観測し、削除を自動再試行しない。C5 Datasetへ自動importしない。今回、Brokerが送信受付で発行した要求hashを使うDesktop Owner登録面を対話paneへ接続した。ownerがredacted定義を明示記入し、登録専用native確認とBroker再照合を通す。Desktopの回帰Caseタブは引き続き公開metadataだけを表示する。C5への明示importとWindows installed product上のOwner操作実証は未完了の`release_blocker`として保持する。

続くC7の現行単位では、`docs/specs/credential-vault.md`を正本とするowner専用の新規資格情報登録と、通常IPCの検証付きmetadata一覧を接続した。秘密値は`ProtectedStore::Purpose::Credential`のWindows DPAPIへ保管し、資格情報からAuthority、Permission、Approvalを生成しない。秘密値の取得・Runtime／Tool／MCP／A2A注入、更新、失効、削除、接続先変更、Recovery、GUI管理面は未接続のrelease_blockerとして保持する。

続くC8では、`docs/specs/mcp-contract.md`を正本とするMCP外部概念射影契約を追加した。Server、Tool、Resource、Prompt、Transport、Credential ref、Trust、Capability diffをmetadata_onlyとしてSchema／fixture／Conformanceへ接続し、MCP metadataからAuthority、Permission、Approvalを生成しない。C9では`docs/specs/mcp-connection-center.md`を正本として、owner controlからstdio Serverのdiscovery／catalog取得と通常IPCのmetadata一覧をRust Brokerへ接続した。Tool実行、Credential実値注入、Streamable HTTP、OAuth、consent、disconnect、quarantineは未接続のrelease_blockerである。

C10では、`docs/specs/operation-profile.md`を正本とする運用プロファイルを追加した。ProfileはRuntime、Adapter、要求Capability、Content Exposure、network exposure、resource limit、UI preferenceだけを保持し、Rust Brokerが永続化・再検証・監査を行う。Profile適用は要求の記録に限定し、ProfileからPermission、Approval、Authority、Credentialを生成しない。Desktop設定画面から一覧、作成、適用要求、export、削除をBroker経由で操作できる。import契約はBrokerに接続済みだが、専用ファイル選択UIは追加していない。

C11では、`docs/specs/update-center.md`を正本とする更新センターを追加した。更新候補の正本化、Broker所有Ed25519 trustによる署名検査、署名済み候補の永続化、一覧、延期、download／適用／rollback要求のAuditをRust BrokerとDesktop設定画面へ接続した。外部download、install、process、rollbackの実行はsuspendedのままであり、要求receiptを実行完了へ昇格しない。信頼設定未構成、Windows installed productの更新証拠、owner GOは正式releaseのrelease_blockerとして保持する。

C12では、`docs/specs/notification-center.md`を正本とする通知センターを追加した。Rust Brokerが監査eventからsummaryを限定射影し、Desktop通知画面へ通常認証済みIPCで返す。既読・破棄状態はhash結合して永続化し、通知の開く操作はGUI navigationだけに限定する。監査reason、payload、metadata、資格情報を表示せず、通知登録経路も追加しない。Windows native toastは既存host capability未接続のため未成立として保持する。

C13では、`docs/specs/observability-center.md`を正本とする観測センターを追加した。Rust BrokerのAudit確定処理をboundedなSpan、Trace、Metricへ内部射影し、通常認証済み`観測一覧`とDesktop観測センターへ接続した。観測は`INTERNAL_STATE`に限定し、Auditのreason・payload・metadata・秘密値を露出せず、権限を生成しない。OpenTelemetry export、C14のTrace Inspector、Runtime全体の実測、installed product証拠は未成立として保持する。

C14では、`docs/specs/trace-inspector.md`を正本とする読み取り専用のTrace InspectorをDesktopへ接続した。C13の`観測一覧`を再利用し、Broker内部で実測されたSpanについて開始・終了・所要時間・状態・親Span・エラー分類をbounded waterfall表示する。Runtime、Adapter、Tool、外部通信は実測経路が未接続のため画面上で未成立として明示する。Trace表示はPermission、Approval、Authority、Capability、Credentialを生成せず、OpenTelemetry export、外部collector、Runtime全体の実測、installed product証拠、8時間運用は`release_blocker`として保持する。


C16の現行単位では、`docs/specs/a2a-connection-center.md`を正本とするowner専用のA2A Agent Card取得、Rust Security Broker統治、loopback HTTPのbounded検証、`LIVE_RUNTIME` metadata-only receipt、通常IPCの接続一覧を追加する。Agent Cardの宣言はTrust、Permission、Approval、Authorityを生成せず、Credential refへ実値を注入しない。C16補完では接続receiptのmetadataをBroker永続storeへ保存し、再起動後は`INTERNAL_STATE`・再承認要求の状態へ降格して一覧へ復元する。HTTPS、TLS再接続、公開endpoint discovery、Task／Message送信、Artifact本文、Stream購読、quarantine、複数Agent比較、Desktop専用接続画面、実A2A Test Harnessは未接続の`release_blocker`として保持する。

C17の現行単位では、`docs/specs/host-registry.md`を正本とするHost registryをRust Security Brokerへ接続した。`Host登録`はowner controlだけ、`Host一覧`は通常認証済みIPCだけを受け付ける。Host ID、Platform、Trust、接続状態、certificate／identity hash、Runtime／Agent summaryをmetadata-only receiptへ射影し、`hosts.json`へbounded・atomicに保存する。登録時は`pending_review`へ固定し、Host metadataからPermission、Approval、Authority、Credentialを生成しない。Host切替、Runtime／Agent一覧、Device Link実認証、Desktop Host操作面、Host間Workspace隔離は未接続の`release_blocker`として保持する。
C18の現行単位では、`docs/specs/host-operation-surface.md`を正本として`Host一覧`をDesktop Snapshotへ接続し、通常IPCの`Host切替`をBroker監査付きの表示コンテキスト操作として追加した。Host AのPermission、Approval、AuthorityはHost Bへ再利用せず、選択Hostと現在Broker観測Hostが一致しない場合のRuntime／Agent個別一覧は`未観測`とする。live Host再接続、Trust検証、remote Runtime／Agent discovery、Host間Workspace隔離は引き続き`release_blocker`である。

C19の現行単位では、`docs/specs/adapter-management-surface.md`を正本としてRuntime CenterのAdapter catalogをRust Brokerへ接続した。`アダプター一覧`は通常IPCのmetadata-only projection、導入・検証・有効化・無効化・隔離・更新・削除はowner controlのbounded state transitionとし、署名検査はBroker所有Ed25519 trustに限定する。通常IPCの変更要求はowner再承認待ちで停止し、隔離済みRuntime IDは登録、lifecycle、資源観測、対話から再利用しない。外部artifactの実download・filesystem導入・process管理・実削除、Windows installed product evidenceは`release_blocker`である。

C20の現行単位では、`docs/specs/windows-tray-surface.md`を正本としてWindows常駐トレイの表示・操作入口を追加した。Win32トレイはウィンドウ前面化、Broker由来の実行系状態・保留承認件数・重大通知件数のbounded表示、終了入口だけを担い、取得不能値を0へ変換しない。全Runtime停止はFlutterから通常認証済みBroker IPCへ`全Runtime停止要求`を送り、Brokerがlifecycle対象を列挙したowner再承認待ちreceiptを返す。直接kill、owner承認生成、実停止、権限生成は行わない。`release_blocker`はWindows installed productでのトレイ実機証拠と、owner再承認後の実lifecycle停止統合である。

C21の現行単位では、`docs/specs/command-palette-surface.md`を正本として既存コマンドパレットへRuntime、Agent、履歴、評価、MCP、通知、資源、資格情報、更新、Host、停止要求確認の画面遷移を登録した。C22では`docs/specs/global-search-surface.md`を正本として、Ctrl+Shift+Fの全体検索とboundedな表示用indexを追加する。検索結果は画面遷移だけを行い、検索metadata、History、Profile、MCP、A2A、Agent Card、TelemetryからPermission、Approval、Authority、Credentialを生成・再利用しない。実Runtimeのlive横断取得、本文検索、検索結果からの操作、Windows installed product evidenceは`release_blocker`または`known_limitation`として保持する。

C23では`docs/specs/desktop-ux-integration.md`を正本として、既存20画面を削除せず、Desktop Navigationを運用・安全・開発・設定・すべての論理グループへ整理する。グループ選択はUI表示状態に閉じ、別グループへの画面遷移を妨げず、Permission、Approval、Authority、Credential、Broker IPCを生成・変更しない。Mobileへの投影はC24の対象とし、Windows installed productでの実画面証拠とowner GOは引き続き`release_blocker`である。

## 現行追加指示：総合機能拡張 rev1（2026-09-13）

owner添付の[実装指示書](docs/総合機能拡張_rev1/実装指示書.md)・[工程表](docs/総合機能拡張_rev1/工程表.md)・[実装仕様書](docs/総合機能拡張_rev1/実装仕様書.md)全体を実装対象へ追加する。先行rev2でWindowsから実行可能な検証を進め、その後C0からC34を順に実装する。外部条件待ちは該当するrelease証拠だけに限定し、他工程の開発停止条件にしない。追加要求の受領記録、工程状態、基準検証、証拠境界は[総合拡張の進捗](docs/総合機能拡張_rev1/進捗.md)に置く。C1以降の新機能・性能・8時間運用・障害注入・全数監査を先行rev2の試験数で代替しない。

追加範囲の未完成はregistryの `comprehensive_extension_rev1_completion` に登録し、既存release consumerへ接続する。これは開発の禁止ではなく完成製品releaseの未成立条件である。owner GO、実機証拠、運用署名、配布条件の既存関門も維持する。

C2の内容履歴はC7のWindows安全保管に依存するため、[Windows保護bytes境界](docs/specs/windows-protection.md)のOS接続を前提単位として先行する。保管・内容閲覧の権限接続を省略せず、この前提単位でC2/C7完成を主張しない。

## 現行 owner 指示 rev2（2026-09-10）

現行追加工程は、統治規則 → Baseline → 日本語意味正本 → 契約 → Conformance → Shell Core → MINIDORA Adapter → Desktop 実行系操作 → 実行系比較 → Flutter 共通化 → Mobile → 端末連携 → Android → macOS → iOS → Windows 回帰 → Linux 回帰 → 全数監査 → 文書の順とする。各単位はローカルで実装・試験・監査し、完成分を順次 main へ commit / push して remote HEAD を確認する。

最初の単位は `AGENTS.md` 3.1 と運用モデルの統治変更である。自動 CI は禁止を維持し、GitHub Actions は `workflow_dispatch` の手動補助のみ許可する。今回の統治単位は workflow を追加・実行しない。品質基準、安全境界、release gate、owner GO は維持する。Mobile と端末連携の開発着手は許可されたが、未検証 platform を完成・verified と扱わず、Windows-first release の証拠を代替しない。

実行系対話要求・応答・セッション・比較結果を日本語で先に定義し、Flutter → Shell Core / Rust broker → Adapter → Runtime の責任を維持する。MINIDORA の実物 API は実装前に確認し、内部実装を Shell Core へ輸入しない。

Flutter共通化単位では `packages/gui_shell_ui` の analyze/test をlocal validationへ追加する。共有層は表示・通常client契約に限定し、DesktopとMobileの接続・資格保管はplatform側に残す。Mobile正式projectの足場と端末連携の完成を区別し、未接続の画面を実状態として報告しない。

Apple platform単位は、ローカルhostがWindows/WSLでMacがないため、`.github/workflows/apple-manual-build.yml` を手動補助に使用する。対象は固定Flutter 3.44.0による共有UI・Desktop・Mobileの解析とFlutter試験、macOS開発app・iOS Simulator appのbuild、Mac上のRust helper試験とする。Flutter試験はMac上の開発用試験であり、iOS Simulatorの起動・native plugin・Keychainの実検証とは区別する。triggerはworkflow_dispatchのみ、contents権限はread、actionとFlutterはcommit固定、成果物の保管は3日とする。実行環境・対象commit・log・artifact hash・自動変更差分を記録し、追跡ソースの自動変更は成功として扱わない。専用runner上の利用可能なiPhone Simulatorを明示選択し、native安全保管の統合試験と、固定参照MINIDORA・実Rust broker・TLS・保管再読取・OS前景復帰を通す統合試験を実行する。試験は非秘密値と専用prefixで製品資格から分離し、使用したSimulatorを終了する。実機install・launch・Keychain保護・対話・owner GOはこの補助実行で証明しない。

Android仮想端末の補助単位では `.github/workflows/android-manual-emulator.yml` をworkflow_dispatch限定で使用する。Windows hostではSDK一覧を確認したが、安定版emulatorとAPI35 imageは圧縮状態だけで約2.18GB、観測時の空き容量は約2.19GBで別途展開する余地に乏しい（初回のpreviewを含む2.20GBという読取は進捗文書で訂正）。製品構造や実機凍結を変更せず、Ubuntu 24.04の隔離runnerでAPI35 x86_64の明示作成AVDとnative安全保管試験を実行する。固定参照MINIDORA二実行系・実Rust broker・TLS・native再読取・OS背景復帰・端末失効を通す統合試験も同じ専用AVDで行う。KVM accessは当該runner利用者に限定する。ADB対象はemulator-5554だけに固定し、仮想端末属性とAVD名を照合する。実機探索・接続は行わない。固定Flutter、解析・試験、toolchain情報、対象commitと追跡差分を記録し、終了時に自分が起動したemulatorを停止する。これは仮想環境上の補助証拠であり、Android実機・正式署名・配布・release完成の証拠ではない。

rev2の要求監査では、Mobile実機証拠・Mobile正式配布・Desktop起動回帰を `release_blockers.registry.json` へ未解決のmanual gateとして追加する。既存Windows証拠とowner GOは保持し、旧post_v1_scopeによるrev2要求の除外を防ぐ。

2026-09-11のowner指示: Windows画面検証を再開する。Android実機検証は凍結し、再開指示まで実行・端末接続要求を止める。凍結項目は未検証のまま保持し、release gateの合格やowner GOへ置き換えない。

2026-09-11のowner確定指示: 監査アンカーはオフラインEd25519署名checkpoint方式を採用する。独立したRust release検証経路、Collectorとrelease再検証、owner管理の継続性記録を追加する。秘密鍵を本体・Repository・設定・環境変数へ保存せず、実鍵生成と署名はownerの手動操作に限定する。

## 完了監査の優先事項

### 現在の開発・検証条件（2026-09-13 owner指示）

現在、実機検証を実行できるOSはWindowsだけである。Windowsでは配置・起動・画面・broker・回帰の実検証を進める。Android実機検証は再開指示まで凍結し、その他の非Windows実機検証も利用不能として保持する。端末接続や実機証拠の提出を繰り返し要求せず、非Windowsは利用可能なbuild・自動試験・仮想環境で開発を継続する。SimulatorやWSLgの結果は実機結果へ昇格しない。

監査アンカーの実運用公開鍵固定とオフライン署名は、正式release直前まで外部条件待ちとする。通常開発でownerに秘密鍵生成・接続・署名を要求しない。実機証拠、実運用署名、正式配布条件、owner GOの未成立は正式releaseの判定に保持し、それだけを開発停止条件にしない。未実装・失敗・回帰が見つかった場合は、利用可能な環境で修正と検証を続ける。

通常の集約検証は `python tooling/validate_all.py --desktop-platform windows --include-mobile-release` を使用する。Linux開発環境ではplatformをlinuxとする。`--strict-release` は正式release判定に使用し、外部条件に変化がない間の開発進捗確認として反復しない。開発検査の失敗は修正対象であり、実機待ちを理由に無視しない。検証条件が変わるまでは、未検証範囲を保持したまま同じ待機報告を繰り返さない。

現時点の実証拠と対象commitは `docs/REV2_PROGRESS.md` 冒頭の現況節を参照する。以下の旧時点の監査項目は当時の要求・課題の記録であり、現在の合否は対象commitに結合した実証拠とrelease consumerで判定する。

~~~yaml
- item: Ghost Invariants
  classification: required_for_v1
  status: implemented_for_current_scope
  reason: <code>packages/shell_core/state_snapshot.py</code> は、静的な invariant flag ではなく、計測した <code>InvariantEvaluator</code> の結果を報告するようになった。
  required_action: 意図的な違反テストを conformance に維持し、新しい invariant surface の追加に合わせて拡張する。
  blocks_release: no

- item: Normalization Firewall
  classification: required_for_v1
  status: implemented_for_current_scope
  reason: Shell Core は、生の受信 payload を保持し、key を正規化し、authority alias を除去し、authority に類する値を検出し、曖昧な payload を隔離し、正規化 audit metadata を記録する。
  required_action: Unicode／大文字小文字／zero-width／camelCase／envelope／value-only の権限昇格テストを通過状態に保つ。
  blocks_release: no

- item: Language policy runtime convergence
  classification: release_blocker
  status: partially_started_not_passed
  reason: <code>native/rust_helper</code> 配下で Rust broker の production IPC／process 作業を開始している。認証付き <code>127.0.0.1</code> loopback IPC、process ごとの暗号学的 session secret、request size limit、永続 audit／replay／session file store、再起動後の replay 拒否、audit hash-chain の再起動時検証、改竄／不正形式の永続状態の拒否、IPC negative test、および normalization、policy eligibility、approval edit／rehash、content projection、audit verification、recovery mapping、command-envelope eligibility に対する Python-oracle parity は実装済みである。Flutter の <code>main.dart</code> は product authority status に <code>ShellCoreClient.product()</code> と認証付き broker IPC を使用するようになり、broker unavailable／auth／stale／malformed response の経路は、local JSON authority ではなく SUSPEND／fail-closed snapshot として表現される。<code>tooling/release_runtime_assertions.py --check</code> は <code>tooling/validate_all.py</code> と <code>tooling/evidence_bundle.py --check</code> に接続され、現在の product authority surface に Python authority process startup、Python snapshot generator invocation、no-FFI-authority direct bridge token が存在せず、authority operation が broker-mediated であることを検証する。Windows の Flutter analyze／test は <code>cmd /c pushd</code> 経由の <code>flutter.bat</code> で通過する。外部 Flutter shell script が CRLF line ending であるため、WSL から直接実行する <code>flutter</code> は引き続き失敗する。<code>ShellCoreClient.local()</code> は development／diagnostic 専用のままである。Command dispatch は SUSPEND のままで、broker health は現在も <code>authority_cutover_status=not_active</code> を報告し、installed no-Python-runtime evidence と Windows installed-path broker proof は未完了である。
  required_action: installed Windows app evidence から broker path を実証し、active dispatch の前に command-envelope execution gate を完成させ、installed product runtime では Python が dev／test／migration oracle のみに限定されることを実証し、completed product release の前に Windows installed-path broker evidence を収集する。
  blocks_release: yes

- item: Windows installer, first-run, and real Setup Doctor
  classification: release_blocker
  status: not_passed
  reason: installed app path の Setup Doctor と Windows installer／first-run smoke は、現在も Windows-first product の主要 blocker である。
  required_action: installed app path diagnostics、Windows installer／first-run flow、artifact／hash evidence、および strict Windows validation を実装する。
  blocks_release: yes
~~~

## 0. 製品定義

GUI Shell は、local Runtime、agent、tool、service を対象とする PC-first の AI Runtime／Agent Operation Shell であり、`LLM-readable application responsibility substrate`（LLM が読むアプリケーション責任基盤）である。

BLUE-TANUKI 専用 GUI ではない。

BLUE-TANUKI は参照コンシューマー／Runtime であり、adapter boundary を介して接続しなければならない。GUI Shell Core に BLUE-TANUKI 固有 logic を含めてはならない。BLUE-TANUKI の live integration は GUI-Shell v1.0 の release dependency ではない。

## v1.0 デスクトップリリースの範囲

GUI-Shell v1.0 は Windows-first とする。

platform の優先順位:

- 第一対象: Windows
- portability の計画対象: macOS
- 開発／検証用 slice: Linux

Linux の build と launch smoke は通過しており有用だが、それだけでは最終 product proof にならない。主要な product gate は Windows である。

GUI-Shell v1.0 は、検証済みの macOS support を主張しない。macOS host で検証するまでは、macOS support を supported、ready、complete と宣伝してはならない。

~~~yaml
- item: Linux desktop build smoke
  classification: required_for_v1
  reason: Linux development／verification build smoke は 2026-05-25 に通過した。
  required_action: development verification slice として <code>cd apps/desktop_flutter && flutter build linux</code> を通過状態に保つ。
  blocks_release: no

- item: Linux desktop launch smoke
  classification: required_for_v1
  reason: Linux launch smoke は 2026-05-25 に WSLg 上で通過し、Dashboard、NavigationRail、Runtime Status、Invariant Status の表示を確認した。ただし、これは Windows-first release evidence の代替ではない。
  required_action: Windows product gate の完了中も Linux launch smoke evidence を現行に保つ。
  blocks_release: no

- item: Windows desktop release gates
  classification: release_blocker
  reason: Windows が主要 product target である。Windows project support、Flutter analyze、Flutter test、Windows build、native launch smoke は development evidence として通過済みであり、Block E は staged installed app、broker smoke、Setup Doctor、installed first-run の collector を定義済みである。installed-path first-run evidence、installed-path Setup Doctor evidence、broker-mediated installed Flutter <code>.exe</code> launch evidence、No-Python launch evidence、strict Windows release validation、owner GO は未通過である。
  required_action: Windows development smoke を現行に保ち、その後 staged installed app launch、installed-path first-run、installed-path Setup Doctor、broker authenticated IPC／restart／crash／no-Python／no-FFI evidence、No-Python installed Flutter <code>.exe</code> launch evidence、strict Windows release validation、owner GO を通過させる。
  blocks_release: yes

- item: macOS planned portability target
  classification: known_limitation
  reason: 現在利用できる macOS validation environment がないため、GUI-Shell v1.0 は検証済み macOS support を主張しない。
  required_action: macOS support を supported、ready、complete と主張する前に、macOS host で検証する。
  blocks_release: no

- item: Windows Setup Doctor diagnostics
  classification: release_blocker
  reason: Windows 固有の Setup Doctor diagnostics は主要 product gate の一部である。
  required_action: app path から Windows Setup Doctor diagnostics smoke を通過させる。
  blocks_release: yes

- item: Windows installer and first-run plan
  classification: release_blocker
  reason: Windows installer と first-run behavior は主要 product gate の一部である。
  required_action: Windows installer／first-run flow を完成させて検証する。
  blocks_release: yes
~~~

## 1. 譲れない優先順位

1. 安全性
2. 堅牢性
3. operator にとっての明瞭さ／UX
4. 製品機能
5. 利便性

feature の完成を、安全性、authority boundary、auditability、recovery、operator visibility より優先してはならない。

## 2. 中核原則

GUI Shell は control plane であり、visual wrapper ではない。

UI は Runtime state を表示し、operator input を収集してよい。
UI は authority を生成してはならず、permission を付与してはならず、trust を再解釈してはならず、adapter conformance を迂回してはならず、sensitive action を隠してはならない。

contract の責任主体は schema と conformance である。

## 3. Phase 0 で固定した決定

現在の実行経路では、次の決定を固定する。

- 汎用 Runtime Operation Shell の方向性
- BLUE-TANUKI は参照 Runtime のみ
- BLUE-TANUKI は adapter boundary を介して接続
- authority-sensitive implementation path は Flutter UI + Rust Security Broker
- Rust helper は限定された non-authority native diagnostics／operations
- Compose Multiplatform は watchlist candidate
- Tauri は desktop-heavy fallback
- contract model は JSON Schema-first
- 実装順序は Conformance-first
- UI framework の governance risk には FrameworkRiskProfile
- 権限除去の適合規則（Authority Strip Conformance）
- 内容露出の境界（Content Exposure Boundary）
- 承認の表示／編集境界（Approval visibility／edit boundary）

## 4. 目標 architecture

~~~text
Runtime / Agent / Tool / Local Service
  -> Adapter
      -> schema validation
      -> authority strip
      -> content exposure policy
      -> capability declaration
      -> diagnostic normalization
  -> Rust Security Broker
      -> restricted IPC endpoint
      -> schema-validated broker envelope
      -> runtime registry authority path
      -> capability / permission eligibility
      -> approval validation and protected-field enforcement
      -> audit append / verification authority
      -> recovery classification
      -> command-envelope eligibility
      -> process / credential / update gated execution
  -> Shell Core Contracts
      -> runtime-neutral JSON Schema / protocol semantics
      -> migration oracle parity fixtures until cutover
  -> Flutter UI Layer
      -> Flutter rendering
      -> operator input
      -> navigation
      -> local UI state
  -> Rust Helper
      -> bounded non-authority native diagnostics
      -> bounded non-authority native operations
~~~

## 5. Phase 計画

正本となる Phase 定義は [docs/PHASE_STRATEGY.md](./docs/PHASE_STRATEGY.md) に置く。このファイルは実行ロードマップだけを保持する。

- Phase A の状態: complete
- Phase B の状態: owner-use complete
- Phase C: 次は OSS claim hygiene
- Phase D: 実測した Windows installed-path release evidence は後続
- Phase E: OSS v1.0 RC は後続
- Phase F: paid／product QC は後続

completed product release は主張していない。release-ready claim の前には、実装言語方針の Runtime convergence、Phase D の実測した Windows installed-path evidence、explicit owner GO が引き続き <code>release_blocker</code> である。

現在状態から、Windows-first OSS v1.0 の product release、実証済み LLM-readable extension substrate の capability、initial public release、post-public product QC までを統合した正本の実行ロードマップは次のとおりである。

~~~text
docs/implementation/GUI_SHELL_LLM_SUBSTRATE_COMPLETION_ROADMAP.md
~~~

統合 C／L／R／P／F block model には、そのロードマップを使用する。このファイルは Phase roadmap と現在の release blocker を保持する。

## LLM-readable extension surface の作業流

この作業流は、GUI Shell の contract を LLM development／integration agent が第一級の implementation／integration surface として読み、利用するという architecture の方向性を追加する。

LLM は GUI Shell contract の第一級 implementation／integration consumer だが、authority source では決してない。

この作業流は、Windows-first release gate および Rust Security Broker の convergence blocker と併存する。現在の <code>release_blocker</code> を隠したり、改名したり、完了扱いにしたりしてはならない。

### Stage L0: 定義の固定

- README／AGENTS／standard の定義を整合させる。
- human-authority と LLM-extension-agent の boundary を明示する。
- 非主張事項を記録する。
- 既存の Phase A／Phase B status は変更しない。

### Stage L1: contract 設計

- 明示的な machine-readable extension／module integration contract が必要か判断する。
- 既存 schema を、LLM-built module の onboarding requirement に対応付ける。
- 必要な negative case と failure behavior を定義する。
- schema surface を追加する代わりに既存 contract で十分な場合は、その旨を報告する。

### Stage L2: conformance の試験基盤

- 限定された reference extension／adapter scenario を追加する。
- authority を昇格できないことを実証する。
- approval を迂回できないことを実証する。
- audit evidence を出力することを実証する。
- 必要な場合に failure が RecoveryAction または SUSPEND へ対応付くことを実証する。

### Stage L3: agent 間再現 evidence

- 複数の development agent に repository を独立して読ませ、同じ限定的 extension task を実装させる。
- 両者が contract boundary を保持し conformance を通過するか比較する。
- 差分と failure mode を報告する。

### Stage L4: 公開標準／ecosystem の主張 gate

- evidence が存在して初めて、GUI Shell が development agent 間で実証済みの LLM-readable extension substrate であると主張できるか判断する。
- measured reproduction evidence と owner approval より前に、public standard status、ecosystem adoption、proven interoperability を主張しない。

### Phase 0: standard／選定の固定

目標: 概念上および技術上の選定 boundary を固定する。

成果物:

- <code>docs/specs/gui-shell-spec-v1.md</code>
- <code>docs/standards/gui-shell-extended-standard.md</code>
- <code>docs/research/flutter-governance-risk.md</code>
- <code>docs/research/compose-mp-watchlist.md</code>
- <code>docs/research/tauri-fallback.md</code>
- <code>CLAIM.md</code>
- <code>CONFIG.md</code>
- <code>AUDIT.md</code>
- <code>SECURITY.md</code>
- <code>TROUBLESHOOTING.md</code>

終了条件:

- GUI Shell が汎用 Shell として明確に定義されている。
- BLUE-TANUKI 固有 logic は Shell Core で禁止される。
- Flutter risk と migration condition が文書化されている。
- Phase 0 claim boundary が明示されている。

### Phase 1: schema／contract の閉包

目標: UI 実装前に、すべての core contract を定義する。

成果物:

- <code>specs/runtime.schema.json</code>
- <code>specs/adapter.schema.json</code>
- <code>specs/capability.schema.json</code>
- <code>specs/permission.schema.json</code>
- <code>specs/approval.schema.json</code>
- <code>specs/audit.schema.json</code>
- <code>specs/recovery.schema.json</code>
- <code>specs/diagnostic.schema.json</code>
- <code>specs/update.schema.json</code>
- <code>specs/content_exposure.schema.json</code>
- <code>specs/framework_risk_profile.schema.json</code>
- <code>tooling/schema_check/check_schemas.py</code>

必要な invariant:

- すべての schema に <code>$schema</code>、<code>$id</code>、<code>title</code>、<code>type</code> が含まれなければならない。
- Adapter metadata は untrusted とする。
- adapter では <code>authority_strip=true</code> を必須とする。
- content visibility は <code>none</code>、<code>hash_only</code>、<code>summary</code>、<code>redacted</code>、<code>full</code> を扱わなければならない。
- Approval payload には tagged SHA-256 hash を使用しなければならない。
- Framework risk を明示的に表現しなければならない。

終了条件:

~~~bash
python3 tooling/schema_check/check_schemas.py
~~~

が通過する。

### Phase 2: conformance の閉包

目標: product UI が safety contract に先行することを防ぐ。

成果物:

- <code>tooling/conformance_tests/run_conformance_skeleton.py</code>
- <code>docs/specs/gui-shell-spec-v1.md</code>
- <code>docs/specs/adapter-conformance.md</code>
- <code>docs/specs/content-exposure-policy.md</code>
- <code>docs/specs/approval-visibility-boundary.md</code>
- <code>docs/specs/authority-strip-conformance.md</code>

必須の conformance check:

- 受信した authority key を除去する。
- 外部 metadata が authority を昇格させることはできない。
- GUI input が Runtime で許可されていない authority context を生成することはできない。
- memory、cache、previous state だけで authority を付与することはできない。
- <code>content_visibility=full</code> でない限り、full content を表示できない。
- <code>authority_fields</code>、<code>sealed_fields</code>、<code>hidden_fields</code>、<code>sacred_fields</code> は編集できない。
- 編集した approval payload を再 hash／再検証する。
- sensitive action は capability、permission、approval state、AuditEvent、および失敗時の RecoveryAction に対応付けなければならない。

終了条件:

~~~bash
python3 tooling/conformance_tests/run_conformance_skeleton.py
~~~

が、意味のある failure-case coverage を伴って通過する。

### Phase 3: Shell Core の skeleton

目標: framework-independent な Shell Core を構築する。

成果物:

- <code>packages/shell_core/</code>
- <code>packages/shell_contracts/</code>
- framework-neutral な UI state abstraction に限る <code>packages/shell_ui/</code>
- Runtime 登録簿（Runtime Registry）
- Adapter 読込器（Adapter Loader）
- Permission 台帳（Permission Ledger）
- Approval 待ち行列（Approval Queue）
- Audit 保管庫（Audit Store）
- Recovery 目録（Recovery Catalog）
- Update 方針保管庫（Update Policy Store）

必須規則:

- Shell Core は Flutter を import してはならない。
- Shell Core は BLUE-TANUKI の内部実装を import してはならない。
- Shell Core は adapter metadata を信頼してはならない。
- Shell Core は memory／cache を authority として扱ってはならない。
- Shell Core は inspection 用の deterministic state snapshot を公開しなければならない。

終了条件:

- core test が通過する。
- sensitive action routing を Flutter なしで test できる。
- BLUE-TANUKI adapter を Shell Core の変更なしに開発できる。

### Phase 4: Rust helper の境界

目標: hidden authority を作らず、限定的な native capability を追加する。

成果物:

- <code>native/rust_helper/</code>
- process の診断
- filesystem の診断
- network の診断
- update の検証
- audit の hash 化
- 安全な IPC
- 構造化された helper response

必須規則:

- Rust helper を独立した authority path にしてはならない。
- すべての helper action は capability-scoped でなければならない。
- sensitive な helper action はすべて permission／approval linkage を要求しなければならない。
- helper output は schema-valid でなければならない。
- helper failure は RecoveryAction に対応付けなければならない。

終了条件:

~~~bash
cd native/rust_helper && cargo test
~~~

が Rust の導入環境で通過する。

### Phase 5: BLUE-TANUKI 参照 adapter

目標: BLUE-TANUKI を最初の Runtime として adapter のみを介して接続する。

成果物:

- <code>packages/blue_tanuki_adapter/</code>
- health 用 adapter
- ready 用 adapter
- Runtime snapshot 用 adapter
- authority trace 用 adapter
- notification 用 adapter
- approval 用 adapter
- audit export 用 adapter
- diagnostics 用 adapter
- recovery 用 adapter

必須規則:

- GUI Shell の利便性のために BLUE-TANUKI Core を変更しない。
- Shell Core に BLUE-TANUKI の内部実装を import しない。
- BLUE-TANUKI adapter は Runtime state を汎用 GUI Shell schema へ正規化しなければならない。
- Runtime 固有概念は adapter layer 内に留めなければならない。

終了条件:

- Adapter conformance test が通過する。
- BLUE-TANUKI state を汎用的に表示できる。
- BLUE-TANUKI 固有 authority logic が Shell Core に存在しない。

### Phase 6: デスクトップ Flutter operator Shell

目標: 最初の可視化された operator Shell を構築する。

成果物:

- <code>apps/desktop_flutter/</code>
- 概況画面（Dashboard）
- 診断画面（Setup Doctor）
- Runtime 管理画面（Runtime Center）
- Permission 管理画面（Permission Center）
- Approval 管理画面（Approval Center）
- Audit 閲覧画面（Audit Viewer）
- Recovery 管理画面（Recovery Center）
- 設定画面（Settings）
- Runtime invariant の表示面

必須規則:

- Flutter の責任は rendering のみとする。
- Flutter は permission semantics を定義してはならない。
- Flutter は audit semantics を定義してはならない。
- Flutter は authority を付与してはならない。
- contract が許可しない限り、Flutter は full content を表示してはならない。
- Flutter UI action は Shell Core API を通さなければならない。

終了条件:

~~~bash
cd apps/desktop_flutter && flutter analyze
~~~

が Flutter の導入環境で通過する。

### Phase 7: installer／first-run 経路

目標: low-level complexity を露出せず Shell を利用可能にする。

成果物:

- <code>installer/windows/</code>
- <code>installer/macos/</code>
- <code>installer/linux/</code>
- 初回実行 wizard
- environment の診断
- dependency の検査
- Runtime 接続の検査
- recovery の手順

必須規則:

- 通常ユーザーの主要経路として CLI／WSL／npm／Git の複雑性を露出しない。
- Setup Doctor は failure を operator 向けの言葉で説明しなければならない。
- installation は permission を暗黙に付与してはならない。
- installer state を authority にしてはならない。

終了条件:

- 非 expert user が app path から install、launch し、Runtime state を確認できる。
- failure が分類され、recoverable である。

### Phase 8: モバイル Shell／companion

目標: desktop authority を迂回せず mobile participation を追加する。

成果物:

- <code>apps/mobile_flutter/</code>
- device の pairing
- notification の表示
- approval の review
- Runtime の状態
- emergency stop の request
- recovery 手順の表示

必須規則:

- mobile は Shell Core を迂回してはならない。
- mobile approval は field visibility／edit constraint を維持しなければならない。
- mobile device identity を明示しなければならない。
- device pairing は auditable でなければならない。

終了条件:

- mobile は policy の範囲内で observe／approve できる。
- mobile は hidden authority path を生成できない。

### Phase 9: release の強化

目標: 明示的な claim boundary を伴う OSS release を準備する。

成果物:

- release の checklist
- security の review
- license の検証
- signed build の計画
- update の検証
- 互換性 matrix
- 適合性 report
- 監査 evidence bundle

終了条件:

- owner が release claim を明示的に承認する。
- 適用されるすべての validation が通過する。
- 公開 README の claim が実際の implementation state と一致する。

## 6. 現在の claim boundary

後続の promotion までは、GUI Shell は次の事項だけを主張する。

- v1.0 product-completion scaffolding を備えた desktop-first AI Runtime／Agent Operation Shell の skeleton
- schema-first の contract
- conformance-first の作業順序
- 最初の implementation candidate としての Flutter + Rust helper
- adapter のみを介した参照 Runtime としての BLUE-TANUKI
- permission、approval、audit、recovery、policy evaluation、deterministic state snapshot、content exposure のための framework-independent core asset

現時点では、次の事項を主張しない。

- production readiness（本番準備完了）
- signed installer readiness（署名済み installer の準備完了）
- stable mobile readiness（安定した mobile の準備完了）
- BLUE-TANUKI integration の完全な実装
- Rust helper の完全な実装
- Flutter product UI の完全な実装
- security の完全性

## 7. 必須 validation

各完了作業報告の前に、最低限、次を検証する。

~~~bash
python tooling/schema_check/check_schemas.py
python tooling/conformance_tests/run_conformance_skeleton.py
~~~

<code>python</code> が利用できない場合:

~~~bash
python3 tooling/schema_check/check_schemas.py
python3 tooling/conformance_tests/run_conformance_skeleton.py
~~~

Rust が導入されている場合:

~~~bash
cd native/rust_helper && cargo test
~~~

Flutter が導入されている場合:

~~~bash
cd apps/desktop_flutter && flutter analyze
cd apps/mobile_flutter && flutter analyze
~~~

集約 reporter:

~~~bash
python3 tooling/validate_all.py
~~~

## 8. release 規則

次の条件を満たすまでは release readiness を主張しない。

- schema validation が通過する
- conformance validation が通過する
- 適用される場合は Rust helper test が通過する
- 適用される場合は Flutter analysis が通過する
- sensitive action の audit evidence が存在する
- installer behavior が検証されている
- owner が release promotion を明示的に承認する

## 2026-09-26 Windows collector／C28現況

Windows installed smoke collectorをRust Desktop起動器経由へ更新した。別Windows user profileとrun固有LOCALAPPDATAを要求し、endpoint、Broker lifecycle Audit、通常起動時のhealth受理Audit、artifact hashを実runから採る。health AuditはBroker側の認証済み要求受理・永続記録を示すが、client応答受信や呼出し元PIDは証明しない。validatorはendpoint／Auditに加えruntimeとconfig pathをisolated profileへ結び付ける。Conformance 205件、Schema 132件、日本語strict監査、Windows python-only集約の全10検査はPASS。Rust起動器、Broker helper、Flutter Release executableがworkspaceにないため、実built productのcollector実行は未確認である。初回config生成とBroker統治Setup Doctor production exportは未接続。該当するWindows first-run／Setup Doctor gateは`release_blocker`のまま、`release_ready=false`を維持する。

C28 clean commit `b81fc607e65ecaf7fc8bc5de53376e39949e0f20`からの8時間再試行は約0.578秒で`大量対話`段階に失敗した。`%LOCALAPPDATA%\GUI-Shell\development-evidence\c28-8h-b81fc60-20260925-145127.json`（SHA-256 `AC69F9D977F9FAE6E10941F7FB3555CA5D06F2585C09FBFB119CCC7A1D3DC0B0`）はfailed記録として保持する。`dialogue_result_not_success`とhealth read headersの`ConnectionReset`が記録された。fixture serverのflush成功はclient受信証拠ではなく、8時間完遂も原因確定も成立しない。

### 2026-09-29 Windows installed collector v15追補

C33で失敗したUI Automationの画面領域取得をRaw Viewから`Control View`へ修正し、対象frontend PIDのtray menuだけを探索するようcollectorを境界付けした。製品source commit `ae337eee62074229244fb0502fa498c3daa1a3e3`から一時配置したRelease版を実起動した診断専用観測では、`Control View`の122要素とDashboard／NavigationRail／Runtime Status／Invariant Statusの4画面領域を取得し、画面領域検証器が受理した。trayからの通常終了がfrontendへ届き、frontendの強制終了なし、launcher終了code 0、Broker endpoint除去まで確認した。初回設定生成、Setup Doctor報告の成功、通常Broker正常性要求の受理、config Audit hash一致、Python path除去も観測した。証拠fileは`release_evidence/windows_installed_smoke_diagnostic_v15_ae337eee.json`（SHA-256 `EF203EB9CB98FC53FF97705113EFE837DA67C57893F49D72BFD7CB7BAACEA9F0`）と画面領域投影`release_evidence/visible_surfaces_diagnostic_v15_ae337eee.json`（SHA-256 `5545D3D593248D0D8D214C6C18D19C0BD6ED05EA41ABB128EC0382503A0138D7`）。これは診断用の`LIVE_RUNTIME`部分証拠であり、Computer Useの画面観測はvalidator証拠に含めない。

v15は画面領域の要素取得を最大10,000件に制限し、欠落・上限到達・重複runtime IDを証明失敗として扱う。正式証拠検証器は`Control View`、欠落のない取得、全要素の投影を要求する。今回の実行は`DiagnosticOnly`で、stagingと同じWindows profileを使用したため、profile分離と実行来歴、完全な証拠一式、Setup Doctorの操作者向け可読性、総合証拠一式内のBroker smoke、外部監査基点を満たさない。したがってWindows installed first-run、証拠来歴分離、Setup Doctor、集約Broker smoke、Audit anchorは`release_blocker`のまま、`release_ready=false`を維持する。検査と履歴の詳細は`docs/REV3_PROGRESS.md`および`docs/WINDOWS_RELEASE_EVIDENCE.md`を参照。
