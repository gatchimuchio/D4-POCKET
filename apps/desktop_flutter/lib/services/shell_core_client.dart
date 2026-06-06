import 'dart:convert';
import 'dart:io';

import '../models/generated_contracts.dart';
import 'broker_client.dart';

class ShellCoreClient {
  const ShellCoreClient._(this.snapshot, this.mode);

  final ShellSnapshot snapshot;
  final String mode;

  static Future<ShellCoreClient> product({
    BrokerTransport? transport,
  }) async {
    try {
      final broker = transport ?? await BrokerClient.connect();
      final healthResponse = await broker.request('health');
      final health = _acceptedResponseBodyMap(
        healthResponse,
        'health',
        bodyKey: 'health',
      );

      final normalizeResponse = await broker.request(
        'normalize_payload',
        payload: {'client_payload': 'desktop_flutter_authority_probe'},
      );
      _requireAccepted(normalizeResponse, 'normalize_payload');

      final approval = _brokerProjectionProbeApproval();
      final projectionResponse = await broker.request(
        'content_projection',
        payload: approval,
      );
      final projection = _acceptedResponseBodyMap(
        projectionResponse,
        'content_projection',
      );

      final protectedEditResponse = await broker.request(
        'approval_edit',
        payload: {
          'approval': approval,
          'field': 'payload_hash',
          'value':
              'sha256:0000000000000000000000000000000000000000000000000000000000000000',
        },
      );
      final protectedEdit = _acceptedResponseBodyMap(
        protectedEditResponse,
        'approval_edit',
      );

      final commandResponse = await broker.request(
        'command_envelope',
        payload: _brokerCommandProbePayload(),
      );
      if (commandResponse['status'] != 'suspended') {
        throw BrokerClientException(
          'command envelope was not suspended by broker: '
          '${commandResponse['status']}',
        );
      }

      return ShellCoreClient._(
        _brokerSnapshot(
          healthResponse: healthResponse,
          health: health,
          normalizeResponse: normalizeResponse,
          projectionResponse: projectionResponse,
          projection: projection,
          protectedEditResponse: protectedEditResponse,
          protectedEdit: protectedEdit,
          commandResponse: commandResponse,
        ),
        'broker',
      );
    } on Object catch (error) {
      return ShellCoreClient._(
        _brokerUnavailableSnapshot(error.toString()),
        'broker_unavailable',
      );
    }
  }

  factory ShellCoreClient.local({String? snapshotPath}) {
    final paths = snapshotPath == null
        ? _candidateSnapshotPaths()
        : <String>[snapshotPath];
    for (final resolvedPath in paths) {
      final file = File(resolvedPath);
      if (!file.existsSync()) {
        continue;
      }
      try {
        final json =
            jsonDecode(file.readAsStringSync()) as Map<String, Object?>;
        final snapshot = ShellSnapshot.fromJson(json).copyWith(
          snapshotSource: json['snapshot_source'] as String? ?? 'local',
          snapshotPath: resolvedPath,
          snapshotGeneratedAt: json['snapshot_generated_at'] as String? ??
              json['generated_at'] as String? ??
              json['snapshot_freshness'] as String? ??
              file.lastModifiedSync().toIso8601String(),
          snapshotFreshness: json['snapshot_freshness'] as String? ??
              file.lastModifiedSync().toIso8601String(),
        );
        return ShellCoreClient._(snapshot, 'local');
      } on Object {
        return ShellCoreClient._(
          _snapshotWithLocalIssue(
            problemId: 'local-snapshot-parse-failed',
            item: 'local snapshot parse failed',
            classification: 'required_for_v1',
            severity: 'warning',
            message: 'Local owner-operation snapshot could not be parsed.',
            target: resolvedPath,
            requiredAction:
                'Refresh the development diagnostic snapshot before using local inspection mode.',
            source: 'fallback',
            freshness: 'parse failed',
          ),
          'local',
        );
      }
    }
    return ShellCoreClient._(
      _snapshotWithLocalIssue(
        problemId: 'local-snapshot-missing',
        item: 'local snapshot missing',
        classification: 'known_limitation',
        severity: 'info',
        message: 'Local owner-operation snapshot file is missing.',
        target: paths.first,
        requiredAction:
            'Create a development diagnostic snapshot before using local inspection mode.',
        source: 'fallback',
        freshness: 'missing',
      ),
      'local',
    );
  }

  factory ShellCoreClient.mock() {
    return const ShellCoreClient._(
      _mockSnapshot,
      'mock',
    );
  }

  ShellSnapshot getSnapshot() => snapshot;
}

Map<String, Object?> _acceptedResponseBodyMap(
  Map<String, Object?> response,
  String operation, {
  String bodyKey = 'body',
}) {
  _requireAccepted(response, operation);
  final body = response[bodyKey];
  if (body is! Map) {
    throw BrokerClientException('$operation response missing $bodyKey object');
  }
  return Map<String, Object?>.from(body);
}

void _requireAccepted(Map<String, Object?> response, String operation) {
  if (response['status'] != 'accepted') {
    final error = response['error'];
    final message = error is Map
        ? error['message']?.toString() ?? response.toString()
        : response.toString();
    throw BrokerClientException('$operation broker request rejected: $message');
  }
}

