import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/screens/mcp_connection_center.dart';
import 'package:gui_shell_desktop/services/mcp_connection_client.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerClientException, BrokerTransport;

class _McpTransport implements BrokerTransport {
  final operations = <String>[];
  final payloads = <Map<String, Object?>?>[];
  bool connected = true;
  String listedEvidence = 'INTERNAL_STATE';
  String displayName = 'fixture server';
  final toolMetadata = <String, Object?>{
    'tool_id': 'tool-${List<String>.filled(64, "a").join()}',
    'name': 'tool-fixture',
    'description_summary': '',
    'input_schema_hash': 'sha256:${List<String>.filled(64, "b").join()}',
    'risk': 'unknown',
    'status': {
      'status': 'supported',
      'reason': 'MCP wire schemaを検証済み',
    },
  };

  Map<String, Object?> _receipt({
    String evidence = 'INTERNAL_STATE',
    String serverId = 'mcp-fixture',
  }) =>
      {
        '版': 1,
        '契約種別': 'MCP外部概念射影',
        'Server': {'server_id': serverId, '表示名': displayName},
        'Transport': {'kind': 'stdio'},
        'Tool': [toolMetadata],
        'Resource': <Object?>[],
        'Prompt': <Object?>[],
        'Credential ref': <String, Object?>{},
        'Trust': <String, Object?>{},
        'Capability diff': <String, Object?>{},
        '権限生成': 'なし',
        '公開範囲': 'metadata_only',
        '証拠種別': evidence,
        '接続状態': 'connected',
        '能力ID': 'mcp.connection.connect',
        '権限ID': 'permission.mcp.connection.connect',
        '承認状態': 'owner_control_approved',
        '復旧ID': 'recover-mcp-connection',
        '接続監査ID': 'audit-mcp-fixture',
      };

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    operations.add(operation);
    payloads.add(payload);
    if (operation == 'MCP接続一覧') {
      final connections = connected
          ? [_receipt(evidence: listedEvidence)]
          : <Map<String, Object?>>[];
      return {
        'status': 'accepted',
        'evidence_source': 'INTERNAL_STATE',
        'body': {
          '版': 1,
          'MCP接続一覧': connections,
          '件数': connections.length,
          '公開範囲': 'metadata_only',
          '証拠種別': 'INTERNAL_STATE',
        },
      };
    }
    if (operation == 'MCP接続') {
      connected = true;
      return {
        'status': 'accepted',
        'evidence_source': 'INTERNAL_STATE',
        'body': _receipt(serverId: payload!['ServerID']! as String),
      };
    }
    if (operation == 'MCP切断') {
      connected = false;
      return {
        'status': 'accepted',
        'evidence_source': 'LIVE_RUNTIME',
        'body': {
          '版': 1,
          'ServerID': 'mcp-fixture',
          '接続状態': 'disconnected',
          '能力ID': 'mcp.connection.disconnect',
          '権限ID': 'permission.mcp.connection.disconnect',
          '承認状態': 'owner_control_approved',
          '復旧ID': 'retry-mcp-disconnect',
          '権限生成': 'なし',
          '公開範囲': 'metadata_only',
          '証拠種別': 'LIVE_RUNTIME',
          '切断監査ID': 'audit-mcp-disconnect',
        },
      };
    }
    throw StateError('予期しない操作: $operation');
  }
}

