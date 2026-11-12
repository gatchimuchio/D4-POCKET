import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:math';
import 'dart:typed_data';

import 'package:broker_contract/broker_request_envelope.dart'
    as broker_contract;
import 'package:broker_contract/broker_transport.dart'
    show BrokerClientException, BrokerTransport;

const _maximumRequestBytes = 4 * 1024 * 1024;
const _maximumResponseBytes = 4 * 1024 * 1024;
const _endpointFields = <String>{
  'host',
  'port',
  'session_id',
  'session_secret',
  'credential_role',
  'transport',
  'max_request_bytes',
};
const _nativeConfirmationOperations = <String>{
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
  '作業領域承認',
  '作業領域失効',
  '作業領域全体基準点保存',
  '作業領域OS選択',
  'AgentCLI実行fileOS選択',
  'Host登録',
  '資格情報登録',
  'アダプター導入',
  'アダプター更新',
  'アダプター検証',
  'アダプター有効化',
  'アダプター無効化',
  'アダプター隔離',
  'アダプター削除',
};

/// 信頼されたnative hostから受け取る、検証範囲を限定したnormal-role接続情報。
///
/// fileを読まず、Owner資格も受け付けない。host session終了時にdisposeし、
/// 可変secret bufferを上書きする。
class AuthenticatedLoopbackBrokerEndpoint {
  AuthenticatedLoopbackBrokerEndpoint._({
    required this.port,
    required this.sessionId,
    required this.maxRequestBytes,
    required Uint8List sessionSecret,
  }) : _sessionSecret = sessionSecret;

  factory AuthenticatedLoopbackBrokerEndpoint.fromJson(Object? value) {
    if (value is! Map || value.keys.any((key) => key is! String)) {
      throw const BrokerClientException('Broker接続情報の形式が不正です');
    }
    final json = Map<String, Object?>.from(value);
    final secret = json['session_secret'];
    final sessionId = json['session_id'];
    final port = json['port'];
    final maxRequestBytes = json['max_request_bytes'];
    if (json.length != _endpointFields.length ||
        !json.keys.toSet().containsAll(_endpointFields) ||
        json['host'] != '127.0.0.1' ||
        json['credential_role'] != 'normal' ||
        json['transport'] != 'authenticated_loopback_tcp' ||
        port is! int ||
        port < 1 ||
        port > 65535 ||
        sessionId is! String ||
        !RegExp(r'^broker-session-[a-f0-9]{32}$').hasMatch(sessionId) ||
        secret is! String ||
        !RegExp(r'^[a-f0-9]{64}$').hasMatch(secret) ||
        maxRequestBytes is! int ||
        maxRequestBytes < 256 ||
        maxRequestBytes > _maximumRequestBytes) {
      throw const BrokerClientException('通常Broker接続情報を検証できません');
    }
    return AuthenticatedLoopbackBrokerEndpoint._(
      port: port,
      sessionId: sessionId,
      maxRequestBytes: maxRequestBytes,
      sessionSecret: Uint8List.fromList(ascii.encode(secret)),
    );
  }

  final int port;
  final String sessionId;
  final int maxRequestBytes;
  final Uint8List _sessionSecret;
  bool _disposed = false;

  bool get isDisposed => _disposed;

  void dispose() {
    if (_disposed) return;
    _sessionSecret.fillRange(0, _sessionSecret.length, 0);
    _disposed = true;
  }
}

typedef BrokerOperationTimeout = Duration Function(String operation);

/// 既存のnormal認証loopback endpointを使う、信頼された非Flutter host用のBrokerTransport。
/// native Owner操作は実行できない。
///
/// Brokerが権限源であり続ける。native Owner確認を要する要求はfail-closedで拒否する。
/// その操作が必要なhostは、既存platformのnative Owner確認経路へ委譲する。
class AuthenticatedLoopbackBrokerTransport implements BrokerTransport {
  AuthenticatedLoopbackBrokerTransport({
    required AuthenticatedLoopbackBrokerEndpoint endpoint,
    BrokerOperationTimeout? timeoutForOperation,
    this.connectTimeout = const Duration(seconds: 3),
    this.maxResponseBytes = _maximumResponseBytes,
  })  : _endpoint = endpoint,
        _timeoutForOperation = timeoutForOperation ?? _defaultRequestTimeout {
    if (connectTimeout <= Duration.zero ||
        maxResponseBytes < 64 ||
        maxResponseBytes > _maximumResponseBytes) {
      throw const BrokerClientException('Broker通信上限の設定が不正です');
    }
  }

  final AuthenticatedLoopbackBrokerEndpoint _endpoint;
  final BrokerOperationTimeout _timeoutForOperation;
  final Duration connectTimeout;
  final int maxResponseBytes;
  bool _disposed = false;

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    if (_disposed || _endpoint.isDisposed) {
      throw const BrokerClientException('Broker接続は終了しています');
    }
    if (operation.isEmpty || operation.length > 128) {
      throw const BrokerClientException('Broker操作名の形式が不正です');
    }
    if (_nativeConfirmationOperations.contains(operation)) {
      throw const BrokerClientException(
        'この非GUI transportはnative Owner確認を実行できません',
      );
    }

    final timeout = _timeoutForOperation(operation);
    if (timeout <= Duration.zero) {
      throw const BrokerClientException('Broker応答期限の設定が不正です');
    }
    final requestId = 'runtime-host-${_randomHex(16)}';
    final envelope = broker_contract.buildBrokerRequestEnvelope(
      requestId: requestId,
      operation: operation,
      nonce: _randomHex(32),
      issuedAt: _rfc3339Seconds(DateTime.now().toUtc()),
      metadata: const {'client': 'runtime_neutral_agent_host'},
      payload: payload,
    );
    envelope['session_id'] = operation == 'health' ? null : _endpoint.sessionId;

