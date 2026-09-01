import 'package:flutter/material.dart';

import 'shared.dart';

class MobileDashboard extends StatelessWidget {
  const MobileDashboard({super.key});

  @override
  Widget build(BuildContext context) {
    return const MobilePage(
      title: 'モバイル概要',
      children: [
        StatusTile(
          icon: Icons.hub_outlined,
          title: '実行系',
          subtitle: 'BLUE-TANUKIはアダプター契約を通じて準備完了です',
        ),
        StatusTile(
          icon: Icons.fact_check_outlined,
          title: '承認',
          subtitle: '墨消し射影による確認が1件保留中です',
        ),
        StatusTile(
          icon: Icons.link_outlined,
          title: 'ペアリング',
          subtitle: 'device_id、pairing_id、操作者確認が必要です',
        ),
      ],
    );
  }
}
