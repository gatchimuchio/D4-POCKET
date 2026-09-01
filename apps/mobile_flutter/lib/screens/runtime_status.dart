import 'package:flutter/material.dart';

import 'shared.dart';

class RuntimeStatus extends StatelessWidget {
  const RuntimeStatus({super.key});

  @override
  Widget build(BuildContext context) {
    return const MobilePage(
      title: '実行系状態',
      children: [
        StatusTile(
          icon: Icons.check_circle_outline,
          title: 'blue_tanuki',
          subtitle: '準備完了',
        ),
        StatusTile(
          icon: Icons.security_outlined,
          title: '権限',
          subtitle: '権限源は引き続きShell Coreです',
        ),
      ],
    );
  }
}
