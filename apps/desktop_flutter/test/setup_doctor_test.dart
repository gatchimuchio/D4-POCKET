import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/screens/audit_viewer.dart';
import 'package:gui_shell_desktop/screens/problems_panel.dart';
import 'package:gui_shell_desktop/screens/recovery_center.dart';
import 'package:gui_shell_desktop/screens/setup_doctor.dart';
import 'package:gui_shell_desktop/services/broker_client.dart';
import 'package:gui_shell_desktop/services/shell_core_client.dart';

void main() {
  testWidgets('Setup DoctorはBrokerから最新報告と証拠種別・監査IDを再取得する', (tester) async {
    final transport = _SetupDoctorTransport();
    final client = await ShellCoreClient.product(transport: transport);
    transport.nextReportResponse = _setupDoctorResponse(
      status: 'pass',
      message: 'Brokerが再取得した最新状態です。',
      auditId: 'audit-setup-doctor-refresh',
    );

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: SetupDoctor(client: client)),
      ),
    );
    await tester.tap(find.byKey(const ValueKey('setup-doctor-refresh')));
    await tester.pumpAndSettle();

    expect(find.text('診断状態: 正常'), findsOneWidget);
    expect(find.text('Brokerが再取得した最新状態です。'), findsWidgets);
    expect(find.text('証拠種別: Broker実行時観測'), findsWidgets);
    expect(find.text('監査イベントID: audit-setup-doctor-refresh'), findsOneWidget);
    expect(
      find.text('診断報告ID: setup-doctor-${'a' * 64}'),
      findsOneWidget,
    );
    expect(find.textContaining('起動時のSnapshot'), findsNothing);
    final requests = transport.requests
        .where((request) => request['operation'] == 'Setup Doctor報告取得')
        .toList();
    expect(requests, hasLength(2));
    expect(requests.last['payload'], const <String, Object?>{'version': 1});
  });

  testWidgets('Setup Doctor再取得失敗時は前回結果を捨てunknownに閉じる', (tester) async {
    final transport = _SetupDoctorTransport();
    final client = await ShellCoreClient.product(transport: transport);
    transport.nextReportResponse = {
      'request_id': 'test-setup-doctor-rejected',
      'operation': 'Setup Doctor報告取得',
      'status': 'rejected',
      'evidence_source': 'LIVE_RUNTIME',
      'audit_event_id': 'audit-setup-doctor-rejected',
      'error': {
        'code': 'fixture_failure',
        'message': 'SENSITIVE_SYNTHETIC_BROKER_DETAIL',
      },
      'body': null,
    };

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: SetupDoctor(client: client)),
      ),
    );
    await tester.tap(find.byKey(const ValueKey('setup-doctor-refresh')));
    await tester.pumpAndSettle();

    expect(find.text('診断状態: 不明'), findsOneWidget);
    expect(find.text('最新の診断を取得できません。Broker接続を確認して再試行してください。'), findsOneWidget);
    expect(find.text('起動時の診断です。'), findsNothing);
    expect(
        find.textContaining('SENSITIVE_SYNTHETIC_BROKER_DETAIL'), findsNothing);
    expect(find.textContaining('監査イベントID:'), findsNothing);
  });

  testWidgets('LIVE_RUNTIME以外のSetup Doctor応答は表示しない', (tester) async {
    final transport = _SetupDoctorTransport();
    final client = await ShellCoreClient.product(transport: transport);
    final wrongEvidence = _setupDoctorResponse(
      status: 'pass',
      message: '誤った証拠種別の結果です。',
      auditId: 'audit-setup-doctor-wrong-evidence',
    );
    wrongEvidence['evidence_source'] = 'INTERNAL_STATE';
    transport.nextReportResponse = wrongEvidence;

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: SetupDoctor(client: client)),
      ),
    );
    await tester.tap(find.byKey(const ValueKey('setup-doctor-refresh')));
    await tester.pumpAndSettle();

    expect(find.text('診断状態: 不明'), findsOneWidget);
    expect(find.text('誤った証拠種別の結果です。'), findsNothing);
    expect(find.text('起動時の診断です。'), findsNothing);
    expect(
      find.textContaining('audit-setup-doctor-wrong-evidence'),
      findsNothing,
    );
  });

  test('Scope/Severity不一致・片側欠落はunknownへ閉じ旧形式はunknownのまま保つ', () async {
    final transport = _SetupDoctorTransport();
    final client = await ShellCoreClient.product(transport: transport);
    final inconsistent = _setupDoctorResponse(
      status: 'pass',
      message: '不一致の分類を含む応答です。',
      auditId: 'audit-setup-doctor-inconsistent-classification',
    );
    final inconsistentBody =
        Map<String, Object?>.from(inconsistent['body']! as Map);
    final inconsistentChecks = List<Map<String, Object?>>.from(
      (inconsistentBody['checks']! as List).map(
        (check) => Map<String, Object?>.from(check as Map),
      ),
    );
    inconsistentChecks.first['scope'] = 'audit';
    inconsistentBody['checks'] = inconsistentChecks;
    inconsistent['body'] = inconsistentBody;
    transport.nextReportResponse = inconsistent;

    final rejected = await client.refreshSetupDoctorReport();
    expect(rejected.status, 'unknown');
    expect(rejected.checks.first.scope, 'unknown');
    expect(rejected.checks.first.severity, 'unknown');

    final wrongSeverity = _setupDoctorResponse(
      status: 'pass',
      message: 'statusと異なる重大度を含む応答です。',
      auditId: 'audit-setup-doctor-wrong-severity',
    );
    final wrongSeverityBody =
        Map<String, Object?>.from(wrongSeverity['body']! as Map);
    final wrongSeverityChecks = List<Map<String, Object?>>.from(
      (wrongSeverityBody['checks']! as List).map(
        (check) => Map<String, Object?>.from(check as Map),
      ),
    );
    wrongSeverityChecks.first['severity'] = 'critical';
    wrongSeverityBody['checks'] = wrongSeverityChecks;
    wrongSeverity['body'] = wrongSeverityBody;
    transport.nextReportResponse = wrongSeverity;

    final rejectedSeverity = await client.refreshSetupDoctorReport();
    expect(rejectedSeverity.status, 'unknown');
    expect(rejectedSeverity.checks.first.severity, 'unknown');

    final partial = _setupDoctorResponse(
      status: 'pass',
      message: '分類片側が欠落した応答です。',
      auditId: 'audit-setup-doctor-partial-classification',
    );
    final partialBody = Map<String, Object?>.from(partial['body']! as Map);
    final partialChecks = List<Map<String, Object?>>.from(
      (partialBody['checks']! as List).map(
        (check) => Map<String, Object?>.from(check as Map),
      ),
    );
    partialChecks.first.remove('scope');
    partialBody['checks'] = partialChecks;
    partial['body'] = partialBody;
    transport.nextReportResponse = partial;

    final rejectedPartial = await client.refreshSetupDoctorReport();
    expect(rejectedPartial.status, 'unknown');

    final legacy = _setupDoctorResponse(
      status: 'warning',
      message: '分類を持たない旧形式の報告です。',
      unknownFirstCheck: true,
      auditId: 'audit-setup-doctor-legacy',
    );
    final legacyBody = Map<String, Object?>.from(legacy['body']! as Map);
    final legacyChecks = List<Map<String, Object?>>.from(
      (legacyBody['checks']! as List).map(
        (check) => Map<String, Object?>.from(check as Map),
      ),
    );
    for (final check in legacyChecks) {
      check.remove('scope');
      check.remove('severity');
    }
    legacyBody['checks'] = legacyChecks;
    legacy['body'] = legacyBody;
    transport.nextReportResponse = legacy;

    final compatible = await client.refreshSetupDoctorReport();
    expect(compatible.status, 'warning');
    expect(compatible.checks.first.scope, 'unknown');
    expect(compatible.checks.first.severity, 'unknown');
  });

  testWidgets('Broker client切替後に旧報告の遅延応答を表示しない', (tester) async {
    final firstTransport = _SetupDoctorTransport();
    final firstClient =
        await ShellCoreClient.product(transport: firstTransport);
    final secondClient = await ShellCoreClient.product(
      transport: _SetupDoctorTransport(initialMessage: '切替先BrokerのSnapshotです。'),
    );
    final pending = Completer<Map<String, Object?>>();
    firstTransport.holdNextReport = pending;

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: SetupDoctor(client: firstClient)),
      ),
    );
    await tester.tap(find.byKey(const ValueKey('setup-doctor-refresh')));
    await tester.pump();
    expect(
      firstTransport.requests
          .where((request) => request['operation'] == 'Setup Doctor報告取得'),
      hasLength(2),
    );

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: SetupDoctor(client: secondClient)),
      ),
    );
    pending.complete(
      _setupDoctorResponse(
        status: 'pass',
        message: '旧Brokerの遅延報告です。',
        auditId: 'audit-stale-setup-doctor',
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('切替先BrokerのSnapshotです。'), findsWidgets);
    expect(find.text('旧Brokerの遅延報告です。'), findsNothing);
  });

  testWidgets('Broker以外のSnapshotでは再取得要求を送らない', (tester) async {
    final transport = _SetupDoctorTransport();
    final client = ShellCoreClient.mock(transport: transport);

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: SetupDoctor(client: client)),
      ),
    );

    final button = tester.widget<OutlinedButton>(
      find.byKey(const ValueKey('setup-doctor-refresh')),
    );
    expect(button.onPressed, isNull);
    expect(find.textContaining('試験用Snapshot'), findsOneWidget);
    expect(transport.requests, isEmpty);
  });

  testWidgets('問題一覧はactive時に現在のBroker診断を取得する', (tester) async {
    final transport = _SetupDoctorTransport();
    final client = await ShellCoreClient.product(transport: transport);
    transport.nextReportResponse = _setupDoctorResponse(
      status: 'warning',
      message: '現在のBroker診断で確認した状態です。',
      unknownFirstCheck: true,
      auditId: 'audit-problems-current',
    );

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: ProblemsPanel(client: client, active: false)),
      ),
    );
    await tester.pumpAndSettle();
    expect(
      transport.requests
          .where((request) => request['operation'] == 'Setup Doctor報告取得'),
      hasLength(1),
    );

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: ProblemsPanel(client: client, active: true)),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('現在のBroker診断で確認した状態です。'), findsOneWidget);
    expect(find.text('監査イベントID: audit-problems-current'), findsOneWidget);
    expect(find.text('起動時の診断です。'), findsNothing);
    expect(
      transport.requests
          .where((request) => request['operation'] == 'Setup Doctor報告取得'),
      hasLength(2),
    );
  });

  testWidgets('問題一覧はBroker由来の範囲・重大度・復旧案内を表示専用で示す', (tester) async {
    final transport = _SetupDoctorTransport();
    final client = await ShellCoreClient.product(transport: transport);
    transport.nextReportResponse = _setupDoctorResponse(
      status: 'warning',
      message: 'Brokerの実行環境診断です。',
      warningCheckId: 'setup_doctor.runtime_connection',
      recoveryInstruction: 'Broker接続を確認してください。',
      auditId: 'audit-problems-classification',
    );

    await tester.pumpWidget(
      MaterialApp(home: Scaffold(body: ProblemsPanel(client: client))),
    );
    await tester.pumpAndSettle();

    expect(find.text('診断範囲'), findsOneWidget);
    expect(find.text('重大度'), findsOneWidget);
    expect(find.text('実行環境'), findsOneWidget);
    expect(find.text('注意'), findsOneWidget);
    expect(find.text('Brokerの実行環境診断です。'), findsOneWidget);
    expect(find.text('Broker接続を確認してください。'), findsOneWidget);
    expect(
        find.byKey(const ValueKey('problems-panel-refresh')), findsOneWidget);
  });

  testWidgets('復旧画面はBrokerの最新案内を表示し再試行安全性を捏造しない', (tester) async {
    final transport = _SetupDoctorTransport();
    final client = await ShellCoreClient.product(transport: transport);
    transport.nextReportResponse = _setupDoctorResponse(
      status: 'warning',
      message: '最新の復旧対象をBrokerが観測しました。',
      unknownFirstCheck: true,
      auditId: 'audit-recovery-current',
    );

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: RecoveryCenter(client: client, active: true)),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('最新の復旧対象をBrokerが観測しました。'), findsOneWidget);
    expect(find.text('Broker接続後に再取得してください。'), findsOneWidget);
    expect(find.text('監査イベントID: audit-recovery-current'), findsOneWidget);
    expect(find.text('診断表示は再試行の安全性や実行許可を意味しません。'), findsOneWidget);
    expect(find.text('再試行可能'), findsNothing);
  });

  testWidgets('復旧画面はBroker切替後に旧Brokerの遅延報告を破棄する', (tester) async {
    final firstTransport = _SetupDoctorTransport();
    final firstClient =
        await ShellCoreClient.product(transport: firstTransport);
    final secondClient = await ShellCoreClient.product(
      transport: _SetupDoctorTransport(initialMessage: '切替先Brokerの最新診断です。'),
    );
    final pending = Completer<Map<String, Object?>>();
    firstTransport.holdNextReport = pending;

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: RecoveryCenter(client: firstClient, active: true)),
      ),
    );
    await tester.pump();
    expect(
      firstTransport.requests
          .where((request) => request['operation'] == 'Setup Doctor報告取得'),
      hasLength(2),
    );

    await tester.pumpWidget(
      MaterialApp(
        home:
            Scaffold(body: RecoveryCenter(client: secondClient, active: true)),
      ),
    );
    await tester.pumpAndSettle();
    pending.complete(
      _setupDoctorResponse(
        status: 'warning',
        message: '旧Brokerの復旧報告です。',
        unknownFirstCheck: true,
        auditId: 'audit-stale-recovery',
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('切替先Brokerの最新診断です。'), findsOneWidget);
    expect(find.text('旧Brokerの復旧報告です。'), findsNothing);
  });

  testWidgets('ProblemsのBroker取得失敗時は起動Snapshotを現在状態として残さない', (tester) async {
    final transport = _SetupDoctorTransport();
    final client = await ShellCoreClient.product(transport: transport);
    transport.nextReportResponse = {
      'request_id': 'test-setup-doctor-rejected',
      'operation': 'Setup Doctor報告取得',
      'status': 'rejected',
      'evidence_source': 'LIVE_RUNTIME',
      'audit_event_id': 'audit-setup-doctor-rejected',
      'error': {
        'code': 'fixture_failure',
        'message': 'SENSITIVE_SYNTHETIC_BROKER_DETAIL',
      },
      'body': null,
    };

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: ProblemsPanel(client: client, active: true)),
      ),
    );
    await tester.pumpAndSettle();

    expect(
      find.text('意味: 最新の問題状態を取得できません。Broker接続を確認して再試行してください。'),
      findsOneWidget,
    );
    expect(find.text('起動時の診断です。'), findsNothing);
    expect(
        find.textContaining('SENSITIVE_SYNTHETIC_BROKER_DETAIL'), findsNothing);
    expect(find.textContaining('audit-setup-doctor-rejected'), findsNothing);
  });

  testWidgets('監査ビューアーは明示要求後にBrokerの検証済みpageを表示する', (tester) async {
    final transport = _SetupDoctorTransport();
    final client = await ShellCoreClient.product(transport: transport);
    transport.nextAuditResponse = _auditInspectionResponse();

    await tester.pumpWidget(
      MaterialApp(home: Scaffold(body: AuditViewer(client: client))),
    );
    expect(find.text('Broker監査一覧は未取得'), findsOneWidget);
    expect(
      transport.requests.where((request) => request['operation'] == '監査一覧取得'),
      isEmpty,
    );

    await tester.tap(find.byKey(const ValueKey('audit-inspection-refresh')));
    await tester.pumpAndSettle();

    expect(find.text('監査鎖状態: 検証済み'), findsOneWidget);
    expect(find.text('ハッシュ鎖状態: 検証済み'), findsOneWidget);
    expect(
      find.text('状態: 認証済みBroker Sessionのread-only Grant確認済み'),
      findsOneWidget,
    );
    expect(
      find.text('Owner Approval／Permission／Authorityの発行・変更: なし'),
      findsOneWidget,
    );
    expect(find.text('個別Grant対応: Audit pageには投影されません'), findsOneWidget);
    expect(find.text('broker-audit-1'), findsOneWidget);
    expect(find.textContaining('safe-action'), findsOneWidget);
    expect(find.textContaining('PRIVATE_REQUEST'), findsNothing);
    expect(find.textContaining('PRIVATE_REASON'), findsNothing);
    final request = transport.requests.singleWhere(
      (item) => item['operation'] == '監査一覧取得',
    );
    expect(request['payload'], const <String, Object?>{
      'version': 1,
      'limit': 100,
    });
  });

  testWidgets('監査一覧の再取得失敗時に前回pageを消しBroker詳細を表示しない', (tester) async {
    final transport = _SetupDoctorTransport();
    final client = await ShellCoreClient.product(transport: transport);
    transport.nextAuditResponse = _auditInspectionResponse();

    await tester.pumpWidget(
      MaterialApp(home: Scaffold(body: AuditViewer(client: client))),
    );
    await tester.tap(find.byKey(const ValueKey('audit-inspection-refresh')));
    await tester.pumpAndSettle();
    expect(find.text('broker-audit-1'), findsOneWidget);

    transport.nextAuditResponse = {
      'request_id': 'test-audit-rejected',
      'operation': '監査一覧取得',
      'status': 'suspended',
      'evidence_source': 'LIVE_RUNTIME',
      'audit_event_id': 'broker-audit-2',
      'error': {
        'code': 'audit_integrity_failure',
        'message': 'PRIVATE_BROKER_ERROR_DETAIL',
      },
      'body': null,
    };
    await tester.tap(find.byKey(const ValueKey('audit-inspection-refresh')));
    await tester.pumpAndSettle();

    expect(find.text('broker-audit-1'), findsNothing);
    expect(find.text('Broker監査一覧は表示できません'), findsOneWidget);
    expect(find.textContaining('PRIVATE_BROKER_ERROR_DETAIL'), findsNothing);
  });

  testWidgets('さらに古い監査eventは取得したcursorだけをBrokerへ送る', (tester) async {
    final transport = _SetupDoctorTransport();
    final client = await ShellCoreClient.product(transport: transport);
    transport.auditResponses.addAll([
      _auditLatestPageResponse(),
      _auditOlderPageResponse(),
    ]);

    await tester.pumpWidget(
      MaterialApp(home: Scaffold(body: AuditViewer(client: client))),
    );
    await tester.tap(find.byKey(const ValueKey('audit-inspection-refresh')));
    await tester.pumpAndSettle();
    expect(find.text('さらに古い監査event'), findsOneWidget);
    expect(find.text('broker-audit-103'), findsOneWidget);

    await tester
        .ensureVisible(find.byKey(const ValueKey('audit-inspection-older')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('audit-inspection-older')));
    await tester.pumpAndSettle();

    expect(find.text('broker-audit-1'), findsOneWidget);
    expect(find.text('broker-audit-3'), findsOneWidget);
    expect(find.text('broker-audit-4'), findsNothing);
    expect(find.text('さらに古い監査event'), findsNothing);
    final auditRequests = transport.requests
        .where((request) => request['operation'] == '監査一覧取得')
        .toList();
    expect(auditRequests, hasLength(2));
    expect(auditRequests.last['payload'], const <String, Object?>{
      'version': 1,
      'limit': 100,
      'before_event_id': 'broker-audit-4',
    });
  });
}

