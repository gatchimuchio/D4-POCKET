# Agent Runtime の契約（Agent Runtime Contract）

Agent Runtime は第一級の Runtime type である。Shell Core の Permission、Approval、Audit、Recovery の各 semantics によって制御しなければならない。

中核 record:

- `AgentRuntime`
- `AgentSession`
- `AgentTask`
- `AgentWorkspace`
- `AgentToolCall`
- `AgentPermissionRequest`
- `AgentDiff`
- `AgentCommit`
- `AgentPullRequest`
- `AgentRunLog`

Conformance 要件:

- workspace 外への access は default deny とする。
- secret path の read は default deny とする。
- shell command には Permission mapping が必要である。
- `git push` には明示的な Approval が必要である。
- 生成された diff には Audit evidence が必要である。
- Agent の auto-permission mode は advisory に限定する。
- state-changing action には rollback candidate が必要である。

作業領域の詳細検査は[作業領域インスペクタの契約](workspace-inspection.md)へ接続する。内部差分の生成成功を、取得権限・表示権限・製品経路の完成とみなさない。

## Agent作業要求と権限の分離

`agent_task_request.schema.json`は、Agent作業要求をAgent Runtime ID、Session ID、Broker登録Workspace ID、指示本文だけで表す。Workspace path、実行file、command、sandbox設定、Permission ID、Approval ID、Audit IDを要求本文から受け取らない。識別子の形式適合だけでは、登録関係、実体、Trust、Permission、Approval、実行可能性を証明しない。Brokerは要求ごとに現在のRuntime・Session・Workspace登録関係を照合し、古い要求、再送、異なるRuntimeやWorkspaceへの流用を拒否しなければならない。

`agent_task.schema.json`の版2は、実行Taskの状態投影としてRuntime／Session／Workspace ID、Broker計算の指示hash、状態、Audit参照を結ぶ。`description`は非機密の固定label専用とし、Broker実装は固定語彙から生成する。JSON Schemaは自由文字列の秘密性を判定できないため、指示本文・Agent出力・Credential実値をこのfieldへ流さないことをConsumer側の必須条件とする。出力本文をrecordへ埋め込まず、必要な本文取得は別のContent Exposure BoundaryとOwner許可へ従わせる。状態値は`pending`、`running`、`blocked`、`completed`、`failed`、`cancelled`、`quarantined`に限る。`result_hash`は任意の本文hash参照であり、結果本文やTask成功の証拠ではない。版なしの旧形は履歴互換のため読み取り可能とするが、Runtime／Workspace結合や実行状態の証拠に使用しない。いずれの版もPermission、Approval、command、filesystem path、Credential、raw outputを含めない。

対話要求への一回限りのOwner承認（対話Approval）は、Agentに指示を送る承認であり、Workspaceへの書込みPermissionやAgent作業Taskの実行承認ではない。作業実行には、Brokerが解決する限定Workspace Permission、Task本文と実行条件に結合した別個の一回限りOwner Approval、実行前後のAuditEvent、失敗時RecoveryActionをすべて要求する。Agent metadata、UI、Task本文、履歴、Profile、Capability宣言からこれらを生成・再利用しない。指示本文のhashはBrokerが計算し、Task要求元の自己申告hashを受け付けない。監査・log・traceには本文やCredential実値を複写せず、必要なhashと状態だけを保持する。

