import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/screens/settings.dart';
import 'package:gui_shell_desktop/services/shell_core_client.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart' show BrokerTransport;

void main() {
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

      await _enter(tester, 'compose-runtime-ids', 'runtime.local');
      await _enter(tester, 'compose-agent-ids', 'agent.codex');
      await _enter(tester, 'compose-tool-ids', 'tool.notes');
      await _enter(tester, 'compose-mcp-ids', 'mcp.local');
      await _enter(
        tester,
        'compose-capability-requirements',
        'runtime.read\nagent.metadata',
      );
      await _select(tester, 'compose-theme-mode', '暗い');
      await _select(tester, 'compose-density', 'コンパクト');
      await _select(tester, 'compose-content-visibility', '伏字を含む表示');

      await _tap(tester, 'compose-manifest-button');
      final compose = transport.lastPayload('GUI Shell構成');
      expect(compose['runtime_ids'], ['runtime.local']);
      expect(compose['agent_ids'], ['agent.codex']);
      expect(compose['tool_ids'], ['tool.notes']);
      expect(compose['mcp_connection_ids'], ['mcp.local']);
      expect(compose['theme'], {'theme_id': 'd4-pocket', 'mode': 'dark'});
      expect(compose['capability_requirements'], [
        'runtime.read',
        'agent.metadata',
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
      await _tap(tester, 'compose-export-button');
      final export = transport.lastPayload('GUI Shell書出し');
      final moduleSelection = export['module_selection'] as Map;
      final selectedModules =
          moduleSelection['optional_module_ids'] as List<dynamic>;
      expect(selectedModules, isNot(contains('shell.history')));
      expect(export['compose_manifest'], compose);

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
      'GUI Shell構成' => <String, Object?>{
          'compose_manifest': payload,
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
      'body': body,
    };
  }
}
