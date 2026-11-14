import 'package:broker_contract/broker_transport.dart'
    show BrokerClientException;

import 'agent_task_client.dart';

typedef MobileAgentTaskBrokerRequest = Future<Map<String, Object?>> Function(
  String operation,
  Map<String, Object?> payload,
);

const _requestIdPattern = r'^[a-f0-9]{32}$';
const _safeIdentifierPattern = r'^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$';
const _hashPattern = r'^sha256:[a-f0-9]{64}$';
const _mobileTaskStates = {'Owner確認待ち', '開始候補', 'Task状態'};

class MobileAgentTaskDraft {
  const MobileAgentTaskDraft({
    required this.requestId,
    required this.runtimeId,
    required this.sessionId,
    required this.workspaceId,
    required this.instruction,
    this.instructionHash,
    required this.createdAt,
    required this.expiresAt,
    this.deviceId,
    this.taskId,
  });

  final String requestId;
  final String? deviceId;
  final String runtimeId;
  final String sessionId;
  final String workspaceId;
  final String instruction;
  final String? instructionHash;
  final int createdAt;
  final int expiresAt;
  final String? taskId;

  bool get linked => taskId != null;

  AgentTaskRequest toAgentTaskRequest() => AgentTaskRequest(
        runtimeId: runtimeId,
        sessionId: sessionId,
        workspaceId: workspaceId,
        instruction: instruction,
        mobileTaskRequestId: requestId,
      );

  factory MobileAgentTaskDraft.fromInbox(Object? raw) {
    final row = _stringMap(raw, '端末Task申請');
    const desktopRowKeys = {'申請ID', '端末ID', '要求', '作成時刻', '期限', 'task_id'};
    const mobileRowKeys = {'申請ID', '要求', '作成時刻', '期限', 'task_id'};
    final desktop = _exactKeys(row, desktopRowKeys);
    if (!desktop && !_exactKeys(row, mobileRowKeys)) _invalid();
    final body = _stringMap(row['要求'], '端末Task要求');
    const requestKeys = {
      'agent_runtime_id',
      'session_id',
      'workspace_id',
      'instruction',
    };
    if (!_exactKeys(body, requestKeys)) _invalid();
    return MobileAgentTaskDraft._create(
      requestId: row['申請ID'],
      deviceId: desktop ? row['端末ID'] : null,
      runtimeId: body['agent_runtime_id'],
      sessionId: body['session_id'],
      workspaceId: body['workspace_id'],
      instruction: body['instruction'],
      instructionHash: null,
      createdAt: row['作成時刻'],
      expiresAt: row['期限'],
      taskId: row['task_id'],
    );
  }

  factory MobileAgentTaskDraft.fromOwnRequest(Object? raw) {
    final row = _stringMap(raw, '自端末Task申請');
    const keys = {
      '申請ID',
      '実行系ID',
      '対話セッションID',
      '作業領域ID',
      '指示hash',
      '作成時刻',
      '期限',
      'task_id',
    };
    if (!_exactKeys(row, keys)) _invalid();
    return MobileAgentTaskDraft._create(
      requestId: row['申請ID'],
      runtimeId: row['実行系ID'],
      sessionId: row['対話セッションID'],
      workspaceId: row['作業領域ID'],
      instruction: '',
      instructionHash: row['指示hash'],
      createdAt: row['作成時刻'],
      expiresAt: row['期限'],
      taskId: row['task_id'],
    );
  }

