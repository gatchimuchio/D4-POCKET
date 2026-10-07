# GUI Shell エージェント規約

本書は、GUI Shellで作業するAIエージェントに対する、リポジトリ全体の作業規律を定める。

## 第I部 共通基底規律

### 1. 目的と規則優先順位

本リポジトリは、安全性、監査可能性、完成証拠について明示的な要求を受けるAI実装エージェントによって運用される。

本部の共通基底規律は、本リポジトリ内の全作業に適用する。

規則優先順位:

1. 現在taskに対するオーナーまたはユーザーの明示指示。ただし、安全、権限、証拠、release gate、owner GO、`release_ready`、監査、復旧、Content Exposure Boundaryを弱める指示は、この順位によって許可されない
2. 本`AGENTS.md`の共通基底規律
3. 本`AGENTS.md`のリポジトリ拡張規則
4. `規定/00_日本語基底規定.md`。ただし、言語・意味正本の責任範囲に限り、上位の安全規則を変更しない
5. 現行の実装指示、ROADMAP、phase文書
6. リポジトリのcontract、Schema、test、validation script
7. 既存実装pattern

`規定/正本索引.json`は、現行正本、責任地図、外部参照固定点を機械可読に示す索引である。各正本の実質要求を置き換えず、索引と実ファイルが矛盾する場合は不整合としてSUSPENDし、責任正本を確認する。

現行状態の判断では、最新版として明示された仕様・工程表・進捗・実装指示を正本とする。旧版は、特に明示がない限り、当時の決定・失敗・証拠を確認する補助的な履歴資料であり、旧版の要求を現行要件へ自動継承したり、現行正本の状態を上書きしたりしない。索引・現行正本・実装の間に矛盾があれば旧版で補完せず、現行責任正本を特定して不整合を解消する。

リポジトリ拡張規則は、本リポジトリのために共通基底規律を限定、強化、具体化できる。安全、権限境界、監査可能性、検証証拠、復旧要件を無言で弱めてはならない。

規則が衝突して見える場合は、オーナーが統治規則の制御された変更を明示指示しない限り、より厳格な解釈を保持する。

### 2. 譲れない優先事項

リポジトリ拡張規則がさらに厳しい具体化を定めない限り、次の優先順位を適用する。

1. 安全性
2. 堅牢性
3. 操作者の明瞭性・監査可能性
4. contractとruntimeの完全性
5. 製品機能
6. 利便性

機能完成は、安全、権限境界、監査可能性、復旧、検証証拠より上位にならない。

利便性を、隠れた権限、虚偽の完成主張、未検証のruntime保証、説明のない回避策、失敗処理の弱体化の理由にしてはならない。

### 3. 境界付き実装規律

複雑な仕様を、広範な生成の許可として扱わない。

各taskで次を行う。

- 編集前に既存file、contract、test、validation command、関連文書を確認する。
- taskを満たす、保守可能で最小の変更を実装する。
- リポジトリ固有の境界を保持する。
- 機会的refactorを行わない。
- 推測的機能を追加しない。
- 明示要求なしに、Permission、権限、runtime到達範囲、依存、toolchain、環境前提を広げない。
- 完了報告前に、taskが導入した残骸、古いTODO、放棄した部分経路、一時的な実装残余を除去する。

大量の生成構造は、完全性の証拠ではない。

#### 3.1 ローカル品質基準と手動補助実行

開発、試験、差分監査、品質判定の正本はローカル作業ツリーに置く。GitHub は完成した局所成果を順次還元する記録・共有面とする。自動 CI、push / pull_request / merge_group trigger、CI 必須 status check、CI green による完成判定は禁止する。

GitHub Actions は、ローカルに存在しない OS の build、artifact 生成、重い検査・testの補助に限り、`workflow_dispatch` のみで使用できる。ownerは検査・test目的の手動Actions使用を許可しており、ローカル検証の不足を補うために必要な場合は、一時的な検証branchを作成・pushしてよい。branchは検査対象commitの固定に限り、PRや自動triggerを作らない。失敗時は同branch上で修正して再検査し、対象blockの検証が閉じたら、成功した正確なcommitを`main`へfast-forwardし、remote HEADを照合してから一時branchをlocal／remote双方で削除する。検査が失敗中のcommitを`main`へ統合しない。Actions結果は対象commit上で実行した検査だけの証拠であり、workflowは品質基準面ではなく、実機起動、製品完成、release readinessの証拠を代替しない。必要性、対象commit、trigger、結果、artifact、証明する範囲としない範囲を記録する。workflowを追加する場合はactive ROADMAPまたはphase instructionに目的を置き、local conformanceで手動起動限定を検査する。

