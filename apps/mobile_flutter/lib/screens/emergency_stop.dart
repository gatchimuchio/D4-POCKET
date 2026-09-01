import 'package:flutter/material.dart';

import 'shared.dart';

class EmergencyStop extends StatelessWidget {
  const EmergencyStop({super.key});

  @override
  Widget build(BuildContext context) {
    return const MobilePage(
      title: '緊急停止',
      children: [
        StatusTile(
          icon: Icons.stop_circle_outlined,
          title: '要求のみ',
          subtitle: 'モバイルはShell Coreを通じて停止を要求します。独立した権限主体ではありません',
        ),
        StatusTile(
          icon: Icons.receipt_long_outlined,
          title: '監査必須',
          subtitle: '停止要求にはAuditEventとRecoveryActionの対応が必要です',
        ),
      ],
    );
  }
}
