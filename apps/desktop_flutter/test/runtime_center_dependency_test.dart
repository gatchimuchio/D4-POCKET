import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/models/generated_contracts.dart';
import 'package:gui_shell_desktop/screens/runtime_center.dart';
import 'package:gui_shell_desktop/services/shell_core_client.dart';

void main() {
  testWidgets('Adapterの宣言Runtime IDを同じBroker Snapshotの登録一覧と照合する',
      (tester) async {
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
          body: RuntimeCenter(
              client: _client(
        source: 'broker',
        freshness: '2026-11-17T00:00:00Z',
        runtimes: const [
          RuntimeRecord(
            runtimeId: 'runtime-a',
            name: 'Fixture runtime',
            status: 'ready',
            adapterId: 'adapter-a',
            diagnosticSummary: 'not shown',
          ),
        ],
      ))),
    ));

    expect(
      find.textContaining(
        '宣言Runtime依存: runtime-aの登録metadataあり（status=ready）',
      ),
      findsOneWidget,
    );
    expect(find.textContaining('package導入・版互換性・起動は未確認'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('登録なしは未導入と断定せず、Broker以外のSnapshotはunknownにする', (tester) async {
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
          body: RuntimeCenter(
              client: _client(
        source: 'broker',
        freshness: '2026-11-17T00:00:00Z',
      ))),
    ));
    expect(
      find.textContaining('runtime-aはBroker Snapshotの登録Runtime一覧にありません'),
      findsOneWidget,
    );
    expect(find.textContaining('Runtime製品の未導入とは断定しません'), findsOneWidget);

    await tester.pumpWidget(MaterialApp(
      home: Scaffold(body: RuntimeCenter(client: _client(source: 'fixture'))),
    ));
    await tester.pumpAndSettle();
    expect(
      find.textContaining('Brokerの現在Snapshotを確認できないためunknown'),
      findsOneWidget,
    );
    expect(tester.takeException(), isNull);
  });
}

ShellCoreClient _client({
  required String source,
  String? freshness,
  List<RuntimeRecord> runtimes = const [],
}) {
  final snapshot = ShellSnapshot.fromJson({
    'runtimes': [
      for (final runtime in runtimes)
        {
          'runtime_id': runtime.runtimeId,
          'name': runtime.name,
          'status': runtime.status,
          'adapter_id': runtime.adapterId,
          'diagnostic_summary': runtime.diagnosticSummary,
        },
    ],
    'adapter_catalog': [
      {
        'adapter_id': 'adapter-a',
        'runtime_id': 'runtime-a',
        'publisher': 'fixture publisher',
        'version': '1.0.0',
        'signature': 'verified',
        'hash': 'sha256:${'1' * 64}',
        'requested_capabilities': <Object?>[],
        'granted_capabilities': <Object?>[],
        'denied_capabilities': <Object?>[],
        'trust_status': 'verified',
        'last_verified': 'fixture',
        'update_available': false,
        'known_risks': <Object?>[],
        'source': 'owner_manifest',
        'transport': 'stdio',
        'content_exposure': 'summary',
        'permission_diff': <Object?>[],
        'compatibility': 'compatible',
        'signature_status': 'verified',
        'verification_status': 'verified',
        'management_state': 'verified',
        'active_state': 'inactive',
        'evidence_source': 'INTERNAL_STATE',
        'authority_strip': true,
      },
    ],
    'snapshot_freshness': freshness ?? 'unknown',
  }).copyWith(snapshotSource: source);
  return ShellCoreClient.mock(snapshot: snapshot);
}
