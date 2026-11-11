import 'package:broker_contract/broker_transport.dart'
    show BrokerClientException;

import 'agent_task_client.dart' show AgentTaskRecord, AgentTaskRequest;

typedef AgentTaskStarter =
    Future<AgentTaskRecord> Function(AgentTaskRequest request);

/// Compareの取消待機をSession単位で分離し、同じSessionの二重要求だけを防ぐ。
class AgentTaskCancellationGate {
  final Set<String> _pendingSessionIds = <String>{};

  bool get hasPending => _pendingSessionIds.isNotEmpty;

  bool isPending(String sessionId) => _pendingSessionIds.contains(sessionId);

  bool tryBegin(String sessionId) =>
      sessionId.isNotEmpty && _pendingSessionIds.add(sessionId);

  Future<T?> runIfAvailable<T>(
    String sessionId,
    Future<T> Function() action,
  ) async {
    if (!tryBegin(sessionId)) return null;
    try {
      return await action();
    } finally {
      finish(sessionId);
    }
  }

  void finish(String sessionId) {
    _pendingSessionIds.remove(sessionId);
  }
}

class AgentComparisonTaskOutcome {
  const AgentComparisonTaskOutcome({this.record, this.error});

  final AgentTaskRecord? record;
  final Object? error;
}

/// 同一指示を独立したSession／WorkspaceのTaskとして同時に開始する。
/// 同じRuntimeでも、Brokerが別々のTask実行として認可・起動する。
/// 片側の開始失敗は他方へ伝播させず、Authority判断は各Broker要求に委ねる。
Future<List<AgentComparisonTaskOutcome>> startAgentComparisonTasks({
  required AgentTaskStarter start,
  required AgentTaskRequest agentA,
  required AgentTaskRequest agentB,
}) async {
  agentA.validate();
  agentB.validate();
  if (agentA.instruction != agentB.instruction ||
      agentA.sessionId == agentB.sessionId ||
      agentA.workspaceId == agentB.workspaceId) {
    throw const BrokerClientException(
      'Compare要求は同一Task本文と独立したSession／Workspaceが必要です',
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

  return Future.wait([startIndependently(agentA), startIndependently(agentB)]);
}
