import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:flutter/services.dart';
import 'package:gui_shell_desktop/services/broker_client.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('Agent Task grantはnative Owner確認の応答待ち時間を使う', () {
    expect(
      brokerRequestTimeoutForOperation('AgentTaskWorkspacePermissionGrant'),
      const Duration(seconds: 305),
    );
    expect(
      brokerRequestTimeoutForOperation('AgentTaskOwnerApprovalGrant'),
      const Duration(seconds: 305),
    );
    expect(
      brokerRequestTimeoutForOperation('MCP Tool実行'),
      const Duration(seconds: 305),
    );
    expect(
      brokerRequestTimeoutForOperation('AgentTask実行'),
      const Duration(seconds: 5),
    );
  });

  test('payload_hash matches the Rust broker null payload vector', () {
    expect(
      brokerPayloadHashForTest(null),
      'sha256:74234e98afe7498fb5daf1f36ac2d78acc339464f950703b8c019892f982b90b',
    );
  });

  test('payload_hash canonicalizes object key order', () {
    const expected =
        'sha256:d3626ac30a87e6f7a6428233b3c68299976865fa5508e4267c5415c76af7a772';

    expect(brokerPayloadHashForTest({'b': 1, 'a': 2}), expected);
    expect(brokerPayloadHashForTest({'a': 2, 'b': 1}), expected);
  });

  test('payload_hash canonicalizes nested payloads', () {
    final payload = <String, Object?>{
      'z': [
        {'b': 1, 'a': 2},
        null,
        true,
      ],
      'a': {
        'd': 'text',
        'c': [3, 2, 1],
      },
    };

    expect(
      brokerPayloadHashForTest(payload),
      'sha256:8895d6e5b558a29b870d1156bfb1e95fcbab9933f2360c35edaa78d734c8c87a',
    );
  });

  test('payload_hash matches Rust broker normalize payload vector', () {
    expect(
      brokerPayloadHashForTest({
        'client_payload': 'desktop_flutter_authority_probe',
      }),
      'sha256:787a213a62a6dd88756a81d1b68234f88759d36308adc933625aa48a4507a93b',
    );
  });

  test('Dartは資格を含めずRunner channelへ要求JSONだけを渡す', () async {
    const channel = MethodChannel('gui_shell/broker');
    final messenger =
        TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
    messenger.setMockMethodCallHandler(channel, (call) async {
      expect(call.method, 'request');
      expect(call.arguments, isA<String>());
      final request =
          jsonDecode(call.arguments! as String) as Map<String, Object?>;
      expect(
          request.keys,
          containsAll([
            'request_id',
            'operation',
            'payload_hash',
            'nonce',
            'issued_at',
            'metadata',
            'payload',
          ]));
      expect(request.keys, isNot(contains('session_id')));
      expect(request.keys, isNot(contains('session_secret')));
      expect(request.keys, isNot(contains('credential_role')));
      return jsonEncode({
        'request_id': request['request_id'],
        'operation': request['operation'],
        'status': 'accepted',
        'evidence_source': 'INTERNAL_STATE',
        'audit_event_id': 'broker-audit-test',
        'error': null,
        'health': null,
        'body': <String, Object?>{},
        'shutdown_requested': false,
      });
    });
    try {
      final client = await BrokerClient.connect();
      final response = await client.request('health', payload: {'version': 1});
      expect(response['status'], 'accepted');
      expect(response['operation'], 'health');
    } finally {
      messenger.setMockMethodCallHandler(channel, null);
    }
  });
}
