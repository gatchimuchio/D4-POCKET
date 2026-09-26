import '../models/generated_contracts.dart';

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
    if (agentRuntimeIds.toSet().length != agentRuntimeIds.length) {
      return AgentComparisonProjection(
        available: false,
        statusMessage: '同一Agent実行系の重複を検出したため比較を停止しました。',
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
      statusMessage: 'Workspace参照が重複していない公開投影です。実行時の隔離は未検証です。',
      sessionIds: sessionIds,
    );
  }
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

class AgentHandoffProjection {
  const AgentHandoffProjection({
    required this.sessionId,
    required this.taskSummary,
    required this.diffSummary,
    required this.testStatus,
    required this.statusMessage,
  });

  final String sessionId;
  final String taskSummary;
  final String diffSummary;
  final String testStatus;
  final String statusMessage;

  factory AgentHandoffProjection.fromSession(AgentSessionRecord session) {
    return AgentHandoffProjection(
      sessionId: session.sessionId,
      taskSummary: _publicSummary(session.task),
      diffSummary: _publicSummary(session.diffSummary),
      testStatus: _publicSummary(session.testStatus),
      statusMessage: '公開概要だけを渡し、target条件で権限・承認を再評価します。',
    );
  }
}

String _publicSummary(String value) {
  final redacted = value.replaceAll(
    RegExp(
      r'(?:secret|password|token|credential|api[_ -]?key|private[_ -]?key)\s*[:=]\s*[^\s,;]+',
      caseSensitive: false,
    ),
    '[redacted]',
  );
  if (redacted.length <= 256) {
    return redacted;
  }
  return '${redacted.substring(0, 256)}…';
}
