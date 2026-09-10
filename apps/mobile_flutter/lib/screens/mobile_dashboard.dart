import 'package:flutter/material.dart';

import 'shared.dart';

class MobileDashboard extends StatelessWidget {
  const MobileDashboard({super.key, this.status = '未接続'});
  final String status;

  @override
  Widget build(BuildContext context) {
    return MobilePage(
      title: 'モバイル概要',
      children: [
        StatusTile(icon: Icons.hub_outlined, title: '実行系', subtitle: status),
        StatusTile(
          icon: Icons.fact_check_outlined,
          title: '承認',
          subtitle: '対話ごとにDesktop ownerの承認が必要です。Mobileは承認資格を保持しません',
        ),
        StatusTile(
          icon: Icons.link_outlined,
          title: 'ペアリング',
          subtitle: '接続先画面で端末IDと招待を確認して結合します',
        ),
      ],
    );
  }
}
