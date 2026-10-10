# B1 成立原理・対案・責任境界

状態: B1有限設計QCの受入記録。B0の未知は変更せず、実作用・製品接続はB2へ送る。
対象基準HEAD: `f29c09012f529b704b10accaf45e8e73c31fcaba`。
適用正本: 採用記録、補遺実装仕様書§1–7、工程表B1、実装指示書§4、現行`AGENTS.md`。

## 1. 有限範囲と判定

B1の目的は、緩衝が何を意味し、どこで誰が作用し、どの反例なら成立を拒否するかを設計として固定すること。B1では製品の実作用を主張しない。

受入した出力は次のとおり。

1. B0で直接観測または強い推定と分類済みの代表事象を、作用可能な境界と作用不能な境界へ対応づけた。
2. 診断のみ・万能な権限Hub・個別の固定化・型付き非権限調停を比較し、最小原理を選んだ。
3. 状態、意味同等性、作用範囲、再確認、失敗・復旧、結果証拠を定義した。
4. `buffer_intervention_record.schema.json`と正常・否定fixtureをconformance経路へ接続し、候補の誤昇格、根拠不足の吸収、副作用FAIL、復旧未完了を拒否した。
5. `BUF-T02/T03/T17/T20/T22`を設計時の成立条件、反例、後続の実動作確認へ分離した。

**判定: PASS / CLOSED（設計・契約形状の範囲）。** Schema/conformance PASSは製品consumer、実操作、緩衝効果、意味同等性の実証ではない。実際に元Toolを通し効果を測るのはB2以降の条件である。

## 2. 日本語意味正本

**緩衝調停**とは、元の要求、成功条件、対象、権限を変えず、境界間の衝突を特定範囲内の可逆な変換・隔離・環境固定・状態調停、または既存Toolchain機能によって処理し、元のToolを実行して独立照合する媒介作用である。失敗の分類、表示、正規化、READY宣言、単純な再試行だけは緩衝ではない。

緩衝成立には、以下をすべて要する。

- 元要求、成功条件、入力、source、Toolchain、Host、Workspaceを識別できる。
- 適用直前に識別指紋を再照合し、一つでも変化・不明なら候補を破棄して再評価する。
- 作用は明示された狭い対象と既存権限の内側に限る。より広い権限、新しいApproval、秘密、OS保護の迂回を生成しない。
- 変換は意味保存の証拠を持ち、非可逆・未知field・lossyな場合は適用しない。
- 作用後に、指定された元Toolを同じ要求意味で実行し、元の成功条件を独立した証拠で検査する。
- 因果効果を主張する場合、介入前後のsource/input/Host/Workspace/Toolchain/成功条件を一致させ、副作用なしを対照試験で照合する。
- 作用前後の状態差、出力、残余、副作用、復旧状態を記録する。

この契約内のTask/Host/Workspace ID、証拠参照、Tool名、状態は識別・記録の値であり、AuthorityやTrustを作らない。`候補のみ`・`作用後未照合`・`未成立`・`未観測`は`吸収成立`へ読み替えない。

## 3. B0実記録と作用原理の対応

