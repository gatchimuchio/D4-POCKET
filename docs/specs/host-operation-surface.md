# Host操作面

## 位置付け

C18のHost操作面は、C17のHost registryをD4 Pocket Desktopへ接続する。`Host一覧`は通常認証済みIPCから取得し、Hostの表示名、Platform、接続状態、Trust、Runtime／Agentのboundedなsummaryだけを表示する。

Host metadataの証拠種別は`INTERNAL_STATE`であり、未接続HostのRuntime／Agent一覧をliveであるかのように生成しない。登録時のRuntime／Agent件数はsummaryであって、個別一覧の観測証拠ではない。

## Host切替

`Host切替`は通常IPCの表示コンテキスト操作であり、Rust BrokerがHost IDをregistryと照合して監査する。切替はPermission、Approval、Authority、Credentialを生成しない。前のHostの承認、Permission、Authorityを新しいHostへ持ち越さず、receiptは`承認状態=not_reused`、`authority_strip=true`、`metadata_only`に固定する。

存在しないHost、未知field、owner control経路からの切替要求は拒否する。Brokerが切替を監査できない場合、UIは切替成功を表示しない。

## Runtime／Agent一覧

選択Hostを現在のBroker実行Host（local）として扱えるのは、product初期化で受理した`snapshot_source=broker`のホスト能力観測とHost IDが一致する場合だけである。`mock`、fallback、診断、未検証snapshot、表示名、Host metadataだけではlocal／remote区分やlive状態を推定しない。

一致した場合だけ、そのHostのRuntime／Agentを現在のBroker観測として表示する。一致しない登録Hostは別Hostとして表示し、remote接続と個別Runtime／Agent状態は未観測のままにする。registryのRuntime／Agent件数はmetadata summaryとして別表示し、個別live一覧へ展開しない。snapshot sourceがBroker確定経路でない場合は、全HostのRuntime／Agentを未観測として扱う。

Host registryの接続状態・Trustは登録metadataとして表示し、Broker現在HostのLIVE_RUNTIME観測と混同しない。ホスト能力画面の`degraded`は観測された機能状態の投影であり、Permission、Approval、Authority、Credential、接続Trustを生成しない。

## 残存範囲

- live Host再接続、Trust検証、Runtime／Agentの個別remote discoveryは`release_blocker`。
- Host間Permission転用拒否とHost切替receiptの監査はC18で検証する。
- C18のHost操作面だけでHostの実接続やD4 Pocket全体のrelease readinessを主張しない。
