import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/services/shell_core_client.dart';
import 'package:gui_shell_desktop/services/windows_tray_client.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart' show BrokerTransport;

class _TrayTransport implements BrokerTransport {
  final operations = <String>[];

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    operations.add(operation);
    return {
      'status': 'accepted',
      'body': {
        '版': 1,
        '通知一覧': <Object?>[],
        '件数': 3,
        '未読件数': 3,
        '重大件数': 3,
        '証拠種別': 'INTERNAL_STATE',
        '表示範囲': 'summary',
        '権限生成': 'なし',
        '操作': 'navigation_only',
      },
    };
  }
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('常駐トレイ射影は未Broker snapshotを不明にし通知件数だけ取得する', () async {
    final transport = _TrayTransport();
    final projection = await WindowsTrayProjection.fromSnapshot(
      ShellCoreClient.mock().getSnapshot(),
      transport,
    );

    expect(projection.runtimeStatus, '不明');
    expect(projection.pendingApprovalCount, '不明');
    expect(projection.criticalNotificationCount, 3);
    expect(projection.evidenceSource, '不明');
    expect(projection.stopRequestSupported, isTrue);
    expect(transport.operations, ['通知一覧']);
    expect(projection.toJson()['critical_notification_count'], 3);
  });

  test('常駐トレイ射影はBroker transportなしで件数を0へ変換しない', () async {
    final projection = await WindowsTrayProjection.fromSnapshot(
      ShellCoreClient.mock().getSnapshot(),
      null,
    );

    expect(projection.pendingApprovalCount, '不明');
    expect(projection.criticalNotificationCount, '不明');
    expect(projection.stopRequestSupported, isFalse);
  });

  test('隠れた常駐トレイはBrokerをpollせず全ての測定値を不明にする', () async {
    final transport = _TrayTransport();
    final projection = await WindowsTrayProjection.fromSnapshot(
      ShellCoreClient.mock().getSnapshot(),
      transport,
      windowVisible: false,
    );

    expect(projection.runtimeStatus, '不明');
    expect(projection.pendingApprovalCount, '不明');
    expect(projection.criticalNotificationCount, '不明');
    expect(projection.evidenceSource, '不明');
    expect(projection.stopRequestSupported, isTrue);
    expect(transport.operations, isEmpty);
  });

  test('トレイ初期化と可視性はnativeの観測値を返す', () async {
    const channel = MethodChannel('gui_shell/tray_test');
    final messenger =
        TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
    messenger.setMockMethodCallHandler(channel, (call) async {
      return switch (call.method) {
        'initialize' => true,
        'isWindowVisible' => false,
        _ => null,
      };
    });
    addTearDown(() => messenger.setMockMethodCallHandler(channel, null));

    final client = WindowsTrayClient(channel: channel);
    expect(await client.start(onAction: (_) {}), isTrue);
    expect(client.available, isTrue);
    expect(await client.isWindowVisible(), isFalse);
    await client.dispose();
  });

  test('nativeの可視性通知を型検査して通知する', () async {
    const channel = MethodChannel('gui_shell/tray_visibility_test');
    final messenger =
        TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
    messenger.setMockMethodCallHandler(channel, (call) async {
      return call.method == 'initialize' ? true : null;
    });
    addTearDown(() => messenger.setMockMethodCallHandler(channel, null));

    final visibilityChanges = <bool>[];
    final client = WindowsTrayClient(channel: channel);
    await client.start(
      onAction: (_) {},
      onVisibilityChanged: visibilityChanges.add,
    );
    const codec = StandardMethodCodec();
    await messenger.handlePlatformMessage(
      channel.name,
      codec.encodeMethodCall(
        const MethodCall('onWindowVisibilityChanged', false),
      ),
      (_) {},
    );
    await messenger.handlePlatformMessage(
      channel.name,
      codec.encodeMethodCall(
        const MethodCall('onWindowVisibilityChanged', 'false'),
      ),
      (_) {},
    );

    expect(visibilityChanges, [false]);
    await client.dispose();
  });

  test('native可視性契約がない場合はトレイを利用可能としない', () async {
    const channel = MethodChannel('gui_shell/tray_unavailable_test');
    final messenger =
        TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
    messenger.setMockMethodCallHandler(channel, (_) async => null);
    addTearDown(() => messenger.setMockMethodCallHandler(channel, null));

    final client = WindowsTrayClient(channel: channel);
    expect(await client.start(onAction: (_) {}), isFalse);
    expect(client.available, isFalse);
    expect(await client.isWindowVisible(), isNull);
    await client.dispose();
  });
}
