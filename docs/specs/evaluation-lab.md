# 評価ラボ

## 1. 目的と責任境界

評価ラボは、同一の不変な評価Dataset revisionに対する複数実行系の運用観測を、再現可能な安全projectionとして記録し比較するC5のcontractおよびBroker実装である。Schema、fixture、conformanceは評価recordの構造を検査する。BrokerのC5経路は、ownerが登録したDataset revisionを読み、各Case/Runtimeごとに既存の対話requestとsessionを作り、既存のowner対話承認後に取得した公開応答projectionだけを決定論評価器へ渡す。実行系への直通Adapter dispatch、endpoint、commandを追加せず、C4のライフサイクル承認を再利用しない。

評価結果は運用観測であり、Authority、Capability、Permission、Approval、Auditの有効性、Recovery、security判断、release可否を生成、変更、承認、代替しない。評価recordの存在や成立判定だけを、runtime health、security integrity、release readinessの証拠へ昇格してはならない。各実行は既存の統治された対話経路を使用し、評価ラボ専用のAdapter dispatch、隠れたIPC、Approval再利用を追加しない。

本C5 contractはnormal IPCの既存の権限境界を変更しない。normal IPC、公開manifest、Flutter表示へ、raw input、期待本文、期待断片、regex本文、JSON Schema本文、応答本文、credential、endpoint、command、環境変数を返してはならない。`full`表示を必要とするowner登録payloadは、`評価Dataset登録`のowner control経路だけで扱い、normal IPCへ複写しない。

LLM-as-judgeは未実装である。将来参考表示を追加しても、`評価方式=決定論`のcontract結果、Authority、Permission、Approval、Audit、release/security決定を置き換えられない。

## 2. Dataset revisionと公開projection

評価Datasetは`評価DatasetID`、単調増加する`revision`、`定義hash`で識別するimmutable revisionである。Case、Evaluator、Experiment、Result、Comparisonは対象DatasetIDとDataset定義hashを明示する。Case内容、Evaluator設定、順序、公開範囲を変えるときは既存revisionを上書きせず、新revisionを登録する。過去のExperimentやResultが参照するDataset定義hashは変更しない。

owner登録時、Brokerは検証済みのaccepted Dataset manifestを永続監査の順序で照合する。同一の`評価DatasetID`では、後続revisionは既存の確定revisionより厳密に大きくなければならない。同じ`(評価DatasetID, revision)`の再登録、または既存revision以下への巻戻しはrejectする。revision番号の欠番は、この単調増加規則だけでは拒否しない。単一recordしか見ないJSON Schemaはこの履歴関係を表せないため、`tooling/evaluation_contract_check.py`の`Dataset改版列検査`とBrokerがfail-closedに検査する。

normal IPCで返す公開Dataset manifestには、DatasetID、revision、定義hash、非公開保管ID、暗号文hash、表示名、Case数、CaseIDと定義hashだけを置く。非公開保管IDはDatasetID/revisionに対応するprivate payloadの保管位置を照合するための識別子であり、raw inputを示さない。暗号文hashは再起動後にprotected payloadを監査manifestと照合するためだけに使う。`evaluation_case.schema.json`と`evaluation_evaluator.schema.json`は、owner登録payloadから導く静的なhash_only contractであり、normal IPCのruntime API responseではない。したがってruntime manifestは入力hash、Evaluator ID、種類、設定hashを出さない。これらの静的contractも入力本文や設定本文を持たず、hashや件数から原文を復元できるとは主張しない。

ownerがDataset revisionを登録するためのprivate payloadは`evaluation_dataset_registration.schema.json`で規定する。このpayloadだけが入力本文とEvaluator設定本文を含められる。Windows実装では`ProtectedStore::Purpose::Evaluation`の専用purposeへ保管し、plaintext fallback、History用途への混入、公開manifestへの複写を禁止する。owner CLIは最終要素を再解析点を追従せず一度だけ開き、同じ開いたfileの属性が通常fileであることを確認してから読む。事前の属性確認や検査後のpath再解決はしない。非Windows環境または保護保管未登録環境での登録はfail-closedとする。

## 3. 評価器（`Evaluator`）

Evaluatorの種類は次に固定する。

