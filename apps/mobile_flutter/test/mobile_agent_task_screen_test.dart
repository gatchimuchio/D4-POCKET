import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_mobile/screens/mobile_agent_task.dart';
import 'package:gui_shell_mobile/services/device_link_client.dart';
import 'package:gui_shell_mobile/services/device_link_controller.dart';

const _sessionId = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
const _requestId = 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
const _instructionHash =
    'sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
const _mcpCapabilityHash =
    'sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc';

Map<String, Object?> _response(
  String operation,
  String status,
  Object? body, {
  Object? error,
  String evidenceSource = 'INTERNAL_STATE',
}) => {
  'operation': operation,
  'status': status,
  'audit_event_id': 'broker-audit-mobile-screen',
  'evidence_source': evidenceSource,
  'body': body,
  'error': error,
};

class _TaskSubmissionPort implements DeviceLinkNativePort {
  _TaskSubmissionPort({required this.definitiveRejection});

  final bool definitiveRejection;
  int sessionCloseCount = 0;

  @override
  Future<DeviceLinkSnapshot> readState() async => const DeviceLinkSnapshot(
    storageReady: true,
    paired: true,
    connected: true,
    foreground: true,
    status: '接続中',
  );

  @override
  Future<List<DeviceLinkLocalRecoveryAuditEvent>>
  readLocalRecoveryAudit() async => const [];

  @override
  Future<DeviceLinkSnapshot> pair() => readState();

  @override
  Future<DeviceLinkSnapshot> disconnect() => readState();

  @override
  Future<DeviceLinkSnapshot> localDelete() => readState();

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    switch (operation) {
      case '実行系列挙':
        return _response(operation, 'accepted', {
          '実行系': ['codex-runtime'],
        });
      case '作業領域一覧':
        return _response(operation, 'accepted', {
          '作業領域': [
            {'実行系ID': 'codex-runtime', '作業領域ID': 'workspace-a'},
          ],
        });
      case 'AgentTask端末申請一覧':
        return _response(operation, 'accepted', {'申請': []});
      case '対話開始':
        return _response(operation, 'accepted', {
          '実行系ID': 'codex-runtime',
          '対話セッションID': _sessionId,
          '状態': '利用中',
        });
      case 'AgentTask端末申請':
        if (definitiveRejection) {
          return _response(
            operation,
            'rejected',
            null,
            evidenceSource: 'LIVE_RUNTIME',
            error: {'code': 'AgentTask申請上限'},
          );
        }
        throw StateError('応答消失');
      case '対話終了':
        sessionCloseCount++;
        return _response(operation, 'accepted', {
          '対話セッションID': _sessionId,
          '状態': '終了',
        });
      default:
        throw StateError('想定外のBroker操作: $operation');
    }
  }
}

class _McpTaskPort extends _TaskSubmissionPort {
  _McpTaskPort({this.capabilityAvailable = true})
    : super(definitiveRejection: false);

  final bool capabilityAvailable;
  final operations = <String>[];
  Map<String, Object?>? submittedPayload;

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    operations.add(operation);
    if (operation == 'AgentTask端末MCP能力取得') {
      return _response(operation, 'accepted', {
        '版': 1,
        'Agent能力hash': capabilityAvailable ? _mcpCapabilityHash : null,
        'Agent能力件数': capabilityAvailable ? 1 : 0,
        'Agent能力状態': capabilityAvailable ? 'available' : 'unsupported',
        '公開範囲': 'metadata_only',
      });
    }
    if (operation == 'AgentTask端末申請') {
      submittedPayload = payload;
      final now = DateTime.now().toUtc().millisecondsSinceEpoch ~/ 1000;
      return _response(operation, 'accepted', {
        '版': 1,
        '申請ID': _requestId,
        '状態': 'Owner確認待ち',
        '実行系ID': 'codex-runtime',
        '対話セッションID': _sessionId,
        '作業領域ID': 'workspace-a',
        '指示hash': _instructionHash,
        '期限': now + 900,
      });
    }
    return super.request(operation, payload: payload);
  }
}

class _LifecyclePort implements DeviceLinkNativePort {
  _LifecyclePort({
    required this.runtimeId,
    required this.workspaceId,
    this.requests = const [],
    this.firstRuntimeResponse,
    this.firstRuntimeStarted,
    this.statusResponse,
    this.statusRequestStarted,
  });

