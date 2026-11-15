import 'dart:convert';

import 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerClientException, BrokerTransport;

const Map<String, String> guiShellOptionalExportModules = {
  'shell.setup_doctor': '環境診断',
  'shell.history': '実行履歴',
  'shell.evaluation_lab': '評価ラボ',
  'shell.host_capabilities': 'ホスト能力',
  'shell.notifications': '通知',
  'shell.observability': '観測',
  'shell.trace_inspector': '追跡情報',
  'shell.host_operations': 'Host操作',
};

class ExportClient {
  const ExportClient(this._transport);

  final BrokerTransport _transport;

  static const _optionalModuleIds = {
    'shell.setup_doctor',
    'shell.history',
    'shell.evaluation_lab',
    'shell.host_capabilities',
    'shell.notifications',
    'shell.observability',
    'shell.trace_inspector',
    'shell.host_operations',
  };

  static const _unprunableCoreIds = {
    'core.security_broker',
    'core.cryptography',
    'core.audit_finality',
    'core.credentials',
    'core.os_controls',
    'core.permission_enforcement',
    'core.approval_enforcement',
    'core.content_exposure',
  };

  static const _mandatoryModuleIds = {
    'shell.dashboard',
    'shell.runtime_operation',
    'shell.agent_operation',
    'shell.authority',
    'shell.approval',
    'shell.audit',
    'shell.recovery',
    'shell.problems',
    'shell.evidence',
    'shell.settings',
  };

  static const _moduleDependencies = {
    'shell.trace_inspector': {'shell.observability'},
  };

  Future<Map<String, Object?>> export({
    required String exportId,
    required Map<String, Object?> composeManifest,
    String targetPlatform = 'windows',
    String distributionChannel = 'local',
    List<String>? optionalModuleIds,
    String? updatePublicKeyDerHex,
    String? updatePackageSourceUrl,
    String updatePackageSourceChannel = 'stable',
  }) async {
    final updateTrustConfigured =
        (updatePublicKeyDerHex?.trim().isNotEmpty ?? false) ||
            (updatePackageSourceUrl?.trim().isNotEmpty ?? false);
    const operation = 'GUI Shell書出し';
    final response = await _transport.request(
      operation,
      payload: {
        'version': 1,
        'export_id': exportId,
        'compose_manifest': composeManifest,
        'target_platform': targetPlatform,
        'export_mode': 'manifest_file',
        'distribution_channel': distributionChannel,
        if (optionalModuleIds != null)
          'module_selection': {
            'optional_module_ids': optionalModuleIds,
          },
        if (updateTrustConfigured)
          'update_trust': {
            'public_key_der_hex': updatePublicKeyDerHex?.trim() ?? '',
            'package_sources': [
              {
                'channel': updatePackageSourceChannel,
                'base_url': updatePackageSourceUrl?.trim() ?? '',
              },
            ],
          },
      },
    );
    if (response['operation'] != operation) {
      throw const BrokerClientException('GUI Shell書出し応答の操作識別が不正です');
    }
    if (response['status'] != 'accepted') {
      final error = response['error'];
      final code = error is Map ? error['code'] : null;
      if (code == 'owner_required') {
        throw const BrokerClientException(
          'Owner確認が完了しなかったため、書出しは拒否されました。',
        );
      }
      final safeCode =
          code is String && RegExp(r'^[a-z0-9_]{1,96}$').hasMatch(code)
              ? ' ($code)'
              : '';
      throw BrokerClientException('GUI Shell書出しがBrokerに拒否されました$safeCode');
    }
    if (response['error'] != null ||
        response['evidence_source'] != 'INTERNAL_STATE' ||
        response['audit_event_id'] is! String ||
        !_validAuditId(response['audit_event_id'] as String)) {
      throw const BrokerClientException('GUI Shell書出し応答の証拠または監査参照が不正です');
    }
    final body = response['body'];
    final receipt = _objectMap(body);
    if (receipt == null ||
        !_validExportReceipt(
          receipt,
          exportId: exportId,
          composeManifest: composeManifest,
          targetPlatform: targetPlatform,
          distributionChannel: distributionChannel,
          optionalModuleIds: optionalModuleIds,
          auditEventId: response['audit_event_id'] as String,
        )) {
      throw const BrokerClientException(
        'GUI Shell書出しreceiptが要求・監査・Module・Authority境界と一致しません',
      );
    }
    return receipt;
  }
}