  factory MobileAgentTaskDraft._create({
    required Object? requestId,
    required Object? runtimeId,
    required Object? sessionId,
    required Object? workspaceId,
    required Object? instruction,
    required Object? instructionHash,
    required Object? createdAt,
    required Object? expiresAt,
    required Object? taskId,
    Object? deviceId,
  }) {
    if (requestId is! String ||
        !RegExp(_requestIdPattern).hasMatch(requestId) ||
        runtimeId is! String ||
        !RegExp(_safeIdentifierPattern).hasMatch(runtimeId) ||
        sessionId is! String ||
        !RegExp(_requestIdPattern).hasMatch(sessionId) ||
        workspaceId is! String ||
        !RegExp(_safeIdentifierPattern).hasMatch(workspaceId) ||
        instruction is! String ||
        (instruction.isEmpty && instructionHash == null) ||
        (instruction.isNotEmpty && instruction.trim().isEmpty) ||
        instruction.runes.length > 32768 ||
        (instructionHash != null &&
            (instructionHash is! String ||
                !RegExp(_hashPattern).hasMatch(instructionHash))) ||
        createdAt is! int ||
        createdAt <= 0 ||
        expiresAt is! int ||
        expiresAt <= createdAt ||
        (deviceId != null &&
            (deviceId is! String ||
                !RegExp(_requestIdPattern).hasMatch(deviceId))) ||
        (taskId != null &&
            (taskId is! String ||
                !RegExp(_requestIdPattern).hasMatch(taskId)))) {
      _invalid();
    }
    final digest = AgentTaskRequest(
      runtimeId: runtimeId,
      sessionId: sessionId,
      workspaceId: workspaceId,
      instruction: instruction,
    );
    // Own-request projections intentionally omit the Task body on Mobile.
    // The identifiers and stored instruction hash have already been checked
    // above; the execution request validator requires a body and is not
    // applicable to this redacted projection.
    if (instruction.isNotEmpty) digest.validate();
    return MobileAgentTaskDraft(
      requestId: requestId,
      deviceId: deviceId as String?,
      runtimeId: runtimeId,
      sessionId: sessionId,
      workspaceId: workspaceId,
      instruction: instruction,
      instructionHash: instructionHash as String?,
      createdAt: createdAt,
      expiresAt: expiresAt,
      taskId: taskId as String?,
    );
  }
}

class MobileAgentTaskStatus {
  const MobileAgentTaskStatus({
    required this.requestId,
    required this.state,
    this.task,
  });

  final String requestId;
  final String state;
  final MobileAgentTaskRecord? task;

  bool get canStart => state == '開始候補';
}

class MobileAgentTaskRecord {
  const MobileAgentTaskRecord({
    required this.taskId,
    required this.runtimeId,
    required this.sessionId,
    required this.workspaceId,
    required this.instructionHash,
    required this.status,
    required this.resultHash,
    required this.resultContentAvailable,
  });

  final String taskId;
  final String runtimeId;
  final String sessionId;
  final String workspaceId;
  final String instructionHash;
  final String status;
  final String? resultHash;
  final bool resultContentAvailable;
}

class MobileAgentTaskClient {
  MobileAgentTaskClient(this._request);

  final MobileAgentTaskBrokerRequest _request;

  Future<MobileAgentTaskDraft> submit(AgentTaskRequest request) async {
    request.validate();
    if (request.mobileTaskRequestId != null ||
        request.mcpCapabilitySnapshotHash != null ||
        request.mcpResultId != null ||
        request.mcpResultHash != null ||
        request.requireUnusedTaskSession) {
      _invalid();
    }
    final body = await _acceptedBody(
      'AgentTask端末申請',
      request.toPayload(),
      distinguishDefinitiveRejection: true,
    );
    const keys = {
      '版',
      '申請ID',
      '状態',
      '実行系ID',
      '対話セッションID',
      '作業領域ID',
      '指示hash',
      '期限',
    };
    if (!_exactKeys(body, keys) ||
        body['版'] != 1 ||
        body['状態'] != 'Owner確認待ち' ||
        body['実行系ID'] != request.runtimeId ||
        body['対話セッションID'] != request.sessionId ||
        body['作業領域ID'] != request.workspaceId ||
        body['期限'] is! int ||
        (body['期限'] as int) <= 0) {
      _invalid();
    }
    final requestId = body['申請ID'];
    final hash = body['指示hash'];
    if (requestId is! String ||
        !RegExp(_requestIdPattern).hasMatch(requestId) ||
        hash is! String ||
        !RegExp(_hashPattern).hasMatch(hash) ||
        (body['期限'] as int) <= 900) {
      _invalid();
    }
    return MobileAgentTaskDraft(
      requestId: requestId,
      runtimeId: request.runtimeId,
      sessionId: request.sessionId,
      workspaceId: request.workspaceId,
      instruction: request.instruction,
      instructionHash: hash,
      createdAt: (body['期限']! as int) - 900,
      expiresAt: body['期限']! as int,
    );
  }

