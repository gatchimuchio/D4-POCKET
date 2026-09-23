# C14 追跡情報閲覧

## 目的

C14は、C13の内部観測からTraceの処理段階を読み取り、開始、終了、所要時間、状態、親Span、エラー分類をboundedに表示する読み取り専用画面である。Trace表示は性能・挙動・運用状態の観測であり、Auditの責任・安全・証拠を置き換えない。

## 現行範囲

- DesktopにTrace Inspectorを追加する。
- 既存の通常認証済み`観測一覧`を再利用し、TraceID filterと手動更新を提供する。
- Spanの開始・終了を同一時間軸へ射影し、処理時間waterfallとして表示する。
- status、duration、parent、error分類、関連Audit IDを表示する。
- Broker内部観測の範囲と証拠種別を表示し、snapshotをTraceの根拠にしない。

## 対象段階の証拠境界

現時点でproduction観測が接続されている対象はBrokerだけである。Runtime、Adapter、Tool、外部通信はTrace Inspectorの分類対象として将来拡張できるが、実測Spanがない間は未接続として表示し、Broker観測を他段階へ複製しない。A2AはC15以降の契約・接続作業に従う。

## 安全境界

Trace Inspectorは読み取り専用で、Trace表示からPermission、Approval、Authority、Capability、Credentialを生成しない。Audit reason、payload、metadata、秘密値、raw contentは表示しない。Traceのparent、error、関連Audit IDは観測の限定射影であり、現在の承認や実行権限を再構成しない。

## 未成立範囲

外部collector、OpenTelemetry export、Runtime／Adapter／Tool／外部通信の実測Span、A2Aの実接続、長期Trace保存、8時間運用証拠は別工程である。本単位の画面表示を、それらのproduction接続や正式release証拠とは扱わない。