bool _validExportReceipt(
  Map<String, Object?> value, {
  required String exportId,
  required Map<String, Object?> composeManifest,
  required String targetPlatform,
  required String distributionChannel,
  required List<String>? optionalModuleIds,
  required String auditEventId,
}) {
  const receiptFields = {
    'version',
    'operation',
    'status',
    'export_id',
    'export_manifest',
    'manifest_file_status',
    'manifest_file',
    'build_status',
    'artifact_status',
    'credential_inherited',
    'permission_inherited',
    'approval_inherited',
    'audit_chain_inherited',
    'authority_strip',
    'evidence_source',
    'audit_id',
  };
  if (!_hasExactKeys(value, receiptFields) ||
      value['version'] != 1 ||
      value['operation'] != 'GUI Shell書出し' ||
      value['status'] != 'accepted' ||
      value['export_id'] != exportId ||
      value['manifest_file_status'] != 'written' ||
      value['build_status'] != 'not_started' ||
      value['artifact_status'] != 'not_built' ||
      value['credential_inherited'] != false ||
      value['permission_inherited'] != false ||
      value['approval_inherited'] != false ||
      value['audit_chain_inherited'] != false ||
      value['authority_strip'] != true ||
      value['evidence_source'] != 'INTERNAL_STATE' ||
      value['audit_id'] != auditEventId) {
    return false;
  }

  final manifest = _objectMap(value['export_manifest']);
  final appIdentity =
      manifest == null ? null : _objectMap(manifest['app_identity']);
  final auditStore =
      manifest == null ? null : _objectMap(manifest['audit_store']);
  final settings = manifest == null ? null : _objectMap(manifest['settings']);
  final runtimeManifest =
      manifest == null ? null : _objectMap(manifest['runtime_manifest']);
  final adapterConfiguration =
      manifest == null ? null : _objectMap(manifest['adapter_configuration']);
  final distribution =
      manifest == null ? null : _objectMap(manifest['distribution_metadata']);
  final modulePlan =
      manifest == null ? null : _objectMap(manifest['module_plan']);
  final file = _objectMap(value['manifest_file']);
  if (manifest == null ||
      !_hasExactKeys(manifest, const {
        'app_identity',
        'audit_store',
        'settings',
        'runtime_manifest',
        'adapter_configuration',
        'capability_requirements',
        'distribution_metadata',
        'inheritance_policy',
        'module_plan',
      }) ||
      appIdentity == null ||
      !_hasExactKeys(appIdentity, const {
        'app_id',
        'display_name',
        'target_platform',
      }) ||
      auditStore == null ||
      !_hasExactKeys(auditStore, const {
        'store_id',
        'chain_status',
        'inherited',
      }) ||
      settings == null ||
      !_hasExactKeys(settings, const {
        'locale',
        'density',
        'content_visibility',
      }) ||
      runtimeManifest == null ||
      !_hasExactKeys(runtimeManifest, const {'runtime_ids'}) ||
      adapterConfiguration == null ||
      !_hasExactKeys(adapterConfiguration, const {
        'agent_ids',
        'tool_ids',
        'mcp_connection_ids',
      }) ||
      distribution == null ||
      !_hasExactKeys(distribution, const {
        'target_platform',
        'artifact_status',
        'installer_status',
        'signed',
        'channel',
      }) ||
      modulePlan == null ||
      !_validExportManifest(
        manifest,
        appIdentity: appIdentity,
        auditStore: auditStore,
        settings: settings,
        runtimeManifest: runtimeManifest,
        adapterConfiguration: adapterConfiguration,
        distribution: distribution,
        modulePlan: modulePlan,
        composeManifest: composeManifest,
        targetPlatform: targetPlatform,
        distributionChannel: distributionChannel,
        optionalModuleIds: optionalModuleIds,
      ) ||
      file == null ||
      !_validManifestFile(
        file,
        appId: appIdentity['app_id'] as String,
      )) {
    return false;
  }
  return true;
}

