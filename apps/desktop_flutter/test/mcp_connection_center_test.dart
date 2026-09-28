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

  Map<String, Object?> _receipt({String evidence = 'INTERNAL_STATE'}) => {
        '版': 1,
        '契約種別': 'MCP外部概念射影',
        'Server': {'server_id': 'mcp-fixture', '表示名': 'fixture server'},
        'Transport': {'kind': 'stdio'},
        'Tool': [
          {'name': 'tool-fixture', 'description_summary': 'secret-marker'}
        ],
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
  test('MCP clientは内部状態一覧だけを限定metadataへ射影する', () async {
    final transport = _McpTransport();
    final connections = await McpConnectionClient(transport).list();

    expect(connections.single.serverId, 'mcp-fixture');
    expect(connections.single.displayName, 'fixture server');
    expect(connections.single.transport, 'stdio');
    expect(connections.single.toolCount, 1);
    expect(transport.operations, ['MCP接続一覧']);
    expect(transport.payloads.single, {'版': 1});

    transport.listedEvidence = 'LIVE_RUNTIME';
    await expectLater(
      McpConnectionClient(transport).list(),
      throwsA(isA<BrokerClientException>()),
    );
  });

  testWidgets('Desktop panelは一覧後にOwner確認付きBroker切断を要求する', (tester) async {
    final transport = _McpTransport();
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
        body: McpConnectionCenterPanel(transport: transport),
      ),
    ));

    await tester.tap(find.text('接続一覧を取得'));
    await tester.pumpAndSettle();
    expect(find.text('fixture server'), findsOneWidget);
    expect(find.textContaining('secret-marker'), findsNothing);
    expect(find.textContaining('サーバーID: mcp-fixture'), findsOneWidget);

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
}
