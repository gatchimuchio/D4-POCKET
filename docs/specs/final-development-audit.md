# C32 最終開発監査

## 目的

C32は、総合機能拡張rev1のC0〜C31について、各作業単位の意味正本、Contract、Code、Production path、Test、Negative、Recovery、Audit、UI、Evidenceの対応漏れを確認する開発監査である。これは機能実装そのもの、Windows installed productの実証、正式release、owner GOを代替しない。

対応表の機械可読正本は`docs/specs/final-development-audit.json`である。工程ごとの残存事項は、成功主張へ昇格させず、`release_blocker`、`post_v1_scope`、`known_limitation`のいずれかへ分類する。

## 監査経路

```text
final-development-audit.json
  → tooling/final_development_audit.py
  → C0〜C31の必須項目・参照path・工程番号を検査
  → 開発監査結果をJSONで出力
```

実行:

```bash
python tooling/final_development_audit.py
```

監査器は、必要項目の空欄、重複工程、受領記録にない工程、参照path欠落、進捗表との番号不一致を失敗にする。C32〜C34は監査表の未監査工程として明示し、C0〜C31の表が成立しても`release_ready=false`を維持する。監査結果へ資格値、対話本文、raw stdout/stderr、外部secretを保存しない。

## 証拠境界

- `LIVE_RUNTIME`: 実Broker IPCまたは開発用localhost経路で観測した範囲だけを示す。
- `INTERNAL_STATE`: Broker内部状態、永続状態、監査chain、bounded projectionの範囲だけを示す。
- `CONFIG`: 文書・Schema・manifest・設定の整合だけを示す。
- `FIXTURE`: 意図的に作った正常／負例／fake serverの範囲だけを示す。
- `EXTERNAL_EVIDENCE`: 外部hostや実機から取得し、source commitとartifactへ結合した範囲だけを示す。

C30のlocal matrix PASS、C28の短時間smoke、C29の8件PASS、Rust／Flutter試験PASSは、installed product、外部Runtime／Agent／MCP／A2A、実端末、8時間運用、正式署名、owner GOの証拠ではない。C28の8時間実測は途中失敗を保持し、PASSへ書き換えない。

## C32監査結果の意味

監査器の終了値0は「対応表の構造と参照が成立した」ことを意味する。全機能完成、release readiness、権限安全性のruntime保証ではない。終了値1は対応漏れまたは参照不整合であり、該当工程を未成立として修正する。

現行残存の主な`release_blocker`は、Windows installed-pathのprovenance／first-run／Setup Doctor／Broker／Audit anchor外部改変証拠、C28の8時間実測、外部Runtime／Agent、実端末、C33／C34、正式署名、owner GO、正式releaseである。
