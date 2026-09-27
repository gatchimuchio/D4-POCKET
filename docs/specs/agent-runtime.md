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

このSchemaは要求境界を定義するだけで、Rust Brokerの消費経路は未接続である。Permission／Approval queueへの結合、task起動、Workspace隔離、結果・diff保存も実装しない。Codex Adapterは引き続きread-onlyであり、要求Schemaの追加によってwrite-capableな実行は有効にならない。これらは`release_blocker`として残る。
