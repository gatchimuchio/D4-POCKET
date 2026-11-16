import 'dart:convert';

import 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerTransport, BrokerClientException;

const int _maximumSpans = 256;
const int _maximumTraceIdBytes = 128;
const int _maximumMetrics = 16;
const String _observationScope = 'broker_audit_finalization';
const String _auditSeparation = 'Audit=責任・安全・証拠; Observation=性能・挙動・運用状態';
const Set<String> _observationBodyKeys = {
  '版',
  'Span一覧',
  'Trace一覧',
  'Metric一覧',
  '件数',
  '証拠種別',
  '観測範囲',
  'Auditとの責任分離',
  'OpenTelemetry export',
  '権限生成',
};
const Set<String> _spanKeys = {
  '版',
  'SpanID',
  'TraceID',
  '親SpanID',
  '対象',
  '操作',
  '開始EpochMillis',
  '終了EpochMillis',
  '所要Millis',
  '状態',
  'エラー分類',
  '関連監査ID',
  '証拠種別',
};
const Set<String> _traceKeys = {
  '版',
  'TraceID',
  'SpanID一覧',
  '件数',
  '証拠種別',
  '生成源',
  '親TraceID',
};
const Set<String> _metricKeys = {
  '版',
  'MetricID',
  '名称',
  '値',
  '単位',
  '種別',
  '観測件数',
  '状態',
  '証拠種別',
};

class ObservationClient {
  const ObservationClient(this._transport);

  final BrokerTransport _transport;

  Future<Map<String, Object?>> list({
    int limit = 200,
    String? traceId,
  }) async {
    if (limit < 1 || limit > _maximumSpans) {
      throw const BrokerClientException('観測一覧の上限が契約範囲外です');
    }
    if (traceId != null &&
        (traceId.isEmpty ||
            utf8.encode(traceId).length > _maximumTraceIdBytes)) {
      throw const BrokerClientException('TraceIDが契約範囲外です');
    }

    final payload = <String, Object?>{
      '版': 1,
      '上限': limit,
      if (traceId != null) 'TraceID': traceId,
    };
    late final Map<String, Object?> response;
    try {
      response = await _transport.request('観測一覧', payload: payload);
    } on Object {
      throw const BrokerClientException('観測一覧のBroker通信を確認できません');
    }
    if (response['status'] != 'accepted') {
      throw const BrokerClientException('観測一覧がBrokerに拒否されました');
    }
    if (response['error'] != null) {
      throw const BrokerClientException('観測一覧accepted応答にerrorが含まれています');
    }
    final rawBody = response['body'];
    if (rawBody is! Map || rawBody.keys.any((key) => key is! String)) {
      throw const BrokerClientException('観測一覧応答が契約に適合しません');
    }
    final body = <String, Object?>{
      for (final entry in rawBody.entries) entry.key as String: entry.value,
    };
    _validateBody(body, limit: limit);
    return body;
  }

  void _validateBody(Map<String, Object?> body, {required int limit}) {
    if (body.keys.toSet().difference(_observationBodyKeys).isNotEmpty ||
        _observationBodyKeys.difference(body.keys.toSet()).isNotEmpty ||
        body['版'] != 1 ||
        body['証拠種別'] != 'INTERNAL_STATE' ||
        body['観測範囲'] != _observationScope ||
        body['Auditとの責任分離'] != _auditSeparation ||
        body['OpenTelemetry export'] != 'unsupported' ||
        body['権限生成'] != 'なし') {
      throw const BrokerClientException('観測一覧応答が契約に適合しません');
    }

    final spans = body['Span一覧'];
    final traces = body['Trace一覧'];
    final metrics = body['Metric一覧'];
    final count = body['件数'];
    if (spans is! List ||
        traces is! List ||
        metrics is! List ||
        count is! int) {
      throw const BrokerClientException('観測一覧応答が契約に適合しません');
    }
    if (spans.length > limit ||
        traces.length > limit ||
        metrics.length > _maximumMetrics ||
        count != spans.length) {
      throw const BrokerClientException('観測一覧応答が契約に適合しません');
    }

    final spanMaps = <Map<String, Object?>>[];
    for (final item in spans) {
      final span = _stringMap(item);
      if (span == null || !_validSpan(span)) {
        throw const BrokerClientException('観測一覧応答が契約に適合しません');
      }
      spanMaps.add(span);
    }
    final traceMaps = <Map<String, Object?>>[];
    for (final item in traces) {
      final trace = _stringMap(item);
      if (trace == null || !_validTrace(trace)) {
        throw const BrokerClientException('観測一覧応答が契約に適合しません');
      }
      traceMaps.add(trace);
    }
    for (final item in metrics) {
      final metric = _stringMap(item);
      if (metric == null || !_validMetric(metric)) {
        throw const BrokerClientException('観測一覧応答が契約に適合しません');
      }
    }
    if (!_validTraceRelations(spanMaps, traceMaps)) {
      throw const BrokerClientException('観測一覧応答が契約に適合しません');
    }
  }

