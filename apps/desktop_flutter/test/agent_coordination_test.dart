import 'dart:async';

import 'package:flutter_test/flutter_test.dart';

import 'package:gui_shell_desktop/models/generated_contracts.dart';
import 'package:gui_shell_desktop/services/agent_task_client.dart';
import 'package:gui_shell_desktop/services/agent_coordination.dart';
import 'package:gui_shell_desktop/services/broker_client.dart'
    show BrokerClientException;

AgentSessionRecord _session(
  String id,
  String workspace, {
  String? agentRuntimeId,
  String task = '文書を更新する',
  String status = 'completed',
}) {
  return AgentSessionRecord(
    sessionId: id,
    agentRuntimeId: agentRuntimeId ?? 'runtime-$id',
    status: status,
    evidenceSource: 'FIXTURE',
    workspace: workspace,
    task: task,
    changedFiles: const ['README.md'],
    toolCalls: const ['git.diff'],
    shellCommands: const [],
    testStatus: '未確認',
    diffSummary: '1 file changed',
    pendingApprovalCount: 0,
    rollbackCandidate: 'rollback-$id',
    auditEventId: 'audit-$id',
  );
}

void main() {
  test('Compareは両Taskを並行開始し片側失敗を他方へ伝播させない', () async {
    const instruction = '同一の合成Task';
    const sessionA = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
    const sessionB = 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
    const requestA = AgentTaskRequest(
      runtimeId: 'codex-a',
      sessionId: sessionA,
      workspaceId: 'workspace-a',
      instruction: instruction,
    );
    const requestB = AgentTaskRequest(
      runtimeId: 'codex-b',
      sessionId: sessionB,
      workspaceId: 'workspace-b',
      instruction: instruction,
    );
    final first = Completer<AgentTaskRecord>();
    final second = Completer<AgentTaskRecord>();
    final dispatched = <String>[];

    final result = startAgentComparisonTasks(
      agentA: requestA,
      agentB: requestB,
      start: (request) {
        dispatched.add(request.sessionId);
        return request.sessionId == sessionA ? first.future : second.future;
      },
    );
    await Future<void>.delayed(Duration.zero);
    expect(dispatched, [sessionA, sessionB]);

    second.complete(const AgentTaskRecord(
      taskId: 'cccccccccccccccccccccccccccccccc',
      runtimeId: 'codex-b',
      sessionId: sessionB,
      workspaceId: 'workspace-b',
      instructionHash:
          'sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
      status: 'running',
      auditEventId: 'audit-task-b',
    ));
    first.completeError(const BrokerClientException('Agent A rejected'));

    final outcomes = await result;
    expect(outcomes, hasLength(2));
    expect(outcomes[0].record, isNull);
    expect(outcomes[0].error, isA<BrokerClientException>());
    expect(outcomes[1].record?.sessionId, sessionB);
    expect(outcomes[1].error, isNull);
  });

  test('CompareはTask不一致または共有識別子ならdispatch前に拒否する', () async {
    const sessionA = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
    const sessionB = 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
    var dispatchCount = 0;
    const requestA = AgentTaskRequest(
      runtimeId: 'codex-a',
      sessionId: sessionA,
      workspaceId: 'workspace-a',
      instruction: 'Task A',
    );
    const requestB = AgentTaskRequest(
      runtimeId: 'codex-b',
      sessionId: sessionB,
      workspaceId: 'workspace-b',
      instruction: 'Task B',
    );

    await expectLater(
      startAgentComparisonTasks(
        agentA: requestA,
        agentB: requestB,
        start: (_) async {
          dispatchCount++;
          throw StateError('要求を送信してはならない');
        },
      ),
      throwsA(isA<BrokerClientException>()),
    );
    expect(dispatchCount, 0);
  });

  test('Agent比較は二つの独立Workspaceだけを対象にする', () {
    final single = AgentComparisonProjection.fromSessions([
      _session('a', 'workspace-a'),
    ]);
    expect(single.available, isFalse);

    final isolated = AgentComparisonProjection.fromSessions([
      _session('a', 'workspace-a'),
      _session('b', 'workspace-b'),
    ]);
    expect(isolated.available, isTrue);
    expect(isolated.statusMessage, contains('Agent runtime IDとWorkspace参照'));
    expect(isolated.statusMessage, contains('実行時の隔離は未検証'));

    final contaminated = AgentComparisonProjection.fromSessions([
      _session('a', 'workspace-a'),
      _session('b', 'workspace-a'),
    ]);
    expect(contaminated.available, isFalse);
    expect(contaminated.statusMessage, contains('同一Workspace'));
  });

  test('Agent比較は件数と識別できる参照を契約範囲内で要求する', () {
    final missingWorkspace = AgentComparisonProjection.fromSessions([
      _session('a', 'workspace-a'),
      _session('b', '   '),
    ]);
    expect(missingWorkspace.available, isFalse);
    expect(missingWorkspace.statusMessage, contains('Workspace参照'));

    final missingSessionId = AgentComparisonProjection.fromSessions([
      _session('', 'workspace-a'),
      _session('b', 'workspace-b'),
    ]);
    expect(missingSessionId.available, isFalse);
    expect(missingSessionId.statusMessage, contains('セッション識別子'));

    final malformedSessionId = AgentComparisonProjection.fromSessions([
      _session('agent/one', 'workspace-a'),
      _session('agent-two', 'workspace-b'),
    ]);
    expect(malformedSessionId.available, isFalse);
    expect(malformedSessionId.statusMessage, contains('セッション識別子'));

    final duplicateSessionId = AgentComparisonProjection.fromSessions([
      _session('agent-one', 'workspace-a'),
      _session('agent-one', 'workspace-b'),
    ]);
    expect(duplicateSessionId.available, isFalse);
    expect(duplicateSessionId.statusMessage, contains('重複'));

    final tooMany = AgentComparisonProjection.fromSessions([
      for (var index = 0; index < 9; index++)
        _session('agent-$index', 'workspace-$index'),
    ]);
    expect(tooMany.available, isFalse);
    expect(tooMany.statusMessage, contains('8件'));
  });

  test('Agent比較は識別不能または重複したAgent実行系を拒否する', () {
    final missingRuntime = AgentComparisonProjection.fromSessions([
      _session('agent-one', 'workspace-a', agentRuntimeId: ''),
      _session('agent-two', 'workspace-b'),
    ]);
    expect(missingRuntime.available, isFalse);
    expect(missingRuntime.statusMessage, contains('Agent実行系識別子'));

    final malformedRuntime = AgentComparisonProjection.fromSessions([
      _session('agent-one', 'workspace-a', agentRuntimeId: 'agent/one'),
      _session('agent-two', 'workspace-b'),
    ]);
    expect(malformedRuntime.available, isFalse);
    expect(malformedRuntime.statusMessage, contains('Agent実行系識別子'));

    final duplicateRuntime = AgentComparisonProjection.fromSessions([
      _session('agent-one', 'workspace-a', agentRuntimeId: 'codex-cli'),
      _session('agent-two', 'workspace-b', agentRuntimeId: 'codex-cli'),
    ]);
    expect(duplicateRuntime.available, isFalse);
    expect(duplicateRuntime.statusMessage, contains('同一Agent実行系'));
  });
}
