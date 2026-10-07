import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/foundation.dart' show TargetPlatform;
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
import 'package:gui_shell_desktop/screens/host_capability_center.dart';
import 'package:gui_shell_desktop/screens/host_operation_center.dart';
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
  testWidgets('macOS作業領域OS選択は取消で入力保持し投影だけを反映する', (tester) async {
    Map<String, Object?> projection(String state, String? root,
            {bool approval = false}) =>
        {
          ..._brokerAcceptedBody('作業領域OS選択', {
            'version': 1,
            'selection_status': state,
            'workspace_root': root,
            'selection_request_hash': brokerPayloadHash({'version': 1}),
            'scope_lifetime': 'broker_process',
            'permission_generated': false,
            'approval_generated': approval,
            'registration_generated': false,
          }),
          'evidence_source': 'INTERNAL_STATE',
        };
    final transport = _FakeBrokerTransport([
      ..._shellCoreProductBootstrapResponses(),
      projection('cancelled', null),
      projection('selected', '/public-project'),
      projection('selected', '/injected-root', approval: true),
    ]);
    final client = await ShellCoreClient.product(transport: transport);
    await tester.pumpWidget(
        MaterialApp(home: Scaffold(body: AgentCenter(client: client))));
    await tester.tap(find.text('登録を開始'));
    await tester.pumpAndSettle();
    final root = find.byType(TextFormField).at(4);
    await tester.enterText(root, '/previous-input');
    final select = find.byKey(const ValueKey('macos-select-workspace'));
    for (final expected in [
      '/previous-input',
      '/public-project',
      '/public-project'
    ]) {
      await tester.ensureVisible(select);
      await tester.tap(select);
      await tester.pumpAndSettle();
      expect(tester.widget<TextFormField>(root).controller!.text, expected);
    }
    expect(find.text('OS選択が成立していません。入力を保持し、自動再送しません。'), findsOneWidget);
    final selections =
        transport.requests.where((r) => r['operation'] == '作業領域OS選択').toList();
    expect(selections.length, 3);
    for (final request in selections) {
      expect(request['payload'], {'version': 1});
    }
    expect(transport.operations, isNot(contains('AgentCLI実行系作業領域登録')));
    expect(transport.operations,
        isNot(contains('AgentTaskWorkspacePermissionGrant')));
    expect(tester.takeException(), isNull);
  }, variant: TargetPlatformVariant.only(TargetPlatform.macOS));
  Finder findSurfaceSemanticsIdentifier(String label) {
    final identifier = surfaceSemanticsIdentifier(label);
    return find.byWidgetPredicate(
      (widget) =>
          widget is Semantics && widget.properties.identifier == identifier,
      description: 'Semantics(identifier: $identifier)',
    );
  }

  test('Agent Adapter Provider状態を観測値として保持する', () {
    final adapter = AgentAdapterRecord.fromJson({
      'adapter_id': 'codex-cli',
      'agent_id': 'codex',
      'provider': 'OpenAI',
      'provider_id': 'openai_codex_cli',
      'version': '0.160.0',
      'model': 'model-test-01',
      'status': 'degraded',
      'capabilities': [
        {
          'capability_id': 'model_selection',
          'support': {'status': 'supported'},
        },
      ],
      'provider_health': {'status': 'unknown'},
      'automatic_fallback': false,
      'authentication': {
        'method': 'codex_cli_managed',
        'status': 'unknown',
        'secret_value_present': false,
      },
      'evidence_source': 'LIVE_RUNTIME',
      'evidence_reason': 'CLI interface確認のみ',
    });

    expect(adapter.providerHealthStatus, 'unknown');
    expect(adapter.automaticFallback, isFalse);
    expect(adapter.authenticationMethod, 'codex_cli_managed');
    expect(adapter.authenticationStatus, 'unknown');
    expect(adapter.model, 'model-test-01');
  });

  testWidgets('履歴の遷移先がナビゲーションに存在し離脱できる', (tester) async {
    await tester.pumpWidget(const GuiShellDesktopApp());
    final rail = tester.widget<NavigationRail>(find.byType(NavigationRail));
    expect(rail.destinations.length, 21);
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
    expect(rail.destinations.length, 21);
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
    expect(find.textContaining('未確認（Broker由来の実測証拠がありません）'), findsOneWidget);
    expect(find.textContaining('未観測（snapshotはBroker確定経路ではありません'), findsWidgets);
    expect(find.textContaining('/ Broker観測'), findsNothing);
    expect(find.text('現在のHost'), findsOneWidget);
    expect(find.textContaining('Permission、Approval、Authority'), findsWidgets);
  });

  testWidgets('ホスト能力mockはBroker実測またはlocal／remoteの証拠へ昇格しない', (tester) async {
    await tester.pumpWidget(const GuiShellDesktopApp());

    final rail = tester.widget<NavigationRail>(find.byType(NavigationRail));
    rail.onDestinationSelected!(15);
    await tester.pumpAndSettle();

    expect(find.text('D4 Pocket ホスト能力'), findsOneWidget);
    expect(find.textContaining('試験・診断用の表示です'), findsOneWidget);
    expect(find.textContaining('所在: 未確認'), findsOneWidget);
    expect(find.textContaining('fixture／診断値をBroker実測へ昇格しない'), findsOneWidget);
  });

  testWidgets('ホスト能力画面はBroker受理snapshotの状態を表示情報に限定する', (tester) async {
    await tester.pumpWidget(MaterialApp(
      home:
          Scaffold(body: HostCapabilityCenter(client: ShellCoreClient.mock())),
    ));

    expect(find.textContaining('PermissionやApprovalは生成しません'), findsOneWidget);
    expect(find.textContaining('fixture／診断値をBroker実測へ昇格しない'), findsOneWidget);
    expect(find.textContaining('表示用状態（未観測）: degraded'), findsOneWidget);
  });

  testWidgets('診断snapshotの別登録HostをRuntime／Agentのlive一覧へ昇格しない', (tester) async {
    final base = ShellCoreClient.mock().getSnapshot();
    const registeredOtherHost = HostRegistryRecord(
      hostId: 'registered-other-host',
      displayName: '登録済み別Host',
      platform: 'linux',
      connectionState: 'pending_review',
      trustState: 'pending_review',
      runtimeCount: 2,
      agentCount: 1,
      evidenceSource: 'INTERNAL_STATE',
      visibility: 'metadata_only',
      authorityStrip: true,
    );
    final client = ShellCoreClient.local(
      snapshot: base.copyWith(hosts: [...base.hosts, registeredOtherHost]),
    );
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(body: HostOperationCenter(client: client)),
    ));
    await tester.tap(find.text('登録済み別Host').first);
    await tester.pumpAndSettle();

    expect(
      find.textContaining('未確認（Broker由来の実測証拠がありません）'),
      findsOneWidget,
    );
    expect(find.textContaining('Runtime summary: 2件 / Agent summary: 1件'),
        findsOneWidget);
    for (final runtime in base.runtimes) {
      expect(find.text(runtime.runtimeId), findsNothing);
    }
    expect(find.textContaining('snapshotはBroker確定経路ではありません'), findsWidgets);
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

  testWidgets('概要から導入・更新センターへ直行できる', (WidgetTester tester) async {
    tester.view.physicalSize = const Size(1200, 1000);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(const GuiShellDesktopApp());
    await tester.pumpAndSettle();

    final openSetup = find.byKey(const ValueKey('open-product-setup'));
    expect(openSetup, findsOneWidget);
    await tester.tap(openSetup);
    await tester.pumpAndSettle();

    final updateCenterUnavailable =
        find.textContaining('更新センター: Broker接続がないため操作できません。');
    expect(updateCenterUnavailable, findsOneWidget);
    final updateCenterRect = tester.getRect(updateCenterUnavailable);
    expect(updateCenterRect.top, lessThan(1000));
    expect(updateCenterRect.bottom, greaterThan(0));
    expect(tester.takeException(), isNull);
  });

  testWidgets('first-runから登録Agent Taskを完了しHandoffと履歴へ接続する',
      (WidgetTester tester) async {
    const sessionId = 'c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1';
    const targetSessionId = 'f4f4f4f4f4f4f4f4f4f4f4f4f4f4f4f4';
    const taskId = 'd2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2';
    const instructionHash =
        'sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
    const resultHash =
        'sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
    const resultSummary = 'P12で承認済み結果を受信Taskへ渡す';
    const historyApprovalId = '77777777777777777777777777777777';
    final historyGrant = <String, Object?>{
      'approval_id': historyApprovalId,
      'runtime_id': 'codex-p12-synthetic',
      'expires_at': DateTime.now().millisecondsSinceEpoch ~/ 1000 + 240,
    };
    Map<String, Object?> historyResponse(
      String operation,
      Map<String, Object?> body,
    ) =>
        {
          'request_id': 'test-$operation',
          'operation': operation,
          'status': 'accepted',
          'evidence_source': 'INTERNAL_STATE',
          'audit_event_id': 'audit-$operation',
          'error': null,
          'health': null,
          'body': body,
          'shutdown_requested': false,
        };
    final approvedResult = jsonEncode({
      'result_summary': resultSummary,
      'artifacts': [
        {'name': 'note.md', 'content': 'Agent申告artifact'}
      ],
      'changed_files': ['note.md'],
      'diff': '+# Agent申告diff',
      'test_result': 'Agent申告test結果（独立検証なし）',
    });
    tester.view.physicalSize = const Size(1200, 1000);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final transport = _FakeBrokerTransport([
      _brokerHealthResponse(),
      _brokerHostCapabilityResponse(),
      _brokerHostListResponse(),
      _brokerAdapterListResponse(),
      _brokerAgentAdapterListResponse(),
      _brokerDialogueSessionListResponse(sessions: [
        {
          '対話セッションID': targetSessionId,
          '実行系ID': 'codex-p12-target',
          '状態': '利用中',
          '作成監査ID': 'audit-p12-target-session',
          '作業領域ID': 'workspace-p12-target',
          '作業領域結合監査ID': 'audit-p12-target-workspace',
        },
      ]),
      _brokerAcceptedBody('normalize_payload', {'quarantined': false}),
      _brokerAcceptedBody('content_projection', {'redacted_payload': {}}),
      _brokerAcceptedBody('approval_edit', {'ok': false}),
      _brokerCommandSuspendedResponse(),
      _brokerAcceptedBody('プロファイル一覧', {}),
      _brokerAcceptedBody('通知一覧', {}),
      _brokerAcceptedBody('観測一覧', {}),
      _brokerAcceptedBody('観測一覧', {}),
      _brokerAcceptedBody('A2A接続一覧', {}),
      _brokerAcceptedBody('AgentCLI実行系作業領域登録', {
        'runtime_id': 'codex-p12-synthetic',
        'workspace_id': 'workspace-p12-synthetic',
        'registration_lifetime': 'broker_process',
        'task_execution': 'supported',
        'permission_generated': false,
        'approval_generated': false,
        'credential_value_accepted': false,
      }),
      _brokerAcceptedBody('対話開始', {
        '対話セッションID': sessionId,
        '実行系ID': 'codex-p12-synthetic',
        '状態': '利用中',
      }),
      _brokerDialogueSessionListResponse(sessions: [
        {
          '対話セッションID': targetSessionId,
          '実行系ID': 'codex-p12-target',
          '状態': '利用中',
          '作成監査ID': 'audit-p12-target-session',
          '作業領域ID': 'workspace-p12-target',
          '作業領域結合監査ID': 'audit-p12-target-workspace',
        },
        {
          '対話セッションID': sessionId,
          '実行系ID': 'codex-p12-synthetic',
          '状態': '利用中',
          '作成監査ID': 'audit-p12-session-created',
          '作業領域ID': 'workspace-p12-synthetic',
          '作業領域結合監査ID': 'audit-p12-workspace-bound',
        },
      ]),
      _brokerAcceptedBody('Agent作業要求検査', {
        '版': 1,
        '状態': '要求検査済み',
        '実行状態': '未実行',
        'Permission状態': '未付与',
        'Approval状態': '未取得',
        '実行系ID': 'codex-p12-synthetic',
        '対話セッションID': sessionId,
        '作業領域ID': 'workspace-p12-synthetic',
        '指示hash': instructionHash,
      }),
      _brokerAcceptedBody('AgentTaskWorkspacePermissionGrant', {
        'permission_id': 'e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3',
        'agent_runtime_id': 'codex-p12-synthetic',
        'session_id': sessionId,
        'workspace_id': 'workspace-p12-synthetic',
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
        '実行系ID': 'codex-p12-synthetic',
        '対話セッションID': sessionId,
        '作業領域ID': 'workspace-p12-synthetic',
        '指示hash': instructionHash,
        '実行条件hash': resultHash,
        '適用ポリシー': 'gui-shell-agent-task-sandbox-v1-max-runtime-900s',
        'expires_at_epoch_seconds': 1900000000,
        'use_limit': 1,
        'uses_remaining': 1,
        'status': 'issued_unconsumed',
      }),
      _brokerAcceptedBody('AgentTask実行', {
        'task_id': taskId,
        'record_version': 2,
        'agent_runtime_id': 'codex-p12-synthetic',
        'session_id': sessionId,
        'workspace_id': 'workspace-p12-synthetic',
        'description': 'Agent作業Task（結果本文とWorkspace差分は別の権限経路）',
        'instruction_hash': instructionHash,
        'status': 'running',
        'audit_event_id': 'audit-p12-task-start',
        'result_content_available': false,
      }),
      _brokerAcceptedBody('AgentTask状態', {
        'task_id': taskId,
        'record_version': 2,
        'agent_runtime_id': 'codex-p12-synthetic',
        'session_id': sessionId,
        'workspace_id': 'workspace-p12-synthetic',
        'description': 'Agent作業Task（結果本文とWorkspace差分は別の権限経路）',
        'instruction_hash': instructionHash,
        'status': 'completed',
        'audit_event_id': 'audit-p12-task-complete',
        'result_hash': resultHash,
        'result_content_available': true,
      }),
      _brokerAcceptedBody('AgentTask結果表示承認', {
        'task_id': taskId,
        'result_hash': resultHash,
        'content_visibility': 'full',
        'approval_id': 'a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5',
        'expires_at_epoch_seconds': 1900000000,
        'use_limit': 1,
        'uses_remaining': 1,
      }),
      _brokerAcceptedBody('AgentTask結果取得', {
        'task_id': taskId,
        'result_hash': resultHash,
        'content_visibility': 'full',
        'projection': {'result_hash': resultHash, 'text': approvedResult},
      }),
      _brokerAcceptedBody('Agent作業要求検査', {
        '版': 1,
        '状態': '要求検査済み',
        '実行状態': '未実行',
        'Permission状態': '未付与',
        'Approval状態': '未取得',
        '実行系ID': 'codex-p12-target',
        '対話セッションID': targetSessionId,
        '作業領域ID': 'workspace-p12-target',
        '指示hash': instructionHash,
      }),
      historyResponse('対話履歴閲覧状態', {
        'grant': historyGrant,
        'page': null,
      }),
      historyResponse('対話履歴閲覧', {
        'grant': historyGrant,
        'page': {
          'version': 1,
          'entries': const <Object?>[],
          'next_cursor': 0,
          'has_more': false,
          'head_hash': null,
        },
      }),
      historyResponse('対話履歴閲覧状態', {
        'grant': historyGrant,
        'page': null,
      }),
      historyResponse('対話履歴閲覧状態', {
        'grant': historyGrant,
        'page': null,
      }),
      historyResponse('対話履歴閲覧状態', {
        'grant': historyGrant,
        'page': null,
      }),
      historyResponse('AgentTask履歴閲覧', {
        'grant': historyGrant,
        'task_page': {
          'version': 1,
          'entries': [
            {
              'audit_event_id': 'task-history-complete',
              'event_hash':
                  'sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc',
              'record': {
                'version': 1,
                'task_id': taskId,
                'runtime_id': 'codex-p12-synthetic',
                'session_id': sessionId,
                'workspace_id': 'workspace-p12-synthetic',
                'instruction_hash': instructionHash,
                'status': 'completed',
                'created_at': 1900000000,
                'updated_at': 1900000001,
                'result_hash': resultHash,
                'failure_class': null,
                'start_audit_event_id': 'task-start-p12-history',
                'latest_audit_event_id': 'task-history-complete',
              },
            },
          ],
          'next_cursor': 1,
          'has_more': false,
          'head_hash':
              'sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd',
        },
      }),
      historyResponse('対話履歴閲覧状態', {
        'grant': historyGrant,
        'page': null,
      }),
    ]);
    final client = await ShellCoreClient.product(transport: transport);
    expect(client.mode, 'broker');
    expect(client.initialUiConfiguration.locale, 'ja-JP');

    await tester.pumpWidget(GuiShellDesktopApp(client: client));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('open-agent-setup')));
    await tester.pumpAndSettle();

    expect(find.text('エージェントセンター'), findsWidgets);
    expect(find.text('Codex実行系と作業領域'), findsOneWidget);
    expect(find.text('登録を開始'), findsOneWidget);
    expect(transport.operations, contains('初回設定取得'));
    expect(transport.operations, contains('Setup Doctor報告取得'));
    expect(transport.operations, isNot(contains('AgentCLI実行系作業領域登録')));

    await tester.tap(find.text('登録を開始'));
    await tester.pumpAndSettle();
    expect(find.text('OpenAI（Codex CLI経由・現在の実装経路）'), findsOneWidget);

    final fields = find.byType(TextFormField);
    await tester.enterText(fields.at(0), 'model-p12-synthetic');
    await tester.enterText(fields.at(1), 'codex-p12-synthetic');
    await tester.enterText(fields.at(2), r'C:\Tools\Codex\codex.exe');
    await tester.enterText(fields.at(3), 'workspace-p12-synthetic');
    await tester.enterText(fields.at(4), r'C:\d4-p12-synthetic-workspace');
    await tester.tap(find.text('native Owner確認へ進む'));
    await tester.pumpAndSettle();

    final registrationRequest = transport.requests.singleWhere(
      (request) => request['operation'] == 'AgentCLI実行系作業領域登録',
    );
    final registrationResponse = transport.returnedResponses.singleWhere(
      (response) => response['operation'] == 'AgentCLI実行系作業領域登録',
    );
    expect(registrationResponse['status'], 'accepted');
    final registration = registrationRequest['payload']! as Map;
    expect(registration['provider_model_selection'], {
      'version': 1,
      'provider_id': 'openai_codex_cli',
      'model_id': 'model-p12-synthetic',
      'authentication_source': 'codex_cli_managed',
      'automatic_fallback': false,
    });
    expect(registration['runtime_id'], 'codex-p12-synthetic');
    expect(registration['workspace_id'], 'workspace-p12-synthetic');
    expect(registration['workspace_root'], r'C:\d4-p12-synthetic-workspace');
    expect(registration.containsKey('permission'), isFalse);
    expect(registration.containsKey('approval_id'), isFalse);
    expect(find.textContaining('Broker起動中だけ登録しました'), findsOneWidget);
    expect(find.textContaining('OpenAI（Codex CLI経由） / model-p12-synthetic'),
        findsOneWidget);
    expect(find.text('登録Workspaceで対話Sessionを開始'), findsOneWidget);

    await tester.tap(find.text('登録Workspaceで対話Sessionを開始'));
    await tester.pumpAndSettle();
    expect(find.text(sessionId), findsOneWidget);

    final preflightButton = find.text('Task実行能力を事前検査（実行なし）').last;
    await tester.ensureVisible(preflightButton);
    await tester.tap(preflightButton);
    await tester.pumpAndSettle();
    await tester.enterText(
      find.descendant(
        of: find.byType(AlertDialog),
        matching: find.byType(TextFormField),
      ),
      'rev5 P12 統合Task',
    );
    await tester.tap(find.text('Broker事前検査'));
    await tester.pumpAndSettle();

    await tester.ensureVisible(find.text('Workspace PermissionのOwner確認'));
    await tester.tap(find.text('Workspace PermissionのOwner確認'));
    await tester.pumpAndSettle();
    await tester.ensureVisible(find.text('Task一回ApprovalのOwner確認'));
    await tester.tap(find.text('Task一回ApprovalのOwner確認'));
    await tester.pumpAndSettle();
    await tester.ensureVisible(find.text('Taskを一回実行'));
    await tester.tap(find.text('Taskを一回実行'));
    await tester.pumpAndSettle();
    expect(find.text('Task状態: running'), findsOneWidget);

    await tester.ensureVisible(find.text('Task状態を更新'));
    await tester.tap(find.text('Task状態を更新'));
    await tester.pumpAndSettle();
    expect(find.text('Task状態: completed'), findsOneWidget);
    expect(find.textContaining(resultHash), findsOneWidget);

    final visibility = find.byKey(
      const ValueKey('agent-task-visibility-$sessionId'),
    );
    await tester.ensureVisible(visibility);
    await tester.tap(visibility);
    await tester.pumpAndSettle();
    await tester.tap(find.text('full').last);
    await tester.pumpAndSettle();
    final showResult = find.byKey(
      const ValueKey('agent-task-show-result-$sessionId'),
    );
    await tester.ensureVisible(showResult);
    await tester.tap(showResult);
    await tester.pumpAndSettle();
    expect(find.textContaining(resultSummary), findsOneWidget);
    final projectedResult = jsonDecode(
      tester
          .widget<SelectableText>(
            find.byKey(const ValueKey('agent-task-result-text')),
          )
          .data!,
    ) as Map<String, Object?>;
    expect(projectedResult['changed_files'], ['note.md']);
    expect(projectedResult['diff'], '+# Agent申告diff');
    expect(projectedResult['test_result'], 'Agent申告test結果（独立検証なし）');

    final previewHandoff = find.byKey(
      const ValueKey('preview-agent-handoff'),
    );
    await tester.ensureVisible(previewHandoff);
    expect(tester.widget<OutlinedButton>(previewHandoff).onPressed, isNotNull);
    await tester.tap(previewHandoff);
    await tester.pumpAndSettle();
    final handoffPreview = tester.widget<SelectableText>(
      find.byKey(const ValueKey('agent-handoff-preview')),
    );
    final handoff = jsonDecode(handoffPreview.data!) as Map<String, Object?>;
    expect(handoff['source_session_id'], sessionId);
    expect(handoff['target_session_id'], targetSessionId);
    expect(handoff['approved_result_hash'], resultHash);
    expect(handoff['authority_reassessment_required'], isTrue);
    expect(handoff['permission_reused'], isFalse);
    expect(handoff['approval_reused'], isFalse);
    expect(handoff['credential_included'], isFalse);
    expect(handoff['hidden_context_included'], isFalse);

    await tester.tap(find.byKey(const ValueKey('prepare-agent-handoff')));
    await tester.pumpAndSettle();
    expect(
      find.text('Handoff内容を新規Taskとして事前検査しました。PermissionとApprovalは移送されていません。'),
      findsOneWidget,
    );
    final handoffPreflight = transport.requests.singleWhere(
      (request) =>
          request['operation'] == 'Agent作業要求検査' &&
          ((request['payload']! as Map)['session_id'] == targetSessionId),
    );
    final handoffInstruction =
        (handoffPreflight['payload']! as Map)['instruction'] as String;
    expect(handoffInstruction, contains(resultHash));
    expect(handoffInstruction, contains('"permission_reused":false'));
    expect(handoffInstruction, contains('"approval_reused":false'));
    expect(handoffInstruction, contains('"credential_included":false'));
    expect(
      transport.requests.where((request) =>
          request['operation'] == 'AgentTaskWorkspacePermissionGrant' &&
          ((request['payload']! as Map)['session_id'] == targetSessionId)),
      isEmpty,
    );
    expect(
      transport.requests.where((request) =>
          request['operation'] == 'AgentTaskOwnerApprovalGrant' &&
          ((request['payload']! as Map)['session_id'] == targetSessionId)),
      isEmpty,
    );
    final navigation =
        tester.widget<NavigationRail>(find.byType(NavigationRail));
    navigation.onDestinationSelected!(13);
    await tester.pumpAndSettle();
    expect(find.text('実行履歴'), findsWidgets);
    await tester.tap(find.text('Agent Task履歴'));
    await tester.pumpAndSettle();
    expect(find.textContaining('Agent Taskの過去状態です。'), findsOneWidget);
    final historyEntry = find.byKey(const ValueKey('agent-task-$taskId'));
    expect(historyEntry, findsOneWidget);
    await tester.ensureVisible(historyEntry);
    await tester.tap(historyEntry);
    await tester.pumpAndSettle();
    expect(find.textContaining('結果hash: $resultHash'), findsOneWidget);
    expect(find.textContaining('Authorityは復元しません。'), findsOneWidget);

    tester
        .widget<NavigationRail>(find.byType(NavigationRail))
        .onDestinationSelected!(5);
    await tester.pumpAndSettle();
    expect(find.textContaining('別のWorkspace承認と基準点'), findsWidgets);
    final historyRequest = transport.requests.singleWhere(
      (request) => request['operation'] == 'AgentTask履歴閲覧',
    );
    expect(
        (historyRequest['payload']! as Map)['approval_id'], historyApprovalId);
    expect((historyRequest['payload']! as Map)['query'], {
      'after': 0,
      'limit': 50,
    });
    expect(
      transport.operations,
      containsAllInOrder([
        'AgentCLI実行系作業領域登録',
        '対話開始',
        'Agent作業要求検査',
        'AgentTaskWorkspacePermissionGrant',
        'AgentTaskOwnerApprovalGrant',
        'AgentTask実行',
        'AgentTask状態',
        'AgentTask結果表示承認',
        'AgentTask結果取得',
        'Agent作業要求検査',
        '対話履歴閲覧状態',
        '対話履歴閲覧',
        '対話履歴閲覧状態',
        '対話履歴閲覧状態',
        '対話履歴閲覧状態',
        'AgentTask履歴閲覧',
        '対話履歴閲覧状態',
      ]),
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('DashboardからCompare候補を選び独立Broker事前検査へ接続する',
      (WidgetTester tester) async {
    const sessionA = 'a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1';
    const sessionB = 'b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2';
    const instructionHash =
        'sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
    const instruction = 'P12 Compare合成Task';
    final transport = _FakeBrokerTransport([
      _brokerHealthResponse(),
      _brokerHostCapabilityResponse(),
      _brokerHostListResponse(),
      _brokerAdapterListResponse(),
      _brokerAgentAdapterListResponse(),
      _brokerDialogueSessionListResponse(sessions: [
        {
          '対話セッションID': sessionA,
          '実行系ID': 'codex-p12-agent-a',
          '状態': '利用中',
          '作成監査ID': 'audit-p12-session-a',
          '作業領域ID': 'workspace-p12-agent-a',
          '作業領域結合監査ID': 'audit-p12-workspace-a',
        },
        {
          '対話セッションID': sessionB,
          '実行系ID': 'codex-p12-agent-b',
          '状態': '利用中',
          '作成監査ID': 'audit-p12-session-b',
          '作業領域ID': 'workspace-p12-agent-b',
          '作業領域結合監査ID': 'audit-p12-workspace-b',
        },
      ]),
      _brokerAcceptedBody('normalize_payload', {'quarantined': false}),
      _brokerAcceptedBody('content_projection', {'redacted_payload': {}}),
      _brokerAcceptedBody('approval_edit', {'ok': false}),
      _brokerCommandSuspendedResponse(),
      _brokerAcceptedBody('プロファイル一覧', {}),
      _brokerAcceptedBody('通知一覧', {}),
      _brokerAcceptedBody('観測一覧', {}),
      _brokerAcceptedBody('観測一覧', {}),
      _brokerAcceptedBody('A2A接続一覧', {}),
      for (final session in [
        (sessionA, 'codex-p12-agent-a', 'workspace-p12-agent-a'),
        (sessionB, 'codex-p12-agent-b', 'workspace-p12-agent-b'),
      ])
        _brokerAcceptedBody('Agent作業要求検査', {
          '版': 1,
          '状態': '要求検査済み',
          '実行状態': '未実行',
          'Permission状態': '未付与',
          'Approval状態': '未取得',
          '実行系ID': session.$2,
          '対話セッションID': session.$1,
          '作業領域ID': session.$3,
          '指示hash': instructionHash,
        }),
    ]);
    final client = await ShellCoreClient.product(transport: transport);

    await tester.pumpWidget(GuiShellDesktopApp(client: client));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('open-agent-setup')));
    await tester.pumpAndSettle();

    final compareA = find.byKey(const ValueKey('compare-agent-a-session'));
    final compareB = find.byKey(const ValueKey('compare-agent-b-session'));
    expect(tester.widget<DropdownButton<String>>(compareA).value, sessionA);
    expect(tester.widget<DropdownButton<String>>(compareB).value, sessionB);
    expect(
      find.text(
          '独立した2 Sessionを選択済みです。同じTaskの事前検査後、各Permission／Approvalを個別に取得します。'),
      findsOneWidget,
    );

    final prepare = find.byKey(const ValueKey('prepare-agent-comparison'));
    await tester.ensureVisible(prepare);
    expect(tester.widget<OutlinedButton>(prepare).onPressed, isNotNull);
    await tester.tap(prepare);
    await tester.pumpAndSettle();
    final instructionField = find.descendant(
      of: find.byType(AlertDialog),
      matching: find.byType(TextFormField),
    );
    await tester.enterText(instructionField, instruction);
    await tester.tap(find.text('両Agentを事前検査'));
    await tester.pumpAndSettle();

    expect(
      find.text(
          '同じTask本文を別Sessionで検査しました。各AgentのWorkspace PermissionとTask Approvalを個別に行ってください。'),
      findsOneWidget,
    );
    expect(
      find.text('Agent A Permission／Approval: 未付与 / 未取得'),
      findsOneWidget,
    );
    expect(
      find.text('Agent B Permission／Approval: 未付与 / 未取得'),
      findsOneWidget,
    );
    final preflightRequests = transport.requests
        .where((request) => request['operation'] == 'Agent作業要求検査')
        .map((request) => request['payload']! as Map)
        .toList();
    expect(preflightRequests, hasLength(2));
    expect(
      preflightRequests.map((request) => request['session_id']).toSet(),
      {sessionA, sessionB},
    );
    expect(
      preflightRequests.map((request) => request['workspace_id']).toSet(),
      {'workspace-p12-agent-a', 'workspace-p12-agent-b'},
    );
    expect(
      preflightRequests.map((request) => request['instruction']).toSet(),
      {instruction},
    );
    expect(
      transport.operations,
      isNot(contains('AgentTaskWorkspacePermissionGrant')),
    );
    expect(
        transport.operations, isNot(contains('AgentTaskOwnerApprovalGrant')));
    expect(transport.operations, isNot(contains('AgentTask実行')));
    expect(tester.takeException(), isNull);
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

  testWidgets('MCP検索とコマンドからMCP欄へ移動し同じBroker transportで一覧を読む',
      (WidgetTester tester) async {
    final transport = _McpNavigationTransport();
    await tester.pumpWidget(
      GuiShellDesktopApp(client: ShellCoreClient.mock(transport: transport)),
    );

    await tester.sendKeyDownEvent(LogicalKeyboardKey.control);
    await tester.sendKeyDownEvent(LogicalKeyboardKey.shift);
    await tester.sendKeyEvent(LogicalKeyboardKey.keyF);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.shift);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.control);
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField), 'MCP');
    await tester.pumpAndSettle();
    await tester.tap(find.text('MCP接続'));
    await tester.pumpAndSettle();

    final listButton = find.byKey(const ValueKey('mcp-load-connections'));
    expect(find.text('MCP接続センター').hitTestable(), findsOneWidget);
    await tester.ensureVisible(listButton);
    await tester.tap(listButton);
    await tester.pumpAndSettle();
    expect(find.text('Brokerが保持するMCP接続はありません。'), findsOneWidget);

    await tester.sendKeyDownEvent(LogicalKeyboardKey.control);
    await tester.sendKeyEvent(LogicalKeyboardKey.keyK);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.control);
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byWidgetPredicate(
        (widget) =>
            widget is TextField && widget.decoration?.labelText == 'コマンドパレット',
      ),
      'MCP接続',
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text('MCP接続一覧とOwner確認付き切断を開く'));
    await tester.pumpAndSettle();

    expect(find.text('MCP接続センター').hitTestable(), findsOneWidget);
    expect(
      transport.operations.where((operation) => operation.startsWith('MCP')),
      ['MCP接続一覧'],
    );
    expect(
      transport.requests.singleWhere(
        (request) => (request['operation']! as String).startsWith('MCP'),
      ),
      {
        'operation': 'MCP接続一覧',
        'payload': const {'版': 1},
      },
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('DashboardからMCP Toolを明示確認後にBrokerへ要求しhash receiptを表示する',
      (WidgetTester tester) async {
    final transport = _McpToolP12IntegrationTransport();
    await tester.pumpWidget(
      GuiShellDesktopApp(client: ShellCoreClient.mock(transport: transport)),
    );
    await tester.pumpAndSettle();
    expect(
      transport.operations.where((operation) => operation.startsWith('MCP')),
      isEmpty,
    );

    await tester.sendKeyDownEvent(LogicalKeyboardKey.control);
    await tester.sendKeyDownEvent(LogicalKeyboardKey.shift);
    await tester.sendKeyEvent(LogicalKeyboardKey.keyF);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.shift);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.control);
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField), 'MCP');
    await tester.pumpAndSettle();
    await tester.tap(find.text('MCP接続'));
    await tester.pumpAndSettle();

    expect(find.text('MCP接続センター').hitTestable(), findsOneWidget);
    final listButton = find.byKey(const ValueKey('mcp-load-connections'));
    await tester.ensureVisible(listButton);
    await tester.tap(listButton);
    await tester.pumpAndSettle();
    expect(find.text('P12試験MCP'), findsOneWidget);

    await tester.ensureVisible(find.text('Tool一覧：1件'));
    await tester.tap(find.text('Tool一覧：1件'));
    await tester.pumpAndSettle();
    await tester.ensureVisible(find.text('p12-fixture-tool'));
    await tester.tap(find.text('確認して実行'));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField).last, '{"query":"P12"}');
    await tester.tap(find.text('入力内容を確認'));
    await tester.pumpAndSettle();

    expect(find.textContaining('"query": "P12"'), findsOneWidget);
    expect(
      transport.operations.where((operation) => operation.startsWith('MCP')),
      ['MCP接続一覧'],
    );
    await tester.tap(find.text('Windows確認へ進む'));
    await tester.pumpAndSettle();

    expect(
      transport.operations.where((operation) => operation.startsWith('MCP')),
      ['MCP接続一覧', 'MCP Tool実行'],
    );
    expect(
      transport.requests.last,
      {
        'operation': 'MCP Tool実行',
        'payload': {
          '版': 1,
          '操作': '実行',
          'ServerID': 'p12-mcp-fixture',
          'ToolID': transport.toolId,
          '名前': 'p12-fixture-tool',
          'arguments': {'query': 'P12'},
        },
      },
    );
    expect(find.textContaining('result hash: sha256:'), findsOneWidget);
    expect(find.textContaining('fixture-secret-result'), findsNothing);
    expect(tester.takeException(), isNull);
  });

  testWidgets('全体検索からCompose、コマンドからExportへ進み同一ManifestをBrokerへ渡す',
      (WidgetTester tester) async {
    final transport = _ComposeExportNavigationTransport();
    await tester.pumpWidget(
      GuiShellDesktopApp(client: ShellCoreClient.mock(transport: transport)),
    );

    await tester.sendKeyDownEvent(LogicalKeyboardKey.control);
    await tester.sendKeyDownEvent(LogicalKeyboardKey.shift);
    await tester.sendKeyEvent(LogicalKeyboardKey.keyF);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.shift);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.control);
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byWidgetPredicate(
        (widget) =>
            widget is TextField && widget.decoration?.labelText == '全体検索',
      ),
      'GUI Shell構成',
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text('GUI Shell構成').last);
    await tester.pumpAndSettle();

    expect(find.text('GUI Shell構成').hitTestable(), findsOneWidget);
    final runtimeIds = find.byKey(const ValueKey('compose-runtime-ids'));
    await tester.ensureVisible(runtimeIds);
    await tester.enterText(runtimeIds, 'fixture-runtime');
    final agentIds = find.byKey(const ValueKey('compose-agent-ids'));
    await tester.ensureVisible(agentIds);
    await tester.enterText(agentIds, 'fixture-agent');
    final composeButton = find.byKey(const ValueKey('compose-manifest-button'));
    await tester.ensureVisible(composeButton);
    await tester.tap(composeButton);
    await tester.pumpAndSettle();

    await tester.sendKeyDownEvent(LogicalKeyboardKey.control);
    await tester.sendKeyEvent(LogicalKeyboardKey.keyK);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.control);
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byWidgetPredicate(
        (widget) =>
            widget is TextField && widget.decoration?.labelText == 'コマンドパレット',
      ),
      '独立Appを書き出す',
    );
    await tester.pumpAndSettle();
    await tester.tap(
      find.byWidgetPredicate(
        (widget) => widget is Text && widget.data == '独立Appを書き出す',
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('GUI Shell Windows書出し').hitTestable(), findsOneWidget);
    final exportButton = find.byKey(const ValueKey('compose-export-button'));
    await tester.ensureVisible(exportButton);
    await tester.tap(exportButton);
    await tester.pumpAndSettle();

    final composeRequest = transport.requests.singleWhere(
      (request) => request['operation'] == 'GUI Shell構成',
    );
    final exportRequest = transport.requests.singleWhere(
      (request) => request['operation'] == 'GUI Shell書出し',
    );
    final composedManifest = composeRequest['payload']! as Map<String, Object?>;
    final exportPayload = exportRequest['payload']! as Map<String, Object?>;
    expect(composedManifest['runtime_ids'], ['fixture-runtime']);
    expect(composedManifest['agent_ids'], ['fixture-agent']);
    expect(exportPayload['compose_manifest'], composedManifest);
    expect(
      transport.operations.where(
        (operation) =>
            operation == 'GUI Shell構成' || operation == 'GUI Shell書出し',
      ),
      ['GUI Shell構成', 'GUI Shell書出し'],
    );
    expect(
        find.textContaining('fixture://d4-pocket/export.json'), findsWidgets);
    expect(tester.takeException(), isNull);
  });

  testWidgets('DashboardからUpdate Centerを開き既存Brokerで候補一覧を読む',
      (WidgetTester tester) async {
    final transport = _UpdateCenterNavigationTransport();
    await tester.pumpWidget(
      GuiShellDesktopApp(client: ShellCoreClient.mock(transport: transport)),
    );
    await tester.pumpAndSettle();
    expect(
      transport.operations.where((operation) => operation == '更新一覧'),
      isEmpty,
    );

    await tester.tap(find.byKey(const ValueKey('open-product-setup')));
    await tester.pumpAndSettle();

    expect(find.text('更新センター').hitTestable(), findsOneWidget);
    expect(find.text('署名検査済みの更新候補はありません。'), findsOneWidget);
    expect(
      transport.operations.where((operation) => operation == '更新一覧'),
      ['更新一覧'],
    );
    expect(
      transport.requests.singleWhere(
        (request) => request['operation'] == '更新一覧',
      ),
      {
        'operation': '更新一覧',
        'payload': const {'版': 1},
      },
    );
    expect(
      transport.operations,
      isNot(contains('更新候補取得')),
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('Desktop UX統合が既存画面を論理グループで絞り込む', (WidgetTester tester) async {
    await tester.pumpWidget(const GuiShellDesktopApp());

    final rail = tester.widget<NavigationRail>(find.byType(NavigationRail));
    expect(rail.destinations.length, 21);
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

  testWidgets('製品クライアントがブローカー経由の権限スナップショットを描画する', (tester) async {
    final transport = _FakeBrokerTransport([
      _brokerHealthResponse(),
      _brokerHostCapabilityResponse(),
      _brokerHostListResponse(
        additionalHosts: [_brokerRemoteHostRecord()],
      ),
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

    await tester.pumpWidget(MaterialApp(
      home: Scaffold(body: HostOperationCenter(client: client)),
    ));
    expect(find.textContaining('現在のBroker実行Host（ローカル）'), findsOneWidget);
    expect(find.textContaining('snapshotはBroker確定経路ではありません'), findsNothing);
    await tester.tap(find.text('登録Remote fixture').first);
    await tester.pumpAndSettle();
    expect(
      find.textContaining('別登録Host（remote接続・個別状態は未観測）'),
      findsOneWidget,
    );
    expect(
      find.textContaining('Runtime summary: 2件 / Agent summary: 1件'),
      findsOneWidget,
    );
    for (final runtime in snapshot.runtimes) {
      expect(find.text(runtime.runtimeId), findsNothing);
    }

    await tester.pumpWidget(MaterialApp(
      home: Scaffold(body: HostCapabilityCenter(client: client)),
    ));
    expect(
        find.textContaining('認証済みRust Brokerの現在の実行Host（ローカル）'), findsOneWidget);
    expect(find.textContaining('degradedを含む観測状態: degraded'), findsOneWidget);
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
    await tester.enterText(fields.at(0), '--dangerous');
    await tester.enterText(fields.at(1), 'codex-r2-synthetic');
    await tester.enterText(fields.at(2), r'C:\Tools\Codex\codex.exe');
    await tester.enterText(fields.at(3), 'workspace-r2-synthetic');
    await tester.enterText(fields.at(4), r'C:\d4-r2-synthetic-workspace');
    await tester.enterText(fields.at(5), '.env\nsecrets');
    await tester.tap(find.text('native Owner確認へ進む'));
    await tester.pumpAndSettle();
    expect(transport.operations, isNot(contains('AgentCLI実行系作業領域登録')));
    expect(find.textContaining('模型識別子はASCII英数字で始まる'), findsOneWidget);

    await tester.enterText(fields.at(0), 'model-test-01');
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
    expect(registration['provider_model_selection'], {
      'version': 1,
      'provider_id': 'openai_codex_cli',
      'model_id': 'model-test-01',
      'authentication_source': 'codex_cli_managed',
      'automatic_fallback': false,
    });
    expect(registration.containsKey('permission'), isFalse);
    expect(registration.containsKey('approval_id'), isFalse);
    expect(find.textContaining('OpenAI（Codex CLI経由） / model-test-01'),
        findsOneWidget);
    expect(find.textContaining('提供元接続・模型利用可否: 不明（CLI接続面のみ確認）'), findsOneWidget);
    expect(find.text('自動代替実行: 無効'), findsOneWidget);
    expect(find.text('Task実行: unsupported'), findsOneWidget);
    expect(find.text('登録Workspaceで対話Sessionを開始'), findsOneWidget);
  });

  testWidgets('Provider保管庫modeはmetadataのCredential IDだけ登録要求へ渡す',
      (WidgetTester tester) async {
    const credentialId = '11111111111111111111111111111111';
    const syntheticSecret = 'synthetic-provider-key-never-project';
    final credentialMetadata = {
      '版': 1,
      '資格情報ID': credentialId,
      '用途': 'provider_api_key',
      '接続対象': 'openai_codex_cli',
      '種類': 'api_key',
      '保管方式': 'windows_dpapi',
      '状態': '有効',
      '作成時刻UnixMillis': 1000,
      '最終使用時刻UnixMillis': null,
      '失効時刻UnixMillis': null,
      '暗号文hash': 'sha256:${List<String>.filled(64, 'a').join()}',
      '作成監査ID': 'audit-provider-credential-created',
      '公開範囲': 'metadata_only',
      '証拠種別': 'INTERNAL_STATE',
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
      {
        'request_id': 'test-資格情報一覧',
        'operation': '資格情報一覧',
        'status': 'accepted',
        'evidence_source': 'INTERNAL_STATE',
        'audit_event_id': 'audit-provider-credential-list',
        'error': null,
        'body': {
          '版': 1,
          '資格情報一覧': [credentialMetadata],
          '件数': 1,
          '公開範囲': 'metadata_only',
          '証拠種別': 'INTERNAL_STATE',
        },
        'shutdown_requested': false,
      },
      _brokerAcceptedBody('AgentCLI実行系作業領域登録', {
        'runtime_id': 'codex-provider-vault-fixture',
        'workspace_id': 'workspace-provider-vault-fixture',
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
    final vaultMode = find.text('D4 Pocket資格情報保管庫');
    await tester.ensureVisible(vaultMode);
    await tester.tap(vaultMode);
    await tester.pumpAndSettle();

    final credentialPicker = find.byType(DropdownButtonFormField<String>);
    await tester.ensureVisible(credentialPicker);
    await tester.tap(credentialPicker);
    await tester.pumpAndSettle();
    await tester.tap(find.text(credentialId).last);
    await tester.pumpAndSettle();

    final fields = find.byType(TextFormField);
    await tester.enterText(fields.at(0), 'model-test-01');
    await tester.enterText(fields.at(1), 'codex-provider-vault-fixture');
    await tester.enterText(fields.at(2), r'C:\Tools\Codex\codex.exe');
    await tester.enterText(fields.at(3), 'workspace-provider-vault-fixture');
    await tester.enterText(fields.at(4), r'C:\d4-provider-vault-workspace');
    await tester.tap(find.text('native Owner確認へ進む'));
    await tester.pumpAndSettle();

    final listRequest = transport.requests
        .singleWhere((request) => request['operation'] == '資格情報一覧');
    expect(listRequest['payload'], {'版': 1});
    final registrationRequest = transport.requests
        .singleWhere((request) => request['operation'] == 'AgentCLI実行系作業領域登録');
    final selection = (registrationRequest['payload']!
        as Map)['provider_model_selection'] as Map;
    expect(selection['authentication_source'], 'broker_credential_vault');
    expect(selection['credential_id'], credentialId);
    expect(jsonEncode(registrationRequest), isNot(contains(syntheticSecret)));
    expect(transport.requests.toString(), isNot(contains(syntheticSecret)));
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
      await tester.enterText(fields.at(0), 'model-test-01');
      await tester.enterText(fields.at(1), runtime);
      await tester.enterText(fields.at(2), r'C:\Tools\Codex\codex.exe');
      await tester.enterText(fields.at(3), workspace);
      await tester.enterText(fields.at(4), root);
      await tester.enterText(fields.at(5), '.env');
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
    expect(
      requests.every((request) =>
          (request['provider_model_selection'] as Map)['model_id'] ==
          'model-test-01'),
      isTrue,
    );
    expect(requests, hasLength(2));
    expect(
        requests.every((request) =>
            !request.containsKey('permission') &&
            !request.containsKey('approval_id')),
        isTrue);
  });

  testWidgets('Agent Centerは独立2 Taskを開始・比較し一方だけを選択する',
      (WidgetTester tester) async {
    const sessionA = 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
    const sessionB = 'cccccccccccccccccccccccccccccccc';
    const sessionC = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
    const taskA = 'dddddddddddddddddddddddddddddddd';
    const taskB = 'eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee';
    const taskC = 'ffffffffffffffffffffffffffffffff';
    const instructionHash =
        'sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
    const resultHashA =
        'sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
    const resultHashB =
        'sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc';
    Map<String, Object?> taskRecord({
      required String taskId,
      required String runtimeId,
      required String sessionId,
      required String workspaceId,
      required String status,
      required String auditId,
      String? resultHash,
    }) =>
        {
          'task_id': taskId,
          'record_version': 2,
          'agent_runtime_id': runtimeId,
          'session_id': sessionId,
          'workspace_id': workspaceId,
          'description': 'Agent作業Task（結果本文とWorkspace差分は別の権限経路）',
          'instruction_hash': instructionHash,
          'status': status,
          'audit_event_id': auditId,
          if (resultHash != null) 'result_hash': resultHash,
          'result_content_available': resultHash != null,
        };
    Map<String, Object?> permissionReceipt({
      required String runtimeId,
      required String sessionId,
      required String workspaceId,
      required String registrationHash,
    }) =>
        {
          'permission_id': sessionId,
          'agent_runtime_id': runtimeId,
          'session_id': sessionId,
          'workspace_id': workspaceId,
          'workspace_registration_hash': registrationHash,
          'operation': 'agent_task.execute',
          'scope': 'session_workspace_once',
          'decision': 'allow',
          'source': 'owner',
          'expires_at_epoch_seconds': 1900000000,
          'use_limit': 1,
          'uses_remaining': 1,
          'status': 'active',
        };
    Map<String, Object?> approvalReceipt({
      required String runtimeId,
      required String sessionId,
      required String workspaceId,
      required String executionHash,
    }) =>
        {
          '状態': 'Owner Approval発行済み',
          '実行状態': '未実行',
          '実行系ID': runtimeId,
          '対話セッションID': sessionId,
          '作業領域ID': workspaceId,
          '指示hash': instructionHash,
          '実行条件hash': executionHash,
          '適用ポリシー': 'gui-shell-agent-task-sandbox-v1-max-runtime-900s',
          'expires_at_epoch_seconds': 1900000000,
          'use_limit': 1,
          'uses_remaining': 1,
          'status': 'issued_unconsumed',
        };

    final transport = _FakeBrokerTransport([
      _brokerHealthResponse(),
      _brokerHostCapabilityResponse(),
      _brokerHostListResponse(),
      _brokerAdapterListResponse(),
      _brokerAgentAdapterListResponse(),
      _brokerDialogueSessionListResponse(sessions: [
        {
          '対話セッションID': sessionA,
          '実行系ID': 'codex-agent-a',
          '状態': '利用中',
          '作成監査ID': 'audit-session-a',
          '作業領域ID': 'workspace-agent-a',
          '作業領域結合監査ID': 'audit-workspace-a',
        },
        {
          '対話セッションID': sessionB,
          '実行系ID': 'codex-agent-b',
          '状態': '利用中',
          '作成監査ID': 'audit-session-b',
          '作業領域ID': 'workspace-agent-b',
          '作業領域結合監査ID': 'audit-workspace-b',
        },
        {
          '対話セッションID': sessionC,
          '実行系ID': 'codex-agent-c',
          '状態': '利用中',
          '作成監査ID': 'audit-session-c',
          '作業領域ID': 'workspace-agent-c',
          '作業領域結合監査ID': 'audit-workspace-c',
        },
      ]),
      _brokerAcceptedBody('normalize_payload', {'quarantined': false}),
      _brokerAcceptedBody('content_projection', {'redacted_payload': {}}),
      _brokerAcceptedBody('approval_edit', {'ok': false}),
      _brokerCommandSuspendedResponse(),
      for (final side in [
        ('codex-agent-a', sessionA, 'workspace-agent-a'),
        ('codex-agent-b', sessionB, 'workspace-agent-b'),
      ])
        _brokerAcceptedBody('Agent作業要求検査', {
          '版': 1,
          '状態': '要求検査済み',
          '実行状態': '未実行',
          'Permission状態': '未付与',
          'Approval状態': '未取得',
          '実行系ID': side.$1,
          '対話セッションID': side.$2,
          '作業領域ID': side.$3,
          '指示hash': instructionHash,
        }),
      _brokerAcceptedBody(
        'AgentTaskWorkspacePermissionGrant',
        permissionReceipt(
          runtimeId: 'codex-agent-a',
          sessionId: sessionA,
          workspaceId: 'workspace-agent-a',
          registrationHash:
              'sha256:1111111111111111111111111111111111111111111111111111111111111111',
        ),
      ),
      _brokerAcceptedBody(
        'AgentTaskWorkspacePermissionGrant',
        permissionReceipt(
          runtimeId: 'codex-agent-b',
          sessionId: sessionB,
          workspaceId: 'workspace-agent-b',
          registrationHash:
              'sha256:2222222222222222222222222222222222222222222222222222222222222222',
        ),
      ),
      _brokerAcceptedBody(
        'AgentTaskOwnerApprovalGrant',
        approvalReceipt(
          runtimeId: 'codex-agent-a',
          sessionId: sessionA,
          workspaceId: 'workspace-agent-a',
          executionHash: resultHashA,
        ),
      ),
      _brokerAcceptedBody(
        'AgentTaskOwnerApprovalGrant',
        approvalReceipt(
          runtimeId: 'codex-agent-b',
          sessionId: sessionB,
          workspaceId: 'workspace-agent-b',
          executionHash: resultHashB,
        ),
      ),
      _brokerAcceptedBody(
        'AgentTask実行',
        taskRecord(
          taskId: taskA,
          runtimeId: 'codex-agent-a',
          sessionId: sessionA,
          workspaceId: 'workspace-agent-a',
          status: 'running',
          auditId: 'audit-task-a-start',
        ),
      ),
      _brokerAcceptedBody(
        'AgentTask実行',
        taskRecord(
          taskId: taskB,
          runtimeId: 'codex-agent-b',
          sessionId: sessionB,
          workspaceId: 'workspace-agent-b',
          status: 'running',
          auditId: 'audit-task-b-start',
        ),
      ),
      _brokerAcceptedBody(
        'AgentTask状態',
        taskRecord(
          taskId: taskA,
          runtimeId: 'codex-agent-a',
          sessionId: sessionA,
          workspaceId: 'workspace-agent-a',
          status: 'completed',
          auditId: 'audit-task-a-complete',
          resultHash: resultHashA,
        ),
      ),
      _brokerAcceptedBody(
        'AgentTask状態',
        taskRecord(
          taskId: taskB,
          runtimeId: 'codex-agent-b',
          sessionId: sessionB,
          workspaceId: 'workspace-agent-b',
          status: 'completed',
          auditId: 'audit-task-b-complete',
          resultHash: resultHashB,
        ),
      ),
      _brokerAcceptedBody('AgentTask結果表示承認', {
        'task_id': taskA,
        'result_hash': resultHashA,
        'content_visibility': 'full',
        'approval_id': '11111111111111111111111111111111',
        'expires_at_epoch_seconds': 1900000000,
        'use_limit': 1,
        'uses_remaining': 1,
      }),
      _brokerAcceptedBody('AgentTask結果取得', {
        'task_id': taskA,
        'result_hash': resultHashA,
        'content_visibility': 'full',
        'projection': {
          'result_hash': resultHashA,
          'text': 'candidate patch body: add isolated implementation',
        },
      }),
      _brokerAcceptedBody('Agent作業要求検査', {
        '版': 1,
        '状態': '要求検査済み',
        '実行状態': '未実行',
        'Permission状態': '未付与',
        'Approval状態': '未取得',
        '実行系ID': 'codex-agent-c',
        '対話セッションID': sessionC,
        '作業領域ID': 'workspace-agent-c',
        '指示hash': instructionHash,
      }),
      _brokerAcceptedBody(
        'AgentTaskWorkspacePermissionGrant',
        permissionReceipt(
          runtimeId: 'codex-agent-c',
          sessionId: sessionC,
          workspaceId: 'workspace-agent-c',
          registrationHash:
              'sha256:3333333333333333333333333333333333333333333333333333333333333333',
        ),
      ),
      _brokerAcceptedBody(
        'AgentTaskOwnerApprovalGrant',
        approvalReceipt(
          runtimeId: 'codex-agent-c',
          sessionId: sessionC,
          workspaceId: 'workspace-agent-c',
          executionHash: resultHashA,
        ),
      ),
      _brokerAcceptedBody(
        'AgentTask実行',
        taskRecord(
          taskId: taskC,
          runtimeId: 'codex-agent-c',
          sessionId: sessionC,
          workspaceId: 'workspace-agent-c',
          status: 'running',
          auditId: 'audit-task-c-start',
        ),
      ),
    ]);
    final client = await ShellCoreClient.product(transport: transport);

    await tester.pumpWidget(MaterialApp(
      home: Scaffold(body: AgentCenter(client: client)),
    ));
    final prepareComparison =
        find.byKey(const ValueKey('prepare-agent-comparison'));
    await tester.ensureVisible(prepareComparison);
    await tester.tap(prepareComparison);
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byType(TextFormField),
      '独立Compare用の安全なTask',
    );
    await tester.tap(find.text('両Agentを事前検査'));
    await tester.pumpAndSettle();

    expect(find.textContaining(instructionHash), findsOneWidget);
    expect(
      find.text('Agent A Permission／Approval: 未付与 / 未取得'),
      findsOneWidget,
    );
    expect(
      find.text('Agent B Permission／Approval: 未付与 / 未取得'),
      findsOneWidget,
    );
    final permissionButtons = find.text('Workspace PermissionのOwner確認');
    await tester.ensureVisible(permissionButtons.first);
    await tester.tap(permissionButtons.first);
    await tester.pumpAndSettle();
    await tester.ensureVisible(permissionButtons.last);
    await tester.tap(permissionButtons.last);
    await tester.pumpAndSettle();

    final approvalButtons = find.text('Task一回ApprovalのOwner確認');
    await tester.ensureVisible(approvalButtons.first);
    await tester.tap(approvalButtons.first);
    await tester.pumpAndSettle();
    await tester.ensureVisible(approvalButtons.last);
    await tester.tap(approvalButtons.last);
    await tester.pumpAndSettle();

    await tester.ensureVisible(
      find.byKey(const ValueKey('start-agent-comparison')),
    );
    await tester.tap(find.byKey(const ValueKey('start-agent-comparison')));
    await tester.pumpAndSettle();
    expect(find.text('Task状態: running'), findsNWidgets(4));
    expect(
        transport.operations.where((op) => op == 'AgentTask実行'), hasLength(2));

    await tester.ensureVisible(
      find.byKey(const ValueKey('refresh-agent-comparison')),
    );
    await tester.tap(find.byKey(const ValueKey('refresh-agent-comparison')));
    await tester.pumpAndSettle();
    expect(find.text('Task状態: completed'), findsNWidgets(4));
    expect(find.textContaining('audit-task-a-complete'), findsNWidgets(3));
    expect(find.textContaining('audit-task-b-complete'), findsNWidgets(3));
    expect(find.text('Resource: runtime実測値は未提供のためunknown'), findsOneWidget);
    expect(find.text('Test実行record／Resource実測値: unknown'), findsNWidgets(2));

    final selectedA = find.byKey(
      const ValueKey('select-compare-result-$sessionA'),
    );
    final selectedB = find.byKey(
      const ValueKey('select-compare-result-$sessionB'),
    );
    await tester.ensureVisible(selectedA);
    await tester.tap(selectedA);
    await tester.pumpAndSettle();
    expect(tester.widget<ChoiceChip>(selectedA).selected, isTrue);
    expect(tester.widget<ChoiceChip>(selectedB).selected, isFalse);
    expect(find.textContaining('Authorityを与えません'), findsOneWidget);

    final visibilityA =
        find.byKey(const ValueKey('agent-task-visibility-$sessionA'));
    await tester.ensureVisible(visibilityA);
    await tester.tap(visibilityA);
    await tester.pumpAndSettle();
    await tester.tap(find.text('full').last);
    await tester.pumpAndSettle();
    final showResultA =
        find.byKey(const ValueKey('agent-task-show-result-$sessionA'));
    await tester.ensureVisible(showResultA);
    await tester.tap(showResultA);
    await tester.pumpAndSettle();
    expect(find.textContaining('candidate patch body'), findsOneWidget);

    final prepareApply =
        find.byKey(const ValueKey('prepare-selected-result-apply'));
    await tester.ensureVisible(prepareApply);
    expect(tester.widget<OutlinedButton>(prepareApply).onPressed, isNotNull);
    await tester.tap(prepareApply);
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byType(TextFormField),
      'このcandidate patchだけを適用する',
    );
    await tester.tap(find.text('適用先Taskを事前検査'));
    await tester.pumpAndSettle();

    final applyPermission =
        find.byKey(const ValueKey('agent-task-permission-$sessionC'));
    await tester.ensureVisible(applyPermission);
    await tester.tap(applyPermission);
    await tester.pumpAndSettle();
    final applyApproval =
        find.byKey(const ValueKey('agent-task-approval-$sessionC'));
    await tester.ensureVisible(applyApproval);
    await tester.tap(applyApproval);
    await tester.pumpAndSettle();
    final applyStart = find.byKey(const ValueKey('agent-task-start-$sessionC'));
    await tester.ensureVisible(applyStart);
    await tester.tap(applyStart);
    await tester.pumpAndSettle();
    expect(find.text('Task状態: running'), findsOneWidget);

    final applyTaskRequest = transport.requests.lastWhere(
      (request) =>
          request['operation'] == 'AgentTask実行' &&
          ((request['payload']! as Map)['session_id'] == sessionC),
    );
    final applyInstruction =
        ((applyTaskRequest['payload']! as Map)['instruction'] as String);
    expect(applyInstruction, contains('candidate patch body'));
    expect(applyInstruction, contains('このcandidate patchだけを適用する'));
    expect(applyInstruction, contains('未信頼データ'));
    expect(applyTaskRequest['payload'], isNot(contains('approval_id')));

    final starts = transport.requests
        .where((request) => request['operation'] == 'AgentTask実行')
        .map((request) => request['payload']! as Map)
        .toList(growable: false);
    expect(starts, hasLength(3));
    expect(starts.take(2).map((request) => request['instruction']).toSet(),
        {'独立Compare用の安全なTask'});
    expect(starts.take(2).map((request) => request['session_id']).toSet(),
        {sessionA, sessionB});
    expect(starts.take(2).map((request) => request['workspace_id']).toSet(),
        {'workspace-agent-a', 'workspace-agent-b'});
  });

  testWidgets('Agent Centerはfull承認結果を別Sessionへ引き継ぎ、新規grantを要求する',
      (WidgetTester tester) async {
    const sessionA = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
    const sessionB = 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
    const sourceTask = 'cccccccccccccccccccccccccccccccc';
    const targetTask = 'dddddddddddddddddddddddddddddddd';
    const instructionHash =
        'sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
    const resultHash =
        'sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
    const resultText =
        r'{"result_summary":"公開文書を更新した","artifacts":[{"name":"patch.txt","content":"patch body"}],"changed_files":["README.md"],"diff":"+## 更新","test_result":"Agent申告: test passed（独立検証なし）"}';

    Map<String, Object?> taskRecord({
      required String taskId,
      required String runtimeId,
      required String sessionId,
      required String workspaceId,
      required String status,
      required String auditId,
      String? resultHashValue,
    }) =>
        {
          'task_id': taskId,
          'record_version': 2,
          'agent_runtime_id': runtimeId,
          'session_id': sessionId,
          'workspace_id': workspaceId,
          'description': 'Agent作業Task（結果本文とWorkspace差分は別の権限経路）',
          'instruction_hash': instructionHash,
          'status': status,
          'audit_event_id': auditId,
          if (resultHashValue != null) 'result_hash': resultHashValue,
          'result_content_available': resultHashValue != null,
        };
    Map<String, Object?> permissionReceipt({
      required String sessionId,
      required String workspaceId,
      required String registrationHash,
    }) =>
        {
          'permission_id': sessionId,
          'agent_runtime_id': sessionId == sessionA ? 'codex-a' : 'codex-b',
          'session_id': sessionId,
          'workspace_id': workspaceId,
          'workspace_registration_hash': registrationHash,
          'operation': 'agent_task.execute',
          'scope': 'session_workspace_once',
          'decision': 'allow',
          'source': 'owner',
          'expires_at_epoch_seconds': 1900000000,
          'use_limit': 1,
          'uses_remaining': 1,
          'status': 'active',
        };
    Map<String, Object?> approvalReceipt({
      required String sessionId,
    }) =>
        {
          '状態': 'Owner Approval発行済み',
          '実行状態': '未実行',
          '実行系ID': sessionId == sessionA ? 'codex-a' : 'codex-b',
          '対話セッションID': sessionId,
          '作業領域ID': sessionId == sessionA ? 'workspace-a' : 'workspace-b',
          '指示hash': instructionHash,
          '実行条件hash': resultHash,
          '適用ポリシー': 'gui-shell-agent-task-sandbox-v1-max-runtime-900s',
          'expires_at_epoch_seconds': 1900000000,
          'use_limit': 1,
          'uses_remaining': 1,
          'status': 'issued_unconsumed',
        };

    final transport = _FakeBrokerTransport([
      _brokerHealthResponse(),
      _brokerHostCapabilityResponse(),
      _brokerHostListResponse(),
      _brokerAdapterListResponse(),
      _brokerAgentAdapterListResponse(),
      _brokerDialogueSessionListResponse(sessions: [
        {
          '対話セッションID': sessionA,
          '実行系ID': 'codex-a',
          '状態': '利用中',
          '作成監査ID': 'audit-session-a',
          '作業領域ID': 'workspace-a',
          '作業領域結合監査ID': 'audit-workspace-a',
        },
        {
          '対話セッションID': sessionB,
          '実行系ID': 'codex-b',
          '状態': '利用中',
          '作成監査ID': 'audit-session-b',
          '作業領域ID': 'workspace-b',
          '作業領域結合監査ID': 'audit-workspace-b',
        },
      ]),
      _brokerAcceptedBody('normalize_payload', {'quarantined': false}),
      _brokerAcceptedBody('content_projection', {'redacted_payload': {}}),
      _brokerAcceptedBody('approval_edit', {'ok': false}),
      _brokerCommandSuspendedResponse(),
      _brokerAcceptedBody('Agent作業要求検査', {
        '版': 1,
        '状態': '要求検査済み',
        '実行状態': '未実行',
        'Permission状態': '未付与',
        'Approval状態': '未取得',
        '実行系ID': 'codex-a',
        '対話セッションID': sessionA,
        '作業領域ID': 'workspace-a',
        '指示hash': instructionHash,
      }),
      _brokerAcceptedBody(
        'AgentTaskWorkspacePermissionGrant',
        permissionReceipt(
          sessionId: sessionA,
          workspaceId: 'workspace-a',
          registrationHash:
              'sha256:1111111111111111111111111111111111111111111111111111111111111111',
        ),
      ),
      _brokerAcceptedBody(
        'AgentTaskOwnerApprovalGrant',
        approvalReceipt(sessionId: sessionA),
      ),
      _brokerAcceptedBody(
        'AgentTask実行',
        taskRecord(
          taskId: sourceTask,
          runtimeId: 'codex-a',
          sessionId: sessionA,
          workspaceId: 'workspace-a',
          status: 'completed',
          auditId: 'audit-source-task-start',
          resultHashValue: resultHash,
        ),
      ),
      _brokerAcceptedBody('AgentTask結果表示承認', {
        'task_id': sourceTask,
        'result_hash': resultHash,
        'content_visibility': 'full',
        'approval_id': '22222222222222222222222222222222',
        'expires_at_epoch_seconds': 1900000000,
        'use_limit': 1,
        'uses_remaining': 1,
      }),
      _brokerAcceptedBody('AgentTask結果取得', {
        'task_id': sourceTask,
        'result_hash': resultHash,
        'content_visibility': 'full',
        'projection': {'result_hash': resultHash, 'text': resultText},
      }),
      _brokerAcceptedBody('Agent作業要求検査', {
        '版': 1,
        '状態': '要求検査済み',
        '実行状態': '未実行',
        'Permission状態': '未付与',
        'Approval状態': '未取得',
        '実行系ID': 'codex-b',
        '対話セッションID': sessionB,
        '作業領域ID': 'workspace-b',
        '指示hash': instructionHash,
      }),
      _brokerAcceptedBody(
        'AgentTaskWorkspacePermissionGrant',
        permissionReceipt(
          sessionId: sessionB,
          workspaceId: 'workspace-b',
          registrationHash:
              'sha256:3333333333333333333333333333333333333333333333333333333333333333',
        ),
      ),
      _brokerAcceptedBody(
        'AgentTaskOwnerApprovalGrant',
        approvalReceipt(sessionId: sessionB),
      ),
      _brokerAcceptedBody(
        'AgentTask実行',
        taskRecord(
          taskId: targetTask,
          runtimeId: 'codex-b',
          sessionId: sessionB,
          workspaceId: 'workspace-b',
          status: 'running',
          auditId: 'audit-target-task-start',
        ),
      ),
    ]);
    final client = await ShellCoreClient.product(transport: transport);
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(body: AgentCenter(client: client)),
    ));

    final sourcePreflight = find.text('Task実行能力を事前検査（実行なし）').first;
    await tester.ensureVisible(sourcePreflight);
    await tester.tap(sourcePreflight);
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextFormField), '公開文書を更新する');
    await tester.tap(find.text('Broker事前検査'));
    await tester.pumpAndSettle();
    final sourcePermission =
        find.byKey(const ValueKey('agent-task-permission-$sessionA'));
    await tester.ensureVisible(sourcePermission);
    await tester.tap(sourcePermission);
    await tester.pumpAndSettle();
    final sourceApproval =
        find.byKey(const ValueKey('agent-task-approval-$sessionA'));
    await tester.ensureVisible(sourceApproval);
    await tester.tap(sourceApproval);
    await tester.pumpAndSettle();
    final sourceStart =
        find.byKey(const ValueKey('agent-task-start-$sessionA'));
    await tester.ensureVisible(sourceStart);
    await tester.tap(sourceStart);
    await tester.pumpAndSettle();

    final visibility =
        find.byKey(const ValueKey('agent-task-visibility-$sessionA'));
    await tester.ensureVisible(visibility);
    await tester.tap(visibility);
    await tester.pumpAndSettle();
    await tester.tap(find.text('full').last);
    await tester.pumpAndSettle();
    final showResult =
        find.byKey(const ValueKey('agent-task-show-result-$sessionA'));
    await tester.ensureVisible(showResult);
    await tester.tap(showResult);
    await tester.pumpAndSettle();

    final preview = find.byKey(const ValueKey('preview-agent-handoff'));
    await tester.ensureVisible(preview);
    await tester.tap(preview);
    await tester.pumpAndSettle();
    final bundlePreview = find.byKey(const ValueKey('agent-handoff-preview'));
    expect(bundlePreview, findsOneWidget);
    expect(tester.widget<SelectableText>(bundlePreview).data,
        contains('patch.txt'));
    expect(tester.widget<SelectableText>(bundlePreview).data,
        contains('test_result'));

    final prepare = find.byKey(const ValueKey('prepare-agent-handoff'));
    await tester.ensureVisible(prepare);
    await tester.tap(prepare);
    await tester.pumpAndSettle();
    final targetPermission =
        find.byKey(const ValueKey('agent-task-permission-$sessionB'));
    await tester.ensureVisible(targetPermission);
    await tester.tap(targetPermission);
    await tester.pumpAndSettle();
    final targetApproval =
        find.byKey(const ValueKey('agent-task-approval-$sessionB'));
    await tester.ensureVisible(targetApproval);
    await tester.tap(targetApproval);
    await tester.pumpAndSettle();
    final targetStart =
        find.byKey(const ValueKey('agent-task-start-$sessionB'));
    await tester.ensureVisible(targetStart);
    await tester.tap(targetStart);
    await tester.pumpAndSettle();

    final targetStartRequest = transport.requests.singleWhere((request) =>
        request['operation'] == 'AgentTask実行' &&
        ((request['payload']! as Map)['session_id'] == sessionB));
    final targetPayload = targetStartRequest['payload']! as Map;
    final targetInstruction = targetPayload['instruction'] as String;
    final transferredJson = targetInstruction.substring(
      targetInstruction.indexOf('\n') + 1,
    );
    final transferred =
        Map<String, Object?>.from(jsonDecode(transferredJson) as Map);
    expect(transferred['task_context'], '公開文書を更新する');
    expect(transferred['approved_result_hash'], resultHash);
    expect(transferred['changed_files'], ['README.md']);
    expect(transferred['diff'], '+## 更新');
    expect(transferred['test_result'], contains('独立検証なし'));
    expect(transferred['authority_reassessment_required'], isTrue);
    expect(transferred['permission_reused'], isFalse);
    expect(transferred['approval_reused'], isFalse);
    expect(targetPayload, isNot(contains('approval_id')));
    expect(targetPayload, isNot(contains('permission_id')));
    expect(find.text('Agent引き継ぎ記録'), findsOneWidget);
    expect(find.textContaining('audit-target-task-start'), findsNWidgets(3));

    final handoffRequests = transport.requests.where((request) =>
        request['operation'] == 'Agent作業要求検査' &&
        ((request['payload']! as Map)['session_id'] == sessionB));
    expect(handoffRequests, hasLength(1));
    final grantOperations = transport.requests
        .where((request) =>
            (request['operation'] == 'AgentTaskWorkspacePermissionGrant' ||
                request['operation'] == 'AgentTaskOwnerApprovalGrant') &&
            ((request['payload']! as Map)['session_id'] == sessionB))
        .map((request) => request['operation'])
        .toSet();
    expect(grantOperations,
        {'AgentTaskWorkspacePermissionGrant', 'AgentTaskOwnerApprovalGrant'});
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
    await tester.enterText(registrationFields.at(0), 'model-test-01');
    await tester.enterText(registrationFields.at(1), 'codex-r2-synthetic');
    await tester.enterText(
        registrationFields.at(2), r'C:\Tools\Codex\codex.exe');
    await tester.enterText(registrationFields.at(3), 'workspace-r2-synthetic');
    await tester.enterText(
        registrationFields.at(4), r'C:\d4-r2-synthetic-workspace');
    await tester.enterText(registrationFields.at(5), '.env');
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
    await tester.enterText(registrationFields.at(0), 'model-test-01');
    await tester.enterText(registrationFields.at(1), 'codex-r2-synthetic');
    await tester.enterText(
        registrationFields.at(2), r'C:\Tools\Codex\codex.exe');
    await tester.enterText(registrationFields.at(3), 'workspace-r2-synthetic');
    await tester.enterText(
        registrationFields.at(4), r'C:\d4-r2-synthetic-workspace');
    await tester.enterText(registrationFields.at(5), '.env');
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

  test('Adapter Manifest導入はnative Owner Broker操作へ本文を限定送信する', () async {
    final transport = _FakeBrokerTransport([
      ..._shellCoreProductBootstrapResponses(),
      _brokerAdapterManifestMutationResponse('導入', 'ui_fixture_adapter'),
    ]);
    final client = await ShellCoreClient.product(transport: transport);
    final result = await client.manageAdapterManifest(
      '導入',
      _adapterManifestFixture(),
    );

    expect(result.status, 'accepted');
    expect(result.adapterId, 'ui_fixture_adapter');
    final request = transport.requests.last;
    expect(request['operation'], 'アダプター導入');
    expect(request['payload'], {
      '版': 1,
      '操作': '導入',
      'Manifest': _adapterManifestFixture(),
    });
    expect(request['payload'], isNot(contains('permission')));
  });

  test('Adapter Manifest更新は対象IDと現在hashを再結合しstale要求を送らない', () async {
    final transport = _FakeBrokerTransport([
      ..._shellCoreProductBootstrapResponses(),
      _brokerAdapterManifestMutationResponse(
        '更新',
        'mock_local_llm_adapter',
        managementState: 'disabled',
      ),
    ]);
    final client = await ShellCoreClient.product(transport: transport);
    const currentHash =
        'sha256:1111111111111111111111111111111111111111111111111111111111111111';
    final manifest = _adapterManifestFixture(
      adapterId: 'mock_local_llm_adapter',
      version: '1.0.1',
    );
    final result = await client.manageAdapterManifest(
      '更新',
      manifest,
      currentAdapterId: 'mock_local_llm_adapter',
      currentAdapterHash: currentHash,
    );

    expect(result.status, 'accepted');
    expect(transport.requests.last, {
      'operation': 'アダプター更新',
      'payload': {
        '版': 1,
        '操作': '更新',
        'Adapter hash': currentHash,
        'Manifest': manifest,
      },
    });
    final beforeInvalid = transport.requests.length;
    await expectLater(
      client.manageAdapterManifest(
        '更新',
        _adapterManifestFixture(adapterId: 'different_adapter'),
        currentAdapterId: 'mock_local_llm_adapter',
        currentAdapterHash: currentHash,
      ),
      throwsA(isA<BrokerClientException>()),
    );
    expect(transport.requests, hasLength(beforeInvalid));
  });

  testWidgets('Runtime CenterからManifest JSONをnative Owner経路へ送りcatalogを更新する',
      (WidgetTester tester) async {
    final listAfterInstall = _brokerAdapterListInternalResponse();
    final transport = _FakeBrokerTransport([
      ..._shellCoreProductBootstrapResponses(),
      _brokerAdapterManifestMutationResponse('導入', 'ui_fixture_adapter'),
      listAfterInstall,
    ]);
    final client = await ShellCoreClient.product(transport: transport);
    await tester.pumpWidget(
      MaterialApp(home: Scaffold(body: RuntimeCenter(client: client))),
    );

    final installButton =
        find.byKey(const ValueKey('adapter-manifest-install'));
    await tester.ensureVisible(installButton);
    await tester.pumpAndSettle();
    await tester.tap(installButton);
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const ValueKey('adapter-manifest-json-input')),
      jsonEncode(_adapterManifestFixture()),
    );
    await tester.tap(find.byKey(const ValueKey('adapter-manifest-submit')));
    await tester.pumpAndSettle();

    expect(transport.operations, contains('アダプター導入'));
    expect(find.textContaining('ui_fixture_adapter: accepted'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('起動後のAdapter一覧に表示されたrecordを現在hashへ束縛して操作する',
      (WidgetTester tester) async {
    final list = _brokerAdapterListInternalResponse();
    final body = list['body']! as Map<String, Object?>;
    final record = (body['Adapter一覧']! as List).single as Map<String, Object?>;
    record['Adapter ID'] = 'ui_fixture_adapter';
    final transport = _FakeBrokerTransport([
      ..._shellCoreProductBootstrapResponses(),
      _brokerAdapterManifestMutationResponse('導入', 'ui_fixture_adapter'),
      list,
      _brokerAdapterSuspendedResponse(),
    ]);
    final client = await ShellCoreClient.product(transport: transport);
    expect(
        client.snapshot.adapterCatalog
            .any((a) => a.adapterId == 'ui_fixture_adapter'),
        isFalse);
    await tester.pumpWidget(
        MaterialApp(home: Scaffold(body: RuntimeCenter(client: client))));
    final install = find.byKey(const ValueKey('adapter-manifest-install'));
    await tester.ensureVisible(install);
    await tester.tap(install);
    await tester.pumpAndSettle();
    await tester.enterText(
        find.byKey(const ValueKey('adapter-manifest-json-input')),
        jsonEncode(_adapterManifestFixture()));
    await tester.tap(find.byKey(const ValueKey('adapter-manifest-submit')));
    await tester.pumpAndSettle();
    final verify = find.byKey(const ValueKey('adapter-ui_fixture_adapter-検証'));
    await tester.ensureVisible(verify);
    await tester.pumpAndSettle();
    await tester.tap(verify);
    await tester.pumpAndSettle();
    expect(transport.requests.last['operation'], 'アダプター検証');
    expect(transport.requests.last['payload'], {
      '版': 1,
      '操作': '検証',
      'Adapter ID': 'ui_fixture_adapter',
      'Adapter hash': record['hash'],
    });
    expect(find.textContaining('owner_reapproval_required'), findsOneWidget);
    expect(tester.takeException(), isNull);
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

class _McpNavigationTransport implements BrokerTransport {
  final operations = <String>[];
  final requests = <Map<String, Object?>>[];

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    operations.add(operation);
    requests.add({'operation': operation, 'payload': payload});
    if (operation != 'MCP接続一覧') {
      throw BrokerClientException('fixtureでは未対応の要求です: $operation');
    }
    return {
      'request_id': 'fixture-mcp-list',
      'operation': 'MCP接続一覧',
      'status': 'accepted',
      'evidence_source': 'INTERNAL_STATE',
      'audit_event_id': 'audit-mcp-list',
      'error': null,
      'body': {
        '版': 1,
        'MCP接続一覧': <Object?>[],
        '件数': 0,
        '公開範囲': 'metadata_only',
        '証拠種別': 'INTERNAL_STATE',
      },
      'shutdown_requested': false,
    };
  }
}

class _McpToolP12IntegrationTransport implements BrokerTransport {
  final operations = <String>[];
  final requests = <Map<String, Object?>>[];
  final toolId = 'tool-${List<String>.filled(142, 'a').join()}';

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    operations.add(operation);
    requests.add({'operation': operation, 'payload': payload});
    if (operation == 'MCP接続一覧') {
      return {
        'operation': operation,
        'status': 'accepted',
        'evidence_source': 'INTERNAL_STATE',
        'body': {
          '版': 1,
          'MCP接続一覧': [
            {
              '版': 1,
              '契約種別': 'MCP外部概念射影',
              'Server': {
                'server_id': 'p12-mcp-fixture',
                '表示名': 'P12試験MCP',
              },
              'Transport': {'kind': 'stdio'},
              'Tool': [
                {
                  'tool_id': toolId,
                  'name': 'p12-fixture-tool',
                  'description_summary': '',
                  'input_schema_hash':
                      'sha256:${List<String>.filled(64, 'b').join()}',
                  'risk': 'unknown',
                  'status': {
                    'status': 'supported',
                    'reason': 'fixture内の形状確認だけを表す',
                  },
                },
              ],
              'Resource': <Object?>[],
              'Prompt': <Object?>[],
              'Credential ref': <String, Object?>{},
              'Trust': <String, Object?>{},
              'Capability diff': <String, Object?>{},
              '権限生成': 'なし',
              '公開範囲': 'metadata_only',
              '証拠種別': 'INTERNAL_STATE',
              '接続状態': 'connected',
              '能力ID': 'mcp.connection.connect',
              '権限ID': 'permission.mcp.connection.connect',
              '承認状態': 'owner_control_approved',
              '復旧ID': 'recover-mcp-connection',
              '接続監査ID': 'fixture-mcp-list-audit',
              '実行状態': 'ready',
            },
          ],
          '件数': 1,
          '公開範囲': 'metadata_only',
          '証拠種別': 'INTERNAL_STATE',
        },
      };
    }
    if (operation == 'MCP Tool実行') {
      return {
        'operation': operation,
        'status': 'accepted',
        // 実Broker receiptのwire形状を模すだけのfixtureであり、LIVE_RUNTIME証拠ではない。
        'evidence_source': 'LIVE_RUNTIME',
        'body': {
          '版': 1,
          '契約種別': 'MCP Tool実行receipt',
          'ServerID': payload!['ServerID'],
          'ToolID': payload['ToolID'],
          '名前': payload['名前'],
          'arguments_hash': 'sha256:${List<String>.filled(64, 'a').join()}',
          'result_hash': 'sha256:${List<String>.filled(64, 'c').join()}',
          'Tool error': false,
          'content_count': 1,
          'content_types': ['text'],
          '接続状態': 'connected',
          '能力ID': 'mcp.tool.call',
          '権限ID': 'permission.mcp.tool.call.one_shot',
          '承認状態': 'native_owner_confirmed',
          '承認監査ID': 'fixture-mcp-tool-approval',
          '復旧ID': 'inspect-mcp-tool-side-effect',
          '権限生成': 'Broker内一回限りPermissionを消費',
          '公開範囲': 'hash_only',
          '証拠種別': 'LIVE_RUNTIME',
          '監査ID': 'fixture-mcp-tool-audit',
        },
      };
    }
    throw BrokerClientException('P12 fixtureでは未対応の要求です: $operation');
  }
}

class _ComposeExportNavigationTransport implements BrokerTransport {
  final operations = <String>[];
  final requests = <Map<String, Object?>>[];

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    operations.add(operation);
    requests.add({'operation': operation, 'payload': payload});
    if (operation == 'GUI Shell構成') {
      return {
        'request_id': 'fixture-compose',
        'operation': operation,
        'status': 'accepted',
        'evidence_source': 'FIXTURE',
        'audit_event_id': 'fixture-compose-audit',
        'error': null,
        'body': {'compose_manifest': payload},
        'shutdown_requested': false,
      };
    }
    if (operation == 'GUI Shell書出し') {
      return {
        'request_id': 'fixture-export',
        'operation': operation,
        'status': 'accepted',
        'evidence_source': 'FIXTURE',
        'audit_event_id': 'fixture-export-audit',
        'error': null,
        'body': {
          'manifest_file': {
            'path': 'fixture://d4-pocket/export.json',
            'temporary_file_status': 'cleaned',
          },
        },
        'shutdown_requested': false,
      };
    }
    throw BrokerClientException('fixtureでは未対応の要求です: $operation');
  }
}

class _UpdateCenterNavigationTransport implements BrokerTransport {
  final operations = <String>[];
  final requests = <Map<String, Object?>>[];

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    operations.add(operation);
    requests.add({'operation': operation, 'payload': payload});
    if (operation != '更新一覧') {
      throw BrokerClientException('fixtureでは未対応の要求です: $operation');
    }
    return {
      'request_id': 'fixture-update-list',
      'operation': operation,
      'status': 'accepted',
      'evidence_source': 'FIXTURE',
      'audit_event_id': 'fixture-update-list-audit',
      'error': null,
      'body': {
        '版': 1,
        '更新一覧': <Object?>[],
        '件数': 0,
        '署名信頼設定': 'unconfigured',
        'download実行': 'suspended',
        'download_job': null,
        '適用実行': 'suspended',
        'rollback実行': 'suspended',
        'rollback状態': {
          '状態': 'unavailable',
          '現在版': null,
          '対象版': null,
        },
        '証拠種別': 'FIXTURE',
      },
      'shutdown_requested': false,
    };
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

Map<String, Object?> _brokerAdapterManifestMutationResponse(
  String operation,
  String adapterId, {
  String managementState = 'installed',
}) {
  final brokerOperation = operation == '導入' ? 'アダプター導入' : 'アダプター更新';
  return {
    'request_id': 'test-$brokerOperation',
    'operation': brokerOperation,
    'status': 'accepted',
    'evidence_source': 'INTERNAL_STATE',
    'audit_event_id': 'audit-$brokerOperation',
    'error': null,
    'health': null,
    'body': {
      '版': 1,
      'Adapter ID': adapterId,
      '管理状態': managementState,
      '検証状態': 'pending_review',
      '公開範囲': 'metadata_only',
      '証拠種別': 'INTERNAL_STATE',
      '権限生成': 'なし',
      'authority_strip': true,
      '操作': operation,
      '実行状態': 'accepted',
      '監査ID': 'audit-$brokerOperation',
      '復旧ID': 'recover-adapter-management',
    },
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

Map<String, Object?> _brokerHostListResponse({
  List<Map<String, Object?>> additionalHosts = const [],
}) {
  final response = _brokerAcceptedBody('Host一覧', {
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
  final body = response['body'] as Map<String, Object?>;
  final hosts = List<Map<String, Object?>>.from(body['Host一覧'] as List)
    ..addAll(additionalHosts);
  body['Host一覧'] = hosts;
  body['件数'] = hosts.length;
  return response;
}

Map<String, Object?> _brokerRemoteHostRecord() => {
      '版': 1,
      'Host ID': 'remote-host-fixture',
      '表示名': '登録Remote fixture',
      'Platform': 'linux',
      '接続状態': 'pending_review',
      'Trust': {
        'state': 'pending_review',
        'evidence_source': 'INTERNAL_STATE',
        'requires_operator_review': true,
      },
      '証明書/identity': {
        '種別': 'identity_hash',
        'hash': 'sha256:${List.filled(64, 'b').join()}',
      },
      'Runtime summary': {
        'runtime_count': 2,
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
      '登録監査ID': 'audit-host-remote',
    };

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

Map<String, Object?> _brokerAdapterListInternalResponse() {
  final response = _brokerAdapterListResponse();
  response['evidence_source'] = 'INTERNAL_STATE';
  return response;
}

List<Map<String, Object?>> _shellCoreProductBootstrapResponses() => [
      _brokerHealthResponse(),
      _brokerHostCapabilityResponse(),
      _brokerHostListResponse(),
      _brokerAdapterListResponse(),
      _brokerAgentAdapterListResponse(),
      _brokerAcceptedBody('normalize_payload', {'quarantined': false}),
      _brokerAcceptedBody('content_projection', {
        'redacted_payload': const <String, Object?>{},
      }),
      _brokerAcceptedBody('approval_edit', {'ok': false}),
      _brokerCommandSuspendedResponse(),
    ];

Map<String, Object?> _adapterManifestFixture({
  String adapterId = 'ui_fixture_adapter',
  String version = '1.0.0',
}) =>
    {
      '版': 1,
      'Adapter ID': adapterId,
      'Runtime ID': 'ui_fixture_runtime',
      '発行者': 'UI試験fixture',
      'source': 'owner_manifest',
      'version': version,
      'transport': 'mock',
      'Content Exposure': 'summary',
      '要求Capability': ['runtime.read'],
      '許可差分': ['content_visibility:none->summary'],
      '既知の危険': ['試験用未検証発行者'],
      '互換性': 'compatible',
      'authority_strip': true,
      'signed_manifest': true,
      '署名対象': '7b',
      '署名': List.filled(128, '0').join(),
      '署名者fingerprint': 'sha256:${List.filled(64, '0').join()}',
    };

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
