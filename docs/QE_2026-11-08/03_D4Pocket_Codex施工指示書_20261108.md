# D4 Pocket Codex施工指示書

文書ID：`D4-QE-CODEX-20261108`  
版：v2.1（2026-11-08、五役による施工責任・製品確認の分離）  
対象：`gatchimuchio/D4-POCKET`／`main`  
上位：`D4-QE-SPEC-20261108` ＞ `D4-QE-ROADMAP-20261108` ＞ 本書。`AGENTS.md`の安全・権限・Git・証拠に関する共通基底と、既採用rev5／QE要求を保持する。  
観測固定点：`66c2ed5e40fa0ed277ef372e97cdf9e91c1eedc7`（2026-11-08現在。作業開始時にremoteを再取得）  
効力：2026-11-05採用済み3点と2026-11-06のv1.1改訂の仕事・施工委譲を引き継ぐ**実行命令**。開発指示者から本3点を受領し施工を指示されたCodexは、委任済み工程を実行する。通常の工程開始・画面操作・テスト・Git保全について再承認を求めない。本書は**Codexの開発運用**を定義し、D4 Pocket製品のユーザー向け確認UXは上位製品仕様書に委ねる。

## 0. 最上位命令――何を完成させるか

**D4 Pocketの仕事遂行・能力実行主体はAI／LLM／Agent。D4 Pocketはその身体・能力接続／実行基盤。製品の使用者はユーザー。Codexは製品を開発・検証する施工者。** 「AIが仕事をする」ことと「Codexが製品を施工する」ことを混同しない。

開発時にも設計者・指示者・施工者・使用者・責任者の**五役を分離**する。

| 役割 | 開発運用における意味 |
|---|---|
| **設計者** | 製品仕様・成立条件を設計する。設計作業が委託されても、その結果の妥当性は独立に検証する |
| **指示者** | Codexへ目的・要求・制約・施工範囲を指示する。既に委託した通常操作を毎回再承認する主体ではない |
| **施工者** | **Codex**。指示・仕様に従って実装、通常GUI操作、試験、修正、Git保全、報告を実行する |
| **使用者** | 完成したD4 Pocketを使用する製品ユーザー。開発中のCodexに操作許可を与える役割ではない |
| **責任者** | 対象ごとの設計採否・品質・資産保全・出荷などの最終責任を負う。通常施工の実行担当と同一とは限らない |

指示者・設計者・責任者が一人の人間であっても、判断の対象と権限を統合しない。**施工者への包括指示と、製品使用者がAIへ与える利用許可は、対象も主体も異なる。**

**同名の区別**：開発施工者の`Codex`と、D4 Pocketの実行試験で接続される`Codex CLI`（製品側AI Runtime）は別主体である。後者のTask／Tool／Permissionを前者の開発操作として数えず、前者のフルアクセスを後者の製品利用者Permissionへ移さない。

目的は、AI自身が必要なSoftware／Tool／Service／他Agentを発見・選択・組合せ・使用し、実結果を次の意思決定へ反映し、人間の許可された目的を実際に完遂する製品を成立させること。GUIを作ること、Toolを人間が手動起動できること、ManifestやSchemaを増やすこと、テスト数を増やすことが目的ではない。**AI→実Capability→実結果→AIの判断・継続・完了**を本番の実consumerで閉じる。

旧P／B／BUF／Q要求、既存安全境界、既受入れの有限CLOSEDと過去FAILを継承する。全体をMVPへ縮小しない。W00を作り直したりW01を未着手へ戻したりせず、最初に現HEADを照合して**W02/G2の実働とUIハングの解消**へ進む。

**この3点セットの授受を追加承認の依頼に変換してはならない。** 以下の確認要否は、あくまで**開発施工者Codexに対する開発指示者の確認**に限る。D4 Pocket製品内のユーザー許可・Brokerの権限判定は別の製品仕様であり、Codexの質問要否の分類に混ぜない。

## 1. Codex開発施工の確認・停止条件（最重要）

