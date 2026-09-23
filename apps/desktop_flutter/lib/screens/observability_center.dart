import 'package:flutter/material.dart';

import '../services/observation_client.dart';
import '../services/shell_core_client.dart';
import 'shared.dart';

class ObservabilityCenter extends StatefulWidget {
  const ObservabilityCenter({super.key, required this.client});

  final ShellCoreClient client;

  @override
  State<ObservabilityCenter> createState() => _ObservabilityCenterState();
}

class _ObservabilityCenterState extends State<ObservabilityCenter> {
  ObservationClient? _observationClient;
  Future<Map<String, Object?>>? _observationsFuture;

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
      _observationsFuture = null;
      return;
    }
    _observationsFuture = client.list();
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
        if (client == null)
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
                  child: Text('観測一覧を取得できません: ${snapshot.error}'),
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
}
