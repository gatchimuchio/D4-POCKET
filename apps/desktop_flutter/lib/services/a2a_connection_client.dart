import 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerClientException, BrokerTransport;

class A2aConnectionSummary {
  const A2aConnectionSummary({
    required this.agentId,
    required this.displayName,
    required this.descriptionSummary,
    required this.version,
    required this.endpointHash,
    required this.connectionState,
    required this.evidenceSource,
    required this.approvalState,
    required this.trustReason,
    required this.connectionAuditId,
  });

  final String agentId;
  final String displayName;
  final String descriptionSummary;
  final String version;
  final String endpointHash;
  final String connectionState;
  final String evidenceSource;
  final String approvalState;
  final String trustReason;
  final String connectionAuditId;
}

class A2aConnectionClient {
  const A2aConnectionClient(this._transport);

  final BrokerTransport _transport;

  static bool _validAgentId(String value) =>
      RegExp(r'^[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}$').hasMatch(value);

  Future<A2aConnectionSummary> connect({
    required String agentId,
    required String agentCardUri,
  }) async {
    final normalizedId = agentId.trim();
    final normalizedUri = agentCardUri.trim();
    if (!_validAgentId(normalizedId) || !_validLoopbackUri(normalizedUri)) {
      throw const BrokerClientException(
        'Agent IDまたはloopback HTTP Agent Card URIが不正です',
      );
    }

    final payload = <String, Object?>{
      '版': 1,
      '操作': '接続',
      'AgentID': normalizedId,
      'Agent Card URI': normalizedUri,
      'protocol_version': '1.0',
      'Transport': 'http',
      'Credential ref': {
        'credential_id': '00000000000000000000000000000000',
        'purpose': 'A2A接続',
        'target': normalizedId,
        'required': false,
        'status': 'missing',
      },
    };
    final response = await _transport.request('A2A接続', payload: payload);
    final body = _acceptedBody(response, 'A2A接続');
    if (response['evidence_source'] != 'INTERNAL_STATE' ||
        response['audit_event_id'] != body['接続監査ID']) {
      throw const BrokerClientException('A2A接続応答の証拠または監査参照が不正です');
    }
    final summary = _parseReceipt(body);
    if (summary.agentId != normalizedId ||
        summary.evidenceSource != 'LIVE_RUNTIME' ||
        summary.connectionState != 'connected') {
      throw const BrokerClientException('A2A接続receiptが現在の接続要求と一致しません');
    }
    return summary;
  }

  Future<List<A2aConnectionSummary>> list() async {
    final response = await _transport.request(
      'A2A接続一覧',
      payload: const {'版': 1},
    );
    final body = _acceptedBody(response, 'A2A接続一覧');
    const fields = {'版', 'A2A接続一覧', '件数', '公開範囲', '証拠種別'};
    final raw = body['A2A接続一覧'];
    if (body.length != fields.length ||
        body.keys.any((key) => !fields.contains(key)) ||
        body['版'] != 1 ||
        body['公開範囲'] != 'metadata_only' ||
        body['証拠種別'] != 'INTERNAL_STATE' ||
        response['evidence_source'] != 'INTERNAL_STATE' ||
        raw is! List ||
        raw.length > 64 ||
        body['件数'] != raw.length) {
      throw const BrokerClientException('A2A接続一覧の公開範囲または件数が不正です');
    }
    final connections = raw.map((item) {
      if (item is! Map || item.keys.any((key) => key is! String)) {
        throw const BrokerClientException('A2A接続receiptがobjectではありません');
      }
      return _parseReceipt(Map<String, Object?>.from(item));
    }).toList(growable: false);
    final ids = connections.map((item) => item.agentId).toSet();
    if (ids.length != connections.length) {
      throw const BrokerClientException('A2A接続一覧にAgent IDの重複があります');
    }
    return List.unmodifiable(connections);
  }

  static Map<String, Object?> _acceptedBody(
    Map<String, Object?> response,
    String operation,
  ) {
    if (response['operation'] != operation ||
        response['status'] != 'accepted' ||
        response['error'] != null ||
        response['audit_event_id'] is! String ||
        !_validAuditId(response['audit_event_id'] as String)) {
      throw BrokerClientException('$operation はBrokerに受理されませんでした');
    }
    final body = response['body'];
    if (body is! Map || body.keys.any((key) => key is! String)) {
      throw BrokerClientException('$operation 応答にbody objectがありません');
    }
    return Map<String, Object?>.from(body);
  }

