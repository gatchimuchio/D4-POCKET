import 'package:flutter/material.dart';

import '../services/shell_core_client.dart';
import 'shared.dart';

class ApprovalCenter extends StatelessWidget {
  const ApprovalCenter({super.key, required this.client});

  final ShellCoreClient client;

  @override
  Widget build(BuildContext context) {
    final snapshot = client.getSnapshot();
    final approvalsKnown = snapshot.snapshotSource == 'broker' &&
        snapshot.pendingApprovalsInventoryKnown;
    final approvals = snapshot.pendingApprovals;
    return ShellPage(
      title: '承認センター',
      children: [
        if (!approvalsKnown)
          const EmptyStatePanel(
            title: '承認待ち状態は未取得',
            meaning: '現在のBroker snapshotに承認一覧が含まれていません。空配列は「承認なし」を意味しません。',
            phaseBBlocked: false,
            nextAction: 'Brokerの承認一覧Consumerが接続されるまで、承認状態は不明として扱います。',
          )
        else if (approvals.isEmpty)
          const EmptyStatePanel(
            title: '保留中の承認なし',
            meaning: 'Shell Coreには、所有者の確認を待つ承認射影がありません。',
            phaseBBlocked: false,
            nextAction: '所有者利用を継続できます。Shell Coreが新しい承認を発行すると、ここに表示されます。',
          )
        else
          for (final approval in approvals)
            BorderedPanel(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    '${approval.operation}  ${approval.status}',
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                  const SizedBox(height: 8),
                  Text('可視性: ${approval.contentVisibility}'),
                  const SizedBox(height: 8),
                  Wrap(
                    spacing: 8,
                    runSpacing: 8,
                    children: [
                      for (final entry in approval.projectedContent.entries)
                        Chip(label: Text('${entry.key}: ${entry.value}')),
                    ],
                  ),
                  const SizedBox(height: 8),
                  SectionList(title: '編集可能な項目', rows: approval.editableFields),
                  SectionList(title: '保護された項目', rows: approval.protectedFields),
                ],
              ),
            ),
      ],
    );
  }
}
