import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../models/generated_contracts.dart';
import '../services/shell_core_client.dart';
import 'shared.dart';

class ProblemsPanel extends StatelessWidget {
  const ProblemsPanel({super.key, required this.client});

  final ShellCoreClient client;

  @override
  Widget build(BuildContext context) {
    final snapshot = client.getSnapshot();
    final problems = snapshot.problems;
    final recoveryById = {
      for (final recovery in snapshot.recoveryPlaybook)
        if (recovery.recoveryId.isNotEmpty) recovery.recoveryId: recovery
    };
    final authorityByRecoveryId = {
      for (final authority in snapshot.authorityMap)
        if (authority.recoveryId.isNotEmpty) authority.recoveryId: authority
    };
    return ShellPage(
      title: 'Problems Panel',
      children: [
        const SectionList(title: 'Phase B Boundary', rows: [
          'Release blockers are visible here without making Phase B owner-use fail.',
          'Rows are display-only; authority and recovery execution remain Shell Core responsibilities.',
        ]),
        const SectionList(title: 'Problem to Recovery Map', rows: [
          'safe_to_ignore_for_phase_b=true means the owner can continue Phase B daily operation.',
          'blocks_owner_use=true means the owner-use loop needs attention before Phase B can stay complete.',
          'blocks_completed_product_release=true remains a later strict release blocker.',
        ]),
        if (problems.isEmpty)
          const EmptyStatePanel(
            title: 'No problems',
            meaning:
                'The current snapshot reports no owner-use or release problems.',
            phaseBBlocked: false,
            nextAction:
                'Continue owner-use operation or refresh diagnostics after local changes.',
          )
        else
          SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: DataTable(
              columns: const [
                DataColumn(label: Text('Item')),
                DataColumn(label: Text('Classification')),
                DataColumn(label: Text('Recovery')),
                DataColumn(label: Text('Safe for Phase B')),
                DataColumn(label: Text('Blocks Owner Use')),
                DataColumn(label: Text('Blocks Product Release')),
                DataColumn(label: Text('Reason')),
                DataColumn(label: Text('Required Action')),
                DataColumn(label: Text('Related')),
                DataColumn(label: Text('Copy')),
              ],
              rows: [
                for (final problem in problems)
                  _problemRow(problem, recoveryById, authorityByRecoveryId),
              ],
            ),
          ),
      ],
    );
  }

  DataRow _problemRow(
    ProblemRecord problem,
    Map<String, RecoveryPlaybookRecord> recoveryById,
    Map<String, AuthorityMapRecord> authorityByRecoveryId,
  ) {
    final recovery = recoveryById[problem.recoveryId];
    final authority = authorityByRecoveryId[problem.recoveryId];
    final safeForPhaseB = problem.safeToIgnoreForPhaseB ||
        recovery?.safeToIgnoreForPhaseB == true;
    final blocksOwnerUse =
        problem.blocksOwnerUse || recovery?.blocksOwnerUse == true;
    final blocksProductRelease = problem.blocksCompletedProductRelease ||
        problem.blocksRelease ||
        recovery?.blocksCompletedProductRelease == true;
    final commandOrPath = [
      if (recovery?.command.isNotEmpty == true) recovery!.command,
      if (recovery?.path.isNotEmpty == true) recovery!.path,
      if (recovery == null) problem.target,
    ].join(' | ');
    final related = [
      'recovery_id: ${problem.recoveryId}',
      if (problem.target.isNotEmpty) 'evidence/path: ${problem.target}',
      if (authority != null) 'runtime_id: ${authority.runtimeId}',
      if (authority != null) 'authority: ${authority.capabilityId}',
    ].join('\n');
    return DataRow(cells: [
      DataCell(Text(problem.item.isEmpty ? problem.message : problem.item)),
      DataCell(Text(problem.classification)),
      DataCell(Text(problem.recoveryId)),
      DataCell(Text(safeForPhaseB ? 'true' : 'false')),
      DataCell(Text(blocksOwnerUse ? 'yes' : 'no')),
      DataCell(Text(blocksProductRelease ? 'yes' : 'no')),
      DataCell(Text(problem.reason)),
      DataCell(Text(problem.requiredAction)),
      DataCell(Text(related)),
      DataCell(
          _CopyActions(commandOrPath: commandOrPath, path: problem.target)),
    ]);
  }
}

class _CopyActions extends StatelessWidget {
  const _CopyActions({required this.commandOrPath, required this.path});

  final String commandOrPath;
  final String path;

  @override
  Widget build(BuildContext context) {
    return Wrap(
      spacing: 4,
      children: [
        if (commandOrPath.isNotEmpty)
          IconButton(
            tooltip: 'Copy command or path',
            icon: const Icon(Icons.copy, size: 18),
            onPressed: () => Clipboard.setData(
              ClipboardData(text: commandOrPath),
            ),
          ),
        if (path.isNotEmpty)
          IconButton(
            tooltip: 'Copy related path',
            icon: const Icon(Icons.folder_copy_outlined, size: 18),
            onPressed: () => Clipboard.setData(ClipboardData(text: path)),
          ),
      ],
    );
  }
}
