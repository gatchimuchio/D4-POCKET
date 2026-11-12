import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:broker_contract/broker_request_envelope.dart'
    as broker_contract;

import 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerTransport, BrokerClientException;
export 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerTransport, BrokerClientException;

const _nativeOwnerConfirmationOperations = <String>{
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
  '更新有効版切替要求',
  '製品起動項目修復要求',
  '製品アンインストール要求',
  'AgentTaskWorkspacePermissionGrant',
  'AgentTaskOwnerApprovalGrant',
  'AgentTask結果表示承認',
  '作業領域承認',
  '作業領域失効',
  '作業領域全体基準点保存',
  'AgentCLI実行系作業領域登録',
  'アダプター導入',
  'アダプター更新',
  'アダプター検証',
  'アダプター有効化',
  'アダプター無効化',
  'アダプター隔離',
  'アダプター削除',
};

Duration brokerRequestTimeoutForOperation(String operation) =>
    operation == '更新候補取得'
        ? const Duration(seconds: 40)
        : (defaultTargetPlatform == TargetPlatform.windows ||
                    defaultTargetPlatform == TargetPlatform.linux) &&
                const {
                  'A2A接続',
                  'A2Aメッセージ送信',
                  'A2Aタスク状態取得',
                  'A2Aストリーム送信',
                }.contains(operation)
            ? const Duration(seconds: 325)
            : (defaultTargetPlatform == TargetPlatform.macOS ||
                        defaultTargetPlatform == TargetPlatform.linux) &&
                    operation == 'Host登録'
                ? const Duration(seconds: 305)
                : defaultTargetPlatform == TargetPlatform.macOS &&
                        {'MCP接続', 'MCP切断', 'MCP Tool実行'}.contains(operation)
                    ? const Duration(seconds: 335)
                    : defaultTargetPlatform == TargetPlatform.macOS &&
                            {'A2A接続', 'A2Aメッセージ送信'}.contains(operation)
                        ? const Duration(seconds: 315)
                        : defaultTargetPlatform == TargetPlatform.macOS &&
                                {
                                  'A2Aタスク状態取得',
                                  'A2Aストリーム送信',
                                }.contains(operation)
                            ? const Duration(seconds: 5)
                            : operation == '資格情報登録'
                                ? const Duration(seconds: 610)
                                : (operation == '作業領域OS選択' ||
                                        operation == 'AgentCLI実行fileOS選択')
                                    ? const Duration(seconds: 305)
                                    : operation == 'AgentCLI実行系作業領域登録'
                                        ? const Duration(seconds: 320)
                                        : _nativeOwnerConfirmationOperations
                                                .contains(operation)
                                            ? const Duration(seconds: 305)
                                            : const Duration(seconds: 5);

class BrokerClient implements BrokerTransport {
  BrokerClient._(this._channel);

  final MethodChannel _channel;
  int _counter = 0;

  static Future<BrokerClient> connect({MethodChannel? channel}) async =>
      BrokerClient._(channel ?? const MethodChannel('gui_shell/broker'));

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    _counter += 1;
    final issuedAt = _rfc3339Seconds(DateTime.now().toUtc());
    final request = broker_contract.buildBrokerRequestEnvelope(
      requestId: 'flutter-request-$_counter',
      operation: operation,
      nonce: 'flutter-nonce-${DateTime.now().microsecondsSinceEpoch}-$_counter',
      issuedAt: issuedAt,
      metadata: const {'client': 'desktop_flutter'},
      payload: payload,
    );
    final String responseText;
    try {
      final response = await _channel
          .invokeMethod<String>('request', jsonEncode(request))
          .timeout(brokerRequestTimeoutForOperation(operation));
      if (response == null || utf8.encode(response).length > 4 * 1024 * 1024) {
        throw const BrokerClientException('broker応答が空または上限超過です');
      }
      responseText = response;
    } on PlatformException {
      throw const BrokerClientException('安全Brokerとの通信に失敗しました');
    } on MissingPluginException {
      throw const BrokerClientException('安全Brokerとの通信路が利用できません');
    } on TimeoutException {
      throw const BrokerClientException('安全Brokerの応答が期限を超過しました');
    }

    final Object? decoded;
    try {
      decoded = jsonDecode(responseText);
    } on FormatException {
      throw const BrokerClientException('broker応答のJSONが不正です');
    }
    if (decoded is! Map) {
      throw const BrokerClientException('broker応答がobjectではありません');
    }
    if (decoded['request_id'] != request['request_id'] ||
        decoded['operation'] != operation) {
      throw const BrokerClientException('broker応答が現在の要求と一致しません');
    }
    return Map<String, Object?>.from(decoded);
  }
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

String brokerPayloadHash(Map<String, Object?>? payload) =>
    broker_contract.brokerPayloadHash(payload);

String brokerPayloadHashForTest(Map<String, Object?>? payload) =>
    broker_contract.brokerPayloadHash(payload);