| 種類 | private設定field | private設定の意味 |
| --- | --- | --- |
| `exact` | `期待本文` | 期待本文との完全一致 |
| `contains` | `期待断片` | 期待断片を含むこと |
| `regex` | `パターン` | owner登録済みpatternとの照合 |
| `json_schema` | `期待Schema` | owner登録済みJSON Schemaへの適合 |
| `reference_count` | `最小数`、`最大数` | 参照数の上下限 |
| `route` | `期待経路` | 観測されたroute |
| `status` | `期待状態` | 対話結果状態 |
| `capability` | `必要能力一覧` | 観測された能力表示 |
| `latency_threshold` | `最大Millis` | Broker monotonic latencyの上限 |

`regex`と`json_schema`の本文は公開projectionに入れない。`json_schema`評価器は外部refをfetchせず、登録済みの有界なlocal構造だけを評価する。Capabilityの観測や宣言は権限付与ではない。`latency_threshold`は0から86,400,000までの整数Millisであり、Brokerが計測した単調時計の値だけを使用する。UI時計、C3資源観測履歴、wall-clock推測を使わない。必要な表示範囲や観測根拠が得られないEvaluatorは、推測で成立にせず`評価不能`にする。

## 4. 評価実験・結果・比較（`Experiment` / `Result` / `Comparison`）

ExperimentはDatasetID、Dataset定義hash、対象Runtime一覧、状態、計画Case数、結果数、監査相関だけを持つ安全projectionである。状態は`準備済み`、`承認待ち`、`実行中`、`完了`、`中断`、`評価不能`に限る。`完了`は`結果数=計画Case数×対象Runtime数`、開始時刻、終了時刻をすべて満たす場合だけを示す。開始・終了時刻が両方ある場合、終了時刻は開始時刻より前になれない。再起動復元で全Resultと通常対話終了を確認できた場合は、復元時の現在時計ではなく、監査済みResult実行時刻の最小値と最大値を開始・終了時刻にする。全Resultが監査済みでも既存対話の終了回収中なら`実行中`のままとする。`中断`はC4 terminal隔離または再起動復元により部分Resultと終了時刻なしで表され得る。必須の`計画監査ID`は、不変な計画監査recordへの安全な相関である。この計画recordはCaseID、RuntimeID、対話request ID、対話session ID、request hash、`Session開始要求監査ID`、`Session開始監査ID`、`送信要求監査ID`、`送信内容hash`、作成監査ID、`隔離要求監査ID`、隔離監査IDを保持する。`送信内容hash`は既存対話送信へ渡したcanonical payloadのhashであり、private入力の復元や表示を許可しない。計画recordはprivate入力、Evaluator設定、raw応答を持たない。復元時はSession開始要求→Session開始→送信要求→対話作成→隔離要求→隔離→計画→Experiment→対話開始/終端→結果証跡→Resultの監査順序と、各request/Session/送信内容hashの一致を検証し、隔離されていないApprovalを後から結合してはならない。各Case/Runtime実行は独立した対話request、session、audit相関を持ち、別Case、別Runtime、別ExperimentのsessionまたはApprovalを共有しない。

内部のResultはCaseID、RuntimeID、対話request/session、作成・開始・終了・評価audit相関、`実行系隔離予約監査ID`、成立/不成立/評価不能/中断、Evaluator別判定と理由code、実行時刻、latency、応答hashを持つ。作成監査IDは必須である。`結果状態=成功`では開始監査ID、終了監査ID、応答hash、厳密な対話結果証跡が必須である。`結果状態=中止`では`判定=中断`、`結果状態=評価不能`では`判定=評価不能`に固定する。`実行系隔離予約監査ID`は通常の中止と中止以外ではnullであり、C4 terminal隔離に起因する中止だけが、同じRuntimeかつ同じExperimentの先行する実行系隔離予約監査recordを一意に指せる。予約record、Runtime、Experiment、順序のいずれかが一致しない中止Resultは復元しない。owner承認の表示範囲から応答hashを取得できない成功応答は、部分的なEvaluatorだけを成立へ昇格させず`評価不能`として記録し、既存対話の終了へ進む。owner承認前の中断、期限切れ、未送信は開始監査IDとLatencyMillisをnullで表し、開始済みまたは計測済みと偽らない。終了監査IDは既存対話経路が終了eventを記録した場合だけ設定し、未送信であっても常にnullと断定しない。本文、参照本文、route本文、Capability本文、秘密情報は持たない。`成功`であっても各Evaluatorの判定が未取得なら結果全体を成立へ昇格しない。

