import 'package:flutter/material.dart';

import '../models/generated_contracts.dart';
import '../services/runtime_resource_client.dart';
import '../services/shell_core_client.dart';
import 'shared.dart';

const _resourceNetworkIoKey = 'NetworkIOBytes';
const _resourceRamWorkingSetKey = 'RAMWorkingSetBytes';

class RuntimeCenter extends StatefulWidget {
  const RuntimeCenter({
    super.key,
    required this.client,
    this.resourceClient,
    this.connectResource = connectRuntimeResourceClient,
  });

  final ShellCoreClient client;
  final RuntimeResourceClient? resourceClient;
  final RuntimeResourceClientConnector connectResource;

  @override
  State<RuntimeCenter> createState() => _RuntimeCenterState();
}

class _RuntimeCenterState extends State<RuntimeCenter>
    with SingleTickerProviderStateMixin {
  String? _selectedRuntimeId;
  RuntimeResourceClient? _resourceClient;
  RuntimeResourceObservation? _resourceObservation;
  final _resourceRuntimeId = TextEditingController();
  late final TabController _tabs;
  bool _resourceBusy = false;
  int _resourceGeneration = 0;
  String _resourceMessage =
      '資源は自動更新しません。実行系IDを指定して、監査付きの一回観測を実行してください。';

  @override
  void initState() {
    super.initState();
    _resourceClient = widget.resourceClient;
    _tabs = TabController(length: 2, vsync: this);
  }

  @override
  void didUpdateWidget(covariant RuntimeCenter oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.resourceClient != widget.resourceClient ||
        oldWidget.connectResource != widget.connectResource) {
      _resourceGeneration++;
      _resourceClient = widget.resourceClient;
      _resourceObservation = null;
      _resourceBusy = false;
      _resourceMessage = '接続条件が変わりました。資源を更新して再観測してください。';
    }
  }

  @override
  void dispose() {
    _resourceGeneration++;
    _resourceRuntimeId.dispose();
    _tabs.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final snapshot = widget.client.getSnapshot();
    final selectedRuntime = _selectedRuntime(snapshot);
    return ShellPage(
      title: '実行系センター',
      children: [
        Material(
          color: Colors.transparent,
          child: TabBar(
            controller: _tabs,
            tabs: const [
              Tab(text: '状態'),
              Tab(text: '資源'),
            ],
          ),
        ),
        AnimatedBuilder(
          animation: _tabs,
          builder: (context, _) => _tabs.index == 0
              ? _overview(snapshot, selectedRuntime)
              : _resourcePanel(selectedRuntime),
        ),
      ],
    );
  }

  Widget _overview(ShellSnapshot snapshot, RuntimeRecord? selectedRuntime) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if (snapshot.runtimes.isEmpty)
          const EmptyStatePanel(
            title: '接続中の実行系なし',
            meaning: '現在のスナップショットに実行系の記録がありません。',
            phaseBBlocked: false,
            nextAction: '実行系の検出完了後に、ブローカーの製品経路へ再接続するか診断を更新してください。',
          )
        else
          LayoutBuilder(
            builder: (context, constraints) {
              final table = _RuntimeTable(
                snapshot: snapshot,
                selectedRuntimeId: selectedRuntime?.runtimeId ??
                    snapshot.runtimes.first.runtimeId,
                onSelected: (runtimeId) =>
                    setState(() => _selectedRuntimeId = runtimeId),
              );
              final detail = _RuntimeDetailPanel(
                snapshot: snapshot,
                runtime: selectedRuntime ?? snapshot.runtimes.first,
              );
              if (constraints.maxWidth < 980) {
                return Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [table, const SizedBox(height: 12), detail],
                );
              }
              return Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Expanded(flex: 3, child: table),
                  const SizedBox(width: 12),
                  Expanded(flex: 2, child: detail),
                ],
              );
            },
          ),
        const SizedBox(height: 16),
        for (final adapter in snapshot.adapterCatalog)
          Padding(
            padding: const EdgeInsets.only(bottom: 16),
            child: SectionList(
              title: 'アダプター台帳: ${adapter.adapterId}',
              rows: [
                '実行系: ${adapter.runtimeId}',
                '発行者: ${adapter.publisher}',
                '版: ${adapter.version}',
                '署名: ${adapter.signature}',
                'ハッシュ: ${adapter.hash}',
                '信頼: ${adapter.trustStatus}',
                '要求能力: ${adapter.requestedCapabilities.join(', ')}',
                '付与能力: ${adapter.grantedCapabilities.join(', ')}',
                '拒否能力: ${adapter.deniedCapabilities.join(', ')}',
                '既知の危険: ${adapter.knownRisks.join(', ')}',
              ],
            ),
          ),
        for (final diff in snapshot.permissionDiffs)
          Padding(
            padding: const EdgeInsets.only(bottom: 16),
            child: SectionList(
              title: '許可差分: ${diff.subject}',
              rows: [
                for (final item in diff.added) '+ $item',
                for (final item in diff.removed) '- $item',
                for (final item in diff.changed) '~ $item',
                for (final item in diff.dangerous) '! $item',
              ],
            ),
          ),
        SectionList(
          title: '実行系の権限経路',
          rows: [
            for (final item in snapshot.authorityMap)
              '${item.runtimeId} -> ${item.capabilityId} -> ${item.permissionId} -> ${item.approvalId} -> ${item.auditEventId} -> ${item.recoveryId}',
          ],
        ),
        const SizedBox(height: 16),
        const SectionList(
          title: '権限境界',
          rows: [
            '実行系の能力、許可、承認、監査、復旧の判断はShell Coreが保持します。',
            'Flutterはブローカー経由の権限状態または診断専用のローカルデータを表示するだけで、権限の付与、承認、変更を行いません。',
          ],
        ),
      ],
    );
  }

  Widget _resourcePanel(RuntimeRecord? selectedRuntime) {
    final observation = _resourceObservation;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        BorderedPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text('実行系資源', style: Theme.of(context).textTheme.titleMedium),
              const SizedBox(height: 8),
              const Text(
                'ここに表示する資源値は、下の「資源を更新」でBrokerが監査付きで取得した一回の観測だけです。状態タブのスナップショットやUI入力は資源証拠として使いません。',
              ),
              const SizedBox(height: 12),
              Wrap(
                spacing: 12,
                runSpacing: 8,
                crossAxisAlignment: WrapCrossAlignment.center,
                children: [
                  SizedBox(
                    width: 300,
                    child: TextField(
                      key: const ValueKey('runtime-resource-runtime-id'),
                      controller: _resourceRuntimeId,
                      enabled: !_resourceBusy,
                      autocorrect: false,
                      enableSuggestions: false,
                      decoration: const InputDecoration(
                        labelText: '実行系ID',
                        helperText: 'PIDや接続先は入力できません。',
                      ),
                    ),
                  ),
                  FilledButton.icon(
                    key: const ValueKey('runtime-resource-refresh'),
                    onPressed: _resourceBusy ? null : _refreshResource,
                    icon: const Icon(Icons.refresh),
                    label: const Text('資源を更新'),
                  ),
                  if (selectedRuntime != null)
                    TextButton(
                      onPressed: _resourceBusy
                          ? null
                          : () => setState(
                                () => _resourceRuntimeId.text =
                                    selectedRuntime.runtimeId,
                              ),
                      child: const Text('状態タブの選択IDを入力'),
                    ),
                ],
              ),
              const SizedBox(height: 8),
              Text(_resourceMessage),
              if (_resourceBusy) ...[
                const SizedBox(height: 8),
                const LinearProgressIndicator(),
              ],
            ],
          ),
        ),
        if (observation != null) ...[
          const SizedBox(height: 16),
          _ResourceObservationPanel(observation: observation),
        ],
      ],
    );
  }

  Future<void> _refreshResource() async {
    final runtimeId = _resourceRuntimeId.text.trim();
    if (!RuntimeResourceClient.validRuntimeId(runtimeId)) {
      setState(() {
        _resourceObservation = null;
        _resourceMessage = '実行系IDは英数字、`.`、`_`、`-`だけで指定してください。';
      });
      return;
    }
    final generation = ++_resourceGeneration;
    setState(() {
      _resourceBusy = true;
      _resourceObservation = null;
      _resourceMessage = 'Brokerへ資源観測を要求しています。';
    });
    try {
      _resourceClient ??= widget.resourceClient ?? await widget.connectResource();
      final observation = await _resourceClient!.observe(runtimeId);
      if (!mounted || generation != _resourceGeneration) {
        return;
      }
      setState(() {
        _resourceObservation = observation;
        _resourceMessage = '観測監査ID ${observation.auditId} を確認しました。自動更新は行いません。';
      });
    } catch (_) {
      if (mounted && generation == _resourceGeneration) {
        setState(() {
          _resourceObservation = null;
          _resourceMessage = '資源観測の監査記録、結合状態、または応答を確認できません。';
        });
      }
    } finally {
      if (mounted && generation == _resourceGeneration) {
        setState(() => _resourceBusy = false);
      }
    }
  }

  RuntimeRecord? _selectedRuntime(ShellSnapshot snapshot) {
    if (snapshot.runtimes.isEmpty) {
      return null;
    }
    return snapshot.runtimes.firstWhere(
      (runtime) => runtime.runtimeId == _selectedRuntimeId,
      orElse: () => snapshot.runtimes.first,
    );
  }
}

