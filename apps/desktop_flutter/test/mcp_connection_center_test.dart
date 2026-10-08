import 'dart:ui' show SemanticsAction, SemanticsFlag;

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/screens/mcp_connection_center.dart';
import 'package:gui_shell_desktop/services/mcp_connection_client.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart'
    show BrokerClientException, BrokerTransport;

class _McpTransport implements BrokerTransport {
  final operations = <String>[];
  final payloads = <Map<String, Object?>?>[];
  bool connected = true;
  final credentialMetadata = <Map<String, Object?>>[
    {
      '版': 1,
      '資格情報ID': '0123456789abcdef0123456789abcdef',
      '用途': 'mcp_transport',
      '接続対象': 'mcp-fixture',
      '種類': 'api_key',
      '保管方式': 'windows_dpapi',
      '状態': '有効',
      '作成時刻UnixMillis': 1000,
      '最終使用時刻UnixMillis': null,
      '失効時刻UnixMillis': null,
      '暗号文hash': 'sha256:${List<String>.filled(64, 'a').join()}',
      '作成監査ID': 'audit-credential-created',
      '公開範囲': 'metadata_only',
      '証拠種別': 'INTERNAL_STATE',
    },
    {
      '版': 1,
      '資格情報ID': 'abcdef0123456789abcdef0123456789',
      '用途': 'mcp_transport',
      '接続対象': 'other-server',
      '種類': 'oauth',
      '保管方式': 'windows_dpapi',
      '状態': '有効',
      '作成時刻UnixMillis': 1001,
      '最終使用時刻UnixMillis': null,
      '失効時刻UnixMillis': null,
      '暗号文hash': 'sha256:${List<String>.filled(64, 'b').join()}',
      '作成監査ID': 'audit-credential-other',
      '公開範囲': 'metadata_only',
      '証拠種別': 'INTERNAL_STATE',
    },
    {
      '版': 1,
      '資格情報ID': '11111111111111111111111111111111',
      '用途': 'provider_api_key',
      '接続対象': 'openai_codex_cli',
      '種類': 'api_key',
      '保管方式': 'windows_dpapi',
      '状態': '有効',
      '作成時刻UnixMillis': 1002,
      '最終使用時刻UnixMillis': null,
      '失効時刻UnixMillis': null,
      '暗号文hash': 'sha256:${List<String>.filled(64, 'c').join()}',
      '作成監査ID': 'audit-provider-created',
      '公開範囲': 'metadata_only',
      '証拠種別': 'INTERNAL_STATE',
    },
    {
      '版': 1,
      '資格情報ID': '22222222222222222222222222222222',
      '用途': 'provider_api_key',
      '接続対象': 'other-provider',
      '種類': 'api_key',
      '保管方式': 'windows_dpapi',
      '状態': '有効',
      '作成時刻UnixMillis': 1003,
      '最終使用時刻UnixMillis': null,
      '失効時刻UnixMillis': null,
      '暗号文hash': 'sha256:${List<String>.filled(64, 'd').join()}',
      '作成監査ID': 'audit-provider-wrong-target',
      '公開範囲': 'metadata_only',
      '証拠種別': 'INTERNAL_STATE',
    },
    {
      '版': 1,
      '資格情報ID': '33333333333333333333333333333333',
      '用途': 'provider_api_key',
      '接続対象': 'openai_codex_cli',
      '種類': 'api_key',
      '保管方式': 'windows_dpapi',
      '状態': '失効',
      '作成時刻UnixMillis': 1004,
      '最終使用時刻UnixMillis': null,
      '失効時刻UnixMillis': 2000,
      '暗号文hash': 'sha256:${List<String>.filled(64, 'e').join()}',
      '作成監査ID': 'audit-provider-revoked',
      '公開範囲': 'metadata_only',
      '証拠種別': 'INTERNAL_STATE',
    },
  ];
  String listedEvidence = 'INTERNAL_STATE';
  String executionState = 'ready';
  String displayName = 'fixture server';
  final toolMetadata = <String, Object?>{
    'tool_id': 'tool-${List<String>.filled(142, "a").join()}',
    'name': 'tool-fixture',
    'description_summary': '',
    'input_schema_hash': 'sha256:${List<String>.filled(64, "b").join()}',
    'risk': 'unknown',
    'status': {
      'status': 'supported',
      'reason': 'MCP wire schemaを検証済み',
    },
  };
  final resourceMetadata = <String, Object?>{
    'resource_id': 'resource-${List<String>.filled(64, "c").join()}',
    'name': 'fixture-resource',
    'uri_template_hash': 'sha256:${List<String>.filled(64, "d").join()}',
    'mime_type': 'application/octet-stream',
    'status': {
      'status': 'supported',
      'reason': 'MCP resource metadataを検証済み',
    },
  };
  final promptMetadata = <String, Object?>{
    'prompt_id': 'prompt-${List<String>.filled(64, "e").join()}',
    'name': 'fixture-prompt',
    'description_summary': '',
    'argument_schema_hash': 'sha256:${List<String>.filled(64, "f").join()}',
    'status': {
      'status': 'supported',
      'reason': 'MCP prompt metadataを検証済み',
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
        'Resource': [resourceMetadata],
        'Prompt': [promptMetadata],
        'Credential ref': <String, Object?>{},
        'Trust': <String, Object?>{},
        'Capability diff': <String, Object?>{},
        '権限生成': 'なし',
        '公開範囲': 'metadata_only',
        '証拠種別': evidence,
        '接続状態': 'connected',
        '実行状態': executionState,
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
    if (operation == '資格情報一覧') {
      return {
        'status': 'accepted',
        'evidence_source': 'INTERNAL_STATE',
        'body': {
          '版': 1,
          '資格情報一覧': credentialMetadata,
          '件数': credentialMetadata.length,
          '公開範囲': 'metadata_only',
          '証拠種別': 'INTERNAL_STATE',
        },
      };
    }
    if (operation == '資格情報失効') {
      final id = payload!['資格情報ID']! as String;
      final index =
          credentialMetadata.indexWhere((entry) => entry['資格情報ID'] == id);
      if (index < 0) {
        return {
          'status': 'rejected',
          'error': {'code': 'credential_not_found'},
        };
      }
      final revoked = {
        ...credentialMetadata[index],
        '状態': '失効',
        '失効時刻UnixMillis': 2000,
      };
      credentialMetadata[index] = revoked;
      return {
        'status': 'accepted',
        'evidence_source': 'INTERNAL_STATE',
        'body': revoked,
      };
    }
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
    if (operation == 'MCP Tool実行') {
      return {
        'status': 'accepted',
        'evidence_source': 'LIVE_RUNTIME',
        'body': {
          '版': 1,
          '契約種別': 'MCP Tool実行receipt',
          'ServerID': payload!['ServerID'],
          'ToolID': payload['ToolID'],
          '名前': payload['名前'],
          'arguments_hash': 'sha256:${List<String>.filled(64, "a").join()}',
          'result_hash': 'sha256:${List<String>.filled(64, "b").join()}',
          'Tool error': false,
          'content_count': 1,
          'content_types': ['text'],
          '接続状態': 'connected',
          '能力ID': 'mcp.tool.call',
          '権限ID': 'permission.mcp.tool.call.one_shot',
          '承認状態': 'native_owner_confirmed',
          '承認監査ID': 'audit-mcp-tool-approval',
          '復旧ID': 'inspect-mcp-tool-side-effect',
          '権限生成': 'Broker内一回限りPermissionを消費',
          '公開範囲': 'hash_only',
          '証拠種別': 'LIVE_RUNTIME',
          '監査ID': 'audit-mcp-tool-call',
        },
      };
    }
    throw StateError('予期しない操作: $operation');
  }
}

void main() {
  test('Mac MCP接続は絶対pathとmissing参照だけを送り相対pathと注入を拒否する', () async {
    debugDefaultTargetPlatformOverride = TargetPlatform.macOS;
    addTearDown(() => debugDefaultTargetPlatformOverride = null);
    final transport = _McpTransport()..connected = false;
    final client = McpConnectionClient(transport);
    await client.connect(
        serverId: 'mcp-fixture',
        executable: '/fixture/server',
        workspace: '/fixture/workspace',
        argumentsText: '');
    expect(transport.operations, ['MCP接続']);
    expect(transport.payloads.single?['Credential ref'],
        containsPair('status', 'missing'));
    for (final executable in ['server', '/fixture/server']) {
      await expectLater(
          client.connect(
              serverId: 'mcp-fixture',
              executable: executable,
              workspace: '/fixture/workspace',
              argumentsText: '',
              credentialId: executable.startsWith('/') ? 'a' * 32 : null,
              credentialEnvironmentVariable:
                  executable.startsWith('/') ? 'MCP_API_KEY' : null),
          throwsA(isA<BrokerClientException>()));
    }
    expect(transport.operations, ['MCP接続']);
  });
  testWidgets('Mac MCP投影はnative確認・hash-only・限定groupと未対応注入を表示する',
      (tester) async {
    debugDefaultTargetPlatformOverride = TargetPlatform.macOS;
    addTearDown(() => debugDefaultTargetPlatformOverride = null);
    tester.view.physicalSize = const Size(1400, 2200);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final transport = _McpTransport();
    await tester.pumpWidget(MaterialApp(
        home: Scaffold(
      body: McpConnectionCenterPanel(transport: transport),
    )));
    expect(find.textContaining('Mac native Owner確認'), findsNWidgets(2));
    expect(find.textContaining('D4所有process group'), findsOneWidget);
    await tester.enterText(find.byType(TextField).at(0), 'mcp-fixture');
    await tester.tap(find.text('対象ServerのCredential metadata一覧を取得'));
    await tester.pumpAndSettle();
    expect(
        tester
            .widget<DropdownButtonFormField<String>>(
                find.byKey(const ValueKey('mcp-credential-selection')))
            .onChanged,
        isNull);
    expect(transport.operations, ['資格情報一覧']);
    expect(find.textContaining('secret-marker'), findsNothing);
    debugDefaultTargetPlatformOverride = null;
  });
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

  test('MCP clientは対象一致するCredential metadataだけを選択し値を要求しない', () async {
    final transport = _McpTransport()..connected = false;
    final client = McpConnectionClient(transport);
    final credentials = await client.listCredentials(
      targetServerId: 'mcp-fixture',
    );
    expect(credentials.map((entry) => entry.credentialId), [
      '0123456789abcdef0123456789abcdef',
    ]);
    expect(transport.operations, ['資格情報一覧']);
    expect(transport.payloads.single, {'版': 1});

    await client.connect(
      serverId: 'mcp-fixture',
      executable: r'C:\Program Files\D4 Pocket\mcp-fixture.exe',
      workspace: r'C:\Users\Public\D4PocketWorkspace',
      argumentsText: '',
      credentialId: credentials.single.credentialId,
      credentialEnvironmentVariable: 'MCP_API_KEY',
    );
    final reference = transport.payloads.last!['Credential ref']! as Map;
    expect(reference, {
      'credential_id': '0123456789abcdef0123456789abcdef',
      'purpose': 'mcp_transport',
      'target': 'mcp-fixture',
      'required': true,
      'status': 'configured',
      'environment_variable': 'MCP_API_KEY',
    });
    expect(transport.payloads.toString(), isNot(contains('secret-marker')));
  });

  test('Provider一覧は用途・接続先・種類・有効状態が一致するmetadataだけ返す', () async {
    final transport = _McpTransport();
    final credentials =
        await McpConnectionClient(transport).listProviderCredentials();

    expect(credentials.map((entry) => entry.credentialId), [
      '11111111111111111111111111111111',
    ]);
    expect(transport.operations, ['資格情報一覧']);
    expect(transport.payloads.single, {'版': 1});
    expect(transport.payloads.toString(),
        isNot(contains('synthetic-provider-key')));
  });

  test('MCP clientはCredential IDと登録metadataをBrokerへ送りreceiptを検査する', () async {
    final transport = _McpTransport();
    final client = McpConnectionClient(transport);
    final selected =
        (await client.listCredentials(targetServerId: 'mcp-fixture')).single;
    final revoked = await client.revokeCredential(
      credential: selected,
    );

    expect(revoked.status, '失効');
    expect(transport.operations, ['資格情報一覧', '資格情報失効']);
    expect(transport.payloads.last, {
      '版': 1,
      '資格情報ID': '0123456789abcdef0123456789abcdef',
      '用途': 'mcp_transport',
      '接続対象': 'mcp-fixture',
      '暗号文hash': 'sha256:${List<String>.filled(64, 'a').join()}',
      '作成監査ID': 'audit-credential-created',
    });
    expect(transport.payloads.toString(), isNot(contains('secret-marker')));
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
    expect(connections.single.resourceCount, 1);
    expect(connections.single.resources.single.name, 'fixture-resource');
    expect(connections.single.prompts.single.name, 'fixture-prompt');
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

    final leakedResourceUri = _McpTransport()
      ..resourceMetadata['uri'] = 'file:///secret/resource';
    await expectLater(
      McpConnectionClient(leakedResourceUri).list(),
      throwsA(isA<BrokerClientException>()),
    );

    final unredactedPrompt = _McpTransport()
      ..promptMetadata['description_summary'] = 'secret-marker';
    await expectLater(
      McpConnectionClient(unredactedPrompt).list(),
      throwsA(isA<BrokerClientException>()),
    );
  });

  test('MCP clientはnative確認後のTool receiptをhash-onlyで検証する', () async {
    final transport = _McpTransport();
    final client = McpConnectionClient(transport);
    final receipt = await client.callTool(
      serverId: 'mcp-fixture',
      tool: McpToolSummary(
        toolId: transport.toolMetadata['tool_id']! as String,
        name: transport.toolMetadata['name']! as String,
        inputSchemaHash: transport.toolMetadata['input_schema_hash']! as String,
      ),
      arguments: const {'query': 'needle'},
    );

    expect(receipt.resultHash, startsWith('sha256:'));
    expect(receipt.contentTypes, ['text']);
    expect(transport.operations, ['MCP Tool実行']);
    expect(transport.payloads.single, {
      '版': 1,
      '操作': '実行',
      'ServerID': 'mcp-fixture',
      'ToolID': transport.toolMetadata['tool_id'],
      '名前': 'tool-fixture',
      'arguments': {'query': 'needle'},
    });
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

    await tester.ensureVisible(find.text('Resource一覧：1件'));
    await tester.tap(find.text('Resource一覧：1件'));
    await tester.pumpAndSettle();
    expect(find.text('fixture-resource'), findsOneWidget);
    expect(find.textContaining('URIのhash: sha256:'), findsOneWidget);
    expect(find.textContaining('file:///secret/resource'), findsNothing);

    await tester.ensureVisible(find.text('Prompt一覧：1件'));
    await tester.tap(find.text('Prompt一覧：1件'));
    await tester.pumpAndSettle();
    expect(find.text('fixture-prompt'), findsOneWidget);
    expect(find.textContaining('引数仕様hash: sha256:'), findsOneWidget);
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

  testWidgets('Desktop panelは入力全表示の確認後だけTool要求を送り結果本文を表示しない', (tester) async {
    final transport = _McpTransport();
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
        body: McpConnectionCenterPanel(transport: transport),
      ),
    ));

    await tester.ensureVisible(find.text('接続一覧を取得'));
    await tester.tap(find.text('接続一覧を取得'));
    await tester.pumpAndSettle();
    await tester.ensureVisible(find.text('Tool一覧：1件'));
    await tester.tap(find.text('Tool一覧：1件'));
    await tester.pumpAndSettle();
    await tester.ensureVisible(find.text('確認して実行'));
    await tester.tap(find.text('確認して実行'));
    await tester.pumpAndSettle();
    final argumentEditor = find.byType(TextField).last;
    await tester.enterText(argumentEditor, '{"query":"needle"}');
    await tester.tap(find.text('入力内容を確認'));
    await tester.pumpAndSettle();

    expect(find.textContaining('"query": "needle"'), findsOneWidget);
    expect(transport.operations, ['MCP接続一覧']);
    await tester.tap(find.text('Windows確認へ進む'));
    await tester.pumpAndSettle();

    expect(transport.operations, ['MCP接続一覧', 'MCP Tool実行']);
    expect(find.textContaining('result hash: sha256:'), findsOneWidget);
    expect(find.textContaining('SECRET_RESULT_MARKER'), findsNothing);
    expect(find.textContaining('needle'), findsNothing);
  });

