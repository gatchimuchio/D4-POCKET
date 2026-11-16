import 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerTransport, BrokerClientException;

enum BrokerUiStatus {
  ready('READY'),
  setupRequired('SETUP_REQUIRED'),
  authRequired('AUTH_REQUIRED'),
  unsupported('UNSUPPORTED'),
  degraded('DEGRADED'),
  suspended('SUSPENDED'),
  recoverableError('RECOVERABLE_ERROR');

  const BrokerUiStatus(this.label);

  final String label;
}

class BrokerHealthObservation {
  const BrokerHealthObservation({
    required this.responseStatus,
    required this.healthStatus,
    required this.evidenceSource,
    required this.authorityCutoverStatus,
    required this.commandDispatchEnabled,
    required this.persistenceRequired,
    required this.persistenceReady,
  });

  final String responseStatus;
  final String healthStatus;
  final String evidenceSource;
  final String authorityCutoverStatus;
  final bool commandDispatchEnabled;
  final bool persistenceRequired;
  final bool persistenceReady;

  BrokerUiStatus get uiStatus {
    if (responseStatus == 'suspended' ||
        healthStatus == 'suspend' ||
        (persistenceRequired && !persistenceReady)) {
      return BrokerUiStatus.suspended;
    }
    if (healthStatus == 'degraded') return BrokerUiStatus.degraded;
    if (!commandDispatchEnabled || authorityCutoverStatus != 'active') {
      return BrokerUiStatus.suspended;
    }
    return BrokerUiStatus.ready;
  }

  String get safeSummary =>
      'Broker内部状態: $healthStatus / 権限切替: $authorityCutoverStatus / '
      'Task dispatch: ${commandDispatchEnabled ? 'enabled' : 'disabled'} / '
      '永続化準備: ${persistenceReady ? 'ready' : 'not ready'} / '
      '証拠: $evidenceSource';
}

class BrokerHealthClient {
  const BrokerHealthClient._();

  static Future<BrokerHealthObservation> read(
    BrokerTransport transport,
  ) async {
    final response = await transport.request('health');
    const responseFields = <String>{
      'request_id',
      'operation',
      'status',
      'evidence_source',
      'audit_event_id',
      'error',
      'health',
      'body',
      'shutdown_requested',
    };
    final requestId = response['request_id'];
    final auditEventId = response['audit_event_id'];
    final responseStatus = response['status'];
    final responseEvidence = response['evidence_source'];
    final accepted = responseStatus == 'accepted';
    final suspended = responseStatus == 'suspended';
    if (response.length != responseFields.length ||
        response.keys.any((key) => !responseFields.contains(key)) ||
        requestId is! String ||
        !_isSafeHealthIdentifier(requestId) ||
        response['operation'] != 'health' ||
        (!accepted && !suspended) ||
        auditEventId is! String ||
        !_isSafeHealthIdentifier(auditEventId) ||
        response['body'] != null ||
        response['shutdown_requested'] != false ||
        (accepted &&
            (responseEvidence != 'LIVE_RUNTIME' ||
                response['error'] != null)) ||
        (suspended &&
            (responseEvidence != 'INTERNAL_STATE' ||
                response['error'] is! Map))) {
      throw const BrokerClientException('Broker Health応答を確認できません');
    }

    final rawHealth = response['health'];
    if (rawHealth is! Map) {
      throw const BrokerClientException('Broker Health内容を確認できません');
    }
    final health = Map<String, Object?>.from(rawHealth);
    const healthFields = <String>{
      'broker_id',
      'status',
      'boundary_role',
      'authority_cutover_status',
      'command_dispatch_enabled',
      'audit_append_enabled',
      'audit_persistence',
      'replay_persistence',
      'session_persistence',
      'persistence_required',
      'persistence_ready',
      'evidence_source',
    };
    final brokerId = health['broker_id'];
    final healthEvidence = health['evidence_source'];
    final healthStatus = health['status'];
    final authorityCutoverStatus = health['authority_cutover_status'];
    final commandDispatchEnabled = health['command_dispatch_enabled'];
    final persistenceRequired = health['persistence_required'];
    final persistenceReady = health['persistence_ready'];
    if (health.length != healthFields.length ||
        health.keys.any((key) => !healthFields.contains(key)) ||
        brokerId is! String ||
        brokerId.trim().isEmpty ||
        brokerId.length > 128 ||
        !const {'ready', 'degraded', 'suspend'}.contains(healthStatus) ||
        health['boundary_role'] != 'rust_security_broker_candidate' ||
        authorityCutoverStatus != 'not_active' ||
        commandDispatchEnabled != false ||
        health['audit_append_enabled'] != true ||
        !const {'in_memory_skeleton', 'durable_file_store'}
            .contains(health['audit_persistence']) ||
        !const {'in_memory_session_only', 'durable_file_store'}
            .contains(health['replay_persistence']) ||
        !const {'in_memory_session_only', 'durable_file_store'}
            .contains(health['session_persistence']) ||
        persistenceRequired is! bool ||
        persistenceReady is! bool ||
        healthEvidence != responseEvidence) {
      throw const BrokerClientException('Broker Health内容の契約が不正です');
    }
    return BrokerHealthObservation(
      responseStatus: responseStatus as String,
      healthStatus: healthStatus as String,
      evidenceSource: responseEvidence as String,
      authorityCutoverStatus: authorityCutoverStatus as String,
      commandDispatchEnabled: commandDispatchEnabled as bool,
      persistenceRequired: persistenceRequired,
      persistenceReady: persistenceReady,
    );
  }
}

bool _isSafeHealthIdentifier(String value) =>
    value.trim().isNotEmpty &&
    value.length <= 256 &&
    !value.codeUnits.any((unit) => unit < 0x20 || unit == 0x7f);