### 1.1 開発指示者への確認は三分類だけ

| 区分 | 適用条件 | 施工者Codexの挙動 | 開発指示者への確認 |
|---|---|---|---|
| **A：委任済み施工** | W00〜W08および関連する正本整合・編集・GUI／CLI／API操作・検査・原因調査・Git・許可済み手動Actions | **そのまま実行**。フルアクセスは委任の対象を実行する技術手段 | **不要。通常施工の再確認は禁止** |
| **B：外部の必須操作確認** | 現に使うComputer Use／OS／アプリの適用規則が、特定の権限付与操作等へ本人の実行直前確認を明示的に要求 | 該当操作だけ所定の確認を行い、拒否・期限を尊重。他の施工を継続 | **本当に強制された一操作だけ** |
| **C：新しい指示・責任者判断** | 施工範囲・安全規則の実質変更、復元不能な資産消失、本人資格・署名、契約外課金・対外公開、正式Release GO | 技術準備と影響分析まで進め、権限外の確定行為を停止 | **新たな意思決定に必要な部分だけ** |

**D4 Pocket製品内のユーザーPermission／Approvalは、このA〜Cのどれでもない。** それは製品仕様の検証対象である。製品のユーザー役割と開発指示者の権限を代入したり、製品試験の許可表示を「Codexへの施工確認が必要」と解釈したりしない。開発環境のフルアクセスもAの技術手段であって、別の製品利用者の資格・許可を生成しない。

### 1.2 質問の要否を決める手順

毎操作の開始時に、内部で次を**短く判定して実行**する。開発指示者へ逐次報告して判定を待つ作業ではない。

1. 今する作用の`対象／変更／可逆性／外部副作用／必要権限`を特定。
2. **A内の通常施工**なら即実行。GUI画面を開く、フォーカスする、設定を読む、ボタンを押す、コード編集、開発環境の準備、テスト、ログ採取、Git保全、通常push等は原則ここに属する。
3. **Bの外部必須確認**が実際のツール規則・OS仕様・対象アプリの強制により成立する場合に限り、所定の一回の確認を実施。『安全のため念のため聞く』、例示された一般ポリシーの無根拠な拡張解釈は禁止。
4. **Cの新判断**に該当する場合は、必要な新しい指示者・責任者判断だけを求める。既存委任内で解決できる技術問題・通常の実装選択を指示者に丸投げしない。
5. **製品の使用者Permission**が必要な場合は開発施工の確認分類へ入れず、製品Brokerの正規経路で扱い、証拠として記録する。試験に必要な使用者固有の資格・本人操作が得られないときは、その**製品機能試験だけ**を`PRODUCT_AUTH_PENDING`とし、他の開発を続行する。開発施工Aを製品Permissionに置換しない。
6. B／Cが未解決でも、その作用だけ`AWAITING_EXTERNAL_INTERACTIVE_GATE`／`RESPONSIBLE_DECISION_REQUIRED`で停止し、依存しない他のW Batch・検査・改修・証拠採取を継続。

**結果として、確認が発生するのは「本当に本人の操作・意思が追加で必要な特定行為」のみ。** 仕事全体の再承認を兼ねた質問をしない。操作がブロックされたときは、規則の根拠、対象、scope、期限、試行済みの正規手段、独立継続できる作業を記録する。

### 1.3 具体例（曖昧な都度確認を防止）

