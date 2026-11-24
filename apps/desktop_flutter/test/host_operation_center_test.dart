import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/models/generated_contracts.dart';
import 'package:gui_shell_desktop/screens/host_operation_center.dart';
import 'package:gui_shell_desktop/services/shell_core_client.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart' show BrokerTransport;

class _HostSwitchTransport implements BrokerTransport {
  Completer<void>? switchGate;
  bool malformedStopReceipt = false;
  final operations = <String>[];
  final requests = <Map<String, Object?>>[];

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    operations.add(operation);
    requests.add(Map<String, Object?>.from(payload ?? const {}));
    if (operation == 'Host切替') {
      final gate = switchGate;
      if (gate != null) await gate.future;
      final hostId = payload?['Host ID'] as String? ?? '';
      return {
        'operation': operation,
        'status': 'accepted',
        'error': null,
        'evidence_source': 'INTERNAL_STATE',
        'audit_event_id': 'audit-host-switch-$hostId',
        'body': {
          '選択Host ID': hostId,
          '接続状態': 'pending_review',
          'Trust': 'pending_review',
          '証拠種別': 'INTERNAL_STATE',
          '権限生成': 'なし',
          'authority_strip': true,
          '承認状態': 'not_reused',
          '監査ID': 'audit-host-switch-$hostId',
        },
      };
    }
    if (operation == 'Host停止') {
      final hostId = payload?['Host ID'] as String? ?? '';
      final auditId = 'audit-host-stop-$hostId';
      return {
        'operation': operation,
        'status': 'accepted',
        'error': null,
        'evidence_source': 'INTERNAL_STATE',
        'audit_event_id': auditId,
        'body': {
          '版': 1,
          '操作': 'Host停止',
          '停止対象Host ID': malformedStopReceipt ? 'host-wrong' : hostId,
          '停止範囲': 'display_context_only',
          '接続状態': 'pending_review',
          'Trust': 'pending_review',
          '公開範囲': 'metadata_only',
          '証拠種別': 'INTERNAL_STATE',
          '権限生成': 'なし',
          'authority_strip': true,
          '承認状態': 'not_reused',
          '再利用禁止': ['Permission', 'Approval', 'Authority'],
          '遠隔Host停止': false,
          '復旧方法': 'Host一覧から再選択',
          '監査ID': auditId,
        },
      };
    }
    throw StateError('unexpected operation');
  }
}

HostRegistryRecord _host(String id, String name) => HostRegistryRecord(
      hostId: id,
      displayName: name,
      platform: 'windows',
      connectionState: 'pending_review',
      trustState: 'pending_review',
      runtimeCount: 0,
      agentCount: 0,
      evidenceSource: 'INTERNAL_STATE',
      visibility: 'metadata_only',
      authorityStrip: true,
    );

ShellCoreClient _client(
  _HostSwitchTransport transport,
  List<HostRegistryRecord> hosts,
) {
  final base = ShellCoreClient.mock(transport: transport);
  return ShellCoreClient.mock(
    transport: transport,
    snapshot: base.getSnapshot().copyWith(hosts: hosts),
  );
}

Widget _screen(
  ShellCoreClient client, {
  VoidCallback? onHostContextChanged,
}) =>
    MaterialApp(
      home: Scaffold(
        body: HostOperationCenter(
          client: client,
          onHostContextChanged: onHostContextChanged,
        ),
      ),
    );

