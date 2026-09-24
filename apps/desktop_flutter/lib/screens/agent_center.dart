import 'package:flutter/material.dart';

import '../services/shell_core_client.dart';
import '../services/agent_coordination.dart';
import 'shared.dart';
import 'workspace_inspector.dart';

class AgentCenter extends StatelessWidget {
  const AgentCenter({super.key, required this.client});

  final ShellCoreClient client;

  @override
  Widget build(BuildContext context) {
    final sessions = client.getSnapshot().agentSessions;
    final comparison = AgentComparisonProjection.fromSessions(sessions);
    return ShellPage(
      title: 'エージェントセンター',
      children: [
        if (client.workspaceClient != null)
          BorderedPanel(
              child: WorkspaceInspector(client: client.workspaceClient!)),
        BorderedPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                '複数Agent比較',
                style: Theme.of(context).textTheme.titleMedium,
              ),
              const SizedBox(height: 8),
              SectionList(title: '状態', rows: [comparison.statusMessage]),
              SectionList(
                title: '対象セッション',
                rows: comparison.sessionIds.isEmpty
                    ? ['なし']
                    : comparison.sessionIds,
              ),
              SectionList(
                title: '安全境界',
                rows: [
                  comparison.available
                      ? 'Workspace隔離を確認済み'
                      : '比較実行はBroker接続済みの投影だけに限定',
                  'Authority・Approval・Credentialは共有しない',
                ],
              ),
            ],
          ),
        ),
        for (final session in sessions)
          BorderedPanel(
            child: _HandoffPanel(
              projection: AgentHandoffProjection.fromSession(session),
            ),
          ),
        for (final session in sessions)
          BorderedPanel(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  session.sessionId,
                  style: Theme.of(context).textTheme.titleMedium,
                ),
                const SizedBox(height: 8),
                SectionList(title: '作業領域', rows: [session.workspace]),
                SectionList(title: 'タスク', rows: [session.task]),
                SectionList(title: '変更ファイル', rows: session.changedFiles),
                SectionList(title: '道具呼出し', rows: session.toolCalls),
                SectionList(title: 'シェルコマンド', rows: session.shellCommands),
                SectionList(title: '試験状態', rows: [session.testStatus]),
                SectionList(title: '差分概要', rows: [session.diffSummary]),
                SectionList(
                  title: '保留中の承認',
                  rows: ['${session.pendingApprovalCount}'],
                ),
                SectionList(title: '巻戻し候補', rows: [session.rollbackCandidate]),
                SectionList(title: '監査リンク', rows: [session.auditEventId]),
              ],
            ),
          ),
      ],
    );
  }
}

class _HandoffPanel extends StatelessWidget {
  const _HandoffPanel({required this.projection});

  final AgentHandoffProjection projection;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          'Agent Handoff投影: ${projection.sessionId}',
          style: Theme.of(context).textTheme.titleMedium,
        ),
        const SizedBox(height: 8),
        SectionList(title: '状態', rows: [projection.statusMessage]),
        SectionList(title: 'Task概要', rows: [projection.taskSummary]),
        SectionList(title: '差分概要', rows: [projection.diffSummary]),
        SectionList(title: '試験状態', rows: [projection.testStatus]),
      ],
    );
  }
}
