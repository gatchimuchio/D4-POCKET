# C29 障害注入検証

## 目的

C29は、障害を成功、権限、承認、監査、復旧完了へ昇格させないことを、実Broker経路と境界付きfixtureで確認する開発用検証単位である。

対象は次の8件とする。

```text
Runtime crash相当
Broker crash
MCP timeout
A2A timeout
credential unavailable
disk full simulation
audit failure
corrupt state
```

## 検証経路

`tooling/failure_injection_validation.py`は、各caseに一時storeと実Rust Brokerを起動し、通常資格とowner資格を分離したIPCを使う。

- Runtime crash相当: localhost Runtime fixtureを停止し、対話を通信失敗として確定した後、再接続を確認する。
- Broker crash: Broker processを停止し、停止後のIPCを失敗させ、明示再起動後のhealthだけを確認する。
- MCP timeout: 実MCP stdio Adapterへ応答停止fixtureを接続し、`mcp_timeout`を拒否へ射影する。
- A2A timeout: localhostのA2Aカード応答を検証用fixtureで停止させ、`a2a_timeout`を拒否へ射影する。
- credential unavailable: 必須Credentialを持つMCP接続を、実値注入なしに拒否する。
- disk full simulation: regular fileをstore pathへ置き換え、書込不能をBroker起動拒否として観測する。実ディスク容量枯渇の証拠ではない。
- audit failure: `audit.jsonl`を一時的にdirectoryへ置き換え、監査追記失敗を`Suspended`へ射影する。
- corrupt state: `a2a_connections.json`をmalformedにして、Broker再起動を拒否する。

## 証拠境界

Broker IPC、Broker process、Rust Adapterは`LIVE_RUNTIME`、MCP／A2Aの応答停止は`FIXTURE`、temporary storeの破損・書込不能は`INTERNAL_STATE`として記録する。fixtureを使った結果をinstalled製品、実外部Runtime、実外部MCP／A2A、Windows release evidenceへ昇格させない。

応答本文、資格値、Credential実値、接続先秘密、監査raw payloadは証拠JSONへ保存しない。失敗時は失敗状態と境界付きの分類だけを返す。

## 実行

```text
python tooling/failure_injection_validation.py
```

通常の統合検証には、短時間のC29 smokeとしてこのコマンドを接続する。C29 smokeのPASSは、実installed製品の障害耐性、8時間運用、外部サービスのSLA、正式releaseを意味しない。

## 残存範囲

- `release_blocker`: Windows installed productの実process／実Runtime／実MCP／実A2A障害証拠、外部資格 unavailable、実容量枯渇、owner GO、C0-C34全数完成。
- `known_limitation`: disk fullはregular file置換による書込不能simulationであり、物理ディスク容量の枯渇を再現しない。
