import 'package:flutter/material.dart';

import '../services/device_link_controller.dart';
import '../services/mobile_projection_client.dart';
import 'shared.dart';

class MobileNotifications extends StatefulWidget {
  const MobileNotifications({
    super.key,
    this.events = const [],
    this.controller,
    this.connected = false,
  });

  final List<String> events;
  final DeviceLinkController? controller;
  final bool connected;

  @override
  State<MobileNotifications> createState() => _MobileNotificationsState();
}

class _MobileNotificationsState extends State<MobileNotifications> {
  Future<MobileNotificationSummary>? _future;

  @override
  void didUpdateWidget(MobileNotifications oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.connected != widget.connected) _future = null;
  }

  Future<MobileNotificationSummary> _load() =>
      MobileProjectionClient(widget.controller!).notifications();

  void _refresh() {
    _future = widget.connected && widget.controller != null ? _load() : null;
    setState(() {});
  }

  @override
  Widget build(BuildContext context) {
    if (widget.connected && widget.controller != null && _future == null) {
      _future = _load();
    }
    return MobilePage(
      title: '通知',
      children: [
        const Text('Desktop Brokerの監査eventから作られたsummaryだけを表示します。通知は権限を生成しません。'),
        if (widget.connected && widget.controller != null) ...[
          Align(
            alignment: Alignment.centerLeft,
            child: OutlinedButton.icon(
              onPressed: widget.controller!.busy ? null : _refresh,
              icon: const Icon(Icons.refresh),
              label: const Text('通知を更新'),
            ),
          ),
          FutureBuilder<MobileNotificationSummary>(
            future: _future,
            builder: (context, snapshot) {
              if (snapshot.connectionState == ConnectionState.waiting) {
                return const LinearProgressIndicator();
              }
              if (snapshot.hasError) {
                return const StatusTile(
                  icon: Icons.error_outline,
                  title: 'Desktop通知を取得できません',
                  subtitle: 'Broker接続と監査状態を確認してください。',
                );
              }
              final summary = snapshot.data!;
              return Column(
                children: [
                  StatusTile(
                    icon: Icons.notifications_outlined,
                    title: 'Desktop通知',
                    subtitle:
                        '件数: ${summary.count} / 未読: ${summary.unreadCount} / 重大: ${summary.criticalCount}',
                  ),
                  for (final item in summary.items)
                    StatusTile(
                      icon: _icon(item.severity),
                      title: item.title,
                      subtitle: '${item.summary}（${item.state}）',
                    ),
                ],
              );
            },
          ),
        ] else
          const StatusTile(
            icon: Icons.help_outline,
            title: 'Desktop通知は未観測',
            subtitle: '未接続中はMobile起動中の接続状態だけを表示します。',
          ),
        for (final event in widget.events)
          StatusTile(
            icon: Icons.notifications_outlined,
            title: '接続状態',
            subtitle: event,
          ),
        const StatusTile(
          icon: Icons.warning_amber_outlined,
          title: '復旧警告',
          subtitle: '作用を実行する前に許可の承認が必要です。',
        ),
      ],
    );
  }

  IconData _icon(String severity) => switch (severity) {
    'critical' => Icons.error_outline,
    'warning' => Icons.warning_amber_outlined,
    _ => Icons.info_outline,
  };
}
