import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:math';

import 'package:crypto/crypto.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart';

const _invalid = BrokerClientException('端末連携のデータを確認できません。');
const _paused = BrokerClientException('接続確認が済むまで操作を停止しています。');

String newDeviceId() {
  final random = Random.secure();
  return List.generate(
    16,
    (_) => random.nextInt(256).toRadixString(16).padLeft(2, '0'),
  ).join();
}

/// 重複keyはjsonDecodeが最後の値を採用する前に拒否する。
Map<String, Object?> strictObject(String source, {int limit = 8192}) {
  if (utf8.encode(source).length > limit) throw _invalid;
  var offset = 0;
  void space() {
    while (offset < source.length && ' \r\n\t'.contains(source[offset])) {
      offset++;
    }
  }

  String string() {
    final start = offset++;
    while (offset < source.length) {
      final c = source[offset++];
      if (c == '\\') {
        offset++;
      } else if (c == '"') {
        return jsonDecode(source.substring(start, offset)) as String;
      }
    }
    throw _invalid;
  }

  Object? value(int depth) {
    space();
    if (depth > 32 || offset >= source.length) throw _invalid;
    final c = source[offset];
    if (c == '"') return string();
    if (c == '{' || c == '[') {
      offset++;
      final object = <String, Object?>{};
      final list = <Object?>[];
      final end = c == '{' ? '}' : ']';
      space();
      if (offset < source.length && source[offset] == end) {
        offset++;
        return c == '{' ? object : list;
      }
      while (offset < source.length) {
        if (c == '{') {
          space();
          if (offset >= source.length || source[offset] != '"') throw _invalid;
          final key = string();
          space();
          if (object.containsKey(key) ||
              offset >= source.length ||
              source[offset++] != ':')
            throw _invalid;
          object[key] = value(depth + 1);
        } else {
          list.add(value(depth + 1));
        }
        space();
        if (offset >= source.length) throw _invalid;
        final separator = source[offset++];
        if (separator == end) return c == '{' ? object : list;
        if (separator != ',') throw _invalid;
      }
      throw _invalid;
    }
    final start = offset;
    while (offset < source.length && !',]} \r\n\t'.contains(source[offset])) {
      offset++;
    }
    if (start == offset) throw _invalid;
    return jsonDecode(source.substring(start, offset));
  }

  try {
    final result = value(0);
    space();
    if (offset != source.length || result is! Map<String, Object?>)
      throw _invalid;
    return result;
  } catch (_) {
    throw _invalid;
  }
}

/// 到達先は招待が固定するprivate IPv4だけ。URL・DNS・任意runtime指定を許容しない。
class DeviceCredential {
  DeviceCredential._(this.data, this.invitation);
  final Map<String, Object?> data;
  final bool invitation;
  String text(String key) => data[key] as String;
  int get port => data['port'] as int;
  int get expiry => data['有効期限'] as int;
  String get id => text(invitation ? '招待ID' : '結合ID');
  String get secret => text(invitation ? '招待秘密' : '端末秘密');
  bool get expired => expiry <= DateTime.now().millisecondsSinceEpoch ~/ 1000;

  factory DeviceCredential.parse(
    String source, {
    required bool invitation,
    required String deviceId,
  }) {
    final data = strictObject(source);
    final keys = {
      '版',
      'HostID',
      '接続先Host',
      'port',
      '証明書hash',
      '端末ID',
      '有効期限',
      invitation ? '招待ID' : '結合ID',
      invitation ? '招待秘密' : '端末秘密',
    };
    bool hex(Object? value, int length) =>
        value is String && RegExp('^[0-9a-f]{$length}\$').hasMatch(value);
    if (data.length != keys.length ||
        !data.keys.every(keys.contains) ||
        data['版'] != 1 ||
        data['版'] is! int ||
        !hex(data['HostID'], 32) ||
        !hex(data['端末ID'], 32) ||
        data['端末ID'] != deviceId ||
        !hex(data['証明書hash'], 64) ||
        !hex(data[invitation ? '招待ID' : '結合ID'], 32) ||
        !hex(data[invitation ? '招待秘密' : '端末秘密'], 64) ||
        data['port'] is! int ||
        (data['port'] as int) < 1 ||
        (data['port'] as int) > 65535 ||
        data['有効期限'] is! int ||
        data['接続先Host'] is! String)
      throw _invalid;
    final host = data['接続先Host'] as String;
    final address = InternetAddress.tryParse(host);
    if (address == null ||
        address.type != InternetAddressType.IPv4 ||
        address.address != host)
      throw _invalid;
    final bytes = address.rawAddress;
    if (!(bytes[0] == 127 ||
        bytes[0] == 10 ||
        (bytes[0] == 172 && bytes[1] >= 16 && bytes[1] <= 31) ||
        (bytes[0] == 192 && bytes[1] == 168)))
      throw _invalid;
    final result = DeviceCredential._(Map.unmodifiable(data), invitation);
    final now = DateTime.now().millisecondsSinceEpoch ~/ 1000;
    if (result.expiry <= now ||
        result.expiry > now + (invitation ? 300 : 28800) + 60)
      throw _invalid;
    return result;
  }

