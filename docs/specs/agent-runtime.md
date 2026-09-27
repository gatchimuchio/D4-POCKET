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

Agent Taskの要求検査・Permission／Approvalを扱う各Broker操作は、構造検査済みAdapter metadataに`task_execution` capabilityがあり、その状態が`supported`であることを前提条件として毎回確認する。欠落、`unknown`、`unsupported`は`AgentTask実行非対応`として検査・発行を拒否する。加えて、Adapter実装がBroker登録rootと一致する物理Workspace識別子を提供しなければTask関連操作を拒否する。これは宣言に基づく利用可否gateと物理登録対応のfail-closed検査であり、metadataはPermission、Approval、Trust、実行可能性を付与せず、root識別子の一致もsandbox実効性や書込隔離を証明しない。対応宣言があっても、別個のBroker実行Consumerと実証済み隔離がなければTaskは起動できない。

## Agent Task用Workspace Permission

Agent Task用の書込Permissionは`agent_task_workspace_permission_request.schema.json`のRuntime／Session／Workspace IDだけからBrokerが解決し、OwnerがRust Desktopのnative確認を通したときだけ発行する。Permissionは現在のSession、Workspace登録hash、Adapter固定rootのdevice/file IDへ結合し、固定operation `agent_task.execute`、5分期限、一回限りとする。requestからpath、command、sandbox、Permission ID、期限を受け取らず、同じ物理Workspaceを再登録した場合やSession／Runtimeの終了・隔離後は使えない。Sessionが中止・期限超過・監査失敗・worker障害で隔離される場合、BrokerはPermission記録を直ちに破棄する。発行Auditを確定できない場合はPermissionを取り消し、Broker再起動でも揮発Permissionは失効する。

このWorkspace PermissionはTask固有のOwner Approvalではない。Rust Desktopのnative Owner確認は、現行Workspace Permissionを前提に、本文hashとRuntime／Session／Workspace登録／Permission内部識別子／固定実行条件policyから計算した条件hashへ結合したApprovalを揮発状態で発行する。発行要求本文はnative確認文に表示せず、応答・Auditへ複写しない。Approvalは5分で失効し、Task preflightは現在の本文と条件が一致する場合だけ有効と表示する。ただし、Approvalを一回消費してPermission・Approval・条件を実行直前に原子的再検証するTask実行consumerは未接続であり、固定policy識別子はsandbox実装やWindows隔離の証明ではない。PermissionまたはApproval発行はTask保存、process起動、filesystem書込を行わない。Codex Adapterはread-onlyのまま、実Task起動、実行前後AuditEvent、RecoveryAction、diff保存は`release_blocker`である。
