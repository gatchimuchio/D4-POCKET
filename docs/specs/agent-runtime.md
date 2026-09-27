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

対話要求への一回限りのOwner承認（対話Approval）は、Agentに指示を送る承認であり、Workspaceへの書込みPermissionやAgent作業Taskの実行承認ではない。作業実行には、Brokerが解決する限定Workspace Permission、Task本文と実行条件に結合した別個の一回限りOwner Approval、実行前後のAuditEvent、失敗時RecoveryActionをすべて要求する。Agent metadata、UI、Task本文、履歴、Profile、Capability宣言からこれらを生成・再利用しない。指示本文のhashはBrokerが計算し、Task要求元の自己申告hashを受け付けない。監査・log・traceには本文やCredential実値を複写せず、必要なhashと状態だけを保持する。

Rust Brokerの`Agent作業要求検査`はこの要求を受け、現在のAgent Adapter、利用中Session、Session作成時と同一のWorkspace登録hashを再照合する。検査成功時も返すのはBroker計算の指示hash、「未実行」状態、現在のSession・Workspace登録hashに結び付くPermissionの有効／未付与状態、Task Approval未取得だけであり、要求本文やPermission IDを応答・Auditへ複写せず、Taskを保存・起動しない。Permission状態はBroker内の現行記録、二つの期限、Session／Workspace／登録hashの一致から毎回再評価する。これは`INTERNAL_STATE`の登録関係検査であって、Runtime稼働やWorkspace隔離の証拠ではない。Broker envelopeの同一nonce再送は既存replay防護が拒否するが、新しいnonceによる同一内容の検査は副作用のない再検査であり、Task実行承認ではない。

## Agent Task用Workspace Permission

Agent Task用の書込Permissionは`agent_task_workspace_permission_request.schema.json`のRuntime／Session／Workspace IDだけからBrokerが解決し、OwnerがRust Desktopのnative確認を通したときだけ発行する。Permissionは現在のSessionとWorkspace登録hashへ結合し、固定operation `agent_task.execute`、5分期限、一回限りとする。requestからpath、command、sandbox、Permission ID、期限を受け取らず、同じ物理Workspaceを再登録した場合やSession／Runtimeの終了・隔離後は使えない。発行Auditを確定できない場合はPermissionを取り消し、Broker再起動でも揮発Permissionは失効する。

このWorkspace PermissionはTask固有のOwner Approvalではない。各Task実行には引き続き本文hashと実行条件へ結合した別個の一回限りOwner Approvalを必須とし、PermissionだけではTask保存、process起動、filesystem書込を行わない。現時点でBrokerにはTask実行consumerがなく、Task実行経路は未接続であるため、Permission grantの発行は実行可能化を意味しない。Codex Adapterはread-onlyのまま、実Task起動、実行前後AuditEvent、RecoveryAction、diff保存は`release_blocker`である。
