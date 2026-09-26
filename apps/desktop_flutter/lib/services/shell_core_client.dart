import 'dart:convert';

import '../models/generated_contracts.dart';
import 'broker_client.dart';
import 'workspace_client.dart';

class D4PocketUiConfiguration {
  const D4PocketUiConfiguration({
    required this.theme,
    required this.density,
    required this.locale,
  });

  const D4PocketUiConfiguration.defaults()
      : theme = 'system',
        density = 'compact',
        locale = 'ja-JP';

  final String theme;
  final String density;
  final String locale;
}

class ShellCoreClient {
  const ShellCoreClient._(
    this.snapshot,
    this.mode, [
    this.workspaceClient,
    this.brokerTransport,
    this.initialUiConfiguration = const D4PocketUiConfiguration.defaults(),
  ]);

  final WorkspaceClient? workspaceClient;
  final BrokerTransport? brokerTransport;
  final D4PocketUiConfiguration initialUiConfiguration;

  final ShellSnapshot snapshot;
  final String mode;

  static Future<ShellCoreClient> product({BrokerTransport? transport}) async {
    try {
      final broker = transport ?? await BrokerClient.connect();
      final healthResponse = await broker.request('health');
      final health = _acceptedResponseBodyMap(
        healthResponse,
        'health',
        bodyKey: 'health',
      );
      final initialConfigurationResponse = await broker.request(
        '初回設定取得',
        payload: const <String, Object?>{'version': 1},
      );
      final initialUiConfiguration = _validatedFirstRunConfiguration(
        _acceptedResponseBodyMap(
          initialConfigurationResponse,
          '初回設定取得',
        ),
      );
      Map<String, Object?> setupDoctorReport;
      try {
        final setupDoctorResponse = await broker.request(
          'Setup Doctor報告取得',
          payload: const <String, Object?>{'version': 1},
        );
        setupDoctorReport = _validatedSetupDoctorReport(
          _acceptedResponseBodyMap(
            setupDoctorResponse,
            'Setup Doctor報告取得',
          ),
        );
      } on Object {
        setupDoctorReport = _unknownSetupDoctorReport(
          'Rust Brokerから有効なSetup Doctor報告を取得できません。',
        );
      }
      final hostCapabilityResponse = await broker.request('ホスト能力');
      final hostCapability = _acceptedResponseBodyMap(
        hostCapabilityResponse,
        'ホスト能力',
      );
      final hostListResponse = await broker.request(
        'Host一覧',
        payload: {'版': 1},
      );
      final hostList = _acceptedResponseBodyMap(hostListResponse, 'Host一覧');
      final adapterListResponse = await broker.request(
        'アダプター一覧',
        payload: {'版': 1},
      );
      final adapterList = _acceptedResponseBodyMap(
        adapterListResponse,
        'アダプター一覧',
      );
      final agentAdapterListResponse = await broker.request(
        'Agent一覧',
        payload: const <String, Object?>{},
      );
      final agentAdapterList = _acceptedResponseBodyMap(
        agentAdapterListResponse,
        'Agent一覧',
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
          'ブローカーがコマンド封筒を停止しなかった: '
          '${commandResponse['status']}',
        );
      }

      return ShellCoreClient._(
        _brokerSnapshot(
          healthResponse: healthResponse,
          health: health,
          setupDoctorReport: setupDoctorReport,
          hostCapabilityResponse: hostCapabilityResponse,
          hostCapability: hostCapability,
          hostListResponse: hostListResponse,
          hostList: hostList,
          adapterListResponse: adapterListResponse,
          adapterList: adapterList,
          agentAdapterListResponse: agentAdapterListResponse,
          agentAdapterList: agentAdapterList,
          normalizeResponse: normalizeResponse,
          projectionResponse: projectionResponse,
          projection: projection,
          protectedEditResponse: protectedEditResponse,
          protectedEdit: protectedEdit,
          commandResponse: commandResponse,
        ),
        'broker',
        WorkspaceClient(broker),
        broker,
        initialUiConfiguration,
      );
    } on Object catch (error) {
      return ShellCoreClient._(
        _brokerUnavailableSnapshot(error.toString()),
        'broker_unavailable',
      );
    }
  }

  factory ShellCoreClient.local({ShellSnapshot? snapshot}) {
    if (snapshot == null) {
      return const ShellCoreClient._(_localFallbackSnapshot, 'local');
    }
    return ShellCoreClient._(
      snapshot.copyWith(
        phaseStatus: snapshot.phaseStatus.copyWith(
          completedProductReleaseClaimed: false,
        ),
        operationStatus: snapshot.operationStatus.copyWith(
          releaseState: 'not claimed',
        ),
        snapshotSource: 'in_memory_diagnostic',
        snapshotPath: 'メモリ内診断値',
        snapshotGeneratedAt: 'unknown',
        snapshotFreshness: 'unknown',
      ),
      'local',
    );
  }

  factory ShellCoreClient.mock() {
    return const ShellCoreClient._(_mockSnapshot, 'mock');
  }

  ShellSnapshot getSnapshot() => snapshot;

  Future<HostSelectionRecord> selectHost(String hostId) async {
    HostRegistryRecord? host;
    for (final item in snapshot.hosts) {
      if (item.hostId == hostId) {
        host = item;
        break;
      }
    }
    if (host == null) {
      throw BrokerClientException('未登録Hostへ切り替えできません: $hostId');
    }
    final transport = brokerTransport;
    if (transport == null) {
      return HostSelectionRecord(
        hostId: host.hostId,
        connectionState: host.connectionState,
        trustState: host.trustState,
        evidenceSource: 'INTERNAL_STATE',
        authorityGenerated: 'なし',
        authorityStrip: true,
        approvalState: 'not_reused',
        auditId: 'local-selection-not-audited',
      );
    }
    final response = await transport.request(
      'Host切替',
      payload: {'版': 1, 'Host ID': hostId},
    );
    final body = _acceptedResponseBodyMap(response, 'Host切替');
    return HostSelectionRecord(
      hostId: body['選択Host ID']?.toString() ?? hostId,
      connectionState: body['接続状態']?.toString() ?? 'unknown',
      trustState: body['Trust']?.toString() ?? 'unknown',
      evidenceSource: body['証拠種別']?.toString() ?? 'unknown',
      authorityGenerated: body['権限生成']?.toString() ?? '不明',
      authorityStrip: body['authority_strip'] == true,
      approvalState: body['承認状態']?.toString() ?? 'unknown',
      auditId: body['監査ID']?.toString() ??
          response['audit_event_id']?.toString() ??
          '',
    );
  }

  Future<AdapterManagementResult> manageAdapter(
    String adapterId,
    String operation,
  ) async {
    final adapter = snapshot.adapterCatalog.where(
      (item) => item.adapterId == adapterId,
    );
    if (adapter.isEmpty) {
      throw BrokerClientException('未登録Adapterへ操作できません: $adapterId');
    }
    if (operation == '導入' || operation == '更新') {
      return const AdapterManagementResult(
        status: 'suspended',
        operation: 'アダプター管理',
        adapterId: '',
        managementState: 'owner_manifest_required',
        verificationState: 'unknown',
        auditId: 'not-audited',
        recoveryId: 'recover-adapter-management',
        message: '導入・更新はBrokerへowner Manifestを提示できる経路が必要です。',
      );
    }
    final transport = brokerTransport;
    if (transport == null) {
      return AdapterManagementResult(
        status: 'suspended',
        operation: operation,
        adapterId: adapterId,
        managementState: adapter.first.managementState,
        verificationState: adapter.first.verificationStatus,
        auditId: 'not-audited',
        recoveryId: 'recover-adapter-management',
        message: 'Broker接続がないためAdapter操作を停止しました。',
      );
    }
    const operations = <String, String>{
      '検証': 'アダプター検証',
      '有効化': 'アダプター有効化',
      '無効化': 'アダプター無効化',
      '隔離': 'アダプター隔離',
      '削除': 'アダプター削除',
    };
    final brokerOperation = operations[operation];
    if (brokerOperation == null) {
      throw BrokerClientException('未対応のAdapter操作です: $operation');
    }
    final response = await transport.request(
      brokerOperation,
      payload: {
        '版': 1,
        '操作': operation,
        'Adapter ID': adapter.first.adapterId,
        'Adapter hash': adapter.first.hash,
      },
    );
    final body = response['body'] is Map
        ? Map<String, Object?>.from(response['body'] as Map)
        : const <String, Object?>{};
    final error = response['error'];
    return AdapterManagementResult(
      status: response['status']?.toString() ?? 'unknown',
      operation: response['operation']?.toString() ?? brokerOperation,
      adapterId: body['Adapter ID']?.toString() ?? adapterId,
      managementState:
          body['管理状態']?.toString() ?? adapter.first.managementState,
      verificationState:
          body['検証状態']?.toString() ?? adapter.first.verificationStatus,
      auditId: response['audit_event_id']?.toString() ??
          body['監査ID']?.toString() ??
          '',
      recoveryId: body['復旧ID']?.toString() ?? 'recover-adapter-management',
      message: error is Map
          ? error['message']?.toString() ?? response.toString()
          : body['承認状態']?.toString() ?? response.toString(),
    );
  }

  Future<TrayStopResult> requestAllRuntimeStop() async {
    final transport = brokerTransport;
    if (transport == null) {
      return const TrayStopResult(
        status: 'suspended',
        stopPerformed: false,
        approvalState: 'owner_reapproval_required',
        targetCount: 0,
        auditId: '',
        recoveryId: 'recover-runtime-stop-request',
        message: 'Broker接続がないため全Runtime停止要求を停止しました。',
      );
    }
    final response = await transport.request(
      '全Runtime停止要求',
      payload: {'版': 1},
    );
    final body = _acceptedResponseBodyMap(response, '全Runtime停止要求');
    final stopPerformed = body['停止実行済み'] == true;
    if (stopPerformed || body['承認状態'] != 'owner_reapproval_required') {
      throw const BrokerClientException(
        '全Runtime停止要求の応答が停止完了または承認境界を保持していません',
      );
    }
    final targets = body['対象'];
    if (targets is! List || targets.length > 256) {
      throw const BrokerClientException(
        '全Runtime停止要求の対象件数が上限を超えているか不正です',
      );
    }
    return TrayStopResult(
      status: response['status']?.toString() ?? 'unknown',
      stopPerformed: false,
      approvalState: body['承認状態']?.toString() ?? 'unknown',
      targetCount: targets.length,
      auditId: response['audit_event_id']?.toString() ?? '',
      recoveryId: body['復旧ID']?.toString() ?? 'recover-runtime-stop-request',
      message: response['error'] is Map
          ? (response['error'] as Map)['message']?.toString() ??
              response.toString()
          : '全Runtime停止要求をBrokerへ渡しました。owner再承認後に個別lifecycleを評価します。',
    );
  }
}

