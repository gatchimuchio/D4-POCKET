import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/services/broker_client.dart';

void main() {
  test('broker接続資格はloopback以外と改行付きsecretを拒否する', () {
    final valid = <String, Object?>{
      'host': '127.0.0.1',
      'port': 1234,
      'session_id': 'session',
      'session_secret': 'a' * 64,
      'transport': 'authenticated_loopback_tcp',
      'max_request_bytes': 65536
    };
    expect(BrokerEndpoint.fromJson(valid).port, 1234);
    for (final change in [
      {'host': 'example.invalid'},
      {'host': '0.0.0.0'},
      {'port': 0},
      {'session_secret': '${'a' * 64}\n'}
    ]) {
      expect(() => BrokerEndpoint.fromJson({...valid, ...change}),
          throwsA(isA<BrokerClientException>()));
    }
  });
}