Rust Brokerの`Agent作業要求検査`はこの要求を受け、現在のAgent Adapter、利用中Session、Session作成時と同一のWorkspace登録hashを再照合する。さらにTask検査とPermission／Approval発行では、Adapterが起動時に固定したWorkspaceのdevice/file IDと、Brokerが保持する登録rootのdevice/file IDの一致を必須とする。識別子の欠落または不一致は`作業領域不在`として拒否し、Workspace IDやmetadata宣言では代替しない。検査成功時も返すのはBroker計算の指示hash、「未実行」状態、現在のSession・Workspace登録hashに結び付くPermissionと、別途発行されたTask Owner Approvalの有効／未取得状態だけであり、要求本文やPermission／Approval IDを応答・Auditへ複写せず、Taskを保存・起動しない。Permission状態はBroker内の現行記録、二つの期限、Session／Workspace／登録hashの一致から毎回再評価する。Approval状態も本文hash、Permissionの内部識別子、現在の登録関係、固定実行条件policy、wall／monotonic期限を毎回再照合する。物理root照合は`INTERNAL_STATE`の登録対応検査であり、Runtime稼働、path上の継続的同一性、Workspace隔離やsandboxの実証ではない。Broker envelopeの同一nonce再送は既存replay防護が拒否するが、新しいnonceによる同一内容の検査は副作用のない再検査であり、Task実行承認の消費ではない。

Agent Taskの要求検査・Permission／Approval・実行を扱う各Broker操作は、構造検査済みAdapter metadataに`task_execution` capabilityがあり、その状態が`supported`であることを前提条件として毎回確認する。欠落、`unknown`、`unsupported`は`AgentTask実行非対応`として検査・発行・起動を拒否する。加えて、Adapter実装がBroker登録rootと一致する物理Workspace識別子を提供しなければTask関連操作を拒否する。これは宣言に基づく利用可否gateと物理登録対応のfail-closed検査であり、metadataはPermission、Approval、Trust、実行可能性を付与せず、root識別子の一致もsandbox実効性や書込隔離を証明しない。Rust Brokerの実行Consumerは実装済みだが、consumerの存在は実証済み隔離を意味しない。現在のCodex Adapterは未対応のままで、実AdapterはConsumerに加えて実行隔離を検証するまで`supported`を宣言してはならない。

## Agent TaskのBroker実行Consumer

`AgentTask実行`は、Task request本文からBrokerが計算したinstruction hashを使い、現在のAdapter／利用中Session／同一Workspace登録hash／Adapter固定root identityを再照合する。Adapter metadataの`task_execution=supported`に加え、Rust Adapter実装自身がTask実行対応を返すことを要求する。どちらか一方だけでは起動できない。さらに、Workspace PermissionとOwner Approvalのwall／monotonic期限、本文hash、実行条件hashを同じBroker排他区間内で再検証し、開始Auditを確定した後にPermissionとApprovalを不可分に消費してからworkerを起動する。拒否、期限切れ、本文・登録差替時はgrantを消費せず起動しない。worker起動後の取消・失敗・期限超過では、同じgrantを再利用できない。

Task状態はBroker内の独立したbounded projectionであり、通常対話のApproval・Session作業記録へ混ぜない。同一Sessionまたは同一Workspaceにつき同時実行は1件、Broker全体で最大4件、直近状態recordは最大128件とし、上限到達時は最古のterminal recordだけを監査参照を残して置換する。`AgentTask状態`と`AgentTask取消`はtask IDだけを受ける。取消応答は停止完了を意味せず、Adapter workerが実停止してterminal状態を返すまではTaskをrunningとして保持する。取消要求後または期限後に届いた成功応答はBrokerが採用しない。期限到達では取消を要求し、完了応答を受けるまで成功・停止へ昇格しない。ただし取消flagと遅延応答の不採用はOS process群の強制終了を保証しない。

開始・終了・取消要求Auditは固定label、task ID、instruction／result hashと状態だけを保持し、本文・出力・Credentialを含めない。worker出力本文はBrokerのTask状態recordやAuditへ保存せず、Broker側でhash化後に破棄する。`result_hash`と`completed`はAgent processの応答完了を示すに限り、Workspace変更の正しさ、test成功、隔離、release readinessを証明しない。Workspace差分の列挙・内容表示は別のWorkspace inspection／Content Exposure Boundaryへ従う。worker終了Auditを確定できない場合はTaskを`quarantined`へ移し取消を要求し、成功結果を返さない。

