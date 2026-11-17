import 'dart:convert';

import 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerTransport, BrokerClientException;
import '../models/generated_contracts.dart';

class ComposeAgentCatalogEntry {
  const ComposeAgentCatalogEntry({
    required this.runtimeId,
    required this.agent,
    required this.workspaceMode,
    required this.hostPlatforms,
    required this.networkScope,
  });

  final String runtimeId;
  final AgentAdapterRecord agent;
  final String workspaceMode;
  final List<String> hostPlatforms;
  final String networkScope;
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

  static const _inheritanceFields = {
    'authority',
    'permission',
    'approval',
    'credential',
    'audit_chain',
  };

  static const _composeManifestFields = {
    'version',
    'compose_id',
    'display_name',
    'runtime_ids',
    'agent_ids',
    'tool_ids',
    'mcp_connection_ids',
    'theme',
    'capability_requirements',
    'settings',
    'inheritance_policy',
    'output_mode',
  };

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
    final adapterByAgentId = <String, String>{};
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
      final metadataMap = Map<String, Object?>.from(metadata);
      final workspace = _objectMap(metadataMap['workspace_requirements']);
      final host = _objectMap(metadataMap['host_requirements']);
      const workspaceFields = {'mode', 'boundary_policy', 'secret_paths'};
      const validModes = {'required', 'optional'};
      const validPlatforms = {
        'windows',
        'macos',
        'linux',
        'android',
        'ios',
        'unknown'
      };
      const validNetworkScopes = {
        'loopback_only',
        'outbound_allowed',
        'unsupported',
        'unknown',
      };
      const hostFields = {'platforms', 'network_scope', 'process_spawn'};
      final secretPaths = workspace?['secret_paths'];
      final rawPlatforms = host?['platforms'];
      if (workspace == null ||
          workspace.length != workspaceFields.length ||
          !workspace.keys.toSet().containsAll(workspaceFields) ||
          !validModes.contains(workspace['mode']) ||
          workspace['boundary_policy'] != 'deny_outside_workspace' ||
          secretPaths is! List ||
          secretPaths.length > 64 ||
          secretPaths.any(
              (path) => path is! String || path.isEmpty || path.length > 256) ||
          host == null ||
          host.length != hostFields.length ||
          !host.keys.toSet().containsAll(hostFields) ||
          rawPlatforms is! List ||
          rawPlatforms.isEmpty ||
          rawPlatforms.length > 8 ||
          rawPlatforms.any((platform) =>
              platform is! String || !validPlatforms.contains(platform)) ||
          !validNetworkScopes.contains(host['network_scope'])) {
        throw const BrokerClientException(
          'Compose CatalogのWorkspace／Host要件が不正です',
        );
      }
      final agent = AgentAdapterRecord.fromJson(metadataMap);
      final existingAdapterId = adapterByAgentId.putIfAbsent(
        agent.agentId,
        () => agent.adapterId,
      );
      if (agent.agentId.isEmpty ||
          agent.adapterId.isEmpty ||
          existingAdapterId != agent.adapterId ||
          !agentRuntimeIds.add(runtimeId) ||
          !const {'ready', 'degraded', 'unavailable', 'unsupported'}
              .contains(agent.status)) {
        throw const BrokerClientException(
          'Compose CatalogのAgent metadataが不正または重複しています',
        );
      }
      agents.add(ComposeAgentCatalogEntry(
        runtimeId: runtimeId,
        agent: agent,
        workspaceMode: workspace['mode']! as String,
        hostPlatforms: List.unmodifiable(rawPlatforms.cast<String>()),
        networkScope: host['network_scope']! as String,
      ));
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
    final body = _acceptedAuditedBody(response, 'GUI Shell構成');
    const fields = {
      'version',
      'operation',
      'status',
      'compose_manifest',
      'build_status',
      'app_identity_status',
      'permission_generated',
      'authority_strip',
      'inheritance_policy',
      'evidence_source',
      'audit_id',
    };
    final acceptedManifest = _objectMap(body['compose_manifest']);
    if (!_hasExactKeys(body, fields) ||
        body['version'] != 1 ||
        body['operation'] != 'GUI Shell構成' ||
        body['status'] != 'accepted' ||
        body['build_status'] != 'not_started' ||
        body['app_identity_status'] != 'not_generated' ||
        body['permission_generated'] != false ||
        body['authority_strip'] != true ||
        !_hasNoInheritedAuthority(body['inheritance_policy']) ||
        body['evidence_source'] != 'INTERNAL_STATE' ||
        acceptedManifest == null ||
        !_isComposeManifest(acceptedManifest) ||
        !_sameJson(acceptedManifest, manifest)) {
      throw const BrokerClientException(
        'GUI Shell構成receiptが要求またはAuthority境界と一致しません',
      );
    }
    return body;
  }

  Future<Map<String, Object?>> preview({
    required Map<String, Object?>? currentManifest,
    required Map<String, Object?> candidateManifest,
    String buildTarget = 'windows',
  }) async {
    if (!const {'windows', 'desktop', 'mobile'}.contains(buildTarget)) {
      throw const BrokerClientException('Compose Preview対象platformが不正です');
    }
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
    final body = _acceptedAuditedBody(response, 'GUI Shell構成Preview');
    const fields = {
      'version',
      'operation',
      'status',
      'current_manifest',
      'candidate_manifest',
      'build_target',
      'preview_mode',
      'diff',
      'permission_requirements',
      'version_preview',
      'build_status',
      'export_status',
      'authority_strip',
      'inheritance_policy',
      'evidence_source',
      'audit_id',
    };
    final current = body['current_manifest'];
    final currentMap = current == null ? null : _objectMap(current);
    final candidate = _objectMap(body['candidate_manifest']);
    if (!_hasExactKeys(body, fields) ||
        body['version'] != 1 ||
        body['operation'] != 'GUI Shell構成Preview' ||
        body['status'] != 'accepted' ||
        body['build_target'] != buildTarget ||
        body['preview_mode'] != 'version_rollback' ||
        body['build_status'] != 'not_started' ||
        body['export_status'] != 'not_started' ||
        body['authority_strip'] != true ||
        !_hasNoInheritedAuthority(body['inheritance_policy']) ||
        body['evidence_source'] != 'INTERNAL_STATE' ||
        (current != null &&
            (currentMap == null || !_isComposeManifest(currentMap))) ||
        !_sameJson(current, currentManifest) ||
        candidate == null ||
        !_isComposeManifest(candidate) ||
        !_sameJson(candidate, candidateManifest) ||
        !_validPreviewDiff(body['diff']) ||
        !_validPreviewPermissions(
          body['permission_requirements'],
          candidate['capability_requirements'] as List,
        ) ||
        !_validVersionPreview(body['version_preview'], current != null)) {
      throw const BrokerClientException(
        'GUI Shell構成Preview receiptが要求またはAuthority境界と一致しません',
      );
    }
    return body;
  }

  Map<String, Object?> _acceptedAuditedBody(
    Map<String, Object?> response,
    String operation,
  ) {
    final body = _acceptedBody(response, operation);
    final auditId = response['audit_event_id'];
    if (response['evidence_source'] != 'INTERNAL_STATE' ||
        auditId is! String ||
        !_validAuditId(auditId) ||
        body['audit_id'] != auditId) {
      throw BrokerClientException('$operation 応答の証拠または監査参照が不正です');
    }
    return body;
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
  if (response['operation'] != operation) {
    throw BrokerClientException('$operation 応答の操作識別が不正です');
  }
  if (response['status'] != 'accepted') {
    final error = response['error'];
    final code = error is Map ? error['code'] : null;
    final safeCode =
        code is String && RegExp(r'^[a-z0-9_]{1,96}$').hasMatch(code)
            ? ' ($code)'
            : '';
    throw BrokerClientException('$operation がBrokerに拒否されました$safeCode');
  }
  if (response['error'] != null) {
    throw BrokerClientException('$operation accepted応答にerrorが含まれています');
  }
  final body = response['body'];
  final result = _objectMap(body);
  if (result == null) {
    throw BrokerClientException('$operation 応答bodyがobjectではありません');
  }
  return result;
}