class _RuntimeTable extends StatelessWidget {
  const _RuntimeTable({
    required this.snapshot,
    required this.selectedRuntimeId,
    required this.onSelected,
  });

  final ShellSnapshot snapshot;
  final String selectedRuntimeId;
  final ValueChanged<String> onSelected;

  @override
  Widget build(BuildContext context) {
    return SingleChildScrollView(
      scrollDirection: Axis.horizontal,
      child: DataTable(
        columns: const [
          DataColumn(label: Text('実行系')),
          DataColumn(label: Text('状態')),
          DataColumn(label: Text('アダプター')),
          DataColumn(label: Text('診断')),
        ],
        rows: [
          for (final runtime in snapshot.runtimes)
            DataRow(
              selected: runtime.runtimeId == selectedRuntimeId,
              onSelectChanged: (_) => onSelected(runtime.runtimeId),
              cells: [
                DataCell(Text(runtime.name)),
                DataCell(Text(runtime.status)),
                DataCell(Text(runtime.adapterId)),
                DataCell(Text(runtime.diagnosticSummary)),
              ],
            ),
        ],
      ),
    );
  }
}

class _RuntimeDetailPanel extends StatelessWidget {
  const _RuntimeDetailPanel({required this.snapshot, required this.runtime});

  final ShellSnapshot snapshot;
  final RuntimeRecord runtime;