品質基準面は owner / Codex が実行する local validation、smoke、release verification、Windows 実機 evidence とする。Codex は要求から正常・境界・失敗・権限否定・回帰試験を導出して実行し、未試験を成功へ昇格しない。完成した作業単位ごとに試験、監査、commit、push、remote HEAD 確認を閉じてから次単位へ進む。検査の削除・弱体化には owner 承認を必須とし、Authority、Approval、Audit、Recovery、Content Exposure Boundary、owner GO、release gate を弱めない。

### 4. 完成証拠規則

完成の主張は証拠ではない。

作業blockを完了と報告する前に、次を特定する。

- 実装した挙動
- その挙動を実行するproduction、runtime、contract、validationの経路
- 実際に実行した正確なvalidation command
- 正確な結果
- 実行しなかったvalidation
- 残存するstub、mock、placeholder、TODO、未接続contract、環境制約、既知の制約

文書、Schemaの存在、mock成功、fixture成功、単体test成功だけを、production経路または製品挙動が完成した証拠として報告してはならない。

security-critical、authority-critical、audit-critical、recovery-critical、release-criticalな変更では、実際の統治経路が実行されたことを何の証拠が示すかを記載する。

リポジトリ状態を変更する実装taskでは、オーナーがlocal-only、audit-only、review-only、no-commit、no-pushに明示限定しない限り、完成にはリポジトリ状態の閉包も必要である。

リポジトリ状態を変更するtaskは、次を満たすまで完了ではない。

- file編集前に、リポジトリの2世代backup規約で現在push済みのリポジトリ状態を保存する。
- 意図した変更を実装し、taskの残骸を残さない。
- 必須validationを実行し、正確な結果を把握する。
- 意図した変更をすべて正確なmessageでcommitする。
- 指定remote branchへcommitをpushする。
- push先remote HEADが報告commitと一致することを確認する。
- リポジトリbackup規約に従って、復旧可能な2世代backupを確認する。
- push後にworking treeとbranchの整合を確認する。

リポジトリ状態を変更した作業の完了報告には、次を含める。

- 作業branch
- commit hash（コミット識別値）
- push結果
- remote HEAD確認
- backup世代のrefとhash
- rollback point（復旧地点）
- validation結果

commit、push、remote確認、backup確認を完了できない場合、taskを完了と報告しない。失敗した正確なcommand、理由、現在のリポジトリ状態、最も安全な復旧点を報告する。

### 5. 証拠源・幽霊不変条件禁止規則

次だけに基づいて、runtime health、system integrity、security invariant、authority integrity、release readinessを報告してはならない。

- configurationの検証
- Schemaの検証
- 自己生成したstate object
- mock化したruntime state
- fixtureのみの結果
- 静的objectまたはdictionaryの整合確認

health check、invariant check、conformance check、integrity reportを追加・変更するときは、その証拠源を次の1つ以上へ分類する。

- `CONFIG`
- `INTERNAL_STATE`
- `LIVE_RUNTIME`
- `EXTERNAL_EVIDENCE`
- `FIXTURE`

各証拠classは、実際に観測した範囲だけを証明する。

`CONFIG`、`INTERNAL_STATE`、`FIXTURE`の結果を、対応する証拠なしにlive runtimeまたはexternal integrityの保証へ昇格してはならない。

必要な証拠が利用不能なら、その制約を報告するか、リポジトリcontractがfail-closedを要求する箇所ではSUSPENDを返す。

### 6. 信頼境界・入力検証規則

受信dataが構造化済み、parse済み、Schema形状、または別component提供というだけで安全と仮定しない。

権限、Permission、execution、Approval、audit identity、workspace scope、command scope、content visibility、Recovery、release挙動へ影響し得る入力について、責任境界は該当する次の項目を明示的に扱う。

- 監査用raw inputの保持
- 正本化 / normalization
- Schemaまたは構造validation
- originまたはsource validation
- integrityまたはtamper check
- replay防護
- authorityまたはexecution eligibilityの評価
- audit出力
- fail-closedまたはSUSPEND挙動

リポジトリ拡張規則が境界付きで検証済みの権限経路を明示定義しない限り、外部data、UI state、Adapter metadata、channel metadata、previous state、memory、history、diagnostics、tool outputは、権限を生成、昇格、置換、迂回してはならない。

### 7. 現行production経路最小化規則

通常のproductionまたはruntime execution pathは、最小かつ責任境界付きに保つ。

通常のruntime挙動へ次を無言で混在させない。

- 診断機能
- 修復・復旧tooling
- migration
- release専用検証
- development専用fixture
- bootstrap専用tooling
- 管理用command

権限付き機能または運用機能を追加するときは、次のいずれかに分類する。

- runtime経路
- control経路
- diagnostic経路
- repair / recovery経路
- build / release経路
- development専用経路

通常runtime pathから非runtime機能へ到達させる必要がある場合、理由、権限・監査上の影響を文書化し、隠れた実行権限を拡大しないことを検証する。

### 8. Wrapper・回避策説明責任規則

