import 'broker_transport.dart';

const _listOperation = '回帰Case一覧';
const _maximumSafeInteger = 9007199254740991;

final _idPattern = RegExp(r'^[a-f0-9]{32}$');
final _hashPattern = RegExp(r'^sha256:[a-f0-9]{64}$');
final _runtimePattern = RegExp(r'^[A-Za-z0-9][A-Za-z0-9_.-]{0,127}$');
final _auditPattern = RegExp(r'^[A-Za-z0-9][A-Za-z0-9_.:-]{0,255}$');

/// 通常資格でC6回帰Caseの公開metadataだけをページ取得する。
/// private定義、登録、削除、承認、実行操作はこのClientに存在しない。
class RegressionCaseClient {
  RegressionCaseClient(this.transport);

  final BrokerTransport transport;

  static const int maxPageSize = 100;

  Future<RegressionCasePage> list({int after = 0, int limit = 50}) async {
    if (after < 0 ||
        after > _maximumSafeInteger ||
        limit < 1 ||
        limit > maxPageSize) {
      _reject();
    }
    final response = _asMap(
      await transport.request(
        _listOperation,
        payload: {'版': 1, 'after': after, 'limit': limit},
      ),
    );
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
    final auditId = response['audit_event_id'];
    if (!_safeText(response['request_id'], 256) ||
        response['operation'] != _listOperation ||
        response['status'] != 'accepted' ||
        !_isAuditId(auditId) ||
        response['evidence_source'] != 'INTERNAL_STATE' ||
        response['error'] != null ||
        response['health'] != null ||
        response['shutdown_requested'] != false) {
      _reject();
    }
    return RegressionCasePage._parse(
      _asMap(response['body']),
      auditId as String,
      after: after,
      limit: limit,
    );
  }
}

class RegressionCasePage {
  const RegressionCasePage._({
    required this.cases,
    required this.totalCount,
    required this.nextCursor,
    required this.auditId,
  });

  final List<RegressionCaseSummary> cases;
  final int totalCount;
  final int? nextCursor;
  final String auditId;

  static RegressionCasePage _parse(
    Map<String, Object?> body,
    String auditId, {
    required int after,
    required int limit,
  }) {
    _exactKeys(body, const {
      '版',
      '回帰Case一覧',
      '件数',
      '合計件数',
      '次cursor',
      '公開範囲',
      '証拠種別',
    });
    final rawCases = body['回帰Case一覧'];
    final count = body['件数'];
    final total = body['合計件数'];
    final nextCursor = body['次cursor'];
    if (body['版'] != 1 ||
        rawCases is! List ||
        rawCases.length > limit ||
        count != rawCases.length ||
        !_safeInteger(total) ||
        body['公開範囲'] != 'metadata_only' ||
        body['証拠種別'] != 'INTERNAL_STATE') {
      _reject();
    }
    final totalCount = total as int;
    if (totalCount < after + rawCases.length) _reject();
    final expectedCursor =
        after + rawCases.length < totalCount ? after + rawCases.length : null;
    if (nextCursor != expectedCursor) _reject();
    final cases =
        rawCases.map(RegressionCaseSummary._parse).toList(growable: false);
    if (cases.map((item) => item.caseId).toSet().length != cases.length) {
      _reject();
    }
    return RegressionCasePage._(
      cases: List.unmodifiable(cases),
      totalCount: totalCount,
      nextCursor: nextCursor as int?,
      auditId: auditId,
    );
  }
}

class RegressionCaseSummary {
  const RegressionCaseSummary._({
    required this.caseId,
    required this.definitionHash,
    required this.ciphertextHash,
    required this.displayName,
    required this.requestId,
    required this.requestHash,
    required this.runtimeId,
    required this.resultStatus,
    required this.responseHash,
    required this.endAuditId,
    required this.requiredConditionCount,
    required this.forbiddenConditionCount,
    required this.requiredReferenceCount,
    required this.createdAtUnixMillis,
    required this.createdAuditId,
  });

