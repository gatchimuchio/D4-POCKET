import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/services/compose_client.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerClientException, BrokerTransport;

void main() {
  test('GUI Shell構成はBrokerへManifestだけを送りreceiptを返す', () async {
    final transport = _FakeComposeTransport({
      'status': 'accepted',
      'body': {
        'build_status': 'not_started',
        'app_identity_status': 'not_generated',
        'permission_generated': false,
        'compose_manifest': {'compose_id': 'd4-pocket-local'},
      },
    });
    final manifest = buildComposeManifestDraft(
      composeId: ' d4-pocket-local ',
      displayName: ' D4 Pocket local ',
      runtimeIds: 'runtime.local',
      agentIds: 'agent.example',
      toolIds: 'tool.notes',
      mcpConnectionIds: 'mcp.local',
    );
    final receipt = await ComposeClient(transport).compose(manifest);

    expect(transport.operation, 'GUI Shell構成');
    expect(transport.payload?['compose_id'], 'd4-pocket-local');
    expect(transport.payload?['runtime_ids'], manifest['runtime_ids']);
    expect(transport.payload?['agent_ids'], manifest['agent_ids']);
    expect(transport.payload?['tool_ids'], manifest['tool_ids']);
    expect(
      transport.payload?['mcp_connection_ids'],
      manifest['mcp_connection_ids'],
    );
    expect(manifest['output_mode'], 'manifest_only');
    expect(manifest['inheritance_policy'], {
      'authority': 'none',
      'permission': 'none',
      'approval': 'none',
      'credential': 'none',
      'audit_chain': 'none',
    });
    expect(receipt['permission_generated'], isFalse);
    expect(receipt['build_status'], 'not_started');
  });

  test('GUI Shell構成の拒否を成功へ昇格しない', () async {
    final transport = _FakeComposeTransport({
      'status': 'rejected',
      'error': {'message': 'gui_shell_compose_invalid'},
    });

    expect(
      () => ComposeClient(transport).compose(const {}),
      throwsA(isA<BrokerClientException>()),
    );
  });

  test('GUI Shell構成Previewは差分とrollback非実行状態を返す', () async {
    final transport = _FakeComposeTransport({
      'status': 'accepted',
      'body': {
        'preview_mode': 'version_rollback',
        'diff': {
          'changed_fields': ['initial_configuration']
        },
        'version_preview': {'rollback_available': false},
      },
    });
    const currentManifest = {
      'version': 1,
      'compose_id': 'existing',
    };
    final receipt = await ComposeClient(transport).preview(
      currentManifest: currentManifest,
      candidateManifest: const {'version': 1, 'compose_id': 'd4-pocket-local'},
    );

    expect(transport.operation, 'GUI Shell構成Preview');
    expect(transport.payload?['preview_mode'], 'version_rollback');
    expect(transport.payload?['current_manifest'], currentManifest);
    expect(receipt['version_preview'], isA<Map>());
  });

  test('Compose Catalogは同じAgent型の別Runtime instanceを保持する', () async {
    final transport = _FakeComposeTransport(_catalogResponse(
      runtimeIds: const ['codex-a', 'codex-b'],
      agents: [
        _agentEntry(
          'codex-a',
          adapterId: 'codex-cli',
          agentId: 'codex',
          model: 'model-a',
        ),
        _agentEntry(
          'codex-b',
          adapterId: 'codex-cli',
          agentId: 'codex',
          model: 'model-b',
        ),
      ],
    ));

    final catalog = await ComposeClient(transport).refreshRuntimeAgentCatalog();

    expect(catalog.runtimeIds, ['codex-a', 'codex-b']);
    expect(catalog.agents, hasLength(2));
    expect(
        catalog.agents.map((entry) => entry.runtimeId), ['codex-a', 'codex-b']);
    expect(
        catalog.agents.map((entry) => entry.agent.agentId), ['codex', 'codex']);
    expect(catalog.agents.map((entry) => entry.agent.model),
        ['model-a', 'model-b']);
  });

  test('Compose CatalogはRuntime重複とAgent型IDの別Adapter衝突を拒否する', () async {
    final duplicateRuntime = _FakeComposeTransport(_catalogResponse(
      runtimeIds: const ['codex-a'],
      agents: [
        _agentEntry('codex-a', adapterId: 'codex-cli', agentId: 'codex'),
        _agentEntry('codex-a', adapterId: 'codex-cli', agentId: 'codex'),
      ],
    ));
    await expectLater(
      ComposeClient(duplicateRuntime).refreshRuntimeAgentCatalog(),
      throwsA(isA<BrokerClientException>()),
    );

    final conflictingAgentType = _FakeComposeTransport(_catalogResponse(
      runtimeIds: const ['codex-a', 'other'],
      agents: [
        _agentEntry('codex-a', adapterId: 'codex-cli', agentId: 'codex'),
        _agentEntry('other', adapterId: 'other-adapter', agentId: 'codex'),
      ],
    ));
    await expectLater(
      ComposeClient(conflictingAgentType).refreshRuntimeAgentCatalog(),
      throwsA(isA<BrokerClientException>()),
    );
  });

  test('構成参照IDは空行と前後空白だけを整理し重複はBroker検証へ残す', () {
    expect(
      composeReferenceIdsFromLines(
        '  runtime.local  \r\n\nagent.example\n runtime.local ',
      ),
      ['runtime.local', 'agent.example', 'runtime.local'],
    );
  });

  test('Theme、表示設定、機能要件の選択をManifestへ反映する', () {
    final manifest = buildComposeManifestDraft(
      composeId: 'compose.dark',
      displayName: '暗色構成',
      runtimeIds: 'runtime.local',
      agentIds: 'agent.codex',
      toolIds: 'tool.notes',
      mcpConnectionIds: 'mcp.local',
      themeMode: 'dark',
      density: 'compact',
      contentVisibility: 'redacted',
      capabilityRequirements: 'runtime.read\n\nagent.metadata',
    );

    expect(manifest['theme'], {'theme_id': 'd4-pocket', 'mode': 'dark'});
    expect(manifest['settings'], {
      'locale': 'ja-JP',
      'density': 'compact',
      'content_visibility': 'redacted',
    });
    expect(manifest['capability_requirements'], [
      'runtime.read',
      'agent.metadata',
    ]);
    expect(manifest['inheritance_policy'], {
      'authority': 'none',
      'permission': 'none',
      'approval': 'none',
      'credential': 'none',
      'audit_chain': 'none',
    });
  });
}

Map<String, Object?> _catalogResponse({
  required List<String> runtimeIds,
  required List<Map<String, Object?>> agents,
}) =>
    {
      'status': 'accepted',
      'operation': 'GUI Shell構成Catalog',
      'evidence_source': 'INTERNAL_STATE',
      'body': {
        'version': 1,
        'runtime_ids': runtimeIds,
        'agents': agents,
        'evidence_source': 'INTERNAL_STATE',
        'authority_strip': true,
        'permission_generated': false,
      },
    };

Map<String, Object?> _agentEntry(
  String runtimeId, {
  required String adapterId,
  required String agentId,
  String model = 'fixture-model',
}) =>
    {
      'runtime_id': runtimeId,
      'metadata': {
        'adapter_id': adapterId,
        'agent_id': agentId,
        'provider': 'OpenAI',
        'version': '1.0.0',
        'model': model,
        'status': 'ready',
        'capabilities': <Object?>[],
      },
    };

class _FakeComposeTransport implements BrokerTransport {
  _FakeComposeTransport(this.response);

  final Map<String, Object?> response;
  String? operation;
  Map<String, Object?>? payload;

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    this.operation = operation;
    this.payload = payload;
    return response;
  }
}