  static A2aConnectionSummary _parseReceipt(Map<String, Object?> receipt) {
    const fields = {
      '版',
      '契約種別',
      'protocol_version',
      'AgentID',
      'Agent Card',
      'Task',
      'Message',
      'Artifact',
      'Stream',
      'Trust',
      'Capability diff',
      '権限生成',
      'authority_strip',
      '公開範囲',
      '証拠種別',
      '接続状態',
      '能力ID',
      '権限ID',
      '承認状態',
      '復旧ID',
      '接続監査ID',
    };
    if (receipt.length != fields.length ||
        receipt.keys.any((key) => !fields.contains(key)) ||
        receipt['版'] != 1 ||
        receipt['契約種別'] != 'A2A外部概念射影' ||
        receipt['protocol_version'] != '1.0' ||
        receipt['権限生成'] != 'なし' ||
        receipt['authority_strip'] != true ||
        receipt['公開範囲'] != 'metadata_only' ||
        receipt['能力ID'] != 'a2a.connection.connect' ||
        receipt['権限ID'] != 'permission.a2a.connection.connect' ||
        receipt['復旧ID'] != 'recover-a2a-connection') {
      throw const BrokerClientException('A2A接続receiptの固定境界が不正です');
    }

    final agentId = receipt['AgentID'];
    final evidence = receipt['証拠種別'];
    final state = receipt['接続状態'];
    final approval = receipt['承認状態'];
    final auditId = receipt['接続監査ID'];
    if (agentId is! String ||
        !_validAgentId(agentId) ||
        !((evidence == 'LIVE_RUNTIME' &&
                state == 'connected' &&
                approval == 'owner_control_approved') ||
            (evidence == 'INTERNAL_STATE' &&
                state == 'restored_pending_review' &&
                approval == 'owner_reapproval_required')) ||
        auditId is! String ||
        !_validAuditId(auditId)) {
      throw const BrokerClientException('A2A接続receiptの状態または識別子が不正です');
    }

    final card = _object(receipt['Agent Card'], 'Agent Card');
    const cardFields = {
      'agent_id',
      'display_name',
      'description_summary',
      'version',
      'endpoint_hash',
      'supported_interfaces',
      'capabilities',
      'skills',
      'authentication',
      'origin',
      'status',
    };
    final displayName = card['display_name'];
    final description = card['description_summary'];
    final version = card['version'];
    final endpointHash = card['endpoint_hash'];
    if (card.length != cardFields.length ||
        card.keys.any((key) => !cardFields.contains(key)) ||
        card['agent_id'] != agentId ||
        displayName is! String ||
        !_validExternalText(displayName, 256) ||
        description is! String ||
        !_validExternalText(description, 1024) ||
        version is! String ||
        !_validExternalText(version, 128) ||
        endpointHash is! String ||
        !_validSha256(endpointHash) ||
        card['origin'] != 'live_runtime' ||
        card['status'] != 'discovered') {
      throw const BrokerClientException(
          'A2A Agent Cardのmetadata projectionが不正です');
    }

    final trust = _object(receipt['Trust'], 'Trust');
    if (trust.length != 4 ||
        trust.keys.any((key) => !{
              'state',
              'evidence_source',
              'reason',
              'requires_operator_review'
            }.contains(key)) ||
        trust['state'] != 'pending_review' ||
        trust['evidence_source'] != 'LIVE_RUNTIME' ||
        trust['requires_operator_review'] != true ||
        trust['reason'] is! String ||
        (trust['reason'] as String).isEmpty ||
        (trust['reason'] as String).length > 512) {
      throw const BrokerClientException('A2A Trust projectionが不正です');
    }
    final capabilityDiff =
        _object(receipt['Capability diff'], 'Capability diff');
    const capabilityFields = {
      'status',
      'added',
      'removed',
      'changed',
      'requires_operator_review',
      'evidence_source',
    };
    if (capabilityDiff.length != capabilityFields.length ||
        capabilityDiff.keys.any((key) => !capabilityFields.contains(key)) ||
        capabilityDiff['status'] != 'not_evaluated' ||
        capabilityDiff['requires_operator_review'] != true ||
        capabilityDiff['evidence_source'] != 'LIVE_RUNTIME' ||
        !{'added', 'removed', 'changed'}.every((key) =>
            capabilityDiff[key] is List &&
            (capabilityDiff[key] as List).isEmpty)) {
      throw const BrokerClientException('A2A capability差分のprojectionが不正です');
    }
    for (final key in const ['Task', 'Message', 'Artifact', 'Stream']) {
      final values = receipt[key];
      if (values is! List || values.length > 256) {
        throw const BrokerClientException('A2A接続receiptの配列が不正です');
      }
    }
    if ((receipt['Task'] as List).isNotEmpty ||
        (receipt['Message'] as List).isNotEmpty ||
        (receipt['Artifact'] as List).isNotEmpty) {
      throw const BrokerClientException('未対応のA2A Task内容を表示できません');
    }

    final authentication = _object(card['authentication'], 'authentication');
    if (authentication.length != 3 ||
        authentication.keys.any((key) => !{
              'schemes',
              'credential_ref',
              'secret_value_present'
            }.contains(key)) ||
        authentication['secret_value_present'] != false ||
        authentication['schemes'] is! List ||
        (authentication['schemes'] as List).length > 16) {
      throw const BrokerClientException('A2A認証metadataに秘密値または未知状態があります');
    }
    final credential =
        _object(authentication['credential_ref'], 'Credential ref');
    const credentialFields = {
      'credential_id',
      'purpose',
      'target',
      'required',
      'status',
    };
    if (credential.length != credentialFields.length ||
        credential.keys.any((key) => !credentialFields.contains(key)) ||
        credential['credential_id'] is! String ||
        !RegExp(r'^[a-f0-9]{32}$')
            .hasMatch(credential['credential_id'] as String) ||
        credential['purpose'] != 'A2A接続' ||
        credential['target'] != agentId ||
        credential['required'] != false ||
        credential['status'] != 'missing') {
      throw const BrokerClientException('A2A Credential参照の範囲が不正です');
    }

    return A2aConnectionSummary(
      agentId: agentId,
      displayName: displayName,
      descriptionSummary: description,
      version: version,
      endpointHash: endpointHash,
      connectionState: state as String,
      evidenceSource: evidence as String,
      approvalState: approval as String,
      trustReason: trust['reason'] as String,
      connectionAuditId: auditId,
    );
  }

