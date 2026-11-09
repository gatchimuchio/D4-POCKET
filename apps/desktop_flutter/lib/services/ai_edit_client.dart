import 'dart:convert';

import 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerClientException, BrokerTransport;

import 'broker_client.dart' show brokerPayloadHash;

class AiEditClient {
  const AiEditClient(this._transport);

  final BrokerTransport _transport;

  Future<Map<String, Object?>> propose({
    required String editId,
    required String scope,
    required String instruction,
    required List<String> targetPaths,
    required List<String> expectedChanges,
  }) async {
    final submittedPaths = List<String>.unmodifiable(targetPaths);
    final submittedChanges = List<String>.unmodifiable(expectedChanges);
    final payload = {
      'version': 1,
      'edit_id': editId,
      'scope': scope,
      'instruction': instruction,
      'target_paths': submittedPaths,
      'expected_changes': submittedChanges,
      'source': 'owner_directed_agent',
      'repository_rules_acknowledged': true,
      'self_approval': false,
      'execution_mode': 'proposal_only',
    };
    final response = await _transport.request(
      'GUI Shell編集提案',
      payload: payload,
    );
    if (response['status'] != 'accepted') {
      final error = response['error'];
      final message = error is Map
          ? error['message']?.toString() ?? response.toString()
          : response.toString();
      throw BrokerClientException('GUI Shell編集提案が拒否されました: $message');
    }
    final body = response['body'];
    if (body is! Map || body.keys.any((key) => key is! String)) {
      throw const BrokerClientException('GUI Shell編集提案応答bodyがobjectではありません');
    }
    final receipt = Map<String, Object?>.from(body);
    const receiptKeys = {
      'version',
      'operation',
      'status',
      'edit_id',
      'scope',
      'target_paths',
      'expected_change_count',
      'execution_mode',
      'review_required',
      'apply_status',
      'files_written',
      'permission_generated',
      'approval_state',
      'authority_strip',
      'evidence_source',
      'instruction_hash',
      'audit_id',
    };
    final returnedPaths = receipt['target_paths'];
    final auditId = response['audit_event_id'];
    final instructionHash = receipt['instruction_hash'];
    final pathMatches = returnedPaths is List &&
        returnedPaths.length == submittedPaths.length &&
        returnedPaths.asMap().entries.every(
              (entry) => entry.value == submittedPaths[entry.key],
            );
    if (response['operation'] != 'GUI Shell編集提案' ||
        response['evidence_source'] != 'INTERNAL_STATE' ||
        response['error'] != null ||
        auditId is! String ||
        auditId.isEmpty ||
        auditId.length > 256 ||
        auditId.runes.any((rune) => rune < 0x20 || rune == 0x7f) ||
        receipt.length != receiptKeys.length ||
        receipt.keys.any((key) => !receiptKeys.contains(key)) ||
        receipt['version'] != 1 ||
        receipt['operation'] != 'GUI Shell編集提案' ||
        receipt['status'] != 'accepted' ||
        receipt['edit_id'] != editId ||
        receipt['scope'] != scope ||
        !pathMatches ||
        receipt['expected_change_count'] != submittedChanges.length ||
        receipt['execution_mode'] != 'proposal_only' ||
        receipt['review_required'] != true ||
        receipt['apply_status'] != 'not_started' ||
        receipt['files_written'] != false ||
        receipt['permission_generated'] != false ||
        receipt['approval_state'] != 'owner_review_required' ||
        receipt['authority_strip'] != true ||
        receipt['evidence_source'] != 'INTERNAL_STATE' ||
        instructionHash != brokerPayloadHash(payload) ||
        receipt['audit_id'] != auditId) {
      throw const BrokerClientException(
        'GUI Shell編集提案のreceiptが要求・Audit・Authority境界と一致しません',
      );
    }
    return receipt;
  }
}

String aiEditJson(Map<String, Object?> value) =>
    const JsonEncoder.withIndent('  ').convert(value);
