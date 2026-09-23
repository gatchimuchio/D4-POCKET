import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/services/notification_client.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart' show BrokerTransport;

class _NotificationTransport implements BrokerTransport {
  final operations = <String>[];
  final payloads = <Map<String, Object?>?>[];

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    operations.add(operation);
    payloads.add(payload);
    return {
      'status': 'accepted',
      'body': {
        '版': 1,
        '通知一覧': <Object?>[],
        '件数': 0,
        '未読件数': 0,
        '重大件数': 0,
        '証拠種別': 'INTERNAL_STATE',
        '表示範囲': 'summary',
        '権限生成': 'なし',
        '操作': 'navigation_only',
      },
    };
  }
}

void main() {
  test('通知clientはBrokerのnavigation-only操作へ限定される', () async {
    final transport = _NotificationTransport();
    final client = NotificationClient(transport);
    await client.list(unreadOnly: true);
    await client.markRead(
      notificationId: 'notification-broker-audit-1',
      notificationHash: 'sha256:${'a' * 64}',
    );
    await client.dismiss(
      notificationId: 'notification-broker-audit-1',
      notificationHash: 'sha256:${'a' * 64}',
    );
    await client.markAllRead();
    expect(transport.operations, ['通知一覧', '通知既読', '通知破棄', '通知全既読']);
    expect(transport.payloads.first?['未読のみ'], isTrue);
  });
}
