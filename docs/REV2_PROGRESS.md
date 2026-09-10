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
