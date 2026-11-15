import 'package:flutter/material.dart';

import '../services/shell_core_client.dart';
import 'shared.dart';

class PermissionCenter extends StatelessWidget {
  const PermissionCenter({super.key, required this.client});

  final ShellCoreClient client;

  @override
  Widget build(BuildContext context) {
    final snapshot = client.getSnapshot();
    final permissionsKnown = snapshot.snapshotSource == 'broker' &&
        snapshot.permissionInventoryKnown;
    return ShellPage(
      title: '許可センター',
      children: [
        if (!permissionsKnown)
          const EmptyStatePanel(
            title: 'Permission一覧は未取得',
            meaning:
                '現在のBroker snapshotに実Permission一覧が含まれていません。空配列や診断用fixtureを実状態として扱いません。',
            phaseBBlocked: false,
            nextAction: 'BrokerのPermission一覧Consumerが接続されるまで、発行状態は不明として扱います。',
          )
        else if (snapshot.permissions.isEmpty)
          const EmptyStatePanel(
            title: '表示対象のPermissionなし',
            meaning: 'Brokerが現在のPermission一覧を取得済みとして返し、その一覧に表示対象がありません。',
            phaseBBlocked: false,
            nextAction: '必要な作用は、その操作のBroker認可経路から個別に要求してください。',
          )
        else
          DataTable(
            columns: const [
              DataColumn(label: Text('許可')),
              DataColumn(label: Text('能力')),
              DataColumn(label: Text('判断')),
              DataColumn(label: Text('出所')),
              DataColumn(label: Text('有効期限')),
            ],
            rows: [
              for (final permission in snapshot.permissions)
                DataRow(
                  cells: [
                    DataCell(Text(permission.permissionId)),
                    DataCell(Text(permission.capabilityId)),
                    DataCell(Text(permission.decision)),
                    DataCell(Text(permission.source)),
                    DataCell(Text(permission.expiresAt ?? 'なし')),
                  ],
                ),
            ],
          ),
      ],
    );
  }
}