class _SetupDoctorTransport implements BrokerTransport {
  _SetupDoctorTransport({this.initialMessage = '起動時の診断です。'});

  final String initialMessage;
  final requests = <Map<String, Object?>>[];
  int _reportCount = 0;
  Map<String, Object?>? nextReportResponse;
  Map<String, Object?>? nextAuditResponse;
  final auditResponses = <Map<String, Object?>>[];
  Completer<Map<String, Object?>>? holdNextReport;

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    requests.add({
      'operation': operation,
      if (payload != null) 'payload': Map<String, Object?>.from(payload),
    });
    if (operation == '監査一覧取得') {
      if (auditResponses.isNotEmpty) return auditResponses.removeAt(0);
      final next = nextAuditResponse;
      nextAuditResponse = null;
      if (next != null) return next;
      throw const BrokerClientException('No audit fixture response');
    }
    if (operation == 'Setup Doctor報告取得') {
      _reportCount++;
      final next = nextReportResponse;
      if (next != null) {
        nextReportResponse = null;
        return next;
      }
      final pending = holdNextReport;
      if (pending != null) {
        holdNextReport = null;
        return pending.future;
      }
      return _setupDoctorResponse(
        status: 'warning',
        message: initialMessage,
        unknownFirstCheck: true,
        auditId: 'audit-setup-doctor-$_reportCount',
      );
    }
    if (operation == 'health') {
      return {
        ..._accepted(operation, const <String, Object?>{}),
        'health': {
          'broker_id': 'test-broker',
          'status': 'ready',
          'authority_cutover_status': 'active',
          'command_dispatch_enabled': false,
          'persistence_ready': true,
          'evidence_source': 'LIVE_RUNTIME',
        },
        'body': null,
      };
    }
    if (operation == '初回設定取得') {
      return _accepted(operation, {
        'version': 1,
        'product': 'D4 Pocket',
        'ui_preferences': {
          'theme': 'system',
          'density': 'compact',
          'locale': 'ja-JP',
        },
      });
    }
    if (operation == 'ホスト能力') {
      return _accepted(operation, {
        '版': 1,
        'ホストID': 'fixture-host',
        '表示名': '試験Host',
        'プラットフォーム': 'windows',
        '状態': 'ready',
        '能力': <Map<String, Object?>>[],
      });
    }
    if (operation == 'Host一覧') {
      return _accepted(operation, {'版': 1, 'Host一覧': <Object?>[]});
    }
    if (operation == 'アダプター一覧') {
      return _accepted(operation, {'Adapter一覧': <Object?>[]});
    }
    if (operation == 'Agent一覧') {
      return _accepted(operation, {'Agent': <Object?>[]});
    }
    if (operation == '対話セッション一覧') {
      return {
        ..._accepted(operation, {'版': 1, '対話セッション': <Object?>[]}),
        'evidence_source': 'INTERNAL_STATE',
      };
    }
    if (operation == 'normalize_payload') {
      return _accepted(operation, {'quarantined': false});
    }
    if (operation == 'content_projection') {
      return _accepted(operation, {'redacted_payload': <String, Object?>{}});
    }
    if (operation == 'approval_edit') {
      return _accepted(operation, {'ok': false});
    }
    if (operation == 'command_envelope') {
      return {
        'request_id': 'test-$operation',
        'operation': operation,
        'status': 'suspended',
        'evidence_source': 'LIVE_RUNTIME',
        'audit_event_id': 'audit-$operation',
        'error': {'code': 'suspended', 'message': 'fixture'},
        'body': null,
      };
    }
    throw BrokerClientException('Unexpected fixture operation: $operation');
  }
}

