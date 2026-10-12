# D4Pocket エージェント実装・監査規定

本書はD4Pocket（旧GUI-Shell）のAI実装エージェントが守るリポジトリ共通規律である。BLUE-TANUKIのQE思想を開発方法論として採用する一方、D4Pocket固有の権限、安全、復旧、OS別証拠、Git運用を保持する。BLUE側のHDS-BRAIN実装、192小単位、別リポジトリの成果や書込み権限は移植しない。

## 第I部 共通基底規律

### 0. QE品質工学の最上位原則・実行順序

**QE（品質工学）が開発全体を包含し、全体設計→一気通貫BUILD→実物QC→対象別QA→独立したOwner最終判断へ接続する。** 現行ステージ名は`QE_DESIGN → QE_BUILD → SYSTEM_SHAPED → BUILD_COVERED → QE_QC → QE_QA → QE_RELEASE_REVIEW` とする。これは本案件の品質操作・工程配置であり、一般の規格名称を再定義するものではない。

- `QE_DESIGN`：対象・目的・要求母集団・成立条件・干渉原理・責任・反例を先に確定する。
- `QE_BUILD`：コード片やSchemaの数ではなく、producer→consumerの実接続を一気通貫で前進させる。
- `SYSTEM_SHAPED`：限定された一仕事が、通常入口→現行Broker認可→元Toolの作用→結果→Audit／表示／帰還まで実際に通る。
- `BUILD_COVERED`：全要求・機能・OS別consumerを照合し、`CODE_PRESENT`／`WIRED`／`BUILD_GAP`／`BLOCKED_AUTHORITY`を区別する。未接続の必須機能を隠さない。品質・公開の合格ではない。
- `QE_QC`：結合済みの実物を全体検査し、不具合・根因・副作用・OS差・資源・性能・長期運転を観測する。必要な上流へ戻り、是正・同条件再試験を行う。
- `QE_QA`：同一候補のQC証拠から、対象・機能・OS・版・条件ごとに`QA_SCOPED`／`QA_NOT_ESTABLISHED`／`OUT_OF_SCOPE`を判断する。
- `QE_RELEASE_REVIEW`：正式出荷、鍵、署名、不可逆判断、Final GOはOwnerが独立して決める。

**作業の切れ目と品質／Gitを閉じる単位を分離する。** 毎小タスクの開始・切替・再開時に、Ownerの最新指示、適用AGENTS.md、指定指示書、最新HEAD・remote・dirty差分、目的・作用・入出力・次consumerを再読する。これは目的ドリフトを減らすための再同期であって内部状態が消えた保証ではない。小単位では接続状態と残差を記録するだけでよく、関連する小単位を**BUILD_BATCH**へ束ねて必要な技術確認・Git保全・報告を閉じる。小単位のたびに全suite・QA判定・バックアップ回転・commit/pushを繰り返さない。

**安全第一・堅牢性第二。** Authority、Permission、Approval、Credential、秘密、Audit、Recovery、Content Exposure、Workspace／Host、process監督、OS保護、非冪等外部作用の境界はBUILD中も常時維持し、当該変更の必要な否定試験・安全停止をその場で実施する。不成立なら危険な経路を無効化・隔離する。既存試験・リリースゲートの削除や恒久skipは許可しない。

対象OSはWindows-firstの優先順位を維持した上で**Windows／macOS／Linux（Desktop）、Android／iOS（Mobile）**。Linux nativeは独立した製品対象であり、WSL Ubuntu、WSLg X11、headlessの開発用証拠をLinux nativeの導入・GUI・権限・配布・Waylandの完成へ転用しない。Linuxの追加要求`D4-QE-R13`と試験`D4-LNX-T01〜T08`、Linux専用`QEB-D4-LNX-01〜03`は2026-10-12再構成3文書v1.1の**正本候補**。対象範囲・施工着手・正式サポートは別途採否する。

