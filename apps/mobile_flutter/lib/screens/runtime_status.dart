import 'package:flutter/material.dart';

import 'shared.dart';

class RuntimeStatus extends StatelessWidget {
  const RuntimeStatus({
    super.key,
    this.runtimes = const [],
    this.connected = false,
  });
  final List<String> runtimes;
  final bool connected;

  @override
  Widget build(BuildContext context) {
    return MobilePage(
      title: '実行系状態',
      children: [
        Text(
          connected
              ? 'Desktopが登録している実行系。稼働・応答成功を保証する一覧ではありません。'
              : '未接続。現在の実行系状態は未確認です。',
        ),
        if (connected)
          for (final runtime in runtimes)
            StatusTile(
              icon: Icons.hub_outlined,
              title: runtime,
              subtitle: '登録を観測',
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
