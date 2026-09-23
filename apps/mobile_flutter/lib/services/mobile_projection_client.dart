import 'package:gui_shell_ui/runtime_dialogue_client.dart';

/// Desktop Brokerが返す通知summaryのMobile向け投影。
/// 本文、reason、metadata、権限情報は受け取らない。
class MobileNotificationSummary {
  const MobileNotificationSummary({
    required this.items,
    required this.count,
    required this.unreadCount,
    required this.criticalCount,
    required this.auditId,
  });

  final List<MobileNotificationItem> items;
  final int count;
  final int unreadCount;
  final int criticalCount;
  final String auditId;
}

class MobileNotificationItem {
  const MobileNotificationItem({
    required this.title,
    required this.summary,
    required this.severity,
    required this.state,
  });

  final String title;
  final String summary;
  final String severity;
  final String state;
}

/// 緊急停止の実行結果ではなく、Brokerが保持する停止要求の状態だけを返す。
class MobileStopRequestProjection {
  const MobileStopRequestProjection({
    required this.targets,
    required this.approvalState,
    required this.stopExecuted,
    required this.auditId,
  });

  final List<MobileStopTarget> targets;
  final String approvalState;
  final bool stopExecuted;
  final String auditId;
}

class MobileStopTarget {
  const MobileStopTarget({
    required this.runtimeId,
    required this.state,
    required this.approvalRequired,
    required this.stopExecuted,
    required this.reapprovalState,
  });

  final String runtimeId;
  final String state;
  final bool approvalRequired;
  final bool stopExecuted;
  final String reapprovalState;
}

class MobileProjectionClient {
  const MobileProjectionClient(this.transport);

  final BrokerTransport transport;

  Future<MobileNotificationSummary> notifications() async {
    final response = await transport.request(
      '通知一覧',
      payload: const {'版': 1, '未読のみ': false, '上限': 64},
    );
    final body = _acceptedBody(response, '通知一覧');
    _exactKeys(body, const {
      '版',
      '通知一覧',
      '件数',
      '未読件数',
      '重大件数',
      '証拠種別',
      '表示範囲',
      '権限生成',
      '操作',
    });
    final rawItems = body['通知一覧'];
    final count = body['件数'];
    final unread = body['未読件数'];
    final critical = body['重大件数'];
    if (body['版'] != 1 ||
        body['証拠種別'] != 'INTERNAL_STATE' ||
        body['表示範囲'] != 'summary' ||
        body['権限生成'] != 'なし' ||
        body['操作'] != 'navigation_only' ||
        rawItems is! List ||
        rawItems.length > 64 ||
        count is! int ||
        unread is! int ||
        critical is! int ||
        [count, unread, critical].any((value) => value < 0 || value > 256)) {
      _reject('通知summaryの境界を確認できません');
    }
    final items = rawItems.map(_parseNotification).toList(growable: false);
    if (count != items.length || unread < 0 || critical < 0) {
      _reject('通知件数の境界を確認できません');
    }
    return MobileNotificationSummary(
      items: items,
      count: count,
      unreadCount: unread,
      criticalCount: critical,
      auditId: _text(response['audit_event_id']),
    );
  }

  Future<MobileStopRequestProjection> stopRequest() async {
    final response = await transport.request(
      '全Runtime停止要求',
      payload: const {'版': 1},
    );
    final body = _acceptedBody(response, '全Runtime停止要求');
    _exactKeys(body, const {
      '版',
      '要求種別',
      '対象',
      '承認状態',
      '停止実行済み',
      '証拠種別',
      '権限生成',
      '復旧ID',
    });
    final rawTargets = body['対象'];
    if (body['版'] != 1 ||
        body['要求種別'] != '全Runtime停止要求' ||
        rawTargets is! List ||
        rawTargets.length > 128 ||
        body['承認状態'] != 'owner_reapproval_required' ||
        body['停止実行済み'] != false ||
        body['証拠種別'] != 'INTERNAL_STATE' ||
        body['権限生成'] != 'なし' ||
        body['復旧ID'] != 'recover-runtime-stop-request') {
      _reject('停止要求の表示境界を確認できません');
    }
    return MobileStopRequestProjection(
      targets: rawTargets.map(_parseStopTarget).toList(growable: false),
      approvalState: body['承認状態'] as String,
      stopExecuted: false,
      auditId: _text(response['audit_event_id']),
    );
  }
}

MobileNotificationItem _parseNotification(Object? raw) {
  if (raw is! Map) _reject('通知項目の形式を確認できません');
  final value = Map<String, Object?>.from(raw);
  _exactKeys(value, const {
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
  });
  if (value['版'] != 1 ||
      value['表示範囲'] != 'summary' ||
      value['権限生成'] != 'なし' ||
      !{'unread', 'read'}.contains(value['状態']) ||
      !{'info', 'warning', 'critical'}.contains(value['severity'])) {
    _reject('通知項目の表示境界を確認できません');
  }
  return MobileNotificationItem(
    title: _text(value['タイトル']),
    summary: _text(value['概要']),
    severity: value['severity'] as String,
    state: value['状態'] as String,
  );
}

MobileStopTarget _parseStopTarget(Object? raw) {
  if (raw is! Map) _reject('停止対象の形式を確認できません');
  final value = Map<String, Object?>.from(raw);
  _exactKeys(value, const {'実行系ID', '状態', '承認必要', '停止実行済み', '再承認状態', '復旧ID'});
  if (value['承認必要'] != true ||
      value['停止実行済み'] != false ||
      value['再承認状態'] != 'owner_reapproval_required' ||
      value['復旧ID'] != 'recover-runtime-stop-request') {
    _reject('停止対象の権限境界を確認できません');
  }
  return MobileStopTarget(
    runtimeId: _text(value['実行系ID']),
    state: _text(value['状態']),
    approvalRequired: true,
    stopExecuted: false,
    reapprovalState: value['再承認状態'] as String,
  );
}

Map<String, Object?> _acceptedBody(
  Map<String, Object?> response,
  String operation,
) {
  if (response['operation'] != operation ||
      response['status'] != 'accepted' ||
      response['body'] is! Map) {
    _reject('$operation はDesktopで拒否されました');
  }
  return Map<String, Object?>.from(response['body'] as Map);
}

void _exactKeys(Map<String, Object?> value, Set<String> keys) {
  if (value.length != keys.length || !keys.every(value.containsKey)) {
    _reject('Mobile投影の未知fieldを拒否しました');
  }
}

String _text(Object? value) {
  if (value is! String ||
      value.isEmpty ||
      value.length > 256 ||
      value.runes.any((codePoint) => codePoint < 32 || codePoint == 127)) {
    _reject('Mobile投影の文字列を確認できません');
  }
  return value;
}

Never _reject(String message) => throw BrokerClientException(message);
