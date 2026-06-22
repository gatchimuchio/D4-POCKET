# GUI-Shell 言語方針 監査レポート v0.1

Status: 暫定採用方針  
Scope: GUI-Shell language, runtime, safety-boundary, and validation responsibility policy

## 1. 結論

GUI-Shell の言語方針は、以下を暫定決定とする。

```text
UI層: Flutter / Dart
安全境界層: Rust
除外: TypeScript / Node は本体から外す
除外: Python は runtime から外す
許容: Python は dev-only tooling、TypeScript は外部SDK・サンプル用途に限定
```

最終構成は、Flutter/Dart UI + Rust safety boundary + 独立プロセス IPC を基本形とする。

GUI-Shell は BLUE-TANUKI の内包物ではなく、汎用の操作・承認・監査・復旧 Shell として独立製品の位置づけを維持する。したがって、判断軸は「作業者が保守しやすいか」ではなく、ユーザーから見て安全・堅牢・安定であるかに置く。

## 2. 判断軸

今回の判断軸は以下の3点に固定する。

1. 安全性

   - 承認・署名・権限・監査境界が破れにくいこと。
   - UI層が侵害されても、実行権限・署名鍵・監査証跡が直接破壊されないこと。

2. 堅牢性

   - 異常終了・IPC失敗・外部runtime不調・依存破損時に、復旧・停止・隔離ができること。
   - 破損時に安全側へ倒れること。

3. 安定性

   - インストール、起動、常駐、更新、通知、OS別動作、GUI表示が安定すること。
   - local validation・release gate・開発環境の複雑性が、ユーザー向け品質保証を壊さないこと。

補助評価軸として保守性・開発速度は考慮するが、最上位ではない。

## 3. ケビンレビューの整理

ケビンの主張の核は以下である。

- 現状の GUI-Shell / BLUE-TANUKI には、Flutter/Dart、TypeScript/Node、Rust、Python が混在しており、toolchain 負荷が重い。
- ローカル環境・WSLg・Flutter SDK PATH など、環境依存の問題が release gate に漏れている。
- 単一リポ・単一スタック・restricted IPC によって、同等の安全境界を維持しつつ運用負荷を下げられる可能性がある。
- static schema 検証へ寄りすぎると、非構造化リアルタイム信号や運用異常に対して brittle になるリスクがある。

このうち、採用すべき指摘は以下である。

```text
採用:
- validation 基準面から WSLg 前提を除去する
- Flutter SDK不在で全体release gateが死ぬ構造を修正する
- ローカルWindows/WSL前提を品質判定基準面へ漏らさない
- schema検証とruntime検証を分ける
- 不要なtoolchainを削る
- release責任境界を明確化する
```

採用しない指摘は以下である。

```text
不採用:
- GUI-ShellをBLUE-TANUKIへ内包する
- GUI-Shell本体をTypeScript/Node中心へ寄せる
- 単一スタック化それ自体を安全性の根拠にする
- 実装効率を安全・堅牢・安定より上位に置く
```

## 4. Claude案の整理

Claudeの一度目の案は、Rust中心収束を提案した。

```text
提案:
- コア・暗号・プロセス検証: Rust
- UI層: Tauri / egui / iced
- スキーマ生成: Rust macro または Python dev-only
- 排除: TypeScript/Node、Flutter/Dart
```

この案の正しい点は以下である。

- Rustは安全境界、署名検証、IPC、プロセス制御に強い。
- TypeScript/Node統一は、GUI-Shellの安全・堅牢・安定という基準には弱い。
- Python runtime配置は避けるべき。
- Flutter/Dart単独で安全境界まで背負わせるのは弱い。

一方で、不足していた点は以下である。

- Rustは safe Rust の範囲では強いが、unsafe、依存crate、FFI、OS連携を含めると、言語選定だけでは安全は成立しない。
- Tauriは Rust全面ではなく Rust + WebView であり、HTML/CSS/JS frontend領域が戻ってくる。
- egui / iced など純Rust UIは、現時点ではGUI-Shellのような基盤製品に求められるUI成熟度・デザイナビリティ・クロスOS UXで不利。
- 安全・堅牢・安定のうち、GUI製品としての安定性を過小評価していた。

そのため、Rust全面化は不採用とする。

## 5. 言語別評価

### 5.1 Flutter / Dart

採用する。GUI-Shell本体UIの主軸とする。

用途:

```text
- UI
- 画面遷移
- 承認フロー
- 監査ビュー
- 設定
- onboarding
- 通知
- desktop / mobile 展開
- 状態表示
```

理由:

- GUI基盤アプリとしての開発効率が高い。
- desktop / mobile 展開を同一思想で扱いやすい。
- Electronよりネイティブ寄りのアプリ感を作りやすい。
- Dart の null safety により、UI層の事故を一定程度抑制できる。
- GUI-Shellの価値である、ユーザー体験・初期導入・状態可視化・操作導線の品質を作りやすい。

懸念:

- Googleプロダクト寿命リスク。
- Flutter SDK 依存が local validation / release gate に漏れると重くなる。
- native security boundary までDartに背負わせると弱い。

対応:

- FlutterはUI層に限定する。
- 安全境界はRustへ分離する。
- local validation では UI build / GUI smoke / security boundary test を分離する。

### 5.2 Rust

採用する。ただし、Rust全面化ではなく安全境界層に限定する。

