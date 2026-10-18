# D4 Pocket — クイックスタート

D4 Pocketは開発中の汎用AI Runtime／Agent操作基盤です。以下は**ソースコードから実装・契約を検証するための手順**であり、正式出荷製品のインストール案内ではありません。

- プロジェクト概要・技術的特徴：[README.md](README.md)
- 最新工程と実測：[docs/REV5_PRODUCT_PROGRESS.md](docs/REV5_PRODUCT_PROGRESS.md)
- 安全・権限・Git運用：[AGENTS.md](AGENTS.md)
- 出荷判断：[RELEASE_CHECKLIST.md](RELEASE_CHECKLIST.md)

## 1. 最初の確認 — Contract / Conformance

Python 3系を利用できる環境で、リポジトリのrootから次を実行します。

```bash
python tooling/schema_check/check_schemas.py
python tooling/conformance_tests/run_conformance_skeleton.py
```

`python`が存在しない環境では`python3`に置き換えてください。これらは契約・fixture・Conformance経路を検査するもので、FlutterやRustのインストール、実機起動、製品Releaseの成立証拠ではありません。

**参考実績：** 2026年10月11日のB4-L2において、Schema 167件、正常例163件、否定fixture215件、Conformance 245 checksのPASSを記録しています。固定の期待出力値ではなく、検査対象commitと実行環境の実測結果を記録してください。

## 2. Rust / Flutterを検査する

それぞれのToolchainを用意した環境で実行します。

```bash
cd native/rust_helper
cargo test
```

Flutter検査は、OneDrive上のsource treeへ`.dart_tool`や`build`を生成しないよう、`validate_all.py`の隔離snapshot経路を使います。例えばDesktop analyzeとtestはそれぞれ次のように対象を限定して実行できます。Desktop testには必要なRust helper buildも含まれます。

```bash
python -X utf8 tooling/validate_all.py --flutter-step desktop_flutter_analyze --desktop-platform windows
python -X utf8 tooling/validate_all.py --flutter-step desktop_flutter_test --desktop-platform windows
```

対象限定実行はFlutter stepの結果を必須扱いし、未実行・失敗を成功へ変換しません。隔離対象は`validate_all.py`が定義するanalyze／test／buildに限られ、`flutter run`などの直接実行はこの保護経路を通りません。

2026年10月11日のB4-L2ではRust直列test 569 passed / 0 failed / 13 ignoredを記録。一方、Flutter analyzerは既存deprecated info 5件、`cargo fmt --check`は既存の形式差分により終了値1を記録しています。これらの履歴を無言でPASSへ変換しないでください。現在の個別事実と必要な検証対象は[進捗](docs/REV5_PRODUCT_PROGRESS.md)を確認してください。

Mobileはrev5のP13でAndroid Emulator／iOS Simulator等の有限受入を実施していますが、物理端末での正式保証・配布を意味しません。

## 3. 開発時のOwner操作

既存の局所Owner用ランチャーを使用する場合だけ、対応する環境で次を実行します。

```bash
bash scripts/launch_owner_desktop.sh
```

```powershell
powershell -ExecutionPolicy Bypass -File scripts\launch_owner_desktop.ps1
```

これらは開発・診断用の起動経路であり、Installed-path最終証拠、署名済み配布物、Release readinessを生成しません。実行前に[AGENTS.md](AGENTS.md)、[CONFIG.md](CONFIG.md)、[セキュリティ方針](SECURITY.md)を確認してください。

## 4. 判断を混同しない

- **Schema適合**と**実Runtime作用**と**製品の安全な使用**は別の成立条件です。
- Windowsの<code>FEATURE COMPLETE</code>は正式Releaseではありません。
- P13 Mobile／Non-Windowsの有限Product Build受入は、実端末・署名・配布保証ではありません。
- B4の有限製品経路受入は2026-10-17にCLOSEDです。対象は`r2-e2e` staged buildとlocalhost合成APIによる一経路であり、通常Release・installed provenance・緩衝作用の独立照合を意味しません。次工程はB5自己適用QCです。
- 通常Releaseは<code>task_execution=unsupported</code>、<code>release_ready=false</code>。Human ownerの独立GOも未成立です。

製品主張の射程は[CLAIM.md](CLAIM.md)、作業の現行状態は[進捗](docs/REV5_PRODUCT_PROGRESS.md)、歴史的な試験結果は[VALIDATION.txt](VALIDATION.txt)へ分離して確認します。
