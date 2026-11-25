import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/models/generated_contracts.dart';
import 'package:gui_shell_desktop/screens/runtime_center.dart';
import 'package:gui_shell_desktop/services/broker_client.dart';
import 'package:gui_shell_desktop/services/shell_core_client.dart';

void main() {
  const secretMarker = 'adapter-private-error-marker';

  test('Adapter拒否の自由記述を表示せず固定状態へ正規化する', () async {
    final transport = _AdapterTransport(
      response: _suspendedResponse(secretMarker),
    );
    final client = _client(transport);

    final result = await client.manageAdapter('fixture_adapter', '検証');

    expect(result.status, 'suspended');
    expect(result.message, contains('owner_reapproval_required'));
    expect(result.message, isNot(contains(secretMarker)));
    expect(result.auditId, 'audit-adapter-operation-1');
    expect(result.recoveryId, 'recover-adapter-management');
    expect(transport.requests, hasLength(1));
  });

  test('Adapter Manifest拒否の自由記述を結果へ含めない', () async {
    final transport = _AdapterTransport(
      response: {
        ..._responseEnvelope(
          operation: 'アダプター導入',
          status: 'rejected',
          errorMessage: secretMarker,
        ),
        'body': null,
      },
    );
    final result = await _client(transport).manageAdapterManifest(
      '導入',
      const {'Adapter ID': 'fixture_adapter'},
    );

    expect(result.status, 'rejected');
    expect(result.message, 'BrokerがAdapter操作を拒否しました。');
    expect(result.message, isNot(contains(secretMarker)));
    expect(result.auditId, 'audit-adapter-operation-1');
    expect(transport.requests, hasLength(1));
  });

  test('Authority境界が崩れたaccepted receiptを失敗にする', () async {
    final response = _suspendedResponse(secretMarker)
      ..['status'] = 'accepted'
      ..['body'] = {
        ...(_suspendedResponse(secretMarker)['body']! as Map),
        '実行状態': 'accepted',
        '承認状態': 'owner_control_approved',
        '管理状態': 'verified',
        'authority_strip': false,
      };
    response['error'] = null;
    final transport = _AdapterTransport(response: response);

    await expectLater(
      _client(transport).manageAdapter('fixture_adapter', '検証'),
      throwsA(
        isA<BrokerClientException>().having(
          (error) => error.message,
          'safe message',
          isNot(contains(secretMarker)),
        ),
      ),
    );
    expect(transport.requests, hasLength(1));
  });

  testWidgets('Runtime Center masks transport details and does not resend',
      (tester) async {
    final transport = _AdapterTransport(failure: StateError(secretMarker));
    final adapterClient = _client(transport);
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: RuntimeCenter(client: adapterClient)),
      ),
    );

    final verifyButton =
        find.byKey(const ValueKey('adapter-fixture_adapter-検証'));
    await tester.ensureVisible(verifyButton);
    await tester.tap(verifyButton);
    await tester.pumpAndSettle();

    expect(find.textContaining(secretMarker), findsNothing);
    expect(find.textContaining('結果を確認できません'), findsOneWidget);
    expect(find.textContaining('再送しないでください'), findsOneWidget);
    expect(transport.requests, hasLength(1));
  });

  testWidgets('Broker切替後に旧Adapter receiptから新Brokerを更新しない', (tester) async {
    final oldResponse = Completer<Map<String, Object?>>();
    final oldTransport = _AdapterTransport(responseCompleter: oldResponse);
    final newTransport = _AdapterTransport();
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: RuntimeCenter(client: _client(oldTransport))),
      ),
    );

    final verifyButton =
        find.byKey(const ValueKey('adapter-fixture_adapter-検証'));
    await tester.ensureVisible(verifyButton);
    await tester.tap(verifyButton);
    await tester.pump();
    expect(oldTransport.requests, ['アダプター一覧', 'アダプター検証']);

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: RuntimeCenter(client: _client(newTransport))),
      ),
    );
    oldResponse.complete(_acceptedVerificationResponse());
    await tester.pumpAndSettle();

    expect(newTransport.requests, isEmpty);
    expect(find.textContaining('audit-adapter-operation-2'), findsNothing);
    expect(find.textContaining('旧BrokerのAudit'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('Broker切替中に開いたManifestを新Brokerへ送信しない', (tester) async {
    final oldTransport = _AdapterTransport();
    final newTransport = _AdapterTransport();
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: RuntimeCenter(client: _client(oldTransport))),
      ),
    );

    final installButton =
        find.byKey(const ValueKey('adapter-manifest-install'));
    await tester.ensureVisible(installButton);
    await tester.tap(installButton);
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const ValueKey('adapter-manifest-json-input')),
      jsonEncode(_adapterManifestFixture()),
    );

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: RuntimeCenter(client: _client(newTransport))),
      ),
    );
    await tester.tap(find.byKey(const ValueKey('adapter-manifest-submit')));
    await tester.pumpAndSettle();

    expect(oldTransport.requests, isEmpty);
    expect(newTransport.requests, isEmpty);
    expect(tester.takeException(), isNull);
  });
}

