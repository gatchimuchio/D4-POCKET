import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_mobile/screens/mobile_agent_task.dart';
import 'package:gui_shell_mobile/services/device_link_client.dart';
import 'package:gui_shell_mobile/services/device_link_controller.dart';

const _sessionId = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';

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
}
