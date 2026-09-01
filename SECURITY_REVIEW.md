# Security レビュー

現行段階: 段階Bの所有者利用は完了。このファイルは内部操作と主張衛生のために security posture を記録するものであり、有償/製品 QC の sign-off ではない。

## 成立済み境界

- item: Shell Core が policy 評価を所有する
  classification: required_for_v1
  status: contract 層 check あり

- item: adapter metadata は非信頼である
  classification: required_for_v1
  status: conformance check あり

- item: memory、cache、previous state、local UI state は権限を与えられない
  classification: required_for_v1
  status: conformance check あり

- item: Rust helper は診断/framing/hash/署名境界の責任だけを持つ
  classification: required_for_v1
  status: source 境界 check あり

- item: Flutter app は操作者表層の責任だけを持つ
  classification: required_for_v1
  status: source 境界 check あり

## リリース遮断要因

- item: Windows インストール先証拠
  classification: release_blocker
  aggregate_of: windows_evidence_provenance_isolation, windows_installer_first_run_smoke, windows_setup_doctor_smoke, windows_broker_installed_smoke
  reason: Windows 優先の完成製品 release には、起動、config、audit、可視表層、分離由来、artifact hash 接続、broker 実測 field 由来、インストール済み app が生成した環境診断製品診断の実測インストール先証拠が必要である。
  required_action: native Windows で `release_evidence/windows_installed_smoke.json` を生成・検証する。
  blocks_release: yes

- item: 所有者 GO
  classification: release_blocker
  registry_id: owner_go
  reason: release 主張の昇格には、blocker 解消後の明示的な所有者承認が必要である。
  required_action: 完成製品 release を主張する前に明示的な所有者 GO を得る。
  blocks_release: yes

## 現行の v1 必須証拠

- item: 永続 audit storage smoke
  classification: required_for_v1
  reason: `tooling/release_smoke.py` は現行実装経路で snapshot 保存/読込みと追記専用 audit chain check を通過する。
  required_action: release smoke を通過する状態に保ち、release 主張前にインストール済み Windows 証拠から同じ経路を証明する。
  blocks_release: no

- item: audit chain 検証 smoke
  classification: required_for_v1
  reason: `tooling/release_smoke.py` は audit chain 接続と改変検知を検証する。
  required_action: audit chain smoke を通過する状態に保つ。
  blocks_release: no

## 後続 security QC

- item: update 機構を出荷する場合の署名済み update 検証
  classification: post_v1_scope
  reason: 所有者が update 機構を明示的に出荷しない限り、update 配布は段階Bの所有者利用操作範囲外である。
  required_action: update 機構をv1.0から除外するか、署名済み update 検証 test を通過する。
  blocks_release: no

- item: installer 挙動レビュー
  classification: required_for_v1
  reason: Windows インストール先証拠検証器は、合成、手動、浅いまたは未実測の証拠を拒否する。
  required_action: release 主張前に強化済み Windows 証拠 collector と検証器を通過する。
  blocks_release: no

- item: 依存関係/license レビュー
  classification: post_v1_scope
  reason: 有償/製品 QC と広範な第三者配布には、段階Bの所有者利用操作より広い依存関係/license レビューが必要である。
  required_action: OSS release candidate または有償/製品 release の前に依存関係/license レビューを追加する。
  blocks_release: no

## v1後の範囲

- item: mobile の暗号学的 pairing
  classification: post_v1_scope
  reason: 所有者が範囲を変更しない限り、mobile 完全 release はv1.0範囲外である。
  blocks_release: no

- item: enterprise admin 向け security
  classification: post_v1_scope
  reason: enterprise admin はv1.0範囲外である。
  blocks_release: no

- item: cloud 向け security
  classification: post_v1_scope
  reason: cloud service はv1.0範囲外である。
  blocks_release: no

## 既知の制限

- item: 局所の単一利用者のみ
  classification: known_limitation
  reason: 受け入れ済みのv1.0製品範囲である。
  required_action: README.md と CLAIM.md を整合させた状態に保つ。
  blocks_release: no