  static Map<String, Object?> _object(Object? value, String label) {
    if (value is! Map || value.keys.any((key) => key is! String)) {
      throw BrokerClientException('A2A $label がobjectではありません');
    }
    return Map<String, Object?>.from(value);
  }

  static bool _validAuditId(String value) =>
      value.length <= 256 &&
      RegExp(r'^[A-Za-z0-9_.:-]+$').hasMatch(value) &&
      !value.contains('http://') &&
      !value.contains('https://');

  static bool _validSha256(String value) =>
      RegExp(r'^sha256:[a-f0-9]{64}$').hasMatch(value);

  static bool _validExternalText(String value, int maxLength) {
    const bidiControls = {
      0x061c,
      0x200e,
      0x200f,
      0x2028,
      0x2029,
      0x202a,
      0x202b,
      0x202c,
      0x202d,
      0x202e,
      0x2066,
      0x2067,
      0x2068,
      0x2069,
    };
    if (value.isEmpty || value.length > maxLength) return false;
    return value.runes.every((rune) =>
        rune >= 0x20 &&
        !(rune >= 0x7f && rune <= 0x9f) &&
        !bidiControls.contains(rune));
  }

  static bool _validLoopbackUri(String value) {
    if (value.isEmpty ||
        value.length > 2048 ||
        value.contains('@') ||
        value.contains('?') ||
        value.contains('#') ||
        value.contains('\\') ||
        value.codeUnits.any((unit) => unit <= 0x20 || unit >= 0x7f)) {
      return false;
    }
    final uri = Uri.tryParse(value);
    if (uri == null ||
        uri.scheme != 'http' ||
        uri.userInfo.isNotEmpty ||
        uri.hasQuery ||
        uri.hasFragment ||
        uri.host.isEmpty ||
        uri.port < 1 ||
        uri.port > 65535 ||
        uri.path.length > 1024 ||
        uri.path.codeUnits.any((unit) => unit < 0x21 || unit > 0x7e)) {
      return false;
    }
    final octets = uri.host.split('.');
    if (octets.length != 4 ||
        octets.any((item) =>
            item.isEmpty ||
            item.length > 3 ||
            int.tryParse(item) == null ||
            int.parse(item) > 255)) {
      return false;
    }
    return int.parse(octets.first) == 127;
  }
}
