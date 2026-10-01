import 'broker_client.dart' show BrokerClientException, BrokerTransport;

const _agentTaskExecutionPolicy =
    'gui-shell-agent-task-sandbox-v1-max-runtime-900s';

const _agentTaskStatuses = {
  'pending',
  'running',
  'blocked',
  'completed',
  'failed',
  'cancelled',
  'quarantined',
};

class AgentTaskRequest {
  const AgentTaskRequest({
    required this.runtimeId,
    required this.sessionId,
    required this.workspaceId,
    required this.instruction,
  });

  final String runtimeId;
  final String sessionId;
  final String workspaceId;
  final String instruction;

  Map<String, Object?> toPayload() => {
        'agent_runtime_id': runtimeId,
        'session_id': sessionId,
        'workspace_id': workspaceId,
        'instruction': instruction,
      };

  void validate() {
    if (!_runtimeId.hasMatch(runtimeId) ||
        !_sessionId.hasMatch(sessionId) ||
        !_runtimeId.hasMatch(workspaceId) ||
        instruction.trim().isEmpty ||
        instruction.runes.length > 32768) {
      throw const BrokerClientException('Agent Task要求の形式が不正です');
    }
  }
}

class AgentTaskPreflight {
  const AgentTaskPreflight({
    required this.instructionHash,
    required this.permissionStatus,
    required this.approvalStatus,
  });

  final String instructionHash;
  final String permissionStatus;
  final String approvalStatus;
}

class AgentTaskRecord {
  const AgentTaskRecord({
    required this.taskId,
    required this.runtimeId,
    required this.sessionId,
    required this.workspaceId,
    required this.instructionHash,
    required this.status,
    required this.auditEventId,
    this.resultHash,
  });

  final String taskId;
  final String runtimeId;
  final String sessionId;
  final String workspaceId;
  final String instructionHash;
  final String status;
  final String auditEventId;
  final String? resultHash;
}

class AgentTaskClient {
  AgentTaskClient(this.transport);

  final BrokerTransport transport;

  Future<AgentTaskPreflight> inspect(AgentTaskRequest request) async {
    request.validate();
    final body = await _acceptedBody('Agent作業要求検査', request.toPayload());
    if (!_hasExactKeys(body, const {
      '版',
      '状態',
      '実行状態',
      'Permission状態',
      'Approval状態',
      '実行系ID',
      '対話セッションID',
      '作業領域ID',
      '指示hash',
    })) {
      throw const BrokerClientException('Agent Task事前検査の応答が不正です');
    }
    final permissionStatus = body['Permission状態'];
    final approvalStatus = body['Approval状態'];
    final instructionHash = body['指示hash'];
    if (body['版'] != 1 ||
        body['状態'] != '要求検査済み' ||
        body['実行状態'] != '未実行' ||
        body['実行系ID'] != request.runtimeId ||
        body['対話セッションID'] != request.sessionId ||
        body['作業領域ID'] != request.workspaceId ||
        !_hash.hasMatch(instructionHash is String ? instructionHash : '') ||
        !const {'有効', '未付与'}.contains(permissionStatus) ||
        !const {'有効', '未取得'}.contains(approvalStatus)) {
      throw const BrokerClientException('Agent Task事前検査の応答が要求と一致しません');
    }
    return AgentTaskPreflight(
      instructionHash: instructionHash as String,
      permissionStatus: permissionStatus as String,
      approvalStatus: approvalStatus as String,
    );
  }

