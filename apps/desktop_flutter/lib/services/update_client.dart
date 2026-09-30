import 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerTransport, BrokerClientException;

class UpdateClient {
  const UpdateClient(this._transport);

  final BrokerTransport _transport;

  static String signatureStatusLabel(Object? status) => switch (status) {
        'verified' => '現在のBroker trustで検証済み',
        'verification_stale' => '現在のBroker trustで未検証',
        'legacy_unbound' => '旧候補（package未結合）',
        _ => '不明',
      };

  static String packageSourceLabel(Object? value) {
    if (value is! Map) return '配布元状態不明';
    final status = value['状態'];
    final urlValue = value['URL'];
    if (status == 'unconfigured' && urlValue == null) {
      return '配布元未設定（実downloadは保留）';
    }
    if (status == 'ineligible' && urlValue == null) {
      return '取得元なし（候補を現在trustで再検証できません）';
    }
    if (status != 'configured' || urlValue is! String) {
      return '配布元状態不明';
    }
    final uri = Uri.tryParse(urlValue);
    if (uri == null ||
        uri.scheme != 'https' ||
        uri.host.isEmpty ||
        uri.userInfo.isNotEmpty ||
        uri.hasQuery ||
        uri.hasFragment) {
      return '配布元状態不明';
    }
    return '配布元=${uri.host}（実downloadは保留）';
  }

  Future<Map<String, Object?>> list() async {
    final response = await _transport.request('更新一覧', payload: const {'版': 1});
    return _acceptedBody(response, '更新一覧');
  }

  Future<Map<String, Object?>> verify(
    Map<String, Object?> candidate,
  ) =>
      _request('更新署名検査', {'版': 1, '候補': candidate});

  Future<Map<String, Object?>> requestDownload({
    required String updateId,
    required String candidateHash,
  }) =>
      _request('更新download要求', {
        '版': 1,
        '更新ID': updateId,
        '候補hash': candidateHash,
      });

  Future<Map<String, Object?>> requestApply({
    required String updateId,
    required String candidateHash,
  }) =>
      _request('更新適用要求', {
        '版': 1,
        '更新ID': updateId,
        '候補hash': candidateHash,
      });

  Future<Map<String, Object?>> defer({
    required String updateId,
    required String candidateHash,
    required String deferredUntil,
  }) =>
      _request('更新延期', {
        '版': 1,
        '更新ID': updateId,
        '候補hash': candidateHash,
        '延期期限': deferredUntil,
      });

  Future<Map<String, Object?>> requestRollback({
    required String updateId,
    required String candidateHash,
  }) =>
      _request('更新rollback要求', {
        '版': 1,
        '更新ID': updateId,
        '候補hash': candidateHash,
      });

  Future<Map<String, Object?>> _request(
    String operation,
    Map<String, Object?> payload,
  ) async {
    final response = await _transport.request(operation, payload: payload);
    if (response['status'] != 'accepted' && response['status'] != 'suspended') {
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
