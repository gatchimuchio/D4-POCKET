import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/services/ai_edit_client.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerClientException, BrokerTransport;

void main() {
  test('GUI Shell編集提案は審査待ちReceiptを返し自動applyしない', () async {
    final transport = _FakeAiEditTransport({
      'status': 'accepted',
      'body': {
        'execution_mode': 'proposal_only',
        'review_required': true,
        'files_written': false,
        'permission_generated': false,
      },
    });
    final receipt = await AiEditClient(transport).propose(
      editId: 'edit-settings',
      scope: 'ui',
      instruction: '設定面を確認する',
      targetPaths: const ['apps/desktop_flutter/lib/screens/settings.dart'],
      expectedChanges: const ['owner review'],
    );

    expect(transport.operation, 'GUI Shell編集提案');
    expect(transport.payload?['self_approval'], isFalse);
    expect(receipt['review_required'], isTrue);
    expect(receipt['files_written'], isFalse);
  });

  test('GUI Shell編集提案の拒否を成功へ昇格しない', () async {
    final transport = _FakeAiEditTransport({
      'status': 'rejected',
      'error': {'message': 'owner_required'},
    });

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
}

class _FakeAiEditTransport implements BrokerTransport {
  _FakeAiEditTransport(this.response);

  final Map<String, Object?> response;
  String? operation;
  Map<String, Object?>? payload;

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    this.operation = operation;
    this.payload = payload;
    return response;
  }
}
