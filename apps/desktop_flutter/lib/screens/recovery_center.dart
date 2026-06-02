import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../models/generated_contracts.dart';
import '../services/shell_core_client.dart';
import 'shared.dart';

class RecoveryCenter extends StatelessWidget {
  const RecoveryCenter({super.key, required this.client});

  final ShellCoreClient client;

  @override
  Widget build(BuildContext context) {
    final snapshot = client.getSnapshot();
    final recoveries = snapshot.recoveryActions;
    return ShellPage(
      title: 'Recovery Playbook',
      children: [
        if (snapshot.recoveryPlaybook.isEmpty)
          const EmptyStatePanel(
            title: 'No recovery action',
            meaning:
                'The current snapshot has no recovery playbook rows to display.',
            phaseBBlocked: false,
            nextAction:
                'Continue owner-use operation or refresh diagnostics after validation changes.',
          )
        else
          SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: DataTable(
              columns: const [
                DataColumn(label: Text('Recovery')),
                DataColumn(label: Text('Item')),
                DataColumn(label: Text('Severity')),
                DataColumn(label: Text('Classification')),
                DataColumn(label: Text('Safe for Phase B')),
                DataColumn(label: Text('Blocks Owner Use')),
                DataColumn(label: Text('Blocks Product Release')),
                DataColumn(label: Text('Required Action')),
                DataColumn(label: Text('Command')),
                DataColumn(label: Text('Path')),
                DataColumn(label: Text('Copy')),
              ],
              rows: [
                for (final item in snapshot.recoveryPlaybook)
                  DataRow(cells: [
                    DataCell(Text(item.recoveryId)),
                    DataCell(Text(item.item)),
                    DataCell(Text(item.severity)),
                    DataCell(Text(item.classification)),
                    DataCell(
                        Text(item.safeToIgnoreForPhaseB ? 'true' : 'false')),
                    DataCell(Text(item.blocksOwnerUse ? 'yes' : 'no')),
                    DataCell(Text(
                        item.blocksCompletedProductRelease ? 'yes' : 'no')),
                    DataCell(Text(item.requiredAction)),
                    DataCell(Text(item.command)),
                    DataCell(Text(item.path)),
                    DataCell(_RecoveryCopyActions(item: item)),
                  ]),
              ],
            ),
          ),
        for (final recovery in recoveries)
          BorderedPanel(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                ListTile(
                  contentPadding: EdgeInsets.zero,
                  leading: const Icon(Icons.health_and_safety_outlined),
                  title: Text(recovery.recoveryId),
                  subtitle: Text(recovery.message),
                  trailing:
                      Text(recovery.safeToRetry ? 'retryable' : 'blocked'),
                ),
                SectionList(title: 'Playbook', rows: [
                  'severity: ${recovery.severity}',
                  'can_auto_fix: false',
                  'pre_check: verify Shell Core authorization',
                  'action_steps: copy command / open logs / retry through approved capability',
                  'post_check: rerun validation or Setup Doctor check',
                  'rollback: use related audit event and recovery mapping',
                ]),
              ],
            ),
          ),
      ],
    );
  }
}

class _RecoveryCopyActions extends StatelessWidget {
  const _RecoveryCopyActions({required this.item});

  final RecoveryPlaybookRecord item;

  @override
  Widget build(BuildContext context) {
    return Wrap(
      spacing: 4,
      children: [
        if (item.command.isNotEmpty)
          IconButton(
            tooltip: 'Copy command',
            icon: const Icon(Icons.copy, size: 18),
            onPressed: () =>
                Clipboard.setData(ClipboardData(text: item.command)),
          ),
        if (item.path.isNotEmpty)
          IconButton(
            tooltip: 'Copy path',
            icon: const Icon(Icons.folder_copy_outlined, size: 18),
            onPressed: () => Clipboard.setData(ClipboardData(text: item.path)),
          ),
      ],
    );
  }
}
