import '../models/generated_contracts.dart';
import 'broker_client.dart';

// 公開metadataだけ。承認・Trust・接続作用は既存Rust Brokerが所有する。
class HostRegistrationClient {
  // 既存wireの固定field名。UIで基盤を表示するための原形保持。
  static const _platformField = 'Platform';
  static const _hostIdField = 'Host ID';
  const HostRegistrationClient(this.transport);
  final BrokerTransport transport;

  Future<String> register({
    required String hostId,
    required String displayName,
    required String platform,
    required String identityHash,
    required int runtimeCount,
    required int agentCount,
  }) async {
    if (!_id(hostId) ||
        !_text(displayName) ||
        !_platforms.contains(platform) ||
        !_hash(identityHash) ||
        !_count(runtimeCount) ||
        !_count(agentCount)) {
      throw const BrokerClientException('Hostの公開入力が不正です');
    }
    final response = await transport.request('Host登録', payload: {
      '版': 1,
      '操作': '登録',
      'Host ID': hostId,
      '表示名': displayName,
      'Platform': platform,
      '接続状態': 'pending_review',
      'Trust': 'pending_review',
      '証明書/identity': {'種別': 'identity_hash', 'hash': identityHash},
      'Runtime summary': {
        'runtime_count': runtimeCount,
        'agent_count': agentCount,
        'evidence_source': 'INTERNAL_STATE'
      },
      '最終接続': null,
    });
    final body = _accepted(response, 'Host登録');
    final record = _record(body);
    if (record.hostId != hostId ||
        record.displayName != displayName ||
        record.platform != platform ||
        record.runtimeCount != runtimeCount ||
        record.agentCount != agentCount ||
        (body['証明書/identity'] as Map)['hash'] != identityHash ||
        body['登録監査ID'] != response['audit_event_id']) {
      throw const BrokerClientException('Host登録の同一対象receiptを確認できません');
    }
    return response['audit_event_id']! as String;
  }

  Future<List<HostRegistryRecord>> refresh() async {
    final body = _accepted(
        await transport.request('Host一覧', payload: {'版': 1}), 'Host一覧');
    final rows = body['Host一覧'];
    if (!_fields(body, {'版', 'Host一覧', '件数', '公開範囲', '証拠種別'}) ||
        body['版'] != 1 ||
        body['公開範囲'] != 'metadata_only' ||
        body['証拠種別'] != 'INTERNAL_STATE' ||
        rows is! List ||
        rows.length > 64 ||
        body['件数'] != rows.length) {
      throw const BrokerClientException('Host一覧の公開境界が不正です');
    }
    final records = rows.map(_record).toList(growable: false);
    if (records.map((r) => r.hostId).toSet().length != records.length) {
      throw const BrokerClientException('Host一覧に重複IDがあります');
    }
    return List.unmodifiable(records);
  }

  static Map _accepted(Map<String, Object?> response, String operation) {
    if (response['operation'] != operation ||
        response['status'] != 'accepted' ||
        response['error'] != null ||
        response['evidence_source'] != 'INTERNAL_STATE' ||
        !_text(response['audit_event_id']) ||
        response['body'] is! Map) {
      // 任意応答・入力をerrorへ展開しない。結果不明の場合も再送しない。
      throw const BrokerClientException(
          'Host操作は未成立です。Brokerの拒否・Auditを確認してください。自動再送しません。');
    }
    return response['body']! as Map;
  }

  static HostRegistryRecord _record(Object? value) {
    final trust = value is Map ? value['Trust'] : null;
    final identity = value is Map ? value['証明書/identity'] : null;
    final summary = value is Map ? value['Runtime summary'] : null;
    if (value is! Map ||
        !_fields(value, {
          '版',
          'Host ID',
          '表示名',
          'Platform',
          '接続状態',
          'Trust',
          '証明書/identity',
          'Runtime summary',
          '最終接続',
          '公開範囲',
          '証拠種別',
          '権限生成',
          'authority_strip',
          '能力ID',
          '権限ID',
          '承認状態',
          '復旧ID',
          '登録監査ID'
        }) ||
        value['版'] != 1 ||
        !_id(value['Host ID']) ||
        !_text(value['表示名']) ||
        !_platforms.contains(value['Platform']) ||
        value['接続状態'] != 'pending_review' ||
        !_fields(
            trust, {'state', 'evidence_source', 'requires_operator_review'}) ||
        (trust as Map)['state'] != 'pending_review' ||
        trust['evidence_source'] != 'INTERNAL_STATE' ||
        trust['requires_operator_review'] != true ||
        !_fields(identity, {'種別', 'hash'}) ||
        !{'certificate_hash', 'identity_hash'}
            .contains((identity as Map)['種別']) ||
        !_hash(identity['hash']) ||
        !_fields(
            summary, {'runtime_count', 'agent_count', 'evidence_source'}) ||
        !_count((summary as Map)['runtime_count']) ||
        !_count(summary['agent_count']) ||
        summary['evidence_source'] != 'INTERNAL_STATE' ||
        value['最終接続'] != null ||
        value['公開範囲'] != 'metadata_only' ||
        value['証拠種別'] != 'INTERNAL_STATE' ||
        value['権限生成'] != 'なし' ||
        value['authority_strip'] != true ||
        value['能力ID'] != 'host.registry.register' ||
        value['権限ID'] != 'permission.host.registry.register' ||
        value['承認状態'] != 'owner_control_approved' ||
        value['復旧ID'] != 'recover-host-registration' ||
        !_text(value['登録監査ID'])) {
      throw const BrokerClientException('Host metadataの未審査・非権限境界が不正です');
    }
    return HostRegistryRecord(
        hostId: value[_hostIdField] as String,
        displayName: value['表示名'] as String,
        platform: value[_platformField] as String,
        connectionState: 'pending_review',
        trustState: 'pending_review',
        runtimeCount: summary['runtime_count'] as int,
        agentCount: summary['agent_count'] as int,
        evidenceSource: 'INTERNAL_STATE',
        visibility: 'metadata_only',
        authorityStrip: true);
  }

  static const _platforms = {
    'windows',
    'linux',
    'macos',
    'android',
    'ios',
    'unknown'
  };
  static bool _fields(Object? value, Set<String> keys) =>
      value is Map &&
      value.length == keys.length &&
      value.keys.every(keys.contains);
  static bool _text(Object? value) =>
      value is String &&
      value.trim().isNotEmpty &&
      value.runes.length <= 256 &&
      !value.runes.any((r) => r < 32 || r == 127);
  static bool _id(Object? value) =>
      value is String &&
      value.length <= 128 &&
      RegExp(r'^[A-Za-z0-9][A-Za-z0-9_.:-]*$').hasMatch(value) &&
      _text(value);
  static bool _hash(Object? value) =>
      value is String &&
      value.length == 71 &&
      RegExp(r'^sha256:[a-f0-9]{64}$').hasMatch(value);
  static bool _count(Object? value) =>
      value is int && value >= 0 && value <= 256;
}
