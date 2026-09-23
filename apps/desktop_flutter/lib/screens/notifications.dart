import 'package:flutter/material.dart';

import '../services/notification_client.dart';
import '../services/shell_core_client.dart';
import 'shared.dart';

typedef NotificationNavigation = void Function(String target);

class NotificationsScreen extends StatefulWidget {
  const NotificationsScreen({
    super.key,
    required this.client,
    this.onNavigate,
  });

  final ShellCoreClient client;
  final NotificationNavigation? onNavigate;

  @override
  State<NotificationsScreen> createState() => _NotificationsScreenState();
}

class _NotificationsScreenState extends State<NotificationsScreen> {
  NotificationClient? _notificationClient;
  Future<Map<String, Object?>>? _notificationsFuture;
  bool _unreadOnly = false;
  String? _message;

  @override
  void initState() {
    super.initState();
    _notificationClient = widget.client.brokerTransport == null
        ? null
        : NotificationClient(widget.client.brokerTransport!);
    _refresh();
  }

  void _refresh() {
    final client = _notificationClient;
    if (client == null) {
      _notificationsFuture = null;
      return;
    }
    _notificationsFuture = client.list(unreadOnly: _unreadOnly);
  }

  Future<void> _run(Future<Map<String, Object?>> operation) async {
    try {
      await operation;
      if (!mounted) return;
      setState(() {
        _message = '通知の表示状態を更新しました。';
        _refresh();
      });
    } on Object catch (error) {
      if (!mounted) return;
      setState(() => _message = error.toString());
    }
  }

  @override
  Widget build(BuildContext context) {
    final client = _notificationClient;
    return ShellPage(
      title: '通知センター',
      children: [
        const BorderedPanel(
          child: Text(
            '監査eventからsummaryだけを表示します。通知は権限を生成せず、開く操作は画面遷移だけを行います。',
          ),
        ),
        if (client == null)
          const BorderedPanel(
            child:
                Text('Rust Broker接続がないため、通知を取得できません。ローカルsnapshotを通知の根拠にはしません。'),
          )
        else
          FutureBuilder<Map<String, Object?>>(
            future: _notificationsFuture,
            builder: (context, snapshot) {
              if (snapshot.connectionState == ConnectionState.waiting) {
                return const BorderedPanel(child: LinearProgressIndicator());
              }
              if (snapshot.hasError) {
                return BorderedPanel(
                  child: Text('通知一覧を取得できません: ${snapshot.error}'),
                );
              }
              final body = snapshot.data ?? const <String, Object?>{};
              return _notificationPanel(client, body);
            },
          ),
        if (_message != null) BorderedPanel(child: Text(_message!)),
      ],
    );
  }

  Widget _notificationPanel(
    NotificationClient client,
    Map<String, Object?> body,
  ) {
    final rawItems = body['通知一覧'];
    final items = rawItems is List
        ? rawItems.whereType<Map>().map(Map<String, Object?>.from).toList()
        : <Map<String, Object?>>[];
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        BorderedPanel(
          child: Wrap(
            spacing: 12,
            runSpacing: 8,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              Text('件数: ${body['件数'] ?? 0}'),
              Text('未読: ${body['未読件数'] ?? 0}'),
              Text('重大: ${body['重大件数'] ?? 0}'),
              FilterChip(
                label: const Text('未読のみ'),
                selected: _unreadOnly,
                onSelected: (value) => setState(() {
                  _unreadOnly = value;
                  _refresh();
                }),
              ),
              FilledButton.tonal(
                onPressed: () => _run(client.markAllRead()),
                child: const Text('全て既読'),
              ),
              OutlinedButton(
                onPressed: () => setState(_refresh),
                child: const Text('更新'),
              ),
            ],
          ),
        ),
        if (items.isEmpty)
          const EmptyStatePanel(
            title: '通知なし',
            meaning: '現在のBroker監査射影に表示対象の通知はありません。',
            phaseBBlocked: false,
            nextAction: '実行系や承認の状態を確認し、必要なら監査ビューアーを開いてください。',
          )
        else
          for (final item in items) _notificationCard(client, item),
      ],
    );
  }

  Widget _notificationCard(
    NotificationClient client,
    Map<String, Object?> item,
  ) {
    final severity = item['severity']?.toString() ?? 'unknown';
    final target = item['遷移先']?.toString() ?? 'dashboard';
    final id = item['通知ID']?.toString() ?? '';
    final notificationHash = item['通知hash']?.toString() ?? '';
    final unread = item['状態'] == 'unread';
    return Padding(
      padding: const EdgeInsets.only(top: 12),
      child: BorderedPanel(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(_severityIcon(severity)),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    item['タイトル']?.toString() ?? '通知',
                    style: const TextStyle(fontWeight: FontWeight.w600),
                  ),
                ),
                Chip(label: Text(severity)),
              ],
            ),
            const SizedBox(height: 6),
            Text(item['概要']?.toString() ?? ''),
            const SizedBox(height: 6),
            Text(
              'source: ${item['source'] ?? 'unknown'} / 状態: ${unread ? '未読' : '既読'} / 監査: ${item['関連監査ID'] ?? 'unknown'}',
              style: Theme.of(context).textTheme.bodySmall,
            ),
            const SizedBox(height: 8),
            Wrap(
              spacing: 8,
              children: [
                OutlinedButton(
                  onPressed: widget.onNavigate == null
                      ? null
                      : () => widget.onNavigate!(target),
                  child: const Text('開く'),
                ),
                if (unread)
                  TextButton(
                    onPressed: () => _run(
                      client.markRead(
                        notificationId: id,
                        notificationHash: notificationHash,
                      ),
                    ),
                    child: const Text('既読'),
                  ),
                TextButton(
                  onPressed: () => _run(
                    client.dismiss(
                      notificationId: id,
                      notificationHash: notificationHash,
                    ),
                  ),
                  child: const Text('破棄'),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }

  IconData _severityIcon(String severity) => switch (severity) {
        'critical' => Icons.gpp_bad_outlined,
        'error' => Icons.error_outline,
        'warning' => Icons.warning_amber_outlined,
        _ => Icons.info_outline,
      };
}