本§0、§19、§20、§25、§28〜31は採用済みの**QE施工方法**を優先して示す。既存のrev5原本、採用済み緩衝補遺（`BUF-R01〜R09`、`BUF-T01〜T24`）、製品契約、安全・release gate、過去のCLOSED／FAILは削除しない。従来文書に小単位ごとの全suite／個別QA／毎回Git閉包が残れば、その部分は旧施工方式の残差として別途文書同期し、今回の`AGENTS.md`だけの更新で他文書を改竄しない。安全上の強い条件は常に保持する。

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

品質基準面は owner / Codex が実行する local validation、smoke、release verification、Windows 実機 evidence とする。Codex は要求から正常・境界・失敗・権限否定・回帰試験を導出して実行し、未試験を成功へ昇格しない。小単位は目的と実consumerの再同期に使い、関連小単位を束ねたBUILD_BATCH末に必要な技術確認・監査・commit/push・remote HEAD照合を閉じる。全体QC・QAを各小単位へ前倒ししない。ただし安全重要変更の必要な否定・停止・復旧確認と既存release gateはその作用時に維持する。検査の削除・弱体化には owner 承認を必須とし、Authority、Approval、Audit、Recovery、Content Exposure Boundary、owner GO、release gate を弱めない。

### 4. 完成証拠規則

完成の主張は証拠ではない。

BUILD_BATCHまたは統治変更ブロックを完了と報告する前に、次を特定する。小単位は途中の接続状態・次consumerを記録し、完成を主張しない。

- 実装した挙動
- その挙動を実行するproduction、runtime、contract、validationの経路
- 実際に実行した正確なvalidation command
- 正確な結果
- 実行しなかったvalidation
- 残存するstub、mock、placeholder、TODO、未接続contract、環境制約、既知の制約

文書、Schemaの存在、mock成功、fixture成功、単体test成功だけを、production経路または製品挙動が完成した証拠として報告してはならない。

security-critical、authority-critical、audit-critical、recovery-critical、release-criticalな変更では、実際の統治経路が実行されたことを何の証拠が示すかを記載する。

変更を伴うBUILD_BATCHまたは統治変更ブロックは、オーナーがlocal-only、audit-only、review-only、no-commit、no-pushへ明示限定しない限り、Git閉包をもって完了する。小単位の再同期は別である。

リポジトリ状態を変更したBUILD_BATCH／統治変更ブロックは、次を満たすまで完了ではない。

- file編集前に、リポジトリの2世代backup規約で現在push済みのリポジトリ状態を保存する。
- 意図した変更を実装し、taskの残骸を残さない。
- 該当QE段階で必要な技術確認と安全gateを実行し、成功・失敗・NOT_RUNを正確に把握する。
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
- macOS資格情報の登録と論理失効は`docs/specs/macos-credential-vault.md`の限定control経路とする。公開要求はmetadataだけで、親Rust native UIが秘密値を既存private pipeへ書き、子Rustの別個期限付きOwner確認後に既存Broker receiverへ渡す。Swift／Flutterへ秘密値や承認boolを返さず、Keychain保存先からPermission・Approval・Agent trustを生成しない。現metadataへ束縛した論理失効だけを許可し、秘密注入・物理削除・任意保管先探索へ広げない。
- macOSのMCP接続・Tool実行・切断だけを`docs/specs/macos-mcp-center.md`の限定control／runtime経路へ追加する。Rustの期限付きnative確認が同一要求hashを既存Broker receiverへ渡し、Brokerが現在catalog・Schema・一回Permission・Auditを再評価する。資格情報保管の秘密注入禁止に対する追加consumerは`docs/specs/macos-mcp-credential-binding.md`の公開ID参照・別個native確認・現在用途／対象／失効／暗号文照合・対象stdio環境一つだけに限定する。Provider／Agent／A2Aへ広げず、参照単独からAuthorityを生成せず、秘密をFlutter／Swift／応答／Auditへ返さない。App Sandboxを継承し、停止はChild PIDから束縛した所有process groupだけを確認する。Windows Job Objectの全子孫／Broker crash保証をMacへ転用せず、group停止未確認・結果不明・Audit失敗を成功へ昇格しない。Flutter／SwiftにOwner bool・任意PID制御・別Authority経路を追加しない。
- macOSの作業領域OS選択は、通常GUI親processに静的同梱するRust UI部品のNSOpenPanelだけを使う。固定C ABIはversion-only公開要求と既存helperへのpipe fdを受け、OS scope実値をそのprivate pipeだけへ書く。Flutter／Swiftへbookmark・scope実値を返さず、Flutter入力のprivate frameをRunnerが拒否する。Rust helperはOS bookmarkを起動中だけ解決し、既存Brokerは選択投影だけを別sourceで処理する。このUI専用FFIへBroker資格、署名検証、Approval token、外部command dispatch、Audit確定を移してはならない。Owner承認・Permission・Approval・登録は引き続き別個の独立Broker経路で再評価する。UIからpath・bookmark・scopeを受け取らず、拒否・終了時は各Rust processが同じOS URLのaccessを終了する。sandboxを無効化せず、helperのinherit-onlyを保持し、scopeを永続化しない。責任正本は`docs/specs/macos-workspace-selection.md`。P13の追加CLI file選択だけは`docs/specs/macos-agent-cli-selection.md`に従い、同じ限定UI／private輸送から別sourceへ接続する。選択そのものからprocessを起動せず、sandbox外の明示選択fileのprobeは別個Owner確認後の既存Broker登録だけで許可する。download・任意command・Task Permission・Approvalへの転用は許可しない。

