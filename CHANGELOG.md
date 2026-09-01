# 変更履歴

## 未 release

- GUI Shell の境界を保ちつつ、最上位 repository 文書の形式を BLUE-TANUKI 参照 style と整合させた。
- 段階0/段階1の運用向けに、主張、設定、audit、security、troubleshooting、quickstart 文書を追加した。
- README に要約、固定表層、明示的な境界、architecture、検証、最上位参照を追加した。
- `AGENTS.md` を BLUE-TANUKI style の優先順位、main直接 backup flow、検証 gate、GUI Shell 用報告形式に合わせて再構成した。
- repository flow、2世代 backup model、release 主張規則を記録する `docs/OPERATING_MODEL.md` を追加した。
- 段階0から release 強化までの実行順序を示す `ROADMAP.md` を追加した。
- Authority Strip Conformance 文書を追加した。
- 権限除去、metadata 昇格、GUI 生成権限文脈、非権限 state、content exposure、保護された approval field、approval 編集の再 hash/再検証、機密 action の audit/recovery 対応について、失敗事例を覆う conformance check を強化した。
- 有効な contract 例と無効 fixture 拒否により schema 検証を強化した。
- Tauri fallback 調査 note を追加した。
- MIT license を追加した。
- 曖昧な BLUE-TANUKI 固定表現を、段階0の参照 runtime contract 対象という表現へ置き換えた。
- adapter 権限昇格、安全でない update policy、不正な approval hash、content exposure の既定 full 表示に対する negative contract fixture を追加した。
- `examples/contracts/*.valid.json` から検査を駆動する conformance coverage を追加した。
- `full_payload` は approval storage に存在できるが、`content_visibility=full` でない限り UI 射影で公開してはならないと文書化した。
- invalid contract fixture を全 schema へ拡張し、fixture coverage を schema/conformance 検証の一部にした。
- framework 非依存 contract 読込み、runtime registry、adapter loader、permission ledger、approval queue、audit store、recovery catalog、update policy store、content exposure 射影、機密 action routing を備えた段階3 Shell Core 骨格を開始した。
- Shell Core が adapter metadata permission を無視し、memory/cache/previous-state 権限を拒否し、必須対応を通じて機密 action を route し、full visibility まで full payload を隠し、Flutter/BLUE-TANUKI 内部 import を避けることを証明する conformance check を追加した。

## 0.1.0-段階0

- 汎用 GUI Shell repository 骨格を初期化した。
- 段階0標準を追加した。
- framework risk register 文書を追加した。
- JSON Schema contract を追加した。
- conformance test 骨格を追加した。
- Flutter desktop/mobile 予約境界を追加した。
- Rust helper 予約境界を追加した。
- BLUE-TANUKI adapter 予約境界を追加した。
