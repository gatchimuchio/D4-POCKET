# blue_tanuki_adapter 参照境界

BLUE-TANUKI 用の参照 adapter。

規則:

- BLUE-TANUKI は段階0の参照 runtime 契約対象として固定する。
- Adapter は GUI Shell 契約を BLUE-TANUKI runtime endpoint へ変換する。
- BLUE-TANUKI 固有の挙動を Shell Core へ漏らしてはならない。
- Adapter metadata は非信頼であり、権限を昇格させてはならない。
