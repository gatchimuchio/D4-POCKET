import 'dart:convert';

import 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerClientException, BrokerTransport;

class McpToolSummary {
  const McpToolSummary({
    required this.toolId,
    required this.name,
    required this.inputSchemaHash,
  });

  final String toolId;
  final String name;
  final String inputSchemaHash;
}

class McpConnectionSummary {
  const McpConnectionSummary({
    required this.serverId,
    required this.displayName,
    required this.transport,
    required this.tools,
    required this.resourceCount,
    required this.promptCount,
  });

  final String serverId;
  final String displayName;
  final String transport;
  final List<McpToolSummary> tools;
  final int resourceCount;
  final int promptCount;

  int get toolCount => tools.length;
}

class McpConnectionClient {
  const McpConnectionClient(this._transport);

  final BrokerTransport _transport;

  Future<McpConnectionSummary> connect({
    required String serverId,
    required String executable,
    required String workspace,
    required String argumentsText,
  }) async {
    final arguments = _arguments(argumentsText);
    if (!_validIdentifier(serverId) ||
        !_validWindowsPath(executable) ||
        !_validWindowsPath(workspace)) {
      throw const BrokerClientException('MCP接続設定の識別子または絶対pathが不正です');
    }
    final payload = <String, Object?>{
      '版': 1,
      '操作': '接続',
      'ServerID': serverId,
      '実行file': executable,
      '引数': arguments,
      'workspace': workspace,
      'Transport': 'stdio',
      'Credential ref': {
        'credential_id': '00000000000000000000000000000000',
        'purpose': 'mcp_transport',
        'target': serverId,
        'required': false,
        'status': 'missing',
      },
    };
    final response = await _transport.request('MCP接続', payload: payload);
    final body = _acceptedBody(response, 'MCP接続');
    if (response['evidence_source'] != 'INTERNAL_STATE') {
      throw const BrokerClientException('MCP接続の証拠範囲が不正です');
    }
    final connection = _connectionSummary(body);
    if (connection.serverId != serverId) {
      throw const BrokerClientException('MCP接続receiptが要求対象と一致しません');
    }
    return connection;
  }

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
    if (!_validIdentifier(serverId)) {
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
        !_validIdentifier(serverId) ||
        displayName is! String ||
        displayName.length > 512 ||
        _containsControl(displayName) ||
        _containsBidiControl(displayName) ||
        transportKind != 'stdio') {
      throw const BrokerClientException('MCP接続metadataの表示項目が不正です');
    }
    final toolSummaries = _toolSummaries(tools);
    return McpConnectionSummary(
      serverId: serverId,
      displayName: displayName,
      transport: transportKind as String,
      tools: toolSummaries,
      resourceCount: resources.length,
      promptCount: prompts.length,
    );
  }

  List<McpToolSummary> _toolSummaries(List<Object?> values) {
    return List.unmodifiable(values.map((raw) {
      if (raw is! Map || raw.keys.any((key) => key is! String)) {
        throw const BrokerClientException('MCP Tool metadataがobjectではありません');
      }
      final tool = raw.cast<String, Object?>();
      const toolFields = {
        'tool_id',
        'name',
        'description_summary',
        'input_schema_hash',
        'risk',
        'status',
      };
      if (tool.length != toolFields.length ||
          !tool.keys.toSet().containsAll(toolFields)) {
        throw const BrokerClientException('MCP Tool metadataのfieldが不正です');
      }
      final toolId = tool['tool_id'];
      final name = tool['name'];
      final description = tool['description_summary'];
      final schemaHash = tool['input_schema_hash'];
      final risk = tool['risk'];
      final status = tool['status'];
      if (toolId is! String ||
          !RegExp(r'^tool-[a-f0-9]{1,251}$').hasMatch(toolId) ||
          name is! String ||
          !_safeToolName(name) ||
          description != '' ||
          schemaHash is! String ||
          !RegExp(r'^sha256:[a-f0-9]{64}$').hasMatch(schemaHash) ||
          risk != 'unknown' ||
          status is! Map ||
          status.keys.any((key) => key is! String)) {
        throw const BrokerClientException('MCP Tool metadata値が不正です');
      }
      final statusFields = status.cast<String, Object?>();
      if (statusFields.length != 2 ||
          statusFields['status'] != 'supported' ||
          statusFields['reason'] is! String ||
          utf8.encode(statusFields['reason']! as String).length > 256 ||
          _containsControl(statusFields['reason']! as String)) {
        throw const BrokerClientException('MCP Tool metadata状態が不正です');
      }
      return McpToolSummary(
        toolId: toolId,
        name: name,
        inputSchemaHash: schemaHash,
      );
    }));
  }

  bool _safeToolName(String value) =>
      value.isNotEmpty &&
      utf8.encode(value).length <= 256 &&
      !_containsControl(value) &&
      !_containsBidiControl(value);

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

  List<String> _arguments(String value) {
    if (value.isEmpty) return const [];
    final arguments = const LineSplitter().convert(value);
    if (arguments.length > 32 ||
        arguments.any((argument) =>
            argument.isEmpty ||
            utf8.encode(argument).length > 1024 ||
            _containsControl(argument))) {
      throw const BrokerClientException('MCP接続引数は1行1項目で最大32件です');
    }
    return arguments;
  }

  bool _validIdentifier(String value) =>
      value.isNotEmpty &&
      utf8.encode(value).length <= 128 &&
      !_containsControl(value);

  bool _validWindowsPath(String value) =>
      value.isNotEmpty &&
      utf8.encode(value).length <= 1024 &&
      !_containsControl(value) &&
      RegExp(r'^(?:[A-Za-z]:[\\/]|\\\\[^\\/]+[\\/][^\\/]+)').hasMatch(value);

  bool _containsControl(String value) => value.runes.any(
        (rune) => rune <= 0x1f || (rune >= 0x7f && rune <= 0x9f),
      );

  bool _containsBidiControl(String value) => value.runes.any(
        (rune) =>
            rune == 0x061c ||
            rune == 0x200e ||
            rune == 0x200f ||
            (rune >= 0x202a && rune <= 0x202e) ||
            (rune >= 0x2066 && rune <= 0x206f),
      );
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
