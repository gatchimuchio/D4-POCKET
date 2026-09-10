import 'package:flutter/material.dart';
import '../services/device_link_client.dart';
import '../services/device_link_controller.dart';
import 'shared.dart';

class DeviceConnection extends StatefulWidget {
  const DeviceConnection({super.key, required this.controller});
  final DeviceLinkController controller;
  @override
  State<DeviceConnection> createState() => _DeviceConnectionState();
}

class _DeviceConnectionState extends State<DeviceConnection> {
  final _input = TextEditingController();
  DeviceCredential? _invite;
  String? _error;
  @override
  void dispose() {
    _input.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final c = widget.controller;
    final credential = c.credential;
    return MobilePage(
      title: '接続先と端末結合',
      children: [
        Text(c.status),
        if (!c.storageReady)
          OutlinedButton(
            onPressed: c.busy ? null : c.initialize,
            child: const Text('安全保管を再確認'),
          ),
        if (c.deviceId != null) SelectableText('この端末ID: ${c.deviceId}'),
        const Text(
          '同じネットワークのDesktopで、この端末IDへの招待を発行してください。招待は安全な対面手段で受け取り、Hostと証明書hashを照合してください。',
        ),
        if (credential != null) ...[
          SelectableText(
            'Host: ${credential.text('接続先Host')}:${credential.port}\nHostID: ${credential.text('HostID')}\n証明書hash: ${credential.text('証明書hash')}\n結合ID: ${credential.id}',
          ),
          Text(
            '期限: ${DateTime.fromMillisecondsSinceEpoch(credential.expiry * 1000).toLocal()}',
          ),
          FilledButton(
            onPressed: c.busy ? null : c.verify,
            child: const Text('資格を再確認'),
          ),
        ] else ...[
          TextField(
            controller: _input,
            obscureText: true,
            enableSuggestions: false,
            enableIMEPersonalizedLearning: false,
            autofillHints: const [],
            autocorrect: false,
            maxLength: 8192,
            enabled: c.storageReady && !c.busy,
            onChanged: (_) {
              if (_invite != null) setState(() => _invite = null);
            },
            decoration: const InputDecoration(
              labelText: 'Desktopからの招待JSON',
              border: OutlineInputBorder(),
            ),
          ),
          OutlinedButton(
            onPressed: !c.storageReady || c.busy
                ? null
                : () {
                    try {
                      final value = c.invitation(_input.text);
                      setState(() {
                        _invite = value;
                        _error = null;
                      });
                    } catch (_) {
                      setState(() {
                        _invite = null;
                        _error = '招待の形式・期限・端末IDを確認してください。';
                      });
                    }
                  },
            child: const Text('招待を確認'),
          ),
          if (_invite != null) ...[
            SelectableText(
              '接続先: ${_invite!.text('接続先Host')}:${_invite!.port}\nHostID: ${_invite!.text('HostID')}\n証明書hash: ${_invite!.text('証明書hash')}',
            ),
            FilledButton(
              onPressed: c.busy
                  ? null
                  : () async {
                      final invite = _invite!;
                      setState(() {
                        _invite = null;
                        _input.clear();
                      });
                      await c.pair(invite);
                    },
              child: const Text('操作者確認: Desktopの表示と一致したので結合'),
            ),
          ],
        ],
        if (_error != null)
          Text(
            _error!,
            style: TextStyle(color: Theme.of(context).colorScheme.error),
          ),
      ],
    );
  }
}

class DeviceSettings extends StatelessWidget {
  const DeviceSettings({super.key, required this.controller});
  final DeviceLinkController controller;
  @override
  Widget build(BuildContext context) => MobilePage(
    title: '設定と保存資格',
    children: [
      const Text(
        '資格はAndroidの安全保管／iOS Keychainへ保存します。招待秘密や対話本文は永続保存しません。Desktop再起動・資格失効・8時間経過後は新しい招待が必要です。',
      ),
      Text(controller.status),
      FilledButton.tonal(
        onPressed: controller.busy || controller.credential == null
            ? null
            : controller.disconnect,
        child: const Text('Desktopの結合を解除して保存資格を削除'),
      ),
      const Text('Desktopへ接続できない場合だけ端末内を削除できます。Desktop側の失効は別途ownerが確認してください。'),
      OutlinedButton(
        onPressed: controller.busy || !controller.storageReady
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
                if (confirmed == true)
                  await controller.disconnect(localOnly: true);
              },
        child: const Text('通信不能時: 端末内の資格だけ削除'),
      ),
    ],
  );
}
