import 'dart:convert';

import 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerTransport, BrokerClientException;
import '../models/generated_contracts.dart';

class ComposeAgentCatalogEntry {
  const ComposeAgentCatalogEntry({
    required this.runtimeId,
    required this.agent,
  });

  final String runtimeId;
  final AgentAdapterRecord agent;
}

class ComposeRuntimeAgentCatalog {
  const ComposeRuntimeAgentCatalog({
    required this.runtimeIds,
    required this.agents,
  });

  final List<String> runtimeIds;
  final List<ComposeAgentCatalogEntry> agents;
}

class ComposeClient {
  const ComposeClient(this._transport);

  final BrokerTransport _transport;

  Future<ComposeRuntimeAgentCatalog> refreshRuntimeAgentCatalog() async {
    final response = await _transport.request(
      'GUI Shell構成Catalog',
      payload: const <String, Object?>{},
    );
    if (response['operation'] != 'GUI Shell構成Catalog' ||
        response['evidence_source'] != 'INTERNAL_STATE') {
      throw const BrokerClientException(
        'Compose CatalogのBroker識別または証拠種別が不正です',
      );
    }
    final body = _acceptedBody(response, 'GUI Shell構成Catalog');
    const allowedKeys = {
      'version',
      'runtime_ids',
      'agents',
      'evidence_source',
      'authority_strip',
      'permission_generated',
    };
    if (body.keys.any((key) => !allowedKeys.contains(key)) ||
        body.length != allowedKeys.length ||
        body['version'] != 1 ||
        body['evidence_source'] != 'INTERNAL_STATE' ||
        body['authority_strip'] != true ||
        body['permission_generated'] != false) {
      throw const BrokerClientException(
        'Compose Catalogのmetadata-only contractが不正です',
      );
    }
    final rawRuntimeIds = body['runtime_ids'];
    final rawAgents = body['agents'];
    if (rawRuntimeIds is! List ||
        rawAgents is! List ||
        rawRuntimeIds.length > 64 ||
        rawAgents.length > 64) {
      throw const BrokerClientException('Compose Catalogの件数が不正です');
    }
    final runtimeIds = <String>[];
    final runtimeIdSet = <String>{};
    for (final value in rawRuntimeIds) {
      if (value is! String ||
          !RegExp(r'^[A-Za-z0-9][A-Za-z0-9_.-]{0,127}$').hasMatch(value) ||
          !runtimeIdSet.add(value)) {
        throw const BrokerClientException(
          'Compose CatalogのRuntime IDが不正または重複しています',
        );
      }
      runtimeIds.add(value);
    }
    final agents = <ComposeAgentCatalogEntry>[];
    final agentIds = <String>{};
    final adapterIds = <String>{};
    final agentRuntimeIds = <String>{};
    for (final value in rawAgents) {
      if (value is! Map) {
        throw const BrokerClientException(
            'Compose Catalogに不正なAgent recordがあります');
      }
      final record = Map<String, Object?>.from(value);
      final runtimeId = record['runtime_id'];
      final metadata = record['metadata'];
      if (runtimeId is! String ||
          !runtimeIdSet.contains(runtimeId) ||
          metadata is! Map) {
        throw const BrokerClientException(
          'Compose CatalogのAgentとRuntimeの対応が不正です',
        );
      }
      final agent = AgentAdapterRecord.fromJson(
        Map<String, Object?>.from(metadata),
      );
      if (agent.agentId.isEmpty ||
          agent.adapterId.isEmpty ||
          !agentIds.add(agent.agentId) ||
          !adapterIds.add(agent.adapterId) ||
          !agentRuntimeIds.add(runtimeId) ||
          !const {'ready', 'degraded', 'unavailable', 'unsupported'}
              .contains(agent.status)) {
        throw const BrokerClientException(
          'Compose CatalogのAgent metadataが不正または重複しています',
        );
      }
      agents.add(ComposeAgentCatalogEntry(runtimeId: runtimeId, agent: agent));
    }
    return ComposeRuntimeAgentCatalog(
      runtimeIds: List.unmodifiable(runtimeIds),
      agents: List.unmodifiable(agents),
    );
  }

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
  String themeMode = 'system',
  String density = 'comfortable',
  String contentVisibility = 'summary',
  String capabilityRequirements = '',
}) =>
    {
      'version': 1,
      'compose_id': composeId.trim(),
      'display_name': displayName.trim(),
      'runtime_ids': composeReferenceIdsFromLines(runtimeIds),
      'agent_ids': composeReferenceIdsFromLines(agentIds),
      'tool_ids': composeReferenceIdsFromLines(toolIds),
      'mcp_connection_ids': composeReferenceIdsFromLines(mcpConnectionIds),
      'theme': {'theme_id': 'd4-pocket', 'mode': themeMode},
      'capability_requirements':
          composeReferenceIdsFromLines(capabilityRequirements),
      'settings': {
        'locale': 'ja-JP',
        'density': density,
        'content_visibility': contentVisibility,
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
