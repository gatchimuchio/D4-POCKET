import 'dart:convert';

import 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerClientException, BrokerTransport;

class AiEditClient {
  const AiEditClient(this._transport);

  final BrokerTransport _transport;

  Future<Map<String, Object?>> propose({
    required String editId,
    required String scope,
    required String instruction,
    required List<String> targetPaths,
    required List<String> expectedChanges,
  }) async {
    final response = await _transport.request(
      'GUI Shell編集提案',
      payload: {
        'version': 1,
        'edit_id': editId,
        'scope': scope,
        'instruction': instruction,
        'target_paths': targetPaths,
        'expected_changes': expectedChanges,
        'source': 'owner_directed_agent',
        'repository_rules_acknowledged': true,
        'self_approval': false,
        'execution_mode': 'proposal_only',
      },
    );
    if (response['status'] != 'accepted') {
      final error = response['error'];
      final message = error is Map
          ? error['message']?.toString() ?? response.toString()
          : response.toString();
      throw BrokerClientException('GUI Shell編集提案が拒否されました: $message');
    }
    final body = response['body'];
    if (body is! Map) {
      throw const BrokerClientException('GUI Shell編集提案応答bodyがobjectではありません');
    }
    return Map<String, Object?>.from(body);
  }
}

String aiEditJson(Map<String, Object?> value) =>
    const JsonEncoder.withIndent('  ').convert(value);