Map<String, Object?> _acceptedVerificationResponse() => {
      'request_id': 'adapter-request-2',
      'operation': 'アダプター検証',
      'status': 'accepted',
      'evidence_source': 'INTERNAL_STATE',
      'audit_event_id': 'audit-adapter-operation-2',
      'error': null,
      'health': null,
      'body': {
        '版': 1,
        'Adapter ID': 'fixture_adapter',
        '操作': '検証',
        '実行状態': 'accepted',
        '管理状態': 'verified',
        '検証状態': 'verified',
        '権限生成': 'なし',
        'authority_strip': true,
        '公開範囲': 'metadata_only',
        '証拠種別': 'INTERNAL_STATE',
        '復旧ID': 'recover-adapter-management',
      },
      'shutdown_requested': false,
    };

Map<String, Object?> _adapterManifestFixture() => {
      '版': 1,
      'Adapter ID': 'fixture_new_adapter',
      'Runtime ID': 'fixture_runtime',
      '発行者': 'UI試験fixture',
      'source': 'owner_manifest',
      'version': '1.0.0',
      'transport': 'mock',
      'Content Exposure': 'summary',
      '要求Capability': ['runtime.read'],
      '許可差分': ['content_visibility:none->summary'],
      '既知の危険': ['試験用未検証発行者'],
      '互換性': 'compatible',
      'authority_strip': true,
      'signed_manifest': true,
      '署名対象': '7b',
      '署名': List.filled(128, '0').join(),
      '署名者fingerprint': 'sha256:${List.filled(64, '0').join()}',
    };

ShellCoreClient _client(BrokerTransport transport) => ShellCoreClient.mock(
      snapshot: ShellSnapshot.fromJson({
        'adapter_catalog': [
          {
            'adapter_id': 'fixture_adapter',
            'runtime_id': 'fixture_runtime',
            'publisher': 'fixture publisher',
            'version': '1.0.0',
            'signature': 'unverified',
            'hash': 'sha256:${'1' * 64}',
            'requested_capabilities': <String>[],
            'granted_capabilities': <String>[],
            'denied_capabilities': <String>[],
            'trust_status': 'unverified',
            'last_verified': 'unknown',
            'update_available': false,
            'known_risks': <String>[],
            'management_state': 'installed',
            'verification_status': 'pending_review',
            'evidence_source': 'INTERNAL_STATE',
          },
        ],
      }),
      transport: transport,
    );

Map<String, Object?> _suspendedResponse(String errorMessage) => {
      ..._responseEnvelope(
        operation: 'アダプター検証',
        status: 'suspended',
        errorMessage: errorMessage,
      ),
      'body': {
        '版': 1,
        '操作': '検証',
        'Adapter ID': 'fixture_adapter',
        '実行状態': 'suspended',
        '承認状態': 'owner_reapproval_required',
        '権限生成': 'なし',
        'authority_strip': true,
        '公開範囲': 'metadata_only',
        '証拠種別': 'INTERNAL_STATE',
        '復旧ID': 'recover-adapter-management',
      },
    };

Map<String, Object?> _responseEnvelope({
  required String operation,
  required String status,
  required String errorMessage,
}) =>
    {
      'request_id': 'adapter-request-1',
      'operation': operation,
      'status': status,
      'evidence_source': 'INTERNAL_STATE',
      'audit_event_id': 'audit-adapter-operation-1',
      'error': {
        'code': 'adapter_operation_denied',
        'message': errorMessage,
        'recoverable': true,
        'audit_event_required': true,
        'fail_closed': true,
      },
      'health': null,
      'shutdown_requested': false,
    };

class _AdapterTransport implements BrokerTransport {
  _AdapterTransport({this.response, this.responseCompleter, this.failure});

  final Map<String, Object?>? response;
  final Completer<Map<String, Object?>>? responseCompleter;
  final Object? failure;
  final requests = <String>[];

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    requests.add(operation);
    if (failure != null) throw failure!;
    if (operation == 'アダプター一覧') return _adapterListResponse();
    if (responseCompleter != null) return responseCompleter!.future;
    return response!;
  }
}

Map<String, Object?> _adapterListResponse() => {
      'request_id': 'adapter-list-request',
      'operation': 'アダプター一覧',
      'status': 'accepted',
      'evidence_source': 'INTERNAL_STATE',
      'audit_event_id': 'audit-adapter-list',
      'error': null,
      'health': null,
      'body': {
        '版': 1,
        'Adapter一覧': [
          {
            '版': 1,
            'Adapter ID': 'fixture_adapter',
            'Runtime ID': 'fixture_runtime',
            '発行者': 'fixture publisher',
            'source': 'owner_manifest',
            'version': '1.0.0',
            'transport': 'mock',
            'Content Exposure': 'redacted',
            '要求Capability': <String>[],
            '許可差分': <String>[],
            '既知の危険': <String>[],
            '危険等級': '高',
            '互換性': 'compatible',
            'hash': 'sha256:${'1' * 64}',
            '署名状態': 'unverified',
            '検証状態': 'pending_review',
            '管理状態': 'installed',
            '有効状態': 'inactive',
            '更新可能': false,
            '公開範囲': 'metadata_only',
            '証拠種別': 'INTERNAL_STATE',
            '権限生成': 'なし',
            'authority_strip': true,
            '最終検証': null,
            '監査ID': 'audit-adapter-state',
            '復旧ID': 'recover-adapter-management',
          },
        ],
        '件数': 1,
        '公開範囲': 'metadata_only',
        '証拠種別': 'INTERNAL_STATE',
        '権限生成': 'なし',
        'authority_strip': true,
      },
      'shutdown_requested': false,
    };
