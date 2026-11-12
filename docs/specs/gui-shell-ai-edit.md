# GUI Shell編集提案の意味正本

## 目的

`GUI Shell編集提案`は、Owner／Developerが明示開始した実装Agentの要求を、構成・UI・Contractの変更候補として審査待ちへ記録する開発経路である。製品Runtimeが自分自身を変更する機能ではない。

## Broker経路

Owner資格経路から編集指示、対象path、予定変更を認証済みRust Brokerへ渡す。Brokerは版、識別子、範囲、対象path、規約確認、自己承認禁止、`proposal_only`を再検証し、`INTERNAL_STATE`のAuditEvent付きReceiptを返す。指示本文はReceiptへ返さず、要求hashだけを結合する。

## 適用範囲

対象pathは`apps/desktop_flutter`、`packages`、`specs`、`docs`、`tooling`、`native/rust_helper/src`、`規定`の境界内に限定する。`.git`、Manifest、release evidence、secret、credential、private、signingに関係するpathは拒否する。外部Agentのmetadataや履歴はAuthority、Permission、Approvalを生成しない。

## 自動適用禁止

現行単位の`execution_mode`は`proposal_only`、`review_required=true`、`apply_status=not_started`、`files_written=false`に固定する。編集Agentの提案をそのままapplyせず、Owner／Developerが差分、Contract、テスト、監査、権限境界を確認してから、別の明示的な開発作業として実装する。自己承認、Permission生成、Approval迂回、秘密値取得は行わない。

## Agent Centerへの別Task受け渡し

Settingsの編集提案画面は受理Receipt後、指示本文、対象path、予定変更、指示hash、Audit IDをOwner／Developerが確認できる。明示的な確認後、本文とReceipt参照を未信頼contextとしてAgent Centerへ渡し、登録済みSessionの選択後に既存Agent作業要求検査だけを行う。Agent Centerの事前検査Dialogでは内容を固定表示し、MCP能力追加を要求しない。

この受け渡しはproposalの自動applyではなく、独立した通常Broker Task要求の準備である。受け渡し自体はPermission、Approval、Credential、Agent trust、Task実行を生成・継承しない。target_pathsはTaskの希望範囲であり、OSまたはBrokerのアクセス境界ではない。実際のアクセス範囲はBroker Workspace Permissionが決める。Task指示は対象path内に限定し、必要範囲が合わない場合は書込み前に停止してOwner／Developerへ報告する。通常Broker経路で現在のWorkspace PermissionとTaskごとの一回Owner Approvalを別々に取得する。Task事前検査が成功するまで下書きを保持し、取消・Broker拒否時は下書きを消費しない。受理後もファイル変更や実行完了と誤表示しない。

## 完了結果からの制御適用Task準備

完了したAI Edit Taskの結果を別Workspaceへ引き継ぐ場合、Agent Centerは、元proposalのedit ID／要求payload hash／Audit ID、元Task ID／指示hash、Brokerが返した完了状態とresult hash、現在のContent Exposure許可がすべて一致したときだけ結果を表示対象にする。Owner／Developerがfull Content Exposureを別途許可した場合は、boundedな結果本文に加えてsummary・changed files・diff・test結果を表示できる。これらAgent申告は未検証として表示し、検証済み事実または実ファイル差分として扱わない。

結果を実装Agentへ渡す操作は、元Sessionとは異なるSessionかつ異なるWorkspaceで、Broker登録状態が利用中であり、Task要求／Task記録がまだない場合に限る。Owner／Developerが適用範囲を明示入力した後、D4 Pocketは提案・結果・出所ID/hash・入力範囲を未信頼contextとして通常のBroker Task要求へ束縛し、事前検査だけを行う。適用先要求は`require_unused_task_session=true`を要求し、Brokerは耐久Audit上のTask開始履歴を事前検査・Owner Approval発行・Task開始の各時点で照合する。履歴の本文や一覧はこの照合から表示せず、履歴の破損・読取失敗はfail-closedとする。この条件はPermissionやApprovalを生成しない。元Task・結果projection・proposal provenance・対象Sessionの識別、Workspace、Audit結合、完了状態、結果本文を操作前、入力待機後、Broker応答後に再照合する。不一致や途中変更があればTask準備を停止する。

この機能は結果の直接適用ではない。事前検査からWorkspaceへの書込み、Task開始、Permission／Approval発行を行わない。準備された適用Taskは通常のAgent Task経路へ戻し、対象WorkspaceのPermissionとTaskごとの一回Owner Approvalを別々に新規取得する。提案・結果からPermission、Approval、Credential、Trust、Authorityを継承しない。Flutterはfilesystemやprocessを直接操作しない。結果本文とAgentの変更・test申告は未信頼データのまま保持する。

適用Taskは別Session／別Workspaceの実装作業として、提案元Agentのsummary・changed files・diff・test申告を証拠とせず、変更前の適用先Workspaceを確認し、Owner／Developerの指示と現在のWorkspace Permissionの範囲内で作業する。適用先で関連test／検査を新たに実行し、正確なcommand・exit status、実際に観測した変更file／diff、Recovery状態を報告する。実行不能は`NOT_RUN`、失敗は`FAIL`として残し、Broker Workspace Inspectorで照合するまではAgent申告と明示する。この指示やその実行結果は検証・Approval・Authorityを生成せず、適用元Workspaceへの直接書込みも許可しない。
