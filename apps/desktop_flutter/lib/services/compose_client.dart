import 'dart:convert';

import 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerTransport, BrokerClientException;

class ComposeClient {
  const ComposeClient(this._transport);

  final BrokerTransport _transport;

  Future<Map<String, Object?>> compose(Map<String, Object?> manifest) async {
    final response = await _transport.request(
      'GUI Shell構成',
      payload: manifest,
    );
    return _acceptedBody(response, 'GUI Shell構成');
  }
}

String composeJson(Map<String, Object?> value) =>
    const JsonEncoder.withIndent('  ').convert(value);

Map<String, Object?> _acceptedBody(
  Map<String, Object?> response,
  String operation,
) {
  if (response['status'] != 'accepted') {
    final error = response['error'];
    final message = error is Map
        ? error['message']?.toString() ?? response.toString()
        : response.toString();
    throw BrokerClientException('$operation が拒否されました: $message');
  }
  final body = response['body'];
  if (body is! Map) {
    throw BrokerClientException('$operation 応答bodyがobjectではありません');
  }
  return Map<String, Object?>.from(body);
}
