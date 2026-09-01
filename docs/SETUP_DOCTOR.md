# 環境診断

環境診断は、読取り専用の環境・契約状態表層である。

各 check は次を報告する。

- `check_id`
- `status`
- `message`
- `recovery_instruction`
- `grants_authority=false`

欠落した依存関係は操作者から見えなければならない。開発者向けに CLI fallback を許可するが、通常利用者の経路は desktop 環境診断画面とする。
