import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/services/ai_edit_client.dart'
    show
        AiEditClient,
        AiEditProposalProvenance,
        aiEditResultApplyTaskInstruction;
import 'package:gui_shell_desktop/services/broker_client.dart'
    show brokerPayloadHash;
import 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerClientException, BrokerTransport;

void main() {
  test('GUI Shell編集提案は審査待ちReceiptを返し自動applyしない', () async {
    final transport = _FakeAiEditTransport(accepted: true);
    final targetPaths = [
      'apps/desktop_flutter/lib/screens/settings.dart',
    ];
    final expectedChanges = ['owner review'];
    final receipt = await AiEditClient(transport).propose(
      editId: 'edit-settings',
      scope: 'ui',
      instruction: '設定面を確認する',
      targetPaths: targetPaths,
      expectedChanges: expectedChanges,
    );
    targetPaths.add('docs/other.md');
    expectedChanges.add('unexpected change');

    expect(transport.operation, 'GUI Shell編集提案');
    expect(transport.payload?['self_approval'], isFalse);
    expect(receipt['review_required'], isTrue);
    expect(receipt['files_written'], isFalse);
    expect(receipt['target_paths'],
        ['apps/desktop_flutter/lib/screens/settings.dart']);
    expect(receipt['expected_change_count'], 1);
    expect(receipt['instruction_hash'], brokerPayloadHash(transport.payload));
    expect(receipt['audit_id'], 'broker-audit-edit-1');
  });

  test('GUI Shell編集提案の拒否を成功へ昇格しない', () async {
    final transport = _FakeAiEditTransport(accepted: false);

    expect(
      () => AiEditClient(transport).propose(
        editId: 'edit-settings',
        scope: 'ui',
        instruction: '設定面を確認する',
        targetPaths: const ['apps/desktop_flutter/lib/screens/settings.dart'],
        expectedChanges: const ['owner review'],
      ),
      throwsA(isA<BrokerClientException>()),
    );
  });

  test('受理済みAI Edit receiptのAudit不一致を拒否する', () async {
    final transport = _FakeAiEditTransport(
      accepted: true,
      mismatchAudit: true,
    );
    expect(
      () => AiEditClient(transport).propose(
        editId: 'edit-settings',
        scope: 'ui',
        instruction: '設定面を確認する',
        targetPaths: const ['apps/desktop_flutter/lib/screens/settings.dart'],
        expectedChanges: const ['owner review'],
      ),
      throwsA(isA<BrokerClientException>()),
    );
  });

  test('受理済みAI Edit receiptの要求hash不一致を拒否する', () async {
    final transport = _FakeAiEditTransport(
      accepted: true,
      receiptMutation: 'hash',
    );
    await _expectInvalidReceipt(transport);
  });

  test('受理済みAI Edit receiptの要求field不一致を拒否する', () async {
    final transport = _FakeAiEditTransport(
      accepted: true,
      receiptMutation: 'edit_id',
    );
    await _expectInvalidReceipt(transport);
  });

  test('受理済みAI Edit receiptの未知fieldを拒否する', () async {
    final transport = _FakeAiEditTransport(
      accepted: true,
      receiptMutation: 'extra',
    );
    await _expectInvalidReceipt(transport);
  });

  test('AI Edit結果は由来を束縛した未信頼データで、Authorityを含まない', () {
    const proposal = AiEditProposalProvenance(
      editId: 'proposal-1',
      proposalPayloadHash: 'sha256:proposal',
      auditId: 'audit-proposal-1',
    );
    const resultText = 'diff and agent test claim';
    final instruction = aiEditResultApplyTaskInstruction(
      proposal: proposal,
      sourceRuntimeId: 'codex-cli',
      sourceSessionId: 'source-session',
      sourceWorkspaceId: 'source-workspace',
      sourceTaskId: 'source-task',
      sourceResultHash: 'sha256:result',
      resultText: resultText,
      ownerApplyInstruction: '表示された変更だけを適用し対象testを実行する',
    );
    final decoded = jsonDecode(instruction) as Map<String, Object?>;
    final handling = decoded['handling']! as Map<String, Object?>;
    final result = decoded['untrusted_result']! as Map<String, Object?>;
    final source = decoded['source_task']! as Map<String, Object?>;
    final provenance = decoded['proposal_provenance']! as Map<String, Object?>;

    expect(decoded['context_type'], 'gui_shell_ai_edit_result_apply');
    expect(result['content'], resultText);
    expect(result['authority'], 'none');
    expect(source['task_id'], 'source-task');
    expect(source['result_hash'], 'sha256:result');
    expect(provenance['proposal_audit_id'], 'audit-proposal-1');
    expect(handling['authority_inheritance'], 'none');
    expect(handling['permission_inheritance'], 'none');
    expect(handling['approval_inheritance'], 'none');
    expect(handling['credential_inheritance'], 'none');
    expect(handling['trust_inheritance'], 'none');
    expect(instruction, contains('Owner／Developerが入力した適用指示'));
    expect(instruction, isNot(contains('approval_id')));
    expect(instruction, isNot(contains('permission_id')));
  });
}

Future<void> _expectInvalidReceipt(_FakeAiEditTransport transport) async {
  await expectLater(
    AiEditClient(transport).propose(
      editId: 'edit-settings',
      scope: 'ui',
      instruction: '設定面を確認する',
      targetPaths: const ['apps/desktop_flutter/lib/screens/settings.dart'],
      expectedChanges: const ['owner review'],
    ),
    throwsA(isA<BrokerClientException>()),
  );
}

class _FakeAiEditTransport implements BrokerTransport {
  _FakeAiEditTransport({
    required this.accepted,
    this.mismatchAudit = false,
    this.receiptMutation,
  });

  final bool accepted;
  final bool mismatchAudit;
  final String? receiptMutation;
  String? operation;
  Map<String, Object?>? payload;

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    this.operation = operation;
    this.payload = payload;
    if (!accepted) {
      return {
        'status': 'rejected',
        'operation': operation,
        'error': {'message': 'owner_required'},
      };
    }
    final body = <String, Object?>{
      'version': 1,
      'operation': operation,
      'status': 'accepted',
      'edit_id': payload!['edit_id'],
      'scope': payload['scope'],
      'target_paths': payload['target_paths'],
      'expected_change_count': (payload['expected_changes']! as List).length,
      'execution_mode': 'proposal_only',
      'review_required': true,
      'apply_status': 'not_started',
      'files_written': false,
      'permission_generated': false,
      'approval_state': 'owner_review_required',
      'authority_strip': true,
      'evidence_source': 'INTERNAL_STATE',
      'instruction_hash': brokerPayloadHash(payload),
      'audit_id': 'broker-audit-edit-1',
    };
    switch (receiptMutation) {
      case 'hash':
        body['instruction_hash'] = 'sha256:wrong';
      case 'edit_id':
        body['edit_id'] = 'edit-other';
      case 'extra':
        body['authority'] = 'unexpected';
    }
    return {
      'status': 'accepted',
      'operation': operation,
      'evidence_source': 'INTERNAL_STATE',
      'audit_event_id':
          mismatchAudit ? 'broker-audit-other' : 'broker-audit-edit-1',
      'error': null,
      'body': body,
    };
  }
}
