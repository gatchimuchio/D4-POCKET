import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/main.dart';
import 'package:gui_shell_desktop/screens/approval_center.dart';
import 'package:gui_shell_desktop/screens/authority_map.dart';
import 'package:gui_shell_desktop/screens/audit_viewer.dart';
import 'package:gui_shell_desktop/screens/dashboard.dart';
import 'package:gui_shell_desktop/screens/evidence_center.dart';
import 'package:gui_shell_desktop/screens/problems_panel.dart';
import 'package:gui_shell_desktop/screens/recovery_center.dart';
import 'package:gui_shell_desktop/screens/runtime_center.dart';
import 'package:gui_shell_desktop/screens/settings.dart';
import 'package:gui_shell_desktop/screens/setup_doctor.dart';
import 'package:gui_shell_desktop/screens/shared.dart';
import 'package:gui_shell_desktop/screens/trust_center.dart';
import 'package:gui_shell_desktop/services/broker_client.dart';
import 'package:gui_shell_desktop/services/shell_core_client.dart';

void main() {
  testWidgets('GUI Shell desktop app smoke test', (WidgetTester tester) async {
    await tester.pumpWidget(const GuiShellDesktopApp());

    expect(find.byType(MaterialApp), findsOneWidget);
    expect(find.byType(NavigationRail), findsOneWidget);
    expect(find.text('Dashboard'), findsWidgets);
    expect(find.text('Trust'), findsOneWidget);
    expect(find.text('Authority'), findsOneWidget);
  });

  testWidgets('Windows acceptance surfaces expose semantic labels',
      (WidgetTester tester) async {
    final semantics = tester.ensureSemantics();
    try {
      await tester.pumpWidget(const GuiShellDesktopApp());

      for (final label in [
        'Dashboard',
        'NavigationRail',
        'Runtime Status',
        'Invariant Status',
      ]) {
        expect(
          find.bySemanticsIdentifier(surfaceSemanticsIdentifier(label)),
          findsOneWidget,
        );
      }
      expect(find.bySemanticsLabel(RegExp('Dashboard')), findsWidgets);
      expect(find.bySemanticsLabel(RegExp('NavigationRail')), findsOneWidget);
      expect(find.bySemanticsLabel(RegExp('Runtime Status')), findsWidgets);
      expect(find.bySemanticsLabel(RegExp('Invariant Status')), findsWidgets);
    } finally {
      semantics.dispose();
    }
  });

  testWidgets('Dashboard shows Phase A complete and Phase B complete',
      (WidgetTester tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: Dashboard(client: ShellCoreClient.mock())),
      ),
    );

    expect(find.textContaining('Phase A: complete'), findsOneWidget);
    expect(find.textContaining('Phase B: complete'), findsOneWidget);
    expect(find.textContaining('Completed product release: not claimed'),
        findsOneWidget);
  });

  testWidgets('Status bar shows Phase B owner-use and release not claimed',
      (WidgetTester tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ShellStatusBar(snapshot: ShellCoreClient.mock().getSnapshot()),
        ),
      ),
    );

    expect(find.textContaining('Phase: B owner-use'), findsOneWidget);
    expect(find.textContaining('Release: not claimed'), findsOneWidget);
  });

  testWidgets('Command palette searches and navigates',
      (WidgetTester tester) async {
    await tester.pumpWidget(const GuiShellDesktopApp());

    await tester.sendKeyDownEvent(LogicalKeyboardKey.control);
    await tester.sendKeyEvent(LogicalKeyboardKey.keyK);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.control);
    await tester.pumpAndSettle();

    await tester.enterText(find.byType(TextField), 'problems');
    await tester.pumpAndSettle();
    expect(find.text('Open Problems Panel'), findsOneWidget);
    await tester.tap(find.text('Open Problems Panel'));
    await tester.pumpAndSettle();

    expect(find.text('Problems Panel'), findsWidgets);
  });

  testWidgets('Problems panel shows release blockers without Phase B failure',
      (WidgetTester tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: ProblemsPanel(client: ShellCoreClient.mock())),
      ),
    );

    expect(find.text('Problems Panel'), findsOneWidget);
    expect(find.textContaining('measured Windows installed-path first-run'),
        findsOneWidget);
    expect(find.textContaining('release_blocker'), findsWidgets);
    expect(find.text('Recovery'), findsOneWidget);
    expect(find.text('Blocks Owner Use'), findsOneWidget);
    expect(find.text('Blocks Product Release'), findsOneWidget);
    expect(find.textContaining('recover-windows-evidence'), findsWidgets);
    expect(find.textContaining('release_evidence/windows_installed_smoke.json'),
        findsWidgets);
    expect(find.textContaining('without making Phase B owner-use fail'),
        findsOneWidget);
  });

  testWidgets('Evidence center shows strict Windows expected failure',
      (WidgetTester tester) async {
    final releaseEvidence =
        File('release_evidence/windows_installed_smoke.json');
    final existedBefore = releaseEvidence.existsSync();
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: EvidenceCenter(client: ShellCoreClient.mock())),
      ),
    );

    expect(find.text('Evidence Center'), findsOneWidget);
    expect(find.textContaining('strict_windows_release: expected fail'),
        findsOneWidget);
    expect(
        find.textContaining(
            'missing measured Windows evidence: release_blocker'),
        findsOneWidget);
    expect(releaseEvidence.existsSync(), existedBefore);
  });

  testWidgets(
      'Evidence center exposes display-only export and snapshot compare',
      (WidgetTester tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: EvidenceCenter(client: ShellCoreClient.mock())),
      ),
    );

    expect(find.text('Evidence Bundle Export'), findsOneWidget);
    expect(find.text('Snapshot Import / Export'), findsOneWidget);
    expect(find.text('Copy Validation Summary'), findsOneWidget);
    expect(find.text('Preview Import / Compare'), findsOneWidget);
  });

  testWidgets('Recovery playbook marks Windows evidence safe for Phase B',
      (WidgetTester tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: RecoveryCenter(client: ShellCoreClient.mock())),
      ),
    );

    expect(find.text('Recovery Playbook'), findsOneWidget);
    expect(
        find.textContaining('measured Windows installed-path evidence missing'),
        findsOneWidget);
    expect(find.textContaining('true'), findsWidgets);
    expect(find.text('Command'), findsOneWidget);
    expect(find.text('Path'), findsOneWidget);
  });

  testWidgets('Settings screen searches and filters phase release settings',
      (WidgetTester tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: SettingsScreen(client: ShellCoreClient.mock())),
      ),
    );

    expect(find.text('Settings'), findsOneWidget);
    await tester.enterText(find.byType(TextField), 'release');
    await tester.pumpAndSettle();

    expect(find.textContaining('release.state'), findsOneWidget);
    expect(find.textContaining('not claimed'), findsWidgets);
  });

  testWidgets('Trust and Authority surfaces are restored',
      (WidgetTester tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: TrustCenter(client: ShellCoreClient.mock())),
      ),
    );

    expect(find.text('Trust Center'), findsOneWidget);
    expect(find.textContaining('workspace_trust'), findsOneWidget);
    expect(find.textContaining('Shell Core capability'), findsOneWidget);

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: AuthorityMap(client: ShellCoreClient.mock())),
      ),
    );

    expect(find.text('Authority Map'), findsOneWidget);
    expect(find.textContaining('filesystem.write'), findsWidgets);
    expect(find.textContaining('Authority decisions remain in Shell Core'),
        findsOneWidget);
  });

  testWidgets('Runtime detail pane and audit filters are visible',
      (WidgetTester tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: RuntimeCenter(client: ShellCoreClient.mock())),
      ),
    );

    expect(find.text('Runtime Detail'), findsOneWidget);
    expect(find.textContaining('runtime_id: blue_tanuki'), findsOneWidget);
    expect(find.textContaining('capabilities:'), findsOneWidget);

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: AuditViewer(client: ShellCoreClient.mock())),
      ),
    );

    expect(find.text('Audit Timeline Filters'), findsOneWidget);
    expect(find.textContaining('hash_chain_status'), findsOneWidget);
    expect(find.text('Copy'), findsOneWidget);
  });

  test('Setup Doctor client surface is structured and non-authoritative', () {
    final snapshot = ShellCoreClient.local().getSnapshot();

    expect(ShellCoreClient.local().mode, 'local');
    expect(ShellCoreClient.mock().mode, 'mock');
    expect(snapshot.setupDoctorChecks, isNotEmpty);
    expect(snapshot.installerGrantsAuthority, isFalse);
    expect(snapshot.installerSilentlyApprovesPermissions, isFalse);
    expect(
      snapshot.setupDoctorChecks.where((check) => check.grantsAuthority),
      isEmpty,
    );
  });

  test('product client renders broker-mediated authority snapshot', () async {
    final transport = _FakeBrokerTransport([
      _brokerHealthResponse(),
      _brokerAcceptedBody('normalize_payload', {'quarantined': false}),
      _brokerAcceptedBody('content_projection', {
        'redacted_payload': {'path': 'notes/today.md', 'content': '[redacted]'}
      }),
      _brokerAcceptedBody('approval_edit', {
        'ok': false,
        'error': 'field is not editable: payload_hash',
      }),
      _brokerCommandSuspendedResponse(),
    ]);

    final client = await ShellCoreClient.product(transport: transport);
    final snapshot = client.getSnapshot();

    expect(client.mode, 'broker');
    expect(snapshot.snapshotSource, 'broker');
    expect(snapshot.snapshotPath, 'broker://127.0.0.1/health');
    expect(snapshot.operationStatus.runtimeStatus, 'suspend');
    expect(snapshot.pendingApprovals.single.projectedContent['content'],
        '[redacted]');
    expect(snapshot.pendingApprovals.single.projectedContent.values,
        isNot(contains('hidden')));
    expect(
      snapshot.setupDoctorChecks.any((check) =>
          check.checkId == 'broker.protected_field_edit' &&
          check.status == 'pass'),
      isTrue,
    );
    expect(
      snapshot.problems.any((problem) =>
          problem.item == 'broker command dispatch suspended' &&
          problem.classification == 'release_blocker'),
      isTrue,
    );
    expect(
      snapshot.problems.any(
          (problem) => problem.requiredAction.toLowerCase().contains('python')),
      isFalse,
    );
    expect(transport.operations, [
      'health',
      'normalize_payload',
      'content_projection',
      'approval_edit',
      'command_envelope',
    ]);
  });

  test('product client fails closed when broker is unavailable', () async {
    final client = await ShellCoreClient.product(
      transport: const _FailingBrokerTransport('broker unavailable'),
    );
    final snapshot = client.getSnapshot();

    expect(client.mode, 'broker_unavailable');
    expect(snapshot.snapshotSource, 'broker_unavailable');
    expect(snapshot.operationStatus.runtimeStatus, 'suspend');
    expect(snapshot.operationStatus.trustStatus, 'blocked');
    expect(snapshot.pendingApprovals, isEmpty);
    expect(snapshot.problems.single.classification, 'release_blocker');
    expect(snapshot.problems.single.blocksRelease, isTrue);
    expect(
        snapshot.setupDoctorChecks
            .where((check) => check.checkId == 'broker.fail_closed'),
        isNotEmpty);
  });

  test('product client fails closed on authentication rejection', () async {
    final client = await ShellCoreClient.product(
      transport: _FakeBrokerTransport([
        _brokerRejectedResponse(
          'health',
          'broker_authentication_failed',
          'broker IPC authentication failed',
        ),
      ]),
    );

    expect(client.mode, 'broker_unavailable');
    expect(client.getSnapshot().operationStatus.runtimeStatus, 'suspend');
  });

  test('product client fails closed on stale broker session', () async {
    final client = await ShellCoreClient.product(
      transport: _FakeBrokerTransport([
        _brokerHealthResponse(),
        _brokerRejectedResponse(
          'normalize_payload',
          'broker_stale_session',
          'broker session is stale',
        ),
      ]),
    );

    expect(client.mode, 'broker_unavailable');
    expect(client.getSnapshot().operationStatus.trustStatus, 'blocked');
  });

  test('product client fails closed on malformed broker response', () async {
    final client = await ShellCoreClient.product(
      transport: _FakeBrokerTransport([
        {'status': 'accepted', 'operation': 'health'},
      ]),
    );

    expect(client.mode, 'broker_unavailable');
    expect(client.getSnapshot().snapshotSource, 'broker_unavailable');
  });

  testWidgets('Setup Doctor shows lightweight environment snapshot',
      (WidgetTester tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: SetupDoctor(client: ShellCoreClient.mock())),
      ),
    );

    expect(find.text('Environment Snapshot'), findsOneWidget);
    expect(find.textContaining('network_exposure:'), findsOneWidget);
    expect(find.textContaining('config/snapshot path:'), findsOneWidget);
  });

  test('local client reads structured snapshot data', () {
    final tempDir = Directory.systemTemp.createTempSync('gui-shell-test-');
    addTearDown(() => tempDir.deleteSync(recursive: true));
    final snapshotFile = File('${tempDir.path}/shell_snapshot.json');
    snapshotFile.writeAsStringSync(jsonEncode({
      'runtimes': [
        {
          'runtime_id': 'runtime-from-json',
          'name': 'Runtime From Json',
          'status': 'ready',
          'adapter_id': 'adapter-from-json',
          'diagnostic_summary': 'loaded from local snapshot'
        }
      ],
      'agent_sessions': [],
      'permissions': [],
      'pending_approvals': [],
      'audit_events': [],
      'recovery_actions': [],
      'invariant_flags': {
        'flutter_imported_by_shell_core': true,
        'blue_tanuki_imported_by_shell_core': false
      },
      'setup_doctor_status': 'pass',
      'installer_grants_authority': false,
      'installer_silently_approves_permissions': false,
      'setup_doctor_checks': [
        {
          'check_id': 'local.json',
          'status': 'pass',
          'message': 'Loaded local diagnostic JSON',
          'recovery_instruction': null,
          'grants_authority': false
        }
      ]
    }));

    final client = ShellCoreClient.local(snapshotPath: snapshotFile.path);
    final snapshot = client.getSnapshot();

    expect(client.mode, 'local');
    expect(snapshot.runtimes.single.runtimeId, 'runtime-from-json');
    expect(snapshot.setupDoctorChecks.single.checkId, 'local.json');
    expect(snapshot.invariantFlags['flutter_imported_by_shell_core'], isTrue);
    expect(snapshot.snapshotSource, 'local');
    expect(snapshot.operationStatus.releaseState, 'not claimed');
  });

  test('local snapshot fallback does not claim release-ready', () {
    final tempDir = Directory.systemTemp.createTempSync('gui-shell-missing-');
    addTearDown(() => tempDir.deleteSync(recursive: true));
    final missingPath = '${tempDir.path}/missing_snapshot.json';

    final snapshot =
        ShellCoreClient.local(snapshotPath: missingPath).getSnapshot();

    expect(snapshot.snapshotSource, 'fallback');
    expect(snapshot.snapshotFreshness, 'missing');
    expect(snapshot.operationStatus.releaseState, 'not claimed');
    expect(
        snapshot.problems
            .any((problem) => problem.item == 'local snapshot missing'),
        isTrue);
  });

  test('local snapshot parse failure falls back safely', () {
    final tempDir = Directory.systemTemp.createTempSync('gui-shell-bad-json-');
    addTearDown(() => tempDir.deleteSync(recursive: true));
    final snapshotFile = File('${tempDir.path}/bad_snapshot.json')
      ..writeAsStringSync('{bad json');

    final snapshot =
        ShellCoreClient.local(snapshotPath: snapshotFile.path).getSnapshot();

    expect(snapshot.snapshotSource, 'fallback');
    expect(snapshot.snapshotFreshness, 'parse failed');
    expect(snapshot.operationStatus.releaseState, 'not claimed');
    expect(
        snapshot.problems
            .any((problem) => problem.item == 'local snapshot parse failed'),
        isTrue);
  });

  testWidgets('Setup Doctor UI displays local diagnostic data',
      (WidgetTester tester) async {
    final tempDir = Directory.systemTemp.createTempSync('gui-shell-ui-test-');
    addTearDown(() => tempDir.deleteSync(recursive: true));
    final snapshotFile = File('${tempDir.path}/shell_snapshot.json');
    snapshotFile.writeAsStringSync(jsonEncode({
      'runtimes': [
        {
          'runtime_id': 'runtime-ui-json',
          'name': 'Runtime UI Json',
          'status': 'ready',
          'adapter_id': 'adapter-ui-json',
          'diagnostic_summary': 'loaded from local snapshot'
        }
      ],
      'agent_sessions': [],
      'permissions': [],
      'pending_approvals': [],
      'audit_events': [],
      'recovery_actions': [],
      'invariant_flags': {},
      'setup_doctor_status': 'pass',
      'installer_grants_authority': false,
      'installer_silently_approves_permissions': false,
      'setup_doctor_checks': [
        {
          'check_id': 'local.ui',
          'status': 'pass',
          'message': 'UI loaded local diagnostic JSON',
          'recovery_instruction': null,
          'grants_authority': false
        }
      ]
    }));

    await tester.pumpWidget(
      MaterialApp(
        home: SetupDoctor(
          client: ShellCoreClient.local(snapshotPath: snapshotFile.path),
        ),
      ),
    );

    expect(find.textContaining('local.ui: pass'), findsOneWidget);
    expect(find.textContaining('runtime-ui-json: ready'), findsOneWidget);
  });

  testWidgets('Approval Center does not expose hidden full payload',
      (WidgetTester tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: ApprovalCenter(client: ShellCoreClient.local())),
      ),
    );

    expect(find.textContaining('Visibility: redacted'), findsOneWidget);
    expect(find.textContaining('[redacted]'), findsOneWidget);
    expect(find.textContaining('hello'), findsNothing);
  });
}

