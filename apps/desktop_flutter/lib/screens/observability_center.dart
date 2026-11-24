import 'package:flutter/material.dart';

import '../services/observation_client.dart';
import '../services/shell_core_client.dart';
import 'shared.dart';

class ObservabilityCenter extends StatefulWidget {
  const ObservabilityCenter({
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
  State<ObservabilityCenter> createState() => _ObservabilityCenterState();
}

class _ObservabilityCenterState extends State<ObservabilityCenter> {
  ObservationClient? _observationClient;
  Future<Map<String, Object?>>? _observationsFuture;
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
  void didUpdateWidget(covariant ObservabilityCenter oldWidget) {
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
    _observationsFuture = null;
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
      _observationsFuture = null;
      return;
    }
    _observationsFuture = _load(
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
    final observations = await client.list(hostId: hostId, taskIds: taskIds);
    return generation == _requestGeneration
        ? observations
        : const <String, Object?>{};
  }

  @override
  Widget build(BuildContext context) {
    final client = _observationClient;
    return ShellPage(
      title: '観測センター',
      children: [
        const BorderedPanel(
          child: Text(
            'Span、Trace、Metricは性能・挙動・運用状態の内部観測です。Auditの責任・安全・証拠を置き換えず、観測から権限を生成しません。',
          ),
        ),
        if (widget.taskIds != null && widget.hostId == null)
          const BorderedPanel(
            child: Text(
              '選択Hostが現在のBroker Hostと一致しないため、観測を表示しません。',
            ),
          )
        else if (client == null)
          const BorderedPanel(
            child: Text(
              'Rust Broker接続がないため、観測を取得できません。snapshotを実測値の代替にはしません。',
            ),
          )
        else
          FutureBuilder<Map<String, Object?>>(
            future: _observationsFuture,
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
                        '観測一覧を取得できません。Broker接続と観測状態を確認してください。',
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
              return _observationPanel(snapshot.data ?? const {});
            },
          ),
      ],
    );
  }

  Widget _observationPanel(Map<String, Object?> body) {
    final metrics = _maps(body['Metric一覧']);
    final spans = _maps(body['Span一覧']);
    final traces = _maps(body['Trace一覧']);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        BorderedPanel(
          child: Wrap(
            spacing: 16,
            runSpacing: 8,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              Text('Span: ${body['件数'] ?? 0}'),
              Text('Trace: ${traces.length}'),
              Text('Metric: ${metrics.length}'),
              if (widget.hostId != null)
                Text('Host: ${widget.hostId} / 現在Task: ${widget.taskIds?.length ?? '未指定'}'),
              OutlinedButton(
                onPressed: () => setState(_refresh),
                child: const Text('更新'),
              ),
            ],
          ),
        ),
        BorderedPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Text(
                '計測範囲',
                style: TextStyle(fontWeight: FontWeight.w600),
              ),
              const SizedBox(height: 4),
              Text('Window (UTC): ${_windowText(spans)}'),
              Text('Source: ${_sourceText(body, traces)}'),
              Text('Scope: ${body['観測範囲'] ?? 'unknown'}'),
              if (widget.hostId != null)
                Text('要求Context: Host=${widget.hostId} / Task=${widget.taskIds?.length ?? '未指定'}件'),
            ],
          ),
        ),
        BorderedPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Text(
                '監査との責任分離',
                style: TextStyle(fontWeight: FontWeight.w600),
              ),
              const SizedBox(height: 4),
              Text(body['Auditとの責任分離']?.toString() ?? '未取得'),
              const SizedBox(height: 4),
              Text('OpenTelemetry出力: ${body['OpenTelemetry export'] ?? '未対応'}'),
              const SizedBox(height: 4),
              Text(
                  '証拠種別: ${body['証拠種別'] ?? 'unknown'} / 権限生成: ${body['権限生成'] ?? 'unknown'}'),
            ],
          ),
        ),
        if (metrics.isNotEmpty) _metricPanel(metrics),
        if (spans.isEmpty)
          const EmptyStatePanel(
            title: '観測Spanなし',
            meaning: '現在のBroker内部で確定したAudit処理の観測はありません。',
            phaseBBlocked: false,
            nextAction: 'Broker操作後に更新してください。',
          )
        else
          for (final span in spans) _spanPanel(span),
      ],
    );
  }

  Widget _metricPanel(List<Map<String, Object?>> metrics) {
    return BorderedPanel(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text('Metric（計測値）',
              style: TextStyle(fontWeight: FontWeight.w600)),
          const SizedBox(height: 8),
          for (final metric in metrics)
            Padding(
              padding: const EdgeInsets.only(bottom: 4),
              child: Text(
                '${metric['名称'] ?? metric['MetricID'] ?? 'Metric'}: ${metric['値'] ?? 'unknown'} ${metric['単位'] ?? ''} (${metric['状態'] ?? 'unknown'})',
              ),
            ),
        ],
      ),
    );
  }

  Widget _spanPanel(Map<String, Object?> span) {
    return Padding(
      padding: const EdgeInsets.only(top: 12),
      child: BorderedPanel(
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              '${span['対象'] ?? 'unknown'} / ${span['操作'] ?? 'unknown'}',
              style: const TextStyle(fontWeight: FontWeight.w600),
            ),
            const SizedBox(height: 4),
            Text(
                'Trace識別子: ${span['TraceID'] ?? '不明'} / Span識別子: ${span['SpanID'] ?? '不明'}'),
            Text(
                '開始 (UTC): ${_utcTimestamp(span['開始EpochMillis'])} / 終了 (UTC): ${_utcTimestamp(span['終了EpochMillis'])}'),
            Text(
                '状態: ${span['状態'] ?? 'unknown'} / 所要: ${span['所要Millis'] ?? 'unknown'} ms'),
            Text(
                '関連Audit: ${span['関連監査ID'] ?? 'unknown'} / 証拠: ${span['証拠種別'] ?? 'unknown'}'),
          ],
        ),
      ),
    );
  }

  List<Map<String, Object?>> _maps(Object? value) => value is List
      ? value.whereType<Map>().map(Map<String, Object?>.from).toList()
      : <Map<String, Object?>>[];

  String _windowText(List<Map<String, Object?>> spans) {
    final starts =
        spans.map((span) => span['開始EpochMillis']).whereType<int>().toList();
    final ends =
        spans.map((span) => span['終了EpochMillis']).whereType<int>().toList();
    if (starts.isEmpty || ends.isEmpty) return 'unknown (観測Spanなし)';
    starts.sort();
    ends.sort();
    return '${_utcTimestamp(starts.first)} – ${_utcTimestamp(ends.last)}';
  }

  String _sourceText(
    Map<String, Object?> body,
    List<Map<String, Object?>> traces,
  ) {
    final sources = traces
        .map((trace) => trace['生成源'])
        .whereType<String>()
        .toSet()
        .toList()
      ..sort();
    return '${body['証拠種別'] ?? 'unknown'} / ${sources.isEmpty ? '生成源unknown' : sources.join(', ')}';
  }

  String _utcTimestamp(Object? value) {
    if (value is! int || value < 0) return 'unknown';
    try {
      return DateTime.fromMillisecondsSinceEpoch(value, isUtc: true)
          .toIso8601String();
    } on Object {
      return '$value epoch ms';
    }
  }
}