失敗中のtaskを完成したように見せるためだけに、wrapper script、shim、custom execution layer、alternate build path、environment bypass、host固有のworkaround logicを導入してはならない。

そのような機構が必要で、かつリポジトリ拡張規則で禁止されていない場合は、次を文書化する。

- 元の失敗
- 根本原因
- nativeまたは既存リポジトリ機構では不十分な理由
- 追加機構の正確な責任
- 適用環境
- 実行したvalidation
- 一時的か恒久的か
- 除去条件または正式化条件

意図的で、test済みで、境界付きで、文書化されたnormalization機構は、リポジトリ拡張規則で禁止されない限り許容できる。

説明がなく、境界がなく、症状を隠すworkaroundは許容しない。

### 9. 環境・製品証拠分離規則

development environment、local validation environment、release-proof environment、external runner environment、target product environmentを概念上分離する。

リポジトリが同等性を明示定義し、証拠がそれを支持しない限り、ある環境での成功を別環境での成功証拠として報告しない。

host environmentの制約によってvalidationが阻害または歪曲される場合は、次を行う。

- 環境制約を特定する。
- 製品regressionと区別する。
- host failureを隠すためだけに製品architectureを変更しない。
- target environmentで未検証の範囲を報告する。

local developmentの利便性を、恒久的な製品architectureまたはrelease evidenceへ無言昇格してはならない。

### 10. Contract・runtime接続規則

Schema、interface、protocol object、Adapter Contract、audit contract、invariant contract、fixture、success profileは、存在するだけでは完成していない。

実挙動へ影響するcontractを追加・変更するときは、次を特定する。

- それを消費するproduction、runtime、validator、または統治されたexecution path
- それを実行するvalidationまたはconformance path
- reject、block、audit、SUSPENDすべきnegative caseまたはfailure case
- 意図的に未接続または延期する部分

意図した統治経路で実行されないcontractについて、挙動の完成を主張しない。

### 11. 監査結果・報告規則

完了した全作業報告で、次を区別する。

- 観測した実装事実
- 実際に実行したvalidation
- 未検証の主張
- 環境制約付きcheck
- 残存risk
- 意図的な延期範囲
- リポジトリ固有のrelease blocker

リポジトリがrelease gate分類を使用する場合、その分類を保持して適用する。

safety-critical、authority-critical、execution-criticalな要件を検証できない場合、成功を推論しない。SUSPEND、blocker、またはリポジトリ固有の同等状態を報告する。

## 第II部 リポジトリ拡張規則

### 12. リポジトリの同一性と射程

本リポジトリを通常のapp scaffoldとして扱わない。

GUI Shellは、汎用のRuntime Operation Shell制御planeである。

Flutter、Adapter、reference runtime、local cache、memory、installer、native helper、product UIは、下流または境界付きの実装面であり、権限を所有しない。

GUI Shellは、汎用のGUI Shell / Runtime Operation Shellを実装する。

BLUE-TANUKI専用GUIではない。

BLUE-TANUKIは最初のreference runtimeであり、Adapter Boundaryを介して接続しなければならない。

GUI Shellは、LLMが読むapplication responsibility substrateでもある。contract、安全境界、Adapter model、Approval model、Audit model、Recovery model、拡張規則は、LLM開発・統合エージェントが第一級の実装・統合面として読み、使用することを意図する。

LLMはGUI Shell contractの第一級の実装・統合consumerだが、権限源には決してならない。

#### LLM開発・統合エージェント規則

本リポジトリで作業するAIまたはLLM実装エージェントは、次を守る。

- 新機能、module、Adapter、tool、service、runtime integrationの必須接続面としてGUI Shell contractを扱う。
- 各変更がruntime path、control path、diagnostic path、repair / recovery path、build / release path、development-only pathのどれに属するかを特定する。
- 宣言済みcontractとconformance boundaryの外へ、Adapter、tool、module、external connection、privileged behaviorを追加しない。
- LLM output、memory、external metadata、generated configuration、GUI state、Adapter metadata、tool response、local cache、previous state、diagnosticsを通じて権限を与えない。
- 自身のsensitive actionを決して自己承認しない。
- 統合を容易にするために、Approval、Audit、Recovery、Authority Strip、Content Exposure、broker boundaryの挙動を弱めない。
- 拡張の完成を主張する前に、消費contract、必須conformance test、必須failure case、統治されたruntime pathを特定する。
- 提案統合に新contractが必要な場合、shortcutを無言で即興せず、その必要性を報告する。
- human ownerのApproval、Recovery判断、release claim、最終責任を明示状態に保つ。

### 13. 構造制約

