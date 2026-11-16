import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/services/observation_client.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerTransport, BrokerClientException;

class _FakeObservationTransport implements BrokerTransport {
  _FakeObservationTransport({this.transportError});

  final operations = <String>[];
  Map<String, Object?>? lastPayload;
  final Object? transportError;
  Map<String, Object?> response = {
    'status': 'accepted',
    'body': _validBody(),
  };

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    operations.add(operation);
    lastPayload = payload == null ? null : Map<String, Object?>.from(payload);
    if (transportError != null) throw transportError!;
    return response;
  }
}

Map<String, Object?> _validBody() => {
      '版': 1,
      'Span一覧': <Map<String, Object?>>[],
      'Trace一覧': <Map<String, Object?>>[],
      'Metric一覧': <Map<String, Object?>>[],
      '件数': 0,
      '証拠種別': 'INTERNAL_STATE',
      '観測範囲': 'broker_audit_finalization',
      'Auditとの責任分離': 'Audit=責任・安全・証拠; Observation=性能・挙動・運用状態',
      'OpenTelemetry export': 'unsupported',
      '権限生成': 'なし',
    };

Map<String, Object?> _sampleBody() => _validBody()
  ..['Span一覧'] = [
    {
      '版': 1,
      'SpanID': 'observation-span-1',
      'TraceID': 'observation-trace-1',
      '親SpanID': null,
      '対象': 'Broker',
      '操作': 'MCP Tool実行',
      '開始EpochMillis': 1700000000000,
      '終了EpochMillis': 1700000000012,
      '所要Millis': 12,
      '状態': 'accepted',
      'エラー分類': null,
      '関連監査ID': 'broker-audit-1',
      '証拠種別': 'INTERNAL_STATE',
    },
  ]
  ..['Trace一覧'] = [
    {
      '版': 1,
      'TraceID': 'observation-trace-1',
      'SpanID一覧': ['observation-span-1'],
      '件数': 1,
      '証拠種別': 'INTERNAL_STATE',
      '生成源': 'broker_audit_finalization',
      '親TraceID': null,
    },
  ]
  ..['Metric一覧'] = [
    {
      '版': 1,
      'MetricID': 'broker.operation.count',
      '名称': '操作観測数',
      '値': 1,
      '単位': 'count',
      '種別': 'counter',
      '観測件数': 1,
      '状態': 'observed',
      '証拠種別': 'INTERNAL_STATE',
    },
  ]
  ..['件数'] = 1;

