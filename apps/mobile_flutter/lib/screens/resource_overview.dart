import 'package:flutter/material.dart';
import 'package:gui_shell_ui/runtime_resource_client.dart';

import '../services/device_link_controller.dart';
import 'shared.dart';

class ResourceOverview extends StatefulWidget {
  const ResourceOverview({
    super.key,
    required this.controller,
    required this.runtimes,
    required this.connected,
  });

  final DeviceLinkController controller;
  final List<String> runtimes;
  final bool connected;

  @override
  State<ResourceOverview> createState() => _ResourceOverviewState();
}

class _ResourceOverviewState extends State<ResourceOverview> {
  Future<List<_ResourceResult>>? _future;

  @override
  void didUpdateWidget(ResourceOverview oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.connected != widget.connected ||
        oldWidget.runtimes.length != widget.runtimes.length ||
        !_same(oldWidget.runtimes, widget.runtimes)) {
      _future = null;
    }
  }

  bool _same(List<String> left, List<String> right) {
    if (left.length != right.length) return false;
    for (var index = 0; index < left.length; index++) {
      if (left[index] != right[index]) return false;
    }
    return true;
  }

  void _refresh() {
    _future = _load();
    setState(() {});
  }

  Future<List<_ResourceResult>> _load() async {
    final client = RuntimeResourceClient(widget.controller);
    final results = <_ResourceResult>[];
    for (final runtime in widget.runtimes.take(16)) {
      try {
        results.add(_ResourceResult(runtime, await client.observe(runtime)));
      } on Object catch (error) {
        results.add(_ResourceResult(runtime, null, error: error));
      }
    }
    return results;
  }

  @override
  Widget build(BuildContext context) {
    if (widget.connected && _future == null && widget.runtimes.isNotEmpty) {
      _future = _load();
    }
    return MobilePage(
      title: '資源概要',
      children: [
        const Text('Brokerがその時点で取得できたread-only実測だけを表示します。取得不能値を0へ変換しません。'),
        if (!widget.connected)
          const StatusTile(
            icon: Icons.help_outline,
            title: '未接続',
            subtitle: '資源値は未観測です。',
          )
        else if (widget.runtimes.isEmpty)
          const StatusTile(
            icon: Icons.help_outline,
            title: '対象なし',
            subtitle: '登録済み実行系がないため、資源値は未観測です。',
          )
        else ...[
          Align(
            alignment: Alignment.centerLeft,
            child: OutlinedButton.icon(
              onPressed: widget.controller.busy ? null : _refresh,
              icon: const Icon(Icons.refresh),
              label: const Text('資源を更新'),
            ),
          ),
          FutureBuilder<List<_ResourceResult>>(
            future: _future,
            builder: (context, snapshot) {
              if (snapshot.connectionState == ConnectionState.waiting) {
                return const LinearProgressIndicator();
              }
              if (snapshot.hasError) {
                return const StatusTile(
                  icon: Icons.error_outline,
                  title: '資源観測失敗',
                  subtitle: 'Brokerの実測応答を確認できません。',
                );
              }
              return Column(
                children: snapshot.data!
                    .map((result) => _resourceTile(result))
                    .toList(growable: false),
              );
            },
          ),
        ],
        const StatusTile(
          icon: Icons.lock_outline,
          title: '統治',
          subtitle: '資源観測はread-onlyです。観測値、履歴、UI状態からPermissionやApprovalを生成しません。',
        ),
      ],
    );
  }

  Widget _resourceTile(_ResourceResult result) {
    final observation = result.observation;
    if (observation == null) {
      return StatusTile(
        icon: Icons.help_outline,
        title: result.runtime,
        subtitle: '未観測。Brokerの応答を確認できません。',
      );
    }
    final cpu = _metricByPrefix(observation, 'CPU利用率');
    final ram = _metricByPrefix(observation, 'RAM');
    return StatusTile(
      icon: observation.binding.isBound ? Icons.memory : Icons.help_outline,
      title: result.runtime,
      subtitle:
          '結合: ${observation.binding.status} / CPU: ${cpu} / RAM: ${ram} / 証拠: ${observation.binding.isBound ? 'LIVE_RUNTIME' : 'INTERNAL_STATE'}',
    );
  }

  String _metric(RuntimeResourceObservation observation, String key) {
    final metric = observation.metric(key);
    if (!metric.isMeasured) return '不明（${metric.reason}）';
    return '${metric.value}（${metric.evidenceSource}）';
  }

  String _metricByPrefix(
    RuntimeResourceObservation observation,
    String prefix,
  ) {
    final key = observation.metrics.keys.firstWhere(
      (candidate) => candidate.startsWith(prefix),
    );
    return _metric(observation, key);
  }
}

class _ResourceResult {
  const _ResourceResult(this.runtime, this.observation, {this.error});

  final String runtime;
  final RuntimeResourceObservation? observation;
  final Object? error;
}
