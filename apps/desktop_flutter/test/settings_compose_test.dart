import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/screens/settings.dart';
import 'package:gui_shell_desktop/services/broker_client.dart'
    show brokerPayloadHash;
import 'package:gui_shell_desktop/services/shell_core_client.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart' show BrokerTransport;

import 'support/d4p_receipt_fixtures.dart';

void main() {
  testWidgets('更新センターからBrokerへ配布元候補取得を要求し一覧を更新する', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(800, 900);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final transport = _ComposeScreenTransport();
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SettingsScreen(
            client: ShellCoreClient.mock(transport: transport),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    final fetchButton = find.text('配布元から候補取得');
    await tester.ensureVisible(fetchButton);
    await tester.tap(fetchButton);
    await tester.pumpAndSettle();

    expect(
        transport.requests.where((request) => request['operation'] == '更新候補取得'),
        hasLength(1));
    expect(
      transport.lastPayload('更新候補取得'),
      {'版': 1},
    );
    expect(
      transport.requests.where((request) => request['operation'] == '更新一覧'),
      hasLength(2),
      reason: '候補取得後にBrokerの保存済み一覧を読み直す',
    );
    expect(
      find.text('Brokerが署名検証済み候補を取得しました（2件）。'),
      findsOneWidget,
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('SettingsのBroker切替で旧Compose catalogの遅延応答を破棄する', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(800, 900);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final oldTransport = _ComposeScreenTransport(
      mcpServerDisplayName: 'Broker A MCP',
      holdMcpCatalog: true,
    );
    final newTransport = _ComposeScreenTransport(
      mcpServerDisplayName: 'Broker B MCP',
    );

    Widget settingsFor(BrokerTransport transport) => MaterialApp(
          home: Scaffold(
            body: SettingsScreen(
              client: ShellCoreClient.mock(transport: transport),
            ),
          ),
        );

    await tester.pumpWidget(settingsFor(oldTransport));
    await tester.pumpAndSettle();
    final refresh = find.byKey(
      const ValueKey('compose-mcp-catalog-refresh'),
    );
    await tester.ensureVisible(refresh);
    await tester.tap(refresh);
    await tester.pump();
    await oldTransport.mcpCatalogRequestStarted.future.timeout(
      const Duration(seconds: 2),
    );

    await tester.pumpWidget(settingsFor(newTransport));
    await tester.pumpAndSettle();
    oldTransport.releasePendingMcpCatalog();
    await tester.pumpAndSettle();
    expect(find.text('Broker A MCP'), findsNothing);

    await _tap(tester, 'compose-mcp-catalog-refresh');
    expect(find.text('Broker B MCP'), findsOneWidget);
    expect(
      newTransport.requests
          .where((request) => request['operation'] == 'MCP接続一覧'),
      hasLength(1),
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('更新trustの片方だけではExport要求を送らない', (tester) async {
    tester.view.physicalSize = const Size(800, 900);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final transport = _ComposeScreenTransport();
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SettingsScreen(
            client: ShellCoreClient.mock(transport: transport),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    final trustPanel = find.byKey(
      const ValueKey('export-update-trust-advanced'),
    );
    await tester.ensureVisible(trustPanel);
    await tester.tap(trustPanel);
    await tester.pumpAndSettle();
    await _enter(
      tester,
      'export-update-public-key',
      '302a300506032b6570032100d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a',
    );
    await _tap(tester, 'compose-export-button');

    expect(
      transport.requests
          .where((request) => request['operation'] == 'GUI Shell書出し'),
      isEmpty,
    );
    expect(
      find.text('更新trustを設定する場合は、公開鍵とHTTPS配布元の両方を入力してください。'),
      findsOneWidget,
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('Broker切替中のCompose MCP Tool結果はunknownを通知し再送しない', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(800, 900);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final oldTransport = _ComposeScreenTransport(holdMcpToolCall: true);
    final newTransport = _ComposeScreenTransport();
    var unknownOutcomeCount = 0;

    Widget settingsFor(BrokerTransport transport) => MaterialApp(
          home: Scaffold(
            body: SettingsScreen(
              client: ShellCoreClient.mock(transport: transport),
              onBrokerMutationOutcomeUnknown: () => unknownOutcomeCount++,
            ),
          ),
        );

    await tester.pumpWidget(settingsFor(oldTransport));
    await tester.pumpAndSettle();
    await _tap(tester, 'compose-runtime-agent-catalog-refresh');
    await _tap(tester, 'compose-agent-agent.codex');
    await _tap(tester, 'compose-mcp-catalog-refresh');
    await _tap(tester, 'compose-mcp-server-mcp.local');
    await _tap(tester, 'compose-mcp-tool-${oldTransport.composeToolId}');
    await _tap(tester, 'compose-manifest-button');

    final toolAction = find.byKey(
      ValueKey('compose-tool-run-${oldTransport.composeToolId}'),
    );
    await tester.ensureVisible(toolAction);
    await tester.tap(toolAction);
    await tester.pumpAndSettle();
    await _enter(tester, 'compose-tool-action-arguments', '{"query":"needle"}');
    final submit = find.byKey(const ValueKey('compose-tool-action-submit'));
    await tester.ensureVisible(submit);
    await tester.tap(submit);
    for (var frame = 0;
        frame < 10 && !oldTransport.mcpToolCallRequestStarted.isCompleted;
        frame++) {
      await tester.pump(const Duration(milliseconds: 100));
    }
    expect(oldTransport.mcpToolCallRequestStarted.isCompleted, isTrue);

    await tester.pumpWidget(settingsFor(newTransport));
    await tester.pumpAndSettle();
    expect(unknownOutcomeCount, 1);

    oldTransport.releasePendingMcpToolCall();
    await tester.pumpAndSettle();
    expect(
      oldTransport.requests
          .where((request) => request['operation'] == 'MCP Tool実行'),
      hasLength(1),
    );
    expect(
      newTransport.requests
          .where((request) => request['operation'] == 'MCP Tool実行'),
      isEmpty,
    );
    expect(find.textContaining('Compose構成から既存MCP Toolを実行しました'), findsNothing);
    expect(tester.takeException(), isNull);
  });

  testWidgets('Broker切替中のAI Edit提案はunknownを通知し旧応答を投影・再送しない', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(800, 900);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final oldTransport = _ComposeScreenTransport(holdAiEditProposal: true);
    final newTransport = _ComposeScreenTransport();
    var unknownOutcomeCount = 0;

    Widget settingsFor(BrokerTransport transport) => MaterialApp(
          home: Scaffold(
            body: SettingsScreen(
              client: ShellCoreClient.mock(transport: transport),
              onBrokerMutationOutcomeUnknown: () => unknownOutcomeCount++,
            ),
          ),
        );

    await tester.pumpWidget(settingsFor(oldTransport));
    await tester.pumpAndSettle();
    await _enter(
      tester,
      'compose-ai-edit-expected-changes',
      'Previewで対象platformを表示する',
    );
    final proposalButton = find.byKey(const ValueKey('compose-ai-edit-button'));
    await tester.ensureVisible(proposalButton);
    await tester.tap(proposalButton);
    await tester.pump();
    await oldTransport.aiEditProposalRequestStarted.future.timeout(
      const Duration(seconds: 2),
    );

    await tester.pumpWidget(settingsFor(newTransport));
    await tester.pumpAndSettle();
    expect(unknownOutcomeCount, 1);

    oldTransport.releasePendingAiEditProposal();
    await tester.pumpAndSettle();
    expect(
      oldTransport.requests
          .where((request) => request['operation'] == 'GUI Shell編集提案'),
      hasLength(1),
    );
    expect(
      newTransport.requests
          .where((request) => request['operation'] == 'GUI Shell編集提案'),
      isEmpty,
    );
    expect(find.byKey(const ValueKey('compose-ai-edit-review-button')),
        findsNothing);
    expect(find.textContaining('編集提案を審査待ちで記録しました'), findsNothing);
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'Compose画面が選択をManifest、Preview、Module plan、編集提案へつなぐ',
    (tester) async {
      tester.view.physicalSize = const Size(800, 900);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final transport = _ComposeScreenTransport();
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: SettingsScreen(
              client: ShellCoreClient.mock(transport: transport),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final initialLayoutError = tester.takeException();
      expect(initialLayoutError, isNull, reason: '初期表示');
      expect(find.text('D4 Pocket 製品版: 開発版'), findsOneWidget);

      await _tap(tester, 'compose-runtime-agent-catalog-refresh');
      await _tap(tester, 'compose-agent-agent.codex');
      await _tap(tester, 'compose-mcp-catalog-refresh');
      await _tap(tester, 'compose-mcp-server-mcp.local');
      await _tap(tester, 'compose-mcp-tool-${transport.composeToolId}');
      await _enter(
        tester,
        'compose-capability-requirements',
        'task_execution',
      );
      await _select(tester, 'compose-theme-mode', '暗い');
      await _select(tester, 'compose-density', 'コンパクト');
      await _select(tester, 'compose-content-visibility', '伏字を含む表示');

      await _tap(tester, 'compose-manifest-button');
      final composeRequest = transport.lastPayload('GUI Shell構成');
      expect(composeRequest['catalog_snapshot_hash'], _composeSnapshotHash);
      final compose =
          composeRequest['compose_manifest']! as Map<String, Object?>;
      expect(compose['runtime_ids'], ['runtime.local']);
      expect(compose['agent_ids'], ['agent.codex']);
      expect(compose['tool_ids'], [transport.composeToolId]);
      expect(compose['mcp_connection_ids'], ['mcp.local']);
      expect(
        transport.requests
            .where((request) => request['operation'] == 'MCP接続一覧'),
        hasLength(1),
      );
      expect(
        transport.requests
            .where((request) => request['operation'] == 'MCP Tool実行'),
        isEmpty,
        reason: 'Compose Catalog選択はTool実行やPermissionを発生させない',
      );
      expect(compose['theme'], {'theme_id': 'd4-pocket', 'mode': 'dark'});
      expect(compose['capability_requirements'], [
        'task_execution',
      ]);
      expect(compose['settings'], {
        'locale': 'ja-JP',
        'density': 'compact',
        'content_visibility': 'redacted',
      });
      expect(compose['output_mode'], 'manifest_only');
      expect(compose['inheritance_policy'], {
        'authority': 'none',
        'permission': 'none',
        'approval': 'none',
        'credential': 'none',
        'audit_chain': 'none',
      });

      final composeToolAction = find.byKey(
        ValueKey('compose-tool-run-${transport.composeToolId}'),
      );
      await tester.ensureVisible(composeToolAction);
      expect(tester.widget<IconButton>(composeToolAction).onPressed, isNotNull);
      final composeNameField = find.byWidgetPredicate(
        (widget) =>
            widget is TextField && widget.decoration?.labelText == '構成表示名',
      );
      await tester.enterText(composeNameField, '編集後の構成');
      await tester.pump();
      expect(tester.widget<IconButton>(composeToolAction).onPressed, isNull);
      await tester.enterText(
        composeNameField,
        compose['display_name']! as String,
      );
      await tester.pump();
      await tester.ensureVisible(composeToolAction);
      await tester.pumpAndSettle();
      expect(tester.widget<IconButton>(composeToolAction).onPressed, isNotNull);
      await _tap(tester, 'compose-tool-run-${transport.composeToolId}');
      await _enter(
        tester,
        'compose-tool-action-arguments',
        '{"query":"needle"}',
      );
      await _tap(tester, 'compose-tool-action-submit');
      final composedToolCall = transport.lastPayload('MCP Tool実行');
      expect(composedToolCall['ServerID'], 'mcp.local');
      expect(composedToolCall['ToolID'], transport.composeToolId);
      expect(composedToolCall['arguments'], {'query': 'needle'});
      expect(composedToolCall['compose_manifest'], compose);
      expect(
        find.textContaining('Compose構成から既存MCP Toolを実行しました'),
        findsOneWidget,
      );

      await _select(tester, 'export-target-platform', 'Linux');
      await _tap(tester, 'compose-preview-button');
      final preview = transport.lastPayload('GUI Shell構成Preview');
      expect(preview['current_manifest'], compose);
      expect(preview['candidate_manifest'], compose);
      expect(preview['build_target'], 'desktop');
      expect(preview['preview_mode'], 'version_rollback');

      final historyModule = find.byKey(
        const ValueKey('export-module-shell.history'),
      );
      await tester.ensureVisible(historyModule);
      await tester.tap(historyModule);
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull, reason: 'Module選択');
      final trustPanel = find.byKey(
        const ValueKey('export-update-trust-advanced'),
      );
      await tester.ensureVisible(trustPanel);
      await tester.tap(trustPanel);
      await tester.pumpAndSettle();
      await _enter(
        tester,
        'export-update-public-key',
        '302a300506032b6570032100d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a',
      );
      await _enter(
        tester,
        'export-update-source',
        'https://updates.example.com/d4-pocket/stable',
      );
      await _tap(tester, 'compose-export-button');
      final export = transport.lastPayload('GUI Shell書出し');
      expect(export['target_platform'], 'linux');
      final moduleSelection = export['module_selection'] as Map;
      final selectedModules =
          moduleSelection['optional_module_ids'] as List<dynamic>;
      expect(selectedModules, isNot(contains('shell.history')));
      expect(export['compose_manifest'], compose);
      expect(export['update_trust'], {
        'public_key_der_hex':
            '302a300506032b6570032100d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a',
        'package_sources': [
          {
            'channel': 'stable',
            'base_url': 'https://updates.example.com/d4-pocket/stable',
          },
        ],
      });

      await _enter(
        tester,
        'compose-ai-edit-expected-changes',
        'Previewで対象platformを表示する\n現行の権限境界を維持する',
      );
      await _tap(tester, 'compose-ai-edit-button');
      final proposal = transport.lastPayload('GUI Shell編集提案');
      expect(proposal['execution_mode'], 'proposal_only');
      expect(proposal['self_approval'], isFalse);
      expect(proposal['repository_rules_acknowledged'], isTrue);
      expect(proposal['expected_changes'], [
        'Previewで対象platformを表示する',
        '現行の権限境界を維持する',
      ]);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('AI Editは内容確認後にだけ未信頼Task下書きをAgent Centerへ渡す', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(800, 900);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final transport = _ComposeScreenTransport();
    String? preparedTaskInstruction;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SettingsScreen(
            client: ShellCoreClient.mock(transport: transport),
            onPrepareAiEditTask: (proposal) {
              preparedTaskInstruction =
                  proposal.toImplementationTaskInstruction();
            },
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    await _enter(
      tester,
      'compose-ai-edit-expected-changes',
      'Previewで対象platformを表示する\n権限境界を変更しない',
    );
    await _tap(tester, 'compose-ai-edit-button');
    final proposal = transport.lastPayload('GUI Shell編集提案');
    expect(proposal['execution_mode'], 'proposal_only');
    expect(proposal['self_approval'], isFalse);
    expect(proposal['expected_changes'], [
      'Previewで対象platformを表示する',
      '権限境界を変更しない',
    ]);
    expect(preparedTaskInstruction, isNull);

    await _tap(tester, 'compose-ai-edit-review-button');
    final dialog = find.byType(AlertDialog);
    expect(find.descendant(of: dialog, matching: find.text('GUI Shell編集提案を確認')),
        findsOneWidget);
    expect(
      find.descendant(
        of: dialog,
        matching: find.textContaining('構成Preview結果へ対象platformを表示する'),
      ),
      findsOneWidget,
    );
    expect(
      find.descendant(
        of: dialog,
        matching: find
            .textContaining('apps/desktop_flutter/lib/screens/settings.dart'),
      ),
      findsOneWidget,
    );
    expect(
      find.descendant(
        of: dialog,
        matching: find.textContaining(
          'Previewで対象platformを表示する\n権限境界を変更しない',
        ),
      ),
      findsOneWidget,
    );
    expect(
      find.descendant(
        of: dialog,
        matching: find.textContaining(brokerPayloadHash(proposal)),
      ),
      findsOneWidget,
    );
    expect(
      find.descendant(
          of: dialog, matching: find.text('Audit ID: audit-compose-ai-edit')),
      findsOneWidget,
    );
    await tester.tap(find.descendant(of: dialog, matching: find.text('戻る')));
    await tester.pumpAndSettle();
    expect(preparedTaskInstruction, isNull);

    await _tap(tester, 'compose-ai-edit-review-button');
    await tester
        .tap(find.byKey(const ValueKey('compose-ai-edit-review-confirm')));
    await tester.pumpAndSettle();
    expect(preparedTaskInstruction, isNotNull);
    final taskEnvelope = jsonDecode(preparedTaskInstruction!) as Map;
    final handedProposal = taskEnvelope['proposal'] as Map;
    final handling = taskEnvelope['handling'] as Map;
    expect(taskEnvelope['context_type'], 'gui_shell_ai_edit_proposal');
    expect(handedProposal['instruction'], proposal['instruction']);
    expect(handedProposal['target_paths'], proposal['target_paths']);
    expect(handedProposal['expected_changes'], proposal['expected_changes']);
    expect(handedProposal['instruction_hash'], brokerPayloadHash(proposal));
    expect(handedProposal['audit_id'], 'audit-compose-ai-edit');
    expect(handedProposal['execution_mode'], 'proposal_only');
    expect(handling['proposal_content_is_untrusted'], isTrue);
    expect(handling['authority_inheritance'], 'none');
    expect(handling['permission_inheritance'], 'none');
    expect(handling['approval_inheritance'], 'none');
    expect(handling['credential_inheritance'], 'none');
    expect(handling['trust_inheritance'], 'none');
    expect(
      transport.requests
          .where((request) => request['operation'] == 'GUI Shell編集提案'),
      hasLength(1),
    );
    expect(
      transport.requests.any((request) => {
            'Agent作業要求検査',
            'AgentTaskWorkspacePermissionGrant',
            'AgentTaskOwnerApprovalGrant',
            'AgentTask実行',
          }.contains(request['operation'])),
      isFalse,
      reason: 'Settings確認は下書きを渡すだけでAgent Taskや権限操作を始めない',
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('AI Editは空または上限外の予定変更をBrokerへ送らない', (tester) async {
    tester.view.physicalSize = const Size(800, 900);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final transport = _ComposeScreenTransport();
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SettingsScreen(
            client: ShellCoreClient.mock(transport: transport),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    await _tap(tester, 'compose-ai-edit-button');
    expect(find.text('予定変更を1行以上入力してください。'), findsOneWidget);

    await _enter(
      tester,
      'compose-ai-edit-expected-changes',
      List<String>.generate(33, (index) => '変更項目$index').join('\n'),
    );
    await _tap(tester, 'compose-ai-edit-button');
    expect(find.text('予定変更は32項目以内、各240文字以内で入力してください。'), findsOneWidget);

    await _enter(
      tester,
      'compose-ai-edit-expected-changes',
      List<String>.filled(241, 'あ').join(),
    );
    await _tap(tester, 'compose-ai-edit-button');
    expect(find.text('予定変更は32項目以内、各240文字以内で入力してください。'), findsOneWidget);
    expect(
      transport.requests
          .where((request) => request['operation'] == 'GUI Shell編集提案'),
      isEmpty,
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('Composeは同じAgent型のRuntime instanceを1行で選択し対応先を維持する', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(800, 900);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final transport = _ComposeScreenTransport();
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SettingsScreen(
            client: ShellCoreClient.mock(transport: transport),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    await _tap(tester, 'compose-runtime-agent-catalog-refresh');
    expect(find.byKey(const ValueKey('compose-agent-agent.codex')),
        findsOneWidget);
    expect(find.textContaining('runtime.local=fixture-model'), findsOneWidget);
    expect(find.textContaining('runtime.local.secondary=fixture-model-b'),
        findsOneWidget);

    await _tap(tester, 'compose-runtime-runtime.local');
    await _tap(tester, 'compose-runtime-runtime.local.secondary');
    await _tap(tester, 'compose-agent-agent.codex');
    await _tap(tester, 'compose-manifest-button');
    final firstCompose =
        transport.lastPayload('GUI Shell構成')['compose_manifest']!
            as Map<String, Object?>;
    expect(firstCompose['runtime_ids'], [
      'runtime.local',
      'runtime.local.secondary',
    ]);
    expect(firstCompose['agent_ids'], ['agent.codex']);

    await _tap(tester, 'compose-runtime-runtime.local');
    expect(
      tester
          .widget<CheckboxListTile>(
            find.byKey(const ValueKey('compose-agent-agent.codex')),
          )
          .value,
      isTrue,
    );
    await _tap(tester, 'compose-manifest-button');
    final secondCompose =
        transport.lastPayload('GUI Shell構成')['compose_manifest']!
            as Map<String, Object?>;
    expect(secondCompose['runtime_ids'], [
      'runtime.local.secondary',
    ]);
    expect(secondCompose['agent_ids'], ['agent.codex']);
    expect(tester.takeException(), isNull);
  });
}

Future<void> _enter(WidgetTester tester, String key, String value) async {
  final field = find.byKey(ValueKey(key));
  await tester.ensureVisible(field);
  await tester.enterText(field, value);
  await tester.pumpAndSettle();
  expect(tester.takeException(), isNull, reason: '$key入力');
}

Future<void> _select(
  WidgetTester tester,
  String key,
  String choice,
) async {
  final field = find.byKey(ValueKey(key));
  await tester.ensureVisible(field);
  await tester.tap(field);
  await tester.pumpAndSettle();
  await tester.tap(find.text(choice).last);
  await tester.pumpAndSettle();
  expect(tester.takeException(), isNull, reason: '$key選択');
}

Future<void> _tap(WidgetTester tester, String key) async {
  final button = find.byKey(ValueKey(key));
  await tester.ensureVisible(button);
  await tester.tap(button);
  await tester.pumpAndSettle();
  expect(tester.takeException(), isNull, reason: '$key実行');
}

const _composeSnapshotHash =
    'sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
const _composeHostSwitchGeneration =
    'sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';

Map<String, Object?> _composeAgent(String runtimeId, String model) => {
      'runtime_id': runtimeId,
      'dependencies': {
        'status': 'partial',
        'items': [
          {'kind': 'runtime', 'id': runtimeId},
        ],
      },
      'metadata': {
        'adapter_id': 'adapter.codex.fixture',
        'agent_id': 'agent.codex',
        'provider': 'fixture-provider',
        'version': '1.0.0',
        'model': model,
        'status': 'ready',
        'capabilities': [
          {
            'capability_id': 'task_execution',
            'support': {
              'status': 'supported',
              'reason': 'fixture compatibility only',
            },
          },
        ],
        'workspace_requirements': {
          'mode': 'required',
          'boundary_policy': 'deny_outside_workspace',
          'secret_paths': ['.env', '.ssh', 'secrets/'],
        },
        'host_requirements': {
          'platforms': ['windows'],
          'network_scope': 'loopback_only',
          'process_spawn': {
            'status': 'unsupported',
            'reason': 'fixture only',
          },
        },
      },
    };

Map<String, Object?> _composeRuntime(String runtimeId, String model) {
  final metadata =
      _composeAgent(runtimeId, model)['metadata']! as Map<String, Object?>;
  return {
    'runtime_id': runtimeId,
    'adapter_id': metadata['adapter_id'],
    'version': metadata['version'],
    'abi': 'unknown',
    'architecture': 'unknown',
    'os': ['windows'],
    'status': metadata['status'],
    'capabilities': metadata['capabilities'],
    'permissions': {'status': 'not_granted', 'requirements': <Object?>[]},
    'dependencies': {
      'status': 'partial',
      'items': [
        {'kind': 'adapter', 'id': metadata['adapter_id']},
      ],
    },
    'scope': {
      'workspace': metadata['workspace_requirements'],
      'host': metadata['host_requirements'],
    },
    'evidence_source': 'INTERNAL_STATE',
  };
}

Map<String, Object?> _composeHost() => {
      'host_id': 'gui-shell-local-windows',
      'display_name': 'Local Windows',
      'kind': 'local',
      'version': 'unknown',
      'abi': 'unknown',
      'architecture': 'x86_64',
      'os': 'windows',
      'status': 'degraded',
      'connection_state': 'local',
      'trust_state': 'local_runtime_only',
      'permissions': {'status': 'not_granted', 'requirements': <Object?>[]},
      'dependencies': {'status': 'unknown', 'items': <Object?>[]},
      'scope': {'capabilities': <Object?>[]},
      'evidence_source': 'LIVE_RUNTIME',
      'authority_strip': true,
    };

Map<String, Object?> _composeTool(String toolId) => {
      'server_id': 'mcp.local',
      'server_name': 'Local MCP',
      'server_version': '1.0.0',
      'protocol_version': '2026-07-28',
      'tool_id': toolId,
      'name': 'echo',
      'input_schema_hash':
          'sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc',
      'status': 'connected',
      'authentication_status': 'missing',
      'abi': 'unknown',
      'os': ['unknown'],
      'dependencies': {
        'status': 'partial',
        'items': [
          {'kind': 'mcp_server', 'id': 'mcp.local'},
        ],
      },
      'scope': {
        'transport': 'stdio',
        'boundary': 'current_registered_connection',
      },
      'permission_requirement':
          'separate_owner_confirmation_and_one_shot_permission',
      'permission_generated': false,
      'evidence_source': 'LIVE_RUNTIME',
    };

class _ComposeScreenTransport implements BrokerTransport {
  _ComposeScreenTransport({
    this.mcpServerDisplayName = 'Fixture MCP',
    this.holdMcpCatalog = false,
    this.holdMcpToolCall = false,
    this.holdAiEditProposal = false,
  });

  final List<Map<String, Object?>> requests = [];
  final String composeToolId = 'tool-${List<String>.filled(142, "a").join()}';
  final String mcpServerDisplayName;
  final bool holdMcpCatalog;
  final bool holdMcpToolCall;
  final bool holdAiEditProposal;
  final Completer<void> mcpCatalogRequestStarted = Completer<void>();
  final Completer<void> mcpToolCallRequestStarted = Completer<void>();
  final Completer<void> aiEditProposalRequestStarted = Completer<void>();
  final Completer<Map<String, Object?>> _pendingMcpCatalog =
      Completer<Map<String, Object?>>();
  final Completer<Map<String, Object?>> _pendingMcpToolCall =
      Completer<Map<String, Object?>>();
  final Completer<Map<String, Object?>> _pendingAiEditProposal =
      Completer<Map<String, Object?>>();

  void releasePendingMcpCatalog() {
    if (_pendingMcpCatalog.isCompleted) return;
    _pendingMcpCatalog.complete(_mcpConnectionListBody());
  }

  void releasePendingMcpToolCall() {
    if (_pendingMcpToolCall.isCompleted) return;
    _pendingMcpToolCall.complete(_mcpToolCallBody());
  }

  void releasePendingAiEditProposal() {
    if (_pendingAiEditProposal.isCompleted) return;
    _pendingAiEditProposal.complete(
      _aiEditProposalBody(lastPayload('GUI Shell編集提案')),
    );
  }

  Future<Map<String, Object?>> _waitForMcpCatalog() {
    if (!mcpCatalogRequestStarted.isCompleted) {
      mcpCatalogRequestStarted.complete();
    }
    return _pendingMcpCatalog.future;
  }

  Future<Map<String, Object?>> _waitForMcpToolCall() {
    if (!mcpToolCallRequestStarted.isCompleted) {
      mcpToolCallRequestStarted.complete();
    }
    return _pendingMcpToolCall.future;
  }

  Future<Map<String, Object?>> _waitForAiEditProposal() {
    if (!aiEditProposalRequestStarted.isCompleted) {
      aiEditProposalRequestStarted.complete();
    }
    return _pendingAiEditProposal.future;
  }

  Map<String, Object?> lastPayload(String operation) => requests
          .lastWhere((request) => request['operation'] == operation)['payload']!
      as Map<String, Object?>;

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    requests.add({'operation': operation, 'payload': payload ?? {}});
    if (operation == 'GUI Shell書出し') {
      return exportBrokerResponse(payload!);
    }
    final body = switch (operation) {
      'プロファイル一覧' => <String, Object?>{'Profiles': <Object?>[]},
      '更新一覧' => <String, Object?>{'更新一覧': <Object?>[]},
      'GUI Shell構成Catalog' => <String, Object?>{
          'version': 3,
          'snapshot_hash': _composeSnapshotHash,
          'runtime_ids': ['runtime.local', 'runtime.local.secondary'],
          'runtimes': [
            _composeRuntime('runtime.local', 'fixture-model'),
            _composeRuntime('runtime.local.secondary', 'fixture-model-b'),
          ],
          'agents': [
            _composeAgent('runtime.local', 'fixture-model'),
            _composeAgent('runtime.local.secondary', 'fixture-model-b'),
          ],
          'tools': [_composeTool(composeToolId)],
          'hosts': [_composeHost()],
          'host_switch_generation': _composeHostSwitchGeneration,
          'evidence_source': 'INTERNAL_STATE',
          'authority_strip': true,
          'permission_generated': false,
        },
      'MCP接続一覧' =>
        holdMcpCatalog ? await _waitForMcpCatalog() : _mcpConnectionListBody(),
      'MCP Tool実行' =>
        holdMcpToolCall ? await _waitForMcpToolCall() : _mcpToolCallBody(),
      'GUI Shell編集提案' => holdAiEditProposal
          ? await _waitForAiEditProposal()
          : _aiEditProposalBody(payload!),
      '更新候補取得' => <String, Object?>{
          '状態': 'updated',
          '候補件数': 2,
        },
      'GUI Shell構成' => <String, Object?>{
          'version': 1,
          'operation': operation,
          'status': 'accepted',
          'compose_manifest': Map<String, Object?>.fromEntries(
            (payload!['compose_manifest']! as Map<String, Object?>)
                .entries
                .toList()
                .reversed,
          ),
          'build_status': 'not_started',
          'app_identity_status': 'not_generated',
          'permission_generated': false,
          'authority_strip': true,
          'inheritance_policy': const {
            'authority': 'none',
            'permission': 'none',
            'approval': 'none',
            'credential': 'none',
            'audit_chain': 'none',
          },
          'evidence_source': 'INTERNAL_STATE',
          'audit_id': 'broker-audit-compose',
        },
      'GUI Shell構成Preview' => <String, Object?>{
          'version': 1,
          'operation': operation,
          'status': 'accepted',
          'current_manifest': payload!['current_manifest'],
          'candidate_manifest': payload['candidate_manifest'],
          'build_target': payload['build_target'],
          'preview_mode': 'version_rollback',
          'diff': const {
            'runtime_added': <String>[],
            'runtime_removed': <String>[],
            'agent_added': <String>[],
            'agent_removed': <String>[],
            'tool_added': <String>[],
            'tool_removed': <String>[],
            'mcp_added': <String>[],
            'mcp_removed': <String>[],
            'capability_added': <String>[],
            'capability_removed': <String>[],
            'changed_fields': <String>[],
          },
          'permission_requirements': [
            for (final capability in (payload['candidate_manifest']!
                as Map<String, Object?>)['capability_requirements']! as List)
              {
                'capability': capability,
                'permission_status': 'not_generated',
                'approval_status': 'not_requested',
              },
          ],
          'version_preview': {
            'current_version': payload['current_manifest'] == null ? null : 1,
            'candidate_version': 1,
            'rollback_available': false,
            'rollback_status': 'not_executable',
            'rollback_reason': 'Previewはrollback要求を実行しない',
          },
          'build_status': 'not_started',
          'export_status': 'not_started',
          'authority_strip': true,
          'inheritance_policy': const {
            'authority': 'none',
            'permission': 'none',
            'approval': 'none',
            'credential': 'none',
            'audit_chain': 'none',
          },
          'evidence_source': 'INTERNAL_STATE',
          'audit_id': 'broker-audit-compose',
        },
      _ => <String, Object?>{},
    };
    return {
      'status': 'accepted',
      'operation': operation,
      if (operation == 'MCP接続一覧') 'evidence_source': 'INTERNAL_STATE',
      if (operation == 'GUI Shell構成Catalog')
        'evidence_source': 'INTERNAL_STATE',
      if (operation == 'GUI Shell構成' || operation == 'GUI Shell構成Preview')
        'evidence_source': 'INTERNAL_STATE',
      if (operation == 'GUI Shell構成' || operation == 'GUI Shell構成Preview')
        'audit_event_id': 'broker-audit-compose',
      if (operation == 'MCP Tool実行') 'evidence_source': 'LIVE_RUNTIME',
      if (operation == 'GUI Shell編集提案') 'evidence_source': 'INTERNAL_STATE',
      if (operation == 'GUI Shell編集提案')
        'audit_event_id': 'audit-compose-ai-edit',
      'error': null,
      'body': body,
    };
  }

  Map<String, Object?> _mcpToolCallBody() => {
        '版': 1,
        '契約種別': 'MCP Tool実行receipt',
        'ServerID': 'mcp.local',
        'ToolID': composeToolId,
        '名前': 'echo',
        'arguments_hash': 'sha256:${List<String>.filled(64, 'a').join()}',
        'result_hash': 'sha256:${List<String>.filled(64, 'b').join()}',
        'Tool error': false,
        'content_count': 1,
        'content_types': ['text'],
        '接続状態': 'connected',
        '能力ID': 'mcp.tool.call',
        '権限ID': 'permission.mcp.tool.call.one_shot',
        '承認状態': 'native_owner_confirmed',
        '承認監査ID': 'audit-compose-approval',
        '復旧ID': 'inspect-mcp-tool-side-effect',
        '権限生成': 'Broker内一回限りPermissionを消費',
        '公開範囲': 'hash_only',
        '証拠種別': 'LIVE_RUNTIME',
        '監査ID': 'audit-compose-tool-call',
      };

  Map<String, Object?> _aiEditProposalBody(Map<String, Object?> payload) => {
        'version': 1,
        'operation': 'GUI Shell編集提案',
        'status': 'accepted',
        'edit_id': payload['edit_id'],
        'scope': payload['scope'],
        'target_paths': payload['target_paths'],
        'expected_change_count': (payload['expected_changes']! as List).length,
        'execution_mode': 'proposal_only',
        'review_required': true,
        'apply_status': 'not_started',
        'files_written': false,
        'permission_generated': false,
        'approval_state': 'owner_review_required',
        'authority_strip': true,
        'evidence_source': 'INTERNAL_STATE',
        'instruction_hash': brokerPayloadHash(payload),
        'audit_id': 'audit-compose-ai-edit',
      };

  Map<String, Object?> _mcpConnectionListBody() => {
        '版': 1,
        'MCP接続一覧': [
          {
            '版': 1,
            '契約種別': 'MCP外部概念射影',
            'Server': {
              'server_id': 'mcp.local',
              '表示名': mcpServerDisplayName,
            },
            'Transport': {'kind': 'stdio'},
            'Tool': [
              {
                'tool_id': composeToolId,
                'name': 'echo',
                'description_summary': '',
                'input_schema_hash':
                    'sha256:${List<String>.filled(64, "b").join()}',
                'risk': 'unknown',
                'status': {
                  'status': 'supported',
                  'reason': 'fixture MCP schema',
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
            '実行状態': 'ready',
            '能力ID': 'mcp.connection.connect',
            '権限ID': 'permission.mcp.connection.connect',
            '承認状態': 'owner_control_approved',
            '復旧ID': 'recover-mcp-connection',
            '接続監査ID': 'audit-mcp-fixture',
          },
        ],
        '件数': 1,
        'Agent能力hash': 'sha256:${List<String>.filled(64, "c").join()}',
        'Agent能力件数': 1,
        'Agent能力状態': 'available',
        '公開範囲': 'metadata_only',
        '証拠種別': 'INTERNAL_STATE',
      };
}