- macOSのA2A接続は`docs/specs/macos-a2a-center.md`の限定control経路とする。同一要求hashへ束縛したRust native Owner確認だけを既存Broker receiverへ渡し、既存IPv4 loopback HTTP Agent Card取得・metadata-only一覧を使用する。Credential注入・Task／Message／Artifact／Stream作用・Trust／権限生成へ広げず、Flutter／Swiftの承認bool・Owner資格・別Authority経路を追加しない。App Sandboxと既存Audit／Recovery／内容露出境界を保持する。

macOSのHost登録は`docs/specs/macos-host-registration.md`の限定control経路とする。既存Host構造検査と同一要求hashをRust native Owner確認へ束縛し、承認された要求だけを既存Brokerへ渡す。公開metadata登録・一覧・表示切替に限り、remote接続、Trust、Permission、Approval、Credential、Task実行権限を生成しない。Flutter／Swift承認bool・任意Owner操作への拡大を禁止する。

macOSのWorkspace読取承認・失効・全体baseline保存だけを`docs/specs/macos-workspace-inspector.md`の限定control経路へ追加する。Rust native確認は同一要求hash・Workspace ID・登録hash・内容露出範囲へ束縛し、現在登録・期限・secret境界・bounded読取・Auditを既存Brokerが再評価する。Task実行・書込・Credential・Agent trust・OS scopeを生成せず、Flutter／SwiftのOwner bool、任意root／path、別filesystem経路を追加しない。App SandboxとWindows経路を保持する。

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

### 19. QE段階別の技術検査・QC・QA

**QE_BUILDの小単位では**、構文・型、変更したproducer／consumer、必要build・限定smokeを確認し、全Rust／Flutter suite、全数Conformance、故障注入、耐久、個別QA票を一律には要求しない。Authority、Approval、Credential、秘密、Audit、Recovery、Content Exposure、process、OS／Workspace境界に影響する変更は必要な拒否・停止・回復の検査をその場で行う。不成立なら危険な実行経路は有効化しない。試験や安全gateの削除・恒久skipを禁止する。

**BUILD_BATCH末では**対象の型／compile・build、関連test、実consumer短時間smoke、差分、日本語基底、変更に関係するSchema／Conformanceを検査する。契約変更は日本語意味正本→Schema→必要な否定・適合試験→実consumerの順に接続する。安全・出荷の必須gateがより強い検証を課す場合はそれを保持する。

