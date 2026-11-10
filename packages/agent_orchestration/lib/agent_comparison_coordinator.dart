import 'package:broker_contract/broker_transport.dart'
    show BrokerClientException;

import 'agent_task_client.dart' show AgentTaskRecord, AgentTaskRequest;

typedef AgentTaskStarter =
    Future<AgentTaskRecord> Function(AgentTaskRequest request);

class AgentComparisonTaskOutcome {
  const AgentComparisonTaskOutcome({this.record, this.error});

  final AgentTaskRecord? record;
  final Object? error;
}

/// 同一指示を独立したRuntime／Session／WorkspaceのTaskとして同時に開始する。
/// 片側の開始失敗は他方へ伝播させず、Authority判断は各Broker要求に委ねる。
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

  return Future.wait([startIndependently(agentA), startIndependently(agentB)]);
}
