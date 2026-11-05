# D4 Pocket Codex施工指示書

文書ID：`D4-QE-CODEX-20261105`  
版：v1.0（2026-11-05、健全化後の開発施工指示案）  
対象：`gatchimuchio/D4-POCKET`  
観測固定点：`main` `55a43a7c26e27ace212805318914d43e25665e4f`  
併読：`D4-QE-SPEC-20261105`、`D4-QE-ROADMAP-20261105`  
効力：Ownerが本3点をCodexへ施工指示として渡した場合に適用する。既存`AGENTS.md`、採用済みQE運用、安全境界、上位指示を無断で置換しない。

## 0. 最上位命令

この作業の目的は、**AI／LLM／Agent自身がD4 Pocketを能力の保管庫・接続機構・実行基盤として利用できる、汎用的な実製品を完成させること**である。Ownerは権限・責任・公開の決裁主体、Agentは能力を使用する主体である。D4 Pocketを単なる管理GUI、外付けTool一覧、単一Agent用Console、BLUE-TANUKIの専用部品として設計し直してはならない。

実装の成否はコード行数、テスト数、文書数、Git操作回数ではなく、**実producerが作った結果を必要なconsumerが安全に利用し、結果・証拠・復旧まで閉じたこと**で評価する。完成を「最低限動くMVP」に再定義しない。

必要な仕事は自律的に進める。十分な根拠がある範囲では細かな編集ごとにOwnerへ確認しない。一方、未許可の資格使用、署名・配布・公開、Owner GO、権限境界変更、既存記録消失などの意味の大きな判断は独断で進めない。

## 1. 施工開始時の必須観測

まず対象リポジトリを開き、次の順で**現行状態**を読み直す。過去セッション要約や本書の観測固定点を最新事実と見なさない。

1. `git status --short --branch`、`git rev-parse HEAD`、remote／main同期状態、作業treeと他セッションの書込み有無を確認。**単一writer**と既存ユーザー差分を尊重する。
2. `AGENTS.md`、Codex global instructions、リポジトリ固有の規定、日本語基底規定・正本索引、適用する開発運用ルールを先に確認。
3. rev5製品の原本要求（P0〜P13）と`docs/QE_2026-10-22/F07_rev5原本要求照合_20261030.md`、2026-10-22に採用されたQE正本3点・採用記録を照合。旧版を破棄せず、それぞれが何を定義・更新したかを分ける。
4. `docs/REV5_PRODUCT_PROGRESS.md`、`docs/FINAL_QA_QUEUE.md`、`release_blockers.registry.json`、H01〜H07健全化記録、B0〜B6・`BUF-R01〜R09`・`BUF-T01〜T24`の実装／検査履歴を確認。
5. 実現に関係するRust Broker、CLI／Runtime Adapter、MCP Center、A2A Center、Flutter Shell、Compose、Export、AI Edit、Module Pruning、OS配布、Schema・Contract・既存テストの本体を読んで**実consumerがあるか**を確かめる。
6. 本3点の要求ID `D4-N01〜N11`／工程ID `W00〜W08`を現物へマッピングし、古い前提との差異を表示する。差異があれば現物を優先して本書側の改訂を提案し、都合よく既存要求を消さない。

**初期観測の期待値（実行時に再検証すること）**：2026-11-05基点で`main=55a43a7c...`、H01〜H07は履歴上処置済み、Conformance 267 checks等の有限PASS記録あり、strict日本語基底監査はFAIL残存、通常Releaseの`task_execution=unsupported`、`release_ready=false`、Release Blockerは16件未解決、B5 OPEN／B6未完了。**これらを見ずに成功宣言しない。**

## 2. 保持しなければならない原理・実装境界

### 2.1 不変条件

- AI／LLM／Agentは能力を発見・選択・消費する**第一級の実用主体**。Ownerは許可・責任・決裁主体。
- 権限の最終判定はRust Security Broker。Flutter UI、Python参照実装、Agent出力、Tool応答、Adapter metadata、Compose Manifest、独立Export Appへ権限判定を移譲しない。
- `Capability → Permission → Approval → AuditEvent → RecoveryAction`の因果を維持。Content Exposureは既存の`none / hash_only / summary / redacted / full`に従う。
- Credential／Token／秘密の平文露出、無断Tool起動、自己承認、Tool応答由来の権限、Handoff時の権限複製、Replayによる重複作用を禁止。
- Windows-firstだがLinuxはnative desktopの必須対象、macOS・Android・iOSの既存rev5製品要求も削除しない。
- BLUE-TANUKIやMINIDORAは**外部接続し得る対象**であり、別リポのHDS-BRAINや非公開原理をD4 Coreへ埋め込まない。
- P0〜P13・B0〜B4の同条件で受入れた部分を保持。以前のPASS/FAIL・版・OS・fixtureの意味を書き換えない。

