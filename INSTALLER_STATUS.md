# Installer 状態

現行段階: 段階Bの所有者利用は完了。Installer 作業はまだ有償/製品 QC として扱わず、完成製品 release には実測済み Windows インストール先証拠が必要である。

## 実装済み領域

- item: `installer/setup_doctor.py`
  classification: required_for_v1
  status: 構造化状態 check あり

- item: 依存関係の recovery 手順
  classification: required_for_v1
  status: あり

- item: `installer_grants_authority=false`
  classification: required_for_v1
  status: あり

- item: `installer_silently_approves_permissions=false`
  classification: required_for_v1
  status: あり

- item: installer 境界文書
  classification: required_for_v1
  status: あり

## リリース遮断要因

- item: Windows インストール先の初回起動証拠が欠落
  classification: release_blocker
  registry_id: windows_installer_first_run_smoke
  reason: Windows 優先の完成製品 release には、分離実行の由来、source commit、artifact hash、evidence bundle hash、UIAutomation 診断 tree を備えたインストール先の実測初回起動証拠が必要である。
  required_action: 可視表層診断 tree、broker 実測 field 由来、config path、audit directory 確認入力、installed manifest を伴う固有の staged 実行から、強化済み Windows installed smoke collector を実行する。
  blocks_release: yes

- item: Windows 環境診断のインストール先証拠が未収集
  classification: release_blocker
  registry_id: windows_setup_doctor_smoke
  reason: PowerShell 環境診断 collector は引き続き外部 installer/config/broker 確認証拠である。インストール済み Flutter app は機械可読な環境診断製品出力を生成できるが、native Windows 証拠はまだ収集・検証されていない。
  required_action: インストール済み app が環境診断製品出力証拠を書き出すように、分離 Windows installed smoke を実行し、`python tooling/windows_release_evidence.py` を通過する。
  blocks_release: yes

## 既知の制限

- item: macOS packaged installer は未検証
  classification: known_limitation
  reason: GUI-Shell v1.0 は Windows 優先で、macOS 検証環境は利用できない。
  required_action: macOS installer 対応を主張する前に macOS 上で検証する。
  blocks_release: no

- item: Linux packaged installer はrelease gate対象外
  classification: known_limitation
  reason: Linux は現在、開発・検証の局所範囲であり、Windows 優先の製品 release 対象ではない。
  required_action: Linux build/smoke を開発に有用な状態に保ち、Linux 製品対応を主張する前に Linux installer 検証を追加する。
  blocks_release: no

## 後続 QC

- item: packaged 失敗用の installer recovery 手順が欠落
  classification: post_v1_scope
  reason: 有償/製品 QC には、段階Bの所有者利用操作よりも広い installer recovery と rollback coverage が必要である。
  required_action: 有償/製品 release の前に、installer 失敗 recovery catalog、rollback note、長時間 packaging smoke を追加する。
  blocks_release: no
