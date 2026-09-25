# 回帰Caseの登録と一覧

回帰Case登録は、完了済みの通常対話結果を、ownerが明示的に再現評価用のCaseとして保存するC6の独立contractである。C5の評価Dataset revision、対話履歴、cache、previous state、LLM出力、Adapter metadataから自動的に昇格しない。

## 登録条件

登録はowner control経路だけが実行できる。Brokerは要求IDと要求hashを現在のin-memory対話制御へ照合し、通常対話の完了状態、`表示範囲=full`、結果証跡の永続監査、終了監査ID、成功または保留の結果を再確認する。古いhash、別の要求、対話承認、履歴承認、Profile、MCP metadata、Agent metadataは権限や登録資格にならない。

元の対話入力と応答本文は自動コピーしない。ownerは`入力方式=owner_explicit_redacted`を指定し、サニタイズ済みの入力、必要条件、禁止条件、期待状態、必要参照、期待経路を明示する。Brokerには既知のcredential markerを拒否する境界を置くが、任意の秘密値がないことを証明する検査ではないため、ownerのredaction責任を置き換えない。

## 保管と公開

private定義はWindowsの`ProtectedStore::Purpose::Regression`へ暗号化し、C5の`Purpose::Evaluation`と分離する。plaintext fallback、normal IPC（通常IPC）へのraw本文返却、Audit reasonへのprivate本文複写を行わない。公開receiptはCase ID、要求ID/hash、結果hash、終了監査ID、保管hash、件数および証拠種別だけを`hash_only`で返す。

登録のaccepted Auditはreceiptのhash-only projectionを含み、入力本文・条件・参照・期待経路を含まない。保管後のAudit確定に失敗した場合、Brokerは成功を返さず、残存暗号文を再利用しないRecoveryとして保管監査の再確認を要求する。

## 公開一覧

通常IPCは、登録時に監査へ確定した公開receiptからCaseのmetadataだけをページ取得できる。要求は版、0始まりcursor、1〜100件のlimitに限定し、一覧は監査順で返す。cursorは一覧位置であり、権限・承認・保管IDとして扱わない。各ページの対象暗号文についてBrokerがProtectedStoreのfile metadataと暗号文hashを照合し、欠落・改変・link・共有競合・監査不整合があれば部分一覧を返さず拒否する。

通常IPCの一覧は、Case ID、公開表示名、要求／結果の識別hash、終了監査ID、条件・参照の件数、作成時刻、保管hash、証拠種別だけを`metadata_only`として返す。入力本文、条件、必要参照、期待経路、復号値、owner資格、authority情報は返さない。一覧は内容閲覧・実行・承認を許可せず、保管状態の内部観測に限る。証拠種別は`INTERNAL_STATE`である。

## C5との境界と延期

C6のreceiptはC5のDatasetへ自動で追加されず、Dataset revisionや評価結果を生成しない。将来のC5 importは別のowner操作、別revision、再検証を持つ独立変更とする。通常画面は公開一覧だけを読み取る。Owner専用GUI登録、Caseの削除と中断Recovery、C5 Dataset revisionへの明示importは別の作業単位で接続する。

このcontractの成立は、実行系のhealth、Permission、Approval、Authority、security integrity、release readinessの証拠ではない。失敗時は要求修正、対話再確認、保管監査修復などのRecoveryActionへ接続し、入力本文をエラー、Audit、traceへ出さない。