class AdapterManagementResult {
  const AdapterManagementResult({
    required this.status,
    required this.operation,
    required this.adapterId,
    required this.managementState,
    required this.verificationState,
    required this.auditId,
    required this.recoveryId,
    required this.message,
  });

  final String status;
  final String operation;
  final String adapterId;
  final String managementState;
  final String verificationState;
  final String auditId;
  final String recoveryId;
  final String message;
}

class TrayStopResult {
  const TrayStopResult({
    required this.status,
    required this.stopPerformed,
    required this.approvalState,
    required this.targetCount,
    required this.auditId,
    required this.recoveryId,
    required this.message,
  });

  final String status;
  final bool stopPerformed;
  final String approvalState;
  final int targetCount;
  final String auditId;
  final String recoveryId;
  final String message;
}

Map<String, Object?> _acceptedResponseBodyMap(
  Map<String, Object?> response,
  String operation, {
  String bodyKey = 'body',
}) {
  _requireAccepted(response, operation);
  final body = response[bodyKey];
  if (body is! Map) {
    throw BrokerClientException('$operation 応答に $bodyKey object がありません');
  }
  return Map<String, Object?>.from(body);
}

Map<String, Object?> _hostCapabilitySnapshotJson(
  Map<String, Object?> body,
) {
  final capabilities = body['能力'];
  if (capabilities is! List) {
    throw const BrokerClientException('ホスト能力応答に能力配列がありません');
  }
  return {
    'host_id': body['ホストID']?.toString() ?? '',
    'display_name': body['表示名']?.toString() ?? '',
    'platform': body['プラットフォーム']?.toString() ?? 'unknown',
    'status': body['状態']?.toString() ?? 'unavailable',
    'capabilities': [
      for (final item in capabilities)
        if (item is Map)
          {
            'capability_id': item['能力ID']?.toString() ?? '',
            'status': item['状態']?.toString() ?? 'unavailable',
            'evidence_source': item['証拠種別']?.toString() ?? 'unknown',
            'reason': item['理由']?.toString() ?? '',
          },
    ],
  };
}

