# D4 Pocket — クイックスタート

> **現行採用（2026-11-24、v11）**: [要求仕様](docs/V11_2026-11-24/01_D4Pocket_REQUIREMENTS_v11_JA.jsonl) / [設計仕様](docs/V11_2026-11-24/02_D4Pocket_DESIGN_v11_JA.jsonl) / [実装開発工程](docs/V11_2026-11-24/03_D4Pocket_IMPLEMENTATION_PLAN_v11_JA.jsonl) / [品質工程](docs/V11_2026-11-24/04_D4Pocket_QUALITY_PLAN_v11_JA.jsonl) / [実行指示](docs/V11_2026-11-24/05_D4Pocket_IMPLEMENTATION_INSTRUCTIONS_v11_JA.jsonl)。実装と品質は独立した仕事。本人追加GOは実装/品質完了の条件ではない。未委任の署名・外部公開は別作用、製品使用時のNative Owner/認証Brokerの本人認可は保持。本書の以下の日付付き旧記述は当時の記録として保持し、現行正本の効力を上書きしない。


D4 Pocketは開発中の汎用AI Runtime／Agent操作基盤です。以下は**ソースコードから実装・契約を検証するための手順**であり、正式出荷製品のインストール案内ではありません。

- プロジェクト概要・技術的特徴：[README.md](README.md)
- 現行正本：[要求仕様](docs/V11_2026-11-24/01_D4Pocket_REQUIREMENTS_v11_JA.jsonl)・[設計仕様](docs/V11_2026-11-24/02_D4Pocket_DESIGN_v11_JA.jsonl)・[実装工程](docs/V11_2026-11-24/03_D4Pocket_IMPLEMENTATION_PLAN_v11_JA.jsonl)・[品質工程](docs/V11_2026-11-24/04_D4Pocket_QUALITY_PLAN_v11_JA.jsonl)・[実行指示](docs/V11_2026-11-24/05_D4Pocket_IMPLEMENTATION_INSTRUCTIONS_v11_JA.jsonl)・[採用記録](docs/V11_2026-11-24/採用記録.md)
- 継続証拠：[v8施工記録](docs/QE_2026-11-16/進捗.md)・[v5 N/W受入台帳](docs/QE_2026-11-14/進捗.md)（旧対象・時点の履歴と未閉包を保持。v11採用だけで再評価しない）
- rev5の製品受入れ履歴：[docs/REV5_PRODUCT_PROGRESS.md](docs/REV5_PRODUCT_PROGRESS.md)
- 安全・権限・Git運用：[AGENTS.md](AGENTS.md)
- 出荷判断：[RELEASE_CHECKLIST.md](RELEASE_CHECKLIST.md)

## 1. 作業対象に応じた契約確認

先に現行の実装指示と対象Source／Consumerを確認し、変更に関係する検査だけを選びます。次は該当する変更がある場合に実行します。

```bash
python -X utf8 tooling/current_governance_check.py
python tooling/schema_check/check_schemas.py
python tooling/conformance_tests/run_conformance_skeleton.py
```

`current_governance_check.py`はAGENTS／正本索引／現行文書を変更した場合、Schema検査はSchemaを変更した場合、Conformance検査はその契約へ影響する変更を行った場合に使います。`python`が存在しない環境では`python3`に置き換えてください。検査結果はその対象と範囲に限り、製品Releaseの成立証拠ではありません。

**参考実績：** 2026年10月11日のB4-L2において、Schema 167件、正常例163件、否定fixture215件、Conformance 245 checksのPASSを記録しています。固定の期待出力値ではなく、検査対象commitと実行環境の実測結果を記録してください。

## 2. Rust / Flutterの構築確認

実装作業ではSourceの構文・型・依存・リンク・Build成立を確認します。次はRustのcompile確認例です。

```bash
python -X utf8 tooling/run_cargo_isolated.py check --locked --offline --manifest-path native/rust_helper/Cargo.toml
python -X utf8 tooling/validate_all.py --flutter-step desktop_flutter_analyze --desktop-platform windows
```

Cargoの既定`target`はsourceの隣へ生成されます。OneDrive等の同期対象sourceでは生成先を安全な一時領域へ分離するため、この入口を使用してください。Cargoの終了値をそのまま返し、一時targetの回収に失敗した場合も成功扱いしません。隔離対象はCargo `target`であり、任意のbuild scriptや共有package cacheの書込みをsandboxするものではありません。