  testWidgets('Mac MCP Toolの入力確認は小さい製品窓でも画面を破壊しない', (tester) async {
    final semantics = tester.ensureSemantics();
    debugDefaultTargetPlatformOverride = TargetPlatform.macOS;
    addTearDown(() => debugDefaultTargetPlatformOverride = null);
    tester.view.physicalSize = const Size(800, 600);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final transport = _McpTransport();
    await tester.pumpWidget(MaterialApp(
      locale: const Locale('ja', 'JP'),
      supportedLocales: const [Locale('ja', 'JP')],
      localizationsDelegates: GlobalMaterialLocalizations.delegates,
      theme: ThemeData(useMaterial3: true, visualDensity: VisualDensity.compact),
      home: Scaffold(body: McpConnectionCenterPanel(transport: transport)),
    ));
    await tester.ensureVisible(find.text('接続一覧を取得'));
    await tester.tap(find.text('接続一覧を取得'));
    await tester.pumpAndSettle();
    await tester.ensureVisible(find.text('Tool一覧：1件'));
    await tester.tap(find.text('Tool一覧：1件'));
    await tester.pumpAndSettle();
    await tester.ensureVisible(find.text('確認して実行'));
    await tester.tap(find.text('確認して実行'));
    await tester.pumpAndSettle();
    final editor = find.byType(TextField).last;
    final input = tester.widget<TextField>(editor);
    expect(input.autocorrect, isFalse);
    expect(input.enableSuggestions, isFalse);
    expect(input.smartQuotesType, SmartQuotesType.disabled);
    expect(input.smartDashesType, SmartDashesType.disabled);
    await tester.tap(editor);
    await tester.enterText(editor, '{"text":"macos-public-input"}');
    await tester.tap(find.text('入力内容を確認'));
    await tester.pump();
    // 入力routeの終了中に確認routeを重ねず、秘密なしの本文を順に確認する。
    expect(find.byType(AlertDialog).evaluate().length, lessThanOrEqualTo(1));
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
    expect(find.textContaining('"text": "macos-public-input"'), findsOneWidget);
    expect(transport.operations, ['MCP接続一覧']);
    await tester.tap(find.text('Mac確認へ進む'));
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
    expect(transport.operations, ['MCP接続一覧', 'MCP Tool実行']);
    expect(find.textContaining('result hash: sha256:'), findsOneWidget);
    expect(find.textContaining('SECRET_RESULT_MARKER'), findsNothing);
    semantics.dispose();
    debugDefaultTargetPlatformOverride = null;
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

  testWidgets('Desktop panelは対象Credential metadataと環境変数名だけを接続要求へ送る',
      (tester) async {
    final transport = _McpTransport()..connected = false;
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
        body: McpConnectionCenterPanel(transport: transport),
      ),
    ));

    await tester.enterText(find.byType(TextField).at(0), 'mcp-fixture');
    await tester.enterText(
      find.byType(TextField).at(1),
      r'C:\Program Files\D4 Pocket\mcp-fixture.exe',
    );
    await tester.enterText(
      find.byType(TextField).at(2),
      r'C:\Users\Public\D4PocketWorkspace',
    );
    await tester.tap(find.text('対象ServerのCredential metadata一覧を取得'));
    await tester.pumpAndSettle();

    final dropdown = find.byKey(const ValueKey('mcp-credential-selection'));
    await tester.ensureVisible(dropdown);
    await tester.tap(dropdown);
    await tester.pumpAndSettle();
    await tester
        .tap(find.textContaining('0123456789abcdef0123456789abcdef').last);
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const ValueKey('mcp-credential-environment-variable')),
      'MCP_API_KEY',
    );
    await tester.tap(find.text('MCP接続を開始'));
    await tester.pumpAndSettle();

    expect(transport.operations, ['資格情報一覧', 'MCP接続']);
    expect(transport.payloads.last!['Credential ref'], {
      'credential_id': '0123456789abcdef0123456789abcdef',
      'purpose': 'mcp_transport',
      'target': 'mcp-fixture',
      'required': true,
      'status': 'configured',
      'environment_variable': 'MCP_API_KEY',
    });
    expect(transport.payloads.toString(), isNot(contains('secret-marker')));
    expect(find.textContaining('synthetic'), findsNothing);
  });

  testWidgets('Desktop panelのCredential失効は選択metadataをBrokerへ送り一覧を更新する',
      (tester) async {
    final transport = _McpTransport()..connected = false;
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
        body: McpConnectionCenterPanel(transport: transport),
      ),
    ));
    await tester.enterText(find.byType(TextField).at(0), 'mcp-fixture');
    await tester.tap(find.text('対象ServerのCredential metadata一覧を取得'));
    await tester.pumpAndSettle();

    final revoke = find.byKey(const ValueKey(
        'mcp-credential-revoke-0123456789abcdef0123456789abcdef'));
    await tester.ensureVisible(revoke);
    final semantics = tester.ensureSemantics();
    try {
      await tester.pumpAndSettle();
      final revokeSemantics =
          tester.getSemantics(find.bySemanticsLabel('資格情報を失効'));
      expect(revokeSemantics.hasFlag(SemanticsFlag.isButton), isTrue);
      expect(revokeSemantics.getSemanticsData().hasAction(SemanticsAction.tap),
          isTrue);
    } finally {
      semantics.dispose();
    }
    await tester.tap(revoke);
    await tester.pumpAndSettle();

    expect(transport.operations, ['資格情報一覧', '資格情報失効', '資格情報一覧']);
    expect(transport.payloads[1], {
      '版': 1,
      '資格情報ID': '0123456789abcdef0123456789abcdef',
      '用途': 'mcp_transport',
      '接続対象': 'mcp-fixture',
      '暗号文hash': 'sha256:${List<String>.filled(64, 'a').join()}',
      '作成監査ID': 'audit-credential-created',
    });
    expect(transport.credentialMetadata.first['状態'], '失効');
    expect(find.text('api_key ・ 失効'), findsOneWidget);
    expect(find.textContaining('secret-marker'), findsNothing);
  });

  testWidgets('Desktop panelはServer ID変更後に旧対象Credentialを再利用しない',
      (tester) async {
    final transport = _McpTransport()..connected = false;
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
        body: McpConnectionCenterPanel(transport: transport),
      ),
    ));

    await tester.enterText(find.byType(TextField).at(0), 'mcp-fixture');
    await tester.tap(find.text('対象ServerのCredential metadata一覧を取得'));
    await tester.pumpAndSettle();
    expect(
        find.byKey(const ValueKey('mcp-credential-selection')), findsOneWidget);

    await tester.enterText(find.byType(TextField).at(0), 'other-server');
    await tester.pumpAndSettle();
    expect(
        find.byKey(const ValueKey('mcp-credential-selection')), findsNothing);
    expect(
      tester
          .widget<TextField>(
            find.byKey(const ValueKey('mcp-credential-environment-variable')),
          )
          .enabled,
      isFalse,
    );
    expect(transport.operations, ['資格情報一覧']);
  });
}
