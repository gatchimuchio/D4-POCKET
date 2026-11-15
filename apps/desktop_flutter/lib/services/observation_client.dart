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
    final response = await _transport.request('観測一覧', payload: payload);
    if (response['status'] != 'accepted') {
      throw const BrokerClientException('観測一覧がBrokerに拒否されました');
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
        !spans.every((item) => item is Map) ||
        !traces.every((item) => item is Map) ||
        !metrics.every((item) => item is Map) ||
        count != spans.length) {
      throw const BrokerClientException('観測一覧応答が契約に適合しません');
    }
  }
}