class _FakeBrokerTransport implements BrokerTransport {
  _FakeBrokerTransport(this._responses);

  final List<Map<String, Object?>> _responses;
  final List<String> operations = [];

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    operations.add(operation);
    if (_responses.isEmpty) {
      throw BrokerClientException('no fake broker response for $operation');
    }
    return _responses.removeAt(0);
  }
}

class _FailingBrokerTransport implements BrokerTransport {
  const _FailingBrokerTransport(this.message);

  final String message;

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) {
    throw BrokerClientException(message);
  }
}

Map<String, Object?> _brokerHealthResponse() {
  return {
    'request_id': 'test-health',
    'operation': 'health',
    'status': 'accepted',
    'evidence_source': 'LIVE_RUNTIME',
    'audit_event_id': 'audit-health',
    'error': null,
    'health': {
      'broker_id': 'gui-shell-rust-broker',
      'status': 'ready',
      'boundary_role': 'rust_security_broker_candidate',
      'authority_cutover_status': 'not_active',
      'command_dispatch_enabled': false,
      'audit_append_enabled': true,
      'audit_persistence': 'durable_file_store',
      'replay_persistence': 'durable_file_store',
      'session_persistence': 'durable_file_store',
      'persistence_required': true,
      'persistence_ready': true,
      'evidence_source': 'LIVE_RUNTIME',
    },
    'body': null,
    'shutdown_requested': false,
  };
}

