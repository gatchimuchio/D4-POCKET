import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_ui/gui_shell_ui.dart';
import 'package:gui_shell_ui/regression_case_client.dart';

final _dialogueRequestHash = 'sha256:${'b' * 64}';

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
  final startPayloads = <Map<String, Object?>>[];
  List<Map<String, Object?>> workspaceRegistrations = [
    {
      '作業領域ID': 'workspace-left',
      '実行系ID': 'left',
      '登録hash': 'sha256:${'a' * 64}',
      '承認状態': 'denied',
      '有効期限': null,
      '表示範囲': 'none',
      'approval_id': null,
    },
    {
      '作業領域ID': 'workspace-right',
      '実行系ID': 'right',
      '登録hash': 'sha256:${'b' * 64}',
      '承認状態': 'approved',
      '有効期限': 1780000300,
      '表示範囲': 'full',
      'approval_id': 'approval-workspace-right',
    },
  ];
  List<String> runtimeNames = ['left', 'right'];
  int counter = 0;
  bool complete = false;
  bool failLeft = false;
  bool failRight = false;
  bool swap = false;
  String? failOperation;
  String? progressState;
  String? sendHashOverride;
  bool omitSendHash = false;
  bool omitSendExpiry = false;
  bool addSendAuthority = false;
  Map<String, Object?>? resultOverride;
  bool includeRecord = false;
  Map<String, Object?>? registrationResponse;
  Map<String, Object?>? registrationPayload;
  void Function(Map<String, Object?>)? mutateRecord;
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
        body = {'実行系': runtimeNames};
      case '作業領域一覧':
        body = {'作業領域': workspaceRegistrations};
      case '対話開始':
        startPayloads.add(Map<String, Object?>.from(p));
        final id = (++counter).toRadixString(16).padLeft(32, '0');
        sessions[id] = p['実行系ID']! as String;
        body = {'実行系ID': p['実行系ID'], '対話セッションID': id, '状態': '利用中'};
      case '対話送信':
        final id = (++counter).toRadixString(16).padLeft(32, '0');
        requests[id] = p['対話セッションID']! as String;
        inputs.add(p['入力']! as String);
        body = {
          '要求ID': id,
          if (!omitSendHash) '要求hash': sendHashOverride ?? _dialogueRequestHash,
          '状態': '承認待ち',
          if (!omitSendExpiry) '期限': 1780000300,
          if (addSendAuthority) 'owner': true,
        };
      case '対話取得':
        if (pollWait != null) await pollWait!.future;
        final id = p['要求ID']! as String;
        final session = requests[id]!;
        final runtime = sessions[session]!;
        body = {
          '要求ID': id,
          '状態': progressState ?? (complete ? '完了' : '承認待ち'),
          '結果': complete
              ? resultOverride ??
                  result(swap ? 'other' : runtime, session, id,
                      failed: runtime == 'left' ? failLeft : failRight)
              : null
        };
        if (includeRecord) {
          final record = <String, Object?>{
            '要求ID': id,
            '実行系ID': runtime,
            '対話セッションID': session,
            '作成時刻': 100,
            '開始時刻': complete ? 110 : null,
            '終了時刻': complete ? 120 : null,
            '作成監査ID': 'created',
            '開始監査ID': complete ? 'started' : null,
            '終了監査ID': complete ? 'finished' : null,
          };
          mutateRecord?.call(record);
          body['実行記録'] = record;
        }
      case '回帰Case登録':
        registrationPayload = Map<String, Object?>.from(p);
        body = registrationResponse ??
            (throw StateError('回帰Case登録response fixtureがありません'));
      case '対話中止':
        body = {'要求ID': p['要求ID'], '状態': '中止'};
      case '対話終了':
        body = {'対話セッションID': p['対話セッションID'], '状態': '終了'};
      default:
        throw StateError('試験で許可していない操作');
    }
    return {
      'request_id': 'fixture-request',
      'operation': operation,
      'audit_event_id': 'fixture-audit',
      'status': 'accepted',
      'evidence_source': 'INTERNAL_STATE',
      'error': null,
      'health': null,
      'body': body,
      'shutdown_requested': false,
    };
  }
}

