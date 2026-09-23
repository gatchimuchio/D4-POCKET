# 通知センター

状態: C12実装正本

## 意味

通知センターは、Rust Brokerが確定した監査eventから、操作者向けのsummaryを限定射影する。通知をcallerが登録する操作は持たず、監査reason、payload、metadata、資格情報を表示しない。

通知のsourceはBroker operationの固定対応からだけ決まり、`Runtime`、`Approval`、`Audit`、`Recovery`、`Device`、`MCP`、`A2A`、`Update`、`Evaluation`に限定する。通知は`INTERNAL_STATE`の観測であり、Permission、Approval、Authority、Capability、Credentialを生成しない。

## 実装経路

Desktop Flutterの通知画面 → 通常認証済みBroker IPC → Rust Notification Center → 監査eventの限定射影と`notifications.json`の表示状態。

提供する操作は`通知一覧`、`通知既読`、`通知破棄`、`通知全既読`である。既読・破棄は現在の通知hashへ結合し、再起動後もBrokerが再読込する。通知の「開く」は既存GUI画面へのnavigationだけで、sensitive actionを直接実行しない。

## 上限と失敗

監査走査は直近4096 event、返却は最大256件に制限する。通知表示状態の未知field、重複ID、hash不正、malformed file、stale hashはfail-closedで扱う。native Windows toastは既存Broker host capabilityが未接続のため、本単位では実装済みと主張しない。

## 検証

`notification.schema.json`、`notification_list.schema.json`、`notification_action.schema.json`、負例fixture、Rust Notification Center単体試験、Desktop client／navigation試験、Schema／Conformanceへ接続する。
