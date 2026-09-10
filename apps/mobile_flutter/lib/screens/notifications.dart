import 'package:flutter/material.dart';

import 'shared.dart';

class MobileNotifications extends StatelessWidget {
  const MobileNotifications({super.key, this.events = const []});
  final List<String> events;

  @override
  Widget build(BuildContext context) {
    return MobilePage(
      title: '通知',
      children: [
        const Text('この起動中に観測した接続通知です。バックグラウンド通知やDesktop監査履歴ではありません。'),
        for (final event in events)
          StatusTile(
            icon: Icons.notifications_outlined,
            title: '接続状態',
            subtitle: event,
          ),
        StatusTile(
          icon: Icons.warning_amber_outlined,
          title: '復旧警告',
          subtitle: '作用を実行する前に許可の承認が必要です',
        ),
      ],
    );
  }
}
