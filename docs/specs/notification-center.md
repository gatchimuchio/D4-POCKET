# 通知センター

状態: C12実装正本

## 意味

通知センターは、Rust Brokerが確定した監査eventから、操作者向けのsummaryを限定射影する。通知をcallerが登録する操作は持たず、監査reason、payload、metadata、資格情報を表示しない。

通知のsourceはBroker operationまたはBrokerが検証した構造化Task Auditの固定対応からだけ決まり、`Runtime`、`Task`、`Permission`、`Approval`、`Audit`、`Recovery`、`Device`、`MCP`、`A2A`、`Update`、`Evaluation`に限定する。Agent TaskはBrokerのhash検証済み履歴から開始・完了・失敗・取消・中断のstatusだけを投影し、指示本文・結果本文・Task識別子・failure classを表示しない。Workspace PermissionとOwner Approvalの発行・拒否は各Broker operationから区別して投影する。通知は`INTERNAL_STATE`の観測であり、Permission、Approval、Authority、Capability、Credentialを生成しない。

## 実装経路

Desktop Flutterの通知画面 → 通常認証済みBroker IPC → Rust Notification Center → 監査eventの限定射影と`notifications.json`の表示状態。

提供する操作は`通知一覧`、`通知既読`、`通知破棄`、`通知全既読`である。既読・破棄は現在の通知hashへ結合し、再起動後もBrokerが再読込する。通知の「開く」は既存GUI画面へのnavigationだけで、sensitive actionを直接実行しない。

`全Runtime停止要求`の監査eventはRuntime通知としてProblemsへ案内する。受理時のsummaryは停止要求の受付、Runtime未停止、Owner再承認の必要を明示する。通知には要求payloadや対象内容を含めず、開く操作はProblemsへの画面遷移だけを行う。

## 上限と失敗

監査走査は直近4096 event、返却は最大256件に制限する。通知表示状態の未知field、重複ID、hash不正、malformed file、stale hashはfail-closedで扱う。native Windows toastは既存Broker host capabilityが未接続のため、本単位では実装済みと主張しない。

Desktop Client／画面はBroker拒否文、transport例外、malformed response由来の例外詳細を操作者向け表示へ流さず、固定の通知失敗文へ正規化する。診断詳細を新しいログやAuditへ複製しない。

## 検証

`notification.schema.json`、`notification_list.schema.json`、`notification_action.schema.json`、負例fixture、Rust Notification Center単体試験、Desktop client／navigation試験、Schema／Conformanceへ接続する。Task完了・失敗AuditとPermission operationの投影、summary／Authority非生成境界をRust単体試験とSchema／Conformanceで確認する。これらはFIXTURE／INTERNAL_STATE範囲であり、installed productやLIVE_RUNTIMEの証拠へ昇格しない。