  Future<void> grantWorkspacePermission(AgentTaskRequest request) async {
    request.validate();
    final body = await _acceptedBody('AgentTaskWorkspacePermissionGrant', {
      'agent_runtime_id': request.runtimeId,
      'session_id': request.sessionId,
      'workspace_id': request.workspaceId,
    });
    const receiptKeys = {
      'permission_id',
      'agent_runtime_id',
      'session_id',
      'workspace_id',
      'workspace_registration_hash',
      'operation',
      'scope',
      'decision',
      'source',
      'expires_at_epoch_seconds',
      'use_limit',
      'uses_remaining',
      'status',
    };
    final permissionId = body['permission_id'];
    final runtimeId = body['agent_runtime_id'];
    final sessionId = body['session_id'];
    final workspaceId = body['workspace_id'];
    final registrationHash = body['workspace_registration_hash'];
    if (!_hasExactKeys(body, receiptKeys) ||
        permissionId is! String ||
        !_sessionId.hasMatch(permissionId) ||
        runtimeId != request.runtimeId ||
        sessionId != request.sessionId ||
        workspaceId != request.workspaceId ||
        registrationHash is! String ||
        !_hash.hasMatch(registrationHash) ||
        body['operation'] != 'agent_task.execute' ||
        body['scope'] != 'session_workspace_once' ||
        body['decision'] != 'allow' ||
        body['source'] != 'owner' ||
        !_positiveSafeInteger(body['expires_at_epoch_seconds']) ||
        body['use_limit'] != 1 ||
        body['uses_remaining'] != 1 ||
        body['status'] != 'active') {
      throw const BrokerClientException('Workspace Permission receiptが不正です');
    }
  }

  Future<void> grantOwnerApproval(AgentTaskRequest request) async {
    request.validate();
    final body = await _acceptedBody(
      'AgentTaskOwnerApprovalGrant',
      request.toPayload(),
    );
    if (!_hasExactKeys(body, const {
          '状態',
          '実行状態',
          '実行系ID',
          '対話セッションID',
          '作業領域ID',
          '指示hash',
          '実行条件hash',
          '適用ポリシー',
          'expires_at_epoch_seconds',
          'use_limit',
          'uses_remaining',
          'status',
        }) ||
        body['状態'] != 'Owner Approval発行済み' ||
        body['実行状態'] != '未実行' ||
        body['実行系ID'] != request.runtimeId ||
        body['対話セッションID'] != request.sessionId ||
        body['作業領域ID'] != request.workspaceId ||
        !_hash.hasMatch(
            body['指示hash'] is String ? body['指示hash'] as String : '') ||
        !_hash.hasMatch(
            body['実行条件hash'] is String ? body['実行条件hash'] as String : '') ||
        body['適用ポリシー'] != _agentTaskExecutionPolicy ||
        !_positiveSafeInteger(body['expires_at_epoch_seconds']) ||
        body['use_limit'] != 1 ||
        body['uses_remaining'] != 1 ||
        body['status'] != 'issued_unconsumed') {
      throw const BrokerClientException('Owner Approval receiptが不正です');
    }
  }

  Future<AgentTaskRecord> start(AgentTaskRequest request) async {
    request.validate();
    final body = await _acceptedBody('AgentTask実行', request.toPayload());
    return _taskRecord(body, expected: request);
  }

  Future<AgentTaskRecord> state(String taskId) async {
    _validateTaskId(taskId);
    final body = await _acceptedBody('AgentTask状態', {'task_id': taskId});
    return _taskRecord(body, expectedTaskId: taskId);
  }

  Future<AgentTaskRecord> cancel(String taskId) async {
    _validateTaskId(taskId);
    final body = await _acceptedBody('AgentTask取消', {'task_id': taskId});
    return _taskRecord(body, expectedTaskId: taskId);
  }

