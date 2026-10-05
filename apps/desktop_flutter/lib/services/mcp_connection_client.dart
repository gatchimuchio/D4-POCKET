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

class McpResourceSummary {
  const McpResourceSummary({
    required this.resourceId,
    required this.name,
    required this.uriTemplateHash,
  });

  final String resourceId;
  final String name;
  final String uriTemplateHash;
}

class McpPromptSummary {
  const McpPromptSummary({
    required this.promptId,
    required this.name,
    required this.argumentSchemaHash,
  });

  final String promptId;
  final String name;
  final String argumentSchemaHash;
}

class McpCredentialSummary {
  const McpCredentialSummary({
    required this.credentialId,
    required this.purpose,
    required this.target,
    required this.kind,
    required this.status,
    required this.revokedAt,
    required this.ciphertextHash,
    required this.createdAuditId,
  });

  final String credentialId;
  final String purpose;
  final String target;
  final String kind;
  final String status;
  final int? revokedAt;
  final String ciphertextHash;
  final String createdAuditId;
}

class McpConnectionSummary {
  const McpConnectionSummary({
    required this.serverId,
    required this.displayName,
    required this.transport,
    required this.tools,
    required this.resources,
    required this.prompts,
    required this.connectionState,
    required this.executionState,
  });

  final String serverId;
  final String displayName;
  final String transport;
  final List<McpToolSummary> tools;
  final List<McpResourceSummary> resources;
  final List<McpPromptSummary> prompts;
  final String connectionState;
  final String executionState;

  int get toolCount => tools.length;
  int get resourceCount => resources.length;
  int get promptCount => prompts.length;
}

class McpToolCallReceipt {
  const McpToolCallReceipt({
    required this.serverId,
    required this.toolId,
    required this.name,
    required this.argumentsHash,
    required this.resultHash,
    required this.toolError,
    required this.contentTypes,
    required this.connectionState,
    required this.auditId,
  });

  final String serverId;
  final String toolId;
  final String name;
  final String argumentsHash;
  final String resultHash;
  final bool toolError;
  final List<String> contentTypes;
  final String connectionState;
  final String auditId;
}

class McpConnectionClient {
  const McpConnectionClient(this._transport);

  final BrokerTransport _transport;

