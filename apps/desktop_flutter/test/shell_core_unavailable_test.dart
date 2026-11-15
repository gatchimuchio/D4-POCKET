import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/services/broker_client.dart'
    show BrokerTransport;
import 'package:gui_shell_desktop/screens/evidence_center.dart';
import 'package:gui_shell_desktop/services/shell_core_client.dart';

void main() {
  test('Broker接続失敗で例外本文と合成証拠をsnapshotへ含めない', () async {
    const secretMarker = 'BROKER_FAILURE_SECRET_MARKER';
    final client = await ShellCoreClient.product(
      transport: const _FailingBrokerTransport(secretMarker),
    );
    final snapshot = client.getSnapshot();

    expect(client.mode, 'broker_unavailable');
    expect(snapshot.snapshotSource, 'broker_unavailable');
    expect(snapshot.operationStatus.runtimeStatus, 'suspend');
    expect(snapshot.operationStatus.trustStatus, 'blocked');
    expect(snapshot.pendingApprovals, isEmpty);
    expect(snapshot.auditEvents, isEmpty);
    expect(snapshot.auditChainStatus, 'not_ready');
    expect(snapshot.evidence, isEmpty);
    expect(snapshot.evidenceSummary.missingSetupDoctorEvidence, isTrue);
    expect(
      snapshot.runtimes.single.diagnosticSummary,
      '認証済みBroker IPCを確立できず、最新のBroker状態を取得できません。',
    );
    expect(
      snapshot.runtimes.single.diagnosticSummary,
      isNot(contains(secretMarker)),
    );
    expect(snapshot.problems.single.blocksOwnerUse, isTrue);
    expect(snapshot.problems.single.requiredAction, contains('Rustブローカー'));
  });

  testWidgets('証拠センターがBroker未接続を合成証拠へ昇格しない', (
    WidgetTester tester,
  ) async {
    final client = await ShellCoreClient.product(
      transport: const _FailingBrokerTransport('broker unavailable'),
    );
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: EvidenceCenter(client: client)),
      ),
    );

    expect(find.text('証拠なし'), findsOneWidget);
    expect(
      find.text('非合成の環境診断証拠の不足: release_blocker'),
      findsOneWidget,
    );
    expect(find.textContaining('broker-unavailable-fail-closed'), findsNothing);
  });
}

class _FailingBrokerTransport implements BrokerTransport {
  const _FailingBrokerTransport(this.message);

  final String message;

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) {
    throw StateError(message);
  }
}
