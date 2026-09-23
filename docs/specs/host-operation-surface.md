# Host操作面

## 位置付け

C18のHost操作面は、C17のHost registryをD4 Pocket Desktopへ接続する。`Host一覧`は通常認証済みIPCから取得し、Hostの表示名、Platform、接続状態、Trust、Runtime／Agentのboundedなsummaryだけを表示する。

Host metadataの証拠種別は`INTERNAL_STATE`であり、未接続HostのRuntime／Agent一覧をliveであるかのように生成しない。登録時のRuntime／Agent件数はsummaryであって、個別一覧の観測証拠ではない。

## Host切替

`Host切替`は通常IPCの表示コンテキスト操作であり、Rust BrokerがHost IDをregistryと照合して監査する。切替はPermission、Approval、Authority、Credentialを生成しない。前のHostの承認、Permission、Authorityを新しいHostへ持ち越さず、receiptは`承認状態=not_reused`、`authority_strip=true`、`metadata_only`に固定する。

存在しないHost、未知field、owner control経路からの切替要求は拒否する。Brokerが切替を監査できない場合、UIは切替成功を表示しない。

## Runtime／Agent一覧

選択Hostが現在のBroker観測Host IDと一致し、現在のBrokerが返した観測を利用できる場合だけ、DesktopはそのRuntime／Agentを現在観測として表示する。それ以外のHostではregistryの件数と`未観測`を表示し、推測した名前や状態を追加しない。

## 残存範囲

- live Host再接続、Trust検証、Runtime／Agentの個別remote discoveryは`release_blocker`。
- Host間Permission転用拒否とHost切替receiptの監査はC18で検証する。
- C18のHost操作面だけでHostの実接続やD4 Pocket全体のrelease readinessを主張しない。
