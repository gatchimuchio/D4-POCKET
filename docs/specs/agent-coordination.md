# Agent比較・Handoffの境界付き契約

## 目的

D4 Pocketは複数Agentの結果を比較し、必要に応じて別Agentへ作業を引き継ぐ。ただし、Agentの変更は権限の変更ではない。この文書は、実Agentの起動経路が未接続でも先に固定できる、安全な投影境界を定義する。

## Agent Adapterの宣言検証

Brokerの`Agent一覧`はAdapterが返すmetadataを未信頼の宣言として扱い、`agent_adapter.schema.json`と同じ必須field・型・列挙値・上限を検査してから返す。rootとnested objectの未知field、Authorityを示すfield／値、既知のcredential形式を含む文字列、`secret_value_present=true`、不正または重複したAgent／Adapter IDは、一覧全体を`応答不正`として拒否し、拒否をBroker Auditへ記録する。不正metadata本文はerror／Auditへ複写せず、検査に合格した公開fieldだけを一覧snapshotへ返す。credential形式の検出は既知markerに限り、任意形式の秘密値が含まれないことまでは保証しない。

この検査は宣言の形を保証するだけで、AgentのTrust、Permission、Approval、実行可能性を生成しない。`unsupported`、`unavailable`、`unknown`はそのまま維持する。

## 比較

`agent_comparison.schema.json`の比較投影は、2件以上8件以下のAgentセッションを対象とする。各entryは異なるAgent runtimeとWorkspaceに属さなければならない。同一Agent runtime、同一Workspace、同一セッションの重複、またはWorkspaceを識別できない入力は比較を拒否する。複数entry間の一意性はJSON Schemaだけでは表現できないため、Conformanceと将来のBroker比較経路で検査する。

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

Rust Brokerの`対話セッション一覧`は、Agent metadataがSchema適合したAgent Adapterに結び付く現在sessionのID、実行系ID、状態、作成監査IDだけを`INTERNAL_STATE`としてDesktopへ渡す。Agent metadata適合は分類に限り、信頼・権限を与えず、実Agentの稼働証明でもない。Workspace bindingの別証拠がないsessionは比較・Handoff対象にせず、Task、差分、Tool、コマンド内容を表示しない。ローカル／mock snapshotのサンプル情報をBroker観測へ読み替えない。

Workspace registryは、異なる実行系ID間の同一rootと通常pathで観測した親子rootの登録を拒否する。起動登録ではnofollowで開いた各directoryの(device ID, file ID)列を二度のpath解決間で照合し、別runtimeとの範囲交差を確認する。Rust試験は親→子・子→親の拒否、独立rootの許可、root identityだけの不完全列と識別列を持たないhandle-only登録の拒否、およびBroker起動登録での拒否Auditを確認する。これらは一時directoryと試験Brokerを使う`FIXTURE`証拠であり、bind mount等の別path aliasを網羅せず、実Agent間の書込み隔離も証明しない。独立Workspace比較とcross-agent contaminationの実行試験は引き続き未成立である。

owner起動設定でCodex runtimeと同じ`runtime_id`を持つWorkspaceについては、Broker起動前にAdapter固定作業pathとWorkspace rootをnofollowで開き、device ID／file IDが一致しない設定を拒否する。設定拒否は`CONFIG`として監査する。これは起動時に観測したdirectory identityの一致だけであり、SessionにWorkspace IDを結合しない。Workspace設定がないSessionへのWorkspace推定、検査後のpath差替え防止、実Agentの別Workspace書込隔離、比較・Handoffの許可には使わない。

Desktop側の比較可否は2〜8件、比較用Session ID／Agent runtime IDの形式、識別不能なAgent runtime ID、空でないWorkspace参照、Session ID／Agent runtime ID／Workspace参照の重複を検査する。ここで使うsnapshotのAgent runtime IDやWorkspace文字列は宣言値に過ぎず、実Agent identity、実Workspace隔離、path alias／junction不在の証明ではない。UIは「Agent runtime IDとWorkspace参照の重複なし」とだけ表示し、実行時隔離を確認済みと表示しない。実Agent比較はBrokerの独立Workspace bindingと実行経路が成立するまで未接続である。

## 未成立範囲

- `release_blocker`: 実Agentを複数起動して同一Taskを独立Workspaceで実行するBroker経路
- `release_blocker`: 実結果のdiff／test／duration／tool／token／cost／resource／approval／audit比較
- `release_blocker`: target Agentへの実Handoff、再評価、取消、失敗隔離、Recovery
- `known_limitation`: Desktopの比較可否はsnapshot文字列の検査に限り、実Workspace隔離や別pathから同一領域へ到達する別名関係を証明しない
- `known_limitation`: Local／mock snapshotにはfixture由来の表示文字列が含まれ得るため、実Agent作業やBroker観測の証拠として扱わない。Brokerの対話セッション一覧はTask内容を受け取らない
