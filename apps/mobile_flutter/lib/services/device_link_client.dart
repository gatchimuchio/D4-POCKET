import 'dart:convert';

import 'package:flutter/services.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart';

const _deviceLinkChannel = MethodChannel('gui_shell/mobile_device_link');
const _operations = <String>{
  '実行系列挙',
  'Agent一覧',
  '作業領域一覧',
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

const _channelFailure = BrokerClientException(
  '端末のnative接続経路を利用できません。操作を停止しました。',
);

/// Flutterへ公開する状態。端末ID・接続先・資格・secretは含めない。
class DeviceLinkSnapshot {
  const DeviceLinkSnapshot({
    required this.storageReady,
    required this.paired,
    required this.connected,
    required this.foreground,
    required this.status,
  });

  final bool storageReady;
  final bool paired;
  final bool connected;
  final bool foreground;
  final String status;

  factory DeviceLinkSnapshot.fromNative(Object? raw) {
    if (raw is! Map) throw _channelFailure;
    final value = <String, Object?>{};
    for (final entry in raw.entries) {
      if (entry.key is! String) throw _channelFailure;
      value[entry.key as String] = entry.value;
    }
    const keys = {
      'storage_ready',
      'paired',
      'connected',
      'foreground',
      'status',
    };
    if (value.length != keys.length || !keys.containsAll(value.keys)) {
      throw _channelFailure;
    }
    final status = value['status'];
    if (value['storage_ready'] is! bool ||
        value['paired'] is! bool ||
        value['connected'] is! bool ||
        value['foreground'] is! bool ||
        status is! String ||
        status.isEmpty ||
        status.length > 160 ||
        status.runes.any((unit) => unit < 0x20 || unit == 0x7f)) {
      throw _channelFailure;
    }
    return DeviceLinkSnapshot(
      storageReady: value['storage_ready'] as bool,
      paired: value['paired'] as bool,
      connected: value['connected'] as bool,
      foreground: value['foreground'] as bool,
      status: status,
    );
  }
}

class DeviceLinkLocalRecoveryAuditEvent {
  const DeviceLinkLocalRecoveryAuditEvent({
    required this.eventId,
    required this.timestamp,
  });

  final String eventId;
  final String timestamp;
  static const maximumEvents = 32;
  static const _payloadHash =
      'sha256:2f0b366fb62efa9e741f0127c676e648d81b263a471c35d0dd00814a174e5a92';
  static final _utcTimestampPattern = RegExp(
    r'^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})(?:\.(\d{3}))?Z$',
  );

  static List<DeviceLinkLocalRecoveryAuditEvent> fromNative(Object? raw) {
    if (raw is! List || raw.length > maximumEvents) throw _channelFailure;
    final events = raw.map(_eventFromNative).toList(growable: false);
    if (events.map((event) => event.eventId).toSet().length != events.length) {
      throw _channelFailure;
    }
    return events;
  }

  static bool _isCanonicalUtcTimestamp(String value) {
    final match = _utcTimestampPattern.firstMatch(value);
    final parsed = DateTime.tryParse(value);
    if (match == null || parsed == null || !parsed.isUtc) return false;
    final parts = [
      for (var index = 1; index <= 6; index++) int.parse(match.group(index)!),
    ];
    final fraction = match.group(7) ?? '';
    final millisecond = int.parse((fraction + '000').substring(0, 3));
    return parsed.year == parts[0] &&
        parsed.month == parts[1] &&
        parsed.day == parts[2] &&
        parsed.hour == parts[3] &&
        parsed.minute == parts[4] &&
        parsed.second == parts[5] &&
        parsed.millisecond == millisecond &&
        parsed.microsecond == 0;
  }

  static DeviceLinkLocalRecoveryAuditEvent _eventFromNative(Object? raw) {
    if (raw is! Map) throw _channelFailure;
    final value = <String, Object?>{};
    for (final entry in raw.entries) {
      if (entry.key is! String) throw _channelFailure;
      value[entry.key as String] = entry.value;
    }
    const eventKeys = {
      'event_id',
      'timestamp',
      'actor',
      'action',
      'target',
      'result',
      'payload_hash',
      'metadata',
    };
    final metadata = value['metadata'];
    if (value.length != eventKeys.length ||
        !eventKeys.containsAll(value.keys) ||
        value['event_id'] is! String ||
        !RegExp(r'^[a-f0-9]{32}$').hasMatch(value['event_id'] as String) ||
        value['timestamp'] is! String ||
        !_isCanonicalUtcTimestamp(value['timestamp'] as String) ||
        value['actor'] != 'shell' ||
        value['action'] != 'mobile_local_credential_delete' ||
        value['target'] != 'mobile_device_link_credential' ||
        value['result'] != 'success' ||
        value['payload_hash'] != _payloadHash ||
        metadata is! Map) {
      throw _channelFailure;
    }
    final metadataValue = <String, Object?>{};
    for (final entry in metadata.entries) {
      if (entry.key is! String) throw _channelFailure;
      metadataValue[entry.key as String] = entry.value;
    }
    const metadataKeys = {
      'audit_scope',
      'evidence_source',
      'desktop_revocation',
      'authority_effect',
      'operator_identity',
    };
    if (metadataValue.length != metadataKeys.length ||
        !metadataKeys.containsAll(metadataValue.keys) ||
        metadataValue['audit_scope'] != 'mobile_local_recovery' ||
        metadataValue['evidence_source'] != 'INTERNAL_STATE' ||
        metadataValue['desktop_revocation'] != 'unconfirmed' ||
        metadataValue['authority_effect'] != 'none' ||
        metadataValue['operator_identity'] != 'unverified') {
      throw _channelFailure;
    }
    return DeviceLinkLocalRecoveryAuditEvent(
      eventId: value['event_id'] as String,
      timestamp: value['timestamp'] as String,
    );
  }
}

abstract interface class DeviceLinkNativePort implements BrokerTransport {
  Future<DeviceLinkSnapshot> readState();
  Future<List<DeviceLinkLocalRecoveryAuditEvent>> readLocalRecoveryAudit();
  Future<DeviceLinkSnapshot> pair();
  Future<DeviceLinkSnapshot> disconnect();
  Future<DeviceLinkSnapshot> localDelete();
}

/// 固定MethodChannelの薄いUI adapter。保管・TLS・招待parseはnativeに限定する。
class MethodChannelDeviceLink implements DeviceLinkNativePort {
  MethodChannelDeviceLink({MethodChannel? channel})
    : _channel = channel ?? _deviceLinkChannel;

  final MethodChannel _channel;

  Future<DeviceLinkSnapshot> _state(
    String method, [
    Map<String, Object?>? args,
  ]) async {
    final value = await _call(method, args ?? const {'version': 1});
    return DeviceLinkSnapshot.fromNative(value);
  }

  @override
  Future<DeviceLinkSnapshot> readState() => _state('read_state');

  @override
  Future<List<DeviceLinkLocalRecoveryAuditEvent>>
  readLocalRecoveryAudit() async {
    final raw = await _call('read_recovery_audit', const {'version': 1});
    return DeviceLinkLocalRecoveryAuditEvent.fromNative(raw);
  }

  @override
  Future<DeviceLinkSnapshot> pair() => _state('pair');

  @override
  Future<DeviceLinkSnapshot> disconnect() => _state('disconnect');

  @override
  Future<DeviceLinkSnapshot> localDelete() => _state('local_delete');

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    if (!_operations.contains(operation)) throw _channelFailure;
    final body = payload ?? const <String, Object?>{};
    if (operation == '作業領域一覧' && body.isNotEmpty) {
      throw _channelFailure;
    }
    if (operation == '対話履歴閲覧') {
      _validateHistoryPayload(body);
    } else {
      _validateSafeTree(body, 0);
    }
    _bounded(body: body);
    final response = await _call('broker_request', {
      'version': 1,
      'broker_operation': operation,
      'payload': body,
    });
    return _brokerResponse(response, operation);
  }

  Future<Object?> _call(String method, Map<String, Object?> args) async {
    try {
      return await _channel.invokeMethod<Object?>(method, args);
    } on PlatformException {
      throw _channelFailure;
    } on MissingPluginException {
      throw _channelFailure;
    } catch (_) {
      throw _channelFailure;
    }
  }
}

