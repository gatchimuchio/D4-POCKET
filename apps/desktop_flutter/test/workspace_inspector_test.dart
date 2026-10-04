import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/screens/workspace_inspector.dart';
import 'package:gui_shell_desktop/services/broker_client.dart';
import 'package:gui_shell_desktop/services/workspace_client.dart';
import 'support/test_broker_tcp_transport.dart';

class TestBroker implements BrokerTransport {
  String visibility = 'full';
  bool folders = false;
  String baselineHash = "sha256:${'c' * 64}";
  String baselineEvidence = 'LIVE_RUNTIME';
  String diffKind = "text";
  String? approval = 'approval-a';
  int expires = DateTime.now().millisecondsSinceEpoch ~/ 1000 + 240;
  bool revokeAfterRead = false;
  List<Map<String, Object?>> additionalRegistrations = [];
  void Function(Map<String, Object?>)? mutate;
  Completer<void>? pendingRead;
  final operations = <String>[];
  Map<String, Object?> get registration => {
        '作業領域ID': 'workspace-a',
        '実行系ID': 'runtime-a',
        '登録hash': 'sha256:${'a' * 64}',
        '承認状態': approval == null ? 'denied' : 'approved',
        '有効期限': approval == null ? null : expires,
        '表示範囲': approval == null ? 'none' : visibility,
        'approval_id': approval,
      };
  @override
  Future<Map<String, Object?>> request(String operation,
      {Map<String, Object?>? payload}) async {
    operations.add(operation);
    Map<String, Object?> body;
    String evidence = 'INTERNAL_STATE';
    if (operation == '作業領域一覧') {
      body = {
        '作業領域': [registration, ...additionalRegistrations]
      };
    } else if (operation == '作業領域承認') {
      approval = 'approval-created';
      expires = DateTime.now().millisecondsSinceEpoch ~/ 1000 + 240;
      visibility = payload!['表示範囲'] as String;
      body = {
        '作業領域ID': payload['作業領域ID'],
        'approval_id': approval,
        'permission_id': 'workspace.inspect.$approval',
        'capability_id': 'workspace.inspect',
        'recovery_id': 'workspace.reapprove',
        '有効期限': expires,
        '表示範囲': visibility,
      };
    } else if (operation == '作業領域失効') {
      approval = null;
      visibility = 'none';
      body = {'作業領域ID': payload!['作業領域ID'], '承認状態': 'revoked'};
    } else if (operation == '作業領域全体基準点保存') {
      evidence = baselineEvidence;
      body = {
        'version': 1,
        'operation': operation,
        '要求hash': brokerPayloadHash(payload),
        '作業領域ID': 'workspace-a',
        '実行系ID': 'runtime-a',
        '登録hash': 'sha256:${'a' * 64}',
        'approval_id': approval,
        '有効期限': expires,
        '表示範囲': visibility,
        'projection': {'基準点hash': baselineHash, '対象数': 1},
      };
    } else {
      await pendingRead?.future;
      final tree = operation == '作業領域ツリー';
      Object? projection;
      switch (visibility) {
        case 'none':
          projection = null;
        case 'hash_only':
          projection = {'sha256': 'sha256:${'b' * 64}'};
        case 'summary':
        case 'redacted':
          projection = {'説明': 'この表示範囲に提供できる承認済み内容はありません'};
        case 'full':
          projection = tree
              ? {
                  'entries': [
                    {'path': 'file.txt', 'kind': 'file', 'bytes': 6}
                  ]
                }
              : {
                  'path': payload!['相対path'],
                  'bytes': utf8.encode('本文').length,
                  'sha256': 'sha256:${'b' * 64}',
                  'binary': false,
                  'text': '本文'
                };
      }
      body = <String, Object?>{
        'version': 1,
        'operation': operation,
        '要求hash': brokerPayloadHash(payload),
        '作業領域ID': 'workspace-a',
        '実行系ID': 'runtime-a',
        '登録hash': 'sha256:${'a' * 64}',
        'approval_id': approval,
        '有効期限': expires,
        '表示範囲': visibility,
        'projection': projection,
      };
      if (folders && tree && visibility == 'full') {
        body['projection'] = {
          'entries': payload!['相対path'] == ''
              ? [
                  {'path': 'folder', 'kind': 'directory', 'bytes': null}
                ]
              : [
                  {'path': 'folder/nested.txt', 'kind': 'file', 'bytes': 6}
                ]
        };
      }
      if (visibility == 'full' && operation == '作業領域比較範囲') {
        body['projection'] = {
          '基準点hash': baselineHash,
          '相対paths': ['file.txt']
        };
      }
      if (visibility == 'full' && operation == '作業領域変更一覧') {
        body['projection'] = {
          '基準点hash': baselineHash,
          'changes': [
            {'path': 'file.txt', 'status': 'modified'}
          ],
          'unchanged': 2,
          'excluded_secrets': 1
        };
      }
      if (visibility == 'full' &&
          (operation == '作業領域差分' || operation == '作業領域復旧プレビュー')) {
        body['projection'] = {
          '基準点hash': baselineHash,
          'diff': {
            'version': 1,
            'kind': diffKind,
            'before': {'bytes': 4, 'sha256': 'sha256:${'a' * 64}'},
            'after': {'bytes': 4, 'sha256': 'sha256:${'b' * 64}'},
            'unified': diffKind == 'text'
                ? '--- a/file\n+++ b/file\n@@ -1,1 +1,1 @@\n-前\n+後\n'
                : null,
            'rows': diffKind == 'text'
                ? [
                    {
                      'kind': 'changed',
                      'before': {'number': 1, 'text': '前', 'newline': true},
                      'after': {'number': 1, 'text': '後', 'newline': true}
                    }
                  ]
                : [],
          }
        };
      }
      if (visibility == 'full' && operation == '作業領域復旧プレビュー') {
        (body['projection'] as Map).addAll(<String, Object>{
          'action': 'replace',
          'baseline_content_available': true,
          'execution_permitted': false
        });
      }
      mutate?.call(body);
      evidence =
          operation != '作業領域比較範囲' && {'full', 'hash_only'}.contains(visibility)
              ? 'LIVE_RUNTIME'
              : 'INTERNAL_STATE';
      if (revokeAfterRead) approval = null;
    }
    return {
      'status': 'accepted',
      'operation': operation,
      'error': null,
      'audit_event_id': 'audit-a',
      'evidence_source': evidence,
      'body': body
    };
  }
}

