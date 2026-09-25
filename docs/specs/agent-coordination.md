# Agent比較・Handoffの境界付き契約

## 目的

D4 Pocketは複数Agentの結果を比較し、必要に応じて別Agentへ作業を引き継ぐ。ただし、Agentの変更は権限の変更ではない。この文書は、実Agentの起動経路が未接続でも先に固定できる、安全な投影境界を定義する。

## Agent Adapterの宣言検証

Brokerの`Agent一覧`はAdapterが返すmetadataを未信頼の宣言として扱い、`agent_adapter.schema.json`と同じ必須field・型・列挙値・上限を検査してから返す。rootとnested objectの未知field、Authorityを示すfield／値、既知のcredential形式を含む文字列、`secret_value_present=true`、不正または重複したAgent／Adapter IDは、一覧全体を`応答不正`として拒否し、拒否をBroker Auditへ記録する。不正metadata本文はerror／Auditへ複写せず、検査に合格した公開fieldだけを一覧snapshotへ返す。credential形式の検出は既知markerに限り、任意形式の秘密値が含まれないことまでは保証しない。

この検査は宣言の形を保証するだけで、AgentのTrust、Permission、Approval、実行可能性を生成しない。`unsupported`、`unavailable`、`unknown`はそのまま維持する。

## 比較

`agent_comparison.schema.json`の比較投影は、2件以上8件以下のAgentセッションを対象とする。各セッションは、異なるWorkspaceでなければならない。同一Workspace、同一セッションの重複、またはWorkspaceを識別できない入力は比較を拒否する。

比較できるのは、次のboundedな公開概要だけである。

- 完了状態と失敗状態
- 出力概要、差分概要、試験状態
- 変更ファイル数、道具呼出し数
- 所要時間、Token、Cost、Resourceの観測値または`unknown`
- 承認状態の概要と監査ID

取得不能な値を`0`や成功へ補完しない。比較投影は`INTERNAL_STATE`の証拠であり、実Agentの実行、外部Agentの再現性、release readinessを証明しない。

## Agent引き継ぎ

`agent_handoff.schema.json`のHandoff投影が渡すのは、Task概要、差分概要、試験状態、公開実行概要だけである。次は固定して引き継がない。

- 権限
- Permission
- Approval
- Credential実値
- 非公開コンテキスト

target Agentの条件でAuthorityを再評価することを必須とする。Handoff投影は引き継ぎ操作を実行せず、対象Agentの起動、Workspace作成、Approval、監査確定、復旧操作はBroker統治経路が別途成立するまで`unsupported`または`release_blocker`として扱う。

## UI責任

Desktop Agent Centerは、現在のsnapshotから比較可否と公開Handoff概要を読み取り表示するだけである。FlutterはAgentを起動せず、Workspace、Permission、Approval、Credential、process、networkを直接操作しない。同一Workspaceを検出した場合はfail-closedで比較を止める。

## 未成立範囲

- `release_blocker`: 実Agentを複数起動して同一Taskを独立Workspaceで実行するBroker経路
- `release_blocker`: 実結果のdiff／test／duration／tool／token／cost／resource／approval／audit比較
- `release_blocker`: target Agentへの実Handoff、再評価、取消、失敗隔離、Recovery
- `known_limitation`: 現行UI投影のTask概要は既存snapshotの表示用文字列からboundedに生成される。秘密値の非混入はBroker由来の公開投影が正本になるまで、これだけでは実証しない
