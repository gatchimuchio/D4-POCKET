import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:flutter/services.dart';
import 'package:gui_shell_desktop/services/broker_client.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('不一致なRunner応答はUIへ受け入れない', () async {
    const channel = MethodChannel('gui_shell/broker');
    final messenger =
        TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
    messenger.setMockMethodCallHandler(channel, (call) async {
      final request = jsonDecode(call.arguments! as String) as Map;
      return jsonEncode({
        'request_id': 'another-request',
        'operation': request['operation'],
      });
    });
    try {
      final client = await BrokerClient.connect();
      await expectLater(
        client.request('health'),
        throwsA(isA<BrokerClientException>()),
      );
    } finally {
      messenger.setMockMethodCallHandler(channel, null);
    }
  });
}
