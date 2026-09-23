import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/services/observation_client.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart' show BrokerTransport;

class _FakeObservationTransport implements BrokerTransport {
  final operations = <String>[];
  Map<String, Object?> response = {
    'status': 'accepted',
    'body': {
      '版': 1,
      'Span一覧': [],
      'Trace一覧': [],
      'Metric一覧': [],
      '件数': 0,
      '証拠種別': 'INTERNAL_STATE',
      '観測範囲': 'broker_audit_finalization',
      'Auditとの責任分離': 'Audit=責任・安全・証拠; Observation=性能・挙動・運用状態',
      'OpenTelemetry export': 'unsupported',
      '権限生成': 'なし',
    },
  };

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    operations.add(operation);
    expect(payload?['版'], 1);
    return response;
  }
}

void main() {
  test('観測clientはBrokerのbounded一覧だけを要求する', () async {
    final transport = _FakeObservationTransport();
    final body = await ObservationClient(transport).list(
      limit: 12,
      traceId: 'observation-trace-1',
    );

    expect(transport.operations, ['観測一覧']);
    expect(body['証拠種別'], 'INTERNAL_STATE');
    expect(body['OpenTelemetry export'], 'unsupported');
  });
}
