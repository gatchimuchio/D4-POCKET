import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/screens/a2a_connection_center.dart';
import 'package:gui_shell_desktop/services/a2a_connection_client.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerClientException, BrokerTransport;

void main() {
  test('loopback接続と一覧だけをBrokerへ送り資格情報実値を含めない', () async {
    final receipt = _receipt();
    final transport = _A2aTransport([
      _response('A2A接続', 'audit-a2a-connect', receipt,
          evidence: 'INTERNAL_STATE'),
      _response(
        'A2A接続一覧',
        'audit-a2a-list',
        _listBody([receipt]),
        evidence: 'INTERNAL_STATE',
      ),
    ]);
    final client = A2aConnectionClient(transport);

    final connected = await client.connect(
      agentId: 'fixture-agent',
      agentCardUri: 'http://127.0.0.1:9080/.well-known/agent-card.json',
    );
    final listed = await client.list();

    expect(connected.evidenceSource, 'LIVE_RUNTIME');
    expect(connected.approvalState, 'owner_control_approved');
    expect(listed.single.trustReason, contains('owner review前'));
    expect(transport.operations, ['A2A接続', 'A2A接続一覧']);
    expect(transport.payloads.first?['Transport'], 'http');
    expect(
      transport.payloads.first?['Credential ref'],
      {
        'credential_id': '00000000000000000000000000000000',
        'purpose': 'A2A接続',
        'target': 'fixture-agent',
        'required': false,
        'status': 'missing',
      },
    );
    expect(jsonEncode(transport.payloads), isNot(contains('secret-marker')));
    expect(jsonEncode(transport.payloads), isNot(contains('Authorization')));
  });

  test('外部host・HTTPS・userinfo・query・fragmentは送信前に拒否する', () async {
    final transport = _A2aTransport(const []);
    final client = A2aConnectionClient(transport);
    for (final uri in [
      'http://192.168.1.9:9080/card',
      'https://127.0.0.1:9080/card',
      'http://user@127.0.0.1:9080/card',
      'http://127.0.0.1:9080/card?token=secret-marker',
      'http://127.0.0.1:9080/card#fragment',
      'http://127.0.0.1:9080/card with-space',
    ]) {
      await expectLater(
        client.connect(agentId: 'fixture-agent', agentCardUri: uri),
        throwsA(isA<BrokerClientException>()),
      );
    }
    expect(transport.operations, isEmpty);
  });

  test('未知Authority fieldを含むBroker receiptを表示用に受理しない', () async {
    final forged = _receipt()..['Authority'] = {'approved': true};
    final transport = _A2aTransport([
      _response('A2A接続', 'audit-a2a-connect', forged,
          evidence: 'INTERNAL_STATE'),
    ]);

    await expectLater(
      A2aConnectionClient(transport).connect(
        agentId: 'fixture-agent',
        agentCardUri: 'http://127.0.0.1:9080/card',
      ),
      throwsA(isA<BrokerClientException>()),
    );
  });

  test('Agent Cardの表示metadataに行区切りや方向制御があれば表示を拒否する', () async {
    for (final hostileText in [
      'Agent\nTrust: verified',
      'Agent\u202e trusted',
    ]) {
      final receipt = _receipt();
      (receipt['Agent Card'] as Map<String, Object?>)['display_name'] =
          hostileText;
      final transport = _A2aTransport([
        _response('A2A接続', 'audit-a2a-connect', receipt,
            evidence: 'INTERNAL_STATE'),
      ]);
      await expectLater(
        A2aConnectionClient(transport).connect(
          agentId: 'fixture-agent',
          agentCardUri: 'http://127.0.0.1:9080/card',
        ),
        throwsA(isA<BrokerClientException>()),
      );
    }
  });

  testWidgets('画面はmetadataとpending reviewを表示し接続URIを残さない', (tester) async {
    const endpoint = 'http://127.0.0.1:9080/card';
    final receipt = _receipt();
    final transport = _A2aTransport([
      _response('A2A接続一覧', 'audit-empty', _listBody(const []),
          evidence: 'INTERNAL_STATE'),
      _response('A2A接続', 'audit-a2a-connect', receipt,
          evidence: 'INTERNAL_STATE'),
      _response(
        'A2A接続一覧',
        'audit-a2a-list',
        _listBody([receipt]),
        evidence: 'INTERNAL_STATE',
      ),
    ]);

    await tester.pumpWidget(MaterialApp(
      home: Scaffold(body: A2aConnectionCenter(transport: transport)),
    ));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField).at(0), 'fixture-agent');
    await tester.enterText(find.byType(TextField).at(1), endpoint);
    await tester.tap(find.text('Owner確認して接続'));
    await tester.pumpAndSettle();

    expect(transport.operations, ['A2A接続一覧', 'A2A接続', 'A2A接続一覧']);
    expect(find.textContaining('Fixture Agent'), findsOneWidget);
    expect(find.textContaining('未審査（pending_review）'), findsOneWidget);
    expect(find.textContaining('sha256:'), findsOneWidget);
    expect(find.text(endpoint), findsNothing);
    expect(
      tester.widget<TextField>(find.byType(TextField).at(1)).controller!.text,
      isEmpty,
    );
  });

  testWidgets('Broker拒否を接続成功として表示しない', (tester) async {
    final transport = _A2aTransport([
      _response('A2A接続一覧', 'audit-empty', _listBody(const []),
          evidence: 'INTERNAL_STATE'),
      {
        'operation': 'A2A接続',
        'status': 'rejected',
        'error': {'code': 'a2a_connection_failed'},
        'evidence_source': 'INTERNAL_STATE',
        'audit_event_id': 'audit-a2a-rejected',
        'body': null,
      },
    ]);

    await tester.pumpWidget(MaterialApp(
      home: Scaffold(body: A2aConnectionCenter(transport: transport)),
    ));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField).at(0), 'fixture-agent');
    await tester.enterText(
        find.byType(TextField).at(1), 'http://127.0.0.1:9080/card');
    await tester.tap(find.text('Owner確認して接続'));
    await tester.pumpAndSettle();

    expect(find.textContaining('接続metadataを受理しました'), findsNothing);
    expect(find.textContaining('成功表示はしていません'), findsOneWidget);
  });
}

