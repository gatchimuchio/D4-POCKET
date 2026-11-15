import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/services/export_client.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerClientException, BrokerTransport;

import 'support/d4p_receipt_fixtures.dart';

void main() {
  test('GUI Shell書出しは新規identityと非継承Receiptを返す', () async {
    final transport = _FakeExportTransport();
    final receipt = await ExportClient(transport).export(
      exportId: 'export-settings',
      composeManifest: _composeManifest(),
      optionalModuleIds: const ['shell.trace_inspector'],
    );

    expect(transport.operation, 'GUI Shell書出し');
    expect(transport.payload?['target_platform'], 'windows');
    expect(transport.payload?['export_mode'], 'manifest_file');
    expect(
      (transport.payload?['module_selection'] as Map)['optional_module_ids'],
      ['shell.trace_inspector'],
    );
    expect(receipt['authority_strip'], isTrue);
    expect(receipt['permission_inherited'], isFalse);
    final manifest = receipt['export_manifest'] as Map;
    final plan = manifest['module_plan'] as Map;
    expect(
      plan['included_module_ids'],
      containsAll(['shell.trace_inspector', 'shell.observability']),
    );
    expect((receipt['manifest_file'] as Map)['path'],
        r'C:\D4Pocket\Exports\d4-pocket-app-11111111111111111111111111111111.json');
  });

  test('Linux向けExport targetをBroker要求とReceiptへ保持する', () async {
    final transport = _FakeExportTransport();
    await ExportClient(transport).export(
      exportId: 'export-linux',
      composeManifest: _composeManifest(),
      targetPlatform: 'linux',
    );

    expect(transport.payload?['target_platform'], 'linux');
  });

  test('Exportは公開更新trustだけをBrokerへ送り秘密鍵欄を作らない', () async {
    final transport = _FakeExportTransport();
    await ExportClient(transport).export(
      exportId: 'export-settings',
      composeManifest: _composeManifest(),
      updatePublicKeyDerHex:
          '302a300506032b6570032100d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a',
      updatePackageSourceUrl: 'https://updates.example.com/d4-pocket/stable',
      updatePackageSourceChannel: 'stable',
    );

    final trust = transport.payload?['update_trust'] as Map;
    expect(
      trust['public_key_der_hex'],
      startsWith('302a300506032b6570032100'),
    );
    expect(trust['package_sources'], [
      {
        'channel': 'stable',
        'base_url': 'https://updates.example.com/d4-pocket/stable',
      },
    ]);
    expect(trust.containsKey('private_key'), isFalse);
  });

  test('authority継承を示すReceiptを拒否する', () async {
    final transport = _FakeExportTransport((payload) {
      final response = exportBrokerResponse(payload);
      (response['body']! as Map<String, Object?>)['permission_inherited'] =
          true;
      return response;
    });

    await expectLater(
      ExportClient(transport).export(
        exportId: 'export-settings',
        composeManifest: _composeManifest(),
      ),
      throwsA(isA<BrokerClientException>()),
    );
  });

  test('Export ID、監査参照、Manifest file境界の不一致を個別に拒否する', () async {
    for (final corruptReceipt in <void Function(Map<String, Object?>)>[
      (body) => body['export_id'] = 'different-export',
      (body) => body['audit_id'] = 'different-audit',
      (body) => (body['manifest_file']! as Map<String, Object?>)['path'] =
          r'C:\D4Pocket\Exports\other.json',
    ]) {
      final transport = _FakeExportTransport((payload) {
        final response = exportBrokerResponse(payload);
        corruptReceipt(response['body']! as Map<String, Object?>);
        return response;
      });

      await expectLater(
        ExportClient(transport).export(
          exportId: 'export-settings',
          composeManifest: _composeManifest(),
        ),
        throwsA(isA<BrokerClientException>()),
      );
    }
  });

  test('Export targetとModule依存Closureの不一致を拒否する', () async {
    for (final corruptReceipt in <void Function(Map<String, Object?>)>[
      (body) {
        final manifest = body['export_manifest']! as Map<String, Object?>;
        (manifest['app_identity']! as Map<String, Object?>)['target_platform'] =
            'macos';
      },
      (body) {
        final manifest = body['export_manifest']! as Map<String, Object?>;
        final plan = manifest['module_plan']! as Map<String, Object?>;
        (plan['included_module_ids']! as List).remove('shell.observability');
      },
    ]) {
      final transport = _FakeExportTransport((payload) {
        final response = exportBrokerResponse(payload);
        corruptReceipt(response['body']! as Map<String, Object?>);
        return response;
      });

      await expectLater(
        ExportClient(transport).export(
          exportId: 'export-settings',
          composeManifest: _composeManifest(),
          targetPlatform: 'linux',
          optionalModuleIds: const ['shell.trace_inspector'],
        ),
        throwsA(isA<BrokerClientException>()),
      );
    }
  });

  test('GUI Shell書出しの拒否を成功へ昇格せず自由文も露出しない', () async {
    const privateDetail = 'fixture-secret-credential-detail';
    final transport = _FakeExportTransport(
      (_) => {
        'operation': 'GUI Shell書出し',
        'status': 'rejected',
        'error': {
          'code': 'owner_required',
          'message': privateDetail,
        },
      },
    );

    await expectLater(
      ExportClient(transport).export(
        exportId: 'export-settings',
        composeManifest: _composeManifest(),
      ),
      throwsA(
        isA<BrokerClientException>().having(
          (error) => error.message,
          'message',
          allOf(
            contains('Owner確認が完了しなかった'),
            isNot(contains(privateDetail)),
          ),
        ),
      ),
    );
  });
}

Map<String, Object?> _composeManifest() => {
      'version': 1,
      'compose_id': 'compose-settings',
      'display_name': 'D4 Pocket Fixture',
      'runtime_ids': ['runtime-fixture'],
      'agent_ids': ['agent-fixture'],
      'tool_ids': <String>[],
      'mcp_connection_ids': <String>[],
      'theme': {'theme_id': 'd4-pocket', 'mode': 'system'},
      'capability_requirements': <String>[],
      'settings': {
        'locale': 'ja-JP',
        'density': 'comfortable',
        'content_visibility': 'summary',
      },
      'inheritance_policy': const {
        'authority': 'none',
        'permission': 'none',
        'approval': 'none',
        'credential': 'none',
        'audit_chain': 'none',
      },
      'output_mode': 'manifest_only',
    };

class _FakeExportTransport implements BrokerTransport {
  _FakeExportTransport([this.responseBuilder]);

  final Map<String, Object?> Function(Map<String, Object?> payload)?
      responseBuilder;
  String? operation;
  Map<String, Object?>? payload;

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    this.operation = operation;
    this.payload = payload;
    final requestPayload = payload ?? const <String, Object?>{};
    return responseBuilder?.call(requestPayload) ??
        exportBrokerResponse(requestPayload);
  }
}
