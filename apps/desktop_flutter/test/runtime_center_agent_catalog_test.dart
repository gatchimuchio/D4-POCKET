import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/screens/runtime_center.dart';
import 'package:gui_shell_desktop/services/shell_core_client.dart';
import 'package:gui_shell_ui/runtime_dialogue_client.dart' show BrokerTransport;

void main() {
  testWidgets('Runtime CenterはBroker Agent Catalogを能力境界付きで表示する',
      (tester) async {
    final transport = _CatalogTransport(_catalogResponse());
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body:
              RuntimeCenter(client: ShellCoreClient.mock(transport: transport)),
        ),
      ),
    );

    final refresh = find.byKey(const ValueKey('runtime-agent-catalog-refresh'));
    await tester.ensureVisible(refresh);
    await tester.tap(refresh);
    await tester.pumpAndSettle();

    expect(transport.operations, ['GUI Shell構成Catalog']);
    expect(find.textContaining('BrokerからRuntime 1件、Agent 1件'), findsOneWidget);
    final entry = find.byKey(
      const ValueKey('runtime-agent-metadata-minidora-local-minidora'),
    );
    await tester.ensureVisible(entry);
    await tester.tap(entry);
    await tester.pumpAndSettle();

    expect(find.textContaining('dialogue: supported'), findsOneWidget);
    expect(find.textContaining('task_execution: unsupported'), findsOneWidget);
    expect(find.textContaining('Workspace: required / deny_outside_workspace'),
        findsOneWidget);
    expect(
      find.textContaining('認証方式／状態: unknown / unknown（秘密値は表示しません）'),
      findsOneWidget,
    );
    expect(
      find.textContaining(
          'Host: windows / network=loopback_only / process_spawn=unsupported'),
      findsOneWidget,
    );
    expect(find.textContaining('Provider health: unknown'), findsOneWidget);
    expect(find.textContaining('Permission／Approval／Trust／Authorityを生成しません'),
        findsOneWidget);
    expect(find.textContaining('secret-marker'), findsNothing);
    expect(tester.takeException(), isNull);
  });

  testWidgets('Broker切替後に旧Agent Catalog metadataを残さない', (tester) async {
    final pending = Completer<Map<String, Object?>>();
    final oldTransport = _CatalogTransport.completer(pending);
    final newTransport =
        _CatalogTransport(_catalogResponse(agentId: 'new-agent'));
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: RuntimeCenter(
              client: ShellCoreClient.mock(transport: oldTransport)),
        ),
      ),
    );

    final refresh = find.byKey(const ValueKey('runtime-agent-catalog-refresh'));
    await tester.ensureVisible(refresh);
    await tester.tap(refresh);
    await tester.pump();
    expect(oldTransport.operations, ['GUI Shell構成Catalog']);

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: RuntimeCenter(
              client: ShellCoreClient.mock(transport: newTransport)),
        ),
      ),
    );
    pending.complete(_catalogResponse(agentId: 'stale-agent'));
    await tester.pumpAndSettle();

    expect(find.textContaining('stale-agent'), findsNothing);
    expect(find.textContaining('Broker接続が変わりました。Agent Catalogを再取得してください。'),
        findsOneWidget);
    expect(newTransport.operations, isEmpty);
    expect(tester.takeException(), isNull);
  });
}

class _CatalogTransport implements BrokerTransport {
  _CatalogTransport(this.response) : completer = null;

  _CatalogTransport.completer(this.completer) : response = null;

  final Map<String, Object?>? response;
  final Completer<Map<String, Object?>>? completer;
  final List<String> operations = [];

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    operations.add(operation);
    final pending = completer;
    if (pending != null) return pending.future;
    return response!;
  }
}

Map<String, Object?> _catalogResponse({String agentId = 'minidora'}) => {
      'status': 'accepted',
      'operation': 'GUI Shell構成Catalog',
      'evidence_source': 'INTERNAL_STATE',
      'body': {
        'version': 1,
        'runtime_ids': ['minidora-local'],
        'agents': [
          {
            'runtime_id': 'minidora-local',
            'metadata': {
              'adapter_id': 'minidora-product-http',
              'agent_id': agentId,
              'provider': 'MINIDORA',
              'provider_id': 'minidora_product_api',
              'version': 'MINIDORA-PRODUCT-API-v1',
              'model': 'unknown',
              'status': 'unsupported',
              'capabilities': [
                {
                  'capability_id': 'dialogue',
                  'support': {
                    'status': 'supported',
                    'reason': 'chat API',
                  },
                },
                {
                  'capability_id': 'task_execution',
                  'support': {
                    'status': 'unsupported',
                    'reason': 'no Task endpoint',
                  },
                },
              ],
              'workspace_requirements': {
                'mode': 'required',
                'boundary_policy': 'deny_outside_workspace',
                'secret_paths': ['.env', '.ssh', 'secrets/'],
              },
              'provider_health': {
                'status': 'unknown',
                'reason': 'not probed',
              },
              'automatic_fallback': false,
              'tool_support': {
                'status': 'unsupported',
                'reason': 'no tool contract',
              },
              'mcp_support': {
                'status': 'unsupported',
                'reason': 'no MCP contract',
              },
              'session_support': {
                'status': 'supported',
                'reason': 'session hash bound',
              },
              'cancellation_support': {
                'status': 'unknown',
                'reason': 'server stop unknown',
              },
              'usage_metrics_support': {
                'status': 'unknown',
                'reason': 'not observed',
              },
              'cost_metrics_support': {
                'status': 'unknown',
                'reason': 'not observed',
              },
              'authentication': {
                'method': 'unknown',
                'status': 'unknown',
                'secret_value_present': false,
              },
              'host_requirements': {
                'platforms': ['windows'],
                'network_scope': 'loopback_only',
                'process_spawn': {
                  'status': 'unsupported',
                  'reason': 'adapter does not launch runtime',
                },
              },
              'evidence_source': 'INTERNAL_STATE',
              'evidence_reason': 'metadata only',
            },
          },
        ],
        'evidence_source': 'INTERNAL_STATE',
        'authority_strip': true,
        'permission_generated': false,
      },
    };