ShellSnapshot _brokerSnapshot({
  required Map<String, Object?> healthResponse,
  required Map<String, Object?> health,
  required Map<String, Object?> normalizeResponse,
  required Map<String, Object?> projectionResponse,
  required Map<String, Object?> projection,
  required Map<String, Object?> protectedEditResponse,
  required Map<String, Object?> protectedEdit,
  required Map<String, Object?> commandResponse,
}) {
  final now = DateTime.now().toUtc().toIso8601String();
  final healthStatus = health['status']?.toString() ?? 'unknown';
  final cutoverStatus =
      health['authority_cutover_status']?.toString() ?? 'unknown';
  final persistenceReady = health['persistence_ready'] == true;
  final commandDispatchEnabled = health['command_dispatch_enabled'] == true &&
      _boolInBody(commandResponse, 'dispatch_enabled');
  final protectedEditRejected = protectedEdit['ok'] == false;
  final brokerReady = healthStatus == 'ready' && persistenceReady;
  final runtimeStatus =
      brokerReady && cutoverStatus == 'active' ? 'ready' : 'suspend';
  final problems = <Map<String, Object?>>[];
  if (!brokerReady) {
    problems.add(_releaseProblemJson(
      problemId: 'broker-persistence-not-ready',
      item: 'broker durable store not ready',
      severity: 'error',
      category: 'broker_ipc',
      message:
          'Rust broker did not report durable audit/replay/session readiness.',
      target: 'rust_security_broker',
      recoveryId: 'recover-broker-persistence',
      requiredAction:
          'Start the Rust broker with durable store access and reconnect the product UI.',
      blocksOwnerUse: true,
    ));
  }
  if (cutoverStatus != 'active') {
    problems.add(_releaseProblemJson(
      problemId: 'broker-authority-cutover-not-active',
      item: 'broker authority cutover not active',
      severity: 'warning',
      category: 'authority_cutover',
      message:
          'Flutter is broker-mediated, but the broker has not claimed active authority cutover.',
      target: 'authority_cutover_status',
      recoveryId: 'recover-authority-cutover',
      requiredAction:
          'Complete broker authority cutover and only then allow active authority operations.',
    ));
  }
  if (!commandDispatchEnabled) {
    problems.add(_releaseProblemJson(
      problemId: 'broker-command-dispatch-suspended',
      item: 'broker command dispatch suspended',
      severity: 'warning',
      category: 'command_envelope',
      message:
          'External command dispatch remains suspended by the Rust broker.',
      target: 'command_dispatch_enabled',
      recoveryId: 'recover-command-dispatch',
      requiredAction:
          'Finish command-envelope authority mapping before enabling external dispatch.',
    ));
  }
  if (!protectedEditRejected) {
    problems.add(_releaseProblemJson(
      problemId: 'approval-protected-field-edit-not-rejected',
      item: 'approval protected field edit not rejected',
      severity: 'error',
      category: 'approval',
      message:
          'Broker did not reject a protected approval payload_hash edit probe.',
      target: 'approval_edit',
      recoveryId: 'recover-approval-protected-edit',
      requiredAction:
          'Keep authority actions suspended until protected field enforcement is restored.',
      blocksOwnerUse: true,
    ));
  }

  final projectedContent = projection.containsKey('redacted_payload')
      ? Map<String, Object?>.from(projection['redacted_payload'] as Map? ?? {})
      : projection;
  final projectionRedacted = !jsonEncode(projectedContent).contains('hidden');
  if (!projectionRedacted) {
    problems.add(_releaseProblemJson(
      problemId: 'broker-content-projection-leaked-full-payload',
      item: 'broker content projection leaked full payload',
      severity: 'error',
      category: 'content_projection',
      message:
          'Broker content projection returned data from the hidden full payload probe.',
      target: 'content_projection',
      recoveryId: 'recover-content-projection',
      requiredAction:
          'Keep authority actions suspended and restore content visibility enforcement.',
      blocksOwnerUse: true,
    ));
  }
  final auditEvents = [
    _auditJson(healthResponse, 'broker.health', 'accepted'),
    _auditJson(normalizeResponse, 'broker.normalize_payload', 'accepted'),
    _auditJson(projectionResponse, 'broker.content_projection', 'accepted'),
    _auditJson(
      protectedEditResponse,
      'broker.approval_protected_edit',
      protectedEditRejected ? 'rejected' : 'accepted',
    ),
    _auditJson(commandResponse, 'broker.command_envelope', 'suspended'),
  ];
  final recoveryPlaybook =
      problems.map(_problemToRecoveryPlaybookJson).toList(growable: false);

  return ShellSnapshot.fromJson({
    'phase_status': {
      'phase_a_status': 'complete',
      'phase_b_status': 'complete',
      'phase_c_status': 'broker-mediated',
      'phase_d_status': 'pending',
      'phase_e_status': 'pending',
      'phase_f_status': 'later',
      'completed_product_release_claimed': false,
    },
    'operation_status': {
      'runtime_status': runtimeStatus,
      'invariant_status': protectedEditRejected ? 'ok' : 'blocked',
      'trust_status': brokerReady ? 'restricted' : 'blocked',
      'pending_approvals_count': 0,
      'audit_chain_status':
          persistenceReady ? 'durable_file_store' : 'not_ready',
      'problems_count': problems.length,
      'release_state': 'not claimed',
    },
    'runtimes': [
      {
        'runtime_id': 'gui_shell_rust_broker',
        'name': 'Rust Security Broker',
        'status': runtimeStatus,
        'adapter_id': 'restricted_ipc',
        'diagnostic_summary':
            'status=$healthStatus persistence_ready=$persistenceReady cutover=$cutoverStatus',
      }
    ],
    'agent_sessions': [],
    'permissions': [],
    'pending_approvals': [],
    'audit_events': auditEvents,
    'recovery_actions': [
      for (final problem in problems)
        {
          'recovery_id': problem['recovery_id'],
          'severity': problem['severity'],
          'message': problem['required_action'],
          'safe_to_retry': true,
        }
    ],
    'invariant_flags': {
      'flutter_imported_by_shell_core': false,
      'blue_tanuki_imported_by_shell_core': false,
      'adapter_metadata_can_escalate_authority': false,
      'memory_cache_previous_state_can_grant_authority': false,
      'full_payload_projected_without_full_visibility': false,
      'installer_setup_state_can_grant_authority': false,
      'mobile_device_state_can_grant_authority': false,
      'flutter_product_uses_local_snapshot_for_authority': false,
      'authority_bridge_uses_ffi': false,
      'approval_protected_field_edit_allowed': !protectedEditRejected,
    },
    'setup_doctor_status': brokerReady ? 'warning' : 'fail',
    'installer_grants_authority': false,
    'installer_silently_approves_permissions': false,
    'setup_doctor_checks': [
      {
        'check_id': 'broker.ipc',
        'status': brokerReady ? 'pass' : 'fail',
        'message':
            'Flutter product path used authenticated broker IPC for authority status.',
        'recovery_instruction':
            brokerReady ? null : 'Restart the Rust broker and reconnect.',
        'grants_authority': false,
      },
      {
        'check_id': 'broker.persistence',
        'status': persistenceReady ? 'pass' : 'fail',
        'message':
            'persistence_ready=${persistenceReady.toString()} audit=${health['audit_persistence']} replay=${health['replay_persistence']} session=${health['session_persistence']}',
        'recovery_instruction':
            persistenceReady ? null : 'Repair durable broker store access.',
        'grants_authority': false,
      },
      {
        'check_id': 'broker.protected_field_edit',
        'status': protectedEditRejected ? 'pass' : 'fail',
        'message': protectedEditRejected
            ? 'Protected payload_hash edit rejected.'
            : 'Protected edit probe was not rejected.',
        'recovery_instruction': protectedEditRejected
            ? null
            : 'Keep authority actions suspended and restore approval edit enforcement.',
        'grants_authority': false,
      },
      {
        'check_id': 'broker.command_dispatch',
        'status': commandDispatchEnabled ? 'pass' : 'warning',
        'message':
            'command_dispatch_enabled=${commandDispatchEnabled.toString()}',
        'recovery_instruction': commandDispatchEnabled
            ? null
            : 'Complete command-envelope dispatch authority mapping.',
        'grants_authority': false,
      },
    ],
    'trust_records': [
      {
        'scope': 'broker_session',
        'state': brokerReady ? 'restricted' : 'blocked',
        'source': 'authenticated_loopback_tcp',
        'expires_at': null,
        'blocked_operations':
            commandDispatchEnabled ? [] : ['external_command_dispatch'],
      },
      {
        'scope': 'authority_cutover',
        'state': cutoverStatus,
        'source': 'rust_security_broker',
        'expires_at': null,
        'blocked_operations':
            cutoverStatus == 'active' ? [] : ['authority_actions'],
      },
    ],
    'authority_map': [],
    'adapter_catalog': [],
    'permission_diffs': [],
    'problems': problems,
    'evidence': [
      {
        'evidence_id': 'broker-ipc-product-path',
        'kind': 'LIVE_RUNTIME',
        'status': brokerReady ? 'pass' : 'fail',
        'path': 'broker://127.0.0.1/health',
        'hash': '',
        'exportable': false,
      },
      {
        'evidence_id': 'broker-protected-edit-probe',
        'kind': 'LIVE_RUNTIME',
        'status': protectedEditRejected ? 'pass' : 'fail',
        'path': 'broker://127.0.0.1/approval_edit',
        'hash': '',
        'exportable': false,
      },
      {
        'evidence_id': 'broker-redacted-projection-probe',
        'kind': 'LIVE_RUNTIME',
        'status': projectionRedacted ? 'pass' : 'fail',
        'path': 'broker://127.0.0.1/content_projection',
        'hash': '',
        'exportable': false,
      },
      {
        'evidence_id': 'broker-command-dispatch-suspended',
        'kind': 'LIVE_RUNTIME',
        'status': commandDispatchEnabled ? 'fail' : 'pass',
        'path': 'broker://127.0.0.1/command_envelope',
        'hash': '',
        'exportable': false,
      },
    ],
    'settings': [
      {
        'key': 'authority.product_path',
        'group': 'broker',
        'default': 'broker_ipc',
        'current': 'broker_ipc',
        'effective': 'broker_ipc',
        'source': 'rust_security_broker',
        'modified': false,
        'dangerous': false,
        'authority_related': true,
      },
      {
        'key': 'authority.local_snapshot',
        'group': 'broker',
        'default': 'diagnostic_only',
        'current': 'diagnostic_only',
        'effective': 'diagnostic_only',
        'source': 'flutter_product',
        'modified': false,
        'dangerous': false,
        'authority_related': true,
      },
    ],
    'audit_chain_status': persistenceReady ? 'durable_file_store' : 'not_ready',
    'network_exposure': '127.0.0.1 restricted broker IPC',
    'release_blocker_count':
        problems.where((problem) => problem['blocks_release'] == true).length,
    'evidence_summary': {
      'schema_check': 'not evaluated by product UI',
      'conformance_check_count': 0,
      'release_smoke': 'not evaluated by product UI',
      'release_gate_check': 'not evaluated by product UI',
      'evidence_bundle': 'not evaluated by product UI',
      'validate_all': 'not evaluated by product UI',
      'strict_windows_release': 'pending',
      'missing_measured_windows_evidence': true,
      'missing_setup_doctor_evidence': false,
      'owner_go': 'missing',
    },
    'recovery_playbook': recoveryPlaybook,
    'snapshot_source': 'broker',
    'snapshot_path': 'broker://127.0.0.1/health',
    'snapshot_generated_at': now,
    'snapshot_freshness': now,
  });
}

