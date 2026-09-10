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