    final List<int> requestBytes;
    try {
      requestBytes = utf8.encode(jsonEncode(envelope));
    } on Object {
      throw const BrokerClientException('Broker要求をJSON化できません');
    }
    if (requestBytes.length + 1 > _endpoint.maxRequestBytes) {
      throw const BrokerClientException('Broker要求が設定上限を超過しています');
    }

    Socket? socket;
    final sessionSecret = Uint8List.fromList(_endpoint._sessionSecret);
    try {
      socket = await Socket.connect(
        InternetAddress.loopbackIPv4,
        _endpoint.port,
        timeout: connectTimeout,
      );
      socket.add(sessionSecret);
      socket.add(const [0x0a]);
      socket.add(requestBytes);
      socket.add(const [0x0a]);
      await socket.flush().timeout(timeout);
      final responseBytes = await _readResponseLine(
        socket,
        timeout,
        maxResponseBytes,
      );
      final Object? decoded;
      try {
        decoded = jsonDecode(utf8.decode(responseBytes));
      } on Object {
        throw const BrokerClientException('Broker応答のJSONが不正です');
      }
      if (decoded is! Map || decoded.keys.any((key) => key is! String)) {
        throw const BrokerClientException('Broker応答の形式が不正です');
      }
      final response = Map<String, Object?>.from(decoded);
      if (response['request_id'] != requestId ||
          response['operation'] != operation) {
        throw const BrokerClientException('Broker応答が現在の要求と一致しません');
      }
      return response;
    } on BrokerClientException {
      rethrow;
    } on TimeoutException {
      throw const BrokerClientException('Broker応答が期限を超過しました');
    } on SocketException {
      throw const BrokerClientException('認証Brokerとの通信に失敗しました');
    } on Object {
      throw const BrokerClientException('認証Brokerとの通信に失敗しました');
    } finally {
      socket?.destroy();
      sessionSecret.fillRange(0, sessionSecret.length, 0);
      requestBytes.fillRange(0, requestBytes.length, 0);
    }
  }

  void dispose({bool disposeEndpoint = true}) {
    _disposed = true;
    if (disposeEndpoint) _endpoint.dispose();
  }
}

Duration _defaultRequestTimeout(String operation) => switch (operation) {
      'MCP接続' || 'MCP切断' || 'MCP Tool実行' => const Duration(seconds: 305),
      'AgentTaskWorkspacePermissionGrant' ||
      'AgentTaskOwnerApprovalGrant' ||
      'AgentTask結果表示承認' ||
      '作業領域承認' ||
      '作業領域失効' ||
      '作業領域基準点保存' ||
      '作業領域全体基準点保存' =>
        const Duration(seconds: 305),
      '資格情報登録' => const Duration(seconds: 610),
      '更新候補取得' => const Duration(seconds: 40),
      _ => const Duration(seconds: 5),
    };

Future<List<int>> _readResponseLine(
  Socket socket,
  Duration timeout,
  int maxResponseBytes,
) async {
  final completer = Completer<List<int>>();
  final bytes = <int>[];
  late final StreamSubscription<List<int>> subscription;
  final timer = Timer(timeout, () {
    if (!completer.isCompleted) {
      completer.completeError(
        const BrokerClientException('Broker応答が期限を超過しました'),
      );
      unawaited(subscription.cancel());
    }
  });
  subscription = socket.listen(
    (chunk) {
      if (completer.isCompleted) return;
      final newline = chunk.indexOf(0x0a);
      final count = newline < 0 ? chunk.length : newline;
      if (bytes.length + count > maxResponseBytes) {
        completer.completeError(
          const BrokerClientException('Broker応答が上限を超過しています'),
        );
      } else if (newline >= 0 && newline + 1 != chunk.length) {
        completer.completeError(
          const BrokerClientException('Broker応答frameに余分なdataがあります'),
        );
      } else {
        bytes.addAll(chunk.take(count));
        if (newline >= 0) {
          if (bytes.isNotEmpty && bytes.last == 0x0d) bytes.removeLast();
          if (bytes.length > maxResponseBytes) {
            completer.completeError(
              const BrokerClientException('Broker応答が上限を超過しています'),
            );
          } else {
            completer.complete(bytes);
          }
        }
      }
      if (completer.isCompleted) unawaited(subscription.cancel());
    },
    onError: (Object _, StackTrace stack) {
      if (!completer.isCompleted) {
        completer.completeError(
          const BrokerClientException('認証Brokerとの通信に失敗しました'),
          stack,
        );
      }
    },
    onDone: () {
      if (!completer.isCompleted) {
        completer.completeError(
          const BrokerClientException('Broker応答が完了前に切断されました'),
        );
      }
    },
  );
  try {
    return await completer.future;
  } finally {
    timer.cancel();
    await subscription.cancel();
  }
}

String _randomHex(int byteCount) {
  final random = Random.secure();
  return List<int>.generate(
    byteCount,
    (_) => random.nextInt(256),
  ).map((byte) => byte.toRadixString(16).padLeft(2, '0')).join();
}

String _rfc3339Seconds(DateTime value) {
  final year = value.year.toString().padLeft(4, '0');
  final month = value.month.toString().padLeft(2, '0');
  final day = value.day.toString().padLeft(2, '0');
  final hour = value.hour.toString().padLeft(2, '0');
  final minute = value.minute.toString().padLeft(2, '0');
  final second = value.second.toString().padLeft(2, '0');
  return '$year-$month-${day}T$hour:$minute:${second}Z';
}
