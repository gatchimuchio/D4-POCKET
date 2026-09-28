# 実行系資源観測の意味正本

状態: C3 の接続済み契約。WindowsではRust Brokerのregistry、通常認証済みIPC、Flutter Runtime Centerの「資源」表示、実MINIDORA loopback listenerへの開発用実接続を実装した。Windows開発hostでの実接続・画面確認は、installed artifactまたは正式releaseの証拠ではない。

## 対象と非対象

実行系資源観測は、登録済みRuntimeの資源状態をBrokerが一回読み取るdiagnostic経路である。要求は runtime_resource_query.schema.json の「版」と「実行系ID」だけを受け付ける。PID、port、URL、Adapter名、履歴長、Permission、Approval、authority、metadata は要求から受け付けず、UIやAdapter metadataから観測対象を作らない。

この単位はread-onlyである。開始、停止、再起動、一時停止、再開、隔離、process kill、Runtimeへの通信、設定変更、履歴の永続化を行わない。C4は同じBroker所有registryを再利用できるが、ライフサイクルCapability、Permission、Approval、Audit、RecoveryをC3の観測から継承しない。

runtime_resource_observation.schema.json は観測結果の構造である。Schema適合、画面表示、内部cache、前回sample、fixtureは、現在の稼働、実測、現在権限、外部Runtimeの健全性を証明しない。

## Broker所有binding

Brokerは登録時に、検証済みの明示IPv4 loopback endpoint（addressとportの組）のlistener所有processをOSから解決する。別の127/8 addressで同じportをlistenするprocessを対象へ混ぜない。最初の所有PID・process作成時刻を得た後、同じendpointの所有PIDと作成時刻をもう一度照合し、両方が一致した有限snapshotだけをregistry内部へ保持する。endpoint自体はIPC応答へ返さない。UI、IPC payload、Adapter metadata、以前のPID、短期履歴、設定fileからPIDまたはendpointを指定・復元しない。

各観測では、保持した同一endpointをprocess読取の前・中間・返却直前に再解決し、そのたびに登録済みPIDと一致することを確認する。各process sampleの作成時刻も登録snapshotと一致しなければならない。listenerの消失、別processへの移動、endpoint解決不能、PID再利用、作成時刻不一致は `binding_mismatch` とし、processを読めない場合は `exited` とする。登録時に結合を確定できなければ `unbound`、非対応platformは `unsupported` とする。失効時は内部endpoint、PID、作成時刻、前回CPU sample、短期履歴を破棄し、PIDと作成時刻をnullにする。再登録なしに別processへ追従しない。このとき全metricを unknown にし、「短期履歴」を空にして過去sampleを返さない。過去の成功sample、0、推定値、aggregate値で穴埋めしない。

「登録時刻UnixMillis」と「登録監査ID」はBrokerが実行系登録を受理した時点の監査対応である。各観測の監査対応は「観測監査ID」で返す。IPC応答の audit_event_id は「観測監査ID」と完全一致しなければならない。accepted監査eventのpayload_hashは、この「観測監査ID」を含むcanonical観測bodyのSHA-256でなければならず、受信eventのquery hashとは区別する。これにより監査chainと返却したPID結合、時刻、metric、履歴を照合できる。どちらも観測値の正しさや現在権限を単独で保証しない。小数metricを含むaccepted body hashの照合では、RustがIPC応答へ出力した小数tokenを保持してcanonical JSONへ再正本化する。Python検証は通常のfloatへ落として再encodeする方式を使わず、例えばRustの`1e-6`をPythonの`1e-06`へ変えてhashを変化させてはならない。

## metricの厳格な状態

「計測」と「短期履歴」の各metricは、次の二つのobjectのいずれか一方だけである。

measured は「状態、非負数値の値、証拠種別」を持つ。unknown は「状態、nullの値、証拠種別、理由」を持つ。

両方とも未知fieldを拒否する。unknownに0、空文字、前回値、推定値、欠損を入れることはできない。measuredに理由を混入できない。したがって実測した失敗要求数0はmeasuredとして許される一方、未観測の失敗要求数を0で表示することは許されない。

証拠種別=LIVE_RUNTIMEはOSまたは接続済みRuntimeからこの観測で得た値、INTERNAL_STATEはBrokerが保持する要求集計だけを表す。後者はOS process資源や外部Runtimeの稼働を証明しない。