List<Map<String, Object?>> _hostRegistrySnapshotJson(
  Map<String, Object?> body,
) {
  final hosts = body['Host一覧'];
  if (hosts is! List) {
    throw const BrokerClientException('Host一覧応答にHost配列がありません');
  }
  return [
    for (final item in hosts)
      if (item is Map) _hostRegistryRecordSnapshotJson(item),
  ];
}

List<Map<String, Object?>> _adapterCatalogSnapshotJson(
  Map<String, Object?> body,
) {
  final adapters = body['Adapter一覧'];
  if (adapters is! List) {
    throw const BrokerClientException('アダプター一覧応答にAdapter配列がありません');
  }
  return [
    for (final item in adapters)
      if (item is Map) _adapterCatalogRecordSnapshotJson(item),
  ];
}

List<Map<String, Object?>> _agentAdapterSnapshotJson(
  Map<String, Object?> body,
) {
  final agents = body['Agent'];
  if (agents is! List) {
    throw const BrokerClientException('Agent一覧応答にAgent配列がありません');
  }
  return [
    for (final item in agents)
      if (item is Map) Map<String, Object?>.from(item),
  ];
}

const _adapterIdSnapshotKey = 'adapter_id';
const _runtimeIdSnapshotKey = 'runtime_id';
const _adapterIdInputKey = 'Adapter ID';
const _runtimeIdInputKey = 'Runtime ID';

Map<String, Object?> _adapterCatalogRecordSnapshotJson(Map item) {
  final raw = Map<String, Object?>.from(item);
  return {
    _adapterIdSnapshotKey: raw[_adapterIdInputKey]?.toString() ?? '',
    _runtimeIdSnapshotKey: raw[_runtimeIdInputKey]?.toString() ?? '',
    'publisher': raw['発行者']?.toString() ?? '',
    'version': raw['version']?.toString() ?? '',
    'signature': raw['署名状態']?.toString() ?? 'unknown',
    'hash': raw['hash']?.toString() ?? '',
    'requested_capabilities': raw['要求Capability'] is List
        ? List<Object?>.from(raw['要求Capability'] as List)
        : const <Object?>[],
    'granted_capabilities': const <Object?>[],
    'denied_capabilities': const <Object?>[],
    'trust_status': raw['署名状態']?.toString() ?? 'unknown',
    'last_verified': raw['最終検証']?.toString() ?? '',
    'update_available': raw['更新可能'] == true,
    'known_risks': raw['既知の危険'] is List
        ? List<Object?>.from(raw['既知の危険'] as List)
        : const <Object?>[],
    'source': raw['source']?.toString() ?? 'unknown',
    'transport': raw['transport']?.toString() ?? 'unknown',
    'content_exposure': raw['Content Exposure']?.toString() ?? 'unknown',
    'permission_diff': raw['許可差分'] is List
        ? List<Object?>.from(raw['許可差分'] as List)
        : const <Object?>[],
    'compatibility': raw['互換性']?.toString() ?? 'unknown',
    'signature_status': raw['署名状態']?.toString() ?? 'unknown',
    'verification_status': raw['検証状態']?.toString() ?? 'unknown',
    'management_state': raw['管理状態']?.toString() ?? 'unknown',
    'active_state': raw['有効状態']?.toString() ?? 'unknown',
    'evidence_source': raw['証拠種別']?.toString() ?? 'unknown',
    'visibility': raw['公開範囲']?.toString() ?? 'none',
    'authority_strip': raw['authority_strip'] == true,
    'audit_id': raw['監査ID']?.toString() ?? '',
    'recovery_id': raw['復旧ID']?.toString() ?? '',
  };
}

