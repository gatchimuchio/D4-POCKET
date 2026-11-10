# GUI Shell編集提案の意味正本

## 目的

`GUI Shell編集提案`は、Owner／Developerが明示開始した実装Agentの要求を、構成・UI・Contractの変更候補として審査待ちへ記録する開発経路である。製品Runtimeが自分自身を変更する機能ではない。

## Broker経路

Owner資格経路から編集指示、対象path、予定変更を認証済みRust Brokerへ渡す。Brokerは版、識別子、範囲、対象path、規約確認、自己承認禁止、`proposal_only`を再検証し、`INTERNAL_STATE`のAuditEvent付きReceiptを返す。指示本文はReceiptへ返さず、要求hashだけを結合する。

## 適用範囲

対象pathは`apps/desktop_flutter`、`packages`、`specs`、`docs`、`tooling`、`native/rust_helper/src`、`規定`の境界内に限定する。`.git`、Manifest、release evidence、secret、credential、private、signingに関係するpathは拒否する。外部Agentのmetadataや履歴はAuthority、Permission、Approvalを生成しない。

## 自動適用禁止

現行単位の`execution_mode`は`proposal_only`、`review_required=true`、`apply_status=not_started`、`files_written=false`に固定する。編集Agentの提案をそのままapplyせず、Owner／Developerが差分、Contract、テスト、監査、権限境界を確認してから、別の明示的な開発作業として実装する。自己承認、Permission生成、Approval迂回、秘密値取得は行わない。

## Agent Centerへの別Task受け渡し

Settingsの編集提案画面は受理Receipt後、指示本文、対象path、予定変更、指示hash、Audit IDをOwner／Developerが確認できる。明示的な確認後、本文とReceipt参照を未信頼contextとしてAgent Centerへ渡し、登録済みSessionの選択後に既存Agent作業要求検査だけを行う。Agent Centerの事前検査Dialogでは内容を固定表示し、MCP能力追加を要求しない。

この受け渡しはproposalの自動applyではなく、独立した通常Broker Task要求の準備である。受け渡し自体はPermission、Approval、Credential、Agent trust、Task実行を生成・継承しない。target_pathsはTaskの希望範囲であり、OSまたはBrokerのアクセス境界ではない。実際のアクセス範囲はBroker Workspace Permissionが決める。Task指示は対象path内に限定し、必要範囲が合わない場合は書込み前に停止してOwner／Developerへ報告する。通常Broker経路で現在のWorkspace PermissionとTaskごとの一回Owner Approvalを別々に取得する。Task事前検査が成功するまで下書きを保持し、取消・Broker拒否時は下書きを消費しない。受理後もファイル変更や実行完了と誤表示しない。
