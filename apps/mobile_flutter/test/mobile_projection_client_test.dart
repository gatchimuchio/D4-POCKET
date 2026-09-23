import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_mobile/services/mobile_projection_client.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart';

class _Transport implements BrokerTransport {
  _Transport(this.responses);

  final Map<String, Map<String, Object?>> responses;
  final calls = <String>[];

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    calls.add(operation);
    return responses[operation]!;
  }
}

Map<String, Object?> _response(String operation, Map<String, Object?> body) => {
      'operation': operation,
      'status': 'accepted',
      'audit_event_id': 'audit-1',
      'body': body,
    };

void main() {
  test('通知はsummaryと件数を検証して本文を受け取らない', () async {
    final transport = _Transport({
      '通知一覧': _response('通知一覧', {
        '版': 1,
        '通知一覧': [
          {
            '版': 1,
            '通知ID': 'notification-1',
            'source': 'Broker',
            'severity': 'warning',
            'タイトル': 'Brokerの状態更新',
            '概要': 'Broker操作を保留しました。',
            '作成順': 1,
            '関連監査ID': 'audit-1',
            '関連監査hash': 'sha256:NaN',
            '遷移先': 'dashboard',
            '状態': 'unread',
            '通知hash': 'sha256:NaN',
            '表示範囲': 'summary',
            '権限生成': 'なし',
          },
        ],
        '件数': 1,
        '未読件数': 1,
        '重大件数': 0,
        '証拠種別': 'INTERNAL_STATE',
        '表示範囲': 'summary',
        '権限生成': 'なし',
        '操作': 'navigation_only',
      }),
    });

    final result = await MobileProjectionClient(transport).notifications();

    expect(result.count, 1);
    expect(result.items.single.summary, contains('保留'));
    expect(transport.calls, ['通知一覧']);
  });

  test('停止要求は実停止済みを受理しない', () async {
    final transport = _Transport({
      '全Runtime停止要求': _response('全Runtime停止要求', {
        '版': 1,
        '要求種別': '全Runtime停止要求',
        '対象': [
          {
            '実行系ID': 'local',
            '状態': 'ready',
            '承認必要': true,
            '停止実行済み': false,
            '再承認状態': 'owner_reapproval_required',
            '復旧ID': 'recover-runtime-stop-request',
          },
        ],
        '承認状態': 'owner_reapproval_required',
        '停止実行済み': false,
        '証拠種別': 'INTERNAL_STATE',
        '権限生成': 'なし',
        '復旧ID': 'recover-runtime-stop-request',
      }),
    });

    final result = await MobileProjectionClient(transport).stopRequest();

    expect(result.stopExecuted, isFalse);
    expect(result.targets.single.approvalRequired, isTrue);
  });

  test('通知の権限生成fieldを改変した応答を拒否する', () async {
    final transport = _Transport({
      '通知一覧': _response('通知一覧', {
        '版': 1,
        '通知一覧': <Object?>[],
        '件数': 0,
        '未読件数': 0,
        '重大件数': 0,
        '証拠種別': 'INTERNAL_STATE',
        '表示範囲': 'summary',
        '権限生成': 'permission-granted',
        '操作': 'navigation_only',
      }),
    });

    await expectLater(
      MobileProjectionClient(transport).notifications(),
      throwsA(isA<BrokerClientException>()),
    );
  });
}