  @override
  Widget build(BuildContext context) {
    final adapter = _adapterForRuntime(snapshot, runtime.runtimeId);
    final authority = snapshot.authorityMap
        .where((item) => item.runtimeId == runtime.runtimeId)
        .toList();
    final relatedProblems = snapshot.problems
        .where(
          (problem) =>
              problem.target.contains(runtime.runtimeId) ||
              authority.any((item) => item.recoveryId == problem.recoveryId),
        )
        .toList();
    return BorderedPanel(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text('実行系の詳細', style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 8),
          Text('実行系ID: ${runtime.runtimeId}'),
          Text('状態: ${runtime.status}'),
          Text('アダプター: ${runtime.adapterId}'),
          Text('スナップショット出所: ${snapshot.snapshotSource}'),
          Text('ネットワーク公開範囲: ${snapshot.networkExposure}'),
          const Divider(),
          Text('能力: ${authority.map((item) => item.capabilityId).join(', ')}'),
          Text('許可: ${authority.map((item) => item.permissionId).join(', ')}'),
          Text('承認: ${authority.map((item) => item.approvalId).join(', ')}'),
          Text(
            '直近監査: ${authority.map((item) => item.auditEventId).join(', ')}',
          ),
          Text('関連復旧: ${authority.map((item) => item.recoveryId).join(', ')}'),
          if (adapter != null) ...[
            const Divider(),
            Text('アダプター信頼: ${adapter.trustStatus}'),
            Text('要求能力: ${adapter.requestedCapabilities.join(', ')}'),
            Text('付与能力: ${adapter.grantedCapabilities.join(', ')}'),
            Text('拒否能力: ${adapter.deniedCapabilities.join(', ')}'),
          ],
          const Divider(),
          if (relatedProblems.isEmpty)
            const Text('関連問題: なし')
          else
            for (final problem in relatedProblems)
              Text('関連問題: ${problem.item} → ${problem.recoveryId}'),
        ],
      ),
    );
  }

  AdapterCatalogRecord? _adapterForRuntime(
    ShellSnapshot snapshot,
    String runtimeId,
  ) {
    for (final adapter in snapshot.adapterCatalog) {
      if (adapter.runtimeId == runtimeId) {
        return adapter;
      }
    }
    return null;
  }
}