- UI frameworkはFlutter
- MobileのFlutterは表示・操作者入力・非秘密のBroker要求だけを担い、Device Linkの招待・資格・秘密保管・TLS/network接続を直接扱わない。招待入力・接続先確認はAndroid/iOS native UI内で完結させ、招待・資格の実値をDart、platform-channel引数／戻り値、snapshot、error、log、traceへ渡さない
- Mobile native platform adapterはOS安全保管と、既存Device Link TLSからDesktop Rust Brokerへのtransportだけを担う。Permission、Approval、Audit、Recovery、Agent trustの判定を行わず、既存Rust Brokerを迂回する別Authority経路、平文fallback、自動再送を作らない。platform channelはこの境界付きMobile transportの接続口に限り、新たな権限経路として扱わない
- Native helperはRust
- Contract: JSON Schema
- 実装言語・安全境界方針: `docs/LANGUAGE_POLICY.md`
- 日本語基底・意味正本: `規定/00_日本語基底規定.md`
- Reference runtime: BLUE-TANUKI。Adapter経由に限る
- Shell Coreはframework非依存を保つ
- Adapter Contractはruntime中立を保つ
- Flutterは交換可能なUI Layerを保つ
- Flutter / DartはUI Product Layerであり、Authority Boundaryになってはならない
- OS lifecycleなどnativeが観測すべき実状態をFlutterが設定・偽装できるchannel methodを設けない。画面の観測はローカル要求を止めるために使えるが、native側が持つ実際のforeground状態を上書きしてはならない。
- Mobileの招待・端末資格・secretをdebug VM extension、Dart integration test、shell argument、環境変数、log、trace、test artifactへ渡してはならない。platform実動作試験がnative境界のまま成立しない場合は安全なtest harnessの設計まで当該live testを保留し、Release blockerを維持する。過去のDart直接経路の成功を現行native経路の証拠へ転用しない。
- Rustは、権限に敏感なhelper、broker、IPC、Audit、signature、runtime command envelope作業のnative safety boundaryとする
- TypeScript / NodeをGUI-Shell core runtimeにしてはならない。external SDK、Adapter sample、protocol client sample、bridge exampleの範囲に限定する
- PythonをGUI-Shell runtime dependencyにしてはならない。dev-only tooling、Schema generation、migration helper、local validation、release evidence validation、一時validation scriptの範囲に限定する
- Authority-sensitiveなFlutter-Rust接続は、独立process IPCを優先する。FFI / direct bridgeは、authority、signature、approval token、external command dispatch、audit finalizationの境界外でのみ許可する
- Windows Desktop Broker要求に限り、Flutterの`MethodChannel('gui_shell/broker')`からWindows Runnerへ要求JSON文字列だけを渡し、Rust起動器が生成した名前付きpipeで同じ起動器のFlutter child PIDを照合してから既存認証Brokerへrelayする経路を許可する。このMethodChannelとRunnerは権限判断・資格保持・endpoint探索を行わず、新しいAuthority経路ではない。Dart direct Socket/file/credential access、Runnerによる認可、PID照合なしのpipe接続、既存Brokerを迂回するfallbackは禁止する
- オーナーが明示要求しない限り、GUI Shellの利便性のためにBLUE-TANUKI実装を変更してはならない
- macOS Desktopでは同じ`gui_shell/broker`要求をnative Runnerから同梱Rust helperの継承匿名pipeへ渡す。helperはApp Sandboxを継承し、既存認証Brokerのnormal経路だけへ中継する。Flutter／RunnerへBroker資格を渡さず、Owner専用操作はBrokerが拒否する。任意helper指定、任意command、資格file探索、sandbox無効化、別Authority経路は禁止する。責任正本は`docs/specs/macos-desktop-channel.md`。
- macOSのAdapter導入・更新・検証・有効化・無効化・隔離・削除、およびAgent CLI実行系／Workspaceの起動中登録に限り、Rust helperが要求とhashを検査し、Rust所有の期限付きOS確認画面で明示承認された同一要求を既存Brokerのprocess内Owner receiverへ渡す。既存record操作は対象IDと現在hashも束縛し、署名検証・状態遷移はBrokerが再評価する。Agent登録は既存CLI probe・APFS root／保護領域検査を使い、sandbox権限・Task能力・Credential・Permission・Approvalを生成しない。Flutter／Runnerから承認boolやOwner資格を受け付けず、この明示集合以外のOwner操作へ広げない。責任正本は`docs/specs/macos-desktop-channel.md`のOwner追加契約。
- macOSの作業領域OS選択はRust所有NSOpenPanelだけを使い、OS user-selected accessをRust内で起動中保持する。既存匿名pipe／process内receiverを使うが、Brokerは選択投影だけを別sourceで処理し、Owner承認・Permission・Approval・登録へ昇格させない。UIからpath・bookmark・scopeを受け取らず、終了時に同じOS URLのaccessを終了する。sandbox外CLI実行へ拡張しない。責任正本は`docs/specs/macos-workspace-selection.md`。

### 14. 境界の意味

#### Shell Core

Shell Coreは次を所有する。

