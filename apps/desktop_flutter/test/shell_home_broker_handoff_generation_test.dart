import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/main.dart' show ShellHomePage;
import 'package:gui_shell_desktop/services/broker_client.dart'
    show brokerPayloadHash;
import 'package:gui_shell_desktop/services/shell_core_client.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart' show BrokerTransport;

void main() {
  testWidgets('Broker切替時に未消費のAI Edit提案を新Brokerへ持ち越さない', (tester) async {
    tester.view.physicalSize = const Size(1440, 1000);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final oldTransport = _AiEditProposalTransport();
    final newTransport = _AiEditProposalTransport();
    Widget app(ShellCoreClient client) => MaterialApp(
          home: ShellHomePage(client: client),
        );

    await tester.pumpWidget(app(ShellCoreClient.mock(transport: oldTransport)));
    await tester.pumpAndSettle();

    final rail = tester.widget<NavigationRail>(find.byType(NavigationRail));
    final settingsIndex = rail.destinations.indexWhere((destination) {
      final label = destination.label;
      return label is Text && label.data == '設定';
    });
    expect(settingsIndex, greaterThanOrEqualTo(0));
    rail.onDestinationSelected!(settingsIndex);
    await tester.pumpAndSettle();

    final expectedChanges = find.byKey(
      const ValueKey('compose-ai-edit-expected-changes'),
    );
    await tester.ensureVisible(expectedChanges);
    await tester.enterText(expectedChanges, 'Broker A由来の未信頼提案');
    final submit = find.byKey(const ValueKey('compose-ai-edit-button'));
    await tester.ensureVisible(submit);
    await tester.tap(submit);
    await tester.pumpAndSettle();

    final review = find.byKey(
      const ValueKey('compose-ai-edit-review-button'),
    );
    await tester.ensureVisible(review);
    await tester.tap(review);
    await tester.pumpAndSettle();
    await tester.tap(
      find.byKey(const ValueKey('compose-ai-edit-review-confirm')),
    );
    await tester.pumpAndSettle();

    final proposalPanel = find.byKey(
      const ValueKey('agent-ai-edit-task-draft'),
    );
    expect(proposalPanel, findsOneWidget);
    expect(oldTransport.requests, contains('GUI Shell編集提案'));

    await tester.pumpWidget(app(ShellCoreClient.mock(transport: newTransport)));
    await tester.pumpAndSettle();

    expect(proposalPanel, findsNothing);
    expect(
      newTransport.requests,
      isNot(contains('GUI Shell編集提案')),
      reason: '切替先Brokerへ旧Broker由来のproposalを再送しない',
    );
    expect(tester.takeException(), isNull);
  });
}

class _AiEditProposalTransport implements BrokerTransport {
  final List<String> requests = [];

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    requests.add(operation);
    if (operation != 'GUI Shell編集提案') {
      return {
        'request_id': 'fixture-request',
        'operation': operation,
        'status': 'rejected',
        'evidence_source': 'INTERNAL_STATE',
        'audit_event_id': 'fixture-rejected',
        'error': null,
        'body': <String, Object?>{},
        'shutdown_requested': false,
      };
    }

    final submitted = payload ?? const <String, Object?>{};
    final expectedChanges = submitted['expected_changes']! as List;
    return {
      'request_id': 'fixture-ai-edit-request',
      'operation': operation,
      'status': 'accepted',
      'evidence_source': 'INTERNAL_STATE',
      'audit_event_id': 'audit-broker-a-ai-edit',
      'error': null,
      'body': {
        'version': 1,
        'operation': operation,
        'status': 'accepted',
        'edit_id': submitted['edit_id'],
        'scope': submitted['scope'],
        'target_paths': submitted['target_paths'],
        'expected_change_count': expectedChanges.length,
        'execution_mode': 'proposal_only',
        'review_required': true,
        'apply_status': 'not_started',
        'files_written': false,
        'permission_generated': false,
        'approval_state': 'owner_review_required',
        'authority_strip': true,
        'evidence_source': 'INTERNAL_STATE',
        'instruction_hash': brokerPayloadHash(submitted),
        'audit_id': 'audit-broker-a-ai-edit',
      },
      'shutdown_requested': false,
    };
  }
}
