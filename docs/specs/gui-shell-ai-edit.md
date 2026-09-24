# GUI Shell編集提案の意味正本

## 目的

`GUI Shell編集提案`は、Owner／Developerが明示開始した実装Agentの要求を、構成・UI・Contractの変更候補として審査待ちへ記録する開発経路である。製品Runtimeが自分自身を変更する機能ではない。

## Broker経路

Owner資格経路から編集指示、対象path、予定変更を認証済みRust Brokerへ渡す。Brokerは版、識別子、範囲、対象path、規約確認、自己承認禁止、`proposal_only`を再検証し、`INTERNAL_STATE`のAuditEvent付きReceiptを返す。指示本文はReceiptへ返さず、要求hashだけを結合する。

## 適用範囲

対象pathは`apps/desktop_flutter`、`packages`、`specs`、`docs`、`tooling`、`native/rust_helper/src`、`規定`の境界内に限定する。`.git`、Manifest、release evidence、secret、credential、private、signingに関係するpathは拒否する。外部Agentのmetadataや履歴はAuthority、Permission、Approvalを生成しない。

## 自動適用禁止

現行単位の`execution_mode`は`proposal_only`、`review_required=true`、`apply_status=not_started`、`files_written=false`に固定する。編集Agentの提案をそのままapplyせず、Owner／Developerが差分、Contract、テスト、監査、権限境界を確認してから、別の明示的な開発作業として実装する。自己承認、Permission生成、Approval迂回、秘密値取得は行わない。
