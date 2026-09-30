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

Desktop Agent Centerは、product modeではBrokerが返したAgent対話SessionのID・状態・監査参照・登録Workspace IDだけを実値として表示する。`local`／`mock` snapshotのAgent sessionは実行結果として表示しない。Task、diff、Tool、command情報は現在のBroker一覧contractにないため表示しない。従来の実行情報surfaceは項目名と「Brokerから未取得」等の明示状態に限り残し、fixture値、架空の0件、実値らしい合成内容を表示しない。保留中のApprovalが未取得であることをApprovalがない証拠として扱わない。比較の識別子重複検査は比較機能そのものではなく、実Agent実行・隔離結果も示さない。Handoffは常に未接続表示とし、fixtureやregex redactionで公開概要を合成しない。FlutterはAgentを起動せず、Workspace、Permission、Approval、Credential、process、networkを直接操作しない。同一Workspaceを検出した場合はfail-closedで比較候補を拒否する。

Rust Brokerの`対話セッション一覧`は、Agent metadataがSchema適合したAgent Adapterに結び付き、同一Runtimeの登録WorkspaceへSession開始時に明示結合した現在対話sessionのID、実行系ID、状態、作成監査ID、Workspace ID、結合監査IDだけを`INTERNAL_STATE`としてDesktopへ渡す。Agent metadata適合は分類に限り、Trust・Permission・Approval・Authorityを与えず、実Agentの稼働証明でもない。Workspace結合は登録IDの対応だけであり、Agent専用実行Session、実行directory、書込み分離を証明しない。実Agent Sessionと実結果がまだないため比較・Handoffは未接続のまま、Task、差分、Tool、コマンド内容を表示しない。ローカル／mock snapshotのサンプル情報をBroker観測へ読み替えない。

Workspace registryは、異なる実行系ID間の同一rootと通常pathで観測した親子rootの登録を拒否する。起動登録ではnofollowで開いた各directoryの(device ID, file ID)列を二度のpath解決間で照合し、別runtimeとの範囲交差を確認する。Rust試験は親→子・子→親の拒否、独立rootの許可、root identityだけの不完全列と識別列を持たないhandle-only登録の拒否、およびBroker起動登録での拒否Auditを確認する。これらは一時directoryと試験Brokerを使う`FIXTURE`証拠であり、bind mount等の別path aliasを網羅せず、実Agent間の書込み隔離も証明しない。

2026-10-01のWindows直接probeは、Rustが生成するTask permission profileを使う実Codex CLI／MxC sandbox processを合成Workspace Aから起動し、隣接する合成Workspace Bのmarker読取と新規file作成がWindows Access Deniedで拒否されることを確認した。証拠は指定した一回のsandbox child・二つの合成pathに対する`LIVE_RUNTIME`であり、markerとWorkspaceは`FIXTURE`である。これは二つのAgentの同時実行、production Broker／IPC、Owner Approval、Audit／Recovery、実Task、installed productを通らず、path alias全般も検査しない。したがってcross-agent contaminationの実行試験全体は未成立であり、独立Workspace比較・複数AgentのBroker実行と隔離は引き続き`release_blocker`である。

owner起動設定でCodex runtimeと同じ`runtime_id`を持つWorkspaceについては、Broker起動前にAdapter固定作業pathとWorkspace rootをnofollowで開き、device ID／file IDが一致しない設定を拒否する。設定拒否は`CONFIG`として監査する。対話開始ではAgent AdapterにWorkspace IDを明示させ、Workspace registryが現在保持する登録とruntime IDの一致をRust Brokerで確認した後、Sessionへ結合し、別AuditEventへ登録hashを含む関係を記録する。Codex Adapterは登録時に固定rootの(device ID, file ID)を保持し、各taskのprocess spawn直前にnofollowで開き直して一致しないrootを拒否する。Windowsではvolume rootからworkspaceまでのdirectory handleをprocess spawn完了まで保持し、通常のNTFS path上でのdirectory rename／deleteによる差し替えを防ぐ。これはpath alias全般、管理者が作るmount等、Unixのcheck-to-spawn raceを網羅しない。実Agentの別Workspace書込隔離、Taskの成功、比較・Handoffの許可にも使わない。Mobile Device LinkのWorkspace選択は、認証済み一覧からWorkspace ID／Runtime IDのみを返す許可済み経路として実装する。Rust Brokerの通常Workspace一覧handlerでRuntime対応を検査し、Device Link応答ではpath・登録hash・Approval metadataを除去する。Mobile選択値はSession要求に過ぎずAuthorityを与えず、Task実行には作業Task専用の別Owner Approvalが必要である。実機TLS統合とinstalled productでの証拠は未成立。

Desktop側の比較可否は2〜8件、比較用Session ID／Agent runtime IDの形式、識別不能なAgent runtime ID、空でないWorkspace参照、Session ID／Agent runtime ID／Workspace参照の重複を検査する。ここで使うsnapshotのAgent runtime IDやWorkspace文字列は宣言値に過ぎず、実Agent identity、実Workspace隔離、path alias／junction不在の証明ではない。UIは「Agent runtime IDとWorkspace参照の重複なし」とだけ表示し、実行時隔離を確認済みと表示しない。実Agent比較はBrokerの独立Workspace bindingと実行経路が成立するまで未接続である。

## 未成立範囲

- `release_blocker`: 実Agentを複数起動して同一Taskを独立Workspaceで実行するBroker経路
- `release_blocker`: Windows以外のtask spawnに残るpath identity再確認とprocess起動間のrace、および通常path identity列で観測できないmount等の別名範囲。Windowsの通常NTFS pathに対するdirectory handle guardは差し替え範囲を限定するだけで、別名範囲や実Agent隔離の証拠ではない
- `release_blocker`: 実結果のdiff／test／duration／tool／token／cost／resource／approval／audit比較
- `release_blocker`: target Agentへの実Handoff、再評価、取消、失敗隔離、Recovery
- `known_limitation`: Desktopの比較可否はsnapshot文字列の検査に限り、実Workspace隔離や別pathから同一領域へ到達する別名関係を証明しない
- `known_limitation`: Local／mock snapshotにはfixture由来の表示文字列が含まれ得るため、実Agent作業やBroker観測の証拠として扱わない。Brokerの対話セッション一覧はTask内容を受け取らない
- `release_blocker`: Mobile Device Linkからinstalled Desktop Brokerまでの実Agent対話経路のend-to-end実証。Source経路のWorkspace選択はID／Runtime IDだけに限定し、Broker側Runtime照合とowner Approval境界を維持するが、実機TLS接続・切断・失効・再接続・Task／Approvalの実動作は未検証
