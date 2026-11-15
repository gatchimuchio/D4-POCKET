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

    final response = await transport.request(
      '監査一覧取得',
      payload: const <String, Object?>{'version': 1, 'limit': 100},
    );
    final body = response['body'];
    final auditEventId = response['audit_event_id'];
    if (response['status'] != 'accepted' ||
        response['evidence_source'] != 'LIVE_RUNTIME' ||
        response['error'] != null ||
        auditEventId is! String ||
        !RegExp(r'^broker-audit-[1-9][0-9]*$').hasMatch(auditEventId) ||
        body is! Map) {
      throw const FormatException('実Brokerの監査一覧応答が不正');
    }
    const pageFields = <String>{
      'version',
      'integrity_status',
      'anchor_status',
      'verified_event_count',
      'snapshot_head_event_id',
      'has_older_events',
      'next_before_event_id',
      'events',
    };
    final page = Map<String, Object?>.from(body);
    final events = page['events'];
    if (page.length != pageFields.length ||
        !page.keys.every(pageFields.contains) ||
        page['version'] != 1 ||
        page['integrity_status'] != 'verified' ||
        page['anchor_status'] != 'verified' ||
        page['verified_event_count'] is! int ||
        page['snapshot_head_event_id'] is! String ||
        page['has_older_events'] is! bool ||
        events is! List ||
        events.isEmpty ||
        events.length > 100 ||
        !events.any(
          (event) => event is Map && event['action'] == 'MCP接続一覧',
        )) {
      throw const FormatException('実Brokerの検証済み監査pageが不正');
    }
    const eventFields = <String>{
      'event_id',
      'action',
      'result',
      'payload_hash',
      'previous_event_hash',
      'event_hash',
    };
    for (final value in events) {
      if (value is! Map ||
          value.length != eventFields.length ||
          !value.keys.every(eventFields.contains)) {
        throw const FormatException('監査eventに非公開項目が含まれる');
      }
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
      await transport.request(
        '監査一覧取得',
        payload: const <String, Object?>{'version': 1, 'limit': 100},
      );
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
