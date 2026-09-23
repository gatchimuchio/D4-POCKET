import 'package:flutter/material.dart';

import 'shared.dart';

/// MCPはowner専用のDesktop管理面。Mobileでmetadataを権限へ昇格させない。
class MobileMcpStatus extends StatelessWidget {
  const MobileMcpStatus({super.key});

  @override
  Widget build(BuildContext context) => const MobilePage(
    title: 'MCP状態',
    children: [
      StatusTile(
        icon: Icons.hub_outlined,
        title: '未観測',
        subtitle:
            'MCP接続一覧はDesktop ownerの管理面に限定しています。Mobileは接続、Tool実行、Credential参照を行いません。',
      ),
      StatusTile(
        icon: Icons.lock_outline,
        title: '権限境界',
        subtitle:
            'MCP metadata、Trust、Capability差分、Credential refはAuthorityを生成しません。',
      ),
    ],
  );
}
