import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/screens/observability_center.dart';
import 'package:gui_shell_desktop/screens/trace_inspector.dart';
import 'package:gui_shell_desktop/services/shell_core_client.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart' show BrokerTransport;

class _ObservationTransport implements BrokerTransport {
  _ObservationTransport(this.label);

  final String label;
  final requests = <Map<String, Object?>>[];
  Completer<void>? nextObservationGate;

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    requests.add({
      'operation': operation,
      'payload': Map<String, Object?>.from(payload ?? const {}),
    });
    if (operation != '観測一覧') {
      throw StateError('unexpected operation');
    }
    final gate = nextObservationGate;
    nextObservationGate = null;
    if (gate != null) await gate.future;
    return {
      'status': 'accepted',
      'body': {
        '版': 1,
        'Span一覧': [
          {
            'TraceID': 'trace-$label',
            'SpanID': 'span-$label',
            '対象': 'Broker $label',
            '操作': 'fixture-observation',
            '状態': 'completed',
            '所要Millis': 12,
            '関連監査ID': 'audit-$label',
            '証拠種別': 'INTERNAL_STATE',
            '開始EpochMillis': 1700000000000,
            '終了EpochMillis': 1700000000012,
          },
        ],
        'Trace一覧': [
          {'TraceID': 'trace-$label'},
        ],
        'Metric一覧': [
          {
            '名称': 'Fixture Metric $label',
            '値': 3,
            '単位': 'count',
            '状態': 'measured',
          },
        ],
        '件数': 1,
        '証拠種別': 'INTERNAL_STATE',
        '観測範囲': 'broker_audit_finalization',
        'Auditとの責任分離': 'Audit=責任・安全・証拠; Observation=性能・挙動・運用状態',
        'OpenTelemetry export': 'unsupported',
        '権限生成': 'なし',
      },
    };
  }
}

Widget _observabilityScreen(_ObservationTransport transport) => MaterialApp(
      home: Scaffold(
        body: ObservabilityCenter(
          client: ShellCoreClient.mock(transport: transport),
        ),
      ),
    );

Widget _traceScreen(_ObservationTransport transport) => MaterialApp(
      home: Scaffold(
        body: TraceInspector(
          client: ShellCoreClient.mock(transport: transport),
        ),
      ),
    );

void main() {
  testWidgets('Observability CenterはBroker切替後に旧遅延応答を表示しない', (tester) async {
    final oldRequest = Completer<void>();
    final first = _ObservationTransport('A')..nextObservationGate = oldRequest;
    final second = _ObservationTransport('B');
    await tester.pumpWidget(_observabilityScreen(first));
    await tester.pump();
    expect(first.requests, hasLength(1));

    await tester.pumpWidget(_observabilityScreen(second));
    await tester.pumpAndSettle();
    expect(find.text('Broker B / fixture-observation'), findsOneWidget);
    expect(find.text('Broker A / fixture-observation'), findsNothing);

    oldRequest.complete();
    await tester.pumpAndSettle();
    expect(find.text('Broker B / fixture-observation'), findsOneWidget);
    expect(find.text('Broker A / fixture-observation'), findsNothing);
    expect(second.requests, hasLength(1));
  });

  testWidgets('Trace InspectorはBroker切替時にTrace選択と旧遅延応答を破棄する', (tester) async {
    final first = _ObservationTransport('A');
    final second = _ObservationTransport('B');
    await tester.pumpWidget(_traceScreen(first));
    await tester.pumpAndSettle();
    expect(find.text('Broker A / fixture-observation / completed'),
        findsOneWidget);

    await tester.tap(find.byType(DropdownButton<String?>));
    await tester.pumpAndSettle();
    await tester.tap(find.text('trace-A').last);
    await tester.pumpAndSettle();
    expect(first.requests.last['payload'], {
      '版': 1,
      '上限': 256,
      'TraceID': 'trace-A',
    });

    final oldRequest = Completer<void>();
    first.nextObservationGate = oldRequest;
    await tester.tap(find.text('更新'));
    await tester.pump();

    await tester.pumpWidget(_traceScreen(second));
    await tester.pumpAndSettle();
    expect(find.text('Broker B / fixture-observation / completed'),
        findsOneWidget);
    expect(
        find.text('Broker A / fixture-observation / completed'), findsNothing);
    expect(second.requests, hasLength(1));
    expect(second.requests.single['payload'], {'版': 1, '上限': 256});

    oldRequest.complete();
    await tester.pumpAndSettle();
    expect(find.text('Broker B / fixture-observation / completed'),
        findsOneWidget);
    expect(
        find.text('Broker A / fixture-observation / completed'), findsNothing);
  });
}
