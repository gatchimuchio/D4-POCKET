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
      title: '復旧手順',
      children: [
        if (snapshot.recoveryPlaybook.isEmpty)
          const EmptyStatePanel(
            title: '復旧対応なし',
            meaning: '現在のスナップショットに表示対象の復旧手順がありません。',
            phaseBBlocked: false,
            nextAction: '所有者利用を継続するか、検証変更後に診断を更新してください。',
          )
        else
          SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: DataTable(
              columns: const [
                DataColumn(label: Text('復旧')),
                DataColumn(label: Text('項目')),
                DataColumn(label: Text('重大度')),
                DataColumn(label: Text('分類')),
                DataColumn(label: Text('段階Bで継続可能')),
                DataColumn(label: Text('所有者利用を遮断')),
                DataColumn(label: Text('製品リリースを遮断')),
                DataColumn(label: Text('必要な対応')),
                DataColumn(label: Text('コマンド')),
                DataColumn(label: Text('パス')),
                DataColumn(label: Text('コピー')),
              ],
              rows: [
                for (final item in snapshot.recoveryPlaybook)
                  DataRow(
                    cells: [
                      DataCell(Text(item.recoveryId)),
                      DataCell(Text(item.item)),
                      DataCell(Text(item.severity)),
                      DataCell(Text(item.classification)),
                      DataCell(Text(item.safeToIgnoreForPhaseB ? 'はい' : 'いいえ')),
                      DataCell(Text(item.blocksOwnerUse ? 'はい' : 'いいえ')),
                      DataCell(
                        Text(item.blocksCompletedProductRelease ? 'はい' : 'いいえ'),
                      ),
                      DataCell(Text(item.requiredAction)),
                      DataCell(Text(item.command)),
                      DataCell(Text(item.path)),
                      DataCell(_RecoveryCopyActions(item: item)),
                    ],
                  ),
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
                  trailing: Text(recovery.safeToRetry ? '再試行可能' : '遮断中'),
                ),
                SectionList(
                  title: '手順',
                  rows: [
                    '重大度: ${recovery.severity}',
                    '自動修正: 不可',
                    '事前確認: Shell Coreの許可を検証',
                    '対応手順: コマンドをコピー／ログを開く／承認済み能力を通じて再試行',
                    '事後確認: 検証または環境診断を再実行',
                    '巻戻し: 関連する監査事象と復旧対応を使用',
                  ],
                ),
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
            tooltip: 'コマンドをコピー',
            icon: const Icon(Icons.copy, size: 18),
            onPressed: () =>
                Clipboard.setData(ClipboardData(text: item.command)),
          ),
        if (item.path.isNotEmpty)
          IconButton(
            tooltip: 'パスをコピー',
            icon: const Icon(Icons.folder_copy_outlined, size: 18),
            onPressed: () => Clipboard.setData(ClipboardData(text: item.path)),
          ),
      ],
    );
  }
}
