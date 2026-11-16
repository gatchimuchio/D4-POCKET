import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/main.dart';
import 'package:gui_shell_desktop/services/shell_core_client.dart';
import 'package:gui_shell_desktop/services/windows_tray_client.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart' show BrokerTransport;

class _TrayTransport implements BrokerTransport {
  _TrayTransport({
    this.criticalCount = 3,
    this.healthResponse,
    this.failHealth = false,
  });

  final int criticalCount;
  final Map<String, Object?>? healthResponse;
  final bool failHealth;
  final operations = <String>[];
  bool _holdNextNotification = false;
  Completer<void>? _notificationStarted;
  Completer<void>? _notificationRelease;
  bool _holdNextStopRequest = false;
  bool _failHeldStopRequest = false;
  Completer<void>? _stopRequestStarted;
  Completer<void>? _stopRequestRelease;

  void holdNextNotification() {
    _holdNextNotification = true;
    _notificationStarted = Completer<void>();
    _notificationRelease = Completer<void>();
  }

  Future<void> get notificationStarted => _notificationStarted!.future;

  void releaseNotification() => _notificationRelease!.complete();

  void holdNextStopRequest({bool fail = false}) {
    _holdNextStopRequest = true;
    _failHeldStopRequest = fail;
    _stopRequestStarted = Completer<void>();
    _stopRequestRelease = Completer<void>();
  }

  Future<void> get stopRequestStarted => _stopRequestStarted!.future;

  void releaseStopRequest() => _stopRequestRelease!.complete();

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    operations.add(operation);
    if (operation == 'health') {
      if (failHealth) throw StateError('SECRET_HEALTH_ERROR');
      return healthResponse ?? _trayHealthResponse();
    }
    if (operation == '通知一覧' && _holdNextNotification) {
      _holdNextNotification = false;
      _notificationStarted!.complete();
      await _notificationRelease!.future;
    }
    if (operation == '全Runtime停止要求') {
      if (_holdNextStopRequest) {
        _holdNextStopRequest = false;
        _stopRequestStarted!.complete();
        await _stopRequestRelease!.future;
        if (_failHeldStopRequest) {
          throw StateError('SECRET_SENTINEL_old_broker_stop_failure');
        }
        return _acceptedStopRequest();
      }
      return _acceptedStopRequest();
    }
    final notifications = List<Map<String, Object?>>.generate(
      criticalCount,
      (index) => {
        '版': 1,
        '通知ID': 'notification-broker-audit-tray-$index',
        'source': 'Audit',
        'severity': 'critical',
        'タイトル': '監査の状態更新',
        '概要': '監査eventを確認してください。',
        '作成順': index,
        '関連監査ID': 'broker-audit-tray-$index',
        '関連監査hash': 'sha256:${'a' * 64}',
        '遷移先': 'audit',
        '状態': 'unread',
        '通知hash': 'sha256:${'b' * 64}',
        '表示範囲': 'summary',
        '権限生成': 'なし',
      },
    );
    return {
      'status': 'accepted',
      'body': {
        '版': 1,
        '通知一覧': notifications,
        '件数': notifications.length,
        '未読件数': notifications.length,
        '重大件数': criticalCount,
        '証拠種別': 'INTERNAL_STATE',
        '表示範囲': 'summary',
        '権限生成': 'なし',
        '操作': 'navigation_only',
      },
    };
  }

  Map<String, Object?> _acceptedStopRequest() => {
        'operation': '全Runtime停止要求',
        'status': 'accepted',
        'evidence_source': 'INTERNAL_STATE',
        'audit_event_id': 'broker-audit-tray-stop',
        'error': null,
        'body': {
          '版': 1,
          '承認状態': 'owner_reapproval_required',
          '停止実行済み': false,
          '対象': <Object?>[],
          '復旧ID': 'recover-runtime-stop-request',
        },
      };
}

