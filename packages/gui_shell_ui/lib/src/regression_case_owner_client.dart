import 'broker_transport.dart';

const _deleteOperation = '回帰Case削除';
const _recoveryOperation = '回帰Case削除中断確認';

final _caseIdPattern = RegExp(r'^[a-f0-9]{32}$');
final _hashPattern = RegExp(r'^sha256:[a-f0-9]{64}$');
final _auditIdPattern = RegExp(r'^[A-Za-z0-9][A-Za-z0-9_.:-]{0,255}$');

/// Owner操作の要求口。Owner資格を保持せず、Windows Rust起動器の操作別確認が
/// 通らない要求はBrokerの通常資格経路で拒否される。
class RegressionCaseOwnerClient {
  RegressionCaseOwnerClient(this.transport);

  final BrokerTransport transport;

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

bool _hash(Object? value) => value is String && _hashPattern.hasMatch(value);

bool _safeInteger(Object? value) =>
    value is int && value >= 0 && value <= 9007199254740991;

Never _reject() =>
    throw const BrokerClientException('Owner確認または回帰Case操作の結果を確認できません');
