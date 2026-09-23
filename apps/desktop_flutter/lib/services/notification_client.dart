import 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerTransport, BrokerClientException;

class NotificationClient {
  const NotificationClient(this._transport);

  final BrokerTransport _transport;

  Future<Map<String, Object?>> list({
    bool unreadOnly = false,
    int limit = 200,
  }) async {
    final response = await _transport.request(
      '通知一覧',
      payload: {
        '版': 1,
        '未読のみ': unreadOnly,
        '上限': limit,
      },
    );
    return _acceptedBody(response, '通知一覧');
  }

  Future<Map<String, Object?>> markRead({
    required String notificationId,
    required String notificationHash,
  }) =>
      _request('通知既読', {
        '版': 1,
        '通知ID': notificationId,
        '通知hash': notificationHash,
      });

  Future<Map<String, Object?>> dismiss({
    required String notificationId,
    required String notificationHash,
  }) =>
      _request('通知破棄', {
        '版': 1,
        '通知ID': notificationId,
        '通知hash': notificationHash,
      });

  Future<Map<String, Object?>> markAllRead() =>
      _request('通知全既読', const {'版': 1});

  Future<Map<String, Object?>> _request(
    String operation,
    Map<String, Object?> payload,
  ) async {
    final response = await _transport.request(operation, payload: payload);
    if (response['status'] != 'accepted') {
      final error = response['error'];
      final message = error is Map
          ? error['message']?.toString() ?? response.toString()
          : response.toString();
      throw BrokerClientException('$operation が拒否されました: $message');
    }
    return _body(response, operation);
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
  return _body(response, operation);
}

Map<String, Object?> _body(Map<String, Object?> response, String operation) {
  final body = response['body'];
  if (body is! Map) {
    throw BrokerClientException('$operation 応答bodyがobjectではありません');
  }
  return Map<String, Object?>.from(body);
}