  final String runtimeId;
  final String workspaceId;
  final List<Map<String, Object?>> requests;
  final Completer<Map<String, Object?>>? firstRuntimeResponse;
  final Completer<void>? firstRuntimeStarted;
  final Completer<Map<String, Object?>>? statusResponse;
  final Completer<void>? statusRequestStarted;
  int runtimeCalls = 0;
  int statusRequestCount = 0;
  final operations = <String>[];

  @override
  Future<DeviceLinkSnapshot> readState() async => const DeviceLinkSnapshot(
    storageReady: true,
    paired: true,
    connected: true,
    foreground: true,
    status: '接続中',
  );

  @override
  Future<List<DeviceLinkLocalRecoveryAuditEvent>>
  readLocalRecoveryAudit() async => const [];

  @override
  Future<DeviceLinkSnapshot> pair() => readState();

  @override
  Future<DeviceLinkSnapshot> disconnect() => readState();

  @override
  Future<DeviceLinkSnapshot> localDelete() => readState();

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    operations.add(operation);
    switch (operation) {
      case '実行系列挙':
        runtimeCalls++;
        if (runtimeCalls == 1 && firstRuntimeResponse != null) {
          firstRuntimeStarted?.complete();
          return firstRuntimeResponse!.future;
        }
        return _response(operation, 'accepted', {
          '実行系': [runtimeId],
        });
      case '作業領域一覧':
        return _response(operation, 'accepted', {
          '作業領域': [
            {'実行系ID': runtimeId, '作業領域ID': workspaceId},
          ],
        });
      case 'AgentTask端末申請一覧':
        return _response(operation, 'accepted', {'申請': requests});
      case 'AgentTask端末状態':
        if (statusRequestStarted != null &&
            !statusRequestStarted!.isCompleted) {
          statusRequestStarted!.complete();
        }
        final requestIndex = statusRequestCount++;
        if (requestIndex == 0 && statusResponse != null) {
          return statusResponse!.future;
        }
        return _response(operation, 'accepted', {
          '申請ID': payload?['申請ID'],
          '状態': 'Owner確認待ち',
        });
      default:
        throw StateError('想定外のBroker操作: $operation');
    }
  }
}

Map<String, Object?> _ownRequestRow() => {
  '申請ID': _requestId,
  '実行系ID': 'runtime-old',
  '対話セッションID': _sessionId,
  '作業領域ID': 'workspace-old',
  '指示hash': _instructionHash,
  '作成時刻': 1_800_000_000,
  '期限': 1_800_000_900,
  'task_id': null,
};

