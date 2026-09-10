# rev2 実装進捗と証拠境界

## 統治変更（2026-09-10）

対象はローカル品質判定と手動補助 Actions の分離。製品 runtime の権限・実行経路は変更しない。自動 CI は禁止を維持し、手動起動条件を構造として検査する。証拠分類は CONFIG / FIXTURE であり、外部実行や branch protection の保証ではない。

`python tooling/schema_check/check_schemas.py`、`python tooling/conformance_tests/run_conformance_skeleton.py`、`python tooling/日本語基底監査.py --strict` は PASS。Conformance は142項目。手動起動の文字列・列挙・対応形式、不在、実ファイル読取りを確認し、自動起動の混在、重複鍵、不正 YAML、独自タグを拒否した。

`PYTHONUTF8=1` を設定した Windows の `python tooling/validate_all.py --desktop-platform=windows` は FAIL。既存の `packaging_portability_check` だけが失敗し、他の13検査は PASS。Rust は42単体＋4統合、Flutter は31試験、desktop/mobile analyze と broker parity も PASS。日本語4ファイルの展開名不整合は変更前から存在し、独立した最小 ZIP でも再現した。UTF-8 locale 指定でも改善しない。統治変更に起因する失敗ではない。

WSL/Linux の `python3 tooling/validate_all.py --python-only --desktop-platform=linux` は9検査すべて PASS。これは Python 側の検証であり、reporter が併記する過去の Linux build / launch 記録を今回の実行証拠とは扱わない。Windows の環境依存の失敗は以下へ分離して保持する。

- item: Windows の ZIP 展開名不整合
  classification: release_blocker
  reason: Git for Windows 同梱 unzip で日本語名が変化する環境依存の失敗。統治変更で検査を除外しない。
  required_action: 次の Baseline 単位で標準展開機構を確認し、同一の manifest / conformance / release gate 検査を維持して修正する。
  blocks_release: yes

- item: installed-path 証拠・実機検証・owner GO
  classification: release_blocker
  reason: 統治単位は製品の完成証拠を作成しない。strict release は未実行である。
  required_action: 製品単位で正式な実機証拠と明示承認を揃える。
  blocks_release: yes

後続の実行系対話、MINIDORA Adapter、比較、Mobile、端末連携、各 platform の実装と検証は未完了。今回の統治単位の完成をそれらの完成へ昇格しない。これらの owner 指示内の未実装は rev2 完成に対する release_blocker として次単位以降で扱う。GitHub Actions は未使用。

## Baseline の Windows 配布検証修正

Git for Windows 同梱 unzip 6.00 は最小 ZIP の日本語名も文字化けさせた。`LC_ALL=C.UTF-8` と `-UU` でも再現した。一方、Windows 標準 tar.exe（bsdtar 3.8.8）は `LC_ALL=C` のまま同じ ZIP の名前を保持した。

検証専用経路を Windows は System32/tar.exe、POSIX は従来の unzip に分けた。独自 wrapper、代替の製品 runtime、検査除外は追加しない。展開後の manifest、conformance、release gate 検査は維持する。適用範囲は dev / release validation の source ZIP 展開のみである。新しい日本語名・空白を含むパスの実展開、内容hash一致、破損ZIPの拒否、展開器不在を試験する。これは当該 OS の外部展開器に対する EXTERNAL_EVIDENCE であり installed 製品証拠ではない。

本修正は Windows 標準機構の恒久的な選択である。将来対応OSの標準展開器が変わる場合は、同じ名前・内容・失敗の試験と実展開後検査を成立させて選択を見直す。

検証結果: Windows の `python tooling/validate_all.py --desktop-platform=windows` は14検査すべて PASS（終了値0）。WSL/Linux の `python3 tooling/validate_all.py --python-only --desktop-platform=linux` は9検査すべて PASS。Conformance は143項目。上記の Windows ZIP 展開名不整合はこの修正で解消した。installed-path 証拠と owner GO は未解消の release_blocker のままであり、開発用集約結果の pass を製品 release の許可にしない。

## 日本語意味正本・契約・Conformance

`docs/specs/runtime-dialogue.md` で要求・応答・セッション・比較、取消の限界、表示境界、権限と監査の前提を定義した。MINIDORA API の実コードは commit `3400a3bb68b37efa1dc14ee8aaa28fda779bf1f8` へ固定した。参照系のコードは変更しない。

