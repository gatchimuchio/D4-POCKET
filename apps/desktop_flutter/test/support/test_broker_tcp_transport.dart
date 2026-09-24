import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:gui_shell_desktop/services/broker_client.dart'
    show brokerPayloadHashForTest;
import 'package:gui_shell_ui/runtime_dialogue_client.dart' show BrokerTransport;

class TestBrokerTransportException implements Exception {
  const TestBrokerTransportException();
}

class TestBrokerTcpTransport implements BrokerTransport {
  TestBrokerTcpTransport._(this._endpoint);

  final _TestEndpoint _endpoint;
  int _counter = 0;

  static Future<TestBrokerTcpTransport> connect(String sessionFile) async {
    final decoded = jsonDecode(await File(sessionFile).readAsString());
    if (decoded is! Map) {
      throw const TestBrokerTransportException();
    }
    return TestBrokerTcpTransport._(_TestEndpoint.fromJson(
      Map<String, Object?>.from(decoded),
    ));
  }

  @override
  Future<Map<String, Object?>> request(String operation,
      {Map<String, Object?>? payload}) async {
    _counter += 1;
    final now = DateTime.now().toUtc();
    final request = <String, Object?>{
      'request_id': 'flutter-test-request-$_counter',
      'session_id': operation == 'health' ? null : _endpoint.sessionId,
      'operation': operation,
      'payload_hash': brokerPayloadHashForTest(payload),
      'nonce':
          'flutter-test-nonce-${DateTime.now().microsecondsSinceEpoch}-$_counter',
      'issued_at':
          '${now.year.toString().padLeft(4, '0')}-${now.month.toString().padLeft(2, '0')}-${now.day.toString().padLeft(2, '0')}T${now.hour.toString().padLeft(2, '0')}:${now.minute.toString().padLeft(2, '0')}:${now.second.toString().padLeft(2, '0')}Z',
      'metadata': {'client': 'desktop_flutter_test'},
      if (payload != null) 'payload': payload,
    };
    final socket = await Socket.connect(
        InternetAddress.loopbackIPv4, _endpoint.port,
        timeout: const Duration(seconds: 5));
    try {
      socket.write('${_endpoint.secret}\n${jsonEncode(request)}\n');
      await socket.flush();
      final line = await _readResponseLine(socket);
      final decoded = jsonDecode(utf8.decode(line));
      if (decoded is! Map ||
          decoded['request_id'] != request['request_id'] ||
          decoded['operation'] != operation) {
        throw const TestBrokerTransportException();
      }
      return Map<String, Object?>.from(decoded);
    } finally {
      socket.destroy();
    }
  }
}

class _TestEndpoint {
  const _TestEndpoint(this.port, this.sessionId, this.secret);

  final int port;
  final String sessionId;
  final String secret;

  factory _TestEndpoint.fromJson(Map<String, Object?> json) {
    const fields = {
      'host',
      'port',
      'session_id',
      'session_secret',
      'credential_role',
      'transport',
      'max_request_bytes',
    };
    if (json.length != fields.length ||
        !json.keys.every(fields.contains) ||
        json['host'] != '127.0.0.1' ||
        json['port'] is! int ||
        (json['port']! as int) < 1 ||
        (json['port']! as int) > 65535 ||
        json['credential_role'] != 'normal' ||
        json['transport'] != 'authenticated_loopback_tcp' ||
        json['session_id'] is! String ||
        json['session_secret'] is! String ||
        !RegExp(r'^[a-f0-9]{64}$')
            .hasMatch(json['session_secret']! as String) ||
        json['max_request_bytes'] is! int ||
        (json['max_request_bytes']! as int) < 1) {
      throw const TestBrokerTransportException();
    }
    return _TestEndpoint(
      json['port']! as int,
      json['session_id']! as String,
      json['session_secret']! as String,
    );
  }
}

Future<List<int>> _readResponseLine(Socket socket) async {
  const maxBytes = 4 * 1024 * 1024;
  final completer = Completer<List<int>>();
  final bytes = <int>[];
  late final StreamSubscription<List<int>> subscription;
  final timer = Timer(const Duration(seconds: 5), () {
    if (!completer.isCompleted) {
      completer.completeError(const TestBrokerTransportException());
      unawaited(subscription.cancel());
    }
  });
  subscription = socket.listen((chunk) {
    if (completer.isCompleted) return;
    final newline = chunk.indexOf(0x0a);
    final count = newline < 0 ? chunk.length : newline;
    if (bytes.length + count > maxBytes) {
      completer.completeError(const TestBrokerTransportException());
    } else {
      bytes.addAll(chunk.take(count));
      if (newline >= 0) {
        if (newline + 1 != chunk.length) {
          completer.completeError(const TestBrokerTransportException());
        } else {
          if (bytes.isNotEmpty && bytes.last == 0x0d) bytes.removeLast();
          completer.complete(bytes);
        }
      }
    }
    if (completer.isCompleted) unawaited(subscription.cancel());
  }, onError: (Object error, StackTrace stack) {
    if (!completer.isCompleted) completer.completeError(error, stack);
  }, onDone: () {
    if (!completer.isCompleted) {
      completer.completeError(const TestBrokerTransportException());
    }
  });
  try {
    return await completer.future;
  } finally {
    timer.cancel();
    await subscription.cancel();
  }
}