class _ResourceObservationPanel extends StatelessWidget {
  const _ResourceObservationPanel({required this.observation});

  final RuntimeResourceObservation observation;

  @override
  Widget build(BuildContext context) {
    final binding = observation.binding;
    const metrics = <_MetricPresentation>[
      _MetricPresentation('稼働時間Millis', '稼働時間', 'ms'),
      _MetricPresentation('CPU累積時間Millis', 'CPU累積時間', 'ms'),
      _MetricPresentation('CPU利用率Percent', 'CPU利用率', '%'),
      _MetricPresentation('RAMWorkingSetBytes', 'RAM Working Set', 'bytes'),
      _MetricPresentation('RAMPrivateBytes', 'RAM Private', 'bytes'),
      _MetricPresentation('DiskIOBytes', 'Disk I/O', 'bytes'),
      _MetricPresentation('NetworkIOBytes', 'Network I/O', 'bytes'),
      _MetricPresentation('GPU利用率Percent', 'GPU利用率', '%'),
      _MetricPresentation('VRAMBytes', 'VRAM', 'bytes'),
      _MetricPresentation('処理中要求数', '処理中要求数', '件'),
      _MetricPresentation('平均応答Millis', '平均応答', 'ms'),
      _MetricPresentation('失敗要求数', '失敗要求数', '件'),
    ];
    return BorderedPanel(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text('最新のBroker資源観測',
              style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 8),
          SelectableText(
            '実行系ID: ${observation.runtimeId}\n'
            '観測時刻: ${_formatUnixMillis(observation.observedAtUnixMillis)}\n'
            '観測監査ID: ${observation.auditId}',
          ),
          const Divider(),
          Text('結合: ${binding.status}'),
          Text('根拠: ${binding.basis}'),
          if (binding.pid != null) Text('PID: ${binding.pid}'),
          if (binding.processCreationAtUnixMillis != null)
            Text('PID作成時刻: ${_formatUnixMillis(binding.processCreationAtUnixMillis!)}'),
          Text('登録時刻: ${_formatUnixMillis(binding.registeredAtUnixMillis)}'),
          Text('登録監査ID: ${binding.registrationAuditId}'),
          if (binding.reason != null) Text('結合できない理由: ${binding.reason}'),
          const Divider(),
          Text('統治: ${observation.governance.capabilityId} → '
              '${observation.governance.permissionId} → '
              '${observation.governance.approvalState} → '
              '${observation.governance.recoveryId}'),
          const SizedBox(height: 12),
          Text('計測', style: Theme.of(context).textTheme.titleSmall),
          const SizedBox(height: 8),
          Wrap(
            spacing: 12,
            runSpacing: 12,
            children: [
              for (final item in metrics)
                _ResourceMetricCard(
                  presentation: item,
                  metric: observation.metric(item.key),
                ),
            ],
          ),
          const SizedBox(height: 16),
          _ResourceHistorySection(history: observation.history),
        ],
      ),
    );
  }
}

class _MetricPresentation {
  const _MetricPresentation(this.key, this.label, this.unit);

  final String key;
  final String label;
  final String unit;
}

class _ResourceMetricCard extends StatelessWidget {
  const _ResourceMetricCard({
    required this.presentation,
    required this.metric,
  });

  final _MetricPresentation presentation;
  final RuntimeResourceMetric metric;

  @override
  Widget build(BuildContext context) {
    final value = metric.isMeasured
        ? '${_formatNumber(metric.value!)} ${presentation.unit}'
        : '不明';
    return SizedBox(
      width: 232,
      child: BorderedPanel(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(value, style: Theme.of(context).textTheme.titleMedium),
            Text(presentation.label),
            Text('証拠: ${metric.evidenceSource}'),
            if (!metric.isMeasured) Text('理由: ${metric.reason}'),
          ],
        ),
      ),
    );
  }
}

class _ResourceHistorySection extends StatelessWidget {
  const _ResourceHistorySection({required this.history});

  final List<RuntimeResourceHistorySample> history;

