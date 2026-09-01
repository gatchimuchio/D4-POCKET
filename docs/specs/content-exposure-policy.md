# 内容露出方針（Content Exposure Policy）

## 表示範囲の level

```text
none
hash_only
summary
redacted
full
```

## 規則

- `none`: 未加工の content を表示してはならない。
- `hash_only`: payload hash だけを表示できる。
- `summary`: Runtime / Adapter が承認した summary だけを表示できる。
- `redacted`: redacted diff / content だけを表示できる。
- `full`: content の全文を表示できる。

## 既定値

default は `none` である。

Policy はより強い visibility value を許可できるが、安全側の default は `none` のまま維持しなければならない。

有効な Approval Contract または Content Exposure Contract が `content_visibility=full` と定める場合に限り、full payload の表示を許可する。

`full_payload` は Approval の保存領域に存在し得る。保存されていること自体は表示権限ではない。有効な visibility が `full` でない限り、UI projection はこれを抑止しなければならない。
