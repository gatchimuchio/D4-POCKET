import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_ui/gui_shell_ui.dart';
import 'package:gui_shell_desktop/screens/history_content_dialog.dart';
import 'history_screen_test.dart' show Fixture;

class ContentFixture extends Fixture {
  ContentFixture() {
    withProof = true;
    withContent = true;
  }
  bool contentRevoked = false;
  int contentExpiry = DateTime.now().millisecondsSinceEpoch ~/ 1000 + 50;
  Completer<void>? contentGate, statusGate;
  void Function(Map<String, Object?>)? alter;
  Map<String, Object?> get contentGrant => {
        'approval_id': '7' * 32,
        '要求ID': 'd' * 32,
        '保存監査ID': 'saved',
        '保存監査hash': 'sha256:${'8' * 64}',
        'expires_at': contentExpiry
      };
  @override
  Future<Map<String, Object?>> request(String op,
      {Map<String, Object?>? payload}) async {
    if (!op.startsWith('対話内容')) return super.request(op, payload: payload);
    operations.add(op);
    if (op != '対話内容閲覧' && op != '対話内容閲覧状態') {
      throw StateError('clientが承認操作を発行した');
    }
    if (op == '対話内容閲覧状態' && statusGate != null) await statusGate!.future;
    final body = <String, Object?>{
      'grant': contentRevoked ? null : contentGrant,
      'content': null
    };
    if (op == '対話内容閲覧') {
      final page = await super.request('対話履歴閲覧', payload: {
        'query': {
          'latest_per_request': true,
          'filter': {'実行系ID': 'local'}
        }
      });
      final entry = (((page['body'] as Map)['page'] as Map)['entries'] as List)
          .single as Map;
      body['content'] = {
        '版': 1,
        '要求': {
          '要求ID': 'd' * 32,
          '実行系ID': 'local',
          '対話セッションID': 'e' * 32,
          '入力': '保存された入力'
        },
        '要求hash': 'sha256:${'b' * 64}',
        '実行記録': (entry['record'] as Map)['実行記録'],
        '結果証跡': entry['result_evidence'],
        '結果': {
          '要求ID': 'd' * 32,
          '実行系ID': 'local',
          '対話セッションID': 'e' * 32,
          '状態': '成功',
          '表示範囲': 'full',
          '本文': '保存された応答',
          '参照': <String>[],
          '能力': <String>[],
          '経路': 'local',
          '追跡ID': 'a' * 32,
          '追跡hash': 'sha256:${'a' * 64}',
          '応答hash': 'sha256:${'f' * 64}',
          '失敗分類': '',
          '復旧': ''
        }
      };
      alter?.call(body);
      if (contentGate != null) await contentGate!.future;
    }
    return {
      'operation': op,
      'status': 'accepted',
      'evidence_source': 'INTERNAL_STATE',
      'audit_event_id': 'content-read',
      'body': body
    };
  }
}

Future<HistoryEntry> entry(HistoryClient c) async =>
    (await c.page((await c.status())!)).entries.single;
void main() {
  test('保存内容clientは別対象と未知fieldと途中失効を拒否する', () async {
    final f = ContentFixture();
    final c = HistoryClient(f);
    final e = await entry(c);
    expect(e.receipt!.auditId, 'saved');
    final grant = (await c.contentStatus())!;
    expect((await c.content(e, grant)).input, '保存された入力');
    for (var caseId = 0; caseId < 6; caseId++) {
      f.alter = (body) {
        final content = body['content'] as Map;
        switch (caseId) {
          case 0:
            content['余分'] = '拒否';
          case 1:
            (content['要求'] as Map)['要求ID'] = 'a' * 32;
          case 2:
            content['要求hash'] = 'sha256:${'0' * 64}';
          case 3:
            (content['結果'] as Map)['表示範囲'] = 'hash_only';
          case 4:
            (content['実行記録'] as Map)['終了監査ID'] = 'other';
          case 5:
            f.contentRevoked = true;
        }
      };
      await expectLater(
          c.content(e, grant), throwsA(isA<BrokerClientException>()));
    }
  });
  testWidgets('内容は失効と背景化で消え遅延応答から復活しない', (tester) async {
    for (final background in [false, true]) {
      final f = ContentFixture();
      final c = HistoryClient(f);
      final e = await entry(c);
      if (background) f.contentGate = Completer<void>();
      await tester.pumpWidget(
          MaterialApp(home: HistoryContentDialog(client: c, entry: e)));
      await tester.pumpAndSettle();
      if (background) {
        tester.binding
            .handleAppLifecycleStateChanged(AppLifecycleState.inactive);
        await tester.pump();
        f.contentGate!.complete();
      } else {
        expect(find.text('保存された応答'), findsOneWidget);
        f.contentRevoked = true;
        await tester.pump(const Duration(seconds: 2));
      }
      await tester.pumpAndSettle();
      expect(find.text('保存された応答'), findsNothing);
      expect(find.text('保存された入力'), findsNothing);
      if (background) {
        tester.binding
            .handleAppLifecycleStateChanged(AppLifecycleState.resumed);
      }
      await tester.pumpWidget(const SizedBox());
    }
  });
  testWidgets('実時間の期限を過ぎた本文を消去する', (tester) async {
    final f = ContentFixture();
    final c = HistoryClient(f);
    final e = await entry(c);
    f.contentExpiry = DateTime.now().millisecondsSinceEpoch ~/ 1000 + 5;
    await tester.pumpWidget(
        MaterialApp(home: HistoryContentDialog(client: c, entry: e)));
    await tester.pumpAndSettle();
    expect(find.text('保存された応答'), findsOneWidget);
    await tester
        .runAsync(() => Future<void>.delayed(const Duration(seconds: 5)));
    await tester.pump(const Duration(milliseconds: 150));
    expect(find.text('保存された応答'), findsNothing);
    await tester.pumpWidget(const SizedBox());
  });
  testWidgets('内容の再確認中は本文を表示しない', (tester) async {
    final f = ContentFixture();
    final c = HistoryClient(f);
    final e = await entry(c);
    await tester.pumpWidget(
        MaterialApp(home: HistoryContentDialog(client: c, entry: e)));
    await tester.pumpAndSettle();
    expect(find.text('保存された応答'), findsOneWidget);
    f.statusGate = Completer<void>();
    await tester.pump(const Duration(seconds: 2));
    await tester.pump();
    expect(find.text('保存された応答'), findsNothing);
    f.contentRevoked = true;
    f.statusGate!.complete();
    await tester.pumpAndSettle();
    expect(find.text('保存された応答'), findsNothing);
    await tester.pumpWidget(const SizedBox());
    expect(find.text('保存された応答'), findsNothing);
  });
}
