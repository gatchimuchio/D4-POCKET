# D4Pocket Codex QE施工指示書

文書ID：`D4-QE-INST-20261022`　版：`是正施工指示案 2.0`　改訂日：2026-10-22\
状態：**Ownerの再構成・是正指示に基づくCodex引継ぎ用。実施には最新AGENTSの排他・復旧点成立が必要。mainへ未反映。**\
対応仕様：`D4Pocket_01_QE統合実装仕様書_20261022.md`\
対応工程：`D4Pocket_02_QE統合工程表_20261022.md`\
対象：`gatchimuchio/D4-POCKET`。**Linuxは独立したDesktop製品対象OS**として含み、Windows・macOS・Android・iOSと区別する。BLUE-TANUKIとミニドラは参照に限る。\
現行観測固定点：D4Pocket main `2cb330ed6d2e5a0d0014f0bb1d809276ab70cb8f`。旧候補参照点`e3ed5644efc3c350cdc9e8479bd9ece820d04035`（旧BLUE参照`4e32c0cd776afbdeb37dc0c5537ca180981188fa`）は**履歴上の設計基準であり、施工時のHEADではない**。

## 0. 命令の性格と境界

本書は、**QEをD4Pocketに適用する方法**と2026-10-22全数監査で見つかったF01〜F07を是正する施工手順を定める。Ownerの今回の明示指示は再構成・その整合性是正を対象とするが、**単一writerと二世代復旧点が成立していない現時点のmain編集・remote pushを許す意味ではない**。`AGENTS.md`の現行義務と衝突する新ルールは、正式採用・規約同期を確認するまで**提案にすぎない**。本書の提示・過去会話をIssue/PR作成、branch/tagの危険な強制更新、無条件のActions起動、B4の再施工、正式Q0〜Q7開始、Owner GOとみなさない。無断作成Issue #4を要求・承認根拠へ転用しない。

最上位順序：`QE_DESIGN → QE_BUILD → SYSTEM_SHAPED → BUILD_COVERED → QE_QC → QE_QA → QE_RELEASE_REVIEW`。実物を構築する段階と、実物の品質を集中的に評価する段階を分離する。**安全第一・堅牢性第二**を常時維持し、Ownerの許可されない実作用・権限回避を行わない。

実装の主成果は **「元の目的を満たす実際のproducer→consumer接続」** である。チェックリスト・Schema・mock・発行したIssueや報告書の個数ではない。

## 1. 毎タスク開始・切替・復帰の再同期（必ず実施）

実装タスクを小分けするのは目的ドリフト防止のためであり、小単位毎の全suite実行やGit世代回転を増やすためではない。以下を、**小単位の開始ごと、中断から再開するごと、担当・セッション交代ごと**に実行する。

1. 最新Owner指示、適用範囲、停止命令を確認。rootと最寄りの `AGENTS.md`、実際の現行施工指示、仕様・工程、必要な日本語正本を**読み直す**。前回セッションの文章を規範として再利用しない。
2. `origin/main` とlocal `main`、現在HEAD、作業treeのdirty差分、他writerと未完作業を観測する。他者の差分を `reset --hard`／`git clean`／revert／上書きしてはならない。不明なら編集は止める。
3. この小単位の **目的、元要求ID、入力、作用、必要な出力、consumer、期待する観測、対象OS/Host（Linux native／WSL/WSLg／headlessを区別）、危険境界、非対象、停止条件** を短く固定する。元目的を `BUF-T09` のような試験IDへ置換しない。
4. 関係する実コード・production経路・Contract/Schema・局所test・既存のCLOSED証拠を読む。調査だけのsubagentは並列化できるが、編集・正本更新・Git操作のwriterは一つに限定する。
5. 指示された最小差分を進め、途中状態を `CODE_PRESENT`／`WIRED`／`BUILD_GAP`／`BLOCKED_AUTHORITY` と、次のconsumer・残差で記録する。
6. この切れ目だけでは全体QC／QA票／全suite／2世代backup回転／commit/pushを強制しない。次の小単位へ進む前に改めて本§1を行う。