  @override
  Widget build(BuildContext context) {
    if (history.isEmpty) {
      return const SectionList(
        title: '短期履歴',
        rows: ['短期履歴はまだありません。未取得を数値0として表示しません。'],
      );
    }
    final latest = history.last;
    const latestMetrics = <_MetricPresentation>[
      _MetricPresentation(_resourceNetworkIoKey, '履歴のネットワークI/O', 'バイト'),
      _MetricPresentation('平均応答Millis', '履歴の平均応答', 'ms'),
      _MetricPresentation('エラー率Percent', '履歴のエラー率', '%'),
    ];
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text('短期履歴', style: Theme.of(context).textTheme.titleSmall),
        Text('保存点: ${history.length} ／ 直近: ${_formatUnixMillis(latest.observedAtUnixMillis)}'),
        const SizedBox(height: 8),
        _HistorySparkline(
          title: 'CPU利用率の短期履歴',
          color: Colors.teal,
          unit: '%',
          values: [
            for (final sample in history)
              sample.metric('CPU利用率Percent').value,
          ],
        ),
        const SizedBox(height: 8),
        _HistorySparkline(
          title: 'RAMワーキングセットの短期履歴',
          color: Colors.indigo,
          unit: 'バイト',
          values: [
            for (final sample in history)
              sample.metric(_resourceRamWorkingSetKey).value,
          ],
        ),
        const SizedBox(height: 12),
        Wrap(
          spacing: 12,
          runSpacing: 12,
          children: [
            for (final item in latestMetrics)
              _ResourceMetricCard(
                presentation: item,
                metric: latest.metric(item.key),
              ),
          ],
        ),
      ],
    );
  }
}

class _HistorySparkline extends StatelessWidget {
  const _HistorySparkline({
    required this.title,
    required this.color,
    required this.unit,
    required this.values,
  });

  final String title;
  final Color color;
  final String unit;
  final List<num?> values;

  @override
  Widget build(BuildContext context) {
    final measured = values.whereType<num>().toList(growable: false);
    if (measured.isEmpty) {
      return BorderedPanel(
        child: Text('$title: 測定値がありません。unknownを線や数値0として描画しません。'),
      );
    }
    final minimum = measured.reduce((left, right) => left < right ? left : right);
    final maximum = measured.reduce((left, right) => left > right ? left : right);
    return BorderedPanel(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(title),
          const SizedBox(height: 4),
          SizedBox(
            height: 116,
            width: double.infinity,
            child: CustomPaint(
              painter: _SparklinePainter(values: values, color: color),
            ),
          ),
          Text(
            '測定点 ${measured.length}/${values.length}。系列内で最小 ${_formatNumber(minimum)} $unit から最大 ${_formatNumber(maximum)} $unit へ正規化して描画します。',
          ),
        ],
      ),
    );
  }
}

class _SparklinePainter extends CustomPainter {
  const _SparklinePainter({required this.values, required this.color});

  final List<num?> values;
  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    final frame = Paint()
      ..color = color.withValues(alpha: 0.28)
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1;
    canvas.drawRect(Offset.zero & size, frame);
    final measured = values.whereType<num>().toList(growable: false);
    if (measured.isEmpty) {
      return;
    }
    var minimum = measured.first.toDouble();
    var maximum = minimum;
    for (final value in measured.skip(1)) {
      final number = value.toDouble();
      if (number < minimum) minimum = number;
      if (number > maximum) maximum = number;
    }
    final span = maximum - minimum;
    final path = Path();
    final points = values.length < 2 ? 1 : values.length - 1;
    var started = false;
    for (var index = 0; index < values.length; index++) {
      final value = values[index];
      if (value == null) {
        started = false;
        continue;
      }
      final x = size.width * index / points;
      final normalized = span == 0 ? 0.5 : (value.toDouble() - minimum) / span;
      final y = size.height - normalized * size.height;
      if (started) {
        path.lineTo(x, y);
      } else {
        path.moveTo(x, y);
        started = true;
      }
    }
    canvas.drawPath(
      path,
      Paint()
        ..color = color
        ..style = PaintingStyle.stroke
        ..strokeWidth = 2,
    );
  }

  @override
  bool shouldRepaint(covariant _SparklinePainter oldDelegate) =>
      oldDelegate.values != values || oldDelegate.color != color;
}

String _formatUnixMillis(int value) =>
    DateTime.fromMillisecondsSinceEpoch(value, isUtc: true)
        .toLocal()
        .toIso8601String();

String _formatNumber(num value) => value.toString();
