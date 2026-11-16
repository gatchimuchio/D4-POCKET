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
    if (response['status'] != 'accepted' || response['error'] != null) {
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
  late final Map<String, Object?> converted;
  try {
    converted = Map<String, Object?>.from(body);
  } on Object {
    throw BrokerClientException('$operation 応答の通知形式が不正です。表示を停止しました。');
  }
  if (operation == '通知一覧') return _validatedNotificationList(converted);
  return converted;
}

const _notificationRecordFields = <String>{
  '版',
  '通知ID',
  'source',
  'severity',
  'タイトル',
  '概要',
  '作成順',
  '関連監査ID',
  '関連監査hash',
  '遷移先',
  '状態',
  '通知hash',
  '表示範囲',
  '権限生成',
};

Map<String, Object?> _validatedNotificationList(Map<String, Object?> body) {
  final rawItems = body['通知一覧'];
  if (body['版'] is! int ||
      body['版'] != 1 ||
      rawItems is! List ||
      rawItems.length > 256 ||
      body['証拠種別'] != 'INTERNAL_STATE' ||
      body['表示範囲'] != 'summary' ||
      body['権限生成'] != 'なし' ||
      body['操作'] != 'navigation_only') {
    throw _invalidNotificationList();
  }

  final items = <Map<String, Object?>>[];
  for (final rawItem in rawItems) {
    if (rawItem is! Map) throw _invalidNotificationList();
    late final Map<String, Object?> item;
    try {
      item = Map<String, Object?>.from(rawItem);
    } on Object {
      throw _invalidNotificationList();
    }
    items.add(_validatedNotificationRecord(item));
  }

  final itemCount = body['件数'];
  final unreadCount = body['未読件数'];
  final criticalCount = body['重大件数'];
  final actualUnreadCount =
      items.where((item) => item['状態'] == 'unread').length;
  final actualCriticalCount =
      items.where((item) => item['severity'] == 'critical').length;
  if (itemCount is! int ||
      unreadCount is! int ||
      criticalCount is! int ||
      itemCount != items.length ||
      unreadCount != actualUnreadCount ||
      criticalCount != actualCriticalCount) {
    throw _invalidNotificationList();
  }

  // Only the bounded, declared projection crosses into the UI consumer.
  return {
    '版': 1,
    '通知一覧': items,
    '件数': itemCount,
    '未読件数': unreadCount,
    '重大件数': criticalCount,
    '証拠種別': 'INTERNAL_STATE',
    '表示範囲': 'summary',
    '権限生成': 'なし',
    '操作': 'navigation_only',
  };
}

Map<String, Object?> _validatedNotificationRecord(Map<String, Object?> item) {
  if (item.length != _notificationRecordFields.length ||
      !_notificationRecordFields.every(item.containsKey) ||
      item['版'] is! int ||
      item['版'] != 1) {
    throw _invalidNotificationList();
  }

  final notificationId = item['通知ID'];
  final source = item['source'];
  final severity = item['severity'];
  final title = item['タイトル'];
  final summary = item['概要'];
  final createdOrder = item['作成順'];
  final auditId = item['関連監査ID'];
  final auditHash = item['関連監査hash'];
  final target = item['遷移先'];
  final state = item['状態'];
  final notificationHash = item['通知hash'];

  if (notificationId is! String ||
      source is! String ||
      severity is! String ||
      title is! String ||
      summary is! String ||
      createdOrder is! int ||
      auditId is! String ||
      auditHash is! String ||
      target is! String ||
      state is! String ||
      notificationHash is! String ||
      notificationId.length > 256 ||
      createdOrder < 0 ||
      !_validAuditId(auditId) ||
      notificationId != 'notification-$auditId' ||
      !_validSha256(auditHash) ||
      !_validSha256(notificationHash) ||
      title.isEmpty ||
      title.length > 256 ||
      summary.isEmpty ||
      summary.length > 512 ||
      !_validNotificationRoute(source, target) ||
      !const {'info', 'warning', 'error', 'critical'}.contains(severity) ||
      !const {'unread', 'read'}.contains(state) ||
      item['表示範囲'] != 'summary' ||
      item['権限生成'] != 'なし') {
    throw _invalidNotificationList();
  }
  return item;
}

bool _validAuditId(String value) =>
    value.startsWith('broker-audit-') &&
    value.length <= 256 &&
    value.codeUnits.every((unit) => unit > 0x20 && unit != 0x7f);

bool _validSha256(String value) =>
    value.length == 71 &&
    value.startsWith('sha256:') &&
    value.codeUnits.skip(7).every(
          (unit) =>
              (unit >= 0x30 && unit <= 0x39) || (unit >= 0x61 && unit <= 0x66),
        );

bool _validNotificationRoute(String source, String target) => switch (source) {
      'Task' => target == 'agent' || target == 'recovery',
      'Permission' => target == 'permission',
      'Approval' => target == 'approval',
      'Device' => target == 'device',
      'MCP' || 'A2A' => target == 'agent',
      'Update' => target == 'settings',
      'Evaluation' => target == 'evaluation',
      'Recovery' => target == 'recovery',
      'Audit' => target == 'audit',
      'Runtime' => target == 'runtime',
      _ => false,
    };

BrokerClientException _invalidNotificationList() =>
    const BrokerClientException('通知一覧応答の形式が不正です。表示を停止しました。');
