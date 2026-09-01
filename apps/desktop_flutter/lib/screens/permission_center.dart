import 'package:flutter/material.dart';

import '../services/shell_core_client.dart';
import 'shared.dart';

class PermissionCenter extends StatelessWidget {
  const PermissionCenter({super.key, required this.client});

  final ShellCoreClient client;

  @override
  Widget build(BuildContext context) {
    final snapshot = client.getSnapshot();
    return ShellPage(
      title: '許可センター',
      children: [
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
