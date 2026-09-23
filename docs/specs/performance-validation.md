# C27 性能検証の責任正本

## 目的

C27は、総合機能拡張で追加されたboundedな表示・projection経路に、開発時点で明らかな劣化または停止がないかを測定する。これは性能SLA、実installed製品のrelease証拠、GPU frame保証を定義する文書ではない。

## 測定経路

```text
tooling/performance_validation.py
  → tooling.shell_snapshot.build_shell_snapshot()
  → apps/desktop_flutter/test/c27_performance_validation_test.dart
  → Flutterの表示projection・Broker応答clientのfixture
```

Flutter試験は、次を5秒の開発用watchdog内で測定する。

- 起動前のShellCore model初期化
- snapshot JSONの契約model読込
- Runtime／Audit／Historyの表示用metadata projection
- global searchのbounded indexと検索
- notification countの200件bounded response
- resource pollingのunknown保持と応答解析
- 4096行large diffの遅延描画

`GlobalSearchIndex`の上限、通知件数、資源履歴、差分行の上限は各production contractの上限をfixtureへ反映する。fixtureの成功は実Runtime、実通知source、実installed artifactの性能を証明しない。

## 証拠分類

- snapshot生成: `INTERNAL_STATE`
- Flutter表示投影の検証データ: `FIXTURE`
- 実installed製品の起動・画面応答・GPU frame: このC27では未測定

fixture応答または内部snapshotの測定結果を`LIVE_RUNTIME`や`EXTERNAL_EVIDENCE`へ昇格させない。測定不能な資源値は`unknown`のまま扱い、0へ変換しない。

## 未成立範囲

次は別のWindows実機・運用証拠が必要であり、C27のローカル測定だけでは成立しない。

- clean installed productのcold start／warm start
- Windows実画面でのUI freeze・frame time
- 実Runtime、実Agent、実MCP、実A2Aを含む負荷
- 8時間運用中のmemory／resource leak
