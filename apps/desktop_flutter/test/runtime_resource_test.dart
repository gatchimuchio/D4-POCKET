import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/screens/runtime_center.dart';
import 'package:gui_shell_desktop/services/shell_core_client.dart';
import 'package:gui_shell_ui/runtime_resource_client.dart';

class _ResourceFixture implements BrokerTransport {
  _ResourceFixture({this.runtimeIds = const ['local']});

  final List<String> runtimeIds;
  final calls = <String>[];
  final payloads = <Map<String, Object?>>[];
  Completer<Map<String, Object?>>? delayedObservation;

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    calls.add(operation);
    final requestPayload = Map<String, Object?>.from(payload ?? const {});
    payloads.add(requestPayload);
    if (operation == '実行系列挙') {
      return {
        'request_id': 'runtime-catalog-request-1',
        'operation': operation,
        'status': 'accepted',
        'evidence_source': 'INTERNAL_STATE',
        'audit_event_id': 'runtime-catalog-audit-1',
        'error': null,
        'health': null,
        'body': {'実行系': runtimeIds},
        'shutdown_requested': false,
      };
    }
    if (operation == '実行系資源観測') {
      final delayed = delayedObservation;
      if (delayed != null) return await delayed.future;
      return _response(requestPayload['実行系ID'] as String);
    }
    throw StateError('このfixtureでは$operationを扱いません。');
  }
}

Map<String, Object?> _measured(num value, {String source = 'LIVE_RUNTIME'}) =>
    {'状態': 'measured', '値': value, '証拠種別': source};

Map<String, Object?> _unknown(String reason) =>
    {'状態': 'unknown', '値': null, '証拠種別': 'INTERNAL_STATE', '理由': reason};

Map<String, Object?> _response(String runtimeId) {
  final metrics = <String, Object?>{
    '稼働時間Millis': _measured(2000),
    'CPU累積時間Millis': _measured(100),
    'CPU利用率Percent': _measured(0.003),
    'RAMWorkingSetBytes': _measured(4096),
    'RAMPrivateBytes': _measured(2048),
    'DiskIOBytes': _unknown('個別Disk I/Oは未対応'),
    'NetworkIOBytes': _unknown('個別Network I/Oは未対応'),
    'GPU利用率Percent': _unknown('GPU観測は未対応'),
    'VRAMBytes': _unknown('VRAM観測は未対応'),
    '処理中要求数': _measured(0, source: 'INTERNAL_STATE'),
    '平均応答Millis': _unknown('対話の応答時間をミリ秒精度で測定していません'),
    '失敗要求数': _measured(0, source: 'INTERNAL_STATE'),
  };
  return {
    'request_id': 'resource-request-1',
    'operation': '実行系資源観測',
    'status': 'accepted',
    'evidence_source': 'LIVE_RUNTIME',
    'audit_event_id': 'resource-audit-1',
    'error': null,
    'health': null,
    'shutdown_requested': false,
    'body': {
      '版': 1,
      '実行系ID': runtimeId,
      '観測時刻UnixMillis': 1700000001000,
      '観測監査ID': 'resource-audit-1',
      '結合': {
        '状態': 'bound',
        '根拠': 'loopback_tcp_listener_owner_pid',
        'PID': 4242,
        'PID作成時刻UnixMillis': 1700000000000,
        '登録時刻UnixMillis': 1700000000000,
        '登録監査ID': 'resource-registration-audit-1',
      },
      '統治': {
        '能力ID': 'runtime.resource.observe',
        '権限ID': 'permission.runtime.resource.observe',
        '承認状態': 'not_required',
        '復旧ID': 'recover-runtime-resource-binding',
      },
      '計測': metrics,
      '短期履歴': [
        {
          '観測時刻UnixMillis': 1700000001000,
          'CPU利用率Percent': _measured(0.003),
          'RAMWorkingSetBytes': _measured(4096),
          'NetworkIOBytes': _unknown('個別Network I/Oは未対応'),
          '平均応答Millis': _unknown('対話の応答時間をミリ秒精度で測定していません'),
          'エラー率Percent': _measured(0, source: 'INTERNAL_STATE'),
        },
      ],
    },
  };
}

void main() {
  testWidgets('資源タブはBroker登録済みRuntimeに束縛した一回観測を表示する', (tester) async {
    final fixture = _ResourceFixture();
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: RuntimeCenter(
            client: ShellCoreClient.mock(transport: fixture),
            resourceClient: RuntimeResourceClient(fixture),
          ),
        ),
      ),
    );

    expect(fixture.calls, isEmpty);
    await tester.tap(find.text('資源'));
    await tester.pumpAndSettle();
    expect(fixture.calls, ['実行系列挙']);
    expect(find.byType(TextField), findsNothing);
    expect(find.byKey(const ValueKey('runtime-catalog-selection-resource')),
        findsOneWidget);
    expect(find.text('local'), findsOneWidget);
    await tester.tap(find.byKey(const ValueKey('runtime-resource-refresh')));
    await tester.pumpAndSettle();

    expect(fixture.calls, ['実行系列挙', '実行系資源観測']);
    expect(fixture.payloads, [
      <String, Object?>{},
      {'版': 1, '実行系ID': 'local'},
    ]);
    expect(find.text('PID: 4242'), findsOneWidget);
    expect(find.text('証拠: LIVE_RUNTIME'), findsWidgets);
    expect(find.text('理由: 個別Disk I/Oは未対応'), findsOneWidget);
    expect(find.text('0.003 %'), findsWidgets);
    expect(find.text('CPU利用率の短期履歴'), findsOneWidget);
    expect(find.text('RAMワーキングセットの短期履歴'), findsOneWidget);
  });

  testWidgets('Runtime切替後に完了した旧資源応答を表示しない', (tester) async {
    final fixture = _ResourceFixture(runtimeIds: ['local', 'other']);
    final delayedObservation = Completer<Map<String, Object?>>();
    fixture.delayedObservation = delayedObservation;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: RuntimeCenter(
            client: ShellCoreClient.mock(transport: fixture),
            resourceClient: RuntimeResourceClient(fixture),
          ),
        ),
      ),
    );

    await tester.tap(find.text('資源'));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('runtime-resource-refresh')));
    await tester.pump();
    await tester
        .tap(find.byKey(const ValueKey('runtime-catalog-selection-resource')));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 300));
    await tester.tap(find.text('other').last);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 300));

    delayedObservation.complete(_response('local'));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 300));

    expect(find.text('PID: 4242'), findsNothing);
    expect(find.text('Runtimeを切り替えました。現在の状態を再取得してください。'), findsOneWidget);
  });
}
