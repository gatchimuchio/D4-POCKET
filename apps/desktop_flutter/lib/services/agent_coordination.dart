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
    final sessionIds = sessions.map((session) => session.sessionId).toList();
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
      statusMessage: '独立Workspaceの公開投影を比較できます。権限・承認・秘密値は共有しません。',
      sessionIds: sessionIds,
    );
  }
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
