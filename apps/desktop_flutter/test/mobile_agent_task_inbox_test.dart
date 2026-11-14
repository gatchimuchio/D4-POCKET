import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/screens/mobile_agent_task_inbox.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart' show BrokerTransport;

const _requestId = 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
const _deviceId = 'cccccccccccccccccccccccccccccccc';
const _sessionId = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
const _instruction = 'inspect the selected files';

void main() {
  testWidgets('Desktop受信箱はBroker下書き本文を表示し読込だけで実行しない', (
    tester,
  ) async {
    final transport = _InboxTransport();
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: MobileAgentTaskInbox(transport: transport)),
      ),
    );

    await tester
        .tap(find.byKey(const ValueKey('mobile-agent-task-inbox-refresh')));
    await tester.pumpAndSettle();

    expect(transport.operations, ['AgentTask端末申請一覧']);
    expect(find.textContaining(_requestId), findsOneWidget);
    expect(find.text(_instruction), findsNothing);

    await tester.tap(find.textContaining(_requestId));
    await tester.pumpAndSettle();

    expect(find.text(_instruction), findsOneWidget);
    expect(transport.operations, ['AgentTask端末申請一覧']);
  });
}

class _InboxTransport implements BrokerTransport {
  final List<String> operations = [];

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    operations.add(operation);
    if (operation != 'AgentTask端末申請一覧' ||
        payload == null ||
        payload.isNotEmpty) {
      throw StateError('unexpected Broker request: $operation');
    }
    return {
      'operation': operation,
      'status': 'accepted',
      'audit_event_id': 'broker-audit-mobile-inbox',
      'evidence_source': 'INTERNAL_STATE',
      'body': {
        '申請': [
          {
            '申請ID': _requestId,
            '端末ID': _deviceId,
            '要求': {
              'agent_runtime_id': 'codex-runtime',
              'session_id': _sessionId,
              'workspace_id': 'workspace-a',
              'instruction': _instruction,
            },
            '作成時刻': 1800000000,
            '期限': 1800000900,
            'task_id': null,
          },
        ],
      },
      'error': null,
    };
  }
}