Map<String, Object?> _hostRegistryRecordSnapshotJson(Map item) {
  final raw = Map<String, Object?>.from(item);
  final trust = Map<String, Object?>.from(raw['Trust'] as Map? ?? const {});
  final summary = Map<String, Object?>.from(
    raw['Runtime summary'] as Map? ?? const {},
  );
  return {
    'host_id': raw['Host ID']?.toString() ?? '',
    'display_name': raw['表示名']?.toString() ?? '',
    'platform': raw['Platform']?.toString() ?? 'unknown',
    'connection_state': raw['接続状態']?.toString() ?? 'unknown',
    'trust_state': trust['state']?.toString() ?? 'unknown',
    'runtime_summary': {
      'runtime_count':
          summary['runtime_count'] is int ? summary['runtime_count'] : 0,
      'agent_count': summary['agent_count'] is int ? summary['agent_count'] : 0,
    },
    'evidence_source': raw['証拠種別']?.toString() ?? 'unknown',
    'visibility': raw['公開範囲']?.toString() ?? 'none',
    'authority_strip': raw['authority_strip'] == true,
    'last_connection': raw['最終接続'],
  };
}

void _requireAccepted(Map<String, Object?> response, String operation) {
  if (response['status'] != 'accepted') {
    final error = response['error'];
    final message = error is Map
        ? error['message']?.toString() ?? response.toString()
        : response.toString();
    throw BrokerClientException(
      '$operation の broker request が拒否されました: $message',
    );
  }
}

Map<String, Object?> _unknownSetupDoctorReport(String message) {
  const checkIds = <String>[
    'setup_doctor.ran_from_installed_app_path',
    'setup_doctor.runtime_connection',
    'setup_doctor.authority_boundary',
    'setup_doctor.network_public_bind',
    'setup_doctor.recovery_instruction',
    'setup_doctor.audit_storage',
    'setup_doctor.config_created',
  ];
  return <String, Object?>{
    'status': 'unknown',
    'checks': [
      for (final checkId in checkIds)
        <String, Object?>{
          'check_id': checkId,
          'status': 'unknown',
          'message': message,
          'recovery_instruction': 'Broker接続を確認してSetup Doctorを再取得してください。',
          'evidence_class': 'INTERNAL_STATE',
          'grants_authority': false,
        },
    ],
  };
}

D4PocketUiConfiguration _validatedFirstRunConfiguration(
    Map<String, Object?> configuration) {
  const expectedFields = <String>{'version', 'product', 'ui_preferences'};
  const expectedPreferences = <String>{'theme', 'density', 'locale'};
  final preferences = configuration['ui_preferences'];
  if (configuration.keys.toSet().length != expectedFields.length ||
      !configuration.keys.toSet().containsAll(expectedFields) ||
      configuration['version'] != 1 ||
      configuration['product'] != 'D4 Pocket' ||
      preferences is! Map ||
      preferences.keys.toSet().length != expectedPreferences.length ||
      !preferences.keys.toSet().containsAll(expectedPreferences) ||
      preferences['theme'] != 'system' ||
      preferences['density'] != 'compact' ||
      preferences['locale'] != 'ja-JP') {
    throw const BrokerClientException('Broker初回UI設定が固定Schemaまたは既定値に適合しません');
  }
  return const D4PocketUiConfiguration.defaults();
}

