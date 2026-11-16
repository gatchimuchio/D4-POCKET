import 'package:broker_contract/broker_transport.dart'
    show BrokerClientException, BrokerTransport;
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_shell_desktop/screens/provider_credential_metadata_panel.dart';

void main() {
  testWidgets('設定表示中にBrokerから有効Provider metadataだけを取り、秘密値を出さない',
      (tester) async {
    const secretMarker = 'PROVIDER_SECRET_SENTINEL';
    final transport = _CredentialTransport(secretAuditId: secretMarker);
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
        body: ProviderCredentialMetadataPanel(
          transport: transport,
          active: true,
        ),
      ),
    ));
    await tester.pumpAndSettle();

    expect(transport.operations, ['資格情報一覧']);
    expect(find.textContaining('11111111111111111111111111111111'),
        findsOneWidget);
    expect(find.textContaining('linux_secret_service'), findsOneWidget);
    expect(
        find.textContaining('22222222222222222222222222222222'), findsNothing);
    expect(find.textContaining(secretMarker), findsNothing);
    expect(find.textContaining('外部Providerでの認証成功も確認しません'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('inactiveまたはBroker不在なら資格情報を読まない', (tester) async {
    final transport = _CredentialTransport();
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
        body: ProviderCredentialMetadataPanel(
          transport: transport,
          active: false,
        ),
      ),
    ));
    expect(transport.operations, isEmpty);
    expect(find.text('設定を開くとBrokerからmetadataを取得します。'), findsOneWidget);

    await tester.pumpWidget(const MaterialApp(
      home: Scaffold(
        body: ProviderCredentialMetadataPanel(transport: null, active: true),
      ),
    ));
    expect(transport.operations, isEmpty);
    expect(find.textContaining('Credential状態はunknown'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });
}

class _CredentialTransport implements BrokerTransport {
  _CredentialTransport({this.secretAuditId});

  final String? secretAuditId;
  final operations = <String>[];

  @override
  Future<Map<String, Object?>> request(
    String operation, {
    Map<String, Object?>? payload,
  }) async {
    operations.add(operation);
    if (operation != '資格情報一覧') {
      throw const BrokerClientException('unexpected operation');
    }
    return {
      'status': 'accepted',
      'evidence_source': 'INTERNAL_STATE',
      'body': {
        '版': 1,
        '件数': 3,
        '公開範囲': 'metadata_only',
        '証拠種別': 'INTERNAL_STATE',
        '資格情報一覧': [
          _credential(
            id: '11111111111111111111111111111111',
            purpose: 'provider_api_key',
            target: 'openai_codex_cli',
            storage: 'linux_secret_service',
            status: '有効',
            revokedAt: null,
            auditId: secretAuditId,
          ),
          _credential(
            id: '22222222222222222222222222222222',
            purpose: 'provider_api_key',
            target: 'other_provider',
            storage: 'linux_secret_service',
            status: '有効',
            revokedAt: null,
          ),
          _credential(
            id: '33333333333333333333333333333333',
            purpose: 'provider_api_key',
            target: 'openai_codex_cli',
            storage: 'linux_secret_service',
            status: '失効',
            revokedAt: 2000,
          ),
        ],
      },
    };
  }
}

Map<String, Object?> _credential({
  required String id,
  required String purpose,
  required String target,
  required String storage,
  required String status,
  required int? revokedAt,
  String? auditId,
}) =>
    {
      '版': 1,
      '資格情報ID': id,
      '用途': purpose,
      '接続対象': target,
      '種類': 'api_key',
      '保管方式': storage,
      '状態': status,
      '作成時刻UnixMillis': 1000,
      '最終使用時刻UnixMillis': null,
      '失効時刻UnixMillis': revokedAt,
      '暗号文hash': 'sha256:${'a' * 64}',
      '作成監査ID': auditId ?? 'audit-credential-$id',
      '公開範囲': 'metadata_only',
      '証拠種別': 'INTERNAL_STATE',
    };
