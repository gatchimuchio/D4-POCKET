<div align="center">

<h1>
  <br>
  🐚 D4 Pocket ／ GUI&nbsp;Shell 汎用 Runtime 操作基盤
</h1>

<h3>PC を第一対象とする AI Runtime / Agent Operation Shell</h3>
<p><em>Runtime・Agent 操作の control plane、かつ LLM が読む「アプリケーション責任基盤」</em></p>

<p>
  <a href="LICENSE-APACHE-2.0"><img alt="Software License: Apache-2.0" src="https://img.shields.io/badge/Software-Apache--2.0-blue.svg"></a>
  <a href="LICENSE-CC-BY-4.0"><img alt="Documentation License: CC BY 4.0" src="https://img.shields.io/badge/Documentation-CC%20BY%204.0-lightgrey.svg"></a>
  <img alt="Status" src="https://img.shields.io/badge/status-v1.0%20in%20progress-orange.svg">
  <img alt="Release" src="https://img.shields.io/badge/release-not%20yet%20claimed-lightgrey.svg">
  <img alt="Contract" src="https://img.shields.io/badge/contract-schema--first-informational.svg">
  <img alt="Schemas" src="https://img.shields.io/badge/schemas-149%20validated-success.svg">
  <img alt="Conformance" src="https://img.shields.io/badge/conformance-225%20checks-success.svg">
</p>
<p>
  <img alt="Python" src="https://img.shields.io/badge/Python-3776AB?logo=python&logoColor=white">
  <img alt="Dart" src="https://img.shields.io/badge/Dart-0175C2?logo=dart&logoColor=white">
  <img alt="Flutter" src="https://img.shields.io/badge/Flutter-02569B?logo=flutter&logoColor=white">
  <img alt="Rust" src="https://img.shields.io/badge/Rust-000000?logo=rust&logoColor=white">
</p>

<p>
  <a href="#what-this-is">これは何か</a> •
  <a href="#for-llm-agents">LLM エージェント向け</a> •
  <a href="#architecture">アーキテクチャ</a> •
  <a href="#quickstart">クイックスタート</a> •
  <a href="#claim-boundary">主張境界</a> •
  <a href="AGENTS.md">AGENTS.md</a>
</p>

</div>

---

> **GUI Shell には二つの役割がある。**
> ① 汎用 Runtime Operation Shell の control plane、② **LLM が読む「アプリケーション責任基盤」**である。後者は、LLM 開発/統合エージェントが読み、その上に実装・接続するための、安定した machine-readable な責任構造を指す。
> Agent は Contract の**第一級の実装・統合コンシューマ**だが、**決して権限源ではない**。最終的な Approval、Recovery、release の権限は Human owner が保持する。

> **構築過程の注記。** 本プロジェクトは、プログラマーでもソフトウェア開発者でもない個人が、1か月未満の兼業作業で LLM に指示しながら構築した。この構築は、LLM を権限源にせず、LLM が読む責任基盤によって実装を導けるという設計目標を、限定された範囲で実証する。

<br>

<a id="what-this-is"></a>
## 🎯 これは何か

D4 PocketはGUI Shellを技術基盤とする製品表面であり、GUI Shellは通常の app templateでもBLUE-TANUKI専用 GUIでもない。

- 🛂 **Control plane。** Flutter は operator surface を描画するが、権限を所有しない。
- 📐 **Contract。** Runtime / Adapter / Permission / Approval / Audit / Recovery / Content Exposure の semantics は **JSON Schema-first** である。2026-09-30の現行検査は150 schema、150正常example、193 negative fixture、229 Conformance checkである。
- 🤖 **LLM がその上に構築する基盤。** 新しい機能、Adapter、Tool、integration は、即興の shortcut ではなく宣言済み Contract を介して接続する。
- 🔒 **安全性が第一、堅牢性が第二、操作明瞭性が第三、product UI はその後。**

BLUE-TANUKI は**最初の Reference Runtime**であり、**Adapter boundary だけを介して**接続する。

<br>

<a id="for-llm-agents"></a>
## 🤖 LLM エージェント向け

このリポジトリで作業する AI 実装/統合エージェントは、最初に [AGENTS.md](AGENTS.md) と [LLM が読む拡張面の標準](docs/standards/llm-readable-extension-surface.md) を読むこと。