現在のWindows first最小証拠は、作成時刻に結合したPID、稼働時間、累積process CPU時間、二観測間差分からのCPU利用率、working set、private bytesである。結合が `bound` のときだけ、これらを `LIVE_RUNTIME` として返す。稼働時間、累積process CPU時間、CPU利用率は、clock差分、時刻変換、または数値精度を安全に確保できない場合に、推定値や0で補完せず `LIVE_RUNTIME` のunknownにする。working setとprivate bytesは現在のprocess sampleから得たmeasuredだけを返す。CPU利用率Percentは、100ns単位のprocess CPU時間差分を高精度の経過wall timeとavailable_parallelismで割り、100を掛ける。すなわち delta process CPU / elapsed wall / logical processor capacity × 100 とする。CPU差分が作れない初回sample、時間差不足、0〜100の範囲外、または安全な数値精度を確保できない場合はunknownにする。

Disk I/O、Network I/O、GPU利用率、VRAMは、対応するprocess別の根拠sourceが接続されるまで `INTERNAL_STATE` のunknownにする。process aggregate I/O、host全体のnetwork、推定GPU値をこれらの項目へ転記しない。「処理中要求数」と「失敗要求数」はBrokerが実際に保持する要求集計を得られる場合だけ `INTERNAL_STATE` のmeasuredとして返す。

現在の対話開始・終了時刻は監査用の秒精度であり、ミリ秒応答時間の実測ではない。そのため `平均応答Millis` は高精度の単調時計を記録するまで常に `INTERNAL_STATE` のunknownとし、秒時刻の差や0で補完しない。短期履歴の `エラー率Percent` は、完了要求数を分母、失敗要求数を分子として小数を保持して計算する。完了要求がない、失敗数が完了数を超える、または安全な数値精度を確保できない場合はunknownにする。

## 履歴

「短期履歴」はCPU、RAM、Network、Latency、Error rateの短期sampleだけを持つ。最大60件で、Broker内のbounded memoryに限る。永続監査、再実行、Permission、Approval、現在稼働状態を短期履歴から生成しない。観測失敗、binding不成立、画面離脱、Broker再起動の扱いは実装時にfail-closedで定義し、過去sampleを現在の実測へ昇格しない。

## 統治と表示

観測は Capability runtime.resource.observe、Runtimeに束縛した Permission permission.runtime.resource.observe、read-onlyの承認状態 not_required、AuditEvent、RecoveryAction recover-runtime-resource-binding に対応づける。これらはBrokerが固定し、UI、Adapter metadata、query、historyから書換えない。監査確定に失敗した場合は観測成功を返さず、値を推測しない。

boundのIPC応答は evidence_source=LIVE_RUNTIME とする。unbound、binding_mismatch、exited、unsupported のIPC応答は evidence_source=INTERNAL_STATE とする。これは応答全体の分類であり、各metricの証拠種別を上書きしない。

Flutterは検証済みのRuntimeResourceObservationを表示するだけであり、既存Shell snapshot、固定diagnostic値、fixture、前回表示からC3の実測値を作らない。unknownは理由とともに未観測として表示する。

Mobileの資源画面は`IndexedStack`の非表示中やDevice Link切断中に観測要求を開始しない。接続中に画面を選択した場合だけ観測し、画面離脱・接続断で現在表示と進行中応答を無効化する。画面へ戻った後は現在接続で新規観測する。Runtimeごとの要求は逐次・上限16件で、一度に一回だけ実行し、周期pollingは行わない。

## 契約と検証範囲

構造contractは runtime_resource_query.schema.json と runtime_resource_observation.schema.json、IPC operationは「実行系資源観測」とする。conformanceはPID入力拒否、strict metric union、現在C3で接続済みのmetricごとの状態・証拠種別の組合せ、binding不成立時の全unknown/短期履歴空、短期履歴60件上限、固定統治field、IPC enumを検査する。

構造contractだけで実装や証拠は成立しないが、C3ではRust broker実装、Windows API実測、通常IPC dispatch、Flutter client/UI、隔離した実MINIDORAへのWindows開発用実接続を接続した。開発用のsource worktree、debug build、fixture、画面操作だけを、clean source commitに結合したinstalled artifactまたは正式releaseの証拠へ昇格しない。Android物理実機検証は凍結中であり、Windows以外の物理実機証拠はrelease evidence deferredとして別途必要である。

- item: C3のinstalled/release証拠とWindows以外の物理実機証拠
  classification: release_blocker
  reason: Windowsの開発用実接続は、clean source commitに結合したinstalled artifact、installation path、artifact hash、strict release条件を確認する証拠ではない。Android物理実機検証は凍結中であり、Windows以外の物理実機証拠も未取得である。
  required_action: 正式releaseを検討する時点で、clean source commitからinstalled artifactを生成してWindowsの実画面・実接続を再確認し、artifact hashとinstallation pathを記録する。Windows以外の物理実機は各platformで必要な証拠を取得するまでdeferredとし、Android物理実機はownerが凍結を解除するまで実行しない。
  blocks_release: yes
