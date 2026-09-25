import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_ui/regression_case_client.dart';

const _caseId = 'cccccccccccccccccccccccccccccccc';
final _definitionHash = 'sha256:${List.filled(64, 'd').join()}';
final _ciphertextHash = 'sha256:${List.filled(64, 'e').join()}';

void main() {
  test('削除はCase IDと両hashだけを送信しLIVE_RUNTIME receiptを検証する', () async {
    final transport = _FixtureTransport({
      '回帰Case削除': _accepted(
        '回帰Case削除',
        'LIVE_RUNTIME',
        {
          '版': 1,
          '回帰CaseID': _caseId,
          '定義hash': _definitionHash,
          '暗号文hash': _ciphertextHash,
          '削除承認監査ID': 'audit.delete.approval',
          '状態': '削除確定',
          '証拠種別': 'LIVE_RUNTIME',
        },
      ),
    });

    final receipt = await RegressionCaseOwnerClient(transport).delete(
      caseId: _caseId,
      definitionHash: _definitionHash,
      ciphertextHash: _ciphertextHash,
    );

    expect(transport.operation, '回帰Case削除');
    expect(transport.payload, {
      '版': 1,
      '回帰CaseID': _caseId,
      '定義hash': _definitionHash,
      '暗号文hash': _ciphertextHash,
    });
    expect(receipt.approvalAuditId, 'audit.delete.approval');
    expect(receipt.auditId, 'audit.delete.result');
  });

  test('中断照合はCase IDだけを送り、過去のApproval IDを要求へ含めない', () async {
    final transport = _FixtureTransport({
      '回帰Case削除中断確認': _accepted(
        '回帰Case削除中断確認',
        'LIVE_RUNTIME',
        {
          '版': 1,
          '回帰CaseID': _caseId,
          '削除承認監査ID': 'audit.delete.approval',
          '暗号文hash': _ciphertextHash,
          '現在状態': '暗号文残存・再試行可能',
          '観測監査head': _definitionHash,
          '観測時刻UnixMillis': 1780000000000,
          '証拠種別': 'LIVE_RUNTIME',
        },
      ),
    });

    final receipt =
        await RegressionCaseOwnerClient(transport).recover(caseId: _caseId);

    expect(transport.operation, '回帰Case削除中断確認');
    expect(transport.payload, {'版': 1, '回帰CaseID': _caseId});
    expect(receipt.state, '暗号文残存・再試行可能');
    expect(receipt.auditId, 'audit.delete.result');
  });

  test('Authority風の余分な応答fieldと不一致hashを拒否する', () async {
    final extraField = _accepted(
      '回帰Case削除',
      'LIVE_RUNTIME',
      {
        '版': 1,
        '回帰CaseID': _caseId,
        '定義hash': _definitionHash,
        '暗号文hash': _ciphertextHash,
        '削除承認監査ID': 'audit.delete.approval',
        '状態': '削除確定',
        '証拠種別': 'LIVE_RUNTIME',
        'owner': true,
      },
    );
    await expectLater(
      RegressionCaseOwnerClient(_FixtureTransport({
        '回帰Case削除': extraField,
      })).delete(
        caseId: _caseId,
        definitionHash: _definitionHash,
        ciphertextHash: _ciphertextHash,
      ),
      throwsA(isA<BrokerClientException>()),
    );

    final transport = _FixtureTransport({
      '回帰Case削除': _accepted(
        '回帰Case削除',
        'LIVE_RUNTIME',
        {
          '版': 1,
          '回帰CaseID': _caseId,
          '定義hash': _definitionHash,
          '暗号文hash': 'sha256:${'f' * 64}',
          '削除承認監査ID': 'audit.delete.approval',
          '状態': '削除確定',
          '証拠種別': 'LIVE_RUNTIME',
        },
      ),
    });
    await expectLater(
      RegressionCaseOwnerClient(transport).delete(
        caseId: _caseId,
        definitionHash: _definitionHash,
        ciphertextHash: _ciphertextHash,
      ),
      throwsA(isA<BrokerClientException>()),
    );
  });

  test('要求IDまたはhash形式が不正ならBrokerへ送らない', () async {
    final transport = _FixtureTransport(const {});
    await expectLater(
      RegressionCaseOwnerClient(transport).recover(caseId: '../case'),
      throwsA(isA<BrokerClientException>()),
    );
    await expectLater(
      RegressionCaseOwnerClient(transport).delete(
        caseId: _caseId,
        definitionHash: 'not-a-hash',
        ciphertextHash: _ciphertextHash,
      ),
      throwsA(isA<BrokerClientException>()),
    );
    expect(transport.operation, isNull);
  });
}

class _FixtureTransport implements BrokerTransport {
  _FixtureTransport(this.responses);

  final Map<String, Map<String, Object?>> responses;
  String? operation;
  Map<String, Object?>? payload;

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    this.operation = operation;
    this.payload = payload;
    final response = responses[operation];
    if (response == null) throw StateError('想定外の操作: $operation');
    return response;
  }
}

Map<String, Object?> _accepted(
  String operation,
  String evidenceSource,
  Map<String, Object?> body,
) =>
    {
      'request_id': 'request-owner-fixture',
      'operation': operation,
      'status': 'accepted',
      'evidence_source': evidenceSource,
      'audit_event_id': 'audit.delete.result',
      'error': null,
      'health': null,
      'body': body,
      'shutdown_requested': false,
    };
