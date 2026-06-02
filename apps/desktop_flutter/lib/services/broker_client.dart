import 'dart:convert';
import 'dart:io';

abstract class BrokerTransport {
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  });
}

class BrokerClient implements BrokerTransport {
  BrokerClient._(this._endpoint);

  final BrokerEndpoint _endpoint;
  int _counter = 0;

  static Future<BrokerClient> connect({
    String? sessionFile,
  }) async {
    final resolvedSession = sessionFile ??
        Platform.environment['GUI_SHELL_BROKER_ENDPOINT_JSON'] ??
        Platform.environment['GUI_SHELL_BROKER_SESSION_JSON'] ??
        _candidateSessionFilePath();
    if (resolvedSession == null || resolvedSession.isEmpty) {
      throw const BrokerClientException('broker endpoint file not configured');
    }
    final file = File(resolvedSession);
    if (!file.existsSync()) {
      throw BrokerClientException(
        'broker endpoint file not found: $resolvedSession',
      );
    }
    return BrokerClient._(
      BrokerEndpoint.fromJson(_readJsonFile(resolvedSession)),
    );
  }

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    _counter += 1;
    final issuedAt = _rfc3339Seconds(DateTime.now().toUtc());
    final request = <String, Object?>{
      'request_id': 'flutter-request-$_counter',
      'session_id': operation == 'health' ? null : _endpoint.sessionId,
      'operation': operation,
      'payload_hash':
          'sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
      'nonce': 'flutter-nonce-${DateTime.now().microsecondsSinceEpoch}-$_counter',
      'issued_at': issuedAt,
      'metadata': {'client': 'desktop_flutter'},
      if (payload != null) 'payload': payload,
    };

    final socket = await Socket.connect(
      _endpoint.host,
      _endpoint.port,
      timeout: const Duration(seconds: 5),
    );
    try {
      socket.write('${_endpoint.sessionSecret}\n');
      socket.write('${jsonEncode(request)}\n');
      await socket.flush();
      final raw = await utf8.decoder
          .bind(socket)
          .join()
          .timeout(const Duration(seconds: 5));
      final lines = raw.trim().split('\n').where((item) => item.isNotEmpty);
      if (lines.isEmpty) {
        throw const BrokerClientException('broker response was empty');
      }
      final line = lines.last;
      final decoded = jsonDecode(line);
      if (decoded is! Map) {
        throw const BrokerClientException('broker response was not an object');
      }
      return Map<String, Object?>.from(decoded);
    } finally {
      socket.destroy();
    }
  }

  BrokerEndpoint get endpoint => _endpoint;
}

class BrokerEndpoint {
  const BrokerEndpoint({
    required this.host,
    required this.port,
    required this.sessionId,
    required this.sessionSecret,
    required this.transport,
    required this.maxRequestBytes,
  });

  final String host;
  final int port;
  final String sessionId;
  final String sessionSecret;
  final String transport;
  final int maxRequestBytes;

  factory BrokerEndpoint.fromJson(Map<String, Object?> json) {
    return BrokerEndpoint(
      host: json['host'] as String? ?? '127.0.0.1',
      port: json['port'] as int? ?? 0,
      sessionId: json['session_id'] as String? ?? '',
      sessionSecret: json['session_secret'] as String? ?? '',
      transport: json['transport'] as String? ?? 'authenticated_loopback_tcp',
      maxRequestBytes: json['max_request_bytes'] as int? ?? 0,
    );
  }
}

class BrokerClientException implements Exception {
  const BrokerClientException(this.message);

  final String message;

  @override
  String toString() => message;
}

Map<String, Object?> _readJsonFile(String path) {
  final decoded = jsonDecode(File(path).readAsStringSync());
  if (decoded is! Map) {
    throw const BrokerClientException('broker endpoint file was not an object');
  }
  return Map<String, Object?>.from(decoded);
}

Directory _brokerRuntimeRoot() {
  final override = Platform.environment['GUI_SHELL_BROKER_RUNTIME_DIR'];
  if (override != null && override.isNotEmpty) {
    return Directory(override)..createSync(recursive: true);
  }
  if (Platform.isWindows) {
    final localAppData = Platform.environment['LOCALAPPDATA'];
    if (localAppData != null && localAppData.isNotEmpty) {
      return Directory('$localAppData\\GUI-Shell\\broker')
        ..createSync(recursive: true);
    }
  }
  return Directory('.gui_shell/broker')..createSync(recursive: true);
}

String? _candidateSessionFilePath() {
  final root = _brokerRuntimeRoot();
  final runtimeSession =
      '${root.path}${Platform.pathSeparator}broker_session.json';
  if (File(runtimeSession).existsSync()) {
    return runtimeSession;
  }
  final executableDir = File(Platform.resolvedExecutable).parent.path;
  final candidates = <String>[
    '$executableDir${Platform.pathSeparator}broker_session.json',
    '.gui_shell/broker/broker_session.json',
    '.gui-shell/broker/broker_session.json',
  ];
  for (final candidate in candidates) {
    if (File(candidate).existsSync()) {
      return candidate;
    }
  }
  return null;
}

String _rfc3339Seconds(DateTime value) {
  final year = value.year.toString().padLeft(4, '0');
  final month = value.month.toString().padLeft(2, '0');
  final day = value.day.toString().padLeft(2, '0');
  final hour = value.hour.toString().padLeft(2, '0');
  final minute = value.minute.toString().padLeft(2, '0');
  final second = value.second.toString().padLeft(2, '0');
  return '$year-$month-${day}T$hour:$minute:${second}Z';
}
