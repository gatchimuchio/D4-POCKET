import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/screens/mcp_connection_center.dart';
import 'package:gui_shell_desktop/services/broker_client.dart';
import 'package:gui_shell_desktop/services/mcp_connection_client.dart';

class _LinuxCredentialTransport implements BrokerTransport {
  final payloads = <Map<String, Object?>?>[];
  Map<String, Object?>? entry;
  String? receiptPurposeOverride;

  @override
  Future<Map<String, Object?>> request(String operation,
      {Map<String, Object?>? payload}) async {
    payloads.add(payload);
    if (operation == '資格情報登録') {
      entry = {
        '版': 1,
        '資格情報ID': payload!['資格情報ID'],
        '用途': receiptPurposeOverride ?? payload['用途'],
        '接続対象': payload['接続対象'],
        '種類': 'api_key',
        '保管方式': 'linux_secret_service',
        '状態': '有効',
        '作成時刻UnixMillis': 1000,
        '最終使用時刻UnixMillis': null,
        '失効時刻UnixMillis': null,
        '暗号文hash': 'sha256:${'a' * 64}',
        '作成監査ID': 'audit-linux-created',
        '公開範囲': 'metadata_only',
        '証拠種別': 'INTERNAL_STATE',
      };
    } else if (operation == '資格情報失効') {
      entry!['状態'] = '失効';
      entry!['失効時刻UnixMillis'] = 1001;
    }
    return {
      'request_id': 'linux-test',
      'operation': operation,
      'status': 'accepted',
      'evidence_source': 'INTERNAL_STATE',
      'audit_event_id': 'audit-linux-test',
      'error': null,
      'body': operation == '資格情報一覧'
          ? {
              '版': 1,
              '資格情報一覧': [if (entry != null) entry],
              '件数': entry == null ? 0 : 1,
              '公開範囲': 'metadata_only',
              '証拠種別': 'INTERNAL_STATE',
            }
          : entry,
    };
  }
}

void main() {
  test('Linux credential登録は公開metadataだけを要求しSecret Service storageを固定する',
      () async {
    final transport = _LinuxCredentialTransport();
    final client = McpConnectionClient(transport);
    await client.registerLinuxNativeCredential(
        credentialId: 'b' * 32, targetServerId: 'linux-server');
    expect(transport.payloads.first!.keys.toSet(),
        {'版', '資格情報ID', '用途', '接続対象', '種類'});
    final entry =
        (await client.listCredentials(targetServerId: 'linux-server')).single;
    expect(entry.storage, 'linux_secret_service');
    final revoked = await client.revokeCredential(credential: entry);
    expect(revoked.status, '失効');
    expect(revoked.storage, 'linux_secret_service');
  });

  test('Linux Provider Credential登録は固定対象と公開metadataだけをBrokerへ渡す', () async {
    debugDefaultTargetPlatformOverride = TargetPlatform.linux;
    addTearDown(() => debugDefaultTargetPlatformOverride = null);
    final transport = _LinuxCredentialTransport();
    final client = McpConnectionClient(transport);

    await client.registerLinuxNativeProviderCredential(credentialId: 'c' * 32);

    expect(transport.payloads.single, {
      '版': 1,
      '資格情報ID': 'c' * 32,
      '用途': 'provider_api_key',
      '接続対象': 'openai_codex_cli',
      '種類': 'api_key',
    });
    expect(
      (await client.listProviderCredentials()).single.credentialId,
      'c' * 32,
    );
    expect(transport.payloads.toString(), isNot(contains('秘密値')));
  });

  test('Provider Credential native registration is unavailable off Linux',
      () async {
    debugDefaultTargetPlatformOverride = TargetPlatform.windows;
    addTearDown(() => debugDefaultTargetPlatformOverride = null);
    final transport = _LinuxCredentialTransport();

    await expectLater(
      McpConnectionClient(transport)
          .registerLinuxNativeProviderCredential(credentialId: 'd' * 32),
      throwsA(isA<BrokerClientException>()),
    );
    expect(transport.payloads, isEmpty);
  });

  test('Linux Provider Credential registration rejects a mismatched receipt',
      () async {
    debugDefaultTargetPlatformOverride = TargetPlatform.linux;
    addTearDown(() => debugDefaultTargetPlatformOverride = null);
    final transport = _LinuxCredentialTransport()
      ..receiptPurposeOverride = 'mcp_transport';

    await expectLater(
      McpConnectionClient(transport)
          .registerLinuxNativeProviderCredential(credentialId: 'e' * 32),
      throwsA(isA<BrokerClientException>()),
    );
    expect(transport.payloads.single!['用途'], 'provider_api_key');
  });

  test(
      'Linux Provider Credential registration rejects malformed IDs before IPC',
      () async {
    debugDefaultTargetPlatformOverride = TargetPlatform.linux;
    addTearDown(() => debugDefaultTargetPlatformOverride = null);
    final transport = _LinuxCredentialTransport();

    await expectLater(
      McpConnectionClient(transport)
          .registerLinuxNativeProviderCredential(credentialId: 'invalid'),
      throwsA(isA<BrokerClientException>()),
    );
    expect(transport.payloads, isEmpty);
  });

  testWidgets('Linux MCP画面はGTK native入力を開始しFlutter秘密入力を作らない', (tester) async {
    debugDefaultTargetPlatformOverride = TargetPlatform.linux;
    try {
      final transport = _LinuxCredentialTransport();
      await tester.pumpWidget(MaterialApp(
          home:
              Scaffold(body: McpConnectionCenterPanel(transport: transport))));
      await tester.enterText(find.byType(TextField).first, 'linux-server');
      final button =
          find.byKey(const ValueKey('linux-native-credential-register'));
      await tester.ensureVisible(button);
      await tester.tap(button);
      await tester.pumpAndSettle();
      expect(transport.payloads.first!.keys.toSet(),
          {'版', '資格情報ID', '用途', '接続対象', '種類'});
      expect(find.textContaining('Linux Secret Service登録後のmetadata'),
          findsOneWidget);
      expect(find.byKey(const ValueKey('mcp-credential-selection')),
          findsOneWidget);
      expect(
          find.textContaining('Linux Desktop sandboxはありません'), findsOneWidget);
      expect(
          find
              .byType(TextField)
              .evaluate()
              .any((e) => (e.widget as TextField).obscureText),
          isFalse);
    } finally {
      debugDefaultTargetPlatformOverride = null;
    }
  });
}
