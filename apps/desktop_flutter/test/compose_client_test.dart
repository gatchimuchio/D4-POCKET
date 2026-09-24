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
    final receipt = await ComposeClient(transport).compose({
      'version': 1,
      'compose_id': 'd4-pocket-local',
    });

    expect(transport.operation, 'GUI Shell構成');
    expect(transport.payload?['compose_id'], 'd4-pocket-local');
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
    final receipt = await ComposeClient(transport).preview(
      currentManifest: null,
      candidateManifest: const {'version': 1, 'compose_id': 'd4-pocket-local'},
    );

    expect(transport.operation, 'GUI Shell構成Preview');
    expect(transport.payload?['preview_mode'], 'version_rollback');
    expect(receipt['version_preview'], isA<Map>());
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
