import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/screens/notifications.dart';
import 'package:gui_shell_desktop/services/shell_core_client.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart' show BrokerTransport;

class _NotificationTransport implements BrokerTransport {
  _NotificationTransport(this.label);

  final String label;
  final operations = <String>[];
  final gates = <String, Completer<void>>{};
  Map<String, Object?>? notificationBody;

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    operations.add(operation);
    final gate = gates[operation];
    if (gate != null) await gate.future;
    if (operation != '通知一覧') {
      return {
        'status': 'accepted',
        'body': <String, Object?>{},
      };
    }
    return {
      'status': 'accepted',
      'body': notificationBody ??
          {
            '版': 1,
            '通知一覧': [
              {
                '版': 1,
                'severity': 'info',
                '遷移先': 'agent',
                '通知ID': 'notification-broker-audit-$label',
                '通知hash': 'sha256:${'a' * 64}',
                '状態': 'unread',
                'タイトル': '通知 $label',
                '概要': '概要 $label',
                'source': 'Task',
                '作成順': 0,
                '関連監査ID': 'broker-audit-$label',
                '関連監査hash': 'sha256:${'b' * 64}',
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
          },
    };
  }
}

Widget _screen(_NotificationTransport transport) => MaterialApp(
      home: Scaffold(
        body: NotificationsScreen(
          client: ShellCoreClient.mock(transport: transport),
        ),
      ),
    );

void main() {
  testWidgets('Broker切替後は新通知だけを表示し旧要求完了を反映しない', (tester) async {
    final first = _NotificationTransport('A');
    final second = _NotificationTransport('B');
    await tester.pumpWidget(_screen(first));
    await tester.pumpAndSettle();
    expect(find.text('通知 A'), findsOneWidget);

    first.gates['通知全既読'] = Completer<void>();
    first.gates['通知一覧'] = Completer<void>();
    await tester.tap(find.text('全て既読'));
    await tester.pump();
    await tester.tap(find.text('更新'));
    await tester.pump();

    await tester.pumpWidget(_screen(second));
    await tester.pumpAndSettle();
    expect(find.text('通知 B'), findsOneWidget);
    expect(find.text('通知 A'), findsNothing);
    expect(
      find.text(
        'Broker接続先が切り替わったため、切替前の通知更新の反映結果は不明です。自動再送していません。必要なら現在のBrokerの通知一覧を更新してください。',
      ),
      findsOneWidget,
    );

    first.gates['通知全既読']!.complete();
    first.gates['通知一覧']!.complete();
    await tester.pumpAndSettle();
    expect(find.text('通知 B'), findsOneWidget);
    expect(find.text('通知 A'), findsNothing);
    expect(find.text('通知の表示状態を更新しました。'), findsNothing);
    expect(
      find.text(
        'Broker接続先が切り替わったため、切替前の通知更新の反映結果は不明です。自動再送していません。必要なら現在のBrokerの通知一覧を更新してください。',
      ),
      findsOneWidget,
    );
    expect(second.operations, ['通知一覧']);
    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('不正なBroker通知一覧を通知なしと表示しない', (tester) async {
    final transport = _NotificationTransport('malformed')
      ..notificationBody = {
        '版': 1,
        '通知一覧': 'C:\\Users\\owner\\secret.json',
        '件数': 0,
        '未読件数': 0,
        '重大件数': 0,
        '証拠種別': 'INTERNAL_STATE',
        '表示範囲': 'summary',
        '権限生成': 'なし',
        '操作': 'navigation_only',
      };
    await tester.pumpWidget(_screen(transport));
    await tester.pumpAndSettle();

    expect(find.text('通知一覧を取得できません。Broker接続と通知状態を確認してください。'), findsOneWidget);
    expect(find.text('通知なし'), findsNothing);
    expect(find.textContaining('secret'), findsNothing);
    await tester.pumpWidget(const SizedBox());
  });
}
