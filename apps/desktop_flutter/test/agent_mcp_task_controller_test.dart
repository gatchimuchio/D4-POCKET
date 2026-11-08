import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerClientException, BrokerTransport;

import 'package:gui_shell_desktop/services/agent_mcp_task_controller.dart';
import 'package:gui_shell_desktop/services/agent_task_client.dart';
import 'package:gui_shell_desktop/services/mcp_connection_client.dart';

const _session = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
const _sourceTaskId = 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
const _followupTaskId = 'cccccccccccccccccccccccccccccccc';
const _resultId = 'mcp-agent-result-dddddddddddddddddddddddddddddddd';
const _approvalId = 'eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee';
const _toolId =
    'tool-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
const _sourceResultHash =
    'sha256:1111111111111111111111111111111111111111111111111111111111111111';
const _toolResultHash =
    'sha256:2222222222222222222222222222222222222222222222222222222222222222';
const _contextHash =
    'sha256:3333333333333333333333333333333333333333333333333333333333333333';
const _capabilityHash =
    'sha256:4444444444444444444444444444444444444444444444444444444444444444';
const _workspaceHash =
    'sha256:5555555555555555555555555555555555555555555555555555555555555555';
const _instructionHash =
    'sha256:6666666666666666666666666666666666666666666666666666666666666666';
const _conditionsHash =
    'sha256:7777777777777777777777777777777777777777777777777777777777777777';

