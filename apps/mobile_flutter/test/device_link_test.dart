import 'dart:async';
import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_mobile/main.dart';
import 'package:gui_shell_mobile/screens/device_connection.dart';
import 'package:gui_shell_mobile/services/device_link_client.dart';
import 'package:gui_shell_mobile/services/device_link_controller.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart';

Map<String, Object?> data({bool invitation = false}) => {
  '版': 1,
  'HostID': 'a' * 32,
  '端末ID': 'b' * 32,
  '接続先Host': '127.0.0.1',
  'port': 12345,
  '証明書hash': 'c' * 64,
  '有効期限': DateTime.now().millisecondsSinceEpoch ~/ 1000 + 100,
  invitation ? '招待ID' : '結合ID': 'd' * 32,
  invitation ? '招待秘密' : '端末秘密': 'e' * 64,
};
DeviceCredential parse(Map<String, Object?> value, {bool invitation = false}) =>
    DeviceCredential.parse(
      jsonEncode(value),
      invitation: invitation,
      deviceId: 'b' * 32,
    );

class MemoryStore implements DeviceStore {
  final values = <String, String>{DeviceLinkController.deviceKey: 'b' * 32};
  bool failWrite = false;
  bool failRead = false;
  bool failDelete = false;
  bool retainOnDelete = false;
  bool failReadAfterDelete = false;
  Completer<void>? credentialReadWait;
  @override
  Future<String?> read(String key) async {
    if (key == DeviceLinkController.credentialKey && credentialReadWait != null)
      await credentialReadWait!.future;
    if (failRead) throw StateError('fixture');
    return values[key];
  }

  @override
  Future<void> write(String key, String value) async {
    if (failWrite) throw StateError('fixture');
    values[key] = value;
  }

  @override
  Future<void> delete(String key) async {
    if (failDelete) throw StateError('fixture');
    if (!retainOnDelete) values.remove(key);
    if (failReadAfterDelete) failRead = true;
  }
}

class LinkFixture extends DeviceLinkClient {
  LinkFixture(super.credential, this.calls);
  final List<String> calls;
  bool reject = false;
  Completer<void>? wait;
  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    if (!active || reject) throw const BrokerClientException('fixture拒否');
    calls.add(operation);
    if (wait != null) await wait!.future;
    if (!active) throw const BrokerClientException('fixture停止');
    return {
      'operation': operation,
      'audit_event_id': 'fixture',
      'status': 'accepted',
      'body': operation == '実行系列挙'
          ? {
              '実行系': ['left', 'right'],
            }
          : {'状態': '接続中'},
    };
  }

  @override
  Future<DeviceCredential> pair() async {
    calls.add('端末結合');
    return parse(data());
  }
}