void main() {
  test('MCP clientは接続設定を固定Broker payloadへ射影する', () async {
    final transport = _McpTransport()..connected = false;
    final connection = await McpConnectionClient(transport).connect(
      serverId: 'mcp-fixture',
      executable: r'C:\Program Files\D4 Pocket\mcp-fixture.exe',
      workspace: r'C:\Users\Public\D4PocketWorkspace',
      argumentsText: '--stdio\n--mode local',
    );

    expect(connection.serverId, 'mcp-fixture');
    expect(transport.operations, ['MCP接続']);
    expect(transport.payloads.single, {
      '版': 1,
      '操作': '接続',
      'ServerID': 'mcp-fixture',
      '実行file': r'C:\Program Files\D4 Pocket\mcp-fixture.exe',
      '引数': ['--stdio', '--mode local'],
      'workspace': r'C:\Users\Public\D4PocketWorkspace',
      'Transport': 'stdio',
      'Credential ref': {
        'credential_id': '00000000000000000000000000000000',
        'purpose': 'mcp_transport',
        'target': 'mcp-fixture',
        'required': false,
        'status': 'missing',
      },
    });

    await expectLater(
      McpConnectionClient(transport).connect(
        serverId: 'mcp-fixture',
        executable: 'relative.exe',
        workspace: r'C:\Users\Public\D4PocketWorkspace',
        argumentsText: '',
      ),
      throwsA(isA<BrokerClientException>()),
    );
    expect(transport.operations, ['MCP接続']);
  });

  test('MCP clientは内部状態一覧だけを限定metadataへ射影する', () async {
    final transport = _McpTransport();
    final connections = await McpConnectionClient(transport).list();

    expect(connections.single.serverId, 'mcp-fixture');
    expect(connections.single.displayName, 'fixture server');
    expect(connections.single.transport, 'stdio');
    expect(connections.single.toolCount, 1);
    expect(connections.single.tools.single.name, 'tool-fixture');
    expect(
        connections.single.tools.single.inputSchemaHash, startsWith('sha256:'));
    expect(transport.operations, ['MCP接続一覧']);
    expect(transport.payloads.single, {'版': 1});

    transport.listedEvidence = 'LIVE_RUNTIME';
    await expectLater(
      McpConnectionClient(transport).list(),
      throwsA(isA<BrokerClientException>()),
    );

    final bidiMetadata = _McpTransport()..displayName = 'mcp\u202efixture';
    await expectLater(
      McpConnectionClient(bidiMetadata).list(),
      throwsA(isA<BrokerClientException>()),
    );

    final untrustedDescription = _McpTransport()
      ..toolMetadata['description_summary'] = 'secret-marker';
    await expectLater(
      McpConnectionClient(untrustedDescription).list(),
      throwsA(isA<BrokerClientException>()),
    );

    final injectedMetadata = _McpTransport()
      ..toolMetadata['untrusted_summary'] = 'secret-marker';
    await expectLater(
      McpConnectionClient(injectedMetadata).list(),
      throwsA(isA<BrokerClientException>()),
    );

    final bidiToolName = _McpTransport()
      ..toolMetadata['name'] = 'tool\u202efixt';
    await expectLater(
      McpConnectionClient(bidiToolName).list(),
      throwsA(isA<BrokerClientException>()),
    );

    final malformedSchemaHash = _McpTransport()
      ..toolMetadata['input_schema_hash'] = 'not-a-hash';
    await expectLater(
      McpConnectionClient(malformedSchemaHash).list(),
      throwsA(isA<BrokerClientException>()),
    );
  });

  testWidgets('Desktop panelは一覧後にOwner確認付きBroker接続と切断を要求する', (tester) async {
    final transport = _McpTransport();
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
        body: McpConnectionCenterPanel(transport: transport),
      ),
    ));

    await tester.ensureVisible(find.text('接続一覧を取得'));
    await tester.tap(find.text('接続一覧を取得'));
    await tester.pumpAndSettle();
    expect(find.text('fixture server'), findsOneWidget);
    expect(find.textContaining('secret-marker'), findsNothing);
    expect(find.textContaining('サーバーID: mcp-fixture'), findsOneWidget);

    await tester.ensureVisible(find.text('Tool一覧：1件'));
    await tester.tap(find.text('Tool一覧：1件'));
    await tester.pumpAndSettle();
    expect(find.text('tool-fixture'), findsOneWidget);
    expect(find.textContaining('入力仕様hash: sha256:'), findsOneWidget);
    expect(find.textContaining('secret-marker'), findsNothing);

    await tester.ensureVisible(find.text('切断'));
    await tester.tap(find.text('切断'));
    await tester.pumpAndSettle();
    expect(transport.operations, ['MCP接続一覧', 'MCP切断', 'MCP接続一覧']);
    expect(transport.payloads[1], {
      '版': 1,
      '操作': '切断',
      'ServerID': 'mcp-fixture',
    });
    expect(find.text('Brokerが保持するMCP接続はありません。'), findsOneWidget);
  });

  testWidgets('Desktop panelは秘密値を求めず接続設定をBrokerへ送る', (tester) async {
    final transport = _McpTransport()..connected = false;
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
        body: McpConnectionCenterPanel(transport: transport),
      ),
    ));

    await tester.ensureVisible(find.byType(TextField).at(0));
    await tester.enterText(find.byType(TextField).at(0), 'mcp-fixture');
    await tester.ensureVisible(find.byType(TextField).at(1));
    await tester.enterText(
      find.byType(TextField).at(1),
      r'C:\Program Files\D4 Pocket\mcp-fixture.exe',
    );
    await tester.ensureVisible(find.byType(TextField).at(2));
    await tester.enterText(
      find.byType(TextField).at(2),
      r'C:\Users\Public\D4PocketWorkspace',
    );
    await tester.ensureVisible(find.byType(TextField).at(3));
    await tester.enterText(find.byType(TextField).at(3), '--stdio');
    await tester.ensureVisible(find.text('MCP接続を開始'));
    await tester.tap(find.text('MCP接続を開始'));
    await tester.pumpAndSettle();

    expect(transport.operations, ['MCP接続']);
    expect(transport.payloads.single?['Credential ref'], {
      'credential_id': '00000000000000000000000000000000',
      'purpose': 'mcp_transport',
      'target': 'mcp-fixture',
      'required': false,
      'status': 'missing',
    });
    expect(find.text('fixture server'), findsOneWidget);
    expect(find.textContaining('secret-marker'), findsNothing);
  });
}
