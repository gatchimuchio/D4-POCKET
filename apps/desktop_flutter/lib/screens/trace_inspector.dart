import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../services/observation_client.dart';
import '../services/shell_core_client.dart';
import 'shared.dart';

class TraceInspector extends StatefulWidget {
  const TraceInspector({
    super.key,
    required this.client,
    this.hostId,
    this.taskIds,
    this.hostContextGeneration = 0,
    this.taskContextGeneration = 0,
  });

  final ShellCoreClient client;
  final String? hostId;
  final List<String>? taskIds;
  final int hostContextGeneration;
  final int taskContextGeneration;

  @override
  State<TraceInspector> createState() => _TraceInspectorState();
}

class _TraceInspectorState extends State<TraceInspector> {
  ObservationClient? _observationClient;
  Future<Map<String, Object?>>? _tracesFuture;
  String? _selectedTraceId;
  int _requestGeneration = 0;

  @override
  void initState() {
    super.initState();
    _observationClient = _clientFor(widget.client);
    _refresh();
  }

  ObservationClient? _clientFor(ShellCoreClient client) {
    final transport = client.brokerTransport;
    return transport == null ? null : ObservationClient(transport);
  }

  @override
  void didUpdateWidget(covariant TraceInspector oldWidget) {
    super.didUpdateWidget(oldWidget);
    final clientChanged = !identical(
      oldWidget.client.brokerTransport,
      widget.client.brokerTransport,
    );
    final scopeChanged = oldWidget.hostId != widget.hostId ||
        !_sameTaskIds(oldWidget.taskIds, widget.taskIds) ||
        oldWidget.hostContextGeneration != widget.hostContextGeneration ||
        oldWidget.taskContextGeneration != widget.taskContextGeneration;
    if (!clientChanged && !scopeChanged) {
      return;
    }

    _observationClient = _clientFor(widget.client);
    _selectedTraceId = null;
    _tracesFuture = null;
    _refresh();
  }

  bool _sameTaskIds(List<String>? left, List<String>? right) {
    if (left == null || right == null) return left == null && right == null;
    if (left.length != right.length) return false;
    for (var index = 0; index < left.length; index++) {
      if (left[index] != right[index]) return false;
    }
    return true;
  }

  void _refresh() {
    final generation = ++_requestGeneration;
    final client = _observationClient;
    final taskIds = widget.taskIds == null
        ? null
        : List<String>.unmodifiable(widget.taskIds!);
    if (client == null || (taskIds != null && widget.hostId == null)) {
      _tracesFuture = null;
      return;
    }
    _tracesFuture = _load(
      client,
      generation,
      hostId: widget.hostId,
      taskIds: taskIds,
    );
  }

  Future<Map<String, Object?>> _load(
    ObservationClient client,
    int generation, {
    required String? hostId,
    required List<String>? taskIds,
  }) async {
    final observations = await client.list(
      limit: 256,
      traceId: _selectedTraceId,
      hostId: hostId,
      taskIds: taskIds,
    );
    return generation == _requestGeneration
        ? observations
        : const <String, Object?>{};
  }

