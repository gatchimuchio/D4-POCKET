import '../models/generated_contracts.dart';
import 'package:agent_orchestration/agent_task_client.dart'
    show AgentTaskRecord, AgentTaskResultProjection;

class AgentComparisonProjection {
  const AgentComparisonProjection({
    required this.available,
    required this.statusMessage,
    required this.sessionIds,
  });

  final bool available;
  final String statusMessage;
  final List<String> sessionIds;

  factory AgentComparisonProjection.fromSessions(
    List<AgentSessionRecord> sessions,
  ) {
    if (sessions.length < 2) {
      return const AgentComparisonProjection(
        available: false,
        statusMessage: '比較対象が2件未満です。実Agent比較は未接続です。',
        sessionIds: [],
      );
    }
    if (sessions.length > 8) {
      return const AgentComparisonProjection(
        available: false,
        statusMessage: '比較対象が上限の8件を超えたため停止しました。',
        sessionIds: [],
      );
    }
    final sessionIds = sessions.map((session) => session.sessionId).toList();
    final agentRuntimeIds =
        sessions.map((session) => session.agentRuntimeId).toList();
    if (sessionIds.any((id) => !_isComparisonId(id))) {
      return const AgentComparisonProjection(
        available: false,
        statusMessage: 'セッション識別子を検証できないため比較を停止しました。',
        sessionIds: [],
      );
    }
    if (agentRuntimeIds.any((id) => !_isComparisonId(id))) {
      return const AgentComparisonProjection(
        available: false,
        statusMessage: 'Agent実行系識別子を検証できないため比較を停止しました。',
        sessionIds: [],
      );
    }
    if (sessions.any((session) => session.workspace.trim().isEmpty)) {
      return const AgentComparisonProjection(
        available: false,
        statusMessage: 'Workspace参照が不足しているため比較を停止しました。',
        sessionIds: [],
      );
    }
    final workspaces = sessions.map((session) => session.workspace).toSet();
    if (sessionIds.toSet().length != sessionIds.length) {
      return AgentComparisonProjection(
        available: false,
        statusMessage: '同一セッションの重複を検出したため比較を停止しました。',
        sessionIds: sessionIds,
      );
    }
    if (workspaces.length != sessions.length) {
      return AgentComparisonProjection(
        available: false,
        statusMessage: '同一Workspaceの混在を検出したため比較を停止しました。',
        sessionIds: sessionIds,
      );
    }
    return AgentComparisonProjection(
      available: true,
      statusMessage:
          'SessionとWorkspace参照は独立しています。同じRuntime IDも比較できます。実行時の隔離は未検証です。',
      sessionIds: sessionIds,
    );
  }
}

/// Compare選択結果の適用元と適用先を固定する不変snapshot。
///
/// 未信頼の結果本文はデータとしてのみ保持し、Permission／Approval等の
/// Authorityは保持しない。
class AgentSelectedResultApplySnapshot {
  AgentSelectedResultApplySnapshot.capture({
    required this.agentASessionId,
    required this.agentBSessionId,
    required this.selectedSessionId,
    required this.comparisonInstructionHash,
    required this.comparisonStartedAt,
    required AgentSessionRecord agentA,
    required AgentSessionRecord agentB,
    required AgentSessionRecord source,
    required AgentSessionRecord target,
    required AgentTaskRecord sourceTask,
    required AgentTaskResultProjection sourceResult,
    required AgentTaskRecord? targetTask,
  })  : _agentA = _ComparisonApplySessionBinding(agentA),
        _agentB = _ComparisonApplySessionBinding(agentB),
        _source = _ComparisonApplySessionBinding(source),
        _target = _ComparisonApplySessionBinding(target),
        _sourceTask = _ComparisonApplyTaskBinding(sourceTask),
        _sourceResult = _ComparisonApplyResultBinding(sourceResult),
        _targetTask =
            targetTask == null ? null : _ComparisonApplyTaskBinding(targetTask);

  final String agentASessionId;
  final String agentBSessionId;
  final String selectedSessionId;
  final String comparisonInstructionHash;
  final DateTime? comparisonStartedAt;
  final _ComparisonApplySessionBinding _agentA;
  final _ComparisonApplySessionBinding _agentB;
  final _ComparisonApplySessionBinding _source;
  final _ComparisonApplySessionBinding _target;
  final _ComparisonApplyTaskBinding _sourceTask;
  final _ComparisonApplyResultBinding _sourceResult;
  final _ComparisonApplyTaskBinding? _targetTask;