void main() {
  test('proposal parserは通常回答と厳密なMCP要求を区別し不正要求を拒否する', () {
    expect(AgentMcpTaskController.parseProposal('通常の回答です'), isNull);
    final valid = AgentMcpTaskController.parseProposal(
      jsonEncode({
        'd4_mcp_tool_request': {
          'server_id': 'mcp-fixture',
          'tool_id': _toolId,
          'name': 'lookup',
          'arguments': {'query': 'Tokyo'},
        },
      }),
    );
    expect(valid?.serverId, 'mcp-fixture');
    expect(valid?.arguments, {'query': 'Tokyo'});

    for (final invalid in [
      '{"d4_mcp_tool_request":',
      jsonEncode({
        'd4_mcp_tool_request': {
          'server_id': 'mcp-fixture',
          'tool_id': _toolId,
          'name': 'lookup',
          'arguments': <String, Object?>{},
          'permission': 'allow',
        },
      }),
      jsonEncode({
        'd4_mcp_tool_request': {
          'server_id': 'mcp-fixture',
          'tool_id': 'tool-invalid',
          'name': 'lookup',
          'arguments': <String, Object?>{},
        },
      }),
    ]) {
      expect(
        () => AgentMcpTaskController.parseProposal(invalid),
        throwsA(isA<BrokerClientException>()),
      );
    }
  });

  test('結果→実Tool→新Permission/Approval→同Agent後続Taskを一回だけ接続する', () async {
    final transport = _FakeBrokerTransport(_successfulResponses());
    final updates = <AgentMcpTaskUpdate>[];
    var ownerReviewWaits = 0;
    final controller = AgentMcpTaskController(
      tasks: AgentTaskClient(transport),
      mcp: McpConnectionClient(transport),
    );

    final outcome = await controller.continueAfter(
      sourceRequest: _sourceRequest,
      sourceTask: _sourceTask,
      isCurrent: () => true,
      waitForOwnerReview: () async => ownerReviewWaits++,
      onUpdate: updates.add,
    );

    expect(outcome.kind, AgentMcpTaskOutcomeKind.followupStarted);
    expect(outcome.toolReceipt?.auditId, 'audit-mcp-tool-call');
    expect(outcome.followupTask?.taskId, _followupTaskId);
    expect(ownerReviewWaits, 4);
    expect(transport.operations, [
      'AgentTask結果表示承認',
      'AgentTask結果取得',
      'MCP接続一覧',
      'MCP Tool実行',
      'Agent作業要求検査',
      'AgentTaskWorkspacePermissionGrant',
      'AgentTaskOwnerApprovalGrant',
      'AgentTask実行',
    ]);
    expect(
      transport.requests.where((entry) => entry['operation'] == 'MCP Tool実行'),
      hasLength(1),
    );
    final followupPayload = transport.requests.last['payload']! as Map;
    expect(followupPayload['session_id'], _session);
    expect(followupPayload['workspace_id'], 'workspace-local');
    expect(followupPayload['mcp_result_id'], _resultId);
    expect(followupPayload['mcp_result_hash'], _contextHash);
    expect(followupPayload['mcp_capability_snapshot_hash'], _capabilityHash);
    expect(followupPayload['instruction'], contains('[D4 Pocket 継続Task]'));
    expect(followupPayload, isNot(contains('permission_id')));
    expect(followupPayload, isNot(contains('approval_id')));
    expect(
      updates.map((update) => update.stage),
      containsAllInOrder([
        AgentMcpTaskStage.resultExposureApproval,
        AgentMcpTaskStage.resultProjectionReady,
        AgentMcpTaskStage.catalogValidation,
        AgentMcpTaskStage.toolOwnerApproval,
        AgentMcpTaskStage.toolCompleted,
        AgentMcpTaskStage.followupPreflight,
        AgentMcpTaskStage.workspacePermission,
        AgentMcpTaskStage.workspacePermissionGranted,
        AgentMcpTaskStage.taskOwnerApproval,
        AgentMcpTaskStage.taskOwnerApprovalGranted,
        AgentMcpTaskStage.followupTaskStarted,
      ]),
    );
    final followupInspection = transport.requests.singleWhere(
      (entry) =>
          entry['operation'] == 'Agent作業要求検査' &&
          ((entry['payload']! as Map)['mcp_result_id'] == _resultId),
    );
    expect(followupInspection['payload'], isNot(contains('permission_id')));
    expect(followupInspection['payload'], isNot(contains('approval_id')));
  });

  test('通常のTask結果ではMCPやPermission/Approvalを要求しない', () async {
    final responses = _successfulResponses(resultText: 'Task完了しました。');
    responses.removeRange(2, responses.length);
    final transport = _FakeBrokerTransport(responses);
    final controller = AgentMcpTaskController(
      tasks: AgentTaskClient(transport),
      mcp: McpConnectionClient(transport),
    );

    final outcome = await controller.continueAfter(
      sourceRequest: _sourceRequest,
      sourceTask: _sourceTask,
      isCurrent: () => true,
      waitForOwnerReview: () async {},
      onUpdate: (_) {},
    );

    expect(outcome.kind, AgentMcpTaskOutcomeKind.noToolProposal);
    expect(transport.operations, ['AgentTask結果表示承認', 'AgentTask結果取得']);
  });

  test('現在のCatalogに提案ToolがなければTool作用前に停止する', () async {
    final responses = _successfulResponses();
    final catalog = responses[2]['body']! as Map<String, Object?>;
    catalog['MCP接続一覧'] = <Object?>[];
    final transport = _FakeBrokerTransport(responses);
    final controller = AgentMcpTaskController(
      tasks: AgentTaskClient(transport),
      mcp: McpConnectionClient(transport),
    );

    await expectLater(
      controller.continueAfter(
        sourceRequest: _sourceRequest,
        sourceTask: _sourceTask,
        isCurrent: () => true,
        waitForOwnerReview: () async {},
        onUpdate: (_) {},
      ),
      throwsA(isA<BrokerClientException>()),
    );
    expect(transport.operations, [
      'AgentTask結果表示承認',
      'AgentTask結果取得',
      'MCP接続一覧',
    ]);
  });

  test('Tool errorのunsupported contextではPermission/Approvalなしで止める', () async {
    final responses = _successfulResponses();
    final receipt = responses[3]['body']! as Map<String, Object?>;
    receipt['Tool error'] = true;
    receipt['Agent結果context'] = {
      '状態': 'unsupported',
      'result_id': null,
      'context_hash': null,
      'tool_result_hash': _toolResultHash,
    };
    final transport = _FakeBrokerTransport(responses);
    final controller = AgentMcpTaskController(
      tasks: AgentTaskClient(transport),
      mcp: McpConnectionClient(transport),
    );

    final outcome = await controller.continueAfter(
      sourceRequest: _sourceRequest,
      sourceTask: _sourceTask,
      isCurrent: () => true,
      waitForOwnerReview: () async {},
      onUpdate: (_) {},
    );

    expect(outcome.kind, AgentMcpTaskOutcomeKind.toolResultUnavailable);
    expect(transport.operations, [
      'AgentTask結果表示承認',
      'AgentTask結果取得',
      'MCP接続一覧',
      'MCP Tool実行',
    ]);
  });

  test('Tool errorをavailable context付きで返されたら後続Taskを開始しない', () async {
    final responses = _successfulResponses();
    (responses[3]['body']! as Map<String, Object?>)['Tool error'] = true;
    final transport = _FakeBrokerTransport(responses);
    final controller = AgentMcpTaskController(
      tasks: AgentTaskClient(transport),
      mcp: McpConnectionClient(transport),
    );

    await expectLater(
      controller.continueAfter(
        sourceRequest: _sourceRequest,
        sourceTask: _sourceTask,
        isCurrent: () => true,
        waitForOwnerReview: () async {},
        onUpdate: (_) {},
      ),
      throwsA(isA<BrokerClientException>()),
    );
    expect(transport.operations, [
      'AgentTask結果表示承認',
      'AgentTask結果取得',
      'MCP接続一覧',
      'MCP Tool実行',
    ]);
  });

  test('後続Taskに既存Permissionが見える場合は新規発行せず停止する', () async {
    final responses = _successfulResponses(permissionStatus: '有効');
    responses.removeLast();
    final transport = _FakeBrokerTransport(responses);
    final controller = AgentMcpTaskController(
      tasks: AgentTaskClient(transport),
      mcp: McpConnectionClient(transport),
    );

    await expectLater(
      controller.continueAfter(
        sourceRequest: _sourceRequest,
        sourceTask: _sourceTask,
        isCurrent: () => true,
        waitForOwnerReview: () async {},
        onUpdate: (_) {},
      ),
      throwsA(isA<BrokerClientException>()),
    );
    expect(transport.operations, [
      'AgentTask結果表示承認',
      'AgentTask結果取得',
      'MCP接続一覧',
      'MCP Tool実行',
      'Agent作業要求検査',
    ]);
  });

  test('現在Taskが失効済みならBroker要求を一切出さない', () async {
    final transport = _FakeBrokerTransport([]);
    final controller = AgentMcpTaskController(
      tasks: AgentTaskClient(transport),
      mcp: McpConnectionClient(transport),
    );

    await expectLater(
      controller.continueAfter(
        sourceRequest: _sourceRequest,
        sourceTask: _sourceTask,
        isCurrent: () => false,
        waitForOwnerReview: () async {},
        onUpdate: (_) {},
      ),
      throwsA(isA<BrokerClientException>()),
    );
    expect(transport.operations, isEmpty);
  });

  test('Catalog確認中にTaskが失効したらTool作用前に停止する', () async {
    var current = true;
    final transport = _FakeBrokerTransport(
      _successfulResponses(),
      afterRequest: (operation) {
        if (operation == 'MCP接続一覧') current = false;
      },
    );
    final controller = AgentMcpTaskController(
      tasks: AgentTaskClient(transport),
      mcp: McpConnectionClient(transport),
    );

    await expectLater(
      controller.continueAfter(
        sourceRequest: _sourceRequest,
        sourceTask: _sourceTask,
        isCurrent: () => current,
        waitForOwnerReview: () async {},
        onUpdate: (_) {},
      ),
      throwsA(isA<BrokerClientException>()),
    );
    expect(transport.operations, [
      'AgentTask結果表示承認',
      'AgentTask結果取得',
      'MCP接続一覧',
    ]);
  });
}

