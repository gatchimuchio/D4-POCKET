# Audit 証拠

## 現行証拠

- item: AuditEvent schema は中核 audit field を必須とする
  classification: required_for_v1
  status: schema 検証あり

- item: PolicyEvaluator は audit_event.event_id を必須とする
  classification: required_for_v1
  status: conformance あり

- item: payload があるとき PolicyEvaluator は audit_event.payload_hash を必須とする
  classification: required_for_v1
  status: conformance あり

- item: AuditStore は memory 上で前の hash との接続を保つ
  classification: required_for_v1
  status: contract 層の実装あり

- item: audit chain 改変検知
  classification: required_for_v1
  status: 局所 HMAC audit anchor 検証を含む conformance と release smoke あり

- item: Shell Core 永続化と audit smoke
  classification: required_for_v1
  status: `tooling/release_smoke.py` は現行実装経路で snapshot 保存/読込み、追記専用 audit chain 検証、HMAC audit anchor 検証、改変検知を通過する。

- item: MANIFEST 整合性の対象範囲
  classification: required_for_v1
  status: `python3 tooling/manifest.py --check` は通過し、`tooling/release_gate_check.py` に含まれる。
  reason: MANIFEST は Shell Core、tooling、schema、desktop Flutter、Rust helper、root governance/release 文書、docs を対象とする。MANIFEST は完成製品の release readiness を主張しない。
  required_action: `python3 tooling/manifest.py --write` で `MANIFEST.sha256.json` を現行に保つ。MANIFEST は自身を自身の file list から除外する。
  blocks_release: no

## 段階Bの内部操作

- item: 操作者向け audit chain 検証
  classification: required_for_v1
  reason: 段階Bの所有者操作では、desktop shell から audit 状態を見える必要がある。
  required_action: 監査タイムラインと証拠センターの表層を Shell Core snapshot/evidence data に接続した状態に保つ。
  blocks_release: no

- item: audit 出力検証 tooling
  classification: required_for_v1
  reason: evidence bundle 出力は開発証拠用に存在し、実測済み Windows インストール先証拠が通過するまで非権限的な状態を保つ。
  required_action: `tooling/evidence_bundle.py --check` を通過する状態に保つ。
  blocks_release: no

## 残る release blocker

- item: 実測済み Windows インストール先 audit 証拠
  classification: release_blocker
  aggregate_of: windows_evidence_provenance_isolation, windows_installer_first_run_smoke
  reason: 完成製品 release には、インストール済み app path からの config/audit 初期化を証明する native Windows インストール先証拠が依然として必要である。
  required_action: native Windows で実測済み `release_evidence/windows_installed_smoke.json` を生成し、`python tooling/windows_release_evidence.py` を通過する。
  blocks_release: yes

- item: インストール済み app の環境診断製品証拠
  classification: release_blocker
  registry_id: windows_setup_doctor_smoke
  reason: Native Windows launch smoke は開発証拠である。厳格 R2 には、分離 native Windows 実行でインストール済み app が生成した環境診断製品証拠が必要である。現行 PowerShell 環境診断 collector は外部確認証拠に過ぎない。
  required_action: 分離 Windows installed smoke を通じて、インストール済み app が生成した環境診断出力証拠を記録する。実測済み `windows_installed_smoke.json`、環境診断製品証拠、所有者 GO が揃うまで、厳格 release は失敗し続けなければならない。
  blocks_release: yes

- item: audit anchor の外部改変耐性証明
  classification: release_blocker
  registry_id: audit_anchor_external_tamper_evidence_proof
  reason: 局所 HMAC audit anchor 検証だけでは、同一利用者または administrator/root による書換えへの耐性を証明できない。
  required_action: audit anchor ファイル用の Windows ACL/DPAPI、外部 anchor、または署名済み証拠の証明を記録し、厳格 Windows release 検証を通過する。
  blocks_release: yes
