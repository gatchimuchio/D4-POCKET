# C28 長時間運用検証

## 責任

この文書は、C28の開発用検証器が実行する範囲と、実installed製品へ昇格できない範囲を定める。C28の最低要求は、8時間、大量対話、Runtime再起動、Broker再起動、network切断・再接続、memory/resource leak観測である。

## 検証器

`tooling/long_run_validation.py` は、毎回一時directoryへ実Brokerを起動し、通常IPCとowner control IPCを分離して実行する。RuntimeはMINIDORA公開APIと同じHTTP境界を持つlocalhost fixtureであり、製品Runtimeや秘密値を使用しない。fixtureは再起動時の残留handlerを避けるため直列HTTP serverとして動作し、停止時に要求処理の完了を待つ。

1. 反復対話を開始し、owner承認、結果取得、終了、履歴の`limit=100` bounded読取を行う。
2. Runtime相当としてHTTP fixtureを停止・再起動し、Brokerを保持した再接続を確認する。
3. 開発用lifecycle fixtureをowner承認経路から再起動する。
4. Brokerをshutdownし、同一永続storeから再起動してnonce再利用を避けた新規IPCを実行する。
5. HTTP fixtureを停止した対話が成功へ昇格しないことを確認し、fixture復帰後の対話成功を確認する。
6. Brokerのworking set、永続store bytes、対話応答時間、HTTP要求数をbounded sampleとして記録する。

C28検証器自身の記録量も上限を持つ。資源sampleは最大512件、HTTP結果履歴は直近32件、trace fixtureの参照記録は最大128件とする。HTTP pathは`health`、`capabilities`、`chat`、`trace`、`other`の固定route labelへ正規化し、trace IDや任意pathを履歴・sampleへ蓄積しない。HTTP要求総数は単調counterとして保持する。上限を越えたtrace参照はfixture上で期限切れとして扱う。このbounded telemetryは検証器のメモリと証拠JSONの増大を制限するためのもので、製品履歴保持契約ではない。

Broker再起動時に検証器自身が所有する一時normal／owner資格fileを削除する。既に存在しないfileは正常とする。削除が`PermissionError`となった場合は最大20回、50ms間隔で再試行し、成功した再試行回数を`Broker資格file削除再試行数`として報告する。上限後も削除できない場合はC28をfailedにする。資格file cleanup以外のBroker操作、Audit、store失敗は再試行で成功へ変換しない。

Runtime fixtureのHTTP resultは、status lineとbodyのwrite／flush後に`server_flush_succeeded`として記録する。書込み例外時は本文や資格を含めず例外classだけを`server_write_failed:<class>`として記録して再送出する。この証拠はfixture server側のflush結果であり、client processがresponseを受信・検証したことまでは証明しない。

C28のdevelopment Broker起動時だけ`GUI_SHELL_C28_DIAGNOSTICS=1`を子processへ渡す。debug buildのMINIDORA AdapterはHTTP transport失敗時に、固定route分類、固定stage、標準I/O error kindだけをstderrへ最大32行記録する。URL、trace ID、request／response body、credential、error messageは記録しない。検証器は許可値と形式が一致する行だけを最大32行reportへ採用し、Broker stderr全文も例外messageもreportへ複写しない。失敗reportの`エラー`は例外型名と採用済みtransport分類だけを含む。release buildでは診断は無効である。診断は失敗位置を分類する補助証拠で、対話成功やclient受信を証明しない。

既定の短時間smokeは通常検証から実行できる。8時間の実測は次で明示的に開始する。

```text
python tooling/long_run_validation.py --duration-hours 8 --output <一時の証拠JSON>
```

`--output`のJSONは、実行中は`状態=running`、終了後は`状態=passed`または`failed`としてatomicに更新する。出力には接続資格、credential実値、対話本文、owner秘密を保存しない。

## 証拠分類と限界

- Broker IPC、Broker再起動、実際の監査・履歴storeは`LIVE_RUNTIME`の範囲で観測する。
- Runtime応答はlocalhostの検証fixtureであり、Runtime側は`FIXTURE`である。
- working setとstore bytesは`INTERNAL_STATE`／host観測であり、リーク不存在の保証や製品SLAではない。
- 5秒未満の検証、短時間smoke、正常なexit codeだけで8時間成立を主張しない。
- Windows installed appのcold／warm start、GPU／VRAM、外部Agent・MCP・A2Aの実負荷、実機のOS lifecycle、正式release artifactは別のrelease evidenceで検証する。

資源値が取得できない場合は`unknown`として記録し、0へ置換しない。失敗した対話は、接続断による予定内失敗と予期しない失敗を分け、後者は検証失敗として終了する。