Map<String, Object?> _brokerAcceptedBody(
  String operation,
  Map<String, Object?> body,
) {
  return {
    'request_id': 'test-$operation',
    'operation': operation,
    'status': 'accepted',
    'evidence_source': 'LIVE_RUNTIME',
    'audit_event_id': 'audit-$operation',
    'error': null,
    'health': null,
    'body': body,
    'shutdown_requested': false,
  };
}

Map<String, Object?> _brokerCommandSuspendedResponse() {
  return {
    'request_id': 'test-command-envelope',
    'operation': 'command_envelope',
    'status': 'suspended',
    'evidence_source': 'INTERNAL_STATE',
    'audit_event_id': 'audit-command-envelope',
    'error': {
      'code': 'broker_command_dispatch_disabled',
      'message': 'external command dispatch is disabled',
      'recoverable': true,
      'audit_event_required': true,
      'fail_closed': true,
    },
    'health': null,
    'body': {
      'dispatch_enabled': false,
      'eligibility': {'allowed': true, 'errors': []},
    },
    'shutdown_requested': false,
  };
}

Map<String, Object?> _brokerRejectedResponse(
  String operation,
  String code,
  String message,
) {
  return {
    'request_id': 'test-$operation-rejected',
    'operation': operation,
    'status': 'rejected',
    'evidence_source': 'INTERNAL_STATE',
    'audit_event_id': 'audit-$operation-rejected',
    'error': {
      'code': code,
      'message': message,
      'recoverable': true,
      'audit_event_required': true,
      'fail_closed': true,
    },
    'health': null,
    'body': null,
    'shutdown_requested': false,
  };
}