「読み直し」は現行の正本・コードから文脈を再構成する操作であり、モデル内部状態が完全消去されたという保証ではない。作業時間ではなく、接続と実際の状態を成果にする。

## 2. 施工前の唯一の正本照合

参照・照合順：現在のOwner命令と安全境界 → 最新D4 `AGENTS.md`／日本語基底・正本索引 → Ownerが採用した最新版rev5原本3点 → 採用済み緩衝3点・採用記録・今回のQE3点（採用された場合のみ） → 現行`ROADMAP.md`・`docs/REV5_PRODUCT_PROGRESS.md`・`docs/FINAL_QA_QUEUE.md` → 該当Contract/Schema/Conformance/Code/Test。競合時は黙って片方を消さず、責任と適用期間を整理し、Owner採否が必要な方法論差分を記録する。

必ず確認する既存ファイル：`AGENTS.md`、`docs/OPERATING_MODEL.md`、`規定/00_日本語基底規定.md`、`規定/正本索引.json`、`docs/specs/gui-shell-spec-v1.md`、関連する`docs/specs/*`、`specs/*`、`tooling/conformance_tests/*`、`docs/緩衝基盤_rev5補遺_QC/{実装仕様書,工程表,実装指示書,採用記録,B4_製品接続設計}.md`、`release_blockers.registry.json`、既存Q台帳。

**現行再観測（2026-10-22）**：P13/B0〜B3局所CLOSED、B4は2026-10-17のWindows Desktop staged `r2-e2e`限定CLOSED、B5 OPEN（BUF-T04部分観測、T05 OPEN、T06全体OPEN、T23局所PASS）、B6未完了。Q0〜Q7はDEFERRED、通常Release `task_execution=unsupported`／`release_ready=false`。2026-10-12時点の「B4全体OPEN」は歴史的snapshotであり、現在工程へ復元しない。mainを再fetchし状態が更新されたら再判断する。

rev5原本3点の本文は本再構成で独立確認できていない。Codexが原本にアクセスできない場合、完全な置換・逐条整合を主張せず、既存現行正本を保持した限定差分を提案し、不明箇所を報告する。公開repoに非公開HDS／QE原典をコピーしない。

## 3. 文書同期・歴史的B4・Linux継続の責任を分ける

### A：`QE-GOV-SYNC`（現行Owner依頼の文書同期。ただしremote書込みには§11の前提）

**目的**：BLUE-TANUKIのQE工程をD4の方法論として整合させ、旧規定の矛盾を消す。**D4製品機能・Rust/Flutter・B4製品実装へ手を触れない。**

作業範囲：D4 `AGENTS.md` §§3.1/4/19/20/24〜30、`docs/OPERATING_MODEL.md`、`ROADMAP.md`、`docs/REV5_PRODUCT_PROGRESS.md`の将来工程注記、`docs/FINAL_QA_QUEUE.md`の用語注記、`規定/正本索引.json`、`docs/緩衝基盤_rev5補遺_QC/`の3点。新QE3点の同義要求・試験を採用する場合は、既存内容を**差分で**統合し、古いファイルの全文を短縮版で上書きしない。

Aでやること：

1. 既存の各規定・指示を全文検索し、「毎小単位でfull suite／QC合格／QA合格／backup回転／commit-push」と「BUILD_BATCH」という定義の衝突を節・適用範囲・変更後の解釈ごとに台帳化する。
2. **全工程はQE、BUILD中は局所技術確認と安全、BUILD_COVERED後に実物QE_QC、さらに対象別QE_QA**を正本間で一貫させる。`BUF-R08`の全工程品質責任を削除せず、いつ何の品質を検証するかへ正確に分解する。
3. 毎タスクのAGENTS・指示書再読を保持し、小単位の完了票の濫造を防ぐ。関連小単位を `BUILD_BATCH` にまとめて本番consumerの実接続・技術確認・Git閉包を行う。
4. D4のremote**tag** 2世代、local-first/direct-main、**Windows-firstとLinuxを含む独立OS対応**、`workflow_dispatch`手動Actions、実Broker authority、Shell Coreの責任、Owner承認・秘密・監査・復旧を維持する。BLUEのremote**branch**運用を移植しない。
5. `BUF-R01〜R09`、`BUF-T01〜T24`、**採否待ちの詳細案`D4-QE-R13`／`D4-LNX-T01〜T08`**、P/B/Q/FQ各ID・CLOSED・OPEN・UNKNOWN・旧FAIL/PASSを全件保全。Linux native必須要求は現行AGENTSでも成立している。B0〜B4を回帰なしに再施工しない。正式Qの`DEFERRED`を解除しない。
6. 完了時に、**原本→改訂箇所→新規則→保持した安全条件→残差**の対照表を作る。文書lint・日本語基底の関連gate・diff check・必要なSchema参照確認を行い、実際のexitとNOT_RUNを記録する。
7. 現行D4規約で必要な復旧・Git閉包を実施できた場合のみ当該同期単位をCLOSEDと報告。**同期後は停止**。B4実装、Q0、公開へ自動移行しない。

