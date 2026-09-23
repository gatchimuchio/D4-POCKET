import 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerTransport, BrokerClientException;

class UpdateClient {
  const UpdateClient(this._transport);

  final BrokerTransport _transport;

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
