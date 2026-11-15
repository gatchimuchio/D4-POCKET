import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/screens/mobile_agent_task_inbox.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart' show BrokerTransport;

const _requestId = 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
const _deviceId = 'cccccccccccccccccccccccccccccccc';
const _sessionId = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
const _taskId = 'dddddddddddddddddddddddddddddddd';
const _resultHash =
    'sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
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

  testWidgets('Broker切替後は受信箱を読み直し旧遅延応答を破棄する', (
    tester,
  ) async {
    final oldInbox = Completer<Map<String, Object?>>();
    final oldTransport = _InboxTransport(
      instruction: 'old broker request',
      inboxResponse: oldInbox,
    );
    final newTransport = _InboxTransport(instruction: 'new broker request');

    await tester.pumpWidget(_inboxApp(oldTransport));
    await tester
        .tap(find.byKey(const ValueKey('mobile-agent-task-inbox-refresh')));
    await tester.pump();
    expect(oldTransport.operations, ['AgentTask端末申請一覧']);

    await tester.pumpWidget(_inboxApp(newTransport));
    await tester.pumpAndSettle();
    expect(newTransport.operations, ['AgentTask端末申請一覧']);

    oldInbox.complete(_inboxResponse('old broker request'));
    await tester.pumpAndSettle();
    await tester.tap(find.textContaining(_requestId));
    await tester.pumpAndSettle();

    expect(find.text('new broker request'), findsOneWidget);
    expect(find.text('old broker request'), findsNothing);
  });

  testWidgets('Broker切替で本文確認を失効し進行中処理を新Brokerへ続けない', (
    tester,
  ) async {
    final permissionResult = Completer<Map<String, Object?>>();
    final oldTransport = _InboxTransport(
      instruction: 'old broker request',
      permissionGrantResponse: permissionResult,
    );
    final newTransport = _InboxTransport(instruction: 'new broker request');

    await tester.pumpWidget(_inboxApp(oldTransport));
    await tester
        .tap(find.byKey(const ValueKey('mobile-agent-task-inbox-refresh')));
    await tester.pumpAndSettle();
    await tester.tap(find.textContaining(_requestId));
    await tester.pumpAndSettle();
    await tester.tap(find.text('依頼本文を確認した'));
    await tester.pump();
    await tester.tap(find.text('Workspace Permissionを個別発行'));
    await tester.pump();
    expect(
        oldTransport.operations, contains('AgentTaskWorkspacePermissionGrant'));

    await tester.pumpWidget(_inboxApp(newTransport));
    await tester.pumpAndSettle();
    permissionResult.complete(_permissionResponse());
    await tester.pumpAndSettle();

    expect(
      newTransport.operations,
      ['AgentTask端末申請一覧'],
    );
    expect(
      oldTransport.operations,
      ['AgentTask端末申請一覧', 'AgentTaskWorkspacePermissionGrant'],
    );

    await tester.tap(find.textContaining(_requestId));
    await tester.pumpAndSettle();
    final review =
        tester.widget<CheckboxListTile>(find.byType(CheckboxListTile));
    expect(review.value, isFalse);
    final grantButton = tester.widget<OutlinedButton>(
      find.widgetWithText(OutlinedButton, 'Workspace Permissionを個別発行'),
    );
    expect(grantButton.onPressed, isNull);
  });

  testWidgets('Broker切替で前Brokerの結果dialogを閉じる', (tester) async {
    final oldTransport = _InboxTransport(
      instruction: 'old broker request',
      taskId: _taskId,
    );
    final newTransport = _InboxTransport(includeRequest: false);

    await tester.pumpWidget(_inboxApp(oldTransport));
    await tester
        .tap(find.byKey(const ValueKey('mobile-agent-task-inbox-refresh')));
    await tester.pumpAndSettle();
    await tester.tap(find.textContaining(_requestId));
    await tester.pumpAndSettle();
    await tester.tap(find.text('状態をBrokerから取得'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('native Owner確認後に結果／diffを表示'));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 300));
    expect(find.text('Summary: old broker result'), findsOneWidget);

    await tester.pumpWidget(_inboxApp(newTransport));
    await tester.pumpAndSettle();

    expect(find.text('Summary: old broker result'), findsNothing);
    expect(find.text('Agent Task結果（未検証のAgent申告）'), findsNothing);
  });
}

Widget _inboxApp(BrokerTransport transport) => MaterialApp(
      home: Scaffold(
        body: SingleChildScrollView(
          child: MobileAgentTaskInbox(transport: transport),
        ),
      ),
    );

