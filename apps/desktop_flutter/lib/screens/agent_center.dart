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
      title: 'Agent Center',
      children: [
        for (final session in sessions)
          BorderedPanel(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(session.sessionId,
                    style: Theme.of(context).textTheme.titleMedium),
                const SizedBox(height: 8),
                SectionList(title: 'Workspace', rows: [session.workspace]),
                SectionList(title: 'Task', rows: [session.task]),
                SectionList(title: 'Changed Files', rows: session.changedFiles),
                SectionList(title: 'Tool Calls', rows: session.toolCalls),
                SectionList(
                    title: 'Shell Commands', rows: session.shellCommands),
                SectionList(title: 'Test Status', rows: [session.testStatus]),
                SectionList(title: 'Diff Summary', rows: [session.diffSummary]),
                SectionList(
                    title: 'Pending Approvals',
                    rows: ['${session.pendingApprovalCount}']),
                SectionList(
                    title: 'Rollback Candidate',
                    rows: [session.rollbackCandidate]),
                SectionList(title: 'Audit Link', rows: [session.auditEventId]),
              ],
            ),
          ),
      ],
    );
  }
}