Map<String, Object?> _auditInspectionResponse() => {
      'request_id': 'test-audit-inspection',
      'operation': '監査一覧取得',
      'status': 'accepted',
      'evidence_source': 'LIVE_RUNTIME',
      'audit_event_id': 'broker-audit-2',
      'error': null,
      'body': {
        'version': 1,
        'integrity_status': 'verified',
        'anchor_status': 'verified',
        'verified_event_count': 1,
        'snapshot_head_event_id': 'broker-audit-1',
        'has_older_events': false,
        'next_before_event_id': null,
        'events': [
          {
            'event_id': 'broker-audit-1',
            'action': 'safe-action',
            'result': 'success',
            'payload_hash': 'sha256:${'0' * 64}',
            'previous_event_hash': null,
            'event_hash': 'sha256:${'1' * 64}',
          },
        ],
      },
    };

Map<String, Object?> _auditLatestPageResponse() => _auditPageResponse(
      requestId: 'test-audit-latest',
      auditId: 'broker-audit-105',
      count: 103,
      head: 'broker-audit-103',
      eventSequences: List<int>.generate(100, (index) => index + 4),
      hasOlder: true,
      nextBefore: 'broker-audit-4',
    );

Map<String, Object?> _auditOlderPageResponse() => _auditPageResponse(
      requestId: 'test-audit-older',
      auditId: 'broker-audit-105',
      count: 104,
      head: 'broker-audit-104',
      eventSequences: const [1, 2, 3],
      hasOlder: false,
    );