  Future<McpConnectionSummary> connect({
    required String serverId,
    required String executable,
    required String workspace,
    required String argumentsText,
    String? credentialId,
    String? credentialEnvironmentVariable,
  }) async {
    final arguments = _arguments(argumentsText);
    if (!_validIdentifier(serverId) ||
        !_validWindowsPath(executable) ||
        !_validWindowsPath(workspace) ||
        (credentialId == null) != (credentialEnvironmentVariable == null) ||
        (credentialId != null &&
            (!_validCredentialId(credentialId) ||
                !_safeCredentialEnvironmentVariable(
                    credentialEnvironmentVariable!)))) {
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
      'Credential ref': credentialId == null
          ? {
              'credential_id': '00000000000000000000000000000000',
              'purpose': 'mcp_transport',
              'target': serverId,
              'required': false,
              'status': 'missing',
            }
          : {
              'credential_id': credentialId,
              'purpose': 'mcp_transport',
              'target': serverId,
              'required': true,
              'status': 'configured',
              'environment_variable': credentialEnvironmentVariable,
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

  Future<List<McpCredentialSummary>> listCredentials({
    required String targetServerId,
  }) async {
    if (!_validIdentifier(targetServerId)) {
      throw const BrokerClientException('MCP Server識別子の形式が不正です');
    }
    return List.unmodifiable((await _listCredentials()).where((entry) =>
        entry.purpose == 'mcp_transport' && entry.target == targetServerId));
  }

  Future<List<McpCredentialSummary>> listProviderCredentials() async =>
      List.unmodifiable((await _listCredentials()).where((entry) =>
          entry.purpose == 'provider_api_key' &&
          entry.target == 'openai_codex_cli' &&
          entry.kind == 'api_key' &&
          entry.status == '有効'));

  Future<List<McpCredentialSummary>> _listCredentials() async {
    final response = await _transport.request(
      '資格情報一覧',
      payload: const {'版': 1},
    );
    final body = _acceptedBody(response, '資格情報一覧');
    const bodyFields = {'版', '資格情報一覧', '件数', '公開範囲', '証拠種別'};
    if (body.length != bodyFields.length ||
        !body.keys.toSet().containsAll(bodyFields) ||
        body['版'] != 1 ||
        body['公開範囲'] != 'metadata_only' ||
        body['証拠種別'] != 'INTERNAL_STATE' ||
        response['evidence_source'] != 'INTERNAL_STATE') {
      throw const BrokerClientException('資格情報一覧の公開境界が不正です');
    }
    final rawEntries = body['資格情報一覧'];
    if (rawEntries is! List ||
        rawEntries.length > 256 ||
        body['件数'] != rawEntries.length) {
      throw const BrokerClientException('資格情報一覧の件数が不正です');
    }
    final entries = rawEntries.map((raw) {
      if (raw is! Map || raw.keys.any((key) => key is! String)) {
        throw const BrokerClientException('資格情報metadataがobjectではありません');
      }
      final entry = raw.cast<String, Object?>();
      const fields = {
        '版',
        '資格情報ID',
        '用途',
        '接続対象',
        '種類',
        '保管方式',
        '状態',
        '作成時刻UnixMillis',
        '最終使用時刻UnixMillis',
        '失効時刻UnixMillis',
        '暗号文hash',
        '作成監査ID',
        '公開範囲',
        '証拠種別',
      };
      final id = entry['資格情報ID'];
      final purpose = entry['用途'];
      final target = entry['接続対象'];
      final kind = entry['種類'];
      final createdAt = entry['作成時刻UnixMillis'];
      final lastUsedAt = entry['最終使用時刻UnixMillis'];
      final revokedAt = entry['失効時刻UnixMillis'];
      final ciphertextHash = entry['暗号文hash'];
      final createdAuditId = entry['作成監査ID'];
      if (entry.length != fields.length ||
          !entry.keys.toSet().containsAll(fields) ||
          entry['版'] != 1 ||
          id is! String ||
          !_validCredentialId(id) ||
          purpose is! String ||
          purpose.isEmpty ||
          purpose.length > 256 ||
          _containsControl(purpose) ||
          target is! String ||
          target.isEmpty ||
          target.length > 256 ||
          _containsControl(target) ||
          kind is! String ||
          !const {'api_key', 'oauth', 'basic', 'ssh', 'custom'}
              .contains(kind) ||
          entry['保管方式'] != 'windows_dpapi' ||
          !const {'有効', '失効'}.contains(entry['状態']) ||
          createdAt is! int ||
          createdAt < 1 ||
          (lastUsedAt != null && (lastUsedAt is! int || lastUsedAt < 0)) ||
          (entry['状態'] == '有効' && revokedAt != null) ||
          (entry['状態'] == '失効' &&
              (revokedAt is! int || revokedAt < createdAt)) ||
          ciphertextHash is! String ||
          !RegExp(r'^sha256:[a-f0-9]{64}$').hasMatch(ciphertextHash) ||
          createdAuditId is! String ||
          createdAuditId.isEmpty ||
          entry['公開範囲'] != 'metadata_only' ||
          entry['証拠種別'] != 'INTERNAL_STATE') {
        throw const BrokerClientException('資格情報metadataのfieldまたは境界が不正です');
      }
      return McpCredentialSummary(
        credentialId: id,
        purpose: purpose,
        target: target,
        kind: kind,
        status: entry['状態'] as String,
        revokedAt: revokedAt as int?,
        ciphertextHash: ciphertextHash,
        createdAuditId: createdAuditId,
      );
    }).toList(growable: false);
    return List.unmodifiable(entries);
  }

  Future<McpCredentialSummary> revokeCredential({
    required McpCredentialSummary credential,
  }) async {
    if (!_validCredentialId(credential.credentialId) ||
        credential.status != '有効' ||
        credential.purpose.isEmpty ||
        credential.purpose.length > 256 ||
        _containsControl(credential.purpose) ||
        credential.target.isEmpty ||
        credential.target.length > 256 ||
        _containsControl(credential.target) ||
        !RegExp(r'^sha256:[a-f0-9]{64}$').hasMatch(credential.ciphertextHash) ||
        credential.createdAuditId.isEmpty ||
        credential.createdAuditId.length > 256 ||
        _containsControl(credential.createdAuditId)) {
      throw const BrokerClientException('Credential識別子の形式が不正です');
    }
    final response = await _transport.request(
      '資格情報失効',
      payload: {
        '版': 1,
        '資格情報ID': credential.credentialId,
        '用途': credential.purpose,
        '接続対象': credential.target,
        '暗号文hash': credential.ciphertextHash,
        '作成監査ID': credential.createdAuditId,
      },
    );
    final body = _acceptedBody(response, '資格情報失効');
    const fields = {
      '版',
      '資格情報ID',
      '用途',
      '接続対象',
      '種類',
      '保管方式',
      '状態',
      '作成時刻UnixMillis',
      '最終使用時刻UnixMillis',
      '失効時刻UnixMillis',
      '暗号文hash',
      '作成監査ID',
      '公開範囲',
      '証拠種別',
    };
    final id = body['資格情報ID'];
    final purpose = body['用途'];
    final target = body['接続対象'];
    final kind = body['種類'];
    final createdAt = body['作成時刻UnixMillis'];
    final lastUsedAt = body['最終使用時刻UnixMillis'];
    final revokedAt = body['失効時刻UnixMillis'];
    final ciphertextHash = body['暗号文hash'];
    final createdAuditId = body['作成監査ID'];
    if (body.length != fields.length ||
        !body.keys.toSet().containsAll(fields) ||
        body['版'] != 1 ||
        id != credential.credentialId ||
        purpose is! String ||
        purpose != credential.purpose ||
        purpose.isEmpty ||
        purpose.length > 256 ||
        _containsControl(purpose) ||
        target is! String ||
        target != credential.target ||
        target.isEmpty ||
        target.length > 256 ||
        _containsControl(target) ||
        kind is! String ||
        !const {'api_key', 'oauth', 'basic', 'ssh', 'custom'}.contains(kind) ||
        createdAt is! int ||
        createdAt < 1 ||
        (lastUsedAt != null && (lastUsedAt is! int || lastUsedAt < 0)) ||
        body['状態'] != '失効' ||
        revokedAt is! int ||
        revokedAt < createdAt ||
        ciphertextHash is! String ||
        ciphertextHash != credential.ciphertextHash ||
        !RegExp(r'^sha256:[a-f0-9]{64}$').hasMatch(ciphertextHash) ||
        createdAuditId is! String ||
        createdAuditId != credential.createdAuditId ||
        createdAuditId.isEmpty ||
        body['保管方式'] != 'windows_dpapi' ||
        body['公開範囲'] != 'metadata_only' ||
        body['証拠種別'] != 'INTERNAL_STATE' ||
        response['evidence_source'] != 'INTERNAL_STATE') {
      throw const BrokerClientException('Credential失効receiptの公開境界が不正です');
    }
    return McpCredentialSummary(
      credentialId: credential.credentialId,
      purpose: purpose,
      target: target,
      kind: kind,
      status: '失効',
      revokedAt: revokedAt,
      ciphertextHash: ciphertextHash,
      createdAuditId: createdAuditId,
    );
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

  Future<McpToolCallReceipt> callTool({
    required String serverId,
    required McpToolSummary tool,
    required Map<String, Object?> arguments,
  }) async {
    if (!_validIdentifier(serverId) ||
        !RegExp(r'^tool-[a-f0-9]{142}$').hasMatch(tool.toolId) ||
        !_safeMetadataName(tool.name) ||
        utf8.encode(jsonEncode(arguments)).length > 32 * 1024) {
      throw const BrokerClientException('MCP Tool実行要求が不正または上限超過です');
    }
    final response = await _transport.request(
      'MCP Tool実行',
      payload: {
        '版': 1,
        '操作': '実行',
        'ServerID': serverId,
        'ToolID': tool.toolId,
        '名前': tool.name,
        'arguments': arguments,
      },
    );
    final body = _acceptedBody(response, 'MCP Tool実行');
    const fields = {
      '版',
      '契約種別',
      'ServerID',
      'ToolID',
      '名前',
      'arguments_hash',
      'result_hash',
      'Tool error',
      'content_count',
      'content_types',
      '接続状態',
      '能力ID',
      '権限ID',
      '承認状態',
      '承認監査ID',
      '復旧ID',
      '権限生成',
      '公開範囲',
      '証拠種別',
      '監査ID',
    };
    final contentTypes = body['content_types'];
    final contentCount = body['content_count'];
    if (body.length != fields.length ||
        !body.keys.toSet().containsAll(fields) ||
        body['版'] != 1 ||
        body['契約種別'] != 'MCP Tool実行receipt' ||
        body['ServerID'] != serverId ||
        body['ToolID'] != tool.toolId ||
        body['名前'] != tool.name ||
        body['arguments_hash'] is! String ||
        !RegExp(r'^sha256:[a-f0-9]{64}$')
            .hasMatch(body['arguments_hash'] as String) ||
        body['result_hash'] is! String ||
        !RegExp(r'^sha256:[a-f0-9]{64}$')
            .hasMatch(body['result_hash'] as String) ||
        body['Tool error'] is! bool ||
        contentCount is! int ||
        contentCount < 0 ||
        contentCount > 128 ||
        contentTypes is! List ||
        contentTypes.length != contentCount ||
        contentTypes.any((value) =>
            value is! String ||
            !const {'text', 'image', 'audio', 'resource_link', 'resource'}
                .contains(value)) ||
        !const {'connected', 'quarantined'}.contains(body['接続状態']) ||
        body['能力ID'] != 'mcp.tool.call' ||
        body['権限ID'] != 'permission.mcp.tool.call.one_shot' ||
        body['承認状態'] != 'native_owner_confirmed' ||
        body['承認監査ID'] is! String ||
        (body['承認監査ID'] as String).isEmpty ||
        body['復旧ID'] != 'inspect-mcp-tool-side-effect' ||
        body['権限生成'] != 'Broker内一回限りPermissionを消費' ||
        body['公開範囲'] != 'hash_only' ||
        body['証拠種別'] != 'LIVE_RUNTIME' ||
        response['evidence_source'] != 'LIVE_RUNTIME' ||
        body['監査ID'] is! String ||
        (body['監査ID'] as String).isEmpty) {
      throw const BrokerClientException('MCP Tool実行receiptの境界が不正です');
    }
    return McpToolCallReceipt(
      serverId: serverId,
      toolId: tool.toolId,
      name: tool.name,
      argumentsHash: body['arguments_hash'] as String,
      resultHash: body['result_hash'] as String,
      toolError: body['Tool error'] as bool,
      contentTypes: List.unmodifiable(contentTypes.cast<String>()),
      connectionState: body['接続状態'] as String,
      auditId: body['監査ID'] as String,
    );
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
      '実行状態',
    };
    if (receipt.length != receiptFields.length ||
        !receipt.keys.toSet().containsAll(receiptFields) ||
        receipt['版'] != 1 ||
        receipt['契約種別'] != 'MCP外部概念射影' ||
        receipt['権限生成'] != 'なし' ||
        receipt['公開範囲'] != 'metadata_only' ||
        receipt['証拠種別'] != 'INTERNAL_STATE' ||
        !const {'connected', 'quarantined'}.contains(receipt['接続状態']) ||
        (receipt['接続状態'] == 'connected' && receipt['実行状態'] != 'ready') ||
        (receipt['接続状態'] == 'quarantined' &&
            receipt['実行状態'] != 'quarantined') ||
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
    final resourceSummaries = _resourceSummaries(resources);
    final promptSummaries = _promptSummaries(prompts);
    return McpConnectionSummary(
      serverId: serverId,
      displayName: displayName,
      transport: transportKind as String,
      tools: toolSummaries,
      resources: resourceSummaries,
      prompts: promptSummaries,
      connectionState: receipt['接続状態'] as String,
      executionState: receipt['実行状態'] as String,
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
          !_safeMetadataName(name) ||
          description != '' ||
          schemaHash is! String ||
          !RegExp(r'^sha256:[a-f0-9]{64}$').hasMatch(schemaHash) ||
          risk != 'unknown' ||
          !_supportedMetadataStatus(status)) {
        throw const BrokerClientException('MCP Tool metadata値が不正です');
      }
      return McpToolSummary(
        toolId: toolId,
        name: name,
        inputSchemaHash: schemaHash,
      );
    }));
  }

  List<McpResourceSummary> _resourceSummaries(List<Object?> values) {
    return List.unmodifiable(values.map((raw) {
      if (raw is! Map || raw.keys.any((key) => key is! String)) {
        throw const BrokerClientException(
            'MCP Resource metadataがobjectではありません');
      }
      final resource = raw.cast<String, Object?>();
      const fields = {
        'resource_id',
        'name',
        'uri_template_hash',
        'mime_type',
        'status',
      };
      final resourceId = resource['resource_id'];
      final name = resource['name'];
      final uriHash = resource['uri_template_hash'];
      if (resource.length != fields.length ||
          !resource.keys.toSet().containsAll(fields) ||
          resourceId is! String ||
          !RegExp(r'^resource-[a-f0-9]{1,247}$').hasMatch(resourceId) ||
          name is! String ||
          !_safeMetadataName(name) ||
          uriHash is! String ||
          !RegExp(r'^sha256:[a-f0-9]{64}$').hasMatch(uriHash) ||
          resource['mime_type'] != 'application/octet-stream' ||
          !_supportedMetadataStatus(resource['status'])) {
        throw const BrokerClientException('MCP Resource metadata値が不正です');
      }
      return McpResourceSummary(
        resourceId: resourceId,
        name: name,
        uriTemplateHash: uriHash,
      );
    }));
  }

  List<McpPromptSummary> _promptSummaries(List<Object?> values) {
    return List.unmodifiable(values.map((raw) {
      if (raw is! Map || raw.keys.any((key) => key is! String)) {
        throw const BrokerClientException('MCP Prompt metadataがobjectではありません');
      }
      final prompt = raw.cast<String, Object?>();
      const fields = {
        'prompt_id',
        'name',
        'description_summary',
        'argument_schema_hash',
        'status',
      };
      final promptId = prompt['prompt_id'];
      final name = prompt['name'];
      final schemaHash = prompt['argument_schema_hash'];
      if (prompt.length != fields.length ||
          !prompt.keys.toSet().containsAll(fields) ||
          promptId is! String ||
          !RegExp(r'^prompt-[a-f0-9]{1,249}$').hasMatch(promptId) ||
          name is! String ||
          !_safeMetadataName(name) ||
          prompt['description_summary'] != '' ||
          schemaHash is! String ||
          !RegExp(r'^sha256:[a-f0-9]{64}$').hasMatch(schemaHash) ||
          !_supportedMetadataStatus(prompt['status'])) {
        throw const BrokerClientException('MCP Prompt metadata値が不正です');
      }
      return McpPromptSummary(
        promptId: promptId,
        name: name,
        argumentSchemaHash: schemaHash,
      );
    }));
  }

  bool _supportedMetadataStatus(Object? value) {
    if (value is! Map || value.keys.any((key) => key is! String)) return false;
    final status = value.cast<String, Object?>();
    final reason = status['reason'];
    return status.length == 2 &&
        status['status'] == 'supported' &&
        reason is String &&
        utf8.encode(reason).length <= 256 &&
        !_containsControl(reason);
  }

  bool _safeMetadataName(String value) =>
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

  bool _validCredentialId(String value) =>
      RegExp(r'^[a-f0-9]{32}$').hasMatch(value);

  bool _safeCredentialEnvironmentVariable(String value) {
    if (!RegExp(r'^[A-Za-z_][A-Za-z0-9_]{0,127}$').hasMatch(value)) {
      return false;
    }
    final normalized = value.toUpperCase();
    return !{
          'PATH',
          'SYSTEMROOT',
          'WINDIR',
          'TEMP',
          'TMP',
          'USERPROFILE',
          'HOME',
          'APPDATA',
          'LOCALAPPDATA',
          'PROGRAMDATA',
          'SYSTEMDRIVE',
          'COMSPEC',
          'PATHEXT',
          'PSMODULEPATH',
        }.contains(normalized) &&
        !normalized.startsWith('GUI_SHELL_');
  }

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