### 2.2 正常例と負例の両立

新機能は一つの正例だけでは完成しない。最低限、正常→権限不足→不正入力／未対応Capability→取消／期限切れ→障害・再起動→復旧のうち作用に必要な境界を選び試験する。安全目的を唱えるだけの無意味な空実装・多重検査・自明なMock・過大な抽象化は避ける。信頼境界では必要な検証を欠かさない。

## 3. 実装進行命令

### 3.1 W00：実装要件・接続・証拠の固定（最初に実行）

リポジトリを全面棚卸しし、次のレコードを作る。既に同等の現行成果があれば再生成せず、差分だけ補う。

| フィールド | 記入内容 |
|---|---|
| Requirement | rev5の原ID／P／BUF／Q／`D4-Nxx`、原文参照元 |
| Producer | どの実装がどの実結果・能力を生み出すか |
| Consumer | どのAgent／Broker／UI／App／OSがその結果を実消費するか |
| State | `CODE_PRESENT / WIRED / BUILD_GAP / BLOCKED_AUTHORITY`と成立条件 |
| Authority | Owner／Broker／Agentの責任・Permission・Approval境界 |
| Verification | 正常・失敗・回復、実行command、fixture有無、OS、artifact・commit SHA |
| Verdict | 現同条件での有限PASS／未成立／証拠不足、次の一手 |

成果物は**必要consumer別の優先差分表**である。既受入れのP2 Compare・P3 Handoff等を理由なく全面やり直さず、通常Release Agent Task、AI→MCP往復、異種Runtime、Compose実在照合、Export実物消費の不足を実装ターゲットとして特定する。

W00は「調査を行ったから完了」ではない。実際の最初のBUILD_BATCHへ入るためのProducer／Consumer／受入れ条件が特定され、後続工程の先行依存が確定したとき閉じる。

### 3.2 W01：通常Release Agent Taskの閉包

- 現行`codex-cli` adapterとtest-only `r2-e2e`の成立範囲を確認し、**通常Releaseで`task_execution=unsupported`となる根因**をコード・契約・Safety Gateで特定する。
- 実Agent起動、Task受理、Broker認可、Workspace、結果、Audit、停止・cancel・deadline、process cleanupを接続する。表示だけ先に`available`へ変更しない。
- Owner自身が許可操作できるUI経路と、Agent実行経路の権限を切り分ける。
- B5でこの経路を壊す安全重大課題があれば必要範囲だけ先行是正し、残る長期・資源測定とB6は後続の統合QCへ保持する。
- 一つの実仕事が成功・拒否・失敗／復旧まで追跡できたら`G1`証拠を固定する。

### 3.3 W02：Agent→MCP Tool→Agentの一巡

- `mcp_center.rs`のOwner一回Tool実行を維持しつつ、Agentが能力を要求した後にBrokerの許可を経て**実Toolの結果を受領し、その結果で後続Taskを変える**本番consumerを作る。
- AgentはTool登録・能力を観測できるが、Tool metadataやTool本文が権限を授与しない。秘密は既存exposure規則に従う。
- Tool不在、接続断、引数不正、資格失効、重複非冪等呼出し、異常応答の拒否とAuditを確認する。
- 「Tool Centerの手動呼出しが成功した」「Mockが返った」はG2の代用にならない。

### 3.4 W03：異種Runtime／Agentと実Handoff

- `codex-cli`のみに依存する部分を実装上の共通境界へ正規化し、**別種類の実Runtime**で登録・Task実行・結果を通す。Adapterごとの固有機能を捏造せず`unsupported`を正しく表現する。
- P2 Compare／P3 Handoffの既存成果を取り込み、独立Agent間のcontext・成果・diff・test・失敗を実consumerが利用することを示す。
- A2AはAgent Card表示の他にTask／Message／Artifact等の**本当に対応する操作**を対象版・相手を指定して試験する。対応していないStream等を実装済みとして表明しない。
- 引継ぎ先へPermission／Approval／Credentialを自動移譲しない。Owner Gate再確認を保持する。

### 3.5 W04：Compose・Capabilityの実在閉包