Map<String, Object?> _brokerResponse(Object? raw, String operation) {
  if (raw is! Map) throw _channelFailure;
  final source = <String, Object?>{};
  for (final entry in raw.entries) {
    if (entry.key is! String) throw _channelFailure;
    source[entry.key as String] = entry.value;
  }
  const allowed = {
    'operation',
    'status',
    'audit_event_id',
    'evidence_source',
    'body',
    'error',
  };
  if (source.keys.any((key) => !allowed.contains(key)) ||
      source['operation'] != operation ||
      !{'accepted', 'rejected', 'suspended'}.contains(source['status']) ||
      source['audit_event_id'] is! String ||
      (source['audit_event_id'] as String).isEmpty ||
      (source['audit_event_id'] as String).length > 256 ||
      (source['audit_event_id'] as String).runes.any(
        (unit) => unit < 0x21 || unit == 0x7f,
      ) ||
      !{
        'LIVE_RUNTIME',
        'INTERNAL_STATE',
        'CONFIG',
        'EXTERNAL_EVIDENCE',
        'FIXTURE',
      }.contains(source['evidence_source'])) {
    throw _channelFailure;
  }
  final body = source['body'];
  if (body != null) _validateResponseTree(body, 0);
  final error = source['error'];
  Object? safeError;
  if (error != null) {
    if (error is! Map || error['code'] is! String) throw _channelFailure;
    safeError = {'code': error['code']};
  }
  if (source['status'] == 'accepted' && body is! Map) throw _channelFailure;
  if (source['status'] != 'accepted' && error == null) throw _channelFailure;
  return {
    'operation': operation,
    'status': source['status'],
    'audit_event_id': source['audit_event_id'],
    'evidence_source': source['evidence_source'],
    'body': body,
    'error': safeError,
  };
}