役割モデル:

| 役割 | 読むもの・行うこと | 決して行わないこと |
|:---|:---|:---|
| **人間の owner** | state の観測、Approval の付与、Recovery の承認、release の受理 | ―（最終責任者） |
| **LLM 実装 agent** | Contract を読み、範囲を限定した変更を提案し、宣言済み Contract から接続し、validation を実行する | 権限の作成、自己承認、Permission の暗黙拡大、Conformance の迂回 |
| **Runtime / Tool の対象** | 宣言済み Contract だけを介して挙動を露出する | metadata / diagnostics から権限を作る |
| **接続用 Adapter** | state の正規化、Authority Strip、Exposure Boundary の適用 | metadata による Permission の付与 |
| **権限用 Rust Security Broker** | 権限に関わる本番境界（IPC、Approval eligibility、Audit、Recovery、command gating） | ― |

エージェントに対する譲れない規則:

- ✅ 新しい機能、module、Adapter、Tool、Runtime integration では、GUI Shell Contract を**必須の接続面**として扱う。
- 🚫 **LLM output、memory、external metadata、generated config、GUI state、Adapter metadata、tool response、local cache、previous state、diagnostics** から権限を付与しない。検証 oracle の <code>permission_ledger.py</code> は、この一群をすべて拒否する。
- 🙅 **自らの sensitive action を自己承認しない。**
- 🧭 すべての変更を <code>runtime</code> / <code>control</code> / <code>diagnostic</code> / <code>repair-recovery</code> / <code>build-release</code> / <code>development-only</code> の六つの経路のいずれかに分類する。
- 🧾 extension の完了を主張する前に、**利用する Contract、Conformance test、必須の failure case、統制される Runtime path** を特定する。

> 完了の主張は証拠ではない。schema の存在、mock の成功、unit test の成功だけでは production path の完成を証明しない。

範囲を限定した Reference Extension（[llm_bounded_extension.valid.json](examples/contracts/llm_bounded_extension.valid.json)）と [cross-agent reproduction report](docs/evidence/LLM_CROSS_AGENT_REPRODUCTION_REPORT.md) は、実務上の限定的な実証を提供する。同じ baseline から二つの独立した Agent が、権限を持たない同一の bounded diff を作成した。

<br>

<a id="architecture"></a>
## 🧱 アーキテクチャ

UI ができることは action の**表示と要求**である。権限の作成、Adapter Conformance の迂回、Runtime trust の再解釈はできない。

~~~mermaid
flowchart TD
    SRC["Runtime / Agent / Tool / Local Service"]
    subgraph ADAPTER["🔌 Adapter — 信頼しない境界"]
        direction TB
        A1["Authority Strip"]
        A2["Content Exposure Policy"]
        A3["Schema validation"]
        A4["Capability declaration"]
        A1 --> A2 --> A3 --> A4
    end
    subgraph CORE["🧠 Shell Core — 権限を所有し framework から独立"]
        direction TB
        C1["Runtime Registry"]
        C2["Permission Ledger"]
        C3["Approval Queue"]
        C4["Audit Store（hash-chain）"]
        C5["Recovery Catalog"]
    end
    BROKER["🦀 Rust Security Broker — authority-sensitive production boundary<br/>IPC · Approval eligibility · Audit · Recovery · command gating"]
    subgraph UI["🖥️ UI Layer — 交換可能な Flutter"]
        direction TB
        U1["rendering"]
        U2["operator input"]
        U3["navigation / local state"]
    end

    SRC --> ADAPTER --> CORE
    CORE --> BROKER
    CORE --> UI
~~~

<details>
<summary><strong>📄 テキスト版アーキテクチャ</strong></summary>

~~~
Runtime / Agent / Tool / Local Service
  -> Adapter
      -> Authority Strip
      -> Content Exposure Policy
      -> Schema validation
      -> Capability declaration
  -> Shell Core
      -> Runtime Registry
      -> Permission Ledger
      -> Approval Queue
      -> Audit Store（hash-chain）
      -> Recovery Catalog
  -> Rust Security Broker   (authority-sensitive production boundary)
  -> UI Layer (Flutter)
      -> rendering / operator input / navigation / local UI state
