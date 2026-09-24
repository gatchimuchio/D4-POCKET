import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/screens/runtime_center.dart';
import 'package:gui_shell_desktop/services/shell_core_client.dart';
import 'package:gui_shell_ui/runtime_lifecycle_client.dart';
import 'support/test_broker_tcp_transport.dart';

class _LifecycleFixture implements BrokerTransport {
  _LifecycleFixture({required this.supported});

  final bool supported;
  final calls = <String>[];
  final payloads = <Map<String, Object?>>[];
  bool _approvalRequested = false;

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    calls.add(operation);
    payloads.add(Map<String, Object?>.from(payload ?? const {}));
    switch (operation) {
      case '実行系ライフサイクル状態':
        return _response(
          operation,
          supported ? 'LIVE_RUNTIME' : 'INTERNAL_STATE',
          supported ? 'lifecycle-status-audit-1' : 'lifecycle-status-audit-2',
          supported
              ? _supportedStatus(_approvalRequested)
              : _unsupportedStatus(),
        );
      case '実行系ライフサイクル承認要求':
        _approvalRequested = true;
        return _response(
          operation,
          'INTERNAL_STATE',
          'lifecycle-approval-audit-1',
          _approval('pending'),
        );
      default:
        throw StateError('このfixtureでは$operationを扱いません。');
    }
  }
}

Map<String, Object?> _response(
  String operation,
  String evidenceSource,
  String auditId,
  Map<String, Object?> body,
) =>
    {
      'request_id': 'lifecycle-request-1',
      'operation': operation,
      'status': 'accepted',
      'evidence_source': evidenceSource,
      'audit_event_id': auditId,
      'error': null,
      'health': null,
      'body': body,
      'shutdown_requested': false,
    };

Map<String, Object?> _unsupportedStatus() => {
      '版': 1,
      '実行系ID': 'no-lifecycle',
      '対応': false,
      '状態': 'not_supported',
      '証拠種別': 'INTERNAL_STATE',
      '操作一覧': const [],
      '承認一覧': const [],
    };

Map<String, Object?> _supportedStatus(bool approvalRequested) => {
      '版': 1,
      '実行系ID': 'fixture-runtime',
      '対応': true,
      '状態': 'stopped',
      '証拠種別': 'LIVE_RUNTIME',
      '操作一覧': [
        {
          '操作': 'start',
          '能力ID': 'runtime.lifecycle.start',
          '権限ID': 'permission.runtime.lifecycle.start',
          '承認必要': true,
          '復旧ID': 'recover-runtime-lifecycle-start',
          '実行可能': true,
        },
      ],
      '承認一覧': approvalRequested ? [_approval('pending')] : const [],
    };

Map<String, Object?> _approval(String state) => {
      '版': 1,
      '承認ID': 'lifecycle-approval-1',
      '承認hash':
          'sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
      '実行系ID': 'fixture-runtime',
      '操作': 'start',
      '状態': state,
      '有効期限UnixSeconds': 4102444800,
      '統治': {
        '能力ID': 'runtime.lifecycle.start',
        '権限ID': 'permission.runtime.lifecycle.start',
        '承認ID': 'lifecycle-approval-1',
        '承認状態': state,
        '復旧ID': 'recover-runtime-lifecycle-start',
      },
    };

Future<void> _waitForFiles(List<File> files) async {
  final deadline = DateTime.now().add(const Duration(seconds: 15));
  while (files.any((file) => !file.existsSync())) {
    if (DateTime.now().isAfter(deadline)) {
      fail('lifecycle実接続用Brokerの接続file生成が期限を超過しました。');
    }
    await Future<void>.delayed(const Duration(milliseconds: 30));
  }
}

Future<void> _approveLifecycle(
  String executable,
  String ownerSessionFile,
  RuntimeLifecycleApproval approval,
) async {
  final result = await Process.run(executable, [
    '実行系ライフサイクル承認',
    '--session-file',
    ownerSessionFile,
    '承認',
    approval.approvalId,
    approval.approvalHash,
  ]);
  expect(result.exitCode, 0, reason: result.stderr.toString());
}

