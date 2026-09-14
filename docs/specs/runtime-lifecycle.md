# 実行系ライフサイクル（Runtime Lifecycle）

## 目的と責任境界

実行系ライフサイクルは、登録済み Runtime に対する開始、停止、再起動、一時停止、再開、隔離を、Broker の認証済み IPC と統治経路を通じて扱う。

Flutter は状態と Broker が返した操作一覧を表示し、operator input を収集してよい。Flutter、Adapter metadata、過去の UI state、memory、cache、診断出力は、Capability、Permission、Approval、Audit、Recovery を生成または変更してはならない。

通常 UI は process kill、PID 指定、endpoint 指定、command、argv、環境変数を送信しない。Shell Core は Runtime の内部 process を直接操作しない。Rust Broker が、登録済み Lifecycle Adapter と Broker 所有の統治状態だけを用いる。

BrokerCommandEnvelope の generic dispatch は引き続き停止したままとする。C4 はこれを有効化せず、専用の lifecycle IPC operation だけを追加する。

## 操作

操作識別子は、既存外部接続との互換性のため次の固定値を使う。

    start
    stop
    restart
    pause
    resume
    quarantine

UI の表示名は日本語で開始、停止、再起動、一時停止、再開、隔離とする。

各操作は Adapter が宣言し、Broker が登録時に固定した Capability の operations に含まれる場合だけ候補になる。Capability は宣言であり、実行権限ではない。Runtime / Adapter manifest も説明用 input であり、それ自体で Permission を付与しない。

## IPC surface

C4 IPC surface は次の4 operation を持つ。通常資格経路は状態、承認要求、操作の3 operation だけを受け付け、承認は owner 専用資格経路だけが受け付ける。

BrokerEndpoint は `credential_role` を必須の固定fieldとして持ち、値は `normal` または `owner` だけである。Desktop Broker client は完全なendpoint field集合と `credential_role=normal` だけを受理し、owner、欠落、未知値、余分fieldをSocket接続・資格送信の前に拒否する。owner CLI は `credential_role=owner` だけを受理し、normal、欠落、未知値、余分fieldをBroker接続の前に拒否する。通常資格fileとowner資格fileは互換の代替経路ではない。

    実行系ライフサイクル状態
    実行系ライフサイクル承認要求
    実行系ライフサイクル承認
    実行系ライフサイクル操作

実行系ライフサイクル状態の payload は、版と実行系 ID だけを持つ。

    {"版": 1, "実行系ID": "registered-runtime"}

実行系ライフサイクル承認要求の payload は、版、実行系 ID、操作を持つ。

    {"版": 1, "実行系ID": "registered-runtime", "操作": "pause"}

実行系ライフサイクル承認は owner 専用である。通常 Flutter は owner 資格を取得、保存、送信しない。

    {"版": 1, "承認ID": "lifecycle-approval", "承認hash": "sha256:..."}

実行系ライフサイクル操作の payload は、版、実行系 ID、操作、承認 ID を持つ。

    {"版": 1, "実行系ID": "registered-runtime", "操作": "pause", "承認ID": "lifecycle-approval"}

外側の IpcRequest は request ID、現行 Broker session、canonical payload hash、nonce、issued-at を結合する。Broker は要求時刻、session、nonce、payload hash、metadata の authority 注入を先に検証する。

操作 payload に次の値を追加してはならない。

    PID
    endpoint
    command
    argv
    env
    Capability ID
    Permission ID
    監査ID
    復旧ID
    呼出し側状態
    authority
    metadata

## 統治と実行

Broker は、操作実行時に現在の Broker 所有 state から次を全て照合する。

1. Runtime が登録済みである。
2. Adapter が操作を含む Capability を宣言している。
3. Capability と operation の対応が一致する。
4. 現在の Permission が runtime、Capability、operation、target scope に対応し、allow である。
5. 現在の Approval が runtime、operation、target scope、canonical payload hash に対応し、approved である。
6. 対応する RecoveryAction が登録済みである。
7. durable audit / replay / session state が利用可能である。
8. 現在の lifecycle state から操作が許可される。

この照合のどれかが失敗した場合、Broker は Lifecycle Adapter を呼ばず、状態を変更せず、rejected または suspended を返す。Permission denied、Approval missing、Recovery missing、unknown runtime、stale request、replayed nonce、audit unavailable は、いずれも fail-closed とする。