Aの権限はOwnerが明示したファイルと作用に限る。Issue #4、別repo、通常Release Capability、CI追加、実資格、外部公開操作は対象外。

### B：`QEB-D4-B4-PRODUCT`（2026-10-17に限定受入済み。歴史的手順・回帰時だけ参照）

**既存成立範囲**：2026-10-17に`Codex Adapter→Broker→durable Audit→Agent Center`の接続をWindows Desktop `r2-e2e` staged経路で限定受入した。実Codex CLI、Permission/Approval別監査、hash-only結果表示を確認したが、合成localhost API、非Git source未照合、TEMP cleanup未試行、通常installed版未実証は保留される。以下は当時の設計・比較基準として保持し、回帰の実証なしに再実施しない。

Bの入出力を事前固定：

| 項目 | 必ず照合する対象 |
|---|---|
| 元要求 | Agent/Runtime/Session/Workspace・実行条件・Task目的。ユーザー指示hashと品質試験IDを混同しない |
| 認可 | 本物のBroker管理する現在Permission/Approval。UI・緩衝記録・Adapter metadataは非権限 |
| 作用 | `WorkspaceTaskScratch`による型付きTEMP/TMP媒介と元Codex CLI作用。無断のpath/credential/lockfile操作をしない |
| 結果 | 元Toolの終了状態、Task終端、cleanup、結果hash、Audit ID、Agent Centerのhash-only公開projection |
| 否定例 | Workspaceすり替え、session/Host/指示hash不一致、権限不足、監査保存不能、secret/Content Exposure逸脱、未監督process |
| 証拠区分 | Fixture、Rust/Flutter局所、実Codex CLI、Desktop通常IPC、installed productを混同しない |
| 残差 | `task_execution=unsupported`、`release_ready=false`、非Gitの吸収実証条件、元Task成功条件との分離を維持 |

**過去B初回の手順**：当時のHEAD／B4製品接続設計と関連symbolを照合し、未接続consumerを限定して差分施工した。現在の次作業ではない。現在の`B5_自己適用.md`を読んでB5の未成立条件を選び、Windows B4を再構築しない。

B4のCLOSEDは単一製品経路の該当staged Task、限定拒否、結果本文非露出・Auditが実測できた**観測射程**でのみ主張する。TEMP cleanupは未試行につき成功へ昇格しない。これだけでSYSTEM_SHAPED、BUILD_COVERED、B5、全QC、通常Releaseを自動CLOSEDにしない。

### C：`QEB-D4-LNX-01〜03`（Linux native製品対応。詳細IDは未採用、現行施工は継続可能）

**目的**：Linuxは現行AGENTSですでに必須独立Desktop対象OS。現行Linux施工は本候補の追加詳細IDが未採用でも継続できる。2026-10-22 Linux installed MCP synthetic X11（GitHub手動Action #107）は限定PASSだが、**Linux nativeの全UI/Broker/Owner/Agent Taskや通常配布/Releaseの完成証拠ではない**。P13・Windows B4のCLOSEDを変えない。

