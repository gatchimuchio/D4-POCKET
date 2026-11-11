import 'dart:async';
import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:agent_orchestration/agent_orchestration.dart'
    show AgentTaskCancellationGate, startAgentComparisonTasks;

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
  test('Compare取消はA/Bを並行実行し、同Session重複を拒否して個別に解放する', () async {
    const sessionA = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
    const sessionB = 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
    final gate = AgentTaskCancellationGate();
    final cancelA = Completer<String>();
    final cancelB = Completer<String>();
    var duplicateCalls = 0;

    final requestA = gate.runIfAvailable(sessionA, () => cancelA.future);
    expect(gate.isPending(sessionA), isTrue);

    final duplicate = await gate.runIfAvailable(sessionA, () async {
      duplicateCalls++;
      return 'duplicate';
    });
    expect(duplicate, isNull);
    expect(duplicateCalls, 0);

    final requestB = gate.runIfAvailable(sessionB, () => cancelB.future);
    expect(gate.isPending(sessionA), isTrue);
    expect(gate.isPending(sessionB), isTrue);
    expect(gate.hasPending, isTrue);

    cancelB.complete('cancelled-b');
    expect(await requestB, 'cancelled-b');
    expect(gate.isPending(sessionB), isFalse);
    expect(gate.isPending(sessionA), isTrue);

    final failedRequest = gate.runIfAvailable(sessionB, () async {
      throw StateError('cancel failed');
    });
    expect(gate.isPending(sessionB), isTrue);
    await expectLater(failedRequest, throwsStateError);
    expect(gate.isPending(sessionB), isFalse);
    expect(gate.isPending(sessionA), isTrue);

    cancelA.complete('cancelled-a');
    expect(await requestA, 'cancelled-a');
    expect(gate.hasPending, isFalse);
    expect(gate.tryBegin(''), isFalse);
  });

  test('Compareはfull承認・結果hash一致時だけ上限付きAgent申告成果を投影する', () {
    const resultHash =
        'sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
    AgentTaskResultProjection projection({
      String visibility = 'full',
      String? projectedHash,
      List<Object?> changedFiles = const ['lib/result.dart'],
      String diff = '+result',
    }) =>
        AgentTaskResultProjection(
          taskId: 'task-compare-result',
          resultHash: resultHash,
          visibility: visibility,
          projection: {
            'result_hash': projectedHash ?? resultHash,
            'text': jsonEncode({
              'result_summary': '実装結果の要約',
              'artifacts': [
                {'name': 'patch.txt', 'content': '公開差分'},
              ],
              'changed_files': changedFiles,
              'diff': diff,
              'test_result': 'Agent申告: test passed（未検証）',
            }),
          },
        );

    final visible = projection();
    expect(visible.declaredResult?.summary, '実装結果の要約');
    expect(visible.declaredResult?.changedFiles, ['lib/result.dart']);
    expect(visible.declaredResult?.diff, '+result');
    expect(
      visible.agentClaimedTestResult,
      'Agent申告: test passed（未検証）',
    );
    expect(projection(visibility: 'summary').declaredResult, isNull);
    expect(
      projection(
        projectedHash:
            'sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
      ).declaredResult,
      isNull,
    );
    expect(
      projection(changedFiles: const ['../private/token.txt']).declaredResult,
      isNull,
    );
    expect(projection(diff: 'x' * 8193).declaredResult, isNull);
  });

  test('同じRuntimeのCompare Taskを並行開始し片側失敗を他方へ伝播させない', () async {
    const instruction = '同一の合成Task';
    const sessionA = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
    const sessionB = 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
    const requestA = AgentTaskRequest(
      runtimeId: 'codex-cli',
      sessionId: sessionA,
      workspaceId: 'workspace-a',
      instruction: instruction,
    );
    const requestB = AgentTaskRequest(
      runtimeId: 'codex-cli',
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
      runtimeId: 'codex-cli',
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

  test('CompareはTask本文不一致をdispatch前に拒否する', () async {
    const sessionA = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
    const sessionB = 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
    var dispatchCount = 0;
    const requestA = AgentTaskRequest(
      runtimeId: 'codex-cli',
      sessionId: sessionA,
      workspaceId: 'workspace-a',
      instruction: 'Task A',
    );
    const requestB = AgentTaskRequest(
      runtimeId: 'codex-cli',
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

  test('Compareは共有SessionまたはWorkspaceをdispatch前に拒否する', () async {
    const sessionA = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
    const sessionB = 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
    const requestA = AgentTaskRequest(
      runtimeId: 'codex-cli',
      sessionId: sessionA,
      workspaceId: 'workspace-a',
      instruction: '同じTask',
    );
    var dispatchCount = 0;

    for (final requestB in [
      const AgentTaskRequest(
        runtimeId: 'codex-cli',
        sessionId: sessionA,
        workspaceId: 'workspace-b',
        instruction: '同じTask',
      ),
      const AgentTaskRequest(
        runtimeId: 'codex-cli',
        sessionId: sessionB,
        workspaceId: 'workspace-a',
        instruction: '同じTask',
      ),
    ]) {
      await expectLater(
        startAgentComparisonTasks(
          agentA: requestA,
          agentB: requestB,
          start: (_) async {
            dispatchCount++;
            throw StateError('共有scopeをdispatchしてはならない');
          },
        ),
        throwsA(isA<BrokerClientException>()),
      );
    }
    expect(dispatchCount, 0);
  });

  test('Agent比較は二つの独立Workspaceだけを対象にする', () {
    final single = AgentComparisonProjection.fromSessions([
      _session('a', 'workspace-a'),
    ]);
    expect(single.available, isFalse);

    final isolated = AgentComparisonProjection.fromSessions([
      _session('a', 'workspace-a', agentRuntimeId: 'codex-cli'),
      _session('b', 'workspace-b', agentRuntimeId: 'codex-cli'),
    ]);
    expect(isolated.available, isTrue);
    expect(isolated.statusMessage, contains('同じRuntime IDも比較できます'));
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

  test('Agent比較は識別不能Runtimeを拒否し同一Runtimeを許容する', () {
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
    expect(duplicateRuntime.available, isTrue);
    expect(duplicateRuntime.statusMessage, contains('同じRuntime IDも比較できます'));
  });

  test('Compare結果ApplyのsnapshotはSession・Task・本文の変更を拒否する', () {
    const sessionA = 'agent-a';
    const sessionB = 'agent-b';
    const sessionTarget = 'agent-target';
    final resultHash = 'sha256:${'a' * 64}';
    final sessions = [
      _session(
        sessionA,
        'workspace-a',
        agentRuntimeId: 'codex-cli',
        status: '利用中',
      ),
      _session(
        sessionB,
        'workspace-b',
        agentRuntimeId: 'codex-cli',
        status: '利用中',
      ),
      _session(
        sessionTarget,
        'workspace-target',
        agentRuntimeId: 'codex-cli',
        status: '利用中',
      ),
    ];
    final sourceTask = AgentTaskRecord(
      taskId: 'task-a',
      runtimeId: 'codex-cli',
      sessionId: sessionA,
      workspaceId: 'workspace-a',
      instructionHash: 'sha256:${'b' * 64}',
      status: 'completed',
      auditEventId: 'audit-task-a',
      resultHash: resultHash,
      resultContentAvailable: true,
    );
    AgentTaskResultProjection result(String text) => AgentTaskResultProjection(
          taskId: 'task-a',
          resultHash: resultHash,
          visibility: 'full',
          projection: {'result_hash': resultHash, 'text': text},
        );
    final sourceResult = result('candidate result');
    final startedAt = DateTime.utc(2026, 11, 12);
    final snapshot = AgentSelectedResultApplySnapshot.capture(
      agentASessionId: sessionA,
      agentBSessionId: sessionB,
      selectedSessionId: sessionA,
      comparisonInstructionHash: 'sha256:${'c' * 64}',
      comparisonStartedAt: startedAt,
      agentA: sessions[0],
      agentB: sessions[1],
      source: sessions[0],
      target: sessions[2],
      sourceTask: sourceTask,
      sourceResult: sourceResult,
      targetTask: null,
    );

    bool current({
      String? selected,
      String? selectedA,
      String? selectedB,
      String? instructionHash,
      DateTime? started,
      List<AgentSessionRecord>? liveSessions,
      AgentTaskRecord? liveTask,
      AgentTaskResultProjection? liveResult,
      AgentTaskRecord? targetTask,
      bool targetHasRequest = false,
    }) =>
        snapshot.matchesCurrent(
          currentAgentASessionId: selectedA ?? sessionA,
          currentAgentBSessionId: selectedB ?? sessionB,
          currentSelectedSessionId: selected ?? sessionA,
          currentComparisonInstructionHash:
              instructionHash ?? 'sha256:${'c' * 64}',
          currentComparisonStartedAt: started ?? startedAt,
          currentSessions: liveSessions ?? sessions,
          currentSourceTask: liveTask ?? sourceTask,
          currentSourceResult: liveResult ?? sourceResult,
          currentTargetTask: targetTask,
          currentTargetHasRequest: targetHasRequest,
        );

    expect(current(), isTrue);
    expect(current(selected: sessionB), isFalse);
    expect(current(selectedA: sessionB), isFalse);
    expect(
        current(started: startedAt.add(const Duration(seconds: 1))), isFalse);
    expect(
      current(
        liveTask: AgentTaskRecord(
          taskId: 'task-replaced',
          runtimeId: 'codex-cli',
          sessionId: sessionA,
          workspaceId: 'workspace-a',
          instructionHash: sourceTask.instructionHash,
          status: 'completed',
          auditEventId: sourceTask.auditEventId,
          resultHash: resultHash,
          resultContentAvailable: true,
        ),
      ),
      isFalse,
    );
    expect(current(liveResult: result('changed body with same hash')), isFalse);
    expect(current(targetHasRequest: true), isFalse);
    expect(
      current(
        liveSessions: [
          sessions[0],
          sessions[1],
          _session(
            sessionTarget,
            'workspace-replaced',
            agentRuntimeId: 'codex-cli',
            status: '利用中',
          ),
        ],
      ),
      isFalse,
    );
    expect(
      current(liveSessions: [...sessions, sessions[2]]),
      isFalse,
    );
  });
}
