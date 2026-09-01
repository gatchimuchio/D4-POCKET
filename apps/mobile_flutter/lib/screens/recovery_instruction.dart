import 'package:flutter/material.dart';

import 'shared.dart';

class RecoveryInstruction extends StatelessWidget {
  const RecoveryInstruction({super.key});

  @override
  Widget build(BuildContext context) {
    return const MobilePage(
      title: '復旧手順',
      children: [
        StatusTile(
          icon: Icons.health_and_safety_outlined,
          title: '許可の復旧',
          subtitle: '保留中の承認を確認し、Shell Coreの承認後に再試行してください',
        ),
        StatusTile(
          icon: Icons.link_off_outlined,
          title: '取消し経路',
          subtitle: 'ペアリングにはdevice_id、pairing_id、監査事象、取消し、復旧経路が含まれます',
        ),
      ],
    );
  }
}