| 具体的な行為 | 正しい区分と実行 |
|---|---|
| D4 Pocketを起動、設定画面へ移動、検証画面でTaskを入力、UI結果を読む | **A**：Codexが通常GUI操作する。「開いてよいか」「実行してよいか」を聞かない |
| ソース編集、局所ビルド・ローカルテスト、隔離環境の構築、Python／Rust／Flutter Toolingの使用 | **A**：前提・差分・危険境界を監査して自走 |
| Gitの二世代backup tag、検証後commit／direct-main push、既許可のmanual `workflow_dispatch` | **A**：`AGENTS.md`のsingle-writer・保全・安全検査規則どおり実行。pushごとの再承認なし |
| 外部ツールがアプリへのWorkspace Permission発行に、その場本人確認を強制する | **B**：**該当Permission発行の一回のみ**専用確認。これを全GUI操作や全工程の承認へ拡大しない |
| 必須確認なしの普通の画面上の「Allow」「Connect」「Continue」等の操作 | ボタンの名称でBに昇格しない。**開発者側の許可済み作用ならA**としてCodexが進める |
| Tool拒否、Session期限切れ、ログ採取不能、ビルド失敗、UIハング | **技術失敗**。Codexが原因調査・是正・再試験。直ちに開発指示者の承認待ちへ変換しない |
| 責任者本人の実署名鍵・Publisher資格の使用、一般配布の最終確定、製品Release GO | **C**：技術準備はAとして完成し、本人の最終決裁だけを分離 |
| 契約にない公開サーバーへの本番データ送信、予期しない支払い、復旧不能な履歴消失 | **C**：実行前に変更と影響を正確に示して判断を待つ |

**別系統の製品試験**：D4 Pocket製品内の使用者Permission／Task Approval発行はA〜Cに分類しない。Rust Broker・製品native確認の正規経路を通す。施工委任を使用者のApprovalへ代入せず、試験手段の出自と製品内の許可／拒否を記録する。

### 1.4 過去のWorkspace Permission事案を勝手に一般化しない

2026-11-06、過去のCodexは「Computer Use規則上、事前承認済みでもアプリのWorkspace Permission発行直前の確認が必要であり、1回・5分以内の確認対象」と報告した。これは**当時の操作についての報告**であり、全てのWorkspace Permissionや全画面操作の普遍規則ではない。

- まず現在の具体的な操作経路と上位規則の適用を確認する。必要なら**その一発行**に限って規則どおり本人へ照会する。規則の回避のため別の手段へ乗換えない。
- 確認を求めたら、`operation identity（対象App・scope・作用・期限）`と回答・失効の状態を保持。未回答・拒否の同一要求を無意味に繰り返さない。対象・scope・有効期限が実際に変わったときだけ別要求とする。
- 開発ツール側のPermissionが得られない場合、そのツールを使った操作は**未実施**と記録する。製品側Taskの実施有無・許可状態とは混同しない。W02/G2やW01/G1の証拠を捏造せず、他の独立コード・負例・fixture・原因調査・文書整合・Git作業を止めない。
- 外部ポリシーによりどうしてもユーザー介入が必要であれば、回数・時間・対象・しないことを最小限で説明する。確認回数を品質保証とみなさない。

**最終ルール**：再確認を不要にするのは**施工契約内の通常操作**であり、製品Brokerや外部強制規則を解除する指示ではない。外部規則を偽造して質問を増やす行為も、外部規則を無視して質問を減らす行為も禁止する。

## 2. 施工開始時と作業単位の再同期

開始・切替・中断復帰時に、次を読む。**再読は内部の観測であり、開発指示者への再承認ではない。**

1. 最新の開発指示者の指示、root／最寄り`AGENTS.md`、Codex global instructions、安全・証拠・Git規則。
2. `git fetch`後の`origin/main`、local `main`、`HEAD`、`git status --short --branch`、dirty差分、他writer、現在のbackup refs。ユーザー差分・他作業を無断破棄しない。
3. **本v2.1の仕様書→工程表→指示書**、採用記録と進捗、`規定/00_日本語基底規定.md`と`規定/正本索引.json`。2026-11-05 repo版と2026-11-06 v1.1の履歴を、現在の効力と混同しない。
4. rev5原本、2026-10-22採用QE、`BUF-R01〜R09`／`BUF-T01〜T24`、P／B／Q、`docs/REV5_PRODUCT_PROGRESS.md`、`docs/FINAL_QA_QUEUE.md`、`release_blockers.registry.json`、該当Gate。
5. 今のBatchに必要なContract・Schema、Rust Broker、Agent Adapter、MCP／A2A、Flutter、Compose、Export、AI Edit、OS配布、test、実consumerと履歴証拠。
6. **原目的と主語を短く固定**：`AIは何の仕事を完遂するか／必要なCapabilityは何か／実作用・結果を誰が消費するか／製品使用者の許可境界／現OS・source・artifact／次Gate`。未確認・旧事実は区別する。

