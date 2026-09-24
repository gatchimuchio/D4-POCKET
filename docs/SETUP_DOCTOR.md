# 環境診断

環境診断は、読取り専用の環境・契約状態表層である。

各 check は次を報告する。

- `check_id`
- `status`
- `message`
- `recovery_instruction`
- `grants_authority=false`

欠落した依存関係は操作者から見えなければならない。開発者向けに CLI fallback を許可するが、通常利用者の経路は desktop 環境診断画面とする。

製品版の画面は、現在の製品snapshotが観測した実行系状態、診断結果、監査鎖状態、復旧案内を示す。Flutter／Dart、Python、Rust、Gitなどの開発toolchainを通常利用者向けの製品依存として表示してはならない。開発toolchainの検査は開発者向け検証文書・CLIに分離する。ローカルsnapshot/config/auditの生pathも通常の診断要約へ表示せず、製品状態や権限の根拠として扱わない。
