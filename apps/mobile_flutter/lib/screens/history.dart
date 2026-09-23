import 'package:flutter/material.dart';
import 'package:gui_shell_ui/history_client.dart';

import '../services/device_link_controller.dart';
import 'shared.dart';

class MobileHistory extends StatefulWidget {
  const MobileHistory({
    super.key,
    required this.controller,
    required this.connected,
  });

  final DeviceLinkController controller;
  final bool connected;

  @override
  State<MobileHistory> createState() => _MobileHistoryState();
}

class _MobileHistoryState extends State<MobileHistory> {
  Future<HistoryPage?>? _future;

  @override
  void didUpdateWidget(MobileHistory oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.connected != widget.connected) _future = null;
  }

  Future<HistoryPage?> _load() async {
    final client = HistoryClient(widget.controller);
    final grant = await client.status();
    if (grant == null) return null;
    return client.page(grant);
  }

  void _refresh() {
    _future = widget.connected ? _load() : null;
    setState(() {});
  }

  @override
  Widget build(BuildContext context) {
    if (widget.connected && _future == null) _future = _load();
    return MobilePage(
      title: '履歴',
      children: [
        const Text(
          '履歴は現在のowner承認がある場合だけmetadataを表示します。Mobileは承認を発行・延長・失効せず、対話本文も取得しません。',
        ),
        if (!widget.connected)
          const StatusTile(
            icon: Icons.help_outline,
            title: '未接続',
            subtitle: '履歴は未観測です。',
          )
        else ...[
          Align(
            alignment: Alignment.centerLeft,
            child: OutlinedButton.icon(
              onPressed: widget.controller.busy ? null : _refresh,
              icon: const Icon(Icons.refresh),
              label: const Text('履歴を更新'),
            ),
          ),
          FutureBuilder<HistoryPage?>(
            future: _future,
            builder: (context, snapshot) {
              if (snapshot.connectionState == ConnectionState.waiting) {
                return const LinearProgressIndicator();
              }
              if (snapshot.hasError) {
                return const StatusTile(
                  icon: Icons.error_outline,
                  title: '履歴を取得できません',
                  subtitle: '現在承認、監査状態、またはDesktop接続を確認してください。',
                );
              }
              final page = snapshot.data;
              if (page == null || page.entries.isEmpty) {
                return const StatusTile(
                  icon: Icons.history_outlined,
                  title: '現在承認なし',
                  subtitle:
                      'Desktop ownerが履歴閲覧を承認していないため、本文・履歴metadataを表示しません。',
                );
              }
              return Column(
                children: page.entries
                    .map(
                      (entry) => StatusTile(
                        icon: _icon(entry.state),
                        title: entry.state,
                        subtitle:
                            '監査ID: ${entry.auditId}${entry.failure == null ? '' : ' / 失敗: ${entry.failure}'}',
                      ),
                    )
                    .toList(growable: false),
              );
            },
          ),
        ],
      ],
    );
  }

  IconData _icon(String state) => switch (state) {
    '成功' => Icons.check_circle_outline,
    '失敗' => Icons.error_outline,
    '中止' => Icons.cancel_outlined,
    _ => Icons.pending_outlined,
  };
}
