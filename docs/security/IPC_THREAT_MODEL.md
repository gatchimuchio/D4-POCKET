# IPC脅威モデル

状態: Phase 3の本番ブローカープロセスおよびFlutterブローカー製品経路を開始済み
日付: 2026-06-03
範囲: Flutter UIからRust Security Brokerを経てAdapter / Runtime IPCへ至る経路

## 1. 境界

GUI-Shellのauthority-sensitive経路は、Flutter UI processではなくRust Security Broker processを境界にする。
Authority-sensitiveなFlutterとRustの接続にはrestricted IPCを使用する。approval-token、authority decision、external command dispatch、audit finalization、credential、およびrecovery authorizationにはFFI/direct bridgeを使用しない。

~~~text
Flutter GUIプロセス
  -> 制限済みIPC要求
Rust Security Brokerプロセス
  -> ブローカーが検証したenvelope
Adapter / External Runtime
~~~

この文書はbroker production pathのthreat modelである。Flutter product entryはbroker-mediated pathへ切り替わったが、command dispatchはまだsuspendedであり、`authority_cutover_status=not_active`で、Windows installed-pathの`LIVE_RUNTIME` proofは未取得である。

## 2. 脅威と必須対処

| 脅威 | 必須対処 | 現在のskeleton範囲 | リリース分類 |
| --- | --- | --- | --- |
| 偽装UI要求 | session id、nonce、payload hash、operation、IPC auth secretを検証し、失敗時はrejected / auditedとする。 | authenticated loopback IPCとtyped envelope validationを実装済み。Flutter product clientはbroker session fileを使用する。 | Windows installed pathで証明するまで`release_blocker aggregate_of=windows_broker_installed_smoke` |
| 再送された承認要求 | nonce replayを拒否し、broker auditへ記録する。 | persisted nonce storeはbroker restart後のreplayも拒否する。 | Windows installed pathで証明するまで`release_blocker aggregate_of=windows_broker_installed_smoke` |
| 偽造されたランタイムmetadata | authority-like key/valueを検出し、adapter metadataをauthorityとして扱わない。 | broker metadata scannerおよびPython-oracle parity pathで実装済み。 | Windows installed pathで証明するまで`release_blocker aggregate_of=windows_broker_installed_smoke` |
| 不正形式のenvelope | request_id、operation、payload_hash、nonceを必須とし、fail closedにする。 | Rust brokerで実装し、Rust IPC testで検証済み。 | installed proofまで`release_blocker aggregate_of=windows_broker_installed_smoke` |
| 古い、または不正形式の`issued_at` | RFC3339およびfreshness windowを検証し、失敗時はrejected / auditedとする。 | 300秒のfreshness windowをRust IPC integration testで網羅。 | installed proofまで`release_blocker aggregate_of=windows_broker_installed_smoke` |
| 永続状態を利用できない場合 | audit/replay/session persistenceが必要なときにstoreがなければhealth suspend / operation rejectとする。 | durable file storeを実装済み。unavailable modeはfail-closedのままである。 | installed proofまで`release_blocker aggregate_of=windows_broker_installed_smoke` |
| audit anchor keyとlogの同居 | HMAC anchorはlocal corruption / partial tamper detectionとして扱う。同一権限でkey / anchor / logを同時更新できる相手に対する外部完全性証明としては扱わない。 | `audit_anchor.json`と`audit_anchor.key`を実装済み。external anchor / Windows key-protection proofは未完了。 | product release proofがWindows ACL / DPAPI / external-anchor handlingを定義するまで`release_blocker registry_id=audit_anchor_external_tamper_evidence_proof` |
| authority-like key/valueの別名 | case、zero-width、camelCase、separator、およびaliasを拒否する。 | broker metadata scannerとparity harnessで実装済み。 | installed proofまで`release_blocker aggregate_of=windows_broker_installed_smoke` |
| Unicode、case、zero-widthによるnormalization迂回 | Unicode、case、zero-widthをnegative testへ含める。 | NFKC、zero-width、case、camelCaseをRust unit scopeおよびPython oracle parityで網羅。 | installed proofまで`release_blocker aggregate_of=windows_broker_installed_smoke` |
| 古いsession | session mismatchをrejected / auditedとする。 | production broker-serverはprocessごとにsessionを生成し、Flutter product testはfail-closed stale session handlingを網羅する。 | installed proofまで`release_blocker aggregate_of=windows_broker_installed_smoke` |
| brokerを利用できない場合 | Flutterはauthorityを推論してはならず、UIはfail-closed / SUSPEND状態へ入らなければならない。 | `ShellCoreClient.product()`は`broker_unavailable` SUSPEND snapshotを返し、local JSON authorityを読まない。Flutter product testはWindowsの`flutter.bat`を通して合格し、`tooling/release_runtime_assertions.py --check`はfail-closed test coverageの存在を検証する。 | installed proofまで`release_blocker aggregate_of=windows_broker_installed_smoke` |
| 承認中のbroken pipeまたはcrash | approval finalizationを完了してはならず、RecoveryActionを必須とする。 | 文書化だけ。 | `release_blocker aggregate_of=windows_broker_installed_smoke` |
| `audit append failure`（監査追記の失敗） | audit appendが失敗した場合、brokerはfinalizationを遮断しなければならない。 | 文書化だけ。 | `release_blocker aggregate_of=windows_broker_installed_smoke` |
| keychainを利用できない場合 | credential-gated operationはfail closedにしなければならない。 | 文書化だけ。 | credential-gated operationをv1.0へ含めるまでは`post_v1_scope` |

