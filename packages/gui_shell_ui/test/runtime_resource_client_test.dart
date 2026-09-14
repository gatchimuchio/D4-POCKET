import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_ui/runtime_resource_client.dart';

class _Fixture implements BrokerTransport {
  _Fixture(this.response);

  Map<String, Object?> response;
  final calls = <String>[];
  final payloads = <Map<String, Object?>>[];

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    calls.add(operation);
    payloads.add(Map<String, Object?>.from(payload ?? const {}));
    return Map<String, Object?>.from(response);
  }
}

Map<String, Object?> _measured(num value, {String source = 'LIVE_RUNTIME'}) =>
    {'状態': 'measured', '値': value, '証拠種別': source};

Map<String, Object?> _unknown(
  String reason, {
  String source = 'INTERNAL_STATE',
}) =>
    {'状態': 'unknown', '値': null, '証拠種別': source, '理由': reason};

Map<String, Object?> _metrics({bool allUnknown = false}) {
  final values = <String, Object?>{
    '稼働時間Millis': _measured(2500),
    'CPU累積時間Millis': _measured(120),
    'CPU利用率Percent': _measured(12.5),
    'RAMWorkingSetBytes': _measured(2048),
    'RAMPrivateBytes': _measured(1024),
    'DiskIOBytes': _unknown('個別Disk I/Oは未対応'),
    'NetworkIOBytes': _unknown('個別Network I/Oは未対応'),
    'GPU利用率Percent': _unknown('GPU観測は未対応'),
    'VRAMBytes': _unknown('VRAM観測は未対応'),
    '処理中要求数': _measured(1, source: 'INTERNAL_STATE'),
    '平均応答Millis': _unknown('対話の応答時間をミリ秒精度で測定していません'),
    '失敗要求数': _measured(0, source: 'INTERNAL_STATE'),
  };
  if (allUnknown) {
    for (final key in values.keys.toList(growable: false)) {
      values[key] = _unknown('結合を確認できません');
    }
  }
  return values;
}

Map<String, Object?> _history() => {
      '観測時刻UnixMillis': 1700000000100,
      'CPU利用率Percent': _measured(12.5),
      'RAMWorkingSetBytes': _measured(2048),
      'NetworkIOBytes': _unknown('個別Network I/Oは未対応'),
      '平均応答Millis': _unknown('対話の応答時間をミリ秒精度で測定していません'),
      'エラー率Percent': _measured(0, source: 'INTERNAL_STATE'),
    };

Map<String, Object?> _body({
  String bindingStatus = 'bound',
  bool allUnknown = false,
  List<Object?>? history,
}) {
  final bound = bindingStatus == 'bound';
  return {
    '版': 1,
    '実行系ID': 'local',
    '観測時刻UnixMillis': 1700000000200,
    '観測監査ID': 'audit-1',
    '結合': {
      '状態': bindingStatus,
      '根拠': bound ? 'loopback_tcp_listener_owner_pid' : 'unsupported',
      'PID': bound ? 4242 : null,
      'PID作成時刻UnixMillis': bound ? 1700000000000 : null,
      '登録時刻UnixMillis': 1700000000000,
      '登録監査ID': 'registration-audit-1',
      if (!bound) '理由': '現在のhostでは観測できません',
    },
    '統治': {
      '能力ID': 'runtime.resource.observe',
      '権限ID': 'permission.runtime.resource.observe',
      '承認状態': 'not_required',
      '復旧ID': 'recover-runtime-resource-binding',
    },
    '計測': _metrics(allUnknown: allUnknown),
    '短期履歴': history ?? (bound ? [_history()] : <Object?>[]),
  };
}

Map<String, Object?> _response(
  Map<String, Object?> body, {
  String evidenceSource = 'LIVE_RUNTIME',
}) =>
    {
      'request_id': 'fixture-request-1',
      'operation': '実行系資源観測',
      'status': 'accepted',
      'evidence_source': evidenceSource,
      'audit_event_id': 'audit-1',
      'error': null,
      'health': null,
      'body': body,
      'shutdown_requested': false,
    };