現時点の固定観測：W00=`SCOPED_PASS`、W01=`CLOSED`（Windows normal Release Codex CLIのみ）、**W02=`OPEN`**、W03〜W08=`PLANNED`、B5=`OPEN`、B6未完了、Q0〜Q7=`DEFERRED`、`release_ready=false`、16 Release Blocker未解決。2026-11-05／06版のW01=`IN_PROGRESS`・Windows通常Releaseの一律`unsupported`は現HEADの現在状態ではない。`66c2ed5`から変更があれば実装・証拠・進捗を照合して更新する。

## 3. 最初に行う施工――正本同期とW02/G2

### 3.1 正本・能力表示の有限整合（新機能を止めない）

1. 新3点を開発指示者から受領した最新版としてRepositoryに取り込み、旧版のSHA・採用時点・証拠を保持したまま、版・採用記録・現行参照を整合化する。正本同士に意味的な衝突があれば、勝手に旧要求を削除せず、責任と影響範囲を特定する。
2. `AGENTS.md`§29・§31、README／QUICKSTART、旧`gui-shell-spec-v1.md`の**AIとGUIの主語**、W01限定`supported`、現在W02 `OPEN`の説明を現在の正しい範囲に直す。履歴内の当時の表記は改竄しない。
3. `tooling/validate_all.py --python-only`の2系統FAILを診断する。日本語基底35 files／239 findingsは意味問題と技術語検出を分け、packaging portabilityでは**Source文書の日本語パス**と**製品配布に収容するパス**を区別して規則を一貫させる。判定器の恣意的無効化は禁止。
4. `release_gate_check.py`が現行N／W・OS・実consumer・証拠を検査できるか照合。未達W02〜W08のまま旧Blockerだけ閉じてGOになり得る穴を、厳格な機械条件で塞ぐ。`release_ready=false`を保持する。

これらを大規模な「文書全面整理」に拡大してW02へ進まないのは不適合。必要な差分を有限Batchで処理し、AI主体の実consumer実装を直ちに進める。

### 3.2 W02-B01：UIハングの実測・原因特定

- `docs/QE_2026-11-05/W02_作業記録_20261107.md`を読み、2026-11-08の候補`w02-20261107-main-a460f57`でMCP接続accepted／`broker-audit-7552〜7554`、その後Windows UIの`Responding=False`となった**観測**を保持する。根因をFlutter・Rust・MCP・Windowsのどれかへ推測で固定しない。
- Source／dirty snapshot／build hash／installed app／profile／fixture／Agent／Toolを同一性として結合。UI thread・Dart isolate・Rust Broker queue・認証IPC・stdio子process・CPU／latency・Auditの時刻相関を採取する。
- 再現条件のある局所修正をし、同じ操作の応答性と終了・cleanupを確認。fake Flutter widget PASSを実製品の正常性の証拠に転用しない。

### 3.3 W02-B02：AI主導のTool実行・帰還経路

1. 既存`native/rust_helper/src/broker/mcp_center.rs`のCatalog snapshot、AgentのTask結果に束縛したJSON要求照合、one-shot Broker Permission、製品使用者のnative確認、Tool stdio、bounded／Zeroizing結果context、後続Task再認可を確認して再利用する。
2. 現行`apps/desktop_flutter/lib/screens/agent_center.dart`の手動提案／後続Taskボタンは**製品使用者による観測・介入の補助手段**と位置付ける。AIが自分で出した能力要求をBrokerへ渡し、結果で次Taskを進める共通Controller／Contractを実producer→consumerへ設ける。UIを常時操作する外部人間がいないとToolが使えない構造を残さない。
3. 連続実行でも**製品使用者のPermission／Approvalは必要なときに必ず分離**し、Tool結果・AI出力から権限を自動生成しない。次Taskへ無条件の旧Approval継承をしない。
4. Toolがない／Catalog失効／引数不正／資格失効／replay／非冪等作用／異常結果／巨大結果／timeout／切断をfail-closedとし、結果・失敗・Auditの相関を壊さない。
5. W01/G1の実Task、拒否、cancel、deadline、process cleanupの既受入れを保持。W02を動かすためにW01の許可を無効化・偽装しない。

