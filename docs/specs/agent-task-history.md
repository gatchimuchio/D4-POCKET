# Agent Task履歴の契約

## 目的と証拠範囲

Agent Task履歴は、Brokerが確定したAuditからTaskごとの最後の状態を読み取るための履歴投影である。通常対話のExecution Historyとは別の一覧として提供し、Task本文・Agent結果本文を保存または返却しない。

履歴項目は`INTERNAL_STATE`の過去観測であり、現在のRuntime稼働、Task実行継続、成功の外部証拠、Workspaceの現在内容を保証しない。`running`も開始Audit以後に終端Auditがないという状態で、現在実行中の証明ではない。Broker再起動で開始だけが残ったTaskは再開せず、Recovery Auditにより`suspended`として投影する。

## 閲覧境界

- 閲覧は既存のOwner発行・期限付き履歴承認を再利用し、承認に結び付いたRuntimeだけを返す。
- 要求からRuntime ID、Workspace、Permission、Approval、Authority、credentialを指定できない。
- 履歴を読むための現在承認はBrokerが保持し、再起動後に復元しない。
- pageは最大50件。cursorと監査chain headを返す。
- Task履歴はTask開始・終端の構造化Audit recordからBrokerが再構成する。Audit recordのpayload hash、要求ID、stage、状態、Runtime／Session／Workspace結合、開始Audit参照を照合し、不整合時は拒否する。
- 表示項目はTask ID、Runtime／Session／Workspace ID、状態、時刻、instruction／result hash、failure class、開始／最新Audit参照に限定する。本文、path、Credential実値、Permission／Approval ID、隠れた状態を含めない。

履歴情報はPermission／Approval／Credential／Trustを発行、再利用、復元しない。再実行、Workspace変更、結果表示は、それぞれ現在の独立したBroker統治経路を必要とする。

## 旧Auditとの互換境界

構造化Task metadataの導入前に作られたAgent Task AuditにはRuntime／Session／Workspaceを安全にRuntime別へ結び付ける公開情報がない。その記録は書換え・推測補完せず、新しいTask履歴一覧へ混ぜない。既存Audit chainとRecovery用途は保持する。

## 接続面

- Broker操作名: `AgentTask履歴閲覧`
- 要求Contract: `specs/runtime_agent_task_history_request.schema.json`
- 履歴ページContract: `specs/agent_task_history_page.schema.json`
- 応答Contract: `specs/runtime_agent_task_history_access.schema.json`
- Rust製品経路の投影: `native/rust_helper/src/broker/execution_history.rs`
- Runtime別の閲覧承認強制: `native/rust_helper/src/broker/history_access.rs`
- Desktopの呼出し元と画面: 共通UI `HistoryClient` と `HistoryScreen`

Taskのinstruction本文・result本文の表示は、この履歴APIの責務外である。結果本文を見せる既存のAgent Center経路では、個別のContent Exposure承認を引き続き要求する。
