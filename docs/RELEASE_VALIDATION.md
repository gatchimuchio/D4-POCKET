# リリース検証

このリポジトリで「リリース」とは、完成製品のリリースを意味する。

~~~yaml
- item: skeleton、preview、alpha、beta、およびscaffoldの各状態
  classification: release_blocker
  example: true
  reason: これらの状態は完成製品のリリース状態ではない。
  blocks_release: yes
~~~

## 開発検証

開発検証では次を満たす。

- スキーマ検査が合格しなければならない。
- 適合性検査が合格しなければならない。
- 利用できないツールチェーンは`not_run`として記録してよい。

~~~yaml
  classification: release_blocker
  blocks_release: yes
~~~

- 開発検証は完成製品のリリース判定には適さない。

~~~yaml
  classification: release_blocker
  blocks_release: yes
~~~

`not_run`を許容するのは開発検証だけである。

## リリース検証

リリース検証では次を満たす。

- 範囲内の構成要素に`not_run`を認めない。
- 未分類の残存リスクを認めない。
- いずれかの`release_blocker`が存在すれば検証は失敗する。
- `validate_all.py --strict-release`が合格しなければならない。
- `python3 tooling/release_smoke.py`が合格しなければならない。
- `python3 tooling/evidence_bundle.py --check`が合格しなければならない。
- `python3 tooling/release_runtime_assertions.py --check`が合格しなければならない。
- Windows完成製品のリリースでは`python3 tooling/windows_release_evidence.py`が合格しなければならない。
- Windowsのインストール済みスモーク証拠には`installer\windows\collect_broker_smoke.ps1`の証拠を含めなければならない。
- インストール済みアプリが生成したSetup Doctor製品エクスポート証拠を含めなければならない。`installer\windows\collect_setup_doctor.ps1`は外部プローブ証拠に限る。
- Windowsのインストール済み初回実行証拠は、インストール済みRustブローカーを介し、`-NoPythonRuntime`起動証拠を伴って、インストール済みFlutterの`.exe`を起動しなければならない。
- Rustヘルパーがリリース範囲内なら`cargo test`が合格しなければならない。
- デスクトップアプリがリリース範囲内ならデスクトップの`flutter analyze`が合格しなければならない。
- モバイルの`flutter analyze`が必要なのは、モバイルがリリース範囲内の場合だけである。
- インストーラーがリリース範囲内なら、機械可読なインストール先証拠を伴うインストーラー・スモークが合格しなければならない。

範囲内の構成要素について、リリース検証での`not_run`は`release_blocker`である。

## 分類規則

~~~yaml
- classification: `release_blocker`
  reason: v1.0完成製品のリリースに必要であり、未完了、未検証、失敗、または未実行の状態である。
  blocks_release: yes

- classification: `post_v1_scope`
  reason: v1.0完成デスクトップ製品の範囲外であることを明示している。
  blocks_release: no

- classification: `known_limitation`
  reason: リリース向け主張に記載した、受容済みのv1.0制限である。
  blocks_release: no
~~~