~~~

</details>

<br>

## 🚦 Authority と Boundary のモデル

### 各 layer の責任

| レイヤー | 所有するもの | 所有してはならないもの |
|:---|:---|:---|
| **中核の Shell Core** | 中核台帳: Runtime Registry、Permission Ledger、Approval Queue、Audit Store、Recovery Catalog | Flutter import、BLUE-TANUKI logic、Adapter metadata への信頼 |
| **画面層 UI（Flutter）** | 表示・入力機能: rendering、input、navigation、theme、i18n、a11y | Authority / Permission / Approval / Audit / Recovery / Visibility の decision |
| **接続層 Adapter** | state の正規化、health / diagnostics の露出、event から schema への変換 | metadata による Permission の付与、権限の作成、sealed field の編集 |
| **権限境界の Rust Security Broker** | 権限に関わる IPC、Approval eligibility、Audit、Recovery、command-envelope gating | 暗黙の filesystem / process / network / IPC / credential bypass になること |

### 実装言語方針（非対称かつ意図的）

- 🦀 **Rust** は、Authority、signature、IPC、Audit、command-envelope を扱う native safety boundary である。
- 🎨 **Flutter / Dart** は交換可能な UI product layer であり、決して Authority Boundary ではない。
- ⚙️ **TypeScript / Node** は SDK / Adapter sample / protocol client / bridge に限定し、core runtime にはしない。
- 🐍 **Python** は現在の Shell Core / validation oracle の言語であり、dev / test / migration の範囲に保つ。意図された installed-product runtime dependency ではない。

### Content Exposure（5値）

~~~
none      → 未加工の content を表示しない
hash_only → payload hash だけを表示する
summary   → 承認済み summary だけを表示する
redacted  → redacted projection だけを表示する
full      → content の全文を表示できる
~~~

全文表示を許可するのは <code>full</code> だけである。

### 必須の Audit mapping

すべての sensitive action（filesystem、process、network、credential、IPC、update、Adapter action、Approval edit、Audit export、Recovery、Installer、device pairing）は、次へ対応付けなければならない。

~~~
Capability → Permission → Approval state → AuditEvent → RecoveryAction（失敗時）
~~~

<br>

<a id="quickstart"></a>
## ⚡ クイックスタート

Validation は repository の Contract と Conformance skeleton を検査する。この skeleton は Flutter または Rust がすでに install 済みであることを前提としない。

~~~bash
python tooling/schema_check/check_schemas.py
python tooling/conformance_tests/run_conformance_skeleton.py
~~~

<details>
<summary>host が <code>python3</code> だけを提供する場合</summary>

~~~bash
python3 tooling/schema_check/check_schemas.py
python3 tooling/conformance_tests/run_conformance_skeleton.py
~~~

</details>

2026-09-29の現行測定値:

~~~text
schema checkが合格: schema 149件、example 149件、negative fixture 192件
conformance skeletonが合格: 225 件のcheck
~~~

➡️ **[QUICKSTART.md](QUICKSTART.md)** も参照すること。

<br>

## ✅ 検証

実装作業を報告する前に、次を必ず実行する。

~~~bash
python tooling/schema_check/check_schemas.py
python tooling/conformance_tests/run_conformance_skeleton.py
~~~

toolchain に応じた条件付き検証:

~~~bash
cd native/rust_helper      && cargo test
cd apps/desktop_flutter    && flutter analyze
~~~

Mobile は現行rev5のP13 Product Build対象である。AndroidはAPI 35 Emulator上で製品UIからnative招待・Keystore・TLSを経た実Rust Brokerへの結合、Runtime表示、HOME復帰、切断がPASSした。iOSも専用Simulator上で製品Flutter UI・native招待／確認・既存Swift service・実Rust Brokerを通る基本経路がXCUITest 1件でPASSし、Runtime表示、HOME復帰、設定からの離脱が成立した。iOS接続試験は専用入力flag付きbuildに限定される。両platformの基本接続単位をCLOSEDとし、P13の残る製品機能差分へ進む。通常Release・物理端末・実端末の安全保管・配布identityは未成立で、Mobile release blockerを維持する。現行状態と証拠範囲は[rev5製品進捗](docs/REV5_PRODUCT_PROGRESS.md)を正本とする。