従来の検査コマンドは存続し、変更した範囲・OS・QE段階に応じて適用する。Schema／Conformanceに変更した場合はその検査を実施し、全機能候補に対する全面回帰はQE_QCで実施する。

```bash
python tooling/schema_check/check_schemas.py
python tooling/conformance_tests/run_conformance_skeleton.py
cd native/rust_helper && cargo test
cd native/windows_dns && cargo test -- --test-threads=1
cd apps/desktop_flutter && flutter analyze
cd apps/mobile_flutter && flutter analyze
```

`python`不在なら`python3`を使用する。Windows固有の検証をLinuxに流用しない。実施したcommand、環境、exit、証拠class、未実施の`NOT_RUN`と理由を記録し、必要な証拠が欠ける成立状態をCLOSEDへ上げない。

`BUILD_COVERED`の後、QE_QCで実物全体の正常・異常・安全・長時間・資源・性能・OS差・緩衝実作用と副作用を試験し、是正時には必要な上流まで戻って同条件で再試験する。QE_QAは候補版・OS・条件別の保証判定、Owner GOはさらに独立。既存`docs/FINAL_QA_QUEUE.md`のQ0〜Q7・FQ識別子と過去結果は保持し、正式Q段階の新規開始は明示modeと候補の指定がある場合のみ。Q0新候補凍結、Q1〜Q5実物検査、Q6同一候補証拠集約、対象別QE_QA、Q7 Owner判断を区別する。

Linuxは`LINUX_NATIVE`／`LINUX_WSL_DEV`／`LINUX_WSLG_X11`／`LINUX_HEADLESS`を別証拠として扱い、distro・版・kernel・arch・X11/Wayland・権限・process監督・Linux配布／installed製品を個別に確認する。WSL／WSLgの成功をLinux nativeの製品合格とは呼ばない。

### 20. Git運用方針

本リポジトリは、direct-main owner workflowを使用する。

変更を伴うBUILD_BATCH／統治変更ブロックは、オーナーのlocal-only・no-commit・no-push明示限定がない限り、限定検証後にcommit・pushしてremoteを照合する。小単位の再同期ごとにGit世代回転やcommit/pushは行わない。

標準workflow:

1. `main`で作業する
2. `main`への直接作業を標準とし、通常のfeature branchまたはpull requestはオーナーが明示要求しない限り作らない。前項のowner許可済み手動Actions検査に限り、一時`codex/`検証branchを例外として使用できる。検査対象commitを記録し、検証が閉じた後は成功した正確なcommitだけを`main`へfast-forwardし、remote HEAD確認後に一時branchをlocal／remote双方で削除する
3. 変更するBUILD_BATCH／統治変更ブロックの編集前に、`origin`をfetch / pruneし、`main`がcleanかつ`origin/main`と整合し、他writerの差分がないことを確認する。不整合なら編集前に解消するかblockerを報告する
4. BUILD_BATCH／統治変更ブロックの編集前に一度だけlocalの2世代backup pairをrotateし、現在push済みの変更前状態を保存する。小単位の切れ目ごとには回転しない
   - `codex/backup-main`が存在する場合、`codex/backup-main-prev`を`codex/backup-main`へforce-updateする
   - `codex/backup-main`を現在push済みの`main`へforce-updateする
5. 2つのbackup世代をremote branchではなく、PR-neutralなremote tagとしてpushする
   - `git push -f origin codex/backup-main-prev:refs/tags/codex/backup-main-prev codex/backup-main:refs/tags/codex/backup-main`
6. 境界付き実装と§19で該当するQE段階の技術確認・安全検証を行う。失敗とNOT_RUNは隠さない
7. 完了したBUILD_BATCH／統治変更ブロックを`main`へ限定commitする。手動Actions検証branchでは必要な検証が成功した正確なcommitだけを`main`へfast-forwardする
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