Map<String, Object?> _auditPageResponse({
  required String requestId,
  required String auditId,
  required int count,
  required String head,
  required List<int> eventSequences,
  required bool hasOlder,
  String? nextBefore,
}) {
  String hashFor(int sequence) =>
      'sha256:${sequence.toRadixString(16).padLeft(64, '0')}';
  return {
    'request_id': requestId,
    'operation': '監査一覧取得',
    'status': 'accepted',
    'evidence_source': 'LIVE_RUNTIME',
    'audit_event_id': auditId,
    'error': null,
    'body': {
      'version': 1,
      'integrity_status': 'verified',
      'anchor_status': 'verified',
      'verified_event_count': count,
      'snapshot_head_event_id': head,
      'has_older_events': hasOlder,
      'next_before_event_id': nextBefore,
      'events': [
        for (final sequence in eventSequences)
          {
            'event_id': 'broker-audit-$sequence',
            'action': 'safe-action',
            'result': 'success',
            'payload_hash': hashFor(0),
            'previous_event_hash': sequence == 1 ? null : hashFor(sequence - 1),
            'event_hash': hashFor(sequence),
          },
      ],
    },
  };
}

Map<String, Object?> _accepted(
  String operation,
  Map<String, Object?> body,
) =>
    {
      'request_id': 'test-$operation',
      'operation': operation,
      'status': 'accepted',
      'evidence_source': 'LIVE_RUNTIME',
      'audit_event_id': 'audit-$operation',
      'error': null,
      'body': body,
    };

