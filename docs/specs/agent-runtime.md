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