一括 reporter を使う場合:

~~~bash
python3 tooling/validate_all.py            # development slice
python3 tooling/validate_all.py --strict-release --desktop-platform=windows
~~~

厳格な Windows release validation には、native Windows の隔離された installed-path evidence が必要である。その evidence は source commit、clean worktree state、artifact hash、UIAutomation diagnostic tree、Broker の measured field provenance、installed app が生成した Setup Doctor の product export を含まなければならない。過去の Windows PASS record は owner-trial の履歴にすぎない。

> 実際に通っていない validation を「通った」と報告してはならない。

記録済み evidence の全履歴は **[VALIDATION.txt](VALIDATION.txt)** を参照すること。

<br>

## 🔒 Phase 0 で固定した面

- 汎用 Runtime Operation Shell の方向性
- BLUE-TANUKI を Phase 0 Reference Runtime Contract の対象として固定し、**Adapter だけを介して**接続する。
- **Flutter + Rust helper** を第一の実装候補とする。
- **Compose Multiplatform** を watchlist、**Tauri** を desktop-heavy fallback とする。
- UI framework の governance risk には <code>FrameworkRiskProfile</code> を使う。
- 適合境界: Adapter Conformance / Content Exposure Boundary / Authority Strip Conformance
- Schema-first / Conformance-first の作業順序

<br>

## 📂 リポジトリ構成

~~~
specs/                       # gate を担う Contract schema
  runtime · runtime_manifest · adapter · adapter_manifest
  capability · permission · approval · audit · recovery
  content_exposure · diagnostic · update · framework_risk_profile
  agent_runtime · agent_session · agent_task · agent_tool_call
  agent_workspace · agent_diff · agent_comparison · agent_handoff
  broker_command_envelope · broker_session · broker_health · broker_error
  ipc_request · ipc_response

packages/
  shell_core/         # 権限を所有する core（Python、framework 非依存）
  shell_ui/           # UI 部品
  shell_contracts/    # Contract 定義
  runtime_catalog/    # Runtime Registry の参照実装
  agent_runtime/      # Agent Runtime の参照実装
  blue_tanuki_adapter/# Reference Runtime 用 Adapter

apps/
  desktop_flutter/  mobile_flutter/

native/
  rust_helper/        # Rust Security Broker（authority-sensitive boundary）

examples/contracts/   # valid / invalid fixture、llm_bounded_extension を含む
docs/
  standards/ · specs/ · architecture/ · security/
  implementation/ · evidence/ · research/
installer/  windows/ · macos/ · linux/
tooling/    schema_check · conformance_tests · broker_parity · ...
~~~

<br>

## 📊 製品状態と履歴

現行工程はD4 Pocket / GUI-Shell rev5の`PRODUCT_BUILD_MODE`である。P2 Multi-Agent Compare、P3 Agent Handoff、Windows側P12 Product IntegrationはProduct Build受入れを閉じており、現在は独立trackのP13 Mobile／Non-Windowsを進めている。iOSは手動Actions run 37592632518で製品Flutter UI→native招待／確認→既存Swift service→実Broker、Runtime表示、HOME復帰、設定からの離脱がXCUITest 1件でPASSし、基本製品経路をCLOSEDとした。これは試験入力flag付きSimulator buildの証拠。先行run 37584511577のSwift TLS／Keychain部品接続もCLOSEDのまま保持する。Androidは手動Actions run 37580429609で製品UI→native招待／Keystore／TLS→実Rust Brokerの基本経路がPASSしCLOSED。次はP13の残る製品機能差分であり、物理端末・公開配布は未成立のまま別gateで管理する。Windows Feature Completeは達成済みだが、通常Release `task_execution=unsupported`、`release_ready=false`を維持し、Final QAは現行modeの指示により延期中である。現行phaseと証拠範囲は[rev5製品進捗](docs/REV5_PRODUCT_PROGRESS.md)、Final QAの過去記録は[Final QA Queue](docs/FINAL_QA_QUEUE.md)を参照する。

### 2026-09-29 の履歴

