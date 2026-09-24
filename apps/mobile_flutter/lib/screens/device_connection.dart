import 'package:flutter/material.dart';

import '../services/device_link_controller.dart';
import 'shared.dart';

class DeviceConnection extends StatelessWidget {
  const DeviceConnection({super.key, required this.controller});
  final DeviceLinkController controller;

  @override
  Widget build(BuildContext context) => MobilePage(
    title: 'Desktop端末結合',
    children: [
      Text(controller.status),
      if (!controller.storageReady)
        OutlinedButton(
          onPressed: controller.busy ? null : controller.initialize,
          child: const Text('native安全保管を再確認'),
        ),
      const Text(
        '結合時はnative画面に端末IDが表示されます。Desktop ownerへ端末IDを伝え、発行された招待を安全な対面手段で受け取ってください。招待JSONと資格はFlutter画面へ渡りません。',
      ),
      if (controller.hasStoredCredential) ...[
        const Text('端末資格はOS安全保管に保持され、画面やFlutter stateには公開されません。'),
        FilledButton(
          onPressed: controller.busy ? null : controller.verify,
          child: const Text('Desktop資格を再確認'),
        ),
      ] else
        FilledButton(
          onPressed:
              !controller.storageReady ||
                  controller.busy ||
                  !controller.foreground
              ? null
              : controller.pair,
          child: const Text('native画面で招待を入力して端末結合'),
        ),
    ],
  );
}

class DeviceSettings extends StatelessWidget {
  const DeviceSettings({super.key, required this.controller});
  final DeviceLinkController controller;

  @override
  Widget build(BuildContext context) => MobilePage(
    title: '設定と端末資格',
    children: [
      const Text(
        '端末資格はAndroid KeystoreまたはiOS ThisDeviceOnly Keychainで保護します。招待秘密や対話本文をFlutterや一般設定へ保存しません。Desktop再起動・資格失効・8時間経過後は新しい招待が必要です。',
      ),
      Text(controller.status),
      FilledButton.tonal(
        onPressed: controller.busy || !controller.hasStoredCredential
            ? null
            : controller.disconnect,
        child: const Text('Desktopの結合を解除して端末資格を削除'),
      ),
      const Text('Desktopへ接続できない場合だけ端末内を削除できます。Desktop側の失効は別途ownerが確認してください。'),
      OutlinedButton(
        onPressed:
            controller.busy ||
                !controller.storageReady ||
                !controller.hasStoredCredential
            ? null
            : () async {
                final confirmed = await showDialog<bool>(
                  context: context,
                  builder: (context) => AlertDialog(
                    title: const Text('端末内だけ削除しますか？'),
                    content: const Text(
                      'Desktop側の資格は失効しません。ownerによる失効確認が必要です。',
                    ),
                    actions: [
                      TextButton(
                        onPressed: () => Navigator.pop(context, false),
                        child: const Text('戻る'),
                      ),
                      FilledButton(
                        onPressed: () => Navigator.pop(context, true),
                        child: const Text('端末内だけ削除'),
                      ),
                    ],
                  ),
                );
                if (confirmed == true) {
                  await controller.disconnect(localOnly: true);
                }
              },
        child: const Text('通信不能時: 端末内の資格だけ削除'),
      ),
    ],
  );
}
