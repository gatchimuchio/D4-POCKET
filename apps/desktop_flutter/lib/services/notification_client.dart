import 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerTransport, BrokerClientException;

class NotificationClient {
  const NotificationClient(this._transport);

  final BrokerTransport _transport;

  Future<Map<String, Object?>> list({
    bool unreadOnly = false,
    int limit = 200,
  }) =>
      _request('通知一覧', {
        '版': 1,
        '未読のみ': unreadOnly,
        '上限': limit,
      });

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
    late final Map<String, Object?> response;
    try {
      response = await _transport.request(operation, payload: payload);
    } on Object {
      throw BrokerClientException(_failureMessage(operation));
    }
    if (response['status'] != 'accepted') {
      throw BrokerClientException(_failureMessage(operation));
    }
    return _body(response, operation);
  }
}

String _failureMessage(String operation) => switch (operation) {
      '通知一覧' => '通知一覧を取得できません。Broker接続と通知状態を確認してください。',
      _ => '通知の状態を更新できません。Broker接続と通知状態を確認してください。',
    };

Map<String, Object?> _body(Map<String, Object?> response, String operation) {
  final body = response['body'];
  if (body is! Map) {
    throw BrokerClientException('$operation 応答の通知形式が不正です。表示を停止しました。');
  }
  try {
    return Map<String, Object?>.from(body);
  } on Object {
    throw BrokerClientException('$operation 応答の通知形式が不正です。表示を停止しました。');
  }
}