void main() {
  testWidgets('登録ダイアログ中のBroker切替で入力を新Brokerへ送らない', (tester) async {
    final first = _HostSwitchTransport();
    final second = _HostSwitchTransport();
    var client = _client(first, [_host('host-a', 'Host A')]);
    late StateSetter rebuild;

    await tester.pumpWidget(MaterialApp(
      home: StatefulBuilder(
        builder: (context, setState) {
          rebuild = setState;
          return Scaffold(body: HostOperationCenter(client: client));
        },
      ),
    ));
    await tester.tap(find.text('Hostを登録'));
    await tester.pumpAndSettle();
    for (final entry in {
      'Host識別子': 'host-a-new',
      'Host表示名': 'Host A 新規',
      '公開identity hash': 'sha256:${'a' * 64}',
      '申告Runtime件数': '0',
      '申告Agent件数': '0',
    }.entries) {
      final field = find.widgetWithText(TextFormField, entry.key);
      await tester.ensureVisible(field);
      await tester.enterText(field, entry.value);
    }

    rebuild(() {
      client = _client(second, [_host('host-b', 'Host B')]);
    });
    await tester.pump();
    await tester.tap(find.text('Owner確認して登録'));
    await tester.pumpAndSettle();

    expect(first.operations, isEmpty);
    expect(second.operations, isEmpty);
    expect(find.text('Host B'), findsWidgets);
    expect(find.text('Host A 新規'), findsNothing);
  }, variant: TargetPlatformVariant.only(TargetPlatform.linux));

  testWidgets('Broker受理後にHost文脈変更を通知する', (tester) async {
    final transport = _HostSwitchTransport();
    var notifications = 0;
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
        body: HostOperationCenter(
          client: _client(transport, [
            _host('host-a-current', 'Host A 現在'),
            _host('host-a-target', 'Host A 切替先'),
          ]),
          onHostContextChanged: () => notifications++,
        ),
      ),
    ));

    await tester.tap(find.widgetWithText(ListTile, 'Host A 切替先').first);
    await tester.pump();
    final switchButton = find.widgetWithText(FilledButton, 'Host切替');
    await tester.ensureVisible(switchButton);
    await tester.tap(switchButton);
    await tester.pumpAndSettle();

    expect(transport.operations, ['Host切替']);
    expect(transport.requests.single['Host ID'], 'host-a-target');
    expect(notifications, 1);
    expect(find.textContaining('権限生成=なし'), findsOneWidget);
  });

  testWidgets('Host停止はBroker receiptを検証してD4の選択文脈だけを解除する', (tester) async {
    final transport = _HostSwitchTransport();
    var notifications = 0;
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
        body: HostOperationCenter(
          client: _client(transport, [
            _host('host-a-current', 'Host A 現在'),
            _host('host-a-target', 'Host A 切替先'),
          ]),
          onHostContextChanged: () => notifications++,
        ),
      ),
    ));

    await tester.tap(find.widgetWithText(ListTile, 'Host A 切替先').first);
    await tester.pump();
    final switchButton = find.widgetWithText(FilledButton, 'Host切替');
    await tester.ensureVisible(switchButton);
    await tester.tap(switchButton);
    await tester.pumpAndSettle();
    await tester.tap(find.text('表示コンテキスト停止'));
    await tester.pumpAndSettle();

    expect(transport.operations, ['Host切替', 'Host停止']);
    expect(transport.requests.last, {'版': 1, 'Host ID': 'host-a-target'});
    expect(notifications, 2);
    expect(
      find.textContaining('遠隔Host／Runtimeは停止していません'),
      findsOneWidget,
    );
    expect(find.text('表示コンテキスト停止'), findsNothing);
  });

  testWidgets('Host停止receipt不一致では選択文脈を解除しない', (tester) async {
    final transport = _HostSwitchTransport()..malformedStopReceipt = true;
    var notifications = 0;
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
        body: HostOperationCenter(
          client: _client(transport, [
            _host('host-a-current', 'Host A 現在'),
            _host('host-a-target', 'Host A 切替先'),
          ]),
          onHostContextChanged: () => notifications++,
        ),
      ),
    ));

    await tester.tap(find.widgetWithText(ListTile, 'Host A 切替先').first);
    await tester.pump();
    final switchButton = find.widgetWithText(FilledButton, 'Host切替');
    await tester.ensureVisible(switchButton);
    await tester.tap(switchButton);
    await tester.pumpAndSettle();
    await tester.tap(find.text('表示コンテキスト停止'));
    await tester.pumpAndSettle();

    expect(transport.operations, ['Host切替', 'Host停止']);
    expect(notifications, 1);
    expect(find.text('表示コンテキスト停止'), findsOneWidget);
    expect(
      find.textContaining('停止結果を確認できません'),
      findsOneWidget,
    );
  });

  testWidgets('Host操作画面はBroker切替後に旧Hostと遅延切替結果を破棄する', (tester) async {
    final oldRequest = Completer<void>();
    final first = _HostSwitchTransport()..switchGate = oldRequest;
    final second = _HostSwitchTransport();
    final oldCurrent = _host('host-a-current', 'Host A 現在');
    final oldCandidate = _host('host-a-candidate', 'Host A 切替先');
    final current = _host('host-b-current', 'Host B 現在');
    var notifications = 0;
    void onHostContextChanged() => notifications++;

    await tester.pumpWidget(_screen(
      _client(first, [oldCurrent, oldCandidate]),
      onHostContextChanged: onHostContextChanged,
    ));
    expect(find.text('Host A 現在'), findsWidgets);
    expect(find.text('Host A 切替先'), findsWidgets);

    await tester.tap(find.widgetWithText(ListTile, 'Host A 切替先').first);
    await tester.pump();
    final switchButton = find.widgetWithText(FilledButton, 'Host切替');
    await tester.ensureVisible(switchButton);
    await tester.tap(switchButton);
    await tester.pump();
    expect(first.operations, ['Host切替']);
    expect(first.requests.single['Host ID'], 'host-a-candidate');

    await tester.pumpWidget(_screen(
      _client(second, [current]),
      onHostContextChanged: onHostContextChanged,
    ));
    await tester.pumpAndSettle();
    expect(find.text('Host B 現在'), findsWidgets);
    expect(find.text('Host A 現在'), findsNothing);
    expect(find.text('Host A 切替先'), findsNothing);
    final activeButton = tester.widget<FilledButton>(
      find.widgetWithText(FilledButton, '現在のHost'),
    );
    expect(activeButton.onPressed, isNull);
    expect(second.operations, isEmpty);

    oldRequest.complete();
    await tester.pumpAndSettle();
    expect(find.text('Host B 現在'), findsWidgets);
    expect(find.text('Host A 現在'), findsNothing);
    expect(find.text('Host A 切替先'), findsNothing);
    expect(find.textContaining('Host切替を監査しました。'), findsNothing);
    expect(second.operations, isEmpty);
    expect(notifications, 0);
  }, variant: TargetPlatformVariant.only(TargetPlatform.windows));
}
