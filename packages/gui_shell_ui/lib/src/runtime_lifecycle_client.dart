import 'broker_transport.dart';

const _stateOperation = '実行系ライフサイクル状態';
const _approvalRequestOperation = '実行系ライフサイクル承認要求';
const _executeOperation = '実行系ライフサイクル操作';

const _actions = <String>{
  'start',
  'stop',
  'restart',
  'pause',
  'resume',
  'quarantine',
};

const _states = <String>{
  'stopped',
  'ready',
  'paused',
  'quarantined',
  'unknown',
  'not_supported',
};

final _runtimeId = RegExp(r'^[A-Za-z0-9][A-Za-z0-9_.-]{0,127}$');
final _identifier = RegExp(r'^[A-Za-z0-9][A-Za-z0-9_.-]{0,255}$');
final _hash = RegExp(r'^sha256:[a-f0-9]{64}$');

/// Brokerが所有する実行系ライフサイクル状態を表示し、承認要求または
/// 承認済みの一回限りの遷移を依頼するclient。
///
/// FlutterはCapability、Permission、Approval、Audit、Recoveryを発行・編集
/// しない。PID、endpoint、command、argv、環境変数も受け取らない。
class RuntimeLifecycleClient {
  RuntimeLifecycleClient(this.transport);

  final BrokerTransport transport;

  static bool validRuntimeId(Object? value) =>
      value is String && _runtimeId.hasMatch(value);

  static bool validAction(Object? value) =>
      value is String && _actions.contains(value);

  Future<RuntimeLifecycleStatus> status(String runtimeId) async {
    if (!validRuntimeId(runtimeId)) _reject();
    final response = await _accepted(
      _stateOperation,
      {'版': 1, '実行系ID': runtimeId},
    );
    return RuntimeLifecycleStatus._parse(
      response.body,
      runtimeId,
      response.evidenceSource,
    );
  }

  Future<RuntimeLifecycleApproval> requestApproval(
    String runtimeId,
    String action,
  ) async {
    if (!validRuntimeId(runtimeId) || !validAction(action)) _reject();
    final response = await _accepted(
      _approvalRequestOperation,
      {'版': 1, '実行系ID': runtimeId, '操作': action},
    );
    if (response.evidenceSource != 'INTERNAL_STATE') _reject();
    return RuntimeLifecycleApproval._parse(
      response.body,
      runtimeId: runtimeId,
      action: action,
      requiredState: 'pending',
    );
  }

  Future<RuntimeLifecycleTransition> execute(
    String runtimeId,
    String action,
    String approvalId,
  ) async {
    if (!validRuntimeId(runtimeId) ||
        !validAction(action) ||
        !_identifier.hasMatch(approvalId)) {
      _reject();
    }
    final response = await _accepted(
      _executeOperation,
      {
        '版': 1,
        '実行系ID': runtimeId,
        '操作': action,
        '承認ID': approvalId,
      },
    );
    if (response.evidenceSource != 'LIVE_RUNTIME') _reject();
    return RuntimeLifecycleTransition._parse(
      response.body,
      runtimeId: runtimeId,
      action: action,
      responseAuditId: response.auditId,
    );
  }

  Future<_AcceptedEnvelope> _accepted(
    String operation,
    Map<String, Object?> payload,
  ) async {
    final raw = await transport.request(operation, payload: payload);
    final response = _object(raw);
    _exactKeys(response, const {
      'request_id',
      'operation',
      'status',
      'evidence_source',
      'audit_event_id',
      'error',
      'health',
      'body',
      'shutdown_requested',
    });
    final auditId = response['audit_event_id'];
    final evidenceSource = response['evidence_source'];
    if (response['request_id'] is! String ||
        !_identifier.hasMatch(response['request_id'] as String) ||
        response['operation'] != operation ||
        response['status'] != 'accepted' ||
        auditId is! String ||
        !_identifier.hasMatch(auditId) ||
        !{'LIVE_RUNTIME', 'INTERNAL_STATE'}.contains(evidenceSource) ||
        response['error'] != null ||
        response['health'] != null ||
        response['shutdown_requested'] != false) {
      _reject();
    }
    return _AcceptedEnvelope(
      _object(response['body']),
      auditId,
      evidenceSource as String,
    );
  }
}

class _AcceptedEnvelope {
  const _AcceptedEnvelope(this.body, this.auditId, this.evidenceSource);

  final Map<String, Object?> body;
  final String auditId;
  final String evidenceSource;
}

/// Brokerが返す現在のライフサイクル状態の表示射影。
class RuntimeLifecycleStatus {
  RuntimeLifecycleStatus._(
    this.runtimeId,
    this.supported,
    this.state,
    this.evidenceSource,
    List<RuntimeLifecycleOperation> operations,
    List<RuntimeLifecycleApproval> approvals,
  )   : operations = List.unmodifiable(operations),
        approvals = List.unmodifiable(approvals);

  final String runtimeId;
  final bool supported;
  final String state;
  final String evidenceSource;
  final List<RuntimeLifecycleOperation> operations;
  final List<RuntimeLifecycleApproval> approvals;