void main() {
  test('実行記録の対応・型・時刻と監査の組を検査し旧応答と区別する', () async {
    final f = DialogueFixture()..complete = true;
    final client = RuntimeDialogueClient(f);
    final session = await client.start('left');
    final reference = await client.sendReference(session, '試験');
    final request = reference.requestId;
    expect(reference.requestHash, _dialogueRequestHash);
    expect((await client.poll(request, 'left', session)).record, isNull);
    f.includeRecord = true;
    final record = (await client.poll(request, 'left', session)).record!;
    expect(record.time('開始時刻'), '1970-01-01T00:01:50.000Z');
    expect(record.audit('終了監査ID'), 'finished');
    expect(() => record.fields['開始時刻'] = 0, throwsUnsupportedError);
    for (final mutate in <void Function(Map<String, Object?>)>[
      (v) => v['要求ID'] = 'f' * 32,
      (v) => v['実行系ID'] = 'other',
      (v) => v['対話セッションID'] = 'f' * 32,
      (v) => v['開始時刻'] = 110.0,
      (v) => v['終了時刻'] = 8640000000001,
      (v) => v['終了時刻'] = -9223372036854775808,
      (v) => v['開始監査ID'] = null,
      (v) => v['終了監査ID'] = '',
      (v) => v['作成監査ID'] = '改行\n混入',
      (v) => v['作成監査ID'] = 'a' * 257,
      (v) => v['owner'] = true,
    ]) {
      f.mutateRecord = mutate;
      await expectLater(client.poll(request, 'left', session),
          throwsA(isA<BrokerClientException>()));
    }
    f.mutateRecord = (v) => v['終了時刻'] = 90;
    expect((await client.poll(request, 'left', session)).record!.fields['終了時刻'],
        90);
    expect(() => DialogueExecutionRecord.parse(null, request, 'left', session),
        throwsA(isA<BrokerClientException>()));
  });

  test('Workspace一覧はBrokerの登録metadataだけをRuntime別に限定して返す', () async {
    final f = DialogueFixture();
    final workspaces = await RuntimeDialogueClient(f).workspaceIdsByRuntime();
    expect(workspaces, {
      'left': ['workspace-left'],
      'right': ['workspace-right'],
    });
    expect(f.calls, ['作業領域一覧']);
  });

  test('対話開始は選択したWorkspace IDだけを追加し未指定Runtimeの互換性を保つ', () async {
    final f = DialogueFixture();
    final client = RuntimeDialogueClient(f);
    await client.start('left');
    await client.start('right', workspaceId: 'workspace-right');
    expect(f.startPayloads, [
      {'実行系ID': 'left'},
      {'実行系ID': 'right', '作業領域ID': 'workspace-right'},
    ]);
  });

  testWidgets('選択した登録Workspace IDを新規Session開始へ渡す', (tester) async {
    final f = DialogueFixture();
    await tester.pumpWidget(MaterialApp(
        home: Scaffold(
            body: RuntimeDialogueScreen(
                connect: () async => RuntimeDialogueClient(f)))));
    await tester.pumpAndSettle();
    final workspacePicker =
        find.byKey(const ValueKey('dialogue-workspace-left'));
    expect(workspacePicker, findsOneWidget);
    await tester.ensureVisible(workspacePicker);
    await tester.tap(workspacePicker);
    await tester.pumpAndSettle();
    await tester.tap(find.text('workspace-left').last);
    await tester.pumpAndSettle();
    await tester.ensureVisible(find.text('新規セッション'));
    await tester.tap(find.text('新規セッション').first);
    await tester.pumpAndSettle();
    expect(f.startPayloads, [
      {'実行系ID': 'left', '作業領域ID': 'workspace-left'},
    ]);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('Workspace選択に未対応の接続面はWorkspace一覧を要求しない', (tester) async {
    final f = DialogueFixture();
    await tester.pumpWidget(MaterialApp(
        home: Scaffold(
            body: RuntimeDialogueScreen(
      connect: () async => RuntimeDialogueClient(f),
      workspaceSelectionSupported: false,
    ))));
    await tester.pumpAndSettle();
    expect(f.calls, ['実行系列挙']);
    expect(find.byKey(const ValueKey('dialogue-workspace-left')), findsNothing);
    expect(find.textContaining('Workspace選択に未対応'), findsOneWidget);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('実行記録をUTCと監査参照で表示し新規セッションで破棄する', (tester) async {
    final f = DialogueFixture()..includeRecord = true;
    await tester.pumpWidget(MaterialApp(
        home: Scaffold(
            body: RuntimeDialogueScreen(
                connect: () async => RuntimeDialogueClient(f)))));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField), '試験入力');
    await tester.ensureVisible(find.byKey(const ValueKey('dialogue-send')));
    await tester.tap(find.byKey(const ValueKey('dialogue-send')));
    await tester.pumpAndSettle();
    await tester.pump(const Duration(seconds: 1));
    await tester.pumpAndSettle();
    await tester.ensureVisible(find.text('実行記録'));
    await tester.tap(find.text('実行記録'));
    await tester.pumpAndSettle();
    expect(find.text('開始: 未記録'), findsOneWidget);
    f.complete = true;
    await tester.pump(const Duration(seconds: 1));
    await tester.pumpAndSettle();
    expect(find.text('終了: 1970-01-01T00:02:00.000Z'), findsOneWidget);
    expect(find.text('終了監査: finished'), findsOneWidget);
    await tester.ensureVisible(find.text('新規セッション'));
    await tester.tap(find.text('新規セッション'));
    await tester.pumpAndSettle();
    expect(find.text('終了監査: finished'), findsNothing);
    expect(f.calls.where((v) => v == '対話送信').length, 1);
    expect(f.calls.where((v) => v == '対話承認'), isEmpty);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('完了対話からownerが明示記入したredacted Caseだけを登録する', (tester) async {
    const requestId = '00000000000000000000000000000002';
    const caseId = 'cccccccccccccccccccccccccccccccc';
    final f = DialogueFixture()
      ..complete = true
      ..includeRecord = true
      ..registrationResponse = {
        '版': 1,
        '回帰CaseID': caseId,
        '定義hash': 'sha256:${'d' * 64}',
        '非公開保管ID': caseId,
        '暗号文hash': 'sha256:${'e' * 64}',
        '公開表示名': 'public sanitized case',
        '要求ID': requestId,
        '要求hash': _dialogueRequestHash,
        '実行系ID': 'left',
        '結果状態': '成功',
        '応答hash': 'sha256:${'f' * 64}',
        '終了監査ID': 'finished',
        '公開範囲': 'hash_only',
        '必要条件数': 1,
        '禁止条件数': 0,
        '必要参照数': 0,
        '作成時刻UnixMillis': 1780000000000,
        '作成監査ID': 'audit.case.created',
        '証拠種別': 'INTERNAL_STATE',
      };
    final owner = RegressionCaseOwnerClient(f);
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
        body: RuntimeDialogueScreen(
          connect: () async => RuntimeDialogueClient(f),
          connectOwnerRegistration: () async => owner,
        ),
      ),
    ));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField), 'original-dialogue-input');
    await tester.ensureVisible(find.byKey(const ValueKey('dialogue-send')));
    await tester.tap(find.byKey(const ValueKey('dialogue-send')));
    await tester.pumpAndSettle();
    await tester.pump(const Duration(seconds: 1));
    await tester.pumpAndSettle();

    final register =
        find.byKey(const ValueKey('dialogue-register-case-$requestId'));
    expect(register, findsOneWidget);
    await tester.ensureVisible(register);
    await tester.tap(register);
    await tester.pumpAndSettle();
    expect(
      tester
          .widget<TextFormField>(
            find.byKey(const ValueKey('regression-registration-input')),
          )
          .controller!
          .text,
      isEmpty,
    );
    await tester.enterText(
      find.byKey(const ValueKey('regression-registration-name')),
      'public sanitized case',
    );
    await tester.enterText(
      find.byKey(const ValueKey('regression-registration-input')),
      'rewritten sanitized input',
    );
    await tester.enterText(
      find.byKey(const ValueKey('regression-registration-required')),
      'must remain bounded',
    );
    await tester.enterText(
      find.byKey(const ValueKey('regression-registration-route')),
      'runtime-to-result',
    );
    await tester.ensureVisible(
      find.byKey(const ValueKey('regression-registration-submit')),
    );
    await tester
        .tap(find.byKey(const ValueKey('regression-registration-submit')));
    await tester.pumpAndSettle();

    expect(f.registrationPayload, {
      '版': 1,
      '要求ID': requestId,
      '要求hash': _dialogueRequestHash,
      '公開表示名': 'public sanitized case',
      '入力方式': 'owner_explicit_redacted',
      '入力': {
        '内容表示範囲': 'full',
        '本文': 'rewritten sanitized input',
      },
      '必要条件': ['must remain bounded'],
      '禁止条件': <String>[],
      '期待状態': '成功',
      '必要参照': <String>[],
      '期待経路': 'runtime-to-result',
    });
    expect(find.textContaining('登録済み回帰事例: $caseId'), findsOneWidget);
    expect(find.textContaining('rewritten sanitized input'), findsNothing);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('full表示でない対話には登録操作を出さない', (tester) async {
    final f = DialogueFixture()
      ..complete = true
      ..includeRecord = true
      ..resultOverride = result(
        'left',
        '00000000000000000000000000000001',
        '00000000000000000000000000000002',
        scope: 'summary',
      );
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
        body: RuntimeDialogueScreen(
          connect: () async => RuntimeDialogueClient(f),
          connectOwnerRegistration: () async => RegressionCaseOwnerClient(f),
        ),
      ),
    ));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField), 'input');
    await tester.ensureVisible(find.byKey(const ValueKey('dialogue-send')));
    await tester.tap(find.byKey(const ValueKey('dialogue-send')));
    await tester.pumpAndSettle();
    await tester.pump(const Duration(seconds: 1));
    await tester.pumpAndSettle();
    expect(
        find.byKey(const ValueKey(
          'dialogue-register-case-00000000000000000000000000000002',
        )),
        findsNothing);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('終了監査記録がない対話には登録操作を出さない', (tester) async {
    final f = DialogueFixture()
      ..complete = true
      ..includeRecord = true
      ..mutateRecord = (record) {
        record['終了時刻'] = null;
        record['終了監査ID'] = null;
      };
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
        body: RuntimeDialogueScreen(
          connect: () async => RuntimeDialogueClient(f),
          connectOwnerRegistration: () async => RegressionCaseOwnerClient(f),
        ),
      ),
    ));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField), 'input');
    await tester.ensureVisible(find.byKey(const ValueKey('dialogue-send')));
    await tester.tap(find.byKey(const ValueKey('dialogue-send')));
    await tester.pumpAndSettle();
    await tester.pump(const Duration(seconds: 1));
    await tester.pumpAndSettle();
    expect(
      find.byKey(const ValueKey(
        'dialogue-register-case-00000000000000000000000000000002',
      )),
      findsNothing,
    );
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('重複実行系を選択欄へ渡さず接続エラーを表示する', (tester) async {
    final f = DialogueFixture()..runtimeNames = ['left', 'left'];
    await tester.pumpWidget(MaterialApp(
        home: Scaffold(
            body: RuntimeDialogueScreen(
                connect: () async => RuntimeDialogueClient(f)))));
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
    expect(find.textContaining('brokerに接続できません'), findsOneWidget);
    expect(
        tester
            .widget<FilledButton>(find.byKey(const ValueKey('dialogue-send')))
            .onPressed,
        isNull);
    expect(f.calls, ['実行系列挙']);
    await tester.pumpWidget(const SizedBox.shrink());
  });
  test('実行系列挙の重複を拒否し一意な順序と空一覧を保持する', () async {
    final f = DialogueFixture();
    final client = RuntimeDialogueClient(f);
    for (final names in [
      ['left', 'left'],
      ['left', 'right', 'left']
    ]) {
      f.runtimeNames = names;
      await expectLater(
          client.runtimes(), throwsA(isA<BrokerClientException>()));
    }
    for (final names in [
      <String>[],
      ['right', 'left']
    ]) {
      f.runtimeNames = names;
      expect(await client.runtimes(), names);
    }
  });
  test('owner登録用の要求hashが欠落・不正なら要求参照を返さない', () async {
    for (final fixture in [
      DialogueFixture()..omitSendHash = true,
      DialogueFixture()..sendHashOverride = 'sha256:wrong',
      DialogueFixture()..omitSendExpiry = true,
      DialogueFixture()..addSendAuthority = true,
    ]) {
      final client = RuntimeDialogueClient(fixture);
      final session = await client.start('left');
      await expectLater(
        client.sendReference(session, 'sanitized test'),
        throwsA(isA<BrokerClientException>()),
      );
    }
  });
  test('検証後の元配列の変更を表示結果へ反映しない', () {
    for (final scope in ['none', 'full']) {
      final raw = result('left', 'a' * 32, 'b' * 32, scope: scope);
      final parsed = DialogueResult.parse(raw, 'b' * 32, 'left', 'a' * 32);
      for (final key in ['参照', '能力']) {
        (raw[key] as List<String>).add('検証後に追加された未検証内容');
        expect(parsed.list(key), isEmpty);
      }
    }
  });
  test('公開済み結果の配列も変更できず検証済み内容を保持する', () {
    final raw = result('left', 'a' * 32, 'b' * 32)
      ..['参照'] = ['公開参照']
      ..['能力'] = ['公開能力'];
    final parsed = DialogueResult.parse(raw, 'b' * 32, 'left', 'a' * 32);
    for (final key in ['参照', '能力']) {
      final original = List<String>.of(parsed.list(key));
      expect(() => parsed.list(key).clear(), throwsUnsupportedError);
      expect(() => (parsed.fields[key] as List).add('差替え'),
          throwsUnsupportedError);
      (raw[key] as List).clear();
      expect(parsed.list(key), original);
    }
  });
  test('中止進捗に成功本文が混在した応答を拒否する', () async {
    final f = DialogueFixture()
      ..complete = true
      ..progressState = '中止';
    final client = RuntimeDialogueClient(f);
    final session = await client.start('left');
    final request = await client.send(session, '試験入力');
    await expectLater(client.poll(request, 'left', session),
        throwsA(isA<BrokerClientException>()));
    for (final state in ['保留', '失敗']) {
      f.resultOverride = {
        ...result('left', session, request, failed: state == '失敗'),
        '状態': state,
      };
      await expectLater(client.poll(request, 'left', session),
          throwsA(isA<BrokerClientException>()));
    }
    f.resultOverride = {
      ...result('left', session, request, scope: 'none'),
      '状態': '中止',
      '失敗分類': '取消',
      '復旧': '新規セッション',
    };
    final cancelled = await client.poll(request, 'left', session);
    expect(cancelled.state, '中止');
    expect(cancelled.result!.text('状態'), '中止');
    expect(cancelled.result!.text('本文'), isEmpty);
  });
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
    expect(f.calls, ['実行系列挙', '作業領域一覧']);
    await tester.pumpWidget(screen(false));
    await tester.pump(const Duration(seconds: 2));
    expect(f.calls, ['実行系列挙', '作業領域一覧']);
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
    expect(f.calls, ['実行系列挙', '作業領域一覧']);
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
