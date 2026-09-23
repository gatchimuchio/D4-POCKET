import 'dart:convert';

import 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerTransport, BrokerClientException;

import 'broker_client.dart' show brokerPayloadHash;

class ProfileClient {
  const ProfileClient(this._transport);

  final BrokerTransport _transport;

  Future<List<Map<String, Object?>>> list() async {
    final response = await _transport.request(
      'プロファイル一覧',
      payload: const {'版': 1},
    );
    final body = _acceptedBody(response, 'プロファイル一覧');
    final profiles = body['Profiles'];
    if (profiles is! List) {
      throw const BrokerClientException('Profile一覧のProfilesがarrayではありません');
    }
    return [
      for (final profile in profiles)
        if (profile is Map)
          (Map<String, Object?>.from(profile)
            ..['profile_hash'] = brokerPayloadHash(
              Map<String, Object?>.from(profile),
            )),
    ];
  }

  Future<Map<String, Object?>> create(Map<String, Object?> profile) =>
      _request('プロファイル作成', profile);

  Future<Map<String, Object?>> copy({
    required String sourceProfileId,
    required String profileId,
    required String displayName,
  }) =>
      _request('プロファイル複製', {
        '版': 1,
        'source_ProfileID': sourceProfileId,
        'ProfileID': profileId,
        '表示名': displayName,
      });

  Future<Map<String, Object?>> apply({
    required String profileId,
    required String profileHash,
  }) =>
      _request('プロファイル適用要求', {
        '版': 1,
        'ProfileID': profileId,
        'profile_hash': profileHash,
      });

  Future<Map<String, Object?>> delete({
    required String profileId,
    String? profileHash,
  }) =>
      _request('プロファイル削除', {
        '版': 1,
        'ProfileID': profileId,
        if (profileHash != null) 'profile_hash': profileHash,
      });

  Future<Map<String, Object?>> export(String profileId) =>
      _request('プロファイルexport', {'版': 1, 'ProfileID': profileId});

  Future<Map<String, Object?>> importProfile(Map<String, Object?> profile) =>
      _request('プロファイルimport', {'版': 1, 'Profile': profile});

  Future<Map<String, Object?>> _request(
    String operation,
    Map<String, Object?> payload,
  ) async {
    final response = await _transport.request(operation, payload: payload);
    return _acceptedBody(response, operation);
  }
}

Map<String, Object?> _acceptedBody(
  Map<String, Object?> response,
  String operation,
) {
  if (response['status'] != 'accepted') {
    final error = response['error'];
    final message = error is Map
        ? error['message']?.toString() ?? response.toString()
        : response.toString();
    throw BrokerClientException('$operation が拒否されました: $message');
  }
  final body = response['body'];
  if (body is! Map) {
    throw BrokerClientException('$operation 応答bodyがobjectではありません');
  }
  return Map<String, Object?>.from(body);
}

String profileJson(Map<String, Object?> value) =>
    const JsonEncoder.withIndent('  ').convert(value);
