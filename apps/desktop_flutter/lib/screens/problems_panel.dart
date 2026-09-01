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
        if (recovery.recoveryId.isNotEmpty) recovery.recoveryId: recovery,
    };
    final authorityByRecoveryId = {
      for (final authority in snapshot.authorityMap)
        if (authority.recoveryId.isNotEmpty) authority.recoveryId: authority,
    };
    return ShellPage(
      title: '問題一覧',
      children: [
        const SectionList(
          title: '段階Bの境界',
          rows: [
            'リリース遮断要因は、段階Bの所有者利用を失敗扱いにせず、ここに表示します。',
            '各行は表示専用です。権限判断と復旧実行は引き続きShell Coreの責任です。',
          ],
        ),
        const SectionList(
          title: '問題と復旧の対応',
          rows: [
            '`safe_to_ignore_for_phase_b=true`は、所有者が段階Bの日常操作を継続できることを示します。',
            '`blocks_owner_use=true`は、段階Bの完了状態を維持する前に所有者利用循環への対応が必要であることを示します。',
            '`blocks_completed_product_release=true`は、後続の厳格リリースを遮断します。',
          ],
        ),
        if (problems.isEmpty)
          const EmptyStatePanel(
            title: '問題なし',
            meaning: '現在のスナップショットには、所有者利用またはリリースの問題がありません。',
            phaseBBlocked: false,
            nextAction: '所有者利用を継続するか、ローカル変更後に診断を更新してください。',
          )
        else
          SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: DataTable(
              columns: const [
                DataColumn(label: Text('項目')),
                DataColumn(label: Text('分類')),
                DataColumn(label: Text('復旧')),
                DataColumn(label: Text('段階Bで継続可能')),
                DataColumn(label: Text('所有者利用を遮断')),
                DataColumn(label: Text('製品リリースを遮断')),
                DataColumn(label: Text('理由')),
                DataColumn(label: Text('必要な対応')),
                DataColumn(label: Text('関連')),
                DataColumn(label: Text('コピー')),
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
      '復旧ID: ${problem.recoveryId}',
      if (problem.target.isNotEmpty) '証拠／パス: ${problem.target}',
      if (authority != null) '実行系ID: ${authority.runtimeId}',
      if (authority != null) '権限: ${authority.capabilityId}',
    ].join('\n');
    return DataRow(
      cells: [
        DataCell(Text(problem.item.isEmpty ? problem.message : problem.item)),
        DataCell(Text(problem.classification)),
        DataCell(Text(problem.recoveryId)),
        DataCell(Text(safeForPhaseB ? 'はい' : 'いいえ')),
        DataCell(Text(blocksOwnerUse ? 'はい' : 'いいえ')),
        DataCell(Text(blocksProductRelease ? 'はい' : 'いいえ')),
        DataCell(Text(problem.reason)),
        DataCell(Text(problem.requiredAction)),
        DataCell(Text(related)),
        DataCell(
          _CopyActions(commandOrPath: commandOrPath, path: problem.target),
        ),
      ],
    );
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
            tooltip: 'コマンドまたはパスをコピー',
            icon: const Icon(Icons.copy, size: 18),
            onPressed: () =>
                Clipboard.setData(ClipboardData(text: commandOrPath)),
          ),
        if (path.isNotEmpty)
          IconButton(
            tooltip: '関連パスをコピー',
            icon: const Icon(Icons.folder_copy_outlined, size: 18),
            onPressed: () => Clipboard.setData(ClipboardData(text: path)),
          ),
      ],
    );
  }
}