Map<String, Object?> _trayHealthResponse({
  String responseStatus = 'accepted',
  String responseEvidence = 'LIVE_RUNTIME',
  String healthStatus = 'ready',
  String? healthEvidence,
  bool persistenceReady = true,
}) {
  final evidence = healthEvidence ?? responseEvidence;
  return {
    'request_id': 'tray-health',
    'operation': 'health',
    'status': responseStatus,
    'evidence_source': responseEvidence,
    'audit_event_id': 'broker-audit-tray-health',
    'error': responseStatus == 'suspended'
        ? {
            'code': 'broker_persistence_unavailable',
            'message': 'SECRET_HEALTH_ERROR',
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

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('常駐トレイは毎回Broker Healthを取得し通知件数だけを併記する', () async {
    final transport = _TrayTransport();
    final projection = await WindowsTrayProjection.fromBroker(transport);

    expect(projection.runtimeStatus, 'suspend');
    expect(projection.pendingApprovalCount, '不明');
    expect(
      projection.criticalNotificationCount,
      kGuiShellModuleNotifications ? 3 : '不明',
    );
    expect(projection.evidenceSource, 'INTERNAL_STATE');
    expect(projection.stopRequestSupported, isTrue);
    if (kGuiShellModuleNotifications) {
      expect(transport.operations, ['health', '通知一覧']);
      expect(projection.toJson()['critical_notification_count'], 3);
    } else {
      expect(transport.operations, ['health']);
      expect(projection.toJson()['critical_notification_count'], '不明');
    }
  });

  test('Broker Health取得失敗は状態unknownにし有効な通知射影を保つ', () async {
    final transport = _TrayTransport(failHealth: true);
    final projection = await WindowsTrayProjection.fromBroker(transport);

    expect(projection.runtimeStatus, '不明');
    expect(
      projection.evidenceSource,
      kGuiShellModuleNotifications ? 'INTERNAL_STATE' : '不明',
    );
    expect(projection.pendingApprovalCount, '不明');
    expect(
      projection.criticalNotificationCount,
      kGuiShellModuleNotifications ? 3 : '不明',
    );
    expect(
      projection.toJson().toString(),
      isNot(contains('SECRET_HEALTH_ERROR')),
    );
  });

  test('Broker Healthに不明fieldがあれば現在状態を採用しない', () async {
    final response = _trayHealthResponse()..['unexpected'] = 'ready';
    final projection = await WindowsTrayProjection.fromBroker(
      _TrayTransport(healthResponse: response),
    );

    expect(projection.runtimeStatus, '不明');
    expect(
      projection.evidenceSource,
      kGuiShellModuleNotifications ? 'INTERNAL_STATE' : '不明',
    );
  });

  test('suspended HealthはInternal Stateとして安全に投影する', () async {
    final projection = await WindowsTrayProjection.fromBroker(
      _TrayTransport(
        healthResponse: _trayHealthResponse(
          responseStatus: 'suspended',
          responseEvidence: 'INTERNAL_STATE',
          healthStatus: 'suspend',
          healthEvidence: 'INTERNAL_STATE',
          persistenceReady: false,
        ),
      ),
    );

    expect(projection.runtimeStatus, 'suspend');
    expect(projection.evidenceSource, 'INTERNAL_STATE');
    expect(
      projection.toJson().toString(),
      isNot(contains('SECRET_HEALTH_ERROR')),
    );
  });

  test('常駐トレイ射影はBroker transportなしで件数を0へ変換しない', () async {
    final projection = await WindowsTrayProjection.fromBroker(null);

    expect(projection.pendingApprovalCount, '不明');
    expect(projection.criticalNotificationCount, '不明');
    expect(projection.stopRequestSupported, isFalse);
  });

  test('現在Broker Healthが読めてもApproval一覧未取得は0件と表示しない', () async {
    final projection = await WindowsTrayProjection.fromBroker(_TrayTransport());

    expect(projection.pendingApprovalCount, '不明');
  });

  test('隠れた常駐トレイはBrokerをpollせず全ての測定値を不明にする', () async {
    final transport = _TrayTransport();
    final projection = await WindowsTrayProjection.fromBroker(
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

  test('StatusNotifier hostの後着と切断をFlutterへ反映する', () async {
    const channel = MethodChannel('gui_shell/tray_late_availability_test');
    final messenger =
        TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
    messenger.setMockMethodCallHandler(channel, (call) async {
      return switch (call.method) {
        'initialize' => null,
        'isWindowVisible' => true,
        _ => null,
      };
    });
    addTearDown(() => messenger.setMockMethodCallHandler(channel, null));

    final visibilityChanges = <bool>[];
    final client = WindowsTrayClient(channel: channel);
    expect(
      await client.start(
        onAction: (_) {},
        onVisibilityChanged: visibilityChanges.add,
      ),
      isFalse,
    );
    expect(client.available, isFalse);

    const codec = StandardMethodCodec();
    await messenger.handlePlatformMessage(
      channel.name,
      codec.encodeMethodCall(
        const MethodCall('onTrayAvailabilityChanged', true),
      ),
      (_) {},
    );
    expect(client.available, isTrue);
    expect(visibilityChanges, [true]);

    await messenger.handlePlatformMessage(
      channel.name,
      codec.encodeMethodCall(
        const MethodCall('onTrayAvailabilityChanged', false),
      ),
      (_) {},
    );
    expect(client.available, isFalse);
    expect(visibilityChanges, [true, false]);
    await client.dispose();
  });

  testWidgets('ウィンドウを隠すとトレイ更新を止め、再表示後に再開する', (tester) async {
    const channel = MethodChannel('gui_shell/tray');
    final messenger =
        TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
    var windowVisible = true;
    var visibilityQueries = 0;
    final transport = _TrayTransport();
    final published = <Map<Object?, Object?>>[];
    messenger.setMockMethodCallHandler(channel, (call) async {
      return switch (call.method) {
        'initialize' => windowVisible,
        'isWindowVisible' => () {
            visibilityQueries += 1;
            return windowVisible;
          }(),
        'publish' => () {
            published.add(call.arguments! as Map<Object?, Object?>);
            return null;
          }(),
        _ => null,
      };
    });
    addTearDown(() async {
      await tester.pumpWidget(const SizedBox.shrink());
      messenger.setMockMethodCallHandler(channel, null);
    });

    await tester.pumpWidget(
      GuiShellDesktopApp(client: ShellCoreClient.mock(transport: transport)),
    );
    await tester.pumpAndSettle();
    expect(visibilityQueries, 2);
    int notificationPollCount() =>
        transport.operations.where((operation) => operation == '通知一覧').length;

    final initialNotificationPolls = notificationPollCount();
    expect(initialNotificationPolls, greaterThan(0));

    await tester.pump(const Duration(seconds: 30));
    await tester.pumpAndSettle();
    expect(visibilityQueries, 4);
    expect(notificationPollCount(), initialNotificationPolls + 1);

    Future<void> reportNativeVisibility(bool visible) async {
      const codec = StandardMethodCodec();
      await messenger.handlePlatformMessage(
        channel.name,
        codec.encodeMethodCall(
          MethodCall('onWindowVisibilityChanged', visible),
        ),
        (_) {},
      );
      await tester.pumpAndSettle();
    }

    windowVisible = false;
    await reportNativeVisibility(false);
    expect(published.last['runtime_status'], '不明');
    final queriesWhenHidden = visibilityQueries;
    await tester.pump(const Duration(seconds: 60));
    await tester.pumpAndSettle();
    expect(visibilityQueries, queriesWhenHidden);
    expect(notificationPollCount(), initialNotificationPolls + 1);

    windowVisible = true;
    await reportNativeVisibility(true);
    expect(visibilityQueries, queriesWhenHidden + 2);
    expect(notificationPollCount(), initialNotificationPolls + 2);
    await tester.pump(const Duration(seconds: 30));
    await tester.pumpAndSettle();
    expect(visibilityQueries, queriesWhenHidden + 4);
    expect(notificationPollCount(), initialNotificationPolls + 3);
  });

  testWidgets('通知取得中に隠した場合は完了後の古い射影を破棄する', (tester) async {
    const channel = MethodChannel('gui_shell/tray');
    final messenger =
        TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
    var windowVisible = true;
    final transport = _TrayTransport();
    final published = <Map<Object?, Object?>>[];
    messenger.setMockMethodCallHandler(channel, (call) async {
      return switch (call.method) {
        'initialize' => windowVisible,
        'isWindowVisible' => windowVisible,
        'publish' => () {
            published.add(call.arguments! as Map<Object?, Object?>);
            return null;
          }(),
        _ => null,
      };
    });
    addTearDown(() async {
      await tester.pumpWidget(const SizedBox.shrink());
      messenger.setMockMethodCallHandler(channel, null);
    });

    await tester.pumpWidget(
      GuiShellDesktopApp(client: ShellCoreClient.mock(transport: transport)),
    );
    await tester.pumpAndSettle();
    final priorNotificationPolls =
        transport.operations.where((operation) => operation == '通知一覧').length;

    transport.holdNextNotification();
    await tester.pump(const Duration(seconds: 30));
    await transport.notificationStarted;
    expect(
      transport.operations.where((operation) => operation == '通知一覧').length,
      priorNotificationPolls + 1,
    );

    windowVisible = false;
    const codec = StandardMethodCodec();
    await messenger.handlePlatformMessage(
      channel.name,
      codec.encodeMethodCall(
        const MethodCall('onWindowVisibilityChanged', false),
      ),
      (_) {},
    );
    await tester.pumpAndSettle();
    expect(published.last['runtime_status'], '不明');
    final hiddenProjectionCount = published.length;

    transport.releaseNotification();
    await tester.pumpAndSettle();
    expect(published, hasLength(hiddenProjectionCount));
    expect(published.last['runtime_status'], '不明');
  });

  testWidgets('Broker切替中の旧通知射影を捨てて新Broker状態をすぐ取得する', (tester) async {
    const channel = MethodChannel('gui_shell/tray');
    final messenger =
        TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
    var windowVisible = true;
    final oldTransport = _TrayTransport(criticalCount: 1);
    final newTransport = _TrayTransport(criticalCount: 2);
    final published = <Map<Object?, Object?>>[];
    messenger.setMockMethodCallHandler(channel, (call) async {
      return switch (call.method) {
        'initialize' => windowVisible,
        'isWindowVisible' => windowVisible,
        'publish' => () {
            published.add(call.arguments! as Map<Object?, Object?>);
            return null;
          }(),
        _ => null,
      };
    });
    addTearDown(() async {
      await tester.pumpWidget(const SizedBox.shrink());
      messenger.setMockMethodCallHandler(channel, null);
    });

    await tester.pumpWidget(
      GuiShellDesktopApp(client: ShellCoreClient.mock(transport: oldTransport)),
    );
    await tester.pumpAndSettle();
    expect(published.last['critical_notification_count'], 1);

    oldTransport.holdNextNotification();
    await tester.pump(const Duration(seconds: 30));
    await oldTransport.notificationStarted;

    await tester.pumpWidget(
      GuiShellDesktopApp(client: ShellCoreClient.mock(transport: newTransport)),
    );
    await tester.pump();
    expect(newTransport.operations, isEmpty);

    oldTransport.releaseNotification();
    await tester.pumpAndSettle();

    expect(newTransport.operations, contains('通知一覧'));
    expect(published.last['critical_notification_count'], 2);
    expect(
      published.where(
          (projection) => projection['critical_notification_count'] == 1),
      hasLength(1),
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('Broker切替後に旧Brokerの停止要求receiptを表示しない', (tester) async {
    const channel = MethodChannel('gui_shell/tray');
    final messenger =
        TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
    messenger.setMockMethodCallHandler(channel, (call) async {
      return switch (call.method) {
        'initialize' => true,
        'isWindowVisible' => true,
        'publish' => null,
        _ => null,
      };
    });
    addTearDown(() async {
      await tester.pumpWidget(const SizedBox.shrink());
      messenger.setMockMethodCallHandler(channel, null);
    });

    final oldTransport = _TrayTransport()..holdNextStopRequest();
    final newTransport = _TrayTransport();
    await tester.pumpWidget(
      GuiShellDesktopApp(client: ShellCoreClient.mock(transport: oldTransport)),
    );
    await tester.pumpAndSettle();

    const codec = StandardMethodCodec();
    final pendingAction = messenger.handlePlatformMessage(
      channel.name,
      codec.encodeMethodCall(
        const MethodCall('onTrayAction', 'stop_request'),
      ),
      (_) {},
    );
    await oldTransport.stopRequestStarted;

    await tester.pumpWidget(
      GuiShellDesktopApp(client: ShellCoreClient.mock(transport: newTransport)),
    );
    await tester.pumpAndSettle();
    oldTransport.releaseStopRequest();
    await pendingAction;
    await tester.pumpAndSettle();

    expect(
      find.textContaining(
        'Broker接続先が切り替わったため、旧Brokerの停止要求結果を破棄しました',
      ),
      findsOneWidget,
    );
    expect(find.textContaining('全Runtime停止要求をBrokerへ渡しました'), findsNothing);
    expect(oldTransport.operations, contains('全Runtime停止要求'));
    expect(newTransport.operations, isNot(contains('全Runtime停止要求')));
    expect(tester.takeException(), isNull);
  });

  testWidgets('Broker切替後に旧Brokerの停止要求errorも露出しない', (tester) async {
    const channel = MethodChannel('gui_shell/tray');
    final messenger =
        TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
    messenger.setMockMethodCallHandler(channel, (call) async {
      return switch (call.method) {
        'initialize' => true,
        'isWindowVisible' => true,
        'publish' => null,
        _ => null,
      };
    });
    addTearDown(() async {
      await tester.pumpWidget(const SizedBox.shrink());
      messenger.setMockMethodCallHandler(channel, null);
    });

    final oldTransport = _TrayTransport()..holdNextStopRequest(fail: true);
    final newTransport = _TrayTransport();
    await tester.pumpWidget(
      GuiShellDesktopApp(client: ShellCoreClient.mock(transport: oldTransport)),
    );
    await tester.pumpAndSettle();

    const codec = StandardMethodCodec();
    final pendingAction = messenger.handlePlatformMessage(
      channel.name,
      codec.encodeMethodCall(
        const MethodCall('onTrayAction', 'stop_request'),
      ),
      (_) {},
    );
    await oldTransport.stopRequestStarted;

    await tester.pumpWidget(
      GuiShellDesktopApp(client: ShellCoreClient.mock(transport: newTransport)),
    );
    await tester.pumpAndSettle();
    oldTransport.releaseStopRequest();
    await pendingAction;
    await tester.pumpAndSettle();

    expect(
      find.textContaining(
        'Broker接続先が切り替わったため、旧Brokerの停止要求結果を破棄しました',
      ),
      findsOneWidget,
    );
    expect(find.textContaining('全Runtime停止要求の応答を確認できません'), findsNothing);
    expect(find.textContaining('SECRET_SENTINEL'), findsNothing);
    expect(newTransport.operations, isNot(contains('全Runtime停止要求')));
    expect(tester.takeException(), isNull);
  });
}
