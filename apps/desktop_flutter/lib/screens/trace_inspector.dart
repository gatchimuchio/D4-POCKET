import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../services/observation_client.dart';
import '../services/shell_core_client.dart';
import 'shared.dart';

class TraceInspector extends StatefulWidget {
  const TraceInspector({super.key, required this.client});

  final ShellCoreClient client;

  @override
  State<TraceInspector> createState() => _TraceInspectorState();
}

class _TraceInspectorState extends State<TraceInspector> {
  ObservationClient? _observationClient;
  Future<Map<String, Object?>>? _tracesFuture;
  String? _selectedTraceId;

  @override
  void initState() {
    super.initState();
    _observationClient = widget.client.brokerTransport == null
        ? null
        : ObservationClient(widget.client.brokerTransport!);
    _refresh();
  }

  void _refresh() {
    final client = _observationClient;
    if (client == null) {
      _tracesFuture = null;
      return;
    }
    _tracesFuture = client.list(limit: 256, traceId: _selectedTraceId);
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
        if (client == null)
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
                  child: Text('Trace一覧を取得できません: ${snapshot.error}'),
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
                ),
              OutlinedButton(
                onPressed: () => setState(_refresh),
                child: const Text('更新'),
              ),
            ],
          ),
        ),
        if (spans.isEmpty)
          const EmptyStatePanel(
            title: 'Traceなし',
            meaning: '現在のBroker内部観測に表示対象のTraceはありません。',
            phaseBBlocked: false,
            nextAction: 'Broker操作後に更新してください。',
          )
        else
          _waterfallPanel(spans),
      ],
    );
  }

  Widget _waterfallPanel(List<Map<String, Object?>> spans) {
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
    final width = ((ended - started) / range)
        .clamp(0.015, availableWidth)
        .toDouble();
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

  int? _integer(Object? value) => value is num ? value.toInt() : null;

  List<Map<String, Object?>> _maps(Object? value) => value is List
      ? value.whereType<Map>().map(Map<String, Object?>.from).toList()
      : <Map<String, Object?>>[];
}