Map<String, Object?>? _objectMap(Object? value) {
  if (value is! Map || value.keys.any((key) => key is! String)) return null;
  return Map<String, Object?>.from(value);
}

bool _hasExactKeys(Map<String, Object?> value, Set<String> expected) =>
    value.length == expected.length &&
    value.keys.every((key) => expected.contains(key));

bool _validAuditId(String value) =>
    RegExp(r'^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$').hasMatch(value);

bool _hasNoInheritedAuthority(Object? value) {
  final policy = _objectMap(value);
  return policy != null &&
      policy.length == ComposeClient._inheritanceFields.length &&
      policy.keys.every(ComposeClient._inheritanceFields.contains) &&
      policy.values.every((item) => item == 'none');
}

bool _isComposeManifest(Map<String, Object?> value) {
  if (value.length != ComposeClient._composeManifestFields.length ||
      !value.keys.every(ComposeClient._composeManifestFields.contains) ||
      value['version'] != 1 ||
      value['output_mode'] != 'manifest_only' ||
      !_hasNoInheritedAuthority(value['inheritance_policy'])) {
    return false;
  }
  final composeId = value['compose_id'];
  final displayName = value['display_name'];
  if (composeId is! String ||
      !RegExp(r'^[A-Za-z0-9._-]{1,64}$').hasMatch(composeId) ||
      displayName is! String ||
      displayName.trim().isEmpty ||
      displayName.runes.length > 128) {
    return false;
  }
  if (!_isUniqueStringList(value['runtime_ids'], 32, 128) ||
      !_isUniqueStringList(value['agent_ids'], 32, 128) ||
      !_isUniqueStringList(value['tool_ids'], 64, 147) ||
      !_isUniqueStringList(value['mcp_connection_ids'], 32, 128) ||
      !_isUniqueStringList(value['capability_requirements'], 64, 128)) {
    return false;
  }
  final theme = _objectMap(value['theme']);
  if (theme == null ||
      !_hasExactKeys(theme, const {'theme_id', 'mode'}) ||
      theme['theme_id'] is! String ||
      (theme['theme_id']! as String).trim().isEmpty ||
      (theme['theme_id']! as String).runes.length > 64 ||
      !const {'system', 'light', 'dark'}.contains(theme['mode'])) {
    return false;
  }
  final settings = _objectMap(value['settings']);
  if (settings == null ||
      !_hasExactKeys(
        settings,
        const {'locale', 'density', 'content_visibility'},
      ) ||
      settings['locale'] is! String ||
      !RegExp(r'^[A-Za-z]{2}-[A-Za-z]{2}$')
          .hasMatch(settings['locale']! as String) ||
      !const {'compact', 'comfortable'}.contains(settings['density']) ||
      !const {'none', 'hash_only', 'summary', 'redacted', 'full'}
          .contains(settings['content_visibility'])) {
    return false;
  }
  return true;
}