  @override
  Widget build(BuildContext context) {
    final client = _observationClient;
    return ShellPage(
      title: '追跡情報',
      children: [
        const BorderedPanel(
          child: Text(
            'Traceの処理段階、所要時間、状態、親子関係、エラー分類を表示します。表示値はBroker内部観測であり、Auditの責任・安全・証拠ではありません。',
          ),
        ),
        const BorderedPanel(
          child: Text(
            '現在の実測対象: Broker。Runtime、Adapter、Tool、外部通信は、対応するproduction観測経路が接続されるまで実測済みとして表示しません。',
          ),
        ),
        const BorderedPanel(
          child: Text(
            'Trace表示からPermission、Approval、Authority、Capability、Credentialを生成しません。',
          ),
        ),
        if (widget.taskIds != null && widget.hostId == null)
          const BorderedPanel(
            child: Text(
              '選択Hostが現在のBroker Hostと一致しないため、Traceを表示しません。',
            ),
          )
        else if (client == null)
          const BorderedPanel(
            child: Text(
              'Rust Broker接続がないため、Traceを取得できません。snapshotをTraceの根拠にはしません。',
            ),
          )
        else
          FutureBuilder<Map<String, Object?>>(
            future: _tracesFuture,
            builder: (context, snapshot) {
              if (snapshot.connectionState == ConnectionState.waiting) {
                return const BorderedPanel(child: LinearProgressIndicator());
              }
              if (snapshot.hasError) {
                return BorderedPanel(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      const Text(
                        'Trace一覧を取得できません。Broker接続と観測状態を確認してください。',
                      ),
                      const SizedBox(height: 8),
                      OutlinedButton(
                        onPressed: () => setState(_refresh),
                        child: const Text('再試行'),
                      ),
                    ],
                  ),
                );
              }
              return _tracePanel(snapshot.data ?? const {});
            },
          ),
      ],
    );
  }

  Widget _tracePanel(Map<String, Object?> body) {
    final spans = _maps(body['Span一覧']);
    final traces = _maps(body['Trace一覧']);
    final metrics = _maps(body['Metric一覧']);
    final traceIds = traces
        .map((trace) => trace['TraceID']?.toString())
        .whereType<String>()
        .toList();
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        BorderedPanel(
          child: Wrap(
            spacing: 12,
            runSpacing: 8,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              Text('Trace: ${traceIds.length}'),
              Text('Span: ${spans.length}'),
              Text('Metric: ${metrics.length}'),
              if (widget.hostId != null)
                Text('Host: ${widget.hostId} / 現在Task: ${widget.taskIds?.length ?? '未指定'}'),
              if (traceIds.isNotEmpty)
                DropdownButton<String?>(
                  value: _selectedTraceId,
                  hint: const Text('Traceを選択'),
                  items: [
                    const DropdownMenuItem<String?>(
                      value: null,
                      child: Text('全Trace'),
                    ),
                    for (final traceId in traceIds)
                      DropdownMenuItem<String?>(
                        value: traceId,
                        child: Text(traceId),
                      ),
                  ],
                  onChanged: (value) => setState(() {
                    _selectedTraceId = value;
                    _refresh();
                  }),
                )
              else if (_selectedTraceId != null)
                OutlinedButton(
                  onPressed: () => setState(() {
                    _selectedTraceId = null;
                    _refresh();
                  }),
                  child: const Text('全Traceを表示'),
                ),
              OutlinedButton(
                onPressed: () => setState(_refresh),
                child: const Text('更新'),
              ),
            ],
          ),
        ),
        if (metrics.isNotEmpty) _metricPanel(metrics),
        if (traces.isNotEmpty) _traceLineagePanel(traces),
        if (spans.isEmpty)
          EmptyStatePanel(
            title: 'Traceなし',
            meaning: _selectedTraceId == null
                ? '現在のBroker内部観測に表示対象のTraceはありません。'
                : '選択したTraceは現在のBroker内部観測にありません。',
            phaseBBlocked: false,
            nextAction: _selectedTraceId == null
                ? 'Broker操作後に更新してください。'
                : '全Traceを表示して直近の観測を取得できます。',
          )
        else
          _waterfallPanel(
            spans,
            evidenceSource: _traceSource(body, traces),
            scope: body['観測範囲']?.toString() ?? 'unknown',
          ),
      ],
    );
  }

  Widget _traceLineagePanel(List<Map<String, Object?>> traces) {
    final traceIds = traces
        .map((trace) => trace['TraceID'])
        .whereType<String>()
        .toSet();
    return BorderedPanel(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text(
            'Task／Tool／Followup Trace相関（Broker内部観測）',
            style: TextStyle(fontWeight: FontWeight.w600),
          ),
          const SizedBox(height: 8),
          for (final trace in traces)
            Padding(
              padding: const EdgeInsets.only(bottom: 8),
              child: Text(
                _traceLineageText(trace, traceIds),
                style: Theme.of(context).textTheme.bodySmall,
              ),
            ),
          Text(
            '同じTraceIDのSpanは同じ観測Traceに属します。親TraceIDが応答範囲外の場合、親の内容は取得・補完していません。これは相関表示であり、Auditや実行権限の根拠ではありません。',
            style: Theme.of(context).textTheme.bodySmall,
          ),
        ],
      ),
    );
  }

  String _traceLineageText(
    Map<String, Object?> trace,
    Set<String> responseTraceIds,
  ) {
    final traceId = trace['TraceID']?.toString() ?? 'unknown';
    final parentTraceId = trace['親TraceID'];
    final parent = switch (parentTraceId) {
      final String id when responseTraceIds.contains(id) => '$id（応答内）',
      final String id => '$id（応答範囲外）',
      _ => 'なし',
    };
    return '$traceId / Span: ${trace['件数'] ?? 'unknown'} / 親Trace: $parent';
  }

  Widget _metricPanel(List<Map<String, Object?>> metrics) => BorderedPanel(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Text(
              '表示範囲のMetric（Broker内部観測）',
              style: TextStyle(fontWeight: FontWeight.w600),
            ),
            const SizedBox(height: 8),
            for (final metric in metrics)
              Padding(
                padding: const EdgeInsets.only(bottom: 8),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      '${metric['名称'] ?? metric['MetricID'] ?? 'Metric'}: ${metric['値'] ?? 'unknown'} ${metric['単位'] ?? ''} (${metric['状態'] ?? 'unknown'})',
                      style: const TextStyle(fontWeight: FontWeight.w500),
                    ),
                    Text(
                      '種別: ${metric['種別'] ?? 'unknown'} / 観測件数: ${metric['観測件数'] ?? 'unknown'} / Source: ${metric['証拠種別'] ?? 'unknown'}',
                      style: Theme.of(context).textTheme.bodySmall,
                    ),
                  ],
                ),
              ),
          ],
        ),
      );

  Widget _waterfallPanel(
    List<Map<String, Object?>> spans, {
    required String evidenceSource,
    required String scope,
  }) {
    final starts = spans
        .map((span) => _integer(span['開始EpochMillis']))
        .whereType<int>()
        .toList();
    final ends = spans
        .map((span) => _integer(span['終了EpochMillis']))
        .whereType<int>()
        .toList();
    final minimum = starts.isEmpty ? 0 : starts.reduce(math.min);
    final maximum = ends.isEmpty ? minimum : ends.reduce(math.max);
    final range = math.max(1, maximum - minimum).toDouble();
    return BorderedPanel(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text(
            '処理時間 waterfall',
            style: TextStyle(fontWeight: FontWeight.w600),
          ),
          const SizedBox(height: 4),
          Text('観測対象: ${_bodyScope(spans)}'),
          Text('Window (UTC): ${_windowText(minimum, maximum)}'),
          Text('Source: $evidenceSource'),
          Text('Scope: $scope'),
          const SizedBox(height: 10),
          for (final span in spans)
            _spanRow(span, minimum: minimum, range: range),
        ],
      ),
    );
  }

  Widget _spanRow(
    Map<String, Object?> span, {
    required int minimum,
    required double range,
  }) {
    final started = _integer(span['開始EpochMillis']) ?? minimum;
    final ended = _integer(span['終了EpochMillis']) ?? started;
    final left = ((started - minimum) / range).clamp(0.0, 1.0).toDouble();
    final availableWidth = math.max(0.015, 1.0 - left).toDouble();
    final width =
        ((ended - started) / range).clamp(0.015, availableWidth).toDouble();
    final status = span['状態']?.toString() ?? '不明';
    final error = span['エラー分類']?.toString();
    return Padding(
      padding: const EdgeInsets.only(top: 10),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            '${span['対象'] ?? '不明'} / ${span['操作'] ?? '不明'} / $status',
            style: const TextStyle(fontWeight: FontWeight.w600),
          ),
          const SizedBox(height: 4),
          SizedBox(
            height: 20,
            child: LayoutBuilder(
              builder: (context, constraints) => Stack(
                children: [
                  const Positioned.fill(
                    child: DecoratedBox(
                      decoration: BoxDecoration(color: Color(0x1f607d8b)),
                    ),
                  ),
                  Positioned(
                    left: constraints.maxWidth * left,
                    width: constraints.maxWidth * width,
                    top: 2,
                    bottom: 2,
                    child: DecoratedBox(
                      decoration: BoxDecoration(
                        color: _statusColor(context, status),
                        borderRadius: BorderRadius.circular(4),
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ),
          Text(
            'Trace識別子: ${span['TraceID'] ?? '不明'} / Span識別子: ${span['SpanID'] ?? '不明'} / 所要: ${span['所要Millis'] ?? '不明'} ms',
            style: Theme.of(context).textTheme.bodySmall,
          ),
          Text(
            '開始 (UTC): ${_utcTimestamp(span['開始EpochMillis'])} / 終了 (UTC): ${_utcTimestamp(span['終了EpochMillis'])} / Source: ${span['証拠種別'] ?? 'unknown'}',
            style: Theme.of(context).textTheme.bodySmall,
          ),
          Text(
            '親Span: ${span['親SpanID'] ?? 'なし'} / 関連Audit: ${span['関連監査ID'] ?? '不明'}',
            style: Theme.of(context).textTheme.bodySmall,
          ),
          if (error != null && error.isNotEmpty)
            Text('エラー分類: $error', style: Theme.of(context).textTheme.bodySmall),
        ],
      ),
    );
  }

  Color _statusColor(BuildContext context, String status) => switch (status) {
        'accepted' => Theme.of(context).colorScheme.primary,
        'rejected' || 'suspended' => Theme.of(context).colorScheme.error,
        _ => Theme.of(context).colorScheme.secondary,
      };

  String _bodyScope(List<Map<String, Object?>> spans) {
    final targets = spans
        .map((span) => span['対象']?.toString())
        .whereType<String>()
        .toSet()
        .toList();
    return targets.isEmpty ? '不明' : targets.join('、');
  }

  String _traceSource(
    Map<String, Object?> body,
    List<Map<String, Object?>> traces,
  ) {
    final sources = traces
        .map((trace) => trace['生成源'])
        .whereType<String>()
        .toSet()
        .toList()
      ..sort();
    final evidence = '${body['証拠種別'] ?? 'unknown'}';
    return '$evidence / ${sources.isEmpty ? '生成源unknown' : sources.join(', ')}';
  }

  String _windowText(int start, int end) =>
      '${_utcTimestamp(start)} – ${_utcTimestamp(end)}';

  String _utcTimestamp(Object? value) {
    if (value is! int || value < 0) return 'unknown';
    try {
      return DateTime.fromMillisecondsSinceEpoch(value, isUtc: true)
          .toIso8601String();
    } on Object {
      return '$value epoch ms';
    }
  }

  int? _integer(Object? value) => value is num ? value.toInt() : null;

  List<Map<String, Object?>> _maps(Object? value) => value is List
      ? value.whereType<Map>().map(Map<String, Object?>.from).toList()
      : <Map<String, Object?>>[];
}