  Map<String, Object?>? _stringMap(Object? value) {
    if (value is! Map || value.keys.any((key) => key is! String)) {
      return null;
    }
    return {
      for (final entry in value.entries) entry.key as String: entry.value,
    };
  }

  bool _hasExactKeys(Map<String, Object?> value, Set<String> expected) =>
      value.length == expected.length && expected.every(value.containsKey);

  bool _validSpan(Map<String, Object?> span) {
    final spanId = span['SpanID'];
    final traceId = span['TraceID'];
    final startedAt = span['開始EpochMillis'];
    final endedAt = span['終了EpochMillis'];
    final duration = span['所要Millis'];
    final error = span['エラー分類'];
    final parentSpanId = span['親SpanID'];
    return _hasExactKeys(span, _spanKeys) &&
        span['版'] == 1 &&
        spanId is String &&
        RegExp(r'^observation-span-[1-9][0-9]*$').hasMatch(spanId) &&
        traceId is String &&
        RegExp(r'^observation-trace-[1-9][0-9]*$').hasMatch(traceId) &&
        (parentSpanId == null ||
            (parentSpanId is String &&
                RegExp(r'^observation-span-[1-9][0-9]*$')
                    .hasMatch(parentSpanId))) &&
        span['対象'] == 'Broker' &&
        span['操作'] is String &&
        (span['操作'] as String).isNotEmpty &&
        (span['操作'] as String).length <= 128 &&
        startedAt is int &&
        startedAt >= 0 &&
        endedAt is int &&
        endedAt >= startedAt &&
        duration is int &&
        duration >= 0 &&
        duration <= 86400000 &&
        endedAt - startedAt == duration &&
        span['状態'] is String &&
        RegExp(r'^[a-z][a-z0-9_-]{0,31}$').hasMatch(span['状態'] as String) &&
        (error == null ||
            (error is String &&
                error.length <= 128 &&
                RegExp(r'^[a-z][a-z0-9_.-]*$').hasMatch(error))) &&
        _validIdentifier(span['関連監査ID']) &&
        span['証拠種別'] == 'INTERNAL_STATE';
  }

  bool _validTrace(Map<String, Object?> trace) {
    final traceId = trace['TraceID'];
    final spanIds = trace['SpanID一覧'];
    final count = trace['件数'];
    final parentTraceId = trace['親TraceID'];
    return _hasExactKeys(trace, _traceKeys) &&
        trace['版'] == 1 &&
        traceId is String &&
        RegExp(r'^observation-trace-[1-9][0-9]*$').hasMatch(traceId) &&
        spanIds is List &&
        spanIds.isNotEmpty &&
        spanIds.length <= 1024 &&
        spanIds.every((spanId) =>
            spanId is String &&
            RegExp(r'^observation-span-[1-9][0-9]*$').hasMatch(spanId)) &&
        count is int &&
        count == spanIds.length &&
        trace['証拠種別'] == 'INTERNAL_STATE' &&
        trace['生成源'] == 'broker_audit_finalization' &&
        (parentTraceId == null ||
            (parentTraceId is String &&
                RegExp(r'^observation-trace-[1-9][0-9]*$')
                    .hasMatch(parentTraceId)));
  }

  bool _validMetric(Map<String, Object?> metric) {
    final value = metric['値'];
    final sampleCount = metric['観測件数'];
    return _hasExactKeys(metric, _metricKeys) &&
        metric['版'] == 1 &&
        metric['MetricID'] is String &&
        RegExp(r'^broker\.[a-z0-9_.-]+$')
            .hasMatch(metric['MetricID'] as String) &&
        metric['名称'] is String &&
        (metric['名称'] as String).isNotEmpty &&
        (metric['名称'] as String).length <= 128 &&
        (value == null || (value is num && value.isFinite && value >= 0)) &&
        const {'count', 'ms'}.contains(metric['単位']) &&
        const {'counter', 'gauge'}.contains(metric['種別']) &&
        sampleCount is int &&
        sampleCount >= 0 &&
        sampleCount <= 1024 &&
        const {'observed', 'unknown'}.contains(metric['状態']) &&
        metric['証拠種別'] == 'INTERNAL_STATE';
  }

  bool _validTraceRelations(
    List<Map<String, Object?>> spans,
    List<Map<String, Object?>> traces,
  ) {
    final spanTraces = <String, String>{};
    for (final span in spans) {
      final spanId = span['SpanID'] as String;
      if (spanTraces.containsKey(spanId)) return false;
      spanTraces[spanId] = span['TraceID'] as String;
    }
    final seenTraces = <String>{};
    final seenSpans = <String>{};
    for (final trace in traces) {
      final traceId = trace['TraceID'] as String;
      if (!seenTraces.add(traceId)) return false;
      for (final spanId in trace['SpanID一覧'] as List) {
        if (!seenSpans.add(spanId as String) || spanTraces[spanId] != traceId) {
          return false;
        }
      }
    }
    return seenSpans.length == spanTraces.length;
  }

  bool _validIdentifier(Object? value) =>
      value is String &&
      value.isNotEmpty &&
      utf8.encode(value).length <= 128 &&
      RegExp(r'^[A-Za-z0-9_.:-]+$').hasMatch(value);
}
