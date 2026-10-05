import 'agent_task_client.dart';
import 'broker_client.dart' show BrokerClientException;
import '../models/generated_contracts.dart';

typedef AgentTaskStarter = Future<AgentTaskRecord> Function(
  AgentTaskRequest request,
);

class AgentComparisonTaskOutcome {
  const AgentComparisonTaskOutcome({this.record, this.error});

  final AgentTaskRecord? record;
  final Object? error;
}

/// 独立した二つのAgent Taskを同時に開始し、各側の失敗を分離して保持する。
Future<List<AgentComparisonTaskOutcome>> startAgentComparisonTasks({
  required AgentTaskStarter start,
  required AgentTaskRequest agentA,
  required AgentTaskRequest agentB,
}) async {
  agentA.validate();
  agentB.validate();
  if (agentA.instruction != agentB.instruction ||
      agentA.runtimeId == agentB.runtimeId ||
      agentA.sessionId == agentB.sessionId ||
      agentA.workspaceId == agentB.workspaceId) {
    throw const BrokerClientException(
      'Compare要求は同一Task本文と独立したRuntime／Session／Workspaceが必要です',
    );
  }

  Future<AgentComparisonTaskOutcome> startIndependently(
    AgentTaskRequest request,
  ) async {
    try {
      return AgentComparisonTaskOutcome(record: await start(request));
    } on Object catch (error) {
      return AgentComparisonTaskOutcome(error: error);
    }
  }

  return Future.wait([
    startIndependently(agentA),
    startIndependently(agentB),
  ]);
}

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
      statusMessage: 'Agent runtime IDとWorkspace参照が重複していない公開投影です。実行時の隔離は未検証です。',
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