  Future<List<MobileAgentTaskDraft>> desktopInbox() async {
    final body = await _acceptedBody('AgentTask端末申請一覧', const {});
    final rows = body['申請'];
    if (body.length != 1 || rows is! List || rows.length > 64) _invalid();
    final result =
        rows.map(MobileAgentTaskDraft.fromInbox).toList(growable: false);
    if (result.map((draft) => draft.requestId).toSet().length !=
        result.length) {
      _invalid();
    }
    return result;
  }

  Future<List<MobileAgentTaskDraft>> ownRequests() async {
    final body = await _acceptedBody('AgentTask端末申請一覧', const {});
    final rows = body['申請'];
    if (body.length != 1 || rows is! List || rows.length > 16) _invalid();
    final result =
        rows.map(MobileAgentTaskDraft.fromOwnRequest).toList(growable: false);
    if (result.map((draft) => draft.requestId).toSet().length !=
        result.length) {
      _invalid();
    }
    return result;
  }

  Future<MobileAgentTaskStatus> status(MobileAgentTaskDraft draft) async {
    _validateRequestId(draft.requestId);
    final body = await _acceptedBody('AgentTask端末状態', {
      '申請ID': draft.requestId,
    });
    if (body['申請ID'] != draft.requestId ||
        !_mobileTaskStates.contains(body['状態'])) {
      _invalid();
    }
    if (body['状態'] == 'Task状態') {
      if (!_exactKeys(body, const {'申請ID', '状態', 'Task'})) _invalid();
      final taskBody = _stringMap(body['Task'], 'Agent Task状態');
      final task = _parseTask(taskBody, expectedTaskId: null);
      if (!_matchesDraft(task, draft)) {
        _invalid();
      }
      return MobileAgentTaskStatus(
        requestId: draft.requestId,
        state: 'Task状態',
        task: task,
      );
    }
    if (body.length != 2 ||
        !_exactKeys(body, const {'申請ID', '状態'}) ||
        !const {'Owner確認待ち', '開始候補'}.contains(body['状態'])) {
      _invalid();
    }
    return MobileAgentTaskStatus(
      requestId: draft.requestId,
      state: body['状態']! as String,
    );
  }

  Future<MobileAgentTaskRecord> start(MobileAgentTaskDraft draft) async {
    _validateRequestId(draft.requestId);
    final body = await _acceptedBody('AgentTask端末実行', {
      '申請ID': draft.requestId,
    });
    if (body.length != 3 ||
        body['申請ID'] != draft.requestId ||
        body['状態'] != 'Task開始済み') {
      _invalid();
    }
    final task = _parseTask(_stringMap(body['Task'], 'Agent Task開始応答'));
    if (!_matchesDraft(task, draft)) _invalid();
    return task;
  }

  Future<MobileAgentTaskRecord> cancel(MobileAgentTaskDraft draft) async {
    _validateRequestId(draft.requestId);
    final body = await _acceptedBody('AgentTask端末取消', {
      '申請ID': draft.requestId,
    });
    if (body.length != 3 ||
        body['申請ID'] != draft.requestId ||
        body['状態'] != 'Task取消要求済み') {
      _invalid();
    }
    final task = _parseTask(_stringMap(body['Task'], 'Agent Task取消応答'));
    if (!_matchesDraft(task, draft)) _invalid();
    return task;
  }

