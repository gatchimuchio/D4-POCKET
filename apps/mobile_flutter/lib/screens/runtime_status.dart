import 'package:flutter/material.dart';
import 'package:gui_shell_ui/runtime_lifecycle_client.dart';

import '../services/device_link_controller.dart';
import 'shared.dart';

class RuntimeStatus extends StatefulWidget {
  const RuntimeStatus({
    super.key,
    this.controller,
    this.runtimes = const [],
    this.connected = false,
  });

  final DeviceLinkController? controller;
  final List<String> runtimes;
  final bool connected;

  @override
  State<RuntimeStatus> createState() => _RuntimeStatusState();
}

class _RuntimeStatusState extends State<RuntimeStatus> {
  final _futures = <String, Future<RuntimeLifecycleStatus>>{};

  @override
  void didUpdateWidget(RuntimeStatus oldWidget) {
    super.didUpdateWidget(oldWidget);
    _syncFutures();
  }

  void _syncFutures() {
    final controller = widget.controller;
    if (!widget.connected || controller == null) {
      _futures.clear();
      return;
    }
    final ids = widget.runtimes.toSet();
    _futures.removeWhere((id, _) => !ids.contains(id));
    for (final runtime in ids) {
      _futures.putIfAbsent(
        runtime,
        () => RuntimeLifecycleClient(controller).status(runtime),
      );
    }
  }

  void _refresh() {
    _futures.clear();
    _syncFutures();
    setState(() {});
  }

  @override
  Widget build(BuildContext context) {
    _syncFutures();
    return MobilePage(
      title: '実行系状態',
      children: [
        Text(
          widget.connected
              ? 'Desktop Brokerが返した現在状態です。表示成功は実行権限や処理成功を意味しません。'
              : '未接続。現在の実行系状態は未確認です。',
        ),
        if (widget.connected)
          Align(
            alignment: Alignment.centerLeft,
            child: OutlinedButton.icon(
              onPressed: _refresh,
              icon: const Icon(Icons.refresh),
              label: const Text('状態を更新'),
            ),
          ),
        if (widget.connected && widget.runtimes.isEmpty)
          const StatusTile(
            icon: Icons.help_outline,
            title: '登録実行系なし',
            subtitle: 'Desktop Brokerから登録済み実行系を観測できません。',
          ),
        if (widget.connected)
          for (final runtime in widget.runtimes) _runtimeTile(runtime),
        const StatusTile(
          icon: Icons.security_outlined,
          title: '権限',
          subtitle: '権限源は引き続きShell Coreです。Mobileの表示状態から権限は生成されません。',
        ),
      ],
    );
  }

  Widget _runtimeTile(String runtime) {
    final future = _futures[runtime];
    if (future == null) {
      return StatusTile(
        icon: Icons.help_outline,
        title: runtime,
        subtitle: '未観測',
      );
    }
    return FutureBuilder<RuntimeLifecycleStatus>(
      future: future,
      builder: (context, snapshot) {
        if (snapshot.connectionState == ConnectionState.waiting) {
          return StatusTile(
            icon: Icons.sync,
            title: runtime,
            subtitle: '状態を観測中',
          );
        }
        if (snapshot.hasError) {
          return StatusTile(
            icon: Icons.error_outline,
            title: runtime,
            subtitle: '未観測。Broker応答を確認できません。',
          );
        }
        final status = snapshot.data!;
        return StatusTile(
          icon: status.supported ? Icons.hub_outlined : Icons.help_outline,
          title: runtime,
          subtitle: '${status.state}（${status.evidenceSource}）',
        );
      },
    );
  }
}
