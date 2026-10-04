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

## Windows Desktop起動中のAgent CLI実行系とWorkspace登録

Desktop Agent CenterからのCLI登録要求は`AgentCLI実行系作業領域登録`だけを使い、要求Schemaは`agent_cli_runtime_workspace_registration.schema.json`に従う。要求の`adapter_id`はAdapter層の明示対応表で解決し、Broker Coreは製品固有Adapterを直接生成しない。FlutterはAdapter選択と識別子・絶対path・秘密path除外指定を入力／表示するだけで、file／process／Credentialへ直接アクセスしない。Rust起動器はPID-bound既存pipeを通った要求から固定fieldだけを読み、Adapter ID、Runtime ID、CLI実行file、Workspace ID／root、秘密path除外をWindows native default-No Owner確認へ表示する。通常資格、UI state、metadataから登録やAuthorityを作らない。

OwnerがYesを選んだ後だけ、Adapter層は対応する実物CLI interfaceを検査し、BrokerはWindows rootのnofollow・NTFS・実体identity、Broker store／資格／ProtectedStoreとの重複、AdapterとWorkspaceのroot identity一致、secret除外pathを検証する。CLI probeはstdinを閉じ、stderrを破棄し、出力上限と実行期限を適用し、環境を許可listに制限する。入力にCredential値はなく、probeはTask、Model要求、Workspace変更を開始しない。登録はBroker process内の揮発状態だけで、永続設定へ書かず、終了時に消える。件数は起動中最大8件、Workspace registryの既存上限は16件とする。

登録成功はAgent利用可能性やTrustではない。応答と監査はRuntime／Workspace ID、hash、登録状態だけを返し、path本文や秘密内容を複写しない。登録はPermission、Approval、Credential、Agent Trustを作らず、Codexの`task_execution=unsupported`を維持する。途中失敗または最終Audit失敗では新規Workspace／Runtime登録をBroker内で取り消す。Task preflight、Task実行、結果・diff表示には別々の実装・検証が必要であり、この登録経路のproduction確認を正のTask E2Eへ昇格しない。

## Agent TaskのBroker実行Consumer

`AgentTask実行`は、Task request本文からBrokerが計算したinstruction hashを使い、現在のAdapter／利用中Session／同一Workspace登録hash／Adapter固定root identityを再照合する。Adapter metadataの`task_execution=supported`に加え、Rust Adapter実装自身がTask実行対応を返すことを要求する。どちらか一方だけでは起動できない。さらに、Workspace PermissionとOwner Approvalのwall／monotonic期限、本文hash、実行条件hashを同じBroker排他区間内で再検証し、開始Auditを確定した後にPermissionとApprovalを不可分に消費してからworkerを起動する。拒否、期限切れ、本文・登録差替時はgrantを消費せず起動しない。worker起動後の取消・失敗・期限超過では、同じgrantを再利用できない。

Task Owner Approval receiptの`適用ポリシー`は`gui-shell-agent-task-sandbox-v1-max-runtime-900s`へ固定する。この識別子は開始後の最大実行時間900秒を含む固定条件を表し、Broker、native確認画面、Schemaで一致させる。識別子の一致は実行隔離やTask対応の証拠ではなく、Codex Adapterの`task_execution=unsupported` gateを変更しない。

Task状態はBroker内の独立したbounded projectionであり、通常対話のApproval・Session作業記録へ混ぜない。同一Sessionまたは同一Workspaceにつき同時実行は1件、Broker全体で最大4件、直近状態recordは最大128件とし、上限到達時は最古のterminal recordだけを監査参照を残して置換する。`AgentTask状態`と`AgentTask取消`はtask IDだけを受ける。取消応答は停止完了を意味せず、Adapter workerが実停止してterminal状態を返すまではTaskをrunningとして保持する。workerは内部受信結果へ単調完了時刻を付し、Brokerは応答を受け取った時刻でなく、Owner取消受理時刻とTask deadlineに対する完了時刻で結果を判定する。Owner取消受理後に完了した成功結果は採用せず、worker終端を確認して`cancelled`にする。取消受理前に完了した結果はBroker pollが遅れても取消要求で書き換えない。期限到達でBrokerが停止を要求した後にworkerが完了した取消応答または成功結果は、取消成功・Task成功へ昇格せず`failed`／`期限超過`に分類する。期限到達時は受信済みの期限前結果を先に確認し、結果未着の場合だけprocess停止を要求する。停止不能を示す`通信失敗`は期限超過で隠さない。取消flagと遅延応答の不採用だけではOS process群の強制終了を保証しない。

