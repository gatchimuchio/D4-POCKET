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

    final contaminated = AgentComparisonProjection.fromSessions([
      _session('a', 'workspace-a'),
      _session('b', 'workspace-a'),
    ]);
    expect(contaminated.available, isFalse);
    expect(contaminated.statusMessage, contains('同一Workspace'));
  });

  test('Handoff投影は公開概要と権限再評価だけを示す', () {
    final projection = AgentHandoffProjection.fromSession(
      _session('a', 'workspace-a', task: 'token=do-not-display'),
    );
    expect(projection.taskSummary, '[redacted]');
    expect(projection.statusMessage, contains('再評価'));
  });
}