ShellSnapshot _brokerUnavailableSnapshot(String reason) {
  final now = DateTime.now().toUtc().toIso8601String();
  final problem = _releaseProblemJson(
    problemId: 'broker-product-path-unavailable',
    item: 'broker product path unavailable',
    severity: 'error',
    category: 'broker_ipc',
    message:
        'Flutter product authority path could not establish accepted broker IPC.',
    target: 'rust_security_broker',
    recoveryId: 'recover-broker-product-path',
    requiredAction:
        'Restart or repair the Rust broker; do not fall back to local JSON for authority.',
    blocksOwnerUse: true,
  );
  return ShellSnapshot.fromJson({
    'phase_status': {
      'phase_a_status': 'complete',
      'phase_b_status': 'complete',
      'phase_c_status': 'blocked',
      'phase_d_status': 'pending',
      'phase_e_status': 'pending',
      'phase_f_status': 'later',
      'completed_product_release_claimed': false,
    },
    'operation_status': {
      'runtime_status': 'suspend',
      'invariant_status': 'blocked',
      'trust_status': 'blocked',
      'pending_approvals_count': 0,
      'audit_chain_status': 'not_ready',
      'problems_count': 1,
      'release_state': 'not claimed',
    },
    'runtimes': [
      {
        'runtime_id': 'gui_shell_rust_broker',
        'name': 'Rust Security Broker',
        'status': 'unavailable',
        'adapter_id': 'restricted_ipc',
        'diagnostic_summary': reason,
      }
    ],
    'agent_sessions': [],
    'permissions': [],
    'pending_approvals': [],
    'audit_events': [
      {
        'event_id': 'broker-product-path-unavailable',
        'action': 'broker.connect',
        'result': 'rejected',
        'payload_hash':
            'sha256:0000000000000000000000000000000000000000000000000000000000000000',
      }
    ],
    'recovery_actions': [
      {
        'recovery_id': 'recover-broker-product-path',
        'severity': 'error',
        'message':
            'Authority actions are disabled until broker IPC is restored.',
        'safe_to_retry': true,
      }
    ],
    'invariant_flags': {
      'flutter_imported_by_shell_core': false,
      'blue_tanuki_imported_by_shell_core': false,
      'adapter_metadata_can_escalate_authority': false,
      'memory_cache_previous_state_can_grant_authority': false,
      'full_payload_projected_without_full_visibility': false,
      'installer_setup_state_can_grant_authority': false,
      'mobile_device_state_can_grant_authority': false,
      'flutter_product_uses_local_snapshot_for_authority': false,
      'authority_bridge_uses_ffi': false,
      'broker_unavailable_fail_closed': true,
    },
    'setup_doctor_status': 'fail',
    'installer_grants_authority': false,
    'installer_silently_approves_permissions': false,
    'setup_doctor_checks': [
      {
        'check_id': 'broker.ipc',
        'status': 'fail',
        'message': 'Broker IPC is unavailable or rejected authentication.',
        'recovery_instruction':
            'Restart the Rust broker and reconnect; local JSON is diagnostic-only.',
        'grants_authority': false,
      },
      {
        'check_id': 'broker.fail_closed',
        'status': 'pass',
        'message':
            'Flutter product path returned SUSPEND instead of local snapshot authority.',
        'recovery_instruction': null,
        'grants_authority': false,
      },
    ],
    'trust_records': [
      {
        'scope': 'broker_session',
        'state': 'blocked',
        'source': 'restricted_ipc',
        'expires_at': null,
        'blocked_operations': [
          'authority_actions',
          'external_command_dispatch'
        ],
      }
    ],
    'authority_map': [],
    'adapter_catalog': [],
    'permission_diffs': [],
    'problems': [problem],
    'evidence': [
      {
        'evidence_id': 'broker-unavailable-fail-closed',
        'kind': 'LIVE_RUNTIME',
        'status': 'pass',
        'path': 'broker://127.0.0.1/connect',
        'hash': '',
        'exportable': false,
      }
    ],
    'settings': [],
    'audit_chain_status': 'not_ready',
    'network_exposure': '127.0.0.1 restricted broker IPC',
    'release_blocker_count': 1,
    'evidence_summary': {
      'schema_check': 'not evaluated by product UI',
      'conformance_check_count': 0,
      'release_smoke': 'not evaluated by product UI',
      'release_gate_check': 'not evaluated by product UI',
      'evidence_bundle': 'not evaluated by product UI',
      'validate_all': 'not evaluated by product UI',
      'strict_windows_release': 'pending',
      'missing_measured_windows_evidence': true,
      'missing_setup_doctor_evidence': false,
      'owner_go': 'missing',
    },
    'recovery_playbook': [_problemToRecoveryPlaybookJson(problem)],
    'snapshot_source': 'broker_unavailable',
    'snapshot_path': 'broker://127.0.0.1/connect',
    'snapshot_generated_at': now,
    'snapshot_freshness': 'unavailable',
  });
}