四つの Schema、正常・権限混入の否定fixture、開発専用の関係検査を接続した。要求上限、本文上限、参照数上限、空白入力、識別子末尾の改行、未知field、閉じたセッション、応答対応、非全文表示への漏洩、左右成功・片側失敗・両失敗・応答入替え・セッション共有・表示許可転用を検査する。これらは CONFIG / FIXTURE 証拠であり、MINIDORA の live 動作、実行系停止、権限発行の証拠ではない。

- item: 製品対話の実装接続
  classification: release_blocker
  reason: 現単位の消費経路は Schema catalog と開発用 conformance。Rust 外部送信、UI、端末連携は未接続。
  required_action: 次単位で統治済み Rust 経路と Adapter、UI を実装し、実物通信と失敗・権限否定・取消を検証する。
  blocks_release: yes

契約単位の検証: Schema 30件、正常例30件、否定例32件、Conformance 145項目は PASS。Windows 開発用一括検証14検査と WSL/Linux の Python 側9検査は PASS。比較単独での空白入力の見逃しを追加監査で検出し、応答・比較の関係検査も拒否するよう補強した。権限判定や許可発行をこの開発検証へ持ち込まない。

製品接続の前提確認: 現行 `native/rust_helper/src/broker/authority.rs` の production_default は decision=deny、approvals=[] であり、owner 承認の登録経路がない。継続指示に基づき専用の Rust ローカルowner制御操作を採用した。以下の単位で通常UI資格と分離する。既存汎用commandのdeny/suspendを解除する変更ではない。

## Rust対話Core・MINIDORA Adapter・owner制御

専用の対話要求を通常IPCで保留し、別資格のowner CLIによる要求hash一致・期限内・一回限りの承認後だけ固定loopback APIを呼ぶ。Shell Coreは汎用traitを使い、MINIDORA固有のHTTPと応答射影はAdapterへ分離した。Flutter、Python、metadataから権限を発生させない。既存汎用command dispatchはsuspendedを保持する。

通信期限、要求・受信上限、worker数上限、セッション隔離、取消後の採用防止、raw受信と表示射影の分離を実装した。監査失敗時は送信または結果公開を拒否する。成功Adapter応答もCoreでセッションと構造を再検査する。owner資格は同一OS利用者に対する強い隔離や人間本人性の証明ではない。Windows installed-path保護の未検証を隠さない。

検証: `cargo test --manifest-path native/rust_helper/Cargo.toml` は53単体・5統合がPASS。`python tooling/minidora_live_check.py --reference C:/Users/mzcum/codex-work/MINIDORA-reference` は固定commitの実MINIDORA二processを用いPASS。通常資格拒否、owner CLI承認、基本会話、trace整合、表示分離、保留、片側停止・両側停止、監査chainの再起動読取を実行した。実API証拠はLIVE_RUNTIMEだが、基礎Core・外部検索能力の保証へ拡張しない。Schema31件・正常例31件・否定例33件、Conformance146項目、日本語厳格監査もPASS。

HTTP固定ヘッダーとCRLFだけを `JBE-007` の局所固定表記へ登録した。監査器の検出は弱めず、説明と診断は日本語のまま残す。Brokerは非同期状態を所有するためClone/Eqの導出を廃止した。生存workerや承認待ち状態を複製する内部APIは提供しない。

- item: Desktop比較UI・Mobile・端末連携・platform別実機証拠
  classification: release_blocker
  reason: この単位で接続したのはRust対話経路と実API検証。UI実装、secure storage、端末資格、Android/iOSの実動作は後続単位に残る。
  required_action: Flutter共有化と対話表示、二実行系比較、端末連携、platform別buildと実機検証を順次実装する。
  blocks_release: yes

- item: owner資格のOS保護とinstalled-path検証
  classification: release_blocker
  reason: ローカル開発資格の分離は同一利用者の任意processや管理者への耐性を証明しない。
  required_action: 既存の資格保護・監査anchorのrelease gateに従いWindows installed-path証拠を収集する。
  blocks_release: yes

集約検証: Windowsの `python tooling/validate_all.py --desktop-platform=windows` は14検査すべてPASS（終了値0）。WSL/Linuxでも `cargo test --manifest-path native/rust_helper/Cargo.toml` の53単体・5統合がPASSし、Linux binaryとLinux側の同一commit参照cloneによる `tooling/minidora_live_check.py` がPASSした。Windows cloneをWSL Gitで確認した際の改行差分は参照コードを書き換えず別cloneで分離した。GitHub Actionsは未使用。これらは開発環境の検証であり、installed-pathやMobile実機、owner GOのrelease_blockerを解除しない。
