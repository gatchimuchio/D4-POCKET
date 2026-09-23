# 回帰Case登録

回帰Case登録は、完了済みの通常対話結果を、ownerが明示的に再現評価用のCaseとして保存するC6の独立contractである。C5の評価Dataset revision、対話履歴、cache、previous state、LLM出力、Adapter metadataから自動的に昇格しない。

## 登録条件

登録はowner control経路だけが実行できる。Brokerは要求IDと要求hashを現在のin-memory対話制御へ照合し、通常対話の完了状態、`表示範囲=full`、結果証跡の永続監査、終了監査ID、成功または保留の結果を再確認する。古いhash、別の要求、対話承認、履歴承認、Profile、MCP metadata、Agent metadataは権限や登録資格にならない。

元の対話入力と応答本文は自動コピーしない。ownerは`入力方式=owner_explicit_redacted`を指定し、サニタイズ済みの入力、必要条件、禁止条件、期待状態、必要参照、期待経路を明示する。Brokerには既知のcredential markerを拒否する境界を置くが、任意の秘密値がないことを証明する検査ではないため、ownerのredaction責任を置き換えない。

## 保管と公開

private定義はWindowsの`ProtectedStore::Purpose::Regression`へ暗号化し、C5の`Purpose::Evaluation`と分離する。plaintext fallback、normal IPC（通常IPC）へのraw本文返却、Audit reasonへのprivate本文複写を行わない。公開receiptはCase ID、要求ID/hash、結果hash、終了監査ID、保管hash、件数および証拠種別だけを`hash_only`で返す。

登録のaccepted Auditはreceiptのhash-only projectionを含み、入力本文・条件・参照・期待経路を含まない。保管後のAudit確定に失敗した場合、Brokerは成功を返さず、残存暗号文を再利用しないRecoveryとして保管監査の再確認を要求する。

## C5との境界と延期

C6のreceiptはC5のDatasetへ自動で追加されず、Dataset revisionや評価結果を生成しない。将来のC5 importは別のowner操作、別revision、再検証を持つ独立変更とする。通常画面の表示とowner CLIはこのcontractの要求を開始できるが、GUIからのprivate登録導線・Case一覧・削除Recoveryは別の作業単位で接続する。

このcontractの成立は、実行系のhealth、Permission、Approval、Authority、security integrity、release readinessの証拠ではない。失敗時は要求修正、対話再確認、保管監査修復などのRecoveryActionへ接続し、入力本文をエラー、Audit、traceへ出さない。