| B0事象・境界 | 必要な意味 | 原理上の候補 | 成立を拒否する条件／上流帰還 |
|---|---|---|---|
| `37553724749` Rust target API不整合、`37555551420` Swift throwing差、`37557097383` `NWConnection` API差 | 指定targetで本来のcompile/API契約を満たす | 既存のtarget/toolchain宣言・compile機能を再利用し、明示されたtargetに適合 | 実際のsource/API誤りはsource/契約修正へ戻す。diagnostic正規化や別target成功で吸収扱いしない |
| `37557804963` Simulator Keychain entitlement拒否 | OSが許可したKeychain作用だけを試験 | 署名・entitlement条件の観測と、同一の許可済み試験Host選択 | OS拒否は緩衝対象ではない。無効化・別資格・別権限へ迂回せず`未成立` |
| `36380633052`／`36381508834` Windows test TEMP短縮pathとWorkspace契約の衝突 | production path規則を変えず、test用rootを同一job内で識別 | test harnessの専用環境binding。ただし対象はtest processだけ | productのpath許容を広げた場合、またはtest環境と製品環境を同一証拠扱いした場合は不成立 |
| `26960969593` Broker cold-start timeout推定 | 固定deadline内に実Broker readyを観測 | 既存Broker状態観測と、要求時に固定済みの残り時間内のwait | timeout延長だけ、session未生成を成功扱い、deadline後継続は不成立。要求期限またはBroker状態へ帰還 |
| `34741774416`／`34742562494` Android emulator実容量不足 | 実測空き容量と起動要求量の整合 | 許可済みHost profileの選択、実測による実行前拒否 | 見かけのpartition値、環境外SDＫ削除、無断resource増加では吸収しない |
| `37203399508` test oracleの拒否code drift、`37633822477` UI試験selector timeout | testが現行contract・要素責任面を正しく測る | 独立test oracle/selectorを責任境界に合わせて修正 | 製品処理未到達を製品成功・失敗にしない。test修正だけをruntime緩衝と呼ばない |
| `37635033580` Actions step開始前の外部runner failure | 実行された検査範囲だけを主張 | なし。未実行と記録し、許可された手動runner条件が戻るのを待つ | runner failureを緩衝基盤の成功・製品failureへ読み替えない |
| `OBS-02` OneDrive Cloud Files生成物／cache候補 | source不変で、同じ入力の実Tool成果を得る | 一時出力隔離を候補とする。source/input/output hashとcleanupの実証が前提 | ASCII場所へ移しただけ、source違い、cache汚染、成果hash不一致では未成立。installed同等性を主張しない |
| secret・hardlink・Host/Workspace境界 | 登録範囲と機密境界をそのまま維持 | 既存Broker/Workspaceの現在判定を再利用 | Broker拒否、未知root、TOCTOU変化時は作用停止。Mediatorからpath・secretを直接読む権限を作らない |

これらはB0台帳の代表例であり、全203 failureを同じ根因へ統合しない。13 run-level UNKNOWN、取得不能log等を既知原因に変更しない。全行への共通機構割当はB3の範囲である。

## 4. 代替仮説と採用原理

| 案 | 利点 | 欠陥／反例 | B1判定 |
|---|---|---|---|
| 診断・error normalizationだけ | 低侵襲、観測を揃えやすい | 元Toolを通さず、上位へ再試行を返すだけ | 緩衝の定義を満たさず不採用 |
| あらゆる差を自動変更する万能Hub／wrapper | 既知外の差にも介入できるように見える | 任意command・path・依存変更、権限拡大、再現不能、問題隠蔽を誘発 | 不採用。固定した型付き作用を超えない |
| 全Hostを一つの固定profileに統一 | 既知の版差・設定差を減らせる | OSの許可差、利用者環境、正当な複数targetを消せない | 基準profileとしては候補。全問題の解決策にはしない |
| 型付き非権限調停＋既存Broker所有の限定作用 | 共通の証拠・意味・失敗モデルと狭い作用を分離できる | 既存Brokerの許可経路が対象作用を表現できなければ実行不能 | **設計原理として採用。** 不能時はBrokerを迂回せず未成立 |
| 各Toolchainのnative機能を再利用 | lock、target、cache等の既存意味を活かす | Toolchainごとの契約とversion差を検証する必要がある | 型付き調停内で最初に比較する候補。固有機能をCoreへハードコードしない |

実装言語、process配置、Broker IPC、Mediatorの保存・cache方式はB1で決めない。最初のB2縦断対象の実行経路と既存Consumerを確認し、同じ設計条件のもとで必要最小限だけを選ぶ。

## 5. 契約の責任境界と状態

1. Agent/Ownerは目的と通常操作要求を定義する。Agent出力・memory・履歴・metadataは権限源ではない。
2. 既存Rust Brokerだけが現在のRuntime/Host/Workspace、Permission、独立Approval、Audit、Recoveryを評価する。Mediator記録の識別子はその評価を代替しない。
3. Mediatorは境界事実、候補機構、意味保持条件、再照合fingerprint、作用結果、未解決を計算・記録する非権限責任である。任意command/path、credential、Approval tokenを受け取らない。
4. 作用主体は既存Brokerに束縛されたAdapter/Host/Workspace/Process所有経路だけ。操作を既存許可で表せない場合は停止し、新しい橋渡しを作らない。
5. 元Toolchainは目的の操作を実行する。独立照合は同じToolのexit codeだけでなく、成功条件と成果hashを対象にする。
6. Audit/Content Exposureは既存のBroker所有規則に従う。Mediatorの自由文へsecretやraw payloadを複製しない。

状態は次の意味に固定する。

