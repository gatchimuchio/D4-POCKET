import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_mobile/services/device_link_client.dart';
import 'package:gui_shell_mobile/services/device_link_controller.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart';

const _snapshot = <String, Object?>{
  'storage_ready': true,
  'paired': true,
  'connected': true,
  'foreground': true,
  'status': '接続を確認しました',
};

const _localRecoveryEvent = <String, Object?>{
  'event_id': 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
  'timestamp': '2026-09-25T12:00:00Z',
  'actor': 'shell',
  'action': 'mobile_local_credential_delete',
  'target': 'mobile_device_link_credential',
  'result': 'success',
  'payload_hash':
      'sha256:2f0b366fb62efa9e741f0127c676e648d81b263a471c35d0dd00814a174e5a92',
  'metadata': <String, Object?>{
    'audit_scope': 'mobile_local_recovery',
    'evidence_source': 'INTERNAL_STATE',
    'desktop_revocation': 'unconfirmed',
    'authority_effect': 'none',
    'operator_identity': 'unverified',
  },
};

Map<String, Object?> _accepted(String operation, [Object? body = const {}]) => {
  'operation': operation,
  'status': 'accepted',
  'audit_event_id': 'audit-1',
  'evidence_source': 'LIVE_RUNTIME',
  'body': body,
  'error': null,
};

class _FakeNativePort implements DeviceLinkNativePort {
  _FakeNativePort({DeviceLinkSnapshot? initial})
    : state =
          initial ??
          const DeviceLinkSnapshot(
            storageReady: true,
            paired: false,
            connected: false,
            foreground: true,
            status: '未接続',
          );

  DeviceLinkSnapshot state;
  final calls = <String>[];
  final requests = <String>[];
  final recoveryAudit = <DeviceLinkLocalRecoveryAuditEvent>[];

  @override
  Future<DeviceLinkSnapshot> readState() async {
    calls.add('read_state');
    return state;
  }

  @override
  Future<List<DeviceLinkLocalRecoveryAuditEvent>>
  readLocalRecoveryAudit() async {
    calls.add('read_recovery_audit');
    return List.unmodifiable(recoveryAudit);
  }

  @override
  Future<DeviceLinkSnapshot> pair() async {
    calls.add('pair');
    state = const DeviceLinkSnapshot(
      storageReady: true,
      paired: true,
      connected: true,
      foreground: true,
      status: 'native結合を確認しました',
    );
    return state;
  }

  @override
  Future<DeviceLinkSnapshot> disconnect() async {
    calls.add('disconnect');
    return _unpaired();
  }

  @override
  Future<DeviceLinkSnapshot> localDelete() async {
    calls.add('local_delete');
    recoveryAudit.add(
      DeviceLinkLocalRecoveryAuditEvent(
        eventId: 'a' * 32,
        timestamp: '2026-09-25T12:00:00Z',
      ),
    );
    return _unpaired();
  }

