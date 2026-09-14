import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/services/broker_client.dart';

void main() {
  test('broker接続資格は通常資格だけを厳格に受理する', () {
    final valid = <String, Object?>{
      'host': '127.0.0.1',
      'port': 1234,
      'session_id': 'session',
      'session_secret': 'a' * 64,
      'credential_role': 'normal',
      'transport': 'authenticated_loopback_tcp',
      'max_request_bytes': 65536
    };
    expect(BrokerEndpoint.fromJson(valid).port, 1234);
    for (final change in [
      {'host': 'example.invalid'},
      {'host': '0.0.0.0'},
      {'port': 0},
      {'credential_role': 'owner'},
      {'credential_role': 'unknown'},
      {'session_secret': '${'a' * 64}\n'}
    ]) {
      expect(() => BrokerEndpoint.fromJson({...valid, ...change}),
          throwsA(isA<BrokerClientException>()));
    }
    final missingRole = Map<String, Object?>.from(valid)
      ..remove('credential_role');
    expect(() => BrokerEndpoint.fromJson(missingRole),
        throwsA(isA<BrokerClientException>()));
    expect(
      () => BrokerEndpoint.fromJson({...valid, 'unexpected': true}),
      throwsA(isA<BrokerClientException>()),
    );
  });
}