  Future<Map<String, Object?>> _acceptedBody(
    String operation,
    Map<String, Object?> payload, {
    bool distinguishDefinitiveRejection = false,
  }) async {
    final response = await _request(operation, payload);
    final auditId = response['audit_event_id'];
    if (response['operation'] != operation ||
        auditId is! String ||
        auditId.isEmpty ||
        auditId.length > 256) {
      _invalid();
    }
    final error = response['error'];
    if (distinguishDefinitiveRejection &&
        response['status'] == 'rejected' &&
        error is Map &&
        error.length == 1 &&
        error['code'] is String &&
        (error['code'] as String).isNotEmpty) {
      throw const MobileAgentTaskSubmissionRejected();
    }
    if (response['status'] != 'accepted' || error != null) {
      throw const BrokerClientException('端末Agent Task操作がBrokerで拒否されました');
    }
    if (response['body'] is! Map) _invalid();
    return _stringMap(response['body'], 'Broker応答');
  }

  static MobileAgentTaskRecord _parseTask(
    Map<String, Object?> body, {
    String? expectedTaskId,
  }) {
    const keys = {
      'task_id',
      '実行系ID',
      '対話セッションID',
      '作業領域ID',
      '指示hash',
      '状態',
      '結果hash',
      '結果本文あり',
    };
    if (!_exactKeys(body, keys)) {
      _invalid();
    }
    final taskId = body['task_id'];
    final runtime = body['実行系ID'];
    final session = body['対話セッションID'];
    final workspace = body['作業領域ID'];
    final instructionHash = body['指示hash'];
    final status = body['状態'];
    final resultHash = body['結果hash'];
    final contentAvailable = body['結果本文あり'];
    if (taskId is! String ||
        !RegExp(_requestIdPattern).hasMatch(taskId) ||
        (expectedTaskId != null && taskId != expectedTaskId) ||
        runtime is! String ||
        !RegExp(_safeIdentifierPattern).hasMatch(runtime) ||
        session is! String ||
        !RegExp(_requestIdPattern).hasMatch(session) ||
        workspace is! String ||
        !RegExp(_safeIdentifierPattern).hasMatch(workspace) ||
        instructionHash is! String ||
        !RegExp(_hashPattern).hasMatch(instructionHash) ||
        status is! String ||
        !const {
          'pending',
          'running',
          'blocked',
          'completed',
          'failed',
          'cancelled',
          'quarantined',
        }.contains(status) ||
        (resultHash != null &&
            (resultHash is! String ||
                !RegExp(_hashPattern).hasMatch(resultHash))) ||
        contentAvailable is! bool) {
      _invalid();
    }
    return MobileAgentTaskRecord(
      taskId: taskId,
      runtimeId: runtime,
      sessionId: session,
      workspaceId: workspace,
      instructionHash: instructionHash,
      status: status,
      resultHash: resultHash as String?,
      resultContentAvailable: contentAvailable,
    );
  }

  static bool _matchesDraft(
    MobileAgentTaskRecord task,
    MobileAgentTaskDraft draft,
  ) =>
      task.runtimeId == draft.runtimeId &&
      task.sessionId == draft.sessionId &&
      task.workspaceId == draft.workspaceId &&
      (draft.instructionHash == null ||
          task.instructionHash == draft.instructionHash);

  static void _validateRequestId(String requestId) {
    if (!RegExp(_requestIdPattern).hasMatch(requestId)) _invalid();
  }
}

/// Brokerが申請を明示的に拒否した応答。
/// 通信失敗・応答不明・suspendedとは区別し、送信元Sessionを安全に閉じられる。
class MobileAgentTaskSubmissionRejected extends BrokerClientException {
  const MobileAgentTaskSubmissionRejected()
      : super('端末Agent Task申請はBrokerに拒否されました');
}

bool _exactKeys(Map<String, Object?> value, Set<String> keys) =>
    value.length == keys.length && keys.every(value.containsKey);
Map<String, Object?> _stringMap(Object? raw, String label) {
  if (raw is! Map || raw.keys.any((key) => key is! String)) _invalid();
  return Map<String, Object?>.from(raw);
}

Never _invalid() => throw const BrokerClientException('端末Agent Task契約の形式が不正です');
