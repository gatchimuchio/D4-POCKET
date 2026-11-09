import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/screens/settings.dart';
import 'package:gui_shell_desktop/services/shell_core_client.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart' show BrokerTransport;

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
      final compose = transport.lastPayload('GUI Shell構成');
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
      await tester.tap(composeToolAction);
      await tester.pumpAndSettle();
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

      await _tap(tester, 'compose-preview-button');
      final preview = transport.lastPayload('GUI Shell構成Preview');
      expect(preview['current_manifest'], compose);
      expect(preview['candidate_manifest'], compose);
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
      await _select(tester, 'export-target-platform', 'Linux');
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

      await _tap(tester, 'compose-ai-edit-button');
      final proposal = transport.lastPayload('GUI Shell編集提案');
      expect(proposal['execution_mode'], 'proposal_only');
      expect(proposal['self_approval'], isFalse);
      expect(proposal['repository_rules_acknowledged'], isTrue);
      expect(tester.takeException(), isNull);
    },
  );
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

class _ComposeScreenTransport implements BrokerTransport {
  final List<Map<String, Object?>> requests = [];
  final String composeToolId = 'tool-${List<String>.filled(142, "a").join()}';

  Map<String, Object?> lastPayload(String operation) => requests
          .lastWhere((request) => request['operation'] == operation)['payload']!
      as Map<String, Object?>;

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    requests.add({'operation': operation, 'payload': payload ?? {}});
    final body = switch (operation) {
      'プロファイル一覧' => <String, Object?>{'Profiles': <Object?>[]},
      '更新一覧' => <String, Object?>{'更新一覧': <Object?>[]},
      'GUI Shell構成Catalog' => <String, Object?>{
          'version': 1,
          'runtime_ids': ['runtime.local'],
          'agents': [
            {
              'runtime_id': 'runtime.local',
              'metadata': {
                'adapter_id': 'adapter.codex.fixture',
                'agent_id': 'agent.codex',
                'provider': 'fixture-provider',
                'version': '1.0.0',
                'model': 'fixture-model',
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
              },
            },
          ],
          'evidence_source': 'INTERNAL_STATE',
          'authority_strip': true,
          'permission_generated': false,
        },
      'MCP接続一覧' => _mcpConnectionListBody(),
      'MCP Tool実行' => <String, Object?>{
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
        },
      '更新候補取得' => <String, Object?>{
          '状態': 'updated',
          '候補件数': 2,
        },
      'GUI Shell構成' => <String, Object?>{
          'compose_manifest': Map<String, Object?>.fromEntries(
            payload!.entries.toList().reversed,
          ),
          'build_status': 'not_started',
          'app_identity_status': 'not_generated',
          'permission_generated': false,
        },
      'GUI Shell構成Preview' => <String, Object?>{
          'preview_mode': 'version_rollback',
          'build_status': 'not_started',
          'export_status': 'not_started',
          'version_preview': {'rollback_available': false},
        },
      'GUI Shell書出し' => <String, Object?>{
          'manifest_file': {
            'path': 'fixture/manifest.json',
            'temporary_file_status': 'removed',
          },
        },
      'GUI Shell編集提案' => <String, Object?>{
          'execution_mode': 'proposal_only',
          'review_required': true,
          'files_written': false,
        },
      _ => <String, Object?>{},
    };
    return {
      'status': 'accepted',
      'operation': operation,
      if (operation == 'MCP接続一覧') 'evidence_source': 'INTERNAL_STATE',
      if (operation == 'GUI Shell構成Catalog')
        'evidence_source': 'INTERNAL_STATE',
      if (operation == 'MCP Tool実行') 'evidence_source': 'LIVE_RUNTIME',
      'body': body,
    };
  }

  Map<String, Object?> _mcpConnectionListBody() => {
        '版': 1,
        'MCP接続一覧': [
          {
            '版': 1,
            '契約種別': 'MCP外部概念射影',
            'Server': {'server_id': 'mcp.local', '表示名': 'Fixture MCP'},
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