Map<String, Object?> _brokerProjectionProbeApproval() {
  return {
    'approval_id': 'broker-projected-approval',
    'runtime_id': 'gui_shell_rust_broker',
    'permission_id': 'permission.broker.command_envelope',
    'status': 'pending',
    'content_visibility': 'redacted',
    'payload_hash':
        'sha256:2222222222222222222222222222222222222222222222222222222222222222',
    'summary': 'Broker-mediated approval projection probe',
    'redacted_payload': {'path': 'notes/today.md', 'content': '[redacted]'},
    "full_payload": {'path': 'notes/today.md', 'content': 'hidden'},
    'editable_fields': ['path', 'payload_hash'],
    'authority_fields': ['permission_id'],
  };
}

Map<String, Object?> _brokerCommandProbePayload() {
  return {
    'action': {
      'operation': 'command_envelope.dispatch',
      'runtime_id': 'gui_shell_rust_broker',
      'capability_id': 'command_envelope.dispatch',
      'permission_id': 'permission.broker.command_envelope',
      'approval_id': 'broker-projected-approval',
      'target_scope': 'broker_command',
      'recovery_action': {'recovery_id': 'recover-command-dispatch'},
      'adapter_metadata': {'client': 'desktop_flutter'},
    },
  };
}

bool _boolInBody(Map<String, Object?> response, String key) {
  final body = response['body'];
  if (body is! Map) {
    return false;
  }
  return body[key] == true;
}