D4 Pocket統合rev2は、C33のWindows最大到達点とGUI Shell構成Manifest／Preview／編集提案の開発検証まで実装・検証済みである。Rust Broker、共有Flutter、Desktop Flutter、Mobile Flutter、Evidence assertion、C27性能smoke、C28短時間運用smoke、C29障害注入smoke、C30回帰matrix、C33のrelease buildおよびBroker smokeは開発環境でPASSしている。GUI Shell構成はManifest-only、Previewは読み取り専用、編集提案は審査待ちである。Windows書出しはOwner確認後に独立Manifest JSON fileをBroker固定保存先へ生成し、hash付きReceipt／Auditを返す段階まで成立したが、実行可能App package、独立Runtime、binary pruning、Installerや配布は未成立である。C28の8時間実測、Windows installed productの総合証拠、外部Runtime／Agent／MCP／A2A、実端末、正式署名、owner GOは未成立であり、PASSを製品releaseへ昇格させない。

2026-09-29、Rust source commit `6441ae8b2827d2afa01f9963ff2472fd2b889ce2`に対する手動Windows Actions #17で、Rust全target checkと12 test target／391件が成功し、そのcommitを`main`へ統合した。これはhosted Windows上の指定commitに対するRust検査と偽CLI `FIXTURE`の証拠であり、installed product、実Agent隔離、ローカルApplication Control、release readinessを証明しない。

2026-09-29当時の基準検査数と証拠は [docs/REV2_PROGRESS.md](docs/REV2_PROGRESS.md) と [総合機能拡張rev1の進捗](docs/総合機能拡張_rev1/進捗.md) に保存する。これは履歴snapshotであり、現行rev5工程や現在の実装状態を示す正本ではない。Agent Adapterの対話開始ではWorkspace IDを必須とし、Rust Brokerが同一Runtimeの登録Workspaceとの対応を監査してDesktopへ投影する。Desktop共有対話画面はBrokerの登録一覧から対象RuntimeのWorkspace IDを明示選択して通常要求へ渡す。Agent Task要求Schema、別Owner Approval、Workspace Permission、実行前の原子的再検証・一回消費、Brokerのbounded Task状態／取消経路は接続済みである。ただしCodex Adapterはread-onlyのままTask実行非対応であり、実Agent起動、実Agentのprocess-tree終了保証、隔離Workspace書込み、結果本文／diff表示、実Agent比較、Handoffは未成立である。対話ApprovalはWorkspace書込みPermissionやTask専用Approvalではない。C30のAgent probeもPATH上のCodex CLI version/help interfaceだけを観測し、実task、credential、workspace書込を証明しない。GUI Shell構成のCapability requirementはPermissionを生成せず、Previewもrollback実行可能性を生成しない。編集提案は`proposal_only`であり、自動applyや自己承認を行わない。Manifest fileのApp ID／Audit store IDは将来用の新規識別子であり、実packageや物理storeではない。Developer専用のFlutter画面選択Windows Release buildは独立製品、Owner権限、binary除去完了の証拠ではない。独立製品Export、pruning、Installer、署名、Distribution、実起動は未成立である。

この repository は **v1.0 product completion に向けた作業中であり、product release をまだ主張していない。** 機械判定上の状態は `not yet a completed product release` である。

Public review snapshot として tag を付けた GitHub Release は、完成製品の release ではない。この repository における完成製品の release readiness は、<code>release_blockers.registry.json</code> と明示的な owner GO によって引き続き gate される。

意図的に、次の順で優先する。

~~~
1. standard
2. specification
3. Conformance Boundary
4. Runtime Adapter Contract
5. Helper Boundary
        ── product UI より先に ──
~~~

<br>

<a id="claim-boundary"></a>
## 🧾 主張境界

誇張せずに述べた現在の事実:

- ✅ **Phase A/B は owner-use の範囲で完了している。** owner は desktop shell を日常の local operation に使え、status、problem、evidence、Recovery、Trust、Runtime、Authority の各 surface を確認できる。
- ✅ development slice として **schema + conformance が通過**している（2026-09-30時点で150 schema、150正常example、193 negative fixture、229 check）。Windows書出しはOwner確認後にManifest JSON fileを新規生成するが、実行可能App、独立Runtime、binary pruning、Installer、配布は成立していない。
- 🧪 C33ではWindows release buildと、clean isolated runにおけるRust Broker smoke（認証IPC、永続store、replay拒否、再起動後health、crash fail-closed）がPASSした。ただしinstalled productの総合evidence validatorは、UI Automation surface不足、Setup Doctorのinstalled path不一致、外部Audit anchorが未取得のため失敗しており、Windows正式証拠は成立していない。
- ✅ **LLM が読む基盤は definition-locked** であり、範囲を限定した Reference Extension が一つ、cross-agent reproduction report が一つある。
- ⛔ **v1.0 product release はまだ主張していない。** active <code>release_blocker</code> は <code>release_blockers.registry.json</code> に正規化されている。内容は Windows installed-path provenance、first-run、Setup Doctor、Broker evidence、Audit anchor の external tamper-evidence proof、明示的な owner GO である。Rust Broker の production authority cutover に関する表現は、独立した registry blocker ではなく、Windows installed-path の Broker / Runtime evidence blocker を通じて表現する。
- 🧪 **公開 proof pack の境界。** 公開 Windows proof pack には、実測 Windows installed-path evidence に由来する redacted review copy が含まれる。これらは canonical release evidence ではなく、この公開 repository 上の完成製品 release blocker を解消しない。
- ⚠️ **Windows-first。** Linux validation は development slice であり、最終的な product proof ではない。macOS、iOS、Androidの実機とWindows installed productの現行証拠は別関門であり、supportedまたはrelease proofとして宣伝してはならない。
- ✅ **P13 Mac資格情報の有限製品受入れはCLOSED。** native秘密入力・別個Owner確認・Keychain登録・公開metadata一覧・論理失効・通常終了が[手動検収run](https://github.com/gatchimuchio/D4-POCKET/actions/runs/37742214244)でPASS。Debug合成秘密とtest identityの範囲であり、Provider／MCP注入・Task・正式配布・最終QAへ拡大しない。契約と過去FAILは[macOS資格情報保管庫](docs/specs/macos-credential-vault.md)を参照。
- ✅ **P13 Mac MCP接続・Tool・切断の有限製品受入れはCLOSED。** 既存Broker／native Owner経由の実stdio Tool、hash-only表示、一回消費Audit、明示切断・通常終了・process回収が[手動検収run](https://github.com/gatchimuchio/D4-POCKET/actions/runs/37763440541)でPASS。合成公開Serverとtest identityの範囲。Credential注入・離脱group／crash保証・正式配布へ拡大しない。[有限契約と過去FAIL](docs/specs/macos-mcp-center.md)を参照。
- ✅ **P13 Mac MCP資格情報結合の有限製品受入れはCLOSED。** 公開ID選択・別個native Owner確認・現在Keychain recordから対象stdioへの実受渡し・公開使用時刻／使用Audit・切断・通常終了・回収が[手動検収run](https://github.com/gatchimuchio/D4-POCKET/actions/runs/37787435817)でPASS。秘密値はUI／Auditへ返さない。native Debug合成秘密・test identityの範囲で、Provider／Agent／A2A・正式配布・最終QAへ拡大しない。[有限契約と過去FAIL](docs/specs/macos-mcp-credential-binding.md)を参照。
- ✅ **P13 Mac A2A接続の有限製品受入れはCLOSED。** 別個native Owner・既存Brokerのloopback Card取得・metadata-only未審査／hash表示・URI消去・通常終了・回収が[手動検収run](https://github.com/gatchimuchio/D4-POCKET/actions/runs/37797680275)でPASS。合成公開Cardとtest identityの範囲。Trust／Task／Credential・外部host・正式配布へ広げない。[有限契約と過去FAIL](docs/specs/macos-a2a-center.md)を参照。
- ⚠️ この基盤作業は、権限を持たない task に対して統制された LLM-readable extension behavior を限定的に実証する。public standard への採用、広範な第三者 interoperability、installed-product behavior は証明しない。

repository 文書にある未完了項目は、すべて <code>release_blocker</code> / <code>post_v1_scope</code> / <code>known_limitation</code> に分類する。[CLAIM.md](CLAIM.md) と [RELEASE_CHECKLIST.md](RELEASE_CHECKLIST.md) を参照すること。

<br>

## 📚 主要文書

| 文書 | 説明 |
|:---|:---|
| [AGENTS.md](AGENTS.md) | repository の Agent 規則と作業規律 |
| [docs/specs/gui-shell-spec-v1.md](docs/specs/gui-shell-spec-v1.md) | GUI-Shell v1 実装仕様 |
| [docs/standards/llm-readable-extension-surface.md](docs/standards/llm-readable-extension-surface.md) | LLM が読む責任基盤の標準 |
| [docs/standards/gui-shell-extended-standard.md](docs/standards/gui-shell-extended-standard.md) | 拡張標準 |
| [ROADMAP.md](ROADMAP.md) | Phase の計画と実行順序 |
| [docs/OPERATING_MODEL.md](docs/OPERATING_MODEL.md) | リポジトリの作業流、backup model、validation gate |
| [CLAIM.md](CLAIM.md) | 現在の主張境界 |
| [AUDIT.md](AUDIT.md) | Audit と invariant の要件 |
| [SECURITY.md](SECURITY.md) | セキュリティ姿勢と報告方法 |
| [Rust Broker IPC 通信規約](docs/architecture/RUST_BROKER_IPC_PROTOCOL.md) | Rust Broker の IPC 通信規約 |
| [TROUBLESHOOTING.md](TROUBLESHOOTING.md) | validation / setup の失敗対応 |

<br>

## 📝 ライセンス

成果物の種類ごとにライセンスを分離する。

- ソースコード、テスト、ツールその他のソフトウェア構成物: [Apache License 2.0](LICENSE-APACHE-2.0)
- 仕様、設計、READMEその他の文書・知的成果物: [Creative Commons Attribution 4.0 International](LICENSE-CC-BY-4.0)

これは、すべてのファイルについていずれかを任意に選べるデュアルライセンスではない。適用範囲と第三者由来物の扱いは [LICENSE](LICENSE) と [NOTICE](NOTICE) を参照すること。

<br>

<div align="center">
<sub>Schema-first・Conformance-first・Safety-first。LLM がその上に構築する基盤であり、LLM が保持する権限ではない。</sub>
</div>

## rev2対話のLinux開発検証範囲

- item: Linux対話UIの実起動証拠はWSLgのX11に限定
  classification: known_limitation
  reason: Linuxで解析・共有6試験・Desktop32試験・Mobile14試験・release build・製品Dart clientの実MINIDORA接続は成立した。最新releaseのPID一致とIsViewableも観測した。DRI3取得のlibEGL警告があり、GPU性能の保証はしない。実ウィンドウの可視状態はGTK標準の `GDK_BACKEND=x11` を指定して確認した。既定WaylandのウィンドウはX11検査器では観測できず、Waylandの画面成立をこの証拠から主張しない。
  required_action: Wayland環境の画面対応を主張するときは、その環境に対応した表示検証を別途行う。
  blocks_release: no

## rev2端末連携の境界

端末連携の操作と責任は [端末連携仕様](docs/specs/device-link.md) を参照する。

- item: Desktop再起動時の端末再結合
  classification: known_limitation
  reason: 秘密鍵と結合資格を起動世代内に限定し、Desktop再起動で全招待・結合・未完了対話を失効させる。通信の再接続は有効期限内なら可能だが、Host再起動後の資格自動復元は行わない。
  required_action: Desktop再起動後はownerが新しい招待を発行しMobileで結合し直す。未完了要求を自動再送しない。
  blocks_release: no

## P13開発用一時source copy

- item: CLI投影の局所Flutter試験用ASCII copyの回収未確認
  classification: known_limitation
  reason: `C:\Users\ohira\AppData\Local\Temp\d4-cli-flutter-c2eac7a89629418b930ad182bed68e3a`は追跡済みDesktop／shared sourceのdev-only複製。試験終了後の検証済み絶対pathへの削除はtool policyで実行前に拒否され、別shell等では迂回していない。製品経路・配布物・Authority・Credentialには含めない。
  required_action: 許可された通常の一時folder回収手段で当該copyだけを除去する。製品機能のAcceptanceを延長しない。
  blocks_release: no
