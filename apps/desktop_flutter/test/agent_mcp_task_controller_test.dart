import 'dart:async';
import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerClientException, BrokerTransport;

import 'package:gui_shell_desktop/services/agent_mcp_task_controller.dart';
import 'package:gui_shell_desktop/services/agent_task_execution_coordinator.dart';
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
      jsonEncode({
        'd4_mcp_tool_request': {
          'server_id': 'mcp-fixture',
          'tool_id': _toolId,
          'name': 'lookup',
          'arguments': Map<String, Object?>.fromEntries(
            List.generate(33, (index) => MapEntry('p$index', index)),
          ),
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
    final controller = AgentMcpTaskController(
      tasks: AgentTaskClient(transport),
      mcp: McpConnectionClient(transport),
    );

    final outcome = await controller.continueAfter(
      sourceRequest: _sourceRequest,
      sourceTask: _sourceTask,
      isCurrent: () => true,
      onUpdate: updates.add,
    );

    expect(outcome.kind, AgentMcpTaskOutcomeKind.followupStarted);
    expect(outcome.toolReceipt?.auditId, 'audit-mcp-tool-call');
    expect(outcome.followupTask?.taskId, _followupTaskId);
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

  test('Agent Task継続は画面observerから独立してBroker経路を続行する', () async {
    final transport = _FakeBrokerTransport(_successfulResponses());
    final coordinator = AgentTaskExecutionCoordinator(transport);
    final updates = <AgentTaskExecutionUpdate>[];
    final subscription = coordinator.updates.listen(updates.add);

    await coordinator.observeStartedTask(
      request: _sourceRequest,
      task: _sourceTask,
    );

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
    expect(transport.requests.last['payload'],
        containsPair('mcp_result_id', _resultId));
    expect(transport.requests.last['payload'],
        containsPair('mcp_result_hash', _contextHash));
    expect(
      updates.any((update) =>
          update.sessionId == _session &&
          update.sourceTaskId == _sourceTaskId &&
          update.mcpUpdate?.stage == AgentMcpTaskStage.followupTaskStarted),
      isTrue,
      reason: '画面callbackなしでBroker継続と表示通知を分離する',
    );

    coordinator.dispose();
    await subscription.cancel();
  });

  test('AIは複数のMCP結果に応じて後続Taskを続け各回で認可を取り直す', () async {
    final secondResultId =
        'mcp-agent-result-${List<String>.filled(32, '9').join()}';
    final secondResultHash = 'sha256:${List<String>.filled(64, 'a').join()}';
    final secondContextHash = 'sha256:${List<String>.filled(64, 'b').join()}';
    final finalTaskId = List<String>.filled(32, '6').join();
    final secondPermissionId = List<String>.filled(32, 'd').join();
    final responses = _successfulResponses();
    final firstFollowup = responses.last['body']! as Map<String, Object?>;
    firstFollowup['status'] = 'completed';
    firstFollowup['audit_event_id'] = 'audit-first-followup-complete';
    firstFollowup['result_hash'] = secondResultHash;
    firstFollowup['result_content_available'] = true;

    final secondRound = _successfulResponses(
      resultText: jsonEncode({
        'd4_mcp_tool_request': {
          'server_id': 'mcp-fixture',
          'tool_id': _toolId,
          'name': 'lookup',
          'arguments': {'query': 'followup result'},
        },
      }),
    );
    Map<String, Object?> responseBody(int index) =>
        secondRound[index]['body']! as Map<String, Object?>;

    responseBody(0)
      ..['task_id'] = _followupTaskId
      ..['result_hash'] = secondResultHash
      ..['approval_id'] = List<String>.filled(32, '7').join();
    final secondProjection =
        responseBody(1)['projection']! as Map<String, Object?>;
    secondProjection['result_hash'] = secondResultHash;
    responseBody(1)
      ..['task_id'] = _followupTaskId
      ..['result_hash'] = secondResultHash;
    final secondReceipt = responseBody(3);
    secondReceipt['arguments_hash'] = secondResultHash;
    secondReceipt['result_hash'] = _conditionsHash;
    secondReceipt['監査ID'] = 'audit-second-mcp-tool-call';
    secondReceipt['承認監査ID'] = 'audit-second-mcp-tool-approval';
    secondReceipt['Agent結果context'] = {
      '状態': 'available',
      'result_id': secondResultId,
      'context_hash': secondContextHash,
      'tool_result_hash': _conditionsHash,
    };
    responseBody(5)['permission_id'] = secondPermissionId;
    responseBody(7)
      ..['task_id'] = finalTaskId
      ..['status'] = 'failed'
      ..['audit_event_id'] = 'audit-second-followup-failed'
      ..['result_content_available'] = false;
    responses.addAll(secondRound);

    final transport = _FakeBrokerTransport(responses);
    final coordinator = AgentTaskExecutionCoordinator(transport);
    final terminalTask = Completer<AgentTaskRecord>();

    await coordinator.observeStartedTask(
      request: _sourceRequest,
      task: _sourceTask,
      onUpdate: (update) {
        final task = update.task;
        if (task?.taskId == finalTaskId &&
            task?.status == 'failed' &&
            !terminalTask.isCompleted) {
          terminalTask.complete(task!);
        }
      },
    );
    await terminalTask.future.timeout(
      const Duration(seconds: 2),
      onTimeout: () => throw StateError(
        '終端Task未観測: operations=${transport.operations.length}',
      ),
    );

    const oneTurn = [
      'AgentTask結果表示承認',
      'AgentTask結果取得',
      'MCP接続一覧',
      'MCP Tool実行',
      'Agent作業要求検査',
      'AgentTaskWorkspacePermissionGrant',
      'AgentTaskOwnerApprovalGrant',
      'AgentTask実行',
    ];
    expect(transport.operations, [...oneTurn, ...oneTurn]);
    expect(
      transport.requests
          .where((entry) => entry['operation'] == 'MCP Tool実行')
          .map((entry) => (entry['payload']! as Map)['source_agent_task_id'])
          .toList(),
      [_sourceTaskId, _followupTaskId],
    );
    expect(
      transport.requests
          .where((entry) => entry['operation'] == 'MCP Tool実行')
          .map(
              (entry) => (entry['payload']! as Map)['source_agent_result_hash'])
          .toList(),
      [_sourceResultHash, secondResultHash],
    );
    final followupRequests = transport.requests
        .where((entry) => entry['operation'] == 'AgentTask実行')
        .map((entry) => entry['payload']! as Map)
        .toList();
    expect(followupRequests, hasLength(2));
    expect(followupRequests[0]['mcp_result_id'], _resultId);
    expect(followupRequests[1]['mcp_result_id'], secondResultId);
    expect(followupRequests[0]['mcp_result_hash'], _contextHash);
    expect(followupRequests[1]['mcp_result_hash'], secondContextHash);
    for (final request in followupRequests) {
      expect(request, isNot(contains('permission_id')));
      expect(request, isNot(contains('approval_id')));
    }
    expect(
      transport.operations.where(
          (operation) => operation == 'AgentTaskWorkspacePermissionGrant'),
      hasLength(2),
    );
    expect(
      transport.operations
          .where((operation) => operation == 'AgentTaskOwnerApprovalGrant'),
      hasLength(2),
    );
    coordinator.dispose();
  });

  test('後続TaskのOwner Approval拒否後はTaskを開始しない', () async {
    final responses = _successfulResponses();
    responses[6] = {
      'operation': 'AgentTaskOwnerApprovalGrant',
      'status': 'rejected',
      'audit_event_id': 'audit-followup-owner-declined',
      'error': {'code': 'owner_denied', 'message': 'Owner declined'},
    };
    final transport = _FakeBrokerTransport(responses);
    final coordinator = AgentTaskExecutionCoordinator(transport);
    final refusal = Completer<AgentTaskExecutionUpdate>();

    await coordinator.observeStartedTask(
      request: _sourceRequest,
      task: _sourceTask,
      onUpdate: (update) {
        if (update.error != null && !refusal.isCompleted) {
          refusal.complete(update);
        }
      },
    );
    final update = await refusal.future.timeout(const Duration(seconds: 2));

    expect(update.error, contains('Agent Task操作がBrokerで拒否されました'));
    expect(
      transport.operations,
      [
        'AgentTask結果表示承認',
        'AgentTask結果取得',
        'MCP接続一覧',
        'MCP Tool実行',
        'Agent作業要求検査',
        'AgentTaskWorkspacePermissionGrant',
        'AgentTaskOwnerApprovalGrant',
      ],
    );
    expect(transport.operations, isNot(contains('AgentTask実行')));
    coordinator.dispose();
  });

  test('disposeは待機中のTask監視pollを解除しBroker Taskを取消しない', () async {
    final transport = _FakeBrokerTransport([]);
    final coordinator = AgentTaskExecutionCoordinator(transport);
    final firstUpdate = Completer<void>();
    final monitoring = coordinator.observeStartedTask(
      request: _sourceRequest,
      task: const AgentTaskRecord(
        taskId: _sourceTaskId,
        runtimeId: 'codex-local',
        sessionId: _session,
        workspaceId: 'workspace-local',
        instructionHash: _instructionHash,
        status: 'running',
        auditEventId: 'audit-pending-monitor-dispose',
      ),
      allowMcpContinuation: false,
      onUpdate: (_) {
        if (!firstUpdate.isCompleted) firstUpdate.complete();
      },
    );

    await firstUpdate.future.timeout(const Duration(seconds: 1));
    coordinator.dispose();
    await monitoring.timeout(const Duration(seconds: 1));

    expect(transport.operations, isEmpty);
  });

  test('同SessionのTask置換は旧Task監視pollを解除する', () async {
    final transport = _FakeBrokerTransport([]);
    final coordinator = AgentTaskExecutionCoordinator(transport);
    final firstUpdate = Completer<void>();
    final oldMonitoring = coordinator.observeStartedTask(
      request: _sourceRequest,
      task: const AgentTaskRecord(
        taskId: _sourceTaskId,
        runtimeId: 'codex-local',
        sessionId: _session,
        workspaceId: 'workspace-local',
        instructionHash: _instructionHash,
        status: 'running',
        auditEventId: 'audit-replaced-monitor',
      ),
      allowMcpContinuation: false,
      onUpdate: (_) {
        if (!firstUpdate.isCompleted) firstUpdate.complete();
      },
    );
    await firstUpdate.future.timeout(const Duration(seconds: 1));

    await coordinator.observeStartedTask(
      request: _sourceRequest,
      task: const AgentTaskRecord(
        taskId: _followupTaskId,
        runtimeId: 'codex-local',
        sessionId: _session,
        workspaceId: 'workspace-local',
        instructionHash: _instructionHash,
        status: 'completed',
        auditEventId: 'audit-current-replacement-task',
      ),
      allowMcpContinuation: false,
      onUpdate: (_) {},
    );
    await oldMonitoring.timeout(const Duration(seconds: 1));

    expect(transport.operations, isEmpty);
    coordinator.dispose();
  });

  test('同じSession識別子でも別Runtime／WorkspaceのTask監視は独立する', () async {
    const scopes = [
      (
        runtimeId: 'codex-first',
        workspaceId: 'workspace-shared',
        taskId: '11111111111111111111111111111111',
        auditId: 'audit-first-task-state',
      ),
      (
        runtimeId: 'codex-second',
        workspaceId: 'workspace-shared',
        taskId: '22222222222222222222222222222222',
        auditId: 'audit-second-task-state',
      ),
      (
        runtimeId: 'codex-first',
        workspaceId: 'workspace-other',
        taskId: '33333333333333333333333333333333',
        auditId: 'audit-third-task-state',
      ),
    ];
    final transport = _FakeBrokerTransport(
      scopes
          .map(
            (scope) => _accepted('AgentTask状態', {
              'task_id': scope.taskId,
              'record_version': 2,
              'agent_runtime_id': scope.runtimeId,
              'session_id': _session,
              'workspace_id': scope.workspaceId,
              'description': 'Agent作業Task（結果本文とWorkspace差分は別の権限経路）',
              'instruction_hash': _instructionHash,
              'status': 'failed',
              'audit_event_id': scope.auditId,
              'result_content_available': false,
            }),
          )
          .toList(),
    );
    final coordinator = AgentTaskExecutionCoordinator(transport);
    final runs = scopes
        .map(
          (scope) => coordinator.observeStartedTask(
            request: AgentTaskRequest(
              runtimeId: scope.runtimeId,
              sessionId: _session,
              workspaceId: scope.workspaceId,
              instruction:
                  'Runtime ${scope.runtimeId}のWorkspace ${scope.workspaceId}のTask',
            ),
            task: AgentTaskRecord(
              taskId: scope.taskId,
              runtimeId: scope.runtimeId,
              sessionId: _session,
              workspaceId: scope.workspaceId,
              instructionHash: _instructionHash,
              status: 'running',
              auditEventId: 'audit-${scope.taskId}-start',
            ),
            allowMcpContinuation: false,
          ),
        )
        .toList();

    await Future.wait(runs).timeout(
      const Duration(seconds: 5),
    );

    final stateRequests = transport.requests
        .where((request) => request['operation'] == 'AgentTask状態')
        .map((request) => (request['payload']! as Map)['task_id'])
        .toSet();
    expect(stateRequests, scopes.map((scope) => scope.taskId).toSet());
    coordinator.dispose();
  });

  test('未完了の画面observer FutureはTask継続を停止しない', () async {
    final observerGate = Completer<void>();
    var observed = 0;
    final coordinator = AgentTaskExecutionCoordinator(
      _FakeBrokerTransport(_successfulResponses()),
    );

    await coordinator
        .observeStartedTask(
          request: _sourceRequest,
          task: const AgentTaskRecord(
            taskId: _followupTaskId,
            runtimeId: 'codex-local',
            sessionId: _session,
            workspaceId: 'workspace-local',
            instructionHash: _instructionHash,
            status: 'completed',
            auditEventId: 'audit-completed-without-continuation',
          ),
          allowMcpContinuation: false,
          onUpdate: (_) {
            observed++;
            return observerGate.future;
          },
        )
        .timeout(const Duration(seconds: 1));

    expect(observed, 1);
    observerGate.complete();
    coordinator.dispose();
  });

  test('同一SessionでTaskが置き換わると旧継続をBroker副作用前に止める', () async {
    final resultGrantGate = Completer<void>();
    final requestStarted = Completer<void>();
    final transport = _FakeBrokerTransport(
      _successfulResponses(),
      blockedRequests: {'AgentTask結果表示承認': resultGrantGate},
      requestStarted: requestStarted,
    );
    final coordinator = AgentTaskExecutionCoordinator(transport);
    final oldRun = coordinator.observeStartedTask(
      request: _sourceRequest,
      task: _sourceTask,
      onUpdate: (_) {},
    );
    await requestStarted.future.timeout(const Duration(seconds: 1));

    await coordinator.observeStartedTask(
      request: _sourceRequest,
      task: const AgentTaskRecord(
        taskId: _followupTaskId,
        runtimeId: 'codex-local',
        sessionId: _session,
        workspaceId: 'workspace-local',
        instructionHash: _instructionHash,
        status: 'completed',
        auditEventId: 'audit-new-current-task',
      ),
      allowMcpContinuation: false,
      onUpdate: (_) {},
    );
    resultGrantGate.complete();
    await oldRun;

    expect(transport.operations, ['AgentTask結果表示承認']);
    coordinator.dispose();
  });

  test('失効した後続Taskの取消要求と未確認状態をSessionへ通知する', () async {
    final result = await _runStaleFollowup();

    expect(result.transport.operations, [
      'AgentTask結果表示承認',
      'AgentTask結果取得',
      'MCP接続一覧',
      'MCP Tool実行',
      'Agent作業要求検査',
      'AgentTaskWorkspacePermissionGrant',
      'AgentTaskOwnerApprovalGrant',
      'AgentTask実行',
      'AgentTask取消',
    ]);
    expect(result.transport.requests.last['payload'], {
      'task_id': _followupTaskId,
    });
    expect(
      result.updates
          .singleWhere((update) => update.sessionRecoveryNotice != null)
          .sessionRecoveryNotice,
      contains('停止完了はBroker状態の再確認が必要です'),
    );
  });

  test('失効した後続Taskの取消要求失敗をSessionへ通知し再送しない', () async {
    final result = await _runStaleFollowup(cancelRequestFails: true);

    expect(result.transport.operations.last, 'AgentTask取消');
    expect(
      result.transport.operations
          .where((operation) => operation == 'AgentTask取消'),
      hasLength(1),
    );
    expect(
      result.updates
          .singleWhere((update) => update.sessionRecoveryNotice != null)
          .sessionRecoveryNotice,
      contains('取消要求に失敗しました'),
    );
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

Future<
    ({
      List<AgentTaskExecutionUpdate> updates,
      _FakeBrokerTransport transport,
    })> _runStaleFollowup({bool cancelRequestFails = false}) async {
  final taskStartGate = Completer<void>();
  final taskStartRequested = Completer<void>();
  final responses = _successfulResponses()
    ..add(
      _accepted('AgentTask取消', {
        'task_id': _followupTaskId,
        'record_version': 2,
        'agent_runtime_id': 'codex-local',
        'session_id': _session,
        'workspace_id': 'workspace-local',
        'description': 'Agent作業Task（結果本文とWorkspace差分は別の権限経路）',
        'instruction_hash': _instructionHash,
        'status': 'running',
        'audit_event_id': 'audit-stale-followup-cancel',
        'result_content_available': false,
      }),
    );
  final transport = _FakeBrokerTransport(
    responses,
    blockedRequests: {'AgentTask実行': taskStartGate},
    requestStarted: taskStartRequested,
    failingOperations: cancelRequestFails ? {'AgentTask取消'} : const {},
  );
  final coordinator = AgentTaskExecutionCoordinator(transport);
  final updates = <AgentTaskExecutionUpdate>[];
  final oldRun = coordinator.observeStartedTask(
    request: _sourceRequest,
    task: _sourceTask,
    onUpdate: updates.add,
  );

  await taskStartRequested.future.timeout(const Duration(seconds: 1));
  await coordinator.observeStartedTask(
    request: _sourceRequest,
    task: const AgentTaskRecord(
      taskId: '99999999999999999999999999999999',
      runtimeId: 'codex-local',
      sessionId: _session,
      workspaceId: 'workspace-local',
      instructionHash: _instructionHash,
      status: 'completed',
      auditEventId: 'audit-replacement-task',
    ),
    allowMcpContinuation: false,
    onUpdate: (_) {},
  );

  taskStartGate.complete();
  await oldRun.timeout(const Duration(seconds: 1));
  coordinator.dispose();
  return (updates: updates, transport: transport);
}

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
  _FakeBrokerTransport(
    this.responses, {
    this.afterRequest,
    this.blockedRequests = const {},
    this.requestStarted,
    this.failingOperations = const {},
  });

  final List<Map<String, Object?>> responses;
  final void Function(String operation)? afterRequest;
  final Map<String, Completer<void>> blockedRequests;
  final Completer<void>? requestStarted;
  final Set<String> failingOperations;
  final operations = <String>[];
  final requests = <Map<String, Object?>>[];

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    operations.add(operation);
    requests.add({'operation': operation, 'payload': payload});
    final started = requestStarted;
    final blockedRequest = blockedRequests[operation];
    if (blockedRequest != null) {
      if (started != null && !started.isCompleted) started.complete();
      await blockedRequest.future;
    }
    if (failingOperations.contains(operation)) {
      throw const BrokerClientException('試験用Broker取消失敗');
    }
    if (responses.isEmpty) throw StateError('No fake response for $operation');
    final response = responses.removeAt(0);
    afterRequest?.call(operation);
    return response;
  }
}
