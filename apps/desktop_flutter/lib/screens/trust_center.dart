import 'package:flutter/material.dart';

import '../services/shell_core_client.dart';
import 'shared.dart';

class TrustCenter extends StatelessWidget {
  const TrustCenter({super.key, required this.client});

  final ShellCoreClient client;

  @override
  Widget build(BuildContext context) {
    final trust = client.getSnapshot().trustRecords;
    return ShellPage(
      title: '信頼センター',
      children: [
        DataTable(
          columns: const [
            DataColumn(label: Text('範囲')),
            DataColumn(label: Text('状態')),
            DataColumn(label: Text('出所')),
            DataColumn(label: Text('有効期限')),
            DataColumn(label: Text('遮断された操作')),
          ],
          rows: [
            for (final item in trust)
              DataRow(
                cells: [
                  DataCell(Text(item.scope)),
                  DataCell(StatusPill(label: '信頼', value: item.state)),
                  DataCell(Text(item.source)),
                  DataCell(Text(item.expiresAt ?? 'なし')),
                  DataCell(Text(item.blockedOperations.join(', '))),
                ],
              ),
          ],
        ),
        const SectionList(
          title: '変更境界',
          rows: [
            '信頼状態の変更には、Shell Coreの能力、許可、承認、監査事象、復旧対応が必要です。',
            '制限、不信、隔離、期限切れ、不明の信頼状態では、エージェント／実行系の実行を制限します。',
          ],
        ),
      ],
    );
  }
}
