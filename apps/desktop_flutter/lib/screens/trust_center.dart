import 'package:flutter/material.dart';

import '../services/shell_core_client.dart';
import 'shared.dart';

class TrustCenter extends StatelessWidget {
  const TrustCenter({super.key, required this.client});

  final ShellCoreClient client;

  @override
  Widget build(BuildContext context) {
    final snapshot = client.getSnapshot();
    final brokerObserved = snapshot.snapshotSource == 'broker';
    final trust = snapshot.trustRecords;
    return ShellPage(
      title: '信頼センター',
      children: [
        if (!brokerObserved)
          const EmptyStatePanel(
            title: '信頼状態はBroker未確認',
            meaning:
                '現在のBrokerから受理したSnapshotではないため、fixture／診断用Trust recordを実際の信頼状態として表示しません。',
            phaseBBlocked: false,
            nextAction: '現在のBroker Snapshotを取得するまで、信頼状態はunknownとして扱います。',
          )
        else if (trust.isEmpty)
          const EmptyStatePanel(
            title: 'Broker SnapshotにTrust recordなし',
            meaning:
                '現在のBroker Snapshotに表示可能なTrust recordがありません。信頼状態の成功や権限を推測しません。',
            phaseBBlocked: false,
            nextAction: 'Brokerが信頼recordを返すまで、Agent／Runtimeをtrustedとして扱いません。',
          )
        else
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