Map<String, Object?> _validatedSetupDoctorReport(Map<String, Object?> report) {
  const reportFields = <String>{
    'version',
    'report_id',
    'generated_at',
    'status',
    'evidence_source',
    'checks',
    'installer_grants_authority',
    'installer_silently_approves_permissions',
  };
  const checkFields = <String>{
    'check_id',
    'status',
    'message',
    'recovery_instruction',
    'evidence_class',
    'grants_authority',
  };
  const checkIds = <String>{
    'setup_doctor.ran_from_installed_app_path',
    'setup_doctor.runtime_connection',
    'setup_doctor.authority_boundary',
    'setup_doctor.network_public_bind',
    'setup_doctor.recovery_instruction',
    'setup_doctor.audit_storage',
    'setup_doctor.config_created',
  };
  final actualReportFields = report.keys.toSet();
  final checks = report['checks'];
  final reportId = report['report_id'];
  final generatedAt = report['generated_at'];
  if (actualReportFields.length != reportFields.length ||
      !actualReportFields.containsAll(reportFields) ||
      reportId is! String ||
      !RegExp(r'^setup-doctor-[a-f0-9]{64}$').hasMatch(reportId) ||
      generatedAt is! String ||
      DateTime.tryParse(generatedAt) == null ||
      report['version'] != 1 ||
      report['evidence_source'] != 'LIVE_RUNTIME' ||
      report['installer_grants_authority'] != false ||
      report['installer_silently_approves_permissions'] != false ||
      checks is! List ||
      checks.length != checkIds.length) {
    return _unknownSetupDoctorReport('Brokerが返したSetup Doctor報告の形を確認できません。');
  }
  final seen = <String>{};
  final normalized = <Map<String, Object?>>[];
  var hasWarningOrUnknown = false;
  var hasFailure = false;
  for (final item in checks) {
    if (item is! Map) {
      return _unknownSetupDoctorReport(
          'Brokerが返したSetup Doctor checkの形を確認できません。');
    }
    final check = Map<String, Object?>.from(item);
    final actualCheckFields = check.keys.toSet();
    final id = check['check_id'];
    final status = check['status'];
    final evidenceClass = check['evidence_class'];
    if (actualCheckFields.length != checkFields.length ||
        !actualCheckFields.containsAll(checkFields) ||
        id is! String ||
        !checkIds.contains(id) ||
        !seen.add(id) ||
        status is! String ||
        !const {'pass', 'warning', 'fail', 'unknown'}.contains(status) ||
        check['message'] is! String ||
        (check['message'] as String).isEmpty ||
        check['recovery_instruction'] is! String ||
        (check['recovery_instruction'] as String).isEmpty ||
        !const {'CONFIG', 'INTERNAL_STATE', 'LIVE_RUNTIME'}
            .contains(evidenceClass) ||
        check['grants_authority'] != false) {
      return _unknownSetupDoctorReport(
          'Brokerが返したSetup Doctor checkの内容を検証できません。');
    }
    hasFailure |= status == 'fail';
    hasWarningOrUnknown |= status == 'warning' || status == 'unknown';
    normalized.add(check);
  }
  final expectedStatus =
      hasFailure ? 'fail' : (hasWarningOrUnknown ? 'warning' : 'pass');
  if (seen.length != checkIds.length || report['status'] != expectedStatus) {
    return _unknownSetupDoctorReport(
        'Brokerが返したSetup Doctor report全体の状態が一致しません。');
  }
  return <String, Object?>{'status': expectedStatus, 'checks': normalized};
}

