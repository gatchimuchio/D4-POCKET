import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/screens/host_operation_center.dart';
import 'package:gui_shell_desktop/services/broker_client.dart';
import 'package:gui_shell_desktop/services/host_registration_client.dart';
import 'package:gui_shell_desktop/services/shell_core_client.dart';

const id = 'mac-host-fixture';
const hash =
    'sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';

Map<String, Object?> receipt() => {
      '版': 1,
      'Host ID': id,
      '表示名': 'Mac公開Host',
      'Platform': 'macos',
      '接続状態': 'pending_review',
      'Trust': {
        'state': 'pending_review',
        'evidence_source': 'INTERNAL_STATE',
        'requires_operator_review': true
      },
      '証明書/identity': {'種別': 'identity_hash', 'hash': hash},
      'Runtime summary': {
        'runtime_count': 2,
        'agent_count': 1,
        'evidence_source': 'INTERNAL_STATE'
      },
      '最終接続': null,
      '公開範囲': 'metadata_only',
      '証拠種別': 'INTERNAL_STATE',
      '権限生成': 'なし',
      'authority_strip': true,
      '能力ID': 'host.registry.register',
      '権限ID': 'permission.host.registry.register',
      '承認状態': 'owner_control_approved',
      '復旧ID': 'recover-host-registration',
      '登録監査ID': 'audit-host-register',
    };

class PublicHostBroker implements BrokerTransport {
  PublicHostBroker({this.reject = false, this.change});
  final bool reject;
  final void Function(Map<String, Object?>)? change;
  final operations = <String>[];
  Map<String, Object?>? registration;
  @override
  Future<Map<String, Object?>> request(String operation,
      {Map<String, Object?>? payload}) async {
    operations.add(operation);
    final row = receipt();
    change?.call(row);
    Object? body;
    if (operation == 'Host登録') {
      registration = payload;
      body = row;
    } else if (operation == 'Host一覧') {
      body = {
        '版': 1,
        'Host一覧': [row],
        '件数': 1,
        '公開範囲': 'metadata_only',
        '証拠種別': 'INTERNAL_STATE'
      };
    } else if (operation == 'Host切替') {
      body = {
        '選択Host ID': id,
        '接続状態': 'pending_review',
        'Trust': 'pending_review',
        '証拠種別': 'INTERNAL_STATE',
        '権限生成': 'なし',
        'authority_strip': true,
        '承認状態': 'not_reused',
        '監査ID': 'audit-host-switch'
      };
    }
    return {
      'operation': operation,
      'status': reject ? 'rejected' : 'accepted',
      'error': null,
      'evidence_source': 'INTERNAL_STATE',
      'audit_event_id':
          operation == 'Host登録' ? 'audit-host-register' : 'audit-host-list',
      'body': body
    };
  }
}

Future<String> register(PublicHostBroker broker) =>
    HostRegistrationClient(broker).register(
        hostId: id,
        displayName: 'Mac公開Host',
        platform: 'macos',
        identityHash: hash,
        runtimeCount: 2,
        agentCount: 1);

Future<void> enter(WidgetTester tester) async {
  await tester.tap(find.text('Hostを登録'));
  await tester.pumpAndSettle();
  for (final entry in {
    'Host識別子': id,
    'Host表示名': 'Mac公開Host',
    '公開identity hash': hash,
    '申告Runtime件数': '2',
    '申告Agent件数': '1'
  }.entries) {
    final field = find.widgetWithText(TextFormField, entry.key);
    await tester.ensureVisible(field);
    await tester.enterText(field, entry.value);
  }
  await tester.tap(find.text('Owner確認して登録'));
  await tester.pumpAndSettle();
}

void main() {
  test('Mac Hostだけがnative確認の有限待機を使い一覧・切替は通常待機', () {
    debugDefaultTargetPlatformOverride = TargetPlatform.macOS;
    addTearDown(() => debugDefaultTargetPlatformOverride = null);
    expect(brokerRequestTimeoutForOperation('Host登録'),
        const Duration(seconds: 305));
    expect(
        brokerRequestTimeoutForOperation('Host一覧'), const Duration(seconds: 5));
    expect(
        brokerRequestTimeoutForOperation('Host切替'), const Duration(seconds: 5));
  });
  test('公開Host要求と同一receiptだけを受理し未知件数を0へ補完しない', () async {
    final broker = PublicHostBroker();
    expect(await register(broker), 'audit-host-register');
    expect(broker.registration!.keys.toSet(), {
      '版',
      '操作',
      'Host ID',
      '表示名',
      'Platform',
      '接続状態',
      'Trust',
      '証明書/identity',
      'Runtime summary',
      '最終接続'
    });
    final rows = await HostRegistrationClient(broker).refresh();
    expect(rows.single.runtimeCount, 2);
    expect(rows.single.agentCount, 1);
    for (final change in <void Function(Map<String, Object?>)>[
      (r) => r['Host ID'] = 'wrong-host',
      (r) => r['権限生成'] = 'generated',
      (r) => (r['Runtime summary'] as Map).remove('runtime_count'),
      (r) => r['登録監査ID'] = 'other-audit',
    ]) {
      await expectLater(register(PublicHostBroker(change: change)),
          throwsA(isA<BrokerClientException>()));
    }
    await expectLater(register(PublicHostBroker(reject: true)),
        throwsA(isA<BrokerClientException>()));
  });
  testWidgets('Mac公開Host登録後に一覧更新し起動後のHostへ表示切替する', (tester) async {
    final broker = PublicHostBroker();
    await tester.pumpWidget(MaterialApp(
        home: Scaffold(
            body: HostOperationCenter(
                client: ShellCoreClient.mock(transport: broker)))));
    await enter(tester);
    expect(broker.operations, ['Host登録', 'Host一覧']);
    expect(find.textContaining('Host metadataを登録しました。未審査のままです。Audit='),
        findsWidgets);
    final switchButton = find.widgetWithText(FilledButton, 'Host切替');
    await tester.ensureVisible(switchButton);
    await tester.tap(switchButton);
    await tester.pumpAndSettle();
    expect(broker.operations.last, 'Host切替');
    expect(find.textContaining('承認=not_reused / 権限生成=なし'), findsWidgets);
  }, variant: TargetPlatformVariant.only(TargetPlatform.macOS));
  testWidgets('native拒否をHost登録成功にしない・自動再送しない', (tester) async {
    final broker = PublicHostBroker(reject: true);
    await tester.pumpWidget(MaterialApp(
        home: Scaffold(
            body: HostOperationCenter(
                client: ShellCoreClient.mock(transport: broker)))));
    await enter(tester);
    expect(broker.operations, ['Host登録']);
    expect(find.textContaining('Host登録または一覧更新は未成立です。'), findsWidgets);
    expect(find.textContaining('Host metadataを登録しました。'), findsNothing);
  }, variant: TargetPlatformVariant.only(TargetPlatform.macOS));
}