1. **環境固定**：指定されたLinux distro/version、kernel、arch、libc、filesystem、display server（X11／Wayland）、実行Host、source commit、Toolchain/依存lockを記録。`LINUX_NATIVE`、`LINUX_WSL_DEV`、`LINUX_WSLG_X11`、`LINUX_HEADLESS`を分け、同一扱いしない（`D4-LNX-T01`）。未指定のOS対象は勝手に正式サポートへ拡大しない。
2. **BATCH-01：実consumer**：Linux nativeのFlutter Desktop→現在のOwner操作→認証IPC→Broker→Adapter/Runtime→実Agent Task→Tool結果／Audit／Agent Centerを現物確認。未接続なら`BUILD_GAP`、権限不足なら`BLOCKED_AUTHORITY`。通常Linux Owner/IPC経路を捏造せず、既存安全契約の意味を維持（`D4-LNX-T02／D4-LNX-T03／D4-LNX-T05`）。
3. **BATCH-02：OS作用**：Linuxでprocess子孫監督・cancel/deadline/crash・Recovery・scratch cleanup、Workspaceのsymlink/hardlink・secret非露出、資格保管・IPC・権限拒否、既存Toolの実作用を確認。Windows Job ObjectのPASSを流用せず、Linux実装上の監督方法を設計・検証する（`D4-LNX-T03／D4-LNX-T04／D4-LNX-T07`）。
4. **BATCH-02／03：配布・母集団**：Linux用成果物をsource/hashで結び、build/package/install/first-run/update/rollback/uninstallの導線とOwner操作を区別して確認。対象GUI（X11・Wayland）の一方を通しただけで他方の合格にしない。全既存Runtime/Host/Workspace/緩衝consumerについて要求・実装・未接続・拒否を全件対応（`D4-LNX-T05／D4-LNX-T06／D4-LNX-T07`）。
5. **後段QC**：同条件のLinux native介入前後、正常・異常・8h/24h・OS間の影響と資源消費を検証する（`D4-LNX-T08`）。Linux nativeの実際の製品品質とLinux WSL/WSLgの開発確認を別欄に記録し、対象別QAは証拠が揃うまで`QA_NOT_ESTABLISHED`とする。
6. **権限と工程境界**：このC系列の**追加ID・詳細判定は未採用案**であり、現行のLinux native必須施工を停止させるものではない。既存Owner委任・AGENTS・進行中Linuxブランチの作用範囲で続行し、変更追加には権限とsingle-writerを確認。Windows-first、B4-Windows、既存P13/CLOSED、2世代backup remote tag、手動Actionsと安全・Release gateを維持。Linux限定PASSを全体合格にせず、B5／B6／正式Qへ自動移行しない。

## 4. BUILD_BATCH運転アルゴリズム（採用済みQE規約かつ明示施工範囲のみ）

1. **BATCH定義**：実際に接続する一つの仕事、ownerの元目的、入出力、対象ファイル、許された作用、安全境界、次consumer、受入条件を定める。小単位に分けるが成果物は実接続。
2. **現在状態の確認**：AGENTS、指示、最新origin/main、local HEAD/dirty差分と他writerを再読。CLOSED証拠の無意味な再試験をしない。
3. **復旧点の保全**：許可された編集の前に、現行D4規約どおり2世代 `refs/tags/codex/backup-main[-prev]` を保存。予定/実際SHAとrefsの種類を照合する。上書きに競合があれば停止し、forceで他者を消さない。
4. **小単位の実装**：最小コード・契約変更を行い、毎小単位で§1再読。現在のconsumerを本当に進める。新しいwrapper/抽象型/環境迂回を根拠なく増やさない。
5. **危険変更時の即時確認**：Authority、Approval、secret、Content Exposure、Audit/Recovery、Process監督、OS/Workspace境界、非冪等作用に触れたら即座に関係する否定試験・fail-closedを実行。不成立なら危険なconsumerはSUSPEND/disableし、続行しない。
6. **バッチ末の技術確認**：変更対象のコンパイル・型、必要build、接続部分の限定実smoke、関係するSchema/Conformance・回帰・秘密非露出、`git diff --check`、日本語基底・文書gate。**Linux対象のバッチではLinux nativeを基準にした実測と、WSL/WSLg/headlessの証拠を別々に記録。**実施・FAIL・NOT_RUNを厳密記録。全面QC・24h・全OS・QA判定は対象実物が揃うまで開始しない。
7. **Git閉包**：許可済みの正確な差分だけstage。private HDS、資格、運転状態、raw証拠、生成物を公開しない。direct-mainの限定commit/push、remote HEADと2世代tag ref/hash、clean状態とrollback pointを照合。D4の手動Actions検証が必要なら承認済み`workflow_dispatch`手順に限る。
8. **報告・次consumer**：何が実際に動いたか、未接続、危険により停止、使った証拠階層、限定確認とNOT_RUN、次の接続先、Git保全を報告する。小単位ごとのQA合格票を成果にしない。

