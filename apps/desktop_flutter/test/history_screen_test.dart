import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_ui/gui_shell_ui.dart';
import 'package:gui_shell_desktop/screens/history_screen.dart';

class Fixture implements BrokerTransport {
  bool revoked = false;
  bool withProof = false;
  bool withContent = false;
  Map<String, Object?>? sent;
  Map? lastFilter;
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
      lastFilter = (payload['query'] as Map)['filter'] as Map;
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
            'content_receipt': withContent
                ? {
                    'audit_event_id': 'saved',
                    'event_hash': 'sha256:${'8' * 64}',
                    'receipt': {
                      '版': 1,
                      '要求ID': 'd' * 32,
                      '対話セッションID': 'e' * 32,
                      '実行系ID': runtime,
                      '要求hash': 'sha256:${'b' * 64}',
                      '終了監査ID': 'ended',
                      '保存承認監査ID': 'save-approved',
                      '暗号文hash': 'sha256:${'9' * 64}',
                      '証拠種別': 'INTERNAL_STATE'
                    }
                  }
                : null,
            'result_evidence': withProof
                ? {
                    '版': 1,
                    '要求ID': 'd' * 32,
                    '対話セッションID': 'e' * 32,
                    '実行系ID': runtime,
                    '要求hash': 'sha256:${'b' * 64}',
                    '終了監査ID': 'ended',
                    '表示範囲': 'full',
                    '応答hash': 'sha256:${'f' * 64}',
                    '能力申告hash': 'sha256:${'a' * 64}',
                    '経路申告hash': 'sha256:${'a' * 64}',
                    '追跡参照hash': 'sha256:${'a' * 64}',
                    '証拠種別': 'INTERNAL_STATE'
                  }
                : null,
            'audit_context': {
              '版': 1,
              '要求hash': 'sha256:${'b' * 64}',
              '作成操作': '対話送信',
              '承認': {
                '監査ID': 'started',
                '操作': '対話承認',
                '要求hash': 'sha256:${'b' * 64}',
                '内容表示範囲': 'full'
              },
              '承認能力': ['対話送信'],
              '復旧対応': '接続再確認',
              '現在権限': false
            },
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
  testWidgets('結果証跡のhashを表示し失効後は破棄する', (tester) async {
    final f = Fixture()..withProof = true;
    await tester.pumpWidget(MaterialApp(
        home: Scaffold(body: HistoryScreen(client: HistoryClient(f)))));
    await tester.pumpAndSettle();
    await tester.tap(find.byType(ExpansionTile));
    await tester.pumpAndSettle();
    expect(find.textContaining('応答hash: sha256:${'f' * 64}'), findsOneWidget);
    expect(find.textContaining('実使用証明ではありません'), findsOneWidget);
    f.revoked = true;
    await tester.pump(const Duration(seconds: 3));
    await tester.pumpAndSettle();
    expect(find.textContaining('応答hash: sha256:${'f' * 64}'), findsNothing);
    await tester.pumpWidget(const SizedBox());
  });
  test('結果証跡は要求対応と表示範囲とhashを照合する', () async {
    final c = HistoryClient(Fixture());
    final e = (await c.page((await c.status())!)).entries.single;
    Map<String, Object?> proof() => {
          '版': 1,
          '要求ID': 'd' * 32,
          '対話セッションID': 'e' * 32,
          '実行系ID': 'local',
          '要求hash': e.context.requestHash,
          '終了監査ID': 'ended',
          '表示範囲': 'full',
          '応答hash': e.context.requestHash,
          '能力申告hash': e.context.requestHash,
          '経路申告hash': e.context.requestHash,
          '追跡参照hash': e.context.requestHash,
          '証拠種別': 'INTERNAL_STATE'
        };
    expect(
        HistoryResultEvidence.parse(proof(), e.record, e.context, e.state)!
            .responseHash,
        e.context.requestHash);
    expect(HistoryResultEvidence.parse(null, e.record, e.context, e.state),
        isNull);
    for (final key in [
      '要求ID',
      '対話セッションID',
      '実行系ID',
      '要求hash',
      '終了監査ID',
      '表示範囲',
      '応答hash',
      '能力申告hash',
      '経路申告hash',
      '追跡参照hash',
      '証拠種別',
      '本文'
    ]) {
      final bad = proof();
      bad[key] = '不一致';
      expect(
          () => HistoryResultEvidence.parse(bad, e.record, e.context, e.state),
          throwsA(isA<BrokerClientException>()));
    }
    expect(
        () => HistoryResultEvidence.parse(proof(), e.record, e.context, '失敗'),
        throwsA(isA<BrokerClientException>()));
  });
  test('監査文脈は現在権限と異要求の承認を拒否する', () async {
    final c = HistoryClient(Fixture());
    final grant = (await c.status())!;
    final entry = (await c.page(grant)).entries.single;
    Map<String, Object?> context() => {
          '版': 1,
          '要求hash': 'sha256:${'b' * 64}',
          '作成操作': '対話送信',
          '承認': {
            '監査ID': 'started',
            '操作': '対話承認',
            '要求hash': 'sha256:${'b' * 64}',
            '内容表示範囲': 'full'
          },
          '承認能力': ['対話送信'],
          '復旧対応': '接続再確認',
          '現在権限': false
        };
    expect(HistoryAuditContext.parse(context(), entry.record).approvalId,
        'started');
    for (final field in ['現在権限', '要求hash', '承認能力', '承認', '復旧対応']) {
      final bad = context();
      bad[field] = switch (field) {
        '現在権限' => true,
        '要求hash' => 'sha256:${'c' * 64}',
        '承認能力' => ['shell'],
        '承認' => null,
        _ => '任意実行'
      };
      expect(() => HistoryAuditContext.parse(bad, entry.record),
          throwsA(isA<BrokerClientException>()));
    }
  });
  test('要求とSession検索は完全一致で応答も照合する', () async {
    final f = Fixture();
    final c = HistoryClient(f);
    final grant = (await c.status())!;
    final p = await c.page(grant, requestId: 'd' * 32, sessionId: 'e' * 32);
    expect(p.entries.length, 1);
    expect(f.lastFilter,
        {'実行系ID': 'local', '要求ID': 'd' * 32, '対話セッションID': 'e' * 32});
    await expectLater(c.page(grant, requestId: 'f' * 32),
        throwsA(isA<BrokerClientException>()));
    await expectLater(c.page(grant, sessionId: 'f' * 32),
        throwsA(isA<BrokerClientException>()));
    final calls = f.operations.length;
    await expectLater(c.page(grant, requestId: 'invalid'),
        throwsA(isA<BrokerClientException>()));
    expect(f.operations.length, calls);
  });
  testWidgets('検索条件は要求とSessionを組み合わせ解除できる', (tester) async {
    final f = Fixture();
    await tester.pumpWidget(MaterialApp(
        home: Scaffold(body: HistoryScreen(client: HistoryClient(f)))));
    await tester.pumpAndSettle();
    await tester.enterText(
        find.widgetWithText(TextField, '要求ID（完全一致）'), 'd' * 32);
    await tester.enterText(
        find.widgetWithText(TextField, 'Session ID（完全一致）'), 'e' * 32);
    await tester.tap(find.text('検索'));
    await tester.pumpAndSettle();
    expect(f.lastFilter!['要求ID'], 'd' * 32);
    expect(f.lastFilter!['対話セッションID'], 'e' * 32);
    await tester.tap(find.text('検索条件を解除'));
    await tester.pumpAndSettle();
    expect(f.lastFilter, {'実行系ID': 'local'});
    await tester.pumpWidget(const SizedBox());
  });
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
    f.gate = Completer<void>();
    await tester.pump(const Duration(seconds: 2));
    await tester.pump();
    f.gate!.complete();
    await tester.pumpAndSettle();
    expect(find.text('再実行・分岐の入力へ').hitTestable(), findsOneWidget);
    await tester.tap(find.text('再実行・分岐の入力へ'));
    await tester.pumpAndSettle();
    await tester.enterText(
        find.byKey(const ValueKey('history-replay-input')), '変更した入力');
    f.gate = Completer<void>();
    await tester.pump(const Duration(seconds: 2));
    await tester.pump();
    f.gate!.complete();
    await tester.pumpAndSettle();
    final field = tester
        .widget<TextField>(find.byKey(const ValueKey('history-replay-input')));
    expect(field.controller!.text, '変更した入力');
    expect(field.focusNode!.hasFocus, true);
    await tester.tap(find.text('分岐の承認待ち要求を作成'));
    await tester.pumpAndSettle();
    expect(f.sent!['入力'], '変更した入力');
    expect(find.textContaining('新要求は承認待ちです。'), findsOneWidget);
    expect(find.textContaining('新Session: ${'1' * 32}'), findsOneWidget);
    expect(find.textContaining('作成種別: 対話分岐'), findsOneWidget);
    expect(find.textContaining('参照元監査: history-1'), findsOneWidget);
    expect(find.textContaining('参照元監査hash: sha256:${'c' * 64}'), findsOneWidget);
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
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
    await tester.pump();
    f.gate!.complete();
    await tester.pumpAndSettle();
    expect(find.textContaining('成功 ／'), findsNothing);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pumpAndSettle();
    expect(find.textContaining('成功 ／'), findsNothing);
    await tester.pumpWidget(const SizedBox());
  });
}
