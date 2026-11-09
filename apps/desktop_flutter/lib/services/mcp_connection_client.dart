import 'package:agent_orchestration/mcp_connection_client.dart' as runtime;
import 'package:agent_orchestration/runtime_platform.dart';
import 'package:flutter/foundation.dart'
    show TargetPlatform, defaultTargetPlatform;

export 'package:agent_orchestration/mcp_connection_client.dart'
    show
        McpAgentResultContext,
        McpConnectionSummary,
        McpCredentialSummary,
        McpPromptSummary,
        McpResourceSummary,
        McpToolCallReceipt,
        McpToolSummary;

class McpConnectionClient extends runtime.McpConnectionClient {
  McpConnectionClient(super.transport)
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
