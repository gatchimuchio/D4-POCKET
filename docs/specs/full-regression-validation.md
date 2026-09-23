# C30 全数回帰検証

## 責任

C30は、既存機能と総合機能拡張rev1で追加した境界を、現在利用できるlocal validation経路へ対応付ける。対象はTrust、Authority、Permission、Approval、Audit、Recovery、Evidence、Runtime、Agent、Dialogue、Compare、Device Linkである。

`tooling/full_regression_validation.py`は、各対象を既存のRust Broker、Evidence assertion、localhost Runtime fixture、Agent interface probe、共有Flutter、Mobile Flutterの検証へ割り当てる。新しい権限、Approval、Runtime、Agent接続を生成しない。

## 証拠境界

- Rust全試験はBroker、Authority boundary、永続Audit、Recovery、負例を含むが、installed製品の実行証拠ではない。
- Evidence checkは設定・内部状態・fixtureの整合だけを確認し、外部監査anchorの継続性や署名を生成しない。
- Runtime／Dialogueは実Broker IPCとlocalhostのMINIDORA API fixtureを使う。外部Runtime、8時間運用、installed productを証明しない。
- Agentは実物CLIのversion/help interfaceだけを確認し、task実行、workspace書込、credential、比較、handoffを行わない。未導入Agentはunsupportedまたはunavailableとして扱う。
- Compareは共有Flutterの評価projection fixture、Device LinkはMobile client fixtureである。実Agent比較、実端末、TLS、native secure storageを証明しない。

検証器はraw stdout/stderrをreportへ保存しない。資格値、対話本文、秘密値を回帰証拠へ持ち込まない。

## 実行

```text
python tooling/full_regression_validation.py
```

全行が実行されて終了状態が`passed`のときだけ、C30のlocal regression pathが成立する。`failed`、`timeout`、`unavailable`は隠さず、該当範囲を未成立として残す。local pathのPASSは、C31の文書更新、C32の全機能監査、C33のWindows最大到達点、正式releaseの証拠を代替しない。