通常IPCとFlutterへは内部Resultを返さない。`evaluation_public_result.schema.json`の公開Resultは`評価ExperimentID`、`評価DatasetID`、`評価CaseID`、`実行系ID`、`判定`、`評価器判定一覧`、`LatencyMillis`、`総合hash`、`評価監査ID`だけを持つ。対話request/session ID、開始・終了監査ID、応答hash、結果状態、C4隔離予約監査ID、raw応答、private入力を通常経路へ含めない。公開projectionの`評価監査ID`は内部監査recordへの安全な参照であり、監査内容の閲覧権限を与えない。

Comparisonは同一DatasetIDかつ同一Dataset定義hashを参照する一つのExperimentについて、複数Runtimeの集計をrecordする。`実験一覧`の全entryは、比較要求で指定した一つの`評価ExperimentID`と一致しなければならず、異なるExperimentIDを混在させてはならない。Runtimeは重複させない。各Runtime entryの成立数、不成立数、評価不能数、中断数の合計は計画Case数と一致し、rootの各分類数はRuntime entryの同分類数の合計と一致しなければならない。比較は全Case×Runtimeの終端Resultと、その各sessionの通常`対話終了`監査を復元できた場合だけ許可する。C4 terminal隔離は外部実行停止を保証せず通常の終了監査を代替しないため、隔離後の中断Resultは状態観測に留め、比較可能性の証拠へ昇格させない。標準JSON Schemaはarray entry間の値一致を表せないため、この関係は`tooling/evaluation_contract_check.py`とBrokerがfail-closedに検査する。成立数、不成立数、評価不能数、中断数、平均latencyをrecordする。route/reference/capabilityの比較値は`same`、`different`、`unknown`だけであり、生の内容や権限判断を持たない。既存の`runtime_dialogue_comparison.schema.json`は一対の対話requestの非混線検査を担うため、この集計contractで置き換えない。

## 5. UI、C6、および延期範囲

Flutterは公開projectionの表示、対象Runtimeの選択、実験の開始・進行・比較表示だけを担い、ownerのprivate Dataset登録、Evaluator実行、Authority判定、Permission、Approval、Audit、release/security判断を所有しない。C5は通常IPCの`評価Dataset一覧`、`評価実験開始`、`評価実験状態`、`評価比較`と、owner controlの`評価Dataset登録`を接続する。Rust/Brokerは各Case/Runtimeを既存の対話承認経路へ接続するが、Flutterや評価器へ権限を移さない。

C6の対話からRegression Caseを自動または手動登録する機構は、このC5 contractのscope外である。対話履歴、cache、previous state、LLM出力、Adapter metadataからCaseを生成・昇格することはC5では許可しない。C6は`docs/specs/regression-case.md`のowner登録、private storage、Audit、結果証跡照合を持つ独立contractとして扱い、C5 Dataset revisionへ自動追加しない。

## 6. Contractと検査

この仕様の構造contractは次である。

- `evaluation_dataset.schema.json`
- `evaluation_case.schema.json`
- `evaluation_evaluator.schema.json`
- `evaluation_dataset_registration.schema.json`
- `evaluation_experiment.schema.json`
- `evaluation_result.schema.json`
- `evaluation_public_result.schema.json`
- `evaluation_comparison.schema.json`

`tooling/evaluation_contract_check.py`は、公開projectionのraw field禁止、通常公開Resultへの対話ID・開始終了監査ID・応答hash・C4隔離予約監査ID混入禁止、owner登録の用途・方式固定、Dataset revision列の単調増加と同一`(評価DatasetID, revision)`の再登録拒否、一つの評価ExperimentIDだけを含むRuntime集計、Runtime重複、Dataset定義hash不一致、Resultの成功/中止/評価不能相関、非有限latency、LLM authority用途を検査するdevelopment-only validatorである。production authorityを生成せず、fixtureや自己生成objectの成功をruntime/release証拠にしない。