  RuntimeLifecycleOperation? operation(String action) {
    for (final item in operations) {
      if (item.action == action) return item;
    }
    return null;
  }

  RuntimeLifecycleApproval? approvedFor(String action) {
    final now = DateTime.now().toUtc().millisecondsSinceEpoch ~/ 1000;
    for (final approval in approvals) {
      if (approval.action == action &&
          approval.state == 'approved' &&
          approval.expiresAtUnixSeconds > now) {
        return approval;
      }
    }
    return null;
  }

  static RuntimeLifecycleStatus _parse(
    Map<String, Object?> body,
    String requestedRuntimeId,
    String responseEvidenceSource,
  ) {
    _exactKeys(body, const {
      '版',
      '実行系ID',
      '対応',
      '状態',
      '証拠種別',
      '操作一覧',
      '承認一覧',
    });
    final supported = body['対応'];
    final state = body['状態'];
    final evidenceSource = body['証拠種別'];
    final rawOperations = body['操作一覧'];
    final rawApprovals = body['承認一覧'];
    if (body['版'] != 1 ||
        body['実行系ID'] != requestedRuntimeId ||
        supported is! bool ||
        state is! String ||
        !_states.contains(state) ||
        evidenceSource is! String ||
        !{'LIVE_RUNTIME', 'INTERNAL_STATE'}.contains(evidenceSource) ||
        evidenceSource != responseEvidenceSource ||
        rawOperations is! List ||
        rawApprovals is! List ||
        rawOperations.length > _actions.length ||
        rawApprovals.length > _actions.length) {
      _reject();
    }
    final operations = rawOperations
        .map(RuntimeLifecycleOperation._parse)
        .toList(growable: false);
    final approvals = rawApprovals
        .map((item) => RuntimeLifecycleApproval._parse(
              _object(item),
              requiredState: null,
            ))
        .toList(growable: false);
    if (operations.map((item) => item.action).toSet().length !=
            operations.length ||
        approvals.map((item) => item.approvalId).toSet().length !=
            approvals.length) {
      _reject();
    }
    if (!supported) {
      if (state != 'not_supported' ||
          evidenceSource != 'INTERNAL_STATE' ||
          operations.isNotEmpty ||
          approvals.isNotEmpty) {
        _reject();
      }
    } else if (state == 'not_supported' ||
        (operations.isEmpty && state != 'quarantined')) {
      _reject();
    }
    if (state == 'quarantined' &&
        (operations.isNotEmpty || approvals.isNotEmpty)) {
      _reject();
    }
    for (final approval in approvals) {
      final operation =
          operations.where((item) => item.action == approval.action);
      if (operation.length != 1) _reject();
    }
    return RuntimeLifecycleStatus._(
      requestedRuntimeId,
      supported,
      state,
      evidenceSource,
      operations,
      approvals,
    );
  }
}

/// Adapterが宣言し、Brokerが再照合した一つの閉じた操作。
class RuntimeLifecycleOperation {
  const RuntimeLifecycleOperation._(
    this.action,
    this.capabilityId,
    this.permissionId,
    this.requiresApproval,
    this.recoveryId,
    this.executable,
  );

  final String action;
  final String capabilityId;
  final String permissionId;
  final bool requiresApproval;
  final String recoveryId;
  final bool executable;

  static RuntimeLifecycleOperation _parse(Object? raw) {
    final value = _object(raw);
    _exactKeys(value, const {
      '操作',
      '能力ID',
      '権限ID',
      '承認必要',
      '復旧ID',
      '実行可能',
    });
    final action = value['操作'];
    if (!RuntimeLifecycleClient.validAction(action) ||
        value['能力ID'] != 'runtime.lifecycle.$action' ||
        value['権限ID'] != 'permission.runtime.lifecycle.$action' ||
        value['承認必要'] != true ||
        value['復旧ID'] != 'recover-runtime-lifecycle-$action' ||
        value['実行可能'] is! bool) {
      _reject();
    }
    return RuntimeLifecycleOperation._(
      action as String,
      value['能力ID'] as String,
      value['権限ID'] as String,
      true,
      value['復旧ID'] as String,
      value['実行可能'] as bool,
    );
  }
}

/// Brokerに保管された短期承認の安全な表示射影。
class RuntimeLifecycleApproval {
  const RuntimeLifecycleApproval._(
    this.approvalId,
    this.approvalHash,
    this.runtimeId,
    this.action,
    this.state,
    this.expiresAtUnixSeconds,
    this.governance,
  );

  final String approvalId;
  final String approvalHash;
  final String runtimeId;
  final String action;
  final String state;
  final int expiresAtUnixSeconds;
  final RuntimeLifecycleGovernance governance;

