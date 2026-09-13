import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_ui/gui_shell_ui.dart';

Map<String, Object?> result(String runtime, String session, String request,
        {String scope = 'full', bool failed = false}) =>
    {
      '要求ID': request,
      '実行系ID': runtime,
      '対話セッションID': session,
      '状態': failed ? '失敗' : '成功',
      '表示範囲': scope,
      '本文': failed || scope != 'full' ? '' : '$runtimeの応答',
      '参照': <String>[],
      '能力': <String>[],
      '経路': '',
      '追跡ID': '',
      '追跡hash': '',
      '応答hash': '',
      '失敗分類': failed ? '通信失敗' : '',
      '復旧': failed ? '接続再確認' : '',
    };

class DialogueFixture implements BrokerTransport {
  final calls = <String>[];
  final sessions = <String, String>{};
  final requests = <String, String>{};
  final inputs = <String>[];
  int counter = 0;
  bool complete = false;
  bool failLeft = false;
  bool failRight = false;
  bool swap = false;
  String? failOperation;
  Completer<void>? pollWait;
  @override
  Future<Map<String, Object?>> request(String operation,
      {Map<String, Object?>? payload}) async {
    calls.add(operation);
    if (operation == failOperation) throw const BrokerClientException('試験用失敗');
    final p = payload!;
    Map<String, Object?> body;
    switch (operation) {
      case '実行系列挙':
        body = {
          '実行系': ['left', 'right']
        };
      case '対話開始':
        final id = (++counter).toRadixString(16).padLeft(32, '0');
        sessions[id] = p['実行系ID']! as String;
        body = {'実行系ID': p['実行系ID'], '対話セッションID': id, '状態': '利用中'};
      case '対話送信':
        final id = (++counter).toRadixString(16).padLeft(32, '0');
        requests[id] = p['対話セッションID']! as String;
        inputs.add(p['入力']! as String);
        body = {'要求ID': id, '状態': '承認待ち'};
      case '対話取得':
        if (pollWait != null) await pollWait!.future;
        final id = p['要求ID']! as String;
        final session = requests[id]!;
        final runtime = sessions[session]!;
        body = {
          '要求ID': id,
          '状態': complete ? '完了' : '承認待ち',
          '結果': complete
              ? result(swap ? 'other' : runtime, session, id,
                  failed: runtime == 'left' ? failLeft : failRight)
              : null
        };
      case '対話中止':
        body = {'要求ID': p['要求ID'], '状態': '中止'};
      case '対話終了':
        body = {'対話セッションID': p['対話セッションID'], '状態': '終了'};
      default:
        throw StateError('試験で許可していない操作');
    }
    return {
      'operation': operation,
      'audit_event_id': 'fixture-audit',
      'status': 'accepted',
      'body': body
    };
  }
}

