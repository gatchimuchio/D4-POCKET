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
import 'package:gui_shell_desktop/services/setup_doctor_export.dart';
import 'package:gui_shell_desktop/services/surface_semantics_export.dart';

void main() {
  Finder findSurfaceSemanticsIdentifier(String label) {
    final identifier = surfaceSemanticsIdentifier(label);
    return find.byWidgetPredicate(
      (widget) =>
          widget is Semantics && widget.properties.identifier == identifier,
      description: 'Semantics(identifier: $identifier)',
    );
  }

  testWidgets('GUI Shellデスクトップアプリの簡易試験', (WidgetTester tester) async {
    await tester.pumpWidget(const GuiShellDesktopApp());

    expect(find.byType(MaterialApp), findsOneWidget);
    expect(find.byType(NavigationRail), findsOneWidget);
    expect(find.text('概要'), findsWidgets);
    expect(find.text('信頼'), findsOneWidget);
    expect(find.text('権限'), findsOneWidget);
  });

  testWidgets('GUI Shellデスクトップアプリが製品基準の外枠を持つ', (WidgetTester tester) async {
    await tester.pumpWidget(const GuiShellDesktopApp());

    final app = tester.widget<MaterialApp>(find.byType(MaterialApp));
    expect(app.title, kGuiShellProductTitle);
    expect(app.themeMode, ThemeMode.system);
    expect(app.theme, isNotNull);
    expect(app.darkTheme, isNotNull);
    expect(find.byType(GuiShellFatalErrorScreen), findsNothing);
  });

  testWidgets('Windows受入画面が意味ラベルを公開する', (WidgetTester tester) async {
    final semantics = tester.ensureSemantics();
    SurfaceSemanticsRegistry.resetForTest();
    try {
      await tester.pumpWidget(const GuiShellDesktopApp());

      for (final label in [
        'Dashboard',
        'NavigationRail',
        'Runtime Status',
        'Invariant Status',
      ]) {
        expect(findSurfaceSemanticsIdentifier(label), findsOneWidget);
      }
      expect(find.bySemanticsLabel(RegExp('概要')), findsWidgets);
      expect(find.bySemanticsLabel(RegExp('ナビゲーション')), findsOneWidget);
      expect(find.bySemanticsLabel(RegExp('実行系状態')), findsWidgets);
      expect(find.bySemanticsLabel(RegExp('不変条件状態')), findsWidgets);
      final export = buildSurfaceSemanticsExport(path: 'surface.json');
      expect(export['source'], 'flutter_semantics_runtime_export');
      expect(export['path'], 'surface.json');
      expect(
        export['registered_surfaces'],
        containsAll(kRequiredSurfaceSemanticsLabels),
      );
      expect(export['visible_surfaces'], isEmpty);
      expect(export['surface_match_requirements_met'], isFalse);
      expect(export['evidence_class'], 'INTERNAL_STATE');
    } finally {
      SurfaceSemanticsRegistry.resetForTest();
      semantics.dispose();
    }
  });

  testWidgets('非表示でbuildした要素と破棄済み要素を可視証拠にしない', (tester) async {
    SurfaceSemanticsRegistry.resetForTest();
    try {
      await tester.pumpWidget(MaterialApp(
        home: Offstage(
          child: Column(children: [
            for (final label in kRequiredSurfaceSemanticsLabels)
              SurfaceSemantics(label: label, child: const Text('非表示')),
          ]),
        ),
      ));
      expect(find.text('非表示'), findsNothing);
      for (final remove in [false, true]) {
        if (remove) await tester.pumpWidget(const SizedBox.shrink());
        final export = buildSurfaceSemanticsExport();
        expect(export['visible_surfaces'], isEmpty);
        expect(export['registered_surfaces'],
            containsAll(kRequiredSurfaceSemanticsLabels));
        expect(export['surface_matches'], isEmpty);
        expect(export['surface_match_requirements_met'], isFalse);
        final tree = export['diagnostic_tree'] as Map;
        expect(tree['observed_elements'], isEmpty);
        expect(export['evidence_class'], 'INTERNAL_STATE');
      }
    } finally {
      SurfaceSemanticsRegistry.resetForTest();
    }
  });

  testWidgets('概要画面が段階Aと段階Bの完了状態を表示する', (WidgetTester tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: Dashboard(client: ShellCoreClient.mock())),
      ),
    );

    expect(find.textContaining('段階A: complete'), findsOneWidget);
    expect(find.textContaining('段階B: complete'), findsOneWidget);
    expect(find.textContaining('完成製品リリース: 未主張'), findsOneWidget);
  });

  testWidgets('状態バーが段階Bの所有者利用とリリース未主張を表示する', (WidgetTester tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ShellStatusBar(snapshot: ShellCoreClient.mock().getSnapshot()),
        ),
      ),
    );

    expect(find.textContaining('段階: B 所有者利用'), findsOneWidget);
    expect(find.textContaining('リリース: not claimed'), findsOneWidget);
  });

  testWidgets('コマンドパレットが検索して移動する', (WidgetTester tester) async {
    await tester.pumpWidget(const GuiShellDesktopApp());

    await tester.sendKeyDownEvent(LogicalKeyboardKey.control);
    await tester.sendKeyEvent(LogicalKeyboardKey.keyK);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.control);
    await tester.pumpAndSettle();

    await tester.enterText(find.byType(TextField), '問題');
    await tester.pumpAndSettle();
    expect(find.text('問題一覧を開く'), findsOneWidget);
    await tester.tap(find.text('問題一覧を開く'));
    await tester.pumpAndSettle();

    expect(find.text('問題一覧'), findsWidgets);
  });

  testWidgets('問題一覧が段階Bを失敗扱いにせずリリース遮断要因を表示する', (WidgetTester tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: ProblemsPanel(client: ShellCoreClient.mock())),
      ),
    );

    expect(find.text('問題一覧'), findsOneWidget);
    expect(
      find.textContaining('Windowsインストール先の初回起動実測証拠なし'),
      findsOneWidget,
    );
    expect(find.textContaining('release_blocker'), findsWidgets);
    expect(find.text('復旧'), findsOneWidget);
    expect(find.text('所有者利用を遮断'), findsOneWidget);
    expect(find.text('製品リリースを遮断'), findsOneWidget);
    expect(find.textContaining('recover-windows-evidence'), findsWidgets);
    expect(
      find.textContaining('release_evidence/windows_installed_smoke.json'),
      findsWidgets,
    );
    expect(find.textContaining('段階Bの所有者利用を失敗扱いにせず'), findsOneWidget);
  });

  testWidgets('証拠センターがWindows厳格検証の予期された失敗を表示する', (WidgetTester tester) async {
    final releaseEvidence = File(
      'release_evidence/windows_installed_smoke.json',
    );
    final existedBefore = releaseEvidence.existsSync();
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: EvidenceCenter(client: ShellCoreClient.mock())),
      ),
    );

    expect(find.text('証拠センター'), findsOneWidget);
    expect(
      find.textContaining('strict_windows_release: expected fail'),
      findsOneWidget,
    );
    expect(
      find.textContaining('Windows実測証拠の不足: release_blocker'),
      findsOneWidget,
    );
    expect(releaseEvidence.existsSync(), existedBefore);
  });

  testWidgets('証拠センターが表示専用の書出しとスナップショット比較を提供する', (WidgetTester tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: EvidenceCenter(client: ShellCoreClient.mock())),
      ),
    );

    expect(find.text('証拠束の書き出し'), findsOneWidget);
    expect(find.text('スナップショットの読込み／書出し'), findsOneWidget);
    expect(find.text('検証概要をコピー'), findsOneWidget);
    expect(find.text('読込み前確認／比較'), findsOneWidget);
  });

  testWidgets('復旧手順がWindows証拠を段階Bで継続可能と表示する', (WidgetTester tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: RecoveryCenter(client: ShellCoreClient.mock())),
      ),
    );

    expect(find.text('復旧手順'), findsOneWidget);
    expect(
      find.textContaining('Windowsインストール先の実測証拠なし'),
      findsOneWidget,
    );
    expect(find.textContaining('はい'), findsWidgets);
    expect(find.text('コマンド'), findsOneWidget);
    expect(find.text('パス'), findsOneWidget);
  });

  testWidgets('設定画面が段階／リリース設定を検索して絞り込む', (WidgetTester tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: SettingsScreen(client: ShellCoreClient.mock())),
      ),
    );

    expect(find.text('設定'), findsWidgets);
    await tester.enterText(find.byType(TextField), 'release');
    await tester.pumpAndSettle();

    expect(find.textContaining('release.state'), findsOneWidget);
    expect(find.textContaining('not claimed'), findsWidgets);
  });

  testWidgets('信頼画面と権限画面が利用可能である', (WidgetTester tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: TrustCenter(client: ShellCoreClient.mock())),
      ),
    );

    expect(find.text('信頼センター'), findsOneWidget);
    expect(find.textContaining('workspace_trust'), findsOneWidget);
    expect(find.textContaining('Shell Coreの能力'), findsOneWidget);

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: AuthorityMap(client: ShellCoreClient.mock())),
      ),
    );

    expect(find.text('権限対応図'), findsOneWidget);
    expect(find.textContaining('filesystem.write'), findsWidgets);
    expect(find.textContaining('権限判断はShell Coreに保持'), findsOneWidget);
  });

  testWidgets('実行系詳細と監査絞込みが表示される', (WidgetTester tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: RuntimeCenter(client: ShellCoreClient.mock())),
      ),
    );

    expect(find.text('実行系の詳細'), findsOneWidget);
    expect(find.textContaining('実行系ID: blue_tanuki'), findsOneWidget);
    expect(find.text('能力: filesystem.write'), findsOneWidget);

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: AuditViewer(client: ShellCoreClient.mock())),
      ),
    );

    expect(find.text('監査時系列の絞込み'), findsOneWidget);
    expect(find.textContaining('ハッシュ鎖状態'), findsOneWidget);
    expect(find.text('コピー'), findsOneWidget);
  });

  test('環境診断のクライアント面が構造化され権限を持たない', () {
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

  test('製品クライアントがブローカー経由の権限スナップショットを描画する', () async {
    final transport = _FakeBrokerTransport([
      _brokerHealthResponse(),
      _brokerAcceptedBody('normalize_payload', {'quarantined': false}),
      _brokerAcceptedBody('content_projection', {
        'redacted_payload': {'path': 'notes/today.md', 'content': '[redacted]'},
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
    expect(snapshot.operationStatus.pendingApprovalsCount, 0);
    expect(snapshot.permissions, isEmpty);
    expect(snapshot.pendingApprovals, isEmpty);
    expect(snapshot.authorityMap, isEmpty);
    expect(
      snapshot.evidence.any(
        (record) =>
            record.evidenceId == 'broker-redacted-projection-probe' &&
            record.status == 'pass',
      ),
      isTrue,
    );
    expect(
      snapshot.setupDoctorChecks.any(
        (check) =>
            check.checkId == 'broker.protected_field_edit' &&
            check.status == 'pass',
      ),
      isTrue,
    );
    expect(
      snapshot.problems.any(
        (problem) =>
            problem.problemId == 'broker-command-dispatch-suspended' &&
            problem.classification == 'release_blocker',
      ),
      isTrue,
    );
    expect(
      snapshot.problems.any(
        (problem) => problem.requiredAction.toLowerCase().contains('python'),
      ),
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

  test('ブローカー利用不可時に製品クライアントが閉鎖側へ失敗する', () async {
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
      snapshot.setupDoctorChecks.where(
        (check) => check.checkId == 'broker.fail_closed',
      ),
      isNotEmpty,
    );
  });

  test('認証拒否時に製品クライアントが閉鎖側へ失敗する', () async {
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

  test('期限切れブローカーセッションで製品クライアントが閉鎖側へ失敗する', () async {
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

  test('不正なブローカー応答で製品クライアントが閉鎖側へ失敗する', () async {
    final client = await ShellCoreClient.product(
      transport: _FakeBrokerTransport([
        {'status': 'accepted', 'operation': 'health'},
      ]),
    );

    expect(client.mode, 'broker_unavailable');
    expect(client.getSnapshot().snapshotSource, 'broker_unavailable');
  });

  testWidgets('環境診断が軽量な環境スナップショットを表示する', (WidgetTester tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: SetupDoctor(client: ShellCoreClient.mock())),
      ),
    );

    expect(find.text('環境スナップショット'), findsOneWidget);
    expect(find.textContaining('ネットワーク公開範囲:'), findsOneWidget);
    expect(find.textContaining('設定／スナップショットのパス:'), findsOneWidget);
  });

  test('ローカルクライアントが構造化スナップショットを読む', () {
    final tempDir = Directory.systemTemp.createTempSync('gui-shell-test-');
    addTearDown(() => tempDir.deleteSync(recursive: true));
    final snapshotFile = File('${tempDir.path}/shell_snapshot.json');
    snapshotFile.writeAsStringSync(
      jsonEncode({
        'runtimes': [
          {
            'runtime_id': 'runtime-from-json',
            'name': 'Runtime From Json',
            'status': 'ready',
            'adapter_id': 'adapter-from-json',
            'diagnostic_summary': 'loaded from local snapshot',
          },
        ],
        'agent_sessions': [],
        'permissions': [],
        'pending_approvals': [],
        'audit_events': [],
        'recovery_actions': [],
        'invariant_flags': {
          'flutter_imported_by_shell_core': true,
          'blue_tanuki_imported_by_shell_core': false,
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
            'grants_authority': false,
          },
        ],
      }),
    );

    final client = ShellCoreClient.local(snapshotPath: snapshotFile.path);
    final snapshot = client.getSnapshot();

    expect(client.mode, 'local');
    expect(snapshot.runtimes.single.runtimeId, 'runtime-from-json');
    expect(snapshot.setupDoctorChecks.single.checkId, 'local.json');
    expect(snapshot.invariantFlags['flutter_imported_by_shell_core'], isTrue);
    expect(snapshot.snapshotSource, 'local');
    expect(snapshot.operationStatus.releaseState, 'not claimed');
  });

  test('環境診断の製品書出しが正式なアプリ生成証拠である', () {
    final tempDir = Directory.systemTemp.createTempSync('gui-shell-export-');
    addTearDown(() => tempDir.deleteSync(recursive: true));
    final configFile = File('${tempDir.path}/gui_shell.json')
      ..writeAsStringSync('{}');
    final auditDir = Directory('${tempDir.path}/audit')..createSync();
    const exePath =
        r'C:\Users\owner\AppData\Local\GUI-Shell\installed-runs\run-1\app\gui_shell_desktop.exe';
    final export = buildSetupDoctorProductExport(
      ShellCoreClient.mock().getSnapshot(),
      command: exePath,
      context: {
        'installed_app_path': exePath,
        'installed_app_path_confirmed': true,
        'app_artifact_sha256': 'sha256:${List.filled(64, '1').join()}',
        'config_path': configFile.path,
        'audit_dir': auditDir.path,
        'restricted_loopback_bind': true,
      },
    );

    expect(export['formal_product_evidence'], isTrue);
    final source = export['evidence_source'] as Map<String, Object?>;
    expect(source['source_kind'], 'installed_app_machine_readable_export');
    expect(source['product_generated'], isTrue);
    expect(source['collector_derives_checks'], isFalse);
    expect(source['synthetic'], isFalse);
    expect(export['ran_from_installed_app_path'], isTrue);
    final checks = export['checks'] as List<Map<String, Object?>>;
    expect(
      kRequiredSetupDoctorCheckIds.every(
        (checkId) => checks.any((check) => check['check_id'] == checkId),
      ),
      isTrue,
    );
    expect(checks.any((check) => check['grants_authority'] != false), isFalse);
    expect(
      checks.any((check) => (check['recovery_instruction'] as String).isEmpty),
      isFalse,
    );
  });

  test('環境診断の製品書出しがインストール後初回設定を作成する', () async {
    final tempDir = Directory.systemTemp.createTempSync('gui-shell-export-');
    addTearDown(() => tempDir.deleteSync(recursive: true));
    final exportFile = File('${tempDir.path}/setup_doctor.json');
    final contextFile = File('${tempDir.path}/setup_doctor_context.json');
    final configFile = File('${tempDir.path}/config/gui_shell.json');
    final auditDir = Directory('${tempDir.path}/audit')..createSync();
    const exePath =
        r'C:\Users\owner\AppData\Local\GUI-Shell\installed-runs\run-1\app\gui_shell_desktop.exe';
    await contextFile.writeAsString(
      jsonEncode({
        'installed_app_path': exePath,
        'installed_app_path_confirmed': true,
        'app_artifact_sha256': 'sha256:${List.filled(64, '1').join()}',
        'config_path': configFile.path,
        'audit_dir': auditDir.path,
        'restricted_loopback_bind': true,
      }),
    );

    await writeSetupDoctorProductExportIfRequested(
      ShellCoreClient.mock().getSnapshot(),
      environment: {
        kSetupDoctorExportPathEnv: exportFile.path,
        kSetupDoctorContextPathEnv: contextFile.path,
      },
      resolvedExecutable: exePath,
    );

    expect(configFile.existsSync(), isTrue);
    final config =
        jsonDecode(configFile.readAsStringSync()) as Map<String, Object?>;
    expect(config['created_by'], 'gui_shell_desktop_installed_first_run');
    expect(config['installer_grants_authority'], isFalse);
    expect(config['installer_silently_approves_permissions'], isFalse);
    final export =
        jsonDecode(exportFile.readAsStringSync()) as Map<String, Object?>;
    final checks = export['checks'] as List<Object?>;
    final configCheck = checks.cast<Map<String, Object?>>().singleWhere(
          (check) => check['check_id'] == 'first_run.config_created',
        );
    expect(configCheck['status'], 'pass');
  });

  test('ローカルスナップショットの代替処理がリリース準備完了を主張しない', () {
    final tempDir = Directory.systemTemp.createTempSync('gui-shell-missing-');
    addTearDown(() => tempDir.deleteSync(recursive: true));
    final missingPath = '${tempDir.path}/missing_snapshot.json';

    final snapshot = ShellCoreClient.local(
      snapshotPath: missingPath,
    ).getSnapshot();

    expect(snapshot.snapshotSource, 'fallback');
    expect(snapshot.snapshotFreshness, 'missing');
    expect(snapshot.operationStatus.releaseState, 'not claimed');
    expect(
      snapshot.problems.any(
        (problem) => problem.problemId == 'local-snapshot-missing',
      ),
      isTrue,
    );
  });

  test('ローカルスナップショットの解析失敗が安全に代替処理へ移る', () {
    final tempDir = Directory.systemTemp.createTempSync('gui-shell-bad-json-');
    addTearDown(() => tempDir.deleteSync(recursive: true));
    final snapshotFile = File('${tempDir.path}/bad_snapshot.json')
      ..writeAsStringSync('{bad json');

    final snapshot = ShellCoreClient.local(
      snapshotPath: snapshotFile.path,
    ).getSnapshot();

    expect(snapshot.snapshotSource, 'fallback');
    expect(snapshot.snapshotFreshness, 'parse failed');
    expect(snapshotAgeLabel(snapshot), '解析失敗');
    expect(snapshot.operationStatus.releaseState, 'not claimed');
    expect(
      snapshot.problems.any(
        (problem) => problem.problemId == 'local-snapshot-parse-failed',
      ),
      isTrue,
    );
  });

  testWidgets('環境診断UIがローカル診断データを表示する', (WidgetTester tester) async {
    final tempDir = Directory.systemTemp.createTempSync('gui-shell-ui-test-');
    addTearDown(() => tempDir.deleteSync(recursive: true));
    final snapshotFile = File('${tempDir.path}/shell_snapshot.json');
    snapshotFile.writeAsStringSync(
      jsonEncode({
        'runtimes': [
          {
            'runtime_id': 'runtime-ui-json',
            'name': 'Runtime UI Json',
            'status': 'ready',
            'adapter_id': 'adapter-ui-json',
            'diagnostic_summary': 'loaded from local snapshot',
          },
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
            'grants_authority': false,
          },
        ],
      }),
    );

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

  testWidgets('承認センターが非表示の完全内容を公開しない', (WidgetTester tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: ApprovalCenter(client: ShellCoreClient.local())),
      ),
    );

    expect(find.textContaining('可視性: redacted'), findsOneWidget);
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
      throw BrokerClientException('$operation 用の fake broker 応答がありません');
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