AgentTaskRequest get _sourceRequest => const AgentTaskRequest(
      runtimeId: 'codex-local',
      sessionId: _session,
      workspaceId: 'workspace-local',
      instruction: '検索結果を調べ、目的に応じて必要な変更を行う',
      mcpCapabilitySnapshotHash: _capabilityHash,
    );

AgentTaskRecord get _sourceTask => const AgentTaskRecord(
      taskId: _sourceTaskId,
      runtimeId: 'codex-local',
      sessionId: _session,
      workspaceId: 'workspace-local',
      instructionHash: _instructionHash,
      status: 'completed',
      auditEventId: 'audit-source-task-complete',
      resultHash: _sourceResultHash,
      resultContentAvailable: true,
    );

List<Map<String, Object?>> _successfulResponses({
  String? resultText,
  String permissionStatus = '未付与',
}) {
  final proposal = resultText ??
      jsonEncode({
        'd4_mcp_tool_request': {
          'server_id': 'mcp-fixture',
          'tool_id': _toolId,
          'name': 'lookup',
          'arguments': {'query': 'Tokyo'},
        },
      });
  return [
    _accepted('AgentTask結果表示承認', {
      'task_id': _sourceTaskId,
      'result_hash': _sourceResultHash,
      'content_visibility': 'full',
      'approval_id': _approvalId,
      'expires_at_epoch_seconds': 1900000000,
      'use_limit': 1,
      'uses_remaining': 1,
    }),
    _accepted('AgentTask結果取得', {
      'task_id': _sourceTaskId,
      'result_hash': _sourceResultHash,
      'content_visibility': 'full',
      'projection': {'result_hash': _sourceResultHash, 'text': proposal},
    }),
    _accepted(
        'MCP接続一覧',
        {
          '版': 1,
          'MCP接続一覧': [_connection()],
          '件数': 1,
          'Agent能力hash': _capabilityHash,
          'Agent能力件数': 1,
          'Agent能力状態': 'available',
          '公開範囲': 'metadata_only',
          '証拠種別': 'INTERNAL_STATE',
        },
        evidenceSource: 'INTERNAL_STATE'),
    _accepted(
        'MCP Tool実行',
        {
          '版': 1,
          '契約種別': 'MCP Tool実行receipt',
          'ServerID': 'mcp-fixture',
          'ToolID': _toolId,
          '名前': 'lookup',
          'arguments_hash': _sourceResultHash,
          'result_hash': _toolResultHash,
          'Tool error': false,
          'content_count': 1,
          'content_types': ['text'],
          '接続状態': 'connected',
          '能力ID': 'mcp.tool.call',
          '権限ID': 'permission.mcp.tool.call.one_shot',
          '承認状態': 'native_owner_confirmed',
          '承認監査ID': 'audit-mcp-tool-approval',
          '復旧ID': 'inspect-mcp-tool-side-effect',
          '権限生成': 'Broker内一回限りPermissionを消費',
          '公開範囲': 'hash_only',
          '証拠種別': 'LIVE_RUNTIME',
          '監査ID': 'audit-mcp-tool-call',
          'Agent結果context': {
            '状態': 'available',
            'result_id': _resultId,
            'context_hash': _contextHash,
            'tool_result_hash': _toolResultHash,
          },
        },
        evidenceSource: 'LIVE_RUNTIME'),
    _accepted('Agent作業要求検査', {
      '版': 1,
      '状態': '要求検査済み',
      '実行状態': '未実行',
      'Permission状態': permissionStatus,
      'Approval状態': '未取得',
      '実行系ID': 'codex-local',
      '対話セッションID': _session,
      '作業領域ID': 'workspace-local',
      '指示hash': _instructionHash,
    }),
    _accepted('AgentTaskWorkspacePermissionGrant', {
      'permission_id': 'ffffffffffffffffffffffffffffffff',
      'agent_runtime_id': 'codex-local',
      'session_id': _session,
      'workspace_id': 'workspace-local',
      'workspace_registration_hash': _workspaceHash,
      'operation': 'agent_task.execute',
      'scope': 'session_workspace_once',
      'decision': 'allow',
      'source': 'owner',
      'expires_at_epoch_seconds': 1900000000,
      'use_limit': 1,
      'uses_remaining': 1,
      'status': 'active',
    }),
    _accepted('AgentTaskOwnerApprovalGrant', {
      '状態': 'Owner Approval発行済み',
      '実行状態': '未実行',
      '実行系ID': 'codex-local',
      '対話セッションID': _session,
      '作業領域ID': 'workspace-local',
      '指示hash': _instructionHash,
      '実行条件hash': _conditionsHash,
      '適用ポリシー': 'gui-shell-agent-task-sandbox-v1-max-runtime-900s',
      'expires_at_epoch_seconds': 1900000000,
      'use_limit': 1,
      'uses_remaining': 1,
      'status': 'issued_unconsumed',
    }),
    _accepted('AgentTask実行', {
      'task_id': _followupTaskId,
      'record_version': 2,
      'agent_runtime_id': 'codex-local',
      'session_id': _session,
      'workspace_id': 'workspace-local',
      'description': 'Agent作業Task（結果本文とWorkspace差分は別の権限経路）',
      'instruction_hash': _instructionHash,
      'status': 'running',
      'audit_event_id': 'audit-followup-start',
      'result_content_available': false,
    }),
  ];
}