void main() {
  test('資源観測はPIDでなく実行系IDだけを送り、監査付きの実測を返す',
      () async {
    final fixture = _Fixture(_response(_body()));
    final observation = await RuntimeResourceClient(fixture).observe('local');

    expect(fixture.calls, ['実行系資源観測']);
    expect(fixture.payloads, [
      {'版': 1, '実行系ID': 'local'},
    ]);
    expect(observation.binding.isBound, isTrue);
    expect(observation.binding.pid, 4242);
    expect(observation.auditId, 'audit-1');
    expect(observation.metric('RAMWorkingSetBytes').value, 2048);
    expect(observation.metric('DiskIOBytes').isMeasured, isFalse);
    expect(observation.metric('DiskIOBytes').reason, '個別Disk I/Oは未対応');
    expect(observation.history, hasLength(1));
  });

  test('結合済みでも安全な数値を作れないLIVE_RUNTIME metricはunknownで返す',
      () async {
    final body = _body();
    final metrics = body['計測'] as Map<String, Object?>;
    for (final key in const <String>[
      '稼働時間Millis',
      'CPU累積時間Millis',
      'CPU利用率Percent',
    ]) {
      metrics[key] = _unknown('安全な数値精度を確保できません', source: 'LIVE_RUNTIME');
    }

    final observation = await RuntimeResourceClient(_Fixture(_response(body)))
        .observe('local');
    for (final key in const <String>[
      '稼働時間Millis',
      'CPU累積時間Millis',
      'CPU利用率Percent',
    ]) {
      expect(observation.metric(key).isMeasured, isFalse);
      expect(observation.metric(key).evidenceSource, 'LIVE_RUNTIME');
    }
  });

  test('unknownを数値に置換した応答、余分なfield、監査不一致を拒否する',
      () async {
    final invalidMetric = _body();
    (invalidMetric['計測'] as Map<String, Object?>)['DiskIOBytes'] = {
      '状態': 'unknown',
      '値': 0,
      '証拠種別': 'INTERNAL_STATE',
      '理由': '不正な0置換',
    };
    final fixture = _Fixture(_response(invalidMetric));
    final client = RuntimeResourceClient(fixture);
    await expectLater(client.observe('local'), throwsA(isA<BrokerClientException>()));

    fixture.response = _response(_body())..['余分'] = true;
    await expectLater(client.observe('local'), throwsA(isA<BrokerClientException>()));

    final auditMismatch = _body()..['観測監査ID'] = 'other-audit';
    fixture.response = _response(auditMismatch);
    await expectLater(client.observe('local'), throwsA(isA<BrokerClientException>()));

    fixture.response = _response(_body())..['audit_event_id'] = 1;
    await expectLater(client.observe('local'), throwsA(isA<BrokerClientException>()));
  });

  test('非結合は全metric unknownと空履歴、INTERNAL_STATEだけを許可する',
      () async {
    final body = _body(bindingStatus: 'unsupported', allUnknown: true);
    final fixture = _Fixture(_response(body, evidenceSource: 'INTERNAL_STATE'));
    final observation = await RuntimeResourceClient(fixture).observe('local');
    expect(observation.binding.status, 'unsupported');
    expect(observation.history, isEmpty);
    expect(observation.metrics.values.every((item) => !item.isMeasured), isTrue);

    final invalid = _body(bindingStatus: 'unsupported', allUnknown: true);
    (invalid['計測'] as Map<String, Object?>)['処理中要求数'] =
        _measured(0, source: 'INTERNAL_STATE');
    fixture.response = _response(invalid, evidenceSource: 'INTERNAL_STATE');
    await expectLater(
      RuntimeResourceClient(fixture).observe('local'),
      throwsA(isA<BrokerClientException>()),
    );

    fixture.response = _response(body);
    await expectLater(
      RuntimeResourceClient(fixture).observe('local'),
      throwsA(isA<BrokerClientException>()),
    );
  });

  test('C3で未接続のmetricと証拠種別の組合せを厳格に拒否する', () async {
    final fixture = _Fixture(_response(_body()));

    for (final key in const <String>[
      'DiskIOBytes',
      'NetworkIOBytes',
      'GPU利用率Percent',
      'VRAMBytes',
      '平均応答Millis',
    ]) {
      final measured = _body();
      (measured['計測'] as Map<String, Object?>)[key] =
          _measured(1, source: 'INTERNAL_STATE');
      fixture.response = _response(measured);
      await expectLater(
        RuntimeResourceClient(fixture).observe('local'),
        throwsA(isA<BrokerClientException>()),
      );

      final wrongSource = _body();
      (wrongSource['計測'] as Map<String, Object?>)[key] =
          _unknown('未接続の根拠', source: 'LIVE_RUNTIME');
      fixture.response = _response(wrongSource);
      await expectLater(
        RuntimeResourceClient(fixture).observe('local'),
        throwsA(isA<BrokerClientException>()),
      );
    }

    for (final key in const <String>[
      '稼働時間Millis',
      'CPU累積時間Millis',
      'CPU利用率Percent',
      'RAMWorkingSetBytes',
      'RAMPrivateBytes',
    ]) {
      final wrongSource = _body();
      (wrongSource['計測'] as Map<String, Object?>)[key] =
          _measured(1, source: 'INTERNAL_STATE');
      fixture.response = _response(wrongSource);
      await expectLater(
        RuntimeResourceClient(fixture).observe('local'),
        throwsA(isA<BrokerClientException>()),
      );
    }
    for (final key in const <String>['処理中要求数', '失敗要求数']) {
      final wrongSource = _body();
      (wrongSource['計測'] as Map<String, Object?>)[key] =
          _measured(1, source: 'LIVE_RUNTIME');
      fixture.response = _response(wrongSource);
      await expectLater(
        RuntimeResourceClient(fixture).observe('local'),
        throwsA(isA<BrokerClientException>()),
      );
    }

    final invalidHistory = _body();
    final history = (invalidHistory['短期履歴'] as List<Object?>).single
        as Map<String, Object?>;
    history['平均応答Millis'] = _measured(1, source: 'INTERNAL_STATE');
    fixture.response = _response(invalidHistory);
    await expectLater(
      RuntimeResourceClient(fixture).observe('local'),
      throwsA(isA<BrokerClientException>()),
    );

    final unboundWrongSource =
        _body(bindingStatus: 'unsupported', allUnknown: true);
    (unboundWrongSource['計測'] as Map<String, Object?>)['CPU利用率Percent'] =
        _unknown('未結合', source: 'LIVE_RUNTIME');
    fixture.response = _response(
      unboundWrongSource,
      evidenceSource: 'INTERNAL_STATE',
    );
    await expectLater(
      RuntimeResourceClient(fixture).observe('local'),
      throwsA(isA<BrokerClientException>()),
    );
  });

  test('実行系ID以外の入力を拒否し、Brokerへ要求しない', () async {
    final fixture = _Fixture(_response(_body()));
    final client = RuntimeResourceClient(fixture);
    for (final value in ['', 'PID=4242', 'local endpoint', 'a' * 129]) {
      await expectLater(
        client.observe(value),
        throwsA(isA<BrokerClientException>()),
      );
    }
    expect(fixture.calls, isEmpty);
  });
}
