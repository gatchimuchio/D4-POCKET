import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/screens/mcp_connection_center.dart';
import 'package:gui_shell_desktop/services/mcp_connection_client.dart';
import 'package:gui_shell_desktop/services/broker_client.dart';

class _Transport implements BrokerTransport {
  final payloads = <Map<String, Object?>?>[];
  Map<String, Object?>? receipt;
  bool inject = false;
  @override
  Future<Map<String, Object?>> request(String operation,
      {Map<String, Object?>? payload}) async {
    payloads.add(payload);
    if (operation == '資格情報登録') {
      receipt = {
        '版': 1,
        '資格情報ID': payload!['資格情報ID'],
        '用途': 'mcp_transport',
        '接続対象': payload['接続対象'],
        '種類': 'api_key',
        '保管方式': 'macos_keychain',
        '状態': '有効',
        '作成時刻UnixMillis': 1000,
        '最終使用時刻UnixMillis': null,
        '失効時刻UnixMillis': null,
        '暗号文hash': 'sha256:${'a' * 64}',
        '作成監査ID': 'audit-created',
        '公開範囲': 'metadata_only',
        '証拠種別': 'INTERNAL_STATE',
        if (inject) '秘密値': 'forbidden-extra'
      };
    }
    if (operation == '資格情報失効') {
      receipt!['状態'] = '失効';
      receipt!['失効時刻UnixMillis'] = 1001;
    }
    return {
      'request_id': 'test',
      'operation': operation,
      'status': 'accepted',
      'evidence_source': 'INTERNAL_STATE',
      'audit_event_id': 'audit-test',
      'error': null,
      'body': operation == '資格情報一覧'
          ? {
              '版': 1,
              '資格情報一覧': [if (receipt != null) receipt],
              '件数': receipt == null ? 0 : 1,
              '公開範囲': 'metadata_only',
              '証拠種別': 'INTERNAL_STATE'
            }
          : receipt
    };
  }
}

void main() {
  test('Mac native登録は公開metadataだけを要求しstorage束縛を保つ', () async {
    final transport = _Transport();
    final client = McpConnectionClient(transport);
    await client.registerMacNativeCredential(
        credentialId: 'a' * 32, targetServerId: 'test-server');
    expect(transport.payloads.first!.keys.toSet(),
        {'版', '資格情報ID', '用途', '接続対象', '種類'});
    final entry =
        (await client.listCredentials(targetServerId: 'test-server')).single;
    expect(entry.storage, 'macos_keychain');
    final revoked = await client.revokeCredential(credential: entry);
    expect(revoked.status, '失効');
    expect(revoked.storage, 'macos_keychain');
    expect(brokerRequestTimeoutForOperation('資格情報登録'),
        const Duration(seconds: 610));
  });
  test('native登録receiptへ秘密fieldを混ぜたら投影を拒否する', () async {
    final transport = _Transport()..inject = true;
    await expectLater(
        McpConnectionClient(transport).registerMacNativeCredential(
            credentialId: 'a' * 32, targetServerId: 'test-server'),
        throwsA(isA<BrokerClientException>()));
  });
  testWidgets('Mac画面はnative入力を起動し秘密入力widgetを作らない', (tester) async {
    debugDefaultTargetPlatformOverride = TargetPlatform.macOS;
    try {
      final transport = _Transport();
      await tester.pumpWidget(MaterialApp(
          home:
              Scaffold(body: McpConnectionCenterPanel(transport: transport))));
      await tester.enterText(find.byType(TextField).first, 'test-server');
      final button =
          find.byKey(const ValueKey('macos-native-credential-register'));
      await tester.ensureVisible(button);
      await tester.tap(button);
      await tester.pumpAndSettle();
      expect(transport.payloads.first!.keys.toSet(),
          {'版', '資格情報ID', '用途', '接続対象', '種類'});
      expect(find.textContaining('Keychain登録後のmetadata'), findsOneWidget);
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
