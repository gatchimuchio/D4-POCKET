import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerClientException, BrokerTransport;

import 'package:gui_shell_desktop/services/agent_task_client.dart';

const _session = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
const _task = 'cccccccccccccccccccccccccccccccc';
const _instructionHash =
    'sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
const _conditionsHash =
    'sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd';

void main() {
  test('事前検査はBrokerの要求・hash・未実行状態を厳密に照合する', () async {
    final transport = _FakeBrokerTransport([
      _accepted('Agent作業要求検査', {
        '版': 1,
        '状態': '要求検査済み',
        '実行状態': '未実行',
        'Permission状態': '未付与',
        'Approval状態': '未取得',
        '実行系ID': 'codex-local',
        '対話セッションID': _session,
        '作業領域ID': 'workspace-local',
        '指示hash': _instructionHash,
      }),
    ]);

    final result = await AgentTaskClient(transport).inspect(_request);

    expect(result.instructionHash, _instructionHash);
    expect(result.permissionStatus, '未付与');
    expect(result.approvalStatus, '未取得');
    expect(transport.operations, ['Agent作業要求検査']);
    expect(transport.requests.single['payload'], _request.toPayload());
  });

  test('未対応gateはTask本文を表示せずPermission／Approvalへ進まない', () async {
    final transport = _FakeBrokerTransport([
      {
        ..._envelope('Agent作業要求検査'),
        'status': 'rejected',
        'error': {
          'code': 'AgentTask実行非対応',
          'message': 'PRIVATE_TASK_SENTINEL',
        },
      },
    ]);

    try {
      await AgentTaskClient(transport).inspect(_request);
      fail('unsupported gate should reject');
    } on BrokerClientException catch (error) {
      expect(error.toString(), contains('AgentTask実行非対応'));
      expect(error.toString(), isNot(contains('PRIVATE_TASK_SENTINEL')));
    }
    expect(transport.operations, ['Agent作業要求検査']);
  });

  test('Owner Approval receiptの旧policy IDと未知fieldを拒否する', () async {
    final receipt = _approvalReceipt()
      ..['適用ポリシー'] = 'gui-shell-agent-task-sandbox-v1';
    final transport = _FakeBrokerTransport([
      _accepted('AgentTaskOwnerApprovalGrant', receipt),
    ]);

    await expectLater(
      AgentTaskClient(transport).grantOwnerApproval(_request),
      throwsA(isA<BrokerClientException>()),
    );
    expect(transport.operations, ['AgentTaskOwnerApprovalGrant']);
  });

  test('Owner Approval receiptは現行policy IDと一回・未消費条件を要求する', () async {
    final transport = _FakeBrokerTransport([
      _accepted('AgentTaskOwnerApprovalGrant', _approvalReceipt()),
    ]);

    await AgentTaskClient(transport).grantOwnerApproval(_request);

    expect(transport.operations, ['AgentTaskOwnerApprovalGrant']);
  });

  test('Task stateはhashと状態だけを受け付け、出力本文fieldは拒否する', () async {
    final record = _taskRecord('running')
      ..['output'] = 'PRIVATE_OUTPUT_SENTINEL';
    final transport = _FakeBrokerTransport([
      _accepted('AgentTask状態', record),
    ]);

    await expectLater(
      AgentTaskClient(transport).state(_task),
      throwsA(isA<BrokerClientException>()),
    );
    expect(transport.operations, ['AgentTask状態']);
  });

  test('Task startとcancelは同じTask ID・実行境界を維持する', () async {
    final transport = _FakeBrokerTransport([
      _accepted('AgentTask実行', _taskRecord('running')),
      _accepted('AgentTask取消', _taskRecord('running')),
    ]);
    final client = AgentTaskClient(transport);

    final started = await client.start(_request);
    final cancelled = await client.cancel(_task);

    expect(started.taskId, _task);
    expect(started.status, 'running');
    expect(cancelled.taskId, _task);
    expect(cancelled.status, 'running');
    expect(transport.operations, ['AgentTask実行', 'AgentTask取消']);
  });

  test('不正なTask要求はBrokerへ送信しない', () async {
    final transport = _FakeBrokerTransport([]);
    const invalid = AgentTaskRequest(
      runtimeId: '../outside',
      sessionId: _session,
      workspaceId: 'workspace-local',
      instruction: 'do work',
    );

    await expectLater(
      AgentTaskClient(transport).inspect(invalid),
      throwsA(isA<BrokerClientException>()),
    );
    expect(transport.operations, isEmpty);
  });
}

const _request = AgentTaskRequest(
  runtimeId: 'codex-local',
  sessionId: _session,
  workspaceId: 'workspace-local',
  instruction: '合成Task instruction',
);

Map<String, Object?> _approvalReceipt() => {
      '状態': 'Owner Approval発行済み',
      '実行状態': '未実行',
      '実行系ID': _request.runtimeId,
      '対話セッションID': _request.sessionId,
      '作業領域ID': _request.workspaceId,
      '指示hash': _instructionHash,
      '実行条件hash': _conditionsHash,
      '適用ポリシー': 'gui-shell-agent-task-sandbox-v1-max-runtime-900s',
      'expires_at_epoch_seconds': 1790000000,
      'use_limit': 1,
      'uses_remaining': 1,
      'status': 'issued_unconsumed',
    };

Map<String, Object?> _taskRecord(String status) => {
      'task_id': _task,
      'record_version': 2,
      'agent_runtime_id': _request.runtimeId,
      'session_id': _request.sessionId,
      'workspace_id': _request.workspaceId,
      'description': 'Agent作業Task（内容は別のWorkspace差分経路で確認）',
      'instruction_hash': _instructionHash,
      'status': status,
      'audit_event_id': 'audit-task-start',
    };

Map<String, Object?> _accepted(String operation, Map<String, Object?> body) => {
      ..._envelope(operation),
      'status': 'accepted',
      'error': null,
      'body': body,
    };

Map<String, Object?> _envelope(String operation) => {
      'operation': operation,
      'audit_event_id': 'audit-${operation.hashCode}',
      'body': null,
    };

class _FakeBrokerTransport implements BrokerTransport {
  _FakeBrokerTransport(this.responses);

  final List<Map<String, Object?>> responses;
  final List<String> operations = [];
  final List<Map<String, Object?>> requests = [];

  @override
  Future<Map<String, Object?>> request(String operation,
      {Map<String, Object?>? payload}) async {
    operations.add(operation);
    requests.add({'operation': operation, 'payload': payload});
    if (responses.isEmpty) {
      throw StateError('fake Broker応答がありません: $operation');
    }
    return responses.removeAt(0);
  }
}
