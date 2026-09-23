# C13 観測センター

## 目的

C13は、性能・挙動・運用状態を確認するための内部観測面を追加する。観測はAuditの責任・安全・証拠を置き換えず、観測結果からPermission、Approval、Authority、Capabilityを生成しない。

## 現行範囲

- Rust Broker内にboundedなSpan、Trace、Metric modelを保持する。
- BrokerがAudit eventを確定した処理について、`Instant`による確定処理時間をSpanへ記録する。
- 通常認証済みIPCの`観測一覧`で、上限とTraceID filterを受け付ける。
- Desktopの観測センターはMetric、Span、責任分離、証拠種別を表示する。
- 観測はsession内memoryに限定し、Audit永続storeとは別管理にする。
- responseの証拠種別は`INTERNAL_STATE`に固定する。

## 境界

Auditのreason、payload hash、metadata、credential、raw contentはSpan、Trace、Metric、Desktop表示へ投影しない。callerが観測を登録する操作はない。OpenTelemetry exportは未対応であり、外部送信やexport adapterを実行しない。

現行の観測範囲は`broker_audit_finalization`だけである。これはOS resource、Runtime health、外部通信、全requestのwall-clockを証明しない。Runtime resource observationは既存C3の別契約として扱う。

## bounded要件

- Broker内のSpan保持は最大1024件。超過時は最古の観測を破棄する。
- IPC返却は最大256Span、256Trace、16Metricとする。
- Metric値が測定不能な場合は`unknown`とし、測定不能を0へ変換しない。
- 観測一覧の監査は通常のBroker Auditへ記録するが、そのAuditのreasonは観測bodyへ戻さない。

## 後続範囲

C14 Trace Inspectorでのwaterfall、親子Span、詳細な失敗経路表示は本単位に含めない。OpenTelemetry export adapter、外部collector接続、長期保存、8時間運用証拠も別作業であり、本単位の完成主張に含めない。