用途:

```text
- secure IPC broker
- process supervisor
- signature verification
- hash-chain / audit verification
- keychain / credential access
- installer / updater補助
- tamper detection
- authority boundary
- external runtime 接続時のcommand envelope検証
```

理由:

- メモリ安全性、型安全性、低レイヤー制御、OS連携に強い。
- 安全境界・監査境界・署名検証など、破られてはいけない層に適している。
- GUI層から権限・署名鍵・実行制御を隔離するための中核になる。

制約:

- unsafe使用は原則禁止、例外は明示レビュー。
- 依存crateのunsafe・脆弱性・ライセンスを監査する。
- Rustだから安全、とは扱わない。
- Rust UI全面化は現時点では採用しない。

### 5.3 TypeScript / Node

GUI-Shell本体には採用しない。

許容用途:

```text
- external SDK
- adapter sample
- protocol client sample
- BLUE-TANUKI側の接続例
- web系runtimeとのブリッジ例
```

不採用用途:

```text
- GUI-Shell本体runtime
- 承認境界
- audit境界
- authority境界
- 常駐中核プロセス
```

理由:

- npm依存爆発とサプライチェーン面の負担が大きい。
- GUI-ShellがWeb管理画面方向に寄りやすい。
- Node runtime を中核に置くと、権限境界が曖昧化しやすい。
- 実装効率は高いが、安全・堅牢・安定を最上位とするGUI-Shell本体には合わない。

### 5.4 Python

runtimeには採用しない。

許容用途:

```text
- dev-only tooling
- schema generation
- migration helper
- CI補助
- 一時的な検証スクリプト
```

不採用用途:

```text
- GUI-Shell起動必須処理
- release runtime依存
- 承認境界
- audit境界
- 常駐プロセス
```

理由:

- 動的型であり、runtime契約破壊が実行時まで顕在化しにくい。
- 配布アプリの安定性に不要な依存を増やす。
- 依存管理や環境差分がCI・配布で問題化しやすい。

## 6. Flutter-Rust接続方針

Flutter と Rust の接続は、原則として独立プロセス + IPC を採用する。

```text
Flutter GUI process
  ↓ restricted IPC
Rust security broker process
  ↓ restricted IPC / signed envelope
external runtime / BLUE-TANUKI / other agents
```

### 6.1 flutter_rust_bridge / FFI 直結について

FFI直結は便利だが、信頼境界用途では弱い。

理由:

- 同一プロセス内結合になりやすい。
- UI層が侵害された場合、Rust層を同時に巻き込みやすい。
- GUI-Shellの目的である「操作面と安全境界の分離」が弱くなる。

そのため、FFIは以下に限定する。

```text
許容:
- 非権限処理
- UI補助の高速化
- ローカル計算
- セキュリティ境界外の処理

不許可:
- 署名鍵アクセス
- authority decision
- approval token handling
- external runtime command dispatch
- audit finalization
```

## 7. Local Validation / Release Gate 方針

GitHub Actions / CI workflow は GUI-Shell の品質判定経路ではない。`.github/workflows` 配下に YAML workflow を置かず、品質判定は owner / Codex が明示実行する local validation、smoke、release verification、Windows 実機 evidence を正とする。

ケビン指摘を踏まえ、local validation / release gate は以下の責任分離を行う。

```text
1. Contract validation
   - JSON Schema / protocol / fixtures
   - OS非依存
   - 軽量・高速

2. Rust Security Boundary validation
   - cargo test
   - cargo audit
   - cargo deny
   - unsafe policy check
   - IPC protocol tests

3. Flutter UI validation
   - dart analyze
   - flutter test
   - platform build smoke
   - GUI smoke はOS別に分離

4. Integration validation
   - Flutter process ↔ Rust broker IPC
   - signed envelope roundtrip
   - authority rejection tests
   - audit replay tests

5. OS-specific release evidence
   - Windows / macOS / Linux に分離
   - WSLg前提を品質判定基準面へ漏らさない
   - Flutter SDK不在で全体release gateが死なないよう分離
```

重要なのは、全 validation を一枚岩にしないこと。Flutter SDKやGUI smokeの失敗が、contract/security validation の結果を無効化しない構成にする。

## 8. 最終採用方針

GUI-Shellの言語方針は以下で固定する。

```text
Adopt:
  - Flutter / Dart for UI product layer
  - Rust for native security boundary
  - JSON Schema / protocol files for contracts

Limit:
  - Python to dev-only tooling
  - TypeScript to external SDK / samples only

Reject:
  - TypeScript/Node as core runtime
  - Python as runtime dependency
  - Rust-only GUI for V1
  - Flutter-only security boundary
  - FFI direct bridge for authority boundary
```

## 9. 監査結論

ケビンの「多言語が重い」という指摘は、validation・検証・環境漏出の改善材料として採用する。
Claudeの「Rust中心」は、安全境界層に限定して採用する。
チャッピー案の「Flutter/Dart UI + Rust安全境界」は、GUI-Shellの安全・堅牢・安定という判断軸に最も整合する。

したがって、GUI-Shellは以下の構成で進める。

```text
GUI-Shell = Flutter/Dart UI + Rust security broker + restricted IPC + protocol contracts
```

この構成は、作業者都合の単一化ではなく、ユーザーから見た安全・堅牢・安定を最大化するための二層構成である。
