import 'package:flutter/material.dart';

import '../services/shell_core_client.dart';
import 'shared.dart';

class AgentCenter extends StatelessWidget {
  const AgentCenter({super.key, required this.client});

  final ShellCoreClient client;

  @override
  Widget build(BuildContext context) {
    final sessions = client.getSnapshot().agentSessions;
    return ShellPage(
      title: 'エージェントセンター',
      children: [
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
