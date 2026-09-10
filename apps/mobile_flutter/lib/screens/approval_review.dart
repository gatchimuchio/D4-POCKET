import 'package:flutter/material.dart';

import 'shared.dart';

class ApprovalReview extends StatelessWidget {
  const ApprovalReview({super.key});

  @override
  Widget build(BuildContext context) {
    return const MobilePage(
      title: '承認確認',
      children: [
        StatusTile(
          icon: Icons.visibility_off_outlined,
          title: '射影',
          subtitle: 'この画面では承認対象を取得していません。対話結果はCoreが許可した表示範囲だけを表示します',
        ),
        StatusTile(
          icon: Icons.lock_outline,
          title: '保護項目',
          subtitle: 'runtime_id、permission_id、payload_hashは固定されています',
        ),
        StatusTile(
          icon: Icons.history_outlined,
          title: '監査',
          subtitle: '承認作用にはShell Coreの監査事象が必要です',
        ),
      ],
    );
  }
}