- 現行の`manifest_only`という正直な状態を維持しながら、登録されたRuntime／Tool／Agent／Capabilityと構成参照を突き合わせる。
- `not_registered / unsupported / permission_required / wired / build_verified / installed_verified`等の状態は必要な契約へ正規化し、**名前を付けただけで動く扱いをしない**。
- Owner UIから構成条件、使える作用、使えない理由が理解できるようにする。Producerが返した能力一覧を実consumerへ接続する。
- 実Compose後の作用・安全拒否・不足Capabilityを一連で実証する。

### 3.6 W05：独立Export・Module・AI Edit

- 現行のExport Manifest・Developer-only Windows bundle・AI Edit proposal-only経路を保持し、生成した構成を独立Appの実Runtimeが消費する導入線を成立させる。
- 選択Moduleはmanifestだけでなく、実Appの作用・非作用と対応させる。Binary pruning・artifact size・performanceは実物比較で判定する。
- AIの生成提案→差分→正本／契約検査→Developer／Owner審査→導入という分離を維持。Runtime自身が自由にSourceを改変し公開する構造にしない。
- App identity、Audit identity、署名未成立／成立、元source SHA、配布artifactの対応を曖昧にしない。

### 3.7 W06／W07：各OSの通常利用・製品体験

- W06 Windows：通常installed appの起動→Setup Doctor→Broker連結→AI Task→Tool→Audit／Recovery→Update／Rollback／Uninstallを同一候補artifactで試験する。配布identity、first-run、ユーザーデータ保全、Owner UI操作を含める。
- W07 Linux：native desktop実Session／X11・Wayland適用範囲を切り分け、Xvfb・WSL・headless等の限定証拠を一般デスクトップ成功に拡張しない。macOS・Android・iOSも各々の実機・配布・Device Link境界を独立評価する。
- 現物や必要資格を入手できないOSは**`EVIDENCE_MISSING`**として保持し、Windowsの結果で代替しない。環境提供や署名・配布に追加Owner決裁が必要ならそこで停止する。

### 3.8 W08：B5／B6・QC／QA・Release Review

- B5の未解決現象は再現条件、根因観測、是正差分、再試験を対応付ける。OneDrive、loopback／TLS、SAC、process cleanup、自己適用、長期・資源影響を実条件ごとに切る。
- B6で新たな結合と既存P／B受入れに回帰がないか確認。境界を跨ぐTask／Tool／Export／Update／Recoveryの相互干渉を試験する。
- 候補を凍結して`QE_QC → QE_QA`を実施し、Q0〜Q7を現候補・各OS・実配布範囲に沿って評価する。**旧版限定PASSを今の正式QAに転写しない。**
- `release_blockers.registry.json`の16未解決の現状態を再照合し、各項目は必要証拠・Owner決裁に基づき更新する。
- Ownerが明示GOするまで`release_ready=false`を保持。Codexは署名、一般公開、配布、Releaseタグ確定、Owner承認の代行をしない。

## 4. 作業バッチと検証の実務規則

1. `AGENTS.md`で定める`QE_DESIGN → QE_BUILD → SYSTEM_SHAPED → BUILD_COVERED → QE_QC → QE_QA → QE_RELEASE_REVIEW`を保持する。各状態を単純な「テストPASS」で置換しない。
2. **縦断的に意味のあるBUILD_BATCH**を組む。一つのConsumerが動くために必要ならRust・Flutter・Schema・試験をまとめて変えてよい。逆に小さな命名・整形だけを「一工程」として水増ししない。
3. 実装前に既存コード・テスト・契約を読み、成立済み部品の再利用・拡張を優先。全面書換え、Fake接続、意味のない抽象化、テストの出力偽装は禁止。
4. 仕様上の矛盾・測定上の失敗は発生源まで追跡し、正本要求と因果が対応する修正に限る。CIや検査を通すためだけに判定器・fixture・期待値を緩めない。
5. 実際のローカル検証は対象差分に相応した範囲で行う。結合節目ではSchema、Conformance、Rust test、Flutter analyze／test、Manifest、Release Gateを同一候補へ適用する。**1行編集のたびの全テスト、Commit、Pushは不要。**
6. 既存の検査入口・オプションは実行時に`--help`やコードで確認する。代表例は以下（コマンドの存在と引数は現行リポの正本を優先する）。

   ```powershell
   python tooling/schema_check/check_schemas.py
   python tooling/conformance_tests/run_conformance_skeleton.py
   python tooling/manifest.py --check
   python tooling/release_gate_check.py --release-track non_windows
   python -X utf8 tooling/run_cargo_isolated.py test --locked --offline --manifest-path native/rust_helper/Cargo.toml
   python tooling/validate_all.py --flutter-step desktop_flutter_analyze --desktop-platform windows
   ```

