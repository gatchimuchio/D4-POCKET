import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/services/broker_client.dart';

void main() {
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
}