**注意：** 方法論候補の採用前は、現行D4 `AGENTS.md` §19の必須`cargo test`／Flutter analyze／Schema／Conformance、§20の作業単位Git閉包等を無言で省略しない。新しい工程で検査強度を再配置するにはOwnerがQE規律を正式採用していることと、現行契約・release gateの安全条件を保持したことが必要。

## 5. 検査と証拠の必須ルール

- **QE_BUILD**：構文・型・対象test・必要build・限定LIVE_RUNTIME・重大安全拒否だけを作業に応じて実施。全Rust/Flutter suiteや全数Conformanceを毎小単位に課さないが、**採用後も**重大リスクと現行正式gateを理由なく省略しない。
- **BUILD_COVERED**：既存製品要求と`BUF-R01〜R09`、**Linux追加`D4-QE-R13`／`D4-LNX-T01〜T08`**の全件について、実consumerがあるか、何が未接続か、`BLOCKED_AUTHORITY`かをOS別に全数照合。実際に欠落する必須能力を一つでも隠せば不成立。
- **QE_QC**：B5/B6、許可された正式Q0〜Q5で`BUF-T01〜T24`に加え**Linux native `D4-LNX-T01〜T08`**を含む包括的検査。元不具合の再現、介入なし／ありの同条件比較、別境界への問題移転、資源と長時間、意味同等性、安全、正常既存機能の回帰を確認。不具合があれば設計まで必要な箇所を戻す。
- **QE_QA**：Q6の同一候補証拠に基づき対象・OS（Linuxはdistro/kernel/GUI/backendを限定）・版・条件だけを保証判定。WSL/WSLgのPASSだけでLinux native/installedを保証しない。`QA_SCOPED`／`QA_NOT_ESTABLISHED`／`OUT_OF_SCOPE` を使い、未検証を保証しない。
- **QE_RELEASE_REVIEW**：Q7でOwnerがPublisher/署名/Audit key/不可逆公開GOを判断。Codex・LLM・UI・履歴からOwner承認を生成しない。

**緩衝基盤特有の判定禁止事項**：未観測を「吸収不能」や「吸収成功」としない。B3の153行分類を全件効果測定と読まない。`FQ-TEST-LOOPBACK`を単独retry成功だけで閉じない。source commit未取得の現行v2 recordを `吸収成立` にしない。`BUF-T09`などの品質試験IDを元Agent Taskの目的達成条件にしない。test result／同等性／権限条件／失敗logを改変して成功扱いしない。

## 6. 既存Q0〜Q7との接続・再凍結

現行 `docs/FINAL_QA_QUEUE.md` の互換識別子、FQ項目、2026-10-07の旧Q0/Q1結果、延期Q2、待機Q3〜Q7はそのまま保存する。正式Q stageはOwnerの明示した現行mode・対象candidate・開始条件が揃うまで`DEFERRED`のまま。B5/B6の緩衝QCが実施できても、正式Qへの自動切替にはならない。

正式QCで製品コードが変わったら、古い候補のPASS/FAILと変更前hashを保持し、新たなcandidateをQ0で再凍結。**Linux nativeはOS・distro・GUI・配布版が一致する別証拠として管理**し、Windows・macOS・WSLgの成功を転用しない。必要な関係Q1〜Q5を新candidateへ行い、Q6で整合した証拠を束ね、**別の対象別QA判断を経て**Q7のOwner最終判断へ渡す。`task_execution=unsupported`と`release_ready=false`を勝手に解除しない。