Map<String, Object?> _connection() => {
      '版': 1,
      '契約種別': 'MCP外部概念射影',
      'Server': {'server_id': 'mcp-fixture', '表示名': 'fixture server'},
      'Transport': {'kind': 'stdio'},
      'Tool': [
        {
          'tool_id': _toolId,
          'name': 'lookup',
          'description_summary': '',
          'input_schema_hash': _capabilityHash,
          'risk': 'unknown',
          'status': {'status': 'supported', 'reason': 'test fixture'},
        },
      ],
      'Resource': <Object?>[],
      'Prompt': <Object?>[],
      'Credential ref': <String, Object?>{},
      'Trust': <String, Object?>{},
      'Capability diff': <String, Object?>{},
      '権限生成': 'なし',
      '公開範囲': 'metadata_only',
      '証拠種別': 'INTERNAL_STATE',
      '接続状態': 'connected',
      '能力ID': 'mcp.connection.connect',
      '権限ID': 'permission.mcp.connection.connect',
      '承認状態': 'owner_control_approved',
      '復旧ID': 'recover-mcp-connection',
      '接続監査ID': 'audit-mcp-connection',
      '実行状態': 'ready',
    };

Map<String, Object?> _accepted(
  String operation,
  Map<String, Object?> body, {
  String? evidenceSource,
}) =>
    {
      'operation': operation,
      'status': 'accepted',
      'audit_event_id': 'audit-$operation',
      'body': body,
      if (evidenceSource != null) 'evidence_source': evidenceSource,
    };

class _FakeBrokerTransport implements BrokerTransport {
  _FakeBrokerTransport(this.responses, {this.afterRequest});

  final List<Map<String, Object?>> responses;
  final void Function(String operation)? afterRequest;
  final operations = <String>[];
  final requests = <Map<String, Object?>>[];

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    operations.add(operation);
    requests.add({'operation': operation, 'payload': payload});
    if (responses.isEmpty) throw StateError('No fake response for $operation');
    final response = responses.removeAt(0);
    afterRequest?.call(operation);
    return response;
  }
}