- `候補のみ`: 設計候補であり、権限・適用可否・作用実績はない。
- `作用後未照合`: 作用と元Tool結果はあるが独立照合がなく、成功ではない。
- `吸収成立`: 実作用、元Tool成功（終了値0）、独立照合PASS、同条件の介入前後対照、要求意味一致、副作用照合PASS、復旧完了または不要が揃った場合だけ。
- `未成立`: 拒否、意味不一致、作用／元Tool失敗、副作用、復旧未完了等。
- `未観測`: 必要な外部条件・証拠がなく、成立状態を確定できない。

`吸収成立`もPermission、Approval、release readiness、他Host/Workspace/Agentでの再利用権限に変換しない。fingerprint不一致、Replay、期限切れ、前提変更時は古い候補を破棄し、現在条件でBrokerが再評価する。

## 6. 要求×設計反例×後続試験

| 試験ID | B1設計の受入条件・反例 | B1で行った確認 | B2以降で必要な実動作証拠 |
|---|---|---|---|
| `BUF-T02` | ABI/target/依存矛盾。lockfileが変わる、未知versionを許可する候補は拒否 | action範囲と固定指紋を契約化 | 同一source・lock・targetで隔離/拒否が実行され、元Toolが結果を返す |
| `BUF-T03` | 変換前後の意味同等性。未知field、lossy変換、round-trip不一致は非適用 | 証拠・作用・独立照合が別fieldでないと`吸収成立`になれないschema条件 | 実データの往復/意味比較と不一致時の無作用を確認 |
| `BUF-T17` | Adapterなし、原因未知、OS拒否、環境不在で安全停止 | `未成立`/`未観測`と権限なし記録を定義 | production Broker経由の拒否、無fallback、state不変、Auditを確認 |
| `BUF-T20` | 反例または相反測定後、根因モデル・作用・上流条件を更新する | 各事象に根拠状態と未解決を保持し、分類のみを吸収としない設計 | 失敗注入後に当該上流条件へ帰還し、修正版で同条件再測定 |
| `BUF-T22` | 介入前後を同一source/input/Host/Toolchain/成功条件に揃える。retry、作業場所変更、assertion緩和だけは因果証拠にしない | fingerprint項目と独立照合・副作用記録を契約化 | matched runでexit/result hash、時間・資源、状態差、cleanupを比較 |

B1のSchema/conformanceで試したのは、契約形状、Authority field拒否、候補から実行実績への誤昇格拒否、`吸収成立`に実作用・元Tool成功・独立照合・同条件対照・副作用PASS・復旧状態を必須とする条件だけである。上表のproduction経路試験は未実行であり、その成功をB1のPASSへ混入しない。

## 7. 機械契約とconsumer

- Schema: [`specs/buffer_intervention_record.schema.json`](../../specs/buffer_intervention_record.schema.json)
- 正常例: [`examples/contracts/buffer_intervention_record.valid.json`](../../examples/contracts/buffer_intervention_record.valid.json)
- 否定例: Authority注入、根拠なし`吸収成立`。既存schema/conformance runnerで検査する。
- 現在のconsumer: schema/conformance検査だけ（設計契約検査）。製品Runtime consumerなし。
- 将来consumer: B2で選定する実在Mediator/Broker経路。B2で既存consumerが契約を満たせない場合、production接続前に契約へ戻る。
- `吸収成立`のSchema条件は実作用・元Tool成功・独立照合PASS・同条件対照・副作用PASS・復旧状態を必須にするが、証拠参照の真偽やfingerprint間の一致を検証するruntime consumerはまだない。
- schema存在、fixture合格、文書記録から実作用・Broker認可・Audit・意味同等性を主張しない。

## 8. B1受入範囲外・次工程

- 緩衝処理のruntime実装、既存Broker/Adapter/UIへの接続、権限・Approval・Audit/Recovery実作用。
- OneDrive/NTFS/Cloud Files、Windows SAC、実機・Simulator、別Hostでの実効果。
- 203行の全原因への共通機構割当、全製品経路への適用、自己適用、性能/長時間/fault injection。
- `task_execution`、`release_ready`、正式Q0–Q7、release blocker。

次はB2の有限作業指示で、B0記録から一つの実測可能な事象を選び、既存Broker権限のまま元Toolchainの前後を通す。候補はB2開始時の現行証拠で再選定し、ここで特定事象の実作用を予断しない。