ShellSnapshot _brokerSnapshot({
  required Map<String, Object?> healthResponse,
  required Map<String, Object?> health,
  required Map<String, Object?> setupDoctorReport,
  required Map<String, Object?> hostCapabilityResponse,
  required Map<String, Object?> hostCapability,
  required Map<String, Object?> hostListResponse,
  required Map<String, Object?> hostList,
  required Map<String, Object?> adapterListResponse,
  required Map<String, Object?> adapterList,
  required Map<String, Object?> agentAdapterListResponse,
  required Map<String, Object?> agentAdapterList,
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
    problems.add(
      _releaseProblemJson(
        problemId: 'broker-persistence-not-ready',
        item: 'ブローカー永続保管庫の準備未完了',
        severity: 'error',
        category: 'broker_ipc',
        message: 'Rustブローカーが監査／再生／セッションの永続化準備完了を報告していません。',
        target: 'rust_security_broker',
        recoveryId: 'recover-broker-persistence',
        requiredAction: '永続保管庫へアクセスできる状態でRustブローカーを起動し、製品UIを再接続してください。',
        blocksOwnerUse: true,
      ),
    );
  }
  if (cutoverStatus != 'active') {
    problems.add(
      _releaseProblemJson(
        problemId: 'broker-authority-cutover-not-active',
        item: 'ブローカー権限切替えが未稼働',
        severity: 'warning',
        category: 'authority_cutover',
        message: 'Flutterはブローカーを介していますが、ブローカーは権限切替えの稼働を主張していません。',
        target: 'authority_cutover_status',
        recoveryId: 'recover-authority-cutover',
        requiredAction: 'ブローカー権限切替えを完了した後に限り、能動的な権限操作を許可してください。',
      ),
    );
  }
  if (!commandDispatchEnabled) {
    problems.add(
      _releaseProblemJson(
        problemId: 'broker-command-dispatch-suspended',
        item: 'ブローカーのコマンド配送は停止中',
        severity: 'warning',
        category: 'command_envelope',
        message: '外部コマンド配送はRustブローカーによって引き続き停止されています。',
        target: 'command_dispatch_enabled',
        recoveryId: 'recover-command-dispatch',
        requiredAction: '外部配送を有効にする前に、コマンド封筒の権限対応を完了してください。',
      ),
    );
  }
  if (!protectedEditRejected) {
    problems.add(
      _releaseProblemJson(
        problemId: 'approval-protected-field-edit-not-rejected',
        item: '承認の保護項目編集が拒否されなかった',
        severity: 'error',
        category: 'approval',
        message: 'ブローカーが承認の保護項目payload_hashに対する編集試行を拒否しませんでした。',
        target: 'approval_edit',
        recoveryId: 'recover-approval-protected-edit',
        requiredAction: '保護項目の強制を復旧するまで権限作用を停止してください。',
        blocksOwnerUse: true,
      ),
    );
  }

  final projectedContent = projection.containsKey('redacted_payload')
      ? Map<String, Object?>.from(projection['redacted_payload'] as Map? ?? {})
      : projection;
  final projectionRedacted = !jsonEncode(projectedContent).contains('hidden');
  if (!projectionRedacted) {
    problems.add(
      _releaseProblemJson(
        problemId: 'broker-content-projection-leaked-full-payload',
        item: 'ブローカー内容射影から完全内容が漏えい',
        severity: 'error',
        category: 'content_projection',
        message: 'ブローカー内容射影が、非表示の完全内容に対する試行からデータを返しました。',
        target: 'content_projection',
        recoveryId: 'recover-content-projection',
        requiredAction: '権限作用を停止し、内容可視性の強制を復旧してください。',
        blocksOwnerUse: true,
      ),
    );
  }
  final auditEvents = [
    _auditJson(healthResponse, 'broker.health', 'accepted'),
    _auditJson(hostCapabilityResponse, 'broker.host_capability', 'accepted'),
    _auditJson(hostListResponse, 'broker.host_list', 'accepted'),
    _auditJson(adapterListResponse, 'broker.adapter_list', 'accepted'),
    _auditJson(
        agentAdapterListResponse, 'broker.agent_adapter_list', 'accepted'),
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
            '状態=$healthStatus 永続化準備=$persistenceReady 切替え=$cutoverStatus',
      },
    ],
    'host_capabilities': [_hostCapabilitySnapshotJson(hostCapability)],
    'hosts': _hostRegistrySnapshotJson(hostList),
    'agent_sessions': [],
    'agent_adapters': _agentAdapterSnapshotJson(agentAdapterList),
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
        },
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
    'setup_doctor_status': setupDoctorReport['status']?.toString() ?? 'unknown',
    'installer_grants_authority': false,
    'installer_silently_approves_permissions': false,
    'setup_doctor_checks': setupDoctorReport['checks'],
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
    'adapter_catalog': _adapterCatalogSnapshotJson(adapterList),
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
      'schema_check': '製品UIでは未評価',
      'conformance_check_count': 0,
      'release_smoke': '製品UIでは未評価',
      'release_gate_check': '製品UIでは未評価',
      'evidence_bundle': '製品UIでは未評価',
      'validate_all': '製品UIでは未評価',
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
    item: 'ブローカー製品経路を利用不可',
    severity: 'error',
    category: 'broker_ipc',
    message: 'Flutter製品の権限経路は、受理されたブローカーIPCを確立できませんでした。',
    target: 'rust_security_broker',
    recoveryId: 'recover-broker-product-path',
    requiredAction: 'Rustブローカーを再起動または修復してください。権限根拠をローカルJSONへ代替しないでください。',
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
      },
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
      },
    ],
    'recovery_actions': [
      {
        'recovery_id': 'recover-broker-product-path',
        'severity': 'error',
        'message': 'ブローカーIPCを復旧するまで権限作用は無効です。',
        'safe_to_retry': true,
      },
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
        'message': 'ブローカーIPCを利用できないか、認証が拒否されました。',
        'recovery_instruction': 'Rustブローカーを再起動して再接続してください。ローカルJSONは診断専用です。',
        'grants_authority': false,
      },
      {
        'check_id': 'broker.fail_closed',
        'status': 'pass',
        'message': 'Flutter製品経路はローカルスナップショットを権限化せず、SUSPENDを返しました。',
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
          'external_command_dispatch',
        ],
      },
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
      },
    ],
    'settings': [],
    'audit_chain_status': 'not_ready',
    'network_exposure': '127.0.0.1 restricted broker IPC',
    'release_blocker_count': 1,
    'evidence_summary': {
      'schema_check': '製品UIでは未評価',
      'conformance_check_count': 0,
      'release_smoke': '製品UIでは未評価',
      'release_gate_check': '製品UIでは未評価',
      'evidence_bundle': '製品UIでは未評価',
      'validate_all': '製品UIでは未評価',
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
    'summary': 'ブローカー経由の承認射影試行',
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
      diagnosticSummary: '模擬アダプター契約を利用可能',
    ),
  ],
  agentSessions: [
    AgentSessionRecord(
      sessionId: 'agent-session-1',
      workspace: '/workspace/project',
      task: '文書を更新する',
      changedFiles: ['README.md', 'docs/STRATEGY.md'],
      toolCalls: ['shell.command', 'git.diff'],
      shellCommands: [
        'python3 tooling/conformance_tests/run_conformance_skeleton.py',
      ],
      testStatus: '適合検査に合格',
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
        'authority_context',
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
      message: 'この作用を実行する前に許可が必要です。',
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
      message: 'ホストOSを検出しました',
      recoveryInstruction: null,
      grantsAuthority: false,
    ),
    SetupDoctorCheckRecord(
      checkId: 'filesystem.permission',
      status: 'pass',
      message: '監査保管先へ書込み可能です',
      recoveryInstruction: null,
      grantsAuthority: false,
    ),
    SetupDoctorCheckRecord(
      checkId: 'network.public_bind',
      status: 'warning',
      message: '公開bindには操作者の明示確認が必要です',
      recoveryInstruction: '許可と承認が公開bindを明示的に認めない限り、実行系はlocalhostに限定してください。',
      grantsAuthority: false,
    ),
  ],
  trustRecords: [
    TrustRecord(
      scope: 'workspace_trust',
      state: 'restricted',
      source: 'ローカル方針',
      expiresAt: null,
      blockedOperations: ['process.spawn', 'network.public_bind'],
    ),
    TrustRecord(
      scope: 'runtime_trust',
      state: 'trusted',
      source: '署名済みマニフェスト',
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
      source: 'インストール先証拠なし',
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
      warning: '承認保留中',
    ),
  ],
  adapterCatalog: [
    AdapterCatalogRecord(
      adapterId: 'blue_tanuki_reference',
      runtimeId: 'blue_tanuki',
      publisher: 'GUI-Shell参照実装',
      version: '0.1.0',
      signature: 'development',
      hash: 'sha256:pending',
      requestedCapabilities: ['filesystem.write'],
      grantedCapabilities: [],
      deniedCapabilities: ['network.public_bind'],
      trustStatus: 'inherited',
      lastVerified: '開発簡易検査',
      updateAvailable: false,
      knownRisks: ['参照アダプター専用'],
    ),
  ],
  permissionDiffs: [
    PermissionDiffRecord(
      subject: 'blue_tanuki_reference',
      added: ['filesystem.write'],
      removed: [],
      changed: ['content_visibility: full → redacted（完全表示から墨消し表示）'],
      dangerous: [],
    ),
  ],
  problems: [
    ProblemRecord(
      problemId: 'windows-installed-evidence-missing',
      severity: 'blocked',
      category: 'missing_evidence',
      message: 'Windowsインストール先の証拠がありません。',
      target: 'release_evidence/windows_installed_smoke.json',
      recoveryId: 'recover-windows-evidence',
      item: 'Windowsインストール先の初回起動実測証拠なし',
      classification: 'release_blocker',
      reason: 'インストール先の初回起動を実測した証拠が記録されていません。',
      requiredAction: 'ネイティブWindowsで強化済みインストール簡易検査を実行してください。',
      blocksRelease: true,
    ),
    ProblemRecord(
      problemId: 'setup-doctor-installed-evidence-missing',
      severity: 'blocked',
      category: 'missing_evidence',
      message: 'インストール先で生成した非合成の環境診断証拠がありません。',
      target: 'release_evidence/windows_installed_smoke.json',
      recoveryId: 'recover-setup-doctor-evidence',
      item: 'インストール先の非合成環境診断証拠なし',
      classification: 'release_blocker',
      reason: 'インストール済みアプリのパスから環境診断を実行した証明がありません。',
      requiredAction: 'インストール済みWindowsアプリのパスから環境診断を実行し、必須検査を記録してください。',
      blocksRelease: true,
    ),
    ProblemRecord(
      problemId: 'owner-go-missing',
      severity: 'blocked',
      category: 'release_gate',
      message: '所有者GOがありません。',
      target: 'リリース確認表',
      recoveryId: 'recover-owner-go',
      item: '所有者GOなし',
      classification: 'release_blocker',
      reason: '完成製品のリリースには所有者の明示承認が必要です。',
      requiredAction: 'リリース遮断要因を解消した後に所有者GOを記録してください。',
      blocksRelease: true,
    ),
    ProblemRecord(
      problemId: 'macos-unverified',
      severity: 'info',
      category: 'scope',
      message: 'macOSは未検証です。',
      target: 'デスクトップ対応表',
      recoveryId: 'recover-macos-validation',
      item: 'macOS未検証',
      classification: 'known_limitation',
      reason: 'macOS検証環境を利用できません。',
      requiredAction: 'macOS対応を主張する前にmacOS上で検証してください。',
      blocksRelease: false,
    ),
    ProblemRecord(
      problemId: 'mobile-post-v1',
      severity: 'info',
      category: 'scope',
      message: 'モバイル完全リリースはv1後の範囲です。',
      target: 'モバイル状態',
      recoveryId: 'recover-mobile-scope',
      item: 'モバイルはv1後の範囲',
      classification: 'post_v1_scope',
      reason: '所有者が範囲を変更しない限り、v1.0はWindows優先のデスクトップ版です。',
      requiredAction: 'モバイルのリリース作業を保留してください。',
      blocksRelease: false,
    ),
    ProblemRecord(
      problemId: 'paid-qc-later',
      severity: 'info',
      category: 'scope',
      message: '有償製品の品質管理は後続段階です。',
      target: '段階戦略',
      recoveryId: 'recover-paid-qc',
      item: '有償製品の品質管理は後続',
      classification: 'post_v1_scope',
      reason: '段階Bは所有者利用の堅牢化であり、有償製品の品質管理ではありません。',
      requiredAction: '有償製品の品質管理は段階Fまで保留してください。',
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
      source: 'Shell Core方針',
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
      source: '許可台帳',
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
      source: '厳格リリース関門',
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
      item: 'Windowsインストール先の実測証拠なし',
      severity: 'release',
      classification: 'release_blocker',
      safeToIgnoreForPhaseB: true,
      requiredAction: 'ネイティブWindowsで強化済みインストール簡易検査を実行してください。',
      blocksCompletedProductRelease: true,
    ),
    RecoveryPlaybookRecord(
      item: 'インストール先の非合成環境診断証拠なし',
      severity: 'release',
      classification: 'release_blocker',
      safeToIgnoreForPhaseB: true,
      requiredAction: 'インストール先から環境診断を実行し、非合成の検査結果を記録してください。',
      blocksCompletedProductRelease: true,
    ),
    RecoveryPlaybookRecord(
      item: '所有者GOなし',
      severity: 'release',
      classification: 'release_blocker',
      safeToIgnoreForPhaseB: true,
      requiredAction: 'リリース遮断要因の検査合格後に所有者GOを記録してください。',
      blocksCompletedProductRelease: true,
    ),
    RecoveryPlaybookRecord(
      item: 'macOS未検証',
      severity: 'scope',
      classification: 'known_limitation',
      safeToIgnoreForPhaseB: true,
      requiredAction: 'macOS対応を主張する前にmacOS上で検証してください。',
      blocksCompletedProductRelease: false,
    ),
    RecoveryPlaybookRecord(
      item: 'モバイル完全リリース',
      severity: 'scope',
      classification: 'post_v1_scope',
      safeToIgnoreForPhaseB: true,
      requiredAction: 'モバイル完全リリースはv1後まで保留してください。',
      blocksCompletedProductRelease: false,
    ),
    RecoveryPlaybookRecord(
      item: '段階Bの所有者利用上の問題',
      severity: 'owner-use',
      classification: 'required_for_v1',
      safeToIgnoreForPhaseB: false,
      requiredAction: '概要、状態、問題、証拠、復旧の各画面を利用可能に保ってください。',
      blocksCompletedProductRelease: false,
    ),
  ],
  hosts: [
    HostRegistryRecord(
      hostId: 'gui-shell-local-windows',
      displayName: 'ローカル Windows',
      platform: 'windows',
      connectionState: 'pending_review',
      trustState: 'pending_review',
      runtimeCount: 1,
      agentCount: 1,
      evidenceSource: 'INTERNAL_STATE',
      visibility: 'metadata_only',
      authorityStrip: true,
    ),
  ],
  hostCapabilities: [
    HostCapabilityRecord(
      hostId: 'gui-shell-local-windows',
      displayName: 'ローカル Windows',
      platform: 'windows',
      status: 'degraded',
      capabilities: [
        HostCapabilityItemRecord(
          capabilityId: 'filesystem',
          status: 'ready',
          evidenceSource: 'LIVE_RUNTIME',
          reason: 'Brokerの永続保管先を観測できる',
        ),
      ],
    ),
  ],
  snapshotSource: 'mock',
  snapshotPath: '組込み模擬値',
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
      name: 'ローカルShell Core',
      status: 'diagnostic',
      adapterId: 'local_setup_doctor',
      diagnosticSummary: 'ローカル診断スナップショットの代替値',
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
      message: 'メモリ内診断スナップショットが注入されていません',
      recoveryInstruction: '診断データを明示的に渡すか、製品操作ではBroker経路を使用してください。',
      grantsAuthority: false,
    ),
  ],
  trustRecords: [
    TrustRecord(
      scope: 'workspace_trust',
      state: 'unknown',
      source: '診断データ未注入',
      expiresAt: null,
      blockedOperations: ['agent.execute'],
    ),
    TrustRecord(
      scope: 'installer_trust',
      state: 'unknown',
      source: 'インストール先証拠なし',
      expiresAt: null,
      blockedOperations: ['release_ready_claim'],
    ),
  ],
  authorityMap: [],
  adapterCatalog: [],
  permissionDiffs: [],
  problems: [
    ProblemRecord(
      problemId: 'local-snapshot-not-injected',
      severity: 'warning',
      category: 'missing_diagnostic_input',
      message: 'メモリ内Shell Core診断スナップショットが渡されていません。',
      target: 'メモリ内診断値',
      recoveryId: 'recover-local-snapshot',
      item: '代替スナップショット使用中',
      classification: 'known_limitation',
      reason: '明示注入された診断データがないため、安全な代替値を使用しています。',
      requiredAction: '表示専用の診断が必要な場合だけShellSnapshotを明示的に渡してください。',
      blocksRelease: false,
    ),
  ],
  evidence: [
    EvidenceRecord(
      evidenceId: 'local-shell-snapshot',
      kind: 'snapshot',
      status: 'missing',
      path: 'メモリ内診断値',
      hash: '',
      exportable: false,
    ),
  ],
  settings: [
    SettingRecord(
      key: 'snapshot.path',
      group: 'local',
      defaultValue: 'メモリ内診断値',
      currentValue: '未注入',
      effectiveValue: '未注入',
      source: '診断クライアント初期値',
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
      source: '代替不変条件',
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
      item: 'メモリ内診断スナップショット未注入',
      severity: 'owner-use',
      classification: 'required_for_v1',
      safeToIgnoreForPhaseB: false,
      requiredAction: '表示専用の診断が必要な場合だけShellSnapshotを明示的に渡してください。',
      blocksCompletedProductRelease: false,
    ),
    RecoveryPlaybookRecord(
      item: 'Windowsインストール先の実測証拠なし',
      severity: 'release',
      classification: 'release_blocker',
      safeToIgnoreForPhaseB: true,
      requiredAction: 'ネイティブWindowsで強化済みインストール簡易検査を実行してください。',
      blocksCompletedProductRelease: true,
    ),
  ],
  snapshotSource: 'fallback',
  snapshotPath: 'メモリ内診断値（未注入）',
  snapshotGeneratedAt: 'unknown',
  snapshotFreshness: 'unknown',
);
