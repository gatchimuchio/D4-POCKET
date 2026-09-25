import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_ui/regression_case_client.dart';

void main() {
  test('通常資格で要求するのはbounded cursor pageだけ', () async {
    final transport = _FixtureTransport(
      _accepted(_page(nextCursor: 1, totalCount: 2)),
    );
    final page = await RegressionCaseClient(transport).list(after: 0, limit: 1);

    expect(page.cases.single.displayName, '公開Case');
    expect(page.totalCount, 2);
    expect(page.nextCursor, 1);
    expect(page.auditId, 'audit-regression-list-1');
    expect(transport.operation, '回帰Case一覧');
    expect(transport.payload, const {'版': 1, 'after': 0, 'limit': 1});
  });

  test('private field、不正件数、cursor不整合を拒否する', () async {
    final withPrivate = _page(nextCursor: null, totalCount: 1);
    final items =
        (withPrivate['回帰Case一覧'] as List).cast<Map<String, Object?>>();
    items[0]['期待経路'] = 'private route';
    await expectLater(
      RegressionCaseClient(_FixtureTransport(_accepted(withPrivate))).list(),
      throwsA(isA<BrokerClientException>()),
    );

    final wrongCursor = _page(nextCursor: null, totalCount: 2);
    await expectLater(
      RegressionCaseClient(_FixtureTransport(_accepted(wrongCursor))).list(),
      throwsA(isA<BrokerClientException>()),
    );

    await expectLater(
      RegressionCaseClient(_FixtureTransport(
              _accepted(_page(nextCursor: null, totalCount: 1))))
          .list(limit: 101),
      throwsA(isA<BrokerClientException>()),
    );
  });
}

class _FixtureTransport implements BrokerTransport {
  _FixtureTransport(this.response);

  final Map<String, Object?> response;
  String? operation;
  Map<String, Object?>? payload;

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    this.operation = operation;
    this.payload = payload;
    return response;
  }
}

Map<String, Object?> _accepted(Map<String, Object?> body) => {
      'request_id': 'request-regression-list-1',
      'operation': '回帰Case一覧',
      'status': 'accepted',
      'evidence_source': 'INTERNAL_STATE',
      'audit_event_id': 'audit-regression-list-1',
      'error': null,
      'health': null,
      'body': body,
      'shutdown_requested': false,
    };

Map<String, Object?> _page(
        {required int? nextCursor, required int totalCount}) =>
    {
      '版': 1,
      '回帰Case一覧': [
        {
          '回帰CaseID': 'cccccccccccccccccccccccccccccccc',
          '定義hash': _hash('d'),
          '暗号文hash': _hash('e'),
          '公開表示名': '公開Case',
          '要求ID': 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
          '要求hash': _hash('b'),
          '実行系ID': 'codex',
          '結果状態': '成功',
          '応答hash': _hash('f'),
          '終了監査ID': 'audit-dialogue-end',
          '必要条件数': 1,
          '禁止条件数': 0,
          '必要参照数': 2,
          '作成時刻UnixMillis': 1780000000000,
          '作成監査ID': 'audit-regression-created',
          '公開範囲': 'metadata_only',
          '証拠種別': 'INTERNAL_STATE',
        },
      ],
      '件数': 1,
      '合計件数': totalCount,
      '次cursor': nextCursor,
      '公開範囲': 'metadata_only',
      '証拠種別': 'INTERNAL_STATE',
    };

String _hash(String value) => 'sha256:${List.filled(64, value).join()}';
