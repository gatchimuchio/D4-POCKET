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
const _workspaceRegistrationHash =
    'sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc';
const _resultHash =
    'sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee';

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

  test('Workspace Permission receiptはSchemaと現在要求への結合を照合する', () async {
    final transport = _FakeBrokerTransport([
      _accepted('AgentTaskWorkspacePermissionGrant', _permissionReceipt()),
    ]);

    await AgentTaskClient(transport).grantWorkspacePermission(_request);

    expect(transport.operations, ['AgentTaskWorkspacePermissionGrant']);
    expect(transport.requests.single['payload'], {
      'agent_runtime_id': _request.runtimeId,
      'session_id': _request.sessionId,
      'workspace_id': _request.workspaceId,
    });
  });

  test('Workspace Permission receiptの誤結合・昇格・再利用を拒否する', () async {
    final invalidReceipts = [
      _permissionReceipt()..['permission_id'] = 'invalid-id',
      _permissionReceipt()..['agent_runtime_id'] = 'other-runtime',
      _permissionReceipt()..['session_id'] = 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
      _permissionReceipt()..['workspace_id'] = 'other-workspace',
      _permissionReceipt()..['workspace_registration_hash'] = 'sha256:bad',
      _permissionReceipt()..['operation'] = 'filesystem_write',
      _permissionReceipt()..['scope'] = 'all_sessions',
      _permissionReceipt()..['decision'] = 'deny',
      _permissionReceipt()..['source'] = 'adapter',
      _permissionReceipt()..['expires_at_epoch_seconds'] = 0,
      _permissionReceipt()..['use_limit'] = 2,
      _permissionReceipt()..['uses_remaining'] = 0,
      _permissionReceipt()..['status'] = 'consumed',
      _permissionReceipt()..['unexpected'] = true,
    ];

    for (final receipt in invalidReceipts) {
      final transport = _FakeBrokerTransport([
        _accepted('AgentTaskWorkspacePermissionGrant', receipt),
      ]);
      await expectLater(
        AgentTaskClient(transport).grantWorkspacePermission(_request),
        throwsA(isA<BrokerClientException>()),
      );
    }
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

  test('結果表示はnative Owner Approvalと一回取得を分離しTask/hash/visibilityへ結合する',
      () async {
    final record = AgentTaskRecord(
      taskId: _task,
      runtimeId: _request.runtimeId,
      sessionId: _request.sessionId,
      workspaceId: _request.workspaceId,
      instructionHash: _instructionHash,
      status: 'completed',
      auditEventId: 'audit-task-complete',
      resultHash: _resultHash,
      resultContentAvailable: true,
    );
    final transport = _FakeBrokerTransport([
      _accepted('AgentTask結果表示承認', _resultApprovalReceipt()),
      _accepted('AgentTask結果取得', _resultReceipt('full')),
    ]);
    final client = AgentTaskClient(transport);

    final approval = await client.grantResultExposure(record, 'full');
    final result = await client.readResult(approval);

    expect(transport.operations, ['AgentTask結果表示承認', 'AgentTask結果取得']);
    expect(transport.requests.first['payload'], {
      'task_id': _task,
      'result_hash': _resultHash,
      'content_visibility': 'full',
    });
    expect(transport.requests.last['payload'], {
      'task_id': _task,
      'approval_id': 'ffffffffffffffffffffffffffffffff',
    });
    expect(result.text, 'Agentが返した未検証報告');
    expect(result.visibility, 'full');
  });

  test('結果表示は未完了・本文なし・hashなし・未知範囲からApprovalを作らない', () async {
    final transport = _FakeBrokerTransport([]);
    final client = AgentTaskClient(transport);
    AgentTaskRecord task(
            {String status = 'completed',
            String? hash = _resultHash,
            bool available = true}) =>
        AgentTaskRecord(
          taskId: _task,
          runtimeId: _request.runtimeId,
          sessionId: _request.sessionId,
          workspaceId: _request.workspaceId,
          instructionHash: _instructionHash,
          status: status,
          auditEventId: 'audit-task-complete',
          resultHash: hash,
          resultContentAvailable: available,
        );

    for (final item in [
      task(status: 'running'),
      task(available: false),
      task(hash: null),
    ]) {
      await expectLater(
        client.grantResultExposure(item, 'full'),
        throwsA(isA<BrokerClientException>()),
      );
    }
    await expectLater(
      client.grantResultExposure(task(), 'unknown'),
      throwsA(isA<BrokerClientException>()),
    );
    expect(transport.operations, isEmpty);
  });

  test('結果Approval receiptの誤結合・再利用・未知fieldとfull本文上限を拒否する', () async {
    final record = AgentTaskRecord(
      taskId: _task,
      runtimeId: _request.runtimeId,
      sessionId: _request.sessionId,
      workspaceId: _request.workspaceId,
      instructionHash: _instructionHash,
      status: 'completed',
      auditEventId: 'audit-task-complete',
      resultHash: _resultHash,
      resultContentAvailable: true,
    );
    final badReceipts = [
      _resultApprovalReceipt()
        ..['task_id'] = 'dddddddddddddddddddddddddddddddd',
      _resultApprovalReceipt()..['result_hash'] = _instructionHash,
      _resultApprovalReceipt()..['content_visibility'] = 'none',
      _resultApprovalReceipt()..['approval_id'] = 'bad',
      _resultApprovalReceipt()..['uses_remaining'] = 2,
      _resultApprovalReceipt()..['unexpected'] = true,
    ];
    for (final receipt in badReceipts) {
      final client = AgentTaskClient(_FakeBrokerTransport([
        _accepted('AgentTask結果表示承認', receipt),
      ]));
      await expectLater(
        client.grantResultExposure(record, 'full'),
        throwsA(isA<BrokerClientException>()),
      );
    }

    const oversized = AgentTaskResultApproval(
      taskId: _task,
      resultHash: _resultHash,
      visibility: 'full',
      approvalId: 'ffffffffffffffffffffffffffffffff',
    );
    final client = AgentTaskClient(_FakeBrokerTransport([
      _accepted(
          'AgentTask結果取得',
          _resultReceipt('full',
              text: List<String>.filled(1048577, 'x').join())),
    ]));
    await expectLater(
      client.readResult(oversized),
      throwsA(isA<BrokerClientException>()),
    );
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

Map<String, Object?> _permissionReceipt() => {
      'permission_id': 'eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee',
      'agent_runtime_id': _request.runtimeId,
      'session_id': _request.sessionId,
      'workspace_id': _request.workspaceId,
      'workspace_registration_hash': _workspaceRegistrationHash,
      'operation': 'agent_task.execute',
      'scope': 'session_workspace_once',
      'decision': 'allow',
      'source': 'owner',
      'expires_at_epoch_seconds': 1900000000,
      'use_limit': 1,
      'uses_remaining': 1,
      'status': 'active',
    };

Map<String, Object?> _taskRecord(String status) => {
      'task_id': _task,
      'record_version': 2,
      'agent_runtime_id': _request.runtimeId,
      'session_id': _request.sessionId,
      'workspace_id': _request.workspaceId,
      'description': 'Agent作業Task（結果本文とWorkspace差分は別の権限経路）',
      'instruction_hash': _instructionHash,
      'status': status,
      'audit_event_id': 'audit-task-start',
      'result_content_available': false,
    };

Map<String, Object?> _resultApprovalReceipt() => {
      'task_id': _task,
      'result_hash': _resultHash,
      'content_visibility': 'full',
      'approval_id': 'ffffffffffffffffffffffffffffffff',
      'expires_at_epoch_seconds': 1900000000,
      'use_limit': 1,
      'uses_remaining': 1,
    };

Map<String, Object?> _resultReceipt(String visibility, {String? text}) => {
      'task_id': _task,
      'result_hash': _resultHash,
      'content_visibility': visibility,
      'projection': visibility == 'full'
          ? {'result_hash': _resultHash, 'text': text ?? 'Agentが返した未検証報告'}
          : null,
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
