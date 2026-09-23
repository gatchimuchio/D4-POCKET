import 'package:flutter/material.dart';

import '../services/device_link_controller.dart';
import '../services/mobile_projection_client.dart';
import 'shared.dart';

class EmergencyStop extends StatefulWidget {
  const EmergencyStop({super.key, required this.controller});
  final DeviceLinkController controller;

  @override
  State<EmergencyStop> createState() => _EmergencyStopState();
}

class _EmergencyStopState extends State<EmergencyStop> {
  MobileStopRequestProjection? _projection;
  String? _error;

  Future<void> _refreshStopRequest() async {
    setState(() => _error = null);
    try {
      final projection = await MobileProjectionClient(
        widget.controller,
      ).stopRequest();
      if (!mounted) return;
      setState(() => _projection = projection);
    } on Object {
      if (!mounted) return;
      setState(() => _error = '停止要求の現在状態を確認できません。実停止は行っていません。');
    }
  }

  @override
  Widget build(BuildContext context) {
    final controller = widget.controller;
    return MobilePage(
      title: '緊急停止要求',
      children: [
        FilledButton(
          onPressed: controller.busy || !controller.ready
              ? null
              : _refreshStopRequest,
          child: const Text('停止要求の状態を確認'),
        ),
        Text(controller.status),
        if (_error != null)
          StatusTile(
            icon: Icons.error_outline,
            title: '未観測',
            subtitle: _error!,
          ),
        if (_projection != null) ...[
          StatusTile(
            icon: Icons.stop_circle_outlined,
            title: 'owner再承認待ち',
            subtitle:
                '対象: ${_projection!.targets.length} / 停止実行済み: ${_projection!.stopExecuted} / 監査: ${_projection!.auditId}',
          ),
          for (final target in _projection!.targets)
            StatusTile(
              icon: Icons.pause_circle_outline,
              title: target.runtimeId,
              subtitle:
                  '状態: ${target.state} / ${target.reapprovalState} / 実停止は未実行',
            ),
        ],
        const StatusTile(
          icon: Icons.stop_circle_outlined,
          title: 'この端末の対話を中止して結合解除',
          subtitle:
              '結合解除は未送信要求と遅延結果の採用を止めます。送信済み実行系の計算停止・全体停止はDesktop側でownerが確認してください。',
        ),
        const StatusTile(
          icon: Icons.receipt_long_outlined,
          title: '監査必須',
          subtitle:
              '停止要求にはAuditEventとRecoveryActionの対応が必要です。Mobileは実停止を実行しません。',
        ),
      ],
    );
  }
}
