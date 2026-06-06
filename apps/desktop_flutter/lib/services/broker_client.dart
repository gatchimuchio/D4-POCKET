import 'dart:collection';
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
      'payload_hash': _payloadHash(payload),
      'nonce':
          'flutter-nonce-${DateTime.now().microsecondsSinceEpoch}-$_counter',
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

String brokerPayloadHashForTest(Map<String, Object?>? payload) =>
    _payloadHash(payload);

String _payloadHash(Map<String, Object?>? payload) {
  final canonicalJson = jsonEncode(_canonicalizeJsonValue(payload));
  return _sha256Tagged(utf8.encode(canonicalJson));
}

Object? _canonicalizeJsonValue(Object? value) {
  if (value is Map) {
    final sorted = SplayTreeMap<String, Object?>();
    for (final entry in value.entries) {
      sorted[entry.key.toString()] = _canonicalizeJsonValue(entry.value);
    }
    return sorted;
  }
  if (value is Iterable && value is! String) {
    return value.map(_canonicalizeJsonValue).toList(growable: false);
  }
  return value;
}

String _sha256Tagged(List<int> bytes) {
  final digest = _sha256(bytes);
  final hex = StringBuffer('sha256:');
  for (final byte in digest) {
    hex.write(byte.toRadixString(16).padLeft(2, '0'));
  }
  return hex.toString();
}

List<int> _sha256(List<int> input) {
  const mask = 0xffffffff;
  const k = <int>[
    0x428a2f98,
    0x71374491,
    0xb5c0fbcf,
    0xe9b5dba5,
    0x3956c25b,
    0x59f111f1,
    0x923f82a4,
    0xab1c5ed5,
    0xd807aa98,
    0x12835b01,
    0x243185be,
    0x550c7dc3,
    0x72be5d74,
    0x80deb1fe,
    0x9bdc06a7,
    0xc19bf174,
    0xe49b69c1,
    0xefbe4786,
    0x0fc19dc6,
    0x240ca1cc,
    0x2de92c6f,
    0x4a7484aa,
    0x5cb0a9dc,
    0x76f988da,
    0x983e5152,
    0xa831c66d,
    0xb00327c8,
    0xbf597fc7,
    0xc6e00bf3,
    0xd5a79147,
    0x06ca6351,
    0x14292967,
    0x27b70a85,
    0x2e1b2138,
    0x4d2c6dfc,
    0x53380d13,
    0x650a7354,
    0x766a0abb,
    0x81c2c92e,
    0x92722c85,
    0xa2bfe8a1,
    0xa81a664b,
    0xc24b8b70,
    0xc76c51a3,
    0xd192e819,
    0xd6990624,
    0xf40e3585,
    0x106aa070,
    0x19a4c116,
    0x1e376c08,
    0x2748774c,
    0x34b0bcb5,
    0x391c0cb3,
    0x4ed8aa4a,
    0x5b9cca4f,
    0x682e6ff3,
    0x748f82ee,
    0x78a5636f,
    0x84c87814,
    0x8cc70208,
    0x90befffa,
    0xa4506ceb,
    0xbef9a3f7,
    0xc67178f2,
  ];

  final message = List<int>.from(input);
  final bitLength = message.length * 8;
  message.add(0x80);
  while ((message.length % 64) != 56) {
    message.add(0);
  }
  for (var shift = 56; shift >= 0; shift -= 8) {
    message.add((bitLength >> shift) & 0xff);
  }

  var h0 = 0x6a09e667;
  var h1 = 0xbb67ae85;
  var h2 = 0x3c6ef372;
  var h3 = 0xa54ff53a;
  var h4 = 0x510e527f;
  var h5 = 0x9b05688c;
  var h6 = 0x1f83d9ab;
  var h7 = 0x5be0cd19;

  for (var chunk = 0; chunk < message.length; chunk += 64) {
    final w = List<int>.filled(64, 0);
    for (var index = 0; index < 16; index += 1) {
      final offset = chunk + (index * 4);
      w[index] = ((message[offset] << 24) |
              (message[offset + 1] << 16) |
              (message[offset + 2] << 8) |
              message[offset + 3]) &
          mask;
    }
    for (var index = 16; index < 64; index += 1) {
      final s0 = _rotr(w[index - 15], 7) ^
          _rotr(w[index - 15], 18) ^
          (w[index - 15] >> 3);
      final s1 = _rotr(w[index - 2], 17) ^
          _rotr(w[index - 2], 19) ^
          (w[index - 2] >> 10);
      w[index] = (w[index - 16] + s0 + w[index - 7] + s1) & mask;
    }

    var a = h0;
    var b = h1;
    var c = h2;
    var d = h3;
    var e = h4;
    var f = h5;
    var g = h6;
    var h = h7;

    for (var index = 0; index < 64; index += 1) {
      final s1 = _rotr(e, 6) ^ _rotr(e, 11) ^ _rotr(e, 25);
      final ch = (e & f) ^ (((~e) & mask) & g);
      final temp1 = (h + s1 + ch + k[index] + w[index]) & mask;
      final s0 = _rotr(a, 2) ^ _rotr(a, 13) ^ _rotr(a, 22);
      final maj = (a & b) ^ (a & c) ^ (b & c);
      final temp2 = (s0 + maj) & mask;
      h = g;
      g = f;
      f = e;
      e = (d + temp1) & mask;
      d = c;
      c = b;
      b = a;
      a = (temp1 + temp2) & mask;
    }

    h0 = (h0 + a) & mask;
    h1 = (h1 + b) & mask;
    h2 = (h2 + c) & mask;
    h3 = (h3 + d) & mask;
    h4 = (h4 + e) & mask;
    h5 = (h5 + f) & mask;
    h6 = (h6 + g) & mask;
    h7 = (h7 + h) & mask;
  }

  final digest = <int>[];
  for (final word in [h0, h1, h2, h3, h4, h5, h6, h7]) {
    digest.add((word >> 24) & 0xff);
    digest.add((word >> 16) & 0xff);
    digest.add((word >> 8) & 0xff);
    digest.add(word & 0xff);
  }
  return digest;
}

int _rotr(int value, int bits) {
  return ((value >> bits) | (value << (32 - bits))) & 0xffffffff;
}
