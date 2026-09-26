import 'package:flutter_test/flutter_test.dart';

import 'package:gui_shell_desktop/models/generated_contracts.dart';
import 'package:gui_shell_desktop/services/agent_coordination.dart';

AgentSessionRecord _session(
  String id,
  String workspace, {
  String task = '文書を更新する',
}) {
  return AgentSessionRecord(
    sessionId: id,
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

  test('Handoff投影は公開概要と権限再評価だけを示す', () {
    final projection = AgentHandoffProjection.fromSession(
      _session('a', 'workspace-a', task: 'token=do-not-display'),
    );
    expect(projection.taskSummary, '[redacted]');
    expect(projection.statusMessage, contains('再評価'));
  });
}