Map<String, Object?> _auditJson(
  Map<String, Object?> response,
  String action,
  String result,
) {
  return {
    'event_id': response['audit_event_id']?.toString() ?? action,
    'action': action,
    'result': result,
    'payload_hash':
        'sha256:1111111111111111111111111111111111111111111111111111111111111111',
  };
}

Map<String, Object?> _releaseProblemJson({
  required String problemId,
  required String item,
  required String severity,
  required String category,
  required String message,
  required String target,
  required String recoveryId,
  required String requiredAction,
  bool blocksOwnerUse = false,
}) {
  return {
    'problem_id': problemId,
    'severity': severity,
    'category': category,
    'message': message,
    'target': target,
    'recovery_id': recoveryId,
    'item': item,
    'classification': 'release_blocker',
    'reason': message,
    'required_action': requiredAction,
    'blocks_release': true,
    'safe_to_ignore_for_phase_b': !blocksOwnerUse,
    'blocks_owner_use': blocksOwnerUse,
    'blocks_completed_product_release': true,
  };
}

Map<String, Object?> _problemToRecoveryPlaybookJson(
  Map<String, Object?> problem,
) {
  return {
    'recovery_id': problem['recovery_id']?.toString() ?? '',
    'item': problem['item']?.toString() ?? '',
    'severity': problem['severity']?.toString() ?? 'warning',
    'classification':
        problem['classification']?.toString() ?? 'release_blocker',
    'safe_to_ignore_for_phase_b': problem['safe_to_ignore_for_phase_b'] == true,
    'required_action': problem['required_action']?.toString() ?? '',
    'blocks_completed_product_release':
        problem['blocks_completed_product_release'] == true,
    'blocks_owner_use': problem['blocks_owner_use'] == true,
    'command': '',
    'path': problem['target']?.toString() ?? '',
  };
}

List<String> _candidateSnapshotPaths() {
  final explicit = Platform.environment['GUI_SHELL_SNAPSHOT_JSON'];
  if (explicit != null && explicit.isNotEmpty) {
    return [explicit];
  }
  final paths = <String>[];
  if (Platform.isWindows) {
    final localAppData = Platform.environment['LOCALAPPDATA'];
    if (localAppData != null && localAppData.isNotEmpty) {
      paths.add('$localAppData\\GUI-Shell\\shell_snapshot.json');
    }
  }
  paths.add('.gui_shell/shell_snapshot.json');
  paths.add('.gui-shell/shell_snapshot.json');
  return paths;
}

ShellSnapshot _snapshotWithLocalIssue({
  required String problemId,
  required String item,
  required String classification,
  required String severity,
  required String message,
  required String target,
  required String requiredAction,
  required String source,
  required String freshness,
}) {
  final problem = ProblemRecord(
    problemId: problemId,
    severity: severity,
    category: 'local_snapshot',
    message: message,
    target: target,
    recoveryId: 'recover-local-snapshot',
    item: item,
    classification: classification,
    reason: message,
    requiredAction: requiredAction,
    blocksRelease: false,
  );
  final problems = [problem, ..._localFallbackSnapshot.problems];
  return _localFallbackSnapshot.copyWith(
    problems: problems,
    operationStatus: _localFallbackSnapshot.operationStatus.copyWith(
      problemsCount: problems.length,
      releaseState: 'not claimed',
    ),
    snapshotSource: source,
    snapshotPath: target,
    snapshotGeneratedAt: freshness,
    snapshotFreshness: freshness,
  );
}