  String get sourceRuntimeId => _source.agentRuntimeId;
  String get sourceSessionId => _source.sessionId;
  String get sourceWorkspace => _source.workspace;
  String get sourceTaskId => _sourceTask.taskId;
  String? get resultHash => _sourceResult.resultHash;
  String? get resultText => _sourceResult.text;
  String get targetRuntimeId => _target.agentRuntimeId;
  String get targetSessionId => _target.sessionId;
  String get targetWorkspace => _target.workspace;

  bool matchesCurrent({
    required String? currentAgentASessionId,
    required String? currentAgentBSessionId,
    required String? currentSelectedSessionId,
    required String? currentComparisonInstructionHash,
    required DateTime? currentComparisonStartedAt,
    required List<AgentSessionRecord> currentSessions,
    required AgentTaskRecord? currentSourceTask,
    required AgentTaskResultProjection? currentSourceResult,
    required AgentTaskRecord? currentTargetTask,
    required bool currentTargetHasRequest,
  }) {
    if (currentAgentASessionId != agentASessionId ||
        currentAgentBSessionId != agentBSessionId ||
        currentSelectedSessionId != selectedSessionId ||
        currentComparisonInstructionHash != comparisonInstructionHash ||
        currentComparisonStartedAt != comparisonStartedAt ||
        selectedSessionId != sourceSessionId ||
        (selectedSessionId != agentASessionId &&
            selectedSessionId != agentBSessionId) ||
        currentTargetHasRequest) {
      return false;
    }

    final agentA = _uniqueCurrentSession(currentSessions, _agentA);
    final agentB = _uniqueCurrentSession(currentSessions, _agentB);
    final source = _uniqueCurrentSession(currentSessions, _source);
    final target = _uniqueCurrentSession(currentSessions, _target);
    if (agentA == null || agentB == null || source == null || target == null) {
      return false;
    }

    final expectedTargetTask = _targetTask;
    final targetTaskMatches = expectedTargetTask == null
        ? currentTargetTask == null
        : expectedTargetTask.matches(currentTargetTask);
    if (!_sourceTask.matches(currentSourceTask) ||
        !_sourceResult.matches(currentSourceResult) ||
        !targetTaskMatches ||
        !_sourceTask.isCompletedFor(_source) ||
        _sourceTask.taskId != _sourceResult.taskId ||
        _sourceTask.resultHash == null ||
        _sourceTask.resultHash != _sourceResult.resultHash ||
        _sourceResult.visibility != 'full' ||
        _sourceResult.text == null ||
        (expectedTargetTask != null &&
            !expectedTargetTask.isBoundTo(_target))) {
      return false;
    }

    return AgentComparisonProjection.fromSessions([
      agentA,
      agentB,
      target,
    ]).available;
  }
}

/// AI Editの完了結果を独立Workspaceへ渡す直前の不変snapshot。
///
/// 提案由来情報・結果本文は由来照合用の未信頼データであり、Authorityを
/// 保持しない。適用先Taskは通常のBroker事前検査と新規認可を通る。
class AgentAiEditResultApplySnapshot {
  AgentAiEditResultApplySnapshot.capture({
    required AgentSessionRecord source,
    required AgentSessionRecord target,
    required AgentTaskRecord sourceTask,
    required AgentTaskResultProjection sourceResult,
    required this.aiEditTaskId,
    required this.preflightInstructionHash,
    required this.proposalEditId,
    required this.proposalPayloadHash,
    required this.proposalAuditId,
  })  : _source = _AiEditApplySessionBinding(source),
        _target = _AiEditApplySessionBinding(target),
        _sourceTask = _AiEditApplyTaskBinding(sourceTask),
        _sourceResult = _AiEditApplyResultBinding(sourceResult);

  final String aiEditTaskId;
  final String preflightInstructionHash;
  final String proposalEditId;
  final String proposalPayloadHash;
  final String proposalAuditId;
  final _AiEditApplySessionBinding _source;
  final _AiEditApplySessionBinding _target;
  final _AiEditApplyTaskBinding _sourceTask;
  final _AiEditApplyResultBinding _sourceResult;

  String get sourceRuntimeId => _source.runtimeId;
  String get sourceSessionId => _source.sessionId;
  String get sourceWorkspace => _source.workspace;
  String get sourceTaskId => _sourceTask.taskId;
  String get resultHash => _sourceResult.resultHash;
  String? get resultText => _sourceResult.text;
  String get targetRuntimeId => _target.runtimeId;
  String get targetSessionId => _target.sessionId;
  String get targetWorkspace => _target.workspace;