- runtimeのregistry
- permissionのledger
- approvalのqueue
- auditのstore
- recoveryのcatalog
- updateのpolicy
- content exposureの強制
- adapter conformanceの強制

Shell Coreは次を行ってはならない。

- Flutterをimportする
- BLUE-TANUKI固有logicを含む
- Adapter metadataを信頼する
- memory、cache、previous stateだけを権限として使用する
- Permissionを無言で広げる

#### UI層（Layer）

Flutterは次を所有できる。

- rendering
- operatorの入力
- navigation
- localのUI state
- theme
- localization
- accessibility

Flutterは次を所有してはならない。

- authorityの判定
- Permissionの意味
- Approvalの意味
- Auditの意味
- Recoveryの分類
- content visibilityの規則
- runtime trustの規則

GUI表示成功は、runtime contract完成または権限安全性の証拠ではない。

#### Adapter層（Layer）

Adapterは次を行える。

- runtime stateを正規化する
- runtime healthを露出する
- runtime diagnosticsを露出する
- runtime eventをGUI Shell Schemaへ変換する

Adapterは次を行ってはならない。

- metadataを通じてPermissionを付与する
- runtimeが付与していないauthority contextを生成する
- 許可されたvisibilityを超えてraw payloadを表示する
- sealed、hidden、sacred、authority fieldを編集する
- Approval stateを迂回する
- Audit生成を迂回する

Adapterが露出するhealthまたはdiagnosticsは、それが表す証拠範囲を記載しなければならない。

#### Rust helper層

Rust helperは、境界付きのnative diagnosticsとoperationを実行できる。

Rust helperは次を行ってはならない。

- 隠れた権限経路になる
- filesystem、process、network、credential、IPC、updateへのaccessを無言で導入する
- Capability、Permission、Approval、Audit、Recoveryの対応なしにsensitive actionを実行する
- 構造化されていないsensitive dataを返す

### 15. リポジトリ固有の禁止pattern

次を行ってはならない。

- UI widgetへauthority decisionを置く
- Adapter metadataにPermissionを付与させる
- Adapter metadataにauthority contextを生成させる
- memory、local cache、previous stateだけに権限を付与させる
- `content_visibility=full`でないのに全文contentを表示する
- Approval payloadのauthority、sealed、hidden、sacred fieldを編集する
- 隠れたnetwork、filesystem、process、credential、IPC、update accessを導入する
- runtime Permissionを無言で広げる
- BLUE-TANUKI固有logicをShell Coreへ追加する
- core contractをFlutter固有code内へ置く
- validation evidenceなしにrelease readinessを主張する
- first-run成功を製品完成として扱う
- Product UI完成をcontract完成として扱う
- ROADMAP外の推測的機能を作る
- taskに必要でない広範なrefactorを行う

### 16. 必須監査対応

すべてのsensitive actionは、次へ対応づけなければならない。

- Capability
- Permission
- Approval state
- AuditEvent
- failure時のRecoveryAction

sensitive actionには次を含む。

- filesystemへのaccess
- processのexecution / control
- networkへのaccess
- credentialへのaccess
- IPC
- updateのverification
- runtime Adapterのaction
- Approval payloadのedit
- Auditのexport / inspection
- Recoveryのexecution
- installer stateのchange
- deviceのpairing

### 17. 内容露出規則

許可するcontent visibility値:

```text
none
hash_only
summary
redacted
full
```

規則:

- `none`: raw contentを表示しない
- `hash_only`: payload hashだけを表示する
- `summary`: 承認済みsummaryだけを表示する
- `redacted`: redacted projectionだけを表示する
- `full`: full contentを表示できる

全文payload表示を許可するのは`full`だけである。

### 18. Approval編集規則

Approval編集はfield scopeを限定しなければならない。

次の編集を許可しない。

- authorityのfield
- sealedのfield
- hiddenのfield
- sacred domainのfield
- runtimeのidentity
- permissionのidentity
- auditのidentity
- payload hashの直接編集

許可された編集の後は、次を行う。

- payloadを再hashする
- payloadを再validationする
- 必要に応じてApprovalをvalidation requiredとしてmarkする
- AuditEventをemitする

### 19. コミット前の必須検証

最低限、次を実行する。

```bash
python tooling/schema_check/check_schemas.py
python tooling/conformance_tests/run_conformance_skeleton.py
```

`python`が利用不能なら、次を実行する。

```bash
python3 tooling/schema_check/check_schemas.py
python3 tooling/conformance_tests/run_conformance_skeleton.py
```

Rustが導入済みでRust helperを変更した場合は、次を実行する。

```bash
cd native/rust_helper && cargo test
```

WindowsでWindows DNS helper crateを変更した場合は、独立crateのnative API試験も実行する。

```bash
cd native/windows_dns && cargo test -- --test-threads=1
```

Flutterが導入済みでFlutter appを変更した場合は、次を実行する。