bool _isUniqueStringList(Object? value, int maxItems, int maxCharacters) {
  if (value is! List || value.length > maxItems) return false;
  final items = <String>[];
  for (final item in value) {
    if (item is! String ||
        item.trim().isEmpty ||
        item.runes.length > maxCharacters) {
      return false;
    }
    items.add(item);
  }
  return items.toSet().length == items.length;
}

bool _validPreviewDiff(Object? value) {
  final diff = _objectMap(value);
  if (diff == null ||
      !_hasExactKeys(diff, const {
        'runtime_added',
        'runtime_removed',
        'agent_added',
        'agent_removed',
        'tool_added',
        'tool_removed',
        'mcp_added',
        'mcp_removed',
        'capability_added',
        'capability_removed',
        'changed_fields',
      })) {
    return false;
  }
  for (final entry in diff.entries) {
    final maximum = entry.key.startsWith('tool_')
        ? 64
        : entry.key.startsWith('capability_')
            ? 64
            : entry.key == 'changed_fields'
                ? 32
                : 32;
    if (!_isUniqueStringList(entry.value, maximum, 147)) return false;
  }
  return true;
}

bool _validPreviewPermissions(Object? value, List requirements) {
  if (value is! List ||
      value.length != requirements.length ||
      value.length > 64) {
    return false;
  }
  for (var index = 0; index < value.length; index++) {
    final item = _objectMap(value[index]);
    if (item == null ||
        !_hasExactKeys(
          item,
          const {'capability', 'permission_status', 'approval_status'},
        ) ||
        item['capability'] != requirements[index] ||
        item['permission_status'] != 'not_generated' ||
        item['approval_status'] != 'not_requested') {
      return false;
    }
  }
  return true;
}

bool _validVersionPreview(Object? value, bool hasCurrentManifest) {
  final preview = _objectMap(value);
  return preview != null &&
      _hasExactKeys(
        preview,
        const {
          'current_version',
          'candidate_version',
          'rollback_available',
          'rollback_status',
          'rollback_reason',
        },
      ) &&
      preview['current_version'] == (hasCurrentManifest ? 1 : null) &&
      preview['candidate_version'] == 1 &&
      preview['rollback_available'] == false &&
      preview['rollback_status'] == 'not_executable' &&
      preview['rollback_reason'] == 'Previewはrollback要求を実行しない';
}

bool _sameJson(Object? left, Object? right) {
  try {
    return jsonEncode(_orderedJson(left)) == jsonEncode(_orderedJson(right));
  } on Object {
    return false;
  }
}

Object? _orderedJson(Object? value) {
  if (value is Map) {
    if (value.keys.any((key) => key is! String)) {
      throw const FormatException('JSON object key must be a string');
    }
    final keys = value.keys.cast<String>().toList()..sort();
    return <String, Object?>{
      for (final key in keys) key: _orderedJson(value[key]),
    };
  }
  if (value is List) return value.map(_orderedJson).toList(growable: false);
  if (value == null || value is String || value is bool || value is num) {
    return value;
  }
  throw const FormatException('Value is not JSON-compatible');
}