  bool matchesCurrent({
    required List<AgentSessionRecord> currentSessions,
    required AgentTaskRecord? currentSourceTask,
    required AgentTaskResultProjection? currentSourceResult,
    required AgentTaskRecord? currentTargetTask,
    required bool currentTargetHasRequest,
    required String? currentAiEditTaskId,
    required String? currentPreflightInstructionHash,
    required String? currentProposalEditId,
    required String? currentProposalPayloadHash,
    required String? currentProposalAuditId,
  }) {
    if (currentTargetHasRequest ||
        currentTargetTask != null ||
        currentAiEditTaskId != aiEditTaskId ||
        currentPreflightInstructionHash != preflightInstructionHash ||
        currentProposalEditId != proposalEditId ||
        currentProposalPayloadHash != proposalPayloadHash ||
        currentProposalAuditId != proposalAuditId ||
        aiEditTaskId != _sourceTask.taskId ||
        preflightInstructionHash != _sourceTask.instructionHash ||
        proposalEditId.isEmpty ||
        proposalPayloadHash.isEmpty ||
        proposalAuditId.isEmpty) {
      return false;
    }

    final source = _uniqueAiEditApplySession(currentSessions, _source);
    final target = _uniqueAiEditApplySession(currentSessions, _target);
    if (source == null ||
        target == null ||
        !_sourceTask.matches(currentSourceTask) ||
        !_sourceResult.matches(currentSourceResult) ||
        !_sourceTask.isCompletedFor(_source) ||
        _sourceTask.taskId != _sourceResult.taskId ||
        _sourceTask.resultHash != _sourceResult.resultHash ||
        !_sourceTask.resultContentAvailable ||
        _sourceResult.visibility != 'full' ||
        _sourceResult.text == null ||
        _sourceResult.text!.isEmpty ||
        _source.sessionId == _target.sessionId ||
        _source.workspace == _target.workspace) {
      return false;
    }

    return AgentComparisonProjection.fromSessions([source, target]).available;
  }
}

AgentSessionRecord? _uniqueAiEditApplySession(
  List<AgentSessionRecord> sessions,
  _AiEditApplySessionBinding expected,
) {
  final matches = sessions
      .where((session) => session.sessionId == expected.sessionId)
      .toList(growable: false);
  if (matches.length != 1 || !expected.matches(matches.single)) return null;
  return matches.single;
}

class _AiEditApplySessionBinding {
  _AiEditApplySessionBinding(AgentSessionRecord session)
      : sessionId = session.sessionId,
        runtimeId = session.agentRuntimeId,
        status = session.status,
        workspace = session.workspace,
        workspaceAuditEventId = session.workspaceAuditEventId;

  final String sessionId;
  final String runtimeId;
  final String status;
  final String workspace;
  final String workspaceAuditEventId;

  bool matches(AgentSessionRecord session) =>
      status == '利用中' &&
      session.sessionId == sessionId &&
      session.agentRuntimeId == runtimeId &&
      session.status == status &&
      session.workspace == workspace &&
      session.workspaceAuditEventId == workspaceAuditEventId;
}

class _AiEditApplyTaskBinding {
  _AiEditApplyTaskBinding(AgentTaskRecord task)
      : taskId = task.taskId,
        runtimeId = task.runtimeId,
        sessionId = task.sessionId,
        workspaceId = task.workspaceId,
        instructionHash = task.instructionHash,
        status = task.status,
        auditEventId = task.auditEventId,
        resultHash = task.resultHash,
        resultContentAvailable = task.resultContentAvailable;

  final String taskId;
  final String runtimeId;
  final String sessionId;
  final String workspaceId;
  final String instructionHash;
  final String status;
  final String auditEventId;
  final String? resultHash;
  final bool resultContentAvailable;

  bool matches(AgentTaskRecord? task) =>
      task != null &&
      task.taskId == taskId &&
      task.runtimeId == runtimeId &&
      task.sessionId == sessionId &&
      task.workspaceId == workspaceId &&
      task.instructionHash == instructionHash &&
      task.status == status &&
      task.auditEventId == auditEventId &&
      task.resultHash == resultHash &&
      task.resultContentAvailable == resultContentAvailable;

  bool isCompletedFor(_AiEditApplySessionBinding session) =>
      status == 'completed' &&
      runtimeId == session.runtimeId &&
      sessionId == session.sessionId &&
      workspaceId == session.workspace;
}