Task状態recordはBroker processの揮発状態で、再起動後に照会できない。PermissionとApprovalも揮発し再利用できない。Broker終了またはSession隔離時のCancellation flagだけではOS child process群の終了証明にならない。scratch回復journalと起動時reaperを実装したが、実Adapterはprocess tree supervision、期限、取消、scratch cleanupを実装・検証するまで`task_execution=supported`にしてはならない。現行Codex Adapterはread-only Dialogueに`--sandbox read-only`を使い、Task専用経路には`default_permissions=d4p-agent-task`のprofileを指定する。Windows Task commandは`windows.sandbox="mxc"`を明示し、filesystem policyは`:root=deny`、`:minimal=read`、`:workspace`継承、およびglob走査深度32までのWorkspace root内`**/*.env`、`**/.env.*`、`**/.ssh/**`、`**/secrets/**` denyを組み合わせる。networkも無効にする。Task起動時は登録Workspace直下へ乱数名scratch directoryを作成してchildのTEMP/TMPをそこへ限定し、process群停止後にhandle経由で削除する。これは当該CLI／Windowsで利用可能なsandbox方式を固定する設定であり、深度32超や列挙されていない別名secret pathを包括的に拒否する保証ではない。Broker capability metadataは`unsupported`のままであり、実Codex Taskは起動できない。実Agentの書込隔離・外部path拒否、Task取消／期限、通常終端・crash後cleanupのWindows実runtime検証、差分内容UIおよびLIVE_RUNTIME conformanceは未成立の`release_blocker`である。Job Objectはprocess群の終了管理であってfilesystem cleanupやsandboxではない。

2026-09-28のhelper再検査は当時の履歴として保持する。Task profileのfilesystem設定を単一overrideへまとめた構成では、通常NTFSの合成`.env`読取とWorkspace外書込は拒否、Workspace内書込は許可された一方、UserProfile／LocalAppData内の合成外部file読取とOneDrive `ReparsePoint` `.env`読取は許可された。`:root=deny`はelevated backendでeffective `:root` readが必要として拒否され、unelevated backendもread-only accessを拒否した。この結果を、後述する別backendでの2026-09-29観測へ読み替えない。

### Codex CLI mxc sandboxのWindows実体確認（2026-09-29）

