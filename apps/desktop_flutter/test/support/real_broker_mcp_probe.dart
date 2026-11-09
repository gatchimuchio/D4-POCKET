import 'dart:convert';
import 'dart:io';

import 'package:agent_orchestration/mcp_connection_client.dart';
import 'package:agent_orchestration/runtime_platform.dart';
import 'package:broker_contract/broker_transport.dart'
    show BrokerClientException;
import 'package:broker_ipc_client/broker_ipc_client.dart';

const _successMarker = 'D4P_REAL_BROKER_DART_PROBE_PASS';

Future<void> main() async {
  try {
    final lines = await stdin
        .transform(utf8.decoder)
        .transform(const LineSplitter())
        .toList();
    if (lines.length != 2) {
      throw const FormatException('Broker接続情報は二件必要');
    }

    await _expectAcceptedList(lines[0]);
    await _expectAuthenticationRejectedList(lines[1]);
    stdout.writeln(_successMarker);
  } on Object {
    stderr.writeln('D4P_REAL_BROKER_DART_PROBE_FAIL');
    exitCode = 1;
  }
}

Future<void> _expectAcceptedList(String line) async {
  final endpoint =
      AuthenticatedLoopbackBrokerEndpoint.fromJson(jsonDecode(line));
  final transport = AuthenticatedLoopbackBrokerTransport(endpoint: endpoint);
  try {
    final connections = await McpConnectionClient(
      transport,
      platform: AgentRuntimePlatform.other,
    ).list();
    if (connections.isNotEmpty) {
      throw const FormatException('想定外のMCP接続が含まれる');
    }
  } finally {
    transport.dispose();
  }
}

Future<void> _expectAuthenticationRejectedList(String line) async {
  final endpoint =
      AuthenticatedLoopbackBrokerEndpoint.fromJson(jsonDecode(line));
  final transport = AuthenticatedLoopbackBrokerTransport(endpoint: endpoint);
  var rejected = false;
  try {
    try {
      await McpConnectionClient(
        transport,
        platform: AgentRuntimePlatform.other,
      ).list();
    } on BrokerClientException {
      rejected = true;
    }
  } finally {
    transport.dispose();
  }
  if (!rejected) {
    throw const FormatException('不正なBroker資格が受理された');
  }
}