## 7. 停止・権限の条件

次の場合、該当する実作用は開始または継続しない。安全な読取り・現状整理と明確な報告だけに限定する。

- 明示的なOwner委任がない／指示の対象repo・ファイル・作用が不明／Ownerが停止している。
- 現行rev5・AGENTS・QE案の衝突があり、Owner採否が必要な場合。
- remote HEAD・local HEAD・dirty state・他writerに未解消の競合がある場合。既存変更の破壊回復を禁止する。
- 現在Permission/Approvalが不足し、OS保護・Owner確認・Credential・Content Exposureを迂回しなければ実行できない場合。
- Auditが成立しない、監督なしprocess、機密露出、未認可外部実作用、未確認の不可逆変更が発生する場合。
- 必須の技術検査・Git復旧点・remote照合が不成立で、完了と報告できない場合。

B4製品実装で安全上有効化できない経路は `BLOCKED_AUTHORITY` として残し、危険な代替経路を作らない。止まった作業の上位目的を忘れず、元の指示へ戻って再同期する。

## 8. Codexの報告形式（実際に行った範囲のみ）

```text
QE_STAGE：
OWNER指示・対象/非対象：
BUILD_BATCH ID／小単位ID：
現行 main/remote HEAD・dirty/他writer：
元目的／要求ID／次consumer：
接続前 → 接続後の実経路：
CODE_PRESENT／WIRED／BUILD_GAP／BLOCKED_AUTHORITY：
変更したContract／Code／Test／Docsと安全境界：
実行した最小技術確認（command, OS/distro/kernel/GUI、Linux native・WSL・WSLg・headlessの区分, exit, evidence class）：
FAIL／NOT_RUN／原因UNKNOWN：
緩衝介入の元Tool結果・意味同等性・独立照合（ある場合だけ）：
Git（許可済みの変更を行った場合）：commit／push／remote SHA／refs/tags/backup-main[-prev]／rollback point：
保持した既存CLOSED、release gates、正式Qの状態：
次のconsumer・必要な別Owner判断：
```

実物QCならさらに対象candidate、同条件比較、根因確度、修正・再試験、性能・資源・副作用・下流転移を付す。QAなら対象・版・OS・条件と`QA_SCOPED/QA_NOT_ESTABLISHED/OUT_OF_SCOPE`、Owner公開判断とは別である旨を記録する。

## 9. 適用前後の完了判定

**この3点セットの採用**は、文書の要求ID／試験ID／工程状態とrev5原本の差分がOwnerに提示され、規範の衝突が解消したときだけ「統合規則の採用」と呼ぶ。単に3つのMarkdownを書き換えて完了ではない。`QE-GOV-SYNC`は実施が明示された場合に限り、変更先のAGENTS/運用/緩衝補遺/進捗/QA queueの規範が同じQE意味で読めるようにし、差分検証後そこで止める。

**B4以後の完成**は別の仕事である。次のCodexタスクを自動発行しない。実際の製品動作・品質・保証・公開を3点セットの存在だけで成立済みとしてはならない。

## 10. 操作・要求の索引

- QE方法論：`D4-QE-R01〜R13`（仕様書§3）。
- 緩衝基盤要求：`BUF-R01〜R09`（仕様書§5）。
- 緩衝試験：`BUF-T01〜T24`（仕様書§6）。
- **Linux追加試験案**：`D4-LNX-T01〜T08`（仕様書§6.1）。Linux nativeは現行正本で必須の独立対象OS。`QEB-D4-LNX-01〜03`の詳細工程ID採否とは独立して、現行AGENTS/Owner権限内のLinux BUILDを継続し、OS別QC/QA証拠を管理する。
- 既存局所開発：P0〜P13、B0〜B6（工程表§1・§4）。
- 既存正式検査：Q0〜Q7、`FQ-*`（工程表§5、現行 `docs/FINAL_QA_QUEUE.md`）。
- Authority：D4現行Broker、Adapter、Workspace、Host、Approval、Audit、Recovery、Content Exposureの責任正本を保持。
- 旧Issue #4は設計・許可・受入れの根拠外。