### 3.4 W02-B03〜B05：実物受入れ・反証・閉包

- Windows normal Releaseの**同一候補**で、実Codex Task→実Catalog→Agent Tool要求→正規の製品使用者認可→実MCP Tool→未信頼結果→**結果に依存して行動が変わる同Agentの新Task**→成果・Audit・正常終了まで実行。
- 反例は無権限、異常結果、replay、Credential／Scope喪失、取消・期限、故障・再起動、秘密露出・Workspace外作用を扱う。必要な作用が安全に実証できないならその経路を無効化し、G2をOPENで保持。
- `W02_作業記録`の歴史的Rust library全体`540 passed／8 failed／13 ignored`を現HEADの成功として転記しない。Rust／Flutterの現HEAD全試験が未実施なら`NOT_RUN`と記録。現環境で可能な局所試験とWindows実物の正しい証拠を対応させる。
- 同一候補の実結果で**G2が成立した場合だけ**W02を必要な限定範囲でCLOSEDにし、W03へ進む。形式的なハッシュ一致、fixture、2つのUIボタンの個別成功は代用にならない。

## 4. W03〜W08の施工命令

| 工程 | Codexが実装・検証すること | 終了の判定 |
|---|---|---|
| **W03/G3** | 異種の独立Runtimeを同一Task・Capability・Result契約に接続し、実Taskを2種類以上で実施。Agent間の目的・結果・Artifact・diff・検査を引継ぎ、外部A2A実作用を個別試験 | Runtime差異・unsupported、実Handoff／A2A、権限非継承が証拠付きで成立 |
| **W04/G4** | Catalog／Composeの実在能力・依存・製品使用者認可・互換の照合から実Tool作用へ接続 | `manifest_only`と実`wired`を区別。構成結果が元作用を達成、ないCapability拒否 |
| **W05/G5** | 独立App Export、Runtime Manifestの実消費、Module選択・除外、AI Edit提案をCodexが独立審査・テスト・制御適用 | 実installed独立App、Core維持、対象機能・除外・hash／測定が一致。製品Runtimeの自己承認はしない |
| **W06/G6-WIN** | Windowsの通常installer、初回、Setup Doctor、Broker、AI Task、Tool、Audit、Recovery、Update／Rollback／Uninstall、配布identityを同一候補で試験 | 開発用stagedやGUI画面存在でなく製品実機の同一証拠 |
| **W07/G6-OS** | Linux native Desktop、macOS、Android／iOSのOS／実機／署名・配布条件を個別検証。既存portable契約とOS保護を維持 | OSごとに成功／未観測／未対応と根拠を固定。WSL/Xvfb等の局所証拠で代用しない |
| **W08/G7** | B5実干渉調停、B6全体QC、同一候補Q0〜Q7、長時間・安全・性能・資源・更新・復旧、Release Blocker照合 | QC／QAを終え出荷責任者の署名・正式公開・Final GOだけ独立判定。GO前は`release_ready=false` |

W03以降も、既存の実装経路と有限CLOSEDを先に照合して差分施工。技術的に可能な複数言語・層の改修は一つの意味的BUILD_BATCHに束ねる。製品AIとCodex開発者を同じ自己変更主体として混ぜない。

## 5. BUILD_BATCH・検証・Gitの強制規律