7. OneDrive同期フォルダーをビルドキャッシュ・一時出力・大量Test artifactの置き場にしない。既存のisolated Cargo／Flutter Tooling、保全・同期確認・回復手順を使う。ユーザー資産・証拠の削除は保護条件の確認なしに行わない。
8. GitHub Actionsは現行ルールどおり**手動`workflow_dispatch`のみ**。Push-trigger CIの新設、意図しないActions発火、失敗状態の無断上書きを行わない。特に以前のWorkflowサイズ制約を再発させない。
9. リポジトリのsingle-writer、直接main運用と既存backup refs（`refs/tags/codex/backup-main`および`...-prev`）の保持・更新規則に従う。他者差分・未コミット変更・証拠を無断破棄しない。Pushは現行規則と許可の範囲で安全な検証後に行う。
10. 未公開秘密をCommit、Trace、Issue、Artifact、画面へ流出させない。公共リポへの非公開HDS／個人情報のコピーを禁止する。

## 5. 進捗記録・報告の強制項目

各`BUILD_BATCH`後の報告には、次を短く記載する。推測で値を埋めない。

```text
Batch ID / 対象要件 / Gate:
Source HEAD（before → after）/ 実行OS・環境:
Producer → Consumer（実動した結果／失敗した箇所）:
権限・Owner操作の境界:
差分の意味（新規実装、再利用、契約／UI／テスト）:
検査と結果（PASS / FAIL / NOT_RUN、fixtureかliveか）:
Evidence（commit SHA、artifact hash、ログまたは文書パス）:
継承した既存CLOSED／回帰の有無:
残件（CODE_PRESENT / WIRED / BUILD_GAP / BLOCKED_AUTHORITY）:
次の必要Consumer／停止または継続の根拠:
```

**判定の区別**：

- `CODE_PRESENT`：コードや定義が存在するだけ。実consumerによる使用は未確認。
- `WIRED`：Producer→Consumerが接続されたことを検査済み。通常実機・配布完成を意味しない。
- `BUILD_GAP`：必要な接続・動作・契約が不足。
- `BLOCKED_AUTHORITY`：資格、ハード、署名、Owner判断など実行権限外の条件が不足。
- `SCOPED_PASS`：OS／候補／fixture有無／機能範囲を限定した有限受入れ。
- `QA_NOT_ESTABLISHED`：正式QAの成立根拠が不足。失敗と未実施は分けて記録。

成果の分類は後から名称変更してもよいが、過去のFAIL・限定PASSを消して履歴上の成功へ書き換えない。独立して証拠を確認できる形で記録する。

## 6. 停止・相談が必要な場合

以下は現物を観測してから、必要最小限の論点でOwnerに判断を求める。

- 既存のOwner権限、Brokerの許可判定、秘密露出、安全側停止を弱めなければ要求が満たせない場合。
- Source・証拠・Git履歴・公開リポ内容の不可逆的削除、保全されていない大規模置換が必要な場合。
- 実資格、署名鍵、外部課金、外部人間への連絡・承認、端末提供、配布や一般公開を要する場合。
- 正本同士の意味が衝突し、独断で片方を捨てると製品要求が変わる場合。
- 安全の成立根拠が欠け、実Tool作用を許すと不適切な外部変更・秘密露出が起こりうる場合。

調査だけで判定できない事項は`UNKNOWN / NOT_OBSERVED`として保存し、不能と断定せず、新規権限を自動取得して回避しない。

## 7. 完了条件・次手

1. まず**W00**を現行Gitの現物で実施し、要求・実producer／consumer・証拠を固定する。
2. 直ちに**W01の最小の実仕事単位**へ進み、通常Release Taskの阻害要因を除き、Owner認可つき実結果を通す。W00の文書作成だけで停止しない。
3. W02以降は工程表の依存順で一つずつ有限の成果を固定し、既存P0〜P13／B0〜B4を保存する。独立問題は条件付きで同一Batchに統合してよい。
4. W08とQ0〜Q7が同一候補に対して成立しても、Owner本人の**明示GOがなければRelease未承認**と報告する。

最終報告では「実製品が何をできるようになったか」「どのOS・Runtime・Consumerで観測したか」「何が未成立か」を最初に述べる。Git操作数・検査件数・提案書作成量だけを成果にしない。
