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

参照pathはRepository内の相対参照に限る。`/`とbackslashの区切りを現在OSのpathへ正規化し、空欄・相対表記の`.`・親directory遡行・絶対path・drive付きpath・NUL文字を拒否する。symlink解決後の実体もRepository内へ限定する。参照の不正・解決不能は当該参照の監査エラーとして記録して後続を検査し、失敗JSONと終了値1へ接続する。実際に欠けた参照は引き続き不在エラーにする。

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

## 2026-10-22 D4-AUD-F01 是正記録

状態：`VERIFIED`（F01の有限是正を閉鎖）。段階：`QE_BUILD`。変更前のlocal／remote mainは`e9582717eb49fa715196ced024f4617e6d8fdde6`でclean。writer一覧では本task以外のactiveなD4 writerはなく、3つの未統合remote branchは保持した。変更前のremote backupは`refs/tags/codex/backup-main=e9582717…`、`refs/tags/codex/backup-main-prev=2cb330ed…`と異なる復旧点だった。現mainが既に保存済みのため、同一SHAの二世代を作る再回転は行っていない。G0は`CLEAR`。

原因は旧`_relative_path`がPOSIX区切りを常にbackslashへ変換していたこと。監査入力は変更せず、全298参照出現（重複を含む、実対象99）中、区切りを含む256出現がPOSIX上で偽の不在になった。32工程・必須10面・受領記録・未監査C32〜C34・既存負例・Release Gateを維持した。

| 対象環境 | 修正前 | 修正後 |
|---|---|---|
| Windows環境（11、build 26300）、Python 3.12.10 | 監査エラー0件、終了値0 | 監査エラー0件、終了値0、対象回帰試験6件PASS |
| Ubuntu環境（24.04.5）、WSL2 kernel 6.18.54.1、Python 3.12.3 | 偽不在256件、終了値1。追加した現行対応表の回帰試験もFAIL | 監査エラー0件、終了値0、対象回帰試験6件PASS |

両環境の回帰試験は、区切り両形式の実在参照、欠落、空欄、親directory遡行、POSIX／Windowsの絶対path・drive表記、root外／内部の実symlink、不正な参照を含む行、参照解決エラーを検査した。不正行でも失敗JSONと終了値1を返し、正常表でも`release_ready=false`／`release_blocker=true`を確認した。

実行command：

```text
python -B -X utf8 -m unittest tooling.conformance_tests.test_final_development_audit -v
python -B -X utf8 tooling/final_development_audit.py
wsl -d Ubuntu-24.04 --cd /mnt/c/Users/ohira/OneDrive/ドキュメント/ChatGPT/D4ポケット --exec python3 -B -m unittest tooling.conformance_tests.test_final_development_audit -v
wsl -d Ubuntu-24.04 --cd /mnt/c/Users/ohira/OneDrive/ドキュメント/ChatGPT/D4ポケット --exec python3 -B tooling/final_development_audit.py
```

検査したコードのSHA-256は監査器`5B46CF3E5BF550477F43944BE540DFCE5647E5112C0D9B00D6D668D94D8978DB`、回帰試験`61B5A74E0F6CAE0041A09117D9099A94756C57D27DFA005E324D2245333EE8D7`。証拠は実対応表の`CONFIG`と否定入力の`FIXTURE`であり、POSIX側は`LINUX_WSL_DEV`である。Linux native製品の受入れではない。Rust／Flutter試験はF01で変更がないため`NOT_RUN`。既存の製品`release_blocker`、B4 CLOSED、B5 OPEN、正式Q0〜Q7 `DEFERRED`を保持する。次は`D4-AUD-F03`の実績文書一致を扱う。
