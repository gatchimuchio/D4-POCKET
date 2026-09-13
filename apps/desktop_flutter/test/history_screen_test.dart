import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_ui/gui_shell_ui.dart';
import 'package:gui_shell_desktop/screens/history_screen.dart';

class Fixture implements BrokerTransport {
  bool revoked = false;
  Map<String, Object?>? sent;
  void Function(Map<String, Object?>)? alterReplay;
  bool revokeDuringRead = false;
  String runtime = 'local';
  String state = '成功';
  Completer<void>? gate;
  final operations = <String>[];
  final expiry = DateTime.now().millisecondsSinceEpoch ~/ 1000 + 290;
  Map<String, Object?> get grant =>
      {'approval_id': 'a' * 32, 'runtime_id': 'local', 'expires_at': expiry};
  @override
  Future<Map<String, Object?>> request(String op,
      {Map<String, Object?>? payload}) async {
    operations.add(op);
    if (op == '対話履歴閲覧') {
      expect((payload!['query'] as Map)['latest_per_request'], true);
    }
    if (op == '対話再実行' || op == '対話分岐') {
      sent = payload;
      final body = <String, Object?>{
        '要求ID': 'f' * 32,
        '要求hash': 'sha256:${'0' * 64}',
        '状態': '承認待ち',
        '期限': expiry,
        '対話セッションID': '1' * 32,
        '実行系ID': 'local',
        '参照監査ID': 'history-1',
        '参照event_hash': 'sha256:${'c' * 64}',
        '種別': op
      };
      alterReplay?.call(body);
      return {
        'operation': op,
        'status': 'accepted',
        'evidence_source': 'INTERNAL_STATE',
        'audit_event_id': 'replay-audit',
        'body': body
      };
    }
    final isRead = op == '対話履歴閲覧';
    if (isRead && gate != null) await gate!.future;
    final body = <String, Object?>{
      'grant': revoked ? null : grant,
      'page': null
    };
    if (isRead) {
      body['page'] = {
        'version': 1,
        'next_cursor': 5,
        'has_more': false,
        'head_hash': 'sha256:${'b' * 64}',
        'entries': [
          {
            'audit_event_id': 'history-1',
            'event_hash': 'sha256:${'c' * 64}',
            'record': {
              '版': 1,
              '状態': state,
              '失敗分類': null,
              '実行記録': {
                '要求ID': 'd' * 32,
                '実行系ID': runtime,
                '対話セッションID': 'e' * 32,
                '作成時刻': 1,
                '開始時刻': 2,
                '終了時刻': 3,
                '作成監査ID': 'created',
                '開始監査ID': 'started',
                '終了監査ID': 'ended'
              }
            }
          }
        ]
      };
      if (revokeDuringRead) revoked = true;
    }
    return {
      'operation': op,
      'status': 'accepted',
      'evidence_source': 'INTERNAL_STATE',
      'audit_event_id': 'audit',
      'body': body
    };
  }
}

void main() {
  test('再要求clientは参照と新規性と現在承認を検査する', () async {
    final f = Fixture();
    final c = HistoryClient(f);
    final grant = (await c.status())!;
    final parent = (await c.page(grant)).entries.single;
    final result = await c.replay(grant, parent, '明示入力', branch: true);
    expect(result['状態'], '承認待ち');
    expect(f.sent!['入力'], '明示入力');
    expect(f.sent!.containsKey('対話セッションID'), false);
    for (final entry in {
      '要求ID': 'd' * 32,
      '対話セッションID': 'e' * 32,
      '実行系ID': 'other',
      '状態': '成功',
      '参照監査ID': 'different',
      '参照event_hash': 'sha256:${'a' * 64}',
      '種別': '対話再実行'
    }.entries) {
      f.alterReplay = (body) => body[entry.key] = entry.value;
      await expectLater(c.replay(grant, parent, '明示入力', branch: true),
          throwsA(isA<BrokerClientException>()));
    }
    f.alterReplay = (_) => f.revoked = true;
    await expectLater(c.replay(grant, parent, '明示入力', branch: true),
        throwsA(isA<BrokerClientException>()));
    expect(f.operations.contains('対話承認'), false);
  });
  testWidgets('画面から明示入力で承認待ち要求を作り背景化で破棄する', (tester) async {
    final f = Fixture();
    await tester.pumpWidget(MaterialApp(
        home: Scaffold(body: HistoryScreen(client: HistoryClient(f)))));
    await tester.pumpAndSettle();
    await tester.tap(find.textContaining('成功 ／'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('再実行・分岐の入力へ'));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField), '変更した入力');
    await tester.tap(find.text('分岐の承認待ち要求を作成'));
    await tester.pumpAndSettle();
    expect(f.sent!['入力'], '変更した入力');
    expect(find.textContaining('新要求は承認待ちです。'), findsOneWidget);
    expect(f.operations.contains('対話承認'), false);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    await tester.pumpAndSettle();
    expect(find.textContaining('新要求は承認待ちです。'), findsNothing);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pumpWidget(const SizedBox());
  });
  test('製品clientは対応と現在承認を再確認する', () async {
    final f = Fixture();
    final c = HistoryClient(f);
    final grant = (await c.status())!;
    final p = await c.page(grant);
    expect(p.entries.single.state, '成功');
    expect(f.operations, ['対話履歴閲覧状態', '対話履歴閲覧', '対話履歴閲覧状態']);
    f.runtime = 'other';
    await expectLater(c.page(grant), throwsA(isA<BrokerClientException>()));
    f.runtime = 'local';
    f.state = '承認待ち';
    await expectLater(c.page(grant), throwsA(isA<BrokerClientException>()));
    f.state = '成功';
    f.revokeDuringRead = true;
    await expectLater(c.page(grant), throwsA(isA<BrokerClientException>()));
  });
  testWidgets('実履歴表示と失効時の破棄', (tester) async {
    final f = Fixture();
    await tester.pumpWidget(MaterialApp(
        home: Scaffold(body: HistoryScreen(client: HistoryClient(f)))));
    await tester.pumpAndSettle();
    expect(find.textContaining('成功 ／'), findsOneWidget);
    f.revoked = true;
    await tester.pump(const Duration(seconds: 2));
    await tester.pumpAndSettle();
    expect(find.textContaining('成功 ／'), findsNothing);
    expect(f.operations.any((op) => op == '対話履歴承認'), false);
    await tester.pumpWidget(const SizedBox());
  });
  testWidgets('背景化と遅延応答は履歴を再表示しない', (tester) async {
    final f = Fixture();
    f.gate = Completer<void>();
    await tester.pumpWidget(MaterialApp(
        home: Scaffold(body: HistoryScreen(client: HistoryClient(f)))));
    await tester.pump();
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
    await tester.pump();
    f.gate!.complete();
    await tester.pumpAndSettle();
    expect(find.textContaining('成功 ／'), findsNothing);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pumpAndSettle();
    expect(find.textContaining('成功 ／'), findsNothing);
    await tester.pumpWidget(const SizedBox());
  });
}