## 3. 証拠源の規則

- `CONFIG`: `specs/`配下のJSON Schemaファイル。
- `INTERNAL_STATE`: Rust broker unit testおよびtyped envelope validation。
- `LIVE_RUNTIME`: Rust broker process integrationと、将来のWindows installed-path evidence。
- `EXTERNAL_EVIDENCE`: 将来のsigned artifact / installed path evidence。
- `FIXTURE`: `examples/contracts/`配下のexampleおよびnegative fixture。
- `CONFIG / FIXTURE / LIVE_RUNTIME`の混合表明: `tooling/release_runtime_assertions.py --check`は、現行product entry、Python authority process startupがないこと、FFI/direct bridge tokenがないこと、`broker fail-closed test coverage`、およびlocal broker restart/crash persistence testの存在を検証する。

`CONFIG`、`INTERNAL_STATE`、`FIXTURE`の結果は、`LIVE_RUNTIME` broker proofへ昇格しない。

## 4. Fail-Closed要求

- 不正形式の要求: 拒否して監査する。
- 不正形式または古い`issued_at`: 拒否して監査する。
- 古いsession: 拒否して監査する。
- 再送nonce: 拒否して監査する。
- authority metadata: 拒否して監査する。
- リリース分類: unavailable persistence / broker caseについてWindows installed proofが存在するまでは`release_blocker`とする。
- migration前のcommand envelope dispatch: suspendして監査する。
- `broker audit append failure`（ブローカー監査追記の失敗）: finalizationを遮断する。
- 必須のpersistent stateを利用できない場合（installed proofまでは`release_blocker`）: healthをsuspendし、operationを拒否する。
- brokerを利用できない場合（installed proofまでは`release_blocker`）: Flutterはunavailable stateを表示し、authority decisionを行わない。

## 5. 監査アンカー脅威モデル

`audit_anchor.json`はaudit event countとhead event hashを`audit_anchor.key`でHMAC化する。これはbroker store内のaudit log truncate、partial rewrite、anchor mismatch、およびlocal corruptionを検出するための`INTERNAL_STATE / LIVE_RUNTIME` local evidenceである。

このanchorはexternal notarization、signed release artifact、remote transparency log、またはOS key-protection proofの代替ではない。`audit_anchor.key`、`audit_anchor.json`、`audit.jsonl`を同一権限で同時に書き換えられるsame-user attackerは、整合するHMAC anchorを再生成できる。administrator / root adversaryに対する改ざん耐性も、このlocal file-store anchorだけでは主張しない。

Windows product release claimでは、installed app path上でkey / anchor / audit logの配置、ACL、DPAPIなどのOS key protection、またはexternal anchor / signed evidenceのどれを採用するかを明示する必要がある。そのevidence sourceを`EXTERNAL_EVIDENCE`またはWindows installed-path `LIVE_RUNTIME`として検証しなければならない。

## 6. 現在の制限

~~~yaml
- item: Flutter broker unavailable installed proof absent
  classification: release_blocker
  registry_id: windows_broker_installed_smoke
  reason: Flutter product codeはbroker unavailable / auth / stale / malformed response pathをfail closeするようになり、release runtime assertionはfail-closed coverage tokenを検証し、Windows Flutter analyze/testは合格した。Windows installed-path proofはまだ存在しない。
  required_action: installed Windowsのbroker-down / crash / stale-session UI fail-closed testを実行する。
  blocks_release: yes

- item: Flutter broker authority surface active proof incomplete
  classification: release_blocker
  aggregate_of: windows_broker_installed_smoke
  reason: 製品main.dartはbroker IPCを使用し、broker由来のauthority status/projectionを描画する。これは独立したregistry blockerではない。v1.0がcommand dispatch scopeを明示的に変更するまでは、installed-path broker/runtime evidenceによって表す。
  required_action: v1.0 authority surfaceについてWindows installed-path broker evidenceを収集する。所有者がv1.0 scopeを変更しない限りcommand dispatchをsuspendedに保つ。
  blocks_release: yes

- item: Windows installed-path broker proof absent
  classification: release_blocker
  registry_id: windows_broker_installed_smoke
  reason: authenticated loopback IPCとdurable storeはlocal Rust integration testで証明されており、installed Windows app evidenceでは証明されていない。
  required_action: Windows installed-pathのbroker launch、authenticated IPC、restart persistence、およびcrash fail-closed evidence collectionを実行する。
  blocks_release: yes

- item: Audit anchor external tamper-evidence proof absent
  classification: release_blocker
  registry_id: audit_anchor_external_tamper_evidence_proof
  reason: broker-local HMAC anchorはlocal corruptionとpartial tamperを検出するが、key、anchor、audit logは現在同じproduct store trust region内にある。same-userまたはadministrator/rootによる書き換えへの耐性には、Windows ACL/DPAPI evidence、external anchoring、signed evidence、または明示的に受容したより狭いthreat modelが必要である。
  required_action: product release claimの前に、Windows installed-path key-protection modelまたはexternal-anchor modelを定義して検証する。
  blocks_release: yes
~~~