  final String caseId;
  final String definitionHash;
  final String ciphertextHash;
  final String displayName;
  final String requestId;
  final String requestHash;
  final String runtimeId;
  final String resultStatus;
  final String responseHash;
  final String endAuditId;
  final int requiredConditionCount;
  final int forbiddenConditionCount;
  final int requiredReferenceCount;
  final int createdAtUnixMillis;
  final String createdAuditId;

  static RegressionCaseSummary _parse(Object? raw) {
    final value = _asMap(raw);
    _exactKeys(value, const {
      '回帰CaseID',
      '定義hash',
      '暗号文hash',
      '公開表示名',
      '要求ID',
      '要求hash',
      '実行系ID',
      '結果状態',
      '応答hash',
      '終了監査ID',
      '必要条件数',
      '禁止条件数',
      '必要参照数',
      '作成時刻UnixMillis',
      '作成監査ID',
      '公開範囲',
      '証拠種別',
    });
    if (!_matches(value['回帰CaseID'], _idPattern) ||
        !_matches(value['定義hash'], _hashPattern) ||
        !_matches(value['暗号文hash'], _hashPattern) ||
        !_safeText(value['公開表示名'], 128) ||
        !_matches(value['要求ID'], _idPattern) ||
        !_matches(value['要求hash'], _hashPattern) ||
        !_matches(value['実行系ID'], _runtimePattern) ||
        !const {'成功', '保留'}.contains(value['結果状態']) ||
        !_matches(value['応答hash'], _hashPattern) ||
        !_isAuditId(value['終了監査ID']) ||
        !_boundedInt(value['必要条件数'], 0, 16) ||
        !_boundedInt(value['禁止条件数'], 0, 16) ||
        !_boundedInt(value['必要参照数'], 0, 64) ||
        !_safeInteger(value['作成時刻UnixMillis']) ||
        !_isAuditId(value['作成監査ID']) ||
        value['公開範囲'] != 'metadata_only' ||
        value['証拠種別'] != 'INTERNAL_STATE') {
      _reject();
    }
    return RegressionCaseSummary._(
      caseId: value['回帰CaseID'] as String,
      definitionHash: value['定義hash'] as String,
      ciphertextHash: value['暗号文hash'] as String,
      displayName: value['公開表示名'] as String,
      requestId: value['要求ID'] as String,
      requestHash: value['要求hash'] as String,
      runtimeId: value['実行系ID'] as String,
      resultStatus: value['結果状態'] as String,
      responseHash: value['応答hash'] as String,
      endAuditId: value['終了監査ID'] as String,
      requiredConditionCount: value['必要条件数'] as int,
      forbiddenConditionCount: value['禁止条件数'] as int,
      requiredReferenceCount: value['必要参照数'] as int,
      createdAtUnixMillis: value['作成時刻UnixMillis'] as int,
      createdAuditId: value['作成監査ID'] as String,
    );
  }
}

Map<String, Object?> _asMap(Object? value) {
  if (value is! Map) _reject();
  return Map<String, Object?>.from(value);
}

void _exactKeys(Map<String, Object?> value, Set<String> allowed) {
  if (value.keys.toSet().difference(allowed).isNotEmpty ||
      allowed.difference(value.keys.toSet()).isNotEmpty) {
    _reject();
  }
}

bool _matches(Object? value, RegExp pattern) =>
    value is String && pattern.hasMatch(value);

bool _safeText(Object? value, int maxLength) =>
    value is String &&
    value.isNotEmpty &&
    value.length <= maxLength &&
    !value.runes.any((point) => point < 32 || point == 127);

bool _isAuditId(Object? value) => _matches(value, _auditPattern);

bool _safeInteger(Object? value) =>
    value is int && value >= 0 && value <= _maximumSafeInteger;

bool _boundedInt(Object? value, int minimum, int maximum) =>
    value is int && value >= minimum && value <= maximum;

Never _reject() => throw const BrokerClientException('回帰Caseの一覧応答を確認できません');
