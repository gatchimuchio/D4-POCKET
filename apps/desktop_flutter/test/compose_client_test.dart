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

  test('構成参照IDは空行と前後空白だけを整理し重複はBroker検証へ残す', () {
    expect(
      composeReferenceIdsFromLines(
        '  runtime.local  \r\n\nagent.example\n runtime.local ',
      ),
      ['runtime.local', 'agent.example', 'runtime.local'],
    );
  });
}

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
