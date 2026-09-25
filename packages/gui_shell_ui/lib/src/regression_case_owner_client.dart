import 'dart:convert';

import 'broker_transport.dart';

const _deleteOperation = '回帰Case削除';
const _recoveryOperation = '回帰Case削除中断確認';
const _registrationOperation = '回帰Case登録';

final _caseIdPattern = RegExp(r'^[a-f0-9]{32}$');
final _hashPattern = RegExp(r'^sha256:[a-f0-9]{64}$');
final _auditIdPattern = RegExp(r'^[A-Za-z0-9][A-Za-z0-9_.:-]{0,255}$');
final _runtimePattern = RegExp(r'^[A-Za-z0-9][A-Za-z0-9_.-]{0,127}$');

/// Owner操作の要求口。Owner資格を保持せず、Windows Rust起動器の操作別確認が
/// 通らない要求はBrokerの通常資格経路で拒否される。
class RegressionCaseOwnerClient {
  RegressionCaseOwnerClient(this.transport);

  final BrokerTransport transport;

  Future<RegressionCaseRegistrationReceipt> register({
    required String requestId,
    required String requestHash,
    required String displayName,
    required String redactedInput,
    required List<String> requiredConditions,
    required List<String> forbiddenConditions,
    required String expectedStatus,
    required List<String> requiredReferences,
    required String expectedRoute,
  }) async {
    if (!_caseIdPattern.hasMatch(requestId) ||
        !_hashPattern.hasMatch(requestHash) ||
        !_safeText(displayName, 128) ||
        redactedInput.trim().isEmpty ||
        redactedInput.runes.length > 4096 ||
        !_validList(requiredConditions, 16, 512) ||
        !_validList(forbiddenConditions, 16, 512) ||
        !_validList(requiredReferences, 64, 2048) ||
        !const {'成功', '保留'}.contains(expectedStatus) ||
        expectedRoute.trim().isEmpty ||
        expectedRoute.runes.length > 256) {
      _reject();
    }
    final payload = {
      '版': 1,
      '要求ID': requestId,
      '要求hash': requestHash,
      '公開表示名': displayName,
      '入力方式': 'owner_explicit_redacted',
      '入力': {'内容表示範囲': 'full', '本文': redactedInput},
      '必要条件': List<String>.unmodifiable(requiredConditions),
      '禁止条件': List<String>.unmodifiable(forbiddenConditions),
      '期待状態': expectedStatus,
      '必要参照': List<String>.unmodifiable(requiredReferences),
      '期待経路': expectedRoute,
    };
    if (utf8.encode(jsonEncode(payload)).length > 48 * 1024) _reject();
    final response = await transport.request(
      _registrationOperation,
      payload: payload,
    );
    final body = _acceptedBody(
      response,
      operation: _registrationOperation,
      evidenceSource: 'INTERNAL_STATE',
    );
    _exactKeys(body, const {
      '版',
      '回帰CaseID',
      '定義hash',
      '非公開保管ID',
      '暗号文hash',
      '公開表示名',
      '要求ID',
      '要求hash',
      '実行系ID',
      '結果状態',
      '応答hash',
      '終了監査ID',
      '公開範囲',
      '必要条件数',
      '禁止条件数',
      '必要参照数',
      '作成時刻UnixMillis',
      '作成監査ID',
      '証拠種別',
    });
    if (body['版'] != 1 ||
        !_matches(body['回帰CaseID'], _caseIdPattern) ||
        body['非公開保管ID'] != body['回帰CaseID'] ||
        !_hash(body['定義hash']) ||
        !_hash(body['暗号文hash']) ||
        body['公開表示名'] != displayName ||
        body['要求ID'] != requestId ||
        body['要求hash'] != requestHash ||
        !_runtimeId(body['実行系ID']) ||
        body['結果状態'] != expectedStatus ||
        !_hash(body['応答hash']) ||
        !_auditId(body['終了監査ID']) ||
        body['公開範囲'] != 'hash_only' ||
        body['必要条件数'] != requiredConditions.length ||
        body['禁止条件数'] != forbiddenConditions.length ||
        body['必要参照数'] != requiredReferences.length ||
        !_safeInteger(body['作成時刻UnixMillis']) ||
        !_auditId(body['作成監査ID']) ||
        body['証拠種別'] != 'INTERNAL_STATE') {
      _reject();
    }
    return RegressionCaseRegistrationReceipt._(
      caseId: body['回帰CaseID'] as String,
      definitionHash: body['定義hash'] as String,
      ciphertextHash: body['暗号文hash'] as String,
      requestId: requestId,
      requestHash: requestHash,
      displayName: displayName,
      expectedStatus: expectedStatus,
      auditId: response['audit_event_id'] as String,
    );
  }

  Future<RegressionCaseDeleteReceipt> delete({
    required String caseId,
    required String definitionHash,
    required String ciphertextHash,
  }) async {
    if (!_caseIdPattern.hasMatch(caseId) ||
        !_hashPattern.hasMatch(definitionHash) ||
        !_hashPattern.hasMatch(ciphertextHash)) {
      _reject();
    }
    final response = await transport.request(
      _deleteOperation,
      payload: {
        '版': 1,
        '回帰CaseID': caseId,
        '定義hash': definitionHash,
        '暗号文hash': ciphertextHash,
      },
    );
    final body = _acceptedBody(
      response,
      operation: _deleteOperation,
      evidenceSource: 'LIVE_RUNTIME',
    );
    _exactKeys(body, const {
      '版',
      '回帰CaseID',
      '定義hash',
      '暗号文hash',
      '削除承認監査ID',
      '状態',
      '証拠種別',
    });
    if (body['版'] != 1 ||
        body['回帰CaseID'] != caseId ||
        body['定義hash'] != definitionHash ||
        body['暗号文hash'] != ciphertextHash ||
        !_auditId(body['削除承認監査ID']) ||
        body['状態'] != '削除確定' ||
        body['証拠種別'] != 'LIVE_RUNTIME') {
      _reject();
    }
    return RegressionCaseDeleteReceipt._(
      caseId: caseId,
      definitionHash: definitionHash,
      ciphertextHash: ciphertextHash,
      approvalAuditId: body['削除承認監査ID'] as String,
      auditId: response['audit_event_id'] as String,
    );
  }