  DeviceLinkSnapshot _unpaired() => state = const DeviceLinkSnapshot(
    storageReady: true,
    paired: false,
    connected: false,
    foreground: true,
    status: 'native資格を削除し回復記録を保存しました。Desktop側の失効は未確認です。',
  );

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    requests.add(operation);
    if (operation == '実行系列挙') {
      return _accepted(operation, {
        '実行系': ['runtime-a'],
      });
    }
    if (operation == '作業領域一覧') {
      return {
        ..._accepted(operation, {
          '作業領域': [
            {'作業領域ID': 'workspace-a', '実行系ID': 'runtime-a'},
          ],
        }),
        'evidence_source': 'INTERNAL_STATE',
      };
    }
    return _accepted(operation);
  }
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('native状態の余分なfieldと資格値を拒否する', () {
    expect(DeviceLinkSnapshot.fromNative(_snapshot).paired, isTrue);
    expect(
      () =>
          DeviceLinkSnapshot.fromNative({..._snapshot, 'device_id': 'a' * 32}),
      throwsA(isA<BrokerClientException>()),
    );
    expect(
      () =>
          DeviceLinkSnapshot.fromNative({..._snapshot, 'credential': 'secret'}),
      throwsA(isA<BrokerClientException>()),
    );
  });

  test('controllerは資格を受け取らずnative結合と通常要求を委譲する', () async {
    final native = _FakeNativePort();
    final controller = DeviceLinkController(nativePort: native);
    addTearDown(controller.dispose);

    await controller.initialize();
    expect(controller.storageReady, isTrue);
    expect(controller.ready, isFalse);
    await controller.pair();

    expect(controller.ready, isTrue);
    expect(controller.hasStoredCredential, isTrue);
    expect(controller.runtimes, ['runtime-a']);
    expect(native.calls, ['read_state', 'pair']);
    expect(native.requests, ['実行系列挙']);
  });

  test('Mobile対話clientはnative Device Link経由の選択用Workspace投影を使う', () async {
    final native = _FakeNativePort();
    final controller = DeviceLinkController(nativePort: native);
    addTearDown(controller.dispose);
    await controller.initialize();
    await controller.pair();

    expect(await controller.dialogue.workspaceIdsByRuntime(), {
      'runtime-a': ['workspace-a'],
    });
    expect(native.requests, ['実行系列挙', '作業領域一覧']);
  });

  test('local-only削除はnative回復記録を要求しDesktop失効を主張しない', () async {
    final native = _FakeNativePort(
      initial: const DeviceLinkSnapshot(
        storageReady: true,
        paired: true,
        connected: false,
        foreground: true,
        status: '接続不能',
      ),
    );
    final controller = DeviceLinkController(nativePort: native);
    addTearDown(controller.dispose);
    await controller.initialize();
    native.calls.clear();
    native.requests.clear();

    await controller.disconnect(localOnly: true);

    expect(native.calls, ['local_delete']);
    expect(native.requests, isEmpty);
    expect(controller.hasStoredCredential, isFalse);
    expect(controller.status, contains('回復記録'));
    expect(controller.status, contains('Desktop側の失効は未確認'));
    final events = await controller.readLocalRecoveryAudit();
    expect(native.calls, ['local_delete', 'read_recovery_audit']);
    expect(events.single.eventId, 'a' * 32);
    expect(events.single.timestamp, '2026-09-25T12:00:00Z');
  });

  test('端末内回復記録projectionは固定fieldだけを受け取りsecret混入を拒否する', () {
    final parsed = DeviceLinkLocalRecoveryAuditEvent.fromNative([
      _localRecoveryEvent,
    ]);
    expect(parsed.single.eventId, 'b' * 32);
    expect(
      DeviceLinkLocalRecoveryAuditEvent.fromNative([
        {..._localRecoveryEvent, 'timestamp': '2026-09-25T12:00:00.123Z'},
      ]).single.timestamp,
      '2026-09-25T12:00:00.123Z',
    );
    expect(
      () => DeviceLinkLocalRecoveryAuditEvent.fromNative([
        {..._localRecoveryEvent, 'credential_secret': 'secret'},
      ]),
      throwsA(isA<BrokerClientException>()),
    );
    for (final timestamp in [
      '2026-02-30T12:00:00Z',
      '2026-09-25T12:00:00Z\n',
      '2026-09-25T12:00:00+00:00',
      '2026-09-25T12:00:00.1Z',
      '2026-09-25T12:00:00.1234Z',
    ]) {
      expect(
        () => DeviceLinkLocalRecoveryAuditEvent.fromNative([
          {..._localRecoveryEvent, 'timestamp': timestamp},
        ]),
        throwsA(isA<BrokerClientException>()),
      );
    }
  });

  test('回復記録照会はversionだけをnativeへ渡しDesktop Broker要求を作らない', () async {
    const channel = MethodChannel('test/gui_shell/mobile_device_link_recovery');
    final calls = <MethodCall>[];
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (call) async {
          calls.add(call);
          return [_localRecoveryEvent];
        });
    addTearDown(
      () => TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, null),
    );
    final client = MethodChannelDeviceLink(channel: channel);

    final events = await client.readLocalRecoveryAudit();

    expect(events.single.eventId, 'b' * 32);
    expect(calls.single.method, 'read_recovery_audit');
    expect(calls.single.arguments, {'version': 1});
  });

  test('前景復帰はnative状態で再確認し背景要求を自動再送しない', () async {
    final native = _FakeNativePort(
      initial: const DeviceLinkSnapshot(
        storageReady: true,
        paired: true,
        connected: true,
        foreground: true,
        status: '接続',
      ),
    );
    final controller = DeviceLinkController(nativePort: native);
    addTearDown(controller.dispose);
    await controller.initialize();
    native.requests.clear();

    controller.setForeground(false);
    await expectLater(
      controller.request('対話送信', payload: {'入力': '再送しない'}),
      throwsA(isA<BrokerClientException>()),
    );

    expect(native.requests, isEmpty);
    expect(native.calls, ['read_state']);

    native.state = const DeviceLinkSnapshot(
      storageReady: true,
      paired: true,
      connected: false,
      foreground: false,
      status: 'OSは背景状態',
    );
    controller.setForeground(true);
    await Future<void>.delayed(const Duration(milliseconds: 10));
    expect(native.calls, ['read_state', 'read_state']);
    expect(controller.foreground, isFalse);
    expect(controller.ready, isFalse);

    native.state = const DeviceLinkSnapshot(
      storageReady: true,
      paired: true,
      connected: true,
      foreground: true,
      status: 'OSは前景状態',
    );
    controller.setForeground(true);
    await Future<void>.delayed(const Duration(milliseconds: 10));
    expect(native.calls, ['read_state', 'read_state', 'read_state']);
    expect(controller.ready, isTrue);
  });

  test('履歴承認参照はboundedな履歴閲覧要求だけに許可する', () async {
    const channel = MethodChannel('test/gui_shell/mobile_device_link');
    final calls = <MethodCall>[];
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (call) async {
          calls.add(call);
          final arguments = Map<Object?, Object?>.from(call.arguments as Map);
          return _accepted(arguments['broker_operation'] as String);
        });
    addTearDown(
      () => TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, null),
    );
    final client = MethodChannelDeviceLink(channel: channel);

    await client.request(
      '対話履歴閲覧',
      payload: {
        'approval_id': 'a' * 32,
        'query': {'after': 0, 'limit': 20},
      },
    );
    expect(calls, hasLength(1));
    final arguments = Map<Object?, Object?>.from(calls.single.arguments as Map);
    final payload = Map<Object?, Object?>.from(arguments['payload'] as Map);
    expect(payload['approval_id'], 'a' * 32);

    await expectLater(
      client.request('通知一覧', payload: {'approval_id': 'a' * 32}),
      throwsA(isA<BrokerClientException>()),
    );
    await expectLater(
      client.request(
        '対話履歴閲覧',
        payload: {
          'approval_id': 'a' * 32,
          'query': {'after': -1, 'limit': 20},
        },
      ),
      throwsA(isA<BrokerClientException>()),
    );
    expect(calls, hasLength(1));
  });

  test('他operationはIPC前に入れ子の権限・秘密fieldを拒否する', () async {
    const channel = MethodChannel('test/gui_shell/mobile_device_link_payload');
    var calls = 0;
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (call) async {
          calls++;
          return _accepted('対話送信');
        });
    addTearDown(
      () => TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, null),
    );
    final client = MethodChannelDeviceLink(channel: channel);

    for (final payload in [
      {'permission': 'granted'},
      {
        'nested': {'client_credential': 'not-forwarded'},
      },
      {'招待秘密': 'not-forwarded'},
    ]) {
      await expectLater(
        client.request('対話送信', payload: payload),
        throwsA(isA<BrokerClientException>()),
      );
    }
    expect(calls, 0);
  });

  test('Broker応答は自由文errorを除去し資格漏えいを拒否する', () async {
    const channel = MethodChannel('test/gui_shell/mobile_device_link_response');
    Object? response = {
      'operation': '通知一覧',
      'status': 'rejected',
      'audit_event_id': 'audit-1',
      'evidence_source': 'INTERNAL_STATE',
      'body': null,
      'error': {'code': '権限拒否', 'message': 'sensitive detail'},
    };
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (call) async => response);
    addTearDown(
      () => TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, null),
    );
    final client = MethodChannelDeviceLink(channel: channel);

    final safe = await client.request('通知一覧');
    expect(safe['error'], {'code': '権限拒否'});
    expect(safe.toString(), isNot(contains('sensitive detail')));

    response = {
      ..._accepted('通知一覧', {
        'nested': {'credential_secret': 'leak'},
      }),
    };
    await expectLater(
      client.request('通知一覧'),
      throwsA(isA<BrokerClientException>()),
    );
  });

  test('native実装がない場合は一般化errorでfail-closedする', () async {
    final client = MethodChannelDeviceLink(
      channel: const MethodChannel('test/gui_shell/missing_device_link'),
    );
    await expectLater(
      client.readState(),
      throwsA(
        isA<BrokerClientException>().having(
          (error) => error.message,
          'message',
          isNot(contains('secret')),
        ),
      ),
    );
  });
}
