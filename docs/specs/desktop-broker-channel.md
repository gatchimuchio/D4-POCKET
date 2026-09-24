# Windows Desktop Flutter–Broker要求路

状態: rev2契約。実装・実行証拠が成立するまでは未完成。

## 1. 責任

Windows DesktopのFlutterは表示と利用者入力を担う。Flutter/Dartからendpoint file、Broker資格、Socket、networkへ直接到達してはならない。Rust Security Brokerが操作の認可、Approval、Audit、Recoveryの唯一の正本であり、Windows Runnerは要求を運ぶだけである。authority-sensitiveな処理へFFIを使用しない。

```text
Flutter/Dart
  → Windows Runner MethodChannel（要求JSON文字列のみ）
  → Rust起動器が所有する一時名前付きpipe
  → 起動器が生成した当該Flutter processのPID照合
  → Rust内transport relay
  → 既存authenticated loopback TCP Broker経路
  → 既存normal-role検証・操作別判定・Audit
```

名前付きpipeはUI要求を既存Brokerへ運ぶtransportであり、別の権限判断、Owner経路、operation実装を追加しない。relayは要求を既存Brokerへ渡す前に、許可field集合を検査し、Brokerが発行した当該起動sessionの`session_id`だけを補う。認証secretはRustプロセス内に留め、Flutter/DartとWindows Runnerへ渡さない。

## 2. 要求形式

FlutterがMethodChannel `gui_shell/broker` の`request` methodへ渡す値は、UTF-8 JSON objectを表す文字列である。JSONの形は`specs/desktop_broker_channel_request.schema.json`を正とする。`session_id`、`session_secret`、`credential_role`、endpoint、Owner資格、Authority宣言はchannel要求のtop-levelに存在してはならない。未知fieldも拒否する。

要求本文と許可されたoperation-specific payloadは、操作契約で必要な場合に限り受け付ける。channel、Runner、relayは要求本文をlog、trace、error、Auditへ複写しない。metadataは信頼しない説明dataであり、permission、Approval、Owner、host、runtime、sessionを昇格させない。Brokerは既存のnormal IPCと同じ入力正規化、Schema、replay/stale検査、操作別authority、Audit、Recoveryを適用する。

## 3. Pipeの接続境界

- pipe名はRust起動器が起動ごとに生成する接続先札であり、秘密・Capability・Permission・Approvalではない。
- Rustが作ったserverだけがlistenし、remote client接続を拒否する。
- Rustは`GetNamedPipeClientProcessId`でpipe接続元を観測し、同じ起動器が起動したFlutter child processのPIDと完全一致する場合だけ本文を受け付ける。
- PID不一致、未設定、pipe名不正、JSON不正、上限超過、期限超過、Broker停止はfail-closedとする。失敗時にTCP、snapshot、別endpoint、Owner資格へfallbackしない。
- Runnerはpipeの接続・frame送受信だけを担い、要求を解釈して権限を付与せず、認証secretを保持しない。
- relayがpipeで受けた要求を処理するときは、必ず既存のauthenticated loopback TCP Brokerへnormal secretで再接続する。Brokerが停止・拒否・Audit失敗を返した場合、その応答を変更せずUIへ返す。

## 4. 上限・応答・復旧

要求は一接続一JSON lineとし、UTF-8 byte上限はBrokerの`max_request_bytes`を超えてはならない。応答は既存`ipc_response`を一JSON lineで返し、要求ID・operationが一致しない応答、空応答、不正JSON、4 MiB超過、5秒期限超過をUI clientが拒否する。Runnerまたはpipeの不通はBroker権限を推定せず、製品UIを`broker_unavailable` / `suspend`として扱う。通常TCPへのDart fallbackは禁止する。

## 5. 必須適合確認

Schema/fixtureは接続要求の形だけを証明し、Windows pipe、PID照合、既存Brokerへの転送、実際の権限判断を証明しない。production pathの完成には少なくとも次の`LIVE_RUNTIME`確認が必要である。

- clean Windows buildのRust起動器 → Flutter Runner → pipe → 既存Brokerの正常往復と永続Audit。
- normal requestでOwner-only操作が拒否されること、およびchannelからOwner資格を指定できないこと。
- 別process PID、別起動pipe、top-level authority/session/credential field、malformed JSON、size超過、stale/replay nonce、Broker停止の拒否とfail-closed表示。
- pipe障害時に直接Socket・endpoint file・snapshotへfallbackしないこと。
- Flutter production sourceにendpoint file読取、Broker secret保持、direct Socket/network接続が残らないこと。

## 6. 移行境界

現行TCP BrokerとOwner CLI経路は維持する。既存のFlutter TCP clientを削除するまでは本契約をproduction適合とみなさない。Android/iOSのDevice Linkは別Transport/別証拠面であり、このWindows Desktop契約によって完了扱いしない。Flutter内のSetup Doctor、snapshot、export等のfilesystem/network/process利用も独立に調査・移譲する。

この契約の作成は`release_blocker`を解消しない。Schema・fixture・conformanceはCONFIG/FIXTURE証拠に限られ、Runtime接続は別途実装・実測する。