```bash
cd apps/desktop_flutter && flutter analyze
cd apps/mobile_flutter && flutter analyze
```

validationを実行できない場合は、理由を報告する。

実際に成功していないvalidationを、成功したと主張してはならない。

### 20. Git運用方針

本リポジトリは、direct-main owner workflowを使用する。

完了した各作業blockはcommitし、pushしなければならない。オーナーがcommitしない、またはpushしないと明示指示しない限り、完了したリポジトリ変更をlocal working treeだけに残さない。

標準workflow:

1. `main`で作業する
2. `main`への直接作業を標準とし、通常のfeature branchまたはpull requestはオーナーが明示要求しない限り作らない。前項のowner許可済み手動Actions検査に限り、一時`codex/`検証branchを例外として使用できる。検査対象commitを記録し、検証が閉じた後は成功した正確なcommitだけを`main`へfast-forwardし、remote HEAD確認後に一時branchをlocal／remote双方で削除する
3. リポジトリ状態を変更するtaskでfile変更を開始する前に、`origin`をfetch / pruneし、`main`がcleanかつ`origin/main`と整合していることを確認する。不整合なら、編集前に解消するかblockerを報告する
4. localの2世代backup pairをrotateし、現在push済みの変更前状態を保存する
   - `codex/backup-main`が存在する場合、`codex/backup-main-prev`を`codex/backup-main`へforce-updateする
   - `codex/backup-main`を現在push済みの`main`へforce-updateする
5. 2つのbackup世代をremote branchではなく、PR-neutralなremote tagとしてpushする
   - `git push -f origin codex/backup-main-prev:refs/tags/codex/backup-main-prev codex/backup-main:refs/tags/codex/backup-main`
6. 境界付き実装と必須validationを行う
7. 完了した作業blockを`main`へ直接commitする。手動Actions検証branchを使用した場合は、検証が成功した正確なcommitだけを`main`へfast-forwardする
8. commitまたはfast-forward直後に`main`をpushする
9. `git ls-remote origin refs/heads/main`がlocal `HEAD`と一致することを確認する
10. remote backup tagの存在を確認してhashを記録する
    - `refs/tags/codex/backup-main`
    - `refs/tags/codex/backup-main-prev`
11. remoteに`codex/backup-main`または`codex/backup-main-prev` branchが存在する場合、`main`がcleanかつ整合した後にremote backup branchを削除する。GitHub上のbackup branchはpull request候補を生成するため、保持してはならない
12. `git status --short --branch`がcleanかつ`origin/main`と整合することを確認する
13. backup、commit、push、remote HEAD確認、remote backup確認のいずれかが失敗した場合、失敗した正確なcommandと理由を報告する

バックアップブランチ:

```text
codex/backup-main
codex/backup-main-prev
```

リモートバックアップref:

```text
refs/tags/codex/backup-main
refs/tags/codex/backup-main-prev
```

Backup branchはlocal recovery refに限る。GitHubがpull request候補として表示しないよう、remote backup世代はtagとしてpushする。backup refからpull requestをopen、request、mergeしてはならない。

オーナーがその緊急handoffを明示要求した場合に限り、backup branchをremote branchとしてpushする。この例外でbackup branchをGitHubへpushした場合、GitHubがpull request候補として表示し得ることを報告し、オーナーが不要とした時点でcleanupする。

追加のbackup世代を作らない。

次をstageしてはならない。

- secret
- localのruntime state
- Flutterのbuild output
- Rustのtarget output
- installerのartifact
- localのcache
- 明示要求されていないgenerated log

### 21. 完了報告とリリース関門分類

完了したすべての変更報告に、次を含める。

1. 概要
2. 変更file
3. risk分類
4. validation結果
5. release gate分類
6. 分類済みの残存risk
7. 作業branch
8. commit hash、または`not committed`
9. push結果、または`not pushed`
10. remote HEAD確認
11. backup世代refとhash
12. rollback point（復旧地点）

validation結果は、どのcommandが成功、失敗、未実行かを明記しなければならない。

本リポジトリで`release`は、完成した製品releaseを意味する。

final report、release report、validation report、remaining risk sectionでは、未完了項目をすべて次のいずれかに分類する。

- `release_blocker`
- `post_v1_scope`
- `known_limitation`

分類のない`remaining risks`、`still needed`、`not run`、`not implemented`、`not verified`、`TODO`、`skeleton only`、`future work`項目を許可しない。

`release_blocker`が1つでもあればreleaseを主張できない。

`post_v1_scope`項目は、v1.0 scope外である理由を明記しなければならない。

`known_limitation`は、release前に`README.md`、`CLAIM.md`、`RELEASE_CHECKLIST.md`のいずれかへ文書化しなければならない。

残存riskの形式:

```text
- item:
  classification: release_blocker | post_v1_scope | known_limitation
  reason:
  required_action:
  blocks_release: yes | no
```

### 22. 日本語基底・文書言語

