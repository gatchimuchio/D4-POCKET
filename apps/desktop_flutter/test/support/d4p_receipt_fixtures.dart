const _optionalModuleIds = <String>[
  'shell.setup_doctor',
  'shell.history',
  'shell.evaluation_lab',
  'shell.host_capabilities',
  'shell.notifications',
  'shell.observability',
  'shell.trace_inspector',
  'shell.host_operations',
];

const _coreModuleIds = <String>[
  'core.security_broker',
  'core.cryptography',
  'core.audit_finality',
  'core.credentials',
  'core.os_controls',
  'core.permission_enforcement',
  'core.approval_enforcement',
  'core.content_exposure',
];

const _mandatoryModuleIds = <String>[
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
];

Map<String, Object?> composeBrokerResponse(
  Map<String, Object?> payload, {
  String auditId = 'fixture-compose-audit',
}) {
  const operation = 'GUI Shell構成';
  final manifest = payload['compose_manifest'] is Map
      ? Map<String, Object?>.from(payload['compose_manifest']! as Map)
      : payload;
  return _acceptedBrokerResponse(
    operation,
    auditId,
    {
      'version': 1,
      'operation': operation,
      'status': 'accepted',
      'compose_manifest': Map<String, Object?>.from(manifest),
      'build_status': 'not_started',
      'app_identity_status': 'not_generated',
      'permission_generated': false,
      'authority_strip': true,
      'inheritance_policy': const {
        'authority': 'none',
        'permission': 'none',
        'approval': 'none',
        'credential': 'none',
        'audit_chain': 'none',
      },
      'evidence_source': 'INTERNAL_STATE',
      'audit_id': auditId,
    },
  );
}

Map<String, Object?> exportBrokerResponse(
  Map<String, Object?> payload, {
  String auditId = 'fixture-export-audit',
  String path = r'C:\D4Pocket\Exports',
}) {
  const operation = 'GUI Shell書出し';
  final composeManifest = Map<String, Object?>.from(
    payload['compose_manifest']! as Map,
  );
  final exportId = payload['export_id']! as String;
  final targetPlatform = payload['target_platform']! as String;
  final distributionChannel = payload['distribution_channel']! as String;
  final moduleSelection = payload['module_selection'];
  final explicitSelection = moduleSelection is Map;
  final requestedSet = explicitSelection
      ? ((moduleSelection['optional_module_ids']! as List).cast<String>())
          .toSet()
      : _optionalModuleIds.toSet();
  final includedOptional = <String>{...requestedSet};
  if (includedOptional.contains('shell.trace_inspector')) {
    includedOptional.add('shell.observability');
  }
  final requested =
      _optionalModuleIds.where(requestedSet.contains).toList(growable: false);
  final included = [
    ..._coreModuleIds,
    ..._mandatoryModuleIds,
    ..._optionalModuleIds.where(includedOptional.contains),
  ];
  final excluded =
      _optionalModuleIds.where((id) => !includedOptional.contains(id)).toList();
  const appSuffix = '11111111111111111111111111111111';
  const storeSuffix = '22222222222222222222222222222222';
  const appId = 'd4-pocket-app-$appSuffix';
  const fileName = '$appId.json';
  final fullPath = '$path\\$fileName';
  return _acceptedBrokerResponse(
    operation,
    auditId,
    {
      'version': 1,
      'operation': operation,
      'status': 'accepted',
      'export_id': exportId,
      'export_manifest': {
        'app_identity': {
          'app_id': appId,
          'display_name': composeManifest['display_name'],
          'target_platform': targetPlatform,
        },
        'audit_store': {
          'store_id': 'audit-store-$storeSuffix',
          'chain_status': 'new',
          'inherited': false,
        },
        'settings': composeManifest['settings'],
        'runtime_manifest': {'runtime_ids': composeManifest['runtime_ids']},
        'adapter_configuration': {
          'agent_ids': composeManifest['agent_ids'],
          'tool_ids': composeManifest['tool_ids'],
          'mcp_connection_ids': composeManifest['mcp_connection_ids'],
        },
        'capability_requirements': composeManifest['capability_requirements'],
        'distribution_metadata': {
          'target_platform': targetPlatform,
          'artifact_status': 'not_built',
          'installer_status': 'not_started',
          'signed': false,
          'channel': distributionChannel,
        },
        'inheritance_policy': const {
          'authority': 'none',
          'permission': 'none',
          'approval': 'none',
          'credential': 'none',
          'audit_chain': 'none',
        },
        'module_plan': {
          'catalog_version': 1,
          'selection_mode':
              explicitSelection ? 'explicit_optional' : 'all_optional',
          'requested_optional_module_ids': requested,
          'unprunable_core_ids': _coreModuleIds,
          'mandatory_module_ids': _mandatoryModuleIds,
          'included_module_ids': included,
          'excluded_optional_module_ids': excluded,
          'binary_pruning_status': 'not_applied',
        },
      },
      'manifest_file_status': 'written',
      'manifest_file': {
        'file_name': fileName,
        'path': fullPath,
        'sha256': 'sha256:${List<String>.filled(64, 'a').join()}',
        'byte_length': 512,
        'temporary_file_status': 'removed',
        'temporary_file_name': '.$appSuffix.manifest.tmp',
        'recovery_action': 'none',
      },
      'build_status': 'not_started',
      'artifact_status': 'not_built',
      'credential_inherited': false,
      'permission_inherited': false,
      'approval_inherited': false,
      'audit_chain_inherited': false,
      'authority_strip': true,
      'evidence_source': 'INTERNAL_STATE',
      'audit_id': auditId,
    },
  );
}

Map<String, Object?> _acceptedBrokerResponse(
  String operation,
  String auditId,
  Map<String, Object?> body,
) =>
    {
      'request_id': 'fixture-request',
      'operation': operation,
      'status': 'accepted',
      'evidence_source': 'INTERNAL_STATE',
      'audit_event_id': auditId,
      'error': null,
      'body': body,
      'shutdown_requested': false,
    };