bool _validExportManifest(
  Map<String, Object?> manifest, {
  required Map<String, Object?> appIdentity,
  required Map<String, Object?> auditStore,
  required Map<String, Object?> settings,
  required Map<String, Object?> runtimeManifest,
  required Map<String, Object?> adapterConfiguration,
  required Map<String, Object?> distribution,
  required Map<String, Object?> modulePlan,
  required Map<String, Object?> composeManifest,
  required String targetPlatform,
  required String distributionChannel,
  required List<String>? optionalModuleIds,
}) {
  final appId = appIdentity['app_id'];
  final storeId = auditStore['store_id'];
  final requested = optionalModuleIds == null
      ? ExportClient._optionalModuleIds
      : optionalModuleIds.toSet();
  if (optionalModuleIds != null &&
      (requested.length != optionalModuleIds.length ||
          !requested.every(ExportClient._optionalModuleIds.contains))) {
    return false;
  }
  if (appId is! String ||
      !RegExp(r'^d4-pocket-app-[0-9a-f]{32}$').hasMatch(appId) ||
      storeId is! String ||
      !RegExp(r'^audit-store-[0-9a-f]{32}$').hasMatch(storeId) ||
      appIdentity['display_name'] != composeManifest['display_name'] ||
      appIdentity['target_platform'] != targetPlatform ||
      auditStore['chain_status'] != 'new' ||
      auditStore['inherited'] != false ||
      !_sameJson(settings, composeManifest['settings']) ||
      !_sameJson(
          runtimeManifest, {'runtime_ids': composeManifest['runtime_ids']}) ||
      !_sameJson(
        adapterConfiguration,
        {
          'agent_ids': composeManifest['agent_ids'],
          'tool_ids': composeManifest['tool_ids'],
          'mcp_connection_ids': composeManifest['mcp_connection_ids'],
        },
      ) ||
      !_sameJson(
        manifest['capability_requirements'],
        composeManifest['capability_requirements'],
      ) ||
      distribution['target_platform'] != targetPlatform ||
      distribution['artifact_status'] != 'not_built' ||
      distribution['installer_status'] != 'not_started' ||
      distribution['signed'] != false ||
      distribution['channel'] != distributionChannel ||
      !_hasNoInheritedAuthority(manifest['inheritance_policy'])) {
    return false;
  }
  return _validModulePlan(modulePlan, requested, optionalModuleIds == null);
}

bool _validModulePlan(
  Map<String, Object?> value,
  Set<String> requested,
  bool allOptional,
) {
  const fields = {
    'catalog_version',
    'selection_mode',
    'requested_optional_module_ids',
    'unprunable_core_ids',
    'mandatory_module_ids',
    'included_module_ids',
    'excluded_optional_module_ids',
    'binary_pruning_status',
  };
  if (!_hasExactKeys(value, fields) ||
      value['catalog_version'] != 1 ||
      value['selection_mode'] !=
          (allOptional ? 'all_optional' : 'explicit_optional') ||
      value['binary_pruning_status'] != 'not_applied') {
    return false;
  }
  final requestedList = _stringList(value['requested_optional_module_ids']);
  final coreList = _stringList(value['unprunable_core_ids']);
  final mandatoryList = _stringList(value['mandatory_module_ids']);
  final includedList = _stringList(value['included_module_ids']);
  final excludedList = _stringList(value['excluded_optional_module_ids']);
  if (requestedList == null ||
      coreList == null ||
      mandatoryList == null ||
      includedList == null ||
      excludedList == null ||
      !_sameStringSet(requestedList, requested) ||
      !_sameStringSet(coreList, ExportClient._unprunableCoreIds) ||
      !_sameStringSet(mandatoryList, ExportClient._mandatoryModuleIds)) {
    return false;
  }
  final expectedOptional = <String>{};
  void include(String id) {
    if (!expectedOptional.add(id)) return;
    for (final dependency
        in ExportClient._moduleDependencies[id] ?? const <String>{}) {
      include(dependency);
    }
  }

  for (final id in requested) {
    include(id);
  }
  final excluded = ExportClient._optionalModuleIds.difference(expectedOptional);
  final expectedIncluded = {
    ...ExportClient._unprunableCoreIds,
    ...ExportClient._mandatoryModuleIds,
    ...expectedOptional,
  };
  final expectedExcluded = excluded;
  return _sameStringSet(includedList, expectedIncluded) &&
      _sameStringSet(excludedList, expectedExcluded);
}

