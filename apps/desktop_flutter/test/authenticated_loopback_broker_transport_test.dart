import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:broker_contract/broker_request_envelope.dart'
    as broker_contract;
import 'package:broker_contract/broker_transport.dart'
    show BrokerClientException;
import 'package:broker_ipc_client/broker_ipc_client.dart';
import 'package:flutter_test/flutter_test.dart';

const _secret =
    'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
const _sessionId = 'broker-session-0123456789abcdef0123456789abcdef';

Map<String, Object?> _endpoint(
  int port, {
  String role = 'normal',
  String host = '127.0.0.1',
  String transport = 'authenticated_loopback_tcp',
  String secret = _secret,
  int maxRequestBytes = 1024 * 1024,
  String sessionId = _sessionId,
}) =>
    {
      'host': host,
      'port': port,
      'session_id': sessionId,
      'session_secret': secret,
      'credential_role': role,
      'transport': transport,
      'max_request_bytes': maxRequestBytes,
    };

Future<List<String>> _readRequestLines(Socket socket) async {
  final lines = <String>[];
  await for (final line in socket
      .cast<List<int>>()
      .transform(utf8.decoder)
      .transform(const LineSplitter())) {
    lines.add(line);
    if (lines.length == 2) break;
  }
  return lines;
}

Future<void> _reply(
  Socket socket,
  Map<String, Object?> request, {
  String? requestId,
  String? operation,
  Map<String, Object?>? body,
}) async {
  socket.add(
    utf8.encode(
      '${jsonEncode({
            'request_id': requestId ?? request['request_id'],
            'operation': operation ?? request['operation'],
            'status': 'accepted',
            'evidence_source': 'FIXTURE',
            'audit_event_id': 'broker-audit-test',
            'error': null,
            'body': body ?? <String, Object?>{},
            'shutdown_requested': false,
          })}\n',
    ),
  );
  await socket.flush();
  await socket.close();
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('通常資格で既存Broker loopback frameを送信し応答を相関する', () async {
    final server = await ServerSocket.bind(InternetAddress.loopbackIPv4, 0);
    final captured = Completer<(String, Map<String, Object?>)>();
    final subscription = server.listen((socket) async {
      try {
        final lines = await _readRequestLines(socket);
        final request = Map<String, Object?>.from(jsonDecode(lines[1]) as Map);
        captured.complete((lines[0], request));
        await _reply(socket, request, body: const {'status': 'running'});
      } on Object catch (error, stack) {
        if (!captured.isCompleted) captured.completeError(error, stack);
        socket.destroy();
      }
    });
    final endpoint = AuthenticatedLoopbackBrokerEndpoint.fromJson(
      _endpoint(server.port),
    );
    final transport = AuthenticatedLoopbackBrokerTransport(
      endpoint: endpoint,
    );
    try {
      final response = await transport.request(
        'AgentTask状態',
        payload: const {'task_id': 'task-1'},
      );
      final (authLine, request) = await captured.future;
      expect(authLine, _secret);
      expect(request['session_id'], _sessionId);
      expect(request['operation'], 'AgentTask状態');
      expect(
          request['payload_hash'],
          broker_contract.brokerPayloadHash(
            const {'task_id': 'task-1'},
          ));
      expect(request, isNot(contains('session_secret')));
      expect(request, isNot(contains('credential_role')));
      expect(request['metadata'], const {
        'client': 'runtime_neutral_agent_host',
      });
      expect(response['request_id'], request['request_id']);
      expect(response['body'], const {'status': 'running'});
    } finally {
      transport.dispose();
      await subscription.cancel();
      await server.close();
    }
  });

  test('Owner資格とloopback外接続情報は受理しない', () async {
    expect(
      () => AuthenticatedLoopbackBrokerEndpoint.fromJson(
        _endpoint(12345, role: 'owner'),
      ),
      throwsA(isA<BrokerClientException>()),
    );
    expect(
      () => AuthenticatedLoopbackBrokerEndpoint.fromJson(
        _endpoint(12345, host: '192.0.2.1'),
      ),
      throwsA(isA<BrokerClientException>()),
    );
    expect(
      () => AuthenticatedLoopbackBrokerEndpoint.fromJson(
        _endpoint(12345, transport: 'other'),
      ),
      throwsA(isA<BrokerClientException>()),
    );
  });

  test('接続終了後のendpointはtransport要求を拒否する', () async {
    final endpoint = AuthenticatedLoopbackBrokerEndpoint.fromJson(
      _endpoint(12345),
    );
    final transport = AuthenticatedLoopbackBrokerTransport(
      endpoint: endpoint,
    );
    endpoint.dispose();
    expect(endpoint.isDisposed, isTrue);
    await expectLater(
      transport.request('AgentTask状態', payload: const {'task_id': 't'}),
      throwsA(isA<BrokerClientException>()),
    );
    transport.dispose(disposeEndpoint: false);
  });

  test('native Owner操作はBrokerへ送らず拒否する', () async {
    var acceptedConnections = 0;
    final server = await ServerSocket.bind(InternetAddress.loopbackIPv4, 0);
    final subscription = server.listen((socket) {
      acceptedConnections++;
      socket.destroy();
    });
    final endpoint = AuthenticatedLoopbackBrokerEndpoint.fromJson(
      _endpoint(server.port),
    );
    final transport = AuthenticatedLoopbackBrokerTransport(
      endpoint: endpoint,
      timeoutForOperation: (_) => const Duration(milliseconds: 10),
    );
    try {
      for (final operation in <String>[
        'GUI Shell書出し',
        '回帰Case登録',
        '回帰Case削除',
        '回帰Case削除中断確認',
        '資格情報失効',
        'A2A接続',
        'A2Aメッセージ送信',
        'A2Aタスク状態取得',
        'A2Aストリーム送信',
        'MCP接続',
        'MCP切断',
        'MCP Tool実行',
        '更新download要求',
        '更新適用要求',
        '更新有効版切替要求',
        '更新rollback要求',
        '製品起動項目修復要求',
        '製品アンインストール要求',
        'AgentTaskWorkspacePermissionGrant',
        'AgentTaskOwnerApprovalGrant',
        'AgentTask結果表示承認',
        'AgentCLI実行系作業領域登録',
        'Host登録',
        '資格情報登録',
        '作業領域承認',
        '作業領域失効',
        '作業領域全体基準点保存',
        '作業領域OS選択',
        'AgentCLI実行fileOS選択',
        'アダプター導入',
        'アダプター更新',
        'アダプター検証',
        'アダプター有効化',
        'アダプター無効化',
        'アダプター隔離',
        'アダプター削除',
      ]) {
        await expectLater(
          transport.request(operation, payload: const {}),
          throwsA(
            isA<BrokerClientException>().having(
              (error) => error.message,
              'message',
              contains('native Owner確認'),
            ),
          ),
          reason: '$operation はnative確認経路へ委譲する',
        );
      }
      await Future<void>.delayed(Duration.zero);
      expect(acceptedConnections, 0);
    } finally {
      transport.dispose();
      await subscription.cancel();
      await server.close();
    }
  });

  test('設定済みrequest byte上限を送信前に適用する', () async {
    final endpoint = AuthenticatedLoopbackBrokerEndpoint.fromJson(
      _endpoint(12345, maxRequestBytes: 256),
    );
    final transport = AuthenticatedLoopbackBrokerTransport(
      endpoint: endpoint,
    );
    try {
      await expectLater(
        transport.request(
          'AgentTask状態',
          payload: {'data': List<String>.filled(512, 'x').join()},
        ),
        throwsA(
          isA<BrokerClientException>().having(
            (error) => error.message,
            'message',
            contains('上限'),
          ),
        ),
      );
    } finally {
      transport.dispose();
    }
  });

  test('要求IDの異なるBroker応答を拒否しsecretをerrorへ含めない', () async {
    final server = await ServerSocket.bind(InternetAddress.loopbackIPv4, 0);
    final subscription = server.listen((socket) async {
      final lines = await _readRequestLines(socket);
      final request = Map<String, Object?>.from(jsonDecode(lines[1]) as Map);
      await _reply(
        socket,
        request,
        requestId: 'runtime-host-stale-response',
      );
    });
    final endpoint = AuthenticatedLoopbackBrokerEndpoint.fromJson(
      _endpoint(server.port),
    );
    final transport = AuthenticatedLoopbackBrokerTransport(
      endpoint: endpoint,
    );
    try {
      try {
        await transport.request('AgentTask状態', payload: const {'task_id': 't'});
        fail('不一致応答が受理された');
      } on BrokerClientException catch (error) {
        expect(error.message, contains('一致しません'));
        expect(error.message, isNot(contains(_secret)));
      }
    } finally {
      transport.dispose();
      await subscription.cancel();
      await server.close();
    }
  });

  test('Broker応答byte上限を超えたframeを拒否する', () async {
    final server = await ServerSocket.bind(InternetAddress.loopbackIPv4, 0);
    final subscription = server.listen((socket) async {
      final lines = await _readRequestLines(socket);
      final request = Map<String, Object?>.from(jsonDecode(lines[1]) as Map);
      await _reply(
        socket,
        request,
        body: {'padding': List<String>.filled(256, 'x').join()},
      );
    });
    final endpoint = AuthenticatedLoopbackBrokerEndpoint.fromJson(
      _endpoint(server.port),
    );
    final transport = AuthenticatedLoopbackBrokerTransport(
      endpoint: endpoint,
      maxResponseBytes: 128,
    );
    try {
      await expectLater(
        transport.request('AgentTask状態', payload: const {'task_id': 't'}),
        throwsA(
          isA<BrokerClientException>().having(
            (error) => error.message,
            'message',
            contains('上限'),
          ),
        ),
      );
    } finally {
      transport.dispose();
      await subscription.cancel();
      await server.close();
    }
  });
}