1. `AGENTS.md`に沿う`QE_DESIGN → QE_BUILD → SYSTEM_SHAPED → BUILD_COVERED → QE_QC → QE_QA → QE_RELEASE_REVIEW`を保持。小単位のたびの全Rust／Flutter suite・QA・Git回転を強制せず、**意味ある実consumer単位**で閉じる。
2. 実装前に関連Code／Contract／Schema／Test／現証拠を読み、既存Producerを最大限再利用する。デモ専用配線、Mockでの成功捏造、判断器の緩和、既存製品能力の無断削除を禁止。
3. BUILD中の検査は差分のcompile／型／局所正常・拒否・障害／安全境界が最低条件。Authority／Credential／秘密／Audit／Recoveryへ影響する変更は同時点で必要な拒否・失効・cleanupを確認。
4. 検査入口は実Repositoryに存在する正しい引数を確認して選ぶ。代表：

   ```bash
   python tooling/schema_check/check_schemas.py
   python tooling/conformance_tests/run_conformance_skeleton.py
   python tooling/manifest.py --check
   python tooling/release_gate_check.py --strict-release
   python tooling/validate_all.py --python-only
   python -X utf8 tooling/run_cargo_isolated.py test --locked --offline --manifest-path native/rust_helper/Cargo.toml
   python tooling/validate_all.py --flutter-step desktop_flutter_test --desktop-platform windows
   ```

   `python`がない場合は`python3`。`--strict-release`はRelease Blockerが残ればFAILが正しく、BUILD途中でPASSを要求して解除条件を書換えない。Rust／Flutter ToolchainやOSが使えないなら`NOT_RUN`。Windows固有の検証をLinuxの結果で代理しない。
5. OneDrive同期source配下へRust／Flutterのbuild-cache・大容量一時出力を置かず、既存のisolated Toolingを使う。証拠とユーザーデータ・git差分を破壊しない。
6. 本Repoは**single-writer、direct-main**。編集前にfetch・dirty／remote競合・保全範囲を確認。変更を伴うBUILD_BATCHの直前、既存`refs/tags/codex/backup-main`と`...backup-main-prev`を規定順で2世代rotateしてremoteへ保全。検証後のみ限定commit／pushし、`git ls-remote origin refs/heads/main`とlocal HEAD一致を確認。小単位の切れ目には毎回実行しない。
7. GitHub Actionsは必要な検証用の手動`workflow_dispatch`のみ。既存規約の検証branch例外とcleanupを守る。push-trigger CI・PRを新設しない。remote pushは施工委任内の通常作業であり、コミットごとに開発指示者の別承認を求めない。
8. 現行正本／AGENTS／README／進捗／Release Registryの変更は履歴・受領SHA・不変の失敗を保存して行う。既存証拠・秘密・非公開HDS実装を公開リポへコピーしない。

## 6. 状態・確認履歴・結果報告

BUILD_BATCHごとに以下を短く記録する。**開発指示者へ確認を投げた回数を成果指標にしない。**

```text
Batch / W・N要件 / 現在の主語と目的 / 次の実consumer:
Before → After HEAD / OS・Runtime / Source snapshot・Artifact hash:
実Producer → 実Consumer → 実結果（何ができるようになったか）:
開発施工の確認区分（A：委任済み／B：外部必須／C：新判断）と適用した一操作:
製品機能試験の使用者Permission／Approval（該当時のみ、開発確認とは別）:
現物の状態と失敗の根因（観測済み・推論・未観測を分離）:
TEST PASS / FAIL / NOT_RUN、FIXTURE / LIVE_RUNTIME / LIVE_PRODUCT:
旧有限CLOSEDを保存した根拠、未解決Blocker、Rollback／Recovery:
Git backup tag / Commit / Push / Remote HEAD / working tree:
次の安全なBatch、保留があるなら該当する一操作と根拠:
```

運用状態の語義：

