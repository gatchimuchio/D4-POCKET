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

  Future<Map<String, Object?>> preview({
    required Map<String, Object?>? currentManifest,
    required Map<String, Object?> candidateManifest,
    String buildTarget = 'windows',
  }) async {
    final response = await _transport.request(
      'GUI Shell構成Preview',
      payload: {
        'version': 1,
        'current_manifest': currentManifest,
        'candidate_manifest': candidateManifest,
        'build_target': buildTarget,
        'preview_mode': 'version_rollback',
      },
    );
    return _acceptedBody(response, 'GUI Shell構成Preview');
  }
}

/// 画面の改行入力をManifestの参照識別子へ変換する。
/// 空行だけを除外し、重複はBrokerの契約検証に残す。
List<String> composeReferenceIdsFromLines(String input) => input
    .split('\n')
    .map((line) => line.trim())
    .where((line) => line.isNotEmpty)
    .toList(growable: false);

Map<String, Object?> buildComposeManifestDraft({
  required String composeId,
  required String displayName,
  required String runtimeIds,
  required String agentIds,
  required String toolIds,
  required String mcpConnectionIds,
}) =>
    {
      'version': 1,
      'compose_id': composeId.trim(),
      'display_name': displayName.trim(),
      'runtime_ids': composeReferenceIdsFromLines(runtimeIds),
      'agent_ids': composeReferenceIdsFromLines(agentIds),
      'tool_ids': composeReferenceIdsFromLines(toolIds),
      'mcp_connection_ids': composeReferenceIdsFromLines(mcpConnectionIds),
      'theme': {'theme_id': 'd4-pocket', 'mode': 'system'},
      'capability_requirements': ['runtime.read', 'agent.metadata'],
      'settings': {
        'locale': 'ja-JP',
        'density': 'comfortable',
        'content_visibility': 'summary',
      },
      'inheritance_policy': {
        'authority': 'none',
        'permission': 'none',
        'approval': 'none',
        'credential': 'none',
        'audit_chain': 'none',
      },
      'output_mode': 'manifest_only',
    };

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