GUI-Shellの基底言語、規定言語、内部意味正本、設計言語、監査言語、運用報告言語は日本語とする。詳細な対象、局所例外、意味監査、版差境界は`規定/00_日本語基底規定.md`を正本とし、現行正本と外部参照固定点は`規定/正本索引.json`で確認する。

```text
日本語で対象化・差異化・関係化
→ 日本語で定義・設計・監査
→ 日本語正本成立
→ 実務上やむを得ない外部接続だけ他言語へ局所射影
```

他言語は、正確性、互換性、実行可能性、検索再現性のため必要な接続面に限り、局所例外として使用できる。

局所例外には次を含む。

- conventionalなcode commentのうち、外部規約または既存project慣行が固定する表現
- Schemaのidentifier
- protocolのterm
- command nameとoption
- packageのmetadata
- programming languageの予約語・標準構文
- 外部API・library・frameworkの固定識別子
- URL、path、environment variable、commit hash、branch、tag、版識別子
- 外部toolが固定文言を要求する短いagent instruction text

例外表記の一般語義を内部意味へ逆流させず、GUI-Shell側の責任、境界、採否、失敗時挙動は日本語で定義する。

次の確立用語・固有名は原形を保持する。

- GUI Shell
- Runtime Operation Shell
- Shell Core
- Adapter Contract
- Authority Strip Conformance
- Content Exposure Boundary
- FrameworkRiskProfile
- Approval
- AuditEvent
- RecoveryAction
- BLUE-TANUKI
- Rust helper

既存文書に英語正本または日英並列正本が残ることを、本規定への適合とみなさない。移行は対象、責任、互換性、test、履歴を固定した境界付き変更として行い、機械翻訳や一括置換を完成証拠にしない。

### 23. 製品姿勢

GUI Shellは、操作を快適にしてよい。

GUI Shellは、権限を隠してはならない。

UIは操作面であり、system authorityではない。

Schemaとconformanceはcontract gateである。

快適性のために、Approval、Audit、visibility、Recovery要件を弱めない。

local owner operationを、堅牢性低下の理由にしない。

Product UI完成をcontract完成として扱わない。

## 第III部 現行作業・phase指針

### 24. 正本

第I部の優先順位modelの範囲内で、リポジトリ固有の現行指針を次の順に使用する。

1. 作業対象についてユーザーが明示した最新版の仕様・工程表・実装指示書と、それへ同期した現行進捗。これらが現行phaseの対象・Acceptance・状態を決める。
2. `docs/specs/adapter-conformance.md`、`docs/specs/content-exposure-policy.md`、`docs/specs/approval-visibility-boundary.md`および関連する`docs/specs/`契約。`docs/specs/gui-shell-spec-v1.md`は基盤仕様として扱い、最新版の現行指示で置き換えられていない技術責任範囲にだけ適用する。
3. `specs/*.schema.json`
4. `tooling/conformance_tests/`
5. 現行工程の入口・release gateを示す`ROADMAP.md`と、適用範囲内の技術選択文脈を示す`docs/standards/gui-shell-extended-standard.md`
6. 既存実装pattern

同じ仕様・工程表・実装指示書に複数版がある場合、明示された最新版だけを現行正本とする。旧版は当時の決定・失敗・証拠を確認する補助的な履歴資料であり、要求・Acceptance・進捗を現行状態へ自動継承しない。

日本語の意味正本については`規定/00_日本語基底規定.md`、正本の所在と責任については`規定/正本索引.json`を併せて確認する。これらは、上記実装contractの安全要件を弱めない。

作業時点の現行状態は、編集開始前にfetchしてremoteと照合した最新Repositoryの正本・Contract・Code・Testで確定する。添付文書、過去会話、過去進捗記録は要求・履歴として参照できるが、現行状態の証拠や現行正本を置き換えない。remote更新または正本間の矛盾を検出した場合は、古い状態を前提に続行せず、最新側を読み直して適用範囲を確認する。

衝突がある場合は、Shell Core authority boundary、Schema integrity、conformance coverage、operator safetyを保持する、より厳格な規則を選ぶ。

### 25. 必須作業順序

オーナーが明示的に別の指示をしない限り、次の順で作業する。

1. 最新版の現行仕様・工程表・実装指示書と現行進捗を確認し、phase、Acceptance、実状態を確定する。旧版は補助履歴としてのみ読む
2. `docs/specs/gui-shell-spec-v1.md`を基盤仕様として確認し、現行最新版に置き換えられていない技術責任範囲に限って適用する
3. 関連する`docs/specs/` contract文書を読む
4. `docs/standards/gui-shell-extended-standard.md`を読む
5. `specs/`配下の関連Schemaを読む
6. Shell Core / UI / Adapter / Rust helperの境界を保持する
7. contract変更時は実装前にSchemaを追加または更新する
8. Product UIより前にconformance testを追加または更新する
9. 最小かつ境界付きのcodeを実装する
10. validationを実行する
11. 正確な結果を報告する