**改訂履歴（1.1）**：Linuxを独立製品対象OSとして指示書へ明記。Linux native・WSL/WSLg/headlessの検証分離、`QEB-D4-LNX-01〜03`、`D4-QE-R13`、`D4-LNX-T01〜T08`とOS別QC/QAを追加。既存のA/B委任、完了履歴、安全・Git tag・release条件は維持する。

## 11. 今回の是正施工命令 `D4-AUD-20261022`（F01〜F07）

**開始時の必須再読**：上記§1に加えて、今回の仕様書§12、工程表§8、2026-10-22全数監査報告、最新main `AGENTS.md`全文／`docs/REV5_PRODUCT_PROGRESS.md`／`docs/緩衝基盤_rev5補遺_QC/B4_製品接続設計.md`／`B5_自己適用.md`／`docs/FINAL_QA_QUEUE.md`／`規定/正本索引.json`を再確認。指示の原文と今回の固定SHAを意味の正本から取り違えず、施工時最新SHAで再観測する。

### `D4-AUD-G0`：Git書込み前に停止条件を解消

1. remote/mainとlocal/mainのHEAD、dirty差分、3 remote `codex/*` branchの最新HEAD／未統合diff／担当writerを記録する。2026-10-22の監査ではmain `2cb330ed6d2e5a0d0014f0bb1d809276ab70cb8f`。Linux branch `codex/linux-installed-workspace-acceptance`は監査後にも前進（観測SHA `9659fa63e1b499723635ee4754c56bc930ba6ba2`）。他者の変更をreset・checkout・上書き・rebase・deleteしない。
2. 二世代remote tag `refs/tags/codex/backup-main`／`refs/tags/codex/backup-main-prev`のSHAと実際の復元対象・前回rotation履歴を確認。監査時は両tagがmainと同じSHAを指しており、2世代になっていない。単一writerが明確になり、履歴を壊さない異なる復旧対象が識別できるまでは**mainへの編集／push、tagの上書きを停止**する。強制更新・破壊的整備が必要な場合はOwnerの対象SHA込みの明示判断を求める。
3. 未解除なら調査・この3文書の提出・パッチ検証計画までは進め、状態を`BLOCKED_AUTHORITY / NOT_PUSHED`として停止する。Codexが作業を開始した・commitした・GitHubへ反映したという虚偽報告を禁止。

### `D4-AUD-F01`：C32監査器修正（G0成立後の最優先コード是正）

**不具合の生成原因**：`tooling/final_development_audit.py`の`_relative_path(value)`は`ROOT / value.replace("/", "\\")`を返す。POSIX上はbackslashがdirectory separatorではなくfilenameの一部となり、存在する`docs/specs/...`等256参照が不在と判定される。

**実装指示**：参照値をplatformに依らないrepository-relative pathへ正規化する。入力としてPOSIX区切りを受け、既存値にWindows区切りがある場合は検証の上で正規化する。`..`、absolute、drive-qualified、空欄・root外への解決を拒否し、単一の参照不正は監査結果へFAILとして加える（監査器の異常終了を避ける）。root下のsymlinkがroot外を指す場合の参照も拒否する。プログラムの`release_ready=false`保持、監査項目数・参照母集団・既存負例・CLOSED実績を変更しない。

**試験と採否**：Linux/POSIXで元の256誤不在が解消すること。実在／不在、`../`、`/etc/...`、`C:\\...`、backslash表記、root外symlinkの負例を追加。存在検査・異常行検出ともEXITに対応させ、Windowsでも従来正しい参照がPASSし、実際のmissing-referenceはFAILのままであること。導入環境にRust/Flutterがない場合、その試験は`NOT_RUN`であり通過済みにしない。

### `D4-AUD-F02〜F07`：独立・有限の是正単位