開始・終了・取消要求Auditは固定label、task ID、instruction／result hashと状態だけを保持し、本文・出力・Credentialを含めない。worker出力本文はBrokerのTask状態recordやAuditへ保存せず、Broker側でhash化後に破棄する。`result_hash`と`completed`はAgent processの応答完了を示すに限り、Workspace変更の正しさ、test成功、隔離、release readinessを証明しない。Workspace差分の列挙・内容表示は別のWorkspace inspection／Content Exposure Boundaryへ従う。worker終了Auditを確定できない場合はTaskを`quarantined`へ移し取消を要求し、成功結果を返さない。

Task状態recordはBroker processの揮発状態で、再起動後に照会できない。PermissionとApprovalも揮発し再利用できない。Broker終了またはSession隔離時のCancellation flagだけではOS child process群の終了証明にならない。Windows Job Objectの異常終了試験、fake Codex CLIでの期限超過・子孫停止、および永続Brokerの中断Task再起動監査を実装・fixture検証したが、実製品経路の一体Recoveryは未実証である。scratch回復journalと起動時reaperは、現在のWorkspace再登録時にroot identityを再照合してから回収し、不一致は保持してTaskを拒否する。現行Codex Adapterはread-only Dialogueに`--sandbox read-only`を使い、Task専用経路には`default_permissions=d4p-agent-task`のprofileを指定する。Windows Task commandは`windows.sandbox="mxc"`を明示し、filesystem policyは`:root=deny`、`:minimal=read`、`:workspace`継承、およびglob走査深度32までのWorkspace root内`**/*.env`、`**/.env.*`、`**/.ssh/**`、`**/secrets/**` denyを組み合わせる。networkも無効にする。Task起動時は登録Workspace直下へ乱数名scratch directoryを作成し、Rustが起動するCodex CLI process自身の`TEMP`／`TMP`へ設定して、process群停止後にhandle経由で削除する。ただし、実`codex exec`のMxC shell childでは`TEMP`／`TMP`が当該scratchと一致しなかった。CLIの`shell_environment_policy.set`にも同じscratchを明示した追試でも一致せず、その理由とMxC child一時領域の物理cleanupは未確認である。したがってこのscratchをAgent tool childの一時領域境界やcleanup保証として扱わない。Task profileの実効範囲は当該CLI／Windowsと個別probeの範囲に限られ、深度32超や列挙されていない別名secret pathを包括的に拒否する保証ではない。通常ReleaseのBroker capability metadataは`unsupported`のままであり、実Task pathはまだ通常Releaseへ開いていない。実Agentの書込隔離・外部path拒否、Task deadline／crashからのinstalled product一体Recovery、通常終端cleanup、差分内容UIおよびLIVE_RUNTIME conformanceは未成立の`release_blocker`である。Job Objectはprocess群の終了管理であってfilesystem cleanupやsandboxではない。

### 中断Taskの再起動時隔離（2026-10-04）

永続Brokerは起動時に既存Audit chainを走査し、Task開始Auditに完了・失敗・取消の終端Auditが結び付かないtask IDを検出する。そのTaskは再開せず、`Agent Task中断回復（状態隔離・RecoveryAction=Workspace差分を確認）`／`suspended`／`INTERNAL_STATE`を一度だけ追記する。回復記録は開始event hashから計算し、指示本文、Agent出力、Permission、Approval、Credential、pathを含めない。既に終端Auditまたは回復AuditがあるTaskは再分類せず、Permission／Approvalも履歴から復元しない。これはTaskが中断状態にあることを示す内部Audit判定であり、process停止原因をcrashと断定する証拠ではない。

2026-10-04のWindows testでは、未終端Taskを再起動後に隔離すること、完了Taskを隔離しないこと、再々起動でAuditを重複させないことを検証した。別のprocess-boundary testではBroker library process強制終了後のscratch回収とRecovery Audit、Windows Job Object testでは異常終了したowner processの子・孫process停止、Adapter fake CLI testでは期限超過後の子孫heartbeat停止とBroker scratch解消を確認した。Broker永続再起動・scratch回収は`FIXTURE`、Windows process supervisionとfake CLIの指定process挙動は各fixture境界に限る。実Codex Taskを実行中のinstalled D4 Pocketを落とし、Task終端・process群・Recovery／Audit・Workspace結果を一体で確認する`LIVE_RUNTIME`検証は未実施であり、R2 blockerと`task_execution=unsupported`を維持する。

