import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/services/export_client.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerClientException, BrokerTransport;

void main() {
  test('GUI Shell Windows書出しは新規identityと非継承Receiptを返す', () async {
    final transport = _FakeExportTransport({
      'status': 'accepted',
      'body': {
        'build_status': 'not_started',
        'artifact_status': 'not_built',
        'authority_strip': true,
        'credential_inherited': false,
        'permission_inherited': false,
        'approval_inherited': false,
        'audit_chain_inherited': false,
        'export_manifest': {
          'app_identity': {'app_id': 'd4-pocket-app-example'},
          'audit_store': {'inherited': false},
        },
      },
    });
    final receipt = await ExportClient(transport).export(
      exportId: 'export-settings',
      composeManifest: const {'version': 1, 'output_mode': 'manifest_only'},
    );

    expect(transport.operation, 'GUI Shell書出し');
    expect(transport.payload?['target_platform'], 'windows');
    expect(transport.payload?['export_mode'], 'manifest_only');
    expect(receipt['authority_strip'], isTrue);
    expect(receipt['permission_inherited'], isFalse);
  });

  test('GUI Shell Windows書出しの拒否を成功へ昇格しない', () async {
    final transport = _FakeExportTransport({
      'status': 'rejected',
      'error': {'message': 'owner_required'},
    });

    expect(
      () => ExportClient(transport).export(
        exportId: 'export-settings',
        composeManifest: const {},
      ),
      throwsA(isA<BrokerClientException>()),
    );
  });
}

class _FakeExportTransport implements BrokerTransport {
  _FakeExportTransport(this.response);

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
