import 'dart:async';
import 'dart:convert';
import 'dart:io';

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

  test('Broker応答はsocketを維持しても最初のJSON行で完結する', () async {
    final root =
        await Directory.systemTemp.createTemp('gui-shell-broker-client-');
    final server = await ServerSocket.bind(InternetAddress.loopbackIPv4, 0);
    final releaseConnection = Completer<void>();
    final endpoint = File('${root.path}${Platform.pathSeparator}endpoint.json');
    final serverTask = () async {
      final socket = await server.first;
      try {
        final lines = await socket
            .cast<List<int>>()
            .transform(utf8.decoder)
            .transform(const LineSplitter())
            .take(2)
            .toList();
        final request = _requestFromLines(lines);
        socket.write('${jsonEncode({
              'request_id': request['request_id'],
              'operation': request['operation'],
              'status': 'accepted',
              'evidence_source': 'INTERNAL_STATE',
              'audit_event_id': 'broker-audit-test',
              'error': null,
              'health': null,
              'body': <String, Object?>{},
              'shutdown_requested': false,
            })}\n');
        await socket.flush();
        await releaseConnection.future;
      } finally {
        await socket.close();
      }
    }();

    try {
      await endpoint.writeAsString(
        jsonEncode({
          'host': '127.0.0.1',
          'port': server.port,
          'session_id': 'broker-client-test-session',
          'session_secret':
              'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
          'credential_role': 'normal',
          'transport': 'authenticated_loopback_tcp',
          'max_request_bytes': 1024,
        }),
      );
      final client = await BrokerClient.connect(sessionFile: endpoint.path);
      final response =
          await client.request('health').timeout(const Duration(seconds: 2));

      expect(response['status'], 'accepted');
      expect(response['operation'], 'health');
    } finally {
      if (!releaseConnection.isCompleted) {
        releaseConnection.complete();
      }
      await server.close();
      await serverTask.timeout(const Duration(seconds: 2));
      await root.delete(recursive: true);
    }
  });
}

Map<String, dynamic> _requestFromLines(List<String> lines) {
  if (lines.length != 2) {
    throw StateError('通常IPC接続が資格と要求を送信しませんでした。');
  }
  final request = jsonDecode(lines[1]);
  if (request is! Map<String, dynamic>) {
    throw StateError('通常IPC要求がJSONオブジェクトではありません。');
  }
  return request;
}
