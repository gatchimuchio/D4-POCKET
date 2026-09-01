import 'package:flutter/material.dart';

import '../services/shell_core_client.dart';
import 'shared.dart';

class AuthorityMap extends StatelessWidget {
  const AuthorityMap({super.key, required this.client});

  final ShellCoreClient client;

  @override
  Widget build(BuildContext context) {
    final snapshot = client.getSnapshot();
    return ShellPage(
      title: '権限対応図',
      children: [
        SingleChildScrollView(
          scrollDirection: Axis.horizontal,
          child: DataTable(
            columns: const [
              DataColumn(label: Text('実行系')),
              DataColumn(label: Text('能力')),
              DataColumn(label: Text('許可')),
              DataColumn(label: Text('承認')),
              DataColumn(label: Text('監査')),
              DataColumn(label: Text('復旧')),
              DataColumn(label: Text('危険度')),
            ],
            rows: [
              for (final item in snapshot.authorityMap)
                DataRow(
                  cells: [
                    DataCell(Text(item.runtimeId)),
                    DataCell(Text(item.capabilityId)),
                    DataCell(Text(item.permissionId)),
                    DataCell(Text(item.approvalId)),
                    DataCell(Text(item.auditEventId)),
                    DataCell(Text(item.recoveryId)),
                    DataCell(
                      Text(
                        item.dangerous
                            ? '危険'
                            : (item.warning.isEmpty ? '対応済み' : item.warning),
                      ),
                    ),
                  ],
                ),
            ],
          ),
        ),
        const SectionList(
          title: '表示境界',
          rows: [
            '権限判断はShell Coreに保持します。',
            '非権限源からの試行は、Shell Coreの監査と復旧対応が作用を許可するまで警告として扱います。',
          ],
        ),
        SectionList(
          title: '書き出し前確認',
          rows: [
            for (final item in snapshot.authorityMap)
              '${item.runtimeId} -> ${item.capabilityId} -> ${item.permissionId} -> ${item.approvalId} -> ${item.auditEventId} -> ${item.recoveryId}',
          ],
        ),
      ],
    );
  }
}