void main() {
  for (final failure in ['対話開始', '対話終了']) {
    testWidgets('新規切替の$failure失敗で終了済み結果と未終了結果を区別する', (tester) async {
      tester.view.physicalSize = const Size(1400, 1000);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final f = DialogueFixture()..complete = true;
      await tester.pumpWidget(MaterialApp(
          home: Scaffold(
              body: RuntimeDialogueScreen(
                  connect: () async => RuntimeDialogueClient(f)))));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextField), '前のセッションの入力');
      await tester.tap(find.byKey(const ValueKey('dialogue-send')));
      await tester.pumpAndSettle();
      await tester.pump(const Duration(seconds: 1));
      await tester.pumpAndSettle();
      expect(find.text('leftの応答'), findsOneWidget);
      f.failOperation = failure;
      f.calls.clear();
      await tester.tap(find.text('新規セッション'));
      await tester.pumpAndSettle();
      expect(find.textContaining('セッションを開始できません'), findsOneWidget);
      if (failure == '対話開始') {
        expect(f.calls, ['対話終了', '対話開始']);
        expect(find.text('leftの応答'), findsNothing);
        expect(find.textContaining('要求: '), findsNothing);
        expect(find.text('状態: 未開始'), findsOneWidget);
      } else {
        expect(f.calls, ['対話終了']);
        expect(find.text('leftの応答'), findsOneWidget);
        expect(find.textContaining('セッション: '), findsOneWidget);
      }
      f.failOperation = null;
      await tester.tap(find.text('新規セッション'));
      await tester.pumpAndSettle();
      expect(find.text('状態: 入力待ち'), findsOneWidget);
      expect(find.text('leftの応答'), findsNothing);
      expect(f.inputs, ['前のセッションの入力']);
      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pumpAndSettle();
    });
  }
  testWidgets('現在待機中の要求の通信失敗は表示する', (tester) async {
    final wait = Completer<void>();
    final f = DialogueFixture()..pollWait = wait;
    await tester.pumpWidget(MaterialApp(
        home: Scaffold(
            body: RuntimeDialogueScreen(
                connect: () async => RuntimeDialogueClient(f)))));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField), '現在の要求');
    await tester.ensureVisible(find.byKey(const ValueKey('dialogue-send')));
    await tester.tap(find.byKey(const ValueKey('dialogue-send')));
    await tester.pumpAndSettle();
    await tester.pump(const Duration(seconds: 1));
    wait.completeError(const BrokerClientException('現在の通信失敗'));
    await tester.pumpAndSettle();
    expect(find.textContaining('応答を取得できません'), findsOneWidget);
    expect(f.inputs, ['現在の要求']);
    await tester.pumpWidget(const SizedBox.shrink());
    await tester.pumpAndSettle();
  });
  for (final failed in [false, true]) {
    testWidgets('中止した旧要求の遅延${failed ? '失敗' : '成功'}を新規セッションに表示しない',
        (tester) async {
      tester.view.physicalSize = const Size(1400, 1000);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final wait = Completer<void>();
      final f = DialogueFixture()
        ..complete = true
        ..pollWait = wait;
      await tester.pumpWidget(MaterialApp(
          home: Scaffold(
              body: RuntimeDialogueScreen(
                  connect: () async => RuntimeDialogueClient(f)))));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextField), '旧要求');
      await tester.tap(find.byKey(const ValueKey('dialogue-send')));
      await tester.pumpAndSettle();
      await tester.pump(const Duration(seconds: 1));
      expect(f.calls.where((v) => v == '対話取得'), hasLength(1));
      await tester.tap(find.text('中止'));
      await tester.pumpAndSettle();
      expect(find.text('状態: 中止'), findsOneWidget);
      await tester.tap(find.text('新規セッション'));
      await tester.pumpAndSettle();
      expect(find.text('状態: 入力待ち'), findsOneWidget);
      if (failed) {
        wait.completeError(const BrokerClientException('旧要求の通信失敗'));
      } else {
        wait.complete();
      }
      await tester.pumpAndSettle();
      expect(find.textContaining('応答を取得できません'), findsNothing);
      expect(find.text('leftの応答'), findsNothing);
      expect(find.text('状態: 入力待ち'), findsOneWidget);
      expect(f.inputs, ['旧要求']);
      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pumpAndSettle();
    });
  }
  testWidgets('停止中は接続せず復帰時に照会だけ再開する', (tester) async {
    final f = DialogueFixture();
    Widget screen(bool active) => MaterialApp(
        home: Scaffold(
            body: RuntimeDialogueScreen(
                connect: () async => RuntimeDialogueClient(f),
                active: active)));
    await tester.pumpWidget(screen(false));
    await tester.pumpAndSettle();
    await tester.pump(const Duration(seconds: 2));
    expect(f.calls, isEmpty);
    await tester.pumpWidget(screen(true));
    await tester.pumpAndSettle();
    expect(f.calls, ['実行系列挙']);
    await tester.pumpWidget(screen(false));
    await tester.pump(const Duration(seconds: 2));
    expect(f.calls, ['実行系列挙']);
    await tester.pumpWidget(const SizedBox.shrink());
  });
  testWidgets('デモ表示は対話を送信しない', (tester) async {
    final f = DialogueFixture();
    await tester.pumpWidget(MaterialApp(
        home: Scaffold(
            body: RuntimeDialogueScreen(
                connect: () async => RuntimeDialogueClient(f),
                client: RuntimeDialogueClient(f),
                readOnly: true))));
    await tester.pumpAndSettle();
    expect(
        tester
            .widget<FilledButton>(find.byKey(const ValueKey('dialogue-send')))
            .onPressed,
        isNull);
    expect(f.calls, ['実行系列挙']);
    await tester.pumpWidget(const SizedBox.shrink());
    await tester.pumpAndSettle();
  });
  test('対話clientが承認操作と空白入力を拒否する', () async {
    final fixture = DialogueFixture();
    final client = RuntimeDialogueClient(fixture);
    await expectLater(
        client.operation('対話承認', {}), throwsA(isA<BrokerClientException>()));
    await expectLater(
        client.send('a' * 32, ' '), throwsA(isA<BrokerClientException>()));
    expect(fixture.calls, isEmpty);
  });
  test('左右の成功と失敗とscopeを分離し別応答を拒否する', () async {
    for (final left in [false, true]) {
      for (final right in [false, true]) {
        final f = DialogueFixture()
          ..complete = true
          ..failLeft = left
          ..failRight = right;
        final c = RuntimeDialogueClient(f);
        final a = await c.start('left');
        final b = await c.start('right');
        final p = await c.send(a, '同一入力');
        final q = await c.send(b, '同一入力');
        expect(a, isNot(b));
        expect(p, isNot(q));
        expect(f.inputs, ['同一入力', '同一入力']);
        expect((await c.poll(p, 'left', a)).result!.text('状態'),
            left ? '失敗' : '成功');
        expect((await c.poll(q, 'right', b)).result!.text('状態'),
            right ? '失敗' : '成功');
        f.swap = true;
        await expectLater(
            c.poll(p, 'left', a), throwsA(isA<BrokerClientException>()));
      }
    }
    for (final scope in ['none', 'hash_only', 'summary', 'redacted']) {
      final v = result('left', 'a' * 32, 'b' * 32, scope: scope);
      expect(
          DialogueResult.parse(v, 'b' * 32, 'left', 'a' * 32).text('本文'), '');
      v['本文'] = '漏洩本文';
      expect(() => DialogueResult.parse(v, 'b' * 32, 'left', 'a' * 32),
          throwsA(isA<BrokerClientException>()));
    }
  });
  testWidgets('画面から同一入力を左右へ送り片側失敗を独立表示する', (tester) async {
    tester.view.physicalSize = const Size(1400, 1000);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final f = DialogueFixture();
    await tester.pumpWidget(MaterialApp(
        home: Scaffold(
            body: RuntimeDialogueScreen(
                connect: () async => RuntimeDialogueClient(f),
                client: RuntimeDialogueClient(f)))));
    await tester.pumpAndSettle();
    await tester.tap(find.byType(Switch));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField), '同じ質問');
    await tester.tap(find.byKey(const ValueKey('dialogue-send')));
    await tester.pumpAndSettle();
    expect(f.inputs, ['同じ質問', '同じ質問']);
    expect(f.calls.where((op) => op == '対話承認'), isEmpty);
    expect(find.text('状態: 承認待ち'), findsNWidgets(2));
    f.complete = true;
    f.failRight = true;
    await tester.pump(const Duration(seconds: 1));
    await tester.pumpAndSettle();
    expect(find.text('leftの応答'), findsOneWidget);
    expect(find.textContaining('失敗: 通信失敗'), findsOneWidget);
    expect(find.text('rightの応答'), findsNothing);
    await tester.pumpWidget(const SizedBox.shrink());
    await tester.pumpAndSettle();
  });
  testWidgets('中止は要求だけに結合し再送は自動で行わない', (tester) async {
    final f = DialogueFixture();
    await tester.pumpWidget(MaterialApp(
        home: Scaffold(
            body: RuntimeDialogueScreen(
                connect: () async => RuntimeDialogueClient(f),
                client: RuntimeDialogueClient(f)))));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField), 'こんにちは');
    await tester.ensureVisible(find.byKey(const ValueKey('dialogue-send')));
    await tester.tap(find.byKey(const ValueKey('dialogue-send')));
    await tester.pumpAndSettle();
    await tester.ensureVisible(find.text('中止'));
    await tester.tap(find.text('中止'));
    await tester.pumpAndSettle();
    expect(find.text('状態: 中止'), findsOneWidget);
    expect(f.calls.where((op) => op == '対話送信').length, 1);
    await tester.pumpWidget(const SizedBox.shrink());
    await tester.pumpAndSettle();
  });
}