BUILD_BATCH／統治変更ブロックの完了報告は次を含める。小単位の途中報告は`QE_STAGE`・進んだ実consumer・次consumer・途中状態・最小技術確認・NOT_RUNを優先し、個別QA合格票を作らない。

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

validation結果は、どのcommandが成功、失敗、未実行かを明記しなければならない。QE_QCは実物・候補版・OS・根因・是正・再検査・副作用を、QE_QAは対象・版・OS・条件・証拠の`QA_SCOPED/QA_NOT_ESTABLISHED/OUT_OF_SCOPE`を報告する。

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

### 25. 毎タスクの再読とBUILD_BATCHの必須作業順序

1. **小単位を開始・切替・再開するたび**、Owner最新指示、適用AGENTS、指定指示書、現行仕様・工程・進捗を読み直す。過去会話・古いphase番号で現況を推定しない。
2. 現在の`main`／remote HEADとdirty差分、他writer、関連code・contract・testを照合する。既存作業のreset／clean／revert／上書きを禁止する。
3. 元目的、対象・非対象、入力・作用・出力、次consumer、対象OS／Host、観測結果、安全・停止条件を短く固定する。
4. 基盤仕様`docs/specs/gui-shell-spec-v1.md`、該当`docs/specs/`、拡張標準、関連Schemaを必要範囲で読み、Shell Core／UI／Adapter／Rust helperの責任を維持する。
5. 契約変更は意味正本→Schema→必要なConformance→実consumerへ接続する。技術的に継続を阻む欠陥は局所修正し、安全・権限・秘密の欠陥は直ちに修復・停止する。
6. 小単位では`CODE_PRESENT/WIRED/BUILD_GAP/BLOCKED_AUTHORITY`を記録して次consumerへ進み、新しい切れ目で1へ戻る。小単位ごとの全suite／QC／QA／Git回転は行わない。
7. 実接続が前進したBUILD_BATCH末だけ、§19の必要技術確認・安全検証、§20のD4固有2世代**remote tag**・限定commit/push・remote HEAD照合を行い、根拠・未実施・残差を報告する。
8. `BUILD_COVERED`後に実物QE_QC、対象別QE_QA、Owner最終判断へ進む。ただし既存Q0〜Q7の新規開始、通常Release、Owner GOは別の明示条件とする。

後続の安全、検証可能性、正式出荷の根拠を弱める局所最適化を禁止する。

### 26. 現行指示・ROADMAP参照

現行ROADMAPの正本は`ROADMAP.md`である。

基盤の共通実装仕様は`docs/specs/gui-shell-spec-v1.md`である。同じ製品工程に対する仕様・工程表・実装指示書は、ユーザーが明示した最新版が現行正本となる。現在のrev5製品・緩衝基盤工程は、最新rev5指示、採用済み緩衝補遺と`docs/REV5_PRODUCT_PROGRESS.md`がphase・Acceptance・状態を決める。§0がQE施工方法を定め、2026-10-12のQE再構成3文書は別のOwner採用まで正本候補である。旧版は最新版で置き換えられていない基盤責任範囲または補助履歴に限って参照し、現行phaseを上書きしない。

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
- Windows-firstの検証優先順位を維持し、Linux native製品の証拠とWSL／WSLg／headless開発証拠を分離すること（Linuxを要求母集団から除外しない）
- macOS host validationが成立するまでのmacOS known limitation処理

### 28. QE_BUILD・実物QC・対象別QAの検査配置

QEは要求・設計・実装・観測・是正を通じた上位思想である。小単位の有限Acceptanceは次の実装へ進めるための局所条件であり、製品全体の品質やreleaseを保証しない。実物がまだ結合していないQE_BUILDでは、実consumer接続と技術継続に必要な確認を優先し、全suite／長時間／網羅的fault injectionはBUILD_COVERED後のQE_QCへ配置する。ただし重大な安全・Authority・Permission・Approval・Audit・Recovery・secret・Content Exposureの確認は全段階で即時実施する。

