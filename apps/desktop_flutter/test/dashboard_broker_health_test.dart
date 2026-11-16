import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/screens/dashboard.dart';
import 'package:gui_shell_desktop/services/shell_core_client.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart' show BrokerTransport;

class _HealthTransport implements BrokerTransport {
  _HealthTransport(this._respond);

  final Future<Map<String, Object?>> Function() _respond;
  final operations = <String>[];

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) {
    operations.add(operation);
    return _respond();
  }
}

Map<String, Object?> _healthResponse({
  String responseStatus = 'accepted',
  String healthStatus = 'ready',
  bool persistenceReady = true,
}) {
  final evidence =
      responseStatus == 'accepted' ? 'LIVE_RUNTIME' : 'INTERNAL_STATE';
  return {
    'request_id': 'dashboard-health',
    'operation': 'health',
    'status': responseStatus,
    'evidence_source': evidence,
    'audit_event_id': 'broker-audit-dashboard-health',
    'error': responseStatus == 'suspended'
        ? {
            'code': 'broker_persistence_unavailable',
            'message': 'private detail',
            'recoverable': true,
            'audit_event_required': true,
            'fail_closed': true,
          }
        : null,
    'health': {
      'broker_id': 'gui-shell-rust-broker',
      'status': healthStatus,
      'boundary_role': 'rust_security_broker_candidate',
      'authority_cutover_status': 'not_active',
      'command_dispatch_enabled': false,
      'audit_append_enabled': true,
      'audit_persistence': 'durable_file_store',
      'replay_persistence': 'durable_file_store',
      'session_persistence': 'durable_file_store',
      'persistence_required': true,
      'persistence_ready': persistenceReady,
      'evidence_source': evidence,
    },
    'body': null,
    'shutdown_requested': false,
  };
}

Widget _dashboard(ShellCoreClient client, {bool active = true}) => MaterialApp(
      home: Scaffold(
        body: Dashboard(
          key: const ValueKey('dashboard'),
          client: client,
          active: active,
        ),
      ),
    );

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('Dashboardはsnapshot値で代用せず現在Broker Healthを表示する', (tester) async {
    final transport = _HealthTransport(() async => _healthResponse());
    await tester
        .pumpWidget(_dashboard(ShellCoreClient.mock(transport: transport)));
    await tester.pumpAndSettle();

    expect(transport.operations, ['health']);
    expect(find.text('Broker状態'), findsOneWidget);
    // The Broker process reports ready, but command dispatch and authority
    // cutover remain disabled, so the operation state is suspended.
    expect(find.text('SUSPENDED'), findsOneWidget);
    expect(find.textContaining('権限切替: not_active'), findsOneWidget);
    expect(find.textContaining('Task dispatch: disabled'), findsOneWidget);
  });

  testWidgets('Brokerがない診断clientはUNSUPPORTEDとしsnapshotを現在値にしない',
      (tester) async {
    await tester.pumpWidget(_dashboard(ShellCoreClient.local()));
    await tester.pumpAndSettle();

    expect(find.text('UNSUPPORTED'), findsOneWidget);
    expect(find.textContaining('Broker状態'), findsOneWidget);
  });

  testWidgets('Dashboard非表示中はHealthを取得せず再表示時に一度取得する', (tester) async {
    final transport = _HealthTransport(() async => _healthResponse());
    final client = ShellCoreClient.mock(transport: transport);
    await tester.pumpWidget(_dashboard(client, active: false));
    expect(transport.operations, isEmpty);

    await tester.pumpWidget(_dashboard(client));
    await tester.pumpAndSettle();

    expect(transport.operations, ['health']);
    expect(find.text('SUSPENDED'), findsOneWidget);
  });

  testWidgets('Broker切替中の旧Generation Health応答を破棄する', (tester) async {
    final oldResponse = Completer<Map<String, Object?>>();
    final newResponse = Completer<Map<String, Object?>>();
    final oldTransport = _HealthTransport(() => oldResponse.future);
    final newTransport = _HealthTransport(() => newResponse.future);

    await tester.pumpWidget(
      _dashboard(ShellCoreClient.mock(transport: oldTransport)),
    );
    expect(oldTransport.operations, ['health']);

    await tester.pumpWidget(
      _dashboard(ShellCoreClient.mock(transport: newTransport)),
    );
    expect(newTransport.operations, isEmpty);

    oldResponse.complete(
      _healthResponse(healthStatus: 'degraded'),
    );
    await tester.pump();
    await tester.pump();
    expect(newTransport.operations, ['health']);
    expect(find.text('DEGRADED'), findsNothing);

    newResponse.complete(_healthResponse());
    await tester.pumpAndSettle();
    expect(find.text('SUSPENDED'), findsOneWidget);
    expect(find.text('DEGRADED'), findsNothing);
  });

  testWidgets('Broker Health失敗の詳細を隠し再取得後に現在値を表示する', (tester) async {
    var healthReads = 0;
    final transport = _HealthTransport(() async {
      healthReads++;
      if (healthReads == 1) throw StateError('SECRET_HEALTH_DETAIL');
      return _healthResponse();
    });
    await tester
        .pumpWidget(_dashboard(ShellCoreClient.mock(transport: transport)));
    await tester.pumpAndSettle();

    expect(find.text('RECOVERABLE_ERROR'), findsOneWidget);
    expect(find.textContaining('SECRET_HEALTH_DETAIL'), findsNothing);
    final retry = find.byKey(const ValueKey('dashboard-broker-health-retry'));
    expect(retry, findsOneWidget);
    await tester.ensureVisible(retry);
    await tester.pump();
    await tester.tap(retry);
    await tester.pumpAndSettle();

    expect(healthReads, 2);
    expect(find.text('SUSPENDED'), findsOneWidget);
    expect(find.textContaining('SECRET_HEALTH_DETAIL'), findsNothing);
  });
}