  Future<RegressionCaseRecoveryReceipt> recover({
    required String caseId,
  }) async {
    if (!_caseIdPattern.hasMatch(caseId)) _reject();
    final response = await transport.request(
      _recoveryOperation,
      payload: {'版': 1, '回帰CaseID': caseId},
    );
    final body = _acceptedBody(
      response,
      operation: _recoveryOperation,
      evidenceSource: 'LIVE_RUNTIME',
    );
    _exactKeys(body, const {
      '版',
      '回帰CaseID',
      '削除承認監査ID',
      '暗号文hash',
      '現在状態',
      '観測監査head',
      '観測時刻UnixMillis',
      '証拠種別',
    });
    final state = body['現在状態'];
    if (body['版'] != 1 ||
        body['回帰CaseID'] != caseId ||
        !_auditId(body['削除承認監査ID']) ||
        !_hash(body['暗号文hash']) ||
        !_hash(body['観測監査head']) ||
        !_safeInteger(body['観測時刻UnixMillis']) ||
        !const {'暗号文不在・中断照合済み', '暗号文残存・再試行可能'}.contains(state) ||
        body['証拠種別'] != 'LIVE_RUNTIME') {
      _reject();
    }
    return RegressionCaseRecoveryReceipt._(
      caseId: caseId,
      approvalAuditId: body['削除承認監査ID'] as String,
      ciphertextHash: body['暗号文hash'] as String,
      state: state as String,
      observedAuditHead: body['観測監査head'] as String,
      observedAtUnixMillis: body['観測時刻UnixMillis'] as int,
      auditId: response['audit_event_id'] as String,
    );
  }
}

class RegressionCaseDeleteReceipt {
  const RegressionCaseDeleteReceipt._({
    required this.caseId,
    required this.definitionHash,
    required this.ciphertextHash,
    required this.approvalAuditId,
    required this.auditId,
  });

  final String caseId;
  final String definitionHash;
  final String ciphertextHash;
  final String approvalAuditId;
  final String auditId;
}

class RegressionCaseRegistrationReceipt {
  const RegressionCaseRegistrationReceipt._({
    required this.caseId,
    required this.definitionHash,
    required this.ciphertextHash,
    required this.requestId,
    required this.requestHash,
    required this.displayName,
    required this.expectedStatus,
    required this.auditId,
  });

  final String caseId;
  final String definitionHash;
  final String ciphertextHash;
  final String requestId;
  final String requestHash;
  final String displayName;
  final String expectedStatus;
  final String auditId;
}

class RegressionCaseRecoveryReceipt {
  const RegressionCaseRecoveryReceipt._({
    required this.caseId,
    required this.approvalAuditId,
    required this.ciphertextHash,
    required this.state,
    required this.observedAuditHead,
    required this.observedAtUnixMillis,
    required this.auditId,
  });

  final String caseId;
  final String approvalAuditId;
  final String ciphertextHash;
  final String state;
  final String observedAuditHead;
  final int observedAtUnixMillis;
  final String auditId;
}

Map<String, Object?> _acceptedBody(
  Map<String, Object?> response, {
  required String operation,
  required String evidenceSource,
}) {
  _exactKeys(response, const {
    'request_id',
    'operation',
    'status',
    'evidence_source',
    'audit_event_id',
    'error',
    'health',
    'body',
    'shutdown_requested',
  });
  if (!_auditId(response['request_id']) ||
      !_auditId(response['audit_event_id']) ||
      response['operation'] != operation ||
      response['status'] != 'accepted' ||
      response['evidence_source'] != evidenceSource ||
      response['error'] != null ||
      response['health'] != null ||
      response['shutdown_requested'] != false ||
      response['body'] is! Map) {
    _reject();
  }
  return Map<String, Object?>.from(response['body'] as Map);
}

void _exactKeys(Map<String, Object?> value, Set<String> expected) {
  if (value.keys.toSet().difference(expected).isNotEmpty ||
      expected.difference(value.keys.toSet()).isNotEmpty) {
    _reject();
  }
}

bool _auditId(Object? value) =>
    value is String && _auditIdPattern.hasMatch(value);

bool _matches(Object? value, RegExp pattern) =>
    value is String && pattern.hasMatch(value);

bool _safeText(Object? value, int maxLength) =>
    value is String &&
    value.isNotEmpty &&
    value.runes.length <= maxLength &&
    !value.runes.any((point) => point < 32 || point == 127);

bool _hash(Object? value) => value is String && _hashPattern.hasMatch(value);

bool _safeInteger(Object? value) =>
    value is int && value >= 0 && value <= 9007199254740991;

bool _runtimeId(Object? value) =>
    value is String && _runtimePattern.hasMatch(value);

bool _validList(List<String> values, int maxItems, int maxItemLength) =>
    values.length <= maxItems &&
    values.every((value) =>
        value.trim().isNotEmpty && value.runes.length <= maxItemLength);

Never _reject() =>
    throw const BrokerClientException('Owner確認または回帰Case操作の結果を確認できません');
