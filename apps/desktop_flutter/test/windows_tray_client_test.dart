import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/services/shell_core_client.dart';
import 'package:gui_shell_desktop/services/windows_tray_client.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart' show BrokerTransport;

class _TrayTransport implements BrokerTransport {
  final operations = <String>[];

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    operations.add(operation);
    return {
      'status': 'accepted',
      'body': {
        '版': 1,
        '通知一覧': <Object?>[],
        '件数': 3,
        '未読件数': 3,
        '重大件数': 3,
        '証拠種別': 'INTERNAL_STATE',
        '表示範囲': 'summary',
        '権限生成': 'なし',
        '操作': 'navigation_only',
      },
    };
  }
}

void main() {
  test('常駐トレイ射影は未Broker snapshotを不明にし通知件数だけ取得する', () async {
    final transport = _TrayTransport();
    final projection = await WindowsTrayProjection.fromSnapshot(
      ShellCoreClient.mock().getSnapshot(),
      transport,
    );

    expect(projection.runtimeStatus, '不明');
    expect(projection.pendingApprovalCount, '不明');
    expect(projection.criticalNotificationCount, 3);
    expect(projection.evidenceSource, '不明');
    expect(projection.stopRequestSupported, isTrue);
    expect(transport.operations, ['通知一覧']);
    expect(projection.toJson()['critical_notification_count'], 3);
  });

  test('常駐トレイ射影はBroker transportなしで件数を0へ変換しない', () async {
    final projection = await WindowsTrayProjection.fromSnapshot(
      ShellCoreClient.mock().getSnapshot(),
      null,
    );

    expect(projection.pendingApprovalCount, '不明');
    expect(projection.criticalNotificationCount, '不明');
    expect(projection.stopRequestSupported, isFalse);
  });
}
