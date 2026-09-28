import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_mobile/screens/resource_overview.dart';
import 'package:gui_shell_mobile/services/device_link_controller.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart';

class _Transport implements BrokerTransport {
  final calls = <String>[];
  Completer<void>? gate;

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    final runtime = payload?['実行系ID'] as String;
    calls.add(runtime);
    if (gate != null) await gate!.future;
    return _response(runtime);
  }
}

class _Controller extends DeviceLinkController {
  _Controller(this.transport);

  final _Transport transport;

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) => transport.request(operation, payload: payload);
}

Map<String, Object?> _measured(num value, {String source = 'LIVE_RUNTIME'}) => {
  '状態': 'measured',
  '値': value,
  '証拠種別': source,
};

Map<String, Object?> _unknown(String reason) => {
  '状態': 'unknown',
  '値': null,
  '証拠種別': 'INTERNAL_STATE',
  '理由': reason,
};

Map<String, Object?> _response(String runtime) {
  final auditId = 'resource-audit-$runtime';
  final metrics = <String, Object?>{
    '稼働時間Millis': _measured(2000),
    'CPU累積時間Millis': _measured(100),
    'CPU利用率Percent': _measured(12.5),
    'RAMWorkingSetBytes': _measured(4096),
    'RAMPrivateBytes': _measured(2048),
    'DiskIOBytes': _unknown('個別Disk I/Oは未対応'),
    'NetworkIOBytes': _unknown('個別Network I/Oは未対応'),
    'GPU利用率Percent': _unknown('GPU観測は未対応'),
    'VRAMBytes': _unknown('VRAM観測は未対応'),
    '処理中要求数': _measured(0, source: 'INTERNAL_STATE'),
    '平均応答Millis': _unknown('対話応答をミリ秒精度で測定していません'),
    '失敗要求数': _measured(0, source: 'INTERNAL_STATE'),
  };
  return {
    'request_id': 'resource-request-$runtime',
    'operation': '実行系資源観測',
    'status': 'accepted',
    'evidence_source': 'LIVE_RUNTIME',
    'audit_event_id': auditId,
    'error': null,
    'health': null,
    'shutdown_requested': false,
    'body': {
      '版': 1,
      '実行系ID': runtime,
      '観測時刻UnixMillis': 1700000001000,
      '観測監査ID': auditId,
      '結合': {
        '状態': 'bound',
        '根拠': 'loopback_tcp_listener_owner_pid',
        'PID': 4242,
        'PID作成時刻UnixMillis': 1700000000000,
        '登録時刻UnixMillis': 1700000000000,
        '登録監査ID': 'registration-audit-$runtime',
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
          'CPU利用率Percent': _measured(12.5),
          'RAMWorkingSetBytes': _measured(4096),
          'NetworkIOBytes': _unknown('個別Network I/Oは未対応'),
          '平均応答Millis': _unknown('対話応答をミリ秒精度で測定していません'),
          'エラー率Percent': _measured(0, source: 'INTERNAL_STATE'),
        },
      ],
    },
  };
}

void main() {
  testWidgets('資源観測は選択中だけ行い、離脱時に結果を破棄する', (tester) async {
    final transport = _Transport();
    final controller = _Controller(transport);
    addTearDown(controller.dispose);
    Widget page(bool active) => MaterialApp(
      home: Scaffold(
        body: ResourceOverview(
          controller: controller,
          runtimes: const ['local'],
          connected: true,
          active: active,
        ),
      ),
    );

    await tester.pumpWidget(page(false));
    await tester.pumpAndSettle();
    expect(transport.calls, isEmpty);

    await tester.pumpWidget(page(true));
    await tester.pumpAndSettle();
    expect(transport.calls, ['local']);
    expect(find.textContaining('CPU: 12.5'), findsOneWidget);

    await tester.pumpWidget(page(false));
    await tester.pumpAndSettle();
    expect(transport.calls, ['local']);
    expect(find.textContaining('CPU: 12.5'), findsNothing);

    await tester.pumpWidget(page(true));
    await tester.pumpAndSettle();
    expect(transport.calls, ['local', 'local']);
    expect(find.textContaining('CPU: 12.5'), findsOneWidget);
  });

  testWidgets('離脱中の応答を採用せず、後続Runtimeを観測しない', (tester) async {
    final transport = _Transport()..gate = Completer<void>();
    final controller = _Controller(transport);
    addTearDown(controller.dispose);
    Widget page(bool active) => MaterialApp(
      home: Scaffold(
        body: ResourceOverview(
          controller: controller,
          runtimes: const ['local', 'other'],
          connected: true,
          active: active,
        ),
      ),
    );

    await tester.pumpWidget(page(true));
    await tester.pump();
    expect(transport.calls, ['local']);

    await tester.pumpWidget(page(false));
    await tester.pumpAndSettle();
    transport.gate!.complete();
    await tester.pumpAndSettle();

    expect(transport.calls, ['local']);
    expect(find.textContaining('CPU: 12.5'), findsNothing);
  });
}