実物QE_QCでは不具合の根因へ戻り、目的・設計・実装・試験器を必要に応じて是正して同条件で再試験する。QE_QAはQC証拠に基づく条件付き保証でありOwnerのFinal GOではない。既存`docs/FINAL_QA_QUEUE.md`、Q0〜Q7、FQ識別子、旧Q0/Q1履歴は保持し、現行`DEFERRED`を方法論の更新で解除しない。CLOSEDの工程は関係する再現可能な回帰がない限り再開せず、未観測・UNKNOWN・旧FAILを消さない。

### 29. rev5製品・B工程の現況と施工境界

作業時には最新`docs/REV5_PRODUCT_PROGRESS.md`と明示Owner指示を読み直す。2026-10-12の照合時点ではP2 Compare／P3 Handoff／P12 Windows／P13 Mobile・Non-Windowsは局所CLOSED、B0〜B3はCLOSED、B4-L1／B4-L2は局所CLOSED、**B4全体はOPEN**である。B4は既存Windows Desktop IPC→Broker→実Codex→Agent Centerの単一製品縦断が未実証。B5/B6と新候補の正式Q0〜Q7は未完了、正式Q工程は`DEFERRED`。記録済みのB4-L2 test PASSはこの製品全体受入れに昇格させない。

CLOSEDを理由なく再施工しない。B4の実製品接続、Linux native追加BUILD_BATCH、B5/B6の緩衝自己適用・実物QC、正式Q0〜Q7はそれぞれ別の明示指示・受入条件が必要。Linux未実装をP13やWindows B4の局所完了記録へ遡及させない。B3の153原因対応行の分類を全原因の解消や全緩衝作用の実証と呼ばない。既存R2-A〜H、`FQ-TEST-LOOPBACK`、取得不能証拠の残余を維持する。

通常Release `task_execution=unsupported`、`release_ready=false`、release blocker、正式なPublisher／署名／鍵／Owner GOは不変とする。

### 30. 緩衝基盤補遺とQEへの継承

2026-10-09採用の緩衝基盤rev5補遺3点（`docs/緩衝基盤_rev5補遺_QC/`）とその採用記録は要求意味・旧結果の根拠として保持する。旧`BUF-R08`の「構想から実装・試験・是正まで品質責任を貫く」要求は、QEが全開発を包含する規律として引き継ぐ。QE_BUILD中にも安全・意味・責任を検証するが、全体実物の詳細なQE_QCと対象別QE_QAを小単位ごとに繰り返さない。

緩衝基盤の本体は**合法な変換・隔離・環境固定・状態調停によって元Toolの実行を成立させること**であり、診断・エラー報告機能ではない。全ボトルネックを観測対象に残し、原因不明を成功にも吸収不能にも変えない。元Taskの目的達成条件と`BUF-T09`等の品質試験IDを別々に保持する。正式Q0〜Q7は未承認で開始しない。

### 31. D4Pocket QEとLinux追加設計の採用境界

2026-10-12作成のLinux明示改訂3点（D4Pocket QE統合実装仕様書／統合工程表／Codex QE施工指示書 v1.1）は、現時点では**未採用の詳細正本候補**である。本AGENTS.mdの更新はOwnerが許可した**本ファイルのQE方法論同期**のみであり、それだけで候補全文・新Linux工程・B4以降のコード変更・Actions実行・他リポジトリ変更・releaseを承認しない。次の委任があれば、rev5原本・採用済み補遺・現行進捗と逐条照合し、差分を安全に同期する。

D4Pocket固有のremoteバックアップ正本は`refs/tags/codex/backup-main`と`refs/tags/codex/backup-main-prev`。BLUE側の`refs/heads`へ自動変更しない。D4の手動`workflow_dispatch`限定Actions運用、direct-main、単一writer、Shell Core／Broker authority、各OSの証拠区分を保持する。LinuxはWindows・macOSと並ぶ独立Desktop対象だが、native製品の実接続・導入・Release・保証が実証済みとは主張しない。既存試験・Release Gateを省略せず、作業開始時に現物と正本を再読する。