void _validateHistoryPayload(Map<String, Object?> payload) {
  if (payload.length != 2 ||
      !payload.keys.toSet().containsAll({'approval_id', 'query'})) {
    throw _channelFailure;
  }
  final approvalId = payload['approval_id'];
  final query = payload['query'];
  if (approvalId is! String ||
      !RegExp(r'^[a-f0-9]{32}$').hasMatch(approvalId) ||
      query is! Map<String, Object?>) {
    throw _channelFailure;
  }
  const queryKeys = {
    'after',
    'limit',
    'latest_per_request',
    'include_audit_context',
    'include_result_evidence',
    'include_content_receipt',
    'filter',
  };
  if (!query.keys.toSet().containsAll({'after', 'limit'}) ||
      query.keys.any((key) => !queryKeys.contains(key)) ||
      query['after'] is! int ||
      (query['after'] as int) < 0 ||
      query['limit'] is! int ||
      (query['limit'] as int) < 1 ||
      (query['limit'] as int) > 100) {
    throw _channelFailure;
  }
  for (final key in const {
    'latest_per_request',
    'include_audit_context',
    'include_result_evidence',
    'include_content_receipt',
  }) {
    if (query.containsKey(key) && query[key] is! bool) throw _channelFailure;
  }
  final filter = query['filter'];
  if (filter == null) return;
  if (filter is! Map<String, Object?>) throw _channelFailure;
  const filterKeys = {'要求ID', '対話セッションID', '実行系ID', '状態'};
  if (filter.keys.any((key) => !filterKeys.contains(key)))
    throw _channelFailure;
  for (final entry in filter.entries) {
    final value = entry.value;
    final valid = switch (entry.key) {
      '要求ID' || '対話セッションID' =>
        value is String && RegExp(r'^[a-f0-9]{32}$').hasMatch(value),
      '実行系ID' =>
        value is String &&
            value.length <= 128 &&
            RegExp(r'^[A-Za-z0-9][A-Za-z0-9_.-]*$').hasMatch(value),
      '状態' =>
        value is String &&
            const {'承認待ち', '実行中', '成功', '保留', '失敗', '中止'}.contains(value),
      _ => false,
    };
    if (!valid) throw _channelFailure;
  }
}

void _validateSafeTree(Object? value, int depth) {
  if (depth > 32) throw _channelFailure;
  if (value == null || value is bool || value is String || value is int) return;
  if (value is double) {
    if (!value.isFinite) throw _channelFailure;
    return;
  }
  if (value is List) {
    if (value.length > 4096) throw _channelFailure;
    for (final item in value) {
      _validateSafeTree(item, depth + 1);
    }
    return;
  }
  if (value is Map) {
    if (value.length > 4096) throw _channelFailure;
    for (final entry in value.entries) {
      if (entry.key is! String || _forbiddenPayloadKey(entry.key as String)) {
        throw _channelFailure;
      }
      _validateSafeTree(entry.value, depth + 1);
    }
    return;
  }
  throw _channelFailure;
}

void _validateResponseTree(Object? value, int depth) {
  if (depth > 32) throw _channelFailure;
  if (value == null || value is bool || value is String || value is int) return;
  if (value is double) {
    if (!value.isFinite) throw _channelFailure;
    return;
  }
  if (value is List) {
    if (value.length > 4096) throw _channelFailure;
    for (final item in value) {
      _validateResponseTree(item, depth + 1);
    }
    return;
  }
  if (value is Map) {
    if (value.length > 4096) throw _channelFailure;
    for (final entry in value.entries) {
      if (entry.key is! String || _forbiddenResponseKey(entry.key as String)) {
        throw _channelFailure;
      }
      _validateResponseTree(entry.value, depth + 1);
    }
    return;
  }
  throw _channelFailure;
}

bool _forbiddenPayloadKey(String key) {
  final normalized = key.toLowerCase().replaceAll(RegExp(r'[^a-z0-9]'), '');
  return const [
        'owner',
        'authority',
        'permission',
        'approval',
        'audit',
        'credential',
        'secret',
        'token',
        'private',
        'password',
      ].any((term) => normalized.contains(term)) ||
      const {'招待秘密', '端末秘密', '資格秘密', '端末資格'}.contains(key);
}

bool _forbiddenResponseKey(String key) {
  final normalized = key.toLowerCase().replaceAll(RegExp(r'[^a-z0-9]'), '');
  if (const {'secretvaluepresent', 'secretpaths'}.contains(normalized)) {
    return false;
  }
  if (normalized.contains('secret')) return true;
  return const [
        'credential',
        'password',
        'privatekey',
        'accesstoken',
        'refreshtoken',
      ].any((term) => normalized.contains(term)) ||
      const {'招待秘密', '端末秘密', '資格秘密', '端末資格'}.contains(key);
}

void _bounded({required Object body}) {
  try {
    if (utf8.encode(jsonEncode(body)).length > 64 * 1024) throw _channelFailure;
  } catch (_) {
    throw _channelFailure;
  }
}
