import 'package:flutter/material.dart';

import 'shared.dart';

class MobileNotifications extends StatelessWidget {
  const MobileNotifications({super.key});

  @override
  Widget build(BuildContext context) {
    return const MobilePage(
      title: '通知',
      children: [
        StatusTile(
          icon: Icons.notifications_active_outlined,
          title: '実行系準備完了',
          subtitle: 'アダプターが参照実行系の準備完了を報告しています',
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