  static RuntimeLifecycleApproval _parse(
    Map<String, Object?> value, {
    String? runtimeId,
    String? action,
    String? requiredState,
  }) {
    _exactKeys(value, const {
      '版',
      '承認ID',
      '承認hash',
      '実行系ID',
      '操作',
      '状態',
      '有効期限UnixSeconds',
      '統治',
    });
    final approvalId = value['承認ID'];
    final approvalHash = value['承認hash'];
    final parsedRuntimeId = value['実行系ID'];
    final parsedAction = value['操作'];
    final state = value['状態'];
    final expires = value['有効期限UnixSeconds'];
    if (value['版'] != 1 ||
        approvalId is! String ||
        !_identifier.hasMatch(approvalId) ||
        approvalHash is! String ||
        !_hash.hasMatch(approvalHash) ||
        !RuntimeLifecycleClient.validRuntimeId(parsedRuntimeId) ||
        !RuntimeLifecycleClient.validAction(parsedAction) ||
        !{'pending', 'approved'}.contains(state) ||
        expires is! int ||
        expires <= 0 ||
        (runtimeId != null && parsedRuntimeId != runtimeId) ||
        (action != null && parsedAction != action) ||
        (requiredState != null && state != requiredState)) {
      _reject();
    }
    return RuntimeLifecycleApproval._(
      approvalId,
      approvalHash,
      parsedRuntimeId as String,
      parsedAction as String,
      state as String,
      expires,
      RuntimeLifecycleGovernance._parse(
        value['統治'],
        action: parsedAction,
        approvalId: approvalId,
        approvalState: state,
      ),
    );
  }
}

/// Brokerが固定した統治対応。UIは表示だけを行う。
class RuntimeLifecycleGovernance {
  const RuntimeLifecycleGovernance._(
    this.capabilityId,
    this.permissionId,
    this.approvalId,
    this.approvalState,
    this.recoveryId,
  );

  final String capabilityId;
  final String permissionId;
  final String approvalId;
  final String approvalState;
  final String recoveryId;

  static RuntimeLifecycleGovernance _parse(
    Object? raw, {
    required Object? action,
    required Object? approvalId,
    required String approvalState,
  }) {
    final value = _object(raw);
    _exactKeys(value, const {
      '能力ID',
      '権限ID',
      '承認ID',
      '承認状態',
      '復旧ID',
    });
    if (!RuntimeLifecycleClient.validAction(action) ||
        value['能力ID'] != 'runtime.lifecycle.$action' ||
        value['権限ID'] != 'permission.runtime.lifecycle.$action' ||
        value['承認ID'] != approvalId ||
        value['承認状態'] != approvalState ||
        value['復旧ID'] != 'recover-runtime-lifecycle-$action') {
      _reject();
    }
    return RuntimeLifecycleGovernance._(
      value['能力ID'] as String,
      value['権限ID'] as String,
      value['承認ID'] as String,
      value['承認状態'] as String,
      value['復旧ID'] as String,
    );
  }
}

/// 最終監査へ結合された一回限りの実行結果。
class RuntimeLifecycleTransition {
  const RuntimeLifecycleTransition._(
    this.runtimeId,
    this.action,
    this.previousState,
    this.nextState,
    this.observedAtUnixMillis,
    this.evidenceSource,
    this.governance,
    this.auditId,
  );

  final String runtimeId;
  final String action;
  final String previousState;
  final String nextState;
  final int observedAtUnixMillis;
  final String evidenceSource;
  final RuntimeLifecycleGovernance governance;
  final String auditId;

  static RuntimeLifecycleTransition _parse(
    Map<String, Object?> value, {
    required String runtimeId,
    required String action,
    required String responseAuditId,
  }) {
    _exactKeys(value, const {
      '版',
      '実行系ID',
      '操作',
      '遷移前状態',
      '遷移後状態',
      '観測時刻UnixMillis',
      '証拠種別',
      '統治',
      'ライフサイクル監査ID',
    });
    final previousState = value['遷移前状態'];
    final nextState = value['遷移後状態'];
    final observedAt = value['観測時刻UnixMillis'];
    final evidenceSource = value['証拠種別'];
    final auditId = value['ライフサイクル監査ID'];
    if (value['版'] != 1 ||
        value['実行系ID'] != runtimeId ||
        value['操作'] != action ||
        previousState is! String ||
        nextState is! String ||
        !_states.contains(previousState) ||
        !_states.contains(nextState) ||
        observedAt is! int ||
        observedAt < 0 ||
        evidenceSource != 'LIVE_RUNTIME' ||
        auditId != responseAuditId ||
        auditId is! String ||
        !_identifier.hasMatch(auditId)) {
      _reject();
    }
    final governance = RuntimeLifecycleGovernance._parse(
      value['統治'],
      action: action,
      approvalId: _object(value['統治'])['承認ID'],
      approvalState: 'consumed',
    );
    return RuntimeLifecycleTransition._(
      runtimeId,
      action,
      previousState,
      nextState,
      observedAt,
      'LIVE_RUNTIME',
      governance,
      responseAuditId,
    );
  }
}

Map<String, Object?> _object(Object? raw) {
  if (raw is! Map) _reject();
  final result = <String, Object?>{};
  for (final entry in raw.entries) {
    if (entry.key is! String) _reject();
    result[entry.key as String] = entry.value;
  }
  return result;
}

void _exactKeys(Map<String, Object?> value, Set<String> keys) {
  if (value.length != keys.length || !keys.every(value.containsKey)) _reject();
}

Never _reject() => throw const BrokerClientException('実行系ライフサイクルの応答を確認できません');
