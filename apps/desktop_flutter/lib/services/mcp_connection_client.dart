import 'dart:convert';

import 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerClientException, BrokerTransport;

class McpConnectionSummary {
  const McpConnectionSummary({
    required this.serverId,
    required this.displayName,
    required this.transport,
    required this.toolCount,
    required this.resourceCount,
    required this.promptCount,
  });

  final String serverId;
  final String displayName;
  final String transport;
  final int toolCount;
  final int resourceCount;
  final int promptCount;
}

class McpConnectionClient {
  const McpConnectionClient(this._transport);

  final BrokerTransport _transport;

  Future<List<McpConnectionSummary>> list() async {
    final response = await _transport.request(
      'MCP接続一覧',
      payload: const {'版': 1},
    );
    final body = _acceptedBody(response, 'MCP接続一覧');
    const requiredBodyFields = {
      '版',
      'MCP接続一覧',
      '件数',
      '公開範囲',
      '証拠種別',
    };
    if (body.length != requiredBodyFields.length ||
        !body.keys.toSet().containsAll(requiredBodyFields) ||
        body['版'] != 1 ||
        body['公開範囲'] != 'metadata_only' ||
        body['証拠種別'] != 'INTERNAL_STATE' ||
        response['evidence_source'] != 'INTERNAL_STATE') {
      throw const BrokerClientException('MCP接続一覧の境界が不正です');
    }
    final rawConnections = body['MCP接続一覧'];
    final count = body['件数'];
    if (rawConnections is! List ||
        rawConnections.length > 128 ||
        count != rawConnections.length) {
      throw const BrokerClientException('MCP接続一覧の件数が不正です');
    }
    return List.unmodifiable(rawConnections.map(_connectionSummary));
  }

  Future<void> disconnect(String serverId) async {
    if (serverId.isEmpty || utf8.encode(serverId).length > 128) {
      throw const BrokerClientException('MCP Server識別子の形式が不正です');
    }
    final response = await _transport.request(
      'MCP切断',
      payload: {
        '版': 1,
        '操作': '切断',
        'ServerID': serverId,
      },
    );
    final body = _acceptedBody(response, 'MCP切断');
    const requiredReceiptFields = {
      '版',
      'ServerID',
      '接続状態',
      '能力ID',
      '権限ID',
      '承認状態',
      '復旧ID',
      '権限生成',
      '公開範囲',
      '証拠種別',
      '切断監査ID',
    };
    if (body.length != requiredReceiptFields.length ||
        !body.keys.toSet().containsAll(requiredReceiptFields) ||
        body['版'] != 1 ||
        body['ServerID'] != serverId ||
        body['接続状態'] != 'disconnected' ||
        body['能力ID'] != 'mcp.connection.disconnect' ||
        body['権限ID'] != 'permission.mcp.connection.disconnect' ||
        body['承認状態'] != 'owner_control_approved' ||
        body['復旧ID'] != 'retry-mcp-disconnect' ||
        body['権限生成'] != 'なし' ||
        body['公開範囲'] != 'metadata_only' ||
        body['証拠種別'] != 'LIVE_RUNTIME' ||
        response['evidence_source'] != 'LIVE_RUNTIME' ||
        body['切断監査ID'] is! String ||
        (body['切断監査ID'] as String).isEmpty) {
      throw const BrokerClientException('MCP切断receiptの境界が不正です');
    }
  }

  McpConnectionSummary _connectionSummary(Object? raw) {
    if (raw is! Map) {
      throw const BrokerClientException('MCP接続receiptがobjectではありません');
    }
    final receipt = Map<String, Object?>.from(raw);
    const receiptFields = {
      '版',
      '契約種別',
      'Server',
      'Transport',
      'Tool',
      'Resource',
      'Prompt',
      'Credential ref',
      'Trust',
      'Capability diff',
      '権限生成',
      '公開範囲',
      '証拠種別',
      '接続状態',
      '能力ID',
      '権限ID',
      '承認状態',
      '復旧ID',
      '接続監査ID',
    };
    if (receipt.length != receiptFields.length ||
        !receipt.keys.toSet().containsAll(receiptFields) ||
        receipt['版'] != 1 ||
        receipt['契約種別'] != 'MCP外部概念射影' ||
        receipt['権限生成'] != 'なし' ||
        receipt['公開範囲'] != 'metadata_only' ||
        receipt['証拠種別'] != 'INTERNAL_STATE' ||
        receipt['接続状態'] != 'connected' ||
        receipt['能力ID'] != 'mcp.connection.connect' ||
        receipt['権限ID'] != 'permission.mcp.connection.connect' ||
        receipt['承認状態'] != 'owner_control_approved' ||
        receipt['復旧ID'] != 'recover-mcp-connection' ||
        receipt['接続監査ID'] is! String ||
        (receipt['接続監査ID'] as String).isEmpty) {
      throw const BrokerClientException('MCP接続receiptの境界が不正です');
    }
    final server = _objectField(receipt, 'Server');
    final transport = _objectField(receipt, 'Transport');
    final tools = _arrayField(receipt, 'Tool');
    final resources = _arrayField(receipt, 'Resource');
    final prompts = _arrayField(receipt, 'Prompt');
    for (final key in ['Credential ref', 'Trust', 'Capability diff']) {
      _objectField(receipt, key);
    }
    final serverId = server['server_id'];
    final displayName = server['表示名'];
    final transportKind = transport['kind'];
    if (serverId is! String ||
        serverId.isEmpty ||
        utf8.encode(serverId).length > 128 ||
        displayName is! String ||
        displayName.length > 512 ||
        transportKind != 'stdio') {
      throw const BrokerClientException('MCP接続metadataの表示項目が不正です');
    }
    return McpConnectionSummary(
      serverId: serverId,
      displayName: displayName,
      transport: transportKind as String,
      toolCount: tools.length,
      resourceCount: resources.length,
      promptCount: prompts.length,
    );
  }

  Map<String, Object?> _objectField(Map<String, Object?> source, String key) {
    final value = source[key];
    if (value is! Map) {
      throw BrokerClientException('MCP metadataの$keyがobjectではありません');
    }
    return Map<String, Object?>.from(value);
  }

  List<Object?> _arrayField(Map<String, Object?> source, String key) {
    final value = source[key];
    if (value is! List || value.length > 256) {
      throw BrokerClientException('MCP metadataの$keyが不正です');
    }
    return value.cast<Object?>();
  }
}

Map<String, Object?> _acceptedBody(
  Map<String, Object?> response,
  String operation,
) {
  if (response['status'] != 'accepted') {
    throw BrokerClientException('$operationはBrokerにより拒否されました');
  }
  final body = response['body'];
  if (body is! Map) {
    throw BrokerClientException('$operationのresponse bodyがobjectではありません');
  }
  return Map<String, Object?>.from(body);
}
