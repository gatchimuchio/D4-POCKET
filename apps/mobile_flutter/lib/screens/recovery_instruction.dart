import 'package:flutter/material.dart';

import 'shared.dart';

// 旧画面のdevice_idは端末ID、pairing_idは結合IDに対応する。
// 正本device-linkの資格・失効を使い、旧preview値を復旧の根拠にしない。
class RecoveryInstruction extends StatelessWidget {
  const RecoveryInstruction({super.key});

  @override
  Widget build(BuildContext context) {
    return const MobilePage(
      title: '復旧経路と手順',
      children: [
        StatusTile(
          icon: Icons.health_and_safety_outlined,
          title: '許可の復旧',
          subtitle: '接続先画面で資格を再確認してください。保留中の対話は同じ要求を照会し、自動再送しません',
        ),
        StatusTile(
          icon: Icons.link_off_outlined,
          title: '取消し経路',
          subtitle:
              '期限切れ・Desktop再起動時はownerが旧端末資格を失効させ、設定で保存資格を削除して新しい招待を受け取ってください',
        ),
      ],
    );
  }
}