  void matchesInvitation(DeviceCredential invite) {
    for (final key in ['版', 'HostID', '接続先Host', 'port', '証明書hash', '端末ID']) {
      if (data[key] != invite.data[key]) throw _invalid;
    }
  }
}

class DeviceLinkClient implements BrokerTransport {
  DeviceLinkClient(this.credential);
  final DeviceCredential credential;
  bool _active = true;
  int _generation = 0;
  final _sockets = <SecureSocket>{};
  bool get active => _active;
  void setActive(bool value) {
    _active = value;
    if (!value) {
      _generation++;
      for (final socket in _sockets.toList()) {
        socket.destroy();
      }
      _sockets.clear();
    }
  }

  bool _certificate(X509Certificate certificate) {
    final now = DateTime.now().toUtc();
    return sha256.convert(certificate.der).toString() ==
            credential.text('証明書hash') &&
        !now.isBefore(certificate.startValidity) &&
        now.isBefore(certificate.endValidity);
  }

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    const allowed = {
      '端末確認',
      '端末離脱',
      '実行系列挙',
      'Agent一覧',
      '対話開始',
      '対話送信',
      '対話取得',
      '対話中止',
      '対話終了',
      '実行系ライフサイクル状態',
      '実行系資源観測',
      '通知一覧',
      '全Runtime停止要求',
      '対話履歴閲覧状態',
      '対話履歴閲覧',
    };
    if (credential.invitation
        ? operation != '端末結合'
        : !allowed.contains(operation))
      throw _invalid;
    if (!_active || credential.expired) throw _paused;
    final generation = _generation;
    final nonce = newDeviceId();
    final raw = utf8.encode(
      '${jsonEncode({'版': 1, 'HostID': credential.text('HostID'), '端末ID': credential.text('端末ID'), '資格ID': credential.id, '資格秘密': credential.secret, 'nonce': nonce, '発行時刻': DateTime.now().millisecondsSinceEpoch ~/ 1000, '操作': operation, '内容': payload ?? <String, Object?>{}})}\n',
    );
    if (raw.length > 64 * 1024) throw _invalid;
    SecureSocket? socket;
    Timer? deadline;
    try {
      socket = await SecureSocket.connect(
        credential.text('接続先Host'),
        credential.port,
        context: SecurityContext(withTrustedRoots: false),
        onBadCertificate: _certificate,
        timeout: const Duration(seconds: 5),
      );
      _sockets.add(socket);
      deadline = Timer(const Duration(seconds: 5), socket.destroy);
      // OS trustによる成功の場合にも必ず照合する。ここより前に資格を送らない。
      if (!_active || generation != _generation) throw _paused;
      if (socket.peerCertificate == null ||
          !_certificate(socket.peerCertificate!))
        throw _invalid;
      socket.add(raw);
      await socket.flush();
      final bytes = <int>[];
      var complete = false;
      await for (final chunk in socket) {
        bytes.addAll(chunk);
        if (bytes.length > 4 * 1024 * 1024) throw _invalid;
        final newline = bytes.indexOf(10);
        if (newline >= 0) {
          if (newline != bytes.length - 1) throw _invalid;
          complete = true;
          break;
        }
      }
      if (!_active || generation != _generation) throw _paused;
      if (!complete) throw _invalid;
      final response = strictObject(utf8.decode(bytes), limit: 4 * 1024 * 1024);
      if (response['request_id'] != nonce ||
          response['operation'] != operation ||
          response['status'] != 'accepted' ||
          response['audit_event_id'] is! String ||
          (response['audit_event_id'] as String).isEmpty ||
          response['body'] is! Map<String, Object?>) {
        throw const BrokerClientException(
          'Desktopが操作を拒否しました。資格・時刻・監査を確認してください。',
        );
      }
      final body = response['body'] as Map<String, Object?>;
      if ((operation == '端末確認' && (body.length != 1 || body['状態'] != '接続中')) ||
          (operation == '端末離脱' && (body.length != 1 || body['状態'] != '失効')))
        throw _invalid;
      return response;
    } on BrokerClientException {
      rethrow;
    } catch (_) {
      throw const BrokerClientException('Hostへの安全な接続を確認できません。自動再送は行いません。');
    } finally {
      deadline?.cancel();
      _sockets.remove(socket);
      socket?.destroy();
    }
  }

  Future<DeviceCredential> pair() async {
    final response = await request('端末結合');
    final result = DeviceCredential.parse(
      jsonEncode(response['body']),
      invitation: false,
      deviceId: credential.text('端末ID'),
    );
    result.matchesInvitation(credential);
    return result;
  }
}