const _mockSnapshot = ShellSnapshot(
  phaseStatus: PhaseStatusRecord(
    phaseAStatus: 'complete',
    phaseBStatus: 'complete',
    phaseCStatus: 'next',
    phaseDStatus: 'later',
    phaseEStatus: 'later',
    phaseFStatus: 'later',
    completedProductReleaseClaimed: false,
  ),
  operationStatus: OperationStatusRecord(
    runtimeStatus: 'ready',
    invariantStatus: 'ok',
    trustStatus: 'restricted',
    pendingApprovalsCount: 1,
    auditChainStatus: 'verified',
    problemsCount: 6,
    releaseState: 'not claimed',
  ),
  runtimes: [
    RuntimeRecord(
      runtimeId: 'blue_tanuki',
      name: 'BLUE-TANUKI',
      status: 'ready',
      adapterId: 'blue_tanuki_reference',
      diagnosticSummary: 'mock adapter contract available',
    ),
  ],
  agentSessions: [
    AgentSessionRecord(
      sessionId: 'agent-session-1',
      workspace: '/workspace/project',
      task: 'Update documentation',
      changedFiles: ['README.md', 'docs/STRATEGY.md'],
      toolCalls: ['shell.command', 'git.diff'],
      shellCommands: [
        'python3 tooling/conformance_tests/run_conformance_skeleton.py'
      ],
      testStatus: 'conformance passed',
      diffSummary: '2 files changed',
      pendingApprovalCount: 1,
      rollbackCandidate: 'rollback-1',
      auditEventId: 'audit-1',
    ),
  ],
  permissions: [
    PermissionRecord(
      permissionId: 'permission.fs.write.workspace',
      capabilityId: 'filesystem.write',
      decision: 'ask',
      source: 'policy',
    ),
  ],
  pendingApprovals: [
    ApprovalRecord(
      approvalId: 'approval-1',
      operation: 'filesystem.write',
      status: 'pending',
      contentVisibility: 'redacted',
      projectedContent: {'path': 'notes/today.md', 'content': '[redacted]'},
      editableFields: ['path'],
      protectedFields: [
        'runtime_id',
        'permission_id',
        'payload_hash',
        'authority_context'
      ],
    ),
  ],
  auditEvents: [
    AuditRecord(
      eventId: 'audit-1',
      action: 'approval.requested',
      result: 'success',
      payloadHash:
          'sha256:2222222222222222222222222222222222222222222222222222222222222222',
    ),
  ],
  recoveryActions: [
    RecoveryRecord(
      recoveryId: 'recover-1',
      severity: 'warning',
      message: 'Permission is required before this action can run.',
      safeToRetry: true,
    ),
  ],
  invariantFlags: {
    'flutter_imported_by_shell_core': false,
    'blue_tanuki_imported_by_shell_core': false,
    'adapter_metadata_can_escalate_authority': false,
    'memory_cache_previous_state_can_grant_authority': false,
    'full_payload_projected_without_full_visibility': false,
    'installer_setup_state_can_grant_authority': false,
    'mobile_device_state_can_grant_authority': false,
  },
  setupDoctorStatus: 'warning',
  installerGrantsAuthority: false,
  installerSilentlyApprovesPermissions: false,
  setupDoctorChecks: [
    SetupDoctorCheckRecord(
      checkId: 'host.os',
      status: 'pass',
      message: 'Host OS detected',
      recoveryInstruction: null,
      grantsAuthority: false,
    ),
    SetupDoctorCheckRecord(
      checkId: 'filesystem.permission',
      status: 'pass',
      message: 'Audit storage path writable',
      recoveryInstruction: null,
      grantsAuthority: false,
    ),
    SetupDoctorCheckRecord(
      checkId: 'network.public_bind',
      status: 'warning',
      message: 'Public bind requires explicit operator review',
      recoveryInstruction:
          'Keep runtimes on localhost unless permission and approval explicitly allow public bind.',
      grantsAuthority: false,
    ),
  ],
  trustRecords: [
    TrustRecord(
      scope: 'workspace_trust',
      state: 'restricted',
      source: 'local policy',
      expiresAt: null,
      blockedOperations: ['process.spawn', 'network.public_bind'],
    ),
    TrustRecord(
      scope: 'runtime_trust',
      state: 'trusted',
      source: 'signed manifest',
      expiresAt: null,
      blockedOperations: [],
    ),
    TrustRecord(
      scope: 'adapter_trust',
      state: 'inherited',
      source: 'runtime_trust',
      expiresAt: null,
      blockedOperations: [],
    ),
    TrustRecord(
      scope: 'installer_trust',
      state: 'unknown',
      source: 'installed-path evidence missing',
      expiresAt: null,
      blockedOperations: ['release_ready_claim'],
    ),
  ],
  authorityMap: [
    AuthorityMapRecord(
      runtimeId: 'blue_tanuki',
      capabilityId: 'filesystem.write',
      permissionId: 'permission.fs.write.workspace',
      approvalId: 'approval-1',
      auditEventId: 'audit-1',
      recoveryId: 'recover-1',
      dangerous: false,
      warning: 'approval pending',
    ),
  ],
  adapterCatalog: [
    AdapterCatalogRecord(
      adapterId: 'blue_tanuki_reference',
      runtimeId: 'blue_tanuki',
      publisher: 'GUI-Shell reference',
      version: '0.1.0',
      signature: 'development',
      hash: 'sha256:pending',
      requestedCapabilities: ['filesystem.write'],
      grantedCapabilities: [],
      deniedCapabilities: ['network.public_bind'],
      trustStatus: 'inherited',
      lastVerified: 'development smoke',
      updateAvailable: false,
      knownRisks: ['reference adapter only'],
    ),
  ],
  permissionDiffs: [
    PermissionDiffRecord(
      subject: 'blue_tanuki_reference',
      added: ['filesystem.write'],
      removed: [],
      changed: ['content_visibility: full -> redacted'],
      dangerous: [],
    ),
  ],
  problems: [
    ProblemRecord(
      problemId: 'windows-installed-evidence-missing',
      severity: 'blocked',
      category: 'missing_evidence',
      message: 'Windows installed-path evidence is missing.',
      target: 'release_evidence/windows_installed_smoke.json',
      recoveryId: 'recover-windows-evidence',
      item: 'measured Windows installed-path first-run evidence missing',
      classification: 'release_blocker',
      reason: 'Measured installed-path first-run evidence is not recorded.',
      requiredAction:
          'Run hardened Windows installed smoke collection on native Windows.',
      blocksRelease: true,
    ),
    ProblemRecord(
      problemId: 'setup-doctor-installed-evidence-missing',
      severity: 'blocked',
      category: 'missing_evidence',
      message: 'Non-synthetic installed-path Setup Doctor evidence is missing.',
      target: 'release_evidence/windows_installed_smoke.json',
      recoveryId: 'recover-setup-doctor-evidence',
      item: 'non-synthetic installed-path Setup Doctor evidence missing',
      classification: 'release_blocker',
      reason: 'Setup Doctor has not been proven from the installed app path.',
      requiredAction:
          'Run Setup Doctor from the installed Windows app path and record required checks.',
      blocksRelease: true,
    ),
    ProblemRecord(
      problemId: 'owner-go-missing',
      severity: 'blocked',
      category: 'release_gate',
      message: 'Owner GO missing.',
      target: 'release checklist',
      recoveryId: 'recover-owner-go',
      item: 'owner GO missing',
      classification: 'release_blocker',
      reason: 'Completed product release requires explicit owner approval.',
      requiredAction: 'Record owner GO after release blockers are cleared.',
      blocksRelease: true,
    ),
    ProblemRecord(
      problemId: 'macos-unverified',
      severity: 'info',
      category: 'scope',
      message: 'macOS remains unverified.',
      target: 'desktop platform matrix',
      recoveryId: 'recover-macos-validation',
      item: 'macOS unverified',
      classification: 'known_limitation',
      reason: 'No macOS validation environment is available.',
      requiredAction: 'Validate on macOS before claiming macOS support.',
      blocksRelease: false,
    ),
    ProblemRecord(
      problemId: 'mobile-post-v1',
      severity: 'info',
      category: 'scope',
      message: 'Mobile full release is post-v1 scope.',
      target: 'mobile status',
      recoveryId: 'recover-mobile-scope',
      item: 'mobile post-v1 scope',
      classification: 'post_v1_scope',
      reason: 'v1.0 is Windows-first desktop unless owner changes scope.',
      requiredAction: 'Defer mobile release work.',
      blocksRelease: false,
    ),
    ProblemRecord(
      problemId: 'paid-qc-later',
      severity: 'info',
      category: 'scope',
      message: 'Paid/product QC is a later phase.',
      target: 'phase strategy',
      recoveryId: 'recover-paid-qc',
      item: 'paid/product QC later',
      classification: 'post_v1_scope',
      reason: 'Phase B is owner-use hardening, not paid/product QC.',
      requiredAction: 'Defer paid/product QC until Phase F.',
      blocksRelease: false,
    ),
  ],
  evidence: [
    EvidenceRecord(
      evidenceId: 'windows-installed-smoke',
      kind: 'installed-path',
      status: 'missing',
      path: 'release_evidence/windows_installed_smoke.json',
      hash: '',
      exportable: false,
    ),
    EvidenceRecord(
      evidenceId: 'development-validation',
      kind: 'validation',
      status: 'passed',
      path: 'tooling/validate_all.py',
      hash: '',
      exportable: true,
    ),
  ],
  settings: [
    SettingRecord(
      key: 'content_visibility.default',
      group: 'authority',
      defaultValue: 'redacted',
      currentValue: 'redacted',
      effectiveValue: 'redacted',
      source: 'Shell Core policy',
      modified: false,
      dangerous: false,
      authorityRelated: true,
    ),
    SettingRecord(
      key: 'network.public_bind',
      group: 'runtime',
      defaultValue: 'blocked',
      currentValue: 'blocked',
      effectiveValue: 'blocked',
      source: 'permission ledger',
      modified: false,
      dangerous: true,
      authorityRelated: true,
    ),
    SettingRecord(
      key: 'phase.owner_use',
      group: 'phase',
      defaultValue: 'complete',
      currentValue: 'complete',
      effectiveValue: 'complete',
      source: 'docs/PHASE_STRATEGY.md',
      modified: false,
      dangerous: false,
      authorityRelated: false,
    ),
    SettingRecord(
      key: 'release.state',
      group: 'release',
      defaultValue: 'not claimed',
      currentValue: 'not claimed',
      effectiveValue: 'not claimed',
      source: 'strict release gate',
      modified: false,
      dangerous: true,
      authorityRelated: true,
    ),
  ],
  auditChainStatus: 'verified',
  networkExposure: 'localhost only',
  releaseBlockerCount: 4,
  evidenceSummary: EvidenceSummaryRecord(
    schemaCheck: 'passed',
    conformanceCheckCount: 138,
    releaseSmoke: 'passed',
    releaseGateCheck: 'passed',
    evidenceBundle: 'passed',
    validateAll: 'passed',
    strictWindowsRelease: 'expected fail',
    missingMeasuredWindowsEvidence: true,
    missingSetupDoctorEvidence: true,
    missingAuditAnchorExternalTamperEvidence: true,
    ownerGo: 'missing',
  ),
  recoveryPlaybook: [
    RecoveryPlaybookRecord(
      item: 'measured Windows installed-path evidence missing',
      severity: 'release',
      classification: 'release_blocker',
      safeToIgnoreForPhaseB: true,
      requiredAction: 'Run hardened Windows installed smoke on native Windows.',
      blocksCompletedProductRelease: true,
    ),
    RecoveryPlaybookRecord(
      item: 'non-synthetic installed-path Setup Doctor evidence missing',
      severity: 'release',
      classification: 'release_blocker',
      safeToIgnoreForPhaseB: true,
      requiredAction:
          'Run installed-path Setup Doctor and record non-synthetic checks.',
      blocksCompletedProductRelease: true,
    ),
    RecoveryPlaybookRecord(
      item: 'owner GO missing',
      severity: 'release',
      classification: 'release_blocker',
      safeToIgnoreForPhaseB: true,
      requiredAction: 'Record owner GO after release blockers pass.',
      blocksCompletedProductRelease: true,
    ),
    RecoveryPlaybookRecord(
      item: 'macOS unverified',
      severity: 'scope',
      classification: 'known_limitation',
      safeToIgnoreForPhaseB: true,
      requiredAction: 'Validate on macOS before claiming macOS support.',
      blocksCompletedProductRelease: false,
    ),
    RecoveryPlaybookRecord(
      item: 'mobile full release',
      severity: 'scope',
      classification: 'post_v1_scope',
      safeToIgnoreForPhaseB: true,
      requiredAction: 'Defer mobile full release until post-v1.',
      blocksCompletedProductRelease: false,
    ),
    RecoveryPlaybookRecord(
      item: 'Phase B owner-use usability issue',
      severity: 'owner-use',
      classification: 'required_for_v1',
      safeToIgnoreForPhaseB: false,
      requiredAction:
          'Keep dashboard, status, problems, evidence, and recovery surfaces usable.',
      blocksCompletedProductRelease: false,
    ),
  ],
  snapshotSource: 'mock',
  snapshotPath: 'embedded mock',
  snapshotGeneratedAt: 'static',
  snapshotFreshness: 'static',
);