class _AiEditApplyResultBinding {
  _AiEditApplyResultBinding(AgentTaskResultProjection result)
      : taskId = result.taskId,
        resultHash = result.resultHash,
        visibility = result.visibility,
        text = result.text;

  final String taskId;
  final String resultHash;
  final String visibility;
  final String? text;

  bool matches(AgentTaskResultProjection? result) =>
      result != null &&
      result.taskId == taskId &&
      result.resultHash == resultHash &&
      result.visibility == visibility &&
      result.text == text;
}

AgentSessionRecord? _uniqueCurrentSession(
  List<AgentSessionRecord> sessions,
  _ComparisonApplySessionBinding expected,
) {
  final matches = sessions
      .where((session) => session.sessionId == expected.sessionId)
      .toList(growable: false);
  if (matches.length != 1 || !expected.matches(matches.single)) return null;
  return matches.single;
}

class _ComparisonApplySessionBinding {
  _ComparisonApplySessionBinding(AgentSessionRecord session)
      : sessionId = session.sessionId,
        agentRuntimeId = session.agentRuntimeId,
        status = session.status,
        workspace = session.workspace,
        workspaceAuditEventId = session.workspaceAuditEventId;

  final String sessionId;
  final String agentRuntimeId;
  final String status;
  final String workspace;
  final String workspaceAuditEventId;

  bool matches(AgentSessionRecord session) =>
      status == '利用中' &&
      session.sessionId == sessionId &&
      session.agentRuntimeId == agentRuntimeId &&
      session.status == status &&
      session.workspace == workspace &&
      session.workspaceAuditEventId == workspaceAuditEventId;
}

class _ComparisonApplyTaskBinding {
  _ComparisonApplyTaskBinding(AgentTaskRecord task)
      : taskId = task.taskId,
        runtimeId = task.runtimeId,
        sessionId = task.sessionId,
        workspaceId = task.workspaceId,
        instructionHash = task.instructionHash,
        status = task.status,
        auditEventId = task.auditEventId,
        resultHash = task.resultHash,
        resultContentAvailable = task.resultContentAvailable;

  final String taskId;
  final String runtimeId;
  final String sessionId;
  final String workspaceId;
  final String instructionHash;
  final String status;
  final String auditEventId;
  final String? resultHash;
  final bool resultContentAvailable;

  bool matches(AgentTaskRecord? task) =>
      task != null &&
      task.taskId == taskId &&
      task.runtimeId == runtimeId &&
      task.sessionId == sessionId &&
      task.workspaceId == workspaceId &&
      task.instructionHash == instructionHash &&
      task.status == status &&
      task.auditEventId == auditEventId &&
      task.resultHash == resultHash &&
      task.resultContentAvailable == resultContentAvailable;

  bool isCompletedFor(_ComparisonApplySessionBinding session) =>
      status == 'completed' &&
      runtimeId == session.agentRuntimeId &&
      sessionId == session.sessionId &&
      workspaceId == session.workspace;

  bool isBoundTo(_ComparisonApplySessionBinding session) =>
      runtimeId == session.agentRuntimeId &&
      sessionId == session.sessionId &&
      workspaceId == session.workspace;
}

class _ComparisonApplyResultBinding {
  _ComparisonApplyResultBinding(AgentTaskResultProjection result)
      : taskId = result.taskId,
        resultHash = result.resultHash,
        visibility = result.visibility,
        text = result.text;

  final String taskId;
  final String resultHash;
  final String visibility;
  final String? text;

  bool matches(AgentTaskResultProjection? result) =>
      result != null &&
      result.taskId == taskId &&
      result.resultHash == resultHash &&
      result.visibility == visibility &&
      result.text == text;
}

bool _isComparisonId(String value) {
  if (value.isEmpty || value.length > 256) {
    return false;
  }
  if (!_isAsciiAlphaNumeric(value.codeUnitAt(0))) {
    return false;
  }
  for (var index = 1; index < value.length; index++) {
    final codeUnit = value.codeUnitAt(index);
    if (!_isAsciiAlphaNumeric(codeUnit) &&
        codeUnit != 0x5f &&
        codeUnit != 0x2e &&
        codeUnit != 0x3a &&
        codeUnit != 0x2d) {
      return false;
    }
  }
  return true;
}

bool _isAsciiAlphaNumeric(int codeUnit) {
  return (codeUnit >= 0x30 && codeUnit <= 0x39) ||
      (codeUnit >= 0x41 && codeUnit <= 0x5a) ||
      (codeUnit >= 0x61 && codeUnit <= 0x7a);
}
