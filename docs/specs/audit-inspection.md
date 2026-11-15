# Broker永続監査一覧取得

## 主語と目的

主語はRust Brokerが保持する永続監査記録であり、DesktopのAudit Viewerはその限定読取consumerである。表示目的は、Brokerの現在の保存内容と検証状態を操作者が確認することである。Flutterの起動時Snapshotやローカル生成行は、実監査記録の代替にしない。

## 要求と応答

- 要求操作は`監査一覧取得`。認証済み通常Broker Sessionから行う読み取り専用要求であり、Owner操作、Permission、Approval、Credential、Trust、Task実行能力を生成・変更しない。
- 要求は`version=1`、`limit=1..200`、任意の`before_event_id`だけを受け付ける。未知field、未知版、範囲外件数、不正または現存しないcursorは拒否する。
- Brokerは固定Audit storeを読取り、全記録のevent hash chainとHMAC anchorを検証する。検証済み永続記録がBrokerの現在in-memory chainと一致しない、永続storeがない、または検証に失敗する場合、記録本文を返さず停止する。
- 応答は最新側から最大200件を選び、古い順に並べたbounded pageを返す。本文は`event_id`、`action`、固定分類の`result`、`payload_hash`、`previous_event_hash`、`event_hash`だけを含む。`request_id`、`reason`、元payload、秘密値、自由記述、pathは投影しない。eventにtimestampがないため生成・推定しない。
- `snapshot_head_event_id`と`verified_event_count`は取得時に検証したchain prefixを示す。要求自体も監査されるため、このheadには`received`記録までが含まれ、応答envelopeの`audit_event_id`で示す最終`accepted`記録は後続する。循環hashを作らないため、この差を保持する。
- `has_older_events`と`next_before_event_id`で過去pageへ移動できる。各要求は独立してBrokerが再検証し、新しいAudit eventを追記する。

## 表示・証拠範囲

DesktopはBroker modeでのみ明示取得・再取得し、失敗時は以前のpageを消去する。画面は「取得境界までBroker内chainとHMAC anchorを検証済み」と表示する。これはBroker固定store内の検証であり、外部tamper evidence、署名者同一性、Publisher署名を主張しない。Broker自由記述reasonやIPC error messageを画面へ表示しない。

## 停止条件

永続storeの不在・破損・検証不一致、応答operation／evidence／Audit ID／Schema不一致ではpageを一切表示しない。失敗はBrokerの既存Audit／fail-closed経路に残し、UIからの再試行は操作者が明示する場合だけ行う。
