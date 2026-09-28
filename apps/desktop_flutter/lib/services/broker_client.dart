import 'dart:async';
import 'dart:collection';
import 'dart:convert';

import 'package:flutter/services.dart';

import 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerTransport, BrokerClientException;
export 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerTransport, BrokerClientException;

class BrokerClient implements BrokerTransport {
  BrokerClient._(this._channel);

  final MethodChannel _channel;
  int _counter = 0;

  static Future<BrokerClient> connect({MethodChannel? channel}) async =>
      BrokerClient._(channel ?? const MethodChannel('gui_shell/broker'));

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    _counter += 1;
    final issuedAt = _rfc3339Seconds(DateTime.now().toUtc());
    final request = <String, Object?>{
      'request_id': 'flutter-request-$_counter',
      'operation': operation,
      'payload_hash': _payloadHash(payload),
      'nonce':
          'flutter-nonce-${DateTime.now().microsecondsSinceEpoch}-$_counter',
      'issued_at': issuedAt,
      'metadata': {'client': 'desktop_flutter'},
      if (payload != null) 'payload': payload,
    };
    final String responseText;
    try {
      final ownerConfirmationOperation = const {
        'GUI Shell書出し',
        '回帰Case登録',
        '回帰Case削除',
        '回帰Case削除中断確認',
        'MCP切断',
      }.contains(operation);
      final response = await _channel
          .invokeMethod<String>('request', jsonEncode(request))
          .timeout(ownerConfirmationOperation
              ? const Duration(seconds: 305)
              : const Duration(seconds: 5));
      if (response == null || utf8.encode(response).length > 4 * 1024 * 1024) {
        throw const BrokerClientException('broker応答が空または上限超過です');
      }
      responseText = response;
    } on PlatformException {
      throw const BrokerClientException('安全Brokerとの通信に失敗しました');
    } on MissingPluginException {
      throw const BrokerClientException('安全Brokerとの通信路が利用できません');
    } on TimeoutException {
      throw const BrokerClientException('安全Brokerの応答が期限を超過しました');
    }

    final Object? decoded;
    try {
      decoded = jsonDecode(responseText);
    } on FormatException {
      throw const BrokerClientException('broker応答のJSONが不正です');
    }
    if (decoded is! Map) {
      throw const BrokerClientException('broker応答がobjectではありません');
    }
    if (decoded['request_id'] != request['request_id'] ||
        decoded['operation'] != operation) {
      throw const BrokerClientException('broker応答が現在の要求と一致しません');
    }
    return Map<String, Object?>.from(decoded);
  }
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

String brokerPayloadHash(Map<String, Object?>? payload) =>
    _payloadHash(payload);

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
