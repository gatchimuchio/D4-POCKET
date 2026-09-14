import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_ui/runtime_lifecycle_client.dart';

class _Fixture implements BrokerTransport {
  final calls = <String>[];
  final payloads = <Map<String, Object?>>[];
  Map<String, Object?> response = const {};

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

Map<String, Object?> _response(
  String operation,
  String evidenceSource,
  String auditId,
  Map<String, Object?> body,
) =>
    {
      'request_id': 'lifecycle-request-1',
      'operation': operation,
      'status': 'accepted',
      'evidence_source': evidenceSource,
      'audit_event_id': auditId,
      'error': null,
      'health': null,
      'body': body,
      'shutdown_requested': false,
    };

Map<String, Object?> _operation({bool executable = true}) => {
      '操作': 'start',
      '能力ID': 'runtime.lifecycle.start',
      '権限ID': 'permission.runtime.lifecycle.start',
      '承認必要': true,
      '復旧ID': 'recover-runtime-lifecycle-start',
      '実行可能': executable,
    };

Map<String, Object?> _governance(String approvalId, String approvalState) => {
      '能力ID': 'runtime.lifecycle.start',
      '権限ID': 'permission.runtime.lifecycle.start',
      '承認ID': approvalId,
      '承認状態': approvalState,
      '復旧ID': 'recover-runtime-lifecycle-start',
    };

Map<String, Object?> _approval(String state) => {
      '版': 1,
      '承認ID': 'lifecycle-approval-1',
      '承認hash':
          'sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
      '実行系ID': 'fixture-runtime',
      '操作': 'start',
      '状態': state,
      '有効期限UnixSeconds': 4102444800,
      '統治': _governance('lifecycle-approval-1', state),
    };

Map<String, Object?> _status({List<Object?> approvals = const []}) => {
      '版': 1,
      '実行系ID': 'fixture-runtime',
      '対応': true,
      '状態': 'stopped',
      '証拠種別': 'LIVE_RUNTIME',
      '操作一覧': [_operation()],
      '承認一覧': approvals,
    };

void main() {
  test('lifecycle clientは統治IDを入力せず、Brokerが返した承認だけで一回遷移を要求する', () async {
    final fixture = _Fixture()
      ..response = _response(
        '実行系ライフサイクル状態',
        'LIVE_RUNTIME',
        'status-audit-1',
        _status(),
      );
    final client = RuntimeLifecycleClient(fixture);

    final initial = await client.status('fixture-runtime');
    expect(initial.supported, isTrue);
    expect(initial.operation('start')!.executable, isTrue);
    expect(initial.approvedFor('start'), isNull);

    fixture.response = _response(
      '実行系ライフサイクル承認要求',
      'INTERNAL_STATE',
      'approval-request-audit-1',
      _approval('pending'),
    );
    final pending = await client.requestApproval('fixture-runtime', 'start');
    expect(pending.state, 'pending');

    fixture.response = _response(
      '実行系ライフサイクル操作',
      'LIVE_RUNTIME',
      'transition-audit-1',
      {
        '版': 1,
        '実行系ID': 'fixture-runtime',
        '操作': 'start',
        '遷移前状態': 'stopped',
        '遷移後状態': 'ready',
        '観測時刻UnixMillis': 1700000000000,
        '証拠種別': 'LIVE_RUNTIME',
        '統治': _governance('lifecycle-approval-1', 'consumed'),
        'ライフサイクル監査ID': 'transition-audit-1',
      },
    );
    final transition = await client.execute(
      'fixture-runtime',
      'start',
      pending.approvalId,
    );

    expect(transition.nextState, 'ready');
    expect(transition.auditId, 'transition-audit-1');
    expect(fixture.calls, [
      '実行系ライフサイクル状態',
      '実行系ライフサイクル承認要求',
      '実行系ライフサイクル操作',
    ]);
    expect(fixture.payloads, [
      {'版': 1, '実行系ID': 'fixture-runtime'},
      {'版': 1, '実行系ID': 'fixture-runtime', '操作': 'start'},
      {
        '版': 1,
        '実行系ID': 'fixture-runtime',
        '操作': 'start',
        '承認ID': 'lifecycle-approval-1',
      },
    ]);
  });

  test('unsupported runtimeは操作を作らず、証拠種別の不一致を拒否する', () async {
    final fixture = _Fixture()
      ..response = _response(
        '実行系ライフサイクル状態',
        'INTERNAL_STATE',
        'status-audit-2',
        {
          '版': 1,
          '実行系ID': 'no-capability',
          '対応': false,
          '状態': 'not_supported',
          '証拠種別': 'INTERNAL_STATE',
          '操作一覧': const [],
          '承認一覧': const [],
        },
      );
    final status =
        await RuntimeLifecycleClient(fixture).status('no-capability');
    expect(status.operations, isEmpty);

    fixture.response = _response(
      '実行系ライフサイクル状態',
      'LIVE_RUNTIME',
      'status-audit-3',
      {..._status(), '証拠種別': 'INTERNAL_STATE'},
    );
    await expectLater(
      RuntimeLifecycleClient(fixture).status('fixture-runtime'),
      throwsA(isA<BrokerClientException>()),
    );
  });

  test('隔離済み実行系は操作も承認も持たない限定状態照会だけを受け入れる', () async {
    final fixture = _Fixture()
      ..response = _response(
        '実行系ライフサイクル状態',
        'LIVE_RUNTIME',
        'quarantine-status-audit-1',
        {
          '版': 1,
          '実行系ID': 'fixture-runtime',
          '対応': true,
          '状態': 'quarantined',
          '証拠種別': 'LIVE_RUNTIME',
          '操作一覧': const [],
          '承認一覧': const [],
        },
      );
    final status =
        await RuntimeLifecycleClient(fixture).status('fixture-runtime');
    expect(status.state, 'quarantined');
    expect(status.operations, isEmpty);
    expect(status.approvals, isEmpty);

    fixture.response = _response(
      '実行系ライフサイクル状態',
      'LIVE_RUNTIME',
      'quarantine-status-audit-2',
      {
        '版': 1,
        '実行系ID': 'fixture-runtime',
        '対応': true,
        '状態': 'quarantined',
        '証拠種別': 'LIVE_RUNTIME',
        '操作一覧': [_operation(executable: false)],
        '承認一覧': const [],
      },
    );
    await expectLater(
      RuntimeLifecycleClient(fixture).status('fixture-runtime'),
      throwsA(isA<BrokerClientException>()),
    );
  });
}