bool _validManifestFile(Map<String, Object?> value, {required String appId}) {
  const fields = {
    'file_name',
    'path',
    'sha256',
    'byte_length',
    'temporary_file_status',
    'temporary_file_name',
    'recovery_action',
  };
  final suffix = appId.substring('d4-pocket-app-'.length);
  final fileName = '$appId.json';
  final path = value['path'];
  final temporaryStatus = value['temporary_file_status'];
  final recoveryAction = value['recovery_action'];
  final temporaryName = value['temporary_file_name'];
  final normalizedPath = path is String ? path.replaceAll('\\', '/') : '';
  final isAbsolute = normalizedPath.startsWith('/') ||
      RegExp(r'^[A-Za-z]:/').hasMatch(normalizedPath) ||
      normalizedPath.startsWith('//');
  final expectedRecovery = temporaryStatus == 'removed'
      ? 'none'
      : 'owner_review_and_remove_after_hash_verification';
  return _hasExactKeys(value, fields) &&
      value['file_name'] == fileName &&
      path is String &&
      path.isNotEmpty &&
      path.runes.length <= 2048 &&
      isAbsolute &&
      normalizedPath.endsWith('/$fileName') &&
      !normalizedPath.split('/').contains('..') &&
      value['sha256'] is String &&
      RegExp(r'^sha256:[0-9a-f]{64}$').hasMatch(value['sha256']! as String) &&
      value['byte_length'] is int &&
      (value['byte_length']! as int) >= 1 &&
      (value['byte_length']! as int) <= 65536 &&
      const {'removed', 'cleanup_pending'}.contains(temporaryStatus) &&
      temporaryName == '.$suffix.manifest.tmp' &&
      recoveryAction == expectedRecovery;
}

bool _hasNoInheritedAuthority(Object? value) {
  final policy = _objectMap(value);
  const fields = {
    'authority',
    'permission',
    'approval',
    'credential',
    'audit_chain',
  };
  return policy != null &&
      _hasExactKeys(policy, fields) &&
      policy.values.every((entry) => entry == 'none');
}

Map<String, Object?>? _objectMap(Object? value) {
  if (value is! Map || value.keys.any((key) => key is! String)) return null;
  return Map<String, Object?>.from(value);
}

bool _hasExactKeys(Map<String, Object?> value, Set<String> expected) =>
    value.length == expected.length && value.keys.every(expected.contains);

List<String>? _stringList(Object? value) {
  if (value is! List || value.any((item) => item is! String)) return null;
  final result = value.cast<String>();
  return result.toSet().length == result.length ? result : null;
}

bool _sameStringSet(List<String> left, Set<String> right) =>
    left.length == right.length && left.toSet().containsAll(right);

bool _validAuditId(String value) =>
    RegExp(r'^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$').hasMatch(value);

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

String exportJson(Map<String, Object?> value) =>
    const JsonEncoder.withIndent('  ').convert(value);