void main() {
  for (final failure in [
    '削除',
    '破損',
    '秘密変更',
    'Host変更',
    '期限変更',
    '端末ID変更',
    '読取障害',
  ]) {
    test('復帰時の保存資格の$failureでは通信を再開しない', () async {
      final saved = data();
      final store = MemoryStore()
        ..values[DeviceLinkController.credentialKey] = jsonEncode(saved);
      final calls = <String>[];
      final c = DeviceLinkController(
        store: store,
        connect: (v) => LinkFixture(v, calls),
      );
      addTearDown(c.dispose);
      await c.initialize();
      expect(c.ready, isTrue);
      calls.clear();
      c.setForeground(false);
      switch (failure) {
        case '削除':
          store.values.remove(DeviceLinkController.credentialKey);
        case '破損':
          store.values[DeviceLinkController.credentialKey] = '{}';
        case '秘密変更':
          store.values[DeviceLinkController.credentialKey] = jsonEncode({
            ...saved,
            '端末秘密': 'f' * 64,
          });
        case 'Host変更':
          store.values[DeviceLinkController.credentialKey] = jsonEncode({
            ...saved,
            'HostID': 'f' * 32,
          });
        case '期限変更':
          store.values[DeviceLinkController.credentialKey] = jsonEncode({
            ...saved,
            '有効期限': (saved['有効期限'] as int) + 10,
          });
        case '端末ID変更':
          store.values[DeviceLinkController.deviceKey] = 'f' * 32;
        case '読取障害':
          store.failRead = true;
      }
      c.setForeground(true);
      await Future<void>.delayed(Duration.zero);
      expect(c.ready, isFalse);
      expect(calls, isEmpty);
      await expectLater(
        c.request('対話送信'),
        throwsA(isA<BrokerClientException>()),
      );
    });
  }
  test('保存資格の読取中にbackgroundへ移ったら通信しない', () async {
    final store = MemoryStore()
      ..values[DeviceLinkController.credentialKey] = jsonEncode(data());
    final calls = <String>[];
    final c = DeviceLinkController(
      store: store,
      connect: (v) => LinkFixture(v, calls),
    );
    addTearDown(c.dispose);
    await c.initialize();
    calls.clear();
    final wait = Completer<void>();
    store.credentialReadWait = wait;
    final verifying = c.verify();
    await Future<void>.delayed(Duration.zero);
    c.setForeground(false);
    wait.complete();
    await verifying;
    expect(c.ready, isFalse);
    expect(calls, isEmpty);
    store.credentialReadWait = null;
    c.setForeground(true);
    await Future<void>.delayed(Duration.zero);
    expect(c.ready, isTrue);
    expect(calls, ['端末確認', '実行系列挙']);
  });
  testWidgets('Host照合の明示操作より前に招待を送らない', (tester) async {
    final calls = <String>[];
    final c = DeviceLinkController(
      store: MemoryStore(),
      connect: (v) => LinkFixture(v, calls),
    );
    await c.initialize();
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ListenableBuilder(
            listenable: c,
            builder: (_, _) => DeviceConnection(controller: c),
          ),
        ),
      ),
    );
    await tester.enterText(
      find.byType(TextField),
      jsonEncode(data(invitation: true)),
    );
    await tester.tap(find.text('招待を確認'));
    await tester.pumpAndSettle();
    expect(calls, isEmpty);
    final confirm = find.text('操作者確認: Desktopの表示と一致したので結合');
    await tester.ensureVisible(confirm);
    await tester.tap(confirm);
    await tester.pumpAndSettle();
    expect(c.ready, isTrue);
    expect(calls, ['端末結合', '端末確認', '実行系列挙']);
    expect(find.byType(TextField), findsNothing);
    await tester.pumpWidget(const SizedBox.shrink());
    c.dispose();
  });
  test('保管の一時読取障害は復帰時に再読取して接続を確認する', () async {
    final store = MemoryStore()..failRead = true;
    final c = DeviceLinkController(
      store: store,
      connect: (v) => LinkFixture(v, []),
    );
    await c.initialize();
    expect(c.storageReady, isFalse);
    store.failRead = false;
    c.setForeground(false);
    c.setForeground(true);
    await Future<void>.delayed(Duration.zero);
    expect(c.storageReady, isTrue);
    expect(c.ready, isFalse);
    c.dispose();
  });
  test('資格確認中にbackgroundとresumeが重なっても再確認する', () async {
    final store = MemoryStore()
      ..values[DeviceLinkController.credentialKey] = jsonEncode(data());
    late LinkFixture link;
    final c = DeviceLinkController(
      store: store,
      connect: (v) => link = LinkFixture(v, []),
    );
    await c.initialize();
    final wait = Completer<void>();
    link.wait = wait;
    final checking = c.verify();
    c.setForeground(false);
    c.setForeground(true);
    wait.complete();
    await checking;
    await Future<void>.delayed(Duration.zero);
    expect(c.ready, isTrue);
    expect(link.calls.where((v) => v == '対話送信'), isEmpty);
    c.dispose();
  });
  test('破損した保存資格を新しい招待で黙って上書きしない', () async {
    final store = MemoryStore()
      ..values[DeviceLinkController.credentialKey] = '{}';
    final c = DeviceLinkController(
      store: store,
      connect: (v) => LinkFixture(v, []),
    );
    await c.initialize();
    expect(c.ready, isFalse);
    expect(
      () => c.invitation(jsonEncode(data(invitation: true))),
      throwsA(isA<BrokerClientException>()),
    );
    await c.disconnect(localOnly: true);
    expect(c.invitation(jsonEncode(data(invitation: true))).invitation, isTrue);
    c.dispose();
  });
  test('重複key・未知field・型・Host・期限・端末の不一致を拒否', () {
    expect(
      () => strictObject('{"a":1,"a":2}'),
      throwsA(isA<BrokerClientException>()),
    );
    expect(
      () => strictObject('{"x":[{"a":1,"a":2}]}'),
      throwsA(isA<BrokerClientException>()),
    );
    expect(
      () => strictObject('{"a":1,}'),
      throwsA(isA<BrokerClientException>()),
    );
    expect(strictObject('{"a":"\\\"","b":[true,null,1]}')['b'], [
      true,
      null,
      1,
    ]);
    for (final change in <Map<String, Object?>>[
      {'unknown': true},
      {'版': 1.0},
      {'port': 0},
      {'port': 65536},
      {'port': '443'},
      {'接続先Host': 'example.com'},
      {'接続先Host': '8.8.8.8'},
      {'接続先Host': '0.0.0.0'},
      {'接続先Host': '::1'},
      {'接続先Host': 'http://127.0.0.1'},
      {'有効期限': 0},
      {'有効期限': DateTime.now().millisecondsSinceEpoch ~/ 1000 + 30000},
      {'端末ID': 'f' * 32},
      {'端末秘密': 'e' * 63},
      {'証明書hash': 'g' * 64},
    ]) {
      expect(
        () => parse({...data(), ...change}),
        throwsA(isA<BrokerClientException>()),
      );
    }
    for (final host in ['10.0.2.2', '172.16.0.1', '192.168.1.2', '127.0.0.1']) {
      expect(parse({...data(), '接続先Host': host}).text('接続先Host'), host);
    }
    expect(
      () => strictObject('{"a":"${'x' * 8192}"}'),
      throwsA(isA<BrokerClientException>()),
    );
  });
  test('資格応答のHost固定点を招待と照合', () {
    final invite = parse(data(invitation: true), invitation: true);
    parse(data()).matchesInvitation(invite);
    for (final field in ['HostID', '証明書hash', '接続先Host', 'port']) {
      final changed = {
        ...data(),
        field: field == 'port'
            ? 4321
            : field == '接続先Host'
            ? '10.0.0.2'
            : 'f' * (field == 'HostID' ? 32 : 64),
      };
      expect(
        () => parse(changed).matchesInvitation(invite),
        throwsA(isA<BrokerClientException>()),
      );
    }
  });
  test('owner操作と停止中の通信をnetwork接続前に拒否', () async {
    final client = DeviceLinkClient(parse(data()));
    for (final op in ['対話承認', '端末招待', 'shutdown']) {
      await expectLater(
        client.request(op),
        throwsA(isA<BrokerClientException>()),
      );
    }
    client.setActive(false);
    await expectLater(
      client.request('端末確認'),
      throwsA(isA<BrokerClientException>()),
    );
  });
  test('起動・復帰は資格を照会し送信を再実行しない', () async {
    final store = MemoryStore()
      ..values[DeviceLinkController.credentialKey] = jsonEncode(data());
    final calls = <String>[];
    final c = DeviceLinkController(
      store: store,
      connect: (v) => LinkFixture(v, calls),
    );
    await c.initialize();
    expect(c.ready, isTrue);
    expect(calls, ['端末確認', '実行系列挙']);
    c.setForeground(false);
    await expectLater(c.request('対話送信'), throwsA(isA<BrokerClientException>()));
    expect(c.ready, isFalse);
    c.setForeground(true);
    await Future<void>.delayed(Duration.zero);
    expect(c.ready, isTrue);
    expect(calls, ['端末確認', '実行系列挙', '端末確認', '実行系列挙']);
    c.dispose();
  });
  test('失効後は入力を停止し、平文や別Hostへfallbackしない', () async {
    final store = MemoryStore()
      ..values[DeviceLinkController.credentialKey] = jsonEncode(data());
    late LinkFixture link;
    final c = DeviceLinkController(
      store: store,
      connect: (v) => link = LinkFixture(v, []),
    );
    await c.initialize();
    link.reject = true;
    await c.verify();
    expect(c.ready, isFalse);
    expect(link.active, isFalse);
    expect(store.values[DeviceLinkController.credentialKey], isNotNull);
    c.dispose();
  });
  test('安全保管の読取失敗では結合を開始しない', () async {
    final store = MemoryStore()..failRead = true;
    final calls = <String>[];
    final c = DeviceLinkController(
      store: store,
      connect: (v) => LinkFixture(v, calls),
    );
    await c.initialize();
    expect(c.storageReady, isFalse);
    expect(c.ready, isFalse);
    expect(calls, isEmpty);
    c.dispose();
  });
  test('結合後の保管失敗は失効を要求して接続成功にしない', () async {
    final store = MemoryStore();
    final calls = <String>[];
    final c = DeviceLinkController(
      store: store,
      connect: (v) => LinkFixture(v, calls),
    );
    await c.initialize();
    store.failWrite = true;
    await c.pair(c.invitation(jsonEncode(data(invitation: true))));
    expect(c.ready, isFalse);
    expect(calls, ['端末結合', '端末離脱']);
    expect(
      store.values.containsKey(DeviceLinkController.credentialKey),
      isFalse,
    );
    c.dispose();
  });
  test('通信不能時のlocal削除をserver失効と区別する', () async {
    final store = MemoryStore()
      ..values[DeviceLinkController.credentialKey] = jsonEncode(data());
    final calls = <String>[];
    final c = DeviceLinkController(
      store: store,
      connect: (v) => LinkFixture(v, calls),
    );
    await c.initialize();
    calls.clear();
    await c.disconnect(localOnly: true);
    expect(calls, isEmpty);
    expect(c.status, contains('失効は未確認'));
    expect(c.credential, isNull);
    c.dispose();
  });
  test('資格削除失敗を解除完了と表示しない', () async {
    final store = MemoryStore()
      ..values[DeviceLinkController.credentialKey] = jsonEncode(data());
    final c = DeviceLinkController(
      store: store,
      connect: (v) => LinkFixture(v, []),
    );
    await c.initialize();
    store.failDelete = true;
    await c.disconnect();
    expect(c.ready, isFalse);
    expect(c.status, contains('確認できません'));
    expect(c.credential, isNotNull);
    c.dispose();
  });
  test('利用可能な資格なしにDesktopの解除成功を表示しない', () async {
    final store = MemoryStore()
      ..values[DeviceLinkController.credentialKey] = '{}';
    final calls = <String>[];
    final c = DeviceLinkController(
      store: store,
      connect: (v) => LinkFixture(v, calls),
    );
    await c.initialize();
    await c.disconnect();
    expect(calls, isEmpty);
    expect(c.status, contains('確認できません'));
    expect(c.hasStoredCredential, isTrue);
    expect(store.values[DeviceLinkController.credentialKey], '{}');
    c.dispose();
  });
  for (final localOnly in [false, true]) {
    test('保存資格の不在確認後に解除完了を表示する（localOnly=$localOnly）', () async {
      final store = MemoryStore()
        ..values[DeviceLinkController.credentialKey] = jsonEncode(data());
      final calls = <String>[];
      final c = DeviceLinkController(
        store: store,
        connect: (v) => LinkFixture(v, calls),
      );
      await c.initialize();
      calls.clear();
      await c.disconnect(localOnly: localOnly);
      expect(c.status, contains('削除しました'));
      expect(c.ready, isFalse);
      expect(c.credential, isNull);
      expect(c.hasStoredCredential, isFalse);
      expect(store.values[DeviceLinkController.credentialKey], isNull);
      expect(calls, localOnly ? isEmpty : ['端末離脱']);
      c.dispose();
    });
    for (final readFailure in [false, true]) {
      test(
        '削除後の${readFailure ? '読取失敗' : '資格残存'}を完了にしない（localOnly=$localOnly）',
        () async {
          final store = MemoryStore()
            ..values[DeviceLinkController.credentialKey] = jsonEncode(data());
          final calls = <String>[];
          final c = DeviceLinkController(
            store: store,
            connect: (v) => LinkFixture(v, calls),
          );
          await c.initialize();
          calls.clear();
          store.retainOnDelete = !readFailure;
          store.failReadAfterDelete = readFailure;
          await c.disconnect(localOnly: localOnly);
          expect(c.ready, isFalse);
          expect(c.status, contains('確認できません'));
          expect(c.credential, isNotNull);
          expect(c.hasStoredCredential, isTrue);
          expect(calls, localOnly ? isEmpty : ['端末離脱']);
          await expectLater(
            c.request('対話送信'),
            throwsA(isA<BrokerClientException>()),
          );
          c.dispose();
        },
      );
    }
  }
  testWidgets('未接続を表示し既存6画面と対話・接続先・設定を保持', (tester) async {
    final c = DeviceLinkController(
      store: MemoryStore(),
      connect: (v) => LinkFixture(v, []),
    );
    await tester.pumpWidget(GuiShellMobileApp(controller: c));
    await tester.pumpAndSettle();
    expect(find.textContaining('未接続'), findsOneWidget);
    expect(find.textContaining('準備完了'), findsNothing);
    await tester.tap(find.byTooltip('Open navigation menu'));
    await tester.pumpAndSettle();
    for (final name in [
      '概要',
      '確認',
      '通知',
      '実行系',
      '停止',
      '復旧',
      '対話',
      '接続先',
      '設定',
    ]) {
      expect(
        find.descendant(
          of: find.byType(NavigationDrawer),
          matching: find.text(name),
        ),
        findsOneWidget,
      );
    }
    await tester.pumpWidget(const SizedBox.shrink());
    c.dispose();
  });
}