void main() {
  test('debug BrokerとFlutter lifecycle clientを実接続し、承認済み開始と終端隔離を検証する', () async {
    final executable = File(
      '../../native/rust_helper/target/debug/gui_shell_rust_helper${Platform.isWindows ? '.exe' : ''}',
    ).absolute.path;
    expect(File(executable).existsSync(), isTrue,
        reason: '先にRust helperをdebug buildしてください。');

    final root =
        await Directory.systemTemp.createTemp('gui-shell-lifecycle-ipc-');
    Process? process;
    try {
      final normal = '${root.path}${Platform.pathSeparator}normal.json';
      final owner = '${root.path}${Platform.pathSeparator}owner.json';
      process = await Process.start(executable, [
        'broker-server',
        '--store-dir',
        '${root.path}${Platform.pathSeparator}store',
        '--session-file',
        normal,
        '--owner-session-file',
        owner,
        '--enable-development-lifecycle-fixture',
      ]);
      final output = process.stdout.drain<void>();
      final errors = process.stderr.drain<void>();
      await _waitForFiles([File(normal), File(owner)]);

      await expectLater(
        TestBrokerTcpTransport.connect(owner),
        throwsA(isA<TestBrokerTransportException>()),
      );

      final transport = await TestBrokerTcpTransport.connect(normal);
      final client = RuntimeLifecycleClient(transport);
      final initial = await client.status('development-lifecycle-fixture');
      expect(initial.state, 'stopped');
      expect(initial.evidenceSource, 'INTERNAL_STATE');
      expect(initial.approvedFor('start'), isNull);

      final startPending = await client.requestApproval(
        'development-lifecycle-fixture',
        'start',
      );
      expect(startPending.state, 'pending');
      await _approveLifecycle(executable, owner, startPending);
      final startApproved =
          await client.status('development-lifecycle-fixture');
      final startApproval = startApproved.approvedFor('start');
      expect(startApproval, isNotNull);

      final started = await client.execute(
        'development-lifecycle-fixture',
        'start',
        startApproval!.approvalId,
      );
      expect(started.previousState, 'stopped');
      expect(started.nextState, 'ready');
      expect(started.evidenceSource, 'LIVE_RUNTIME');
      expect(started.auditId, startsWith('broker-audit-'));

      final ready = await client.status('development-lifecycle-fixture');
      expect(ready.state, 'ready');
      final quarantinePending = await client.requestApproval(
        'development-lifecycle-fixture',
        'quarantine',
      );
      await _approveLifecycle(executable, owner, quarantinePending);
      final quarantineApproved =
          await client.status('development-lifecycle-fixture');
      final quarantineApproval = quarantineApproved.approvedFor('quarantine');
      expect(quarantineApproval, isNotNull);

      final quarantined = await client.execute(
        'development-lifecycle-fixture',
        'quarantine',
        quarantineApproval!.approvalId,
      );
      expect(quarantined.nextState, 'quarantined');
      expect(quarantined.evidenceSource, 'LIVE_RUNTIME');

      final terminal = await client.status('development-lifecycle-fixture');
      expect(terminal.state, 'quarantined');
      expect(terminal.operations, isEmpty);
      expect(terminal.approvals, isEmpty);

      await transport.request('shutdown');
      expect(await process.exitCode.timeout(const Duration(seconds: 10)), 0);
      await Future.wait([output, errors]);
    } finally {
      process?.kill();
      if (process != null) await process.exitCode;
      await root.delete(recursive: true);
    }
  });

  testWidgets('Capabilityなしの実行系にはライフサイクル操作を出さない', (tester) async {
    final fixture = _LifecycleFixture(supported: false);
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: RuntimeCenter(
            client: ShellCoreClient.mock(),
            lifecycleClient: RuntimeLifecycleClient(fixture),
          ),
        ),
      ),
    );

    await tester.tap(find.text('ライフサイクル').first);
    await tester.pumpAndSettle();
    expect(fixture.calls, isEmpty);
    await tester.enterText(
      find.byKey(const ValueKey('runtime-lifecycle-runtime-id')),
      'no-lifecycle',
    );
    await tester.tap(find.byKey(const ValueKey('runtime-lifecycle-refresh')));
    await tester.pumpAndSettle();

    expect(fixture.payloads, [
      {'版': 1, '実行系ID': 'no-lifecycle'},
    ]);
    expect(find.text('ライフサイクルCapabilityなし'), findsOneWidget);
    expect(find.byKey(const ValueKey('runtime-lifecycle-request-start')),
        findsNothing);
    expect(find.byKey(const ValueKey('runtime-lifecycle-execute-start')),
        findsNothing);
  });

  testWidgets('承認要求画面は固定payloadだけを送り、owner資格入力を置かない', (tester) async {
    final fixture = _LifecycleFixture(supported: true);
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: RuntimeCenter(
            client: ShellCoreClient.mock(),
            lifecycleClient: RuntimeLifecycleClient(fixture),
          ),
        ),
      ),
    );

    await tester.tap(find.text('ライフサイクル').first);
    await tester.pumpAndSettle();
    expect(find.byType(TextField), findsOneWidget);
    expect(find.text('PID、接続先、command、argvは入力できません。'), findsOneWidget);
    expect(find.textContaining('owner資格を入力'), findsNothing);

    await tester.enterText(
      find.byKey(const ValueKey('runtime-lifecycle-runtime-id')),
      'fixture-runtime',
    );
    await tester.tap(find.byKey(const ValueKey('runtime-lifecycle-refresh')));
    await tester.pumpAndSettle();
    final requestStart =
        find.byKey(const ValueKey('runtime-lifecycle-request-start'));
    await tester.ensureVisible(requestStart);
    await tester.tap(requestStart);
    await tester.pumpAndSettle();

    expect(fixture.calls, [
      '実行系ライフサイクル状態',
      '実行系ライフサイクル承認要求',
      '実行系ライフサイクル状態',
    ]);
    expect(fixture.payloads, [
      {'版': 1, '実行系ID': 'fixture-runtime'},
      {'版': 1, '実行系ID': 'fixture-runtime', '操作': 'start'},
      {'版': 1, '実行系ID': 'fixture-runtime'},
    ]);
    expect(find.byType(TextField), findsOneWidget);
    expect(find.byKey(const ValueKey('runtime-lifecycle-execute-start')),
        findsNothing);
    expect(find.textContaining('owner CLIで承認後に状態を更新してください。'), findsOneWidget);
  });
}
