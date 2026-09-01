# Cross-platform framework 評価

| 評価軸 | Flutter + Rust helper 案 | Compose MP + Rust helper 案 | Tauri + TypeScript + Rust 案 |
|---|---|---|---|
| 安全性 | 権限を UI 外に保てば良好 | 権限を UI 外に保てば良好 | 良好だが bridge audit の負担が大きい |
| 頑強性 | 高い | 中〜高 | desktop で高い |
| 安定性 | 中 | 中〜高 | 中 |
| PC/mobile 等価性 | 高い | 中〜高 | desktop 偏重 |
| 製品成熟度 | 高い | 中 | desktop で高い |
| 運営主体の risk | Google 依存 | JetBrains/Kotlin 依存 | WebView/Rust/TS 依存 |
| 現行の役割 | 第一候補 | 監視対象 | desktop 偏重時の fallback |