Future<void> _scrollRootTo(WidgetTester tester, Finder target) async {
  final root = find.byType(SingleChildScrollView).first;
  final scrollable =
      find.descendant(of: root, matching: find.byType(Scrollable)).first;
  await tester.dragUntilVisible(target, scrollable, const Offset(0, -300));
  await tester.pumpAndSettle();
}

void main() {
  test('Workspace読取Approval・baseline・失効をそれぞれBrokerで照合する', () async {
    final broker = TestBroker()
      ..approval = null
      ..visibility = 'none';
    final client = WorkspaceClient(broker);
    final unapproved = (await client.list()).single;
    final approved = await client.approveContent(unapproved, 'full');
    expect(approved.current(DateTime.now()), isTrue);
    expect(approved.visibility, 'full');
    final baseline = await client.captureWholeBaseline(approved);
    expect(baseline.operation, '作業領域全体基準点保存');
    expect(baseline.baselineHash, broker.baselineHash);
    expect(baseline.projection?['対象数'], 1);
    final changes = await client.read(baseline.registration, '',
        tree: false, changes: true, baselineHash: baseline.baselineHash);
    expect(changes.projection?['changes'], [
      {'path': 'file.txt', 'status': 'modified'}
    ]);
    await client.revokeContent(approved);
    expect(broker.operations, [
      '作業領域一覧',
      '作業領域承認',
      '作業領域一覧',
      '作業領域全体基準点保存',
      '作業領域一覧',
      '作業領域変更一覧',
      '作業領域一覧',
      '作業領域比較範囲',
      '作業領域一覧',
      '作業領域失効',
      '作業領域一覧',
    ]);
  });

  test('Workspace baselineはfull承認とLIVE_RUNTIME証拠を必須にする', () async {
    final broker = TestBroker()..visibility = 'hash_only';
    final client = WorkspaceClient(broker);
    await expectLater(client.captureWholeBaseline((await client.list()).single),
        throwsA(isA<BrokerClientException>()));

    broker
      ..visibility = 'full'
      ..baselineEvidence = 'INTERNAL_STATE';
    await expectLater(client.captureWholeBaseline((await client.list()).single),
        throwsA(isA<BrokerClientException>()));
  });

  test('通常TCP応答の別要求IDと別操作を拒否する', () async {
    final root = await Directory.systemTemp.createTemp('gui-shell-reply-bind-');
    final server = await ServerSocket.bind(InternetAddress.loopbackIPv4, 0);
    var wrongId = true;
    final accepted = server.listen((socket) async {
      final lines = socket
          .cast<List<int>>()
          .transform(utf8.decoder)
          .transform(const LineSplitter());
      var index = 0;
      await for (final line in lines) {
        if (++index != 2) continue;
        final request = jsonDecode(line) as Map;
        socket.write('${jsonEncode({
              'request_id': wrongId ? 'other' : request['request_id'],
              'operation': wrongId ? request['operation'] : 'shutdown'
            })}\n');
        await socket.flush();
        socket.destroy();
        break;
      }
    });
    try {
      final endpoint = File('${root.path}/endpoint.json');
      await endpoint.writeAsString(jsonEncode({
        'host': '127.0.0.1',
        'port': server.port,
        'session_id': 'test-session',
        'session_secret': 'a' * 64,
        'credential_role': 'normal',
        'transport': 'authenticated_loopback_tcp',
        'max_request_bytes': 65536
      }));
      final client = await TestBrokerTcpTransport.connect(endpoint.path);
      await expectLater(
        client.request('health'),
        throwsA(isA<TestBrokerTransportException>()),
      );
      wrongId = false;
      await expectLater(
        client.request('health'),
        throwsA(isA<TestBrokerTransportException>()),
      );
    } finally {
      await accepted.cancel();
      await server.close();
      await root.delete(recursive: true);
    }
  });

  test('Owner資格だけの直接IPCからWorkspace内容を取得できない', () async {
    final executable = File(
            '../../native/rust_helper/target/debug/gui_shell_rust_helper${Platform.isWindows ? '.exe' : ''}')
        .absolute
        .path;
    expect(File(executable).existsSync(), isTrue,
        reason: '先にRust helperをbuildしてください');
    final root = await Directory.systemTemp.createTemp('gui-shell-ui-read-');
    Process? process;
    try {
      final project = await Directory('${root.path}/project').create();
      await File('${project.path}/file.txt').writeAsString('実接続本文');
      await Directory('${project.path}/folder').create();
      await File('${project.path}/folder/nested.txt').writeAsString('配下の本文');
      await File('${project.path}/.env').writeAsString('試験の非公開値');
      final config = File('${root.path}/workspace.json');
      await config.writeAsString(jsonEncode({
        'version': 1,
        'workspaces': [
          {
            'runtime_id': 'gui_shell_rust_broker',
            'workspace_id': 'workspace-a',
            'root_path': await project.resolveSymbolicLinks(),
            'secret_paths': <String>[]
          }
        ]
      }));
      final normal = '${root.path}/normal.json';
      final owner = '${root.path}/owner.json';
      process = await Process.start(executable, [
        'broker-server',
        '--store-dir',
        '${root.path}/store',
        '--session-file',
        normal,
        '--owner-session-file',
        owner,
        '--workspace-config',
        config.path
      ]);
      final output = process.stdout.drain<void>();
      final errors = process.stderr.drain<void>();
      final deadline = DateTime.now().add(const Duration(seconds: 15));
      while (!File(normal).existsSync() || !File(owner).existsSync()) {
        if (DateTime.now().isAfter(deadline)) fail('試験Broker起動期限超過');
        await Future<void>.delayed(const Duration(milliseconds: 30));
      }
      final transport = await TestBrokerTcpTransport.connect(normal);
      final client = WorkspaceClient(transport);
      final denied = (await client.list()).single;
      await expectLater(client.read(denied, 'file.txt', tree: false),
          throwsA(isA<BrokerClientException>()));
      final ownerCliBypass = await Process.run(executable, [
        '作業領域制御',
        '--session-file',
        owner,
        '作業領域承認',
        denied.id,
        denied.hash,
        'full'
      ]);
      expect(ownerCliBypass.exitCode, isNot(0));
      await expectLater(client.read(denied, 'file.txt', tree: false),
          throwsA(isA<BrokerClientException>()));
      await transport.request('shutdown');
      expect(await process.exitCode.timeout(const Duration(seconds: 10)), 0);
      await Future.wait([output, errors]);
    } finally {
      process?.kill();
      if (process != null) await process.exitCode;
      await root.delete(recursive: true);
    }
  });
  test('全表示範囲のprojectionを検査しowner操作を送らない', () async {
    for (final visibility in [
      'none',
      'hash_only',
      'summary',
      'redacted',
      'full'
    ]) {
      final broker = TestBroker()..visibility = visibility;
      final client = WorkspaceClient(broker);
      final selected = (await client.list()).single;
      final result = await client.read(selected, 'file.txt', tree: false);
      expect(result.registration.visibility, visibility);
      expect(broker.operations, ['作業領域一覧', '作業領域読取', '作業領域一覧']);
    }
  });
  test('登録・承認・期限・要求・表示範囲の不一致とbinary本文を拒否', () async {
    for (final mutation in <void Function(Map<String, Object?>)>[
      (v) => v['登録hash'] = 'sha256:${'c' * 64}',
      (v) => v['approval_id'] = 'other',
      (v) => v['有効期限'] = 1,
      (v) => v['要求hash'] = 'sha256:${'c' * 64}',
      (v) => v['表示範囲'] = 'hash_only',
      (v) => (v['projection'] as Map)['binary'] = true,
      (v) => v['authority'] = 'owner',
    ]) {
      final broker = TestBroker()..mutate = mutation;
      final client = WorkspaceClient(broker);
      await expectLater(
          client.read((await client.list()).single, 'file.txt', tree: false),
          throwsA(isA<BrokerClientException>()));
    }
  });
  test('directoryの数値サイズとfileのnullサイズを拒否する', () async {
    for (final entry in [
      {'path': 'folder', 'kind': 'directory', 'bytes': 0},
      {'path': 'file.txt', 'kind': 'file', 'bytes': null},
    ]) {
      final broker = TestBroker()
        ..mutate = (v) => v['projection'] = {
              'entries': [entry]
            };
      final client = WorkspaceClient(broker);
      await expectLater(
          client.read((await client.list()).single, '', tree: true),
          throwsA(isA<BrokerClientException>()));
    }
  });
  test('取得中の失効と期限切れを拒否する', () async {
    final broker = TestBroker()..revokeAfterRead = true;
    final client = WorkspaceClient(broker);
    await expectLater(
        client.read((await client.list()).single, 'file.txt', tree: false),
        throwsA(isA<BrokerClientException>()));
    broker.approval = 'approval-b';
    broker.expires = 1;
    await expectLater(
        client.read((await client.list()).single, 'file.txt', tree: false),
        throwsA(isA<BrokerClientException>()));
  });
  testWidgets('実一覧と本文を描画し承認失効確認で本文を消す', (tester) async {
    final broker = TestBroker();
    await tester.pumpWidget(MaterialApp(
        home: Scaffold(
            body: SingleChildScrollView(
                child: WorkspaceInspector(client: WorkspaceClient(broker))))));
    await tester.pumpAndSettle();
    await _scrollRootTo(tester, find.text('workspace-a'));
    await tester.tap(find.text('workspace-a'));
    await tester.pumpAndSettle();
    await _scrollRootTo(tester, find.text('file.txt'));
    await tester.tap(find.text('file.txt'));
    await tester.pumpAndSettle();
    expect(find.text('本文'), findsOneWidget);
    broker.approval = null;
    await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 2200)));
    await tester.pump(const Duration(seconds: 3));
    await tester.pumpAndSettle();
    expect(find.text('本文'), findsNothing);
    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('Agent Center内InspectorでOwner確認後にbaseline差分へ進める', (tester) async {
    final broker = TestBroker();
    await tester.pumpWidget(MaterialApp(
        home: Scaffold(
            body: SingleChildScrollView(
                child: WorkspaceInspector(client: WorkspaceClient(broker))))));
    await tester.pumpAndSettle();
    await tester.tap(find.text('workspace-a'));
    await tester.pumpAndSettle();
    final capture = find.text('native Owner確認で比較baselineを保存');
    await tester.ensureVisible(capture);
    await tester.tap(capture);
    await tester.pumpAndSettle();
    expect(find.text('取得file数: 1'), findsOneWidget);
    final compare = find.text('baseline以降の変更file／差分を表示');
    await tester.ensureVisible(compare);
    await tester.tap(compare);
    await tester.pumpAndSettle();
    expect(find.text('変更 1件・変更なし 2件・secret除外 1件'), findsOneWidget);
    expect(find.text('file.txt'), findsOneWidget);
    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('未承認Workspaceを選択しても本文は出さずOwner確認操作を示す', (tester) async {
    final broker = TestBroker()
      ..approval = null
      ..visibility = 'none';
    await tester.pumpWidget(MaterialApp(
        home: Scaffold(
            body: SingleChildScrollView(
                child: WorkspaceInspector(client: WorkspaceClient(broker))))));
    await tester.pumpAndSettle();
    await tester.tap(find.text('workspace-a'));
    await tester.pumpAndSettle();
    expect(find.text('native Owner確認でWorkspace読取を許可'), findsOneWidget);
    expect(find.text('本文'), findsNothing);
    await tester.pumpWidget(const SizedBox());
  });

  testWidgets(
      'Agent Center用InspectorはTaskのRuntime／Workspaceだけ表示し非active時に本文を消す',
      (tester) async {
    final broker = TestBroker()
      ..additionalRegistrations = [
        {
          '作業領域ID': 'workspace-other',
          '実行系ID': 'runtime-other',
          '登録hash': 'sha256:${'f' * 64}',
          '承認状態': 'approved',
          '有効期限': DateTime.now().millisecondsSinceEpoch ~/ 1000 + 240,
          '表示範囲': 'full',
          'approval_id': 'approval-other',
        },
      ];
    Widget buildInspector({required bool active}) => MaterialApp(
          home: Scaffold(
            body: SingleChildScrollView(
              child: WorkspaceInspector(
                client: WorkspaceClient(broker),
                runtimeId: 'runtime-a',
                workspaceId: 'workspace-a',
                active: active,
              ),
            ),
          ),
        );

    await tester.pumpWidget(buildInspector(active: true));
    await tester.pumpAndSettle();
    expect(find.text('workspace-a'), findsOneWidget);
    expect(find.text('workspace-other'), findsNothing);
    await tester.tap(find.text('workspace-a'));
    await tester.pumpAndSettle();
    await _scrollRootTo(tester, find.text('file.txt'));
    await tester.tap(find.text('file.txt'));
    await tester.pumpAndSettle();
    expect(find.text('本文'), findsOneWidget);

    await tester.pumpWidget(buildInspector(active: false));
    await tester.pumpAndSettle();
    expect(find.text('workspace-a'), findsNothing);
    expect(find.text('本文'), findsNothing);
    await tester.pumpWidget(const SizedBox());
  });
  testWidgets('フォルダーから配下本文へ移動しnullサイズを表示しない', (tester) async {
    final broker = TestBroker()..folders = true;
    await tester.pumpWidget(MaterialApp(
        home: Scaffold(
            body: SingleChildScrollView(
                child: WorkspaceInspector(client: WorkspaceClient(broker))))));
    await tester.pumpAndSettle();
    await tester.tap(find.text('workspace-a'));
    await tester.pumpAndSettle();
    expect(find.text('フォルダー'), findsOneWidget);
    expect(find.text('null バイト'), findsNothing);
    await _scrollRootTo(tester, find.text('folder'));
    await tester.tap(find.text('folder'));
    await tester.pumpAndSettle();
    await _scrollRootTo(tester, find.text('folder/nested.txt'));
    await tester.ensureVisible(find.text('folder/nested.txt'));
    await tester.tap(find.text('folder/nested.txt'));
    await tester.pumpAndSettle();
    expect(find.text('本文'), findsOneWidget);
    await tester.tap(find.text('作業領域の先頭へ'));
    await tester.pumpAndSettle();
    expect(find.text('folder'), findsOneWidget);
    await tester.pumpWidget(const SizedBox());
  });

  test('基準点置換・binary本文混入・行番号不正を拒否する', () async {
    final broker = TestBroker();
    final client = WorkspaceClient(broker);
    final selected = (await client.list()).single;
    final old = broker.baselineHash;
    broker.baselineHash = 'sha256:${'d' * 64}';
    await expectLater(
        client.read(selected, 'file.txt', tree: false, baselineHash: old),
        throwsA(isA<BrokerClientException>()));
    for (final kind in ['binary', 'oversized', 'unchanged']) {
      broker.diffKind = kind;
      final view = await client.read(selected, 'file.txt',
          tree: false, baselineHash: broker.baselineHash);
      expect((view.projection!['diff'] as Map)['unified'], isNull);
    }
    broker.mutate = (v) {
      if (v['operation'] == '作業領域差分') {
        ((v['projection'] as Map)['diff'] as Map)['unified'] = '混入本文';
      }
    };
    await expectLater(
        client.read(selected, 'file.txt',
            tree: false, baselineHash: broker.baselineHash),
        throwsA(isA<BrokerClientException>()));
    broker.diffKind = 'text';
    broker.mutate = (v) {
      if (v['operation'] == '作業領域差分') {
        ((((v['projection'] as Map)['diff'] as Map)['rows'] as List)
            .first['before'] as Map)['number'] = 2;
      }
    };
    await expectLater(
        client.read(selected, 'file.txt',
            tree: false, baselineHash: broker.baselineHash),
        throwsA(isA<BrokerClientException>()));
  });

  test('復旧プレビューの実行許可・候補矛盾・本文保持矛盾を拒否する', () async {
    for (final mutation in <void Function(Map)>[
      (p) => p['execution_permitted'] = true,
      (p) => p['action'] = 'remove',
      (p) => p['baseline_content_available'] = false,
    ]) {
      final broker = TestBroker()
        ..mutate = (body) {
          if (body['operation'] == '作業領域復旧プレビュー') {
            mutation(body['projection'] as Map);
          }
        };
      final client = WorkspaceClient(broker);
      await expectLater(
          client.read((await client.list()).single, 'file.txt',
              tree: false, preview: true, baselineHash: broker.baselineHash),
          throwsA(isA<BrokerClientException>()));
    }
  });

  testWidgets('復旧候補と逆方向ラベルを表示し実行操作を提供しない', (tester) async {
    final broker = TestBroker();
    await tester.pumpWidget(MaterialApp(
        home: Scaffold(
            body: SingleChildScrollView(
                child: WorkspaceInspector(client: WorkspaceClient(broker))))));
    await tester.pumpAndSettle();
    await tester.tap(find.text('workspace-a'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('基準点の対象file'));
    await tester.pumpAndSettle();
    await tester.ensureVisible(find.text('file.txt'));
    await tester.tap(find.text('file.txt'));
    await tester.pumpAndSettle();
    await tester.ensureVisible(find.text('復旧プレビュー'));
    await tester.tap(find.text('復旧プレビュー'));
    await tester.pumpAndSettle();
    expect(find.text('復旧候補: 内容置換'), findsOneWidget);
    expect(find.text('この画面から復旧は実行できません。'), findsOneWidget);
    await tester.ensureVisible(find.text('左右比較'));
    await tester.tap(find.text('左右比較'));
    await tester.pumpAndSettle();
    expect(find.text('現在の内容'), findsOneWidget);
    expect(find.text('復旧先の基準点'), findsOneWidget);
    expect(
        broker.operations.where(
            (v) => v.contains('実行') || v.contains('承認') || v.contains('保存')),
        isEmpty);
    await tester.pumpWidget(const SizedBox());
  });

  test('変更一覧の未知状態・重複path・別基準点・件数矛盾を拒否する', () async {
    for (final mutation in <void Function(Map)>[
      (p) => p['基準点hash'] = 'wrong',
      (p) => p['changes'] = [
            {'path': 'x', 'status': 'unknown'}
          ],
      (p) => p['changes'] = [
            {'path': 'x', 'status': 'added'},
            {'path': 'x', 'status': 'deleted'}
          ],
      (p) => p['changes'] = [
            {'path': '../x', 'status': 'added'}
          ],
      (p) => p['unchanged'] = 8192,
      (p) => p['excluded_secrets'] = -1,
    ]) {
      final broker = TestBroker()
        ..mutate = (body) {
          if (body['operation'] == '作業領域変更一覧') {
            mutation(body['projection'] as Map);
          }
        };
      final client = WorkspaceClient(broker);
      await expectLater(
          client.read((await client.list()).single, '',
              tree: false, changes: true, baselineHash: broker.baselineHash),
          throwsA(isA<BrokerClientException>()));
    }
  });

  test('全体基準点の空集合と128を超える対象を受理する', () async {
    for (final count in [0, 129]) {
      final broker = TestBroker()
        ..mutate = (body) {
          if (body['operation'] == '作業領域比較範囲') {
            (body['projection'] as Map)['相対paths'] =
                List.generate(count, (i) => 'file-$i');
          }
        };
      final client = WorkspaceClient(broker);
      final result = await client.read((await client.list()).single, '',
          tree: false, scope: true);
      expect((result.projection!['相対paths'] as List).length, count);
    }
  });

  testWidgets('4096件の対象一覧を遅延描画し末尾から差分を開く', (tester) async {
    final broker = TestBroker()
      ..mutate = (body) {
        if (body['operation'] == '作業領域比較範囲') {
          (body['projection'] as Map)['相対paths'] =
              List.generate(4096, (i) => 'file-$i');
        }
      };
    await tester.pumpWidget(MaterialApp(
        home: Scaffold(
            body: SingleChildScrollView(
                child: WorkspaceInspector(client: WorkspaceClient(broker))))));
    await tester.pumpAndSettle();
    await tester.tap(find.text('workspace-a'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('基準点の対象file'));
    await tester.pumpAndSettle();
    expect(find.text('対象 4096件'), findsOneWidget);
    expect(find.text('file-0'), findsOneWidget);
    expect(find.text('file-4095'), findsNothing);
    expect(find.byType(ListTile).evaluate().length, lessThan(30));
    final scroll = tester.state<ScrollableState>(find.descendant(
        of: find.byType(ListView), matching: find.byType(Scrollable)));
    scroll.position.jumpTo(scroll.position.maxScrollExtent);
    await tester.pumpAndSettle();
    await tester.ensureVisible(find.text('file-4095'));
    await tester.tap(find.text('file-4095'));
    await tester.pumpAndSettle();
    expect(find.text('file-4095'), findsOneWidget);
    expect(find.textContaining('--- a/file'), findsOneWidget);
    expect(broker.operations.last, '作業領域一覧');
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('変更一覧から差分を開き失効時に表示を破棄する', (tester) async {
    final broker = TestBroker();
    await tester.pumpWidget(MaterialApp(
        home: Scaffold(
            body: SingleChildScrollView(
                child: WorkspaceInspector(client: WorkspaceClient(broker))))));
    await tester.pumpAndSettle();
    await tester.tap(find.text('workspace-a'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('基準点の対象file'));
    await tester.pumpAndSettle();
    await _scrollRootTo(tester, find.text('全体基準点の変更一覧'));
    await tester.tap(find.text('全体基準点の変更一覧'));
    await tester.pumpAndSettle();
    expect(find.text('変更 1件・変更なし 2件・secret除外 1件'), findsOneWidget);
    await tester.ensureVisible(find.text('file.txt'));
    await tester.tap(find.text('file.txt'));
    await tester.pumpAndSettle();
    expect(find.textContaining('--- a/file'), findsOneWidget);
    broker.approval = null;
    await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 2200)));
    await tester.pump(const Duration(seconds: 3));
    await tester.pumpAndSettle();
    expect(find.textContaining('--- a/file'), findsNothing);
    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('基準点の対象から統合差分と左右比較を切り替える', (tester) async {
    final broker = TestBroker();
    await tester.pumpWidget(MaterialApp(
        home: Scaffold(
            body: SingleChildScrollView(
                child: WorkspaceInspector(client: WorkspaceClient(broker))))));
    await tester.pumpAndSettle();
    await tester.tap(find.text('workspace-a'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('基準点の対象file'));
    await tester.pumpAndSettle();
    await _scrollRootTo(tester, find.text('file.txt'));
    await tester.tap(find.text('file.txt'));
    await tester.pumpAndSettle();
    expect(find.textContaining('--- a/file'), findsOneWidget);
    await tester.ensureVisible(find.text('左右比較'));
    await tester.tap(find.text('左右比較'));
    await tester.pumpAndSettle();
    expect(find.text('前'), findsOneWidget);
    expect(find.text('後'), findsOneWidget);
    expect(find.text('変更'), findsOneWidget);
    broker.pendingRead = Completer<void>();
    await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 2200)));
    await tester.pump(const Duration(seconds: 3));
    await tester.pump();
    expect(find.text('前'), findsNothing);
    broker.pendingRead!.complete();
    broker.pendingRead = null;
    await tester.pumpAndSettle();
    expect(
        tester
            .widget<ChoiceChip>(find.widgetWithText(ChoiceChip, '左右比較'))
            .selected,
        isTrue);
    expect(find.text('前'), findsOneWidget);
    broker.baselineHash = 'sha256:${'d' * 64}';
    await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 2200)));
    await tester.pump(const Duration(seconds: 3));
    await tester.pumpAndSettle();
    expect(find.text('前'), findsNothing);
    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('非表示後に完了した遅延読取を復元しない', (tester) async {
    final broker = TestBroker();
    await tester.pumpWidget(MaterialApp(
        home: Scaffold(
            body: WorkspaceInspector(client: WorkspaceClient(broker)))));
    await tester.pumpAndSettle();
    broker.pendingRead = Completer<void>();
    await tester.tap(find.text('workspace-a'));
    await tester.pump();
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    await tester.pump();
    broker.pendingRead!.complete();
    await tester.pumpAndSettle();
    expect(find.text('file.txt'), findsNothing);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pumpWidget(const SizedBox());
  });
}