Map<String, Object?> _setupDoctorResponse({
  required String status,
  required String message,
  required String auditId,
  bool unknownFirstCheck = false,
  String? warningCheckId,
  String recoveryInstruction = 'Broker接続後に再取得してください。',
}) {
  const checkIds = <String>[
    'setup_doctor.ran_from_installed_app_path',
    'setup_doctor.runtime_connection',
    'setup_doctor.authority_boundary',
    'setup_doctor.network_public_bind',
    'setup_doctor.recovery_instruction',
    'setup_doctor.audit_storage',
    'setup_doctor.config_created',
  ];
  const scopes = <String>[
    'installation',
    'runtime',
    'authority',
    'network',
    'recovery',
    'audit',
    'configuration',
  ];
  Map<String, Object?> buildCheck(int index) {
    final checkStatus = unknownFirstCheck && index == 0
        ? 'unknown'
        : warningCheckId == checkIds[index]
            ? 'warning'
            : 'pass';
    return {
      'check_id': checkIds[index],
      'scope': scopes[index],
      'status': checkStatus,
      'severity': switch (checkStatus) {
        'pass' => 'info',
        'warning' => 'warning',
        'fail' => 'critical',
        _ => 'unknown',
      },
      'message': message,
      'recovery_instruction': recoveryInstruction,
      'evidence_class': switch (index) {
        1 || 5 => 'LIVE_RUNTIME',
        2 || 4 || 6 => 'CONFIG',
        _ => 'INTERNAL_STATE',
      },
      'grants_authority': false,
    };
  }

  final checks = [
    for (var index = 0; index < checkIds.length; index++) buildCheck(index),
  ];
  return {
    'request_id': 'test-Setup Doctor報告取得',
    'operation': 'Setup Doctor報告取得',
    'status': 'accepted',
    'evidence_source': 'LIVE_RUNTIME',
    'audit_event_id': auditId,
    'error': null,
    'body': {
      'version': 1,
      'report_id': 'setup-doctor-${'a' * 64}',
      'generated_at': '2026-11-15T00:00:00Z',
      'status': status,
      'evidence_source': 'LIVE_RUNTIME',
      'checks': checks,
      'installer_grants_authority': false,
      'installer_silently_approves_permissions': false,
    },
  };
}