Map<String, Object?> _accepted(String operation, Map<String, Object?> body) => {
      'operation': operation,
      'status': 'accepted',
      'audit_event_id': 'broker-audit-mobile-inbox',
      'evidence_source': 'INTERNAL_STATE',
      'body': body,
      'error': null,
    };

Map<String, Object?> _inboxResponse(
  String instruction, {
  String? taskId,
  bool includeRequest = true,
}) =>
    _accepted(
      'AgentTask端末申請一覧',
      {
        '申請': includeRequest
            ? [
                {
                  '申請ID': _requestId,
                  '端末ID': _deviceId,
                  '要求': {
                    'agent_runtime_id': 'codex-runtime',
                    'session_id': _sessionId,
                    'workspace_id': 'workspace-a',
                    'instruction': instruction,
                  },
                  '作成時刻': 1800000000,
                  '期限': 1800000900,
                  'task_id': taskId,
                },
              ]
            : [],
      },
    );

Map<String, Object?> _taskResponse() => _accepted('AgentTask状態', {
      'task_id': _taskId,
      'record_version': 2,
      'agent_runtime_id': 'codex-runtime',
      'session_id': _sessionId,
      'workspace_id': 'workspace-a',
      'description': 'Agent作業Task（結果本文とWorkspace差分は別の権限経路）',
      'instruction_hash':
          'sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
      'status': 'completed',
      'audit_event_id': 'broker-audit-task-complete',
      'result_hash': _resultHash,
      'result_content_available': true,
    });

Map<String, Object?> _resultApprovalResponse() => _accepted(
      'AgentTask結果表示承認',
      {
        'task_id': _taskId,
        'result_hash': _resultHash,
        'content_visibility': 'full',
        'approval_id': _deviceId,
        'expires_at_epoch_seconds': 1800000900,
        'use_limit': 1,
        'uses_remaining': 1,
      },
    );

Map<String, Object?> _resultResponse() => _accepted('AgentTask結果取得', {
      'task_id': _taskId,
      'result_hash': _resultHash,
      'content_visibility': 'full',
      'projection': {
        'result_hash': _resultHash,
        'text': jsonEncode({
          'result_summary': 'old broker result',
          'artifacts': <Object?>[],
          'changed_files': <String>[],
          'diff': '',
          'test_result': 'pass',
        }),
      },
    });

Map<String, Object?> _permissionResponse() => _accepted(
      'AgentTaskWorkspacePermissionGrant',
      {
        'permission_id': _deviceId,
        'agent_runtime_id': 'codex-runtime',
        'session_id': _sessionId,
        'workspace_id': 'workspace-a',
        'workspace_registration_hash':
            'sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
        'operation': 'agent_task.execute',
        'scope': 'session_workspace_once',
        'decision': 'allow',
        'source': 'owner',
        'expires_at_epoch_seconds': 1800000900,
        'use_limit': 1,
        'uses_remaining': 1,
        'status': 'active',
      },
    );

class _InboxTransport implements BrokerTransport {
  _InboxTransport({
    this.instruction = _instruction,
    this.inboxResponse,
    this.permissionGrantResponse,
    this.taskId,
    this.includeRequest = true,
  });

  final String instruction;
  final Completer<Map<String, Object?>>? inboxResponse;
  final Completer<Map<String, Object?>>? permissionGrantResponse;
  final String? taskId;
  final bool includeRequest;
  final List<String> operations = [];

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    operations.add(operation);
    if (payload == null) throw StateError('missing payload: $operation');
    if (operation == 'AgentTask端末申請一覧' && payload.isEmpty) {
      final response = inboxResponse;
      if (response != null) return await response.future;
      return _inboxResponse(
        instruction,
        taskId: taskId,
        includeRequest: includeRequest,
      );
    }
    if (operation == 'AgentTaskWorkspacePermissionGrant') {
      final response = permissionGrantResponse;
      if (response != null) return await response.future;
      return _permissionResponse();
    }
    if (operation == 'AgentTask状態') return _taskResponse();
    if (operation == 'AgentTask結果表示承認') {
      return _resultApprovalResponse();
    }
    if (operation == 'AgentTask結果取得') return _resultResponse();
    if (operation == 'Agent作業要求検査') {
      return _accepted(operation, {
        '版': 1,
        '状態': '要求検査済み',
        '実行状態': '未実行',
        'Permission状態': '有効',
        'Approval状態': '未取得',
        '実行系ID': 'codex-runtime',
        '対話セッションID': _sessionId,
        '作業領域ID': 'workspace-a',
        '指示hash':
            'sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
      });
    }
    throw StateError('unexpected Broker request: $operation');
  }
}
