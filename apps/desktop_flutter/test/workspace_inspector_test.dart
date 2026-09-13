import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/screens/workspace_inspector.dart';
import 'package:gui_shell_desktop/services/broker_client.dart';
import 'package:gui_shell_desktop/services/workspace_client.dart';

class TestBroker implements BrokerTransport {
  String visibility = 'full';
  bool folders = false;
  String baselineHash = "sha256:${'c' * 64}";
  String diffKind = "text";
  String? approval = 'approval-a';
  int expires = DateTime.now().millisecondsSinceEpoch ~/ 1000 + 240;
  bool revokeAfterRead = false;
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
        '作業領域': [registration]
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
      if (visibility == 'full' && operation == '作業領域差分') {
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

void main() {
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
        'transport': 'authenticated_loopback_tcp',
        'max_request_bytes': 65536
      }));
      final client = await BrokerClient.connect(sessionFile: endpoint.path);
      await expectLater(
          client.request('health'), throwsA(isA<BrokerClientException>()));
      wrongId = false;
      await expectLater(
          client.request('health'), throwsA(isA<BrokerClientException>()));
    } finally {
      await accepted.cancel();
      await server.close();
      await root.delete(recursive: true);
    }
  });

  test('独立Brokerとowner CLIから製品Dart読取まで実接続する', () async {
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
      final transport = await BrokerClient.connect(sessionFile: normal);
      final client = WorkspaceClient(transport);
      final denied = (await client.list()).single;
      await expectLater(client.read(denied, 'file.txt', tree: false),
          throwsA(isA<BrokerClientException>()));
      final granted = await Process.run(executable, [
        '作業領域制御',
        '--session-file',
        owner,
        '作業領域承認',
        denied.id,
        denied.hash,
        'full'
      ]);
      expect(granted.exitCode, 0);
      final selected = (await client.list()).single;
      final view = await client.read(selected, 'file.txt', tree: false);
      expect(view.projection!['text'], '実接続本文');
      await File('${project.path}/large')
          .writeAsBytes(List<int>.filled(70000, 120));
      final saved = await Process.run(executable, [
        '作業領域制御',
        '--session-file',
        owner,
        '作業領域基準点保存',
        selected.id,
        selected.hash,
        'file.txt',
        'large'
      ]);
      expect(saved.exitCode, 0);
      final scope = await client.read(selected, '', tree: false, scope: true);
      expect(scope.projection!['相対paths'], ['file.txt', 'large']);
      final baselineHash = scope.projection!['基準点hash'] as String;
      await File('${project.path}/file.txt').writeAsString('比較後の本文\n');
      final diff = await client.read(selected, 'file.txt',
          tree: false, baselineHash: baselineHash);
      expect((diff.projection!['diff'] as Map)['kind'], 'text');
      expect((diff.projection!['diff'] as Map)['unified'], contains('比較後の本文'));

      await File('${project.path}/large')
          .writeAsBytes(List<int>.filled(70001, 121));
      final large = await client.read(selected, 'large',
          tree: false, baselineHash: baselineHash);
      final largeDiff = large.projection!['diff'] as Map;
      expect(largeDiff['kind'], 'oversized');
      expect(largeDiff['unified'], isNull);
      expect(largeDiff['rows'], isEmpty);
      expect((largeDiff['before'] as Map)['bytes'], 70000);
      expect((largeDiff['after'] as Map)['bytes'], 70001);
      final tree = await client.read(selected, '', tree: true);
      expect((tree.projection!['entries'] as List).length, 3);
      final nested = await client.read(selected, 'folder', tree: true);
      expect((nested.projection!['entries'] as List).single['path'],
          'folder/nested.txt');
      final nestedFile =
          await client.read(selected, 'folder/nested.txt', tree: false);
      expect(nestedFile.projection!['text'], '配下の本文');
      await expectLater(client.read(selected, '.env', tree: false),
          throwsA(isA<BrokerClientException>()));
      final whole = await Process.run(executable, [
        '作業領域制御',
        '--session-file',
        owner,
        '作業領域全体基準点保存',
        selected.id,
        selected.hash
      ]);
      expect(whole.exitCode, 0);
      final wholeScope =
          await client.read(selected, '', tree: false, scope: true);
      final wholeHash = wholeScope.projection!['基準点hash'] as String;
      await File('${project.path}/new.txt').writeAsString('新規');
      await File('${project.path}/file.txt').writeAsString('変更');
      await File('${project.path}/folder/nested.txt').delete();
      final changes = await client.read(selected, '',
          tree: false, changes: true, baselineHash: wholeHash);
      expect(changes.projection!['changes'], [
        {'path': 'file.txt', 'status': 'modified'},
        {'path': 'folder/nested.txt', 'status': 'deleted'},
        {'path': 'new.txt', 'status': 'added'}
      ]);
      final newDiff = await client.read(selected, 'new.txt',
          tree: false, baselineHash: wholeHash);
      expect((newDiff.projection!['diff'] as Map)['before'], isNull);
      final revoked = await Process.run(executable, [
        '作業領域制御',
        '--session-file',
        owner,
        '作業領域失効',
        selected.id,
        selected.hash
      ]);
      expect(revoked.exitCode, 0);
      await expectLater(client.read(selected, 'file.txt', tree: false),
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
    await tester.tap(find.text('workspace-a'));
    await tester.pumpAndSettle();
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
    await tester.tap(find.text('folder'));
    await tester.pumpAndSettle();
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
    await tester.tap(find.text('file.txt'));
    await tester.pumpAndSettle();
    expect(find.textContaining('--- a/file'), findsOneWidget);
    await tester.ensureVisible(find.text('左右比較'));
    await tester.tap(find.text('左右比較'));
    await tester.pumpAndSettle();
    expect(find.text('前'), findsOneWidget);
    expect(find.text('後'), findsOneWidget);
    expect(find.text('変更'), findsOneWidget);
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