  Future<Map<String, Object?>> _acceptedBody(
    String operation,
    Map<String, Object?> payload,
  ) async {
    final response = await transport.request(operation, payload: payload);
    final auditId = response['audit_event_id'];
    if (response['operation'] != operation ||
        auditId is! String ||
        auditId.isEmpty ||
        auditId.length > 256) {
      throw const BrokerClientException('Agent Task Broker応答の対応または監査参照が不正です');
    }
    if (response['status'] != 'accepted') {
      final error = response['error'];
      final code = error is Map ? error['code'] : null;
      const safeCodes = {
        'AgentTask実行非対応',
        '要求不正',
        '実行系不在',
        '作業領域不在',
        '権限拒否',
        'セッション不一致',
        '通信失敗',
        '期限超過',
        '応答不正',
        '監査失敗',
        '取消',
      };
      final safeCode = safeCodes.contains(code) ? ': $code' : '';
      throw BrokerClientException('Agent Task操作がBrokerで拒否されました$safeCode');
    }
    if (response['error'] != null || response['body'] is! Map) {
      throw const BrokerClientException('Agent Task Broker応答の本文が不正です');
    }
    final rawBody = response['body'] as Map;
    if (rawBody.keys.any((key) => key is! String)) {
      throw const BrokerClientException('Agent Task Broker応答のfieldが不正です');
    }
    return Map<String, Object?>.from(rawBody);
  }

  AgentTaskRecord _taskRecord(
    Map<String, Object?> body, {
    AgentTaskRequest? expected,
    String? expectedTaskId,
  }) {
    const requiredKeys = {
      'task_id',
      'record_version',
      'agent_runtime_id',
      'session_id',
      'workspace_id',
      'description',
      'instruction_hash',
      'status',
      'audit_event_id',
    };
    if (!requiredKeys.every(body.containsKey) ||
        body.keys.any(
            (key) => !requiredKeys.contains(key) && key != 'result_hash')) {
      throw const BrokerClientException('Agent Task状態recordのfieldが不正です');
    }
    final taskId = body['task_id'];
    final runtimeId = body['agent_runtime_id'];
    final sessionId = body['session_id'];
    final workspaceId = body['workspace_id'];
    final instructionHash = body['instruction_hash'];
    final status = body['status'];
    final auditId = body['audit_event_id'];
    final resultHash = body['result_hash'];
    if (body['record_version'] != 2 ||
        body['description'] != 'Agent作業Task（内容は別のWorkspace差分経路で確認）' ||
        taskId is! String ||
        !_sessionId.hasMatch(taskId) ||
        (expectedTaskId != null && taskId != expectedTaskId) ||
        runtimeId is! String ||
        !_runtimeId.hasMatch(runtimeId) ||
        sessionId is! String ||
        !_sessionId.hasMatch(sessionId) ||
        workspaceId is! String ||
        !_runtimeId.hasMatch(workspaceId) ||
        instructionHash is! String ||
        !_hash.hasMatch(instructionHash) ||
        status is! String ||
        !_agentTaskStatuses.contains(status) ||
        auditId is! String ||
        auditId.isEmpty ||
        auditId.length > 256 ||
        (resultHash != null &&
            (resultHash is! String || !_hash.hasMatch(resultHash)))) {
      throw const BrokerClientException('Agent Task状態recordの識別・値が不正です');
    }
    if (expected != null &&
        (runtimeId != expected.runtimeId ||
            sessionId != expected.sessionId ||
            workspaceId != expected.workspaceId)) {
      throw const BrokerClientException('Agent Task状態recordが現在の要求と一致しません');
    }
    return AgentTaskRecord(
      taskId: taskId,
      runtimeId: runtimeId,
      sessionId: sessionId,
      workspaceId: workspaceId,
      instructionHash: instructionHash,
      status: status,
      auditEventId: auditId,
      resultHash: resultHash as String?,
    );
  }

  void _validateTaskId(String taskId) {
    if (!_sessionId.hasMatch(taskId)) {
      throw const BrokerClientException('Agent Task IDが不正です');
    }
  }
}

bool _hasExactKeys(Map<String, Object?> value, Set<String> keys) =>
    value.length == keys.length && keys.every(value.containsKey);

bool _positiveSafeInteger(Object? value) =>
    value is int && value > 0 && value <= 9007199254740991;

final _runtimeId = RegExp(r'^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$');
final _sessionId = RegExp(r'^[a-f0-9]{32}$');
final _hash = RegExp(r'^sha256:[a-f0-9]{64}$');
