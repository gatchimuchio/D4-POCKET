import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/services/compose_client.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerClientException, BrokerTransport;

void main() {
  test('GUI Shell構成はBrokerへManifestだけを送りreceiptを返す', () async {
    final manifest = buildComposeManifestDraft(
      composeId: ' d4-pocket-local ',
      displayName: ' D4 Pocket local ',
      runtimeIds: 'runtime.local',
      agentIds: 'agent.example',
      toolIds: 'tool.notes',
      mcpConnectionIds: 'mcp.local',
    );
    final transport = _FakeComposeTransport(_composeResponse(manifest));
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
      'operation': 'GUI Shell構成',
      'error': {
        'code': 'gui_shell_compose_invalid',
        'message': 'secret-value-must-not-reach-the-ui',
      },
    });

    try {
      await ComposeClient(transport).compose(const {});
      fail('Broker拒否をacceptedへ昇格してはいけない');
    } on BrokerClientException catch (error) {
      expect(error.message, contains('gui_shell_compose_invalid'));
      expect(
          error.message, isNot(contains('secret-value-must-not-reach-the-ui')));
    }
  });

  test('ComposeはAuthority昇格、要求不一致、Audit不一致のreceiptを拒否する', () async {
    final manifest = buildComposeManifestDraft(
      composeId: 'd4-pocket-local',
      displayName: 'D4 Pocket local',
      runtimeIds: '',
      agentIds: '',
      toolIds: '',
      mcpConnectionIds: '',
    );
    final authorityEscalation = _composeResponse(
      manifest,
      permissionGenerated: true,
    );
    final staleManifest = _composeResponse(
      buildComposeManifestDraft(
        composeId: 'd4-pocket-other',
        displayName: 'Different configuration',
        runtimeIds: '',
        agentIds: '',
        toolIds: '',
        mcpConnectionIds: '',
      ),
    );
    final auditMismatch =
        _composeResponse(manifest, bodyAuditId: 'broker-audit-18');

    for (final response in [
      authorityEscalation,
      staleManifest,
      auditMismatch
    ]) {
      await expectLater(
        ComposeClient(_FakeComposeTransport(response)).compose(manifest),
        throwsA(isA<BrokerClientException>()),
      );
    }
  });

  test('GUI Shell構成Previewは差分とrollback非実行状態を返す', () async {
    final currentManifest = buildComposeManifestDraft(
      composeId: 'existing',
      displayName: 'Existing configuration',
      runtimeIds: '',
      agentIds: '',
      toolIds: '',
      mcpConnectionIds: '',
    );
    final candidateManifest = buildComposeManifestDraft(
      composeId: 'd4-pocket-local',
      displayName: 'D4 Pocket local',
      runtimeIds: '',
      agentIds: '',
      toolIds: '',
      mcpConnectionIds: '',
    );
    final transport = _FakeComposeTransport(_previewResponse(
      currentManifest: currentManifest,
      candidateManifest: candidateManifest,
      buildTarget: 'desktop',
    ));
    final receipt = await ComposeClient(transport).preview(
      currentManifest: currentManifest,
      candidateManifest: candidateManifest,
      buildTarget: 'desktop',
    );

    expect(transport.operation, 'GUI Shell構成Preview');
    expect(transport.payload?['preview_mode'], 'version_rollback');
    expect(transport.payload?['build_target'], 'desktop');
    expect(transport.payload?['current_manifest'], currentManifest);
    expect(transport.payload?['candidate_manifest'], candidateManifest);
    expect(receipt['version_preview'], isA<Map>());
  });

  test('PreviewはAuthority昇格と別target・別Manifestのreceiptを拒否する', () async {
    final candidate = buildComposeManifestDraft(
      composeId: 'd4-pocket-local',
      displayName: 'D4 Pocket local',
      runtimeIds: '',
      agentIds: '',
      toolIds: '',
      mcpConnectionIds: '',
    );
    final authorityEscalation = _previewResponse(
      currentManifest: null,
      candidateManifest: candidate,
      buildTarget: 'desktop',
      authorityStrip: false,
    );
    final wrongTarget = _previewResponse(
      currentManifest: null,
      candidateManifest: candidate,
      buildTarget: 'windows',
    );
    final staleManifest = _previewResponse(
      currentManifest: null,
      candidateManifest: buildComposeManifestDraft(
        composeId: 'd4-pocket-other',
        displayName: 'Different configuration',
        runtimeIds: '',
        agentIds: '',
        toolIds: '',
        mcpConnectionIds: '',
      ),
      buildTarget: 'desktop',
    );

    for (final response in [authorityEscalation, wrongTarget, staleManifest]) {
      await expectLater(
        ComposeClient(_FakeComposeTransport(response)).preview(
          currentManifest: null,
          candidateManifest: candidate,
          buildTarget: 'desktop',
        ),
        throwsA(isA<BrokerClientException>()),
      );
    }
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

Map<String, Object?> _composeResponse(
  Map<String, Object?> manifest, {
  bool permissionGenerated = false,
  bool authorityStrip = true,
  String bodyAuditId = 'broker-audit-17',
}) {
  final body = <String, Object?>{
    'version': 1,
    'operation': 'GUI Shell構成',
    'status': 'accepted',
    'compose_manifest': manifest,
    'build_status': 'not_started',
    'app_identity_status': 'not_generated',
    'permission_generated': permissionGenerated,
    'authority_strip': authorityStrip,
    'inheritance_policy': _noInheritance(),
    'evidence_source': 'INTERNAL_STATE',
    'audit_id': bodyAuditId,
  };
  return _acceptedResponse('GUI Shell構成', body, auditId: 'broker-audit-17');
}

Map<String, Object?> _previewResponse({
  required Map<String, Object?>? currentManifest,
  required Map<String, Object?> candidateManifest,
  required String buildTarget,
  bool authorityStrip = true,
}) {
  final requirements = candidateManifest['capability_requirements']! as List;
  final body = <String, Object?>{
    'version': 1,
    'operation': 'GUI Shell構成Preview',
    'status': 'accepted',
    'current_manifest': currentManifest,
    'candidate_manifest': candidateManifest,
    'build_target': buildTarget,
    'preview_mode': 'version_rollback',
    'diff': {
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
      'changed_fields': <String>['initial_configuration'],
    },
    'permission_requirements': [
      for (final capability in requirements)
        {
          'capability': capability,
          'permission_status': 'not_generated',
          'approval_status': 'not_requested',
        },
    ],
    'version_preview': {
      'current_version': currentManifest == null ? null : 1,
      'candidate_version': 1,
      'rollback_available': false,
      'rollback_status': 'not_executable',
      'rollback_reason': 'Previewはrollback要求を実行しない',
    },
    'build_status': 'not_started',
    'export_status': 'not_started',
    'authority_strip': authorityStrip,
    'inheritance_policy': _noInheritance(),
    'evidence_source': 'INTERNAL_STATE',
    'audit_id': 'broker-audit-17',
  };
  return _acceptedResponse('GUI Shell構成Preview', body);
}

Map<String, Object?> _acceptedResponse(
  String operation,
  Map<String, Object?> body, {
  String auditId = 'broker-audit-17',
}) =>
    {
      'status': 'accepted',
      'operation': operation,
      'evidence_source': 'INTERNAL_STATE',
      'audit_event_id': auditId,
      'error': null,
      'body': body,
    };

Map<String, String> _noInheritance() => const {
      'authority': 'none',
      'permission': 'none',
      'approval': 'none',
      'credential': 'none',
      'audit_chain': 'none',
    };