結果契約は、状態、承認、成功遷移だけを相互排他的な固定形で返す。結果に種別項目を追加しない。成功遷移は、版、実行系ID、操作、遷移前後状態、仲介器の観測時刻、`LIVE_RUNTIME`、ライフサイクル監査IDを必須にする。統治は、能力ID、権限ID、承認ID、承認状態、復旧IDの5項目を持ち、成功後の承認状態は`consumed`でなければならない。外側のIPC応答にある監査event IDは、この監査IDと一致しなければならない。

状態照会は、対応、現在状態、証拠種別、Broker 生成の操作一覧、承認一覧を返す。各操作は、操作、能力 ID、権限 ID、承認必要=true、復旧 ID、実行可能だけを持つ。実行可能は、現在状態、Capability、Permission、Recovery により承認要求を出せることを示すだけであり、直接実行を許可しない。実行には別途、Brokerが照合した approved Approval が必要である。

承認一覧と承認要求・owner承認の結果は、版、承認 ID、承認hash、実行系 ID、操作、状態、有効期限UnixSeconds、統治を持つ。統治の承認 ID・承認状態は承認射影と一致しなければならない。Adapter が lifecycle Capability を宣言しない場合、対応=false、状態=not_supported、証拠種別=INTERNAL_STATE、操作一覧と承認一覧は空である。UI はこの場合に lifecycle button を表示しない。

## 遷移と隔離

通常の成功遷移は次を基本とする。

- 停止済みから`start`を実行すると準備完了になる。
- 準備完了から`stop`を実行すると停止済みになる。
- 準備完了から`restart`を実行すると準備完了に戻る。
- 準備完了から`pause`を実行すると一時停止になる。
- 一時停止から`resume`を実行すると準備完了に戻る。
- 停止済み、準備完了、一時停止から`quarantine`を実行すると隔離状態になる。

Broker は実行後の状態を再観測できた場合だけ成功遷移を LIVE_RUNTIME として返す。過去の UI 表示や caller supplied state から遷移可否を推論しない。

quarantined は通常 lifecycle 操作、通常対話、通常実行を復帰させない terminal state である。quarantine の監査予約を Adapter 呼出し前に append できた時点で、Broker は対象実行系IDを terminal とし、通常対話Adapter、資源観測、作業領域root・承認・基準点を直ちに除去する。以後の Adapter 失敗、Broker 停止、または再起動は、この通常経路を復元しない。再起動後は検証済み監査chainの予約から terminal 状態を復元し、Lifecycle Adapter が再登録されなくても quarantined と空の操作一覧・承認一覧を返す。同じ実行系IDの通常Adapterまたは作業領域の再登録は拒否する。限定診断が必要な場合は、別の read-only diagnostic path を使い、quarantine を解除する shortcut にしてはならない。解除や再稼働は、将来明示される RecoveryAction の統治経路以外から行わない。

C4 の terminal 対象は、trusted registration で固定する実行系IDである。Broker は、任意の別IDが同じ物理Runtimeまたは同じendpointを指すことを推定しない。物理実体をまたぐ隔離を必要とする production Lifecycle Adapter は、導入時に不変のphysical identity/bindingをContract化し、Broker が同じbindingへの別ID登録を拒否できることを検証しなければならない。

## 監査と失敗

すべての lifecycle 要求の受理、拒否、保留、成功遷移、失敗遷移は append-only audit chain に記録する。監査が事前に利用不能なら Adapter を呼ばない。実行後の監査 finalization が失敗した場合、Broker は成功を主張せず、現在状態を隠さず、隔離または明示的な Recovery 状態へ fail-closed に遷移させる。

監査 event に raw command、argv、環境変数、credential、全文 payload を保存しない。要求 hash と結果の正本 hash、Runtime、operation、decision、evidence source、Recovery 対応だけを必要最小限に結合する。

## MINIDORA と開発証拠

MINIDORA / BLUE-TANUKI の現行 Adapter は lifecycle Capability を宣言していない。この Capability を C4 の表示や試験のために追加してはならない。Capability 不在の Runtime は操作を表示・実行しない negative case とする。

Windows では、限定された development-only fixture Runtime / Lifecycle Adapter を用いて、Broker、専用 child、Flutter debug 画面の実装・自動試験・画面検証を進める。Android 物理実機は凍結を維持する。macOS、iOS、Linux、Android の物理実機証拠がないことは development blocker ではなく、release evidence の release_blocker である。
