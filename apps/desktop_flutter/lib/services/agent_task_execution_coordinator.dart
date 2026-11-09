import 'package:agent_orchestration/agent_task_execution_coordinator.dart'
    as runtime;
import 'package:agent_orchestration/runtime_platform.dart';
import 'package:flutter/foundation.dart'
    show TargetPlatform, defaultTargetPlatform;

export 'package:agent_orchestration/agent_task_execution_coordinator.dart'
    show AgentTaskExecutionUpdate, AgentTaskExecutionUpdateHandler;

class AgentTaskExecutionCoordinator
    extends runtime.AgentTaskExecutionCoordinator {
  AgentTaskExecutionCoordinator(super.transport)
      : super(platform: _agentRuntimePlatform(defaultTargetPlatform));
}

AgentRuntimePlatform _agentRuntimePlatform(TargetPlatform platform) =>
    switch (platform) {
      TargetPlatform.windows => AgentRuntimePlatform.windows,
      TargetPlatform.macOS => AgentRuntimePlatform.macos,
      TargetPlatform.linux => AgentRuntimePlatform.linux,
      TargetPlatform.android => AgentRuntimePlatform.android,
      TargetPlatform.iOS => AgentRuntimePlatform.ios,
      TargetPlatform.fuchsia => AgentRuntimePlatform.other,
    };