| 単位 | 作業命令 | 受入証拠と禁止事項 |
|---|---|---|
| F02復旧点 | G0で安全な2世代tag復元先・rotation履歴を明示し、単一writer／承認範囲でrefsを是正 | remoteが指す異なる復旧可能なSHAを読取り再確認。無確認force更新・過去点破壊は禁止 |
| F03文書同期 | README、CLAIM、RELEASE_CHECKLIST、COMPATIBILITY_MATRIX、MOBILE_STATUS等の記述を日時・commit・環境・結果ごとに照合 | Linux手動Action #107はX11 synthetic installed MCPの限定PASS。Windows B4 staged CLOSED／B5 OPEN／Linux全体未受入を同時表現。古い履歴を消さない |
| F04言語 | 厳格audit検出33ファイル・220行の個別判定と修正 | 違反と許容識別子を根拠付き分類、実違反だけ直す。全検査の元データと差分を残し、安全・wire契約の翻訳誤りを作らない |
| F05branch | 3 remote codex branchのowner・diff・取込み／進行を実際に確認 | Linux進行中 branchを勝手にdeleteしない。回収対象は統合と無用性を証明してから最小操作 |
| F06索引 | `規定/正本索引.json`の`docs/specs/mcp-contract.md`重複が同義か確認して整理 | 109参照／108実対象の意味を監査し、残った全参照の存在・所有責任を検査。必要な二重役割なら削除を強制しない |
| F07rev5 | `採用記録.md`中SHAと照合したrev5原本3点を取得して要件全数差分を作成 | 原本未取得なら`UNVERIFIED_ORIGINAL`。3点を単なる新規短縮書で上書きせず、Ownerに採否材料を提出 |

### 付帯のB5とLinuxの現行判断

- **B4**：2026-10-17 Windows Desktop staged統合受入でCLOSED。再現可能なregressionが出なければB4コードには触れない。`作用後未照合`・TEMP cleanup未試行・installed未実証を保存。
- **B5**：OPEN。既存のWindows対象と`BUF-T04`部分観測・`T05`OPEN・`T06`全体OPEN・`T23`局所PASSを再読。過去の受入済み局所結果を再実施の名で上書きせず、残る根因・安全境界を最小単位で進める。pktmonのformatted eventはpacket数ではない。
- **Linux**：2026-10-22 Action #107のXvfb/X11 synthetic installed MCP限定PASSと他Linux未検証項目を別管理。現行Linux workを詳細QE候補の未採用を理由に止めない。反対に既存branchへの同時書込みはしない。Wayland、一般配布、実資格・native全体は未受入。
- **Release**：16件の現行未閉包を元台帳で個別保全し、`FQ-TEST-LOOPBACK`ほか既知FAIL、Q0〜Q7 `DEFERRED`、`task_execution=unsupported`、`release_ready=false`、Owner GOなしを維持する。

### `D4-AUD`の停止・報告フォーム

```text
参照したOwner原文／AGENTS revision／現行SHA／3正本の版:
Writer確認・dirty・3 branch・backup tag 2世代のSHA／復旧可能性:
G0: CLEAR / BLOCKED_AUTHORITY（理由と必要な承認）
F01〜F07各件: NOT_STARTED / IN_PROGRESS / VERIFIED / BLOCKED（実際の証拠）
B4 CLOSEDを保持した証拠、B5 OPEN、Linux OS別状態:
変更前後の意味・機構・実consumer、負例、未観測:
テスト: command・OS/Host/source commit・exit・件数・NOT_RUN:
GitHub: NOT_PUSHED / commit SHA＋main/remote SHA／backup refs／Action run:
追加で必要なOwner判断（対象・目的・リスク・戻し方）:
```

**完了宣言の基準**：G0と関係F単位が検査済み、remoteの変更・根拠・復旧点の対応が実測された単位だけ`VERIFIED`。本3文書を作成した事実は施工完了ではない。正式rev5の逐条同等性は原本未取得なら`UNVERIFIED_ORIGINAL`のまま。

**改訂履歴（2.0）**：1.1のQE・BUF・独立Linux設計を保持し、B4 CLOSED/B5 OPEN、G0、F01〜F07の施工命令、安全なGit停止・根拠境界・Codex報告規格を追加した。