void main() {
  Future<void> submitFromScreen(
    WidgetTester tester,
    _TaskSubmissionPort port,
  ) async {
    final controller = DeviceLinkController(nativePort: port)..ready = true;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: MobileAgentTaskScreen(
            controller: controller,
            connected: true,
            active: true,
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    final instruction = find.byKey(
      const ValueKey('mobile-agent-task-instruction'),
    );
    await tester.ensureVisible(instruction);
    await tester.enterText(instruction, '選択した作業領域を確認する');
    await tester.pump();
    final submit = find.byKey(const ValueKey('mobile-agent-task-submit'));
    await tester.ensureVisible(submit);
    await tester.tap(submit);
    await tester.pumpAndSettle();
    controller.dispose();
  }

  testWidgets('申請応答が不明ならSessionを維持し、再送を促さない', (tester) async {
    final port = _TaskSubmissionPort(definitiveRejection: false);
    await submitFromScreen(tester, port);

    expect(port.sessionCloseCount, 0);
    expect(find.textContaining('申請結果が不明です'), findsOneWidget);
    expect(find.textContaining('Sessionを維持しました'), findsOneWidget);
    expect(find.textContaining('再送せず'), findsOneWidget);
  });

  testWidgets('Brokerの明示拒否時だけ空Sessionを閉じる', (tester) async {
    final port = _TaskSubmissionPort(definitiveRejection: true);
    await submitFromScreen(tester, port);

    expect(port.sessionCloseCount, 1);
    expect(find.textContaining('Brokerが申請を拒否しました'), findsOneWidget);
    expect(find.textContaining('Sessionを終了しました'), findsOneWidget);
  });

  testWidgets('MCP選択時は現在の能力hashを先に取得してTask申請へ含める', (tester) async {
    tester.view.physicalSize = const Size(800, 1200);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final port = _McpTaskPort();
    final controller = DeviceLinkController(nativePort: port)..ready = true;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: MobileAgentTaskScreen(
            controller: controller,
            connected: true,
            active: true,
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    final instruction = find.byKey(
      const ValueKey('mobile-agent-task-instruction'),
    );
    await tester.ensureVisible(instruction);
    await tester.enterText(instruction, 'MCP能力を使って調査する');
    final mcpToggle = find.byKey(
      const ValueKey('mobile-agent-task-mcp-capabilities'),
    );
    await tester.ensureVisible(mcpToggle);
    await tester.tap(mcpToggle);
    await tester.pumpAndSettle();
    final submit = find.byKey(const ValueKey('mobile-agent-task-submit'));
    await tester.ensureVisible(submit);
    await tester.tap(submit);
    await tester.pumpAndSettle();

    expect(
      port.operations.indexOf('AgentTask端末MCP能力取得'),
      lessThan(port.operations.indexOf('対話開始')),
    );
    expect(
      port.operations.indexOf('AgentTask端末MCP能力取得'),
      lessThan(port.operations.indexOf('AgentTask端末申請')),
    );
    expect(
      port.submittedPayload?['mcp_capability_snapshot_hash'],
      _mcpCapabilityHash,
    );
    expect(find.textContaining('申請をBrokerへ送信しました'), findsOneWidget);
    controller.dispose();
  });

  testWidgets('MCP能力を確認できない場合はSessionもTask申請も作らない', (tester) async {
    tester.view.physicalSize = const Size(800, 1200);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final port = _McpTaskPort(capabilityAvailable: false);
    final controller = DeviceLinkController(nativePort: port)..ready = true;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: MobileAgentTaskScreen(
            controller: controller,
            connected: true,
            active: true,
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    final instruction = find.byKey(
      const ValueKey('mobile-agent-task-instruction'),
    );
    await tester.ensureVisible(instruction);
    await tester.enterText(instruction, 'MCP能力を使って調査する');
    final mcpToggle = find.byKey(
      const ValueKey('mobile-agent-task-mcp-capabilities'),
    );
    await tester.ensureVisible(mcpToggle);
    await tester.tap(mcpToggle);
    await tester.pumpAndSettle();
    final submit = find.byKey(const ValueKey('mobile-agent-task-submit'));
    await tester.ensureVisible(submit);
    await tester.tap(submit);
    await tester.pumpAndSettle();

    expect(port.operations, contains('AgentTask端末MCP能力取得'));
    expect(port.operations, isNot(contains('対話開始')));
    expect(port.operations, isNot(contains('AgentTask端末申請')));
    expect(find.textContaining('Task申請を送信しませんでした'), findsOneWidget);
    controller.dispose();
  });

  testWidgets('再接続後の新しい一覧を古い読取応答で上書きしない', (tester) async {
    final staleRuntimeResponse = Completer<Map<String, Object?>>();
    final firstRuntimeStarted = Completer<void>();
    final port = _LifecyclePort(
      runtimeId: 'runtime-current',
      workspaceId: 'workspace-current',
      firstRuntimeResponse: staleRuntimeResponse,
      firstRuntimeStarted: firstRuntimeStarted,
    );
    final controller = DeviceLinkController(nativePort: port)..ready = true;
    var connected = true;
    late StateSetter update;

    await tester.pumpWidget(
      MaterialApp(
        home: StatefulBuilder(
          builder: (context, setState) {
            update = setState;
            return Scaffold(
              body: MobileAgentTaskScreen(
                controller: controller,
                connected: connected,
                active: true,
              ),
            );
          },
        ),
      ),
    );
    await firstRuntimeStarted.future.timeout(const Duration(seconds: 2));

    update(() => connected = false);
    await tester.pump();
    update(() => connected = true);
    await tester.pump();
    await tester.pumpAndSettle();
    expect(find.text('runtime-current'), findsOneWidget);

    staleRuntimeResponse.complete(
      _response('実行系列挙', 'accepted', {
        '実行系': ['runtime-stale'],
      }),
    );
    await tester.pumpAndSettle();
    expect(find.text('runtime-current'), findsOneWidget);
    expect(find.text('runtime-stale'), findsNothing);
    controller.dispose();
  });

  testWidgets('切断中に残った状態読取が再接続後の操作を塞がない', (tester) async {
    final staleStatusResponse = Completer<Map<String, Object?>>();
    final statusRequestStarted = Completer<void>();
    final port = _LifecyclePort(
      runtimeId: 'runtime-current',
      workspaceId: 'workspace-current',
      requests: [_ownRequestRow()],
      statusResponse: staleStatusResponse,
      statusRequestStarted: statusRequestStarted,
    );
    final controller = DeviceLinkController(nativePort: port)..ready = true;
    var connected = true;
    late StateSetter update;
    tester.view.physicalSize = const Size(800, 1200);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      MaterialApp(
        home: StatefulBuilder(
          builder: (context, setState) {
            update = setState;
            return Scaffold(
              body: MobileAgentTaskScreen(
                controller: controller,
                connected: connected,
                active: true,
              ),
            );
          },
        ),
      ),
    );
    await tester.pumpAndSettle();
    final requestCard = find.byKey(
      const ValueKey('mobile-agent-task-$_requestId'),
    );
    await tester.tap(requestCard);
    await tester.pumpAndSettle();
    await tester.tap(find.text('Broker状態'));
    await tester.pump();
    expect(statusRequestStarted.isCompleted, isTrue);

    update(() => connected = false);
    await tester.pump();
    update(() => connected = true);
    await tester.pump();
    await tester.pumpAndSettle();
    expect(find.text('runtime-current'), findsOneWidget);

    await tester.tap(requestCard);
    await tester.pumpAndSettle();
    await tester.tap(find.text('Broker状態'));
    await tester.pumpAndSettle();
    expect(
      port.operations.where((operation) => operation == 'AgentTask端末状態'),
      hasLength(2),
    );
    expect(find.textContaining('Owner確認待ち'), findsOneWidget);

    staleStatusResponse.complete(
      _response('AgentTask端末状態', 'accepted', {
        '申請ID': _requestId,
        '状態': '開始候補',
      }),
    );
    await tester.pumpAndSettle();
    expect(find.textContaining('Owner確認待ち'), findsOneWidget);
    expect(find.text('開始候補'), findsNothing);
    controller.dispose();
  });

  testWidgets('別Controllerへ切り替えた後に旧Task状態を表示しない', (tester) async {
    final oldStatusResponse = Completer<Map<String, Object?>>();
    final oldStatusStarted = Completer<void>();
    final oldPort = _LifecyclePort(
      runtimeId: 'runtime-old',
      workspaceId: 'workspace-old',
      requests: [_ownRequestRow()],
      statusResponse: oldStatusResponse,
      statusRequestStarted: oldStatusStarted,
    );
    final newPort = _LifecyclePort(
      runtimeId: 'runtime-new',
      workspaceId: 'workspace-new',
    );
    final oldController = DeviceLinkController(nativePort: oldPort)
      ..ready = true;
    final newController = DeviceLinkController(nativePort: newPort)
      ..ready = true;
    var controller = oldController;
    late StateSetter update;
    tester.view.physicalSize = const Size(800, 1200);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      MaterialApp(
        home: StatefulBuilder(
          builder: (context, setState) {
            update = setState;
            return Scaffold(
              body: MobileAgentTaskScreen(
                controller: controller,
                connected: true,
                active: true,
              ),
            );
          },
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(oldPort.operations, ['実行系列挙', '作業領域一覧', 'AgentTask端末申請一覧']);
    expect(find.textContaining('取得できません'), findsNothing);
    final requestCard = find.byKey(
      const ValueKey('mobile-agent-task-$_requestId'),
    );
    expect(requestCard, findsOneWidget);
    await tester.tap(requestCard);
    await tester.pumpAndSettle();
    await tester.tap(find.text('Broker状態'));
    await tester.pump();
    expect(oldStatusStarted.isCompleted, isTrue);

    update(() => controller = newController);
    await tester.pump();
    await tester.pumpAndSettle();
    expect(find.text('runtime-new'), findsOneWidget);
    expect(find.textContaining(_requestId), findsNothing);

    oldStatusResponse.complete(
      _response('AgentTask端末状態', 'accepted', {
        '申請ID': _requestId,
        '状態': '開始候補',
      }),
    );
    await tester.pumpAndSettle();
    expect(find.text('runtime-new'), findsOneWidget);
    expect(find.textContaining(_requestId), findsNothing);
    expect(find.text('開始候補'), findsNothing);
    oldController.dispose();
    newController.dispose();
  });
}