- `AUTHORIZED_CONSTRUCTION`：本3点の施工契約内。**確認せず実行する**。
- `AWAITING_EXTERNAL_INTERACTIVE_GATE`：外部規則の**具体的な一操作**について、その場確認が必須と観測された状態。開発指示者の施工再承認でも製品Task完了でもない。
- `RESPONSIBLE_DECISION_REQUIRED`：明示契約外・責任者固有の意思決定が必要。通常Git pushや技術的な試験は含まない。
- `TECHNICAL_FAILURE`：UI停止、ビルドFAIL、Tool異常など。Codexが原因・修正・回帰を担当し、開発指示者に判断を丸投げしない。
- `SCOPED_PASS`：特定source・artifact・OS・Provider・modeで限定受入れ。
- `QA_NOT_ESTABLISHED`：正式QAの根拠が不足。実FAILとNOT_RUNを分けて保持。

製品機能試験側の別状態：`PRODUCT_AUTH_PENDING`はD4製品Brokerで**製品使用者の許可**が未成立という意味だけを持ち、開発指示者の施工確認・外部Computer Useの確認待ちとは混同しない。

確認の重複防止：本人確認が本当に必要だった場合だけ`operation_id`、対象App／Workspace、scope、作用、根拠規則、要求時刻・期限、ユーザー回答、有効／失効を記録。同一未変更の確認を繰り返さない。環境やscopeが変化して新しい資格が必要になった場合は、変わった条件だけを新しい判断として提示。回答を他のPermissionや工程へ使い回さない。

## 7. 停止条件と継続条件

### 7.1 本当に止める範囲

- 安全・認可・秘密・Audit・復旧の境界を弱めないと成立しない作用。
- 未保全のGit履歴・ユーザー資産・証拠の不可逆的消失を伴う変更。
- 未委任の外部公開・本番対外送信・契約外課金、本人署名鍵・Publisher／Production Secret、Release Final GO。
- 現行正本に意味の衝突があり、単なる現況同期でなく**設計者・開発指示者が定めた製品要求自体**を変える必要がある場合。
- 外部Computer Use等の明示規則が特定のPermission発行を本人確認なしでは禁止する場合。
- Product Runtimeの権限がないまま実Tool作用させる場合。製品内の保護は開発指示で解除しない。

各停止は**対象操作・影響領域限定**。観測と明確な根拠がなければ勝手に停止要件を創作しない。権限拒否を受けた経路を別ツール・API・GUIへ切替えて規則を回避しない。

### 7.2 止めずに進めるもの

許可済み編集・既存UI操作・コード調査・原因分析・fixture負例・局所テスト・新Consumer実装・正本整合・既許可のGit保全・許可された手動Actionsは、別作用の本人承認待ちを理由に停止しない。本人でなければ操作できない対象が最後まで未取得なら`BLOCKED_AUTHORITY`とし、依存しない機能を完成させ、該当Gateの未成立を正直に報告する。

## 8. 完了条件と次の直接行動

1. 本3点と現HEAD・採用記録・W00対応表・W01証拠・W02作業記録を照合。新版をRepositoryへ**履歴保全付きで同期**し、正本・AGENTS・進捗・能力表示の齟齬を有限修正。**これ自体も既委譲の施工範囲であり、再承認を求めない。**
2. **W02-B01のハング因果とB02のAI主導MCP共通Contract**へ直ちに着手。通常GUIやテストの一手ごとに確認しない。製品Brokerと外部強制ゲートは別に正しく通す。
3. 実Codex→実Tool→結果依存後続TaskのG2、安全負例、同一candidate証拠を得てW02を限定CLOSEDへ。根因未観測・未実施なら状態を残して、独立施工を継続。
4. W03→W04→W05→W06→W07→W08と、工程表の成立順に自律進行。既存能力の継承・保全・安全・必要なGit閉包を各BUILD_BATCHで満たす。
5. 同一候補の必要QC・QA・Blockerを閉じても、**署名・一般配布・Release Final GOは出荷責任者の独立判断**。これを通常開発施工の再承認と混同しない。

**報告の冒頭は必ず「AIが何をできるようになったか／できなかったか、実結果を誰が消費したか、残る成立条件は何か」とする。** 実装件数・形式的承認回数・Schema数・文書量では成功を表現しない。
