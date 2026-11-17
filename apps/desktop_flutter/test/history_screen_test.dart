import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_ui/gui_shell_ui.dart';
import 'package:gui_shell_desktop/screens/history_screen.dart';

class Fixture implements BrokerTransport {
  bool revoked = false;
  bool failStatus = false;
  bool withProof = false;
  bool withContent = false;
  bool withInputSummary = false;
  Map<String, Object?>? sent;
  Map? lastFilter;
  void Function(Map<String, Object?>)? alterReplay;
  void Function(Map<String, Object?>)? alterTaskBranch;
  void Function(Map<String, Object?>)? alterRecord;
  bool revokeDuringRead = false;
  String runtime = 'local';
  String taskRuntime = 'local';
  String taskWorkspace = 'workspace-a';
  void Function(Map<String, Object?>)? alterTaskRecord;
  String state = '成功';
  Completer<void>? gate;
  final operations = <String>[];
  final taskRequests = <String, Map<String, Object?>>{};
  int expiry = DateTime.now().millisecondsSinceEpoch ~/ 1000 + 290;
  Map<String, Object?> get grant =>
      {'approval_id': 'a' * 32, 'runtime_id': runtime, 'expires_at': expiry};

  Map<String, Object?> historyRecord() {
    final record = <String, Object?>{
      '版': withInputSummary ? 2 : 1,
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
      },
      if (withInputSummary)
        '入力概要': {'表示範囲': 'hash_only', '入力hash': 'sha256:${'1' * 64}'}
    };
    alterRecord?.call(record);
    return record;
  }

  @override
  Future<Map<String, Object?>> request(String op,
      {Map<String, Object?>? payload}) async {
    operations.add(op);
    if (op == '対話履歴閲覧状態' && failStatus) {
      throw const BrokerClientException('fixture failure');
    }
    if (op == '対話履歴閲覧') {
      expect((payload!['query'] as Map)['latest_per_request'], true);
      lastFilter = (payload['query'] as Map)['filter'] as Map;
    }
    if (op == 'AgentTask履歴閲覧') {
      final query = payload!['query'] as Map;
      expect(query.keys.toSet(), {'after', 'limit'});
      if (revokeDuringRead) revoked = true;
      final record = <String, Object?>{
        'version': 1,
        'task_id': 'a' * 32,
        'runtime_id': taskRuntime,
        'session_id': 'b' * 32,
        'workspace_id': taskWorkspace,
        'instruction_hash': 'sha256:${'1' * 64}',
        'status': 'completed',
        'created_at': 100,
        'updated_at': 101,
        'result_hash': 'sha256:${'2' * 64}',
        'failure_class': null,
        'start_audit_event_id': 'task-start',
        'latest_audit_event_id': 'task-finish'
      };
      alterTaskRecord?.call(record);
      return {
        'operation': op,
        'status': 'accepted',
        'evidence_source': 'INTERNAL_STATE',
        'audit_event_id': 'task-page-audit',
        'body': {
          'grant': revoked ? null : grant,
          'task_page': {
            'version': 1,
            'entries': [
              {
                'audit_event_id': 'task-finish',
                'event_hash': 'sha256:${'3' * 64}',
                'record': record
              }
            ],
            'next_cursor': 1,
            'has_more': false,
            'head_hash': 'sha256:${'4' * 64}'
          }
        }
      };
    }
    if (op == '対話再実行' || op == '対話分岐') {
      sent = payload;
      final isTaskBranch =
          op == '対話分岐' && payload?.containsKey('source_task_id') == true;
      final body = <String, Object?>{
        '要求ID': 'f' * 32,
        '要求hash': isTaskBranch ? 'sha256:${'5' * 64}' : 'sha256:${'0' * 64}',
        '状態': isTaskBranch ? 'Task開始前' : '承認待ち',
        '期限': expiry,
        '対話セッションID': '1' * 32,
        '実行系ID': 'local',
        '参照監査ID': isTaskBranch ? payload!['参照監査ID'] : 'history-1',
        '参照event_hash':
            isTaskBranch ? payload!['参照event_hash'] : 'sha256:${'c' * 64}',
        '種別': isTaskBranch ? 'AgentTask分岐' : op,
        if (isTaskBranch) ...{
          'source_task_id': payload!['source_task_id'],
          '作業領域ID': 'workspace-a'
        }
      };
      if (isTaskBranch) {
        alterTaskBranch?.call(body);
      } else {
        alterReplay?.call(body);
      }
      return {
        'operation': op,
        'status': 'accepted',
        'evidence_source': 'INTERNAL_STATE',
        'audit_event_id': isTaskBranch ? 'task-branch-audit' : 'replay-audit',
        'body': body
      };
    }
    if (op == 'Agent作業要求検査') {
      taskRequests[op] = payload ?? const {};
      return {
        'operation': op,
        'status': 'accepted',
        'evidence_source': 'INTERNAL_STATE',
        'audit_event_id': 'task-inspect-audit',
        'body': {
          '版': 1,
          '状態': '要求検査済み',
          '実行状態': '未実行',
          'Permission状態': '未付与',
          'Approval状態': '未取得',
          '実行系ID': 'local',
          '対話セッションID': '1' * 32,
          '作業領域ID': 'workspace-a',
          '指示hash': 'sha256:${'5' * 64}'
        }
      };
    }
    if (op == 'AgentTaskWorkspacePermissionGrant') {
      taskRequests[op] = payload ?? const {};
      return {
        'operation': op,
        'status': 'accepted',
        'evidence_source': 'INTERNAL_STATE',
        'audit_event_id': 'task-permission-audit',
        'body': {
          'permission_id': '2' * 32,
          'agent_runtime_id': 'local',
          'session_id': '1' * 32,
          'workspace_id': 'workspace-a',
          'workspace_registration_hash': 'sha256:${'6' * 64}',
          'operation': 'agent_task.execute',
          'scope': 'session_workspace_once',
          'decision': 'allow',
          'source': 'owner',
          'expires_at_epoch_seconds': expiry,
          'use_limit': 1,
          'uses_remaining': 1,
          'status': 'active'
        }
      };
    }
    if (op == 'AgentTaskOwnerApprovalGrant') {
      taskRequests[op] = payload ?? const {};
      return {
        'operation': op,
        'status': 'accepted',
        'evidence_source': 'INTERNAL_STATE',
        'audit_event_id': 'task-owner-approval-audit',
        'body': {
          '状態': 'Owner Approval発行済み',
          '実行状態': '未実行',
          '実行系ID': 'local',
          '対話セッションID': '1' * 32,
          '作業領域ID': 'workspace-a',
          '指示hash': 'sha256:${'5' * 64}',
          '実行条件hash': 'sha256:${'7' * 64}',
          '適用ポリシー': 'gui-shell-agent-task-sandbox-v1-max-runtime-900s',
          'expires_at_epoch_seconds': expiry,
          'use_limit': 1,
          'uses_remaining': 1,
          'status': 'issued_unconsumed'
        }
      };
    }
    if (op == 'AgentTask実行') {
      taskRequests[op] = payload ?? const {};
      return {
        'operation': op,
        'status': 'accepted',
        'evidence_source': 'INTERNAL_STATE',
        'audit_event_id': 'task-start-audit',
        'body': {
          'task_id': '3' * 32,
          'record_version': 2,
          'agent_runtime_id': 'local',
          'session_id': '1' * 32,
          'workspace_id': 'workspace-a',
          'description': 'Agent作業Task（結果本文とWorkspace差分は別の権限経路）',
          'instruction_hash': 'sha256:${'5' * 64}',
          'status': 'pending',
          'audit_event_id': 'task-start-audit',
          'result_content_available': false
        }
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
            'record': historyRecord()
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
  test('履歴承認の残り時間は壁時計と単調時計の早い方に制限する', () async {
    final grant = (await HistoryClient(Fixture()).status())!;
    expect(grant.untilExpiry, greaterThan(Duration.zero));
    expect(grant.untilExpiry, lessThanOrEqualTo(const Duration(minutes: 5)));
  });

  test('入力概要は版2のhash_only metadataだけを受理する', () async {
    final f = Fixture()..withInputSummary = true;
    final c = HistoryClient(f);
    final grant = (await c.status())!;
    final entry = (await c.page(grant)).entries.single;
    expect(entry.inputSummary!.hash, 'sha256:${'1' * 64}');
    for (final alter in <void Function(Map<String, Object?>)>[
      (v) => v.remove('入力概要'),
      (v) => (v['入力概要'] as Map)['表示範囲'] = 'full',
      (v) => v['入力概要'] = {
            '表示範囲': 'hash_only',
            '入力hash': 'sha256:${'1' * 64}',
            '文字数': 4,
          },
      (v) => (v['入力概要'] as Map)['入力hash'] = 'sha256:invalid',
      (v) => v['入力概要'] = {
            '表示範囲': 'hash_only',
            '入力hash': 'sha256:${'1' * 64}',
            '本文': '漏洩',
          },
      (v) => v['版'] = 3,
      (v) => v['版'] = '2',
    ]) {
      f.alterRecord = alter;
      await expectLater(c.page(grant), throwsA(isA<BrokerClientException>()));
    }
  });

  testWidgets('履歴詳細にhash_only入力概要を表示する', (tester) async {
    final f = Fixture()..withInputSummary = true;
    await tester.pumpWidget(MaterialApp(
        home: Scaffold(body: HistoryScreen(client: HistoryClient(f)))));
    await tester.pumpAndSettle();
    await tester.tap(find.byType(ExpansionTile));
    await tester.pumpAndSettle();
    expect(find.textContaining('入力概要: hash_only／sha256:${'1' * 64}'),
        findsOneWidget);
    expect(find.textContaining('入力本文は返しません。hashは候補照合に使えますが、本文の正しさ・現在権限は示しません。'),
        findsOneWidget);
    await tester.pumpWidget(const SizedBox());
  });

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
    expect(
        find.textContaining('参照元監査hash: sha256:${'c' * 64}'), findsOneWidget);
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
  test('Agent Task履歴は現在Runtime承認・Audit hash・本文非露出を要求する', () async {
    final f = Fixture();
    final c = HistoryClient(f);
    final grant = (await c.status())!;
    final page = await c.taskPage(grant);
    expect(page.entries.single.status, 'completed');
    expect(page.entries.single.runtimeId, grant.runtime);
    expect(page.entries.single.auditId, 'task-finish');
    expect(f.operations, ['対話履歴閲覧状態', '対話履歴閲覧状態', 'AgentTask履歴閲覧', '対話履歴閲覧状態']);
    f.taskRuntime = 'other-runtime';
    await expectLater(c.taskPage(grant), throwsA(isA<BrokerClientException>()));
    f.taskRuntime = 'local';
    f.taskWorkspace = 'unsafe workspace';
    await expectLater(c.taskPage(grant), throwsA(isA<BrokerClientException>()));
    f.taskWorkspace = 'workspace-a';
    f.alterTaskRecord = (record) {
      record['status'] = 'failed';
      record['failure_class'] = '中断回復';
      record['result_hash'] = null;
    };
    await expectLater(c.taskPage(grant), throwsA(isA<BrokerClientException>()));
    f.alterTaskRecord = null;
    f.alterTaskRecord = (record) => record['result_body'] = 'hidden content';
    await expectLater(c.taskPage(grant), throwsA(isA<BrokerClientException>()));
    f.alterTaskRecord = null;
    f.revokeDuringRead = true;
    await expectLater(c.taskPage(grant), throwsA(isA<BrokerClientException>()));
  });
  test('Agent Task履歴分岐は現行終端参照と新Session・新instruction hashを検査する', () async {
    final f = Fixture();
    final c = HistoryClient(f);
    final grant = (await c.status())!;
    final source = (await c.taskPage(grant)).entries.single;
    final branch = await c.branchAgentTask(grant, source, 'fresh task input');
    expect(f.sent, {
      'approval_id': grant.id,
      '実行系ID': grant.runtime,
      '参照監査ID': source.auditId,
      '参照event_hash': source.eventHash,
      'source_task_id': source.taskId,
      '入力': 'fresh task input'
    });
    expect(branch['状態'], 'Task開始前');
    expect(branch['種別'], 'AgentTask分岐');
    expect(branch['source_task_id'], source.taskId);
    expect(branch['対話セッションID'], isNot(source.sessionId));
    expect(branch['branch_audit_event_id'], 'task-branch-audit');
    final activeSource = AgentTaskHistoryEntry(
      source.taskId,
      source.runtimeId,
      source.sessionId,
      source.workspaceId,
      source.instructionHash,
      'running',
      source.createdAt,
      source.updatedAt,
      null,
      null,
      source.startAuditEventId,
      source.auditId,
      source.eventHash,
    );
    await expectLater(
      c.branchAgentTask(grant, activeSource, 'fresh task input'),
      throwsA(isA<BrokerClientException>()),
    );

    f.alterTaskBranch = (body) => body['source_task_id'] = 'c' * 32;
    await expectLater(
      c.branchAgentTask(grant, source, 'fresh task input'),
      throwsA(isA<BrokerClientException>()),
    );
  });
  testWidgets('Task履歴一覧はhashと状態だけを表示し承認失効で破棄する', (tester) async {
    final f = Fixture();
    await tester.pumpWidget(MaterialApp(
        home: Scaffold(body: HistoryScreen(client: HistoryClient(f)))));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Agent Task履歴'));
    await tester.pumpAndSettle();
    expect(find.textContaining('completed ／ ${'a' * 32}'), findsOneWidget);
    expect(find.textContaining('導入前の記録はAuditに保持されますが、この一覧へ推測再構成しません。'),
        findsOneWidget);
    await tester.tap(find.byKey(ValueKey('agent-task-${'a' * 32}')));
    await tester.pumpAndSettle();
    expect(
        find.textContaining(
            '本文・Permission・Approval・Credential・Authorityは復元しません。'),
        findsOneWidget);
    expect(find.textContaining('secret Task content'), findsNothing);
    f.revoked = true;
    await tester.pump(const Duration(seconds: 2));
    await tester.pumpAndSettle();
    expect(find.textContaining('completed ／ ${'a' * 32}'), findsNothing);
    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('Agent Task履歴から新Permission・Approvalを経て新Taskだけを開始する',
      (tester) async {
    final f = Fixture();
    await tester.pumpWidget(MaterialApp(
        home: Scaffold(body: HistoryScreen(client: HistoryClient(f)))));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Agent Task履歴'));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(ValueKey('agent-task-${'a' * 32}')));
    await tester.pumpAndSettle();
    final branchButton = find.byKey(ValueKey('agent-task-branch-${'a' * 32}'));
    await tester.ensureVisible(branchButton);
    await tester.pumpAndSettle();
    await tester.tap(branchButton);
    await tester.pumpAndSettle();
    await tester.enterText(
        find.byKey(const ValueKey('agent-task-history-branch-input')),
        'new task instruction');
    await tester.tap(find.text('新しいSessionを作成して事前検査'));
    await tester.pumpAndSettle();
    expect(find.textContaining('Permission: 未付与'), findsOneWidget);
    expect(f.sent!['source_task_id'], 'a' * 32);
    expect(f.sent!['入力'], 'new task instruction');
    expect(f.operations.contains('AgentTask実行'), false);

    final permissionButton = find.text('新しいWorkspace Permissionを発行');
    await tester.ensureVisible(permissionButton);
    await tester.pumpAndSettle();
    await tester.tap(permissionButton);
    await tester.pumpAndSettle();
    expect(f.operations, contains('AgentTaskWorkspacePermissionGrant'));
    final approvalButton = find.text('新しいOwner Approvalを発行');
    await tester.ensureVisible(approvalButton);
    await tester.pumpAndSettle();
    await tester.tap(approvalButton);
    await tester.pumpAndSettle();
    expect(f.operations, contains('AgentTaskOwnerApprovalGrant'));
    final startButton = find.text('新しいAgent Taskを開始');
    await tester.ensureVisible(startButton);
    await tester.pumpAndSettle();
    await tester.tap(startButton);
    await tester.pumpAndSettle();
    final permissionIndex =
        f.operations.indexOf('AgentTaskWorkspacePermissionGrant');
    final approvalIndex = f.operations.indexOf('AgentTaskOwnerApprovalGrant');
    final startIndex = f.operations.indexOf('AgentTask実行');
    expect(permissionIndex, greaterThanOrEqualTo(0));
    expect(approvalIndex, greaterThan(permissionIndex));
    expect(startIndex, greaterThan(approvalIndex));
    expect(f.taskRequests['Agent作業要求検査']!['require_unused_task_session'], true);
    expect(
        f.taskRequests['AgentTaskOwnerApprovalGrant']![
            'require_unused_task_session'],
        true);
    expect(f.taskRequests['AgentTask実行']!['require_unused_task_session'], true);
    expect(find.textContaining('新Task: ${'3' * 32} ／ pending'), findsOneWidget);
    expect(find.textContaining('分岐Audit: task-branch-audit'), findsOneWidget);
    expect(find.textContaining('結果本文はここでは取得していません。'), findsOneWidget);
    expect(find.textContaining('new task instruction'), findsNothing);
    await tester.tap(find.text('閉じる'));
    await tester.pumpAndSettle();
    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('Broker切替で旧履歴と遅延応答を破棄して新接続先から読み直す', (tester) async {
    final first = Fixture()..runtime = 'broker-a';
    final second = Fixture()..runtime = 'broker-b';
    Widget screen(Fixture fixture) => MaterialApp(
          home: Scaffold(
            body: HistoryScreen(client: HistoryClient(fixture)),
          ),
        );

    await tester.pumpWidget(screen(first));
    await tester.pumpAndSettle();
    expect(find.text('Runtime: broker-a ／ 1件の要求'), findsOneWidget);

    first.gate = Completer<void>();
    await tester.tap(find.text('先頭から更新'));
    await tester.pump();

    await tester.pumpWidget(screen(second));
    await tester.pumpAndSettle();
    expect(find.text('Runtime: broker-b ／ 1件の要求'), findsOneWidget);
    expect(find.text('Runtime: broker-a ／ 1件の要求'), findsNothing);

    first.gate!.complete();
    await tester.pumpAndSettle();
    expect(find.text('Runtime: broker-b ／ 1件の要求'), findsOneWidget);
    expect(find.text('Runtime: broker-a ／ 1件の要求'), findsNothing);
    await tester.pumpWidget(const SizedBox());
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

  testWidgets('履歴画面は承認状態だけを定期確認し一覧は手動更新する', (tester) async {
    final f = Fixture();
    await tester.pumpWidget(MaterialApp(
        home: Scaffold(body: HistoryScreen(client: HistoryClient(f)))));
    await tester.pumpAndSettle();

    int historyReads() =>
        f.operations.where((operation) => operation == '対話履歴閲覧').length;
    expect(historyReads(), 1);

    f.state = '保留';
    await tester.pump(const Duration(seconds: 2));
    await tester.pumpAndSettle();
    expect(historyReads(), 1);
    expect(find.textContaining('成功 ／'), findsOneWidget);

    await tester.tap(find.text('先頭から更新'));
    await tester.pumpAndSettle();
    expect(historyReads(), 2);
    expect(find.textContaining('保留 ／'), findsOneWidget);

    await tester.pump(const Duration(seconds: 2));
    await tester.pumpAndSettle();
    expect(historyReads(), 2);
    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('定期Grant照会に失敗したら履歴表示を消去する', (tester) async {
    final f = Fixture();
    await tester.pumpWidget(MaterialApp(
        home: Scaffold(body: HistoryScreen(client: HistoryClient(f)))));
    await tester.pumpAndSettle();
    expect(find.textContaining('成功 ／'), findsOneWidget);

    f.failStatus = true;
    await tester.pump(const Duration(seconds: 2));
    await tester.pumpAndSettle();

    expect(find.textContaining('成功 ／'), findsNothing);
    expect(
      find.text('現在の履歴閲覧承認を確認できないため表示を消去しました。'),
      findsOneWidget,
    );
    expect(
      f.operations.where((operation) => operation == '対話履歴閲覧').length,
      1,
    );
    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('履歴承認期限の到来で表示を破棄する', (tester) async {
    final f = Fixture()
      ..expiry = DateTime.now().millisecondsSinceEpoch ~/ 1000 + 2;
    await tester.pumpWidget(MaterialApp(
        home: Scaffold(body: HistoryScreen(client: HistoryClient(f)))));
    await tester.pumpAndSettle();
    expect(find.textContaining('成功 ／'), findsOneWidget);
    await tester
        .runAsync(() => Future<void>.delayed(const Duration(seconds: 3)));
    await tester.pump(const Duration(seconds: 3));
    await tester.pumpAndSettle();
    expect(find.textContaining('成功 ／'), findsNothing);
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
