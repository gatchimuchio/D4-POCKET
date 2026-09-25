import 'package:flutter/material.dart';

import '../services/device_link_client.dart';
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

class DeviceSettings extends StatefulWidget {
  const DeviceSettings({super.key, required this.controller});
  final DeviceLinkController controller;

  @override
  State<DeviceSettings> createState() => _DeviceSettingsState();
}

class _DeviceSettingsState extends State<DeviceSettings> {
  bool _readingAudit = false;
  bool _auditLoaded = false;
  String? _auditError;
  List<DeviceLinkLocalRecoveryAuditEvent> _recoveryEvents = const [];

  Future<void> _readRecoveryAudit() async {
    if (_readingAudit) return;
    setState(() {
      _readingAudit = true;
      _auditError = null;
    });
    try {
      final events = await widget.controller.readLocalRecoveryAudit();
      if (!mounted) return;
      setState(() {
        _recoveryEvents = events;
        _auditLoaded = true;
      });
    } catch (_) {
      if (!mounted) return;
      setState(() {
        _auditError = '端末内回復記録を確認できません。native保管状態を再確認してください。';
      });
    } finally {
      if (mounted) setState(() => _readingAudit = false);
    }
  }

  @override
  Widget build(BuildContext context) => MobilePage(
    title: '設定と端末資格',
    children: [
      const Text(
        '端末資格はAndroid KeystoreまたはiOS ThisDeviceOnly Keychainで保護します。招待秘密や対話本文をFlutterや一般設定へ保存しません。Desktop再起動・資格失効・8時間経過後は新しい招待が必要です。',
      ),
      Text(widget.controller.status),
      FilledButton.tonal(
        onPressed:
            widget.controller.busy || !widget.controller.hasStoredCredential
            ? null
            : widget.controller.disconnect,
        child: const Text('Desktopの結合を解除して端末資格を削除'),
      ),
      const Text('Desktopへ接続できない場合だけ端末内を削除できます。Desktop側の失効は別途ownerが確認してください。'),
      OutlinedButton(
        onPressed:
            widget.controller.busy ||
                !widget.controller.storageReady ||
                !widget.controller.hasStoredCredential
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
                  await widget.controller.disconnect(localOnly: true);
                  await _readRecoveryAudit();
                }
              },
        child: const Text('通信不能時: 端末内の資格だけ削除'),
      ),
      const Text(
        '端末内回復記録は端末内の表示用記録です。Desktopの監査連鎖とは別であり、Desktop失効や操作者本人を証明しません。',
      ),
      OutlinedButton(
        onPressed:
            _readingAudit ||
                widget.controller.busy ||
                !widget.controller.storageReady ||
                !widget.controller.foreground
            ? null
            : _readRecoveryAudit,
        child: Text(_readingAudit ? '端末内回復記録を確認中' : '端末内回復記録を確認'),
      ),
      if (_auditError != null) Text(_auditError!),
      if (_auditLoaded && _recoveryEvents.isEmpty)
        const Text('保存された端末内回復記録はありません。'),
      if (_auditLoaded)
        ..._recoveryEvents.map(
          (event) => ListTile(
            key: ValueKey(event.eventId),
            title: const Text('端末内資格を削除'),
            subtitle: Text('${event.timestamp}・Desktop側失効未確認'),
          ),
        ),
    ],
  );
}
