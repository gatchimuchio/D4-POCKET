import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/main.dart';
import 'package:gui_shell_desktop/models/generated_contracts.dart';
import 'package:gui_shell_desktop/screens/approval_center.dart';
import 'package:gui_shell_desktop/screens/authority_map.dart';
import 'package:gui_shell_desktop/screens/audit_viewer.dart';
import 'package:gui_shell_desktop/screens/agent_center.dart';
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

const _requiredSurfaceSemanticsLabels = [
  'Dashboard',
  'NavigationRail',
  'Runtime Status',
  'Invariant Status',
];

String _testSessionId(String character) => List.filled(32, character).join();

void main() {
  Finder findSurfaceSemanticsIdentifier(String label) {
    final identifier = surfaceSemanticsIdentifier(label);
    return find.byWidgetPredicate(
      (widget) =>
          widget is Semantics && widget.properties.identifier == identifier,
      description: 'Semantics(identifier: $identifier)',
    );
  }

  testWidgets('履歴の遷移先がナビゲーションに存在し離脱できる', (tester) async {
    await tester.pumpWidget(const GuiShellDesktopApp());
    final rail = tester.widget<NavigationRail>(find.byType(NavigationRail));
    expect(rail.destinations.length, 20);
    rail.onDestinationSelected!(13);
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
    expect(find.text('先頭から更新'), findsOneWidget);
    tester
        .widget<NavigationRail>(find.byType(NavigationRail))
        .onDestinationSelected!(0);
    await tester.pumpAndSettle();
    expect(find.text('先頭から更新'), findsNothing);
  });

  testWidgets('評価ラボをNavigationRailから開ける', (tester) async {
    await tester.pumpWidget(const GuiShellDesktopApp());

    final rail = tester.widget<NavigationRail>(find.byType(NavigationRail));
    expect(rail.destinations.length, 20);
    rail.onDestinationSelected!(14);
    await tester.pumpAndSettle();

    expect(find.text('運用観測の境界'), findsOneWidget);
    expect(find.text('Dataset'), findsOneWidget);
  });

  testWidgets('ホスト能力をNavigationRailから開ける', (tester) async {
    await tester.pumpWidget(const GuiShellDesktopApp());

    final rail = tester.widget<NavigationRail>(find.byType(NavigationRail));
    rail.onDestinationSelected!(15);
    await tester.pumpAndSettle();

    expect(find.text('D4 Pocket ホスト能力'), findsOneWidget);
    expect(find.text('filesystem'), findsOneWidget);
    expect(find.textContaining('PermissionやApprovalは生成しません'), findsOneWidget);
  });

  testWidgets('通知センターをNavigationRailから開ける', (tester) async {
    await tester.pumpWidget(const GuiShellDesktopApp());

    final rail = tester.widget<NavigationRail>(find.byType(NavigationRail));
    rail.onDestinationSelected!(16);
    await tester.pumpAndSettle();

    expect(find.text('通知センター'), findsWidgets);
    expect(find.textContaining('Broker接続がないため'), findsOneWidget);
  });

  testWidgets('観測センターをNavigationRailから開ける', (tester) async {
    await tester.pumpWidget(const GuiShellDesktopApp());

    final rail = tester.widget<NavigationRail>(find.byType(NavigationRail));
    rail.onDestinationSelected!(17);
    await tester.pumpAndSettle();

    expect(find.text('観測センター'), findsWidgets);
    expect(find.textContaining('Broker接続がないため'), findsOneWidget);
  });

  testWidgets('Trace InspectorをNavigationRailから開ける', (tester) async {
    await tester.pumpWidget(const GuiShellDesktopApp());

    final rail = tester.widget<NavigationRail>(find.byType(NavigationRail));
    rail.onDestinationSelected!(18);
    await tester.pumpAndSettle();

    expect(find.text('追跡情報'), findsWidgets);
    expect(find.textContaining('Broker接続がないため'), findsOneWidget);
    expect(
      find.textContaining('Permission、Approval、Authority'),
      findsOneWidget,
    );
  });

  testWidgets('Host操作面でHost切替と観測境界を表示する', (tester) async {
    await tester.pumpWidget(const GuiShellDesktopApp());

    final rail = tester.widget<NavigationRail>(find.byType(NavigationRail));
    rail.onDestinationSelected!(19);
    await tester.pumpAndSettle();

    expect(find.text('D4 Pocket Host操作面'), findsOneWidget);
    expect(find.text('Host一覧'), findsOneWidget);
    expect(find.text('Runtime一覧'), findsOneWidget);
    expect(find.text('Agent一覧'), findsOneWidget);
    expect(find.text('現在のHost'), findsOneWidget);
    expect(find.textContaining('Permission、Approval、Authority'), findsWidgets);
  });

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
      final registered = SurfaceSemanticsRegistry.observed;
      expect(registered.keys, containsAll(_requiredSurfaceSemanticsLabels));
      expect(
        registered.values,
        everyElement(startsWith('gui_shell.surface.')),
      );
    } finally {
      SurfaceSemanticsRegistry.resetForTest();
      semantics.dispose();
    }
  });

  testWidgets('非表示要素の登録状態を可視証拠として扱わない', (tester) async {
    SurfaceSemanticsRegistry.resetForTest();
    try {
      await tester.pumpWidget(MaterialApp(
        home: Offstage(
          child: Column(children: [
            for (final label in _requiredSurfaceSemanticsLabels)
              SurfaceSemantics(label: label, child: const Text('非表示')),
          ]),
        ),
      ));
      expect(find.text('非表示'), findsNothing);
      for (final remove in [false, true]) {
        if (remove) await tester.pumpWidget(const SizedBox.shrink());
        final registered = SurfaceSemanticsRegistry.observed;
        expect(registered.keys, containsAll(_requiredSurfaceSemanticsLabels));
        expect(find.text('非表示'), findsNothing);
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

  testWidgets('コマンドパレットがC21の新機能別コマンドを表示する', (WidgetTester tester) async {
    await tester.pumpWidget(const GuiShellDesktopApp());

    await tester.sendKeyDownEvent(LogicalKeyboardKey.control);
    await tester.sendKeyEvent(LogicalKeyboardKey.keyK);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.control);
    await tester.pumpAndSettle();

    await tester.enterText(find.byType(TextField), 'MCP接続');
    await tester.pumpAndSettle();
    expect(
      find.byWidgetPredicate(
        (widget) => widget is Text && widget.data == 'MCP接続',
      ),
      findsOneWidget,
    );
    expect(find.text('MCP接続一覧とOwner確認付き切断を開く'), findsOneWidget);
  });

  testWidgets('全体検索がCtrl+Shift+Fで開き画面遷移だけを行う', (WidgetTester tester) async {
    await tester.pumpWidget(const GuiShellDesktopApp());

    await tester.sendKeyDownEvent(LogicalKeyboardKey.control);
    await tester.sendKeyDownEvent(LogicalKeyboardKey.shift);
    await tester.sendKeyEvent(LogicalKeyboardKey.keyF);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.shift);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.control);
    await tester.pumpAndSettle();

    expect(find.text('全体検索'), findsWidgets);
    expect(find.textContaining('検索結果は表示専用'), findsOneWidget);
    await tester.enterText(find.byType(TextField), 'MCP');
    await tester.pumpAndSettle();
    expect(find.text('MCP接続'), findsOneWidget);
    await tester.tap(find.text('MCP接続'));
    await tester.pumpAndSettle();
    expect(find.text('設定'), findsWidgets);
  });

  testWidgets('Desktop UX統合が既存画面を論理グループで絞り込む', (WidgetTester tester) async {
    await tester.pumpWidget(const GuiShellDesktopApp());

    final rail = tester.widget<NavigationRail>(find.byType(NavigationRail));
    expect(rail.destinations.length, 20);
    expect(find.text('すべて'), findsOneWidget);

    await tester.tap(find.text('すべて'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('安全').last);
    await tester.pumpAndSettle();

    final safetyRail = tester.widget<NavigationRail>(
      find.byType(NavigationRail),
    );
    expect(safetyRail.destinations.length, 7);
    expect(safetyRail.selectedIndex, 0);
    expect(find.text('信頼センター'), findsWidgets);
    expect(find.text('安全'), findsOneWidget);
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
      _brokerHostCapabilityResponse(),
      _brokerHostListResponse(),
      _brokerAdapterListResponse(),
      _brokerAgentAdapterListResponse(),
      _brokerDialogueSessionListResponse(sessions: [
        {
          '対話セッションID': _testSessionId('a'),
          '実行系ID': 'runtime-codex',
          '状態': '利用中',
          '作成監査ID': 'audit-session-created',
          '作業領域ID': 'codex-workspace',
          '作業領域結合監査ID': 'audit-workspace-bound',
        },
      ]),
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
    expect(snapshot.agentAdapters.single.agentId, 'codex');
    expect(snapshot.agentSessions.single.sessionId, _testSessionId('a'));
    expect(snapshot.agentSessions.single.agentRuntimeId, 'runtime-codex');
    expect(snapshot.agentSessions.single.status, '利用中');
    expect(snapshot.agentSessions.single.evidenceSource, 'INTERNAL_STATE');
    expect(snapshot.agentSessions.single.workspace, 'codex-workspace');
    expect(snapshot.agentSessions.single.workspaceAuditEventId,
        'audit-workspace-bound');
    expect(snapshot.agentSessions.single.task, isEmpty);
    expect(snapshot.agentSessions.single.auditEventId, 'audit-session-created');
    expect(
      snapshot.auditEvents.any(
        (event) => event.eventId == 'audit-session-list-read',
      ),
      isTrue,
    );
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
            check.checkId == 'setup_doctor.config_created' &&
            check.status == 'unknown',
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
      '初回設定取得',
      'Setup Doctor報告取得',
      'ホスト能力',
      'Host一覧',
      'アダプター一覧',
      'Agent一覧',
      '対話セッション一覧',
      'normalize_payload',
      'content_projection',
      'approval_edit',
      'command_envelope',
    ]);
    expect(
      transport.requests.singleWhere(
        (request) => request['operation'] == 'Setup Doctor報告取得',
      )['payload'],
      const <String, Object?>{'version': 1},
    );
    expect(
      transport.requests.singleWhere(
        (request) => request['operation'] == '初回設定取得',
      )['payload'],
      const <String, Object?>{'version': 1},
    );
    expect(
      transport.requests.singleWhere(
        (request) => request['operation'] == 'Agent一覧',
      )['payload'],
      const <String, Object?>{},
    );
  });

  testWidgets('Broker対話sessionは検証済みWorkspace参照と隔離未検証を区別して表示する',
      (WidgetTester tester) async {
    final transport = _FakeBrokerTransport([
      _brokerHealthResponse(),
      _brokerHostCapabilityResponse(),
      _brokerHostListResponse(),
      _brokerAdapterListResponse(),
      _brokerAgentAdapterListResponse(),
      _brokerDialogueSessionListResponse(sessions: [
        {
          '対話セッションID': _testSessionId('b'),
          '実行系ID': 'runtime-codex',
          '状態': '利用中',
          '作成監査ID': 'audit-session-created',
          '作業領域ID': 'codex-workspace',
          '作業領域結合監査ID': 'audit-workspace-bound',
        },
      ]),
      _brokerAcceptedBody('normalize_payload', {'quarantined': false}),
      _brokerAcceptedBody('content_projection', {'redacted_payload': {}}),
      _brokerAcceptedBody('approval_edit', {'ok': false}),
      _brokerCommandSuspendedResponse(),
    ]);
    final client = await ShellCoreClient.product(transport: transport);

    await tester.pumpWidget(MaterialApp(
      home: Scaffold(body: AgentCenter(client: client)),
    ));

    expect(find.text('runtime-codex'), findsOneWidget);
    expect(find.text('audit-session-created'), findsOneWidget);
    expect(find.text('codex-workspace'), findsOneWidget);
    expect(find.text('audit-workspace-bound'), findsOneWidget);
    expect(find.textContaining('書込み隔離は未検証'), findsOneWidget);
    expect(
      find.textContaining('Capability表示だけでは権限になりません'),
      findsOneWidget,
    );
    expect(
      find.text(
        'タスク: Broker記録は未取得（Taskが存在しない証拠ではありません）',
      ),
      findsOneWidget,
    );
    expect(
      find.textContaining('保留中の承認: 未取得（承認がないことを意味しません）'),
      findsOneWidget,
    );
    expect(find.textContaining('文書を更新する'), findsNothing);
  });

  testWidgets('mock AgentのTask・diff・Tool・command fixtureを実行結果として表示しない',
      (WidgetTester tester) async {
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(body: AgentCenter(client: ShellCoreClient.mock())),
    ));

    expect(
      find.textContaining('Local／mock snapshotはAgent実行結果ではない'),
      findsOneWidget,
    );
    expect(
      find.textContaining('ローカルの模擬データは、Broker上のエージェント実行として表示しません'),
      findsOneWidget,
    );
    expect(find.textContaining('Agent引き継ぎ'), findsOneWidget);
    expect(find.text('登録を開始'), findsNothing);
    expect(find.textContaining('未接続です。Task成果'), findsOneWidget);
    expect(find.text('文書を更新する'), findsNothing);
    expect(find.text('README.md'), findsNothing);
    expect(find.text('git.diff'), findsNothing);
    expect(
      find.text(
          'python3 tooling/conformance_tests/run_conformance_skeleton.py'),
      findsNothing,
    );
  });

  testWidgets('Codex登録UIは指定値だけをnative Owner確認Broker要求へ渡す',
      (WidgetTester tester) async {
    final transport = _FakeBrokerTransport([
      _brokerHealthResponse(),
      _brokerHostCapabilityResponse(),
      _brokerHostListResponse(),
      _brokerAdapterListResponse(),
      _brokerAgentAdapterListResponse(),
      _brokerDialogueSessionListResponse(),
      _brokerAcceptedBody('normalize_payload', {'quarantined': false}),
      _brokerAcceptedBody('content_projection', {'redacted_payload': {}}),
      _brokerAcceptedBody('approval_edit', {'ok': false}),
      _brokerCommandSuspendedResponse(),
      _brokerAcceptedBody('AgentCLI実行系作業領域登録', {
        'runtime_id': 'codex-r2-synthetic',
        'workspace_id': 'workspace-r2-synthetic',
        'registration_lifetime': 'broker_process',
        'task_execution': 'unsupported',
        'permission_generated': false,
        'approval_generated': false,
        'credential_value_accepted': false,
      }),
    ]);
    final client = await ShellCoreClient.product(transport: transport);
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(body: AgentCenter(client: client)),
    ));

    await tester.tap(find.text('登録を開始'));
    await tester.pumpAndSettle();
    final fields = find.byType(TextFormField);
    await tester.enterText(fields.at(0), 'codex-r2-synthetic');
    await tester.enterText(fields.at(1), r'C:\Tools\Codex\codex.exe');
    await tester.enterText(fields.at(2), 'workspace-r2-synthetic');
    await tester.enterText(fields.at(3), r'C:\d4-r2-synthetic-workspace');
    await tester.enterText(fields.at(4), '.env\nsecrets');
    await tester.tap(find.text('native Owner確認へ進む'));
    await tester.pumpAndSettle();
    await tester.pump();

    expect(transport.operations, contains('AgentCLI実行系作業領域登録'));
    final registrationResponse = transport.returnedResponses.singleWhere(
        (response) => response['operation'] == 'AgentCLI実行系作業領域登録');
    expect(registrationResponse['status'], 'accepted');
    final registrationRequest = transport.requests
        .singleWhere((request) => request['operation'] == 'AgentCLI実行系作業領域登録');
    final registration = registrationRequest['payload']! as Map;
    expect(registration['runtime_id'], 'codex-r2-synthetic');
    expect(registration['adapter_id'], 'codex-cli');
    expect(registration['workspace_root'], r'C:\d4-r2-synthetic-workspace');
    expect(registration['secret_paths'], ['.env', 'secrets']);
    expect(registration.containsKey('permission'), isFalse);
    expect(registration.containsKey('approval_id'), isFalse);
    expect(find.text('Task実行: unsupported'), findsOneWidget);
    expect(find.text('登録Workspaceで対話Sessionを開始'), findsOneWidget);
  });

  testWidgets('Agent CenterはCompare用に異なるRuntimeとWorkspaceを複数登録状態として保持する',
      (WidgetTester tester) async {
    Map<String, Object?> registrationResponse(
            String runtime, String workspace) =>
        _brokerAcceptedBody('AgentCLI実行系作業領域登録', {
          'runtime_id': runtime,
          'workspace_id': workspace,
          'registration_lifetime': 'broker_process',
          'task_execution': 'unsupported',
          'permission_generated': false,
          'approval_generated': false,
          'credential_value_accepted': false,
        });

    final transport = _FakeBrokerTransport([
      _brokerHealthResponse(),
      _brokerHostCapabilityResponse(),
      _brokerHostListResponse(),
      _brokerAdapterListResponse(),
      _brokerAgentAdapterListResponse(),
      _brokerDialogueSessionListResponse(),
      _brokerAcceptedBody('normalize_payload', {'quarantined': false}),
      _brokerAcceptedBody('content_projection', {'redacted_payload': {}}),
      _brokerAcceptedBody('approval_edit', {'ok': false}),
      _brokerCommandSuspendedResponse(),
      registrationResponse('codex-agent-a', 'workspace-agent-a'),
      registrationResponse('codex-agent-b', 'workspace-agent-b'),
    ]);
    final client = await ShellCoreClient.product(transport: transport);
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(body: AgentCenter(client: client)),
    ));

    Future<void> register(String runtime, String workspace, String root) async {
      await tester.tap(find.text('登録を開始'));
      await tester.pumpAndSettle();
      final fields = find.byType(TextFormField);
      await tester.enterText(fields.at(0), runtime);
      await tester.enterText(fields.at(1), r'C:\Tools\Codex\codex.exe');
      await tester.enterText(fields.at(2), workspace);
      await tester.enterText(fields.at(3), root);
      await tester.enterText(fields.at(4), '.env');
      await tester.tap(find.text('native Owner確認へ進む'));
      await tester.pumpAndSettle();
    }

    await register('codex-agent-a', 'workspace-agent-a', r'C:\d4-test\agent-a');
    await register('codex-agent-b', 'workspace-agent-b', r'C:\d4-test\agent-b');
    await register(
      'codex-agent-a',
      'workspace-agent-c',
      r'C:\d4-test\agent-c',
    );

    expect(find.text('Broker内登録: codex-agent-a'), findsOneWidget);
    expect(find.text('Broker内登録: codex-agent-b'), findsOneWidget);
    expect(find.text('登録Workspaceで対話Sessionを開始'), findsNWidgets(2));
    expect(
      find.text('同一RuntimeまたはWorkspaceを比較用に重複登録できません。'),
      findsOneWidget,
    );
    final requests = transport.requests
        .where((request) => request['operation'] == 'AgentCLI実行系作業領域登録')
        .map((request) => request['payload']! as Map)
        .toList();
    expect(requests.map((request) => request['runtime_id']).toSet(),
        {'codex-agent-a', 'codex-agent-b'});
    expect(requests.map((request) => request['workspace_id']).toSet(),
        {'workspace-agent-a', 'workspace-agent-b'});
    expect(requests, hasLength(2));
    expect(
        requests.every((request) =>
            !request.containsKey('permission') &&
            !request.containsKey('approval_id')),
        isTrue);
  });

  testWidgets('Agent CenterはSession結合後のTask unsupportedを事前検査しgrantへ進まない',
      (WidgetTester tester) async {
    const sessionId = 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
    final transport = _FakeBrokerTransport([
      _brokerHealthResponse(),
      _brokerHostCapabilityResponse(),
      _brokerHostListResponse(),
      _brokerAdapterListResponse(),
      _brokerAgentAdapterListResponse(),
      _brokerDialogueSessionListResponse(),
      _brokerAcceptedBody('normalize_payload', {'quarantined': false}),
      _brokerAcceptedBody('content_projection', {'redacted_payload': {}}),
      _brokerAcceptedBody('approval_edit', {'ok': false}),
      _brokerCommandSuspendedResponse(),
      _brokerAcceptedBody('AgentCLI実行系作業領域登録', {
        'runtime_id': 'codex-r2-synthetic',
        'workspace_id': 'workspace-r2-synthetic',
        'registration_lifetime': 'broker_process',
        'task_execution': 'unsupported',
        'permission_generated': false,
        'approval_generated': false,
        'credential_value_accepted': false,
      }),
      _brokerAcceptedBody('対話開始', {
        '対話セッションID': sessionId,
        '実行系ID': 'codex-r2-synthetic',
        '状態': '利用中',
      }),
      _brokerDialogueSessionListResponse(sessions: [
        {
          '対話セッションID': sessionId,
          '実行系ID': 'codex-r2-synthetic',
          '状態': '利用中',
          '作成監査ID': 'audit-session-created-r2',
          '作業領域ID': 'workspace-r2-synthetic',
          '作業領域結合監査ID': 'audit-workspace-bound-r2',
        },
      ]),
      {
        'request_id': 'test-agent-task-preflight',
        'operation': 'Agent作業要求検査',
        'status': 'rejected',
        'evidence_source': 'INTERNAL_STATE',
        'audit_event_id': 'audit-agent-task-preflight',
        'error': {
          'code': 'AgentTask実行非対応',
          'message': 'PRIVATE_TASK_SENTINEL',
        },
        'body': null,
        'shutdown_requested': false,
      },
    ]);
    final client = await ShellCoreClient.product(transport: transport);

    await tester.pumpWidget(MaterialApp(
      home: Scaffold(body: AgentCenter(client: client)),
    ));
    await tester.tap(find.text('登録を開始'));
    await tester.pumpAndSettle();
    final registrationFields = find.byType(TextFormField);
    await tester.enterText(registrationFields.at(0), 'codex-r2-synthetic');
    await tester.enterText(
        registrationFields.at(1), r'C:\Tools\Codex\codex.exe');
    await tester.enterText(registrationFields.at(2), 'workspace-r2-synthetic');
    await tester.enterText(
        registrationFields.at(3), r'C:\d4-r2-synthetic-workspace');
    await tester.enterText(registrationFields.at(4), '.env');
    await tester.tap(find.text('native Owner確認へ進む'));
    await tester.pumpAndSettle();

    await tester.tap(find.text('登録Workspaceで対話Sessionを開始'));
    await tester.pumpAndSettle();
    expect(find.text(sessionId), findsOneWidget);

    final preflightButton = find.text('Task実行能力を事前検査（実行なし）');
    await tester.ensureVisible(preflightButton);
    await tester.pumpAndSettle();
    await tester.tap(preflightButton);
    await tester.pumpAndSettle();
    final instructionField = find.byType(TextFormField);
    await tester.enterText(instructionField, '合成のTask事前検査指示');
    await tester.tap(find.text('Broker事前検査'));
    await tester.pumpAndSettle();

    expect(transport.operations, contains('Agent作業要求検査'));
    expect(transport.operations,
        isNot(contains('AgentTaskWorkspacePermissionGrant')));
    expect(
        transport.operations, isNot(contains('AgentTaskOwnerApprovalGrant')));
    expect(transport.operations, isNot(contains('AgentTask実行')));
    expect(find.textContaining('AgentTask実行非対応'), findsOneWidget);
    expect(find.textContaining('PRIVATE_TASK_SENTINEL'), findsNothing);
    expect(find.text('Task実行: unsupported'), findsOneWidget);
    expect(
      find.text('タスク: Broker記録は未取得（Taskが存在しない証拠ではありません）'),
      findsOneWidget,
    );
    expect(find.textContaining('Task実行・状態は未接続'), findsNothing);
  });

  testWidgets('Agent CenterはBroker事前検査後に分離Owner確認とTask状態照会を使う',
      (WidgetTester tester) async {
    const sessionId = 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
    const taskId = 'cccccccccccccccccccccccccccccccc';
    const instructionHash =
        'sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
    const resultHash =
        'sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
    Map<String, Object?> taskRecord({
      required String status,
      String? resultHash,
    }) =>
        {
          'task_id': taskId,
          'record_version': 2,
          'agent_runtime_id': 'codex-r2-synthetic',
          'session_id': sessionId,
          'workspace_id': 'workspace-r2-synthetic',
          'description': 'Agent作業Task（結果本文とWorkspace差分は別の権限経路）',
          'instruction_hash': instructionHash,
          'status': status,
          'audit_event_id': 'audit-task-$status',
          if (resultHash != null) 'result_hash': resultHash,
          'result_content_available': resultHash != null,
        };
    final transport = _FakeBrokerTransport([
      _brokerHealthResponse(),
      _brokerHostCapabilityResponse(),
      _brokerHostListResponse(),
      _brokerAdapterListResponse(),
      _brokerAgentAdapterListResponse(),
      _brokerDialogueSessionListResponse(),
      _brokerAcceptedBody('normalize_payload', {'quarantined': false}),
      _brokerAcceptedBody('content_projection', {'redacted_payload': {}}),
      _brokerAcceptedBody('approval_edit', {'ok': false}),
      _brokerCommandSuspendedResponse(),
      _brokerAcceptedBody('AgentCLI実行系作業領域登録', {
        'runtime_id': 'codex-r2-synthetic',
        'workspace_id': 'workspace-r2-synthetic',
        'registration_lifetime': 'broker_process',
        'task_execution': 'supported',
        'permission_generated': false,
        'approval_generated': false,
        'credential_value_accepted': false,
      }),
      _brokerAcceptedBody('対話開始', {
        '対話セッションID': sessionId,
        '実行系ID': 'codex-r2-synthetic',
        '状態': '利用中',
      }),
      _brokerDialogueSessionListResponse(sessions: [
        {
          '対話セッションID': sessionId,
          '実行系ID': 'codex-r2-synthetic',
          '状態': '利用中',
          '作成監査ID': 'audit-session-created-r2',
          '作業領域ID': 'workspace-r2-synthetic',
          '作業領域結合監査ID': 'audit-workspace-bound-r2',
        },
      ]),
      _brokerAcceptedBody('Agent作業要求検査', {
        '版': 1,
        '状態': '要求検査済み',
        '実行状態': '未実行',
        'Permission状態': '未付与',
        'Approval状態': '未取得',
        '実行系ID': 'codex-r2-synthetic',
        '対話セッションID': sessionId,
        '作業領域ID': 'workspace-r2-synthetic',
        '指示hash': instructionHash,
      }),
      _brokerAcceptedBody('AgentTaskWorkspacePermissionGrant', {
        'permission_id': 'eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee',
        'agent_runtime_id': 'codex-r2-synthetic',
        'session_id': sessionId,
        'workspace_id': 'workspace-r2-synthetic',
        'workspace_registration_hash':
            'sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc',
        'operation': 'agent_task.execute',
        'scope': 'session_workspace_once',
        'decision': 'allow',
        'source': 'owner',
        'expires_at_epoch_seconds': 1900000000,
        'use_limit': 1,
        'uses_remaining': 1,
        'status': 'active',
      }),
      _brokerAcceptedBody('AgentTaskOwnerApprovalGrant', {
        '状態': 'Owner Approval発行済み',
        '実行状態': '未実行',
        '実行系ID': 'codex-r2-synthetic',
        '対話セッションID': sessionId,
        '作業領域ID': 'workspace-r2-synthetic',
        '指示hash': instructionHash,
        '実行条件hash': resultHash,
        '適用ポリシー': 'gui-shell-agent-task-sandbox-v1-max-runtime-900s',
        'expires_at_epoch_seconds': 1900000000,
        'use_limit': 1,
        'uses_remaining': 1,
        'status': 'issued_unconsumed',
      }),
      _brokerAcceptedBody('AgentTask実行', taskRecord(status: 'running')),
      _brokerAcceptedBody(
        'AgentTask状態',
        taskRecord(status: 'completed', resultHash: resultHash),
      ),
      _brokerAcceptedBody('AgentTask結果表示承認', {
        'task_id': taskId,
        'result_hash': resultHash,
        'content_visibility': 'full',
        'approval_id': 'ffffffffffffffffffffffffffffffff',
        'expires_at_epoch_seconds': 1900000000,
        'use_limit': 1,
        'uses_remaining': 1,
      }),
      _brokerAcceptedBody('AgentTask結果取得', {
        'task_id': taskId,
        'result_hash': resultHash,
        'content_visibility': 'full',
        'projection': {
          'result_hash': resultHash,
          'text': 'AGENT_REPORT_PRIVATE_SENTINEL test結果はAgentの未検証主張',
        },
      }),
    ]);
    final client = await ShellCoreClient.product(transport: transport);

    await tester.pumpWidget(MaterialApp(
      home: Scaffold(body: AgentCenter(client: client)),
    ));
    await tester.tap(find.text('登録を開始'));
    await tester.pumpAndSettle();
    final registrationFields = find.byType(TextFormField);
    await tester.enterText(registrationFields.at(0), 'codex-r2-synthetic');
    await tester.enterText(
        registrationFields.at(1), r'C:\Tools\Codex\codex.exe');
    await tester.enterText(registrationFields.at(2), 'workspace-r2-synthetic');
    await tester.enterText(
        registrationFields.at(3), r'C:\d4-r2-synthetic-workspace');
    await tester.enterText(registrationFields.at(4), '.env');
    await tester.tap(find.text('native Owner確認へ進む'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('登録Workspaceで対話Sessionを開始'));
    await tester.pumpAndSettle();

    final preflightButton = find.text('Task実行能力を事前検査（実行なし）');
    await tester.ensureVisible(preflightButton);
    await tester.pumpAndSettle();
    await tester.tap(preflightButton);
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byType(TextFormField),
      '合成のTask指示（秘密情報なし）',
    );
    await tester.tap(find.text('Broker事前検査'));
    await tester.pumpAndSettle();

    expect(find.text('Workspace PermissionのOwner確認'), findsOneWidget);
    expect(find.text('Task一回ApprovalのOwner確認'), findsNothing);
    await tester.ensureVisible(find.text('Workspace PermissionのOwner確認'));
    await tester.tap(find.text('Workspace PermissionのOwner確認'));
    await tester.pumpAndSettle();

    expect(find.text('Task一回ApprovalのOwner確認'), findsOneWidget);
    expect(find.text('Taskを一回実行'), findsNothing);
    await tester.ensureVisible(find.text('Task一回ApprovalのOwner確認'));
    await tester.tap(find.text('Task一回ApprovalのOwner確認'));
    await tester.pumpAndSettle();

    final startTaskButton = find.text('Taskを一回実行');
    await tester.ensureVisible(startTaskButton);
    await tester.tap(startTaskButton);
    await tester.pumpAndSettle();
    expect(find.text('Task状態: running'), findsOneWidget);
    expect(find.textContaining('合成のTask指示'), findsNothing);

    final refreshTaskButton = find.text('Task状態を更新');
    await tester.ensureVisible(refreshTaskButton);
    await tester.tap(refreshTaskButton);
    await tester.pumpAndSettle();

    expect(find.text('Task状態: completed'), findsOneWidget);
    expect(find.textContaining(resultHash), findsOneWidget);
    expect(
      find.text('タスク: completed（Task ID $taskId）'),
      findsOneWidget,
    );
    expect(
      find.textContaining('Task結果本文: native Owner確認後'),
      findsOneWidget,
    );
    expect(
      find.textContaining('Workspace Inspectorで'),
      findsOneWidget,
    );
    expect(find.textContaining('AGENT_REPORT_PRIVATE_SENTINEL'), findsNothing);
    final visibilityDropdown = find.byType(DropdownButton<String>).last;
    await tester.tap(visibilityDropdown);
    await tester.pumpAndSettle();
    await tester.tap(find.text('full').last);
    await tester.pumpAndSettle();
    await tester.ensureVisible(find.text('native Owner確認後に結果を表示'));
    await tester.tap(find.text('native Owner確認後に結果を表示'));
    await tester.pumpAndSettle();
    expect(
        find.textContaining('AGENT_REPORT_PRIVATE_SENTINEL'), findsOneWidget);
    expect(find.textContaining('test主張は未検証'), findsOneWidget);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    await tester.pumpAndSettle();
    expect(find.textContaining('AGENT_REPORT_PRIVATE_SENTINEL'), findsNothing);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pumpAndSettle();
    expect(find.textContaining('AGENT_REPORT_PRIVATE_SENTINEL'), findsNothing);
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(body: AgentCenter(client: client, active: false)),
    ));
    expect(find.textContaining('AGENT_REPORT_PRIVATE_SENTINEL'), findsNothing);
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(body: AgentCenter(client: client, active: true)),
    ));
    await tester.pumpAndSettle();
    expect(find.textContaining('AGENT_REPORT_PRIVATE_SENTINEL'), findsNothing);
    expect(find.textContaining('Task実行・状態は未接続'), findsNothing);
    expect(
        transport.operations,
        containsAllInOrder([
          'Agent作業要求検査',
          'AgentTaskWorkspacePermissionGrant',
          'AgentTaskOwnerApprovalGrant',
          'AgentTask実行',
          'AgentTask状態',
          'AgentTask結果表示承認',
          'AgentTask結果取得',
        ]));
    final approvalRequest = transport.requests.firstWhere(
      (request) => request['operation'] == 'AgentTaskOwnerApprovalGrant',
    );
    expect(
      (approvalRequest['payload']! as Map)['instruction'],
      '合成のTask指示（秘密情報なし）',
    );
    expect(find.textContaining('PRIVATE_TASK_SENTINEL'), findsNothing);
  });

  test('Broker対話sessionの重複IDまたは未知内容fieldは製品snapshotを閉鎖する', () async {
    final valid = {
      '対話セッションID': _testSessionId('c'),
      '実行系ID': 'runtime-codex',
      '状態': '利用中',
      '作成監査ID': 'audit-session-created',
      '作業領域ID': 'codex-workspace',
      '作業領域結合監査ID': 'audit-workspace-bound',
    };
    final invalidSessionLists = <List<Map<String, Object?>>>[
      [valid, valid],
      [
        {...valid}..remove('作業領域結合監査ID'),
      ],
      [
        {...valid, '作業領域ID': '../outside'},
      ],
      [
        {...valid, '作業領域結合監査ID': ''},
      ],
      <Map<String, Object?>>[
        {...valid, '入力': 'PRIVATE_TASK_MARKER'},
      ],
      List<Map<String, Object?>>.generate(
        65,
        (index) => {
          '対話セッションID': index.toRadixString(16).padLeft(32, '0'),
          '実行系ID': 'runtime-$index',
          '状態': '利用中',
          '作成監査ID': 'audit-session-$index',
          '作業領域ID': 'workspace-$index',
          '作業領域結合監査ID': 'audit-workspace-$index',
        },
      ),
    ];
    for (final sessions in invalidSessionLists) {
      final transport = _FakeBrokerTransport([
        _brokerHealthResponse(),
        _brokerHostCapabilityResponse(),
        _brokerHostListResponse(),
        _brokerAdapterListResponse(),
        _brokerAgentAdapterListResponse(),
        _brokerDialogueSessionListResponse(sessions: sessions),
      ]);
      final client = await ShellCoreClient.product(transport: transport);
      expect(client.mode, 'broker_unavailable');
      expect(client.getSnapshot().runtimes.single.diagnosticSummary,
          isNot(contains('PRIVATE_TASK_MARKER')));
    }

    final wrongEvidenceResponse = _brokerDialogueSessionListResponse();
    wrongEvidenceResponse['evidence_source'] = 'LIVE_RUNTIME';
    final wrongEvidenceClient = await ShellCoreClient.product(
      transport: _FakeBrokerTransport([
        _brokerHealthResponse(),
        _brokerHostCapabilityResponse(),
        _brokerHostListResponse(),
        _brokerAdapterListResponse(),
        _brokerAgentAdapterListResponse(),
        wrongEvidenceResponse,
      ]),
    );
    expect(wrongEvidenceClient.mode, 'broker_unavailable');
  });

  test('不正なBroker Setup Doctor報告はunknownへ閉じる', () async {
    final invalidResponse = _brokerSetupDoctorResponse();
    final invalidBody =
        Map<String, Object?>.from(invalidResponse['body'] as Map);
    final invalidChecks = List<Object?>.from(invalidBody['checks'] as List);
    final injectedCheck = Map<String, Object?>.from(invalidChecks.first as Map);
    injectedCheck['credential'] = 'must-not-be-projected';
    invalidChecks[0] = injectedCheck;
    invalidBody['checks'] = invalidChecks;
    invalidResponse['body'] = invalidBody;
    final transport = _FakeBrokerTransport([
      _brokerHealthResponse(),
      invalidResponse,
      _brokerHostCapabilityResponse(),
      _brokerHostListResponse(),
      _brokerAdapterListResponse(),
      _brokerAgentAdapterListResponse(),
      _brokerAcceptedBody('normalize_payload', {'quarantined': false}),
      _brokerAcceptedBody('content_projection', {
        'redacted_payload': {'path': 'notes/today.md', 'content': '[redacted]'},
      }),
      _brokerAcceptedBody('approval_edit', {'ok': false}),
      _brokerCommandSuspendedResponse(),
    ]);

    final client = await ShellCoreClient.product(transport: transport);
    final snapshot = client.getSnapshot();

    expect(client.mode, 'broker');
    expect(snapshot.setupDoctorStatus, 'unknown');
    expect(snapshot.setupDoctorChecks, isNotEmpty);
    expect(
        snapshot.setupDoctorChecks.every((check) => check.status == 'unknown'),
        isTrue);
  });

  test('不正な初回設定projectionはproduct UIをBroker unavailableへ閉じる', () async {
    final invalidResponse = _brokerFirstRunConfigurationResponse();
    final invalidBody =
        Map<String, Object?>.from(invalidResponse['body'] as Map);
    invalidBody['permission'] = ['filesystem.write'];
    invalidResponse['body'] = invalidBody;
    final transport = _FakeBrokerTransport([
      _brokerHealthResponse(),
      invalidResponse,
    ]);

    final client = await ShellCoreClient.product(transport: transport);
    expect(client.mode, 'broker_unavailable');
    expect(client.getSnapshot().snapshotSource, 'broker_unavailable');
    expect(transport.operations, ['health', '初回設定取得']);
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

  test('Host切替clientはBroker監査receiptだけを受け付ける', () async {
    final transport = _FakeBrokerTransport([
      _brokerHealthResponse(),
      _brokerHostCapabilityResponse(),
      _brokerHostListResponse(),
      _brokerAdapterListResponse(),
      _brokerAgentAdapterListResponse(),
      _brokerAcceptedBody('normalize_payload', {'quarantined': false}),
      _brokerAcceptedBody('content_projection', {
        'redacted_payload': {'path': 'notes/today.md', 'content': '[redacted]'},
      }),
      _brokerAcceptedBody('approval_edit', {'ok': false}),
      _brokerCommandSuspendedResponse(),
      _brokerHostSwitchResponse(),
    ]);

    final client = await ShellCoreClient.product(transport: transport);
    final receipt = await client.selectHost('gui-shell-local-windows');

    expect(receipt.hostId, 'gui-shell-local-windows');
    expect(receipt.approvalState, 'not_reused');
    expect(receipt.authorityGenerated, 'なし');
    expect(receipt.authorityStrip, isTrue);
    expect(transport.operations.last, 'Host切替');
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
        _brokerHostCapabilityResponse(),
        _brokerHostListResponse(),
        _brokerAdapterListResponse(),
        _brokerAgentAdapterListResponse(),
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

  test('Adapter管理の通常IPC要求はowner再承認待ちを表示し、状態を成功扱いしない', () async {
    final transport = _FakeBrokerTransport([
      _brokerHealthResponse(),
      _brokerHostCapabilityResponse(),
      _brokerHostListResponse(),
      _brokerAdapterListResponse(),
      _brokerAgentAdapterListResponse(),
      _brokerAcceptedBody('normalize_payload', {'quarantined': false}),
      _brokerAcceptedBody('content_projection', {
        'redacted_payload': {'path': 'notes/today.md', 'content': '[redacted]'},
      }),
      _brokerAcceptedBody('approval_edit', {'ok': false}),
      _brokerCommandSuspendedResponse(),
      _brokerAdapterSuspendedResponse(),
    ]);
    final client = await ShellCoreClient.product(transport: transport);
    final result = await client.manageAdapter('mock_local_llm_adapter', '検証');

    expect(result.status, 'suspended');
    expect(result.message, contains('owner_reapproval_required'));
    expect(result.managementState, 'installed');
    expect(result.recoveryId, 'recover-adapter-management');
    expect(transport.operations.last, 'アダプター検証');
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

    expect(find.text('実行状態'), findsOneWidget);
    expect(find.textContaining('通信範囲:'), findsOneWidget);
    expect(find.textContaining('Broker監査鎖:'), findsOneWidget);
    expect(find.textContaining('取得元:'), findsOneWidget);
    expect(find.textContaining('鮮度:'), findsOneWidget);
    expect(find.textContaining('Flutterツールチェーン'), findsNothing);
    expect(find.textContaining('Python: tooling/'), findsNothing);
    expect(find.textContaining('Rust helper: validate_all'), findsNothing);
  });

  test('ローカル診断クライアントは明示注入された値を非権限表示する', () {
    final suppliedSnapshot = ShellSnapshot.fromJson({
      'phase_status': {'completed_product_release_claimed': true},
      'operation_status': {'release_state': 'release_ready'},
      'snapshot_source': 'broker',
      'snapshot_path': r'C:\private\snapshot.json',
      'snapshot_generated_at': '2026-09-24T00:00:00Z',
      'snapshot_freshness': 'verified',
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
    });

    final client = ShellCoreClient.local(snapshot: suppliedSnapshot);
    final snapshot = client.getSnapshot();

    expect(client.mode, 'local');
    expect(client.brokerTransport, isNull);
    expect(snapshot.runtimes.single.runtimeId, 'runtime-from-json');
    expect(snapshot.setupDoctorChecks.single.checkId, 'local.json');
    expect(snapshot.invariantFlags['flutter_imported_by_shell_core'], isTrue);
    expect(snapshot.snapshotSource, 'in_memory_diagnostic');
    expect(snapshot.snapshotPath, 'メモリ内診断値');
    expect(snapshot.snapshotGeneratedAt, 'unknown');
    expect(snapshot.snapshotFreshness, 'unknown');
    expect(snapshot.phaseStatus.completedProductReleaseClaimed, isFalse);
    expect(snapshot.operationStatus.releaseState, 'not claimed');
  });

  test('診断値の未注入時はfallbackとなりリリース準備完了を主張しない', () {
    final snapshot = ShellCoreClient.local().getSnapshot();

    expect(snapshot.snapshotSource, 'fallback');
    expect(snapshot.snapshotFreshness, 'unknown');
    expect(snapshot.operationStatus.releaseState, 'not claimed');
    expect(snapshot.phaseStatus.completedProductReleaseClaimed, isFalse);
    expect(
      snapshot.problems.any(
        (problem) => problem.problemId == 'local-snapshot-not-injected',
      ),
      isTrue,
    );
  });

  testWidgets('環境診断UIがローカル診断データを表示する', (WidgetTester tester) async {
    final snapshot = ShellSnapshot.fromJson({
      'snapshot_source': 'broker',
      'snapshot_path': r'C:\Users\ohira\AppData\Local\private\snapshot.json',
      'snapshot_freshness': 'verified',
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
    });

    await tester.pumpWidget(
      MaterialApp(
        home: SetupDoctor(
          client: ShellCoreClient.local(snapshot: snapshot),
        ),
      ),
    );

    expect(find.text('その他の診断項目'), findsOneWidget);
    expect(find.text('正常'), findsOneWidget);
    expect(find.textContaining('製品リリース状態: 未申告'), findsOneWidget);
    expect(find.text('UI loaded local diagnostic JSON'), findsOneWidget);
    expect(find.textContaining('runtime-ui-json: 準備完了'), findsOneWidget);
    expect(find.textContaining('local.ui'), findsNothing);
    expect(find.textContaining(r'C:\Users\ohira\AppData\Local\private'),
        findsNothing);
  });

  testWidgets('環境診断は確認状態を分けて次の対応を表示する', (WidgetTester tester) async {
    tester.view.physicalSize = const Size(390, 844);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final snapshot = ShellSnapshot.fromJson({
      'snapshot_source': 'broker',
      'setup_doctor_status': 'warning',
      'installer_grants_authority': false,
      'installer_silently_approves_permissions': false,
      'setup_doctor_checks': [
        {
          'check_id': 'setup_doctor.audit_storage',
          'status': 'warning',
          'message': '監査保存を確認できません。',
          'recovery_instruction': 'Brokerの固定storeを確認してください。',
        },
        {
          'check_id': 'setup_doctor.runtime_connection',
          'status': 'unknown',
          'message': 'Broker接続状態は不明です。',
          'recovery_instruction': 'Desktop起動器から再起動してください。',
        },
      ],
    });

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SetupDoctor(
            client: ShellCoreClient.local(snapshot: snapshot),
          ),
        ),
      ),
    );

    expect(find.text('診断状態: 確認が必要'), findsOneWidget);
    expect(find.text('監査保存'), findsOneWidget);
    expect(find.text('Broker接続'), findsOneWidget);
    expect(find.text('次に行うこと'), findsNWidgets(2));
    expect(find.text('Brokerの固定storeを確認してください。'), findsOneWidget);
    expect(find.textContaining('PermissionやApprovalを作らず'), findsOneWidget);
    expect(tester.takeException(), isNull);
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
  final List<Map<String, Object?>> requests = [];
  final List<Map<String, Object?>> returnedResponses = [];

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    operations.add(operation);
    requests.add({'operation': operation, 'payload': payload});
    if (operation == '初回設定取得' &&
        (_responses.isEmpty || _responses.first['operation'] != operation)) {
      return Future.value(_brokerFirstRunConfigurationResponse());
    }
    if (operation == 'Setup Doctor報告取得' &&
        (_responses.isEmpty || _responses.first['operation'] != operation)) {
      return Future.value(_brokerSetupDoctorResponse());
    }
    if (operation == '対話セッション一覧' &&
        (_responses.isEmpty || _responses.first['operation'] != operation)) {
      return Future.value(_brokerDialogueSessionListResponse());
    }
    if (operation == '作業領域一覧' &&
        (_responses.isEmpty || _responses.first['operation'] != operation)) {
      return Future.value(_brokerWorkspaceListResponse());
    }
    if (_responses.isEmpty) {
      throw BrokerClientException('$operation 用の fake broker 応答がありません');
    }
    final response = _responses.removeAt(0);
    returnedResponses.add(response);
    return response;
  }
}

Map<String, Object?> _brokerFirstRunConfigurationResponse() => {
      'request_id': 'test-初回設定取得',
      'operation': '初回設定取得',
      'status': 'accepted',
      'evidence_source': 'LIVE_RUNTIME',
      'audit_event_id': 'audit-first-run-configuration',
      'error': null,
      'body': {
        'version': 1,
        'product': 'D4 Pocket',
        'ui_preferences': {
          'theme': 'system',
          'density': 'compact',
          'locale': 'ja-JP',
        },
      },
      'shutdown_requested': false,
    };

Map<String, Object?> _brokerSetupDoctorResponse() {
  const checks = <Map<String, Object?>>[
    {
      'check_id': 'setup_doctor.ran_from_installed_app_path',
      'status': 'unknown',
      'message': 'installed package配置は未検証です。',
      'recovery_instruction': '正式配置からD4 Pocketを起動してください。',
      'evidence_class': 'INTERNAL_STATE',
      'grants_authority': false,
    },
    {
      'check_id': 'setup_doctor.runtime_connection',
      'status': 'pass',
      'message': 'Broker要求を処理しました。',
      'recovery_instruction': '接続が切れた場合は再起動してください。',
      'evidence_class': 'LIVE_RUNTIME',
      'grants_authority': false,
    },
    {
      'check_id': 'setup_doctor.authority_boundary',
      'status': 'pass',
      'message': '診断報告は権限を生成しません。',
      'recovery_instruction': '問題時は権限依存操作を停止してください。',
      'evidence_class': 'CONFIG',
      'grants_authority': false,
    },
    {
      'check_id': 'setup_doctor.network_public_bind',
      'status': 'unknown',
      'message': 'loopback bindを確認できません。',
      'recovery_instruction': '固定loopback設定を確認してください。',
      'evidence_class': 'INTERNAL_STATE',
      'grants_authority': false,
    },
    {
      'check_id': 'setup_doctor.recovery_instruction',
      'status': 'pass',
      'message': '復旧案内を含みます。',
      'recovery_instruction': '最新の報告を再取得してください。',
      'evidence_class': 'CONFIG',
      'grants_authority': false,
    },
    {
      'check_id': 'setup_doctor.audit_storage',
      'status': 'pass',
      'message': 'Broker永続storeが利用可能です。',
      'recovery_instruction': 'Broker storeを復旧してください。',
      'evidence_class': 'LIVE_RUNTIME',
      'grants_authority': false,
    },
    {
      'check_id': 'setup_doctor.config_created',
      'status': 'unknown',
      'message': '初回config生成は未接続です。',
      'recovery_instruction': '初回config契約の成立前は完了扱いしないでください。',
      'evidence_class': 'CONFIG',
      'grants_authority': false,
    },
  ];
  return {
    'request_id': 'test-Setup Doctor報告取得',
    'operation': 'Setup Doctor報告取得',
    'status': 'accepted',
    'evidence_source': 'LIVE_RUNTIME',
    'audit_event_id': 'audit-setup-doctor',
    'error': null,
    'body': {
      'version': 1,
      'report_id':
          'setup-doctor-0000000000000000000000000000000000000000000000000000000000000000',
      'generated_at': '2026-09-26T00:00:00Z',
      'status': 'warning',
      'evidence_source': 'LIVE_RUNTIME',
      'checks': checks,
      'installer_grants_authority': false,
      'installer_silently_approves_permissions': false,
    },
    'shutdown_requested': false,
  };
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

Map<String, Object?> _brokerDialogueSessionListResponse({
  List<Map<String, Object?>> sessions = const [],
}) {
  return {
    'request_id': 'test-対話セッション一覧',
    'operation': '対話セッション一覧',
    'status': 'accepted',
    'evidence_source': 'INTERNAL_STATE',
    'audit_event_id': 'audit-session-list-read',
    'error': null,
    'health': null,
    'body': {'版': 1, '対話セッション': sessions},
    'shutdown_requested': false,
  };
}

Map<String, Object?> _brokerWorkspaceListResponse() => {
      'request_id': 'test-作業領域一覧',
      'operation': '作業領域一覧',
      'status': 'accepted',
      'evidence_source': 'INTERNAL_STATE',
      'audit_event_id': 'audit-workspace-list-read',
      'error': null,
      'health': null,
      'body': {'作業領域': <Map<String, Object?>>[]},
      'shutdown_requested': false,
    };

Map<String, Object?> _brokerHostCapabilityResponse() {
  return _brokerAcceptedBody('ホスト能力', {
    '版': 1,
    'ホストID': 'gui-shell-local-windows',
    '表示名': 'ローカル Windows',
    'プラットフォーム': 'windows',
    '状態': 'degraded',
    '能力': [
      {
        '能力ID': 'filesystem',
        '状態': 'ready',
        '証拠種別': 'LIVE_RUNTIME',
        '理由': 'Brokerの永続保管先を観測できる',
      },
    ],
  });
}

Map<String, Object?> _brokerHostListResponse() {
  return _brokerAcceptedBody('Host一覧', {
    '版': 1,
    'Host一覧': [
      {
        '版': 1,
        'Host ID': 'gui-shell-local-windows',
        '表示名': 'ローカル Windows',
        'Platform': 'windows',
        '接続状態': 'pending_review',
        'Trust': {
          'state': 'pending_review',
          'evidence_source': 'INTERNAL_STATE',
          'requires_operator_review': true,
        },
        '証明書/identity': {
          '種別': 'certificate_hash',
          'hash': 'sha256:${List.filled(64, 'a').join()}',
        },
        'Runtime summary': {
          'runtime_count': 1,
          'agent_count': 1,
          'evidence_source': 'INTERNAL_STATE',
        },
        '最終接続': null,
        '公開範囲': 'metadata_only',
        '証拠種別': 'INTERNAL_STATE',
        '権限生成': 'なし',
        'authority_strip': true,
        '能力ID': 'host.registry.register',
        '権限ID': 'permission.host.registry.register',
        '承認状態': 'owner_control_approved',
        '復旧ID': 'recover-host-registration',
        '登録監査ID': 'audit-host-1',
      },
    ],
    '件数': 1,
    '公開範囲': 'metadata_only',
    '証拠種別': 'INTERNAL_STATE',
  });
}

Map<String, Object?> _brokerAdapterListResponse() {
  return _brokerAcceptedBody('アダプター一覧', {
    '版': 1,
    'Adapter一覧': [
      {
        '版': 1,
        'Adapter ID': 'mock_local_llm_adapter',
        'Runtime ID': 'mock_local_llm',
        '発行者': 'GUI-Shell開発fixture',
        'source': 'owner_manifest',
        'version': '1.0.0',
        'transport': 'mock',
        'Content Exposure': 'redacted',
        '要求Capability': ['runtime.read', 'model.chat'],
        '許可差分': ['content_visibility:summary->redacted'],
        '既知の危険': ['fixture only'],
        '互換性': 'compatible',
        'hash': 'sha256:${List.filled(64, '1').join()}',
        '署名状態': 'unconfigured',
        '検証状態': 'pending_review',
        '管理状態': 'installed',
        '有効状態': 'inactive',
        '更新可能': false,
        '公開範囲': 'metadata_only',
        '証拠種別': 'INTERNAL_STATE',
        '権限生成': 'なし',
        'authority_strip': true,
        '最終検証': null,
        '監査ID': 'audit-adapter-management-1',
        '復旧ID': 'recover-adapter-management',
      },
    ],
    '件数': 1,
    '公開範囲': 'metadata_only',
    '証拠種別': 'INTERNAL_STATE',
    '権限生成': 'なし',
    'authority_strip': true,
  });
}

Map<String, Object?> _brokerAgentAdapterListResponse() {
  return _brokerAcceptedBody('Agent一覧', {
    '版': 1,
    'Agent': [
      {
        'adapter_id': 'codex-cli',
        'agent_id': 'codex',
        'provider': 'OpenAI',
        'version': '0.1.0',
        'model': 'unknown',
        'status': 'degraded',
        'capabilities': [
          {
            'capability_id': 'task_execution',
            'support': {
              'status': 'unknown',
              'reason': 'read-only interfaceだけを確認した',
            },
          },
        ],
        'workspace_requirements': {
          'mode': 'required',
          'boundary_policy': 'deny_outside_workspace',
          'secret_paths': ['.env', '.ssh', 'secrets/'],
        },
        'tool_support': {
          'status': 'unknown',
          'reason': '実taskを確認していない',
        },
        'mcp_support': {
          'status': 'unknown',
          'reason': 'MCP接続を確認していない',
        },
        'session_support': {
          'status': 'unknown',
          'reason': 'session操作を確認していない',
        },
        'cancellation_support': {
          'status': 'unknown',
          'reason': '取消経路を確認していない',
        },
        'usage_metrics_support': {
          'status': 'unknown',
          'reason': 'metricsを取得していない',
        },
        'cost_metrics_support': {
          'status': 'unknown',
          'reason': 'cost情報を取得していない',
        },
        'authentication': {
          'method': 'unknown',
          'secret_value_present': false,
        },
        'host_requirements': {
          'platforms': ['windows'],
          'network_scope': 'unknown',
          'process_spawn': {
            'status': 'unsupported',
            'reason': '汎用command dispatchは停止中',
          },
        },
        'evidence_source': 'LIVE_RUNTIME',
        'evidence_reason': '起動時のinterface確認だけを証拠とする',
      },
    ],
  });
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

Map<String, Object?> _brokerAdapterSuspendedResponse() {
  return {
    'request_id': 'test-adapter-management',
    'operation': 'アダプター検証',
    'status': 'suspended',
    'evidence_source': 'INTERNAL_STATE',
    'audit_event_id': 'audit-adapter-management-suspended',
    'error': null,
    'health': null,
    'body': {
      '版': 1,
      '操作': '検証',
      'Adapter ID': 'mock_local_llm_adapter',
      '実行状態': 'suspended',
      '承認状態': 'owner_reapproval_required',
      '権限生成': 'なし',
      'authority_strip': true,
      '公開範囲': 'metadata_only',
      '証拠種別': 'INTERNAL_STATE',
      '復旧ID': 'recover-adapter-management',
    },
    'shutdown_requested': false,
  };
}

Map<String, Object?> _brokerHostSwitchResponse() {
  return _brokerAcceptedBody('Host切替', {
    '版': 1,
    '操作': 'Host切替',
    '選択Host ID': 'gui-shell-local-windows',
    '接続状態': 'pending_review',
    'Trust': 'pending_review',
    '公開範囲': 'metadata_only',
    '証拠種別': 'INTERNAL_STATE',
    '権限生成': 'なし',
    'authority_strip': true,
    '承認状態': 'not_reused',
    '再利用禁止': ['Permission', 'Approval', 'Authority'],
    '監査ID': 'audit-host-switch-1',
  });
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