class _A2aTransport implements BrokerTransport {
  _A2aTransport(this.responses);

  final List<Map<String, Object?>> responses;
  final operations = <String>[];
  final payloads = <Map<String, Object?>?>[];

  @override
  Future<Map<String, Object?>> request(String operation,
      {Map<String, Object?>? payload}) async {
    operations.add(operation);
    payloads.add(payload);
    if (responses.isEmpty) throw StateError('fixture応答がありません: $operation');
    return responses.removeAt(0);
  }
}

Map<String, Object?> _response(
  String operation,
  String auditId,
  Map<String, Object?> body, {
  required String evidence,
}) =>
    {
      'operation': operation,
      'status': 'accepted',
      'error': null,
      'evidence_source': evidence,
      'audit_event_id': auditId,
      'body': body,
    };

Map<String, Object?> _listBody(List<Map<String, Object?>> receipts) => {
      '版': 1,
      'A2A接続一覧': receipts,
      '件数': receipts.length,
      '公開範囲': 'metadata_only',
      '証拠種別': 'INTERNAL_STATE',
    };

Map<String, Object?> _receipt() => {
      '版': 1,
      '契約種別': 'A2A外部概念射影',
      'protocol_version': '1.0',
      'AgentID': 'fixture-agent',
      'Agent Card': {
        'agent_id': 'fixture-agent',
        'display_name': 'Fixture Agent',
        'description_summary': '未信頼の試験metadata',
        'version': '1.0-test',
        'endpoint_hash': 'sha256:${List<String>.filled(64, 'a').join()}',
        'supported_interfaces': const [],
        'capabilities': const {},
        'skills': const [],
        'authentication': {
          'schemes': const [],
          'credential_ref': {
            'credential_id': '00000000000000000000000000000000',
            'purpose': 'A2A接続',
            'target': 'fixture-agent',
            'required': false,
            'status': 'missing',
          },
          'secret_value_present': false,
        },
        'origin': 'live_runtime',
        'status': 'discovered',
      },
      'Task': const [],
      'Message': const [],
      'Artifact': const [],
      'Stream': const [],
      'Trust': {
        'state': 'pending_review',
        'evidence_source': 'LIVE_RUNTIME',
        'reason': 'Agent Cardは宣言情報であり、owner review前のTrustを生成しない',
        'requires_operator_review': true,
      },
      'Capability diff': {
        'status': 'not_evaluated',
        'added': const [],
        'removed': const [],
        'changed': const [],
        'requires_operator_review': true,
        'evidence_source': 'LIVE_RUNTIME',
      },
      '権限生成': 'なし',
      'authority_strip': true,
      '公開範囲': 'metadata_only',
      '証拠種別': 'LIVE_RUNTIME',
      '接続状態': 'connected',
      '能力ID': 'a2a.connection.connect',
      '権限ID': 'permission.a2a.connection.connect',
      '承認状態': 'owner_control_approved',
      '復旧ID': 'recover-a2a-connection',
      '接続監査ID': 'audit-a2a-connect',
    };