const _localFallbackSnapshot = ShellSnapshot(
  phaseStatus: PhaseStatusRecord(
    phaseAStatus: 'complete',
    phaseBStatus: 'complete',
    phaseCStatus: 'next',
    phaseDStatus: 'later',
    phaseEStatus: 'later',
    phaseFStatus: 'later',
    completedProductReleaseClaimed: false,
  ),
  operationStatus: OperationStatusRecord(
    runtimeStatus: 'diagnostic',
    invariantStatus: 'ok',
    trustStatus: 'unknown',
    pendingApprovalsCount: 1,
    auditChainStatus: 'unknown',
    problemsCount: 1,
    releaseState: 'not claimed',
  ),
  runtimes: [
    RuntimeRecord(
      runtimeId: 'local_shell_core',
      name: 'Local Shell Core',
      status: 'diagnostic',
      adapterId: 'local_setup_doctor',
      diagnosticSummary: 'local diagnostic snapshot fallback',
    ),
  ],
  agentSessions: [],
  permissions: [],
  pendingApprovals: [
    ApprovalRecord(
      approvalId: 'local-approval-redacted',
      operation: 'diagnostic.review',
      status: 'pending',
      contentVisibility: 'redacted',
      projectedContent: {'summary': '[redacted]'},
      editableFields: [],
      protectedFields: ['payload_hash', 'authority_context'],
    ),
  ],
  auditEvents: [],
  recoveryActions: [],
  invariantFlags: {
    'flutter_imported_by_shell_core': false,
    'blue_tanuki_imported_by_shell_core': false,
    'adapter_metadata_can_escalate_authority': false,
    'memory_cache_previous_state_can_grant_authority': false,
    'full_payload_projected_without_full_visibility': false,
    'installer_setup_state_can_grant_authority': false,
    'mobile_device_state_can_grant_authority': false,
  },
  setupDoctorStatus: 'warning',
  installerGrantsAuthority: false,
  installerSilentlyApprovesPermissions: false,
  setupDoctorChecks: [
    SetupDoctorCheckRecord(
      checkId: 'local.snapshot',
      status: 'warning',
      message: 'Local Shell Core snapshot file not found',
      recoveryInstruction:
          'Refresh the development diagnostic snapshot or set GUI_SHELL_SNAPSHOT_JSON for local inspection.',
      grantsAuthority: false,
    ),
  ],
  trustRecords: [
    TrustRecord(
      scope: 'workspace_trust',
      state: 'unknown',
      source: 'local snapshot missing',
      expiresAt: null,
      blockedOperations: ['agent.execute'],
    ),
    TrustRecord(
      scope: 'installer_trust',
      state: 'unknown',
      source: 'installed-path evidence missing',
      expiresAt: null,
      blockedOperations: ['release_ready_claim'],
    ),
  ],
  authorityMap: [],
  adapterCatalog: [],
  permissionDiffs: [],
  problems: [
    ProblemRecord(
      problemId: 'local-snapshot-missing',
      severity: 'warning',
      category: 'missing_evidence',
      message: 'Local Shell Core snapshot file is missing.',
      target: '.gui_shell/shell_snapshot.json',
      recoveryId: 'recover-local-snapshot',
      item: 'fallback snapshot active',
      classification: 'known_limitation',
      reason: 'Local snapshot was not available; safe fallback is active.',
      requiredAction:
          'Refresh the development diagnostic snapshot for local inspection.',
      blocksRelease: false,
    ),
  ],
  evidence: [
    EvidenceRecord(
      evidenceId: 'local-shell-snapshot',
      kind: 'snapshot',
      status: 'missing',
      path: '.gui_shell/shell_snapshot.json',
      hash: '',
      exportable: false,
    ),
  ],
  settings: [
    SettingRecord(
      key: 'snapshot.path',
      group: 'local',
      defaultValue: '.gui_shell/shell_snapshot.json',
      currentValue: '.gui_shell/shell_snapshot.json',
      effectiveValue: '.gui_shell/shell_snapshot.json',
      source: 'GUI_SHELL_SNAPSHOT_JSON',
      modified: false,
      dangerous: false,
      authorityRelated: false,
    ),
    SettingRecord(
      key: 'release.state',
      group: 'release',
      defaultValue: 'not claimed',
      currentValue: 'not claimed',
      effectiveValue: 'not claimed',
      source: 'fallback invariant',
      modified: false,
      dangerous: true,
      authorityRelated: true,
    ),
  ],
  auditChainStatus: 'unknown',
  networkExposure: 'unknown',
  releaseBlockerCount: 1,
  evidenceSummary: EvidenceSummaryRecord(
    schemaCheck: 'passed',
    conformanceCheckCount: 138,
    releaseSmoke: 'passed',
    releaseGateCheck: 'passed',
    evidenceBundle: 'passed',
    validateAll: 'passed',
    strictWindowsRelease: 'expected fail',
    missingMeasuredWindowsEvidence: true,
    missingSetupDoctorEvidence: true,
    missingAuditAnchorExternalTamperEvidence: true,
    ownerGo: 'missing',
  ),
  recoveryPlaybook: [
    RecoveryPlaybookRecord(
      item: 'local Shell Core snapshot missing',
      severity: 'owner-use',
      classification: 'required_for_v1',
      safeToIgnoreForPhaseB: false,
      requiredAction:
          'Refresh the development diagnostic snapshot or set GUI_SHELL_SNAPSHOT_JSON for local inspection.',
      blocksCompletedProductRelease: false,
    ),
    RecoveryPlaybookRecord(
      item: 'measured Windows installed-path evidence missing',
      severity: 'release',
      classification: 'release_blocker',
      safeToIgnoreForPhaseB: true,
      requiredAction: 'Run hardened Windows installed smoke on native Windows.',
      blocksCompletedProductRelease: true,
    ),
  ],
  snapshotSource: 'fallback',
  snapshotPath: '.gui_shell/shell_snapshot.json',
  snapshotGeneratedAt: 'missing',
  snapshotFreshness: 'missing',
);