後続phaseの安全性、検査可能性、validation可能性を低下させる方法でlocal taskを最適化してはならない。

### 26. 現行指示・ROADMAP参照

現行ROADMAPの正本は`ROADMAP.md`である。

基盤の共通実装仕様は`docs/specs/gui-shell-spec-v1.md`である。同じ製品工程に対する仕様・工程表・実装指示書は、ユーザーが明示した最新版が現行正本となる。現在のrev5製品構築工程では、最新rev5指示と`docs/REV5_PRODUCT_PROGRESS.md`が現行phase・Acceptance・状態を決める。旧版は最新版で置き換えられていない基盤責任範囲または補助履歴に限って参照し、現行phaseを上書きしない。

拡張標準は`docs/standards/gui-shell-extended-standard.md`である。

日本語基底と意味正本は`規定/00_日本語基底規定.md`である。

現行正本、責任地図、外部参照固定点は`規定/正本索引.json`である。

実装言語と安全境界の方針は`docs/LANGUAGE_POLICY.md`である。

`specs/`配下のSchemaは、runtime、Adapter、Capability、Permission、Approval、Audit、Recovery、diagnostic、update、content exposure、framework risk、runtime manifest、Adapter manifest、agent runtimeの各surfaceに対するcontract gateを定義する。

### 27. 段階固有規則

`ROADMAP.md`の現行phaseとrelease boundaryに従う。

リポジトリ文書内のrelease gate blockerが解消され、strict validationが成功し、明示的なowner GOがあるまで、完成製品releaseを主張してはならない。

phase固有実装は、次を保持する。

- Schema-firstのcontract change
- conformance-firstのcoverage
- Shell CoreのFlutterからの独立
- BLUE-TANUKI reference runtimeのAdapter Boundary背後への隔離
- 境界付きRust helper authority
- Windows-first release evidenceとLinux development evidenceの分離
- macOS host validationが成立するまでのmacOS known limitation処理

### 28. 工程内Acceptanceと最終品質保証の分離

工程内Acceptanceは、その製品機能が開発を続けられる状態にあるかを確認する。製品全体の完成・release acceptanceとは分ける。必要な安全・Authority・Approval・Audit・Recovery・Content Exposure境界は常に維持するが、最終出荷級の全数回帰、長時間稼働、網羅的fault injection、性能、Formal Evidenceを未統合の機能へ前倒ししない。

統合後の徹底検査は、現行rev5仕様が定めるFinal QA工程へ送る。必要な証拠がAcceptance Contractを満たしたら工程を閉じる。「さらに検査できる」ことだけを理由に同じ工程を延長しない。Acceptance外で見つけた事項は、現工程の機能成立・安全境界を直接破壊しない限りFinal QA、後続工程、release gate、既知制約のいずれかへ明記して移送する。

### 29. D4 Pocket rev5 製品構築先行運用

現行の工程順、Acceptance、進捗、Final QA移送先は`docs/REV5_PRODUCT_PROGRESS.md`、`docs/FINAL_QA_QUEUE.md`、`ROADMAP.md`およびユーザーが提示した最新rev5正本で確定する。`docs/REV4_ACCEPTANCE_LEDGER.md`とrev2/rev3進捗は履歴・証拠の出所として保持し、rev5の現行phaseを上書きしない。

R2 Agent Taskは製品開発を続けるための機能が成立したものとして扱い、R2-A〜Hの追加探索・深掘りをR3以降の開始条件にしない。未実施または最終統合状態で再確認が必要な項目はFinal QA queueに残す。これはR2-A〜Hがすべて合格した、通常ReleaseのTask capabilityを昇格した、またはrelease-readyであるという主張ではない。明示されたRelease Gateと通常Releaseのfail-closed状態を保持する。

現行製品phaseとその状態は、最新Repositoryの`docs/REV5_PRODUCT_PROGRESS.md`を正本、`ROADMAP.md`を入口として確認する。過去の指示書にある「次に開始するphase」やphase番号を現在状態へ転記せず、進捗正本に記録された最新OPEN phaseとAcceptanceから続行する。現時点でCLOSEDの製品Acceptanceは、再現可能な回帰が該当条件を壊した場合だけ再開する。以後は最新rev5で定める残りの製品機能を順に完成し、Feature Complete後に限ってQ0〜Q7 Final QAを開始する。

通常のOwner承認・入力を開発進行の待ち条件にしない。製品のAuthority境界は維持したうえで、テスト専用Owner identity、fixture、UI automation等の隔離された試験経路を用いる。production identity・鍵・不可逆な事業判断・Final GOなど、Owner本人にしか成立させられない事項だけをOwner待ちとして残す。実際のOwner確認画面を自動承認するために既存安全境界を迂回してはならない。
