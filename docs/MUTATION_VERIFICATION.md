# 変異検証

状態基準日: 2026-05-25

このファイルは、適合性検査の恒真性修正に対する変異検証の証拠を記録する。破壊した変異コードはコミットしていない。

## 適合性検査の恒真性修正

~~~yaml
- item: 本番authority stripの変異
  mutation_target: packages/shell_core/adapter_loader.py::strip_authority_keys
  mutation: 一時的に入力値を未変更のまま返した。
  expected_failure: 入力authority keyとauthority metadataが除去後も残るため、適合性検査は失敗しなければならない。
  observed_failure: python3 tooling/conformance_tests/run_conformance_skeleton.pyは、除去されなかったauthority、permission_grant、approval_state、role、trust_levelの証拠を伴って失敗した。
  revert_confirmation: 本番strip_authority_keysを復元し、最終の適合性検査は合格した。
  final_validation_result: 従来の変異検証基準線は合格し、現在の拡張適合性基準線は78検査で合格する。

- item: 本番approval can_editの変異
  mutation_target: packages/shell_core/approval_queue.py::ApprovalQueue.can_edit
  mutation: すべてのフィールドについて一時的にTrueを返した。
  expected_failure: authority、sealed、hidden、sacred、protectedの各フィールドが編集可能になるため、適合性検査は失敗しなければならない。
  observed_failure: python3 tooling/conformance_tests/run_conformance_skeleton.pyは、保護フィールドが編集・書き込み可能と報告されて失敗した。
  revert_confirmation: 本番ApprovalQueue.can_editを復元し、最終の適合性検査は合格した。
  final_validation_result: 従来の変異検証基準線は合格し、現在の拡張適合性基準線は78検査で合格する。

- item: 本番approval edit guardの変異
  mutation_target: packages/shell_core/approval_queue.py::ApprovalQueue.edit
  mutation: 保護フィールドのガードを一時的に迂回した。
  expected_failure: 保護フィールドを書き込め、待機中の承認状態が変化するため、適合性検査は失敗しなければならない。
  observed_failure: python3 tooling/conformance_tests/run_conformance_skeleton.pyは、保護フィールドへの書き込みと待機中承認の変異を検出して失敗した。
  revert_confirmation: 本番ApprovalQueue.editのガードを復元し、最終の適合性検査は合格した。
  final_validation_result: 従来の変異検証基準線は合格し、現在の拡張適合性基準線は78検査で合格する。
~~~

## 最終検証

~~~yaml
- command: python3 tooling/schema_check/check_schemas.py
  status: passed
  evidence: schema check passed: 19 schemas, 19 examples, 19 negative fixtures

- command: python3 tooling/conformance_tests/run_conformance_skeleton.py
  status: passed
  evidence: conformance skeleton passed: 78 checks

- command: python3 tooling/validate_all.py
  status: passed
  evidence: 開発検証は、スキーマ、適合性、リリースゲート、Rustヘルパーテスト、デスクトップFlutterの静的解析・テスト・ビルド、およびモバイルFlutter静的解析を伴って合格した。
~~~

## リリース分類

~~~yaml
- item: 適合性検査の恒真性blocker
  classification: required_for_v1
  status: resolved
  reason: 本番authority strippingと本番approval guardの挙動は適合性検査で網羅され、変異検証済みである。
  blocks_release: no

- item: 将来のauthority key重複定義
  classification: release_blocker
  status: policy
  reason: authority keyの重複は、テストローカルまたはモジュールローカルな恒真性を再発させ得る。
  required_action: packages/shell_core/authority_keys.pyをAUTHORITY_KEYSの単一情報源として維持する。
  blocks_release: yes
~~~
