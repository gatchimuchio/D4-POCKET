import 'package:flutter/material.dart';

import 'shared.dart';
import '../services/device_link_controller.dart';

class EmergencyStop extends StatelessWidget {
  const EmergencyStop({super.key, required this.controller});
  final DeviceLinkController controller;

  @override
  Widget build(BuildContext context) {
    return MobilePage(
      title: '緊急停止',
      children: [
        FilledButton(
          onPressed: controller.busy || !controller.ready
              ? null
              : controller.disconnect,
          child: const Text('この端末の対話を中止して結合解除'),
        ),
        Text(controller.status),
        StatusTile(
          icon: Icons.stop_circle_outlined,
          title: 'この端末の対話の採用を停止',
          subtitle:
              '結合解除は未送信要求と遅延結果の採用を止めます。送信済み実行系の計算停止・全体停止はDesktop側で確認してください',
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
