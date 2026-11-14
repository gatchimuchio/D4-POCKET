import 'package:agent_orchestration/agent_orchestration.dart';
import 'package:flutter_test/flutter_test.dart';

final _idA = List.filled(32, 'a').join();
final _idB = List.filled(32, 'b').join();
final _idC = List.filled(32, 'c').join();
final _idD = List.filled(32, 'd').join();
final _hashF = 'sha256:${List.filled(64, 'f').join()}';

Map<String, Object?> _accepted(String operation, Object body) => {
  'operation': operation,
  'status': 'accepted',
  'audit_event_id': 'broker-audit-mobile-task',
  'evidence_source': 'INTERNAL_STATE',
  'body': body,
  'error': null,
};

Map<String, Object?> _taskRecord({String status = 'pending'}) => {
  'task_id': _idD,
  '実行系ID': 'codex-runtime',
  '対話セッションID': _idA,
  '作業領域ID': 'workspace-a',
  '指示hash': _hashF,
  '状態': status,
  '結果hash': null,
  '結果本文あり': false,
};

void main() {
  test('申請はAuthorityを含まず、Brokerのfalse保証を照合する', () async {
    String? operationSeen;
    Map<String, Object?>? payloadSeen;
    final client = MobileAgentTaskClient((operation, payload) async {
      operationSeen = operation;
      payloadSeen = payload;
      final now = DateTime.now().toUtc().millisecondsSinceEpoch ~/ 1000;
      return _accepted(operation, {
        '版': 1,
        '申請ID': _idB,
        '状態': 'Owner確認待ち',
        '実行系ID': 'codex-runtime',
        '対話セッションID': _idA,
        '作業領域ID': 'workspace-a',
        '指示hash': _hashF,
        '期限': now + 900,
      });
    });

    final draft = await client.submit(
      AgentTaskRequest(
        runtimeId: 'codex-runtime',
        sessionId: _idA,
        workspaceId: 'workspace-a',
        instruction: 'inspect the selected files',
      ),
    );

    expect(operationSeen, 'AgentTask端末申請');
    expect(payloadSeen, {
      'agent_runtime_id': 'codex-runtime',
      'session_id': _idA,
      'workspace_id': 'workspace-a',
      'instruction': 'inspect the selected files',
    });
    expect(draft.instructionHash, _hashF);
    expect(draft.linked, isFalse);
    expect(draft.toAgentTaskRequest().mobileTaskRequestId, _idB);
    expect(
      draft.toAgentTaskRequest().toPayload()['mobile_task_request_id'],
      _idB,
    );
  });

  test('状態確認後の開始と取消は申請IDだけをBrokerへ送る', () async {
    var ready = true;
    final calls = <String>[];
    final client = MobileAgentTaskClient((operation, payload) async {
      calls.add(operation);
      if (operation == 'AgentTask端末状態') {
        return _accepted(operation, {
          '申請ID': _idB,
          '状態': ready ? '開始候補' : 'Owner確認待ち',
        });
      }
      if (operation == 'AgentTask端末実行') {
        expect(payload, {'申請ID': _idB});
        return _accepted(operation, {
          '申請ID': _idB,
          '状態': 'Task開始済み',
          'Task': _taskRecord(),
        });
      }
      if (operation == 'AgentTask端末取消') {
        expect(payload, {'申請ID': _idB});
        return _accepted(operation, {
          '申請ID': _idB,
          '状態': 'Task取消要求済み',
          'Task': _taskRecord(status: 'cancelled'),
        });
      }
      throw StateError('unexpected operation: $operation');
    });
    final draft = MobileAgentTaskDraft(
      requestId: _idB,
      runtimeId: 'codex-runtime',
      sessionId: _idA,
      workspaceId: 'workspace-a',
      instruction: 'inspect the selected files',
      instructionHash: _hashF,
      createdAt: 100,
      expiresAt: 1000,
    );

    expect((await client.status(draft)).canStart, isTrue);
    final task = await client.start(draft);
    expect(task.status, 'pending');
    final cancelled = await client.cancel(draft);
    expect(cancelled.status, 'cancelled');
    expect(calls, ['AgentTask端末状態', 'AgentTask端末実行', 'AgentTask端末取消']);

    ready = false;
    final review = await client.status(draft);
    expect(review.canStart, isFalse);
  });

  test('自端末一覧は本文を返さない指示hash付きprojectionを受理する', () async {
    String? operationSeen;
    final client = MobileAgentTaskClient((operation, payload) async {
      operationSeen = operation;
      expect(payload, isEmpty);
      return _accepted(operation, {
        '申請': [
          {
            '申請ID': _idB,
            '実行系ID': 'codex-runtime',
            '対話セッションID': _idA,
            '作業領域ID': 'workspace-a',
            '指示hash': _hashF,
            '作成時刻': 100,
            '期限': 1000,
            'task_id': null,
          },
        ],
      });
    });

    final requests = await client.ownRequests();

    expect(operationSeen, 'AgentTask端末申請一覧');
    expect(requests, hasLength(1));
    expect(requests.single.requestId, _idB);
    expect(requests.single.instruction, isEmpty);
    expect(requests.single.instructionHash, _hashF);
  });

  test('inbox rejects hidden fields and duplicate request IDs', () async {
    Future<Map<String, Object?>> unsafeInbox(
      String operation,
      Map<String, Object?> payload,
    ) async => _accepted(operation, {
      '申請': [
        {
          '申請ID': _idB,
          '実行系ID': 'codex-runtime',
          '対話セッションID': _idA,
          '作業領域ID': 'workspace-a',
          '指示hash': _hashF,
          '作成時刻': 100,
          '期限': 1000,
          'task_id': null,
          'mobile_task_request_id': _idC,
        },
      ],
    });
    final client = MobileAgentTaskClient(unsafeInbox);
    await expectLater(client.ownRequests(), throwsA(isA<Exception>()));
  });
}