ローカルの実インストール済みCodex CLI `0.158.0-alpha.2.1`で`codex sandbox`を起動し、Task profileの`:workspace`継承、`windows.sandbox="mxc"`、同一filesystem table内の`:root="deny"`／`:minimal="read"`／Workspace内deny globを適用した。合成markerを用いた直接CLI processの`LIVE_RUNTIME`観測では、OneDrive上の実Repository READMEと通常Workspace markerは読取可能、OneDrive `ReparsePoint`の`.env` markerとWorkspace外の合成`C:\` markerは読取拒否、Repository内の合成書込は許可、Workspace外への合成書込は拒否された。`TEMP`／`TMP`はTask用scratchを模したOneDrive Workspace内の合成directoryを指定し、child processから同directoryへの書込も確認した。`C:\Windows\win.ini`は`:minimal=read`に含まれるため読取可能だった。従って`:root=deny`は`:minimal`等の明示範囲まで含む全path拒否を意味しない。Windows Application Control、Registry、他のOS保護設定は変更していない。

このprobeはCodex CLIの直接sandbox commandであり、`codex exec`のAgent Task、Rust Broker経路、Owner Approval、Credential、Model実行を通していない。Rustが実生成する`WorkspaceTaskScratch`の作成・cleanupやBrokerからの実Task開始も試していない。従ってこの結果は当該CLI／当該端末で指定した合成pathに対する拒否・許可だけを示し、一般的なpath isolation、実Taskの安全性、`task_execution=supported`を証明しない。Rust config testとConformanceは`mxc`および`:root=deny`の固定を検査する。実Taskの起動・scratch lifecycle・失敗／取消／期限・process終了・cleanupをBroker経由で検証するまで、`task_execution=unsupported`と関連`release_blocker`を維持する。

### Agent Task用deny patternのWindows再検査（2026-09-29）

同じ実Codex CLI `0.158.0-alpha.2.1`の`codex sandbox`を合成markerへ直接実行し、`TEMP`／`TMP`をWorkspace直下のscratchへ向けてTask環境を模した。現行profileでは`.env`、`.ssh`、`secrets/`、Workspace外markerは拒否されたが、`.env.production`と深さ10の`.env`は許可された。Task scratchへ環境変数を向けない比較ではTEMP配下の外部markerも許可されたため、Task実行時にTEMP／TMPをscratchへ固定することは外部path境界の一部である。

候補profileへWorkspace内`**/.env.*` denyとglob走査上限32を追加して再試験した結果、`.env.production`と深さ10の`.env`は拒否され、Task scratchを指定したWorkspace外markerも拒否された。別名marker `config/credential-backup.txt`は引き続き許可される。深さ32を超えるpath、実`codex exec`、Broker／Owner Approval経由、WorkspaceへのAgent書込、Task取消・crash・回復、Audit／result hash、diff／test結果／Content Exposureはこの直接sandbox試験では確認していない。この観測は`LIVE_RUNTIME`だが証明範囲は当該CLIのWindows sandboxと合成pathだけであり、`task_execution=unsupported`を維持する。

### Workspace登録secret pathのCodex Taskへの伝播（2026-09-29）

OwnerがWorkspace登録で指定したsecret pathは、登録時に正規化・保持された現在値をBrokerの`DialogueWorkspaceBinding`からTask専用contextへ渡し、Codex Taskの単一filesystem overrideへ射影する。各登録pathはWorkspace相対pathとして再検証し、完全一致と`path/**`の両方をdenyする。登録path中のglob文字として解釈され得る`[`、`]`、`{`、`}`はliteral patternへescapeする。登録数256件、生成override 12 KiBを上限とし、超過・不正pathはCLI spawn前にfail-closedで拒否する。既定deny glob、`:root=deny`、`:minimal=read`、glob走査深度32、network無効を維持する。Task contextのDebug表示はpath名でなく件数だけを出し、回復journalにはpath名を保存しない。生成したglob文字列はCodex CLIの起動引数に渡るため、ローカルのprocess command lineからpath名が見える可能性は残る。Credential実値を含めてはならない。

この接続のRust unit／Fake CLI／Conformance testは、登録値の伝播、glob生成、literal escape、上限、Debug redactionを検査する。手動構成した同等globを使うWindows mxc直接probeは合成登録secretの完全一致・子孫と深さ64の拒否を観測したが、Rust生成設定・`codex exec`・Broker／Owner Approval・実Agent Taskの連続経路証拠ではない。したがってこの追補も`LIVE_RUNTIME` sandbox probeと`FIXTURE`接続testを越えて主張せず、`task_execution=unsupported`および関連`release_blocker`を維持する。

### Rust生成Task設定を使うWindows直接probe（2026-09-29）

明示指定した実Codex CLI `0.158.0-alpha.2.1`に対するignored Rust testを追加した。testは`build_codex_command`が生成するconfig overrideをそのまま抽出し、そこからpermission profile名を取り出して必須の`codex sandbox --permission-profile`へ渡す。隔離した一時`CODEX_HOME`と合成Workspaceだけを使い、実Agent、`codex exec`、認証情報、model requestは起動しない。合成の登録file完全一致、登録directoryの子file、literal `[]`／`{}` pathのreadは拒否され、glob decoyのreadとWorkspace内writeは成功した。初回呼出しはsandbox helper必須の`--permission-profile`欠落でusage errorとなったためtestを修正した。初回のTEMP scratch補助probeはRust生成環境値をhelper commandへ正確に伝えたかを記録しておらず、期待scratchにmarkerがなかったことだけでは環境伝播を判定できなかった。後続probeでは`build_codex_command`が設定するTEMP／TMPだけを直接`sandbox`起動へ渡し、子process内の値がどちらもWorkspaceTaskScratchと一致しないことを確認した。この結果は直接`sandbox`子processの限定的な`LIVE_RUNTIME`観測であり、実`codex exec`内のAgentやそのtool childのTEMP／TMP挙動、scratch cleanupを証明しない。Broker、Owner Approval、Agent Task、process lifecycle、scratch隔離は未成立で、`task_execution=unsupported`を維持する。

2026-09-29の追補では、`tooling/codex_mxc_exec_temp_probe.py`がloopback偽Responses APIから固定`exec_command`を返し、実Codex CLI `exec`と実MxC shell childを3回実行した。現行Task permission設定相当の直接CLI probeでは、3/3回ともtool childのTEMP／TMPは互いに一致するがRust生成WorkspaceTaskScratchとは一致せず、TEMP markerはchild内で書込後、CLI終了時にはhostから見えなくなった。Workspace内scratchへの書込も3/3回成功した。これは実model、Rust Broker、Owner Approval、production Task経路、取消／期限／crashを通さない限定`LIVE_RUNTIME`証拠であり、host非可視を削除保証と扱わない。TEMP/TMPのscratch mismatchの設計上の意味と異常終端時cleanupは未解決で、Adapterの`task_execution=unsupported`およびrelease blockerを維持する。

### Broker crash後のscratch回復

未検証の予約状態を`reserved`、nofollow open後にdirectory identityを得た状態を`active`として記録上も分離する。

Task開始前にBroker永続storeへ回復記録を予約し、Workspace直下のscratch directoryを作成してnofollow handleからdevice/file IDを観測した後にだけ記録を`active`へ進める。予約記録と有効記録は既存のBroker store keyを用いるHMACで認証し、件数・byte数をboundedにする。記録内容はTask／Runtime／Workspace ID、nonceを含まない安定`recovery_binding_hash`、root identity、固定形式のscratch直接子名、active時のscratch identityに限り、絶対path、指示、出力、Credential、Permission、Approvalを保存しない。通常終了は開いたdirectory handleからscratchを削除した後に記録を完了する。

起動時は現在のWorkspace登録とrootを再検証してから回復する。`recovery_binding_hash`はRuntime／Workspace ID、除外指定、root・祖先identityからBrokerが決定論的に再計算する。これは起動ごとに変わる権限用registration hashとは別物で、Permission、Approval、Trustを生成・再利用しない。HMAC、Runtime／Workspace ID、安定binding hash、root identity、scratchの直接子名と実体identityがすべて一致し、回復開始Auditが確定した記録だけをnofollowで開いて削除する。pathが既にない場合は記録だけを再照合して解消する。作成予約後に実体identityを確認できなかったpath、symlink／reparse、identity不一致、別登録、破損journalは削除せず保持し、未解決記録があるWorkspaceのTaskをfail-closedで拒否する。記録prefixを走査して所有不明directoryを削除することはない。

この実装・Schema・negative fixture・Rust試験は局所回復契約とBrokerのWorkspace起動登録経路を検証する。回復試験ではRust test harnessの子process内でBroker libraryを起動し、journal有効化後に親から子OS processを強制終了してから、親が永続storeを再読込した新Brokerの`作業領域起動登録`でscratch削除・journal解消・回復Auditを確認する。focused試験1件とRust全target 366件が成功した。これは実filesystemとBroker libraryを用いるprocess-boundary `FIXTURE`であり、production `broker-server`／IPC listener、実Agent Task、電源断、導入製品のcleanupを含む`LIVE_RUNTIME`復旧証拠ではない。AuditEvent内の証拠分類値は試験全体の証拠範囲を昇格させない。実Broker processの異常終了を含む製品経路の検証が成立するまで、このreaperを実環境でのcleanup保証と扱わず、Agent Taskの`release_blocker`を維持する。

### Windows process群監督に対するunsafe例外レビュー

`native/process_supervision/src/windows_job.rs`だけに、Windows Job Object、停止状態threadの割当て・再開、およびJob内process数の照会に必要なWin32 unsafe呼出しを隔離する。Rust Broker crateの`#![forbid(unsafe_code)]`は変更せず、新crateは`#![deny(unsafe_op_in_unsafe_fn)]`を適用する。各unsafe blockは直前の`SAFETY`説明でpointer、handle、構造体寿命、access範囲を特定する。

標準`std::process::Child`は初期thread handleを公開しない。Jobへの割当前にchildが実行を始めないよう`CREATE_SUSPENDED`で生成し、専用Jobへ割り当ててからthreadを再開する必要がある。採用可能な既存safe APIでは、child起動前の割当てと`KILL_ON_JOB_CLOSE`の同時保証を構成できなかったため、このOS接続だけを独立crateへ隔離する。Childのprocess handleは借用のまま使い、Job、thread、snapshotのhandle所有・closeを分け、Jobにbreakaway許可を設定しない。

検証は`native/process_supervision`の強制owner終了後にchild停止を確認するWindows実process試験、およびBroker Adapter側のchild取消・回収試験で行う。これはprocess-supervision機構の`LIVE_RUNTIME`証拠であり、Codex CLI本体の起動、Task実行、sandbox、Workspace書込隔離の証拠ではない。追加依存のunsafeやこの例外の拡張は別途明示レビューし、Task能力を有効化する根拠に使わない。

## Agent Task用Workspace Permission

Agent Task用の書込Permissionは`agent_task_workspace_permission_request.schema.json`のRuntime／Session／Workspace IDだけからBrokerが解決し、OwnerがRust Desktopのnative確認を通したときだけ発行する。Permissionは現在のSession、Workspace登録hash、Adapter固定rootのdevice/file IDへ結合し、固定operation `agent_task.execute`、5分期限、一回限りとする。requestからpath、command、sandbox、Permission ID、期限を受け取らず、同じ物理Workspaceを再登録した場合やSession／Runtimeの終了・隔離後は使えない。Sessionが中止・期限超過・監査失敗・worker障害で隔離される場合、BrokerはPermission記録を直ちに破棄する。発行Auditを確定できない場合はPermissionを取り消し、Broker再起動でも揮発Permissionは失効する。

このWorkspace PermissionはTask固有のOwner Approvalではない。Rust Desktopのnative Owner確認は、現行Workspace Permissionを前提に、本文hashとRuntime／Session／Workspace登録／Permission内部識別子／固定実行条件policyから計算した条件hashへ結合したApprovalを揮発状態で発行する。発行要求本文はnative確認文に表示せず、応答・Auditへ複写しない。Approvalは5分で失効し、Task preflightは現在の本文と条件が一致する場合だけ有効と表示する。独立したRust Broker Task Consumerは実装され、実行直前の再検証・Permission／Approval一回消費・bounded状態・開始／terminal Auditへ接続された。Codex AdapterはTask専用permission profileを`-c` overrideで指定し、`--ignore-user-config`でuser configを除いた後もTaskに限って`windows.sandbox="mxc"`と`:root=deny`を明示する。Workspace TEMP/TMP scratch cleanupも実装済みだが、metadataは`unsupported`でありBroker Consumerから起動されない。Rust／Conformance testはCLI config overrideの固定内容を検査し、前述の直接CLI smokeは合成markerに対する限定的な拒否挙動を観測するだけである。Task／実隔離・失敗時Recovery・結果/diff表示およびBroker経由LIVE_RUNTIME試験は`release_blocker`であり、CLI helper、process fixture、Consumer testを製品Task実行の証拠へ昇格しない。PermissionまたはApprovalの発行だけではTask保存、process起動、filesystem書込を行わない。