2026-09-30の実装では、Rustが生成する全WorkspaceWrite commandに`shell_environment_policy.set`で同じBroker-owned scratchを`TEMP`／`TMP`として明示し、test API専用だった設定をproduction Task経路へ移した。固定command testとfake CLI fixtureはこの設定値を検査する。これはCodex CLIへの設定伝達を示す`FIXTURE`であり、MxC childの実環境値がscratchと一致することや内部一時領域の削除を示さない。既存の直接CLI観測では明示設定後も値が一致しなかったため、この変更を隔離・cleanupの解決として扱わず、`task_execution=unsupported`とrelease blockerを維持する。

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

Codex CLIの`:workspace_roots`配下にある`deny` globはread拒否であり、matching fileの作成・変更を拒否しない（[OpenAI公式Permissions仕様](https://learn.chatgpt.com/docs/permissions)）。実MxC shell childの合成probeでも、`.env`、`.env.*`、`.ssh/**`、`secrets/**`の読取は各3/3回拒否された一方、同じpattern内への合成書込は各3/3回成功した。従って広域deny globをwrite隔離の証拠にしてはならない。Owner登録secret pathからRustが生成するliteral exact `deny`は別規則であり、実Rust生成profileを用いたWindows sandbox試験では登録fileの読取・新規書込と登録directory配下の読取・新規作成を拒否し、通常Workspace書込は許可した。未登録alias path、Brokerからの実`codex exec` tool child、実Agent Task、cancel／deadline／crash後のprocess群停止・cleanupは未検証であり、`task_execution=unsupported`と関連`release_blocker`を維持する。

### Rust生成Task設定を使うWindows直接probe（2026-09-29）

明示指定した実Codex CLI `0.158.0-alpha.2.1`に対するignored Rust testを追加した。testは`build_codex_command`が生成するconfig overrideをそのまま抽出し、そこからpermission profile名を取り出して必須の`codex sandbox --permission-profile`へ渡す。隔離した一時`CODEX_HOME`と合成Workspaceだけを使い、実Agent、`codex exec`、認証情報、model requestは起動しない。合成の登録file完全一致、登録directoryの子file、literal `[]`／`{}` pathのreadは拒否され、glob decoyのreadとWorkspace内writeは成功した。初回呼出しはsandbox helper必須の`--permission-profile`欠落でusage errorとなったためtestを修正した。初回のTEMP scratch補助probeはRust生成環境値をhelper commandへ正確に伝えたかを記録しておらず、期待scratchにmarkerがなかったことだけでは環境伝播を判定できなかった。後続probeでは`build_codex_command`が設定するTEMP／TMPだけを直接`sandbox`起動へ渡し、子process内の値がどちらもWorkspaceTaskScratchと一致しないことを確認した。この結果は直接`sandbox`子processの限定的な`LIVE_RUNTIME`観測であり、実`codex exec`内のAgentやそのtool childのTEMP／TMP挙動、scratch cleanupを証明しない。Broker、Owner Approval、Agent Task、process lifecycle、scratch隔離は未成立で、`task_execution=unsupported`を維持する。

2026-09-29の追補では、`tooling/codex_mxc_exec_temp_probe.py`がloopback偽Responses APIから固定`exec_command`を返し、実Codex CLI `exec`と実MxC shell childを3回実行した。現行Task permission設定相当の直接CLI probeでは、3/3回ともtool childのTEMP／TMPは互いに一致するがRust生成WorkspaceTaskScratchとは一致せず、TEMP markerはchild内で書込後、CLI終了時にはhostから見えなくなった。Workspace内scratchへの書込も3/3回成功した。続く一回の追試では、Codex CLIの`shell_environment_policy.set`にも同じ合成scratch pathを指定したが、MxC childのTEMP／TMPは引き続きscratchと一致せず、Packages配下の`AC/Temp`形式だった。これらは実model、Rust Broker、Owner Approval、production Task経路、取消／期限／crashを通さない限定`LIVE_RUNTIME`証拠であり、host非可視を削除保証と扱わない。MxC child一時領域の物理cleanup、scratch mismatchの設計上の意味は未解決で、Adapterの`task_execution=unsupported`およびrelease blockerを維持する。

### Codex CLIの非Git Workspace起動条件（2026-09-29）

登録済みWorkspaceはGit repositoryであることを必須にしない。実Codex CLI `0.158.0-alpha.2.1`は、Git管理外の合成Workspaceに対して`codex exec`を`--skip-git-repo-check`なしで起動すると、Responses APIへ要求する前に`Not inside a trusted directory and --skip-git-repo-check was not specified.`で終了した。実Codex CLIの`exec --help`は当該optionを提示し、既存の隔離loopback偽Responses API probeは同option付きで3回とも固定tool commandとturnを完了した。実Rust Adapterの共通`exec` command builderはread-only DialogueとTaskの両方へ同optionを固定し、起動時のCLI interface検査で要求する。非対応CLIはAdapter初期化を拒否してfail-closedにする。

`--skip-git-repo-check`はCodex CLI自身が破壊的変更防止のため設けるGit repository確認を迂回するoptionであり、無害な一般設定として扱わない。OpenAI公式説明も、環境の安全性を確信している場合に限って確認を迂回する用途としている（[Non-interactive mode](https://learn.chatgpt.com/docs/non-interactive-mode?translationFallback=ja-JP)）。本経路ではBroker登録済みWorkspaceと現行Sessionの結合を維持する。read-only Dialogueは従来の`--sandbox read-only`を維持し、TaskはWorkspace write Permission、別個のOwner Approval、Task用sandbox設定を維持する。どちらもCLI option・Workspace内容・Adapter metadataからAuthorityを生成しない。この変更は両`exec`経路でCLI側Git確認を弱める限定的なtrust変更であり、Brokerやsandboxを通る実Taskの隔離成立を証明しない。Owner承認、実BrokerからのTask起動、実Workspaceの秘密file／外部path拒否、process lifecycleとcleanupは未成立であり、`task_execution=unsupported`と関連`release_blocker`を維持する。

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

Desktop Flutter BrokerClientとWindows RunnerのPID-bound pipeは、同じnative Owner確認operation集合を長いが有限の応答待ちとして扱う。pipeの310秒上限は最大300秒のnative確認待ちをtransportが途中切断しないためのもので、通常要求は5秒を維持する。この待ち時間はOwner確認、Permission／Approval、Task実行の権限を生成せず、Flutter／pipe timeoutもBroker処理の取消または発行拒否を意味しない。呼出側はtimeoutを承認・拒否のどちらにも読み替えず、状態を再照合してから再操作する。最終判断はRust Brokerの現行stateとnative Owner確認に限る。

このWorkspace PermissionはTask固有のOwner Approvalではない。Rust Desktopのnative Owner確認は、現行Workspace Permissionを前提に、本文hashとRuntime／Session／Workspace登録／Permission内部識別子／固定実行条件policyから計算した条件hashへ結合したApprovalを揮発状態で発行する。発行要求本文はnative確認文に表示せず、応答・Auditへ複写しない。Approvalは5分で失効し、Task preflightは現在の本文と条件が一致する場合だけ有効と表示する。独立したRust Broker Task Consumerは実装され、実行直前の再検証・Permission／Approval一回消費・bounded状態・開始／terminal Auditへ接続された。Codex AdapterはTask専用permission profileを`-c` overrideで指定し、`--ignore-user-config`でuser configを除いた後もTaskに限って`windows.sandbox="mxc"`と`:root=deny`を明示する。Workspace TEMP/TMP scratch cleanupも実装済みだが、metadataは`unsupported`でありBroker Consumerから起動されない。Rust／Conformance testはCLI config overrideの固定内容を検査し、前述の直接CLI smokeは合成markerに対する限定的な拒否挙動を観測するだけである。Task／実隔離・失敗時Recovery・結果/diff表示およびBroker経由LIVE_RUNTIME試験は`release_blocker`であり、CLI helper、process fixture、Consumer testを製品Task実行の証拠へ昇格しない。PermissionまたはApprovalの発行だけではTask保存、process起動、filesystem書込を行わない。

2026-09-30、明示指定した実Codex CLI `0.158.0-alpha.2.1`を資格情報なしloopback偽Responses APIへ接続し、Windows ignored Rust試験からBrokerのWorkspaceRegistry、Task Consumer、`WorkspaceTaskScratch`、process群監督を通して固定Taskを実行した。合成登録secretのreadとWorkspace外read／writeは拒否され、許可Workspace write、Task終端後のBroker管理scratch不在、開始／完了Audit callbackを確認した。PermissionとTask Owner Approvalが揃わない拒否要求ではCLI接続自体が起きず、実行後のPermission／Approval再利用も拒否された。非loopback要求はloopback proxyが拒否した。

この試験の`Broker統合CodexFixtureAdapter`は、製品Adapterの`task_execution=unsupported`をassertした後、試験内だけ能力metadataを`supported`へ上書きし、`FIXTURE`出所を付す。Owner確認はBroker libraryへのsynthetic confirmation値であり、Desktop native確認UIやproduction IPC／`broker-server`、耐久AuditStoreを通らない。実CLI／MxC childのprocessとfilesystem動作は当該Windows上の`LIVE_RUNTIME`だが、Authority、owner同意、metadataはfixture境界である。偽APIと固定command、stream retry、marker更新は試験専用であり、製品retry policyや実Agentの相互運用を示さない。

先行試行ではWindows test serverの受入socketがnonblocking状態を引き継ぎ、`read_line`の`WSAEWOULDBLOCK`でHTTP要求前に切断した。受入socketをblockingへ戻すfixture修正後も、CodexのResponses stream body decodeが一時失敗したため、固定toolをmarker有無で冪等に応答する限定fixture retryを設けた。最終の明示LIVE試験は3回成功したが、失敗試行は履歴に残し、成功反復へ加算しない。

その後の最終source再試行では2回成功後の1回がTask `failed`で終わった。観測上はResponses API 3 POST、合成Workspace marker作成済み、非loopback `chatgpt.com` CONNECT拒否1件であり、失敗原因は特定できていない。本文や資格値を出さない応答解析失敗要約を試験専用で追加したが、失敗は再現せず、その後の単発1回と連続5回は成功した。漏えい否定assertを最終fixture文面へ修正した後も、最終sourceでさらに5回連続成功した。孤立失敗は履歴・blockerへ残し、反復成功で安定性や原因解明を主張しない。

この結果は正常終端時のin-process Broker統合経路を狭く検証するものである。OneDrive Cloud Filesと通常NTFSの双方、深度／hardlink aliasの実tool-child拒否、実Desktop Owner確認、production Broker IPCとdurable Audit、cancel／deadline／crash時のprocess停止とRecovery、MxC内部TEMPの物理cleanup、result/diff表示、実provider interoperabilityは未検証のまま残る。従って製品metadataは`unsupported`、関連`release_blocker`、`release_ready=false`を維持する。

### 登録secretのhardlink alias再検証（2026-09-29）

完全一致pathのMxC denyは、同一fileを指す未登録hardlink aliasまで名前解決する保証ではない。合成secret fileへのhardlinkをsandbox起動前に作った初回probeでは、登録pathのdenyを迂回してalias側からreadでき、probeはexit code 45となった。この結果を受け、`WorkspaceReader::from_registered_dir`は登録secret fileと登録secret directory配下をhandle経由でbounded走査し、regular fileのlink countが1でない場合、reparse／volume境界等を含むunsafe状態としてfail-closedに拒否する。走査上限は深さ64、合計4096 entryであり、上限超過も拒否する。Task起動直前にはAdapterが登録root identityを再照合して同じ検査を実行し、登録後・起動前に追加されたaliasも起動させない。これらRust unit testは`FIXTURE`である。

さらに、`build_codex_command`由来のTask permission overrideを用いたWindows直接`codex sandbox` probe内で、PowerShell `New-Item -ItemType HardLink`により実行中の合成secret aliasをWorkspace内へ作ろうとした。実Codex CLI `0.158.0-alpha.2.1`の当該直接MxC childでは作成が`UnauthorizedAccessException`、HRESULT `0x80070005`で拒否され、通常Workspace writeは引き続き成功した。同じprobeでは登録secretの深さ40 pathと大文字・区切り別aliasのread／writeも拒否された。これは合成Workspace内で実施した直接sandbox childの`LIVE_RUNTIME`観測であり、Rust Brokerからの実`codex exec` tool child、Owner Approval、production Agent Task、他CLI版や他のalias形式を証明しない。先行probeで観測した起動前hardlink迂回と、実`codex exec` tool childでのhardlink作成・read拒否の連続検証は同一の証拠へ混同しない。

2026-09-30の追試では、loopback偽Responses APIから固定命令を返す実Codex CLI `exec`を通じて、実MxC `exec_command` child内から同じ合成secretを指すhardlink作成を3回試した。3/3回とも`Win32Exception` HRESULT `0x80004005`、Win32 `NativeErrorCode 5`（アクセス拒否）で作成に失敗し、aliasはhost側にも現れなかった。通常Workspace read／writeは成功し、合成secret本文は出力していない。証拠は手動構成したRust相当permission profileを使う直接CLIの`LIVE_RUNTIME`であり、Rust生成設定・Broker・Owner Approval・production Agent Taskを通していない。TEMP／TMPとWorkspaceTaskScratchの不一致も残るため、Task capabilityは`unsupported`のままとする。

同じ実MxC child検査へ起動前aliasを追加した。合成登録secret fileをhost側でhardlink化し、同一file identityとlink count 2を確認してからCodex CLIを起動したところ、未登録aliasからのreadは3/3回成功し、登録exact pathからのreadは3/3回拒否された。動的hardlink作成は別のexact-deny synthetic fileを対象にし、引き続き3/3回NativeErrorCode 5で拒否された。従って、MxC exact-path denyは起動前hardlink aliasを防がず、RustのWorkspace登録／Task preflightでmultiple linkを拒否してからspawnする境界が必要である。probeはそのRust preflightを意図的に迂回した直接`LIVE_RUNTIME`であり、Brokerやproduction Taskの挙動を示さない。`--runs 3`のbatch試行2回は最初のCLI呼出しでcommand exit 0後にResponses stream decode errorとなったためturn成功数へ加えず、個別`--runs 1`の完了turn 3件だけを反復数とする。

TEMP／TMPがMxC child内でBroker `WorkspaceTaskScratch`と一致しない問題も残る。したがって、登録時／Task直前のhardlink検査と直接MxC childの作成拒否は限定された対策・証拠であり、Agent Adapter capabilityを`supported`へ変更しない。Broker経由の実Task、Owner承認、深度超過・別名secretを含む実tool child隔離、cancel／deadline／crash時停止、Audit／Recovery、scratch cleanupを検証するまで`task_execution=unsupported`、関連`release_blocker`、`release_ready=false`を維持する。

### Task spawn直前の登録secret再検査（2026-09-30）

Task Adapterは、scratch作成前の登録secret検査に加え、WorkspaceWrite用commandを構成した後、process生成の直前にも登録Workspaceとsecret pathを再検査する。Windows fixtureはscratch作成後に登録secretのhardlink aliasを加え、実spawn関数がCLI起動へ進む前に`作業領域不在`で拒否することを確認する。これは`FIXTURE`証拠であり、Workspace contentsを外部processから原子的に固定するものではないため、最終検査とprocess生成の間に起きる同時変更を排除しない。MxC tool-child隔離／TEMP cleanupの実証でもない。Task capabilityは`unsupported`のまま維持する。

### 通常Broker IPCからのAgent Task権限発行拒否（2026-09-30）

Windows向けBroker実測収集器の第6版は、通常のloopback接続資格を用いて本番Broker接続口へ`AgentTaskWorkspacePermissionGrant`と`AgentTaskOwnerApprovalGrant`を別々に送り、両方が`desktop_native_owner_confirmation_required`で拒否されることを検査する。現行Rust実装からbuildした単体Release helperによる`LIVE_RUNTIME`実測で両拒否を確認し、収集器とrelease evidence validatorの必須条件にした。この否定経路が示すのは通常IPCによるOwner権限発行の拒否だけであり、Rust Desktop起動器上でのOwner確認成功、確認後の実Task、installed product、durable Audit統合を証明しない。`task_execution=unsupported`とrelease blockerは維持する。

### 実Codex CLI Broker Taskの取消観測（2026-09-30）

Windowsのignored Rust統合testは、実Codex CLI `0.158.0-alpha.2.1`と実MxC shell childを、localhost限定の偽Responses APIから固定合成命令だけ受ける構成で起動する。既存の正常Taskに続けて、合成Workspace内でheartbeat fileを更新し続けるtool childをBroker `AgentTask取消`から停止し、terminal状態が`cancelled`、結果hashなし、取消後のheartbeat不変、Broker所有`.d4p-tmp-` scratchなし、開始／取消要求／terminal Auditありを確認する試験を追加した。Audit callback、Owner確認、Task capability metadata上書きはRust test fixtureである。

この取消経路を最後まで通った観測は1回である。元の正常Task段階は過去にも成功実績がある一方、2026-09-30の再試行2回ではResponses stream切断と`chatgpt.com` CONNECT 1件（loopback proxyが拒否）が発生し、新しい取消段階へ到達しなかった。HTTP request parse、server応答書込、loopback外接続拒否にfixture側の異常は観測されず、原因は未確定である。既知slugへの試験model差替えもtool childを起動できず、採用せず戻した。失敗を成功へ読み替えない。

この`LIVE_RUNTIME`観測の範囲は、当該Windowsで`Codex CLI`の子process群がRust試験内Broker consumerからの取消要求によって停止した一度の挙動に限る。権限発行、`Adapter`の対応metadata、監査保存callbackには`FIXTURE`を使い、実製品の`broker-server`／IPC、Desktop native Owner画面、耐久Auditを通していない。反復可能な隔離、期限・crash後のRecovery、MxC内部一時領域の物理cleanupも示さない。Rust管理scratchとMxC内部TEMPは別の一時領域である。Codex Adapterの`task_execution=unsupported`、関連`release_blocker`、`release_ready=false`を維持する。

### MxC AppContainer一時領域とBroker scratchの責任分離（2026-10-01）

Broker-owned `WorkspaceTaskScratch`と、MxC tool childが実際に受け取る`TEMP`／`TMP`は別の領域として扱う。Windows AppContainerは`TEMP`／`TMP`をAppContainer profile配下の`AC\Temp`へリダイレクトする（[Microsoft Learn: Launch an AppContainer](https://learn.microsoft.com/en-us/windows/win32/secauthz/implementing-an-appcontainer)）ため、child値がBroker scratchと一致することを安全不変条件にしない。Broker scratchはBrokerが所有する作業・回収対象であり、AppContainer側の一時領域はMxC／Windows sandbox profileの寿命と権限境界に従う。

Task対応を成立させるには、Broker scratchとAppContainer一時領域の双方を区別した上で、production Broker経路におけるtask間・同時task間の分離、実際のsandbox process群停止後の通常終了／取消／期限超過／crash時cleanup、およびcleanup失敗時のRecoveryを検証する。AppContainer環境handleのclose、child終了後にhostからpathが見えないこと、または`TEMP`／`TMP`がBroker scratchと違うこと単独では、物理削除・task間非共有を証明しない。条件が閉じるまでは`task_execution=unsupported`と既存`release_blocker`を維持する。

### Release Broker processの未対応Task拒否経路（2026-10-01）

Rust source／testがcommit `8ec3434b3a713a00654b97c14d19e21cb31e69a7`と一致するWindows Release Broker processを起動し、実Codex CLI `0.159.2`の登録だけを行う統合testを追加した。通常認証loopback IPCでAgent Sessionを作成した後、Task要求検査・Task起動はmetadataの`unsupported`により拒否され、通常資格によるWorkspace Permission／Owner Approval発行要求もnative Owner確認必須として拒否される。拒否AuditはBroker終了後に永続storeから再読込でき、指示本文と合成secretは応答・Auditに含まれない。

この`LIVE_RUNTIME`証拠はRelease helper process、通常認証IPC、実CLIの登録・能力投影、Brokerの拒否とfile-backed Audit再読込に限る。Workspace／store／secretは合成`FIXTURE`で、native Owner UIは操作せず、Task processは起動していない。scratchが作成されないことはTask後始末の証拠ではない。正の実Task、installed Desktop経路、sandbox隔離、取消／期限／crash Recovery、MxC一時領域の後始末、結果／diffのContent Exposureは未成立であり、Codex Adapterを`task_execution=unsupported`に保つ。

### MxC TEMP実体の正常終了・取消後照合（2026-10-01）

実Codex CLI `0.159.2`と実MxC shell childを使うignored Rust統合testへ、固定loopback偽Responses APIがchild内で報告したTEMP pathを、Task terminal後に単一directoryとして照合する観測を加えた。親test processから実行中markerを読め、正常完了後とBroker取消後はいずれもmarker readが`NotFound`、同じTEMP rootの`read_dir`も`NotFound`となった。観測結果は状態とentry件数だけを出し、絶対path・entry名・本文を記録しない。focused runは1回で、正常完了と取消を各1回観測した。

この`LIVE_RUNTIME`範囲は当該CLI版とMxC childの正常完了／Broker取消後に、報告されたTEMP directory pathが存在しなかったことだけである。Broker consumer、Owner判断、Audit callback、Workspace、CODEX_HOME、loopback APIは`FIXTURE`であり、installed Desktop UI、native Owner confirmation、Release Broker／authenticated IPC上の正Task、file-backed positive Audit、deadline／process crash／電源断、全一時entryのcleanupを証明しない。TEMP rootが終了後に不在だった一度の観測をtask間隔離や異常終了cleanupへ一般化せず、`task_execution=unsupported`、関連`release_blocker`、`release_ready=false`を維持する。
### 2026-10-04 run50 Audit projection補正

起動時の中断Task判定は永続AuditのTask IDを相関し、開始／終端の固定意味markerをAuditEvent.operationまたはAuditEvent.reasonの完全一致で検出する。Broker operation名と意味markerを別fieldへ射影するproduction Auditと、旧形式で意味markerをoperationへ直接置く記録の双方を読む。Task本文からmarkerを導出せず、Permission／Approvalも復元しない。

修正前sourceのinstalled run50ではoperation=AgentTask実行、reason=Agent Task実行開始（Permission／Owner Approval一回消費）の実記録を再起動Recoveryが見落とし、同Task IDのRecovery Auditを追加しなかった。この失敗で旧単体fixtureがproduction Audit形状を再現していないことが分かった。コードを両field照合へ修正し、production形状、旧形状、完了Task除外、再起動冪等性のfocused testは1 passed。crash fixtureの準備PID札はatomic rename公開へ変更し、空file読みraceもfocused testで再検証した。

修正後sourceを使ったinstalled D4 Pocket実Codex Task中のLauncher／Broker crash、process群停止、永続Recoveryの統合LIVE_RUNTIME検証はまだ未実施。run50はRecovery成功の証拠ではなく、R2 blockerと通常Release task_execution=unsupportedを維持する。

### D4 Pocket rev3 R2有限完了条件

R2の未完了範囲を、現行ユーザー指示で次の有限条件に固定する。すでに成立したpositive Task completion、Permission／Approval一回消費、durable Auditの再起動読戻し、native tray正常終了、局所cross-Workspace隔離は、対応する既存証拠を再利用し、fixture名を変えただけの再試験を要求しない。Task／Agent間のscratch・状態隔離は別の未成立条件であり、既存cross-Workspace試験と同一視しない。

R2を閉じるために残る条件は、(1) installed active Taskの取消／期限到達・process子孫停止・Recovery、(2) active Task中のCodex／Broker／Launcher crash・process子孫停止・Recovery、(3) installed product経路におけるTask間scratch／Agent状態の相互非汚染、(4) result／diff／changed files／test resultのContent ExposureをAgent Centerへ接続、(5) OneDrive Cloud Filesと通常NTFSの双方でinstalled経路のsecret／Workspace境界を検証、(6) D4 Pocketが所有するWorkspaceTaskScratchの保証範囲だけを確定・検証、(7) 通常Releaseでの有限な`task_execution`昇格gate、(8) runtime provenanceとinstalled artifact identityを現行Release evidenceへ結合、である。各条件の実行・判定は`release_blockers.registry.json`の`r2_bounded_exit_gate`に同期する。

MxC childの`TEMP`／`TMP`がWindows AppContainer profileへリダイレクトされる場合、その内部directoryの物理削除はD4 Pocketの保証として主張しない。D4 Pocketが保証すべき対象はBroker-owned `WorkspaceTaskScratch`の範囲・Task間分離・通常終了／取消／期限／crash後の停止と回収、および回収不能時のfail-closed Recoveryである。MxC／Windows側のTEMP寿命を観測しただけでD4のscratch cleanupを代替せず、逆にD4の保証外であるTEMP削除をR2 blockerへ追加しない。

通常Releaseの`task_execution`は、上記8条件と既存の閉鎖条件がすべて適切な証拠classでPASSした場合に限り`supported`へ変更する。PASS後は追加の仮説的riskを理由に`unsupported`を維持しない。未解決項目がある間はfail-closedで`unsupported`を保つ。R2完了後は新たな局所最適化を挟まずR3 Multi-Agent Compareへ進み、R3完了後にR4 Handoffへ進む。
