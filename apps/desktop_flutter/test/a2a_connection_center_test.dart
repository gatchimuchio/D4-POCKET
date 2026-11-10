import 'dart:convert';

import 'package:flutter/foundation.dart';
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

  test('HTTPS Agent Card要求はhttps transportとしてnative Brokerへ束縛する', () async {
    final transport = _A2aTransport([
      _response('A2A接続', 'audit-a2a-connect', _receipt(),
          evidence: 'INTERNAL_STATE'),
    ]);
    await A2aConnectionClient(transport).connect(
      agentId: 'fixture-agent',
      agentCardUri: 'https://agent.example/.well-known/agent-card.json',
    );

    expect(transport.operations, ['A2A接続']);
    expect(transport.payloads.single?['Transport'], 'https');
    expect(
      transport.payloads.single?['Agent Card URI'],
      'https://agent.example/.well-known/agent-card.json',
    );
  });

  test('public HTTP・不正host・userinfo・query・fragmentは送信前に拒否する', () async {
    final transport = _A2aTransport(const []);
    final client = A2aConnectionClient(transport);
    for (final uri in [
      'http://192.168.1.9:9080/card',
      'http://user@127.0.0.1:9080/card',
      'https://user@agent.example/card',
      'https://bad_host.example/card',
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

  test('Brokerが接続済みと判定したinterfaceはTrust pending_reviewのまま表示する', () async {
    final receipt = _receipt();
    final interfaces = (receipt['Agent Card']
        as Map<String, Object?>)['supported_interfaces'] as List<Object?>;
    (interfaces.single as Map<String, Object?>)['status'] = 'supported';
    final transport = _A2aTransport([
      _response('A2A接続', 'audit-a2a-connect', receipt,
          evidence: 'INTERNAL_STATE'),
    ]);

    final connected = await A2aConnectionClient(transport).connect(
      agentId: 'fixture-agent',
      agentCardUri: 'http://127.0.0.1:9080/card',
    );
    expect(connected.supportedInterfaces.single.status, 'supported');
    expect(connected.approvalState, 'owner_control_approved');
  });

  test('Message送信はBroker応答のbounded metadataだけを返す', () async {
    const rawMessage = 'loopback-send-fixture';
    final transport = _A2aTransport([
      _sendResponse(
        'audit-a2a-send',
        agentId: 'fixture-agent',
        result: {
          'kind': 'task',
          'task_id': 'remote-task-42',
          'context_id': null,
          'state': 'working',
          'artifact_count': 0,
          'history_count': 1,
          'result_hash': 'sha256:${List<String>.filled(64, 'c').join()}',
        },
      ),
    ]);
    final result = await A2aConnectionClient(transport).sendMessage(
      agentId: 'fixture-agent',
      interfaceUri: 'http://127.0.0.1:9080/rpc',
      message: rawMessage,
    );

    expect(result.status, 'accepted');
    expect(result.resultKind, 'task');
    expect(result.remoteState, 'working');
    expect(result.remoteId, 'remote-task-42');
    expect(result.auditEventId, 'audit-a2a-send');
    expect(transport.operations, ['A2Aメッセージ送信']);
    expect(transport.payloads.single?['AgentID'], 'fixture-agent');
    expect(transport.payloads.single?['Interface URI'],
        'http://127.0.0.1:9080/rpc');
    expect(transport.payloads.single?['Message'], rawMessage);
    expect(result.remoteId, 'remote-task-42');
  });

  test('外部作用のunknown応答は再送可能にしない', () async {
    final transport = _A2aTransport([
      {
        'operation': 'A2Aメッセージ送信',
        'status': 'suspended',
        'error': {'code': 'a2a_message_outcome_unknown'},
        'evidence_source': 'LIVE_RUNTIME',
        'audit_event_id': 'audit-a2a-unknown',
        'body': {
          '版': 1,
          'AgentID': 'fixture-agent',
          'interface_hash': 'sha256:${List<String>.filled(64, 'b').join()}',
          'status': 'unknown',
          'message_hash': 'sha256:${List<String>.filled(64, 'd').join()}',
          'result': null,
          'Trust': 'pending_review',
          '権限生成': 'なし',
          'authority_strip': true,
          '公開範囲': 'metadata_only',
          '再送可能': false,
        },
      },
    ]);

    final result = await A2aConnectionClient(transport).sendMessage(
      agentId: 'fixture-agent',
      interfaceUri: 'http://127.0.0.1:9080/rpc',
      message: 'uncertain-send',
    );
    expect(result.status, 'unknown');
    expect(result.auditEventId, 'audit-a2a-unknown');
    expect(transport.operations, ['A2Aメッセージ送信']);
  });

  test('GetTaskはTask ID完全一致・history zero・metadata-only応答だけを受理する', () async {
    final transport = _A2aTransport([
      _taskGetResponse('audit-a2a-task-get', taskId: 'remote-task-42'),
    ]);
    final result = await A2aConnectionClient(transport).getTask(
      agentId: 'fixture-agent',
      interfaceUri: 'http://127.0.0.1:9080/rpc',
      taskId: 'remote-task-42',
    );
    expect(result.status, 'accepted');
    expect(result.state, 'working');
    expect(result.historyCount, 0);
    expect(transport.operations, ['A2Aタスク状態取得']);
    expect(transport.payloads.single, {
      '版': 1,
      '操作': '状態取得',
      'AgentID': 'fixture-agent',
      'Interface URI': 'http://127.0.0.1:9080/rpc',
      'TaskID': 'remote-task-42',
    });

    final mismatch =
        _taskGetResponse('audit-a2a-task-mismatch', taskId: 'different-task');
    await expectLater(
      A2aConnectionClient(_A2aTransport([mismatch])).getTask(
        agentId: 'fixture-agent',
        interfaceUri: 'http://127.0.0.1:9080/rpc',
        taskId: 'remote-task-42',
      ),
      throwsA(isA<BrokerClientException>()),
    );
    final leaked =
        _taskGetResponse('audit-a2a-task-leak', taskId: 'remote-task-42');
    final leakedBody = Map<String, Object?>.from(leaked['body'] as Map);
    leakedBody['private_payload'] = 'remote-task-secret-marker';
    leaked['body'] = leakedBody;
    await expectLater(
      A2aConnectionClient(_A2aTransport([leaked])).getTask(
        agentId: 'fixture-agent',
        interfaceUri: 'http://127.0.0.1:9080/rpc',
        taskId: 'remote-task-42',
      ),
      throwsA(isA<BrokerClientException>()),
    );
  });

  test('GetTaskは選択Artifactだけをfresh requestで全文表示し本文をmetadata結果へ漏らさない', () async {
    const artifactId = 'artifact-output-1';
    final artifactHash = 'sha256:${List<String>.filled(64, 'a').join()}';
    final contentHash = 'sha256:${List<String>.filled(64, 'd').join()}';
    final metadata = await A2aConnectionClient(_A2aTransport([
      _taskGetResponse(
        'audit-a2a-artifacts',
        taskId: 'remote-task-42',
        state: 'completed',
        artifactManifest: [
          {
            'artifact_id': artifactId,
            'artifact_hash': artifactHash,
            'part_count': 1,
            'text_only': true,
            'text_bytes': 19,
          },
        ],
      ),
    ])).getTask(
      agentId: 'fixture-agent',
      interfaceUri: 'http://127.0.0.1:9080/rpc',
      taskId: 'remote-task-42',
    );
    expect(metadata.artifacts.single.artifactId, artifactId);
    expect(metadata.artifacts.single.textOnly, isTrue);
    expect(metadata.selectedArtifact, isNull);

    const visibleText = 'private result body';
    final transport = _A2aTransport([
      _taskGetArtifactResponse(
        'audit-a2a-artifact-full',
        taskId: 'remote-task-42',
        artifactId: artifactId,
        artifactHash: artifactHash,
        contentHash: contentHash,
        parts: const [visibleText],
      ),
    ]);
    final result = await A2aConnectionClient(transport).getTask(
      agentId: 'fixture-agent',
      interfaceUri: 'http://127.0.0.1:9080/rpc',
      taskId: 'remote-task-42',
      artifactId: artifactId,
      artifactHash: artifactHash,
    );
    expect(result.selectedArtifact?.parts, [visibleText]);
    expect(result.selectedArtifact?.contentHash, contentHash);
    expect(transport.payloads.single?['ArtifactID'], artifactId);
    expect(transport.payloads.single?['Artifact hash'], artifactHash);
    expect(jsonEncode(transport.payloads), isNot(contains(visibleText)));

    final forged = _taskGetResponse(
      'audit-a2a-artifact-forged',
      taskId: 'remote-task-42',
    );
    final forgedBody = forged['body'] as Map<String, Object?>;
    final forgedResult = forgedBody['result'] as Map<String, Object?>;
    forgedResult['selected_artifact'] = {
      'artifact_id': artifactId,
      'artifact_hash': artifactHash,
      'content_hash': contentHash,
      'byte_length': visibleText.length,
      'parts': [visibleText],
    };
    await expectLater(
      A2aConnectionClient(_A2aTransport([forged])).getTask(
        agentId: 'fixture-agent',
        interfaceUri: 'http://127.0.0.1:9080/rpc',
        taskId: 'remote-task-42',
      ),
      throwsA(isA<BrokerClientException>()),
    );
  });

  test('GetTaskの結果unknownでも自動再照会しない', () async {
    final transport = _A2aTransport([
      _taskGetUnknown('audit-a2a-task-unknown'),
    ]);
    final result = await A2aConnectionClient(transport).getTask(
      agentId: 'fixture-agent',
      interfaceUri: 'http://127.0.0.1:9080/rpc',
      taskId: 'remote-task-42',
    );
    expect(result.status, 'unknown');
    expect(result.state, isNull);
    expect(transport.operations, ['A2Aタスク状態取得']);
  });

  test('旧保存metadataのunknown interface hashをunknownのまま表示できる', () async {
    final receipt = _receipt();
    final interfaces = (receipt['Agent Card']
        as Map<String, Object?>)['supported_interfaces'] as List<Object?>;
    final interface = interfaces.single as Map<String, Object?>;
    interface['protocol_version'] = 'unknown';
    interface['interface_hash'] = null;
    interface['status'] = 'unknown';
    final transport = _A2aTransport([
      _response('A2A接続', 'audit-a2a-connect', receipt,
          evidence: 'INTERNAL_STATE'),
    ]);

    final connected = await A2aConnectionClient(transport).connect(
      agentId: 'fixture-agent',
      agentCardUri: 'http://127.0.0.1:9080/card',
    );

    expect(connected.supportedInterfaces.single.interfaceHash, isNull);
    expect(connected.supportedInterfaces.single.status, 'unknown');
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
    expect(find.textContaining('sha256:'), findsNWidgets(2));
    expect(find.textContaining('接続方式照合値: sha256:'), findsOneWidget);
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
    expect(
      tester.widget<TextField>(find.byType(TextField).at(1)).controller!.text,
      isEmpty,
    );
  });

  testWidgets('送信画面はnative確認経由のMessageを一回投入し本文を結果表示しない', (tester) async {
    const endpoint = 'http://127.0.0.1:9080/card';
    const message = 'send-this-to-fixture-only';
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
      _sendResponse(
        'audit-a2a-send',
        agentId: 'fixture-agent',
        result: {
          'kind': 'message',
          'message_id': 'remote-message-7',
          'context_id': null,
          'task_id': null,
          'role': 'ROLE_AGENT',
          'part_count': 1,
          'result_hash': 'sha256:${List<String>.filled(64, 'c').join()}',
        },
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
    final agentDropdown = find.byType(DropdownButtonFormField<String>);
    await tester.ensureVisible(agentDropdown);
    await tester.tap(agentDropdown);
    await tester.pumpAndSettle();
    await tester.tap(find.text('Fixture Agent (fixture-agent)').last);
    await tester.pumpAndSettle();
    final interfaceUriField = find.byType(TextField).at(2);
    await tester.ensureVisible(interfaceUriField);
    await tester.enterText(interfaceUriField, 'http://127.0.0.1:9080/rpc');
    final messageField = find.byType(TextField).at(3);
    await tester.ensureVisible(messageField);
    await tester.enterText(messageField, message);
    final sendButton = find.text('native Owner確認して一回送信');
    await tester.ensureVisible(sendButton);
    await tester.tap(sendButton);
    await tester.pumpAndSettle();

    expect(transport.operations.last, 'A2Aメッセージ送信');
    expect(transport.payloads.last?['Message'], message);
    expect(find.textContaining('remote-message-7'), findsOneWidget);
    expect(find.text(message), findsNothing);
    expect(
        tester.widget<TextField>(find.byType(TextField).at(2)).controller!.text,
        isEmpty);
    expect(
        tester.widget<TextField>(find.byType(TextField).at(3)).controller!.text,
        isEmpty);
  });

  testWidgets('Task状態はWindowsの明示操作だけで一回照会し状態metadataを表示する', (tester) async {
    debugDefaultTargetPlatformOverride = TargetPlatform.windows;
    try {
      const endpoint = 'http://127.0.0.1:9080/card';
      final receipt = _receipt();
      final transport = _A2aTransport([
        _response('A2A接続一覧', 'audit-empty', _listBody(const []),
            evidence: 'INTERNAL_STATE'),
        _response('A2A接続', 'audit-a2a-connect', receipt,
            evidence: 'INTERNAL_STATE'),
        _response('A2A接続一覧', 'audit-a2a-list', _listBody([receipt]),
            evidence: 'INTERNAL_STATE'),
        _sendResponse('audit-a2a-send-task', agentId: 'fixture-agent', result: {
          'kind': 'task',
          'task_id': 'remote-task-42',
          'context_id': null,
          'state': 'submitted',
          'artifact_count': 0,
          'history_count': 1,
          'result_hash': 'sha256:${List<String>.filled(64, 'c').join()}',
        }),
        _taskGetResponse('audit-a2a-task-get', taskId: 'remote-task-42'),
      ]);
      await tester.pumpWidget(MaterialApp(
        home: Scaffold(body: A2aConnectionCenter(transport: transport)),
      ));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextField).at(0), 'fixture-agent');
      await tester.enterText(find.byType(TextField).at(1), endpoint);
      await tester.tap(find.text('Owner確認して接続'));
      await tester.pumpAndSettle();
      final dropdown = find.byType(DropdownButtonFormField<String>);
      await tester.ensureVisible(dropdown);
      await tester.tap(dropdown);
      await tester.pumpAndSettle();
      await tester.tap(find.text('Fixture Agent (fixture-agent)').last);
      await tester.pumpAndSettle();
      await tester.enterText(
          find.byType(TextField).at(2), 'http://127.0.0.1:9080/rpc');
      await tester.enterText(find.byType(TextField).at(3), 'fixture message');
      await tester.tap(find.text('native Owner確認して一回送信'));
      await tester.pumpAndSettle();
      expect(
          transport.operations, ['A2A接続一覧', 'A2A接続', 'A2A接続一覧', 'A2Aメッセージ送信']);
      expect(find.textContaining('Task ID（memory-only）: remote-task-42'),
          findsOneWidget);
      final getTaskButton = find.text('native Owner確認してTask状態・Artifact一覧を一回取得');
      await tester.ensureVisible(getTaskButton);
      await tester.tap(getTaskButton);
      await tester.pumpAndSettle();
      expect(transport.operations.last, 'A2Aタスク状態取得');
      expect(transport.payloads.last?['TaskID'], 'remote-task-42');
      expect(transport.payloads.last?['Interface URI'],
          'http://127.0.0.1:9080/rpc');
      expect(find.textContaining('Task state=working'), findsOneWidget);
      expect(jsonEncode(transport.payloads), isNot(contains('private-task')));
      final operationsAfterQuery = List<String>.from(transport.operations);
      debugDefaultTargetPlatformOverride = TargetPlatform.linux;
      await tester.pumpWidget(MaterialApp(
        home: Scaffold(body: A2aConnectionCenter(transport: transport)),
      ));
      await tester.pumpAndSettle();
      expect(
          find.text('native Owner確認してTask状態・Artifact一覧を一回取得'), findsOneWidget);
      debugDefaultTargetPlatformOverride = TargetPlatform.macOS;
      await tester.pumpWidget(MaterialApp(
        home: Scaffold(body: A2aConnectionCenter(transport: transport)),
      ));
      await tester.pumpAndSettle();
      expect(find.text('native Owner確認してTask状態・Artifact一覧を一回取得'), findsNothing);
      expect(transport.operations, operationsAfterQuery);
    } finally {
      debugDefaultTargetPlatformOverride = null;
    }
  });

  testWidgets('Linuxで選択したcompleted text Artifactだけをfresh要求後に表示する',
      (tester) async {
    debugDefaultTargetPlatformOverride = TargetPlatform.linux;
    try {
      const endpoint = 'http://127.0.0.1:9080/card';
      const artifactId = 'artifact-fixture-1';
      const visibleText = 'untrusted-artifact-visible-only';
      A2aTaskQueryResult? transferredArtifact;
      final artifactHash = 'sha256:${List<String>.filled(64, 'a').join()}';
      final contentHash = 'sha256:${List<String>.filled(64, 'b').join()}';
      final receipt = _receipt();
      final transport = _A2aTransport([
        _response('A2A接続一覧', 'audit-empty', _listBody(const []),
            evidence: 'INTERNAL_STATE'),
        _response('A2A接続', 'audit-a2a-connect', receipt,
            evidence: 'INTERNAL_STATE'),
        _response('A2A接続一覧', 'audit-a2a-list', _listBody([receipt]),
            evidence: 'INTERNAL_STATE'),
        _sendResponse('audit-a2a-send-task', agentId: 'fixture-agent', result: {
          'kind': 'task',
          'task_id': 'remote-task-42',
          'context_id': null,
          'state': 'submitted',
          'artifact_count': 0,
          'history_count': 1,
          'result_hash': 'sha256:${List<String>.filled(64, 'c').join()}',
        }),
        _taskGetResponse(
          'audit-a2a-task-metadata',
          taskId: 'remote-task-42',
          state: 'completed',
          artifactManifest: [
            {
              'artifact_id': artifactId,
              'artifact_hash': artifactHash,
              'part_count': 1,
              'text_only': true,
              'text_bytes': utf8.encode(visibleText).length,
            },
          ],
        ),
        _taskGetArtifactResponse(
          'audit-a2a-task-artifact',
          taskId: 'remote-task-42',
          artifactId: artifactId,
          artifactHash: artifactHash,
          contentHash: contentHash,
          parts: const [visibleText],
        ),
      ]);
      await tester.pumpWidget(MaterialApp(
        home: Scaffold(
          body: A2aConnectionCenter(
            transport: transport,
            onPrepareAgentTask: (source) => transferredArtifact = source,
          ),
        ),
      ));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextField).at(0), 'fixture-agent');
      await tester.enterText(find.byType(TextField).at(1), endpoint);
      await tester.tap(find.text('Owner確認して接続'));
      await tester.pumpAndSettle();
      final dropdown = find.byType(DropdownButtonFormField<String>);
      await tester.ensureVisible(dropdown);
      await tester.tap(dropdown);
      await tester.pumpAndSettle();
      await tester.tap(find.text('Fixture Agent (fixture-agent)').last);
      await tester.pumpAndSettle();
      await tester.enterText(
          find.byType(TextField).at(2), 'http://127.0.0.1:9080/rpc');
      await tester.enterText(find.byType(TextField).at(3), 'fixture message');
      await tester.ensureVisible(find.text('native Owner確認して一回送信'));
      await tester.tap(find.text('native Owner確認して一回送信'));
      await tester.pumpAndSettle();

      final query = find.text('native Owner確認してTask状態・Artifact一覧を一回取得');
      await tester.ensureVisible(query);
      await tester.tap(query);
      await tester.pumpAndSettle();
      expect(find.textContaining('Artifact ID=artifact-fixture-1'),
          findsOneWidget);
      expect(find.text(visibleText), findsNothing);
      final show = find.text('native Owner確認してこのtext Artifactを全文表示');
      await tester.ensureVisible(show);
      await tester.tap(show);
      await tester.pumpAndSettle();

      expect(
          transport.operations.where((item) => item == 'A2Aタスク状態取得').length, 2);
      expect(transport.payloads[4]?['ArtifactID'], isNull);
      expect(transport.payloads[5]?['ArtifactID'], artifactId);
      expect(transport.payloads[5]?['Artifact hash'], artifactHash);
      expect(find.text(visibleText), findsOneWidget);
      expect(jsonEncode(transport.payloads), isNot(contains(visibleText)));
      expect(find.textContaining('保存・Audit本文記録はありません'), findsOneWidget);
      final transfer = find.byKey(
        const ValueKey('handoff-a2a-artifact-to-agent-center'),
      );
      await tester.ensureVisible(transfer);
      await tester.tap(transfer);
      await tester.pumpAndSettle();
      expect(transferredArtifact?.taskId, 'remote-task-42');
      expect(transferredArtifact?.selectedArtifact?.parts, [visibleText]);
      expect(find.text(visibleText), findsNothing);
      expect(
        transport.operations.where((item) => item == 'A2Aタスク状態取得'),
        hasLength(2),
      );
    } finally {
      debugDefaultTargetPlatformOverride = null;
    }
  });

  testWidgets('復元pending receiptを新しいOwner確認で再検証できる', (tester) async {
    const endpoint = 'http://127.0.0.1:9081/card';
    final pending = _receipt()
      ..['証拠種別'] = 'INTERNAL_STATE'
      ..['接続状態'] = 'restored_pending_review'
      ..['承認状態'] = 'owner_reapproval_required';
    final renewed = _receipt()..['接続監査ID'] = 'audit-a2a-revalidated';
    final transport = _A2aTransport([
      _response(
        'A2A接続一覧',
        'audit-pending-list',
        _listBody([pending]),
        evidence: 'INTERNAL_STATE',
      ),
      _response('A2A接続', 'audit-a2a-revalidated', renewed,
          evidence: 'INTERNAL_STATE'),
      _response(
        'A2A接続一覧',
        'audit-renewed-list',
        _listBody([renewed]),
        evidence: 'INTERNAL_STATE',
      ),
    ]);

    await tester.pumpWidget(MaterialApp(
      home: Scaffold(body: A2aConnectionCenter(transport: transport)),
    ));
    await tester.pumpAndSettle();
    expect(find.text('接続状態: 再確認が必要'), findsOneWidget);
    expect(find.textContaining('過去の承認は再利用しません'), findsOneWidget);

    await tester.enterText(find.byType(TextField).at(0), 'fixture-agent');
    await tester.enterText(find.byType(TextField).at(1), endpoint);
    await tester.tap(find.text('Owner確認して接続'));
    await tester.pumpAndSettle();

    expect(transport.operations, ['A2A接続一覧', 'A2A接続', 'A2A接続一覧']);
    expect(transport.payloads[1]?['AgentID'], 'fixture-agent');
    expect(transport.payloads[1]?['Agent Card URI'], endpoint);
    expect(find.textContaining('接続metadataを受理しました'), findsOneWidget);
    expect(find.text(endpoint), findsNothing);
    expect(
      tester.widget<TextField>(find.byType(TextField).at(1)).controller!.text,
      isEmpty,
    );
    expect(find.textContaining('接続状態: 接続済み'), findsOneWidget);
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

Map<String, Object?> _sendResponse(
  String auditId, {
  required String agentId,
  required Map<String, Object?> result,
}) =>
    {
      'operation': 'A2Aメッセージ送信',
      'status': 'accepted',
      'error': null,
      'evidence_source': 'LIVE_RUNTIME',
      'audit_event_id': auditId,
      'body': {
        '版': 1,
        'AgentID': agentId,
        'interface_hash': 'sha256:${List<String>.filled(64, 'b').join()}',
        'status': 'accepted',
        'message_hash': 'sha256:${List<String>.filled(64, 'd').join()}',
        'result': result,
        'Trust': 'pending_review',
        '権限生成': 'なし',
        'authority_strip': true,
        '公開範囲': 'metadata_only',
        '再送可能': false,
      },
    };

Map<String, Object?> _taskGetResponse(
  String auditId, {
  required String taskId,
  String state = 'working',
  List<Map<String, Object?>> artifactManifest = const [],
}) =>
    {
      'operation': 'A2Aタスク状態取得',
      'status': 'accepted',
      'error': null,
      'evidence_source': 'LIVE_RUNTIME',
      'audit_event_id': auditId,
      'body': {
        '版': 1,
        'AgentID': 'fixture-agent',
        'interface_hash': 'sha256:${List<String>.filled(64, 'b').join()}',
        'status': 'accepted',
        'result': {
          'kind': 'task',
          'task_id': taskId,
          'context_id': 'context-fixture-1',
          'state': state,
          'artifact_count': artifactManifest.length,
          'history_count': 0,
          'result_hash': 'sha256:${List<String>.filled(64, 'c').join()}',
          'artifact_manifest': artifactManifest,
          'selected_artifact': null,
        },
        'Trust': 'pending_review',
        '権限生成': 'なし',
        'authority_strip': true,
        '公開範囲': 'metadata_only',
        '履歴公開': false,
        'Credential使用': false,
      },
    };

Map<String, Object?> _taskGetArtifactResponse(
  String auditId, {
  required String taskId,
  required String artifactId,
  required String artifactHash,
  required String contentHash,
  required List<String> parts,
}) {
  final response = _taskGetResponse(
    auditId,
    taskId: taskId,
    state: 'completed',
    artifactManifest: [
      {
        'artifact_id': artifactId,
        'artifact_hash': artifactHash,
        'part_count': parts.length,
        'text_only': true,
        'text_bytes':
            parts.fold<int>(0, (sum, part) => sum + utf8.encode(part).length),
      },
    ],
  );
  final body = response['body'] as Map<String, Object?>;
  body['公開範囲'] = 'full';
  final result = body['result'] as Map<String, Object?>;
  result['selected_artifact'] = {
    'artifact_id': artifactId,
    'artifact_hash': artifactHash,
    'content_hash': contentHash,
    'byte_length':
        parts.fold<int>(0, (sum, part) => sum + utf8.encode(part).length),
    'parts': parts,
  };
  return response;
}

Map<String, Object?> _taskGetUnknown(String auditId) => {
      'operation': 'A2Aタスク状態取得',
      'status': 'suspended',
      'error': {'code': 'a2a_task_query_outcome_unknown'},
      'evidence_source': 'LIVE_RUNTIME',
      'audit_event_id': auditId,
      'body': {
        '版': 1,
        'AgentID': 'fixture-agent',
        'interface_hash': 'sha256:${List<String>.filled(64, 'b').join()}',
        'status': 'unknown',
        'result': null,
        'Trust': 'pending_review',
        '権限生成': 'なし',
        'authority_strip': true,
        '公開範囲': 'metadata_only',
        '履歴公開': false,
        'Credential使用': false,
      },
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
        'supported_interfaces': <Map<String, Object?>>[
          {
            'binding': 'jsonrpc',
            'protocol_version': '1.0',
            'interface_hash': 'sha256:${List<String>.filled(64, 'b').join()}',
            'transport': 'http',
            'status': 'supported',
            'reason': '無認証A2A v1.0 JSON-RPC SendMessage接続済み',
          },
        ],
        'capabilities': {
          'streaming': false,
          'push_notifications': false,
          'state_transition_history': false,
          'extended_agent_card': false,
        },
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
