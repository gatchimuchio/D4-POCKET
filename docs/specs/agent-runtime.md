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

Task状態recordはBroker processの揮発状態で、再起動後は状態照会できない。PermissionとApprovalも揮発し再利用できない。Broker終了またはSession隔離時のCancellation flagだけではOS child process群の終了証明にならないため、実Adapterはprocess tree supervision、期限、取消、scratch cleanupを実装・検証するまで`task_execution=supported`にしてはならない。現行Codex Adapterは、read-only Dialogueと固定`sandbox=workspace-write` Task実装の両方でWindows Job Objectを使い、Task起動時は登録Workspace直下へ乱数名scratch directoryを作成してchildのTEMP/TMPをそこへ限定し、process群停止後にhandle経由で削除する。ただしBroker capability metadataは`unsupported`のままであり、実Codex Taskは起動できない。Codex CLIのversion／help interfaceは起動時に実物確認するが、実Agentの書込隔離・拒否境界、Task取消／期限、全終端cleanup、差分内容UIおよびLIVE_RUNTIME conformanceは未成立の`release_blocker`である。`workspace-write`だけではWorkspace内の`.env`等の秘密file読取を防げず、その拒否機構は未実証である。さらにBroker crash／強制終了／電源断後に残るscratch directoryの回収機構も未実装である。Job Objectはprocess群の終了管理であってfilesystem cleanupやsandboxではない。

### Windows process群監督に対するunsafe例外レビュー

`native/process_supervision/src/windows_job.rs`だけに、Windows Job Object、停止状態threadの割当て・再開、およびJob内process数の照会に必要なWin32 unsafe呼出しを隔離する。Rust Broker crateの`#![forbid(unsafe_code)]`は変更せず、新crateは`#![deny(unsafe_op_in_unsafe_fn)]`を適用する。各unsafe blockは直前の`SAFETY`説明でpointer、handle、構造体寿命、access範囲を特定する。

標準`std::process::Child`は初期thread handleを公開しない。Jobへの割当前にchildが実行を始めないよう`CREATE_SUSPENDED`で生成し、専用Jobへ割り当ててからthreadを再開する必要がある。採用可能な既存safe APIでは、child起動前の割当てと`KILL_ON_JOB_CLOSE`の同時保証を構成できなかったため、このOS接続だけを独立crateへ隔離する。Childのprocess handleは借用のまま使い、Job、thread、snapshotのhandle所有・closeを分け、Jobにbreakaway許可を設定しない。

検証は`native/process_supervision`の強制owner終了後にchild停止を確認するWindows実process試験、およびBroker Adapter側のchild取消・回収試験で行う。これはprocess-supervision機構の`LIVE_RUNTIME`証拠であり、Codex CLI本体の起動、Task実行、sandbox、Workspace書込隔離の証拠ではない。追加依存のunsafeやこの例外の拡張は別途明示レビューし、Task能力を有効化する根拠に使わない。

## Agent Task用Workspace Permission

Agent Task用の書込Permissionは`agent_task_workspace_permission_request.schema.json`のRuntime／Session／Workspace IDだけからBrokerが解決し、OwnerがRust Desktopのnative確認を通したときだけ発行する。Permissionは現在のSession、Workspace登録hash、Adapter固定rootのdevice/file IDへ結合し、固定operation `agent_task.execute`、5分期限、一回限りとする。requestからpath、command、sandbox、Permission ID、期限を受け取らず、同じ物理Workspaceを再登録した場合やSession／Runtimeの終了・隔離後は使えない。Sessionが中止・期限超過・監査失敗・worker障害で隔離される場合、BrokerはPermission記録を直ちに破棄する。発行Auditを確定できない場合はPermissionを取り消し、Broker再起動でも揮発Permissionは失効する。

このWorkspace PermissionはTask固有のOwner Approvalではない。Rust Desktopのnative Owner確認は、現行Workspace Permissionを前提に、本文hashとRuntime／Session／Workspace登録／Permission内部識別子／固定実行条件policyから計算した条件hashへ結合したApprovalを揮発状態で発行する。発行要求本文はnative確認文に表示せず、応答・Auditへ複写しない。Approvalは5分で失効し、Task preflightは現在の本文と条件が一致する場合だけ有効と表示する。独立したRust Broker Task Consumerは実装され、実行直前の再検証・Permission／Approval一回消費・bounded状態・開始／terminal Auditへ接続された。Codex Adapterの固定workspace-write runnerとWorkspace TEMP/TMP scratch cleanupも実装済みだが、metadataは`unsupported`でありBroker Consumerから起動されない。実Task／実隔離・失敗時Recovery・結果/diff表示およびLIVE_RUNTIME試験は`release_blocker`であり、CLI help、process fixture、Consumer testをsandbox・製品実行の証拠へ昇格しない。PermissionまたはApprovalの発行だけではTask保存、process起動、filesystem書込を行わない。