void main() {
  test('観測clientは契約内のbounded一覧をBrokerへ要求する', () async {
    final transport = _FakeObservationTransport();
    final body = await ObservationClient(transport).list(
      limit: 12,
      traceId: 'observation-trace-1',
    );

    expect(transport.operations, ['観測一覧']);
    expect(transport.lastPayload, {
      '版': 1,
      '上限': 12,
      'TraceID': 'observation-trace-1',
    });
    expect(body['証拠種別'], 'INTERNAL_STATE');
    expect(body['OpenTelemetry export'], 'unsupported');
  });

  test('範囲外の上限とTraceIDはBrokerへ送らない', () async {
    final transport = _FakeObservationTransport();
    final client = ObservationClient(transport);

    for (final limit in [0, 257]) {
      await expectLater(
        () => client.list(limit: limit),
        throwsA(isA<BrokerClientException>()),
      );
    }
    for (final traceId in ['', List.filled(43, 'あ').join()]) {
      await expectLater(
        () => client.list(traceId: traceId),
        throwsA(isA<BrokerClientException>()),
      );
    }

    expect(transport.operations, isEmpty);
  });

  test('128-byte TraceIDと256件上限は受理する', () async {
    final transport = _FakeObservationTransport();
    await ObservationClient(transport).list(
      limit: 256,
      traceId: List.filled(128, 'x').join(),
    );

    expect(transport.lastPayload?['上限'], 256);
    expect(transport.lastPayload?['TraceID'], List.filled(128, 'x').join());
  });

  test('nested schemaに合うSpan/Trace/Metricだけを受け入れる', () async {
    final transport = _FakeObservationTransport()
      ..response = {'status': 'accepted', 'body': _sampleBody()};
    final body = await ObservationClient(transport).list();

    expect(body['件数'], 1);
  });

  test('nested objectの余分な秘密fieldとSpan/Trace不整合を拒否する', () async {
    final invalidBodies = <Map<String, Object?>>[];

    final spanSecret = _sampleBody();
    (spanSecret['Span一覧'] as List).single['秘密本文'] = 'secret marker';
    invalidBodies.add(spanSecret);

    final metricSecret = _sampleBody();
    (metricSecret['Metric一覧'] as List).single['raw_input'] = 'private';
    invalidBodies.add(metricSecret);

    final badTraceLink = _sampleBody();
    ((badTraceLink['Trace一覧'] as List).single['SpanID一覧'] as List)[0] =
        'observation-span-2';
    invalidBodies.add(badTraceLink);

    final badDuration = _sampleBody();
    (badDuration['Span一覧'] as List).single['所要Millis'] = 13;
    invalidBodies.add(badDuration);

    for (final body in invalidBodies) {
      final transport = _FakeObservationTransport()
        ..response = {'status': 'accepted', 'body': body};
      await expectLater(
        () => ObservationClient(transport).list(),
        throwsA(isA<BrokerClientException>()),
      );
    }
  });

  test('契約外・上限超過・Authority誤表記の応答を拒否する', () async {
    final invalidBodies = <Map<String, Object?>>[];

    final missingField = _validBody()..remove('証拠種別');
    invalidBodies.add(missingField);

    final extraField = _validBody()..['追加'] = '未定義';
    invalidBodies.add(extraField);

    final wrongCount = _validBody()..['件数'] = 1;
    invalidBodies.add(wrongCount);

    final wrongAuthority = _validBody()..['権限生成'] = 'あり';
    invalidBodies.add(wrongAuthority);

    final wrongEvidence = _validBody()..['証拠種別'] = 'EXTERNAL_EVIDENCE';
    invalidBodies.add(wrongEvidence);

    final oversized = _validBody()
      ..['Span一覧'] = List<Map<String, Object?>>.generate(
        257,
        (_) => <String, Object?>{},
      )
      ..['件数'] = 257;
    invalidBodies.add(oversized);

    final oversizedTraces = _validBody()
      ..['Trace一覧'] = List<Map<String, Object?>>.generate(
        257,
        (_) => <String, Object?>{},
      );
    invalidBodies.add(oversizedTraces);

    final oversizedMetrics = _validBody()
      ..['Metric一覧'] = List<Map<String, Object?>>.generate(
        17,
        (_) => <String, Object?>{},
      );
    invalidBodies.add(oversizedMetrics);

    for (final body in invalidBodies) {
      final transport = _FakeObservationTransport()
        ..response = {'status': 'accepted', 'body': body};
      await expectLater(
        () => ObservationClient(transport).list(),
        throwsA(isA<BrokerClientException>()),
      );
    }

    final nonObject = _FakeObservationTransport()
      ..response = {'status': 'accepted', 'body': <Object?>[]};
    await expectLater(
      () => ObservationClient(nonObject).list(),
      throwsA(isA<BrokerClientException>()),
    );
  });

  test('Broker拒否の自由記述を例外へ露出しない', () async {
    final transport = _FakeObservationTransport()
      ..response = {
        'status': 'rejected',
        'error': {'message': 'secret/path/command detail'},
      };

    await expectLater(
      () => ObservationClient(transport).list(),
      throwsA(
        isA<BrokerClientException>().having(
          (error) => error.toString(),
          'sanitized error',
          isNot(contains('secret/path/command detail')),
        ),
      ),
    );
  });

  test('transport例外とaccepted errorを観測Client例外へ露出しない', () async {
    const marker = 'secret-marker C:/private/observation trace';
    final transportFailure = _FakeObservationTransport(
      transportError: StateError(marker),
    );

    await expectLater(
      () => ObservationClient(transportFailure).list(),
      throwsA(
        isA<BrokerClientException>().having(
          (error) => error.toString(),
          'sanitized error',
          isNot(contains(marker)),
        ),
      ),
    );

    final acceptedError = _FakeObservationTransport()
      ..response = {
        'status': 'accepted',
        'error': {'message': marker},
        'body': _validBody(),
      };
    await expectLater(
      () => ObservationClient(acceptedError).list(),
      throwsA(
        isA<BrokerClientException>().having(
          (error) => error.toString(),
          'sanitized error',
          isNot(contains(marker)),
        ),
      ),
    );
  });
}