Flutter構築確認は、OneDrive上のsource treeへ`.dart_tool`や`build`を生成しないよう、`validate_all.py`の隔離snapshot経路を使います。上の例はDesktop analyzeを対象限定で実行します。Unit／E2E suiteやOS別製品QAは独立した品質作業として現行品質工程に従い、通常の実装完了条件へ混ぜません。

対象限定実行はFlutter stepの結果を必須扱いし、未実行・失敗を成功へ変換しません。`validate_all.py`が定義するanalyze／build stepは隔離snapshot上で実行します。Owner用launcherも隔離起動scriptを使いますが、任意に直接実行する`flutter run`は保護経路を通りません。Broker永続storeは既存開発データを維持するため従来locationに残り、短期session fileとbuild生成物はsnapshot内で回収します。

2026年10月11日のB4-L2ではRust直列test 569 passed / 0 failed / 13 ignoredを記録。一方、Flutter analyzerは既存deprecated info 5件、`cargo fmt --check`は既存の形式差分により終了値1を記録しています。これらの履歴を無言でPASSへ変換しないでください。当時の個別事実は[rev5受入れ履歴](docs/REV5_PRODUCT_PROGRESS.md)、現在の施工対象は冒頭のv11実装工程・実行指示を確認してください。

MobileのSource・Native・Builderはv11実装工程の対象として構築します。Android Emulator／iOS Simulatorへのinstalled製品QAやiOS Device配布物の証拠は独立した品質工程で扱います。過去のP13受入れは当時の範囲に属し、現候補のQA・正式配布を成立させません。

## 3. 開発時のOwner操作

既存の局所Owner用ランチャーを使用する場合だけ、対応する環境で次を実行します。

```bash
bash scripts/launch_owner_desktop.sh
```

```powershell
powershell -NoProfile -File scripts\launch_owner_desktop.ps1
```

これらは開発・診断用の起動経路であり、Installed-path最終証拠、署名済み配布物、Release readinessを生成しません。実行ポリシーが許可する通常のPowerShell経路で起動してください。ポリシーに拒否された場合は迂回せず、実行環境の制約として扱います。実行前に[AGENTS.md](AGENTS.md)、[CONFIG.md](CONFIG.md)、[セキュリティ方針](SECURITY.md)を確認してください。

## 4. 判断を混同しない

以下のB4・W01等は旧版・記録時点に結合した限定証拠です。「次工程」「他Runtime／OS」「未成立」も当時の対象・時点に限り、現在の状態や工程へ転記しません。現在の施工順・実装退出は冒頭のv11実装工程・実行指示に従います。

- **Schema適合**と**実Runtime作用**と**製品の安全な使用**は別の成立条件です。
- Windowsの<code>FEATURE COMPLETE</code>は正式Releaseではありません。
- P13 Mobile／Non-Windowsの有限Product Build受入は、実端末・署名・配布保証ではありません。
- 2026-10-17時点のB4有限製品経路受入はCLOSEDと記録しました。対象は`r2-e2e` staged buildとlocalhost合成APIによる一経路であり、通常Release・installed provenance・緩衝作用の独立照合を意味しません。当時の次工程はB5自己適用QCでした。
- v9 W01/G1の記録ではWindows normal Release／Codex CLIに限り<code>task_execution=supported</code>でした。他Runtime／OSの能力とW02以降は当時未成立でした。<code>release_ready=false</code>、16件のRelease Blocker、Ownerの独立GO未成立も当時の状態です。

製品主張の射程は[CLAIM.md](CLAIM.md)、現行実装工程は[v11工程表](docs/V11_2026-11-24/03_D4Pocket_IMPLEMENTATION_PLAN_v11_JA.jsonl)、品質作業は[v11品質工程](docs/V11_2026-11-24/04_D4Pocket_QUALITY_PLAN_v11_JA.jsonl)、適用範囲は[v11採用記録](docs/V11_2026-11-24/採用記録.md)で確認します。継続証拠は[v8施工記録](docs/QE_2026-11-16/進捗.md)・[v5 N/W受入台帳](docs/QE_2026-11-14/進捗.md)、v4/v3の採用時状態は[旧v4進捗](docs/QE_2026-11-13/進捗.md)・[旧v3進捗](docs/QE_2026-11-09/進捗.md)、rev5の受入れ履歴は[REV5_PRODUCT_PROGRESS.md](docs/REV5_PRODUCT_PROGRESS.md)、歴史的な試験結果は[VALIDATION.txt](VALIDATION.txt)で保持します。
